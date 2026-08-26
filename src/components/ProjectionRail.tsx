import type { AgentStatus, ProjectionPlan, RuntimeState } from "../lib/types";
import { useI18n } from "../lib/i18n";
import { StatusPill } from "./StatusPill";

interface ProjectionRailProps {
  agents: AgentStatus[];
  sourceDigest?: string;
  activeProfileName?: string;
  runtimeState: RuntimeState;
  plan?: ProjectionPlan;
  desiredConnections: Set<string>;
  pendingCount: number;
  loading: boolean;
  applying: boolean;
  onToggle: (agentId: string, connected: boolean) => void;
  onInspect: () => void;
  onApply: () => void;
}

export function ProjectionRail({
  agents,
  sourceDigest,
  activeProfileName,
  runtimeState,
  plan,
  desiredConnections,
  pendingCount,
  loading,
  applying,
  onToggle,
  onInspect,
  onApply,
}: ProjectionRailProps) {
  const { t } = useI18n();
  const detectedAgents = agents.filter((agent) => agent.installed || agent.connected);
  const connectedCount = detectedAgents.filter((agent) => agent.connected).length;
  const conflictCount = detectedAgents.filter((agent) => agent.state === "conflict").length;
  const orderedAgents = [...agents].sort(
    (left, right) => Number(right.installed || right.connected) - Number(left.installed || left.connected),
  );

  const stepSummary = (action: string) => {
    if (action === "blocked") return t("projection.step.conflict");
    if (action === "createLink") return t("projection.step.createLink");
    if (action === "createIndependentFile") return t("projection.step.createIndependentFile");
    if (action === "detachManagedInclude") return t("projection.step.detachManagedInclude");
    if (action === "migrateLegacyInclude") return t("projection.step.migrateLegacyInclude");
    if (action === "refreshManagedBlock") {
      return t("projection.step.refreshManagedBlock");
    }
    if (action === "replaceLegacyLink") return t("projection.step.replaceLegacyLink");
    return t("projection.step.change");
  };

  const targetDetail = (agent: AgentStatus) => {
    if (agent.targetKind === "missing") return t("projection.detail.missing");
    if (agent.targetKind === "connectedLink") return t("projection.detail.connectedLink");
    if (agent.targetKind === "legacyLink") return t("projection.detail.legacyLink");
    if (agent.targetKind === "independentFile") return t("projection.detail.independentFile");
    if (agent.targetKind === "legacyInclude") return t("projection.detail.legacyInclude");
    if (agent.targetKind === "foreignLink") return t("projection.detail.foreignLink");
    return t("projection.detail.invalidManagedFile");
  };

  let statusTone = "success";
  let statusTitle = t("projection.detectedTitle", {
    detected: detectedAgents.length,
    connected: connectedCount,
  });
  let statusBody = t("projection.detectedBody");

  if (plan?.blocked) {
    statusTone = "danger";
    statusTitle = t("projection.blockedTitle");
    statusBody = t("projection.blockedBody");
  } else if (plan && plan.changeCount > 0) {
    statusTone = "attention";
    statusTitle = t("projection.connectionChangesReady", { count: plan.changeCount });
    statusBody = t("projection.connectionChangesBody");
  } else if (pendingCount > 0) {
    statusTone = "attention";
    statusTitle = t("projection.selectionPendingTitle", { count: pendingCount });
    statusBody = t("projection.selectionPendingBody");
  } else if (conflictCount > 0) {
    statusTone = "danger";
    statusTitle = t("projection.conflictDetectedTitle", { count: conflictCount });
    statusBody = t("projection.conflictDetectedBody");
  } else if (detectedAgents.length === 0) {
    statusTone = "attention";
    statusTitle = t("projection.noneDetectedTitle");
    statusBody = t("projection.noneDetectedBody");
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
            <strong>{activeProfileName ?? t("projection.uninitialized")}</strong>
            <code>current/AGENTS.md · {sourceDigest ?? runtimeState}</code>
          </span>
          <span className="source-port" aria-hidden="true" />
        </div>

        <ul className="rail-stack" aria-label={t("projection.targets")}>
          {orderedAgents.map((agent) => {
            const available = agent.installed || agent.connected;
            const desiredConnected = desiredConnections.has(agent.id);
            const pending =
              desiredConnected !== agent.connected ||
              (desiredConnected && agent.state === "drifted");
            const step = plan?.steps.find((candidate) => candidate.agentId === agent.id);
            const previewedAction = step && step.action !== "none";
            const connectionLabel = !available
              ? t("projection.notDetected")
              : desiredConnected
                ? t("projection.centralLink")
                : agent.targetKind === "missing"
                  ? t("projection.independentEmpty")
                  : t("projection.independentFile");

            return (
              <li
                className={`agent-rail rail-${agent.state} ${pending ? "is-pending" : ""} ${
                  available ? "" : "is-unavailable"
                }`}
                key={agent.id}
              >
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
                      {available && (
                        <span className="detected-badge" title={agent.detectionDetail}>
                          {t("projection.detected")}
                        </span>
                      )}
                    </span>
                    <code title={agent.targetPath}>{agent.targetPath}</code>
                    <small className="agent-mode">{connectionLabel}</small>
                    {available && <small className="agent-detail">{targetDetail(agent)}</small>}
                    {previewedAction && (
                      <small className={`agent-action-preview action-${step.action}`}>
                        {stepSummary(step.action)}
                      </small>
                    )}
                  </span>
                  <span className="agent-node-side">
                    <StatusPill state={available ? agent.state : "unavailable"} />
                    <button
                      type="button"
                      className={`connection-switch ${desiredConnected ? "is-on" : ""}`}
                      role="switch"
                      aria-checked={desiredConnected}
                      aria-label={t("projection.toggleLabel", { agent: agent.label })}
                      disabled={!available || loading || applying}
                      onClick={() => onToggle(agent.id, !desiredConnected)}
                    >
                      <span className="switch-track" aria-hidden="true">
                        <span className="switch-knob" />
                      </span>
                      <span>{desiredConnected ? t("projection.joined") : t("projection.independent")}</span>
                    </button>
                  </span>
                </article>
              </li>
            );
          })}
        </ul>
      </div>

      <footer className={`projection-status-bar tone-${statusTone}`} aria-live="polite">
        <span className="projection-status-glyph" aria-hidden="true">
          {statusTone === "success" ? "✓" : statusTone === "danger" ? "!" : "↗"}
        </span>
        <span className="projection-status-copy">
          <strong>{statusTitle}</strong>
          <span>{statusBody}</span>
        </span>
        {pendingCount > 0 && (
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
                    ? t("projection.applyOne")
                    : t("projection.applyMany", { count: plan.changeCount })}
              </button>
            )}
          </span>
        )}
      </footer>
    </section>
  );
}
