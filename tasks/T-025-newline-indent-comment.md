---
id: T-025
title: "Insert a newline with the document's line ending and the current indentation, indent and outdent the selected lines, and toggle `//` comments on them, each as one undo step"
tier: medium            # ADR-0002 default: in-memory editing commands in one new group
state: draft
owner_role: planner
scorecard_items: [edit.newline, edit.indent, edit.comment-toggle]   # edit.newline: new item, created by the test author (R-060 answer 6)
depends_on: [T-018]     # selection + apply_edits (T-015 transitively)
touches:
  - crates/kiln-commands/src/edit.rs        # edit.* (slot from T-015)
  - crates/kiln-commands/tests/edit.rs      # implementer's own integration tests
  - docs/protocol/edit.md                   # new
  - docs/commands.json                      # generated
budget_tokens: 500000   # medium (v1.1, provisional)
iteration: 1
found_by: planner
origin_task: null
---

# T-025 Newline, indent, outdent and comment toggling

## Goal
The evaluator found that a newline typed into a CRLF file through `doc.insert` is written as
LF, which leaves the file mixed (`evaluations/iter-0.md`, io.crlf). For file I/O,
bytes-as-read stays the rule: `doc.insert` inserts exactly what it is given. After this task
the editor's Enter key has its own command, `edit.newline`. It inserts **the document's line
ending** followed by the current line's indentation. Further commands shift the selected lines
by one indentation level (`edit.indent`, `edit.outdent`) and toggle `//` line comments on them
(`edit.toggle-comment`). Each command is one undo step, made with `Buffer::apply_edits`, and
moves the selection by T-018's mapping rule.

No gate path. Arm: **odd** (the implementer writes the acceptance tests first on
`task/T-025-tests`).

Reading list: `tasks/T-018-*.md` (selection, mapping rule, `apply_edits`); `crates/kiln-text/src/line_ending.rs`
(dominant line ending, ties go to LF, then CRLF, then CR); `evaluations/iter-0.md` "io.crlf".

## Interface
All four take `{doc}` (an open document; else `no_such_document`), are `mutating: true`, and
return `{doc, len_chars, changed_lines, cursor: CursorResult}`, where `changed_lines` is the
number of lines edited (for `edit.newline`, 1). There are no other refusals and no new kind.

### Definitions (exact)
- Lines split on LF, CRLF and lone CR; "content" is a line without its break.
- **Selected lines**: from the line of `selection.start` to the line of `selection.end`. If
  the selection is non-empty, ends at column 0 and spans more than one line, its last line is
  excluded.
- **Leading whitespace** of a line: its longest prefix of `' '` and `'\t'`.
- **Blank line**: content that is empty or all leading whitespace.
- **Document line ending**: `doc.info.line_ending` of the current text (dominant kind); LF
  when the text has no line break.

### Commands
- `edit.newline`: replaces the selection with the document line ending, followed by the
  leading whitespace of the line containing `selection.start`, truncated to the part before
  `selection.start`. Afterwards the caret (anchor = head) is after the inserted text.
- `edit.indent`: inserts 4 spaces at the start of every selected line whose content is not
  empty.
- `edit.outdent`: on every selected line whose content starts with `\t`, removes that tab.
  Otherwise it removes up to 4 leading spaces.
- `edit.toggle-comment`: considers the selected lines that are not blank. If every considered
  line has `//` right after its leading whitespace, it removes that `//` and one following
  space if present. Otherwise it inserts `// ` at column `m` of every considered line, where
  `m` is the minimum leading-whitespace length (in chars) of the considered lines. With no
  considered line it changes nothing.

A command that changes nothing (e.g. outdent with no leading whitespace) adds no undo step.
Selections other than `edit.newline`'s map through the edits by T-018's rule.

`docs/protocol/edit.md` documents the commands and the definitions, with the examples.

## Pinned
- (a) Errors: schema → `no_such_document`. Nothing else can fail.
- (b) Files: none (the line ending reaches disk only through `doc.save`, byte-exact).
- (c) Order: line set and line ending are computed on the text before the command.
- (d) Offsets chars; indentation unit fixed at 4 spaces (no configuration in this task).
- (e) Threat model: n/a.
- (f) Snapshots: none; `doc.insert` is unchanged (io.crlf guard).
- (g) Worked examples (executed against a naive Python model):
  - `fn f() {\r\n    let x = 1;\r\n}\r\n`, caret 24 (end of line 1), `edit.newline` → text
    `fn f() {\r\n    let x = 1;\r\n    \r\n}\r\n`, caret 30. After `doc.save` the file has 4
    CRLF and no LF.
  - `ab` (no break), caret 1, `edit.newline` → `a\nb`, caret 2.
  - `fn a() {\n\n\tx\n}`, `cursor.set {anchor:0, head:13}` (13 = column 0 of line 3, so lines
    0–2), `edit.indent` → `    fn a() {\n\n    \tx\n}`, anchor 4, head 21, `changed_lines: 2`.
    Then `edit.outdent` → the original text, anchor 0, head 13.
  - `    a\n\n  b\n`, selection `0..11`, `edit.toggle-comment` → `  //   a\n\n  // b\n`. Again →
    `    a\n\n  b\n`.
  - `//x\n`, caret 0, toggle → `x\n`; toggle → `// x\n`. An uncomment-first toggle is not
    reversed exactly; this is documented, and AC-4 scopes the round trip.
