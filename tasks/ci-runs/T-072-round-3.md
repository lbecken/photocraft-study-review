# CI run 44333697372 (platforms.yml) on ci/candidate @ bff0d7a38562d3ef1a0d6cb83fff781673b5040a — T-072 round 3

Record by hand; method as `tasks/ci-runs/README.md` (artefacts first; job logs for the image header, the cache step, the log-only lines and the disk step). Every value below comes from this run's job listing, its artefacts or its job logs. No judgement of results is made here; the reviewer compares them with the expectation written beforehand.
**Complete: the run had `status: completed` when first read (about 16:07Z); no waiting was needed. Nothing was still running at the end.**

## Run header
- id 44333697372, run_number 33, attempt 1, event push, workflow `platforms`, title "Join task/T-072-tests into task/T-072 (third join: the tests of rulin…" (as listed)
- branch `ci/candidate`; commit `bff0d7a38562d3ef1a0d6cb83fff781673b5040a` (equals the head asked for); created_at 2026-11-18T15:16:22Z; updated_at 2026-11-18T15:43:32Z
- status `completed`, run conclusion **failure** (cause by job: `gate-windows` failure; `window-probe (windows-2025)` failure; `gate-macos` cancelled; `window-probe (macos-15)` cancelled)

## Job table
Wall time = `completed_at` − `started_at` of the job (job listing).
| job | id | runner label | conclusion | wall | first failing step |
|---|---|---|---|---|---|
| gate-linux | 134904519508 | ubuntu-24.04 | success | 21m59s (15:16:26Z–15:38:25Z) | none |
| gate-macos | 134904518622 | macos-15 | **cancelled** | 15m01s (15:16:23Z–15:31:24Z) | no steps listed; `runner_id` 0, empty `runner_name` (`get_workflow_job`): no runner was assigned |
| gate-windows | 134904519366 | windows-2025 | **failure** | 27m07s (15:16:25Z–15:43:32Z) | Gate (15:16:55Z–15:25:18Z); every later step success or skipped |
| window-linux | 134904519325 | ubuntu-24.04 | success | 1m04s (15:16:25Z–15:17:29Z) | none |
| window-probe (macos-15) | 134904519191 | macos-15 | **cancelled** | 15m01s (15:16:23Z–15:31:24Z) | no steps listed; `runner_id` 0, empty `runner_name` (`get_workflow_job`): no runner was assigned |
| window-probe (windows-2025) | 134904519010 | windows-2025 | **failure** | 2m12s (15:16:25Z–15:18:37Z) | Window probe (15:18:30Z–15:18:32Z); Build kiln success |

Artefacts (`list_workflow_run_artifacts`; all four fetched with `download_workflow_run_artifact` and curl, size equal to `size_in_bytes`, `testzip()` clean; the scratch directory is removed after this record):
`gate-linux-44333697372` 31,538 B (id 14440619970); `gate-windows-44333697372` 24,573 B (14442342536); `window-linux-44333697372` 577 B (14438814961); `window-probe-windows-2025-44333697372` 344 B (14439883791). 4 artefacts in all: **no `gate-macos-…` and no `window-probe-macos-15-…` artefact exists.**
Job logs (`get_job_logs`, then the logs URL fetched with curl) were read for gate-linux and gate-windows. No log of a macOS job was read (no runner, no steps).

## Gate jobs

