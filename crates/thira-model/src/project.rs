use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thira_time::{FrameRate, Rational, Time};

use crate::{MediaId, MediaKind, MediaRef, ModelError, ProjectId, Timeline, Track, TrackKind};

/// Picture and sound format of a timeline (PRD PRJ-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Format {
    pub width: u32,
    pub height: u32,
    /// Width of one pixel relative to its height; 1 for square pixels.
    pub pixel_aspect: Rational,
    pub frame_rate: FrameRate,
    pub sample_rate: u32,
}

impl Format {
    /// 1920x1080, square pixels, 48 kHz audio.
    pub fn hd_1080(frame_rate: FrameRate) -> Self {
        Self {
            width: 1920,
            height: 1080,
            pixel_aspect: Rational::ONE,
            frame_rate,
            sample_rate: 48_000,
        }
    }

    /// Display aspect ratio, such as 16/9.
    ///
    /// # Panics
    /// Panics if `height` is zero; [`Format::is_valid`] rules that out.
    pub fn display_aspect(&self) -> Rational {
        Rational::from_integer(i64::from(self.width)) * self.pixel_aspect
            / Rational::from_integer(i64::from(self.height))
    }

    pub fn is_valid(&self) -> bool {
        self.width > 0
            && self.height > 0
            && self.sample_rate > 0
            && self.pixel_aspect > Rational::ZERO
    }
}

/// An editing project: the media pool and the timelines built from it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    /// Format given to new timelines.
    pub default_format: Format,
    /// Media pool, keyed by ID for stable ordering.
    pub media: Arc<BTreeMap<MediaId, MediaRef>>,
    pub timelines: Vec<Arc<Timeline>>,
}

impl Project {
    /// An empty project.
    pub fn new(name: impl Into<String>, default_format: Format) -> Self {
        Self {
            id: ProjectId::new(),
            name: name.into(),
            default_format,
            media: Arc::default(),
            timelines: Vec::new(),
        }
    }

    /// Checks every invariant the editor relies on. Returns the first problem found.
    pub fn validate(&self) -> Result<(), ModelError> {
        if !self.default_format.is_valid() {
            return Err(ModelError::InvalidFormat);
        }
        let mut ids = HashSet::new();
        let mut unique = |id: uuid::Uuid| {
            if ids.insert(id) {
                Ok(())
            } else {
                Err(ModelError::DuplicateId(id.to_string()))
            }
        };
        for (key, media) in self.media.iter() {
            if *key != media.id {
                return Err(ModelError::MediaKeyMismatch(media.id));
            }
        }
        for timeline in &self.timelines {
            unique(timeline.id.as_uuid())?;
            if !timeline.format.is_valid() {
                return Err(ModelError::InvalidTimelineFormat(timeline.id));
            }
            for (tracks, expected) in [
                (&timeline.video_tracks, TrackKind::Video),
                (&timeline.audio_tracks, TrackKind::Audio),
            ] {
                for track in tracks {
                    unique(track.id.as_uuid())?;
                    if track.kind != expected {
                        return Err(ModelError::TrackKindMismatch {
                            track: track.id,
                            expected,
                            actual: track.kind,
                        });
                    }
                    for clip in &track.clips {
                        unique(clip.id.as_uuid())?;
                    }
                    self.validate_track(track)?;
                }
            }
        }
        Ok(())
    }

    fn validate_track(&self, track: &Track) -> Result<(), ModelError> {
        for clip in &track.clips {
            if clip.duration <= Time::ZERO {
                return Err(ModelError::EmptyClip(clip.id));
            }
            if clip.start < Time::ZERO || clip.source_in < Time::ZERO {
                return Err(ModelError::NegativeTime(clip.id));
            }
            let media = self
                .media
                .get(&clip.media)
                .ok_or(ModelError::MissingMedia {
                    clip: clip.id,
                    media: clip.media,
                })?;
            let stream_ok = match track.kind {
                TrackKind::Video => media.has_video(),
                TrackKind::Audio => media.has_audio(),
            };
            if !stream_ok {
                return Err(ModelError::StreamMismatch {
                    clip: clip.id,
                    media: media.id,
                    kind: track.kind,
                });
            }
            // Stills can be held for any length; other media must cover the clip.
            if media.kind != MediaKind::Image
                && let Some(len) = media.duration
                && clip.source_out() > len
            {
                return Err(ModelError::BeyondMedia(clip.id));
            }
        }
        for pair in track.clips.windows(2) {
            if let [a, b] = pair
                && a.end() > b.start
            {
                return Err(ModelError::Overlap(a.id, b.id));
            }
        }
        Ok(())
    }
}
