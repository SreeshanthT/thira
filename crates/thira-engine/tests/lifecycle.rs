//! New / open / save / close through the engine thread (issue #10).

use std::path::PathBuf;

use thira_edit::{Command, Transaction};
use thira_engine::{Engine, EngineError, EngineHandle};
use thira_model::{AudioStream, Clip, FrameRate, MediaId, MediaKind, MediaRef, Time, VideoStream};

fn edit_something(engine: &EngineHandle) {
    let project = engine.snapshot().expect("snapshot");
    let rate = FrameRate::FPS_30;
    let media = MediaRef {
        id: MediaId::new(),
        name: "a.mp4".into(),
        path: PathBuf::from("a.mp4"),
        kind: MediaKind::Video,
        duration: Some(Time::from_frame(900, rate)),
        video: Some(VideoStream {
            width: 1920,
            height: 1080,
            frame_rate: Some(rate),
        }),
        audio: Some(AudioStream {
            channels: 2,
            sample_rate: 48_000,
        }),
    };
    let v1 = project.timelines[0].video_tracks[0].id;
    let clip = Clip::new(
        media.id,
        Time::ZERO,
        Time::from_frame(30, rate),
        Time::from_frame(90, rate),
    );
    engine
        .apply(Transaction::new(
            "Add media and clip",
            vec![
                Command::SetProjectName {
                    name: "Holiday".into(),
                },
                Command::AddMedia { media },
                Command::InsertClip { track: v1, clip },
            ],
        ))
        .expect("edit applies");
}

#[test]
fn reopened_project_is_identical_to_the_saved_one() {
    let dir = tempfile::tempdir().expect("temp dir");
    let engine = Engine::spawn();
    engine.new_project("Untitled".into(), false).expect("new");
    edit_something(&engine);
    let saved = engine.snapshot().expect("snapshot");

    let info = engine.save_as(dir.path().join("Holiday")).expect("save as");
    assert_eq!(
        info.path.as_deref(),
        Some(dir.path().join("Holiday.thira").as_path())
    );
    assert!(!info.dirty);
    engine.close(false).expect("close a saved project");
    assert_eq!(engine.info().expect("info"), None);

    let info = engine
        .open(dir.path().join("Holiday.thira"), false)
        .expect("open");
    assert_eq!(info.name, "Holiday");
    assert!(!info.dirty);
    assert_eq!(engine.snapshot().expect("snapshot"), saved);
}

#[test]
fn unsaved_changes_block_close_new_and_open_unless_forced() {
    let dir = tempfile::tempdir().expect("temp dir");
    let engine = Engine::spawn();
    engine.new_project("A".into(), false).expect("new");
    assert!(
        !engine.info().expect("info").expect("open").dirty,
        "a new project starts clean"
    );
    edit_something(&engine);
    assert!(engine.info().expect("info").expect("open").dirty);

    assert!(matches!(engine.close(false), Err(EngineError::Unsaved)));
    assert!(matches!(
        engine.new_project("B".into(), false),
        Err(EngineError::Unsaved)
    ));
    assert!(matches!(
        engine.open(dir.path().join("x.thira"), false),
        Err(EngineError::Unsaved)
    ));
    assert_eq!(
        engine.info().expect("info").map(|i| i.name),
        Some("Holiday".into())
    );

    engine
        .new_project("B".into(), true)
        .expect("forced new discards changes");
    assert_eq!(
        engine.info().expect("info").map(|i| i.name),
        Some("B".into())
    );
}

#[test]
fn save_tracks_dirty_state_and_needs_a_path_first() {
    let dir = tempfile::tempdir().expect("temp dir");
    let engine = Engine::spawn();
    engine.new_project("A".into(), false).expect("new");
    assert!(matches!(engine.save(), Err(EngineError::NoPath)));
    engine.save_as(dir.path().join("a.thira")).expect("save as");

    edit_something(&engine);
    let info = engine.info().expect("info").expect("open");
    assert!(info.dirty);
    assert_eq!(info.undo_label.as_deref(), Some("Add media and clip"));
    assert!(!engine.save().expect("save").dirty);

    // Undo after saving makes the project differ from the file again.
    engine.undo().expect("undo").expect("something to undo");
    assert!(engine.info().expect("info").expect("open").dirty);
}

#[test]
fn failed_open_keeps_the_current_project() {
    let dir = tempfile::tempdir().expect("temp dir");
    let bogus = dir.path().join("bogus.thira");
    std::fs::write(&bogus, "not a project, just text that is long enough").expect("write");

    let engine = Engine::spawn();
    engine.new_project("Keep me".into(), false).expect("new");
    let err = engine.open(bogus, false).expect_err("not a project");
    assert_eq!(err.code(), "not_a_project");
    assert_eq!(
        engine.info().expect("info").map(|i| i.name),
        Some("Keep me".into())
    );
}

#[test]
fn requests_without_a_project_fail_cleanly() {
    let engine = Engine::spawn();
    assert!(matches!(engine.save(), Err(EngineError::NoProject)));
    assert!(matches!(engine.undo(), Err(EngineError::NoProject)));
    assert!(matches!(engine.snapshot(), Err(EngineError::NoProject)));
    engine.close(false).expect("closing nothing is fine");
}

#[test]
fn handles_work_from_many_threads() {
    let engine = Engine::spawn();
    engine.new_project("Shared".into(), false).expect("new");
    let threads: Vec<_> = (0..8)
        .map(|i| {
            let engine = engine.clone();
            std::thread::spawn(move || {
                engine
                    .apply(Transaction::single(
                        "rename",
                        Command::SetProjectName {
                            name: format!("n{i}"),
                        },
                    ))
                    .expect("applies")
            })
        })
        .collect();
    let mut versions: Vec<u64> = threads
        .into_iter()
        .map(|t| t.join().expect("thread").version)
        .collect();
    versions.sort_unstable();
    // The single engine thread applied them one at a time.
    assert_eq!(versions, (1..=8).collect::<Vec<_>>());
}

#[test]
fn errors_serialize_with_a_code_for_the_ui() {
    let json = serde_json::to_string(&EngineError::Unsaved).expect("serializes");
    assert_eq!(
        json,
        r#"{"code":"unsaved","message":"the project has unsaved changes"}"#
    );
}
