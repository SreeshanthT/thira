//! Renders the test pattern offscreen and checks pixels. Runs on a software adapter
//! when there is no GPU (WARP on Windows, llvmpipe/lavapipe on Linux).

use std::sync::{Mutex, OnceLock};

use thira_render::{Gpu, TestPattern, Viewport, render_rgba, wgpu};

/// One device for all tests; creating many in parallel is slow on software adapters.
fn gpu() -> &'static Mutex<(Gpu, TestPattern)> {
    static GPU: OnceLock<Mutex<(Gpu, TestPattern)>> = OnceLock::new();
    GPU.get_or_init(|| {
        let gpu = Gpu::headless().expect("a GPU or software adapter");
        eprintln!("rendering on {:?}", gpu.summary());
        let pattern = TestPattern::new(&gpu);
        Mutex::new((gpu, pattern))
    })
}

struct Frame {
    width: u32,
    pixels: Vec<u8>,
}

impl Frame {
    fn at(&self, x: u32, y: u32) -> [u8; 3] {
        let i = ((y * self.width + x) * 4) as usize;
        [self.pixels[i], self.pixels[i + 1], self.pixels[i + 2]]
    }
}

fn render(size: (u32, u32), viewport: Viewport, background: wgpu::Color, t: f32) -> Frame {
    let mut guard = gpu().lock().unwrap_or_else(|e| e.into_inner());
    let (gpu, pattern) = &mut *guard;
    let pixels = render_rgba(gpu, pattern, size, viewport, background, t).expect("renders");
    assert_eq!(pixels.len(), (size.0 * size.1 * 4) as usize);
    Frame {
        width: size.0,
        pixels,
    }
}

fn assert_close(actual: [u8; 3], expected: [u8; 3], what: &str) {
    let close = actual.iter().zip(expected).all(|(a, e)| a.abs_diff(e) <= 2);
    assert!(close, "{what}: got {actual:?}, expected {expected:?}");
}

const L: u8 = 191; // 75% of 255
const BARS: [[u8; 3]; 8] = [
    [L, L, L], // white
    [L, L, 0], // yellow
    [0, L, L], // cyan
    [0, L, 0], // green
    [L, 0, L], // magenta
    [L, 0, 0], // red
    [0, 0, L], // blue
    [0, 0, 0], // black
];

#[test]
fn draws_eight_color_bars() {
    let frame = render(
        (640, 360),
        Viewport::full(640, 360),
        wgpu::Color::BLACK,
        0.0,
    );
    for (i, expected) in BARS.iter().enumerate() {
        let x = i as u32 * 80 + 40;
        assert_close(frame.at(x, 10), *expected, &format!("bar {i}"));
    }
}

#[test]
fn box_moves_over_time() {
    // The box is 20% of the height (72 px) and vertically centered (rows 144..216).
    let start = render(
        (640, 360),
        Viewport::full(640, 360),
        wgpu::Color::BLACK,
        0.0,
    );
    assert_close(start.at(36, 180), [255, 255, 255], "box at t=0");
    assert_close(start.at(320, 180), BARS[4], "middle at t=0 is magenta");

    // Halfway through its 4-second sweep the box spans x 284..356.
    let later = render(
        (640, 360),
        Viewport::full(640, 360),
        wgpu::Color::BLACK,
        2.0,
    );
    assert_close(later.at(320, 180), [255, 255, 255], "box at t=2");
    assert_close(later.at(36, 180), BARS[0], "left edge at t=2 is white bar");
}

#[test]
fn draws_only_inside_the_viewport() {
    let background = wgpu::Color {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    let viewport = Viewport {
        x: 100,
        y: 50,
        width: 200,
        height: 100,
    };
    let frame = render((400, 300), viewport, background, 0.0);
    assert_close(frame.at(50, 25), [0, 0, 255], "outside, top-left");
    assert_close(frame.at(350, 250), [0, 0, 255], "outside, bottom-right");
    // Inside: bars are 25 px wide in a 200 px viewport, measured from x = 100.
    assert_close(frame.at(100 + 12, 55), BARS[0], "first bar");
    assert_close(frame.at(100 + 7 * 25 + 12, 55), BARS[7], "last bar");
}

#[test]
fn viewport_outside_the_target_draws_only_background() {
    let viewport = Viewport {
        x: 500,
        y: 0,
        width: 100,
        height: 100,
    };
    let frame = render((400, 300), viewport, wgpu::Color::BLACK, 0.0);
    assert!(
        frame
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| p[..3] == [0, 0, 0])
    );
}

#[test]
fn rejects_invalid_sizes() {
    let mut guard = gpu().lock().unwrap_or_else(|e| e.into_inner());
    let (gpu, pattern) = &mut *guard;
    let result = render_rgba(
        gpu,
        pattern,
        (0, 10),
        Viewport::full(0, 10),
        wgpu::Color::BLACK,
        0.0,
    );
    assert!(result.is_err());
}
