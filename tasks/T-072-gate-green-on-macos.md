---
id: T-072
title: "Make every CI gate job show which tests ran and why one failed, on success too; make incidents/ a ledger path; settle the nightly run"
tier: high              # METHODOLOGY v1.5 §4: gate paths (.github/workflows/platforms.yml, xtask/ledger.toml, xtask/tests/); the approval row must contain `gate paths`. No dependency change, no product code.
state: queued
owner_role: planner
scorecard_items: []     # produces evidence; changes no behaviour of Kiln's binaries
depends_on: [T-079, T-073]   # both merged (bc8d097; 8bf4c3a): T-073 changed platforms.yml and acceptance_t070.rs (spec check after T-073, below)
touches:                # one role per path (D-153); acceptance_t070.rs left out under Q-3 A1 and Q-4 A (spec check S-5); restored if either is ruled otherwise
  - .github/workflows/platforms.yml          # GATE PATH: implementer: the evidence steps of the three gate jobs, the core-crates label, `paths-ignore` (incidents); under Q-5 A one more gate-windows step
  - xtask/ledger.toml                        # GATE PATH: implementer: one entry `incidents/`, mode `any`
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
A CI record cannot always say which tests ran and why one failed (line numbers at main `72522f2`, after T-073):
- on a green gate job no test output is uploaded (the "Full test output (gate failed)" steps, `platforms.yml:107-108`
  macOS, `:205-206` Windows, `:292-293` Linux, `if: failure() && steps.gate.outcome == 'failure'`), so T-079's round 2
  could not show by name that the 12 registered product tests were absent on macOS;
- on a red one the log prints a prefix cut at 200 lines (`head -200`, `:113`, `:211`, `:298`), and a failing test's
  panic message is in no job log (`tasks/ci-runs/T-070-negative.md`; again in `T-073-round-2.md`: the message of the
  Windows failure of `acceptance_t002::disk_save_replaces_via_rename_without_leftovers` is "not read");
- the core-crates step's label says `cargo test --locked -p $p` (`:97`, `:181`, `:282`) while the command has
  `--no-fail-fast` (`:98`, `:182`, `:283`; T-071 R2-4).
After this task each gate job (`gate-linux`, `gate-macos`, `gate-windows`), green or red, uploads the list of every
test name per test binary (`cargo test --workspace --locked --no-fail-fast -- --list`, ruling Q-2 A's variant, measured
in the spec check S-3), and a red one also the full output of `cargo test --workspace --locked --no-fail-fast`, whose
log prints every failing test's name with the first lines of its message, without a line cap. Also: `incidents/` is a
ledger path (D-223, T-043 Q-8 B); findings 1 and 3 of T-070's review round 2 are closed (finding 2 to T-077, Q-4); the
"Runners, wall times and disk" table of `docs/development.md` shows a green macOS run. Gate paths: `platforms.yml`,
`xtask/ledger.toml`, `xtask/tests/acceptance_t072.rs`, `xtask/tests/guard_t072.rs` (and `acceptance_t070.rs` only if
Q-3 or Q-4 is ruled otherwise).

Reading list: METHODOLOGY v1.5 §8.1 ("CI records", "CI tasks"); `tasks/T-070-*.md` Interface (1);
`docs/development.md` "CI"; `tasks/ci-runs/T-079-round-1.md`, `-round-2.md`, `T-070-negative.md`; `reviews/T-070-r2.md`.

