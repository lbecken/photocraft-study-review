---
id: T-070
title: "Run the gate in CI on macOS and Windows as tracking jobs, check files out byte for byte, keep the reports, probe the window on each platform, and run the Linux window procedure under Xvfb on every code push"
tier: high              # METHODOLOGY v1.4 §4: gate paths (.github/workflows/platforms.yml, .github/scripts/, xtask/tests/); the approval row must contain `gate paths`. No dependency change.
state: ready
owner_role: planner
scorecard_items: []     # produces evidence; changes no behaviour of Kiln's binaries
depends_on: [T-069]     # merged (30c66909, D-179): the existing CI job is green, so a run can be read
touches:                # one role per path (D-153)
  - .github/workflows/platforms.yml          # GATE PATH: implementer (new; ci.yml is NOT edited: T-041 owns it)
  - .gitattributes                           # implementer (new; one line, Pinned (b))
  - docs/development.md                      # implementer: the "CI" section only (lines 535–540 on main 15555cc), not the T-069 paragraph after it
  - .github/scripts/window-procedure.sh      # GATE PATH: test author (new; the T-065 procedure as a file, Interface (3))
  - xtask/tests/acceptance_t070.rs           # GATE PATH: test author
budget_tokens: 1100000  # high (v1.4)
iteration: 2
found_by: planner       # ADR-0009 (R-255)
origin_task: null
---

# T-070 Run the gate in CI on macOS and Windows as tracking jobs

## Goal
Today CI runs the gate on Linux only (`.github/workflows/ci.yml`, one job on `ubuntu-24.04`), and the
window procedure of T-065 exists only as a block in the test author's log entry of
`tasks/T-065-window.md` (run R-251, lines 672–810 between the fences at 671 and 811, on main `15555cc`); no gate test opens a window
(D-174). Nothing is known about Kiln on macOS or Windows beyond compiling here with the C code stubbed
(ADR-0009 probe a3–a5). After this task:
- every push to `main` that changes a path outside the ledger, every push to a branch `ci/<anything>`,
  a nightly run and a manual dispatch run the whole gate on macOS and on Windows, and keep its report
  and, when it fails, the full test output;
- the same events run the T-065 window procedure in a real window under Xvfb on Linux (ruling Q-3 A,
  D-175); the procedure is a file in the repository that only a high-tier task can change;
- a push to `ci/<anything>` also runs the Linux gate, so a candidate head (§8.1) is checked on all three;
- a clone with Git's Windows defaults gets every file byte for byte as committed.
The macOS and Windows jobs are **tracking** (§8.1, ruling Q-1 A): red results are expected at merge
(below) and do not make main red; each becomes **counted** by a decision row naming its first green
run on main.

Gate paths changed: `.github/workflows/platforms.yml`, `.github/scripts/window-procedure.sh`,
`xtask/tests/acceptance_t070.rs`. The approval row needs `gate paths`; not `dependency versions`.

Reading list: METHODOLOGY v1.4 §8, §8.1; ADR-0009 (Decision 3, 5, 6; rulings at its end);
`.github/workflows/ci.yml`; `docs/development.md` "The gate", "Policy files", "CI"; `xtask/ledger.toml`;
`tasks/T-041-no-ci-run-for-ledger-only-pushes.md` "Interface (1)" (the ledger pattern rule, reused
here); `tasks/T-065-window.md`, test author log R-251 (the procedure) and `docs/protocol/ui.md` "The
window".

## Interface

### (1) `.github/workflows/platforms.yml`
- `name: platforms`; `permissions: contents: read`; no `concurrency` key (as `ci.yml`: every state is
  checked to the end); no `pull_request` or `pull_request_target` trigger (no code from forks runs).
- Triggers:
  - `push` with `branches: [main, 'ci/**']` and `paths-ignore` built from `xtask/ledger.toml` exactly as
    T-041's Interface (1) builds it: for each entry with `mode = "any"`, in file order, `e` and `e/**`
    (`e` = the path without a trailing `/`); `add-only` entries contribute nothing. On main 15555cc that
    is 18 patterns, `tasks` … `methodology/**`.
  - `schedule`: once a day at a fixed minute (the implementer picks it and states it in
    `docs/development.md`); runs on the default branch.
  - `workflow_dispatch` (no inputs).
- Defaults: `defaults.run.shell: bash` (Git Bash on Windows), `CARGO_TERM_COLOR: always`,
  `CARGO_INCREMENTAL: "0"`, as `ci.yml`.
- Every `uses:` is pinned by a full 40-hex commit sha with the release named in a comment
  (`actions/checkout` and `Swatinem/rust-cache` with the shas `ci.yml` uses; `actions/upload-artifact`
  at a sha the implementer reads from its release page and the reviewer re-reads).
- Checkout: `fetch-depth: 1`, `persist-credentials: false` (as `ci.yml`).

