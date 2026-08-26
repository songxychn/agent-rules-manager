export type DeployMode = "include" | "symlink";
export type TargetState = "inSync" | "ready" | "drifted" | "conflict";

export interface AgentStatus {
  id: string;
  label: string;
  targetPath: string;
  mode: DeployMode;
  state: TargetState;
  detail: string;
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
