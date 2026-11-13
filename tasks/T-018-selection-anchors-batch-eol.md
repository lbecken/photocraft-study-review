---
id: T-018
title: "Give each document a selection and anchors that follow every edit, apply multi-range edits as one undo step, move the cursor by char, word and line, and normalise line endings on request"
tier: medium            # ADR-0002 default: in-memory behaviour in kiln-text plus two command groups; no persistence, process, gate, security or dependency criterion
state: queued
owner_role: planner
scorecard_items: [edit.selection, io.mixed-eol]
depends_on: [T-015, T-010]   # module slots cursor.rs / eol.rs; T-010's `not_a_boundary` reason and PositionError mapping
touches:
  - crates/kiln-text/src/lib.rs              # new mods + re-exports
  - crates/kiln-text/src/buffer.rs           # selection, anchors, apply_edits, normalize_line_endings
  - crates/kiln-text/src/history.rs          # batch steps
  - crates/kiln-text/src/selection.rs        # new: Selection, AnchorId, mapping, movement
  - crates/kiln-text/tests/selection.rs      # implementer's own integration tests
  - crates/kiln-commands/src/cursor.rs       # cursor.* (slot from T-015)
  - crates/kiln-commands/src/eol.rs          # eol.* (slot from T-015)
  - docs/protocol/cursor.md                  # new
  - docs/protocol/eol.md                     # new
  - docs/commands.json                       # generated
budget_tokens: 500000   # medium (v1.1, provisional)
iteration: 1
found_by: planner
origin_task: null
---

# T-018 Selections, anchors, batch edits and line-ending normalisation

## Goal
After this task each open document has one selection (an anchor and a head). It maps through
every edit, including `doc.insert`/`delete`/`replace`, undo and redo, without the commands that
make the edit knowing about it. A script can read, set and move it by char, word and line. Any
layer can place anchors (single positions that follow edits the same way); T-024 uses them so
that diagnostics follow edits. A list of non-overlapping range edits applies as **one** undo
step. That is the primitive behind replace-all (T-021), replace in files (T-022), indent and
comment toggling (T-025) and formatting (T-026). And `eol.normalize` makes a mixed file
uniform on explicit request. File I/O keeps bytes-as-read: nothing is normalised at open or
save.

No gate path. Arm: **even** (independent test author).

Reading list: `crates/kiln-text/src/{lib,buffer,history,line_ending,position}.rs` (undo grouping
rule in `lib.rs`); `tasks/T-010-*.md` Interface (valid positions; `InsideLineBreak`);
`tasks/T-015-*.md` "Ownership" and `Parts`; `docs/protocol/` (new directory: one file per group,
director ruling R-056).

## Interface

### `kiln_text` (L0)
```rust
pub use selection::{AnchorId, Movement, Selection};
pub use buffer::BatchError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection { pub anchor: usize, pub head: usize }       // char offsets
impl Selection { pub fn start(self) -> usize; pub fn end(self) -> usize; pub fn is_empty(self) -> bool; }
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AnchorId(/* private */);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Movement { Char, Word, Line }
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BatchError {
    /// Edit `index` starts before the end of edit `index - 1`, or at the same start.
    Unordered { index: usize },
    /// Edit `index` is out of bounds or inverted.
    Edit { index: usize, error: EditError },
}

impl Buffer {
    pub fn selection(&self) -> Selection;                                   // new buffer: 0,0
    /// Fails with OutOfBounds, or InsideLineBreak for a position between the \r and \n of a CRLF.
    pub fn set_selection(&mut self, s: Selection) -> Result<(), PositionError>;
    pub fn add_anchor(&mut self, pos: usize) -> Result<AnchorId, PositionError>;   // same validity
    pub fn anchor(&self, id: AnchorId) -> Option<usize>;                    // None once removed
    pub fn remove_anchor(&mut self, id: AnchorId) -> bool;
    /// The head's new position for one movement step (does not change the buffer).
    pub fn moved(&self, from: usize, by: Movement, forward: bool) -> usize;
    /// Apply `edits` (ranges in the current text) as one undo step.
    pub fn apply_edits(&mut self, edits: &[(std::ops::Range<usize>, &str)]) -> Result<(), BatchError>;
    /// Rewrite every line break as `to`, as one undo step; returns the number of breaks rewritten.
    pub fn normalize_line_endings(&mut self, to: LineEnding) -> usize;
}
```