### gate-linux (job 134904519508)
- Runner image (job log "Runner Image"): `Image: ubuntu-24.04`, `Version: 20261111.370.1`; runner version 2.338.2.
- Rust versions listed by the cache step: `1.97.0 x86_64-unknown-linux-gnu 2d8144b7880597b6e6d3dfd63a9a9efae3f533d3` and `1.99.1 x86_64-unknown-linux-gnu 58cf8f9c35b355199727627d07ae34a4e2846d67`. Toolchain step printed `cargo 1.97.0 (c980f4866 2026-06-30)`, `clippy 0.1.97`, `rustfmt 1.9.0-stable`. Cache: `Cache hit for: v0-rust-gate-linux-Linux-x64-f37a6dc5-6187dc6e`, `Cache Size: ~358 MB`, `full match: true`.
- Steps (job listing):
| step | conclusion | start–end (Z) | duration |
|---|---|---|---|
| Set up job | success | 15:16:27–15:16:28 | 1s |
| Checkout | success | 15:16:28–15:16:29 | 1s |
| Install the pinned toolchain (rust-toolchain.toml) | success | 15:16:29–15:16:39 | 10s |
| Cache cargo registry and target | success | 15:16:39–15:16:46 | 7s |
| Gate | success | 15:16:46–15:31:37 | 14m51s |
| Gate report | success | 15:31:37–15:31:37 | 0s |
| Core crates (reported, not required) | success | 15:31:37–15:38:18 | 6m41s |
| Test list | success | 15:38:18–15:38:19 | 1s |
| Full test output (gate failed) | **skipped** | 15:38:19 | 0s |
| Disk | success | 15:38:19–15:38:20 | 1s |
| Upload reports | success | 15:38:20–15:38:21 | 1s |
| Post Cache, Post Checkout, Complete job | success, success, success | 15:38:21 | 0s each |
- Gate report (`gate-report.json`, 1,986 B): `passed: true`, `failed_step: null`, 886.5 s.
| step | status | seconds | detail |
|---|---|---|---|
| fmt | pass | 1.1 | formatted |
| clippy | pass | 12.8 | no warnings (all targets, -D warnings) |
| layers | pass | 0.6 | 11 crates, 14 internal edges, layer table holds |
| ui-commands | pass | 0.04 | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | pass | 0.09 | reviews: 60 of 72 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 695 tracked file(s) clean (0 not UTF-8, skipped) |
| schema | pass | 6.0 | 30 commands; audit ok; docs/commands.json current |
| scorecard | pass | 228.0 | 64 items: 47 verified, 0 implemented, 0 failing, 17 planned. |
| test | pass | 634.6 | 1175 passed, 0 failed, 0 ignored in 137 suites |
| no-skip | pass | 3.2 | self-test ok (17 known findings); 203 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (19 pending), today = 2026-11-18 |
| fuzz, budgets | empty | 0.0 | no targets registered yet (xtask/fuzz.toml, xtask/budgets.toml) |
- "Test list": 1 s. Log lines, verbatim: `TEST LIST: exit status 0` / `TEST LIST: the status is recorded, not used; the gate step decides the colour`.
- `test-list.txt` (76,803 B, 1,571 lines): 136 binary lines (128 `Running`, 8 `Doc-tests`), 1,174 lines ending `: test` (test names). Commands: `grep -c Running`, `grep -c Doc-tests`, `grep -c ': test$'` on the ANSI-stripped file.
- "Full test output (gate failed)": **skipped** (gate green). No `TEST OUTPUT:` line, nothing printed, no `test-output.txt`.
- Artefact files: `gate-report.json` 1,986 B; `core-crates.txt` 40,370 B; `test-list.txt` 76,803 B. Zip 31,538 B.
- Disk (job log): `Filesystem      Size  Used Avail Use% Mounted on`, `/dev/root       145G   65G   80G  45% /`; `DISK: du -sh target` → `4.5G	target`.
- `core-crates.txt`: labels `==== cargo test --locked --no-fail-fast -p kiln-text|kiln-syntax|kiln-search|xtask`; each ends `exit status 0`. kiln-text 78 passed, kiln-syntax 53, kiln-search 55, xtask 315; 0 failed in all.