| Job id | `runs-on` | Runs on | Steps (in order) |
|---|---|---|---|
| `gate-macos` | `macos-15` | every trigger | checkout; `rustup toolchain install` and the three version lines (as `ci.yml`); cache; **`cargo xtask ci --keep-going --report gate-report.json`**; then, each with `if: always()`: the core-crates step; on failure of the gate only (`if: failure()`), `cargo test --workspace --locked --no-fail-fast` with stdout and stderr to `test-output.txt` (its own exit status ignored, so the gate step alone decides the colour); a disk line (`df -h` of the workspace's volume) and `du -sh target`; upload of `gate-report.json`, `core-crates.txt`, `test-output.txt` (whichever exist) as artefact `gate-macos-<run_id>`, retention 30 days |
| `gate-windows` | `windows-2025` | every trigger | the same, artefact `gate-windows-<run_id>` |
| `gate-linux` | `ubuntu-24.04` | only `ci/**` pushes, `schedule`, `workflow_dispatch` (`if:` on `github.ref != 'refs/heads/main' \|\| github.event_name != 'push'`) | the same as `gate-macos`; on pushes to main `ci.yml` stays the Linux gate (no duplicate run) |
| `window-linux` | `ubuntu-24.04` | every trigger | checkout; toolchain; cache; `cargo build -p kiln --locked`; `sh .github/scripts/window-procedure.sh target/debug/kiln "$RUNNER_TEMP/window"` with its output to `window-procedure.txt`; a line listing which of `xvfb-run`, `Xvfb`, `libxkbcommon-x11-0`, `libxcb-xkb1`, a Mesa GL driver are installed (`dpkg-query -W`, for the record); upload of `window-procedure.txt` as `window-linux-<run_id>`. Packages may be installed **on the runner** (`sudo apt-get install -y …`) only if round 1 shows one missing, and each is named in `docs/development.md` |
| `window-probe` | matrix `macos-15`, `windows-2025` | every trigger | checkout; toolchain; cache; `cargo build -p kiln --locked`; if the build succeeded, one run of `kiln --root <fresh dir with a.rs> --script <three-line script: snapshot, doc.list, snapshot>` bounded to 120 s by a means that exists on that runner (GNU `timeout` is not assumed on macOS); records exit status, stdout and stderr into `window-probe.txt`; upload as `window-probe-<os>-<run_id>` |

- The **core-crates step**: `cargo test --locked -p kiln-text -p kiln-syntax -p kiln-search -p xtask`
  with its output to `core-crates.txt`, `if: always()`. It is reported, not required: its exit status
  does not change the job's colour (the gate step does). Reason: on Windows the workspace does not
  build until T-073 (6 errors in `crates/kiln-workspace/src/disk.rs`, probe a3), and these four
  packages do not depend on `kiln-workspace` (`xtask/layers.toml`; `cargo tree`), so this step is the
  only Windows evidence before T-073.
- Nothing in the workflow hides a failure: no `continue-on-error` anywhere; the gate step's exit
  status is the job's colour; `window-linux` is red when the procedure's exit status is non-zero.
- "Tracking" and "counted" are not in the workflow file: they are the record's (§8.1). The director
  records the first green run on main of `gate-macos`, `gate-windows`, `window-linux` and each
  `window-probe` leg in a decision row; from that row on that job counts for main's colour.

### (2) `.gitattributes`
Exactly one line: `* -text`. Effect (VERIFIED, R-261, Pinned (g)): no end-of-line conversion on checkout
or commit for any path, whatever `core.autocrlf` says, so every checkout has the committed bytes and the
CRLF corpus file `crates/kiln-syntax/tests/corpus/rust/crlf_lines.rs.txt` keeps its CRs.

### (3) `.github/scripts/window-procedure.sh`
The R-251 procedure of `tasks/T-065-window.md` (the block between the ```` ```sh ```` fence after
"Window procedure (replaces R-248's in full)" and its closing fence), as a file, with two changes only:
- it **exits 0 only if** it printed `WINDOW CASES PASS` and `PLAIN PASS` and either `AC-3 PASS` or
  `AC-3 PRECONDITION FAILED` (the runner already has a library installed; then AC-3 is reported as not
  checked), and exits 1 otherwise (today it always exits 0; the verdict is only printed);
- its usage line names the file instead of `ac2.sh`.
Every case, fixture, comparison and verdict line stays as R-251 wrote it. From this task on this file
is the window procedure of record: the reviewer and the evaluator run it from main (D-176), and only a
high-tier task's test author changes it (`.github/` is a gate path).

### (4) `docs/development.md` "CI"
States: the two workflows and their jobs; triggers and the ledger filter; which job is the Linux gate on
main (`ci.yml`) and that `gate-linux` runs only for candidates, nightly and dispatch; the artefacts and
their retention; the core-crates step and that it is reported, not required; "tracking" and "counted"
as §8.1 defines them and that the record (decision rows) says which jobs count; the candidate branch
`ci/T-NNN` (§8.1); the window procedure's file and who may change it; the packages installed on the
runner, if any; the measured wall time of each job (Pinned (i)).

## Pinned
- (a) Errors: n/a (no Rust interface). A job's colour is its gate step's (or the procedure's) exit
  status, nothing else.
- (b) Files: the workflow, `.gitattributes` (one line `* -text`), the procedure; artefacts only on
  GitHub; no job writes to the repository.
- (c) Order: as the table; the reports are uploaded after the gate whatever its result.
- (d) Platform: labels pinned (`macos-15` arm64 3 CPU 7 GB, `windows-2025` x64 4 CPU 16 GB,
  `ubuntu-24.04` x64 4 CPU 16 GB, 14 GB disk each; docs.github.com "GitHub-hosted runners", read by
  summarising fetch in R-255, KNOWN not verified); toolchain 1.97.0 from `rust-toolchain.toml` (the
  images carry 1.99.1; `ci.yml`'s job log of run 352 shows rustup installing 1.97.0 over it).
- (e) Threat model: no secret is read; `contents: read`; no fork code; packages only from the runner's
  own mirror; every action pinned by sha.
- (f) Tests pinning what this changes: none found. `grep -rln "\.github\|gitattributes" xtask/tests
  xtask/src` on 15555cc lists `precheck.rs`, `queue.rs`, `policy.rs`, `acceptance_t005.rs`,
  `acceptance_t005_r3.rs`: all about `.github/` as a gate path or a ledger exclusion; none reads a
  workflow file, and this task changes neither rule.
- (g) Worked example (VERIFIED, R-261, scratch, nothing built): a clone of main with
  `core.autocrlf=true` has **587 of 588** tracked files differing from their blobs; the same after a
  commit adding `.gitattributes` with `* -text` and a fresh clone with `core.autocrlf=true`: **0 of 589**,
  and `crlf_lines.rs.txt` keeps 118 CR bytes (`cmp` of each `git ls-files` path against `git cat-file -p
  HEAD:<path>`). Without the attributes the gate fails `schema` and
  `kiln_cli::acceptance_t020_cli::syntax_commands_are_read_only` (ADR-0009 probe c2).
- (h) Evaluator: none (no item). The director reads the runs (§8.1).
- (i) Sentences for `docs/development.md`, each with its evidence: the jobs and triggers (the workflow
  file; AC-1); "files are checked out as committed on every platform" (AC-2; Pinned (g));
  "the window procedure runs in a real window under Xvfb on every code push" (AC-4, the recorded run);
  **wall time per job — to be measured by the director** from the recorded runs (the implementer cannot
  run hosted runners); figures of disk use — to be measured by the director from the disk line.

### What the director must push and read (only the director pushes)
- **Round n** (n = 1, 2, …): when the implementer's log entry says `CI-REQUEST: <40-hex sha>` (a commit on
  `task/T-070`), the director runs `git push origin +<sha>:refs/heads/ci/T-070` (a forced update of that
  one branch; never main). If no run of `platforms` starts within 5 minutes (a path filter on a new
  branch can match nothing, KNOWN uncertainty), the director starts it by `workflow_dispatch` on that ref.
- The director reads the run with the GitHub Actions tools (`actions_list` `list_workflow_jobs`,
  `get_job_logs`; artefacts with `download_workflow_run_artifact`) and writes into this task's director
  log: run id, head sha, and per job conclusion, failed step, wall time, disk line, and the first lines
  of the window probe and of the procedure's verdicts. The implementer and the test author read only that
  record (and the artefact files the director saves under the task's scratch if asked).
- **Expected rounds: 2–3.** Round 1 discovers runner facts (installed X libraries and Mesa, macOS start
  delay, the probe's line, disk); round 2 fixes the workflow; a third if the procedure needs a package.
  Each round costs the slowest job's wall time (planning figure 10–20 min for macOS/Windows, ADR-0009;
  the Linux gate takes 347–490 s, R-255 probe d3) plus macOS queueing.
- **Before queueing** (§8.1): the rebased head pushed to `ci/T-070` must have `gate-linux` and
  `window-linux` green (Linux counts); `gate-macos`, `gate-windows` and `window-probe` are tracking and
  only recorded. After the merge, the director reads `platforms` and `ci.yml` on main and records both.

### Expected to FAIL at this task's merge, and why that is acceptable
- `gate-windows`: red. `kiln-workspace` does not compile for Windows (6 errors in `disk.rs:86,189,213,361`,
  probe a3) and 33 test files or modules use Unix-only APIs (238 errors on main 15555cc, R-261 re-check
  with the probe's stub; `win-files.txt`). The gate step fails at `clippy` and later steps (`--keep-going`
  reports all). The core-crates step is expected to pass for `kiln-text`, `kiln-syntax`, `kiln-search`
  and to fail for `xtask` (6 of its test files or modules use Unix APIs; T-071). Acceptable: tracking
  (Q-1 A); T-071, T-073–T-076 make it green.
- `window-probe` on Windows: `cargo build -p kiln` fails (same cause), so the probe records "not run".
- `gate-macos`: unknown; everything compiles for macOS (probe a4). Red is acceptable (tracking) and is
  filed as T-072's input.
- `window-probe` on macOS: unknown (session, OpenGL); whatever it prints is T-077's input.
- Not acceptable at merge: `gate-linux` or `window-linux` red on the candidate.

## Acceptance criteria
Local (gate tests by the independent test author in `xtask/tests/acceptance_t070.rs`, reading files as
text; no YAML crate is in the lock, `grep -c 'name = "serde_yaml\|yaml' Cargo.lock` = 0, so the test parses
the lines it needs and adds no dependency):
- AC-1: the workflow file has the triggers of Interface (1) (both branch patterns, the `paths-ignore`
  list equal, in order, to the one derived from `xtask/ledger.toml` as T-041 derives it, `schedule`,
  `workflow_dispatch`, no `pull_request*`); `permissions` `contents: read` and nothing else; no
  `concurrency`; no `continue-on-error`; the five job ids with their pinned `runs-on` labels; each gate
  job runs `cargo xtask ci --keep-going --report`; `gate-linux` is limited as stated; every `uses:` has a
  40-hex sha. Oracle: exact values; the ledger list computed from `xtask/ledger.toml` by the test.
- AC-2: a toy repository whose `.gitattributes` is the task's file, holding one LF file and one CRLF file,
  cloned with `core.autocrlf=true`, has both files byte-equal to their blobs; the same repository without
  the attribute file has the LF file changed (the negative control). Oracle: bytes.
- AC-3: `.github/scripts/window-procedure.sh` contains every case line of R-251 (`case_ ac2`, `ac2x3`,
  `dblclick`, `scroll100`, `ctrlq`, `noroot`), the AC-3 block and the plain-window block, and its exit rule
  (Interface (3)) — tested by running it with a fake `kiln` (the test author's R-251 approach: a script
  whose window form runs the real `kiln script`) under Xvfb where available; where `xvfb-run` is absent
  the test fails naming it (never passes). *(If the test author finds this cannot run inside the gate
  deterministically, they say so in a QUESTION and AC-3 becomes the reviewer's run; ruling Q-3 below.)*
Observed only through CI (the director's record is the evidence; the reviewer may claim only what a
recorded run shows, §8.1):
- AC-4: on the final candidate head, `gate-linux` and `window-linux` green; the procedure's artefact shows
  `WINDOW CASES PASS` and `PLAIN PASS` and the AC-3 line.
- AC-5: on the same run, `gate-macos`, `gate-windows`, both `window-probe` legs completed (any colour) and
  uploaded their artefacts; the Windows core-crates file shows the three core crates' test results.
- AC-6 (reviewer, round 1): a second candidate with one deliberately failing assertion added in a
  scratch commit on top (never merged) turns `gate-macos`, `gate-windows`, `gate-linux` red, and the
  `test-output.txt` artefact shows that test's panic message. The director pushes it as
  `ci/T-070-negative` on the reviewer's `CI-REQUEST` and records it.
- AC-7 (reviewer, locally): the procedure file run here against main's `kiln` under Xvfb, with the two
  libraries unpacked into scratch as ADR-0008 Q-2 A allows, exits 0; with a `kiln` whose window form is
  broken (the test author's or reviewer's mutant) it exits 1.

What cannot be tested locally: that the triggers fire (including a first push of a `ci/` branch with a
path filter), runner contents, artefact upload, any macOS or Windows result, wall times.

## Out of scope and implementer notes
- Do not edit `ci.yml` (T-041 adds its filter and will extend its drift test to `platforms.yml`).
- Do not change `xtask/src/` (the gate's steps, its report format) — T-071 and later.
- Do not make macOS or Windows green (T-071 to T-076), or change window code (T-077), or build release
  artefacts (T-078).
- No `continue-on-error`, no `if: false`, no retries of a failed step.
- The implementer may iterate on CI only through `CI-REQUEST` lines; logs and artefacts come back through
  the director's record. Commit before each request.

## Collisions and order
- T-041 (`ci.yml`, `docs/development.md` "CI", `xtask/tests/acceptance_t041.rs`): shares
  `docs/development.md` → not in flight together (§5.2). T-041 after T-070; its scope note gains
  "`platforms.yml` too".
- T-043 and T-064 also list `docs/development.md` → not in flight with T-070.
- T-071 lists `docs/development.md` → after T-070 (it also depends on it).
- T-054 (`Cargo.toml` profiles): independent; if round 1 shows a runner out of disk, T-054 is specified
  before T-073 (ADR-0009).
- UI tasks T-057–T-059, T-066, T-068: no shared path; may be in flight with T-070.
- T-077 later edits `platforms.yml` (window on macOS/Windows); T-078 too (release builds).

## Estimate
High tasks of this stage: 887,544 (T-065, D-168), 997,800 (T-055), 1,190,305 (T-056); stage-1 high median
810,888. Little Rust (one gate test), but 2–3 CI rounds with waiting: **about 850,000** builder and review
tokens; wall time dominated by CI.

## Questions and rulings
- **Q-1 (planner, R-261). Does `gate-linux` run on main pushes too?** A: no; `ci.yml` stays the Linux gate
  on main until T-041 (as specified). B: yes, duplicate Linux runs on main. **Recommendation: A.**
- **Q-2 (planner, R-261). The core-crates step.** A: reported, not required (as specified). B: required on
  Windows (the job colour then depends on it while the gate step is red anyway). C: drop it.
  **Recommendation: A** — it is the only Windows evidence before T-073 and cannot hide a gate failure.
- **Q-3 (planner, R-261). Is AC-3 a gate test?** A: yes, the test author runs the procedure with a fake
  `kiln` under Xvfb inside `acceptance_t070.rs` — measured here? Not yet: whether `xvfb-run` works inside a
  gate test run in a queue worktree is unmeasured (probe: the test author runs it once in the worktree
  and reports). B: AC-3 is a reviewer's run, and the gate test only checks the file's structure.
  **Recommendation: B** unless that probe passes, because a gate test needing a display would fail on a
  machine without `xvfb-run` (NEW-3's lesson).
- **Q-4 (planner, R-261). `AC-3 PRECONDITION FAILED` on the runner.** A: accepted (reported, procedure
  exits 0). B: treated as a failure. **Recommendation: A** — the runner's packages are not ours; the
  no-library exit stays covered by the evaluator's local run (ADR-0008 Q-2 A).

### Spec check R-261
Against main `15555cce3bfafdd8a579127ff49c3acd7f717ba6` (v1.4; D-175 to D-179). Changed from the R-255 draft:
- `depends_on` T-069 is merged (D-179); CI run 366 green.
- Added `gate-linux` for `ci/**`, schedule and dispatch (a candidate needs a Linux run in CI; `ci.yml` runs
  only on main: `on.push.branches: [main]`); kept `ci.yml` untouched.
- The procedure becomes a file `.github/scripts/window-procedure.sh` owned by the test author, with an exit
  status (R-251's block always exits 0: it only prints verdicts, read in `tasks/T-065-window.md` lines
  672–810). Added the core-crates step (xtask and the three core crates do not depend on `kiln-workspace`).
- `.gitattributes` content pinned and VERIFIED (Pinned (g); scratch `r261/`, no build, deleted).
- The Windows failure re-measured on 15555cc: 238 errors in 33 files (R-255: 198; T-069 added Unix code to
  six test files), `kiln-workspace` lib still the only product crate failing.
- No YAML crate in the lock: AC-1 is a text check without a new dependency.
- `docs/development.md` is shared with T-041, T-043, T-064, T-071: sequencing stated.
- Not checked: the runner facts (only round 1 can), `actions/upload-artifact`'s sha (the implementer reads
  it), whether a first push of a `ci/` branch with `paths-ignore` starts a run (fallback: dispatch).

### Rulings on spec check R-261 (director, D-180)
- **Q-1: A.** `gate-linux` does not run on pushes to main; `ci.yml` stays the Linux gate there.
- **Q-2: A.** The core-crates step is recorded and does not change a job's colour.
- **Q-3: B.** The window procedure is run by the reviewer here and by the `window-linux` job in CI; it is not a
  step of the local gate. (The alternative, a local gate step under Xvfb, was not measured; it would also make the
  local gate fetch two libraries from the network, which is reason enough not to probe it.)
- **Q-4: A.** Where the runner already has the two X libraries installed, the procedure's check without them
  cannot run there; it is reported as not run on that runner, not as a failure.
- **Pushing `ci/T-070` branches** to the owner's repository is asked of the owner before the first push (the
  director's standing permission covers `main` only). Until the owner answers, the builders work up to their first
  `CI-REQUEST` and the director holds the request.
- Tests added by this task use no Unix-only API (T-071's rule is not merged yet).

## Log
### director
<!-- end director log -->
### planner
- 2026-11-15 · R-255 · draft · did: filed from ADR-0009 · next: director
- 2026-11-15 · R-261 · draft · did: spec check and completion against 15555cc (above) · not run: anything on a hosted runner · next: director (rulings Q-1–Q-4, specified)
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
- 2026-11-15 · R-264 · ready (base 0bc14b8) · did: read AGENTS.md, the test-author brief, METHODOLOGY
  §8.1, this spec, ADR-0009 and its rulings, `tasks/T-065-window.md` R-251, `docs/protocol/ui.md`
  "The window"; did not read `task/T-070` or its worktree. Rulings acknowledged and accepted: Q-1 A
  (my test requires `gate-linux` not to run for a push to main), Q-2 A (the core-crates step is
  required to run whatever the gate's result; its colour is not tested), Q-3 B (AC-3 in the gate is
  the file's structure and the exit rule for a run in which no verdict can pass; the run with a real
  window is the reviewer's, AC-7, and `window-linux`'s, AC-4), Q-4 A (the procedure exits 0 with
  `AC-3 PRECONDITION FAILED`, which is its report that AC-3 was not run there), the `ci/` push
  permission (not mine), "no Unix-only API" (none used; no `cfg`).
  - **`.github/scripts/window-procedure.sh`.** R-251's block (lines 671–811 of
    `tasks/T-065-window.md`) with every line unchanged, inside a function `procedure`; the usage
    line names the file; a wrapper passes the output through line by line and exits 0 only if it
    saw the lines `WINDOW CASES PASS`, `PLAIN PASS: …` and `AC-3 PASS` or `AC-3 PRECONDITION
    FAILED: …`, else 1 (also when R-251's `exit 99` ends the body early). `diff` against the block:
    the usage line, 8 added lines before the body, 14 after it.
  - **Run here** (Xvfb, the two libraries unpacked into scratch by the procedure itself, `kiln`
    built from base 0bc14b8 in this worktree, `sh .github/scripts/window-procedure.sh
    target/debug/kiln <scratch>/run-base`, 15 s): every case `exit window=N headless=N` equal (0,
    3, 0, 0, 0, 0), `without counts: EQUAL | with counts: EQUAL`, `files EQUAL`; `WINDOW CASES
    PASS`; `AC-3: exit=2 stdout_bytes=0 stderr_lines=1 panicked=0`, `AC-3 PASS`; `missing script:
    exit=2 stdout_bytes=0`; three plain runs drawn and stable, the two without a path the same
    screen, `PLAIN PASS`; **exit 0**. With `ldconfig` replaced on `PATH` by one that lists both
    libraries: `AC-3 PRECONDITION FAILED: a library is installed`, the rest as before, **exit 0**
    (Q-4 A).
  - **Broken variants** (wrappers around the same `kiln`): (k1) window form with the first
    response line's id altered → cases ac2, ac2x3, noroot `DIFF at line 1 … NOT EQUAL`, `WINDOW
    CASES FAIL`, **exit 1**; (k2) plain window drops its start paths → `PLAIN FAIL: the screen with
    start path a.rs equals the one without`, **exit 1**; (k3) "no window" exits 0 instead of 2 →
    `AC-3: exit=0 …`, `AC-3 FAIL`, **exit 1**. Each other verdict passed in these runs.
  - **Gate tests** `xtask/tests/acceptance_t070.rs` (11). Result on base (0bc14b8, no
    `platforms.yml`, no `.gitattributes`, no procedure): all 11 fail by **assertion** (`… does not
    exist (T-070 adds it)`); in AC-2 the negative control runs first and passes. On my branch the
    three AC-3 tests pass (they test my file) and the eight others fail by the same assertion.
    - AC-1: `ac1_name_permissions_no_concurrency_no_continue_on_error`,
      `ac1_triggers_and_the_ledger_filter` (the `paths-ignore` list computed from
      `xtask/ledger.toml` as T-041 Interface (1) says; branches; one daily cron at a fixed minute;
      dispatch without inputs; no `pull_request*`), `ac1_five_jobs_with_their_runner_labels_and_conditions`
      (labels; the probe's matrix; the `if:` of every job evaluated for push to main, push to
      `ci/T-070`, schedule, dispatch on main and on `ci/T-070`: `gate-linux` runs for all but a
      push to main, the four others for all),
      `ac1_gate_jobs_run_the_gate_and_upload_the_reports_whatever_its_result` (per gate job: the
      Interface's steps in order, the gate step without `if:`; core crates, disk and upload run
      after a failed and a passed gate, test output only after a failed one; artefact name, 30
      days, the three files), `ac1_window_jobs_build_run_and_upload`,
      `ac1_defaults_env_checkout_and_every_action_pinned_by_sha` (bash default; the two env values
      per job; checkout depth 1, no credentials; every `uses:` with a 40-hex sha, checkout and
      rust-cache at `ci.yml`'s shas; a comment naming the release on the line or the line above).
      No YAML crate: a reader for the block subset in the file, and an evaluator for `if:`
      (`==`, `!=`, `&&`, `||`, `!`, status functions, `startsWith`); it fails naming a construct it
      cannot read (anchors, multi-line flow, `?` keys), never passes it.
    - AC-2: `ac2_autocrlf_clone_keeps_every_byte_with_the_attribute_file` (toy repository, LF and
      CRLF file, the task's `.gitattributes`, `git clone -c core.autocrlf=true`, both files equal to
      `git cat-file -p HEAD:<f>`; control without the file: the LF file changes, a precondition
      panic if not), `ac2_gitattributes_is_the_one_pinned_line`.
    - AC-3: `ac3_procedure_holds_every_line_of_r251_in_order` (oracle: the block read from
      `tasks/T-065-window.md`; every line but the usage line in the file, in order; the six cases),
      `ac3_usage_line_names_the_file`, `ac3_exit_status_is_1_when_no_verdict_passes` (the file run
      by `sh` with a `kiln` that does not exist and an empty directory as `PATH`: it runs to its end
      (`WINDOW CASES FAIL`, `PLAIN NOT RUN …`), prints no pass line, exits 1; no display, no network,
      0.1 s here).
  - **Self-check.** Stub (scratch, deleted, never committed): a `platforms.yml` written from the
    Interface and `.gitattributes` `* -text`; all 11 pass; a reformatted stub (block-list branches,
    flow `permissions`, quoted `"on"`, `if:` without `${{ }}` and with its operands swapped, inline
    comments) passes too. 30 workflow mutants, each killed (test in brackets): continue-on-error
    [name_permissions]; gate-linux `== 'push'` and without `if:` [five_jobs]; no `--keep-going`,
    upload without `if: always()`, test output `always()`, retention 7, report not uploaded, wrong
    artefact name, core crates without `-p xtask`, upload before the gate [gate_jobs]; ledger list
    one short, two swapped, `add-only` entry included, `pull_request` added, no schedule, hourly
    cron, no dispatch, `paths:` instead of `paths-ignore` [triggers]; tag instead of sha [pinned];
    short sha, other checkout sha [pinned, gate_jobs]; `actions: write` added, `concurrency` added
    [name_permissions]; labels swapped, `macos-latest` in the matrix, an `if:` on gate-windows
    [five_jobs]; persist-credentials true, shell pwsh [pinned]; procedure path wrong [window_jobs].
    Attributes: `* text=auto`, `*.rs -text` [both ac2 tests]; an extra `*.sh eol=lf` line, `*
    binary`, `* -text eol=crlf` [pinned_line only: equivalent in behaviour for the toy]. Procedure:
    R-251's block as is (always exit 0) and `exit 0` at the end [exit_status]; a case line dropped
    and a verdict line reworded [holds_every_line]; usage still `ac2.sh` [usage_line]. Two survive
    the gate, as Q-3 B implies: the exit rule counting only `WINDOW CASES PASS` (with k2: exit 0
    while printing `PLAIN FAIL`; the file of record exits 1 with k2) and the precondition not
    accepted (with the fake `ldconfig`: exit 1; the file exits 0) — both killed only by the run
    under Xvfb, which is AC-7's and AC-4's. `cargo clippy -p xtask --all-targets --locked -D
    warnings` clean with the stub in place.
  - **For the director: what to read from a run** (workflow `platforms`, the run on the final
    candidate head pushed to `ci/T-070`; check the run's head sha equals the candidate's).
    - AC-4: job `gate-linux` conclusion `success` (not `skipped`), its step running `cargo xtask ci
      --keep-going --report gate-report.json` `success`. Job `window-linux` conclusion `success`;
      artefact `window-linux-<run_id>`, file `window-procedure.txt`: six lines beginning `CASE
      ac2:`, `CASE ac2x3:`, `CASE dblclick:`, `CASE scroll100:`, `CASE ctrlq:`, `CASE noroot:`; the
      line `WINDOW CASES PASS`; the line `AC-3 PASS` or a line beginning `AC-3 PRECONDITION
      FAILED` (record which: the second means AC-3 was not checked on the runner, Q-4 A); a line
      beginning `PLAIN PASS`. Also record, not required: each case's `with counts:` value, the
      `missing script:` line, the `dpkg-query` line.
    - AC-5: the same run: jobs `gate-macos`, `gate-windows` and both `window-probe` legs (macos-15,
      windows-2025) have status `completed` and conclusion `success` or `failure` (`cancelled` or
      `skipped` = not met); artefacts `gate-macos-<run_id>`, `gate-windows-<run_id>` (with at least
      `gate-report.json` and `core-crates.txt`), and two `window-probe-…-<run_id>` with
      `window-probe.txt`. In `gate-windows`' `core-crates.txt`: for each of `kiln_text`,
      `kiln_syntax`, `kiln_search` a `Running …` line naming it followed by a `test result:` line
      (record each `ok`/`FAILED` and counts). See QUESTION (Q-a): as pinned this is not expected.
    - AC-6: run on `ci/T-070-negative` (head = candidate + the reviewer's scratch commit): jobs
      `gate-macos`, `gate-windows`, `gate-linux` conclusion `failure`, failed step the `cargo xtask
      ci …` step; in each `gate-<os>-<run_id>` artefact `test-output.txt` contains the panic
      message the reviewer names for its test (on Windows, if the workspace does not build, the
      file shows compile errors instead: record that, it is the T-073 state, not the test's run).
  QUESTION: (Q-a) The core-crates step runs one `cargo test --locked -p kiln-text -p kiln-syntax
  -p kiln-search -p xtask`. Measured here (scratch workspace, two packages, one with a test file
  that does not compile, cargo 1.97.0): `cargo test -p good -p bad` runs **no** test of `good`,
  with or without `--no-fail-fast` (exit 101 at the compile error). The spec expects `xtask`'s
  tests not to compile on Windows (6 files, T-071), so `core-crates.txt` would show compile errors
  and no result for the three core crates, and AC-5's last clause cannot be met. Proposed: the step
  runs one `cargo test --locked -p <package>` per package, each appending to `core-crates.txt`
  (still one step, still reported, not required). My test accepts that (one step whose `run`
  holds `cargo test`, `--locked`, the four `-p`, `core-crates.txt`); separate steps would need my
  test changed. Recommendation: rule it for the implementer.
  QUESTION: (Q-b, for the record) In one of 8 runs of the procedure here, case `ac2x3` gave
  `exit window=5 headless=3` (stdout equal, window stderr empty); 20 repeats of that case gave 3
  each time. Exit 5 is not a `kiln` exit code (`docs/protocol/ui.md`); `/usr/bin/xvfb-run` exits 5
  on "problem while cleaning up temporary directory" (line 87): INFERRED to be `xvfb-run`, with
  other agents' displays running here. Under the exit rule that turns `window-linux` red. I changed
  nothing (every comparison stays R-251's; no retries, spec). If a CI run shows it, the director
  decides.
  Notes, not tested (the Interface does not pin them): `window-probe` with GitHub's default
  `fail-fast: true` can cancel the macOS leg when the Windows leg fails (AC-5 then reads
  `cancelled`); `window-linux`'s upload runs after a failed procedure only with `if: always()`.
  T-077 and T-078 edit `platforms.yml`: my tests require the five jobs, not that no others exist;
  a T-077 change to `window-probe` needs `ac1_five_jobs…`/`ac1_window_jobs…` updated by a high-tier
  test author.
  · gate attempts: 1 of `cargo xtask ci --keep-going` (10 min 25 s): every step PASS or EMPTY but
  `test` (930 passed, 3 failed: three `acceptance_t005_r3` tests, which then passed 11 of 11 alone;
  the volume was full during the run, 0 MB free after it, INFERRED cause; `cargo test` stopped at
  that suite). Then `cargo test -p xtask --locked --no-fail-fast`: every suite ok but
  `acceptance_t070` (3 passed, 8 failed by the assertion above, as expected on this branch);
  `hygiene` and `no-skip` PASS · not run: the gate tests on
  macOS or Windows (the exit-rule test needs `sh` there: Git Bash on Windows, unmeasured), any
  hosted runner · missing context: none · next: join, reviewer (AC-7 with this file)
- 2026-11-15 · R-267 · ready · rulings of D-182 (read in the task file on main) acknowledged and
  accepted before acting: m1 (one `cargo test --locked -p <package>` per package in the one
  core-crates step, each appending to `core-crates.txt`; my test will require it), m2 (a window
  case's exit status is `kiln`'s, recorded inside the wrapped command; I change the procedure and
  its block test), m3 (window-probe does not cancel a leg; window uploads run whatever the result;
  I add the checks), m4 (`sudo apt-get update` allowed in `window-linux`; my tests do not forbid
  it), m5 and m6 (noted; nothing for my tests), the test-output condition
  `failure() && steps.gate.outcome == 'failure'` (my test accepts it and `failure()`), round 1 on
  the joined branch, and the disk rule (23 GB free on `/home/user` at start).
  · did, by ruling (D-182):
  m1: `ac1_gate_jobs_…` now requires the core-crates step to run one `cargo test --locked -p <p>`
  per package (four lines, or a `for v in <the four>; do … -p "$v" …` loop), each appending
  (`>>` or `tee -a`, on the command or the loop's `done`); a `cargo test` naming 2+ packages fails
  the test. Because the job's `bash` runs with `-e`, it also requires that a failing package does
  not stop the others or the step (`||` after each, or after the loop; or `set +e` and a final
  `true`/`:`/`exit 0`): that is what the ruling's purpose needs. m2: in the procedure the
  wrapped command writes `kiln`'s status to `w_<case>.st` (AC-3: `n.st`) in the scratch directory
  (outside the fixture copies, so `diff -r` is unchanged) and the case reads it there; not written
  means `WINDOW EXIT NOT RECORDED …` and the case fails (AC-3: `exit=not-recorded`, AC-3 FAIL); a
  wrapper status that differs is printed (`wrapper exit=N (not kiln's; not used)`). The R-251
  lines allowed to differ, now named in `R251_REPLACED` and required to be absent: the usage line;
  `xv sh -c 'exec "$0" "$@" 2>"w_'"$n"'.err"' … > "w_$n.out"; wc=$?` (root case);
  `(cd "w_$n" && xv sh -c 'exec … > "../w_$n.out"); wc=$?` (noroot case);
  `xvfb-run -a … sh -c "exec '$K' --root n a.rs 2>n.err" > n.out; e=$?` (AC-3). One line is added
  after the CASE line. AC-3's line is my extension of m2 (the same wrapper, the same risk); the
  director may rule it out. m3: `ac1_window_jobs_…` requires `strategy.fail-fast: false` on
  `window-probe` and that the uploads of `window-linux` and `window-probe` run on a failed and a
  passed run. m6: the evaluator now reads `steps.<id>.outcome`/`.conclusion` (before this it
  panicked on them); the test-output step must run when the gate step (by its `id:`) failed and
  not on a passed run, so `failure()` and `failure() && steps.gate.outcome == 'failure'` both pass.
  m4: nothing forbids `sudo apt-get update` (mutant below).
  · procedure under Xvfb (kiln built from this branch, 23 then 14 GB free): 10 of 10 runs exit 0,
  each with `WINDOW CASES PASS`, `AC-3 PASS`, `PLAIN PASS`; cases ac2/ac2x3/dblclick/scroll100/
  ctrlq/noroot exit window/headless 0/0, 3/3, 0/0, 0/0, 0/0, 0/0 in every run; no `wrapper exit`
  line (the status-5 run of R-264 did not recur). Forced: an `xvfb-run` that runs the real one and
  exits 5: exit 0, every case PASS with `wrapper exit=5 … not used`; an `xvfb-run` that runs
  nothing: exit 1, 6 × `WINDOW EXIT NOT RECORDED`, `AC-3: exit=not-recorded`, AC-3 FAIL. Broken
  kiln variants, each exit 1: first window line altered (NOT EQUAL in ac2, ac2x3, noroot; my
  wrapper also exits 0 there, so ac2x3 shows EXIT DIFFERS too); plain window drops its path
  (`PLAIN FAIL`); no-window exits 0 (`AC-3: exit=0`, AC-3 FAIL); window exits 4 in dblclick only
  (`exit window=4 headless=0`, EXIT DIFFERS, WINDOW CASES FAIL).
  · self-check: scratch stub workflow + `* -text` (deleted, never committed): 11 of 11 pass. 22
  mutants: killed: single 4-package invocation, loop without xtask, no `||`, `>` not `>>`, no
  `--locked`, `set +e` without a final `true`, test-output naming another step / `always()` /
  gate step without `id:`, fail-fast removed / true, window-linux upload without `if:`, probe
  upload without `if:` / `success()`, each of the three R-251 lines reverted; pass as they
  should: four written-out lines, `set +e` + `tee -a` + `true`, test-output `failure()`, uploads
  `success() || failure()`, no `apt-get update`. (`>` first survived, the `|| echo … >>` on the
  same line matched; fixed: only the invocation's part before `||` counts.) Clippy `-D warnings`
  clean with the stub. Committed state: `acceptance_t070` 3 passed, 8 failed by assertion
  (no workflow, no `.gitattributes` on this branch), as before · gate attempts: none of
  `cargo xtask ci` (not asked; acceptance_t070, clippy, hygiene run) · not run: anything on a
  hosted runner; my tests against the implementer's workflow (the join) · next: director (join)
- 2026-11-15 · R-268 · ready · rulings of D-183 acknowledged and accepted before acting: m7 (AC-3's
  line under m2 stays), m8 (my core-crates reader counted the `cargo test` inside an `echo` as an
  invocation; I make it count commands only, keeping every m1 requirement).
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
