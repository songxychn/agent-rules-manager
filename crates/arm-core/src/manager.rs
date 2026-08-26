use crate::{
    default_adapters, AgentAdapter, AgentStatus, ApplyOutcome, ArmError, DeployMode, PlanStep,
    ProjectionPlan, RollbackOutcome, TargetState, WorkspaceSnapshot,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const SOURCE_FILE: &str = "AGENTS.md";
const INCLUDE_START: &str = "<!-- agent-rules-manager:start -->";
const INCLUDE_END: &str = "<!-- agent-rules-manager:end -->";

const STARTER_RULES: &str = r#"# Shared agent rules

These instructions apply to every supported coding agent unless a project-level rule is more specific.

## Working style

- Read the relevant code and instructions before editing.
- Keep changes scoped to the requested behavior.
- Verify changes in proportion to risk.
"#;

#[derive(Debug, Clone)]
pub struct RulesManager {
    library_root: PathBuf,
    state_root: PathBuf,
    adapters: Vec<AgentAdapter>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum FileState {
    Missing,
    File { content: String },
    Symlink { target: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupEntry {
    target_path: String,
    original: FileState,
    applied_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupSnapshot {
    id: String,
    created_at: String,
    entries: Vec<BackupEntry>,
}

#[derive(Debug, Clone)]
struct PreparedChange {
    agent_id: String,
    path: PathBuf,
    original: FileState,
    desired: FileState,
}

impl RulesManager {
    pub fn new(library_root: PathBuf, state_root: PathBuf, home: &Path) -> Self {
        Self {
            library_root,
            state_root,
            adapters: default_adapters(home),
        }
    }

    pub fn with_adapters(
        library_root: PathBuf,
        state_root: PathBuf,
        adapters: Vec<AgentAdapter>,
    ) -> Self {
        Self {
            library_root,
            state_root,
            adapters,
        }
    }

    pub fn source_path(&self) -> PathBuf {
        self.library_root.join(SOURCE_FILE)
    }

    pub fn initialize(&self) -> Result<PathBuf, ArmError> {
        let source = self.source_path();
        if source.exists() {
            return Ok(source);
        }
        fs::create_dir_all(&self.library_root)
            .map_err(|error| ArmError::io(self.library_root.display().to_string(), error))?;
        write_atomic(&source, STARTER_RULES)?;
        Ok(source)
    }

    pub fn snapshot(&self) -> Result<WorkspaceSnapshot, ArmError> {
        let source_path = self.source_path();
        let source_exists = source_path.exists();
        let (source_digest, source_modified_at) = if source_exists {
            let source_content = fs::read_to_string(&source_path)
                .map_err(|error| ArmError::io(source_path.display().to_string(), error))?;
            let modified_at = fs::metadata(&source_path)
                .and_then(|metadata| metadata.modified())
                .ok()
                .map(DateTime::<Utc>::from)
                .map(|timestamp| timestamp.to_rfc3339());
            (Some(short_digest(&source_content)), modified_at)
        } else {
            (None, None)
        };
        let agents = self
            .adapters
            .iter()
            .map(|adapter| self.inspect_adapter(adapter, &source_path))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(WorkspaceSnapshot {
            library_root: self.library_root.display().to_string(),
            source_path: source_path.display().to_string(),
            source_exists,
            source_digest,
            source_modified_at,
            latest_backup: self.latest_backup_path()?.and_then(|path| {
                path.file_stem()
                    .map(|value| value.to_string_lossy().to_string())
            }),
            agents,
        })
    }

    pub fn plan(&self, selected: &[String]) -> Result<ProjectionPlan, ArmError> {
        let source_path = self.source_path();
        if !source_path.exists() {
            return Err(ArmError::MissingSource(source_path.display().to_string()));
        }
        let selected = self.resolve_selection(selected)?;
        let mut blocked = false;
        let mut change_count = 0;
        let mut steps = Vec::new();

        for adapter in self
            .adapters
            .iter()
            .filter(|adapter| selected.contains(&adapter.id))
        {
            let status = self.inspect_adapter(adapter, &source_path)?;
            let (action, summary) = match status.state {
                TargetState::InSync => ("none", "Already projects the canonical rules."),
                TargetState::Ready => {
                    change_count += 1;
                    match adapter.mode {
                        DeployMode::Include => (
                            "addInclude",
                            "Append a managed include block without replacing existing content.",
                        ),
                        DeployMode::Symlink => (
                            "createLink",
                            "Create a symbolic link to the canonical rules file.",
                        ),
                    }
                }
                TargetState::Drifted => {
                    change_count += 1;
                    (
                        "refreshManagedBlock",
                        "Refresh the existing Agent Rules Manager block.",
                    )
                }
                TargetState::Conflict => {
                    blocked = true;
                    (
                        "blocked",
                        "An unmanaged file or link is present; import or move it before apply.",
                    )
                }
            };
            steps.push(PlanStep {
                agent_id: status.id,
                agent_label: status.label,
                target_path: status.target_path,
                state: status.state,
                action: action.into(),
                summary: summary.into(),
            });
        }

        Ok(ProjectionPlan {
            source_path: source_path.display().to_string(),
            blocked,
            change_count,
            steps,
        })
    }

    pub fn apply(&self, selected: &[String]) -> Result<ApplyOutcome, ArmError> {
        let source = self.source_path();
        let source = fs::canonicalize(&source)
            .map_err(|error| ArmError::io(source.display().to_string(), error))?;
        let plan = self.plan(selected)?;
        if plan.blocked {
            let targets = plan
                .steps
                .iter()
                .filter(|step| step.state == TargetState::Conflict)
                .map(|step| step.target_path.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(ArmError::Blocked(targets));
        }

        let changed_steps = plan
            .steps
            .iter()
            .filter(|step| matches!(step.state, TargetState::Ready | TargetState::Drifted))
            .collect::<Vec<_>>();
        if changed_steps.is_empty() {
            return Ok(ApplyOutcome {
                changed: Vec::new(),
                backup_id: None,
            });
        }

        let backup_id = Utc::now().format("%Y%m%dT%H%M%S%.9fZ").to_string();
        let mut prepared = Vec::new();
        for step in &changed_steps {
            let path = PathBuf::from(&step.target_path);
            let original = read_file_state(&path)?;
            let adapter = self
                .adapters
                .iter()
                .find(|adapter| adapter.id == step.agent_id)
                .ok_or_else(|| ArmError::UnknownAgent(step.agent_id.clone()))?;
            let desired = desired_adapter_state(adapter, &source, &original)?;
            prepared.push(PreparedChange {
                agent_id: step.agent_id.clone(),
                path,
                original,
                desired,
            });
        }
        let backup = BackupSnapshot {
            id: backup_id.clone(),
            created_at: Utc::now().to_rfc3339(),
            entries: prepared
                .iter()
                .map(|change| {
                    Ok(BackupEntry {
                        target_path: change.path.display().to_string(),
                        original: change.original.clone(),
                        applied_digest: file_state_digest(&change.desired)?,
                    })
                })
                .collect::<Result<Vec<_>, ArmError>>()?,
        };
        let backup_path = self.write_backup(&backup)?;

        let mut completed = Vec::new();
        for (index, change) in prepared.iter().enumerate() {
            let current = match read_file_state(&change.path) {
                Ok(current) => current,
                Err(error) => {
                    let _ = restore_prepared_changes(&prepared[..index]);
                    return Err(error);
                }
            };
            if current != change.original {
                let _ = restore_prepared_changes(&prepared[..index]);
                return Err(ArmError::ApplyDrift(change.path.display().to_string()));
            }
            if let Err(error) = write_file_state(&change.path, &change.desired) {
                if restore_prepared_changes(&prepared[..index]) {
                    let _ = self.archive_backup(&backup_path);
                }
                return Err(error);
            }
            match read_file_state(&change.path) {
                Ok(written) if written == change.desired => {}
                Ok(_) => {
                    let _ = restore_prepared_changes(&prepared[..=index]);
                    return Err(ArmError::ApplyDrift(change.path.display().to_string()));
                }
                Err(error) => {
                    let _ = restore_prepared_changes(&prepared[..=index]);
                    return Err(error);
                }
            }
            completed.push(change.agent_id.clone());
        }

        Ok(ApplyOutcome {
            changed: completed,
            backup_id: Some(backup_id),
        })
    }

    pub fn rollback_latest(&self) -> Result<RollbackOutcome, ArmError> {
        let backup_path = self.latest_backup_path()?.ok_or(ArmError::NoBackup)?;
        let data = fs::read_to_string(&backup_path)
            .map_err(|error| ArmError::io(backup_path.display().to_string(), error))?;
        let backup: BackupSnapshot = serde_json::from_str(&data)?;

        for entry in &backup.entries {
            let path = PathBuf::from(&entry.target_path);
            let current = read_file_state(&path)?;
            let current_digest = file_state_digest(&current)?;
            let original_digest = file_state_digest(&entry.original)?;
            if current_digest != entry.applied_digest && current_digest != original_digest {
                return Err(ArmError::RollbackDrift(entry.target_path.clone()));
            }
        }

        let mut restored = Vec::new();
        for entry in backup.entries.iter().rev() {
            let path = PathBuf::from(&entry.target_path);
            restore_file_state(&path, &entry.original)?;
            restored.push(entry.target_path.clone());
        }

        self.archive_backup(&backup_path)?;

        Ok(RollbackOutcome {
            restored,
            backup_id: backup.id,
        })
    }

    fn resolve_selection(&self, selected: &[String]) -> Result<BTreeSet<String>, ArmError> {
        let known = self
            .adapters
            .iter()
            .map(|adapter| adapter.id.clone())
            .collect::<BTreeSet<_>>();
        if selected.is_empty() {
            return Ok(known);
        }
        for id in selected {
            if !known.contains(id) {
                return Err(ArmError::UnknownAgent(id.clone()));
            }
        }
        Ok(selected.iter().cloned().collect())
    }

    fn inspect_adapter(
        &self,
        adapter: &AgentAdapter,
        source_path: &Path,
    ) -> Result<AgentStatus, ArmError> {
        let (state, detail) = match adapter.mode {
            DeployMode::Include => inspect_include_target(&adapter.target_path, source_path)?,
            DeployMode::Symlink => inspect_symlink_target(&adapter.target_path, source_path)?,
        };
        Ok(AgentStatus {
            id: adapter.id.clone(),
            label: adapter.label.clone(),
            target_path: adapter.target_path.display().to_string(),
            mode: adapter.mode,
            state,
            detail,
        })
    }

    fn write_backup(&self, backup: &BackupSnapshot) -> Result<PathBuf, ArmError> {
        let backup_dir = self.state_root.join("backups");
        fs::create_dir_all(&backup_dir)
            .map_err(|error| ArmError::io(backup_dir.display().to_string(), error))?;
        let path = backup_dir.join(format!("{}.json", backup.id));
        let data = serde_json::to_string_pretty(backup)?;
        write_atomic(&path, &data)?;
        Ok(path)
    }

    fn archive_backup(&self, backup_path: &Path) -> Result<(), ArmError> {
        let restored_dir = self.state_root.join("backups/restored");
        fs::create_dir_all(&restored_dir)
            .map_err(|error| ArmError::io(restored_dir.display().to_string(), error))?;
        let restored_path = restored_dir.join(
            backup_path
                .file_name()
                .ok_or_else(|| ArmError::UnsupportedEntry(backup_path.display().to_string()))?,
        );
        fs::rename(backup_path, restored_path)
            .map_err(|error| ArmError::io(backup_path.display().to_string(), error))
    }

    fn latest_backup_path(&self) -> Result<Option<PathBuf>, ArmError> {
        let backup_dir = self.state_root.join("backups");
        if !backup_dir.exists() {
            return Ok(None);
        }
        let mut backups = fs::read_dir(&backup_dir)
            .map_err(|error| ArmError::io(backup_dir.display().to_string(), error))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "json")
            })
            .collect::<Vec<_>>();
        backups.sort();
        Ok(backups.pop())
    }
}

pub fn default_library_root() -> Result<PathBuf, ArmError> {
    let home = home_dir()?;
    Ok(library_root_for(
        &home,
        env::var_os("AGENT_RULES_HOME").map(PathBuf::from),
    ))
}

fn library_root_for(home: &Path, configured: Option<PathBuf>) -> PathBuf {
    configured.unwrap_or_else(|| home.join(".agent-rules"))
}

pub fn default_state_root() -> Result<PathBuf, ArmError> {
    if let Some(root) = env::var_os("AGENT_RULES_STATE_HOME") {
        return Ok(PathBuf::from(root));
    }
    let home = home_dir()?;
    #[cfg(target_os = "macos")]
    {
        Ok(home.join("Library/Application Support/agent-rules-manager"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/state"))
            .join("agent-rules-manager"))
    }
}

fn home_dir() -> Result<PathBuf, ArmError> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| ArmError::MissingSource("HOME is not set".into()))
}

fn inspect_symlink_target(target: &Path, source: &Path) -> Result<(TargetState, String), ArmError> {
    let metadata = match fs::symlink_metadata(target) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((TargetState::Ready, "No target file yet.".into()))
        }
        Err(error) => return Err(ArmError::io(target.display().to_string(), error)),
    };
    if !metadata.file_type().is_symlink() {
        return Ok((
            TargetState::Conflict,
            "A regular, unmanaged entry already exists.".into(),
        ));
    }
    let linked =
        fs::read_link(target).map_err(|error| ArmError::io(target.display().to_string(), error))?;
    let linked = if linked.is_absolute() {
        linked
    } else {
        target
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(linked)
    };
    let source_canonical = fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
    let linked_canonical = fs::canonicalize(&linked).unwrap_or(linked);
    if linked_canonical == source_canonical {
        Ok((
            TargetState::InSync,
            "Linked directly to the canonical rules.".into(),
        ))
    } else {
        Ok((
            TargetState::Conflict,
            "The existing link points somewhere else.".into(),
        ))
    }
}

