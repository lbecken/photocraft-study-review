---
id: T-001
title: "Buffer with exact position conversions, line access and operation-based undo/redo"
tier: high              # scorecard items undo.undo / undo.redo are high; L0 public API is the base contract
state: queued
owner_role: director
scorecard_items: [text.insert, text.delete, text.positions, text.lines, undo.undo, undo.redo, undo.grouping, undo.roundtrip]
depends_on: []
touches:
  - crates/kiln-text/src
  - crates/kiln-text/Cargo.toml
  - Cargo.toml
  - Cargo.lock
budget_tokens: 700000
iteration: 0
found_by: planner
origin_task: null
---

# T-001 Buffer with exact position conversions, line access and operation-based undo/redo

## Goal
After this task a caller holding a `kiln_text::Buffer` can edit large texts by char index,
convert any position between byte offset, char index, UTF-16 offset and line/column exactly,
read lines split on LF, CRLF and CR, learn which line-ending style the text uses, and undo and
redo its edits operation by operation, with consecutive typing grouped into one step. Every
later crate (documents, commands, syntax, search) builds on this surface, so it is specified
exactly and must not change without a task of its own.

## Reading list
- `docs/CHARTER.md` §3 (layer L0; decisions 3 operation-based undo, 4 determinism, 5 oracles).
- `crates/kiln-text/src/buffer.rs`, `crates/kiln-text/src/lib.rs` (current API).
- `crates/kiln-commands/src/buffer.rs` and `registry.rs` (current callers: `Buffer::new`,
  `from_text`, `insert`, `text`, `len_chars`; `Context` derives `Clone, Debug, Default`).
- `scorecard/items/{text,undo}.*.toml`.

## Interface
All items are exported from the crate root (`kiln_text::…`). Indices without a unit in their
name are **char indices** (Unicode scalar values). Every `Range<usize>` is a half-open char
range. "Valid" is defined per method below. Every fallible method leaves the buffer (text *and*
undo/redo history) unchanged when it returns `Err`.

