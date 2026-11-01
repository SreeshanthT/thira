use thira_model::ModelError;
use thiserror::Error;

/// Why a command or transaction could not be applied. The project is left unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EditError {
    #[error("{kind} {id} not found")]
    NotFound { kind: &'static str, id: String },

    #[error("{kind} {id} already exists")]
    AlreadyExists { kind: &'static str, id: String },

    #[error("index {index} is out of range")]
    IndexOutOfRange { index: usize },

    #[error("the edit would leave the project invalid: {0}")]
    Invalid(#[from] ModelError),
}

impl EditError {
    pub(crate) fn not_found(kind: &'static str, id: impl ToString) -> Self {
        Self::NotFound {
            kind,
            id: id.to_string(),
        }
    }

    pub(crate) fn exists(kind: &'static str, id: impl ToString) -> Self {
        Self::AlreadyExists {
            kind,
            id: id.to_string(),
        }
    }
}
