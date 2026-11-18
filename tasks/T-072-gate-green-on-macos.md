---
id: T-072
title: "Make every CI gate job show which tests ran and why one failed, on success too; make incidents/ a ledger path; settle the nightly run"
tier: high              # METHODOLOGY v1.5 §4: gate paths (.github/workflows/platforms.yml, xtask/ledger.toml, xtask/tests/); the approval row must contain `gate paths`. No dependency change, no product code.
state: specified
owner_role: planner
scorecard_items: []     # produces evidence; changes no behaviour of Kiln's binaries
depends_on: [T-079]     # merged (bc8d097): gate-macos is green, so this task is no longer about making it green
touches:                # one role per path (D-153)
  - .github/workflows/platforms.yml          # GATE PATH: implementer: the evidence steps of the three gate jobs, the core-crates label, `paths-ignore` (incidents)
  - xtask/ledger.toml                        # GATE PATH: implementer: one entry `incidents/`, mode `any`
  - xtask/tests/acceptance_t070.rs           # GATE PATH: implementer: only the expectations this task changes (incidents in the filter; the two pinned layouts loosened; the label), no assertion dropped
  - docs/development.md                      # implementer: the "CI" section only (the evidence-step paragraph, the artefact list, the "Runners, wall times and disk" table and its disk cell)
  - xtask/tests/acceptance_t072.rs           # GATE PATH: test author
  - xtask/tests/guard_t072.rs                # GATE PATH: test author (T-070's guarantees that must stay)
budget_tokens: 1100000  # high (v1.5); estimate about 700,000
iteration: 2
found_by: planner       # ADR-0009 (R-255); re-specified R-336 (v1.5 §5.7)
origin_task: T-070
---

# T-072 Complete the CI evidence (renamed in R-336; file name kept)

Re-specified in full in run R-336 against main `7b86bdf` (T-079 merged at `bc8d097`). The draft's aim, "make the gate
pass on macOS", is reached: `gate-macos` was green on T-079's candidate (`tasks/ci-runs/T-079-round-2.md`: every step
PASS, test "1117 passed, 0 failed, 0 ignored in 133 suites"). What the draft's notes collected (below, kept) is the
workflow's evidence; the macOS document sentences are split off to **T-093** (documents only, different paths).

## Goal
A CI record cannot always say which tests ran and why one failed:
- on a green gate job no test output is uploaded (`platforms.yml:107-108`, `if: failure() && steps.gate.outcome ==
  'failure'`), so T-079's round 2 could not show by name that the 12 registered product tests were absent on macOS;
- on a red one the log prints a prefix cut at 200 lines (`platforms.yml:113`, `head -200`), and a failing test's
  panic message is in no job log (`tasks/ci-runs/T-070-negative.md`);
- the core-crates step's label says `cargo test --locked -p $p` (`platforms.yml:97`) while the command has
  `--no-fail-fast` (`:98`; T-071 R2-4).
After this task each gate job (`gate-linux`, `gate-macos`, `gate-windows`), green or red, uploads the full output of
`cargo test --workspace --locked --no-fail-fast`, and its log prints every failing test's name with the first lines
of its message, without a line cap. Also: `incidents/` is a ledger path (D-223, T-043 Q-8 B); the three follow-ups of
T-070's review round 2 are closed; the "Runners, wall times and disk" table of `docs/development.md` shows a green
macOS run. Gate paths: `platforms.yml`, `xtask/ledger.toml`, `xtask/tests/acceptance_t070.rs`, `acceptance_t072.rs`,
`guard_t072.rs`.

Reading list: METHODOLOGY v1.5 §8.1 ("CI records", "CI tasks"); `tasks/T-070-*.md` Interface (1);
`docs/development.md` "CI"; `tasks/ci-runs/T-079-round-1.md`, `-round-2.md`, `T-070-negative.md`; `reviews/T-070-r2.md`.

## Interface
### (1) The test output of each gate job
- The step now named "Full test output" runs after the gate **whatever its result** (`if: always()`): `cargo test
  --workspace --locked --no-fail-fast > test-output.txt 2>&1`, its exit status recorded, not used (the gate step alone
  decides the colour, as now).
- The log then prints, for every test that failed or whose binary aborted, one block: the `---- <name> stdout ----`
  header (or the binary's `Running` line and the abort lines, as round 1 of T-079 showed for a stack overflow) and the
  first 20 lines after it; and the `test result:` line of every binary. No overall line cap.
- `test-output.txt` is uploaded in the job's artefact on every run (it holds every `test <name> ... <verdict>` line,
  so the record can say by name what ran).
