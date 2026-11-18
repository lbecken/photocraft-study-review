# CI run 44279298877 (platforms.yml) on ci/candidate @ 5858927aacfedc2d3973ee115a41687a9637d80b — T-073 round 2

Record by run R-347; method as `tasks/ci-runs/README.md` (artefacts first).

## Run header
- id 44279298877, run_number 23, attempt 1, event push, workflow `platforms`, title "T-073: implementer log: gate green, CI-REQUEST"
- branch `ci/candidate`; commit `5858927aacfedc2d3973ee115a41687a9637d80b`; created_at 2026-11-18T09:47:58Z; updated_at 2026-11-18T10:15:09Z
- status `completed`, conclusion **failure** (because of `gate-windows` and `window-probe (windows-2025)`); found completed at the first listing, no wait

## Job table
Wall time = `completed_at` − `started_at` of the job.
| job | id | conclusion | wall | first failing step |
|---|---|---|---|---|
| gate-linux | 134715435831 | **success** | 15m21s (09:48:00Z–10:03:21Z) | none |
| gate-macos | 134715435998 | **success** | 27m09s (09:48:00Z–10:15:09Z) | none |
| gate-windows | 134715435321 | failure | 18m10s (09:48:00Z–10:06:10Z) | Gate (09:48:25Z–09:54:13Z); the steps after it (Gate report, Core crates, Workspace lib and T-073 tests, Full test output, Disk, Upload reports) all success |
| window-linux | 134715435942 | **success** | 1m20s (09:48:00Z–09:49:20Z) | none |
| window-probe (macos-15) | 134715435749 | **success** | 1m16s (09:48:01Z–09:49:17Z) | none |
| window-probe (windows-2025) | 134715435916 | failure | 1m48s (09:48:00Z–09:49:48Z) | **Window probe** (09:49:42Z–09:49:43Z); "Build kiln" **success** (09:48:24Z–09:49:42Z) |

Artefacts (`list_workflow_run_artifacts`; each fetched with `download_workflow_run_artifact` and curl, size equal to
`size_in_bytes`, `testzip()` clean; unpacked in a new directory, deleted after this record): `gate-windows-44279298877` 16,091 B
(`gate-report.json`, `core-crates.txt`, `test-output.txt`, `workspace-tests.txt`), `gate-linux-…` 10,353 B and `gate-macos-…`
10,290 B (`gate-report.json`, `core-crates.txt`), `window-linux-…` 577 B (`window-procedure.txt`), `window-probe-macos-15-…` 702 B
(the list gives 702; `stat` 702), `window-probe-windows-2025-…` 344 B (`window-probe.txt`).

Counted window jobs:
- `window-linux`: `WINDOW CASES PASS`; `AC-3: exit=2 stdout_bytes=0 stderr_lines=1 panicked=0 (pass: 2 0 1 0)`, `AC-3 PASS`;
  `PLAIN PASS: the start path changes the screen`.
- `window-probe (macos-15)`: `PROBE: exit status 0`; `PROBE: stdout     2174 bytes, 3 lines`; stderr empty.

## gate-linux and gate-macos (counted)
- `gate-linux` report `passed: true`, 654.9 s: fmt, clippy ("no warnings (all targets, -D warnings)"), layers, ui-commands, hygiene,
  schema, scorecard, test, no-skip PASS; fuzz, budgets EMPTY. Scorecard "64 items: 47 verified, 0 implemented, 0 failing, 17
  planned."; test "1147 passed, 0 failed, 0 ignored in 135 suites"; no-skip "self-test ok (17 known findings); 201 files scanned, 0
  allowlisted skip(s), 35 platform test(s) registered (19 pending), today = 2026-11-18".
- `gate-macos` (counted: **success**) report `passed: true`, 1,095.3 s: every step PASS (fuzz, budgets EMPTY); scorecard
  `macos: 64 items; 216 tests ran; 3 registered absent here; page not compared (computed on Linux)` (one line, no problems); test
  "1130 passed, 0 failed, 0 ignored in 135 suites"; no-skip "… 35 platform test(s) registered (19 pending) …". No failing test to list.

