use std::path::{Path, PathBuf};

use thira_media::{LGPL, MediaError, MediaInfo, StreamInfo, StreamKind, check_ffmpeg, libraries};
use thira_time::{FrameRate, Rational, Time};

fn data(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data")
        .join(name)
}

#[test]
fn loaded_ffmpeg_is_lgpl_and_matches_the_headers() {
    let version = check_ffmpeg().expect("FFmpeg check");
    assert!(
        version.starts_with("8.1"),
        "unexpected FFmpeg version {version}"
    );

    let libraries = libraries();
    assert_eq!(libraries.len(), 7);
    for library in libraries {
        assert_eq!(library.license, LGPL, "lib{}", library.name);
        for option in ["--enable-gpl", "--enable-version3", "--enable-nonfree"] {
            assert!(
                !library.configuration.contains(option),
                "lib{} configured with {option}",
                library.name
            );
        }
    }
}

/// tests/data/clip.mov was made with the packaged FFmpeg:
/// `ffmpeg -f lavfi -i testsrc2=size=64x36:rate=24000/1001:duration=0.5
///  -f lavfi -i sine=frequency=440:sample_rate=48000:duration=0.5 -ac 2
///  -c:v mpeg4 -q:v 10 -c:a aac -b:a 32k -fflags +bitexact -flags:v +bitexact
///  -flags:a +bitexact -map_metadata -1 -shortest clip.mov`
#[test]
fn probe_reads_container_and_streams() {
    let info = thira_media::probe(&data("clip.mov")).expect("probe");
    assert_eq!(
        info,
        MediaInfo {
            format: "mov,mp4,m4a,3gp,3g2,mj2".into(),
            duration: Some(Time::from_seconds(Rational::new(1, 2).expect("1/2"))),
            streams: vec![
                StreamInfo {
                    index: 0,
                    codec: "mpeg4".into(),
                    kind: StreamKind::Video {
                        width: 64,
                        height: 36,
                        frame_rate: Some(FrameRate::FPS_23_976),
                    },
                },
                StreamInfo {
                    index: 1,
                    codec: "aac".into(),
                    kind: StreamKind::Audio {
                        sample_rate: 48_000,
                        channels: 2,
                    },
                },
            ],
        }
    );
}

#[test]
fn probe_reports_missing_files() {
    let path = data("does-not-exist.mov");
    match thira_media::probe(&path) {
        Err(MediaError::Open { path: p, .. }) => assert_eq!(p, path),
        other => panic!("expected an open error, got {other:?}"),
    }
}

#[test]
fn probe_rejects_files_that_are_not_media() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    assert!(matches!(
        thira_media::probe(&path),
        Err(MediaError::Open { .. })
    ));
}

#[test]
fn probe_rejects_paths_with_nul() {
    assert!(matches!(
        thira_media::probe(Path::new("a\0b.mov")),
        Err(MediaError::InvalidPath(_))
    ));
}