- The core-crates step's label is the command it runs: `==== cargo test --locked --no-fail-fast -p <package>`.
### (2) `incidents/` as a ledger path
`xtask/ledger.toml` gains `[[ledger]] path = "incidents/" mode = "any"`; the `paths-ignore` list of `platforms.yml`
gains `incidents` and `incidents/**` at that entry's place in file order (the rule of T-041 that T-070 and
`acceptance_t070.rs` use). `ci.yml` is not changed (T-041).
### (3) The nightly run (D-240)
Runs 12 (`43858930525`) and 13 (`44078955834`) at `c45a17f` have `event: schedule` (VERIFIED, `actions_list
list_workflow_runs` with `event: schedule`, R-336); the trigger is `schedule: - cron: '17 3 * * *'`
(`platforms.yml:36-37`); they started at 2026-11-16T11:34:25Z and 2026-11-17T11:59:32Z, about 8 h 17 min and 8 h 42
min after 03:17Z, and took 29 min 47 s and 28 min 46 s (created → updated). T-070 specified a nightly run (its
Interface (1)). This task keeps it unless ruled otherwise (Q-1) and states in `docs/development.md` that scheduled runs
start late by GitHub's own queueing (measured above).
### (4) T-070 review round 2, findings 1–3
- (1) The probe's 120 s bound is tested as behaviour: the test runs the probe step's script (as the workflow file holds
  it) with a fake `kiln` that never exits and asserts the step ends within the bound plus a stated margin and records
  the kill; and a test checks that every recording step (report, core crates, test output, disk, packages) ends with
  exit status 0 by running each with a failing command inside (where it can run on Linux) or, for steps that need the
  runner, by its last line.
- (2) The two pinned layouts are loosened: the probe's reading of the build result (`steps.<id>.outcome`) and the
  literal `120` are no longer asserted as text where (1) asserts the behaviour.
- (3) The macOS disk cell shows the record's columns as printed, inode columns included.
### (5) `docs/development.md` "CI"
The evidence-step paragraph and the artefact list say (1); the table "Runners, wall times and disk" is replaced by
the figures of a recorded run with `gate-macos` green — **to be measured by the director**, from the record of this
task's candidate on `ci/candidate` (`tasks/ci-runs/T-072-round-N.md`), passed to the implementer before the join.

## Pinned
- (a) n/a (no Rust interface). A job's colour stays its gate step's exit status (for the window jobs as now).
- (b) Files: the workflow; `xtask/ledger.toml`; artefacts on GitHub only.
- (c) Order in a gate job: gate, report, core crates, full test output, disk, upload (as now, the test output now always).
- (d) Platforms: the three pinned labels.
- (e) Threat model: as T-070 (`contents: read`, pinned actions, no secrets).
- (f) Tests pinning what this changes: `acceptance_t070.rs` (`ac1_triggers_and_the_ledger_filter` — the filter
  derived from `xtask/ledger.toml`; `ac1_gate_jobs_run_the_gate_and_upload_the_reports_whatever_its_result`;
  `ac1_window_probe_is_bounded_to_120_seconds`; `ac1_window_probe_runs_after_a_failed_build_and_records_not_run`) —
  read the test names on main (`grep "^fn ac" xtask/tests/acceptance_t070.rs`, R-336); the implementer changes only
  what (1), (2), (4) require.
- (g) Cost, measured from records: the full test output step took 11 min 26 s on macOS in round 1 of T-070
  (`docs/development.md` table) and 10 min 32 s in round 1 of T-079 (05:50:44Z–06:01:16Z, `list_workflow_jobs`);
  on a green run it is added once per gate job — on Windows about 6 s while nothing above `kiln-workspace` compiles
  (05:50:38Z–05:50:44Z). Option B of Q-2 avoids the second run.
- (h) Evaluator: none.
- (i) Sentences for `docs/development.md` with evidence: (1) the workflow file and AC-1/AC-2; (3) the two run ids
  above; (5) to be measured by the director (record named above). The disk cell: copied from that record.
- (j) **Main's gate on the candidate tree** (D-235): the task changes no `xtask/src/` file and no fixture; main's
  `xtask` and the task's are the same tool. What differs is data the gate's tests read from the candidate tree:
  `xtask/ledger.toml` and `platforms.yml`, both read by `acceptance_t070.rs` from the tree under test, which is the
  candidate's own. The queue's policy reads (`config`, `ledger`) use main's `xtask/ledger.toml` until this merges, so
  `incidents/` becomes a ledger path for the director only after the merge. Shown before the join by the
  implementer: main's `cargo xtask ci` (built from main's sources, R-325's method) on main plus the branch's diff,
  logged.

