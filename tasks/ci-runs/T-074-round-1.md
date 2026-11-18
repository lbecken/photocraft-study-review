# CI run 44358453526 (platforms.yml) on ci/candidate @ 720f1d56ac27b83594dd96f16734b04b2f8d4c7f — T-074 round 1

Record by hand; method as `tasks/ci-runs/README.md`; same items as `T-074-early-2.md` (so the two can be compared line by line), with the per-gate-job layout of `T-072-round-5.md`. Title of the run: "CI candidate (not for merge): T-074's reviewed head merged with main and T-072's reviewed head (the tree the queue tests once T-072 is on main)" (commit message). This is task T-074's formal round: its reviewed head merged with main and with T-072's head. Every value comes from this run's job listing, artefacts or job logs. No judgement of results is made here.

**Complete: the run is `completed` (conclusion failure; `updated_at` 2026-11-18T17:58:35Z); every job has finished. `gate-macos` was still running (Core crates step) at the first reads (about 17:56Z) and finished 17:58:34Z. Nothing was still running at the end.**

## Run header
- id 44358453526, run_number 39, attempt 1, event push, workflow `platforms`; branch `ci/candidate`; commit `720f1d56ac27b83594dd96f16734b04b2f8d4c7f` (equals the head asked for); created_at 2026-11-18T17:25:10Z.

## macOS runner question (first)
- `gate-macos` (job 134992871807, label macos-15): **a runner was assigned** (`runner_id` 1000001066, `runner_name` "GitHub Actions 1000001066"); job created 17:25:10Z, started 17:30:13Z (about 5m03s after creation, INFERRED to be the wait for a runner).
- `window-probe (macos-15)` (job 134992872548): **no runner assigned** (no `runner_id`, no `runner_name`, no steps listed); conclusion cancelled; started_at 17:25:10Z, completed_at 17:40:11Z (15m01s).

## Job table
Wall time = `completed_at` − `started_at` of the job (job listing).
| job | id | runner label | conclusion | wall | first failing step |
|---|---|---|---|---|---|
| gate-linux | 134992872417 | ubuntu-24.04 | success | 16m18s (17:25:13Z–17:41:31Z) | none |
| gate-windows | 134992872546 | windows-2025 | **failure** | 26m17s (17:25:14Z–17:51:31Z) | Gate (17:25:42Z–17:33:46Z, 8m04s); every later step success or skipped |
| gate-macos | 134992871807 | macos-15 | success | 28m21s (17:30:13Z–17:58:34Z; job created 17:25:10Z, about 5 min before start, INFERRED) | none |
| window-linux | 134992872590 | ubuntu-24.04 | success | 1m41s (17:25:13Z–17:26:54Z) | none |
| window-probe (windows-2025) | 134992872391 | windows-2025 | **failure** | 1m51s (17:25:12Z–17:27:03Z) | Window probe (17:26:57Z–17:26:58Z); Build kiln success |
| window-probe (macos-15) | 134992872548 | macos-15 | **cancelled** | 15m01s (17:25:10Z–17:40:11Z) | no steps listed; no runner assigned |

## Artefacts
Listed at about 17:55Z (4 artefacts) and again at about 17:59Z (5, with `gate-macos-44358453526`, 31,673 B, id 14456243039, described under gate-macos): `gate-windows-44358453526` 27,308 B (id 14454832267); `gate-linux-44358453526` 31,944 B (id 14454366504); `window-linux-44358453526` 577 B (id 14453593088, not downloaded); `window-probe-windows-2025-44358453526` 344 B (id 14453078171). No `window-probe-macos-15-…` artefact at that time. Fetched with `download_workflow_run_artifact` and curl; size equal to `size_in_bytes`, `testzip()` clean; scratch directory removed after the record. Files: gate-linux: `core-crates.txt` 40,856 B, `gate-report.json` 1,986 B, `test-list.txt` 78,486 B (no `test-output.txt`: gate green). gate-windows: `core-crates.txt` 41,392 B, `gate-report.json` 16,983 B, `test-list.txt` 10,832 B, `test-output.txt` 10,832 B, `windows-tests.txt` 95,467 B, `workspace-tests.txt` 8,004 B.
Job logs read (logs URL + curl): gate-linux and gate-windows (header, cache, `TEST LIST`/`TEST OUTPUT`, disk, summary lines).

## gate-windows (job 134992872546, windows-2025) — main subject
- Runner image (job log): `Image: windows-2025-vs2026`, `Version: 20261103.298.1`; runner 2.338.2; `cargo 1.97.0 (c980f4866 2026-06-30)`; cache `No cache found.`
- Steps (job listing):
| step | conclusion | start–end (Z) | duration |
|---|---|---|---|
| Set up job | success | 17:25:15–17:25:16 | 1s |
| Checkout | success | 17:25:16–17:25:25 | 9s |
| Install the pinned toolchain (rust-toolchain.toml) | success | 17:25:25–17:25:37 | 12s |
| Cache cargo registry and target | success | 17:25:37–17:25:42 | 5s |
| Gate | **failure** | 17:25:42–17:33:46 | 8m04s |
| Gate report | success | 17:33:46–17:33:47 | 1s |
| Core crates (reported, not required) | success | 17:33:47–17:51:04 | 17m17s |
| Workspace lib and T-073 tests (reported, not required) | success | 17:51:04–17:51:11 | 7s |
| Test list | success | 17:51:11–17:51:13 | 2s |
| kiln-workspace test targets (reported, not required) | success | 17:51:13–17:51:21 | 8s |
| Full test output (gate failed) | success (**ran**) | 17:51:21–17:51:22 | 1s |
| Disk | success | 17:51:22–17:51:25 | 3s |
| Upload reports | success | 17:51:25–17:51:27 | 2s |
| Post Cache cargo registry and target | skipped | 17:51:27 | 0s |
| Post Checkout, Complete job | success, success | 17:51:27–17:51:29 | 2s, 0s |
- Gate report (`gate-report.json`, ANSI stripped): `passed: false`, `failed_step: clippy`, 461.23 s.
| step | status | seconds | detail |
|---|---|---|---|
| fmt | pass | 2.28 | formatted |
| clippy | **fail** | 61.82 | kept text 60 lines; first `error` line `error[E0599]: no method named `mode` found for struct `std::fs::Metadata` in the current scope` at `apps\kiln-cli\tests\guard_t037_cli.rs:150:15`; ends `error: could not compile `kiln-cli` (test "guard_t037_cli") due to 16 previous errors` |
| layers | pass | 11.33 | 11 crates, 14 internal edges, layer table holds |
| ui-commands | pass | 0.05 | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | pass | 0.19 | reviews: 64 of 76 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 710 tracked file(s) clean (0 not UTF-8, skipped) |
| schema | pass | 34.73 | 30 commands; audit ok; docs/commands.json current |
| scorecard | **fail** | 342.46 | first line `windows: 64 items; 158 tests ran; 3 registered absent here; page not compared (computed on Linux)`, then `60 scorecard problems:` (62 kept lines; first problem: `building test executables failed on windows (); their tests are not run`; the other 59 are of the form `<item>: lists test `<name>`, which does not exist on windows`) |
| test | **fail** | 2.58 | `0 passed, 0 failed, 0 ignored in 0 suites`; kept text 27 lines; first `error` line `error[E0433]: cannot find `unix` in `os`` (first `-->` line `/rustc/2d8144b7880597b6e6d3dfd63a9a9efae3f533d3/library\std\src\os\mod.rs:84:40`, then `apps\kiln\tests\script_t055.rs:349:14`); ends `error: could not compile `kiln` (test "script_t055") due to 1 previous error`, `error: could not compile `kiln` (test "acceptance_t055") due to 2 previous errors` |
| no-skip | pass | 5.80 | self-test ok (17 known findings); 206 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (10 pending), today = 2026-11-18 |
| fuzz, budgets | empty | 0.0 | no targets registered yet |
- Failing steps of the gate and what each names first: **clippy** (E0599 `mode`, `guard_t037_cli.rs:150:15`), **scorecard** (`building test executables failed on windows (); their tests are not run`), **test** (E0433 `cannot find `unix` in `os``, `script_t055.rs:349:14`); the gate stopped reporting a failed step at `clippy` (`failed_step: clippy`).
- "Test list": 2 s; log `TEST LIST: exit status 101`; `test-list.txt` (10,832 B) holds compile errors only, no binary and no test name. "Full test output": ran (1 s); log `TEST OUTPUT: exit status 101`; `test-output.txt` 10,832 B, 158 non-empty lines, 0 lines `test … FAILED`; its `could not compile` lines: `window_t065` (8 errors), `acceptance_t065` (3), `script_t055` (1), `acceptance_t055` (2).
- Disk (job log): `D:              150G  6.7G  144G   5% /d`; `3.6G	target`.

