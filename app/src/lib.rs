//! Tauri shell for Thira. Keeps no editing logic of its own; IPC handlers call into the engine crates.

/// Returns the application version, for the UI's about box and logs.
#[tauri::command]
fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Starts the application and blocks until the last window closes.
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![app_version])
        .run(tauri::generate_context!())
        .unwrap_or_else(|err| panic!("failed to start Thira: {err}"));
}