### gate-windows (job 134904519366)
- Runner image: `Image: windows-2025-vs2026`, `Version: 20261103.298.1`; runner version 2.338.2. Step shell: `C:\Program Files\Git\bin\bash.EXE --noprofile --norc -e -o pipefail {0}`.
- Rust versions listed by the cache step: `1.97.0 x86_64-pc-windows-msvc 2d8144b7880597b6e6d3dfd63a9a9efae3f533d3` and `1.99.1 x86_64-pc-windows-msvc 58cf8f9c35b355199727627d07ae34a4e2846d67`. Toolchain step printed `cargo 1.97.0 (c980f4866 2026-06-30)`. Cache: `No cache found.`
- Steps:
| step | conclusion | start–end (Z) | duration |
|---|---|---|---|
| Set up job | success | 15:16:26–15:16:27 | 1s |
| Checkout | success | 15:16:27–15:16:36 | 9s |
| Install the pinned toolchain (rust-toolchain.toml) | success | 15:16:36–15:16:49 | 13s |
| Cache cargo registry and target | success | 15:16:49–15:16:55 | 6s |
| Gate | **failure** | 15:16:55–15:25:18 | 8m23s |
| Gate report | success | 15:25:18–15:25:19 | 1s |
| Core crates (reported, not required) | success | 15:25:19–15:43:01 | 17m42s |
| Workspace lib and T-073 tests (reported, not required) | success | 15:43:01–15:43:08 | 7s |
| Test list | success | 15:43:08–15:43:15 | 7s |
| kiln-workspace test targets (reported, not required) | success | 15:43:15–15:43:22 | 7s |
| Full test output (gate failed) | success (**ran**) | 15:43:22–15:43:24 | 2s |
| Disk | success | 15:43:24–15:43:27 | 3s |
| Upload reports | success | 15:43:27–15:43:28 | 1s |
| Post Cache | skipped | 15:43:28 | 0s |
| Post Checkout, Complete job | success, success | 15:43:28–15:43:31 | 3s, 0s |
- Gate report (`gate-report.json`, 17,274 B): `passed: false`, `failed_step: clippy`, 478.9 s.
| step | status | seconds | detail |
|---|---|---|---|
| fmt | pass | 2.4 | formatted |
| clippy | **fail** | 71.8 | kept text (60 lines) ends `error: could not compile `kiln-cli` (test "guard_t037_cli") due to 16 previous errors`; includes `error[E0599]: no method named `mode` found for struct `std::fs::Metadata`` at `apps\kiln-cli\tests\guard_t037_cli.rs:150:15` (also `uid`, `gid`, `from_mode`, `is_fifo` errors in the same file) |
| layers | pass | 12.8 | 11 crates, 14 internal edges, layer table holds |
| ui-commands | pass | 0.05 | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | pass | 0.2 | reviews: 60 of 72 …; session-paths: 695 tracked file(s) clean |
| schema | pass | 37.1 | 30 commands; audit ok; docs/commands.json current |
| scorecard | **fail** | 345.3 | first line `windows: 64 items; 158 tests ran; 3 registered absent here; page not compared (computed on Linux)`, then `60 scorecard problems:` (first: `building test executables failed on windows (); their tests are not run`); kept text 62 lines |
| test | **fail** | 3.2 | `0 passed, 0 failed, 0 ignored in 0 suites`; kept text (27 lines) ends `error: could not compile `kiln` (test "script_t055") due to 1 previous error`, `error: could not compile `kiln` (test "acceptance_t055") due to 2 previous errors` |
| no-skip | pass | 6.0 | self-test ok (17 known findings); 203 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (19 pending), today = 2026-11-18 |
| fuzz, budgets | empty | 0.0 | no targets registered yet (xtask/fuzz.toml, xtask/budgets.toml) |
- "Test list": 7 s. Log lines, verbatim: `TEST LIST: exit status 101` / `TEST LIST: the status is recorded, not used; the gate step decides the colour`.
- `test-list.txt` (9,109 B, 142 lines): compile errors only; **0 binary lines, 0 test names**; 3 `could not compile` lines (`window_t065` 8 errors, `acceptance_t065` 3, `script_t055` 1).
- "Full test output (gate failed)": **ran** (2 s). Log lines, verbatim: `TEST OUTPUT: exit status 101` / `TEST OUTPUT: the status is recorded, not used; the gate step decides the colour`. The printer printed only `error[…]` lines and `error: could not compile` lines. The four `could not compile` lines (log, verbatim): `error: could not compile `kiln` (test "window_t065") due to 8 previous errors`, `error: could not compile `kiln` (test "acceptance_t065") due to 3 previous errors`, `error: could not compile `kiln` (test "script_t055") due to 1 previous error`, `error: could not compile `kiln` (test "acceptance_t055") due to 2 previous errors`. The `error[…]` lines are `E0433 cannot find `unix` in `os``, `E0599 … `from_bytes` found for struct `OsStr``, `E0599 … `from_vec` found for struct `OsString``, `E0277 the size for values of type `[OsString]` cannot be known at compilation time`. `test-output.txt` (10,832 B, 172 lines): 0 lines `test … FAILED`, 0 `Running`/`test result:` lines. No failing test name and no panic message in it (nothing compiled to run).
- Artefact files: `gate-report.json` 17,274 B; `core-crates.txt` 40,754 B; `test-output.txt` 10,832 B; `test-list.txt` 9,109 B; `workspace-tests.txt` 7,134 B; `windows-tests.txt` 62,809 B. Zip 24,573 B.
- Disk (job log): `Filesystem      Size  Used Avail Use% Mounted on`, `D:              150G  6.6G  144G   5% /d`; `DISK: du -sh target` → `3.5G	target`.
- `core-crates.txt`: all four packages `exit status 0`; kiln-text 78, kiln-syntax 53, kiln-search 55, xtask 307 passed; 0 failed.