## Acceptance criteria
Local gate tests by the independent test author (`xtask/tests/acceptance_t072.rs`, reading the workflow as
`acceptance_t070.rs` does; no YAML crate):
- AC-1: in each of the three gate jobs, the set of conditions of the test-output step equals exactly {`always()`}
  (fails on base: `failure() && steps.gate.outcome == 'failure'`); must reject `if: success()`.
- AC-2: the set of commands in the test-output steps that limit output by lines equals ∅ (fails on base: `head -200`
  ×3); must reject `tail -n 500`.
(Not a separate criterion: the upload list already names `test-output.txt` on base, `platforms.yml:132-136`; AC-1 makes
the file exist on every run.)
- AC-4: each core-crates label equals the command it runs (fails on base: the label lacks `--no-fail-fast`).
- AC-5: the set of ledger paths in `xtask/ledger.toml` equals exactly main's plus `incidents/` (mode `any`); and the
  `paths-ignore` list equals the list derived from it (fails on base); must reject `incidents/` with `add-only`.
Guards (`guard_t072.rs`; they hold on base, §6): G-1, the probe step run with a fake `kiln` that never exits ends
within 120 s plus 15 s and records the kill (T-070 r2 finding 1); G-2, every recording step ends with exit status 0;
G-3, no `continue-on-error` anywhere and every `uses:` pinned by sha; G-4, the window jobs' steps unchanged.
Through CI (pending criterion, below; the reviewer claims only what the record shows):
- AC-7: on the candidate, each gate job's artefact holds `test-output.txt` (green or red), and a red job's log prints
  each failing test's block.
- AC-8 (negative candidate, reviewer): one deliberately failing assertion on a scratch commit pushed by the director
  to `ci/candidate` gives, in the record, the failing test's name and its panic message (round 1 of T-070's negative
  run could not).

### The pending CI criterion (v1.5 §8.1), for the approval row
The record (`tasks/ci-runs/T-072-round-N.md`, from the artefacts) of the rebased head on `ci/candidate` shows:
`gate-linux`, `gate-macos`, `window-linux`, `window-probe (macos-15)` green; the `gate-linux` and `gate-macos`
artefacts each holding `test-output.txt` with every `test … ok` line; `gate-windows` red at `kiln-workspace` as now,
its artefact holding `test-output.txt` too; and the negative candidate's record naming the failing test with its
message. **Two CI rounds expected** (the candidate and the negative one).

## Out of scope and implementer notes
- `ci.yml` (T-041); the record-extraction tool (T-089); the window jobs (T-077); release builds (T-078).
- No step may hide a failure: recording steps end with `exit 0`; the gate step decides.
- Each CI round is a `CI-REQUEST` and counts as a gate attempt.

## Collisions and order
- `docs/development.md`: also in the drafts T-043, T-089, T-090, T-091 → not in flight together (§5.2).
- `xtask/ledger.toml`: also edited by nobody else in flight. `.github/workflows/platforms.yml`: T-073 under its Q-1 A,
  T-077, T-078 → after this task.
- T-093 (macOS document sentences): no shared path → may be in flight together.
- T-089 is specified after this merges (it reads this task's artefact layout).

## Estimate
T-070 (the same workflow and test file): 809,362 + one unknown run (D-197); T-079 846,099; T-071 1,286,681. This task
is smaller than T-070 (no new job, no procedure) but has two CI rounds and a negative run: **about 700,000** of the
high ceiling 1,100,000. Arm: high → independent test author.

## Questions and rulings
- **Q-1 (planner, R-336). The nightly run.** A: keep it (T-070's spec; cost measured: about 30 min of the six jobs per
  night, free on this public repository; it starts about 8.5 h late). B: drop `schedule` (every main push that
  changes code already runs the workflow). **Recommendation: A** — it catches runner-image changes on days without a
  code push; the late start does no harm.
- **Q-2 (planner, R-336). The test output on a green run.** A: run `cargo test --workspace --no-fail-fast` again after
  the gate whatever its result (measured cost: about 10.5 min on macOS per green run; Windows seconds until T-092). B:
  make `cargo xtask ci` keep the output of its own `test` step in a file (an `xtask/src/gate.rs` change, no second
  run). **Recommendation: A** for this task — B changes the gate tool, which D-235's rule makes the riskier kind of
  change, and is T-089's natural place; B is unmeasured (no probe of the saved output's completeness), so it is not
  recommended now.

## Notes from T-070's CI rounds (director, 2026-11-15)
- Evidence to start from: `tasks/ci-runs/T-070-round-1.md` and `T-070-round-2.md` (`gate-macos`: the scorecard's 7
  problems; `xtask` 116 passed, 3 failed, names not printed; the first 200 lines of the full output with 11 failing
  tests named).
