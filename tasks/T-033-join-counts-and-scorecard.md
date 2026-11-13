---
id: T-033
title: "Make join count acceptance tests per package, including xtask/tests, and regenerate docs/scorecard.md on the task branch at join"
tier: high              # ADR-0002 "any gate path": xtask/src, xtask/tests; the approval row must contain `gate paths`
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-005, T-013]   # both merged
touches:
  - xtask/src/join.rs            # GATE PATH: per-package identity (1), scorecard regeneration (2)
  - xtask/src/testbins.rs        # GATE PATH: only if the test-binary list must carry more than it does (it already has `package`)
  - xtask/tests/acceptance_t033.rs   # GATE PATH: test author
  - docs/development.md          # "Join" section (not a gate path)
budget_tokens: 850000
iteration: 2
found_by: reviewer
origin_task: T-005        # D-033 (T-005 r3 follow-ups) and the T-004 join miscount (D-025)
---

# T-033 Join counts per package; scorecard file regenerated at join

## Goal
Part B of R-109's T-009 (split approved, R-112 brief). After this task:
(1) `cargo xtask join` counts acceptance tests correctly when two packages have a target of
the same name, and joins tasks whose tests live in `xtask/tests`;
(2) `join` regenerates `docs/scorecard.md` on the task branch, never on `main`, so the director
stops doing it by hand (T-009 Decisions D-5).

**Gate paths.** `xtask/src/join.rs`, `xtask/src/testbins.rs` and `xtask/tests/` are gate paths,
so the task is tier high (ADR-0002); the approval row must begin `Approve T-033` and contain
`gate paths`. No dependency change. Per T-005 D-3, it must pass both main's gate and its own.
If another `xtask/src` file must change, stop and report.

**Dispatch.** Never in flight with T-009, T-034 or T-028: all four change
`docs/development.md`; no `xtask/src` file is shared with them. No logical dependency on T-009
or T-034.

Reading list: `docs/decisions.md` D-019, D-021, D-025 (join counts), D-033, D-036;
`tasks/T-009-merge-queue-follow-ups.md` Decisions D-5 (R-057) and its R-051 log entry (the
miscount reading); `xtask/src/{join,testbins,scorecard}.rs`; `docs/development.md` "Join"; the
T-004 `join` records in `metrics/tasks.jsonl` (for example `acc_total: 6`, `acc_fail_base: 13`
at 2026-11-13T01:18:27Z).

## Interface
Existing commands keep their names, options, exit codes and record formats unless a change is
listed here.

### (1) Join counts
An acceptance test is identified by **(package, target, test name)**, not by target name, so
`crates/kiln-control/tests/acceptance_t004.rs` and `apps/kiln-cli/tests/acceptance_t004.rs`
are two targets. In the `join` record:
- `acc_total` = acceptance tests on the joined branch, across all packages, including
  `xtask/tests`;
- `acc_fail_base` = how many of those did not pass on base, counting failed tests plus the
  tests of targets that did not compile on base;
- therefore `acc_fail_base ≤ acc_total` always.

The guard counts (`guard_total`, `guard_fail_base`, `guard_fail_join`) use the same identity.
Record field names are unchanged (METHODOLOGY §13). Today `join.rs` `CargoOracle::acceptance`
finds a test binary by target name only (`find(|b| b.kind == "test" && &b.target == target)`),
so a second package's same-named target never runs; `testbins.rs`'s binaries already carry a
`package` field.

### (2) Scorecard file regenerated at join (T-009 D-5)
After the merge of `task/T-NNN-tests` into `task/T-NNN` (join step 3) and before the acceptance
run, `join` runs the `scorecard` command of the xtask it was built from in `.worktrees/T-NNN`. If
`docs/scorecard.md` changed, it commits only that file on `task/T-NNN`, subject
`join: regenerate docs/scorecard.md`, body lines `Task: T-NNN` and `Role: director`, with the
repository's commit configuration (signing kept). If nothing changed, no commit. It never
touches `main` or the primary checkout. If the commit fails, `join` exits 2 with
`join: regenerating docs/scorecard.md failed: <error>` and writes no `join` record. The record's
`joined_sha` is the head after this commit. The `review` precondition already allows commits
that change only generated paths after `reviewed_sha`, so a later re-join does not stale a
review. (`publish` already regenerates `docs/scorecard.md` on its own task branch in its step
4; this item is for ordinary tasks.)

## Pinned
- (a) **Errors and precedence.** Join's existing order and exit codes; new: the exit-2 line of
  (2), before any record is written.
- (b) **Files.** Appends `join` records to `metrics/tasks.jsonl` with unchanged field names;
  commits `docs/scorecard.md` on `task/T-NNN` only. Test file `xtask/tests/acceptance_t033.rs`.
