# CI run 44395960776 (platforms.yml) on ci/candidate @ a76ec8ea05b69166d59a8de3b21b9e7f57435889 — T-092 round 2

Record by hand; method as `tasks/ci-runs/README.md`; same items and layout as `T-092-round-1.md` (run 44383948281 @ b924926). Round 2 follows a rework of the two test files `acceptance_t008` and `guard_t008` of `kiln-workspace` (as stated in the brief; the diff is not read here). Every value comes from this run's job listing, artefacts or job logs. No judgement of results is made.

**Complete: the run was `in_progress` at the first read (about 21:15Z; only `gate-macos` still running, in Core crates from 21:16:04Z); `gate-macos` finished 21:25:54Z; every job has finished. Run-level `conclusion` was not re-read after that (the job listing shows all six jobs completed: four success, two failure: `gate-windows`, `window-probe (windows-2025)`).**

## Run header
- id 44395960776, run_number 44, attempt 1, event push, workflow `platforms`; branch `ci/candidate`; commit `a76ec8ea05b69166d59a8de3b21b9e7f57435889` (equals the head asked for); title "T-092: implementer log: CI-REQUEST round 2"; created_at 2026-11-18T20:54:45Z.

## macOS runner question (first)
- `gate-macos` (job 135125774372): **a runner was assigned** (`runner_id` 1000001143); job created 20:54:46Z, started 20:54:49Z (no wait for a runner, unlike round 1).
- `window-probe (macos-15)` (job 135125773790): **a runner was assigned** (`runner_id` 1000001142); started 20:54:48Z.

## Job table
Wall time = `completed_at` − `started_at` of the job (job listing).
| job | id | runner (`runner_id`) | conclusion | wall | first failing step |
|---|---|---|---|---|---|
| gate-linux | 135125774249 | 1000001146 | success | 16m21s (20:54:47Z–21:11:08Z) | none |
| gate-windows | 135125774218 | 1000001145 | **failure** | 16m11s (20:54:47Z–21:10:58Z) | Gate (20:55:13Z–20:59:52Z, 4m39s); every later step success or skipped |
| gate-macos | 135125774372 | 1000001143 | success | 31m05s (20:54:49Z–21:25:54Z; created 20:54:46Z) | none |
| window-linux | 135125774261 | 1000001147 | success | 57s (20:54:47Z–20:55:44Z) | none |
| window-probe (macos-15) | 135125773790 | 1000001142 | success | 51s (20:54:48Z–20:55:39Z) | none |
| window-probe (windows-2025) | 135125774154 | 1000001144 | **failure** | 1m52s (20:54:48Z–20:56:40Z) | Window probe (20:56:33Z–20:56:35Z); Build kiln success |

gate-windows steps (job listing): Gate failure (20:55:13Z–20:59:52Z); Gate report success; Core crates success (20:59:52Z–21:10:40Z, 10m48s); Workspace lib and T-073 tests success (4s); Test list success (1s); kiln-workspace test targets success (7s); Full test output (gate failed) success, ran (1s); Disk, Upload reports success; Post Cache skipped.

Artefacts: `gate-windows-44395960776` (id 14471797634) 24751 B, `gate-linux-44395960776` (id 14470408192) 32306 B, `window-linux-44395960776` 577 B, `window-probe-macos-15-44395960776` (id 14470540753) 703 B, `window-probe-windows-2025-44395960776` (id 14470921116) 344 B; `gate-macos-44395960776` (id 14473800259) 32045 B. Fetched with `download_workflow_run_artifact` and curl (size equal to `size_in_bytes`, `testzip()` clean): gate-windows, gate-linux, the two probe artefacts, gate-macos. `window-linux` not downloaded. Scratch directory removed after the record. Job logs read (logs + grep): gate-windows and gate-linux (image block, status lines), gate-macos (image block, status line).

