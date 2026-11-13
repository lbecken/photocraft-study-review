---
id: T-006
title: "State the D-013 and D-023 spec-gap rulings in the public docs and pin each with a regression guard"
tier: low               # docs + guard tests only; see Q-1 (paths in two layers)
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-001, T-003]
touches:
  - crates/kiln-text/src/lib.rs          # crate docs (undo grouping section) only
  - crates/kiln-text/src/buffer.rs       # doc comments only (`EditError`, `insert`, `delete`, `replace`, `undo`, `redo`)
  - crates/kiln-text/tests/guard_t006.rs
  - crates/kiln-commands/src/doc.rs      # module doc (`//!`) only; NOT the `DOC` constants
  - crates/kiln-commands/src/registry.rs # doc comment of `Command::MUTATING` only
  - crates/kiln-commands/tests/guard_t006.rs
budget_tokens: 150000
iteration: 1            # see Q-3
found_by: reviewer
origin_task: T-001      # also T-003; see Q-3
---

# T-006 State the D-013 and D-023 spec-gap rulings in the public docs and pin each with a regression guard

## Goal
Six rulings the director made after the code was written (D-013 for the text core, D-023 for
the document commands) hold today only because the code happens to behave that way: no
document states them and no test would fail if they were broken. After this task, each ruling
is stated where a caller reads the API (rustdoc of `kiln-text` and `kiln-commands`), and each
has a named test that fails if the behaviour changes. No behaviour changes.

## Reading list
- `docs/decisions.md` D-010 (guards), D-013, D-023.
- `reviews/T-001-r1.md` findings 1–3 (exact probe values); `reviews/T-003-r1.md` SG-4…SG-6.
- `tasks/T-001-text-core.md` Interface (Buffer, `EditError`, grouping rule, no-op edits);
  `tasks/T-003-document-commands.md` Interface (doc commands, `LogEntry`, `MUTATING`, D-1).
- `crates/kiln-text/src/lib.rs` (the "Undo grouping" crate doc already states part of SG-2).

## Interface
Unchanged. No signature, behaviour, schema, `Command::DOC` text, example or
`docs/commands.json` byte changes. Only rustdoc comments and two new guard test files.

The six rulings, each to be stated in the named doc and pinned by one guard test:

| # | Ruling (source) | Where it is stated |
|---|---|---|
| R1 | Edit validation order is: inverted range first, then `start`, then `end`. `delete`/`replace` of a range with `start > end` returns `InvertedRange{start,end}` even when either bound is also out of range; otherwise the first of `start`, `end` that exceeds `len_chars()` is reported as `OutOfBounds{index, len_chars}`. (D-013 SG-1) | `EditError` and `delete`/`replace` doc comments |
| R2 | A call that returns `Err` (any edit) or `false` (`redo`) changes nothing, including an open typing group: the next qualifying single-char insert still joins the group. (`undo()` cannot return `false` while a group is open, because an open group is itself undoable; the docs say so rather than imply a testable case.) (D-013 SG-2) | crate doc "Undo grouping" and `undo`/`redo` doc comments |
| R3 | An edit that would be a no-op (`insert` of `""`, `delete`/`replace` of an empty range with `""`) at an index beyond `len_chars()` is `Err(OutOfBounds{index, len_chars})`, not a no-op `Ok`: indices are validated before emptiness is considered. (D-013 SG-3) | `insert`/`delete`/`replace` doc comments |
| R4 | `MUTATING` is a property of the command, not of one invocation: `doc.open` is mutating (`true`), also for a re-open that returns `already_open: true` and changes nothing. (D-023 SG-4) | `Command::MUTATING` doc comment; `doc.rs` module doc |
| R5 | A document id is assigned only by a successful first open of a path. A failed `doc.open` (invalid path, missing file, undecodable file) consumes no id, and a re-open of an open path returns the existing id. (D-023 SG-5) | `doc.rs` module doc |
| R6 | `doc.save` changes undo/redo history only by ending the current typing group: an available redo step stays available after a save. (D-023 SG-6) | `doc.rs` module doc |

## Acceptance criteria
Low tier: the implementer writes these tests. They are **regression guards** (D-010): each
passes on today's `main`, so they live in `guard_t006.rs` files, not `acceptance_t006*.rs`.
Oracle for every criterion: exact value.

- AC-1 (R1), `kiln_text::guard_t006::edit_error_precedence_is_inverted_then_start_then_end`:
  on `Buffer::from_text("abc")`: `delete(9..5)` → `Err(InvertedRange{start:9,end:5})`;
  `delete(5..4)` → `Err(InvertedRange{start:5,end:4})`; `delete(4..9)` →
  `Err(OutOfBounds{index:4,len_chars:3})`; `delete(1..9)` → `Err(OutOfBounds{index:9,len_chars:3})`;
  the same four for `replace(range, "x")`; text still `"abc"` and `can_undo()` false after all.
- AC-2 (R2), `kiln_text::guard_t006::rejected_edit_or_failed_redo_keeps_typing_group_open`: for
  each interrupting call X in {`insert(99,"x")`, `delete(5..9)`, `delete(2..1)`,
  `replace(7..7,"y")`, `redo()` (returns `false`)}, on a fresh `Buffer::new()`:
  `insert(0,"a")`, `insert(1,"b")`, X (asserting it returns `Err` / `false`), `insert(2,"c")`,
  then one `undo()` → `text() == ""` and `can_undo() == false`. Control case in the same test:
  with `break_undo_group()` as X, one `undo()` leaves `"ab"` (shows the probe can tell the two
  apart).
