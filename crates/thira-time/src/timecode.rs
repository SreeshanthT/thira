use std::fmt;

use crate::{FrameRate, Time};

/// Errors from building or parsing a [`Timecode`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimecodeError {
    /// Drop-frame was requested for a rate other than 29.97 or 59.94.
    DropFrameUnsupported(FrameRate),
    /// Minutes or seconds were 60 or more, or frames were at or above the nominal rate.
    FieldOutOfRange,
    /// A drop-frame label that is skipped, such as `00:01:00;00` at 29.97.
    DroppedLabel,
    /// Timecodes cannot be negative.
    NegativeFrame,
    /// Hours do not fit in `u32`.
    Overflow,
    /// The text is not `HH:MM:SS:FF` or `HH:MM:SS;FF`.
    Syntax,
}

impl fmt::Display for TimecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DropFrameUnsupported(rate) => {
                write!(f, "drop-frame timecode is not defined at {rate}")
            }
            Self::FieldOutOfRange => f.write_str("timecode field out of range"),
            Self::DroppedLabel => f.write_str("label is skipped in drop-frame timecode"),
            Self::NegativeFrame => f.write_str("timecode cannot be negative"),
            Self::Overflow => f.write_str("timecode hours overflow"),
            Self::Syntax => f.write_str("expected HH:MM:SS:FF or HH:MM:SS;FF"),
        }
    }
}

impl std::error::Error for TimecodeError {}

/// An SMPTE timecode label at a given frame rate.
///
/// Labels count whole frames at the nominal rate (30 for 29.97). In drop-frame
/// mode, the first 2 labels (4 at 59.94) of each minute are skipped except every
/// tenth minute, so the label tracks wall-clock time. Drop-frame is written with
/// `;` before the frames field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Timecode {
    rate: FrameRate,
    drop_frame: bool,
    hours: u32,
    minutes: u32,
    seconds: u32,
    frames: u32,
}

impl Timecode {
    /// Builds a timecode from its fields, checking that the label exists.
    pub fn new(
        rate: FrameRate,
        drop_frame: bool,
        hours: u32,
        minutes: u32,
        seconds: u32,
        frames: u32,
    ) -> Result<Self, TimecodeError> {
        check_drop_frame(rate, drop_frame)?;
        if minutes >= 60 || seconds >= 60 || frames >= rate.nominal_fps() {
            return Err(TimecodeError::FieldOutOfRange);
        }
        if drop_frame
            && seconds == 0
            && !minutes.is_multiple_of(10)
            && i64::from(frames) < drops_per_minute(rate)
        {
            return Err(TimecodeError::DroppedLabel);
        }
        Ok(Self {
            rate,
            drop_frame,
            hours,
            minutes,
            seconds,
            frames,
        })
    }

    /// The label of frame `frame`, counting from `00:00:00:00`.
    pub fn from_frame(
        frame: i64,
        rate: FrameRate,
        drop_frame: bool,
    ) -> Result<Self, TimecodeError> {
        check_drop_frame(rate, drop_frame)?;
        if frame < 0 {
            return Err(TimecodeError::NegativeFrame);
        }
        let nominal = i64::from(rate.nominal_fps());
        let mut label = frame;
        if drop_frame {
            // Map the real frame count to the label count by adding back skipped labels.
            let drops = drops_per_minute(rate);
            let per_ten_minutes = nominal * 600 - drops * 9;
            let per_minute = nominal * 60 - drops;
            let tens = frame / per_ten_minutes;
            let rest = frame % per_ten_minutes;
            label += drops * 9 * tens;
            if rest > drops {
                label += drops * ((rest - drops) / per_minute);
            }
        }
        let field = |v: i64| u32::try_from(v).map_err(|_| TimecodeError::Overflow);
        Ok(Self {
            rate,
            drop_frame,
            hours: field(label / (nominal * 3600))?,
            minutes: field(label / (nominal * 60) % 60)?,
            seconds: field(label / nominal % 60)?,
            frames: field(label % nominal)?,
        })
    }

    /// The label for the frame containing `time`.
    pub fn from_time(time: Time, rate: FrameRate, drop_frame: bool) -> Result<Self, TimecodeError> {
        Self::from_frame(time.to_frame(rate), rate, drop_frame)
    }

    /// Frame number of this label, counting from `00:00:00:00`.
    pub fn to_frame(self) -> i64 {
        let nominal = i64::from(self.rate.nominal_fps());
        let total_minutes = i64::from(self.hours) * 60 + i64::from(self.minutes);
        let label =
            (total_minutes * 60 + i64::from(self.seconds)) * nominal + i64::from(self.frames);
        if self.drop_frame {
            label - drops_per_minute(self.rate) * (total_minutes - total_minutes / 10)
        } else {
            label
        }
    }

    /// Start time of this label's frame.
    pub fn to_time(self) -> Time {
        Time::from_frame(self.to_frame(), self.rate)
    }