## gate-linux (job 135125774249, ubuntu-24.04)
- Image (job log): `Image: ubuntu-24.04`, `Version: 20261111.127.1`. Artefact `gate-linux-44395960776`: `core-crates.txt`, `gate-report.json`, `test-list.txt`; no `test-output.txt` (gate green).
- Gate report: `passed: true`, `failed_step: null`, 628.49 s.
| step | status | seconds | detail |
|---|---|---|---|
| fmt | pass | 0.88 | formatted |
| clippy | pass | 9.30 | no warnings (all targets, -D warnings) |
| layers | pass | 0.44 | 11 crates, 14 internal edges, layer table holds |
| ui-commands | pass | 0.03 | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | pass | 0.07 | reviews: 65 of 77 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 720 tracked file(s) clean (0 not UTF-8, skipped) |
| schema | pass | 4.43 | 30 commands; audit ok; docs/commands.json current |
| scorecard | pass | 150.35 | 64 items: 47 verified, 0 implemented, 0 failing, 17 planned. |
| test | pass | 460.70 | `1212 passed, 0 failed, 0 ignored in 142 suites` |
| no-skip | pass | 2.29 | self-test ok (17 known findings); 208 files scanned, 0 allowlisted skip(s), 47 platform test(s) registered (8 pending), today = 2026-11-18 |
| fuzz, budgets | empty | 0.0 | no targets registered yet |
- `TEST LIST` status line (job log): `TEST LIST: exit status 0`.
- No failing test. `core-crates.txt`: `==== kiln-text/kiln-syntax/kiln-search/xtask: exit status 0` (all four); sum of its `test result:` lines 511 passed, 0 failed, 0 ignored (`xtask` 325 passed, 0 failed).
- `acceptance_t092` (xtask): `running 5 tests`, `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.57s`. `guard_t092`: `running 2 tests`, `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.22s`.

## gate-windows (job 135125774218, windows-2025) — the subject
- Runner 1000001145. Image (job log): `Image: windows-2025-vs2026`, `Version: 20261103.298.1`; `cargo 1.97.0 (c980f4866 2026-06-30)`. Artefact `gate-windows-44395960776` (id 14471797634, 24751 B; fetched, size equal, `testzip()` clean): `core-crates.txt` 41934 B, `gate-report.json` 15107 B, `test-list.txt` 10832 B, `test-output.txt` 10832 B, `windows-tests.txt` 53682 B, `workspace-tests.txt` 8382 B (sizes of the files as extracted, ANSI included).
- Job-log status lines: `CORE CRATES: exit status per package: kiln-text=0 kiln-syntax=0 kiln-search=0 xtask=0 (reported, not required)`; `WORKSPACE TESTS: exit status 0 (reported, not required)`; `TEST LIST: exit status 101` and `TEST LIST: the status is recorded, not used; the gate step decides the colour`; `TEST OUTPUT: exit status 101`.
- Gate report (`gate-report.json`, ANSI stripped): `passed: false`, `failed_step: clippy`, 264.31 s.
| step | status | seconds | detail |
|---|---|---|---|
| fmt | pass | 1.59 | formatted |
| clippy | **fail** | 39.50 | kept text 60 lines; first `error` line `error[E0599]: no method named `mode` found for struct `std::fs::Metadata` in the current scope` at `apps\kiln-cli\tests\guard_t037_cli.rs:150:15` (the first `-->` line in the kept text; the kept text begins mid-message, line 3 is the first `error` line); then `uid` (151:15), `gid` (152:15), `from_mode` for `std::fs::Permissions` (215:55), `is_fifo` for `std::fs::FileType` (502:57), `mode` again; ends `error: could not compile `kiln-cli` (test "guard_t037_cli") due to 16 previous errors` |
| layers | pass | 7.54 | 11 crates, 14 internal edges, layer table holds |
| ui-commands | pass | 0.03 | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | pass | 0.11 | reviews: 65 of 77 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 720 tracked file(s) clean (0 not UTF-8, skipped) |
| schema | pass | 20.31 | 30 commands; audit ok; docs/commands.json current |
| scorecard | **fail** | 191.31 | first line `windows: 64 items; 165 tests ran; 8 registered absent here; page not compared (computed on Linux)`, then `47 scorecard problems:`; first problem `building test executables failed on windows (); their tests are not run`; the next listed is `cmd.registry: lists test `kiln_commands::registry::tests::rejects_params_that_violate_schema`, which does not exist on windows` (the rest are `<item>: lists test `<name>`, which does not exist on windows`) |
| test | **fail** | 0.66 | `0 passed, 0 failed, 0 ignored in 0 suites`; kept text 27 lines; first `error` line (kept text line 9) `error: could not compile `kiln` (test "acceptance_t065") due to 3 previous errors`, then `error[E0433]: cannot find `unix` in `os`` at `apps\kiln\tests\acceptance_t055.rs:1317:18`; ends `error: could not compile `kiln` (test "acceptance_t055") due to 2 previous errors` |
| no-skip | pass | 3.26 | self-test ok (17 known findings); 208 files scanned, 0 allowlisted skip(s), 47 platform test(s) registered (8 pending), today = 2026-11-18 |
| fuzz, budgets | empty | 0.0 | no targets registered yet |
- (e) Failing steps of the gate and the first target each names: **clippy** names `kiln-cli` test `guard_t037_cli` (`failed_step: clippy`); **scorecard** names no target in its first problem (`building test executables failed on windows ()`); its first listed test is `kiln_commands::registry::tests::rejects_params_that_violate_schema`; **test** first names `kiln` test `acceptance_t065` (kept text line 9; first source location `apps\kiln\tests\acceptance_t055.rs:1317:18`). No `kiln-workspace` target is named by any of the three.
- "Test list": `test-list.txt` (10832 B) holds only compile errors: `could not compile` lines for `kiln` tests `window_t065` (8 errors), `acceptance_t065` (3), `script_t055` (1), `acceptance_t055` (2); no binary and no test name. "Full test output (gate failed)": ran; `test-output.txt` 10832 B, 172 lines, 0 lines containing `FAILED`; same four `could not compile` lines.

