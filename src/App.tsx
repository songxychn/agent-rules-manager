import { useCallback, useEffect, useRef, useState } from "react";
import { AppIcon, BrandMark } from "./components/AppIcon";
import { LibrarySetupPanel } from "./components/LibrarySetupPanel";
import { HistoryPage } from "./components/HistoryPage";
import { ProfilesPage } from "./components/ProfilesPage";
import { AgentScopePanel } from "./components/AgentScopePanel";
import { ProjectionRail } from "./components/ProjectionRail";
import { NotificationCenter } from "./components/NotificationCenter";
import { useNotifications } from "./lib/notifications";
import { SettingsPage } from "./components/SettingsPage";
import { backend } from "./lib/backend";
import { useI18n } from "./lib/i18n";
import { readPreferredOpenTarget, writePreferredOpenTarget } from "./lib/openTargets";
import type {
  ConnectionChange,
  LibraryPlan,
  OpenTarget,
  ProfileDraft,
  ProjectionPlan,
  WorkspaceSnapshot,
} from "./lib/types";
import "./styles.css";

type View = "control" | "profiles" | "history" | "settings";

function workspaceFingerprint(snapshot: WorkspaceSnapshot | undefined): string {
  if (!snapshot) return "";
  return JSON.stringify({
    libraryState: snapshot.libraryState,
    runtimeState: snapshot.runtimeState,
    activeProfileId: snapshot.activeProfileId,
    sourcePath: snapshot.sourcePath,
    sourceExists: snapshot.sourceExists,
    sourceDigest: snapshot.sourceDigest,
    sourceModifiedAt: snapshot.sourceModifiedAt,
    profiles: snapshot.profiles.map((profile) => [
      profile.id,
      profile.files.map((file) => [file.path, file.digest]),
      profile.isActive,
    ]),
    agents: snapshot.agents.map((agent) => [
      agent.id,
      agent.targetPath,
      agent.mode,
      agent.state,
      agent.targetKind,
      agent.installed,
      agent.connected,
      agent.warning,
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
    .filter((agent) => agent.installed || agent.connected || agent.scope === "project")
    .filter(
      (agent) =>
        desiredConnections.has(agent.id) !== agent.connected ||
        (desiredConnections.has(agent.id) && agent.state === "drifted"),
    )
    .map((agent) => ({
      agentId: agent.id,
      connected: desiredConnections.has(agent.id),
    }));
}

function App() {
  const { t } = useI18n();
  const [view, setView] = useState<View>("control");
  const [snapshot, setSnapshot] = useState<WorkspaceSnapshot>();
  const [projectRoot, setProjectRoot] = useState("");
  const projectRootRef = useRef("");
  const [libraryRoot, setLibraryRoot] = useState("");
  const [libraryInput, setLibraryInput] = useState("");
  const [plan, setPlan] = useState<ProjectionPlan>();
  const [setupPlan, setSetupPlan] = useState<LibraryPlan>();
  const [desiredConnections, setDesiredConnections] = useState<Set<string>>(new Set());
  const [openTargets, setOpenTargets] = useState<OpenTarget[]>([]);
  const [preferredOpenTarget, setPreferredOpenTarget] = useState(readPreferredOpenTarget);
  const [openingTargetId, setOpeningTargetId] = useState<string>();
  const [busy, setBusy] = useState<string>();
  const notifications = useNotifications();
  const { notify } = notifications;
  const snapshotRef = useRef<WorkspaceSnapshot>();
  const refreshInFlight = useRef(false);

  const loadSnapshot = useCallback(async (root?: string) => {
    setBusy("loading");
    const requestedProject = projectRootRef.current;
    try {
      const next = await backend.snapshot(root || undefined, requestedProject || undefined);
      if (requestedProject !== projectRootRef.current) return;
      snapshotRef.current = next;
      setSnapshot(next);
      setLibraryRoot(next.libraryRoot);
      setLibraryInput(next.libraryRoot);
      setDesiredConnections(currentConnections(next));
      setPlan(undefined);
      setSetupPlan(undefined);
    } catch (error) {
      notify({ tone: "error", message: String(error) });
    } finally {
      setBusy(undefined);
    }
  }, [notify]);

  useEffect(() => {
    void loadSnapshot();
  }, [loadSnapshot]);

  useEffect(() => {
    void backend
      .openTargets()
      .then(setOpenTargets)
      .catch((error) => notify({ tone: "error", message: String(error) }));
  }, [notify]);

  const refreshWorkspace = useCallback(
    async () => {
      if (!libraryRoot || refreshInFlight.current) return;
      refreshInFlight.current = true;
      const requestedProject = projectRootRef.current;
      try {
        const [next, nextOpenTargets] = await Promise.all([
          backend.snapshot(libraryRoot, requestedProject || undefined),
          backend.openTargets(),
        ]);
        if (requestedProject !== projectRootRef.current) return;
        const changed = workspaceFingerprint(snapshotRef.current) !== workspaceFingerprint(next);
        snapshotRef.current = next;
        setSnapshot(next);
        setOpenTargets(nextOpenTargets);
        if (changed) {
          setDesiredConnections(currentConnections(next));
          setPlan(undefined);
          setSetupPlan(undefined);
        }
      } catch (error) {
        notify({ tone: "error", message: String(error) });
      } finally {
        refreshInFlight.current = false;
      }
    },
    [libraryRoot, notify],
  );

  useEffect(() => {
    const refreshOnFocus = () => void refreshWorkspace();
    window.addEventListener("focus", refreshOnFocus);
    return () => window.removeEventListener("focus", refreshOnFocus);
  }, [refreshWorkspace]);

  useEffect(() => {
    window.scrollTo({ top: 0, left: 0, behavior: "auto" });
  }, [view]);

  const prepareProjectionConfirmation = async () => {
    const current = snapshotRef.current;
    if (!current) return;
    const changes = connectionChanges(current, desiredConnections);
    if (!changes.length) {
      notify({ tone: "info", message: t("notice.noConnectionChanges") });
      return;
    }
    setBusy("planning");
    try {
      setPlan(await backend.preview(changes, libraryRoot, projectRoot || undefined));
    } catch (error) {
      notify({ tone: "error", message: String(error) });
    } finally {
      setBusy(undefined);
    }
  };

  const applyProjection = async () => {
    if (!plan) return;
    const changes = plan.steps.map((step) => ({
      agentId: step.agentId,
      connected: step.desiredConnected,
    }));
    if (!changes.length) return;
    setBusy("applying");
    try {
      const outcome = await backend.apply(changes, plan.confirmationCount > 0, libraryRoot, projectRoot || undefined);
      notify({
        tone: "success",
        message: outcome.changed.length
          ? outcome.preservedBackupDir
            ? t("notice.appliedWithBackup", {
                agents: outcome.changed.join(", "),
                path: outcome.preservedBackupDir,
              })
            : t("notice.applied", { agents: outcome.changed.join(", ") })
          : t("notice.aligned"),
      });
      await loadSnapshot(libraryRoot);
    } catch (error) {
      notify({ tone: "error", message: String(error) });
    } finally {
      setBusy(undefined);
    }
  };

  const prepareInitializeConfirmation = async () => {
    setBusy("planning-library");
    try {
      setSetupPlan(await backend.previewInitialize(libraryRoot || undefined));
    } catch (error) {
      notify({ tone: "error", message: String(error) });
    } finally {
      setBusy(undefined);
    }
  };

  const initialize = async () => {
    setBusy("initializing");
    try {
      await backend.initialize(libraryRoot || undefined);
      await loadSnapshot(libraryRoot || undefined);
      notify({ tone: "success", message: t("notice.initialized") });
    } catch (error) {
      notify({ tone: "error", message: String(error) });
      setBusy(undefined);
    }
  };

  const openRuleSource = async (
    targetId: string,
    profileId: string,
    relativePath: string,
  ) => {
    const target = openTargets.find((candidate) => candidate.id === targetId);
    if (!target) return;
    setPreferredOpenTarget(targetId);
    writePreferredOpenTarget(targetId);
    setOpeningTargetId(targetId);
    try {
      const opened = await backend.openRuleSource(targetId, profileId, relativePath, libraryRoot);
      notify(
        opened
          ? {
              tone: "success",
              message: t("notice.sourceOpened", {
                file: relativePath,
                app: target.id === "default" ? t("source.open.default") : target.label,
              }),
            }
          : { tone: "info", message: t("notice.demoOpen") },
      );
    } catch (error) {
      notify({ tone: "error", message: String(error) });
    } finally {
      setOpeningTargetId(undefined);
    }
  };

  const createProfile = async (draft: ProfileDraft) => {
    await backend.createProfile(draft, libraryRoot);
    await loadSnapshot(libraryRoot);
    notify({ tone: "success", message: t("notice.profileCreated", { name: draft.name }) });
  };

  const deleteProfile = async (profileId: string) => {
    await backend.deleteProfile(profileId, libraryRoot);
    await loadSnapshot(libraryRoot);
    notify({ tone: "success", message: t("notice.profileDeleted", { id: profileId }) });
  };

  const activateProfile = async (profileId: string) => {
    await backend.activateProfile(profileId, libraryRoot);
    await loadSnapshot(libraryRoot);
    notify({ tone: "success", message: t("notice.profileActivated", { id: profileId }) });
  };

  if (!snapshot) {
    return (
      <main className="loading-screen">
        <div className="loading-notifications"><NotificationCenter {...notifications} /></div>
        <BrandMark />
        <p>{busy === "loading" ? t("loading.inspecting") : t("loading.failed")}</p>
      </main>
    );
  }

  const installed = snapshot.agents.filter((agent) => agent.installed || agent.connected);
  const connected = installed.filter((agent) => agent.connected).length;
  const pendingChanges = connectionChanges(snapshot, desiredConnections);
  const activeProfile = snapshot.profiles.find((profile) => profile.isActive);
  const pageTitle = t(`nav.${view}` as "nav.control");
  const pageCount =
    view === "control"
      ? snapshot.agents.length
      : view === "profiles"
        ? snapshot.profiles.length
        : undefined;

  const renderReadyView = () => {
    if (view === "history") return <HistoryPage key={libraryRoot} libraryRoot={libraryRoot} onRestored={() => loadSnapshot(libraryRoot)} />;
    if (view === "settings") return <SettingsPage />;
    if (view === "profiles") {
      return (
        <ProfilesPage
          profiles={snapshot.profiles}
          runtimeState={snapshot.runtimeState}
          openTargets={openTargets}
          preferredOpenTargetId={preferredOpenTarget}
          openingTargetId={openingTargetId}
          onOpen={(profileId, relativePath, targetId) =>
            void openRuleSource(targetId, profileId, relativePath)}
          onPrepareCreate={(draft) => backend.previewCreateProfile(draft, libraryRoot)}
          onCreate={createProfile}
          onPrepareDelete={(profileId) => backend.previewDeleteProfile(profileId, libraryRoot)}
          onDelete={deleteProfile}
          onPrepareActivate={(profileId) => backend.previewActivateProfile(profileId, libraryRoot)}
          onActivate={activateProfile}
          onHistory={() => setView("history")}
          onError={(message) => notify({ tone: "error", message })}
        />
      );
    }
    if (!activeProfile || snapshot.runtimeState !== "current" || !snapshot.sourceExists) {
      return (
        <section className={`runtime-gate runtime-gate-${snapshot.runtimeState}`}>
          <span className="runtime-gate-route" aria-hidden="true"><i>P</i><b>→</b><i>C</i></span>
          <p className="eyebrow">{t("runtimeGate.eyebrow")}</p>
          <h1>{activeProfile ? t("runtimeGate.refreshTitle") : t("runtimeGate.selectTitle")}</h1>
          <p>{snapshot.libraryDetail}</p>
          <button className="button button-primary" onClick={() => setView("profiles")}>
            {t("runtimeGate.openProfiles")}
          </button>
        </section>
      );
    }
    return (
      <>
        <AgentScopePanel
          projectRoot={projectRoot}
          disabled={Boolean(busy)}
          onChange={async (nextRoot) => {
            setBusy("loading");
            try {
              const next = await backend.snapshot(libraryRoot, nextRoot || undefined);
              projectRootRef.current = nextRoot;
              setProjectRoot(nextRoot);
              snapshotRef.current = next;
              setSnapshot(next);
              setDesiredConnections(currentConnections(next));
              setPlan(undefined);
            } catch (error) {
              notify({ tone: "error", message: String(error) });
            } finally { setBusy(undefined); }
          }}
        />
        <ProjectionRail
          agents={snapshot.agents}
          sourceDigest={snapshot.sourceDigest}
          activeProfileName={activeProfile.name}
          runtimeState={snapshot.runtimeState}
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
          onContinue={() => void prepareProjectionConfirmation()}
          onApply={() => void applyProjection()}
          onCancelConfirmation={() => setPlan(undefined)}
        />
      </>
    );
  };

  return (
    <div className="app-shell">
      <nav className="sidebar" aria-label={t("nav.primary")}>
        <div className="brand-lockup">
          <BrandMark />
          <div><strong>Agent Rules Manager</strong><span>{t("brand.subtitle")}</span></div>
        </div>

        <div className="nav-primary">
          {(["control", "profiles", "history"] as const).map((item) => (
            <button
              className={`nav-item ${view === item ? "is-active" : ""}`}
              aria-current={view === item ? "page" : undefined}
              onClick={() => setView(item)}
              key={item}
            >
              <span className="nav-icon"><AppIcon name={item === "history" ? "rollback" : item} /></span>
              {t(`nav.${item}` as "nav.control")}
            </button>
          ))}
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
            <h1>{pageTitle}</h1>
            {pageCount !== undefined && <span>{pageCount}</span>}
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
            <NotificationCenter {...notifications} />
            {!backend.isTauri && <span className="runtime-badge">{t("runtime.demo")}</span>}
            {view === "control" && (
              <button className="button button-ghost button-small" disabled={Boolean(busy)} onClick={() => setView("history")}>
                <AppIcon name="rollback" />{t("nav.history")}
              </button>
            )}
          </div>
        </header>

        <main className="workspace">
          {snapshot.libraryState !== "ready" && view !== "settings" && view !== "history" ? (
            <LibrarySetupPanel
              state={snapshot.libraryState}
              detail={snapshot.libraryDetail}
              legacySourcePath={snapshot.legacySourcePath}
              plan={setupPlan}
              planning={busy === "planning-library"}
              applying={busy === "initializing"}
              onPrepare={() => void prepareInitializeConfirmation()}
              onApply={() => void initialize()}
              onCancel={() => setSetupPlan(undefined)}
            />
          ) : (
            renderReadyView()
          )}
        </main>
      </div>
    </div>
  );
}

export default App;
