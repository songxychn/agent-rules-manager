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
  const { locale, preference, setPreference, systemLocale, t } = useI18n();
  const [updateView, setUpdateView] = useState<UpdateView>({ phase: "loading" });

  const optionCopy = (value: LanguagePreference) => {
    if (value === "system") {
      return {
        title: t("settings.language.system"),
        description: t("settings.language.systemDescription"),
      };
    }
    if (value === "zh-CN") {
      return {
        title: t("settings.language.zh"),
        description: t("settings.language.zhDescription"),
      };
    }
    return {
      title: t("settings.language.en"),
      description: t("settings.language.enDescription"),
    };
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

  const versionLabel = (version: string) => (
    <div className="language-output" aria-live="polite">
      <span>{t("settings.update.current")}</span>
      <strong>{version}</strong>
    </div>
  );

  return (
    <section className="settings-page" aria-labelledby="settings-heading">
      <header className="settings-masthead">
        <p className="eyebrow">{t("settings.eyebrow")}</p>
        <h1 id="settings-heading">{t("settings.title")}</h1>
        <p>{t("settings.intro")}</p>
      </header>

      <div className="settings-main-column">
        <div className="settings-routing-panel">
          <div className="settings-section-heading">
            <div>
              <p className="eyebrow">{t("settings.language.eyebrow")}</p>
              <h2>{t("settings.language.title")}</h2>
            </div>
            <p>{t("settings.language.description")}</p>
          </div>

          <div className="language-setting">
            <div className="language-field-copy">
              <label htmlFor="interface-language">{t("settings.language.group")}</label>
              <small>{optionCopy(preference).description}</small>
            </div>
            <div className="language-select-wrap">
              <select
                id="interface-language"
                value={preference}
                onChange={(event) => setPreference(event.target.value as LanguagePreference)}
              >
                {options.map((option) => (
                  <option value={option} key={option}>
                    {optionCopy(option).title}
                  </option>
                ))}
              </select>
            </div>
          </div>

          <div className="language-output" aria-live="polite">
            <span>{t("settings.language.output")}</span>
            <strong>{locale === "zh-CN" ? t("language.zh") : t("language.en")}</strong>
            {preference === "system" && (
              <small>
                {t("settings.language.systemDetected", {
                  language: systemLocale === "zh-CN" ? t("language.zh") : t("language.en"),
                })}
              </small>
            )}
          </div>
        </div>

        <div className="settings-routing-panel">
          <div className="settings-section-heading">
            <div>
              <p className="eyebrow">{t("settings.update.eyebrow")}</p>
              <h2>{t("settings.update.title")}</h2>
            </div>
            <p>{t("settings.update.description")}</p>
          </div>

          {updateView.phase === "loading" && (
            <div className="update-body">
              <p>{t("settings.update.checking")}</p>
            </div>
          )}

          {updateView.phase === "unavailable" && (
            <>
              {versionLabel(updateView.version)}
              <div className="update-body">
                <p>{t("settings.update.unavailable")}</p>
              </div>
            </>
          )}

          {updateView.phase === "checking" && (
            <>
              {versionLabel(updateView.version)}
              <div className="update-body">
                <p>{t("settings.update.checking")}</p>
              </div>
            </>
          )}

          {updateView.phase === "upToDate" && (
            <>
              {versionLabel(updateView.version)}
              <div className="update-body">
                <p>{t("settings.update.upToDate")}</p>
                <button className="button button-ghost" type="button" onClick={() => void refreshUpdate()}>
                  {t("settings.update.check")}
                </button>
              </div>
            </>
          )}

          {updateView.phase === "available" && (
            <>
              {versionLabel(updateView.check.currentVersion)}
              <div className="update-body">
                <p>{t("settings.update.available", { version: updateView.check.version })}</p>
                {updateView.check.notes && (
                  <details className="update-notes">
                    <summary>{t("settings.update.notes")}</summary>
                    <p>{updateView.check.notes}</p>
                  </details>
                )}
                <button
                  className="button button-primary"
                  type="button"
                  onClick={() => void installUpdate(updateView.check)}
                >
                  {t("settings.update.install")}
                </button>
              </div>
            </>
          )}

          {updateView.phase === "downloading" && (
            <>
              {versionLabel(updateView.check.currentVersion)}
              <div className="update-body">
                <p>
                  {updateView.percent === undefined
                    ? t("settings.update.downloadingUnknown")
                    : t("settings.update.downloading", { percent: updateView.percent })}
                </p>
                <div
                  className="update-progress"
                  role="progressbar"
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-valuenow={updateView.percent}
                >
                  <span style={{ width: `${updateView.percent ?? 15}%` }} />
                </div>
              </div>
            </>
          )}

          {updateView.phase === "installing" && (
            <div className="update-body">
              <p>{t("settings.update.installing")}</p>
            </div>
          )}

          {updateView.phase === "error" && (
            <>
              {versionLabel(updateView.version)}
              <div className="update-body">
                <p>
                  {t(updateView.install ? "settings.update.installError" : "settings.update.error", {
                    message: updateView.message,
                  })}
                </p>
                <button className="button button-primary" type="button" onClick={() => void refreshUpdate()}>
                  {t("settings.update.check")}
                </button>
              </div>
            </>
          )}
        </div>
      </div>

      <aside className="settings-boundary-note">
        <div className="boundary-glyph" aria-hidden="true">Aa</div>
        <div>
          <p className="eyebrow">{t("settings.safety.eyebrow")}</p>
          <h2>{t("settings.safety.title")}</h2>
          <p>{t("settings.safety.body")}</p>
          <code>{t("settings.storage")}</code>
        </div>
      </aside>
    </section>
  );
}
