---
id: T-043
title: "Write every queue precondition refusal to metrics/queue.jsonl, and let the queue and publish proceed past dirty ledger files they do not read for the task"
tier: high              # ADR-0002 "a gate path": xtask/src, xtask/tests; the approval row must contain `gate paths`
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-005, T-013]   # both merged before this spec was written; no spec check is required by METHODOLOGY §5.7
touches:
  - xtask/src/queue.rs                     # GATE PATH: implementer (items 1 and 2)
  - xtask/src/publish.rs                   # GATE PATH: implementer: step 1's working-tree rule (item 2)
  - xtask/src/main.rs                      # GATE PATH: implementer, only if the refusal record must be written where the refusal is printed
  - xtask/tests/acceptance_t005_support.rs # GATE PATH: implementer: `assert_refused` only (Pinned (f))
  - xtask/tests/acceptance_t005_r3.rs      # GATE PATH: implementer: the `refused` helper only
  - xtask/tests/acceptance_t005_r4.rs      # GATE PATH: implementer: the `refused` helper only
  - xtask/tests/acceptance_t005.rs         # GATE PATH: implementer: the last assertion of `refusals_change_nothing` only
  - xtask/tests/acceptance_t043.rs         # GATE PATH: test author
  - xtask/tests/guard_t043.rs              # GATE PATH: test author (behaviour main already has that this task must keep)
  - docs/development.md                    # implementer: "Merge queue" steps 1–3 and the record paragraph, "Publishing a ledger input" step 1 and the "8, queue refused" row; not a gate path
budget_tokens: 1100000  # high (METHODOLOGY §11, v1.2)
iteration: 2
found_by: incident      # two unrecorded refusals (retro-1 "Tooling and environment"); three refusals for unrelated dirty ledger files on 2026-11-13 (director's request to R-160)
origin_task: T-005
---

# T-043 Queue refusals recorded; unread dirty ledger files do not stop the queue or publish

## Goal
Two changes to the merge queue's checks before it rebases, both in `cargo xtask queue` and
`cargo xtask publish`:

1. **Refusals recorded** (methodology v1.2 change 9, queue half; split from T-033, see T-033 Q-1).
   A refusal by one of the queue's preconditions is appended to `metrics/queue.jsonl` and
   committed on `main` like every other outcome, so first-pass queue success counts it. Today a
   refusal writes nothing: T-015 was refused at `review` (D-050) and T-032 at `approval` (D-058),
   and neither is in the ledger (retro-1, "Tooling and environment").
2. **Unread ledger files do not stop the queue.** An uncommitted or untracked file under a ledger
   path that the queue does not read as evidence for the task it is merging, and that the merge
   does not change, no longer makes `queue` or `publish` refuse. A dirty file that is evidence for
   that task still does. On 2026-11-13 the director had to commit another agent's half-written
   file as work in progress three times to proceed: `publish` of T-040 at 18:17:03Z (untracked
   `retrospectives/scripts/iter1_*`), publish's queue step at 18:23:07Z (untracked
   `retrospectives/iter-1.md`), and `queue T-039` at 21:38:35Z, refused at `ledger` for
   `tasks/T-019-project-tree.md` while a spec check was editing it.

Item 2 is bundled with item 1 because both are the queue's working-tree and precondition checks
in one function (`queue.rs`, before the rebase), item 1 must say which refusals are recorded and
the `ledger` refusal is exactly the one it cannot record (Pinned (a)), and both change the same
two passages of `docs/development.md`. A separate task would change the same lines of
`queue.rs` and could not be in flight with this one (Q-4).

**Gate paths.** Every `xtask/` path in `touches` is a gate path, so the task is tier high; the
approval row must begin `Approve T-043` and contain the words `gate paths`. No dependency changes.
The queue runs main's gate and the task's own gate; both must pass.

