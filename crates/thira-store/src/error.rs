use std::path::PathBuf;

use thira_model::ModelError;
use thiserror::Error;

/// Why a project could not be saved or loaded.
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("file error: {0}")]
    Io(#[from] std::io::Error),

    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("not a Thira project file")]
    NotAProject,

    #[error(
        "project was saved by a newer Thira (format {found}; this build reads up to {supported})"
    )]
    TooNew { found: i64, supported: i64 },

    #[error("project file is damaged: {0}")]
    Corrupt(String),

    #[error("project contents are inconsistent: {0}")]
    Invalid(#[from] ModelError),

    #[error("path is not valid Unicode and cannot be stored: {}", .0.display())]
    NonUnicodePath(PathBuf),
}

impl StoreError {
    pub(crate) fn corrupt(what: impl std::fmt::Display) -> Self {
        Self::Corrupt(what.to_string())
    }
}
