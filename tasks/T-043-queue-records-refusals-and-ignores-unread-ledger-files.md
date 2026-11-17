---
id: T-043
title: "Write every queue precondition refusal to metrics/queue.jsonl as a committed record, and let the queue and publish proceed past dirty ledger files they do not read for the task"
tier: high              # ADR-0002 "a gate path": xtask/src, xtask/tests, xtask/platform-tests.toml; fixed at specified (v1.5 §4); the approval row must begin `Approve T-043` and contain `gate paths`
state: draft            # director only, written with every transition; see METHODOLOGY §3
owner_role: planner
scorecard_items: []
depends_on: [T-071]     # unmerged at R-310 (join running, branch head f7d5113): it edits xtask/tests/acceptance_t005_r3.rs, _r4.rs, xtask/src/queue.rs and publish.rs test modules and creates xtask/platform-tests.toml; this spec is re-checked after it merges (v1.5 change 10)
touches:                # one role per path (D-153); every path but docs/development.md is a GATE PATH
  - xtask/src/queue.rs                     # implementer: the refusal record (Interface 1), the dirty-path classes (Interface 2); its #[cfg(test)] module where a unit test asserts a refusal writes nothing (test names kept: `tests::basic_refusals_write_nothing` is a registry item)
  - xtask/src/publish.rs                   # implementer: step 1's working-tree rule (Interface 2) only
  - xtask/src/main.rs                      # implementer: only if the refusal record must be written where the refusal is printed
  - xtask/tests/acceptance_t005_support.rs # implementer: `Repo::assert_refused` only (Pinned (f))
  - xtask/tests/acceptance_t005_r3.rs      # implementer: the `refused` helper only (T-071 edits this file's cfg lines and `tamper_xtask_binaries`)
  - xtask/tests/acceptance_t005_r4.rs      # implementer: the `refused` helper only (T-071 edits this file's cfg lines and `record`)
  - xtask/tests/acceptance_t005.rs         # implementer: the last assertion of `refusals_change_nothing` only
  - xtask/tests/acceptance_t043.rs         # test author
  - xtask/tests/guard_t043.rs              # test author (behaviour main has that this task keeps)
  - xtask/platform-tests.toml              # test author: entries for its own two files only (Q-3 A)
  - docs/development.md                    # implementer: under "## Merge queue", steps 1–3 and the record example; under "## Publishing a ledger input", step 1; the "8, queue refused" row of "### After a stop"; not a gate path
budget_tokens: 1100000  # high (v1.5)
iteration: 2
found_by: incident      # unrecorded refusals: D-050, D-058 (retro-1), D-154, D-162 (retro-2c1), D-179 (retro-2c2); dirty-ledger refusals of 2026-11-13 (R-160)
origin_task: T-005
---

# T-043 Queue refusals recorded; unread dirty ledger files do not stop the queue or publish

Re-specified in full under methodology v1.5 (change 15: the draft of R-160 was written under v1.2)
by run R-310, at main `1289bf1070e4bda11e6ed08b4e7e514e6f62a155`. The scope additions recorded
since D-098 go to other tasks: see "Scope" below and Q-6.

## Goal
Two changes to the merge queue's checks before it rebases, in `cargo xtask queue` and
`cargo xtask publish`.

1. **Refusals recorded** (METHODOLOGY §8, v1.4 change 13). A refusal by one of the queue's
   preconditions appends one line to `metrics/queue.jsonl` and commits it on `main`, as every
   other outcome is recorded, so first-pass queue success counts it. Today a refusal writes
   nothing (`xtask/src/queue.rs:545-546`: "Refusals return `Err` and write nothing"). Refusals
   found only in decision rows: D-050, D-058 (iteration 1), D-154, D-162 (cut 1), D-179 (cut 2,
   T-069 at `touches`).
2. **Unread ledger files do not stop the queue.** An uncommitted or untracked file under a ledger
   path that the queue does not read as evidence for the task, and that the merge does not
   change, no longer makes `queue` or `publish` refuse. A dirty file that is evidence for that
   task still refuses. On 2026-11-13 the director had to commit other agents' half-written files
   three times to proceed (R-160: `publish` of T-040 for `retrospectives/scripts/iter1_*`, its
   queue step for `retrospectives/iter-1.md`, `queue T-039` for `tasks/T-019-*.md`).

