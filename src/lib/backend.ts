import { invoke } from "@tauri-apps/api/core";
import type {
  ApplyOutcome,
  HistoryRecord,
  RestorePlan,
  RestoreStep,
  ConnectionChange,
  LibraryMutationOutcome,
  LibraryPlan,
  OpenTarget,
  ProfileDraft,
  ProjectionPlan,
  RollbackOutcome,
  WorkspaceSnapshot,
} from "./types";
import { buildProjectionPlan } from "./plan";
import { demoAgents } from "./agentCatalog";

const isTauri = "__TAURI_INTERNALS__" in window;

const demoRoot = "/Users/baizhukui/.agent-rules";

let demoSnapshot: WorkspaceSnapshot = {
  libraryRoot: demoRoot,
  libraryState: "ready",
  libraryDetail: "Profile `work` is rendered and selected on this machine.",
  runtimeState: "current",
  sourcePath: `${demoRoot}/current/AGENTS.md`,
  sourceExists: true,
  sourceDigest: "8d9f20b751a4",
  sourceModifiedAt: "2026-08-26T08:36:00Z",
  activeProfileId: "work",
  activeSourcePath: `${demoRoot}/profiles/work/AGENTS.md`,
  profiles: [
    {
      id: "default",
      name: "Default",
      description: "A minimal baseline for any machine.",
      files: [
        { path: "AGENTS.md", digest: "62a4fb4f20c1", modifiedAt: "2026-08-26T08:36:00Z" },
      ],
      isActive: false,
    },
    {
      id: "work",
      name: "Work machine",
      description: "Repository delivery and review conventions used at work.",
      files: [
        { path: "AGENTS.md", digest: "5c3fe71b9d02", modifiedAt: "2026-08-24T04:18:00Z" },
      ],
      isActive: true,
    },
    {
      id: "personal",
      name: "Personal laptop",
      description: "Preferences for experiments and open-source maintenance.",
      files: [{ path: "AGENTS.md", digest: "7ef49a3ecb44", modifiedAt: "2026-08-21T13:05:00Z" }],
      isActive: false,
    },
  ],
  latestBackup: "20260822T164218.190Z",
  latestLibraryBackup: "20260826T083600.000Z",
  agents: [
    {
      id: "claude",
      label: "Claude Code",
      targetPath: "/Users/baizhukui/.claude/CLAUDE.md",
      mode: "symlink",
      state: "inSync",
      targetKind: "connectedLink",
      installed: true,
      connected: true,
      detectionDetail: "Detected configuration at /Users/baizhukui/.claude.",
      detail: "Linked directly to the canonical rules.",
    },
    {
      id: "codex",
      label: "Codex",
      targetPath: "/Users/baizhukui/.codex/AGENTS.md",
      mode: "symlink",
      state: "inSync",
      targetKind: "connectedLink",
      installed: true,
      connected: true,
      detectionDetail: "Detected codex in the ChatGPT application.",
      detail: "Linked directly to the canonical rules.",
    },
    {
      id: "grok",
      label: "Grok",
      targetPath: "/Users/baizhukui/.grok/AGENTS.md",
      mode: "symlink",
      state: "ready",
      targetKind: "missing",
      installed: false,
      connected: false,
      detectionDetail: "No supported command or configuration directory was detected.",
      detail: "No native rules file exists yet.",
    },
    {
      id: "opencode",
      label: "OpenCode",
      targetPath: "/Users/baizhukui/.config/opencode/AGENTS.md",
      mode: "symlink",
      state: "ready",
      targetKind: "independentFile",
      installed: true,
      connected: false,
      detectionDetail: "Detected configuration at /Users/baizhukui/.config/opencode.",
      detail: "Uses an independent native rules file.",
    },
    {
      id: "qwen",
      label: "Qwen Code",
      targetPath: "/Users/baizhukui/.qwen/QWEN.md",
      mode: "symlink",
      state: "ready",
      targetKind: "missing",
      installed: true,
      connected: false,
      detectionDetail: "Detected configuration at /Users/baizhukui/.qwen.",
      detail: "No native rules file exists yet.",
    },
  ],
};
demoSnapshot.agents = demoAgents("global", "/Users/baizhukui").map((agent) => ({
  ...agent, ...demoSnapshot.agents.find((existing) => existing.id === agent.id),
}));
const demoProjects = new Map<string, WorkspaceSnapshot["agents"]>();
function scopedDemo(projectRoot?: string): WorkspaceSnapshot {
  if (!projectRoot) return demoSnapshot;
  if (!projectRoot.startsWith("/") || projectRoot.split("/").includes("..")) {
    throw new Error("Enter an absolute project directory without '..'.");
  }
  if (!demoProjects.has(projectRoot)) demoProjects.set(projectRoot, demoAgents("project", projectRoot));
  return { ...demoSnapshot, agents: demoProjects.get(projectRoot)! };
}
let demoRollbackProject: string | undefined;
let demoRollbackSnapshot: WorkspaceSnapshot | undefined;
let demoLibraryRollbackSnapshot: WorkspaceSnapshot | undefined;
let demoLibraryRollbackPaths: string[] | undefined;
let demoLibraryRollbackId: string | undefined;

