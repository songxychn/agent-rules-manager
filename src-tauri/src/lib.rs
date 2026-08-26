use arm_core::{
    default_library_root, ApplyOutcome, ProjectionPlan, RollbackOutcome, RulesManager,
    WorkspaceSnapshot,
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
fn initialize_library(app: AppHandle, library_root: Option<String>) -> Result<String, String> {
    build_manager(&app, library_root)?
        .initialize()
        .map(|path| path.display().to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn preview_apply(
    app: AppHandle,
    library_root: Option<String>,
    agents: Vec<String>,
) -> Result<ProjectionPlan, String> {
    build_manager(&app, library_root)?
        .plan(&agents)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_open_targets() -> Vec<open_targets::OpenTarget> {
    open_targets::list_open_targets()
}

#[tauri::command]
fn open_source(
    app: AppHandle,
    library_root: Option<String>,
    target_id: String,
) -> Result<(), String> {
    let source = build_manager(&app, library_root)?.source_path();
    open_targets::open_source(&app, &target_id, &source)
}

#[tauri::command]
fn apply_rules(
    app: AppHandle,
    library_root: Option<String>,
    agents: Vec<String>,
) -> Result<ApplyOutcome, String> {
    build_manager(&app, library_root)?
        .apply(&agents)
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_workspace_snapshot,
            initialize_library,
            preview_apply,
            get_open_targets,
            open_source,
            apply_rules,
            rollback_latest
        ])
        .run(tauri::generate_context!())
        .expect("error while running Agent Rules Manager");
}
