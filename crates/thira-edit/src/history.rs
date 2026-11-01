use thira_model::Project;

use crate::{CommandBus, EditError, ModelChanged, Transaction};

/// The command bus plus unlimited undo and redo for one editing session (PRD TL-6).
///
/// Each undo step is one transaction, or one group of transactions opened with
/// [`Editor::begin_group`] (for example, every intermediate position of a drag).
#[derive(Debug, Clone)]
pub struct Editor {
    bus: CommandBus,
    /// Transactions that undo each step, most recent last.
    undo: Vec<Transaction>,
    /// Transactions that redo each undone step, most recently undone last.
    redo: Vec<Transaction>,
    group: Option<Group>,
}

#[derive(Debug, Clone)]
struct Group {
    depth: usize,
    inverse: Transaction,
}

impl Editor {
    /// Starts a session on a valid project with empty history.
    pub fn new(project: Project) -> Result<Self, EditError> {
        Ok(Self {
            bus: CommandBus::new(project)?,
            undo: Vec::new(),
            redo: Vec::new(),
            group: None,
        })
    }

    pub fn project(&self) -> &Project {
        self.bus.project()
    }

    /// See [`CommandBus::snapshot`].
    pub fn snapshot(&self) -> Project {
        self.bus.snapshot()
    }

    /// See [`CommandBus::version`]. Undo and redo also bump it.
    pub fn version(&self) -> u64 {
        self.bus.version()
    }

    /// Applies a transaction and records it as an undo step. Clears the redo stack.
    /// A failed or empty transaction leaves the history unchanged.
    pub fn apply(&mut self, transaction: Transaction) -> Result<ModelChanged, EditError> {
        let applied = self.bus.apply(transaction)?;
        if applied.inverse.commands.is_empty() {
            return Ok(applied.event);
        }
        self.redo.clear();
        match &mut self.group {
            // Later edits must be undone first, so their inverses go in front.
            Some(group) => {
                let mut commands = applied.inverse.commands;
                commands.append(&mut group.inverse.commands);
                group.inverse.commands = commands;
            }
            None => self.undo.push(applied.inverse),
        }
        Ok(applied.event)
    }

    /// Starts grouping: transactions applied until the matching [`Editor::end_group`]
    /// become one undo step labelled `label`. Groups may nest; the outermost label wins.
    pub fn begin_group(&mut self, label: impl Into<String>) {
        match &mut self.group {
            Some(group) => group.depth += 1,
            None => {
                self.group = Some(Group {
                    depth: 1,
                    inverse: Transaction::new(label, Vec::new()),
                });
            }
        }
    }

    /// Closes the innermost group. Closing the outermost one records the undo step.
    /// Does nothing if no group is open.
    pub fn end_group(&mut self) {
        let Some(group) = &mut self.group else { return };
        group.depth -= 1;
        if group.depth == 0
            && let Some(group) = self.group.take()
            && !group.inverse.commands.is_empty()
        {
            self.undo.push(group.inverse);
        }
    }

    /// Undoes the most recent step. Closes any open group first.
    /// Returns `None` if there is nothing to undo.
    pub fn undo(&mut self) -> Result<Option<ModelChanged>, EditError> {
        self.close_groups();
        let Some(inverse) = self.undo.pop() else {
            return Ok(None);
        };
        match self.bus.apply(inverse.clone()) {
            Ok(applied) => {
                self.redo.push(applied.inverse);
                Ok(Some(applied.event))
            }
            Err(err) => {
                // Cannot happen while all edits go through this editor; keep the step.
                self.undo.push(inverse);
                Err(err)
            }
        }
    }

    /// Redoes the most recently undone step. Returns `None` if there is nothing to redo.
    pub fn redo(&mut self) -> Result<Option<ModelChanged>, EditError> {
        self.close_groups();
        let Some(forward) = self.redo.pop() else {
            return Ok(None);
        };
        match self.bus.apply(forward.clone()) {
            Ok(applied) => {
                self.undo.push(applied.inverse);
                Ok(Some(applied.event))
            }
            Err(err) => {
                self.redo.push(forward);
                Err(err)
            }
        }
    }

    /// Label of the step [`Editor::undo`] would undo, for the Edit menu.
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|t| t.label.as_str())
    }

    /// Label of the step [`Editor::redo`] would redo.
    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|t| t.label.as_str())
    }

    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_depth(&self) -> usize {
        self.redo.len()
    }

    fn close_groups(&mut self) {
        while self.group.is_some() {
            self.end_group();
        }
    }
}
