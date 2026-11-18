# CI run 44253737976 (platforms.yml) on ci/candidate @ 6dd4cd09364703454881766b06c9d539ba43bab9 — T-079 round 2

Record by run R-335 (planner of ADR-0009); method as `tasks/ci-runs/T-079-round-1.md` (artefacts first).

## Run header
- id 44253737976, run_number 20, attempt 1, event push, workflow `platforms`, title "Join task/T-079-tests into task/T-079 (after review round 1)"
- branch `ci/candidate`; commit `6dd4cd09364703454881766b06c9d539ba43bab9`
- created_at 2026-11-18T06:45:48Z; updated_at 2026-11-18T07:13:52Z; status completed, conclusion failure (gate-windows and window-probe (windows-2025) only)

## Job table
| job | id | conclusion | wall | first failing step |
|---|---|---|---|---|
| gate-linux | 134630335768 | **success** | 15m28s (06:45:51Z–07:01:19Z) | none |
| gate-macos | 134630335413 | **success** | 28m00s (06:45:51Z–07:13:51Z) | none ("Full test output (gate failed)" skipped) |
| gate-windows | 134630335845 | failure | 18m53s (06:45:51Z–07:04:44Z) | Gate (06:46:19Z–06:49:29Z) |
| window-linux | 134630335656 | **success** | 1m54s (06:45:51Z–06:47:45Z) | none |
| window-probe (macos-15) | 134630335696 | **success** | 1m36s (06:45:51Z–06:47:27Z) | none |
| window-probe (windows-2025) | 134630335885 | failure | 2m11s (06:45:51Z–06:48:02Z) | Build kiln (06:46:21Z–06:47:58Z) |

Artefacts (6): `gate-macos-…` 10,372 B, `gate-windows-…` 17,557 B, `gate-linux-…` 10,314 B, `window-linux-…` 577 B,
`window-probe-macos-15-…` 703 B, `window-probe-windows-2025-…` 292 B.
Fetched with `download_workflow_run_artifact` into `/home/user/kiln-plan-T-079-artefact/` (deleted after this record),
each zip's size equal to `size_in_bytes`, `testzip()` clean; read as data. **`gate-macos` has no `test-output.txt` this
time**: the gate passed, so the "Full test output (gate failed)" step was skipped; per-test names on macOS come only from
`core-crates.txt` (`xtask` and the core crates, printed in full) and from what the `scorecard` step's verdict implies (below).

