---
id: T-015
title: "Lay the iteration-1 contracts before fan-out: a structured reason on every refusal, the error-kind list generated from code, one empty module per new command group, per-group session state, the external-tool trait and the internal dependency edges"
tier: high              # ADR-0002: changes CommandError, a CHARTER §3 contract (ADR-0004 required); gate path xtask/generated.toml (approval row must contain `gate paths`); dependency edges
state: draft
owner_role: planner
scorecard_items: []     # enabling task; T-007, T-008, T-010, T-030 and every feature task depend on it
depends_on: []          # dispatch after T-006 merges: both touch doc.rs and registry.rs
touches:
  - Cargo.toml                                   # [workspace.dependencies]: kiln-syntax, kiln-search path entries only
  - Cargo.lock                                   # generated
  - xtask/generated.toml                         # GATE PATH: one [[generated]] entry for docs/protocol/error-kinds.md
  - crates/kiln-commands/Cargo.toml              # + kiln-syntax, kiln-search
  - crates/kiln-commands/src/lib.rs              # mod lines, one register line per new group, re-exports of reasons
  - crates/kiln-commands/src/registry.rs         # CommandError with reasons; Context slots, Parts, Docs, set_tools
  - crates/kiln-commands/src/reason.rs           # new: RejectReason, InternalReason, descriptions, exhaustive lower-layer mappings
  - crates/kiln-commands/src/doc.rs              # a reason at every refusal site (messages unchanged)
  - crates/kiln-commands/src/buffer.rs           # a reason at every refusal site (messages unchanged)
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
  - crates/kiln-control/src/lib.rs               # kinds from reasons only; message classification removed
  - crates/kiln-control/src/bin/kiln-error-kinds.rs   # new: renders the kind list
  - crates/kiln-control/tests/error_kinds.rs     # implementer's own tests, incl. the currency test
  - crates/kiln-workspace/Cargo.toml             # + kiln-search, serde, serde_json
  - crates/kiln-workspace/src/lib.rs             # pub mod lines + re-exports of tools
  - crates/kiln-workspace/src/tools.rs           # new: the ToolRunner contract and NoTools
  - crates/kiln-workspace/src/project.rs         # new, empty (T-019)
  - crates/kiln-workspace/src/find.rs            # new, empty (T-022)
  - crates/kiln-workspace/src/diagnostics.rs     # new, empty (T-024)
  - crates/kiln-workspace/src/format.rs          # new, empty (T-026)
  - docs/protocol/error-kinds.md                 # new, generated
  - docs/control-protocol.md                     # kinds table → pointer; kind-derivation sentence
  - docs/commands.json                           # generated; must come out unchanged
budget_tokens: 850000   # high (v1.1)
iteration: 1
found_by: planner
origin_task: null
---

# T-015 Lay the iteration-1 contracts before fan-out

## Goal
Claim C1 says to build contracts and checks first and fan agents out second (principle P1).
Iteration 1 fans out to up to three builders, and every feature adds commands and refusals.
Three things stand in the way today. First, `CommandError::Rejected` carries only a message,
so `kiln-control` derives the protocol `kind` by matching message text, and a reworded message
silently becomes `other`. Second, the kinds are a hand-written table in `docs/control-protocol.md`
that every task adding a kind must edit. Third, `crates/kiln-commands/src/{lib,registry}.rs`
and `crates/kiln-workspace/src/lib.rs` would be edited by every feature task. All of these break
disjoint `touches` (METHODOLOGY §5.2) and CHARTER §3 decision 6.

After this task:
- (a) every refusal and internal error carries a structured reason set where it is raised, and
  `kiln-control` reports `kind = reason.as_str()` without reading any message (part (d) of the
  former T-030, moved here by the director, R-060);
- (b) the list of `(code, kind, when)` pairs is **generated from code** into
  `docs/protocol/error-kinds.md`, and a gate test fails when it is stale. A task that adds a
  kind adds a reason variant and regenerates the file; nobody edits a list by hand;
- (c) each later iteration-1 task owns its own new files ("Ownership");
- (d) the external-tool contract exists with a default that never spawns;
- (e) the dependency graph gains five edges.

Every protocol response, `kiln-cli exec` error object and command result stays byte-identical.

