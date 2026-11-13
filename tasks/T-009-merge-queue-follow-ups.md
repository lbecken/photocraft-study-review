---
id: T-009
title: "Close the merge queue's remaining gaps: bind merge records to their commits, keep signing and report a failed record commit exactly, treat lint escapes correctly, tolerate drafts, count acceptance tests per package"
tier: high              # ADR-0002 "any gate path": xtask/src, xtask/ledger.toml; the approval row must contain `gate paths`
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-005, T-013]   # T-013 first: both touch xtask/src and docs/development.md (see "Order with T-013")
touches:
  - xtask/src                    # GATE PATH
  - xtask/ledger.toml            # GATE PATH: new `history_anchor` key
  - docs/development.md          # queue, join and residual-risk sections (not a gate path)
budget_tokens: 850000
iteration: 1
found_by: reviewer
origin_task: T-005
---

# T-009 Close the merge queue's remaining gaps

## Goal
T-005 made `cargo xtask queue` the only way code reaches `main`. Its final review (r3, findings
19–24) and the T-003/T-004 joins left six gaps (D-033). After this task:
(1) a `merged` line in `metrics/queue.jsonl` counts as evidence only if the queue's own record
commit wrote it;
(2) the record commit keeps the repository's signing configuration, and when it fails after a
merge the queue says exactly how to commit the record by hand instead of reporting a failed
merge;
(3) a crate that drops the workspace lints is a gate change; a lint `allow`/`expect` attribute
added in non-test source is listed for the reviewer; one added in a test file is neither;
(4) the record-commit pathspec and the identity fallback have tests;
(5) untracked drafts no longer block the queue unless they are the queued task's own evidence;
(6) `join` counts acceptance tests correctly when two packages have a target of the same name,
and joins tasks whose tests live in `xtask/tests`.

The former item (7), regenerating `docs/scorecard.md` at join and after every queue rebase, is
removed: D-036 withdrew queue-side regeneration (it would leave `main`'s scorecard stale between
a ledger commit and the next queue run); ledger inputs that change generated output go through
T-013's publish helper instead (Decisions D-4, Q-4).

**Gate paths.** `xtask/src` and `xtask/ledger.toml` are gate paths, so the task is tier high
(ADR-0002). `docs/development.md` documents the list but is not itself a gate path. The
director's approval row in `docs/decisions.md` must begin `Approve T-009` and contain the words
`gate paths`, or the queue refuses the task (T-005 R3). The task changes no dependency (no
`Cargo.toml`, `Cargo.lock`, lint table or build key). Per T-005 D-3, it must pass both main's
gate and its own.

Reading list: `docs/decisions.md` D-019, D-021, D-025 (join counts), D-026 (threat model),
D-030 (R10–R13), D-033, D-036; `reviews/T-005-r3.md` findings 19–26 and its follow-up list;
`tasks/T-005-queue-hardening.md` Interface and Decisions R1–R13 (exit codes, refusals, record
format, precondition order); `tasks/T-013-process-tooling-v11.md` Interface (2) (the `budget`
precondition this task's precondition list must keep); `xtask/src/{precheck,queue,join,policy}.rs`;
`xtask/ledger.toml`; `docs/development.md` "Merge queue" and "Join"; the T-004 `join` records in
`metrics/tasks.jsonl` (for example `acc_total: 6`, `acc_fail_base: 13` at 2026-11-13T01:18:27Z).

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
`history_anchor = "<40-hex sha>"`, set by this task to
`a981f18210cbd59be20e5e7009b9ed8429850b64`, the `head_sha` of T-005's `merged` record
(Decisions D-2). On today's `main` the T-012 and T-014 records are already bound (record commits
`3528ae9` and `ecb6500` each have the task head as first parent and change only
`metrics/queue.jsonl`), so the key is a fallback. If neither exists, the result is
`queue: refused: main-history: no merged record is bound to a queue record commit and xtask/ledger.toml names no history_anchor`.
The per-commit ledger check from the anchor to `main` is unchanged.

