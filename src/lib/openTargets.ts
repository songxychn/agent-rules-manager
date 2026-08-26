import type { OpenTarget } from "./types";

export const DEFAULT_OPEN_TARGET_ID = "default";
export const OPEN_TARGET_STORAGE_KEY = "agent-rules-manager.open-target";

export function resolveOpenTarget(
  targets: readonly OpenTarget[],
  preferredId: string | null | undefined,
): OpenTarget | undefined {
  return (
    targets.find((target) => target.id === preferredId) ??
    targets.find((target) => target.id === DEFAULT_OPEN_TARGET_ID) ??
    targets[0]
  );
}

export function groupOpenTargets(targets: readonly OpenTarget[]) {
  return {
    editors: targets.filter(
      (target) => target.kind === "default" || target.kind === "editor",
    ),
    system: targets.filter(
      (target) => target.kind === "reveal" || target.kind === "terminal",
    ),
  };
}

export function readPreferredOpenTarget(): string {
  try {
    return window.localStorage.getItem(OPEN_TARGET_STORAGE_KEY) ?? DEFAULT_OPEN_TARGET_ID;
  } catch {
    return DEFAULT_OPEN_TARGET_ID;
  }
}

export function writePreferredOpenTarget(targetId: string): void {
  try {
    window.localStorage.setItem(OPEN_TARGET_STORAGE_KEY, targetId);
  } catch {
    // A disabled storage backend should not prevent opening the source file.
  }
}
