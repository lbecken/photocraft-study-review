---
id: T-007
title: "Make replay detect divergence, enforce that read-only commands change nothing, and log every command that ran"
tier: high              # command layer contract (log semantics, examples, docs/commands.json)
state: draft
owner_role: planner
scorecard_items: [cmd.log-replay]
depends_on: [T-004, T-006]
touches:
  - crates/kiln-commands/src
  - crates/kiln-commands/tests/integrity.rs   # implementer's own integration tests
  - docs/commands.json                        # generated: new read-only examples
  - docs/control-protocol.md                  # session-log paragraph only (rule (c))
budget_tokens: 700000
iteration: 1
found_by: reviewer
origin_task: T-003
---

# T-007 Make replay detect divergence, enforce that read-only commands change nothing, and log every command that ran

## Goal
Reviews of T-003 and T-004 found three integrity gaps in the command layer. After this task:
(a) a caller can replay a log and learn exactly where and how the replay diverged from the
original session;
(b) a command declared read-only provably changes nothing but the log, for every read-only
command registered now or later, checked in the gate;
(c) a command that ran but produced a result that fails its own schema is still recorded, so
the log never misses a state change.
Structured refusal reasons and the T-004 follow-ups are task T-030 (Decisions D-1).

## Reading list
- `docs/decisions.md` D-020 (3) (no rollback in replay), D-023.
- `reviews/T-003-r1.md` SG-2, SG-3, N-4 and "Specific questions" 5.
- `tasks/T-003-document-commands.md` Interface.
- `crates/kiln-commands/src/{registry,doc,buffer}.rs`; `docs/control-protocol.md` "The session
  log as a script" and the sentence on unrecorded failures under "Error codes and kinds".
- Tests that must stay green **unchanged**: `crates/kiln-commands/tests/acceptance_t003*.rs`,
  `guard_t003.rs`, `guard_t004.rs` (they pin `LogEntry`'s three fields and
  `ReplayError.error: CommandError`), and every `kiln-control` / `kiln-cli` acceptance test.

## Interface