```rust
/// Kept exactly as on main (variants, fields, Display wording may be improved).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditError {
    OutOfBounds { index: usize, len_chars: usize },
    InvertedRange { start: usize, end: usize },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PositionError {
    /// `index` (in the unit of the call: byte, char or UTF-16) exceeds `len` (same unit).
    OutOfBounds { index: usize, len: usize },
    /// The byte offset falls inside a multi-byte UTF-8 sequence.
    NotCharBoundary { byte: usize },
    /// The UTF-16 offset falls between the two halves of a surrogate pair.
    InsideSurrogatePair { utf16: usize },
    /// The char index lies between the `\r` and `\n` of a CRLF (no line/column exists there).
    InsideLineBreak { char_idx: usize },
    /// `line >= len_lines()`.
    LineOutOfBounds { line: usize, len_lines: usize },
    /// `col` exceeds the length in chars of that line's content.
    ColumnOutOfBounds { line: usize, col: usize, line_len: usize },
}
// Both error types implement std::error::Error + Display.

/// Zero-based line and zero-based column; the column counts chars.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LineCol { pub line: usize, pub col: usize }

/// Line-ending style of a text. A line break is `\r\n`, a lone `\n` or a lone `\r`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LineEndings { None, Lf, Crlf, Cr, Mixed }

/// One kind of line break.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LineEnding { Lf, Crlf, Cr }

/// Clone + Debug + Default. PartialEq/Eq compare the TEXT ONLY (history is ignored).
/// A clone carries an independent copy of the undo/redo history.
pub struct Buffer { /* private */ }

impl Buffer {
    // construction and size (unchanged signatures)
    pub fn new() -> Self;                         // empty text, empty history
    pub fn from_text(text: &str) -> Self;         // empty history
    pub fn len_chars(&self) -> usize;
    pub fn len_bytes(&self) -> usize;             // UTF-8 bytes
    pub fn len_utf16(&self) -> usize;             // UTF-16 code units
    pub fn len_lines(&self) -> usize;             // line breaks + 1 (so "" has 1 line)
    pub fn is_empty(&self) -> bool;

    // reading (unchanged signatures for text/slice)
    pub fn text(&self) -> String;
    pub fn slice(&self, range: Range<usize>) -> Result<String, EditError>;
    /// Content of line `line` without its line break.
    pub fn line(&self, line: usize) -> Result<String, PositionError>;
    /// Char index of the first char of `line` (== len_chars() for an empty last line).
    pub fn line_start(&self, line: usize) -> Result<usize, PositionError>;
    pub fn line_endings(&self) -> LineEndings;
    /// The most frequent kind of line break; ties go to Lf, then Crlf, then Cr; None if the
    /// text has no line break.
    pub fn dominant_line_ending(&self) -> Option<LineEnding>;

    // position conversions; valid inputs are 0..=len in the input unit
    pub fn char_to_byte(&self, char_idx: usize) -> Result<usize, PositionError>;
    pub fn byte_to_char(&self, byte: usize) -> Result<usize, PositionError>;
    pub fn char_to_utf16(&self, char_idx: usize) -> Result<usize, PositionError>;
    pub fn utf16_to_char(&self, utf16: usize) -> Result<usize, PositionError>;
    pub fn char_to_line_col(&self, char_idx: usize) -> Result<LineCol, PositionError>;
    pub fn line_col_to_char(&self, pos: LineCol) -> Result<usize, PositionError>;

    // edits; each successful non-empty edit is recorded as one undo operation
    /// Unchanged signature: insert before the char at `char_idx` (== len_chars() appends).
    pub fn insert(&mut self, char_idx: usize, text: &str) -> Result<(), EditError>;
    /// Unchanged signature: delete `range`, return the removed text.
    pub fn delete(&mut self, range: Range<usize>) -> Result<String, EditError>;
    /// Replace `range` with `text` as ONE undo step; return the removed text.
    pub fn replace(&mut self, range: Range<usize>, text: &str) -> Result<String, EditError>;

    // undo / redo
    pub fn undo(&mut self) -> bool;               // false (and no change) if nothing to undo
    pub fn redo(&mut self) -> bool;               // false (and no change) if nothing to redo
    pub fn can_undo(&self) -> bool;
    pub fn can_redo(&self) -> bool;
    /// End the current typing group: the next insert starts a new undo step.
    pub fn break_undo_group(&mut self);
}
```

Definitions used by the ACs:
- **Naive model.** A plain `String` `m`. Char index `c` ↔ byte `m.char_indices().nth(c)` (or
  `m.len()` at the end); UTF-16 offset of `c` = Σ `len_utf16()` of the first `c` chars; lines =
  `m` split on the regex `\r\n|\n|\r` (CRLF matched first); `LineCol{line, col}` of `c` = the line
  containing `c` and `c − line_start`.
- **Valid char index for line/column**: any `0..=len_chars()` except an index whose previous char
  is `\r` and whose char is `\n` (→ `InsideLineBreak`). Valid `LineCol`: `line < len_lines()`,
  `col <= line(line).chars().count()`.
- **`line_endings()`**: `None` if the text has no line break; `Lf` / `Crlf` / `Cr` if every
  line break is of that kind; otherwise `Mixed`.
- **Grouping rule (undo.grouping).** An `insert` joins the previous undo step instead of starting
  a new one iff all hold: (a) the previous undo step consists only of inserts joined by this
  rule; (b) `text` is exactly one char and that char is not `\n` or `\r`; (c) the previous step's
  inserted chars are single non-line-break chars too; (d) `char_idx` equals the char index right
  after the previous step's last inserted char; (e) no `delete`, `replace`, `undo`, `redo` or
  `break_undo_group` happened since. `delete` and `replace` never join and are never joined.
- **No-op edits** (`insert` of `""`, `delete` or `replace` of an empty range with `""`) change
  nothing and record nothing (they do not break a typing group either). `replace` of an empty
  range with non-empty text is an insert-like step but never joins a group.