### (2) Record commit and commit signing (Decisions D-1)
The record commit uses the repository's commit configuration unchanged, including
`commit.gpgsign`; there is no option to skip signing. It still uses `--no-verify`, the identity
fallback, the pathspec `-- metrics/queue.jsonl`, and `Task:`/`Role: director` lines. If the
fast-forward succeeded and the record commit then fails, for any reason:
- the appended record (result `merged`) stays, uncommitted, in `metrics/queue.jsonl`;
- stderr gets exactly one line:
  `queue: merged T-NNN at <head_sha>, but the record commit failed: <git error>; commit it by hand: git commit -m "queue: record T-NNN merged" -m "Task: T-NNN" -m "Role: director" -- metrics/queue.jsonl`
  (that command produces a bound commit, see (1));
- the queue stops with exit code **3**: merged but not recorded. Exit 0 stays merged and
  recorded, 1 not merged, 2 an error.

If the record commit fails after a non-merged outcome, the exit stays 1 and stderr gets the same
line with that result (`queue: T-NNN <result>, but the record commit failed: …; commit it by hand: git commit -m "queue: record T-NNN <result>" …`).
In every case the next queue run is refused at `ledger` (uncommitted `metrics/` change) until
the record is committed, as today.

### (3) Lint escapes (Decisions D-3)
- (a) **Gate path.** The `Cargo.toml` of a workspace member counts as a gate path (so the task
  must be tier high with an approval containing `gate paths`, as R10) when, in the task head, it
  has no `[lints]` table with exactly `workspace = true` and the merge base's copy did (an
  existing member that drops it) or the file is new at the head (a new member that omits it).
- (b) **Listed, not refused.** A `.rs` file outside every test directory in which the task
  **adds** a lint-suppressing attribute (`allow` or `expect`, at item or crate level, `#[…]` or
  `#![…]`, including inside `cfg_attr(…)`) is not a gate path. The queue prints, for each added
  attribute, one stderr line `queue: T-NNN adds a lint attribute in non-test source: <path>:<line>: <attribute text>`
  and lists them in its summary as `lint attributes added: <path>:<line>, …`. It does not refuse.
  The record format is unchanged.
- (c) **Test files.** An attribute added in a `.rs` file under a `tests/` directory directly
  below a package root (`crates/*/tests/**`, `apps/*/tests/**`, `xtask/tests/**`) is neither a
  gate change nor listed.
- "Added" compares the merge base with the task head: attributes that exist on the merge base do
  not count; nor do moved lines whose attribute text is unchanged (how moves are detected is left
  to the implementer, as long as an added attribute is never missed).

### (4) Tests for two untested paths
No behaviour change: the record commit's pathspec and the identity fallback get acceptance tests
(AC-5, AC-6), which kill reviewer mutants P09 and P14 of T-005 r3.

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

## Pinned
- (a) **Errors and precedence.** Precondition order unchanged: `task-file`, `front-matter`,
  `state`, `config`, `touches`, `join`, `review`, `gate-path`, `dependencies`, `approval`,
  `budget` (T-013), `main-history`; (3a) is part of `gate-path`, (1) of `main-history`. Exit
  codes: 0 merged and recorded, 1 refused or not merged, 2 error, 3 merged but not recorded
  (new). The exact lines are in (1), (2), (3b).
- (b) **Files.** Reads main's committed `metrics/queue.jsonl`, `xtask/ledger.toml` (new optional
  key `history_anchor`, a 40-hex lowercase sha; any other new key stays a `config` error),
  `docs/decisions.md`; appends one line to `metrics/queue.jsonl` and commits it; appends `join`
  records to `metrics/tasks.jsonl` with unchanged field names.
- (c) **Order of checks.** As (a); within (1), binding is decided per record from newest to
  oldest and the first bound record wins; within (5), the tracked-evidence rule is checked before
  the untracked rules.