fn inspect_include_target(target: &Path, source: &Path) -> Result<(TargetState, String), ArmError> {
    let state = read_file_state(target)?;
    match state {
        FileState::Missing => Ok((
            TargetState::Ready,
            "A new Claude instruction file will be created.".into(),
        )),
        FileState::Symlink { target: linked } => {
            let linked = PathBuf::from(linked);
            let linked = if linked.is_absolute() {
                linked
            } else {
                target
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(linked)
            };
            let source = fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
            let linked = fs::canonicalize(&linked).unwrap_or(linked);
            if linked == source {
                Ok((
                    TargetState::InSync,
                    "Claude links directly to the canonical rules.".into(),
                ))
            } else {
                Ok((
                    TargetState::Conflict,
                    "Claude uses an unmanaged symbolic link.".into(),
                ))
            }
        }
        FileState::File { content } => {
            let start = content.find(INCLUDE_START);
            let end = content.find(INCLUDE_END);
            match (start, end) {
                (None, None) => Ok((
                    TargetState::Ready,
                    "Existing Claude instructions will be preserved.".into(),
                )),
                (Some(start), Some(end)) if start < end => {
                    let end = end + INCLUDE_END.len();
                    let current = &content[start..end];
                    let desired = include_block(source);
                    if current == desired {
                        Ok((
                            TargetState::InSync,
                            "Managed include block is current.".into(),
                        ))
                    } else {
                        Ok((
                            TargetState::Drifted,
                            "Managed include block points to a different source.".into(),
                        ))
                    }
                }
                _ => Ok((
                    TargetState::Conflict,
                    "Managed block markers are incomplete or out of order.".into(),
                )),
            }
        }
    }
}

