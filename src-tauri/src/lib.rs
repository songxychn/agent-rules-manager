use arm_core::{
    default_library_root, ApplyOutcome, ConnectionChange, LibraryMutationOutcome, LibraryPlan,
    ProjectionPlan, RollbackOutcome, RulesManager, WorkspaceSnapshot,
};
use std::env;
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
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(RulesManager::new(root, state_root, &home))
}

#[tauri::command]
fn get_workspace_snapshot(
    app: AppHandle,
    library_root: Option<String>,
) -> Result<WorkspaceSnapshot, String> {
    build_manager(&app, library_root)?
        .snapshot()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn preview_initialize(
    app: AppHandle,
    library_root: Option<String>,
) -> Result<LibraryPlan, String> {
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
fn preview_create_pack(
    app: AppHandle,
    library_root: Option<String>,
    id: String,
    name: String,
    description: String,
) -> Result<LibraryPlan, String> {
    build_manager(&app, library_root)?
        .plan_create_pack(&id, &name, &description)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn create_pack(
    app: AppHandle,
    library_root: Option<String>,
    id: String,
    name: String,
    description: String,
) -> Result<LibraryMutationOutcome, String> {
    build_manager(&app, library_root)?
        .create_pack(&id, &name, &description)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn preview_add_pack_file(
    app: AppHandle,
    library_root: Option<String>,
    pack_id: String,
    relative_path: String,
) -> Result<LibraryPlan, String> {
    build_manager(&app, library_root)?
        .plan_add_pack_file(&pack_id, &relative_path)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn add_pack_file(
    app: AppHandle,
    library_root: Option<String>,
    pack_id: String,
    relative_path: String,
) -> Result<LibraryMutationOutcome, String> {
    build_manager(&app, library_root)?
        .add_pack_file(&pack_id, &relative_path)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn preview_create_profile(
    app: AppHandle,
    library_root: Option<String>,
    id: String,
    name: String,
    description: String,
    pack_ids: Vec<String>,
) -> Result<LibraryPlan, String> {
    build_manager(&app, library_root)?
        .plan_create_profile(&id, &name, &description, &pack_ids)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn create_profile(
    app: AppHandle,
    library_root: Option<String>,
    id: String,
    name: String,
    description: String,
    pack_ids: Vec<String>,
) -> Result<LibraryMutationOutcome, String> {
    build_manager(&app, library_root)?
        .create_profile(&id, &name, &description, &pack_ids)
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
    changes: Vec<ConnectionChange>,
) -> Result<ProjectionPlan, String> {
    build_manager(&app, library_root)?
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
    pack_id: String,
    relative_path: String,
) -> Result<(), String> {
    let source = build_manager(&app, library_root)?
        .rule_source_path(&pack_id, &relative_path)
        .map_err(|error| error.to_string())?;
    open_targets::open_source(&app, &target_id, &source)
}

#[tauri::command]
fn apply_rules(
    app: AppHandle,
    library_root: Option<String>,
    changes: Vec<ConnectionChange>,
) -> Result<ApplyOutcome, String> {
    build_manager(&app, library_root)?
        .apply_connections(&changes)
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_workspace_snapshot,
            preview_initialize,
            initialize_library,
            preview_create_pack,
            create_pack,
            preview_add_pack_file,
            add_pack_file,
            preview_create_profile,
            create_profile,
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