- A successful edit clears the redo history.

## Acceptance criteria
- AC-1 (differential): For fixed-seed pseudo-random sequences (≥ 3 seeds × ≥ 2 000 ops) of
  `insert`, `delete`, `replace` at random char positions, with inserted text drawn from ASCII,
  2-, 3- and 4-byte chars, `\n`, `\r` and `\r\n`, including out-of-range and inverted ranges:
  after every op `text()`, `len_chars()`, `len_bytes()`, `len_utf16()`, `len_lines()` and the
  returned removed text equal the naive model; an op the model rejects returns the same
  `EditError` variant and fields and changes nothing. Oracle: naive `String` model.
- AC-2 (positions): For every char index of ≥ 50 generated buffers (same alphabet), the six
  conversions return exactly the naive-model value, and they are inverse:
  `byte_to_char(char_to_byte(c)) == c`, `utf16_to_char(char_to_utf16(c)) == c`,
  `line_col_to_char(char_to_line_col(c)?) == c` for every valid `c`. Oracle: naive model +
  round-trip.
- AC-3 (invalid positions): exact errors: a byte offset inside a multi-byte char →
  `NotCharBoundary{byte}`; a UTF-16 offset between surrogate halves of "😀" →
  `InsideSurrogatePair{utf16}`; any input above its length → `OutOfBounds{index, len}` in that
  unit; the index between `\r` and `\n` of a CRLF → `InsideLineBreak{char_idx}`;
  `line == len_lines()` → `LineOutOfBounds`; `col == line_len + 1` → `ColumnOutOfBounds`.
  Oracle: exact value.
- AC-4 (lines): For each string of the corpus `""`, `"a"`, `"\n"`, `"a\n"`, `"\r\n"`, `"\r"`,
  `"\r\r\n"`, `"\n\r"`, `"a\r\nb\rc\nd"`, `"é\n😀\r\n"` and for generated strings,
  `len_lines()`, `line(i)` and `line_start(i)` for every `i` equal the naive split. Example
  exact values: `"a\r\nb\rc\nd"` → 4 lines `["a","b","c","d"]`, starts `[0,3,5,7]`; `"a\n"` → 2
  lines `["a",""]`, starts `[0,2]`. Oracle: naive model + exact values.
- AC-5 (line-ending style): `line_endings()` is `None` for `""` and `"abc"`, `Lf` for `"a\nb\n"`,
  `Crlf` for `"a\r\nb\r\n"`, `Cr` for `"a\rb"`, `Mixed` for `"a\nb\r\n"` and `"a\r\r\n"`.
  `dominant_line_ending()` is `None` for `"abc"`, `Some(Crlf)` for `"a\r\nb\r\nc\n"`,
  `Some(Lf)` for `"a\nb\r\n"` (tie), `Some(Crlf)` for `"a\r\nb\r"` (tie), `Some(Cr)` for
  `"a\r\r\r\n"`, and for generated strings equals the naive count over the CRLF-first split.
  Oracle: exact value + naive model.
- AC-6 (undo/redo one step): for any single successful non-no-op edit on any buffer from the AC-1
  generator, `undo()` returns `true` and restores the exact previous text; `redo()` returns `true`
  and restores the exact post-edit text; with nothing to undo/redo they return `false` and change
  nothing. Oracle: round-trip.
- AC-7 (round-trip property): after any generated sequence of ≥ 200 successful edits (interleaved
  with `break_undo_group` and some undo/redo), repeated `undo()` until it returns `false` restores
  the text at the start of the sequence, and the texts seen on the way are exactly the model's
  sequence of step boundaries in reverse; repeated `redo()` then reaches the final text again.
  Oracle: round-trip against a model list of texts per undo step (the model applies the grouping
  rule).
- AC-8 (redo cleared): `insert` / `delete` / `replace` after an `undo` makes `can_redo()` false and
  `redo()` returns `false`. Oracle: exact value.
