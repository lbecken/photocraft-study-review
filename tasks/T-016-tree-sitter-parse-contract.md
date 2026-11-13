---
id: T-016
title: "Parse Rust with tree-sitter in kiln-syntax: a syntax tree with error spans, a pinned S-expression form and incremental update, checked against a pinned compiling corpus"
tier: high              # ADR-0002: adds external dependencies (tree-sitter, tree-sitter-rust) and dev-dependencies
state: draft
owner_role: planner
scorecard_items: []     # library contract only; syntax.* items are claimed through commands by T-020 / T-023
depends_on: []
touches:
  - crates/kiln-syntax/Cargo.toml          # + tree-sitter, tree-sitter-rust; dev: syn, proc-macro2
  - crates/kiln-syntax/src                 # lib.rs, tree.rs; empty highlight.rs, brackets.rs, outline.rs
  - crates/kiln-syntax/tests/tree.rs       # implementer's own integration tests
  - crates/kiln-testkit/corpus             # the pinned corpus (written by the TEST AUTHOR, see Pinned (b))
  - apps/kiln-cli/Cargo.toml               # [dev-dependencies] syn, proc-macro2 only (oracles for T-020/T-023)
  - Cargo.lock                             # generated
budget_tokens: 850000   # high (v1.1)
iteration: 1
found_by: planner
origin_task: null
---

# T-016 Parse Rust with tree-sitter in kiln-syntax

## Goal
`kiln-syntax` is empty. After this task a caller can parse a Rust source text into a
`SyntaxTree` that reports whether it holds error or missing nodes, and where. It can render the
tree in a pinned S-expression form with char offsets, and update it for a new text
incrementally. The result always equals a parse from scratch. A pinned corpus of compiling Rust
files lives in `crates/kiln-testkit/corpus/`. That gives the constructed oracle "compiles ⇒
parses without error nodes" (CHARTER §3 decision 5, claim C8) its input, here and in T-020 and
T-023. This task carries the dependency risk alone, so the feature tasks built on it can be
medium.

No gate path is touched (dependency build scripts are not workspace files).

Reading list: CHARTER §3 (decision 5); ADR-0002; `crates/kiln-syntax/src/lib.rs`; `xtask/layers.toml`
(kiln-syntax is L1 and depends on no workspace crate).

## Interface

### Dependencies
In `crates/kiln-syntax/Cargo.toml`, exact pins (`=x.y.z`), declared in the crate manifest and
not in the root `[workspace.dependencies]` (director, R-060 answer 1):
- `tree-sitter` and `tree-sitter-rust`: the newest mutually compatible releases that are at
  least two weeks old, have MSRV ≤ 1.97 and build with the machine's C compiler. As of planning,
  `tree-sitter =0.27.2` (MSRV 1.90) with `tree-sitter-rust =0.24.2` built and parsed in a
  scratch spike (INFERRED fit, not reviewed). Record the chosen versions and the reason in
  your log.
- `[dev-dependencies]`: `syn.workspace = true`, `proc-macro2.workspace = true` (already in
  `Cargo.lock`; for the differential oracles of T-020 and T-023). The same two go into
  `apps/kiln-cli/Cargo.toml` `[dev-dependencies]` and nowhere else.

### Rust (`kiln_syntax::…`)
```rust
pub mod tree;        // this task
pub mod highlight;   // empty (//! only), filled by T-020
pub mod brackets;    // empty, filled by T-020
pub mod outline;     // empty, filled by T-023
pub use tree::{ErrorKind, ErrorSpan, ParseMode, SyntaxTree};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind { Error, Missing }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorSpan { pub kind: ErrorKind, pub start: usize, pub end: usize }   // char offsets
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseMode { Unchanged, Incremental, Full }

impl SyntaxTree {
    /// Parse `text` from scratch.
    pub fn parse(text: &str) -> SyntaxTree;
    /// Bring the tree to `new_text`. `Unchanged` iff `new_text` equals the current text.
    /// Otherwise reuse the old tree for the single changed region (common char prefix and
    /// suffix of old and new text) and report `Incremental`. The implementation may instead
    /// parse from scratch and report `Full`, but only when the incrementally parsed tree
    /// `has_error()`.
    pub fn update(&mut self, new_text: &str) -> ParseMode;
    pub fn text(&self) -> &str;
    pub fn has_error(&self) -> bool;               // any ERROR or MISSING node
    pub fn errors(&self) -> Vec<ErrorSpan>;        // every ERROR and MISSING node, preorder
    pub fn sexp(&self) -> String;                  // pinned form below
    pub fn node_count(&self) -> usize;             // all nodes, named and anonymous, root included
}
```
`SyntaxTree` does not expose `tree_sitter` types publicly; sibling modules use crate-private
accessors that you design. Nothing panics on any input, including empty text, NUL, lone `\r`
and text that is not Rust.