**Dispatch.** `docs/development.md` is also in the `touches` of T-033 and T-041 (§5.2). The
stage-2 draft T-009 (`main-history` bound to record commits) changes `queue.rs`, `precheck.rs`
and `publish.rs`; its spec check must read this task's record commits as queue record commits.

Reading list: `methodology/METHODOLOGY.md` §8, §13; `methodology/CHANGELOG.md` v1.2 row 9;
`docs/decisions.md` D-026 (R5, R6), D-027, D-030 (R12), D-050, D-058, D-091;
`docs/development.md` "Merge queue" and "Publishing a ledger input"; `xtask/src/queue.rs`
(`run`, `is_evidence_path`, `Refusal`), `xtask/src/precheck.rs` (every `show(root, "main", …)`
and `committed_jsonl` call), `xtask/src/publish.rs` step 1, `xtask/ledger.toml`;
`xtask/tests/acceptance_t005_support.rs` (`Snap`, `assert_refused`), `acceptance_t005_r3.rs`
(`refused`, `uncommitted_evidence_does_not_count`), `acceptance_t005_r4.rs` (`refused`),
`acceptance_t005.rs` (`refusals_change_nothing`).

## Interface
Every command keeps its name, options, output and exit codes unless a change is listed here.

### (1) A refusal is a record
When `cargo xtask queue T-NNN` refuses at a **recorded check** (Pinned (a)), it still prints its
one stderr line `queue: refused: <check>: <reason>` and exits 1, and in addition:
- appends one line to `metrics/queue.jsonl`, with the existing fields and one more:
  ```json
  {"task":"T-NNN","attempt":3,"started_at":"…Z","ended_at":"…Z","base_sha":"<main>",
   "head_sha":"<task head as found>","result":"refused","failed_step":"<check>",
   "gate_seconds":0.0,"gate_xtask":"none","gate_xtask_sha":"<base_sha>","regenerated":[],
   "reason":"<the text after `queue: refused: <check>: `>"}
  ```
  `attempt` is 1 + the number of the task's earlier records, refusals included (Q-1), so a task
  refused before it merges has no first-pass success. `head_sha` is the task branch's head when
  the run began (the branch is never moved by a refusal). `regenerated` lists generated paths
  regenerated before a refusal after the rebase, else `[]`. `gate_xtask` is `none` because no
  gate ran.
- commits that line on `main` exactly as the other outcomes' records are committed: one commit
  touching only `metrics/queue.jsonl`, subject `queue: record T-NNN refused`, body lines
  `Task: T-NNN` and `Role: director`. It is a ledger commit, so the next `main-history` check
  tolerates it. If the commit fails, the queue behaves as it does today when another record's
  commit fails (the refusal line is still printed first).
- changes nothing else: the task branch, its worktree and the primary checkout's other files are
  as they were. Unchanged: precondition order, the refusal line, which check refuses.

`publish` passes the queue's output and exit code through in its step 8, as today; a refusal
there now leaves the refusal record commit on `main` too.

### (2) Which dirty files stop the queue
Before its preconditions the queue looks at every path of the primary checkout that is modified,
staged or untracked (as today, `git status`; ignored files are not seen). Each such path is in
exactly one class, decided in this order:
1. **evidence for the task**: anything under `metrics/`; `docs/decisions.md`; a path beginning
   `tasks/T-NNN-` (the task's own file, or another file that would make the `task-file` check see
   two); anything under `reviews/`; `xtask/generated.toml`; `xtask/ledger.toml`; and any path the
   task branch changes (`git diff --name-only --no-renames <merge-base> <task head>`), because
   the fast-forward would meet it → refusal at `ledger` (exit 1, one line, nothing changed, no
   record; as today);
2. **staged** (the index differs from `HEAD` for that path) and under a ledger path → refusal at
   `ledger`, as today (a staged change can stop the fast-forward; Pinned (e));
