import type { LibraryPlan, LibraryState } from "../lib/types";
import { useI18n } from "../lib/i18n";
import { LibraryPlanPreview } from "./LibraryPlanPreview";

interface LibrarySetupPanelProps {
  state: LibraryState;
  detail: string;
  legacySourcePath?: string;
  plan?: LibraryPlan;
  planning: boolean;
  applying: boolean;
  onPreview: () => void;
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
  onPreview,
  onApply,
  onCancel,
}: LibrarySetupPanelProps) {
  const { t } = useI18n();
  const conflict = state === "conflict";
  const legacy = state === "legacy";

  return (
    <section className={`source-missing library-setup ${conflict ? "is-conflict" : ""}`}>
      <span className="setup-blueprint" aria-hidden="true">
        <i>P</i><i>+</i><i>P</i><i>→</i><i>C</i>
      </span>
      <p className="eyebrow">
        {conflict ? t("setup.conflictEyebrow") : legacy ? t("setup.legacyEyebrow") : t("setup.emptyEyebrow")}
      </p>
      <h1>{conflict ? t("setup.conflictTitle") : legacy ? t("setup.legacyTitle") : t("setup.emptyTitle")}</h1>
      <p>{detail}</p>
      {legacySourcePath && <code className="setup-legacy-path">{legacySourcePath}</code>}
      {!conflict && !plan && (
        <button className="button button-primary" disabled={planning} onClick={onPreview}>
          {planning ? t("libraryPlan.previewing") : t("setup.preview")}
        </button>
      )}
      {plan && (
        <LibraryPlanPreview
          plan={plan}
          confirming={applying}
          confirmLabel={legacy ? t("setup.confirmImport") : t("setup.confirmCreate")}
          onCancel={onCancel}
          onConfirm={onApply}
        />
      )}
    </section>
  );
}
