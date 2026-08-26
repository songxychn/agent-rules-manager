import { useCallback, useEffect, useRef, useState } from "react";
import { AppIcon, BrandMark } from "./components/AppIcon";
import { ProjectionRail } from "./components/ProjectionRail";
import { SettingsPage } from "./components/SettingsPage";
import { SourceFilePanel } from "./components/SourceFilePanel";
import { backend } from "./lib/backend";
import { useI18n } from "./lib/i18n";
import {
  readPreferredOpenTarget,
  writePreferredOpenTarget,
} from "./lib/openTargets";
import type { OpenTarget, ProjectionPlan, WorkspaceSnapshot } from "./lib/types";
import "./styles.css";

type Notice = { tone: "success" | "error" | "info"; message: string };

function projectionFingerprint(snapshot: WorkspaceSnapshot | undefined): string {
  if (!snapshot) return "";
  return JSON.stringify({
    sourcePath: snapshot.sourcePath,
    sourceExists: snapshot.sourceExists,
    agents: snapshot.agents.map((agent) => [
      agent.id,
      agent.targetPath,
      agent.mode,
      agent.state,
    ]),
  });
}

function App() {
  const { t } = useI18n();
  const [view, setView] = useState<"control" | "settings">("control");
  const [snapshot, setSnapshot] = useState<WorkspaceSnapshot>();
  const [libraryRoot, setLibraryRoot] = useState("");
  const [libraryInput, setLibraryInput] = useState("");
  const [plan, setPlan] = useState<ProjectionPlan>();
  const [openTargets, setOpenTargets] = useState<OpenTarget[]>([]);
  const [preferredOpenTarget, setPreferredOpenTarget] = useState(readPreferredOpenTarget);
  const [openingTargetId, setOpeningTargetId] = useState<string>();
  const [refreshing, setRefreshing] = useState(false);
  const [busy, setBusy] = useState<"loading" | "planning" | "applying" | "rollback">();
  const [notice, setNotice] = useState<Notice>();
  const snapshotRef = useRef<WorkspaceSnapshot>();
  const refreshInFlight = useRef(false);

  const loadSnapshot = useCallback(async (root?: string) => {
    setBusy("loading");
    try {
      const next = await backend.snapshot(root || undefined);
      snapshotRef.current = next;
      setSnapshot(next);
      setLibraryRoot(next.libraryRoot);
      setLibraryInput(next.libraryRoot);
      setPlan(undefined);
    } catch (error) {
      setNotice({ tone: "error", message: String(error) });
    } finally {
      setBusy(undefined);
    }
  }, []);

  useEffect(() => {
    void loadSnapshot();
  }, [loadSnapshot]);

  useEffect(() => {
    void backend
      .openTargets()
      .then(setOpenTargets)
      .catch((error) => setNotice({ tone: "error", message: String(error) }));
  }, []);

  const refreshWorkspace = useCallback(
    async (announce: boolean) => {
      if (!libraryRoot || refreshInFlight.current) return;
      refreshInFlight.current = true;
      setRefreshing(true);
      try {
        const [next, nextOpenTargets] = await Promise.all([
          backend.snapshot(libraryRoot),
          backend.openTargets(),
        ]);
        const projectionChanged =
          projectionFingerprint(snapshotRef.current) !== projectionFingerprint(next);
        snapshotRef.current = next;
        setSnapshot(next);
        setOpenTargets(nextOpenTargets);
        if (projectionChanged) setPlan(undefined);
        if (announce) {
          setNotice({ tone: "success", message: t("notice.sourceRefreshed") });
        }
      } catch (error) {
        setNotice({ tone: "error", message: String(error) });
      } finally {
        refreshInFlight.current = false;
        setRefreshing(false);
      }
    },
    [libraryRoot, t],
  );

  useEffect(() => {
    const refreshOnFocus = () => void refreshWorkspace(false);
    window.addEventListener("focus", refreshOnFocus);
    return () => window.removeEventListener("focus", refreshOnFocus);
  }, [refreshWorkspace]);

  useEffect(() => {
    window.scrollTo({ top: 0, left: 0, behavior: "auto" });
  }, [view]);

  const preview = async () => {
    const agentIds = snapshotRef.current?.agents.map((agent) => agent.id) ?? [];
    if (!agentIds.length) return;
    setBusy("planning");
    setNotice(undefined);
    try {
      setPlan(await backend.preview(agentIds, libraryRoot));
    } catch (error) {
      setNotice({ tone: "error", message: String(error) });
    } finally {
      setBusy(undefined);
    }
  };

  const openSource = async (targetId: string) => {
    const target = openTargets.find((candidate) => candidate.id === targetId);
    if (!target) return;
    setPreferredOpenTarget(targetId);
    writePreferredOpenTarget(targetId);
    setOpeningTargetId(targetId);
    try {
      const opened = await backend.openSource(targetId, libraryRoot);
      setNotice(
        opened
          ? {
              tone: "success",
              message: t("notice.sourceOpened", {
                app: target.id === "default" ? t("source.open.default") : target.label,
              }),
            }
          : { tone: "info", message: t("notice.demoOpen") },
      );
    } catch (error) {
      setNotice({ tone: "error", message: String(error) });
    } finally {
      setOpeningTargetId(undefined);
    }
  };

  const apply = async () => {
    if (!plan) return;
    const affectedAgentIds = plan.steps
      .filter((step) => step.action !== "none" && step.action !== "blocked")
      .map((step) => step.agentId);
    if (!affectedAgentIds.length) return;
    setBusy("applying");
    try {
      const outcome = await backend.apply(affectedAgentIds, libraryRoot);
      setNotice({
        tone: "success",
        message: outcome.changed.length
          ? t("notice.applied", { agents: outcome.changed.join(", ") })
          : t("notice.aligned"),
      });
      await loadSnapshot(libraryRoot);
    } catch (error) {
      setNotice({ tone: "error", message: String(error) });
    } finally {
      setBusy(undefined);
    }
  };

  const rollback = async () => {
    setBusy("rollback");
    try {
      const outcome = await backend.rollback(libraryRoot);
      setNotice({ tone: "success", message: t("notice.restored", { id: outcome.backupId }) });
      await loadSnapshot(libraryRoot);
    } catch (error) {
      setNotice({ tone: "error", message: String(error) });
    } finally {
      setBusy(undefined);
    }
  };

  const initialize = async () => {
    setBusy("loading");
    try {
      await backend.initialize(libraryRoot || undefined);
      await loadSnapshot(libraryRoot || undefined);
      setNotice({ tone: "success", message: t("notice.initialized") });
    } catch (error) {
      setNotice({ tone: "error", message: String(error) });
      setBusy(undefined);
    }
  };

  if (!snapshot) {
    return (
      <main className="loading-screen">
        <BrandMark />
        <p>{busy === "loading" ? t("loading.inspecting") : t("loading.failed")}</p>
      </main>
    );
  }

  const synced = snapshot.agents.filter((agent) => agent.state === "inSync").length;

  return (
    <div className="app-shell">
      <nav className="sidebar" aria-label={t("nav.primary")}>
        <div className="brand-lockup">
          <BrandMark />
          <div>
            <strong>Agent Rules Manager</strong>
            <span>{t("brand.subtitle")}</span>
          </div>
        </div>

        <div className="nav-primary">
          <button
            className={`nav-item ${view === "control" ? "is-active" : ""}`}
            aria-current={view === "control" ? "page" : undefined}
            onClick={() => setView("control")}
          >
            <span className="nav-icon"><AppIcon name="control" /></span>
            {t("nav.control")}
          </button>
          <button className="nav-item" disabled>
            <span className="nav-icon"><AppIcon name="rules" /></span>
            {t("nav.rulePacks")}
            <small>{t("nav.next")}</small>
          </button>
          <button className="nav-item" disabled>
            <span className="nav-icon"><AppIcon name="profiles" /></span>
            {t("nav.profiles")}
            <small>{t("nav.next")}</small>
          </button>
        </div>

        <div className="sidebar-readout">
          <div>
            <span>{t("readout.paths")}</span>
            <strong>{synced}/{snapshot.agents.length}</strong>
          </div>
          <span className="alignment-track" aria-hidden="true">
            <span style={{ width: `${(synced / snapshot.agents.length) * 100}%` }} />
          </span>
          <small>{t("readout.aligned")}</small>
        </div>

        <button
          className={`nav-item sidebar-settings ${view === "settings" ? "is-active" : ""}`}
          aria-current={view === "settings" ? "page" : undefined}
          onClick={() => setView("settings")}
        >
          <span className="nav-icon"><AppIcon name="settings" /></span>
          {t("nav.settings")}
        </button>
      </nav>

      <div className="main-shell">
        <header className="topbar">
          <div className="page-heading">
            <h1>{view === "settings" ? t("nav.settings") : t("nav.control")}</h1>
            {view === "control" && <span>{snapshot.agents.length}</span>}
          </div>
        <div className="library-loader">
          <label htmlFor="library-root">{t("library.label")}</label>
          <div className="library-field">
            <AppIcon name="folder" />
            <input
              id="library-root"
              aria-label={t("library.label")}
              value={libraryInput}
              onChange={(event) => setLibraryInput(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") void loadSnapshot(libraryInput);
              }}
            />
          </div>
          <button
            className="button button-ghost button-small"
            onClick={() => void loadSnapshot(libraryInput)}
            disabled={busy === "loading"}
          >
            {t("library.load")}
          </button>
        </div>
        <div className="topbar-actions">
          {!backend.isTauri && <span className="runtime-badge">{t("runtime.demo")}</span>}
          <button
            className="button button-ghost button-small"
            disabled={!snapshot.latestBackup || Boolean(busy)}
            onClick={() => void rollback()}
          >
            <AppIcon name="rollback" />
            {busy === "rollback" ? t("rollback.restoring") : t("rollback.latest")}
          </button>
        </div>
        </header>

        <main className="workspace">
        {notice && (
          <div className={`notice notice-${notice.tone}`} role="status">
            <span>{notice.message}</span>
            <button onClick={() => setNotice(undefined)} aria-label={t("notice.dismiss")}>×</button>
          </div>
        )}

        {view === "settings" ? (
          <SettingsPage />
        ) : !snapshot.sourceExists ? (
          <section className="source-missing">
            <p className="eyebrow">{t("source.missing.eyebrow")}</p>
            <h1>{t("source.missing.title")}</h1>
            <p>{t("source.missing.body", { path: snapshot.sourcePath })}</p>
            <button className="button button-primary" onClick={() => void initialize()}>
              {t("source.missing.initialize")}
            </button>
          </section>
        ) : (
          <>
            <ProjectionRail
              agents={snapshot.agents}
              sourceDigest={snapshot.sourceDigest}
              plan={plan}
              loading={busy === "planning"}
              applying={busy === "applying"}
              onInspect={() => void preview()}
              onApply={() => void apply()}
            />

            <div className="workbench-grid">
              <SourceFilePanel
                sourcePath={snapshot.sourcePath}
                sourceDigest={snapshot.sourceDigest}
                sourceModifiedAt={snapshot.sourceModifiedAt}
                targets={openTargets}
                preferredTargetId={preferredOpenTarget}
                openingTargetId={openingTargetId}
                refreshing={refreshing}
                onOpen={(targetId) => void openSource(targetId)}
                onRefresh={() => void refreshWorkspace(true)}
              />
            </div>
          </>
        )}
        </main>
      </div>
    </div>
  );
}

export default App;
