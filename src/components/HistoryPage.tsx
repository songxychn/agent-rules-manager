import { useCallback, useEffect, useRef, useState } from "react";
import { backend } from "../lib/backend";
import { useI18n } from "../lib/i18n";
import type { HistoryRecord, RestorePlan } from "../lib/types";
import { ConfirmDialog } from "./ConfirmDialog";

export function HistoryPage({ libraryRoot, onRestored }: {
  libraryRoot: string;
  onRestored: () => Promise<void>;
}) {
  const { t, locale } = useI18n();
  const [records, setRecords] = useState<HistoryRecord[]>([]);
  const [filter, setFilter] = useState("all");
  const [limit, setLimit] = useState(50);
  const [selected, setSelected] = useState<HistoryRecord>();
  const [plan, setPlan] = useState<RestorePlan>();
  const [busy, setBusy] = useState<"loading" | "preview" | "restore">();
  const [error, setError] = useState<string>();
  const [success, setSuccess] = useState(false);
  const [confirm, setConfirm] = useState(false);
  const generation = useRef(0);
  const load = useCallback(async () => {
    const request = ++generation.current;
    setBusy("loading"); setError(undefined); setSelected(undefined); setPlan(undefined);
    try {
      const next = await backend.history(libraryRoot);
      if (request === generation.current) setRecords(next);
    } catch (e) { if (request === generation.current) setError(String(e)); }
    finally { if (request === generation.current) setBusy(undefined); }
  }, [libraryRoot]);
  useEffect(() => { void load(); return () => { generation.current++; }; }, [load]);

  const select = async (record: HistoryRecord) => {
    const request = ++generation.current;
    setSelected(record); setPlan(undefined); setError(undefined); setSuccess(false);
    if (!record.restorable) { setBusy(undefined); return; }
    setBusy("preview");
    try {
      const next = await backend.previewRestore(record.id, libraryRoot);
      if (request === generation.current) setPlan(next);
    } catch (e) { if (request === generation.current) setError(String(e)); }
    finally { if (request === generation.current) setBusy(undefined); }
  };

  const restore = async () => {
    if (!plan || plan.blocked) return;
    setBusy("restore"); setError(undefined);
    try {
      await backend.restoreHistory(plan.target.id, plan.token, libraryRoot);
      setConfirm(false);
      await onRestored();
      await load();
      setSuccess(true);
    } catch (e) {
      setConfirm(false); setPlan(undefined); setError(String(e)); setBusy(undefined);
    }
  };

  const date = (value: string) => {
    const parsed = new Date(value);
    return Number.isNaN(parsed.getTime()) ? value : new Intl.DateTimeFormat(locale, {
      year: "numeric", month: "short", day: "numeric", hour: "2-digit", minute: "2-digit", second: "2-digit",
    }).format(parsed);
  };
  const title = (record: HistoryRecord) => {
    const operations = ["initialize", "upgradeLibrary", "createProfile", "deleteProfile", "activateProfile", "connectionChange", "restoreHistory", "historyBaseline"];
    const label = t(`history.${operations.includes(record.operation) ? record.operation : "unknown"}` as "history.initialize");
    const subjects = record.subjects.map((subject) => {
      if (record.operation === "restoreHistory") return date(subject);
      if (subject.startsWith("connect:")) return t("history.connect", { name: subject.slice(8) });
      if (subject.startsWith("disconnect:")) return t("history.disconnect", { name: subject.slice(11) });
      return subject;
    });
    return `${label}${subjects.length ? ` · ${subjects.join("、")}` : ""}`;
  };
  const stateLabel = (value: string) => {
    if (value.startsWith("link:")) return t("history.link", { target: value.slice(5) });
    if (value.startsWith("profile:")) return t("history.profile", { id: value.slice(8) });
    return t(`history.${value}` as "history.file");
  };
  const visible = records.filter((r) => filter === "all" || r.scope === filter);
  const disabled = busy === "restore";

  return <section className="history-page">
    <div className="history-heading">
      <div><p className="eyebrow">{t("nav.history")}</p><h2>{t("history.intro")}</h2></div>
      <button className="button button-ghost button-small" disabled={Boolean(busy)} onClick={() => void load()}>{t("history.refresh")}</button>
    </div>
    <p className="history-scope-note">{t("history.scopeNote")}</p>
    {error && <p className="history-error" role="alert">{error}</p>}
    {success && <p className="history-success" role="status">{t("history.success")}</p>}
    <div className="history-layout">
      <div className="history-timeline">
        <label className="history-filter">{t("history.filter")}
          <select value={filter} disabled={disabled} onChange={(e) => { setFilter(e.target.value); setLimit(50); }}>
            {["all", "profiles", "agents", "recovery"].map((value) => <option key={value} value={value}>{t(`history.${value}` as "history.all")}</option>)}
          </select>
        </label>
        {busy === "loading" && <p role="status">{t("history.loading")}</p>}
        {!visible.length && busy !== "loading" && <p className="history-empty">{t("history.empty")}</p>}
        <ol className="history-records">
          {visible.slice(0, limit).map((record) => <li key={record.id}>
            <button className={`history-record ${selected?.id === record.id ? "is-selected" : ""}`} disabled={disabled}
              aria-pressed={selected?.id === record.id} onClick={() => void select(record)}>
              <span className="history-record-time"><time dateTime={record.createdAt}>{record.operation === "historyBaseline" ? t("history.baselineTime") : date(record.createdAt)}</time>
                {record.id === records[0]?.id && <small>{t("history.latest")}</small>}</span>
              <strong>{title(record)}</strong>
              <span className="history-record-scope">{t(`history.${record.scope}` as "history.profiles")}
                {!record.restorable && <span> · {t("history.archived")}</span>}</span>
            </button>
          </li>)}
        </ol>
        {visible.length > limit && <button className="button button-ghost" onClick={() => setLimit(limit + 50)}>{t("history.more")}</button>}
      </div>
      <section className="history-preview" aria-label={t("history.after")} aria-busy={busy === "preview"}>
        {!selected ? <div className="history-placeholder"><span aria-hidden="true">↶</span><h3>{t("history.pick")}</h3><p>{t("history.pickHint")}</p></div> : <>
          <p className="eyebrow">{t(selected.operation === "historyBaseline" ? "history.baselinePreview" : "history.after")}</p>
          <h3>{title(selected)}</h3><time dateTime={selected.createdAt}>{date(selected.createdAt)}</time>
          {!selected.restorable && <p className="history-legacy">{t(selected.unavailableReason === "legacyRestored" ? "history.legacyRestored" : "history.legacyBoundary")}</p>}
          {busy === "preview" && <p role="status">{t("history.previewing")}</p>}
          {plan && <>
            {plan.blocked ? <div className="history-conflicts" role="alert"><strong>{t("history.conflicts")}</strong><p>{t("history.conflictHint")}</p>
              <ul>{plan.conflicts.map((path) => <li key={path}><code>{path}</code></li>)}</ul></div>
              : <p className="history-check">{t(plan.steps.length ? "history.checkPassed" : "history.noChanges")}</p>}
            {plan.steps.length > 0 && <div className="history-effects"><h4>{t("history.result")}</h4>
              {plan.steps.filter((step) => step.before.startsWith("profile:") || step.after.startsWith("profile:")).map((step) => <p key={step.path}>{stateLabel(step.before)} → {stateLabel(step.after)}</p>)}
              {plan.steps.filter((step) => /\/profiles\/[^/]+\/profile\.json$/.test(step.path) && (step.before === "missing" || step.after === "missing")).map((step) =>
                <p key={step.path}>{t(step.after === "missing" ? "history.removeProfileEffect" : "history.restoreProfileEffect", { id: step.path.split("/").at(-2)! })}</p>)}
              {plan.steps.filter((step) => records.some((record) => record.scope === "agents" && record.paths.includes(step.path))).map((step) =>
                <p key={step.path}>{t("history.agentEffect", { name: step.path, before: step.before.startsWith("link:") ? t("history.linkState") : stateLabel(step.before), after: step.after.startsWith("link:") ? t("history.linkState") : stateLabel(step.after) })}</p>)}
              <p>{t("history.undoCount", { count: plan.operations.length })} · {t("history.files", { count: plan.steps.length })}</p>
            </div>}
            <details className="history-detail" open={plan.operations.length > 0 && plan.operations.length <= 4}>
              <summary>{t("history.undoCount", { count: plan.operations.length })}</summary>
              <ol>{plan.operations.map((record) => <li key={record.id}><strong>{title(record)}</strong><time dateTime={record.createdAt}>{date(record.createdAt)}</time></li>)}</ol>
            </details>
            <details className="history-detail"><summary>{t("history.files", { count: plan.steps.length })}</summary>
              {plan.steps.map((step) => <div className="history-file" key={step.path}>
                <strong>{t(`history.${step.action}` as "history.restoreFile")}</strong><code>{step.path}</code>
                <div><span>{t("history.before")}</span><p>{stateLabel(step.before)}</p></div>
                <div><span>{t("history.target")}</span><p>{stateLabel(step.after)}</p></div>
                {(step.beforeContent != null || step.afterContent != null) && <details className="history-content"><summary>{t("history.content")}</summary>
                  <p>{t("history.before")}</p><pre>{step.beforeContent ?? t("history.missing")}</pre>
                  <p>{t("history.target")}</p><pre>{step.afterContent ?? t("history.missing")}</pre>
                </details>}
              </div>)}
            </details>
          </>}
          <details className="history-detail"><summary>{t("history.details")}</summary><code>{selected.id}</code>
            <p>{t("history.paths")}</p><ul>{selected.paths.map((path) => <li key={path}><code>{path}</code></li>)}</ul>
          </details>
          {selected.restorable && <div className="history-actions">
            <button className="button button-ghost button-small" disabled={Boolean(busy)} onClick={() => void select(selected)}>{t("history.previewAgain")}</button>
            <button className="button button-primary button-small" disabled={Boolean(busy) || !plan || plan.blocked || !plan.steps.length} onClick={() => setConfirm(true)}>{t("history.restore")}</button>
          </div>}
        </>}
      </section>
    </div>
    {confirm && plan && <ConfirmDialog title={t("history.confirmTitle")}
      description={`${title(plan.target)} · ${date(plan.target.createdAt)} · ${t("history.undoCount", { count: plan.operations.length })}`}
      confirmLabel={t("history.restore")} confirming={disabled} note={t("history.confirmNote")}
      onCancel={() => setConfirm(false)} onConfirm={() => void restore()} />}
  </section>;
}