### Step "Workspace lib and T-073 tests (reported, not required)" (`workspace-tests.txt`)
Binaries: library `running 43 tests`; `acceptance_t073` `running 1 test`; `guard_t073` `running 10 tests`. 54 tests, 0 failed. Log: `WORKSPACE TESTS: exit status 0 (reported, not required)`. Every library test line, verbatim:
```
running 43 tests
test disk::tests::errors_are_root_independent ... ok
test disk::tests::failed_rename_removes_own_temp_and_keeps_target ... ok
test disk::tests::held_root_check_is_inside_only_for_a_read_identity_equal_to_the_held_one ... ok
test disk::tests::check_location_inside_on_every_platform ... ok
test disk::tests::check_location_outside_on_every_platform ... ok
test disk::tests::longest_file_names_can_be_saved ... ok
test disk::tests::new_rejects_missing_root_and_file_root_without_naming_them ... ok
test disk::tests::root_given_through_a_link_on_every_platform ... ok
test disk::tests::link_at_temp_name_is_not_followed_on_every_platform ... ok
test disk::tests::all_temp_names_taken_fails_without_changes_then_recovers ... ok
test disk::tests::root_removed_on_every_platform ... ok
test disk::tests::save_through_a_link_on_every_platform ... ok
test document::tests::dirty_follows_content ... ok
test disk::tests::root_replaced_by_a_link_on_every_platform ... ok
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
test disk::tests::write_read_and_errors ... ok
test project::tests::limit_edges ... ok
test project::tests::negation_and_deeper_files ... ok
test project::tests::under_uses_prefix_gitignore_and_is_not_itself_subject ... ok
test project::tests::worked_example ... ok
test document::tests::disk_save_onto_path_replaced_by_directory_fails_and_changes_nothing ... ok
test disk::tests::matches_memory_model_on_random_operations ... ok

successes:

successes:
    disk::tests::all_temp_names_taken_fails_without_changes_then_recovers
    disk::tests::check_location_inside_on_every_platform
    disk::tests::check_location_outside_on_every_platform
    disk::tests::errors_are_root_independent
    disk::tests::failed_rename_removes_own_temp_and_keeps_target
    disk::tests::held_root_check_is_inside_only_for_a_read_identity_equal_to_the_held_one
    disk::tests::link_at_temp_name_is_not_followed_on_every_platform
    disk::tests::longest_file_names_can_be_saved
    disk::tests::matches_memory_model_on_random_operations
    disk::tests::new_rejects_missing_root_and_file_root_without_naming_them
    disk::tests::root_given_through_a_link_on_every_platform
    disk::tests::root_removed_on_every_platform
    disk::tests::root_replaced_by_a_link_on_every_platform
    disk::tests::save_through_a_link_on_every_platform
    disk::tests::write_read_and_errors
    document::tests::dirty_follows_content
    document::tests::disk_save_onto_path_replaced_by_directory_fails_and_changes_nothing
    document::tests::edits_keep_encoding_and_inserted_newline_stays_lf
    document::tests::failed_save_keeps_file_and_dirty_state
    document::tests::open_errors
    document::tests::unedited_round_trip_is_byte_exact_for_each_encoding
    document::tests::works_through_trait_objects
    encoding::tests::decode_error_display
    encoding::tests::detects_boms_and_strips_exactly_one
    encoding::tests::encode_decode_inverse_on_generated_strings
    encoding::tests::exact_error_offsets
    encoding::tests::names
    encoding::tests::utf16_with_surrogate_pair
    memory::tests::check_location_is_always_ok_and_changes_nothing
    memory::tests::directories_are_implicit
    memory::tests::failing_writes_change_nothing
    memory::tests::list_dir_lists_implied_directories_in_byte_order
    memory::tests::sibling_with_common_prefix_is_not_a_child
    memory::tests::write_into_missing_parent_is_not_found_and_creates_nothing
    memory::tests::write_then_read_round_trips_and_creates_top_level_files
    path::tests::accepts_valid_paths_verbatim
    path::tests::parent_and_segments
    path::tests::rejects_invalid_paths
    project::tests::bom_and_invalid_lines
    project::tests::limit_edges
    project::tests::negation_and_deeper_files
    project::tests::under_uses_prefix_gitignore_and_is_not_itself_subject
    project::tests::worked_example

test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.86s
```
Result lines, verbatim:
```
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.86s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
```
`MEASURED:` lines of this file (2, both from `guard_t073`), verbatim:
```
MEASURED: acw2_root_parent_replaced_is_outside_root: windows: rename of p while the root is held: refused: kind PermissionDenied, raw OS error Some(5) (Access is denied. (os error 5))
MEASURED: acw2_root_replaced_at_same_path_is_outside_root: windows: rename of root while the root is held: ok
```

### Step "kiln-workspace test targets (reported, not required)" (`windows-tests.txt`)
Every `==== <target>: exit status N` line, verbatim (15 targets):
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
==== guard_t074: exit status 0
==== project: exit status 101
==== tools_contract: exit status 0
```
Summary line (job log), verbatim:
```
WINDOWS TESTS: exit status per target: acceptance_t002=101 acceptance_t002_r2=101 acceptance_t008=101 acceptance_t015_tools=101 acceptance_t019_workspace=101 acceptance_t036_workspace=101 acceptance_t073=0 confinement=101 guard_t008=101 guard_t036_removal_workspace=101 guard_t036_workspace=101 guard_t073=0 guard_t074=0 project=101 tools_contract=0 (reported, not required)
```
Other non-zero targets (from the file: first `error` line and closing line, or the failing test; `acceptance_t002` is given in full below):
```
acceptance_t002_r2: does not compile | error[E0433]: cannot find `unix` in `os` | error: could not compile `kiln-workspace` (test "acceptance_t002_r2") due to 11 previous errors
acceptance_t008: does not compile | error[E0433]: cannot find `unix` in `os` | error: could not compile `kiln-workspace` (test "acceptance_t008") due to 20 previous errors
acceptance_t015_tools: compiles, fails
     test no_tools_never_runs ... FAILED
     test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     thread 'no_tools_never_runs' (9284) panicked at crates\kiln-workspace\tests\acceptance_t015_tools.rs:131:5:
     precondition: the fake `cargo`/`rustfmt` of this test are shell scripts and need Unix
