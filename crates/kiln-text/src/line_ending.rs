//! Line-break kinds, the line-ending style of a text and per-kind break counts.
//!
//! A line break is `\r\n`, a lone `\n` or a lone `\r`, matched CRLF first. No other char
//! (VT, FF, NEL, U+2028, U+2029) breaks a line.

/// Line-ending style of a text. A line break is `\r\n`, a lone `\n` or a lone `\r`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LineEndings {
    /// The text has no line break.
    None,
    /// Every line break is `\n`.
    Lf,
    /// Every line break is `\r\n`.
    Crlf,
    /// Every line break is a lone `\r`.
    Cr,
    /// The text has line breaks of more than one kind.
    Mixed,
}

/// One kind of line break.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LineEnding {
    /// `\n`.
    Lf,
    /// `\r\n`.
    Crlf,
    /// A lone `\r`.
    Cr,
}

/// Number of line breaks of each kind in a text (CRLF matched first).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct BreakCounts {
    pub(crate) lf: usize,
    pub(crate) crlf: usize,
    pub(crate) cr: usize,
}

impl BreakCounts {
    /// Count the breaks of `text` as a standalone string: a leading `\n` counts as LF and a
    /// trailing `\r` as CR, whatever surrounds the string.
    pub(crate) fn of(text: &str) -> Self {
        let mut counts = Self::default();
        let mut bytes = text.bytes().peekable();
        while let Some(b) = bytes.next() {
            match b {
                b'\r' if bytes.peek() == Some(&b'\n') => {
                    bytes.next();
                    counts.crlf += 1;
                }
                b'\r' => counts.cr += 1,
                b'\n' => counts.lf += 1,
                _ => {}
            }
        }
        counts
    }

    /// `self - old + new`, for replacing the breaks of one window of the text by another's.
    ///
    /// `old` and `new` may both miscount a margin char at a window edge (a `\n` whose `\r` is
    /// outside the window counts as LF), so `old` alone can exceed `self` in one field; the
    /// miscount is the same in `new`, so adding first never underflows.
    pub(crate) fn replaced(self, old: Self, new: Self) -> Self {
        Self {
            lf: self.lf + new.lf - old.lf,
            crlf: self.crlf + new.crlf - old.crlf,
            cr: self.cr + new.cr - old.cr,
        }
    }

    pub(crate) fn total(self) -> usize {
        self.lf + self.crlf + self.cr
    }

    pub(crate) fn style(self) -> LineEndings {
        match (self.lf > 0, self.crlf > 0, self.cr > 0) {
            (false, false, false) => LineEndings::None,
            (true, false, false) => LineEndings::Lf,
            (false, true, false) => LineEndings::Crlf,
            (false, false, true) => LineEndings::Cr,
            _ => LineEndings::Mixed,
        }
    }

    /// The most frequent kind; ties go to Lf, then Crlf, then Cr.
    pub(crate) fn dominant(self) -> Option<LineEnding> {
        if self.total() == 0 {
            return None;
        }
        let mut best = (LineEnding::Lf, self.lf);
        for candidate in [(LineEnding::Crlf, self.crlf), (LineEnding::Cr, self.cr)] {
            if candidate.1 > best.1 {
                best = candidate;
            }
        }
        Some(best.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_match_crlf_first_split() {
        assert_eq!(
            BreakCounts::of("a\r\r\nb\n\rc\r"),
            BreakCounts {
                lf: 1,
                crlf: 1,
                cr: 3
            }
        );
        assert_eq!(BreakCounts::of(""), BreakCounts::default());
    }

    #[test]
    fn other_unicode_separators_are_not_breaks() {
        assert_eq!(
            BreakCounts::of("a\u{0B}b\u{0C}c\u{85}d\u{2028}e\u{2029}f"),
            BreakCounts::default()
        );
    }

    #[test]
    fn dominant_tie_order_is_lf_crlf_cr() {
        let all = BreakCounts {
            lf: 2,
            crlf: 2,
            cr: 2,
        };
        assert_eq!(all.dominant(), Some(LineEnding::Lf));
        let crlf_cr = BreakCounts {
            lf: 1,
            crlf: 2,
            cr: 2,
        };
        assert_eq!(crlf_cr.dominant(), Some(LineEnding::Crlf));
        let cr = BreakCounts {
            lf: 1,
            crlf: 0,
            cr: 2,
        };
        assert_eq!(cr.dominant(), Some(LineEnding::Cr));
    }
}
