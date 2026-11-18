# CI run 44327488962 (platforms.yml) on ci/candidate @ 9d6493c88b42934ae94db0eb772fe785cfcd18e6 — T-074 early look 2

Record by hand; method as `tasks/ci-runs/README.md`; same items as `T-074-early-1.md`. A throwaway merge of task T-074's second joined head with T-072's branch (title: "CI early look 2 (not for merge): T-074 second joined head with T-072's second joined head"). Every value comes from this run's job listing, artefacts or job logs. No judgement of results is made here.
**Complete: the run is `completed` (conclusion failure); every job has finished. `gate-macos` was still running at the first reads (15:15Z–15:26Z) and finished 15:28:40Z (read 15:32Z). Nothing was still running at the end.**

## Run header
- id 44327488962, run_number 31, attempt 1, event push, workflow `platforms`
- branch `ci/candidate`; commit `9d6493c88b42934ae94db0eb772fe785cfcd18e6` (equals the head asked for); created_at 2026-11-18T14:44:00Z
- Run status when the job listing was read (15:15Z): in_progress (only `gate-macos` unfinished). Final (read 15:32Z): `status` completed, run conclusion **failure** (cause by job: `gate-windows` failure, `gate-macos` failure, `window-probe (windows-2025)` failure, `window-probe (macos-15)` cancelled); `updated_at` 2026-11-18T15:28:40Z.

## Job table
Wall time = `completed_at` − `started_at` of the job (job listing).
| job | id | runner label | conclusion | wall | first failing step |
|---|---|---|---|---|---|
| gate-linux | 134882603773 | ubuntu-24.04 | success | 15m27s (14:44:02Z–14:59:29Z) | none |
| gate-windows | 134882602490 | windows-2025 | **failure** | 19m48s (14:44:02Z–15:03:50Z) | Gate (14:44:27Z–14:50:28Z, 6m01s); every later step success or skipped |
| gate-macos | 134882603063 | macos-15 | **failure** | 39m36s (14:49:04Z–15:28:40Z; job created 14:44:00Z, about 5 min before start, INFERRED) | Gate (14:49:36Z–15:07:08Z, 17m32s), failed on `test` only; every later step success or skipped |
| window-linux | 134882602869 | ubuntu-24.04 | success | 1m28s (14:44:03Z–14:45:31Z) | none |
| window-probe (windows-2025) | 134882603335 | windows-2025 | **failure** | 2m19s (14:44:02Z–14:46:21Z) | Window probe (14:46:14Z–14:46:16Z); Build kiln success |
| window-probe (macos-15) | 134882603112 | macos-15 | **cancelled** | 15m01s (14:44:00Z–14:59:01Z) | no steps listed; no `runner_id`, no `runner_name`: no runner assigned |

## Artefacts
`gate-linux-44327488962` 31,703 B (id 14439001531); `gate-windows-44327488962` 27,226 B (14438424358); `window-linux-44327488962` 577 B and `window-probe-windows-2025-44327488962` 344 B (listed, not downloaded). Fetched with `download_workflow_run_artifact` and curl; size equal, `testzip()` clean; scratch directory removed after the record. `gate-macos-44327488962` 54,386 B (id 14442425008, created 15:28:35Z; files `core-crates.txt` 40,764 B, `gate-report.json` 5,265 B, `test-list.txt` 77,074 B, `test-output.txt` 97,006 B). Listed at 15:32Z: 5 artefacts; no `window-probe-macos-15-…` artefact.
Files: gate-linux: `core-crates.txt` 40,283 B, `gate-report.json` 1,987 B, `test-list.txt` 78,032 B (no `test-output.txt`: gate green). gate-windows: `core-crates.txt` 40,575 B, `gate-report.json` 17,171 B, `test-list.txt` 10,832 B, `test-output.txt` 10,832 B, `windows-tests.txt` 95,467 B, `workspace-tests.txt` 8,004 B.
Job logs read (logs URL + curl): gate-linux, gate-windows, gate-macos.

