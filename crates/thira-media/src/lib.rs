//! Media I/O for Thira through FFmpeg (PRD MED-1).
//!
//! FFmpeg is an LGPL 2.1+ build linked dynamically, provisioned by
//! `pnpm ffmpeg:fetch`. The build script refuses GPL, (L)GPLv3 and non-free
//! builds, and [`check_ffmpeg`] checks the libraries actually loaded at run time.
//!
//! This is the only crate allowed to use `unsafe` (for FFmpeg calls rsmpeg
//! does not wrap). Each use needs `#[allow(unsafe_code)]` and a `// SAFETY:`
//! comment.
#![deny(
    clippy::undocumented_unsafe_blocks,
    clippy::multiple_unsafe_ops_per_block
)]

mod error;
mod ffmpeg;
mod probe;

pub use error::MediaError;
pub use ffmpeg::{LGPL, Library, check_ffmpeg, libraries};
pub use probe::{MediaInfo, StreamInfo, StreamKind, probe};
