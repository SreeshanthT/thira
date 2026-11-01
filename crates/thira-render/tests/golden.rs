//! Golden frames for the test pattern (issue #12). CI renders these on software
//! adapters (WARP on Windows, lavapipe on Linux, paravirtual Metal on macOS) and
//! compares them with PNGs rendered on a real GPU.
//!
//! Sizes are chosen so every edge lands on a pixel boundary (40 px bars, a 36 px
//! box at x = 0 or 142), so rasterizers have no edge to disagree about.
//! Regenerate with `THIRA_UPDATE_GOLDEN=1 cargo test -p thira-render --test golden`.

use std::path::{Path, PathBuf};

use thira_golden::{Image, Tolerance, assert_golden};
use thira_render::{Gpu, TestPattern, Viewport, render_rgba, wgpu};

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn check(name: &str, size: (u32, u32), viewport: Viewport, background: wgpu::Color, t: f32) {
    let gpu = Gpu::headless().expect("a GPU or software adapter");
    let mut pattern = TestPattern::new(&gpu);
    let rgba = render_rgba(&gpu, &mut pattern, size, viewport, background, t).expect("renders");
    let image = Image {
        width: size.0,
        height: size.1,
        rgba,
    };
    assert_golden(&golden_dir(), name, &image, Tolerance::ROUNDING);
}

#[test]
fn pattern_at_start() {
    check(
        "pattern-t0",
        (320, 180),
        Viewport::full(320, 180),
        wgpu::Color::BLACK,
        0.0,
    );
}

#[test]
fn pattern_halfway() {
    check(
        "pattern-t2",
        (320, 180),
        Viewport::full(320, 180),
        wgpu::Color::BLACK,
        2.0,
    );
}

#[test]
fn pattern_in_a_viewport() {
    // The app's background color around a viewer inset by (40, 30).
    let background = wgpu::Color {
        r: 0.086,
        g: 0.090,
        b: 0.102,
        a: 1.0,
    };
    let viewport = Viewport {
        x: 40,
        y: 30,
        width: 320,
        height: 180,
    };
    check("pattern-viewport", (400, 240), viewport, background, 0.0);
}
