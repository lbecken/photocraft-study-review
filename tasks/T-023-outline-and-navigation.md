---
id: T-023
title: "List a document's outline, go to a line and column clamped to the document, go to an outline symbol, and find symbols by name across the project's Rust files"
tier: medium            # ADR-0002 default: in-memory, plus reads through T-019's walk and the unchanged `Vfs::read` (no new disk code, no dependency, no CHARTER §3 contract); confirmed by the director (D-108, note N-3)
state: self_verified
owner_role: planner
scorecard_items: [syntax.outline, nav.goto-line, nav.goto-symbol, nav.workspace-symbol]
depends_on: [T-016, T-018, T-019, T-020, T-032]   # T-016, T-018, T-019 (3b1a95d), T-032 merged; T-020 (`too_large`, `syntax::MAX_CHARS`) must merge first (spec check R-180); T-021 is not a dependency but merges first (D-092 order)
touches:
  - crates/kiln-syntax/src/outline.rs        # implementer: ItemKind, OutlineItem, outline (slot from T-016)
  - crates/kiln-syntax/src/lib.rs            # implementer: the crate doc's status sentence "[`outline`] is still empty" only (as T-020 left it)
  - crates/kiln-syntax/tests/outline.rs      # implementer's own integration tests
  - crates/kiln-commands/src/nav.rs          # implementer: nav.* commands, their JSON types (slot from T-015)
  - crates/kiln-commands/src/cursor.rs       # implementer: `fn cursor_result` → `pub(crate) fn cursor_result` only (R-180 blocker 2); nothing else
  - docs/protocol/nav.md                     # implementer: new; the sentences of Pinned (i)
  - apps/kiln-cli/tests/acceptance_t023_cli.rs  # implementer (odd arm), on task/T-023-tests: every AC through kiln-cli
  # docs/commands.json changes: generated (`cargo xtask schema --write`); docs/scorecard.md: generated
  # scorecard/items/*.toml: NOT a builder path; the director ties the tests to the items at the join (METHODOLOGY §6)
  # NOT touched: crates/kiln-commands/src/syntax.rs and reason.rs (T-020's; `MAX_CHARS` is already `pub(crate)`), crates/kiln-syntax/src/tree.rs, any Cargo.toml or Cargo.lock, docs/control-protocol.md, docs/protocol/cursor.md, docs/protocol/project.md
budget_tokens: 700000   # medium (v1.2, D-091 change 3)
iteration: 2
found_by: planner
origin_task: null
---

# T-023 Outline and navigation

## Goal
After this task a script can:
- list a document's outline: the `fn`, `struct`, `enum`, `trait`, `impl` and `mod` items, each
  with its name, range, name range and nesting depth (`nav.outline`);
- move the selection to a line and column, both clamped to the document (`nav.goto-line`);
- select an outline item's name (`nav.goto-symbol`);
- find items by name across the project's Rust files; an open document's current text is used
  in place of its file (`nav.workspace-symbols`).

No gate path. No dependency change: `kiln-commands` already depends on `kiln-syntax` and
`kiln-workspace`, and `apps/kiln-cli` already has `syn` and `proc-macro2` as dev-dependencies
(T-016). No new error kind; `too_large` is T-020's (D-108 Q-1). No CLI option or exit code
changes, so `docs/control-protocol.md` is not edited (D-091 change 18 does not apply).

**Written against** main `49d9233` (T-019 merged at `3b1a95d`) **and two sibling branches not
yet on main**, which must both merge before dispatch: T-020 (`task/T-020` at `c0c12fc`) and T-021
(`task/T-021` at `46b93d6`). What this spec relies on from them is the list "To re-check at
dispatch" under "Questions and rulings"; whoever dispatches confirms each line on main first.

Acceptance-test protocol (medium tier, odd task number: METHODOLOGY §4, §6 "Experiment arm").
There is no test author. The implementer, in one run:
1. writes the acceptance tests first, through the public interface, in exactly one file,
   `apps/kiln-cli/tests/acceptance_t023_cli.rs`;
2. commits it on `task/T-023-tests`, from the implementer's base, before any implementation
   code is written or committed, so the order is visible in the history;
3. shows each acceptance test failing on the base (the test name, the command it ran and the
   failure by assertion), and records that in its log; on base every `nav.*` request answers
   `unknown_command`, so every test that asserts `ok` fails by assertion;
4. then writes the code on `task/T-023`, which adds no `acceptance_t*` or `guard_t*` file and
   nothing under `scorecard/` (METHODOLOGY §5.6);
5. lists in its log, per scorecard item, the tests it wrote; the director ties them to the items
   at the join (table "Scorecard items and intended test names").
The reviewer's mutation spot-check (≥ 10 seeded bugs) is the only independent check of these
tests. No guard file is required; none is planned.

Reading list: `crates/kiln-syntax/src/tree.rs` (module doc: every walk is iterative; the
crate-private `root()` and `char_at_byte()`, kept for this task); `tasks/T-016-*.md` Pinned (b)
(corpus layout, `MANIFEST.toml`); `crates/kiln-commands/src/cursor.rs` and
`docs/protocol/cursor.md` (selection, `CursorResult`, `break_undo_group` on success);
`docs/protocol/doc-positions.md` (0-based `line`/`col`, lines split on LF, CRLF, lone CR);
`crates/kiln-workspace/src/project.rs` and `docs/protocol/project.md` (the walk);
`crates/kiln-workspace/src/document.rs` (`Document::open`, decoding);
`crates/kiln-commands/src/project.rs` (how a `VfsError` becomes a refusal);
after T-020 merges: `crates/kiln-commands/src/syntax.rs` (`MAX_CHARS`, `check_size`, the cache
this task does not use) and `docs/protocol/syntax.md` (Size limit, Time, Memory); D-044 (NoTree),
D-097 (parse time not bounded), D-101 (no time bound, release-build figures); this file's
"Questions and rulings".

## Interface

### `kiln_syntax::outline` (L1)
```rust
// crates/kiln-syntax/src/outline.rs — kiln-syntax has no serde dependency and gains none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind { Fn, Struct, Enum, Trait, Impl, Mod }
impl ItemKind { pub fn as_str(self) -> &'static str; }          // "fn", "struct", "enum", "trait", "impl", "mod"
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineItem { pub kind: ItemKind, pub name: String, pub start: usize, pub end: usize,
                         pub name_start: usize, pub name_end: usize, pub depth: usize }
pub fn outline(tree: &SyntaxTree) -> Vec<OutlineItem>;          // preorder = order of `start`
```
It uses the crate-private `SyntaxTree::root()` and `char_at_byte()` (`tree.rs`, not edited;
their `#[allow(dead_code)]` stays). The walk is iterative (a `TreeCursor` or an explicit stack),
as every walk over the tree is (`tree.rs` module doc; T-020 ruling Q-5).

Rules (exact):
- Item nodes: `function_item` and `function_signature_item` → `fn`; `struct_item` →
  `struct`; `enum_item` → `enum`; `trait_item` → `trait`; `impl_item` → `impl`; `mod_item` →
  `mod` (inline or `mod x;`).