3. **unread ledger file**: unstaged-modified or untracked, under an entry of main's
   `xtask/ledger.toml` (any mode) → ignored: the queue goes on, prints one stderr line beginning
   `queue: ignoring uncommitted ledger paths not read for T-NNN: ` naming them, and leaves each
   exactly as it found it (contents, untracked or modified state);
4. anything else → the existing working-tree error (exit 2, no record), unchanged.
If main's `xtask/ledger.toml` cannot be read or is invalid, class 3 is empty (every dirty path
outside class 1 and 2 is class 4), and the `config` precondition refuses as today.

`publish` applies the same classes in its step 1 to the paths that are not its inputs, with
`T-NNN` its new task id (whose files cannot exist yet) and "the paths the branch changes" being
its inputs and `docs/scorecard.md`; its refusal line stays `publish: refused: …`. Its later
steps commit only their own paths, as today, and its step 8 meets the same rule in the queue.

### (3) `docs/development.md`
These passages state what the tools then do (wording is the implementer's; the reviewer checks
each sentence against the built binary, METHODOLOGY §7):
- "Merge queue" steps 1–3: the classes of (2) replace "the working tree has (non-evidence)
  changes outside `metrics/`" and the evidence list of step 2; step 3's "nothing changed and no
  record written" becomes the record of (1); the sentence "A refusal writes and commits nothing."
  is replaced by what (1) commits; the JSON example's `result` list gains `refused` and the new
  `reason` field.
- "Publishing a ledger input" step 1: its working-tree rule follows (2); the "After a stop" row
  "8, queue refused" says that the refusal record is committed on `main` as well.

## Pinned
- (a) **Errors and precedence.** Recorded checks: every check that prints
  `queue: refused: <check>: ` except `ledger`, that is `task-file`, `front-matter`, `state`,
  `config`, `touches`, `join`, `review`, `gate-path`, `dependencies` (before or after the
  rebase), `approval`, `budget`, `main-history`. Not recorded: `ledger` (the record would be
  committed while evidence, possibly `metrics/` itself, is uncommitted) and the step-1 errors
  (exit 2: `main` not checked out, missing or dirty worktree, no commits beyond `main`, the class-4
  working-tree error), which are not refusals. A refusal record is written while the queue lock
  is held. Exit codes unchanged: 1 for a refusal, 2 for errors.
- (b) **Files.** `metrics/queue.jsonl`: appended and committed (one line per refusal, fields as
  in (1)). Reads main's committed `xtask/ledger.toml` for class 3. Writes nothing else. Test
  files `xtask/tests/acceptance_t043.rs` and `xtask/tests/guard_t043.rs`: no other package has a
  target with either stem (`find . -path ./target -prune -o -name 'acceptance_t043*' -print` and
  the same for `guard_t043*` print nothing on main `b53d370f4a5f22eeffd34969b49f34170e0edf57`).
- (c) **Order of checks.** Lock → `main` checked out → the classes of (2) (class 1 or 2 present:
  `ledger`; class 4 present: error) → worktree checks of today → preconditions in today's order
  (first failure: refusal record, then exit 1) → build of main's xtask → rebase → … (today).
- (d) **Paths and platform.** Repository-relative paths with `/`; quoted paths in `git status`
  output are unquoted before classification (as `is_evidence_path` does today). Linux.
