//! Operation-based undo/redo history (CHARTER §3 decision 3).
//!
//! Each undo step is one [`Op`]: "at char `at`, `removed` was replaced by `inserted`". Undo
//! applies the inverse edit, redo re-applies it; no snapshot of the text is ever stored. A
//! typing group is one `Op` whose `inserted` grows by one char per joined insert: the joined
//! inserts are contiguous by the grouping rule, so concatenation is exact.

/// One recorded edit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Op {
    /// Char index where the edit starts.
    pub(crate) at: usize,
    /// Text that was removed at `at`.
    pub(crate) removed: String,
    /// Text that was inserted at `at`.
    pub(crate) inserted: String,
    /// `removed.chars().count()`.
    pub(crate) removed_chars: usize,
    /// `inserted.chars().count()`.
    pub(crate) inserted_chars: usize,
}

impl Op {
    pub(crate) fn new(at: usize, removed: String, inserted: &str) -> Self {
        Self {
            at,
            removed_chars: removed.chars().count(),
            removed,
            inserted_chars: inserted.chars().count(),
            inserted: inserted.to_owned(),
        }
    }
}

/// Undo and redo stacks plus whether the top undo step is an open typing group.
#[derive(Clone, Debug, Default)]
pub(crate) struct History {
    undo: Vec<Op>,
    redo: Vec<Op>,
    /// True iff the top undo step consists only of single-char typing inserts and nothing has
    /// ended the group since (delete, replace, undo, redo, `break_undo_group`, a non-typing
    /// insert).
    typing_open: bool,
}

impl History {
    /// Record a successful non-no-op edit. `typing` is true for an `insert` of exactly one char
    /// that is not a line break; such an edit joins the open typing group when it continues it.
    pub(crate) fn record(&mut self, op: Op, typing: bool) {
        self.redo.clear();
        if typing
            && self.typing_open
            && let Some(top) = self.undo.last_mut()
            && top.at + top.inserted_chars == op.at
        {
            top.inserted.push_str(&op.inserted);
            top.inserted_chars += op.inserted_chars;
            return;
        }
        self.undo.push(op);
        self.typing_open = typing;
    }

    /// Take the step to undo; the caller applies its inverse and hands it to [`Self::undone`].
    pub(crate) fn pop_undo(&mut self) -> Option<Op> {
        let op = self.undo.pop()?;
        self.typing_open = false;
        Some(op)
    }

    pub(crate) fn undone(&mut self, op: Op) {
        self.redo.push(op);
    }

    /// Take the step to redo (an empty redo stack changes nothing, so an open
    /// typing group stays open); the caller re-applies it and hands it to [`Self::redone`].
    pub(crate) fn pop_redo(&mut self) -> Option<Op> {
        let op = self.redo.pop()?;
        self.typing_open = false;
        Some(op)
    }

    pub(crate) fn redone(&mut self, op: Op) {
        self.undo.push(op);
    }

    pub(crate) fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub(crate) fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub(crate) fn break_group(&mut self) {
        self.typing_open = false;
    }
}