## gate-windows (job 134882602490, windows-2025) — main subject
- Runner image (job log): `Image: windows-2025-vs2026`, `Version: 20261103.298.1`; runner 2.338.2. Rust versions listed by the cache step: `1.97.0 x86_64-pc-windows-msvc 2d8144b7880597b6e6d3dfd63a9a9efae3f533d3`, `1.99.1 x86_64-pc-windows-msvc 58cf8f9c35b355199727627d07ae34a4e2846d67`; `cargo 1.97.0 (c980f4866 2026-06-30)`; cache `No cache found.`
- Steps (job listing):
| step | conclusion | start–end (Z) | duration |
|---|---|---|---|
| Set up job | success | 14:44:03–14:44:04 | 1s |
| Checkout | success | 14:44:04–14:44:11 | 7s |
| Install the pinned toolchain | success | 14:44:11–14:44:23 | 12s |
| Cache cargo registry and target | success | 14:44:23–14:44:27 | 4s |
| Gate | **failure** | 14:44:27–14:50:28 | 6m01s |
| Gate report | success | 14:50:28 | 0s |
| Core crates (reported, not required) | success | 14:50:28–15:03:28 | 13m00s |
| Workspace lib and T-073 tests (reported, not required) | success | 15:03:28–15:03:33 | 5s |
| Test list | success | 15:03:33–15:03:34 | 1s |
| kiln-workspace test targets (reported, not required) | success | 15:03:34–15:03:41 | 7s |
| Full test output (gate failed) | success (**ran**) | 15:03:41–15:03:42 | 1s |
| Disk | success | 15:03:42–15:03:45 | 3s |
| Upload reports | success | 15:03:45–15:03:46 | 1s |
| Post Cache | skipped | 15:03:46 | 0s |
| Post Checkout, Complete job | success, success | 15:03:46–15:03:48 | 2s, 0s |
- Gate report (`gate-report.json`): `passed: false`, `failed_step: clippy`, 342.29 s.
| step | status | seconds | detail |
|---|---|---|---|
| fmt | pass | 1.78 | formatted |
| clippy | **fail** | 54.96 | kept text 60 lines, ends `error: could not compile `kiln-cli` (test "guard_t037_cli") due to 16 previous errors` (errors such as `E0599 no method named `gid`/`dev` found for struct `std::fs::Metadata`` at `apps\kiln-cli\tests\guard_t037_cli.rs`) |
| layers | pass | 9.00 | 11 crates, 14 internal edges, layer table holds |
| ui-commands | pass | 0.04 | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | pass | 0.14 | reviews: 60 of 72 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 697 tracked file(s) clean |
| schema | pass | 26.54 | 30 commands; audit ok; docs/commands.json current |
| scorecard | **fail** | 245.04 | first line `windows: 64 items; 158 tests ran; 3 registered absent here; page not compared (computed on Linux)`, then `60 scorecard problems:` (first: `building test executables failed on windows (); their tests are not run`) |
| test | **fail** | 0.91 | `0 passed, 0 failed, 0 ignored in 0 suites`; kept text 27 lines, ends `error: could not compile `kiln` (test "script_t055") due to 1 previous error`, `error: could not compile `kiln` (test "acceptance_t055") due to 2 previous errors` |
| no-skip | pass | 3.88 | self-test ok (17 known findings); 205 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (10 pending), today = 2026-11-18 |
| fuzz, budgets | empty | 0.0 | no targets registered yet |
- "Test list": 1 s; log `TEST LIST: exit status 101`; `test-list.txt` (10,832 B) has no binary and no test name (compile errors only). "Full test output": ran (1 s); `test-output.txt` 10,832 B, 158 non-empty lines, 0 lines `test … FAILED`; its `could not compile` lines: `window_t065` (8 errors), `acceptance_t065` (3), `script_t055` (1), `acceptance_t055` (2).
- Disk (job log): `D:              220G  3.8G  217G   2% /d`; `3.6G	target`.

