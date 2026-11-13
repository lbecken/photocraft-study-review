---
id: T-017
title: "Compile and run regular expressions in kiln-search with char-offset matches, line-aware anchors, pinned empty-match iteration and replacement expansion"
tier: high              # ADR-0002: dependency-table change (kiln-search gains `regex`)
state: tests_joined
owner_role: planner
scorecard_items: []     # library contract; search.* items are claimed by T-021 through commands
depends_on: []
touches:
  - crates/kiln-search/Cargo.toml        # + regex (exact pin, same version as Cargo.lock)
  - crates/kiln-search/src               # lib.rs, regex.rs
  - crates/kiln-search/tests/regex.rs    # implementer's own integration tests
  - Cargo.lock                           # generated; the queue regenerates it, so overlap with another in-flight task on this path is permitted
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

No gate path, so the approval row does not need `gate paths`. It does not need
`dependency versions` either: the change only adds a dependency edge and changes no locked
version (R-067 verified: no package added, removed or re-versioned). The task is tier high
because it adds a dependency (ADR-0002).

Reading list: CHARTER §3 decision 5; ADR-0002; `crates/kiln-search/src/lib.rs`;
`docs/control-protocol.md` (char offsets).

## Interface
`crates/kiln-search/Cargo.toml`: `regex = "=1.13.1"` with default features (no
`default-features = false`; case folding and `\w` need `unicode`). That is the version already
in `Cargo.lock` through `jsonschema`, so no package changes version. It is declared in the
crate manifest (director, R-060 answer 1). The exact pin also holds `regex` at 1.13.1 for the
whole workspace (`jsonschema`, `tree-sitter`); a later update edits this manifest. The manifest
comment says so.

```rust
pub mod regex;     // this task
pub use regex::{CharMatch, Captures, PatternError, Regex, RegexOptions, TemplateError};

#[derive(Clone, Debug)]
pub struct Regex { /* private: the engine, the group names */ }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RegexOptions { pub case_insensitive: bool }

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CharMatch { pub start: usize, pub end: usize }      // chars, start <= end

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Captures { pub groups: Vec<Option<CharMatch>> }    // [0] = whole match; len = group_count() + 1

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternError { pub message: String }               // Display = message

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemplateError { pub reference: String }            // see Expansion

impl Regex {
    pub fn new(pattern: &str, options: RegexOptions) -> Result<Regex, PatternError>;
    pub fn group_count(&self) -> usize;                         // explicit groups
    /// All matches, in the iteration order defined below.
    pub fn find_all(&self, text: &str) -> Vec<Captures>;
    /// `replacement` with `$` references to `caps` (taken from `text`) expanded.
    pub fn expand(&self, caps: &Captures, text: &str, replacement: &str)
        -> Result<String, TemplateError>;
}
```
`PatternError` and `TemplateError` implement `Display` and `std::error::Error`.

### Semantics (the specification)
- Syntax and matching are those of the `regex` crate 1.13 with Unicode enabled, multi-line mode
  and CRLF mode on. So `^` matches at the start of the text and after every `\n`, every `\r\n`
  and every lone `\r`. `$` matches at the end and before each of these, but never between the
  `\r` and `\n` of a CRLF. `.` matches any char except `\r` and `\n`.
- Inline flags (`(?i)`, `(?-i)`, `(?m)`, `(?-m)`, `(?s)`, `(?R)`, `(?-R)`, `(?x)`, `(?U)`,
  `(?-u)` and their combinations) are accepted and override the options and the modes above
  for their scope, as in the crate; e.g. `(?s)` makes `.` match `\r` and `\n`. T-021 documents
  this for users in `docs/protocol/search.md`.
- `case_insensitive: true` = Unicode simple case folding. So `σ` matches `Σ` and `ς`, `ſ`
  matches `s` and `S`, and `k` matches `K` (U+212A Kelvin sign); VERIFIED in a spike. A class
  is folded before it is negated: case-insensitively `[…]` matches `c` iff some member of the
  class matches `c` case-insensitively, and `[^…]` matches iff no member does (so `(?i)[^a]`
  excludes `A`).
- Iteration (leftmost-first, non-overlapping): the search starts at char 0. Each match is the
  leftmost one, with the regex crate's leftmost-first preference among alternatives. After a
  match ending at `e` the search resumes at `e`. If the leftmost-first match found from `e` is
  empty and starts at `e`, the previous match's end, it is discarded and the search restarts at
  `e+1`; no other match starting at `e` is tried. Anchors and `\b` still see the whole text.
  After a reported empty match at `s`, the search resumes at `s+1`. Empty matches can occur
  between the `\r` and `\n` of a CRLF.
