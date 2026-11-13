//! # kiln-search — layer L1
//!
//! Responsibility (CHARTER §3): literal and regex search and replace over a buffer.
//!
//! May depend on: `kiln-text` (L0). Must not depend on `kiln-syntax` (same layer). Headless.
//! External dependency: the `regex` crate (T-017), the engine behind [`Regex`].
//!
//! Contents:
//! - [`regex`] (T-017): compiled regular expressions whose matches and capture groups are char
//!   offsets, with line anchors in Kiln's line model (`\n`, `\r\n`, lone `\r`), pinned
//!   empty-match iteration and `$` replacement expansion.
//! - [`query`] (T-021): literal and regex queries, case-sensitive or not, optionally
//!   whole-word, with bounded search ([`CompiledQuery::find`]) and replacement planning
//!   ([`CompiledQuery::replace_first`], [`CompiledQuery::replace_all`]) on a `&str`.

pub mod query;
pub mod regex;

pub use query::{CompiledQuery, Found, MAX_PATTERN_CHARS, Query, QueryError};
pub use regex::{Captures, CharMatch, PatternError, Regex, RegexOptions, TemplateError};
