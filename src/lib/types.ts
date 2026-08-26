export type DeployMode = "include" | "symlink";
export type TargetState = "inSync" | "ready" | "drifted" | "conflict";
export type TargetKind =
  | "missing"
  | "connectedLink"
  | "independentFile"
  | "legacyInclude"
  | "foreignLink"
  | "invalidManagedFile";

export interface AgentStatus {
  id: string;
  label: string;
  targetPath: string;
  mode: DeployMode;
  state: TargetState;
  targetKind: TargetKind;
  installed: boolean;
  connected: boolean;
  detectionDetail: string;
  detail: string;
}

export interface ConnectionChange {
  agentId: string;
  connected: boolean;
}

export interface WorkspaceSnapshot {
  libraryRoot: string;
  sourcePath: string;
  sourceExists: boolean;
  sourceDigest?: string;
  sourceModifiedAt?: string;
  latestBackup?: string;
  agents: AgentStatus[];
}

export type OpenTargetKind = "default" | "editor" | "reveal" | "terminal";

export interface OpenTarget {
  id: string;
  label: string;
  kind: OpenTargetKind;
}

export interface PlanStep {
  agentId: string;
  agentLabel: string;
  targetPath: string;
  state: TargetState;
  desiredConnected: boolean;
  action: string;
  summary: string;
}

export interface ProjectionPlan {
  sourcePath: string;
  blocked: boolean;
  changeCount: number;
  steps: PlanStep[];
}

export interface ApplyOutcome {
  changed: string[];
  backupId?: string;
}

export interface RollbackOutcome {
  restored: string[];
  backupId: string;
}