Both are the queue's working-tree and precondition handling in `queue::run` before the rebase.
Item 1 must say which refusals are recorded; the `ledger` refusal is the one it cannot record
(Pinned (a)). Both change the same passages of `docs/development.md` (D-098 Q-4: one task).

**Gate paths.** Every `xtask/` path in `touches` is a gate path, so the task is tier high. The
approval row must begin `Approve T-043` and contain `gate paths`. No dependency changes.

Reading list: METHODOLOGY §8, §8.1, §13; `docs/development.md` "## Merge queue" (steps 1–3,
"### Preconditions", the record paragraph), "## Publishing a ledger input"; `xtask/src/queue.rs`
(`run` from line 547, `is_evidence_path` line 261, `commit_record` line 773), `xtask/src/precheck.rs`
(the module comment, every `show(root, "main", …)` and `committed_jsonl` call),
`xtask/src/publish.rs` step 1, `xtask/ledger.toml`; `xtask/tests/acceptance_t005_support.rs`
(`Repo`, `assert_refused`), `acceptance_t005_r3.rs` and `_r4.rs` (`refused`), `acceptance_t005.rs`
(`refusals_change_nothing`); `xtask/platform-tests.toml` (after T-071).

### Scope: what is in T-043 and what goes elsewhere
The additions recorded since D-098 are not all queue-refusal work; one ceiling cannot hold them
(the two tasks that built the queue cost 1,672,352 (T-005) and 1,790,657 (T-013) tokens, by
`metrics/tasks.jsonl`). Proposed split (Q-6), in this order after T-071 merges:

