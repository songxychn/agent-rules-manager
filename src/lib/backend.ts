import { invoke } from "@tauri-apps/api/core";
import type {
  ApplyOutcome,
  ConnectionChange,
  LibraryMutationOutcome,
  LibraryPlan,
  OpenTarget,
  ProfileDraft,
  ProfileFileDraft,
  ProjectionPlan,
  RollbackOutcome,
  WorkspaceSnapshot,
} from "./types";
import { buildProjectionPlan } from "./plan";

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
        { path: "rules/safety.md", digest: "a1d19e80bc3e", modifiedAt: "2026-08-25T09:12:00Z" },
      ],
      isActive: false,
    },
    {
      id: "work",
      name: "Work machine",
      description: "Repository delivery and review conventions used at work.",
      files: [
        { path: "AGENTS.md", digest: "5c3fe71b9d02", modifiedAt: "2026-08-24T04:18:00Z" },
        { path: "rules/review.md", digest: "0b79a43e04a8", modifiedAt: "2026-08-24T04:18:00Z" },
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
let demoRollbackSnapshot: WorkspaceSnapshot | undefined;
let demoLibraryRollbackSnapshot: WorkspaceSnapshot | undefined;
let demoLibraryRollbackPaths: string[] | undefined;
let demoLibraryRollbackId: string | undefined;

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
  async snapshot(libraryRoot?: string): Promise<WorkspaceSnapshot> {
    if (isTauri) return invoke("get_workspace_snapshot", { libraryRoot });
    return structuredClone(demoSnapshot);
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
    return mutation([
      `${demoRoot}/profiles/${draft.id}/profile.json`,
      `${demoRoot}/profiles/${draft.id}/AGENTS.md`,
    ]);
  },
  async previewAddProfileFile(
    draft: ProfileFileDraft,
    libraryRoot?: string,
  ): Promise<LibraryPlan> {
    if (isTauri) return invoke("preview_add_profile_file", { ...draft, libraryRoot });
    const profile = demoSnapshot.profiles.find((candidate) => candidate.id === draft.profileId);
    const blocked = !profile || profile.files.some((file) => file.path === draft.relativePath);
    const path = `${demoRoot}/profiles/${draft.profileId}/${draft.relativePath}`;
    return {
      operation: "addProfileFile",
      blocked,
      changeCount: blocked ? 0 : 2,
      summary: blocked
        ? `\`${draft.relativePath}\` already exists or the Profile is unavailable.`
        : `Add \`${draft.relativePath}\` after the existing sources in Profile \`${draft.profileId}\`.`,
      steps: blocked
        ? []
        : [
            step(
              `${demoRoot}/profiles/${draft.profileId}/profile.json`,
              "updateProfile",
              "Append the ordered path.",
            ),
            step(path, "createRuleFile", "Create the Markdown source."),
          ],
    };
  },
  async addProfileFile(
    draft: ProfileFileDraft,
    libraryRoot?: string,
  ): Promise<LibraryMutationOutcome> {
    if (isTauri) return invoke("add_profile_file", { ...draft, libraryRoot });
    demoSnapshot = {
      ...demoSnapshot,
      runtimeState:
        demoSnapshot.activeProfileId === draft.profileId
          ? "stale"
          : demoSnapshot.runtimeState,
      profiles: demoSnapshot.profiles.map((profile) =>
        profile.id === draft.profileId
          ? {
              ...profile,
              files: [...profile.files, { path: draft.relativePath, digest: "newfile00000" }],
            }
          : profile,
      ),
    };
    return mutation([
      `${demoRoot}/profiles/${draft.profileId}/profile.json`,
      `${demoRoot}/profiles/${draft.profileId}/${draft.relativePath}`,
    ]);
  },
  async previewRemoveProfileFile(
    draft: ProfileFileDraft,
    libraryRoot?: string,
  ): Promise<LibraryPlan> {
    if (isTauri) return invoke("preview_remove_profile_file", { ...draft, libraryRoot });
    const profile = demoSnapshot.profiles.find((candidate) => candidate.id === draft.profileId);
    const fileIndex = profile?.files.findIndex((file) => file.path === draft.relativePath) ?? -1;
    const blocked = !profile || fileIndex <= 0;
    const manifestPath = `${demoRoot}/profiles/${draft.profileId}/profile.json`;
    const sourcePath = `${demoRoot}/profiles/${draft.profileId}/${draft.relativePath}`;
    return {
      operation: "removeProfileFile",
      blocked,
      changeCount: blocked ? 0 : 2,
      summary: !profile
        ? `Profile \`${draft.profileId}\` is unavailable.`
        : fileIndex === 0
          ? `\`AGENTS.md\` is the required first source in Profile \`${draft.profileId}\` and cannot be removed.`
          : fileIndex < 0
            ? `\`${draft.relativePath}\` is not declared by Profile \`${draft.profileId}\`; no file will be deleted.`
            : `Remove \`${draft.relativePath}\` from Profile \`${draft.profileId}\` and delete its declared Markdown source with rollback protection.`,
      steps: blocked
        ? []
        : [
            step(
              manifestPath,
              "updateProfile",
              "Remove the Markdown path from the ordered instruction manifest.",
            ),
            step(
              sourcePath,
              "deleteRuleFile",
              "Delete the declared Markdown source after snapshotting its exact contents.",
            ),
          ],
    };
  },
  async removeProfileFile(
    draft: ProfileFileDraft,
    libraryRoot?: string,
  ): Promise<LibraryMutationOutcome> {
    if (isTauri) return invoke("remove_profile_file", { ...draft, libraryRoot });
    const profile = demoSnapshot.profiles.find((candidate) => candidate.id === draft.profileId);
    const fileIndex = profile?.files.findIndex((file) => file.path === draft.relativePath) ?? -1;
    if (!profile || fileIndex < 0) {
      throw new Error(`Unknown Profile source: ${draft.profileId}/${draft.relativePath}`);
    }
    if (fileIndex === 0) {
      throw new Error(
        `\`AGENTS.md\` is the required first source in Profile \`${draft.profileId}\` and cannot be removed.`,
      );
    }
    const preview = await this.previewRemoveProfileFile(draft, libraryRoot);
    const changed = preview.steps.map((item) => item.path);
    const backupId = "demo-remove-profile-file-snapshot";
    demoLibraryRollbackSnapshot = structuredClone(demoSnapshot);
    demoLibraryRollbackPaths = changed;
    demoLibraryRollbackId = backupId;
    demoSnapshot = {
      ...demoSnapshot,
      runtimeState: profile.isActive ? "stale" : demoSnapshot.runtimeState,
      latestLibraryBackup: backupId,
      profiles: demoSnapshot.profiles.map((candidate) =>
        candidate.id === draft.profileId
          ? {
              ...candidate,
              files: candidate.files.filter((file) => file.path !== draft.relativePath),
        }
          : candidate,
      ),
    };
    return { changed, backupId };
  },
  async previewDeleteProfile(profileId: string, libraryRoot?: string): Promise<LibraryPlan> {
    if (isTauri) return invoke("preview_delete_profile", { profileId, libraryRoot });
    const profile = demoSnapshot.profiles.find((candidate) => candidate.id === profileId);
    const blocked = !profile || profile.isActive;
    const directory = `${demoRoot}/profiles/${profileId}`;
    const sourceDirectories = new Set<string>();
    for (const file of profile?.files ?? []) {
      const parts = file.path.split("/");
      for (let length = 1; length < parts.length; length += 1) {
        sourceDirectories.add(`${directory}/${parts.slice(0, length).join("/")}`);
      }
    }
    const directories = [...sourceDirectories].sort(
      (left, right) => right.split("/").length - left.split("/").length || right.localeCompare(left),
    );
    directories.push(directory);
    return {
      operation: "deleteProfile",
      blocked,
      changeCount: blocked ? 0 : (profile?.files.length ?? 0) + 1 + directories.length,
      summary: !profile
        ? `Profile \`${profileId}\` is unavailable.`
        : profile.isActive
          ? `Profile \`${profileId}\` is active on this machine; switch to another Profile before deleting it.`
          : `Delete inactive Profile \`${profileId}\` and its ${profile.files.length} declared Markdown source(s) after creating a rollback snapshot.`,
      steps: blocked
        ? []
        : [
            ...profile.files.map((file) =>
              step(
                `${directory}/${file.path}`,
                "deleteProfileSource",
                "Delete this declared Markdown source after snapshotting its exact contents.",
              ),
            ),
            step(
              `${directory}/profile.json`,
              "deleteProfileManifest",
              "Delete the Profile manifest after every declared source is snapshotted.",
            ),
            ...directories.map((path) =>
              step(
                path,
                "deleteProfileDirectory",
                path === directory
                  ? "Remove the empty Profile directory."
                  : "Remove this empty Profile source directory.",
              ),
            ),
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
    return mutation([`${demoRoot}/current`, "<local state>/machine.json"]);
  },
  async preview(changes: ConnectionChange[], libraryRoot?: string): Promise<ProjectionPlan> {
    if (isTauri) {
      return invoke("preview_apply", { changes, libraryRoot });
    }
    return buildProjectionPlan(demoSnapshot, changes);
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
  async apply(changes: ConnectionChange[], libraryRoot?: string): Promise<ApplyOutcome> {
    if (isTauri) {
      return invoke("apply_rules", { changes, libraryRoot });
    }
    const requested = new Map(changes.map((change) => [change.agentId, change.connected]));
    const changed: string[] = [];
    const before = structuredClone(demoSnapshot);
    demoSnapshot = {
      ...demoSnapshot,
      latestBackup: "demo-apply-snapshot",
      agents: demoSnapshot.agents.map((agent) => {
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
    if (changed.length) demoRollbackSnapshot = before;
    return { changed, backupId: changed.length ? "demo-apply-snapshot" : undefined };
  },
  async rollback(libraryRoot?: string): Promise<RollbackOutcome> {
    if (isTauri) {
      return invoke("rollback_latest", { libraryRoot });
    }
    if (!demoRollbackSnapshot) throw new Error("No rollback snapshot is available");
    const current = demoSnapshot;
    demoSnapshot = { ...demoRollbackSnapshot, latestBackup: undefined };
    demoRollbackSnapshot = undefined;
    return {
      restored: current.agents
        .filter((agent) => {
          const original = demoSnapshot.agents.find((candidate) => candidate.id === agent.id);
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
