---
id: T-020
title: "Report a document's syntax tree with incremental reparse, highlight spans that partition the text, and bracket matching that ignores strings and comments"
tier: medium            # ADR-0002 default: in-memory behaviour, new command group; tree-sitter and the oracle dev-deps were added by T-016
state: draft
owner_role: planner
scorecard_items: [syntax.parse, syntax.incremental, syntax.highlight, syntax.brackets]
depends_on: [T-015, T-016, T-032]
touches:
  - crates/kiln-syntax/src/highlight.rs       # slot from T-016
  - crates/kiln-syntax/src/brackets.rs        # slot from T-016
  - crates/kiln-syntax/src/tree.rs            # R-109: unit-test case only (T-016 review N2); no API change
  - crates/kiln-syntax/tests/highlight.rs     # implementer's own integration tests
  - crates/kiln-commands/src/syntax.rs        # syntax.* + SyntaxState (slot from T-015)
  - crates/kiln-commands/src/reason.rs        # R-109: one row, RejectReason::TooLarge (D-056 size limit)
  - apps/kiln-cli/tests/acceptance_t020.rs    # test author
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
tree a fresh parse would give. All three commands are read-only. A document longer than
1,048,576 chars is refused by every `syntax.*` command with the new kind `too_large`, so no
command parses an unbounded text (D-056).

No gate path and no dependency change: this task edits no `Cargo.toml` and leaves `Cargo.lock`
as it is (the crates it uses, `tree-sitter`, `tree-sitter-rust` and the dev-dependencies `syn`
and `proc-macro2` of `kiln-cli`, are on main since T-016; that change added 7 packages and also
changed the build-only dependency list of the existing `serde_json` entry, D-056). Arm:
**even** (independent test author).

Reading list: `tasks/T-016-*.md` (SyntaxTree, sexp form, corpus layout);
`crates/kiln-syntax/src/tree.rs` (the merged API: `parse`, `update`, `text`, `has_error`,
`errors`, `sexp`, `node_count`, and the crate-internal `root()` and `char_at_byte` for the
sibling modules); `reviews/T-016-r1.md` S1 and N2; `docs/decisions.md` D-044, D-056;
`crates/kiln-syntax/tests/corpus/MANIFEST.toml`; `tasks/T-015-*.md` (`SyntaxState`, `Parts`);
`crates/kiln-commands/src/reason.rs` (the `reasons!` table); `tasks/T-007-*.md` (b) read-only
property.

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

`SyntaxTree::parse` and `SyntaxTree::update` return `Result<_, ParseError>` (T-016); the only
error is `ParseError::NoTree`, which cannot occur with the pinned versions (T-016 Interface).
If it does, the command fails with `internal`/`other` (message `tree-sitter returned no tree`),
and the cache entry for that document is left as it was (D-044). No `unwrap`/`expect` on these
results.

