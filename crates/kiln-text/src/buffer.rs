//! The editable text buffer.

use std::fmt;
use std::ops::Range;

use ropey::Rope;

use crate::history::{History, Op};
use crate::line_ending::{BreakCounts, LineEnding, LineEndings};
use crate::position::{LineCol, PositionError};

/// An editable text buffer addressed by **char indices** (Unicode scalar values).
///
/// Storage is a [`ropey::Rope`] (edits are `O(log n)`, no whole-text copy), built with only
/// the `cr_lines` line-break set so the rope's lines are exactly those split on `\r\n`, `\n`
/// and lone `\r`. Undo/redo is operation-based: each step stores the inverse edit, never a
/// snapshot.
///
/// `PartialEq`/`Eq` compare the text only; the history is ignored. A clone carries an
/// independent copy of the history.
#[derive(Clone, Debug, Default)]
pub struct Buffer {
    rope: Rope,
    /// Line breaks per kind, kept up to date on every edit.
    breaks: BreakCounts,
    history: History,
}

impl PartialEq for Buffer {
    fn eq(&self, other: &Self) -> bool {
        self.rope == other.rope
    }
}

impl Eq for Buffer {}

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

fn is_line_break(c: char) -> bool {
    c == '\n' || c == '\r'
}

impl Buffer {
    /// An empty buffer with an empty history.
    pub fn new() -> Self {
        Self::default()
    }

