---
id: T-070
title: "Run the gate in CI on macOS and Windows as tracking jobs, check files out byte for byte, keep the reports, probe the window on each platform, and run the Linux window procedure under Xvfb on every code push"
tier: high              # METHODOLOGY v1.4 §4: gate paths (.github/workflows/platforms.yml, .github/scripts/, xtask/tests/); the approval row must contain `gate paths`. No dependency change.
state: queued
owner_role: planner
scorecard_items: []     # produces evidence; changes no behaviour of Kiln's binaries
depends_on: [T-069]     # merged (30c66909, D-179): the existing CI job is green, so a run can be read
touches:                # one role per path (D-153)
  - .github/workflows/platforms.yml          # GATE PATH: implementer (new; ci.yml is NOT edited: T-041 owns it)
  - .gitattributes                           # implementer (new; one line, Pinned (b))
  - docs/development.md                      # implementer: the "CI" section only (lines 535–540 on main 15555cc), not the T-069 paragraph after it
  - .github/scripts/window-procedure.sh      # GATE PATH: test author (new; the T-065 procedure as a file, Interface (3))
  - xtask/tests/acceptance_t070.rs           # GATE PATH: test author
  - xtask/tests/guard_t070.rs                # GATE PATH: test author (m9, D-189: the three tests of the procedure file, which pass on the tests branch)
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