### gate-windows, T-073 step and the 14-target step
"Workspace lib and T-073 tests (reported, not required)" (`workspace-tests.txt`; log line `WORKSPACE TESTS: exit status 0 (reported, not required)`): lib `36 passed; 0 failed`, `acceptance_t073` `1 passed; 0 failed`, `guard_t073` `10 passed; 0 failed`: 47 passed, 0 failed. Two `MEASURED:` lines, verbatim:
- `MEASURED: acw2_root_parent_replaced_is_outside_root: windows: rename of p while the root is held: refused: kind PermissionDenied, raw OS error Some(5) (Access is denied. (os error 5))`
- `MEASURED: acw2_root_replaced_at_same_path_is_outside_root: windows: rename of root while the root is held: ok`

"kiln-workspace test targets (reported, not required)" (`windows-tests.txt`; 14 `==== cargo test … --test <target>` blocks), every status line verbatim:
```
==== acceptance_t002: exit status 101
==== acceptance_t002_r2: exit status 101
==== acceptance_t008: exit status 101
==== acceptance_t015_tools: exit status 101
==== acceptance_t019_workspace: exit status 101
==== acceptance_t036_workspace: exit status 101
==== acceptance_t073: exit status 0
==== confinement: exit status 101
==== guard_t008: exit status 101
==== guard_t036_removal_workspace: exit status 101
==== guard_t036_workspace: exit status 101
==== guard_t073: exit status 0
==== project: exit status 101
==== tools_contract: exit status 0
```
Summary line (job log, verbatim): `WINDOWS TESTS: exit status per target: acceptance_t002=101 acceptance_t002_r2=101 acceptance_t008=101 acceptance_t015_tools=101 acceptance_t019_workspace=101 acceptance_t036_workspace=101 acceptance_t073=0 confinement=101 guard_t008=101 guard_t036_removal_workspace=101 guard_t036_workspace=101 guard_t073=0 project=101 tools_contract=0 (reported, not required)`.

Non-zero targets:
- `acceptance_t002` (`running 15 tests`, `test result: FAILED. 14 passed; 1 failed`): `test disk_save_replaces_via_rename_without_leftovers ... FAILED`; `thread 'disk_save_replaces_via_rename_without_leftovers' (6860) panicked at crates\kiln-workspace\tests\acceptance_t002.rs:940:5:` / `precondition: permission bits and inode numbers are Unix metadata; the Windows checks of atomic replace are not written yet (ADR-0009: T-073 and T-074 write the Windows behaviour and its tests)`.
- `acceptance_t015_tools` (`running 3 tests`, `test result: FAILED. 2 passed; 1 failed`): `test no_tools_never_runs ... FAILED`; `thread 'no_tools_never_runs' (7696) panicked at crates\kiln-workspace\tests\acceptance_t015_tools.rs:131:5:` / `precondition: the fake `cargo`/`rustfmt` of this test are shell scripts and need Unix`.
- Compile errors (no test ran), closing lines: `acceptance_t002_r2` 11 errors; `acceptance_t008` 20; `acceptance_t019_workspace` 5; `acceptance_t036_workspace` 16; `confinement` 24; `guard_t008` 1; `guard_t036_removal_workspace` 3; `guard_t036_workspace` 1; `project` 3 (each `error: could not compile `kiln-workspace` (test "<target>") due to N previous error(s)`).
- Exit 0: `acceptance_t073` (1 passed), `guard_t073` (10 passed), `tools_contract` (5 passed).

## `xtask` tests (`core-crates.txt`, block `==== cargo test --locked --no-fail-fast -p xtask`)
Commands: ANSI-stripped file; `awk '/-p xtask$/{x=1} x&&/^test result/{p+=$4;f+=$6;n++} END{print p,f,n}'`; `awk '/-p xtask$/{x=1} x&&/Running/{r=$0} x&&/^test result/{print r" => "$0}'` for the per-binary lines; `grep -nE 'g1_probe|FAILED|panicked'`.
| platform | `==== xtask: exit status` | binaries | whole package | `acceptance_t072` | `guard_t072` | unit-test binary (`src/main.rs`) | failing tests |
|---|---|---|---|---|---|---|---|
| Linux | 0 | 25 | 315 passed, 0 failed | `20 passed; 0 failed`, finished in 0.64s | `8 passed; 0 failed`, finished in 125.24s | `138 passed; 0 failed`, 4.29s | none |
| macOS | not run: `gate-macos` cancelled, no runner, no artefact | – | – | – | – | – | – |
| Windows | 0 | 25 | 307 passed, 0 failed | `20 passed; 0 failed`, finished in 2.43s | `8 passed; 0 failed`, finished in 127.99s | `134 passed; 0 failed`, 106.92s | none |

