# CI run 44215218474 (platforms.yml) on ci/candidate @ d00e21d5901ce6c35124f985ed3bd3638044c96d — T-071 round 3

Record by run R-318 (planner of ADR-0009) under METHODOLOGY v1.5 §8.1 "CI records"; method as
`tasks/ci-runs/T-071-round-1.md`. The superseded run 44213781136 (`88f38ca…`) is not read.

## Run header
- id 44215218474, run_number 16, attempt 1, event push, workflow `platforms`, title "Join task/T-071-tests into task/T-071 (the corrected s1 test)"
- branch `ci/candidate`; commit `d00e21d5901ce6c35124f985ed3bd3638044c96d`
- created_at 2026-11-18T01:21:46Z; updated_at 2026-11-18T01:51:11Z; status completed, **conclusion failure**

## Job table
| job | id | conclusion | wall (completed − started) | first failing step |
|---|---|---|---|---|
| gate-linux | 134503241249 | **success** | 19m27s (01:21:49Z–01:41:16Z) | none |
| gate-macos | 134503241201 | failure | 29m22s (01:21:49Z–01:51:11Z) | Gate (01:22:03Z–01:30:25Z); Upload reports success |
| gate-windows | 134503241075 | failure | 18m22s (01:21:49Z–01:40:11Z) | Gate (01:22:16Z–01:25:18Z) |
| window-linux | 134503240653 | **success** | 2m00s (01:21:49Z–01:23:49Z) | none |
| window-probe (macos-15) | 134503241098 | **success** | 1m56s (01:21:50Z–01:23:46Z) | none |
| window-probe (windows-2025) | 134503241758 | failure | 2m04s (01:21:48Z–01:23:52Z) | Build kiln (01:22:14Z–01:23:47Z) |

Artefacts (6, all uploaded this time): `gate-macos-…` 33,563 B, `gate-linux-…` 10,114 B, `gate-windows-…` 17,891 B,
`window-probe-macos-15-…` 703 B, `window-linux-…` 577 B, `window-probe-windows-2025-…` 292 B.

Sources: Actions API (`list_workflow_runs`, `list_workflow_jobs`, `list_workflow_run_artifacts`,
`download_workflow_run_artifact`); all six artefacts fetched 2026-11-18 about 01:52Z into
`/home/user/kiln-plan-T-071-artefact/` (deleted after this record), each zip's size equal to the API's
`size_in_bytes` and `testzip()` clean; read as data. No job log was needed.

## 1. Gate steps (`gate-report.json`)
| step | Linux | macOS | Windows |
|---|---|---|---|
| fmt | PASS | PASS | PASS |
| clippy | PASS "no warnings (all targets, -D warnings)" | PASS 59.2 s | **FAIL** (compiler notes; `kiln-workspace`) |
| layers | PASS | PASS | PASS |
| ui-commands | PASS | PASS | PASS |
| hygiene | PASS "52 of 64 review file(s) …; 654 tracked file(s) clean" | PASS | PASS |
| schema | PASS "30 commands; audit ok; docs/commands.json current" | PASS | **FAIL** "kiln-cli commands --audit failed:" |
| scorecard | PASS "64 items: 47 verified, 0 implemented, 0 failing, 17 planned." | **FAIL** | **FAIL** |
| test | PASS "1122 passed, 0 failed, 0 ignored in 131 suites" | **FAIL** "25 passed, 1 failed, 0 ignored in 2 suites" (`ac8_stdout_write_failure_exits_1`) | **FAIL** "0 passed, 0 failed, 0 ignored in 0 suites" (`is_fifo` … `disk.rs:1353`) |
| no-skip | PASS | PASS | PASS |
| fuzz, budgets | EMPTY | EMPTY | EMPTY |
| report | `passed: true`, 846.8 s | `passed: false`, failed_step scorecard, 487.1 s | `passed: false`, failed_step clippy, 158.4 s |

`no-skip` detail, identical on all three: "self-test ok (17 known findings); 197 files scanned, 0
allowlisted skip(s), **8 platform test(s) registered (0 pending)**, today = 2026-11-18".

`scorecard`, first line printed, then problems:
- Linux: "64 items: 47 verified, 0 implemented, 0 failing, 17 planned." (no problems).
- macOS: "**macos: 64 items; 219 tests ran; 0 registered absent here; page not compared (computed on Linux)**",
  then "6 scorecard problems:" — `kiln_cli::guard_t038_cli::disk_refusals_through_control`,
  `kiln::acceptance_t065::ac1_no_display_exits_2_with_one_line`, `…::ac1_no_display_every_window_form`,
  `…::ac1_wayland_only_or_unreachable_display_exits_2`, `kiln_cli::acceptance_t019_cli::tree_order_and_counts`,
  `…::deep_trees_stop_at_the_depth_bound`, each "fails on macos". No "docs/scorecard.md is stale".
