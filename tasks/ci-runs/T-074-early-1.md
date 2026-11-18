# CI run 44307677646 (platforms.yml) on ci/candidate @ cec40579598ee6a2e033d0ccb434c7e2373e2217 — T-074 early look 1

Record by hand; method as `tasks/ci-runs/README.md`. **DRAFT: written while the run was in progress (first check about 13:05Z); to be extended.**

## Run header
- id 44307677646, run_number 29, attempt 1, event push, title "CI early look (not for merge): T-074 joined head with T-072's joined …"
- branch `ci/candidate`; commit `cec40579598ee6a2e033d0ccb434c7e2373e2217`; created_at 2026-11-18T12:49:57Z
- first check (updated_at 13:04:59Z): status in_progress.

## Job table (first check, about 13:05Z)
| job | id | conclusion | note |
|---|---|---|---|
| gate-linux | 134813362754 | in progress | started 12:50:00Z; Gate failure (12:50:18Z-13:05:24Z); Core crates running |
| gate-macos | 134813362960 | in progress | started 12:55:01Z; Gate running |
| gate-windows | 134813362421 | **failure** | 12:49:59Z-13:16:38Z = 26m39s; first failing step Gate (12:50:28Z-12:58:47Z, 8m19s); every later step success/skipped |
| window-linux | 134813362613 | success | 12:50:00Z-12:51:40Z = 1m40s |
| window-probe (macos-15) | 134813362978 | cancelled | 12:49:58Z-13:04:59Z = 15m01s; the job listing shows no steps |
| window-probe (windows-2025) | 134813362953 | failure | 12:50:00Z-12:52:15Z = 2m15s; failing step Window probe (12:52:09Z-12:52:11Z) |

## gate-windows (job 134813362421, windows-2025) — main subject
Artefact `gate-windows-44307677646` (id 14430946111, 26,881 B listed; fetched by `download_workflow_run_artifact` + curl, size equal, `testzip()` clean) was uploaded at 13:16:34Z, before the job finished; files: `core-crates.txt` 42,334 B; `gate-report.json` 16,412 B; `test-list.txt` 10,832 B; `test-output.txt` 10,832 B; `windows-tests.txt` 79,568 B; `workspace-tests.txt` 8,004 B.
Steps (job listing): Set up job success 12:50:00Z-12:50:01Z (1s); Checkout success 8s; Install the pinned toolchain success 14s; Cache cargo registry and target success 5s; Gate **failure** 12:50:28Z-12:58:47Z (8m19s); Gate report success 0s; Core crates (reported, not required) success 12:58:47Z-13:16:11Z (17m24s); Workspace lib and T-073 tests success 13:16:11Z-13:16:19Z (8s); Test list success 13:16:19Z-13:16:20Z (1s); kiln-workspace test targets success 13:16:20Z-13:16:29Z (9s); Full test output (gate failed) success 13:16:29Z-13:16:30Z (1s); Disk success 13:16:30Z-13:16:34Z (4s); Upload reports success 13:16:34Z-13:16:35Z (1s); Post Cache cargo registry and target skipped; Post Checkout success 2s; Complete job success 0s. Job completed `failure` at 13:16:38Z (wall 26m39s from started_at 12:49:59Z).

### Gate report (`gate-report.json`): `passed: false`, `failed_step: clippy`, 475.38 s
| step | status | seconds | detail (first line / counts) |
|---|---|---|---|
| fmt | PASS | 2.3 | formatted |
| clippy | FAIL | 71.9 | kept text is cargo `Checking`/`Compiling` lines (2,105 chars; first line `Checking ecolor v0.37.0`) |
| layers | PASS | 12.2 | 11 crates, 14 internal edges, layer table holds |
| ui-commands | PASS | 0.05 | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | PASS | 0.17 | reviews: 59 of 71 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 696 tracked file(s) clean (0 not UTF-8, skipped) |
| schema | PASS | 35.9 | 30 commands; audit ok; docs/commands.json current |
| scorecard | FAIL | 345.5 | windows: 64 items; 158 tests ran; 3 registered absent here; page not compared (computed on Linux) / 60 scorecard problems: / - building test executables failed on windows (); their tests are not run |
| test | FAIL | 1.5 | 0 passed, 0 failed, 0 ignored in 0 suites (then compile errors, `E0277` `[OsString]` in `apps\kiln\tests\window_t065.rs`) |
| no-skip | PASS | 5.8 | self-test ok (17 known findings); 205 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (10 pending), today = 2026-11-18 |
| fuzz | EMPTY | 0.0001 | no targets registered yet (xtask/fuzz.toml) |
| budgets | EMPTY | 0.0001 | no targets registered yet (xtask/budgets.toml) |

