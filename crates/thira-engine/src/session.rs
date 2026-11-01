use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use thira_edit::Editor;
use thira_model::{Format, FrameRate, Project, Timeline, Track, TrackKind};

use crate::EngineError;

/// One open project.
#[derive(Debug)]
pub struct Session {
    editor: Editor,
    path: Option<PathBuf>,
    /// Editor version at the last save or open.
    saved_version: u64,
}

/// What the UI shows about the open project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectInfo {
    pub name: String,
    /// `None` until the project is first saved.
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub version: u64,
    pub undo_label: Option<String>,
    pub redo_label: Option<String>,
}

impl Session {
    /// A new, unsaved project with one timeline holding a video and an audio track.
    pub fn create(name: &str) -> Self {
        let format = Format::hd_1080(FrameRate::FPS_30);
        let mut project = Project::new(name, format);
        let mut timeline = Timeline::new("Timeline 1", format);
        timeline
            .video_tracks
            .push(Arc::new(Track::new(TrackKind::Video, "V1")));
        timeline
            .audio_tracks
            .push(Arc::new(Track::new(TrackKind::Audio, "A1")));
        project.timelines.push(Arc::new(timeline));
        Self::from_project(project, None).unwrap_or_else(|_| unreachable!("new project is valid"))
    }

    /// Opens a project file.
    pub fn open(path: &Path) -> Result<Self, EngineError> {
        let path = std::path::absolute(path).map_err(thira_store::StoreError::from)?;
        let project = thira_store::load(&path)?;
        Self::from_project(project, Some(path))
    }

    fn from_project(project: Project, path: Option<PathBuf>) -> Result<Self, EngineError> {
        let editor = Editor::new(project)?;
        Ok(Self {
            saved_version: editor.version(),
            editor,
            path,
        })
    }

    /// Saves to the file the project was opened from or last saved to.
    pub fn save(&mut self) -> Result<(), EngineError> {
        let path = self.path.clone().ok_or(EngineError::NoPath)?;
        thira_store::save(self.editor.project(), &path)?;
        self.saved_version = self.editor.version();
        Ok(())
    }

    /// Saves to `path`, adding the `.thira` extension if missing, and makes it the
    /// project's file.
    pub fn save_as(&mut self, path: &Path) -> Result<(), EngineError> {
        let mut path = std::path::absolute(path).map_err(thira_store::StoreError::from)?;
        if path
            .extension()
            .is_none_or(|ext| ext != thira_store::EXTENSION)
        {
            let mut name = path.file_name().unwrap_or_default().to_os_string();
            name.push(".");
            name.push(thira_store::EXTENSION);
            path.set_file_name(name);
        }
        thira_store::save(self.editor.project(), &path)?;
        self.path = Some(path);
        self.saved_version = self.editor.version();
        Ok(())
    }

    /// True if anything was applied, undone or redone since the last save or open.
    pub fn is_dirty(&self) -> bool {
        self.editor.version() != self.saved_version
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn editor(&self) -> &Editor {
        &self.editor
    }

    pub fn editor_mut(&mut self) -> &mut Editor {
        &mut self.editor
    }

    pub fn info(&self) -> ProjectInfo {
        ProjectInfo {
            name: self.editor.project().name.clone(),
            path: self.path.clone(),
            dirty: self.is_dirty(),
            version: self.editor.version(),
            undo_label: self.editor.undo_label().map(str::to_owned),
            redo_label: self.editor.redo_label().map(str::to_owned),
        }
    }
}