- AC-9 (grouping): on `Buffer::new()`, `insert(0,"a")`, `insert(1,"b")`, `insert(2,"c")` then one
  `undo()` gives `""`. Each of the following breaks the group so one `undo()` gives `"ab"`:
  third insert at 1 instead of 2 (then text before undo is `"acb"`); third insert of `"cd"`;
  third insert of `"\n"`; `break_undo_group()` before the third insert; a `delete(0..0)`-style
  no-op does NOT break it (one undo gives `""`). Oracle: exact value.
- AC-10 (no-op and failed edits): no-op edits and rejected edits leave `can_undo()`, `can_redo()`
  and the result of a following `undo()` exactly as before. Oracle: exact value.
- AC-11 (large text): a 10 MiB buffer (generated, mixed alphabet, ~200 000 lines) built with
  `from_text`, then ≥ 200 random edits and conversions at random positions, matches the naive model
  as in AC-1/AC-2 (correctness only; latency: D-2). Oracle: naive model.
- AC-12 (equality and clones): two buffers with equal text compare equal whatever their history;
  undoing on a clone does not change the original. Oracle: exact value.
- AC-13 (compatibility): `kiln-cli exec buffer.insert '{"offset":3,"text":"!"}' --text abc` still
  prints `{"len_chars":4}` and every existing test in the workspace passes unchanged. Oracle:
  exact value (existing tests are the gate's `test` step).

Not observable through the interface (reviewer checks it, no test can): that undo is stored as
operations (inverse edits), not snapshots of the text (CHARTER §3 decision 3), and that storage
is a rope / tree with sub-linear edits rather than one contiguous `String`.

## Scorecard items and intended test names
Tests in `crates/kiln-text/tests/acceptance_t001.rs` (names `kiln_text::acceptance_t001::<fn>`).
The test author replaces the bootstrap unit-test lists of `text.insert` / `text.delete` with
these (D-3) and fills the others:

| Item | Tests |
|---|---|
| text.insert | `insert_matches_naive_model`, `insert_rejects_out_of_range_and_leaves_buffer_unchanged` |
| text.delete | `delete_and_replace_match_naive_model`, `delete_rejects_inverted_and_out_of_range` |
| text.positions | `conversions_match_naive_model`, `conversions_round_trip_on_valid_positions`, `invalid_positions_give_exact_errors` |
| text.lines | `lines_match_naive_split_for_lf_crlf_cr`, `line_endings_style_exact_values`, `dominant_line_ending_matches_naive_count` |
| undo.undo | `undo_restores_text_before_each_edit_kind`, `undo_and_redo_with_empty_history_are_noops` |
| undo.redo | `redo_reapplies_undone_edit`, `edit_after_undo_clears_redo` |
| undo.grouping | `adjacent_typed_chars_group_into_one_step`, `group_breaks_on_gap_multichar_linebreak_and_explicit_break` |
| undo.roundtrip | `undo_all_restores_original_redo_all_restores_final`, `noop_and_rejected_edits_do_not_touch_history` |

Also (no item; see D-2): `large_buffer_edits_match_naive_model`, `equality_ignores_history_and_clones_are_independent`.

## Out of scope
- Grapheme-cluster boundaries (cursor movement and deletion by grapheme) — needs Unicode
  segmentation data; belongs to the `edit.selection` task.
- Line-ending conversion or normalisation (`io.mixed-eol`, a later kiln-text/commands task) and
  any file I/O or encoding (T-002).
- Explicit multi-edit transactions (`begin/end group` around several edits) — the first task with
  a multi-edit command (`search.replace`, `fmt.rustfmt`, `ws.replace-in-files`).
- Grapheme-cluster handling of any kind (D-1).
- A latency budget runner and the scorecard item `text.rope` (D-2): its title couples rope
  storage to "within the edit latency budget", so it cannot be split and stays unclaimed until
  the budgets-runner task lands. Edit-change notifications for syntax/search.
- Any change outside `touches` (including `kiln-commands` and `docs/development.md`'s layout
  line). The only dependency added is `ropey` (D-1); no other crate, no dev-dependency.

## Implementer notes
- Keep the signatures the commands crate uses (`new`, `from_text`, `insert`, `text`,
  `len_chars`) and `Clone + Debug + Default` on `Buffer`, or `kiln-commands` stops compiling and
  you would have to leave `touches`.
- The scorecard items `text.insert` / `text.delete` list unit tests
  `kiln_text::buffer::tests::{insert_into_empty_buffer, insert_at_start_middle_and_end,
  insert_uses_char_indices_not_bytes, insert_out_of_bounds_is_rejected_and_buffer_unchanged,
  delete_returns_removed_text, delete_empty_range_is_noop,
  delete_rejects_inverted_and_out_of_bounds_ranges, random_edits_match_naive_model}`. Until the
  joined test branch replaces those lists, keep tests with exactly those names in module
  `buffer::tests`, or `scorecard --check` fails (you may not edit `scorecard/`).
- Storage uses the `ropey` crate (D-1): add it to `[workspace.dependencies]` in the root
  `Cargo.toml` with an exact version requirement and to `crates/kiln-text/Cargo.toml` as
  `ropey.workspace = true`. `ropey`'s own line breaks differ from this spec's (it also breaks on
  U+000B, U+000C, U+0085, U+2028, U+2029 unless configured); the Interface definitions win.
- `Cargo.lock` conflict: T-002 also changes `Cargo.lock` and runs in parallel. Whichever of the two
  reaches the queue second will conflict there; until T-005 lands, the director resolves it by
  regenerating the lock file (T-005's generated-file policy then automates this).
- Determinism: no randomness in the crate; tests use fixed-seed generators written in the test
  file (no dev-dependency: the test author may not change `Cargo.*`).
- Your own unit tests should exercise the storage's internal boundaries (whatever your structure
  splits on) with edits that straddle them, differentially against a `String`.
- Performance target (not gated, D-2): an insert or delete in a 10 MiB buffer should not copy the
  whole text; state in your log entry how you checked it.

## Decisions
Director's answers to the planner's questions of R-003 (2026-11-12).
- D-1 (was Q-1): storage uses the director-approved `ropey` crate; no in-crate rope.
  Grapheme-cluster handling stays out of this task. T-001 and T-002 still run in parallel; the
  second to reach the queue conflicts on `Cargo.lock` and, until T-005 lands, the director
  resolves that by regenerating the lock file.
- D-2 (was Q-2): the budgets runner is a separate later task. `text.rope` cannot be split
  (storage and latency are one item), so T-001 leaves it unclaimed.
- D-3 (was Q-3): the independent acceptance tests replace the bootstrap unit tests as the
  scorecard evidence for `text.insert` and `text.delete`.
- D-4 (was Q-4, accepted as proposed): the column unit is chars.
- D-5 (follows from the T-002 decision on line endings): documents report the detected dominant
  line ending, so this task adds `LineEnding` and `dominant_line_ending()` (T-003 reports them).

## Questions
None open.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · tokens (if known) · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-12 · R-003 · (new) · did: specified the L0 public surface (positions, lines, line-ending
  style, edits, op-based undo/redo with an exact grouping rule) against main `48683de`; ACs use a
  naive `String` model, round-trip and exact values. Tier high: scorecard items `undo.undo` and
  `undo.redo` are high-tier and the API is the base contract of every later crate (no CHARTER §3
  text changes, so no ADR). `touches` limited to `crates/kiln-text/src` by forbidding new
  dependencies (Q-1). Unsure: the dependency choice (Q-1), whether `text.rope` can be claimed
  without a budget runner (Q-2, not claimed), re-pointing the bootstrap items (Q-3), column unit
  (Q-4); "operation-based, not snapshots" is not black-box observable and is left to the reviewer.
  · gate attempts: 0 · not run: gate (no code) · missing context: none · next: director review.
- 2026-11-12 · R-003 · draft · did: applied the director's answers (D-1…D-4): `ropey` added,
  so `touches` gains `crates/kiln-text/Cargo.toml`, `Cargo.toml`, `Cargo.lock`; noted the
  `Cargo.lock` conflict with T-002 and how it is resolved; `text.rope` stays unclaimed because the
  item cannot be split; grapheme handling explicitly out of scope. · gate attempts: 0 · next:
  director review.
<!-- end planner log -->
### implementer
- 2026-11-12T23:00Z · R-004 · ready · ACs restated: edits/sizes/removed text/errors equal a
  `String` model (AC-1, AC-11); six conversions exact and mutually inverse, exact error variants
  for out-of-range, mid-UTF-8, mid-surrogate, mid-CRLF, line/col out of range (AC-2, AC-3); lines
  split CRLF-first on `\r\n|\n|\r` (AC-4); style and dominant kind with Lf>Crlf>Cr ties (AC-5);
  op-based undo/redo with the five-part typing rule, no-ops/rejects invisible to history,
  edits clear redo (AC-6..AC-10); `Eq` on text only, independent clones (AC-12); CLI unchanged
  (AC-13). · did: `ropey =1.6.1` (default-features off, `cr_lines`+`simd`, so ropey breaks
  exactly on LF/CRLF/CR) as storage; per-kind break counts kept incrementally from a
  one-char-margin window per edit; history = stacks of inverse `Op`s, a typing group is one
  `Op` whose inserted text grows; new modules `history.rs`, `line_ending.rs`, `position.rs`.
  Unit tests keep the 8 bootstrap names and add multi-chunk differential edits (3 seeds ×
  1 500 ops on ~6–12k-char texts, CRLF made/split at edit edges), all-index conversion checks,
  undo/redo against a step-boundary model (3 seeds × 400 edits), exact grouping/no-op/clone
  cases; the differential test caught one bug (window-count underflow), fixed. Checked from
  outside: `kiln-cli exec buffer.insert '{"offset":3,"text":"!"}' --text abc` → `{"len_chars":4}`.
  Performance (not gated): a scratch release binary outside the repo on a 10 MiB, 291 273-line
  buffer measured 1 000 inserts 1.9 ms, 1 000 deletes 1.3 ms, 2 000 undos 1.7 ms vs 5.3 ms for a
  single full `text()` copy, so an edit does not copy the text. · gate attempts: 1 (last: green;
  fuzz/budgets EMPTY) · not run: the acceptance tests (not visible to me); full workspace test
  rebuild only via the gate · missing context: (1) error precedence when a range is both
  inverted and out of range, and start-vs-end when both are out of range, is not specified; I
  kept main's order (InvertedRange, then start, then end); (2) whether a rejected edit or a
  `redo()`/`undo()` that returns `false` ends a typing group is not stated; I chose "no" (they
  change nothing), consistent with "history unchanged on Err"; (3) feature unification: if any
  future crate enables ropey's default `unicode_lines`, line semantics silently change — a unit
  test (`unicode_separators_do_not_break_lines`) guards it. · next: join with T-001-tests, review.
<!-- end implementer log -->
### test author
- 2026-11-12 · R-005 · ready · did: wrote 21 tests in `crates/kiln-text/tests/acceptance_t001.rs`
  and 1 in `apps/kiln-cli/tests/acceptance_t001.rs` from the Interface and ACs only (did not look
  at `task/T-001`); filled the `tests` lists of the 8 scorecard items with the names in the task
  table (D-3 replaces the bootstrap lists of `text.insert` / `text.delete`). Oracles in the test
  file: naive `Vec<char>` model, CRLF-first split, a per-step history model that applies the
  grouping rule, round trips, exact values; fixed-seed SplitMix64 generators.
  Criterion → tests → result on base (`b8411f5` + these tests):
  AC-1 → `insert_matches_naive_model`, `insert_rejects_out_of_range_and_leaves_buffer_unchanged`,
  `delete_and_replace_match_naive_model`, `delete_rejects_inverted_and_out_of_range` (3 seeds ×
  2 000 ops for each differential test) → compile;
  AC-2 → `conversions_match_naive_model` (60 buffers, every char/byte/UTF-16 offset and every
  line/col, incl. one past the end), `conversions_round_trip_on_valid_positions` → compile;
  AC-3 → `invalid_positions_give_exact_errors` (+ `error_types_implement_error_and_display`) →
  compile; AC-4 → `lines_match_naive_split_for_lf_crlf_cr` → compile;
  AC-5 → `line_endings_style_exact_values`, `dominant_line_ending_matches_naive_count` → compile;
  AC-6 → `undo_restores_text_before_each_edit_kind`, `undo_and_redo_with_empty_history_are_noops`,
  `redo_reapplies_undone_edit` → compile; AC-8 → `edit_after_undo_clears_redo` → compile;
  AC-9 → `adjacent_typed_chars_group_into_one_step`,
  `group_breaks_on_gap_multichar_linebreak_and_explicit_break` → compile;
  AC-7 → `undo_all_restores_original_redo_all_restores_final` (4 seeds × 250 non-no-op edits) →
  compile; AC-10 → `noop_and_rejected_edits_do_not_touch_history` → compile;
  AC-11 → `large_buffer_edits_match_naive_model` (≥ 10 MiB, ~200 000 lines, 250 edits, 252
  conversions, 100 line reads; text and len_lines checked every 25 edits and at the end, the other
  lengths and the removed text after every edit, to keep the debug-build model fast) → compile;
  AC-12 → `equality_ignores_history_and_clones_are_independent` → compile;
  AC-13 → `kiln_cli::acceptance_t001::cli_exec_buffer_insert_still_prints_len_chars` → PASS on
  base. Kept as a regression guard: AC-13 is a compatibility property, so a failure on base is
  impossible by design; its "existing tests pass" half is the gate's test step.
  Every kiln-text test fails on base only because the new API is missing (compile), so the
  reviewer's revert check applies.
  Edge and hostile inputs: empty buffer; CRLF, lone CR, `\r\r\n`, `\n\r`; CRLF joined and split
  by edits; VT/FF/NEL/U+2028/U+2029, which must not break lines (ropey's default would); 2-, 3-
  and 4-byte chars; surrogate pairs; offsets inside chars; one-past and `usize::MAX` indices;
  inverted ranges; no-op edits; rejected edits while redo is pending.
  Test validation (not committed): I temporarily replaced `crates/kiln-text/src/lib.rs` in this
  worktree with a throwaway naive implementation of the Interface. All 21 tests passed (the large
  test took 121 s with that O(n) stub), and clippy `-D warnings` and fmt were clean. I then ran 5
  hand mutants (U+2028 as a line break, a no-op clears redo, undo leaves the group open, wrong
  byte error, joining without adjacency), and each was caught by at least 1 test. The source was
  restored with `git checkout`.
  QUESTION: (Q-a) for delete/replace with both bounds above `len_chars` (start ≤ end), which
  index does `OutOfBounds` report? The tests accept start or end. Proposed: "the first bound
  out of range, start before end (as on main)". (Q-b) is `insert(i, "")` / `delete(i..i)` /
  `replace(i..i, "")` with `i > len_chars` a no-op `Ok` or `Err(OutOfBounds)`? Not generated.
  Proposed: validate first, so `Err` (as on main). (Q-c) which variant does an inverted range
  that is also out of range give? Not generated. Proposed: `InvertedRange` (checked first, as
  on main). (Q-d) does a rejected edit, or an `undo()`/`redo()` that returns `false`, end an open
  typing group? Rule (e) says the call "happened", but "no change" points the other way. Not
  tested. Proposed: they change nothing, including the group.
  · gate attempts: 1 (last: red clippy, scorecard, test — all from compile errors in
  `crates/kiln-text/tests/acceptance_t001.rs` alone, as expected for `compile` cases; fmt,
  layers, ui-commands, schema, no-skip pass) · not run: nothing else · tokens: n/a · missing
  context: none · next: join with `task/T-001`, reviewer revert check.
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
