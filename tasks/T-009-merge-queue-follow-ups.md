---
id: T-009
title: "Close the merge queue's remaining gaps: bind merge records to their commits, report a failed record commit exactly, treat lint escapes correctly, tolerate drafts, count acceptance tests per package, regenerate the scorecard file at join, finish the publish helper's follow-ups and name a near-miss approval row"
tier: high              # ADR-0002 "any gate path": xtask/src, xtask/tests, xtask/ledger.toml; the approval row must contain `gate paths`
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-005, T-013]   # both merged (T-013 `3ef7600`, D-055)
touches:
  - xtask/src                    # GATE PATH
  - xtask/ledger.toml            # GATE PATH: new `history_anchor` key
  - xtask/tests/acceptance_t009.rs   # GATE PATH: test author
  - xtask/tests/guard_t009.rs        # GATE PATH: test author (R-109: guards for behaviour main already has)
  - docs/development.md          # queue, join, publish and residual-risk sections (not a gate path)
budget_tokens: 850000
iteration: 1
found_by: reviewer
origin_task: T-005
---

# T-009 Close the merge queue's remaining gaps

## Goal
T-005 made `cargo xtask queue` the only way code reaches `main`. Its final review (r3, findings
19–24) and the T-003/T-004 joins left gaps (D-033); the T-013 round-2 review left five
follow-ups (D-053, `reviews/T-013-r2.md` "Follow-ups"); and D-058 found a refusal that did not
say what it saw. After this task:
(1) a `merged` line in `metrics/queue.jsonl` counts as evidence only if the queue's own record
commit wrote it;
(2) when the record commit fails after a merge the queue says exactly how to commit the record
by hand and exits 3 instead of reporting a failed merge, and `publish` reports that case as a
merge, not as "did not merge";
(3) a new workspace member that omits the workspace lints is a gate change; a lint
`allow`/`expect` attribute added in non-test source is listed for the reviewer; one added in a
test file is neither;
(4) the record-commit pathspec, the identity fallback and the kept signing configuration have
regression guards;
(5) untracked drafts no longer block the queue unless they are the queued task's own evidence;
(6) `join` counts acceptance tests correctly when two packages have a target of the same name,
and joins tasks whose tests live in `xtask/tests`;
(7) `join` regenerates `docs/scorecard.md` on the task branch, never on `main`, so the director
stops doing it by hand (Decisions D-5);
(8) every `publish: commit failed:` line gives a hand command that works, also when `git add`
was what failed (T-013 r2 follow-up 2);
(9) the publish exit codes are stated once and correctly, and a failed content commit in step 4
is pinned at exit 1 (follow-up 3);
(10) a failing transition or state edit in publish steps 3, 6 or 8 ends with publish's own stop
line instead of a generic error (follow-up 4);
(11) the publish paths T-013 left untested have tests (follow-up 1);
(12) the dead `_` separator check in the ceiling parser is removed or explained (follow-up 5);
(13) an `approval` refusal names every row whose Decision cell contains `Approve T-NNN` but does
not begin with it (D-058).

Regeneration by the queue (after every rebase) stays out: D-036 withdrew it; ledger inputs that
change generated output go through T-013's `cargo xtask publish` (merged).

**Gate paths.** `xtask/src`, `xtask/tests` and `xtask/ledger.toml` are gate paths, so the task
is tier high (ADR-0002). `docs/development.md` documents the list but is not itself a gate
path. The director's approval row in `docs/decisions.md` must begin `Approve T-009` and contain
the words `gate paths`, or the queue refuses the task (T-005 R3). The task changes no
dependency (no `Cargo.toml`, `Cargo.lock`, lint table or build key). Per T-005 D-3, it must
pass both main's gate and its own.

**Size (R-109).** With (8)–(13) the task holds thirteen items; T-013, with seven, cost
1,790,657 tokens against 850,000 (D-054). This file is one reviewable spec, but it is larger
than one high-tier budget; the planner proposes to the director to split it along the groups
below (the groups share no item and each is testable alone; no new task file is created here):
- **A, queue:** (1), (2), (4), (5), (13), with AC-1…AC-6, AC-8, AC-10, AC-15, AC-16.
- **B, join:** (6), (7), with AC-9, AC-11.
- **C, lint and publish:** (3), (8)–(12), with AC-7, AC-12…AC-14.
(2)'s publish half (AC-16) needs (2)'s exit 3, so it stays in A.

