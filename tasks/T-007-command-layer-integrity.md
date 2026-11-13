---
id: T-007
title: "Make replay detect divergence, enforce that read-only commands change nothing, log every command that ran, and give refusals a structured reason"
tier: high              # command schema (CommandError shape, examples, docs/commands.json); L3 + L4
state: draft
owner_role: planner
scorecard_items: [cmd.log-replay]
depends_on: [T-004, T-006, T-008]
touches:
  - crates/kiln-commands/src
  - crates/kiln-commands/tests/integrity.rs   # implementer's own integration tests
  - crates/kiln-control/src/lib.rs            # forced by the CommandError change; kinds from reasons; D-032 cap fix
  - docs/control-protocol.md                  # kind derivation sentence, log rule, D-032 items
  - docs/commands.json                        # generated: new read-only examples
budget_tokens: 700000
iteration: 1
found_by: reviewer
origin_task: T-003
---

# T-007 Make replay detect divergence, enforce that read-only commands change nothing, log every command that ran, and give refusals a structured reason

## Goal
Four integrity gaps in the command layer that reviews of T-003 and T-004 found, plus three
T-004 follow-ups (see Q-3). After this task:
(a) a caller can replay a log and learn exactly where and how the replay diverged from the
original session;
(b) a command declared read-only provably changes nothing but the log, for every read-only
command registered now or later, checked in the gate;
(c) a command that ran but produced a result that fails its own schema is still recorded, so
the log never omits a state change;
(d) refusals carry a structured reason, so `kiln-control` reports the documented error kinds
without reading message text. Rewording a message can no longer turn a kind into `other`;
(e) the error-size cap is measured without the echoed `id`, as the protocol document says.

## Reading list
- `docs/decisions.md` D-020 (3) (no rollback in replay), D-023, D-026 (threat model),
  D-031 (9)–(10), D-032.
- `reviews/T-003-r1.md` SG-2, SG-3, N-4 and "Specific questions" 5; `reviews/T-004-r3.md` R3-SG1, R3-N1…N3.
- `tasks/T-003-document-commands.md` Interface; `tasks/T-004-control-protocol-cli-run.md` Decisions.
- `crates/kiln-commands/src/{registry,doc,buffer}.rs`; `crates/kiln-control/src/lib.rs`
  (`Failure::from_command`, `rejected_kind`, `cap_error`, `command_error_json`, `ERROR_KINDS`).