| Task | Purpose | Contents | State of the file |
|---|---|---|---|
| **T-043** (this) | queue: refusals recorded; unread ledger files | Interface 1, 2 | full spec |
| T-079 | product tests marked by platform (D-219: after T-043) | as specified | specified |
| **T-087** | two queue precondition rules added since | tier ≥ the highest tier of the claimed items, checked at `front-matter` (v1.4 change 1, D-174 (1)); a review file allowed in a low-tier branch's diff (D-179) | draft |
| **T-088** | record checks of the metrics and hygiene tools | `metrics transition` writes the task file's state line (D-144 change 12, D-146); `metrics add` refuses `started_at` = `ended_at` with tokens over 10,000 (D-144 change 12); every cited `D-NNN` in `tasks/`, `reviews/`, `docs/` has a row (gate `hygiene`, D-144 change 12); `killed_by_own_tests` required in review front matter (v1.5 change 9) | draft |
| **T-089** | CI record extraction | `cargo xtask ci-record` turns a directory of downloaded gate artefacts into `tasks/ci-runs/T-NNN-<round>.md` (v1.5 change 1); after T-072, which fixes the artefacts' content | draft |
| **T-090** | the queue checks a pending CI criterion | a task with a CI criterion pending at its join is refused at a new check until a CI record for its rebased head shows every counted job green (v1.5 change 11, ADR-0009 Decision 5's proposal) | draft |

Not T-043's, and not in the drafts above:
- `acc_test_defects` with who found each is a field of the join record. The join is T-033's tool
  (T-033 already owns join counting and `--acc-defects`). D-219 (9) says "into the tools with
  T-043"; Q-7 asks whether it moves to T-033.
- `incidents/` as a ledger path changes `xtask/ledger.toml`. `platforms.yml`'s `paths-ignore` is
  built from that file, and `xtask/tests/acceptance_t070.rs` checks the two agree (lines 933–937),
  so the change also edits `.github/workflows/platforms.yml`. It belongs with T-041 (which builds
  `ci.yml`'s same list from `ledger.toml`), or with T-072 (which edits `platforms.yml`) (Q-8).
- Booking-order refusals already exist: `metrics add` refuses an end time earlier than the task's
  latest record (R-297's refused booking) and one later than now (R-301; `xtask/src/metrics.rs`
  lines 344–357, 506–513). Nothing is to be built for them.
- `worktree new` refuses a task that is not `ready` in main's committed file
  (`xtask/src/worktree.rs:72-105`, by design). The fix is T-088's state line, not a change to
  `worktree`.
- Join tooling (counting by path, scorecard regeneration, tree identity of a rebuilt branch,
  one role per path at the join, `acc_defects` classification): T-033. The ledger-only CI
  filter for `ci.yml`: T-041.

## Interface
Every command keeps its name, options, output and exit codes unless a change is listed here.

### (1) A refusal is a record
When `cargo xtask queue T-NNN` refuses at a **recorded check** (Pinned (a)), it still prints its
one stderr line `queue: refused: <check>: <reason>` and exits 1, and in addition:
- appends one line to `metrics/queue.jsonl` with the existing fields and one more, `reason`:
  ```json
  {"task":"T-NNN","attempt":3,"started_at":"…Z","ended_at":"…Z","base_sha":"<main>",
   "head_sha":"<task head as found>","result":"refused","failed_step":"<check>",
   "gate_seconds":0.0,"gate_xtask":"none","gate_xtask_sha":"<base_sha>","regenerated":[],
   "reason":"<the text after `queue: refused: <check>: `>"}
  ```
  - `attempt` is 1 + the number of the task's earlier records, refusals included (D-098 Q-1).
  - `head_sha` is the task branch's head when the run began; a refusal never moves the branch.
  - `regenerated` lists the generated paths regenerated before a refusal after the rebase, else `[]`.
  - `gate_xtask` is `none` (D-098 Q-5).
  - The fields `gate_paths` and `unknown_token_runs` are written as for other outcomes.
- commits that line on `main` as `commit_record` commits the other outcomes: one commit that
  changes only `metrics/queue.jsonl`, with subject `queue: record T-NNN refused` and body lines
  `Task: T-NNN` and `Role: director`. It is a ledger commit, so the next `main-history` check
  tolerates it. If the commit fails, the queue behaves as it does today when another record's
  commit fails; the refusal line is printed first.
- changes nothing else (AC-1 as an equality of changed paths).

`publish` passes the queue's output and exit code through in its step 8, as today. A refusal
there also leaves the refusal record commit on `main`.

### (2) Which dirty files stop the queue
Before its preconditions the queue looks at every path of the primary checkout that `git status`
reports modified, staged or untracked (ignored files are not seen). Each path is in exactly one
class, decided in this order:
1. **evidence for the task** → refusal at `ledger`, as today (exit 1, one line, nothing changed,
   no record). Evidence is:
   - anything under `metrics/`;
   - `docs/decisions.md`;
   - a path beginning `tasks/T-NNN-` (the task's own file, or another file that would make the
     `task-file` check see two);
   - anything under `reviews/`;
   - `xtask/generated.toml` and `xtask/ledger.toml`;
   - any path the task branch changes (`git diff --name-only --no-renames <merge-base> <task head>`).
     The fast-forward stops on such a path (probe P1).
2. **unread ledger file**: modified (staged or not) or untracked, under an entry of main's
   committed `xtask/ledger.toml` (any mode), and not class 1 → ignored. The queue goes on and
   prints one stderr line beginning `queue: ignoring uncommitted ledger paths not read for T-NNN: `
   that names them. It leaves each exactly as it found it: contents, and staged, modified or
   untracked state.
3. anything else → the existing working-tree error (exit 2, no record), unchanged.

If main's `xtask/ledger.toml` cannot be read or is invalid, class 2 is empty: every dirty path
outside class 1 is class 3, and the `config` precondition refuses as today.

`publish` applies the same classes in its step 1 to the paths that are not its inputs. There
`T-NNN` is its new task id (whose files cannot exist yet), and "the paths the branch changes" are
its inputs and `docs/scorecard.md`. Its refusal line stays `publish: refused: …`. Its later steps
commit only their own paths, as today, and its step 8 meets the same rule in the queue.

(The R-160 draft also refused a **staged** unrelated ledger change, as a guess that `git merge`
may abort when the index differs. Probe P1 shows it does not, and the queue's record commit names
its path, so staged unrelated changes are class 2 here. Q-2 asks the director to confirm.)

### (3) `docs/development.md`
These passages state what the tools then do. The wording is the implementer's; each sentence
follows Pinned (i):
- "## Merge queue" step 1: the classes of (2) replace "the working tree has (non-evidence)
  changes outside `metrics/`".
- Step 2: the evidence list becomes class 1.
- Step 3: "nothing changed and no record written" becomes the record of (1).
- The record example's `result` list gains `refused`, and the new field `reason` is named.
- "## Publishing a ledger input" step 1: its working-tree rule follows (2).
- "### After a stop" row "8, queue refused": the refusal record is committed on `main` as well.

### Platforms (ADR-0009, the platform-test rule of T-071)
The queue, worktree and publish tools run on the director's Linux machine only (ADR-0009 (c);
T-071 ruling Q-5 A, D-199). The new test files `acceptance_t043.rs` and `guard_t043.rs` build toy
repositories and run the built `xtask` as the T-005 tests do. They are behind
`#![cfg(target_os = "linux")]` at file level, each with one entry in `xtask/platform-tests.toml`
(`item = "<file>"`, `platforms = "linux"`, `counterpart = "none"`, the reason T-071's entries
give). The entry's `doc`/`quote` names the record sentence the implementer adds to
`docs/development.md` (D-n1, Pinned (i)). Q-3 offers the alternative. The edited T-005 helpers
keep T-071's cfg lines as they are.

## Pinned
- (a) **Recorded checks**, as an equality: the set of checks whose refusal writes a record is
  exactly {`task-file`, `front-matter`, `state`, `config`, `touches`, `join`, `review`,
  `gate-path`, `dependencies` (before or after the rebase), `approval`, `budget`,
  `main-history`}. That is every check that prints `queue: refused: <check>: ` except `ledger`
  (precheck.rs module comment and the `const` names at lines 584–1000 of main `1289bf1`). Not
  recorded:
  - `ledger`: the record would be committed while evidence, possibly `metrics/` itself, is
    uncommitted;
  - the step-1 errors (exit 2): `main` not checked out, the worktree missing, dirty or on another
    branch, no commits beyond `main`, a class-3 path. These are not refusals.

  Variant outside the list, to be tested: a `ledger` refusal leaves `metrics/queue.jsonl` and
  `main` unchanged (AC-5). A refusal record is written while the queue lock is held. Exit codes
  are unchanged: 1 for a refusal, 2 for errors. If a later task adds a check (T-087's tier rule is
  inside `front-matter`; T-090 adds one), it is recorded unless that task's spec says otherwise.
- (b) **Files.** `metrics/queue.jsonl`: appended and committed, one line per refusal, fields as
  in (1). The queue also reads main's committed `xtask/ledger.toml` for class 2, and writes
  nothing else.
  - Test files: no package has a target named `acceptance_t043*` or `guard_t043*`
    (`find . -path ./target -prune -o -path ./.worktrees -prune -o \( -name 'acceptance_t043*' -o -name 'guard_t043*' \) -print`
    printed nothing on main `1289bf1`, R-310).
  - `xtask/platform-tests.toml` exists on T-071's branch (`git show f7d5113:xtask/platform-tests.toml`).
- (c) **Order of checks.**
  1. Lock.
  2. `main` checked out.
  3. The classes of (2): class 1 present gives `ledger`; class 3 present gives the error; class 2
     gives the one line.
  4. The worktree checks of today.
  5. The preconditions in today's order. The first failure writes and commits the refusal record,
     then exits 1.
  6. Build of main's xtask, rebase, … as today. The post-rebase `dependencies` refusal also
     writes its record after the branch is put back.
- (d) **Paths and platform.** Repository-relative paths with `/`. Quoted paths in `git status`
  output are unquoted before classification, as `is_evidence_path` does today
  (`p.trim_matches('"')`). Linux (Platforms).
- (e) **Threat model** (D-026: honest mistakes and corner-cutting).
  - The `ledger` refusal exists so that evidence the director believes committed is not silently
    ignored (D-026 R6, D-027). Class 1 keeps that for everything the preconditions read from main:
    the task file and the `tasks/` listing, `docs/decisions.md`, `metrics/tasks.jsonl`,
    `metrics/queue.jsonl`, and the two policy files.
  - Review files are read from the task head. `reviews/` stays in class 1 as today, since it is
    not a ledger path.
  - A dirty path the task's diff changes is class 1, because `git merge --ff-only` stops on it
    (P1).
  - Not defended: a director who leaves another task's evidence uncommitted and then queues that
    other task. That run is refused at `ledger`, as today.
- (f) **Existing tests this task changes.**
  - The refusal helpers assert that a refusal changes nothing, including `metrics/queue.jsonl`
    and main's head. They are `acceptance_t005_support.rs` `Repo::assert_refused` (line 581),
    `acceptance_t005_r3.rs` `refused` (line 107) and `acceptance_t005_r4.rs` `refused` (line 94).
    Helper call lines on main `1289bf1`, by
    `grep -c "assert_refused(\|refused(&r\|refused(r" xtask/tests/acceptance_t005*.rs xtask/tests/acceptance_t013_queue.rs`:
    t005 35, t005_r2 7, t005_r3 22, t005_r4 13, support 1, t013_queue 14.
  - Each helper keeps every assertion it has for check `ledger`. For any other check, "nothing
    changed" becomes: `main` advanced by exactly one commit whose parent is the old `main`. That
    commit changes exactly {`metrics/queue.jsonl`} and appends exactly one line. The line has
    `task` = the queued id, `result` `refused`, `failed_step` = the check, and `reason` equal to
    the reason the helper already extracts. The task branch head, the worktree status and the
    primary checkout's `git status --porcelain` are equal to before.
  - `acceptance_t005.rs` `refusals_change_nothing`: its final byte-equality assertion becomes:
    the bytes captured after the merged record are a prefix of the final content, and every line
    after the prefix is a `refused` record.
  - Test names stay. Any other assertion of a T-005 or T-013 test that fails only because a
    refusal now commits its record is adapted the same way and listed in the implementer's log.
    One that fails for another reason stops the run with a `QUESTION:`.
  - The `ledger` tests use only the queued task's own evidence and pass unchanged (R-160's
    reading; to be re-checked after T-071 merges).
  - Unit tests in `queue.rs` that assert a refusal writes nothing change with the code and keep
    their names. `tests::basic_refusals_write_nothing` is a registry item of T-071, so the name
    cannot change without the test author's registry edit (one role per path).
  - These edits are the implementer's, on `task/T-043`, so that its gate is green before the join
    (D-098 Q-3). The implementer adds no `acceptance_t*` file.