- Captures in repetitions: a group inside a repetition holds its range from the last iteration
  in which it took part. It is not reset when an enclosing repetition iterates again.
- Quantified sub-patterns that can match the empty string (`(a*)*`, `(|a)+`, `(^)*`, …): the
  results are the engine's, whose NFA drops a path that re-enters a state at the same
  position. They differ from a textbook backtracker's and are not modelled; the worked
  examples in Pinned (g) document them.
- Expansion (Kiln's own rule; the implementer cannot call `regex::Captures::expand`, because
  `kiln_search::Captures` holds char offsets). `replacement` is scanned left to right:
  - `$$` → `$`.
  - `${` up to the next `}` → a reference whose name is everything between them, any chars,
    possibly empty. With no `}` later in the template, the `$` is literal.
  - Otherwise `$` followed by the longest non-empty run of `[0-9A-Za-z_]` → a reference with
    that name. With an empty run, the `$` is literal.
  - A reference whose name is all ASCII digits and parses as `usize` is a group number;
    otherwise it is a group name.
  - A reference to a group that exists in the pattern (a number ≤ `group_count()`, or the name
    of a named group) expands to that group's text, or to nothing if the group did not take
    part. A group index ≥ `caps.groups.len()`, or a range with `start > end` or
    `end > text.chars().count()`, also expands to nothing.
  - A reference to a group that does **not** exist is an error: `expand` returns
    `Err(TemplateError { reference })` for the first such reference from the left, where
    `reference` is the name exactly as written. `Display` is `no capture group "` + reference
    + `"`. Whether a template is an error depends only on the pattern and `replacement`, never
    on `caps` or `text`.
  - Every other char is copied. `expand` never panics.
  Where this differs from regex 1.13.1's `Captures::expand`: the engine expands a reference to
  a group that does not exist to nothing — e.g. `$9` with two groups, `${}`, `$1a`, `$12x`,
  `${a!}`, `${1 }`, `$99999999999999999999` — and here each is an error. The scanning (what
  counts as a reference and what stays literal) is the same.
- Errors: a pattern that does not compile, that exceeds the regex crate's default size limits,
  or that nests groups deeper than the crate's limit (250) gives `PatternError`. `message` is
  the engine's `Display`; if it is longer than 200 chars, it is cut to its first 199 chars
  followed by `…`, never splitting a char (as the control protocol cuts details,
  docs/control-protocol.md:67–70). Messages are not stable. The empty pattern is valid here: it
  matches at every char position. T-021 decides what commands accept.
- No panic on any pattern, text, captures or template.

## Pinned
- (a) Two error types: `PatternError` from `new`, `TemplateError` from `expand`. Precedence:
  n/a between them (different calls); within one template, the first non-existent reference
  from the left.
- (b) Files: none.
- (c) Order: n/a.
- (d) Offsets: chars; `CharMatch` never splits a char. Platform independent.
- (e) Threat model (dependency): the regex crate guarantees linear-time matching. The reviewer
  probes pathological patterns (`(a*)*b` on 100,000 `a`s, `\p{L}{1000}`, deeply nested groups,
  a 100 kB pattern). Each must answer or error within 5 s in a debug build, and must not panic.
