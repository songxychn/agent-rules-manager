mod adapters;
mod library;
mod manager;
mod model;

pub use adapters::default_adapters;
pub use manager::{default_library_root, default_state_root, RulesManager};
pub use model::{
    AgentAdapter, AgentStatus, ApplyOutcome, ArmError, ConnectionChange, DeployMode,
    LibraryMutationOutcome, LibraryPlan, LibraryPlanStep, LibraryState, PlanStep, ProfileSummary,
    ProjectionPlan, RollbackOutcome, RuleFileSummary, RuntimeState, TargetKind, TargetState,
    WorkspaceSnapshot,
};