- (g) **Worked examples.**
  - E1 (derived from the rule): a toy task with one `gate_failed` record (attempt 1) refused at
    `approval` gets a second line `{"task":"T-003","attempt":2,…,"result":"refused","failed_step":"approval",…}`,
    and `main` is one commit ahead with subject `queue: record T-003 refused`. Queued again after
    the row is committed and merged, it gets attempt 3.
  - E2, executed with git 2.43.0 in a scratch repository under `/home/user` (R-310; P1). Commands:
    `git init -b main`; base commit with `tasks/T-099.md`, `retrospectives/x.md`, `code.txt`;
    branch `task` changes `code.txt`; on `main`: `echo staged > tasks/T-099.md; git add tasks/T-099.md;
    echo un > retrospectives/x.md; echo new > retrospectives/new.md; git merge --ff-only task`.
    Result: exit 0, fast-forwarded; `git status --porcelain` afterwards is
    `" M retrospectives/x.md"`, `"M  tasks/T-099.md"`, `"?? retrospectives/new.md"`, so staged,
    unstaged and untracked unrelated changes survive. Then, at the base again with
    `echo staged2 > code.txt; git add code.txt; git merge --ff-only task`: "Please commit your
    changes or stash them before you merge. Aborting", exit 1. A dirty path the merge changes
    stops it.
  - E3 (derived): primary checkout with untracked `retrospectives/iter-1.md` and unstaged
    `tasks/T-019-project-tree.md`, then `queue T-039` (whose diff touches neither). Both are
    class 2; the queue runs; both are as before afterwards. With a modified
    `tasks/T-039-cli-memory-seed-undo-noop-byte-docs.md` instead: class 1, `ledger`. With an
    untracked `notes.txt` at the root: class 3, the existing error.
