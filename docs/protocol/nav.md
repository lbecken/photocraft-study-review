# `nav.*`: outline, go to a line, go to a symbol, find symbols in the project

The `nav` group lists the items of a document (its outline), moves a document's selection to a
line and column or to an item's name, and finds items by name across the project's Rust files.
The machine-readable schemas and examples are in `docs/commands.json`
(`kiln-cli commands --schema`).

All offsets are char offsets (Unicode scalar values) into the current text (the BOM is not part
of the text); CRLF counts as two chars. Lines and columns are 0-based; lines split on LF, CRLF
and a lone CR, as in `doc-positions.md`. `doc` is an open document's id, as in the `doc.*`
commands. Optional params, when present, must not be `null` (`invalid_params`); unknown params
are `invalid_params`.

| Command | Mutating | Params | Result |
|---|---|---|---|
| `nav.outline` | no | `{doc}` | `{items: [Item]}` |
| `nav.goto-line` | yes | `{doc, line, col?}` (integers ≥ 0; `col` default 0) | the selection, as `cursor.*` report it |
| `nav.goto-symbol` | yes | `{doc, index}` (integer ≥ 0) | `{item: Item, cursor: <the selection>}` |
| `nav.workspace-symbols` | no | `{query, limit?}` (`limit` 1 to 10,000, default 200) | `{items: [Symbol], truncated, skipped: [{path, reason}], walk_truncated, too_deep}` |

`Item` is `{kind, name, start, end, name_start, name_end, depth}`; `Symbol` is `{path, kind,
name, start, end, name_start, name_end, line, col}`. `kind` is one of `fn`, `struct`, `enum`,
`trait`, `impl`, `mod`. The selection is `{doc, anchor, head, start, end, line, col}`
(`docs/protocol/cursor.md`).

## The outline

`nav.outline` lists the `fn`, `struct`, `enum`, `trait`, `impl` and `mod` items of the document
that are not inside a function body, a block, an `extern` block, a `const` or `static` value or
a macro invocation, in the order of their start.

- `fn` is a function with or without a body (also one declared in a trait); `mod` is an inline
  module or `mod x;`.
- An item is listed when every node strictly between it and the root of the syntax tree is a
  `source_file`, a `declaration_list` (the body of a `mod`, `impl` or `trait`), a `mod_item`,
  an `impl_item`, a `trait_item` or an `ERROR` node (text the parser could not fit into the
  grammar). `depth` is the number of listed items that enclose it.
- `name` is the item's name. For an `impl` it is the text of its type, or `<trait> for <type>`
  for a trait impl, with every run of whitespace collapsed to one space; the `!` of a negative
  impl is not part of it (`impl !Send for X` has the name `Send for X`).
- `name_start..name_end` is the name (for an `impl`: its type); `start..end` is the whole
  item. Attributes and doc comments before an item are not part of it; a visibility such as
  `pub` is.

On the files of set `valid` of the pinned corpus (`crates/kiln-syntax/tests/corpus/`), the
`fn`, `struct`, `enum`, `trait` and `mod` items, with their names, name offsets and depths, and
the `impl` items, by the offset of their type and their depth, are exactly those the `syn`
parser finds.

In a document with syntax errors the outline lists the items the parser's error recovery leaves
in place, by the same rules; which items those are is not specified.

Examples (kind, name, `start..end`, `name_start..name_end`, depth):

1. `impl<T> Display for Foo<T> { fn fmt(&self) {} }\nmod m;\ntrait T { fn x(&self); }` →
   `impl` `Display for Foo<T>` `0..47` `20..26` 0; `fn` `fmt` `29..45` `32..35` 1; `mod` `m`
   `48..54` `52..53` 0; `trait` `T` `55..79` `61..62` 0; `fn` `x` `65..77` `68..69` 1.
2. `fn a() {}\nfn b( {\nfn c() {}\nstruct D;\n` (a syntax error) → `fn` `a` `0..9` `3..4` 0;
   `fn` `c` `18..27` `21..22` 0; `struct` `D` `28..37` `35..36` 0.
3. `/// doc\n#[derive(Debug)]\npub struct S;\n#[cfg(test)]\nmod tests { fn t() {} }` →
   `struct` `S` `25..38` `36..37` 0; `mod` `tests` `52..75` `56..61` 0; `fn` `t` `64..73`
   `67..68` 1.
4. `extern "C" { fn c(); }\nm! { fn inner() {} }\nfn outer() { fn nested() {} struct N; }\nconst _: () = { struct Q; };`
   → only `fn` `outer` `44..83` `47..52` 0.
5. `impl  <T>  Foo < T ,\n U > {}\nimpl !Send for X {}` → `impl` `Foo < T , U >` `0..28`
   `11..25` 0; `impl` `Send for X` `29..48` `44..45` 0.

The reply lists every item; there is no `limit`.

## Going to a line

`nav.goto-line {doc, line, col?}` puts a caret (anchor = head) at the line `min(line, L − 1)`,
where L is `len_lines` (`doc.info`), and the column `min(col, length in chars of that line's
content)`. `nav.goto-line` never refuses a line or column beyond the document: the line is
clamped to the last line and the column to the length of that line without its line break, so
the caret never lands inside a CRLF.

Example, text `ab\r\ncdef\n` (3 lines): `{line: 1, col: 9}` → head 8, line 1, col 4;
`{line: 8}` → head 9, line 2, col 0; `{line: 0, col: 1}` → head 1, line 0, col 1.

