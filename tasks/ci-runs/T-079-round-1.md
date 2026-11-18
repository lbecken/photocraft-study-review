# CI run 44245093360 (platforms.yml) on ci/candidate @ 885f4f1557d4aa1196e79e4136a1f6e1f55da6c4 — T-079 round 1

Record by run R-331 (planner of ADR-0009); method as `tasks/ci-runs/T-071-round-4.md` (artefacts first).

## Run header
- id 44245093360, run_number 19, attempt 1, event push, workflow `platforms`, title "Join task/T-079-tests into task/T-079"
- branch `ci/candidate`; commit `885f4f1557d4aa1196e79e4136a1f6e1f55da6c4`
- created_at 2026-11-18T05:33:37Z; updated_at 2026-11-18T06:01:24Z; status completed, **conclusion failure**

## Job table
| job | id | conclusion | wall | first failing step |
|---|---|---|---|---|
| gate-linux | 134602006547 | **success** | 11m42s (05:33:40Z–05:45:22Z) | none |
| gate-macos | 134602006616 | **failure** | 27m44s (05:33:39Z–06:01:23Z) | Gate (05:33:58Z–05:43:13Z) |
| gate-windows | 134602006510 | failure | 17m10s (05:33:41Z–05:50:51Z) | Gate (05:34:10Z–05:36:46Z) |
| window-linux | 134602006559 | **success** | 1m18s (05:33:39Z–05:34:57Z) | none |
| window-probe (macos-15) | 134602006539 | **success** | 1m12s (05:33:39Z–05:34:51Z) | none |
| window-probe (windows-2025) | 134602006506 | failure | 2m11s (05:33:40Z–05:35:51Z) | Build kiln (05:34:12Z–05:35:45Z) |

Artefacts (6): `gate-macos-…` 32,340 B, `gate-windows-…` 17,512 B, `gate-linux-…` 10,282 B, `window-probe-macos-15-…` 705 B,
`window-linux-…` 577 B, `window-probe-windows-2025-…` 292 B.
Fetched with `download_workflow_run_artifact` into `/home/user/kiln-plan-T-079-artefact/` (deleted after this record),
each zip's size equal to `size_in_bytes`, `testzip()` clean; read as data. Registry read with `git show
885f4f1:xtask/platform-tests.toml` (20 entries).

Counted jobs' verdicts: `window-linux` — six `CASE` lines (ac2 0/0, ac2x3 3/3, dblclick 0/0, scroll100 0/0, ctrlq 0/0,
noroot 0/0), `WINDOW CASES PASS`, `AC-3 PASS`, `PLAIN PASS: the start path changes the screen`; `window-probe (macos-15)` —
`PROBE: exit status 0`, `PROBE: stdout     2174 bytes, 3 lines`. (`window-probe (windows-2025)`: `PROBE: not run (cargo
build -p kiln: failure)`.)

## 1. gate-macos — **not green**
Gate report: `passed: false`, `failed_step: scorecard`, 534.4 s. fmt PASS; clippy PASS "no warnings (all targets, -D
warnings)"; layers PASS; ui-commands PASS; hygiene PASS "reviews: 55 of 67 …; session-paths: 677 tracked file(s) clean";
schema PASS "30 commands; audit ok; docs/commands.json current"; **scorecard FAIL**; **test FAIL**; no-skip PASS "self-test
ok (17 known findings); 199 files scanned, 0 allowlisted skip(s), **20 platform test(s) registered (8 pending)**, today =
2026-11-18"; fuzz, budgets EMPTY.

Scorecard, as printed:
```
macos: 64 items; 216 tests ran; 3 registered absent here; page not compared (computed on Linux)
5 scorecard problems:
  - ws.open-project: test `kiln_cli::acceptance_t019_cli::tree_equals_git_ls_files` fails on macos
  - ws.open-project: test `kiln_cli::acceptance_t019_cli::tree_order_and_counts` fails on macos
  - ws.open-project: test `kiln_cli::acceptance_t019_cli::memory_and_disk_trees_agree` fails on macos
  - ws.open-project: test `kiln_cli::acceptance_t019_cli::tree_under_a_path_applies_prefix_gitignores` fails on macos
  - ws.open-project: test `kiln_cli::acceptance_t019_cli::deep_trees_stop_at_the_depth_bound` fails on macos