### Step "Workspace lib and T-073 tests (reported, not required)" (`workspace-tests.txt`)
Binaries: library `running 43 tests`; `acceptance_t073` `running 1 test`; `guard_t073` `running 10 tests`. 54 tests, 0 failed. No `guard_t074`/`acceptance_t002` binary in this step. Every library test line, verbatim:
```
running 43 tests
test disk::tests::errors_are_root_independent ... ok
test disk::tests::failed_rename_removes_own_temp_and_keeps_target ... ok
test disk::tests::held_root_check_is_inside_only_for_a_read_identity_equal_to_the_held_one ... ok
test disk::tests::check_location_inside_on_every_platform ... ok
test disk::tests::check_location_outside_on_every_platform ... ok
test disk::tests::longest_file_names_can_be_saved ... ok
test disk::tests::new_rejects_missing_root_and_file_root_without_naming_them ... ok
test disk::tests::link_at_temp_name_is_not_followed_on_every_platform ... ok
test disk::tests::root_given_through_a_link_on_every_platform ... ok
test disk::tests::all_temp_names_taken_fails_without_changes_then_recovers ... ok
test disk::tests::root_removed_on_every_platform ... ok
test disk::tests::save_through_a_link_on_every_platform ... ok
test document::tests::dirty_follows_content ... ok
test document::tests::disk_save_onto_path_replaced_by_directory_fails_and_changes_nothing ... ok
test disk::tests::root_replaced_by_a_link_on_every_platform ... ok
test document::tests::edits_keep_encoding_and_inserted_newline_stays_lf ... ok
test document::tests::failed_save_keeps_file_and_dirty_state ... ok
test document::tests::open_errors ... ok
test document::tests::works_through_trait_objects ... ok
test document::tests::unedited_round_trip_is_byte_exact_for_each_encoding ... ok
test encoding::tests::decode_error_display ... ok
test encoding::tests::detects_boms_and_strips_exactly_one ... ok
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
test encoding::tests::encode_decode_inverse_on_generated_strings ... ok
test project::tests::negation_and_deeper_files ... ok
test project::tests::under_uses_prefix_gitignore_and_is_not_itself_subject ... ok
test project::tests::worked_example ... ok
test disk::tests::write_read_and_errors ... ok
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

test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.79s
```
Result lines, verbatim:
```
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.79s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
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
Other non-zero targets (summary from the file: first error, closing line):
```
==== acceptance_t002_r2: exit status 101 | error[E0433]: cannot find `unix` in `os` | error: could not compile `kiln-workspace` (test "acceptance_t002_r2") due to 11 previous errors | no test result line
==== acceptance_t008: exit status 101 | error[E0433]: cannot find `unix` in `os` | error: could not compile `kiln-workspace` (test "acceptance_t008") due to 20 previous errors | no test result line
==== acceptance_t015_tools: exit status 101 | - | - | test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s | test no_tools_never_runs ... FAILED
==== acceptance_t019_workspace: exit status 101 | error[E0433]: cannot find `unix` in `os` | error: could not compile `kiln-workspace` (test "acceptance_t019_workspace") due to 5 previous errors | no test result line
==== acceptance_t036_workspace: exit status 101 | error[E0433]: cannot find `unix` in `os` | error: could not compile `kiln-workspace` (test "acceptance_t036_workspace") due to 16 previous errors | no test result line
==== confinement: exit status 101 | error[E0433]: cannot find `unix` in `os` | error: could not compile `kiln-workspace` (test "confinement") due to 24 previous errors | no test result line
==== guard_t008: exit status 101 | error[E0433]: cannot find `unix` in `os` | error: could not compile `kiln-workspace` (test "guard_t008") due to 1 previous error | no test result line
==== guard_t036_removal_workspace: exit status 101 | error[E0433]: cannot find `unix` in `os` | error: could not compile `kiln-workspace` (test "guard_t036_removal_workspace") due to 3 previous errors | no test result line
==== guard_t036_workspace: exit status 101 | error[E0433]: cannot find `unix` in `os` | error: could not compile `kiln-workspace` (test "guard_t036_workspace") due to 1 previous error | no test result line
==== project: exit status 101 | error[E0433]: cannot find `unix` in `os` | error: could not compile `kiln-workspace` (test "project") due to 3 previous errors | no test result line
```

#### `guard_t074` (exit status 0): the 15 test lines and the result line, verbatim
```
running 15 tests
test awa1_file_link_absolute_on_the_walk ... ok
test awa1_file_link_relative_on_the_walk ... ok
test awa3_temp_name_held_by_a_dangling_file_link_to_outside ... ok
test awa3_temp_name_held_by_a_dangling_junction ... ok
test awa3_temp_name_held_by_a_file_link_to_an_outside_file ... ok
test awa3_temp_name_held_by_a_junction ... ok
test awa1_junction_on_the_walk ... ok
test awa1_directory_link_on_the_walk ... ok
test awa3_temp_names_held_by_a_junction_and_a_dangling_file_link ... ok
test awa4_dotdot_after_a_junction_in_the_root_text ... ok
test awa4_root_removed_while_held ... ok
test awa4_root_given_through_a_directory_link_and_retargeted ... ok
test awa4_root_given_through_a_junction_and_retargeted ... ok
test awa4_root_replaced_by_a_junction ... ok
test awa1_link_below_the_first_level ... ok
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.43s
```
No test failed, so there is no failing stdout and no panic. `grep -c MEASURED:` over `windows-tests.txt` gives 178: 176 inside `guard_t074` (command: the `measured` extraction of a scratch script, which printed `measured 176`) and 2 inside `guard_t073`. Every `MEASURED:` line of `guard_t074`, in order, under the `---- <test> stdout ----` block it belongs to (the file's order of blocks; tests with no such line are not listed):
```
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
---- awa3_temp_name_held_by_a_dangling_file_link_to_outside stdout ----
MEASURED: awa3_temp_name_held_by_a_dangling_file_link_to_outside: windows: fixture: FileAbs made at .kiln-tmp-0 -> D:\a\photocraft-study-review\photocraft-study-review\target\tmp\guard_t074\awa3_temp_name_held_by_a_dangling_file_link_to_outside-6816\out\missing (target exists: false)
MEASURED: awa3_temp_name_held_by_a_dangling_file_link_to_outside: windows: write_atomic(f.txt) beside the links: Ok(())
MEASURED: awa3_temp_name_held_by_a_dangling_file_link_to_outside: windows: Document save of f.txt beside the links: Ok(())
---- awa3_temp_name_held_by_a_dangling_junction stdout ----
MEASURED: awa3_temp_name_held_by_a_dangling_junction: windows: fixture: Junction made at .kiln-tmp-0 -> D:\a\photocraft-study-review\photocraft-study-review\target\tmp\guard_t074\awa3_temp_name_held_by_a_dangling_junction-6816\out\gone (target exists: false)
MEASURED: awa3_temp_name_held_by_a_dangling_junction: windows: write_atomic(f.txt) beside the links: Ok(())
MEASURED: awa3_temp_name_held_by_a_dangling_junction: windows: Document save of f.txt beside the links: Ok(())
---- awa3_temp_name_held_by_a_file_link_to_an_outside_file stdout ----
MEASURED: awa3_temp_name_held_by_a_file_link_to_an_outside_file: windows: fixture: FileAbs made at .kiln-tmp-0 -> D:\a\photocraft-study-review\photocraft-study-review\target\tmp\guard_t074\awa3_temp_name_held_by_a_file_link_to_an_outside_file-6816\out\t.txt (target exists: true)
MEASURED: awa3_temp_name_held_by_a_file_link_to_an_outside_file: windows: write_atomic(f.txt) beside the links: Ok(())
MEASURED: awa3_temp_name_held_by_a_file_link_to_an_outside_file: windows: Document save of f.txt beside the links: Ok(())
---- awa3_temp_name_held_by_a_junction stdout ----
MEASURED: awa3_temp_name_held_by_a_junction: windows: fixture: Junction made at .kiln-tmp-0 -> D:\a\photocraft-study-review\photocraft-study-review\target\tmp\guard_t074\awa3_temp_name_held_by_a_junction-6816\out (target exists: true)
MEASURED: awa3_temp_name_held_by_a_junction: windows: write_atomic(f.txt) beside the links: Ok(())
MEASURED: awa3_temp_name_held_by_a_junction: windows: Document save of f.txt beside the links: Ok(())
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
---- awa3_temp_names_held_by_a_junction_and_a_dangling_file_link stdout ----
MEASURED: awa3_temp_names_held_by_a_junction_and_a_dangling_file_link: windows: fixture: Junction made at .kiln-tmp-0 -> D:\a\photocraft-study-review\photocraft-study-review\target\tmp\guard_t074\awa3_temp_names_held_by_a_junction_and_a_dangling_file_link-6816\out (target exists: true)
MEASURED: awa3_temp_names_held_by_a_junction_and_a_dangling_file_link: windows: fixture: FileAbs made at .kiln-tmp-1 -> D:\a\photocraft-study-review\photocraft-study-review\target\tmp\guard_t074\awa3_temp_names_held_by_a_junction_and_a_dangling_file_link-6816\out\t (target exists: false)
MEASURED: awa3_temp_names_held_by_a_junction_and_a_dangling_file_link: windows: write_atomic(f.txt) beside the links: Ok(())
MEASURED: awa3_temp_names_held_by_a_junction_and_a_dangling_file_link: windows: Document save of f.txt beside the links: Ok(())
---- awa4_dotdot_after_a_junction_in_the_root_text stdout ----
MEASURED: awa4_dotdot_after_a_junction_in_the_root_text: --root k/link/../b: windows: read f.txt: Ok
---- awa4_root_removed_while_held stdout ----
MEASURED: awa4_root_removed_while_held: windows: remove_dir of the held empty root: Ok(())
MEASURED: awa4_root_removed_while_held: empty root removed: windows: read OutsideRoot, write_atomic OutsideRoot, is_file false, list_dir OutsideRoot, check_location OutsideRoot, save -
MEASURED: awa4_root_removed_while_held: empty root removed, a new directory at its name: windows: read OutsideRoot, write_atomic OutsideRoot, is_file false, list_dir OutsideRoot, check_location OutsideRoot, save -
MEASURED: awa4_root_removed_while_held: windows: remove_dir_all of the held root with a file: Ok(())
MEASURED: awa4_root_removed_while_held: root with a file removed: windows: read OutsideRoot, write_atomic OutsideRoot, is_file false, list_dir OutsideRoot, check_location OutsideRoot, save OutsideRoot
MEASURED: awa4_root_removed_while_held: root removed, a junction to a new directory at its name: windows: read OutsideRoot, write_atomic OutsideRoot, is_file false, list_dir OutsideRoot, check_location OutsideRoot, save OutsideRoot
---- awa4_root_given_through_a_directory_link_and_retargeted stdout ----
MEASURED: awa4_root_given_through_a_directory_link_and_retargeted: windows: DirSym as --root: read f.txt: Ok([114, 101, 97, 108, 10])
MEASURED: awa4_root_given_through_a_directory_link_and_retargeted: DirSym retargeted to another directory: windows: read OutsideRoot, write_atomic OutsideRoot, is_file false, list_dir OutsideRoot, check_location OutsideRoot, save OutsideRoot
MEASURED: awa4_root_given_through_a_directory_link_and_retargeted: DirSym pointed back: windows: read f.txt: Ok
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
test failed_save_leaves_file_intact_and_document_dirty ... ok
test line_endings_round_trip_verbatim ... ok
test relpath_accepts_and_rejects_exact_cases ... ok
test save_through_symlink_updates_target ... ok
test encode_decode_round_trip ... ok
test unedited_utf8_files_round_trip_byte_for_byte ... ok
test utf16_le_and_be_with_bom_open_and_save ... ok
test vfs_errors_name_relative_paths_only ... ok
test utf8_bom_detected_stripped_and_restored_on_save ... ok
test disk_vfs_results_do_not_depend_on_root ... ok
test memory_and_disk_vfs_agree_on_random_operation_sequences ... ok

