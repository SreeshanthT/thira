//! Thira's project data model: plain data with validation, no editing behavior.
//!
//! Edits live in `thira-edit`; this crate only defines the shapes and checks
//! that a project is consistent ([`Project::validate`]).
//!
//! # Snapshots
//!
//! The engine thread owns the mutable [`Project`] and publishes clones of it to
//! other threads (ADR 0001). Cloning is cheap: timelines, tracks and the media
//! table sit behind [`Arc`](std::sync::Arc), so a clone copies pointers, and an
//! edit made through [`Arc::make_mut`](std::sync::Arc::make_mut) copies only the
//! track it changes.

mod error;
mod ids;
mod media;
mod project;
mod timeline;

pub use error::ModelError;
pub use ids::{ClipId, MediaId, ProjectId, TimelineId, TrackId};
pub use media::{AudioStream, MediaKind, MediaRef, VideoStream};
pub use project::{Format, Project};
pub use timeline::{Clip, Timeline, Track, TrackKind};

pub use thira_time::{FrameRate, Rational, Time};
