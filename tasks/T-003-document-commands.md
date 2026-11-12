---
id: T-003
title: "Open, edit, undo, save and close documents through schema-validated commands with a replayable command log"
tier: high              # command schema; two layers' contracts meet here; Cargo.toml/Cargo.lock
state: in_progress
owner_role: director
scorecard_items: [ws.documents, ws.dirty-tracking, io.crlf, cmd.log-replay]
depends_on: [T-001, T-002]
touches:
  - crates/kiln-commands/src
  - crates/kiln-commands/Cargo.toml
  - Cargo.toml
  - Cargo.lock
  - docs/commands.json
  - apps/kiln-cli/src/main.rs   # D-020: only the test `schema_lists_every_command`
budget_tokens: 700000
iteration: 0
found_by: planner
origin_task: null
---

# T-003 Open, edit, undo, save and close documents through schema-validated commands with a replayable command log

## Goal
After this task every step of the iteration-0 slice — open a file, edit it, undo/redo, save,
close — and every query needed to observe the result is a named command in the registry, with
JSON-Schema-validated params and results and executable examples, and every successful command
is recorded in a command log that, replayed on the same initial state, reproduces the same
documents and files (CHARTER §3 decision 2).

## Reading list
- `docs/CHARTER.md` §3 decisions 2, 4, 5, 6.
- `crates/kiln-commands/src/{lib,registry,buffer}.rs`; `docs/commands.json`;
  `docs/development.md` "Adding a command".
- `tasks/T-001-text-core.md` Interface (Buffer, `LineEndings`, undo, grouping rule) and
  `tasks/T-002-documents-file-io.md` Interface (`Vfs`, `MemoryFs`, `RelPath`, `Encoding`,
  `Document`, dirty definition), as merged.

## Interface

### Rust (crate root `kiln_commands::…`)
```rust
pub struct Context { /* private */ }          // Debug + Default (Clone is dropped)
impl Context {
    pub fn new() -> Self;                      // empty MemoryFs, empty scratch buffer
    pub fn with_text(text: &str) -> Self;      // unchanged: scratch buffer = text, empty MemoryFs
    pub fn with_files(files: &[(&str, &[u8])]) -> Result<Self, kiln_workspace::PathError>; // MemoryFs
    pub fn with_vfs(vfs: Box<dyn kiln_workspace::Vfs>) -> Self;
    pub fn buffer(&self) -> &kiln_text::Buffer;   // unchanged (scratch buffer of buffer.*)
    pub fn vfs(&self) -> &dyn kiln_workspace::Vfs;
    pub fn log(&self) -> &[LogEntry];
}

/// One successfully executed command, in execution order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogEntry { pub name: String, pub params: serde_json::Value, pub mutating: bool }

pub trait Command {
    // unchanged: NAME, DOC, Params, Output, examples, run
    /// NEW: true iff the command can change state (documents, buffer, files). Read-only queries
    /// (`doc.text`, `doc.info`, `doc.list`, `buffer.text`) are false.
    const MUTATING: bool;
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReplayError { pub index: usize, pub error: CommandError }   // + Display, Error

impl Registry {
    // unchanged: new, register, names, doc, execute, describe, audit
    /// Execute every entry in order on `ctx`; stop at the first error.
    pub fn replay(&self, ctx: &mut Context, log: &[LogEntry]) -> Result<Vec<serde_json::Value>, ReplayError>;
}

pub struct Example {
    pub description: &'static str,
    pub initial_text: &'static str,                 // unchanged
    pub files: Vec<(&'static str, &'static str)>,   // NEW: initial MemoryFs (path, UTF-8 content)
    pub setup: Vec<(&'static str, serde_json::Value)>, // NEW: commands run first; each must succeed
    pub params: serde_json::Value,
    pub result: serde_json::Value,
}
```
`Registry::execute` appends `LogEntry{name, params, mutating}` to the context's log after every
**successful** command, read-only ones included, with `mutating` = the command's `MUTATING`
(D-2); a failed command appends nothing and leaves the context unchanged. `describe()` and
`docs/commands.json` gain a boolean `"mutating"` per command. `audit` runs each example on `Context::with_text(initial_text)` whose
MemoryFs holds `files`, executes `setup` (a failing setup command is an audit problem), then the
example. `docs/commands.json` examples gain the keys `files` and `setup` (empty for `buffer.*`).

