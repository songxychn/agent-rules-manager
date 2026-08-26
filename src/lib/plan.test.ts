import { describe, expect, it } from "vitest";
import { buildProjectionPlan } from "./plan";
import type { WorkspaceSnapshot } from "./types";

const snapshot: WorkspaceSnapshot = {
  libraryRoot: "/home/me/.agent-rules",
  sourcePath: "/home/me/.agent-rules/AGENTS.md",
  sourceExists: true,
  sourceDigest: "abc123",
  sourceModifiedAt: "2026-08-26T08:00:00Z",
  agents: [
    {
      id: "codex",
      label: "Codex",
      targetPath: "/home/me/.codex/AGENTS.md",
      mode: "symlink",
      state: "ready",
      detail: "No target file yet.",
    },
    {
      id: "claude",
      label: "Claude Code",
      targetPath: "/home/me/.claude/CLAUDE.md",
      mode: "include",
      state: "conflict",
      detail: "Managed markers are invalid.",
    },
  ],
};

describe("buildProjectionPlan", () => {
  it("keeps an empty UI selection empty", () => {
    const plan = buildProjectionPlan(snapshot, []);

    expect(plan.steps).toEqual([]);
    expect(plan.changeCount).toBe(0);
    expect(plan.blocked).toBe(false);
  });

  it("reports a selected unmanaged conflict", () => {
    const plan = buildProjectionPlan(snapshot, ["claude"]);

    expect(plan.blocked).toBe(true);
    expect(plan.steps[0].action).toBe("blocked");
  });
});
