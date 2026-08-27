import type { LibraryPlan, LibraryState } from "../lib/types";
import { useI18n } from "../lib/i18n";
import { LibraryChangeConfirm } from "./LibraryChangeConfirm";

interface LibrarySetupPanelProps {
  state: LibraryState;
  detail: string;
  legacySourcePath?: string;
  plan?: LibraryPlan;
  planning: boolean;
  applying: boolean;
  onPrepare: () => void;
  onApply: () => void;
  onCancel: () => void;
}

export function LibrarySetupPanel({
  state,
  detail,
  legacySourcePath,
  plan,
  planning,
  applying,
  onPrepare,
  onApply,
  onCancel,
}: LibrarySetupPanelProps) {
  const { t } = useI18n();
  const conflict = state === "conflict";
  const legacy = state === "legacy";
  const upgrade = state === "upgrade";
  const actionLabel = upgrade
    ? t("setup.startUpgrade")
    : legacy
      ? t("setup.startImport")
      : t("setup.startCreate");
  const confirmLabel = upgrade
    ? t("setup.confirmUpgrade")
    : legacy
      ? t("setup.confirmImport")
      : t("setup.confirmCreate");
  const confirmationTitle = upgrade
    ? t("setup.upgradeQuestion")
    : legacy
      ? t("setup.importQuestion")
      : t("setup.createQuestion");
  const confirmationDescription = upgrade
    ? t("setup.upgradePrompt")
    : legacy
      ? t("setup.importPrompt")
      : t("setup.createPrompt");

  return (
    <section className={`source-missing library-setup ${conflict ? "is-conflict" : ""}`}>
      <span className="setup-blueprint" aria-hidden="true">
        <i>P</i><i>→</i><i>C</i>
      </span>
      <p className="eyebrow">
        {conflict
          ? t("setup.conflictEyebrow")
          : upgrade
            ? t("setup.upgradeEyebrow")
            : legacy
              ? t("setup.legacyEyebrow")
              : t("setup.emptyEyebrow")}
      </p>
      <h1>
        {conflict
          ? t("setup.conflictTitle")
          : upgrade
            ? t("setup.upgradeTitle")
            : legacy
              ? t("setup.legacyTitle")
              : t("setup.emptyTitle")}
      </h1>
      <p>{detail}</p>
      {legacySourcePath && <code className="setup-legacy-path">{legacySourcePath}</code>}
      {!conflict && !plan && (
        <button className="button button-primary" disabled={planning} onClick={onPrepare}>
          {planning ? t("libraryPlan.preparing") : actionLabel}
        </button>
      )}
      {plan && (
        <LibraryChangeConfirm
          plan={plan}
          title={confirmationTitle}
          description={confirmationDescription}
          confirming={applying}
          confirmLabel={confirmLabel}
          onCancel={onCancel}
          onConfirm={onApply}
        />
      )}
    </section>
  );
}
