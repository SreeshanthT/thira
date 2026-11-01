//! Every change to a project goes through here.
//!
//! - [`Command`]: a primitive edit (add a clip, rename a track, …) that is plain
//!   serializable data, so the UI and scripts can send it.
//! - [`Transaction`]: commands applied together, all or nothing.
//! - [`CommandBus`]: owns the [`Project`](thira_model::Project). Applying a
//!   transaction validates the result, bumps the version, and returns the
//!   inverse transaction plus the set of changed objects for the UI.
//! - [`Editor`]: the bus plus unlimited undo and redo, with grouping.
//!
//! Higher-level edits (ripple delete, trims) are built as transactions of
//! primitives, so their inverses are exact by construction.

mod bus;
mod command;
mod error;
mod history;

pub use bus::{Applied, Change, CommandBus, ModelChanged};
pub use command::{Command, Transaction};
pub use error::EditError;
pub use history::Editor;
