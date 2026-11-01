use thiserror::Error;

use crate::{ClipId, MediaId, TimelineId, TrackId};

/// A consistency problem found by [`Project::validate`](crate::Project::validate).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ModelError {
    #[error("format must have non-zero width, height and sample rate, and a positive pixel aspect")]
    InvalidFormat,

    #[error("{0} is used by more than one object")]
    DuplicateId(String),

    #[error("media pool entry {0} is stored under a different key")]
    MediaKeyMismatch(MediaId),

    #[error("track {track} is a {actual:?} track in the {expected:?} track list")]
    TrackKindMismatch {
        track: TrackId,
        expected: crate::TrackKind,
        actual: crate::TrackKind,
    },

    #[error("clip {0} has a zero or negative duration")]
    EmptyClip(ClipId),

    #[error("clip {0} starts before zero on the timeline or in its source")]
    NegativeTime(ClipId),

    #[error("clips {0} and {1} overlap or are out of order")]
    Overlap(ClipId, ClipId),

    #[error("clip {clip} refers to media {media}, which is not in the project")]
    MissingMedia { clip: ClipId, media: MediaId },

    #[error("clip {clip} on a {kind:?} track uses media {media} that has no matching stream")]
    StreamMismatch {
        clip: ClipId,
        media: MediaId,
        kind: crate::TrackKind,
    },

    #[error("clip {0} extends past the end of its media")]
    BeyondMedia(ClipId),

    #[error("timeline {0} has an invalid format")]
    InvalidTimelineFormat(TimelineId),
}