type DemoPoint = { record: HistoryRecord; snapshot: WorkspaceSnapshot; projects: [string, WorkspaceSnapshot["agents"]][] };
const demoHistory: DemoPoint[] = [];
let demoHistorySequence = 0;
function recordDemo(operation: string, subjects: string[], paths: string[], scope: HistoryRecord["scope"] = "profiles") {
  const record: HistoryRecord = {
    id: `demo-history-${++demoHistorySequence}`, createdAt: new Date().toISOString(), operation, subjects, paths, scope, restorable: true,
  };
  demoHistory.push({ record, snapshot: structuredClone(demoSnapshot), projects: structuredClone([...demoProjects]) });
}
// Explicit browser-only sample history; no native files are inspected or changed.
const demoInitial = structuredClone(demoSnapshot);
demoSnapshot.profiles = demoSnapshot.profiles.filter((p) => p.id === "default").map((p) => ({ ...p, isActive: true }));
demoSnapshot.activeProfileId = "default";
const demoStart = structuredClone(demoSnapshot);
demoSnapshot = { ...demoSnapshot, libraryState: "empty", profiles: [], activeProfileId: undefined, activeSourcePath: undefined, runtimeState: "missing", sourceExists: false, agents: demoSnapshot.agents.map((a) => ({ ...a, connected: false, targetKind: "missing", state: "ready" })) };
recordDemo("historyBaseline", [], [], "recovery");
demoSnapshot = demoStart;
recordDemo("initialize", ["Default"], [`${demoRoot}/profiles/default/AGENTS.md`]);
for (const profile of demoInitial.profiles.filter((p) => p.id !== "default")) {
  demoSnapshot.profiles.push({ ...profile, isActive: false });
  recordDemo("createProfile", [profile.name], [`${demoRoot}/profiles/${profile.id}/AGENTS.md`]);
}
demoSnapshot = demoInitial;
recordDemo("activateProfile", ["Work machine"], [`${demoRoot}/current`]);
for (let i = 0; i < demoHistory.length; i++) demoHistory[i].record.createdAt = new Date(Date.now() - (demoHistory.length - i) * 3600000).toISOString();

