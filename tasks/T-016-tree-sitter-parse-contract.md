---
id: T-016
title: "Parse Rust with tree-sitter in kiln-syntax: a syntax tree with error spans, a pinned S-expression form and incremental update, checked against a pinned compiling corpus"
tier: high              # ADR-0002: adds external dependencies (tree-sitter, tree-sitter-rust) and dev-dependencies
state: ready
owner_role: planner
scorecard_items: []     # library contract only; syntax.* items are claimed through commands by T-020 / T-023
depends_on: []
touches:
  - crates/kiln-syntax/Cargo.toml          # + tree-sitter, tree-sitter-rust; dev: syn, proc-macro2, toml
  - crates/kiln-syntax/src                 # lib.rs, tree.rs; empty highlight.rs, brackets.rs, outline.rs
  - crates/kiln-syntax/tests/tree.rs       # implementer's own integration tests
  - crates/kiln-syntax/tests/corpus        # TEST AUTHOR only: corpus files, MANIFEST.toml (hashes), corpus-rustc-run.toml; see Pinned (b)
  - apps/kiln-cli/Cargo.toml               # [dev-dependencies] syn, proc-macro2 only (oracles for T-020/T-023)
  - Cargo.lock                             # generated; the queue regenerates it, so overlap with another in-flight task on this path is permitted
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
files lives in `crates/kiln-syntax/tests/corpus/`. That gives the constructed oracle "compiles ⇒
parses without error nodes" (CHARTER §3 decision 5, claim C8) its input, here and in T-020 and
T-023. This task carries the dependency risk alone, so the feature tasks built on it can be
medium.

No gate path is touched (dependency build scripts are not workspace files), so the approval
row does not need `gate paths`. It does not need `dependency versions` either: the change only
adds packages to `Cargo.lock` (R-067 verified 7 added, none removed or re-versioned). The task is
tier high because it adds a dependency (ADR-0002).

Reading list: CHARTER §3 (decision 5); ADR-0002; `crates/kiln-syntax/src/lib.rs`; `xtask/layers.toml`
(kiln-syntax is L1 and depends on no workspace crate).

## Interface

### Dependencies
In `crates/kiln-syntax/Cargo.toml`, exact pins (`=x.y.z`), declared in the crate manifest and
not in the root `[workspace.dependencies]` (director, R-060 answer 1):
- `tree-sitter = "=0.27.1"` (default features only; never `wasm`) and
  `tree-sitter-rust = "=0.24.2"`.
  Rule: for each of these two direct dependencies, the newest mutually compatible release that
  was at least two weeks old at specification (2026-11-13), has MSRV ≤ 1.97 and builds with the
  machine's C compiler. That selects `tree-sitter` 0.27.1 (published 2026-10-08; 0.27.2 of
  2026-11-07 was six days old) and `tree-sitter-rust` 0.24.2 (2026-03-27); MSRV 1.90 / none.
  The versions are fixed by this spec and do not move with the date. The age rule covers the
  two direct dependencies only; the packages they pull in are what the resolver locks.
  VERIFIED (R-067): builds with gcc 13.3, output identical to 0.27.2 on the spike inputs;
  every worked example below re-executed with 0.27.1 (R-077).
- `[dev-dependencies]`: `syn.workspace = true`, `proc-macro2.workspace = true` (already in
  `Cargo.lock`; for `corpus_is_pinned_and_compiles` and the differential oracles of T-020 and
  T-023) and `toml.workspace = true` (toml 0.8.23, already locked; the tests read
  `MANIFEST.toml` and the run record). `syn` and `proc-macro2` also go into
  `apps/kiln-cli/Cargo.toml` `[dev-dependencies]`, and nowhere else.