acceptance_t019_workspace: does not compile | error[E0433]: cannot find `unix` in `os` | error: could not compile `kiln-workspace` (test "acceptance_t019_workspace") due to 5 previous errors
acceptance_t036_workspace: does not compile | error[E0433]: cannot find `unix` in `os` | error: could not compile `kiln-workspace` (test "acceptance_t036_workspace") due to 16 previous errors
confinement: does not compile | error[E0433]: cannot find `unix` in `os` | error: could not compile `kiln-workspace` (test "confinement") due to 24 previous errors
guard_t008: does not compile | error[E0433]: cannot find `unix` in `os` | error: could not compile `kiln-workspace` (test "guard_t008") due to 1 previous error
guard_t036_removal_workspace: does not compile | error[E0433]: cannot find `unix` in `os` | error: could not compile `kiln-workspace` (test "guard_t036_removal_workspace") due to 3 previous errors
guard_t036_workspace: does not compile | error[E0433]: cannot find `unix` in `os` | error: could not compile `kiln-workspace` (test "guard_t036_workspace") due to 1 previous error
project: does not compile | error[E0433]: cannot find `unix` in `os` | error: could not compile `kiln-workspace` (test "project") due to 3 previous errors
```

#### `guard_t074` (exit status 0): the 15 test lines and the result line, verbatim
```
running 15 tests
test awa1_file_link_relative_on_the_walk ... ok
test awa1_file_link_absolute_on_the_walk ... ok
test awa3_temp_name_held_by_a_dangling_file_link_to_outside ... ok
test awa3_temp_name_held_by_a_dangling_junction ... ok
test awa3_temp_name_held_by_a_file_link_to_an_outside_file ... ok
test awa3_temp_name_held_by_a_junction ... ok
test awa1_directory_link_on_the_walk ... ok
test awa4_dotdot_after_a_junction_in_the_root_text ... ok
test awa3_temp_names_held_by_a_junction_and_a_dangling_file_link ... ok
test awa1_junction_on_the_walk ... ok
test awa4_root_given_through_a_directory_link_and_retargeted ... ok
test awa4_root_removed_while_held ... ok
test awa4_root_given_through_a_junction_and_retargeted ... ok
test awa4_root_replaced_by_a_junction ... ok
test awa1_link_below_the_first_level ... ok
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.52s
```
No test failed, so there is no failing stdout and no panic. `grep -c MEASURED:` over `windows-tests.txt` gives 178: 176 inside `guard_t074` (the awk over lines 1006-1256 of the ANSI-stripped file, which prints each `---- <test> stdout ----` header and the `MEASURED:` lines after it, gave 176 lines under 15 headers) and 2 inside `guard_t073`. Every `MEASURED:` line of `guard_t074`, in order, under the `---- <test> stdout ----` block it belongs to (the file's order of blocks):
```
---- awa1_file_link_relative_on_the_walk stdout ----
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Inside as Middle component, at the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Inside as Middle component, at the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l/x.txt") }))
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Inside as Middle component, at the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Inside as Last component, at the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Inside as Last component, at the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l") }))
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Inside as Last component, at the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Outside as Middle component, at the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Outside as Middle component, at the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l/x.txt") }))
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Outside as Middle component, at the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Outside as Last component, at the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Outside as Last component, at the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l") }))
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Outside as Last component, at the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Nowhere as Middle component, at the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Nowhere as Middle component, at the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l/x.txt") }))
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Nowhere as Middle component, at the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Nowhere as Last component, at the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Nowhere as Last component, at the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l") }))
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Nowhere as Last component, at the first level: windows: write_atomic OutsideRoot
---- awa1_file_link_absolute_on_the_walk stdout ----
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Inside as Middle component, at the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Inside as Middle component, at the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l/x.txt") }))
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Inside as Middle component, at the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Inside as Last component, at the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Inside as Last component, at the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l") }))
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Inside as Last component, at the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Outside as Middle component, at the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Outside as Middle component, at the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l/x.txt") }))
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Outside as Middle component, at the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Outside as Last component, at the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Outside as Last component, at the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l") }))
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Outside as Last component, at the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Nowhere as Middle component, at the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Nowhere as Middle component, at the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l/x.txt") }))
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Nowhere as Middle component, at the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Nowhere as Last component, at the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Nowhere as Last component, at the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l") }))
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Nowhere as Last component, at the first level: windows: write_atomic OutsideRoot
---- awa3_temp_name_held_by_a_dangling_file_link_to_outside stdout ----
MEASURED: awa3_temp_name_held_by_a_dangling_file_link_to_outside: windows: fixture: FileAbs made at .kiln-tmp-0 -> D:\a\photocraft-study-review\photocraft-study-review\target\tmp\guard_t074\awa3_temp_name_held_by_a_dangling_file_link_to_outside-5908\out\missing (target exists: false)
MEASURED: awa3_temp_name_held_by_a_dangling_file_link_to_outside: windows: write_atomic(f.txt) beside the links: Ok(())
MEASURED: awa3_temp_name_held_by_a_dangling_file_link_to_outside: windows: Document save of f.txt beside the links: Ok(())
---- awa3_temp_name_held_by_a_dangling_junction stdout ----
MEASURED: awa3_temp_name_held_by_a_dangling_junction: windows: fixture: Junction made at .kiln-tmp-0 -> D:\a\photocraft-study-review\photocraft-study-review\target\tmp\guard_t074\awa3_temp_name_held_by_a_dangling_junction-5908\out\gone (target exists: false)
MEASURED: awa3_temp_name_held_by_a_dangling_junction: windows: write_atomic(f.txt) beside the links: Ok(())
MEASURED: awa3_temp_name_held_by_a_dangling_junction: windows: Document save of f.txt beside the links: Ok(())
---- awa3_temp_name_held_by_a_file_link_to_an_outside_file stdout ----
MEASURED: awa3_temp_name_held_by_a_file_link_to_an_outside_file: windows: fixture: FileAbs made at .kiln-tmp-0 -> D:\a\photocraft-study-review\photocraft-study-review\target\tmp\guard_t074\awa3_temp_name_held_by_a_file_link_to_an_outside_file-5908\out\t.txt (target exists: true)
MEASURED: awa3_temp_name_held_by_a_file_link_to_an_outside_file: windows: write_atomic(f.txt) beside the links: Ok(())
MEASURED: awa3_temp_name_held_by_a_file_link_to_an_outside_file: windows: Document save of f.txt beside the links: Ok(())
---- awa3_temp_name_held_by_a_junction stdout ----
MEASURED: awa3_temp_name_held_by_a_junction: windows: fixture: Junction made at .kiln-tmp-0 -> D:\a\photocraft-study-review\photocraft-study-review\target\tmp\guard_t074\awa3_temp_name_held_by_a_junction-5908\out (target exists: true)
MEASURED: awa3_temp_name_held_by_a_junction: windows: write_atomic(f.txt) beside the links: Ok(())
MEASURED: awa3_temp_name_held_by_a_junction: windows: Document save of f.txt beside the links: Ok(())
---- awa1_directory_link_on_the_walk stdout ----
MEASURED: awa1_directory_link_on_the_walk: DirSym to Inside as Middle component, at the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_directory_link_on_the_walk: DirSym to Inside as Middle component, at the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l/x.txt") }))
MEASURED: awa1_directory_link_on_the_walk: DirSym to Inside as Middle component, at the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_directory_link_on_the_walk: DirSym to Inside as Last component, at the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_directory_link_on_the_walk: DirSym to Inside as Last component, at the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l") }))
MEASURED: awa1_directory_link_on_the_walk: DirSym to Inside as Last component, at the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_directory_link_on_the_walk: DirSym to Outside as Middle component, at the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_directory_link_on_the_walk: DirSym to Outside as Middle component, at the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l/x.txt") }))
MEASURED: awa1_directory_link_on_the_walk: DirSym to Outside as Middle component, at the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_directory_link_on_the_walk: DirSym to Outside as Last component, at the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_directory_link_on_the_walk: DirSym to Outside as Last component, at the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l") }))
MEASURED: awa1_directory_link_on_the_walk: DirSym to Outside as Last component, at the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_directory_link_on_the_walk: DirSym to Nowhere as Middle component, at the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_directory_link_on_the_walk: DirSym to Nowhere as Middle component, at the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l/x.txt") }))
MEASURED: awa1_directory_link_on_the_walk: DirSym to Nowhere as Middle component, at the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_directory_link_on_the_walk: DirSym to Nowhere as Last component, at the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_directory_link_on_the_walk: DirSym to Nowhere as Last component, at the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l") }))
MEASURED: awa1_directory_link_on_the_walk: DirSym to Nowhere as Last component, at the first level: windows: write_atomic OutsideRoot
---- awa4_dotdot_after_a_junction_in_the_root_text stdout ----
MEASURED: awa4_dotdot_after_a_junction_in_the_root_text: --root k/link/../b: windows: read f.txt: Ok
---- awa3_temp_names_held_by_a_junction_and_a_dangling_file_link stdout ----
MEASURED: awa3_temp_names_held_by_a_junction_and_a_dangling_file_link: windows: fixture: Junction made at .kiln-tmp-0 -> D:\a\photocraft-study-review\photocraft-study-review\target\tmp\guard_t074\awa3_temp_names_held_by_a_junction_and_a_dangling_file_link-5908\out (target exists: true)
MEASURED: awa3_temp_names_held_by_a_junction_and_a_dangling_file_link: windows: fixture: FileAbs made at .kiln-tmp-1 -> D:\a\photocraft-study-review\photocraft-study-review\target\tmp\guard_t074\awa3_temp_names_held_by_a_junction_and_a_dangling_file_link-5908\out\t (target exists: false)
MEASURED: awa3_temp_names_held_by_a_junction_and_a_dangling_file_link: windows: write_atomic(f.txt) beside the links: Ok(())
MEASURED: awa3_temp_names_held_by_a_junction_and_a_dangling_file_link: windows: Document save of f.txt beside the links: Ok(())
---- awa1_junction_on_the_walk stdout ----
MEASURED: awa1_junction_on_the_walk: Junction to Inside as Middle component, at the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_junction_on_the_walk: Junction to Inside as Middle component, at the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l/x.txt") }))
MEASURED: awa1_junction_on_the_walk: Junction to Inside as Middle component, at the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_junction_on_the_walk: Junction to Inside as Last component, at the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_junction_on_the_walk: Junction to Inside as Last component, at the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l") }))
MEASURED: awa1_junction_on_the_walk: Junction to Inside as Last component, at the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_junction_on_the_walk: Junction to Outside as Middle component, at the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_junction_on_the_walk: Junction to Outside as Middle component, at the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l/x.txt") }))
MEASURED: awa1_junction_on_the_walk: Junction to Outside as Middle component, at the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_junction_on_the_walk: Junction to Outside as Last component, at the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_junction_on_the_walk: Junction to Outside as Last component, at the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l") }))
MEASURED: awa1_junction_on_the_walk: Junction to Outside as Last component, at the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_junction_on_the_walk: Junction to Nowhere as Middle component, at the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_junction_on_the_walk: Junction to Nowhere as Middle component, at the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l/x.txt") }))
MEASURED: awa1_junction_on_the_walk: Junction to Nowhere as Middle component, at the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_junction_on_the_walk: Junction to Nowhere as Last component, at the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_junction_on_the_walk: Junction to Nowhere as Last component, at the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l") }))
MEASURED: awa1_junction_on_the_walk: Junction to Nowhere as Last component, at the first level: windows: write_atomic OutsideRoot
---- awa4_root_given_through_a_directory_link_and_retargeted stdout ----
MEASURED: awa4_root_given_through_a_directory_link_and_retargeted: windows: DirSym as --root: read f.txt: Ok([114, 101, 97, 108, 10])
MEASURED: awa4_root_given_through_a_directory_link_and_retargeted: DirSym retargeted to another directory: windows: read OutsideRoot, write_atomic OutsideRoot, is_file false, list_dir OutsideRoot, check_location OutsideRoot, save OutsideRoot
MEASURED: awa4_root_given_through_a_directory_link_and_retargeted: DirSym pointed back: windows: read f.txt: Ok
---- awa4_root_removed_while_held stdout ----
MEASURED: awa4_root_removed_while_held: windows: remove_dir of the held empty root: Ok(())
MEASURED: awa4_root_removed_while_held: empty root removed: windows: read OutsideRoot, write_atomic OutsideRoot, is_file false, list_dir OutsideRoot, check_location OutsideRoot, save -
MEASURED: awa4_root_removed_while_held: empty root removed, a new directory at its name: windows: read OutsideRoot, write_atomic OutsideRoot, is_file false, list_dir OutsideRoot, check_location OutsideRoot, save -
MEASURED: awa4_root_removed_while_held: windows: remove_dir_all of the held root with a file: Ok(())
MEASURED: awa4_root_removed_while_held: root with a file removed: windows: read OutsideRoot, write_atomic OutsideRoot, is_file false, list_dir OutsideRoot, check_location OutsideRoot, save OutsideRoot
MEASURED: awa4_root_removed_while_held: root removed, a junction to a new directory at its name: windows: read OutsideRoot, write_atomic OutsideRoot, is_file false, list_dir OutsideRoot, check_location OutsideRoot, save OutsideRoot
---- awa4_root_given_through_a_junction_and_retargeted stdout ----
MEASURED: awa4_root_given_through_a_junction_and_retargeted: windows: Junction as --root: read f.txt: Ok([114, 101, 97, 108, 10])
MEASURED: awa4_root_given_through_a_junction_and_retargeted: Junction retargeted to another directory: windows: read OutsideRoot, write_atomic OutsideRoot, is_file false, list_dir OutsideRoot, check_location OutsideRoot, save OutsideRoot
MEASURED: awa4_root_given_through_a_junction_and_retargeted: Junction pointed back: windows: read f.txt: Ok
---- awa4_root_replaced_by_a_junction stdout ----
MEASURED: awa4_root_replaced_by_a_junction: windows: rename of the held root: Ok(())
MEASURED: awa4_root_replaced_by_a_junction: a junction to another directory at the root's path: windows: read OutsideRoot, write_atomic OutsideRoot, is_file false, list_dir OutsideRoot, check_location OutsideRoot, save OutsideRoot
MEASURED: awa4_root_replaced_by_a_junction: a junction at the root's path to the held root: windows: read f.txt: Ok
---- awa1_link_below_the_first_level stdout ----
MEASURED: awa1_link_below_the_first_level: FileRel to Inside as Middle component, below the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_link_below_the_first_level: FileRel to Inside as Middle component, below the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("d/l/x.txt") }))
MEASURED: awa1_link_below_the_first_level: FileRel to Inside as Middle component, below the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_link_below_the_first_level: FileRel to Inside as Last component, below the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_link_below_the_first_level: FileRel to Inside as Last component, below the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("d/l") }))
MEASURED: awa1_link_below_the_first_level: FileRel to Inside as Last component, below the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_link_below_the_first_level: FileRel to Outside as Middle component, below the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_link_below_the_first_level: FileRel to Outside as Middle component, below the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("d/l/x.txt") }))
MEASURED: awa1_link_below_the_first_level: FileRel to Outside as Middle component, below the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_link_below_the_first_level: FileRel to Outside as Last component, below the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_link_below_the_first_level: FileRel to Outside as Last component, below the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("d/l") }))
MEASURED: awa1_link_below_the_first_level: FileRel to Outside as Last component, below the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_link_below_the_first_level: FileRel to Nowhere as Middle component, below the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_link_below_the_first_level: FileRel to Nowhere as Middle component, below the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("d/l/x.txt") }))
MEASURED: awa1_link_below_the_first_level: FileRel to Nowhere as Middle component, below the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_link_below_the_first_level: FileRel to Nowhere as Last component, below the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_link_below_the_first_level: FileRel to Nowhere as Last component, below the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("d/l") }))
MEASURED: awa1_link_below_the_first_level: FileRel to Nowhere as Last component, below the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_link_below_the_first_level: FileAbs to Inside as Middle component, below the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_link_below_the_first_level: FileAbs to Inside as Middle component, below the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("d/l/x.txt") }))
MEASURED: awa1_link_below_the_first_level: FileAbs to Inside as Middle component, below the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_link_below_the_first_level: FileAbs to Inside as Last component, below the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_link_below_the_first_level: FileAbs to Inside as Last component, below the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("d/l") }))
MEASURED: awa1_link_below_the_first_level: FileAbs to Inside as Last component, below the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_link_below_the_first_level: FileAbs to Outside as Middle component, below the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_link_below_the_first_level: FileAbs to Outside as Middle component, below the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("d/l/x.txt") }))
MEASURED: awa1_link_below_the_first_level: FileAbs to Outside as Middle component, below the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_link_below_the_first_level: FileAbs to Outside as Last component, below the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_link_below_the_first_level: FileAbs to Outside as Last component, below the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("d/l") }))
MEASURED: awa1_link_below_the_first_level: FileAbs to Outside as Last component, below the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_link_below_the_first_level: FileAbs to Nowhere as Middle component, below the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_link_below_the_first_level: FileAbs to Nowhere as Middle component, below the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("d/l/x.txt") }))
MEASURED: awa1_link_below_the_first_level: FileAbs to Nowhere as Middle component, below the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_link_below_the_first_level: FileAbs to Nowhere as Last component, below the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_link_below_the_first_level: FileAbs to Nowhere as Last component, below the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("d/l") }))
MEASURED: awa1_link_below_the_first_level: FileAbs to Nowhere as Last component, below the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_link_below_the_first_level: DirSym to Inside as Middle component, below the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_link_below_the_first_level: DirSym to Inside as Middle component, below the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("d/l/x.txt") }))
MEASURED: awa1_link_below_the_first_level: DirSym to Inside as Middle component, below the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_link_below_the_first_level: DirSym to Inside as Last component, below the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_link_below_the_first_level: DirSym to Inside as Last component, below the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("d/l") }))
MEASURED: awa1_link_below_the_first_level: DirSym to Inside as Last component, below the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_link_below_the_first_level: DirSym to Outside as Middle component, below the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_link_below_the_first_level: DirSym to Outside as Middle component, below the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("d/l/x.txt") }))
MEASURED: awa1_link_below_the_first_level: DirSym to Outside as Middle component, below the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_link_below_the_first_level: DirSym to Outside as Last component, below the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_link_below_the_first_level: DirSym to Outside as Last component, below the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("d/l") }))
MEASURED: awa1_link_below_the_first_level: DirSym to Outside as Last component, below the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_link_below_the_first_level: DirSym to Nowhere as Middle component, below the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_link_below_the_first_level: DirSym to Nowhere as Middle component, below the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("d/l/x.txt") }))
MEASURED: awa1_link_below_the_first_level: DirSym to Nowhere as Middle component, below the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_link_below_the_first_level: DirSym to Nowhere as Last component, below the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_link_below_the_first_level: DirSym to Nowhere as Last component, below the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("d/l") }))
MEASURED: awa1_link_below_the_first_level: DirSym to Nowhere as Last component, below the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_link_below_the_first_level: Junction to Inside as Middle component, below the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_link_below_the_first_level: Junction to Inside as Middle component, below the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("d/l/x.txt") }))
MEASURED: awa1_link_below_the_first_level: Junction to Inside as Middle component, below the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_link_below_the_first_level: Junction to Inside as Last component, below the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_link_below_the_first_level: Junction to Inside as Last component, below the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("d/l") }))
MEASURED: awa1_link_below_the_first_level: Junction to Inside as Last component, below the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_link_below_the_first_level: Junction to Outside as Middle component, below the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_link_below_the_first_level: Junction to Outside as Middle component, below the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("d/l/x.txt") }))
MEASURED: awa1_link_below_the_first_level: Junction to Outside as Middle component, below the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_link_below_the_first_level: Junction to Outside as Last component, below the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_link_below_the_first_level: Junction to Outside as Last component, below the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("d/l") }))
MEASURED: awa1_link_below_the_first_level: Junction to Outside as Last component, below the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_link_below_the_first_level: Junction to Nowhere as Middle component, below the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_link_below_the_first_level: Junction to Nowhere as Middle component, below the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("d/l/x.txt") }))
MEASURED: awa1_link_below_the_first_level: Junction to Nowhere as Middle component, below the first level: windows: write_atomic OutsideRoot
MEASURED: awa1_link_below_the_first_level: Junction to Nowhere as Last component, below the first level: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_link_below_the_first_level: Junction to Nowhere as Last component, below the first level: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("d/l") }))
MEASURED: awa1_link_below_the_first_level: Junction to Nowhere as Last component, below the first level: windows: write_atomic OutsideRoot
```

#### `acceptance_t002` (exit status 101): every test line, then the failing test's block, verbatim
```
running 15 tests
test bom_only_file_round_trips ... ok
test disk_save_replaces_via_rename_without_leftovers ... FAILED
test dirty_follows_text_versus_last_saved ... ok
test invalid_bytes_are_reported_with_offset_never_replaced ... ok
test line_endings_round_trip_verbatim ... ok
test failed_save_leaves_file_intact_and_document_dirty ... ok
test relpath_accepts_and_rejects_exact_cases ... ok
test save_through_symlink_updates_target ... ok
test encode_decode_round_trip ... ok
test utf16_le_and_be_with_bom_open_and_save ... ok
test unedited_utf8_files_round_trip_byte_for_byte ... ok
test utf8_bom_detected_stripped_and_restored_on_save ... ok
test vfs_errors_name_relative_paths_only ... ok
test disk_vfs_results_do_not_depend_on_root ... ok
test memory_and_disk_vfs_agree_on_random_operation_sequences ... ok