### Size limit (D-056)
The limit is 1,048,576 chars (2^20; chars as everywhere in the protocol, so
`doc.info`'s `len_chars` is the measure). If the document's text has more chars, each of the
three commands refuses before any parse or update:
- `rejected`, kind `too_large`, message exactly
  `document <doc> has <len_chars> chars; syntax commands accept at most 1048576`;
- the cache entry for that document is dropped (its memory is released), so the next
  successful call on that document reports `mode: "full"`.

A document of exactly 1,048,576 chars is accepted. The check is made on every call against
the current text: a document that shrinks below the limit is accepted again. New reason row,
appended as the last row of the `RejectReason` table in `crates/kiln-commands/src/reason.rs`
(after whatever rows are there when this task is implemented), with a `///` doc line:
`TooLarge = "too_large" => "the document is longer than the command accepts"`; regenerate
`docs/commands.json`. No other file changes for it (kinds come from reasons, T-032).

Why a refusal and not a result without a tree: a refusal keeps the three result schemas as
they are and gives a script one error path keyed by kind, while a "no tree" flag would add a
nullable variant to every result and still leave highlight and brackets without an answer.
Why 2^20 chars: the T-016 review measured 247 MB peak for 1 MB of `(` (about 245 bytes per
char, the worst case it found; `reviews/T-016-r1.md` p6), so one cached tree stays near a
quarter GB at worst, while the largest Rust file in this workspace is 85,052 bytes
(`find . -name '*.rs' -not -path './target/*' -not -path './.worktrees/*' -not -path './.git/*' -printf '%s %p\n' | sort -n | tail -1`
on main `e614fee`: `xtask/src/queue.rs`) and the largest corpus file 6,052 bytes.

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

One new error kind, `too_large` (Size limit above); no other. `docs/protocol/syntax.md`
documents the commands, the span rules, the bracket rule and the size limit.

## Pinned
- (a) Errors: schema → `no_such_document` → `too_large` (whole document, Size limit) →
  `out_of_range` (offset, `start`, `end`) → `inverted_range` (`start > end`); `internal`/`other`
  for `ParseError::NoTree` (D-044). Exact message pinned only for `too_large`; the others keep
  the messages their reasons already have.
- (b) Files: none. Corpus files are read by tests only, by path from the workspace root:
  `crates/kiln-syntax/tests/corpus/` (T-016 Pinned (b)). They are stored as `<name>.rs.txt`
  and are selected by their `set` (`valid` | `gap` | `invalid`) in `MANIFEST.toml`, not by a
  glob on `.rs`.
- (c) Order: n/a beyond (a).
- (d) Offsets are chars; the tree is of the document's current text (BOM excluded).
- (e) Threat model: n/a (no security boundary). Large inputs (D-056): parsing is unbounded in
  memory (T-016 review S1: about 245 bytes per byte of `(`; 50 MB of real Rust peaked at
  3.1 GB), so documents above 1,048,576 chars are refused with `too_large` before any parse
  (Size limit). The earlier "a 10 MB document must answer `syntax.tree`" is withdrawn. The
  reviewer probes that a 50 MB document of `(` is refused without the parse (bounded memory,
  answer within seconds).
- (f) Snapshots: none.
- (g) Worked examples (leaves checked with tree-sitter 0.27.2 / rust 0.24.2 in a spike; offsets
  computed by script). T-016 pins `tree-sitter =0.27.1` / `tree-sitter-rust =0.24.2`; R-067
  found 0.27.1 byte-identical to 0.27.2 on its spike inputs, and R-079 dumped every node (kind,
  field, byte range, named, error) of the texts below with both versions: identical. VERIFIED:
  - `fn f() { let s: u8 = 1; } // c` (30 chars) → spans `0..2 keyword`, `2..3 plain`,
    `3..4 function`, `4..9 plain`, `9..12 keyword`, `12..16 plain`, `16..18 type`,
    `18..21 plain`, `21..22 number`, `22..26 plain`, `26..30 comment`.
  - On the same text `syntax.match-bracket {offset:4}` → `{open:4, close:5}`; `{offset:7}` →
    `{open:7, close:24}`; `{offset:0}` → `null`.
  - `fn a() { (` → `syntax.tree` has `has_error: true`; `match-bracket {offset:9}` → `null`.
  - `fn f() {}` → `sexp` = T-016's example; first call `mode: "full"`, second `unchanged`;
    after `doc.insert {offset:8, text:" "}`, `mode: "incremental"`.
  - Size limit (naive model `len_chars > 1048576`, executed by R-109 with Python on the
    strings below; not run against code): `"//" + "é" * 1048574` has 1,048,576 chars and
    2,097,150 UTF-8 bytes → accepted; one more char (`doc.insert {doc:1, offset:0, text:"x"}`)
    → 1,048,577 → `{"code":"rejected","kind":"too_large","message":"document 1 has 1048577 chars; syntax commands accept at most 1048576"}`;
    `doc.undo {doc:1}` → 1,048,576 → accepted again, `mode: "full"`.
- (h) Evaluator calls: `kiln-cli run` with `doc.open` of a corpus file then `syntax.tree`
  (`syntax.parse`); edits followed by `syntax.tree`, compared with `syntax.tree` of a fresh
  session on the edited text (`syntax.incremental`); `syntax.highlight` (`syntax.highlight`);
  `syntax.match-bracket` (`syntax.brackets`).

## Acceptance criteria
All through `kiln-cli run`/`control` or `kiln_control::Host` on files from the pinned corpus
(`crates/kiln-syntax/tests/corpus/`, selected by MANIFEST `set`), copied to a temp root.
- AC-1 (`syntax.parse`): every `valid` corpus file gives `has_error: false` and `errors: []`.
  Every `invalid` file gives `has_error: true` and ≥ 1 error inside the text. Oracle: rustc (the
  corpus's recorded rustc run, tied to the bytes by hash and checked by T-016's
  `corpus_is_pinned_and_compiles`).
- AC-2 (`syntax.incremental`): for every `valid` file, ≥ 100 fixed-seed random
  `doc.insert`/`doc.delete`/`doc.replace`/`doc.undo` per file, each followed by `syntax.tree`.
  Its `has_error`, `errors`, `sexp` and `node_count` equal `syntax.tree` in a fresh session
  holding the same text (`doc.text`). `mode` is `unchanged` iff the text equals the text at the
  previous `syntax.tree` call for that document (a random `doc.undo` can restore it);
  otherwise, after the first call, `incremental` or `full`, and `incremental` only when the
  result has `has_error: false` (`SyntaxTree::update` reports `Incremental` only for an
  error-free incremental tree and otherwise reparses, so `full` with `has_error: false` is
  allowed). Edits keep every text at or below the size limit. Oracle: from-scratch parse.
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
- AC-8 (`syntax.parse`, size limit; through `kiln-cli run`, cheap: about 2 MB of input, one
  comment token, no large parse): a file holding `//` followed by 1,048,574 `é` (1,048,576
  chars, 2,097,150 bytes, so a byte-based limit fails the test), opened as doc 1:
  `syntax.tree` → `ok`, `has_error: false`, `mode: "full"`; `syntax.match-bracket {offset:0}`
  → `{bracket: null}`. Then `doc.insert {doc:1, offset:0, text:"x"}` and each of
  `syntax.tree`, `syntax.highlight {doc:1}`, `syntax.match-bracket {doc:1, offset:0}` and
  `syntax.match-bracket {doc:1, offset:2000000}` → `rejected`/`too_large` with the exact
  message of the worked example (so `too_large` precedes `out_of_range`). Then
  `doc.undo {doc:1}` and `syntax.tree` → `ok` with `mode: "full"` (the refused call dropped
  the cache entry). Every response line is checked; the `sexp` is not pinned. Oracle: exact
  value + the naive model of (g).

Every criterion is an **acceptance** test: on base no `syntax.*` command exists, so each fails
by assertion (`unknown_command`), and `acceptance_t020.rs` compiles on base (it drives
`kiln-cli` and uses `syn`/`proc-macro2`, already `kiln-cli` dev-dependencies). No guard is
added. No test asserts the number or order of `error_kinds` entries or whole-document bytes.
`kiln-cli` has no `toml` dev-dependency (adding one is outside `touches`): tests read
`MANIFEST.toml` by its `path = "…"` and `set = "…"` lines.

## Scorecard items and intended test names
| Item | Tests |
|---|---|
| syntax.parse | `kiln_cli::acceptance_t020::corpus_parses_through_protocol`, `kiln_cli::acceptance_t020::oversized_documents_are_refused` |
| syntax.incremental | `kiln_cli::acceptance_t020::incremental_equals_fresh_session` |
| syntax.highlight | `kiln_cli::acceptance_t020::highlight_partitions_text`, `kiln_cli::acceptance_t020::highlight_agrees_with_tokenizer`, `kiln_cli::acceptance_t020::highlight_marks_fn_names` |
| syntax.brackets | `kiln_cli::acceptance_t020::brackets_agree_with_token_groups`, `kiln_cli::acceptance_t020::bracket_worked_examples` |

## Out of scope and implementer notes
- Outline, goto-symbol (T-023); semantic highlighting; themes or colours (iteration 2); range
  queries over the tree.
- No-oracle note: the `type` kind has only hand-written expectations (worked example and
  fixtures); no independent tokenizer separates type names from other identifiers.
- Drop cache entries of closed documents on the next `syntax.*` call; ids are never reused.
- The size check reads the document's char length before anything is parsed, updated or
  cached. A memory budget across documents, a limit inside `kiln-syntax`, and the same limit
  for T-023's outline commands are out of scope (T-023's spec should adopt `too_large` when it
  is next revised; noted for the director).
