---
id: T-007
title: "Make replay detect divergence, enforce that read-only commands change nothing, and log every command that ran"
tier: high              # ADR-0002: changes how the command log and the registry's audit work = CHARTER §3 contract (L3 "command log for replay"); ADR needed
state: draft
owner_role: planner
scorecard_items: [cmd.log-replay]   # CLI criterion: `kiln-cli replay` (Decisions D-5)
depends_on: [T-006, T-032]   # D-041: CommandError reasons first (T-015 transitively)
touches:
  - crates/kiln-commands/src/registry.rs     # R-060: narrowed from the directory
  - crates/kiln-commands/src/doc.rs          # rich read-only examples
  - crates/kiln-commands/src/buffer.rs       # rich read-only example
  - crates/kiln-commands/src/lib.rs          # re-exports of Divergence, VerifyError
  - crates/kiln-commands/tests/integrity.rs   # implementer's own integration tests
  - apps/kiln-cli/src/main.rs                 # `run --results`, `replay` (Decisions D-5)
  - apps/kiln-cli/tests/replay.rs             # implementer's own CLI tests
  - docs/protocol/replay.md                   # NEW: this task's protocol text (D-6); docs/control-protocol.md is not touched
  # docs/commands.json changes too: a generated path (exempt from `touches`, regenerated on conflict)
budget_tokens: 850000   # high (v1.1)
iteration: 2
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
Structured refusal reasons are T-032 (D-041); the T-004 follow-ups stay in T-030.

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

