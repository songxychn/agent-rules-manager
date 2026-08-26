import type { TargetState } from "../lib/types";
import { useI18n } from "../lib/i18n";

export function StatusPill({ state }: { state: TargetState }) {
  const { t } = useI18n();
  const labels: Record<TargetState, string> = {
    inSync: t("status.inSync"),
    ready: t("status.ready"),
    drifted: t("status.drifted"),
    conflict: t("status.conflict"),
  };
  return <span className={`status-pill status-${state}`}>{labels[state]}</span>;
}
