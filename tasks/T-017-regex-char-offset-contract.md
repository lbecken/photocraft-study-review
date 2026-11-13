---
id: T-017
title: "Compile and run regular expressions in kiln-search with char-offset matches, line-aware anchors, pinned empty-match iteration and replacement expansion"
tier: high              # ADR-0002: dependency-table change (kiln-search gains `regex`)
state: draft
owner_role: planner
scorecard_items: []     # library contract; search.* items are claimed by T-021 through commands
depends_on: []
touches:
  - crates/kiln-search/Cargo.toml        # + regex (exact pin, same version as Cargo.lock)
  - crates/kiln-search/src               # lib.rs, regex.rs
  - crates/kiln-search/tests/regex.rs    # implementer's own integration tests
  - Cargo.lock                           # generated (no version change expected)
budget_tokens: 850000   # high (v1.1)
iteration: 1
found_by: planner
origin_task: null
---

# T-017 Regular expressions with char offsets in kiln-search

## Goal
`kiln-search` is empty. After this task it offers a compiled regular expression whose matches
and capture groups are char offsets (the protocol's unit). Line anchors follow Kiln's line
model (`\n`, `\r\n`, lone `\r`). Iteration over matches has pinned, documented semantics, and
replacement templates expand capture groups. The dependency decision and its exact semantics are
reviewed here, so that T-021 (in-buffer search and replace) and T-022 (find in files) can be
medium.

No gate path. Reading list: CHARTER §3 decision 5; ADR-0002; `crates/kiln-search/src/lib.rs`;
`docs/control-protocol.md` (char offsets).

## Interface
`crates/kiln-search/Cargo.toml`: `regex = "=1.13.1"`. That is the version already in `Cargo.lock`
through `jsonschema`, so no package changes version. It is declared in the crate manifest
(iteration Question Q-1).

```rust
pub mod regex;     // this task
pub use regex::{CharMatch, Captures, PatternError, Regex, RegexOptions};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RegexOptions { pub case_insensitive: bool }

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CharMatch { pub start: usize, pub end: usize }      // chars, start <= end

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Captures { pub groups: Vec<Option<CharMatch>> }    // [0] = whole match; len = group_count() + 1

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternError { pub message: String }               // Display = message

impl Regex {
    pub fn new(pattern: &str, options: RegexOptions) -> Result<Regex, PatternError>;
    pub fn group_count(&self) -> usize;                         // explicit groups
    /// All matches, in the iteration order defined below.
    pub fn find_all(&self, text: &str) -> Vec<Captures>;
    /// `replacement` with `$` references to `caps` (taken from `text`) expanded.
    pub fn expand(&self, caps: &Captures, text: &str, replacement: &str) -> String;
}
```

### Semantics (the specification)
- Syntax and matching are those of the `regex` crate 1.13 with Unicode enabled, multi-line mode
  and CRLF mode on. So `^` matches at the start of the text and after every `\n`, every `\r\n`
  and every lone `\r`. `$` matches at the end and before each of these, but never between the
  `\r` and `\n` of a CRLF. `.` matches any char except `\r` and `\n`.
- `case_insensitive: true` = Unicode simple case folding. So `σ` matches `Σ` and `ς`, `ſ`
  matches `s` and `S`, and `k` matches `K` (U+212A Kelvin sign); VERIFIED in a spike.
- Iteration (leftmost-first, non-overlapping): the search starts at char 0. Each match is the
  leftmost one, with the regex crate's leftmost-first preference among alternatives. After a
  match ending at `e` the search resumes at `e`. An **empty** match at the position where the
  previous match ended is not reported. After an empty match the search resumes one char
  later. Empty matches can occur between the `\r` and `\n` of a CRLF.
- Expansion: `$n` or `${n}` (decimal) and `$name` or `${name}` (`name` is the longest run of
  `[0-9A-Za-z_]`) are replaced by that group's text, or by nothing if the group did not take
  part or does not exist. `$$` gives `$`. Any other `$` stays a literal `$`. Nothing else is
  special.
- Errors: a pattern that does not compile, or that exceeds the regex crate's default size
  limits, gives `PatternError` with the crate's message, at most 200 chars. Messages are not
  stable. The empty pattern is valid here: it matches at every char position. T-021 decides
  what commands accept.
- No panic on any pattern or text.

## Pinned
- (a) One error type, `PatternError`. Precedence n/a.
- (b) Files: none.
- (c) Order: n/a.
- (d) Offsets: chars; `CharMatch` never splits a char. Platform independent.
- (e) Threat model (dependency): the regex crate guarantees linear-time matching. The reviewer
  probes pathological patterns (`(a*)*b` on 100,000 `a`s, `\p{L}{1000}`, deeply nested groups,
  a 100 kB pattern). Each must answer or error within 5 s in a debug build, and must not panic.
- (f) Snapshots: none.
- (g) Worked examples (executed with `regex` 1.13.1 in a scratch spike; ASCII, so bytes = chars):
  - `a*` on `baaab` → matches `0..0`, `1..4`, `5..5`.
  - `^.|.$` on `ab\rcd\r\nef\ngh` → starts 0,1,3,4,7,8,10,11 (each line's first and last char,
    for lines split at `\r`, `\r\n`, `\n`).
  - `.+` on the same text → `0..2`, `3..5`, `7..9`, `10..12`.
  - `(?<w>a)(b)?` on `ac`, expanding `[$1|$2|${w}|$w|$$|$|$9|$x!]` → `[a||a|a|$|$||!]`.
  - case-insensitive `σ|ſ|k` on `ΣςSKK` (last = U+212A) → five matches, one per char.
  - `x*` on `é\r\n` → empty matches at chars 0, 1, 2, 3.
- (h) Scorecard: none.

## Acceptance criteria
- AC-1 (differential against a naive model): the test author writes a naive backtracking
  matcher for this subset: literal chars, `.`, `[…]` and `[^…]` with ranges, `*` `+` `?` and
  their lazy forms, `|`, `(…)`, `(?:…)`, `^`, `$`. It implements the iteration and anchor rules
  above. For ≥ 20,000 fixed-seed random (pattern, text) pairs over the alphabet `a b A _ 0 é σ Σ
  ς ſ s S K k \r \n` plus ` ` and `😀`, `find_all` equals the model, groups included. This is
  run once with `case_insensitive` false and once with true. In case-insensitive mode the model
  compares chars by "equal, or equal `to_lowercase()`, or equal `to_uppercase()`". That rule
  agrees with simple case folding on this alphabet. Oracle: naive model.
- AC-2 (expansion): for random templates over `$ { } 0 1 9 a w _ x !` and random captures,
  `expand` equals a naive implementation of the expansion rule. Oracle: naive model.
- AC-3 (offsets are chars): for texts with `é`, `中`, `😀` and CRLF, every `CharMatch` equals the
  naive model's char offsets, and `text.chars().skip(start).take(end-start)` equals the matched
  text. Oracle: naive model.
- AC-4 (errors): `(`, `[a-`, `a{99999999}`, `\p{Nope}` give `Err(PatternError)` with a
  non-empty message of at most 200 chars; nothing panics. Oracle: exact property.
- AC-5 (worked examples): each example in Pinned (g) holds exactly. Oracle: exact value.

## Scorecard items and intended test names
None. `crates/kiln-search/tests/acceptance_t017.rs`: `matches_equal_naive_backtracker`,
`case_insensitive_matches_equal_naive_model`, `expansion_equals_naive_model`,
`offsets_are_chars`, `invalid_patterns_are_errors`, `worked_examples_hold`.

## Out of scope and implementer notes
Literal search, whole-word, replace on a buffer, commands (T-021); multi-file search (T-022).
Do not add `fancy-regex` or any other engine. Your tests: `src/regex.rs` unit tests and
`crates/kiln-search/tests/regex.rs`.

## Questions and rulings

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13 · R-056 · (new) · did: specified the regex contract. Tier high, by ADR-0002
  "dependency-table change", even though `regex` 1.13.1 is already locked. Every worked
  example was executed against regex 1.13.1 in a scratch crate. The CRLF-mode behaviour (lone
  `\r` is a terminator too) matches kiln-text's line model, so I made it the rule. The oracle is
  a naive backtracker over a pinned subset, written by the test author. Pinned (a)–(c), (f), (h)
  n/a. · gate attempts: 0 · missing context: none · next: director.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