- (e) **Threat model.** D-026: honest mistakes and corner-cutting. The `ledger` refusal exists so
  that evidence the director believes committed is not silently ignored (D-026 R6, D-027); class
  1 keeps that for everything the preconditions read: main's task file and the `tasks/` listing,
  `docs/decisions.md` (`dependencies`, `approval`, `budget`), `metrics/tasks.jsonl` (`join`,
  `budget`), `metrics/queue.jsonl` (`main-history`, and the queue's own record), the two policy
  files (`config`), as read in `precheck.rs` (`show(root, "main", …)`, `committed_jsonl`).
  Review files are read from the task head, not from the primary checkout; `reviews/` stays in
  class 1 as today because it is not a ledger path. A dirty path the task's diff changes is class
  1 because `git merge --ff-only` stops on it after a full gate run. A staged change is class 2:
  `git merge` documents that it may abort when the index differs from `HEAD` (INFERRED from the
  git documentation, not reproduced here). Not defended: a director who leaves another task's
  evidence uncommitted and then queues that other task; that run is refused at `ledger` as today.
- (f) **Existing tests this task changes.** The refusal helpers assert that a refusal changes
  nothing, including `metrics/queue.jsonl` and `main`'s head: 87 call sites (command:
  `grep -hoE '\bassert_refused\(|(^|[^_a-z])refused\(&?r,' xtask/tests/acceptance_t005*.rs xtask/tests/acceptance_t013_queue.rs | wc -l`
  on main `b53d370f4a5f22eeffd34969b49f34170e0edf57`) through three helpers:
  `acceptance_t005_support.rs` `Repo::assert_refused`, `acceptance_t005_r3.rs` `refused` and
  `acceptance_t005_r4.rs` `refused`. Each helper keeps every assertion it has for check
  `ledger`. For any other check, the "nothing changed" comparison becomes: `main` advanced by
  exactly one commit whose parent is the old `main`, which changes only `metrics/queue.jsonl` and
  appends exactly one line to it; that line has `task` = the queued id, `result` `refused`,
  `failed_step` = the check, and `reason` equal to the reason the helper already extracts; the
  task branch head, the worktree status and the primary checkout's status are unchanged.
  `acceptance_t005.rs` `refusals_change_nothing`: its final assertion that the file's bytes are
  unchanged becomes: the bytes captured after the merged record are a prefix of the file's final
  content, and every line after that prefix is a `refused` record. Test names stay. No other
  assertion of a T-005 or T-013 test changes; any assertion that fails only because a refusal now
  adds its record commit is adapted the same way and listed in the implementer's log; one that
  fails for another reason stops the run with a `QUESTION:`. The `ledger` tests
  (`uncommitted_evidence_does_not_count`, and `acceptance_t013_queue.rs`'s
  `metrics/tasks.jsonl` case) use only the queued task's own evidence and pass unchanged under
  (2). These edits are the implementer's, on `task/T-043`, so that its gate can be green before
  the join (Q-3); the implementer adds no `acceptance_t*` file. Unit tests in `queue.rs` that
  assert a refusal writes nothing change with the code (listed in the log).
- (g) **Worked examples** (derived from the rule; not executed with cargo).
  (1) A toy task with one `queue` record (`gate_failed`, attempt 1) refused at `approval`: a
  second line `{"task":"T-003","attempt":2,…,"result":"refused","failed_step":"approval",…}`,
  `main` one commit ahead with subject `queue: record T-003 refused`; queued again after the row
  is committed and merged: `attempt` 3. A task refused at `review` and then merged: its `attempt:1`
  record is `refused`, so `metrics report` counts it as not first-pass.
  (2) Primary checkout with untracked `retrospectives/iter-1.md` and modified (unstaged)
  `tasks/T-019-project-tree.md`; `queue T-039` (diff touches neither): both class 3, the queue
  runs, both files are still untracked and modified afterwards. Same with modified
  `tasks/T-039-cli-memory-seed-undo-noop-byte-docs.md`: class 1, refused at `ledger`. Same with an
  untracked `notes.txt` at the root: class 4, the existing error.
- (h) **Scorecard items.** None.
- (i) **Guarantee sentences in `docs/protocol/`.** None.

## Acceptance criteria
All through the `xtask` binary in throw-away git repositories built with the T-005 fixture
(`acceptance_t005_support.rs`, included with `#[path]` as the T-013 tests do), `GIT_CONFIG_GLOBAL`
and `HOME` isolated, every spawned process with a timeout. Oracle: exact values unless stated; no
test asserts the wording of a reason or of the class-3 line beyond its prefix. AC-1 to AC-5 are
acceptance tests in `xtask/tests/acceptance_t043.rs` (each fails on base); AC-6 is in
`xtask/tests/guard_t043.rs` (passes on base).