---- disk_save_replaces_via_rename_without_leftovers stdout ----

thread 'disk_save_replaces_via_rename_without_leftovers' (5348) panicked at crates\kiln-workspace\tests\acceptance_t002.rs:940:5:
precondition: permission bits and inode numbers are Unix metadata; the Windows checks of atomic replace are not written yet (ADR-0009: T-073 and T-074 write the Windows behaviour and its tests)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    disk_save_replaces_via_rename_without_leftovers

test result: FAILED. 14 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
```

#### `acceptance_t015_tools` (exit status 101), failing test's block, verbatim
```
---- no_tools_never_runs stdout ----

thread 'no_tools_never_runs' (9284) panicked at crates\kiln-workspace\tests\acceptance_t015_tools.rs:131:5:
precondition: the fake `cargo`/`rustfmt` of this test are shell scripts and need Unix
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    no_tools_never_runs

test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## `xtask` tests (`core-crates.txt`, block `==== cargo test --locked --no-fail-fast -p xtask`)
Commands: ANSI-stripped file; `awk '/-p xtask$/{x=1}x&&/Running/{r=$0}x&&/^test result/{print r" => "$0}'` and the same with `p+=$4;f+=$6` (script `xt.sh` in the scratch directory, removed with it). 27 `test result:` lines in the block on each platform.
| platform | `==== xtask: exit status` | whole package | unit-test binary | `acceptance_t074` | `acceptance_t072` | `guard_t072` | failing tests |
|---|---|---|---|---|---|---|---|
| Linux | 0 | 318 passed, 0 failed | 138 passed, 3.00 s | `2 passed; 0 failed`, 0.00 s | `20 passed; 0 failed`, 0.45 s | `8 passed; 0 failed`, 125.20 s | none |
| Windows | 0 | 310 passed, 0 failed | 134 passed, 99.82 s | `2 passed; 0 failed`, 0.01 s | `20 passed; 0 failed`, 2.38 s | `8 passed; 0 failed`, 127.76 s | none |
| macOS | 0 | 313 passed, 0 failed | 136 passed, 20.94 s | `2 passed; 0 failed`, 0.01 s | `20 passed; 0 failed`, 1.28 s | `8 passed; 0 failed`, 135.68 s | none |
`kiln-text` 78, `kiln-syntax` 53, `kiln-search` 55 passed (0 failed), each `exit status 0`, on all three platforms. `grep -cE '^test .* \.\.\. FAILED'` over each `core-crates.txt` gives 0 on all three. The test `g1_probe_with_a_kiln_that_never_exits_ends_within_180_s_and_records_the_kill` (`guard_t072`): verbatim `test g1_probe_with_a_kiln_that_never_exits_ends_within_180_s_and_records_the_kill ... ok` in each of the three `core-crates.txt`. `queue::tests::main_xtask_is_rebuilt_from_base_on_every_run`: verbatim `test queue::tests::main_xtask_is_rebuilt_from_base_on_every_run ... ok` on Linux (`core-crates.txt` line 409) and macOS (line 410); **absent** on Windows (no line with that name; unit-test binary ran, 134 passed).