Reading list: `docs/decisions.md` D-019, D-021, D-025 (join counts), D-026 (threat model),
D-030 (R10–R13), D-033, D-036, D-047 (rulings j–q), D-053, D-054, D-058;
`reviews/T-005-r3.md` findings 19–26 and its follow-up list; `reviews/T-013-r2.md` findings and
"Follow-ups"; `tasks/T-005-queue-hardening.md` Interface and Decisions R1–R13 (exit codes,
refusals, record format, precondition order); `tasks/T-013-process-tooling-v11.md` Interface
(2) (`budget`), (5) (`publish`) and rulings j–q; `xtask/src/{precheck,queue,join,publish,policy}.rs`;
`xtask/ledger.toml`; `docs/development.md` "Merge queue", "Join", "Publishing a ledger input"
and "After a stop"; the T-004 `join` records in `metrics/tasks.jsonl` (for example
`acc_total: 6`, `acc_fail_base: 13` at 2026-11-13T01:18:27Z).

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
(Decisions D-2). On today's `main` (`e614fee`) every `merged` record from T-012 on is already
bound: the record commits of T-012, T-014, T-006, T-015, T-013, T-016 and T-032 each have the
task head as first parent and change only `metrics/queue.jsonl` (`git log --first-parent
--grep='^queue: record .* merged'` with `%P` and `--name-only`; for example `0ed8456`, parent
`cb84134` = T-032's head), so the key is a fallback. If neither exists, the result is
`queue: refused: main-history: no merged record is bound to a queue record commit and xtask/ledger.toml names no history_anchor`.
The per-commit ledger check from the anchor to `main` is unchanged. (Today, `precheck.rs`
takes the newest committed `merged` line as the anchor, bound or not.)

### (2) Record commit failure (Decisions D-1, D-6)
The record commit uses the repository's commit configuration unchanged, including
`commit.gpgsign`; there is no option to skip signing. This is already so on main
(`queue.rs` `commit_record` passes no signing override); (4) adds a guard for it. It still uses
`--no-verify`, the identity fallback, the pathspec `-- metrics/queue.jsonl`, and
`Task:`/`Role: director` lines. If the fast-forward succeeded and the record commit then fails,
for any reason (today: `xtask: … the record was appended … (commit it by hand): …`, exit 2):
- the appended record (result `merged`) stays, uncommitted, in `metrics/queue.jsonl`;
- stderr gets exactly one line:
  `queue: merged T-NNN at <head_sha>, but the record commit failed: <git error>; commit it by hand: git commit -m "queue: record T-NNN merged" -m "Task: T-NNN" -m "Role: director" -- metrics/queue.jsonl`
  (that command produces a bound commit, see (1));
- the queue stops with exit code **3**: merged but the record not committed (Decisions D-6).
  Exit 0 stays merged and recorded, 1 a refusal or a non-merged outcome, 2 a usage or tool
  error.

If the record commit fails after a non-merged outcome, the exit stays 1 and stderr gets the same
line with that result (`queue: T-NNN <result>, but the record commit failed: …; commit it by hand: git commit -m "queue: record T-NNN <result>" …`).
In every case the next queue run is refused at `ledger` (uncommitted `metrics/` change) until
the record is committed, as today.

**(2c) Publish after a queue exit 3.** `publish` step 8 today treats every non-zero queue exit
as "did not merge" (`publish: queue did not merge T-NNN (exit c); the task stays queued; …`),
which is false for exit 3. After this task, on queue exit 3 publish prints exactly one line of
its own after the queue's output,
`publish: the queue merged T-NNN but did not commit its record (exit 3); commit the record as the queue printed, then finish step 8 by hand (docs/development.md, "After a stop")`,
records no transition, makes no commit, leaves the worktree, and exits 3. Every other non-zero
exit keeps T-013's line and exit code. `docs/development.md` "After a stop" gains the row
"8, queue exit 3" (left behind: merged by the queue, its record uncommitted, `queued` committed;
finish: run the queue's printed command, then `metrics transition --task T-NNN --to merged`,
set state merged, ledger commit "publish: T-NNN merged", `worktree rm T-NNN`).

### (3) Lint escapes (Decisions D-3)
- (a) **Gate path.** Already on main (D-030 R10, `precheck.rs` `manifest_gate_keys`; guard
  `queue.rs` `member_lints_and_custom_build_scripts_are_gate_paths`): a member `Cargo.toml`
  whose `[lints]` table or package `build` key differs from both main's and the merge base's
  copy is a gate path, so an existing member that drops `[lints] workspace = true` is refused
  below tier high. **New:** a member `Cargo.toml` that is new at the task head (absent on the
  merge base) and has no `[lints]` table with exactly `workspace = true` is a gate path too
  (today an absent `[lints]` compares equal to an absent file). Nothing that is a gate path
  today stops being one.
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

### (4) Guards for three unchanged paths
No behaviour change: the record commit's pathspec, the identity fallback and the kept signing
configuration get regression guards in `xtask/tests/guard_t009.rs` (AC-4 first half, AC-5,
AC-6). They pass on base by design, which is why they are guards, not acceptance tests; they
kill reviewer mutants P09 and P14 of T-005 r3.

### (5) What `ledger` tolerates
`ledger` (and the dirty-tree check after it) refuses for:
- tracked evidence files with any uncommitted change (modified, deleted, staged, renamed), as
  today;
- untracked files under `metrics/`;
- untracked files that are the queued task's own evidence: `tasks/T-NNN-*.md` and
  `reviews/T-NNN-*`.

Every other untracked file is tolerated by the queue: other tasks' draft task files and
reviews, and any untracked file outside the evidence paths (today both are refused: `git
status --untracked-files=all` feeds `ledger` and the dirty-tree check). The queue never stages,
commits, moves or deletes an untracked file. The refusal message lists the files it refused
for. `publish`'s step-1 checks are not changed by this item: publish keeps refusing any dirty
path apart from its inputs (T-013 (5); its acceptance test with an untracked `notes.txt` stays
green unmodified).

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
Record field names are unchanged (METHODOLOGY §13). (Today `join.rs` `CargoOracle::acceptance`
still finds a test binary by target name only: `find(|b| b.kind == "test" && &b.target == target)`.)

### (7) Scorecard file regenerated at join (Decisions D-5)
After the merge of `task/T-NNN-tests` into `task/T-NNN` (join step 3) and before the acceptance
run, `join` runs the `scorecard` command of the xtask it was built from in `.worktrees/T-NNN`. If
`docs/scorecard.md` changed, it commits only that file on `task/T-NNN`, subject
`join: regenerate docs/scorecard.md`, body lines `Task: T-NNN` and `Role: director`, with the
repository's commit configuration (signing kept). If nothing changed, no commit. It never
touches `main` or the primary checkout. If the commit fails, `join` exits 2 with
`join: regenerating docs/scorecard.md failed: <error>` and writes no `join` record. The record's
`joined_sha` is the head after this commit. The `review` precondition already allows commits
that change only generated paths after `reviewed_sha`, so a later re-join does not stale a review.
(`publish` already regenerates `docs/scorecard.md` on its own task branch in its step 4; this
item is for ordinary tasks.)

### (8) Publish hand commands that work (T-013 r2 follow-up 2)
When a ledger commit of publish fails (steps 2, 3, 6 and 8; a failing `git add` included,
ruling D-047 (l)), the one stderr line becomes
`publish: commit failed: <git error>; stage by hand: git add -- <paths>; commit by hand: git commit -m "<subject>" -m "Task: T-NNN" -m "Role: director" -- <paths>`,
where `<paths>` is the same list as today, in the same order, in both commands. Today the line
has no `stage by hand` clause, so after a failed `git add` the printed `git commit -- <paths>`
names untracked files and fails with `pathspec … did not match` (R-097 toy p3). Exit code (2)
and everything else are unchanged. The line still starts with `publish: commit failed: ` and
still ends with `; commit by hand: git commit -m … -- <paths>`, so T-013's two acceptance tests
that check exactly that prefix and suffix stay green unmodified.

### (9) Publish exit codes, stated once (follow-up 3)
`docs/development.md` and this task state: refusal 1; usage error 2; a failed **ledger** commit
(steps 2, 3, 6, 8) 2 with the line of (8); a stop in step 3, 4 or 7 — a failed content commit
on `task/T-NNN` in step 4 included — 1 with `publish: stopped at step <n>: <error>; T-NNN is <state>`
(ruling D-047 (q)); a stop of (10) 1; the gate's red line 1; step 8 the queue's code (with (2c)
for 3). This is today's behaviour (R-097 confirmed it); T-013's Pinned (a) wording "publish
commit failure 2" is superseded by this sentence, and T-013's merged task file is not edited.

### (10) A failing transition or state edit in steps 3, 6, 8 (follow-up 4)
If, in publish step 3, 6 or 8, a `metrics/tasks.jsonl` transition or the edit of the task
file's `state:` fails (for example a transition refused by the time rules of `metrics`), publish
prints exactly one line of its own,
`publish: stopped at step <n>: <error>; T-NNN is <state>`, where `<state>` is the task's last
transition in the working tree's `metrics/tasks.jsonl` (as for the existing stop line), and
exits 1. Whatever that step already appended stays in the working tree, uncommitted; nothing
is reset. Today such a failure ends with a generic error and no stop line (R-097 finding 4).
`docs/development.md` "After a stop" gains one row per step ("3/6/8, a transition or the state
edit failed": left behind, finish by hand: complete the step's remaining transitions with
`metrics transition`, set the state, ledger commit with the step's subject).

### (11) Owed publish tests (follow-up 1)
No behaviour change beyond (8): the step-2 undo with an existing `metrics/tasks.jsonl` gets a
unit test (it cannot be reached through the command line: ruling D-047 (q)); the ledger commits
of steps 3 and 8 get acceptance tests of their failure line (AC-12).

### (12) Ceiling parser (follow-up 5)
`precheck.rs` `extension_of`: the `b'_'` arm of the separator check is unreachable (a digit run
followed by `_` is already rejected by the id-character test). Remove it or say so in a comment.
No behaviour change; T-013's ruling-j tests stay green unmodified.

### (13) Approval refusal names near-miss rows (D-058)
A row's **Decision cell** is found as today (D-026 R7: a table row outside code blocks and HTML
comments, in the column headed `Decision`). A cell **mentions** `Approve T-NNN` if it contains
that text followed by the cell's end or a character that cannot continue an id (the same id
rule as "begins with" today), anywhere other than at its start. When the `approval` check
refuses and at least one row mentions `Approve T-NNN` without beginning with it, the refusal's
reason is today's reason followed by
`; found `Approve T-NNN` inside, not at the start of, the Decision cell of: <ids>`, where
`<ids>` are those rows' first cells, trimmed, comma-separated in table order (for example
`D-057`). When no row mentions it, the line is exactly today's. The check's outcome (refuse or
pass) is unchanged: a row that does not begin with `Approve T-NNN` still never approves.

