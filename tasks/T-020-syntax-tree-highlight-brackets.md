---
id: T-020
title: "Report a document's syntax tree with incremental reparse, highlight spans that partition the text, and bracket matching that ignores strings and comments"
tier: medium            # ADR-0002 default: in-memory behaviour, new command group; tree-sitter and the oracle dev-deps were added by T-016
state: queued
owner_role: planner
scorecard_items: [syntax.parse, syntax.incremental, syntax.highlight, syntax.brackets]
depends_on: [T-015, T-016, T-032]   # all merged (spec check R-157 on main 891ae13; re-read R-165 on c538ef8)
touches:
  - crates/kiln-syntax/src/highlight.rs       # slot from T-016
  - crates/kiln-syntax/src/brackets.rs        # slot from T-016
  - crates/kiln-syntax/src/tree.rs            # R-109: unit-test case only (T-016 review N2); no API change
  - crates/kiln-syntax/src/lib.rs             # R-157: the crate doc's "Status" paragraph only (it says highlight and brackets are empty)
  - crates/kiln-syntax/tests/highlight.rs     # implementer's own integration tests
  - crates/kiln-commands/src/syntax.rs        # syntax.* + SyntaxState (slot from T-015)
  - crates/kiln-commands/src/reason.rs        # R-109: one row, RejectReason::TooLarge (D-056 size limit); R-157/D-097 Q-6: one unit assertion loosened to presence
  - apps/kiln-cli/tests/acceptance_t020_cli.rs  # test author only (unique stem across crates, D-084 x6); every AC; fails on base
  # apps/kiln-cli/tests/guard_t020_cli.rs: test author only, if it adds a guard (none required); an allowed path
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
command parses an unbounded text (D-056). The limit bounds memory, not time: no time bound
exists for any `syntax.*` command, and the documents say so (Pinned (e), (i); D-097 Q-3).

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
otherwise. `mode` is the one place where a result shows the cache (D-097 Q-2): without it
`syntax.incremental` could not be observed through a public interface. The sentence naming
this exception goes into T-007's spec at T-007's own check; this task does not edit T-007.

`SyntaxTree::parse` and `SyntaxTree::update` return `Result<_, ParseError>` (T-016); the only
error is `ParseError::NoTree`, which cannot occur with the pinned versions (T-016 Interface).
If it does, the command fails with `internal`/`other` (`CommandError::Internal { reason:
InternalReason::Other, message }`, message `tree-sitter returned no tree`, not asserted by any
test), and the cache entry
for that document is left as it was (D-044). No `unwrap`/`expect` on these results.

### Size limit (D-056)
The limit is 1,048,576 chars (2^20; chars as everywhere in the protocol, so
`doc.info`'s `len_chars` is the measure). If the document's text has more chars, each of the
three commands refuses before any parse or update:
- `rejected`, kind `too_large`, with the message
  `document <doc> has <len_chars> chars; syntax commands accept at most 1048576` (the wording
  the implementer uses; messages are not stable, and no test asserts this text: tests assert
  `code` and `kind` only);
- the cache entry for that document is left as it was, like every other refusal (D-097 Q-1;
  `Command::run`'s contract "On error the context must be unchanged", `registry.rs`:328, and
  `docs/control-protocol.md`). The cached text is at most the limit, so keeping it holds no more
  than one call at the limit already did.

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
and every other assertion of that test stays as it is (D-097 Q-6; the implementer logs the
edit under golden rule 4, as in D-061).

Why a refusal and not a result without a tree: a refusal keeps the three result schemas as
they are and gives a script one error path keyed by kind, while a "no tree" flag would add a
nullable variant to every result and still leave highlight and brackets without an answer.
Why 2^20 chars (evidence for this spec only; none of these figures goes into a document,
D-097 Q-3): the T-016 review measured 247 MB peak for 1 MB of `(` (`reviews/T-016-r1.md`
p6). That was not the worst case: R-157 parsed texts of exactly 1,048,576 chars with the merged
`tree.rs` in a debug spike outside the repository and measured peak RSS (VmHWM) up to 415 MB after parsing `;`
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
leaf before it (a stack), and a leaf left unpaired has no partner (D-097 Q-4; it replaces
R-109's "the parent node has the matching counterpart leaf", which was ambiguous when an
`ERROR` parent holds several). On every parent the pinned grammar builds for valid Rust there
is at most one pair of each kind, so the stack only matters under `ERROR` nodes. Otherwise `bracket` is `null`;
for example an unmatched bracket while typing, any other char, or offset `len_chars`. `<`/`>`
are not brackets. Errors: `offset > len_chars` → `out_of_range`.

One new error kind, `too_large` (Size limit above); no other. `docs/protocol/syntax.md`
documents the commands, the span rules, the bracket rule, the size limit and the sentences of
Pinned (i).

## Pinned
- (a) Errors: schema → `no_such_document` → `too_large` (whole document, Size limit) →
  `out_of_range` (`offset`, `start` or `end` beyond `len_chars`) → `inverted_range`
  (`start > end`, both within the text); `internal`/`other` for `ParseError::NoTree` (D-044).
  No message is pinned for tests: tests assert `code` and `kind` (and, for `too_large`, nothing
  more); `docs/control-protocol.md` says messages are not stable. The `too_large` wording in
  Size limit is the implementer's text, not a test oracle. A refusal leaves the cache as it was
  (D-097 Q-1).
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
  - Time: the limit does not bound time (D-097 Q-3). Evidence for this spec only, not text for
    any document: R-157 measured parse time with the merged `tree.rs` in a debug spike outside
    the repository: 1,048,576 `"` took 36.9 s; `#!` repeated took 0.49 s for 8,192 chars,
    1.88 s for 16,384, 7.67 s for 32,768 and 30.3 s for 65,536, about four times as long for
    twice the length, so at the limit it would take hours (extrapolated, not run). After an
    edit that leaves the tree with errors, `SyntaxTree::update` parses twice (incremental, then
    full). No time bound exists for any `syntax.*` command, and no test asserts a time. A
    deterministic work cap is a follow-up task the director files; it is not specified here.
  - Depth: a valid text can nest deeply within the limit (`fn f() { ` + 500,000 `(` + `1` +
    500,000 `)` + ` }`, 1,000,012 chars: parses without error in 1.3 s, 177 MB in R-157's debug
    spike; spec evidence only). Every walk this task adds over the tree is iterative, as
    T-016's are (`tree.rs` module doc), so deep nesting cannot overflow the stack (AC-10,
    D-097 Q-5).
  - The reviewer probes that a 50 MB document of `(` is refused without the parse: same
    script with and without the three refused calls, compared by elapsed time and max RSS
    (`/usr/bin/time -f '%e s %M KB'`). Baseline measured by R-157 with the current `kiln-cli`
    (`doc.open` + `doc.info` of 50,000,000 `(` through `kiln-cli run --root`): 1.12 s,
    168,228 KB (debug binary). No bound is pinned; the reviewer reports both figures.
