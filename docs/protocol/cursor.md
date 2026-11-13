# `cursor.*`: the selection of a document

Every open document has exactly one **selection**: an `anchor` and a `head`, both char offsets.
The head is the caret; the anchor is the end that stays put while a selection is extended. A
newly opened document's selection is `anchor = head = 0`. The selection is not saved, is not
part of `doc.info`, and is dropped when the document is closed.

The selection follows every edit of the document by itself: `doc.insert`, `doc.delete`,
`doc.replace`, `doc.undo`, `doc.redo`, `eol.normalize` and `search.replace` all move it by the
mapping rule below, without being told about it. The machine-readable schemas and examples are in
`docs/commands.json` (`kiln-cli commands --schema`).

Offsets are chars (Unicode scalar values). A **valid position** is an offset `0..=len_chars`
that does not lie between the `\r` and `\n` of a CRLF (the same validity as `doc.position`,
see `doc-positions.md`). Lines split on `\r\n`, a lone `\r` and `\n`.

## Commands

| Command | Mutating | Params | Result |
|---|---|---|---|
| `cursor.get` | no | `{doc}` | the selection |
| `cursor.set` | yes | `{doc, anchor, head?}` | the selection afterwards |
| `cursor.move` | yes | `{doc, by, forward, extend?}` | the selection afterwards |

Every result has the form

```json
{"doc": 1, "anchor": 3, "head": 8, "start": 3, "end": 8, "line": 1, "col": 1}
```

where `start`/`end` are the smaller/larger of `anchor` and `head`, and `line`/`col` are the
head's zero-based line and column (column in chars), as `doc.position` reports them.

- `cursor.get` changes nothing (read-only).
- `cursor.set` sets the selection; an absent `head` defaults to `anchor` (a caret).
  `"head": null` is not a value: it fails the params schema (`invalid_params`).
- `cursor.move` moves the head one step: `by` is `"char"`, `"word"` or `"line"`, `forward` is
  `true` (towards the end; down for lines) or `false`. The step is computed from the head
  before the move. Without `extend` (or with `extend: false`) the selection collapses at the
  new head (`anchor = head`); with `extend: true` only the head moves.

`cursor.set` and `cursor.move` end the document's typing group when they succeed: after typing
`a`, moving the cursor and typing `b`, `doc.undo` removes only `b`. A refused request changes
nothing, the typing group included.

### Refusals

Checked in this order; the first failure is reported and nothing changes:

1. the params schema (`invalid_params`), which also refuses `null` for an optional
   parameter (`head`, `extend`): omit it instead;
2. the document: `doc` must be an open document's id, else `rejected` / `no_such_document`
   (also for `doc: 0`);
3. for `cursor.set`, `anchor` and then `head`: an offset beyond the text is `rejected` /
   `out_of_range`; an offset between the `\r` and `\n` of a CRLF is `rejected` /
   `not_a_boundary`. The message names the end at fault, e.g.
   `invalid anchor: char index 6 is between the \r and \n of a CRLF`.

`cursor.move` cannot produce an invalid position, so after its document check it always
succeeds.

## Mapping rule

An edit replaces the chars `[s, e)` with `n` chars. A selection end (or any anchor) at `p`
moves to:

- `p` if `p < s` (before the edit);
- `p + n - (e - s)` if `p > e` (after the edit: shifted by the change in length);
- `s` if `p == s` and the edit removed something (`e > s`);
- `s + n` otherwise: a position inside the removed range or at its end goes to the end of the
  new text, and so does a caret exactly at a pure insertion point (typing at the caret leaves
  the caret after the typed text).

Then, if the result lies between the `\r` and `\n` of a CRLF (for example because a `\r` was
inserted just before a `\n`), it moves to just before the `\r`. This snap is applied after
each primitive edit, to the text as it is after that edit; in a step of several edits that text
can hold a `\r\n` the final text does not (see the exception in `eol.md`).

`doc.undo` and `doc.redo` move the selection through the edits they apply, in the order they
apply them. A step made of several edits (`eol.normalize`,
`search.replace`) is applied from its last edit (the
highest position) to its first, and so is its undo. An undo therefore maps the selection like
any edit; it does not necessarily restore the selection of before the step. Rejected requests and no-op edits (inserting
`""`, deleting an empty range) move nothing.

### Worked example

Text `abc\r\ndef`:

```json
{"command": "cursor.set", "params": {"doc": 1, "anchor": 1, "head": 6}}
{"command": "doc.insert", "params": {"doc": 1, "offset": 0, "text": "XY"}}
{"command": "cursor.get", "params": {"doc": 1}}
→ {"doc": 1, "anchor": 3, "head": 8, "start": 3, "end": 8, "line": 1, "col": 1}
{"command": "doc.delete", "params": {"doc": 1, "start": 4, "end": 7}}
{"command": "cursor.get", "params": {"doc": 1}}
→ {"doc": 1, "anchor": 3, "head": 5, "start": 3, "end": 5, "line": 0, "col": 5}
```

The insert at 0 is before both ends (+2). The delete removes `c\r\n` (`[4, 7)` of
`XYabc\r\ndef`): the anchor 3 is before it, the head 8 after it (8 − 3 = 5). The text is now
`XYabdef`.

## Movement rule

A **word char** is `_` or an alphanumeric char (any script); everything else, line breaks
included, is a non-word char.

- `char`: one char forward or backward, but a CRLF counts as one step (two chars). It never
  goes past the start or the end of the text.
- `word` forward: skip non-word chars, then word chars. Backward: the mirror image (skip the
  non-word chars before the head, then the word chars before it).
- `line` forward (down) or backward (up): the same column, in chars, on the next or previous
  line, clamped to that line's length (without its break). There is no remembered goal
  column: the column always comes from the current head. Up from the first line goes to 0;
  down from the last line goes to the end of the text.

### Worked examples

- `fn foo_bar(x: u8)`, head 0, `cursor.move {by: "word", forward: true}` four times → heads
  2, 10, 12, 16; then four times backward → 14, 11, 3, 0.
- `abcdef\nxy\nlonger`, head 5, `by: "line"` forward → 9 (line 1, col 2: `xy` has only 2
  chars); forward again → 12 (line 2, col 2, not col 5: no goal column).
- `a\r\nb`, head 1, `by: "char"` forward → 3 (over the whole CRLF).
