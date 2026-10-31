import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

describe("manual Git sync demo", () => {
  beforeEach(() => { vi.resetModules(); vi.stubGlobal("window", {}); });
  afterEach(() => vi.unstubAllGlobals());

  it("previews without mutation, requires conflict decisions, and keeps activation separate", async () => {
    const { backend } = await import("./backend");
    const before = await backend.snapshot();
    const request = { remote: "git@example.test:rules.git", branch: "master", resolutions: [] };
    const first = await backend.previewGitSync(request);
    expect(first.blocked).toBe(true);
    expect(first.conflicts[0].id).toBe("profile:work");
    expect(first.changes.some((c) => c.path === "profiles/personal/AGENTS.md" && c.localBefore !== c.localAfter)).toBe(true);
    expect(await backend.snapshot()).toEqual(before);
    expect(await backend.gitSyncConfig()).toBeNull();
    await expect(backend.applyGitSync(request, first.token)).rejects.toThrow("unresolved");
    const resolved = { ...request, resolutions: [{ id: "profile:work", choice: "useRemote" as const }] };
    const plan = await backend.previewGitSync(resolved);
    expect(plan.blocked).toBe(false);
    await expect(backend.applyGitSync(resolved, first.token)).rejects.toThrow("stale");
    await backend.applyGitSync(resolved, plan.token);
    const after = await backend.snapshot();
    expect(after.runtimeState).toBe("stale");
    expect(after.activeProfileId).toBe(before.activeProfileId);
    expect(after.sourceDigest).toBe(before.sourceDigest);
    expect(after.agents).toEqual(before.agents);
    expect((await backend.previewGitSync({ ...request })).changes).toHaveLength(0);
    await backend.activateProfile("work");
    expect((await backend.snapshot()).runtimeState).toBe("current");
  });

  it("keeps both versions under distinct ids and restores local sync changes through history", async () => {
    const { backend } = await import("./backend");
    const before = await backend.snapshot();
    const point = (await backend.history())[0];
    const request = { remote: "git@example.test:rules.git", branch: "master", resolutions: [{ id: "profile:work", choice: "keepBoth" as const, newProfileId: "work-remote" }] };
    const plan = await backend.previewGitSync(request);
    expect(plan.blocked).toBe(false);
    await backend.applyGitSync(request, plan.token);
    expect((await backend.snapshot()).profiles.some((p) => p.id === "work-remote")).toBe(true);
    expect((await backend.history())[0].operation).toBe("gitSync");
    const restore = await backend.previewRestore(point.id);
    expect(restore.steps.some((s) => s.path.includes("git-sync"))).toBe(true);
    expect(restore.steps.some((s) => s.path.endsWith("personal/AGENTS.md") && s.beforeContent !== s.afterContent)).toBe(true);
    await backend.restoreHistory(point.id, restore.token);
    expect(await backend.snapshot()).toEqual(before);
    expect(await backend.gitSyncConfig()).toBeNull();
  });

  it("rejects stale previews after unrelated local operations and conflicting duplicate ids", async () => {
    const { backend } = await import("./backend");
    const request = { remote: "git@example.test:rules.git", branch: "master", resolutions: [{ id: "profile:work", choice: "keepLocal" as const }] };
    const plan = await backend.previewGitSync(request);
    await backend.createProfile({ id: "new", name: "New", description: "" });
    await expect(backend.applyGitSync(request, plan.token)).rejects.toThrow("stale");
    await expect(backend.previewGitSync({ ...request, resolutions: [{ id: "profile:work", choice: "keepBoth", newProfileId: "default" }] })).rejects.toThrow("unused Profile id");
  });
});
