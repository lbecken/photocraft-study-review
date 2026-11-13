---
id: T-009
title: "Close the merge queue's remaining gaps: bind merge records to their commits, survive signing, catch lint escapes, tolerate drafts, count acceptance tests per crate, keep the scorecard file current"
tier: high              # gate paths (xtask/**); the approval row must contain `gate paths`
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-005]
touches:
  - xtask/src
  - xtask/ledger.toml            # new `history_anchor` key
  - docs/development.md          # queue, join and residual-risk sections
budget_tokens: 700000
iteration: 1
found_by: reviewer
origin_task: T-005
---

# T-009 Close the merge queue's remaining gaps: bind merge records to their commits, survive signing, catch lint escapes, tolerate drafts, count acceptance tests per crate, keep the scorecard file current

## Goal
T-005 made `cargo xtask queue` the only way code reaches `main`. Its final review (r3, findings
19–24) and two joins on T-003/T-004 left seven gaps. After this task:
(1) a `merged` line in `metrics/queue.jsonl` counts as evidence only if the queue's own record
commit wrote it;
(2) a merge that succeeded is never reported as a failure because the record commit failed;
(3) a crate that drops the workspace lints, or source code that adds lint `allow`s, is treated
as a gate change, so it needs tier high and an approval;
(4) the record-commit pathspec and the identity fallback have tests;
(5) untracked drafts no longer block the queue unless they are the queued task's own evidence;
(6) `join` counts acceptance tests correctly when two crates have a target of the same name,
and joins tasks whose tests live in `xtask/tests`;
(7) `docs/scorecard.md` is regenerated where it goes stale (at join, and after every rebase in
the queue), so the director stops doing it by hand (D-012).

**Gate paths.** Every path in `touches` except `docs/development.md` is a gate path. The
director's approval row in `docs/decisions.md` must begin `Approve T-009` and contain the words
`gate paths`, or the queue refuses the task (T-005 R3). The task changes no dependency (no
`Cargo.toml`, `Cargo.lock`, lint table or build key). Per T-005 D-3, it must pass both main's
gate and its own.

## Reading list
- `docs/decisions.md` D-012, D-019, D-021, D-025 (join counts), D-026 (threat model),
  D-030 (R10–R13), D-033; `reviews/T-005-r3.md` findings 19–26 and the follow-up list.
- `tasks/T-005-queue-hardening.md` Interface and Decisions R1–R13 (the queue's exit codes,
  refusals, record format, precondition order).
- `xtask/src/{precheck,queue,join,policy,scorecard}.rs`; `xtask/ledger.toml`;
  `xtask/generated.toml`; `docs/development.md` "Merge queue" and "Join".
- `metrics/tasks.jsonl` T-004 `join` records (for example `acc_total: 6`, `acc_fail_base: 13`
  at 2026-11-13T01:18:27Z).

## Interface
Existing commands keep their names, options, refusal format (`queue: refused: <check>: <reason>`,
one line, exit 1), record formats and precondition order unless a change is listed here.

### (1) Merge records bound to record commits (`main-history`)
A `merged` record `R` (task `T`, `head_sha` `H`) in main's committed `metrics/queue.jsonl` is
**bound** iff main's first-parent history contains a commit `C` such that:
- `C`'s first parent is `H`;
- `C`'s subject is exactly `queue: record T merged`;
- `C` changes exactly one path, `metrics/queue.jsonl`;
- `C`'s change to that file appends exactly one line, byte-identical to `R`'s line.

The `main-history` anchor is the `head_sha` of the newest bound record. An unbound `merged`
line is ignored as evidence: it neither moves the anchor nor is refused by itself. If no record
is bound, the anchor is the commit named by a new key in main's committed `xtask/ledger.toml`:
`history_anchor = "<40-hex sha>"`. This task sets it to `a981f18210cbd59be20e5e7009b9ed8429850b64`,
the `head_sha` of T-005's `merged` record; see Q-2. If neither exists, the result is
`queue: refused: main-history: no merged record is bound to a queue record commit and
xtask/ledger.toml names no history_anchor`. The existing per-commit ledger check from the
anchor to `main` is unchanged.

