---
id: T-021
title: "Search an open document by literal or regex, case-sensitive or not, optionally whole-word, and replace one or all matches as one undo step with capture groups"
tier: medium            # ADR-0002 default: in-memory behaviour; adds public API to kiln-search (not a CHARTER §3 contract, no dependency); the regex dependency was added by T-017
state: draft
owner_role: planner
scorecard_items: [search.literal, search.regex, search.whole-word, search.replace]
depends_on: [T-015, T-017, T-018, T-032]   # all merged (spec check R-158 against main 1dc59d0): module slot; Regex contract; apply_edits; invalid_pattern
touches:
  - crates/kiln-search/src/lib.rs         # re-exports; the crate doc's "Literal search … come with T-021" line
  - crates/kiln-search/src/regex.rs       # R-127: additive only, `Regex::for_each_match` (D-070 s4) and private helpers; T-017's public items unchanged
  - crates/kiln-search/src/query.rs       # new: Query, literal/whole-word matching, replacement planning
  - crates/kiln-search/tests/query.rs     # implementer's own integration tests
  - crates/kiln-search/tests/acceptance_t021_search.rs  # implementer (odd arm), on task/T-021-tests: the Rust-API parts of AC-7 and AC-10
  - crates/kiln-commands/src/search.rs    # search.* (slot from T-015); QueryError → RejectReason mapped here
  - apps/kiln-cli/tests/acceptance_t021_cli.rs          # implementer (odd arm), on task/T-021-tests: AC-1 to AC-9 through kiln-cli
  - docs/protocol/search.md               # new
  - docs/protocol/cursor.md               # the list of selection-moving commands (:8–9) and the "(`eol.normalize`)" example (:80) only, to name `search.replace` (R-158)
  - scorecard/items/search.literal.toml   # director at join (METHODOLOGY §6): this task's test names
  - scorecard/items/search.regex.toml     # director at join
  - scorecard/items/search.whole-word.toml  # director at join
  - scorecard/items/search.replace.toml   # director at join
  # docs/commands.json changes: generated (`cargo xtask schema --write`); docs/scorecard.md: generated
  # NOT touched (R-158): crates/kiln-commands/src/reason.rs (pending Q-3), any Cargo.toml or Cargo.lock, docs/control-protocol.md
budget_tokens: 700000   # medium (v1.2, D-091 change 3)
iteration: 2
found_by: planner
origin_task: null
---

# T-021 Search and replace in an open document

## Goal
After this task `search.find` lists the matches of a query in an open document, as char
ranges, with the exact number of all matches. The query is literal or regex, case-sensitive or
not, and optionally restricted to whole words. `search.replace` replaces the first match at or
after an offset, or all matches, as a single undoable step. In regex mode the replacement can
use capture groups. `kiln-search` gains the `Query` type that find-in-files (T-022) reuses.

No gate path. Arm: **odd**. The implementer writes the acceptance tests on `task/T-021-tests`
first, through the public interface, then the code (METHODOLOGY §6).

Reading list (all on main now): `crates/kiln-search/src/regex.rs` module docs (Iteration,
Expansion, Limits) and `tasks/T-017-*.md` Semantics and rulings s1, s2, s4, s8; the cost of
`Regex::expand`: its private `byte_range` (`crates/kiln-search/src/regex.rs:174–193`) walks
`text` from its start for every reference, so one call is O(position of the group);
`crates/kiln-search/tests/acceptance_t017.rs` (its naive backtracker and case-fold predicate,
which AC-1/AC-2's oracles may copy: `kiln-cli`'s tests cannot depend on `kiln-search`);
`crates/kiln-text/src/buffer.rs:524–556` (`apply_edits`, batch rule) and `docs/protocol/cursor.md`
(mapping rule); `crates/kiln-commands/src/eol.rs` (a command group built on one batch; the
`present` deserializer of ruling x1, D-084); `crates/kiln-commands/src/reason.rs`
(`invalid_pattern`, T-032); the "Spec check R-158" section under Questions and rulings.

## Interface

### `kiln_search` (L1)
`find_all` (T-017) returns every match with `group_count() + 1` slots, about 2.4 kB per match at
100 groups; 100 empty groups on a 1 MB text peaked at about 2.46 GB (T-017 review r2 nit 4,
D-077 s8). The 100-group cap does not bound that, and T-017's API is frozen. This task
therefore adds one streaming primitive to `Regex` and builds every command on it; no command
of this task (or T-022) calls `find_all`.
```rust
// crates/kiln-search/src/regex.rs: additive; `new`, `group_count`, `find_all`, `expand` unchanged.
// Private helpers (e.g. an expansion that takes byte offsets) may be added.
impl Regex {
    /// Calls `f` once per match, in `find_all`'s order, with a `Captures` equal to the one
    /// `find_all(text)` holds at that position; stops after a call that returns
    /// `ControlFlow::Break(())`. Memory beyond the engine is O(group_count()), whatever the
    /// number of matches: the value passed to `f` is reused, so `f` copies what it keeps.
    pub fn for_each_match(&self, text: &str, f: &mut dyn FnMut(&Captures) -> std::ops::ControlFlow<()>);
}

// crates/kiln-search/src/lib.rs
pub mod query;
pub use query::{CompiledQuery, Found, Query, QueryError, MAX_PATTERN_CHARS};

// crates/kiln-search/src/query.rs (uses the existing `CharMatch`, `Captures`, `PatternError`,
// `TemplateError` of `regex.rs`; no new dependency: `kiln-search` keeps exactly `regex = "=1.13.1"`)
pub const MAX_PATTERN_CHARS: usize = 1000;                    // D-071 c3
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Query { pub pattern: String, pub regex: bool, pub case_sensitive: bool, pub whole_word: bool }
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueryError { Empty, TooLong { chars: usize }, Pattern(PatternError) }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found { pub matches: Vec<CharMatch>, pub total: usize }
impl Query {
    /// `Empty` → `TooLong` (pattern longer than `MAX_PATTERN_CHARS` chars, both modes,
    /// checked before compiling) → `Pattern` (regex mode: `Regex::new` with
    /// `RegexOptions { case_insensitive: !case_sensitive }`).
    pub fn compile(&self) -> Result<CompiledQuery, QueryError>;
}
impl CompiledQuery {
    /// The whole-match ranges of the first `limit` matches (whole-word filter applied) and
    /// `total`, the exact number of all matches (D-071 c1). `limit` may be 0 (count only;
    /// T-022 relies on it). Memory: O(limit + group_count()). Time: one pass over the text.
    pub fn find(&self, text: &str, limit: usize) -> Found;
    /// `Ok(())` iff `replacement` expands for this pattern (literal mode: always). Depends on
    /// the pattern and the template only (T-017 expansion rule).
    pub fn check_template(&self, replacement: &str) -> Result<(), TemplateError>;
    /// The replacement text for one match: literal mode → `Ok(replacement)`; regex mode →
    /// T-017's `Regex::expand`, which walks `text` from its start per reference (Reading
    /// list), so it is for single matches only.
    pub fn replacement_for(&self, caps: &Captures, text: &str, replacement: &str)
        -> Result<String, TemplateError>;
    /// The first match (iteration from char 0) whose start ≥ `from`, with its replacement
    /// text. Memory: O(group_count() + replacement text). Stops at that match.
    pub fn replace_first(&self, text: &str, from: usize, replacement: &str)
        -> Result<Option<(CharMatch, String)>, TemplateError>;
    /// Every match's whole range with its replacement text, in order.
    /// Memory: O(matches + total replacement text), never O(matches × groups).
    /// Time: O(len(text) + Σ over matches of (match length × references in `replacement`))
    /// on top of the engine's search, never O(len(text) × matches). No bound on the number
    /// of matches (D-071 c2).
    pub fn replace_all(&self, text: &str, replacement: &str)
        -> Result<Vec<(CharMatch, String)>, TemplateError>;
}
```
Literal mode answers through the same functions (one group, the whole match). Each
function's results equal what a model built on `find_all` would give; only memory and time
differ. `CompiledQuery` has no `find_all` (R-127: removed from the draft).