`guard_t072::g1_probe_with_a_kiln_that_never_exits_ends_within_135_s_and_records_the_kill` (verbatim lines in `core-crates.txt`):
| platform | lines | binary time |
|---|---|---|
| Linux | `test g1_probe_with_a_kiln_that_never_exits_ends_within_135_s_and_records_the_kill has been running for over 60 seconds` (line 761) / `test g1_probe_with_a_kiln_that_never_exits_ends_within_135_s_and_records_the_kill ... ok` (line 762) | `guard_t072` binary `finished in 125.24s` (no per-test time in the file) |
| Windows | the same two lines (lines 768, 769) | `guard_t072` binary `finished in 127.99s` |
| macOS | not run | – |
`grep -nE '^test .* FAILED$'` finds no line in either `core-crates.txt`. The `acceptance_t072` binary lists 20 tests on Linux (`running 20 tests`); `guard_t072` lists 8 (`running 8 tests`).

`queue::tests::main_xtask_is_rebuilt_from_base_on_every_run`: Linux `ubuntu-24.04` 20261111.370.1: `test queue::tests::main_xtask_is_rebuilt_from_base_on_every_run ... ok` (`core-crates.txt` line 411). Windows `windows-2025-vs2026` 20261103.298.1: **absent** (no line with that name; the unit-test binary ran, 134 passed; 18 other `queue::tests::…` lines are present). macOS: not run.

## gate-macos step by step
Job 134904518622, runner label `macos-15`; `status: completed`, `conclusion: cancelled`; created and started 15:16:23Z, completed 15:31:24Z, wall **15m01s**. The job data has no `steps`, `runner_id` 0, empty `runner_name`: no runner was assigned and no step ran. No `gate-macos-44333697372` artefact; no log read. Image version, steps, gate report, `TEST LIST` line, `df` and `target/` size for macOS: none exist in this run.

## Window jobs
- `window-linux` (job 134904519325): success, 1m04s; all steps success (Build kiln 15:16:50Z–15:16:57Z; Window procedure (T-065) under Xvfb 15:17:03Z–15:17:25Z). `window-procedure.txt` (1,207 B) read; it contains `WINDOW CASES PASS` (cases ac2, ac2x3, dblclick, scroll100, ctrlq, noroot, each `exit window=N headless=N`, `lines … EQUAL`, `files EQUAL`), `AC-3: exit=2 stdout_bytes=0 stderr_lines=1 panicked=0 (pass: 2 0 1 0)` with stderr `kiln: no window: Library libxkbcommon-x11.so could not be loaded.`, `AC-3 PASS`, `missing script: exit=2 stdout_bytes=0 (pass: 2 0; 124 = a window opened)`, three `plain …: drawn_and_stable=yes still_running=yes` lines, `PLAIN PASS: the start path changes the screen`.
- `window-probe (windows-2025)` (job 134904519010): failure, 2m12s; Build kiln success (15:16:52Z–15:18:30Z), Window probe failure (15:18:30Z–15:18:32Z), Upload the probe's record success. `window-probe.txt` (265 B), verbatim:
```
PROBE OS: windows-2025 MINGW64_NT-10.0-26100 runnervmm7ud1 3.6.10-5a1665c8.x86_64 2026-09-28 10:40 UTC x86_64 Msys
PROBE: exit status 2
PROBE: stdout 0 bytes, 0 lines
--- stdout
--- stderr
kiln: no window: egui_glow: OpenGL: egui_glow requires opengl 2.0+.
--- end
```
- `window-probe (macos-15)` (job 134904519191): **cancelled**, 15m01s (created and started 15:16:23Z, completed 15:31:24Z); no `steps`, `runner_id` 0, empty `runner_name` (`get_workflow_job`): **no runner was assigned**; no step ran; no artefact, so no `PROBE:` line exists in any file read.