### Commands (group `doc`)
Documents are identified by `doc`: a `u64` id assigned 1, 2, 3, … per `Context` (session) in
increasing order as documents are opened, never reused (D-1). Offsets are char indices. Paths are `RelPath` strings.
Domain failures return error code `rejected` (unknown doc, bad path, file missing, decode error,
out-of-range offsets, dirty close); schema violations return `invalid_params`.
All params objects reject unknown fields. `line_ending` ∈ `"none"|"lf"|"crlf"|"cr"`: the
dominant line ending of the current text (kiln-text `dominant_line_ending()`, `"none"` if there
is no line break); `mixed_line_endings` is true iff kiln-text `line_endings()` is `Mixed`; `encoding` ∈ `"utf-8"|"utf-8-bom"|"utf-16le"|"utf-16be"`.

| Command | Params | Result |
|---|---|---|
| `doc.open` | `{"path": string}` | `{"doc": int, "already_open": bool, "path": string, "encoding": string, "line_ending": string, "mixed_line_endings": bool, "len_chars": int}` |
| `doc.save` | `{"doc": int}` | `{"doc": int, "bytes_written": int, "dirty": false}` |
| `doc.close` | `{"doc": int, "discard"?: bool (default false)}` | `{"doc": int, "closed": true}` |
| `doc.insert` | `{"doc": int, "offset": int, "text": string}` | `{"len_chars": int, "dirty": bool}` |
| `doc.delete` | `{"doc": int, "start": int, "end": int}` | `{"removed": string, "len_chars": int, "dirty": bool}` |
| `doc.replace` | `{"doc": int, "start": int, "end": int, "text": string}` | `{"removed": string, "len_chars": int, "dirty": bool}` |
| `doc.undo` | `{"doc": int}` | `{"undone": bool, "len_chars": int, "dirty": bool}` |
| `doc.redo` | `{"doc": int}` | `{"redone": bool, "len_chars": int, "dirty": bool}` |
| `doc.text` | `{"doc": int}` | `{"text": string, "len_chars": int}` |
| `doc.info` | `{"doc": int}` | `{"doc", "path", "encoding", "line_ending", "mixed_line_endings", "len_chars", "len_lines", "dirty", "can_undo", "can_redo"}` |
| `doc.list` | `{}` | `{"docs": [{"doc": int, "path": string, "dirty": bool}]}` sorted by `doc` |

Semantics: `doc.open` of a path that is already open is not an error: it returns that
document's existing id with `already_open: true` and its current state (D-3), changes nothing,
and is logged like any successful command; a first open returns `already_open: false`. `doc.close` of a dirty
document without `"discard": true` → `rejected`, document stays open. `doc.save` writes through
`Document::save` and also ends the current typing group (`break_undo_group`). Undo grouping is
the T-001 rule, applied across successive `doc.insert` commands. `buffer.insert` / `buffer.text`
are unchanged.

Typed example (`doc.open` on files `{"a.rs": "fn a() {}\r\n"}`):
`{"path":"a.rs"}` → `{"already_open":false,"doc":1,"encoding":"utf-8","len_chars":11,"line_ending":"crlf","mixed_line_endings":false,"path":"a.rs"}`.

## Acceptance criteria
All through `Registry::execute` on `standard_registry()` unless stated.
- AC-1 (slice): on `Context::with_files(&[("a.txt", b"hello\n")])`: `doc.open a.txt` → doc 1;
  `doc.insert {doc:1, offset:5, text:" world"}` → `{"len_chars":12,"dirty":true}`; `doc.save` →
  `{"doc":1,"bytes_written":12,"dirty":false}`; `ctx.vfs().read("a.txt")` == `b"hello world\n"`;
  `doc.info` shows `dirty:false`. Oracle: exact value.
- AC-2 (edits differential): a fixed-seed sequence of ≥ 500 `doc.insert`/`doc.delete`/
  `doc.replace`/`doc.undo`/`doc.redo` commands on one document gives, after each command,
  `doc.text` equal to a model that applies the same edits to a `kiln_text::Buffer` directly
  (results' `removed`, `len_chars`, `undone`/`redone` too). Rejected commands (out-of-range
  offsets) return `rejected` and leave `doc.text`/`doc.info` unchanged. Oracle: naive model
  (the L0 Buffer as reference).
