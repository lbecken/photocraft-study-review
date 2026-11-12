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

mod buffer;
mod history;
mod line_ending;
mod position;

pub use buffer::{Buffer, EditError};
pub use line_ending::{LineEnding, LineEndings};
pub use position::{LineCol, PositionError};
