# `eol.*`: line endings of a document

Kiln keeps a file's bytes as read: opening and saving never change its line endings, so a file
with mixed line endings stays mixed until someone asks otherwise. The `eol` group counts a
document's line breaks and rewrites them **on explicit request**. The machine-readable schemas
and examples are in `docs/commands.json` (`kiln-cli commands --schema`).

A line break is `\r\n` (CRLF), a lone `\n` (LF) or a lone `\r` (CR), matched CRLF first: in
`a\r\r\n` there is one CR and one CRLF. No other char breaks a line.

## `eol.counts` (read-only)

Params: `{"doc": <id>}`. Result: the number of breaks of each kind, and `line_ending` and
`mixed_line_endings` as `doc.info` reports them (the dominant kind, ties going to LF, then
CRLF, then CR, or `"none"` without breaks; and whether more than one kind occurs).

```json
{"command": "eol.counts", "params": {"doc": 1}}        // text "a\r\nb\nc\rd\r\n"
→ {"lf": 1, "crlf": 2, "cr": 1, "line_ending": "crlf", "mixed_line_endings": true}
```

## `eol.normalize` (mutating)

Params: `{"doc": <id>, "to"?: "lf" | "crlf" | "cr"}`. Every line break that differs from `to`
is rewritten as `to`; breaks that already are `to` are left alone. Without `to`, the target
is the document's dominant line ending (`doc.info.line_ending`). The document is changed in
memory only; `doc.save` writes it.

Result: `changed` (the number of breaks rewritten), and `len_chars`, `line_ending` and
`mixed_line_endings` of the text afterwards.

```json
{"command": "eol.normalize", "params": {"doc": 1, "to": "lf"}}   // text "a\r\nb\nc\rd"
→ {"changed": 2, "len_chars": 7, "line_ending": "lf", "mixed_line_endings": false}
```

The text is now `a\nb\nc\nd`. One `doc.undo` restores `a\r\nb\nc\rd` exactly, and `doc.redo`
applies the normalisation again.

- **One undo step.** A normalisation that rewrites anything is one undo step. It ends the
  document's typing group and is never merged with typing.
- **Nothing to do changes nothing.** If no break differs from the target, or the text has no
  line break, the result has `changed: 0` and nothing changes: not the text, not the
  selection, not the undo/redo history, and not an open typing group (typing after it still
  joins the typing before it).
- **The selection follows.** Each rewritten break is an edit of its own for the selection
  (see the mapping rule in `cursor.md`), applied from the last break to the first. A caret
  just after a CRLF that becomes an LF stays just after the new LF; a caret just before it
  stays before it. The text as a whole is not replaced, so a selection elsewhere in the text
  keeps its place relative to the surrounding text.

### Refusals

`doc` must be an open document's id, else `rejected` / `no_such_document` (also for `doc: 0`).
A `to` other than the three names fails the params schema (`invalid_params`).
