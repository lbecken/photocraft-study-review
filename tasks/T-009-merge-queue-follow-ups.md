---
id: T-009
title: "Close the merge queue's remaining gaps: bind merge records to their record commits, report a failed record commit exactly (exit 3, also through publish), guard the record commit, tolerate drafts, and name a near-miss approval row"
tier: high              # ADR-0002 "any gate path": xtask/src, xtask/tests, xtask/ledger.toml; the approval row must contain `gate paths`
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-005, T-013]   # both merged (T-013 `3ef7600`, D-055)
touches:
  - xtask/src/precheck.rs        # GATE PATH: main-history binding (1), approval reason (5)
  - xtask/src/queue.rs           # GATE PATH: record-commit failure (2), ledger tolerance (4)
  - xtask/src/policy.rs          # GATE PATH: the `history_anchor` key of ledger.toml (1)
  - xtask/src/publish.rs         # GATE PATH: step 8 on queue exit 3 only (2c)
  - xtask/ledger.toml            # GATE PATH: new `history_anchor` key
  - xtask/tests/acceptance_t009.rs   # GATE PATH: test author
  - xtask/tests/guard_t009.rs        # GATE PATH: test author (guards for behaviour main already has)
  - docs/development.md          # "Merge queue", publish "After a stop" row (not a gate path)
budget_tokens: 850000
iteration: 1
found_by: reviewer
origin_task: T-005
---

# T-009 Close the merge queue's remaining gaps

## Goal
T-005 made `cargo xtask queue` the only way code reaches `main`. Its final review (r3, findings
19–24) left gaps in the queue (D-033), and D-058 found a refusal that did not say what it saw.
After this task:
(1) a `merged` line in `metrics/queue.jsonl` counts as evidence only if the queue's own record
commit wrote it;
(2) when the record commit fails after a merge the queue says exactly how to commit the record
by hand and exits 3 instead of reporting a failed merge, and `publish` reports that case as a
merge, not as "did not merge";
(3) the record-commit pathspec, the identity fallback and the kept signing configuration have
regression guards;
(4) untracked drafts no longer block the queue unless they are the queued task's own evidence;
(5) an `approval` refusal names every row whose Decision cell contains `Approve T-NNN` but does
not begin with it (D-058).

**Split (D-061 note, R-112).** This file was part A of R-109's thirteen-item T-009. Part B
(join counts per package, scorecard regeneration at join) is T-033; part C (lint escapes and
the publish follow-ups of T-013 r2) is T-034. Item numbers below are new; the mapping from
R-109's numbers is (1)→(1), (2)/(2c)→(2), (4)→(3), (5)→(4), (13)→(5). **Dispatch:** never in
flight with T-033 (`docs/development.md`), T-034 (`precheck.rs`, `queue.rs`, `publish.rs`,
`docs/development.md`) or T-028 (`docs/development.md`).

Regeneration by the queue (after every rebase) stays out: D-036 withdrew it; ledger inputs that
change generated output go through T-013's `cargo xtask publish` (merged).

**Gate paths.** Every `xtask/` path in `touches` is a gate path, so the task is tier high
(ADR-0002). `docs/development.md` documents the list but is not itself a gate path. The
director's approval row in `docs/decisions.md` must begin `Approve T-009` and contain the words
`gate paths`, or the queue refuses the task (T-005 R3). The task changes no dependency (no
`Cargo.toml`, `Cargo.lock`, lint table or build key). Per T-005 D-3, it must pass both main's
gate and its own. If another `xtask/src` file must change, stop and report (golden rule 5).

Reading list: `docs/decisions.md` D-019, D-021, D-026 (threat model), D-030 (R10–R13), D-033,
D-036, D-047 (rulings j–q), D-058; `reviews/T-005-r3.md` findings 19–26 and its follow-up list;
`tasks/T-005-queue-hardening.md` Interface and Decisions R1–R13 (exit codes, refusals, record
format, precondition order); `tasks/T-013-process-tooling-v11.md` Interface (2) (`budget`) and
(5) (`publish` step 8); `xtask/src/{precheck,queue,policy,publish}.rs`; `xtask/ledger.toml`;
`docs/development.md` "Merge queue", "Publishing a ledger input" and "After a stop".

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
(`queue.rs` `commit_record` passes no signing override); (3) adds a guard for it. It still uses
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

