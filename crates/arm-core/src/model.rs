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
    pub aliases: Vec<String>,
    pub scope: String,
    pub docs_url: String,
    pub max_chars: Option<usize>,
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
    LegacyLink,
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
    pub warning: Option<String>,
    pub scope: String,
    pub note: String,
    pub docs_url: String,
    pub max_chars: Option<usize>,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LibraryState {
    Empty,
    Legacy,
    Upgrade,
    Ready,
    Conflict,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeState {
    Missing,
    Current,
    Stale,
    Conflict,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuleFileSummary {
    pub path: String,
    pub digest: String,
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub files: Vec<RuleFileSummary>,
    pub is_active: bool,
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
    pub library_state: LibraryState,
    pub library_detail: String,
    pub runtime_state: RuntimeState,
    /// Stable machine-local projection entrypoint: `<library>/current/AGENTS.md`.
    pub source_path: String,
    pub source_exists: bool,
    pub source_digest: Option<String>,
    pub source_modified_at: Option<String>,
    pub active_profile_id: Option<String>,
    pub active_source_path: Option<String>,
    pub legacy_source_path: Option<String>,
    pub profiles: Vec<ProfileSummary>,
    pub latest_backup: Option<String>,
    pub latest_library_backup: Option<String>,
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
    pub requires_confirmation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionPlan {
    pub source_path: String,
    pub blocked: bool,
    pub change_count: usize,
    pub confirmation_count: usize,
    pub steps: Vec<PlanStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPlanStep {
    pub path: String,
    pub action: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPlan {
    pub operation: String,
    pub blocked: bool,
    pub change_count: usize,
    pub summary: String,
    pub steps: Vec<LibraryPlanStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApplyOutcome {
    pub changed: Vec<String>,
    pub backup_id: Option<String>,
    pub preserved_backup_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryMutationOutcome {
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
    #[error("canonical rules entrypoint does not exist: {0}")]
    MissingSource(String),
    #[error("apply is blocked by an unmanaged target: {0}")]
    Blocked(String),
    #[error(
        "confirmation is required before existing regular files are backed up and replaced: {0}"
    )]
    ConfirmationRequired(String),
    #[error("apply stopped because {0} changed after the plan was prepared")]
    ApplyDrift(String),
    #[error("unknown agent adapter: {0}")]
    UnknownAgent(String),
    #[error("unknown profile: {0}")]
    UnknownProfile(String),
    #[error("invalid rule library: {0}")]
    InvalidLibrary(String),
    #[error("invalid identifier `{0}`; use lowercase letters, digits, `_`, or `-`")]
    InvalidId(String),
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
