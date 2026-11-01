//! Save/load round-trips, format checks, path relinking and crash safety.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use thira_model::{
    AudioStream, Clip, Format, FrameRate, MediaId, MediaKind, MediaRef, Project, Time, Timeline,
    Track, TrackKind, VideoStream,
};
use thira_store::{SCHEMA_VERSION, StoreError, load, save};

const RATE: FrameRate = FrameRate::FPS_29_97;

fn frames(n: i64) -> Time {
    Time::from_frame(n, RATE)
}

fn media(path: PathBuf, kind: MediaKind) -> MediaRef {
    MediaRef {
        id: MediaId::new(),
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        path,
        kind,
        duration: (kind != MediaKind::Image).then(|| frames(108_000)),
        video: (kind != MediaKind::Audio).then_some(VideoStream {
            width: 3840,
            height: 2160,
            frame_rate: (kind == MediaKind::Video).then_some(RATE),
        }),
        audio: (kind != MediaKind::Image).then_some(AudioStream {
            channels: 2,
            sample_rate: 48_000,
        }),
    }
}

/// A project with every kind of media, two timelines, and `clips` clips on V1 and A1.
fn project(media_dir: &Path, clips: i64) -> Project {
    let mut project = Project::new("Round trip ✓", Format::hd_1080(RATE));
    let video = media(media_dir.join("interview.mov"), MediaKind::Video);
    let music = media(media_dir.join("music.wav"), MediaKind::Audio);
    let logo = media(media_dir.join("logo.png"), MediaKind::Image);
    let (v, m, l) = (video.id, music.id, logo.id);
    Arc::make_mut(&mut project.media).extend([video, music, logo].map(|x| (x.id, x)));

    let mut v1 = Track::new(TrackKind::Video, "V1");
    let mut a1 = Track::new(TrackKind::Audio, "A1");
    for i in 0..clips {
        let clip = Clip::new(v, frames(i * 31), frames(i * 7), frames(30));
        a1.clips
            .push(Clip::new(v, clip.start, clip.source_in, clip.duration));
        v1.clips.push(Clip {
            enabled: i % 3 != 0,
            ..clip
        });
    }
    let mut v2 = Track::new(TrackKind::Video, "Logo");
    v2.clips
        .push(Clip::new(l, frames(15), Time::ZERO, frames(900)));
    let mut a2 = Track::new(TrackKind::Audio, "Music");
    a2.clips
        .push(Clip::new(m, Time::ZERO, frames(1), frames(5000)));

    let mut main = Timeline::new("Main", project.default_format);
    main.video_tracks = vec![Arc::new(v1), Arc::new(v2)];
    main.audio_tracks = vec![Arc::new(a1), Arc::new(a2)];
    let mut alt = Timeline::new(
        "Square cut",
        Format {
            width: 1080,
            height: 1080,
            ..project.default_format
        },
    );
    alt.audio_tracks
        .push(Arc::new(Track::new(TrackKind::Audio, "Empty")));
    project.timelines = vec![Arc::new(main), Arc::new(alt)];
    project.validate().expect("fixture is valid");
    project
}

fn temp() -> tempfile::TempDir {
    tempfile::tempdir().expect("temp dir")
}

#[test]
fn save_then_load_round_trips() {
    let dir = temp();
    let file = dir.path().join("Film.thira");
    let original = project(&dir.path().join("media"), 50);
    save(&original, &file).expect("saves");
    assert_eq!(load(&file).expect("loads"), original);
    assert!(
        !dir.path().join("Film.thira.tmp").exists(),
        "temporary file removed"
    );
    assert!(
        !dir.path().join("Film.thira-journal").exists(),
        "no journal left behind"
    );
}

#[test]
fn saving_again_replaces_the_previous_version() {
    let dir = temp();
    let file = dir.path().join("Film.thira");
    let mut p = project(dir.path(), 5);
    save(&p, &file).expect("first save");
    p.name = "Second".into();
    p.timelines.pop();
    // A stale temp file from an earlier crash is overwritten, not loaded.
    fs::write(dir.path().join("Film.thira.tmp"), b"junk").expect("write junk");
    save(&p, &file).expect("second save");
    assert_eq!(load(&file).expect("loads"), p);
}

#[test]
fn loading_never_modifies_the_file() {
    let dir = temp();
    let file = dir.path().join("Film.thira");
    save(&project(dir.path(), 5), &file).expect("saves");
    let before = fs::read(&file).expect("read");
    load(&file).expect("loads");
    assert_eq!(fs::read(&file).expect("read"), before);
}

