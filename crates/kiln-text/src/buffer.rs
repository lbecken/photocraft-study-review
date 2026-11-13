//! The editable text buffer.

use std::fmt;
use std::ops::Range;

use ropey::Rope;

use crate::history::{History, Op};
use crate::line_ending::{BreakCounts, LineEnding, LineEndings};
use crate::position::{LineCol, PositionError};
use crate::selection::{AnchorId, Movement, Selection, is_word_char, map_position};

/// An editable text buffer addressed by **char indices** (Unicode scalar values).
///
/// Storage is a [`ropey::Rope`] (edits are `O(log n)`, no whole-text copy), built with only
/// the `cr_lines` line-break set so the rope's lines are exactly those split on `\r\n`, `\n`
/// and lone `\r`. Undo/redo is operation-based: each step stores the inverse edit, never a
/// snapshot.
///
/// The buffer also holds one [`Selection`] and any number of anchors. Both are mapped through
/// every primitive edit (insert, delete, replace, each edit of a batch, and each edit that undo
/// and redo apply) by the Mapping rule (described on [`Selection`]); setting them is not an edit
/// and touches neither the history nor an open typing group.
///
/// `PartialEq`/`Eq` compare the text only; the history, selection and anchors are ignored. A
/// clone carries an independent copy of all of them.
#[derive(Clone, Debug, Default)]
pub struct Buffer {
    rope: Rope,
    /// Line breaks per kind, kept up to date on every edit.
    breaks: BreakCounts,
    history: History,
    selection: Selection,
    /// Live anchors by id, ids in creation order.
    anchors: Vec<(AnchorId, usize)>,
    /// The id the next `add_anchor` returns.
    next_anchor: u64,
}

/// Why [`Buffer::apply_edits`] rejected a list of edits. Nothing changes when it is returned.
///
/// Edits are checked in index order; for each index the edit's own range is checked first and
/// its order against the previous edit second, and the first failure is reported.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BatchError {
    /// Edit `index` starts before the end of edit `index - 1`, or at the same start.
    Unordered {
        /// Index of the offending edit.
        index: usize,
    },
    /// Edit `index` is out of bounds or inverted.
    Edit {
        /// Index of the offending edit.
        index: usize,
        /// The error [`Buffer::replace`] reports for that range.
        error: EditError,
    },
}

impl fmt::Display for BatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BatchError::Unordered { index } => write!(
                f,
                "edit {index} starts before the end or at the start of the previous edit"
            ),
            BatchError::Edit { index, error } => write!(f, "edit {index}: {error}"),
        }
    }
}

impl std::error::Error for BatchError {}

impl PartialEq for Buffer {
    fn eq(&self, other: &Self) -> bool {
        self.rope == other.rope
    }
}

impl Eq for Buffer {}