### (b) Step "Workspace lib and T-073 tests (reported, not required)" (`workspace-tests.txt`)
Binaries: library `running 45 tests`, `acceptance_t073` `running 1 test`, `guard_t073` `running 10 tests`; 56 tests, 0 failed; `WORKSPACE TESTS: exit status 0`. Library lines, verbatim (45 tests, as in round 1's list by name; order differs):
```
running 45 tests
test disk::tests::errors_are_root_independent ... ok
test disk::tests::check_location_inside_on_every_platform ... ok
test disk::tests::failed_rename_removes_own_temp_and_keeps_target ... ok
test disk::tests::held_root_check_is_inside_only_for_a_read_identity_equal_to_the_held_one ... ok
test disk::tests::leftover_at_temp_name_is_never_touched_on_every_platform ... ok
test disk::tests::check_location_outside_on_every_platform ... ok
test disk::tests::all_temp_names_taken_fails_without_changes_then_recovers ... ok
test disk::tests::failed_save_after_writability_check_leaves_target_and_listing_unchanged_on_every_platform ... ok
test disk::tests::new_rejects_missing_root_and_file_root_without_naming_them ... ok
test disk::tests::longest_file_names_can_be_saved ... ok
test disk::tests::root_given_through_a_link_on_every_platform ... ok
test disk::tests::root_removed_on_every_platform ... ok
test disk::tests::link_at_temp_name_is_not_followed_on_every_platform ... ok
test disk::tests::root_replaced_by_a_link_on_every_platform ... ok
test document::tests::dirty_follows_content ... ok
test disk::tests::save_through_a_link_on_every_platform ... ok
test document::tests::edits_keep_encoding_and_inserted_newline_stays_lf ... ok
test document::tests::failed_save_keeps_file_and_dirty_state ... ok
test document::tests::open_errors ... ok
test document::tests::unedited_round_trip_is_byte_exact_for_each_encoding ... ok
test document::tests::works_through_trait_objects ... ok
test encoding::tests::decode_error_display ... ok
test encoding::tests::detects_boms_and_strips_exactly_one ... ok
test encoding::tests::encode_decode_inverse_on_generated_strings ... ok
test encoding::tests::exact_error_offsets ... ok
test encoding::tests::names ... ok
test encoding::tests::utf16_with_surrogate_pair ... ok
test memory::tests::check_location_is_always_ok_and_changes_nothing ... ok
test memory::tests::directories_are_implicit ... ok
test memory::tests::failing_writes_change_nothing ... ok
test memory::tests::list_dir_lists_implied_directories_in_byte_order ... ok
test memory::tests::sibling_with_common_prefix_is_not_a_child ... ok
test memory::tests::write_into_missing_parent_is_not_found_and_creates_nothing ... ok
test memory::tests::write_then_read_round_trips_and_creates_top_level_files ... ok
test path::tests::accepts_valid_paths_verbatim ... ok
test path::tests::parent_and_segments ... ok
test path::tests::rejects_invalid_paths ... ok
test project::tests::bom_and_invalid_lines ... ok
test project::tests::limit_edges ... ok
test project::tests::negation_and_deeper_files ... ok
test project::tests::under_uses_prefix_gitignore_and_is_not_itself_subject ... ok
test project::tests::worked_example ... ok
test document::tests::disk_save_onto_path_replaced_by_directory_fails_and_changes_nothing ... ok
test disk::tests::write_read_and_errors ... ok
test disk::tests::matches_memory_model_on_random_operations ... ok
test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.67s
```
Other result lines, verbatim:
```
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
```
No failing test in this step.

### Step "kiln-workspace test targets (reported, not required)" (`windows-tests.txt`, ANSI stripped)

Every `==== <target>: exit status N` line, verbatim (15 targets):
```
==== acceptance_t002: exit status 0
==== acceptance_t002_r2: exit status 0
==== acceptance_t008: exit status 0
==== acceptance_t015_tools: exit status 0
==== acceptance_t019_workspace: exit status 101
==== acceptance_t036_workspace: exit status 0
==== acceptance_t073: exit status 0
==== confinement: exit status 0
==== guard_t008: exit status 0
==== guard_t036_removal_workspace: exit status 0
==== guard_t036_workspace: exit status 0
==== guard_t073: exit status 0
==== guard_t074: exit status 0
==== project: exit status 101
==== tools_contract: exit status 0
```
Summary line (job log), verbatim:
```
WINDOWS TESTS: exit status per target: acceptance_t002=0 acceptance_t002_r2=0 acceptance_t008=0 acceptance_t015_tools=0 acceptance_t019_workspace=101 acceptance_t036_workspace=0 acceptance_t073=0 confinement=0 guard_t008=0 guard_t036_removal_workspace=0 guard_t036_workspace=0 guard_t073=0 guard_t074=0 project=101 tools_contract=0 (reported, not required)
```
Every target's `running N tests` and `test result:` lines, verbatim, in file order (a target that does not compile has neither):
```
-- acceptance_t002
running 15 tests
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
-- acceptance_t002_r2
running 8 tests
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
-- acceptance_t008
running 7 tests
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
-- acceptance_t015_tools
running 3 tests
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
-- acceptance_t019_workspace
error: could not compile `kiln-workspace` (test "acceptance_t019_workspace") due to 5 previous errors
-- acceptance_t036_workspace
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
-- acceptance_t073
running 1 test
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
-- confinement
running 4 tests
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
-- guard_t008
running 1 test
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
-- guard_t036_removal_workspace
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
-- guard_t036_workspace
running 2 tests
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
-- guard_t073
running 10 tests
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
-- guard_t074
running 15 tests
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
-- project
error: could not compile `kiln-workspace` (test "project") due to 3 previous errors
-- tools_contract
running 5 tests
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

#### `acceptance_t008` (exit 0) and `guard_t008` (exit 0): test lines and `MEASURED:` lines, verbatim, in order
`acceptance_t008`, the file's block from `running 7 tests` to its `test result:` line (the `MEASURED:` lines stand under the test they belong to):
```
running 7 tests
test root_removed_and_replaced_by_link_is_refused ... ok
test link_swapped_in_after_open_is_refused_at_save ... ok
test root_removed_and_replaced_by_new_directory_is_refused ... ok
test outside_root_wins_over_not_found ... ok
test open_through_escaping_symlinks_is_refused ... ok
test root_swapped_between_requests_is_refused ... ok
test save_through_escaping_symlinks_touches_nothing ... ok

successes:

---- open_through_escaping_symlinks_is_refused stdout ----
MEASURED: acceptance_t008 open: windows: root/rel: reaches the outside content
MEASURED: acceptance_t008 open: windows: root/abs: reaches the outside content
MEASURED: acceptance_t008 open: windows: root/dlink/secret.txt: reaches the outside content
MEASURED: acceptance_t008 open: windows: root/chain: reaches the outside content
MEASURED: acceptance_t008 open: windows: root/t: does not reach: NotFound
MEASURED: acceptance_t008 open: windows: root/px: reaches the outside content

---- save_through_escaping_symlinks_touches_nothing stdout ----
MEASURED: acceptance_t008 save: windows: root/rel: reaches the outside content
MEASURED: acceptance_t008 save: windows: root/abs: reaches the outside content
MEASURED: acceptance_t008 save: windows: root/dlink/secret.txt: reaches the outside content
MEASURED: acceptance_t008 save: windows: root/chain: reaches the outside content
MEASURED: acceptance_t008 save: windows: root/t: does not reach: NotFound
MEASURED: acceptance_t008 save: windows: root/px: reaches the outside content


successes:
    link_swapped_in_after_open_is_refused_at_save
    open_through_escaping_symlinks_is_refused
    outside_root_wins_over_not_found
    root_removed_and_replaced_by_link_is_refused
    root_removed_and_replaced_by_new_directory_is_refused
    root_swapped_between_requests_is_refused
    save_through_escaping_symlinks_touches_nothing

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

==== acceptance_t008: exit status 0
```
`guard_t008`, the file's block (it holds 0 `MEASURED:` lines):
```
running 1 test
test symlinks_inside_root_still_work ... ok

successes:

successes:
    symlinks_inside_root_still_work

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

==== guard_t008: exit status 0
```

#### Targets with a non-zero status
No target compiles-and-fails: no `FAILED` test line and no `panicked` line in `windows-tests.txt` (`grep -c "FAILED\|panicked" windows-tests.txt` gives 0). Two targets do not compile, as in round 1:

**`acceptance_t019_workspace`** (exit 101): does not compile (`due to 5 previous errors`). First three `error` lines, with the `-->` line of each:
```
error[E0433]: cannot find `unix` in `os`
  --> crates\kiln-workspace\tests\acceptance_t019_workspace.rs:19:14
error[E0433]: cannot find `unix` in `os`
  --> crates\kiln-workspace\tests\acceptance_t019_workspace.rs:20:14
error[E0599]: no method named `is_fifo` found for struct `FileType` in the current scope
   --> crates\kiln-workspace\tests\acceptance_t019_workspace.rs:107:61
```
**`project`** (exit 101): does not compile (`due to 3 previous errors`). The three `error` lines, with the `-->` line of each:
```
error[E0433]: cannot find `unix` in `os`
  --> crates\kiln-workspace\tests\project.rs:10:14
error[E0433]: cannot find `unix` in `os`
  --> crates\kiln-workspace\tests\project.rs:11:14
error[E0599]: no method named `is_fifo` found for struct `FileType` in the current scope
  --> crates\kiln-workspace\tests\project.rs:74:65
```

### (c) `MEASURED:` lines
`grep -c "MEASURED:" windows-tests.txt` gives 190: 12 inside the `acceptance_t008` block (all listed above), 2 in `guard_t073`, 176 in `guard_t074` (awk counting per `Running` line). `guard_t008` 0. `workspace-tests.txt` holds 2 (those of `guard_t073`). The 2 + 176 are not listed.

### (d) `xtask` on Windows (`core-crates.txt`)
`==== kiln-text: exit status 0`, `==== kiln-syntax: exit status 0`, `==== kiln-search: exit status 0`, `==== xtask: exit status 0`. Totals (sum of `test result:` lines, `awk '{p+=$4; f+=$6; i+=$8}'`): `xtask` 317 passed, 0 failed, 0 ignored; all four packages 503 passed, 0 failed, 0 ignored. No failing test.
`acceptance_t092`: `running 5 tests` ... `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.72s`. `guard_t092`: `running 2 tests` ... `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.29s`.


## gate-macos (job 135125774372, macos-15) — runner assigned (`runner_id` 1000001143)
- Image (job log): `Image: macos-15-arm64`, `Version: 20261104.0514.1`. Artefact `gate-macos-44395960776` (id 14473800259, 32045 B; fetched, size equal, `testzip()` clean): `core-crates.txt`, `gate-report.json`, `test-list.txt`; no `test-output.txt` (gate green). Job log: `TEST LIST: exit status 0`. Job steps: Gate success (20:55:21Z–21:16:04Z), Core crates success (21:16:04Z–21:25:42Z), Full test output skipped.
- Gate report: `passed: true`, `failed_step: null`, 1229.08 s.
| step | status | seconds | detail |
|---|---|---|---|
| fmt | pass | 1.37 | formatted |
| clippy | pass | 18.70 | no warnings (all targets, -D warnings) |
| layers | pass | 2.28 | 11 crates, 14 internal edges, layer table holds |
| ui-commands | pass | 0.04 | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | pass | 0.37 | reviews: 65 of 77 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 720 tracked file(s) clean (0 not UTF-8, skipped) |
| schema | pass | 10.08 | 30 commands; audit ok; docs/commands.json current |
| scorecard | pass | 276.62 | macos: 64 items; 216 tests ran; 3 registered absent here; page not compared (computed on Linux) |
| test | pass | 916.27 | `1195 passed, 0 failed, 0 ignored in 142 suites` |
| no-skip | pass | 3.34 | self-test ok (17 known findings); 208 files scanned, 0 allowlisted skip(s), 47 platform test(s) registered (8 pending), today = 2026-11-18 |
| fuzz, budgets | empty | 0.0 | no targets registered yet |
- No failing test. `core-crates.txt`: all four `==== <pkg>: exit status 0`; sum of `test result:` lines 506 passed, 0 failed, 0 ignored (`xtask` 320 passed, 0 failed); `grep -c FAILED` 0. `acceptance_t092` (xtask): `running 5 tests`, `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.84s`. `guard_t092`: `running 2 tests`, `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.46s`.

## Test names listed per `kiln-workspace` test binary (`test-list.txt`)
Command: `count.py` (scratch directory, removed): per `Running <target> (target/debug/deps/<name>-<hash>)` line of the ANSI-stripped file, the lines ending `: test` that follow it before the next `Running` line.
| binary | Linux | macOS |
|---|---|---|
| lib (`unittests src/lib.rs`, `kiln_workspace`) | 60 | 60 |
| acceptance_t002 | 15 | 15 |
| acceptance_t002_r2 | 10 | 10 |
| acceptance_t008 | 11 | 11 |
| acceptance_t015_tools | 3 | 3 |
| acceptance_t019_workspace | 7 | 5 |
| acceptance_t036_workspace | 1 | 1 |
| acceptance_t073 | 1 | 1 |
| confinement | 7 | 7 |
| guard_t008 | 1 | 1 |
| guard_t036_removal_workspace | 2 | 2 |
| guard_t036_workspace | 3 | 3 |
| guard_t073 | 10 | 10 |
| guard_t074 | 15 | 15 |
| project | 14 | 12 |
| tools_contract | 5 | 5 |
Differences between the two columns: `acceptance_t019_workspace` (7 vs 5) and `project` (14 vs 12). Windows: `test-list.txt` holds compile errors only (no name listed), so no Windows column.

## Window jobs
- window-linux: success (artefact contents not read).
- window-probe (macos-15): success; probe artefact (`window-probe.txt`): `PROBE OS: macos-15 ...` (macOS 15.8.2), `PROBE: exit status 0`, `PROBE: stdout     2174 bytes, 3 lines`, empty stderr.
- window-probe (windows-2025): failure (step Window probe failed, Build kiln succeeded); probe artefact: `PROBE: exit status 2`, `PROBE: stdout 0 bytes, 0 lines`, stderr `kiln: no window: egui_glow: OpenGL: egui_glow requires opengl 2.0+.`

## Compared with `T-092-round-1.md` (run 44383948281 @ b924926)
Status = `==== <target>: exit status`; passed/failed from the `test result:` line.
| target | round 1 status | round 1 passed / failed | round 2 status | round 2 passed / failed |
|---|---|---|---|---|
| acceptance_t002 | 0 | 15 / 0 | 0 | 15 / 0 |
| acceptance_t002_r2 | 0 | 8 / 0 | 0 | 8 / 0 |
| acceptance_t008 | 101 (compiles, fails) | 5 / 2 | 0 | 7 / 0 |
| acceptance_t015_tools | 0 | 3 / 0 | 0 | 3 / 0 |
| acceptance_t019_workspace | 101 (does not compile) | none | 101 (does not compile) | none |
| acceptance_t036_workspace | 0 | 0 / 0 (`running 0 tests`) | 0 | 0 / 0 (`running 0 tests`) |
| acceptance_t073 | 0 | 1 / 0 | 0 | 1 / 0 |
| confinement | 0 | 4 / 0 | 0 | 4 / 0 |
| guard_t008 | 101 (compiles, fails) | 0 / 1 | 0 | 1 / 0 |
| guard_t036_removal_workspace | 0 | 0 / 0 (`running 0 tests`) | 0 | 0 / 0 (`running 0 tests`) |
| guard_t036_workspace | 0 | 2 / 0 | 0 | 2 / 0 |
| guard_t073 | 0 | 10 / 0 | 0 | 10 / 0 |
| guard_t074 | 0 | 15 / 0 | 0 | 15 / 0 |
| project | 101 (does not compile) | none | 101 (does not compile) | none |
| tools_contract | 0 | 5 / 0 | 0 | 5 / 0 |
Other differences, from the two records: library tests 45 then, 45 now; `MEASURED:` lines in `windows-tests.txt` 178 then, 190 now (12 `acceptance_t008` more; 176 `guard_t074` + 2 `guard_t073` in both); `xtask` on Windows 317 passed then and now; `xtask` Linux 325 now (round 1 states 511 total for the four packages on Linux, 511 now); macOS four-package total 506 then and now; gate failing steps clippy, scorecard, test then and now (first clippy target `guard_t038_cli` then, `guard_t037_cli` now); scorecard problems 50 then, 47 now; `gate-windows` gate-report 466.55 s then, 264.31 s now; `gate-windows` wall 26m52s then, 16m11s now; `gate-macos` wall 27m26s (runner wait about 10 min) then, 31m05s (no wait) now; `gate-windows` test-count lines `1212`/`1195` for Linux/macOS equal in both rounds; `window-probe (windows-2025)` failure in both rounds.

## Not read
Logs of all jobs except the three gate jobs' image blocks and status lines; the `window-linux` artefact; `gate-*` `core-crates.txt` beyond `test result:`/`exit status` lines and the `acceptance_t092`/`guard_t092` blocks; the 2 `guard_t073` and 176 `guard_t074` `MEASURED:` lines (counted, not listed); `test-output.txt` beyond its `could not compile` lines; the Windows `kiln-workspace` blocks of targets other than `acceptance_t008` and `guard_t008` beyond their `running`/`test result:` lines; the registered platform tests (`xtask/platform-tests.toml` at the commit) per platform; the run-level `conclusion` after completion (derived from the job listing); the diff of the rework (what changed in the two test files); whether the brief's "13 of 15 targets compile and pass" matches (the file shows 13 targets with exit status 0 and 2 with 101; no judgement made).