#[test]
fn rejects_files_that_are_not_projects() {
    let dir = temp();
    let text = dir.path().join("notes.thira");
    fs::write(
        &text,
        "hello, this is not a database at all, just some text",
    )
    .expect("write");
    assert!(
        matches!(load(&text), Err(StoreError::NotAProject)),
        "{:?}",
        load(&text)
    );

    let other = dir.path().join("other.db");
    rusqlite::Connection::open(&other)
        .and_then(|c| c.execute_batch("CREATE TABLE t (x); PRAGMA application_id = 42;"))
        .expect("other sqlite file");
    assert!(matches!(load(&other), Err(StoreError::NotAProject)));
}

#[test]
fn rejects_projects_from_a_newer_version() {
    let dir = temp();
    let file = dir.path().join("Film.thira");
    save(&project(dir.path(), 1), &file).expect("saves");
    rusqlite::Connection::open(&file)
        .and_then(|c| c.pragma_update(None, "user_version", SCHEMA_VERSION + 1))
        .expect("bump version");
    match load(&file) {
        Err(StoreError::TooNew { found, supported }) => {
            assert_eq!((found, supported), (SCHEMA_VERSION + 1, SCHEMA_VERSION));
        }
        other => panic!("expected TooNew, got {other:?}"),
    }
}

#[test]
fn reports_damaged_values_instead_of_panicking() {
    let dir = temp();
    let file = dir.path().join("Film.thira");
    save(&project(dir.path(), 3), &file).expect("saves");
    rusqlite::Connection::open(&file)
        .and_then(|c| c.execute("UPDATE clip SET duration_den = 0", []))
        .expect("damage");
    assert!(
        matches!(load(&file), Err(StoreError::Corrupt(_))),
        "{:?}",
        load(&file)
    );
}

#[test]
fn media_follows_a_moved_project_folder() {
    let dir = temp();
    let old_root = dir.path().join("job");
    fs::create_dir_all(old_root.join("media")).expect("mkdir");
    let p = project(&old_root.join("media"), 2);
    for m in p.media.values() {
        fs::write(&m.path, b"x").expect("create media file");
    }
    save(&p, &old_root.join("Film.thira")).expect("saves");

    let new_root = dir.path().join("archive-2026");
    fs::rename(&old_root, &new_root).expect("move folder");
    let loaded = load(&new_root.join("Film.thira")).expect("loads");
    for m in loaded.media.values() {
        assert!(
            m.path.starts_with(&new_root),
            "{} not relinked",
            m.path.display()
        );
        assert!(m.path.exists());
    }
}

#[test]
fn missing_media_keeps_its_stored_path() {
    let dir = temp();
    let file = dir.path().join("Film.thira");
    let p = project(&dir.path().join("gone"), 1);
    save(&p, &file).expect("saves");
    let loaded = load(&file).expect("loads");
    let paths = |p: &Project| p.media.values().map(|m| m.path.clone()).collect::<Vec<_>>();
    assert_eq!(paths(&loaded), paths(&p));
}

const CRASH_TARGET: &str = "THIRA_STORE_CRASH_TARGET";

/// Kills a process that is saving in a loop at many different moments; the project
/// must load cleanly every time (PRD: no corruption on forced exit).
#[test]
fn forced_kill_mid_save_never_corrupts() {
    if let Ok(target) = std::env::var(CRASH_TARGET) {
        save_forever(Path::new(&target));
    }

    let dir = temp();
    let target = dir.path().join("Crash.thira");
    let mut base = project(dir.path(), 2000);
    base.name = "gen 0".into();
    save(&base, &target).expect("initial save");

    let exe = std::env::current_exe().expect("test binary");
    let mut newest = 0;
    for round in 0..12_u64 {
        let mut child = Command::new(&exe)
            .args([
                "forced_kill_mid_save_never_corrupts",
                "--exact",
                "--nocapture",
            ])
            .env(CRASH_TARGET, &target)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn saver");
        // Spread the kills over different points of the save cycle.
        std::thread::sleep(Duration::from_millis(150 + round * 47 % 400));
        child.kill().expect("kill");
        child.wait().expect("reap");

        let loaded =
            load(&target).unwrap_or_else(|e| panic!("round {round}: corrupt after kill: {e}"));
        let generation: u64 = loaded
            .name
            .strip_prefix("gen ")
            .and_then(|n| n.parse().ok())
            .unwrap_or_else(|| panic!("round {round}: unexpected name {:?}", loaded.name));
        newest = newest.max(generation);
        let mut expected = base.clone();
        expected.name = loaded.name.clone();
        assert_eq!(
            loaded, expected,
            "round {round}: contents differ from a complete save"
        );
    }
    assert!(
        newest > 0,
        "the child never completed a save, so the test proved nothing"
    );
}

/// Child process body: save new generations until killed.
fn save_forever(target: &Path) -> ! {
    // Start from the parent's project so every generation differs only by name.
    let mut project = load(target).expect("child loads base");
    let mut generation = 0_u64;
    loop {
        generation += 1;
        project.name = format!("gen {generation}");
        save(&project, target).expect("child save");
    }
}
