---
id: T-020
title: "Report a document's syntax tree with incremental reparse, highlight spans that partition the text, and bracket matching that ignores strings and comments"
tier: medium            # ADR-0002 default: in-memory behaviour, new command group; tree-sitter and the oracle dev-deps were added by T-016
state: draft
owner_role: planner
scorecard_items: [syntax.parse, syntax.incremental, syntax.highlight, syntax.brackets]
depends_on: [T-015, T-016]
touches:
  - crates/kiln-syntax/src/highlight.rs       # slot from T-016
  - crates/kiln-syntax/src/brackets.rs        # slot from T-016
  - crates/kiln-syntax/tests/highlight.rs     # implementer's own integration tests
  - crates/kiln-commands/src/syntax.rs        # syntax.* + SyntaxState (slot from T-015)
  - docs/protocol/syntax.md                   # new
  - docs/commands.json                        # generated
budget_tokens: 500000   # medium (v1.1, provisional)
iteration: 1
found_by: planner
origin_task: null
---

# T-020 Syntax tree, highlight and brackets for an open document

## Goal
After this task a script can ask, for any open document, for its syntax tree (error spans, the
pinned S-expression of T-016, and whether it was reparsed incrementally). It can also ask for
highlight spans that cover the text exactly once, in seven kinds, and for the bracket matching
the one at an offset, with brackets inside strings, chars and comments ignored. Trees are kept
per document between commands and updated incrementally after edits. The result is always the
tree a fresh parse would give. All four commands are read-only.

No gate path. Arm: **even** (independent test author).

Reading list: `tasks/T-016-*.md` (SyntaxTree, sexp form, corpus layout);
`crates/kiln-testkit/corpus/MANIFEST.toml`; `tasks/T-015-*.md` (`SyntaxState`, `Parts`);
`tasks/T-007-*.md` (b) read-only property.

## Interface

### Commands (`doc` must be open, else `rejected`/`no_such_document`)
| Command | Params | Result |
|---|---|---|
| `syntax.tree` | `{doc}` | `{has_error, errors:[{kind:"error"\|"missing", start, end}], sexp, node_count, mode:"full"\|"incremental"\|"unchanged"}` |
| `syntax.highlight` | `{doc, start?, end?}` | `{spans:[{start, end, kind}]}` |
| `syntax.match-bracket` | `{doc, offset}` | `{bracket: {open, close} \| null}` |

All are `mutating: false`. They keep a cache in `SyntaxState` (per document id: the last text
and tree). The cache is not session state (director, R-060). The read-only property concerns observable
state: document text, dirty flag, undo history, ids, the command log and selections. A derived
cache that no command result exposes is not state, and T-007's check compares observable state
only. A command's result never depends on the cache except for `mode`, which says how the tree
was obtained: `full` if no tree was cached for the document, `unchanged` if the text equals the
cached text, `incremental`/`full` per `SyntaxTree::update` otherwise.

### `syntax.highlight` spans (exact)
Spans are sorted, contiguous and non-overlapping. They cover `[start, end)` exactly (default
`0..len_chars`); at the range ends they are clipped. Adjacent spans of the same kind are
merged. Each char gets one kind, the first rule that applies:
1. `comment`: inside a `line_comment` or `block_comment` node (doc comments included).
2. `string`: inside a `string_literal`, `raw_string_literal` or `char_literal` node, quotes
   and prefixes included.
3. `number`: inside an `integer_literal` or `float_literal`.
4. `keyword`: an anonymous leaf whose kind is one of `as async await break const continue crate
   dyn else enum extern false fn for gen if impl in let loop match mod move mut pub ref return
   self Self static struct super trait true type unsafe use where while yield abstract become
   box do final macro override priv typeof unsized virtual try union default safe raw auto
   macro_rules!`, or a named leaf of kind `self`, `crate`, `super` or `mutable_specifier`.
5. `type`: a `type_identifier` or `primitive_type` leaf.
6. `function`: the `name` identifier of a `function_item` or `function_signature_item`. Also
   the callee of a `call_expression`: its `function` field if it is an `identifier`; the `name`
   field of a `scoped_identifier`; the `field` field of a `field_expression`. A
   `generic_function`'s `function` field is resolved the same way.
7. `plain`: everything else.

### `syntax.match-bracket`
If the char at `offset` is one of `( ) [ ] { }`, and it is an anonymous leaf of the tree (so
not inside a string, char or comment), and the parent node has the matching counterpart leaf,
then `bracket` is `{open, close}`: the offsets of the opening and closing char. Otherwise
`bracket` is `null`; for example an unmatched bracket while typing, any other char, or offset
`len_chars`. `<`/`>` are not brackets. Errors: `offset > len_chars` → `out_of_range`.

No new error kind. `docs/protocol/syntax.md` documents the commands, the span rules and the
bracket rule.

## Pinned
- (a) Errors: schema → `no_such_document` → `out_of_range` (offset, `start`, `end`) →
  `inverted_range` (`start > end`).
- (b) Files: none. Corpus files are read by tests only (by path, `crates/kiln-testkit/corpus/`).
- (c) Order: n/a beyond (a).
- (d) Offsets are chars; the tree is of the document's current text (BOM excluded).
- (e) Threat model: n/a (no boundary). Large inputs: a 10 MB document must answer
  `syntax.tree` (its `sexp` is large; that is accepted).