**Publish after a queue exit 3.** `publish` step 8 today treats every non-zero queue exit as
"did not merge" (`publish: queue did not merge T-NNN (exit c); the task stays queued; …`),
which is false for exit 3. After this task, on queue exit 3 publish prints exactly one line of
its own after the queue's output,
`publish: the queue merged T-NNN but did not commit its record (exit 3); commit the record as the queue printed, then finish step 8 by hand (docs/development.md, "After a stop")`,
records no transition, makes no commit, leaves the worktree, and exits 3. Every other non-zero
exit keeps T-013's line and exit code. `docs/development.md` "After a stop" gains the row
"8, queue exit 3" (left behind: merged by the queue, its record uncommitted, `queued` committed;
finish: run the queue's printed command, then `metrics transition --task T-NNN --to merged`,
set state merged, ledger commit "publish: T-NNN merged", `worktree rm T-NNN`).

### (3) Guards for three unchanged paths
No behaviour change: the record commit's pathspec, the identity fallback and the kept signing
configuration get regression guards in `xtask/tests/guard_t009.rs` (AC-4 first half, AC-5,
AC-6). They pass on base by design, which is why they are guards, not acceptance tests; they
kill reviewer mutants P09 and P14 of T-005 r3.

### (4) What `ledger` tolerates
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

### (5) Approval refusal names near-miss rows (D-058)
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
  `budget` (T-013), `main-history`; (1) is part of `main-history`, (5) of `approval`. Queue
  exit codes: 0 merged and recorded, 1 refused or not merged, 2 usage or tool error, 3 merged
  but the record not committed (new, D-6). Publish: exit 3 passed through with the line of (2);
  its other exits as T-013. The exact lines are in (1), (2), (5).
- (b) **Files.** Reads main's committed `metrics/queue.jsonl`, `xtask/ledger.toml` (new optional
  key `history_anchor`, a 40-hex lowercase sha; any other new key stays a `config` error),
  `docs/decisions.md`; appends one line to `metrics/queue.jsonl` and commits it. Tests:
  `xtask/tests/acceptance_t009.rs`, `xtask/tests/guard_t009.rs`.
- (c) **Order of checks.** As (a); within (1), binding is decided per record from newest to
  oldest and the first bound record wins; within (4), the tracked-evidence rule is checked before
  the untracked rules; within (5), near-miss rows are collected only after the check has
  decided to refuse.
- (d) **Paths and platform.** Repository-relative paths; Linux, git ≥ 2.38.
- (e) **Threat model.** D-026: honest mistakes and corner-cutting, not a malicious agent. (1)
  closes F19 (a fake `merged` line in a ledger commit). Still accepted (D-030, listed in
  `docs/development.md`): gated code writing to the primary checkout during the gate (F26),
  cached dependency artifacts, approval rows not tied to a commit; a director who commits a
  fabricated line with the exact hint command right after a real merge is indistinguishable
  from the real record. (5) only adds information to a refusal; it never approves.
- (f) **Tests pinning behaviour this changes.** Every T-005 and T-013 acceptance and guard test
  must pass unmodified. None asserts that untracked drafts are refused (grep `untracked` in
  `xtask/tests`); `acceptance_t005_r3.rs`:828 asserts that an untracked `metrics/tasks.jsonl`
  is refused at `ledger`, and :872–876 that the queued task's own untracked review is, both of
  which (4) keeps. None asserts a record-commit failure's exit code or publish's step-8 line
  for exit 3. The approval refusals of T-005/T-013 are asserted by check name and prefix only.
  Unit tests in `xtask/src` that pin the old anchor, the old exit code, the old publish
  step-8 line or the old approval reason change with the code (disclose in the log).
- (g) **Worked examples.** History `A (merged T-001, record commit R_A) → X (direct code commit
  to crates/k/src/lib.rs) → L1 (ledger commit: tasks/x.md plus a fake merged line for T-099
  with head_sha = X)`. On base the anchor is X (newest `merged` line), only L1 is checked, and
  task B merges (F19). After this task L1's line is unbound (L1 also changes `tasks/x.md`, and
  its subject is not `queue: record T-099 merged`), the anchor stays T-001's head, X is checked
  and refused (AC-1). (5): a decisions table with the row
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
- AC-7 (untracked drafts; acceptance): untracked `tasks/T-099-draft.md`, `reviews/T-099-r1.md`
  and `notes.txt` on main while queueing T-001 → merged, and all three files are still
  untracked and byte-identical afterwards. An untracked `tasks/T-001-x.md` (the queued task's
  own) or an untracked `metrics/extra.jsonl` → `queue: refused: ledger: …` naming that file.
