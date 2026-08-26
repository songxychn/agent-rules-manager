import { useI18n, type LanguagePreference } from "../lib/i18n";

const options: LanguagePreference[] = ["system", "zh-CN", "en"];

export function SettingsPage() {
  const { locale, preference, setPreference, systemLocale, t } = useI18n();

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

  return (
    <section className="settings-page" aria-labelledby="settings-heading">
      <header className="settings-masthead">
        <p className="eyebrow">{t("settings.eyebrow")}</p>
        <h1 id="settings-heading">{t("settings.title")}</h1>
        <p>{t("settings.intro")}</p>
      </header>

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