- AC-1 (refusal record): for each of `approval`, `review`, `touches` and `main-history`, a toy
  task refused at that check: exit 1, one stderr line `queue: refused: <check>: …`; `main`
  advanced by exactly one commit that changes only `metrics/queue.jsonl` and whose body has
  `Task: <id>` and `Role: director`; the appended line has `task`, `result` `refused`,
  `failed_step` = the check, `reason` = the text after the stderr line's prefix, `head_sha` = the
  task branch's head, `base_sha` = `main` before the run, and every field of the record contract
  (`task, attempt, started_at, ended_at, base_sha, head_sha, result, failed_step`); the task
  branch head and its worktree status are unchanged.
- AC-2 (attempts): worked example (g) 1: attempts 1, 2, 3 in that order; and the next queue run
  of the same task after a refusal is not refused at `main-history` because of the refusal
  record commit.
- AC-3 (`dependencies` after the rebase): a toy task whose lock is refused at `dependencies` after
  the rebase (the case of `acceptance_t005_r4.rs`): one `refused` record with `failed_step`
  `dependencies`, the task branch back on its pre-run head.
- AC-4 (unread ledger files): worked example (g) 2 in a toy repository, with an untracked
  `retrospectives/x.md`, an unstaged modification of another task's file `tasks/T-099-*.md` and an
  untracked `handoffs/h.md`: `queue T-001` merges (exit 0, record `merged`), each of the three
  files is byte-for-byte as before and still untracked or modified (`git status --porcelain`
  lists them as before), and stderr has one line beginning
  `queue: ignoring uncommitted ledger paths not read for T-001: ` that names all three. The same
  three files do not stop `publish` of a new scorecard item (exit 0, `publish: T-NNN merged` is
  main's last subject) and are as before afterwards.
- AC-5 (evidence still refuses): each of these in the primary checkout makes `queue T-001` refuse
  at `ledger` with nothing changed and no record: an untracked `tasks/T-001-second.md`; a staged
  change to another task's file `tasks/T-099-*.md`; an unstaged change to a ledger path that
  T-001's branch also changes (a task whose `touches` include `docs/adr/`); an untracked file
  under `metrics/`. An untracked root file `notes.txt` gives exit 2 and no record.
- AC-6 (kept behaviour; guard): in `xtask/tests/guard_t043.rs`, a refusal at `ledger` for the
  queued task's own uncommitted `docs/decisions.md` row writes no record and leaves `main` and
  `metrics/queue.jsonl` unchanged; a queue run with a clean primary checkout merges as today.
  (Must not repeat an assertion of the T-005 or T-013 files.)
- AC-7 (no regression; named command): `cargo xtask ci` green in `.worktrees/T-043` and in the
  queue's main and own gates, with the T-005 tests changed only as Pinned (f) says (reviewer:
  `git diff main...task/T-043 -- xtask/tests/acceptance_t005* xtask/tests/acceptance_t013*`).

## Out of scope and implementer notes
- The join, `metrics` and the spec-check role: T-033. CI: T-041. Binding `merged` records to
  record commits (`main-history`): T-009 (stage 2).
- Recording the `ledger` refusal or the step-1 errors; a new `metrics report` line for refusals
  (the first-pass figure already counts them through `attempt`).
- Planners writing in a worktree of their own (Q-2, option B) is a director practice, not part
  of this task.
- Unit tests in `xtask/src` with `git.rs` `testrepo`. No timing or size bound is stated.
- Never set or export `CARGO_TARGET_DIR` in commands run by hand (D-019).
- When this task merges, METHODOLOGY §8's **(T-033)** marker for refusals becomes true through this
  task (the methodology names T-033 because the split was decided after v1.2 was written).

