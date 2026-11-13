---
id: T-021
title: "Search an open document by literal or regex, case-sensitive or not, optionally whole-word, and replace one or all matches as one undo step with capture groups"
tier: medium            # ADR-0002 default: in-memory; the regex dependency was added by T-017
state: draft
owner_role: planner
scorecard_items: [search.literal, search.regex, search.whole-word, search.replace]
depends_on: [T-015, T-017, T-018, T-032]   # module slot; Regex contract; apply_edits; invalid_pattern
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
`tasks/T-017-*.md` rulings s1 (capture-group cap) and s2 (error messages) and its implementer
note on `expand`'s cost (on `task/T-017`; T-017 is not merged yet, so read the spec, not main);
`tasks/T-018-*.md` (`apply_edits`, mapping rule); `crates/kiln-commands/src/reason.rs`
(reason `invalid_pattern`, declared by T-032).

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
    /// The replacement text for one match: literal mode → `Ok(replacement)` verbatim;
    /// regex mode → T-017's `Regex::expand`, whose `Err(TemplateError)` is passed through.
    /// Cost: T-017's `expand` maps each referenced group's char range to bytes by walking
    /// `text` from its start, so one call costs O(end of the match) per reference.
    pub fn replacement_for(&self, caps: &Captures, text: &str, replacement: &str)
        -> Result<String, TemplateError>;                    // T-017's TemplateError
    /// The replacement texts of `matches` (as `find_all(text)` returned them, in that order),
    /// equal element by element to `replacement_for` on each. Bounded cost: O(len(text) +
    /// Σ over matches of (match length × references in `replacement`)), never
    /// O(len(text) × matches). `Err` exactly when `replacement_for` would give it.
    pub fn replacements(&self, matches: &[Captures], text: &str, replacement: &str)
        -> Result<Vec<String>, TemplateError>;
}
```

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

- `search.find`: `total` is the number of all matches; `matches` holds the first `limit`;
  `truncated = total > limit`.
- `search.replace` with `all: false` replaces the first match whose `start ≥ from`; with
  `all: true` it replaces every match. `from` is ignored then, and there is no wrap-around.
  All replacements are one `Buffer::apply_edits` call, so one `doc.undo` reverts them all.
  `ranges` are the inserted texts' ranges in the new text. With no match, `replaced: 0`
  and nothing changes: no undo step, the typing group is not ended.
- Refusals: `QueryError::Empty` and `Pattern` → `rejected`/`invalid_pattern`. The message is
  `empty search pattern`, or `invalid pattern: <PatternError message>`; for a pattern with
  more than 100 capture groups (T-017 ruling s1) that is exactly
  `invalid pattern: the pattern has more than 100 capture groups`; 100 groups are accepted.
  Other `PatternError` messages are the regex crate's (T-017 s2) and are not pinned here. Then
  `no_such_document`, and `from > len_chars` → `out_of_range`. All kinds exist (T-032's reason
  table); no new kind.
- Replacement naming a group that does not exist (regex mode; T-017 `TemplateError`): whether a
  template is an error depends only on the pattern and the template, so `search.replace`
  checks it before matching, whether or not there is a match, and refuses with
  `rejected`/`invalid_pattern`, message `invalid replacement: <TemplateError Display>` (e.g.
  `invalid replacement: no capture group "3"`). Nothing changes. (Ruled by D-044.)
- Cost of replace-all (T-017 implementer note): `search.replace` computes its replacement
  texts with `replacements`, so replace-all is linear in the text plus the replaced matches,
  not O(text × matches) as one `replacement_for` call per match would be. AC-7 checks it.

`docs/protocol/search.md` documents the commands and all rules above, with the examples,
including T-017's expansion rule (a reference to a group that does not exist is refused, unlike
the `regex` crate's own expansion) and that inline flags override the options.

## Pinned
- (a) Precedence: schema → `no_such_document` → `invalid_pattern` (the pattern, then the
  replacement template) → `out_of_range`.
- (b) Files: none.
- (c) Order: matches are computed on the document's current text before any replacement;
  replacement texts are computed from those captures.
- (d) Offsets are chars. Platform independent.
- (e) Threat model: n/a (no boundary). Pathological patterns are bounded by T-017's engine
  guarantee and its 100-capture-group cap (s1). Replace-all is bounded as in Interface (cost
  of replace-all).
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
  through the Rust API, `replacements` equals `replacement_for` applied to each match on the
  AC-4 inputs. Oracle: exact value + coarse timeout.

## Scorecard items and intended test names
| Item | Tests |
|---|---|
| search.literal | `kiln_cli::acceptance_t021::literal_find_matches_naive_scan` |
| search.regex | `kiln_cli::acceptance_t021::regex_find_matches_naive_backtracker`, `kiln_cli::acceptance_t021::invalid_patterns_are_reported` |
| search.whole-word | `kiln_cli::acceptance_t021::whole_word_filters_at_identifier_boundaries` |
| search.replace | `kiln_cli::acceptance_t021::replace_all_matches_naive_model_and_undoes_in_one_step`, `kiln_cli::acceptance_t021::replace_one_from_offset`, `kiln_cli::acceptance_t021::replace_all_is_linear` |

## Out of scope and implementer notes
- Search across files (T-022); find-next with selection or wrap-around (UI, iteration 2);
  multi-line literal patterns beyond exact char equality; incremental search.
- Your own tests: `crates/kiln-search/tests/query.rs`, unit tests. Acceptance tests (odd arm)
  go on `task/T-021-tests`, committed and failing on base before you write the code.

## Questions and rulings
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
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
