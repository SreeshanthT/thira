use rsmpeg::avutil::AvVersion;
use rsmpeg::{avcodec, avdevice, avfilter, avformat, avutil, swresample, swscale};

use crate::MediaError;

/// The license string every FFmpeg library must report. GPL, (L)GPLv3 and
/// non-free builds report something else.
pub const LGPL: &str = "LGPL version 2.1 or later";

/// One FFmpeg library as loaded at run time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Library {
    /// Name without the `lib` prefix, such as `avcodec`.
    pub name: &'static str,
    pub version: AvVersion,
    /// Major version of the headers Thira was built against.
    pub built_major: u8,
    pub license: String,
    pub configuration: String,
}

macro_rules! library {
    ($name:ident) => {
        Library {
            name: stringify!($name),
            version: $name::version(),
            built_major: $name::VERSION.major,
            license: $name::license().to_string_lossy().into_owned(),
            configuration: $name::configuration().to_string_lossy().into_owned(),
        }
    };
}

/// All FFmpeg libraries Thira links, as loaded at run time.
pub fn libraries() -> Vec<Library> {
    vec![
        library!(avutil),
        library!(avcodec),
        library!(avformat),
        library!(avfilter),
        library!(avdevice),
        library!(swresample),
        library!(swscale),
    ]
}

/// Checks that the loaded FFmpeg is LGPL 2.1+ and ABI-compatible with the
/// headers Thira was built against, and returns its version string.
///
/// The build script checks the package being linked; this checks what is
/// actually loaded, which could differ if another FFmpeg is found first.
pub fn check_ffmpeg() -> Result<String, MediaError> {
    for library in libraries() {
        if library.license != LGPL {
            return Err(MediaError::License {
                library: library.name,
                license: library.license,
            });
        }
        if library.version.major != library.built_major {
            return Err(MediaError::Version {
                library: library.name,
                loaded: library.version.to_string(),
                built: library.built_major,
            });
        }
    }
    Ok(avutil::version_info().to_string_lossy().into_owned())
}
