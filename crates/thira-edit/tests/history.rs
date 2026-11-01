//! Undo/redo history, including the 1,000 random operation round-trip (issue #8).

use std::path::PathBuf;
use std::sync::Arc;

use thira_edit::{Command, Editor, Transaction};
use thira_model::{
    AudioStream, Clip, Format, FrameRate, MediaId, MediaKind, MediaRef, Project, Time, Timeline,
    Track, TrackKind, VideoStream,
};

const RATE: FrameRate = FrameRate::FPS_25;

fn frames(n: i64) -> Time {
    Time::from_frame(n, RATE)
}

fn media() -> MediaRef {
    MediaRef {
        id: MediaId::new(),
        name: "clip.mov".into(),
        path: PathBuf::from("clip.mov"),
        kind: MediaKind::Video,
        duration: Some(frames(10_000)),
        video: Some(VideoStream {
            width: 1920,
            height: 1080,
            frame_rate: Some(RATE),
        }),
        audio: Some(AudioStream {
            channels: 2,
            sample_rate: 48_000,
        }),
    }
}

fn editor() -> Editor {
    let mut project = Project::new("History", Format::hd_1080(RATE));
    let m = media();
    Arc::make_mut(&mut project.media).insert(m.id, m);
    let mut timeline = Timeline::new("Main", project.default_format);
    timeline
        .video_tracks
        .push(Arc::new(Track::new(TrackKind::Video, "V1")));
    timeline
        .audio_tracks
        .push(Arc::new(Track::new(TrackKind::Audio, "A1")));
    project.timelines.push(Arc::new(timeline));
    Editor::new(project).expect("valid")
}

fn rename(name: &str) -> Transaction {
    Transaction::single(
        format!("Rename to {name}"),
        Command::SetProjectName { name: name.into() },
    )
}

/// Deterministic pseudo-random numbers (SplitMix64), so failures reproduce.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % n as u64).expect("fits")
    }

    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + i64::try_from(self.next() % u64::try_from(hi - lo).expect("hi > lo")).expect("fits")
    }
}

/// A random edit against the current project. Some are invalid (overlaps, removing
/// media in use) on purpose: they must fail without touching the project or history.
fn random_edit(rng: &mut Rng, project: &Project, step: usize) -> Transaction {
    let timeline = &project.timelines[0];
    let tracks: Vec<&Arc<Track>> = timeline.tracks().collect();
    let track = tracks[rng.below(tracks.len())];
    let media_ids: Vec<MediaId> = project.media.keys().copied().collect();
    let some_media = media_ids[rng.below(media_ids.len())];

    let command = match rng.below(9) {
        0..=2 => {
            let start =
                track.clips.last().map_or(Time::ZERO, Clip::end) + frames(rng.range(-20, 50));
            let start = start.max(Time::ZERO);
            let clip = Clip::new(
                some_media,
                start,
                frames(rng.range(0, 100)),
                frames(rng.range(1, 200)),
            );
            Command::InsertClip {
                track: track.id,
                clip,
            }
        }
        3 if !track.clips.is_empty() => {
            let clip = &track.clips[rng.below(track.clips.len())];
            Command::RemoveClip {
                track: track.id,
                id: clip.id,
            }
        }
        4 if !track.clips.is_empty() => {
            let clip = &track.clips[rng.below(track.clips.len())];
            let shorter = (clip.duration - frames(rng.range(0, 5))).max(frames(1));
            Command::UpdateClip {
                track: track.id,
                clip: Clip {
                    duration: shorter,
                    enabled: !clip.enabled,
                    ..clip.clone()
                },
            }
        }
        5 => Command::RenameTrack {
            id: track.id,
            name: format!("T{step}"),
        },
        6 => {
            let kind = if rng.below(2) == 0 {
                TrackKind::Video
            } else {
                TrackKind::Audio
            };
            let index = rng.below(3);
            Command::AddTrack {
                timeline: timeline.id,
                index,
                track: Track::new(kind, format!("N{step}")),
            }
        }
        7 => {
            if rng.below(2) == 0 {
                Command::AddMedia { media: media() }
            } else {
                Command::RemoveMedia { id: some_media }
            }
        }
        _ => Command::SetProjectName {
            name: format!("P{step}"),
        },
    };
    Transaction::single(format!("step {step}"), command)
}