### (2) Record commit and commit signing
The record commit uses the repository's commit configuration unchanged, including
`commit.gpgsign` (so a signed-commits policy on `main` keeps holding; see Q-1). It still uses
`--no-verify`, the identity fallback and the pathspec `-- metrics/queue.jsonl`. Suppose the
fast-forward succeeded and the record commit then fails, for any reason. Then:
- the appended record (result `merged`) stays in `metrics/queue.jsonl`;
- stderr gets one line: `queue: merged T-NNN at <head_sha>, but the record commit failed: <git error>; run: git commit -m "queue: record T-NNN merged" -- metrics/queue.jsonl` (the exact command that produces a bound commit, see (1));
- the exit code is **3**, which means merged but not recorded. Exit 0 stays merged and recorded,
  1 not merged, and 2 an error.

If the record commit fails after a non-merged outcome, the exit stays 1 and stderr gets the same
hint with that result. The next queue run is refused at `ledger` until the record is committed,
as today.

### (3) Lint escapes are gate changes
A path counts as a gate path (so it needs tier high and an approval containing `gate paths`,
like R10) when, comparing the merge base with the task head:
- (a) it is the `Cargo.toml` of a workspace member that has no `[lints]` table with exactly
  `workspace = true` in the task head (a new member that omits it, or an existing one that
  drops it);
- (b) it is a `.rs` file in which the task **adds** a lint-level attribute: `allow`, `expect`,
  `warn` or `deny` at item or crate level (`#[…]` or `#![…]`), including inside `cfg_attr(…)`.
  Attributes that already exist on the merge base do not count. Neither do moved lines whose
  attribute text is unchanged (left to the implementer how moves are detected, as long as an
  added attribute is never missed).

The refusal names the path and the reason, for example
`queue: refused: gate-path: task/T-NNN changes gate path(s) crates/x/Cargo.toml (no [lints] workspace = true); such a task must be tier high (it is medium)`.
Test files are not exempt (see Q-3).

### (5) What `ledger` tolerates
`ledger` (and the dirty-tree check after it) refuses for:
- tracked evidence files with any uncommitted change (modified, deleted, staged, renamed), as
  today;
- untracked files under `metrics/`;
- untracked files that are the queued task's own evidence: `tasks/T-NNN-*.md` and
  `reviews/T-NNN-*`.

Every other untracked file is tolerated: other tasks' draft task files and reviews, and any
untracked file outside the evidence paths. The queue never stages, commits, moves or deletes
an untracked file. The refusal message lists the files it refused for.

### (6) Join counts
An acceptance test is identified by **(package, target, test name)**, not by target name, so
`crates/kiln-control/tests/acceptance_t004.rs` and `apps/kiln-cli/tests/acceptance_t004.rs`
are two targets. In the `join` record:
- `acc_total` = acceptance tests on the joined branch, across all packages, including
  `xtask/tests`;
- `acc_fail_base` = how many of those did not pass on base, counting failed tests plus the
  tests of targets that did not compile on base;
- therefore `acc_fail_base ≤ acc_total` always.

The guard counts (`guard_total`, `guard_fail_base`, `guard_fail_join`) use the same identity.
Record field names are unchanged (METHODOLOGY §13).

### (7) Scorecard file regeneration (planner judgement: sound, for `docs/scorecard.md` only)
- `join`, after the merge (step 3) and before the acceptance run, runs the `scorecard` command
  of the xtask it was built from in the joined worktree. If `docs/scorecard.md` changed, it
  commits only that file on `task/T-NNN` with subject `join: regenerate docs/scorecard.md` and
  `Task: T-NNN` / `Role: director` lines.
- `queue`, after every successful rebase (not only after a generated-path conflict), runs
  main's `scorecard` in the rebased worktree and commits `docs/scorecard.md` if it changed (the
  existing regeneration commit form). It adds the path to the record's existing `regenerated`
  list.

Why this is sound: `docs/scorecard.md` is derived only from `scorecard/items/`, evaluator
confirmations and test names. Main's xtask renders it, and the gate's `scorecard --check`
still verifies it. It goes stale for reasons outside the task: a join adds test names, and a
director ledger commit adds an item or an evaluation (for example the new
`ws.path-confinement` item). Why only this file: regenerating `docs/commands.json` or
`Cargo.lock` at join would hide an implementer's missing update, which the gate must catch.

