export type DeployMode = "include" | "symlink";
export type TargetState = "inSync" | "ready" | "drifted" | "conflict";
export type TargetKind =
  | "missing"
  | "connectedLink"
  | "legacyLink"
  | "independentFile"
  | "legacyInclude"
  | "foreignLink"
  | "invalidManagedFile";
export type LibraryState = "empty" | "legacy" | "upgrade" | "ready" | "conflict";
export type RuntimeState = "missing" | "current" | "stale" | "conflict";

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
  scope?: "global" | "project";
  note?: string;
  docsUrl?: string;
  maxChars?: number;
  warning?: string;
  detail: string;
}

export interface ConnectionChange {
  agentId: string;
  connected: boolean;
}

export interface RuleFileSummary {
  path: string;
  digest: string;
  modifiedAt?: string;
}

export interface ProfileSummary {
  id: string;
  name: string;
  description: string;
  files: RuleFileSummary[];
  isActive: boolean;
}

export interface WorkspaceSnapshot {
  libraryRoot: string;
  libraryState: LibraryState;
  libraryDetail: string;
  runtimeState: RuntimeState;
  sourcePath: string;
  sourceExists: boolean;
  sourceDigest?: string;
  sourceModifiedAt?: string;
  activeProfileId?: string;
  activeSourcePath?: string;
  legacySourcePath?: string;
  profiles: ProfileSummary[];
  latestBackup?: string;
  latestLibraryBackup?: string;
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
  requiresConfirmation: boolean;
}

export interface ProjectionPlan {
  sourcePath: string;
  blocked: boolean;
  changeCount: number;
  confirmationCount: number;
  steps: PlanStep[];
}

export interface LibraryPlanStep {
  path: string;
  action: string;
  summary: string;
}

export interface LibraryPlan {
  operation: string;
  blocked: boolean;
  changeCount: number;
  summary: string;
  steps: LibraryPlanStep[];
}

export interface ApplyOutcome {
  changed: string[];
  backupId?: string;
  preservedBackupDir?: string;
}

export interface LibraryMutationOutcome {
  changed: string[];
  backupId?: string;
}

export interface RollbackOutcome {
  restored: string[];
  backupId: string;
}

export interface ProfileDraft {
  id: string;
  name: string;
  description: string;
}
