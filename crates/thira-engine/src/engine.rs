use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::thread;

use thira_edit::{ModelChanged, Transaction};
use thira_model::Project;

use crate::{EngineError, ProjectInfo, Session};

/// Work for the engine thread: runs with exclusive access to the session slot.
type Job = Box<dyn FnOnce(&mut Option<Session>) + Send>;

/// Starts the engine thread.
#[derive(Debug)]
pub struct Engine;

impl Engine {
    /// Spawns the engine thread. It stops when every [`EngineHandle`] is dropped.
    ///
    /// # Panics
    /// Panics if the OS cannot create a thread.
    pub fn spawn() -> EngineHandle {
        let (tx, rx) = mpsc::channel::<Job>();
        thread::Builder::new()
            .name("thira-engine".into())
            .spawn(move || {
                let mut session = None;
                for job in rx {
                    job(&mut session);
                }
            })
            .expect("failed to spawn the engine thread");
        EngineHandle { tx }
    }
}

/// Cloneable, thread-safe handle for sending requests to the engine thread.
/// Each call blocks until the engine has handled it.
#[derive(Debug, Clone)]
pub struct EngineHandle {
    tx: Sender<Job>,
}

impl EngineHandle {
    /// Runs `f` on the engine thread and returns its result.
    fn call<R: Send + 'static>(
        &self,
        f: impl FnOnce(&mut Option<Session>) -> R + Send + 'static,
    ) -> Result<R, EngineError> {
        let (reply, result) = mpsc::channel();
        self.tx
            .send(Box::new(move |session| {
                // The caller may have given up waiting; nothing to do then.
                let _ = reply.send(f(session));
            }))
            .map_err(|_| EngineError::Stopped)?;
        result.recv().map_err(|_| EngineError::Stopped)
    }

    /// Runs `f` on the open session.
    fn with_session<R: Send + 'static>(
        &self,
        f: impl FnOnce(&mut Session) -> Result<R, EngineError> + Send + 'static,
    ) -> Result<R, EngineError> {
        self.call(|slot| slot.as_mut().ok_or(EngineError::NoProject).and_then(f))?
    }

    /// Replaces the open project, refusing if it has unsaved changes and `force` is false.
    fn replace(
        &self,
        force: bool,
        make: impl FnOnce() -> Result<Option<Session>, EngineError> + Send + 'static,
    ) -> Result<Option<ProjectInfo>, EngineError> {
        self.call(move |slot| {
            if !force && slot.as_ref().is_some_and(Session::is_dirty) {
                return Err(EngineError::Unsaved);
            }
            // Build the new session first, so a failed open keeps the current project.
            *slot = make()?;
            Ok(slot.as_ref().map(Session::info))
        })?
    }

    /// Starts a new, unsaved project.
    pub fn new_project(&self, name: String, force: bool) -> Result<ProjectInfo, EngineError> {
        self.replace(force, move || Ok(Some(Session::create(&name))))?
            .ok_or(EngineError::NoProject)
    }

    /// Opens a project file.
    pub fn open(&self, path: PathBuf, force: bool) -> Result<ProjectInfo, EngineError> {
        self.replace(force, move || Session::open(&path).map(Some))?
            .ok_or(EngineError::NoProject)
    }

    /// Closes the open project.
    pub fn close(&self, force: bool) -> Result<(), EngineError> {
        self.replace(force, || Ok(None)).map(|_| ())
    }

    pub fn save(&self) -> Result<ProjectInfo, EngineError> {
        self.with_session(|s| s.save().map(|()| s.info()))
    }

    pub fn save_as(&self, path: PathBuf) -> Result<ProjectInfo, EngineError> {
        self.with_session(move |s| s.save_as(&path).map(|()| s.info()))
    }

    /// Information about the open project, or `None` if none is open.
    pub fn info(&self) -> Result<Option<ProjectInfo>, EngineError> {
        self.call(|slot| slot.as_ref().map(Session::info))
    }

    /// A snapshot of the open project (cheap; see `thira_model`).
    pub fn snapshot(&self) -> Result<Project, EngineError> {
        self.with_session(|s| Ok(s.editor().snapshot()))
    }

    pub fn apply(&self, transaction: Transaction) -> Result<ModelChanged, EngineError> {
        self.with_session(move |s| Ok(s.editor_mut().apply(transaction)?))
    }

    /// Returns `None` when there is nothing to undo.
    pub fn undo(&self) -> Result<Option<ModelChanged>, EngineError> {
        self.with_session(|s| Ok(s.editor_mut().undo()?))
    }

    /// Returns `None` when there is nothing to redo.
    pub fn redo(&self) -> Result<Option<ModelChanged>, EngineError> {
        self.with_session(|s| Ok(s.editor_mut().redo()?))
    }
}