successes:

successes:
    bom_only_file_round_trips
    dirty_follows_text_versus_last_saved
    disk_vfs_results_do_not_depend_on_root
    encode_decode_round_trip
    failed_save_leaves_file_intact_and_document_dirty
    invalid_bytes_are_reported_with_offset_never_replaced
    line_endings_round_trip_verbatim
    memory_and_disk_vfs_agree_on_random_operation_sequences
    relpath_accepts_and_rejects_exact_cases
    save_through_symlink_updates_target
    unedited_utf8_files_round_trip_byte_for_byte
    utf16_le_and_be_with_bom_open_and_save
    utf8_bom_detected_stripped_and_restored_on_save
    vfs_errors_name_relative_paths_only

failures:

---- disk_save_replaces_via_rename_without_leftovers stdout ----

thread 'disk_save_replaces_via_rename_without_leftovers' (4864) panicked at crates\kiln-workspace\tests\acceptance_t002.rs:940:5:
precondition: permission bits and inode numbers are Unix metadata; the Windows checks of atomic replace are not written yet (ADR-0009: T-073 and T-074 write the Windows behaviour and its tests)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    disk_save_replaces_via_rename_without_leftovers

test result: FAILED. 14 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
```

#### `acceptance_t015_tools` (exit status 101), failing test's block, verbatim
```
running 3 tests
test no_tools_never_runs ... FAILED
test tool_error_texts_are_exact ... ok
test workspace_reexports_resolve ... ok