- (f) Snapshots: none.
- (g) Worked examples (leaves checked with tree-sitter 0.27.2 / rust 0.24.2 in a spike; offsets
  computed by script):
  - `fn f() { let s: u8 = 1; } // c` (30 chars) → spans `0..2 keyword`, `2..3 plain`,
    `3..4 function`, `4..9 plain`, `9..12 keyword`, `12..16 plain`, `16..18 type`,
    `18..21 plain`, `21..22 number`, `22..26 plain`, `26..30 comment`.
  - On the same text `syntax.match-bracket {offset:4}` → `{open:4, close:5}`; `{offset:7}` →
    `{open:7, close:24}`; `{offset:0}` → `null`.
  - `fn a() { (` → `syntax.tree` has `has_error: true`; `match-bracket {offset:9}` → `null`.
  - `fn f() {}` → `sexp` = T-016's example; first call `mode: "full"`, second `unchanged`;
    after `doc.insert {offset:8, text:" "}`, `mode: "incremental"`.
- (h) Evaluator calls: `kiln-cli run` with `doc.open` of a corpus file then `syntax.tree`
  (`syntax.parse`); edits followed by `syntax.tree`, compared with `syntax.tree` of a fresh
  session on the edited text (`syntax.incremental`); `syntax.highlight` (`syntax.highlight`);
  `syntax.match-bracket` (`syntax.brackets`).

## Acceptance criteria
All through `kiln-cli run`/`control` or `kiln_control::Host` on files from the pinned corpus,
copied to a temp root.
- AC-1 (`syntax.parse`): every `valid` corpus file gives `has_error: false` and `errors: []`.
  Every `invalid` file gives `has_error: true` and ≥ 1 error inside the text. Oracle: rustc (the
  corpus is rustc-checked by T-016's test).
- AC-2 (`syntax.incremental`): for every `valid` file, ≥ 100 fixed-seed random
  `doc.insert`/`doc.delete`/`doc.replace`/`doc.undo` per file, each followed by `syntax.tree`.
  Its `has_error`, `errors`, `sexp` and `node_count` equal `syntax.tree` in a fresh session
  holding the same text (`doc.text`). `mode` is `incremental` whenever `has_error` is false, and
  never `full` after the first call when `has_error` is false. Oracle: from-scratch parse.
- AC-3 (`syntax.highlight`, partition): for every corpus file and ≥ 50 random sub-ranges, the
  spans are sorted, contiguous, non-overlapping, merged and cover the range exactly, and every
  kind is one of the seven. Oracle: constructed property.
- AC-4 (`syntax.highlight`, differential with proc-macro2): on every `valid` file, tokenize with
  `proc_macro2` (span locations, `byte_range`). Consider only `Literal` tokens whose source
  slice equals the token's text, which excludes desugared doc comments. Those slices equal the
  union of `string` and `number` spans: string, byte string, char and C string literals are
  `string`, numeric ones `number`. Every `keyword` span's text is in the pinned list. Every
  `comment` span starts with `//` or `/*`. Oracle: an independent tokenizer.
- AC-5 (`syntax.highlight`, functions and types): on every `valid` file, each fn definition
  name found by `syn` (free fns, impl and trait methods, outside fn bodies) is a `function`
  span at the ident's offset. The worked example holds exactly. Oracle: syn + exact value.
- AC-6 (`syntax.brackets`, differential): on every `valid` file, take each proc-macro2 `Group`
  delimited by parentheses, brackets or braces whose source chars at the open and close span
  are that delimiter (doc-comment groups excluded). `match-bracket` at the open offset and at
  the close offset both return that pair. At every other offset whose char is a bracket,
  `bracket` is `null`, because those brackets are in strings, chars or comments. Oracle: an
  independent tokenizer.
- AC-7 (`syntax.brackets`, editing): the worked examples hold. `offset` = `len_chars` → `null`;
  `len_chars + 1` → `out_of_range`. Oracle: exact value.

## Scorecard items and intended test names
| Item | Tests |
|---|---|
| syntax.parse | `kiln_cli::acceptance_t020::corpus_parses_through_protocol` |
| syntax.incremental | `kiln_cli::acceptance_t020::incremental_equals_fresh_session` |
| syntax.highlight | `kiln_cli::acceptance_t020::highlight_partitions_text`, `kiln_cli::acceptance_t020::highlight_agrees_with_tokenizer`, `kiln_cli::acceptance_t020::highlight_marks_fn_names` |
| syntax.brackets | `kiln_cli::acceptance_t020::brackets_agree_with_token_groups`, `kiln_cli::acceptance_t020::bracket_worked_examples` |

## Out of scope and implementer notes
- Outline, goto-symbol (T-023); semantic highlighting; themes or colours (iteration 2); range
  queries over the tree.
- No-oracle note: the `type` kind has only hand-written expectations (worked example and
  fixtures); no independent tokenizer separates type names from other identifiers.
- Drop cache entries of closed documents on the next `syntax.*` call; ids are never reused.
- Your tests: `crates/kiln-syntax/tests/highlight.rs`, unit tests in touched files.

## Questions and rulings

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13 · R-056 · (new) · did: specified four syntax items as one in-memory command
  group. The oracles are rustc (corpus), a fresh parse (incremental), proc-macro2 tokens
  (literals, brackets) and syn (fn names). `type` has no oracle beyond hand-written
  expectations, as stated in Out of scope. Tier medium: no ADR-0002 high criterion (the
  dependencies were added in T-016). Arm even, paired against the odd T-023 (outline/nav, also
  syntax-based). The worked examples' leaves were checked in a tree-sitter spike. Pinned (b),
  (e), (f) n/a. Unsure: the cache in a read-only command versus T-007's audit, recorded as a
  stop condition in Interface. · gate attempts: 0 · missing context: T-007's final state
  comparison · next: director.
- 2026-11-13 · R-060 · draft · did: recorded the director's ruling on the cache (R-060 answer 2); the stop condition is removed. The same sentence is in T-007 (b). · gate attempts: 0 · missing context: none · next: director.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