- The root node is where the walk starts, whatever its kind (with the pinned grammar a broken
  file's root can be `ERROR`; Pinned (g) example 2). An item node is listed iff every node
  strictly between it and the root is `source_file`, `declaration_list`, `mod_item`,
  `impl_item`, `trait_item` **or `ERROR`** (D-108 Q-2). So nothing inside fn bodies, blocks,
  `extern` blocks (`foreign_mod_item`), `const`/`static` initialisers or macro invocations
  (`token_tree`) is listed. `depth` = the number of listed items that enclose it.
- `name`: the source text of the `name` field. For `impl` it is the source text of the `type`
  field, or `<trait text> for <type text>` when there is a `trait` field, with every run of
  whitespace (`char::is_whitespace`) collapsed to one space. The `!` of a negative impl is not
  inside the `trait` field, so `impl !Send for X` has the name `Send for X` (probe R-180).
  `name_start`/`name_end` cover the `name` field, or for `impl` the `type` field. `start`/`end`
  cover the whole item node; attributes and doc comments are sibling nodes, so they are not
  inside it; a visibility (`pub`) is. All offsets are chars into the parsed text. With the pinned
  grammar every item node has its `name` (or `type`) field; should one lack it (error
  recovery), the item is not listed.
- Under error recovery, which items are present depends on tree-sitter; no guarantee is made
  beyond the rules above (Pinned (i) N3).

### Commands
Common: `doc` is an open document's id (an integer, as in `doc.*`). Optional params, when
present, must not be `null` (`invalid_params`), as for `cursor.set` (ruling x1, D-084). Unknown
params are `invalid_params` (`deny_unknown_fields`). Each command has at least one `Example`
(run by the registry audit on `MemoryFs`).

| Command | Mutating | Params | Result |
|---|---|---|---|
| `nav.outline` | no | `{doc}` | `{items:[Item]}` |
| `nav.goto-line` | yes | `{doc, line, col?}` (integers ≥ 0; `col` default 0) | the `cursor.*` result (`CursorResult`, T-018) |
| `nav.goto-symbol` | yes | `{doc, index}` (integer ≥ 0) | `{item: Item, cursor: CursorResult}` |
| `nav.workspace-symbols` | no | `{query, limit?}` (`limit` 1..=10000, default 200) | `{items:[Symbol], truncated, skipped:[{path, reason}], walk_truncated, too_deep}` (D-108 Q-3, Q-4) |

`Item` = `{kind, name, start, end, name_start, name_end, depth}` (`kind` as `ItemKind::as_str`).
`Symbol` = `{path, kind, name, start, end, name_start, name_end, line, col}`. The JSON types live
in `nav.rs` (as `project.rs` does for `project.tree`). `CursorResult` and `cursor_result` are
`cursor.rs`'s; the only edit there is making `cursor_result` `pub(crate)`.

- `nav.outline`: `outline` of a fresh `SyntaxTree::parse` of the document's current text.
  It neither reads nor changes T-020's `SyntaxState` (the tree cache of `syntax.*`), so it never
  changes the `mode` a later `syntax.tree` reports (AC-2). A document longer than 1,048,576
  chars (`crate::syntax::MAX_CHARS`, T-020) is refused with `rejected`/`too_large` before
  anything is parsed (D-108 Q-1). The reply lists every item; there is no `limit`.
- `nav.goto-line`: with L = `len_lines` (`doc.info`), line′ = `min(line, L − 1)`; col′ =
  `min(col, length in chars of line′'s content, without its break)`. The selection becomes a
  caret (anchor = head) at the char offset of (line′, col′), so never inside a CRLF. `line` and
  `col` beyond the document are clamped, never refused. No parse and no size limit.
- `nav.goto-symbol`: `index` into the items `nav.outline` gives for the current text. The
  selection becomes anchor = `name_start`, head = `name_end`. An `index` ≥ the number of items →
  `rejected`/`out_of_range`. Size limit as `nav.outline` (D-108 Q-1).
- Both `nav.goto-*` commands, when they succeed, end the document's typing group, as
  `cursor.set` does (`docs/protocol/cursor.md`); a refused request changes nothing, the
  selection and the typing group included. They do not edit the text, so the undo history and
  the dirty flag are unchanged.
- `nav.workspace-symbols` (D-108 Q-3, Q-4):
  - Files: `kiln_workspace::project::tree(vfs, None, 1_000_000)` — the walk of `project.tree {}`
    with that command's largest `limit` — and of its entries those of kind `file` whose name
    ends in `.rs` (case-sensitive), in the walk's order. `symlink` and `other` entries are not
    searched (never followed or opened, T-019), nor anything the walk ignores (`target`, `.git`,
    `.gitignore` patterns) or does not descend (depth bound). `walk_truncated` is the walk's
    `truncated`; `too_deep` is its `too_deep`.
  - Text of a file (stated here; T-022's draft rule is not used): if an open document has the
    file's path, that document's current text (saved or not), and nothing is read; otherwise
    `Vfs::read` of the file, decoded as `doc.open` decodes it (`Document::open`: UTF-8, UTF-8
    with BOM, UTF-16 LE/BE by BOM; the BOM is not part of the text).
  - A file is skipped, listed in `skipped` as `{path, reason}` in the walk's order, and the
    command goes on, when: its read fails (`reason` = the kind of the `VfsError` as
    `project.tree` reports it: `not_found`, `not_a_file`, `outside_root`, `io_error`; for
    example a file removed or replaced after it was listed); it does not decode
    (`decode_error`); or its text is longer than 1,048,576 chars (`too_large`, checked before
    parsing). Each reason is the name of an existing kind. A file with syntax errors is not
    skipped: its outline is what the rules give.
  - Matching: an item matches iff `name.to_lowercase()` contains `query.to_lowercase()`
    (Rust's `str::to_lowercase`). An empty `query` → `rejected`/`invalid_pattern` (an existing
    kind, whose description already says "the search pattern is empty").
  - Rank: exact matches (equal lowercase names) first, then prefix matches, then the others;
    within a rank by the file's position in the walk's order, then `start`. The first `limit`
    are returned; `truncated` is true iff more items matched.
  - `start`, `end`, `name_start`, `name_end` are chars into the file's text as above; `line`
    and `col` are the 0-based line and column of `name_start` by `doc-positions.md`'s rule.
  - Errors of the walk itself (listing a directory, reading a `.gitignore`) fail the whole
    command with `project.tree`'s refusal for that error (`not_found`, `not_a_file`,
    `outside_root`, `io_error`), as T-019 pins them; nothing is skipped for them.
  - Files are processed one at a time; the memory a file's tree takes is released before the
    next file is parsed, and at most `limit` items per rank need be kept.

Parsing: `SyntaxTree::parse` returns `Result<SyntaxTree, ParseError>` (T-016); the only error,
`ParseError::NoTree`, cannot occur with the pinned versions. If it does, the command fails with
`internal`/`other` and `nav.workspace-symbols` does not add the file to `skipped` (D-044). No
`unwrap`/`expect` on it.

`docs/protocol/nav.md` documents the four commands, the rules above, the examples of Pinned (g),
and, word for word, the sentences of Pinned (i). `docs/protocol/cursor.md` is not changed: its
list at :8–9 names commands that edit the text, and `nav.goto-*` do not.

## Pinned
- (a) Errors and precedence. `nav.outline`: schema (`invalid_params`) → `no_such_document` →
  `too_large` (D-108 Q-1). `nav.goto-line`: schema → `no_such_document`; nothing else.
  `nav.goto-symbol`: schema → `no_such_document` → `too_large` (D-108 Q-1) → `out_of_range`.
  `nav.workspace-symbols`: schema (including `limit` 0 or 10001) → `invalid_pattern` (empty
  `query`) → a walk error (`project.tree`'s kinds and order, T-019). All kinds exist after
  T-020; no new kind, no `reason.rs` change. Refusal declarations per command are T-038's,
  which runs after this task (D-092). A refused request changes nothing and is not logged.
- (b) Files: read only, through T-019's walk (`list_dir`, `.gitignore` reads) and `Vfs::read` of
  the searched files; nothing is written, created or opened for writing. Test file: the one in
  `touches`, a name distinct across crates.
- (c) Order: outline items by `start` (preorder); workspace symbols by rank, walk order, `start`;
  `skipped` in walk order. Per file: text (open document or read + decode) → size check →
  parse → outline → match.
- (d) Offsets are chars; lines split on LF, CRLF and lone CR (`doc-positions.md`); paths are
  root-relative RelPaths as `project.tree` prints them. Unix for the on-disk tests (symlinks,
  non-UTF-8 files), as T-019.
- (e) Threat model and resource bounds. No new boundary: every directory and file is reached
  through T-019's walk and the unchanged `Vfs::read` (T-008 confinement), so a hostile project
  cannot make the command read outside the root or follow a link (T-019 Pinned (e)). Bounds, each
  a behaviour, never a time (D-097, D-101):
  - files searched: the `.rs` `file` entries of a walk capped at 1,000,000 entries; beyond it
    `walk_truncated` is true (D-108 Q-3);
  - per file: a text over 1,048,576 chars is skipped `too_large` before it is parsed; but it has
    been read and decoded whole by then, because `Vfs` has no size query (N-5), so memory grows
    with the largest file in the project;
  - no bound on the total size of the files, on the number of items parsed, or on time: parse
    time is not bounded by the size limit (D-097: grows with the square of the length for some
    inputs; `docs/protocol/syntax.md`, Time), and this command
    parses every searched file, so one hostile file can hold it without bound. No test asserts a
    time; each timeout in a test is a hang detector and is labelled so. A deterministic work
    cap on parsing is the follow-up the director files (D-097 Q-3); this task does not attempt
    it;
  - reply: at most `limit` symbols plus one `skipped` row per skipped file (no cap on `skipped`).
  - concurrency: a change by another process while the command runs is accepted (T-008 D-3): a
    file that vanishes or changes kind between the listing and the read is skipped with its
    kind (D-108 Q-4); a directory that does so fails the walk (T-019).
- (f) Snapshots: none. No test of this task pins a message text, a whole document, the command
  list, `error_kinds` or a `when` text (golden rule 9; D-091 change 6): the tests assert
  `code`/`kind`, results and selections, never `error.message`. Messages in this spec are the
  implementation's wording, checked by the reviewer on the diff and the binary.
- (g) Worked examples (outline leaves: probe R-180, a throwaway implementation of the rules above
  over the merged `SyntaxTree`, tree-sitter 0.27.1, debug build; goto-line and ranking: a naive
  Python model, R-180):
  1. Text `impl<T> Display for Foo<T> { fn fmt(&self) {} }\nmod m;\ntrait T { fn x(&self); }`
     → items, in order (kind, name, `start..end`, `name_start..name_end`, depth):
     `impl` `Display for Foo<T>` `0..47` `20..26` 0; `fn` `fmt` `29..45` `32..35` 1; `mod` `m`
     `48..54` `52..53` 0; `trait` `T` `55..79` `61..62` 0; `fn` `x` `65..77` `68..69` 1.
     `nav.goto-symbol {index:1}` → anchor 32, head 35. `{index:5}` → `out_of_range`.
  2. Text `fn a() {}\nfn b( {\nfn c() {}\nstruct D;\n` (the root is `ERROR@0..38`) → `fn` `a`
     `0..9` `3..4` 0; `fn` `c` `18..27` `21..22` 0; `struct` `D` `28..37` `35..36` 0 (probe R-180:
     the same whether or not `ERROR` counts as a container).
  3. Text `/// doc\n#[derive(Debug)]\npub struct S;\n#[cfg(test)]\nmod tests { fn t() {} }` →
     `struct` `S` `25..38` `36..37` 0; `mod` `tests` `52..75` `56..61` 0; `fn` `t` `64..73`
     `67..68` 1 (attributes and doc comments outside, `pub` inside).
  4. Text `extern "C" { fn c(); }\nm! { fn inner() {} }\nfn outer() { fn nested() {} struct N; }\nconst _: () = { struct Q; };`
     → only `fn` `outer` `44..83` `47..52` 0.
  5. Text `impl  <T>  Foo < T ,\n U > {}\nimpl !Send for X {}` → `impl` `Foo < T , U >` `0..28`
     `11..25` 0; `impl` `Send for X` `29..48` `44..45` 0.
  6. Text `ab\r\ncdef\n` (3 lines): `nav.goto-line {line:1, col:9}` → head 8, line 1, col 4;
     `{line:8}` → head 9, line 2, col 0; `{line:0, col:1}` → head 1, line 0, col 1.
  7. Project files `a.rs` = `fn foo() {}\nstruct FooBar;\n`, `b/c.rs` =
     `mod foo { fn barfoo() {} }\n`, `z.txt` = `fn foo() {}` (not `.rs`):
     `nav.workspace-symbols {query:"FOO"}` → items (path, kind, name, `name_start`, line, col):
     `a.rs` `fn` `foo` 3 0 3; `b/c.rs` `mod` `foo` 4 0 4; `a.rs` `struct` `FooBar` 19 1 7;
     `b/c.rs` `fn` `barfoo` 13 0 13; `truncated: false`, `skipped: []`, `walk_truncated: false`,
     `too_deep: 0`. With `limit: 2` → the first two, `truncated: true`. With `query: ""` →
     `invalid_pattern`.
- (h) Evaluator calls: `kiln-cli run <script> --root <dir>` with `doc.open`, then
  `nav.outline` (`syntax.outline`); `nav.goto-line` then `cursor.get` (`nav.goto-line`);
  `nav.outline` then `nav.goto-symbol` then `cursor.get` (`nav.goto-symbol`);
  `nav.workspace-symbols` over a project root, also after a `doc.insert` into an open file
  (`nav.workspace-symbol`). `kiln-cli exec nav.workspace-symbols '{"query":"x"}' --root <dir>`
  also works without a script.
- (i) Sentences for `docs/protocol/nav.md` that state a guarantee or a limit, to be copied (any
  line wrapping; may be joined into paragraphs, not widened). A `<…>` slot is a figure the
  implementer measures itself through `kiln-cli` on a **release** build of its worktree
  (`cargo build --release -p kiln-cli`, its own `target/`), each run under `timeout`, with the
  exact command and its output recorded in its log; a run that reaches its timeout is written
  "more than <timeout> s". No other sentence stating a guarantee or a limit is added; an
  implementer who needs one records its evidence in its log (D-091 change 5).
  - N1. "`nav.outline` lists the `fn`, `struct`, `enum`, `trait`, `impl` and `mod` items of the
    document that are not inside a function body, a block, an `extern` block, a `const` or
    `static` value or a macro invocation, in the order of their start." Evidence: AC-1, AC-2.
  - N2. "On the files of set `valid` of the pinned corpus (`crates/kiln-syntax/tests/corpus/`),
    the `fn`, `struct`, `enum`, `trait` and `mod` items, with their names, name offsets and
    depths, and the `impl` items, by the offset of their type and their depth, are exactly those
    the `syn` parser finds." Evidence: AC-1.
  - N3. "In a document with syntax errors the outline lists the items the parser's error
    recovery leaves in place, by the same rules; which items those are is not specified."
    Evidence: AC-2 example 2 (one case, not a guarantee); wording as ruled (D-108 Q-2).
  - N4 (D-108 Q-1). "`nav.outline` and `nav.goto-symbol` refuse a document longer than
    1,048,576 chars with `rejected` / `too_large`, as the `syntax.*` commands do; a document of
    exactly 1,048,576 chars is accepted. `nav.goto-line` parses nothing and has no such limit."
    Evidence: AC-7.
  - N5. "`nav.goto-line` never refuses a line or column beyond the document: the line is
    clamped to the last line and the column to the length of that line without its line break,
    so the caret never lands inside a CRLF." Evidence: AC-3.
  - N6. "`nav.goto-line` and `nav.goto-symbol` end the document's typing group when they
    succeed, as `cursor.set` does; they do not change the text, the undo history or the dirty
    flag. A refused request changes nothing." Evidence: AC-3, AC-4.
  - N7. "The `nav.*` commands parse the current text afresh on every call and do not use or
    change the tree that the `syntax.*` commands keep, so they never change the `mode` that
    `syntax.tree` reports." Evidence: AC-2.
  - N8. "`nav.workspace-symbols` searches the entries of kind `file` whose name ends in `.rs`
    that `project.tree {}` would list; it does not follow symbolic links, and files that
    `project.tree` ignores or does not reach are not searched. For a file that is open as a
    document, the document's current text is searched, saved or not, and the file is not read."
    Evidence: AC-5.
  - N9 (D-108 Q-4). "A file that cannot be read (`not_found`, `not_a_file`, `outside_root`,
    `io_error`, for example because it was removed after it was listed), that is not valid
    text (`decode_error`), or whose text is longer than 1,048,576 chars (`too_large`) is listed
    in `skipped` with that reason, and the search goes on. An error while listing a directory
    or reading a `.gitignore` fails the whole command, as for `project.tree`." Evidence: AC-5
    (`decode_error`), AC-7 (`too_large`), the implementer's unit test with a `Vfs` whose read
    fails (the other four; not observable through `kiln-cli` without a concurrent process).
  - N10 (D-108 Q-3). "The search covers at most the first 1,000,000 entries of the walk, the
    largest `limit` of `project.tree`; `walk_truncated` is true when the walk stopped there, and
    `too_deep` counts the directories not descended because of the depth bound." Evidence:
    AC-8 (`too_deep`), by construction (`walk_truncated`, not testable at 1,000,000 entries in
    the gate; the reviewer checks the code).
  - N11. "A file is read and decoded whole before its length is checked, so a large file costs
    memory even when it is skipped as `too_large`; files are searched one at a time. Measured
    with a release build, a project holding one file of 1,048,576 chars (`a;` repeated) took a
    peak of about <M> MiB." Evidence: by construction (N-5); `<M>`: the `Maximum resident set
    size` of `/usr/bin/time -v timeout 1800 target/release/kiln-cli run s.jsonl --root <dir>`
    with the script `nav.workspace-symbols {query:"x"}`, in MiB to two significant figures.
  - N12. "No time bound exists for any `nav.*` command that parses: parsing time depends on the
    text, not only on its length, and `nav.workspace-symbols` parses every file it searches.
    Measured with a release build, `nav.workspace-symbols` over a project holding one file of
    `#!` repeated (65,536 chars) took <T> s." Evidence: D-097, `docs/protocol/syntax.md` (Time);
    `<T>`: `/usr/bin/time -f '%e s' timeout 1800 target/release/kiln-cli run s.jsonl --root <dir>`
    with the script `nav.workspace-symbols {query:"x"}`, minus the same over an empty project,
    to two significant figures. If `<T>` is under 1 s the implementer writes the figure anyway
    and logs a `QUESTION:` (the sentence then needs the director's wording).
  - N13. "The search is not a snapshot: if another process changes the project while the command
    runs, the result can mix the states before and after." Evidence: by construction (T-019 S11;
    each file is read when it is reached).
  - N14. "Matches are ranked: exact matches of the name (ignoring case) first, then names that
    begin with the query, then names that contain it; within a rank by the file's place in the
    walk and then by the item's start. `truncated` is true when more than `limit` items
    matched." Evidence: AC-5, AC-8.

## Acceptance criteria
File: `apps/kiln-cli/tests/acceptance_t023_cli.rs` drives the built binary
(`env!("CARGO_BIN_EXE_kiln-cli")`) through `kiln-cli run` or `kiln-cli control`. Send many
requests per process (one `control` session or one `run` script per batch), not one process per
random case. Every spawned process has a timeout, a hang detector labelled so, never a time
assertion. No test reads `error.message`, a whole document, the command list or `error_kinds`
(Pinned (f)). Corpus files are read by path from
`concat!(env!("CARGO_MANIFEST_DIR"), "/../../crates/kiln-syntax/tests/corpus/")`, stored as
`<name>.rs.txt` and selected by their `set` in `MANIFEST.toml` (T-016 Pinned (b)); `kiln-cli`
has no `toml` dev-dependency (adding one is a dependency change), so the test reads the
`path = "…"` and `set = "…"` lines of `[[file]]` entries by hand. "Base" is the implementer's
base commit, where every `nav.*` request answers `unknown_command`; every AC therefore **fails on
base by assertion**.
- AC-1 (`syntax.outline`; differential with syn): on every `valid` corpus file, opened with
  `doc.open` (under `--root` of a temporary copy, or `--memory-seed`), the `nav.outline` items of
  kinds other than `impl` equal the items found by walking `syn::parse_file`'s `syn::File`.
  The walk takes `Item::{Fn, Struct, Enum, Trait, Mod}`, recurses into inline `mod` content
  (depth + 1) and into `Item::Impl`/`Item::Trait` for `ImplItem::Fn`/`TraitItem::Fn` (depth + 1),
  and never enters fn bodies, `extern` blocks or macros. They are compared as (kind, ident text,
  ident start offset, depth); `impl` items equal syn's `Item::Impl`s by (start offset of
  `self_ty`, depth). Offsets: proc-macro2's `span-locations` give a 1-based line and a column in
  chars on lines split at `\n` only (not Kiln's line rule), so the char offset is the char index
  where that `\n`-line starts plus the column. Probe R-180 (throwaway implementation, debug): 33
  files, 555 items, 0 differences; no `valid` file starts with a BOM or `#!` (which
  `syn::parse_file` strips). Oracle: independent parser (syn).
- AC-2 (`syntax.outline`; exact): examples 1 to 5 of Pinned (g) hold exactly (every field of
  every item, and the item count). On example 1: `syntax.tree`, then `nav.outline`, then
  `syntax.tree` again → the second `syntax.tree` reports `mode: "unchanged"` (N7). On every
  corpus file of every set, the source slice `name_start..name_end` (by chars) equals `name` for
  every item that is not `impl` (probe R-180: 0 mismatches on `valid`). Oracle: hand-written
  expectation (probe-checked) + consistency.
- AC-3 (`nav.goto-line`): for ≥ 2,000 fixed-seed random texts over `a b é 😀 \r \n` and random
  `line`/`col` (including 0, the last line, far out of range, `u32::MAX`, and `col` omitted),
  `nav.goto-line` returns, and `cursor.get` then reports, the selection of a naive clamp model
  (anchor = head; line split on LF, CRLF, lone CR). Example 6 of Pinned (g) holds. A negative,
  fractional, string or `null` `line` or `col`, and an unknown param, → `invalid_params`; an
  unknown `doc` → `no_such_document`; both leave `cursor.get` unchanged. Typing group: after
  `doc.insert` of one char, `nav.goto-line`, and `doc.insert` of one char at the new caret, one
  `doc.undo` removes only the second char; `doc.info`'s `dirty` and `can_undo` are unchanged by
  `nav.goto-line` alone. Oracle: naive model + exact value.
- AC-4 (`nav.goto-symbol`): for every corpus file of every set and every index `i` below the
  item count of `nav.outline`, `nav.goto-symbol {index: i}` returns `item` equal to item `i`, and
  `cursor` and a following `cursor.get` equal `{anchor: name_start, head: name_end}` of that item.
  `index` = the item count → `out_of_range`, selection unchanged. The typing-group check of AC-3
  holds with `nav.goto-symbol` in place of `nav.goto-line`. Oracle: consistency with
  `nav.outline` + exact value.
- AC-5 (`nav.workspace-symbol`; naive model): a project directory (`--root`) holding every
  `valid` corpus file copied flat as `src/<stem>.rs` (the stem is the file name without
  `.rs.txt`), plus a `.gitignore`d copy (`.gitignore` `ignored.rs`), a copy under `target/`, a
  symbolic link `link.rs` to one of the copies (listed `symlink`, not searched), a copy named
  `notes.txt` and a non-UTF-8 file `bad.rs` (bytes `fn \xff() {}`). For ≥ 30 queries (exact
  names, prefixes, inner substrings, case-changed, `impl` names such as `for`, and two that
  match nothing), the result equals a naive model built from public commands: `project.tree {}`
  for the file list, then per `file` entry ending in `.rs`, in tree order, `doc.open` in a fresh
  session (a `decode_error` refusal → `{path, reason: "decode_error"}` in `skipped`) and
  `nav.outline`, then the filter, rank, `limit` and `truncated` of the Interface, `line`/`col`
  from `doc.position`. Example 7 of Pinned (g) holds exactly. Open documents: after `doc.open`
  of `src/<one>.rs` and `doc.insert` of `fn zzz_new() {}` at its end, `zzz_new` is found in that
  file with offsets in the edited text; after `doc.close {discard:true}` it is not. Oracle: naive
  model built from public commands + exact value.
- AC-6 (`syntax.outline`; deep nesting, T-020 ruling Q-5): a document of `mod m {` repeated
  20,000 times, then `fn f() {}`, then `}` repeated 20,000 times (160,009 chars) → `nav.outline`
  gives 20,001 items, the last a `fn` `f` at depth 20,000, and the process exits normally (no
  stack overflow). Probe R-180 (debug, throwaway, recursive walk on a 1 GiB stack) gave
  20,001 items, last depth 20,000; the acceptance test runs on the default stack, which is
  what catches a recursive walk. No time is asserted. Oracle: exact value.
- AC-7 (size limit; D-108 Q-1): a document of exactly 1,048,576 chars (`//` then `é` repeated,
  one comment, as T-020's unit test builds it) → `nav.outline` `ok` with `items: []`; after a
  one-char `doc.insert` → `nav.outline` and `nav.goto-symbol {index:0}` → `too_large`, and
  `nav.goto-line {line:0, col:5}` → `ok`. In a project, a `.rs` file of 1,048,577 chars →
  listed in `skipped` with reason `too_large` by `nav.workspace-symbols`, while a file of
  1,048,576 chars is not in `skipped` (it is parsed; the `//é…` text has no items). Oracle: exact value.
- AC-8 (`nav.workspace-symbol`; limits and refusals): `query: ""` → `invalid_pattern`; `limit:
  0`, `limit: 10001`, `limit: null`, a missing `query` and an unknown param → `invalid_params`;
  a project with 250 files `f<i>.rs` each holding `fn hit() {}` gives, without `limit`, 200 items
  and `truncated: true`, and with `limit: 10000` all 250 in tree order (byte order of the names)
  and `truncated: false`; a chain of 130 nested directories `d/…/d` with `x.rs` holding
  `fn deep() {}` at the bottom → `deep` not found, `too_deep: 1` (D-108 Q-3). Oracle: exact
  value.
- No criterion asserts a time (D-097, D-101).

## Scorecard items and intended test names
For the director, who ties tests to items at the join (METHODOLOGY §6); the builders do not
edit `scorecard/`. The implementer uses these names, or lists the names it used in its log.
| Item | Intended tests (AC) |
|---|---|
| syntax.outline | `kiln_cli::acceptance_t023_cli::outline_agrees_with_syn` (AC-1), `kiln_cli::acceptance_t023_cli::outline_worked_examples` (AC-2), `kiln_cli::acceptance_t023_cli::outline_survives_deep_nesting` (AC-6), `kiln_cli::acceptance_t023_cli::outline_size_limit` (AC-7) |
| nav.goto-line | `kiln_cli::acceptance_t023_cli::goto_line_clamps` (AC-3), `kiln_cli::acceptance_t023_cli::goto_line_ends_typing_group` (AC-3) |
| nav.goto-symbol | `kiln_cli::acceptance_t023_cli::goto_symbol_selects_name` (AC-4) |
| nav.workspace-symbol | `kiln_cli::acceptance_t023_cli::workspace_symbols_match_naive_model` (AC-5), `kiln_cli::acceptance_t023_cli::workspace_symbols_limits_and_refusals` (AC-8) |
The `too_large` half of AC-7 for `nav.workspace-symbols` may sit in `outline_size_limit`; it is
not tied to `nav.workspace-symbol` separately. (The item file `nav.workspace-symbol.toml` says
`crate = "kiln-workspace"`; the command lives in `kiln-commands`, because `kiln-workspace` has no
`kiln-syntax` dependency and adding one is a dependency change. That field is the director's.)

## Out of scope and implementer notes
- Other item kinds (const, static, type, union, macro_rules, enum variants, fields); fuzzy
  ranking (the palette task, T-031); a cached symbol index or reuse of T-020's `SyntaxState`
  (each call parses); a `path` (subtree) param for `nav.workspace-symbols`; a size query on
  `Vfs` (a confinement-boundary change; N-5); a deterministic work cap on parsing (the director's
  follow-up, D-097 Q-3); declaring refusal kinds per command (T-038).
- No dependency change (no `Cargo.toml` or `Cargo.lock` edit: the queue refuses it as
  `dependencies`). `kiln-syntax` gains no `serde`; `kiln-workspace` gains no `kiln-syntax`.
- No change to `syntax.rs`, `reason.rs`, `tree.rs`, `project.rs` (either crate), `cursor.md`,
  `project.md`, `docs/control-protocol.md`. Use `crate::syntax::MAX_CHARS` (already `pub(crate)`
  after T-020); the size check itself is written in `nav.rs` (T-020's `check_size` is private,
  and its message names `syntax` commands).
- Your own tests: `crates/kiln-syntax/tests/outline.rs`, unit tests in `outline.rs` and
  `nav.rs` (among them a `Vfs` whose `read` fails after `list_dir` listed the file, for N9).
  Acceptance tests: the protocol in Goal.
- For planning only (probe R-180, debug, throwaway; not for any document): the corpus
  differential and the 20,000-deep nesting case are cheap; neither needs a large timeout.

## Questions and rulings
- Director (2026-11-13, D-044): when the parse contract returns `ParseError::NoTree`, the command fails with code `internal`, kind `other`, as proposed.

### Spec check R-180 (planner, 2026-11-13, against main `49d9233`; main `d162413` differs only in `handoffs/`)
Verdict at R-180: ready to dispatch once Q-1 to Q-4 were ruled and both T-020 and T-021 have
merged. Q-1 to Q-4 are ruled (D-108, below) and applied by R-182; the spec above reads as
decided. It is dispatched once both siblings have merged and the list "To re-check at
dispatch" holds on main. The PROPOSED marker of R-079 is removed: D-044 ruled it.

**How checked.** Read against main and, for the siblings, against their branch heads with
`git show task/T-020:<path>` / `git show task/T-021:<path>` from the primary checkout (read
only). The T-020 worktree was not used as evidence: its `syntax.rs` held an uncommitted edit
(`len_chars >= MAX_CHARS` where the branch has `>`), consistent with a review's seeded
mutant in progress. Probes (VERIFIED, debug build, all in a private clone of main `d162413`
under the scratch directory, removed afterwards with its `target/` of 335 MB; no cargo in the
primary checkout or a worktree; no `kiln-cli` built): a throwaway `kiln_syntax::outline`
written from the Rules above (with a switch for Q-2) and three throwaway tests in
`crates/kiln-syntax/tests/`, run with `timeout 1200 cargo test -p kiln-syntax --test probe_t023
-- --nocapture` (and `probe2`–`probe4`):
- P1 the (g) examples 1–5 and 2 (root `ERROR`): leaves and every `start`/`end` as now pinned;
  example 2's root is `ERROR@0..38`.
- P2 AC-1's differential on the 33 `valid` files: 555 items (364 fn, 89 impl, 57 struct, 23
  enum, 12 trait, 10 mod), 0 differences from the syn walk; 0 name-slice mismatches; no file
  starts with a BOM or `#!`.
- P3 eight broken inputs, with and without `ERROR` as a container: identical outlines in all
  eight (in these inputs `ERROR` contained items only as the root); see Q-2.
- P4 deep nesting: 1,000 / 5,000 / 20,000 nested `mod`s parse in 0.008 / 0.068 / 0.118 s with
  no error and give 1,001 / 5,001 / 20,001 items (last depth = N).
- P5 the goto-line and ranking examples (6, 7) against a naive Python model (`python3 -I`).

**Blockers and corrections (fixed above unless a question is named).**
1. The reading list and the workspace rule pointed at T-022's "Text of a file" and its skip
   reasons `decode_error`, `binary`. T-022 is a stage-2 draft that is not dispatched (ADR-0005),
   and `binary` is not a kind. The rule is now stated in this file; `binary` dropped (Q-4).
2. `nav.goto-*` return `CursorResult (T-018)`, but `CursorResult`'s fields and `cursor_result`
   are private to `cursor.rs` (`crates/kiln-commands/src/cursor.rs`: `fn cursor_result`, private
   fields), so `nav.rs` cannot build one. `cursor.rs` added to `touches` for one visibility
   change (`pub(crate) fn cursor_result`); T-038 also touches `cursor.rs` but runs after this
   task (D-092).
3. `ItemKind` was to derive serde ("serde/as_str"): `kiln-syntax` has no `serde` dependency
   (`crates/kiln-syntax/Cargo.toml`), and adding one is a dependency change (high). Now
   `as_str` only; the JSON types are in `nav.rs`.
4. No size limit: `nav.outline` would parse a document of any length. T-020 (branch, `syntax.rs`
   `MAX_CHARS = 1_048_576`, `RejectReason::TooLarge`; its spec, Out of scope) leaves adopting
   `too_large` to this spec; its documents record 689 MiB at the limit and quadratic parse time
   (`docs/protocol/syntax.md` on `task/T-020`). → Q-1; `depends_on` gains T-020.
5. The ancestor rule ("every ancestor … is `source_file`, …") and "items in ERROR subtrees are
   listed if their node is present" contradict each other, and with the pinned grammar a broken
   file's root itself can be `ERROR` (P1), which the rule left undefined. The root is now
   excluded from the check; `ERROR` below it → Q-2.
6. `nav.workspace-symbols` did not say which walk `limit` it uses (`project::tree` requires one;
   `project.tree {}` defaults to 50,000), nor what the reply says when the walk is truncated or
   stops at the depth bound of 128 segments; nor what a read failure of a listed file does
   (only decode failures were handled; a file removed between listing and reading is
   `not_found`). → Q-3, Q-4.
7. AC-2 required `start` and `end` "for every item", but (g) gave name ranges only. Now pinned
   from P1 (`impl` `0..47`, `fmt` `29..45`, `mod` `48..54`, `trait` `55..79`, `x` `65..77`).
8. Test names `kiln_cli::acceptance_t023::…` and no acceptance file in `touches`; the stem must
   be unique across crates. Now `apps/kiln-cli/tests/acceptance_t023_cli.rs`, names
   `kiln_cli::acceptance_t023_cli::…`, one test per behaviour, and a table per item.
9. AC-1/AC-5 assumed the corpus could be selected "by `set` in `MANIFEST.toml`" from `kiln-cli`
   tests: `kiln-cli` has no `toml` dev-dependency (`apps/kiln-cli/Cargo.toml`); the AC now says
   to read the two keys by hand. AC-1 also needed the offset conversion stated (proc-macro2
   splits lines at `\n` only; `crlf_lines.rs.txt` is in the `valid` set).
10. The typing group and the selection were specified for `nav.goto-line` only; `nav.goto-symbol`
   now follows `cursor.set` too (cursor.md: "`cursor.set` and `cursor.move` end the document's
   typing group when they succeed … A refused request changes nothing"), and AC-3/AC-4 test it.
11. No criterion for deep nesting, although T-020's ruling Q-5 makes every new walk over the tree
   iterative and `tree.rs`'s module doc promises it. AC-6 added (P4).
12. Stale front matter and process text: `budget_tokens` 500000 → 700000 and the v1.2 comment;
   `touches` lacked the test file, `cursor.rs` and the crate doc of `kiln-syntax/src/lib.rs`
   (whose status sentence T-020 rewrites to "[`outline`] is still empty"); `docs/commands.json`
   moved to a generated-file comment; no Pinned (i) (D-091 change 5: N1–N14 added); no Pinned (f)
   against brittle pins (change 6: added); the odd-arm protocol was one line (now as T-021's);
   no AC named its file or its result on base (all fail on base by assertion: every `nav.*`
   request is `unknown_command` there).

**Notes (not blockers; answered by D-108, below).**
- N-1. The draft's AC-1 holds for an implementation of the Rules on the merged tree (P2), so
  the oracle is usable; AC-1 is not a vacuous test (the throwaway's first version was compared
  item by item, and a recursion bug would show as a depth difference).
- N-2. `docs/protocol/cursor.md` is not edited: its list at :8–9 (which T-021 extends with
  `search.replace`) names commands that edit the text. T-023's guarantee about the selection is
  in `nav.md` (N6). So T-021's edit of that file does not conflict with this task.
- N-3. Tier medium kept, for the director to confirm at `specified`: the files are reached only
  through T-019's merged walk and the unchanged `Vfs::read` (T-008's confinement code); this task
  adds no disk code, no dependency, no CHARTER §3 contract (adding commands is not one). The
  ADR-0002 criterion "paths that reach disk" is the one to weigh: the paths come from the walk,
  as T-022's draft argues for find-in-files.
- N-4. `nav.workspace-symbol` item: `crate = "kiln-workspace"` in its item file; the command is
  in `kiln-commands` (above). The director's field; `xtask scorecard` only checks that it names a
  workspace member (`xtask/src/scorecard.rs:144`).
- N-5. `Vfs` has no size query (`crates/kiln-workspace/src/vfs.rs`: `read`, `write_atomic`,
  `is_file`, `list_dir`), so a file's length is known only after `read`; a size-before-read
  check would be a `Vfs` change on the confinement boundary (high, T-036/T-037's files). Stated
  as a limit (N11), not attempted.
- N-6. T-031 (deferred) binds `ctrl+shift+o` to `nav.outline` with params `{}`, which this
  command refuses (`doc` is required). For T-031's own spec check.
- N-7. `nav.outline` has no `limit`: its reply is bounded by the size limit (Q-1) only. At
  1,048,576 chars of `mod m;` it would list about 175,000 items; T-021's check measured about
  1.3 kB per reply object on the way out (D-101 T2), so such a reply could take a few hundred
  MB. Not a blocker; accepted without a `limit` (D-108).

**Questions put to the director (ruled by D-108 as recommended; kept as the record).**
- Q-1 (size limit). Options: (a) **`nav.outline` and `nav.goto-symbol` refuse a document longer
  than 1,048,576 chars with `too_large`, and `nav.workspace-symbols` skips such a file with
  reason `too_large`**, reusing T-020's constant and kind; `nav.goto-line` (no parse) has no
  limit. Consequence: the same memory bound as `syntax.*` (689 MiB measured at the limit by
  T-020), one rule for the user; a document the `syntax.*` commands refuse has no outline either.
  (b) No limit: memory grows with the document without bound (T-020's figures scale with the
  length), and the outline and the tree commands disagree on what they accept. (c) A lower limit
  for `nav.workspace-symbols` only, to bound the per-file cost of a project search: no
  measurement behind any figure; it would be a new number to measure. Recommendation: (a).
- Q-2 (items below an `ERROR` node that is not the root). Options: (a) **`ERROR` counts as a
  container: an item under it is listed when the other nodes on the way are containers**
  (the draft's "listed if their node is present"); more items survive an edit that breaks the
  file, which is what an outline in an editor is for; no guarantee is made there. (b) Strict:
  nothing under a non-root `ERROR` is listed; the outline is smaller while the file is broken
  and the rule is simpler to state. In all eight probes (P3) both give the same outline, because
  `ERROR` contained items only as the root, which both options treat alike; no AC depends on the
  answer. Recommendation: (a), with N3's "not specified" wording.
- Q-3 (how much of the project). Options: (a) **walk with `limit` 1,000,000 (the largest
  `project.tree` allows); the reply carries `walk_truncated` and `too_deep` from the walk**;
  no new param. Consequence: entries are kept for the whole walk (about 1,000,000 paths at the
  cap; not measured), and a caller sees when the search was incomplete. (b) A `files_limit?`
  param (1..=1,000,000, default 50,000 as `project.tree`) with the same two fields: more control,
  one more param and default to document. (c) The draft's implicit `project.tree {}` (50,000)
  without reporting it: a large project is searched partly and the reply does not say so.
  Recommendation: (a).
- Q-4 (files that cannot be searched). Options: (a) **a file whose read fails (`not_found`,
  `not_a_file`, `outside_root`, `io_error`), that does not decode (`decode_error`) or that is too
  large (`too_large`) is listed in `skipped` with that kind's name and the search goes on; a
  text holding NUL is searched like any other (`doc.open` opens it), so there is no `binary`
  reason**; directory and `.gitignore` errors still fail the command (T-019's walk unchanged).
  Consequence: a file removed or replaced while the command runs does not fail a project-wide
  search; four of the reasons cannot be produced through `kiln-cli` in the gate, so they are
  covered by the implementer's unit test with a failing `Vfs` and the reviewer's reading.
  (b) Any read failure fails the whole command (as `project.tree` does for directories):
  simpler, but one vanished file makes the search fail. (c) (a) plus `binary` for NUL, as T-022's
  draft: a reason name that is not a kind, for a text `doc.open` accepts. Recommendation: (a).
- Split (D-091; METHODOLOGY §5.7). The split rule binds high-tier specs; this one is medium.
  The candidate line is the three document-level items (`syntax.outline`, `nav.goto-line`,
  `nav.goto-symbol`) against `nav.workspace-symbol`. For a split: Q-3 and Q-4 and the project
  limits (N8–N13) would not hold back three items; the first task would have no disk reads at
  all. Against: a second spec, check, review round and join (about one medium task's review
  cost, as T-021's check estimated), both editing `nav.rs` and `nav.md`, so serial; the
  workspace command is a loop over the outline and adds about a third of this spec. ADR-0005
  needs all four items in stage 1 either way (41 of 64 counts them). Recommendation: keep one
  task if Q-1 to Q-4 are ruled before dispatch; if Q-3/Q-4 are still open then, split off
  `nav.workspace-symbol` as its own task depending on this one. (They were ruled, D-108: one
  task.)
- Parallel work. Disjoint `touches` with T-036 and T-037 (`disk.rs`, `vfs.rs`, `paths.md`,
  `doc.rs`), so they may be in flight together. Not with T-038 (it touches `cursor.rs` and must
  see `nav.*` registered, D-092). Not with T-020 or T-021 (they must merge first).
### Rulings on Q-1 to Q-4 of the spec check R-180 (director, D-108)
- Q-1: (a). `nav.outline` and `nav.goto-symbol` refuse a document over 1,048,576 chars with `too_large`; `nav.workspace-symbol` skips such a file and reports it as `too_large`; `nav.goto-line` has no size limit.
- Q-2: (a). An `ERROR` node below the root is treated like the other containers.
- Q-3: (a). The workspace walk uses `limit` 1,000,000 and the reply reports `walk_truncated` and `too_deep`.
- Q-4: (a). A file that cannot be searched is listed in `skipped` with the existing kind name (`not_found`, `not_a_file`, `outside_root`, `io_error`, `decode_error`, `too_large`) and the search goes on; there is no `binary` kind; an error on a directory or a `.gitignore` still fails the command.
- Notes N-3 to N-7: the tier stays medium (every file is reached through T-019's walk and the unchanged `Vfs::read`; no new path to disk). The `crate` field of the `nav.workspace-symbol` item file is the director's and is looked at when the tests are tied at the join. N-5 (a file is read whole before its length is checked) stays as a stated limit. N-6 concerns T-031, a deferred draft, and is noted there when it is checked. N-7 is accepted.
- No test asserts a time, and no figure enters a document except from the implementer's release-build measurement (as for T-020 and T-021).
- Rulings on the implementer's report of R-194 (director, D-122). (1) The command is named `nav.workspace-symbols`, as the Interface, Pinned and the ACs have it; `nav.workspace-symbol` is the id of the scorecard item, and the director's dispatch message used the item's id for the command by mistake. (2) Sentence N11 says "about 340 MiB" for two runs whose maximum resident set size was 345,692 and 345,808 KiB, which is 337.6 and 337.7 MiB; the figure in the document becomes "about 338 MiB", computed as KiB divided by 1,024, like the figures in `docs/protocol/syntax.md`. Nothing else changes.

### To re-check at dispatch
Checked by the director on main `88c7d29` on 2026-11-14, after T-020 (`04a8fdf9c9b0d759eb131b6a7eae851cfa46708b`) and T-021 (`f01179bc68b7f760ebea674ca59e2731ac18fcf9`) merged: all nine facts below hold (commands: `grep` for `MAX_CHARS`, `TooLarge`, `struct SyntaxState`, `last successful`, `outline`, `fn root`, `fn char_at_byte`, the section headings of `docs/protocol/syntax.md`, `fn cursor_result`, `InvalidPattern`; `git diff 12f57a9..88c7d29 --stat` shows T-021 changed none of `cursor.rs`, `kiln-syntax` or `syntax.rs`). One difference in wording to know: the first sentence of the "Time" section of `syntax.md` now reads "the size limit bounds the length of the text a command accepts, not the time a parse takes or the memory it needs" (T-020 ruling r1, D-110).
Read by R-180/R-182 on the branches, not on main: T-020 at `c0c12fc30d473ff1d0f5b71c8ce9d4a0b66e0c12`,
T-021 at `46b93d64a018268b60528146cf16f55a36f10555`; their reviews may still change details. Once
both have merged, confirm on main (and edit the named lines of this spec if one differs):
1. `crates/kiln-commands/src/syntax.rs` has `pub(crate) const MAX_CHARS: usize = 1_048_576`, and
   `syntax.*` refuse `len_chars > MAX_CHARS` (exactly 1,048,576 accepted). Used by Interface,
   Pinned (a), N4, AC-7.
2. `RejectReason::TooLarge` with kind `too_large` exists in `crates/kiln-commands/src/reason.rs`.
3. T-020's tree cache (`SyntaxState`) is reachable from `nav.rs` only by editing `syntax.rs`
   (its `tree` is private), and `syntax.tree`'s `mode` is defined by "the last successful
   `syntax.*` call" (N7, AC-2). If either changed, N7 needs a look.
4. `crates/kiln-syntax/src/lib.rs`'s status sentence still says "[`outline`] is still empty"
   (`touches`: that sentence only), and `outline.rs` is still the empty slot.
5. `crates/kiln-syntax/src/tree.rs` still has the crate-private `root()` and `char_at_byte()`,
   and its module doc still says every walk is iterative.
6. `docs/protocol/syntax.md` still has the sections "Size limit" and "Time" that Pinned (e)
   and N12 cite.
7. `crates/kiln-commands/src/cursor.rs` still has the private `fn cursor_result` and
   `CursorResult` (the one visibility change in `touches`); T-021 does not touch it.
8. T-021 changed nothing this spec uses: `invalid_pattern` (T-032, on main) still exists; its
   edit of `docs/protocol/cursor.md` does not matter here (this task does not touch it).
9. `docs/commands.json` is generated (regenerated by the queue on conflict, D-097).

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13 · R-056 · (new) · did: grouped all navigation items into one command group. The
  outline oracle is syn (an independent parser), and the workspace-symbol oracle is a model
  built from public commands. Lines and cols are 0-based, matching T-010's `doc.position`. Tier
  medium (no ADR-0002 high criterion; the walk is T-019's). Arm odd, paired against the even
  T-020. The outline example's leaves were checked in a tree-sitter spike; the goto-line
  example was executed against a naive model. Pinned (e), (f) n/a. · gate attempts: 0 · missing
  context: none · next: director.
- 2026-11-13 · R-066 · draft · did: `invalid_pattern` and the reason API are T-032's (D-041); `depends_on` gains T-032 explicitly. · gate attempts: 0 · missing context: none · next: director.
- 2026-11-13 · R-079 · draft · did: followed T-016's revised contract (R-077/R-079): corpus
  path `crates/kiln-syntax/tests/corpus/`, `.rs.txt` files selected by MANIFEST `set`; AC-5
  copies them as `<stem>.rs` (spec check R-067 note 17); `parse` returns `Result` (handling
  PROPOSED in Interface). Re-ran the (g) outline text's tree on 0.27.1 and 0.27.2: identical.
  · gate attempts: 0 · missing context: none · next: director.
  QUESTION: on `ParseError::NoTree`, fail `nav.outline`/`nav.workspace-symbols` with
  `internal`/`other` (proposed), or list the file under `skipped` with a new reason?
- 2026-11-13 · R-180 · draft · did: spec check against main `49d9233` (T-016, T-018, T-019 at
  `3b1a95d`, T-032 merged; `d162413` differs in `handoffs/` only) and the unmerged siblings T-020
  (`c0c12fc`) and T-021 (`46b93d6`), read with `git show` from the branches (the T-020 worktree
  held an uncommitted review mutant, so it was not used). Found 12 blockers or corrections, all
  fixed or turned into Q-1 to Q-4 ("Spec check R-180"): T-022's unmerged text rule and the
  non-kind `binary`; `CursorResult` private to `cursor.rs` (one visibility change added to
  `touches`); serde on `ItemKind` would be a dependency; no size limit (T-020's `too_large`,
  Q-1); ancestor rule against the ERROR sentence and a root that is `ERROR` (Q-2); the walk's
  limit, truncation, depth bound and read failures unstated (Q-3, Q-4); (g) lacked `start`/`end`;
  test-file stem and names; `toml` not available to `kiln-cli` tests; typing group of
  goto-symbol; no deep-nesting AC; front matter (budget 700,000, `touches`, generated
  `docs/commands.json`). Applied v1.2: Pinned (i) N1–N14 with evidence and two release-build
  figure slots (N11 memory, N12 time), Pinned (f), the odd-arm protocol, each AC's file and
  base result (all fail on base by assertion), the per-item test table without `scorecard/`
  paths. Probes (debug, private clone, removed): a throwaway outline from the Rules reproduced
  every (g) leaf and agreed with syn on all 33 `valid` corpus files (555 items, 0 differences);
  20,000 nested mods gave 20,001 items; (g) 6 and 7 executed against a Python model. Split:
  keep one task unless Q-3/Q-4 are open at dispatch. Tier medium kept for the director to
  confirm (N-3). D-044's ruling acknowledged and applied (PROPOSED removed). Pinned (e) is no
  longer n/a (resource bounds). · gate attempts: 0 · not run: `cargo xtask ci`; no `kiln-cli`
  built; any release-build measurement (none needed; N11/N12 are the implementer's) · missing
  context: none · next: director rules Q-1 to Q-4; dispatch after T-020 and T-021 merge (re-check
  the "Assumed of the siblings" lines if they merge differently). Did not write a
  `tasks/spec-checks/T-023.md` file (this run's brief allowed edits to this file only).
  QUESTION: Q-1 (size limit: recommend (a) `too_large` as `syntax.*`), Q-2 (`ERROR` below the
  root: recommend (a) a container), Q-3 (walk extent: recommend (a) 1,000,000 with
  `walk_truncated`, `too_deep`), Q-4 (unsearchable files: recommend (a) skip with the kind's name,
  no `binary`).
- 2026-11-13 · R-182 · draft · did: finalised the spec on D-108 (Q-1 to Q-4 (a), tier medium,
  notes N-3 to N-7 answered); rulings acknowledged and accepted. Every PENDING mark replaced by
  its D-108 ruling; the R-180 verdict, question and note headings now read as the record. The
  sibling assumptions moved to a new "To re-check at dispatch" list (nine facts, with the branch
  heads read: T-020 `c0c12fc30d473ff1d0f5b71c8ce9d4a0b66e0c12`, T-021
  `46b93d64a018268b60528146cf16f55a36f10555`). Builder read-through, top to bottom: no
  contradiction found between Interface, Pinned, ACs and `touches`; AC-7's "a file of 1,048,576
  chars is searched" made observable ("not in `skipped`"); (g) example 2's Q-2 remark restated.
  Time and figure check: removed T-020's 17.00 s figure from Pinned (e) (now a pointer to
  `syntax.md`, Time), the probe's 0.12 s from AC-6 and the debug times from Out of scope, and
  "for hours" from Pinned (e) (no measurement behind it); the remaining figures are exact
  behaviours, input sizes, probe counts given as evidence, or the implementer's release-build
  slots `<M>` (N11) and `<T>` (N12). No AC reads `error.message`, a whole document, the command
  list or `error_kinds` (Pinned (f); AC preamble). · gate attempts: 0 · not run: no git, no
  cargo (none needed) · missing context: none · next: director: ready to dispatch once T-020 and
  T-021 have merged and "To re-check at dispatch" holds on main.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