### Rulings on the test author's questions of R-264 (director, D-182; told to both builders)
- **m1 (Q-a).** The core-crates step runs one `cargo test --locked -p <package>` per package (`kiln-text`,
  `kiln-syntax`, `kiln-search`, `xtask`), each appending to `core-crates.txt`, inside the one step, so that a
  package whose tests do not compile on a platform does not stop the others (measured by the test author: with one
  `cargo test -p a -p b` cargo runs no test at all when one package's test file fails to compile).
- **m2 (Q-b).** A window case's exit status is the status of `kiln` itself, not of the virtual-display wrapper: the
  procedure records `kiln`'s status inside the wrapped command and reads it from there; if it was not recorded the
  case fails and says so. (In 1 of 8 runs here a case showed the wrapper's own status 5; 20 repeats gave 3 every
  time.) The test author changes the procedure file and its test of the block's lines accordingly, stating each
  line that differs from the block in `tasks/T-065-window.md`.
- **m3.** The window-probe matrix does not cancel one platform when the other fails, and the uploads of the window
  jobs happen whatever the procedure's or the probe's result; the implementer makes it so and the test author
  adds the checks to its workflow tests.
- **Disk.** The disk was full while three builders compiled (0 MB free, reported by the test author); a builder
  checks `df -h /home/user` before a gate run and stops and reports below 14 GB free.

### Rulings on the implementer's questions of R-263 (director, D-182; told to both builders)
- **m4 (question c).** The `window-linux` job may refresh the runner's package lists (`sudo apt-get update`)
  before the procedure; it installs nothing.
- **m5 (question a).** A window-probe leg is green only when the window form ran the script and `kiln` exited 0;
  its colour is `kiln`'s exit status. A leg that cannot open a window is red and stays tracked ("built, not
  observed", ADR-0009 Q-5).
- **m6 (question b).** `window-linux` is tracked until a decision row names its first green run on main, like the
  other jobs of this workflow; AC-4 still needs it green on the candidate before the task is queued.
- The condition of the test-output step, `failure() && steps.gate.outcome == 'failure'`, is accepted as the
  spec's "only when the gate failed"; the test author's workflow test accepts either form.
- **Round 1 of CI is run on the joined branch** (workflow and procedure together), not on the implementer's
  commit alone, once the owner has allowed pushing `ci/…` branches.

### Rulings after the first merge of the two branches (director, D-183; told to the test author)
- **m7.** The test author's extension of m2 to AC-3's line (the wrapped command records `kiln`'s status there too)
  is accepted.
- **m8.** With the two branches merged (`e3b80ddb27d87b7b81f0a3c17dbbf856f34f1a3d`), 10 of the 11 gate tests pass
  against the implementer's workflow and one fails for a fault of the test:
  `ac1_gate_jobs_run_the_gate_and_upload_the_reports_whatever_its_result` counts the packages of the core-crates
  step twice ("the step tests [kiln-text, kiln-syntax, kiln-search, xtask, kiln-text, …]"), because the step
  echoes the command it is about to run (`echo "==== cargo test --locked -p $p" >> core-crates.txt`) and the test's
  reader takes the echoed text for an invocation. The step is as m1 rules it: a loop over the four packages with
  one `cargo test --locked -p "$p"` each, appended to `core-crates.txt`, under `set +e` with a final `exit 0`. The
  test counts invocations, not text inside an `echo` or a quoted string.

### Ruling after the refused join (director, D-189; told to the test author)
- **m9.** The join refused the task (`refused_base_passes`): the three AC-3 tests
  (`ac3_procedure_holds_every_line_of_r251_in_order`, `ac3_usage_line_names_the_file`,
  `ac3_exit_status_is_1_when_no_verdict_passes`) pass on the tests branch, because the file they test,
  `.github/scripts/window-procedure.sh`, is the test author's own and is on that branch; an acceptance test must
  fail on the base. They are tests of a file that does not depend on the implementation: they move, unchanged, to
  `xtask/tests/guard_t070.rs` (added to `touches`), where the join expects tests that pass on the base and on the
  joined branch. The eight tests of the workflow and of the attribute file stay in `acceptance_t070.rs`.

### Rulings on review round 1 (director, D-193; `reviews/T-070-r1.md` at `3c7399be9767809cca0ea749686aa71a70a9760b`; told to both builders)
- **g1 (finding 1, defect; test author).** The procedure's exit status cannot be turned by what `kiln` prints: a
  line of `kiln`'s output that looks like a verdict is not a verdict (its echoed lines are told apart from the
  procedure's own, for example by an indent on every echoed line), and the wrapper exits non-zero when any FAIL
  or NOT RUN verdict was printed. The reviewer's forged `AC-3 PASS` case is added to the guard tests.
- **g2 (finding 2; implementer).** Every recording step of the workflow ends with exit status 0 whatever its
  commands return, the Disk step included, so that the sentence in `docs/development.md` is true.
- **g3 (findings 3 and 4; implementer).** The CI section states wall times and disk as the recorded run shows
  them (run 43752760283, in the task's director log on main and `tasks/ci-runs/T-070-round-1.md`): job wall times
  `gate-linux` 15 min 34 s, `gate-macos` 21 min 20 s, `gate-windows` 9 min 15 s, `window-linux` 2 min 13 s,
  `window-probe` 2 min 26 s (macOS) and 2 min 8 s (Windows); the `df` and `du` lines as printed. "14 GB disk each"
  and "not measured yet" go.
- **g4 (finding 5 and the two surviving mutants; test author).** The workflow tests also require: the procedure
  step's exit status is not masked; the probe step exits with `kiln`'s status; the probe step runs whatever the
  build's result and records "not run" when the build failed; the probe's time bound is the stated one; the window
  job builds the profile whose binary the procedure runs.
- **g5 (finding 6, spec gap).** AC-6 is read from the job logs of the run on `ci/T-070-negative`: on each of the
  three gate jobs the `CORE CRATES` line shows `kiln-text=101` (round 1 showed 0), `gate-linux` fails at "Gate" with
  a `GATE FAILED` line naming `test`, and the panic sentence is quoted where a log shows it; on Windows the
  test-output shows compile errors, not the sentence, until T-073. Artefacts are read if the director's tools can
  download them; the criterion does not depend on it.
- **g6 (finding 7, spec gap).** T-077 changes `window-probe`, which these tests pin: `xtask/tests/acceptance_t070.rs`
  is noted for T-077's `touches`.
- **g7 (finding 8, nit; implementer).** The probe's watchdog ends a `kiln` that ignores the first signal.
- AC-4 is read again on the final candidate: after the rework and the re-join the director pushes the joined head
  to `ci/T-070` (round 2).

## Log
### director
- 2026-11-15 CI round 1 (D-187). Candidate `cd1121b9e69a949bddd0d91afce5b96887747894` (both branches merged) pushed
  to `ci/T-070` at 19:30:06Z; run id 43752760283 (run 1 of `platforms.yml`), read with the Actions tools by a helper
  under the director (extraction: `tasks/ci-runs/T-070-round-1.md`; artefact contents were not downloaded).
  - `gate-linux`: success, 15 min 34 s. `GATE PASSED in 647.6s`; test `990 passed, 0 failed, 0 ignored in 120
    suites`; scorecard `64 items: 44 verified, 0 implemented, 0 failing, 20 planned.`; `CORE CRATES: exit status per
    package: kiln-text=0 kiln-syntax=0 kiln-search=0 xtask=0`; disk `8.6G target`.
  - `window-linux`: success, 2 min 13 s. Six `CASE` lines with equal exit statuses (ac2 0/0, ac2x3 3/3, dblclick
    0/0, scroll100 0/0, ctrlq 0/0, noroot 0/0); `WINDOW CASES PASS`; `AC-3 PASS` (the two X libraries are not
    installed on the runner: `dpkg-query: no packages found matching libxkbcommon-x11-0`, likewise `libxcb-xkb1`);
    `missing script: exit=2`; `PLAIN PASS: the start path changes the screen`; `APT LISTS: 58 files`.
  - `window-probe (macos-15)`: success, 2 min 26 s. `PROBE: exit status 0`; stdout 3 lines (two snapshots showing
    `Path`, `Open`, `Documents`; one `doc.list`); stderr empty. macOS 15.8.2, arm64.
  - `window-probe (windows-2025)`: failure at "Build kiln" (6 errors in `crates/kiln-workspace/src/disk.rs`);
    `PROBE: not run (cargo build -p kiln: failure)`.
  - `gate-windows`: failure at "Gate", 9 min 15 s. `GATE FAILED in 194.7s: clippy, schema, scorecard, test`; fmt,
    layers, ui-commands, hygiene, no-skip pass. Core crates: kiln-text=0, kiln-syntax=0, kiln-search=0 (all `test
    result: ok`), xtask=101. Disk `2.5G target`.
  - `gate-macos`: failure at "Gate", 21 min 20 s. `GATE FAILED in 355.1s: scorecard, test`; fmt, clippy, layers,
    ui-commands, hygiene, schema, no-skip pass. The test-output step shows 46 binaries, 366 passed and 11 failed in
    its first 200 lines (the step cuts there; the full list is in the artefact): three tests need `/dev/full`;
    five no-display tests of `kiln` time out (a window opens, or the binary hangs, where Linux has no display);
    one link-loop precondition gets a different error (os error 62); `deep_trees_stop_at_the_depth_bound` cannot
    make a 2,100-deep chain; `tree_order_and_counts` differs. Core crates: three ok, xtask 116 passed and 3 failed
    (names not printed). Disk `4.4G target`.
  - Artefacts: six, one per job (`gate-linux-…` 8,822 B, `gate-macos-…` 29,358 B, `gate-windows-…` 8,377 B,
    `window-linux-…` 577 B, two `window-probe-…`).
  - Read against the criteria: AC-4 holds on this candidate (`gate-linux` and `window-linux` green with the
    verdict lines). AC-5 holds (both other gates and both probes completed with success or failure; artefacts
    present; the Windows core-crates output has a `test result:` line for each of the three crates). AC-6 (the
    negative candidate) is still to run.
<!-- end director log -->
- **CI round 2 on the final candidate (AC-4, AC-5)**: run 43765532298 of `platforms.yml` on `ci/T-070` at the joined
  head `3fac206ea934707dd42607b320c4c29c361b215c` (pushed 2026-11-15T21:46:25Z; extraction
  `tasks/ci-runs/T-070-round-2.md`, read by a helper with the Actions tools). Per job, the same conclusions as round 1:
  `gate-linux` success (`GATE PASSED in 396.6s`, 999 passed, 0 failed in 121 suites; core crates all 0; 10 min 6 s);
  `window-linux` success (`WINDOW CASES PASS`, `AC-3 PASS`, `PLAIN PASS`; the echoed lines of `kiln` are indented by
  two spaces and the verdicts start at column 0, which is g1 on a runner); `window-probe (macos-15)` success
  (`PROBE: exit status 0`); `gate-macos` failure at Gate (`GATE FAILED in 488.3s: scorecard, test`; `xtask` 116
  passed, 3 failed; the printed prefix of the full output 366 passed, 11 failed); `gate-windows` failure at Gate
  (`clippy, schema, scorecard, test`; 6 compile errors in `kiln-workspace`, 4 in `xtask` tests naming
  `xtask/src/hygiene.rs` and `xtask/src/publish.rs`); `window-probe (windows-2025)` failure at "Build kiln", and the
  probe step ran and recorded `PROBE: not run (cargo build -p kiln: failure)`, which is g4's third property on a
  runner. The macOS and Windows gates are red as ADR-0009 expects until T-072 to T-074.
- **The negative candidate (AC-6, read as g5 rules)**: run 43764233597 on `ci/T-070-negative` at
  `4bc231c3755d583707b8031bdfecb4d93fbb628f` (extraction `tasks/ci-runs/T-070-negative.md`). On each of the three gate
  jobs the `CORE CRATES` line shows `kiln-text=101` (0 in rounds 1 and 2). `gate-linux` fails at "Gate" with
  `GATE FAILED in 438.9s: test` (`622 passed, 1 failed`), and the log names the test
  (`t070_negative_candidate_fails_on_purpose ... FAILED`, `-p kiln-text --test negative_t070`). **The panic sentence
  is in none of the log parts read** (tails of 400 to 800 lines per gate job): the gate's summary and the "Full test
  output" step show the failing test's name, not its message. On Windows no `test result:` line for `kiln-text` was
  found, so that 101 may be the panic or a build failure: not determined. AC-6 holds as g5 words it (the exit
  status, the failing gate step and the test's name are in the logs); that a failing test's message is not in a job
  log is a limit of what these jobs report, noted in T-072 (the task that has to read macOS failures from them).

### planner
- 2026-11-15 · R-255 · draft · did: filed from ADR-0009 · next: director
- 2026-11-15 · R-261 · draft · did: spec check and completion against 15555cc (above) · not run: anything on a hosted runner · next: director (rulings Q-1–Q-4, specified)
<!-- end planner log -->
### implementer
- 2026-11-15 · R-263 · start · rulings acknowledged, all accepted: Q-1 A (`gate-linux` not on pushes to
  main; `ci.yml` stays the Linux gate there), Q-2 A (core-crates step reported, its exit status never
  changes a job's colour), Q-3 B (the procedure is run by `window-linux` in CI and by the reviewer here,
  not by the local gate), Q-4 A (`AC-3 PRECONDITION FAILED` on the runner is "not checked there", not a
  failure: the test author's exit rule), D-180 (pushes of `ci/T-070` only through the director; no
  test of mine uses a Unix-only API — I add no test). The criteria in my words: AC-1 the workflow
  file's text has exactly the specified triggers (two branch patterns, the 18 ledger patterns in
  `ledger.toml` order, schedule, dispatch, no PR trigger), `contents: read` only, no `concurrency`, no
  `continue-on-error`, the five job ids on their labels, each gate job runs `cargo xtask ci
  --keep-going --report`, `gate-linux` limited by the stated `if:`, every `uses:` pinned by 40 hex;
  AC-2 `* -text` makes an autocrlf clone byte-equal; AC-3 the test author's procedure file; AC-4 on
  the final candidate `gate-linux` and `window-linux` green and the procedure artefact shows the three
  verdicts; AC-5 the macOS/Windows jobs and both probe legs complete and upload, Windows core-crates
  shows the three core crates' results; AC-6 a deliberately failing test turns the three gate jobs red
  and `test-output.txt` shows its panic; AC-7 the reviewer's local run of the procedure.
- 2026-11-15 · R-263 · round 0 (local, before any CI run) · did: `.gitattributes` (`* -text`, one
  line); `.github/workflows/platforms.yml` with the five jobs of Interface (1); the "CI" section of
  `docs/development.md` (the first paragraph rewritten and new paragraphs before the T-069 paragraph,
  which is unchanged). Choices where the spec is silent: nightly cron `17 3 * * *` (03:17 UTC, stated
  in the docs); `timeout-minutes` 120 for gate jobs, 60 for window jobs (stated); `fail-fast: false`
  on the probe matrix (one leg must not cancel the other); a "Gate report" step (`if: always()`,
  prints `gate-report.json`) between the gate and the core-crates step, as `ci.yml` has one; the
  full-test-output step's condition is `failure() && steps.gate.outcome == 'failure'` (a failure of
  the gate only, as the table says); the recording steps run with `set +e` and end `exit 0`, so only
  the gate / build / procedure / probe step decides the colour; the probe's watchdog is a bash loop
  (`kill -0` each second, kill at 120 s), which needs nothing but bash on all three runners.
  · `actions/upload-artifact` v7.0.2 = `cf430e030ddbb5b0abf93d22962f4752f3646cd9`: `git ls-remote
  https://github.com/actions/upload-artifact 'refs/tags/v7*'` (lightweight tag, equal to `v7` and
  `main`), and the release page github.com/actions/upload-artifact/releases/tag/v7.0.2 (summarising
  fetch: published, marked Latest, shows that sha). `actions/checkout` v7.0.1 `3d3c42e5…` and
  `Swatinem/rust-cache` v2.9.3 `d1527c8f…` (peeled annotated tag) re-read the same way: equal to
  `ci.yml`'s.
  · Checked locally (scratch `impl-T-070/`, deleted at the end): (1) YAML parse (PyYAML) plus a script
  that recomputes the `paths-ignore` list from `xtask/ledger.toml` (18 patterns, equal in order),
  lists triggers, permissions, env, defaults, each job's `runs-on`/`if`/matrix, every step's
  `if`/`id`, artefact names and retention, and checks every `uses:` against `@[0-9a-f]{40}` (all 15
  pinned); no `concurrency`, no `continue-on-error`. (2) `actionlint` 1.7.12 (from the PyPI wrapper
  `actionlint-py`, in a scratch venv) on `platforms.yml` and `ci.yml`: exit 0, no finding;
  shellcheck was not available, so the `run:` scripts were not shell-linted. (3) Every `run:` step
  extracted and run with `bash --noprofile --norc -eo pipefail` (as Actions runs `shell: bash`) from
  the worktree root, Linux: toolchain (prints cargo 1.97.0, clippy 0.1.97, rustfmt 1.9.0); gate
  `cargo xtask ci --keep-going --report gate-report.json` → `GATE PASSED in 581.8s`, the report
  printed by the "Gate report" step, and that step without a report ("GATE REPORT: none written",
  exit 0); core crates → `CORE CRATES: exit status 0`, 3 m 51 s here; disk (`df -h .`, `du -sh
  target` = 2.0G); the package-record step (here: `xvfb` and three Mesa packages present,
  `libxkbcommon-x11-0`, `libxcb-xkb1`, `libegl-mesa0` absent, exit 0); the probe step three ways:
  the real `kiln` with no display → `PROBE: exit status 2`, stderr "kiln: no window: winit
  EventLoopError …", step exit 2; `BUILD_OUTCOME=failure` → "PROBE: not run (cargo build -p kiln:
  failure)", exit 0; a fake `kiln` that hangs, with the limit cut to 3 s in a copy → killed after
  3.0 s, "timed out", exit 143, no process left; a fake whose window form runs the real `kiln
  script` → exit 0 and three response lines (the three-line script is valid). The summary grep of
  the full-test-output step was fixed after `rustc` 1.97 showed panic lines carry a thread id
  (`thread 'w' (6944) panicked at …`). `sh … | tee` under `pipefail` keeps a failing exit status
  (checked with `sh -c 'exit 1' | tee`). (4) AC-2's effect on a toy repository (one LF, one CRLF
  file; clone with `-c core.autocrlf=true`): without `.gitattributes` the LF file differs from its
  blob; with it both are byte-equal; `git check-attr text` on `crlf_lines.rs.txt` = `unset`.
  · Not run here: the window procedure step (the file is the test author's and not on this branch;
  only its command line is mine); the full-test-output step (it is `cargo test --workspace` again);
  anything on a hosted runner: triggers, `paths-ignore` on a new `ci/` branch, uploads, any macOS or
  Windows result, wall times, disk. `docs/development.md` states no measured runner figure.
  · QUESTION (a): the colour of a `window-probe` leg is not pinned. I made the probe step exit with
  `kiln`'s exit status (a timeout shows as 143), so a leg is green only when the window form
  ran the script and exited 0, and red otherwise; the record is uploaded either way. Is that the
  intended colour (it decides when a leg can become counted)?
  · QUESTION (b): Interface (1)'s last bullet lists `window-linux` among the jobs that count only
  from a decision row, while "Before queueing" says Linux counts and AC-4 needs `window-linux` green
  on the candidate. I wrote it as tracking until its row (Interface (1)). Which is it?
  · QUESTION (c): the procedure fetches the two libraries with `apt-get download`, which needs the
  runner's package lists. If round 1 shows that fails, may the job run `sudo apt-get update` (no
  install; the runner's own mirror, Pinned (e)) before the procedure? I added nothing for it yet;
  the package-record step prints how many files `/var/lib/apt/lists` holds.
  · gate attempts: 1 (`cargo xtask ci --keep-going --report …` through the workflow's own step, started
  at accc222; the docs line of 8780a48 was written while it ran), PASSED in 581.8 s; final attempt after this entry below ·
  missing context: none · next: director, CI round 1 on the head commit named in my report.
  · gate attempt 2: `cargo xtask ci` at 6f469df37a387996dbcb243c612e39f60fd8a7f0, `GATE PASSED in 489.1s`
  (11 steps, fuzz and budgets EMPTY). Attempts in all: 2, both green.
  · CI-REQUEST: 6f469df37a387996dbcb243c612e39f60fd8a7f0 (the gated commit; the commit after it adds
  only these two log lines). Expected: `gate-linux` green (prints the three versions; `GATE PASSED`;
  core crates exit 0; disk line); `window-linux` green only if `apt-get download` works on the runner
  and Xvfb + Mesa open the window: the artefact should show the six `CASE` lines, `WINDOW CASES
  PASS`, `AC-3 PASS` or `AC-3 PRECONDITION FAILED`, `PLAIN PASS`; if the procedure file is not on the
  pushed commit (it is the test author's), the step fails at `sh` and the job is red — that round
  then only shows the package record; `gate-macos` unknown (records which gate steps fail);
  `gate-windows` red at `clippy` and later steps, core-crates log with `test result: ok` for
  kiln-text, kiln-syntax, kiln-search and compile errors for xtask; `window-probe` macOS: one PROBE
  exit line with stderr (unknown); Windows: red at "Build kiln", record "PROBE: not run". Lines to
  copy: each job's `GATE PASSED|GATE FAILED` line and the report's `failed_step`, `CORE CRATES:`,
  the `test result:` lines of core-crates, `DISK:` with the two figures, `COMMAND`/`PACKAGE`/`APT
  LISTS`/`LDCONFIG` lines, every `PROBE` line and the first 5 lines of stderr, the procedure's
  verdict lines, and the wall time of every job.
- 2026-11-15 · R-266 · rework · rulings of D-182 acknowledged, all accepted: m1 (one `cargo test
  --locked -p <package>` per core package inside the one step, each appending to `core-crates.txt`;
  the step still ends with exit 0); m2 (the test author's: the procedure reads `kiln`'s own status;
  nothing of mine changes); m3 (`fail-fast: false` stays; every upload of the window jobs runs
  whatever the procedure's or probe's result); m4 (`window-linux` runs `sudo apt-get update` before
  the procedure, installs nothing); m5 (a probe leg's colour is `kiln`'s exit status: as built);
  m6 (`window-linux` tracked until its decision row; the docs stay); the test-output condition
  accepted; round 1 of CI runs on the joined branch, my CI-REQUEST for 6f469df is not used; disk
  rule (no build below 14 GB free; this rework runs no build and no whole gate, by the director's
  instruction).
  · did (53887cf, 3eb8c0c): m1 — the core-crates step of the three gate jobs (identical copies)
  loops over `kiln-text kiln-syntax kiln-search xtask`, one `cargo test --locked -p "$p"` each,
  appending to `core-crates.txt` between `==== cargo test --locked -p <p>` and `==== <p>: exit
  status <rc>` lines; the log line is `CORE CRATES: exit status per package: kiln-text=… xtask=…
  (reported, not required)`; `exit 0`. m3 — checked, no change needed: `window-probe` keeps
  `fail-fast: false`; the upload steps of `window-linux` and of `window-probe` have `if: always()`
  (the probe step also has `if: always()`, so "not run" is written when the build fails). m4 — new
  step "Refresh the runner's package lists (installs nothing)", `sudo apt-get update`, after the
  build and before the procedure. `docs/development.md`: the core-crates bullet (one run per
  package and why), the setup-step list gains the refresh, "Packages installed … none" gains the
  refresh sentence. m5, m6: no change (as built, as written).
  · re-checked: PyYAML parse plus my structure script (18 `paths-ignore` patterns equal to
  `ledger.toml`'s, triggers, `contents: read`, no `concurrency`/`continue-on-error`/`pull_request`,
  15 `uses:` all 40-hex, every job's label, `if:` and matrix, every step's `if:`); `actionlint`
  1.7.12 on `platforms.yml` and `ci.yml`: exit 0; the new core-crates script run with `bash
  --noprofile --norc -eo pipefail` against a fake `cargo` whose `xtask` run fails to compile with
  exit 101: the three others still run and report, the file holds all four sections, the summary
  reads `kiln-text=0 kiln-syntax=0 kiln-search=0 xtask=101`, step exit 0. Not run: the real `cargo
  test` per package (no build in this rework); `sudo apt-get update` (it would change this
  machine's package lists); no hosted runner. Whole gate not run (the director's instruction; the
  join runs it); `cargo xtask hygiene` on the committed state: result below.
  · `cargo xtask hygiene` at 365d0daed981f6c12363ad4dd20fc057fd7d1f19: PASS (reviews ok; session-paths: 590 tracked files
  clean) · gate attempts this run: 0 (by instruction) · missing context: none · next: director, join.
#### Rework after review round 1
- 2026-11-15 · R-277 · rework · rulings of D-193 acknowledged, all accepted: g2, g3, g7 are mine; g1, g4 the
  test author's (g4: I make the workflow meet what its tests will require, below); g5, g6 the director's
  (AC-6 read from the logs of `ci/T-070-negative`; T-077's `touches`); AC-4 read again in round 2. Read
  `reviews/T-070-r1.md` at 3c7399b and `tasks/ci-runs/T-070-round-1.md` on main.
  · g2 (0dd0b46): every recording step now begins `set +e` and ends `exit 0`: Disk (three gate jobs,
  identical), Gate report (three), the package record of `window-linux`; core crates and the full test
  output already did. Run with `bash --noprofile --norc -eo pipefail`: Disk normal 0; Disk with a `du`
  that exits 1: 0 (the reviewer's finding: 1 before); `df` and `du` both failing: 0; Gate report without a
  file: 0, with a `cat` that fails: 0; package record normal 0, with failing `ldconfig` and `dpkg-query`:
  0. A structure check of the parsed YAML: every `if: always()` `run:` step except the probe (whose
  status is `kiln`'s, m5) has `set +e` and ends `exit 0`.
  · g3 (23e57b4): `docs/development.md` "Runners, wall times and disk" replaces "Runners and wall times":
  the six wall times of run 43752760283 (candidate `cd1121b`) as the ruling gives them, the three `df -h
  .` lines and `du -sh target` figures as the extraction prints them, that the macOS and Windows gate
  figures are of red runs (macOS: 11 min 26 s of it the test-output step), that the window on macOS is
  still built, not observed (§8.1, ADR-0009 Q-5). "14 GB disk each" and "not measured yet" are gone;
  the CPU and memory figures stay only as GitHub's documented figures, which no run shows.
  · g7 (0dd0b46, 23e57b4): after SIGTERM at 120 s the watchdog checks `kiln` each second for 5 s and then
  sends `kill -9`, recording "PROBE: SIGTERM ignored for 5 s, sent SIGKILL". The docs' probe row says so.
  Run with the bound cut to 3 s in a copy (only that line changed): a `kiln` with `trap "" TERM` →
  SIGKILL, step exit 137 after 8 s; a plain hang → 143 after 4 s; normal exit 0 → 0 after 1 s; exit 2 →
  2; nothing left running after any of them.
  · g4, already true, no change needed (line numbers at 23e57b4):
  - procedure step not masked: `platforms.yml:340` `sh .github/scripts/window-procedure.sh target/debug/kiln
    "$RUNNER_TEMP/window" 2>&1 | tee window-procedure.txt` under `defaults.run.shell: bash` (`:52`), which
    Actions runs with `-eo pipefail`; with a fake procedure exiting 1 the step exits 1, exiting 0 → 0;
  - probe exits with `kiln`'s status: `:455` `exit "$rc"` (runs above: 137, 143, 0, 2);
  - probe runs whatever the build's result: `:399` `if: always()`; `:410` `if [ "$BUILD_OUTCOME" != success ]`
    records "PROBE: not run (cargo build -p kiln: failure)" and exits 0 (run: `BUILD_OUTCOME=failure`);
  - time bound as documented: `:425` `while [ "$i" -lt 120 ]`, one `sleep 1` per turn, docs "bounded to
    120 s (SIGTERM at 120 s, SIGKILL 5 s later …)";
  - profile: `:332` `cargo build -p kiln --locked` (dev profile) builds `target/debug/kiln`, which `:340` runs.
  No conflict with a ruling or the spec found.
  · re-checked: PyYAML parse plus my structure script (18 `paths-ignore` patterns equal to the ledger's, `contents:
  read`, no `concurrency` or `continue-on-error`, 15 `uses:` all 40-hex); `actionlint` 1.7.12 on both workflows:
  exit 0 (shellcheck still not available); `CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0 cargo
  test -p xtask --locked --test acceptance_t070 --test guard_t070`: 8 + 3 passed (the test author's g4 checks
  were not on this branch yet); `cargo xtask hygiene`: result below. Disk before the build: 19 GB free.
  · not checked here: anything on a hosted runner (round 2 runs on the re-join); `kill -9` of a native
  process from Git Bash on Windows and the SIGKILL path on macOS (Linux only here); the whole gate
  (not run: the change is to the workflow, its docs and this log; the director re-joins).
  · `cargo xtask hygiene` at 29e24bba296e4ae31034254340164eadf121929e: PASS (43 of 55 reviews checked, ok; session-paths: 594 tracked
  files clean) · gate attempts this run: 0 · missing context: none · next: test author (g1, g4), director re-join.
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
  · did (m8): the core-crates reader now splits a line into statements at `;`, `&&` and `&`
  outside quotes and counts a statement as a `cargo test` invocation only when its command word
  is `cargo test` (after `do`/`then`/`if`/`!`/`{`/`(` and assignments); an `echo`, `printf` or
  `:` line and quoted words invoke nothing. Every m1 requirement is unchanged. · self-check
  (scratch stub, deleted, never committed): 11 of 11 pass; 21 mutants as expected. Fail: one
  invocation naming all four packages; loop without `xtask`; no `||`; `>` not `>>`; no
  `--locked`; `set +e` without a final `true`; the implementer's step of the join without
  `exit 0`, with `>`, without `--locked`, without `xtask`; that step with its `cargo test` line
  removed (only the `echo` lines name it); test-output naming another step / `always()` / gate
  step without `id:`; fail-fast removed; either window upload without `if:`. Pass: four
  written-out lines; `set +e` + `tee -a` + `true`; the implementer's step of the join as given in
  D-183; test-output `failure()`. Clippy `-D warnings` clean with the stub. Committed state:
  `acceptance_t070` 3 passed, 8 failed by assertion (no workflow on this branch) · gate attempts:
  none of `cargo xtask ci` (not asked) · not run: the joined branch itself; anything on a hosted
  runner · next: director (join)
- 2026-11-15 · R-273 · ready · ruling m9 (D-189) acknowledged and accepted before acting: the three
  AC-3 tests move unchanged in what they assert to `xtask/tests/guard_t070.rs` (they pass on the
  base, the procedure file being mine); the eight workflow and `.gitattributes` tests stay in
  `acceptance_t070.rs`.
  · did (m9): moved the AC-3 section (`r251_block`, `R251_REPLACED` and the three tests) to the new
  `xtask/tests/guard_t070.rs`, byte-identical (diff of the section: none), with copies of the
  helpers it needs (`root`, `subject`, `oracle`, `scratch`, `Out`, `run_to`, `find_sh`,
  `sh_path`; identical except that `scratch` uses the sub-directory `t070-guard`, not `t070-acc`);
  removed them, `PROCEDURE`, `find_sh` and `sh_path` from `acceptance_t070.rs` and pointed its
  header at the guard file. Nothing else changed. · results on this branch (built with
  `CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0`, 8.8 GB free at start):
  `guard_t070` 3 passed; `acceptance_t070` 0 passed, 8 failed, each on `… does not exist (T-070
  adds it)` (6 the workflow, 2 `.gitattributes`); clippy `-D warnings` clean · gate attempts:
  none of `cargo xtask ci` (not asked) · not run: the joined branch; no stub re-run (the
  acceptance tests' code is unchanged, only deleted around) · next: director (join)
- 2026-11-15 · R-278 · ready · review round 1 (`reviews/T-070-r1.md` at 3c7399b) read; rulings of
  D-193 (read in the task file on main) acknowledged and accepted before acting: g1 (echoed lines
  of `kiln` are told apart from verdicts; the wrapper exits non-zero on any FAIL or NOT RUN verdict;
  forged cases in the guard tests) and g4 (five more workflow requirements in the acceptance tests)
  are mine; g2, g3, g7 are the implementer's; g5, g6 and the round-2 reading of AC-4 noted.
  · did, g1: the procedure defines `echo` before `procedure` (no R-251 line changed): it prints
  its text without reading escapes and indents every line after the first, so kiln's stderr,
  echoed by R-251's `window stderr:` and AC-3 `stderr:` lines, never starts a line; a verdict is a
  line at column 0. The wrapper also exits 1 when it read `WINDOW CASES FAIL`, `AC-3 FAIL`,
  `PLAIN FAIL: …` or `PLAIN NOT RUN: …`. Forged FAIL lines from kiln do not turn a passing run red
  (chosen: by g1 a kiln line is no verdict either way; a run that fails by itself stays red through
  its own FAIL). Guards added to `guard_t070.rs` (they run the file's header, its two R-251 echo
  lines and its wrapper around a stand-in body; no display, no kiln):
  `g1_forged_ac3_pass_in_kiln_stderr_is_not_a_verdict` (the reviewer's k4, with a literal `\n`,
  and with no AC-3 verdict of the procedure's own), `g1_forged_plain_pass_in_kiln_stderr_is_not_a_verdict`,
  `g1_forged_fail_lines_in_kiln_stderr_do_not_turn_a_passing_run_red`,
  `g1_any_fail_or_not_run_verdict_exits_1_even_beside_the_passes`. They fail on the R-273 file
  (all four) and on each part undone (no `echo`: 3; no continuation indent: 3; no FAIL rule: 1;
  NOT RUN not counted: 1). Under Xvfb with kiln built here: base exit 0; k4 (no-window stderr
  adds `AC-3 PASS`) exit 1, the line indented, `AC-3 FAIL`; forged `PLAIN PASS` while the plain
  window drops its path exit 1; forged FAIL/NOT RUN lines in every window case of a passing run
  exit 0; k1, k2, k3 exit 1 each.
  g4: five tests added to `acceptance_t070.rs`, each killing its mutant on a scratch stub (never
  committed): `ac1_window_linux_procedure_status_is_not_masked` (M9 `| tee … || true`; also
  `set +o pipefail`, `; true`, `; exit 0`, `shell: sh`), `ac1_window_probe_exits_with_kiln_status`
  (M10 `exit 0`; also no final exit), `ac1_window_probe_runs_after_a_failed_build_and_records_not_run`
  (M11 no `if:`; also `if: success()`, no read of `steps.<build id>.outcome`, no "not run", build
  step without `id:`), `ac1_window_probe_is_bounded_to_120_seconds` (M12 `120` → `1200`; a 120 in
  a message does not count), `ac1_window_jobs_build_the_profile_whose_binary_they_run` (M20
  `--release`; also `--target`, the probe built `--release` running `target/debug`). Pass as they
  should: `set -o pipefail`, `--profile dev`, `exit $rc`, the bound in a variable or in `env:`,
  `if: success() || failure()`; one acceptable form (`limit=120`) first failed, fixed (an
  assignment is not a message). Stub: 13 of 13 pass; clippy `-D warnings` clean. On this branch:
  `guard_t070` 7 passed; `acceptance_t070` 0 passed, 13 failed by assertion (11 the workflow, 2
  `.gitattributes`) · gate attempts: none of `cargo xtask ci` (not asked) · not run: the joined
  branch (the implementer's g2/g3/g7 rework); the earlier m1/m3 mutants were not re-run (their
  code is unchanged); anything on a hosted runner · next: director (re-join)
<!-- end test author log -->
### reviewer
- 2026-11-15 · R-274 · round 1 · request-changes · reviewed `bbbb0e52cd51497e33f3b045e3c067500f2343e7`
  (base 0bc14b8). Rulings read in main's task file and accepted: Q-1 A to Q-4 A (D-180), m1 to m6 (D-182),
  m7, m8 (D-183), m9 (D-189). I question none of them, but AC-6's Windows clause cannot be met while the
  workspace does not compile there (review finding 6). · did: workflow read line by line; ledger filter
  recomputed (18, equal); action shas checked by `git ls-remote` (3 of 3); every Linux-runnable `run:`
  script run under `bash -eo pipefail`, including failure paths; the procedure under Xvfb 5 of 5 exit 0,
  1 with k1/k2/k3, 0 with a wrapper exiting 5 (m2); its text against R-251 (only the named lines differ);
  `.gitattributes` on a toy repository and on an autocrlf clone (0 of 593 differ; base 587 of 588;
  `schema` and `syntax_commands_are_read_only` pass in the clone); 20 mutants (12 killed by gate tests,
  6 only by a procedure, 2 survived); the negative candidate `scratch/T-070-negative` at
  `4bc231c3755d583707b8031bdfecb4d93fbb628f`. · findings: 4 defects (1 in the procedure, 3 document
  sentences), 1 test gap, 2 spec gaps, 1 nit (`reviews/T-070-r1.md`). · CI-REQUEST:
  4bc231c3755d583707b8031bdfecb4d93fbb628f as `ci/T-070-negative` (AC-6; what to read is in the review)
  · not run: hosted runners; actionlint, shellcheck · next: director
- 2026-11-15 · R-281 · round 2 (final) · approve · reviewed `3fac206ea934707dd42607b320c4c29c361b215c`
  (previous `3c7399b`). Rulings g1 to g7 (D-193) read in main's task file and accepted. · did: the
  procedure under Xvfb (base 3 of 3 exit 0; k1, k2, k3, k4 and the new k5 each exit 1, with forged
  lines indented); recording steps with failing `df`/`du`/`cat`/`ldconfig`/`dpkg-query` all exit 0; probe
  watchdog (hang: 143 after 121 s; SIGTERM ignored: 137 after 125 s); the new doc figures checked
  against `tasks/ci-runs/T-070-round-1.md`; 14 mutants, all killed (10 by gate tests: 6 acceptance,
  4 guard; 4 only by a procedure). · round-1 findings: 1, 2, 3, 4, 5, 8 fixed; 6, 7 ruled (g5, g6).
  · new: 1 test gap and 2 nits, all follow-ups (`reviews/T-070-r2.md`), with what the director's two
  CI records should show · not run: the CI runs (the director's); actionlint, shellcheck · next: director
<!-- end reviewer log -->