successes:

successes:
    tool_error_texts_are_exact
    workspace_reexports_resolve

failures:

---- no_tools_never_runs stdout ----

thread 'no_tools_never_runs' (4604) panicked at crates\kiln-workspace\tests\acceptance_t015_tools.rs:131:5:
precondition: the fake `cargo`/`rustfmt` of this test are shell scripts and need Unix
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    no_tools_never_runs

test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## `xtask` tests (`core-crates.txt`, block `==== cargo test --locked --no-fail-fast -p xtask`)
Commands: ANSI-stripped file; `awk '/-p xtask$/{x=1}x&&/Running/{r=$0}x&&/^test result/{print r" => "$0}'` and the same with `p+=$4;f+=$6`.
| platform | `==== xtask: exit status` | binaries | whole package | unit-test binary | `acceptance_t074` | `acceptance_t072` | `guard_t072` | failing tests |
|---|---|---|---|---|---|---|---|---|
| Linux | 0 | 26 | 313 passed, 0 failed | 138 passed, 4.11 s | `2 passed; 0 failed`, 0.00 s | `17 passed; 0 failed`, 0.42 s | `7 passed; 0 failed`, 125.22 s | none |
| Windows | 0 | 26 | 305 passed, 0 failed | 134 passed, 71.87 s | `2 passed; 0 failed`, 0.00 s | `17 passed; 0 failed`, 1.38 s | `7 passed; 0 failed`, 127.41 s | none |
| macOS | 101 | 26 | 307 passed, 1 failed | 136 passed, 24.20 s | `2 passed; 0 failed`, 0.01 s | `17 passed; 0 failed`, 1.15 s | **`6 passed; 1 failed`**, 136.59 s | `g1_probe_with_a_kiln_that_never_exits_ends_within_135_s_and_records_the_kill` (`guard_t072`), see the gate-macos section |
`kiln-text`, `kiln-syntax`, `kiln-search` end `exit status 0` on all three platforms. No line `test … FAILED` in the Linux and Windows `core-crates.txt`; one in the macOS one (below).

