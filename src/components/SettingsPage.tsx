import { useCallback, useEffect, useState } from "react";
import { useI18n, type LanguagePreference } from "../lib/i18n";
import {
  appVersion,
  checkForAppUpdate,
  desktopUpdatesAvailable,
  downloadPercent,
  relaunchApp,
  type AppUpdateCheck,
} from "../lib/appUpdate";

const options: LanguagePreference[] = ["system", "zh-CN", "en"];

type UpdateView =
  | { phase: "loading" }
  | { phase: "unavailable"; version: string }
  | { phase: "checking"; version: string }
  | { phase: "upToDate"; version: string }
  | { phase: "available"; check: Extract<AppUpdateCheck, { status: "available" }> }
  | { phase: "downloading"; check: Extract<AppUpdateCheck, { status: "available" }>; percent?: number }
  | { phase: "installing"; version: string }
  | { phase: "error"; version: string; message: string; install?: boolean };

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function SettingsPage() {
  const { preference, setPreference, systemLocale, t } = useI18n();
  const [updateView, setUpdateView] = useState<UpdateView>({ phase: "loading" });

  const languageTitle = (value: LanguagePreference) => {
    if (value === "system") return t("settings.language.system");
    if (value === "zh-CN") return t("settings.language.zh");
    return t("settings.language.en");
  };

  const refreshUpdate = useCallback(async () => {
    const version = await appVersion();
    if (!desktopUpdatesAvailable()) {
      setUpdateView({ phase: "unavailable", version });
      return;
    }
    setUpdateView({ phase: "checking", version });
    try {
      const check = await checkForAppUpdate();
      if (check.status === "upToDate") {
        setUpdateView({ phase: "upToDate", version: check.currentVersion });
        return;
      }
      if (check.status === "available") {
        setUpdateView({ phase: "available", check });
        return;
      }
      setUpdateView({ phase: "unavailable", version });
    } catch (error) {
      setUpdateView({ phase: "error", version, message: errorMessage(error) });
    }
  }, []);

  useEffect(() => {
    void refreshUpdate();
  }, [refreshUpdate]);

  const installUpdate = async (check: Extract<AppUpdateCheck, { status: "available" }>) => {
    setUpdateView({ phase: "downloading", check });
    let downloaded = 0;
    let total: number | undefined;
    try {
      await check.downloadAndInstall((event) => {
        if (event.event === "Started") {
          total = event.data.contentLength;
          downloaded = 0;
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
        }
        setUpdateView({
          phase: "downloading",
          check,
          percent: downloadPercent(downloaded, total),
        });
      });
      setUpdateView({ phase: "installing", version: check.version });
      if (!/windows/i.test(navigator.userAgent)) {
        await relaunchApp();
      }
    } catch (error) {
      setUpdateView({
        phase: "error",
        version: check.currentVersion,
        message: errorMessage(error),
        install: true,
      });
    }
  };

  const version =
    updateView.phase === "available" || updateView.phase === "downloading"
      ? updateView.check.currentVersion
      : updateView.phase === "loading"
        ? undefined
        : updateView.version;

  let updateStatus = t("settings.update.checking");
  let updateAction: { label: string; onClick: () => void; primary?: boolean } | undefined;
  if (updateView.phase === "unavailable") {
    updateStatus = t("settings.update.unavailable");
  } else if (updateView.phase === "upToDate") {
    updateStatus = t("settings.update.upToDate");
    updateAction = { label: t("settings.update.check"), onClick: () => void refreshUpdate() };
  } else if (updateView.phase === "available") {
    updateStatus = t("settings.update.available", { version: updateView.check.version });
    updateAction = {
      label: t("settings.update.install"),
      onClick: () => void installUpdate(updateView.check),
      primary: true,
    };
  } else if (updateView.phase === "downloading") {
    updateStatus =
      updateView.percent === undefined
        ? t("settings.update.downloadingUnknown")
        : t("settings.update.downloading", { percent: updateView.percent });
  } else if (updateView.phase === "installing") {
    updateStatus = t("settings.update.installing");
  } else if (updateView.phase === "error") {
    updateStatus = t(updateView.install ? "settings.update.installError" : "settings.update.error", {
      message: updateView.message,
    });
    updateAction = {
      label: t("settings.update.check"),
      onClick: () => void refreshUpdate(),
      primary: true,
    };
  }

  return (
    <section className="settings-page" aria-labelledby="settings-heading">
      <header className="settings-masthead">
        <p className="eyebrow">{t("settings.eyebrow")}</p>
        <h1 id="settings-heading">{t("settings.title")}</h1>
        <p>{t("settings.intro")}</p>
      </header>

      <div className="settings-panel">
        <div className="settings-row">
          <div className="settings-row-copy">
            <label htmlFor="interface-language">{t("settings.language.title")}</label>
            <p id="language-note">{t("settings.safety.body")}</p>
            {preference === "system" && (
              <small>
                {t("settings.language.systemDetected", {
                  language: systemLocale === "zh-CN" ? t("language.zh") : t("language.en"),
                })}
              </small>
            )}
          </div>
          <div className="language-select-wrap">
            <select
              id="interface-language"
              value={preference}
              aria-describedby="language-note"
              onChange={(event) => setPreference(event.target.value as LanguagePreference)}
            >
              {options.map((option) => (
                <option value={option} key={option}>
                  {languageTitle(option)}
                </option>
              ))}
            </select>
          </div>
        </div>

        <div className="settings-row">
          <div className="settings-row-copy">
            <strong>{t("settings.update.title")}</strong>
            <p aria-live="polite">{updateStatus}</p>
            {updateView.phase === "available" && updateView.check.notes && (
              <details className="update-notes">
                <summary>{t("settings.update.notes")}</summary>
                <p>{updateView.check.notes}</p>
              </details>
            )}
            {updateView.phase === "downloading" && (
              <div
                className="update-progress"
                role="progressbar"
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={updateView.percent}
              >
                <span style={{ width: `${updateView.percent ?? 15}%` }} />
              </div>
            )}
          </div>
          <div className="settings-row-actions">
            {version && <span className="settings-version">{version}</span>}
            {updateAction && (
              <button
                className={`button ${updateAction.primary ? "button-primary" : "button-ghost"} button-small`}
                type="button"
                onClick={updateAction.onClick}
              >
                {updateAction.label}
              </button>
            )}
          </div>
        </div>
      </div>
    </section>
  );
}