**(b) Read-only means unchanged.** A command with `MUTATING = false` must leave the context's
**observable state** exactly as it found it (director, R-060): what some command result can
expose — document text, dirty flag, undo history, ids, the command log, and selections once
T-018 lands. A derived cache that no command result exposes (such as T-020's syntax-tree cache)
is not state, and the check must not compare it. T-015's group state structs are compared only
as their owning task specifies (T-022's previews, T-031's keymap are observable through
commands); until then they are empty (spec check R-062, finding 24). The log and the results are not part of this comparison (`execute`
appends the command's own entry and result as for any command). "The context" means:
- the scratch buffer: its text and its undo/redo history, including whether a typing group is
  open;
- the set of open documents and, for each, its id, path, encoding, text, last-saved text and
  undo/redo history, including an open typing group;
- the next document id;
- the VFS: any `write_atomic` call counts as a change, even if it writes identical bytes.

`audit()` checks this on every example of every read-only command. A violation is one problem
line containing `` `<name>` example <i> (<description>): read-only command changed state ``.
Every read-only command registered on the `main` this task starts from (today `doc.text`,
`doc.info`, `doc.list`, `buffer.text`; also `doc.line` and `doc.position` if T-010 has merged
by then) gains at least one example whose setup leaves the context rich enough to expose a violation:
- for `doc.*`: two open documents, one whose last edit is a single-char `doc.insert` (an open
  typing group), and one with an available redo step and unsaved changes;
- for `buffer.text`: a setup whose last command is a single-char `buffer.insert`.

Existing examples stay, in their positions (new ones are appended: `guard_t003.rs` reads
`examples[0]` of `buffer.insert` and `buffer.text`). Run `cargo xtask schema --write`.

**(c) A command that ran is logged** (Decisions D-2). Suppose a command's `run` returned `Ok`,
but its result does not serialize or fails the result schema. Then `execute` returns
`Err(CommandError::Internal{..})`, as today, with today's message shapes:
`` result of `<name>` violates its schema: … `` or `` result of `<name>` does not serialize: … ``.
The error carries `reason: InternalReason::ResultSchema` (schema) or `InternalReason::Other` (serialization), from T-032. The command **is** appended to the log (name, params as given, `mutating`), and `results()`
gets the produced value (`null` if it did not serialize). Every other failure still appends
nothing and leaves the context unchanged: unknown command, invalid params, `rejected`, or an
error returned by `run`. The `LogEntry` and `Context::log` doc comments change from
"successfully executed" to "ran to completion". `docs/protocol/replay.md` states this exception
(`internal`/`result_schema` and the serialization case) and that it supersedes the sentence in
`docs/control-protocol.md` "Error codes and kinds" saying a command that returns an error is not
recorded; T-030 corrects that sentence in place (D-6: this task does not edit the shared file). Panics are not covered; see Out of scope.

### (d) Verified replay through the CLI (Decisions D-5)
```text
kiln-cli run <script> [--root <dir> | --memory] --log <file> [--results <file>]
kiln-cli replay <log-script> <results-file> [--root <dir> | --memory]
```
- `run --results <file>` (only together with `--log`; alone it is a usage error, exit 2) writes,
  after the last request, one line per line of the `--log` script, in the same order:
  `{"id":<n>,"result":<value>}` (sorted keys, `\n`-terminated), `<n>` the script line's `id`,
  `<value>` the result the logged request produced (the response's `result`; `null` for a request
  that rule (c) logs although its response is an error). Created or truncated before the first
  request, like `--log`. Every other output of `run` is unchanged.
- `replay` reads the log script (the `--log` format) and the results file, then replays the
  script on a fresh session over the given file system with `replay_verified` semantics
  (a): entries run in order; it stops at the first divergence; nothing is rolled back (files the
  replayed entries saved stay saved). It writes exactly one line to stdout:
  - all equal: `{"ok":true,"replayed":<n>}`, exit 0;
  - divergence: `{"divergence":<d>,"id":<script id or null>,"index":<i>,"ok":false}`, exit 3,
    where `<d>` is one of `{"actual":…,"expected":…,"kind":"result_mismatch"}`,
    `{"error":<error object exactly as \`exec\` prints it>,"kind":"failed"}`,
    `{"kind":"length_mismatch","log":<n>,"results":<m>}` (`id` null, nothing run).
    (`mutating_mismatch` cannot occur: a script carries no flag; registered flags are used.)
  - exit 2, nothing on stdout, a message on stderr: usage error, unreadable file, a script line
    that is not a valid request, a results line that is not `{"id":…,"result":…}`, or ids of the
    results file that differ from the script's ids in order; exit 1: writing stdout failed.
- `docs/protocol/replay.md` documents (d), the results file and these exit codes; `kiln-cli
  --help` lists the new forms.
- No `kiln-control` change: if `kiln-cli` cannot build this from `kiln-commands`,
  `kiln-workspace` and the existing `Host` API, stop and report.

## Pinned
- (a) Errors, payloads, precedence: `Divergence` variants and `VerifyError{index, divergence}`
  as in the Interface; precedence inside one entry: unknown name → `MutatingMismatch` →
  execution error (`Failed`) → `ResultMismatch`; `LengthMismatch` before any entry runs. The
  audit problem line and the `Internal` message shapes are exact (Semantics (b), (c)).
- (b) Files: `docs/commands.json` (regenerated: appended examples only); the results file and
  the `replay` output of (d); new `docs/protocol/replay.md` (the rule (c) text and (d));
  one sentence (Semantics (c)); no file system access by the code beyond the existing VFS.
- (c) Order of checks: Semantics (a) steps 1–4; `execute` keeps its order (lookup → params
  schema → run → result serialization → result schema), and the log push moves after the
  result checks only for the (c) cases.
- (d) Path base, platform, file types: n/a (in-memory `Context::with_files`; no host path).
- (e) Threat model: n/a for the gate; the property is integrity, not security. "Liar" commands
  model future honest mistakes, not malicious code (a liar could still panic; out of scope).
- (f) Tests pinning a snapshot this changes: `acceptance_t003*.rs`, `guard_t003.rs`,
  `guard_t004.rs` pin `LogEntry`'s three fields, `ReplayError.error: CommandError` and
  `examples[0]` of the buffer commands — all kept. `docs/commands.json` changes and is
  regenerated (`cargo xtask schema --write`); `apps/kiln-cli/tests/cli.rs` compares the dump to
  that file, so it stays green. No test pins the number of examples (grep `"examples"` in
  `crates`, `apps`).
- (g) Worked examples: AC-3's values follow from `doc.text`'s result shape
  `{"len_chars":…,"text":…}` on today's `main` (planner R-055 ran `doc.text` through
  `kiln-cli run`: `{"len_chars":2,"text":"xa"}` for a two-char document). AC-4…AC-9 are rules,
  not computed values.
- (h) `cmd.log-replay`, evaluator's call: `kiln-cli run s.jsonl --root A --log l.jsonl
  --results r.jsonl`, then `kiln-cli replay l.jsonl r.jsonl --root B` on a fresh copy of A
  (`{"ok":true,…}`, exit 0) and on a copy with one file changed (the exact divergence line,
  exit 3) — AC-11, AC-12.

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
- AC-10 (through the CLI): `kiln-cli commands --schema` lists, for every command whose
  `mutating` is `false`, at least one example whose `setup` opens two documents (one ending in a
  single-char `doc.insert`, one with a redo step and unsaved changes; for `buffer.text`, a setup
  ending in a single-char `buffer.insert`), and `kiln-cli commands --audit` exits 0. Fails on
  base (no such examples). Oracle: exact value over the parsed JSON.
- AC-11 (`cmd.log-replay`, CLI round trip): for fixed-seed random sessions (≥ 300 requests,
  failing ones included) over files on disk, `run --log l --results r` then `replay l r --root`
  on a fresh copy of the initial directory → `{"ok":true,"replayed":<lines of l>}`, exit 0, and
  the copy's files equal the original session's files byte for byte. The same with `--memory`
  on scripts that open no file. Oracle: round trip.
- AC-12 (`cmd.log-replay`, CLI divergence): AC-3's scenario through the CLI (`a.txt` = `one` when
  recorded, `two` at replay) → exactly
  `{"divergence":{"actual":{"len_chars":3,"text":"two"},"expected":{"len_chars":3,"text":"one"},"kind":"result_mismatch"},"id":2,"index":1,"ok":false}`,
  exit 3; a target missing `a.txt` → `kind: "failed"` with the `rejected`/`not_found` error
  object, `index` 0; a results file one line short → `length_mismatch`; `--results` without
  `--log`, a results file with a wrong id → exit 2, empty stdout. Oracle: exact value.

## Scorecard items and intended test names
Tests in `crates/kiln-commands/tests/acceptance_t007.rs` (`kiln_commands::acceptance_t007::<fn>`).

| Item | Tests |
|---|---|
| cmd.log-replay | add `kiln_cli::acceptance_t007_cli::replay_round_trips_through_cli`, `kiln_cli::acceptance_t007_cli::replay_reports_first_divergence_through_cli` (AC-11, AC-12; existing tests stay), and `verified_replay_round_trips`, `verified_replay_reports_first_divergence` (Rust API, additional) |

Also (no item): `results_are_recorded_for_every_logged_command`,
`read_only_commands_change_nothing_but_the_log`, `audit_catches_read_only_liars`,
`command_that_ran_is_logged_even_with_bad_result`, and in
`apps/kiln-cli/tests/acceptance_t007_cli.rs` `read_only_commands_publish_rich_examples` (AC-10;
the distinct target name avoids two `acceptance_t007` targets in two packages, which the join
miscounts until T-009 (6) lands).

## Out of scope
- Structured reasons in `CommandError`, kind derivation in `kiln-control`, and the D-032 cap
  follow-ups: T-030.
- Panics inside `run` (state possibly changed and not logged): a command fuzzer task.
- Enforcing read-only in the type system (`&Context`): a CHARTER §3 contract change (ADR).
- `kiln-control`, `kiln-workspace`, `kiln-text`, `apps/kiln` and any `kiln-cli` subcommand other
  than `run --results` and `replay` (if a change is needed, stop and report);
  `docs/control-protocol.md` (D-6; T-030 corrects its stale sentence). `docs/development.md` "Adding a command" (docs follow-up).
- Any change to `Cargo.toml`, `Cargo.lock`, `xtask/`, `.cargo/`, `.github/`, lint tables or
  build keys. This task touches **no gate path and no dependency**.

## Implementer notes
- Your tests: unit tests in `crates/kiln-commands/src` and `crates/kiln-commands/tests/integrity.rs`;
  CLI tests in `apps/kiln-cli/tests/replay.rs`.
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
- Dispatch note (R-057, replaces the R-051/R-055 notes): T-007 no longer touches
  `docs/control-protocol.md` (D-6). Its `touches` overlap T-006, T-010, T-030
  (`crates/kiln-commands/src`) and T-011 (`apps/kiln-cli/src/main.rs`). It may run in parallel
  with T-008, T-009, T-013.
- Tier note (R-055): under ADR-0002 a change to how the command log or the registry's audit
  works is a CHARTER §3 contract change, so the director writes an ADR before `ready`
  (METHODOLOGY §4 table) and the task stays its own task (P1).

- D-5 (director R-057, was Q-6): keep the `cmd.log-replay` claim by making verification
  observable: a `kiln-cli` way to replay a recorded log against a root and report divergence,
  planner's choice of form, output and exit codes pinned. Specified as (d), AC-11, AC-12.
- D-6 (director R-057, all product tasks): a task documents its commands in its own new file
  `docs/protocol/<group>.md` and touches `docs/control-protocol.md` only to add an error-kind
  line. T-007 adds no kind, so it does not touch that file.

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
- 2026-11-13T03:50Z · R-055 · draft · did: brought the spec to v1.1: tier high under ADR-0002
  (criterion: it changes how the command log and the registry's audit work, which ADR-0002
  names as a CHARTER §3 contract change; so an ADR is due), budget 850k; added Pinned; dropped
  the `cmd.log-replay` claim for lack of any CLI/control-observable criterion (Q-6) and added
  AC-10 through `kiln-cli commands`; made (b) cover `doc.line`/`doc.position` if T-010 merges
  first; pinned that new examples are appended (`guard_t003.rs` reads `examples[0]`); named the
  CLI test file `acceptance_t007_cli.rs`. Arm n/a (high: independent test author). · gate
  attempts: 0 · missing context: none · next: director answers Q-6, writes the ADR.
- 2026-11-13T03:56Z · R-057 · draft · did: applied D-5 (`run --results`, `kiln-cli replay`,
  exact output and exit codes, AC-11/AC-12, `cmd.log-replay` claimed again) and D-6 (protocol
  text in `docs/protocol/replay.md`; `docs/control-protocol.md` and the generated
  `docs/commands.json` left out of `touches`). Worked example of AC-12 is AC-3's values in the
  CLI's sorted-key form (not executed: `replay` does not exist yet). · gate attempts: 0 ·
  missing context: none · next: director review, ADR.
- 2026-11-13 · R-060 · draft · did: contracts-first restructuring: depends on T-006 and T-015 (T-015 now carries structured reasons, so rule (c)'s `Internal` error is raised with `InternalReason::ResultSchema` from the start; there is no "T-007 before T-030" order any more). `touches` narrowed from `crates/kiln-commands/src` to `registry.rs`, `doc.rs`, `buffer.rs`, `lib.rs`, so the feature modules of T-015 stay disjoint. Added the director's observable-state sentence to (b). References to T-030 for reasons are superseded by T-015; T-030 still corrects the "not recorded" sentence of `docs/control-protocol.md`. · gate attempts: 0 · missing context: none · next: director.
- 2026-11-13 · R-066 · draft · did: `depends_on` T-006 (merged), T-032: rule (c) raises `Internal { reason: InternalReason::ResultSchema | Other }` from T-032, and `registry.rs` is serialised after T-015 and T-032. Added the group-state sentence to (b) (finding 24). · gate attempts: 0 · missing context: none · next: director.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
