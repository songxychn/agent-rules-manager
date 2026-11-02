import { useCallback, useEffect, useRef, useState } from "react";
import { backend } from "./backend";
import {
  SYNC_WATCH_FOCUS_GAP_MS,
  SYNC_WATCH_INTERVAL_MS,
  isSyncPending,
  summarizeSyncPlan,
  syncPendingKey,
  type SyncWatchStatus,
  type SyncWatchSummary,
} from "./syncWatch";

const SYNC_WATCH_TICK_MS = 60_000;

/**
 * Read-only remote check (L0): fetches the saved connection and builds a sync
 * preview. It never applies a plan, writes rule sources or pushes.
 */
export function useSyncWatch({ libraryRoot, enabled, paused, onPending, onError }: {
  libraryRoot: string;
  enabled: boolean;
  paused: boolean;
  onPending: (summary: SyncWatchSummary) => void;
  onError: (message: string) => void;
}) {
  const active = enabled && Boolean(libraryRoot);
  const [status, setStatus] = useState<SyncWatchStatus>({ phase: active ? "idle" : "off" });
  const [checking, setChecking] = useState(false);
  const generation = useRef(0);
  const inFlight = useRef<number>();
  const lastRun = useRef(0);
  const lastKey = useRef<string>();
  const callbacks = useRef({ onPending, onError });
  callbacks.current = { onPending, onError };

  useEffect(() => {
    generation.current++;
    inFlight.current = undefined;
    lastRun.current = 0;
    lastKey.current = undefined;
    setChecking(false);
    setStatus({ phase: active ? "idle" : "off" });
  }, [active, libraryRoot]);

  const check = useCallback(async () => {
    const current = generation.current;
    if (!active || inFlight.current === current) return;
    inFlight.current = current;
    lastRun.current = Date.now();
    setChecking(true);
    const report = (key: string | undefined, notice?: () => void) => {
      if (key !== lastKey.current && notice) notice();
      lastKey.current = key;
    };
    try {
      const config = await backend.gitSyncConfig(libraryRoot);
      if (current !== generation.current) return;
      if (!config) {
        setStatus({ phase: "unconfigured" });
        report(undefined);
        return;
      }
      const plan = await backend.previewGitSync({ ...config, resolutions: [] }, libraryRoot);
      if (current !== generation.current) return;
      const summary = summarizeSyncPlan(plan);
      const checkedAt = Date.now();
      if (isSyncPending(summary)) {
        setStatus({ phase: "pending", checkedAt, summary });
        report(syncPendingKey(plan), () => callbacks.current.onPending(summary));
      } else {
        setStatus({ phase: "clean", checkedAt });
        report(undefined);
      }
    } catch (error) {
      if (current !== generation.current) return;
      const message = String(error);
      setStatus({ phase: "error", checkedAt: Date.now(), message });
      report(`error:${message}`, () => callbacks.current.onError(message));
    } finally {
      if (inFlight.current === current) {
        inFlight.current = undefined;
        setChecking(false);
      }
    }
  }, [active, libraryRoot]);

  useEffect(() => {
    if (!active || paused) return;
    const runAfter = (gap: number) => {
      if (Date.now() - lastRun.current >= gap) void check();
    };
    runAfter(SYNC_WATCH_FOCUS_GAP_MS);
    const timer = window.setInterval(() => runAfter(SYNC_WATCH_INTERVAL_MS), SYNC_WATCH_TICK_MS);
    const onFocus = () => runAfter(SYNC_WATCH_FOCUS_GAP_MS);
    window.addEventListener("focus", onFocus);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener("focus", onFocus);
    };
  }, [active, paused, check]);

  // After a manual sync the previous result is obsolete; the next scheduled or
  // resumed check runs immediately and may notify again.
  const invalidate = useCallback(() => {
    lastRun.current = 0;
    lastKey.current = undefined;
  }, []);

  return { status, checking, check, invalidate };
}