## gate-windows, step by step (`gate-report.json`, ANSI stripped)
`passed: false`, `failed_step: clippy`, 328.1 s. The report keeps only the last lines of a failing step's output (clippy 60
lines, test 27 lines), so a failing step's very first error lines are "not read" where the kept text starts mid-output.
| step | status | detail (first line) and what the failures name |
|---|---|---|
| fmt | PASS | formatted |
| clippy | **FAIL** | kept text starts at `--> apps\kiln-cli\tests\acceptance_t008_cli.rs:700:13` (E0599 `dev`/`ino` not found on `std::fs::Metadata`, lines 700–703), ends `error: could not compile `kiln-cli` (test "acceptance_t008_cli") due to 15 previous errors`; also `Some errors have detailed explanations: E0433, E0599.` — one crate and target named: `kiln-cli`, test target `acceptance_t008_cli` (a product **test** target; round 1 named `acceptance_t019_cli`) |
| layers | PASS | 11 crates, 14 internal edges, layer table holds |
| ui-commands | PASS | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | PASS | reviews: 58 of 70 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 685 tracked file(s) clean (0 not UTF-8, skipped) |
| schema | PASS | 30 commands; audit ok; docs/commands.json current |
| scorecard | **FAIL** | first line `windows: 64 items; 158 tests ran; 3 registered absent here; page not compared (computed on Linux)`; `60 scorecard problems:` of which 1 is `building test executables failed on windows (); their tests are not run`, 58 are `lists test …, which does not exist on windows` (kiln_cli, kiln_commands, kiln_workspace and kiln tests), and 1 is **`io.save-atomic: test `kiln_workspace::acceptance_t002::disk_save_replaces_via_rename_without_leftovers` fails on windows`** — a test that was built and ran and failed; no message for it in any artefact (see below) |
| test | **FAIL** | "0 passed, 0 failed, 0 ignored in 0 suites"; kept text: `error: could not compile `kiln` (test "acceptance_t065") due to 3 previous errors`, `(test "window_t065") due to 8 previous errors`, `(test "script_t055") due to 1 previous error` (E0433 `cannot find `unix` in `os``, `apps\kiln\tests\script_t055.rs:349:14`), `(test "acceptance_t055") due to 2 previous errors` — product **test** targets of `kiln` |
| no-skip | PASS | self-test ok (17 known findings); 201 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (19 pending), today = 2026-11-18 |
| fuzz | EMPTY | no targets registered yet (xtask/fuzz.toml) |
| budgets | EMPTY | no targets registered yet (xtask/budgets.toml) |

`test-output.txt` (172 lines, the whole-workspace output of the failed gate): errors in `apps\kiln\tests\acceptance_t065.rs`,
`window_t065.rs`, `script_t055.rs`, `acceptance_t055.rs` only (E0433 `unix`, E0277 `[OsString]`, E0599 `from_vec`, `from_bytes`);
final lines `could not compile kiln (test "window_t065") due to 8 previous errors`, `(test "acceptance_t065") due to 3`,
`(test "script_t055") due to 1`, `(test "acceptance_t055") due to 2`. **No error in `test-output.txt` or in the kept clippy,
scorecard or test text names a library or a binary; every named target is a test target** (`kiln-cli` `acceptance_t008_cli`;
`kiln` `acceptance_t065`, `window_t065`, `script_t055`, `acceptance_t055`). None names `kiln-workspace`. Compile errors in
test targets reached after the first failing ones are not shown (cargo stops), and clippy's first lines are not kept.

The failure message of `acceptance_t002::disk_save_replaces_via_rename_without_leftovers`: **not read** (the scorecard step
prints only that it fails; `test-output.txt` stops at the build errors; `workspace-tests.txt` ran only `--lib`, `acceptance_t073`,
`guard_t073`; `core-crates.txt` has no such test; no job log read).

## The reported step "Workspace lib and T-073 tests" (`workspace-tests.txt`)
Compiled `kiln-workspace`; `Finished `test` profile [unoptimized + debuginfo] target(s) in 4.92s`. Three binaries, all ok:
- `Running unittests src\lib.rs` — `running 36 tests` — `test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.68s`
- `Running tests\acceptance_t073.rs` — `running 1 test` — `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`:
  `acq4_same_file_is_declared_only_as_windows_dependency_at_exact_version ... ok`
- `Running tests\guard_t073.rs` — `running 10 tests` — `test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s`;
  each `ok`: `acw2_missing_root_is_refused`, `acw2_file_as_root_is_refused`, `acw2_existing_root_is_accepted_and_used`,
  `acw2_save_creates_temp_exclusively`, `acw2_root_parent_replaced_is_outside_root`,
  `acw2_save_of_new_file_creates_it_and_leaves_no_temp`, `acw2_save_keeps_writable_target_writable`,
  `acw2_root_replaced_at_same_path_is_outside_root`, `acw2_save_of_read_only_target_is_refused_and_changes_nothing`,
  `acw2_save_replaces_target_by_rename_and_leaves_no_temp`.
Totals: 36 + 1 + 10 = 47 passed, 0 failed (47 `test … ... ok` lines, none other).
The new unit test: `test disk::tests::held_root_check_is_inside_only_for_a_read_identity_equal_to_the_held_one ... ok`
(also in the `successes:` list). The other 35 lib tests are not listed one by one here: the artefact's per-test lines for the lib
binary are the `successes:` list (35 + the new one), all counted in `36 passed`.
`MEASURED:` lines, exactly as printed:
```
MEASURED: acw2_root_parent_replaced_is_outside_root: windows: rename of p while the root is held: refused: kind PermissionDenied, raw OS error Some(5) (Access is denied. (os error 5))
MEASURED: acw2_root_replaced_at_same_path_is_outside_root: windows: rename of root while the root is held: ok
```
No test failed (no failure message to quote).

