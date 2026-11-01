use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thira_time::{FrameRate, Time};

use crate::MediaId;

/// What kind of file a [`MediaRef`] points to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    /// Moving pictures, possibly with audio.
    Video,
    /// Audio only.
    Audio,
    /// A still image. Has no intrinsic duration; clips may be any length.
    Image,
}

/// The video stream used from a media file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VideoStream {
    pub width: u32,
    pub height: u32,
    /// `None` for still images and variable-frame-rate sources without a nominal rate.
    pub frame_rate: Option<FrameRate>,
}

/// The audio stream used from a media file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AudioStream {
    pub channels: u16,
    pub sample_rate: u32,
}

/// A source file in the project's media pool. Source media is never modified.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaRef {
    pub id: MediaId,
    /// Display name, initially the file name.
    pub name: String,
    pub path: PathBuf,
    pub kind: MediaKind,
    /// `None` for still images, or until the file has been probed.
    pub duration: Option<Time>,
    pub video: Option<VideoStream>,
    pub audio: Option<AudioStream>,
}

impl MediaRef {
    pub fn has_video(&self) -> bool {
        self.video.is_some()
    }

    pub fn has_audio(&self) -> bool {
        self.audio.is_some()
    }
}