- (h) **Scorecard items.** None.
- (i) **Sentences for `docs/development.md`**, worded for copying, each with its evidence:
  - D-n1 "A refusal by a precondition (every check but `ledger`) is recorded: the queue appends
    one line with `result` `refused`, `failed_step` the check and `reason` the refusal's text to
    `metrics/queue.jsonl` and commits it on `main`; it counts as an attempt." **Unchecked** (new
    behaviour; its test is AC-1, AC-2).
  - D-n2 "Uncommitted changes to ledger paths that are not evidence for the queued task are left
    as they are and named on one stderr line; evidence for the task still refuses at `ledger`."
    **Unchecked** (new behaviour; AC-4, AC-5).
  - D-n3 "A staged, unstaged or untracked change to a path the merge does not change survives the
    fast-forward; a change to a path it changes stops it." Evidence: E2 (git 2.43.0, R-310).
  - Kept, unchanged sentences of step 2 ("Evidence is read only from main's committed tree…")
    carry their existing evidence (D-026 R6; `guard_t043` AC-6).

## Acceptance criteria
All tests run the built `xtask` binary in throw-away git repositories built with the T-005 fixture
(`acceptance_t005_support.rs`, included with `#[path]` as the T-013 tests do), with
`GIT_CONFIG_GLOBAL` and `HOME` isolated and every spawned process under a timeout. Oracle: exact
values unless stated. No test asserts the wording of a reason or of the class-2 line beyond its
prefix. AC-1 to AC-5 are in `xtask/tests/acceptance_t043.rs`; each fails on base, where a refusal
writes nothing and dirty ledger files refuse. AC-6 is in `guard_t043.rs` and passes on base.