```
Test step: "224 passed, 0 failed, 0 ignored in 23 suites", then the abort of one binary.

**One cause, one test binary:** `apps/kiln-cli/tests/acceptance_t019_cli.rs` aborts (full test output, lines 364–375):
```
     Running tests/acceptance_t019_cli.rs (target/debug/deps/acceptance_t019_cli-8420bf0f84ef0d75)

running 7 tests
test limits_and_refusals ... ok
test symlinks_are_listed_not_followed ... ok

thread 'deep_trees_stop_at_the_depth_bound' (278035) has overflowed its stack
fatal runtime error: stack overflow, aborting
error: test failed, to rerun pass `-p kiln-cli --test acceptance_t019_cli`

Caused by:
  process didn't exit successfully: `…/acceptance_t019_cli-8420bf0f84ef0d75` (signal: 6, SIGABRT: process abort signal)
```
The thread that overflows is the test's own (`deep_trees_stop_at_the_depth_bound`, one of R-285's 16, given a portable
fix by this task); the abort kills the binary, so the other four of its tests have no verdict and the scorecard reports
them as failing: `tree_order_and_counts` (one of R-285's 16) and `tree_equals_git_ls_files`, `memory_and_disk_trees_agree`,
`tree_under_a_path_applies_prefix_gitignores` (**new** in the problem list; in round 4 of T-071 they passed on macOS — not
failing by assertion here, just not run). What overflows (the test's tree builder or remover, or something it calls) is
**not determinable** from the artefact: no backtrace is printed.

Whole output: 132 `test result:` lines, 1,110 passed, 0 failed — the aborted binary has no `test result:` line, so its 5
missing verdicts are not in these totals.

## 2. The 16 formerly failing macOS tests (by name in the macOS `test-output.txt`)
| test | macOS now |
|---|---|
| `ac8_stdout_write_failure_exits_1`, `run_and_control_exit_1_when_stdout_cannot_be_written`, `run_exits_1_when_stdout_cannot_be_written` | absent (registered `linux`) |
| `ac1_no_display_every_window_form`, `ac1_no_display_exits_2_with_one_line`, `ac1_wayland_only_or_unreachable_display_exits_2`, `without_a_display_the_window_exits_2_with_one_line`, `kiln_without_subcommand_and_display_exits_nonzero_with_a_message` | absent (registered `linux`) |
| `tree_counts_skipped_names_on_disk`, `disk_list_dir_reports_entries_by_their_own_kind_and_skips_bad_names`, `documented_differences_from_git`, `disk_list_dir_reports_own_kinds_sorted_and_skips_odd_names` (portable halves, original names) | **ok** (4) |
| their `_non_utf8` parts (`…_non_utf8`, 4 entries) | absent (registered `linux`) |
| `disk_refusals_through_control` | **ok** (portable fix) |
| `disk::tests::failed_rename_removes_own_temp_and_keeps_target` | **ok** (portable fix) |
| `deep_trees_stop_at_the_depth_bound` | **aborts (stack overflow)** |
| `tree_order_and_counts` | no verdict (binary aborted) |
Linux: no `test-output.txt` (the gate passed: `test` "1134 passed, 0 failed, 0 ignored in 133 suites"), so product test
names on Linux are **not determinable** from the artefact; the `scorecard` step passed there ("64 items: 47 verified, 0
implemented, 0 failing, 17 planned.").

## 3. The 20 registered tests
- The 8 `xtask` ones (in `core-crates.txt`): exactly as their `platforms` — Linux all `ok`; macOS the 3 `unix` ones `ok`
  (`publish::tests::a_failed_content_commit_in_step_four_exits_one`, `worktree_build_does_not_touch_main_target`,
  `queue::tests::main_xtask_is_rebuilt_from_base_on_every_run`), the 5 `linux` ones absent; Windows all 8 absent.
- The 12 product ones (all `linux`): macOS all 12 absent by name (table above; the scorecard says "3 registered absent
  here", the 3 of them listed in scorecard items); Linux: not determinable by name (no test output uploaded on a green
  gate); Windows: nothing above `kiln-workspace` compiles.

## 4. `xtask` (`core-crates.txt`)
| | Linux | macOS | Windows |
|---|---|---|---|
| exit status line | `==== xtask: exit status 0` | `==== xtask: exit status 0` | `==== xtask: exit status 0` |
| binaries run | 23 | 23 | 23 |
| passed / failed | 286 / 0 | 281 / 0 | 278 / 0 |
| `acceptance_t079` | 2 passed | 2 passed | 2 passed |
| `guard_t079` | 6 passed | 6 passed | 6 passed |
`guard_t079` (all `ok` on all three): `ac2_failed_rename_removes_own_temp_and_keeps_target_has_no_platform_cfg`,
`ac2_deep_trees_stop_at_the_depth_bound_has_no_platform_cfg`, `ac2_disk_refusals_through_control_has_no_platform_cfg`,
`ac2_tree_order_and_counts_has_no_platform_cfg`, `base_list_is_the_pinned_measurement`,
`every_linux_test_name_of_the_base_still_exists`. `acceptance_t079`: `ac1_registry_entries_are_exactly_the_twenty_of_interface_2_and_2a`,
`ac1_pending_entries_name_their_task_and_expire_on_2026_12_31`, `ok` on all three. Core crates `kiln-text`, `kiln-syntax`,
`kiln-search`: exit status 0 on all three.

## 5. Windows
Gate: `passed: false`, failed_step clippy; FAIL at clippy, schema, scorecard, test; no-skip PASS (20 registered, 8 pending).
Scorecard first lines: `windows: 64 items; 18 tests ran; 3 registered absent here; page not compared (computed on Linux)`,
`199 scorecard problems:`, `- building test executables failed on windows (); their tests are not run` (202 in T-071
round 4). Compiles: the three core crates and every `xtask` target; does not: `kiln-workspace` (lib) "due to 6 previous
errors", (lib test) "due to 34 previous errors" (T-073). Among what ran nothing fails (`xtask` 278 / 0, core crates 0).

## 6. Against T-079's pending CI criterion
| Clause | Verdict | Quote |
|---|---|---|
| `gate-linux` green | **met** | job success; `passed: true` |
| `window-linux`, `window-probe (macos-15)` success | **met** | `WINDOW CASES PASS`, `AC-3 PASS`, `PLAIN PASS`; `PROBE: exit status 0` |
| `gate-macos` green, every step PASS or EMPTY | **not met** | `scorecard` and `test` FAIL |
| scorecard line first, no problem after it | **not met** | the line is first, but `5 scorecard problems:` follow (all `acceptance_t019_cli`) |
| full test output 0 failed, none of the 12 registered names in it | not met / met | 0 `FAILED` lines, but one binary aborted (stack overflow); none of the 12 names appears |
| `gate-macos` `xtask` exit status 0, the 5 `linux` `xtask` entries absent by name | **met** | `==== xtask: exit status 0` (281 / 0); the 5 absent |
| `gate-windows` red at `kiln-workspace` | **met** (as expected) | `could not compile kiln-workspace (lib) due to 6 previous errors` |
| `gate-windows` `xtask` exit status 0, every binary run, the 8 absent by name | **met** | `==== xtask: exit status 0`; 23 binaries; 278 / 0; the 8 absent |

## 7. Differences from `tasks/ci-runs/T-071-round-4.md`
- macOS: workspace 1,101 / 16 failed → 1,110 / 0 failed plus one aborted binary; of R-285's 16, 14 are now `ok` or absent
  as registered, `deep_trees_stop_at_the_depth_bound` aborts with a stack overflow, `tree_order_and_counts` has no
  verdict; three other `acceptance_t019_cli` tests lose their verdict (abort). Scorecard 6 → 5 problems (all
  `acceptance_t019_cli`), "0 registered absent here" → "3"; `no-skip` 8 → 20 registered (0 → 8 pending); `xtask` 273 / 0
  → 281 / 0 (23 binaries, `acceptance_t079` and `guard_t079` added).
- Windows: scorecard problems 202 → 199, "0" → "3 registered absent here"; `xtask` 270 / 0 → 278 / 0 (23 binaries).
- Linux: `test` 1122 → 1134 passed (133 suites); `xtask` 278 → 286 passed; `no-skip` 20 registered.

## 8. Not read
- Linux product test names (no `test-output.txt` on a green gate).
- What the overflowing thread was executing (no backtrace in the output); the verdicts of the four other
  `acceptance_t019_cli` tests on macOS.
- Job logs (all artefacts uploaded).