function demoRestorePlan(targetId: string): RestorePlan {
  const index = demoHistory.findIndex((point) => point.record.id === targetId);
  if (index < 0) throw new Error("Unknown recovery point");
  const target = demoHistory[index];
  const steps: RestoreStep[] = [];
  const add = (path: string, before: string, after: string) => {
    if (before !== after) steps.push({ path, before, after, action: after === "missing" ? "remove" : after.startsWith("link:") ? "restoreLink" : "restoreFile" });
  };
  add("<local state>/machine.json", `profile:${demoSnapshot.activeProfileId ?? "—"}`, `profile:${target.snapshot.activeProfileId ?? "—"}`);
  const ids = new Set([...demoSnapshot.profiles, ...target.snapshot.profiles].map((p) => p.id));
  for (const id of ids) {
    const before = demoSnapshot.profiles.find((p) => p.id === id);
    const after = target.snapshot.profiles.find((p) => p.id === id);
    add(`${demoRoot}/profiles/${id}/AGENTS.md`, before ? "file" : "missing", after ? "file" : "missing");
    add(`${demoRoot}/profiles/${id}/profile.json`, before ? "file" : "missing", after ? "file" : "missing");
  }
  const compareAgents = (before: WorkspaceSnapshot["agents"], after: WorkspaceSnapshot["agents"]) => {
    for (const agent of before) {
      const previous = after.find((a) => a.id === agent.id);
      add(agent.targetPath, agent.connected ? `link:${demoRoot}/current/AGENTS.md` : agent.targetKind === "missing" ? "missing" : "file",
        previous?.connected ? `link:${demoRoot}/current/AGENTS.md` : !previous || previous.targetKind === "missing" ? "missing" : "file");
    }
    for (const agent of after.filter((a) => !before.some((b) => a.id === b.id))) {
      add(agent.targetPath, "missing", agent.connected ? `link:${demoRoot}/current/AGENTS.md` : agent.targetKind === "missing" ? "missing" : "file");
    }
  };
  compareAgents(demoSnapshot.agents, target.snapshot.agents);
  const beforeProjects = new Map(demoProjects), afterProjects = new Map(target.projects);
  for (const root of new Set([...beforeProjects.keys(), ...afterProjects.keys()])) compareAgents(beforeProjects.get(root) ?? [], afterProjects.get(root) ?? []);
  return { target: structuredClone(target.record), token: JSON.stringify([demoHistory.map((p) => p.record.id), demoSnapshot, [...demoProjects]]),
    operations: demoHistory.slice(index + 1).map((p) => structuredClone(p.record)).reverse(), steps, conflicts: [], blocked: false };
}

const demoOpenTargets: OpenTarget[] = [
  { id: "default", label: "Default app", kind: "default" },
  { id: "vscode", label: "Visual Studio Code", kind: "editor" },
  { id: "cursor", label: "Cursor", kind: "editor" },
  { id: "typora", label: "Typora", kind: "editor" },
  { id: "textedit", label: "TextEdit", kind: "editor" },
  { id: "intellij-idea", label: "IntelliJ IDEA", kind: "editor" },
  { id: "rider", label: "Rider", kind: "editor" },
  { id: "webstorm", label: "WebStorm", kind: "editor" },
  { id: "finder", label: "Finder", kind: "reveal" },
  { id: "terminal", label: "Terminal", kind: "terminal" },
];

function step(path: string, action: string, summary: string) {
  return { path, action, summary };
}

function plan(operation: string, summary: string, steps: ReturnType<typeof step>[]): LibraryPlan {
  return { operation, blocked: false, changeCount: steps.length, summary, steps };
}

function mutation(paths: string[]): LibraryMutationOutcome {
  return { changed: paths, backupId: paths.length ? "demo-library-snapshot" : undefined };
}

