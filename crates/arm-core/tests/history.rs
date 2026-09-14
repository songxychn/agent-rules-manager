use arm_core::{ConnectionChange, RulesManager};
use std::{fs, path::PathBuf};
use tempfile::TempDir;

fn fixture() -> (TempDir, RulesManager, PathBuf, PathBuf) {
    let temp = TempDir::new().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let library = root.join("library");
    let state = root.join("state");
    let manager = RulesManager::new(library.clone(), state.clone(), &root);
    manager.initialize().unwrap();
    (temp, manager, library, state)
}

#[test]
fn mixed_history_restores_a_point_and_can_undo_the_recovery() {
    let (temp, manager, library, _) = fixture();
    let initial = manager.operation_history().unwrap()[0].id.clone();
    manager.create_profile("work", "Work", "").unwrap();
    manager.activate_profile("work").unwrap();
    manager
        .apply_connections(&[ConnectionChange {
            agent_id: "codex".into(),
            connected: true,
        }])
        .unwrap();
    let latest = manager.operation_history().unwrap()[0].id.clone();
    let native = temp.path().canonicalize().unwrap().join(".codex/AGENTS.md");
    let plan = manager.preview_restore(&initial).unwrap();
    assert!(!plan.blocked, "{:?}", plan.conflicts);
    assert_eq!(plan.operations.len(), 3);
    assert!(plan.steps.iter().any(|s| s.after == "profile:default"));
    manager.restore_history(&initial, &plan.token).unwrap();
    assert_eq!(
        manager.snapshot().unwrap().active_profile_id.as_deref(),
        Some("default")
    );
    assert!(!library.join("profiles/work").exists());
    assert!(fs::symlink_metadata(&native).is_err());
    let records = manager.operation_history().unwrap();
    assert_eq!(records[0].operation, "restoreHistory");
    assert!(records.iter().any(|r| r.id == latest));
    let undo = manager.preview_restore(&latest).unwrap();
    assert!(!undo.blocked, "{:?}", undo.conflicts);
    manager.restore_history(&latest, &undo.token).unwrap();
    assert!(library.join("profiles/work/AGENTS.md").exists());
    assert!(fs::symlink_metadata(native)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(
        manager.snapshot().unwrap().active_profile_id.as_deref(),
        Some("work")
    );
}

#[test]
fn conflict_preflight_changes_nothing_and_returns_the_specific_path() {
    let (_temp, manager, library, state) = fixture();
    let target = manager.operation_history().unwrap()[0].id.clone();
    manager.create_profile("work", "Work", "").unwrap();
    manager.activate_profile("work").unwrap();
    let source = library.join("profiles/work/AGENTS.md");
    fs::write(&source, "external edit").unwrap();
    let current = fs::read_link(library.join("current")).unwrap();
    let plan = manager.preview_restore(&target).unwrap();
    assert!(plan.blocked);
    assert!(plan.conflicts.contains(&source.display().to_string()));
    assert!(manager.restore_history(&target, &plan.token).is_err());
    assert_eq!(fs::read_link(library.join("current")).unwrap(), current);
    assert_eq!(fs::read_to_string(source).unwrap(), "external edit");
    assert!(!state.join("history/pending.json").exists());
}

#[test]
fn stale_preview_rejects_both_new_operations_and_late_file_edits() {
    let (_temp, manager, library, _) = fixture();
    let target = manager.operation_history().unwrap()[0].id.clone();
    manager.create_profile("work", "Work", "").unwrap();
    let plan = manager.preview_restore(&target).unwrap();
    manager.create_profile("personal", "Personal", "").unwrap();
    assert!(manager.restore_history(&target, &plan.token).is_err());
    let plan = manager.preview_restore(&target).unwrap();
    fs::write(library.join("profiles/work/AGENTS.md"), "late edit").unwrap();
    assert!(manager.restore_history(&target, &plan.token).is_err());
    assert!(library.join("profiles/personal").exists());
}

#[test]
fn later_unmanaged_entries_are_not_deleted_with_a_profile_directory() {
    let (_temp, manager, library, _) = fixture();
    let target = manager.operation_history().unwrap()[0].id.clone();
    manager.create_profile("work", "Work", "").unwrap();
    let path = library.join("profiles/work/notes.txt");
    fs::write(&path, "mine").unwrap();
    let plan = manager.preview_restore(&target).unwrap();
    assert!(plan.blocked);
    assert!(plan.conflicts.contains(&path.display().to_string()));
    assert!(manager.restore_history(&target, &plan.token).is_err());
    assert_eq!(fs::read_to_string(path).unwrap(), "mine");
}

#[test]
fn deleted_profile_can_be_recovered_and_deleted_again() {
    let (_temp, manager, library, _) = fixture();
    manager.create_profile("work", "Work", "").unwrap();
    let target = manager.operation_history().unwrap()[0].id.clone();
    manager.delete_profile("work").unwrap();
    let deleted = manager.operation_history().unwrap()[0].id.clone();
    let plan = manager.preview_restore(&target).unwrap();
    manager.restore_history(&target, &plan.token).unwrap();
    assert!(library.join("profiles/work/AGENTS.md").exists());
    let plan = manager.preview_restore(&deleted).unwrap();
    manager.restore_history(&deleted, &plan.token).unwrap();
    assert!(!library.join("profiles/work").exists());
}

#[test]
fn legacy_rollback_invalidates_old_points_but_allows_a_new_history_era() {
    let (_temp, manager, _, _) = fixture();
    manager.create_profile("work", "Work", "").unwrap();
    manager.rollback_library_latest().unwrap();
    assert!(manager
        .operation_history()
        .unwrap()
        .iter()
        .all(|r| !r.restorable));
    manager.create_profile("new", "New", "").unwrap();
    let target = manager.operation_history().unwrap()[0].id.clone();
    manager.create_profile("other", "Other", "").unwrap();
    let plan = manager.preview_restore(&target).unwrap();
    manager.restore_history(&target, &plan.token).unwrap();
}

#[test]
fn lock_and_interrupted_recovery_block_other_mutations() {
    let (_temp, manager, library, state) = fixture();
    fs::write(state.join("mutation.lock"), "busy").unwrap();
    assert!(manager.create_profile("blocked", "Blocked", "").is_err());
    fs::remove_file(state.join("mutation.lock")).unwrap();
    fs::create_dir_all(state.join("history")).unwrap();
    fs::write(state.join("history/pending.json"), "{}").unwrap();
    assert!(manager.create_profile("blocked", "Blocked", "").is_err());
    assert!(!library.join("profiles/blocked").exists());
    assert!(!state.join("mutation.lock").exists());
}

#[cfg(unix)]
#[test]
fn replaced_parent_link_cannot_redirect_a_recovery() {
    let (temp, manager, library, _) = fixture();
    let target = manager.operation_history().unwrap()[0].id.clone();
    manager.create_profile("work", "Work", "").unwrap();
    let outside = temp.path().canonicalize().unwrap().join("outside");
    fs::rename(library.join("profiles/work"), &outside).unwrap();
    std::os::unix::fs::symlink(&outside, library.join("profiles/work")).unwrap();
    let plan = manager.preview_restore(&target).unwrap();
    assert!(plan.blocked);
    assert!(manager.restore_history(&target, &plan.token).is_err());
    assert!(outside.join("AGENTS.md").exists());
}

#[test]
fn an_empty_directory_cleanup_can_itself_be_undone() {
    let (_temp, manager, library, _) = fixture();
    let target = manager.operation_history().unwrap()[0].id.clone();
    manager.create_profile("work", "Work", "").unwrap();
    let latest = manager.operation_history().unwrap()[0].id.clone();
    // Files already at their original (missing) state are supported by legacy
    // rollback semantics; the empty directory must still have recovery material.
    fs::remove_file(library.join("profiles/work/AGENTS.md")).unwrap();
    fs::remove_file(library.join("profiles/work/profile.json")).unwrap();
    let plan = manager.preview_restore(&target).unwrap();
    manager.restore_history(&target, &plan.token).unwrap();
    assert!(!library.join("profiles/work").exists());
    let undo = manager.preview_restore(&latest).unwrap();
    assert!(undo.steps.iter().any(|s| s.action == "createDirectory"));
    manager.restore_history(&latest, &undo.token).unwrap();
    assert!(library.join("profiles/work").is_dir());
}

#[test]
fn recovery_keeps_project_boundaries_for_subsequent_undo() {
    let (temp, manager, _, _) = fixture();
    let root = temp.path().canonicalize().unwrap();
    let project = root.join("project");
    fs::create_dir_all(&project).unwrap();
    let target = manager.operation_history().unwrap()[0].id.clone();
    let project_manager = manager.clone().for_project(&root, &project).unwrap();
    project_manager
        .apply_connections(&[ConnectionChange {
            agent_id: "cursor".into(),
            connected: true,
        }])
        .unwrap();
    let latest = manager.operation_history().unwrap()[0].id.clone();
    let plan = manager.preview_restore(&target).unwrap();
    manager.restore_history(&target, &plan.token).unwrap();
    fs::remove_dir_all(&project).unwrap();
    let undo = manager.preview_restore(&latest).unwrap();
    assert!(undo.blocked);
    assert!(manager.restore_history(&latest, &undo.token).is_err());
    assert!(!project.exists());
}

#[cfg(unix)]
#[test]
fn write_failure_compensates_completed_files_and_preserves_history() {
    use std::os::unix::fs::PermissionsExt;
    let (_temp, manager, library, state) = fixture();
    let target = manager.operation_history().unwrap()[0].id.clone();
    manager.create_profile("alpha", "Alpha", "").unwrap();
    manager.create_profile("beta", "Beta", "").unwrap();
    let before = manager.operation_history().unwrap().len();
    let alpha = fs::read(library.join("profiles/alpha/AGENTS.md")).unwrap();
    let beta_dir = library.join("profiles/beta");
    fs::set_permissions(&beta_dir, fs::Permissions::from_mode(0o500)).unwrap();
    let plan = manager.preview_restore(&target).unwrap();
    let result = manager.restore_history(&target, &plan.token);
    fs::set_permissions(&beta_dir, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(result.is_err());
    assert_eq!(
        fs::read(library.join("profiles/alpha/AGENTS.md")).unwrap(),
        alpha
    );
    assert!(beta_dir.join("AGENTS.md").exists());
    assert!(!state.join("history/pending.json").exists());
    assert_eq!(manager.operation_history().unwrap().len(), before);
}

#[cfg(unix)]
#[test]
fn failed_recovery_backup_never_changes_managed_files() {
    use std::os::unix::fs::PermissionsExt;
    let (_temp, manager, library, state) = fixture();
    let target = manager.operation_history().unwrap()[0].id.clone();
    manager.create_profile("work", "Work", "").unwrap();
    let directory = state.join("history");
    fs::create_dir_all(&directory).unwrap();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o500)).unwrap();
    let plan = manager.preview_restore(&target).unwrap();
    let result = manager.restore_history(&target, &plan.token);
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(result.is_err());
    assert!(library.join("profiles/work/AGENTS.md").exists());
}