- `docs/control-protocol.md` (error table, session-log section).
- `tasks/T-008-path-confinement.md` Interface (`VfsError::OutsideRoot`, kind `outside_root`).
- Tests that must stay green **unchanged**: `crates/kiln-commands/tests/acceptance_t003*.rs`
  and `guard_t003.rs` (they pin `LogEntry`'s three fields and `ReplayError.error: CommandError`),
  `crates/kiln-control/tests/acceptance_t004.rs`, `apps/kiln-cli/tests/acceptance_t004*.rs`.

## Interface

### Rust (`kiln_commands::…`, all re-exported from the crate root)
```rust
/// Why a command refused a request. `as_str()` and the serde form are the protocol `kind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectReason {
    InvalidPath, NoSuchDocument, UnsavedChanges, NotFound, NotAFile, OutsideRoot,
    IoError, DecodeError, OutOfRange, InvertedRange, Other,
}
impl RejectReason { pub fn as_str(self) -> &'static str; }   // "invalid_path", …, "other"

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InternalReason { ResultSchema, Panic, Other }
impl InternalReason { pub fn as_str(self) -> &'static str; }  // "result_schema", "panic", "other"

pub enum CommandError {                     // derives unchanged (Clone, Debug, PartialEq, Eq, Serialize)
    UnknownCommand { message: String },                          // unchanged
    InvalidParams { message: String, details: Vec<String> },     // unchanged
    Rejected { reason: RejectReason, message: String },          // CHANGED: + reason
    Internal { reason: InternalReason, message: String },        // CHANGED: + reason
}
impl CommandError {
    /// CHANGED: the reason is required (the one-argument form is removed).
    pub fn rejected(reason: RejectReason, message: impl Into<String>) -> Self;
    /// `{"code":…,"message":…[,"details":…][,"reason":…]}`; `reason` present for
    /// `rejected` and `internal` only.
    pub fn to_json(&self) -> serde_json::Value;
}
// Display of CommandError: unchanged (the message; for invalid_params "message: d1; d2").

impl Context {
    /// NEW. `results()[i]` is the value command `log()[i]` returned; always `results().len() == log().len()`.
    pub fn results(&self) -> &[serde_json::Value];
}

#[derive(Clone, Debug, PartialEq)]
pub enum Divergence {
    /// `log` and `results` differ in length; nothing was executed.
    LengthMismatch { log: usize, results: usize },
    /// The logged `mutating` flag differs from the registered command's; the entry was not executed.
    MutatingMismatch { logged: bool, registered: bool },
    /// The entry failed (unknown command, invalid params, rejected, internal).
    Failed(CommandError),
    /// The entry ran and stays applied, but returned something else.
    ResultMismatch { expected: serde_json::Value, actual: serde_json::Value },
}

#[derive(Clone, Debug, PartialEq)]
pub struct VerifyError { pub index: usize, pub divergence: Divergence }   // + Display, Error
// Display starts with exactly "replay diverged at log entry {index}: "; the rest is free text.

impl Registry {
    /// UNCHANGED behaviour. Doc comment must say: unverified re-execution; it compares no
    /// results and ignores the logged `mutating` flag; use `replay_verified` to detect divergence.
    pub fn replay(&self, ctx: &mut Context, log: &[LogEntry]) -> Result<Vec<Value>, ReplayError>;
    /// NEW. Verified replay; see semantics (a).
    pub fn replay_verified(&self, ctx: &mut Context, log: &[LogEntry], results: &[Value])
        -> Result<Vec<Value>, VerifyError>;
    /// CHANGED behaviour: also checks the read-only property, see (b).
    pub fn audit(&self) -> Vec<String>;
}
```
`LogEntry`, `ReplayError`, `Example`, `Command` and `execute`'s signature are unchanged.

### Semantics
**(a) Verified replay.** `replay_verified(ctx, log, results)`:
1. If `log.len() != results.len()`: `Err{index: min(log.len(), results.len()), LengthMismatch}`,
   nothing executed.
2. For each entry `i` in order: if no command has `log[i].name` → `Failed(UnknownCommand)`;
   else if the registered `MUTATING` ≠ `log[i].mutating` → `MutatingMismatch`, entry not
   executed; else execute it: error → `Failed(error)`; `Ok(actual)` with `actual != results[i]`
   (exact JSON equality) → `ResultMismatch{expected: results[i], actual}`.
3. Stop at the first divergence. No rollback (D-020 (3)): entries before `i` stay applied, an
   entry that ran stays applied and in `ctx`'s log, nothing after `i` runs.
4. On success return the results (equal to `results`), and `ctx.log()`/`ctx.results()` have
   grown by exactly `log`/`results`.

**(b) Read-only means unchanged.** A command with `MUTATING = false` must leave the context
exactly as it found it; the log and the results are not part of this comparison (`execute`
appends the command's own entry and result as for any command). "The context" means the scratch buffer (text and undo/redo history, including whether a typing group
is open); the set of open documents and, for each, its id, path, encoding, text, last-saved
text and undo/redo history including an open typing group; the next document id; and the VFS:
any `write_atomic` call counts as a change even if it writes identical bytes. `audit()` checks
this on every example of every read-only command. A violation is reported as one problem line
containing `` `<name>` example <i> (<description>): read-only command changed state ``. Every
current read-only command (`doc.text`, `doc.info`, `doc.list`, `buffer.text`) gains at least
one example whose setup leaves the context rich enough to expose a violation: for `doc.*`, two
open documents, one whose last edit is a single-char `doc.insert` (open typing group) and one
with an available redo step and unsaved changes; for `buffer.text`, a last setup command that is
a single-char `buffer.insert`. Existing examples stay. Run `cargo xtask schema --write`.

**(c) A command that ran is logged.** If a command's `run` returned `Ok` but its result does
not serialize or fails the result schema, `execute` returns `Err(Internal{reason, …})` with
`reason = ResultSchema` (schema violation) or `Other` (serialization failure). The command
**is** appended to the log (name, params as given, `mutating`), and `results()` gets the
produced value (`null` if it did not serialize). Every other failure (unknown command, invalid
params, `rejected`, internal error raised inside `run`) still appends nothing and leaves the
context unchanged. The `LogEntry` and `Context::log` doc comments change from "successfully
executed" to "ran to completion". (Panics are not covered; see Out of scope.)

**(d) Structured reasons.** Every refusal in `kiln-commands` sets its reason at the place it is
raised, as follows (this table is the specification; `Other` is produced by no current command):

| Source of the refusal | `RejectReason` |
|---|---|
| `RelPath::new` fails on a request path | `InvalidPath` |
| no open document has the id | `NoSuchDocument` |
| `doc.close` of a dirty document without `discard` | `UnsavedChanges` |
| `VfsError::NotFound` (open or save) | `NotFound` |
| `VfsError::NotAFile` | `NotAFile` |
| `VfsError::OutsideRoot` (T-008) | `OutsideRoot` |
| `VfsError::Io` | `IoError` |
| `OpenError::Decode` | `DecodeError` |
| `EditError::OutOfBounds` (`doc.insert/delete/replace`, `buffer.insert`) | `OutOfRange` |
| `EditError::InvertedRange` | `InvertedRange` |

The mapping from lower-layer error enums to reasons has no catch-all arm, so a variant added
later fails to compile until someone maps it (the reviewer checks this). In `kiln-control`
the `kind` of a `rejected` or `internal` error is the error's `reason.as_str()`, and the host's
caught-panic path produces `InternalReason::Panic`. **No message text is consulted** to choose a
`kind`. Protocol responses for every request are byte-identical to today's (same codes, kinds,
messages, details), except the change in (e). The `(code, kind)` set in
`docs/control-protocol.md` is unchanged except for T-008's `outside_root`. The sentence "The
`kind` of a `rejected` error is currently derived … from the message" is replaced by one saying
the kind comes from a structured reason.

**(e) T-004 follow-ups (D-032).** (1) In a host response, the decision to replace `details` by
`{"truncated":true}` uses the length of the canonical response with `"id":null` in place of
the echoed id. The id is still echoed in full. For `command_error_json` (no id) the measured
text is the canonical error object, as today. A measured length of exactly 4096 bytes keeps
`details`; 4097 truncates. (2) In `docs/control-protocol.md` the sentence "Integer ids are any
JSON integer …" moves from the character-offset paragraph to the id rules of the Request section.
The session-log paragraph states rule (c).

## Acceptance criteria
Through `kiln_commands` on `standard_registry()` and `kiln_control` public items unless stated.
"Liar" commands are test-defined `Command` types registered with `register` on a registry
returned by `standard_registry()`. Inside `run` they may change the context only by calling
`standard_registry()?.execute(ctx, …)` (the only public way), which also appends those nested
commands to the log. Tests must expect that.

- AC-1 (results are recorded): for fixed-seed random sequences (≥ 300 commands, failing ones
  included) on `Context::with_files(F)`, `ctx.results()` equals the list of `Ok` values
  `execute` returned, in order, and `results().len() == log().len()`. Oracle: exact value.
- AC-2 (verified replay round-trip): `replay_verified(fresh with_files(F), log, results)` →
  `Ok(v)` with `v == results`; the fresh context's `log()` and `results()` equal the source's.
  Oracle: round-trip.
- AC-3 (divergent initial state is detected): source `a.txt = "one"`, log `[doc.open a.txt,
  doc.text 1, doc.insert …]`; target `a.txt = "two"` → `Err(VerifyError{index: 1, divergence:
  ResultMismatch{expected: {"len_chars":3,"text":"one"}, actual: {"len_chars":3,"text":"two"}}})`;
  the target's `log().len() == 2` (entries 0 and 1 applied, entry 2 not run). Display starts
  `replay diverged at log entry 1: `. Oracle: exact value.
- AC-4 (tampered `mutating` flag): a log whose `doc.list` entry says `mutating: true` → `Err{index
  of that entry, MutatingMismatch{logged: true, registered: false}}`, and the target's log
  length equals that index (not executed). Same for `doc.insert` logged `false`. Oracle: exact value.
- AC-5 (other divergences): an entry for an unregistered name → `Failed(UnknownCommand{..})`;
  a `doc.open` whose file is missing in the target → `Failed(Rejected{reason: NotFound, ..})`;
  `log.len()=3, results.len()=2` → `Err{index: 2, LengthMismatch{log: 3, results: 2}}` with
  the target unchanged (empty log). Oracle: exact value.
- AC-6 (`replay` unchanged): every T-003 acceptance test passes unmodified; `replay` of a log
  on diverging files still returns `Ok` (unverified, as documented). Oracle: regression.
- AC-7 (read-only commands change nothing, black box): for every command that `describe()`
  marks `"mutating": false` (found at run time, not from a fixed list) and every one of its
  examples: on context A = the example's files + setup, then the command; context B = the same
  without the command; then the same probe on both: `doc.list`, then for each listed document
  `doc.info`, `doc.text`, `doc.undo`, `doc.text`, `doc.redo`, `doc.text`, `doc.info`; then
  `buffer.text`; then the bytes of every example file via `ctx.vfs().read`. All probe results
  are equal between A and B. At least one example per current read-only command has the rich
  setup required in (b). Oracle: differential (with vs without the command).
- AC-8 (audit enforces read-only, for future commands too): `standard_registry()?.audit()` is
  empty and `kiln-cli commands --audit` passes. For each liar below, registered with
  `MUTATING = false` and one example whose setup opens `a.txt` and leaves an open typing
  group (unless stated), `audit()` contains exactly one problem naming it with
  `read-only command changed state`: (i) nested `doc.insert` then `doc.delete` restoring the text;
  (ii) nested `doc.undo` then `doc.redo` (only the typing group ends); (iii) nested `doc.save` of
  a clean, just-opened document (its example's setup only opens `a.txt`; identical bytes are
  rewritten, nothing else changes); (iv) nested `doc.open` of a second file. Nested entries
  that the liars append to the log are not themselves the violation; the state changes are. An
  honest read-only liar that does not touch the context gives no problem. Oracle: exact value.
- AC-9 (a command that ran is logged): a liar `test.bad-result` with `MUTATING = true` and
  output type `{"n": integer}` whose serialization emits `{"n":"x"}`, and whose `run` performs
  a nested `doc.insert` → `execute` returns `Err(Internal{reason: ResultSchema, ..})`;
  `ctx.log()` ends with the nested `doc.insert` entry followed by the `test.bad-result` entry
  (params as given, `mutating: true`), and `ctx.results().last() == {"n":"x"}`. A liar whose
  `run` returns `Err(rejected(OutOfRange, …))` after nothing changed appends nothing. Oracle:
  exact value.
- AC-10 (reasons at their sources): through `execute`, each row of the (d) table gives exactly
  that reason: `doc.open "../x"`; `doc.text {"doc":9}`; dirty `doc.close`; `doc.open` of a
  missing file; of a directory (`MemoryFs` with `d/x`, open `d`); of `b"\xff"`; `doc.insert`
  offset 99; `doc.delete` 2..1; `buffer.insert` offset 99; a `doc.save` on `MemoryFs` with
  `set_fail_writes` → `IoError`; and on a `DiskFs` root with `link -> ../outside/f` (real file,
  presence asserted), `doc.open link` → `OutsideRoot`. `to_json()` of each has `"reason"` equal
  to `as_str()`. Oracle: exact value.
- AC-11 (kinds come from reasons, not text): for every `RejectReason` `r`,
  `kiln_control::command_error_json("doc.open", &json!({}), &CommandError::rejected(r, m))` has
  `kind == r.as_str()` for `m` = `"zz unrelated"` and for `m` = a message that today maps to a
  different kind (for example `"no open document 1"` with `r = NotFound`). The same holds for
  every `InternalReason` (`Internal{reason, message: "x"}`). `as_str()` values are exactly the
  kinds in `docs/control-protocol.md`'s table, and each serde form equals `as_str()`. Oracle:
  exact value.
- AC-12 (protocol unchanged): every T-004 acceptance test passes unmodified, and the
  `docs/control-protocol.md` worked example still gives the documented output byte for byte.
  Oracle: regression + exact value.
- AC-13 (id not counted, D-032): through `Host::handle_line`, the same failing request (for
  example `doc.insert` with a params object that yields `invalid_params` details) sent with
  `"id":1` and with a 5000-character string id gives identical `error` objects; neither has
  `{"truncated":true}` when the `id: 1` line is under 4096 bytes. Through `command_error_json`
  with a hand-built `InvalidParams` whose capped canonical object is exactly 4096 bytes,
  `details` is kept; one byte more gives `{"truncated":true}`. Oracle: differential + exact value.

## Scorecard items and intended test names
Tests in `crates/kiln-commands/tests/acceptance_t007.rs` and
`crates/kiln-control/tests/acceptance_t007.rs`.

| Item | Tests |
|---|---|
| cmd.log-replay | add `kiln_commands::acceptance_t007::verified_replay_round_trips`, `kiln_commands::acceptance_t007::verified_replay_reports_first_divergence` (existing tests stay) |

Also (no item): `read_only_commands_change_nothing_but_the_log`, `audit_catches_read_only_liars`,
`command_that_ran_is_logged_even_with_bad_result`, `refusals_carry_reasons_at_their_sources`
(kiln_commands); `kinds_follow_reasons_not_messages`, `echoed_id_does_not_count_toward_cap`,
`cap_boundary_is_exactly_4096_bytes` (kiln_control).

## Out of scope
- Verifying a log *script* (`kiln-cli run --log` output) against recorded results: that needs
  results in the script, a protocol change; a later task if wanted.
- Panics inside `run` (state possibly changed, not logged): command fuzzer task.
- Enforcing read-only in the type system (`&Context`): a CHARTER §3 contract change (ADR).
- `kiln-workspace`, `kiln-text`, `apps/` (no change needed; if one is, stop and report).
- `docs/development.md` "Adding a command" (docs follow-up).
- Any change to `Cargo.toml`, `Cargo.lock`, `xtask/`, `.cargo/`, `.github/`, lint tables or
  build keys. This task touches **no gate path and no dependency**.

## Implementer notes
- Your tests: unit tests in `crates/kiln-commands/src` and `crates/kiln-control/src/lib.rs`, and
  `crates/kiln-commands/tests/integrity.rs`. The unit tests that pin `rejected_kind`'s message
  matching go away with it. Replace them with tests of the reason mapping, and say so in your
  log (golden rule 4 disclosure).
- Specified: everything in Interface and Semantics. Left to you: how state is compared for
  (b), and how results are stored.
- Determinism: results are stored as returned; no clock or randomness. Messages unchanged.

## Questions
- Q-1 (planner, R-049): rule (c) records a command that ran even though `execute` returns an
  error. The alternative is to leave it unlogged and document that the log may then miss a
  state change. Rollback is not possible in general, because a save has already written the
  file. I recommend recording it.
- Q-2 (planner, R-049): I kept `replay` unverified and added `replay_verified` with its own
  error type, instead of changing `LogEntry` or `ReplayError`. T-003's acceptance tests pin
  `LogEntry`'s exact three fields and match `ReplayError.error` as a `CommandError`, and
  changing them would mean editing another task's acceptance tests. Confirm.
- Q-3 (planner, R-049): part (e) holds the D-032 follow-ups that D-032 assigned to T-006. They
  are here because this task already touches `kiln-control` and the protocol document (see
  T-006 Q-2). Confirm.
- Q-4 (planner, R-049): five parts in one high-tier task at 700k. Split option: T-007 = (a)–(c),
  `kiln-commands` only, and a new task = (d)–(e), L3 + L4. I kept one task because (c) and (d)
  both change `CommandError::Internal`.
- Q-5 (planner, R-049): `OutsideRoot` / `outside_root` assumes T-008 Q-1 is approved. If T-008
  uses another kind, this row follows it.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · tokens (if known) · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13T02:45Z · R-049 · (new) · did: specified (a) `Context::results` +
  `Registry::replay_verified` with `VerifyError{index, Divergence}` (length, mutating flag,
  failure, result mismatch; no rollback); (b) read-only property defined over all context state,
  enforced by `audit` on every read-only command's examples, with richer examples and a
  black-box differential over whatever `describe()` marks read-only (no fixed list, so no
  growing-registry snapshot, cf. D-020); (c) confirmed by reading `registry.rs` `execute`
  that a result failing its schema (or serialization) returns `Internal` after `run` changed
  the context and before the log push, so the change goes unlogged. Specified: log it. (d)
  `RejectReason`/`InternalReason` with a source table and the rule that `kiln-control` reads
  no message text; kind names unchanged. (e) D-032 follow-ups moved here from T-006. Kept
  `LogEntry`/`ReplayError` unchanged because T-003 acceptance tests pin them. Tier high:
  `CommandError` and examples (command schema), L3 + L4; no gate path, no dependency. ·
  gate attempts: 0 · not run: gate (no code) · missing context: none · next: director
  answers Q-1…Q-5.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
