mod adapters;
mod manager;
mod model;

pub use adapters::default_adapters;
pub use manager::{default_library_root, default_state_root, RulesManager};
pub use model::{
    AgentAdapter, AgentStatus, ApplyOutcome, ArmError, DeployMode, PlanStep, ProjectionPlan,
    RollbackOutcome, TargetState, WorkspaceSnapshot,
};