- AC-3 (R3), `kiln_text::guard_t006::empty_edit_out_of_range_is_out_of_bounds`: on
  `Buffer::from_text("abc")`: `insert(9,"")`, `delete(9..9)`, `replace(9..9,"")` each →
  `Err(OutOfBounds{index:9,len_chars:3})`; `insert(3,"")` and `delete(3..3)` → `Ok` (boundary);
  text `"abc"`, `can_undo()` false.
- AC-4 (R4), `kiln_commands::guard_t006::doc_open_is_mutating_even_for_a_reopen`: in
  `standard_registry().describe()`, `doc.open` has `"mutating": true`; on
  `Context::with_files(&[("a.txt", b"hello\n")])`, two `doc.open {"path":"a.txt"}` →
  second result has `already_open: true`, and both `ctx.log()` entries have `mutating == true`.
- AC-5 (R5), `kiln_commands::guard_t006::only_a_successful_first_open_assigns_an_id`: on
  `Context::with_files(&[("a.txt", b"hello\n"), ("b.txt", b"b\n"), ("bad.txt", b"\xff")])`:
  `doc.open` of `"missing.txt"`, `"bad.txt"`, `"../x"` → each `rejected`; then `a.txt` →
  `doc: 1, already_open: false`; `a.txt` again → `doc: 1, already_open: true`; `b.txt` →
  `doc: 2`; `doc.list` → exactly docs 1 and 2.
- AC-6 (R6), `kiln_commands::guard_t006::save_keeps_redo_available`: on
  `Context::with_files(&[("a.txt", b"x")])`: open (doc 1); `doc.insert {doc:1,offset:1,text:"a"}`;
  `doc.undo` → `undone: true`; `doc.info` → `can_redo: true`; `doc.save` → `bytes_written: 1`;
  `doc.info` → `can_redo: true, dirty: false`; `doc.redo` → `{"redone":true,"len_chars":2,"dirty":true}`;
  `doc.text` → `"xa"`; the file still holds `b"x"`.
- AC-7 (docs): each ruling R1–R6 is stated in the place named in the Interface table, in plain
  words a caller can act on (checked by the director at `self_verified`; no test).
- AC-8 (guards are not vacuous): for each of AC-1…AC-6 the implementer seeds one bug that breaks
  that ruling (for example: check `start` before inversion; let a failed `redo` end the group;
  short-circuit empty edits before validation; make `doc.open` read-only; assign the id before
  opening; clear redo in save), runs the guard, sees it fail, restores the code, and records
  the six seeded bugs and results in the implementer log. The seeded bugs are not committed.

## Out of scope
- Any behaviour change, including tightening `kiln-text`'s private `check_range` or any
  `acceptance_t001`/`acceptance_t003` file (another task's acceptance tests are not edited).
- Changing a `Command::DOC` string (that changes `docs/commands.json`, a schema path).
- The T-004 follow-ups that D-032 assigned to T-006 (4096-byte boundary test, `id` excluded
  from the truncation decision, misplaced sentence in `docs/control-protocol.md`): proposed for
  T-007, which already touches those files; see Q-2.
- `docs/development.md` "Adding a command" — a separate docs follow-up.

## Implementer notes
- Use only the public API (`kiln_text::*`, `kiln_commands::{standard_registry, Context, …}`).
- Keep to doc comments in the source files; `cargo xtask schema` must stay green without
  `--write` (proves `docs/commands.json` did not change).
- Determinism: no clock or randomness; fixed inputs only.

## Questions
- Q-1 (planner, R-049): CHARTER §4 lists "anything in two or more layers" as high; this task
  touches L0 (`kiln-text`) and L3 (`kiln-commands`), but only rustdoc and guard tests, which is
  the low row ("docs, tests"). I propose low. Alternatives: rule high, or split into two low
  tasks (one per crate).
- Q-2 (planner, R-049): D-032 sends three T-004 follow-ups to T-006. Two of them need a code
  change in `kiln-control` and an edit to `docs/control-protocol.md`, which would make this task
  medium/high, overlap T-007 and T-008, and remove its parallelism. I moved them to T-007 (part
  e). Confirm, or tell me to move them back here and raise the tier.
- Q-3 (planner, R-049): `iteration: 1` assumes these follow-ups belong to the next iteration
  (iteration 0 is closing); `origin_task` takes one id, so T-001 is named and T-003 noted.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · tokens (if known) · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13T02:45Z · R-049 · (new) · did: specified the six D-013/D-023 rulings as doc text
  plus one regression guard each (`guard_t006.rs` in `kiln-text` and `kiln-commands`, D-010),
  with exact values from the T-001 and T-003 reviews' probes and a seeded-bug check per guard
  so the guards are shown to be able to fail. Read the code: each ruling holds on today's main
  (`kiln-text` `check_range`/`check_index` order, early-return validation, history untouched on
  `Err`/`false`; `doc.rs` assigns the id after a successful open; save calls only
  `break_undo_group`), so these are guards, not acceptance tests. Found that `undo()` cannot
  return `false` while a group is open, so R2 is testable only for `redo()`; stated it. Tier
  low: rustdoc and tests only, but two layers (Q-1). D-032 follow-ups moved to T-007 (Q-2).
  Kept `Command::DOC` strings out so `docs/commands.json` does not change. · gate attempts: 0 ·
  not run: gate, the guard tests (no code written) · missing context: none · next: director
  answers Q-1, Q-2, Q-3.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