- Windows: "**windows: 64 items; 18 tests ran; 0 registered absent here; page not compared (computed on Linux)**",
  then "202 scorecard problems:" — 1 × "building test executables failed on windows (); their tests are not run"
  and 201 × "lists test `…`, which does not exist on windows" (largest groups: ws.path-confinement 28,
  ui.editor-view 22, ui.tabs 14, ui.project-panel 12, ui.status-bar 9).

## 2. `xtask` per platform (`core-crates.txt`, `cargo test --locked --no-fail-fast -p xtask`)
21 binaries on each platform (unit binary, 20 integration binaries incl. two support files with 0 tests); every
`Running` line is followed by a `test result:` line on all three — **every binary compiled and ran, on Windows too**.
| binary | Linux | macOS | Windows |
|---|---|---|---|
| unittests src/main.rs | ok 138 | ok 136 | ok 134 |
| tests/acceptance_t005.rs | ok 16 | ok 16 | **FAILED 15 passed, 1 failed** |
| tests/acceptance_t005_r2.rs | ok 4 | ok 4 | ok 4 |
| tests/acceptance_t005_r3.rs | ok 11 | ok 10 | ok 10 |
| tests/acceptance_t005_r4.rs | ok 6 | ok 5 | ok 5 |
| tests/acceptance_t005_support.rs | ok 0 | ok 0 | ok 0 |
| tests/acceptance_t013_docs.rs | ok 2 | ok 2 | ok 2 |
| tests/acceptance_t013_hygiene.rs | ok 5 | ok 5 | ok 5 |
| tests/acceptance_t013_metrics.rs | ok 9 | ok 9 | ok 9 |
| tests/acceptance_t013_publish.rs | ok 11 | ok 10 | ok 10 |
| tests/acceptance_t013_queue.rs | ok 5 | ok 5 | ok 5 |
| tests/acceptance_t013_report.rs | ok 4 | ok 4 | ok 4 |
| tests/acceptance_t013_support.rs | ok 0 | ok 0 | ok 0 |
| tests/acceptance_t054.rs | ok 7 | ok 7 | ok 7 |
| tests/acceptance_t070.rs | ok 13 | ok 13 | ok 13 |
| tests/acceptance_t071.rs | ok 28 | ok 28 | ok 28 |
| tests/guard_t005.rs | ok 3 | ok 3 | **FAILED 1 passed, 1 failed** |
| tests/guard_t005_r2.rs | ok 1 | ok 1 | ok 1 |
| tests/guard_t054.rs | ok 2 | ok 2 | ok 2 |
| tests/guard_t070.rs | ok 7 | ok 7 | ok 7 |
| tests/guard_t071.rs | ok 6 | ok 6 | ok 6 |
| **total** | **278 passed, 0 failed**; `==== xtask: exit status 0` | **273 passed, 0 failed**; `exit status 0` | **268 passed, 2 failed**; `==== xtask: exit status 101` |

