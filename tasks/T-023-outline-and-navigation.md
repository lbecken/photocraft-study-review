---
id: T-023
title: "List a document's outline, go to a line and column clamped to the document, go to an outline symbol, and find symbols by name across the project's Rust files"
tier: medium            # ADR-0002 default: in-memory plus T-019's existing read path; no new dependency
state: draft
owner_role: planner
scorecard_items: [syntax.outline, nav.goto-line, nav.goto-symbol, nav.workspace-symbol]
depends_on: [T-016, T-018, T-019, T-032]
touches:
  - crates/kiln-syntax/src/outline.rs        # slot from T-016
  - crates/kiln-syntax/tests/outline.rs      # implementer's own integration tests
  - crates/kiln-commands/src/nav.rs          # nav.* (slot from T-015)
  - docs/protocol/nav.md                     # new
  - docs/commands.json                       # generated
budget_tokens: 500000   # medium (v1.1, provisional)
iteration: 1
found_by: planner
origin_task: null
---

# T-023 Outline and navigation

## Goal
After this task a script can:
- list a document's outline: the `fn`, `struct`, `enum`, `trait`, `impl` and `mod` items, each
  with its name, range, name range and nesting depth;
- move the selection to a line and column (both clamped to the document);
- select an outline symbol's name;
- find symbols by name across every Rust file of the project. An open document's current text
  is used in place of its file.

No gate path. Arm: **odd** (the implementer writes the acceptance tests first on
`task/T-023-tests`).

Reading list: `tasks/T-016-*.md` (SyntaxTree, corpus); `tasks/T-018-*.md` (selection, `CursorResult`);
`tasks/T-019-*.md` (walk); `tasks/T-022-*.md` "Text of a file" (same rule here);
`tasks/T-010-*.md` (0-based `line`/`col`).

## Interface

### `kiln_syntax::outline`
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind { Fn, Struct, Enum, Trait, Impl, Mod }       // serde/as_str: "fn", "struct", …
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineItem { pub kind: ItemKind, pub name: String, pub start: usize, pub end: usize,
                         pub name_start: usize, pub name_end: usize, pub depth: usize }