    /// Parses `HH:MM:SS:FF` (non-drop) or `HH:MM:SS;FF` (drop-frame) at `rate`.
    /// Separators before the seconds may be `:` or `;`.
    pub fn parse(text: &str, rate: FrameRate) -> Result<Self, TimecodeError> {
        let bytes = text.as_bytes();
        let seps: Vec<usize> = bytes
            .iter()
            .enumerate()
            .filter(|(_, b)| matches!(b, b':' | b';'))
            .map(|(i, _)| i)
            .collect();
        let [a, b, c] = seps[..] else {
            return Err(TimecodeError::Syntax);
        };
        let drop_frame = bytes[c] == b';';
        let num = |s: &str| -> Result<u32, TimecodeError> {
            if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
                return Err(TimecodeError::Syntax);
            }
            s.parse().map_err(|_| TimecodeError::Overflow)
        };
        Self::new(
            rate,
            drop_frame,
            num(&text[..a])?,
            num(&text[a + 1..b])?,
            num(&text[b + 1..c])?,
            num(&text[c + 1..])?,
        )
    }

    pub const fn rate(self) -> FrameRate {
        self.rate
    }

    pub const fn is_drop_frame(self) -> bool {
        self.drop_frame
    }

    pub const fn hours(self) -> u32 {
        self.hours
    }

    pub const fn minutes(self) -> u32 {
        self.minutes
    }

    pub const fn seconds(self) -> u32 {
        self.seconds
    }

    pub const fn frames(self) -> u32 {
        self.frames
    }
}

impl fmt::Display for Timecode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sep = if self.drop_frame { ';' } else { ':' };
        write!(
            f,
            "{:02}:{:02}:{:02}{sep}{:02}",
            self.hours, self.minutes, self.seconds, self.frames
        )
    }
}

fn check_drop_frame(rate: FrameRate, drop_frame: bool) -> Result<(), TimecodeError> {
    if drop_frame && !rate.supports_drop_frame() {
        return Err(TimecodeError::DropFrameUnsupported(rate));
    }
    Ok(())
}

/// Labels skipped at the start of each non-tenth minute: 2 at 29.97, 4 at 59.94.
fn drops_per_minute(rate: FrameRate) -> i64 {
    i64::from(rate.nominal_fps()) / 15
}

#[cfg(test)]
mod tests {
    use super::*;

    const DF: FrameRate = FrameRate::FPS_29_97;

    fn tc(frame: i64, rate: FrameRate, drop: bool) -> String {
        Timecode::from_frame(frame, rate, drop)
            .expect("valid timecode")
            .to_string()
    }

    #[test]
    fn drop_frame_29_97_reference_points() {
        assert_eq!(tc(0, DF, true), "00:00:00;00");
        assert_eq!(tc(1799, DF, true), "00:00:59;29");
        assert_eq!(tc(1800, DF, true), "00:01:00;02");
        assert_eq!(tc(17_981, DF, true), "00:09:59;29");
        assert_eq!(tc(17_982, DF, true), "00:10:00;00");
        assert_eq!(tc(107_892, DF, true), "01:00:00;00");
    }

    #[test]
    fn drop_frame_59_94_skips_four_labels() {
        let rate = FrameRate::FPS_59_94;
        assert_eq!(tc(3599, rate, true), "00:00:59;59");
        assert_eq!(tc(3600, rate, true), "00:01:00;04");
        assert_eq!(tc(215_784, rate, true), "01:00:00;00");
    }

    #[test]
    fn non_drop() {
        assert_eq!(tc(1800, DF, false), "00:01:00:00");
        assert_eq!(tc(86_400, FrameRate::FPS_24, false), "01:00:00:00");
        assert_eq!(tc(24, FrameRate::FPS_23_976, false), "00:00:01:00");
    }

    #[test]
    fn rejects_dropped_labels_and_bad_fields() {
        assert_eq!(
            Timecode::parse("00:01:00;00", DF),
            Err(TimecodeError::DroppedLabel)
        );
        assert_eq!(
            Timecode::parse("00:01:00;01", DF),
            Err(TimecodeError::DroppedLabel)
        );
        assert!(Timecode::parse("00:10:00;00", DF).is_ok());
        assert_eq!(
            Timecode::parse("00:00:00:30", DF),
            Err(TimecodeError::FieldOutOfRange)
        );
        assert_eq!(
            Timecode::parse("00:60:00:00", DF),
            Err(TimecodeError::FieldOutOfRange)
        );
    }

    #[test]
    fn rejects_unsupported_drop_frame() {
        assert_eq!(
            Timecode::parse("00:00:00;00", FrameRate::FPS_25),
            Err(TimecodeError::DropFrameUnsupported(FrameRate::FPS_25))
        );
    }

    #[test]
    fn parse_syntax() {
        assert_eq!(
            Timecode::parse("01;02;03;04", DF).map(|t| t.to_string()),
            Ok("01:02:03;04".into())
        );
        for bad in [
            "",
            "1:2:3",
            "00:00:00:0a",
            "00:00::00",
            "+0:00:00:00",
            "00:00:00:00:00",
        ] {
            assert_eq!(
                Timecode::parse(bad, DF),
                Err(TimecodeError::Syntax),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn negative_frames_are_rejected() {
        assert_eq!(
            Timecode::from_frame(-1, DF, false),
            Err(TimecodeError::NegativeFrame)
        );
    }
}
