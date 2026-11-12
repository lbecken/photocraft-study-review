//! # kiln-text — layer L0
//!
//! Responsibility (CHARTER §3): the text buffer (a rope, eventually), positions (byte, char,
//! UTF-16, line/column), edits, line endings, encodings and operation-based undo/redo.
//!
//! May depend on: nothing in the workspace (lowest layer). Headless and deterministic.
//!
//! Current slice: [`Buffer`] with char-indexed insert and delete. The public API is expressed
//! in char indices and owned `String` results so that the storage can become a rope without
//! changing callers.

mod buffer;

pub use buffer::{Buffer, EditError};
