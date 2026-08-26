import { invoke } from "@tauri-apps/api/core";
import type {
  ApplyOutcome,
  ConnectionChange,
  LibraryMutationOutcome,
  LibraryPlan,
  OpenTarget,
  PackDraft,
  PackFileDraft,
  ProfileDraft,
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
  activeSourcePath: `${demoRoot}/packs/base/AGENTS.md`,
  packs: [
    {
      id: "base",
      name: "Base rules",
      description: "Everyday engineering and safety defaults.",
      files: [
        { path: "AGENTS.md", digest: "62a4fb4f20c1", modifiedAt: "2026-08-26T08:36:00Z" },
        { path: "rules/safety.md", digest: "a1d19e80bc3e", modifiedAt: "2026-08-25T09:12:00Z" },
      ],
    },
    {
      id: "work",
      name: "Work conventions",
      description: "Repository delivery and review conventions used at work.",
      files: [
        { path: "AGENTS.md", digest: "5c3fe71b9d02", modifiedAt: "2026-08-24T04:18:00Z" },
        { path: "rules/review.md", digest: "0b79a43e04a8", modifiedAt: "2026-08-24T04:18:00Z" },
      ],
    },
    {
      id: "personal",
      name: "Personal projects",
      description: "Preferences for experiments and open-source maintenance.",
      files: [
        { path: "AGENTS.md", digest: "7ef49a3ecb44", modifiedAt: "2026-08-21T13:05:00Z" },
      ],
    },
  ],
  profiles: [
    {
      id: "default",
      name: "Default",
      description: "A minimal baseline for any machine.",
      packIds: ["base"],
      isActive: false,
    },
    {
      id: "work",
      name: "Work machine",
      description: "Base safety plus work delivery conventions.",
      packIds: ["base", "work"],
      isActive: true,
    },
    {
      id: "personal",
      name: "Personal laptop",
      description: "Base rules plus personal project preferences.",
      packIds: ["base", "personal"],
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
    return plan("initialize", "Create a Rule Pack library and activate its default Profile.", [
      step(`${demoRoot}/schema.json`, "createSchema", "Create the syncable schema."),
      step(`${demoRoot}/packs/base/pack.json`, "createPack", "Create the base Pack manifest."),
      step(`${demoRoot}/packs/base/AGENTS.md`, "createRules", "Create the required rules entrypoint."),
      step(`${demoRoot}/profiles/default.json`, "createProfile", "Create the default Profile."),
      step(`${demoRoot}/.gitignore`, "createIgnore", "Exclude local runtime state."),
      step(`${demoRoot}/.runtime/default-demo`, "renderRuntime", "Render immutable output."),
      step(`${demoRoot}/current`, "switchCurrent", "Select the default runtime."),
      step("<local state>/machine.json", "selectLocalProfile", "Record the machine selection."),
    ]);
  },
  async initialize(libraryRoot?: string): Promise<LibraryMutationOutcome> {
    if (isTauri) return invoke("initialize_library", { libraryRoot });
    demoSnapshot.libraryState = "ready";
    return mutation([`${demoRoot}/schema.json`, `${demoRoot}/packs/base/AGENTS.md`]);
  },
  async previewCreatePack(draft: PackDraft, libraryRoot?: string): Promise<LibraryPlan> {
    if (isTauri) return invoke("preview_create_pack", { ...draft, libraryRoot });
    const directory = `${demoRoot}/packs/${draft.id}`;
    const blocked = demoSnapshot.packs.some((pack) => pack.id === draft.id);
    return {
      operation: "createPack",
      blocked,
      changeCount: blocked ? 0 : 2,
      summary: blocked
        ? `Rule Pack \`${draft.id}\` already exists; no file will be overwritten.`
        : `Create Rule Pack \`${draft.name}\` with a required AGENTS.md entrypoint.`,
      steps: blocked
        ? []
        : [
            step(`${directory}/pack.json`, "createPack", "Create the Rule Pack manifest."),
            step(`${directory}/AGENTS.md`, "createRules", "Create the required AGENTS.md."),
          ],
    };
  },
  async createPack(draft: PackDraft, libraryRoot?: string): Promise<LibraryMutationOutcome> {
    if (isTauri) return invoke("create_pack", { ...draft, libraryRoot });
    const files = [{ path: "AGENTS.md", digest: "new000000000" }];
    demoSnapshot.packs = [
      ...demoSnapshot.packs,
      { id: draft.id, name: draft.name, description: draft.description, files },
    ];
    return mutation([`${demoRoot}/packs/${draft.id}/pack.json`, `${demoRoot}/packs/${draft.id}/AGENTS.md`]);
  },
  async previewAddPackFile(draft: PackFileDraft, libraryRoot?: string): Promise<LibraryPlan> {
    if (isTauri) return invoke("preview_add_pack_file", { ...draft, libraryRoot });
    const pack = demoSnapshot.packs.find((candidate) => candidate.id === draft.packId);
    const blocked = !pack || pack.files.some((file) => file.path === draft.relativePath);
    const path = `${demoRoot}/packs/${draft.packId}/${draft.relativePath}`;
    return {
      operation: "addPackFile",
      blocked,
      changeCount: blocked ? 0 : 2,
      summary: blocked
        ? `\`${draft.relativePath}\` already exists or the Rule Pack is unavailable.`
        : `Add \`${draft.relativePath}\` after the existing sources in Rule Pack \`${draft.packId}\`.`,
      steps: blocked
        ? []
        : [
            step(`${demoRoot}/packs/${draft.packId}/pack.json`, "updatePack", "Append the ordered path."),
            step(path, "createRuleFile", "Create the Markdown source."),
          ],
    };
  },
  async addPackFile(draft: PackFileDraft, libraryRoot?: string): Promise<LibraryMutationOutcome> {
    if (isTauri) return invoke("add_pack_file", { ...draft, libraryRoot });
    demoSnapshot = {
      ...demoSnapshot,
      runtimeState: demoSnapshot.profiles
        .find((profile) => profile.isActive)
        ?.packIds.includes(draft.packId)
        ? "stale"
        : demoSnapshot.runtimeState,
      packs: demoSnapshot.packs.map((pack) =>
        pack.id === draft.packId
          ? {
              ...pack,
              files: [...pack.files, { path: draft.relativePath, digest: "newfile00000" }],
            }
          : pack,
      ),
    };
    return mutation([
      `${demoRoot}/packs/${draft.packId}/pack.json`,
      `${demoRoot}/packs/${draft.packId}/${draft.relativePath}`,
    ]);
  },
  async previewCreateProfile(draft: ProfileDraft, libraryRoot?: string): Promise<LibraryPlan> {
    if (isTauri) {
      return invoke("preview_create_profile", {
        id: draft.id,
        name: draft.name,
        description: draft.description,
        packIds: draft.packIds,
        libraryRoot,
      });
    }
    const path = `${demoRoot}/profiles/${draft.id}.json`;
    const blocked = demoSnapshot.profiles.some((profile) => profile.id === draft.id);
    return {
      operation: "createProfile",
      blocked,
      changeCount: blocked ? 0 : 1,
      summary: blocked
        ? `Profile \`${draft.id}\` already exists; no file will be overwritten.`
        : `Create Profile \`${draft.name}\` from ${draft.packIds.length} ordered Rule Pack(s).`,
      steps: blocked ? [] : [step(path, "createProfile", "Write the ordered Rule Pack ids.")],
    };
  },
  async createProfile(draft: ProfileDraft, libraryRoot?: string): Promise<LibraryMutationOutcome> {
    if (isTauri) {
      return invoke("create_profile", {
        id: draft.id,
        name: draft.name,
        description: draft.description,
        packIds: draft.packIds,
        libraryRoot,
      });
    }
    demoSnapshot.profiles = [
      ...demoSnapshot.profiles,
      { ...draft, isActive: false },
    ];
    return mutation([`${demoRoot}/profiles/${draft.id}.json`]);
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
    const firstPack = demoSnapshot.packs.find((pack) => pack.id === profile.packIds[0]);
    demoSnapshot = {
      ...demoSnapshot,
      activeProfileId: profileId,
      activeSourcePath: firstPack ? `${demoRoot}/packs/${firstPack.id}/AGENTS.md` : undefined,
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
    packId: string,
    relativePath: string,
    libraryRoot?: string,
  ): Promise<boolean> {
    if (isTauri) {
      await invoke("open_rule_source", { targetId, packId, relativePath, libraryRoot });
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
    demoSnapshot.latestLibraryBackup = undefined;
    return { restored: [`${demoRoot}/current`], backupId: "demo-library-snapshot" };
  },
};
