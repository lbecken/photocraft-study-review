---
id: T-015
title: "Lay the iteration-1 contracts before fan-out: one empty module per new command group, per-group session state, the external-tool trait, the new refusal reasons and the internal dependency edges"
tier: high              # ADR-0002: adds dependency edges (kiln-commands → kiln-syntax, kiln-search; kiln-workspace → kiln-search, serde, serde_json) and new protocol error kinds; no gate path
state: draft
owner_role: planner
scorecard_items: []     # enabling task; every later iteration-1 command task depends on it
depends_on: [T-030]     # T-030 brings RejectReason (and transitively T-006, T-007, T-008, T-010)
touches:
  - Cargo.toml                                   # [workspace.dependencies]: kiln-syntax, kiln-search path entries only
  - Cargo.lock                                   # generated
  - crates/kiln-commands/Cargo.toml              # + kiln-syntax, kiln-search
  - crates/kiln-commands/src/lib.rs              # mod lines + one register line per new group
  - crates/kiln-commands/src/registry.rs         # Context state slots, Parts, set_tools, new RejectReason variants, ToolError mapping
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
  - crates/kiln-commands/src/palette.rs          # new, empty group (palette/keymap task, not yet numbered)
  - crates/kiln-commands/src/keymap.rs           # new, empty group + KeymapState (same)
  - crates/kiln-commands/tests/scaffold.rs       # implementer's own integration tests
  - crates/kiln-workspace/Cargo.toml             # + kiln-search, serde, serde_json
  - crates/kiln-workspace/src/lib.rs             # pub mod lines + re-exports of tools
  - crates/kiln-workspace/src/tools.rs           # new: the ToolRunner contract and NoTools
  - crates/kiln-workspace/src/project.rs         # new, empty (T-019)
  - crates/kiln-workspace/src/find.rs            # new, empty (T-022)
  - crates/kiln-workspace/src/diagnostics.rs     # new, empty (T-024)
  - crates/kiln-workspace/src/format.rs          # new, empty (T-026)
  - crates/kiln-control/src/lib.rs               # ONLY the documented-kinds list, if T-030 left one
  - docs/control-protocol.md                     # error-kinds table rows only
  - docs/commands.json                           # generated; must come out unchanged
budget_tokens: 850000   # high (v1.1)
iteration: 1
found_by: planner
origin_task: null
---

# T-015 Lay the iteration-1 contracts before fan-out

## Goal
Iteration 1 fans out to up to three builders at once, and every feature adds commands. Today
`crates/kiln-commands/src/lib.rs` must gain a `mod` line and a register call for every new
command group. `registry.rs` holds `Context`, so any new session state means an edit there.
`docs/control-protocol.md` must gain a table row for every new refusal kind. And
`crates/kiln-workspace/src/lib.rs` must gain a `mod` line for every new module. Each of these
files would be edited by every feature task. That breaks the rule that in-flight tasks have
disjoint `touches` (METHODOLOGY §5.2), and it goes against CHARTER §3 decision 6.

After this task each later iteration-1 task owns its own new files and edits no shared file
(the table in "Ownership" below). Behaviour stays as it is. `kiln-cli commands`, every command's
result and every protocol response are byte-identical to `main` before the task. The
differences are: the documented error kinds gain eleven rows that no command produces yet; the
Rust API gains the tool contract and `Context::set_tools`; and the dependency graph gains five
edges. This is principle P1 ("contracts and checks exist before agents fan out").

Under the director's ruling for iteration-1 specs (R-056), each later task documents its
commands in its own `docs/protocol/<group>.md`. This task is the **only** iteration-1 task in
T-015…T-029 that adds error kinds: all eleven of them, at once, in the one kinds list of
`docs/control-protocol.md`. Later tasks therefore never touch that file, and do not conflict on
rebase there.

No gate path is touched: no `xtask/`, `.github/`, lint table or build key changes.

Reading list: CHARTER §3 (decisions 2, 4, 6); ADR-0002; `crates/kiln-commands/src/{lib,registry}.rs`
as left by T-007 and T-030; `crates/kiln-workspace/src/lib.rs`; `docs/control-protocol.md`
"Error codes and kinds"; `tasks/T-030-*.md` Interface (the reason mapping rule "no catch-all").

## Interface