- (f) Snapshots: none.
- (g) Worked examples (executed with `regex` 1.13.1 in a scratch spike; offsets are chars):
  - `a*` on `baaab` → matches `0..0`, `1..4`, `5..5`.
  - `^.|.$` on `ab\rcd\r\nef\ngh` → starts 0,1,3,4,7,8,10,11 (each line's first and last char,
    for lines split at `\r`, `\r\n`, `\n`).
  - `.+` on the same text → `0..2`, `3..5`, `7..9`, `10..12`.
  - case-insensitive `σ|ſ|k` on `ΣςSKK` (last = U+212A) → five matches, one per char.
  - `x*` on `é\r\n` → empty matches at chars 0, 1, 2, 3.
  - `b|(?:)|a` on `ba` → `0..1`, `2..2` (the empty match at 1 is discarded and the search
    restarts at 2, so `a` at `1..2` is never reported).
  - Captures in repetitions (whole match, then groups): `(?:(a)|b)*` on `ab` → `[0..2, 0..1]`;
    `((a)|b)+` on `ab` → `[0..2, 1..2, 0..1]`; `(?:(a)|(b))+` on `ab` → `[0..2, 0..1, 1..2]`.
  - Expansion, with `(?<w>a)(b)?` matched on `ac` (`group_count() == 2`, group 2 did not take
    part): `[$1|$2|${w}|$w|$0|$$|$|$!]` → `Ok("[a||a|a|a|$|$|$!]")` (the engine gives the
    same); `${ab` → `Ok("${ab")`. Each of these gives `Err(TemplateError { reference })` with
    the reference shown, where the engine gives the empty string: `$9` → `"9"`, `$x!` → `"x"`,
    `${}` → `""`, `$1a` → `"1a"`, `${a!}` → `"a!"`, `${1 }` → `"1 "`,
    `$99999999999999999999` → `"99999999999999999999"`, `$w$9` → `"9"`.
  - Quantified sub-patterns that can match empty (the class AC-1 does not generate;
    hand-written expectations, AC-6). Full `find_all` result, each match as `[whole, group 1]`:
    | Pattern | Text | `find_all` |
    |---|---|---|
    | `(?:a\|())*` | `aa` | `[0..2, None]` |
    | `(\|a)+` | `aa` | `[0..0, 0..0]`, `[1..1, 1..1]`, `[2..2, 2..2]` |
    | `( *.??)+` | ` k0σb0ks` | `[0..1, 0..1]`, then `[s..s, s..s]` for s = 2, 3, …, 8 |
    | `(^)*` | `éS` | `[0..0, 0..0]`, `[1..1, None]`, `[2..2, None]` |
    | `(a*)*` | `b` | `[0..0, 0..0]`, `[1..1, 1..1]` |

  The examples added in R-077 (from `b|(?:)|a` on) were executed in R-077 with `regex` 1.13.1 (`multi_line`, `crlf`) and
  byte→char conversion; the expansion results against a naive model of the rule in Interface
  (the `Ok` cases also against `regex::Captures::expand`).
- (h) Scorecard: none.

## Acceptance criteria
- AC-1 (differential against a naive model): the test author writes a naive backtracking
  matcher for this subset: literal chars, `.`, `[…]` and `[^…]` with ranges, `*` `+` `?` and
  their lazy forms, `|`, `(…)`, `(?:…)`, `^`, `$`. It implements the iteration, anchor and
  capture rules above. For ≥ 20,000 fixed-seed random (pattern, text) pairs over the alphabet
  `a b A _ 0 é σ Σ ς ſ s S`, `K` (U+004B), `k`, `K` (U+212A), `\r \n` plus ` ` and `😀`,
  `find_all` equals the model, groups included. This is run once with `case_insensitive` false
  and once with true. In case-insensitive mode the model compares chars by "equal, or equal
  `to_lowercase()`, or equal `to_uppercase()`". That rule agrees with simple case folding on
  this alphabet (R-067: against every Unicode scalar value). Case-insensitively, `[…]` matches
  `c` iff some member `m` of the class satisfies the predicate with `c`, and `[^…]` matches iff
  no member does.
  The generator never applies `*`, `+`, `?` or a lazy form to a sub-pattern that can match the
  empty string: `^`, `$`, an empty group or branch, a group or alternation with a branch that
  can be empty, a concatenation whose parts can all be empty, or anything already quantified
  with `*` or `?`. Why: on that class the engine's results come from its NFA (a path that
  re-enters a state at the same position dies; `e*` with a nullable `e` behaves as
  `(?:e+)?`), and a naive backtracker legitimately differs (R-067: 91 differences in 20,000
  pairs with such operands, 0 without). That class is covered by AC-6 instead. Oracle: naive
  model.
- AC-2 (expansion): for random templates over `$ { } 0 1 9 a w _ x !` and random captures
  (including captures with fewer or more groups than `group_count() + 1`, `start > end`, and
  ranges past the end of `text`), `expand` equals a naive implementation of the expansion rule,
  `Ok` text and `Err` reference alike, and never panics. Oracle: naive model.
- AC-3 (offsets are chars): for texts with `é`, `中`, `😀` and CRLF, every `CharMatch` equals the
  naive model's char offsets, and `text.chars().skip(start).take(end-start)` equals the matched
  text. Oracle: naive model.
- AC-4 (errors): `(`, `[a-`, `a{99999999}`, `\p{Nope}` and 300 nested groups give
  `Err(PatternError)` with a non-empty message of at most 200 chars; an invalid pattern whose
  engine message exceeds 200 chars (e.g. `(` followed by 300 `a`s) gives exactly its first 199
  chars followed by `…`; nothing panics. Oracle: exact property.
