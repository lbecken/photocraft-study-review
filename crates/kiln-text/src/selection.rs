//! Selections, anchors and cursor movement (T-018).
//!
//! A [`Selection`] is an anchor and a head (char offsets). Selection ends and anchors are
//! *mapped* through every primitive edit by the Mapping rule: an edit that replaces chars
//! `[s, e)` with `n` chars sends a position `p` to
//! - `p` if `p < s`;
//! - `p + n - (e - s)` if `p > e`;
//! - `s` if `p == s` and `e > s`;
//! - `s + n` otherwise (`s < p <= e`, or `p == s` for a pure insertion),
//!
//! and then, if the result lies between the `\r` and `\n` of a CRLF, to just before the `\r`
//! (done by the buffer, which sees the text).

/// One selection: an anchor and a head, both char offsets. The head is the caret.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    /// The end that stays put while the selection is extended.
    pub anchor: usize,
    /// The end that moves (the caret).
    pub head: usize,
}

impl Selection {
    /// The smaller end.
    pub fn start(self) -> usize {
        self.anchor.min(self.head)
    }

    /// The larger end.
    pub fn end(self) -> usize {
        self.anchor.max(self.head)
    }

    /// True iff anchor and head are equal (a caret).
    pub fn is_empty(self) -> bool {
        self.anchor == self.head
    }
}

/// Handle of an anchor placed with [`Buffer::add_anchor`](crate::Buffer::add_anchor). Ids are
/// never reused within one buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AnchorId(u64);

impl AnchorId {
    pub(crate) fn new(n: u64) -> Self {
        Self(n)
    }
}

/// The unit of one cursor movement step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Movement {
    /// One char; a CRLF counts as one step.
    Char,
    /// To the end (forward) or start (backward) of the next word.
    Word,
    /// To the same column on the next or previous line, clamped to its length.
    Line,
}

/// Word char of the Movement rule.
pub(crate) fn is_word_char(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

/// The Mapping rule for one primitive edit replacing `[s, e)` by `n` chars (before the CRLF
/// snap, which needs the text).
pub(crate) fn map_position(p: usize, s: usize, e: usize, n: usize) -> usize {
    if p < s {
        p
    } else if p > e {
        p + n - (e - s)
    } else if p == s && e > s {
        s
    } else {
        s + n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_start_end_empty() {
        let s = Selection { anchor: 7, head: 2 };
        assert_eq!((s.start(), s.end(), s.is_empty()), (2, 7, false));
        let c = Selection { anchor: 3, head: 3 };
        assert_eq!((c.start(), c.end(), c.is_empty()), (3, 3, true));
        assert_eq!(Selection::default(), Selection { anchor: 0, head: 0 });
    }

    #[test]
    fn mapping_rule_four_cases() {
        // Replace [4, 7) by 2 chars.
        assert_eq!(map_position(3, 4, 7, 2), 3); // before
        assert_eq!(map_position(9, 4, 7, 2), 8); // after: 9 + 2 - 3
        assert_eq!(map_position(4, 4, 7, 2), 4); // at start of a non-empty range
        assert_eq!(map_position(5, 4, 7, 2), 6); // inside: s + n
        assert_eq!(map_position(7, 4, 7, 2), 6); // at end: s + n
        // Pure insertion of 3 chars at 4: a caret at the insertion point ends up after it.
        assert_eq!(map_position(4, 4, 4, 3), 7);
        assert_eq!(map_position(3, 4, 4, 3), 3);
        assert_eq!(map_position(5, 4, 4, 3), 8);
        // Pure deletion of [4, 7).
        assert_eq!(map_position(6, 4, 7, 0), 4);
        assert_eq!(map_position(8, 4, 7, 0), 5);
    }

    #[test]
    fn word_chars() {
        for c in ['a', 'Z', '0', '_', 'é', '中'] {
            assert!(is_word_char(c), "{c:?}");
        }
        for c in [' ', '(', ':', '\r', '\n', '😀', '-'] {
            assert!(!is_word_char(c), "{c:?}");
        }
    }
}