- (h) Evaluator calls: `kiln-cli run` with `doc.open` of a CRLF file, `cursor.set`,
  `edit.newline`, `doc.save` and the file's bytes (`edit.newline`); `edit.indent`
  / `edit.outdent` / `doc.undo` (`edit.indent`); `edit.toggle-comment` twice (`edit.comment-toggle`).

## Acceptance criteria
All through `kiln-cli run` or `kiln_control::Host`.
- AC-1 (`edit.newline`): for files whose only line ending is LF,
  CRLF or CR, and for mixed files, `edit.newline` at random selections inserts exactly the
  dominant break (LF if none) plus the truncated indentation, per a naive model. After
  `doc.save` of an LF, CRLF or CR file edited only with `edit.newline` and `doc.insert` of
  non-break text, the file contains only that file's break kind (`eol.counts`, and the bytes).
  Oracle: naive model + bytes on disk.
- AC-2 (`edit.indent`, indent/outdent): for random texts (spaces, tabs, empty lines, CRLF) and
  random selections, `edit.indent` and `edit.outdent` give the naive model's text and
  selection. Oracle: naive model.
- AC-3 (`edit.indent`, round trip): `edit.indent` then `edit.outdent` on the same selection
  restores the exact text. Each is one undo step: one `doc.undo` reverts it. Oracle: round
  trip.
- AC-4 (`edit.comment-toggle`): for random Rust-like texts and selections, the result equals
  the naive model. When the first toggle comments, a second toggle restores the text exactly.
  One `doc.undo` reverts a toggle. Oracle: naive model + round trip.
- AC-5 (worked examples): each holds exactly. A command that changes nothing leaves
  `can_undo` unchanged. Oracle: exact value.

## Scorecard items and intended test names
| Item | Tests |
|---|---|
| edit.newline | `kiln_cli::acceptance_t025::newline_inserts_document_line_ending` |
| edit.indent | `kiln_cli::acceptance_t025::newline_keeps_indentation`, `kiln_cli::acceptance_t025::indent_outdent_match_naive_model`, `kiln_cli::acceptance_t025::indent_outdent_round_trip` |
| edit.comment-toggle | `kiln_cli::acceptance_t025::toggle_comment_matches_naive_model_and_round_trips` |

## Out of scope and implementer notes
- Smart indent after `{`, auto-closing brackets, configurable indent unit or tabs, block
  comments, Enter in the scratch buffer. Changing `doc.insert` or saving (bytes-as-read stays).
- New item (director, R-060 answer 6): the test author creates `scorecard/items/edit.newline.toml`
  on `task/T-025-tests` (an allowed path for this task's own item) with `id = "edit.newline"`,
  `title = "Enter inserts the document's line ending, so a newline typed into a CRLF file keeps it CRLF"`,
  `crate = "kiln-commands"`, `tier = "medium"` and the tests above, and regenerates
  `docs/scorecard.md` (a generated path) on that branch. It cannot be added on `main` directly
  (it changes the generated scorecard, D-034). `edit.indent` keeps the indentation part:
  `newline_keeps_indentation` checks the truncated leading whitespace on LF texts.
- Your own tests: `crates/kiln-commands/tests/edit.rs`, unit tests. Acceptance tests (odd arm)
  go first.

## Questions and rulings

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13 · R-056 · (new) · did: specified the editing commands, with the Enter key as its
  own command inserting the document's dominant line ending. That answers the evaluator's CRLF
  finding without touching file I/O. The newline rule sits under `edit.indent`; no new item
  was invented (Q-7). Tier medium (no ADR-0002 high criterion). Arm odd, paired against the
  even T-026 (fmt), both small-to-medium editing tasks. Every worked example was executed
  against a naive Python model. The round trip holds for comment-first toggles only, stated
  in (g). Pinned (b), (e), (f) n/a. · gate attempts: 0 · missing context: none · next: director.
- 2026-11-13 · R-060 · draft · did: added the item `edit.newline` (answer 6). The test author creates it on the tests branch; AC-1 now claims it, and `edit.indent` keeps the indentation part through a separate test. · gate attempts: 0 · missing context: none · next: director.
- 2026-11-13 · R-066 · draft · did: raises only `no_such_document` through T-015's existing helper; it needs T-032 only transitively, through T-018. · gate attempts: 0 · missing context: none · next: director.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