export const backend = {
  isTauri,
  async history(libraryRoot?: string): Promise<HistoryRecord[]> {
    if (isTauri) return invoke("get_operation_history", { libraryRoot });
    return demoHistory.map((p) => structuredClone(p.record)).reverse();
  },
  async previewRestore(targetId: string, libraryRoot?: string): Promise<RestorePlan> {
    if (isTauri) return invoke("preview_restore_history", { targetId, libraryRoot });
    return demoRestorePlan(targetId);
  },
  async restoreHistory(targetId: string, token: string, libraryRoot?: string): Promise<RollbackOutcome> {
    if (isTauri) return invoke("restore_history", { targetId, token, libraryRoot });
    const plan = demoRestorePlan(targetId);
    if (plan.token !== token || plan.blocked || !plan.steps.length) throw new Error("Recovery preview is stale or has no changes; preview again");
    const target = demoHistory.find((p) => p.record.id === targetId)!;
    demoSnapshot = structuredClone(target.snapshot);
    demoProjects.clear();
    for (const [root, agents] of structuredClone(target.projects)) demoProjects.set(root, agents);
    recordDemo("restoreHistory", [target.record.createdAt], plan.steps.map((s) => s.path), "recovery");
    return { restored: plan.steps.map((s) => s.path), backupId: demoHistory.at(-1)!.record.id };
  },
  async snapshot(libraryRoot?: string, projectRoot?: string): Promise<WorkspaceSnapshot> {
    if (isTauri) return invoke("get_workspace_snapshot", { libraryRoot, projectRoot });
    return structuredClone(scopedDemo(projectRoot));
  },
  async previewInitialize(libraryRoot?: string): Promise<LibraryPlan> {
    if (isTauri) return invoke("preview_initialize", { libraryRoot });
    return plan("initialize", "Create a Profile library and activate its default Profile.", [
      step(`${demoRoot}/schema.json`, "createSchema", "Create the syncable schema."),
      step(`${demoRoot}/profiles/default/profile.json`, "createProfile", "Create the default Profile manifest."),
      step(`${demoRoot}/profiles/default/AGENTS.md`, "createRules", "Create its required rules entrypoint."),
      step(`${demoRoot}/.gitignore`, "createIgnore", "Exclude local runtime state."),
      step(`${demoRoot}/.runtime/default-demo`, "renderRuntime", "Render immutable output."),
      step(`${demoRoot}/current`, "switchCurrent", "Select the default runtime."),
      step("<local state>/machine.json", "selectLocalProfile", "Record the machine selection."),
    ]);
  },
  async initialize(libraryRoot?: string): Promise<LibraryMutationOutcome> {
    if (isTauri) return invoke("initialize_library", { libraryRoot });
    demoSnapshot.libraryState = "ready";
    recordDemo("initialize", ["Default"], [`${demoRoot}/schema.json`]);
    return mutation([`${demoRoot}/schema.json`, `${demoRoot}/profiles/default/AGENTS.md`]);
  },
  async previewCreateProfile(draft: ProfileDraft, libraryRoot?: string): Promise<LibraryPlan> {
    if (isTauri) {
      return invoke("preview_create_profile", {
        id: draft.id,
        name: draft.name,
        description: draft.description,
        libraryRoot,
      });
    }
    const directory = `${demoRoot}/profiles/${draft.id}`;
    const blocked = demoSnapshot.profiles.some((profile) => profile.id === draft.id);
    return {
      operation: "createProfile",
      blocked,
      changeCount: blocked ? 0 : 2,
      summary: blocked
        ? `Profile \`${draft.id}\` already exists; no file will be overwritten.`
        : `Create Profile \`${draft.name}\` with a required AGENTS.md entrypoint.`,
      steps: blocked
        ? []
        : [
            step(`${directory}/profile.json`, "createProfile", "Create the Profile manifest."),
            step(`${directory}/AGENTS.md`, "createRules", "Create the required AGENTS.md."),
          ],
    };
  },
  async createProfile(draft: ProfileDraft, libraryRoot?: string): Promise<LibraryMutationOutcome> {
    if (isTauri) {
      return invoke("create_profile", {
        id: draft.id,
        name: draft.name,
        description: draft.description,
        libraryRoot,
      });
    }
    demoSnapshot.profiles = [
      ...demoSnapshot.profiles,
      { ...draft, files: [{ path: "AGENTS.md", digest: "new000000000" }], isActive: false },
    ];
    recordDemo("createProfile", [draft.name], [`${demoRoot}/profiles/${draft.id}/AGENTS.md`]);
    return mutation([
      `${demoRoot}/profiles/${draft.id}/profile.json`,
      `${demoRoot}/profiles/${draft.id}/AGENTS.md`,
    ]);
  },
  async previewDeleteProfile(profileId: string, libraryRoot?: string): Promise<LibraryPlan> {
    if (isTauri) return invoke("preview_delete_profile", { profileId, libraryRoot });
    const profile = demoSnapshot.profiles.find((candidate) => candidate.id === profileId);
    const blocked = !profile || profile.isActive;
    const directory = `${demoRoot}/profiles/${profileId}`;
    return {
      operation: "deleteProfile",
      blocked,
      changeCount: blocked ? 0 : 3,
      summary: !profile
        ? `Profile \`${profileId}\` is unavailable.`
        : profile.isActive
          ? `Profile \`${profileId}\` is active on this machine; switch to another Profile before deleting it.`
          : `Delete inactive Profile \`${profileId}\` and its AGENTS.md after creating a rollback snapshot.`,
      steps: blocked
        ? []
        : [
            step(
              `${directory}/AGENTS.md`,
              "deleteProfileSource",
              "Back up and delete the Profile’s AGENTS.md.",
            ),
            step(
              `${directory}/profile.json`,
              "deleteProfileManifest",
              "Delete the Profile manifest after every declared source is snapshotted.",
            ),
            step(directory, "deleteProfileDirectory", "Remove the empty Profile directory."),
          ],
    };
  },
  async deleteProfile(profileId: string, libraryRoot?: string): Promise<LibraryMutationOutcome> {
    if (isTauri) return invoke("delete_profile", { profileId, libraryRoot });
    const profile = demoSnapshot.profiles.find((candidate) => candidate.id === profileId);
    if (!profile) throw new Error(`Unknown profile: ${profileId}`);
    if (profile.isActive) {
      throw new Error(
        `Profile \`${profileId}\` is active on this machine; switch to another Profile before deleting it.`,
      );
    }
    const preview = await this.previewDeleteProfile(profileId, libraryRoot);
    const changed = preview.steps.map((item) => item.path);
    const backupId = "demo-delete-profile-snapshot";
    demoLibraryRollbackSnapshot = structuredClone(demoSnapshot);
    demoLibraryRollbackPaths = changed;
    demoLibraryRollbackId = backupId;
    demoSnapshot = {
      ...demoSnapshot,
      profiles: demoSnapshot.profiles.filter((candidate) => candidate.id !== profileId),
      latestLibraryBackup: backupId,
    };
    recordDemo("deleteProfile", [profile.name], changed);
    return { changed, backupId };
  },
  async previewActivateProfile(profileId: string, libraryRoot?: string): Promise<LibraryPlan> {
    if (isTauri) return invoke("preview_activate_profile", { profileId, libraryRoot });
    if (demoSnapshot.activeProfileId === profileId && demoSnapshot.runtimeState === "current") {
      return plan("activateProfile", `Profile \`${profileId}\` is already current on this machine.`, []);
    }
    return plan(
      "activateProfile",
      `Render and activate Profile \`${profileId}\` without changing synced sources.`,
      [
        step(`${demoRoot}/.runtime/${profileId}-demo`, "renderRuntime", "Render immutable output."),
        step(`${demoRoot}/current`, "switchCurrent", "Switch the stable current link."),
        step("<local state>/machine.json", "selectLocalProfile", "Record the local selection."),
      ],
    );
  },
  async activateProfile(profileId: string, libraryRoot?: string): Promise<LibraryMutationOutcome> {
    if (isTauri) return invoke("activate_profile", { profileId, libraryRoot });
    const profile = demoSnapshot.profiles.find((candidate) => candidate.id === profileId);
    if (!profile) throw new Error(`Unknown profile: ${profileId}`);
    const firstFile = profile.files[0];
    demoSnapshot = {
      ...demoSnapshot,
      activeProfileId: profileId,
      activeSourcePath: firstFile
        ? `${demoRoot}/profiles/${profile.id}/${firstFile.path}`
        : undefined,
      runtimeState: "current",
      libraryDetail: `Profile \`${profileId}\` is rendered and selected on this machine.`,
      sourceDigest: `${profileId}00000000`.slice(0, 12),
      profiles: demoSnapshot.profiles.map((candidate) => ({
        ...candidate,
        isActive: candidate.id === profileId,
      })),
    };
    recordDemo("activateProfile", [profile.name], [`${demoRoot}/current`, "<local state>/machine.json"]);
    return mutation([`${demoRoot}/current`, "<local state>/machine.json"]);
  },
  async preview(changes: ConnectionChange[], libraryRoot?: string, projectRoot?: string): Promise<ProjectionPlan> {
    if (isTauri) {
      return invoke("preview_apply", { changes, libraryRoot, projectRoot });
    }
    return buildProjectionPlan(scopedDemo(projectRoot), changes);
  },
  async openTargets(): Promise<OpenTarget[]> {
    if (isTauri) return invoke("get_open_targets");
    return structuredClone(demoOpenTargets);
  },
  async openRuleSource(
    targetId: string,
    profileId: string,
    relativePath: string,
    libraryRoot?: string,
  ): Promise<boolean> {
    if (isTauri) {
      await invoke("open_rule_source", { targetId, profileId, relativePath, libraryRoot });
      return true;
    }
    return false;
  },
  async apply(
    changes: ConnectionChange[],
    confirmExistingFiles: boolean,
    libraryRoot?: string,
    projectRoot?: string,
  ): Promise<ApplyOutcome> {
    if (isTauri) {
      return invoke("apply_rules", { changes, confirmExistingFiles, libraryRoot, projectRoot });
    }
    const current = scopedDemo(projectRoot);
    const preview = buildProjectionPlan(current, changes);
    if (preview.blocked) throw new Error("Projection is blocked.");
    const requested = new Map(changes.map((change) => [change.agentId, change.connected]));
    const existingFiles = current.agents.filter(
      (agent) =>
        requested.get(agent.id) === true &&
        (agent.targetKind === "independentFile" || agent.targetKind === "invalidManagedFile"),
    );
    if (existingFiles.length > 0 && !confirmExistingFiles) {
      throw new Error("Confirmation is required before existing files are backed up and replaced.");
    }
    const changed: string[] = [];
    const before = structuredClone(current);
    const next: WorkspaceSnapshot = {
      ...current,
      latestBackup: "demo-apply-snapshot",
      agents: current.agents.map((agent) => {
        const connected = requested.get(agent.id);
        if (connected !== undefined && connected !== agent.connected) {
          changed.push(agent.id);
          return connected
            ? {
                ...agent,
                mode: "symlink" as const,
                state: "inSync" as const,
                targetKind: "connectedLink" as const,
                connected: true,
                detail: "Linked directly to the canonical rules.",
              }
            : {
                ...agent,
                mode: "symlink" as const,
                state: "ready" as const,
                targetKind: "independentFile" as const,
                connected: false,
                detail: "Uses an independent native rules file.",
              };
        }
        return agent;
      }),
    };
    if (changed.length) {
      demoRollbackSnapshot = before;
      demoRollbackProject = projectRoot;
      if (projectRoot) demoProjects.set(projectRoot, next.agents);
      else demoSnapshot.agents = next.agents;
      demoSnapshot.latestBackup = "demo-apply-snapshot";
      recordDemo("connectionChange", next.agents.filter((a) => changed.includes(a.id)).map((a) => `${a.connected ? "connect" : "disconnect"}:${a.label}`), next.agents.filter((a) => changed.includes(a.id)).map((a) => a.targetPath), "agents");
    }
    return {
      changed,
      backupId: changed.length ? "demo-apply-snapshot" : undefined,
      preservedBackupDir: existingFiles.length
        ? "<local state>/backups/originals/demo-apply-snapshot"
        : undefined,
    };
  },
  async rollback(libraryRoot?: string): Promise<RollbackOutcome> {
    if (isTauri) {
      return invoke("rollback_latest", { libraryRoot });
    }
    if (!demoRollbackSnapshot) throw new Error("No rollback snapshot is available");
    const current = scopedDemo(demoRollbackProject);
    const originalAgents = demoRollbackSnapshot.agents;
    if (demoRollbackProject) demoProjects.set(demoRollbackProject, originalAgents);
    else demoSnapshot.agents = originalAgents;
    demoSnapshot.latestBackup = undefined;
    demoRollbackProject = undefined;
    demoRollbackSnapshot = undefined;
    return {
      restored: current.agents
        .filter((agent) => {
          const original = originalAgents.find((candidate) => candidate.id === agent.id);
          return original?.connected !== agent.connected;
        })
        .map((agent) => agent.targetPath),
      backupId: "demo-apply-snapshot",
    };
  },
  async rollbackLibrary(libraryRoot?: string): Promise<RollbackOutcome> {
    if (isTauri) return invoke("rollback_library_latest", { libraryRoot });
    if (!demoLibraryRollbackSnapshot) {
      demoSnapshot.latestLibraryBackup = undefined;
      return { restored: [`${demoRoot}/current`], backupId: "demo-library-snapshot" };
    }
    const restored = demoLibraryRollbackPaths ?? [];
    const backupId = demoLibraryRollbackId ?? "demo-library-snapshot";
    demoSnapshot = demoLibraryRollbackSnapshot;
    demoLibraryRollbackSnapshot = undefined;
    demoLibraryRollbackPaths = undefined;
    demoLibraryRollbackId = undefined;
    return { restored, backupId };
  },
};