- `crates/kiln-syntax/src/tree.rs` changes only in its unit tests (T-016 review N2, mutant M19):
  add to `common_affixes_respect_char_boundaries` the case where the common byte suffix ends
  inside a multi-byte char: `common_affixes("é", "©") == (0, 0)` (`é` = C3 A9, `©` = C2 A9:
  one common suffix byte that is not a char start). The public and crate-internal API of
  `tree.rs` is used as merged (`root()`, `char_at_byte`); it is not changed. If highlight or
  brackets need a new crate-internal helper there, stop and report instead.
- Your tests: `crates/kiln-syntax/tests/highlight.rs`, unit tests in touched files, unit tests
  of the `TooLarge` row in `syntax.rs`.
- Dispatch: `reason.rs` is shared with T-008, T-010 (in progress) and T-027, so T-020 is never
  in flight with them; the reason row conflicts only by position and is resolved by hand on
  rebase (D-6).

## Questions and rulings
- Director (2026-11-13, D-044): when the parse contract returns `ParseError::NoTree`, the command fails with code `internal`, kind `other`, as proposed.
- Director (2026-11-13, D-056): T-020 first exposes the tree through a command, so its spec
  defines a size limit before dispatch; the T-016 nits (suffix inside a char; "only adds
  packages") go to this spec. Applied by R-109 (Size limit, AC-8, Implementer notes, Goal).

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
- 2026-11-13 · R-066 · draft · did: raises `out_of_range`/`inverted_range` with structured reasons, so `depends_on` gains T-032 (D-041). · gate attempts: 0 · missing context: none · next: director.
- 2026-11-13 · R-079 · draft · did: followed T-016's revised contract (R-077/R-079): corpus
  path `crates/kiln-syntax/tests/corpus/`, `.rs.txt` files selected by MANIFEST `set`; the
  version note in (g); `parse`/`update` return `Result` (handling PROPOSED below); AC-2's mode
  rule per spec check R-067 note 16; AC-1 oracle is the recorded rustc run. · gate attempts: 0
  · missing context: none · next: director.
  QUESTION: map `ParseError::NoTree` in `syntax.*` to `internal`/`other` (proposed; no new
  kind), or another kind? It cannot occur with the pinned versions.
  Re-ran (g)'s trees on 0.27.1 and 0.27.2 in a scratch spike (removed): identical.
- 2026-11-13T12:41Z · R-109 · draft · did: D-056 size limit: chose a structured refusal over a
  no-tree flag (one error path, result schemas unchanged); limit 1,048,576 chars, new row
  `TooLarge`/`too_large` (description and message pinned), checked before any parse, refused
  call drops the cache entry; AC-8 (about 2 MB of `é` in one comment, so cheap and
  char-not-byte); Pinned (a) order, (e) rewritten (the 10 MB requirement withdrawn), (g) size
  example run with Python. T-016 review nits: N2 as a `tree.rs` unit case (tree.rs added to
  `touches`, tests only); N1: T-020 had no "only adds packages" wording, so I added a
  no-dependency statement that describes T-016's lock change correctly. Drift against the
  merged API: "four commands" → three; AC-2's mode rule demanded `incremental` where
  `SyntaxTree::update` may report `full` without errors (fixed to the contract); `reason.rs`
  and `acceptance_t020.rs` added to `touches`; tests cannot use `toml`. Tier stays medium
  (adding a reason row is not a contract change, as for T-010). Pinned (b), (c), (f) remain
  n/a as before. · gate attempts: 0 · not run: no cargo; the limit's memory figure is the
  T-016 review's, not re-measured · missing context: none · next: director; ready for a check.
  QUESTION: should T-023 (outline, also tree-based) adopt `too_large` now, or at its own
  spec check?
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
