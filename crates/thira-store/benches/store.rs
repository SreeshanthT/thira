//! Saving and opening a project with 1,000 clips (PRD G2: saves must stay cheap
//! enough to autosave often).

use std::path::PathBuf;
use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};
use thira_model::{
    Clip, Format, FrameRate, MediaId, MediaKind, MediaRef, Project, Time, Timeline, Track,
    TrackKind, VideoStream,
};
use thira_store::{load, save};

const RATE: FrameRate = FrameRate::FPS_25;
const CLIPS: i64 = 1_000;

fn frames(n: i64) -> Time {
    Time::from_frame(n, RATE)
}

fn project() -> Project {
    let mut project = Project::new("Benchmark", Format::hd_1080(RATE));
    let media = MediaRef {
        id: MediaId::new(),
        name: "a.mp4".into(),
        path: PathBuf::from("a.mp4"),
        kind: MediaKind::Video,
        duration: Some(frames(CLIPS * 100)),
        video: Some(VideoStream {
            width: 1920,
            height: 1080,
            frame_rate: Some(RATE),
        }),
        audio: None,
    };
    let media_id = media.id;
    Arc::make_mut(&mut project.media).insert(media.id, media);
    let mut track = Track::new(TrackKind::Video, "V1");
    for i in 0..CLIPS {
        track
            .clips
            .push(Clip::new(media_id, frames(i * 100), frames(0), frames(100)));
    }
    let mut timeline = Timeline::new("Main", project.default_format);
    timeline.video_tracks.push(Arc::new(track));
    project.timelines.push(Arc::new(timeline));
    project
}

fn store(c: &mut Criterion) {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("benchmark.thira");
    let project = project();
    save(&project, &path).expect("first save");

    // Each save syncs to disk, so fewer samples keep the run short.
    let mut group = c.benchmark_group("store");
    group.sample_size(20);
    group.bench_function("save_1000_clips", |b| {
        b.iter(|| save(&project, &path).expect("save"))
    });
    group.bench_function("load_1000_clips", |b| b.iter(|| load(&path).expect("load")));
    group.finish();
}

criterion_group!(benches, store);
criterion_main!(benches);
