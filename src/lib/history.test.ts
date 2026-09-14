import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

describe("operation history demo", () => {
  beforeEach(() => { vi.resetModules(); vi.stubGlobal("window", {}); });
  afterEach(() => vi.unstubAllGlobals());

  it("previews cross-scope changes without mutation and reverses a recovery", async () => {
    const { backend } = await import("./backend");
    const before = await backend.snapshot();
    const initial = (await backend.history())[0];
    await backend.createProfile({ id: "test", name: "Test", description: "" });
    await backend.activateProfile("test");
    await backend.apply([{ agentId: "cursor", connected: true }], false, undefined, "/test-project");
    const projectBefore = await backend.snapshot(undefined, "/test-project");
    const latest = (await backend.history())[0];
    const current = await backend.snapshot();
    const preview = await backend.previewRestore(initial.id);
    expect(preview.operations).toHaveLength(3);
    expect(preview.steps.some((s) => s.path.includes("/test-project"))).toBe(true);
    expect(await backend.snapshot()).toEqual(current);
    await backend.restoreHistory(initial.id, preview.token);
    expect((await backend.snapshot()).profiles).toEqual(before.profiles);
    expect((await backend.snapshot(undefined, "/test-project")).agents.find((a) => a.id === "cursor")?.connected).toBe(false);
    expect((await backend.history())[0].operation).toBe("restoreHistory");
    const undo = await backend.previewRestore(latest.id);
    await backend.restoreHistory(latest.id, undo.token);
    expect((await backend.snapshot()).profiles).toEqual(current.profiles);
    expect((await backend.snapshot(undefined, "/test-project")).agents).toEqual(projectBefore.agents);
  });

  it("rejects stale confirmation when another operation was added", async () => {
    const { backend } = await import("./backend");
    const initial = (await backend.history())[0];
    await backend.createProfile({ id: "one", name: "One", description: "" });
    const preview = await backend.previewRestore(initial.id);
    await backend.createProfile({ id: "two", name: "Two", description: "" });
    await expect(backend.restoreHistory(initial.id, preview.token)).rejects.toThrow("stale");
    expect((await backend.snapshot()).profiles.some((p) => p.id === "two")).toBe(true);
  });

  it("selecting a point means after its operation and preserves its Profile", async () => {
    const { backend } = await import("./backend");
    await backend.createProfile({ id: "keep", name: "Keep", description: "" });
    const target = (await backend.history())[0];
    await backend.activateProfile("keep");
    const preview = await backend.previewRestore(target.id);
    expect(preview.steps.some((s) => s.path.includes("profiles/keep"))).toBe(false);
    await backend.restoreHistory(target.id, preview.token);
    expect((await backend.snapshot()).profiles.some((p) => p.id === "keep")).toBe(true);
  });
});