    /// A buffer holding `text`, with an empty history.
    pub fn from_text(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            breaks: BreakCounts::of(text),
            history: History::default(),
        }
    }

    /// Length in chars.
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    /// Length in UTF-8 bytes.
    pub fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }

    /// Length in UTF-16 code units.
    pub fn len_utf16(&self) -> usize {
        self.rope.len_utf16_cu()
    }

    /// Number of lines: line breaks + 1 (so `""` has 1 line).
    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    /// True if the buffer holds no text.
    pub fn is_empty(&self) -> bool {
        self.rope.len_bytes() == 0
    }

    /// The whole text as an owned string.
    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    /// The text in the char range `range`.
    pub fn slice(&self, range: Range<usize>) -> Result<String, EditError> {
        self.check_range(&range)?;
        Ok(self.rope.slice(range).to_string())
    }

    /// Content of line `line` without its line break.
    pub fn line(&self, line: usize) -> Result<String, PositionError> {
        self.check_line(line)?;
        let start = self.rope.line_to_char(line);
        let len = self.line_content_len(line);
        Ok(self.rope.slice(start..start + len).to_string())
    }

    /// Char index of the first char of `line` (`== len_chars()` for an empty last line).
    pub fn line_start(&self, line: usize) -> Result<usize, PositionError> {
        self.check_line(line)?;
        Ok(self.rope.line_to_char(line))
    }

    /// The line-ending style of the text.
    pub fn line_endings(&self) -> LineEndings {
        self.breaks.style()
    }

    /// The most frequent kind of line break; ties go to Lf, then Crlf, then Cr; `None` if the
    /// text has no line break.
    pub fn dominant_line_ending(&self) -> Option<LineEnding> {
        self.breaks.dominant()
    }

    /// Byte offset of char index `char_idx` (valid: `0..=len_chars()`).
    pub fn char_to_byte(&self, char_idx: usize) -> Result<usize, PositionError> {
        self.check_char(char_idx)?;
        Ok(self.rope.char_to_byte(char_idx))
    }

    /// Char index at byte offset `byte` (valid: `0..=len_bytes()` on a char boundary).
    pub fn byte_to_char(&self, byte: usize) -> Result<usize, PositionError> {
        let len = self.len_bytes();
        if byte > len {
            return Err(PositionError::OutOfBounds { index: byte, len });
        }
        // ropey rounds a non-boundary offset down to the char containing it.
        let char_idx = self.rope.byte_to_char(byte);
        if self.rope.char_to_byte(char_idx) != byte {
            return Err(PositionError::NotCharBoundary { byte });
        }
        Ok(char_idx)
    }

    /// UTF-16 offset of char index `char_idx` (valid: `0..=len_chars()`).
    pub fn char_to_utf16(&self, char_idx: usize) -> Result<usize, PositionError> {
        self.check_char(char_idx)?;
        Ok(self.rope.char_to_utf16_cu(char_idx))
    }

    /// Char index at UTF-16 offset `utf16` (valid: `0..=len_utf16()`, not inside a surrogate
    /// pair).
    pub fn utf16_to_char(&self, utf16: usize) -> Result<usize, PositionError> {
        let len = self.len_utf16();
        if utf16 > len {
            return Err(PositionError::OutOfBounds { index: utf16, len });
        }
        // ropey rounds an offset inside a surrogate pair down to the char it belongs to.
        let char_idx = self.rope.utf16_cu_to_char(utf16);
        if self.rope.char_to_utf16_cu(char_idx) != utf16 {
            return Err(PositionError::InsideSurrogatePair { utf16 });
        }
        Ok(char_idx)
    }

    /// Line and column of char index `char_idx` (valid: `0..=len_chars()`, not between the
    /// `\r` and `\n` of a CRLF).
    pub fn char_to_line_col(&self, char_idx: usize) -> Result<LineCol, PositionError> {
        self.check_char(char_idx)?;
        if char_idx > 0
            && char_idx < self.len_chars()
            && self.rope.char(char_idx - 1) == '\r'
            && self.rope.char(char_idx) == '\n'
        {
            return Err(PositionError::InsideLineBreak { char_idx });
        }
        let line = self.rope.char_to_line(char_idx);
        Ok(LineCol {
            line,
            col: char_idx - self.rope.line_to_char(line),
        })
    }

    /// Char index of `pos` (valid: `line < len_lines()`, `col <=` the line's content length).
    pub fn line_col_to_char(&self, pos: LineCol) -> Result<usize, PositionError> {
        self.check_line(pos.line)?;
        let line_len = self.line_content_len(pos.line);
        if pos.col > line_len {
            return Err(PositionError::ColumnOutOfBounds {
                line: pos.line,
                col: pos.col,
                line_len,
            });
        }
        Ok(self.rope.line_to_char(pos.line) + pos.col)
    }

    /// Insert `text` before the char at `char_idx` (`char_idx == len_chars()` appends).
    ///
    /// A successful insert of non-empty text is one undo step, or joins the open typing group
    /// (grouping rule in the crate docs). Inserting `""` changes nothing.
    pub fn insert(&mut self, char_idx: usize, text: &str) -> Result<(), EditError> {
        self.check_index(char_idx)?;
        if text.is_empty() {
            return Ok(());
        }
        let mut chars = text.chars();
        let typing = matches!((chars.next(), chars.next()), (Some(c), None) if !is_line_break(c));
        let removed = self.apply(char_idx, char_idx, text);
        self.history
            .record(Op::new(char_idx, removed, text), typing);
        Ok(())
    }

    /// Delete the chars in `range` and return the removed text. A non-empty delete is one
    /// undo step; deleting an empty range changes nothing.
    pub fn delete(&mut self, range: Range<usize>) -> Result<String, EditError> {
        self.replace(range, "")
    }

    /// Replace `range` with `text` as one undo step and return the removed text. Replacing an
    /// empty range with `""` changes nothing. A replace never joins a typing group.
    pub fn replace(&mut self, range: Range<usize>, text: &str) -> Result<String, EditError> {
        self.check_range(&range)?;
        if range.is_empty() && text.is_empty() {
            return Ok(String::new());
        }
        let removed = self.apply(range.start, range.end, text);
        self.history
            .record(Op::new(range.start, removed.clone(), text), false);
        Ok(removed)
    }

    /// Undo the most recent step. Returns `false` (and changes nothing) if there is none.
    pub fn undo(&mut self) -> bool {
        let Some(op) = self.history.pop_undo() else {
            return false;
        };
        self.apply(op.at, op.at + op.inserted_chars, &op.removed);
        self.history.undone(op);
        true
    }

    /// Redo the most recently undone step. Returns `false` (and changes nothing) if there is
    /// none.
    pub fn redo(&mut self) -> bool {
        let Some(op) = self.history.pop_redo() else {
            return false;
        };
        self.apply(op.at, op.at + op.removed_chars, &op.inserted);
        self.history.redone(op);
        true
    }

    /// True if [`Self::undo`] would change the text.
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    /// True if [`Self::redo`] would change the text.
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// End the current typing group: the next insert starts a new undo step.
    pub fn break_undo_group(&mut self) {
        self.history.break_group();
    }

    /// Replace chars `start..end` (already validated) with `text`, keep the break counts up to
    /// date, and return the removed text. Reads only the edited region and one char on each
    /// side of it.
    fn apply(&mut self, start: usize, end: usize, text: &str) -> String {
        // A char's line-break kind depends only on its neighbours (`\r` on the next char,
        // `\n` on the previous one), so a window with one unchanged char of margin on each
        // side holds every break whose kind can change. The margin chars themselves may be
        // misclassified at the window edge (a leading `\n` or trailing `\r` of a CRLF that
        // straddles the edge), but identically before and after the edit, so the difference
        // is exact.
        let win_start = start.saturating_sub(1);
        let win_end = (end + 1).min(self.len_chars());
        let removed = self.rope.slice(start..end).to_string();
        let before = self.rope.slice(win_start..start).to_string();
        let after = self.rope.slice(end..win_end).to_string();
        let old = BreakCounts::of(&format!("{before}{removed}{after}"));
        let new = BreakCounts::of(&format!("{before}{text}{after}"));
        self.breaks = self.breaks.replaced(old, new);
        if start < end {
            self.rope.remove(start..end);
        }
        if !text.is_empty() {
            self.rope.insert(start, text);
        }
        removed
    }

    /// Length in chars of line `line`'s content, without its line break (`line` is valid).
    fn line_content_len(&self, line: usize) -> usize {
        let slice = self.rope.line(line);
        let n = slice.len_chars();
        let last = n.checked_sub(1).map(|i| slice.char(i));
        let second_last = n.checked_sub(2).map(|i| slice.char(i));
        match (second_last, last) {
            (Some('\r'), Some('\n')) => n - 2,
            (_, Some('\n' | '\r')) => n - 1,
            _ => n,
        }
    }

    fn check_line(&self, line: usize) -> Result<(), PositionError> {
        let len_lines = self.len_lines();
        if line >= len_lines {
            return Err(PositionError::LineOutOfBounds { line, len_lines });
        }
        Ok(())
    }

    fn check_char(&self, char_idx: usize) -> Result<(), PositionError> {
        let len = self.len_chars();
        if char_idx > len {
            return Err(PositionError::OutOfBounds {
                index: char_idx,
                len,
            });
        }
        Ok(())
    }

    fn check_index(&self, char_idx: usize) -> Result<(), EditError> {
        let len_chars = self.len_chars();
        if char_idx > len_chars {
            return Err(EditError::OutOfBounds {
                index: char_idx,
                len_chars,
            });
        }
        Ok(())
    }

    /// Validation order (as on main before T-001): inverted range, then start, then end.
    fn check_range(&self, range: &Range<usize>) -> Result<(), EditError> {
        if range.start > range.end {
            return Err(EditError::InvertedRange {
                start: range.start,
                end: range.end,
            });
        }
        self.check_index(range.start)?;
        self.check_index(range.end)
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

    // ---- T-001 unit tests: naive-model differential tests and exact values -----------------

    /// Fixed-seed LCG (deterministic; no dev-dependency).
    struct Gen(u64);

    impl Gen {
        fn next(&mut self, bound: usize) -> usize {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((self.0 >> 33) as usize) % bound.max(1)
        }

        /// Text from an alphabet that stresses every unit: ASCII, 2/3/4-byte chars, all
        /// three line breaks, and separators that must NOT break lines.
        fn text(&mut self, max_chars: usize) -> String {
            const PIECES: &[&str] = &[
                "a", "Z", " ", "é", "ñ", "€", "中", "😀", "🦀", "\n", "\r", "\r\n", "\u{2028}",
                "\u{85}", "\u{0B}",
            ];
            let n = self.next(max_chars + 1);
            (0..n).map(|_| PIECES[self.next(PIECES.len())]).collect()
        }
    }

    /// Lines of `m` split on `\r\n|\n|\r` (CRLF first): (start char index, content).
    fn naive_lines(m: &str) -> Vec<(usize, String)> {
        let chars: Vec<char> = m.chars().collect();
        let mut lines = Vec::new();
        let (mut start, mut i) = (0, 0);
        while i < chars.len() {
            let width = match (chars[i], chars.get(i + 1)) {
                ('\r', Some('\n')) => 2,
                ('\r' | '\n', _) => 1,
                _ => 0,
            };
            if width > 0 {
                lines.push((start, chars[start..i].iter().collect()));
                i += width;
                start = i;
            } else {
                i += 1;
            }
        }
        lines.push((start, chars[start..].iter().collect()));
        lines
    }

    fn naive_counts(m: &str) -> (usize, usize, usize) {
        let (mut lf, mut crlf, mut cr) = (0, 0, 0);
        let b = m.as_bytes();
        let mut i = 0;
        while i < b.len() {
            match (b[i], b.get(i + 1)) {
                (b'\r', Some(b'\n')) => {
                    crlf += 1;
                    i += 1;
                }
                (b'\r', _) => cr += 1,
                (b'\n', _) => lf += 1,
                _ => {}
            }
            i += 1;
        }
        (lf, crlf, cr)
    }

    fn naive_style(m: &str) -> (LineEndings, Option<LineEnding>) {
        let (lf, crlf, cr) = naive_counts(m);
        let style = match (lf > 0, crlf > 0, cr > 0) {
            (false, false, false) => LineEndings::None,
            (true, false, false) => LineEndings::Lf,
            (false, true, false) => LineEndings::Crlf,
            (false, false, true) => LineEndings::Cr,
            _ => LineEndings::Mixed,
        };
        let dominant = if lf + crlf + cr == 0 {
            None
        } else if lf >= crlf && lf >= cr {
            Some(LineEnding::Lf)
        } else if crlf >= cr {
            Some(LineEnding::Crlf)
        } else {
            Some(LineEnding::Cr)
        };
        (style, dominant)
    }

    /// Check sizes, lines, style and every position conversion of `b` against `m`.
    /// `stride` samples every n-th char index (1 = all).
    fn assert_matches_model(b: &Buffer, m: &str, stride: usize) {
        assert_eq!(b.len_chars(), m.chars().count());
        assert_eq!(b.len_bytes(), m.len());
        assert_eq!(b.len_utf16(), m.encode_utf16().count());
        let lines = naive_lines(m);
        assert_eq!(b.len_lines(), lines.len());
        assert_eq!(b.breaks.total() + 1, lines.len());
        assert_eq!((b.line_endings(), b.dominant_line_ending()), naive_style(m));
        for (i, (start, content)) in lines.iter().enumerate().step_by(stride) {
            assert_eq!(b.line(i).unwrap(), *content, "line {i}");
            assert_eq!(b.line_start(i).unwrap(), *start, "line_start {i}");
        }
        let chars: Vec<char> = m.chars().collect();
        let (mut byte, mut utf16, mut line) = (0, 0, 0);
        for c in 0..=chars.len() {
            while line + 1 < lines.len() && lines[line + 1].0 <= c {
                line += 1;
            }
            if c % stride == 0 || c == chars.len() {
                assert_eq!(b.char_to_byte(c), Ok(byte));
                assert_eq!(b.byte_to_char(byte), Ok(c));
                assert_eq!(b.char_to_utf16(c), Ok(utf16));
                assert_eq!(b.utf16_to_char(utf16), Ok(c));
                let inside_crlf = c > 0 && chars[c - 1] == '\r' && chars.get(c) == Some(&'\n');
                if inside_crlf {
                    assert_eq!(
                        b.char_to_line_col(c),
                        Err(PositionError::InsideLineBreak { char_idx: c })
                    );
                } else {
                    let lc = LineCol {
                        line,
                        col: c - lines[line].0,
                    };
                    assert_eq!(b.char_to_line_col(c), Ok(lc), "char {c}");
                    assert_eq!(b.line_col_to_char(lc), Ok(c));
                }
            }
            if let Some(ch) = chars.get(c) {
                byte += ch.len_utf8();
                utf16 += ch.len_utf16();
            }
        }
    }

    fn naive_replace(m: &str, range: Range<usize>, text: &str) -> (String, String) {
        let chars: Vec<char> = m.chars().collect();
        let removed: String = chars[range.clone()].iter().collect();
        let mut out: String = chars[..range.start].iter().collect();
        out.push_str(text);
        out.extend(&chars[range.end..]);
        (out, removed)
    }

    /// Edits on texts large enough to span many rope chunks, with breaks (CRLF in particular)
    /// created and destroyed at edit boundaries; checked against a `String` after every op.
    #[test]
    fn differential_edits_across_rope_chunk_boundaries() {
        for seed in [1u64, 7, 0xdead_beef] {
            let mut g = Gen(seed);
            let mut m = g.text(6000) + &g.text(6000);
            let mut b = Buffer::from_text(&m);
            assert_matches_model(&b, &m, 1);
            for step in 0..1500 {
                let len = m.chars().count();
                let start = g.next(len + 2);
                let end = if g.next(8) == 0 {
                    start.saturating_sub(1 + g.next(3))
                } else {
                    start + g.next(len.saturating_sub(start) + 2).min(40)
                };
                let text = match g.next(4) {
                    0 => "\r".to_owned(),
                    1 => "\n".to_owned(),
                    _ => g.text(4),
                };
                let kind = g.next(3);
                let result = match kind {
                    0 => b.insert(start, &text).map(|()| String::new()),
                    1 => b.delete(start..end),
                    _ => b.replace(start..end, &text),
                };
                let range = if kind == 0 { start..start } else { start..end };
                let text = if kind == 1 { "" } else { text.as_str() };
                if range.start > range.end {
                    assert_eq!(
                        result,
                        Err(EditError::InvertedRange {
                            start: range.start,
                            end: range.end
                        })
                    );
                } else if range.end > len {
                    let index = if range.start > len {
                        range.start
                    } else {
                        range.end
                    };
                    assert_eq!(
                        result,
                        Err(EditError::OutOfBounds {
                            index,
                            len_chars: len
                        })
                    );
                } else {
                    let (next, removed) = naive_replace(&m, range, text);
                    assert_eq!(result.unwrap(), removed);
                    m = next;
                }
                assert_eq!(b.len_chars(), m.chars().count());
                assert_eq!(
                    (b.line_endings(), b.dominant_line_ending()),
                    naive_style(&m)
                );
                assert_eq!(b.len_lines(), naive_lines(&m).len());
                if step % 100 == 0 {
                    assert_eq!(b.text(), m);
                    assert_matches_model(&b, &m, 7);
                }
            }
            assert_eq!(b.text(), m);
            assert_matches_model(&b, &m, 1);
        }
    }

    #[test]
    fn conversions_match_model_on_small_generated_texts() {
        let mut g = Gen(42);
        for _ in 0..200 {
            let m = g.text(30);
            assert_matches_model(&Buffer::from_text(&m), &m, 1);
        }
    }

    #[test]
    fn crlf_is_split_and_rejoined_by_edits() {
        let mut b = Buffer::from_text("a\r\nb");
        assert_eq!(b.line_endings(), LineEndings::Crlf);
        b.insert(2, "x").unwrap();
        assert_eq!(b.text(), "a\rx\nb");
        assert_eq!(b.len_lines(), 3);
        assert_eq!(b.line_endings(), LineEndings::Mixed);
        assert_eq!(b.delete(2..3).unwrap(), "x");
        assert_eq!(b.len_lines(), 2);
        assert_eq!(b.line_endings(), LineEndings::Crlf);
        // A lone CR followed by an inserted LF becomes one CRLF.
        let mut b = Buffer::from_text("a\r");
        b.insert(2, "\n").unwrap();
        assert_eq!(b.len_lines(), 2);
        assert_eq!(b.line_endings(), LineEndings::Crlf);
        assert!(b.undo());
        assert_eq!(b.line_endings(), LineEndings::Cr);
        assert_eq!(b.line(0).unwrap(), "a");
    }

    #[test]
    fn unicode_separators_do_not_break_lines() {
        let b = Buffer::from_text("a\u{0B}b\u{0C}c\u{85}d\u{2028}e\u{2029}f");
        assert_eq!(b.len_lines(), 1);
        assert_eq!(b.line_endings(), LineEndings::None);
    }

    #[test]
    fn invalid_positions_exact_errors() {
        let b = Buffer::from_text("é😀\r\nx");
        assert_eq!(
            b.byte_to_char(1),
            Err(PositionError::NotCharBoundary { byte: 1 })
        );
        assert_eq!(
            b.byte_to_char(4),
            Err(PositionError::NotCharBoundary { byte: 4 })
        );
        assert_eq!(
            b.utf16_to_char(2),
            Err(PositionError::InsideSurrogatePair { utf16: 2 })
        );
        assert_eq!(b.len_bytes(), 9);
        assert_eq!(
            b.byte_to_char(10),
            Err(PositionError::OutOfBounds { index: 10, len: 9 })
        );
        assert_eq!(
            b.char_to_byte(6),
            Err(PositionError::OutOfBounds { index: 6, len: 5 })
        );
        assert_eq!(
            b.utf16_to_char(7),
            Err(PositionError::OutOfBounds { index: 7, len: 6 })
        );
        assert_eq!(
            b.char_to_utf16(6),
            Err(PositionError::OutOfBounds { index: 6, len: 5 })
        );
        assert_eq!(
            b.char_to_line_col(6),
            Err(PositionError::OutOfBounds { index: 6, len: 5 })
        );
        assert_eq!(
            b.char_to_line_col(3),
            Err(PositionError::InsideLineBreak { char_idx: 3 })
        );
        assert_eq!(
            b.line(2),
            Err(PositionError::LineOutOfBounds {
                line: 2,
                len_lines: 2
            })
        );
        assert_eq!(
            b.line_start(2),
            Err(PositionError::LineOutOfBounds {
                line: 2,
                len_lines: 2
            })
        );
        assert_eq!(
            b.line_col_to_char(LineCol { line: 0, col: 3 }),
            Err(PositionError::ColumnOutOfBounds {
                line: 0,
                col: 3,
                line_len: 2
            })
        );
        assert_eq!(b.line_col_to_char(LineCol { line: 0, col: 2 }), Ok(2));
        assert_eq!(b.line_col_to_char(LineCol { line: 1, col: 1 }), Ok(5));
    }

    #[test]
    fn spec_line_and_style_examples() {
        let b = Buffer::from_text("a\r\nb\rc\nd");
        let lines: Vec<String> = (0..b.len_lines()).map(|i| b.line(i).unwrap()).collect();
        let starts: Vec<usize> = (0..b.len_lines())
            .map(|i| b.line_start(i).unwrap())
            .collect();
        assert_eq!(lines, ["a", "b", "c", "d"]);
        assert_eq!(starts, [0, 3, 5, 7]);
        let b = Buffer::from_text("a\n");
        assert_eq!(b.line(1).unwrap(), "");
        assert_eq!(b.line_start(1).unwrap(), 2);
        assert_eq!(Buffer::new().len_lines(), 1);
        for (text, style) in [
            ("", LineEndings::None),
            ("a\nb\n", LineEndings::Lf),
            ("a\r\nb\r\n", LineEndings::Crlf),
            ("a\rb", LineEndings::Cr),
            ("a\nb\r\n", LineEndings::Mixed),
            ("a\r\r\n", LineEndings::Mixed),
        ] {
            assert_eq!(Buffer::from_text(text).line_endings(), style, "{text:?}");
        }
        for (text, dominant) in [
            ("abc", None),
            ("a\r\nb\r\nc\n", Some(LineEnding::Crlf)),
            ("a\nb\r\n", Some(LineEnding::Lf)),
            ("a\r\nb\r", Some(LineEnding::Crlf)),
            ("a\r\r\r\n", Some(LineEnding::Cr)),
        ] {
            assert_eq!(
                Buffer::from_text(text).dominant_line_ending(),
                dominant,
                "{text:?}"
            );
        }
    }

    /// Model of the grouping rule: the list of texts at undo-step boundaries.
    #[test]
    fn undo_redo_round_trip_against_step_model() {
        for seed in [3u64, 11, 99] {
            let mut g = Gen(seed);
            let initial = g.text(3000);
            let mut b = Buffer::from_text(&initial);
            let mut m = initial.clone();
            // texts[i] = text before step i; the last entry is the current text.
            let mut texts = vec![initial.clone()];
            let mut typing_end: Option<usize> = None;
            let mut done = 0;
            while done < 400 {
                match g.next(20) {
                    0 => {
                        b.break_undo_group();
                        typing_end = None;
                    }
                    1 if texts.len() > 1 => {
                        assert!(b.undo());
                        texts.pop();
                        m = texts.last().unwrap().clone();
                        typing_end = None;
                        assert_eq!(b.text(), m);
                        assert!(b.redo());
                        let (n, _) = (b.text(), ());
                        texts.push(n.clone());
                        m = n;
                    }
                    k => {
                        let len = m.chars().count();
                        let typed = k % 3 == 0 && typing_end.is_some();
                        let at = if typed {
                            typing_end.unwrap_or(0)
                        } else {
                            g.next(len + 1)
                        };
                        let text = if k % 2 == 0 || typed {
                            ["x", "é", "😀", "\n"][g.next(4)].to_owned()
                        } else {
                            g.text(3)
                        };
                        let end = (at + g.next(5)).min(len);
                        let (range, txt, joinable) = match k % 4 {
                            0 | 2 => {
                                b.insert(at, &text).unwrap();
                                let one = text.chars().count() == 1 && !text.contains(['\n', '\r']);
                                (at..at, text.as_str(), one)
                            }
                            1 => {
                                b.delete(at..end).unwrap();
                                (at..end, "", false)
                            }
                            _ => {
                                b.replace(at..end, &text).unwrap();
                                (at..end, text.as_str(), false)
                            }
                        };
                        if range.is_empty() && txt.is_empty() {
                            continue; // no-op: nothing recorded, group untouched
                        }
                        let (next, _) = naive_replace(&m, range.clone(), txt);
                        m = next;
                        if joinable && typing_end == Some(at) {
                            *texts.last_mut().unwrap() = m.clone();
                        } else {
                            texts.push(m.clone());
                        }
                        typing_end = joinable.then_some(at + 1);
                        done += 1;
                    }
                }
                assert_eq!(b.text(), m);
            }
            let final_text = m.clone();
            let mut seen = vec![b.text()];
            while b.undo() {
                seen.push(b.text());
            }
            let mut expected = texts.clone();
            expected.reverse();
            assert_eq!(seen, expected);
            assert_eq!(b.text(), initial);
            assert!(!b.can_undo());
            while b.redo() {}
            assert_eq!(b.text(), final_text);
            assert_matches_model(&b, &final_text, 5);
        }
    }

    #[test]
    fn grouping_exact_values() {
        let typed = |third: &dyn Fn(&mut Buffer)| {
            let mut b = Buffer::new();
            b.insert(0, "a").unwrap();
            b.insert(1, "b").unwrap();
            third(&mut b);
            b
        };
        let mut b = typed(&|b| b.insert(2, "c").unwrap());
        assert!(b.undo());
        assert_eq!(b.text(), "");
        assert!(!b.undo());
        let mut b = typed(&|b| b.insert(1, "c").unwrap());
        assert_eq!(b.text(), "acb");
        assert!(b.undo());
        assert_eq!(b.text(), "ab");
        for third in ["cd", "\n", "\r", "\r\n"] {
            let mut b = typed(&|b| b.insert(2, third).unwrap());
            assert!(b.undo());
            assert_eq!(b.text(), "ab", "{third:?}");
        }
        let mut b = typed(&|b| {
            b.break_undo_group();
            b.insert(2, "c").unwrap();
        });
        assert!(b.undo());
        assert_eq!(b.text(), "ab");
        // No-ops and rejected edits do not end the group.
        let mut b = typed(&|b| {
            b.delete(0..0).unwrap();
            b.insert(1, "").unwrap();
            b.replace(2..2, "").unwrap();
            assert!(b.insert(9, "q").is_err());
            assert!(b.delete(0..9).is_err());
            assert!(!b.redo());
            b.insert(2, "c").unwrap();
        });
        assert!(b.undo());
        assert_eq!(b.text(), "");
        // replace of an empty range is insert-like but never joins and is never joined.
        let mut b = typed(&|b| {
            b.replace(2..2, "c").unwrap();
            b.insert(3, "d").unwrap();
        });
        assert!(b.undo());
        assert_eq!(b.text(), "abc");
        assert!(b.undo());
        assert_eq!(b.text(), "ab");
        assert!(b.undo());
        assert_eq!(b.text(), "");
        // A multi-char insert does not open a group for the next char.
        let mut b = Buffer::new();
        b.insert(0, "ab").unwrap();
        b.insert(2, "c").unwrap();
        assert!(b.undo());
        assert_eq!(b.text(), "ab");
        // After undo, typing starts a new step.
        let mut b = typed(&|b| {
            assert!(b.undo());
            b.insert(0, "x").unwrap();
            b.insert(1, "y").unwrap();
        });
        assert_eq!(b.text(), "xy");
        assert!(!b.can_redo());
        assert!(b.undo());
        assert_eq!(b.text(), "");
    }

    #[test]
    fn single_edit_undo_redo_and_redo_cleared() {
        let mut b = Buffer::from_text("héllo\r\nwörld");
        assert!(!b.undo() && !b.redo());
        assert_eq!(b.text(), "héllo\r\nwörld");
        assert_eq!(b.replace(1..8, "😀\r").unwrap(), "éllo\r\nw");
        assert_eq!(b.text(), "h😀\rörld");
        assert!(b.undo());
        assert_eq!(b.text(), "héllo\r\nwörld");
        assert!(b.can_redo() && !b.can_undo());
        assert!(b.redo());
        assert_eq!(b.text(), "h😀\rörld");
        assert!(!b.redo());
        assert!(b.undo());
        b.delete(0..1).unwrap();
        assert!(!b.can_redo() && !b.redo());
        assert!(b.undo());
        assert_eq!(b.text(), "héllo\r\nwörld");
    }

    #[test]
    fn noop_and_rejected_edits_leave_history() {
        let mut b = Buffer::from_text("abc");
        b.insert(3, "d").unwrap();
        assert!(b.undo());
        assert!(b.can_redo());
        #[allow(clippy::reversed_empty_ranges)]
        let inverted = b.replace(2..1, "x");
        assert!(inverted.is_err());
        assert!(b.insert(4, "x").is_err());
        b.insert(0, "").unwrap();
        b.delete(1..1).unwrap();
        assert!(b.can_redo() && !b.can_undo());
        assert!(b.redo());
        assert_eq!(b.text(), "abcd");
    }

    #[test]
    fn equality_ignores_history_and_clones_are_independent() {
        let mut a = Buffer::from_text("xy");
        let mut b = Buffer::new();
        b.insert(0, "x").unwrap();
        b.insert(1, "y").unwrap();
        assert_eq!(a, b);
        let mut c = b.clone();
        assert!(c.undo());
        assert_eq!(c.text(), "");
        assert_eq!(b.text(), "xy");
        assert!(b.can_undo());
        a.insert(0, "!").unwrap();
        assert_ne!(a, b);
    }
}
