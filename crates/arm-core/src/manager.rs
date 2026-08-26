use crate::{
    default_adapters, AgentAdapter, AgentStatus, ApplyOutcome, ArmError, ConnectionChange,
    DeployMode, PlanStep, ProjectionPlan, RollbackOutcome, TargetKind, TargetState,
    WorkspaceSnapshot,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
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

#[derive(Debug, Clone)]
struct TargetInspection {
    state: TargetState,
    kind: TargetKind,
    connected: bool,
    mode: DeployMode,
    detail: String,
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
        let selected = self.resolve_selection(selected)?;
        let changes = selected
            .into_iter()
            .map(|agent_id| ConnectionChange {
                agent_id,
                connected: true,
            })
            .collect::<Vec<_>>();
        self.plan_connections(&changes)
    }

    pub fn plan_connections(
        &self,
        changes: &[ConnectionChange],
    ) -> Result<ProjectionPlan, ArmError> {
        let source_path = self.source_path();
        if !source_path.exists() {
            return Err(ArmError::MissingSource(source_path.display().to_string()));
        }
        let requested = self.resolve_connection_changes(changes)?;
        let mut blocked = false;
        let mut change_count = 0;
        let mut steps = Vec::new();

        for adapter in self
            .adapters
            .iter()
            .filter(|adapter| requested.contains_key(&adapter.id))
        {
            let status = self.inspect_adapter(adapter, &source_path)?;
            let desired_connected = requested[&adapter.id];
            let (action, summary) = match (desired_connected, status.target_kind) {
                (true, TargetKind::ConnectedLink) => {
                    ("none", "Already links to the canonical rules.")
                }
                (true, TargetKind::Missing) => {
                    change_count += 1;
                    (
                        "createLink",
                        "Create a symbolic link to the canonical rules file.",
                    )
                }
                (true, TargetKind::LegacyInclude) => {
                    let current = read_file_state(&adapter.target_path)?;
                    if legacy_include_is_only_managed_content(&current) {
                        change_count += 1;
                        (
                            "migrateLegacyInclude",
                            "Replace the legacy managed include with a symbolic link.",
                        )
                    } else {
                        blocked = true;
                        (
                            "blocked",
                            "Legacy managed content is mixed with agent-specific rules; disconnect it before moving those rules aside.",
                        )
                    }
                }
                (true, TargetKind::IndependentFile) => {
                    blocked = true;
                    (
                        "blocked",
                        "An independent rules file is present and will not be overwritten.",
                    )
                }
                (true, TargetKind::ForeignLink | TargetKind::InvalidManagedFile) => {
                    blocked = true;
                    (
                        "blocked",
                        "An unexpected link or malformed managed file is present.",
                    )
                }
                (false, TargetKind::ConnectedLink) => {
                    change_count += 1;
                    (
                        "createIndependentFile",
                        "Replace the managed link with an independent copy of the current canonical rules.",
                    )
                }
                (false, TargetKind::LegacyInclude) => {
                    change_count += 1;
                    (
                        "detachManagedInclude",
                        "Replace the legacy managed include with an independent copy while preserving surrounding content.",
                    )
                }
                (false, _) => ("none", "Already uses an independent native path."),
            };
            steps.push(PlanStep {
                agent_id: status.id,
                agent_label: status.label,
                target_path: status.target_path,
                state: status.state,
                desired_connected,
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
        let selected = self.resolve_selection(selected)?;
        let changes = selected
            .into_iter()
            .map(|agent_id| ConnectionChange {
                agent_id,
                connected: true,
            })
            .collect::<Vec<_>>();
        self.apply_connections(&changes)
    }

    pub fn apply_connections(
        &self,
        changes: &[ConnectionChange],
    ) -> Result<ApplyOutcome, ArmError> {
        let source = self.source_path();
        let source = fs::canonicalize(&source)
            .map_err(|error| ArmError::io(source.display().to_string(), error))?;
        let plan = self.plan_connections(changes)?;
        if plan.blocked {
            let targets = plan
                .steps
                .iter()
                .filter(|step| step.action == "blocked")
                .map(|step| step.target_path.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(ArmError::Blocked(targets));
        }

        let changed_steps = plan
            .steps
            .iter()
            .filter(|step| step.action != "none" && step.action != "blocked")
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
            let desired = desired_connection_state(adapter, &source, &original, &step.action)?;
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
            if let Err(error) = replace_file_state(&change.path, &change.original, &change.desired) {
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

    fn resolve_connection_changes(
        &self,
        changes: &[ConnectionChange],
    ) -> Result<BTreeMap<String, bool>, ArmError> {
        let known = self
            .adapters
            .iter()
            .map(|adapter| adapter.id.as_str())
            .collect::<BTreeSet<_>>();
        let mut requested = BTreeMap::new();
        for change in changes {
            if !known.contains(change.agent_id.as_str()) {
                return Err(ArmError::UnknownAgent(change.agent_id.clone()));
            }
            requested.insert(change.agent_id.clone(), change.connected);
        }
        Ok(requested)
    }

    fn inspect_adapter(
        &self,
        adapter: &AgentAdapter,
        source_path: &Path,
    ) -> Result<AgentStatus, ArmError> {
        let inspection = inspect_projection_target(&adapter.target_path, source_path)?;
        let (installed, detection_detail) = detect_adapter(adapter);
        Ok(AgentStatus {
            id: adapter.id.clone(),
            label: adapter.label.clone(),
            target_path: adapter.target_path.display().to_string(),
            mode: inspection.mode,
            state: inspection.state,
            target_kind: inspection.kind,
            installed,
            connected: inspection.connected,
            detection_detail,
            detail: inspection.detail,
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

fn detect_adapter(adapter: &AgentAdapter) -> (bool, String) {
    if let Some(path) = adapter
        .detection_paths
        .iter()
        .find(|path| fs::symlink_metadata(path).is_ok())
    {
        return (
            true,
            format!("Detected installation marker at {}.", path.display()),
        );
    }

    for command in &adapter.command_names {
        if let Some(path) = find_command(command) {
            return (
                true,
                format!("Detected {command} at {}.", path.display()),
            );
        }
    }

    (
        false,
        "No supported command or configuration directory was detected.".into(),
    )
}

fn find_command(command: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    for directory in env::split_paths(&path) {
        #[cfg(windows)]
        let candidates = [
            directory.join(command),
            directory.join(format!("{command}.exe")),
            directory.join(format!("{command}.cmd")),
            directory.join(format!("{command}.bat")),
        ];
        #[cfg(not(windows))]
        let candidates = [directory.join(command)];

        for candidate in candidates {
            if fs::metadata(&candidate).is_ok_and(|metadata| metadata.is_file()) {
                return Some(candidate);
            }
        }
    }
    None
}

fn inspect_projection_target(
    target: &Path,
    source: &Path,
) -> Result<TargetInspection, ArmError> {
    match read_file_state(target)? {
        FileState::Missing => Ok(TargetInspection {
            state: TargetState::Ready,
            kind: TargetKind::Missing,
            connected: false,
            mode: DeployMode::Symlink,
            detail: "No native rules file exists yet.".into(),
        }),
        FileState::Symlink { target: linked } => {
            if link_points_to_source(target, Path::new(&linked), source) {
                Ok(TargetInspection {
                    state: TargetState::InSync,
                    kind: TargetKind::ConnectedLink,
                    connected: true,
                    mode: DeployMode::Symlink,
                    detail: "Linked directly to the canonical rules.".into(),
                })
            } else {
                Ok(TargetInspection {
                    state: TargetState::Conflict,
                    kind: TargetKind::ForeignLink,
                    connected: false,
                    mode: DeployMode::Symlink,
                    detail: "The existing symbolic link points somewhere else.".into(),
                })
            }
        }
        FileState::File { content } => match managed_include_span(&content) {
            Ok(None) => Ok(TargetInspection {
                state: TargetState::Ready,
                kind: TargetKind::IndependentFile,
                connected: false,
                mode: DeployMode::Symlink,
                detail: "Uses an independent native rules file.".into(),
            }),
            Ok(Some((start, end))) => {
                let current = &content[start..end];
                let detail = if current == include_block(source) {
                    "Uses the legacy managed include; new connections use symbolic links."
                } else {
                    "The legacy managed include points to a different canonical source."
                };
                Ok(TargetInspection {
                    state: TargetState::Drifted,
                    kind: TargetKind::LegacyInclude,
                    connected: true,
                    mode: DeployMode::Include,
                    detail: detail.into(),
                })
            }
            Err(()) => Ok(TargetInspection {
                state: TargetState::Conflict,
                kind: TargetKind::InvalidManagedFile,
                connected: false,
                mode: DeployMode::Include,
                detail: "Managed block markers are incomplete, duplicated, or out of order."
                    .into(),
            }),
        },
    }
}

fn link_points_to_source(target: &Path, linked: &Path, source: &Path) -> bool {
    let linked = if linked.is_absolute() {
        linked.to_path_buf()
    } else {
        target
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(linked)
    };
    let source = fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
    let linked = fs::canonicalize(&linked).unwrap_or(linked);
    linked == source
}

fn managed_include_span(content: &str) -> Result<Option<(usize, usize)>, ()> {
    let starts = content.match_indices(INCLUDE_START).collect::<Vec<_>>();
    let ends = content.match_indices(INCLUDE_END).collect::<Vec<_>>();
    match (starts.as_slice(), ends.as_slice()) {
        ([], []) => Ok(None),
        ([(start, _)], [(end, _)]) if start < end => Ok(Some((*start, end + INCLUDE_END.len()))),
        _ => Err(()),
    }
}

fn legacy_include_is_only_managed_content(state: &FileState) -> bool {
    let FileState::File { content } = state else {
        return false;
    };
    let Ok(Some((start, end))) = managed_include_span(content) else {
        return false;
    };
    format!("{}{}", &content[..start], &content[end..])
        .trim()
        .is_empty()
}

fn replace_file_state(
    path: &Path,
    original: &FileState,
    desired: &FileState,
) -> Result<(), ArmError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| ArmError::io(parent.display().to_string(), error))?;
    }
    match (original, desired) {
        (FileState::Missing, FileState::File { content }) => write_atomic(path, content),
        (FileState::Missing, FileState::Symlink { target }) => {
            create_symlink(Path::new(target), path)
        }
        (FileState::File { .. }, FileState::File { content }) => write_atomic(path, content),
        (FileState::Symlink { .. }, FileState::File { content }) => {
            fs::remove_file(path).map_err(|error| ArmError::io(path.display().to_string(), error))?;
            if let Err(error) = write_atomic(path, content) {
                let _ = restore_file_state(path, original);
                return Err(error);
            }
            Ok(())
        }
        (FileState::File { .. } | FileState::Symlink { .. }, FileState::Symlink { target }) => {
            fs::remove_file(path).map_err(|error| ArmError::io(path.display().to_string(), error))?;
            if let Err(error) = create_symlink(Path::new(target), path) {
                let _ = restore_file_state(path, original);
                return Err(error);
            }
            Ok(())
        }
        (_, FileState::Missing) => Err(ArmError::UnsupportedEntry(path.display().to_string())),
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

fn desired_connection_state(
    adapter: &AgentAdapter,
    source: &Path,
    current: &FileState,
    action: &str,
) -> Result<FileState, ArmError> {
    match action {
        "createLink" if matches!(current, FileState::Missing) => Ok(FileState::Symlink {
            target: source.display().to_string(),
        }),
        "migrateLegacyInclude" if legacy_include_is_only_managed_content(current) => {
            Ok(FileState::Symlink {
                target: source.display().to_string(),
            })
        }
        "createIndependentFile" => {
            let FileState::Symlink { target: linked } = current else {
                return Err(ArmError::ApplyDrift(
                    adapter.target_path.display().to_string(),
                ));
            };
            if !link_points_to_source(&adapter.target_path, Path::new(linked), source) {
                return Err(ArmError::ApplyDrift(
                    adapter.target_path.display().to_string(),
                ));
            }
            let content = fs::read_to_string(source)
                .map_err(|error| ArmError::io(source.display().to_string(), error))?;
            Ok(FileState::File { content })
        }
        "detachManagedInclude" => {
            let FileState::File { content } = current else {
                return Err(ArmError::ApplyDrift(
                    adapter.target_path.display().to_string(),
                ));
            };
            let Some((start, end)) = managed_include_span(content).map_err(|()| {
                ArmError::ApplyDrift(adapter.target_path.display().to_string())
            })?
            else {
                return Err(ArmError::ApplyDrift(
                    adapter.target_path.display().to_string(),
                ));
            };
            let source_content = fs::read_to_string(source)
                .map_err(|error| ArmError::io(source.display().to_string(), error))?;
            Ok(FileState::File {
                content: format!("{}{}{}", &content[..start], source_content, &content[end..]),
            })
        }
        _ => Err(ArmError::Blocked(adapter.target_path.display().to_string())),
    }
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

        let plan = manager.plan(&[]).expect("plan");
        assert!(!plan.blocked);
        assert_eq!(plan.change_count, 5);

        let applied = manager.apply(&[]).expect("apply");
        assert_eq!(applied.changed.len(), 5);
        assert!(home.join(".claude/CLAUDE.md").is_symlink());
        assert!(home.join(".codex/AGENTS.md").is_symlink());
        assert!(home.join(".qwen/QWEN.md").is_symlink());

        let rollback = manager.rollback_latest().expect("rollback");
        assert_eq!(rollback.restored.len(), 5);
        assert!(!home.join(".claude/CLAUDE.md").exists());
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
        fs::remove_file(&claude).expect("remove managed link");
        fs::write(&claude, "changed later").expect("drift");

        assert!(matches!(
            manager.rollback_latest(),
            Err(ArmError::RollbackDrift(_))
        ));
        assert_eq!(fs::read_to_string(claude).unwrap(), "changed later");
    }

    #[test]
    fn disconnect_materializes_an_independent_file_and_rollback_restores_the_link() {
        let (_root, manager, home) = fixture();
        let source = manager.initialize().expect("initialize");
        manager.apply(&["qwen".into()]).expect("connect qwen");
        let qwen = home.join(".qwen/QWEN.md");
        assert!(qwen.is_symlink());

        let changes = [ConnectionChange {
            agent_id: "qwen".into(),
            connected: false,
        }];
        let plan = manager.plan_connections(&changes).expect("disconnect plan");
        assert!(!plan.blocked);
        assert_eq!(plan.steps[0].action, "createIndependentFile");
        manager
            .apply_connections(&changes)
            .expect("disconnect qwen");

        assert!(!qwen.is_symlink());
        assert_eq!(fs::read_to_string(&qwen).unwrap(), fs::read_to_string(source).unwrap());

        manager.rollback_latest().expect("rollback disconnect");
        assert!(qwen.is_symlink());
    }

    #[test]
    fn legacy_include_can_be_detached_without_losing_surrounding_rules() {
        let (_root, manager, home) = fixture();
        let source = manager.initialize().expect("initialize");
        let claude = home.join(".claude/CLAUDE.md");
        fs::create_dir_all(claude.parent().unwrap()).expect("claude dir");
        fs::write(
            &claude,
            format!("Claude only\n\n{}\n", include_block(&source)),
        )
        .expect("legacy include");

        let changes = [ConnectionChange {
            agent_id: "claude".into(),
            connected: false,
        }];
        let plan = manager.plan_connections(&changes).expect("detach plan");
        assert_eq!(plan.steps[0].action, "detachManagedInclude");
        manager.apply_connections(&changes).expect("detach include");

        let detached = fs::read_to_string(claude).unwrap();
        assert!(detached.contains("Claude only"));
        assert!(detached.contains("# Shared agent rules"));
        assert!(!detached.contains(INCLUDE_START));
    }

    #[test]
    fn configuration_directory_marks_qwen_as_installed() {
        let (_root, manager, home) = fixture();
        fs::create_dir_all(home.join(".qwen")).expect("qwen config");

        let snapshot = manager.snapshot().expect("snapshot");
        let qwen = snapshot
            .agents
            .iter()
            .find(|agent| agent.id == "qwen")
            .expect("qwen adapter");
        assert!(qwen.installed);
        assert_eq!(qwen.target_path, home.join(".qwen/QWEN.md").display().to_string());
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
