//! Property tests for round-trips and drop-frame edge cases.

use proptest::prelude::*;
use thira_time::{FrameRate, Rational, Time, Timecode};

const RATES: [FrameRate; 9] = [
    FrameRate::FPS_23_976,
    FrameRate::FPS_24,
    FrameRate::FPS_25,
    FrameRate::FPS_29_97,
    FrameRate::FPS_30,
    FrameRate::FPS_48,
    FrameRate::FPS_50,
    FrameRate::FPS_59_94,
    FrameRate::FPS_60,
];

fn rate() -> impl Strategy<Value = FrameRate> {
    prop::sample::select(&RATES[..])
}

fn drop_rate() -> impl Strategy<Value = FrameRate> {
    prop::sample::select(&[FrameRate::FPS_29_97, FrameRate::FPS_59_94][..])
}

/// Up to about 100 hours at 60 fps.
fn frame() -> impl Strategy<Value = i64> {
    0..21_600_000i64
}

fn small_rational() -> impl Strategy<Value = Rational> {
    (-1_000_000i64..1_000_000, 1i64..100_000).prop_map(|(n, d)| Rational::new(n, d).expect("d > 0"))
}

proptest! {
    #[test]
    fn rational_addition_is_exact(a in small_rational(), b in small_rational(), c in small_rational()) {
        prop_assert_eq!((a + b) - b, a);
        prop_assert_eq!(a + b, b + a);
        prop_assert_eq!((a + b) + c, a + (b + c));
    }

    #[test]
    fn rational_multiplication_is_exact(a in small_rational(), b in small_rational()) {
        prop_assert_eq!(a * b, b * a);
        if !b.is_zero() {
            prop_assert_eq!((a * b) / b, a);
        }
    }

    #[test]
    fn rational_order_matches_subtraction(a in small_rational(), b in small_rational()) {
        prop_assert_eq!(a < b, (b - a) > Rational::ZERO);
    }

    #[test]
    fn frame_to_time_round_trips(f in frame(), r in rate()) {
        prop_assert_eq!(Time::from_frame(f, r).to_frame(r), f);
        prop_assert_eq!(Time::from_frame(f, r).to_nearest_frame(r), f);
    }

    #[test]
    fn any_time_inside_a_frame_maps_to_that_frame(f in frame(), r in rate(), k in 0i64..1000) {
        // A point k/1000 of the way through frame f.
        let offset = Time::from_seconds(r.frame_duration() * Rational::new(k, 1000).expect("valid"));
        prop_assert_eq!((Time::from_frame(f, r) + offset).to_frame(r), f);
    }

    #[test]
    fn sample_round_trips(s in 0i64..10_000_000_000, sr in prop::sample::select(&[44_100u32, 48_000, 96_000][..])) {
        prop_assert_eq!(Time::from_sample(s, sr).to_sample(sr), s);
    }

    #[test]
    fn timecode_round_trips_through_frames(f in frame(), r in rate(), drop in any::<bool>()) {
        let drop = drop && r.supports_drop_frame();
        let tc = Timecode::from_frame(f, r, drop).expect("valid");
        prop_assert_eq!(tc.to_frame(), f);
        prop_assert_eq!(Timecode::from_time(tc.to_time(), r, drop).expect("valid"), tc);
    }

    #[test]
    fn timecode_round_trips_through_text(f in frame(), r in rate(), drop in any::<bool>()) {
        let drop = drop && r.supports_drop_frame();
        let tc = Timecode::from_frame(f, r, drop).expect("valid");
        prop_assert_eq!(Timecode::parse(&tc.to_string(), r).expect("parses"), tc);
    }

    #[test]
    fn drop_frame_never_produces_a_skipped_label(f in frame(), r in drop_rate()) {
        let tc = Timecode::from_frame(f, r, true).expect("valid");
        let skipped = tc.seconds() == 0 && !tc.minutes().is_multiple_of(10) && tc.frames() < r.nominal_fps() / 15;
        prop_assert!(!skipped, "{tc} should have been skipped");
        // Re-validating through the checked constructor must succeed too.
        prop_assert!(Timecode::new(r, true, tc.hours(), tc.minutes(), tc.seconds(), tc.frames()).is_ok());
    }

    #[test]
    fn consecutive_frames_have_consecutive_labels(f in frame(), r in rate(), drop in any::<bool>()) {
        // Labels advance by exactly one, or jump over the skipped drop-frame labels.
        let drop = drop && r.supports_drop_frame();
        let a = Timecode::from_frame(f, r, drop).expect("valid");
        let b = Timecode::from_frame(f + 1, r, drop).expect("valid");
        let label = |t: Timecode| {
            let n = i64::from(r.nominal_fps());
            ((i64::from(t.hours()) * 60 + i64::from(t.minutes())) * 60 + i64::from(t.seconds())) * n + i64::from(t.frames())
        };
        let step = label(b) - label(a);
        let expected_jump = i64::from(r.nominal_fps()) / 15 + 1;
        prop_assert!(step == 1 || (drop && step == expected_jump), "{a} -> {b} steps {step}");
    }
}

#[test]
fn drop_frame_tracks_wall_clock_over_24_hours() {
    // 24 hours of real time at 29.97 is 2,589,410.6 frames. Drop-frame labels that within
    // 3 frames of 24:00:00 (the scheme's known residual drift); non-drop is 86.4 seconds behind.
    let frames = (Rational::from_integer(24 * 3600) * FrameRate::FPS_29_97.as_rational()).floor();
    assert_eq!(frames, 2_589_410);
    let df = Timecode::from_frame(frames, FrameRate::FPS_29_97, true).expect("valid");
    assert_eq!(df.to_string(), "24:00:00;02");
    let ndf = Timecode::from_frame(frames, FrameRate::FPS_29_97, false).expect("valid");
    assert_eq!(ndf.to_string(), "23:58:33:20");
}
