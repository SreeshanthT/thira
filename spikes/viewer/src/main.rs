//! Viewer spike for issue #2. Not shipped.
//!
//! `THIRA_SPIKE_MODE=transfer` (default): wgpu renders offscreen, each frame is read
//! back and returned to the webview as binary IPC, which draws it with WebGL.
//! `THIRA_SPIKE_MODE=native`: wgpu presents straight to the window; the webview is
//! transparent over it.
//!
//! Results are written as JSON to `THIRA_SPIKE_OUT` (default `spike-result.json`),
//! then the app exits.

mod gpu;

use std::sync::Mutex;
use std::time::{Duration, Instant};

use gpu::Gpu;
use tauri::{Manager, State, ipc::Response};

struct Spike {
    mode: String,
    frames: usize,
    out: String,
    gpu: Mutex<Option<Gpu>>,
}

fn env(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_owned())
}

#[tauri::command]
fn config(spike: State<'_, Spike>) -> serde_json::Value {
    serde_json::json!({ "mode": spike.mode, "frames": spike.frames })
}

/// One frame for the transfer mode: 8 bytes of render+readback time (µs, f64 LE), then RGBA.
#[tauri::command]
fn frame(spike: State<'_, Spike>, width: u32, height: u32, t: f32) -> Result<Response, String> {
    let mut slot = spike.gpu.lock().map_err(|e| e.to_string())?;
    if slot.is_none() {
        *slot = Some(Gpu::new(wgpu::Instance::default(), None)?);
    }
    let gpu = slot.as_mut().ok_or("no gpu")?;
    let start = Instant::now();
    let pixels = gpu.render_to_bytes(t, width, height);
    let micros = start.elapsed().as_secs_f64() * 1e6;
    let mut body = Vec::with_capacity(8 + pixels.len());
    body.extend_from_slice(&micros.to_le_bytes());
    body.extend_from_slice(&pixels);
    Ok(Response::new(body))
}

/// Receives the webview's measurements, adds system details, writes them and exits.
#[tauri::command]
fn report(app: tauri::AppHandle, spike: State<'_, Spike>, results: serde_json::Value) {
    let adapter = spike.gpu.lock().ok().and_then(|g| g.as_ref().map(Gpu::adapter_summary));
    finish(&app, &spike.out, &spike.mode, adapter, results);
}

fn finish(
    app: &tauri::AppHandle,
    out: &str,
    mode: &str,
    adapter: Option<serde_json::Value>,
    results: serde_json::Value,
) {
    let doc = serde_json::json!({
        "mode": mode,
        "os": std::env::consts::OS,
        "adapter": adapter,
        "results": results,
    });
    let text = serde_json::to_string_pretty(&doc).unwrap_or_default();
    println!("{text}");
    if let Err(e) = std::fs::write(out, &text) {
        eprintln!("could not write {out}: {e}");
    }
    app.exit(0);
}

fn stats(mut ms: Vec<f64>) -> serde_json::Value {
    ms.sort_by(f64::total_cmp);
    let pick = |q: f64| ms[((ms.len() - 1) as f64 * q).round() as usize];
    let mean = ms.iter().sum::<f64>() / ms.len() as f64;
    serde_json::json!({
        "frames": ms.len(), "median_ms": pick(0.5), "p95_ms": pick(0.95),
        "max_ms": pick(1.0), "fps_at_mean": 1000.0 / mean,
    })
}

/// Native mode: render and present `frames` frames straight to the window.
///
/// The surface must be created on the main thread (macOS requires NSView access
/// there); rendering and presenting then run on this worker thread.
fn run_native(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    frames: usize,
    out: String,
) {
    let mut gpu = match Gpu::new(instance, Some(&surface)) {
        Ok(g) => g,
        Err(e) => return finish(&app, &out, "native", None, serde_json::json!({ "error": e })),
    };
    let size = window.inner_size().unwrap_or(tauri::PhysicalSize::new(1280, 720));
    let caps = surface.get_capabilities(&gpu.adapter);
    let format = caps.formats.first().copied().unwrap_or(wgpu::TextureFormat::Bgra8Unorm);
    let mut results = serde_json::Map::new();
    for (name, mode) in [("fifo", wgpu::PresentMode::Fifo), ("immediate", wgpu::PresentMode::Immediate)] {
        if !caps.present_modes.contains(&mode) {
            results.insert(name.into(), serde_json::json!("unsupported"));
            continue;
        }
        surface.configure(
            &gpu.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width: size.width.max(1),
                height: size.height.max(1),
                present_mode: mode,
                desired_maximum_frame_latency: 2,
                alpha_mode: caps.alpha_modes[0],
                view_formats: vec![],
                color_space: wgpu::SurfaceColorSpace::Auto,
            },
        );
        let started = Instant::now();
        let mut times = Vec::with_capacity(frames);
        for _ in 0..frames {
            let t0 = Instant::now();
            let frame = match surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
                other => {
                    results.insert(name.into(), serde_json::json!({ "error": format!("{other:?}") }));
                    break;
                }
            };
            let view = frame.texture.create_view(&Default::default());
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            gpu.draw(&mut encoder, &view, format, started.elapsed().as_secs_f32(), size.width, size.height);
            gpu.queue.submit([encoder.finish()]);
            gpu.queue.present(frame);
            times.push(t0.elapsed().as_secs_f64() * 1e3);
        }
        if !times.is_empty() {
            results.insert(name.into(), stats(times));
        }
    }
    results.insert("surface".into(), serde_json::json!({ "width": size.width, "height": size.height, "format": format!("{format:?}") }));
    let adapter = Some(gpu.adapter_summary());
    // Leave the last frame visible briefly so a screenshot can check the overlay.
    std::thread::sleep(Duration::from_millis(
        env("THIRA_SPIKE_HOLD_MS", "0").parse().unwrap_or(0),
    ));
    finish(&app, &out, "native", adapter, serde_json::Value::Object(results));
}

fn main() {
    let spike = Spike {
        mode: env("THIRA_SPIKE_MODE", "transfer"),
        frames: env("THIRA_SPIKE_FRAMES", "300").parse().unwrap_or(300),
        out: env("THIRA_SPIKE_OUT", "spike-result.json"),
        gpu: Mutex::new(None),
    };
    let native = spike.mode == "native";
    let (frames, out) = (spike.frames, spike.out.clone());
    tauri::Builder::default()
        .manage(spike)
        .invoke_handler(tauri::generate_handler![config, frame, report])
        .setup(move |app| {
            if native {
                let window = app.get_webview_window("main").ok_or("no main window")?;
                let handle = app.handle().clone();
                // `setup` runs on the main thread, where the surface has to be created.
                let instance = wgpu::Instance::default();
                let surface = instance.create_surface(window.clone())?;
                std::thread::spawn(move || {
                    // Let the webview paint its transparent overlay first.
                    std::thread::sleep(Duration::from_millis(1500));
                    run_native(handle, window, instance, surface, frames, out);
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| panic!("spike failed to start: {e}"));
}