- AC-5 (worked examples): each example in Pinned (g) holds exactly. Oracle: exact value.
- AC-6 (quantified sub-patterns that can match empty): each row of the table in Pinned (g)
  holds exactly, both case modes. These are the hand-written expectations for the class AC-1's
  generator excludes. Oracle: exact value (the engine's results, R-067 and R-077).

## Scorecard items and intended test names
None. `crates/kiln-search/tests/acceptance_t017.rs`: `matches_equal_naive_backtracker`,
`case_insensitive_matches_equal_naive_model`, `expansion_equals_naive_model`,
`offsets_are_chars`, `invalid_patterns_are_errors`, `worked_examples_hold`,
`nullable_quantifier_examples_hold`.

## Out of scope and implementer notes
Literal search, whole-word, replace on a buffer, commands (T-021); multi-file search (T-022).
Do not add `fancy-regex` or any other engine. Your tests: `src/regex.rs` unit tests and
`crates/kiln-search/tests/regex.rs`.

## Questions and rulings
Director rulings on the test author's questions (2026-11-13, decision D-060; run R-105), given to both builders at once:
- s1 (memory). Pinned (e) says a pathological pattern must "answer or error" — an aborted process is neither. The test author found that `(a)` repeated 34,000 times compiles and then makes `find_all` on a matching text try to allocate about 55 GB (capture slots × engine states). `Regex::new` therefore refuses a pattern with more than 100 explicit capture groups: `Err(PatternError { message })` with `message` exactly `the pattern has more than 100 capture groups`. The check runs after the crate has accepted the pattern (a pattern the crate rejects keeps the crate's error); exactly 100 groups is accepted. Acceptance tests: 100 groups accepted and usable, 101 refused with that message, the 34,000-group pattern refused within 5 s. The reviewer measures peak memory of `find_all` for adversarial patterns under the cap (group-heavy and state-heavy) and reports it; above about 1 GB is a finding.
- s2 (error messages). A `PatternError` message for a pattern the crate rejects is the crate's message for the pattern text exactly as the caller gave it, built with builder options (multi-line, CRLF, case-insensitive) and no inline-flag prefix added to the pattern, then cut as Pinned says.

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
- 2026-11-13 · R-060 · draft · did: dependency placement confirmed (crate manifest). No other change. · gate attempts: 0 · missing context: none · next: director.
- 2026-11-13 · R-077 · draft · did: applied spec check R-067 (`tasks/spec-checks/T-017.md`)
  with the director's rulings. Blocker 2 (AC-1 generator excludes quantified nullable
  sub-patterns, with the reason; new AC-6 with hand-written expectations for that class; the
  capture-in-repetition rule). Ambiguities 3 (restart at `e+1`), 4 (expansion: Kiln's own
  scanning rule, same as the engine's, but a reference to a non-existent group is
  `Err(TemplateError)`; `expand` now returns `Result`; differences listed), 5, 6 (truncation),
  7 (inline flags accepted), 8 (`Regex: Clone + Debug`; `K` U+212A in the alphabet), 9. Notes
  applied as corrections: 12, 13 (default features), 15 (approval wording), 18 (`Cargo.lock`
  kept, overlap permitted by director), 19 (nest limit → `PatternError`). Notes 1, 10, 11, 16,
  17, 20 are observations; 14 is an edit to T-021, outside this file. Executed in a scratch
  spike with `regex` 1.13.1: `b|(?:)|a`, the capture-in-repetition examples, the nullable table
  in both case modes, the `Ok` expansion examples against `Captures::expand`, the 300-group and
  long-message errors (1,050 and 352 chars); the `Err` expansions against a naive model of the
  rule. Tier high (ADR-0002, adds a dependency). Pinned (b), (c), (f), (h) n/a. · gate
  attempts: 0 · missing context: none · next: director.
  QUESTION: the R-077 brief also asks to pin "which offset type parameter is public"; the
  spec check has no such finding and the interface has no type parameter (offsets are
  `usize` chars). What was meant?
  QUESTION: T-021 needs follow-up edits: `expand` now returns `Result<String, TemplateError>`
  (T-021 must map it, e.g. a refusal for a replacement naming a non-existent group); R-067
  note 14 (T-021 AC-2 inherits the generator restriction); and T-021 documents inline flags in
  `docs/protocol/search.md`.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