**Gate path:** `xtask/generated.toml` (one entry). The approval row must contain `gate paths`.
**CHARTER §3 contract change** (the registry's error interface, decision 2): ADR-0004
(`docs/adr/0004-structured-refusal-reasons.md`, drafted in R-060, status proposed) must be
accepted before `ready`.

Reading list: CHARTER §3 (decisions 2, 4, 6); ADR-0002; draft ADR-0004;
`crates/kiln-commands/src/{lib,registry,doc,buffer}.rs` (every `CommandError::rejected` and
`Internal` site); `crates/kiln-control/src/lib.rs` (`Failure::from_command`, `rejected_kind`,
`ERROR_KINDS`, `PANICKED`); `docs/control-protocol.md` "Error codes and kinds";
`xtask/generated.toml`; `docs/development.md` "Generated-file conflicts".

## Interface

### Structured reasons (`kiln_commands::…`, re-exported from the crate root; defined in `reason.rs`)
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectReason {
    // today's kinds
    InvalidPath, NoSuchDocument, UnsavedChanges, NotFound, NotAFile, IoError, DecodeError,
    OutOfRange, InvertedRange, Other,
    // iteration-1 feature kinds (T-016…T-031)
    InvalidPattern, StalePreview, NoSuchPreview, ToolUnavailable, ToolNotFound, ToolTimeout,
    ToolFailed, NoRecording, FormatError, ChangedOnDisk, InvalidKeymap,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InternalReason { ResultSchema, Panic, Other }
impl RejectReason   { pub fn as_str(self) -> &'static str; pub fn description(self) -> &'static str; pub const ALL: &'static [RejectReason]; }
impl InternalReason { pub fn as_str(self) -> &'static str; pub fn description(self) -> &'static str; pub const ALL: &'static [InternalReason]; }

pub enum CommandError {                     // derives unchanged
    UnknownCommand { message: String },                          // unchanged
    InvalidParams { message: String, details: Vec<String> },     // unchanged
    Rejected { reason: RejectReason, message: String },          // CHANGED: + reason
    Internal { reason: InternalReason, message: String },        // CHANGED: + reason
}
impl CommandError {
    pub fn rejected(reason: RejectReason, message: impl Into<String>) -> Self;   // CHANGED: reason required
    /// `{"code":…,"message":…[,"details":…][,"reason":…]}`; `reason` only for rejected/internal.
    pub fn to_json(&self) -> serde_json::Value;
}
// Display of CommandError: unchanged. Patterns `Rejected { .. }` / `Internal { .. }` keep compiling.
```
`ALL` lists the variants in declaration order. `as_str()` equals the serde form. `description`
is the one-line "When" text of the generated list. For today's kinds it is the current table's
cell, word for word. For the new ones it is the "When" column below.

`T-008` adds `OutsideRoot` (`outside_root`) and `T-010` adds `NotABoundary`
(`not_a_boundary`), each in `reason.rs` with its mapping row. This task must not add them.

| New variant | Kind | When (description) | Raised by |
|---|---|---|---|
| `InvalidPattern` | `invalid_pattern` | the search pattern is empty or is not a valid regular expression | T-021, T-022, T-023 |
| `StalePreview` | `stale_preview` | a file of a replace preview changed since the preview | T-022 |
| `NoSuchPreview` | `no_such_preview` | no stored replace preview has this id | T-022 |
| `ToolUnavailable` | `tool_unavailable` | external tools are off for this session (the default) | T-024, T-026 |
| `ToolNotFound` | `tool_not_found` | the external tool's program was not found | T-024, T-026 |
| `ToolTimeout` | `tool_timeout` | the external tool ran longer than its limit and was stopped | T-024, T-026 |
| `ToolFailed` | `tool_failed` | the external tool failed or gave no usable result | T-024 |
| `NoRecording` | `no_recording` | the replayed tool recording has no output for this request | T-024, T-026 |
| `FormatError` | `format_error` | rustfmt could not format the text | T-026 |
| `ChangedOnDisk` | `changed_on_disk` | the file changed or was removed on disk since it was read | T-027 |
| `InvalidKeymap` | `invalid_keymap` | the keymap file is not valid | T-031 |

### Where reasons come from (the specification for today's refusals)
| Source | Reason |
|---|---|
| `RelPath::new` fails on a request path | `InvalidPath` |
| no open document has the id | `NoSuchDocument` |
| `doc.close` of a dirty document without `discard` | `UnsavedChanges` |
| `VfsError::NotFound` / `NotAFile` / `Io` | `NotFound` / `NotAFile` / `IoError` |
| `OpenError::Decode` | `DecodeError` |
| `EditError::OutOfBounds` / `InvertedRange` | `OutOfRange` / `InvertedRange` |
| `ToolError::Unavailable` / `NotFound` / `Timeout` / `Failed` / `NoRecording` | `ToolUnavailable` / `ToolNotFound` / `ToolTimeout` / `ToolFailed` / `NoRecording` |
| result fails its schema | `InternalReason::ResultSchema` |
| result does not serialize | `InternalReason::Other` |
| a panic caught by the `kiln-control` host | `InternalReason::Panic` |

The mappings live in `reason.rs` as functions over the lower layers' error enums (`VfsError`,
`OpenError`, `EditError`, `ToolError`). Each is an exhaustive `match` **without a catch-all arm**,
so a variant added later (T-008's `OutsideRoot`, T-010's position errors, T-027's `SaveError`)
does not compile until it is mapped there. `RejectReason::Other` is produced by no current
command.

### Control protocol and the generated list
- `kind` of a `rejected` / `internal` error = `reason.as_str()`. **No message text is consulted**
  for any `code` or `kind`. `rejected_kind` and the per-document path tracking used only for it
  are removed.
- `crates/kiln-control/src/bin/kiln-error-kinds.rs` renders the list. It contains the
  request-level pairs of `kiln-control` (`invalid_json`, `invalid_request`, `unknown_command`,
  `invalid_params`, in today's order, with today's "When" and "id" cells), then every
  `RejectReason::ALL`, then every `InternalReason::ALL`. Output: a Markdown file whose first
  line is `<!-- generated by kiln-error-kinds; do not edit -->`, followed by a table
  `| Code | Kind | When | id in the response |`. `--write <path>` writes it; no argument prints
  it.
- `xtask/generated.toml` gains, after the existing entries:
  `path = "docs/protocol/error-kinds.md"`, `command = ["cargo", "run", "-q", "--locked", "-p",
  "kiln-control", "--bin", "kiln-error-kinds", "--", "--write", "docs/protocol/error-kinds.md"]`.
- Currency: a `kiln-control` test (`error_kinds_doc_is_current`, run by the gate's `test`
  step) fails, naming the file, when the committed file differs from the rendering.
- `docs/control-protocol.md` "Error codes and kinds": the table is replaced by one sentence
  pointing to `docs/protocol/error-kinds.md`. The sentence "The `kind` of a `rejected` error is
  currently derived … from the message" becomes: kinds come from a structured reason set where
  the error is raised; the list is generated from code. Nothing else in the file changes.

### Ownership: the file each later task adds its commands to
Each module is created with this body plus a `//!` doc naming the group and its task:
```rust
pub(crate) fn register(_registry: &mut Registry) -> Result<(), RegistryError> { Ok(()) }
```
`lib.rs` gets one `mod` line and one register call per module, in this order after
`doc::register`: cursor, eol, project, syntax, search, find, nav, tool, diag, edit, fmt, palette,
keymap. A later task adds `registry.register::<X>()?;` lines only inside its own module.

| Module (`crates/kiln-commands/src/`) | Prefix | Filled by | State struct (in that module) |
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

State structs are created empty (`#[derive(Debug, Default)] pub(crate) struct XState {}`), and
the owning task adds the fields. `crates/kiln-workspace/src/lib.rs` gets `pub mod project; pub
mod find; pub mod tools; pub mod diagnostics; pub mod format;` and `pub use tools::{NoTools,
Tool, ToolError, ToolOutput, ToolRequest, ToolRunner};`. The four non-tool modules contain only
a `//!` doc.

### Session state (`registry.rs`)
```rust
impl Context {
    /// NEW. Replace the external-tool runner. Every constructor starts with `NoTools`.
    pub fn set_tools(&mut self, tools: Box<dyn kiln_workspace::ToolRunner>);
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
pub(crate) struct Docs { /* open documents + id counter: ids 1, 2, 3, …, never reused */ }
impl Docs {
    pub(crate) fn get(&self, id: u64) -> Result<&Document, CommandError>;     // NoSuchDocument
    pub(crate) fn get_mut(&mut self, id: u64) -> Result<&mut Document, CommandError>;
    pub(crate) fn add(&mut self, doc: Document) -> u64;
    pub(crate) fn remove(&mut self, id: u64) -> Result<Document, CommandError>;
    pub(crate) fn id_of(&self, path: &RelPath) -> Option<u64>;
    pub(crate) fn iter(&self) -> impl Iterator<Item = (u64, &Document)>;       // increasing id
}
```
The existing `Context` helpers keep their signatures and behaviour.

### The external-tool contract (`crates/kiln-workspace/src/tools.rs`, CHARTER §3 decision 4)
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tool { Cargo, Rustfmt }                    // as_str: "cargo", "rustfmt"
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ToolRequest { pub tool: Tool, pub args: Vec<String>, pub stdin: Option<String> }  // cwd = project root
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolOutput { pub status: Option<i32>, pub stdout: String, pub stderr: String }    // None = killed by a signal
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolError {
    Unavailable, NotFound { tool: Tool }, Timeout { tool: Tool, seconds: u64 },
    Failed { tool: Tool, message: String }, NoRecording { tool: Tool },
}
pub trait ToolRunner: std::fmt::Debug { fn run(&mut self, request: &ToolRequest) -> Result<ToolOutput, ToolError>; }
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoTools;                                 // every run → Err(Unavailable)
```
`Display` of `ToolError`, exactly:
- `external tools are off for this session (start with --tools real to run them)`;
- `` `<tool>` was not found ``;
- `` `<tool>` timed out after <seconds> s ``;
- `` `<tool>` failed: <message> ``;
- `` no recorded output for this `<tool>` request ``.

No message contains a host path.

### Dependency edges
- Root `[workspace.dependencies]`: `kiln-syntax` and `kiln-search` path entries, nothing else.
- `kiln-commands`: `kiln-syntax.workspace = true`, `kiln-search.workspace = true`.
- `kiln-workspace`: `kiln-search.workspace = true`, `serde.workspace = true`,
  `serde_json.workspace = true` (already locked; used by T-022 and T-024).

## Pinned
- (a) Errors: the reason tables are the specification. Messages and codes are unchanged, and
  precedence is unchanged in every command. `to_json` adds `"reason"` for `rejected`/`internal`
  only; the protocol error object does not carry `reason` (its `kind` already does).
- (b) Files: `docs/protocol/error-kinds.md` (generated; written only by the renderer);
  `docs/control-protocol.md` (table → pointer, one sentence); `xtask/generated.toml` (one
  entry). No runtime file access changes.
- (c) Order of checks: unchanged everywhere.
- (d) n/a: no path handling changes.
- (e) Threat model:
  - gate: the currency test cannot pass vacuously. It parses the committed file and fails
    for a missing file, an extra row, a changed cell, or a reason without a row;
  - integrity: no message text can change a `kind`; the reviewer checks that no mapping has
    a catch-all arm;
  - processes: `NoTools` is the default of every `Context`, so no command can spawn.
- (f) Snapshots:
  - `kiln-control` unit tests of `rejected_kind` go away with it. They are replaced by
    mapping tests, disclosed under golden rule 4;
  - `every_kind_is_documented` is replaced by the currency test plus "every pair the host can
    produce is in the file";
  - every `acceptance_t003*`, `acceptance_t004*` and guard test passes **unchanged**;
  - `kiln-cli commands --schema` output is unchanged.
- (g) Worked examples:
  - today on `main`, `"no open document 1"` maps to `no_such_document` by message
    (`rejected_kind`, `crates/kiln-control/src/lib.rs:357`). After this task,
    `command_error_json("doc.open", &json!({}), &CommandError::rejected(RejectReason::NotFound,
    "no open document 1"))` has `kind: "not_found"`;
  - `kiln-cli exec doc.open '{"path":"x"}' --memory` prints the same bytes before and after
    (exit 2).
- (h) Scorecard: none.

## Acceptance criteria
- AC-1 (reasons at their sources): through `standard_registry().execute`, each refusal row
  gives exactly its reason:
  - `doc.open "../x"`; `doc.text {"doc":9}`; a dirty `doc.close`;
  - `doc.open` of a missing file, of a directory (`MemoryFs` with `d/x`, open `d`), and of
    `b"\xff"`;
  - `doc.insert` at offset 99; `doc.delete` 2..1; `buffer.insert` at offset 99;
  - `doc.save` with `set_fail_writes(true)` → `IoError`.

  `to_json()["reason"] == as_str()`. Oracle: exact value.
- AC-2 (kinds from reasons only): for every `RejectReason` `r`, `command_error_json` of
  `rejected(r, m)` has `code: "rejected"` and `kind: r.as_str()`. This holds for `m` =
  `"zz unrelated"` and for the worked-example message. The same holds for every
  `InternalReason`, with `internal`. Oracle: exact value.
- AC-3 (protocol bytes unchanged): every T-003/T-004 acceptance test passes unmodified. The
  worked example of `docs/control-protocol.md` and a ≥ 300-request fixed-seed session through
  `kiln-cli run` give byte-identical output to the base binary (recorded output committed with
  the tests). Oracle: regression + differential.
- AC-4 (generated list): the committed `docs/protocol/error-kinds.md` equals the rendering.
  Its `rejected`/`internal` kinds are exactly `RejectReason::ALL`/`InternalReason::ALL`, each
  serde form equals `as_str()`, and every pair the host produced in the T-004 kind tests is
  present. With a variant added in a scratch copy, the currency test fails naming the file, and
  running the `generated.toml` command makes it pass. `docs/control-protocol.md` has no kind
  table. Oracle: exact value.
- AC-5 (no process by default): `Context::new()`, `with_files`, `with_vfs` and both `Host`
  constructors hold `NoTools`. `set_tools` replaces it, and a test runner sees the exact
  requests sent through `parts().tools` (unit test). Oracle: exact value.
- AC-6 (`ToolError` text and mapping): each `Display` is exact, and each variant maps to its
  reason. Oracle: exact value.
- AC-7 (layers and ownership): `cargo xtask layers` passes. Every module of the Ownership table
  exists, is registered in the pinned order, and `Registry::names()` is unchanged. Oracle: gate
  step + exact value.
- AC-8 (panic kind): a panic is still `internal`/`panic` with message `` command `<name>`
  panicked ``, now via `InternalReason::Panic`. The panic path itself is exercised by T-029's
  fuzz self-test; here the reviewer checks it by reading. Oracle: reading + T-029.

## Scorecard items and intended test names
None. `crates/kiln-commands/tests/acceptance_t015.rs` (`refusals_carry_reasons_at_their_sources`,
`internal_errors_carry_reasons`, `contexts_start_without_tools`, `tool_errors_display_and_map`),
`crates/kiln-control/tests/acceptance_t015_control.rs` (`kinds_follow_reasons_not_messages`,
`generated_kind_list_is_current_and_complete`), and `apps/kiln-cli/tests/acceptance_t015.rs`
(`cli_output_is_byte_identical_to_base`). Distinct target names are used because of the
join's per-name counting (T-009 (6)).

## Out of scope and implementer notes
- `outside_root` (T-008), `not_a_boundary` (T-010); the error-cap follow-ups (T-030); any
  command; any `ToolRunner` that spawns or replays (T-024); `list_dir` (T-019); `kiln-text`,
  `kiln-syntax`, `kiln-search` sources.
- `docs/development.md` ("Adding a command", the generated-files list) is not in `touches`
  because T-013 holds it. The director updates it as a docs follow-up.
- Determinism: nothing reads a clock or the environment.

## Questions and rulings
- R-060 (director): contracts first. This task absorbs T-030 part (d) and depends on nothing;
  T-007, T-008 and T-010 depend on it and add their kinds as reason variants.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13 · R-056 · (new) · did: planned this enabling task after reading
  `kiln-commands/src`: two group modules, `lib.rs` registers each, `Context` lives in
  `registry.rs`. Without it every feature would edit `lib.rs`, `registry.rs`, the kinds table
  and `kiln-workspace/src/lib.rs`. · gate attempts: 0 · next: director.
- 2026-11-13 · R-060 · draft · did: restructured per the director:
  - absorbed T-030 part (d) (reasons on `CommandError`, kinds from reasons, the generated list
    with a currency test and a `generated.toml` entry); the renderer is a `kiln-control`
    binary, so `apps/kiln-cli/src/main.rs` stays out and T-011 is unaffected;
  - removed `depends_on` T-030; `outside_root` and `not_a_boundary` are left to T-008 and
    T-010;
  - kept the 11 feature kinds;
  - changed the `NoTools` message to name the opt-in flag (T-024's `--tools real`);
  - drafted ADR-0004.

  Tier high: §3 contract, gate path, dependency edges. It touches `doc.rs` and `registry.rs`
  like T-006 (in flight), so dispatch after T-006 merges. `xtask/generated.toml` is in neither
  T-009's nor T-013's `touches`. · gate attempts: 0 · missing context: none · next: director,
  ADR-0004.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