### 2a. "Workspace lib and T-073 tests" (`workspace-tests.txt`, ANSI stripped)
Binaries and result lines, verbatim:
```
     Running unittests src\lib.rs (target\debug\deps\kiln_workspace-ecdfe3d69712069b.exe)
running 43 tests
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.18s
     Running tests\acceptance_t073.rs (target\debug\deps\acceptance_t073-8207a2e07d5a1f0c.exe)
running 1 test
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests\guard_t073.rs (target\debug\deps\guard_t073-ebaebcdeb0c0aed4.exe)
running 10 tests
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
```
Total 54 passed, 0 failed. The step has no `guard_t074` or `acceptance_t002` binary. Every test line of the library binary, verbatim (all 43, none FAILED, so no failing stdout/panic exists):
```
test disk::tests::errors_are_root_independent ... ok
test disk::tests::failed_rename_removes_own_temp_and_keeps_target ... ok
test disk::tests::held_root_check_is_inside_only_for_a_read_identity_equal_to_the_held_one ... ok
test disk::tests::check_location_inside_on_every_platform ... ok
test disk::tests::check_location_outside_on_every_platform ... ok
test disk::tests::longest_file_names_can_be_saved ... ok
test disk::tests::new_rejects_missing_root_and_file_root_without_naming_them ... ok
test disk::tests::all_temp_names_taken_fails_without_changes_then_recovers ... ok
test disk::tests::link_at_temp_name_is_not_followed_on_every_platform ... ok
test disk::tests::root_removed_on_every_platform ... ok
test disk::tests::root_given_through_a_link_on_every_platform ... ok
test disk::tests::root_replaced_by_a_link_on_every_platform ... ok
test document::tests::dirty_follows_content ... ok
test disk::tests::save_through_a_link_on_every_platform ... ok
test document::tests::edits_keep_encoding_and_inserted_newline_stays_lf ... ok
test document::tests::failed_save_keeps_file_and_dirty_state ... ok
test document::tests::open_errors ... ok
test document::tests::unedited_round_trip_is_byte_exact_for_each_encoding ... ok
test document::tests::disk_save_onto_path_replaced_by_directory_fails_and_changes_nothing ... ok
test document::tests::works_through_trait_objects ... ok
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
test encoding::tests::encode_decode_inverse_on_generated_strings ... ok
test project::tests::bom_and_invalid_lines ... ok
test project::tests::limit_edges ... ok
test project::tests::negation_and_deeper_files ... ok
test project::tests::under_uses_prefix_gitignore_and_is_not_itself_subject ... ok
test project::tests::worked_example ... ok
test disk::tests::write_read_and_errors ... ok
test disk::tests::matches_memory_model_on_random_operations ... ok
```
The two `MEASURED:` lines of `guard_t073` in this file (under `---- <test> stdout ----` blocks of the "successes" section), verbatim:
```
---- acw2_root_parent_replaced_is_outside_root stdout ----
MEASURED: acw2_root_parent_replaced_is_outside_root: windows: rename of p while the root is held: refused: kind PermissionDenied, raw OS error Some(5) (Access is denied. (os error 5))
---- acw2_root_replaced_at_same_path_is_outside_root stdout ----
MEASURED: acw2_root_replaced_at_same_path_is_outside_root: windows: rename of root while the root is held: ok
```

