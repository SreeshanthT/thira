//! Tauri shell for Thira. Keeps no editing logic of its own; IPC handlers call into the engine crates.

use std::path::PathBuf;

use tauri::State;
use thira_engine::{Engine, EngineError, EngineHandle, ProjectInfo};

/// Returns the application version, for the UI's about box and logs.
#[tauri::command]
fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

// Engine calls block until the engine thread replies, so these run off the main thread.

/// Starts a new project. Fails with `unsaved` if the open one has changes, unless `force`.
#[tauri::command(async)]
fn new_project(engine: State<'_, EngineHandle>, force: bool) -> Result<ProjectInfo, EngineError> {
    engine.new_project("Untitled".into(), force)
}

#[tauri::command(async)]
fn open_project(
    engine: State<'_, EngineHandle>,
    path: PathBuf,
    force: bool,
) -> Result<ProjectInfo, EngineError> {
    engine.open(path, force)
}

#[tauri::command(async)]
fn save_project(engine: State<'_, EngineHandle>) -> Result<ProjectInfo, EngineError> {
    engine.save()
}

#[tauri::command(async)]
fn save_project_as(
    engine: State<'_, EngineHandle>,
    path: PathBuf,
) -> Result<ProjectInfo, EngineError> {
    engine.save_as(path)
}

#[tauri::command(async)]
fn close_project(engine: State<'_, EngineHandle>, force: bool) -> Result<(), EngineError> {
    engine.close(force)
}

#[tauri::command(async)]
fn project_info(engine: State<'_, EngineHandle>) -> Result<Option<ProjectInfo>, EngineError> {
    engine.info()
}

/// Starts the application and blocks until the last window closes.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Engine::spawn())
        .invoke_handler(tauri::generate_handler![
            app_version,
            new_project,
            open_project,
            save_project,
            save_project_as,
            close_project,
            project_info,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|err| panic!("failed to start Thira: {err}"));
}
