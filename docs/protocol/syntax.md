# `syntax.*`: syntax tree, highlight spans and brackets of a document

The `syntax` group reports what the Rust parser (tree-sitter with the pinned Rust grammar,
`kiln-syntax`) sees in an open document: its syntax tree, highlight spans and bracket
matching. The machine-readable schemas and examples are in `docs/commands.json`
(`kiln-cli commands --schema`).

All offsets are char offsets into the document's current text (the BOM is not part of the
text), as in the `doc.*` commands; CRLF counts as two chars. `doc` is a document id as in
the `doc.*` commands.

## Read-only

`syntax.tree`, `syntax.highlight` and `syntax.match-bracket` are read-only (`mutating:
false`): they change no document's text, dirty flag, undo history or selection, and they do
not end a typing group.

## The tree cache and `mode`

Every result is the one a parse of the current text from scratch gives; only `mode` depends
on earlier calls. `mode` is `full` on the first successful `syntax.*` call for a document,
`unchanged` when the text equals the text of the document's last successful `syntax.*` call,
and otherwise `incremental` or `full`, where `incremental` only occurs for a tree without
errors. A refused call does not change what the next call reports.

## `syntax.tree`

Params: `{"doc": <id>}`. Result:

- `has_error`: whether the tree holds an `ERROR` or `MISSING` node;
- `errors`: every such node in preorder, as `{"kind": "error" | "missing", "start", "end"}`
  (a `missing` node is a token the parser inserted; its `start` equals its `end`);
- `sexp`: the tree as an S-expression: named nodes and `MISSING` nodes in preorder, each as
  `(KIND@START..END` followed by its children and `)`, `FIELD:` before a child reached
  through a field, `ERROR` for an error node, and `(MISSING KIND@S..S)` with an anonymous
  KIND in double quotes;
- `node_count`: the number of nodes, named and anonymous, the root included;
- `mode`: `full`, `incremental` or `unchanged` (above).

```json
{"command": "syntax.tree", "params": {"doc": 1}}        // text "fn f() {}", first call
→ {"has_error": false, "errors": [], "node_count": 10, "mode": "full",
   "sexp": "(source_file@0..9 (function_item@0..9 name:(identifier@3..4) parameters:(parameters@4..6) body:(block@7..9)))"}
```

A second call gives the same with `mode: "unchanged"`; after `doc.insert {"doc": 1,
"offset": 8, "text": " "}` it gives `mode: "incremental"`.

## `syntax.highlight`

Params: `{"doc": <id>, "start"?: <offset>, "end"?: <offset>}`; `start` defaults to 0 and
`end` to the text's length in chars. Omit a parameter for its default; `null` is refused
(`invalid_params`). `start` and `end` are plain char offsets: a value between the `\r` and
`\n` of a CRLF is accepted.

Result: `{"spans": [{"start", "end", "kind"}, …]}`. Highlight spans are sorted, contiguous and
non-overlapping, cover the requested range exactly, and adjacent spans of the same kind are
merged. At the range ends the spans are clipped. An empty range (`start == end`, or an empty
document) gives `spans: []`.

Each char gets one kind, the first rule that applies:

1. `comment`: inside a `line_comment` or `block_comment` node (doc comments included).
2. `string`: inside a `string_literal`, `raw_string_literal` or `char_literal` node, quotes and
   prefixes included (`b'x'`, `c"…"`, `br#"…"#` are single literals).
3. `number`: inside an `integer_literal` or `float_literal`.
4. `keyword`: an anonymous leaf whose kind is one of `as async await break const continue
   crate dyn else enum extern false fn for gen if impl in let loop match mod move mut pub ref
   return self Self static struct super trait true type unsafe use where while yield abstract
   become box do final macro override priv typeof unsized virtual try union default safe raw
   auto macro_rules!`, or a named leaf of kind `self`, `crate`, `super` or
   `mutable_specifier`.
5. `type`: a `type_identifier` or `primitive_type` leaf.
6. `function`: the `name` identifier of a `function_item` or `function_signature_item`. Also
   the callee of a `call_expression`: its `function` field if it is an `identifier`; the
   `name` field of a `scoped_identifier`; the `field` field of a `field_expression`. A
   `generic_function`'s `function` field is resolved the same way.
7. `plain`: everything else.

