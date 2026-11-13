//! Regular expressions with char-offset matches (T-017).
//!
//! The engine is the `regex` crate 1.13 with Unicode enabled, multi-line mode and CRLF mode on:
//! `^` matches at the start of the text and after every `\n`, `\r\n` and lone `\r`; `$` matches
//! at the end and before each of these, never between the `\r` and `\n` of a CRLF; `.` matches
//! any char except `\r` and `\n`. Inline flags (`(?i)`, `(?-m)`, `(?s)`, `(?-R)`, `(?x)`, …) are
//! accepted and override the options and these modes for their scope, as in the crate.
//!
//! All offsets are chars (Unicode scalar values), the control protocol's unit; a [`CharMatch`]
//! never splits a char.
//!
//! # Iteration
//! [`Regex::find_all`] is leftmost-first and non-overlapping. The search starts at char 0. After
//! a match ending at `e` it resumes at `e`; if the leftmost-first match found from `e` is empty
//! and starts at `e` (the previous match's end), it is discarded and the search restarts at
//! `e + 1`, without trying another match at `e`. After a reported empty match at `s` the search
//! resumes at `s + 1`. Anchors and `\b` always see the whole text. A group inside a repetition
//! holds its range from the last iteration in which it took part. Quantified sub-patterns that
//! can match the empty string (`(a*)*`, `(|a)+`, …) give the engine's results.
//!
//! # Expansion
//! [`Regex::expand`] scans the replacement left to right: `$$` is `$`; `${name}` (any chars up
//! to the next `}`, possibly none) and `$name` (the longest non-empty run of `[0-9A-Za-z_]`) are
//! references; any other `$` (including `${` with no later `}`) is literal. A name of ASCII
//! digits that parses as `usize` is a group number, any other name a group name. A reference to
//! an existing group expands to its text, or to nothing if it did not take part (or if `caps`
//! has no such entry or holds a range that does not fit `text`). A reference to a group the
//! pattern does not have is a [`TemplateError`]; unlike `regex::Captures::expand`, which expands
//! it to nothing.

use std::fmt;

/// Longest [`PatternError::message`], in chars.
const MAX_MESSAGE_CHARS: usize = 200;

/// A compiled regular expression whose matches are char offsets.
#[derive(Clone, Debug)]
pub struct Regex {
    /// The engine, built with multi-line and CRLF mode.
    engine: ::regex::Regex,
    /// The name of each group by index (`[0]`, the whole match, has none).
    names: Vec<Option<String>>,
}

/// Options for [`Regex::new`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RegexOptions {
    /// Match with Unicode simple case folding (`σ` matches `Σ` and `ς`).
    pub case_insensitive: bool,
}

/// A range of chars: `start..end`, with `start <= end` for ranges [`Regex`] reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CharMatch {
    /// First char of the range.
    pub start: usize,
    /// One past the last char of the range.
    pub end: usize,
}

/// One match: `groups[0]` is the whole match, `groups[i]` group `i` (`None` if it did not take
/// part). For matches from [`Regex::find_all`], `groups.len() == group_count() + 1`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Captures {
    /// The whole match, then each explicit group.
    pub groups: Vec<Option<CharMatch>>,
}

/// A pattern that does not compile (syntax, size limits, nesting deeper than 250).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternError {
    /// The engine's message, cut to 199 chars plus `…` when longer than 200 chars. Not stable.
    pub message: String,
}

impl fmt::Display for PatternError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for PatternError {}

/// A replacement template that refers to a group the pattern does not have.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemplateError {
    /// The reference's name exactly as written (`9` for `$9`, `a!` for `${a!}`).
    pub reference: String,
}

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "no capture group \"{}\"", self.reference)
    }
}

impl std::error::Error for TemplateError {}

/// `message` cut to its first 199 chars plus `…` if it is longer than 200 chars.
fn truncate_message(message: &str) -> String {
    if message.chars().count() <= MAX_MESSAGE_CHARS {
        return message.to_owned();
    }
    let mut cut: String = message.chars().take(MAX_MESSAGE_CHARS - 1).collect();
    cut.push('…');
    cut
}

/// Converts increasing byte offsets of `text` to char offsets in one forward pass.
struct CharCursor<'t> {
    text: &'t str,
    byte: usize,
    chars: usize,
}

impl<'t> CharCursor<'t> {
    fn new(text: &'t str) -> Self {
        CharCursor {
            text,
            byte: 0,
            chars: 0,
        }
    }

    /// Moves the cursor to byte `b` (a char boundary, `b >= self.byte`).
    fn advance_to(&mut self, b: usize) {
        if let Some(s) = self.text.get(self.byte..b) {
            self.chars += s.chars().count();
            self.byte = b;
        }
    }

