//! Smoke test for CI (issue #11): with `THIRA_SMOKE_OUT=<file>`, waits for the
//! viewer to show frames, resizes the window, waits for frames at the new size,
//! writes a JSON report and exits. Exit code 0 means the viewer rendered and resized.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use tauri::{LogicalSize, Manager, WebviewWindow};

use crate::viewer::{Backend, Stats, ViewerHandle};

const FRAMES: u64 = 30;
const TIMEOUT: Duration = Duration::from_secs(90);

pub fn start(window: WebviewWindow, viewer: ViewerHandle, out: PathBuf) {
    std::thread::spawn(move || {
        let report = run(&window, &viewer);
        let ok = report
            .get("ok")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let text = serde_json::to_string_pretty(&report).unwrap_or_default();
        println!("{text}");
        if let Err(err) = std::fs::write(&out, text) {
            eprintln!("smoke: cannot write {}: {err}", out.display());
        }
        window.app_handle().exit(if ok { 0 } else { 1 });
    });
}

fn run(window: &WebviewWindow, viewer: &ViewerHandle) -> serde_json::Value {
    let first = wait(viewer, 0, None);
    let target = LogicalSize::new(1000.0, 640.0);
    if let Err(err) = window.set_size(target) {
        return serde_json::json!({ "ok": false, "error": format!("resize failed: {err}") });
    }
    let expected = window.inner_size().map(|s| (s.width, s.height)).ok();
    let second = first
        .as_ref()
        .ok()
        .map(|s| wait(viewer, s.frames, Some(s.size)));
    let ok = first.is_ok() && matches!(second, Some(Ok(_)));
    serde_json::json!({
        "ok": ok,
        "backend": viewer.backend,
        "before_resize": result_json(&first),
        "after_resize": second.as_ref().map(result_json),
        "window_inner_size_after_resize": expected,
    })
}

/// Waits until `FRAMES` more frames than `since` have been shown, at a size other
/// than `old_size` when given.
fn wait(viewer: &ViewerHandle, since: u64, old_size: Option<(u32, u32)>) -> Result<Stats, String> {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let stats = viewer.stats();
        if let Some(err) = &stats.error {
            return Err(err.clone());
        }
        let resized = old_size.is_none_or(|old| stats.size != old);
        // The native viewer must also know where to draw; an empty rectangle
        // would present frames that show nothing.
        let placed = viewer.backend == Backend::Transfer
            || stats.viewport.is_some_and(|(_, _, w, h)| w > 0 && h > 0);
        if stats.frames >= since + FRAMES && resized && placed {
            return Ok(stats);
        }
        if Instant::now() > deadline {
            return Err(format!("timed out: {stats:?}"));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn result_json(result: &Result<Stats, String>) -> serde_json::Value {
    match result {
        Ok(stats) => serde_json::to_value(stats).unwrap_or_default(),
        Err(err) => serde_json::json!({ "error": err }),
    }
}
