# CI run 43765532298 (platforms.yml, round 2) on ci/T-070 @ 3fac206ea934707dd42607b320c4c29c361b215c

Extracted from the runs API, the jobs API and job logs only. Times UTC as the API gives them. Wall = job completed_at - started_at (jobs API).

## Run header
- id 43765532298 (run_number 3, attempt 1), event push, workflow `platforms`
- branch ci/T-070; commit 3fac206ea934707dd42607b320c4c29c361b215c, title "Join task/T-070-tests into task/T-070" (Task: T-070, Role: director)
- created_at 2026-11-15T21:46:27Z; updated_at 2026-11-15T22:12:36Z
- status completed, conclusion failure

## Job table
| job | id | conclusion | started | completed | wall | first failing step |
|---|---|---|---|---|---|---|
| gate-linux | 132969648272 | success | 21:46:29Z | 21:56:35Z | 10m06s | none |
| gate-macos | 132969648242 | failure | 21:46:30Z | 22:12:36Z | 26m06s | Gate (21:46:42Z-21:55:07Z) |
| gate-windows | 132969648214 | failure | 21:46:29Z | 21:55:57Z | 9m28s | Gate (21:46:55Z-21:50:38Z) |
| window-linux | 132969648240 | success | 21:46:29Z | 21:47:30Z | 1m01s | none |
| window-probe (macos-15) | 132969648091 | success | 21:46:31Z | 21:47:48Z | 1m17s | none |
| window-probe (windows-2025) | 132969647917 | failure | 21:46:29Z | 21:48:42Z | 2m13s | Build kiln (21:47:00Z-21:48:37Z) |

## gate-linux (success)
Logs read: tail 400 lines (covered the whole Gate step onward).
```
PASS  fmt             0.6s  formatted
PASS  clippy          9.1s  no warnings (all targets, -D warnings)
PASS  layers          0.6s  11 crates, 14 internal edges, layer table holds
PASS  ui-commands     0.0s  self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s)
PASS  hygiene         0.0s  reviews: 43 of 55 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 594 tracked file(s) clean (0 not UTF-8, skipped)
PASS  schema          4.9s  30 commands; audit ok; docs/commands.json current
PASS  scorecard     110.8s  64 items: 44 verified, 0 implemented, 0 failing, 20 planned.
PASS  test          269.4s  999 passed, 0 failed, 0 ignored in 121 suites
PASS  no-skip         1.3s  self-test ok; 186 files scanned, 0 allowlisted skip(s), today = 2026-11-15
EMPTY fuzz            0.0s  no targets registered yet (xtask/fuzz.toml)
EMPTY budgets         0.0s  no targets registered yet (xtask/budgets.toml)
GATE PASSED in 396.6s (11 steps; 2 step(s) EMPTY (nothing registered): fuzz, budgets)
```
Step "Full test output (gate failed)": skipped. Core crates step:
```
==== kiln-text: exit status 0
==== kiln-syntax: exit status 0
==== kiln-search: exit status 0
==== xtask: exit status 0
CORE CRATES: exit status per package: kiln-text=0 kiln-syntax=0 kiln-search=0 xtask=0 (reported, not required)
```
kiln-text test results (per binary, in order): 31, 21, 6, 3, 17, 0 passed, all ok (five binaries with tests; the 17-test one finished in 9.19s).
Disk:
```
Filesystem      Size  Used Avail Use% Mounted on
/dev/root       145G   69G   76G  48% /
DISK: du -sh target
8.6G	target
```