- AC-8 (approval near-miss; acceptance): a high-tier task `T-002` whose only row mentioning it
  is `| D-9 | d | T-001 merged. Approve T-002 for the queue. | x |` → exactly the refusal of
  (5) with `<ids>` = `D-9`; with a second such row `D-10` after it, `<ids>` = `D-9, D-10`;
  with a row `Approve T-0020 …` only (a longer id) → today's line with no suffix; a gate-path
  task whose only row is `| D-9 | d | Note. Approve T-002 incl. gate paths. | x |` → today's
  gate-path reason followed by the suffix. In each case nothing is merged.
- AC-9 (publish after queue exit 3; acceptance): in T-013's publish toy repository with
  `commit.gpgsign=true`, `gpg.format=ssh` and a signing program that signs every payload except
  one containing `queue: record T-050 merged` → publish exits 3; stderr has the queue's line of
  (2) and then exactly publish's exit-3 line of (2); `main` is at `task/T-050`'s head (the queue
  fast-forwarded it), main's committed T-050 state is `queued`, no `publish: T-050 merged`
  commit exists and `.worktrees/T-050` still exists.
- AC-10 (no regression; guard by nature): every T-005 and T-013 acceptance and guard test
  passes unmodified; main's gate and the task's own gate are both green. Oracle: regression.

## Out of scope and implementer notes
- Join counts and scorecard regeneration at join: T-033. Lint escapes and the other publish
  follow-ups (hand commands, exit-code statement, stop line for failing transitions, owed
  tests, the ceiling parser): T-034.
- Regenerating `docs/scorecard.md` in the queue outside generated-path conflicts (withdrawn,
  D-036). Each failed queue attempt moving `main` by a record commit (r3 F25): as R12, no
  change.
- Metrics, budget, worktree state, review lint, session paths: T-013 (merged). Publish changes
  only in step 8's handling of exit 3.
- Any product crate, `Cargo.toml`, `Cargo.lock`, `.cargo/`, `.github/`, `methodology/`, and
  T-013's task file.
- Your tests: unit tests in `xtask/src` (the existing toy-repository helpers in `git.rs`
  `testrepo`). Acceptance and guard files of T-005 and T-013 are not edited.
- Specified: the rules, messages, exit codes and record fields above. Left to you: how bound
  commits are found, how untracked files are classified, how near-miss rows are collected.
- Never set or export `CARGO_TARGET_DIR` in commands you run by hand (D-019). Tests that need a
  target directory create their own.

## Order with T-013
T-013 is merged (`3ef7600`, D-055); this task's worktree starts from a `main` that has the
`budget` precondition, `cargo xtask publish` and the `hygiene` step. The older statement that
"exit 3 needs no change" in publish was wrong: publish passes the code through but prints "did
not merge", hence (2)'s publish paragraph.

## Decisions
Director's answers to the planner's questions of R-051 (2026-11-13, recorded by R-055).
- D-1 (was Q-1): the record commit keeps whatever signing the repository is configured to
  require; there is no bypass flag. If the commit fails, the queue stops with an error that says
  exactly how to commit the record by hand, and the next run refuses at `ledger` until that is
  done. (Specified in (2) as the one stderr line and exit 3 after a merge; confirmed by D-6.)
- D-2 (was Q-2): `history_anchor` is the head recorded by the merged T-005 queue record,
  `a981f18210cbd59be20e5e7009b9ed8429850b64`.
- D-3, D-5 (lint attributes; scorecard at join): moved with their items to T-034 and T-033.
- D-4: queue-side regeneration of `docs/scorecard.md` (after every rebase) is removed; D-036
  withdrew it.
- D-6 (director R-057, was Q-5): "merged but record not committed" exits 3, distinct from usage
  and tool errors (2) and refusals (1), with the exact recovery command in the message.
- D-058 (director): the `approval` refusal should name the row that mentions the task but does
  not begin with `Approve T-NNN`: (5).
- Director (2026-11-13, R-112 brief): R-109's split approved; T-009 keeps part A.

## Questions and rulings
- Q-1 (planner, R-109): split T-009? Answered: yes (part A here, B = T-033, C = T-034).
- Q-2 (planner, R-109; moved to T-034): a new member with exactly `[lints] workspace = true`.

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
- 2026-11-13T12:49Z · R-112 · draft · did: applied the approved split. T-009 keeps part A: queue items,
  renumbered (1) binding, (2) record-commit failure with publish's exit-3 line, (3) guards,
  (4) untracked drafts, (5) approval near-miss; ACs AC-1…AC-10 (R-109's AC-1–6, 8, 15, 16,
  10). Part B → T-033, part C → T-034 (created). `touches` narrowed from `xtask/src` to four
  files. Not in flight with T-033, T-034, T-028. Tier high (gate paths). · gate attempts: 0 ·
  not run: no cargo · missing context: none · next: director; ready for a spec check.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
