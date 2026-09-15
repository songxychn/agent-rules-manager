use arm_core::{
    default_home_dir, default_library_root, ApplyOutcome, ConnectionChange, LibraryMutationOutcome,
    LibraryPlan, ProjectionPlan, RollbackOutcome, RulesManager, WorkspaceSnapshot,
};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

mod open_targets;

fn build_manager(app: &AppHandle, library_root: Option<String>) -> Result<RulesManager, String> {
    let root = match library_root {
        Some(root) => PathBuf::from(root),
        None => default_library_root().map_err(|error| error.to_string())?,
    };
    let state_root = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let home = default_home_dir().map_err(|error| error.to_string())?;
    Ok(RulesManager::new(root, state_root, &home))
}

fn build_projection_manager(
    app: &AppHandle,
    library_root: Option<String>,
    project_root: Option<String>,
) -> Result<RulesManager, String> {
    let manager = build_manager(app, library_root)?;
    match project_root.filter(|path| !path.trim().is_empty()) {
        Some(project) => {
            let home = default_home_dir().map_err(|error| error.to_string())?;
            manager
                .for_project(&home, &PathBuf::from(project))
                .map_err(|e| e.to_string())
        }
        None => Ok(manager),
    }
}

#[tauri::command]
fn get_workspace_snapshot(
    app: AppHandle,
    library_root: Option<String>,
    project_root: Option<String>,
) -> Result<WorkspaceSnapshot, String> {
    build_projection_manager(&app, library_root, project_root)?
        .snapshot()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn preview_initialize(app: AppHandle, library_root: Option<String>) -> Result<LibraryPlan, String> {
    build_manager(&app, library_root)?
        .plan_initialize()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn initialize_library(
    app: AppHandle,
    library_root: Option<String>,
) -> Result<LibraryMutationOutcome, String> {
    build_manager(&app, library_root)?
        .initialize()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn preview_create_profile(
    app: AppHandle,
    library_root: Option<String>,
    id: String,
    name: String,
    description: String,
) -> Result<LibraryPlan, String> {
    build_manager(&app, library_root)?
        .plan_create_profile(&id, &name, &description)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn create_profile(
    app: AppHandle,
    library_root: Option<String>,
    id: String,
    name: String,
    description: String,
) -> Result<LibraryMutationOutcome, String> {
    build_manager(&app, library_root)?
        .create_profile(&id, &name, &description)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn preview_delete_profile(
    app: AppHandle,
    library_root: Option<String>,
    profile_id: String,
) -> Result<LibraryPlan, String> {
    build_manager(&app, library_root)?
        .plan_delete_profile(&profile_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn delete_profile(
    app: AppHandle,
    library_root: Option<String>,
    profile_id: String,
) -> Result<LibraryMutationOutcome, String> {
    build_manager(&app, library_root)?
        .delete_profile(&profile_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn preview_activate_profile(
    app: AppHandle,
    library_root: Option<String>,
    profile_id: String,
) -> Result<LibraryPlan, String> {
    build_manager(&app, library_root)?
        .plan_activate_profile(&profile_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn activate_profile(
    app: AppHandle,
    library_root: Option<String>,
    profile_id: String,
) -> Result<LibraryMutationOutcome, String> {
    build_manager(&app, library_root)?
        .activate_profile(&profile_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn preview_apply(
    app: AppHandle,
    library_root: Option<String>,
    project_root: Option<String>,
    changes: Vec<ConnectionChange>,
) -> Result<ProjectionPlan, String> {
    build_projection_manager(&app, library_root, project_root)?
        .plan_connections(&changes)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_open_targets() -> Vec<open_targets::OpenTarget> {
    open_targets::list_open_targets()
}

#[tauri::command]
fn open_rule_source(
    app: AppHandle,
    library_root: Option<String>,
    target_id: String,
    profile_id: String,
    relative_path: String,
) -> Result<(), String> {
    let source = build_manager(&app, library_root)?
        .rule_source_path(&profile_id, &relative_path)
        .map_err(|error| error.to_string())?;
    open_targets::open_source(&app, &target_id, &source)
}

#[tauri::command]
fn apply_rules(
    app: AppHandle,
    library_root: Option<String>,
    project_root: Option<String>,
    changes: Vec<ConnectionChange>,
    confirm_existing_files: bool,
) -> Result<ApplyOutcome, String> {
    build_projection_manager(&app, library_root, project_root)?
        .apply_connections_confirmed(&changes, confirm_existing_files)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn rollback_latest(
    app: AppHandle,
    library_root: Option<String>,
) -> Result<RollbackOutcome, String> {
    build_manager(&app, library_root)?
        .rollback_latest()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn rollback_library_latest(
    app: AppHandle,
    library_root: Option<String>,
) -> Result<RollbackOutcome, String> {
    build_manager(&app, library_root)?
        .rollback_library_latest()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_operation_history(
    app: AppHandle,
    library_root: Option<String>,
) -> Result<Vec<arm_core::HistoryRecord>, String> {
    build_manager(&app, library_root)?
        .operation_history()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn preview_restore_history(
    app: AppHandle,
    library_root: Option<String>,
    target_id: String,
) -> Result<arm_core::RestorePlan, String> {
    build_manager(&app, library_root)?
        .preview_restore(&target_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn restore_history(
    app: AppHandle,
    library_root: Option<String>,
    target_id: String,
    token: String,
) -> Result<RollbackOutcome, String> {
    build_manager(&app, library_root)?
        .restore_history(&target_id, &token)
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_workspace_snapshot,
            get_operation_history,
            preview_restore_history,
            restore_history,
            preview_initialize,
            initialize_library,
            preview_create_profile,
            create_profile,
            preview_delete_profile,
            delete_profile,
            preview_activate_profile,
            activate_profile,
            preview_apply,
            get_open_targets,
            open_rule_source,
            apply_rules,
            rollback_latest,
            rollback_library_latest
        ])
        .run(tauri::generate_context!())
        .expect("error while running Agent Rules Manager");
}
