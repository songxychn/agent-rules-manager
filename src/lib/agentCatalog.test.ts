import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { demoAgents } from "./agentCatalog";

describe("agent catalog and project demo", () => {
  beforeEach(() => { vi.resetModules(); vi.stubGlobal("window", {}); });
  afterEach(() => vi.unstubAllGlobals());

  it("deduplicates Gemini and Antigravity's shared file", () => {
    const agents = demoAgents("global", "/home/test");
    expect(new Set(agents.map((a) => a.targetPath)).size).toBe(agents.length);
    expect(agents.find((a) => a.id === "gemini")?.label).toContain("Antigravity");
    expect(agents.some((a) => a.id === "cursor")).toBe(false);
  });

  it("keeps project connections isolated and rolls back the latest project's changes", async () => {
    const { backend } = await import("./backend");
    const beforeGlobal = await backend.snapshot();
    const beforeB = await backend.snapshot(undefined, "/project-b");
    const changes = [{ agentId: "cursor", connected: true }];
    expect((await backend.preview(changes, undefined, "/project-a")).changeCount).toBe(1);
    await backend.apply(changes, false, undefined, "/project-a");
    const connectedA = await backend.snapshot(undefined, "/project-a");
    expect(connectedA.agents.find((a) => a.id === "cursor")?.connected).toBe(true);
    expect((await backend.snapshot()).agents).toEqual(beforeGlobal.agents);
    expect((await backend.snapshot(undefined, "/project-b")).agents).toEqual(beforeB.agents);
    expect((await backend.preview(changes, undefined, "/project-a")).changeCount).toBe(0);
    await backend.rollback();
    expect((await backend.snapshot(undefined, "/project-a")).agents.find((a) => a.id === "cursor")?.connected).toBe(false);
  });

  it("rejects invalid demo project paths", async () => {
    const { backend } = await import("./backend");
    await expect(backend.snapshot(undefined, "relative")).rejects.toThrow("absolute");
    await expect(backend.snapshot(undefined, "/project/../elsewhere")).rejects.toThrow("absolute");
  });
});