Counted jobs' verdicts: `window-linux` — `WINDOW CASES PASS`, `AC-3: exit=2 stdout_bytes=0 stderr_lines=1 panicked=0 (pass: 2
0 1 0)`, `AC-3 PASS`, `PLAIN PASS: the start path changes the screen`; `window-probe (macos-15)` — `PROBE: exit status 0`,
`PROBE: stdout     2174 bytes, 3 lines`. (`window-probe (windows-2025)`: `PROBE: not run (cargo build -p kiln: failure)`.)

## 1. gate-macos — **green**
Gate report: `passed: true`, `failed_step: null`, 1,193.7 s. fmt PASS "formatted"; clippy PASS "no warnings (all targets,
-D warnings)"; layers PASS; ui-commands PASS; hygiene PASS "reviews: 56 of 68 …; session-paths: 678 tracked file(s)
clean"; schema PASS "30 commands; audit ok; docs/commands.json current"; **scorecard PASS "macos: 64 items; 216 tests ran;
3 registered absent here; page not compared (computed on Linux)"** (one line, no problem after it); **test PASS "1117
passed, 0 failed, 0 ignored in 133 suites"**; no-skip PASS "self-test ok (17 known findings); 199 files scanned, 0
allowlisted skip(s), **20 platform test(s) registered (8 pending)**, today = 2026-11-18"; fuzz, budgets EMPTY.

`acceptance_t019_cli` (7 tests: `tree_equals_git_ls_files`, `tree_order_and_counts`, `memory_and_disk_trees_agree`,
`symlinks_are_listed_not_followed`, `limits_and_refusals`, `tree_under_a_path_applies_prefix_gitignores`,
`deep_trees_stop_at_the_depth_bound`; read from the source at `6dd4cd0`): all 7 are listed in
`scorecard/items/ws.open-project.toml`, and the macOS `scorecard` step ran the listed tests with **no problem** (a failing,
ignored or missing listed test is a problem there, `docs/development.md` `scorecard` row) — so each of the 7 ran and
passed on macOS. Its own `test result:` line is **not available** (no full output uploaded on a green gate); the gate's
`test` step counts 133 suites with 0 failed (round 1: 23 suites before the abort).

## 2. The 16 formerly failing macOS tests
| test | macOS | evidence |
|---|---|---|
| `ac1_no_display_every_window_form`, `ac1_no_display_exits_2_with_one_line`, `ac1_wayland_only_or_unreachable_display_exits_2` | absent (registered `linux`) | listed in `ui.window`; scorecard "3 registered absent here", no problem |
| `disk_refusals_through_control` | ok | listed in `control.errors`; scorecard no problem |
| `deep_trees_stop_at_the_depth_bound`, `tree_order_and_counts` | **ok** | listed in `ws.open-project`; scorecard no problem |
| `ac8_stdout_write_failure_exits_1`, `run_and_control_exit_1_when_stdout_cannot_be_written`, `run_exits_1_when_stdout_cannot_be_written`, `without_a_display_the_window_exits_2_with_one_line`, `kiln_without_subcommand_and_display_exits_nonzero_with_a_message` | not failing; absent by name **not determinable** | not in any scorecard item; `test` step 0 failed |
| `tree_counts_skipped_names_on_disk`, `disk_list_dir_reports_entries_by_their_own_kind_and_skips_bad_names`, `documented_differences_from_git`, `disk_list_dir_reports_own_kinds_sorted_and_skips_odd_names`, `disk::tests::failed_rename_removes_own_temp_and_keeps_target` | not failing; ok by name **not determinable** | as above |
No test failed or aborted on macOS (`test` "1117 passed, 0 failed, 0 ignored in 133 suites").

## 3. `xtask` (`core-crates.txt`)
| | Linux | macOS | Windows |
|---|---|---|---|
| exit status line | `==== xtask: exit status 0` | `==== xtask: exit status 0` | `==== xtask: exit status 0` |
| binaries run | 23 | 23 | 23 |
| passed / failed | 286 / 0 | 281 / 0 | 278 / 0 |
| `acceptance_t079` | 2 / 0 | 2 / 0 | 2 / 0 |
| `guard_t079` | 6 / 0 | 6 / 0 | 6 / 0 |
Core crates `kiln-text`, `kiln-syntax`, `kiln-search`: exit status 0 on all three. The 8 registered `xtask` tests by name
(`core-crates.txt`): Linux all 8 `ok`; macOS the 3 `unix` ones `ok`, the 5 `linux` ones absent; Windows all 8 absent —
exactly as their `platforms`.

## 4. Windows — unchanged from round 1
Gate red at clippy, schema, scorecard, test (`passed: false`, failed_step clippy); no-skip PASS (20 registered, 8 pending);
scorecard `windows: 64 items; 18 tests ran; 3 registered absent here; page not compared (computed on Linux)`, then `199
scorecard problems:`; does not compile: `kiln-workspace` (lib) "due to 6 previous errors", (lib test) "due to 34 previous
errors" (T-073); `xtask` 278 / 0, exit status 0. Same values as round 1.

## 5. Against T-079's pending CI criterion
| Clause | Verdict | Quote |
|---|---|---|
| `gate-linux` green | **met** | job success; `passed: true`; test "1134 passed, 0 failed, 0 ignored in 133 suites" |
| `window-linux`, `window-probe (macos-15)` success | **met** | `WINDOW CASES PASS`, `AC-3 PASS`, `PLAIN PASS`; `PROBE: exit status 0` |
| `gate-macos` green, every step PASS or EMPTY | **met** | `passed: true`; all steps PASS, fuzz and budgets EMPTY |
| scorecard line first, no problem after it | **met** | `macos: 64 items; 216 tests ran; 3 registered absent here; page not compared (computed on Linux)` (the whole detail) |
| full test output 0 failed, none of the 12 registered names in it | met (0 failed) / **not determinable** (names) | `test` "1117 passed, 0 failed"; no full output uploaded on a green gate, so the 12 names cannot be looked up |
| `gate-macos` `xtask` exit status 0, the 5 `linux` `xtask` entries absent by name | **met** | `==== xtask: exit status 0` (281 / 0); in `core-crates.txt` the 5 `linux` entries have no `test` line, the 3 `unix` ones are `ok` |
| `gate-windows` red at `kiln-workspace` | **met** (as expected) | `could not compile kiln-workspace (lib) due to 6 previous errors` |
| `gate-windows` `xtask` exit status 0, every binary run, the 8 absent by name | **met** | `==== xtask: exit status 0`; 23 binaries; 278 / 0; none of the 8 has a `test` line |

## 6. Differences from round 1
- macOS: gate red → **green**. `acceptance_t019_cli` aborted (stack overflow, 2 of 7 with a verdict) → all 7 pass (scorecard);
  scorecard 5 problems → none; `test` step "224 passed, 0 failed … in 23 suites" (then the abort) → "1117 passed, 0 failed, 0
  ignored in 133 suites". `xtask` unchanged (281 / 0).
- Linux: unchanged (test 1134 / 0 in 133 suites; `xtask` 286 / 0).
- Windows: unchanged (see 4).

## 7. Not read
- The full macOS test output (not uploaded: the gate passed), hence the per-test verdict lines of `acceptance_t019_cli`
  and the presence or absence by name of the 9 of R-285's 16 that are in no scorecard item (and so the 12 registered
  product names on macOS).
- Job logs.