### Ownership: the file each later task adds its commands to
Every module below is created by this task with exactly this body (plus a `//!` doc naming the
group and the task that fills it):
```rust
pub(crate) fn register(_registry: &mut Registry) -> Result<(), RegistryError> { Ok(()) }
```
`lib.rs` gets one `mod` line and one register call per module, in this order after
`doc::register`: cursor, eol, project, syntax, search, find, nav, tool, diag, edit, fmt, palette,
keymap. A later task adds `registry.register::<X>()?;` lines **inside its own module's
`register`** and never edits `lib.rs`.

| Module (`crates/kiln-commands/src/`) | Command prefix | Filled by | Session state struct (in that module) |
|---|---|---|---|
| `cursor.rs` | `cursor.*` | T-018 | — |
| `eol.rs` | `eol.*` | T-018 | — |
| `project.rs` | `project.*` | T-019 | — |
| `syntax.rs` | `syntax.*` | T-020 | `SyntaxState` |
| `search.rs` | `search.*` | T-021 | — |
| `find.rs` | `find.*` | T-022 | `FindState` |
| `nav.rs` | `nav.*` | T-023 | — |
| `tool.rs` | `tool.*` | T-024 | — |
| `diag.rs` | `diag.*` | T-024 | `DiagState` |
| `edit.rs` | `edit.*` | T-025 | — |
| `fmt.rs` | `fmt.*` | T-026 | — |
| `palette.rs` | `palette.*` | palette/keymap task (unnumbered) | — |
| `keymap.rs` | `keymap.*` | palette/keymap task | `KeymapState` |

Each state struct is created empty: `#[derive(Debug, Default)] pub(crate) struct XState {}`. The
owning task adds its fields inside its own module.

`crates/kiln-workspace/src/lib.rs` gets `pub mod project; pub mod find; pub mod tools; pub mod
diagnostics; pub mod format;` and `pub use tools::{NoTools, Tool, ToolError, ToolOutput,
ToolRequest, ToolRunner};`. `project.rs`, `find.rs`, `diagnostics.rs` and `format.rs` contain
only a `//!` doc. They are filled by T-019, T-022, T-024 and T-026, which add `pub` items
inside them and reach them as `kiln_workspace::project::…`, and so on.

### Session state (`crates/kiln-commands/src/registry.rs`)
```rust
impl Context {
    /// NEW. Replace the external-tool runner. Every constructor starts with `NoTools`, so a
    /// context never spawns a process unless its owner installs a runner.
    pub fn set_tools(&mut self, tools: Box<dyn kiln_workspace::ToolRunner>);
    /// NEW. Every part of the context, borrowed mutably at once.
    pub(crate) fn parts(&mut self) -> Parts<'_>;
}

/// NEW (crate-private). Disjoint mutable borrows of everything a command may need.
pub(crate) struct Parts<'a> {
    pub(crate) buffer: &'a mut kiln_text::Buffer,      // the scratch buffer
    pub(crate) vfs: &'a mut dyn kiln_workspace::Vfs,
    pub(crate) docs: &'a mut Docs,
    pub(crate) tools: &'a mut dyn kiln_workspace::ToolRunner,
    pub(crate) syntax: &'a mut crate::syntax::SyntaxState,
    pub(crate) find: &'a mut crate::find::FindState,
    pub(crate) diag: &'a mut crate::diag::DiagState,
    pub(crate) keymap: &'a mut crate::keymap::KeymapState,
}

/// NEW (crate-private). The open documents and the id counter (ids 1, 2, 3, …, never reused).
pub(crate) struct Docs { /* private */ }
impl Docs {
    pub(crate) fn get(&self, id: u64) -> Result<&Document, CommandError>;        // no_such_document
    pub(crate) fn get_mut(&mut self, id: u64) -> Result<&mut Document, CommandError>;
    pub(crate) fn add(&mut self, doc: Document) -> u64;
    pub(crate) fn remove(&mut self, id: u64) -> Result<Document, CommandError>;
    pub(crate) fn id_of(&self, path: &RelPath) -> Option<u64>;
    pub(crate) fn iter(&self) -> impl Iterator<Item = (u64, &Document)>;          // increasing id
}
```
The existing `Context` helpers (`doc`, `doc_mut`, `doc_and_vfs_mut`, `add_doc`, `remove_doc`,
`docs`) keep their signatures and behaviour; `doc.rs` and `buffer.rs` are not edited. The
refusal produced by `Docs::get` is the same `CommandError` (reason, message) as today's
`Context::doc`.