fn write_file_state(path: &Path, state: &FileState) -> Result<(), ArmError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| ArmError::io(parent.display().to_string(), error))?;
    }
    match state {
        FileState::File { content } => write_atomic(path, content),
        FileState::Symlink { target } => create_symlink(Path::new(target), path),
        FileState::Missing => Err(ArmError::UnsupportedEntry(path.display().to_string())),
    }
}

fn restore_prepared_changes(changes: &[PreparedChange]) -> bool {
    let mut restored = true;
    for change in changes.iter().rev() {
        match read_file_state(&change.path) {
            Ok(current) if current == change.desired => {
                if restore_file_state(&change.path, &change.original).is_err() {
                    restored = false;
                }
            }
            Ok(current) if current == change.original => {}
            _ => restored = false,
        }
    }
    restored
}

fn desired_adapter_state(
    adapter: &AgentAdapter,
    source: &Path,
    current: &FileState,
) -> Result<FileState, ArmError> {
    match adapter.mode {
        DeployMode::Include => desired_include_state(&adapter.target_path, source, current),
        DeployMode::Symlink => match current {
            FileState::Missing => Ok(FileState::Symlink {
                target: source.display().to_string(),
            }),
            _ => Err(ArmError::Blocked(adapter.target_path.display().to_string())),
        },
    }
}