### Rust (`kiln_commands::…`, re-exported from the crate root)
```rust
impl Context {
    /// NEW. `results()[i]` is the value recorded for `log()[i]`; always `results().len() == log().len()`.
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
`LogEntry`, `ReplayError`, `CommandError`, `Example`, `Command` and `execute`'s signature are
unchanged (Decisions D-3). Specified: everything in this section and in Semantics. Left to the
implementer: how state is compared for (b), and how results are stored.

### Semantics
**(a) Verified replay.** `replay_verified(ctx, log, results)`:
1. If `log.len() != results.len()`: `Err{index: min(log.len(), results.len()), LengthMismatch}`;
   nothing is executed.
2. For each entry `i` in order: if no command has `log[i].name` → `Failed(UnknownCommand)`;
   else if the registered `MUTATING` ≠ `log[i].mutating` → `MutatingMismatch`, and the entry is
   not executed; else execute it: an error → `Failed(error)`; `Ok(actual)` with
   `actual != results[i]` (exact JSON equality) → `ResultMismatch{expected: results[i], actual}`.
3. Stop at the first divergence. No rollback (D-020 (3)): entries before `i` stay applied, an
   entry that ran stays applied and in `ctx`'s log, and nothing after `i` runs.
4. On success, return the results (equal to `results`); `ctx.log()` and `ctx.results()` have
   grown by exactly `log` and `results`.

**(b) Read-only means unchanged.** A command with `MUTATING = false` must leave the context
exactly as it found it. The log and the results are not part of this comparison (`execute`
appends the command's own entry and result as for any command). "The context" means:
- the scratch buffer: its text and its undo/redo history, including whether a typing group is
  open;
- the set of open documents and, for each, its id, path, encoding, text, last-saved text and
  undo/redo history, including an open typing group;
- the next document id;
- the VFS: any `write_atomic` call counts as a change, even if it writes identical bytes.

`audit()` checks this on every example of every read-only command. A violation is one problem
line containing `` `<name>` example <i> (<description>): read-only command changed state ``.
Every current read-only command (`doc.text`, `doc.info`, `doc.list`, `buffer.text`) gains at
least one example whose setup leaves the context rich enough to expose a violation:
- for `doc.*`: two open documents, one whose last edit is a single-char `doc.insert` (an open
  typing group), and one with an available redo step and unsaved changes;
- for `buffer.text`: a setup whose last command is a single-char `buffer.insert`.

Existing examples stay. Run `cargo xtask schema --write`.

**(c) A command that ran is logged** (Decisions D-2). Suppose a command's `run` returned `Ok`,
but its result does not serialize or fails the result schema. Then `execute` returns
`Err(CommandError::Internal{..})`, as today, with today's message shapes:
`` result of `<name>` violates its schema: … `` or `` result of `<name>` does not serialize: … ``.
The command **is** appended to the log (name, params as given, `mutating`), and `results()`
gets the produced value (`null` if it did not serialize). Every other failure still appends
nothing and leaves the context unchanged: unknown command, invalid params, `rejected`, or an
error returned by `run`. The `LogEntry` and `Context::log` doc comments change from
"successfully executed" to "ran to completion". In `docs/control-protocol.md`, the sentence
saying a command that returns an error is not recorded gains this exception
(`internal`/`result_schema` and the serialization case). Panics are not covered; see Out of scope.

## Acceptance criteria
All through `kiln_commands` on `standard_registry()`. "Liar" commands are test-defined `Command`
types registered with `register` on a registry returned by `standard_registry()`. Inside `run`
they can change the context only by calling `standard_registry()?.execute(ctx, …)`, the only
public way. Those nested calls also append to the log, and tests must expect that.

- AC-1 (results are recorded): for fixed-seed random sequences (≥ 300 commands, failing ones
  included) on `Context::with_files(F)`, `ctx.results()` equals the list of `Ok` values
  `execute` returned, in order, and `results().len() == log().len()`. Oracle: exact value.
- AC-2 (verified replay round-trip): `replay_verified(fresh with_files(F), log, results)` →
  `Ok(v)` with `v == results`; the fresh context's `log()` and `results()` equal the source's.
  Oracle: round-trip.
- AC-3 (divergent initial state is detected): source `a.txt = "one"`, log `[doc.open a.txt,
  doc.text 1, doc.insert …]`; target `a.txt = "two"` → `Err(VerifyError{index: 1, divergence:
  ResultMismatch{expected: {"len_chars":3,"text":"one"}, actual: {"len_chars":3,"text":"two"}}})`;
  the target's `log().len() == 2` (entries 0 and 1 applied, entry 2 not run); its Display starts
  with `replay diverged at log entry 1: `. Oracle: exact value.
- AC-4 (tampered `mutating` flag): a log whose `doc.list` entry says `mutating: true` → `Err{index
  of that entry, MutatingMismatch{logged: true, registered: false}}`, and the target's log
  length equals that index (the entry did not run). The same holds for `doc.insert` logged as
  `false`. Oracle: exact value.
- AC-5 (other divergences): an entry for an unregistered name → `Failed(CommandError::UnknownCommand{..})`;
  a `doc.open` whose file is missing in the target → `Failed(CommandError::Rejected{..})`;
  `log.len()=3, results.len()=2` → `Err{index: 2, LengthMismatch{log: 3, results: 2}}`, with
  the target unchanged (empty log). Oracle: exact value.
- AC-6 (`replay` unchanged): every T-003 acceptance test passes unmodified, and `replay` of a log
  on diverging files still returns `Ok` (unverified, as documented). Oracle: regression.
- AC-7 (read-only commands change nothing, black box): for every command that `describe()`
  marks `"mutating": false` (found at run time, not from a fixed list), and for every one of
  its examples:
  - context A = the example's files and setup, then the command; context B = the same without
    the command;
  - then the same probe on both: `doc.list`; for each listed document `doc.info`, `doc.text`,
    `doc.undo`, `doc.text`, `doc.redo`, `doc.text`, `doc.info`; then `buffer.text`; then the
    bytes of every example file via `ctx.vfs().read`;
  - all probe results are equal between A and B.

  At least one example per current read-only command has the rich setup required in (b).
  Oracle: differential (with vs without the command).
- AC-8 (audit enforces read-only, for future commands too): `standard_registry()?.audit()` is
  empty and `kiln-cli commands --audit` passes. Each liar below is registered with
  `MUTATING = false` and one example whose setup opens `a.txt` and leaves an open typing group
  (unless stated). For each, `audit()` contains exactly one problem naming it, with
  `read-only command changed state`:
  - (i) a nested `doc.insert`, then a `doc.delete` that restores the text;
  - (ii) a nested `doc.undo`, then `doc.redo` (only the typing group ends);
  - (iii) a nested `doc.save` of a clean, just-opened document (its example's setup only opens
    `a.txt`; identical bytes are rewritten and nothing else changes);
  - (iv) a nested `doc.open` of a second file.

  The nested entries the liars append to the log are not the violation; the state changes are.
  An honest read-only liar that does not touch the context gives no problem. Oracle: exact value.
- AC-9 (a command that ran is logged): take a liar `test.bad-result` with `MUTATING = true`,
  output type `{"n": integer}` whose serialization emits `{"n":"x"}`, and a `run` that makes a
  nested `doc.insert`. Then:
  - `execute` returns `Err(CommandError::Internal{..})` whose message starts with
    `` result of `test.bad-result` violates its schema ``;
  - `ctx.log()` ends with the nested `doc.insert` entry followed by the `test.bad-result` entry
    (params as given, `mutating: true`), and `ctx.results().last() == {"n":"x"}`;
  - `replay_verified` of that log and those results on a fresh context stops at that entry with
    `Failed(Internal{..})`.

  A liar whose `run` returns `Err(CommandError::rejected(…))` without changing anything appends
  nothing. Oracle: exact value.

## Scorecard items and intended test names
Tests in `crates/kiln-commands/tests/acceptance_t007.rs` (`kiln_commands::acceptance_t007::<fn>`).

| Item | Tests |
|---|---|
| cmd.log-replay | add `verified_replay_round_trips`, `verified_replay_reports_first_divergence` (existing tests stay) |

Also (no item): `results_are_recorded_for_every_logged_command`,
`read_only_commands_change_nothing_but_the_log`, `audit_catches_read_only_liars`,
`command_that_ran_is_logged_even_with_bad_result`.

## Out of scope
- Structured reasons in `CommandError`, kind derivation in `kiln-control`, and the D-032 cap
  follow-ups: T-030.
- Verifying a log *script* (`kiln-cli run --log` output) against recorded results: that needs
  results in the script, which is a protocol change; a later task if wanted.
- Panics inside `run` (state possibly changed and not logged): a command fuzzer task.
- Enforcing read-only in the type system (`&Context`): a CHARTER §3 contract change (ADR).
- `kiln-control`, `kiln-workspace`, `kiln-text`, `apps/` (no change needed; if one is, stop and
  report). `docs/development.md` "Adding a command" (docs follow-up).
- Any change to `Cargo.toml`, `Cargo.lock`, `xtask/`, `.cargo/`, `.github/`, lint tables or
  build keys. This task touches **no gate path and no dependency**.

## Implementer notes
- Your tests: unit tests in `crates/kiln-commands/src` and `crates/kiln-commands/tests/integrity.rs`.
- Determinism: results are stored as returned; no clock or randomness; messages unchanged.

## Decisions
Director's answers to the planner's questions of R-049 (2026-11-13, run R-051).
- D-1 (was Q-4): split. T-007 keeps (a) verified replay, (b) the read-only property and (c)
  logging a command whose result fails its schema. The new task T-030 takes structured
  rejection reasons with stable kind names in `kiln-control`, plus the three D-032 follow-ups
  (was Q-3). **Order: T-007 lands before T-030.** T-030 changes the shape of `CommandError`,
  which every refusal site and `kiln-control` use, including the `internal` path that rule (c)
  defines here. Landing T-007 first lets T-030 assign a reason to that final path and test it
  once, instead of T-007 having to re-touch reasons. It also lets T-007 start without waiting
  for T-008, which T-030 needs for `outside_root`. They cannot be in flight together: both touch
  `crates/kiln-commands/src` and `docs/control-protocol.md`.
- D-2 (was Q-1): accepted. A command whose result fails its own schema is still logged, and the
  caller gets an `internal` error.
- D-3 (was Q-2): accepted. `replay_verified` is added; `LogEntry` and `ReplayError` do not
  change (T-003's acceptance tests pin them).
- D-4 (was Q-5): `outside_root` is approved; it now concerns T-030, not this task.
- Dispatch note: T-007 shares `docs/control-protocol.md` with T-008, so the two may not be in
  flight at the same time (METHODOLOGY §5.2). T-007 does not depend on T-008's code.

## Questions
None open.

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
- 2026-11-13T03:05Z · R-051 · draft · did: applied the director's answers (Decisions D-1…D-4):
  moved parts (d) and (e) to new task T-030; `kiln-control/src/lib.rs` left `touches`; ACs on
  reasons, kinds and caps went with them; AC-5/AC-9 now name `CommandError` variants without
  reasons; `depends_on` dropped T-008 (no code dependency; the shared protocol document is a
  dispatch constraint, recorded); stated why T-007 lands before T-030. Still high: it changes
  the command layer's log contract and `docs/commands.json` examples. · gate attempts: 0 ·
  missing context: none · next: director review.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
