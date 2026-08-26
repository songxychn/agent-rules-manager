import type { LibraryPlan } from "../lib/types";
import { useI18n } from "../lib/i18n";

interface LibraryPlanPreviewProps {
  plan: LibraryPlan;
  confirming: boolean;
  confirmLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export function LibraryPlanPreview({
  plan,
  confirming,
  confirmLabel,
  onConfirm,
  onCancel,
}: LibraryPlanPreviewProps) {
  const { t } = useI18n();

  return (
    <section className={`library-plan ${plan.blocked ? "is-blocked" : ""}`}>
      <div className="library-plan-heading">
        <span className="library-plan-stamp" aria-hidden="true">
          {plan.blocked ? "!" : "PREVIEW"}
        </span>
        <div>
          <strong>{plan.blocked ? t("libraryPlan.blocked") : t("libraryPlan.ready")}</strong>
          <p>{plan.summary}</p>
        </div>
      </div>

      {plan.steps.length > 0 && (
        <ol className="library-plan-steps">
          {plan.steps.map((step, index) => (
            <li key={`${step.path}-${step.action}`}>
              <span>{String(index + 1).padStart(2, "0")}</span>
              <div>
                <strong>{step.action}</strong>
                <code title={step.path}>{step.path}</code>
                <small>{step.summary}</small>
              </div>
            </li>
          ))}
        </ol>
      )}

      <footer>
        <span>{t("libraryPlan.changeCount", { count: plan.changeCount })}</span>
        <div>
          <button className="button button-ghost button-small" onClick={onCancel}>
            {t("libraryPlan.cancel")}
          </button>
          <button
            className="button button-primary button-small"
            disabled={plan.blocked || plan.changeCount === 0 || confirming}
            onClick={onConfirm}
          >
            {confirming ? t("libraryPlan.applying") : confirmLabel}
          </button>
        </div>
      </footer>
    </section>
  );
}