- (f) Snapshots: one merged unit test pins the position of the last reason row:
  `crates/kiln-commands/src/reason.rs` `outside_root_is_appended_and_not_a_file_is_widened`
  (`ALL.last() == OutsideRoot`); handled in Size limit (D-097 Q-6). The tests that read
  `error_kinds` (`acceptance_t032_control.rs` compares it with `RejectReason::ALL`;
  `acceptance_t010_api.rs`, `positions.rs`, `acceptance_t010.rs`, `acceptance_t008_cli.rs`,
  `acceptance_t032_cli.rs` look for given entries) and `acceptance_t003_registry.rs`
  (`docs/commands.json == describe()`) hold after regeneration; the two `rows.len() == 23`
  checks read the table of `docs/control-protocol.md`, which this task does not edit. T-039's
  merged tests (`acceptance_t039_cli.rs`: option sets against `kiln-cli --help`;
  `guard_t039_cli.rs`: no-op `doc.undo`/`doc.redo` behaviour) are untouched by a new command
  group (R-165, main `c538ef8`). No test
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
  - Stack pairing (D-097 Q-4; R-157 naive model on the pinned grammar's tree): `) ( ( ) ] [`
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
    `{"code":"rejected","kind":"too_large","message":"document 1 has 1048577 chars; syntax commands accept at most 1048576"}`
    (the message shown is the implementer's wording; tests check `code` and `kind` only);
    `doc.undo {doc:1}` → 1,048,576 → accepted again, `mode: "unchanged"` (the refused calls
    left the cache entry of the first call, D-097 Q-1).
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
     Evidence: AC-2, AC-8.
  4. "Highlight spans are sorted, contiguous and non-overlapping, cover the requested range
     exactly, and adjacent spans of the same kind are merged." Evidence: AC-3.
  5. "Brackets inside strings, character literals and comments are never matched; a bracket
     without a partner gives `null`." Evidence: AC-6, AC-7.
  6. Time (D-097 Q-3). Fixed part: "No time bound exists for any `syntax.*` command: the size
     limit bounds memory, not time. Parsing time depends on the text, not only on its length,
     and a call after an edit that leaves the tree with errors parses the text twice."
     Evidence: Pinned (e) (the twice-parse is `SyntaxTree::update`'s documented behaviour).
     Measured part, written only with the implementer's own figures: the implementer builds
     `kiln-cli` in release (`cargo build --release -p kiln-cli` in its worktree) and runs, for
     files of `#!` repeated to 16,384, 32,768 and 65,536 chars, a script of `doc.open` and
     `syntax.tree` under `/usr/bin/time -f '%e s %M KB' target/release/kiln-cli run <script>
     --root <dir>`, and records the commands and outputs in its log. The sentence then reads:
     "Measured with a release build of `kiln-cli` (`syntax.tree` after `doc.open`): a text of
     `#!` repeated took <t1> s at 16,384 characters, <t2> s at 32,768 and <t3> s at 65,536."
     with the three measured values. R-157's debug figures in Pinned (e) are not copied.
  7. Memory (D-097 Q-3). Written only with the implementer's figures: on the same release
     build, max RSS of `kiln-cli run <script> --root <dir>` under `/usr/bin/time -f '%e s %M KB'`
     for two documents of exactly 1,048,576 chars, `;` repeated and `a;` repeated, with the
     script `doc.open`, `syntax.tree`, `doc.insert {doc:1, offset:0, text:" "}`, `syntax.tree`,
     `syntax.highlight {doc:1}`; command and output in the log. The sentence then reads:
     "Memory grows with the length and content of the text; measured with a release build of
     `kiln-cli`, the largest peak at the limit was about <N> MB (1,048,576 characters of
     <`;` or `a;`> repeated, after one edit)." Both measurements are required (D-097 Q-3: the
     documents give the measured figures); a document figure without its command and output in
     the implementer's log is a review finding, and the reviewer re-runs the commands.
  No sentence of `docs/control-protocol.md` changes.

## Acceptance criteria
Acceptance-test protocol (medium tier, even task number: METHODOLOGY §6, D-035 (16)). An
independent test author writes every acceptance test in **`apps/kiln-cli/tests/acceptance_t020_cli.rs`**
on `task/T-020-tests`, and any regression guard in `apps/kiln-cli/tests/guard_t020_cli.rs`
(none is required by this spec). The implementer adds no `acceptance_t*` or `guard_t*` file
and does not read the tests branch before the first join. Every criterion below is tested in
`acceptance_t020_cli.rs` and **fails on base** by assertion: on base no `syntax.*` command
exists, so each request gets `unknown_command` (checked by R-157 on main's binary). All drive
`kiln-cli run`/`control` (or `kiln_control::Host`) on files from the pinned corpus
(`crates/kiln-syntax/tests/corpus/`, selected by MANIFEST `set`) or on the files named,
copied to a temp root.
- AC-1 (`syntax.parse`): every `valid` corpus file gives `has_error: false` and `errors: []`.
  Every `invalid` file gives `has_error: true` and ≥ 1 error inside the text. Oracle: rustc (the
  corpus's recorded rustc run, tied to the bytes by hash and checked by T-016's
  `corpus_is_pinned_and_compiles`). File `acceptance_t020_cli.rs`; fails on base.
- AC-2 (`syntax.incremental`): for every `valid` file, ≥ 100 fixed-seed random
  `doc.insert`/`doc.delete`/`doc.replace`/`doc.undo` per file, each followed by `syntax.tree`.
  Its `has_error`, `errors`, `sexp` and `node_count` equal `syntax.tree` in a fresh session
  holding the same text (`doc.text`). `mode` is `unchanged` iff the text equals the text at the
  previous `syntax.tree` call for that document (a random `doc.undo` can restore it);
  otherwise, after the first call, `incremental` or `full`, and `incremental` only when the
  result has `has_error: false` (`SyntaxTree::update` reports `Incremental` only for an
  error-free incremental tree and otherwise reparses, so `full` with `has_error: false` is
  allowed). Edits keep every text at or below the size limit. Oracle: from-scratch parse.
  File `acceptance_t020_cli.rs`; fails on base.
- AC-3 (`syntax.highlight`, partition): for every corpus file and ≥ 50 random sub-ranges, the
  spans are sorted, contiguous, non-overlapping, merged and cover the range exactly, and every
  kind is one of the seven; an empty sub-range gives `spans: []`. Oracle: constructed property.
  File `acceptance_t020_cli.rs`; fails on base.
- AC-4 (`syntax.highlight`, differential with proc-macro2): on every `valid` file, tokenize with
  `proc_macro2` (span locations, `byte_range`). Consider only `Literal` tokens whose source
  slice equals the token's text, which excludes desugared doc comments. Those slices equal the
  union of `string` and `number` spans: string, byte string, char and C string literals are
  `string`, numeric ones `number`. Every `keyword` span's text is in the pinned list. Every
  `comment` span starts with `//` or `/*`. Oracle: an independent tokenizer. File
  `acceptance_t020_cli.rs`; fails on base.
- AC-5 (`syntax.highlight`, functions and types): on every `valid` file, each fn definition
  name found by `syn` (free fns, impl and trait methods, outside fn bodies) is a `function`
  span at the ident's offset. The worked example holds exactly. Oracle: syn + exact value.
  File `acceptance_t020_cli.rs`; fails on base.
- AC-6 (`syntax.brackets`, differential): on every `valid` file, take each proc-macro2 `Group`
  delimited by parentheses, brackets or braces whose source chars at the open and close span
  are that delimiter (doc-comment groups excluded). `match-bracket` at the open offset and at
  the close offset both return that pair. At every other offset whose char is a bracket,
  `bracket` is `null`, because those brackets are in strings, chars or comments. Oracle: an
  independent tokenizer. File `acceptance_t020_cli.rs`; fails on base.
- AC-7 (`syntax.brackets`, editing): the worked examples of Pinned (g) hold, the two
  stack-pairing examples included. `offset` = `len_chars` → `null`; `len_chars + 1` →
  `rejected`/`out_of_range` (`code` and `kind` asserted). Oracle: exact value. File
  `acceptance_t020_cli.rs`; fails on base.
- AC-8 (`syntax.parse`, size limit; through `kiln-cli run`; about 2 MB of input, one comment
  token, no large parse): a file holding `//`
  followed by 1,048,574 `é` (1,048,576 chars, 2,097,150 bytes, so a byte-based limit fails the
  test), opened as doc 1: `syntax.tree` → `ok`, `has_error: false`, `mode: "full"`;
  `syntax.match-bracket {offset:0}` → `{bracket: null}`. Then
  `doc.insert {doc:1, offset:0, text:"x"}` and each of `syntax.tree`, `syntax.highlight {doc:1}`,
  `syntax.match-bracket {doc:1, offset:0}` and `syntax.match-bracket {doc:1, offset:2000000}`
  → `code: "rejected"`, `kind: "too_large"` (so `too_large` precedes `out_of_range`; the
  message is not asserted). Then `doc.undo {doc:1}` and `syntax.tree` → `ok` with
  `mode: "unchanged"` (the refused calls left the cache entry of the first call, D-097 Q-1).
  `kiln-cli commands --schema` lists an `error_kinds.rejected` entry with kind `too_large`
  (its `when` text is not asserted). Every response line is checked; the `sexp` is not pinned.
  Oracle: exact value + the naive model of (g). File `acceptance_t020_cli.rs`; fails on base.
- AC-9 (read-only; supports `syntax.parse`): `kiln-cli commands --schema` gives `mutating: false`
  for the three commands. In one session on a corpus file: `doc.info` and `cursor.get` before
  and after a sequence of all three commands (valid params and a refused one each) are equal;
  and `doc.insert` of `"a"`, then the three commands, then `doc.insert` of `"b"` at the next
  offset, then one `doc.undo` restores the text from before `"a"` (the typing group was not
  ended). Oracle: exact value (T-007 (b)'s definition of observable state; `mode` is the
  named exception, D-097 Q-2, and is not compared). File `acceptance_t020_cli.rs`; fails on
  base (the three commands are missing from `commands --schema`).
- AC-10 (`syntax.brackets`, `syntax.highlight`, deep nesting; D-097 Q-5): a file holding
  `fn f() { ` + 500,000 `(` + `1` + 500,000 `)` + ` }` (1,000,012 chars): `syntax.tree` → `ok`,
  `has_error: false`; `syntax.highlight` → `ok` with spans covering `0..1000012`;
  `syntax.match-bracket {offset:9}` → `{open:9, close:1000009}` and `{offset:4}` →
  `{open:4, close:5}`. The process exits normally (no stack overflow). No time is asserted.
  (Spec evidence only: R-157's debug spike parsed this text without error, the work of
  `syntax.tree` plus one full walk took about 2.3 s and 245 MB.) Oracle: exact value. File
  `acceptance_t020_cli.rs`; fails on base.

`acceptance_t020_cli.rs` compiles on base (it drives `kiln-cli` and uses `syn`/`proc-macro2`,
already `kiln-cli` dev-dependencies). No test asserts a message text, a time, the number or
order of `error_kinds` entries, a `when` text, whole-document bytes, the text of
`docs/protocol/syntax.md` (the reviewer checks it against Pinned (i) on the diff) or the list
of registered commands (D-091 change 6). `kiln-cli` has no `toml` dev-dependency (adding one is
outside `touches`): tests read `MANIFEST.toml` by its `path = "…"` and `set = "…"` lines. Every
spawned `kiln-cli` has a timeout (D-028 (8)); it is a **hang detector**, not an assertion about
speed: it is set generously, labelled as a hang detector in the test, and its failure message
says the process hung.

## Scorecard items and intended test names
| Item | Tests |
|---|---|
| syntax.parse | `kiln_cli::acceptance_t020_cli::corpus_parses_through_protocol`, `kiln_cli::acceptance_t020_cli::oversized_documents_are_refused`, `kiln_cli::acceptance_t020_cli::syntax_commands_are_read_only` |
| syntax.incremental | `kiln_cli::acceptance_t020_cli::incremental_equals_fresh_session` |
| syntax.highlight | `kiln_cli::acceptance_t020_cli::highlight_partitions_text`, `kiln_cli::acceptance_t020_cli::highlight_agrees_with_tokenizer`, `kiln_cli::acceptance_t020_cli::highlight_marks_fn_names` |
| syntax.brackets | `kiln_cli::acceptance_t020_cli::brackets_agree_with_token_groups`, `kiln_cli::acceptance_t020_cli::bracket_worked_examples`, `kiln_cli::acceptance_t020_cli::deep_nesting_is_answered` |

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
  parsing (a follow-up task the director files, D-097 Q-3), and the same limit for T-023's outline commands are out of scope (T-023's spec
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
  of the `TooLarge` row in `syntax.rs`. No `acceptance_t*` or `guard_t*` file (the test
  author's). Measure and log the figures of Pinned (i) 6 and 7 on a release build. Every command has at least one `Example` (the audit runs
  it on a `MemoryFs`).
- Dispatch: `reason.rs` is shared with T-027 (stage 2, a mapping), T-038 (runs after this task)
  (T-019's Q-4 was ruled (A): no `reason.rs` change, D-095); never in flight with them. The
  reason row conflicts only by position and is resolved by hand on rebase (D-6). T-019 and
  T-021 share only the generated `docs/commands.json`, which does not count as overlapping
  `touches` (D-097), so both may be in flight with this task.

## Questions and rulings
- Director (2026-11-13, D-044): when the parse contract returns `ParseError::NoTree`, the command fails with code `internal`, kind `other`, as proposed.
- Director (2026-11-13, D-056): T-020 first exposes the tree through a command, so its spec
  defines a size limit before dispatch; the T-016 nits (suffix inside a char; "only adds
  packages") go to this spec. Applied by R-109 (Size limit, AC-8, Implementer notes, Goal).

Spec check R-157 (against main `891ae13`) asked six questions; the options it weighed are in
its log entry and report. Q-1: the cache on a `too_large` refusal (keep, or drop as R-109
wrote). Q-2: `mode` shows the cache, against R-060's premise. Q-3: the size limit does not bound
time (`#!` repeated, quadratic). Q-4: bracket partners under a parent holding several. Q-5: a
deep-nesting criterion. Q-6: the merged unit assertion that pins `OutsideRoot` as the last
reason row. Split (D-091): not recommended; the four items share `syntax.rs`, the cache, the
limit, the reason row, `syntax.md` and the acceptance file. All six are ruled below and applied
in the sections above by R-165.
### Rulings on Q-1 to Q-6 of the spec check R-157 (director, D-097)
- Q-1: keep. A refused request leaves the session as it was, the cache entry included (`Command::run`'s contract and `docs/control-protocol.md`).
- Q-2: (A). `mode` stays in the result; it is the one place where a result shows the cache, because without it `syntax.incremental` could not be observed through a public interface. The sentence that names this exception is added to T-007's spec when that task is checked before dispatch; this task does not edit T-007.
- Q-3: (a). The size limit stays at 1,048,576 chars and bounds memory, not time. The documents state that no time bound exists and give the measured figures; every figure in a document is measured by the implementer through `kiln-cli`, on a release build, with the command recorded (the spec check's figures come from a debug spike outside the repository and are not copied into documents). A deterministic work cap is a follow-up task, filed by the director; this task does not attempt it.
- Q-4: as recommended. Bracket partners are found by pairing the parent's direct bracket leaves left to right with a stack.
- Q-5: yes, AC-10 for deep nesting; every new walk over the tree is iterative.
- Q-6: as recommended. `TooLarge` is appended as the last row, and the one assertion in `reason.rs`'s unit test that pins `OutsideRoot` as the last row is changed to assert that the row is present; the edit is logged under golden rule 4 as in D-061.
- Generated files and parallel work: `docs/commands.json` is regenerated by the queue at every merge, so two tasks in flight that both change it do not count as overlapping `touches` (METHODOLOGY §5.2 concerns files that builders edit by hand).
- Ruling on the implementer's question of R-167 (director, D-103; Pinned (i) sentence 7). The spec's script for the memory figure inserts a char into a document that is already at the limit, so the calls after the edit are refused and the sentence would be untrue. Option (a): the edit is a `doc.replace` of one char, which keeps the length. The sentence reports the largest peak the implementer measured that way, with the text and the edit named, in MiB computed from the KiB that `/usr/bin/time` prints (705,656 KiB is 689 MiB), and says it is one measurement on a release build. Sentence 6's time figures say the same about their conditions (elapsed time, on a machine that was running other builds); the growth with the square of the length is what the three figures show. The test author is not affected: no test asserts a figure.
- Ruling on the test author's question of R-169 (director, D-104; answered to both builders). The stack rule of Q-4 (a closer pairs with the nearest unpaired opener among the parent's direct bracket leaves) cannot be told from "the first unpaired opener" by any example in the spec. The test author adds one or two worked examples to `bracket_worked_examples` in which one parent has two openers before a closer as direct children. It establishes the parent and its direct bracket leaves from a node dump of the merged T-016 tree (`kiln_syntax::SyntaxTree`, which is on the base and independent of the bracket code), records the dump in its log, and writes the expected answers by the stack rule: the closer pairs with the nearer opener, and the other opener has no partner (`null`). If no input can be found in which the grammar puts such leaves under one parent, it reports that instead, and the rule stays covered by the implementer's unit tests and the reviewer's mutants. The implementer has nothing to change unless the join shows a failure.
### Rulings on review round 1 (R-179, `reviews/T-020-r1.md`; director, D-110). Each builder acknowledges the rulings that concern it before acting.
- r1 (finding 1, defect; finding 4). The memory sentence that D-103 gave reads as the largest possible peak and is untrue: the reviewer's run of the same script on `1;` repeated peaked about three times higher (debug build). The director's wording in D-103 generalised from the two texts the spec had chosen. The "Memory" section becomes: "Memory grows with the length and content of the text. Measured with a release build of `kiln-cli` at the limit (1,048,576 characters; `doc.open`, `syntax.tree`, a `doc.replace` of one char, `syntax.tree`, `syntax.highlight`): <M1> MiB for `a;` repeated and <M2> MiB for `1;` repeated. These are the maximum resident set size of the whole `kiln-cli run` process in two runs. They are not a bound: no bound on memory is promised, and other texts can need more." The implementer measures `1;` on a release build with the same command as before, fills both figures from its own runs in MiB (KiB divided by 1,024), and records commands and outputs. It adds no sentence about the cause of the difference unless it shows it by a run (for example the same script without the `syntax.highlight` call), in which case it may state what that run showed. The first sentence of the "Time" section says "the size limit bounds memory, not time" (the wording of the director's ruling Q-3); with no bound on memory promised, it becomes: "No time bound exists for any `syntax.*` command: the size limit bounds the length of the text a command accepts, not the time a parse takes or the memory it needs." A cap on the number of spans a `syntax.highlight` reply holds is not part of this task; it goes to the limits follow-up (D-101).
- r2 (findings 2, 3 and 5). The bracket rule is completed: only bracket leaves that are in the text can be partners, so a leaf the parser inserted (a `MISSING` node) is never a partner, and a closer pairs only with an opener of its own kind. This is what the implementation does. The test author adds to `bracket_worked_examples` the reviewer's two cases, with expected answers by this rule and each justified by a node dump of the merged tree as for D-104: `( ]` (offset 0 gives `null`) and `fn f() { g(1; }` (offset 10 gives `null`), each checked from the other end where there is one.
- r3 (the reviewer's note on mutants M3 and M14). Through `kiln-cli` the rule that a refusal leaves the cache unchanged is tested only for `too_large`. The test author adds, to the test that covers refusals, one case each for an `out_of_range` and an `inverted_range` refusal of `syntax.highlight` and an `out_of_range` refusal of `syntax.match-bracket`: after an edit and the refused call, the next `syntax.tree` says `incremental`, and after a successful call followed by the refused call it says `unchanged`.
- r4 (findings 6 and 7, nits). `docs/protocol/syntax.md` says that a bracket the parser inserted is never a partner, in one sentence the implementer writes after running the reviewer's case; the description of `syntax.match-bracket` states the rule as it is (partners are direct bracket children of the same node, of the same kind, paired by a stack) or points to `syntax.md` for it; `docs/commands.json` is regenerated.

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
- 2026-11-13 · R-165 · draft · did: applied the director's rulings D-097 (Q-1 to Q-6, generated
  files): PENDING marks removed; Q-2 stated in Interface without editing T-007; Q-3: every
  debug-spike figure is labelled spec evidence only, Pinned (i) 6 keeps a fixed no-time-bound
  sentence and requires the implementer's release-build measurements through `kiln-cli`
  (commands given) for the time and memory figures; no test asserts a time, and the
  per-process timeout is labelled a hang detector; AC-10's spike timings moved out of the
  oracle. No test asserts a message: AC-7 and AC-8 assert `code` and `kind` only; the
  `too_large` text stays as the implementer's wording (no ruling requires a test to read it;
  R-157 had written that AC-8 checks it). Acceptance-test protocol stated (even arm: the test
  author writes `acceptance_t020_cli.rs` and any `guard_t020_cli.rs`; the implementer adds
  neither); each AC names its file and fails on base. Dispatch note: T-019's Q-4 is ruled (A),
  so `reason.rs` is not shared with it; `docs/commands.json` is not an overlap (D-097).
  Questions section reduced to the record plus the rulings. Main moved to `c538ef8` (T-039):
  only the `doc.undo`/`doc.redo` DOC strings, the CLI section of `docs/control-protocol.md`,
  `doc-positions.md` and two new tests (`--help` option sets, no-op undo/redo) changed;
  nothing this spec relies on changed (AC-9's typing-group step agrees with the new
  `doc.undo` description). Re-read top to bottom: no remaining contradiction found between
  Interface, Pinned, ACs and `touches`. · gate attempts: 0 · not run: no cargo, no new probes ·
  missing context: none · next: director (state).
<!-- end planner log -->
### implementer
- 2026-11-13T22:14Z · R-167 · ready · rulings acknowledged before acting: D-044 (`NoTree` →
  `internal`/`other`) accept; D-056 (size limit before dispatch, T-016 nits here) accept;
  D-097 Q-1 (refusal keeps the cache) accept; Q-2 (`mode` is the one cache-dependent field)
  accept; Q-3 (limit bounds memory not time, figures only from my release measurements)
  accept, but Pinned (i) 7 as written cannot be followed, see QUESTION; Q-4 (stack pairing of
  the parent's direct bracket leaves) accept; Q-5 (AC-10, iterative walks) accept; Q-6 (append
  `TooLarge` last, loosen the one assertion to presence) accept; generated `docs/commands.json`
  not an overlap: accept. ACs restated: AC-1 corpus `has_error`/`errors` per set; AC-2 tree
  after random edits equals a fresh session, `mode` per the cache rule; AC-3 spans partition
  any range; AC-4 string∪number spans are proc-macro2's literal tokens, keywords in the list,
  comments start `//`/`/*`; AC-5 syn fn names are `function` spans; AC-6/AC-7 brackets equal
  proc-macro2 groups, worked and stack examples, `len_chars` → null, +1 → `out_of_range`;
  AC-8 char-based limit, `too_large` before `out_of_range`, cache kept, `too_large` published;
  AC-9 read-only incl. typing group; AC-10 deep nesting answered. · did: `kiln-syntax`:
  `highlight.rs` (`HighlightKind`, `HighlightSpan`, `KEYWORDS`, `highlight(tree, range)`:
  one cursor walk pruned to the range, per-byte kind = first rule, merged into char spans),
  `brackets.rs` (`BracketPair`, `match_bracket(tree, offset)`: cursor descent with
  `goto_first_child_for_byte`, then a stack over the parent's direct non-`MISSING` anonymous
  leaves); `tree.rs` only the N2 unit case `common_affixes("é", "©") == (0, 0)`; `lib.rs`
  Status paragraph only; `tests/highlight.rs` (corpus partition with 60 fixed-seed sub-ranges
  per file agreeing with the full spans, keyword/comment/number checks, proc-macro2 groups and
  literals on the `valid` set, 500,000-level nesting, CRLF/multibyte). `kiln-commands`:
  `syntax.rs` (the three commands, `SyntaxState` = `BTreeMap<u64, SyntaxTree>`, checks in the
  order of Pinned (a) before any parse, the cache written only on success and closed
  documents dropped then, `start?`/`end?` per D-084 x1 with the `"default": null` removed as in
  `cursor.rs`, 7 examples, 4 unit tests incl. the `TooLarge` row); `reason.rs`: row
  `TooLarge = "too_large" => "the document is longer than the command accepts"` appended
  after `OutsideRoot`. Golden rule 4 record (D-097 Q-6, as D-061): in
  `outside_root_is_appended_and_not_a_file_is_widened` the first assertion
  `assert_eq!(RejectReason::ALL.last(), Some(&RejectReason::OutsideRoot))` became
  `assert!(RejectReason::ALL.contains(&RejectReason::OutsideRoot))`; its other assertions are
  unchanged; its doc comment now says why. `docs/commands.json` regenerated;
  `docs/protocol/syntax.md` new: Pinned (i) 1–5 copied verbatim, 6 fixed part verbatim plus
  the measured sentence with my figures, 7 left out (QUESTION). Every sentence checked on the
  built binary: AC-8 script through debug `kiln-cli run --root` (open 1,048,576 → tree `ok`
  `full`, match-bracket 0 → null; insert → the four calls `rejected`/`too_large`; undo →
  tree `unchanged`; `commands --schema` lists `too_large`); read-only script (`doc.info`,
  `cursor.get` equal before/after three good and three refused calls; insert "a", three
  commands, insert "b", one `doc.undo` restores the original text); bracket script on
  `fn g() { let _ = ("(", '['); } // )` + `/* { */ fn h() { (`: brackets in the string, char,
  comments and the unpartnered `{`/`(` give null, `len_chars` null, +1 `out_of_range`;
  highlight partition through release `kiln-cli` on all 45 corpus files × 21 ranges: 945
  calls, 0 violations. · gate attempts: 2 (1: red clippy `collapsible_if` in `brackets.rs`;
  2: green: `GATE PASSED in 291.1s (11 steps; 2 step(s) EMPTY (nothing registered): fuzz,
  budgets)`, test step `712 passed, 0 failed, 0 ignored in 87 suites`). · measurements (one
  `cargo build --release -p kiln-cli` in the worktree at `8963a00`, inputs made under
  `target/` and deleted afterwards, as is `target/release`; load average 3.9–4.9 on 4 cores
  from other builders during the runs, so times are elapsed under contention):
  time, `/usr/bin/time -f '%e s %M KB' target/release/kiln-cli run <script> --root <dir>` with
  script `doc.open` + `syntax.tree`, file `#!` repeated: 16,384 chars `1.10 s 14336 KB`,
  32,768 `4.49 s 18508 KB`, 65,536 `17.00 s 26820 KB` (each `syntax.tree` `ok`,
  `has_error: true`, `mode: full`). Memory, the spec's script (`doc.open`, `syntax.tree`,
  `doc.insert {doc:1, offset:0, text:" "}`, `syntax.tree`, `syntax.highlight {doc:1}`) on
  1,048,576 chars: `;` repeated `2.71 s 525104 KB`, `a;` repeated `1.76 s 426856 KB`; in both
  the insert makes `len_chars` 1,048,577, so ids 4 and 5 are `rejected`/`too_large` and the
  peak is the first `syntax.tree` (exit 3). Supplementary, not in any document: the same with
  `doc.replace {doc:1, start:0, end:1, text:" "}` instead of the insert (length stays at the
  limit; tree 4 `incremental`, highlight 1 span): `;` `4.67 s 675488 KB`, `a;`
  `3.93 s 705656 KB`. · not run: the acceptance tests (independent, not seen); AC-2's random
  edit sequence through the CLI (covered only by `SyntaxTree::update`'s contract, the
  examples and my unit tests); the reviewer's 50 MB `(` probe; no mutation check of my own
  tests; T-007's audit (not merged). · missing context: none. · next: director rules the
  QUESTION; test author; join.
  QUESTION: Pinned (i) 7 cannot be written as pinned. Its script inserts one char into a
  document of exactly 1,048,576 chars, so the document has 1,048,577 and the two calls "after
  one edit" are refused with `too_large` (output above); the sentence "the largest peak at the
  limit was about <N> MB (… after one edit)" would be untrue of the binary. I left the
  sentence out of `syntax.md` rather than reword it. Options: (a) a length-keeping edit
  (`doc.replace` of one char; measured above: about 706 MB for `a;`, 675 MB for `;`); (b) a
  1,048,575-char document plus the insert; (c) drop "after one edit" and report the
  first-parse peak (525 MB for `;`). Each is a director's choice; I will measure and copy
  whichever is ruled.
- 2026-11-13T22:16Z · R-172 · in_progress · ruling acknowledged before acting: D-103 (read in
  the primary checkout's copy of this file under "Questions and rulings"; this worktree's copy
  does not have it and I did not add it): option (a), sentence 7 with the largest
  length-keeping-edit peak in MiB, text and edit named, one measurement on a release build;
  sentence 6 states its conditions and is not a bound. Accept. · did: `docs/protocol/syntax.md`:
  new "Memory" section with sentence 7: about 689 MiB, 1,048,576 chars of `a;` repeated, after
  a `doc.replace` of one char. Figure from R-167's run, no rebuild and no re-measurement:
  `/usr/bin/time -f '%e s %M KB' target/release/kiln-cli run <script> --root <dir>`, script
  `doc.open {path:"asemi.rs"}`, `syntax.tree {doc:1}`,
  `doc.replace {doc:1, start:0, end:1, text:" "}`, `syntax.tree {doc:1}`,
  `syntax.highlight {doc:1}`, output `3.93 s 705656 KB`, exit 0, all five `ok`, tree 4
  `incremental`. 705,656 / 1024 = 689.12 MiB (the `;` run: 675,488 / 1024 = 659.66 MiB, smaller).
  Sentence 6: the pinned measured sentence is unchanged. After it I added that these are
  elapsed times of one run each on a 4-core machine running other builds, that they grow
  about fourfold per doubling (4.49 / 1.10 = 4.08, 17.00 / 4.49 = 3.79), and that they are not
  a bound. · checks: `cargo fmt --all -- --check` clean; `cargo xtask hygiene` PASS. ·
  gate attempts: 0 this run (full gate left to the join, as instructed; last full gate green in
  R-167). · not run: `cargo xtask ci`; no new measurement. · missing context: the ruling was not
  in this worktree's copy of the task file. · next: director; join.
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