### 2b. "kiln-workspace test targets (reported, not required)" (`windows-tests.txt`, ANSI stripped)
Every `==== <target>: exit status N` line, verbatim (15 targets; `guard_t074` is new compared with `T-072-round-1.md`'s 14; the other 14 have the same status as there, in the same order):
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
Targets other than `guard_t074` and `acceptance_t002`: status line only (above); no status differs from `T-072-round-1.md`. Content was not compared further.

#### `guard_t074` (13 tests, verbatim block lines 1006-1168 of the cleaned file: test lines, `successes:` section with the captured stdout blocks, result line)
Test lines:
```
test awa1_file_link_absolute_on_the_walk ... ok
test awa1_file_link_relative_on_the_walk ... ok
test awa3_temp_name_held_by_a_dangling_file_link_to_outside ... ok
test awa3_temp_name_held_by_a_file_link_to_an_outside_file ... ok
test awa3_temp_name_held_by_a_junction ... ok
test awa3_temp_names_held_by_a_junction_and_a_dangling_file_link ... ok
test awa4_dotdot_after_a_junction_in_the_root_text ... ok
test awa1_directory_link_on_the_walk ... ok
test awa4_root_given_through_a_directory_link_and_retargeted ... ok
test awa4_root_removed_while_held ... ok
test awa1_junction_on_the_walk ... ok
test awa4_root_given_through_a_junction_and_retargeted ... ok
test awa4_root_replaced_by_a_junction ... ok
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
```
No test failed (exit status 0), so there is no failing stdout or panic. Every line containing `MEASURED:`, in order, under the `---- <test> stdout ----` block it belongs to (`grep -c MEASURED:` over the whole file gives 98, all inside this target):
```
---- awa1_file_link_absolute_on_the_walk stdout ----
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Inside as Middle component: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Inside as Middle component: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l/x.txt") }))
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Inside as Middle component: windows: write_atomic OutsideRoot
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Inside as Last component: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Inside as Last component: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l") }))
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Inside as Last component: windows: write_atomic OutsideRoot
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Outside as Middle component: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Outside as Middle component: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l/x.txt") }))
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Outside as Middle component: windows: write_atomic OutsideRoot
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Outside as Last component: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Outside as Last component: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l") }))
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Outside as Last component: windows: write_atomic OutsideRoot
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Nowhere as Middle component: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Nowhere as Middle component: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l/x.txt") }))
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Nowhere as Middle component: windows: write_atomic OutsideRoot
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Nowhere as Last component: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Nowhere as Last component: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l") }))
MEASURED: awa1_file_link_absolute_on_the_walk: FileAbs to Nowhere as Last component: windows: write_atomic OutsideRoot
---- awa1_file_link_relative_on_the_walk stdout ----
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Inside as Middle component: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Inside as Middle component: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l/x.txt") }))
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Inside as Middle component: windows: write_atomic OutsideRoot
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Inside as Last component: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Inside as Last component: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l") }))
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Inside as Last component: windows: write_atomic OutsideRoot
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Outside as Middle component: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Outside as Middle component: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l/x.txt") }))
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Outside as Middle component: windows: write_atomic OutsideRoot
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Outside as Last component: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Outside as Last component: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l") }))
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Outside as Last component: windows: write_atomic OutsideRoot
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Nowhere as Middle component: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Nowhere as Middle component: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l/x.txt") }))
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Nowhere as Middle component: windows: write_atomic OutsideRoot
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Nowhere as Last component: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Nowhere as Last component: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l") }))
MEASURED: awa1_file_link_relative_on_the_walk: FileRel to Nowhere as Last component: windows: write_atomic OutsideRoot
---- awa3_temp_name_held_by_a_dangling_file_link_to_outside stdout ----
MEASURED: awa3_temp_name_held_by_a_dangling_file_link_to_outside: windows: write_atomic(f.txt) beside the links: Ok(())
MEASURED: awa3_temp_name_held_by_a_dangling_file_link_to_outside: windows: Document save of f.txt beside the links: Ok(())
---- awa3_temp_name_held_by_a_file_link_to_an_outside_file stdout ----
MEASURED: awa3_temp_name_held_by_a_file_link_to_an_outside_file: windows: write_atomic(f.txt) beside the links: Ok(())
MEASURED: awa3_temp_name_held_by_a_file_link_to_an_outside_file: windows: Document save of f.txt beside the links: Ok(())
---- awa3_temp_name_held_by_a_junction stdout ----
MEASURED: awa3_temp_name_held_by_a_junction: windows: write_atomic(f.txt) beside the links: Ok(())
MEASURED: awa3_temp_name_held_by_a_junction: windows: Document save of f.txt beside the links: Ok(())
---- awa3_temp_names_held_by_a_junction_and_a_dangling_file_link stdout ----
MEASURED: awa3_temp_names_held_by_a_junction_and_a_dangling_file_link: windows: write_atomic(f.txt) beside the links: Ok(())
MEASURED: awa3_temp_names_held_by_a_junction_and_a_dangling_file_link: windows: Document save of f.txt beside the links: Ok(())
---- awa4_dotdot_after_a_junction_in_the_root_text stdout ----
MEASURED: awa4_dotdot_after_a_junction_in_the_root_text: --root k/link/../b: windows: read f.txt: Ok
---- awa1_directory_link_on_the_walk stdout ----
MEASURED: awa1_directory_link_on_the_walk: DirSym to Inside as Middle component: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_directory_link_on_the_walk: DirSym to Inside as Middle component: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l/x.txt") }))
MEASURED: awa1_directory_link_on_the_walk: DirSym to Inside as Middle component: windows: write_atomic OutsideRoot
MEASURED: awa1_directory_link_on_the_walk: DirSym to Inside as Last component: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_directory_link_on_the_walk: DirSym to Inside as Last component: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l") }))
MEASURED: awa1_directory_link_on_the_walk: DirSym to Inside as Last component: windows: write_atomic OutsideRoot
MEASURED: awa1_directory_link_on_the_walk: DirSym to Outside as Middle component: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_directory_link_on_the_walk: DirSym to Outside as Middle component: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l/x.txt") }))
MEASURED: awa1_directory_link_on_the_walk: DirSym to Outside as Middle component: windows: write_atomic OutsideRoot
MEASURED: awa1_directory_link_on_the_walk: DirSym to Outside as Last component: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_directory_link_on_the_walk: DirSym to Outside as Last component: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l") }))
MEASURED: awa1_directory_link_on_the_walk: DirSym to Outside as Last component: windows: write_atomic OutsideRoot
MEASURED: awa1_directory_link_on_the_walk: DirSym to Nowhere as Middle component: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_directory_link_on_the_walk: DirSym to Nowhere as Middle component: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l/x.txt") }))
MEASURED: awa1_directory_link_on_the_walk: DirSym to Nowhere as Middle component: windows: write_atomic OutsideRoot
MEASURED: awa1_directory_link_on_the_walk: DirSym to Nowhere as Last component: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_directory_link_on_the_walk: DirSym to Nowhere as Last component: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l") }))
MEASURED: awa1_directory_link_on_the_walk: DirSym to Nowhere as Last component: windows: write_atomic OutsideRoot
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
---- awa1_junction_on_the_walk stdout ----
MEASURED: awa1_junction_on_the_walk: Junction to Inside as Middle component: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_junction_on_the_walk: Junction to Inside as Middle component: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l/x.txt") }))
MEASURED: awa1_junction_on_the_walk: Junction to Inside as Middle component: windows: write_atomic OutsideRoot
MEASURED: awa1_junction_on_the_walk: Junction to Inside as Last component: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_junction_on_the_walk: Junction to Inside as Last component: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l") }))
MEASURED: awa1_junction_on_the_walk: Junction to Inside as Last component: windows: write_atomic OutsideRoot
MEASURED: awa1_junction_on_the_walk: Junction to Outside as Middle component: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_junction_on_the_walk: Junction to Outside as Middle component: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l/x.txt") }))
MEASURED: awa1_junction_on_the_walk: Junction to Outside as Middle component: windows: write_atomic OutsideRoot
MEASURED: awa1_junction_on_the_walk: Junction to Outside as Last component: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_junction_on_the_walk: Junction to Outside as Last component: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l") }))
MEASURED: awa1_junction_on_the_walk: Junction to Outside as Last component: windows: write_atomic OutsideRoot
MEASURED: awa1_junction_on_the_walk: Junction to Nowhere as Middle component: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_junction_on_the_walk: Junction to Nowhere as Middle component: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l/x.txt") }))
MEASURED: awa1_junction_on_the_walk: Junction to Nowhere as Middle component: windows: write_atomic OutsideRoot
MEASURED: awa1_junction_on_the_walk: Junction to Nowhere as Last component: windows: read OutsideRoot, is_file false, check_location OutsideRoot, list_dir OutsideRoot
MEASURED: awa1_junction_on_the_walk: Junction to Nowhere as Last component: windows: Document open Some(Vfs(OutsideRoot { path: RelPath("l") }))
MEASURED: awa1_junction_on_the_walk: Junction to Nowhere as Last component: windows: write_atomic OutsideRoot
---- awa4_root_given_through_a_junction_and_retargeted stdout ----
MEASURED: awa4_root_given_through_a_junction_and_retargeted: windows: Junction as --root: read f.txt: Ok([114, 101, 97, 108, 10])
MEASURED: awa4_root_given_through_a_junction_and_retargeted: Junction retargeted to another directory: windows: read OutsideRoot, write_atomic OutsideRoot, is_file false, list_dir OutsideRoot, check_location OutsideRoot, save OutsideRoot
MEASURED: awa4_root_given_through_a_junction_and_retargeted: Junction pointed back: windows: read f.txt: Ok
---- awa4_root_replaced_by_a_junction stdout ----
MEASURED: awa4_root_replaced_by_a_junction: windows: rename of the held root: Ok(())
MEASURED: awa4_root_replaced_by_a_junction: a junction to another directory at the root's path: windows: read OutsideRoot, write_atomic OutsideRoot, is_file false, list_dir OutsideRoot, check_location OutsideRoot, save OutsideRoot
MEASURED: awa4_root_replaced_by_a_junction: a junction at the root's path to the held root: windows: read f.txt: Ok
```

#### `acceptance_t002` (15 tests; exit status 101)
```
running 15 tests
test bom_only_file_round_trips ... ok
test dirty_follows_text_versus_last_saved ... ok
test disk_save_replaces_via_rename_without_leftovers ... FAILED
test invalid_bytes_are_reported_with_offset_never_replaced ... ok
test line_endings_round_trip_verbatim ... ok
test failed_save_leaves_file_intact_and_document_dirty ... ok
test relpath_accepts_and_rejects_exact_cases ... ok
test save_through_symlink_updates_target ... ok
test encode_decode_round_trip ... ok
test utf16_le_and_be_with_bom_open_and_save ... ok
test utf8_bom_detected_stripped_and_restored_on_save ... ok
test vfs_errors_name_relative_paths_only ... ok
test unedited_utf8_files_round_trip_byte_for_byte ... ok
test disk_vfs_results_do_not_depend_on_root ... ok
test memory_and_disk_vfs_agree_on_random_operation_sequences ... ok
test result: FAILED. 14 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
```
Complete block of the failing test, verbatim:
```
---- disk_save_replaces_via_rename_without_leftovers stdout ----

thread 'disk_save_replaces_via_rename_without_leftovers' (5140) panicked at crates\kiln-workspace\tests\acceptance_t002.rs:940:5:
precondition: permission bits and inode numbers are Unix metadata; the Windows checks of atomic replace are not written yet (ADR-0009: T-073 and T-074 write the Windows behaviour and its tests)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


```

### 2c. `xtask` tests on Windows (`core-crates.txt`, block after `==== cargo test … -p xtask`)
`==== xtask: exit status 101`; sum of the 26 `test result:` lines of that block: 303 passed, 2 failed (command: `sed -n 322,816p` of the ANSI-stripped file piped to `grep "^test result"` and `awk "{p+=\$4;f+=\$6}"`). Other packages: `kiln-text`, `kiln-syntax`, `kiln-search` exit status 0.
- `acceptance_t074`: `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s`
- `acceptance_t072`: `test result: FAILED. 15 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.66s`
- `guard_t072`: `test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 127.80s`
- failing tests (by name, with messages verbatim):
```
```
The two failing names are `ac1_the_list_step_runs_the_list_records_its_status_and_prints_the_binaries` and `docs_ci_table_shows_a_green_gate_macos_with_the_inode_columns`; no other xtask test failed.
