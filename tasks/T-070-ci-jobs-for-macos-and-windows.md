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
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