With the pinned grammar, `Self` in `Self::new()` is an `identifier` in a path, so it is
`plain` there, not `keyword`.

```json
{"command": "syntax.highlight", "params": {"doc": 1}}   // text "fn f() { let s: u8 = 1; } // c"
→ {"spans": [{"start": 0, "end": 2, "kind": "keyword"}, {"start": 2, "end": 3, "kind": "plain"},
   {"start": 3, "end": 4, "kind": "function"}, {"start": 4, "end": 9, "kind": "plain"},
   {"start": 9, "end": 12, "kind": "keyword"}, {"start": 12, "end": 16, "kind": "plain"},
   {"start": 16, "end": 18, "kind": "type"}, {"start": 18, "end": 21, "kind": "plain"},
   {"start": 21, "end": 22, "kind": "number"}, {"start": 22, "end": 26, "kind": "plain"},
   {"start": 26, "end": 30, "kind": "comment"}]}
```

## `syntax.match-bracket`

Params: `{"doc": <id>, "offset": <offset>}`. Result: `{"bracket": {"open", "close"}}` with
the offsets of the opening and closing char, or `{"bracket": null}`.

The char at `offset` is matched if it is one of `( ) [ ] { }`, it is an anonymous leaf of the
tree (so not inside a string, char literal or comment), and it has a partner among the
anonymous bracket leaves of the same kind that are direct children of the same parent node.
Partners: those leaves are taken left to right, and each closing leaf pairs with the nearest
unpaired opening leaf before it; a leaf left unpaired has no partner. A bracket the parser
inserted to recover (a `MISSING` node, which holds no char) is never a partner: in
`fn f() { g(1; }` the `(` at 10 gives `null`. `<` and `>` are not brackets. Brackets inside
strings, character literals and comments are never matched; a bracket without a partner gives
`null`. Any other char, and `offset` equal to the text's length, also give `null`.

```json
{"command": "syntax.match-bracket", "params": {"doc": 1, "offset": 7}}   // text "fn f() { let s: u8 = 1; } // c"
→ {"bracket": {"open": 7, "close": 24}}
```

In `fn a() { (` the `(` at 9 gives `null`. In `) ( ( ) ] [` only the pair at 4 and 6 is
matched; 0, 2, 8 and 10 give `null`.

## Size limit

A document longer than 1,048,576 characters (`len_chars` in `doc.info`) is refused by every
`syntax.*` command with `rejected` / `too_large` before anything is parsed. A document of
exactly 1,048,576 characters is accepted, and the check is made on every call, so a document
that shrinks back to the limit is accepted again.

## Errors

Checked in this order: params against the schema (`invalid_params`), then
`rejected`/`no_such_document`, then `rejected`/`too_large` (the whole document, above), then
`rejected`/`out_of_range` (`offset`, `start` or `end` beyond the text's length), then
`rejected`/`inverted_range` (`start` after `end`). If tree-sitter returns no tree (T-016:
not possible with the pinned versions), the command fails with `internal`/`other`. Messages
are not stable (`docs/control-protocol.md`).

## Time

No time bound exists for any `syntax.*` command: the size limit bounds the length of the text
a command accepts, not the time a parse takes or the memory it needs. Parsing time depends
on the text, not only on its length, and a call after an edit that leaves the tree with
errors parses the text twice.

Measured with a release build of `kiln-cli` (`syntax.tree` after `doc.open`): a text of `#!`
repeated took 1.10 s at 16,384 characters, 4.49 s at 32,768 and 17.00 s at 65,536. These are
elapsed times of one run each, on a 4-core machine that was running other builds at the
same time; they show the time growing about fourfold for twice the length, and they are not
a bound.

## Memory

Memory grows with the length and content of the text. Measured with a release build of
`kiln-cli` at the limit (1,048,576 characters; `doc.open`, `syntax.tree`, a `doc.replace` of
one char, `syntax.tree`, `syntax.highlight`): 689 MiB for `a;` repeated and 2,073 MiB for `1;`
repeated. These are the maximum resident set size of the whole `kiln-cli run` process in two
runs. They are not a bound: no bound on memory is promised, and other texts can need more.
Without the `syntax.highlight` call, the same script on `1;` repeated peaked at 566 MiB; with
it, the `syntax.highlight` reply held 1,048,575 spans.