fn desired_include_state(
    target: &Path,
    source: &Path,
    current: &FileState,
) -> Result<FileState, ArmError> {
    let desired = include_block(source);
    let content = match current {
        FileState::Missing => format!("{desired}\n"),
        FileState::File { content } => {
            let start = content.find(INCLUDE_START);
            let end = content.find(INCLUDE_END);
            match (start, end) {
                (None, None) => {
                    let separator = if content.is_empty() || content.ends_with('\n') {
                        ""
                    } else {
                        "\n"
                    };
                    format!("{content}{separator}\n{desired}\n")
                }
                (Some(start), Some(end)) if start < end => {
                    let end = end + INCLUDE_END.len();
                    format!("{}{}{}", &content[..start], desired, &content[end..])
                }
                _ => {
                    return Err(ArmError::Blocked(target.display().to_string()));
                }
            }
        }
        FileState::Symlink { .. } => {
            return Err(ArmError::Blocked(target.display().to_string()));
        }
    };
    Ok(FileState::File { content })
}

fn include_block(source: &Path) -> String {
    format!("{INCLUDE_START}\n@{}\n{INCLUDE_END}", source.display())
}

fn read_file_state(path: &Path) -> Result<FileState, ArmError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(FileState::Missing)
        }
        Err(error) => return Err(ArmError::io(path.display().to_string(), error)),
    };
    if metadata.file_type().is_symlink() {
        let target =
            fs::read_link(path).map_err(|error| ArmError::io(path.display().to_string(), error))?;
        return Ok(FileState::Symlink {
            target: target.display().to_string(),
        });
    }
    if metadata.is_file() {
        let content = fs::read_to_string(path)
            .map_err(|error| ArmError::io(path.display().to_string(), error))?;
        return Ok(FileState::File { content });
    }
    Err(ArmError::UnsupportedEntry(path.display().to_string()))
}

