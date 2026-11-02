use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MediaError {
    /// A loaded FFmpeg library is not the LGPL 2.1+ build Thira may ship.
    #[error(
        "lib{library} is licensed \"{license}\", but Thira requires \"{}\"",
        crate::LGPL
    )]
    License {
        library: &'static str,
        license: String,
    },

    /// A loaded FFmpeg library has a different major version (ABI) than the
    /// headers Thira was built against.
    #[error("lib{library} {loaded} is loaded, but Thira was built for major version {built}")]
    Version {
        library: &'static str,
        loaded: String,
        built: u8,
    },

    #[error("cannot open {}: {source}", path.display())]
    Open {
        path: PathBuf,
        #[source]
        source: rsmpeg::error::RsmpegError,
    },

    #[error("path contains a NUL character: {}", .0.display())]
    InvalidPath(PathBuf),
}