/// Why an edit or read was rejected. The buffer is unchanged when an error is returned, and
/// so is its history, including an open typing group.
///
/// Validation order for a char range `start..end` (`delete`, `replace`, `slice`), first match
/// wins:
/// 1. `start > end` → [`EditError::InvertedRange`]`{start, end}`, even when `start` or `end`
///    is also beyond `len_chars()`;
/// 2. `start > len_chars()` → [`EditError::OutOfBounds`]`{index: start, len_chars}`;
/// 3. `end > len_chars()` → [`EditError::OutOfBounds`]`{index: end, len_chars}`.
///
/// On `"abc"`: `delete(9..5)` → `InvertedRange{start: 9, end: 5}`; `delete(4..9)` →
/// `OutOfBounds{index: 4, len_chars: 3}`; `delete(1..9)` → `OutOfBounds{index: 9, len_chars: 3}`.
/// `insert` validates its single index the same way. Indices are validated before the edit's
/// emptiness is considered, so a would-be no-op edit at an out-of-range index is an error.
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
            ..Self::default()
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
    ///
    /// Errors: `char_idx > len_chars()` → [`EditError::OutOfBounds`]`{index: char_idx,
    /// len_chars}`. The index is checked before the text's emptiness, so `insert(9, "")` on a
    /// 3-char buffer is that error, not a no-op `Ok`. On error nothing changes, and an open
    /// typing group stays open.
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
    ///
    /// Errors, in this order (see [`EditError`]): `start > end` → `InvertedRange{start, end}`
    /// even if a bound is also out of range; else the first of `start`, `end` beyond
    /// `len_chars()` → `OutOfBounds{index, len_chars}`. The range is checked before its
    /// emptiness, so `delete(9..9)` on a 3-char buffer is `OutOfBounds{index: 9, len_chars: 3}`,
    /// not a no-op `Ok`. On error nothing changes, and an open typing group stays open.
    pub fn delete(&mut self, range: Range<usize>) -> Result<String, EditError> {
        self.replace(range, "")
    }

    /// Replace `range` with `text` as one undo step and return the removed text. Replacing an
    /// empty range with `""` changes nothing. A replace never joins a typing group.
    ///
    /// Errors, in this order (see [`EditError`]): `start > end` → `InvertedRange{start, end}`
    /// even if a bound is also out of range; else the first of `start`, `end` beyond
    /// `len_chars()` → `OutOfBounds{index, len_chars}`. The range is checked before the edit's
    /// emptiness, so `replace(9..9, "")` on a 3-char buffer is `OutOfBounds{index: 9,
    /// len_chars: 3}`, not a no-op `Ok`. On error nothing changes, and an open typing group
    /// stays open (a rejected replace does not end it).
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
    ///
    /// An open typing group is itself the most recent step, so `undo` never returns `false`
    /// while a group is open; a successful `undo` ends the group.
    ///
    /// The step's inverse edits are applied from the last one (highest position) to the first,
    /// and the selection and anchors are mapped through each of them.
    pub fn undo(&mut self) -> bool {
        let Some(step) = self.history.pop_undo() else {
            return false;
        };
        let starts = step.starts_after();
        for (op, at) in step.ops.iter().zip(starts).rev() {
            self.apply(at, at + op.inserted_chars, &op.removed);
        }
        self.history.undone(step);
        true
    }

    /// Redo the most recently undone step. Returns `false` (and changes nothing) if there is
    /// none.
    ///
    /// "Changes nothing" includes the open typing group: after a `redo` that returns `false`,
    /// the next qualifying single-char insert still joins the group. A successful `redo` ends
    /// the group.
    ///
    /// The step's edits are re-applied as they were first applied (from the last one to the
    /// first), and the selection and anchors are mapped through each of them.
    pub fn redo(&mut self) -> bool {
        let Some(step) = self.history.pop_redo() else {
            return false;
        };
        for op in step.ops.iter().rev() {
            self.apply(op.at, op.at + op.removed_chars, &op.inserted);
        }
        self.history.redone(step);
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

    /// Number of line breaks of kind `kind` in the text (CRLF matched first).
    pub fn line_break_count(&self, kind: LineEnding) -> usize {
        match kind {
            LineEnding::Lf => self.breaks.lf,
            LineEnding::Crlf => self.breaks.crlf,
            LineEnding::Cr => self.breaks.cr,
        }
    }

    /// The selection (a new buffer's is `0, 0`).
    pub fn selection(&self) -> Selection {
        self.selection
    }

    /// Set the selection. Not an edit: the history and an open typing group are unchanged.
    ///
    /// Fails with OutOfBounds, or InsideLineBreak for a position between the \r and \n of a CRLF.
    /// `anchor` is checked before `head`; on error nothing changes.
    pub fn set_selection(&mut self, s: Selection) -> Result<(), PositionError> {
        self.check_position(s.anchor)?;
        self.check_position(s.head)?;
        self.selection = s;
        Ok(())
    }

    /// Place an anchor at `pos` (same validity as [`Self::set_selection`]); it follows edits
    /// by the Mapping rule until removed. Not an edit.
    pub fn add_anchor(&mut self, pos: usize) -> Result<AnchorId, PositionError> {
        self.check_position(pos)?;
        let id = AnchorId::new(self.next_anchor);
        self.next_anchor += 1;
        self.anchors.push((id, pos));
        Ok(id)
    }

    /// The anchor's current position; `None` once removed.
    pub fn anchor(&self, id: AnchorId) -> Option<usize> {
        self.anchors
            .binary_search_by_key(&id, |&(i, _)| i)
            .ok()
            .map(|k| self.anchors[k].1)
    }

    /// Remove an anchor; `false` if it does not exist (any more). Not an edit.
    pub fn remove_anchor(&mut self, id: AnchorId) -> bool {
        match self.anchors.binary_search_by_key(&id, |&(i, _)| i) {
            Ok(k) => {
                self.anchors.remove(k);
                true
            }
            Err(_) => false,
        }
    }

    /// The head's new position for one movement step (does not change the buffer).
    ///
    /// `from` beyond the text is taken as `len_chars()`, and `from` between the `\r` and `\n`
    /// of a CRLF as the position before the `\r`.
    /// - `Char`: one char, two over a CRLF, never past either end.
    /// - `Word` forward: skip non-word chars, then word chars (word char: `_` or
    ///   alphanumeric); backward: the mirror image.
    /// - `Line`: the same column on the next or previous line, clamped to that line's content
    ///   length; up from the first line gives 0, down from the last gives `len_chars()`.
    pub fn moved(&self, from: usize, by: Movement, forward: bool) -> usize {
        let len = self.len_chars();
        let from = self.snap(from.min(len));
        match (by, forward) {
            (Movement::Char, true) => {
                if from >= len {
                    len
                } else if self.rope.char(from) == '\r'
                    && from + 1 < len
                    && self.rope.char(from + 1) == '\n'
                {
                    from + 2
                } else {
                    from + 1
                }
            }
            (Movement::Char, false) => {
                if from == 0 {
                    0
                } else if from >= 2
                    && self.rope.char(from - 1) == '\n'
                    && self.rope.char(from - 2) == '\r'
                {
                    from - 2
                } else {
                    from - 1
                }
            }
            (Movement::Word, true) => {
                let mut i = from;
                while i < len && !is_word_char(self.rope.char(i)) {
                    i += 1;
                }
                while i < len && is_word_char(self.rope.char(i)) {
                    i += 1;
                }
                i
            }
            (Movement::Word, false) => {
                let mut i = from;
                while i > 0 && !is_word_char(self.rope.char(i - 1)) {
                    i -= 1;
                }
                while i > 0 && is_word_char(self.rope.char(i - 1)) {
                    i -= 1;
                }
                i
            }
            (Movement::Line, forward) => {
                let line = self.rope.char_to_line(from);
                let col = from - self.rope.line_to_char(line);
                let target = if forward {
                    if line + 1 >= self.len_lines() {
                        return len;
                    }
                    line + 1
                } else {
                    if line == 0 {
                        return 0;
                    }
                    line - 1
                };
                self.rope.line_to_char(target) + col.min(self.line_content_len(target))
            }
        }
    }

    /// Apply `edits` (ranges in the current text) as one undo step.
    ///
    /// Batch rule: `edits[i].start < edits[i+1].start` and `edits[i].end <= edits[i+1].start`;
    /// each range in bounds and not inverted. Checked in index order, each edit's own range
    /// before its order; the first failure is returned and nothing changes. The effect is that
    /// of applying the edits from the last to the first, mapping the selection and anchors
    /// through each. No-op edits (empty range, empty text) are skipped. A batch that changes
    /// anything is one undo step that ends any open typing group and is never joined; a batch
    /// of only no-ops (or an empty list) changes nothing, the history and typing group
    /// included.
    pub fn apply_edits(&mut self, edits: &[(Range<usize>, &str)]) -> Result<(), BatchError> {
        for (index, (range, _)) in edits.iter().enumerate() {
            self.check_range(range)
                .map_err(|error| BatchError::Edit { index, error })?;
            if let Some((prev, _)) = index.checked_sub(1).map(|i| &edits[i])
                && (range.start <= prev.start || range.start < prev.end)
            {
                return Err(BatchError::Unordered { index });
            }
        }
        let mut ops = Vec::new();
        for (range, text) in edits.iter().rev() {
            if range.is_empty() && text.is_empty() {
                continue;
            }
            let removed = self.apply(range.start, range.end, text);
            ops.push(Op::new(range.start, removed, text));
        }
        if !ops.is_empty() {
            ops.reverse();
            self.history.record_batch(ops);
        }
        Ok(())
    }

    /// Rewrite every line break as `to`, as one undo step; returns the number of breaks rewritten.
    ///
    /// Behaves as one [`Self::apply_edits`] call with one edit per line break whose text
    /// differs from `to`, in text order. If no break differs, nothing changes (history and an
    /// open typing group included) and the result is 0.
    pub fn normalize_line_endings(&mut self, to: LineEnding) -> usize {
        let target = match to {
            LineEnding::Lf => "\n",
            LineEnding::Crlf => "\r\n",
            LineEnding::Cr => "\r",
        };
        let mut edits: Vec<(Range<usize>, &str)> = Vec::new();
        let mut chars = self.rope.chars().enumerate().peekable();
        while let Some((i, c)) = chars.next() {
            let kind = match c {
                '\r' if chars.peek().map(|&(_, n)| n) == Some('\n') => {
                    chars.next();
                    LineEnding::Crlf
                }
                '\r' => LineEnding::Cr,
                '\n' => LineEnding::Lf,
                _ => continue,
            };
            if kind != to {
                let width = if kind == LineEnding::Crlf { 2 } else { 1 };
                edits.push((i..i + width, target));
            }
        }
        let changed = edits.len();
        // The edits are sorted, disjoint and in bounds by construction.
        if self.apply_edits(&edits).is_err() {
            unreachable!("line-break edits are always a valid batch");
        }
        changed
    }

    /// Position validity of the selection and anchors: `0..=len_chars()`, not between the
    /// `\r` and `\n` of a CRLF.
    fn check_position(&self, pos: usize) -> Result<(), PositionError> {
        self.char_to_line_col(pos).map(|_| ())
    }

    /// True iff `pos` lies between the `\r` and `\n` of a CRLF.
    fn inside_crlf(&self, pos: usize) -> bool {
        pos > 0
            && pos < self.len_chars()
            && self.rope.char(pos - 1) == '\r'
            && self.rope.char(pos) == '\n'
    }

    /// `pos`, moved to before the `\r` if it lies inside a CRLF.
    fn snap(&self, pos: usize) -> usize {
        if self.inside_crlf(pos) { pos - 1 } else { pos }
    }

    /// Map the selection and every anchor through the edit that just replaced `[s, e)` by `n`
    /// chars (Mapping rule, then the CRLF snap in the new text).
    fn map_positions(&mut self, s: usize, e: usize, n: usize) {
        let sel = self.selection;
        self.selection = Selection {
            anchor: self.snap(map_position(sel.anchor, s, e, n)),
            head: self.snap(map_position(sel.head, s, e, n)),
        };
        for k in 0..self.anchors.len() {
            let p = self.snap(map_position(self.anchors[k].1, s, e, n));
            self.anchors[k].1 = p;
        }
    }

    /// Replace chars `start..end` (already validated) with `text`, keep the break counts up to
    /// date, map the selection and anchors, and return the removed text. Reads only the edited
    /// region and one char on each side of it (plus one char per mapped position).
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
        self.map_positions(start, end, text.chars().count());
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

    // ---- T-018 unit tests ------------------------------------------------------------------

    /// w3: selection and anchor calls are not edits, whether they succeed or fail.
    #[test]
    fn selection_and_anchor_calls_leave_history_shape() {
        let mut b = Buffer::from_text("a\r\nb");
        b.insert(0, "x").unwrap();
        let before = b.history.shape();
        assert_eq!(before, (1, 0, true));
        b.set_selection(Selection { anchor: 1, head: 0 }).unwrap();
        assert!(b.set_selection(Selection { anchor: 3, head: 0 }).is_err());
        let id = b.add_anchor(5).unwrap();
        assert!(b.add_anchor(6).is_err());
        assert!(b.remove_anchor(id) && !b.remove_anchor(id));
        assert_eq!(b.history.shape(), before);
    }

    /// w2: a normalize that changes nothing leaves the history and the open group as they are;
    /// one that changes something is one closed step.
    #[test]
    fn normalize_history_shape() {
        let mut b = Buffer::from_text("p\nq");
        b.insert(0, "x").unwrap();
        assert_eq!(b.normalize_line_endings(LineEnding::Lf), 0);
        assert_eq!(b.history.shape(), (1, 0, true));
        assert_eq!(b.normalize_line_endings(LineEnding::Crlf), 1);
        assert_eq!(b.history.shape(), (2, 0, false));
        assert!(b.undo());
        assert_eq!(b.text(), "xp\nq");
        assert_eq!(b.history.shape(), (1, 1, false));
    }

    #[test]
    fn rejected_batch_changes_nothing() {
        let mut b = Buffer::from_text("abc");
        b.insert(3, "d").unwrap();
        let id = b.add_anchor(4).unwrap();
        let shape = b.history.shape();
        assert_eq!(
            b.apply_edits(&[(0..1, "x"), (2..9, "y")]),
            Err(BatchError::Edit {
                index: 1,
                error: EditError::OutOfBounds {
                    index: 9,
                    len_chars: 4
                }
            })
        );
        assert_eq!(b.text(), "abcd");
        assert_eq!(b.anchor(id), Some(4));
        assert_eq!(b.history.shape(), shape);
    }

    #[test]
    fn step_starts_after_shift_by_earlier_ops() {
        let mut b = Buffer::from_text("abcdef");
        b.apply_edits(&[(0..1, "XYZ"), (2..4, ""), (5..5, "!")])
            .unwrap();
        assert_eq!(b.text(), "XYZbe!f");
        assert!(b.undo());
        assert_eq!(b.text(), "abcdef");
        assert!(b.redo());
        assert_eq!(b.text(), "XYZbe!f");
    }

    #[test]
    fn moved_clamps_invalid_from() {
        let b = Buffer::from_text("a\r\nb");
        assert_eq!(b.moved(9, Movement::Char, true), 4);
        assert_eq!(b.moved(2, Movement::Char, false), 0);
        assert_eq!(b.moved(2, Movement::Char, true), 3); // from 1, over the CRLF
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