fn restore_file_state(path: &Path, state: &FileState) -> Result<(), ArmError> {
    if fs::symlink_metadata(path).is_ok() {
        fs::remove_file(path).map_err(|error| ArmError::io(path.display().to_string(), error))?;
    }
    match state {
        FileState::Missing => Ok(()),
        FileState::File { content } => write_atomic(path, content),
        FileState::Symlink { target } => create_symlink(Path::new(target), path),
    }
}

fn write_atomic(path: &Path, content: &str) -> Result<(), ArmError> {
    let parent = path
        .parent()
        .ok_or_else(|| ArmError::UnsupportedEntry(path.display().to_string()))?;
    fs::create_dir_all(parent)
        .map_err(|error| ArmError::io(parent.display().to_string(), error))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| ArmError::UnsupportedEntry(path.display().to_string()))?
        .to_string_lossy();
    let temporary = parent.join(format!(
        ".{file_name}.arm-{}-{}",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    fs::write(&temporary, content)
        .map_err(|error| ArmError::io(temporary.display().to_string(), error))?;
    if let Ok(metadata) = fs::metadata(path) {
        fs::set_permissions(&temporary, metadata.permissions())
            .map_err(|error| ArmError::io(temporary.display().to_string(), error))?;
    }
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(ArmError::io(path.display().to_string(), error));
    }
    Ok(())
}

#[cfg(unix)]
fn create_symlink(source: &Path, target: &Path) -> Result<(), ArmError> {
    std::os::unix::fs::symlink(source, target)
        .map_err(|error| ArmError::io(target.display().to_string(), error))
}

#[cfg(windows)]
fn create_symlink(source: &Path, target: &Path) -> Result<(), ArmError> {
    std::os::windows::fs::symlink_file(source, target)
        .or_else(|_| fs::copy(source, target).map(|_| ()))
        .map_err(|error| ArmError::io(target.display().to_string(), error))
}

fn short_digest(content: &str) -> String {
    let digest = Sha256::digest(content.as_bytes());
    format!("{digest:x}")[..12].to_string()
}

