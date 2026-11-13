---
id: T-015
title: "Scaffold iteration 1 before fan-out: one empty module per new command group, per-group session state, the external-tool trait with spawning off by default, glob re-exports in kiln-workspace, and the internal dependency edges"
tier: high              # ADR-0002: dependency-table changes (workspace-internal edges + serde/serde_json in kiln-workspace); no gate path, no §3 contract
state: queued
owner_role: planner
scorecard_items: []     # enabling task
depends_on: []          # no touches held by T-013 (in flight) or any other in-flight task; dispatchable now
touches:
  - Cargo.toml                                   # [workspace.dependencies]: kiln-syntax, kiln-search path entries only (not a gate path)
  - Cargo.lock                                   # generated
  - crates/kiln-commands/Cargo.toml              # + kiln-syntax, kiln-search
  - crates/kiln-commands/src/lib.rs              # mod lines + one register call per new group
  - crates/kiln-commands/src/registry.rs         # Context: Docs, state slots, tools; Parts; set_tools, tools
  - crates/kiln-commands/src/cursor.rs           # new, empty group (T-018)
  - crates/kiln-commands/src/eol.rs              # new, empty group (T-018)
  - crates/kiln-commands/src/project.rs          # new, empty group (T-019)
  - crates/kiln-commands/src/syntax.rs           # new, empty group + SyntaxState (T-020)
  - crates/kiln-commands/src/search.rs           # new, empty group (T-021)
  - crates/kiln-commands/src/find.rs             # new, empty group + FindState (T-022)
  - crates/kiln-commands/src/nav.rs              # new, empty group (T-023)
  - crates/kiln-commands/src/tool.rs             # new, empty group (T-024)
  - crates/kiln-commands/src/diag.rs             # new, empty group + DiagState (T-024)
  - crates/kiln-commands/src/edit.rs             # new, empty group (T-025)
  - crates/kiln-commands/src/fmt.rs              # new, empty group (T-026)
  - crates/kiln-commands/src/palette.rs          # new, empty group (T-031)
  - crates/kiln-commands/src/keymap.rs           # new, empty group + KeymapState (T-031)
  - crates/kiln-commands/tests/scaffold.rs       # implementer's own integration tests
  - crates/kiln-workspace/Cargo.toml             # + kiln-search, serde, serde_json
  - crates/kiln-workspace/src/lib.rs             # pub mod lines; glob re-exports of vfs, document; tools re-exports
  - crates/kiln-workspace/src/tools.rs           # new: ToolRunner contract, NoTools
  - crates/kiln-workspace/src/project.rs         # new, empty (T-019)
  - crates/kiln-workspace/src/find.rs            # new, empty (T-022)
  - crates/kiln-workspace/src/diagnostics.rs     # new, empty (T-024)
  - crates/kiln-workspace/src/format.rs          # new, empty (T-026)
  - crates/kiln-workspace/tests/tools_contract.rs   # implementer's own integration tests
  # docs/commands.json is generated and must come out byte-identical
  - crates/kiln-commands/src/doc.rs               # D-046: only the unit test that assumes every non-listed command is mutating
budget_tokens: 850000   # high (v1.1)
iteration: 1
found_by: planner
origin_task: null
---

# T-015 Scaffold iteration 1 before fan-out

## Goal
Iteration 1 fans out to up to three builders, and every feature adds commands, session state
and lower-layer types. Today each feature would have to edit four shared files:
- `crates/kiln-commands/src/lib.rs`: a `mod` line and a register call per group;
- `registry.rs`: `Context` holds all session state;
- `crates/kiln-workspace/src/lib.rs`: private modules plus named re-exports, so T-019's
  `EntryKind`/`DirListing` and T-027's `SaveError`/`DiskState` would be unreachable;
- the error representation.

That breaks disjoint `touches` (METHODOLOGY §5.2) and CHARTER §3 decision 6. After this task
each later iteration-1 task adds its commands, state and types in files only it owns
("Ownership"). The default tool runner can never start a process (CHARTER §3 decision 4).
`Registry::names()`, every command result, every protocol response and `docs/commands.json`
are byte-identical to `main`.

