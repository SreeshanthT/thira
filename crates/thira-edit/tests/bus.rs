//! Command bus: inverses, atomic transactions, validation and change events.

use std::path::PathBuf;
use std::sync::Arc;

use thira_edit::{Change, Command, CommandBus, EditError, Transaction};
use thira_model::{
    AudioStream, Clip, ClipId, Format, FrameRate, MediaId, MediaKind, MediaRef, ModelError,
    Project, Time, Timeline, TimelineId, Track, TrackId, TrackKind, VideoStream,
};

const RATE: FrameRate = FrameRate::FPS_25;

fn frames(n: i64) -> Time {
    Time::from_frame(n, RATE)
}

fn video_media() -> MediaRef {
    MediaRef {
        id: MediaId::new(),
        name: "a.mp4".into(),
        path: PathBuf::from("a.mp4"),
        kind: MediaKind::Video,
        duration: Some(frames(1000)),
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

struct Fixture {
    bus: CommandBus,
    media: MediaId,
    timeline: TimelineId,
    v1: TrackId,
    a1: TrackId,
    clip: ClipId,
}

/// One timeline with V1 (one clip at 0..100) and A1 (empty).
fn fixture() -> Fixture {
    let mut project = Project::new("Test", Format::hd_1080(RATE));
    let media = video_media();
    let media_id = media.id;
    Arc::make_mut(&mut project.media).insert(media.id, media);
    let clip = Clip::new(media_id, frames(0), frames(0), frames(100));
    let clip_id = clip.id;
    let mut v1 = Track::new(TrackKind::Video, "V1");
    v1.clips.push(clip);
    let a1 = Track::new(TrackKind::Audio, "A1");
    let (v1_id, a1_id) = (v1.id, a1.id);
    let mut timeline = Timeline::new("Main", project.default_format);
    timeline.video_tracks.push(Arc::new(v1));
    timeline.audio_tracks.push(Arc::new(a1));
    let timeline_id = timeline.id;
    project.timelines.push(Arc::new(timeline));
    Fixture {
        bus: CommandBus::new(project).expect("valid fixture"),
        media: media_id,
        timeline: timeline_id,
        v1: v1_id,
        a1: a1_id,
        clip: clip_id,
    }
}

/// One of every command kind, each valid against the fixture.
fn every_command(f: &Fixture) -> Vec<Command> {
    let moved = Clip {
        start: frames(200),
        ..clip_on(f)
    };
    vec![
        Command::SetProjectName {
            name: "Renamed".into(),
        },
        Command::SetDefaultFormat {
            format: Format::hd_1080(FrameRate::FPS_30),
        },
        Command::AddMedia {
            media: video_media(),
        },
        Command::UpdateMedia {
            media: MediaRef {
                name: "renamed.mp4".into(),
                ..media_of(f)
            },
        },
        Command::AddTimeline {
            index: 1,
            timeline: Timeline::new("Second", Format::hd_1080(RATE)),
        },
        Command::UpdateTimeline {
            id: f.timeline,
            name: "Edit".into(),
            format: Format::hd_1080(RATE),
        },
        Command::AddTrack {
            timeline: f.timeline,
            index: 0,
            track: Track::new(TrackKind::Video, "V0"),
        },
        Command::RenameTrack {
            id: f.a1,
            name: "Dialogue".into(),
        },
        Command::RemoveTrack { id: f.a1 },
        Command::InsertClip {
            track: f.v1,
            clip: Clip::new(f.media, frames(500), frames(0), frames(10)),
        },
        Command::UpdateClip {
            track: f.v1,
            clip: moved,
        },
        Command::RemoveClip {
            track: f.v1,
            id: f.clip,
        },
        Command::RemoveTimeline { id: f.timeline },
    ]
}

fn clip_on(f: &Fixture) -> Clip {
    f.bus.project().timelines[0].video_tracks[0].clips[0].clone()
}

fn media_of(f: &Fixture) -> MediaRef {
    f.bus.project().media[&f.media].clone()
}

#[test]
fn every_command_is_undone_exactly_by_its_inverse() {
    let f = fixture();
    for command in every_command(&f) {
        let mut bus = f.bus.clone();
        let original = bus.snapshot();
        let applied = bus
            .apply(Transaction::single("test", command.clone()))
            .unwrap_or_else(|e| panic!("{command:?} failed: {e}"));
        assert_ne!(bus.project(), &original, "{command:?} changed nothing");
        bus.apply(applied.inverse).expect("inverse applies");
        assert_eq!(
            bus.project(),
            &original,
            "inverse of {command:?} did not restore"
        );
    }
}

#[test]
fn whole_transaction_inverts_in_reverse_order() {
    let mut f = fixture();
    let original = f.bus.snapshot();
    let commands = every_command(&f);
    let applied = f
        .bus
        .apply(Transaction::new("everything", commands))
        .expect("applies");
    assert_eq!(applied.inverse.label, "everything");
    f.bus.apply(applied.inverse).expect("inverse applies");
    assert_eq!(f.bus.project(), &original);
}

#[test]
fn failed_command_rolls_back_the_whole_transaction() {
    let mut f = fixture();
    let original = f.bus.snapshot();
    let err = f
        .bus
        .apply(Transaction::new(
            "bad",
            vec![
                Command::SetProjectName {
                    name: "Changed".into(),
                },
                Command::RemoveClip {
                    track: f.v1,
                    id: ClipId::new(),
                },
            ],
        ))
        .expect_err("missing clip");
    assert!(matches!(err, EditError::NotFound { kind: "clip", .. }));
    assert_eq!(f.bus.project(), &original);
    assert_eq!(f.bus.version(), 0);
}

#[test]
fn edits_that_break_invariants_are_rejected() {
    let mut f = fixture();
    let original = f.bus.snapshot();
    // The clip still uses this media.
    let err = f.bus.apply(Transaction::single(
        "rm",
        Command::RemoveMedia { id: f.media },
    ));
    assert_eq!(
        err.map(|_| ()),
        Err(EditError::Invalid(ModelError::MissingMedia {
            clip: f.clip,
            media: f.media
        }))
    );
    // Overlaps the existing 0..100 clip.
    let overlap = Clip::new(f.media, frames(50), frames(0), frames(100));
    let id = overlap.id;
    let err = f.bus.apply(Transaction::single(
        "ins",
        Command::InsertClip {
            track: f.v1,
            clip: overlap,
        },
    ));
    assert_eq!(
        err.map(|_| ()),
        Err(EditError::Invalid(ModelError::Overlap(f.clip, id)))
    );
    assert_eq!(f.bus.project(), &original);
}

#[test]
fn duplicate_and_missing_targets_are_errors() {
    let mut f = fixture();
    let clip = clip_on(&f);
    let dup = f.bus.apply(Transaction::single(
        "dup",
        Command::InsertClip { track: f.a1, clip },
    ));
    assert!(matches!(
        dup,
        Err(EditError::AlreadyExists { kind: "clip", .. })
    ));
    let missing = f.bus.apply(Transaction::single(
        "x",
        Command::RenameTrack {
            id: TrackId::new(),
            name: "x".into(),
        },
    ));
    assert!(matches!(
        missing,
        Err(EditError::NotFound { kind: "track", .. })
    ));
    let bad_index = f.bus.apply(Transaction::single(
        "x",
        Command::AddTrack {
            timeline: f.timeline,
            index: 5,
            track: Track::new(TrackKind::Audio, "A9"),
        },
    ));
    assert_eq!(
        bad_index.map(|_| ()),
        Err(EditError::IndexOutOfRange { index: 5 })
    );
}

#[test]
fn update_clip_keeps_clips_sorted() {
    let mut f = fixture();
    let late = Clip::new(f.media, frames(300), frames(0), frames(10));
    f.bus
        .apply(Transaction::single(
            "ins",
            Command::InsertClip {
                track: f.v1,
                clip: late,
            },
        ))
        .expect("insert");
    // Move the first clip after the second.
    let moved = Clip {
        start: frames(400),
        ..clip_on(&f)
    };
    f.bus
        .apply(Transaction::single(
            "move",
            Command::UpdateClip {
                track: f.v1,
                clip: moved,
            },
        ))
        .expect("move");
    let starts: Vec<Time> = f.bus.project().timelines[0].video_tracks[0]
        .clips
        .iter()
        .map(|c| c.start)
        .collect();
    assert_eq!(starts, [frames(300), frames(400)]);
}

#[test]
fn change_events_list_touched_objects_and_count_versions() {
    let mut f = fixture();
    let clip = Clip::new(f.media, frames(200), frames(0), frames(10));
    let new_clip = clip.id;
    let applied = f
        .bus
        .apply(Transaction::new(
            "two",
            vec![
                Command::InsertClip { track: f.v1, clip },
                Command::RenameTrack {
                    id: f.a1,
                    name: "Dialogue".into(),
                },
            ],
        ))
        .expect("applies");
    assert_eq!(applied.event.version, 1);
    let mut expected = vec![
        Change::Track(f.v1),
        Change::Track(f.a1),
        Change::Clip(new_clip),
    ];
    expected.sort();
    assert_eq!(applied.event.changes, expected);

    let again = f.bus.apply(Transaction::single(
        "rename",
        Command::RenameTrack {
            id: f.a1,
            name: "A".into(),
        },
    ));
    assert_eq!(again.expect("applies").event.version, 2);
    let empty = f.bus.apply(Transaction::default()).expect("empty is fine");
    assert_eq!(empty.event.version, 2);
    assert!(empty.event.changes.is_empty());
}

#[test]
fn snapshots_are_isolated_from_later_edits() {
    let mut f = fixture();
    let snapshot = f.bus.snapshot();
    f.bus
        .apply(Transaction::single(
            "rm",
            Command::RemoveClip {
                track: f.v1,
                id: f.clip,
            },
        ))
        .expect("remove");
    assert_eq!(snapshot.timelines[0].video_tracks[0].clips.len(), 1);
    assert!(
        f.bus.project().timelines[0].video_tracks[0]
            .clips
            .is_empty()
    );
    // The untouched audio track is still shared with the snapshot.
    assert!(Arc::ptr_eq(
        &snapshot.timelines[0].audio_tracks[0],
        &f.bus.project().timelines[0].audio_tracks[0]
    ));
}

#[test]
fn transactions_and_events_serialize_for_ipc() {
    let f = fixture();
    let tx = Transaction::new("all", every_command(&f));
    let json = serde_json::to_string(&tx).expect("serializes");
    assert!(json.contains(r#""type":"insert_clip""#), "{json}");
    let back: Transaction = serde_json::from_str(&json).expect("deserializes");
    assert_eq!(back, tx);

    let event = serde_json::to_string(&Change::Track(f.v1)).expect("serializes");
    assert_eq!(event, format!(r#"{{"kind":"track","id":"{}"}}"#, f.v1));
}

#[test]
fn bus_rejects_an_invalid_starting_project() {
    let mut project = Project::new("Bad", Format::hd_1080(RATE));
    project.default_format.width = 0;
    assert_eq!(
        CommandBus::new(project).map(|_| ()),
        Err(EditError::Invalid(ModelError::InvalidFormat))
    );
}