## Pinned
- (a) **Errors and precedence.** Precondition order unchanged: `task-file`, `front-matter`,
  `state`, `config`, `touches`, `join`, `review`, `gate-path`, `dependencies`, `approval`,
  `budget` (T-013), `main-history`; (3a) is part of `gate-path`, (1) of `main-history`, (13) of
  `approval`. Queue exit codes: 0 merged and recorded, 1 refused or not merged, 2 usage or tool
  error, 3 merged but the record not committed (new, D-6). Publish exit codes: (9). The exact
  lines are in (1), (2), (2c), (3b), (8), (10), (13).
- (b) **Files.** Reads main's committed `metrics/queue.jsonl`, `xtask/ledger.toml` (new optional
  key `history_anchor`, a 40-hex lowercase sha; any other new key stays a `config` error),
  `docs/decisions.md`; appends one line to `metrics/queue.jsonl` and commits it; appends `join`
  records to `metrics/tasks.jsonl` with unchanged field names. Publish's files as T-013.
  Tests: `xtask/tests/acceptance_t009.rs`, `xtask/tests/guard_t009.rs`.
- (c) **Order of checks.** As (a); within (1), binding is decided per record from newest to
  oldest and the first bound record wins; within (5), the tracked-evidence rule is checked before
  the untracked rules; within (13), near-miss rows are collected only after the check has
  decided to refuse.