- AC-1 (refusal record): for each of `approval`, `review`, `touches`, `budget` and
  `main-history`, a toy task refused at that check:
  - exit 1, one stderr line `queue: refused: <check>: …`;
  - `main` advanced by exactly one commit, whose changed paths equal {`metrics/queue.jsonl`} and
    whose body holds `Task: <id>` and `Role: director`;
  - the appended line has `task`, `result` `refused`, `failed_step` = the check, `reason` = the
    text after the stderr prefix, `head_sha` = the task branch's head and `base_sha` = `main`
    before the run, and its key set equals the record contract (the fields of `QueueRecord` plus
    `reason`);
  - the task branch head and `git status --porcelain` of its worktree are equal to before.
  - Variant outside the list: a record whose commit also touches another path fails.
- AC-2 (attempts): example E1. Attempts 1, 2 and 3 in that order. The run after the refusal is not
  refused at `main-history` because of the refusal record commit.
- AC-3 (`dependencies` after the rebase): the toy case of `acceptance_t005_r4.rs` whose lock is
  refused after the rebase writes one `refused` record with `failed_step` `dependencies`, and the
  task branch is back on its pre-run head.
- AC-4 (unread ledger files): in a toy repository, an untracked `retrospectives/x.md`, an unstaged
  modification and a staged modification of another task's file (`tasks/T-099-*.md`, two runs),
  and an untracked `handoffs/h.md`:
  - `queue T-001` merges (exit 0, record `merged`);
  - `git status --porcelain` and each file's bytes after the run are equal to before (variant: a
    tool that commits or stages them fails);
  - stderr holds one line beginning `queue: ignoring uncommitted ledger paths not read for T-001: `
    whose named paths equal the set of those files;
  - the same files do not stop `publish` of a new scorecard item (exit 0, main's last subject
    `publish: T-NNN merged`) and are equal to before afterwards.
- AC-5 (evidence still refuses, with no record): each of these in the primary checkout makes
  `queue T-001` refuse at `ledger`, with `main`, `metrics/queue.jsonl` and `git status --porcelain`
  equal to before:
  - an untracked `tasks/T-001-second.md`;
  - an unstaged change to a ledger path that T-001's branch also changes (a task whose `touches`
    include `docs/adr/`);
  - an untracked file under `metrics/`.

  An untracked root file `notes.txt` gives exit 2 and no record.
- AC-6 (kept behaviour; guard, `guard_t043.rs`): a refusal at `ledger` for the queued task's own
  uncommitted `docs/decisions.md` row writes no record. A queue run from a clean primary checkout
  merges with one `merged` record, as today. No assertion repeats one of the T-005 or T-013 files.
