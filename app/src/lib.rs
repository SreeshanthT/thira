//! Tauri shell for Thira. Keeps no editing logic of its own; IPC handlers call into the engine crates.

mod diagnostics;
mod smoke;
mod viewer;

use std::path::PathBuf;

use tauri::ipc::Response;
use tauri::{Manager, State, WebviewWindow, WindowEvent};
use thira_engine::{Engine, EngineError, EngineHandle, ProjectInfo};
use thira_render::{Viewport, wgpu};
use viewer::{Backend, ViewerHandle};

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

/// Which viewer backend is running: `native` or `transfer` (ADR 0003).
#[tauri::command]
fn viewer_backend(viewer: State<'_, ViewerHandle>) -> Backend {
    viewer.backend
}

/// Where the viewer sits in the window, in physical pixels from the top-left of
/// the webview. `None` hides it.
#[tauri::command]
fn set_viewer_rect(viewer: State<'_, ViewerHandle>, rect: Option<Rect>) {
    viewer.set_viewport(rect.map(|r| Viewport {
        x: r.x,
        y: r.y,
        width: r.width,
        height: r.height,
    }));
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

/// One RGBA frame for the transfer backend, as raw bytes.
#[tauri::command(async)]
fn viewer_frame(
    viewer: State<'_, ViewerHandle>,
    width: u32,
    height: u32,
) -> Result<Response, String> {
    viewer.frame(width, height).map(Response::new)
}

/// Starts the viewer for the main window. Runs on the main thread, where the
/// native surface must be created (ADR 0003).
fn start_viewer(window: &WebviewWindow) -> Result<ViewerHandle, Box<dyn std::error::Error>> {
    Ok(match Backend::choose() {
        Backend::Native => {
            let instance = wgpu::Instance::default();
            let surface = instance.create_surface(window.clone())?;
            let size = window.inner_size()?;
            ViewerHandle::native(instance, surface, (size.width, size.height))
        }
        Backend::Transfer => ViewerHandle::transfer(),
    })
}

/// Starts the application and blocks until the last window closes.
pub fn run() {
    diagnostics::install_panic_hook();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Engine::spawn())
        .setup(|app| {
            // Logging is best effort: the app works without it.
            match app.path().app_log_dir() {
                Ok(dir) => match diagnostics::init(&dir) {
                    Ok(guard) => {
                        app.manage(guard);
                    }
                    Err(err) => eprintln!("logging disabled ({}): {err}", dir.display()),
                },
                Err(err) => eprintln!("logging disabled: {err}"),
            }
            let window = app
                .get_webview_window("main")
                .ok_or("main window missing")?;
            let viewer = start_viewer(&window)?;
            if let Ok(out) = std::env::var("THIRA_SMOKE_OUT") {
                smoke::start(window.clone(), viewer.clone(), out.into());
            }
            app.manage(viewer);
            Ok(())
        })
        .on_window_event(|window, event| {
            let size = match event {
                WindowEvent::Resized(size) => *size,
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => *new_inner_size,
                _ => return,
            };
            if let Some(viewer) = window.try_state::<ViewerHandle>() {
                viewer.resize(size.width, size.height);
            }
        })
        .invoke_handler(tauri::generate_handler![
            app_version,
            new_project,
            open_project,
            save_project,
            save_project_as,
            close_project,
            project_info,
            viewer_backend,
            set_viewer_rect,
            viewer_frame,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|err| panic!("failed to start Thira: {err}"));
}
