use crate::{AgentAdapter, DeployMode};
use std::path::Path;

pub fn default_adapters(home: &Path) -> Vec<AgentAdapter> {
    vec![
        AgentAdapter {
            id: "claude".into(),
            label: "Claude Code".into(),
            target_path: home.join(".claude/CLAUDE.md"),
            mode: DeployMode::Include,
            note: "Managed include block; existing Claude-only instructions stay in place.".into(),
        },
        AgentAdapter {
            id: "codex".into(),
            label: "Codex".into(),
            target_path: home.join(".codex/AGENTS.md"),
            mode: DeployMode::Symlink,
            note: "Direct link to the neutral rules source.".into(),
        },
        AgentAdapter {
            id: "grok".into(),
            label: "Grok".into(),
            target_path: home.join(".grok/AGENTS.md"),
            mode: DeployMode::Symlink,
            note: "Direct link to the neutral rules source.".into(),
        },
        AgentAdapter {
            id: "opencode".into(),
            label: "OpenCode".into(),
            target_path: home.join(".config/opencode/AGENTS.md"),
            mode: DeployMode::Symlink,
            note: "Direct link to the neutral rules source.".into(),
        },
    ]
}
