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
//! resumes at `s + 1`. [`Regex::for_each_match`] visits the same matches in the same order,
//! one at a time (T-021). Anchors and `\b` always see the whole text. A group inside a repetition
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
//!
//! # Limits
//! Besides the engine's own limits (default size limits, nesting deeper than 250), a pattern
//! with more than 100 explicit capture groups is refused with
//! [`PatternError`] (D-060 s1): capture slots times engine states made `find_all` on `(a)`
//! repeated 34,000 times try to allocate about 55 GB. The check runs after the engine has
//! accepted the pattern, so a pattern the engine rejects keeps the engine's message.
//!
//! The cap bounds the engine's working memory, not the result (D-070 s4): [`Regex::find_all`]
//! returns every match with `group_count() + 1` slots, so its memory is proportional to
//! matches × (groups + 1), about 2.4 kB per match at 100 groups; 100 empty groups on a 1 MB
//! text peaked at about 2.46 GB (2.29 GiB). A caller handling untrusted text or patterns must
//! bound the search itself: [`Regex::for_each_match`] (T-021) holds the slots of one match at a
//! time.
//!
//! Time (D-077 s8), in a debug build: compiling a pattern, including a 100 kB one, answers or
//! errors within 5 s. Searching with the small pathological patterns `(a*)*b` on 100,000 `a`s,
//! `\p{L}{1000}` and deeply nested groups also answers or errors within 5 s. For searching with
//! a large pattern this crate promises no time bound, on matching or non-matching text: the
//! engine's search is linear in the text only for a fixed pattern, and its cost grows with
//! pattern size × text size. Measured in debug builds: 245 s for a 100 kB pattern (`[ab]`
//! repeated 25,000 times, then `[^ab]`) on a 100,000-char text it does not match, and 30 s and
//! more for 100 kB patterns on matching texts of 100–200 kB. A caller handling untrusted
//! patterns must bound the pattern size itself.

use std::fmt;
use std::ops::ControlFlow;

/// Most explicit capture groups a pattern may have (D-060 s1).
const MAX_GROUPS: usize = 100;

/// [`PatternError::message`] for a pattern with more than `MAX_GROUPS` groups.
const TOO_MANY_GROUPS: &str = "the pattern has more than 100 capture groups";

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

/// A pattern that does not compile (syntax, size limits, nesting deeper than 250) or has more
/// than 100 capture groups.
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

/// The byte range of each group of one match in the text (`None` if it did not take part),
/// indexed as `Captures::groups`.
pub(crate) type ByteGroups = [Option<(usize, usize)>];

/// A visitor of [`Regex::for_each_match_bytes`].
pub(crate) type MatchVisitor<'a> = dyn FnMut(&Captures, &ByteGroups) -> ControlFlow<()> + 'a;

/// A replacement template with every reference resolved to a group index (T-021).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Template {
    parts: Vec<Part>,
}

/// One part of a [`Template`].
#[derive(Clone, Debug, PartialEq, Eq)]
enum Part {
    /// Text copied as is.
    Literal(String),
    /// The text of this group, or nothing if it did not take part.
    Group(usize),
}

impl Template {
    /// A template that inserts `text` verbatim (literal mode).
    pub(crate) fn verbatim(text: &str) -> Template {
        Template {
            parts: vec![Part::Literal(text.to_owned())],
        }
    }

    /// The expansion for one match whose group byte ranges in `text` are `bytes` (as passed
    /// by [`Regex::for_each_match_bytes`]). Costs the length of the output, never a walk of
    /// `text`.
    pub(crate) fn expand_bytes(&self, text: &str, bytes: &ByteGroups) -> String {
        if let [Part::Literal(s)] = self.parts.as_slice() {
            return s.clone();
        }
        let mut out = String::new();
        for part in &self.parts {
            match part {
                Part::Literal(s) => out.push_str(s),
                Part::Group(i) => {
                    if let Some(Some((s, e))) = bytes.get(*i)
                        && let Some(t) = text.get(*s..*e)
                    {
                        out.push_str(t);
                    }
                }
            }
        }
        out
    }
}