- (d) **Paths and platform.** Repository-relative paths; test directories as in (3c); Linux, git
  ≥ 2.38.
- (e) **Threat model.** D-026: honest mistakes and corner-cutting, not a malicious agent. (1)
  closes F19 (a fake `merged` line in a ledger commit). Still accepted (D-030, listed in
  `docs/development.md`): gated code writing to the primary checkout during the gate (F26),
  cached dependency artifacts, approval rows not tied to a commit; a lint attribute added in
  non-test source merges (it is only listed); a director who commits a fabricated line with the
  exact hint command right after a real merge is indistinguishable from the real record.
- (f) **Tests pinning behaviour this changes.** Every T-005 acceptance and guard test must pass
  unmodified. None asserts that untracked drafts are refused (grep `untracked` in
  `xtask/tests`); `acceptance_t005_r3.rs:828` asserts that an untracked `metrics/tasks.jsonl` is
  refused at `ledger`, which (5) keeps. None asserts a record-commit failure's exit code. Unit
  tests in `xtask/src` that pin the old anchor or old exit code change with the code (disclose in
  the log). T-013's acceptance tests (merged before this task) must pass unmodified.
- (g) **Worked examples.** History `A(merged T-001, record commit) → L1 (ledger commit adding a
  fake merged line for T-099 with head_sha = L1's parent) → X (direct code commit)`: the anchor
  stays T-001's head, so X is checked and refused (AC-1). T-004's join at
  2026-11-13T01:18:27Z: `acc_total` 6 and `acc_fail_base` 13 are consistent with the bug the
  planner of R-051 read in `join.rs` (`CargoOracle::acceptance` looks a target up by name only,
  so a second package's `acceptance_t004` target is not run on the joined branch): INFERRED, not
  reproduced; AC-9 fixes the behaviour independent of the cause.
- (h) **Scorecard items.** None.

## Acceptance criteria
In throw-away git repositories that the tests build (as T-005's acceptance tests do), with
`GIT_CONFIG_GLOBAL`/`HOME` isolated so the host's signing and identity settings do not leak in
unless a case sets them; all through the `xtask` binary. Oracle: exact value (exit code, the one
refusal line, record contents, `git log`/`git show` output) unless stated.

- AC-1 (fake merged line ignored): after a real queue merge of task A, commit on main a ledger
  commit (`tasks/x.md` plus a fabricated `merged` line for task Z with `head_sha` = current
  `main`), then a direct code commit (`crates/k/src/lib.rs`), then queue task B →
  `queue: refused: main-history: commit <sha> ("<subject>") on main after the last queue merge changes `crates/k/src/lib.rs` (M), which xtask/ledger.toml does not allow`.
  Without the fake line, the result is the same (control). With only ledger commits after A, B
  merges (control).
- AC-2 (binding rules): a `merged` line that is unbound for each single reason (its commit also
  touches `tasks/`; subject differs; first parent ≠ `head_sha`; the commit appends two lines)
  does not move the anchor (shown as in AC-1). A record committed by hand with the exact hint
  command of (2), directly after the fast-forward, is bound.
- AC-3 (bootstrap anchor): with no bound record and `history_anchor` set, commits after the
  anchor are checked (a direct code commit is refused); without `history_anchor` and with no
  bound record, the refusal is the exact line in (1); a malformed `history_anchor` → `config`.
- AC-4 (signing kept, failure reported): with `commit.gpgsign=true`, `gpg.format=ssh` and a
  test script as `gpg.ssh.program` that signs, the record commit is signed (the program's
  invocation is observed, or `git log --format=%G?` is not `N`). With a program that exits
  non-zero: exit code 3, stderr is exactly the line of (2), `main` is at the task head,
  `metrics/queue.jsonl` ends with the uncommitted `merged` record, and the next `queue` run of
  any task is refused at `ledger`. After the test repairs the signing program, the printed
  command creates a bound record (AC-2) and the next run is no longer refused at `ledger`.
- AC-5 (record pathspec): while main's gate runs, a test gate stages an unrelated file in the
  primary checkout. After the merge, the record commit's changed paths are exactly
  `metrics/queue.jsonl`, and the unrelated file is still staged and not committed (kills P09).
- AC-6 (identity fallback): with no `user.name`/`user.email` and no `GIT_COMMITTER_*`
  variables, a merge records a commit whose author and committer are
  `kiln queue <queue@kiln.invalid>`, exit 0. With an identity configured, the commit carries
  that identity (kills P14).
- AC-7 (lint attributes): a medium-tier task that adds a member crate without
  `[lints] workspace = true`, or removes that table from an existing member → refused at
  `gate-path` naming that `Cargo.toml`; the same on a high task with a `gate paths` approval
  merges. A medium task that adds `#![allow(clippy::unwrap_used)]` to `crates/a/src/lib.rs`,
  `#[cfg_attr(test, allow(dead_code))]` and `#[expect(unused)]` to another source file →
  merges, and stderr has exactly one listing line per attribute with the right `<path>:<line>`;
  the summary lists them. A medium task that adds `#![allow(dead_code)]` to
  `crates/a/tests/support.rs` → merges with no listing line. Editing other lines of a file whose
  `#[allow]` already exists on the merge base → no listing line.
- AC-8 (untracked drafts): untracked `tasks/T-099-draft.md` and `reviews/T-099-r1.md` on main
  while queueing T-001 → merged, and both files are still untracked and byte-identical
  afterwards. An untracked `tasks/T-001-x.md` (the queued task's own) or an untracked
  `metrics/extra.jsonl` → `queue: refused: ledger: …` naming that file.
- AC-9 (join counts per package): a repository with `crates/a/tests/acceptance_t901.rs` (3
  tests), `apps/b/tests/acceptance_t901.rs` (2 tests) and `xtask/tests/acceptance_t901_x.rs` (1
  test), all failing on base and passing after the join → record `acc_total: 6`,
  `acc_fail_base: 6`, `base_fail_kind: "fails"`, `acc_fail_first_join: 0`, `result: "joined"`.
  If one of the two same-named targets breaks after the join, `acc_fail_first_join` counts its
  failures and `result` is `acceptance_failed` (shows both targets run on the joined branch).
  With one same-named target not compiling on base: `acc_fail_base: 6`, `base_fail_kind`
  `"mixed"` as the existing rules give. In every case `acc_fail_base ≤ acc_total`. A task whose
  only acceptance tests are in `xtask/tests` joins with correct counts.
- AC-10 (no regression): every T-005 and T-013 acceptance and guard test passes unmodified;
  main's gate and the task's own gate are both green. Oracle: regression.

## Out of scope and implementer notes
- Regenerating `docs/scorecard.md` at join or in the queue outside generated-path conflicts
  (withdrawn, D-036; publish helper in T-013; see Q-4). Regenerating `docs/commands.json` or
  `Cargo.lock` outside conflicts.
- Fixture projects under `tests/fixtures/**` escalated to tier high (r3 round-2 reading): a
  separate decision. Each failed queue attempt moving `main` by a record commit (r3 F25): as R12,
  no change.
- Metrics, budget, worktree state, publish, review lint, session paths: T-013.
- Any product crate, `Cargo.toml`, `Cargo.lock`, `.cargo/`, `.github/`, `methodology/`.
- Your tests: unit tests in `xtask/src` (the existing toy-repository helpers in `git.rs`
  `testrepo`). Acceptance and guard files of T-005 and T-013 are not edited.
- Specified: the rules, messages, exit codes and record fields above. Left to you: how bound
  commits are found, how attributes are detected (the `syn` dependency already exists), how
  untracked files are classified.
- Never set or export `CARGO_TARGET_DIR` in commands you run by hand (D-019). Tests that need a
  target directory create their own.

## Order with T-013
Both change `xtask/src` (`queue.rs`/`precheck.rs`, `join.rs`, `main.rs`) and
`docs/development.md`, so their `touches` overlap and they may not be in flight together
(METHODOLOGY §5.2). T-013 lands first (reasons in T-013 "Order with T-009"); this task is
dispatched after T-013 is `merged` and rebases nothing by hand: its worktree starts from a `main`
that already has the `budget` precondition and the publish helper. T-013's helper passes the
queue's exit code through, so exit 3 needs no change there. The cross-crate join count stays
here, not in T-013 (it is the join record's producer; T-013 only reads records).

## Decisions
Director's answers to the planner's questions of R-051 (2026-11-13, recorded by R-055).
- D-1 (was Q-1): the record commit keeps whatever signing the repository is configured to
  require; there is no bypass flag. If the commit fails, the queue stops with an error that says
  exactly how to commit the record by hand, and the next run refuses at `ledger` until that is
  done. (Specified in (2) as the one stderr line and exit 3 after a merge; see Q-5.)
- D-2 (was Q-2): `history_anchor` is the head recorded by the merged T-005 queue record,
  `a981f18210cbd59be20e5e7009b9ed8429850b64`.
- D-3 (was Q-3): a lint `allow` attribute added in a test file is not a gate change. One added in
  non-test source is listed in the queue's output for the reviewer but does not refuse the run.
  (Specified in (3b), (3c); `expect` is treated like `allow`, since it suppresses the lint too.)
- D-4: the former item (7), the queue regenerating `docs/scorecard.md` (at join time and after
  every rebase), is removed; D-036 withdrew it.

## Questions and rulings
- Q-4 (planner, R-055): removing (7) also removes the **join-side** regeneration, which D-036's
  reason does not cover: at a join, items whose tests start passing change status
  (`failing`/`planned` → `implemented`) on `task/T-NNN`, so `docs/scorecard.md` is stale on the
  task branch and its gate is red at `scorecard` until someone regenerates it there (retro-0
  "Tooling and environment": 4 manual regenerations, T-001…T-004). `main` is never affected,
  because the branch reaches `main` only through the queue. Should `join` regenerate and commit
  `docs/scorecard.md` on `task/T-NNN` (subject `join: regenerate docs/scorecard.md`,
  `Role: director`), or does the director keep doing it by hand? Specified: removed, as asked.
- Q-5 (planner, R-055): D-1 says the queue "stops with an error". Specified as exit 3 (merged but
  not recorded), kept distinct from exit 2 (error, nothing merged), so a caller can tell that
  `main` moved. If you meant exit 2, only (2), Pinned (a) and AC-4 change.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

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
- 2026-11-13T03:50Z · R-055 · draft · did: brought the spec to v1.1: re-tiered high by ADR-0002
  (criterion: gate paths `xtask/src`, `xtask/ledger.toml`; `docs/development.md` is not one, so
  the R-051 comment "every path except docs/development.md" is kept in substance); budget 850k;
  Pinned section; recorded the director's answers as Decisions D-1…D-3; removed item (7) (D-4,
  D-036) with its AC-10 and test name; rewrote (3) and AC-7 for D-3 (listing instead of refusal,
  test files exempt); added the `Task:`/`Role:` lines to the hand command so a hand commit
  matches the queue's own; checked that the T-012 and T-014 records on `main` are already bound
  (`git show ecb6500 --stat`, parent `13e1b2c` = T-014 head); kept the cross-crate join fix here
  and set `depends_on` T-013 (shared files). Pinned (h) n/a: no scorecard item. Arm: n/a (high,
  independent test author). · gate attempts: 0 · not run: gate, the join miscount reproduction ·
  missing context: none · next: director answers Q-4, Q-5.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
