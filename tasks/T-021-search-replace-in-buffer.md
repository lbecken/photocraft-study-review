---
id: T-021
title: "Search an open document by literal or regex, case-sensitive or not, optionally whole-word, and replace one or all matches as one undo step with capture groups"
tier: medium            # ADR-0002 default: in-memory behaviour; adds public API to kiln-search (not a CHARTER §3 contract, no dependency); the regex dependency was added by T-017
state: draft
owner_role: planner
scorecard_items: [search.literal, search.regex, search.whole-word, search.replace]
depends_on: [T-015, T-017, T-018, T-032]   # module slot; Regex contract; apply_edits; invalid_pattern
touches:
  - crates/kiln-search/src/lib.rs         # re-exports
  - crates/kiln-search/src/regex.rs       # R-127: additive only, `Regex::for_each_match` (D-070 s4); T-017's items unchanged
  - crates/kiln-search/src/query.rs       # new: Query, literal/whole-word matching, replacement planning
  - crates/kiln-search/tests/query.rs     # implementer's own integration tests
  - crates/kiln-commands/src/search.rs    # search.* (slot from T-015)
  - docs/protocol/search.md               # new
  - docs/commands.json                    # generated
budget_tokens: 500000   # medium (v1.1, provisional)
iteration: 1
found_by: planner
origin_task: null
---

# T-021 Search and replace in an open document

## Goal
After this task `search.find` lists every match of a query in an open document, as char ranges.
The query is literal or regex, case-sensitive or not, and optionally restricted to whole words.
`search.replace` replaces the first match at or after an offset, or all matches, as a single
undoable step. In regex mode the replacement can use capture groups. `kiln-search` gains the
`Query` type that find-in-files (T-022) reuses.

No gate path. Arm: **odd**. The implementer writes the acceptance tests on `task/T-021-tests`
first, through the public interface, then the code (METHODOLOGY §6).

Reading list: `tasks/T-017-*.md` Semantics (iteration, anchors, case folding, expansion);
`tasks/T-017-*.md` rulings s1 (capture-group cap) and s2 (error messages) and its implementer
note on `expand`'s cost (on `task/T-017`; T-017 is not merged yet, so read the spec, not main);
`tasks/T-018-*.md` (`apply_edits`, mapping rule); `crates/kiln-commands/src/reason.rs`
(reason `invalid_pattern`, declared by T-032).

## Interface

