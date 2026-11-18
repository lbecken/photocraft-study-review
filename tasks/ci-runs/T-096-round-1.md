# CI run 44327751679 (platforms.yml) on ci/candidate @ 194721dc63613b982076b91fe602c9122213f7a7 — T-096 round 1

Record by hand; method as `tasks/ci-runs/README.md` (artefacts first; job logs for the image header, the cache step and the disk step). Every value below comes from this run's job listing, its artefacts or its job logs. No judgement of results is made here; the reviewer compares them with the expectation written beforehand.
**Complete: the run is `completed` (conclusion failure) and every job has finished. Waiting needed: at the first reads (15:00Z–15:15Z) `gate-macos` was still running; it finished 15:19:55Z (read 15:23Z). Nothing was still running at the end.**

## Run header
- id 44327751679, run_number 32, attempt 1, event push, workflow `platforms`, title "Join task/T-096-tests into task/T-096"
- branch `ci/candidate`; commit `194721dc63613b982076b91fe602c9122213f7a7` (equals the head asked for); created_at 2026-11-18T14:45:22Z
- Run status when first read (15:00Z): in_progress; job listing at 15:15Z: `gate-macos` in progress (step "Core crates"), all other jobs completed. Final (read 15:23Z): `status` completed, run conclusion **failure** (cause by job: `gate-windows` failure; `window-probe (windows-2025)` failure; `window-probe (macos-15)` cancelled); `updated_at` 2026-11-18T15:19:56Z.

## Job table
Wall time = `completed_at` − `started_at` of the job (job listing).
| job | id | runner label | conclusion | wall | first failing step |
|---|---|---|---|---|---|
| gate-linux | 134883514010 | ubuntu-24.04 | success | 22m17s (14:45:24Z–15:07:41Z) | none |
| gate-windows | 134883514343 | windows-2025 | **failure** | 24m10s (14:45:25Z–15:09:35Z) | Gate (14:46:06Z–14:54:04Z, 7m58s); every later step success or skipped |
| gate-macos | 134883514027 | macos-15 | success | 24m29s (14:55:26Z–15:19:55Z; job created 14:45:22Z, so about 10 min queue before start, INFERRED) | none |
| window-linux | 134883513902 | ubuntu-24.04 | success | 2m21s (14:45:25Z–14:47:46Z) | none |
| window-probe (windows-2025) | 134883513587 | windows-2025 | **failure** | 2m12s (14:45:24Z–14:47:36Z) | Window probe (14:47:29Z–14:47:31Z); Build kiln success |
| window-probe (macos-15) | 134883514020 | macos-15 | **cancelled** | 15m01s (14:45:22Z–15:00:23Z) | no steps listed; no `runner_id`, no `runner_name` in the job data: no runner was assigned |

## Artefacts
(`list_workflow_run_artifacts`; fetched with `download_workflow_run_artifact` and curl; size equal to `size_in_bytes`, `testzip()` clean; scratch directory removed after the record.)
`gate-linux-44327751679` 10,527 B (id 14440146171); `gate-windows-44327751679` 16,346 B (14440605601); `window-linux-44327751679` 577 B (14437372845, listed, not downloaded); `window-probe-windows-2025-44327751679` 344 B (14436754457). `gate-macos-44327751679` 10,332 B (id 14440043153; created 15:19:51Z; files `core-crates.txt` 37,742 B, `gate-report.json` 2,021 B; no `test-list.txt`, no `test-output.txt`). Listed at 15:23Z: 5 artefacts; **no `window-probe-macos-15-…` artefact**.
Files: gate-linux: `gate-report.json` 2,002 B, `core-crates.txt` 38,595 B (**no `test-list.txt`, no `test-output.txt`**; this run has no "Test list" step). gate-windows: `gate-report.json` 16,993 B, `core-crates.txt` 38,435 B, `test-output.txt` 10,832 B, `workspace-tests.txt` 7,134 B (**no `test-list.txt`, no `windows-tests.txt`**; no "Test list" and no "kiln-workspace test targets" step in this run). window-probe-windows: `window-probe.txt`.
Job logs (`get_job_logs`, logs URL fetched with curl) were read for gate-linux, gate-windows and gate-macos.

## Gate jobs, common facts

