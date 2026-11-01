//! The engine facade (ADR 0001).
//!
//! [`Session`] is one open project: the editor with its undo history, the file it
//! came from, and whether it has unsaved changes. [`Engine`] runs at most one
//! session on a dedicated engine thread; the UI shell talks to it through an
//! [`EngineHandle`], so all edits are applied in order by a single writer.

mod engine;
mod error;
mod session;

pub use engine::{Engine, EngineHandle};
pub use error::EngineError;
pub use session::{ProjectInfo, Session};
