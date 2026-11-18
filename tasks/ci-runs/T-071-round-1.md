# CI run 44201932507 (platforms.yml) on ci/candidate @ f7d511352027bdd8b0e48f207c3aac9ad5bbca85 — T-071 round 1

Record by run R-312 (planner of ADR-0009) under METHODOLOGY v1.5 §8.1 "CI records". Sources: the Actions API
(`list_workflow_runs`, `get_workflow_run`, `list_workflow_jobs`, `get_workflow_job`, `get_job_logs`) and the run's
uploaded artefacts (`list_workflow_run_artifacts`, `download_workflow_run_artifact`), fetched 2026-11-18 about
00:15Z into `/home/user/kiln-plan-T-071-artefact/` (deleted after this record) and read as data; after the
restart at 00:19:11Z each zip was re-checked whole (`zipfile.testzip()` None; sizes equal to the API's
`size_in_bytes`). **The `gate-macos` artefact does not exist**: its upload step failed (below), so every macOS
value here comes from the job log (`get_job_logs`, 701 lines, the whole log).

## Run header
- id 44201932507, run_number 14, attempt 1, event push, workflow `platforms`, title "Join task/T-071-tests into task/T-071"
- branch `ci/candidate`; commit `f7d511352027bdd8b0e48f207c3aac9ad5bbca85`
- created_at 2026-11-17T23:44:04Z; updated_at 2026-11-18T00:16:19Z; status completed, **conclusion failure**

## Job table
| job | id | conclusion | wall (completed − started) | first failing step |
|---|---|---|---|---|
| gate-linux | 134457963542 | **success** | 11m23s (23:44:06Z–23:55:29Z) | none |
| gate-macos | 134457963991 | failure | 32m12s (23:44:06Z–00:16:18Z) | Gate (23:44:22Z–23:53:07Z); also "Upload reports" failed |
| gate-windows | 134457963993 | failure | 7m54s (23:44:06Z–23:52:00Z) | Gate (23:44:40Z–23:47:14Z) |
| window-linux | 134457964001 | **success** | 2m09s (23:44:06Z–23:46:15Z) | none |
| window-probe (macos-15) | 134457963923 | **success** | 1m56s (23:44:07Z–23:46:03Z) | none |
| window-probe (windows-2025) | 134457963948 | failure | 2m01s (23:44:07Z–23:46:08Z) | Build kiln (23:44:33Z–23:46:02Z) |

gate-macos "Upload reports" (log): `##[error]Failed to CreateArtifact: Unable to make request: ETIMEDOUT`
(00:16:06Z), after `With the provided path, there will be 3 files uploaded`. Artefacts of the run (5):
`gate-linux-…` 9,732 B, `gate-windows-…` 15,908 B, `window-linux-…` 577 B, `window-probe-macos-15-…` 703 B,
`window-probe-windows-2025-…` 292 B.

## Registry at this commit
`git show f7d5113:xtask/platform-tests.toml`: 7 entries, all `counterpart = "none"`:
| # | file :: item | platforms |
|---|---|---|
| 1 | `xtask/src/queue.rs` :: `tests::basic_refusals_write_nothing` | linux |
| 2 | `xtask/src/worktree.rs` :: `tests::create_seed_list_remove` | linux |
| 3 | `xtask/src/publish.rs` :: `tests::a_failed_content_commit_in_step_four_exits_one` | unix |
| 4 | `xtask/tests/acceptance_t013_publish.rs` :: `publish_refuses_while_the_queue_lock_is_held_and_releases_it_for_the_queue` | linux |
| 5 | `xtask/tests/guard_t005.rs` :: `worktree_build_does_not_touch_main_target` | unix |
| 6 | `xtask/tests/acceptance_t005_r3.rs` :: `tampered_cached_gate_binary_does_not_decide_the_gate` | linux |
| 7 | `xtask/tests/acceptance_t005_r4.rs` :: `planted_future_dated_xtask_does_not_decide_the_gate` | linux |

