use std::fmt;

use crate::Rational;

/// A video frame rate in frames per second, such as `24000/1001` (23.976).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameRate(Rational);

impl FrameRate {
    pub const FPS_23_976: Self = Self::ntsc(24);
    pub const FPS_24: Self = Self::whole(24);
    pub const FPS_25: Self = Self::whole(25);
    pub const FPS_29_97: Self = Self::ntsc(30);
    pub const FPS_30: Self = Self::whole(30);
    pub const FPS_48: Self = Self::whole(48);
    pub const FPS_50: Self = Self::whole(50);
    pub const FPS_59_94: Self = Self::ntsc(60);
    pub const FPS_60: Self = Self::whole(60);

    /// Frames per second as an exact fraction. Returns `None` unless it is positive.
    pub fn new(fps: Rational) -> Option<Self> {
        (fps > Rational::ZERO).then_some(Self(fps))
    }

    /// `n` frames per second exactly.
    const fn whole(n: i64) -> Self {
        Self(Rational::from_integer(n))
    }

    /// The NTSC rate `n * 1000 / 1001`.
    const fn ntsc(n: i64) -> Self {
        Self(Rational::new_const(n * 1000, 1001))
    }

    pub const fn as_rational(self) -> Rational {
        self.0
    }

    /// Duration of one frame in seconds.
    pub fn frame_duration(self) -> Rational {
        Rational::ONE / self.0
    }

    /// The whole frame count used for timecode labels: 30 for 29.97, 24 for 23.976.
    pub fn nominal_fps(self) -> u32 {
        // Rates are positive and well below u32::MAX in practice; clamp just in case.
        u32::try_from(self.0.round().max(1)).unwrap_or(u32::MAX)
    }

    /// Whether drop-frame timecode is defined for this rate (29.97 and 59.94).
    pub fn supports_drop_frame(self) -> bool {
        self == Self::FPS_29_97 || self == Self::FPS_59_94
    }
}

impl fmt::Display for FrameRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.denom() == 1 {
            write!(f, "{} fps", self.0.numer())
        } else {
            write!(f, "{:.3} fps", self.0.to_f64())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ntsc_rates_are_exact() {
        assert_eq!(
            FrameRate::FPS_29_97.as_rational(),
            Rational::new(30000, 1001).expect("valid")
        );
        assert_eq!(
            FrameRate::FPS_23_976.as_rational(),
            Rational::new(24000, 1001).expect("valid")
        );
        assert_eq!(
            FrameRate::FPS_59_94.as_rational(),
            Rational::new(60000, 1001).expect("valid")
        );
    }

    #[test]
    fn nominal_fps() {
        assert_eq!(FrameRate::FPS_23_976.nominal_fps(), 24);
        assert_eq!(FrameRate::FPS_29_97.nominal_fps(), 30);
        assert_eq!(FrameRate::FPS_59_94.nominal_fps(), 60);
        assert_eq!(FrameRate::FPS_25.nominal_fps(), 25);
    }

    #[test]
    fn rejects_non_positive_rates() {
        assert_eq!(FrameRate::new(Rational::ZERO), None);
        assert_eq!(FrameRate::new(Rational::from_integer(-24)), None);
    }

    #[test]
    fn drop_frame_support() {
        assert!(FrameRate::FPS_29_97.supports_drop_frame());
        assert!(FrameRate::FPS_59_94.supports_drop_frame());
        assert!(!FrameRate::FPS_23_976.supports_drop_frame());
        assert!(!FrameRate::FPS_30.supports_drop_frame());
    }

    #[test]
    fn display() {
        assert_eq!(FrameRate::FPS_25.to_string(), "25 fps");
        assert_eq!(FrameRate::FPS_29_97.to_string(), "29.970 fps");
    }
}