- **What the jobs do not show**: a failing test's panic message is in no job log (seen on the negative candidate,
  `tasks/ci-runs/T-070-negative.md`), the three failing `xtask` tests are not named, and the full output is cut at
  200 lines. This task's spec decides whether the report steps print failing tests with their messages (a change
  to `.github/workflows/platforms.yml`, a gate path) or the artefacts are read instead.
- **Follow-ups from T-070's review round 2** (`reviews/T-070-r2.md`, new findings 1 to 3; the first task that edits
  `.github/workflows/platforms.yml` or `xtask/tests/acceptance_t070.rs` takes them, this one or T-077): (1) the
  probe's time bound is tested as the text `120`, not as a bound (`sleep 2` in the watchdog or a dropped `kill -9`
  pass the gate), and no gate test covers "every recording step ends with exit 0"; (2) two assertions pin a layout
  no spec text pins (the probe reading the build's result through `steps.<id>.outcome`, and the literal `120`):
  loosen them when the workflow next changes; (3) `docs/development.md`: the macOS disk cell leaves out the
  record's inode columns although the sentence says "as printed".
- **From the re-specification of T-043 (D-223, ruling Q-8 B):** `incidents/` becomes a ledger path in this task
  (`xtask/ledger.toml`, the `paths-ignore` list of `platforms.yml` and the expectation in
  `xtask/tests/acceptance_t070.rs`, which checks the two against each other).
- **From v1.5 change 1 (D-219):** the evidence steps run every test binary (`--no-fail-fast`) and print failing
  tests with their messages; the full output is not cut at 200 lines.
- **From T-071's review round 2 (D-232, finding R2-4):** the `echo "==== cargo test --locked -p $p"` label of
  the core-crates step no longer matches the command, which has `--no-fail-fast` since T-071.
- **From T-079's CI round 2 (2026-11-18):** when a gate job passes, no full test output is uploaded, so a record
  cannot show by name which tests ran (the absence of registered tests on a green macOS run could not be read).
  The evidence steps upload the test list or the full output on success too.
- **From D-240:** two runs of `platforms.yml` on main that no push started (runs 12 and 13 at `c45a17f`,
  2026-11-16 and 2026-11-17 around midday UTC): find their trigger and say whether it is wanted.

### Rulings on spec R-336 (director, D-245)
- **Q-1: A.** The nightly run stays. The director reads the newest nightly run on main at the start of each
  session and at each handoff; a red counted job there is an incident like any other run on main.
- **Q-2: A**, with one thing for the implementer to measure first: whether listing the tests of the built
  binaries (`cargo test --workspace --locked --no-fail-fast -- --list` after the gate, no second run) gives every
  test name per binary in under a minute; if it does, the green-run evidence is that list beside the gate's
  totals, and the second run is not added. The measurement and the choice go into the implementer's report
  before the join.
- Dispatched after T-073 merges (both edit `platforms.yml` and `acceptance_t070.rs`); the planner re-checks this
  spec then (v1.5 §5.7).

### Spec check (after T-073)
Planner run, v1.5 §5.7 re-check after T-073 merged (`8bf4c3a`, base `7c44427`), against main `72522f2` (drift from the
asked `90bb7f4`: `git diff --stat 90bb7f4 72522f2` = `STATE.md`, `handoffs/2026-11-18.md` only, ledger). What T-073
changed in this spec's paths (`git diff 7c44427 8bf4c3a -- .github/workflows/platforms.yml xtask/tests/acceptance_t070.rs`):
one new `gate-windows` step "Workspace lib and T-073 tests (reported, not required)" (`platforms.yml:191-203`, `if:
always()`, writes `workspace-tests.txt`), that file added to the Windows upload (`:234`), and one new test fn
`t073_gate_windows_reports_the_workspace_lib_and_t073_tests` (`acceptance_t070.rs:1205-1277`) pinning that step's exact
command, its place between gate and upload, `set +e` … `exit 0`, and its file in the upload, in `gate-windows` and in no
other gate job. Every line number below is VERIFIED with `cat -n` / `grep -n` at `72522f2`.

(work in progress: S-1 … follow)

## Log
### director
<!-- end director log -->
### planner
- 2026-11-15 · R-255 · draft · did: filed from ADR-0009 · next: director
- 2026-11-18 · R-336 · draft · did: re-specified in full under v1.5 (cut: workflow evidence here, macOS document sentences to T-093) · not run: CI · next: director
<!-- end planner log -->
