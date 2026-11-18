# CI run 44297420849 (platforms.yml) on ci/candidate @ d6f780562e35f42c071ffe864a91ff02ff29881f — T-072 round 1

Record by hand; method as `tasks/ci-runs/README.md` (artefacts first, job logs only where an artefact does not hold the line).
**INCOMPLETE: `gate-macos` had not finished when the 25-minute wait ended (about 12:33Z); see "Not read".**

## Run header
- id 44297420849, run_number 27, attempt 1, event push, workflow `platforms`, title "Join task/T-072-tests into task/T-072"
- branch `ci/candidate`; commit `d6f780562e35f42c071ffe864a91ff02ff29881f`; created_at 2026-11-18T11:44:31Z
- status at the end of the wait: **in_progress** (`gate-macos` still running). Five jobs completed.
- Superseded run (head `277e25812aaf364f94e4d17a52ec14bf3f2e9b52`, pushed 11:43Z): id 44297266071, run_number 26, completed `failure`; gate-linux success, gate-macos success, gate-windows failure, window-linux success, window-probe (macos-15) success, window-probe (windows-2025) failure. Not used further.

## Job table
Wall time = `completed_at` − `started_at` of the job (job listing).
| job | id | runner label | conclusion | wall | first failing step |
|---|---|---|---|---|---|
| gate-linux | 134777982264 | ubuntu-24.04 | **failure** | 32m50s (11:44:34Z–12:17:24Z) | Gate (11:44:53Z–11:58:52Z), failed on `test` only |
| gate-macos | 134777981910 | macos-15 | **not finished** (in_progress at ~12:33Z; started 11:50:00Z) | not available | Gate (11:50:37Z–12:08:17Z) concluded **failure**; "Full test output (gate failed)" concluded `cancelled` after 9m01s; "Disk" in progress since 12:27:07Z |
| gate-windows | 134777981795 | windows-2025 | **failure** | 16m30s (11:44:33Z–12:01:03Z) | Gate (11:44:57Z–11:49:39Z); every later step success |
| window-linux | 134777981994 | ubuntu-24.04 | success | 1m09s (11:44:34Z–11:45:43Z) | none |
| window-probe (macos-15) | 134777981930 | macos-15 | success | 1m00s (11:50:33Z–11:51:33Z) | none |
| window-probe (windows-2025) | 134777982090 | windows-2025 | failure | 2m21s (11:44:34Z–11:46:55Z) | Window probe (11:46:48Z–11:46:50Z); Build kiln success |

Artefacts (`list_workflow_run_artifacts`, fetched with `download_workflow_run_artifact` and curl; size equal to `size_in_bytes`, `testzip()` clean; scratch directory removed after the record):
`gate-linux-44297420849` 55,429 B; `gate-windows-44297420849` 25,204 B; `window-probe-macos-15-…` 704 B; `window-probe-windows-2025-…` 344 B; `window-linux-…` 577 B (the last three listed, not downloaded). No `gate-macos-…` artefact existed at the last listing.

## gate-linux (ubuntu-24.04, image `ubuntu-24.04` version 20261111.370.1 per the job log header)
Step timings (job listing):
| step | conclusion | duration |
|---|---|---|
| Set up job | success | 1s |
| Checkout | success | 2s |
| Install the pinned toolchain | success | 9s |
| Cache cargo registry and target | success | 7s |
| Gate | **failure** | 13m59s (11:44:53Z–11:58:52Z) |
| Gate report | success | 0s |
| Core crates (reported, not required) | success | 7m00s |
| Test list | success | 1s (12:05:52Z–12:05:53Z) |
| Full test output (gate failed) | success (ran) | 11m28s (12:05:53Z–12:17:21Z) |
| Disk | success | 1s |
| Upload reports | success | 0s |
| Post Cache (skipped), Post Checkout, Complete job | skipped / success / success | 0–1s |

Gate report (`gate-report.json`): `passed: false`, `failed_step: test`, 834.8 s.
fmt PASS 1.2 s "formatted"; clippy PASS 14.2 s "no warnings (all targets, -D warnings)"; layers PASS; ui-commands PASS; hygiene PASS ("reviews: 59 of 71 review file(s) checked …; session-paths: 694 tracked file(s) clean"); schema PASS ("30 commands; audit ok; docs/commands.json current"); scorecard PASS 257.3 s "64 items: 47 verified, 0 implemented, 0 failing, 17 planned."; **test FAIL** 550.6 s "1135 passed, 2 failed, 0 ignored in 121 suites"; no-skip PASS "self-test ok (17 known findings); 203 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (19 pending), today = 2026-11-18"; fuzz, budgets EMPTY.

