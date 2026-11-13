//! Operation-based undo/redo history (CHARTER §3 decision 3).
//!
//! Each undo step is a [`Step`]: one or more [`Op`]s, each "at char `at`, `removed` was
//! replaced by `inserted`". Undo applies the inverse edits, redo re-applies them; no snapshot
//! of the text is ever stored. A typing group is a one-`Op` step whose `inserted` grows by one
//! char per joined insert: the joined inserts are contiguous by the grouping rule, so
//! concatenation is exact. A batch (`Buffer::apply_edits`) is one step of several ops.

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

/// One undo step: its ops in increasing `at` order, non-overlapping, each `at` given in the
/// text *before* the step. The step was applied (and is redone) from the last op to the
/// first, so each op's `at` is exact when it is applied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Step {
    pub(crate) ops: Vec<Op>,
}

impl Step {
    /// Each op's start in the text *after* the step: `at` shifted by the size changes of the
    /// ops before it. Undo applies the inverse of op `i` at that position.
    pub(crate) fn starts_after(&self) -> Vec<usize> {
        let (mut added, mut removed) = (0usize, 0usize);
        self.ops
            .iter()
            .map(|op| {
                // `removed` <= `op.at`: the earlier ops lie before this one.
                let at = op.at + added - removed;
                added += op.inserted_chars;
                removed += op.removed_chars;
                at
            })
            .collect()
    }
}

/// Undo and redo stacks plus whether the top undo step is an open typing group.
#[derive(Clone, Debug, Default)]
pub(crate) struct History {
    undo: Vec<Step>,
    redo: Vec<Step>,
    /// True iff the top undo step consists only of single-char typing inserts and nothing has
    /// ended the group since (delete, replace, batch, undo, redo, `break_undo_group`, a
    /// non-typing insert).
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
            && let [top] = top.ops.as_mut_slice()
            && top.at + top.inserted_chars == op.at
        {
            top.inserted.push_str(&op.inserted);
            top.inserted_chars += op.inserted_chars;
            return;
        }
        self.undo.push(Step { ops: vec![op] });
        self.typing_open = typing;
    }

    /// Record a batch of at least one op as one step; it ends the typing group and is never
    /// joined.
    pub(crate) fn record_batch(&mut self, ops: Vec<Op>) {
        debug_assert!(!ops.is_empty());
        self.redo.clear();
        self.undo.push(Step { ops });
        self.typing_open = false;
    }

    /// Take the step to undo; the caller applies its inverse and hands it to [`Self::undone`].
    pub(crate) fn pop_undo(&mut self) -> Option<Step> {
        let step = self.undo.pop()?;
        self.typing_open = false;
        Some(step)
    }

    pub(crate) fn undone(&mut self, step: Step) {
        self.redo.push(step);
    }

    /// Take the step to redo (an empty redo stack changes nothing, so an open
    /// typing group stays open); the caller re-applies it and hands it to [`Self::redone`].
    pub(crate) fn pop_redo(&mut self) -> Option<Step> {
        let step = self.redo.pop()?;
        self.typing_open = false;
        Some(step)
    }

    pub(crate) fn redone(&mut self, step: Step) {
        self.undo.push(step);
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

    /// Number of undo and redo steps, and whether a typing group is open (for tests).
    #[cfg(test)]
    pub(crate) fn shape(&self) -> (usize, usize, bool) {
        (self.undo.len(), self.redo.len(), self.typing_open)
    }
}