### gate-linux (job 134883514010)
- Runner image (job log "Runner Image"): `Image: ubuntu-24.04`, `Version: 20261116.376.1`; runner version 2.338.2.
- Rust versions listed by the cache step ("Rust Versions"): `1.100.0 x86_64-unknown-linux-gnu 0ef1bc386059596dada8fb65b50425aab2ea637a` and `1.97.0 x86_64-unknown-linux-gnu 2d8144b7880597b6e6d3dfd63a9a9efae3f533d3`. Toolchain step printed `cargo 1.97.0 (c980f4866 2026-06-30)`. Cache: `No cache found.` (log, after "Restoring cache").
- Steps (job listing):
| step | conclusion | start–end (Z) | duration |
|---|---|---|---|
| Set up job | success | 14:45:25–14:45:26 | 1s |
| Checkout | success | 14:45:26–14:45:27 | 1s |
| Install the pinned toolchain (rust-toolchain.toml) | success | 14:45:27–14:45:38 | 11s |
| Cache cargo registry and target | success | 14:45:38–14:45:44 | 6s |
| Gate | success | 14:45:44–15:02:03 | 16m19s |
| Gate report | success | 15:02:03–15:02:03 | 0s |
| Core crates (reported, not required) | success | 15:02:03–15:07:21 | 5m18s |
| Full test output (gate failed) | **skipped** | 15:07:21 | 0s |
| Disk | success | 15:07:21 | 0s |
| Upload reports | success | 15:07:21–15:07:22 | 1s |
| Post Cache, Post Checkout, Complete job | success, success, success | 15:07:22–15:07:40 | 18s, 0s, 0s |
- Gate report (`gate-report.json`): `passed: true`, `failed_step: null`, 965.77 s.
| step | status | seconds | detail |
|---|---|---|---|
| fmt | pass | 1.15 | formatted |
| clippy | pass | 53.04 | no warnings (all targets, -D warnings) |
| layers | pass | 1.20 | 11 crates, 14 internal edges, layer table holds |
| ui-commands | pass | 0.04 | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | pass | 0.09 | reviews: 59 of 71 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 697 tracked file(s) clean (0 not UTF-8, skipped) |
| schema | pass | 22.95 | 30 commands; audit ok; docs/commands.json current |
| scorecard | pass | 296.83 | 64 items: 47 verified, 0 implemented, 0 failing, 17 planned. |
| test | pass | 586.87 | 1148 passed, 0 failed, 0 ignored in 136 suites |
| no-skip | pass | 3.59 | self-test ok (17 known findings); 202 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (19 pending), today = 2026-11-18 |
| fuzz, budgets | empty | 0.0 | no targets registered yet (xtask/fuzz.toml, xtask/budgets.toml) |
- Disk (job log): `/dev/root       145G   65G   80G  45% /`; `4.5G	target`.
- `core-crates.txt` labels: `==== cargo test --locked -p kiln-text|kiln-syntax|kiln-search|xtask` (no `--no-fail-fast` in this run's labels); each ends `exit status 0`. Totals by package (sum of `test result:` lines): kiln-text 78 passed, kiln-syntax 53, kiln-search 55, xtask 288; 0 failed in all.

### gate-windows (job 134883514343)
- Runner image: `Image: windows-2025-vs2026`, `Version: 20261116.315.1`; runner version 2.338.2.
- Rust versions listed by the cache step: `1.100.0 x86_64-pc-windows-msvc 0ef1bc386059596dada8fb65b50425aab2ea637a` and `1.97.0 x86_64-pc-windows-msvc 2d8144b7880597b6e6d3dfd63a9a9efae3f533d3`. Toolchain step printed `cargo 1.97.0 (c980f4866 2026-06-30)`. Cache: `No cache found.`
- Steps:
| step | conclusion | start–end (Z) | duration |
|---|---|---|---|
| Set up job | success | 14:45:26–14:45:28 | 2s |
| Checkout | success | 14:45:28–14:45:36 | 8s |
| Install the pinned toolchain | success | 14:45:36–14:45:59 | 23s |
| Cache cargo registry and target | success | 14:45:59–14:46:06 | 7s |
| Gate | **failure** | 14:46:06–14:54:04 | 7m58s |
| Gate report | success | 14:54:04–14:54:05 | 1s |
| Core crates (reported, not required) | success | 14:54:05–15:09:17 | 15m12s |
| Workspace lib and T-073 tests (reported, not required) | success | 15:09:17–15:09:24 | 7s |
| Full test output (gate failed) | success (**ran**) | 15:09:24–15:09:25 | 1s |
| Disk | success | 15:09:25–15:09:29 | 4s |
| Upload reports | success | 15:09:29–15:09:30 | 1s |
| Post Cache | skipped | 15:09:30 | 0s |
| Post Checkout, Complete job | success, success | 15:09:30–15:09:32 | 2s, 0s |
- Gate report (`gate-report.json`): `passed: false`, `failed_step: clippy`, 454.86 s.
| step | status | seconds | detail |
|---|---|---|---|
| fmt | pass | 5.71 | formatted |
| clippy | **fail** | 63.23 | kept text 60 lines; first error shown `error[E0599]: no method named `mode` found for struct `std::fs::Metadata`` at `apps\kiln-cli\tests\guard_t037_cli.rs:150:15`; last lines `error: could not compile `kiln-cli` (test "guard_t037_cli") due to 16 previous errors`, `warning: build failed, waiting for other jobs to finish...` |
| layers | pass | 11.94 | 11 crates, 14 internal edges, layer table holds |
| ui-commands | pass | 0.05 | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | pass | 0.20 | reviews: 59 of 71 …; session-paths: 697 tracked file(s) clean |
| schema | pass | 34.36 | 30 commands; audit ok; docs/commands.json current |
| scorecard | **fail** | 332.92 | first line `windows: 64 items; 158 tests ran; 3 registered absent here; page not compared (computed on Linux)`, then `60 scorecard problems:` (first: `building test executables failed on windows (); their tests are not run`) |
| test | **fail** | 1.16 | `0 passed, 0 failed, 0 ignored in 0 suites`; kept text 27 lines, ends `error: could not compile `kiln` (test "script_t055") due to 1 previous error`, `error: could not compile `kiln` (test "acceptance_t055") due to 2 previous errors` |
| no-skip | pass | 5.30 | self-test ok (17 known findings); 202 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (19 pending), today = 2026-11-18 |
| fuzz, budgets | empty | 0.0 | no targets registered yet |
- "Full test output (gate failed)": **ran** (1 s). `test-output.txt` (10,832 B, 172 lines) holds only compile errors; its `could not compile` lines, verbatim: `error: could not compile `kiln` (test "window_t065") due to 8 previous errors`, `error: could not compile `kiln` (test "acceptance_t065") due to 3 previous errors`, `error: could not compile `kiln` (test "script_t055") due to 1 previous error`, `error: could not compile `kiln` (test "acceptance_t055") due to 2 previous errors`. 0 lines `test … FAILED`; no `Running` and no `test result:` line. No failing test name, no panic message (nothing compiled to run).
- "Workspace lib and T-073 tests" (`workspace-tests.txt`): lib `running 36 tests`, `36 passed; 0 failed`; `acceptance_t073` 1 passed; `guard_t073` 10 passed; 47 passed, 0 failed.
- Disk (job log): `D:              150G  6.6G  144G   5% /d`; `3.5G	target`.
- `core-crates.txt`: all four packages `exit status 0`. kiln-text 78, kiln-syntax 53, kiln-search 55, xtask 280 passed; 0 failed.

## `xtask` tests (`core-crates.txt`, block `==== cargo test --locked -p xtask`)
Commands: ANSI-stripped file; `awk '/-p xtask$/{x=1}x&&/^test result/{p+=$4;f+=$6;n++}END{print p,f,n}'`; `grep -nE` for the two named tests.
| platform | `==== xtask: exit status` | binaries (`test result:` lines) | whole package | unit-test binary (`src/main.rs`) | failing tests |
|---|---|---|---|---|---|
| Linux | 0 | 24 | 288 passed, 0 failed | `138 passed; 0 failed`, 6.14 s | none |
| Windows | 0 | 24 | 280 passed, 0 failed | `134 passed; 0 failed`, 100.78 s | none |
| macOS | 0 | 24 | 283 passed, 0 failed | `136 passed; 0 failed`, 23.85 s | none |

### `queue::tests::main_xtask_is_rebuilt_from_base_on_every_run` (verbatim result line per platform)
| platform | image version (job log) | result line |
|---|---|---|
| Linux | `ubuntu-24.04` 20261116.376.1 | `test queue::tests::main_xtask_is_rebuilt_from_base_on_every_run ... ok` (`core-crates.txt` line 423) |
| Windows | `windows-2025-vs2026` 20261116.315.1 | **absent**: no line with that name in `core-crates.txt`; the unit-test binary ran (`Running unittests src\main.rs`, `test result: ok. 134 passed; 0 failed`); other `queue::tests::…` lines are present (18 in all; e.g. `test queue::tests::main_history_tolerates_only_ledger_commits ... ok`) |
| macOS | `macos-15-arm64` 20261104.0514.1 | `test queue::tests::main_xtask_is_rebuilt_from_base_on_every_run ... ok` (`core-crates.txt` line 410) |

### `rebuild_from_base_unit_test_passes_when_the_default_toolchain_cannot_build` (binary `acceptance_t096`, `tests/acceptance_t096.rs`)
| platform | lines, verbatim | duration |
|---|---|---|
| Linux | `Running tests/acceptance_t096.rs (target/debug/deps/acceptance_t096-12649863bb950568)` / `running 1 test` / `test rebuild_from_base_unit_test_passes_when_the_default_toolchain_cannot_build ... ok` / `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 13.01s` | binary `finished in 13.01s` (no per-test time in the file) |
| Windows | `Running tests\acceptance_t096.rs (target\debug\deps\acceptance_t096-eb9b714698c595bc.exe)` / `running 1 test` / `test rebuild_from_base_unit_test_passes_when_the_default_toolchain_cannot_build ... ok` / `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.15s` | binary `finished in 20.15s` |
| macOS | `Running tests/acceptance_t096.rs (target/debug/deps/acceptance_t096-a555c74f068e4d8f)` / `running 1 test` / `test rebuild_from_base_unit_test_passes_when_the_default_toolchain_cannot_build ... ok` / `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 19.05s` | binary `finished in 19.05s` |

Failing tests by name in `core-crates.txt` of Linux, Windows and macOS: none (`grep -E '^test .* FAILED$'` finds no line in any). On Windows the gate itself failed (compile errors, see above), no test failed by name.

## Window jobs
- `window-linux`: success, 2m21s (steps all success: Window procedure (T-065) under Xvfb 14:47:03Z–14:47:41Z). Artefact 577 B listed, not downloaded.
- `window-probe (windows-2025)`: failure, 2m12s; step Window probe failed, Build kiln success. `window-probe.txt`, verbatim: `PROBE OS: windows-2025 MINGW64_NT-10.0-26100 runnervmm7ud1 3.6.10-5a1665c8.x86_64 2026-09-28 10:40 UTC x86_64 Msys` / `PROBE: exit status 2` / `PROBE: stdout 0 bytes, 0 lines` / stderr `kiln: no window: egui_glow: OpenGL: egui_glow requires opengl 2.0+.`
- `window-probe (macos-15)` (job 134883514020): **cancelled**, 15m01s (created and started 14:45:22Z, completed 15:00:23Z); the job data has no `steps`, no `runner_id`, no `runner_name`: no runner was assigned. No artefact.

## gate-macos (job 134883514027, runner label macos-15)
- Runner image (job log): `Image: macos-15-arm64`, `Version: 20261104.0514.1`.
- Rust versions listed by the cache step: `1.97.0 aarch64-apple-darwin 2d8144b7880597b6e6d3dfd63a9a9efae3f533d3` and `1.99.1 aarch64-apple-darwin 58cf8f9c35b355199727627d07ae34a4e2846d67`. Toolchain step printed `cargo 1.97.0 (c980f4866 2026-06-30)`. Cache: `Cache hit for: v0-rust-gate-macos-Darwin-arm64-03a9c7df-6187dc6e`, `full match: true`.
- Completeness: job `status: completed`, `conclusion: success`; all steps completed; artefact present (10,332 B).
| step | conclusion | start | end | duration |
|---|---|---|---|---|
| Set up job | success | 14:55:27Z | 14:55:28Z | 1s |
| Checkout | success | 14:55:28Z | 14:55:30Z | 2s |
| Install the pinned toolchain (rust-toolchain.toml) | success | 14:55:31Z | 14:55:40Z | 9s |
| Cache cargo registry and target | success | 14:55:40Z | 14:55:53Z | 13s |
| Gate | success | 14:55:53Z | 15:12:18Z | 16m25s |
| Gate report | success | 15:12:18Z | 15:12:18Z | 0s |
| Core crates (reported, not required) | success | 15:12:18Z | 15:19:49Z | 7m31s |
| Full test output (gate failed) | skipped | 15:19:49Z | 15:19:49Z | 0s |
| Disk | success | 15:19:49Z | 15:19:50Z | 1s |
| Upload reports | success | 15:19:50Z | 15:19:51Z | 1s |
| Post Cache cargo registry and target | success | 15:19:51Z | 15:19:51Z | 0s |
| Post Checkout | success | 15:19:51Z | 15:19:52Z | 1s |
| Complete job | success | 15:19:52Z | 15:19:54Z | 2s |
- Gate report (`gate-report.json`, 2,021 B): `passed: true`, `failed_step: null`, 974.34 s.
| step | status | seconds | detail |
|---|---|---|---|
| fmt | pass | 1.50 | formatted |
| clippy | pass | 15.21 | no warnings (all targets, -D warnings) |
| layers | pass | 2.71 | 11 crates, 14 internal edges, layer table holds |
| ui-commands | pass | 0.03 | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | pass | 0.27 | reviews: 59 of 71 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 697 tracked file(s) clean (0 not UTF-8, skipped) |
| schema | pass | 7.07 | 30 commands; audit ok; docs/commands.json current |
| scorecard | pass | 233.09 | macos: 64 items; 216 tests ran; 3 registered absent here; page not compared (computed on Linux) |
| test | pass | 711.01 | 1131 passed, 0 failed, 0 ignored in 136 suites |
| no-skip | pass | 3.44 | self-test ok (17 known findings); 202 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (19 pending), today = 2026-11-18 |
| fuzz, budgets | empty | 0.0 | no targets registered yet |
- "Full test output (gate failed)": skipped; no `test-output.txt`. No "Test list" step in this run, no `test-list.txt`.
- Disk (job log): `/dev/disk2s5   320Gi   260Gi    42Gi    87%    2.3M  437M    1%   /System/Volumes/Data`; `3.2G	target`.
- `core-crates.txt`: all four packages `exit status 0`; kiln-text 78, kiln-syntax 53, kiln-search 55, xtask 283 passed; 0 failed. Named tests: see the `xtask` section above.
- macOS evidence for the two named tests: `queue::tests::main_xtask_is_rebuilt_from_base_on_every_run` ok (line 410), `rebuild_from_base_unit_test_passes_when_the_default_toolchain_cannot_build` ok (line 674).

## Not read
- The contents of `window-linux-44327751679` (577 B): listed, not downloaded.
- `window-probe (macos-15)`: no steps, no runner, no artefact, no log read; the reason for the cancellation at 15:00:23Z is not shown by the job data (not determinable).
- Job logs beyond the image header, Rust versions, cache and disk lines (the macOS log's runner version line and the Linux/Windows steps' own output were not read); the full text of clippy's, scorecard's and test's failing output on Windows beyond what the gate report keeps (60/62/27 lines).
- `xtask/platform-tests.toml` at 194721d and the source of `main_xtask_is_rebuilt_from_base_on_every_run` and of `acceptance_t096.rs` were not read, so why the first is absent on Windows is not determinable from this run's files.
- Per-test results of the Linux and macOS `test` step beyond `core-crates.txt` (no `test-output.txt`, no `test-list.txt`): not determinable. No test of `kiln-workspace` was run on Linux or macOS in a report file (the gate's counts only).
- No comparison with a previous record was made.

## Re-run of the failed and cancelled jobs (attempt 2)
Added after the record above. The director queued a re-run at about 15:50Z (the run's `run_started_at` for attempt 2 is 2026-11-18T15:48:51Z) because `window-probe (macos-15)` had been cancelled with no runner assigned. Every value comes from the run and job listings (read 16:07Z–16:15Z), `get_workflow_job` and the artefact listing. The sections above describe attempt 1 and are unchanged.

Run 44327751679 now: `run_attempt` 2, `status` completed, `conclusion` **failure**, `updated_at` 2026-11-18T16:13:28Z. Cause by job: `gate-windows` failure and `window-probe (windows-2025)` failure, as in attempt 1; `window-probe (macos-15)` cancelled again. It was still `in_progress` (gate-windows in step "Core crates") when first read at about 16:07Z; I waited until it completed (16:13:28Z).

Jobs re-run in attempt 2 (wall = `completed_at` − `started_at`):
| job | id | conclusion | wall | note |
|---|---|---|---|---|
| gate-windows | 134926864627 | **failure** | 24m31s (15:48:57Z–16:13:28Z) | Gate failure 15:49:27Z–15:57:32Z (8m05s); Gate report, Core crates (15:57:33Z–16:13:12Z, 15m39s), Workspace lib and T-073 tests, Full test output (ran, 1s), Disk, Upload reports all success; Post Cache skipped |
| window-probe (windows-2025) | 134926864700 | **failure** | 1m57s (15:48:57Z–15:50:54Z) | Build kiln success 15:49:26Z–15:50:47Z; Window probe failure 15:50:47Z–15:50:48Z; Upload the probe's record success |
| window-probe (macos-15) | 134926864937 | **cancelled** | 15m01s (15:48:54Z–16:03:55Z) | see below |
The other three jobs (gate-linux, gate-macos, window-linux) were not re-run: the listing shows them under `run_attempt` 2 with their attempt-1 times and conclusions (success, success, success; job ids 134926868214, 134926866650, 134926964914; started 14:45:24Z, 14:55:26Z, 14:45:25Z).

`window-probe (macos-15)` in attempt 2 (job 134926864937, label `macos-15`): created and started 15:48:54Z, completed 16:03:55Z, 15m01s, `conclusion` cancelled. `get_workflow_job`: `runner_id` 0, `runner_name` empty, `runner_group_name` empty, no `steps` key: **no runner was assigned**. Steps with conclusion and duration: **none; no step ran** (nothing to list). The probe's exit-status line: **does not exist**; the `PROBE: exit status N` line is in no file or log read. Artefact file list: **none**; the artefact listing of the run (read 16:15Z, 7 artefacts) has no `window-probe-macos-15-…` artefact. The job log was not read.

Artefacts of the run after attempt 2 (`list_workflow_run_artifacts`): `gate-windows-44327751679` id 14445287288, 16,338 B, created 16:13:24Z (attempt 2; attempt 1's copy id 14440605601, 16,346 B); `window-probe-windows-2025-44327751679` id 14442608516, 344 B, created 15:50:49Z (attempt 2; attempt 1's copy id 14436754457); `gate-linux-44327751679` 10,527 B; `gate-macos-44327751679` 10,332 B; `window-linux-44327751679` 577 B. The contents of attempt 2's `gate-windows` and `window-probe-windows-2025` artefacts were **not read**; the Windows probe's `PROBE:` line and attempt 2's gate result beyond the job's steps above are not read.

## Main after the merge
Added after the record above. The runs on `main` for the push that carried T-096's merge: pushed head `2e1861d0940fcef405cc4a115881122724663336` ("ledger: T-096 merged (D-263); T-072 review round 2 approves with the CI criterion open (R-377)", created 2026-11-18T16:43:06Z). That it contains the merge head `aed3d887a658a72a4f7c874269a4a95451b5988b` was given in the brief and not checked here (no commit listing was read). Every value comes from the run and job listings (`actions_list`, `actions_get get_workflow_job`), the `gate-windows-44350449897` artefact (`gate-report.json`, read) and one job log (`get_job_logs`, curl). Read about 17:10Z–17:20Z.

### `ci.yml` run 44350449912 (run_number 553, push, main @ 2e1861d)
- Status `completed`, conclusion **success**. One job `gate` (job 134964310648, `ubuntu-24.04`, runner 1000001023): success, 16:43:09Z–16:52:31Z (9m22s); Gate step success 16:44:01Z–16:52:27Z.
- Image (job log): `Image: ubuntu-24.04`, `Version: 20261111.127.1`.
- Gate summary lines in the log: `PASS  test          332.6s  1148 passed, 0 failed, 0 ignored in 136 suites`; `GATE PASSED in 502.9s (11 steps; 2 step(s) EMPTY (nothing registered): fuzz, budgets)`; hygiene `reviews: 60 of 72 review file(s) checked …; session-paths: 702 tracked file(s) clean`.
- `queue::tests::main_xtask_is_rebuilt_from_base_on_every_run`: **not shown** by this job's output (the log has per-step summary lines only; `grep -c main_xtask` on the log finds 0 lines); no artefact exists for `ci.yml`. Result: not determinable from this run.

### `platforms.yml` run 44350449897 (run_number 35, push, main @ 2e1861d)
- Status `completed`, conclusion **failure** (created 16:43:06Z, updated 17:12:25Z). Cause by job: `gate-windows` failure and `window-probe (windows-2025)` failure.
- **macOS runner: assigned** for both macOS jobs (`gate-macos` `runner_id` 1000001042, `window-probe (macos-15)` `runner_id` 1000001043; both ran steps; both created 16:43:07Z, started 16:48:15Z and 16:48:39Z).
| job | id | conclusion | wall (started–completed) | note |
|---|---|---|---|---|
| gate-linux | 134964313234 | **skipped** | 16:43:07Z–16:43:07Z (0s) | no runner, no steps, no artefact; the reason for the skip is not shown by the job data (not determinable) |
| gate-macos | 134964311357 | success | 24m10s (16:48:15Z–17:12:25Z) | runner 1000001042; Gate 16:48:42Z–17:04:29Z success |
| gate-windows | 134964311484 | **failure** | 18m23s (16:43:09Z–17:01:32Z) | see below |
| window-linux | 134964311078 | success | 1m16s (16:43:09Z–16:44:25Z) | |
| window-probe (macos-15) | 134964311411 | success | 39s (16:48:39Z–16:49:18Z) | runner 1000001043 |
| window-probe (windows-2025) | 134964311410 | **failure** | 2m11s (16:43:09Z–16:45:20Z) | Window probe step failure (16:45:14Z–16:45:15Z); Build kiln success |
- `gate-windows` (runner 1000001028): the failing step is **Gate** (16:43:39Z–16:49:47Z, 6m08s); Gate report, Core crates, Workspace lib and T-073 tests, Full test output (ran, 1s), Disk, Upload reports success; Post Cache skipped. `gate-report.json` (16,408 B): `passed: false`, `failed_step: clippy`, 349.5 s; failing gate steps `clippy` (50.9 s), `scorecard` (251.2 s; first line `windows: 64 items; 158 tests ran; 3 registered absent here; page not compared (computed on Linux)`), `test` (4.3 s; `0 passed, 0 failed, 0 ignored in 0 suites`); passing: fmt, layers, ui-commands, hygiene, schema, no-skip. The artefact (16,524 B, id 14449861523) holds `core-crates.txt`, `workspace-tests.txt`, `test-output.txt`, `gate-report.json`; `core-crates.txt` ends each package with `exit status 0`; the name `main_xtask_is_rebuilt_from_base_on_every_run` does not occur in it.
- Artefacts of the run: `gate-windows-44350449897` 16,524 B, `window-probe-macos-15-44350449897` 704 B, `window-probe-windows-2025-44350449897` 344 B, `window-linux-44350449897` 577 B; there is no `gate-linux-…` artefact (job skipped) and the `gate-macos-…` artefact was not listed when the run's artefacts were read (the macOS job was still running then; it completed at 17:12:25Z). The macOS and Linux gate results are therefore known from the job listing only: macOS Gate success; the macOS gate report was not read.

### Later pushes to main (ledger files only)
`ci.yml` run 44350743424 (head `bcca328f7a1dc0da2cb0f2072e83ef1e7a8af67b`, pushed 16:44Z): success. `ci.yml` run 44355327777 (head `d01c08b81af198cf51f26822acae31c57236c0e9`, pushed 17:08Z): `in_progress` when last listed (about 17:19Z; `updated_at` 17:08:52Z). `platforms.yml` listed no run on `main` for either push (the newest `platforms.yml` runs on `main` are run 35 above and the nightly 44298617821).

### Nightly run 44298617821 (platforms.yml, run_number 28, event schedule, main @ 6e41d6462639785af4f7912f367ca88773aced9a, created 2026-11-18T11:52:29Z)
Conclusion **failure** (updated 12:20:57Z). Jobs: `gate-linux` success; `gate-macos` success (runner 1000000929 assigned, 11:57:32Z–12:20:57Z); `gate-windows` failure (Gate step failure); `window-linux` success; `window-probe (macos-15)` success (runner 1000000930); `window-probe (windows-2025)` failure (Window probe step failure). Only the job listing was read for this run.