## Compared with round 2 (`T-072-round-2.md`, run 44315616697 attempt 1 on d87f2e8e66dd3b6f14b689fada4f33d0341b8c68)
| item | round 2 | round 3 |
|---|---|---|
| Run / head | 44315616697, run_number 30, d87f2e8 | 44333697372, run_number 33, bff0d7a |
| Run state | completed, `failure` | completed, `failure` |
| window-probe (macos-15) | cancelled 15m01s, no runner | cancelled 15m01s, no runner |
| gate-macos | success, 26m09s | **cancelled** 15m01s, no runner, no artefact |
| gate-linux | success 16m02s; `test` 1171 passed, 137 suites | success 21m59s; `test` 1175 passed, 0 failed, 137 suites |
| gate-windows | failure 24m19s; `failed_step: clippy` | failure 27m07s; `failed_step: clippy` |
| window-linux | success 52s | success 1m04s |
| window-probe (windows-2025) | failure 2m17s | failure 2m12s; `PROBE: exit status 2`, same stderr text as in round 2's attempt-2 artefact |
| Linux image | ubuntu-24.04 20261111.127.1 | ubuntu-24.04 20261111.370.1 |
| Windows image | windows-2025-vs2026 20261103.298.1 | the same |
| Rust versions (cache step) | 1.97.0 and 1.99.1 | the same |
| Cache | Linux full match, Windows none | the same |
| Linux Gate step / report | 10m34s / 630.4 s | 14m51s / 886.5 s |
| Windows Gate step / report | 7m33s / 429.3 s | 8m23s / 478.9 s |
| Linux `test-list.txt` | 76,556 B, 1,567 lines, 136 binaries, 1,170 names | 76,803 B, 1,571 lines, 136 binaries (128+8), 1,174 names |
| Windows `test-list.txt` | 10,832 B, 172 lines, 0 / 0 | 9,109 B, 142 lines, 0 / 0 |
| `TEST LIST: exit status` | Linux 0, Windows 101 | Linux 0, Windows 101 |
| Windows Full test output | ran 1s, `TEST OUTPUT: exit status 101` | ran 2s, `TEST OUTPUT: exit status 101`; `test-output.txt` 10,832 B |
| `xtask` whole package | Linux 311, macOS 306, Windows 303; all 0 failed | Linux 315, Windows 307; 0 failed; macOS not run |
| `acceptance_t072` | `17 passed; 0 failed` on all three | `20 passed; 0 failed` on Linux and Windows |
| `guard_t072` | Linux 7/0 (125.19 s), macOS 7/0 (135.01 s), Windows 7/0 (127.81 s) | Linux 8/0 (125.24 s), Windows 8/0 (127.99 s) |
| `g1_probe_…` | not recorded | `ok` on Linux and Windows |
| Failing tests (xtask) | none | none |
| `queue::tests::main_xtask_is_rebuilt_from_base_on_every_run` | Linux ok, macOS ok, Windows absent | Linux ok, Windows absent, macOS not run |
| Windows T-073 step | lib 36, t073 1, guard 10; 47 passed | the same |
| Windows 14 targets | 14 status lines | the same 14 names and statuses; failures of `acceptance_t002` and `acceptance_t015_tools` with the same messages (thread ids differ) |
| Windows clippy kept text | ends `kiln-cli` `acceptance_t023_cli` (1 error); also `guard_t019_cli` | ends `kiln-cli` `guard_t037_cli` (16 errors) |
| Windows scorecard first line | `windows: 64 items; 158 tests ran; 3 registered absent here …`, `60 scorecard problems:` | the same |
| hygiene (Linux) | 59 of 71 review files; 694 tracked files | 60 of 72 review files; 695 tracked files |
| Linux df / target | `/dev/mapper/cow-root  145G   62G   83G  43% /`, 4.5G | `/dev/root  145G  65G  80G  45% /`, 4.5G |
| Windows df / target | `D:  150G  6.6G  144G   5% /d`, 3.5G | the same |

## Not read
- `window-probe (macos-15)` and `gate-macos`: no steps, no runner, no artefact, no log read; the reason for the cancellation at 15:31:24Z is not shown by the job data (not determinable).
- Job logs of the window jobs; the ubuntu and Windows steps' own output beyond the lines quoted; the full text of clippy's, scorecard's and test's failing output on Windows beyond what the gate report keeps (60/62/27 lines).
- `xtask/platform-tests.toml` at bff0d7a and the source of `main_xtask_is_rebuilt_from_base_on_every_run` and of `guard_t072.rs`: not read, so why the first is absent on Windows is not determinable from this run's files.
- Why the Linux gate counts 1,175 passed while `test-list.txt` has 1,174 names: not explained by any file read. Why Windows `test-list.txt` has 3 `could not compile` lines and `test-output.txt` 4: not examined.
- Per-test results other than those named above (xtask `core-crates.txt` lines were counted, not read test by test).
- Test results on macOS: none exist in this run.