- AC-3 (several documents, ws.documents): open `a.txt` and `b.txt`; edits, undo and save on doc 1
  change neither doc 2's text, `dirty`, `can_undo` nor `b.txt`; `doc.list` lists both with
  correct `dirty`; after `doc.close {doc:1}` (clean), `doc.list` lists only doc 2 and any command
  on doc 1 → `rejected`; re-opening `a.txt` yields doc 3 with `can_undo:false`. Oracle: exact value.
- AC-4 (dirty, ws.dirty-tracking): dirty is false after open, true after an edit, false after
  `doc.save`, false after undoing every edit since the last save, true after undoing past a save
  point (text now differs from the file). Oracle: model "text ≠ last saved text".
- AC-5 (line endings, io.crlf = detection + round-trip preservation): files `lf.txt = "a\nb\n"`,
  `crlf.txt = "a\r\nb\r\n"`, `cr.txt = "a\rb"` open with `line_ending` `"lf"` / `"crlf"` /
  `"cr"` and `mixed_line_endings:false`; `mix.txt = "a\r\nb\r\nc\n"` reports `"crlf"` with
  `mixed_line_endings:true`; `none.txt = "abc"` reports `"none"`. Saving each unedited writes the
  original bytes; inserting `"X"` at 1 and saving writes `"aX\r\nb\r\n"` for CRLF and keeps the
  mixed file's line breaks byte-for-byte; inserting `"\n"` into the CRLF file saves that `\n`
  as `\n` (T-002 D-2). Oracle: exact value + round-trip.
- AC-6 (errors and re-open): `doc.open` of a missing file, of an undecodable file (`b"\xff"`), or
  of an invalid path (`"../x"`) → `rejected`, and `doc.list` is unchanged; `doc.open` of an
  already open, edited `a.txt` returns its existing id with `already_open:true`, the edited
  `len_chars`, and leaves `doc.list`, `doc.text` and `can_undo` unchanged; `doc.close`
  of a dirty doc without discard → `rejected`, doc still listed; with `discard:true` → closed and
  the file is unchanged; `{"doc":1,"offset":0}` (missing `text`) and `{"doc":"1",…}` →
  `invalid_params`; unknown fields → `invalid_params`. Oracle: exact value.
- AC-7 (grouping through commands): `doc.insert` of `"a"`,`"b"`,`"c"` at 0,1,2 then one `doc.undo`
  restores the opened text; with a `doc.save` between `"b"` and `"c"`, one `doc.undo` leaves
  `"ab"` + original. Oracle: exact value.
- AC-8 (log, cmd.log-replay): for fixed-seed random command sequences (incl. failing ones) on a
  context with initial files F: `ctx.log()` lists exactly the successful commands in order with
  their params and `mutating` equal to each command's declared flag (`true` for `doc.insert`,
  `false` for `doc.text`, per the table in `describe()`); `replay` of that log on a fresh `Context::with_files(F)` succeeds, returns the
  same results in order, and leaves identical `doc.list`, `doc.text` of every open doc, VFS
  contents and `log()`. Replaying the replayed context's log again gives the same log
  (idempotence). Oracle: round-trip + idempotence.
- AC-9 (schema and examples): `kiln-cli commands --audit` passes; `describe()` lists all eleven
  `doc.*` commands with params/result schemas, a boolean `mutating`, and ≥ 1 example each; every example's `files` and
  `setup` are present in `docs/commands.json`; `cargo xtask schema` passes. Oracle: exact value
  (the audit runs each example and compares its exact result).
- AC-10 (compatibility): `buffer.insert` / `buffer.text` behave as before and the existing
  `kiln-cli` tests pass. Oracle: exact value.

## Scorecard items and intended test names
Tests in `crates/kiln-commands/tests/acceptance_t003.rs` (`kiln_commands::acceptance_t003::<fn>`).

| Item | Tests |
|---|---|
| ws.documents | `documents_have_independent_text_history_and_dirty_flag`, `close_and_reopen_gives_new_id_and_fresh_history` |
| ws.dirty-tracking | `dirty_after_edit_clean_after_save_and_after_full_undo`, `undo_past_save_point_is_dirty` |
| io.crlf | `lf_and_crlf_detected_on_open_and_kept_on_save`, `mixed_line_endings_reported` |
| cmd.log-replay | `log_replay_reproduces_documents_and_files`, `log_lists_successful_commands_with_mutating_flag`, `replay_of_replayed_log_is_identical` |

