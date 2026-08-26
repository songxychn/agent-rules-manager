import type { TargetState } from "../lib/types";
import { useI18n } from "../lib/i18n";

type DisplayState = TargetState | "unavailable";

export function StatusPill({ state }: { state: DisplayState }) {
  const { t } = useI18n();
  const labels: Record<DisplayState, string> = {
    inSync: t("status.inSync"),
    ready: t("status.ready"),
    drifted: t("status.drifted"),
    conflict: t("status.conflict"),
    unavailable: t("status.unavailable"),
  };
  return <span className={`status-pill status-${state}`}>{labels[state]}</span>;
}
