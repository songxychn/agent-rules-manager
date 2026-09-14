use crate::{
    default_adapters, library, AgentAdapter, AgentStatus, ApplyOutcome, ArmError, ConnectionChange,
    DeployMode, LibraryMutationOutcome, LibraryPlan, PlanStep, ProjectionPlan, RollbackOutcome,
    TargetKind, TargetState, WorkspaceSnapshot,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

const INCLUDE_START: &str = "<!-- agent-rules-manager:start -->";
const INCLUDE_END: &str = "<!-- agent-rules-manager:end -->";

#[derive(Debug, Clone)]
pub struct RulesManager {
    library_root: PathBuf,
    state_root: PathBuf,
    adapters: Vec<AgentAdapter>,
    project_root: Option<PathBuf>,
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
    #[serde(default)]
    project_root: Option<PathBuf>,
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
    preserve_original: bool,
    project_root: Option<PathBuf>,
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
            project_root: None,
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
            project_root: None,
        }
    }

    /// Select project-native targets instead of machine-global targets.
    pub fn for_project(mut self, home: &Path, project: &Path) -> Result<Self, ArmError> {
        self.adapters = crate::project_adapters(home, project)?;
        let project = project
            .canonicalize()
            .map_err(|e| ArmError::io(project.display().to_string(), e))?;
        for protected in [&self.library_root, &self.state_root] {
            let protected = protected
                .canonicalize()
                .unwrap_or(absolute_logical_path(protected)?);
            if self
                .adapters
                .iter()
                .any(|a| a.target_path.starts_with(&protected))
            {
                return Err(ArmError::UnsupportedEntry(project.display().to_string()));
            }
        }
        self.project_root = Some(project);
        Ok(self)
    }

    pub fn source_path(&self) -> PathBuf {
        library::source_path(&self.library_root)
    }

    pub fn plan_initialize(&self) -> Result<LibraryPlan, ArmError> {
        library::plan_initialize(&self.library_root, &self.state_root)
    }

    pub fn initialize(&self) -> Result<LibraryMutationOutcome, ArmError> {
        library::initialize(&self.library_root, &self.state_root)
    }

    pub fn plan_create_profile(
        &self,
        id: &str,
        name: &str,
        description: &str,
    ) -> Result<LibraryPlan, ArmError> {
        library::plan_create_profile(&self.library_root, id, name, description)
    }

    pub fn create_profile(
        &self,
        id: &str,
        name: &str,
        description: &str,
    ) -> Result<LibraryMutationOutcome, ArmError> {
        library::create_profile(&self.library_root, &self.state_root, id, name, description)
    }

    pub fn plan_delete_profile(&self, profile_id: &str) -> Result<LibraryPlan, ArmError> {
        library::plan_delete_profile(&self.library_root, &self.state_root, profile_id)
    }

    pub fn delete_profile(&self, profile_id: &str) -> Result<LibraryMutationOutcome, ArmError> {
        library::delete_profile(&self.library_root, &self.state_root, profile_id)
    }

    pub fn plan_activate_profile(&self, profile_id: &str) -> Result<LibraryPlan, ArmError> {
        library::plan_activate_profile(&self.library_root, &self.state_root, profile_id)
    }

    pub fn activate_profile(&self, profile_id: &str) -> Result<LibraryMutationOutcome, ArmError> {
        library::activate_profile(&self.library_root, &self.state_root, profile_id)
    }

    pub fn rule_source_path(
        &self,
        profile_id: &str,
        relative_path: &str,
    ) -> Result<PathBuf, ArmError> {
        library::rule_source_path(&self.library_root, profile_id, relative_path)
    }

    pub fn rollback_library_latest(&self) -> Result<RollbackOutcome, ArmError> {
        library::rollback_library_latest(&self.state_root)
    }

    pub fn snapshot(&self) -> Result<WorkspaceSnapshot, ArmError> {
        let library = library::inspect(&self.library_root, &self.state_root)?;
        let source_path = library.source_path.clone();
        let legacy_source = library.legacy_adapter_source_path.as_deref();
        let agents = self
            .adapters
            .iter()
            .map(|adapter| self.inspect_adapter(adapter, &source_path, legacy_source))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(WorkspaceSnapshot {
            library_root: self.library_root.display().to_string(),
            library_state: library.state,
            library_detail: library.detail,
            runtime_state: library.runtime_state,
            source_path: source_path.display().to_string(),
            source_exists: library.source_exists,
            source_digest: library.source_digest,
            source_modified_at: library.source_modified_at,
            active_profile_id: library.active_profile_id,
            active_source_path: library
                .active_source_path
                .map(|path| path.display().to_string()),
            legacy_source_path: library
                .legacy_source_path
                .map(|path| path.display().to_string()),
            profiles: library.profiles,
            latest_backup: self.latest_backup_path()?.and_then(|path| {
                path.file_stem()
                    .map(|value| value.to_string_lossy().to_string())
            }),
            latest_library_backup: library::latest_library_backup_id(&self.state_root)?,
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
        let library = library::inspect(&self.library_root, &self.state_root)?;
        let legacy_source = library.legacy_adapter_source_path.as_deref();
        let requested = self.resolve_connection_changes(changes)?;
        let mut blocked = false;
        let mut change_count = 0;
        let mut confirmation_count = 0;
        let mut steps = Vec::new();

        for adapter in self
            .adapters
            .iter()
            .filter(|adapter| requested.contains_key(&adapter.id))
        {
            let status = self.inspect_adapter(adapter, &source_path, legacy_source)?;
            let desired_connected = requested[&adapter.id];
            if desired_connected && status.warning.is_some() {
                blocked = true;
                steps.push(PlanStep {
                    agent_id: status.id,
                    agent_label: status.label,
                    target_path: status.target_path,
                    state: status.state,
                    desired_connected,
                    action: "blocked".into(),
                    summary: status.warning.unwrap(),
                    requires_confirmation: false,
                });
                continue;
            }
            let mut requires_confirmation = false;
            let (action, summary) = match (desired_connected, status.target_kind) {
                (true, TargetKind::ConnectedLink) => (
                    "none",
                    "Already links to the stable current rules entrypoint.",
                ),
                (true, TargetKind::Missing) => {
                    change_count += 1;
                    ("createLink", "Create a symbolic link to current/AGENTS.md.")
                }
                (true, TargetKind::LegacyLink) => {
                    change_count += 1;
                    (
                        "replaceLegacyLink",
                        "Move the managed legacy link to current/AGENTS.md.",
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
                        change_count += 1;
                        confirmation_count += 1;
                        requires_confirmation = true;
                        (
                            "backupAndCreateLink",
                            "Back up the complete existing file, then replace it with a symbolic link after confirmation.",
                        )
                    }
                }
                (true, TargetKind::IndependentFile) => {
                    change_count += 1;
                    confirmation_count += 1;
                    requires_confirmation = true;
                    (
                        "backupAndCreateLink",
                        "Back up the existing independent file, then replace it with a symbolic link after confirmation.",
                    )
                }
                (true, TargetKind::InvalidManagedFile) => {
                    change_count += 1;
                    confirmation_count += 1;
                    requires_confirmation = true;
                    (
                        "backupAndCreateLink",
                        "Back up the complete malformed regular file, then replace it with a symbolic link after confirmation.",
                    )
                }
                (true, TargetKind::ForeignLink) => {
                    blocked = true;
                    (
                        "blocked",
                        "An unexpected symbolic link is present and will not be replaced.",
                    )
                }
                (false, TargetKind::ConnectedLink) => {
                    change_count += 1;
                    (
                        "createIndependentFile",
                        "Replace the managed link with an independent copy of the current canonical rules.",
                    )
                }
                (false, TargetKind::LegacyLink) => {
                    change_count += 1;
                    (
                        "createIndependentFile",
                        "Replace the legacy link with an independent copy of the current rules.",
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
                requires_confirmation,
            });
        }

        Ok(ProjectionPlan {
            source_path: source_path.display().to_string(),
            blocked,
            change_count,
            confirmation_count,
            steps,
        })
    }

    pub fn apply(&self, selected: &[String]) -> Result<ApplyOutcome, ArmError> {
        self.apply_confirmed(selected, false)
    }

    pub fn apply_confirmed(
        &self,
        selected: &[String],
        confirm_existing_files: bool,
    ) -> Result<ApplyOutcome, ArmError> {
        let selected = self.resolve_selection(selected)?;
        let changes = selected
            .into_iter()
            .map(|agent_id| ConnectionChange {
                agent_id,
                connected: true,
            })
            .collect::<Vec<_>>();
        self.apply_connections_confirmed(&changes, confirm_existing_files)
    }

    pub fn apply_connections(
        &self,
        changes: &[ConnectionChange],
    ) -> Result<ApplyOutcome, ArmError> {
        self.apply_connections_confirmed(changes, false)
    }

    pub fn apply_connections_confirmed(
        &self,
        changes: &[ConnectionChange],
        confirm_existing_files: bool,
    ) -> Result<ApplyOutcome, ArmError> {
        let source = self.source_path();
        let metadata = fs::metadata(&source)
            .map_err(|error| ArmError::io(source.display().to_string(), error))?;
        if !metadata.is_file() {
            return Err(ArmError::MissingSource(source.display().to_string()));
        }
        let source = absolute_logical_path(&source)?;
        let library = library::inspect(&self.library_root, &self.state_root)?;
        let legacy_source = library.legacy_adapter_source_path.as_deref();
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
        if plan.confirmation_count > 0 && !confirm_existing_files {
            let targets = plan
                .steps
                .iter()
                .filter(|step| step.requires_confirmation)
                .map(|step| step.target_path.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(ArmError::ConfirmationRequired(targets));
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
                preserved_backup_dir: None,
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
            let desired =
                desired_connection_state(adapter, &source, legacy_source, &original, &step.action)?;
            prepared.push(PreparedChange {
                agent_id: step.agent_id.clone(),
                path,
                original,
                desired,
                preserve_original: step.requires_confirmation,
                project_root: self.project_root.clone(),
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
                        project_root: change.project_root.clone(),
                    })
                })
                .collect::<Result<Vec<_>, ArmError>>()?,
        };
        let preserved_backup_dir = self.write_preserved_originals(&backup_id, &prepared)?;
        let backup_path = match self.write_backup(&backup) {
            Ok(path) => path,
            Err(error) => {
                if let Some(path) = &preserved_backup_dir {
                    let _ = fs::remove_dir_all(path);
                }
                return Err(error);
            }
        };

        let mut completed = Vec::new();
        for (index, change) in prepared.iter().enumerate() {
            if let Err(error) =
                validate_project_boundary(&change.path, change.project_root.as_deref())
            {
                let _ = restore_prepared_changes(&prepared[..index]);
                return Err(error);
            }
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
            if let Err(error) = replace_file_state(&change.path, &change.original, &change.desired)
            {
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
            preserved_backup_dir: preserved_backup_dir.map(|path| path.display().to_string()),
        })
    }

    pub fn rollback_latest(&self) -> Result<RollbackOutcome, ArmError> {
        let backup_path = self.latest_backup_path()?.ok_or(ArmError::NoBackup)?;
        let data = fs::read_to_string(&backup_path)
            .map_err(|error| ArmError::io(backup_path.display().to_string(), error))?;
        let backup: BackupSnapshot = serde_json::from_str(&data)?;

        for entry in &backup.entries {
            let path = PathBuf::from(&entry.target_path);
            validate_project_boundary(&path, entry.project_root.as_deref())?;
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

    fn canonical_agent_id(&self, id: &str) -> Result<String, ArmError> {
        self.adapters
            .iter()
            .find(|adapter| adapter.id == id || adapter.aliases.iter().any(|alias| alias == id))
            .map(|adapter| adapter.id.clone())
            .ok_or_else(|| ArmError::UnknownAgent(id.into()))
    }

    fn resolve_selection(&self, selected: &[String]) -> Result<BTreeSet<String>, ArmError> {
        if selected.is_empty() {
            return Ok(self.adapters.iter().map(|a| a.id.clone()).collect());
        }
        selected
            .iter()
            .map(|id| self.canonical_agent_id(id))
            .collect()
    }

    fn resolve_connection_changes(
        &self,
        changes: &[ConnectionChange],
    ) -> Result<BTreeMap<String, bool>, ArmError> {
        let mut requested = BTreeMap::new();
        for change in changes {
            let id = self.canonical_agent_id(&change.agent_id)?;
            if let Some(previous) = requested.insert(id.clone(), change.connected) {
                if previous != change.connected {
                    return Err(ArmError::Blocked(format!(
                        "Conflicting requests for shared target {id}"
                    )));
                }
            }
        }
        Ok(requested)
    }

    fn inspect_adapter(
        &self,
        adapter: &AgentAdapter,
        source_path: &Path,
        legacy_source: Option<&Path>,
    ) -> Result<AgentStatus, ArmError> {
        validate_project_boundary(&adapter.target_path, self.project_root.as_deref())?;
        let inspection =
            inspect_projection_target(&adapter.target_path, source_path, legacy_source)?;
        let (installed, detection_detail) = detect_adapter(adapter);
        let warning = adapter.max_chars.and_then(|limit| {
            fs::read_to_string(source_path).ok().and_then(|content| {
                let count = content.encode_utf16().count();
                (count > limit).then(|| format!("Rules contain {count} characters; {} supports at most {limit}. Shorten the Profile before connecting; rules are never truncated.", adapter.label))
            })
        });
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
            scope: adapter.scope.clone(),
            note: adapter.note.clone(),
            docs_url: adapter.docs_url.clone(),
            max_chars: adapter.max_chars,
            warning,
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

    fn write_preserved_originals(
        &self,
        backup_id: &str,
        changes: &[PreparedChange],
    ) -> Result<Option<PathBuf>, ArmError> {
        let preserved = changes
            .iter()
            .filter(|change| change.preserve_original)
            .collect::<Vec<_>>();
        if preserved.is_empty() {
            return Ok(None);
        }

        let originals_root = self.state_root.join("backups/originals");
        fs::create_dir_all(&originals_root)
            .map_err(|error| ArmError::io(originals_root.display().to_string(), error))?;
        let final_path = originals_root.join(backup_id);
        if fs::symlink_metadata(&final_path).is_ok() {
            return Err(ArmError::UnsupportedEntry(final_path.display().to_string()));
        }
        let temporary = originals_root.join(format!(
            ".{backup_id}-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir(&temporary)
            .map_err(|error| ArmError::io(temporary.display().to_string(), error))?;

        let write_result = preserved
            .iter()
            .enumerate()
            .try_for_each(|(index, change)| {
                let FileState::File { content } = &change.original else {
                    return Err(ArmError::ApplyDrift(change.path.display().to_string()));
                };
                let file_name = change
                    .path
                    .file_name()
                    .ok_or_else(|| ArmError::UnsupportedEntry(change.path.display().to_string()))?;
                let agent_dir = temporary.join(format!(
                    "{index:02}-{}",
                    safe_backup_component(&change.agent_id)
                ));
                fs::create_dir(&agent_dir)
                    .map_err(|error| ArmError::io(agent_dir.display().to_string(), error))?;
                write_atomic(&agent_dir.join(file_name), content)
            });
        if let Err(error) = write_result {
            let _ = fs::remove_dir_all(&temporary);
            return Err(error);
        }
        if let Err(error) = fs::rename(&temporary, &final_path) {
            let _ = fs::remove_dir_all(&temporary);
            return Err(ArmError::io(final_path.display().to_string(), error));
        }
        Ok(Some(final_path))
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

fn absolute_logical_path(path: &Path) -> Result<PathBuf, ArmError> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    let current =
        env::current_dir().map_err(|error| ArmError::io(path.display().to_string(), error))?;
    Ok(current.join(path))
}

#[cfg(test)]
fn paths_resolve_equal(left: &Path, right: &Path) -> bool {
    let left = fs::canonicalize(left).unwrap_or_else(|_| left.to_path_buf());
    let right = fs::canonicalize(right).unwrap_or_else(|_| right.to_path_buf());
    left == right
}

fn normalize_logical_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn logical_paths_equal(left: &Path, right: &Path) -> bool {
    let make_absolute = |path: &Path| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            env::current_dir()
                .map(|current| current.join(path))
                .unwrap_or_else(|_| path.to_path_buf())
        }
    };
    normalize_logical_path(&make_absolute(left)) == normalize_logical_path(&make_absolute(right))
}

fn safe_backup_component(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "agent".into()
    } else {
        sanitized
    }
}

fn symlink_state_points_to(target_path: &Path, linked: &str, expected: &Path) -> bool {
    let linked = PathBuf::from(linked);
    let linked = if linked.is_absolute() {
        linked
    } else {
        target_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(linked)
    };
    logical_paths_equal(&linked, expected)
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
            return (true, format!("Detected {command} at {}.", path.display()));
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
    legacy_source: Option<&Path>,
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
            } else if legacy_source
                .is_some_and(|legacy| link_points_to_source(target, Path::new(&linked), legacy))
            {
                Ok(TargetInspection {
                    state: TargetState::Drifted,
                    kind: TargetKind::LegacyLink,
                    connected: true,
                    mode: DeployMode::Symlink,
                    detail: "The managed link still points to the migrated root AGENTS.md.".into(),
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
                detail: "Managed block markers are incomplete, duplicated, or out of order.".into(),
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
    logical_paths_equal(&linked, source)
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

/// Project instruction directories may not redirect a write outside the chosen root.
fn validate_project_boundary(path: &Path, root: Option<&Path>) -> Result<(), ArmError> {
    let Some(root) = root else {
        return Ok(());
    };
    let relative = path
        .strip_prefix(root)
        .map_err(|_| ArmError::UnsupportedEntry(path.display().to_string()))?;
    let mut current = root.to_path_buf();
    let parents = relative.parent().unwrap_or_else(|| Path::new(""));
    for component in std::iter::once(None).chain(parents.components().map(Some)) {
        if let Some(component) = component {
            current.push(component);
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) if !metadata.is_dir() || metadata.file_type().is_symlink() => {
                return Err(ArmError::UnsupportedEntry(current.display().to_string()))
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && current != root => {}
            Err(error) => return Err(ArmError::io(current.display().to_string(), error)),
        }
    }
    Ok(())
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
            fs::remove_file(path)
                .map_err(|error| ArmError::io(path.display().to_string(), error))?;
            if let Err(error) = write_atomic(path, content) {
                let _ = restore_file_state(path, original);
                return Err(error);
            }
            Ok(())
        }
        (FileState::File { .. } | FileState::Symlink { .. }, FileState::Symlink { target }) => {
            fs::remove_file(path)
                .map_err(|error| ArmError::io(path.display().to_string(), error))?;
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
        if validate_project_boundary(&change.path, change.project_root.as_deref()).is_err() {
            restored = false;
            continue;
        }
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
    legacy_source: Option<&Path>,
    current: &FileState,
    action: &str,
) -> Result<FileState, ArmError> {
    match action {
        "createLink" if matches!(current, FileState::Missing) => Ok(FileState::Symlink {
            target: source.display().to_string(),
        }),
        "replaceLegacyLink" => {
            let FileState::Symlink { target } = current else {
                return Err(ArmError::ApplyDrift(
                    adapter.target_path.display().to_string(),
                ));
            };
            if !legacy_source
                .is_some_and(|legacy| symlink_state_points_to(&adapter.target_path, target, legacy))
            {
                return Err(ArmError::ApplyDrift(
                    adapter.target_path.display().to_string(),
                ));
            }
            Ok(FileState::Symlink {
                target: source.display().to_string(),
            })
        }
        "migrateLegacyInclude" if legacy_include_is_only_managed_content(current) => {
            Ok(FileState::Symlink {
                target: source.display().to_string(),
            })
        }
        "backupAndCreateLink" if matches!(current, FileState::File { .. }) => {
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
            let points_to_current =
                link_points_to_source(&adapter.target_path, Path::new(linked), source);
            let points_to_legacy = legacy_source.is_some_and(|legacy| {
                link_points_to_source(&adapter.target_path, Path::new(linked), legacy)
            });
            if !points_to_current && !points_to_legacy {
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
            let Some((start, end)) = managed_include_span(content)
                .map_err(|()| ArmError::ApplyDrift(adapter.target_path.display().to_string()))?
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
        let manager =
            RulesManager::with_adapters(library, state, crate::adapters::test_adapters(&home));
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
        assert_eq!(plan.change_count, manager.adapters.len());

        let applied = manager.apply(&[]).expect("apply");
        assert_eq!(applied.changed.len(), manager.adapters.len());
        assert!(home.join(".claude/CLAUDE.md").is_symlink());
        assert!(home.join(".codex/AGENTS.md").is_symlink());
        assert!(home.join(".qwen/QWEN.md").is_symlink());

        let rollback = manager.rollback_latest().expect("rollback");
        assert_eq!(rollback.restored.len(), manager.adapters.len());
        assert!(!home.join(".claude/CLAUDE.md").exists());
        assert!(!home.join(".codex/AGENTS.md").exists());
    }

    #[test]
    fn unmanaged_regular_file_requires_confirmation_and_is_preserved() {
        let (_root, manager, home) = fixture();
        manager.initialize().expect("initialize");
        let codex = home.join(".codex/AGENTS.md");
        fs::create_dir_all(codex.parent().unwrap()).expect("codex dir");
        fs::write(&codex, "keep me").expect("codex file");

        let plan = manager.plan(&["codex".into()]).expect("plan");
        assert!(!plan.blocked);
        assert_eq!(plan.change_count, 1);
        assert_eq!(plan.confirmation_count, 1);
        assert_eq!(plan.steps[0].action, "backupAndCreateLink");
        assert!(plan.steps[0].requires_confirmation);
        assert!(matches!(
            manager.apply(&["codex".into()]),
            Err(ArmError::ConfirmationRequired(_))
        ));
        assert_eq!(fs::read_to_string(&codex).unwrap(), "keep me");

        let applied = manager
            .apply_confirmed(&["codex".into()], true)
            .expect("confirmed apply");
        let preserved_dir = PathBuf::from(
            applied
                .preserved_backup_dir
                .as_deref()
                .expect("preserved backup directory"),
        );
        assert!(codex.is_symlink());
        assert_eq!(
            fs::read_to_string(preserved_dir.join("00-codex/AGENTS.md")).unwrap(),
            "keep me"
        );

        manager.rollback_latest().expect("rollback takeover");
        assert!(!codex.is_symlink());
        assert_eq!(fs::read_to_string(codex).unwrap(), "keep me");
    }

    #[cfg(unix)]
    #[test]
    fn foreign_symlink_stays_blocked_after_takeover_confirmation() {
        let (_root, manager, home) = fixture();
        manager.initialize().expect("initialize");
        let external = home.join("external.md");
        fs::write(&external, "external rules").expect("external rules");
        let codex = home.join(".codex/AGENTS.md");
        fs::create_dir_all(codex.parent().unwrap()).expect("codex dir");
        std::os::unix::fs::symlink(&external, &codex).expect("foreign link");

        let plan = manager.plan(&["codex".into()]).expect("plan");
        assert!(plan.blocked);
        assert_eq!(plan.confirmation_count, 0);
        assert!(matches!(
            manager.apply_confirmed(&["codex".into()], true),
            Err(ArmError::Blocked(_))
        ));
        assert_eq!(fs::read_link(codex).unwrap(), external);
    }

    #[cfg(unix)]
    #[test]
    fn migrated_legacy_connections_can_move_to_current_after_root_removal() {
        let (_root, manager, home) = fixture();
        let library = home.join(".agent-rules");
        fs::create_dir_all(&library).expect("library");
        let legacy = library.join("AGENTS.md");
        fs::write(&legacy, "# Existing rules\n").expect("legacy rules");

        let codex = home.join(".codex/AGENTS.md");
        fs::create_dir_all(codex.parent().unwrap()).expect("codex dir");
        std::os::unix::fs::symlink(&legacy, &codex).expect("legacy codex link");
        let claude = home.join(".claude/CLAUDE.md");
        fs::create_dir_all(claude.parent().unwrap()).expect("claude dir");
        fs::write(&claude, format!("{}\n", include_block(&legacy))).expect("legacy include");

        manager.initialize().expect("import");
        assert!(!legacy.exists());
        assert_eq!(
            fs::read_to_string(library.join("profiles/default/AGENTS.md")).unwrap(),
            "# Existing rules\n"
        );
        let snapshot = manager.snapshot().expect("snapshot");
        assert!(snapshot
            .agents
            .iter()
            .filter(|agent| agent.id == "codex" || agent.id == "claude")
            .all(|agent| agent.state == TargetState::Drifted));

        manager
            .apply(&["codex".into(), "claude".into()])
            .expect("move connections");
        assert!(!legacy.exists());
        assert!(paths_resolve_equal(
            &home.join(".codex/AGENTS.md"),
            &manager.source_path()
        ));
        assert!(claude.is_symlink());
        assert!(paths_resolve_equal(&claude, &manager.source_path()));
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
        manager.initialize().expect("initialize");
        let source = manager.source_path();
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
        assert_eq!(
            fs::read_to_string(&qwen).unwrap(),
            fs::read_to_string(source).unwrap()
        );

        manager.rollback_latest().expect("rollback disconnect");
        assert!(qwen.is_symlink());
    }

    #[test]
    fn legacy_include_can_be_detached_without_losing_surrounding_rules() {
        let (_root, manager, home) = fixture();
        manager.initialize().expect("initialize");
        let source = manager.source_path();
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
        assert_eq!(
            qwen.target_path,
            home.join(".qwen/QWEN.md").display().to_string()
        );
    }

    #[test]
    fn backup_failure_prevents_target_mutation() {
        let root = TempDir::new().expect("temp root");
        let home = root.path().join("home");
        let library = home.join(".agent-rules");
        let state_root = home.join("state");
        fs::create_dir_all(&home).expect("home");
        let manager = RulesManager::with_adapters(
            library,
            state_root.clone(),
            crate::adapters::test_adapters(&home),
        );
        manager.initialize().expect("initialize");
        fs::write(state_root.join("backups"), "not a directory").expect("backup blocker");

        assert!(manager.apply(&["codex".into()]).is_err());
        assert!(!home.join(".codex/AGENTS.md").exists());
    }

    #[test]
    fn preserved_copy_failure_prevents_existing_file_takeover() {
        let root = TempDir::new().expect("temp root");
        let home = root.path().join("home");
        let library = home.join(".agent-rules");
        let state_root = home.join("state");
        fs::create_dir_all(&home).expect("home");
        let manager = RulesManager::with_adapters(
            library,
            state_root.clone(),
            crate::adapters::test_adapters(&home),
        );
        manager.initialize().expect("initialize");
        let codex = home.join(".codex/AGENTS.md");
        fs::create_dir_all(codex.parent().unwrap()).expect("codex dir");
        fs::write(&codex, "keep me").expect("codex file");
        fs::create_dir_all(state_root.join("backups")).expect("backup dir");
        fs::write(state_root.join("backups/originals"), "not a directory")
            .expect("original backup blocker");

        assert!(manager.apply_confirmed(&["codex".into()], true).is_err());
        assert!(!codex.is_symlink());
        assert_eq!(fs::read_to_string(codex).unwrap(), "keep me");
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
                preserve_original: false,
                project_root: None,
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
                preserve_original: false,
                project_root: None,
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
        std::os::unix::fs::symlink("../.agent-rules/current/AGENTS.md", &claude)
            .expect("relative source link");

        let snapshot = manager.snapshot().expect("snapshot");
        let status = snapshot
            .agents
            .iter()
            .find(|agent| agent.id == "claude")
            .expect("claude status");
        assert_eq!(status.state, TargetState::InSync);
    }

    #[cfg(unix)]
    #[test]
    fn direct_runtime_link_is_not_treated_as_the_stable_current_entrypoint() {
        let (_root, manager, home) = fixture();
        manager.initialize().expect("initialize");
        let current = home.join(".agent-rules/current");
        let runtime = home
            .join(".agent-rules")
            .join(fs::read_link(&current).expect("current target"))
            .join("AGENTS.md");
        let codex = home.join(".codex/AGENTS.md");
        fs::create_dir_all(codex.parent().unwrap()).expect("codex dir");
        std::os::unix::fs::symlink(runtime, &codex).expect("direct runtime link");

        let snapshot = manager.snapshot().expect("snapshot");
        let status = snapshot
            .agents
            .iter()
            .find(|agent| agent.id == "codex")
            .expect("codex status");
        assert_eq!(status.state, TargetState::Conflict);
        assert_eq!(status.target_kind, TargetKind::ForeignLink);
    }
    #[test]
    fn shared_antigravity_aliases_apply_once_and_reject_conflicting_requests() {
        let (_root, manager, _home) = fixture();
        manager.initialize().unwrap();
        assert_eq!(
            manager
                .plan(&["gemini".into(), "ag".into(), "antigravity".into()])
                .unwrap()
                .change_count,
            1
        );
        assert!(manager
            .plan_connections(&[
                ConnectionChange {
                    agent_id: "gemini".into(),
                    connected: true
                },
                ConnectionChange {
                    agent_id: "ag".into(),
                    connected: false
                },
            ])
            .is_err());
        let applied = manager.apply(&["ag".into(), "gemini".into()]).unwrap();
        assert_eq!(applied.changed.len(), 1);
        assert_eq!(
            manager.plan(&["antigravity".into()]).unwrap().change_count,
            0
        );
    }

    #[test]
    fn overlong_rules_block_connecting_without_truncation_and_allow_disconnect() {
        let (_root, manager, _home) = fixture();
        manager.initialize().unwrap();
        manager.apply(&["windsurf".into()]).unwrap();
        fs::write(
            manager.library_root.join("profiles/default/AGENTS.md"),
            "中".repeat(6100),
        )
        .unwrap();
        manager.activate_profile("default").unwrap();
        let plan = manager.plan(&["windsurf".into()]).unwrap();
        assert!(plan.blocked);
        assert!(plan.steps[0].summary.contains("6000"));
        assert_eq!(plan.change_count, 0);
        assert!(manager.apply(&["windsurf".into()]).is_err());
        manager
            .apply_connections(&[ConnectionChange {
                agent_id: "windsurf".into(),
                connected: false,
            }])
            .unwrap();
        let target = &manager
            .adapters
            .iter()
            .find(|a| a.id == "windsurf")
            .unwrap()
            .target_path;
        assert_eq!(
            fs::read_to_string(target).unwrap(),
            fs::read_to_string(manager.source_path()).unwrap()
        );
    }

    #[test]
    fn project_connect_switch_disconnect_and_rollback_keep_rules_and_existing_files() {
        let (_root, global, home) = fixture();
        global.initialize().unwrap();
        let project = home.join("project");
        fs::create_dir(&project).unwrap();
        fs::write(project.join("AGENTS.md"), "original project rules").unwrap();
        let manager = global.clone().for_project(&home, &project).unwrap();
        let plan = manager.plan(&["cursor".into()]).unwrap();
        assert_eq!(plan.confirmation_count, 1);
        assert!(manager.apply(&["cursor".into()]).is_err());
        assert_eq!(
            fs::read_to_string(project.join("AGENTS.md")).unwrap(),
            "original project rules"
        );
        manager.apply_confirmed(&["cursor".into()], true).unwrap();
        global.create_profile("work", "Work", "").unwrap();
        fs::write(
            global.library_root.join("profiles/work/AGENTS.md"),
            "switched rules",
        )
        .unwrap();
        global.activate_profile("work").unwrap();
        assert!(fs::read_to_string(project.join("AGENTS.md"))
            .unwrap()
            .contains("switched rules"));
        assert_eq!(manager.plan(&["cursor".into()]).unwrap().change_count, 0);
        manager
            .apply_connections(&[ConnectionChange {
                agent_id: "cursor".into(),
                connected: false,
            }])
            .unwrap();
        assert!(!project.join("AGENTS.md").is_symlink());
        global.rollback_latest().unwrap();
        assert!(project.join("AGENTS.md").is_symlink());
        global.rollback_latest().unwrap();
        assert_eq!(
            fs::read_to_string(project.join("AGENTS.md")).unwrap(),
            "original project rules"
        );
        assert!(!home.join(".codex/AGENTS.md").exists());
    }

    #[test]
    fn project_parent_links_block_apply_and_rollback_outside_the_project() {
        let (_root, global, home) = fixture();
        global.initialize().unwrap();
        let project = home.join("project");
        let outside = home.join("outside");
        fs::create_dir(&project).unwrap();
        fs::create_dir(&outside).unwrap();
        let manager = global.clone().for_project(&home, &project).unwrap();
        std::os::unix::fs::symlink(&outside, project.join(".github")).unwrap();
        assert!(manager.plan(&["copilot-ide".into()]).is_err());
        assert!(manager.apply(&["copilot-ide".into()]).is_err());
        assert!(!outside.join("copilot-instructions.md").exists());
        fs::remove_file(project.join(".github")).unwrap();
        manager.apply(&["copilot-ide".into()]).unwrap();
        fs::rename(project.join(".github"), project.join(".github-original")).unwrap();
        std::os::unix::fs::symlink(&outside, project.join(".github")).unwrap();
        assert!(global.rollback_latest().is_err());
        assert!(!outside.join("copilot-instructions.md").exists());
        assert!(project
            .join(".github-original/copilot-instructions.md")
            .is_symlink());
    }

    #[test]
    fn project_targets_cannot_replace_the_rule_library() {
        let (_root, manager, home) = fixture();
        manager.initialize().unwrap();
        let current = manager.library_root.join("current");
        assert!(manager.clone().for_project(&home, &current).is_err());
    }
}