New steps:
- "Test list": 1 s. Log line (job log): `TEST LIST: exit status 0 (recorded; the gate step decides the colour)`.
- `test-list.txt` (76,556 B, 1,567 lines): 136 binary lines (128 `Running`, 8 `Doc-tests`), 1,170 lines ending `: test` (test names).
- "Full test output (gate failed)" **ran** (the gate was red): 11m28s; log line `TEST OUTPUT: exit status 101 (recorded; the gate step decides the colour)`. `test-output.txt` 97,848 B, 2,023 lines (artefact); summing its `test result:` lines gives 1,169 passed, 2 failed in 137 result lines (the gate report says 1,135 / 121 suites; the difference is not explained by any file read, not determinable).
- Printer output for the red job (artefact `test-output.txt` lines 1866–1890; the log shows the same blocks; failing tests, verbatim):
```
---- ac1_the_list_step_runs_the_list_records_its_status_and_prints_the_binaries stdout ----

thread 'ac1_the_list_step_runs_the_list_records_its_status_and_prints_the_binaries' (184487) panicked at xtask/tests/acceptance_t072.rs:966:17:
job `gate-macos`: no line `TEST LIST: exit status 101`:
     Running unittests src/lib.rs (target/debug/deps/kiln_a-00aa11)
     Running tests/zeta_list.rs (target/debug/deps/zeta_list-00bb22)
   Doc-tests kiln_a
error: could not compile `kiln-b` (test "omega_list") due to 2 previous errors
TEST LIST: exit status 101 (recorded; the gate step decides the colour)

note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- docs_ci_table_shows_a_green_gate_macos_with_the_inode_columns stdout ----

thread 'docs_ci_table_shows_a_green_gate_macos_with_the_inode_columns' (184571) panicked at xtask/tests/acceptance_t072.rs:1476:5:
the gate-macos row's result is not green: | `gate-macos` | red (gate: scorecard, test) | 21 min 20 s, of which 11 min 26 s the full-test-output step | `/dev/disk2s5 320Gi 262Gi 40Gi 87% 2.3M 419M 1% /System/Volumes/Data` (size, used, avail, capacity, iused, ifree, %iused, mounted on) | 4.4G |
```
  (In the job log the first block shows the lines `Running …` of the message not at all and the rest as above; the artefact has them. Not investigated.)
- Artefact files: `gate-report.json` 5,421 B; `core-crates.txt` 41,626 B; `test-output.txt` 97,848 B; `test-list.txt` 76,556 B. Upload zip 55,429 B.
- Disk (job log): `DISK: df -h of the workspace's volume`, `Filesystem      Size  Used Avail Use% Mounted on`, `/dev/root       145G   65G   80G  45% /`; `DISK: du -sh target` → `4.5G	target`.
- `core-crates.txt`: labels `==== cargo test --locked --no-fail-fast -p kiln-text|kiln-syntax|kiln-search|xtask`; `kiln-text`, `kiln-syntax`, `kiln-search` exit status 0; `==== xtask: exit status 101`.

## gate-windows (windows-2025; image version not read: the log was read as a tail only)
Step timings:
| step | conclusion | duration |
|---|---|---|
| Set up job | success | 1s |
| Checkout | success | 6s |
| Install the pinned toolchain | success | 12s |
| Cache cargo registry and target | success | 4s |
| Gate | **failure** | 4m42s (11:44:57Z–11:49:39Z) |
| Gate report | success | 0s |
| Core crates (reported, not required) | success | 11m06s |
| Workspace lib and T-073 tests (reported, not required) | success | 5s |
| Test list | success | 1s (12:00:50Z–12:00:51Z) |
| kiln-workspace test targets (reported, not required) | success | 4s |
| Full test output (gate failed) | success (ran) | 1s |
| Disk | success | 2s |
| Upload reports | success | 2s |
| Post Cache (skipped), Post Checkout, Complete job | skipped / success / success | 0–1s |

