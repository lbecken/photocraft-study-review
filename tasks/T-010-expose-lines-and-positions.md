---
id: T-010
title: "Expose line access and position conversion as registry commands"
tier: medium
state: draft
owner_role: planner
scorecard_items: [text.lines, text.positions]
depends_on: []
touches:
  - crates/kiln-commands/src/doc.rs
  - crates/kiln-commands/src/lib.rs
  - docs/commands.json
  - docs/control-protocol.md
budget_tokens: 400000
iteration: 0
found_by: evaluator
origin_task: T-001
---

# T-010 Expose line access and position conversion as registry commands

## Goal
A script driving `kiln-cli run` / `kiln-cli control` can read the text of line N, the char
offset where line N starts, and convert a position between byte offset, char offset, UTF-16
code unit offset and line/column for an open document. Today none of this is reachable from
outside, so the scorecard items `text.lines` and `text.positions` (status `implemented`,
built by T-001 according to their test names `kiln_text::acceptance_t001::*`) cannot be
confirmed by an outside evaluator, and every script has to fetch the whole document with
`doc.text` and compute offsets itself.

## Observed behaviour (evaluation R-050, main d3978dbf)
- `kiln-cli commands` lists 13 commands: `buffer.insert`, `buffer.text`, `doc.close`,
  `doc.delete`, `doc.info`, `doc.insert`, `doc.list`, `doc.open`, `doc.redo`, `doc.replace`,
  `doc.save`, `doc.text`, `doc.undo`. None takes or returns a line number, a line text, a line
  start offset, a byte offset, a UTF-16 offset or a column.
- The only line-related output is `doc.info.len_lines`. I checked it against a naive count
  (`len(re.findall(r"\r\n|\r|\n", s)) + 1`) on 31 random texts mixing LF, CRLF and CR: all
  agree. Line text and line start offsets (the rest of `text.lines`) are not observable.
- `text.positions` (byte/char/UTF-16/line-col conversions, exact inverses) has no public
  surface at all.

## Steps to reproduce
1. `kiln-cli commands --schema` and search for `line`, `column`, `utf16`, `byte`: only
   `len_lines` in the `doc.info` result.
2. Try to ask for line 3 of a CRLF file: no command exists.

## Interface
For the planner. Suggested shape (not a ruling): `doc.line {doc, line}` →
`{text, start, end, ending}`; `doc.position {doc, from: {char|byte|utf16|line_col}}` →
all four forms; out-of-range and mid-character inputs rejected with `out_of_range` /
a new documented kind.

## Acceptance criteria
- AC-1: For random texts with LF, CRLF, CR, multi-byte and astral chars, `doc.line` for
  every line agrees with a naive split. Oracle: naive model in the test.
- AC-2: For every valid position, converting char → byte/UTF-16/line-col → char returns the
  start position. Oracle: round trip plus naive model.
- AC-3: An offset inside a UTF-8 sequence or a UTF-16 surrogate pair is rejected with a
  structured error. Oracle: exact error code/kind.
- AC-4: The commands appear in `docs/commands.json` with examples that `--audit` accepts.

## Out of scope
Cursor/selection (`edit.selection`), goto-line (`nav.goto-line`).

## Implementer notes
Read-only commands; must be recorded in the log like other read-only commands.

## Questions

## Log
### director
<!-- end director log -->
### planner
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
