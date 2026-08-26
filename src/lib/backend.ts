import { invoke } from "@tauri-apps/api/core";
import type {
  ApplyOutcome,
  ConnectionChange,
  OpenTarget,
  ProjectionPlan,
  RollbackOutcome,
  WorkspaceSnapshot,
} from "./types";
import { buildProjectionPlan } from "./plan";

const isTauri = "__TAURI_INTERNALS__" in window;

let demoSnapshot: WorkspaceSnapshot = {
  libraryRoot: "/Users/baizhukui/.agent-rules",
  sourcePath: "/Users/baizhukui/.agent-rules/AGENTS.md",
  sourceExists: true,
  sourceDigest: "e65a1c71b290",
  sourceModifiedAt: "2026-08-26T08:36:00Z",
  latestBackup: "20260822T164218.190Z",
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

export const backend = {
  isTauri,
  async snapshot(libraryRoot?: string): Promise<WorkspaceSnapshot> {
    if (isTauri) {
      return invoke("get_workspace_snapshot", { libraryRoot });
    }
    return structuredClone(demoSnapshot);
  },
  async initialize(libraryRoot?: string): Promise<string> {
    if (isTauri) {
      return invoke("initialize_library", { libraryRoot });
    }
    demoSnapshot.sourceExists = true;
    return demoSnapshot.sourcePath;
  },
  async preview(changes: ConnectionChange[], libraryRoot?: string): Promise<ProjectionPlan> {
    if (isTauri) {
      return invoke("preview_apply", { changes, libraryRoot });
    }
    return buildProjectionPlan(demoSnapshot, changes);
  },
  async openTargets(): Promise<OpenTarget[]> {
    if (isTauri) {
      return invoke("get_open_targets");
    }
    return structuredClone(demoOpenTargets);
  },
  async openSource(targetId: string, libraryRoot?: string): Promise<boolean> {
    if (isTauri) {
      await invoke("open_source", { targetId, libraryRoot });
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
};