- (d) **Paths and platform.** Repository-relative paths; test directories as in (3c); Linux, git
  ≥ 2.38.
- (e) **Threat model.** D-026: honest mistakes and corner-cutting, not a malicious agent. (1)
  closes F19 (a fake `merged` line in a ledger commit). Still accepted (D-030, listed in
  `docs/development.md`): gated code writing to the primary checkout during the gate (F26),
  cached dependency artifacts, approval rows not tied to a commit; a lint attribute added in
  non-test source merges (it is only listed); a director who commits a fabricated line with the
  exact hint command right after a real merge is indistinguishable from the real record. (13)
  only adds information to a refusal; it never approves.
- (f) **Tests pinning behaviour this changes.** Every T-005 and T-013 acceptance and guard test
  must pass unmodified. None asserts that untracked drafts are refused (grep `untracked` in
  `xtask/tests`); `acceptance_t005_r3.rs`:828 asserts that an untracked `metrics/tasks.jsonl`
  is refused at `ledger`, and :872–876 that the queued task's own untracked review is, both of
  which (5) keeps. None asserts a record-commit failure's exit code. The approval refusals of
  T-005/T-013 are asserted by check name and prefix only. `acceptance_t013_publish.rs`:660 and
  :909 assert the `publish: commit failed: ` prefix and the `; commit by hand: git commit … --
  <paths>` suffix, which (8) keeps. Unit tests in `xtask/src` that pin the old anchor, the old
  exit code, the old publish step-8 line or the old approval reason change with the code
  (disclose in the log).
