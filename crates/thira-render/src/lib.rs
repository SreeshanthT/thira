//! GPU rendering for Thira (ADR 0001, ADR 0003).
//!
//! - [`Gpu`]: the wgpu device, falling back to a software adapter when no GPU
//!   is available (CI machines).
//! - [`TestPattern`]: color bars with a moving box, drawn into a [`Viewport`].
//!   Used to bring up the viewer before real media rendering exists.
//! - [`render_rgba`]: renders offscreen and reads the frame back, for tests,
//!   golden frames and the frame-transfer viewer backend.

mod error;
mod gpu;
mod offscreen;
mod pattern;
mod viewport;

pub use error::RenderError;
pub use gpu::{AdapterSummary, Gpu};
pub use offscreen::render_rgba;
pub use pattern::TestPattern;
pub use viewport::Viewport;

/// Re-exported so callers use the same wgpu version as this crate.
pub use wgpu;
