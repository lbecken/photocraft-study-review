# CI run 43775682214 (platforms.yml, round 1) on ci/T-054 @ 213df3e048dd436f34e2eb6d4652eabc471817cc

Extracted from the runs API, the jobs API and job logs only (GitHub MCP tools). Times UTC as the API gives them. Wall = job completed_at - started_at (jobs API). Logs were read as the last 600 lines of each gate job (the whole Gate step through Disk was inside that window) and the last 250/120/120 lines of the three window jobs.

## Run header
- id 43775682214 (run_number 5, attempt 1), event push, workflow `platforms`
- branch ci/T-054; commit 213df3e048dd436f34e2eb6d4652eabc471817cc, title "Join task/T-054-tests into task/T-054" (Task: T-054, Role: director)
- created_at 2026-11-15T23:35:56Z; updated_at 2026-11-16T00:00:17Z
- status completed, conclusion failure

## Job table
| job | id | conclusion | started | completed | wall | first failing step |
|---|---|---|---|---|---|---|
| gate-linux | 133001317991 | success | 23:35:58Z | 23:47:14Z | 11m16s | none |
| gate-macos | 133001317871 | failure | 23:35:59Z | 00:00:17Z (11-16) | 24m18s | Gate (23:36:12Z-23:44:02Z) |
| gate-windows | 133001318039 | failure | 23:35:59Z | 23:43:55Z | 7m56s | Gate (23:36:26Z-23:39:08Z) |
| window-linux | 133001318035 | success | 23:35:59Z | 23:37:43Z | 1m44s | none |
| window-probe (macos-15) | 133001318014 | success | 23:36:01Z | 23:38:11Z | 2m10s | none |
| window-probe (windows-2025) | 133001318018 | failure | 23:35:59Z | 23:38:03Z | 2m04s | Build kiln (23:36:28Z-23:37:58Z) |

Step "Core crates (reported, not required)": success on all three gate jobs (it always exits 0). "Full test output (gate failed)": skipped on Linux, success (ran) on macOS and Windows.

## gate-linux (success)
Gate summary:
```
PASS  fmt             0.7s  formatted
PASS  clippy         34.6s  no warnings (all targets, -D warnings)
PASS  layers          0.7s  11 crates, 14 internal edges, layer table holds
PASS  ui-commands     0.0s  self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s)
PASS  hygiene         0.1s  reviews: 46 of 58 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 608 tracked file(s) clean (0 not UTF-8, skipped)
PASS  schema         15.4s  30 commands; audit ok; docs/commands.json current
PASS  scorecard     132.3s  64 items: 44 verified, 1 implemented, 0 failing, 19 planned.
PASS  test          277.7s  1025 passed, 0 failed, 0 ignored in 125 suites
PASS  no-skip         1.3s  self-test ok; 190 files scanned, 0 allowlisted skip(s), today = 2026-11-15
EMPTY fuzz            0.0s  no targets registered yet (xtask/fuzz.toml)
EMPTY budgets         0.0s  no targets registered yet (xtask/budgets.toml)
GATE PASSED in 462.9s (11 steps; 2 step(s) EMPTY (nothing registered): fuzz, budgets)
```
Core crates:
```
==== kiln-text: exit status 0
==== kiln-syntax: exit status 0
==== kiln-search: exit status 0
==== xtask: exit status 0
CORE CRATES: exit status per package: kiln-text=0 kiln-syntax=0 kiln-search=0 xtask=0 (reported, not required)
```
Disk:
```
Filesystem            Size  Used Avail Use% Mounted on
/dev/mapper/cow-root  145G   62G   84G  43% /
DISK: du -sh target
4.2G	target
```
The xtask block of the Core crates step printed 19 `test result: ok.` lines (119, 16, 4, 11, 6, 0, 2, 5, 9, 11, 5, 4, 0, 5, 13, 3, 1, 2, 7 passed; 0 failed in each).