- (g) **Worked examples.** History `A (merged T-001, record commit R_A) → X (direct code commit
  to crates/k/src/lib.rs) → L1 (ledger commit: tasks/x.md plus a fake merged line for T-099
  with head_sha = X)`. On base the anchor is X (newest `merged` line), only L1 is checked, and
  task B merges (F19). After this task L1's line is unbound (L1 also changes `tasks/x.md`, and
  its subject is not `queue: record T-099 merged`), the anchor stays T-001's head, X is checked
  and refused (AC-1). (An earlier version put the fake line before X; then the anchor sits
  before X on base too and the example shows nothing; R-109.) T-004's join at
  2026-11-13T01:18:27Z: `acc_total` 6 and `acc_fail_base` 13 are consistent with the bug the
  planner of R-051 read in `join.rs`: INFERRED, not reproduced; AC-9 fixes the behaviour
  independent of the cause. (13): a decisions table with the row
  `| D-057 | 2026-11-13 | T-016 merged. Approve T-032 for the queue. | x | accepted |` and no
  other T-032 row → `queue: refused: approval: main's docs/decisions.md has no row whose Decision begins with `Approve T-032`; found `Approve T-032` inside, not at the start of, the Decision cell of: D-057`
  (today's reason text read from `precheck.rs`; the suffix is new; not executed).
- (h) **Scorecard items.** None.

## Acceptance criteria
In throw-away git repositories that the tests build (as T-005's and T-013's acceptance tests
do), with `GIT_CONFIG_GLOBAL`/`HOME` isolated so the host's signing and identity settings do not
leak in unless a case sets them; all through the `xtask` binary. Oracle: exact value (exit code,
the one refusal line, record contents, `git log`/`git show` output) unless stated. Each AC says
whether it is an **acceptance** test (`acceptance_t009.rs`, fails on base) or a **guard**
(`guard_t009.rs`, passes on base). No test pins a whole document's bytes or a count of rows,
records or kinds that another task changes (D-043, D-051, D-052).

- AC-1 (fake merged line ignored; acceptance): after a real queue merge of task A, commit on
  main a direct code commit X (`crates/k/src/lib.rs`), then a ledger commit (`tasks/x.md` plus a
  fabricated `merged` line for task Z with `head_sha` = X), then queue task B →
  `queue: refused: main-history: commit <X> ("<subject>") on main after the last queue merge changes `crates/k/src/lib.rs` (M), which xtask/ledger.toml does not allow`.
  Without the fake line, the result is the same (control). With only ledger commits after A, B
  merges (control).
- AC-2 (binding rules; acceptance): a `merged` line that is unbound for each single reason (its
  commit also touches `tasks/`; subject differs; first parent ≠ `head_sha`; the commit appends
  two lines) and names a `head_sha` after a direct code commit does not move the anchor (shown as
  in AC-1). A record committed by hand with the exact hint command of (2), directly after the
  fast-forward, is bound.
- AC-3 (bootstrap anchor; acceptance): with no bound record and `history_anchor` set, commits
  after the anchor are checked (a direct code commit is refused); without `history_anchor` and
  with no bound record, the refusal is the exact line in (1); a malformed `history_anchor` →
  `config`.
- AC-4 (signing kept, failure reported): guard half — with `commit.gpgsign=true`,
  `gpg.format=ssh` and a test script as `gpg.ssh.program` that signs, the record commit is
  signed (the program's invocation is observed, or `git log --format=%G?` is not `N`).
  Acceptance half — with a program that exits non-zero: exit code 3, stderr ends with exactly
  the line of (2), `main` is at the task head, `metrics/queue.jsonl` ends with the uncommitted
  `merged` record, and the next `queue` run of any task is refused at `ledger`. After the test
  repairs the signing program, the printed command creates a bound record (AC-2) and the next
  run is no longer refused at `ledger`.
- AC-5 (record pathspec; guard): while main's gate runs, a test gate stages an unrelated file in
  the primary checkout. After the merge, the record commit's changed paths are exactly
  `metrics/queue.jsonl`, and the unrelated file is still staged and not committed (kills P09).
- AC-6 (identity fallback; guard): with no `user.name`/`user.email` and no `GIT_COMMITTER_*`
  variables, a merge records a commit whose author and committer are
  `kiln queue <queue@kiln.invalid>`, exit 0. With an identity configured, the commit carries
  that identity (kills P14).
- AC-7 (lint attributes): acceptance — a medium-tier task that adds a member crate whose
  `Cargo.toml` has no `[lints]` table → refused at `gate-path` naming that `Cargo.toml`; the same
  on a high task with a `gate paths` approval merges; a medium task that adds
  `#![allow(clippy::unwrap_used)]` to `crates/a/src/lib.rs`, `#[cfg_attr(test, allow(dead_code))]`
  and `#[expect(unused)]` to another source file → merges, and stderr has exactly one listing
  line per attribute with the right `<path>:<line>`; the summary lists them. Guard — a medium
  task that removes `[lints] workspace = true` from an existing member → refused at
  `gate-path` naming it (D-030 R10, already on main); a medium task that adds
  `#![allow(dead_code)]` to `crates/a/tests/support.rs` → merges with no listing line; editing
  other lines of a file whose `#[allow]` already exists on the merge base → no listing line.
- AC-8 (untracked drafts; acceptance): untracked `tasks/T-099-draft.md`, `reviews/T-099-r1.md`
  and `notes.txt` on main while queueing T-001 → merged, and all three files are still
  untracked and byte-identical afterwards. An untracked `tasks/T-001-x.md` (the queued task's
  own) or an untracked `metrics/extra.jsonl` → `queue: refused: ledger: …` naming that file.
- AC-9 (join counts per package; acceptance): a repository with
  `crates/a/tests/acceptance_t901.rs` (3 tests), `apps/b/tests/acceptance_t901.rs` (2 tests) and
  `xtask/tests/acceptance_t901_x.rs` (1 test), all failing on base and passing after the join →
  record `acc_total: 6`, `acc_fail_base: 6`, `base_fail_kind: "fails"`, `acc_fail_first_join: 0`,
  `result: "joined"`. If one of the two same-named targets breaks after the join,
  `acc_fail_first_join` counts its failures and `result` is `acceptance_failed` (shows both
  targets run on the joined branch). With one same-named target not compiling on base:
  `acc_fail_base: 6`, `base_fail_kind` `"mixed"` as the existing rules give. In every case
  `acc_fail_base ≤ acc_total`. A task whose only acceptance tests are in `xtask/tests` joins
  with correct counts. (The counts are of the toy repository the test builds, not of this
  repository.)
- AC-10 (no regression; guard by nature): every T-005 and T-013 acceptance and guard test
  passes unmodified; main's gate and the task's own gate are both green. Oracle: regression.
- AC-11 (join regenerates the scorecard file; acceptance): a join whose tests branch adds test
  names to an item's `tests` (status `failing` on the tests branch, `implemented` after the
  merge) leaves `task/T-NNN` with one `join: regenerate docs/scorecard.md` commit that changes
  only that file and carries `Task:`/`Role: director`; the joined gate's `scorecard` step
  passes; `main` and the primary checkout are unchanged (same sha, same status). A join that
  changes no status makes no such commit. Oracle: exact value.
- AC-12 (publish ledger-commit failures; acceptance): in T-013's publish toy repository, a
  `commit-msg` hook on main that rejects exactly one subject (`publish: T-050 ready`, then in
  separate runs `publish: T-050 in progress`, `publish: T-050 queued`, `publish: T-050 merged`;
  the queue's own record commit uses `--no-verify` and is not affected) → exit 2 and one
  stderr line in the form of (8) with that subject and the step's paths; nothing of a later
  step happens. And with `.git/index.lock` present at step 2 (as T-013's test does): the line
  of (8), and after the test removes the lock, running the printed `stage by hand` command and
  then the `commit by hand` command (each through `sh -c` from the repository root) creates the
  `publish: T-050 ready` commit with exactly the two paths.
- AC-13 (stop line for a failing transition; acceptance): in the publish toy repository, the
  toy gate (step 5) appends to the primary checkout's `metrics/tasks.jsonl` a well-formed T-050
  `run` record whose `ended_at` is one day after now (a `run`, so the last transition stays
  `in_progress`), so step 6's first transition is refused by the time rules (a time earlier
  than the task's latest record)
  → exit 1, stderr has exactly one line that starts with `publish: stopped at step 6: `, and
  that line ends with `; T-050 is in_progress` (the `<error>` text is not pinned); no stderr
  line starts with `xtask: `; and main's committed task file says
  `state: in_progress`.
- AC-14 (step-4 content commit; guard): a hook that rejects commits on `task/*` branches makes
  step 4's content commit fail → exit 1, one line starting `publish: stopped at step 4: `, and
  no line starting `publish: commit failed: ` (ruling D-047 (q), already on main).
- AC-15 (approval near-miss; acceptance): a high-tier task `T-002` whose only row mentioning it
  is `| D-9 | d | T-001 merged. Approve T-002 for the queue. | x |` → exactly the refusal of
  (13) with `<ids>` = `D-9`; with a second such row `D-10` after it, `<ids>` = `D-9, D-10`;
  with a row `Approve T-0020 …` only (a longer id) → today's line with no suffix; a gate-path
  task whose only row is `| D-9 | d | Note. Approve T-002 incl. gate paths. | x |` → today's
  gate-path reason followed by the suffix. In each case nothing is merged.
- AC-16 (publish after queue exit 3; acceptance): in the publish toy repository with
  `commit.gpgsign=true`, `gpg.format=ssh` and a signing program that signs every payload except
  one containing `queue: record T-050 merged` → publish exits 3; stderr has the queue's line of
  (2) and then exactly the line of (2c); `main` is at `task/T-050`'s head (the queue
  fast-forwarded it), main's committed T-050 state is `queued`, no `publish: T-050 merged` commit exists
  and `.worktrees/T-050` still exists.

## Out of scope and implementer notes
- Regenerating `docs/scorecard.md` in the queue outside generated-path conflicts (withdrawn,
  D-036). Regenerating `docs/commands.json` or `Cargo.lock` outside conflicts.
- Fixture projects under `tests/fixtures/**` escalated to tier high (r3 round-2 reading): a
  separate decision. Each failed queue attempt moving `main` by a record commit (r3 F25): as R12,
  no change. Whether a new member *with* exactly `[lints] workspace = true` should stop being a
  gate change (today it is one, because its `[lints]` differs from an absent file): not changed
  here (Questions).
- Metrics, budget, worktree state, review lint, session paths: T-013 (merged). Publish changes
  only as (2c), (8), (10), (12)'s neighbour and the docs of (9).
- Any product crate, `Cargo.toml`, `Cargo.lock`, `.cargo/`, `.github/`, `methodology/`, and
  T-013's task file.
- Your tests: unit tests in `xtask/src` (the existing toy-repository helpers in `git.rs`
  `testrepo`), including (11)'s step-2 undo with a committed `metrics/tasks.jsonl` whose bytes
  must be restored (R-097 mutant N6 must be killed). Acceptance and guard files of T-005 and
  T-013 are not edited.
- Specified: the rules, messages, exit codes and record fields above. Left to you: how bound
  commits are found, how attributes are detected (the `syn` dependency already exists), how
  untracked files are classified, how near-miss rows are collected.
- Never set or export `CARGO_TARGET_DIR` in commands you run by hand (D-019). Tests that need a
  target directory create their own.

## Order with T-013
T-013 is merged (`3ef7600`, D-055); this task's worktree starts from a `main` that has the
`budget` precondition, `cargo xtask publish` and the `hygiene` step. The older statement that
"exit 3 needs no change" in publish was wrong: publish passes the code through but prints "did
not merge", hence (2c). The cross-crate join count stays here.

## Decisions
Director's answers to the planner's questions of R-051 (2026-11-13, recorded by R-055).
- D-1 (was Q-1): the record commit keeps whatever signing the repository is configured to
  require; there is no bypass flag. If the commit fails, the queue stops with an error that says
  exactly how to commit the record by hand, and the next run refuses at `ledger` until that is
  done. (Specified in (2) as the one stderr line and exit 3 after a merge; confirmed by D-6.)
- D-2 (was Q-2): `history_anchor` is the head recorded by the merged T-005 queue record,
  `a981f18210cbd59be20e5e7009b9ed8429850b64`.
- D-3 (was Q-3): a lint `allow` attribute added in a test file is not a gate change. One added in
  non-test source is listed in the queue's output for the reviewer but does not refuse the run.
  (Specified in (3b), (3c); `expect` is treated like `allow`, since it suppresses the lint too.)
- D-4: queue-side regeneration of `docs/scorecard.md` (after every rebase) is removed; D-036
  withdrew it.
- D-5 (director R-057, was Q-4): `join` regenerates `docs/scorecard.md` on the task branch
  (never on `main`) and commits it with `Role: director` lines. Specified as (7), AC-11.
- D-6 (director R-057, was Q-5): "merged but record not committed" exits 3, distinct from usage
  and tool errors (2) and refusals (1), with the exact recovery command in the message.
- D-053 (director): the five T-013 round-2 follow-ups go to this spec: (8)–(12).
- D-058 (director): the `approval` refusal should name the row that mentions the task but does
  not begin with `Approve T-NNN`: (13).

## Questions and rulings
- Q-1 (planner, R-109): split T-009 into groups A/B/C (Goal, "Size")? Proposed yes; budget.
- Q-2 (planner, R-109): should a new member with exactly `[lints] workspace = true` (and no
  `build` key) stop being a gate change? Not specified here; it would relax a gate rule.

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
- 2026-11-13T03:56Z · R-057 · draft · did: applied D-5 (join-side regeneration as (7) and
  AC-11; queue-side stays withdrawn) and D-6 (exit 3 wording); Q-4/Q-5 closed. · gate
  attempts: 0 · missing context: none · next: director review.
- 2026-11-13T12:43Z · R-109 · draft · did: checked every item against merged T-013 code (main
  `e614fee`). Still open on main, kept: (1) (`precheck.rs` anchor = newest `merged` line),
  (2)'s failure path (`commit_record` bails, exit 2), (3b)/(3c), (5), (6) (`find` by target
  name), (7). Already done, reworded: (2)'s signing (no override in `commit_record`) and (3a)'s
  existing-member case (D-030 R10), both now guards; (4)'s tests are guards, not acceptance
  tests (they pass on base; the join refuses such acceptance tests). Fixed AC-1/(g): the fake
  line must come after the direct commit, or base refuses too. Found that publish's step 8
  prints "did not merge" on exit 3, so added (2c)/AC-16. Added D-053's five follow-ups as
  (8)–(12) (follow-up 1 split: unit test for the undo, AC-12 for steps 3/8) and D-058 as (13);
  (8)'s new clause keeps T-013's pinned prefix and suffix, so no T-013 test is edited. Added
  `xtask/tests/{acceptance,guard}_t009.rs` to `touches` (gate paths). Tier high (gate paths).
  Pinned (h) n/a (no scorecard item). · gate attempts: 0 · not run: no cargo; (g)'s (13)
  example and the exit-3 signing injection not executed · missing context: none · next:
  director decides the split (Q-1); then a spec check.
  QUESTION: Q-1 split (thirteen items, T-013's seven cost 1.79M); Q-2 new member with
  workspace lints.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
