import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { agentNote, agentWarning, demoAgents } from "./agentCatalog";

describe("agent catalog and project demo", () => {
  beforeEach(() => { vi.resetModules(); vi.stubGlobal("window", {}); });
  afterEach(() => vi.unstubAllGlobals());

  it("deduplicates Gemini and Antigravity's shared file", () => {
    const agents = demoAgents("global", "/home/test");
    expect(new Set(agents.map((a) => a.targetPath)).size).toBe(agents.length);
    expect(agents.find((a) => a.id === "gemini")?.label).toContain("Antigravity");
    expect(agents.find((a) => a.id === "gemini")?.maxBytes).toBe(24000);
    expect(agents.find((a) => a.id === "gemini")?.docsUrl).toBe("https://antigravity.google/docs/rules");
    expect(agents.find((a) => a.id === "gemini")?.maxChars).toBeUndefined();
    expect(agents.some((a) => a.id === "cursor")).toBe(false);
  });

  it("formats byte limits without presenting them as character limits", () => {
    const agent = demoAgents("global", "/home/test").find((a) => a.id === "gemini")!;
    expect(agentWarning(agent, true)).toBeUndefined();
    const warning = { ...agent, warning: "Rules exceed the per-file byte limit." };
    expect(agentWarning(warning, true)).toContain("24,000 字节（UTF-8）");
    expect(agentWarning(warning, true)).not.toContain("字符");
    expect(agentWarning(warning, false)).toBe(warning.warning);
    expect(agentWarning({ ...warning, maxBytes: undefined, maxChars: 6000 }, true)).toContain("6,000 字符");
  });

  it("shows the Windows Zed entry separately while retaining the old selector and path", () => {
    for (const platform of ["macos", "linux"]) {
      const agents = demoAgents("global", "/home/test", platform);
      expect(agents.some((a) => a.id === "zed-windows")).toBe(false);
      expect(agents.find((a) => a.id === "zed")?.targetPath).toBe("/home/test/.config/zed/AGENTS.md");
    }
    const windows = demoAgents("global", "C:/Users/demo", "windows");
    const native = windows.find((a) => a.id === "zed-windows")!;
    expect(native.targetPath).toBe("C:/Users/demo/AppData/Roaming/Zed/AGENTS.md");
    expect(agentNote(native, true)).toContain("%APPDATA%");
    expect(windows.find((a) => a.id === "zed")?.targetPath).toBe("C:/Users/demo/.config/zed/AGENTS.md");
    expect(new Set(windows.map((a) => a.targetPath)).size).toBe(windows.length);
    expect(demoAgents("project", "C:/project", "windows").some((a) => a.id.startsWith("zed"))).toBe(false);
  });

  it("keeps current Cline connections separate from both older rule directories", async () => {
    const { backend } = await import("./backend");
    const before = await backend.snapshot();
    expect(before.agents.find((a) => a.id === "grok")?.label).toBe("Grok Build");
    const changes = [{ agentId: "cline-global", connected: true }];
    const plan = await backend.preview(changes);
    expect(plan.steps[0].targetPath).toBe("/Users/demo/.cline/rules/agent-rules.md");
    await backend.apply(changes, false);
    const after = await backend.snapshot();
    for (const id of ["cline", "cline-cli"]) {
      expect(after.agents.find((a) => a.id === id)).toEqual(before.agents.find((a) => a.id === id));
    }
    await backend.rollback();
    expect((await backend.snapshot()).agents).toEqual(before.agents);
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

  it("previews Devin Desktop separately and preserves the legacy Cascade connection on rollback", async () => {
    const { backend } = await import("./backend");
    const legacyChange = [{ agentId: "windsurf", connected: true }];
    await backend.preview(legacyChange);
    await backend.apply(legacyChange, false);
    const before = await backend.snapshot();
    const devin = before.agents.find((a) => a.id === "devin-desktop")!;
    expect(devin.label).toBe("Devin Desktop（原 Windsurf）");
    expect(devin.installed).toBe(true);
    expect(devin.targetPath).toBe("/Users/demo/.devin/global_rules.md");
    expect(devin.maxChars).toBeUndefined();
    expect(agentNote(devin, true)).toContain("Devin Local 全局规则");
    const legacy = before.agents.find((a) => a.id === "windsurf")!;
    expect(legacy.targetPath).toBe("/Users/demo/.codeium/windsurf/memories/global_rules.md");
    expect(legacy.maxChars).toBe(6000);

    const changes = [{ agentId: "devin-desktop", connected: true }];
    const plan = await backend.preview(changes);
    expect(plan.changeCount).toBe(1);
    expect(plan.steps[0].targetPath).toBe(devin.targetPath);
    expect((await backend.snapshot()).agents).toEqual(before.agents);
    await backend.apply(changes, false);
    expect((await backend.snapshot()).agents.find((a) => a.id === "windsurf")).toEqual(legacy);
    expect((await backend.snapshot()).agents.find((a) => a.id === "devin-desktop")?.connected).toBe(true);
    await backend.rollback();
    expect((await backend.snapshot()).agents).toEqual(before.agents);
  });

  it("rejects invalid demo project paths", async () => {
    const { backend } = await import("./backend");
    await expect(backend.snapshot(undefined, "relative")).rejects.toThrow("absolute");
    await expect(backend.snapshot(undefined, "/project/../elsewhere")).rejects.toThrow("absolute");
  });
});
