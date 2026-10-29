import { useEffect, useRef, useState } from "react";
import { backend } from "../lib/backend";
import { useI18n } from "../lib/i18n";
import type { GitSyncOutcome, GitSyncPlan, GitSyncRequest, GitSyncResolution, GitSyncVersion } from "../lib/types";
import { ConfirmDialog } from "./ConfirmDialog";
import { splitSyncDiff } from "../lib/syncDiff";

function SourceComparison({ before, after }: { before: string | null; after: string | null }) {
  const { t } = useI18n();
  const diff = splitSyncDiff(before, after);
  return <div className="sync-compare">
    <article><h5>{t("sync.before")}</h5><pre>{before === null ? t("sync.missing") : <>{diff.prefix}<mark className="sync-removed">{diff.removed}</mark>{diff.suffix}</>}</pre></article>
    <article><h5>{t("sync.after")}</h5><pre>{after === null ? t("sync.missing") : <>{diff.prefix}<mark className="sync-added">{diff.added}</mark>{diff.suffix}</>}</pre></article>
  </div>;
}

export function GitSyncPage({ libraryRoot, onSynced, onProfiles }: {
  libraryRoot: string;
  onSynced: () => Promise<void>;
  onProfiles: () => void;
}) {
  const { t } = useI18n();
  const [request, setRequest] = useState<GitSyncRequest>({ remote: "", branch: "master", resolutions: [] });
  const [plan, setPlan] = useState<GitSyncPlan>();
  const [reviewed, setReviewed] = useState(false);
  const [busy, setBusy] = useState<"loading" | "preview" | "apply" | undefined>("loading");
  const [error, setError] = useState<string>();
  const [outcome, setOutcome] = useState<GitSyncOutcome>();
  const [confirm, setConfirm] = useState(false);
  const generation = useRef(0);
  useEffect(() => {
    const current = ++generation.current;
    void backend.gitSyncConfig(libraryRoot).then((config) => {
      if (current === generation.current && config) setRequest({ ...config, resolutions: [] });
    }).catch((e) => { if (current === generation.current) setError(String(e)); })
      .finally(() => { if (current === generation.current) setBusy(undefined); });
    return () => { generation.current++; };
  }, [libraryRoot]);

  const editConfig = (field: "remote" | "branch", value: string) => {
    setRequest((r) => ({ ...r, [field]: value, resolutions: [] }));
    setPlan(undefined); setReviewed(false); setOutcome(undefined); setError(undefined);
  };
  const decide = (id: string, choice: GitSyncResolution["choice"] | "", newProfileId?: string) => {
    setRequest((r) => ({ ...r, resolutions: [...r.resolutions.filter((d) => d.id !== id), ...(choice ? [{ id, choice, newProfileId }] : [])] }));
    setReviewed(false); setOutcome(undefined); setError(undefined);
  };
  const preview = async () => {
    const current = ++generation.current;
    setBusy("preview"); setReviewed(false); setError(undefined); setOutcome(undefined);
    try {
      const next = await backend.previewGitSync(request, libraryRoot);
      if (current === generation.current) { setPlan(next); setReviewed(true); }
    } catch (e) { if (current === generation.current) setError(String(e)); }
    finally { if (current === generation.current) setBusy(undefined); }
  };
  const apply = async () => {
    if (!plan || !reviewed || plan.blocked) return;
    const current = ++generation.current;
    setBusy("apply"); setError(undefined);
    try {
      const result = await backend.applyGitSync(request, plan.token, libraryRoot);
      if (current !== generation.current) return;
      setOutcome(result); setPlan(undefined); setReviewed(false); setRequest((r) => ({ ...r, resolutions: [] }));
      await onSynced();
    } catch (e) {
      if (current === generation.current) { setError(String(e)); setReviewed(false); }
    } finally {
      if (current === generation.current) { setConfirm(false); setBusy(undefined); }
    }
  };
  const showVersions = (files: GitSyncVersion[]) => files.length ? files.map((file) => <div key={file.path}>
    <code>{file.path}</code><pre>{file.content}</pre>
  </div>) : <p className="sync-missing">{t("sync.missing")}</p>;
  const localCount = plan?.changes.filter((c) => c.localBefore !== c.localAfter).length ?? 0;
  const remoteCount = plan?.changes.filter((c) => c.remoteBefore !== c.remoteAfter).length ?? 0;

  return <section className="git-sync-page" aria-busy={Boolean(busy)}>
    <div className="history-heading">
      <div><p className="eyebrow">{t("nav.sync")}</p><h2>{t("sync.intro")}</h2></div>
      <button className="button button-ghost button-small" disabled={busy === "apply"} onClick={onProfiles}>{t("nav.profiles")}</button>
    </div>
    <p className="sync-boundary">{t("sync.boundary")}</p>
    {!backend.isTauri && <p className="sync-demo-note">{t("sync.demo")}</p>}
    <form className="sync-config" onSubmit={(e) => { e.preventDefault(); void preview(); }}>
      <label>{t("sync.remote")}<input required disabled={Boolean(busy)} value={request.remote} placeholder="git@github.com:you/agent-rules.git" autoComplete="off" spellCheck={false} onChange={(e) => editConfig("remote", e.target.value)} /></label>
      <label>{t("sync.branch")}<input required disabled={Boolean(busy)} value={request.branch} autoComplete="off" spellCheck={false} onChange={(e) => editConfig("branch", e.target.value)} /></label>
      <button className="button button-primary" disabled={Boolean(busy)} type="submit">{t(busy === "preview" ? "sync.checking" : "sync.check")}</button>
      <p>{t("sync.auth")}</p>
    </form>
    {error && <p className="history-error" role="alert">{error}</p>}
    {outcome && <div className={outcome.pushError ? "history-error" : "history-success"} role="status">
      <strong>{t(outcome.pushError ? "sync.partial" : "sync.success")}</strong>
      {outcome.pushError && <p>{outcome.pushError}</p>}
      {outcome.backupId && <p>{t("sync.backup", { id: outcome.backupId })}</p>}
      <p>{t("sync.activation")}</p>
    </div>}
    {plan && <div className="sync-preview">
      <div className="sync-summary" role="status">
        <h3>{t("sync.summary", { local: localCount, remote: remoteCount })}</h3>
        <p>{t("sync.branchSummary", { branch: plan.config.branch, commit: plan.remoteCommit?.slice(0, 12) ?? t("sync.emptyRemote") })}</p>
        {plan.connectionChanged && <p>{t("sync.connectionChanged")}</p>}
        {!reviewed && <p className="sync-review-needed">{t("sync.reviewNeeded")}</p>}
      </div>
      {plan.blockers.map((reason) => <p className="history-error" key={reason} role="alert">{reason}</p>)}
      {plan.conflicts.length > 0 && <section aria-label={t("sync.conflicts")}>
        <h3>{t("sync.conflicts")}</h3><p>{t("sync.conflictHint")}</p>
        {plan.conflicts.map((conflict) => {
          const decision = request.resolutions.find((d) => d.id === conflict.id);
          return <fieldset className="sync-conflict" key={conflict.id} disabled={Boolean(busy)}>
            <legend>{conflict.id}</legend>
            <details><summary>{t("sync.compareVersions")}</summary><div className="sync-versions">
              <article><h4>{t("sync.base")}</h4>{showVersions(conflict.base)}</article>
              <article><h4>{t("sync.local")}</h4>{showVersions(conflict.local)}</article>
              <article><h4>{t("sync.remoteVersion")}</h4>{showVersions(conflict.remote)}</article>
            </div></details>
            <label>{t("sync.decision")}<select value={decision?.choice ?? ""} onChange={(e) => decide(conflict.id, e.target.value as GitSyncResolution["choice"], decision?.newProfileId)}>
              <option value="">{t("sync.choose")}</option>
              <option value="keepLocal">{t("sync.keepLocal")}</option>
              <option value="useRemote">{t("sync.useRemote")}</option>
              {conflict.canKeepBoth && <option value="keepBoth">{t("sync.keepBoth")}</option>}
            </select></label>
            {decision?.choice === "keepBoth" && <label>{t("sync.copyId")}<input required maxLength={64} pattern="[a-z0-9][a-z0-9_-]{0,63}" placeholder="work-remote" value={decision.newProfileId ?? ""} onChange={(e) => decide(conflict.id, "keepBoth", e.target.value)} /></label>}
            {reviewed && conflict.resolved && <small>{t("sync.resolved")}</small>}
          </fieldset>;
        })}
      </section>}
      <section aria-label={t("sync.files")}>
        <h3>{t("sync.files")}</h3>
        {plan.directories.map((d) => <p key={d.path}><code>{d.path}/</code> · {t(d.action === "create" ? "history.createDirectory" : "history.removeDirectory")}</p>)}
        {!plan.changes.length && <p>{t("sync.noChanges")}</p>}
        {plan.changes.map((change) => <details className="sync-file" key={change.path}>
          <summary><code>{change.path}</code><span>{[change.localBefore !== change.localAfter ? t("sync.localChange") : "", change.remoteBefore !== change.remoteAfter ? t("sync.remoteChange") : ""].filter(Boolean).join(" · ")}</span></summary>
          {change.localBefore !== change.localAfter && <div><h4>{t("sync.localChange")}</h4><SourceComparison before={change.localBefore} after={change.localAfter} /></div>}
          {change.remoteBefore !== change.remoteAfter && <div><h4>{t("sync.remoteChange")}</h4><SourceComparison before={change.remoteBefore} after={change.remoteAfter} /></div>}
        </details>)}
      </section>
      <div className="sync-actions">
        {request.resolutions.length > 0 && <button className="button button-ghost" disabled={Boolean(busy)} onClick={() => { setRequest((r) => ({ ...r, resolutions: [] })); setReviewed(false); }}>{t("sync.resetChoices")}</button>}
        <button className="button button-ghost" disabled={Boolean(busy)} onClick={() => void preview()}>{t("sync.checkAgain")}</button>
        <button className="button button-primary" disabled={Boolean(busy) || !reviewed || plan.blocked || (!plan.changes.length && !plan.connectionChanged)} onClick={() => setConfirm(true)}>{t("sync.apply")}</button>
      </div>
    </div>}
    {confirm && plan && <ConfirmDialog title={t("sync.confirmTitle")} description={t("sync.summary", { local: localCount, remote: remoteCount })} note={t("sync.activation")} confirming={busy === "apply"} confirmLabel={t("sync.confirm")} onConfirm={() => void apply()} onCancel={() => setConfirm(false)} />}
  </section>;
}