### `kiln_search` (L1)
`find_all` (T-017) returns every match with `group_count() + 1` slots, about 2.4 kB per match at
100 groups; 100 empty groups on a 1 MB text peaked at 2.40 GB (T-017 review r1 finding 2,
D-070 s4). The 100-group cap does not bound that, and T-017's API is frozen. This task
therefore adds one streaming primitive to `Regex` and builds every command on it; no command
of this task (or T-022) calls `find_all`.
```rust
// crates/kiln-search/src/regex.rs: additive; `new`, `group_count`, `find_all`, `expand` unchanged
impl Regex {
    /// Calls `f` once per match, in `find_all`'s order, with a `Captures` equal to the one
    /// `find_all(text)` holds at that position; stops after a call that returns
    /// `ControlFlow::Break(())`. Memory beyond the engine is O(group_count()), whatever the
    /// number of matches: the value passed to `f` is reused, so `f` copies what it keeps.
    pub fn for_each_match(&self, text: &str, f: &mut dyn FnMut(&Captures) -> std::ops::ControlFlow<()>);
}

// crates/kiln-search/src/query.rs
pub use query::{CompiledQuery, Found, Query, QueryError, MAX_PATTERN_CHARS};
pub const MAX_PATTERN_CHARS: usize = 1000;                    // PROPOSED, choice 3 below
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Query { pub pattern: String, pub regex: bool, pub case_sensitive: bool, pub whole_word: bool }
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueryError { Empty, TooLong { chars: usize }, Pattern(PatternError) }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found { pub matches: Vec<CharMatch>, pub total: usize }
impl Query {
    /// `Empty` → `TooLong` (pattern longer than `MAX_PATTERN_CHARS` chars, both modes,
    /// checked before compiling) → `Pattern`.
    pub fn compile(&self) -> Result<CompiledQuery, QueryError>;
}
impl CompiledQuery {
    /// The whole-match ranges of the first `limit` matches (whole-word filter applied) and
    /// `total`, the exact number of all matches (PROPOSED, choice 1 below).
    /// Memory: O(limit + group_count()). Time: one pass over the text.
    pub fn find(&self, text: &str, limit: usize) -> Found;
    /// `Ok(())` iff `replacement` expands for this pattern (literal mode: always). Depends on
    /// the pattern and the template only (T-017 expansion rule).
    pub fn check_template(&self, replacement: &str) -> Result<(), TemplateError>;
    /// The replacement text for one match (as before): literal mode → `Ok(replacement)`;
    /// regex mode → T-017's `Regex::expand`. One call walks `text` from its start per
    /// reference (T-017 implementer note), so it is for single matches only.
    pub fn replacement_for(&self, caps: &Captures, text: &str, replacement: &str)
        -> Result<String, TemplateError>;
    /// The first match (iteration from char 0) whose start ≥ `from`, with its replacement
    /// text. Memory: O(group_count() + replacement text). Stops at that match.
    pub fn replace_first(&self, text: &str, from: usize, replacement: &str)
        -> Result<Option<(CharMatch, String)>, TemplateError>;
    /// Every match's whole range with its replacement text, in order.
    /// Memory: O(matches + total replacement text), never O(matches × groups).
    /// Time: O(len(text) + Σ over matches of (match length × references in `replacement`)),
    /// never O(len(text) × matches). PROPOSED, choice 2 below: no bound on the number of
    /// matches.
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
  matches only `\r\n`.
- Regex: T-017's `Regex` with `case_insensitive = !case_sensitive`, with T-017's iteration.
  Inline flags in the pattern (`(?i)`, `(?-i)`, `(?s)`, …) are accepted and override
  `case_sensitive` and the line modes for their scope, as T-017 pins.
- Whole word (both modes): first the matches the search reports without whole-word, then keep
  those with a word boundary at both ends. Word char = `c == '_' || c.is_alphanumeric()`. A
  match `[s, e)` has boundaries iff (`s == 0` or char `s−1` is not a word char) and
  (`e == len` or char `e` is not a word char). Nothing is re-scanned after a rejected match.

### Commands (`doc` must be open)
Common query params: `query` (string), `regex?` (default false), `case_sensitive?` (default
true), `whole_word?` (default false).

| Command | Mutating | Extra params | Result |
|---|---|---|---|
| `search.find` | no | `limit?` integer 1..=100000 (default 10000) | `{matches:[{start,end}], total, truncated}` |
| `search.replace` | yes | `replacement` string; `all?` bool (default false); `from?` offset (default 0) | `{replaced, len_chars, ranges:[{start,end}]}` |

- `search.find`: `CompiledQuery::find(text, limit)`. `total` is the exact number of all
  matches (choice 1); `matches` holds the first `limit`; `truncated = total > limit`.
- `search.replace` with `all: false` replaces the first match whose `start ≥ from`; with
  `all: true` it replaces every match. `from` is ignored then, and there is no wrap-around.
  `all: false` uses `replace_first`, `all: true` uses `replace_all`.
  All replacements are one `Buffer::apply_edits` call, so one `doc.undo` reverts them all.
  `ranges` are the inserted texts' ranges in the new text. With no match, `replaced: 0`
  and nothing changes: no undo step, the typing group is not ended.
- Refusals, after schema and `no_such_document` (Pinned (a)): `QueryError::Empty`, `TooLong`
  and `Pattern` → `rejected`/`invalid_pattern`. The message is `empty search pattern`, or
  `invalid pattern: the pattern has <chars> chars; the limit is 1000` (`TooLong`, both modes),
  or `invalid pattern: <PatternError message>`; for a pattern with
  more than 100 capture groups (T-017 ruling s1) that is exactly
  `invalid pattern: the pattern has more than 100 capture groups`; 100 groups are accepted.
  Other `PatternError` messages are the regex crate's (T-017 s2) and are not pinned here. Then
  `from > len_chars` → `out_of_range`. All kinds exist (T-032's reason table); no new kind.
- Replacement naming a group that does not exist (regex mode; T-017 `TemplateError`): whether a
  template is an error depends only on the pattern and the template, so `search.replace`
  checks it (`check_template`) before matching, whether or not there is a match, and refuses with
  `rejected`/`invalid_pattern`, message `invalid replacement: <TemplateError Display>` (e.g.
  `invalid replacement: no capture group "3"`). Nothing changes. (Ruled by D-044.)
- Bounds (D-070 s4/s5). Memory: `search.find` holds at most `limit` ranges plus one match's
  slots; `search.replace` holds one match's slots (`all: false`) or one range and one
  replacement string per match (`all: true`), never the group slots of more than the current
  match. Time: every search is one pass over the text, whose worst case the engine bounds by
  text length × pattern size; the pattern size is capped at 1,000 chars (choice 3). No wall
  clock is used (CHARTER §3 decision 4). AC-7, AC-8 and AC-9 check the bounds cheaply.

### Choices for the director (PROPOSED; the spec above is written with the recommendation)
1. `total` when the limit is hit. (a) **Exact**: count every match in the same pass without
   keeping it — memory O(limit), time one full pass; `total`/`truncated` keep their meaning
   and AC-6 stays. (b) **At least**: stop after `limit + 1` matches and report `total` as a
   lower bound (a new `total_at_least` flag) — faster on huge match counts, but the result
   schema changes and every client and T-022 must handle an inexact `total`.
   Recommendation: (a); memory is solved by not keeping matches, and the time is the same one
   pass a replace-all already pays.
2. Replace-all on a text with very many matches. (a) **No bound, documented cost**: memory
   O(matches + replacement text), no group slots, so "replace every `x` in a large file"
   works. (b) **Refuse above N matches** (e.g. 100,000) with a new reason (e.g.
   `too_many_matches`) — a hard ceiling, but a new kind, a counting pass first, and a refusal
   users must work around. Recommendation: (a); without group slots the cost is tens of bytes
   per match, and AC-8 holds it under a 512 MiB address-space cap.
3. Pattern size or search time. (a) **Pattern ≤ 1,000 chars**, refused as `invalid_pattern`
   (`TooLong`) — bounds the pattern factor of the engine's text × pattern worst case; a
   100 kB pattern (≥ 30 s on a matching text, s5) cannot be sent. (b) ≤ 10,000 chars — larger
   pasted snippets work, worst case about ten times (a). (c) A time budget per search — needs a
   clock in L1–L3, which CHARTER §3 decision 4 forbids; rejected. Recommendation: (a), for
   both modes so the rule is one line.

`docs/protocol/search.md` documents the commands and all rules above, with the examples,
including T-017's expansion rule (a reference to a group that does not exist is refused, unlike
the `regex` crate's own expansion) and that inline flags override the options.

## Pinned
- (a) Precedence: schema → `no_such_document` → `invalid_pattern` (the pattern: empty, too
  long, not compiling or more than 100 groups; then the replacement template) →
  `out_of_range`. (R-127: the Refusals bullet used to list `invalid_pattern` before
  `no_such_document`; this order governs.)
- (b) Files: none.
- (c) Order: matches are computed on the document's current text before any replacement;
  each replacement text is computed from its match's captures as the match is found.
- (d) Offsets are chars. Platform independent.
- (e) Threat model: n/a (no security boundary). Resource bounds come from this task, not from
  T-017: the 100-group cap bounds only the engine's working memory (D-070 s4). Memory is
  bounded by never keeping more than one match's slots (Interface, "Bounds"); the pattern
  factor of search time by the 1,000-char cap (choice 3). Search time stays linear in the text
  for a given pattern; the reviewer measures a 1,000-char pattern on a 1 MB matching text and
  reports it (above about 5 s in a debug build is a finding).
- (f) Snapshots: none.
- (g) Worked examples (executed against a naive Python model; regex rows against regex 1.13.1):
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
  `search.replace`, `doc.undo`, `doc.text`.

## Acceptance criteria
All through `kiln-cli run` or `kiln_control::Host`.
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
  `can_undo` unchanged. Oracle: naive model + round trip.
- AC-5 (worked examples): every example in Pinned (g) holds exactly. Oracle: exact value.
- AC-6 (limit): a text with 20,001 matches and `limit: 10` gives 10 matches, `total: 20001`,
  `truncated: true`. Oracle: exact value.
- AC-7 (`search.replace`, bounded cost): a document of `ab` repeated 200,000 times (400,000
  chars), regex `(a)b`, replacement `$1$1`, `all: true` → `replaced: 200000` and `doc.text`
  equal to `aa` repeated 200,000 times, under a 20-second timeout on the whole `kiln-cli run`
  (a hang-style bound, not a measurement: one walk from the text's start per reference is
  about 8·10^10 char steps here, far beyond it; the linear cost is well under a second). And
  through the Rust API, `replace_all` equals `replacement_for` applied to each match of
  `find_all` on the AC-4 inputs. Oracle: exact value + coarse timeout.
- AC-8 (bounded memory; cheap: under 1 MB of input, and a correct implementation allocates
  little): `kiln-cli run` is started through `sh -c 'ulimit -v 524288; exec <kiln-cli> run
  s.jsonl --root <dir>'` (a 512 MiB address-space cap; Unix, D-014) under a 30-second timeout.
  The document is `x` repeated 400,000 times; the regex is `()` repeated 100 times (100 empty
  groups, 200 chars). `search.find {limit:10}` → matches `0..0` … `9..9`, `total: 400001`,
  `truncated: true`; `search.replace {replacement:"-", all:true}` → `replaced: 400001`,
  `len_chars: 800001`; `doc.undo` restores the text. Every line `ok`, exit 0. An
  implementation that keeps every match's slots needs about 400,001 × 2.4 kB ≈ 0.96 GB and
  aborts under the cap: the test detects it without using the memory. Oracle: exact value.