### S-expression form (`sexp`), exact
Preorder over **named** nodes plus MISSING nodes. A node prints as `(` KIND `@` START `..` END,
then ` ` + child for each child, then `)`. START and END are char offsets.
- KIND is the grammar's node kind; an ERROR node is `ERROR`.
- A child reached through a field prints as `FIELD:(` …, with no space after the colon.
- A MISSING node prints `(MISSING KIND@S..S)`; if its kind is anonymous, KIND is written in
  double quotes, e.g. `(MISSING ";"@9..9)`.
- Anonymous non-missing nodes are omitted (they still count in `node_count`).

Example (executed against tree-sitter 0.27.2 / tree-sitter-rust 0.24.2 in a scratch spike; the
grammar's own `to_sexp` was `(source_file (function_item name: (identifier) parameters:
(parameters) body: (block)))`):
`SyntaxTree::parse("fn f() {}").sexp()` = `(source_file@0..9 (function_item@0..9
name:(identifier@3..4) parameters:(parameters@4..6) body:(block@7..9)))` (one line).

## Pinned
- (a) Errors: none. Every input parses; error recovery shows up as `ErrorSpan`s.
- (b) Corpus layout. The **test author** writes it on `task/T-016-tests`; the implementer
  never edits it:
  - `crates/kiln-testkit/corpus/rust/*.rs`: ≥ 30 files, ≥ 4,000 lines in all. Each compiles
    alone with `rustc --edition 2024 --crate-type lib --emit metadata`, std only. Crate-level
    `#![allow(...)]` is permitted. Files are UTF-8 without BOM. One file uses CRLF line endings
    and one has non-ASCII identifiers and strings. Between them the files contain every
    construct in this checklist:
    - items: generics with bounds and `where`; lifetimes; `const` generics; associated types
      and consts; trait objects and `impl Trait`; `use<>` precise capturing; `async fn`, async
      blocks, `.await`;
    - control flow: closures with and without `move`; `let … else`; `if let` chains;
      `match` with guards, bindings `@` and nested or-patterns; labeled loops and `break 'a v`;
    - literals: raw strings `r#"…"#`, byte strings, C strings `c"…"`, chars, byte chars,
      integer and float suffixes, numeric separators;
    - attributes inner and outer; doc comments `///`, `//!`, `/** */`; nested block comments;
    - modules: inline `mod`, `macro_rules!` with repetitions, macro invocations in item,
      statement and expression position;
    - other: `unsafe` blocks and `unsafe fn`, `extern "C"` blocks, raw identifiers `r#type`,
      `?`, ranges, slices and arrays, tuple structs, enums with discriminants, `impl` blocks
      for traits with generics, `dyn`, `where Self: Sized`.
  - `corpus/rust-gaps/*.rs`: files that compile but on which the pinned grammar reports
    errors. Each records a known grammar gap rather than hiding it. Example, VERIFIED in the
    spike: `unsafe extern "C" { safe fn x(); }` gives `has_error: true`.
  - `corpus/rust-invalid/*.rs`: ≥ 8 files that rustc rejects for a **syntax** error
    (unbalanced delimiters, `fn (`, `let = ;`, a truncated item …).
  - `corpus/MANIFEST.toml`: one `[[file]]` per file, with `path` (relative to `corpus/`),
    `set` (`valid` | `gap` | `invalid`), `bytes`, `lines`, `fnv1a64` (16 lowercase hex digits
    of FNV-1a 64 over the file's bytes), `origin` (`written for kiln`, or `adapted from <repo
    path> at <commit>`), and for `gap` a `gap = "<one sentence>"`. Nothing is downloaded.
  The pin is the manifest. A file whose bytes do not match its entry, or a file without an
  entry, fails the corpus test.
- (c) Order: n/a (pure functions).
- (d) Offsets are chars (Unicode scalar values), as in the control protocol. CRLF counts as two
  chars. Platform independent.
- (e) Threat model (dependency): the grammar is C code compiled into the binary. The reviewer
  checks the pinned versions, that no `build.rs` is added to the workspace, and that parsing a
  50 MB text and adversarial nesting (10,000 nested `(`) neither panics nor overflows the
  stack. Time is not bounded by this task.
- (f) Snapshots: none (the crate is empty today).
- (g) Worked examples (spike, tree-sitter 0.27.2 / rust 0.24.2):
  `has_error` is false for `fn f(x: Option<i32>) { if let Some(a) = x && a > 1 { } }`, for
  `fn g() -> impl Sized + use<> { 0 }` and for `let Some(x) = Some(1) else { return };` in a
  fn. It is true for `fn a() { (`. The sexp of `fn f() {}` is above.
- (h) Scorecard: none (T-020 and T-023 claim the items through commands).

## Acceptance criteria
- AC-1 (compiles ⇒ no error nodes, pinned corpus): a test proves the corpus first. Every
  `valid` and `gap` file compiles with `rustc` (spawned with a 60 s timeout; this is the one
  marked test that spawns `rustc`; the edition and crate type are pinned above). Every
  `invalid` file fails to compile, and the manifest matches the bytes. Then every `valid` file
  gives `has_error() == false` and `errors() == []`, and every `gap` file gives
  `has_error() == true`. Oracle: rustc + the parser.
- AC-2 (compiles ⇒ no error nodes, live corpus): every `.rs` file under `crates/*/src`,
  `apps/*/src` and `xtask/src` of the repository at test time parses without errors. These
  files compile, because the gate builds them. Oracle: the gate's own compile.
- AC-3 (errors are found): every `invalid` file has at least one `ErrorSpan`, and every span
  lies within `0..=len_chars`. Oracle: exact property.
- AC-4 (incremental == from scratch): take every `valid` and `invalid` corpus file. Apply ≥ 200
  fixed-seed random edits per file: insert, delete or replace of 0–20 chars drawn from the
  file's own text, `{}();"'/\r\n` and `é😀`. After each edit `update(new)` gives
  `sexp() == SyntaxTree::parse(new).sexp()`, `errors()` equal and `node_count()` equal. The mode
  is `Incremental` whenever the new tree has no error, and never `Full` when it has none.
  Oracle: from-scratch parse.
- AC-5 (format): the sexp of the worked example is exact. For ≥ 3 hand-written inputs with a
  MISSING node, the `(MISSING …)` form is exact. Oracle: hand-written expectation, checked
  against the grammar's own `to_sexp` by the test author.
- AC-6 (robustness): `parse` of `""`, `"\0"`, `"\r"`, 1 MB of `(`, and 1 MB of random bytes
  decoded lossily returns; offsets are in range. Oracle: property.

## Scorecard items and intended test names
None. Tests: `crates/kiln-syntax/tests/acceptance_t016.rs` (`corpus_is_pinned_and_compiles`,
`compiling_corpus_parses_without_errors`, `workspace_sources_parse_without_errors`,
`invalid_corpus_reports_errors`, `incremental_update_equals_fresh_parse`, `sexp_form_is_exact`,
`hostile_inputs_parse`).

## Out of scope and implementer notes
- Highlight, brackets, outline (T-020, T-023); any command (T-020); `kiln-text` types (the API
  takes `&str`).
- Your tests: unit tests in `src/tree.rs` and `crates/kiln-syntax/tests/tree.rs`. You may write
  a small private corpus for them in the test files; do not touch `crates/kiln-testkit/corpus`.
- Determinism: tree-sitter parsing is deterministic. Set no timeout or cancellation flag.

## Questions and rulings

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13 · R-056 · (new) · did: specified the tree-sitter contract and corpus. Tier high,
  by ADR-0002 "adding or changing a dependency". Spiked tree-sitter 0.27.2 + tree-sitter-rust
  0.24.2 in a scratch crate (now removed). It builds here. Let-chains, `use<>` and `let … else`
  parse cleanly; `unsafe extern { safe fn }` (2024) does not, hence the `gap` set. The corpus
  is written by the test author so that the implementer cannot pick an easy one. Dev-deps
  syn/proc-macro2 are added here (high) so that T-020/T-023 can stay medium.
  `crates/kiln-testkit` gets data only, no code or Cargo change. Pinned (a), (c), (f), (h) n/a.
  Unsure: whether the corpus should be data-only or also have a loader in kiln-testkit (that
  would be a dev-dependency edge, high, in every consumer); I chose data-only, read by path. ·
  gate attempts: 0 · missing context: none · next: director.
- 2026-11-13 · R-060 · draft · did: dependency placement confirmed by the director (crate manifest; `Cargo.lock` regenerated by the queue). Still `depends_on: []`. · gate attempts: 0 · missing context: none · next: director.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
