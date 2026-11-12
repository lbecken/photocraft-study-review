//! The editable text buffer.

use std::fmt;
use std::ops::Range;

/// An editable text buffer addressed by **char indices** (Unicode scalar values).
///
/// Storage is a plain `String` today. The API deliberately avoids exposing the storage
/// (no `&str` borrow of the whole text, no byte offsets in edit methods) so a rope can
/// replace it without changing callers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Buffer {
    text: String,
    len_chars: usize,
}

/// Why an edit or read was rejected. The buffer is unchanged when an error is returned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditError {
    /// A char index lies beyond the end of the buffer.
    OutOfBounds {
        /// The offending char index.
        index: usize,
        /// The buffer length in chars at the time of the call.
        len_chars: usize,
    },
    /// A range whose start lies after its end.
    InvertedRange {
        /// Range start (char index).
        start: usize,
        /// Range end (char index).
        end: usize,
    },
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EditError::OutOfBounds { index, len_chars } => {
                write!(
                    f,
                    "char index {index} is out of bounds (buffer has {len_chars} chars)"
                )
            }
            EditError::InvertedRange { start, end } => {
                write!(f, "range start {start} is after range end {end}")
            }
        }
    }
}

impl std::error::Error for EditError {}

impl Buffer {
    /// An empty buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// A buffer holding `text`.
    pub fn from_text(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            len_chars: text.chars().count(),
        }
    }

    /// Length in chars.
    pub fn len_chars(&self) -> usize {
        self.len_chars
    }

    /// Length in UTF-8 bytes.
    pub fn len_bytes(&self) -> usize {
        self.text.len()
    }

    /// True if the buffer holds no text.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// The whole text as an owned string.
    pub fn text(&self) -> String {
        self.text.clone()
    }

    /// The text in the char range `range`.
    pub fn slice(&self, range: Range<usize>) -> Result<String, EditError> {
        let bytes = self.byte_range(range)?;
        Ok(self.text[bytes].to_owned())
    }

    /// Insert `text` before the char at `char_idx` (`char_idx == len_chars()` appends).
    pub fn insert(&mut self, char_idx: usize, text: &str) -> Result<(), EditError> {
        let at = self.byte_of(char_idx)?;
        self.text.insert_str(at, text);
        self.len_chars += text.chars().count();
        Ok(())
    }

    /// Delete the chars in `range` and return the removed text (what an undo operation needs).
    pub fn delete(&mut self, range: Range<usize>) -> Result<String, EditError> {
        let bytes = self.byte_range(range.clone())?;
        let removed: String = self.text.drain(bytes).collect();
        self.len_chars -= range.end - range.start;
        Ok(removed)
    }

    fn byte_range(&self, range: Range<usize>) -> Result<Range<usize>, EditError> {
        if range.start > range.end {
            return Err(EditError::InvertedRange {
                start: range.start,
                end: range.end,
            });
        }
        Ok(self.byte_of(range.start)?..self.byte_of(range.end)?)
    }

    fn byte_of(&self, char_idx: usize) -> Result<usize, EditError> {
        if char_idx == self.len_chars {
            return Ok(self.text.len());
        }
        self.text
            .char_indices()
            .nth(char_idx)
            .map(|(byte, _)| byte)
            .ok_or(EditError::OutOfBounds {
                index: char_idx,
                len_chars: self.len_chars,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_into_empty_buffer() {
        let mut b = Buffer::new();
        b.insert(0, "hello").unwrap();
        assert_eq!(b.text(), "hello");
        assert_eq!(b.len_chars(), 5);
    }

    #[test]
    fn insert_at_start_middle_and_end() {
        let mut b = Buffer::from_text("ac");
        b.insert(1, "b").unwrap();
        b.insert(0, ">").unwrap();
        b.insert(4, "<").unwrap();
        assert_eq!(b.text(), ">abc<");
    }

    #[test]
    fn insert_uses_char_indices_not_bytes() {
        let mut b = Buffer::from_text("héllo wörld");
        b.insert(2, "X").unwrap();
        assert_eq!(b.text(), "héXllo wörld");
        assert_eq!(b.len_chars(), 12);
        assert_eq!(b.len_bytes(), 14);
    }

    #[test]
    fn insert_out_of_bounds_is_rejected_and_buffer_unchanged() {
        let mut b = Buffer::from_text("ab");
        let err = b.insert(3, "x").unwrap_err();
        assert_eq!(
            err,
            EditError::OutOfBounds {
                index: 3,
                len_chars: 2
            }
        );
        assert_eq!(b.text(), "ab");
    }

    #[test]
    fn delete_returns_removed_text() {
        let mut b = Buffer::from_text("a😀bc");
        let removed = b.delete(1..3).unwrap();
        assert_eq!(removed, "😀b");
        assert_eq!(b.text(), "ac");
        assert_eq!(b.len_chars(), 2);
    }

    #[test]
    fn delete_empty_range_is_noop() {
        let mut b = Buffer::from_text("abc");
        assert_eq!(b.delete(1..1).unwrap(), "");
        assert_eq!(b.text(), "abc");
    }

    #[test]
    fn delete_rejects_inverted_and_out_of_bounds_ranges() {
        let mut b = Buffer::from_text("abc");
        #[allow(clippy::reversed_empty_ranges)]
        let inverted = b.delete(2..1).unwrap_err();
        assert_eq!(inverted, EditError::InvertedRange { start: 2, end: 1 });
        assert!(matches!(
            b.delete(1..9),
            Err(EditError::OutOfBounds { index: 9, .. })
        ));
        assert_eq!(b.text(), "abc");
    }

    #[test]
    fn slice_reads_char_range() {
        let b = Buffer::from_text("añb");
        assert_eq!(b.slice(1..3).unwrap(), "ñb");
        assert!(b.slice(0..4).is_err());
    }

    /// Differential test against a naive `Vec<char>` model (CHARTER §3 decision 5), driven by a
    /// fixed-seed generator so it is deterministic.
    #[test]
    fn random_edits_match_naive_model() {
        let mut seed: u64 = 0x5eed;
        let mut next = move |bound: usize| {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((seed >> 33) as usize) % bound.max(1)
        };
        let alphabet = ['a', 'é', '😀', '\n', 'z'];
        let mut buffer = Buffer::new();
        let mut model: Vec<char> = Vec::new();
        for _ in 0..2000 {
            if next(3) == 0 && !model.is_empty() {
                let start = next(model.len() + 1);
                let end = start + next(model.len() - start + 1);
                let removed = buffer.delete(start..end).unwrap();
                let expected: String = model.drain(start..end).collect();
                assert_eq!(removed, expected);
            } else {
                let at = next(model.len() + 1);
                let ch = alphabet[next(alphabet.len())];
                buffer.insert(at, &ch.to_string()).unwrap();
                model.insert(at, ch);
            }
            assert_eq!(buffer.len_chars(), model.len());
        }
        assert_eq!(buffer.text(), model.iter().collect::<String>());
    }
}
