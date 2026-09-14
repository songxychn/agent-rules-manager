//! A linear, append-only history over the existing per-operation backups.
//! Restoring a point appends an inverse operation instead of deleting its future.
use crate::manager::{
    file_state_digest, read_file_state, restore_file_state, validate_project_boundary, FileState,
};
use crate::{ArmError, RollbackOutcome};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRecord {
    pub id: String,
    pub created_at: String,
    pub operation: String,
    pub subjects: Vec<String>,
    pub scope: String,
    pub paths: Vec<String>,
    pub restorable: bool,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreStep {
    pub path: String,
    pub action: String,
    pub before: String,
    pub after: String,
    pub before_content: Option<String>,
    pub after_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePlan {
    pub target: HistoryRecord,
    pub token: String,
    pub operations: Vec<HistoryRecord>,
    pub steps: Vec<RestoreStep>,
    pub conflicts: Vec<String>,
    pub blocked: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    target_path: String,
    original: FileState,
    applied_digest: String,
    #[serde(default)]
    project_root: Option<PathBuf>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Backup {
    #[serde(default)]
    history_epoch: String,
    #[serde(default)]
    subjects: Vec<String>,
    id: String,
    created_at: String,
    #[serde(default = "projection_operation")]
    operation: String,
    entries: Vec<Entry>,
    #[serde(default)]
    created_directories: Vec<String>,
    #[serde(default)]
    removed_directories: Vec<String>,
}
fn projection_operation() -> String {
    "connectionChange".into()
}

struct Stored {
    record: HistoryRecord,
    backup: Backup,
    bytes: Vec<u8>,
}
struct Prepared {
    plan: RestorePlan,
    before: BTreeMap<String, FileState>,
    after: BTreeMap<String, FileState>,
    remove_dirs: Vec<PathBuf>,
    create_dirs: Vec<PathBuf>,
    boundaries: BTreeMap<String, PathBuf>,
}

pub(crate) struct MutationGuard {
    path: PathBuf,
}
impl MutationGuard {
    pub(crate) fn acquire(root: &Path) -> Result<Self, ArmError> {
        fs::create_dir_all(root).map_err(|e| ArmError::io(root.display().to_string(), e))?;
        let path = root.join("mutation.lock");
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| ArmError::io(path.display().to_string(), e))?;
        drop(file);
        let guard = Self { path };
        ensure_no_pending(root)?;
        Ok(guard)
    }
}
impl Drop for MutationGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn ensure_no_pending(root: &Path) -> Result<(), ArmError> {
    let path = root.join("history/pending.json");
    if path.exists() {
        return Err(ArmError::Blocked(format!(
            "An interrupted recovery needs inspection: {}",
            path.display()
        )));
    }
    Ok(())
}

fn read_history(root: &Path) -> Result<Vec<Stored>, ArmError> {
    let mut result = Vec::new();
    let epoch = archive_epoch(root)?;
    for (directory, scope, archived) in [
        ("library-backups", "profiles", false),
        ("backups", "agents", false),
        ("history", "recovery", false),
        ("library-backups/restored", "profiles", true),
        ("backups/restored", "agents", true),
    ] {
        let dir = root.join(directory);
        if !dir.exists() {
            continue;
        }
        for item in fs::read_dir(&dir).map_err(|e| ArmError::io(dir.display().to_string(), e))? {
            let path = item
                .map_err(|e| ArmError::io(dir.display().to_string(), e))?
                .path();
            if path.extension().and_then(|x| x.to_str()) != Some("json")
                || path.file_name().and_then(|x| x.to_str()) == Some("pending.json")
            {
                continue;
            }
            let bytes = fs::read(&path).map_err(|e| ArmError::io(path.display().to_string(), e))?;
            let backup: Backup = serde_json::from_slice(&bytes)?;
            let mut subjects = BTreeSet::new();
            for entry in &backup.entries {
                let p = Path::new(&entry.target_path);
                if scope == "agents" {
                    subjects.insert(entry.target_path.clone());
                } else {
                    let parts: Vec<_> = p.components().collect();
                    if let Some(i) = parts.iter().position(|x| x.as_os_str() == "profiles") {
                        if let Some(id) = parts.get(i + 1) {
                            subjects.insert(id.as_os_str().to_string_lossy().into_owned());
                        }
                    }
                }
            }
            result.push(Stored {
                record: HistoryRecord {
                    id: format!("{scope}:{}", backup.id),
                    created_at: backup.created_at.clone(),
                    operation: backup.operation.clone(),
                    subjects: if backup.subjects.is_empty() {
                        subjects.into_iter().collect()
                    } else {
                        backup.subjects.clone()
                    },
                    scope: scope.into(),
                    paths: backup
                        .entries
                        .iter()
                        .map(|e| e.target_path.clone())
                        .collect(),
                    restorable: !archived && backup.history_epoch == epoch,
                    unavailable_reason: if archived {
                        Some("legacyRestored".into())
                    } else if backup.history_epoch != epoch {
                        Some("legacyBoundary".into())
                    } else {
                        None
                    },
                },
                backup,
                bytes,
            });
        }
    }
    result.sort_by(|a, b| {
        a.record
            .created_at
            .cmp(&b.record.created_at)
            .then(a.record.id.cmp(&b.record.id))
    });
    Ok(result)
}

fn baseline(history: &[Stored]) -> Option<HistoryRecord> {
    let first = history.iter().find(|s| s.record.restorable)?;
    Some(HistoryRecord {
        id: format!("baseline:{}", first.record.id),
        created_at: first.record.created_at.clone(),
        operation: "historyBaseline".into(),
        subjects: Vec::new(),
        scope: "recovery".into(),
        paths: Vec::new(),
        restorable: true,
        unavailable_reason: None,
    })
}

pub(crate) fn list(root: &Path) -> Result<Vec<HistoryRecord>, ArmError> {
    let history = read_history(root)?;
    let start = baseline(&history);
    let mut records = history
        .into_iter()
        .rev()
        .map(|s| s.record)
        .collect::<Vec<_>>();
    if let Some(start) = start {
        let boundary = records
            .iter()
            .rposition(|record| record.restorable)
            .map_or(0, |i| i + 1);
        records.insert(boundary, start);
    }
    Ok(records)
}

// Never follow a replaced parent link while inspecting or writing a backup path.
fn validate_path(path: &Path) -> Result<(), ArmError> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(ArmError::UnsupportedEntry(path.display().to_string()));
    }
    for parent in path.ancestors().skip(1) {
        match fs::symlink_metadata(parent) {
            Ok(m) if !m.is_dir() || m.file_type().is_symlink() => {
                return Err(ArmError::UnsupportedEntry(parent.display().to_string()))
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(ArmError::io(parent.display().to_string(), e)),
        }
    }
    Ok(())
}

fn file_content(state: &FileState) -> Option<String> {
    match state {
        FileState::File { content } => Some(content.clone()),
        _ => None,
    }
}

fn describe(state: &FileState) -> String {
    match state {
        FileState::Missing => "missing".into(),
        FileState::File { content } => {
            let profile = serde_json::from_str::<serde_json::Value>(content)
                .ok()
                .and_then(|v| {
                    v.get("activeProfileId")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                });
            profile
                .map(|id| format!("profile:{id}"))
                .unwrap_or_else(|| "file".into())
        }
        FileState::Symlink { target } => format!("link:{target}"),
    }
}

fn prepare(root: &Path, target: &str) -> Result<Prepared, ArmError> {
    ensure_no_pending(root)?;
    let history = read_history(root)?;
    let start = baseline(&history).filter(|r| r.id == target);
    let (selected, following) = if let Some(start) = start {
        let index = history.iter().position(|s| s.record.restorable).unwrap();
        (start, index)
    } else {
        let index = history
            .iter()
            .position(|s| s.record.id == target)
            .ok_or(ArmError::NoBackup)?;
        (history[index].record.clone(), index + 1)
    };
    if !selected.restorable {
        return Err(ArmError::Blocked(
            "This legacy point has no complete recovery chain".into(),
        ));
    }
    let mut digest = Sha256::new();
    digest.update(target.as_bytes());
    for s in &history {
        digest.update(&s.bytes);
        digest.update([s.record.restorable as u8]);
    }
    let mut before = BTreeMap::new();
    let mut after = BTreeMap::new();
    let mut conflicts = BTreeSet::new();
    let mut directories = BTreeMap::new();
    let mut boundaries = BTreeMap::new();
    let operations: Vec<_> = history[following..]
        .iter()
        .filter(|s| s.record.restorable)
        .collect();
    for stored in operations.iter().rev() {
        for entry in stored.backup.entries.iter().rev() {
            let path = Path::new(&entry.target_path);
            if let Some(root) = &entry.project_root {
                boundaries.insert(entry.target_path.clone(), root.clone());
            }
            if validate_path(path)
                .and_then(|_| validate_project_boundary(path, entry.project_root.as_deref()))
                .is_err()
            {
                conflicts.insert(entry.target_path.clone());
                continue;
            }
            if !before.contains_key(&entry.target_path) {
                match read_file_state(path) {
                    Ok(state) => {
                        before.insert(entry.target_path.clone(), state.clone());
                        after.insert(entry.target_path.clone(), state);
                    }
                    Err(_) => {
                        conflicts.insert(entry.target_path.clone());
                        continue;
                    }
                }
            }
            let state = &after[&entry.target_path];
            let hash = file_state_digest(state)?;
            if hash != entry.applied_digest && hash != file_state_digest(&entry.original)? {
                conflicts.insert(entry.target_path.clone());
            }
            after.insert(entry.target_path.clone(), entry.original.clone());
        }
        for dir in &stored.backup.created_directories {
            directories.insert(PathBuf::from(dir), false);
        }
        for dir in &stored.backup.removed_directories {
            directories.insert(PathBuf::from(dir), true);
        }
    }
    digest.update(serde_json::to_vec(&before)?);
    let mut create_dirs: Vec<_> = directories
        .iter()
        .filter(|(_, exists)| **exists)
        .map(|(p, _)| p.clone())
        .collect();
    create_dirs.sort_by_key(|p| p.components().count());
    for dir in &create_dirs {
        if validate_path(&dir.join(".boundary-check")).is_err() {
            conflicts.insert(dir.display().to_string());
        }
    }
    digest.update(serde_json::to_vec(
        &directories
            .iter()
            .map(|(p, exists)| (p, exists, p.exists()))
            .collect::<Vec<_>>(),
    )?);
    let mut remove_dirs: Vec<_> = directories
        .into_iter()
        .filter(|(_, exists)| !exists)
        .map(|(p, _)| p)
        .filter(|dir| {
            !after.iter().any(|(path, state)| {
                !matches!(state, FileState::Missing) && Path::new(path).starts_with(dir)
            })
        })
        .collect();
    remove_dirs.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    for dir in &remove_dirs {
        if validate_path(&dir.join(".boundary-check")).is_err() {
            conflicts.insert(dir.display().to_string());
            continue;
        }
        if !dir.exists() {
            continue;
        }
        for item in fs::read_dir(dir).map_err(|e| ArmError::io(dir.display().to_string(), e))? {
            let p = item
                .map_err(|e| ArmError::io(dir.display().to_string(), e))?
                .path();
            if !remove_dirs.contains(&p)
                && !matches!(
                    after.get(&p.display().to_string()),
                    Some(FileState::Missing)
                )
            {
                conflicts.insert(p.display().to_string());
            }
        }
    }
    let steps = after
        .iter()
        .filter(|(p, state)| before.get(*p) != Some(*state))
        .map(|(p, state)| RestoreStep {
            path: p.clone(),
            action: match state {
                FileState::Missing => "remove",
                FileState::File { .. } => "restoreFile",
                FileState::Symlink { .. } => "restoreLink",
            }
            .into(),
            before: describe(&before[p]),
            after: describe(state),
            before_content: file_content(&before[p]),
            after_content: file_content(state),
        })
        .chain(
            remove_dirs
                .iter()
                .filter(|p| p.exists())
                .map(|p| RestoreStep {
                    path: p.display().to_string(),
                    action: "removeDirectory".into(),
                    before: "directory".into(),
                    after: "missing".into(),
                    before_content: None,
                    after_content: None,
                }),
        )
        .chain(
            create_dirs
                .iter()
                .filter(|p| !p.exists())
                .map(|p| RestoreStep {
                    path: p.display().to_string(),
                    action: "createDirectory".into(),
                    before: "missing".into(),
                    after: "directory".into(),
                    before_content: None,
                    after_content: None,
                }),
        )
        .collect();
    let blocked = !conflicts.is_empty();
    let plan = RestorePlan {
        target: selected,
        token: format!("{:x}", digest.finalize()),
        operations: operations.iter().rev().map(|s| s.record.clone()).collect(),
        steps,
        conflicts: conflicts.into_iter().collect(),
        blocked,
    };
    Ok(Prepared {
        plan,
        before,
        after,
        remove_dirs,
        create_dirs,
        boundaries,
    })
}

pub(crate) fn preview(root: &Path, target: &str) -> Result<RestorePlan, ArmError> {
    Ok(prepare(root, target)?.plan)
}

pub(crate) fn restore(root: &Path, target: &str, token: &str) -> Result<RollbackOutcome, ArmError> {
    let _guard = MutationGuard::acquire(root)?;
    let prepared = prepare(root, target)?;
    if prepared.plan.blocked || prepared.plan.token != token {
        return Err(ArmError::Blocked(
            "Recovery preview is stale or has conflicts; preview again".into(),
        ));
    }
    if prepared.plan.steps.is_empty() {
        return Err(ArmError::Blocked("Already at this state".into()));
    }
    let id = Utc::now().format("%Y%m%dT%H%M%S%.9fZ").to_string();
    let mut created_directories = BTreeSet::new();
    for (path, state) in &prepared.after {
        if matches!(state, FileState::Missing) {
            continue;
        }
        for parent in Path::new(path)
            .ancestors()
            .skip(1)
            .take_while(|p| !p.exists())
        {
            created_directories.insert(parent.display().to_string());
        }
    }
    for dir in &prepared.create_dirs {
        for parent in dir.ancestors().take_while(|p| !p.exists()) {
            created_directories.insert(parent.display().to_string());
        }
    }
    let backup = Backup {
        history_epoch: archive_epoch(root)?,
        subjects: vec![prepared.plan.target.created_at.clone()],
        id: id.clone(),
        created_at: Utc::now().to_rfc3339(),
        operation: "restoreHistory".into(),
        entries: prepared
            .after
            .iter()
            .filter(|(p, s)| prepared.before.get(*p) != Some(*s))
            .map(|(p, state)| {
                Ok(Entry {
                    target_path: p.clone(),
                    original: prepared.before[p].clone(),
                    applied_digest: file_state_digest(state)?,
                    project_root: prepared.boundaries.get(p).cloned(),
                })
            })
            .collect::<Result<Vec<_>, ArmError>>()?,
        created_directories: created_directories.into_iter().collect(),
        removed_directories: prepared
            .remove_dirs
            .iter()
            .filter(|p| p.exists())
            .map(|p| p.display().to_string())
            .collect(),
    };
    let directory = root.join("history");
    fs::create_dir_all(&directory).map_err(|e| ArmError::io(directory.display().to_string(), e))?;
    let pending = directory.join("pending.json");
    let bytes = serde_json::to_vec_pretty(&backup)?;
    // Persist recovery material before touching any managed file.
    {
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&pending)
            .map_err(|e| ArmError::io(pending.display().to_string(), e))?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| ArmError::io(pending.display().to_string(), e))?;
    }
    let mut completed: Vec<&Entry> = Vec::new();
    let execution = (|| {
        for dir in &prepared.create_dirs {
            validate_path(&dir.join(".boundary-check"))?;
            fs::create_dir_all(dir).map_err(|e| ArmError::io(dir.display().to_string(), e))?;
        }
        for entry in &backup.entries {
            let path = Path::new(&entry.target_path);
            validate_path(path)?;
            validate_project_boundary(path, entry.project_root.as_deref())?;
            if read_file_state(path)? != entry.original {
                return Err(ArmError::RollbackDrift(entry.target_path.clone()));
            }
            completed.push(entry);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| ArmError::io(parent.display().to_string(), e))?;
            }
            restore_file_state(path, &prepared.after[&entry.target_path])?;
        }
        for dir in &prepared.remove_dirs {
            if dir.exists() {
                fs::remove_dir(dir).map_err(|e| ArmError::io(dir.display().to_string(), e))?;
            }
        }
        fs::rename(&pending, directory.join(format!("{id}.json")))
            .map_err(|e| ArmError::io(pending.display().to_string(), e))?;
        Ok::<_, ArmError>(())
    })();
    if let Err(error) = execution {
        let mut recovered = true;
        for dir in &backup.removed_directories {
            let dir = Path::new(dir);
            recovered &= validate_path(&dir.join(".boundary-check")).is_ok()
                && fs::create_dir_all(dir).is_ok();
        }
        for entry in completed.into_iter().rev() {
            let path = Path::new(&entry.target_path);
            let result = (|| {
                validate_path(path)?;
                validate_project_boundary(path, entry.project_root.as_deref())?;
                let state = read_file_state(path)?;
                if state == entry.original {
                    return Ok(());
                }
                if state != prepared.after[&entry.target_path] {
                    return Err(ArmError::RollbackDrift(entry.target_path.clone()));
                }
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| ArmError::io(parent.display().to_string(), e))?;
                }
                restore_file_state(path, &entry.original)
            })();
            recovered &= result.is_ok();
        }
        let mut dirs = backup
            .created_directories
            .iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        dirs.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
        for dir in dirs {
            if dir.exists() {
                recovered &= fs::remove_dir(dir).is_ok();
            }
        }
        if recovered {
            let _ = fs::remove_file(&pending);
        }
        return Err(error);
    }
    Ok(RollbackOutcome {
        restored: prepared.plan.steps.iter().map(|s| s.path.clone()).collect(),
        backup_id: id,
    })
}

/// A legacy rollback has no inverse record. New operations start a new safe era.
pub(crate) fn archive_epoch(root: &Path) -> Result<String, ArmError> {
    let mut paths = Vec::new();
    for name in ["backups/restored", "library-backups/restored"] {
        let dir = root.join(name);
        if !dir.exists() {
            continue;
        }
        for entry in fs::read_dir(&dir).map_err(|e| ArmError::io(dir.display().to_string(), e))? {
            let path = entry
                .map_err(|e| ArmError::io(dir.display().to_string(), e))?
                .path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                paths.push(path);
            }
        }
    }
    paths.sort();
    if paths.is_empty() {
        return Ok(String::new());
    }
    let mut digest = Sha256::new();
    for path in paths {
        digest.update(path.to_string_lossy().as_bytes());
        digest.update(fs::read(&path).map_err(|e| ArmError::io(path.display().to_string(), e))?);
    }
    Ok(format!("{:x}", digest.finalize()))
}