## gate-linux (artefact: `gate-report.json`, `core-crates.txt`; no `test-output.txt`: the gate passed)
a. Gate report: `passed: true`, 485.2 s. fmt PASS "formatted"; clippy PASS "no warnings (all targets, -D warnings)";
layers PASS; ui-commands PASS; hygiene PASS "reviews: 51 of 63 review file(s) checked …; session-paths: 653 tracked
file(s) clean"; schema PASS "30 commands; audit ok; docs/commands.json current"; **scorecard PASS "64 items: 47
verified, 0 implemented, 0 failing, 17 planned."**; test PASS "1105 passed, 0 failed, 0 ignored in 131 suites";
**no-skip PASS "self-test ok (17 known findings); 197 files scanned, 0 allowlisted skip(s), 7 platform test(s)
registered (0 pending), today = 2026-11-17"**; fuzz, budgets EMPTY.
b. `xtask` (core-crates step, `==== xtask: exit status 0`): 21 `test result:` lines, **261 passed, 0 failed**. The
unit-test binary: `running 133 tests` … `test result: ok. 133 passed`.
c. Registered tests on Linux: all 7 ran and passed (each `test … ... ok` line found: entries 1–7).
d. `acceptance_t054`: `running 7 tests`, all `ok`, `test result: ok. 7 passed; 0 failed` (names:
`ac4_dev_incremental_is_false`, `ac4_dev_debug_is_line_tables_only`, `ac4_dev_dependencies_debug_is_false`,
`ac4_dev_is_the_only_profile_table`, `ac4_dev_tables_hold_only_the_interface_keys`,
`ac4_release_and_bench_profiles_are_absent`, `ac4_dev_sets_no_opt_level_debug_assertions_or_overflow_checks`).
`guard_t054`:
```
     Running tests/guard_t054.rs (target/debug/deps/guard_t054-7718025f58ade8b0)

running 2 tests
test ac2_panic_location_is_this_file_and_line ... ok
test ac2_backtrace_names_this_file_and_line ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```
e. `acceptance_t071`: `running 16 tests`, `test result: ok. 16 passed; 0 failed`. `guard_t071`: `running 6 tests`,
`test result: ok. 6 passed; 0 failed`. No failure.

## gate-macos (job log only; artefact upload failed)
a. Gate (log lines as printed): fmt PASS; clippy PASS 60.0s; layers PASS; ui-commands PASS; hygiene PASS; schema PASS
22.8s "30 commands; audit ok; docs/commands.json current"; **scorecard FAIL 416.7s "6 scorecard problems:"**
```
- control.errors: test `kiln_cli::guard_t038_cli::disk_refusals_through_control` fails on macos
- ui.window: test `kiln::acceptance_t065::ac1_no_display_exits_2_with_one_line` fails on macos
- ui.window: test `kiln::acceptance_t065::ac1_no_display_every_window_form` fails on macos
- ui.window: test `kiln::acceptance_t065::ac1_wayland_only_or_unreachable_display_exits_2` fails on macos
- ws.open-project: test `kiln_cli::acceptance_t019_cli::tree_order_and_counts` fails on macos
- ws.open-project: test `kiln_cli::acceptance_t019_cli::deep_trees_stop_at_the_depth_bound` fails on macos
```
No platform line "… page not compared (computed on Linux)" is printed (the detail is the problem list), and there
is **no "docs/scorecard.md is stale" problem** (runs 43765532298 and 43775682214 had one). test FAIL 3.4s "25
passed, 1 failed, 0 ignored in 2 suites" (`ac8_stdout_write_failure_exits_1`); **no-skip PASS "self-test ok (17
known findings); 197 files scanned, 0 allowlisted skip(s), 7 platform test(s) registered (0 pending), today =
2026-11-17"**; fuzz, budgets EMPTY; `GATE FAILED in 509.3s: scorecard, test`.
b. `xtask` (core-crates step; the log prints only `test result:` lines, no test names): `==== xtask: exit status 0`;
21 `test result:` lines, **256 passed, 0 failed**; first binary `131 passed`. `CORE CRATES: exit status per
package: kiln-text=0 kiln-syntax=0 kiln-search=0 xtask=0 (reported, not required)`.
c. Registered tests on macOS — by count only (names are not printed): the unit-test binary has 131 against Linux's
133 (entries 1 and 2, `linux`, absent; entry 3, `unix`, runs); `xtask` in all 256 against 261 = 5 fewer, equal to
the five `linux` entries (1, 2, 4, 6, 7). Consistent with the registry; not verifiable by name.
d. `guard_t054`, `acceptance_t054`: **not determinable by name** (no names in the log, no artefact); they are among
the 21 `xtask` binaries, all `ok` (0 failed).
e. `acceptance_t071`, `guard_t071`: as d — not determinable by name; no `xtask` test failed.
f. Whole workspace: the "Full test output" step printed only a prefix (`grep … | head -200`): 47 `test result:`
lines, 306 passed, 10 failed; failing tests named in the prefix (10): `ac8_stdout_write_failure_exits_1`,
`ac1_no_display_every_window_form`, `ac1_no_display_exits_2_with_one_line`,
`ac1_wayland_only_or_unreachable_display_exits_2`, `kiln_without_subcommand_and_display_exits_nonzero_with_a_message`,
`without_a_display_the_window_exits_2_with_one_line`, `run_and_control_exit_1_when_stdout_cannot_be_written`,
`deep_trees_stop_at_the_depth_bound`, `tree_order_and_counts`, `disk_refusals_through_control` — all among R-285's
16 product tests; none new. The whole set (whether the other 6 of the 16 still fail, or any new one) is **not
determinable**: `test-output.txt` was not uploaded. None of R-285's 6 `xtask` failures appears (xtask 0 failed).
`TEST OUTPUT: exit status 101`.

