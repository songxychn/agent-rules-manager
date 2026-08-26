import { describe, expect, it } from "vitest";
import { buildProjectionPlan } from "./plan";
import type { WorkspaceSnapshot } from "./types";

const snapshot: WorkspaceSnapshot = {
  libraryRoot: "/home/me/.agent-rules",
  libraryState: "ready",
  libraryDetail: "Current",
  runtimeState: "current",
  sourcePath: "/home/me/.agent-rules/current/AGENTS.md",
  sourceExists: true,
  sourceDigest: "abc123",
  sourceModifiedAt: "2026-08-26T08:00:00Z",
  activeProfileId: "default",
  packs: [],
  profiles: [],
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

  it("repairs a connected legacy link without toggling it off first", () => {
    const legacySnapshot: WorkspaceSnapshot = {
      ...snapshot,
      agents: [
        {
          ...snapshot.agents[0],
          state: "drifted",
          targetKind: "legacyLink",
          connected: true,
          detail: "Still points to the migrated root rules path.",
        },
      ],
    };

    const plan = buildProjectionPlan(legacySnapshot, [
      { agentId: "codex", connected: true },
    ]);

    expect(plan.blocked).toBe(false);
    expect(plan.changeCount).toBe(1);
    expect(plan.steps[0].action).toBe("replaceLegacyLink");
  });
});