## window-probe (windows-2025) (`window-probe.txt`)
"Build kiln" **success** (09:48:24Z–09:49:42Z); the "Window probe" step failed (it exits with `kiln`'s status):
```
PROBE OS: windows-2025 MINGW64_NT-10.0-26100 runnervmm7ud1 3.6.10-5a1665c8.x86_64 2026-09-28 10:40 UTC x86_64 Msys
PROBE: exit status 2
PROBE: stdout 0 bytes, 0 lines
--- stderr
kiln: no window: egui_glow: OpenGL: egui_glow requires opengl 2.0+.
```
Exit 2 with one stderr line.

## xtask (`core-crates.txt`) and no-skip
| platform | `==== xtask: exit status` | binaries run | totals | no-skip registered |
|---|---|---|---|---|
| Linux | 0 | 23 | 287 passed, 0 failed | 35 (19 pending) |
| macOS | 0 | 23 | 282 passed, 0 failed | 35 (19 pending) |
| Windows | 0 | 23 | 279 passed, 0 failed | 35 (19 pending) |
(Totals: `awk` on the `test result:` lines after `==== cargo test … -p xtask` in each ANSI-stripped file.) `kiln-text`,
`kiln-syntax`, `kiln-search` each `exit status 0` on all three platforms.

## Against T-073's pending CI criterion
| Clause | Verdict | Quote |
|---|---|---|
| `gate-linux`, `gate-macos` green | **met** | `passed: true` both; "1147 passed, 0 failed, 0 ignored in 135 suites" / "1130 passed, 0 failed …" |
| `window-linux`, `window-probe (macos-15)` green | **met** | `WINDOW CASES PASS`, `AC-3 PASS`, `PLAIN PASS …`; `PROBE: exit status 0` |
| `gate-windows`: fmt, layers, ui-commands, hygiene, schema, no-skip PASS | **met** | the six steps PASS; schema "30 commands; audit ok; docs/commands.json current"; no-skip "35 platform test(s) registered (19 pending)" |
| clippy, scorecard, test FAIL | **met** | all three `fail` |
| … each only with errors in test targets other than `kiln-workspace`'s lib test and `acceptance_t073`, named | **met as far as printed, with one item to rule on** | compile errors: clippy `kiln-cli (test "acceptance_t008_cli")`; test and full output `kiln` tests `acceptance_t065`, `window_t065`, `script_t055`, `acceptance_t055`; no library, binary, or `kiln-workspace` target named. The scorecard also names a **failing run** (not a compile error): ``kiln_workspace::acceptance_t002::disk_save_replaces_via_rename_without_leftovers` fails on windows`` — a `kiln-workspace` test target other than the lib test and `acceptance_t073` (so inside the wording), but it is a test failure at run time, not an error, and its message is not read |
| core-crates `xtask` exit status 0 | **met** | `==== xtask: exit status 0` (279 / 0) |
| the new step naming every `acceptance_t073` and `kiln-workspace` lib test `ok` | **met** | lib 36/0, `acceptance_t073` 1/0, `guard_t073` 10/0 |
| `window-probe (windows-2025)`: "Build kiln" success and a `PROBE:` exit-status line | **met** | Build kiln success; `PROBE: exit status 2` |

## Differences from round 1 (`tasks/ci-runs/T-073-round-1.md`)
- Windows: reported step 35 → 36 lib tests (the new one `ok`), 46 → 47 total, `MEASURED:` lines identical; no-skip 23 → 19 pending
  (35 registered both); clippy's named target `acceptance_t019_cli` (3 errors) → `acceptance_t008_cli` (15 errors); test step names
  `script_t055`, `acceptance_t055` as before and, in its kept text, also `acceptance_t065` and `window_t065` (round 1 had them only in
  the full output); scorecard "158 tests ran … 60 scorecard problems" unchanged in count, and now includes the line that
  `kiln_workspace::acceptance_t002::disk_save_replaces_via_rename_without_leftovers` fails on windows (round 1's record does not
  mention such a line; whether it was in round 1's 60 is not determinable from that record); `xtask` 279 unchanged; schema PASS as before;
  probe unchanged (exit 2, no OpenGL 2.0+).
- macOS: green as before; test 1129 → 1130 passed; `xtask` 282 unchanged; no-skip 23 → 19 pending.
- Linux: green as before; test 1146 → 1147 passed; `xtask` 287 unchanged; no-skip 23 → 19 pending.

## Not read
- The message of the failing `acceptance_t002::disk_save_replaces_via_rename_without_leftovers` on Windows, and whether it also failed in round 1.
- The first lines of clippy's, scorecard's and test's failing output (the report keeps only the last 60 / 27 lines); every failing Windows test target.
- Per-test names for the 35 other lib tests and for macOS and Linux product tests (a green gate uploads no `test-output.txt`).
- Job logs (all artefacts uploaded).
