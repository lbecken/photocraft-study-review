---
id: T-017
title: "Compile and run regular expressions in kiln-search with char-offset matches, line-aware anchors, pinned empty-match iteration and replacement expansion"
tier: high              # ADR-0002: dependency-table change (kiln-search gains `regex`)
state: queued
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
Director rulings after review round 1 (2026-11-13, decision D-070; review `reviews/T-017-r1.md`, run R-117: no defect, one test gap, two spec gaps, two nits). Checked against `regex.rs` (the `find_all` loop, the four public functions) and the drafts of T-021 and T-022 before the builders got them:
- s3 (finding 1, test gap; test author). A template reference is a group number only if it is all ASCII digits. Acceptance tests add references that `str::parse::<usize>` accepts but the rule refuses, at least `${+1}`: the result is `Err(TemplateError)` whose `reference` is the text between the braces. The reviewer's surviving mutant (the digits check dropped) must be killed.
- s4 (finding 2, spec gap; corrects s1). s1 claimed the 100-group cap bounds memory. It bounds the engine's working memory, not the result: `find_all` returns every match with `group_count() + 1` slots, about 2.4 kB per match at 100 groups, so peak memory grows with matches × groups (2.40 GB measured for 100 empty groups on a 1 MB text). No cap on groups can bound a quantity that grows with the text. The API of this task does not change: `find_all` keeps its pinned signature, and a bounded search (stop after N matches, or count without building group slots) changes what `find_all`, `search.find`'s `total` and replace-all promise, which is new scope and not round-2 work. The implementer states the cost in the crate documentation of `find_all`: memory proportional to matches × (groups + 1), with the measured figure, and that a caller handling untrusted text or patterns must bound it. The bounded search is specified by the planner in T-021 before T-021 is dispatched.
- s5 (finding 3, spec gap). Pinned (e)'s 5 s bound holds for compiling a pattern and for searching a text the pattern does not match; on a matching text a 100 kB pattern takes longer inside the regex crate itself. Pinned (e) is read that way, the crate documentation says so, and no test asserts a time for a 100 kB pattern on a matching text. A pattern-size or time limit for searches is T-021's to specify with the bounded search.
- s6 (nit 4; implementer). The truncation oracle in `crates/kiln-search/tests/regex.rs` builds the expected engine error with the same builder options as the code (ruling s2).
- s7 (nit 5). The wall-clock assertions stay, as Pinned (e) requires them; no change.
Director rulings after review round 2 (2026-11-13, decision D-077; review `reviews/T-017-r2.md`, run R-134: one defect in the delta, caused by ruling s5):
- s8 (corrects s5; findings 1, 2 and 4 of round 2). s5 said the 5 s bound of Pinned (e) holds "for searching a text the pattern does not match". That is false: the engine's cost grows with pattern size × text size whether or not the text matches, and the round-2 reviewer measured 245 s in a debug build for a 100 kB pattern (`[ab]` 25,000 times, then `[^ab]`) on a 100,000-char text it does not match. s5 generalised from one short text in round 1; the implementer documented s5 faithfully. Now pinned, and only this: (1) compiling a pattern, including a 100 kB one, answers or errors within 5 s; (2) the three probes of Pinned (e) with small patterns (`(a*)*b` on 100,000 `a`s, `\p{L}{1000}`, deeply nested groups) answer or error within 5 s, as the acceptance test asserts; (3) for searching with a large pattern this crate promises no time bound, on matching or non-matching text. Pinned (e)'s "a 100 kB pattern" is read as (1). The implementer corrects the crate documentation to say exactly (1)–(3), with the two measured figures (245 s as above; 30 s and more on matching texts, round 1), and writes the memory figure as "about 2.46 GB (2.29 GiB)": round 1's figure was in KiB and was read as decimal GB. No code, API or test changes. The time bound for searches is T-021's (its pattern cap), and it must hold for non-matching texts too.
- s9 (process). A defect found in round 2 is confirmed fixed in a round 3 that looks at nothing else (METHODOLOGY v1.1); the director authorises that round for this task. There is no round 4.

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
- 2026-11-13T12:23Z · R-104 · ready · did: restated the ACs: (1, 3) `find_all` gives
  leftmost-first, non-overlapping matches with char offsets, groups included, CRLF-mode anchors,
  the discard-and-restart-at-`e+1` rule for an empty match at the previous end, both case modes;
  (2) `expand` follows Kiln's scanning rule, errors on the first reference to a group the pattern
  lacks (independent of `caps`/`text`), tolerates captures that do not fit, never panics; (4)
  `PatternError` for the listed patterns, message cut to 199 chars + `…`; (5, 6) Pinned (g)
  exactly. Built `crates/kiln-search/src/regex.rs` on `regex = "=1.13.1"` (default features,
  declared in the crate manifest with the workspace-pin comment): `RegexBuilder` with
  `multi_line`, `crlf`, `case_insensitive`; own iteration loop over `captures_read_at` (rule as
  specified, not `captures_iter`); byte→char conversion with one forward cursor over the text
  (O(text + matches·groups)); expansion by hand (`scan` + `group_index`; a name is a number only
  if all ASCII digits and parses as `usize`). Lock change: only `kiln-search`'s dependency list
  (no package added or re-versioned). Tests: 7 unit tests in `src/regex.rs`; 11 in
  `tests/regex.rs` (every Pinned (g) example incl. the nullable table in both modes, all `Err`
  expansions with `Display`, odd captures, AC-4 patterns and the exact 199+`…` cut, inline
  flags, pathological probes of Pinned (e) under 5 s in debug, 100k-match linearity, and a
  3,000-pair fixed-seed differential of `find_all` against the engine's own `captures_iter`,
  converted to chars by brute force). Mutation check: with `crlf(false)` two tests fail. Commit
  6dde8c046dbb33f9bdfa82d366d2e9fddeed675a. · gate attempts: 1 (last: green, 11 steps; fuzz
  and budgets EMPTY) · not run: no naive backtracker of my own (the test author's AC-1/AC-3
  oracle); nothing is observable through `kiln-cli` or the control protocol until T-021, so no
  outside check. · missing context: none · next: test author join, reviewer.
  NOTE (not a question): `expand` maps a group's char range to bytes by walking `text` from its
  start (O(end) per reference), because the API passes char offsets and the whole text. T-021's
  replace-all over many matches is then O(text·matches); T-021 may want to pass the matched
  slice or a char index. Not a T-017 contract change.
