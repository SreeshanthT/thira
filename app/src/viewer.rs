//! The viewer (ADR 0003): a render thread that either presents to the window
//! surface under the transparent webview (native) or renders frames the UI pulls
//! over IPC and draws on a canvas (transfer).

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use thira_render::{
    Gpu, Presented, RenderError, SurfaceViewer, TestPattern, Viewport, render_rgba, wgpu,
};

/// Matches `--bg` in the UI stylesheet, so viewer gaps blend with the panels.
const BACKGROUND: wgpu::Color = wgpu::Color {
    r: 0.086,
    g: 0.090,
    b: 0.102,
    a: 1.0,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    Native,
    Transfer,
}

impl Backend {
    /// `THIRA_VIEWER=native|transfer` overrides the per-OS default: native on Windows
    /// and macOS; transfer on Linux until #74 confirms the native surface there.
    pub fn choose() -> Self {
        match std::env::var("THIRA_VIEWER").as_deref() {
            Ok("native") => Self::Native,
            Ok("transfer") => Self::Transfer,
            _ if cfg!(target_os = "linux") => Self::Transfer,
            _ => Self::Native,
        }
    }
}

/// Counters for diagnostics and the smoke test.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Stats {
    pub frames: u64,
    /// Physical size of the last frame: the surface (native) or the requested frame (transfer).
    pub size: (u32, u32),
    /// Last viewer rectangle from the UI (native backend), as `(x, y, width, height)`.
    pub viewport: Option<(u32, u32, u32, u32)>,
    pub adapter: Option<String>,
    pub error: Option<String>,
}

enum Message {
    Resize(u32, u32),
    Viewport(Option<Viewport>),
    Frame {
        width: u32,
        height: u32,
        reply: Sender<Result<Vec<u8>, RenderError>>,
    },
}

/// Handle kept in Tauri state; cheap to clone.
#[derive(Clone)]
pub struct ViewerHandle {
    pub backend: Backend,
    tx: Sender<Message>,
    pub stats: Arc<Mutex<Stats>>,
}

impl ViewerHandle {
    /// Starts the native backend. `surface` must have been created on the main thread.
    pub fn native(
        instance: wgpu::Instance,
        surface: wgpu::Surface<'static>,
        size: (u32, u32),
    ) -> Self {
        Self::spawn(Backend::Native, move |rx, stats| {
            match SurfaceViewer::new(instance, surface, size.0, size.1) {
                Ok(viewer) => run_native(viewer, &rx, &stats),
                Err(err) => record_error(&stats, &err),
            }
        })
    }

    /// Starts the transfer backend: renders offscreen on request.
    pub fn transfer() -> Self {
        Self::spawn(Backend::Transfer, |rx, stats| match Gpu::headless() {
            Ok(gpu) => run_transfer(&gpu, &rx, &stats),
            Err(err) => record_error(&stats, &err),
        })
    }

    fn spawn(
        backend: Backend,
        body: impl FnOnce(Receiver<Message>, Arc<Mutex<Stats>>) + Send + 'static,
    ) -> Self {
        let (tx, rx) = mpsc::channel();
        let stats = Arc::new(Mutex::new(Stats::default()));
        let thread_stats = Arc::clone(&stats);
        thread::Builder::new()
            .name("thira-render".into())
            .spawn(move || body(rx, thread_stats))
            .expect("failed to spawn the render thread");
        Self { backend, tx, stats }
    }

    pub fn resize(&self, width: u32, height: u32) {
        let _ = self.tx.send(Message::Resize(width, height));
    }

    pub fn set_viewport(&self, viewport: Option<Viewport>) {
        let _ = self.tx.send(Message::Viewport(viewport));
    }

    /// Renders one frame for the transfer backend; blocks until it is ready.
    pub fn frame(&self, width: u32, height: u32) -> Result<Vec<u8>, String> {
        let (reply, result) = mpsc::channel();
        self.tx
            .send(Message::Frame {
                width,
                height,
                reply,
            })
            .map_err(|_| "the render thread has stopped".to_owned())?;
        result
            .recv()
            .map_err(|_| "the render thread has stopped".to_owned())?
            .map_err(|e| e.to_string())
    }

    pub fn stats(&self) -> Stats {
        self.stats.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

fn record_error(stats: &Mutex<Stats>, err: &RenderError) {
    eprintln!("viewer: {err}");
    if let Ok(mut s) = stats.lock() {
        s.error = Some(err.to_string());
    }
}

/// Applies one UI or window message to the native viewer.
fn apply(viewer: &mut SurfaceViewer, stats: &Mutex<Stats>, message: Message) {
    match message {
        Message::Resize(w, h) => viewer.resize(w, h),
        Message::Viewport(v) => {
            viewer.set_viewport(v);
            if let Ok(mut s) = stats.lock() {
                s.viewport = v.map(|v| (v.x, v.y, v.width, v.height));
            }
        }
        Message::Frame { reply, .. } => {
            let _ = reply.send(Err(RenderError::Surface(
                "frames are presented natively".into(),
            )));
        }
    }
}

fn run_native(mut viewer: SurfaceViewer, rx: &Receiver<Message>, stats: &Mutex<Stats>) {
    if let Ok(mut s) = stats.lock() {
        s.adapter = Some(format!("{:?}", viewer.summary()));
    }
    let start = Instant::now();
    loop {
        // Apply everything the UI and window sent since the last frame.
        loop {
            match rx.try_recv() {
                Ok(message) => apply(&mut viewer, stats, message),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => return,
            }
        }
        // The test pattern animates, so draw continuously; Fifo paces this to vsync.
        match viewer.render(start.elapsed().as_secs_f32(), BACKGROUND) {
            Ok(Presented::Frame) => {
                if let Ok(mut s) = stats.lock() {
                    s.frames += 1;
                    s.size = viewer.size();
                }
            }
            // Hidden or minimized: wait for a message instead of spinning.
            Ok(Presented::Skipped) => match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(message) => apply(&mut viewer, stats, message),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            },
            Err(err) => {
                record_error(stats, &err);
                return;
            }
        }
    }
}

fn run_transfer(gpu: &Gpu, rx: &Receiver<Message>, stats: &Mutex<Stats>) {
    if let Ok(mut s) = stats.lock() {
        s.adapter = Some(format!("{:?}", gpu.summary()));
    }
    let mut pattern = TestPattern::new(gpu);
    let start = Instant::now();
    for message in rx {
        if let Message::Frame {
            width,
            height,
            reply,
        } = message
        {
            let t = start.elapsed().as_secs_f32();
            let frame = render_rgba(
                gpu,
                &mut pattern,
                (width, height),
                Viewport::full(width, height),
                BACKGROUND,
                t,
            );
            if frame.is_ok()
                && let Ok(mut s) = stats.lock()
            {
                s.frames += 1;
                s.size = (width, height);
            }
            let _ = reply.send(frame);
        }
        // Resize and viewport do not apply: the UI sizes its canvas itself.
    }
}