## gate-linux (job 134992872417, ubuntu-24.04)
- Runner image (job log): `Image: ubuntu-24.04`, `Version: 20261111.127.1`; runner 2.338.2. Rust versions (cache step): `1.97.0 x86_64-unknown-linux-gnu 2d8144b7880597b6e6d3dfd63a9a9efae3f533d3`, `1.99.1 x86_64-unknown-linux-gnu 58cf8f9c35b355199727627d07ae34a4e2846d67`; `cargo 1.97.0 (c980f4866 2026-06-30)`; cache `Cache hit for: v0-rust-gate-linux-Linux-x64-f37a6dc5-6187dc6e`, `full match: true`.
| step | conclusion | start–end (Z) | duration |
|---|---|---|---|
| Set up job | success | 17:25:14–17:25:17 | 3s |
| Checkout | success | 17:25:17–17:25:22 | 5s |
| Install the pinned toolchain | success | 17:25:22–17:25:31 | 9s |
| Cache cargo registry and target | success | 17:25:31–17:25:42 | 11s |
| Gate | success | 17:25:42–17:36:13 | 10m31s |
| Gate report | success | 17:36:13 | 0s |
| Core crates (reported, not required) | success | 17:36:13–17:41:25 | 5m12s |
| Test list | success | 17:41:25–17:41:26 | 1s |
| Full test output (gate failed) | **skipped** | 17:41:26 | 0s |
| Disk | success | 17:41:26 | 0s |
| Upload reports | success | 17:41:26–17:41:28 | 2s |
| Post Cache, Post Checkout, Complete job | success | 17:41:28 | 0s each |
- Log lines: `TEST LIST: exit status 0`. Disk (log): `/dev/mapper/cow-root  145G   62G   83G  43% /`; `4.6G	target`.
- Gate report: `passed: true`, `failed_step: null`, 627.98 s.
| step | status | seconds | detail |
|---|---|---|---|
| fmt | pass | 0.87 | formatted |
| clippy | pass | 9.42 | no warnings (all targets, -D warnings) |
| layers | pass | 0.44 | 11 crates, 14 internal edges, layer table holds |
| ui-commands | pass | 0.03 | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | pass | 0.06 | reviews: 64 of 76 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 710 tracked file(s) clean (0 not UTF-8, skipped) |
| schema | pass | 4.41 | 30 commands; audit ok; docs/commands.json current |
| scorecard | pass | 150.34 | `64 items: 47 verified, 0 implemented, 0 failing, 17 planned.` |
| test | pass | 460.18 | `1200 passed, 0 failed, 0 ignored in 140 suites` |
| no-skip | pass | 2.23 | self-test ok (17 known findings); 206 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (10 pending), today = 2026-11-18 |
| fuzz, budgets | empty | 0.0 | no targets registered yet |
- Totals: gate `test` step 1200 passed, 0 failed; `xtask` 318 passed, 0 failed (table above); `kiln-text`/`kiln-syntax`/`kiln-search` 78/53/55 passed. Failing tests: none (gate green; `core-crates.txt` has no `FAILED` line).
- `test-list.txt` (78,486 B, 1,605 lines; 131 `Running` lines + 8 `Doc-tests`; 1,199 lines ending `: test`; the gate's count 1200 was not reconciled with the list) — no `test-output.txt` (green gate), so per-test results are not in the artefact; the gate's `test` step passed with 0 failed.
- `guard_t074` (kiln-workspace) — `Running tests/guard_t074.rs`, `15 tests, 0 benchmarks`; names listed, verbatim: `awa1_directory_link_on_the_walk`, `awa1_file_link_absolute_on_the_walk`, `awa1_file_link_relative_on_the_walk`, `awa1_junction_on_the_walk`, `awa1_link_below_the_first_level`, `awa3_temp_name_held_by_a_dangling_file_link_to_outside`, `awa3_temp_name_held_by_a_dangling_junction`, `awa3_temp_name_held_by_a_file_link_to_an_outside_file`, `awa3_temp_name_held_by_a_junction`, `awa3_temp_names_held_by_a_junction_and_a_dangling_file_link`, `awa4_dotdot_after_a_junction_in_the_root_text`, `awa4_root_given_through_a_directory_link_and_retargeted`, `awa4_root_given_through_a_junction_and_retargeted`, `awa4_root_removed_while_held`, `awa4_root_replaced_by_a_junction`.
- `acceptance_t074` (xtask) — `Running tests/acceptance_t074.rs`, `2 tests, 0 benchmarks`: `ac3_each_of_the_nine_entries_names_exactly_its_counterpart`, `ac3_no_entry_waits_on_t074`. (Passed in the `xtask` block: `2 passed; 0 failed`.)
- `kiln-workspace` library — `58 tests, 0 benchmarks` on Linux (and the same 58 names on macOS: `diff` of the two sorted name lists is empty); the Windows run of the library (`workspace-tests.txt`) has 43 of them. No library test name on Linux contains `_windows` (`grep -i windows` over the 58 names gives no line), so the "seven tests whose names contain `_windows`" cannot be listed from this artefact; which tests are new against main is not determinable (no list of main in this run). The 15 library tests listed on Linux and absent from the Windows library run, verbatim: `disk::tests::check_location_is_ok_for_every_inside_location`, `disk::tests::check_location_refuses_outside_loops_and_a_replaced_root`, `disk::tests::failed_save_after_writability_check_leaves_target_and_listing_unchanged`, `disk::tests::leftover_at_temp_name_is_never_touched`, `disk::tests::link_target_with_trailing_slash_requires_a_directory`, `disk::tests::new_file_gets_default_mode`, `disk::tests::parent_replaced_by_a_symlink_between_requests_is_outside`, `disk::tests::read_only_target_is_refused_unless_root`, `disk::tests::replace_swaps_inode_keeps_permissions_and_follows_symlinks`, `disk::tests::root_given_through_a_link_that_is_retargeted`, `disk::tests::root_removed_and_replaced_by_a_link_to_a_new_directory_is_outside`, `disk::tests::root_replaced_by_a_symlink_between_requests_is_outside`, `disk::tests::save_creates_temp_owner_only_for_an_existing_target`, `disk::tests::symlink_at_temp_name_is_not_followed`, `disk::tests::temp_file_is_created_owner_only_and_gets_target_mode_before_rename` (no name on the Windows run is absent from the Linux list).

## gate-macos (job 134992871807, runner label macos-15) — runner assigned (`runner_id` 1000001066)
- Runner image (job log): `Image: macos-15-arm64`, `Version: 20261112.0539.3`; runner 2.338.2; `cargo 1.97.0 (c980f4866 2026-06-30)`; cache `Cache hit for: v0-rust-gate-macos-Darwin-arm64-03a9c7df-6187dc6e`, `full match: true`.
- Job: `status` completed, conclusion **success**; created 17:25:10Z, started 17:30:13Z, completed 17:58:34Z; wall 28m21s (started to completed). All steps completed.
| step | conclusion | start | end | duration |
|---|---|---|---|---|
| Set up job | success | 17:30:13Z | 17:30:16Z | 3s |
| Checkout | success | 17:30:16Z | 17:30:19Z | 3s |
| Install the pinned toolchain (rust-toolchain.toml) | success | 17:30:19Z | 17:30:30Z | 11s |
| Cache cargo registry and target | success | 17:30:30Z | 17:30:47Z | 17s |
| Gate | success | 17:30:47Z | 17:49:05Z | 18m18s |
| Gate report | success | 17:49:05Z | 17:49:05Z | 0s |
| Core crates (reported, not required) | success | 17:49:05Z | 17:58:23Z | 9m18s |
| Test list | success | 17:58:23Z | 17:58:27Z | 4s |
| Full test output (gate failed) | **skipped** | 17:58:27Z | 17:58:27Z | 0s |
| Disk | success | 17:58:27Z | 17:58:28Z | 1s |
| Upload reports | success | 17:58:28Z | 17:58:29Z | 1s |
| Post Cache cargo registry and target | success | 17:58:29Z | 17:58:30Z | 1s |
| Post Checkout | success | 17:58:30Z | 17:58:30Z | 0s |
| Complete job | success | 17:58:30Z | 17:58:32Z | 2s |
- Log: `TEST LIST: exit status 0`. Disk (log): `/dev/disk2s5   320Gi   262Gi    40Gi    87%    2.3M  420M    1%   /System/Volumes/Data`; `3.2G	target`.
- Artefact `gate-macos-44358453526`, id 14456243039, 31,673 B (size equal, `testzip()` clean; created 17:58:30Z): `core-crates.txt` 40,686 B, `gate-report.json` 2,024 B, `test-list.txt` 77,528 B; no `test-output.txt` (green gate).
- Gate report: `passed: true`, `failed_step: null`, 1088.12 s.
| step | status | seconds | detail |
|---|---|---|---|
| fmt | pass | 1.12 | formatted |
| clippy | pass | 15.28 | no warnings (all targets, -D warnings) |
| layers | pass | 1.66 | 11 crates, 14 internal edges, layer table holds |
| ui-commands | pass | 0.04 | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | pass | 0.32 | reviews: 64 of 76 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 710 tracked file(s) clean (0 not UTF-8, skipped) |
| schema | pass | 6.63 | 30 commands; audit ok; docs/commands.json current |
| scorecard | pass | 241.24 | `macos: 64 items; 216 tests ran; 3 registered absent here; page not compared (computed on Linux)` |
| test | pass | 818.33 | `1183 passed, 0 failed, 0 ignored in 140 suites` |
| no-skip | pass | 3.50 | self-test ok (17 known findings); 206 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (10 pending), today = 2026-11-18 |
| fuzz, budgets | empty | 0.0 | no targets registered yet |
- Failing tests: none (gate green; `grep -c FAILED core-crates.txt` gives 0). In particular no `guard_t072` test failed: `guard_t072` `8 passed; 0 failed`, 135.68 s (xtask table above); `g1_probe_with_a_kiln_that_never_exits_ends_within_180_s_and_records_the_kill ... ok`.
- `test-list.txt` (77,528 B, 1,588 lines; 131 `Running` lines + 8 `Doc-tests`; 1,182 lines ending `: test`). `guard_t074` (`Running tests/guard_t074.rs`, `15 tests, 0 benchmarks`), names verbatim: `awa1_directory_link_on_the_walk`, `awa1_file_link_absolute_on_the_walk`, `awa1_file_link_relative_on_the_walk`, `awa1_junction_on_the_walk`, `awa1_link_below_the_first_level`, `awa3_temp_name_held_by_a_dangling_file_link_to_outside`, `awa3_temp_name_held_by_a_dangling_junction`, `awa3_temp_name_held_by_a_file_link_to_an_outside_file`, `awa3_temp_name_held_by_a_junction`, `awa3_temp_names_held_by_a_junction_and_a_dangling_file_link`, `awa4_dotdot_after_a_junction_in_the_root_text`, `awa4_root_given_through_a_directory_link_and_retargeted`, `awa4_root_given_through_a_junction_and_retargeted`, `awa4_root_removed_while_held`, `awa4_root_replaced_by_a_junction` (the same 15 as on Linux). `guard_t072` (xtask, `Running tests/guard_t072.rs`, `8 tests, 0 benchmarks`): `g1_probe_with_a_kiln_that_never_exits_ends_within_180_s_and_records_the_kill`, `g2_every_recording_step_ends_with_exit_0`, `g2_every_recording_step_exits_0_when_every_command_it_calls_fails`, `g3_no_continue_on_error_and_every_uses_pinned_by_sha`, `g4_window_jobs_write_and_upload_only_their_own_records`, `guard_evidence_steps_conditions_are_exact_per_gate_job`, `guard_jobs_labels_conditions_and_the_gate_step_decide_the_colour`, `guard_t073_step_keeps_its_command_file_and_condition`. Per-test results of `guard_t074` on macOS: not in the artefact (green gate, no `test-output.txt`); the gate's `test` step passed with 0 failed.

## window jobs
- `window-linux` (job 134992872590): success, 1m41s (17:25:13Z–17:26:54Z); runner assigned (1000001060); artefact `window-linux-44358453526` 577 B listed, not downloaded.
- `window-probe (windows-2025)` (job 134992872391): **failure**, 1m51s; Build kiln success (17:25:41Z–17:26:57Z), Window probe failure (17:26:57Z–17:26:58Z), Upload the probe's record success. Artefact `window-probe-windows-2025-44358453526` (344 B) read; `window-probe.txt`, verbatim:
```
PROBE OS: windows-2025 MINGW64_NT-10.0-26100 runnervmm7ud1 3.6.10-5a1665c8.x86_64 2026-09-28 10:40 UTC x86_64 Msys
PROBE: exit status 2
PROBE: stdout 0 bytes, 0 lines
--- stdout
--- stderr
kiln: no window: egui_glow: OpenGL: egui_glow requires opengl 2.0+.
--- end
```
- `window-probe (macos-15)` (job 134992872548): **cancelled**, 15m01s (17:25:10Z–17:40:11Z); **no runner assigned** (no `runner_id`/`runner_name`, no steps); no `window-probe-macos-15-…` artefact in the final listing (5 artefacts: the four gate/window ones above and `gate-macos`), so no `PROBE:` exit-status line exists for it.

## Compared with the second early look (`T-074-early-2.md`, run 44327488962 @ 9d6493c)
Values of this run first, then the early-2 value.
- Run conclusion: failure here, failure there. Cause by job: `gate-windows` failure (both); `window-probe (windows-2025)` failure (both); `window-probe (macos-15)` cancelled, no runner (both); **`gate-macos`: success here (runner assigned), failure there** (`guard_t072` `g1_…` 6 passed 1 failed); gate-linux success (both); window-linux success (both).
- gate-macos: gate `test` step `1183 passed, 0 failed, 0 ignored in 140 suites` here, `1171 passed, 1 failed, 0 ignored in 130 suites` there; no `test-output.txt` here (green gate), there it existed; `xtask` 313 passed, 0 failed here, 307 passed, 1 failed there; `guard_t072` `8 passed; 0 failed` here, `6 passed; 1 failed` there (the test is named `…within_180_s…` here, `…within_135_s…` there); scorecard line identical (`216 tests ran; 3 registered absent here`); image identical (`20261112.0539.3`); wall 28m21s here, 39m36s there; gate report 1088.12 s here, 1041.90 s there.
- gate-linux: 1200 passed in 140 suites here, 1195 in 139 there; gate green both; gate report 627.98 s vs 602.76 s; image `20261111.127.1` here, `20261111.370.1` there; `xtask` 318 vs 313; `acceptance_t072` 20 vs 17; `guard_t072` 8 vs 7; `guard_t074` 15 tests (same names) both; hygiene `64 of 76`, `710 tracked` vs `60 of 72`, `697`; no-skip `206 files` vs `205`; test-list 1,605 lines / 1,199 `: test` lines vs 1,597 / 1,194; wall 16m18s vs 15m27s.
- gate-windows: same failing steps (clippy, scorecard, test) and the same first lines: clippy `guard_t037_cli` due to 16 previous errors (both); scorecard `windows: 64 items; 158 tests ran; 3 registered absent here`, `60 scorecard problems` (both); test `0 passed, 0 failed, 0 ignored in 0 suites`, compile errors in `script_t055` (1) and `acceptance_t055` (2) (both); `test-output.txt` compile errors `window_t065` 8, `acceptance_t065` 3, `script_t055` 1, `acceptance_t055` 2 (both). Hygiene/no-skip counts as for Linux (`64 of 76`, `206 files`). Library `running 43 tests` (same), `guard_t073` 10 and `acceptance_t073` 1 (same), the 2 `MEASURED:` lines of `guard_t073` (same text); the 15 target statuses are identical line by line (11 targets `exit status 101`: 9 do not compile, `acceptance_t002` and `acceptance_t015_tools` compile and each fail one test; 4 targets `exit status 0`: `acceptance_t073`, `guard_t073`, `guard_t074`, `tools_contract`); `guard_t074` 15 tests all `ok`, same names, 176 `MEASURED:` lines under 15 test headers, **identical to early-2's 176 lines apart from the process id and temp-directory number in the fixture paths** (multiset comparison with those normalised; script `cmp.py`, 0 lines only in either); `acceptance_t002`: same one failing test `disk_save_replaces_via_rename_without_leftovers` with the same panic text at `acceptance_t002.rs:940:5`; `acceptance_t015_tools`: same failing test `no_tools_never_runs` at `acceptance_t015_tools.rs:131:5`; `xtask` 310 passed vs 305, `acceptance_t072` 20 vs 17, `guard_t072` 8 vs 7, unit tests 134 (both; 99.82 s vs 71.87 s); `main_xtask_is_rebuilt_from_base_on_every_run` absent on Windows (both); gate report 461.23 s vs 342.29 s; wall 26m17s vs 19m48s; image `20261103.298.1` (both).
- Not comparable from these files: the early-2 record's Linux per-test results for `guard_t074` were likewise absent.

## Not read
- `window-linux-44358453526` (577 B): listed, not downloaded; its `window-procedure.txt` lines not read.
- `window-probe (macos-15)`: no steps, no runner, no artefact, no log read; the reason for the cancellation is not shown by the job data (not determinable).
- Job logs beyond the image header, Rust versions, cache, `TEST LIST`/`TEST OUTPUT`, disk and summary lines (gate-linux, gate-windows, gate-macos logs were fetched); the logs of window-linux, window-probe (windows-2025).
- gate-linux and gate-macos: per-test results of `guard_t074` and of the `kiln-workspace` library (green gates upload no `test-output.txt`; `core-crates.txt` holds only `kiln-text`, `kiln-syntax`, `kiln-search`, `xtask`); the Linux gate's 1200 was not reconciled with the 1,199 names in `test-list.txt`, nor the macOS 1183 with 1,182.
- Windows: the contents of `test-list.txt` beyond its size (compile errors only); the compile errors of the 9 non-compiling `kiln-workspace` targets beyond the first error and closing line; the full text of clippy's, scorecard's and test's failing output beyond what the gate report keeps (60/62/27 lines); `xtask/platform-tests.toml` at 720f1d5 and the source of `main_xtask_is_rebuilt_from_base_on_every_run` (why it is absent on Windows is not determinable from this run's files).
- Which `kiln-workspace` library tests are new against main (no main run read); the seven `_windows`-named tests: no such names in the list.
- `Rust versions` lines of the gate-macos and gate-windows logs were not re-read; the macOS runner version line was read (2.338.2).