### Rust (`kiln_syntax::…`)
```rust
pub mod tree;        // this task
pub mod highlight;   // empty (//! only), filled by T-020
pub mod brackets;    // empty, filled by T-020
pub mod outline;     // empty, filled by T-023
pub use tree::{ErrorKind, ErrorSpan, ParseError, ParseMode, SyntaxTree};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind { Error, Missing }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorSpan { pub kind: ErrorKind, pub start: usize, pub end: usize }   // char offsets
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseMode { Unchanged, Incremental, Full }
/// tree-sitter produced no tree. `Display`: `tree-sitter returned no tree`; implements
/// `std::error::Error`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError { NoTree }

#[derive(Clone)]           // + a hand-written `Debug`, see below
pub struct SyntaxTree { /* private */ }

impl SyntaxTree {
    /// Parse `text` from scratch.
    pub fn parse(text: &str) -> Result<SyntaxTree, ParseError>;
    /// Bring the tree to `new_text`. `Ok(Unchanged)` iff `new_text` equals the current text.
    /// Otherwise reuse the old tree for the single changed region (common char prefix and
    /// suffix of old and new text) and report `Ok(Incremental)`. The implementation may
    /// instead parse from scratch and report `Ok(Full)`, but only when the incrementally parsed
    /// tree `has_error()`. On `Err`, `self` is unchanged (old text, old tree).
    pub fn update(&mut self, new_text: &str) -> Result<ParseMode, ParseError>;
    pub fn text(&self) -> &str;
    pub fn has_error(&self) -> bool;               // any ERROR or MISSING node
    pub fn errors(&self) -> Vec<ErrorSpan>;        // every ERROR and MISSING node, preorder
    pub fn sexp(&self) -> String;                  // pinned form below
    pub fn node_count(&self) -> usize;             // all nodes, named and anonymous, root included
}
```
- `SyntaxTree: Clone + Debug + Send` (T-015's `SyntaxState` derives `Debug` and T-020 stores
  trees in it). It keeps no `tree_sitter::Parser`, which implements neither `Clone` nor
  `Debug`; a parser is made per `parse`/`update`. `Debug` is hand-written and prints the char
  length of the text and `has_error`, not the text; its exact format is not pinned.
- `ParseError::NoTree` is returned when tree-sitter rejects the pinned grammar in
  `Parser::set_language` or `Parser::parse` returns no tree. With no timeout or cancellation
  flag set (see notes), neither happens for the pinned versions; a unit test in `src/tree.rs`
  proves that `set_language` succeeds for the pinned grammar. No `unwrap`, `expect`, `panic` or
  lint allowance is used for this path.
- `SyntaxTree` does not expose `tree_sitter` types publicly. Sibling modules use exactly these
  crate-private accessors, which T-020 (highlight, brackets) and T-023 (outline) rely on and
  cannot add themselves (`tree.rs` and `lib.rs` are outside their `touches`):
  ```rust
  impl SyntaxTree {
      #[allow(dead_code)] // used by highlight (T-020) and outline (T-023)
      pub(crate) fn root(&self) -> tree_sitter::Node<'_>;
      #[allow(dead_code)] // byte offset in `text()` → char offset
      pub(crate) fn char_at_byte(&self, byte: usize) -> usize;
  }
  ```
  `allow`, not `expect`: T-020/T-023 cannot remove an `expect` from a file outside their
  `touches`.
- Nothing panics on any input, including empty text, NUL, lone `\r` and text that is not Rust.

### S-expression form (`sexp`), exact
Preorder over **named** nodes plus MISSING nodes. A node prints as `(` KIND `@` START `..` END,
then ` ` + child for each child, then `)`. START and END are char offsets.
- KIND is the grammar's node kind; an ERROR node is `ERROR`.
- A child reached through a field prints as `FIELD:(` …, with no space after the colon.
- A MISSING node prints `(MISSING KIND@S..S)`; if its kind is anonymous, KIND is written in
  double quotes, e.g. `(MISSING ";"@9..9)`. Inside the quotes, `\` and `"` are written as `\\`
  and `\"`; no other escaping. tree-sitter-rust produces a MISSING `"` (see AC-5).
- Anonymous non-missing nodes are omitted (they still count in `node_count`).

Example (executed against tree-sitter 0.27.2 / tree-sitter-rust 0.24.2 in a scratch spike, and
again against 0.27.1 / 0.24.2 in R-077 with the same result; the grammar's own `to_sexp` was
`(source_file (function_item name: (identifier) parameters: (parameters) body: (block)))`):
`SyntaxTree::parse("fn f() {}")?.sexp()` = `(source_file@0..9 (function_item@0..9
name:(identifier@3..4) parameters:(parameters@4..6) body:(block@7..9)))` (one line).

## Pinned
- (a) Errors: one type, `ParseError`, with one variant, `NoTree` (Interface). Malformed Rust is
  not an error: every input yields a tree, and error recovery shows up as `ErrorSpan`s.
