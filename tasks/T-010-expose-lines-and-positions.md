---
id: T-010
title: "Expose line access and position conversion as read-only document commands"
tier: medium            # ADR-0002: new commands in one command group, in-memory reads only; adding a command is not a contract change; no persistence, process, gate, security or dependency criterion
state: queued
owner_role: planner
scorecard_items: [text.lines, text.positions]
depends_on: [T-032]     # D-041: reason enum + generated kind list
touches:
  - crates/kiln-commands/src/doc.rs           # doc.line, doc.position (doc command group)
  - crates/kiln-commands/tests/positions.rs   # implementer's own integration tests
  - crates/kiln-commands/src/reason.rs        # R-060: RejectReason::NotABoundary + description + the PositionError mapping
  - docs/protocol/doc-positions.md            # NEW: doc.line / doc.position reference (D-2)
  # docs/commands.json also changes (commands + error_kinds): generated, regenerated, exempt; docs/control-protocol.md is not touched
  # docs/commands.json also changes: a generated path (exempt from `touches`, regenerated on conflict)
budget_tokens: 500000   # medium (v1.1, provisional)
iteration: 1
found_by: evaluator
origin_task: T-001
---

# T-010 Expose line access and position conversion as read-only document commands

## Goal
A script driving `kiln-cli run` or `kiln-cli control` can read the text, start, end and line
break of line N of an open document, and convert a position between char offset, byte offset,
UTF-16 offset and line/column. Today no command takes or returns any of these
(`evaluations/iter-0.md` "Not evaluated", R-050: `kiln-cli commands --schema` mentions only
`doc.info.len_lines`), so the items `text.lines` and `text.positions`, implemented in
`kiln-text` by T-001, could not be confirmed. The `kiln-text` functions exist
(`Buffer::line`, `line_start`, `char_to_byte`, `byte_to_char`, `char_to_utf16`,
`utf16_to_char`, `char_to_line_col`, `line_col_to_char`; `PositionError`), so this task adds two
read-only commands over them and nothing in `kiln-text`.

**Experiment arm (METHODOLOGY §6): even task number → independent test author** on
`task/T-010-tests`; the implementer does not write acceptance tests.

