use crate::{AgentAdapter, ArmError, DeployMode};
use serde::Deserialize;
use std::env;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Definition {
    id: String,
    label: String,
    scope: String,
    path: String,
    commands: Vec<String>,
    markers: Vec<String>,
    apps: Vec<String>,
    aliases: Vec<String>,
    note: String,
    docs_url: String,
    max_chars: Option<usize>,
}

fn definitions() -> Vec<Definition> {
    serde_json::from_str(include_str!("../../../src/lib/agentCatalog.json"))
        .expect("bundled agent catalog must be valid")
}

fn env_root(read_env: &dyn Fn(&str) -> Option<PathBuf>, name: &str, fallback: PathBuf) -> PathBuf {
    read_env(name)
        .filter(|path| path.is_absolute())
        .unwrap_or(fallback)
}

fn adapters(
    home: &Path,
    project: Option<&Path>,
    read_env: &dyn Fn(&str) -> Option<PathBuf>,
) -> Vec<AgentAdapter> {
    let mut result: Vec<AgentAdapter> = Vec::new();
    for definition in definitions()
        .into_iter()
        .filter(|d| (d.scope == "project") == project.is_some())
    {
        let mut target = project.unwrap_or(home).join(&definition.path);
        if project.is_none() {
            target = match definition.id.as_str() {
                "opencode" => env_root(
                    read_env,
                    "OPENCODE_CONFIG_DIR",
                    env_root(read_env, "XDG_CONFIG_HOME", home.join(".config")).join("opencode"),
                )
                .join("AGENTS.md"),
                "qwen" => env_root(read_env, "QWEN_HOME", home.join(".qwen")).join("QWEN.md"),
                "codex" => env_root(read_env, "CODEX_HOME", home.join(".codex")).join("AGENTS.md"),
                "claude" => {
                    env_root(read_env, "CLAUDE_CONFIG_DIR", home.join(".claude")).join("CLAUDE.md")
                }
                "copilot" => env_root(read_env, "COPILOT_HOME", home.join(".copilot"))
                    .join("copilot-instructions.md"),
                "gemini" => env_root(read_env, "GEMINI_CLI_HOME", home.to_path_buf())
                    .join(".gemini/GEMINI.md"),
                "cline" if cfg!(target_os = "linux") && !home.join("Documents").is_dir() => {
                    home.join("Cline/Rules/agent-rules.md")
                }
                _ => target,
            };
        }
        let mut detection_paths = definition
            .markers
            .iter()
            .map(|path| home.join(path))
            .collect::<Vec<_>>();
        if project.is_none() {
            detection_paths.push(target.parent().unwrap().to_path_buf());
        }
        for command in &definition.commands {
            for directory in [
                ".local/bin",
                ".bun/bin",
                ".volta/bin",
                ".npm-global/bin",
                ".yarn/bin",
                "node_modules/.bin",
            ] {
                detection_paths.push(home.join(directory).join(command));
            }
            for directory in ["/opt/homebrew/bin", "/usr/local/bin"] {
                detection_paths.push(Path::new(directory).join(command));
            }
        }
        for app in &definition.apps {
            detection_paths.push(PathBuf::from(format!("/Applications/{app}.app")));
            detection_paths.push(home.join(format!("Applications/{app}.app")));
        }
        // An extension is detectable even before it has created its rules directory.
        let extension_prefix = match definition.id.as_str() {
            "cline" => Some("saoudrizwan.claude-dev-"),
            "roo" => Some("rooveterinaryinc.roo-cline-"),
            "kilo" => Some("kilocode.kilo-code-"),
            "continue" => Some("continue.continue-"),
            "copilot-ide" => Some("github.copilot-"),
            _ => None,
        };
        if let Some(prefix) = extension_prefix {
            for directory in [
                ".vscode/extensions",
                ".vscode-insiders/extensions",
                ".cursor/extensions",
                ".windsurf/extensions",
            ] {
                if let Ok(entries) = std::fs::read_dir(home.join(directory)) {
                    detection_paths.extend(
                        entries
                            .flatten()
                            .filter(|entry| {
                                entry
                                    .file_name()
                                    .to_string_lossy()
                                    .to_lowercase()
                                    .starts_with(prefix)
                            })
                            .map(|entry| entry.path()),
                    );
                }
            }
        }
        if let Some(shared) = result
            .iter_mut()
            .find(|adapter| adapter.target_path == target)
        {
            shared.label = format!("{} / {}", shared.label, definition.label);
            shared.aliases.push(definition.id);
            shared.aliases.extend(definition.aliases);
            shared.command_names.extend(definition.commands);
            shared.detection_paths.extend(detection_paths);
            shared.note = definition.note;
            shared.max_chars = definition.max_chars.or(shared.max_chars);
            continue;
        }
        result.push(AgentAdapter {
            id: definition.id,
            label: definition.label,
            target_path: target,
            mode: DeployMode::Symlink,
            note: definition.note,
            command_names: definition.commands,
            detection_paths,
            aliases: definition.aliases,
            scope: definition.scope,
            docs_url: definition.docs_url,
            max_chars: definition.max_chars,
        });
    }
    result
}