### Matching rules (exact)
- Empty `pattern` → `QueryError::Empty` in both modes.
- Literal: occurrences of `pattern` as a char sequence, scanned left to right, non-overlapping
  (after a match the scan resumes at its end). With `case_sensitive: false`, chars compare by
  Unicode simple case folding, as T-017's case-insensitive mode does. `\r\n` in the pattern
  matches only `\r\n`; a `\n` in the pattern also matches the `\n` of a `\r\n`.
- Regex: T-017's `Regex` with `case_insensitive = !case_sensitive`, with T-017's iteration.
  Inline flags in the pattern (`(?i)`, `(?-i)`, `(?s)`, …) are accepted and override
  `case_sensitive` and the line modes for their scope, as T-017 pins.
- Whole word (both modes): first the matches the search reports without whole-word, then keep
  those with a word boundary at both ends. Word char = `c == '_' || c.is_alphanumeric()`. A
  match `[s, e)` has boundaries iff (`s == 0` or char `s−1` is not a word char) and
  (`e == len` or char `e` is not a word char). Nothing is re-scanned after a rejected match.
- The matches of one search are sorted by start, pairwise distinct in start and
  non-overlapping, so they always form a valid `Buffer::apply_edits` batch (T-018 batch rule;
  T-017 iteration: no two matches share a start, an empty match never sits at the previous
  match's end).

### Commands (`doc` must be open)
Common params: `doc` (integer, an open document's id, as in `doc.*`), `query` (string),
`regex?` (default false), `case_sensitive?` (default true), `whole_word?` (default false).
Optional params, when present, must not be `null` (`invalid_params`/`schema`), as for
`eol.normalize` (ruling x1, D-084). Unknown params are `invalid_params` (`deny_unknown_fields`).

| Command | Mutating | Extra params | Result |
|---|---|---|---|
| `search.find` | no | `limit?` integer 1..=100000 (default 10000) | `{matches:[{start,end}], total, truncated}` |
| `search.replace` | yes | `replacement` string; `all?` bool (default false); `from?` offset (default 0); `limit?` integer 1..=100000 (default 10000), PENDING Q-2 | `{replaced, len_chars, ranges:[{start,end}], truncated}`, `ranges` and `truncated` PENDING Q-2 |

- `search.find`: `CompiledQuery::find(text, limit)`. `total` is the exact number of all
  matches (D-071 c1); `matches` holds the first `limit`; `truncated = total > limit`.
  `limit: 0` or `100001` → `invalid_params`.
- `search.replace` with `all: false` replaces the first match whose `start ≥ from`; with
  `all: true` it replaces every match. `from` is ignored then, and there is no wrap-around.
  `all: false` uses `replace_first`, `all: true` uses `replace_all`.
  All replacements are one `Buffer::apply_edits` call, so one `doc.undo` reverts them all; the
  selection and the anchors move through each replacement by the mapping rule
  (`docs/protocol/cursor.md`), as for `eol.normalize`. With no match, `replaced: 0`
  and nothing changes: no undo step, the typing group is not ended.
- `ranges` (PENDING Q-2, written for its recommendation (a)): the ranges of the inserted texts
  in the new text, in order, for the first `limit` replacements; `truncated = replaced >
  limit`. Every replacement is made whatever `limit` is; `limit` bounds only the reply.
- Refusals, after schema and `no_such_document` (Pinned (a)): `QueryError::Empty`, `TooLong`
  and `Pattern` → `rejected`/`invalid_pattern`. The message is `empty search pattern`, or
  `invalid pattern: the pattern has <chars> chars; the limit is 1000` (`TooLong`, both modes),
  or `invalid pattern: <PatternError message>`; for a pattern with
  more than 100 capture groups (T-017 ruling s1) that is exactly
  `invalid pattern: the pattern has more than 100 capture groups`; 100 groups are accepted.
  Other `PatternError` messages are the regex crate's (T-017 s2) and are not pinned here; the
  protocol cuts every message to 200 chars (`docs/control-protocol.md`, Response). Then
  `from > len_chars` → `out_of_range`; `from` inside a CRLF is not refused. All kinds exist
  (T-032's reason table); no new kind.
- Replacement naming a group that does not exist (regex mode; T-017 `TemplateError`): whether a
  template is an error depends only on the pattern and the template, so `search.replace`
  checks it (`check_template`) before matching, whether or not there is a match, and refuses with
  `rejected`/`invalid_pattern`, message `invalid replacement: <TemplateError Display>` (e.g.
  `invalid replacement: no capture group "3"`). Nothing changes. (Ruled by D-044.)
- Bounds (D-070 s4/s5, D-071). Memory: `search.find` holds at most `limit` ranges plus one match's
  slots; `search.replace` holds one match's slots (`all: false`) or one range and one
  replacement string per match (`all: true`), never the group slots of more than the current
  match, and its reply holds at most `limit` ranges (PENDING Q-2). Time: no bound (PENDING Q-1,
  written for its recommendation (a)); see Pinned (e) for what was measured. No wall clock is
  used (CHARTER §3 decision 4).

`docs/protocol/search.md` documents the commands and all rules above, with the examples of
Pinned (g), T-017's expansion rule (a reference to a group that does not exist is refused,
unlike the `regex` crate's own expansion), that inline flags override the options, and,
word for word, the sentences of Pinned (i).

## Pinned
- (a) Precedence: schema → `no_such_document` → `invalid_pattern` (the pattern: empty, too
  long, not compiling or more than 100 groups; then the replacement template) →
  `out_of_range`. (R-127: the Refusals bullet used to list `invalid_pattern` before
  `no_such_document`; this order governs.)
- (b) Files: none.
- (c) Order: matches are computed on the document's current text before any replacement;
  each replacement text is computed from its match's captures as the match is found.
- (d) Offsets are chars. Platform independent, except AC-8's `ulimit -v`, which is Unix
  (D-014).
- (e) Threat model: n/a (no security boundary). Resource bounds come from this task, not from
  T-017: the 100-group cap bounds only the engine's working memory (D-070 s4). Memory is
  bounded by never keeping more than one match's slots (Interface, "Bounds"). Time (PENDING
  Q-1): the spec check (R-158) measured that the 1,000-char cap does **not** bound search
  time, because a counted repetition is short to write and large to run; the figures are in
  "Spec check R-158" below. Under Q-1's recommendation the task promises no time bound,
  documents the measured figures (Pinned (i), S4) and has no time-bound acceptance test; the
  earlier instruction that the reviewer reports "above about 5 s" as a finding is withdrawn
  (every probe at or under the cap that matters exceeds it). If Q-1 rules otherwise, this
  line, AC-11 and S4 are rewritten with the ruling, and any time-bound test uses a text
  longer than the pattern's minimum match length (D-077 c7).
- (f) Snapshots: none. No test of this task pins a whole document, the list of commands, the
  `error_kinds` list or a `when` text (T-038 changes them; D-091 change 6). The messages pinned
  in Interface are this task's own; no drafted task changes them (T-022 reuses them, T-038
  leaves messages alone, its "Out of scope").
- (g) Worked examples (executed against a naive Python model; regex rows against regex 1.13.1;
  the `search.find` results of the third and fourth rows re-run by the spec check R-158 on a
  throwaway stub over merged `Regex::find_all`: `baaab`/`a*` → `0..0, 1..4, 5..5`,
  `a1 b2`/`(\w)(\d)` → `0..2, 3..5`):
  - text `foo FOO fOo_x éFoo`, literal `Foo`, `case_sensitive:false` → `0..3, 4..7, 8..11,
    15..18`; with `whole_word` → `0..3, 4..7` (`8..11` is followed by `_`, `15..18` preceded by
    `é`).
  - text `aaa b aa`, literal `aa` → `0..2, 6..8`; with `whole_word` → `6..8`.
  - text `baaab`, regex `a*` → `0..0, 1..4, 5..5`; `search.replace {replacement:"-", all:true}`
    → text `-b-b-`, `replaced: 3`, `ranges` `0..1, 2..3, 4..5`.
  - text `a1 b2`, regex `(\w)(\d)`, replacement `$2$1`, `all:true` → `1a 2b`, `replaced: 2`.
    One `doc.undo` → `a1 b2`.
  - literal `$1` replacement in literal mode is inserted verbatim.
  - text `a1 b2`, regex `(\w)(\d)`, replacement `$3` or `$x`, `all:true` → refused
    (`rejected`/`invalid_pattern`, D-044), text unchanged; `$0-` →
    `a1- b2-`. (Executed against a naive model of T-017's rule, R-079.)
- (h) Evaluator calls: `kiln-cli run` with `doc.open`, `search.find` (each option combination),
  `search.replace`, `doc.undo`, `doc.redo`, `doc.text`, `doc.info` (`can_undo`), `cursor.get`.
- (i) Guarantee and limit sentences for `docs/protocol/search.md`, to be copied (any line
  wrapping; `<N>` is filled in by the implementer from its own measurement, ruling c2). No
  other sentence stating a guarantee or a limit is added; an implementer who needs one
  records its evidence in its log (D-091 change 5).
  - S1: "`search.find` reports `total`, the exact number of matches, and lists at most `limit`
    of them; it keeps at most `limit` ranges and the capture groups of one match at a time,
    however many matches there are." Evidence: AC-6, AC-8 (find half; R-158 stub: 16,548 KiB
    peak under a 512 MiB cap, while `find_all` aborts).
  - S2: "`search.replace` with `all: true` replaces every match, however many, as one undo
    step: one `doc.undo` restores the text and one `doc.redo` re-applies it. It keeps one
    range and one replacement text per match, never the capture groups of more than one
    match: about <N> bytes per match, measured on 400,001 matches of 100 empty groups." Evidence:
    AC-4, AC-8 (replace half), the implementer's measurement (c2); R-158 stub figure for
    reference: 104,508 KiB peak for the whole AC-8 script without reply ranges.
  - S3 (PENDING Q-2 (a)): "The reply of `search.replace` lists the ranges of the first `limit`
    replacements (default 10,000) and sets `truncated` when there were more; every match is
    replaced whatever `limit` is." Evidence: AC-8, AC-12.
  - S4 (PENDING Q-1 (a)): "A pattern may have at most 1,000 chars and, in regex mode, at most
    100 capture groups. These limits do not bound the time a search takes: a counted
    repetition is short to write but large to run. Measured in a debug build on texts of
    1,000,000 chars, `[ab]{5000}[^ab]` (15 chars, no match) took 544 s, `[ab]{3000}` (10 chars,
    333 matches) 172 s and `([ab]*)` repeated 100 times (800 chars, one match) 38 s. Kiln sets
    no time limit on a search." Evidence: AC-2, AC-9 (limits); "Spec check R-158", table T1
    (times).
  - S5: "With no match, `search.replace` changes nothing: it adds no undo step and an open
    typing group stays open." Evidence: AC-4.
  - S6: "Matches never overlap and are found from the start of the text; `from` applies only
    to `all: false`, and there is no wrap-around." Evidence: AC-4.
  - S7: "The selection moves through each replacement by the mapping rule of `cursor.md`."
    Evidence: AC-4 (selection model).
  - In `docs/protocol/cursor.md`: `search.replace` is added to the list of commands at :8–9
    that move the selection, and "(`eol.normalize`)" at :80 becomes "(`eol.normalize`,
    `search.replace`)". Evidence: AC-4.
  - `docs/control-protocol.md` is not changed: this task adds commands, not CLI options or exit
    codes (D-091 change 18 does not apply).

