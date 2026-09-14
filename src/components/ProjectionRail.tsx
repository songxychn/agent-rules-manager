import { useState } from "react";
import { agentNote } from "../lib/agentCatalog";
import type { AgentStatus, ProjectionPlan, RuntimeState } from "../lib/types";
import { useI18n } from "../lib/i18n";
import { ConfirmDialog } from "./ConfirmDialog";
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
  onContinue: () => void;
  onApply: () => void;
  onCancelConfirmation: () => void;
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
  onContinue,
  onApply,
  onCancelConfirmation,
}: ProjectionRailProps) {
  const { t, locale } = useI18n();
  const zh = locale === "zh-CN";
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<"all" | "available" | "connected">("available");
  const isAvailable = (agent: AgentStatus) => agent.installed || agent.connected || agent.scope === "project";
  const availableCount = agents.filter(isAvailable).length;
  const detectedAgents = agents.filter((agent) => agent.installed || agent.connected);
  const connectedCount = detectedAgents.filter((agent) => agent.connected).length;
  const conflictCount = detectedAgents.filter((agent) => (agent.state === "conflict" || agent.warning)).length;
  const orderedAgents = [...agents].sort(
    (left, right) => Number(right.installed || right.connected) - Number(left.installed || left.connected),
  );

  const visibleAgents = orderedAgents.filter((agent) => {
    const matchesFilter = filter === "all" || (filter === "available" ? isAvailable(agent) : agent.connected);
    const matchesQuery = `${agent.label} ${agent.id} ${agent.targetPath}`.toLowerCase().includes(query.trim().toLowerCase());
    return matchesFilter && matchesQuery;
  });

  const stepSummary = (action: string) => {
    if (action === "blocked") return t("projection.step.conflict");
    if (action === "backupAndCreateLink") return t("projection.step.backupAndCreateLink");
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
  } else if (plan && plan.confirmationCount > 0) {
    statusTone = "attention";
    statusTitle = t("projection.backupConfirmationTitle", {
      count: plan.confirmationCount,
    });
    statusBody = t("projection.backupConfirmationBody");
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
  const confirmationNeedsBackup = (plan?.confirmationCount ?? 0) > 0;

  return (
    <section className="agent-dashboard" aria-label={t("projection.targets")}>
      <div className="agent-dashboard-summary">
        <span>{t("projection.canonical")} <strong>{activeProfileName ?? t("projection.uninitialized")}</strong></span>
        <span title={`current/AGENTS.md · ${sourceDigest ?? runtimeState}`}>{statusTitle}</span>
      </div>
      <div className="agent-grid-toolbar">
        <input
          type="search"
          aria-label={zh ? "搜索 Agent" : "Search agents"}
          placeholder={zh ? "搜索 Agent…" : "Search agents…"}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
        <div className="agent-grid-filters" role="group" aria-label={zh ? "筛选 Agent" : "Filter agents"}>
          {([
            ["all", zh ? "全部" : "All", agents.length],
            ["available", zh ? "本机" : "Available", availableCount],
            ["connected", zh ? "已接入" : "Connected", connectedCount],
          ] as const).map(([value, label, count]) => (
            <button key={value} type="button" aria-pressed={filter === value} onClick={() => setFilter(value)}>
              {label} <span>{count}</span>
            </button>
          ))}
        </div>
      </div>

      {(pendingCount > 0 || statusTone === "danger" || plan) && (
        <div className={`agent-grid-notice tone-${statusTone}`} role="status">
          <span><strong>{statusTitle}</strong><small>{statusBody}</small></span>
          {pendingCount > 0 && !plan && (
            <button className="button button-primary button-small" onClick={onContinue} disabled={loading || applying}>
              {loading ? t("projection.inspecting") : t("projection.inspect")}
            </button>
          )}
        </div>
      )}

      <ul className="agent-card-grid" aria-label={t("projection.targets")}>
        {visibleAgents.map((agent) => {
          const available = agent.installed || agent.connected;
          const desiredConnected = desiredConnections.has(agent.id);
          const pending = desiredConnected !== agent.connected || (desiredConnected && agent.state === "drifted");
          const step = plan?.steps.find((candidate) => candidate.agentId === agent.id);
          return (
            <li className={`agent-card ${pending ? "is-pending" : ""} ${agent.warning || agent.state === "conflict" ? "has-warning" : ""}`} key={agent.id}>
              <div className="agent-card-heading">
                <span className={`agent-state-dot ${agent.connected ? "is-connected" : ""}`} aria-hidden="true" />
                <strong title={agent.label}>{agent.label}</strong>
                <button
                  type="button"
                  className={`connection-switch ${desiredConnected ? "is-on" : ""}`}
                  role="switch"
                  aria-checked={desiredConnected}
                  aria-label={t("projection.toggleLabel", { agent: agent.label })}
                  disabled={(!available && agent.scope !== "project") || loading || applying}
                  onClick={() => onToggle(agent.id, !desiredConnected)}
                >
                  <span className="switch-track" aria-hidden="true"><span className="switch-knob" /></span>
                </button>
              </div>
              <code className="agent-card-path" title={agent.targetPath}>{agent.targetPath}</code>
              {pending && <small className="agent-card-pending">{zh ? "待保存" : "Unsaved"} · {desiredConnected ? t("projection.joined") : t("projection.independent")}</small>}
              {(agent.warning || agent.state === "conflict") && <small className="agent-card-warning">{agent.warning
                ? zh ? `规则超过 ${agent.maxChars?.toLocaleString()} 字符限制，请精简 Profile。` : agent.warning
                : targetDetail(agent)}</small>}
              {step && step.action !== "none" && <small className="agent-card-pending">{stepSummary(step.action)}</small>}
              <div className="agent-card-footer">
                <StatusPill state={available ? agent.state : "unavailable"} />
                <details className="agent-card-details">
                  <summary aria-label={`${agent.label} · ${zh ? "详情" : "Details"}`}>{zh ? "详情" : "Details"}</summary>
                  <div>
                    <code>{agent.targetPath}</code>
                    <p>{targetDetail(agent)}</p>
                    {available && <p>{t("projection.detected")}</p>}
                    {agent.note && <p>{agentNote(agent, zh)}</p>}
                    {agent.docsUrl && <a href={agent.docsUrl} target="_blank" rel="noreferrer">{t("projection.ruleDocs")}</a>}
                  </div>
                </details>
              </div>
            </li>
          );
        })}
      </ul>
      {visibleAgents.length === 0 && <div className="agent-grid-empty" role="status">
        <p>{zh ? "没有符合条件的 Agent" : "No matching agents"}</p>
        <button className="button button-secondary button-small" onClick={() => { setFilter("all"); setQuery(""); }}>{zh ? "查看全部" : "View all"}</button>
      </div>}

      {plan && (
        <ConfirmDialog
          title={plan.blocked
            ? t("projection.blockedTitle")
            : confirmationNeedsBackup
              ? t("projection.backupConfirmationTitle", { count: plan.confirmationCount })
              : t("projection.connectionChangesReady", { count: plan.changeCount })}
          description={plan.blocked
            ? t("projection.blockedBody")
            : confirmationNeedsBackup
              ? t("projection.backupConfirmationBody")
              : t("projection.connectionChangesBody")}
          tone={plan.blocked ? "danger" : "default"}
          blocked={plan.blocked || plan.changeCount === 0}
          confirming={applying}
          confirmLabel={plan.blocked || plan.changeCount === 0
            ? undefined
            : confirmationNeedsBackup
              ? plan.changeCount === 1
                ? t("projection.backupAndApplyOne")
                : t("projection.backupAndApplyMany", { count: plan.changeCount })
              : plan.changeCount === 1
                ? t("projection.applyOne")
                : t("projection.applyMany", { count: plan.changeCount })}
          cancelLabel={plan.blocked || plan.changeCount === 0 ? t("libraryPlan.close") : undefined}
          note={plan.blocked || plan.changeCount === 0
            ? t("libraryPlan.noChanges")
            : t("libraryPlan.snapshotNote")}
          onCancel={onCancelConfirmation}
          onConfirm={onApply}
        >
          <ul className="connection-confirm-list">
            {plan.steps
              .filter((step) => step.action !== "none")
              .map((step) => (
                <li key={step.agentId}>
                  <span>
                    <strong>{step.agentLabel}</strong>
                    <small>{step.action === "blocked" ? step.summary : stepSummary(step.action)}</small>
                  </span>
                  <code title={step.targetPath}>{step.targetPath}</code>
                </li>
              ))}
          </ul>
        </ConfirmDialog>
      )}
    </section>
  );
}