## gate-macos (failure; Gate step failed)
Gate summary:
```
PASS  fmt             1.0s  formatted
PASS  clippy         48.6s  no warnings (all targets, -D warnings)
PASS  layers          2.9s  11 crates, 14 internal edges, layer table holds
PASS  ui-commands     0.0s  self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s)
PASS  hygiene         0.2s  reviews: 46 of 58 review file(s) checked (medium/high, iteration >= 1), ok; ...
PASS  schema         30.3s  30 commands; audit ok; docs/commands.json current
FAIL  scorecard     365.1s  7 scorecard problems:
FAIL  test            4.2s  25 passed, 1 failed, 0 ignored in 2 suites
PASS  no-skip         1.7s  self-test ok; 190 files scanned, 0 allowlisted skip(s), today = 2026-11-15
EMPTY fuzz / EMPTY budgets
GATE FAILED in 454.0s: scorecard, test
```
Gate `test` step: `test ac8_stdout_write_failure_exits_1 ... FAILED`, then `Running tests/acceptance_t055.rs (...)` and `error: test failed, to rerun pass `-p kiln --test acceptance_t055``. It stopped after the first failing package (2 suites), so no xtask test ran in the gate step.

Core crates:
```
==== kiln-text: exit status 0
==== kiln-syntax: exit status 0
==== kiln-search: exit status 0
test result: FAILED. 116 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 25.23s
==== xtask: exit status 101
CORE CRATES: exit status per package: kiln-text=0 kiln-syntax=0 kiln-search=0 xtask=101 (reported, not required)
```
Only ONE `test result:` line for xtask: a suite of 119 tests with 3 failed (same size, 119, as Linux's first xtask suite). `cargo test -p xtask` has no `--no-fail-fast`, so after that failure the remaining xtask test binaries were not run. The step's grep prints no `Running` lines (cargo colours them, so `^     Running ` does not match), so no test names are printed, and the names of the 3 failed tests are not in the log I read.

Full test output (gate failed), the guard/acceptance names: NO line matches `guard_t054`, `acceptance_t054`, `ac2_backtrace`, `ac2_panic_location` or `ac4_` anywhere in the macOS log (grep count 0). This step is `cargo test --workspace --locked --no-fail-fast ... | grep ... | head -200`, and the printed block is exactly 200 lines long (log lines 333-532), so it was cut by `head -200` before any xtask result; it covers apps/kiln, apps/kiln-cli, and others. Failures printed there (none named in T-054): `ac8_stdout_write_failure_exits_1` (acceptance_t055.rs:329, "open /dev/full ... No such file or directory"), `ac1_*` x3 (acceptance_t065.rs:65, "did not finish within 120s"), `kiln_without_subcommand_and_display_exits_nonzero_with_a_message` (guard_t055.rs), `without_a_display_the_window_exits_2_with_one_line` (window_t065.rs), `run_and_control_exit_1_when_stdout_cannot_be_written` (acceptance_t004_r2.rs, "this test needs /dev/full"), `deep_trees_stop_at_the_depth_bound` and `tree_order_and_counts` (acceptance_t019_cli.rs), `disk_refusals_through_control` (guard_t038_cli.rs), `run_exits_1_when_stdout_cannot_be_written` (run.rs:396). Last line: `TEST OUTPUT: exit status 101 (recorded; the gate step decides the colour)`.

Disk:
```
Filesystem      Size    Used   Avail Capacity iused ifree %iused  Mounted on
/dev/disk2s5   320Gi   260Gi    42Gi    87%    2.3M  437M    1%   /System/Volumes/Data
DISK: du -sh target
3.0G	target
```

## gate-windows (failure; Gate step failed)
Gate summary:
```
PASS  fmt             2.0s  formatted
FAIL  clippy         25.6s  note: found an item that was configured out
PASS  layers         10.9s  11 crates, 14 internal edges, layer table holds
PASS  ui-commands     0.0s  self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s)
PASS  hygiene         0.2s  reviews: 46 of 58 review file(s) checked ...
FAIL  schema         23.0s  kiln-cli commands --audit failed:
FAIL  scorecard      67.9s  error: building test executables failed ():
FAIL  test            5.7s  0 passed, 0 failed, 0 ignored in 0 suites
PASS  no-skip         2.6s  self-test ok; 190 files scanned, 0 allowlisted skip(s), today = 2026-11-15
EMPTY fuzz / EMPTY budgets
GATE FAILED in 137.9s: clippy, schema, scorecard, test
```
Compile failures, unix-only std APIs (E0599) in crates built for Windows, e.g. `no method named `dev` found for struct `std::fs::Metadata``, `no method named `mode` found for mutable reference `&mut std::fs::OpenOptions``, `no method named `is_fifo` found for struct `FileType``, `no associated function or constant named `from_mode` found for struct `Permissions`` (crates\kiln-workspace\src\disk.rs:1248:67).

`xtask` did NOT compile its tests on Windows, so no test file of xtask ran (guard_t054.rs and acceptance_t054.rs never ran). Gate `test` step (`cargo test --workspace`) quotes:
```
         --> xtask\src\hygiene.rs:541:18
      541 |         std::os::unix::fs::symlink("a.txt", root.join("ok-link")).unwrap();
          |                  ^^^^ could not find `unix` in `os`
      error[E0599]: no associated function or constant named `from_mode` found for struct `Permissions` in the current scope
          --> xtask\src\publish.rs:1341:63
      error: could not compile `xtask` (bin "xtask" test) due to 4 previous errors
      error: failed to remove file `D:\a\photocraft-study-review\photocraft-study-review\target\debug\xtask.exe`
        Access is denied. (os error 5)
```
Core crates (xtask line has no `test result:` line, only the exit status; the compile error is not printed there because the step's grep `^error` does not match coloured text):
```
==== kiln-text: exit status 0
==== kiln-syntax: exit status 0
==== kiln-search: exit status 0
==== xtask: exit status 101
CORE CRATES: exit status per package: kiln-text=0 kiln-syntax=0 kiln-search=0 xtask=101 (reported, not required)
```
kiln-text, kiln-syntax and kiln-search all ran and passed (every `test result: ok.`).
Full test output (gate failed): printed nothing except `TEST OUTPUT: exit status 101 (recorded; the gate step decides the colour)` (the step took 6 s, which is a compile failure; no `test result:` lines at all).
Mentions of `guard_t054`, `acceptance_t054`, `ac2_backtrace`, `ac2_panic_location`, `ac4_`: none (grep count 0).

Disk:
```
Filesystem      Size  Used Avail Use% Mounted on
D:              150G  4.3G  146G   3% /d
DISK: du -sh target
1.3G	target
```

## Window jobs
- window-linux: success. Lines: `WINDOW CASES PASS`; `AC-3: exit=2 stdout_bytes=0 stderr_lines=1 panicked=0 (pass: 2 0 1 0)` / `AC-3 PASS` (stderr: `kiln: no window: Library libxkbcommon-x11.so could not be loaded.`); `PLAIN PASS: the start path changes the screen`. Cases ac2, ac2x3, dblclick, scroll100, ctrlq, noroot each `exit window=N headless=N`, `files EQUAL`.
- window-probe (macos-15): success. `PROBE OS: macos-15 Darwin ... arm64`, `ProductVersion: 15.8.2`, `PROBE: exit status 0`, `PROBE: stdout     2174 bytes, 3 lines`. The stdout is a window snapshot (role `window`, `frames":3`), stderr empty.
- window-probe (windows-2025): failure at step Build kiln. `error: could not compile `kiln-workspace` (lib) due to 6 previous errors`; probe record: `PROBE: not run (cargo build -p kiln: failure)`.

## T-054 tests per platform
| test | Linux | macOS | Windows |
|---|---|---|---|
| `ac2_backtrace_names_this_file_and_line` | passed (inferred: no name printed; `cargo test -p xtask` exit 0, all 19 `test result:` lines ok, 0 failed; gate `test` 1025 passed, 0 failed) | not run (xtask unit suite failed 3 of 119, `cargo test -p xtask` stopped after it; not in the truncated full output) | not run (xtask tests did not compile: `could not compile `xtask` (bin "xtask" test) due to 4 previous errors`) |
| `ac2_panic_location_is_this_file_and_line` | passed (inferred, same) | not run (same) | not run (same) |
| `acceptance_t054 (5 tests)` | passed (inferred, same) | not run (same) | not run (same) |

"Passed (inferred)" for Linux rests on the exit status and the zero-failed totals, not on a printed test name.

## Not read
- No log line anywhere names `guard_t054`, `acceptance_t054`, `ac2_backtrace`, `ac2_panic_location` or `ac4_` (grep count 0 in the macOS and Windows logs; the Linux log was read in full and shows no such names either). Reason: the Core crates grep (`^     Running `) misses coloured `Running` lines, so no test or binary names are printed, and the macOS "Full test output" is cut at 200 lines by `head -200` before any xtask result. The per-name verdicts therefore cannot be read from these logs; the uploaded artifact `test-output.txt` of each gate job (not fetchable with these tools) would hold them.
- macOS: the names of the 3 failed tests in the 119-test xtask suite, and whether the xtask integration test binaries (guard_t054, acceptance_t054) ran at all in the full output. By cargo's behaviour in the Core crates step they did not run, but that is inference from the single `test result: FAILED` line, not a read line.
- Log windows: gate jobs tail 600 lines (starts inside Set up/Checkout), window-linux tail 250 (starts in the Build step; the lines before are build output), probes tail 120. No larger window was requested as the guard lines were absent from the whole Gate/Core/Disk range.
- The `Gate report` step's JSON for macOS and Windows was only seen as far as the 600-line window shows.

## T-054 tests from the artefacts (R-294)
Source: the artefacts of run 43775682214 (`ci/T-054` @ `213df3e048dd436f34e2eb6d4652eabc471817cc`), listed with
`list_workflow_run_artifacts` and fetched through `download_workflow_run_artifact` links (2026-11-16 ~00:03Z):
`gate-macos-43775682214` (`core-crates.txt`, `gate-report.json`, `test-output.txt`), `gate-linux-43775682214`
(`core-crates.txt`, `gate-report.json`; no `test-output.txt`, the gate passed), `gate-windows-43775682214`
(`core-crates.txt`, `gate-report.json`, `test-output.txt`). Read as data; the download directory was deleted.

**macOS** (`test-output.txt`, lines 1925–1934 and 1971–1977):
```
     Running tests/acceptance_t054.rs (target/debug/deps/acceptance_t054-9662ca1369a808f4)

running 5 tests
test ac4_dev_incremental_is_false ... ok
test ac4_dev_dependencies_debug_is_false ... ok
test ac4_dev_debug_is_line_tables_only ... ok
test ac4_dev_sets_no_opt_level_debug_assertions_or_overflow_checks ... ok
test ac4_release_and_bench_profiles_are_absent ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/guard_t054.rs (target/debug/deps/guard_t054-b4ac178d0fd90abc)

running 2 tests
test ac2_panic_location_is_this_file_and_line ... ok
test ac2_backtrace_names_this_file_and_line ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
```
Whole output: 125 `test result:` lines, **1,003 passed, 22 failed**. The 22 failing names are exactly the 22 of
run 43765532298 classified in R-285 (`tasks/T-071-*.md` Interface (4), `tasks/T-079-*.md` Interface (1)): none new,
none gone.

**Linux** (`core-crates.txt`, the `cargo test -p xtask` part, lines 592–601 and 638–644):
```
     Running tests/acceptance_t054.rs (target/debug/deps/acceptance_t054-5c903bcb8558e72a)

running 5 tests
test ac4_dev_dependencies_debug_is_false ... ok
test ac4_dev_sets_no_opt_level_debug_assertions_or_overflow_checks ... ok
test ac4_dev_incremental_is_false ... ok
test ac4_dev_debug_is_line_tables_only ... ok
test ac4_release_and_bench_profiles_are_absent ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/guard_t054.rs (target/debug/deps/guard_t054-7718025f58ade8b0)

running 2 tests
test ac2_panic_location_is_this_file_and_line ... ok
test ac2_backtrace_names_this_file_and_line ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

**Windows:** no `xtask` test binary ran. `core-crates.txt` `==== cargo test --locked -p xtask` (line 322) ends with
`error: could not compile `xtask` (test "guard_t005") due to 3 previous errors` (379), `… (test "acceptance_t005_r3")
due to 2 previous errors` (381) and `==== xtask: exit status 101` (382), with no `Running` line for any xtask target;
`test-output.txt` shows `could not compile `kiln-workspace` (lib) due to 6 previous errors` (102), `… xtask (test
"acceptance_t005_r3") …` (119) and `… xtask (bin "xtask" test) due to 4 previous errors` (141). So `guard_t054` and
`acceptance_t054` have **no verdict on Windows** in this run.