## gate-macos (failure; Gate step failed)
Logs read: tail 800 lines.
Gate summary:
```
FAIL  scorecard     384.4s  7 scorecard problems:
FAIL  test            4.2s  25 passed, 1 failed, 0 ignored in 2 suites
GATE FAILED in 488.3s: scorecard, test
```
(fmt, clippy 60.1s, layers, ui-commands, hygiene, schema, no-skip PASS; fuzz, budgets EMPTY.)
Scorecard problems as printed (7):
```
control.errors: test `kiln_cli::guard_t038_cli::disk_refusals_through_control` fails
ui.window: test `kiln::acceptance_t065::ac1_no_display_exits_2_with_one_line` fails
ui.window: test `kiln::acceptance_t065::ac1_no_display_every_window_form` fails
ui.window: test `kiln::acceptance_t065::ac1_wayland_only_or_unreachable_display_exits_2` fails
ws.open-project: test `kiln_cli::acceptance_t019_cli::tree_order_and_counts` fails
ws.open-project: test `kiln_cli::acceptance_t019_cli::deep_trees_stop_at_the_depth_bound` fails
docs/scorecard.md is stale; run `cargo xtask scorecard` and commit it
```
Core crates:
```
==== kiln-text: exit status 0
==== kiln-syntax: exit status 0
==== kiln-search: exit status 0
test result: FAILED. 116 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 26.29s
==== xtask: exit status 101
CORE CRATES: exit status per package: kiln-text=0 kiln-syntax=0 kiln-search=0 xtask=101 (reported, not required)
```
Test counts: the Gate "test" step says `25 passed, 1 failed, 0 ignored in 2 suites` (it stops at the first failing suite); xtask core-crates run: 116 passed, 3 failed (their names are not printed).
Full test output step (21:59:17Z-22:12:30Z): the printed block is a `head -200` prefix of a filtered grep, so it is a prefix of the run, not all of it. Sum of its `test result:` lines (script over the saved log text, 46 binaries): 366 passed, 11 failed. Failing tests whose panic lines are in the printed block (11):
1. ac8_stdout_write_failure_exits_1 (kiln acceptance_t055.rs:329:14)
2. ac1_no_display_every_window_form (acceptance_t065.rs:65:13)
3. ac1_no_display_exits_2_with_one_line (acceptance_t065.rs:65:13)
4. ac1_wayland_only_or_unreachable_display_exits_2 (acceptance_t065.rs:65:13)
5. kiln_without_subcommand_and_display_exits_nonzero_with_a_message (guard_t055.rs:55:13)
6. without_a_display_the_window_exits_2_with_one_line (window_t065.rs:68:13)
7. run_and_control_exit_1_when_stdout_cannot_be_written (kiln-cli acceptance_t004_r2.rs:124:5)
8. deep_trees_stop_at_the_depth_bound (acceptance_t019_cli.rs:1573:5)
9. tree_order_and_counts (acceptance_t019_cli.rs:1018:5)
10. disk_refusals_through_control (guard_t038_cli.rs:86:9, 262:5)
11. run_exits_1_when_stdout_cannot_be_written (kiln-cli run.rs:396:62)
Same set and counts (366 passed, 11 failed, 46 binaries) as round 1's printed prefix; same panic locations.
Disk:
```
Filesystem      Size    Used   Avail Capacity iused ifree %iused  Mounted on
/dev/disk2s5   320Gi   262Gi    40Gi    87%    2.3M  421M    1%   /System/Volumes/Data
DISK: du -sh target
4.4G	target
```