- AC-9 (pattern cap): in both modes a 1,001-char pattern → `rejected`/`invalid_pattern` with
  message `invalid pattern: the pattern has 1001 chars; the limit is 1000`; a 1,000-char
  pattern (`a` repeated) is accepted. Oracle: exact value.

Every AC is an **acceptance** test (on base no `search.*` command exists). The implementer's
own tests (not acceptance) show that `for_each_match` without a break yields exactly
`find_all(text)` element by element on the AC-2 inputs, and that a break after k calls
yields its first k.

## Scorecard items and intended test names
| Item | Tests |
|---|---|
| search.literal | `kiln_cli::acceptance_t021::literal_find_matches_naive_scan` |
| search.regex | `kiln_cli::acceptance_t021::regex_find_matches_naive_backtracker`, `kiln_cli::acceptance_t021::invalid_patterns_are_reported`, `kiln_cli::acceptance_t021::pattern_length_is_capped` |
| search.whole-word | `kiln_cli::acceptance_t021::whole_word_filters_at_identifier_boundaries` |
| search.replace | `kiln_cli::acceptance_t021::replace_all_matches_naive_model_and_undoes_in_one_step`, `kiln_cli::acceptance_t021::replace_one_from_offset`, `kiln_cli::acceptance_t021::replace_all_is_linear`, `kiln_cli::acceptance_t021::search_memory_is_bounded` |

