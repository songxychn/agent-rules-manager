use crate::{AgentAdapter, DeployMode};
use std::env;
use std::path::{Path, PathBuf};

fn detection_paths(home: &Path, command: &str, config_root: PathBuf) -> Vec<PathBuf> {
    vec![
        config_root,
        home.join(format!(".local/bin/{command}")),
        home.join(format!(".bun/bin/{command}")),
        home.join(format!(".volta/bin/{command}")),
        home.join(format!(".npm-global/bin/{command}")),
        home.join(format!(".yarn/bin/{command}")),
        home.join(format!("node_modules/.bin/{command}")),
        PathBuf::from(format!("/opt/homebrew/bin/{command}")),
        PathBuf::from(format!("/usr/local/bin/{command}")),
    ]
}

pub fn default_adapters(home: &Path) -> Vec<AgentAdapter> {
    let opencode_root = env::var_os("OPENCODE_CONFIG_DIR")
        .map(Into::into)
        .or_else(|| {
            env::var_os("XDG_CONFIG_HOME")
                .map(std::path::PathBuf::from)
                .map(|root| root.join("opencode"))
        })
        .unwrap_or_else(|| home.join(".config/opencode"));
    let qwen_root = env::var_os("QWEN_HOME")
        .map(Into::into)
        .unwrap_or_else(|| home.join(".qwen"));

    vec![
        AgentAdapter {
            id: "claude".into(),
            label: "Claude Code".into(),
            target_path: home.join(".claude/CLAUDE.md"),
            mode: DeployMode::Symlink,
            note: "Stable link to the machine-local current Profile.".into(),
            command_names: vec!["claude".into()],
            detection_paths: detection_paths(home, "claude", home.join(".claude")),
        },
        AgentAdapter {
            id: "codex".into(),
            label: "Codex".into(),
            target_path: home.join(".codex/AGENTS.md"),
            mode: DeployMode::Symlink,
            note: "Stable link to the machine-local current Profile.".into(),
            command_names: vec!["codex".into()],
            detection_paths: {
                let mut paths = detection_paths(home, "codex", home.join(".codex"));
                paths.push(PathBuf::from("/Applications/ChatGPT.app"));
                paths
            },
        },
        AgentAdapter {
            id: "grok".into(),
            label: "Grok".into(),
            target_path: home.join(".grok/AGENTS.md"),
            mode: DeployMode::Symlink,
            note: "Stable link to the machine-local current Profile.".into(),
            command_names: vec!["grok".into()],
            detection_paths: detection_paths(home, "grok", home.join(".grok")),
        },
        AgentAdapter {
            id: "opencode".into(),
            label: "OpenCode".into(),
            target_path: opencode_root.join("AGENTS.md"),
            mode: DeployMode::Symlink,
            note: "Stable link to the machine-local current Profile.".into(),
            command_names: vec!["opencode".into()],
            detection_paths: {
                let mut paths = detection_paths(home, "opencode", opencode_root);
                paths.push(home.join(".opencode/bin/opencode"));
                paths.push(PathBuf::from("/Applications/OpenCode.app"));
                paths
            },
        },
        AgentAdapter {
            id: "qwen".into(),
            label: "Qwen Code".into(),
            target_path: qwen_root.join("QWEN.md"),
            mode: DeployMode::Symlink,
            note: "Stable link to the machine-local current Profile.".into(),
            command_names: vec!["qwen".into()],
            detection_paths: detection_paths(home, "qwen", qwen_root),
        },
    ]
}