### Mapping rule (selection ends and anchors), exact
A primitive edit replaces chars `[s, e)` with `n` chars. A position `p` maps to:
- `p` if `p < s`;
- `p + n − (e − s)` if `p > e`;
- `s` if `p == s` and `e > s`;
- `s + n` otherwise. That covers `s < p ≤ e`, and `p == s` for a pure insertion, so a caret
  at the insertion point ends up after the inserted text.

Then, if the result lies between the `\r` and `\n` of a CRLF, it moves to before the `\r`.
`apply_edits` maps through its edits from the last one to the first. Undo and redo map through
the primitive edits they apply, in the order they apply them. Rejected and no-op edits map
nothing.

### Batch rule
Edits must satisfy `edits[i].start < edits[i+1].start` and `edits[i].end <= edits[i+1].start`.
Each range must be in bounds and not inverted. They are checked in index order, and the first
failure is reported. On error nothing changes. The effect is that of applying them from last to
first. One undo step restores the exact previous text. A batch ends any open typing group and
never joins one. If every edit is a no-op (or the list is empty), the history and the typing
group are unchanged.

### Movement rule
Word char = `c == '_' || c.is_alphanumeric()`.
- `Char` forward: +1, but +2 over a CRLF, and never past `len_chars`. Backward likewise.
- `Word` forward: skip non-word chars, then word chars. Backward: the mirror image (skip
  non-word chars before the head, then word chars).
- `Line` forward (down) or backward (up): the same column (in chars) on the next or previous
  line, clamped to that line's content length. There is no remembered goal column. On the
  first line, up gives 0. On the last line, down gives `len_chars`.

### Commands (each `doc` must be open, else `rejected`/`no_such_document`)
| Command | Mutating | Params | Result |
|---|---|---|---|
| `cursor.get` | no | `{doc}` | `CursorResult` |
| `cursor.set` | yes | `{doc, anchor, head?}` (`head` defaults to `anchor`) | `CursorResult` |
| `cursor.move` | yes | `{doc, by: "char"\|"word"\|"line", forward: bool, extend?: bool}` | `CursorResult` |
| `eol.counts` | no | `{doc}` | `{lf, crlf, cr, line_ending, mixed_line_endings}` |
| `eol.normalize` | yes | `{doc, to?: "lf"\|"crlf"\|"cr"}` | `{changed, len_chars, line_ending, mixed_line_endings}` |

