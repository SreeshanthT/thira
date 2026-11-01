use serde::{Serialize, Serializer};
use thira_edit::EditError;
use thira_store::StoreError;
use thiserror::Error;

/// Why an engine request failed.
#[derive(Debug, Error)]
pub enum EngineError {
    #[error("no project is open")]
    NoProject,

    #[error("the project has unsaved changes")]
    Unsaved,

    #[error("the project has not been saved yet; choose a file name")]
    NoPath,

    #[error(transparent)]
    Edit(#[from] EditError),

    #[error(transparent)]
    Store(#[from] StoreError),

    #[error("the engine thread has stopped")]
    Stopped,
}

impl EngineError {
    /// Stable identifier for the UI, so it can react to specific errors
    /// (for example, asking to save on `unsaved`).
    pub fn code(&self) -> &'static str {
        match self {
            Self::NoProject => "no_project",
            Self::Unsaved => "unsaved",
            Self::NoPath => "no_path",
            Self::Edit(_) => "edit",
            Self::Store(StoreError::NotAProject) => "not_a_project",
            Self::Store(StoreError::TooNew { .. }) => "too_new",
            Self::Store(_) => "store",
            Self::Stopped => "stopped",
        }
    }
}

/// Serializes as `{ "code": "...", "message": "..." }` for the UI.
impl Serialize for EngineError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("EngineError", 2)?;
        s.serialize_field("code", self.code())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}
