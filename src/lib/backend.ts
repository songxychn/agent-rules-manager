import { invoke } from "@tauri-apps/api/core";
import type {
  ApplyOutcome,
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
      mode: "include",
      state: "inSync",
      detail: "Managed include block is current.",
    },
    {
      id: "codex",
      label: "Codex",
      targetPath: "/Users/baizhukui/.codex/AGENTS.md",
      mode: "symlink",
      state: "inSync",
      detail: "Linked directly to the canonical rules.",
    },
    {
      id: "grok",
      label: "Grok",
      targetPath: "/Users/baizhukui/.grok/AGENTS.md",
      mode: "symlink",
      state: "inSync",
      detail: "Linked directly to the canonical rules.",
    },
    {
      id: "opencode",
      label: "OpenCode",
      targetPath: "/Users/baizhukui/.config/opencode/AGENTS.md",
      mode: "symlink",
      state: "ready",
      detail: "No target file yet.",
    },
  ],
};

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
  async preview(agents: string[], libraryRoot?: string): Promise<ProjectionPlan> {
    if (isTauri) {
      return invoke("preview_apply", { agents, libraryRoot });
    }
    return buildProjectionPlan(demoSnapshot, agents);
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
  async apply(agents: string[], libraryRoot?: string): Promise<ApplyOutcome> {
    if (isTauri) {
      return invoke("apply_rules", { agents, libraryRoot });
    }
    const selected = new Set(agents);
    const changed: string[] = [];
    demoSnapshot = {
      ...demoSnapshot,
      latestBackup: "demo-apply-snapshot",
      agents: demoSnapshot.agents.map((agent) => {
        if (selected.has(agent.id) && agent.state !== "inSync") {
          changed.push(agent.id);
          return { ...agent, state: "inSync", detail: "Projection is current." };
        }
        return agent;
      }),
    };
    return { changed, backupId: changed.length ? "demo-apply-snapshot" : undefined };
  },
  async rollback(libraryRoot?: string): Promise<RollbackOutcome> {
    if (isTauri) {
      return invoke("rollback_latest", { libraryRoot });
    }
    demoSnapshot = {
      ...demoSnapshot,
      latestBackup: undefined,
      agents: demoSnapshot.agents.map((agent) =>
        agent.id === "opencode"
          ? { ...agent, state: "ready", detail: "No target file yet." }
          : agent,
      ),
    };
    return { restored: [demoSnapshot.agents[3].targetPath], backupId: "demo-apply-snapshot" };
  },
};
