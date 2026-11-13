---
id: T-020
title: "Report a document's syntax tree with incremental reparse, highlight spans that partition the text, and bracket matching that ignores strings and comments"
tier: medium            # ADR-0002 default: in-memory behaviour, new command group; tree-sitter and the oracle dev-deps were added by T-016
state: draft
owner_role: planner
scorecard_items: [syntax.parse, syntax.incremental, syntax.highlight, syntax.brackets]
depends_on: [T-015, T-016, T-032]   # all merged (R-157 spec check against main 891ae13)
touches:
  - crates/kiln-syntax/src/highlight.rs       # slot from T-016
  - crates/kiln-syntax/src/brackets.rs        # slot from T-016
  - crates/kiln-syntax/src/tree.rs            # R-109: unit-test case only (T-016 review N2); no API change
  - crates/kiln-syntax/src/lib.rs             # R-157: the crate doc's "Status" paragraph only (it says highlight and brackets are empty)
  - crates/kiln-syntax/tests/highlight.rs     # implementer's own integration tests
  - crates/kiln-commands/src/syntax.rs        # syntax.* + SyntaxState (slot from T-015)
  - crates/kiln-commands/src/reason.rs        # R-109: one row, RejectReason::TooLarge (D-056 size limit); R-157: one unit assertion (Q-6, PENDING)
  - apps/kiln-cli/tests/acceptance_t020_cli.rs  # test author (unique stem across crates, D-084 x6)
  - docs/protocol/syntax.md                   # new
  # docs/commands.json changes: generated (`cargo xtask schema --write`)
budget_tokens: 700000   # medium (v1.2, D-091 change 3)
iteration: 2
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
command parses an unbounded text (D-056). The limit bounds the length of what is parsed, not
the time a parse takes (Pinned (e), Q-3 PENDING).

No gate path and no dependency change: this task edits no `Cargo.toml` and leaves `Cargo.lock`
as it is (the crates it uses, `tree-sitter`, `tree-sitter-rust` and the dev-dependencies `syn`
and `proc-macro2` of `kiln-cli`, are on main since T-016; that change added 7 packages and also
changed the build-only dependency list of the existing `serde_json` entry, D-056). No CLI
subcommand, option or exit code changes, so D-091 change 18 does not apply and
`docs/control-protocol.md` is not edited. Arm: **even** (independent test author).

Reading list: `tasks/T-016-*.md` (SyntaxTree, sexp form, corpus layout);
`crates/kiln-syntax/src/tree.rs` (the merged API: `parse`, `update`, `text`, `has_error`,
`errors`, `sexp`, `node_count`, and the crate-internal `root()` and `char_at_byte` for the
sibling modules); `reviews/T-016-r1.md` S1, N1, N2 and probes p4–p6; `docs/decisions.md` D-044,
D-056, D-084 (ruling x1, optional parameters); `crates/kiln-syntax/tests/corpus/MANIFEST.toml`;
`tasks/T-015-*.md` (`SyntaxState`, `Parts`); `crates/kiln-commands/src/registry.rs`
(`Context::parts`, `Docs::get`, the `Command::run` contract); `crates/kiln-commands/src/reason.rs`
(the `reasons!` table and its unit tests); `tasks/T-007-*.md` (b) read-only property (drafted,
not merged: nothing on main checks it yet).

## Interface

### Commands (`doc` must be open, else `rejected`/`no_such_document`)
| Command | Params | Result |
|---|---|---|
| `syntax.tree` | `{doc}` | `{has_error, errors:[{kind:"error"\|"missing", start, end}], sexp, node_count, mode:"full"\|"incremental"\|"unchanged"}` |
| `syntax.highlight` | `{doc, start?, end?}` | `{spans:[{start, end, kind}]}` |
| `syntax.match-bracket` | `{doc, offset}` | `{bracket: {open, close} \| null}` |

`doc` is an id as in the `doc.*` commands (`u64`; `doc: 0` is `no_such_document`, D-059 q4).
`start`, `end`, `offset` are char offsets (`usize`). `start` and `end` are optional in the sense
of D-084 ruling x1: omitted when absent; `null` is refused as `invalid_params` by the published
schema. `errors` is `SyntaxTree::errors()` in its order (preorder), `sexp` is
`SyntaxTree::sexp()`, `node_count` is `SyntaxTree::node_count()`. The public Rust API that
`highlight.rs` and `brackets.rs` export is the implementer's choice; no test outside
`kiln-syntax` uses it, and it must not change `tree.rs`'s API (Out of scope).