## Acceptance criteria
In throw-away git repositories that the tests build (as T-005's acceptance tests do), with
`GIT_CONFIG_GLOBAL`/`HOME` isolated so the host's signing and identity settings do not leak in
unless a case sets them. Oracle: exact value (exit code, the one refusal line, record contents,
`git log`/`git show` output) unless stated.

- AC-1 (fake merged line ignored): after a real queue merge of task A, commit on main a
  ledger commit (`tasks/x.md` plus a fabricated `merged` line for task Z with `head_sha` =
  current `main`), then a direct code commit (`crates/k/src/lib.rs`), then queue task B →
  `queue: refused: main-history: commit <sha> ("<subject>") on main after the last queue merge changes `crates/k/src/lib.rs` (M), which xtask/ledger.toml does not allow`.
  Without the fake line, the result is the same (control). With only ledger commits after A, B
  merges (control). Oracle: exact value.
- AC-2 (binding rules): a `merged` line that does not count as bound for each single reason (its
  commit also touches `tasks/`; subject differs; first parent ≠ `head_sha`; the commit appends
  two lines) does not move the anchor (shown as in AC-1). A record committed by hand with the
  exact hint command of (2), directly after the fast-forward, is bound. Oracle: exact value.
- AC-3 (bootstrap anchor): in a repository with no bound record and `history_anchor` set,
  commits after the anchor are checked (a direct code commit is refused). Without
  `history_anchor` and with no bound record, the refusal is the exact line in (1). Oracle: exact
  value.
- AC-4 (signing honoured): with `commit.gpgsign=true` and a working signing program (a test
  script set as `gpg.ssh.program`, `gpg.format=ssh`), the record commit is signed (`git log
  --format=%G?` is not `N`, or the program's invocation is observed). With a signing program
  that exits non-zero: the exit code is 3, stderr is the exact line of (2), `main` is at the
  task head, `metrics/queue.jsonl` ends with the `merged` record, and running the hint command
  (signing disabled) yields a bound record (AC-2). Oracle: exact value.
- AC-5 (record pathspec): while main's gate runs, a test gate stages an unrelated file in the
  primary checkout. After the merge, the record commit's changed paths are exactly
  `metrics/queue.jsonl`, and the unrelated file is still staged and not committed. Oracle:
  exact value (kills reviewer mutant P09).
- AC-6 (identity fallback): with no `user.name`/`user.email` configured and no `GIT_COMMITTER_*`
  variables, a merge records a commit whose author and committer are
  `kiln queue <queue@kiln.invalid>`, and the exit code is 0. With an identity configured, the
  commit carries that identity. Oracle: exact value (kills P14).
- AC-7 (lint escapes): a medium-tier task that:
  - adds a member crate without `[lints] workspace = true` → refused `gate-path` naming its
    `Cargo.toml`;
  - adds `#![allow(clippy::unwrap_used)]` to a `src/lib.rs` → refused `gate-path` naming that
    file;
  - adds `#[cfg_attr(test, allow(dead_code))]` → refused;
  - adds `#[expect(unused)]` → refused.

  The same changes on a high-tier task with an approval containing `gate paths` merge. A task
  that edits other lines of a file whose `#[allow]` attribute already exists on the merge base
  (as `main` has three today) is not refused. Oracle: exact value.
- AC-8 (untracked drafts): untracked `tasks/T-099-draft.md` and `reviews/T-099-r1.md` on main
  while queueing T-001 → the queue runs (merged), and both files are still untracked and
  unchanged afterwards. An untracked `tasks/T-001-x.md` (the queued task's own) or an untracked
  `metrics/extra.jsonl` → `queue: refused: ledger: …` naming that file. Oracle: exact value.
- AC-9 (join counts per crate): a repository with `crates/a/tests/acceptance_t901.rs` (3 tests,
  failing on base), `apps/b/tests/acceptance_t901.rs` (2 tests, failing on base) and
  `xtask/tests/acceptance_t901_x.rs` (1 test, failing on base), all passing after the join →
  record `acc_total: 6`, `acc_fail_base: 6`, `base_fail_kind: "fails"`,
  `acc_fail_first_join: 0`, `result: "joined"`. With one of the two same-named targets not
  compiling on base: `acc_fail_base: 6`, `base_fail_kind: "mixed"` or `"does_not_compile"`, as
  the existing classification rules give. Over all cases, `acc_fail_base ≤ acc_total`. A task
  whose only acceptance tests are in `xtask/tests` joins (`result: "joined"`, correct counts).
  Oracle: exact value.
- AC-10 (scorecard regeneration): a join whose tests branch adds a test name to an item's
  `tests` leaves `task/T-NNN` with a `join: regenerate docs/scorecard.md` commit changing only
  that file, and the joined gate's `scorecard` step passes. A join that changes no item makes no
  such commit. A queue run after a director ledger commit added `scorecard/items/new.toml` on
  main merges (gate green), with `docs/scorecard.md` regenerated in the merged tree and listed
  in the record's `regenerated`. Oracle: exact value.
- AC-11 (no regression): every T-005 acceptance and guard test passes unmodified; main's gate
  and the task's own gate are both green. Oracle: regression.

## Scorecard items and intended test names
None (tooling). Tests in `xtask/tests/acceptance_t009*.rs` (`xtask::acceptance_t009::<fn>`), for
example `fake_merged_line_does_not_move_the_anchor`, `record_commit_failure_after_merge_exits_3`,
`record_commit_takes_only_the_queue_log`, `queue_without_identity_uses_fallback`,
`lint_escapes_are_gate_paths`, `untracked_drafts_are_tolerated`,
`join_counts_same_named_targets_in_two_crates`, `join_regenerates_scorecard_file`.

## Out of scope
- Fixture projects under `tests/fixtures/**` escalated to tier high (r3 round-2 reading): a
  separate decision.
- Each failed queue attempt moving `main` by a record commit (r3 F25): as specified by R12, no
  change.
- Gated code writing to the primary checkout during the gate (F26), cached dependency
  artifacts, approval rows not tied to a commit: accepted residual risks (D-030). The binding of
  (1) removes F19 from that list; `docs/development.md` says so.
- Regenerating `docs/commands.json` or `Cargo.lock` outside generated-path conflicts.
- Any product crate, `Cargo.toml`, `Cargo.lock`, `.cargo/`, `.github/`.

## Implementer notes
- Your tests: unit tests in `xtask/src` (the existing toy-repository helpers in `git.rs`
  `testrepo`). Acceptance and guard files of T-005 are not edited.
- Specified: the rules, messages, exit codes and record fields above. Left to you: how bound
  commits are found, how attributes are detected (the `syn` dependency already exists), and how
  untracked files are classified.
- Never set or export `CARGO_TARGET_DIR` in commands you run by hand (D-019). Tests that need a
  target directory create their own.

## Questions
- Q-1 (planner, R-051): should the record commit honour the repository's signing configuration
  (specified, so `main` stays all-signed where that is policy, with exit 3 when signing fails),
  or always pass `--no-gpg-sign`, so that it never fails for signing but leaves unsigned
  commits on `main`? This environment has `commit.gpgsign=true` with an SSH signing program.
- Q-2 (planner, R-051): the `history_anchor` value. I propose `a981f182…` (the `head_sha` of
  T-005's `merged` record; the commits after it on `main` are ledger commits). If T-006 or T-008
  merges first, its record commit is bound and the anchor is used only as a fallback. Confirm
  the value, or name another.
- Q-3 (planner, R-051): should rule (3b) exempt `tests/` files (an `#![allow(dead_code)]` in a
  shared test-support file, as in `xtask/tests/acceptance_t005_support.rs`)? Specified: no
  exemption, so a medium task whose test author adds one needs tier high. An exemption limited
  to `allow(dead_code)` in `tests/**` is the narrow alternative.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · tokens (if known) · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13T03:05Z · R-051 · (new) · did: specified the seven follow-ups of D-033 /
  T-005 r3 plus the join items. Read the code:
  - `precheck.rs` main-history takes the newest committed `merged` line as the anchor, which
    confirms F19;
  - `queue.rs` `commit_record` bails after the fast-forward, so a merge is reported as an
    error (F20);
  - `queue.rs` `is_evidence_path` counts untracked drafts (F24);
  - `join.rs` `CargoOracle::acceptance` looks targets up by target name only (`find(|b|
    b.target == target)`), so a second package's same-named `acceptance_t004` target never
    runs. That is the likely cause of T-004's `acc_total` 6 vs `acc_fail_base` 13 (INFERRED;
    not reproduced);
  - `acceptance_targets` already searches `xtask/tests`, and AC-9 pins it;
  - the queue regenerates generated files only after a conflict.

  Judged (7) sound for `docs/scorecard.md` only, at join and after every queue rebase: a new
  item committed as a ledger commit (for example `ws.path-confinement`) otherwise leaves
  main's gate red at `scorecard`. Bootstrap anchor via `history_anchor` (Q-2). Tier high:
  gate paths; the approval needs `gate paths`. No dependency change. · gate attempts: 0 · not
  run: gate; did not reproduce the join miscount · missing context: none · next: director
  answers Q-1…Q-3.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
