import type { DownloadEvent } from "@tauri-apps/plugin-updater";

const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export type AppUpdateCheck =
  | { status: "unavailable" }
  | { status: "upToDate"; currentVersion: string }
  | {
      status: "available";
      currentVersion: string;
      version: string;
      notes?: string;
      downloadAndInstall: (onEvent?: (event: DownloadEvent) => void) => Promise<void>;
    };

export function desktopUpdatesAvailable(): boolean {
  return isTauri;
}

export async function appVersion(): Promise<string> {
  if (!isTauri) return "0.1.1";
  const { getVersion } = await import("@tauri-apps/api/app");
  return getVersion();
}

export async function checkForAppUpdate(): Promise<AppUpdateCheck> {
  if (!isTauri) return { status: "unavailable" };
  const { check } = await import("@tauri-apps/plugin-updater");
  const currentVersion = await appVersion();
  const update = await check();
  if (!update) return { status: "upToDate", currentVersion };
  return {
    status: "available",
    currentVersion: update.currentVersion,
    version: update.version,
    notes: update.body ?? undefined,
    downloadAndInstall: (onEvent) => update.downloadAndInstall(onEvent),
  };
}

export async function relaunchApp(): Promise<void> {
  const { relaunch } = await import("@tauri-apps/plugin-process");
  await relaunch();
}

export function downloadPercent(downloaded: number, total?: number): number | undefined {
  if (!total || total <= 0) return undefined;
  return Math.min(100, Math.round((downloaded / total) * 100));
}