    /// Char offset of byte `b` (a char boundary, `b >= self.byte`), without moving the cursor.
    fn chars_at(&self, b: usize) -> usize {
        self.chars + self.text.get(self.byte..b).map_or(0, |s| s.chars().count())
    }
}

/// Byte offset of the char after the one starting at byte `b`, or `None` at the end of `text`.
fn next_char_boundary(text: &str, b: usize) -> Option<usize> {
    text.get(b..)?.chars().next().map(|c| b + c.len_utf8())
}

/// Byte range of the chars `m.start..m.end` of `text`, if `m` fits.
fn byte_range(text: &str, m: CharMatch) -> Option<(usize, usize)> {
    if m.start > m.end {
        return None;
    }
    let mut start = None;
    // `char_indices` plus the end of the text: the byte offset of every char position.
    let positions = text
        .char_indices()
        .map(|(b, _)| b)
        .chain(std::iter::once(text.len()));
    for (i, b) in positions.enumerate() {
        if i == m.start {
            start = Some(b);
        }
        if i == m.end {
            return start.map(|s| (s, b));
        }
    }
    None
}

/// A reference in a template, as scanned.
enum Piece<'r> {
    /// Text copied as is.
    Literal(&'r str),
    /// A `$name` or `${name}` reference.
    Reference(&'r str),
}

/// Splits `replacement` into literal text and references, by the expansion rule.
fn scan(replacement: &str) -> Vec<Piece<'_>> {
    let mut pieces = Vec::new();
    let mut rest = replacement;
    while let Some(dollar) = rest.find('$') {
        if dollar > 0 {
            pieces.push(Piece::Literal(&rest[..dollar]));
        }
        let after = &rest[dollar + 1..];
        if let Some(tail) = after.strip_prefix('$') {
            pieces.push(Piece::Literal("$"));
            rest = tail;
        } else if let Some(braced) = after.strip_prefix('{')
            && let Some(close) = braced.find('}')
        {
            pieces.push(Piece::Reference(&braced[..close]));
            rest = &braced[close + 1..];
        } else {
            let run = after
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .unwrap_or(after.len());
            if run == 0 {
                pieces.push(Piece::Literal("$"));
            } else {
                pieces.push(Piece::Reference(&after[..run]));
            }
            rest = &after[run..];
        }
    }
    if !rest.is_empty() {
        pieces.push(Piece::Literal(rest));
    }
    pieces
}

impl Regex {
    /// Compiles `pattern` with multi-line and CRLF mode on and the given options.
    ///
    /// The empty pattern is valid: it matches at every char position.
    ///
    /// # Errors
    /// [`PatternError`] if the pattern does not compile, exceeds the engine's default size
    /// limits or nests groups deeper than 250.
    pub fn new(pattern: &str, options: RegexOptions) -> Result<Regex, PatternError> {
        let engine = ::regex::RegexBuilder::new(pattern)
            .multi_line(true)
            .crlf(true)
            .case_insensitive(options.case_insensitive)
            .build()
            .map_err(|e| PatternError {
                message: truncate_message(&e.to_string()),
            })?;
        let names = engine
            .capture_names()
            .map(|n| n.map(str::to_owned))
            .collect();
        Ok(Regex { engine, names })
    }

    /// The number of explicit groups (named or not) in the pattern.
    pub fn group_count(&self) -> usize {
        self.names.len().saturating_sub(1)
    }

    /// All matches, in the iteration order of the module documentation.
    pub fn find_all(&self, text: &str) -> Vec<Captures> {
        let mut out = Vec::new();
        let mut cursor = CharCursor::new(text);
        let mut locs = self.engine.capture_locations();
        // Byte position the next search starts at, and the end of the last reported match.
        let mut at = 0;
        let mut last_end: Option<usize> = None;
        while at <= text.len() {
            let Some(whole) = self.engine.captures_read_at(&mut locs, text, at) else {
                break;
            };
            let (start, end) = (whole.start(), whole.end());
            if start == end && last_end == Some(start) {
                // Empty match at the previous match's end: discard, restart one char later.
                match next_char_boundary(text, start) {
                    Some(next) => at = next,
                    None => break,
                }
                continue;
            }
            cursor.advance_to(start);
            let groups = (0..locs.len())
                .map(|i| {
                    locs.get(i).map(|(s, e)| CharMatch {
                        start: cursor.chars_at(s),
                        end: cursor.chars_at(e),
                    })
                })
                .collect();
            out.push(Captures { groups });
            last_end = Some(end);
            if start == end {
                // After a reported empty match at `s`, resume at `s + 1`.
                match next_char_boundary(text, end) {
                    Some(next) => at = next,
                    None => break,
                }
            } else {
                at = end;
            }
        }
        out
    }