All are `mutating: false`. They keep a cache in `SyntaxState` (per document id: the text and
tree of the last successful `syntax.*` call on that document, whichever of the three it was).
The cache is not session state (director, R-060). The read-only property concerns observable
state: document text, dirty flag, undo history (including an open typing group), ids, the
command log and selections. A command's result never depends on the cache except for `mode`,
which says how the tree was obtained: `full` if no tree was cached for the document,
`unchanged` if the text equals the cached text, `incremental`/`full` per `SyntaxTree::update`
otherwise. (Q-2 PENDING: `mode` is a result that depends on the cache, which T-007 (b)'s
definition of observable state would count; recommended: keep `mode` and name it as the one
exception.)

`SyntaxTree::parse` and `SyntaxTree::update` return `Result<_, ParseError>` (T-016); the only
error is `ParseError::NoTree`, which cannot occur with the pinned versions (T-016 Interface).
If it does, the command fails with `internal`/`other` (`CommandError::Internal { reason:
InternalReason::Other, message }`, message `tree-sitter returned no tree`), and the cache entry
for that document is left as it was (D-044). No `unwrap`/`expect` on these results.

### Size limit (D-056)
The limit is 1,048,576 chars (2^20; chars as everywhere in the protocol, so
`doc.info`'s `len_chars` is the measure). If the document's text has more chars, each of the
three commands refuses before any parse or update:
- `rejected`, kind `too_large`, message exactly
  `document <doc> has <len_chars> chars; syntax commands accept at most 1048576`;
- the cache entry for that document is left as it was, like every other refusal (Q-1 PENDING;
  recommended over R-109's "dropped", which contradicts the `Command::run` contract "On error
  the context must be unchanged", `registry.rs`:328). The cached text is at most the limit, so
  keeping it holds no more than one call at the limit already did.

A document of exactly 1,048,576 chars is accepted. The check is made on every call against
the current text: a document that shrinks back to the limit is accepted again. New reason row,
appended as the last row of the `RejectReason` table in `crates/kiln-commands/src/reason.rs`
(after whatever rows are there when this task is implemented; today the last is `OutsideRoot`),
with a `///` doc line:
`TooLarge = "too_large" => "the document is longer than the command accepts"`; regenerate
`docs/commands.json`. No other file changes for it (kinds come from reasons, T-032). The unit
test `outside_root_is_appended_and_not_a_file_is_widened` in `reason.rs` asserts
`RejectReason::ALL.last() == Some(&RejectReason::OutsideRoot)`, which an appended row makes
false: its first assertion becomes `assert!(RejectReason::ALL.contains(&RejectReason::OutsideRoot))`
and every other assertion of that test stays as it is (Q-6 PENDING; the implementer logs the
edit under golden rule 4, as in D-061).

Why a refusal and not a result without a tree: a refusal keeps the three result schemas as
they are and gives a script one error path keyed by kind, while a "no tree" flag would add a
nullable variant to every result and still leave highlight and brackets without an answer.
Why 2^20 chars: the T-016 review measured 247 MB peak for 1 MB of `(` (`reviews/T-016-r1.md`
p6). That was not the worst case: R-157 parsed texts of exactly 1,048,576 chars with the merged
`tree.rs` in a debug build and measured peak RSS (VmHWM) up to 415 MB after parsing `;`
repeated, 449 MB after also computing `errors`, `sexp` (34 MB) and `node_count`, and 631 MB
while updating `a;` repeated after a one-char insert (old and new tree held at once). A
cached tree at the limit therefore costs up to about 0.4 GB and a call after an edit up to about
0.65 GB measured, plus what the command layer adds (not measured; Pinned (i)). The largest Rust
file in this workspace is 85,052 bytes
(`find . -name '*.rs' -not -path './target/*' -not -path './.worktrees/*' -not -path './.git/*' -printf '%s %p\n' | sort -n | tail -1`
on main `891ae13`: `xtask/src/queue.rs`, unchanged since `e614fee`) and the largest corpus file
6,052 bytes.

### `syntax.highlight` spans (exact)
Spans are sorted, contiguous and non-overlapping. They cover `[start, end)` exactly (default
`0..len_chars`); at the range ends they are clipped. An empty range (`start == end`, or an
empty document) gives `spans: []`. `start`/`end` are plain char offsets: a value between the
`\r` and `\n` of a CRLF is accepted (no `not_a_boundary` check). Adjacent spans of the same kind
are merged. Each char gets one kind, the first rule that applies:
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

(R-157 dump with the pinned grammar: `c"…"`, `cr#"…"#`, `br#"…"#` and `b'x'` are single
`string_literal`/`raw_string_literal`/`char_literal` nodes that include their prefix; `Self` in
`Self::new()` is a named `identifier` in the `path` field, so it is `plain` there, not
`keyword`; the callee of `x.m()` is a `field_identifier`. These follow from the rules as
written; no rule changes.)

### `syntax.match-bracket`
If the char at `offset` is one of `( ) [ ] { }`, and it is an anonymous leaf of the tree (so
not inside a string, char or comment), and it has a partner among the anonymous bracket leaves
that are direct children of the same parent node, then `bracket` is `{open, close}`: the
offsets of the opening and closing char. Partners: the parent's direct anonymous leaves of that
bracket pair are taken left to right, each closing leaf pairs with the nearest unpaired opening
leaf before it (a stack), and a leaf left unpaired has no partner (Q-4 PENDING; R-109's "the
parent node has the matching counterpart leaf" is ambiguous when an `ERROR` parent holds
several, and in `) (` it would pair a `(` with a `)` before it). Otherwise `bracket` is `null`;
for example an unmatched bracket while typing, any other char, or offset `len_chars`. `<`/`>`
are not brackets. Errors: `offset > len_chars` → `out_of_range`.

One new error kind, `too_large` (Size limit above); no other. `docs/protocol/syntax.md`
documents the commands, the span rules, the bracket rule, the size limit and the sentences of
Pinned (i).

## Pinned
- (a) Errors: schema → `no_such_document` → `too_large` (whole document, Size limit) →
  `out_of_range` (`offset`, `start` or `end` beyond `len_chars`) → `inverted_range`
  (`start > end`, both within the text); `internal`/`other` for `ParseError::NoTree` (D-044).
  Exact message pinned only for `too_large`; the messages of the other refusals are not pinned
  (`docs/control-protocol.md`: messages are not stable). A refusal leaves the cache as it was
  (Q-1 PENDING).
- (b) Files: none. Corpus files are read by tests only, by path from the workspace root:
  `crates/kiln-syntax/tests/corpus/` (T-016 Pinned (b)). They are stored as `<name>.rs.txt`
  and are selected by their `set` (`valid` | `gap` | `invalid`) in `MANIFEST.toml`, not by a
  glob on `.rs`. On main: 33 `valid`, 2 `gap`, 10 `invalid`
  (`grep -o '^set = "[a-z]*"' crates/kiln-syntax/tests/corpus/MANIFEST.toml | sort | uniq -c`).
- (c) Order: n/a beyond (a). The size check reads the document's char length before anything
  is parsed, updated or cached.
- (d) Offsets are chars; the tree is of the document's current text (BOM excluded).
- (e) Threat model: n/a (no security boundary). Large and hostile inputs (D-056):
  - Memory: parsing is unbounded in memory (T-016 review S1: about 245 bytes per byte of `(`;
    50 MB of real Rust peaked at 3.1 GB), so documents above 1,048,576 chars are refused with
    `too_large` before any parse (Size limit; figures at the limit there). The earlier "a 10 MB
    document must answer `syntax.tree`" is withdrawn.
  - Time: the limit does not bound time. R-157 measured parse time with the merged `tree.rs`
    (debug build): 1,048,576 `"` took 36.9 s; `#!` repeated took 0.49 s for 8,192 chars,
    1.88 s for 16,384, 7.67 s for 32,768 and 30.3 s for 65,536, about four times as long for
    twice the length, so at the limit it would take hours (extrapolated, not run). After an
    edit that leaves the tree with errors, `SyntaxTree::update` parses twice (incremental, then
    full). No time bound is stated for any `syntax.*` command (Q-3 PENDING).
  - Depth: a valid text can nest deeply within the limit (`fn f() { ` + 500,000 `(` + `1` +
    500,000 `)` + ` }`, 1,000,012 chars: parses without error in 1.3 s, 177 MB, R-157). Every
    walk this task adds over the tree is iterative, as T-016's are (`tree.rs` module doc), so
    deep nesting cannot overflow the stack (AC-10, Q-5 PENDING).
  - The reviewer probes that a 50 MB document of `(` is refused without the parse: same
    script with and without the three refused calls, compared by elapsed time and max RSS
    (`/usr/bin/time -f '%e s %M KB'`). Baseline measured by R-157 with the current `kiln-cli`
    (`doc.open` + `doc.info` of 50,000,000 `(` through `kiln-cli run --root`): 1.12 s,
    168,228 KB. No bound is pinned; the reviewer reports both figures.
- (f) Snapshots: one merged unit test pins the position of the last reason row:
  `crates/kiln-commands/src/reason.rs` `outside_root_is_appended_and_not_a_file_is_widened`
  (`ALL.last() == OutsideRoot`); handled in Size limit (Q-6 PENDING). The tests that read
  `error_kinds` (`acceptance_t032_control.rs` compares it with `RejectReason::ALL`;
  `acceptance_t010_api.rs`, `positions.rs`, `acceptance_t010.rs`, `acceptance_t008_cli.rs`,
  `acceptance_t032_cli.rs` look for given entries) and `acceptance_t003_registry.rs`
  (`docs/commands.json == describe()`) hold after regeneration; the two `rows.len() == 23`
  checks read the table of `docs/control-protocol.md`, which this task does not edit. No test
  on main names a `syntax.*` command.
- (g) Worked examples (leaves checked with tree-sitter 0.27.2 / rust 0.24.2 in a spike; offsets
  computed by script). T-016 pins `tree-sitter =0.27.1` / `tree-sitter-rust =0.24.2`; R-067
  found 0.27.1 byte-identical to 0.27.2 on its spike inputs, and R-079 dumped every node (kind,
  field, byte range, named, error) of the texts below with both versions: identical. R-157
  re-executed the highlight and bracket examples against a naive model (a Python reading of
  the rules above over a node dump made with the pinned 0.27.1 / 0.24.2): identical. VERIFIED:
  - `fn f() { let s: u8 = 1; } // c` (30 chars) → spans `0..2 keyword`, `2..3 plain`,
    `3..4 function`, `4..9 plain`, `9..12 keyword`, `12..16 plain`, `16..18 type`,
    `18..21 plain`, `21..22 number`, `22..26 plain`, `26..30 comment`.
  - On the same text `syntax.match-bracket {offset:4}` → `{open:4, close:5}`; `{offset:7}` →
    `{open:7, close:24}`; `{offset:0}` → `null`.
  - `fn a() { (` → `syntax.tree` has `has_error: true`; `match-bracket {offset:9}` → `null`.
  - `fn f() {}` → `sexp` = T-016's example; first call `mode: "full"`, second `unchanged`;
    after `doc.insert {offset:8, text:" "}`, `mode: "incremental"`.
  - Pending Q-4 (stack pairing; R-157 naive model on the pinned grammar's tree): `) ( ( ) ] [`
    (11 chars; the root's `ERROR` node holds `)`@0, `(`@2, a `unit_expression` with `(`@4 and
    `)`@6, an inner `ERROR` with `]`@8, and `[`@10) → offsets 0, 2, 8, 10 give `null`; 4 and 6
    give `{open:4, close:6}`. `fn a() { ( ) (` → 9 and 11 give `{open:9, close:11}`; 7 and 13
    give `null`.
  - Size limit (naive model `len_chars > 1048576`, executed by R-109 with Python on the
    strings below; the base half re-run by R-157 through the current `kiln-cli run --root`:
    `doc.open` reports `len_chars` 1048576, the insert 1048577, the undo 1048576, `dirty:
    false`; `syntax.*` → `unknown_command`; 0.09 s, 22,468 KB max RSS): `"//" + "é" * 1048574`
    has 1,048,576 chars and 2,097,150 UTF-8 bytes → accepted; one more char
    (`doc.insert {doc:1, offset:0, text:"x"}`) → 1,048,577 →
    `{"code":"rejected","kind":"too_large","message":"document 1 has 1048577 chars; syntax commands accept at most 1048576"}`;
    `doc.undo {doc:1}` → 1,048,576 → accepted again, `mode: "unchanged"` (the refused calls
    left the cache entry of the first call; Q-1 PENDING; under R-109's "dropped" it would be
    `full`).
- (h) Evaluator calls: `kiln-cli run` with `doc.open` of a corpus file then `syntax.tree`
  (`syntax.parse`); edits followed by `syntax.tree`, compared with `syntax.tree` of a fresh
  session on the edited text (`syntax.incremental`); `syntax.highlight` (`syntax.highlight`);
  `syntax.match-bracket` (`syntax.brackets`).
- (i) Sentences this task adds to `docs/protocol/syntax.md` that state a guarantee or a limit,
  for the implementer to copy (D-091 change 5); any further one needs, in the implementer's
  log, the command and output that show it:
  1. "A document longer than 1,048,576 characters (`len_chars` in `doc.info`) is refused by
     every `syntax.*` command with `rejected` / `too_large` before anything is parsed. A
     document of exactly 1,048,576 characters is accepted, and the check is made on every
     call, so a document that shrinks back to the limit is accepted again." Evidence: AC-8.
  2. "`syntax.tree`, `syntax.highlight` and `syntax.match-bracket` are read-only (`mutating:
     false`): they change no document's text, dirty flag, undo history or selection, and they
     do not end a typing group." Evidence: AC-9.
  3. "Every result is the one a parse of the current text from scratch gives; only `mode`
     depends on earlier calls. `mode` is `full` on the first successful `syntax.*` call for a
     document, `unchanged` when the text equals the text of the document's last successful
     `syntax.*` call, and otherwise `incremental` or `full`, where `incremental` only occurs
     for a tree without errors. A refused call does not change what the next call reports."
     Evidence: AC-2, AC-8 (last sentence pending Q-1).
  4. "Highlight spans are sorted, contiguous and non-overlapping, cover the requested range
     exactly, and adjacent spans of the same kind are merged." Evidence: AC-3.
  5. "Brackets inside strings, character literals and comments are never matched; a bracket
     without a partner gives `null`." Evidence: AC-6, AC-7.
  6. "No time bound is given for any `syntax.*` command: parsing time depends on the text, not
     only on its length. In a debug build, 32,768 characters of `#!` repeated took about 8 s
     to parse and 65,536 characters about 30 s, four times as long for twice the length; a call
     after an edit that leaves the tree with errors parses the text twice." Evidence: Pinned
     (e), R-157 spike (Q-3 PENDING).
  7. NEEDS A MEASUREMENT before it is written: a memory sentence. The implementer measures
     max RSS with `/usr/bin/time -f '%e s %M KB' kiln-cli run <script> --root <dir>` on a debug
     build for two documents of exactly 1,048,576 chars, `;` repeated and `a;` repeated, with
     the script `doc.open`, `syntax.tree`, `doc.insert {offset:0, text:" "}`, `syntax.tree`,
     `syntax.highlight`, and records command and output in its log. The sentence then reads:
     "Memory grows with the length and content of the text; the largest peak measured at the
     limit was about <N> MB (a document of 1,048,576 characters of `a;` repeated, or `;`, as
     measured), in a debug build." If no measurement is recorded, no memory sentence is
     written.
  No sentence of `docs/control-protocol.md` changes.

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
  kind is one of the seven; an empty sub-range gives `spans: []`. Oracle: constructed property.
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
- AC-7 (`syntax.brackets`, editing): the worked examples hold (the two pending Q-4 included).
  `offset` = `len_chars` → `null`; `len_chars + 1` → `out_of_range`. Oracle: exact value.
- AC-8 (`syntax.parse`, size limit; through `kiln-cli run`, cheap: about 2 MB of input, one
  comment token, no large parse; base half measured at 0.09 s, Pinned (g)): a file holding `//`
  followed by 1,048,574 `é` (1,048,576 chars, 2,097,150 bytes, so a byte-based limit fails the
  test), opened as doc 1: `syntax.tree` → `ok`, `has_error: false`, `mode: "full"`;
  `syntax.match-bracket {offset:0}` → `{bracket: null}`. Then
  `doc.insert {doc:1, offset:0, text:"x"}` and each of `syntax.tree`, `syntax.highlight {doc:1}`,
  `syntax.match-bracket {doc:1, offset:0}` and `syntax.match-bracket {doc:1, offset:2000000}`
  → `rejected`/`too_large` with the exact message of the worked example (so `too_large`
  precedes `out_of_range`). Then `doc.undo {doc:1}` and `syntax.tree` → `ok` with
  `mode: "unchanged"` (the refused calls left the cache entry of the first call; Q-1 PENDING —
  under R-109's wording this would be `"full"`). `kiln-cli commands --schema` lists an
  `error_kinds.rejected` entry with kind `too_large`. Every response line is checked; the
  `sexp` is not pinned. Oracle: exact value + the naive model of (g).
- AC-9 (read-only; supports `syntax.parse`): `kiln-cli commands --schema` gives `mutating: false`
  for the three commands. In one session on a corpus file: `doc.info` and `cursor.get` before
  and after a sequence of all three commands (valid params and a refused one each) are equal;
  and `doc.insert` of `"a"`, then the three commands, then `doc.insert` of `"b"` at the next
  offset, then one `doc.undo` restores the text from before `"a"` (the typing group was not
  ended). Oracle: exact value (T-007 (b)'s definition of observable state).
- AC-10 (`syntax.brackets`, `syntax.highlight`, deep nesting; Q-5 PENDING): a file holding
  `fn f() { ` + 500,000 `(` + `1` + 500,000 `)` + ` }` (1,000,012 chars): `syntax.tree` → `ok`,
  `has_error: false`; `syntax.highlight` → `ok` with spans covering `0..1000012`;
  `syntax.match-bracket {offset:9}` → `{open:9, close:1000009}` and `{offset:4}` →
  `{open:4, close:5}`. The process exits normally (no stack overflow). Measured by R-157 with
  the merged `tree.rs` (debug): parse 1.3 s, 177 MB; `errors`+`sexp`+`node_count` 0.65 s;
  one iterative walk over every node 0.33 s, 245 MB peak in all. Oracle: exact value.

Every criterion is an **acceptance** test: on base no `syntax.*` command exists, so each fails
by assertion (`unknown_command`), and `acceptance_t020_cli.rs` compiles on base (it drives
`kiln-cli` and uses `syn`/`proc-macro2`, already `kiln-cli` dev-dependencies). No guard is
added. No test asserts the number or order of `error_kinds` entries, whole-document bytes, the
text of `docs/protocol/syntax.md` (the reviewer checks it against Pinned (i) on the diff), the
list of registered commands, or a message other than the `too_large` one (D-091 change 6).
`kiln-cli` has no `toml` dev-dependency (adding one is outside `touches`): tests read
`MANIFEST.toml` by its `path = "…"` and `set = "…"` lines. Every spawned `kiln-cli` has a
timeout (D-028 (8)).

## Scorecard items and intended test names
| Item | Tests |
|---|---|
| syntax.parse | `kiln_cli::acceptance_t020_cli::corpus_parses_through_protocol`, `kiln_cli::acceptance_t020_cli::oversized_documents_are_refused`, `kiln_cli::acceptance_t020_cli::syntax_commands_are_read_only` |
| syntax.incremental | `kiln_cli::acceptance_t020_cli::incremental_equals_fresh_session` |
| syntax.highlight | `kiln_cli::acceptance_t020_cli::highlight_partitions_text`, `kiln_cli::acceptance_t020_cli::highlight_agrees_with_tokenizer`, `kiln_cli::acceptance_t020_cli::highlight_marks_fn_names` |
| syntax.brackets | `kiln_cli::acceptance_t020_cli::brackets_agree_with_token_groups`, `kiln_cli::acceptance_t020_cli::bracket_worked_examples`, `kiln_cli::acceptance_t020_cli::deep_nesting_is_answered` (Q-5 PENDING) |

The director ties the names to the items at the join (METHODOLOGY §6); the test author lists
them in its log.

## Out of scope and implementer notes
- Outline, goto-symbol (T-023); semantic highlighting; themes or colours (iteration 2); range
  queries over the tree.
- No-oracle note: the `type` kind has only hand-written expectations (worked example and
  fixtures); no independent tokenizer separates type names from other identifiers.
- Drop cache entries of closed documents on the next successful `syntax.*` call; ids are never
  reused, so this is not observable.
- A memory budget across documents, a limit inside `kiln-syntax`, a time or work cap on
  parsing (Q-3), and the same limit for T-023's outline commands are out of scope (T-023's spec
  should adopt `too_large` when it is next revised; noted for the director).
- Refusal declarations per command (T-038): T-038 runs after this task and declares the
  `syntax.*` refusals itself (D-092); this task adds none.
- `crates/kiln-syntax/src/tree.rs` changes only in its unit tests (T-016 review N2, mutant M19):
  add to `common_affixes_respect_char_boundaries` the case where the common byte suffix ends
  inside a multi-byte char: `common_affixes("é", "©") == (0, 0)` (`é` = C3 A9, `©` = C2 A9:
  one common suffix byte that is not a char start). The public and crate-internal API of
  `tree.rs` is used as merged (`root()`, `char_at_byte`); it is not changed, and their
  `#[allow(dead_code)]` attributes may stay. There is no char → byte helper: `brackets.rs` and
  `highlight.rs` convert from `text()` themselves. If highlight or brackets need a new
  crate-internal helper in `tree.rs`, stop and report instead.
- `crates/kiln-syntax/src/lib.rs`: only the "Status" paragraph of the crate doc, which says
  `highlight` and `brackets` are still empty; no change to its items or re-exports.
- `syntax.rs` reaches the document and `SyntaxState` together through `Context::parts()`
  (`registry.rs`, `pub(crate)`); `registry.rs` is not edited.
- Your tests: `crates/kiln-syntax/tests/highlight.rs`, unit tests in touched files, unit tests
  of the `TooLarge` row in `syntax.rs`. Every command has at least one `Example` (the audit runs
  it on a `MemoryFs`).
- Dispatch: `reason.rs` is shared with T-027 (stage 2, a mapping), T-038 (runs after this task)
  and T-019 only if its Q-4 is ruled (B); never in flight with them. The reason row conflicts
  only by position and is resolved by hand on rebase (D-6). T-021 adds no reason row and shares
  no path with this task except the generated `docs/commands.json`; T-019 the same (plus its
  own `Cargo.lock`, not touched here).

## Questions and rulings
- Director (2026-11-13, D-044): when the parse contract returns `ParseError::NoTree`, the command fails with code `internal`, kind `other`, as proposed.
- Director (2026-11-13, D-056): T-020 first exposes the tree through a command, so its spec
  defines a size limit before dispatch; the T-016 nits (suffix inside a char; "only adds
  packages") go to this spec. Applied by R-109 (Size limit, AC-8, Implementer notes, Goal).

Spec check R-157 (against main `891ae13`; the spec is written for each recommendation, marked
PENDING, until the director rules):
- Q-1 (planner, R-157) — the cache on a `too_large` refusal. (A) Leave it as it was, like
  every refusal: consistent with `Command::run` ("On error the context must be unchanged",
  `registry.rs`:328) and `docs/control-protocol.md` ("a refused request leaves the session
  unchanged"); the next call after the text returns to the cached text reports `unchanged`;
  the kept entry is at most the limit. (B) Drop it (R-109): releases one tree of at most the
  limit, but is a change made by a failing command, observable through `mode`, and contradicts
  both sentences above. Recommended: (A). Written: Size limit, Pinned (a), (g), (i) 3, AC-8.
- Q-2 (planner, R-157) — `mode` exposes the cache. R-060 ruled the cache is not state because
  "no command result exposes" it, but `mode` does: `syntax.tree` twice gives `full`, then
  `unchanged`, so by T-007 (b)'s definition ("what some command result can expose") a
  read-only command would change observable state. (A) Keep `mode`; rule that the syntax cache
  is observable only through `mode`, which T-007 (b) excludes by name (one sentence in T-007's
  spec, not this task's file). (B) Remove `mode`: `syntax.incremental` is then observable only
  as equality with a fresh parse, and whether a tree was reused cannot be seen through any
  interface (the evaluator's "wired but inert" risk). (C) Declare the three commands
  mutating: the cache then is state, but `mutating: true` misdescribes read-only queries and
  every log entry claims a change that did not happen. Recommended: (A). Written:
  Interface (unchanged text, marked PENDING).
- Q-3 (planner, R-157) — time is not bounded at the limit (Pinned (e): `#!` repeated grows
  with the square of the length, 30 s for 65,536 chars in a debug build; hours at the limit,
  extrapolated). (a) Document it (Pinned (i) 6), no time bound, keep 2^20 chars; a deterministic
  work cap in `kiln-syntax` (for instance cancelling a parse from tree-sitter's progress
  callback after a count of calls, not a clock: CHARTER §3 decision 4) becomes a follow-up
  task that changes T-016's API. (b) Lower the limit: no length keeps `#!` input fast that does
  not also refuse ordinary files (16,384 chars already take 1.9 s; this workspace has a
  85,052-byte file). (c) Add the work cap in this task: changes `tree.rs`'s API and T-016's
  contract, outside `touches`. The memory rationale for 2^20 is also corrected (Size limit:
  up to 0.65 GB measured, not "a quarter GB"); halving the limit would halve that. Recommended:
  (a), with the limit kept. Written: Goal, Pinned (e), (i) 6, Out of scope.
- Q-4 (planner, R-157) — bracket partner when the parent holds several brackets of one pair
  (only `ERROR` parents do on the pinned grammar; in `) ( ( ) ] [` R-109's rule would pair
  `(`@2 with `)`@0). (a) Stack pairing over the parent's direct bracket leaves, left to right;
  unpaired → `null`. (b) A partner only when the parent has exactly one opening and one closing
  leaf of that pair, the opening first; else `null`. (c) Leave it unspecified: the test author
  cannot test error trees and the implementer chooses. Recommended: (a) (it equals (b) wherever
  (b) answers, and also pairs `( ) (`). Written: match-bracket rule, Pinned (g), AC-7.
- Q-5 (planner, R-157) — deep nesting. A valid 1,000,012-char text nests 500,000 deep (Pinned
  (e)); a recursive walk would overflow the stack and abort `kiln-cli` (not a catchable
  panic). (A) Add AC-10 and the iterative-walk requirement (cheap: 1.3 s parse measured).
  (B) No criterion; the reviewer probes it. Recommended: (A). Written: Pinned (e), AC-10,
  test name.
- Q-6 (planner, R-157) — the merged unit test pins `OutsideRoot` as the last reason row.
  (A) Append `TooLarge` last (the documented convention, "Later tasks append rows") and change
  that one assertion to `ALL.contains(&OutsideRoot)`, logged under golden rule 4 (precedent
  D-061; the pin is the kind D-091 change 6 forbids: a list a drafted task changes). (B) Insert
  `TooLarge` before `OutsideRoot` so no merged test is edited; breaks the append convention and
  the next task that appends hits the same test. Recommended: (A). Written: Size limit,
  `touches` comment, Pinned (f).
- Note for the director (not a question for this task): METHODOLOGY §5.2 forbids "equal"
  `touches` for in-flight tasks; T-019, T-020 and T-021 all change the generated
  `docs/commands.json`, which `xtask/generated.toml` regenerates on conflict and the
  `touches` check exempts. D-092's plan to run them in parallel needs that exemption stated.
- Split (D-091): not recommended. The four items share one module (`syntax.rs`), one cache,
  the size limit, the reason row, one document and one acceptance file; split tasks would each
  touch `syntax.rs`, `syntax.md` and `docs/commands.json` and so run one after another anyway,
  each with its own test-author and review cycle (three cycles instead of one). Splitting
  highlight out would isolate its rule-heavy review at the cost of one more cycle; the
  split-before-dispatch rule binds high-tier bundles only, and this task is medium.
### Rulings on Q-1 to Q-6 of the spec check R-157 (director, D-097)
- Q-1: keep. A refused request leaves the session as it was, the cache entry included (`Command::run`'s contract and `docs/control-protocol.md`).
- Q-2: (A). `mode` stays in the result; it is the one place where a result shows the cache, because without it `syntax.incremental` could not be observed through a public interface. The sentence that names this exception is added to T-007's spec when that task is checked before dispatch; this task does not edit T-007.
- Q-3: (a). The size limit stays at 1,048,576 chars and bounds memory, not time. The documents state that no time bound exists and give the measured figures; every figure in a document is measured by the implementer through `kiln-cli`, on a release build, with the command recorded (the spec check's figures come from a debug spike outside the repository and are not copied into documents). A deterministic work cap is a follow-up task, filed by the director; this task does not attempt it.
- Q-4: as recommended. Bracket partners are found by pairing the parent's direct bracket leaves left to right with a stack.
- Q-5: yes, AC-10 for deep nesting; every new walk over the tree is iterative.
- Q-6: as recommended. `TooLarge` is appended as the last row, and the one assertion in `reason.rs`'s unit test that pins `OutsideRoot` as the last row is changed to assert that the row is present; the edit is logged under golden rule 4 as in D-061.
- Generated files and parallel work: `docs/commands.json` is regenerated by the queue at every merge, so two tasks in flight that both change it do not count as overlapping `touches` (METHODOLOGY §5.2 concerns files that builders edit by hand).

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
- 2026-11-13T21:36Z · R-157 · draft · spec check: read against main `891ae13` (T-008, T-010,
  T-011, T-016, T-017, T-018, T-032, T-035 merged; `depends_on` all merged). did: blockers
  fixed in the text: the merged unit test `outside_root_is_appended_and_not_a_file_is_widened`
  (`reason.rs`) pins `OutsideRoot` as the last row, so the appended `TooLarge` row turns the
  implementer's gate red (Q-6); the acceptance file `acceptance_t020.rs` → unique stem
  `acceptance_t020_cli.rs` (D-084 x6) and all test names; `crates/kiln-syntax/src/lib.rs` added
  to `touches` (its Status paragraph says highlight/brackets are empty); the size limit's memory
  rationale ("about 245 B/char, the worst case", "a quarter GB at worst") is contradicted by
  measurement (415 MB after parsing 2^20 `;`, 631 MB updating 2^20 `a;`) and corrected; the
  limit bounds no time (`#!` repeated is quadratic: 30.3 s for 65,536 chars; 1,048,576 `"`
  36.9 s), so no time bound is stated (Q-3); the bracket rule is ambiguous under `ERROR`
  parents holding several brackets (Q-4); a refusal that drops the cache contradicts the
  `Command::run` contract (Q-1); `mode` exposes the cache against R-060's premise (Q-2); deep
  nesting (500,000 levels, valid, within the limit) needs iterative walks (Q-5, AC-10).
  Mechanical: `budget_tokens: 700000`, `iteration: 2`; D-084 x1 for `start?`/`end?`; empty
  range → `spans: []`; no `not_a_boundary` for highlight ranges; Pinned (f) lists the merged
  tests that read `error_kinds`; Pinned (i) written (D-091 change 5), one memory sentence marked
  as needing the implementer's measurement; change 6 (no test reads `syntax.md` or pins other
  messages); change 18 n/a (no CLI change); stale dispatch note (T-008, T-010 merged) updated;
  T-007 noted as unmerged; AC-9 (read-only, kind published) added as the evidence for sentence
  2. Tier stays medium (no ADR-0002 high criterion; adding a reason row is not a contract
  change, as for T-010). Pinned (b), (c) n/a in substance; (e) n/a as security. Split: not
  recommended (shared module, cache, limit, row, document). Parallel: T-021 and T-019 share
  only the generated `docs/commands.json` (T-019 also only if its Q-4 is (A)). · gate attempts:
  0 · not run: no cargo in the repository. Probes: `target/debug/kiln-cli run --root` on the
  AC-8 fixture (0.09 s, 22,468 KB; base gives `unknown_command`) and on 50,000,000 `(` (1.12 s,
  168,228 KB); a scratch spike outside the repository linking the merged `tree.rs` by path
  (tree-sitter =0.27.1, tree-sitter-rust =0.24.2, offline, debug) for parse, `sexp`, walk and
  update figures at 2^20 chars, quadratic `#!`, deep nesting, and node dumps for the worked
  examples, which a Python naive model of the highlight and bracket rules reproduced exactly.
  Memory through `kiln-cli` (JSON of a 34 MB `sexp`, 1M spans) not measured. · missing
  context: none · next: director rules Q-1 to Q-6.
  QUESTION: Q-1 (cache on refusal), Q-2 (`mode` versus T-007 (b)), Q-3 (no time bound),
  Q-4 (bracket pairing), Q-5 (deep-nesting criterion), Q-6 (the `OutsideRoot`-last unit
  assertion) — see "Questions and rulings".
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