Split by the director (D-041, from spec check R-062, finding 28). How refusals are represented,
and the list of error kinds, are task T-032 (ADR-0004). This task changes neither: no
`CommandError` change, no new error kind, no `kiln-control` change, no `xtask/` path, no ADR.
None of its `touches` is in the `touches` of T-013 (in flight), T-007, T-008, T-010 or T-011.

Reading list: CHARTER §3 (decisions 4, 6); ADR-0002; `crates/kiln-commands/src/{lib,registry}.rs`;
`crates/kiln-workspace/src/lib.rs`; `tasks/spec-checks/T-015.md` findings 6–8, 12–14, 19, 30, 31.

## Interface

### Ownership: the one line each later task adds
Each new module has exactly this body, plus a `//!` doc naming the group and the task that
fills it:
```rust
use crate::registry::{Registry, RegistryError};
pub(crate) fn register(_registry: &mut Registry) -> Result<(), RegistryError> { Ok(()) }
```
`lib.rs` gets one `mod` line and one `x::register(&mut registry)?;` call per module, in this
order after `doc::register`: cursor, eol, project, syntax, search, find, nav, tool, diag, edit,
fmt, palette, keymap. A later task adds `registry.register::<X>()?;` lines **inside its own
module's `register`**, and never edits `lib.rs` or `registry.rs`.

| Module (`crates/kiln-commands/src/`) | Prefix | Filled by | State struct (defined in that module) |
|---|---|---|---|
| `cursor.rs`, `eol.rs` | `cursor.*`, `eol.*` | T-018 | — |
| `project.rs` | `project.*` | T-019 | — |
| `syntax.rs` | `syntax.*` | T-020 | `SyntaxState` |
| `search.rs` | `search.*` | T-021 | — |
| `find.rs` | `find.*` | T-022 | `FindState` |
| `nav.rs` | `nav.*` | T-023 | — |
| `tool.rs`, `diag.rs` | `tool.*`, `diag.*` | T-024 | `DiagState` (in `diag.rs`) |
| `edit.rs` | `edit.*` | T-025 | — |
| `fmt.rs` | `fmt.*` | T-026 | — |
| `palette.rs`, `keymap.rs` | `palette.*`, `keymap.*` | T-031 | `KeymapState` (in `keymap.rs`) |

Each state struct is created as `#[derive(Debug, Default)] pub(crate) struct XState {}`, and
its owning task adds the fields. A feature task maps its own lower-layer errors to refusals
inside its own module (spec check, finding 12).