- AC-7 (no regression; named command): `cargo xtask ci` is green in `.worktrees/T-043` and in the
  queue's two gates. The T-005 test files are changed only as Pinned (f) says (reviewer:
  `git diff main...task/T-043 -- xtask/tests/acceptance_t005* xtask/tests/acceptance_t013*`).

## Out of scope and implementer notes
- Everything listed under "Scope" for T-087 to T-090, T-033 and T-041.
- Recording the `ledger` refusal or the step-1 errors; a new `metrics report` line for refusals
  (the first-pass figure counts them through `attempt`).
- Unit tests in `xtask/src` with `git.rs` `testrepo`. No timing or size bound is stated.
- Never set `CARGO_TARGET_DIR` in commands run by hand (D-019).

### Collisions and order
- **T-071** (in its join, head `f7d5113`): shares `acceptance_t005_r3.rs`, `_r4.rs`,
  `queue.rs`, `publish.rs` and `platform-tests.toml`. T-043 is dispatched after T-071 merges
  (D-219), and this spec is re-checked then. In particular, check whether T-071 changed the T-005
  helpers' lines or `basic_refusals_write_nothing`.
- **T-079** (specified): edits `xtask/platform-tests.toml`. It runs after T-043 (D-219), not
  beside it.
- **T-072** (draft, workflow report steps): shares no path with T-043. **T-089** (after T-072)
  and **T-090** (after T-089) follow.
- **T-033** (draft, join tooling): its `touches` include `xtask/src/main.rs` and
  `docs/development.md`, so it is not in flight with T-043.
- **T-041** (draft, last): shares `docs/development.md` and possibly `xtask/ledger.toml` (Q-8),
  so it is not in flight with T-043.
