//! Timecode conversions, which the timeline ruler and viewer run constantly.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use thira_time::{FrameRate, Timecode};

fn timecode(c: &mut Criterion) {
    let rate = FrameRate::FPS_29_97;
    // About 10 hours in, past many drop-frame adjustments.
    let frame = 1_078_920;
    c.bench_function("timecode/from_frame_drop_frame", |b| {
        b.iter(|| Timecode::from_frame(black_box(frame), rate, true))
    });
    let text = Timecode::from_frame(frame, rate, true)
        .expect("valid frame")
        .to_string();
    c.bench_function("timecode/parse_drop_frame", |b| {
        b.iter(|| Timecode::parse(black_box(&text), rate))
    });
}

criterion_group!(benches, timecode);
criterion_main!(benches);