Windows failures — **one cause, 2 tests**: a Windows path written unescaped into a TOML string of a toy
`Cargo.toml`.
- `acceptance_t005::seed_reuses_non_member_artifacts_only` (`xtask\tests\acceptance_t005.rs:719:9`):
  ```
  assertion `left == right` failed: generate-lockfile: error: missing escaped value, expected `b`, `e`, `f`, `n`, `r`, `\`, `"`, `x`, `u`, `U`
   --> seedmember\Cargo.toml:7:27
  7 | outsidedep = { path = "D:\a\photocraft-study-review\photocraft-study-review\target\tmp\t005-acc\ac12\outsidedep" }
  error: failed to load manifest for workspace member `D:\a\…\t005-acc\ac12\repo\seedmember`
    left: 101
   right: 0
  ```
- `guard_t005::seeded_worktree_builds_its_own_source` (`xtask\tests\guard_t005.rs:97:9`): the same message, with
  `…\t005-guard\ac10\outsidedep`.
The 22 round-1 failures (touches refusals, toy-repository conflicts, CRLF, separator, error 216) do not recur.

## 3. The 8 registered tests (`xtask/platform-tests.toml` at d00e21d)
| # | item | platforms | Linux | macOS | Windows |
|---|---|---|---|---|---|
| 1 | `queue.rs` `tests::basic_refusals_write_nothing` | linux | ok | absent | absent |
| 2 | `worktree.rs` `tests::create_seed_list_remove` | linux | ok | absent | absent |
| 3 | `publish.rs` `tests::a_failed_content_commit_in_step_four_exits_one` | unix | ok | ok | absent |
| 4 | `acceptance_t013_publish.rs` `publish_refuses_while_the_queue_lock_is_held_and_releases_it_for_the_queue` | linux | ok | absent | absent |
| 5 | `guard_t005.rs` `worktree_build_does_not_touch_main_target` | unix | ok | ok | absent |
| 6 | `acceptance_t005_r3.rs` `tampered_cached_gate_binary_does_not_decide_the_gate` | linux | ok | absent | absent |
| 7 | `acceptance_t005_r4.rs` `planted_future_dated_xtask_does_not_decide_the_gate` | linux | ok | absent | absent |
| 8 | `queue.rs` `tests::main_xtask_is_rebuilt_from_base_on_every_run` | unix | ok | ok | absent |
Each present or absent exactly as its `platforms` says (by name, `test <name> ... ok` lines). Counts agree: unit
binary 138 / 136 / 134.

## 4. `guard_t054` and `acceptance_t054`
On all three: `acceptance_t054` `running 7 tests` … `ok`, 7 passed; `guard_t054` `running 2 tests`:
`test ac2_panic_location_is_this_file_and_line ... ok`, `test ac2_backtrace_names_this_file_and_line ... ok`,
2 passed. **Windows: both `ok`** (`gate-windows` artefact, `core-crates.txt`, the `guard_t054` binary) — the first Windows verdict of the backtrace
guard (T-054 ruling Q-4 A).

## 5. T-071's and T-070's tests
`acceptance_t071` 28 passed, 0 failed; `guard_t071` 6 passed; `acceptance_t070` 13 passed; `guard_t070` 7 passed —
on Linux, macOS and Windows alike. No failure.

## 6. Workspace
- macOS (`test-output.txt`, whole output): 131 `test result:` lines, **1,101 passed, 16 failed**. The 16 failing
  names are exactly R-285's 16 product tests (T-079 Interface (1)): none new, none gone.
- Windows: compiles and passes — `kiln-text`, `kiln-syntax`, `kiln-search` (`exit status 0` each) and every `xtask`
  target. Does not compile: `kiln-workspace` (lib) "due to 6 previous errors" and (lib test) "due to 34 previous
  errors" (T-073), hence nothing above it.

## 7. Window jobs
- `window-linux` success: six `CASE` lines (ac2 0/0, ac2x3 3/3, dblclick 0/0, scroll100 0/0, ctrlq 0/0, noroot 0/0),
  `WINDOW CASES PASS`, `AC-3: exit=2 stdout_bytes=0 stderr_lines=1 panicked=0 (pass: 2 0 1 0)`, `AC-3 PASS`,
  `PLAIN PASS: the start path changes the screen`.
- `window-probe (macos-15)` success: `PROBE: exit status 0`, `PROBE: stdout     2174 bytes, 3 lines`.
- `window-probe (windows-2025)` failure at Build kiln: `PROBE: not run (cargo build -p kiln: failure)`.

## 8. Against T-071's AC-6
Text (main `9aa3ab2`): "AC-6: on the candidate, the `core-crates` step shows `xtask` exit status 0 on `gate-macos`
and on `gate-windows`; `gate-linux` green. The macOS gate stays red until T-079 (product tests) and Windows until
T-073, as `tasks/T-070-*.md` "Expected to FAIL" says; the scorecard step on macOS prints the platform detail line of
Interface (3)." Scorecard clause as ruling s10 words it: "off Linux the step's output names the platform and says the
page was not compared whether it passes or reports problems (when it reports problems, that line comes first)".
| Clause | Verdict | Quote |
|---|---|---|
| `xtask` exit status 0 on gate-macos | **met** | `==== xtask: exit status 0` (273 passed, 0 failed) |
| `xtask` exit status 0 on gate-windows | **not met** | `==== xtask: exit status 101`; 268 passed, 2 failed (`seed_reuses_non_member_artifacts_only`, `seeded_worktree_builds_its_own_source`) |
| gate-linux green | **met** | job success; `passed: true` |
| macOS gate red until T-079 (expected) | met | scorecard and test FAIL; the 16 product tests |
| Windows gate red until T-073 (expected) | met | `could not compile kiln-workspace (lib) due to 6 previous errors` |
| scorecard on macOS: platform line first, page not compared (s10) | **met** | first line `macos: 64 items; 219 tests ran; 0 registered absent here; page not compared (computed on Linux)`, then `6 scorecard problems:` (also on Windows: `windows: … page not compared (computed on Linux)` first) |

## 9. Not read
- Job logs (all artefacts uploaded; not needed).
- The superseded run 44213781136.
