import type { GitSyncPlan } from "./types";

export const SYNC_WATCH_STORAGE_KEY = "agent-rules-manager.sync-watch.v1";
export const SYNC_WATCH_INTERVAL_MS = 15 * 60_000;
export const SYNC_WATCH_FOCUS_GAP_MS = 2 * 60_000;

export interface SyncWatchSummary {
  toLocal: number;
  toRemote: number;
  conflicts: number;
  blocked: boolean;
}

export type SyncWatchStatus =
  | { phase: "off" }
  | { phase: "idle" }
  | { phase: "unconfigured" }
  | { phase: "clean"; checkedAt: number }
  | { phase: "pending"; checkedAt: number; summary: SyncWatchSummary }
  | { phase: "error"; checkedAt: number; message: string };

export function summarizeSyncPlan(plan: GitSyncPlan): SyncWatchSummary {
  return {
    toLocal: plan.changes.filter((c) => c.localBefore !== c.localAfter).length,
    toRemote: plan.changes.filter((c) => c.remoteBefore !== c.remoteAfter).length,
    conflicts: plan.conflicts.length,
    blocked: plan.blockers.length > 0,
  };
}

export function isSyncPending(summary: SyncWatchSummary): boolean {
  return summary.toLocal + summary.toRemote + summary.conflicts > 0 || summary.blocked;
}

// Keyed by remote revision and affected paths, not content, so repeated saves of
// an already-pending file do not raise a new notification on every check.
export function syncPendingKey(plan: GitSyncPlan): string {
  return JSON.stringify([
    plan.remoteCommit,
    plan.changes.map((c) => [c.path, c.localBefore !== c.localAfter, c.remoteBefore !== c.remoteAfter]),
    plan.conflicts.map((c) => c.id),
    plan.blockers,
  ]);
}

export function parseSyncWatchEnabled(value: string | null): boolean {
  return value === "on";
}

export function readSyncWatchEnabled(): boolean {
  try {
    return parseSyncWatchEnabled(window.localStorage.getItem(SYNC_WATCH_STORAGE_KEY));
  } catch {
    return false;
  }
}

export function writeSyncWatchEnabled(enabled: boolean): void {
  try {
    window.localStorage.setItem(SYNC_WATCH_STORAGE_KEY, enabled ? "on" : "off");
  } catch {
    // The choice still applies for this session when storage is unavailable.
  }
}
