import type { LibraryPlan } from "../lib/types";
import { useI18n } from "../lib/i18n";
import { ConfirmDialog } from "./ConfirmDialog";

interface LibraryChangeConfirmProps {
  plan: LibraryPlan;
  tone?: "default" | "danger";
  embedded?: boolean;
  title: string;
  description: string;
  confirming: boolean;
  confirmLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export function LibraryChangeConfirm({
  plan,
  tone = "default",
  embedded = false,
  title,
  description,
  confirming,
  confirmLabel,
  onConfirm,
  onCancel,
}: LibraryChangeConfirmProps) {
  const { t } = useI18n();

  return (
    <ConfirmDialog
      title={plan.blocked ? t("libraryPlan.blocked") : title}
      description={plan.blocked ? plan.summary : description}
      tone={tone}
      blocked={plan.blocked || plan.changeCount === 0}
      embedded={embedded}
      confirming={confirming}
      confirmLabel={plan.blocked || plan.changeCount === 0 ? undefined : confirmLabel}
      cancelLabel={plan.blocked || plan.changeCount === 0 ? t("libraryPlan.close") : undefined}
      note={plan.blocked || plan.changeCount === 0
        ? t("libraryPlan.noChanges")
        : t("libraryPlan.snapshotNote")}
      onCancel={onCancel}
      onConfirm={onConfirm}
    >
      {plan.steps.length > 0 && (
        <details className="library-plan-details">
          <summary>{t("libraryPlan.details", { count: plan.steps.length })}</summary>
          <ol className="library-plan-steps">
            {plan.steps.map((step, index) => (
              <li key={`${step.path}-${step.action}`}>
                <span>{String(index + 1).padStart(2, "0")}</span>
                <code title={step.path}>{step.path}</code>
              </li>
            ))}
          </ol>
        </details>
      )}
    </ConfirmDialog>
  );
}
