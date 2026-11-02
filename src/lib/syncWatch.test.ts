import { describe, expect, it } from "vitest";
import { isSyncPending, parseSyncWatchEnabled, summarizeSyncPlan, syncPendingKey } from "./syncWatch";
import type { GitSyncPlan } from "./types";

function plan(overrides: Partial<GitSyncPlan> = {}): GitSyncPlan {
  return {
    token: "t",
    config: { remote: "git@example.com:me/rules.git", branch: "master" },
    remoteCommit: "abc",
    changes: [],
    directories: [],
    conflicts: [],
    blockers: [],
    blocked: false,
    connectionChanged: false,
    ...overrides,
  };
}

const pull = { path: "profiles/work/AGENTS.md", localBefore: "a", localAfter: "b", remoteBefore: "b", remoteAfter: "b" };
const push = { path: "profiles/home/AGENTS.md", localBefore: "x", localAfter: "x", remoteBefore: "w", remoteAfter: "x" };

describe("background sync check", () => {
  it("counts each direction and conflicts separately", () => {
    const summary = summarizeSyncPlan(plan({
      changes: [pull, push],
      conflicts: [{ id: "profile:work", base: [], local: [], remote: [], canKeepBoth: true, resolved: false }],
    }));
    expect(summary).toEqual({ toLocal: 1, toRemote: 1, conflicts: 1, blocked: false });
    expect(isSyncPending(summary)).toBe(true);
  });

  it("treats a matching preview as clean and a blocker alone as pending", () => {
    expect(isSyncPending(summarizeSyncPlan(plan()))).toBe(false);
    expect(isSyncPending(summarizeSyncPlan(plan({ blockers: ["active Profile"] })))).toBe(true);
  });

  it("does not renotify for further edits to an already pending file", () => {
    const first = syncPendingKey(plan({ changes: [push] }));
    expect(syncPendingKey(plan({ changes: [{ ...push, localBefore: "y", localAfter: "y", remoteAfter: "y" }] }))).toBe(first);
    expect(syncPendingKey(plan({ changes: [push, pull] }))).not.toBe(first);
    expect(syncPendingKey(plan({ changes: [push], remoteCommit: "def" }))).not.toBe(first);
  });

  it("stays off unless explicitly enabled", () => {
    expect(parseSyncWatchEnabled(null)).toBe(false);
    expect(parseSyncWatchEnabled("off")).toBe(false);
    expect(parseSyncWatchEnabled("on")).toBe(true);
  });
});
