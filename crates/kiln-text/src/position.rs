//! Position types and the errors of position conversions and line access.

use std::fmt;

/// Zero-based line and zero-based column; the column counts chars.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LineCol {
    /// Zero-based line index.
    pub line: usize,
    /// Zero-based column, in chars from the start of the line.
    pub col: usize,
}

/// Why a position conversion or a line access was rejected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PositionError {
    /// `index` (in the unit of the call: byte, char or UTF-16) exceeds `len` (same unit).
    OutOfBounds {
        /// The offending index, in the unit of the call.
        index: usize,
        /// The buffer length in the same unit.
        len: usize,
    },
    /// The byte offset falls inside a multi-byte UTF-8 sequence.
    NotCharBoundary {
        /// The offending byte offset.
        byte: usize,
    },
    /// The UTF-16 offset falls between the two halves of a surrogate pair.
    InsideSurrogatePair {
        /// The offending UTF-16 offset.
        utf16: usize,
    },
    /// The char index lies between the `\r` and `\n` of a CRLF (no line/column exists there).
    InsideLineBreak {
        /// The offending char index.
        char_idx: usize,
    },
    /// `line >= len_lines()`.
    LineOutOfBounds {
        /// The requested line.
        line: usize,
        /// The number of lines in the buffer.
        len_lines: usize,
    },
    /// `col` exceeds the length in chars of that line's content.
    ColumnOutOfBounds {
        /// The requested line.
        line: usize,
        /// The requested column.
        col: usize,
        /// Length in chars of the line's content (without its line break).
        line_len: usize,
    },
}

impl fmt::Display for PositionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PositionError::OutOfBounds { index, len } => {
                write!(f, "index {index} is out of bounds (length {len})")
            }
            PositionError::NotCharBoundary { byte } => {
                write!(f, "byte offset {byte} is inside a multi-byte char")
            }
            PositionError::InsideSurrogatePair { utf16 } => {
                write!(f, "UTF-16 offset {utf16} is inside a surrogate pair")
            }
            PositionError::InsideLineBreak { char_idx } => {
                write!(
                    f,
                    "char index {char_idx} is between the \\r and \\n of a CRLF"
                )
            }
            PositionError::LineOutOfBounds { line, len_lines } => {
                write!(
                    f,
                    "line {line} is out of bounds (buffer has {len_lines} lines)"
                )
            }
            PositionError::ColumnOutOfBounds {
                line,
                col,
                line_len,
            } => write!(
                f,
                "column {col} is out of bounds for line {line} ({line_len} chars)"
            ),
        }
    }
}

impl std::error::Error for PositionError {}