- 2026-11-13T12:53Z · R-110 · rework after join (D-060) · did: s1: `Regex::new` refuses a
  pattern the engine accepted with more than 100 explicit groups with message `the pattern has
  more than 100 capture groups`; an engine rejection keeps the engine's message; documented in
  the module docs (`# Limits`) and on `new`/`PatternError`. Unit tests: 100 accepted and usable,
  101 refused, nested/named groups counted, `(?:…)` not counted, 34,000 groups refused < 5 s,
  invalid-and-over-cap keeps the engine error, engine message unchanged for a rejected pattern.
  `tests/regex.rs` now expects the 34,000-group pattern to be an error. Commit
  dedfb94f997914a4ec946b0c0d600326247f2dec. Memory probe (throwaway example, deleted; debug
  build, `/usr/bin/time -v`, ~1 MB matching text): group-heavy `(?:(w…|…)|…)+` (100 groups over
  a 2,000-word alternation, case-insensitive): peak RSS 43,176 kB, but `find_all` took 190 s;
  state-heavy `\w{209}` case-insensitive (largest that fits the size limit): 41,060 kB, 15 s;
  both together `(\w{2})`×100 case-insensitive: 238,740 kB, 27 s (compile 6.7 s). s2: messages
  come from `RegexBuilder::new(pattern)` with the pattern exactly as given and the modes as
  builder options; no prefix is added. · gate attempts: 1 (last: red test:
  `acceptance_t017::invalid_patterns_are_errors` line 1210 expects `nested(250)` (250 groups)
  to compile, which s1 now refuses; that file is the test author's, not edited by me) · not run:
  release-build timings · missing context: none · next: the test author updates the 250-level
  nest assertion for s1 (e.g. with `(?:` groups); then gate again.
  NOTE: the group-heavy probe is under the memory limit but takes 190 s in debug on 1 MB. That
  is outside the Pinned (e) list, but T-022 (find in files) may care.
- 2026-11-13T13:56Z · R-126 · rework after review r1 (D-070) · did: s4: rustdoc of
  `find_all` (`src/regex.rs:271`) and the module `# Limits` docs (`src/regex.rs:39–43`) state
  result memory ∝ matches × (groups + 1), ~2.4 kB/match at 100 groups, 2.40 GB measured for 100
  empty groups on 1 MB, cap bounds the engine's working memory not the result, callers with
  untrusted input must bound the search. s5: `src/regex.rs:45–48` says the 5 s bound covers
  compiling and searching a non-matching text; a 100 kB pattern on a matching text takes longer
  in the engine. s6: `tests/regex.rs:173` builds the expected engine error with
  `RegexBuilder` + `multi_line` + `crlf`. No API, behaviour or timing-test change. Commit
  871e28abb713d82f74cd3706fe33a99d5b264cd6. · gate attempts: 1 (last: green, 11 steps; fuzz and
  budgets EMPTY; test 506 passed) · not run: s3 tests (test author's, at re-join) · missing
  context: none · next: re-join, review round 2.
<!-- end implementer log -->
### test author
- 2026-11-13T12:34Z · R-105 · ready · did: wrote `crates/kiln-search/tests/acceptance_t017.rs`
  (14 tests) from the spec alone, base main f36b83e2f1fbd434c35b459cda5685177f852ce0. No guard
  file: the crate is empty on base, so there is no existing literal search to protect.
  Oracles: a naive backtracking matcher of the AC-1 subset (program of Split/Save/assert
  instructions, the spec's iteration, CRLF anchor and capture rules; case folding by the AC-1
  predicate, classes folded before negation via a brute-force fold set over all scalar values);
  a naive expansion rule; a naive literal search vs a fully escaped pattern; prefix char counts.
  Criterion → tests → result on base (all: compile, E0432/E0433, new API and `regex`):
  AC-1 → `matches_equal_naive_backtracker`, `case_insensitive_matches_equal_naive_model` (20,000
  fixed-seed pairs each, groups and `group_count` included, generator restriction asserted,
  non-degeneracy counts asserted), `naive_model_self_check` (model sanity, passes on its own
  logic but shares the file, so it also does not compile on base);
  AC-2 → `expansion_equals_naive_model` (5 patterns × 4,000 random templates/captures, `Err`
  reference and `Display`, error status independent of caps/text),
  `expansion_ignores_ranges_that_do_not_fit`;
  AC-3 → `offsets_are_chars` (2 × 3,000 model pairs over `a b é É 中 😀 \r \n` space `B`, char
  slicing, planted needles after multi-byte/CRLF prefixes, empty pattern at every char),
  `escaped_literal_matches_literal_search` (3,000 needles incl. all metacharacters, both modes);
  AC-4 → `invalid_patterns_are_errors` (the five patterns, both modes; 300-nested and long
  `(`+300 `a`/`é`/`😀` messages = engine `Display` cut to 199 chars + `…`; nest 250 ok, 251 err),
  `error_message_cut_boundary` (engine messages of exactly 200 and 201 chars found by search),
  `pathological_patterns_within_five_seconds` (Pinned (e) probes);
  AC-5 → `worked_examples_hold` (every Pinned (g) example incl. all expansion rows and `$12x`),
  `api_contract` (derives, default, `len == group_count()+1`), `line_model_and_inline_flags`;
  AC-6 → `nullable_quantifier_examples_hold` (all five rows, both modes).
  The AC-4 oracle calls `regex::RegexBuilder` (multi_line, crlf, case_insensitive) directly: a
  normal dependency of kiln-search after the task, no dev-dependency added.
  Self-check: a throwaway stub over regex 1.13.1 passed all 14 (suite 49 s in debug, mostly
  AC-1); 16 seeded mutants, 16 killed: crlf off (AC-1, AC-3, AC-5, flags), no empty-match
  discard (AC-1, AC-3, AC-5, AC-6), byte offsets (AC-1, AC-3, escaped, AC-5, AC-6), last match
  dropped (8 tests), no message cut (both AC-4 cut tests, Pinned (e)), cut by bytes
  (`invalid_patterns_are_errors`), last group dropped (6), missing group expands to nothing
  instead of `Err` (both AC-2, AC-5), `Display` without quotes (AC-2, AC-5), `$1a` read as `$1`+`a`
  (AC-2, AC-5), group exists iff `< caps.len()` (both AC-2), option `case_insensitive` ignored
  (AC-1 ci, AC-3, escaped, AC-5, flags), groups 1/2 swapped (5), None group filled with `0..0`
  (5), `$$` not an escape (AC-2, AC-5), out-of-range ranges clamped instead of dropped (both
  AC-2). Clippy `-D warnings` clean with the stub; `cargo fmt --all -- --check` clean; stub,
  `Cargo.toml` and `Cargo.lock` changes removed. · gate attempts: 1 (last: red clippy, only
  `acceptance_t017.rs` failing to compile, as expected on base) · not run: the gate with the
  implementation (join) · missing context: none · next: join.
  QUESTION: Pinned (e) "a 100 kB pattern … must answer or error within 5 s". With
  `(a)`×34,000 (the spec check's probe) compiling is fast, but `find_all` on a text that
  matches makes regex 1.13.1 try to allocate about 55 GB and abort the process (observed in the
  stub, `captures_iter`). My test therefore probes only a non-matching text. Proposed wording:
  "compiles (or errors) and `find_all` on a text without a match answers within 5 s"; or should
  the implementation refuse patterns with more than N groups (a `PatternError`), which T-021
  would then inherit?
  QUESTION: AC-4's "exactly its first 199 chars" assumes the engine is built with
  `RegexBuilder` options; an implementation that prefixes inline flags to the pattern (e.g.
  `(?mR)`) gets a different engine message. My test compares with
  `RegexBuilder::new(p).multi_line(true).crlf(true).case_insensitive(ci)`. Please confirm.
- 2026-11-13T12:45Z · R-111 · ready · did: tests for rulings s1/s2 (D-060):
  `hundred_capture_groups_are_accepted` (99 plain + 1 `(?P<n>…)` + 50 `(?:…)` → 100 groups,
  groups and expansion checked), `more_than_hundred_capture_groups_are_refused` (101 plain,
  100 + named, mixed with `(?:…)`, nested; exact message), `huge_group_pattern_is_refused_within_five_seconds`,
  `crate_error_takes_precedence_over_group_cap`. Named groups count and `(?:…)` does not,
  derived from the Interface ("explicit groups"; `len = group_count() + 1`) and Pinned (g).
  One existing assertion contradicted s1 and was changed: in `invalid_patterns_are_errors`,
  250 nested capture groups compiled; now the nest-limit check uses `(?:…)` (250 ok, 251 crate
  error) and 250/251 nested capture groups give the cap message / the crate's error. Base:
  compile (E0432/E0433). Stub self-check: 18/18 pass; 7 cap mutants (limit 99, limit 101, check
  before the crate, named not counted, `(?:…)` counted, no cap, wrong message), 7 killed.
  Clippy with stub and fmt clean; stub removed. · gate attempts: 0 · not run: `cargo xtask ci`
  · missing context: none · next: join.
- 2026-11-13T13:54Z · R-125 · ready · did: ruling s3 (D-070): new test `plus_signed_references_are_not_group_numbers` (`${+1}`, `${+0}`, `${+2}`, `x${0}${+1}` → `Err` with the text between the braces; `$+1` → literal `Ok("$+1")`); `+` added to the AC-2 template alphabet (the naive model already checks all-ASCII-digits before parsing, unchanged). Base: compile. Stub self-check: passes; the digits-check-dropped mutant is killed by both new test and `expansion_equals_naive_model`. fmt clean; stub removed. · gate attempts: 0 · not run: `cargo xtask ci` · missing context: none · next: join.
<!-- end test author log -->
### reviewer
- 2026-11-13T14:05Z · R-117 · tests_joined · did: round-1 review of `43fae907bda9ee0abe2c21b3adb3419e09160dfb`
  (`reviews/T-017-r1.md`), verdict request-changes. Findings: 1 test gap (no test pins
  "all ASCII digits": dropping the check makes `${+1}` group 1; mutant 16 survived), 2 spec
  gaps for the director (`find_all` under the cap peaks at 2.40 GB on 1 MB with `()`×100,
  debug and release; the Pinned (e) 5 s bound for a 100 kB pattern holds only on a non-matching
  text: 85 s in release, inside the crate), 2 nits. Mutants: 17 introduced, 3 equivalent, 13
  killed (all by acceptance), 1 survived. Independent probes: 25,000 random match pairs and
  15,150 expansions against my own models, 0 differences; lock diff, licences, build scripts,
  layers checked. · gate attempts: 1 (last: green, 11 steps; fuzz and budgets EMPTY); `cargo xtask hygiene`
  with the review staged: PASS (5 of 16 reviews checked) · not run: the implementer's exact 190 s probe (pattern not recorded) · missing context: none · next:
  implementer adds the `+1` test; director rules on findings 2 and 3.
<!-- end reviewer log -->
