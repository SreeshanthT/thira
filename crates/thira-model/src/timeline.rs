use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thira_time::Time;

use crate::{ClipId, Format, MediaId, TimelineId, TrackId};

/// A sequence of tracks that plays and exports as one program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timeline {
    pub id: TimelineId,
    pub name: String,
    /// Copied from the project's default format when the timeline is created.
    pub format: Format,
    /// Bottom to top: later tracks composite over earlier ones.
    pub video_tracks: Vec<Arc<Track>>,
    /// In display order, top to bottom.
    pub audio_tracks: Vec<Arc<Track>>,
}

impl Timeline {
    /// An empty timeline with the given format.
    pub fn new(name: impl Into<String>, format: Format) -> Self {
        Self {
            id: TimelineId::new(),
            name: name.into(),
            format,
            video_tracks: Vec::new(),
            audio_tracks: Vec::new(),
        }
    }

    /// All tracks: video first, then audio.
    pub fn tracks(&self) -> impl Iterator<Item = &Arc<Track>> {
        self.video_tracks.iter().chain(&self.audio_tracks)
    }

    /// End of the last clip on any track.
    pub fn duration(&self) -> Time {
        self.tracks()
            .filter_map(|t| t.clips.last())
            .map(Clip::end)
            .max()
            .unwrap_or(Time::ZERO)
    }
}

/// Whether a track holds pictures or sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackKind {
    Video,
    Audio,
}

/// One lane of a timeline. Clips are sorted by start time and never overlap;
/// the space between clips is a gap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Track {
    pub id: TrackId,
    pub kind: TrackKind,
    pub name: String,
    pub clips: Vec<Clip>,
}

impl Track {
    pub fn new(kind: TrackKind, name: impl Into<String>) -> Self {
        Self {
            id: TrackId::new(),
            kind,
            name: name.into(),
            clips: Vec::new(),
        }
    }
}

/// A span of a media file placed on a track.
///
/// The clip shows source time `source_in .. source_in + duration` at timeline
/// time `start .. start + duration`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Clip {
    pub id: ClipId,
    pub media: MediaId,
    /// Position on the timeline.
    pub start: Time,
    /// Offset into the media where the clip begins.
    pub source_in: Time,
    pub duration: Time,
    /// Disabled clips stay on the timeline but are not played or exported.
    pub enabled: bool,
}

impl Clip {
    pub fn new(media: MediaId, start: Time, source_in: Time, duration: Time) -> Self {
        Self {
            id: ClipId::new(),
            media,
            start,
            source_in,
            duration,
            enabled: true,
        }
    }

    /// Timeline time just after the clip's last instant.
    pub fn end(&self) -> Time {
        self.start + self.duration
    }

    /// Source time just after the clip's last instant.
    pub fn source_out(&self) -> Time {
        self.source_in + self.duration
    }
}
