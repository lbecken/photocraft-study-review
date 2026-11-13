---
id: T-033
title: "Make the join count acceptance targets by path, regenerate docs/scorecard.md before its gate, require --acc-defects for a failing join, drop join --revert-check and metrics join-defects, and book spec checks under their own role"
tier: high              # ADR-0002 "a gate path": xtask/src, xtask/tests; the approval row must contain `gate paths`
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-005, T-013]   # both merged before this spec was written; no spec check is required by METHODOLOGY §5.7
touches:
  - xtask/src/join.rs                    # GATE PATH: implementer (items 1–4)
  - xtask/src/testbins.rs                # GATE PATH: implementer, only if a test binary must carry its source path (item 1)
  - xtask/src/main.rs                    # GATE PATH: implementer: `join` and `metrics` options and the usage text (items 3, 4, 5, 6)
  - xtask/src/metrics.rs                 # GATE PATH: implementer: `join-defects` removed, `report` exit, the spec-check role (items 5, 6)
  - xtask/tests/acceptance_t013_report.rs  # GATE PATH: implementer: only the two edits of Pinned (f) 3
  - xtask/tests/acceptance_t013_docs.rs    # GATE PATH: implementer: only the one edit of Pinned (f) 4
  - xtask/tests/acceptance_t033.rs       # GATE PATH: test author
  - xtask/tests/guard_t033.rs            # GATE PATH: test author (behaviour main already has that this task must keep)
  - docs/development.md                  # implementer: the "Scorecard", "Join" and "Metrics" passages of Interface (7); not a gate path
budget_tokens: 1100000  # high (METHODOLOGY §11, v1.2)
iteration: 2
found_by: reviewer      # D-033 (T-005 r3 follow-ups); the v1.2 tool parts are retro-1 changes 1, 8, 9, 10 (D-091)
origin_task: T-005
---

# T-033 Join by path, scorecard page at join, classified failing joins, no revert check, spec-check role

## Goal
After this task the director can join a task whose acceptance or guard files share a file stem
with another package's, and the record counts each file's tests; the join leaves a current
`docs/scorecard.md` on the task branch, so the joined gate no longer fails at `scorecard` for a
stale page; a join whose acceptance tests fail cannot be recorded without the director's
classification of those failures; the commands the method no longer uses (`join --revert-check`,
`metrics join-defects`) are gone; and a spec check is booked to `iter-N` under a role of its own.

The task is the tool part of methodology v1.2 changes 1, 8, 9 (join half) and 10
(`methodology/CHANGELOG.md` v1.2; D-091; D-093). The queue half of change 9 (a precondition
refusal is written to `metrics/queue.jsonl`) is T-043, split off under D-091's rule that a
high-tier spec bundling independent items is split (see Questions, Q-1). The CI half of v1.2 is
T-041. Nothing else of the iteration-1 draft of this task remains.

Incidents behind each item: T-018's first join recorded 6 of 12 acceptance tests because two
`acceptance_t018.rs` files in different crates were identified by stem, and ruling x6 renamed a
file (D-084; retro-1 "Tooling and environment"); the joins of T-011 and T-008 failed the gate at
`scorecard` for a stale page and the director regenerated it by hand (D-075); 28 joins have
`acc_defects` null and the two failing T-013 joins were never classified, `metrics join-defects`
never used (retro-1); 5 revert checks ran in iteration 1 and found no defect (retro-1, change 10).

**Gate paths.** Every `xtask/` path in `touches` is a gate path (`docs/development.md`
"Preconditions", `gate-path`), so the task is tier high; the approval row must begin
`Approve T-033` and contain the words `gate paths`. No dependency changes: no `Cargo.toml` or
`Cargo.lock` edit. The queue runs main's gate and the task's own gate; both must pass.

**Dispatch.** `docs/development.md` is in the `touches` of T-041 and T-043 as well, so none of
the three is in flight with another (METHODOLOGY §5.2). No stage-1 product task (ADR-0005)
touches any path listed here.