Gate report: `passed: false`, `failed_step: clippy`, 266.3 s.
| step | status | detail |
|---|---|---|
| fmt | PASS | formatted |
| clippy | **FAIL** | 41.7 s; kept text (last 60 lines) ends `error: could not compile `kiln-cli` (test "guard_t036_removal_cli") due to 17 previous errors` (E0599 `mode`, `uid`, `gid`, `from_mode`, `dev`, `ino` on `std::fs::Metadata`/`Permissions`, `apps\kiln-cli\tests\guard_t036_removal_cli.rs` lines 161–270) |
| layers | PASS | 11 crates, 14 internal edges, layer table holds |
| ui-commands | PASS | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | PASS | reviews: 59 of 71 …; session-paths: 694 tracked file(s) clean |
| schema | PASS | 30 commands; audit ok; docs/commands.json current |
| scorecard | **FAIL** | `windows: 64 items; 158 tests ran; 3 registered absent here; page not compared (computed on Linux)`; `60 scorecard problems:`; one is "building test executables failed on windows (); their tests are not run"; one is ``io.save-atomic: test `kiln_workspace::acceptance_t002::disk_save_replaces_via_rename_without_leftovers` fails on windows`` |
| test | **FAIL** | "0 passed, 0 failed, 0 ignored in 0 suites"; kept text ends with `could not compile `kiln` (test "acceptance_t065") due to 3 previous errors`, `(test "script_t055") due to 1 previous error`, `(test "acceptance_t055") due to 2 previous errors` |
| no-skip | PASS | 203 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (19 pending) |
| fuzz, budgets | EMPTY | no targets registered yet |

New steps:
- "Test list": 1 s; log: `TEST LIST: exit status 101 (recorded; the gate step decides the colour)`. `test-list.txt` 10,832 B, 172 lines: only compile errors (`error[E0433]` `unix` in `os`, `from_vec`, `from_bytes`, `[OsString]` size); **0 binary lines, 0 test names**. It has the same content as `test-output.txt` (same size, 172 lines; only the order of the error blocks differs).
- "Full test output (gate failed)" ran (1 s); log: `TEST OUTPUT: exit status 101 (recorded; the gate step decides the colour)`. The printer printed only `error[…]` and `error: could not compile` lines: `kiln` tests `window_t065` (8 errors), `acceptance_t065` (3), `script_t055` (1), `acceptance_t055` (2). No failing test names (nothing compiled to run).
- Artefact files: `gate-report.json` 17,180 B; `test-output.txt` 10,832 B; `core-crates.txt` 41,684 B; `workspace-tests.txt` 7,134 B; `test-list.txt` 10,832 B; `windows-tests.txt` 62,809 B. Zip 25,204 B.
- Disk (job log): `Filesystem      Size  Used Avail Use% Mounted on`, `D:              220G  3.7G  217G   2% /d`; `3.5G	target`.

### Item 3: "Workspace lib and T-073 tests" and "kiln-workspace test targets"
"Workspace lib and T-073 tests" (`workspace-tests.txt`): lib `36 passed; 0 failed` (includes `disk::tests::held_root_check_is_inside_only_for_a_read_identity_equal_to_the_held_one ... ok`), `acceptance_t073` `1 passed; 0 failed`, `guard_t073` `10 passed; 0 failed`: 47 passed, 0 failed; log `WORKSPACE TESTS: exit status 0 (reported, not required)`. The two `MEASURED:` lines are the same as in T-073 round 2.

`windows-tests.txt`, every status line verbatim:
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
Log summary line: `WINDOWS TESTS: exit status per target: acceptance_t002=101 acceptance_t002_r2=101 acceptance_t008=101 acceptance_t015_tools=101 acceptance_t019_workspace=101 acceptance_t036_workspace=101 acceptance_t073=0 confinement=101 guard_t008=101 guard_t036_removal_workspace=101 guard_t036_workspace=101 guard_t073=0 project=101 tools_contract=0 (reported, not required)`.