    /// The index of the group a reference names, if the pattern has it.
    fn group_index(&self, name: &str) -> Option<usize> {
        // `parse` alone would accept a leading `+`; a number is ASCII digits only.
        if !name.is_empty()
            && name.bytes().all(|b| b.is_ascii_digit())
            && let Ok(n) = name.parse::<usize>()
        {
            return (n <= self.group_count()).then_some(n);
        }
        self.names.iter().position(|n| n.as_deref() == Some(name))
    }

    /// `replacement` with `$` references to `caps` (taken from `text`) expanded.
    ///
    /// # Errors
    /// [`TemplateError`] for the first reference, from the left, to a group the pattern does
    /// not have. Whether a template is an error depends only on the pattern and `replacement`.
    pub fn expand(
        &self,
        caps: &Captures,
        text: &str,
        replacement: &str,
    ) -> Result<String, TemplateError> {
        let mut out = String::with_capacity(replacement.len());
        for piece in scan(replacement) {
            match piece {
                Piece::Literal(s) => out.push_str(s),
                Piece::Reference(name) => {
                    let index = self.group_index(name).ok_or_else(|| TemplateError {
                        reference: name.to_owned(),
                    })?;
                    let range = caps.groups.get(index).copied().flatten();
                    if let Some((s, e)) = range.and_then(|m| byte_range(text, m)) {
                        out.push_str(&text[s..e]);
                    }
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn re(p: &str) -> Regex {
        Regex::new(p, RegexOptions::default()).unwrap()
    }

    fn m(start: usize, end: usize) -> Option<CharMatch> {
        Some(CharMatch { start, end })
    }

    fn wholes(r: &Regex, text: &str) -> Vec<(usize, usize)> {
        r.find_all(text)
            .iter()
            .map(|c| {
                let w = c.groups[0].unwrap();
                (w.start, w.end)
            })
            .collect()
    }

    #[test]
    fn truncation_keeps_short_messages_and_cuts_long_ones_by_char() {
        let short: String = "é".repeat(200);
        assert_eq!(truncate_message(&short), short);
        let long: String = "é".repeat(201);
        let cut = truncate_message(&long);
        assert_eq!(cut.chars().count(), 200);
        assert_eq!(cut, format!("{}…", "é".repeat(199)));
    }

    #[test]
    fn empty_match_after_a_match_is_discarded() {
        assert_eq!(wholes(&re("a*"), "baaab"), vec![(0, 0), (1, 4), (5, 5)]);
        assert_eq!(wholes(&re("b|(?:)|a"), "ba"), vec![(0, 1), (2, 2)]);
    }

    #[test]
    fn empty_pattern_matches_at_every_char_position() {
        assert_eq!(
            wholes(&re(""), "é😀\r\n"),
            vec![(0, 0), (1, 1), (2, 2), (3, 3), (4, 4)]
        );
        assert_eq!(wholes(&re(""), ""), vec![(0, 0)]);
    }

    #[test]
    fn offsets_are_chars_with_groups() {
        let r = re("(中)(x)?");
        assert_eq!(r.group_count(), 2);
        let caps = r.find_all("é😀中a中");
        assert_eq!(
            caps,
            vec![
                Captures {
                    groups: vec![m(2, 3), m(2, 3), None]
                },
                Captures {
                    groups: vec![m(4, 5), m(4, 5), None]
                },
            ]
        );
    }

    #[test]
    fn scan_splits_references_and_literals() {
        let names: Vec<String> = scan("a$$b${x}$1a$!${c")
            .into_iter()
            .map(|p| match p {
                Piece::Literal(s) => format!("L{s}"),
                Piece::Reference(s) => format!("R{s}"),
            })
            .collect();
        assert_eq!(
            names,
            ["La", "L$", "Lb", "Rx", "R1a", "L$", "L!", "L$", "L{c"]
        );
    }

    #[test]
    fn byte_range_rejects_ranges_that_do_not_fit() {
        assert_eq!(
            byte_range("é中", CharMatch { start: 1, end: 2 }),
            Some((2, 5))
        );
        assert_eq!(
            byte_range("é中", CharMatch { start: 2, end: 2 }),
            Some((5, 5))
        );
        assert_eq!(byte_range("é中", CharMatch { start: 1, end: 3 }), None);
        assert_eq!(byte_range("é中", CharMatch { start: 2, end: 1 }), None);
    }

    #[test]
    fn group_index_numbers_and_names() {
        let r = re("(?<w>a)(b)?");
        assert_eq!(r.group_index("0"), Some(0));
        assert_eq!(r.group_index("02"), Some(2));
        assert_eq!(r.group_index("3"), None);
        assert_eq!(r.group_index("w"), Some(1));
        assert_eq!(r.group_index(""), None);
        assert_eq!(r.group_index("99999999999999999999"), None);
    }
}
