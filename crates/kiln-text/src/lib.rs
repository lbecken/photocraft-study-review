//! # kiln-text — layer L0
//!
//! Responsibility (CHARTER §3): the text buffer (a rope), positions (byte, char, UTF-16,
//! line/column), edits, line endings, encodings and operation-based undo/redo.
//!
//! May depend on: nothing in the workspace (lowest layer). Headless and deterministic.
//!
//! Current slice: [`Buffer`] over a `ropey` rope, addressed by char indices, with exact
//! position conversions, line access split on LF, CRLF and lone CR, line-ending style, and
//! operation-based undo/redo.
//!
//! ## Undo grouping
//! Every successful edit that changes the text is one undo step, except that an `insert` joins
//! the previous step iff: that step consists only of inserts of exactly one non-line-break char
//! each, joined by this rule; the new insert is exactly one char that is not `\n` or `\r`; it
//! lands right after the previous step's last inserted char; and no `delete`, `replace`,
//! successful `undo`/`redo`, or `break_undo_group` happened since. No-op edits (`insert` of
//! `""`, `delete`/`replace` of an empty range with `""`) and rejected edits change neither the
//! text nor the history, and do not end a typing group. Any successful edit clears redo.
//!
//! A call that fails changes nothing at all, the open typing group included: an edit that
//! returns `Err` and a `redo` that returns `false` leave the group open, so the next
//! qualifying single-char insert still joins it. For example, after `insert(0, "a")`,
//! `insert(1, "b")`, a rejected `insert(99, "x")` and `insert(2, "c")`, one `undo` removes
//! `"abc"`. (`undo` cannot return `false` while a group is open: an open group is itself an
//! undo step, so `undo` undoes it.)
//!
//! ## Edit validation
//! Indices are validated before anything else, emptiness included (see [`EditError`]): a
//! range with `start > end` is `InvertedRange`, even when a bound is also out of range;
//! otherwise the first of `start`, `end` beyond `len_chars()` is `OutOfBounds`. An edit that
//! would be a no-op at an index beyond `len_chars()` (for example `insert(9, "")` on a
//! 3-char buffer) is therefore `Err(OutOfBounds)`, not `Ok`.
//!
//! ## Selection, anchors and batches (T-018)
//! A buffer has one [`Selection`] and any number of anchors ([`Buffer::add_anchor`]); both
//! follow every primitive edit by the Mapping rule (task T-018: `p < s` stays, `p > e` shifts,
//! `p == s < e` goes to `s`, otherwise to `s + n`; then out of a CRLF to before its `\r`). Setting the selection or
//! adding/removing an anchor is not an edit: it changes neither the history nor an open typing
//! group. [`Buffer::apply_edits`] applies a sorted list of non-overlapping edits as one undo
//! step that ends any open typing group and is never joined; undoing it applies the inverse
//! edits from the highest position to the lowest. [`Buffer::normalize_line_endings`] is one
//! such batch with one edit per line break that differs from the target.

mod buffer;
mod history;
mod line_ending;
mod position;
mod selection;

pub use buffer::{BatchError, Buffer, EditError};
pub use line_ending::{LineEnding, LineEndings};
pub use position::{LineCol, PositionError};
pub use selection::{AnchorId, Movement, Selection};