- (b) Corpus layout. The corpus is a test fixture in `crates/kiln-syntax/tests/corpus/`. The
  **test author** delivers all of it on `task/T-016-tests`, in the same commits as the
  acceptance tests: the corpus files, `MANIFEST.toml` (the content hashes) and
  `corpus-rustc-run.toml` (the run record). Only the test author writes anything under this
  directory; the implementer never reads or edits it, and the implementer's code and unit
  tests do not depend on its contents. The directory is listed in `touches` because the
  queue's allowed-path rule admits only `acceptance_tNNN*.rs`/`guard_tNNN*.rs` files directly
  in `tests/` (precheck.rs:319–332, 344–360), not fixture files or subdirectories. Cargo builds no
  test target from it (it has no `.rs` file and no `main.rs`). No corpus file has the extension `.rs`: every source file is stored as
  `<name>.rs.txt`, because the gate's `no-skip` step parses every `.rs` file under `crates/`
  with `syn` and aborts on one that does not parse (noskip.rs:471, 503), and an `invalid` file
  never parses. Every corpus file ends with `\n`.
  - "Compiles", for a corpus file, means: the run recorded in
    `corpus/corpus-rustc-run.toml` (below) exited 0 on exactly the bytes whose
    FNV-1a 64 hash is pinned in `MANIFEST.toml`; "rustc rejects" means it exited ≠ 0. No test
    spawns `rustc`.
  - `corpus/rust/*.rs.txt` (set `valid`): ≥ 30 files; the sum of `lines`
    over the `valid` set is ≥ 4,000. Each compiles alone, std only, and `syn::parse_file`
    (syn 2.0.119, as locked) succeeds on it. Crate-level `#![allow(...)]` is permitted. Files
    are UTF-8 without BOM. One file uses CRLF line endings and one has non-ASCII identifiers
    and strings. Between them the files contain every construct in this checklist (AC-7
    checks it):
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
    - other: `unsafe` blocks and `unsafe fn`, `unsafe extern "C"` blocks (edition 2024
      rejects `extern` blocks without `unsafe`), raw identifiers `r#type`, `?`, ranges, slices
      and arrays, tuple structs, enums with discriminants, `impl` blocks for traits with
      generics, `dyn`, `where Self: Sized`.
  - `corpus/rust-gaps/*.rs.txt` (set `gap`): files that compile, on which `syn::parse_file`
    succeeds, but on which the pinned grammar reports errors. Each records a known grammar gap
    rather than hiding it. Example, VERIFIED in the spike: `unsafe extern "C" { safe fn x(); }`
    gives `has_error: true`.
  - `corpus/rust-invalid/*.rs.txt` (set `invalid`): ≥ 8 files with a **syntax** error
    (unbalanced delimiters, `fn (`, `let = ;`, a truncated item …). Each fails
    `syn::parse_file` (the parse-only oracle; rustc's exit status does not say why it failed)
    and rustc rejects it.
  - `corpus/MANIFEST.toml`: one `[[file]]` per file, with `path` (relative to `corpus/`),
    `set` (`valid` | `gap` | `invalid`), `bytes`, `lines` (the number of `\n` bytes),
    `fnv1a64` (16 lowercase hex digits of FNV-1a 64 over the file's raw bytes: offset basis
    `0xcbf29ce484222325`, prime `0x100000001b3`), `origin` (`written for kiln`, or `adapted
    from <repo path> at <commit>`), for an adapted file `license = "<SPDX>"` (compatible with
    `MIT OR Apache-2.0` only), and for `gap` a `gap = "<one sentence>"`. Nothing is downloaded.
  The pin is the manifest. A file whose bytes do not match its entry, or a file without an
  entry, fails the corpus test.
  - `corpus/corpus-rustc-run.toml`, written by the **test author** (director ruling R-079,
    under "Questions and rulings"): the recorded result of one compiler run, committed with the
    corpus as evidence. The test author copies the corpus files into a scratch directory
    outside the worktree and the repository, and runs, there, for each file, with a 60 s
    timeout per file:
    `rustc +1.97.0 --edition 2024 --crate-type lib --emit metadata --crate-name corpus
    --cap-lints allow --out-dir <scratch>/out <file>`
    (`--crate-name` is needed because `.rs.txt` is not a valid crate name; `--out-dir` keeps
    the `.rmeta` files out of the crate directory). No compiler output lands in the worktree.
    The scratch directory is removed afterwards. Layout:
    ```toml
    command = "rustc +1.97.0 --edition 2024 --crate-type lib --emit metadata --crate-name corpus --cap-lints allow --out-dir <scratch>/out <file>"
    toolchain = "rustc 1.97.0 (2d8144b78 2026-07-07)"  # `rustc +1.97.0 --version`, verbatim
    date = "<YYYY-MM-DD>"                                # UTC date of the run
    [[file]]
    path = "rust/generics.rs.txt"                        # as in MANIFEST.toml
    fnv1a64 = "<16 lowercase hex>"                       # of the bytes compiled
    exit = 0
    ```
    `command` is exactly the string above; `<scratch>` and `<file>` are written literally (no
    session or machine path goes into the record). The per-file `fnv1a64` ties the record to
    the exact bytes; after any change to a corpus file the test author re-runs and rewrites
    the record.
  - `corpus_is_pinned_and_compiles` checks, spawning nothing: (i) every file under `corpus/`
    except `MANIFEST.toml` and `corpus-rustc-run.toml` has exactly one entry, every entry has a file, and `bytes`, `lines`
    and `fnv1a64` match; no file ends in `.rs`; (ii) the counts and the line floor above;
    (iii) the run record lists exactly the manifest's paths, each with the manifest's
    `fnv1a64`, `command` equals the pinned string, `toolchain` starts with `rustc 1.97.0 `,
    `exit == 0` for every `valid` and `gap` file and `exit != 0` for every `invalid` one;
    (iv) `syn::parse_file` succeeds on every `valid` and `gap` file and fails on every
    `invalid` one (the spawn-free cross-check).
- (c) Order: n/a (pure functions).
- (d) Offsets are chars (Unicode scalar values), as in the control protocol. CRLF counts as two
  chars. Platform independent.
- (e) Threat model (dependency): the grammar is C code compiled into the binary. The reviewer
  checks the pinned versions, that no `build.rs` is added to the workspace, and that parsing a
  50 MB text and adversarial nesting (10,000 nested `(`) neither panics nor overflows the
  stack. `sexp`, `errors` and `node_count` also return without overflowing the stack on deep
  *valid* nesting: 10,000 nested balanced `(`…`)` in a fn body (R-067: tree-sitter parses it
  with 0 errors, depth about 10,000). Time is not bounded by this task.
- (f) Snapshots: none (the crate is empty today).
- (g) Worked examples (spike, tree-sitter 0.27.2 / rust 0.24.2; re-executed with 0.27.1 / 0.24.2
  in R-077, same results):
  `has_error` is false for `fn f(x: Option<i32>) { if let Some(a) = x && a > 1 { } }`, for
  `fn g() -> impl Sized + use<> { 0 }` and for `let Some(x) = Some(1) else { return };` in a
  fn. It is true for `fn a() { (`. The sexp of `fn f() {}` is above; the MISSING forms are in
  AC-5.
- (h) Scorecard: none (T-020 and T-023 claim the items through commands).

## Acceptance criteria
In every AC, every call of `parse` and `update` returns `Ok`.
- AC-1 (compiles ⇒ no error nodes, pinned corpus): `corpus_is_pinned_and_compiles` proves the
  corpus first, by checks (i)–(iv) of Pinned (b); no test spawns `rustc`. Then every `valid`
  file gives `has_error() == false` and `errors() == []`, and every `gap` file gives
  `has_error() == true`. Oracle: the recorded rustc run, tied to the bytes by the pinned hash
  and cross-checked by `syn`; then the parser.
- AC-2 (compiles ⇒ no error nodes, live corpus): every `.rs` file under `crates/*/src`,
  `apps/*/src` and `xtask/src` of the repository at test time parses without errors. These
  files compile, because the gate builds them. Oracle: the gate's own compile. This is a
  permanent check on every later task: a later task whose valid Rust hits a grammar gap stops
  with `BLOCKED:`, and the director rules (rewrite the code, or a grammar update task).
- AC-3 (errors are found): every `invalid` file has at least one `ErrorSpan`, and every span
  lies within `0..=len_chars`. Oracle: exact property.
- AC-4 (incremental == from scratch): take every `valid` and `invalid` corpus file. Apply ≥ 200
  fixed-seed random edits per file: insert, delete or replace of 0–20 chars drawn from the
  file's own text, `{}();"'/\r\n` and `é😀`. After each edit `update(new)` gives
  `sexp() == SyntaxTree::parse(new)?.sexp()`, `errors()` equal and `node_count()` equal. The
  mode is `Unchanged` iff `new` equals the text before the edit (a zero-length edit, or a
  replace by the same text; about 3% of the draws, R-067). Otherwise it is `Incremental`
  whenever `SyntaxTree::parse(new)?.has_error()` is false; when the fresh tree has an error it
  may be `Incremental` or `Full`. (The interface rule — `Full` only when the incremental tree
  has an error — stands; a test can only see the fresh tree. R-067 found no edit on which the
  incremental tree had an error and the fresh one did not: 0 of 6,572.) Oracle: from-scratch
  parse.
- AC-5 (format): the sexp of the worked example is exact. For these three inputs (executed with
  0.27.1 / 0.24.2, R-077) the sexp contains exactly the `(MISSING …)` text shown; the test
  author may add more:
  - `fn f() { let x = 1 }` → `value:(integer_literal@17..18) (MISSING ";"@18..18)`;
  - `fn f() { 1 + }` → `right:(MISSING identifier@12..12)`;
  - `fn f() { let s = r#"abc"; }` → `value:(string_literal@23..24 (MISSING "\""@24..24))`.
  Oracle: hand-written expectation, checked against the grammar's own `to_sexp` by the test
  author.
- AC-6 (robustness): `parse` of `""`, `"\0"`, `"\r"`, 1 MB of `(`, 1 MB of random bytes decoded
  lossily, and a fn body with 10,000 nested balanced `(`…`)` returns `Ok`; `sexp`, `errors` and
  `node_count` return on each; offsets are in range. Oracle: property.
- AC-7 (corpus coverage): a `syn::visit` census over the `valid` set finds ≥ 1 of each of:
  `ExprLet` inside `&&` in an `if` condition, `Local` with `diverge` (let-else),
  `TypeParamBound::PreciseCapture`, `ExprAsync`, `ExprAwait`, `ExprClosure` with and without
  `move`, `Pat::Ident` with `subpat`, nested `Pat::Or`, `ExprBreak` with label and value,
  `Lit::CStr`, `Lit::ByteStr`, `Lit::Byte`, `Lit::Char`, a suffixed `LitInt`/`LitFloat`,
  `ItemMacro` with `macro_rules`, `ItemForeignMod`, `ItemFn` with `unsafe`, `ExprUnsafe`,
  `ExprTry`, `ExprRange`, a raw `Ident`, `Fields::Unnamed`, an enum discriminant,
  `TypeTraitObject`, `TypeImplTrait`, a const generic and an associated type/const. A text scan
  finds `///`, `//!`, `/**`, a nested `/* /* */ */` and a `\r\n`. Oracle: syn + text scan.

## Scorecard items and intended test names
None. Tests: `crates/kiln-syntax/tests/acceptance_t016.rs` (`corpus_is_pinned_and_compiles`,
`compiling_corpus_parses_without_errors`, `workspace_sources_parse_without_errors`,
`invalid_corpus_reports_errors`, `incremental_update_equals_fresh_parse`, `sexp_form_is_exact`,
`hostile_inputs_parse`, `corpus_covers_checklist`). All of them go in this one file, not in a
separate `acceptance_t016_*.rs` or `guard_t016*.rs`: `corpus_is_pinned_and_compiles` and
`corpus_covers_checklist` use no T-016 API and need the `syn`/`toml` dev-dependencies that the
implementer adds, so in a file of their own they would either pass on base (and the join
refuses tests that pass on base) or not build there.

## Out of scope and implementer notes
- Highlight, brackets, outline (T-020, T-023); any command (T-020); `kiln-text` types (the API
  takes `&str`).
- Your tests: unit tests in `src/tree.rs` and `crates/kiln-syntax/tests/tree.rs`. You may write
  a small private corpus for them in the test files; do not read or touch
  `crates/kiln-syntax/tests/corpus/`.
- Determinism: tree-sitter parsing is deterministic. Set no timeout or cancellation flag.
- The crate doc of `crates/kiln-syntax/src/lib.rs` ("intentionally empty") is replaced;
  `missing_docs` is an error under the gate, so every public item has a doc comment.
- Not here: a C compiler is now needed to build `kiln-syntax`. The director adds it (gcc ≥ 13
  or clang) to the toolchain table of `docs/development.md`, and updates that file's "L1 empty"
  line, after T-013 merges (T-013 holds that file).

## Questions and rulings
- Director ruling R-079 (replaces the R-077 ruling that the implementer makes the recorded
  compiler run, which METHODOLOGY §6 "Independence" makes impossible before the join): the
  **test author** owns the corpus and its evidence. It writes the corpus files (`.rs.txt`),
  records their content hashes in `MANIFEST.toml`, makes the one compiler run in a scratch
  directory outside the worktree, and commits the run record `corpus-rustc-run.toml`
  (command line, toolchain version, per-file exit status, date) on `task/T-016-tests` with
  the corpus. All of it lives in `crates/kiln-syntax/tests/corpus/` (moved from
  `crates/kiln-testkit/corpus/`), a `touches` path that only the test author writes. The
  implementer's code and unit tests must not depend on the corpus contents.

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
- 2026-11-13 · R-077 · draft · did: applied spec check R-067 (`tasks/spec-checks/T-016.md`)
  with the director's rulings. Blockers 1 (`.rs.txt` corpus names) and 2 (AC-4 `Unchanged`
  iff text unchanged). Ambiguities 3 (no test spawns rustc: hash pin + implementer's recorded
  run in `corpus-rustc-run.toml`, run in a scratch dir outside the worktree, `syn` cross-check
  in the gate; path added to `touches`), 4 (pinned `pub(crate)` accessors), 5 (`ParseError::NoTree`;
  `parse`/`update` now return `Result`), 7 (`=0.27.1`, rule stated), 8, 9, 10 (escaping; the
  `"\""` MISSING case pinned at `@24..24`), 11 (`toml` dev-dep), 12, 13 (AC-7), 14. Notes
  applied as corrections: 18 (approval wording), 21, 23 (as behaviour, AC-6), 24 (age rule is
  direct-only), 25 (`Cargo.lock` kept, overlap permitted by director), 26 (licence field,
  `unsafe extern`), 27, 28. Notes 6, 15, 19, 20, 22, 29 are observations; 16 and 17 are edits
  to T-020/T-023, outside this file. Re-executed in a scratch spike with tree-sitter 0.27.1 /
  rust 0.24.2: the `fn f() {}` sexp, all (g) `has_error` values and the three AC-5 MISSING
  forms; the rustc argv on a `.rs.txt` file (exit 0 valid, 1 `fn (`). Tier high (ADR-0002,
  adds a dependency). Pinned (c), (f), (h) n/a. · gate attempts: 0 · missing context: none ·
  next: director.
  QUESTION: the ruling names the implementer for the recorded rustc run, but METHODOLOGY §6
  "Independence" says neither builder reads the other's branch before the first join, and the
  corpus lives on `task/T-016-tests`. As written the implementer can only make the run after
  the first join, so the first join fails on the missing record. Keep this, or let the test
  author make the run (R-067 option A) and the reviewer re-run it?
  QUESTION: T-020 and T-023 need follow-up edits: 0.27.2 → 0.27.1 in T-020 (g); `parse` and
  `update` now return `Result`; R-067 notes 16 (T-020 AC-2 `unchanged`) and 17 (T-023 AC-5
  copies corpus files under `<stem>.rs`).
- 2026-11-13 · R-079 · draft · did: applied the director's ruling R-079 (recorded under
  "Questions and rulings"; answers my first R-077 question). The test author now makes the
  rustc run and commits `corpus-rustc-run.toml` with the corpus; `corpus_commit` dropped (the
  record is committed with the files it describes; per-file hashes tie it to the bytes). The
  corpus moved to `crates/kiln-syntax/tests/corpus/`, kept in `touches` because the queue's
  allowed-path rule (precheck.rs:319–332, 344–360) admits only `acceptance_tNNN*.rs` /
  `guard_tNNN*.rs` directly in `tests/`; the spec says only the test author writes there.
  All acceptance tests stay in `acceptance_t016.rs` (join base rule). T-020, T-021 and T-023
  updated for the new contracts (my second R-077 question). · gate attempts: 0 · missing
  context: none · next: director.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