Reading list: `methodology/METHODOLOGY.md` §3, §6 (Scorecard, Join), §13; `methodology/CHANGELOG.md`
v1.2 rows 1, 8, 9, 10; `docs/decisions.md` D-021, D-075, D-084, D-091, D-093;
`retrospectives/iter-1.md` "Tooling and environment"; `docs/development.md` "Scorecard", "Join",
"Metrics"; `xtask/src/join.rs`, `xtask/src/testbins.rs`, `xtask/src/metrics.rs`,
`xtask/src/main.rs`, `xtask/generated.toml`; `xtask/tests/acceptance_t013_report.rs`,
`xtask/tests/acceptance_t013_docs.rs`, `xtask/tests/acceptance_t005_r2.rs`
(`join_finds_acceptance_and_guard_tests_under_xtask_tests`).

## Interface
Every command keeps its name, options, output, exit codes and record fields unless a change is
listed here. Record field names are the contract of METHODOLOGY §13 and do not change.

### (1) Target identity: by path
An acceptance target is a file `acceptance_tNNN*.rs` and a guard target a file `guard_tNNN*.rs`
directly in a `crates/*/tests/`, `apps/*/tests/` or `xtask/tests/` directory (the places join
searches today). A target is identified by its **repository-relative path** (for example
`crates/kiln-control/tests/acceptance_t018.rs`), and a test by (target path, test name). Two
files with the same stem in different packages are two targets: both are built, both are run on
base and on the joined branch, and each of their tests is counted once. In the `join` record:
- `acc_total` = tests of the acceptance targets that compiled on the joined branch;
- `acc_fail_base` = tests that did not pass on base (failed tests, plus, for a target that did not
  compile on base, the number of its tests on the joined branch);
- `acc_fail_first_join` = failing tests on the joined branch plus one per acceptance target that
  did not compile there (as today);
- `guard_total`, `guard_fail_base`, `guard_fail_join` by the same identity;
- `base_fail_kind` by today's rules, applied per target path.
Wherever join names a test (the `passes on base` refusal, the refusal of (3)), it names it
`<target path>::<test name>`, and a target that did not compile as `<target path>`.