## gate-windows (failure; Gate step failed)
Logs read: tail 400 lines (covered the Gate step onward).
```
FAIL  clippy         26.3s     |
FAIL  schema         45.7s  kiln-cli commands --audit failed:
FAIL  scorecard     105.9s  error: building test executables failed ():
FAIL  test            2.9s  0 passed, 0 failed, 0 ignored in 0 suites
GATE FAILED in 197.1s: clippy, schema, scorecard, test
```
(fmt, layers, ui-commands, hygiene, no-skip PASS; fuzz, budgets EMPTY.) Note: the clippy line text differs from round 1 (`FAIL  clippy ... |` here against `note: found an item that was configured out` in round 1); it is the first printed line of the compiler output.
Compile errors (counts printed by the compiler lines in the step details):
- clippy and schema: `error: could not compile `kiln-workspace` (lib) due to 6 previous errors`; clippy detail lists crates\kiln-workspace\src\disk.rs lines 189:36, 189:48, 213:35, 213:47, 361:14 (plus the E0433 error, with `Some errors have detailed explanations` naming E0433/E0599 in the build-log style)
- scorecard: `error: could not compile `kiln-workspace` (lib test) due to 34 previous errors`; visible locations crates\kiln-workspace\src\disk.rs:1248:67 (from_mode), 1283:65 and 1353:18 (is_fifo)
- test: `error: could not compile `xtask` (bin "xtask" test) due to 4 previous errors`; files xtask\src\hygiene.rs:541:18 (E0433 cannot find `unix` in `os`) and xtask\src\publish.rs:1341:63 (from_mode)
- Files named overall: crates\kiln-workspace\src\disk.rs, xtask\src\hygiene.rs, xtask\src\publish.rs. Total across steps by the compiler's own counts: 6 (lib) + 34 (lib test) + 4 (xtask test).
Core crates:
```
==== kiln-text: exit status 0
==== kiln-syntax: exit status 0
==== kiln-search: exit status 0
==== xtask: exit status 101
CORE CRATES: exit status per package: kiln-text=0 kiln-syntax=0 kiln-search=0 xtask=101 (reported, not required)
```
Full test output step: printed only `TEST OUTPUT: exit status 101 (recorded; the gate step decides the colour)`.
Disk:
```
Filesystem      Size  Used Avail Use% Mounted on
D:              150G  5.6G  145G   4% /d
DISK: du -sh target
2.6G	target
```

