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
      targetKind: "missing",
      installed: true,
      connected: false,
      detectionDetail: "Detected codex.",
      detail: "No target file yet.",
    },
    {
      id: "claude",
      label: "Claude Code",
      targetPath: "/home/me/.claude/CLAUDE.md",
      mode: "symlink",
      state: "ready",
      targetKind: "independentFile",
      installed: true,
      connected: false,
      detectionDetail: "Detected Claude Code.",
      detail: "Uses an independent native rules file.",
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
    const plan = buildProjectionPlan(snapshot, [{ agentId: "claude", connected: true }]);

    expect(plan.blocked).toBe(true);
    expect(plan.steps[0].action).toBe("blocked");
  });

  it("plans both connecting and disconnecting", () => {
    const connectedSnapshot: WorkspaceSnapshot = {
      ...snapshot,
      agents: [
        snapshot.agents[0],
        {
          ...snapshot.agents[1],
          state: "inSync",
          targetKind: "connectedLink",
          connected: true,
          detail: "Linked directly to the canonical rules.",
        },
      ],
    };

    const plan = buildProjectionPlan(connectedSnapshot, [
      { agentId: "codex", connected: true },
      { agentId: "claude", connected: false },
    ]);

    expect(plan.blocked).toBe(false);
    expect(plan.changeCount).toBe(2);
    expect(plan.steps.map((step) => step.action)).toEqual([
      "createLink",
      "createIndependentFile",
    ]);
  });
});