### (2) `docs/scorecard.md` regenerated before the join's gate
When the acceptance tests and the guards pass on the joined branch, and before join runs the
gate there, join runs in `.worktrees/T-NNN` the command that main's `xtask/generated.toml` lists
for `docs/scorecard.md` (today `{xtask} scorecard`), with `{xtask}` standing for the xtask binary
that runs the join (the primary checkout's, as for the queue and `publish`).
- If the command succeeds and `docs/scorecard.md` changed, join commits exactly that file on
  `task/T-NNN` with subject `join: regenerate docs/scorecard.md` and body lines `Task: T-NNN`
  and `Role: director`, using the repository's commit configuration (hooks and signing kept).
  The record's `joined_sha` is the head after this commit.
- If the command succeeds and the file is unchanged, there is no commit.
- If the command fails, join commits nothing and runs the gate, whose `scorecard` step then
  reports the cause; the record says `gate_failed` as for any red gate.
- If the command changes any other path, or the commit fails, join exits 2 with one stderr line
  beginning `join: regenerating docs/scorecard.md failed: `, puts `task/T-NNN` back on the head it
  had before the join (merge undone, worktree clean) and appends no record.
- If main's `xtask/generated.toml` lists no `docs/scorecard.md`, this step does nothing.
Join never writes in the primary checkout or on `main` (apart from appending its record to the
primary checkout's `metrics/tasks.jsonl`, as today).

Scorecard items are tied to tests by the director (METHODOLOGY §6): the director commits the item
edits on `task/T-NNN` before running join; the page is then regenerated from the joined tree.

### (3) A failing join needs its classification
`cargo xtask join T-NNN [--acc-defects N]`. After the acceptance run on the joined branch:
- if `acc_fail_first_join` > 0 and `--acc-defects` is not given, or if `--acc-defects N` is given
  with N > `acc_fail_first_join` (0 included), join **refuses**: exit 1, exactly one stderr line
  beginning `join: refused: `, each failing test and each target that did not compile named on
  stdout, one per line, as in (1); `task/T-NNN` is put back on its pre-join head (merge undone,
  worktree clean) and no record is appended;
- otherwise the record's `acc_defects` is N when given and `null` when not (a join with no
  failure needs no classification).
The director reads the listed failures, decides how many are implementation defects, and runs
`join T-NNN --acc-defects N` again; the second run repeats the whole join.
The `passes on base` refusal keeps its precedence and behaviour (record `refused_base_passes`,
merge undone, exit as today); it is not subject to this rule (Q-3). The records `no_acceptance_tests`
and `conflict` are written before the acceptance run, as today.

### (4) `join --revert-check` removed
`cargo xtask join --revert-check …` and `--revert` no longer exist: the command line is a usage
error (exit 2), nothing is created, run or appended. The `revert_check` records already in
`metrics/tasks.jsonl` stay and are read by nothing, as today.

### (5) `metrics join-defects` and the `join_defects` record kind removed
`cargo xtask metrics join-defects …` is a usage error (exit 2) and appends nothing. `metrics
report` no longer reads `join_defects` records (none exists: `grep -c '"kind":"join_defects"'
metrics/tasks.jsonl` prints 0 on main `b53d370f4a5f22eeffd34969b49f34170e0edf57`); a join's
classification is its own `acc_defects`. The report still prints one `unclassified join: …` line
per failing join of an iteration-≥1 task without `acc_defects`, unchanged in form, and its exit
code no longer depends on them: it exits 0 when it prints the report (Q-4).

### (6) A spec check has its own role
`cargo xtask metrics add --task iter-N --role spec-checker …` appends a `run` record with
`role: "spec-checker"`, `task: "iter-N"`, `tier: null`, under the same rules as the other
overhead roles: `spec-checker` is accepted only on `iter-N`; on a `T-NNN` it is refused (exit 1,
one line `metrics: refused: …`, nothing appended), like `planner`. Nothing else about `add`
changes; records already booked as role `planner` with a note beginning `spec check:` stay as
they are (no record is edited). Name: Q-5.

### (7) `docs/development.md`
These passages state what the tools then do (wording is the implementer's; the reviewer checks
each sentence against the built binary, METHODOLOGY §7):
- "Scorecard", last paragraph: the sentence "The test author fills `tests` (METHODOLOGY §6)."
  is replaced by: the test author lists each item's tests in its log; the director ties them to
  the items in a commit on `task/T-NNN` before the join, and the join regenerates
  `docs/scorecard.md` on the task branch (Interface (2)).
- "Join": the heading loses "(and the optional `--revert-check`)" and keeps its first word
  `Join`; the step list gains the regeneration step of (2) and the refusal of (3); step 5 no
  longer says `null`, "classified later with `metrics join-defects`"; the target identity of (1)
  is stated; the paragraph on `--revert-check` is removed. The section keeps naming the
  reviewer's mutation spot-check for weak base evidence.
- "Metrics": the `join-defects` usage line and its bullet are removed; "`transition` and
  `join-defects` refuse `iter-N`" becomes "`transition` refuses `iter-N`"; the overhead roles
  list gains `spec-checker`; the `report` sentence says the `unclassified join` lines no longer
  set the exit code.

## Pinned
- (a) **Errors and precedence.** Join, in order: the worktree checks of today (missing or dirty
  worktree: error, exit 2) → base run (no acceptance target: record `no_acceptance_tests`, exit
  2, as today) → implementer tests → merge (conflict: record `conflict`, exit 2, as today) →
  joined acceptance and guard runs → base passes: record `refused_base_passes`, merge undone, as
  today → classification refusal of (3): exit 1, nothing recorded, merge undone → acceptance
  failures: record `acceptance_failed` with N, merge kept, exit 1 → guard failures: record
  `guard_failed`, exit 1 → regeneration of (2) (its exit-2 failure: nothing recorded, merge
  undone) → gate: record `joined` (exit 0) or `gate_failed` (exit 1). `--acc-defects` with a value
  that is not a non-negative integer stays a usage error (exit 2). `metrics`: unchanged order;
  the role check of (6) is the existing "role rules" step.
- (b) **Files.** Reads main's committed `xtask/generated.toml` (via the primary checkout's
  `main`, as the queue reads it); writes `docs/scorecard.md` and one commit on `task/T-NNN` only;
  appends `join` records to the primary checkout's `metrics/tasks.jsonl` with unchanged field
  names; `metrics add` appends `run` records. No new record kind. Test files:
  `xtask/tests/acceptance_t033.rs`, `xtask/tests/guard_t033.rs` (no other package has a target
  with either stem: `find . -path ./target -prune -o -name 'acceptance_t033*' -print` and the
  same for `guard_t033*` print nothing on main `b53d370f4a5f22eeffd34969b49f34170e0edf57`).
