use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thira_model::{ClipId, MediaId, Project, TimelineId, TrackId};

use crate::{EditError, Transaction};

/// An object whose state changed. The UI re-reads it, or drops it if it no longer exists.
///
/// Adding or removing a timeline or track lists only that object, not the tracks and
/// clips inside it; the UI treats the whole subtree as replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum Change {
    /// Project-level fields: name, default format.
    Project,
    Media(MediaId),
    /// Timeline fields or its track lists.
    Timeline(TimelineId),
    /// Track fields or its clip list.
    Track(TrackId),
    Clip(ClipId),
}

/// Event sent to the UI after each applied transaction (ADR 0001).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelChanged {
    /// Increases by one per applied transaction. A gap means the UI missed an event.
    pub version: u64,
    pub changes: Vec<Change>,
}

/// Result of a successfully applied transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    /// Applying this restores the previous state exactly.
    pub inverse: Transaction,
    pub event: ModelChanged,
}

/// Sole owner of the project. All edits go through [`CommandBus::apply`].
#[derive(Debug, Clone)]
pub struct CommandBus {
    project: Project,
    version: u64,
}

impl CommandBus {
    /// Takes ownership of a project, which must be valid.
    pub fn new(project: Project) -> Result<Self, EditError> {
        project.validate()?;
        Ok(Self {
            project,
            version: 0,
        })
    }

    /// Read-only view of the current project.
    pub fn project(&self) -> &Project {
        &self.project
    }

    /// A cheap copy of the current project for other threads. Later edits do not affect it.
    pub fn snapshot(&self) -> Project {
        self.project.clone()
    }

    /// Number of transactions applied so far.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Applies all commands in order, then validates the project.
    ///
    /// If any command fails or the result is invalid, the project is restored and
    /// the version is unchanged. An empty transaction changes nothing and does not
    /// bump the version.
    pub fn apply(&mut self, transaction: Transaction) -> Result<Applied, EditError> {
        let Transaction { label, commands } = transaction;
        if commands.is_empty() {
            return Ok(Applied {
                inverse: Transaction::new(label, Vec::new()),
                event: ModelChanged {
                    version: self.version,
                    changes: Vec::new(),
                },
            });
        }

        // Restoring this snapshot undoes a partial failure exactly; it shares all
        // unchanged data, so taking it is cheap.
        let before = self.project.clone();
        let mut changes = BTreeSet::new();
        let mut inverses = Vec::with_capacity(commands.len());
        let result = commands
            .into_iter()
            .try_for_each(|command| {
                inverses.push(command.apply(&mut self.project, &mut changes)?);
                Ok(())
            })
            .and_then(|()| self.project.validate().map_err(EditError::from));
        if let Err(err) = result {
            self.project = before;
            return Err(err);
        }

        inverses.reverse();
        self.version += 1;
        Ok(Applied {
            inverse: Transaction::new(label, inverses),
            event: ModelChanged {
                version: self.version,
                changes: changes.into_iter().collect(),
            },
        })
    }
}