pub fn outline(tree: &SyntaxTree) -> Vec<OutlineItem>;          // preorder by start
```
Rules (exact):
- Item nodes: `function_item` and `function_signature_item` → `fn`; `struct_item`;
  `enum_item`; `trait_item`; `impl_item`; `mod_item` (inline or `mod x;`).
- An item is listed iff every ancestor between it and the root is `source_file`,
  `declaration_list`, `mod_item`, `impl_item` or `trait_item`. So nothing inside fn bodies,
  blocks, `extern` blocks or macro invocations is listed. `depth` = the number of listed items
  that enclose it.
- `name`: the source text of the `name` field. For `impl`, it is the source text of the `type`
  field, or `<trait text> for <type text>` when there is a `trait` field, with every run of
  whitespace collapsed to one space. `name_start`/`name_end` cover the `name` field, or for
  `impl` the `type` field. `start`/`end` cover the whole item node, attributes and doc comments
  excluded. All offsets are chars.
- Items in ERROR subtrees are listed if their node is present; no guarantee is made there.

### Commands (`doc` must be open)
| Command | Mutating | Params | Result |
|---|---|---|---|
| `nav.outline` | no | `{doc}` | `{items:[OutlineItem as JSON]}` |
| `nav.goto-line` | yes | `{doc, line, col?}` (0-based; `col` default 0) | `CursorResult` (T-018) |
| `nav.goto-symbol` | yes | `{doc, index}` | `{item, cursor: CursorResult}` |
| `nav.workspace-symbols` | no | `{query, limit?}` (1..=10000, default 200) | `{items:[{path, kind, name, start, end, name_start, name_end, line, col}], truncated, skipped:[{path, reason}]}` |

- `nav.goto-line`: line = `min(line, len_lines − 1)`; col = `min(col, content length of that
  line)`. The caret goes there (anchor = head), and the typing group ends.
- `nav.goto-symbol`: `index` into `nav.outline`'s items. The selection becomes anchor =
  `name_start`, head = `name_end`. An `index` ≥ the number of items → `rejected`/`out_of_range`.
- `nav.workspace-symbols`: files = `project.tree {}` entries of kind `file` whose name ends in
  `.rs`. Text per T-022's rule (skip `decode_error`, `binary`). Each file's `outline` is used.
  - An item matches iff `name.to_lowercase()` contains `query.to_lowercase()`. An empty
    `query` → `rejected`/`invalid_pattern`.
  - Rank: exact match (equal lowercase) first, then prefix matches, then other matches; within
    a rank, by path in tree order, then `start`. The first `limit` are kept.
  - `line`/`col` are 0-based, of `name_start`.

No new error kind. `docs/protocol/nav.md` documents the commands and rules.

## Pinned
- (a) Precedence: schema → `no_such_document` → `out_of_range`; workspace symbols: schema →
  `invalid_pattern` → walk errors (T-019).
- (b) Files: read only, through T-019's walk and `Vfs::read`; nothing written.
- (c) Order: as in the rules.
- (d) Offsets chars; lines split on LF, CRLF, lone CR; RelPaths.
- (e) Threat model: n/a beyond T-019's walk (reused unchanged).
- (f) Snapshots: none.
- (g) Worked examples:
  - Text (spike-checked leaves) `impl<T> Display for Foo<T> { fn fmt(&self) {} }\nmod m;\ntrait T { fn x(&self); }`
    → items: `impl` `Display for Foo<T>`, name range `20..26`, depth 0; `fn` `fmt`
    `32..35`, depth 1; `mod` `m` `52..53`, depth 0; `trait` `T` `61..62`, depth 0; `fn` `x`
    `68..69`, depth 1.
  - Text `ab\r\ncdef\n` (executed against a naive model): `nav.goto-line {line:1, col:9}` →
    head 8, line 1, col 4. `{line:8}` → head 9, line 2, col 0. `{line:0, col:1}` → head 1.
  - On the first text, `nav.goto-symbol {index:1}` → anchor 32, head 35.
- (h) Evaluator calls: `kiln-cli run` with `doc.open`, `nav.outline` (`syntax.outline`),
  `nav.goto-line` + `cursor.get` (`nav.goto-line`), `nav.goto-symbol` (`nav.goto-symbol`), and
  `nav.workspace-symbols` over a project root (`nav.workspace-symbol`).

## Acceptance criteria
- AC-1 (`syntax.outline`, differential with syn): on every `valid` corpus file, the
  `nav.outline` items of kinds other than `impl` equal the items found by walking `syn::File`.
  The walk takes `Item::{Fn, Struct, Enum, Trait, Mod}`, recurses into inline `mod` content and
  into `Item::Impl`/`Item::Trait` for `ImplItem::Fn`/`TraitItem::Fn`, and never enters fn
  bodies. They are compared as (kind, ident text, ident start offset, depth). `impl` items
  equal syn's `Item::Impl`s by (start offset of `self_ty`, depth). Oracle: an independent
  parser (syn).
- AC-2 (`syntax.outline`, exact): the worked example holds exactly, including `name`, `start`
  and `end` for every item. Oracle: hand-written expectation (spike-checked).
- AC-3 (`nav.goto-line`): for random texts with LF/CRLF/CR and random `line`/`col` (including
  far out of range), the head equals a naive clamp model. A negative or non-integer `line` →
  `invalid_params`. Oracle: naive model.
- AC-4 (`nav.goto-symbol`): for every corpus file and every index, `cursor.get` after
  `nav.goto-symbol` equals `{anchor: name_start, head: name_end}` of that item. The source slice
  of that range equals `name` (except `impl`). `index` = count → `out_of_range`. Oracle:
  consistency with `nav.outline` + exact value.
- AC-5 (`nav.workspace-symbol`): for a project made of corpus files (with a `.gitignore`d
  copy, a `target/` copy and a non-UTF-8 `.rs` file) and ≥ 30 queries, the result equals a
  naive model. The model opens each `project.tree` `.rs` file in a fresh session, calls
  `nav.outline`, then filters and ranks by the rule. After `doc.insert` of `fn zzz_new() {}`
  into an open file, the query `zzz_new` finds it in that file. Oracle: naive model built from
  public commands.

## Scorecard items and intended test names
| Item | Tests |
|---|---|
| syntax.outline | `kiln_cli::acceptance_t023::outline_agrees_with_syn`, `kiln_cli::acceptance_t023::outline_worked_example` |
| nav.goto-line | `kiln_cli::acceptance_t023::goto_line_clamps` |
| nav.goto-symbol | `kiln_cli::acceptance_t023::goto_symbol_selects_name` |
| nav.workspace-symbol | `kiln_cli::acceptance_t023::workspace_symbols_match_naive_model` |

## Out of scope and implementer notes
- Other item kinds (const, static, type, union, macro_rules, enum variants, fields); fuzzy
  ranking (the palette task); a cached symbol index (each call parses; performance is not
  budgeted here).
- Your own tests: `crates/kiln-syntax/tests/outline.rs`, unit tests. Acceptance tests (odd arm)
  go first on `task/T-023-tests`.

## Questions and rulings

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
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