`CursorResult` = `{doc, anchor, head, start, end, line, col}`, where `line`/`col` are the head's
0-based line and column (as in T-010's `doc.position`). `cursor.move` without `extend` sets
anchor = head = the new head, so the selection collapses at the moved head. With `extend` only
the head moves. `cursor.set` and `cursor.move` end the document's typing group whenever they
succeed. `eol.normalize` without `to` uses the document's dominant line ending
(`doc.info.line_ending`). If the document has no line break, nothing changes and the result has
`changed: 0`. `line_ending` and `mixed_line_endings` mean what they mean in `doc.info`.

Refusals: an offset beyond the text → `out_of_range`. A position between `\r` and `\n` →
`not_a_boundary` (T-010's kind, approved R-060; T-010 maps `PositionError::InsideLineBreak` to it). No new error kind. Offsets are chars.

`docs/protocol/cursor.md` and `docs/protocol/eol.md` describe the commands, the mapping rule
and the movement rule in user terms, with the worked examples.

## Pinned
- (a) Errors: as above. Precedence: params schema → document exists → position validity in the
  order `anchor`, `head`. `BatchError`: first failing index in order.
- (b) Files: none read or written. Selections and anchors are not saved.
- (c) Order of checks: see (a). For `cursor.move`, the movement is computed from the head
  before the move.
- (d) Offsets: chars; lines split on LF, CRLF, lone CR (kiln-text). Platform independent.
- (e) Threat model: n/a (in memory, no boundary).
- (f) Existing tests: every `acceptance_t001*`, `t003*`, `t004*`, `t007*`, `t010*` and guard
  test stays green unchanged. Results of existing commands do not change. Selection is not
  part of `doc.info`. T-007's read-only audit must still pass: `cursor.get` and `eol.counts`
  change nothing.
- (g) Worked examples (executed against the naive model in a scratch Python script):
  - Text `abc\r\ndef`, `cursor.set {anchor:1, head:6}`; `doc.insert {offset:0, text:"XY"}` →
    `cursor.get` = anchor 3, head 8. Then `doc.delete {start:4, end:7}` (text `XYabdef`) →
    anchor 3, head 5.
  - Text `fn foo_bar(x: u8)`, head 0, repeated `cursor.move {by:"word", forward:true}` → heads
    2, 10, 12, 16. Then backward → 14, 11, 3, 0.
  - Text `abcdef\nxy\nlonger`, head 5, `by:"line"` forward twice → 9 (line 1, col 2), then 12
    (line 2, col 2; no goal column).
  - Text `a\r\nb`, head 1, `by:"char"` forward → 3.
  - `eol.normalize {to:"lf"}` on `a\r\nb\nc\rd` → text `a\nb\nc\nd`, `changed: 2`,
    `line_ending: "lf"`, `mixed_line_endings: false`. One `doc.undo` restores `a\r\nb\nc\rd`.
- (h) Evaluator calls: `edit.selection` → `kiln-cli control --root d` with `doc.open`,
  `cursor.set`, `doc.insert`/`doc.delete`/`doc.undo`, `cursor.get`, `cursor.move`;
  `io.mixed-eol` → `kiln-cli run` with `doc.open` of a mixed file, `eol.counts`,
  `eol.normalize`, `doc.save`, then the file's bytes.

## Acceptance criteria
- AC-1 (`edit.selection`, control): for fixed-seed random sessions (≥ 2,000 commands over
  texts with LF/CRLF/CR, `é`, `😀`) mixing `doc.insert`, `doc.delete`, `doc.replace`,
  `doc.undo`, `doc.redo`, `cursor.set` and `cursor.move`, `cursor.get` after each command
  equals a naive model of the mapping, movement and collapse rules. Oracle: naive model.
- AC-2 (`edit.selection`, control): every worked example in Pinned (g) gives exactly the stated
  values through `kiln-cli run`. Oracle: exact value.
- AC-3 (`edit.selection`, refusals): `cursor.set` with `anchor` beyond the text →
  `rejected`/`out_of_range`; between `\r` and `\n` → `rejected`/`not_a_boundary`; unknown doc → `no_such_document`. Afterwards `cursor.get` is unchanged.
  Oracle: exact value.
- AC-4 (`io.mixed-eol`, CLI): for random mixed texts saved as files, `eol.counts` equals a
  naive count (CRLF first). After `eol.normalize {to}` for each of the three targets and
  `doc.save`, the file's bytes equal the naive replacement of every `\r\n|\r|\n` by `to`, and
  `changed` equals the number of breaks that differed from `to`. Without `to`, the target is
  the dominant kind (ties go to LF, then CRLF, then CR). Oracle: naive model + bytes on disk.
- AC-5 (`io.mixed-eol`, undo): after `eol.normalize`, one `doc.undo` restores the exact text,
  and `doc.redo` re-applies it. A file opened and saved without `eol.normalize` keeps its bytes
  (io.crlf regression). Oracle: round trip.
- AC-6 (Rust API, extra): `apply_edits` on random sorted, non-overlapping edit lists equals the
  naive last-to-first application. One `undo` restores the text exactly. Invalid lists give the
  pinned `BatchError` index and leave text, selection and history unchanged. Anchors follow the
  mapping rule. Oracle: naive model.

## Scorecard items and intended test names
| Item | Tests |
|---|---|
| edit.selection | `kiln_cli::acceptance_t018::selection_follows_random_sessions`, `kiln_cli::acceptance_t018::cursor_worked_examples`, `kiln_cli::acceptance_t018::cursor_refusals` |
| io.mixed-eol | `kiln_cli::acceptance_t018::eol_counts_and_normalize_match_naive_model`, `kiln_cli::acceptance_t018::eol_normalize_is_one_undo_step` |

Also `kiln_text::acceptance_t018::batch_edits_and_anchors` (no item).

## Out of scope and implementer notes
- Multiple selections; goal column; selection in `doc.info`; the newline/indent/comment
  commands (T-025); goto-line (T-023); Enter inserting the document's line ending (T-025).
- Do not change line endings at open or save, and do not change `doc.insert`'s semantics.
- Performance: mapping must not scan the text per edit. The rope budget (T-028) measures
  `Buffer::insert`/`delete` with the selection present.
- Your tests: unit tests in `crates/kiln-text/src`, `crates/kiln-text/tests/selection.rs`.

## Questions and rulings
Director rulings on the test author's open questions (2026-11-13, decision D-078; run R-135), made before the implementer is dispatched and after reading the Mapping rule, the Batch rule, the Commands section, Pinned (a) and the existing `Buffer` API (`break_undo_group`, `EditError`). Each pins behaviour that is observable through the Rust API or `kiln-cli`; none pins how it is implemented. Where a ruling differs from text above, the ruling governs.
- w1 (undo and redo of a batch). The Mapping rule says undo and redo "map through the primitive edits they apply, in the order they apply them" and leaves that order open for a step that holds several edits. Pinned: undoing a step that was applied as a batch applies its inverse primitive edits from the last one (highest position) to the first, the same order `apply_edits` uses, and maps every selection end and anchor through each of them by the Mapping rule; redo maps as the original `apply_edits` did. The inverse edits need not form a list that `apply_edits` itself would accept (a batch that deleted two adjacent ranges is undone by two insertions at the same position, which the Batch rule's strictly increasing starts would refuse), so the order is pinned here directly and not by reference to an `apply_edits` call. A naive model that applies the inverse edits in that order is the oracle.
- w2 (`normalize_line_endings` and `eol.normalize`). Normalising behaves as one `apply_edits` call with one edit per line break whose text differs from the target, in text order: positions map by the Batch rule, the result is one undo step, an open typing group is ended and never joined, and `changed` is the number of those edits. If no line break differs from the target, nothing changes: text, selection, anchors, history and an open typing group are as before, and `changed` is 0. A whole-text replacement, which would send every position inside the text to its end, is not this behaviour.
- w3 (typing group and the `Buffer` methods). `set_selection`, `add_anchor` and `remove_anchor` are not edits: they change neither the history nor an open typing group, whether they succeed or fail. The commands `cursor.set` and `cursor.move` end the document's typing group when they succeed, as the Commands section says (the buffer already has a public `break_undo_group`); a refused `cursor.set` or `cursor.move` leaves it open.
- w4 (`BatchError` for one edit that fails two checks). For each index the edit's own validity is checked first and its order second: an edit that is out of bounds or inverted and also not after its predecessor gives `Edit { index, error }`, with the `EditError` that `Buffer::replace` reports for that range. This is a choice of precedence, not a consequence of the Batch rule; the first failing index is reported as Pinned (a) says.
The test author adds assertions for w1–w4 to its existing tests before the join; the implementer is dispatched with these rulings.
Director rulings after review round 1 (2026-11-13, decision D-084; review `reviews/T-018-r1.md`, run R-144: two documentation defects, one test gap, three spec gaps, one nit; the reviewer's own model agreed with the code on 12,000 random and 58 fixed commands). Made after reading the review's rows, the generated `params_schema` of `cursor.set`, `cursor.move` and `eol.normalize`, the two documents and `xtask/src/join.rs`:
- x1 (findings 1 and 4; implementer and test author). An optional parameter is omitted when it is absent; `null` is not a value for it. `cursor.set {head: null}` and `eol.normalize {to: null}` are refused with `invalid_params`, as `cursor.move {extend: null}` already is, and the generated schema of an optional parameter is its bare type (today `head` is published as integer-or-null and `to` as Target-or-null). One spelling per command keeps the command log canonical. This is the first optional-parameter precedent in the command surface and holds for later commands unless their spec says otherwise. With it the sentence at `docs/protocol/eol.md:55` is true. The test author adds the two refusals and checks its own tests for any use of `null`.
- x2 (findings 2 and 6). The behaviour stays: positions map through each primitive edit of a batch, and the CRLF snap of the Mapping rule is applied after each of them, so it can act on a `\r\n` that exists only in an intermediate text. In `x\r\r\ny`, normalising to LF moves a caret at 2 (the start of the empty line between the lone CR and the CRLF) to 1, the end of the line before. This is a known wart, accepted here for cost: it needs a lone CR directly before a CRLF, the result is always a valid position, and applying the snap once against the final text would change the Mapping rule, the undo and redo mapping of w1, the test author's model and the tests built on it, on a task already over its budget. It is recorded as a follow-up. The implementer corrects `docs/protocol/eol.md:46-50`: the sentence that a selection elsewhere "keeps its place" is replaced by what is true, including this case.
- x3 (finding 3, test gap; test author). The redo half of w1 is tested where the order matters: `x\r\r\ny`, `normalize_line_endings(Lf)`, `undo`, `set_selection(2,2)`, `redo` → selection (1,1), through the Rust API and through `kiln-cli` (`eol.normalize`, `doc.undo`, `cursor.set`, `doc.redo`, `cursor.get`). The reviewer's surviving mutant M18 must be killed.
- x4 (finding 5). `Buffer::line_break_count(LineEnding) -> usize` is part of this task's Interface (the number of line breaks of that kind); the reviewer judged it acceptable and later tasks may rely on it.
- x5 (finding 7, nit; implementer). `docs/protocol/cursor.md:13` says "Offsets are chars (Unicode scalar values)." without "as everywhere in the `doc` group".
- x6 (tooling; test author). `xtask/src/join.rs` identifies acceptance targets by file stem, so the two files named `acceptance_t018.rs` in `kiln-cli` and `kiln-text` count as one and the join record and the revert-check saw only the `kiln-cli` target. Until the tool is fixed (T-033), acceptance and guard test files have names that are unique across crates: the test author renames `crates/kiln-text/tests/acceptance_t018.rs` to `acceptance_t018_text.rs`, with no change to its tests. The scorecard items name only `kiln_cli::acceptance_t018` tests and are not affected.
- x7 (documentation; implementer; round-2 finding 1, R-149). The exception sentence in `docs/protocol/eol.md` ("it ends just before the lone CR's new LF, at the end of the line before") is true only when the CR before the position is the first `\r` of its run. Observed by the reviewer through `kiln-cli run`: `x\r\r\r\ry`, caret 4, `eol.normalize {to:"lf"}` gives head 1, line 0. The sentence must be true for a run of consecutive `\r` of any length, and the document gains `x\r\r\r\ry` (caret 4 ends at 1) as a second example. The behaviour does not change (x2). The implementer runs each example it writes through `kiln-cli` before committing. Round 3 confirms only this fix, and checks the new sentence against the round-2 exhaustive data (all texts up to length 6 over `x`, `\r`, `\n`).

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
- 2026-11-13T17:43Z · R-150 · in_progress · implementer's report for R-150, recorded here by the director (the brief wrongly told the implementer not to write its log entry on the branch; the director first put this under `### implementer` on main, which made the queue's rebase conflict with the branch's own entries there, attempt 1) · did (x7): rewrote the exception sentence in `docs/protocol/eol.md`. A position between a lone CR and a following `\r` ends just before the new LF of the first `\r` of the run of consecutive `\r` it lies in (the run may end in a CRLF's `\r`), at the end of the line before that run, so it can move up several lines. Added the example `x\r\r\r\ry`, caret 4 → 1. Checked through `kiln-cli run`: `x\r\r\r\ry` carets 2/3/4/5 → 1/1/1/5; `x\r\r\r\ny` 2/3 → 1/1; `x\r\n\r\ry` 4 → 2; `\r\r\r` 1/2/3 → 0/0/3; `x\r\r\ny` 2 → 1. All texts up to length 6 over `x`, `\r`, `\n`, target LF: 6,561 cases, 0 mismatches with the documented rules (547 exception, 142 cascading). No code or tests changed. Commit d0bab85f8c994098d1581b6fb8cf452f0a114987. · gate attempts: 0 (hygiene and fmt green) · not run: `cargo xtask ci`, join, queue · tokens 84,366 · missing context: none · next: director joins; round 3 confirms this fix only.
<!-- end director log -->
### planner
- 2026-11-13 · R-056 · (new) · did: put the selection and anchors in `kiln-text`'s `Buffer`
  (the item's crate). That way every existing edit command maps them without editing `doc.rs`,
  and T-024 reuses the anchors for diagnostics. Added the batch primitive that four later
  tasks need. Tier medium: no ADR-0002 high criterion (no dependency, file, process, gate or
  security boundary; the command groups are new). Arm even (T-018), paired against the odd
  T-021 (search) as the other large in-memory task. Every worked example was executed against
  a naive Python model. Pinned (b), (e) n/a. Unsure: `not_a_boundary` depends on T-010 Q-1. ·
  gate attempts: 0 · missing context: none · next: director.
- 2026-11-13 · R-060 · draft · did: `not_a_boundary` is approved and belongs to T-010 (R-060 answer 8): `depends_on` gains T-010, and the fallback to `other` is removed. · gate attempts: 0 · missing context: none · next: director.
- 2026-11-13 · R-066 · draft · did: needs T-032 only transitively (through T-010); `depends_on` unchanged. · gate attempts: 0 · missing context: none · next: director.
<!-- end planner log -->
### implementer
- 2026-11-13T16:51Z · R-139 · in_progress · did: restated the criteria: one selection per document plus anchors, both mapped through every primitive edit (insert/delete/replace, each batch edit, each edit undo/redo applies) by the four-case Mapping rule and the CRLF snap; `apply_edits` validated per index (own range first, then order: w4), applied last-to-first as one undo step that ends and never joins a typing group, all-no-op lists change nothing; undo of a batch applies inverse edits highest-first (w1); movement by char/word/line with no goal column; `normalize_line_endings` = one `apply_edits` with one edit per differing break (w2); `cursor.set`/`cursor.move` end the typing group only on success, the `Buffer` setters never touch it (w3); `eol.counts`/`eol.normalize` with `doc.info`'s `line_ending`/`mixed_line_endings`. Base `85c4c425e6a5f9bae3e6aadec8e1166d194b6622`; rulings w1–w4 (D-078) read from main's copy of this file. Step 0 against the merged code: slots `cursor.rs`/`eol.rs` exist (T-015); `From<&PositionError>` already maps `OutOfBounds` → `out_of_range` and `InsideLineBreak` → `not_a_boundary` (T-010/T-032), so no `reason.rs` row is needed; `ctx.doc` gives `no_such_document` for `doc: 0`. Built: `kiln-text` `selection.rs` (`Selection`, `AnchorId`, `Movement`, mapping), `history.rs` steps of several ops, `buffer.rs` selection/anchors/`moved`/`apply_edits`/`normalize_line_endings`/`BatchError`; `kiln-commands` `cursor.get/set/move`, `eol.counts/normalize`; `docs/protocol/cursor.md`, `eol.md`; `docs/commands.json` regenerated. Tests: `crates/kiln-text/tests/selection.rs` (fixed-seed sessions against a naive model incl. batches, undo/redo, anchors, normalize; mapping cases; CRLF snap; w1 adjacent-delete undo; w2 no-op normalize keeps history and group; w3 at `Buffer` level; w4 precedence), unit tests in `buffer.rs`, `selection.rs`, `cursor.rs` (w3 through `cursor.set`/`cursor.move`, refusal order), `eol.rs`. Checked from outside with `kiln-cli run` (worked examples of Pinned (g) 1, 2 and 5, refusals, save bytes). Wording drifts, no pinned behaviour changed: (1) `eol.counts` needs per-kind counts that `Buffer` did not expose and `line_ending.rs` is not in `touches`, so `buffer.rs` gains `Buffer::line_break_count(LineEnding) -> usize` (public API beyond the Interface block); (2) `LineEndingName` is reused from `doc.rs` (pub(crate)); `eol.rs` has its own copy of the 4-line name mapping because `doc.rs` is not in `touches`; (3) `docs/protocol/` already existed (T-010); (4) `Buffer::moved` takes an invalid `from` as clamped to `len_chars` and snapped before a CRLF's `\r` (the spec leaves it open; heads are always valid); (5) refusal messages are not pinned: `invalid anchor: <PositionError>` / `invalid head: …`; (6) `head: null` / `to: null` are accepted as absent (schema `Option`). · gate attempts: see next entry · not run: 32-bit build · missing context: none · next: gate.
- 2026-11-13T16:55Z · R-139 · in_progress · did: gate. Attempt 1 red at clippy (`unwrap_used` and `type_complexity` in my `crates/kiln-text/tests/selection.rs` helper code); fixed without allowances (assert on the batch result, a named `Case` type). Attempt 2 green: fmt, clippy, layers, ui-commands, hygiene, schema (20 commands, audit ok, commands.json current), scorecard (63 items, 0 failing), test (602 passed, 0 failed, 0 ignored, 75 suites), no-skip, fuzz EMPTY, budgets EMPTY — 11 steps. · gate attempts: 2 (last: green) · not run: the independent acceptance tests (not visible before the join); 32-bit build · missing context: none · next: director joins with `task/T-018-tests`.
<!-- end implementer log -->
### test author
- 2026-11-13T16:35Z · R-135 · ready · did: step 0 against base `85c4c425e6a5f9bae3e6aadec8e1166d194b6622`: slots `cursor.rs`/`eol.rs` exist and are empty; `out_of_range`, `not_a_boundary`, `no_such_document` exist; `PositionError::{OutOfBounds, InsideLineBreak}`, `EditError`, `LineEnding` exist; `doc.insert`/`delete` accept offsets inside a CRLF (mapping rule covers it). No blocking mismatch. Wording drifts (no pinned behaviour changed): `docs/protocol/` already exists (`cli-memory-seed.md`, `doc-positions.md`), not a new directory; Pinned (f) names `acceptance_t007*`, of which none exist. Tests written from the spec alone:
  AC-1 → `kiln_cli::acceptance_t018::selection_follows_random_sessions` (8 fixed-seed `control` sessions, 2,400 commands, `cursor.get` after each vs a naive model of text, typing-group history, mapping + CRLF snap, movement, collapse; every command's own result checked too) — base: assertion (`unknown_command`).
  AC-2 → `cursor_worked_examples` (all of Pinned (g) via `run`, plus extend/collapse and the line/char ends) — base: assertion.
  AC-3 → `cursor_refusals` (out_of_range / not_a_boundary, anchor before head, no_such_document for doc 0, unopened and closed ids on all five commands, schema before document before position, `cursor.get` unchanged, typing group kept after a refusal and ended by a successful `cursor.set`/`cursor.move`) — base: assertion.
  AC-4 → `eol_counts_and_normalize_match_naive_model` (24 texts × lf/crlf/cr/dominant, ties included, bytes on disk) — base: assertion.
  AC-5 → `eol_normalize_is_one_undo_step` (undo/redo round trip, one step each way, no-break text unchanged, io.crlf bytes kept) — base: assertion; guard `kiln_cli::guard_t018::open_save_and_insert_keep_line_ending_bytes` — base: pass.
  AC-6 → `kiln_text::acceptance_t018::batch_edits_and_anchors` (400 random batches vs naive last-to-first; pinned `BatchError` cases; nothing changes on error; typing-group and redo rules), plus `anchors_follow_random_edit_sequences`, `selection_and_anchor_validity`, `movement_worked_examples`, `normalize_line_endings_is_one_undo_step_and_idempotent` — base: compile.
  Generated docs → `kiln_cli::acceptance_t018::commands_json_lists_cursor_and_eol_commands` (five entries and their mutating flags, the three kinds published, `docs/protocol/{cursor,eol}.md` name their commands) — base: assertion. Pinned (f) → guard `kiln_cli::guard_t018::doc_results_carry_no_selection` — base: pass.
  Self-check: throwaway stub (kiln-text + both command groups, `commands.json` dumped from the binary) passed all 13 tests; 19 mutants, all killed: M1 bias left at an insertion (random_sessions, refusals, kiln-text batch/anchors/validity); M2 deletion start → s+n, M3 inside a deleted range → s, M4 no CRLF snap, M5 snap after `\n` (random_sessions; kiln-text batch + anchors); M6 CRLF as two chars (worked_examples, random_sessions, movement); M7 undo/redo do not map (random_sessions, refusals, anchors); M8 batch as several undo steps (worked_examples, eol undo; kiln-text batch, anchors, normalize); M9 lone CR ignored, M10 CRLF counted as CR+LF, M18 `changed` = all breaks (worked_examples, eol counts, eol undo; normalize); M11 no collapse, M12 `_` not a word char (worked_examples, random_sessions; movement); M13 head checked before anchor (refusals, random_sessions; validity); M14 refused `cursor.set` ends the typing group (refusals); M15 `cursor.move` keeps it open (refusals, random_sessions); M16 dominant ties to CRLF (worked_examples, eol counts); M17 all edit checks before order checks (batch); M19 `col` in bytes (refusals, random_sessions). Stub removed. Scorecard proposal (not edited, per director): `edit.selection` → `kiln_cli::acceptance_t018::selection_follows_random_sessions`, `kiln_cli::acceptance_t018::cursor_worked_examples`, `kiln_cli::acceptance_t018::cursor_refusals`; `io.mixed-eol` → `kiln_cli::acceptance_t018::eol_counts_and_normalize_match_naive_model`, `kiln_cli::acceptance_t018::eol_normalize_is_one_undo_step` (optionally `kiln_cli::guard_t018::open_save_and_insert_keep_line_ending_bytes`). Assumptions: a just-opened document's selection is 0,0 ("new buffer: 0,0"); a normalize that rewrites nothing adds no undo step (crate no-op rule; asserted only in the kiln-text normalize test and, for a text without breaks, via CLI where the spec says "nothing changes"). Deliberately not asserted (spec leaves open): selection/anchor positions after `eol.normalize` and after undo/redo of a batch (its primitive edits are not pinned; the kiln-text random test re-reads them there); `BatchError` when one edit is both unordered and invalid; whether `Buffer::set_selection`/`add_anchor` end a typing group (the Rust tests call `break_undo_group` to stay neutral); whether `eol.normalize` ends an open typing group. · gate attempts: 0 (`cargo xtask ci` not run: under 7 GB free, director said build only touched test targets); `cargo fmt --all -- --check` clean; clippy `-D warnings` clean on kiln-cli tests (bare) and kiln-text + kiln-cli tests (with stub) · not run: `cargo xtask ci`, `cargo xtask schema` · missing context: none · next: director ties the scorecard items; join.
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