- (c) **Order of checks.** As (a). Regeneration happens only on the path that would run the gate.
- (d) **Paths and platform.** Target paths are repository-relative with `/` separators. Linux, the
  git of the bootstrap machine (as the queue).
- (e) **Threat model.** D-026: honest mistakes and corner-cutting, not a malicious agent. What
  this task closes: a record that counts fewer tests than ran (D-084); a failing join recorded
  without a classification (retro-1: 28 nulls). What it does not defend against: a director who
  passes a wrong N (the refusal lists the failures; the number stays a judgement); a test that
  writes files into the worktree during the scorecard command (caught by "any other path" in (2)).
  The regeneration runs code of the task branch (its tests), as join's test runs already do.
- (f) **Existing tests this task changes** (each a consequence of D-091 changes 9 and 10; no
  other assertion of a T-005 or T-013 test may change; a test that fails for another reason stops
  the run with a `QUESTION:`):
  1. `xtask/src/join.rs` unit tests: the revert-check parts of
     `join_with_failing_base_and_passing_implementation` and
     `compile_failure_on_base_is_recorded_as_weak_evidence`, and
     `revert_check_refuses_without_tests_branch_commits`, go with the command;
     `acceptance_failures_keep_the_merge_for_the_implementer` passes `--acc-defects` (the call
     without it is now the refusal). Implementer's unit tests; list the changes in the log.
  2. `xtask/src/metrics.rs` unit test `join_defects_rules` goes with the command.
  3. `xtask/tests/acceptance_t013_report.rs`: the test
     `join_defects_appends_one_record_and_classifies_the_join` and its helper `jd` are removed; in
     `report_computes_the_v11_metrics` the expected exit code of the report with an unclassified
     join changes from 1 to 0; the assertion on the `unclassified join:` line stays.
  4. `xtask/tests/acceptance_t013_docs.rs` `development_md_has_no_stale_v10_references`: the
     assertion that `docs/development.md` contains `join_defects` is removed. Its other
     assertions stay and bind the new text: a `## Join` heading exists, the Join section contains
     `mutation spot-check` and neither of the two removed phrases, the document still contains
     `Publishing a ledger input` and `hygiene`, and none of the four stale strings.
  5. Kept unchanged and passing: `acceptance_t013_docs.rs`
     `a_join_with_weak_base_evidence_points_to_the_mutation_spot_check` (its exact
     `join: base evidence …` line stays as it is), `acceptance_t005_r2.rs`
     `join_finds_acceptance_and_guard_tests_under_xtask_tests` (its T-002 join fails on a guard,
     with no acceptance failure, so it needs no classification), `acceptance_t013_metrics.rs`
     `overhead_runs_are_booked_to_iter_n`, and the fake `revert_check` line written by
     `acceptance_t005.rs`.
  These edits are the implementer's, on `task/T-033`, so that its gate can be green before the
  join (Q-2); the implementer adds no `acceptance_t*` file.
