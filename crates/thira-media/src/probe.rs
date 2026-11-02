use std::ffi::{CStr, CString};
use std::path::Path;

use rsmpeg::avformat::{AVFormatContextInput, AVStreamRef};
use rsmpeg::ffi;
use thira_time::{FrameRate, Rational, Time};

use crate::MediaError;

/// What a media file contains, like `ffprobe` reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaInfo {
    /// Demuxer name, such as `mov,mp4,m4a,3gp,3g2,mj2` or `matroska,webm`.
    pub format: String,
    /// Container duration, when the file states or implies one.
    pub duration: Option<Time>,
    pub streams: Vec<StreamInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamInfo {
    /// Index of the stream in the file.
    pub index: usize,
    /// FFmpeg codec name, such as `h264` or `aac`.
    pub codec: String,
    pub kind: StreamKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamKind {
    Video {
        width: u32,
        height: u32,
        /// `None` when FFmpeg cannot tell (for example, variable frame rate
        /// without a stated average).
        frame_rate: Option<FrameRate>,
    },
    Audio {
        sample_rate: u32,
        channels: u32,
    },
    Subtitle,
    Other,
}

/// Opens `path` and reads its container and stream properties.
pub fn probe(path: &Path) -> Result<MediaInfo, MediaError> {
    // FFmpeg takes UTF-8 paths on Windows and raw bytes elsewhere, which is
    // what the OS-specific encoding gives on each.
    let url = CString::new(path.as_os_str().as_encoded_bytes())
        .map_err(|_| MediaError::InvalidPath(path.to_path_buf()))?;
    let input = AVFormatContextInput::open(&url).map_err(|source| MediaError::Open {
        path: path.to_path_buf(),
        source,
    })?;

    let duration = (input.duration != ffi::AV_NOPTS_VALUE && input.duration >= 0)
        .then(|| Rational::new(input.duration, i64::from(ffi::AV_TIME_BASE)).ok())
        .flatten()
        .map(Time::from_seconds);

    Ok(MediaInfo {
        format: input.iformat().name().to_string_lossy().into_owned(),
        duration,
        streams: input.streams().iter().map(stream_info).collect(),
    })
}

fn stream_info(stream: &AVStreamRef<'_>) -> StreamInfo {
    let par = stream.codecpar();
    let media_type = par.codec_type();
    let kind = if media_type.is_video() {
        let rate = stream.guess_framerate();
        StreamKind::Video {
            width: u32::try_from(par.width).unwrap_or(0),
            height: u32::try_from(par.height).unwrap_or(0),
            frame_rate: rate
                .and_then(|r| Rational::new(r.num.into(), r.den.into()).ok())
                .and_then(FrameRate::new),
        }
    } else if media_type.is_audio() {
        StreamKind::Audio {
            sample_rate: u32::try_from(par.sample_rate).unwrap_or(0),
            channels: u32::try_from(par.ch_layout.nb_channels).unwrap_or(0),
        }
    } else if media_type.is_subtitle() {
        StreamKind::Subtitle
    } else {
        StreamKind::Other
    };

    StreamInfo {
        index: usize::try_from(stream.index).unwrap_or(0),
        codec: codec_name(par.codec_id),
        kind,
    }
}

#[allow(unsafe_code)]
fn codec_name(id: ffi::AVCodecID) -> String {
    // SAFETY: avcodec_get_name accepts any codec ID and has no preconditions.
    let ptr = unsafe { ffi::avcodec_get_name(id) };
    // SAFETY: it returns a static NUL-terminated string, never null
    // ("unknown_codec" for unknown IDs).
    let name = unsafe { CStr::from_ptr(ptr) };
    name.to_string_lossy().into_owned()
}