Non-zero targets:
- `acceptance_t002` (compiled; `15 tests`, `test result: FAILED. 14 passed; 1 failed`): `---- disk_save_replaces_via_rename_without_leftovers stdout ----`, `thread 'disk_save_replaces_via_rename_without_leftovers' (5796) panicked at crates\kiln-workspace\tests\acceptance_t002.rs:940:5:`, `precondition: permission bits and inode numbers are Unix metadata; the Windows checks of atomic replace are not written yet (ADR-0009: T-073 and T-074 write the Windows behaviour and its tests)`.
- `acceptance_t015_tools` (compiled; 3 warnings; `test result: FAILED. 2 passed; 1 failed`): `test no_tools_never_runs ... FAILED`; `thread 'no_tools_never_runs' (9964) panicked at crates\kiln-workspace\tests\acceptance_t015_tools.rs:131:5:`, `precondition: the fake `cargo`/`rustfmt` of this test are shell scripts and need Unix`.
- compile errors (first lines; none printed a test failure): `acceptance_t002_r2` 11 errors, first `error[E0433]: cannot find `unix` in `os`` at `crates\kiln-workspace\tests\acceptance_t002_r2.rs:16:18`; `acceptance_t008` 20 errors, same error at `acceptance_t008.rs:20:14`; `acceptance_t019_workspace` 5, at `:19:14`; `acceptance_t036_workspace` 16, at `:20:14`; `confinement` 24, at `:9:14`; `guard_t008` 1, at `:11:14`; `guard_t036_removal_workspace` 3, at `:26:14`; `guard_t036_workspace` 1, at `:17:14`; `project` 3, at `:10:14`. Each ends `error: could not compile `kiln-workspace` (test "<target>") due to N previous errors`.
- Zero: `acceptance_t073` (1 passed), `guard_t073` (10 passed), `tools_contract` (5 passed).

## Item 4: `xtask` tests (`core-crates.txt`)
| platform | `==== xtask: exit status` | binaries | totals | `acceptance_t072` | `guard_t072` |
|---|---|---|---|---|---|
| Linux | 101 | 25 | 309 passed, 2 failed | ran: `15 passed; 2 failed` | ran: `7 passed; 0 failed`, 125.26 s |
| Windows | 101 | 25 | 301 passed, 2 failed | ran: `15 passed; 2 failed` | ran: `7 passed; 0 failed`, 127.33 s |
| macOS | not read | not read | not read | not read | not read |
Failing on both: exactly the two known tests, `ac1_the_list_step_runs_the_list_records_its_status_and_prints_the_binaries` (panic at `xtask/tests/acceptance_t072.rs:966:17`, message quoted above: ``job `gate-macos`: no line `TEST LIST: exit status 101` `` while the fake run printed `TEST LIST: exit status 101 (recorded; the gate step decides the colour)`) and `docs_ci_table_shows_a_green_gate_macos_with_the_inode_columns` (`acceptance_t072.rs:1476:5`, message quoted above). **No other failing test** on Linux or Windows. Totals: sum of `test result:` lines after `==== cargo test … -p xtask`.
Windows and Git Bash: the job's shell is `C:\Program Files\Git\bin\bash.EXE --noprofile --norc -e -o pipefail` (job log). The step-script tests ran there: all 7 guards and the other 14 acceptance tests passed, and the failing list-step test prints the fake `cargo`'s output (`Running unittests src/lib.rs (target/debug/deps/kiln_a-00aa11)` …), so the script ran under bash. How the tests found bash is not stated in the output. The Windows output of the Windows-only step test `ac9_the_windows_step_reports_every_workspace_test_target_with_its_status` is `ok`.

## Item 5: figures for "Runners, wall times and disk"
| job | runner label, image | total wall | Gate step | Test list | Full test output | disk line (`df`) | `target/` |
|---|---|---|---|---|---|---|---|
| gate-linux | ubuntu-24.04, image ubuntu-24.04 20261111.370.1 | 32m50s | 13m59s (report 834.8 s) | 1s | 11m28s (ran, gate red) | `/dev/root       145G   65G   80G  45% /` | 4.5G |
| gate-macos | macos-15, image not read | not finished | 17m40s (failed) | 4s | 9m01s, `cancelled` | not read (Disk step never completed) | not read |
| gate-windows | windows-2025, image not read | 16m30s | 4m42s (report 266.3 s) | 1s | 1s (ran, nothing compiled) | `D:              220G  3.7G  217G   2% /d` | 3.5G |
macOS inode columns: not read.