- (g) **Worked examples** (counts follow from (1); executed by hand against the rule, not with
  cargo). Toy repository: `crates/a/tests/acceptance_t901.rs` (3 tests),
  `apps/b/tests/acceptance_t901.rs` (2 tests), `xtask/tests/acceptance_t901_x.rs` (1 test), all
  failing on base and passing joined → `acc_total` 6, `acc_fail_base` 6, `base_fail_kind`
  `fails`, `acc_fail_first_join` 0. Today's stem identity would build both `acceptance_t901`
  binaries but look up one by name, so it would report `acc_total` 4 or 5, never 6 (the T-018
  shape: 6 of 12). Same repository, `apps/b/tests/acceptance_t901.rs` not compiling on base:
  `acc_fail_base` = 3 + 2 + 1 = 6, `base_fail_kind` `mixed`. Same repository, one test of
  `apps/b/tests/acceptance_t901.rs` failing joined: `acc_fail_first_join` 1; without
  `--acc-defects`: refused, stdout names `apps/b/tests/acceptance_t901.rs::<that test>`; with
  `--acc-defects 1`: record `acceptance_failed`, `acc_defects` 1; with `--acc-defects 2`: refused.
- (h) **Scorecard items.** None.
- (i) **Guarantee sentences in `docs/protocol/`.** None: this task changes no `docs/protocol/` page.

## Acceptance criteria
All through the `xtask` binary in throw-away git repositories the tests build (as the T-005 and
T-013 acceptance tests do, `GIT_CONFIG_GLOBAL` and `HOME` isolated, every spawned process with a
timeout). Counts are of the toy repository, never of this one. Oracle: exact values unless
stated. No test asserts the wording of a message beyond the prefixes named in the Interface.
AC-1 to AC-6 are acceptance tests in `xtask/tests/acceptance_t033.rs` (each fails on base by
assertion or command error); AC-7 is in `xtask/tests/guard_t033.rs` (passes on base).

- AC-1 (identity by path): the worked examples of Pinned (g), each a separate join: the record
  fields given there; and with two guard targets `crates/a/tests/guard_t901.rs` and
  `apps/b/tests/guard_t901.rs` passing on base and joined, `guard_total` counts the tests of
  both; if the implementation breaks the test in `apps/b/tests/guard_t901.rs` only, the result is
  `guard_failed` with `guard_fail_join` 1.
