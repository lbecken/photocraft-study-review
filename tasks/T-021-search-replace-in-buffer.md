---
id: T-021
title: "Search an open document by literal or regex, case-sensitive or not, optionally whole-word, and replace one or all matches as one undo step with capture groups"
tier: medium            # ADR-0002 default: in-memory; the regex dependency was added by T-017
state: draft
owner_role: planner
scorecard_items: [search.literal, search.regex, search.whole-word, search.replace]
depends_on: [T-015, T-017, T-018]   # module slot; Regex contract; Buffer::apply_edits
touches:
  - crates/kiln-search/src/lib.rs         # re-exports
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
`tasks/T-018-*.md` (`apply_edits`, mapping rule); `tasks/T-015-*.md` (reason `invalid_pattern`).

## Interface

### `kiln_search` (L1)
```rust
pub use query::{Query, QueryError};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Query { pub pattern: String, pub regex: bool, pub case_sensitive: bool, pub whole_word: bool }
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueryError { Empty, Pattern(PatternError) }
impl Query {
    pub fn compile(&self) -> Result<CompiledQuery, QueryError>;
}
impl CompiledQuery {
    pub fn find_all(&self, text: &str) -> Vec<Captures>;   // T-017's Captures; literal: one group (0)
    /// The replacement text for one match: literal mode → `replacement` verbatim;
    /// regex mode → T-017 expansion.
    pub fn replacement_for(&self, caps: &Captures, text: &str, replacement: &str) -> String;
}
```

### Matching rules (exact)
- Empty `pattern` → `QueryError::Empty` in both modes.
- Literal: occurrences of `pattern` as a char sequence, scanned left to right, non-overlapping
  (after a match the scan resumes at its end). With `case_sensitive: false`, chars compare by
  Unicode simple case folding, as T-017's case-insensitive mode does. `\r\n` in the pattern
  matches only `\r\n`.
- Regex: T-017's `Regex` with `case_insensitive = !case_sensitive`, with T-017's iteration.
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

- `search.find`: `total` is the number of all matches; `matches` holds the first `limit`;
  `truncated = total > limit`.
- `search.replace` with `all: false` replaces the first match whose `start ≥ from`; with
  `all: true` it replaces every match. `from` is ignored then, and there is no wrap-around.
  All replacements are one `Buffer::apply_edits` call, so one `doc.undo` reverts them all.
  `ranges` are the inserted texts' ranges in the new text. With no match, `replaced: 0`
  and nothing changes: no undo step, the typing group is not ended.
- Refusals: `QueryError::Empty` and `Pattern` → `rejected`/`invalid_pattern`. The message is
  `empty search pattern`, or `invalid pattern: <PatternError message>`. Then `no_such_document`,
  and `from > len_chars` → `out_of_range`. All kinds exist (T-015); no new kind.

`docs/protocol/search.md` documents the commands and all rules above, with the examples.

## Pinned
- (a) Precedence: schema → `no_such_document` → `invalid_pattern` → `out_of_range`.
- (b) Files: none.
- (c) Order: matches are computed on the document's current text before any replacement;
  replacement texts are computed from those captures.
- (d) Offsets are chars. Platform independent.
- (e) Threat model: n/a (no boundary). Pathological patterns are bounded by T-017's engine
  guarantee.
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
- (h) Evaluator calls: `kiln-cli run` with `doc.open`, `search.find` (each option combination),
  `search.replace`, `doc.undo`, `doc.text`.

## Acceptance criteria
All through `kiln-cli run` or `kiln_control::Host`.
- AC-1 (`search.literal`): for ≥ 5,000 fixed-seed random (text, pattern) pairs over `a b A B _ 0
  é É σ Σ ς ſ s S K k \r \n` and space, `search.find` (literal, both case settings) equals a
  naive scan. The naive scan compares chars with T-017's test predicate ("equal, or equal
  lowercase, or equal uppercase") when case-insensitive. Oracle: naive model.
- AC-2 (`search.regex`): for ≥ 5,000 random pairs in T-017's regex subset, `search.find` equals
  a naive backtracking model, written in the test, of T-017's iteration rules. `(`, `[a-`,
  `a{99999999}` and the empty pattern give `rejected`/`invalid_pattern`, and the session is
  unchanged. Oracle: naive model + exact value.
- AC-3 (`search.whole-word`): for the AC-1 and AC-2 inputs, `whole_word: true` equals the naive
  filter of the non-whole-word result. The worked examples hold. Oracle: naive model.
- AC-4 (`search.replace`): for random texts, queries and replacement templates, `all: true`
  gives `doc.text` equal to the naive replacement, and `replaced` equals the match count.
  `all: false, from: f` replaces exactly the first match with `start ≥ f`. One `doc.undo`
  restores the original text exactly, and `doc.redo` re-applies. `replaced: 0` leaves
  `can_undo` unchanged. Oracle: naive model + round trip.
- AC-5 (worked examples): every example in Pinned (g) holds exactly. Oracle: exact value.
- AC-6 (limit): a text with 20,001 matches and `limit: 10` gives 10 matches, `total: 20001`,
  `truncated: true`. Oracle: exact value.

## Scorecard items and intended test names
| Item | Tests |
|---|---|
| search.literal | `kiln_cli::acceptance_t021::literal_find_matches_naive_scan` |
| search.regex | `kiln_cli::acceptance_t021::regex_find_matches_naive_backtracker`, `kiln_cli::acceptance_t021::invalid_patterns_are_reported` |
| search.whole-word | `kiln_cli::acceptance_t021::whole_word_filters_at_identifier_boundaries` |
| search.replace | `kiln_cli::acceptance_t021::replace_all_matches_naive_model_and_undoes_in_one_step`, `kiln_cli::acceptance_t021::replace_one_from_offset` |

## Out of scope and implementer notes
- Search across files (T-022); find-next with selection or wrap-around (UI, iteration 2);
  multi-line literal patterns beyond exact char equality; incremental search.
- Your own tests: `crates/kiln-search/tests/query.rs`, unit tests. Acceptance tests (odd arm)
  go on `task/T-021-tests`, committed and failing on base before you write the code.

## Questions and rulings

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
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
