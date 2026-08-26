import type { AgentStatus, ProjectionPlan } from "../lib/types";
import { useI18n } from "../lib/i18n";
import { StatusPill } from "./StatusPill";

interface ProjectionRailProps {
  agents: AgentStatus[];
  sourceDigest?: string;
  plan?: ProjectionPlan;
  loading: boolean;
  applying: boolean;
  onInspect: () => void;
  onApply: () => void;
}

export function ProjectionRail({
  agents,
  sourceDigest,
  plan,
  loading,
  applying,
  onInspect,
  onApply,
}: ProjectionRailProps) {
  const { t } = useI18n();
  const attentionCount = agents.filter((agent) => agent.state !== "inSync").length;
  const conflictCount = agents.filter((agent) => agent.state === "conflict").length;

  const stepSummary = (action: string) => {
    if (action === "blocked") return t("projection.step.conflict");
    if (action === "addInclude") return t("projection.step.addInclude");
    if (action === "createLink") return t("projection.step.createLink");
    if (action === "refreshManagedBlock") {
      return t("projection.step.refreshManagedBlock");
    }
    return t("projection.step.change");
  };

  let statusTone = "success";
  let statusTitle = t("projection.syncActiveTitle");
  let statusBody = t("projection.syncActiveBody");

  if (!plan && conflictCount > 0) {
    statusTone = "danger";
    statusTitle = t("projection.conflictDetectedTitle", { count: conflictCount });
    statusBody = t("projection.conflictDetectedBody");
  } else if (!plan && attentionCount > 0) {
    statusTone = "attention";
    statusTitle = t("projection.attentionTitle", { count: attentionCount });
    statusBody = t("projection.attentionBody");
  } else if (plan?.blocked) {
    statusTone = "danger";
    statusTitle = t("projection.blockedTitle");
    statusBody = t("projection.blockedBody");
  } else if (plan && plan.changeCount > 0) {
    statusTone = "attention";
    statusTitle = t("projection.connectionChangesReady", { count: plan.changeCount });
    statusBody = t("projection.connectionChangesBody");
  }

  return (
    <section className="projection-panel" aria-labelledby="projection-heading">
      <div className="section-heading projection-heading-row">
        <div>
          <p className="eyebrow">{t("projection.eyebrow")}</p>
          <h2 id="projection-heading">{t("projection.title")}</h2>
        </div>
        <p className="projection-hint">{t("projection.hint")}</p>
      </div>

      <div className="projection-map">
        <div className="source-node">
          <span className="source-node-icon" aria-hidden="true">
            <span>MD</span>
          </span>
          <span className="source-node-copy">
            <span className="node-kicker">{t("projection.canonical")}</span>
            <strong>AGENTS.md</strong>
            <code>{sourceDigest ?? t("projection.uninitialized")}</code>
          </span>
          <span className="source-port" aria-hidden="true" />
        </div>

        <ul className="rail-stack" aria-label={t("projection.targets")}>
          {agents.map((agent) => {
            const step = plan?.steps.find((candidate) => candidate.agentId === agent.id);
            const previewedAction = step && step.action !== "none";

            return (
              <li className={`agent-rail rail-${agent.state}`} key={agent.id}>
                <span className="rail-line" aria-hidden="true">
                  <span className="rail-flow" />
                </span>
                <article className="agent-node">
                  <span className={`agent-mark agent-mark-${agent.id}`} aria-hidden="true">
                    {agent.label.slice(0, 2).toUpperCase()}
                  </span>
                  <span className="agent-copy">
                    <span className="agent-title-row">
                      <strong>{agent.label}</strong>
                    </span>
                    <code title={agent.targetPath}>{agent.targetPath}</code>
                    <small className="agent-mode">
                      {agent.mode === "include"
                        ? t("projection.managedInclude")
                        : t("projection.symbolicLink")}
                    </small>
                    {previewedAction && (
                      <small className={`agent-action-preview action-${step.action}`}>
                        {stepSummary(step.action)}
                      </small>
                    )}
                  </span>
                  <span className="agent-node-side">
                    <StatusPill state={agent.state} />
                  </span>
                </article>
              </li>
            );
          })}
        </ul>
      </div>

      <footer
        className={`projection-status-bar tone-${statusTone}`}
        aria-live="polite"
      >
        <span className="projection-status-glyph" aria-hidden="true">
          {statusTone === "success" ? "✓" : statusTone === "danger" ? "!" : "↗"}
        </span>
        <span className="projection-status-copy">
          <strong>{statusTitle}</strong>
          <span>{statusBody}</span>
        </span>
        {attentionCount > 0 && (
          <span className="projection-status-actions">
            <button
              className="button button-secondary button-small"
              onClick={onInspect}
              disabled={loading || applying}
            >
              {loading
                ? t("projection.inspecting")
                : plan
                  ? t("projection.recheck")
                  : t("projection.inspect")}
            </button>
            {plan && !plan.blocked && plan.changeCount > 0 && (
              <button
                className="button button-primary button-small"
                onClick={onApply}
                disabled={applying}
              >
                {applying
                  ? t("projection.applying")
                  : plan.changeCount === 1
                    ? t("projection.connectOne")
                    : t("projection.connectMany", { count: plan.changeCount })}
              </button>
            )}
          </span>
        )}
      </footer>
    </section>
  );
}
