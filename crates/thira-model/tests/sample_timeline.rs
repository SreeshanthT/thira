//! Builds a sample project and checks validation, serialization and snapshot sharing.

use std::path::PathBuf;
use std::sync::Arc;

use thira_model::{
    AudioStream, Clip, Format, FrameRate, MediaId, MediaKind, MediaRef, ModelError, Project, Time,
    Timeline, Track, TrackKind, VideoStream,
};

const RATE: FrameRate = FrameRate::FPS_25;

fn frames(n: i64) -> Time {
    Time::from_frame(n, RATE)
}

fn media(name: &str, kind: MediaKind, seconds: Option<i64>) -> MediaRef {
    let video = (kind != MediaKind::Audio).then_some(VideoStream {
        width: 1920,
        height: 1080,
        frame_rate: (kind == MediaKind::Video).then_some(RATE),
    });
    let audio = (kind != MediaKind::Image).then_some(AudioStream {
        channels: 2,
        sample_rate: 48_000,
    });
    MediaRef {
        id: MediaId::new(),
        name: name.into(),
        path: PathBuf::from(format!("media/{name}")),
        kind,
        duration: seconds.map(|s| frames(s * 25)),
        video,
        audio,
    }
}

struct Sample {
    project: Project,
    interview: MediaId,
    music: MediaId,
}

/// Two video tracks (interview + logo overlay) and two audio tracks (dialogue + music).
fn sample() -> Sample {
    let mut project = Project::new("Sample", Format::hd_1080(RATE));
    let interview = media("interview.mp4", MediaKind::Video, Some(60));
    let music = media("music.wav", MediaKind::Audio, Some(120));
    let logo = media("logo.png", MediaKind::Image, None);
    let ids = (interview.id, music.id, logo.id);
    Arc::make_mut(&mut project.media).extend([interview, music, logo].map(|m| (m.id, m)));

    let mut v1 = Track::new(TrackKind::Video, "V1");
    v1.clips = vec![
        Clip::new(ids.0, frames(0), frames(250), frames(100)),
        Clip::new(ids.0, frames(100), frames(500), frames(150)),
    ];
    let mut v2 = Track::new(TrackKind::Video, "V2");
    // Stills can be any length.
    v2.clips = vec![Clip::new(ids.2, frames(25), Time::ZERO, frames(10_000))];
    let mut a1 = Track::new(TrackKind::Audio, "A1");
    a1.clips = v1
        .clips
        .iter()
        .map(|c| Clip::new(c.media, c.start, c.source_in, c.duration))
        .collect();
    let mut a2 = Track::new(TrackKind::Audio, "A2");
    a2.clips = vec![Clip::new(ids.1, frames(0), frames(0), frames(250))];

    let mut timeline = Timeline::new("Main", project.default_format);
    timeline.video_tracks = vec![Arc::new(v1), Arc::new(v2)];
    timeline.audio_tracks = vec![Arc::new(a1), Arc::new(a2)];
    project.timelines.push(Arc::new(timeline));
    Sample {
        project,
        interview: ids.0,
        music: ids.1,
    }
}

fn first_track(project: &mut Project, kind: TrackKind) -> &mut Track {
    let timeline = Arc::make_mut(&mut project.timelines[0]);
    let tracks = match kind {
        TrackKind::Video => &mut timeline.video_tracks,
        TrackKind::Audio => &mut timeline.audio_tracks,
    };
    Arc::make_mut(&mut tracks[0])
}

#[test]
fn sample_project_is_valid() {
    let Sample { project, .. } = sample();
    assert_eq!(project.validate(), Ok(()));
    let timeline = &project.timelines[0];
    assert_eq!(timeline.tracks().count(), 4);
    assert_eq!(timeline.duration(), frames(10_025));
    assert_eq!(project.default_format.display_aspect().to_string(), "16/9");
}