## Interface
### (1) The test evidence of each gate job
Rewritten in the spec check after T-073 for ruling Q-2 A's variant (its condition measured met, S-3; Q-3 asks the
director to confirm; the text as drafted, a second full run on every result, is kept at the end of this subsection as
form A2).
- **Test list** (new step, in each of the three gate jobs, after the core-crates step and before the full test output,
  `if: always()`): `cargo test --workspace --locked --no-fail-fast -- --list > test-list.txt 2>&1`, its exit status
  recorded, not used; the log prints the file's `Running` / `Doc-tests` lines and its `error` lines, then `TEST LIST:
  exit status N`. The step reuses the binaries the gate's `test` step built (same profile and flags but
  `--no-fail-fast`), so it runs no test. On a green job, the record's per-test names are this list and its pass/fail
  facts are the gate's `test` totals (`0 failed`); the counts are not compared for equality (the gate's total counts one
  test twice, S-3).
- **Full test output (gate failed)**: its condition stays as now (`failure() && steps.gate.outcome == 'failure'`):
  `cargo test --workspace --locked --no-fail-fast > test-output.txt 2>&1`, exit status recorded, not used. The log then
  prints, for every test that failed or whose binary aborted, one block: the `---- <name> stdout ----` header (or the
  binary's `Running` line and the abort lines, as round 1 of T-079 showed for a stack overflow) and the first 20 lines
  after it; every `error: could not compile` line and every `error[` line (Windows is red at compile errors until T-092,
  `T-073-round-2.md`; added by this check, S-6); and the `test result:` line of every binary. No overall line cap.
- `test-list.txt` is uploaded in the job's artefact on every run, `test-output.txt` whenever it exists (as now); on
  `gate-windows` also `workspace-tests.txt` (T-073, unchanged) and, under Q-5 A, the file of the step of (6).
- The core-crates step's label is the command it runs: `==== cargo test --locked --no-fail-fast -p <package>`.
- Form A2 (if Q-3 is ruled A2): no list step; the full-test-output step runs `if: always()`; `acceptance_t070.rs` then
  joins `touches` (its `ac1_gate_jobs_run_the_gate_and_upload_the_reports_whatever_its_result` asserts at `:1180` that
  the step runs only when the gate failed: measured failing on prototype A2, S-4).
### (2) `incidents/` as a ledger path
`xtask/ledger.toml` gains `[[ledger]] path = "incidents/" mode = "any"`; the `paths-ignore` list of `platforms.yml`
gains `incidents` and `incidents/**` at that entry's place in file order (the rule of T-041 that T-070 and
`acceptance_t070.rs` use). `ci.yml` is not changed (T-041). `ac1_triggers_and_the_ledger_filter`
(`acceptance_t070.rs:1001`) derives the expected list from the tree's `xtask/ledger.toml` (`:935-954`), so it needs no
edit for this: VERIFIED, it passes unchanged on a prototype with the entry after `methodology/` (S-4). No file under
`incidents/` is added by this task (D-235, S-4).
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
- (2) The two pinned layouts (`ac1_window_probe_runs_after_a_failed_build_and_records_not_run`, `acceptance_t070.rs:1651`,
  and `ac1_window_probe_is_bounded_to_120_seconds`, `:1691`): **moved to T-077** under Q-4 A (T-077 changes the window
  jobs; this task leaves them unchanged, G-4). Under Q-4 B they are loosened here, keeping both fn names (pinned by
  `guard_t079.rs:1590-1591`).
- (3) The macOS disk cell shows the record's columns as printed, inode columns included.
### (5) `docs/development.md` "CI"
The evidence-step paragraph and the artefact list say (1), and also name T-073's Windows step and
`workspace-tests.txt`, which the section does not mention yet (`docs/development.md:625-643`); the sentence "so on
Windows, where the workspace does not build until T-073, it is the only test evidence" (`:633-634`) is stale since
T-073 and is replaced by one that names T-073's step. The table "Runners, wall times and disk" is replaced by the
figures of a recorded run with `gate-macos` green — **to be measured by the director**, from the record of this task's
candidate on `ci/candidate` (`tasks/ci-runs/T-072-round-N.md`), passed to the implementer before the join.
### (6) Under Q-5 A only: one general reported Windows step
In `gate-windows`, after T-073's step and before the full test output, `if: always()`, `set +e` … `exit 0`: for each
file `crates/kiln-workspace/tests/<t>.rs`, in name order, `cargo test --locked --no-fail-fast -p kiln-workspace --test
<t> -- --show-output`, appended to `windows-tests.txt` with a line `==== <t>: exit status N` (the core-crates step's
form), the file in the upload. A test target added later (T-074's `guard_t074.rs`, T-095's) is then reported with no
workflow edit. T-073's step and its test (`acceptance_t070.rs:1210`) are not changed (that test counts only steps whose
command contains T-073's exact command, `:1215-1227`, so a second step does not disturb it). Costs: S-7.

## Pinned
- (a) n/a (no Rust interface). A job's colour stays its gate step's exit status (for the window jobs as now).
- (b) Files: the workflow; `xtask/ledger.toml`; artefacts on GitHub only.
- (c) Order in a gate job: gate, report, core crates, test list, full test output (gate failed), disk, upload; in
  `gate-windows` T-073's step "Workspace lib and T-073 tests (reported, not required)" (`platforms.yml:191-203`) stays
  between core crates and the full test output, the test list after it, and under Q-5 A the step of (6) after that.
  The other window and gate steps as now.
- (d) Platforms: the three pinned labels. Per test file: `acceptance_t072.rs` reads files only (`std::fs`; Linux,
  macOS, Windows alike). `guard_t072.rs`'s G-1 and G-2 run step scripts with `bash` and a fake `kiln`: on Windows with
  Git Bash where found, else they **fail naming why** (§8.1); **no platform cfg** in either file — a cfg'd test would
  need an entry in `xtask/platform-tests.toml`, which is not in `touches` (and is T-074's, S-4).
- (e) Threat model: as T-070 (`contents: read`, pinned actions, no secrets).
- (f) Tests pinning what this changes: `acceptance_t070.rs` (`ac1_triggers_and_the_ledger_filter` — the filter
  derived from `xtask/ledger.toml`; `ac1_gate_jobs_run_the_gate_and_upload_the_reports_whatever_its_result`;
  `ac1_window_probe_is_bounded_to_120_seconds`; `ac1_window_probe_runs_after_a_failed_build_and_records_not_run`) —
  read the test names on main (`grep "^fn ac" xtask/tests/acceptance_t070.rs`, R-336); the implementer changes only
  what (1), (2), (4) require. **Re-read after T-073 (S-4):** the file has 14 test fns at `72522f2` (`grep -c "^fn
  ac\|^fn t073" xtask/tests/acceptance_t070.rs` = 14), the new one T-073's
  `t073_gate_windows_reports_the_workspace_lib_and_t073_tests` (`:1210`), which must keep passing unchanged. Under
  Q-3 A1 and Q-4 A **none** needs an edit: VERIFIED, main's `acceptance_t070` (and all of `cargo test -p xtask`: 287
  passed, 0 failed) passes on prototype A1 (S-4); the `incidents` filter is derived (Interface (2)) and no test checks
  the label's text. Under A2 `ac1_gate_jobs_run_the_gate_and_upload_the_reports_whatever_its_result` changes at
  `:1176-1182` only. `xtask/tests/guard_t079.rs` pins 13 of the 14 names (`:1581-1593`, in its base list of 1,121,
  `BASE_COUNT` `:516`): no test fn of `acceptance_t070.rs` is renamed or removed, whatever is ruled.
- (g) Cost, measured from records: the full test output step took 11 min 26 s on macOS in round 1 of T-070
  (`docs/development.md` table) and 10 min 32 s in round 1 of T-079 (05:50:44Z–06:01:16Z, `list_workflow_jobs`).
  Under A2 that is added on every green run. Under A1 a green run adds the list step instead: locally 1.3–1.4 s
  against 461 s for a second full run (S-3); its time on the runners is **to be measured by the director** from this
  task's round-1 record (the step's `started_at`/`completed_at`, `list_workflow_jobs`). The earlier Windows figure
  "about 6 s while nothing above `kiln-workspace` compiles" (T-079 round 1) predates T-073 and is not carried over;
  `T-073-round-2.md` gives job times only (`gate-windows` 18 min 10 s).
- (h) Evaluator: none.
- (i) Sentences for `docs/development.md` with evidence: (1) the workflow file and AC-1/AC-2; (3) the two run ids
  above; (5) to be measured by the director (record named above). The disk cell: copied from that record.
- (j) **Main's gate on the candidate tree** (D-235): the task changes no `xtask/src/` file and no fixture; main's
  `xtask` and the task's are the same tool. What differs is data the gate's tests read from the candidate tree:
  `xtask/ledger.toml` and `platforms.yml`, both read by `acceptance_t070.rs` from the tree under test, which is the
  candidate's own. The queue's policy reads (`config`, `ledger`) use main's `xtask/ledger.toml` until this merges, so
  `incidents/` becomes a ledger path for the director only after the merge. Shown before the join by the
  implementer: main's `cargo xtask ci` (built from main's sources, R-325's method) on main plus the branch's diff,
  logged. **Checked after T-073 (S-4):** on prototype A1 main's `layers`, `ui-commands`, `hygiene`, `schema`,
  `scorecard --check`, `no-skip` PASS and `cargo test -p xtask` 287/0. The rule change (`incidents/` a ledger path) is
  not used in the same tree: the task adds no file under `incidents/`, and the queue reads main's ledger; no cut.

## Acceptance criteria
Local gate tests by the independent test author (`xtask/tests/acceptance_t072.rs`, reading the workflow as
`acceptance_t070.rs` does; no YAML crate):
Steps are identified by the file they write (`test-list.txt`, `test-output.txt`, under Q-5 A `windows-tests.txt`),
not as "every step that runs `cargo test`": a later reported step in `gate-windows` (T-073's now; T-074's or T-095's
if Q-5 is ruled B) is not these criteria's matter (no brittle pin, D-249).
- AC-1 (A1, rewritten after T-073): in each of the three gate jobs, exactly one step writes `test-list.txt`; it runs
  `cargo test --workspace --locked --no-fail-fast -- --list` after the gate and before the upload; the set of its
  conditions equals exactly {`always()`} (fails on base: no such step); the upload's path list includes
  `test-list.txt`; must reject `if: success()` and a list step after the upload. The full-test-output step keeps
  "runs when the gate failed and only then" (that is `acceptance_t070.rs:1176-1182`, unchanged). Under A2: as
  drafted (the test-output step's conditions equal {`always()`}).
- AC-2: the set of commands in the steps writing `test-list.txt` or `test-output.txt` that limit output by lines
  equals ∅ (fails on base: `head -200` ×3, `platforms.yml:113`, `:211`, `:298`); must reject `tail -n 500`.
(Not a separate criterion: the upload list already names `test-output.txt` on base, `platforms.yml:132-136`, `:230-235`,
`:317-321`.)
- AC-4: each core-crates label equals the command it runs (fails on base: the label lacks `--no-fail-fast`,
  `:97`, `:181`, `:282`).
- AC-9 (only under Q-5 A): in `gate-windows` and no other gate job, one step runs, for each file of
  `crates/kiln-workspace/tests/*.rs`, `cargo test --locked --no-fail-fast -p kiln-workspace --test <name>`, appends to
  `windows-tests.txt` a line with that target's exit status, starts `set +e`, ends `exit 0`, runs `if: always()`, and
  the upload includes the file; checked by running the step's script with a fake `cargo` on PATH in a scratch tree
  with three test files, of which one fails (all three named with their status); must reject a fixed list of `--test`
  names.
- AC-5: the set of ledger paths in `xtask/ledger.toml` equals exactly main's plus `incidents/` (mode `any`); and the
  `paths-ignore` list equals the list derived from it (fails on base); must reject `incidents/` with `add-only`.
Guards (`guard_t072.rs`; they hold on base, §6): G-1, the probe step run with a fake `kiln` that never exits ends
within 120 s plus 15 s and records the kill (T-070 r2 finding 1); G-2, every recording step ends with exit status 0;
G-3, no `continue-on-error` anywhere and every `uses:` pinned by sha; G-4, the window jobs' steps unchanged.
Through CI (pending criterion, below; the reviewer claims only what the record shows):
- AC-7: on the candidate, each gate job's artefact holds `test-list.txt` (green or red; under A2 `test-output.txt`),
  a red job's artefact also `test-output.txt`, and a red job's log prints each failing test's block and every
  `could not compile` line.
- AC-8 (negative candidate, reviewer): one deliberately failing assertion on a scratch commit pushed by the director
  to `ci/candidate` gives, in the record, the failing test's name and its panic message (round 1 of T-070's negative
  run could not).

### The pending CI criterion (v1.5 §8.1), for the approval row
The record (`tasks/ci-runs/T-072-round-N.md`, from the artefacts) of the rebased head on `ci/candidate` shows:
the counted jobs — the Linux gate (`gate-linux` on `ci/candidate`), `window-linux`, `window-probe (macos-15)`,
`gate-macos` — green; the `gate-linux` and `gate-macos` artefacts each holding `test-list.txt` with a `Running` line
per test binary and its test names (under A2: `test-output.txt` with every `test … ok` line); `gate-windows` red as on
main since T-073 (failing steps clippy, scorecard, test, each naming only test targets of other crates than
`kiln-workspace`'s lib and T-073's, as in `T-073-round-2.md`), its artefact holding `test-list.txt` and `test-output.txt`
(the compile errors printed in the log) and `workspace-tests.txt` with T-073's 47 tests `ok`; `window-probe
(windows-2025)` as on main (tracking, exits 2 for want of OpenGL 2.0); the list step's time per gate job (Pinned (g));
and the negative candidate's record naming the failing test with its message. **Two CI rounds expected** (the
candidate and the negative one).

## Out of scope and implementer notes
- `ci.yml` (T-041); the record-extraction tool (T-089); the window jobs (T-077); release builds (T-078).
- No step may hide a failure: recording steps end with `exit 0`; the gate step decides.
- Each CI round is a `CI-REQUEST` and counts as a gate attempt.

## Collisions and order
- `docs/development.md`: also in the drafts T-043, T-089, T-090, T-091 → not in flight together (§5.2).
- `xtask/ledger.toml`: also edited by nobody else in flight. `.github/workflows/platforms.yml`: T-073 merged first
  (`8bf4c3a`; this spec re-checked after it); T-074 (draft, its Q-2 A: after this task), T-095, T-077, T-078 → after
  this task. Order with T-074: S-8.
- T-093 (macOS document sentences): no shared path → may be in flight together.
- T-089 is specified after this merges (it reads this task's artefact layout).

## Estimate
T-070 (the same workflow and test file): 809,362 + one unknown run (D-197); T-079 846,099; T-071 1,286,681. This task
is smaller than T-070 (no new job, no procedure) but has two CI rounds and a negative run: **about 700,000** of the
high ceiling 1,100,000. Arm: high → independent test author. **After T-073 (spec check):** unchanged at about
700,000 under Q-3 A1 and Q-4 A (one step fewer to test in `acceptance_t070.rs`, one list step more); about +40,000 under
Q-5 A (one step, AC-9). T-073 itself, same workflow and two CI rounds, booked 860,977 (D-252) with product code.

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

Prototypes and measurements were made in a scratch clone of main (`72522f2`), never in the primary checkout; the
script that builds both prototype trees is described in S-4 (`proto.py`: ledger entry after `methodology/`, the two
`paths-ignore` lines after `methodology/**`, the three labels, `head -200` replaced by an `awk` block printer, and
either a "Test list" step plus its upload line (A1) or `if: always()` on the full output (A2)).

- **S-1 Corrected in this file (stale since T-073), verdict: fixed.** Goal line numbers now cite all three jobs
  (`:107-108`/`:205-206`/`:292-293`, `:113`/`:211`/`:298`, `:97`/`:181`/`:282`, `:98`/`:182`/`:283`); Pinned (c) names
  T-073's Windows step "Workspace lib and T-073 tests (reported, not required)" (`:191-203`) in the order; Pinned (f)
  names T-073's test fn and the 14-fn count; Pinned (g) drops the pre-T-073 "about 6 s" Windows figure; the pending
  criterion lists the counted jobs as now (the Linux gate, `window-linux`, `window-probe (macos-15)`, `gate-macos`)
  and `gate-windows` red as since T-073 (clippy, scorecard, test; not "at `kiln-workspace`"), with
  `workspace-tests.txt` kept; Interface (5) adds T-073's step to the documented evidence and replaces the stale
  sentence `docs/development.md:633-634`; Collisions: T-073 merged; `depends_on` gains T-073 (merged). Outside this
  file (a ledger file, the director's, at the merge): `tasks/ci-runs/README.md` "What an artefact holds" says
  `test-output.txt` comes "only when the gate failed, until T-072 uploads it always"; under A1 it becomes
  `test-list.txt` always, `test-output.txt` on a red gate.
- **S-2 Unchanged and still true** (VERIFIED at `72522f2`): the trigger `schedule: - cron: '17 3 * * *'` at
  `platforms.yml:35-37`; the upload lists `:132-136`, `:230-235`, `:317-321` name `test-output.txt`; no
  `continue-on-error` (`acceptance_t070.rs` `ac1_name_permissions_no_concurrency_no_continue_on_error` passes); every
  `uses:` pinned by sha; `xtask/ledger.toml` has 9 `any` entries and 1 `add-only` (`scorecard/items/`), no `incidents/`.
- **S-3 Ruling Q-2's variant, measured (Linux, 4 CPU, scratch clone, test binaries already built by `cargo test
  --workspace --locked --no-run`, which from an empty `target/` took `Finished … in 1m 25s` and left 2.8 GB):**

  | command (in the clone's root; output to a file) | wall time | size | lines | test names |
  |---|---|---|---|---|
  | `cargo test --workspace --locked --no-fail-fast -- --list` (3 runs) | 1.41 s, 1.43 s, 1.26 s | 72,960 B | 1,537 | 1,146 in 134 groups (126 binaries + 8 doc-test groups), exit 0 |
  | `cargo test --workspace --locked --no-fail-fast` | 461.1 s | 92,219 B | 1,958 | 1,146 `test … …` lines; `test result:` sum 1,147 passed in 135 lines |
  | `cargo test --workspace --locked --no-fail-fast -- --show-output` | 462.7 s | 170,173 B | 3,857 | as above |

  Time measured with `date +%s.%N` before and after; sizes `wc -c -l`; names by a `python3 -I` reading of both files
  (per binary: `(Running|Doc-tests) <path>` then `^test (.*) \.\.\. ` resp. `^(.*): (test|bench)$`, ANSI stripped). The
  two name sets per binary are equal (1,146 each) but for one line of the full output, where
  `tests/confinement.rs`'s re-run of its own binary interleaved two lines into one; that child run is also why the
  `test result:` lines sum to 1,147 in 135 lines against 134 groups (`confinement.rs` prints a second `test result:
  ok. 1 passed; … 5 filtered out`). So: the list gives every test name per binary in about 1.4 s (the ruling's bound
  is one minute), 79 % of the plain full output's bytes (43 % of the `--show-output` one) and 0.3 % of its time; it runs no test, so it carries no verdict
  and no `MEASURED:` lines (`--show-output` adds 77,954 B, 3 `MEASURED`/`panicked` lines in all). **Recommendation:
  the list (A1)**, as the ruling's condition provides: a green run's names from the list, its verdicts from the gate's
  totals (`0 failed`; counts compared as names, not as numbers, because of the child run), a red run's from the full
  output, which runs then as now. Limits: the runners' times are not measured here (Pinned (g): to be measured by the
  director); on Windows the list holds compile errors and no names until T-092 (same `cargo test --workspace` build).
- **S-4 D-235, main's tool on the planned tree: no blocker, no cut.** On prototype A1 (`72522f2` plus the ledger
  entry, the `paths-ignore` lines, the labels, the list step and the uncapped printer), with the tool built from main's
  sources (this task changes no `xtask/src/` file and no fixture, so main's tool and the task's are one): `cargo xtask
  layers` "PASS layers: 11 crates, 14 internal edges, layer table holds"; `ui-commands` "PASS … self-test ok; 14 kiln-ui
  file(s) scanned, 5 allowlisted path(s)"; `hygiene` "PASS … session-paths: 690 tracked file(s) clean"; `schema` "PASS
  … 30 commands; audit ok; docs/commands.json current"; `scorecard --check` "PASS scorecard: 64 items: 47 verified, 0
  implemented, 0 failing, 17 planned."; `no-skip` "PASS … 201 files scanned, 0 allowlisted skip(s), 35 platform test(s)
  registered (19 pending)". `cargo test --locked --no-fail-fast -p xtask`: exit 0, 287 passed, 0 failed in 23
  binaries, `acceptance_t070` unchanged included. On prototype A2 main's `acceptance_t070` fails exactly one test,
  `ac1_gate_jobs_run_the_gate_and_upload_the_reports_whatever_its_result`, at `acceptance_t070.rs:1180` ("the
  test-output step must run when the gate failed and only then (if: Some(Str("always()")))"; 13 passed, 1 failed): A2
  must edit it, A1 need not. The rule this task changes is the ledger (`incidents/`); it would use the new rule only by
  committing under `incidents/`, which it does not (not in `touches`), and the queue's `ledger`/`main-history` checks
  read main's `xtask/ledger.toml` (`xtask/src/precheck.rs:4-9`), so the new entry acts only after the merge: no cut.
  The new tests: with a platform cfg they would be `cfg-disabled` findings of main's `no-skip` unless registered in
  `xtask/platform-tests.toml`, which is outside `touches`: hence Pinned (d), no platform cfg. Not run: `fmt`, `clippy`,
  `test` (the workspace test was run separately, S-3) and `cargo xtask ci` as a whole on the prototype; the builder
  still runs main's `cargo xtask ci` on the branch before the join (Pinned (j)).
- **S-5 D-249, main's tests that read this task's paths** (`grep -rln <path> xtask/tests crates/*/tests apps/*/tests`
  at `72522f2`, one line per hit):
  - `.github/workflows/platforms.yml`: only `xtask/tests/acceptance_t070.rs` → passes unchanged on A1 (S-4); on A2
    fails at `:1180` (the task edits it then). T-073's `t073_…` test counts only steps containing its exact command
    (`:1215-1227`), so new steps, A1's or (6)'s, do not disturb it.
  - `xtask/ledger.toml`: `acceptance_t070.rs` (`ledger_paths_ignore`, derived: passes); `acceptance_t005.rs:1145-1183`,
    `acceptance_t005_r2.rs:44`, `acceptance_t005_r3.rs:889`, `acceptance_t005_support.rs:220-227,294` copy the shipped
    file into toy repositories (no test names `incidents`: `grep -rn incidents xtask/tests` empty) → pass (287/0, S-4).
  - `xtask/tests/acceptance_t070.rs` (as a path): `guard_t079.rs:1581-1593` pins 13 of its test names in the 1,121-name
    base list, and `guard_t070.rs:4` names it in a comment → every name must stay (Pinned (f)); dropping or renaming
    one, e.g. when loosening the 120-s test, fails `every_linux_test_name_of_the_base_still_exists`.
  - `docs/development.md`: `acceptance_t013_docs.rs:61-85` (no stale v1.0 strings; the "Join" section) → unaffected by
    a "CI" edit unless it adds one of the four strings; `guard_t079.rs:1539` (a test name); `confinement.rs:486`,
    `acceptance_t036_workspace.rs:137`, `acceptance_t036_cli.rs:132`, `acceptance_t037_cli.rs:150`,
    `guard_t036_removal_cli.rs:158`, `guard_t037_cli.rs:147` (message text only) → unaffected.
  - `xtask/tests/acceptance_t072.rs`, `xtask/tests/guard_t072.rs`: no hit (new files); `guard_t079.rs`'s base list is a
    floor, so added names pass.
  - No hit pins a count this task changes (the case that blocked T-073, D-249): `acceptance_t079.rs` pins the registry,
    which this task does not touch; the gate's test total rises by the new tests only.
  Verdict: under A1 and Q-4 A, `acceptance_t070.rs` needs no edit and leaves `touches` (done above; restored if Q-3 A2
  or Q-4 B).
- **S-6 Ambiguity, verdict: resolved in Interface (1), director may strike it.** The drafted printer printed failing
  tests' blocks only; on Windows the full output holds compile errors and no test line (`T-073-round-2.md`:
  `test-output.txt` 172 lines, all build errors), so "why one failed" there is the `could not compile` / `error[` lines.
  Added to Interface (1) and AC-7.
- **S-7 A general Windows step (Interface (6), Q-5), measured cost.** `cargo check --locked --target
  x86_64-pc-windows-msvc -p kiln-workspace --lib --tests --keep-going` in the clone (4 s, deps cached): of the 14
  files in `crates/kiln-workspace/tests/`, 9 do not compile for Windows (`acceptance_t002_r2`, `acceptance_t008`,
  `acceptance_t019_workspace`, `acceptance_t036_workspace`, `confinement`, `guard_t008`,
  `guard_t036_removal_workspace`, `guard_t036_workspace`, `project`: E0433 `unix`, E0599 `is_fifo`/`uid`/`ino`/
  `from_mode`), 5 do (`acceptance_t002`, `acceptance_t015_tools`, `acceptance_t073`, `guard_t073`, `tools_contract`);
  a type check, not a Windows link or run (INFERRED for the runner). Costs: 14 `cargo test` invocations on
  `gate-windows` (time on the runner unmeasured; T-073's step built and ran its 3 targets in 4.92 s,
  `T-073-round-2.md`); `acceptance_t002`'s known Windows failure would then be reported with its message (useful:
  "not read" in that record); one AC (AC-9, test author) and about 40,000 tokens; no change to T-073's step or test.
  What it saves: T-074 Interface (5) and T-095's step edit (each one workflow step and one `acceptance_t070.rs` fn) and
  the shared paths with this task (S-8). The whole-workspace form (93 test files in 8 packages) is left to T-092,
  which makes the gate's own `test` step build on Windows.
- **S-8 Order with T-074 (specified now, beside this task per D-245).** T-074's draft touches `platforms.yml` (a
  `gate-windows` step) and `acceptance_t070.rs` (one fn), both shared with this task's draft, and
  `xtask/platform-tests.toml` (not shared). Shared paths mean not in flight together (§5.2). **Finding: this task
  merges first** (T-074's own Q-2 A agrees; this task is specified, T-074 a draft; this task re-orders every gate
  job's evidence steps and T-074 adds one). T-074's re-check then reads: the order and names of `gate-windows`'s steps
  and the upload list; whether its step's file is picked up by this task's AC-1/AC-2 (it is not: they select by file
  name); `acceptance_t070.rs` as this task leaves it (unchanged under A1/Q-4 A). If T-074 went first, this task would
  re-check Pinned (c) and the step set its tests read in `gate-windows`. Under Q-5 A T-074 drops `platforms.yml` and
  `acceptance_t070.rs` from its `touches`: no shared path is left (this task does not touch `xtask/platform-tests.toml`
  or `disk.rs`), so the two may be in flight together as D-245 planned; T-074's Windows evidence then needs this
  task merged before T-074's CI round (its `guard_t074.rs` reported by (6)), so T-074 still re-checks after this merge.
- **S-9 Not checked:** the branches of tasks in flight (none dispatched that shares a path: T-074 and T-095 are drafts,
  read on main); the `awk` printer against a real failing output (the implementer's; AC-8's negative run shows it);
  any time on the runners.

### Questions from the spec check after T-073 (planner)
- **Q-3. Green-run evidence (ruling Q-2's condition).** A1: the list step (measured 1.3–1.4 s locally, S-3), full
  output only on red as now. A2: full output on every run as drafted (about 10.5 min per green macOS run; edits
  `acceptance_t070.rs`). **Recommendation: A1** — the measured condition of ruling Q-2 holds by a factor above 40; the
  runner time is recorded in round 1, and if it exceeds a minute the director rules A2 then.
- **Q-4. T-070 review round 2 finding 2 (the two window-probe layout pins).** A: to T-077, which changes the window
  jobs (this task's G-4 keeps them unchanged). B: here, keeping both fn names (guard_t079). **Recommendation: A** —
  with A1 it takes `acceptance_t070.rs` out of this task.
- **Q-5. A general Windows step (Interface (6)).** A: add it here (cost S-7). B: no; T-074 and T-095 each add their
  own step and test fn after this task. **Recommendation: A** — the later tasks then need no workflow edit, and T-074
  can be in flight beside this task as D-245 planned.
- **Q-6. The compile-error lines of S-6.** A: keep them in Interface (1) and AC-7. B: strike. **Recommendation: A.**

### Rulings on the spec check after T-073 (director, D-254)
- **Q-3: A1.** Every gate job uploads the list of test names per binary on every run; the full output only on a
  red one (measured: 1.3 s and 72,960 bytes against 461 s and 92,219 bytes for the same 1,146 names).
- **Q-4: A.** The two window-probe tests are not loosened here; that goes to T-077. `xtask/tests/acceptance_t070.rs`
  stays out of `touches`.
- **Q-5: A.** The general reported Windows step is added here (Interface (6), AC-9), so that later Windows tasks
  (T-074, T-095, T-092) do not edit the workflow to have their tests reported. Estimate about 740,000.
- **Q-6: A.** Compile-error lines are printed in the log too.
- Order: T-072 runs beside T-074, which gives up `platforms.yml` and `acceptance_t070.rs`; T-074's CI round is read
  on its head rebased on a main that holds this task.
- D-235 applies: before the join, builder and reviewer run main's `xtask` (built from `git archive main` in a
  scratch directory) on the candidate tree.
- Added 11:20Z (from T-074's planner, sent to the test author): AC-9 as written does not check that the general
  Windows step passes `--show-output`; Interface (6) requires it (the `MEASURED:` lines of later tasks depend on it).
  The test author checks the flag in the step's command; the implementer's step carries it as Interface (6) says.

### Rulings on the test author's questions (director, D-255; sent to both builders)
- **t1.** G-4 stands as written by the test author (the window jobs write and upload only their own records). That
  the window jobs' steps are unchanged by this task is the reviewer's diff check, not a test (T-077 will change them).
- **t2.** Pinned (d)'s bash rule covers both test files: a test that runs a step script uses `bash` (on Windows Git
  Bash where found) and otherwise fails naming why; no platform cfg.
- **t3.** "Abort lines" are the lines with `has overflowed its stack` and `fatal runtime error`; "the first 20 lines"
  of a failing test's output block is exactly 20.
- **t4.** The upload lists are exact sets.

### Rulings on review round 1 (director, D-260; sent to both builders)
- **u1 (finding 1, defect).** The implementer corrects the failure printer in all three gate jobs: while a failing
  test's block is open, a line shaped like a binary heading (`Running …`, `Doc-tests …`) is printed as part of the
  block and counts towards its 20 lines; it is taken as a heading only when no block is open.
- **u2 (finding 2, test gap).** The test author adds fixtures to the printer tests: output with colour codes, and a
  failing block that contains a heading-shaped line (the block of round 1, `tasks/ci-runs/T-072-round-1.md`, is the
  model). The new test must fail on the task's present head `d87f2e8` for finding 1 (state the message) and must
  kill the reviewer's mutants M10 and M16.
- **u3 (finding 3, test gap).** The test author pins the conditions of the evidence steps as an exact set per gate
  job (`guard_t072.rs`): the full-output step's condition includes the gate step's outcome; mutant M12 is killed.
- **u4 (findings 4 to 6, documents).** The implementer corrects the two false sentences of `docs/development.md`
  ("the first run with `gate-macos` green" is not: run 44253737976 was; the sentence about the macOS probe's script
  is moved under the run that showed it, 43752760283, or dropped) and the nit (the Linux `df` header is given only
  if the record has it; "as printed" only where the cell is verbatim).

### Ruling u5 (director, D-261; sent to both builders): G-1's time bound
In run 44327488962 (`tasks/ci-runs/T-074-early-2.md`, a head that holds this task's tests) the guard
`guard_t072::g1_probe_with_a_kiln_that_never_exits_ends_within_135_s_and_records_the_kill` failed on `gate-macos` in
the gate step and in the core-crates step ("the probe step took 136.6 s … (bound 120 s + 15 s)") and passed in the
full test output of the same job: the 15 s of slack is too little on a loaded runner. A guard that fails by the
runner's load would make a counted job on main red at random. The upper bound becomes 120 s + 60 s; the lower
bound (the step does not end before its own limit) and the check that the kill is recorded stay. The test may be
renamed to carry the new bound (it is not on main yet). The test author states in the log what the slack covers.

## Log
### director
<!-- end director log -->
### planner
- 2026-11-15 · R-255 · draft · did: filed from ADR-0009 · next: director
- 2026-11-18 · R-336 · draft · did: re-specified in full under v1.5 (cut: workflow evidence here, macOS document sentences to T-093) · not run: CI · next: director
- 2026-11-18 · spec check: re-check after T-073 merged (v1.5 §5.7), against main `72522f2` · did: corrected what T-073 made stale (line numbers of all three jobs, T-073's Windows step in the order, the 14 test fns of `acceptance_t070.rs`, counted jobs and `gate-windows`'s red steps in the pending criterion, the stale `docs/development.md:633-634` sentence, `depends_on` + T-073); rewrote Interface (1), AC-1, AC-2, AC-7 for ruling Q-2's variant after measuring it (S-3: `-- --list` 1.26–1.43 s, 72,960 B, 1,146 names vs full run 461.1 s / 92,219 B, `--show-output` 462.7 s / 170,173 B; same name sets); D-235 on prototype A1 with main's tool: six steps PASS, `cargo test -p xtask` 287/0, no cut (S-4); D-249 grep of every touched path (S-5): `guard_t079.rs` pins 13 `acceptance_t070` names, A2 alone fails `:1180`; dropped `acceptance_t070.rs` from `touches` under Q-3 A1 / Q-4 A; added optional Interface (6)/AC-9 (general Windows step, Q-5, Windows type-check of 14 targets: 5 compile, S-7); order with T-074: this task first (S-8) · tier: high unchanged (gate paths `platforms.yml`, `xtask/ledger.toml`, `xtask/tests/`) · questions Q-3 to Q-6 · not run: `cargo xtask ci` whole, fmt/clippy on the prototype, anything on the runners, the `awk` printer on a failing output · scratch clone and its `target/` deleted · state line left as it was · next: director
<!-- end planner log -->
### implementer
- 2026-11-18 · R-351 · started. Rulings acknowledged and accepted: D-245 Q-1 A (nightly kept), Q-2 A with its
  measured variant; D-254 Q-3 A1 (list step on every run, full output only on red), Q-4 A (window-probe pins to
  T-077; `acceptance_t070.rs` not touched), Q-5 A (general reported `kiln-workspace` step on `gate-windows`,
  Interface (6)), Q-6 A (compile-error lines printed). Criteria in my words: (AC-1) each gate job has exactly one
  step, `if: always()`, after the gate and before the upload, that runs `cargo test --workspace --locked
  --no-fail-fast -- --list` into `test-list.txt`, and the upload names that file; the full-output step keeps
  its "gate failed" condition. (AC-2) neither the list step nor the full-output step cuts its output by lines
  (no `head -200`). (AC-4) the core-crates label echoes exactly the command run, `--no-fail-fast` included.
  (AC-9) one `gate-windows`-only step, `if: always()`, `set +e` … `exit 0`, loops over the files
  `crates/kiln-workspace/tests/*.rs` (no fixed list), runs `cargo test --locked --no-fail-fast -p kiln-workspace
  --test <t>` for each and appends a `==== <t>: exit status N` line to `windows-tests.txt`, which is uploaded.
  (AC-5) `xtask/ledger.toml` = main's entries plus `incidents/` mode `any`; `paths-ignore` derived from it.
  Guards G-1..G-4: the probe's watchdog, recording steps ending `exit 0`, no `continue-on-error`, pinned
  `uses:`, window jobs unchanged — none of them is to be broken by my edits. (AC-7/AC-8) through CI only.
<!-- end implementer log -->