`crates/kiln-workspace/src/lib.rs`:
- `pub use vfs::{Vfs, VfsError};` becomes `pub use vfs::*;`, and `pub use document::{Document,
  OpenError};` becomes `pub use document::*;`. A later `pub` item in those modules (T-019's
  `EntryKind`, `DirEntry`, `DirListing`; T-027's `DiskState`, `SaveError`) is then exported
  without editing `lib.rs`. Today both modules have no other `pub` items, so nothing new is
  exported.
- It adds `pub mod project; pub mod find; pub mod tools; pub mod diagnostics; pub mod format;`
  and `pub use tools::{NoTools, Tool, ToolError, ToolOutput, ToolRequest, ToolRunner};`.
  `project.rs`, `find.rs`, `diagnostics.rs` and `format.rs` contain only a `//!` doc.

### Session state (`registry.rs`)
```rust
impl Context {
    /// NEW. Replace the external-tool runner. Every constructor starts with `NoTools`.
    pub fn set_tools(&mut self, tools: Box<dyn kiln_workspace::ToolRunner>);
    /// NEW. The current runner (read-only; `Debug` names its type, e.g. `NoTools`).
    pub fn tools(&self) -> &dyn kiln_workspace::ToolRunner;
    /// NEW, crate-private. Disjoint mutable borrows of every part.
    pub(crate) fn parts(&mut self) -> Parts<'_>;
}
pub(crate) struct Parts<'a> {
    pub(crate) buffer: &'a mut kiln_text::Buffer,
    pub(crate) vfs: &'a mut dyn kiln_workspace::Vfs,
    pub(crate) docs: &'a mut Docs,
    pub(crate) tools: &'a mut dyn kiln_workspace::ToolRunner,
    pub(crate) syntax: &'a mut crate::syntax::SyntaxState,
    pub(crate) find: &'a mut crate::find::FindState,
    pub(crate) diag: &'a mut crate::diag::DiagState,
    pub(crate) keymap: &'a mut crate::keymap::KeymapState,
}
/// Owns the open documents and the id counter (ids 1, 2, 3, …, never reused).
pub(crate) struct Docs { /* map + next id */ }
impl Docs {
    pub(crate) fn get(&self, id: u64) -> Result<&Document, CommandError>;     // today's `no open document {id}` refusal
    pub(crate) fn get_mut(&mut self, id: u64) -> Result<&mut Document, CommandError>;
    pub(crate) fn add(&mut self, doc: Document) -> u64;
    pub(crate) fn remove(&mut self, id: u64) -> Result<Document, CommandError>;
    pub(crate) fn id_of(&self, path: &RelPath) -> Option<u64>;
    pub(crate) fn map(&self) -> &BTreeMap<u64, Document>;
    pub(crate) fn iter(&self) -> impl Iterator<Item = (u64, &Document)>;       // increasing id
}
```
- `Context` replaces its fields `docs` and `next_doc` by one `docs: Docs`. The existing helpers
  keep their signatures and behaviour, and delegate: `doc` → `get`, `doc_mut` → `get_mut`,
  `add_doc` → `add`, `remove_doc` → `remove`, and `docs()` returns `Docs::map()`.
  `doc_and_vfs_mut` borrows `docs` and `vfs` disjointly. `doc.rs` and `buffer.rs` are not
  edited.
- **Dead code.** Under `clippy -D warnings`, `Parts`, `parts()`, and every `Docs` method and
  state-struct field reached only through them, carry exactly
  `#[allow(dead_code)] // slot for T-018…T-031 (T-015)`. It is `allow`, not `expect`, because
  the tasks that start using them cannot edit `registry.rs`. Nothing else may carry a lint
  attribute. The reviewer lists each one.

### The external-tool contract (`crates/kiln-workspace/src/tools.rs`)
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tool { Cargo, Rustfmt }
impl Tool { pub fn as_str(self) -> &'static str; }           // "cargo", "rustfmt"
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ToolRequest { pub tool: Tool, pub args: Vec<String>, pub stdin: Option<String> }  // cwd = project root
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolOutput { pub status: Option<i32>, pub stdout: String, pub stderr: String }    // None = killed by a signal
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolError {
    Unavailable, NotFound { tool: Tool }, Timeout { tool: Tool, seconds: u64 },
    Failed { tool: Tool, message: String }, NoRecording { tool: Tool },
}
impl std::fmt::Display for ToolError { … }
impl std::error::Error for ToolError {}
pub trait ToolRunner: std::fmt::Debug { fn run(&mut self, request: &ToolRequest) -> Result<ToolOutput, ToolError>; }
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoTools;                                          // every run → Err(ToolError::Unavailable)
```
`Display`, exactly:
- `external tools are off for this session (start with --tools real to run them)`;
- `` `<tool>` was not found ``;
- `` `<tool>` timed out after <seconds> s ``;
- `` `<tool>` failed: <message> ``;
- `` no recorded output for this `<tool>` request ``.

`<tool>` is `as_str()`. No message contains a host path. There is no serde on these types;
T-024 serialises recordings itself. `NoTools` is the only `ToolRunner` in non-test code after
this task. The mapping of `ToolError` to refusal kinds is T-032's.

### Dependency edges
- Root `[workspace.dependencies]`: `kiln-syntax = { path = "crates/kiln-syntax" }`,
  `kiln-search = { path = "crates/kiln-search" }`. No other change to the root manifest.
- `kiln-commands` `[dependencies]`: `kiln-syntax.workspace = true`, `kiln-search.workspace = true`.
- `kiln-workspace` `[dependencies]`: `kiln-search.workspace = true`, `serde.workspace = true`,
  `serde_json.workspace = true`. These are already locked, so no registry package changes
  version.

## Pinned
- (a) Errors: no command's refusal changes. `ToolError` texts are as above.
- (b) Files: none at run time. `docs/commands.json` is byte-identical after `cargo xtask schema --write`.
- (c) n/a: no new runtime logic beyond the `NoTools` refusal.
- (d) n/a: no path handling.
- (e) Threat model: no process can be started through any `Context`, `Host` or `kiln-cli`
  session, because every constructor installs `NoTools` and `Host` exposes no setter (T-024
  adds the opt-in). The reviewer checks that no other `ToolRunner` impl exists outside tests.
- (f) Snapshots: none change. Every existing acceptance and guard test passes unchanged,
  including `guard_t006.rs` (exact messages, which are kept) and `schema_lists_every_command`.
- (g) Worked example: `kiln-cli commands | sha256sum`, and `kiln-cli exec doc.open
  '{"path":"x"}' --memory` (stderr bytes, exit 2), are equal before and after.
- (h) Scorecard: none.

## Acceptance criteria
- AC-1 (no observable change, CLI; a **guard**, meant to pass on base):
  `apps/kiln-cli/tests/guard_t015_cli.rs` runs fixed scripts through `kiln-cli run` on a
  temporary root, and runs `kiln-cli exec`. The scripts are the worked example of `docs/control-protocol.md` with
  its files, plus a ≥ 300-request fixed-seed session whose request paths come from
  `[a-z0-9/._]`, never contain `: ` and never name a directory, and whose ids are at most
  64 characters. The outputs equal constants embedded in the `.rs` file, recorded on base.
  `kiln-cli commands` output is not compared (AC-5 and the `schema` step cover it). Oracle:
  differential against base output.
- AC-2 (no process by default, Rust API): `format!("{:?}", ctx.tools()) == "NoTools"` for
  `Context::new()`, `with_text`, `with_files`, `with_vfs`. After `set_tools(Box::new(R))`, `tools()`
  is `R`'s `Debug`. `NoTools.run(&req)` is `Err(ToolError::Unavailable)` for requests of both
  tools. For `kiln_control::Host`, the reviewer checks by reading that it builds its `Context`
  through these constructors and never calls `set_tools`. Oracle: exact value; reading for
  `Host`.
- AC-3 (`ToolError` and `Tool`): each `Display` equals the pinned text; `Tool::as_str` is
  exact; `ToolError` is a `std::error::Error`. Oracle: exact value.
- AC-4 (re-exports): `kiln_workspace::{Vfs, VfsError, Document, OpenError, RelPath, MemoryFs,
  DiskFs, Encoding, decode, encode, NoTools, Tool, ToolError, ToolOutput, ToolRequest,
  ToolRunner}` and `kiln_workspace::tools::NoTools` all resolve in an integration test.
  Oracle: compile + exact value.
- AC-5 (layers and ownership): `cargo xtask layers` passes with the five new edges, and
  `Registry::names()` is unchanged (exact value). The module list and order in `lib.rs`, and
  the lint attributes, are checked by the reviewer by reading (spec check, finding 31). Oracle:
  gate step + exact value + reading.

AC-2 to AC-4 fail on base only at compile time (weak evidence, METHODOLOGY §6). The reviewer's
mutation spot-check makes up for it.

## Scorecard items and intended test names
None. Distinct target names: `crates/kiln-commands/tests/acceptance_t015.rs`
(`contexts_start_without_tools`, `set_tools_replaces_the_runner`),
`crates/kiln-workspace/tests/acceptance_t015_tools.rs` (`no_tools_never_runs`,
`tool_error_texts_are_exact`, `workspace_reexports_resolve`) and
`apps/kiln-cli/tests/guard_t015_cli.rs` (`cli_output_is_byte_identical_to_base`).

## Out of scope and implementer notes
- Refusal reasons, kind derivation, error-kind list, `ToolError` → kind mapping: T-032. Any
  command (T-018…T-031). A spawning or replaying runner, and `Host::set_tools`: T-024.
  `list_dir`: T-019.
- Do not put future fields into the state structs; each owning task designs its own.
- `docs/development.md` "Adding a command" ("add it to your group's module") is a docs
  follow-up for the director: that file is T-013's.
- Your tests: `crates/kiln-commands/tests/scaffold.rs`, `crates/kiln-workspace/tests/tools_contract.rs`,
  unit tests (including that a test runner installed with `set_tools` sees the exact requests
  sent through `parts().tools`).

## Questions and rulings
- D-041 (director, R-066): split from the R-060 version. Part (d) of former T-030 and every
  error-kind change move to T-032.
- Director (2026-11-13, D-043), from spec check round 2: the AC-1 guard must not embed the output of `kiln-cli commands` or `commands --schema` — every later task that adds a command changes it and no later task may edit this guard — and its recorded session must avoid behaviour later tasks are specified to change (a path that names a directory, whose refusal T-008 rewords; request ids longer than 64 characters, whose cap T-030 changes). AC-1 above has been edited accordingly.
- Director (2026-11-13): AC-5's "`Registry::names()` is unchanged" is tested as "every command registered on base is still registered"; the exact list is the `schema` gate step's business (`docs/commands.json`). An exact-list test would break every later task that adds a command (the test author's question; same reasoning as D-043).
- Director rulings after review r1 (2026-11-13, D-046) — part of the spec:
  1. Guards must not stop later, already-specified tasks: `guard_t015.rs` matches `CommandError::Rejected { message, .. }` (T-032 adds a field); `guard_t015_cli.rs` uses a command name no planned task will ever register (`zz.no-such-command`) where it needs an unknown command, not `cursor.get` (T-018 adds it). Test author's change.
  2. Test gap: a test must show that the **default** runner of every standard context constructor starts no process — run a request through `ctx.tools()` of `Context::new()`, `with_text`, `with_files` and `with_vfs` with marker-writing fake `cargo`/`rustfmt` first on `PATH`, and assert no marker; checking the runner's `Debug` text alone is not enough. (Amended: this cannot be tested from outside the crate — `tools()` returns `&dyn` and running needs `&mut` — so it is an implementer's unit test in `registry.rs` through `parts().tools`.)
  3. `touches` gains `crates/kiln-commands/src/doc.rs` for one change: the unit test at about lines 1319–1337, which requires every command outside a fixed list to be mutating, is narrowed to the `doc.*` and `buffer.*` commands it was written for, so that a later group may register a read-only command. Implementer's change; nothing else in that file.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13 · R-056 · (new) · did: planned the enabling task. · gate attempts: 0 · next: director.
- 2026-11-13 · R-060 · draft · did: absorbed T-030 part (d) (superseded by R-066). · gate attempts: 0.
- 2026-11-13 · R-066 · draft · did: split per D-041. This task is the scaffold only and has no
  T-013 overlap (T-013 touches `xtask/src`, `xtask/tests/acceptance_t005_support.rs`,
  `docs/CHARTER.md`, `docs/development.md`, reviews). Resolved spec-check findings:
  - 6: glob re-exports;
  - 7: dead code, as `allow` with a fixed comment;
  - 12: features map their own errors in their own module;
  - 13: `Docs` owns map and counter, helpers delegate;
  - 14: `Tool::as_str`, `Display`, `Error`;
  - 15 and 19: AC-1 is the guard `guard_t015_cli.rs` with embedded constants, and the target
    names are distinct;
  - 30: `Context::tools()`;
  - 31: module order by reading.

  Tier high only for the dependency edges. Pinned (b)–(d) n/a: no runtime logic. · gate
  attempts: 0 · missing context: none · next: director.
<!-- end planner log -->
### implementer
- 2026-11-13 · R-072 · ready · ACs restated: (1) CLI/protocol output, `commands`,
  `commands --schema` and `docs/commands.json` unchanged from base (guard; D-043 narrows the guard
  only); (2) every `Context` constructor installs `NoTools`, `set_tools` replaces it, `NoTools`
  refuses both tools with `Unavailable`, `Host` gets no setter; (3) the five `ToolError` texts and
  `Tool::as_str` exact, `ToolError: Error`; (4) the listed names resolve at the `kiln_workspace`
  root and `tools::NoTools`; (5) `cargo xtask layers` passes with the five edges, `names()`
  unchanged, the 13 modules registered in the pinned order after `doc`. · did: added the 13 empty
  group modules and the 4 state structs, `Docs`, `Parts`, `parts()`, `set_tools`/`tools`; the
  `ToolRunner` contract with `NoTools`; glob re-exports and 5 `pub mod`s in kiln-workspace; the
  dependency edges. Checked base vs head by building kiln-cli at d995caa and at HEAD in this
  worktree: `commands | sha256sum`, `commands --schema | sha256sum`, `exec doc.open
  '{"path":"x"}' --memory` (stderr bytes, exit 2) identical. Readings chosen: (a) the group
  modules carry no `///` doc on `register` (body exactly as pinned); state structs carry a one-line
  `///` doc above the pinned derive; (b) `Docs` gets a private `fn new()` (the spec lists none;
  not `Default`, so the counter start of 1 is explicit); (c) `#[allow(dead_code)] // slot for
  T-018…T-031 (T-015)` sits on exactly 4 items: `parts()`, `Parts`, `Docs::id_of`,
  `Docs::iter`. The other `Docs` methods are reached through the existing helpers, and the
  Context state fields need none because `parts()` is allowed; I removed all four and confirmed
  each one is needed; (d) `Docs::id_of` scans the map (first match by increasing id).
  Unit tests in `registry.rs` (`scaffold_tests`), integration tests in `tests/scaffold.rs` and
  `tests/tools_contract.rs`.
  QUESTION: none blocking. For the record: the spec's `Docs` block shows no constructor, and the
  "every Docs method … reached only through them" rule leaves it open whether `get`/`get_mut`/
  `add`/`remove`/`map` need the attribute; I read "only through them" as "not otherwise
  used", so they do not.
- 2026-11-13 · R-072 · ready (D-043 notice read: it binds the AC-1 guard only; no code change) ·
  did: finished the scaffold above. `docs/commands.json` is unchanged against d995caa. The only
  non-test `ToolRunner` impl is `NoTools`, and `kiln-control` was not edited, so `Host` has no
  setter. · gate attempts: 3 (1 and 2: red clippy, both in my own tests: a unit-struct `default()`
  call and an `expect` outside `#[test]`; last: green, 10 steps, 394 tests) · not run: the
  control-protocol worked example and a long session against base (the test author's AC-1 guard
  covers these); acceptance tests (not visible to me) · missing context: none · next: join with
  the test author's branch, then review.
<!-- end implementer log -->
### test author
- 2026-11-13T06:48Z · R-073 · in_progress · did: tests from the spec (AC-1 per D-043) on
  base d995caa. Criterion → tests → result on base:
  - AC-1 → `apps/kiln-cli/tests/guard_t015_cli.rs` `cli_output_is_byte_identical_to_base`
    (worked example via `run` + `--log` and via `control`, files after; 9 `exec` cases incl.
    Pinned (g); a 320-request fixed-seed session via `run` (exit 3, stdout, `--log`, files) and
    via `control`, plus a replay of its log), `session_script_meets_the_d043_constraints` →
    **guard, passes on base**. Constants recorded on base; no `commands`/`--schema` output (D-043);
    no path names a directory; ids ≤ 64 chars.
  - AC-2 → `crates/kiln-commands/tests/acceptance_t015.rs` `contexts_start_without_tools` (new,
    default, with_text, with_files, with_vfs memory/disk; still `NoTools` after every existing
    command), `set_tools_replaces_the_runner` (Debug of R; replace again; back to NoTools;
    buffer/docs/files/log/next id untouched) → **compile**. `crates/kiln-workspace/tests/
    acceptance_t015_tools.rs` `no_tools_never_runs` (both tools, 8 requests, direct/dyn/Box;
    re-runs itself with fake `cargo`/`rustfmt` first on PATH, shown to leave a marker first,
    and checks no marker after) → **compile**.
  - AC-3 → `tool_error_texts_are_exact` (all five texts, both tools, 0/u64::MAX seconds; `Error`
    with no source; exhaustive matches; derives) → **compile**.
  - AC-4 → `workspace_reexports_resolve` (all 16 root names + `tools::*` same types; glob items
    still behave) → **compile**.
  - AC-5 → `crates/kiln-commands/tests/acceptance_t015_layers.rs`
    `the_five_new_dependency_edges_exist` (`cargo metadata --no-deps --offline`, presence only)
    → **assertion**. `guard_t015.rs` `base_commands_are_still_registered`,
    `document_ids_and_refusals_are_unchanged` (Docs delegation: ids, no reuse, exact
    `no open document {id}` for every doc command, dirty-close text) → **guard, pass on base**.
  Self-check: plausible scaffold in a scratch copy (tools.rs, globs, modules, edges, Context
  tools/set_tools; it kept today's BTreeMap fields, so `Docs`/`Parts` were not built and their
  bugs were seeded into the delegating helpers), lint-clean under `clippy --all-targets -D
  warnings`. 18 seeded bugs, 17 killed: B1 Unavailable text, B2 as_str swapped, B3 `120s`,
  B12 Failed drops message, B13 NoRecording text → tool_error_texts_are_exact; B4 NotFound for
  rustfmt, B6 NoTools spawns then refuses → no_tools_never_runs; B5 constructors install a
  spawning runner, B15 a wrapper runner → contexts_start_without_tools + set_tools_…; B7
  set_tools ignored, B8 set_tools resets the session → set_tools_…; B9 ids = len+1, B17 save
  refusal names id+1, B18 close gives back the newest id (first survived the Rust guard, killed
  by the CLI guard; the guard then got a case) → guard_t015 + guard_t015_cli; B11 no
  kiln-workspace→kiln-search, B16 no kiln-commands→kiln-syntax → layers test; B14 named instead
  of glob re-export → acceptance_t015_tools (compile). Survivor B10 (remove_doc refusal
  reworded) is unreachable: `doc.close` calls `doc()` before `remove_doc`, so no caller can see
  it (equivalent mutant). Not exercisable by any test: `Parts`/`Docs` (crate-private), module
  list/order in `lib.rs`, the `#[allow(dead_code)]` comments, that the four new
  `kiln-workspace` modules are item-less (an item-less module cannot be named from a test
  without an unused-import lint), that no other `ToolRunner` impl exists, `Host` never calling
  `set_tools` (no accessor): reviewer reading, as the spec says. A newly registered command is
  deliberately not caught by my tests (D-043); the `schema` step catches it at the join.
  · gate attempts: 1 (last: red clippy, compile errors only in acceptance_t015.rs /
  acceptance_t015_tools.rs, as expected on base; `layers`, `schema` and both guards pass when run
  alone) · not run: gate steps after clippy (gate stops at the first failure) · missing context:
  none · next: join.
  QUESTION: AC-5 says `Registry::names()` is unchanged (exact value). A committed exact-list test
  would break every later task that adds a command, as D-043 found for AC-1, so I committed only
  "every base command is still registered" (guard). Proposed wording: "`Registry::names()` at
  the join equals base, checked by the `schema` gate step (`docs/commands.json` byte-identical);
  the committed guard requires only that every base command stays registered."
- 2026-11-13 · R-076 · round 2 (join: all tests pass; gate red `no-skip`) · did:
  `no_tools_never_runs` no longer returns early: the child copy asserts the `NoTools` values
  and the parent then runs `parent_checks()`; off Unix it fails with a precondition message.
  Also removed the early `return` from the `same` helper in `guard_t015_cli.rs`. Re-checked
  against a stub: lint-clean, passes; B6 (NoTools spawns, then refuses) still killed. · gate
  attempts: `cargo xtask no-skip` green · missing context: none · next: join.
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
