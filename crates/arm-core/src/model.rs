use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DeployMode {
    Include,
    Symlink,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentAdapter {
    pub id: String,
    pub label: String,
    pub target_path: PathBuf,
    pub mode: DeployMode,
    pub note: String,
    pub command_names: Vec<String>,
    pub detection_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TargetState {
    InSync,
    Ready,
    Drifted,
    Conflict,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TargetKind {
    Missing,
    ConnectedLink,
    IndependentFile,
    LegacyInclude,
    ForeignLink,
    InvalidManagedFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatus {
    pub id: String,
    pub label: String,
    pub target_path: String,
    pub mode: DeployMode,
    pub state: TargetState,
    pub target_kind: TargetKind,
    pub installed: bool,
    pub connected: bool,
    pub detection_detail: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionChange {
    pub agent_id: String,
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSnapshot {
    pub library_root: String,
    pub source_path: String,
    pub source_exists: bool,
    pub source_digest: Option<String>,
    pub source_modified_at: Option<String>,
    pub latest_backup: Option<String>,
    pub agents: Vec<AgentStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlanStep {
    pub agent_id: String,
    pub agent_label: String,
    pub target_path: String,
    pub state: TargetState,
    pub desired_connected: bool,
    pub action: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionPlan {
    pub source_path: String,
    pub blocked: bool,
    pub change_count: usize,
    pub steps: Vec<PlanStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApplyOutcome {
    pub changed: Vec<String>,
    pub backup_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RollbackOutcome {
    pub restored: Vec<String>,
    pub backup_id: String,
}

#[derive(Debug, Error)]
pub enum ArmError {
    #[error("cannot access {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid state data: {0}")]
    Json(#[from] serde_json::Error),
    #[error("canonical rules file does not exist: {0}")]
    MissingSource(String),
    #[error("apply is blocked by an unmanaged target: {0}")]
    Blocked(String),
    #[error("apply stopped because {0} changed after the plan was prepared")]
    ApplyDrift(String),
    #[error("unknown agent adapter: {0}")]
    UnknownAgent(String),
    #[error("no rollback snapshot is available")]
    NoBackup,
    #[error("rollback stopped because {0} changed after apply")]
    RollbackDrift(String),
    #[error("unsupported filesystem entry at {0}")]
    UnsupportedEntry(String),
}

impl ArmError {
    pub(crate) fn io(path: impl Into<String>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