#[test]
fn a_thousand_random_edits_undo_and_redo_exactly() {
    for seed in [1, 2, 3] {
        let mut rng = Rng(seed);
        let mut editor = editor();
        // states[i] is the project after i successful steps.
        let mut states = vec![editor.snapshot()];
        let mut attempts = 0;
        while states.len() <= 1000 {
            attempts += 1;
            assert!(attempts < 20_000, "too many rejected edits");
            let before = editor.snapshot();
            let edit = random_edit(&mut rng, editor.project(), states.len());
            if editor.apply(edit).is_ok() {
                states.push(editor.snapshot());
            } else {
                assert_eq!(editor.project(), &before, "failed edit changed the project");
                assert_eq!(
                    editor.undo_depth(),
                    states.len() - 1,
                    "failed edit changed history"
                );
            }
        }
        assert_eq!(editor.undo_depth(), 1000);

        for expected in states.iter().rev().skip(1) {
            editor.undo().expect("undo applies").expect("step to undo");
            assert_eq!(editor.project(), expected, "seed {seed}");
        }
        assert_eq!(editor.undo().expect("ok"), None);

        for expected in states.iter().skip(1) {
            editor.redo().expect("redo applies").expect("step to redo");
            assert_eq!(editor.project(), expected, "seed {seed}");
        }
        assert_eq!(editor.redo().expect("ok"), None);
    }
}

#[test]
fn new_edit_after_undo_clears_redo() {
    let mut editor = editor();
    editor.apply(rename("A")).expect("ok");
    editor.apply(rename("B")).expect("ok");
    editor.undo().expect("ok");
    assert_eq!(editor.redo_label(), Some("Rename to B"));
    editor.apply(rename("C")).expect("ok");
    assert_eq!(editor.redo_depth(), 0);
    assert_eq!(editor.undo_label(), Some("Rename to C"));
    editor.undo().expect("ok");
    assert_eq!(editor.project().name, "A");
}

#[test]
fn group_is_one_undo_step() {
    let mut editor = editor();
    let original = editor.snapshot();
    editor.begin_group("Drag");
    for name in ["x", "y", "z"] {
        editor.apply(rename(name)).expect("ok");
    }
    editor.begin_group("Inner");
    editor.apply(rename("w")).expect("ok");
    editor.end_group();
    assert_eq!(editor.undo_depth(), 0, "still open");
    editor.end_group();

    assert_eq!(editor.undo_depth(), 1);
    assert_eq!(editor.undo_label(), Some("Drag"));
    editor.undo().expect("ok");
    assert_eq!(editor.project(), &original);
    editor.redo().expect("ok");
    assert_eq!(editor.project().name, "w");
}

#[test]
fn undo_closes_an_open_group_and_empty_groups_are_dropped() {
    let mut editor = editor();
    editor.begin_group("Empty");
    editor.end_group();
    assert_eq!(editor.undo_depth(), 0);

    editor.begin_group("Open");
    editor.apply(rename("x")).expect("ok");
    editor.undo().expect("ok").expect("undid the open group");
    assert_eq!(editor.project().name, "History");
    // A stray end_group is harmless.
    editor.end_group();
    assert_eq!(editor.redo_label(), Some("Open"));
}

#[test]
fn undo_and_redo_bump_the_version() {
    let mut editor = editor();
    editor.apply(rename("A")).expect("ok");
    assert_eq!(editor.undo().expect("ok").map(|e| e.version), Some(2));
    assert_eq!(editor.redo().expect("ok").map(|e| e.version), Some(3));
    assert_eq!(editor.version(), 3);
}
