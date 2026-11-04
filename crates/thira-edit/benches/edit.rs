//! One edit and its undo on a large track: the cost of every interaction.

use std::hint::black_box;
use std::path::PathBuf;
use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};
use thira_edit::{Command, Editor, Transaction};
use thira_model::{
    Clip, Format, FrameRate, MediaId, MediaKind, MediaRef, Project, Time, Timeline, Track, TrackId,
    TrackKind, VideoStream,
};

const RATE: FrameRate = FrameRate::FPS_25;
const CLIPS: i64 = 1_000;

fn frames(n: i64) -> Time {
    Time::from_frame(n, RATE)
}

/// A project with one video track of 1,000 consecutive clips.
fn project() -> (Project, MediaId, TrackId) {
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
    let track_id = track.id;
    let mut timeline = Timeline::new("Main", project.default_format);
    timeline.video_tracks.push(Arc::new(track));
    project.timelines.push(Arc::new(timeline));
    (project, media_id, track_id)
}

fn insert_and_undo(c: &mut Criterion) {
    let (project, media, track) = project();
    let mut editor = Editor::new(project).expect("valid project");
    let clip = Clip::new(media, frames(CLIPS * 100), frames(0), frames(100));
    c.bench_function("edit/insert_and_undo_on_1000_clips", |b| {
        b.iter(|| {
            let insert = Command::InsertClip {
                track,
                clip: clip.clone(),
            };
            let changed = editor
                .apply(Transaction::single("Insert", insert))
                .expect("insert");
            black_box(changed);
            black_box(editor.undo().expect("undo"));
        })
    });
}

criterion_group!(benches, insert_and_undo);
criterion_main!(benches);
