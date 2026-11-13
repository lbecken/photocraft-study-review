# `doc.line` and `doc.position`

Two read-only commands of the `doc` group (T-010) that expose `kiln-text`'s line access and
position conversions. Both are `mutating: false`: they change no document, never end a typing
group and never touch undo/redo history. A successful request is recorded in the session log
like any command; a refused one is not. The machine-readable schemas and examples are in
`docs/commands.json` (`kiln-cli commands --schema`).

All examples below use a document whose text is `"a\r\nbé😀\rc\n"` (9 chars, 13 UTF-8 bytes,
10 UTF-16 code units, 4 lines).

## Lines

A text is split into lines at `\r\n`, a lone `\r` and `\n`; a `\r\n` is one break. A text with
k breaks has k + 1 lines (`doc.info.len_lines`), and the last line may be empty.

## `doc.line`

Params: `{"doc": <id>, "line": <integer ≥ 0>}`, no other keys. `doc` is an open document's
id, typed as in every doc command (an unsigned integer).

Result:

| Field | Meaning |
|---|---|
| `line` | the requested line index |
| `text` | the line's content, without its line break |
| `start` | char offset of the line's first char |
| `end` | `start` + the number of chars in `text` |
| `ending` | the break after `text`: `"lf"`, `"crlf"`, `"cr"`, or `"none"` for the last line |

```json
{"command": "doc.line", "params": {"doc": 1, "line": 1}}
→ {"line": 1, "text": "bé😀", "start": 3, "end": 6, "ending": "cr"}
```

The four lines of the example text are `{"text":"a","start":0,"end":1,"ending":"crlf"}`,
`{"text":"bé😀","start":3,"end":6,"ending":"cr"}`, `{"text":"c","start":7,"end":8,"ending":"lf"}`
and `{"text":"","start":9,"end":9,"ending":"none"}`.

## `doc.position`

Params: `{"doc": <id>, "at": <position>}` where `<position>` is exactly one of

- `{"char": n}`: a char offset;
- `{"byte": n}`: a UTF-8 byte offset;
- `{"utf16": n}`: a UTF-16 code-unit offset;
- `{"line_col": {"line": l, "col": c}}`: a zero-based line and a column counted in chars.

All numbers are integers ≥ 0. Result: every form of the same position,
`{"char", "byte", "utf16", "line", "col"}`.

`byte` counts the UTF-8 bytes of the document's text as Kiln decoded it, not the bytes of the
file: a byte-order mark is not counted, and the text of a UTF-16 file is counted in UTF-8. For
example, a UTF-8 file with a BOM holding `héx` (file bytes `EF BB BF 68 C3 A9 78`) gives
`{"char": 2}` → `"byte": 3`, although the `x` is at offset 6 of the file; the same text in a
UTF-16 LE file (file bytes `FF FE 68 00 E9 00 78 00`) also gives `"byte": 3` (file offset 6).
A tool that reports offsets into the file must convert them.

```json
{"command": "doc.position", "params": {"doc": 1, "at": {"char": 5}}}
→ {"char": 5, "byte": 6, "utf16": 5, "line": 1, "col": 2}
```

### Validity

A position is **valid** iff it is a char offset `0..=len_chars` that is not between the `\r`
and `\n` of a CRLF, and it is given in a unit that lands exactly on it: a byte offset on a char
boundary, a UTF-16 offset not inside a surrogate pair, a column within the line's content
(`0..=` its length in chars). On valid positions the conversions are exact inverses: asking
`doc.position` with any of the returned forms returns the same five numbers.

On the example text, char offsets 0, 1, 3…9 are valid; 2 is between the `\r` and `\n` of the
first line break. Byte offsets 5 (inside `é`) and 7 (inside `😀`) and UTF-16 offset 6 (inside the
surrogate pair of `😀`) are not boundaries.

## Refusals

Order of checks: params schema (`invalid_params`), then the document id (`no_such_document`
for an unknown or closed id, including `0`), then the line or position. A refusal has code
`rejected` and changes nothing. Its message is `invalid line: <reason>` for `doc.line` and
`invalid position: <reason>` for `doc.position`, where `<reason>` is the `Display` text of
`kiln-text`'s `PositionError`:

| `PositionError` | kind | example message (example text) |
|---|---|---|
| `OutOfBounds` | `out_of_range` | `invalid position: index 14 is out of bounds (length 13)` (`{"byte":14}`) |
| `LineOutOfBounds` | `out_of_range` | `invalid line: line 4 is out of bounds (buffer has 4 lines)` |
| `ColumnOutOfBounds` | `out_of_range` | `invalid position: column 4 is out of bounds for line 1 (3 chars)` |
| `NotCharBoundary` | `not_a_boundary` | `invalid position: byte offset 5 is inside a multi-byte char` |
| `InsideSurrogatePair` | `not_a_boundary` | `invalid position: UTF-16 offset 6 is inside a surrogate pair` |
| `InsideLineBreak` | `not_a_boundary` | `invalid position: char index 2 is between the \r and \n of a CRLF` |

An offset, a line or a column beyond the end is refused with `out_of_range` (its published
description in `error_kinds` still reads "an offset beyond the end of the text"). A position
inside a character, a surrogate pair or a CRLF line break is refused with `not_a_boundary`,
whichever unit names it (`{"byte": 2}` on the example text is `InsideLineBreak`). The schema
accepts any `u64`; a huge value is refused with `out_of_range`, never a panic.

Params that do not match the schema, such as two keys in `at` (`{"char": 1, "byte": 1}`), an
empty `at`, a negative number or an unknown key, are `invalid_params`.
