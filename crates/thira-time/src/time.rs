use std::fmt;
use std::ops::{Add, Neg, Sub};

use serde::{Deserialize, Serialize};

use crate::{FrameRate, Rational};

/// A point in time or a duration, in exact seconds.
///
/// Times convert to frame and sample indices by flooring: frame `n` covers
/// `[n / fps, (n + 1) / fps)`, so `Time::from_frame(n, r).to_frame(r) == n` for every `n`.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Time(Rational);

impl Time {
    pub const ZERO: Self = Self(Rational::ZERO);

    pub const fn from_seconds(seconds: Rational) -> Self {
        Self(seconds)
    }

    pub const fn seconds(self) -> Rational {
        self.0
    }

    /// Start time of frame `frame` at `rate`.
    ///
    /// # Panics
    /// Panics if the result overflows (beyond roughly 10^15 frames).
    pub fn from_frame(frame: i64, rate: FrameRate) -> Self {
        Self(Rational::from_integer(frame) * rate.frame_duration())
    }

    /// Index of the frame at `rate` that contains this time.
    ///
    /// # Panics
    /// Panics if the result overflows.
    pub fn to_frame(self, rate: FrameRate) -> i64 {
        (self.0 * rate.as_rational()).floor()
    }

    /// Index of the frame boundary at `rate` nearest to this time.
    ///
    /// # Panics
    /// Panics if the result overflows.
    pub fn to_nearest_frame(self, rate: FrameRate) -> i64 {
        (self.0 * rate.as_rational()).round()
    }

    /// Start time of audio sample `sample` at `sample_rate` Hz.
    ///
    /// # Panics
    /// Panics if `sample_rate` is zero.
    pub fn from_sample(sample: i64, sample_rate: u32) -> Self {
        assert!(sample_rate > 0, "sample rate must be positive");
        Self(Rational::from_integer(sample) / Rational::from_integer(i64::from(sample_rate)))
    }

    /// Index of the audio sample at `sample_rate` Hz that contains this time.
    ///
    /// # Panics
    /// Panics if the result overflows.
    pub fn to_sample(self, sample_rate: u32) -> i64 {
        (self.0 * Rational::from_integer(i64::from(sample_rate))).floor()
    }

    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }

    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }
}

impl Add for Time {
    type Output = Self;

    /// # Panics
    /// Panics on overflow.
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Time {
    type Output = Self;

    /// # Panics
    /// Panics on overflow.
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl Neg for Time {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}s", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_at_29_97() {
        let t = Time::from_frame(30, FrameRate::FPS_29_97);
        assert_eq!(t.seconds(), Rational::new(1001, 1000).expect("valid"));
        assert_eq!(t.to_frame(FrameRate::FPS_29_97), 30);
    }

    #[test]
    fn to_frame_floors_inside_a_frame() {
        let rate = FrameRate::FPS_25;
        let mid =
            Time::from_frame(10, rate) + Time::from_seconds(Rational::new(1, 100).expect("valid"));
        assert_eq!(mid.to_frame(rate), 10);
        assert_eq!(mid.to_nearest_frame(rate), 10);
        let late =
            Time::from_frame(11, rate) - Time::from_seconds(Rational::new(1, 100).expect("valid"));
        assert_eq!(late.to_frame(rate), 10);
        assert_eq!(late.to_nearest_frame(rate), 11);
    }

    #[test]
    fn negative_times_floor_toward_negative_infinity() {
        let t = Time::from_seconds(Rational::new(-1, 100).expect("valid"));
        assert_eq!(t.to_frame(FrameRate::FPS_25), -1);
    }

    #[test]
    fn samples() {
        let t = Time::from_sample(48_000, 48_000);
        assert_eq!(t.seconds(), Rational::ONE);
        assert_eq!(t.to_sample(44_100), 44_100);
        // One 29.97 frame is 1601.6 samples at 48 kHz: frame 1 starts inside sample 1601.
        assert_eq!(
            Time::from_frame(1, FrameRate::FPS_29_97).to_sample(48_000),
            1601
        );
    }
}