#[test]
fn json_round_trip_is_lossless() {
    let Sample { project, .. } = sample();
    let json = serde_json::to_string_pretty(&project).expect("serializes");
    let back: Project = serde_json::from_str(&json).expect("deserializes");
    assert_eq!(back, project);
    // Times are stored as exact fractions, not floats.
    assert!(
        json.contains(r#""num": 4"#) && json.contains(r#""den": 1"#),
        "{json}"
    );
}

#[test]
fn deserializing_rejects_invalid_time_values() {
    let bad_rate = r#"{"num": 0, "den": 1}"#;
    assert!(serde_json::from_str::<FrameRate>(bad_rate).is_err());
    let zero_den = r#"{"num": 1, "den": 0}"#;
    assert!(serde_json::from_str::<Time>(zero_den).is_err());
}

#[test]
fn snapshot_clone_shares_untouched_tracks() {
    let Sample { mut project, .. } = sample();
    let snapshot = project.clone();
    first_track(&mut project, TrackKind::Video).clips.pop();

    let (old, new) = (&snapshot.timelines[0], &project.timelines[0]);
    assert_ne!(
        old.video_tracks[0], new.video_tracks[0],
        "edited track was copied"
    );
    assert_eq!(old.video_tracks[0].clips.len(), 2, "snapshot is unchanged");
    for (a, b) in old.tracks().zip(new.tracks()).skip(1) {
        assert!(Arc::ptr_eq(a, b), "untouched tracks are shared, not copied");
    }
    assert!(Arc::ptr_eq(&snapshot.media, &project.media));
}

#[test]
fn detects_overlapping_clips() {
    let Sample { mut project, .. } = sample();
    let track = first_track(&mut project, TrackKind::Video);
    track.clips[1].start = frames(99);
    let (a, b) = (track.clips[0].id, track.clips[1].id);
    assert_eq!(project.validate(), Err(ModelError::Overlap(a, b)));
}

#[test]
fn detects_clip_past_end_of_media() {
    let Sample { mut project, .. } = sample();
    let clip = &mut first_track(&mut project, TrackKind::Video).clips[1];
    clip.duration = frames(60 * 25 - 500 + 1);
    let id = clip.id;
    // Source 500 + 1001 frames ends one frame past the 1500-frame media.
    assert_eq!(project.validate(), Err(ModelError::BeyondMedia(id)));
}

#[test]
fn detects_missing_media_and_stream_mismatch() {
    let Sample {
        mut project,
        interview,
        music,
    } = sample();
    let clip = &mut first_track(&mut project, TrackKind::Video).clips[0];
    clip.media = MediaId::new();
    let (id, missing) = (clip.id, clip.media);
    assert_eq!(
        project.validate(),
        Err(ModelError::MissingMedia {
            clip: id,
            media: missing
        })
    );

    let clip = &mut first_track(&mut project, TrackKind::Video).clips[0];
    clip.media = music;
    assert_eq!(
        project.validate(),
        Err(ModelError::StreamMismatch {
            clip: id,
            media: music,
            kind: TrackKind::Video
        })
    );
    first_track(&mut project, TrackKind::Video).clips[0].media = interview;
    assert_eq!(project.validate(), Ok(()));
}

#[test]
fn detects_empty_and_negative_clips() {
    let Sample { mut project, .. } = sample();
    let clip = &mut first_track(&mut project, TrackKind::Audio).clips[0];
    let id = clip.id;
    clip.duration = Time::ZERO;
    assert_eq!(project.validate(), Err(ModelError::EmptyClip(id)));

    let clip = &mut first_track(&mut project, TrackKind::Audio).clips[0];
    clip.duration = frames(1);
    clip.source_in = -frames(1);
    assert_eq!(project.validate(), Err(ModelError::NegativeTime(id)));
}

#[test]
fn detects_duplicate_ids_and_misplaced_tracks() {
    let Sample { mut project, .. } = sample();
    let track = first_track(&mut project, TrackKind::Audio);
    let dup = track.clips[0].clone();
    track.clips[1].id = dup.id;
    assert_eq!(
        project.validate(),
        Err(ModelError::DuplicateId(dup.id.to_string()))
    );

    let Sample { mut project, .. } = sample();
    let timeline = Arc::make_mut(&mut project.timelines[0]);
    let audio = timeline.audio_tracks.remove(0);
    let id = audio.id;
    timeline.video_tracks.push(audio);
    assert_eq!(
        project.validate(),
        Err(ModelError::TrackKindMismatch {
            track: id,
            expected: TrackKind::Video,
            actual: TrackKind::Audio
        })
    );
}

#[test]
fn detects_invalid_format() {
    let Sample { mut project, .. } = sample();
    project.default_format.width = 0;
    assert_eq!(project.validate(), Err(ModelError::InvalidFormat));
}