## Against the expectation (implementer's "CI expectation", `tasks/T-072-gate-green-on-macos.md` at d6f7805; the head also holds the test author's files, which the expectation excluded)
**gate-linux**
- "green": **not met**. Red, only `test`: 1135 passed, 2 failed — the two known acceptance tests (the test author's files, not in the implementer's branch alone).
- "Test list prints one `Running`/`Doc-tests` line per binary (134 locally)": **met, count differs by the 2 new test files**: 136 (128 `Running` + 8 `Doc-tests`) in `test-list.txt`; the 134 was measured without `acceptance_t072`/`guard_t072` (INFERRED).
- "`TEST LIST: exit status 0`": **met** (log line above).
- "Full test output skipped": **not met, because the gate was red**; it ran for 11m28s and `test-output.txt` was uploaded.
- "`core-crates.txt` holds `==== cargo test --locked --no-fail-fast -p <package>` lines": **met**.
- "artefact `gate-report.json`, `core-crates.txt`, `test-list.txt` (about 1,146 `<name>: test` lines), no `test-output.txt`": files met; `test-output.txt` present (red gate); names **1,170** (1,146 + 24 = 17 + 7 tests of the two new files, INFERRED).
**gate-macos**: all points **not determinable** (job unfinished, no artefact). Seen: Gate concluded `failure` (not green, as the 2 known failures would make it), Test list 4 s, Full test output ran 9m01s and was `cancelled`, Disk step in progress from 12:27:07Z, still running at about 12:33Z. `TEST LIST: exit status` line, list size, names: not read.
**gate-windows**
- "red as on main (clippy, scorecard, test)": **met** (`failed_step: clippy`; clippy, scorecard, test FAIL; the other steps PASS or EMPTY).
- "Test list: errors and `TEST LIST: exit status 101`, few or no names": **met** (101, 0 names, 0 binaries).
- "14 targets in this order, each with `==== <t>: exit status N`": **met** (the 14 lines above, same names and order).
- "the 9 that do not compile exit 101 with `error[`": **met** (acceptance_t002_r2, acceptance_t008, acceptance_t019_workspace, acceptance_t036_workspace, confinement, guard_t008, guard_t036_removal_workspace, guard_t036_workspace, project).
- "`acceptance_t073`, `guard_t073` exit 0": **met**. "(probably) `acceptance_t015_tools`, `tools_contract` exit 0": `tools_contract` **met** (0); `acceptance_t015_tools` **not met** — 101, it compiled and `no_tools_never_runs` panicked with "precondition: the fake `cargo`/`rustfmt` of this test are shell scripts and need Unix".
- "`acceptance_t002` exits 101 with `disk_save_replaces_via_rename_without_leftovers` printed as a `---- … stdout ----` block": **met** (message quoted above).
- "summary line `WINDOWS TESTS: exit status per target: …`": **met** (log).
- "Full test output runs, prints `error[` and `could not compile`, `TEST OUTPUT: exit status 101`": **met**.
- "T-073's step unchanged (47 `ok`)": **met** (36 + 1 + 10).
- "Artefact: gate-report.json, core-crates.txt, test-output.txt, workspace-tests.txt, test-list.txt, windows-tests.txt": **met**, all six present.
**Window jobs**: `window-linux` success, `window-probe (macos-15)` success, `window-probe (windows-2025)` failure with Build kiln success (as on main); the probe's `PROBE: exit status 2` line itself: **not read** (artefact not downloaded).
**"List step's time per gate job in step timings; seconds, not minutes, where the gate built the tests"**: **met**: Linux 1 s, Windows 1 s, macOS 4 s.

## Other observations
- The Linux and Windows gate steps were red only because of the two known acceptance tests (Linux) or the Windows compile/test failures as on main; there is no failing test beyond those two on Linux. Clippy's named Windows target is now `kiln-cli` `guard_t036_removal_cli` (17 errors).
- Why `ac1_the_list_step_…` fails (INFERRED from its message): the test looks for a line `TEST LIST: exit status 101`, while the step prints `TEST LIST: exit status 101 (recorded; the gate step decides the colour)`.

## Not read
- `gate-macos`: everything (unfinished: Full test output cancelled, Disk in progress, no artefact, `get_job_logs` returned HTTP 404 while it ran): gate report, `TEST LIST: exit status` line, `test-list.txt` size/counts, core-crates/xtask totals and the two T-072 test binaries, `df` line with inode columns, `target/` size, image version, total wall.
- Windows image version (log read as a tail of 900 lines); the three window artefacts' contents (including the Windows probe's exit status).
- The first lines of clippy's, scorecard's and test's failing output on Windows (the report keeps the last 60/27 lines).