- **T-087, T-088**: share `docs/development.md` (and T-087 shares `precheck.rs`'s callers), so
  they run after T-043, one at a time.

### Estimate
The `metrics/tasks.jsonl` record for comparable `xtask` tasks with an independent test author:
- T-054 455,026;
- T-070 809,362;
- T-071 643,142 so far (before its review);
- T-005 1,672,352 and T-013 1,790,657, but those built the queue and the publish tool.

T-043 changes two behaviours and adapts three helpers (92 call lines). Estimate about 850,000 to
1,000,000, under the 1,100,000 ceiling, with the helpers' adaptation as the main risk. The
additions moved to T-087 to T-090 would have taken it over.

## Questions and rulings
### Rulings carried from D-098 (R-160's questions; confirmed or reopened here)
- Q-1 (a refusal takes an attempt number): A, as D-098. Not reopened.
- Q-4 (one task for both items): A, as D-098. Not reopened.
- Q-5 (`gate_xtask: "none"`): as D-098. Not reopened.
- D-098 "Order: after T-033" is replaced by D-219 (after T-071, before T-079): T-033 has not run.

### Questions for the director (R-310)
- **Q-2. Staged unrelated ledger changes.**
  - A: class 2, ignored like unstaged ones (as specified). Measured (E2): the fast-forward keeps
    them, and the record commit names its path.
  - B: refuse them at `ledger`, as R-160 proposed. That was unmeasured then and is contradicted by
    E2.
  - **Recommendation: A.**
- **Q-3. Platform of the new test files.**
  - A: Linux-only at file level, with registry entries, as T-071 did for the same tools (ADR-0009
    (c), D-199).
  - B: written to run on all three platforms. **Unmeasured**: whether the T-005 fixture's toy
    repositories run on macOS and Windows is not known (the macOS xtask suite stopped at 3 failing
    unit tests, and on Windows the xtask tests did not compile, `tasks/ci-runs/T-054-round-1.md`
    lines 79–128). Not recommended without a probe on `ci/T-043`.
  - **Recommendation: A.**
- **Q-6. The split** (table under "Scope").
  - A: T-043 as specified; T-087, T-088, T-089, T-090 as drafts in that order, each high.
  - B: fold T-087 into T-043 (its two rules are small). This brings the estimate to or over the
    ceiling, and `precheck.rs` joins `touches`.
  - C: as A, and defer T-089 and T-090 until T-072 has fixed the artefacts' content.
  - **Recommendation: A, with C's deferral of T-089 and T-090**: the queue work before the feature
    phase; the CI tools when T-072 has run.
- **Q-7. `acc_test_defects`.**
  - A: move it to T-033, the join tool it belongs to.
  - B: keep it in T-043's line as D-219 (9) says. That means a join-record change inside a queue
    task, and `join.rs` in `touches`.
  - **Recommendation: A.**
- **Q-8. `incidents/` as a ledger path.**
  - A: with T-041 (it edits `ci.yml`'s ledger filter from `ledger.toml`).
  - B: with T-072 (it edits `platforms.yml`).
  - C: in T-043 (adds `xtask/ledger.toml` and `.github/workflows/platforms.yml` to `touches`, and a
    test author's change of `acceptance_t070`'s expectation).
  - **Recommendation: B**: T-072 comes before T-041 and already edits `platforms.yml`; the
    `ledger.toml` line rides with it.

### Spec check R-310
Re-specification in full (v1.5 change 15), at main `1289bf1`. Read: the R-160 spec and D-098;
v1.5 (`CHANGELOG.md`); D-144, D-174, D-179, D-219; the retro-2c2 sections on T-043; main's
`queue.rs`, `precheck.rs`, `metrics.rs`, `worktree.rs`, `ledger.toml`; T-071's branch
(`git diff --stat main...f7d5113 -- xtask/`, its `platform-tests.toml`, its edits of
`acceptance_t005_r3.rs` and `_r4.rs`); `acceptance_t070.rs`'s ledger check; the CI record of
T-054's round 1.

Changed against R-160:
- `depends_on` [T-005, T-013] → [T-071].
- Budget restated (v1.5).
- Class 2 of the old spec (staged) merged into "unread" after probe E2 (Q-2).
- The platform statement and the registry entries added (Q-3).
- The scope additions sorted into T-087 to T-090, T-033 and T-041 (Q-6 to Q-8).
- Absence criteria written as equalities, each with one variant outside the list (v1.5 change 3).
- Every Pinned sentence carries a command or is marked unchecked (v1.5 change 4).
- Collisions with T-071, T-079, T-072, T-033 and T-041.

Measured: E2 (git 2.43.0, scratch repository under `/home/user/kiln-plan-T-043/`, deleted). No
build was needed: no Pinned sentence states a figure of the binary.

Not checked:
- the T-005 helpers' lines after T-071's merge (re-check);
- whether `basic_refusals_write_nothing` needs other edits than its assertions;
- the macOS and Windows behaviour of the fixture (Q-3 B);
- the `publish` step-1 code path in detail (read only in R-160).

### Rulings on spec R-310 (director, D-223)
- **Q-2: A.** Staged changes to ledger files that are not evidence for the task are treated like unstaged ones
  (the planner's probe with git 2.43.0: `git merge --ff-only` keeps them).
- **Q-3: A.** The new test files are Linux-only at file level with registry entries, as T-071 did for the same
  tools.
- **Q-6.** The split is accepted (T-043, T-087, T-088, T-089, T-090). **The whole tooling batch waits behind the
  platform track** and is put to the owner with the platform report: five high-tier tasks of the director's own
  tools at about 0.9 M tokens each are a spend the owner decides (D-223 changes D-219 in this point).
- **Q-7: A.** `acc_test_defects` belongs to T-033 (the join tool).
- **Q-8: B.** `incidents/` as a ledger path goes with T-072, which edits `platforms.yml`.
- This spec is re-checked against main before any dispatch (T-071 will have merged; v1.5 change 10).

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13T21:40Z · R-160 · (new) · did: drafted from v1.2 change 9 (queue half, split from
  T-033) and the three dirty-ledger refusals of 2026-11-13; read against main `b53d370`. · gate
  attempts: 0 · next: director (Q-1 to Q-5, ruled D-098).
- 2026-11-17 · R-310 · draft · did: re-specified in full under v1.5 at main `1289bf1` (see "Spec
  check R-310"); split the recorded additions into T-087 to T-090 (drafts) and named what goes to
  T-033 and T-041; probe E2 with git; 7 criteria. · gate attempts: 0 · not run: no build (no
  figure pinned), macOS, Windows · missing context: T-071 as merged · next: director (Q-2, Q-3,
  Q-6 to Q-8), then the re-check after T-071 merges.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