## Out of scope and implementer notes
- Search across files (T-022); find-next with selection or wrap-around (UI, iteration 2);
  multi-line literal patterns beyond exact char equality; incremental search.
- Your own tests: `crates/kiln-search/tests/query.rs`, unit tests. Acceptance tests (odd arm)
  go on `task/T-021-tests`, committed and failing on base before you write the code.

## Questions and rulings
Director rulings on the planner's choices (2026-11-13, decision D-071; planner run R-127), made after reading the choices section, AC-8, AC-9 and the `replace_all` signature above, and CHARTER §3 decision 4:
- c1 (`total` when the limit is hit): option (a). `total` stays the exact number of all matches, counted in the same pass without keeping the matches; `truncated = total > limit`. No schema change.
- c2 (replace-all with very many matches): option (a), no bound on the number of matches. The cost follows from the declared result type (one range and one replacement string per match, no group slots); the implementer measures the bytes per match on AC-8's input and states the figure in `docs/protocol/search.md`.
- c3 (pattern size): option (a), at most 1,000 chars, refused as `invalid_pattern` with the message AC-9 pins. A time budget is rejected: CHARTER §3 decision 4 allows no wall-clock dependence in L0–L4 behaviour. The value 1,000 has no measurement behind it yet: the pre-dispatch spec check measures the worst-case search time of a 1,000-char pattern on a 1 MB matching text (debug build); above about 5 s it comes back to the director before dispatch.
- c5 (AC-8, the memory test): kept, with its two halves measured in the spec check using a throwaway stub — that a correct implementation completes `search.find` and replace-all under the 512 MiB address-space cap, and that one keeping every match's slots does not. Known so far: the reviewer of T-017 measured about 2.4 kB per match at 100 groups (2.40 GB for 1,000,001 matches), so 400,001 matches is about 0.96 GB by arithmetic; and a director's probe ran an existing `kiln-cli` (`doc.open` and `doc.info` on a 400,000-char file) under `ulimit -v` 524288 and 131072 with exit 0, peak resident size 15,792 kB without a cap. That probe did not exercise any search or replace command.
- c6 (tier and check): medium, as the planner argues (additive API in an L1 crate; no persistence, process, gate path, dependency or CHARTER §3 contract). A pre-dispatch spec check is required all the same, after T-017 has merged, because the task adds public API to that crate.
Where a ruling differs from text above, the ruling governs; the planner removes the PROPOSED markers when it applies the spec check. `state` stays `draft` until that check returns `ready to dispatch`.
- Director (2026-11-13, D-044): a replacement template that fails to expand is refused as `invalid_pattern`, checked before any matching is done, as proposed.

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
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