## Going to a symbol

`nav.goto-symbol {doc, index}` takes item `index` of the items `nav.outline` gives for the
current text and selects its name: anchor = `name_start`, head = `name_end`. It returns the item
and the selection afterwards. An `index` at or beyond the number of items is refused with
`rejected` / `out_of_range`. On example 1 above, `{index: 1}` → anchor 32, head 35; `{index: 5}`
→ `out_of_range`.

## Selection and typing group

`nav.goto-line` and `nav.goto-symbol` end the document's typing group when they succeed, as
`cursor.set` does; they do not change the text, the undo history or the dirty flag. A refused
request changes nothing.

## Size limit and the tree cache

`nav.outline` and `nav.goto-symbol` refuse a document longer than 1,048,576 chars with
`rejected` / `too_large`, as the `syntax.*` commands do; a document of exactly 1,048,576 chars
is accepted. `nav.goto-line` parses nothing and has no such limit.

The `nav.*` commands parse the current text afresh on every call and do not use or change the
tree that the `syntax.*` commands keep, so they never change the `mode` that `syntax.tree`
reports.

## Finding symbols in the project

`nav.workspace-symbols {query, limit?}` searches the outlines of the project's Rust files.

`nav.workspace-symbols` searches the entries of kind `file` whose name ends in `.rs` that
`project.tree {}` would list; it does not follow symbolic links, and files that `project.tree`
ignores or does not reach are not searched. For a file that is open as a document, the
document's current text is searched, saved or not, and the file is not read. Any other file is
read and decoded as `doc.open` decodes it (UTF-8, UTF-8 with BOM, UTF-16 LE/BE by BOM). A file
with syntax errors is searched; its outline is what the rules above give.

The search covers at most the first 1,000,000 entries of the walk, the largest `limit` of
`project.tree`; `walk_truncated` is true when the walk stopped there, and `too_deep` counts the
directories not descended because of the depth bound.

An item matches when its name, lowercased (Rust's `str::to_lowercase`), contains the query,
lowercased. Matches are ranked: exact matches of the name (ignoring case) first, then names
that begin with the query, then names that contain it; within a rank by the file's place in the
walk and then by the item's start. `truncated` is true when more than `limit` items matched.
`start`, `end`, `name_start` and `name_end` are chars into the file's text; `line` and `col` are
those of `name_start`.

A file that cannot be read (`not_found`, `not_a_file`, `outside_root`, `io_error`, for example
because it was removed after it was listed), that is not valid text (`decode_error`), or whose
text is longer than 1,048,576 chars (`too_large`) is listed in `skipped` with that reason, and
the search goes on. An error while listing a directory or reading a `.gitignore` fails the
whole command, as for `project.tree`. `skipped` is in walk order.

Example: a project of `a.rs` = `fn foo() {}\nstruct FooBar;\n`, `b/c.rs` =
`mod foo { fn barfoo() {} }\n` and `z.txt` = `fn foo() {}` (not `.rs`).
`{query: "FOO"}` → items (path, kind, name, `name_start`, line, col): `a.rs` `fn` `foo` 3 0 3;
`b/c.rs` `mod` `foo` 4 0 4; `a.rs` `struct` `FooBar` 19 1 7; `b/c.rs` `fn` `barfoo` 13 0 13;
`truncated: false`, `skipped: []`, `walk_truncated: false`, `too_deep: 0`. With `limit: 2` →
the first two, `truncated: true`. With `query: ""` → `invalid_pattern`.

The search is not a snapshot: if another process changes the project while the command runs,
the result can mix the states before and after.

## Errors

Checked in this order; the first failure is reported, nothing changes and nothing is logged:

- `nav.outline`: the params schema (`invalid_params`), then `rejected` / `no_such_document`,
  then `rejected` / `too_large`.
- `nav.goto-line`: the params schema, then `rejected` / `no_such_document`; nothing else.
- `nav.goto-symbol`: the params schema, then `rejected` / `no_such_document`, then
  `rejected` / `too_large`, then `rejected` / `out_of_range`.
- `nav.workspace-symbols`: the params schema (also a `limit` of 0 or above 10,000), then
  `rejected` / `invalid_pattern` (an empty `query`), then an error of the walk with
  `project.tree`'s kinds (`not_found`, `not_a_file`, `outside_root`, `io_error`).

If tree-sitter returns no tree (not possible with the pinned versions), the command fails with
`internal` / `other`. Messages are not stable (`docs/control-protocol.md`).

## Memory and time

A file is read and decoded whole before its length is checked, so a large file costs memory
even when it is skipped as `too_large`; files are searched one at a time. Measured with a
release build, a project holding one file of 1,048,576 chars (`a;` repeated) took a peak of
about 340 MiB. (The maximum resident set size of the whole `kiln-cli run` process running
`nav.workspace-symbols {query: "x"}`, two runs, on a 4-core machine that was running other
builds at the same time; a measured figure, not a bound.)

No time bound exists for any `nav.*` command that parses: parsing time depends on the text, not
only on its length, and `nav.workspace-symbols` parses every file it searches. Measured with a
release build, `nav.workspace-symbols` over a project holding one file of `#!` repeated (65,536
chars) took 23 s. (Elapsed time of `kiln-cli run` with `nav.workspace-symbols {query: "x"}`
minus that of the same over an empty project, two runs, on a 4-core machine that was running
other builds at the same time; a measured figure, not a bound.)
