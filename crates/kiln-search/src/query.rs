//! Search queries: literal or regex, case-sensitive or not, optionally whole-word (T-021).
//!
//! A [`Query`] compiles to a [`CompiledQuery`], which searches a `&str` and plans replacements.
//! Offsets are chars, as in [`crate::regex`].
//!
//! # Matching
//! - Literal mode: the occurrences of the pattern as a char sequence, left to right,
//!   non-overlapping (after a match the scan resumes at its end). Without case sensitivity,
//!   chars compare by Unicode simple case folding, as the case-insensitive mode of [`Regex`]
//!   does. A `\r\n` in the pattern matches only `\r\n`; a `\n` also matches the `\n` of a
//!   `\r\n`. (Built on [`Regex`] with the pattern escaped, so both modes share one iteration.)
//! - Regex mode: [`Regex`] with `case_insensitive = !case_sensitive` and its iteration; inline
//!   flags in the pattern override the options for their scope.
//! - Whole word (both modes): the matches of the search without whole-word, keeping those
//!   with a word boundary at both ends. A word char is `_` or an alphanumeric char. Nothing is
//!   re-scanned after a rejected match.
//!
//! The matches of one search are sorted by start, pairwise distinct in start and
//! non-overlapping.
//!
//! # Limits
//! A pattern has at most [`MAX_PATTERN_CHARS`] chars (both modes) and, in regex mode, at most
//! 100 capture groups. These bound the pattern text and the capture slots, not the time a
//! search takes. [`CompiledQuery::find`] holds at most `limit` ranges and one match's slots;
//! [`CompiledQuery::replace_all`] holds one range and one replacement text per match, never the
//! slots of more than the current match, and expands each match without walking the text.

use std::fmt;
use std::ops::ControlFlow;

use crate::regex::{
    Captures, CharMatch, PatternError, Regex, RegexOptions, Template, TemplateError,
};

/// Longest pattern, in chars, in both modes (D-071 c3).
pub const MAX_PATTERN_CHARS: usize = 1000;

/// A search query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Query {
    /// The literal text or the regular expression.
    pub pattern: String,
    /// Regex mode (else literal).
    pub regex: bool,
    /// Compare chars exactly (else by Unicode simple case folding).
    pub case_sensitive: bool,
    /// Keep only matches with a word boundary at both ends.
    pub whole_word: bool,
}

/// Why a [`Query`] does not compile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueryError {
    /// The pattern is empty (both modes).
    Empty,
    /// The pattern has more than [`MAX_PATTERN_CHARS`] chars (both modes).
    TooLong {
        /// The pattern's length in chars.
        chars: usize,
    },
    /// Regex mode: [`Regex::new`] refused the pattern.
    Pattern(PatternError),
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryError::Empty => f.write_str("empty search pattern"),
            QueryError::TooLong { chars } => write!(
                f,
                "invalid pattern: the pattern has {chars} chars; the limit is {MAX_PATTERN_CHARS}"
            ),
            QueryError::Pattern(e) => write!(f, "invalid pattern: {e}"),
        }
    }
}

impl std::error::Error for QueryError {}

/// The result of [`CompiledQuery::find`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    /// The whole-match ranges of the first `limit` matches.
    pub matches: Vec<CharMatch>,
    /// The exact number of all matches.
    pub total: usize,
}

/// A compiled [`Query`].
#[derive(Clone, Debug)]
pub struct CompiledQuery {
    /// The engine (literal mode: the escaped pattern).
    regex: Regex,
    /// Regex mode.
    is_regex: bool,
    /// Whole-word filter.
    whole_word: bool,
}