## gate-windows (artefact: `gate-report.json`, `core-crates.txt`, `test-output.txt`)
a. Gate report: `passed: false`, `failed_step: clippy`, 132.7 s. fmt PASS; **clippy FAIL** (compiler notes, "found an
item that was configured out"); layers PASS; ui-commands PASS; hygiene PASS; **schema FAIL** "kiln-cli commands
--audit failed" (`disk.rs:213` `dev()`/`ino()` … `method not found in Metadata`); **scorecard FAIL "202 scorecard
problems:"** — 1 × "building test executables failed on windows (); their tests are not run" and 201 × "…, which
does not exist on windows"; no platform line printed; test FAIL "0 passed, 0 failed, 0 ignored in 0 suites";
**no-skip PASS "self-test ok (17 known findings); 197 files scanned, 0 allowlisted skip(s), 7 platform test(s)
registered (0 pending), today = 2026-11-17"**; fuzz, budgets EMPTY.
Compile: `test-output.txt` has `could not compile kiln-workspace (lib) due to 6 previous errors` and `… (lib test) due
to 34 previous errors`, and **no `could not compile xtask` line** (in runs 43765532298 and 43775682214 the xtask
tests did not compile). Error kinds printed: E0433 `unix` in `os` 15, `ino` 8, `dev` 4, `from_mode` 2, `is_fifo` 2,
`mode` 2, `uid` 1 (`kiln-workspace` only).
b. `xtask` (core-crates step, `==== xtask: exit status 101`): only the unit-test binary ran (`Running unittests
src\main.rs`, `running 130 tests`): **108 passed, 22 failed**; cargo stopped there, so **no `xtask` integration test
binary ran** (one `Running` line). The 22, by cause (first message lines from the artefact):
- **15 × the queue's `touches` refusal lists files the test did not change** (`queue::tests::` budget_counts_…,
  conflict_is_aborted_and_rejected, dependency_changes_…, gate_path_changes_…, generated_only_conflicts_…,
  green_gate_fast_forwards_main_and_logs, leaky_regeneration_…, lock_rule_is_applied_to_the_rebased_tree,
  main_history_tolerates_only_ledger_commits, medium_and_high_evidence, member_lints_and_custom_build_scripts_…,
  mixed_or_failing_regeneration_…, non_merged_outcomes_…, red_gate_leaves_main_alone_…, widened_gate_paths_…): e.g.
  `queue: refused: touches: task/T-001 changes path(s) outside touches [feature.txt, items, base.txt]: .gitignore,
  xtask/generated.toml, xtask/ledger.toml`; with `left: "touches"` against the expected check (`budget`,
  `dependencies`, `gate-path`, `join`).
- **4 × merge or rebase conflicts in toy repositories**: `join::tests::acceptance_failures_keep_the_merge_…`,
  `join_with_failing_base_and_passing_implementation`, `guards_must_pass_on_base_and_joined_branch` (`merging
  task/T-004-tests into task/T-004 conflicts (aborted)`); `queue::tests::state_front_matter_touches_and_config_refusals`
  (`CONFLICT (content): Merge conflict in tasks/T-001-thing.md`).
