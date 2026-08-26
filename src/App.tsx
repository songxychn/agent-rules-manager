import { useCallback, useEffect, useRef, useState } from "react";
import { AppIcon, BrandMark } from "./components/AppIcon";
import { ProjectionRail } from "./components/ProjectionRail";
import { SettingsPage } from "./components/SettingsPage";
import { SourceOpenControl } from "./components/SourceOpenControl";
import { backend } from "./lib/backend";
import { useI18n } from "./lib/i18n";
import {
  readPreferredOpenTarget,
  writePreferredOpenTarget,
} from "./lib/openTargets";
import type {
  ConnectionChange,
  OpenTarget,
  ProjectionPlan,
  WorkspaceSnapshot,
} from "./lib/types";
import "./styles.css";

type Notice = { tone: "success" | "error" | "info"; message: string };

function projectionFingerprint(snapshot: WorkspaceSnapshot | undefined): string {
  if (!snapshot) return "";
  return JSON.stringify({
    sourcePath: snapshot.sourcePath,
    sourceExists: snapshot.sourceExists,
    sourceDigest: snapshot.sourceDigest,
    sourceModifiedAt: snapshot.sourceModifiedAt,
    agents: snapshot.agents.map((agent) => [
      agent.id,
      agent.targetPath,
      agent.mode,
      agent.state,
      agent.targetKind,
      agent.installed,
      agent.connected,
    ]),
  });
}

function currentConnections(snapshot: WorkspaceSnapshot): Set<string> {
  return new Set(snapshot.agents.filter((agent) => agent.connected).map((agent) => agent.id));
}

function connectionChanges(
  snapshot: WorkspaceSnapshot,
  desiredConnections: Set<string>,
): ConnectionChange[] {
  return snapshot.agents
    .filter((agent) => agent.installed || agent.connected)
    .filter((agent) => desiredConnections.has(agent.id) !== agent.connected)
    .map((agent) => ({
      agentId: agent.id,
      connected: desiredConnections.has(agent.id),
    }));
}

function App() {
  const { t } = useI18n();
  const [view, setView] = useState<"control" | "settings">("control");
  const [snapshot, setSnapshot] = useState<WorkspaceSnapshot>();
  const [libraryRoot, setLibraryRoot] = useState("");
  const [libraryInput, setLibraryInput] = useState("");
  const [plan, setPlan] = useState<ProjectionPlan>();
  const [desiredConnections, setDesiredConnections] = useState<Set<string>>(new Set());
  const [openTargets, setOpenTargets] = useState<OpenTarget[]>([]);
  const [preferredOpenTarget, setPreferredOpenTarget] = useState(readPreferredOpenTarget);
  const [openingTargetId, setOpeningTargetId] = useState<string>();
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
      setDesiredConnections(currentConnections(next));
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
    async () => {
      if (!libraryRoot || refreshInFlight.current) return;
      refreshInFlight.current = true;
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
        if (projectionChanged) {
          setDesiredConnections(currentConnections(next));
          setPlan(undefined);
        }
      } catch (error) {
        setNotice({ tone: "error", message: String(error) });
      } finally {
        refreshInFlight.current = false;
      }
    },
    [libraryRoot],
  );

  useEffect(() => {
    const refreshOnFocus = () => void refreshWorkspace();
    window.addEventListener("focus", refreshOnFocus);
    return () => window.removeEventListener("focus", refreshOnFocus);
  }, [refreshWorkspace]);

  useEffect(() => {
    window.scrollTo({ top: 0, left: 0, behavior: "auto" });
  }, [view]);

  const preview = async () => {
    const current = snapshotRef.current;
    if (!current) return;
    const changes = connectionChanges(current, desiredConnections);
    if (!changes.length) {
      setNotice({ tone: "info", message: t("notice.noConnectionChanges") });
      return;
    }
    setBusy("planning");
    setNotice(undefined);
    try {
      setPlan(await backend.preview(changes, libraryRoot));
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
    const changes = plan.steps.map((step) => ({
      agentId: step.agentId,
      connected: step.desiredConnected,
    }));
    if (!changes.length) return;
    setBusy("applying");
    try {
      const outcome = await backend.apply(changes, libraryRoot);
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

  const installed = snapshot.agents.filter((agent) => agent.installed || agent.connected);
  const connected = installed.filter((agent) => agent.connected).length;
  const pendingChanges = connectionChanges(snapshot, desiredConnections);

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
            <strong>{connected}/{installed.length}</strong>
          </div>
          <span className="alignment-track" aria-hidden="true">
            <span
              style={{
                width: `${installed.length ? (connected / installed.length) * 100 : 0}%`,
              }}
            />
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
            {view === "control" && <span>{installed.length}</span>}
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
            title={t("rollback.tooltip")}
          >
            <AppIcon name="rollback" />
            {busy === "rollback" ? t("rollback.restoring") : t("rollback.latest")}
          </button>
          {view === "control" && snapshot.sourceExists && (
            <SourceOpenControl
              targets={openTargets}
              preferredTargetId={preferredOpenTarget}
              openingTargetId={openingTargetId}
              onOpen={(targetId) => void openSource(targetId)}
            />
          )}
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
              desiredConnections={desiredConnections}
              pendingCount={pendingChanges.length}
              loading={busy === "planning"}
              applying={busy === "applying"}
              onToggle={(agentId, nextConnected) => {
                setDesiredConnections((current) => {
                  const next = new Set(current);
                  if (nextConnected) next.add(agentId);
                  else next.delete(agentId);
                  return next;
                });
                setPlan(undefined);
              }}
              onInspect={() => void preview()}
              onApply={() => void apply()}
            />
          </>
        )}
        </main>
      </div>
    </div>
  );
}

export default App;