## Acceptance criteria
All through `kiln-cli run` or `kiln-cli control` (file `apps/kiln-cli/tests/acceptance_t021_cli.rs`),
except the Rust-API parts of AC-7 and AC-10 (file `crates/kiln-search/tests/acceptance_t021_search.rs`:
`kiln-cli` has no `kiln-search` dependency and adding one is a dependency change). Send many
requests per process (one `control` session or one `run` script per batch), not one process
per random case.
- AC-1 (`search.literal`): for ≥ 5,000 fixed-seed random (text, pattern) pairs over `a b A B _ 0
  é É σ Σ ς ſ s S K k \r \n` and space, `search.find` (literal, both case settings) equals a
  naive scan. The naive scan compares chars with T-017's test predicate ("equal, or equal
  lowercase, or equal uppercase") when case-insensitive. Oracle: naive model.
- AC-2 (`search.regex`): for ≥ 5,000 random pairs in T-017's regex subset, generated with
  T-017's restriction (no `*`, `+`, `?` or lazy form on a sub-pattern that can match the empty
  string; T-017 AC-1 says why), `search.find` equals a naive backtracking model, written in the
  test, of T-017's iteration, anchor and capture rules. `(`, `[a-`,
  `a{99999999}` and the empty pattern give `rejected`/`invalid_pattern`, and the session is
  unchanged. A pattern of 101 groups `(a)(a)…(a)` → `rejected`/`invalid_pattern` with the exact
  message of Interface; 100 groups → `ok`. Oracle: naive model + exact value.