## Questions and rulings
Planner's options for the director (R-160); the spec above is written for the recommendation.
- Q-1 (does a refusal take an attempt number?). (A, recommended) yes: `attempt` counts every
  record of the task, so a refused first run is a first-pass failure (retro-1 named the two
  refusals as invisible to that metric). (B) No: a refusal record repeats the number the next
  real run will have, and first-pass success ignores refusals; the record is then evidence only.
  Under (A) iteration 2's first-pass figure is not comparable with iteration 1's, where refusals
  were not recorded; the retrospective should state that.
- Q-2 (dirty ledger files). (A, recommended) narrow the queue's and publish's checks to the
  evidence they read, as (2). Consequences: one rule in two commands; evidence for the task still
  refuses; a planner's half-written file under another task's id or under `retrospectives/` no
  longer forces a WIP commit; the ignored files are named on stderr. (B) Planners, analysts and
  evaluators write in a worktree of their own and the director brings their files to `main`:
  no tool change, but every such run needs a worktree the tool does not make today (`worktree new`
  takes only `ready` tasks), and the director gains a copy-and-commit step per run, in the role
  retro-1 counts as the main failure source (40 director errors). It also does not help when the
  director's own file is half-written. (C) Both: (A) now, (B) as practice where cheap.
- Q-3 (who edits the T-005 helpers). (A, recommended) the implementer, exactly as Pinned (f), so
  its gate is green before the join; the reviewer checks the diff (AC-7). (B) The test author,
  as in T-013 ruling D-1; then the implementer's gate is red on the predicted refusal tests until
  the join, and `self_verified` needs a ruling (T-013, R-058). Touches open question O-4.
- Q-4 (bundle). Item 2 could be its own task; it would change the same lines of `queue.rs` and the
  same `docs/development.md` passages, so it could not run beside this one, and each high-tier
  task costs a test author and a full review. Recommended: one task, for the reasons in the Goal.
- Q-5 (`gate_xtask: "none"`). A new value in an existing field; `metrics report` does not read
  the field. Alternative: `"main"` (the xtask that ran the preconditions). Recommended `none`.
### Rulings (director, D-098)
- Q-1: a recorded refusal takes an attempt number and counts against first-pass success.
- Q-2: option A. The check is narrowed to the evidence the queue reads for the task it merges; planners keep writing in the primary checkout.
- Q-3: the implementer edits the T-005 refusal helpers, the pins only and as the spec lists, logged under golden rule 4.
- Q-4: the dirty-file item stays in this task.
- Q-5: `gate_xtask` reads `none` on a refusal record.
- Order: after T-033.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13T21:40Z · R-160 · (new) · did: drafted from v1.2 change 9 (queue half, split from
  T-033 by D-091's bundling rule) and from the director's request on the three dirty-ledger
  refusals of 2026-11-13. Tier high: every `xtask/` path is a gate path. Read against main
  `b53d370f4a5f22eeffd34969b49f34170e0edf57`: `queue::run` returns a `Refusal` before any record
  is built (the record and attempt are computed after `precheck::check`), and the post-rebase
  `dependencies` refusal also leaves no record; `is_evidence_path` lists `metrics/`,
  `docs/decisions.md`, `tasks/`, `reviews/` and the two policy files; `precheck.rs` reads from
  main only the task file and `tasks/` listing, `docs/decisions.md`, `metrics/tasks.jsonl`,
  `metrics/queue.jsonl` and the policy files, and reads reviews from the task head; `T-005`'s
  refusal helpers compare `metrics/queue.jsonl` and `main` before and after (Pinned (f)); the
  existing `ledger` tests use only the queued task's own evidence. Pinned (h), (i) n/a. Unsure:
  git's behaviour with a staged unrelated change at `merge --ff-only` (class 2 is the safe
  choice, not measured). · gate attempts: 0 · not run: no cargo; worked examples derived from the
  rule · missing context: none · next: director rules Q-1 to Q-5; no spec check required.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
