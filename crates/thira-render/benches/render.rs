//! Rendering and reading back a 1080p frame. CI runners have no GPU, so there
//! this measures a software adapter: useful for spotting regressions in our own
//! rendering path, not as a playback figure.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use thira_render::{Gpu, TestPattern, Viewport, render_rgba, wgpu};

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;

fn render(c: &mut Criterion) {
    let gpu = Gpu::headless().expect("a GPU or software adapter");
    let mut pattern = TestPattern::new(&gpu);
    let viewport = Viewport {
        x: 0,
        y: 0,
        width: WIDTH,
        height: HEIGHT,
    };
    let mut group = c.benchmark_group("render");
    group.sample_size(20);
    group.bench_function("test_pattern_1080p", |b| {
        b.iter(|| {
            render_rgba(
                &gpu,
                &mut pattern,
                (WIDTH, HEIGHT),
                viewport,
                wgpu::Color::BLACK,
                black_box(0.5),
            )
            .expect("renders")
        })
    });
    group.finish();
}

criterion_group!(benches, render);
criterion_main!(benches);