impl Regex {
    /// Compiles `pattern` with multi-line and CRLF mode on and the given options.
    ///
    /// The empty pattern is valid: it matches at every char position.
    ///
    /// # Errors
    /// [`PatternError`] if the pattern does not compile, exceeds the engine's default size
    /// limits or nests groups deeper than 250, with the engine's message for `pattern` exactly
    /// as given (the options are builder settings, not an inline-flag prefix). Then, for a
    /// pattern the engine accepts with more than 100 explicit groups, `PatternError`
    /// with message `the pattern has more than 100 capture groups`.
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
            .collect::<Vec<_>>();
        if names.len().saturating_sub(1) > MAX_GROUPS {
            return Err(PatternError {
                message: TOO_MANY_GROUPS.to_owned(),
            });
        }
        Ok(Regex { engine, names })
    }

    /// The number of explicit groups (named or not) in the pattern.
    pub fn group_count(&self) -> usize {
        self.names.len().saturating_sub(1)
    }

    /// All matches, in the iteration order of the module documentation.
    ///
    /// Every match is returned with `group_count() + 1` slots, so memory is proportional to
    /// matches × (groups + 1): about 2.4 kB per match at 100 groups, and about 2.46 GB
    /// (2.29 GiB) measured for 100 empty groups on a 1 MB text. The 100-group cap bounds the
    /// engine's working memory, not this result; a caller handling untrusted text or patterns
    /// must bound the search itself (D-070 s4).
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

    /// Calls `f` once per match, in `find_all`'s order, with a `Captures` equal to the one
    /// `find_all(text)` holds at that position; stops after a call that returns
    /// `ControlFlow::Break(())`. Memory beyond the engine is O(group_count()), whatever the
    /// number of matches: the value passed to `f` is reused, so `f` copies what it keeps.
    pub fn for_each_match(&self, text: &str, f: &mut dyn FnMut(&Captures) -> ControlFlow<()>) {
        self.for_each_match_bytes(text, &mut |caps, _| f(caps));
    }

    /// [`Self::for_each_match`], also passing each group's byte range in `text` (`None` if it
    /// did not take part; index as in `Captures::groups`). Both values are reused.
    pub(crate) fn for_each_match_bytes(&self, text: &str, f: &mut MatchVisitor<'_>) {
        let mut cursor = CharCursor::new(text);
        let mut locs = self.engine.capture_locations();
        let mut caps = Captures {
            groups: vec![None; locs.len()],
        };
        let mut bytes: Vec<Option<(usize, usize)>> = vec![None; locs.len()];
        // The same loop as `find_all`: byte position the next search starts at, and the end
        // of the last reported match.
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
            for (i, (slot, byte)) in caps.groups.iter_mut().zip(bytes.iter_mut()).enumerate() {
                *byte = locs.get(i);
                *slot = byte.map(|(s, e)| CharMatch {
                    start: cursor.chars_at(s),
                    end: cursor.chars_at(e),
                });
            }
            if f(&caps, &bytes).is_break() {
                return;
            }
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
    }

    /// `replacement` with every reference resolved, by the expansion rule.
    ///
    /// # Errors
    /// The same [`TemplateError`] as [`Self::expand`] gives for `replacement`: the first
    /// reference, from the left, to a group the pattern does not have.
    pub(crate) fn template(&self, replacement: &str) -> Result<Template, TemplateError> {
        let mut parts = Vec::new();
        for piece in scan(replacement) {
            match piece {
                Piece::Literal(s) => match parts.last_mut() {
                    Some(Part::Literal(prev)) => prev.push_str(s),
                    _ => parts.push(Part::Literal(s.to_owned())),
                },
                Piece::Reference(name) => {
                    let index = self.group_index(name).ok_or_else(|| TemplateError {
                        reference: name.to_owned(),
                    })?;
                    parts.push(Part::Group(index));
                }
            }
        }
        Ok(Template { parts })
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
    fn group_cap_accepts_100_and_refuses_101() {
        let hundred = "(a)".repeat(100);
        let r = re(&hundred);
        assert_eq!(r.group_count(), 100);
        let caps = r.find_all(&"a".repeat(200));
        assert_eq!(caps.len(), 2);
        assert_eq!(caps[1].groups[100], m(199, 200));
        let e = Regex::new(&"(a)".repeat(101), RegexOptions::default()).unwrap_err();
        assert_eq!(e.message, "the pattern has more than 100 capture groups");
        // Named and nested groups count too; non-capturing groups do not.
        let named = format!("(?<x>a){}", "((a))".repeat(50));
        assert!(Regex::new(&named, RegexOptions::default()).is_err());
        assert_eq!(re(&"(?:a)".repeat(500)).group_count(), 0);
    }

    #[test]
    fn group_cap_refuses_34000_groups_quickly() {
        let t = std::time::Instant::now();
        let e = Regex::new(&"(a)".repeat(34_000), RegexOptions::default()).unwrap_err();
        assert_eq!(e.message, TOO_MANY_GROUPS);
        assert!(t.elapsed() < std::time::Duration::from_secs(5));
    }

    #[test]
    fn engine_rejection_keeps_the_engine_message() {
        // Over the cap and invalid: the engine's error wins.
        let p = format!("{}(", "(a)".repeat(101));
        let e = Regex::new(&p, RegexOptions::default()).unwrap_err();
        assert_ne!(e.message, TOO_MANY_GROUPS);
        // A runtime string: an invalid literal would trip clippy's `invalid_regex`.
        let bad = String::from("(?<1>a");
        let engine = ::regex::RegexBuilder::new(&bad)
            .multi_line(true)
            .crlf(true)
            .build()
            .unwrap_err()
            .to_string();
        assert_eq!(
            Regex::new("(?<1>a", RegexOptions::default())
                .unwrap_err()
                .message,
            engine
        );
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
    fn for_each_match_reuses_one_value_and_stops_on_break() {
        let r = re("(a)|(b)");
        let mut seen = Vec::new();
        r.for_each_match("abxa", &mut |c| {
            seen.push(c.clone());
            ControlFlow::Continue(())
        });
        assert_eq!(seen, r.find_all("abxa"));
        let mut n = 0;
        r.for_each_match("abxa", &mut |_| {
            n += 1;
            if n == 2 {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        });
        assert_eq!(n, 2);
    }

    #[test]
    fn template_expands_like_expand() {
        let r = re("(?<w>é)(x)?😀");
        let text = "aé😀bé😀";
        for t in ["$w-$2-$0", "$$1${1}", "", "plain", "$!${"] {
            let template = r.template(t).unwrap();
            let mut got = Vec::new();
            r.for_each_match_bytes(text, &mut |c, b| {
                got.push((
                    template.expand_bytes(text, b),
                    r.expand(c, text, t).unwrap(),
                ));
                ControlFlow::Continue(())
            });
            assert_eq!(got.len(), 2);
            for (a, b) in got {
                assert_eq!(a, b, "{t:?}");
            }
        }
        for t in ["$3", "$x", "a${1}${w2}"] {
            assert_eq!(
                r.template(t).unwrap_err(),
                r.expand(&Captures { groups: Vec::new() }, "", t)
                    .unwrap_err()
            );
        }
        assert_eq!(Template::verbatim("$1").expand_bytes("", &[]), "$1");
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