## window-linux (success)
Logs read: tail 200 lines. Step "Window procedure (T-065) under Xvfb": success. Step "Installed X and GL packages (for the record)": success.
Verdict lines exactly as printed (the six `CASE` lines start at column 0; the echoed lines of kiln's own output are indented by two spaces):
```
CASE ac2: exit window=0 headless=0
  first snapshot Title src/b.rs and Message not_found: True
  lines 12 12 | without counts: EQUAL | with counts: EQUAL
  files EQUAL
CASE ac2x3: exit window=3 headless=3
  lines 13 13 | without counts: EQUAL | with counts: EQUAL
  files EQUAL
CASE dblclick: exit window=0 headless=0
  lines 7 7 | without counts: EQUAL | with counts: EQUAL
  files EQUAL
CASE scroll100: exit window=0 headless=0
  lines 4 4 | without counts: EQUAL | with counts: EQUAL
  files EQUAL
CASE ctrlq: exit window=0 headless=0
  lines 9 9 | without counts: EQUAL | with counts: EQUAL
  files EQUAL
CASE noroot: exit window=0 headless=0
  lines 12 12 | without counts: EQUAL | with counts: EQUAL
  files EQUAL
WINDOW CASES PASS
AC-3: exit=2 stdout_bytes=0 stderr_lines=1 panicked=0 (pass: 2 0 1 0)
  stderr: kiln: no window: Library libxkbcommon-x11.so could not be loaded.
AC-3 PASS
missing script: exit=2 stdout_bytes=0 (pass: 2 0; 124 = a window opened)
plain none: drawn_and_stable=yes still_running=yes
plain none2: drawn_and_stable=yes still_running=yes
plain apath: drawn_and_stable=yes still_running=yes
  two runs without a path: same screen
PLAIN PASS: the start path changes the screen
```
Indented lines: `first snapshot ...`, `lines ...`, `files EQUAL`, `stderr: ...` and `two runs without a path: same screen` (two spaces). The verdict lines `WINDOW CASES PASS`, `AC-3 PASS`, `PLAIN PASS: ...` start at column 0.
Package record step conclusion: success ("Installed X and GL packages (for the record)"). Its output: libxkbcommon-x11-0, libxcb-xkb1, libegl-mesa0 `no packages found`; xvfb 2:21.1.12-1ubuntu1.9 ii; libgl1-mesa-dri, libglx-mesa0, mesa-libgallium 25.2.8-0ubuntu0.24.04.4 ii; `dpkg-query: exit status 1`; `APT LISTS: 58 files in /var/lib/apt/lists`; `libGLX_mesa.so.0 (libc6,x86-64) => /lib/x86_64-linux-gnu/libGLX_mesa.so.0`.

## window-probe (macos-15) (success)
Logs read: tail 120 lines. Step "Window probe": success (21:47:31Z-21:47:36Z). The probe script in this run also has a SIGKILL-after-5s branch (`PROBE: SIGTERM ignored for 5 s, sent SIGKILL`); that line is not printed in this run.
```
PROBE OS: macos-15 Darwin ... 24.6.0 ... arm64
ProductVersion:		15.8.2
BuildVersion:		24H232
PROBE: exit status 0
PROBE: stdout     1864 bytes, 3 lines
--- stdout
(3 JSON lines: s1 snapshot, {"id":1,"ok":true,"result":{"docs":[]}}, s2 snapshot)
--- stderr
--- end
```
The `PROBE:` lines are exactly `PROBE: exit status 0` and `PROBE: stdout     1864 bytes, 3 lines`. stderr empty.

## window-probe (windows-2025) (failure at Build kiln)
Logs read: tail 120 lines. Build kiln: `error: could not compile `kiln-workspace` (lib) due to 6 previous errors`, then `##[error]Process completed with exit code 101.` The "Window probe" step still ran (conclusion success, 21:48:37Z-21:48:38Z, env `BUILD_OUTCOME: failure`) and recorded:
```
PROBE OS: windows-2025 MINGW64_NT-10.0-26100 runnervmm7ud1 3.6.10-5a1665c8.x86_64 2026-09-28 10:40 UTC x86_64 Msys
PROBE: not run (cargo build -p kiln: failure)
```
The only `PROBE:` line is `PROBE: not run (cargo build -p kiln: failure)`; no exit status, no stdout/stderr.

## Compared with round 1 (figures for round 1 from T-070-round-1.md)
| job | round 1 | round 2 |
|---|---|---|
| gate-linux | success, 15m34s | success, 10m06s |
| gate-macos | failure, 21m20s | failure, 26m06s |
| gate-windows | failure, 9m15s | failure, 9m28s |
| window-linux | success, 2m13s | success, 1m01s |
| window-probe (macos-15) | success, 2m26s | success, 1m17s |
| window-probe (windows-2025) | failure (Build kiln), 2m08s | failure (Build kiln), 2m13s |
- gate-linux test line: round 1 `990 passed, 0 failed, 0 ignored in 120 suites`; round 2 `999 passed, 0 failed, 0 ignored in 121 suites`. GATE PASSED 647.6s vs 396.6s.
- gate-macos: round 1 failed steps scorecard and test (`GATE FAILED in 355.1s`); round 2 failed steps scorecard and test (`GATE FAILED in 488.3s`). Scorecard problem list identical (7 lines). xtask core crates: 116 passed, 3 failed in both. Printed-prefix totals 366 passed / 11 failed in both.
- gate-windows: failed steps clippy, schema, scorecard, test in both (`GATE FAILED in 194.7s` vs `197.1s`); kiln-workspace 6 / 34 errors and xtask 4 errors in both.
- window-linux: same verdict lines (WINDOW CASES PASS, AC-3 PASS, PLAIN PASS) in both.
- Probe macOS: same output (exit status 0, 1864 bytes, 3 lines) in both. Probe windows: same (not run).
- Disk: gate-linux du 8.6G (both); gate-macos 4.4G (both); gate-windows 2.5G (round 1) vs 2.6G (round 2).

## Not read
- gate-macos: names of the 3 failing xtask tests (the step prints only `test result:` lines); failing tests beyond the 200-line cap of the Full test output step (artifact test-output.txt not downloaded).
- gate-windows: the Full test output step prints no compile errors (filtered output empty); the E0433 error for disk.rs:86 is named only through the `Some errors have detailed explanations` line style in the first run's build log; in this run's clippy detail I found the five E0599 error lines and the 6-error total but did not locate a separate E0433 line for disk.rs.
- Step start of logs (checkout, toolchain, cache) not read beyond what the tails covered; artifacts not downloaded.
- Waiting: the run was checked at 22:06Z (in progress), 22:10Z (in progress) and found completed after the next wait; the 30-minute limit was not reached.