fn is_word_char(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

/// True iff the match at bytes `[s, e)` of `text` has a word boundary at both ends.
fn has_word_boundaries(text: &str, s: usize, e: usize) -> bool {
    let before = text.get(..s).and_then(|t| t.chars().next_back());
    let after = text.get(e..).and_then(|t| t.chars().next());
    !before.is_some_and(is_word_char) && !after.is_some_and(is_word_char)
}

impl Query {
    /// Compiles the query.
    ///
    /// # Errors
    /// In this order: [`QueryError::Empty`]; [`QueryError::TooLong`] (pattern longer than
    /// [`MAX_PATTERN_CHARS`] chars, both modes, checked before compiling);
    /// [`QueryError::Pattern`] (regex mode: [`Regex::new`] with
    /// `RegexOptions { case_insensitive: !case_sensitive }`).
    pub fn compile(&self) -> Result<CompiledQuery, QueryError> {
        if self.pattern.is_empty() {
            return Err(QueryError::Empty);
        }
        let chars = self.pattern.chars().count();
        if chars > MAX_PATTERN_CHARS {
            return Err(QueryError::TooLong { chars });
        }
        let options = RegexOptions {
            case_insensitive: !self.case_sensitive,
        };
        let regex = if self.regex {
            Regex::new(&self.pattern, options)
        } else {
            Regex::new(&::regex::escape(&self.pattern), options)
        }
        .map_err(QueryError::Pattern)?;
        Ok(CompiledQuery {
            regex,
            is_regex: self.regex,
            whole_word: self.whole_word,
        })
    }
}

impl CompiledQuery {
    /// Calls `f` for each match (whole-word filter applied), in order, with its captures and
    /// group byte ranges; stops after a call that returns `Break`.
    fn each(
        &self,
        text: &str,
        f: &mut dyn FnMut(&Captures, &[Option<(usize, usize)>]) -> ControlFlow<()>,
    ) {
        self.regex.for_each_match_bytes(text, &mut |caps, bytes| {
            if self.whole_word
                && let Some(Some((s, e))) = bytes.first()
                && !has_word_boundaries(text, *s, *e)
            {
                return ControlFlow::Continue(());
            }
            f(caps, bytes)
        });
    }

    /// The template for `replacement` (literal mode: verbatim).
    fn template(&self, replacement: &str) -> Result<Template, TemplateError> {
        if self.is_regex {
            self.regex.template(replacement)
        } else {
            Ok(Template::verbatim(replacement))
        }
    }

    /// The whole-match ranges of the first `limit` matches (whole-word filter applied) and
    /// `total`, the exact number of all matches. `limit` may be 0 (count only).
    /// Memory: O(limit + group_count()). Time: one pass over the text.
    pub fn find(&self, text: &str, limit: usize) -> Found {
        let mut found = Found {
            matches: Vec::new(),
            total: 0,
        };
        self.each(text, &mut |caps, _| {
            if found.total < limit
                && let Some(Some(m)) = caps.groups.first()
            {
                found.matches.push(*m);
            }
            found.total += 1;
            ControlFlow::Continue(())
        });
        found
    }

    /// `Ok(())` iff `replacement` expands for this pattern (literal mode: always). Depends on
    /// the pattern and the template only.
    ///
    /// # Errors
    /// Regex mode: the [`TemplateError`] of the first reference, from the left, to a group the
    /// pattern does not have.
    pub fn check_template(&self, replacement: &str) -> Result<(), TemplateError> {
        self.template(replacement).map(|_| ())
    }

    /// The replacement text for one match: literal mode → `Ok(replacement)`; regex mode →
    /// [`Regex::expand`], which walks `text` from its start per reference, so it is for
    /// single matches only.
    ///
    /// # Errors
    /// As [`Self::check_template`].
    pub fn replacement_for(
        &self,
        caps: &Captures,
        text: &str,
        replacement: &str,
    ) -> Result<String, TemplateError> {
        if self.is_regex {
            self.regex.expand(caps, text, replacement)
        } else {
            Ok(replacement.to_owned())
        }
    }

    /// The first match (iteration from char 0) whose start ≥ `from`, with its replacement
    /// text. Memory: O(group_count() + replacement text). Stops at that match.
    ///
    /// # Errors
    /// As [`Self::check_template`], checked before matching.
    pub fn replace_first(
        &self,
        text: &str,
        from: usize,
        replacement: &str,
    ) -> Result<Option<(CharMatch, String)>, TemplateError> {
        let template = self.template(replacement)?;
        let mut first = None;
        self.each(text, &mut |caps, bytes| match caps.groups.first() {
            Some(Some(m)) if m.start >= from => {
                first = Some((*m, template.expand_bytes(text, bytes)));
                ControlFlow::Break(())
            }
            _ => ControlFlow::Continue(()),
        });
        Ok(first)
    }

    /// Every match's whole range with its replacement text, in order.
    /// Memory: O(matches + total replacement text), never O(matches × groups).
    /// Time: O(len(text) + Σ over matches of (match length × references in `replacement`))
    /// on top of the engine's search, never O(len(text) × matches). No bound on the number
    /// of matches.
    ///
    /// # Errors
    /// As [`Self::check_template`], checked before matching.
    pub fn replace_all(
        &self,
        text: &str,
        replacement: &str,
    ) -> Result<Vec<(CharMatch, String)>, TemplateError> {
        let template = self.template(replacement)?;
        let mut out = Vec::new();
        self.each(text, &mut |caps, bytes| {
            if let Some(Some(m)) = caps.groups.first() {
                out.push((*m, template.expand_bytes(text, bytes)));
            }
            ControlFlow::Continue(())
        });
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(pattern: &str, regex: bool, case_sensitive: bool, whole_word: bool) -> CompiledQuery {
        Query {
            pattern: pattern.to_owned(),
            regex,
            case_sensitive,
            whole_word,
        }
        .compile()
        .unwrap()
    }

    fn ranges(f: &Found) -> Vec<(usize, usize)> {
        f.matches.iter().map(|m| (m.start, m.end)).collect()
    }

    #[test]
    fn worked_examples() {
        let t = "foo FOO fOo_x éFoo";
        let f = q("Foo", false, false, false).find(t, 100);
        assert_eq!(ranges(&f), [(0, 3), (4, 7), (8, 11), (15, 18)]);
        let f = q("Foo", false, false, true).find(t, 100);
        assert_eq!(ranges(&f), [(0, 3), (4, 7)]);
        assert_eq!(ranges(&q("aa", false, true, true).find("aaa b aa", 9)), [(6, 8)]);
        let f = q("a*", true, true, false).find("baaab", 9);
        assert_eq!(ranges(&f), [(0, 0), (1, 4), (5, 5)]);
    }

    #[test]
    fn limit_keeps_the_first_and_counts_all() {
        let f = q("x", false, true, false).find("xxxxx", 2);
        assert_eq!((ranges(&f), f.total), (vec![(0, 1), (1, 2)], 5));
        let f = q("x", false, true, false).find("xxxxx", 0);
        assert_eq!((f.matches.len(), f.total), (0, 5));
    }

    #[test]
    fn literal_mode_escapes_and_folds() {
        assert_eq!(ranges(&q("a*", false, true, false).find("aa*", 9)), [(1, 3)]);
        assert_eq!(q("σ", false, false, false).find("ΣςσS", 9).total, 3);
        assert_eq!(q("\n", false, true, false).find("a\r\nb\n", 9).total, 2);
        assert_eq!(q("\r\n", false, true, false).find("\r\r\n\n", 9).total, 1);
        // `(?i)` is literal text in literal mode.
        assert_eq!(q("(?i)a", false, true, false).find("A", 9).total, 0);
    }

    #[test]
    fn errors_in_order() {
        let e = |p: &str, regex| {
            Query {
                pattern: p.to_owned(),
                regex,
                case_sensitive: true,
                whole_word: false,
            }
            .compile()
            .unwrap_err()
        };
        assert_eq!(e("", true), QueryError::Empty);
        assert_eq!(e(&"(".repeat(1001), true), QueryError::TooLong { chars: 1001 });
        assert_eq!(e(&"é".repeat(1001), false), QueryError::TooLong { chars: 1001 });
        assert!(matches!(e("(", true), QueryError::Pattern(_)));
        assert_eq!(
            e(&"(a)".repeat(101), true).to_string(),
            "invalid pattern: the pattern has more than 100 capture groups"
        );
        assert_eq!(
            e(&"a".repeat(1001), false).to_string(),
            "invalid pattern: the pattern has 1001 chars; the limit is 1000"
        );
        assert_eq!(e("", false).to_string(), "empty search pattern");
    }

    #[test]
    fn replacements() {
        let c = q("(\\w)(\\d)", true, true, false);
        let all = c.replace_all("a1 b2", "$2$1").unwrap();
        let texts: Vec<&str> = all.iter().map(|(_, s)| s.as_str()).collect();
        assert_eq!(texts, ["1a", "2b"]);
        assert_eq!(
            c.replace_first("a1 b2", 1, "$0-").unwrap(),
            Some((CharMatch { start: 3, end: 5 }, "b2-".to_owned()))
        );
        assert_eq!(c.replace_first("a1 b2", 4, "$0").unwrap(), None);
        assert_eq!(c.check_template("$3").unwrap_err().to_string(), "no capture group \"3\"");
        assert!(c.replace_all("", "$x").is_err());
        let lit = q("b", false, true, false);
        assert!(lit.check_template("$3").is_ok());
        assert_eq!(lit.replace_all("ab", "$1").unwrap()[0].1, "$1");
    }
}