- (c) **Order of checks.** Join steps as today, with (2) between the tests-branch merge and the
  acceptance run.
- (d) **Paths and platform.** Acceptance and guard targets in `crates/*/tests/`, `apps/*/tests/`,
  `xtask/tests/` (as `acceptance_targets` finds them today); Linux, git ≥ 2.38.
- (e) **Threat model.** D-026 (honest mistakes). A miscount is an evidence error, not a
  security hole; (1) makes the record match what ran.
- (f) **Tests pinning behaviour this changes.** Every T-005 and T-013 acceptance and guard test
  passes unmodified (`acceptance_t005*` join tests use single-package targets; `acceptance_t013_report.rs`
  reads join records, whose fields are unchanged). Unit tests in `join.rs` that look a target
  up by name change with the code (disclose in the log).
- (g) **Worked examples.** T-004's join: `acc_total` 6 and `acc_fail_base` 13 are consistent
  with the by-name lookup (INFERRED by R-051's reading, not reproduced). AC-1's toy counts
  follow from the identity rule: 3 + 2 + 1 = 6 tests, all failing on base → `acc_fail_base` 6.
- (h) **Scorecard items.** None.

## Acceptance criteria
In throw-away git repositories that the tests build (as T-005's acceptance tests do), with
`GIT_CONFIG_GLOBAL`/`HOME` isolated; all through the `xtask` binary. Oracle: exact value unless
stated. All are **acceptance** tests in `xtask/tests/acceptance_t033.rs` (fail on base) except
AC-3. Counts are of the toy repository the test builds, never of this repository (D-043).

- AC-1 (join counts per package; acceptance): a repository with
  `crates/a/tests/acceptance_t901.rs` (3 tests), `apps/b/tests/acceptance_t901.rs` (2 tests) and
  `xtask/tests/acceptance_t901_x.rs` (1 test), all failing on base and passing after the join →
  record `acc_total: 6`, `acc_fail_base: 6`, `base_fail_kind: "fails"`, `acc_fail_first_join: 0`,
  `result: "joined"`. If one of the two same-named targets breaks after the join,
  `acc_fail_first_join` counts its failures and `result` is `acceptance_failed` (shows both
  targets run on the joined branch). With one same-named target not compiling on base:
  `acc_fail_base: 6`, `base_fail_kind` `"mixed"` as the existing rules give. In every case
  `acc_fail_base ≤ acc_total`. A task whose only acceptance tests are in `xtask/tests` joins
  with correct counts. The same identity for two same-named `guard_t901` targets: both run on
  base and on the joined branch (`guard_total` counts both).
- AC-2 (join regenerates the scorecard file; acceptance): a join whose tests branch adds test
  names to an item's `tests` (status `failing` on the tests branch, `implemented` after the
  merge) leaves `task/T-NNN` with one `join: regenerate docs/scorecard.md` commit that changes
  only that file and carries `Task:`/`Role: director`; the joined gate's `scorecard` step
  passes; `main` and the primary checkout are unchanged (same sha, same status). A join that
  changes no status makes no such commit. With a `commit-msg` hook that rejects that subject:
  exit 2, the line of (2), no `join` record appended.
- AC-3 (no regression; guard by nature): every T-005 and T-013 acceptance and guard test
  passes unmodified; main's gate and the task's own gate are both green.

## Out of scope and implementer notes
- Regenerating `docs/scorecard.md` in the queue (withdrawn, D-036); `docs/commands.json` or
  `Cargo.lock` outside conflicts.
- Queue preconditions, record commits, untracked drafts, approval: T-009. Lint escapes and
  publish: T-034.
- Your tests: unit tests in `xtask/src` (toy-repository helpers in `git.rs` `testrepo`).
  Acceptance and guard files of T-005 and T-013 are not edited.
- Specified: identity, counts, the regeneration step, its commit and its failure line. Left to
  you: how binaries are matched to (package, target).
- Never set or export `CARGO_TARGET_DIR` in commands you run by hand (D-019).

## Questions and rulings
None open.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13T12:50Z · R-112 · (new) · did: created from part B of R-109's T-009 (items (6),
  (7), AC-9, AC-11; split approved by the director). Checked against main: `join.rs` still
  matches binaries by target name; `testbins.rs` binaries already carry `package`, so that file
  is in `touches` only conditionally. Added the guard-identity case to AC-1 and a commit
  failure case to AC-2 (the (2) failure line had no test). Tier high: gate paths
  (`xtask/src/join.rs`, `xtask/tests`). Pinned (h) n/a: no scorecard item. Not in flight with
  T-009, T-034, T-028 (`docs/development.md`). · gate attempts: 0 · not run: no cargo; the
  T-004 miscount is still not reproduced · missing context: none · next: director; spec check.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