fn file_state_digest(state: &FileState) -> Result<String, ArmError> {
    let bytes = serde_json::to_vec(state)?;
    let digest = Sha256::digest(bytes);
    Ok(format!("{digest:x}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fixture() -> (TempDir, RulesManager, PathBuf) {
        let root = TempDir::new().expect("temp root");
        let home = root.path().join("home");
        let library = home.join(".agent-rules");
        let state = home.join("state");
        fs::create_dir_all(&home).expect("home");
        let manager = RulesManager::new(library, state, &home);
        (root, manager, home)
    }

    #[test]
    fn library_root_is_fixed_under_home_unless_explicitly_configured() {
        let home = Path::new("/home/example");

        assert_eq!(library_root_for(home, None), home.join(".agent-rules"));
        assert_eq!(
            library_root_for(home, Some(PathBuf::from("/custom/rules"))),
            PathBuf::from("/custom/rules")
        );
    }

    #[test]
    fn initialize_plan_apply_and_rollback_are_lossless() {
        let (_root, manager, home) = fixture();
        manager.initialize().expect("initialize");
        let claude = home.join(".claude/CLAUDE.md");
        fs::create_dir_all(claude.parent().unwrap()).expect("claude dir");
        fs::write(&claude, "@RTK.md\n").expect("claude file");

        let plan = manager.plan(&[]).expect("plan");
        assert!(!plan.blocked);
        assert_eq!(plan.change_count, 4);

        let applied = manager.apply(&[]).expect("apply");
        assert_eq!(applied.changed.len(), 4);
        assert!(fs::read_to_string(&claude).unwrap().contains("@RTK.md"));
        assert!(fs::read_to_string(&claude).unwrap().contains(INCLUDE_START));
        assert!(home.join(".codex/AGENTS.md").is_symlink());

        let rollback = manager.rollback_latest().expect("rollback");
        assert_eq!(rollback.restored.len(), 4);
        assert_eq!(fs::read_to_string(&claude).unwrap(), "@RTK.md\n");
        assert!(!home.join(".codex/AGENTS.md").exists());
    }

    #[test]
    fn unmanaged_regular_file_blocks_the_whole_apply() {
        let (_root, manager, home) = fixture();
        manager.initialize().expect("initialize");
        let codex = home.join(".codex/AGENTS.md");
        fs::create_dir_all(codex.parent().unwrap()).expect("codex dir");
        fs::write(&codex, "keep me").expect("codex file");

        let plan = manager.plan(&[]).expect("plan");
        assert!(plan.blocked);
        assert!(matches!(manager.apply(&[]), Err(ArmError::Blocked(_))));
        assert_eq!(fs::read_to_string(codex).unwrap(), "keep me");
        assert!(!home.join(".grok/AGENTS.md").exists());
    }

    #[test]
    fn rollback_refuses_post_apply_drift() {
        let (_root, manager, home) = fixture();
        manager.initialize().expect("initialize");
        manager.apply(&["claude".into()]).expect("apply");
        let claude = home.join(".claude/CLAUDE.md");
        fs::write(&claude, "changed later").expect("drift");

        assert!(matches!(
            manager.rollback_latest(),
            Err(ArmError::RollbackDrift(_))
        ));
        assert_eq!(fs::read_to_string(claude).unwrap(), "changed later");
    }

    #[test]
    fn backup_failure_prevents_target_mutation() {
        let root = TempDir::new().expect("temp root");
        let home = root.path().join("home");
        let library = home.join(".agent-rules");
        let invalid_state_root = home.join("state-file");
        fs::create_dir_all(&home).expect("home");
        fs::write(&invalid_state_root, "not a directory").expect("state file");
        let manager = RulesManager::new(library, invalid_state_root, &home);
        manager.initialize().expect("initialize");

        assert!(manager.apply(&["codex".into()]).is_err());
        assert!(!home.join(".codex/AGENTS.md").exists());
    }

    #[test]
    fn automatic_restore_never_overwrites_a_concurrent_change() {
        let root = TempDir::new().expect("temp root");
        let first = root.path().join("first.md");
        let second = root.path().join("second.md");
        fs::write(&first, "applied first").expect("first applied");
        fs::write(&second, "external edit").expect("second external edit");
        let changes = vec![
            PreparedChange {
                agent_id: "first".into(),
                path: first.clone(),
                original: FileState::File {
                    content: "original first".into(),
                },
                desired: FileState::File {
                    content: "applied first".into(),
                },
            },
            PreparedChange {
                agent_id: "second".into(),
                path: second.clone(),
                original: FileState::File {
                    content: "original second".into(),
                },
                desired: FileState::File {
                    content: "applied second".into(),
                },
            },
        ];

        assert!(!restore_prepared_changes(&changes));
        assert_eq!(fs::read_to_string(first).unwrap(), "original first");
        assert_eq!(fs::read_to_string(second).unwrap(), "external edit");
    }

    #[cfg(unix)]
    #[test]
    fn claude_relative_link_to_source_is_in_sync() {
        let (_root, manager, home) = fixture();
        manager.initialize().expect("initialize");
        let claude = home.join(".claude/CLAUDE.md");
        fs::create_dir_all(claude.parent().unwrap()).expect("claude dir");
        std::os::unix::fs::symlink("../.agent-rules/AGENTS.md", &claude)
            .expect("relative source link");

        let snapshot = manager.snapshot().expect("snapshot");
        let status = snapshot
            .agents
            .iter()
            .find(|agent| agent.id == "claude")
            .expect("claude status");
        assert_eq!(status.state, TargetState::InSync);
    }
}