- **1 × CRLF in a toy repository's file**: `publish::tests::success_appends_six_commits_and_never_rewrites_main`
  (`left: "a\r\nx\r\nnew\r\n"`, `right: "a\nx\nnew\n"`). INFERRED for the two groups above as well: the toy
  repositories are created without the repository's `.gitattributes`, and Git for Windows' line-ending conversion
  changes their files.
- **1 × path separator in a message**: `publish::tests::a_held_queue_lock_refuses_publish_before_anything_is_built`
  (`left: "…\.git\kiln-queue.lock"`, `right: "…\.git/kiln-queue.lock"`).
- **1 × running a non-executable as a program**: `queue::tests::main_xtask_is_rebuilt_from_base_on_every_run`
  (`Os { code: 216 … "This version of %1 is not compatible with the version of Windows you're running…"`).
c. Registered tests on Windows: all 7 entries leave Windows out. Unit binary: entries 1, 2, 3 absent (no line names
them; 130 tests against Linux's 133). Entries 4–7 are in integration binaries that did not run — **not
determinable** whether they would be absent.
d. `guard_t054`, `acceptance_t054`: **no verdict on Windows** (their binaries did not run). The Windows verdict of
`ac2_backtrace_names_this_file_and_line` (T-054 ruling Q-4 A) stays pending.
e. `acceptance_t071`, `guard_t071`: no verdict on Windows (not run).
f. Compiles: `kiln-text`, `kiln-syntax`, `kiln-search` (tests pass: exit status 0 each) and every `xtask` target
(no compile error line for `xtask`); does not compile: `kiln-workspace` lib and lib test (T-073), and so nothing
above it.

## Window jobs
- `window-linux` success; `window-procedure.txt`: six `CASE` lines (`ac2 exit window=0 headless=0`, `ac2x3 3 3`,
  `dblclick 0 0`, `scroll100 0 0`, `ctrlq 0 0`, `noroot 0 0`), `WINDOW CASES PASS`, `AC-3: exit=2 stdout_bytes=0
  stderr_lines=1 panicked=0 (pass: 2 0 1 0)`, `AC-3 PASS`, `PLAIN PASS: the start path changes the screen`.
- `window-probe (macos-15)` success: `PROBE: exit status 0`, `PROBE: stdout     2174 bytes, 3 lines`.
- `window-probe (windows-2025)` failure at Build kiln; `PROBE: not run (cargo build -p kiln: failure)`.

## Against T-071's AC-6
Text (`tasks/T-071-platform-tests-rule.md` on main): "AC-6: on the candidate, the `core-crates` step shows `xtask`
exit status 0 on `gate-macos` and on `gate-windows`; `gate-linux` green. The macOS gate stays red until T-079
(product tests) and Windows until T-073, as `tasks/T-070-*.md` "Expected to FAIL" says; the scorecard step on macOS
prints the platform detail line of Interface (3)."
| Clause | Verdict | Quote |
|---|---|---|
| core-crates `xtask` exit 0 on gate-macos | **met** | `==== xtask: exit status 0`; `CORE CRATES: … xtask=0` (log) |
| core-crates `xtask` exit 0 on gate-windows | **not met** | `==== xtask: exit status 101`; `test result: FAILED. 108 passed; 22 failed` (artefact) |
| gate-linux green | **met** | job conclusion success; `passed: true` |
| macOS gate red until T-079 (expected) | met (red, as expected) | `GATE FAILED in 509.3s: scorecard, test` |
| Windows gate red until T-073 (expected) | met (red, as expected) | `could not compile kiln-workspace (lib) due to 6 previous errors` |
| scorecard step on macOS prints the platform detail line | **not met as printed** | detail is `6 scorecard problems: …`; no "page not compared" text in the log. The page was not compared (no "stale" problem, unlike the two earlier runs), so the behaviour is there, but the line is replaced by the problem list when the step fails |

## Not read
- The macOS `test-output.txt`, `core-crates.txt`, `gate-report.json` artefact (upload `ETIMEDOUT`): the whole macOS
  failing set, macOS test names (registered tests, `guard_t054`, `acceptance_t054`, `acceptance_t071`, `guard_t071`).
  A re-run of the job would upload them.
- Windows: any `xtask` integration test (none ran; the core-crates step stops at the first failing binary).
- Window jobs' logs beyond their artefacts.