#[test]
fn baseline_can_undo_the_first_operation_in_a_new_era() {
    let (_temp, manager, library, _) = fixture();
    manager.create_profile("old", "Old", "").unwrap();
    manager.rollback_library_latest().unwrap();
    manager.create_profile("first", "First", "").unwrap();
    let records = manager.operation_history().unwrap();
    let latest = records[0].id.clone();
    let target = records
        .iter()
        .find(|r| r.operation == "historyBaseline")
        .unwrap();
    let plan = manager.preview_restore(&target.id).unwrap();
    assert_eq!(plan.operations.len(), 1);
    manager.restore_history(&target.id, &plan.token).unwrap();
    assert!(!library.join("profiles/first").exists());
    let undo = manager.preview_restore(&latest).unwrap();
    manager.restore_history(&latest, &undo.token).unwrap();
    assert!(library.join("profiles/first/AGENTS.md").exists());
}

#[test]
fn initial_baseline_can_remove_and_recover_an_entire_new_library() {
    let (_temp, manager, library, _) = fixture();
    let records = manager.operation_history().unwrap();
    let target = records
        .iter()
        .find(|r| r.operation == "historyBaseline")
        .unwrap();
    let plan = manager.preview_restore(&target.id).unwrap();
    assert!(!plan.blocked, "{:?}", plan.conflicts);
    manager.restore_history(&target.id, &plan.token).unwrap();
    assert!(!library.exists());
    let undo = manager.preview_restore(&records[0].id).unwrap();
    manager
        .restore_history(&records[0].id, &undo.token)
        .unwrap();
    assert_eq!(
        manager.snapshot().unwrap().active_profile_id.as_deref(),
        Some("default")
    );
}