- AC-2 (scorecard page at join): a toy repository with a scorecard (one item whose `tests` the
  director's commit on `task/T-NNN` sets to a test that exists only on the tests branch) and
  main's `xtask/generated.toml` listing `docs/scorecard.md`: after a passing join, `task/T-NNN`'s
  history after the join merge holds exactly one commit that changes only `docs/scorecard.md` and
  whose body has the lines `Task: T-NNN` and `Role: director`; `xtask scorecard --check` run in
  `.worktrees/T-NNN` exits 0; the record's `joined_sha` is that commit; `main` and the primary
  checkout's status are unchanged apart from `metrics/tasks.jsonl`. A second join of the same task
  with the page already current makes no such commit.
- AC-3 (regeneration commit failure): the same repository with a `commit-msg` hook in the
  repository's configuration that rejects every commit whose subject begins `join:`: exit 2, one
  stderr line beginning `join: regenerating docs/scorecard.md failed: `, no record appended,
  `task/T-NNN` at its pre-join head and its worktree clean.
- AC-4 (failing join needs its classification): the third worked example of Pinned (g): without
  `--acc-defects` exit 1, one stderr line beginning `join: refused: `, the failing test's target
  path and name both on stdout, no record appended, `task/T-NNN` at its pre-join head, worktree
  clean; then with `--acc-defects 2` the same; then with `--acc-defects 1` exit 1, one record with
  `result` `acceptance_failed`, `acc_fail_first_join` 1, `acc_defects` 1, and the tests branch
  merged into `task/T-NNN`. A passing join with `--acc-defects 1` is refused the same way; with
  `--acc-defects 0` it records `acc_defects` 0; without the option it records `null`.
- AC-5 (removed commands): `xtask join --revert-check T-NNN` and `xtask metrics join-defects
  --task T-NNN --joined-sha <40-hex> --acc-defects 1` each exit 2, append nothing to
  `metrics/tasks.jsonl` and create no `.worktrees/T-NNN-revert-check`. `xtask metrics report` on
  a ledger with one failing iteration-1 join without `acc_defects` prints one line beginning
  `unclassified join: ` naming that task and exits 0.
- AC-6 (spec-check role): `xtask metrics add --task iter-2 --role spec-checker …` (other options
  valid) exits 0 and appends one record with `role` `spec-checker`, `task` `iter-2`, `tier` null;
  `--task T-001 --role spec-checker …` with a valid task file exits 1 with one stderr line
  beginning `metrics: refused: ` and appends nothing.
- AC-7 (kept behaviour; guard): in `xtask/tests/guard_t033.rs`, a single-package join that passes
  records `result` `joined` with `acc_defects` null and exits 0; a join with no acceptance target
  records `no_acceptance_tests`; and `metrics add --task iter-2 --role planner …` is still
  accepted. (This file must not repeat any assertion of the T-005 or T-013 files.)
- AC-8 (no regression; named command): `cargo xtask ci` is green in `.worktrees/T-033` and in the
  queue's main gate and own gate, with the T-005 and T-013 tests changed only as Pinned (f) lists
  (reviewer: `git diff main...task/T-033 -- xtask/tests/acceptance_t005* xtask/tests/acceptance_t013*`
  shows those edits and nothing else).

## Out of scope and implementer notes
- Queue precondition refusals written to `metrics/queue.jsonl`, and the queue's and `publish`'s
  handling of ledger files that are dirty in the primary checkout: T-043. CI on ledger-only pushes:
  T-041. `AGENTS.md` and `docs/CHARTER.md`: T-042.
- Regenerating `docs/scorecard.md` in the queue (withdrawn, D-036); changing how `publish`
  regenerates it.
- Re-labelling old `planner` records whose note begins `spec check:`; adding a spec-check figure
  to `metrics report`.
- How a test binary is matched to its source path is the implementer's choice (cargo's build
  messages carry the target's source path); `testbins.rs` is in `touches` only for that.
- Unit tests in `xtask/src` with the toy-repository helpers of `git.rs` `testrepo`. No timing or
  size bound is stated or asked for.
- Never set or export `CARGO_TARGET_DIR` in commands run by hand (D-019).
- The methodology's interim steps for these items (§6 Scorecard, §13) end when this task merges;
  the director edits `methodology/` only at a retrospective, so the marked passages simply become
  true.

## Questions and rulings
Planner's options for the director (R-160); the spec above is written for the recommendation.
- Q-1 (split). Options: (A) one task for all six v1.2 tool items; (B, recommended) this task holds
  the join items (1)–(4) and the two `metrics` items (5), (6), and T-043 holds the queue's
  refusal record and the dirty-ledger rule; (C) this task holds only the join items, T-043 the
  spec-check role and the queue. By file: the join items share `join.rs`, `main.rs`'s `join`
  arm, one record and one toy-repository fixture, and (3) and (5) are one change (the
  classification moves from `metrics join-defects` into `join`); (6) touches only `metrics.rs`,
  which (5) already changes, so it adds no file. The queue item touches `queue.rs`, `publish.rs`
  and 87 refusal-assertion call sites in the T-005 and T-013 suites (command in T-043 Pinned (f))
  and shares no source file with this task. (A) bundles independent work against D-091's rule;
  (C) makes T-043 a bundle of two unrelated items and makes both tasks change `metrics.rs`. Under
  (B) the two tasks share only `docs/development.md` (different sections), which still keeps them
  out of flight together (§5.2). Keeping (6) here is a bundling of a small independent item,
  justified by the shared file; if the director wants it apart, it can only be a fifth task.
- Q-2 (who edits the T-013 tests of Pinned (f)). Options: (A, recommended) the implementer, on
  `task/T-033`, exactly as listed, so its gate is green before the join; the reviewer checks the
  diff (AC-8). (B) The test author, on `task/T-033-tests` (T-013 ruling D-1); then the
  implementer's gate is red on exactly those tests until the join, as in T-013's run R-058
  (15 predicted failures reported), and `self_verified` needs a ruling. (A) touches open question
  O-4 (implementer edits acceptance files); the director rules.
- Q-3 (`refused_base_passes` with failures on the joined branch). The record can carry
  `acc_fail_first_join` > 0 and `acc_defects` null, which METHODOLOGY §13 reads as "required".
  Recommended: keep it out of the rule, because the join proved nothing and was undone; the
  director may instead rule that such a record is written with `acc_defects` null by definition
  and the report skips it.
- Q-4 (`metrics report` exit code). Options: (A, recommended) exit 0 whenever the report is
  printed; the lines stay for the two unclassified T-013 joins of iteration 1, which no command
  can classify once `join-defects` is gone. (B) Keep exit 1 while such lines exist: the report
  then exits 1 permanently on this ledger, so its exit code cannot be used as a check (retro-1
  "Tooling and environment" already notes an exit code that is 1 for ever).
- Q-5 (role name). `spec-checker` (recommended: an agent noun like the other roles) or
  `spec-check`. Either is a value of the record's `role` field only; CHARTER §4's role list is not
  changed, because a spec check is a planner run (METHODOLOGY §5.7).
### Rulings (director, D-098)
- Q-1: split as written (join changes here; queue changes in T-043).
- Q-2: the implementer edits the pins in the merged T-013 tests that v1.2 removes behaviour from, the pin only and exactly as the spec lists, logged under golden rule 4 (precedent D-061, D-069), so its gate is green before the join.
- Q-3: as recommended; `refused_base_passes` stays outside the classification rule.
- Q-4: `metrics report` exits 0 always. It is a report; the queue's `budget` check is what refuses.
- Q-5: the role is named `spec-checker`.
- Order: after T-042; not in flight with T-041 or T-043 (shared `docs/development.md`).

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
- 2026-11-13T21:40Z · R-160 · draft · did: rewrote the spec for methodology v1.2 (changes 1, 8,
  9 join half, 10; D-091, D-093); split the queue half into T-043 (Q-1); dropped the old draft's
  items not needed by v1.2 and its 850,000 budget. Tier high: every `xtask/` path is a gate path
  (`precheck.rs` `is_gate_path`). Read against main `b53d370f4a5f22eeffd34969b49f34170e0edf57`:
  `join.rs` looks a binary up by target name (`CargoOracle::acceptance`); `metrics report`'s exit
  depends only on unclassified joins; no `join_defects` record exists; T-013 tests pin
  `join-defects` and the report's exit 1 (Pinned (f)). The regeneration step uses main's
  `xtask/generated.toml` command and runs only before the gate, so a failing join commits
  nothing. Pinned (h), (i) n/a: no scorecard item, no `docs/protocol/` page. Unsure: whether
  re-running a full join after a classification refusal is acceptable in time (not measured); the
  implementer may keep the built targets, the observable rule stays. · gate attempts: 0 · not
  run: no cargo; the worked examples were derived from the rule, not executed · missing context:
  none · next: director rules Q-1 to Q-5; all `depends_on` merged, so no spec check is required.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