Reading list: `evaluations/iter-0.md` (items `text.lines`, `text.positions`, "Not evaluated");
`scorecard/items/text.lines.toml`, `text.positions.toml`; `crates/kiln-text/src/{buffer,position}.rs`
(the functions above and `PositionError`'s `Display`); `crates/kiln-commands/src/doc.rs`
(how a doc command is written, `no open document` refusals); `docs/development.md` "Adding a
command"; `crates/kiln-commands/src/reason.rs` and `error_kinds` of the generated `docs/commands.json` (T-032).

## Interface

### `doc.line` (read-only, `mutating: false`)
Params `{"doc": <integer ≥ 1>, "line": <integer ≥ 0>}`, no other keys. Result:
```json
{"line": 1, "text": "bé😀", "start": 3, "end": 6, "ending": "cr"}
```
- `text`: the line's content without its line break (`Buffer::line`);
- `start`: char offset of the line's first char (`Buffer::line_start`); `end` = `start` +
  chars in `text`;
- `ending`: the line break after `text`: `"lf"`, `"crlf"`, `"cr"`, or `"none"` for the last line.
  Lines are separated by `\r\n`, `\r` or `\n` (a `\r\n` is one break), so a text with k breaks
  has k + 1 lines, as `doc.info.len_lines` reports; the last line may be empty.

### `doc.position` (read-only)
Params `{"doc": <integer ≥ 1>, "at": <position>}` where `<position>` is exactly one of
`{"char": n}`, `{"byte": n}`, `{"utf16": n}`, `{"line_col": {"line": l, "col": c}}` (integers
≥ 0; `col` counts chars). Result: every form of the same position:
```json
{"char": 5, "byte": 6, "utf16": 5, "line": 1, "col": 2}
```
A position is **valid** iff it is a char offset `0..=len_chars` that is not between the `\r` and
`\n` of a CRLF; it is given in a unit that lands exactly on it (a byte offset on a char boundary,
a UTF-16 offset not inside a surrogate pair, a column within the line's content). Conversions
are exact inverses on valid positions.

### Refusals (both commands; `rejected`, nothing changes, the session log is unchanged)
Message `invalid position: <PositionError Display>` for `doc.position`, `invalid line: <PositionError Display>`
for `doc.line`, where `<PositionError Display>` is `kiln-text`'s text exactly:

| `PositionError` | kind | example message (text `"a\r\nbé😀\rc\n"`) |
|---|---|---|
| `OutOfBounds` | `out_of_range` | `invalid position: index 14 is out of bounds (length 13)` (`{"byte":14}`) |
| `LineOutOfBounds` | `out_of_range` | `invalid line: line 4 is out of bounds (buffer has 4 lines)` |
| `ColumnOutOfBounds` | `out_of_range` | `invalid position: column 4 is out of bounds for line 1 (3 chars)` |
| `NotCharBoundary` | `not_a_boundary` | `invalid position: byte offset 5 is inside a multi-byte char` |
| `InsideSurrogatePair` | `not_a_boundary` | `invalid position: UTF-16 offset 6 is inside a surrogate pair` |
| `InsideLineBreak` | `not_a_boundary` | `invalid position: char index 2 is between the \r and \n of a CRLF` (literal backslashes) |

`(rejected, not_a_boundary)` is a new pair in the specified kind set (approved, Decisions D-1); `out_of_range`'s "When" cell widens to "an offset, line or column beyond the end".
An unknown `doc` id is `no_such_document` as for every doc command. Params that do not match the
schema (two keys in `at`, a negative number, an unknown key) are `invalid_params`/`schema`.

### Reasons (through T-032's mechanism)
In `crates/kiln-commands/src/reason.rs`: add `RejectReason::NotABoundary` (kind
`not_a_boundary`, description "a position inside a character, a surrogate pair or a CRLF line
break") and an exhaustive `impl From<&PositionError> for RejectReason`: `OutOfBounds`, `LineOutOfBounds`,
`ColumnOutOfBounds` → `OutOfRange` (whose description widens to "an offset, line or column
beyond the end"); `NotCharBoundary`, `InsideSurrogatePair`, `InsideLineBreak` → `NotABoundary`.
Regenerate `docs/commands.json`. `kiln-control` is not touched: kinds come from
reasons.

## Pinned
- (a) Errors, payloads, precedence: params schema (`invalid_params`) → document id
  (`no_such_document`) → position or line check (table above). Messages and kinds exact as in
  the table.
- (b) Files: none read or written by the commands (in-memory document); `docs/commands.json`
  gains the two commands (≥ 1 example each, one of them on the CRLF/astral text above);
  new `docs/protocol/doc-positions.md` documents both commands (params, results, the validity
  rule, the refusal table); `docs/commands.json` is regenerated (new kind, widened
  `out_of_range` description); `docs/control-protocol.md` is not touched. Acceptance tests in
  `apps/kiln-cli/tests/acceptance_t010.rs` (CLI) and optionally
  `crates/kiln-commands/tests/acceptance_t010_api.rs` (distinct target names).
- (c) Order of checks: as (a). Both commands are read-only: they never end a typing group or
  touch history (T-007's read-only property, if merged, checks this on their examples).
- (d) Path base, platform, file types: n/a (no path parameter; text already decoded).
- (e) Threat model: n/a (no gate, security or confinement path). Hostile input is huge integers:
  any `u64` value is accepted by the schema and answered with `out_of_range`, never a panic.
- (f) Tests pinning a snapshot this changes: the gate's `schema` step on `docs/commands.json`
  (kept green by regenerating it); no test pins the command count or list (searched `crates`, `apps` for name lists:
  only `contains` checks); `docs/commands.json` is regenerated.
- (g) Worked examples, executed against a naive Python model in R-055 (`re.finditer(r"\r\n|\r|\n")`
  split; `len(t[:c].encode())`, `len(t[:c].encode('utf-16-le'))//2`) on `t = "a\r\nbé😀\rc\n"`
  (len_chars 9, len_bytes 13, len_utf16 10, len_lines 4):
  lines 0…3 = `{"text":"a","start":0,"end":1,"ending":"crlf"}`,
  `{"text":"bé😀","start":3,"end":6,"ending":"cr"}`, `{"text":"c","start":7,"end":8,"ending":"lf"}`,
  `{"text":"","start":9,"end":9,"ending":"none"}`;
  positions char→(byte, utf16, line, col): 0→(0,0,0,0), 1→(1,1,0,1), 2→invalid (inside CRLF),
  3→(3,3,1,0), 4→(4,4,1,1), 5→(6,5,1,2), 6→(10,7,1,3), 7→(11,8,2,0), 8→(12,9,2,1), 9→(13,10,3,0).
  Invalid: byte 5 and 7 (inside `é`, `😀`), utf16 6 (inside the pair), byte 14, line 4,
  `line_col {1,4}`.
- (h) Per item, the evaluator's call: `text.lines` — `kiln-cli run` with `doc.open` of a
  CRLF/CR/LF file and `doc.line` for every line `0..len_lines`, compared with a naive split;
  `text.positions` — `kiln-cli run` with `doc.position` from each of the four forms for every
  position, compared with a naive model, plus the six refusals.

## Acceptance criteria
Through `kiln-cli run --root <dir>` (or `control`), on files the tests write. Oracle: a naive
model in the test (split on `\r\n|\r|\n`; UTF-8 and UTF-16 lengths of prefixes) unless stated.
- AC-1 (`text.lines`, CLI): for fixed-seed random texts (≥ 50) over `a é 😀 中 \n \r\n \r`
  saved as UTF-8 files, a script opens each and asks `doc.line` for every line
  `0..len_lines`; every result equals the naive split's `text`, `start`, `end`, `ending`, and
  `doc.info.len_lines` equals the number of lines.
- AC-2 (`text.positions`, CLI): for the same texts, `doc.position` from `{"char":c}` for every
  valid `c` returns the naive model's five numbers; from each returned `byte`, `utf16` and
  `line_col`, `doc.position` returns the same five numbers (round trip).
- AC-3 (refusals, CLI): on the worked-example text, the six refusals of the table give exactly
  the listed kind and message, `ok:false`, and a following `doc.text` shows the text unchanged;
  `{"char":2}` (inside the CRLF) → `not_a_boundary`; `doc.line` on a closed or unknown id →
  `no_such_document`; `{"at":{"char":1,"byte":1}}` and `{"line":-1}` → `invalid_params`.
  Oracle: exact value.
- AC-4 (read-only and logged): after `doc.line`/`doc.position` requests, `doc.info` shows the
  same `can_undo`/`can_redo`, and a typing group open before them still joins the next
  single-char insert (one `doc.undo` removes both chars); `run --log` lists the successful
  `doc.line`/`doc.position` requests with `mutating: false`. Oracle: exact value.
- AC-5 (schema): `kiln-cli commands --schema` lists both commands with params and result
  schemas and ≥ 1 example each, and `kiln-cli commands --audit` exits 0. Oracle: exact value.
- AC-6 (documented kind): `error_kinds` of the generated `docs/commands.json` lists
  `not_a_boundary`, the `schema` step stays green, `docs/protocol/doc-positions.md`
  exists, and `docs/control-protocol.md` is unchanged. Oracle: exact value.

## Out of scope and implementer notes
- Cursor and selection (`edit.selection`), goto-line (`nav.goto-line`), any change in
  `kiln-text` (if one is needed, stop and report), positions on the scratch buffer
  (`buffer.*`).
- Structured reasons and the generated kind list: T-032.
- Your tests: unit tests in `doc.rs`, integration tests in `crates/kiln-commands/tests/positions.rs`.
  Determinism: pure functions of the document text.
- Specified: names, params, results, refusals, kinds. Left to you: how params are typed, as long
  as the published schema accepts exactly the forms above.
- Dispatch (R-060, replaces R-057): after T-015. `doc.rs` overlaps T-006, T-007, T-027;
  `reason.rs` overlaps T-008, T-027. May run in parallel with T-009, T-011, T-013 and the
  feature tasks that touch neither file.

## Decisions
- D-1 (director R-057, was Q-1): the error kind `not_a_boundary` is approved.
- D-2 (director R-057, all product tasks): a task documents its commands in its own new file
  under `docs/protocol/` (here `doc-positions.md`, a per-task file of the `doc` group so later
  `doc.*` tasks do not share it) and touches `docs/control-protocol.md` only to add an
  error-kind line.

## Questions and rulings
Director rulings on the builders' pre-code checks (2026-11-13, decision D-059; questions from the test author R-108 and the implementer R-107, answered to both at once, each checked against the code on main `7dccc49`). Where a ruling differs from text above, the ruling governs.
- q1 (AC-4). The `--log` file has no `mutating` field (`Host::log_script` writes `{"command","id","params"}` only) and `kiln-control` is not in `touches`. AC-4 reads: after `doc.line`/`doc.position` requests `doc.info` shows the document unchanged; `run --log` lists the successful requests as script lines; `kiln-cli commands --schema` shows `"mutating": false` for both commands; and the registry's session log (`Context::log()`) records them with `mutating: false`.
- q2 (`out_of_range` description). The widening to "an offset, line or column beyond the end" is dropped: the `out_of_range` row of the reason table is not edited by this task, and no test asserts a new description for it. Three merged tests pin today's text, and the wording is not worth changing a T-032 contract test. `docs/protocol/doc-positions.md` states that a line or column beyond the end is refused with `out_of_range`. The task adds exactly one row to the table: `NotABoundary`.
- q3 (stale sentence). D-2's remark that the task adds an error-kind line to `docs/control-protocol.md` is struck; that file is not touched (Pinned (b), AC-6).
- q4 (`doc` parameter). `doc` has the same schema as in the existing doc commands (an unsigned integer, as `doc.info` takes it); `doc: 0` is refused with `no_such_document`, not `invalid_params`.
- q5 (existing unit test). The unit test `describe_lists_doc_commands_with_mutating_flag` in `crates/kiln-commands/src/doc.rs` is updated by the implementer to include the two commands; its hard-coded number of doc commands becomes a check that the expected names are present, so later tasks do not edit it (logged under golden rule 4).
Director rulings after review round 1 (2026-11-13, decision D-064; review `reviews/T-010-r1.md`, run R-113: no defect, one test gap, two nits):
- t1 (test gap, test author). "Document id before position" for `doc.position` is tested with an out-of-range value in every unit the command accepts (`char`, `byte`, `utf16`, `line_col`), each with an unknown document id, through the API and the CLI; the reviewer's surviving mutant (a pre-check of a huge `char` value before the id lookup) must be caught.
- t2 (nit, test author). The guard at `guard_t010.rs:164` that pins the `out_of_range` description is removed: the T-032 acceptance tests already pin that text, and "this task left the row alone" is a diff check the reviewer made (D-052).
- t3 (nit). The repeated bound check near `doc.rs:959` stays; removing it changes no behaviour and is not worth an implementer round.
- t4 (scorecard). The director ties `text.lines` and `text.positions` to the task's CLI-level acceptance tests on the task branch after the re-join (METHODOLOGY §5: `scorecard/` is the director's at join) and regenerates `docs/scorecard.md` there.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13T03:50Z · R-055 · draft · did: turned the evaluator's defect report into a spec:
  two read-only commands `doc.line` and `doc.position` over the existing `kiln-text` functions,
  exact refusals from `PositionError`'s `Display`, one new kind `not_a_boundary` (Q-1). Worked
  example computed with a naive Python model. Tier medium under ADR-0002 (criterion: a new
  command inside one group, no boundary; "adding a command is not a contract change"); budget
  500k; arm independent (even number). `depends_on` T-004 (merged) only; ordering with T-006,
  T-007, T-008, T-030 is a dispatch constraint (shared files), and T-030 now depends on T-010.
  Pinned (d), (e) n/a. Unsure: whether adding a kind to the protocol's specified set needs more
  than the director's approval (T-008 D-1 set the precedent). · gate attempts: 0 · not run: the
  commands (not built) · missing context: none · next: director answers Q-1.
- 2026-11-13T03:56Z · R-057 · draft · did: applied D-1 (kind approved) and D-2 (reference text in
  new `docs/protocol/doc-positions.md`; protocol-document edits limited to the kind lines;
  generated `docs/commands.json` out of `touches`); dispatch note rewritten. · gate attempts: 0
  · missing context: none · next: director review.
- 2026-11-13 · R-060 · draft · did: contracts-first restructuring: depends on T-015 (was T-004); `not_a_boundary` and the `PositionError` mapping go into T-015's `reason.rs` and the generated kind list, so `crates/kiln-control/src/lib.rs` and `docs/control-protocol.md` left `touches`. Text elsewhere that names `rejected_kind` or `ERROR_KINDS` is superseded. Shares `reason.rs` with T-008 and T-027 and `doc.rs` with T-007 and T-027: never in flight with them. T-018 depends on this task (`not_a_boundary`). · gate attempts: 0 · missing context: none · next: director.
- 2026-11-13 · R-066 · draft · did: retargeted to T-032 (D-041): `depends_on` T-032; the list is `error_kinds` in `docs/commands.json`; the mapping is a `From` impl (finding 12). · gate attempts: 0 · missing context: none · next: director.
<!-- end planner log -->
### implementer
- 2026-11-13T12:29Z · R-107 · ready · BLOCKED: spec-vs-merged-code mismatch found in the pre-code
  check; no code written. Criteria restated: two read-only `doc.*` commands over `kiln-text`'s
  line/position functions with the exact result shapes and the six-row refusal table; one new
  reason row `not_a_boundary` + `From<&PositionError>`; `out_of_range`'s description widened;
  `docs/commands.json` regenerated; new `docs/protocol/doc-positions.md`; control-protocol.md
  untouched. The widening of `out_of_range` ("an offset, line or column beyond the end",
  Interface "Reasons" and the refusal-table note) cannot be made without breaking three existing
  tests that pin the T-032 text "an offset beyond the end of the text" (base 7dccc49):
  `crates/kiln-commands/tests/acceptance_t032.rs` (REJECTED row at :757-761, asserted in
  `reasons_have_pinned_names_order_and_descriptions` :848, `reason.description() == when`);
  `apps/kiln-cli/tests/acceptance_t032_cli.rs` (REJECTED :87, pair subsequence check in
  `schema_dump_publishes_the_generated_kind_list` :207); `crates/kiln-commands/tests/reasons.rs`
  `table_rows_are_published_kinds_with_their_when_text` (:209, requires the description to equal
  the "When" cell of `docs/control-protocol.md`:98, a file the spec says is not touched and
  whose own text says "later tasks do not edit table cells"). Pinned (f) says only the `schema`
  step pins anything this changes; it does not list these. The new `NotABoundary` row alone
  breaks none of them (all use ordered-subsequence checks; reasons.rs's 13-row count is about
  the protocol table, which is not edited). Wording drifts noted, would proceed: (1)
  `doc.rs` unit test `describe_lists_doc_commands_with_mutating_flag` pins `doc.len() == 11` and a
  read-only name list (in `touches`; contradicts (f) "no test pins the command count"); I would
  update it to include the two commands. (2) `doc: <integer ≥ 1>` while every doc command takes
  `u64` (`DocParams`); I would keep `u64`, so doc 0 is `no_such_document`.
  QUESTION: keep `out_of_range`'s description unchanged (drop the widening), or authorise edits
  to the three tests and the control-protocol.md cell (outside `touches`, and two are
  `acceptance_t*` files)?
  QUESTION: `doc` minimum 1 in the schema (doc 0 → `invalid_params`) or `u64` like other doc
  commands (doc 0 → `no_such_document`)? · gate attempts: 0 · not run: everything (no code) ·
  missing context: none · next: director rules on the two questions.
- 2026-11-13T12:39Z · R-107 (continued after rulings q1–q5, D-059, read from main's task file)
  · ready · did: `reason.rs`: one row `NotABoundary` ("not_a_boundary", spec description)
  appended after `InvalidKeymap`; `out_of_range` row untouched (q2); exhaustive
  `impl From<&PositionError> for RejectReason` (3 → OutOfRange, 3 → NotABoundary) + unit test.
  `doc.rs`: `doc.line` (`LineParams {doc: u64, line: u64}`, result line/text/start/end/ending,
  ending from the gap to the next line start) and `doc.position` (externally tagged `At` enum
  char|byte|utf16|line_col, schema `oneOf` with `additionalProperties: false`; every form is
  converted to a char index, then `char_to_line_col` refuses a CRLF interior in any unit, e.g.
  `{"byte":2}`), both `MUTATING = false`, 2 + 3 examples on `"a\r\nbé😀\rc\n"`; messages
  `invalid line: ` / `invalid position: ` + `PositionError` Display; `doc` typed `u64` as in
  `doc.info` (q4: doc 0 → `no_such_document`); u64 params saturate to `usize`. Golden rule 4 /
  q5: unit test `describe_lists_doc_commands_with_mutating_flag` — hard-coded `doc.len() == 11`
  replaced by a check that the 13 expected `doc.*` names are present; read-only list gains
  `doc.line`, `doc.position`. New `tests/positions.rs` (7 tests: worked example in all four
  units, 64 fixed-seed texts vs naive split/prefix model with round trips, the refusal table +
  extra cases with exact messages and unchanged info/text/log, u64::MAX → out_of_range,
  precedence invalid_params → no_such_document (incl. doc 0, closed doc) → position, read-only +
  `Context::log()` `mutating: false` + typing group kept, describe/new kind/audit). New
  `docs/protocol/doc-positions.md` (states line/column beyond the end → `out_of_range`, q2).
  `docs/commands.json` regenerated (`cargo xtask schema --write`); `docs/control-protocol.md`
  and `kiln-control` untouched (q1, q3). Checked from outside with `kiln-cli run --root --log`
  on the worked-example file (line 1, `{"char":2}` → not_a_boundary, two keys → invalid_params,
  doc 0 → no_such_document, byte u64::MAX → out_of_range, `--log` lists the successful
  requests) and `kiln-cli commands --audit` (exit 0). · gate attempts: 3 (1 red fmt, 2 red
  clippy `double_ended_iterator_last` in my test, 3 green: 11 steps, fuzz/budgets EMPTY) ·
  not run: 32-bit build (saturation would change the quoted index there); T-007 read-only
  property (not merged) · missing context: none · next: join with test author.
<!-- end implementer log -->
### test author
- 2026-11-13T12:41Z · R-108 · ready · did: pre-code check against base `7dccc49c655b149f210f86004096dac5182fb3b2` found AC-4's `--log … mutating: false` untestable (log script has no `mutating`); answered by D-059 q1–q4 on main, which these tests follow (q2: no widened `out_of_range` text asserted; q3: `docs/control-protocol.md` unchanged left to the reviewer's diff check, not hashed; q4: `doc: 0` → `no_such_document`). Files: `apps/kiln-cli/tests/acceptance_t010.rs` (CLI), `crates/kiln-commands/tests/acceptance_t010_api.rs` (registry API, compiles on base), `apps/kiln-cli/tests/guard_t010.rs`. Oracle: naive split / UTF-8 / UTF-16 prefix model built in the tests; corpus = 13 edge texts + 60 fixed-seed random texts over `a é 😀 中 \n \r\n \r`. Criterion → tests → base result:
  AC-1 → `worked_example_lines_are_exact`, `doc_line_matches_naive_split_for_every_line`, api `exhaustive_lines_and_positions_on_small_texts` → assertion (`unknown_command`).
  AC-2 → `doc_position_round_trips_every_valid_position` (char → five numbers, then byte/utf16/line_col from the returned values, plus every offset of every unit to one past the end against the model), `worked_example_positions_are_exact`, api `every_returned_form_converts_back_to_the_same_position`, api exhaustive → assertion.
  AC-3 → `refusals_have_exact_kinds_and_messages_and_change_nothing` (six table rows exact, `{"char":2}`, `u64::MAX` in every field → `out_of_range`, ids 0/closed/unknown, 20 schema mismatches, precedence, `doc.text` and `--log` unchanged), api `ids_params_and_precedence` → assertion.
  AC-4 (q1) → `line_and_position_are_read_only_and_logged` (doc.info identical, one undo removes `xy`, redo kept, `--log` lines `{command,id,params}`), api exhaustive (`Context::log()` entry `mutating: false`, refusals not logged), `schema_lists_…` (`mutating: false`) → assertion.
  AC-5 → `schema_lists_both_commands_with_examples_and_audit_passes` (both listed, params/result keys, ≥1 example, one on the worked text, single-file examples checked against the model, `--audit` 0) → assertion.
  AC-6 → `not_a_boundary_is_a_generated_kind_and_documented` (kind + pinned description in dump and committed file, committed entries of the two commands equal the generated ones, `docs/protocol/doc-positions.md` exists and names both commands, both kinds, `line_col`, `utf16`, `crlf`), api `not_a_boundary_reason_is_declared_and_published` → assertion.
  Guards (pass on base): `doc_info_len_lines_counts_every_break_kind_once`, `read_only_commands_do_not_end_a_typing_group`, `out_of_range_keeps_its_description` (q2).
  Self-check: throwaway stub (both commands, `NotABoundary`, `From<&PositionError>`, regenerated `docs/commands.json`, stub doc) passed all 15 tests; 15 mutants, 15 killed: end+1 (AC-1 ×3, AC-5), CRLF→lf (AC-1 ×3), mid-CRLF accepted (AC-3, AC-2 ×2), surrogate → `out_of_range` (AC-3, AC-2 ×2), byte/utf16 swapped (AC-2 ×4, AC-5), byte read as char (AC-2 ×4, AC-3), `doc.line` message prefix (AC-3, api), `doc.position` mutating (AC-5, AC-6, api ×2), `line` field 0 (AC-1 ×3, AC-5), `doc.line` breaks typing group (AC-4), utf16 rounds down (AC-3, AC-2 ×2), panic on huge char (AC-3, api), kind description reworded (AC-6, api), last line ending `cr` (AC-1 ×3), column past end accepted (AC-3, AC-2 ×2). Stub removed; fmt/clippy (`--workspace --all-targets -D warnings`) clean with and without it.
  Scorecard: `scorecard/` is outside what this run may change; proposed additions to `tests` — `text.lines`: `kiln_cli::acceptance_t010::doc_line_matches_naive_split_for_every_line`, `kiln_cli::acceptance_t010::worked_example_lines_are_exact`; `text.positions`: `kiln_cli::acceptance_t010::doc_position_round_trips_every_valid_position`, `kiln_cli::acceptance_t010::refusals_have_exact_kinds_and_messages_and_change_nothing`. · gate attempts: 1 (last: red test, only in `acceptance_t010`; fmt, clippy and earlier steps green) · not run: later gate steps (stopped after test); precedence where the position check runs before the id check has no mutant · missing context: none · next: join with `task/T-010`.
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
