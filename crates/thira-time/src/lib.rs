//! Exact time for editing.
//!
//! Floating-point seconds drift: 1/29.97 cannot be represented exactly, so frame
//! positions computed from `f64` slowly slide. Everything here is integer-exact:
//!
//! - [`Rational`]: a reduced fraction with overflow-checked arithmetic.
//! - [`FrameRate`]: video frame rates such as 24000/1001 (23.976).
//! - [`Time`]: a point or duration in seconds, as a [`Rational`]. Converts to and
//!   from frame and audio sample counts.
//! - [`Timecode`]: SMPTE `HH:MM:SS:FF` labels, including drop-frame for 29.97 and
//!   59.94.

mod rate;
mod rational;
mod time;
mod timecode;

pub use rate::FrameRate;
pub use rational::{Rational, RationalError};
pub use time::Time;
pub use timecode::{Timecode, TimecodeError};