## gate-linux (job 134882603773, ubuntu-24.04)
- Runner image (job log): `Image: ubuntu-24.04`, `Version: 20261111.370.1`; runner 2.338.2. (Not 20261116.376.1.) Rust versions (cache step): `1.97.0 x86_64-unknown-linux-gnu 2d8144b7880597b6e6d3dfd63a9a9efae3f533d3`, `1.99.1 x86_64-unknown-linux-gnu 58cf8f9c35b355199727627d07ae34a4e2846d67`; `cargo 1.97.0 (c980f4866 2026-06-30)`; cache `Cache hit for: v0-rust-gate-linux-Linux-x64-f37a6dc5-6187dc6e`, `full match: true`.
- Steps: Set up job 1s; Checkout 1s; toolchain 8s; Cache 5s; Gate success 14:44:17–14:54:23 (10m06s); Gate report 0s; Core crates success 14:54:23–14:59:25 (5m02s); Test list success 1s; Full test output (gate failed) **skipped**; Disk 0s; Upload reports 1s. Disk (log): `/dev/root       145G   65G   80G  45% /`; `4.5G	target`. `TEST LIST: exit status 0`.
- Gate report: `passed: true`, `failed_step: null`, 602.76 s. fmt pass 0.72; clippy pass 9.40 (no warnings); layers pass 0.59; ui-commands pass; hygiene pass (60 of 72 review file(s) checked …; 697 tracked file(s) clean); schema pass 4.24; scorecard pass 142.14 (`64 items: 47 verified, 0 implemented, 0 failing, 17 planned.`); **test pass 443.66: `1195 passed, 0 failed, 0 ignored in 139 suites`**; no-skip pass 1.94 (`205 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (10 pending)`); fuzz, budgets empty.
- Failing tests: none (gate green; `core-crates.txt` has no `FAILED` line).
- `queue::tests::main_xtask_is_rebuilt_from_base_on_every_run` on Linux: **passed**, verbatim `test queue::tests::main_xtask_is_rebuilt_from_base_on_every_run ... ok` (`core-crates.txt` line 409), image 20261111.370.1. On Windows: **absent** (no line with that name; the unit-test binary ran, 134 passed; 18 other `queue::tests::…` lines present).
- `acceptance_t074` (xtask): `2 passed; 0 failed` (above).
- `guard_t074` (kiln-workspace): not in `core-crates.txt`; no `test-output.txt` (green gate). `test-list.txt` (78,032 B, 1,597 lines; 138 binary lines = 130 `Running` + 8 `Doc-tests`; 1,194 lines ending `: test`) lists the binary `Running tests/guard_t074.rs` with `15 tests, 0 benchmarks`; names listed, verbatim: `awa1_directory_link_on_the_walk`, `awa1_file_link_absolute_on_the_walk`, `awa1_file_link_relative_on_the_walk`, `awa1_junction_on_the_walk`, `awa1_link_below_the_first_level`, `awa3_temp_name_held_by_a_dangling_file_link_to_outside`, `awa3_temp_name_held_by_a_dangling_junction`, `awa3_temp_name_held_by_a_file_link_to_an_outside_file`, `awa3_temp_name_held_by_a_junction`, `awa3_temp_names_held_by_a_junction_and_a_dangling_file_link`, `awa4_dotdot_after_a_junction_in_the_root_text`, `awa4_root_given_through_a_directory_link_and_retargeted`, `awa4_root_given_through_a_junction_and_retargeted`, `awa4_root_removed_while_held`, `awa4_root_replaced_by_a_junction`. Per-test results are not in the artefact (the gate's `test` step passed: 0 failed).

## window jobs
- `window-linux` success 1m28s. `window-probe (windows-2025)` failure 2m19s (Window probe failed, Build kiln success; artefact 344 B not downloaded, probe exit status line not read). `window-probe (macos-15)` cancelled after 15m01s, job data with no steps, no runner id or name: no runner assigned; no artefact.

## gate-macos (job 134882603063, runner label macos-15)
- Runner image (job log): `Image: macos-15-arm64`, `Version: 20261112.0539.3`; runner version not re-read. Rust versions listed by the cache step: `1.97.0 aarch64-apple-darwin 2d8144b7880597b6e6d3dfd63a9a9efae3f533d3`, `1.99.1 aarch64-apple-darwin 58cf8f9c35b355199727627d07ae34a4e2846d67`; `cargo 1.97.0 (c980f4866 2026-06-30)`; cache `Cache hit for: v0-rust-gate-macos-Darwin-arm64-03a9c7df-6187dc6e`, `full match: true`.
- Completeness: job `status: completed`, `conclusion: failure`; all steps completed (none in progress or pending); artefact present.
| step | conclusion | start | end | duration |
|---|---|---|---|---|
| Set up job | success | 14:49:04Z | 14:49:05Z | 1s |
| Checkout | success | 14:49:06Z | 14:49:08Z | 2s |
| Install the pinned toolchain (rust-toolchain.toml) | success | 14:49:08Z | 14:49:19Z | 11s |
| Cache cargo registry and target | success | 14:49:19Z | 14:49:36Z | 17s |
| Gate | **failure** | 14:49:36Z | 15:07:08Z | 17m32s |
| Gate report | success | 15:07:08Z | 15:07:08Z | 0s |
| Core crates (reported, not required) | success | 15:07:08Z | 15:16:04Z | 8m56s |
| Test list | success | 15:16:04Z | 15:16:08Z | 4s |
| Full test output (gate failed) | success (**ran**) | 15:16:08Z | 15:28:33Z | 12m25s |
| Disk | success | 15:28:33Z | 15:28:34Z | 1s |
| Upload reports | success | 15:28:34Z | 15:28:36Z | 2s |
| Post Cache cargo registry and target | skipped | 15:28:36Z | 15:28:36Z | 0s |
| Post Checkout | success | 15:28:36Z | 15:28:36Z | 0s |
| Complete job | success | 15:28:36Z | 15:28:39Z | 3s |
- Log lines, verbatim: `TEST LIST: exit status 0` and `TEST OUTPUT: exit status 0` (each followed by `…: the status is recorded, not used; the gate step decides the colour`). Disk (log): `/dev/disk2s5   320Gi   262Gi    40Gi    87%    2.3M  419M    1%   /System/Volumes/Data`; `3.2G	target`.
- Gate report (`gate-report.json`): `passed: false`, `failed_step: test`, 1041.90 s.
| step | status | seconds | detail |
|---|---|---|---|
| fmt | pass | 1.14 | formatted |
| clippy | pass | 18.37 | no warnings (all targets, -D warnings) |
| layers | pass | 1.44 | 11 crates, 14 internal edges, layer table holds |
| ui-commands | pass | 0.04 | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | pass | 0.24 | reviews: 60 of 72 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 697 tracked file(s) clean (0 not UTF-8, skipped) |
| schema | pass | 6.37 | 30 commands; audit ok; docs/commands.json current |
| scorecard | pass | 248.21 | macos: 64 items; 216 tests ran; 3 registered absent here; page not compared (computed on Linux) |
| test | **fail** | 762.97 | `1171 passed, 1 failed, 0 ignored in 130 suites`; kept text (28 lines): `test g1_probe_with_a_kiln_that_never_exits_ends_within_135_s_and_records_the_kill ... FAILED`, `---- g1_probe_with_a_kiln_that_never_exits_ends_within_135_s_and_records_the_kill stdout ----`, the `Running` lines of the xtask test binaries, ends `Running tests/guard_t072.rs (target/debug/deps/guard_t072-b4550740a07877cb)` / `error: test failed, to rerun pass `-p xtask --test guard_t072``; the kept text has no panic message |
| no-skip | pass | 3.11 | self-test ok (17 known findings); 205 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (10 pending), today = 2026-11-18 |
| fuzz, budgets | empty | 0.0 | no targets registered yet |

### Failing test on macOS
The gate's `test` step failed on `g1_probe_with_a_kiln_that_never_exits_ends_within_135_s_and_records_the_kill` (binary `guard_t072`, xtask). The same test in the later "Core crates" step (`core-crates.txt`, lines 760-775), verbatim:
```
test g1_probe_with_a_kiln_that_never_exits_ends_within_135_s_and_records_the_kill has been running for over 60 seconds
test g1_probe_with_a_kiln_that_never_exits_ends_within_135_s_and_records_the_kill ... FAILED

failures:

---- g1_probe_with_a_kiln_that_never_exits_ends_within_135_s_and_records_the_kill stdout ----

thread 'g1_probe_with_a_kiln_that_never_exits_ends_within_135_s_and_records_the_kill' (476938) panicked at xtask/tests/guard_t072.rs:994:5:
the probe step took 136.6 s with a kiln that never exits (bound 120 s + 15 s)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    g1_probe_with_a_kiln_that_never_exits_ends_within_135_s_and_records_the_kill

test result: FAILED. 6 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 136.59s
```
In the "Full test output (gate failed)" step (`test-output.txt`, whole workspace, `cargo test --no-fail-fast`) the same test, verbatim: `test g1_probe_with_a_kiln_that_never_exits_ends_within_135_s_and_records_the_kill ... ok`, and `guard_t072`: `test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 135.05s`. `test-output.txt` has no line `test … FAILED` (`grep -c 'FAILED'` gives 0); the sum of its 139 `test result:` lines is 1178 passed, 0 failed (command: `grep -E '^test result' | awk '{p+=$4;f+=$6;n++}'`). So of the three runs of the test on macOS in this job (gate, Core crates, Full test output) two failed and one passed. No other failing test.

### macOS `xtask` and named tests (`core-crates.txt`, `-p xtask`)
`==== xtask: exit status 101`; kiln-text 78, kiln-syntax 53, kiln-search 55 passed (0 failed, exit status 0); xtask 307 passed, 1 failed over 26 `test result:` lines (command as above). `acceptance_t074` `2 passed; 0 failed`; `acceptance_t072` `17 passed; 0 failed`; `guard_t072` `6 passed; 1 failed` (above). `queue::tests::main_xtask_is_rebuilt_from_base_on_every_run` on macOS: **passed**, verbatim `test queue::tests::main_xtask_is_rebuilt_from_base_on_every_run ... ok` (`core-crates.txt` line 411; `test-output.txt` line 1601), image 20261112.0539.3.

### macOS `guard_t074` (from `test-output.txt`, binary `Running tests/guard_t074.rs`; the step runs without `--show-output`, so there are no `MEASURED:` lines on macOS: `grep -c MEASURED` gives 0 in every macOS file)
15 test lines and result line, verbatim:
```
running 15 tests
test awa1_file_link_absolute_on_the_walk ... ok
test awa1_directory_link_on_the_walk ... ok
test awa1_file_link_relative_on_the_walk ... ok
test awa3_temp_name_held_by_a_dangling_file_link_to_outside ... ok
test awa3_temp_name_held_by_a_dangling_junction ... ok
test awa3_temp_name_held_by_a_file_link_to_an_outside_file ... ok
test awa3_temp_name_held_by_a_junction ... ok
test awa1_junction_on_the_walk ... ok
test awa3_temp_names_held_by_a_junction_and_a_dangling_file_link ... ok
test awa4_dotdot_after_a_junction_in_the_root_text ... ok
test awa4_root_given_through_a_directory_link_and_retargeted ... ok
test awa4_root_removed_while_held ... ok
test awa4_root_given_through_a_junction_and_retargeted ... ok
test awa4_root_replaced_by_a_junction ... ok
test awa1_link_below_the_first_level ... ok
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
```
- `test-list.txt` (77,074 B, 1,580 lines; 130 `Running` + 8 `Doc-tests`; 1,177 lines ending `: test`) lists the same 15 `guard_t074` names. `TEST LIST: exit status 0`.

## Not read
- `window-linux-44327488962` (577 B) and `window-probe-windows-2025-44327488962` (344 B): listed, not downloaded (the Windows probe's `PROBE:` exit status line is not read).
- `window-probe (macos-15)`: no steps, no runner, no artefact, no log read; the cancellation reason is not shown by the job data (not determinable).
- Job logs beyond the image header, Rust versions, cache, `TEST LIST`/`TEST OUTPUT` and disk lines; the runner version of the macOS job; full text of clippy's, scorecard's and test's failing output on Windows beyond what the gate report keeps (60/62/27 lines).
- Linux: per-test results of `guard_t074` (green gate, no `test-output.txt`): not determinable beyond the test list and the passing gate. The Linux gate's `test` count (1195 passed) was not reconciled with `test-list.txt` names (1,194).
- Windows: the contents of `test-list.txt` beyond its size (compile errors only, 10,832 B); the compile errors of the 9 non-compiling `kiln-workspace` targets beyond the first error and closing line; `xtask/platform-tests.toml` at 9d6493c and the source of `main_xtask_is_rebuilt_from_base_on_every_run` (so why it is absent on Windows is not determinable from this run's files).
- macOS: the reason the probe step took 136.6 s in two runs and 135.05 s in the third is not shown by any file read; the source of `guard_t072.rs` was not read.
- No comparison with `T-074-early-1.md` beyond the new target `guard_t074` having 15 tests here (13 there) was made.