Also (no item): `open_edit_save_slice_exact_values`, `edit_commands_match_buffer_model`,
`doc_command_errors_are_rejected_or_invalid_params`, `reopening_open_path_returns_existing_id`, `typing_groups_across_insert_commands_and_save_breaks_group`.
`cmd.registry` / `cmd.schema` are already implemented; the test author may add
`doc_commands_are_described_and_audited` to `cmd.schema`.

## Out of scope
- Disk-backed contexts from the CLI, the control protocol, scripts and replay from a file — T-004.
- Cursors, selections, untitled/new documents, save-as, reload, external-change detection,
  line-ending normalisation command (`io.mixed-eol`) — later tasks.
- Filtering the log by `mutating`, log persistence, log size limits.
- `docs/development.md` "Adding a command" update for the new `Example` fields — a low docs
  follow-up (kept out so this task does not overlap T-005's `touches`).
- Changes in `kiln-text`, `kiln-workspace`, `apps/` (if a needed change lies there: stop, report).

## Implementer notes
- One module per command group (CHARTER §3 decision 6): `crates/kiln-commands/src/doc.rs`.
- Every command must leave the context unchanged on error, including the log.
- Results are produced from state, never echoed from params (e.g. `dirty` comes from the document).
- Run `cargo xtask schema --write` and commit `docs/commands.json`.
- Determinism: doc ids from a per-context counter; no clock, no randomness, no host paths in
  results or error messages.

## Decisions
Director's answers to the planner's questions of R-003 (2026-11-12).
- D-1 (was Q-1): document ids are integers (`u64`), assigned in increasing order, never reused
  within a session (a `Context`).
- D-2 (was Q-2): every command is logged, read-only ones included, with a `mutating` flag. Read
  here as every *successful* command: a failed command leaves the context unchanged, log
  included (registry rule "on error the context is unchanged"). This adds `const MUTATING: bool`
  to the `Command` trait and `"mutating"` to `docs/commands.json`.
- D-3 (was Q-3): opening an already open path is not an error; it returns the existing id with
  `already_open: true`.
- D-4 (was Q-4): no ADR. The `Example` format change (`files`, `setup`) and the `Command::MUTATING`
  addition are recorded in this spec and need the director's approval row like any high-tier task.
- D-5 (from T-002 D-2): `doc.open`/`doc.info` report the dominant line ending plus a
  `mixed_line_endings` flag; this task claims `io.crlf` as detection plus round-trip preservation.
- `Cargo.lock`, `docs/commands.json` are generated files under T-005's policy; until T-005 lands
  the director regenerates them on a rebase conflict.
- Director (2026-11-12): confirmed — only successful commands are logged; a failed command changes no state and is not replayed.
- Director rulings after the implementer's blocked report (2026-11-12, decision D-020) — part of the spec:
  1. `touches` gains `apps/kiln-cli/src/main.rs`, for one change only: the test `schema_lists_every_command` must stop hard-coding the full command list; it asserts that the CLI's listing equals the registry's own list and contains `buffer.insert` and `buffer.text`.
  2. In examples, `files` and `setup` are arrays of pairs as implemented (`[["doc.open", {...}]]`).
  3. `replay` does not roll back: when an entry fails, the entries before it stay applied and the error names the failing index.

## Questions
None open.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · tokens (if known) · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-12 · R-003 · (new) · did: specified eleven `doc.*` commands with params/results,
  `Context` constructors over a VFS, the command log and `Registry::replay`, and the `Example`
  extension needed so doc commands have executable examples; claimed `ws.documents`,
  `ws.dirty-tracking`, `io.crlf`, `cmd.log-replay`. Oracles: the L0 Buffer as reference model,
  log replay round-trip + idempotence, exact values. Tier high: command schema plus
  `Cargo.toml`/`Cargo.lock` (new `kiln-workspace` edge). Unsure: id vs path (Q-1), logging
  read-only commands (Q-2), re-open (Q-3), ADR need (Q-4); `io.crlf` completeness depends on
  T-002 Q-2. · gate attempts: 0 · not run: gate (no code) · missing context: none · next:
  director review.
- 2026-11-12 · R-003 · draft · did: applied the director's answers (D-1…D-5): u64 ids; log
  entries gain `mutating` (new `Command::MUTATING`); re-open returns the existing id with
  `already_open:true`; dominant line ending + `mixed_line_endings` in `doc.open`/`doc.info`; no
  ADR. Interpreted "log every command" as every successful command (stated in D-2). · gate
  attempts: 0 · next: director review.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