pub fn default_adapters(home: &Path) -> Vec<AgentAdapter> {
    adapters(home, None, &|name| env::var_os(name).map(PathBuf::from))
}

pub fn project_adapters(home: &Path, project: &Path) -> Result<Vec<AgentAdapter>, ArmError> {
    if !project.is_absolute()
        || !project.is_dir()
        || project
            .components()
            .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(ArmError::UnsupportedEntry(project.display().to_string()));
    }
    // Resolve the chosen root once; no working-directory fallback or directory creation.
    let project = project
        .canonicalize()
        .map_err(|e| ArmError::io(project.display().to_string(), e))?;
    Ok(adapters(home, Some(&project), &|_| None))
}

#[cfg(test)]
pub(crate) fn test_adapters(home: &Path) -> Vec<AgentAdapter> {
    adapters(home, None, &|_| None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn catalog_has_unique_ids_and_safe_relative_targets() {
        let definitions = definitions();
        let mut ids = BTreeSet::new();
        for definition in definitions {
            assert!(ids.insert(definition.id));
            assert!(matches!(definition.scope.as_str(), "global" | "project"));
            assert!(Path::new(&definition.path)
                .components()
                .all(|c| matches!(c, Component::Normal(_))));
        }
    }

    #[test]
    fn shared_files_are_one_target_with_aliases() {
        let home = tempfile::tempdir().unwrap();
        let adapters = adapters(home.path(), None, &|_| None);
        let targets = adapters
            .iter()
            .map(|a| &a.target_path)
            .collect::<BTreeSet<_>>();
        assert_eq!(targets.len(), adapters.len());
        let ag = adapters
            .iter()
            .find(|a| a.id == "antigravity" || a.aliases.iter().any(|alias| alias == "antigravity"))
            .unwrap();
        assert_eq!(ag.target_path, home.path().join(".gemini/GEMINI.md"));
        assert!(ag.aliases.contains(&"ag".to_string()));
        assert_eq!(ag.max_chars, Some(12000));
    }

    #[test]
    fn project_targets_require_an_explicit_existing_absolute_directory() {
        let home = tempfile::tempdir().unwrap();
        assert!(project_adapters(home.path(), Path::new("relative")).is_err());
        assert!(project_adapters(home.path(), &home.path().join("missing")).is_err());
        let adapters = project_adapters(home.path(), home.path()).unwrap();
        let cursor = adapters.iter().find(|a| a.id == "cursor").unwrap();
        assert_eq!(
            cursor.target_path,
            home.path().canonicalize().unwrap().join("AGENTS.md")
        );
        assert!(adapters.iter().all(|a| a.scope == "project"));
    }
    #[test]
    fn environment_roots_are_resolved_without_changing_process_environment() {
        let home = Path::new("/home/test");
        let env = |name: &str| match name {
            "GEMINI_CLI_HOME" => Some(PathBuf::from("/isolated")),
            "COPILOT_HOME" => Some(PathBuf::from("/copilot")),
            "OPENCODE_CONFIG_DIR" => Some(PathBuf::from("/opencode")),
            "QWEN_HOME" => Some(PathBuf::from("relative-is-ignored")),
            _ => None,
        };
        let adapters = adapters(home, None, &env);
        let target = |id: &str| {
            adapters
                .iter()
                .find(|a| a.id == id)
                .unwrap()
                .target_path
                .clone()
        };
        assert_eq!(target("gemini"), Path::new("/isolated/.gemini/GEMINI.md"));
        assert_eq!(target("antigravity"), home.join(".gemini/GEMINI.md"));
        assert_eq!(
            target("copilot"),
            Path::new("/copilot/copilot-instructions.md")
        );
        assert_eq!(target("opencode"), Path::new("/opencode/AGENTS.md"));
        assert_eq!(target("qwen"), home.join(".qwen/QWEN.md"));
    }
}