- AC-3 (`search.whole-word`): for the AC-1 and AC-2 inputs, `whole_word: true` equals the naive
  filter of the non-whole-word result. The worked examples hold. Oracle: naive model.
- AC-4 (`search.replace`): for random texts, queries and replacement templates (regex mode:
  T-017's AC-2 template alphabet and its expansion rule), `all: true` gives `doc.text` equal to
  the naive replacement, and `replaced` equals the match count. A template that names a group
  that does not exist is refused as in Interface, and the session is unchanged.
  `all: false, from: f` replaces exactly the first match with `start ≥ f`. One `doc.undo`
  restores the original text exactly, and `doc.redo` re-applies. `replaced: 0` leaves
  `can_undo` unchanged and an open typing group open (a one-char `doc.insert`, then a
  `search.replace` with no match, then a one-char `doc.insert` right after the first: one
  `doc.undo` removes both chars). After a replacement with a selection set by `cursor.set`,
  `cursor.get` equals a naive model of the mapping rule applied to the edits from the last to
  the first. Oracle: naive model + round trip.
- AC-5 (worked examples): every example in Pinned (g) holds exactly. Oracle: exact value.
- AC-6 (limit): a text with 20,001 matches and `limit: 10` gives 10 matches, `total: 20001`,
  `truncated: true`. Oracle: exact value.
- AC-7 (`search.replace`, bounded cost): a document of `ab` repeated 200,000 times (400,000
  chars), regex `(a)b`, replacement `$1$1`, `all: true` → `replaced: 200000` and `doc.text`
  equal to `aa` repeated 200,000 times, under a 20-second timeout on the whole `kiln-cli run`
  (a hang-style bound, not a measurement: one walk from the text's start per reference is
  about 8·10^10 char steps here, far beyond it; R-158's stub with a linear expansion ran this
  script in 3.14 s debug, 86,396 KiB peak). And through the Rust API
  (`acceptance_t021_search.rs`), `replace_all` equals `replacement_for` applied to each match
  of `find_all` on the AC-4 inputs. Oracle: exact value + coarse timeout.
- AC-8 (bounded memory): `kiln-cli run` is started through `sh -c 'ulimit -v 524288; exec <kiln-cli> run
  s.jsonl --root <dir>'` (a 512 MiB address-space cap; Unix, D-014) under a 120-second timeout
  (a hang bound; R-158 measured 21–26 s for this script in debug, too close to the 30 s first
  drafted). The document is `x` repeated 400,000 times; the regex is `()` repeated 100 times
  (100 empty groups, 200 chars). `search.find {limit:10}` → matches `0..0` … `9..9`, `total: 400001`,
  `truncated: true`; `search.replace {replacement:"-", all:true}` → `replaced: 400001`,
  `len_chars: 800001`, `truncated: true` and 10,000 `ranges` (PENDING Q-2); `doc.undo` restores
  the text. Every line `ok`, exit 0. An implementation that keeps every match's slots needs
  about 400,001 × 2.4 kB ≈ 0.96 GB and aborts under the cap (measured by R-158: "memory
  allocation of 2424 bytes failed", no response line): the test detects it without using the
  memory. Oracle: exact value.
- AC-9 (pattern cap): in both modes a 1,001-char pattern → `rejected`/`invalid_pattern` with
  message `invalid pattern: the pattern has 1001 chars; the limit is 1000`; a 1,000-char
  pattern (`a` repeated) is accepted. Oracle: exact value.
- AC-10 (`for_each_match`, Rust API, `acceptance_t021_search.rs`): without a break it yields
  exactly `find_all(text)` element by element on the AC-2 inputs; a break after k calls yields
  its first k. Oracle: `find_all` (merged, T-017).
- AC-11 (time): none under Q-1 (a) (Pinned (e)).
- AC-12 (reply limit, PENDING Q-2 (a)): on AC-6's text, `search.replace {all:true, limit:10}` →
  `replaced: 20001`, 10 `ranges`, `truncated: true`, and `doc.text` equals the naive
  replace-all of every match. Oracle: naive model + exact value.

Every AC except AC-10 is an **acceptance** test that fails on base by assertion (on base no
`search.*` command exists, so every request is `unknown_command`); AC-10 fails on base by
compilation (new API), which METHODOLOGY §6 accepts as weak evidence.

## Scorecard items and intended test names
| Item | Tests |
|---|---|
| search.literal | `kiln_cli::acceptance_t021_cli::literal_find_matches_naive_scan` |
| search.regex | `kiln_cli::acceptance_t021_cli::regex_find_matches_naive_backtracker`, `kiln_cli::acceptance_t021_cli::invalid_patterns_are_reported`, `kiln_cli::acceptance_t021_cli::pattern_length_is_capped` |
| search.whole-word | `kiln_cli::acceptance_t021_cli::whole_word_filters_at_identifier_boundaries` |
| search.replace | `kiln_cli::acceptance_t021_cli::replace_all_matches_naive_model_and_undoes_in_one_step`, `kiln_cli::acceptance_t021_cli::replace_one_from_offset`, `kiln_cli::acceptance_t021_cli::replace_all_is_linear`, `kiln_cli::acceptance_t021_cli::search_memory_is_bounded` |

The implementer lists the tests it wrote in its log; the director ties them to the items at
the join (METHODOLOGY §6, D-091 change 8).

## Out of scope and implementer notes
- Search across files (T-022); find-next with selection or wrap-around (UI, iteration 2);
  multi-line literal patterns beyond exact char equality; incremental search.
- No dependency change: no `Cargo.toml` dependency table or `Cargo.lock` edit (that would make
  the task high; the queue refuses it as `dependencies`). `kiln-search` does not gain
  `kiln-text`; `query.rs` works on `&str`.
- No new reason row and no `when` text change in `reason.rs` (PENDING Q-3); refusal
  declarations per command are T-038's, which runs after this task (D-092).
- Your own tests: `crates/kiln-search/tests/query.rs`, unit tests. Acceptance tests (odd arm)
  go on `task/T-021-tests` in the two `acceptance_t021_*.rs` files of `touches`, committed and
  failing on base before you write the code.
- Debug-build cost of the heavy ACs measured by R-158 (stub): AC-8 21–26 s, AC-7 3 s. A
  `search.find` with 100 groups costs about 8.5 s per 400,000 matches in debug (the capture
  engine), so keep other large inputs small.

## Questions and rulings
Director rulings on the planner's choices (2026-11-13, decision D-071; planner run R-127), made after reading the choices section, AC-8, AC-9 and the `replace_all` signature above, and CHARTER §3 decision 4:
- c1 (`total` when the limit is hit): option (a). `total` stays the exact number of all matches, counted in the same pass without keeping the matches; `truncated = total > limit`. No schema change.
- c2 (replace-all with very many matches): option (a), no bound on the number of matches. The cost follows from the declared result type (one range and one replacement string per match, no group slots); the implementer measures the bytes per match on AC-8's input and states the figure in `docs/protocol/search.md`.
- c3 (pattern size): option (a), at most 1,000 chars, refused as `invalid_pattern` with the message AC-9 pins. A time budget is rejected: CHARTER §3 decision 4 allows no wall-clock dependence in L0–L4 behaviour. The value 1,000 has no measurement behind it yet: the pre-dispatch spec check measures the worst-case search time of a 1,000-char pattern on a 1 MB matching text (debug build); above about 5 s it comes back to the director before dispatch.
- c5 (AC-8, the memory test): kept, with its two halves measured in the spec check using a throwaway stub — that a correct implementation completes `search.find` and replace-all under the 512 MiB address-space cap, and that one keeping every match's slots does not. Known so far: the reviewer of T-017 measured about 2.4 kB per match at 100 groups (2.40 GB for 1,000,001 matches), so 400,001 matches is about 0.96 GB by arithmetic; and a director's probe ran an existing `kiln-cli` (`doc.open` and `doc.info` on a 400,000-char file) under `ulimit -v` 524288 and 131072 with exit 0, peak resident size 15,792 kB without a cap. That probe did not exercise any search or replace command.
- c6 (tier and check): medium, as the planner argues (additive API in an L1 crate; no persistence, process, gate path, dependency or CHARTER §3 contract). A pre-dispatch spec check is required all the same, after T-017 has merged, because the task adds public API to that crate.
- c7 (amends c3; decision D-077). The round-2 review of T-017 measured 245 s (debug) for a 100 kB pattern on a 100,000-char text it does NOT match: search time grows with pattern × text whether or not the text matches. The spec check therefore measures the worst case at the 1,000-char cap on a 1 MB text for both a matching and a non-matching adversarial pattern (the class `[ab]` repeated, then `[^ab]`), and any acceptance test of the time bound uses a text longer than the pattern's minimum match length.
Where a ruling differs from text above, the ruling governs; the planner removes the PROPOSED markers when it applies the spec check. `state` stays `draft` until that check returns `ready to dispatch`.
- Director (2026-11-13, D-044): a replacement template that fails to expand is refused as `invalid_pattern`, checked before any matching is done, as proposed.

### Spec check R-158 (planner, 2026-11-13, against main `1dc59d0`; measured on `512b5f3`, whose code differs from `1dc59d0` only in two `doc.rs` DOC strings and tests of T-039)
Verdict: **not ready**; ready to dispatch once Q-1 to Q-3 are ruled and the spec follows the
rulings (it is written for the recommendations, marked PENDING). The R-127 "Choices for the
director" section was removed: c1–c3 above rule it, and the Interface carries the rulings.

**How measured (VERIFIED, debug build, as the gate builds tests).** In a private clone of main
`512b5f3`, never in the primary checkout, each run under
`timeout`: (1) a throwaway `Regex::for_each_match` in `regex.rs`, `find_all`'s loop with one
reused `Captures`; (2) an example `crates/kiln-search/examples/probe_t021.rs` that compiles a
pattern with `Regex::new` and counts matches with `for_each_match` (mode `each`), `find_all`
(`all`), or the engine's own `find_iter` (`engine`, no captures); (3) a throwaway regex-only
`search.find` / `search.replace` in `kiln-commands/src/search.rs` (find keeps `limit` ranges and
counts the rest; replace-all builds `Vec<(CharMatch, String)>` and one `apply_edits`; env
switches select `find_all` instead, or drop the reply's ranges). Commands:
`timeout 900 cargo build -p kiln-search --example probe_t021`, then
`timeout <t> target/debug/examples/probe_t021 <case> each`; and
`timeout 120 cargo build -p kiln-cli`, then
`/usr/bin/time -f '%e s %M KiB' timeout 120 sh -c 'ulimit -v 524288; exec target/debug/kiln-cli run s.jsonl --root root'`
with AC-8's or AC-7's script and file. None of this is committed. Texts: `ab` repeated (1,000,000
chars) unless stated; `a`×1,000,000 and `x`×1,000,000 where named.

**T1. Search time (`for_each_match`, compile time excluded; all compiles took ≤ 0.03 s).**
| Pattern (chars) | Text | Matches | Time |
|---|---|---|---|
| `[ab]`×248 + `[^ab]` (997), the c7 class, non-matching | ab | 0 | 0.029 s |
| same, `case_sensitive:false` | ab | 0 | 0.026 s |
| `(?:a\|b)`×141 + `[^ab]` (alternation) | ab | 0 | 0.025 s |
| `(?:a+)+`×141 + `[^a]` / `(?:a*)*`×141 + `[^a]` (nested quantifiers) | a | 0 | 0.028 / 0.022 s |
| `(?:a\|ab\|aba\|…)` + `[^ab]`, 42 prefixes (alternation of prefixes) | ab | 0 | 0.020 s |
| `(a\|b)`×100 + `[ab]`×123 + `[^ab]` (100 groups) | ab | 0 | 0.031 s |
| `a`×999 + `b` literal (also naive char scan 0.070 s) | a | 0 | 0.120 s |
| `[ab]`×250 (1000) matching | ab | 4,000 | 0.062 s |
| `(a\|b)`×100 + `[ab]`×125 (100 groups) | ab | 4,444 | 0.227 s |
| `()`×100 (200), AC-8's pattern | x | 1,000,001 | **22.6 s** (engine alone 0.67 s) |
| `([ab]*)`×100 (800), one match over the whole text | ab | 1 | **38.2 s** (×10 groups: 4.2 s; ×1: 0.26 s; engine alone 0.047 s) |
| `[ab]{990}[^ab]` (14), long bounded repeat | ab | 0 | 0.107 s |
| `[ab]{2000}[^ab]` / `[ab]{3000}[^ab]` (15) | ab | 0 | 0.39 / 0.83 s |
| `[ab]{4000}[^ab]` (15) | ab | 0 | > 120 s (timeout) |
| `[ab]{5000}[^ab]` (15) | ab | 0 | **544 s** (10,000 chars: 5.9 s; 20,000: 10.9 s) |
| `[ab]{1000}` (10) matching | ab | 1,000 | 0.165 s |
| `[ab]{3000}` (10) matching | ab | 333 | **172 s** (engine alone 170 s) |
| `[ab]{4000}` (10), `(?:[ab]{100}){40}` (17) matching | ab | – | > 200 s (timeout) |
| `\w`×499 + `\W`, `\w`×500 | ab | – | refused at compile (engine size limit, 0.5–0.6 s) |
Worst cases: non-matching, a long counted repeat (`[ab]{5000}[^ab]`, 544 s); matching, a counted
repeat (`[ab]{3000}`, 172 s) and, written out without counting, many groups spanning a long
match (`([ab]*)`×100, 38 s). Everything written out at the cap without counted repetition or
many groups stayed under 0.25 s. The engine's compiled size does not separate fast from slow
(min `size_limit`: `[ab]{3000}` 216,208 B, slow; `.`×1000 1,036,144 B, 0.155 s), so lowering the
engine's size limit is not a time bound either.

**T2. Memory (AC-8's script; peak RSS without cap, then under `ulimit -v 524288`).**
| Variant | No cap | Under 512 MiB |
|---|---|---|
| `doc.open` only | 16,268 KiB | – |
| bounded `search.find {limit:10}` | 17,196 KiB, 8.6 s | ok, 16,548 KiB, 8.5 s |
| unbounded `search.find` (`find_all`) | – | aborts: "memory allocation of 2424 bytes failed", no response |
| bounded replace-all, plan built, before `apply_edits` | 56,968 KiB | – |
| bounded replace-all after `apply_edits` (history) | 104,616 KiB | – |
| whole script, bounded, reply without `ranges` | 104,660 KiB | ok, 104,508 KiB, 23.2 s |
| unbounded replace-all (`find_all`), reply without `ranges` | – | aborts, as above |
| whole script, bounded, reply with all 400,001 `{start,end}` ranges (the drafted result) | 632,344 KiB, 28.6 s | **aborts** ("memory allocation of 632 bytes failed") |
| same with ranges as `[start,end]` pairs | 144,492 KiB | ok, 144,432 KiB, 25.6 s |
So: both halves hold for `search.find`, and the unbounded half holds for replace-all; the
bounded half of replace-all fails as drafted, because the reply's 400,001 range objects cost
about 1.3 kB each on the way out (632,344 − 104,660 KiB over 400,001; INFERRED from these two
runs). What a built implementation will use (its own `Query`, literal path, expansion) cannot be
shown before the task is built; the stub is regex-only and expands only `-` and `$1$1`.

**Blockers and corrections found (all fixed above unless a question is named).**
1. AC-8 as drafted cannot pass with a correct bounded implementation: the `ranges` list of
   400,001 objects alone takes the process over 512 MiB (T2). → Q-2.
2. The 1,000-char cap does not bound search time; c3/c7's condition "above about 5 s comes back
   to the director" is met (T1: 544 s, 172 s, 38 s), and Pinned (e)'s reviewer instruction would
   be an automatic finding. → Q-1.
3. The Rust-API parts of the ACs (AC-7's second half, the `for_each_match` tests) cannot live in
   `kiln-cli`'s tests: `apps/kiln-cli/Cargo.toml` has no `kiln-search` dependency, and adding a
   dev-dependency is a dependency change (high tier, refused by the queue). Moved to
   `crates/kiln-search/tests/acceptance_t021_search.rs` (AC-7, AC-10).
4. Scorecard test names used the stem `acceptance_t021`; `touches` named no acceptance file.
   Now `acceptance_t021_cli.rs` / `acceptance_t021_search.rs`, names `kiln_cli::acceptance_t021_cli::…`.
5. The `invalid_pattern` `when` text on main ("the search pattern is empty or is not a valid
   regular expression", `reason.rs`) is narrower than this task's refusals (a 1,001-char
   literal pattern, an invalid replacement template); `reason.rs` was not in `touches`, and
   T-038 leaves the `when` text of a kind to "their task". → Q-3.
6. Stale: the reading list said T-017 is not merged (it is; its note on `expand`'s cost is now
   the code at `regex.rs:174–193`); `budget_tokens` 500000 → 700000; `iteration` 1 → 2; the
   `pub use query::…` line sat in the `query.rs` block (it is `lib.rs`); the common params did
   not name `doc`; nothing said optional params refuse `null` (D-084 x1), that `find` accepts
   `limit` 0 (T-022 calls `find(text, 0)`), that the matches always form a valid
   `apply_edits` batch, or how the selection moves (now `cursor.md`'s mapping rule, AC-4,
   and a two-line edit of `cursor.md` in `touches`).
7. AC-8's 30 s timeout was 21–26 s of measured debug work: raised to 120 s (a hang bound, as
   T-018's tests use).
8. D-091 change 5: Pinned (i) added (S1–S7). Change 6: Pinned (f). Change 18: not applicable,
   no CLI option or exit code changes; `docs/control-protocol.md` is not touched.
9. `docs/commands.json` is a generated path: moved from `touches` to a comment, as T-038 and
   T-039 do, so it is not a declared overlap with T-019, T-020, T-022, T-023 (METHODOLOGY §5.2).
   Scorecard item files are listed for the director's join edit.

**Questions for the director (the spec is written for each recommendation, marked PENDING).**
- Q-1 (planner, R-158; amends c3/c7). The cap does not bound search time (T1). Options:
  (a) **promise no time bound, document it with the measured figures** (S4), keep the
  1,000-char and 100-group caps as limits on pattern text and slots, no time-bound AC — as
  T-017 s8 did for the crate; consequence: a hostile or careless pattern can hold a session for
  minutes, and T-022 inherits it once per file; (b) also refuse counted repetitions above a
  bound — needs a parse of the pattern: a hand-written scanner in `kiln-search` (fragile: nested
  `{n}`, classes, escapes) or `regex-syntax` as a direct dependency (high tier, approval row);
  it would not cover `([ab]*)`×100 (38 s) or `()`×100 (22.6 s), which need a lower group cap or
  (f); (c) a lower char cap — pointless for time: the slowest probes are 10–15 chars; (d) a step
  budget — the `regex` crate has none; another engine is a dependency change and its own
  task; (e) a time budget — ruled out (CHARTER §3 decision 4, c3); (f) additionally, a
  whole-match-only primitive for `search.find` and reference-free templates (engine alone:
  38.2 s → 0.047 s, 22.6 s → 0.67 s) — new public API whose empty-match iteration must equal
  `find_all`'s group 0; it does not help the counted-repeat cases (170 s engine alone).
  Recommendation: (a) in this task; if the owner wants a bound, a separate task for (b)+(f).
- Q-2 (planner, R-158). The reply of `search.replace {all:true}` (T2). Options: (a) **a `limit?`
  param as in `search.find` (1..=100000, default 10000): `ranges` lists the first `limit`,
  `truncated = replaced > limit`, every match replaced** — mirrors `search.find` and T-022's c4
  preview; AC-8 then passes (≈ 105 MB + 10,000 ranges); the evaluator sees all ranges only when
  `replaced ≤ limit`; (b) all ranges as `[start, end]` pairs — 144 MB measured, passes, but
  differs from `{start,end}` everywhere else (and from `search.find`) and still grows without
  bound; (c) keep all `{start,end}` ranges and drop the replace half of AC-8's memory test,
  documenting about 1.3 kB per replacement in the reply — replace-all of 400,000 matches then
  needs about 630 MB; (d) no `ranges` for `all: true`. Recommendation: (a).
- Q-3 (planner, R-158). The `invalid_pattern` `when` text. Options: (a) this task edits that
  one row of `reason.rs` to "the search pattern is empty, longer than 1,000 chars or not a
  valid regular expression (more than 100 capture groups included), or a replacement template
  names a capture group the pattern does not have" — then `reason.rs` is in `touches`, which
  T-020 also declares (its `TooLarge` row), so T-020 and T-021 cannot be in flight together
  (METHODOLOGY §5.2), against D-092's order; (b) **this task leaves `reason.rs` alone; the
  director adds that `when` text to T-038's §4 list** (T-038 runs after the stage-1 feature
  tasks and declares their refusals anyway); until T-038 merges the published text is
  narrower than the binary, a known gap the reviewer records as a follow-up, not a defect;
  `docs/protocol/search.md` states every case. Recommendation: (b), to keep D-092's parallel
  order.

**Split (D-091, METHODOLOGY §5.7).** The split rule binds high-tier specs; this one is medium.
The four items are not independent: whole-word is a filter on the same search, and replace runs
on the same `Query`, pass and document. Split into find (literal, regex, whole-word) and
replace: + a second spec, check, review round and join (about one medium task's review cost),
both editing `query.rs`, `search.rs` and `search.md`, so serial; − it would let find ship while
Q-2 is open. Keep it one task: Q-2 is small and the shared parts dominate.

**Parallel work.** With T-020 (in progress, its final `touches` read): yes, under Q-3 (b) —
the declared `touches` are disjoint (`docs/commands.json` is generated; the queue regenerates
it on conflict); not under Q-3 (a) (T-020 declares `reason.rs`). With T-019 (in progress): yes (disjoint; T-019's `Cargo.lock` is generated and this
task changes no dependency). T-039 is merged (`c538ef8`); this task touches none of its files. With T-038: not
in flight together (its `reason.rs`, and it must see `search.*` registered, D-092). With T-022:
after this task (`depends_on`). T-019 and T-020 are in progress now (D-097, D-100), so this task is the third
builder of the cap of three compiling at once (§5.4).
### Rulings on Q-1 to Q-3 of the spec check R-158 (director, D-101)
- Q-1: (a). No time bound is promised. The caps of 1,000 pattern chars and 100 groups stay; they bound what they bound and the documents say that search time is not bounded by them. Every figure that goes into a document is measured by the implementer through `kiln-cli` on a release build, with the command recorded; the spec check's figures are from a debug build of a throwaway stub and are evidence for this spec only. No test asserts a time; a timeout in a test is a hang detector and is labelled so. A real bound (refusing large counted repeats, a whole-match-only search) is a follow-up task the director files; this task does not attempt it.
- Q-2: (a). `search.replace` with `all: true` takes a `limit` like `search.find`'s (default 10,000) that caps the `ranges` list in the reply, and reports `truncated`; every match is still replaced and `total` is exact.
- Q-3: (b). This task does not touch `reason.rs`. The published `when` text of `invalid_pattern` stays narrower than the binary (it does not mention a pattern over the cap or a bad replacement template) until T-038 merges; T-038's list of `when` texts gains this one. The gap is known and recorded here; the reviewer does not raise it as a finding against this task.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13 · R-056 · (new) · did: specified in-buffer search and replace on T-017's engine
  and T-018's batch edit. Whole-word is defined as a filter of the ordinary result, so the
  naive model is simple. Case-insensitive literal uses the same folding as the regex engine.
  Tier medium (no ADR-0002 high criterion). Arm odd, paired against the even T-018 (both large
  in-memory, oracle-rich). Every worked example was executed (Python model; regex rows on regex
  1.13.1). Pinned (b), (e), (f) n/a. · gate attempts: 0 · missing context: none · next: director.
- 2026-11-13 · R-066 · draft · did: `invalid_pattern` is T-032's (D-041); `depends_on` gains T-032; `QueryError` is mapped in `search.rs` (finding 12). · gate attempts: 0 · missing context: none · next: director.
- 2026-11-13 · R-079 · draft · did: followed T-017's revised contract (R-077): `expand` returns
  `Result<String, TemplateError>`, so `replacement_for` does too; refusal for a bad template
  PROPOSED (`invalid_pattern`, checked before matching); AC-2 takes T-017's generator
  restriction (spec check R-067 note 14); AC-4 uses T-017's expansion rule; inline flags and
  the expansion difference go into `docs/protocol/search.md`. New (g) example executed against
  a naive Python model of T-017's rule. · gate attempts: 0 · missing context: none · next:
  director.
  QUESTION: a replacement naming a non-existent group: refuse with `invalid_pattern` and
  message `invalid replacement: …` (proposed; no new kind), or a new kind through T-032? And
  is it checked before matching (proposed), so a bad template is refused even with no match?
- 2026-11-13T12:55Z · R-112 · draft · did: checked against T-017's spec (not on main yet): `expand`
  and `replacement_for` already return `Result<_, TemplateError>` here; fixed stale lines (kinds
  are T-032's, the template refusal is ruled by D-044, not PROPOSED); added ruling s1's
  101-group refusal with its exact message (AC-2) and s2's unpinned crate messages; took the
  T-017 implementer's note (`expand` walks the text from its start per reference, so
  replace-all would be O(text × matches)) and chose to bound it: new
  `CompiledQuery::replacements` with a linear cost contract, used by `search.replace` and by
  T-022; AC-7 checks it with a coarse 20-second timeout on 200,000 matches (quadratic ≈ 8·10^10
  steps). · gate attempts: 0 · not run: no cargo; AC-7's step count is arithmetic, not
  measured · missing context: T-017's merged code · next: director.
  QUESTION: is a coarse timeout acceptable as AC-7's oracle (D-028 allows timeouts for hangs),
  or should the bound be stated as a limit only?
- 2026-11-13T13:56Z · R-127 · draft · did: bounded search per D-070 s4/s5 (T-017 review r1
  findings 2–3; `find_all` keeps every match's slots, 2.40 GB measured; a 100 kB pattern takes
  ≥ 30 s on matching text). Added `crates/kiln-search/src/regex.rs` to `touches` for one
  additive primitive, `Regex::for_each_match` (T-017's frozen API unchanged); replaced the
  draft's `CompiledQuery::find_all`/`replacements` by `find(text, limit)`, `check_template`,
  `replace_first`, `replace_all` with memory and time contracts; struck the false claim in
  Pinned (e); added the 1,000-char pattern cap (`QueryError::TooLong`, exact message); AC-8
  (512 MiB `ulimit -v` cap, 400 kB text, 100 empty groups: an unbounded implementation needs
  ≈ 0.96 GB and aborts) and AC-9 (1,000/1,001 chars). Three user-visible choices are written
  as PROPOSED with options and a recommendation (exact `total`; replace-all unbounded but
  slot-free; pattern ≤ 1,000 chars). Fixed a precedence contradiction (Refusals vs Pinned
  (a)). Tier: medium (ADR-0002): the new public API is additive in an L1 crate, which is not a
  CHARTER §3 contract (as adding a command is not), and there is no dependency, persistence,
  process, gate path or security boundary. Needs a pre-dispatch spec check: the interface and
  every bound are new since the last check, and it builds on T-017 code that is not merged. ·
  gate attempts: 0 · not run: no cargo; AC-8's figures are arithmetic from D-070's 2.4 kB per
  match, not measured; `ulimit -v` behaviour of the debug `kiln-cli` not tried · missing
  context: T-017 merged · next: director decides choices 1–3; spec check.
- 2026-11-13T22:20Z · R-158 · draft · did: spec check: read against main `1dc59d0` (T-015,
  T-017, T-018, T-032 and also T-039, T-042 merged; T-019, T-020 in progress); measured in a
  private clone with throwaway stubs (`Regex::for_each_match`, a regex-only `search.*`), debug
  builds, commands and figures in "Spec check R-158". Found 9 blockers or corrections: AC-8's
  replace half cannot pass as drafted (400,001 `{start,end}` reply ranges: 632,344 KiB peak,
  aborts under 512 MiB; without them 104,508 KiB, passes; `find` bounded 16,548 KiB passes,
  `find_all` aborts); the 1,000-char cap does not bound search time (`[ab]{5000}[^ab]`, 15
  chars, 544 s non-matching; `[ab]{3000}` 172 s matching; `([ab]*)`×100 38.2 s; the c7 class
  pattern itself 0.029 s); Rust-API ACs cannot live in `kiln-cli`'s tests (no `kiln-search`
  dependency); test-file stems and scorecard names; `invalid_pattern`'s `when` text narrower
  than the refusals; stale reading list, budget, iteration, `lib.rs` re-export, `doc` param,
  `null` params (D-084 x1), `find(text, 0)` for T-022, selection mapping (now `cursor.md` in
  `touches`); AC-8 timeout 30 → 120 s (21–26 s measured). Applied D-091 changes 5 (Pinned (i),
  S1–S7), 6 (Pinned (f)), 18 (n/a, stated). Removed the R-127 choices section (ruled by D-071).
  Split: keep one task (medium; items share query, pass and document). Tier medium unchanged
  (c6). Pinned (b) n/a: no files; (e) threat model n/a, bounds per Q-1. · gate attempts: 0 ·
  not run: `cargo xtask ci`; any test of the real implementation (not built); release-build
  timings; literal-mode and expansion costs of a real `Query` (the stub is regex-only) · missing
  context: none · next: director rules Q-1 to Q-3; then the spec is ready to dispatch. Did not
  write a `tasks/spec-checks/T-021.md` file (this run's brief allowed edits to this file only).
  QUESTION: Q-1 (time bound: recommend (a) document no bound), Q-2 (replace reply: recommend
  (a) `limit` on `ranges`), Q-3 (`invalid_pattern` `when` text: recommend (b) via T-038).
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
