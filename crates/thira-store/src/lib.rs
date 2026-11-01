//! Reads and writes `.thira` project files (ADR 0002).
//!
//! A project file is a SQLite database holding only the project model.
//! [`save`] writes a complete new database next to the target and atomically
//! replaces it, so a crash leaves either the old project or the new one.
//! [`load`] migrates older files in memory and never modifies the file.

mod error;
mod load;
mod paths;
mod save;
mod schema;

pub use error::StoreError;
pub use load::load;
pub use save::save;
pub use schema::{APPLICATION_ID, SCHEMA_VERSION};

/// File extension for Thira projects, without the dot.
pub const EXTENSION: &str = "thira";