### The external-tool contract (`crates/kiln-workspace/src/tools.rs`, CHARTER §3 decision 4)
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tool { Cargo, Rustfmt }
impl Tool { pub fn as_str(self) -> &'static str; }          // "cargo", "rustfmt"

/// One invocation. The working directory is always the project root; the runner decides
/// where that is. `stdin: None` means no input (stdin closed).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ToolRequest { pub tool: Tool, pub args: Vec<String>, pub stdin: Option<String> }

/// What the tool printed and how it exited. `status: None` = killed by a signal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolOutput { pub status: Option<i32>, pub stdout: String, pub stderr: String }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolError {
    /// No runner is configured (the `NoTools` runner).
    Unavailable,
    /// The tool's executable was not found.
    NotFound { tool: Tool },
    /// The tool ran longer than its limit and was killed.
    Timeout { tool: Tool, seconds: u64 },
    /// Any other failure to run it or to read its output (e.g. output that is not UTF-8).
    Failed { tool: Tool, message: String },
    /// A replaying fake holds no recorded output for this request.
    NoRecording { tool: Tool },
}

pub trait ToolRunner: std::fmt::Debug {
    fn run(&mut self, request: &ToolRequest) -> Result<ToolOutput, ToolError>;
}

/// The default runner: every request fails with `ToolError::Unavailable`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoTools;
```
`Display` of `ToolError`, exactly: `no external tool runner is configured`; `` `<tool>` was not
found ``; `` `<tool>` timed out after <seconds> s ``; `` `<tool>` failed: <message> ``;
`` no recorded output for this `<tool>` request ``. No message contains a host path.
`ToolRunner` has no `Send` bound; implementations are added by T-024.

### Refusal reasons (`RejectReason`, from T-030)
The new variants, with their protocol kinds (`as_str()` and serde form) and where they will come
from:

| Variant | Kind | Raised by (task) |
|---|---|---|
| `InvalidPattern` | `invalid_pattern` | search query empty, or regex does not compile (T-021, T-022) |
| `StalePreview` | `stale_preview` | a replace preview's file changed since the preview (T-022) |
| `NoSuchPreview` | `no_such_preview` | unknown or already applied preview id (T-022) |
| `ToolUnavailable` | `tool_unavailable` | `ToolError::Unavailable` |
| `ToolNotFound` | `tool_not_found` | `ToolError::NotFound` |
| `ToolTimeout` | `tool_timeout` | `ToolError::Timeout` |
| `ToolFailed` | `tool_failed` | `ToolError::Failed`; a tool that exits non-zero without a usable result (T-024) |
| `NoRecording` | `no_recording` | `ToolError::NoRecording` |
| `FormatError` | `format_error` | rustfmt refused the input (T-026) |
| `ChangedOnDisk` | `changed_on_disk` | save over a file changed or removed since it was read (T-027) |
| `InvalidKeymap` | `invalid_keymap` | a user keymap file that fails validation (palette/keymap task) |

This task also adds `pub(crate) fn tool_reason(e: &ToolError) -> RejectReason` in `registry.rs`,
next to T-030's mappings, as an exhaustive `match` with no catch-all (T-030 rule). The mapping
is the table above.

`docs/control-protocol.md`: eleven `rejected` rows are added to the error-kinds table. Their
"When" cells are taken from the column above, in user terms and without task ids. Every other
row and sentence is unchanged. If `kiln-control` still holds a hand-written list of documented
pairs after T-030, the eleven pairs are added to it and nothing else in that file changes.

### Dependency edges
- Root `Cargo.toml` `[workspace.dependencies]`: `kiln-syntax = { path = "crates/kiln-syntax" }`,
  `kiln-search = { path = "crates/kiln-search" }`. No other change to the root manifest.
- `crates/kiln-commands/Cargo.toml` `[dependencies]`: `kiln-syntax.workspace = true`,
  `kiln-search.workspace = true`.
- `crates/kiln-workspace/Cargo.toml` `[dependencies]`: `kiln-search.workspace = true`,
  `serde.workspace = true`, `serde_json.workspace = true`. These are used by T-022 (find-in-files)
  and T-024 (tool recordings, `cargo` JSON). No registry package changes version in `Cargo.lock`:
  serde and serde_json are already locked.

## Pinned
- (a) Errors: no command's behaviour or refusal changes. The new reasons are produced by no
  command in this task. `ToolError` → reason: the table above, exhaustive. Display texts: exact
  as given.
- (b) Files read or written: none at run time. `docs/commands.json` must be byte-identical
  after `cargo xtask schema --write`.
- (c) Order of checks: n/a. No new runtime logic beyond the `NoTools` refusal.
- (d) Paths, platform, file types: n/a. No path handling.
- (e) Threat model: `NoTools` is the default for every `Context` constructor. So no library user
  (tests, `Host::in_memory`, `Host::on_disk`, `kiln-cli`) can spawn a process through a command
  before T-024 installs a runner on purpose. The reviewer checks that no other `ToolRunner`
  implementation exists in non-test code after this task.
- (f) Snapshots this changes: the T-030 test that compares the documented kinds with the
  `RejectReason` values (`documented_kinds_equal_reasons`) has to keep passing. That works
  because the table and the enum change together. No other snapshot is affected:
  `schema_lists_every_command` and every `acceptance_t003*`/`t004*`/`t007*`/`t008*`/`t010*`/`t030*`
  test pass unchanged.
- (g) Worked example (executed on `main` d3978dbf as the "before" model):
  `kiln-cli commands | sha256sum` is equal before and after. `kiln-cli exec doc.open
  '{"path":"x"}' --memory` prints the same `rejected`/`not_found` object before and after (exit 2).
- (h) Scorecard: none.

## Acceptance criteria
- AC-1 (no behaviour change, CLI): for the scripts of every `kiln-cli` acceptance test and the
  worked example of `docs/control-protocol.md`, responses are byte-identical before and after.
  `kiln-cli commands --schema` output equals `main`'s. Oracle: differential against the base
  binary (the test builds base output from a recorded file committed with the tests).
- AC-2 (kinds documented = reasons): the `rejected` kinds in the protocol table are exactly the
  `as_str()` values of `RejectReason`, now including the eleven new kinds. Each new variant's
  serde form equals its `as_str()`. Oracle: exact value (parse the table).
- AC-3 (no process by default): `Context::new()`, `Context::with_files(…)` and
  `Context::with_vfs(…)` hold a runner whose `run` of any `ToolRequest` returns
  `Err(ToolError::Unavailable)`. Through `kiln_control::Host::in_memory` and `Host::on_disk`
  nothing can reach a runner yet, so no command spawns. `set_tools` replaces the runner; a test
  runner installed with it sees the exact `ToolRequest`s sent to it. Since no command uses
  tools yet, this is tested through `Context::set_tools` plus a crate-internal unit test of
  `parts().tools`. Oracle: exact value.
- AC-4 (`ToolError` text and mapping): each variant's `Display` equals the pinned text, and
  `tool_reason` maps each variant to the pinned reason (unit test; the reviewer checks that
  the match has no catch-all). Oracle: exact value.
- AC-5 (layers): `cargo xtask layers` passes with the five new edges; `kiln-syntax` and
  `kiln-search` still depend on no workspace crate. Oracle: the gate step.
- AC-6 (ownership): each module in the Ownership table exists, is registered from `lib.rs` in
  the pinned order, and registers no command (`Registry::names()` is unchanged). Oracle: exact
  value.

## Scorecard items and intended test names
None. Tests: `crates/kiln-commands/tests/acceptance_t015.rs` (`no_command_or_response_changes`,
`documented_kinds_include_iteration1_reasons`, `contexts_start_without_tools`,
`tool_errors_display_and_map`) and `apps/kiln-cli/tests/acceptance_t015.rs`
(`cli_output_is_byte_identical_to_base`).

## Out of scope and implementer notes
- Any command, any `ToolRunner` that spawns or replays (T-024), any `list_dir` (T-019), any
  change to `doc.rs`, `buffer.rs`, `kiln-text`, `kiln-syntax`, `kiln-search` sources.
- Do not put "future" fields into the state structs. Each owning task designs its own.
- Determinism: nothing here reads a clock or the environment.
- `docs/development.md` "Adding a command" will say "add it to your group's module". The
  director updates that as a docs follow-up, because the file is not in `touches`.

## Questions and rulings

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13 · R-056 · (new) · did: read `kiln-commands/src` (two group modules; `lib.rs`
  registers each; `Context` and its private fields live in `registry.rs`) and
  `kiln-workspace/src/lib.rs` (private mods plus re-exports). Every iteration-1 feature would
  have edited `lib.rs`, `registry.rs`, `docs/control-protocol.md` and
  `kiln-workspace/src/lib.rs`, so I planned this enabling task first. Tier high, by
  ADR-0002's "adding or changing a dependency (workspace-internal edges included)". It also
  widens the documented error contract that T-030 made reason-based. Depends on T-030 because
  `RejectReason` must exist; T-030 itself runs after T-007, T-008 and T-010. Pinned (b)–(d)
  are n/a: there is no runtime logic. · gate attempts: 0 · missing context: T-030's final
  location of the reason mappings (assumed `registry.rs`) · next: director.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
