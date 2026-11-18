# CI run 44383948281 (platforms.yml) on ci/candidate @ b924926ab8be01798818f5c7f2bde3f6ba8b4884 — T-092 round 1

Record by hand; method as `tasks/ci-runs/README.md`; same steps as `T-074-round-1.md`. Task T-092 rewrote tests of `kiln-workspace` so that 13 of its 15 test targets compile and pass on Windows (as stated in the brief; not checked here). Every value comes from this run's job listing, artefacts or job logs. No judgement of results is made.

**Complete: the run is `completed` (conclusion failure; `updated_at` 2026-11-18T20:24:52Z); every job has finished. At the first reads (about 20:13Z) `gate-windows` and `gate-macos` were still running; `gate-windows` finished 20:14:15Z and `gate-macos` 20:24:51Z. Nothing was still running at the end.**

## Run header
- id 44383948281, run_number 43, attempt 1, event push, workflow `platforms`; branch `ci/candidate`; commit `b924926ab8be01798818f5c7f2bde3f6ba8b4884` (equals the head asked for); title "Join task/T-092-tests into task/T-092"; created_at 2026-11-18T19:47:20Z.

## macOS runner question (first)
- `gate-macos` (job 135083317097): **a runner was assigned** (`runner_id` 1000001132); job created 19:47:21Z, started 19:57:25Z (about 10 min after creation, INFERRED to be the wait for a runner).
- `window-probe (macos-15)` (job 135083317051): **a runner was assigned** (`runner_id` 1000001133); started 19:57:23Z.

## Job table
Wall time = `completed_at` − `started_at` of the job (job listing).
| job | id | runner (`runner_id`) | conclusion | wall | first failing step |
|---|---|---|---|---|---|
| gate-linux | 135083317213 | 1000001124 | success | 16m20s (19:47:23Z–20:03:43Z) | none |
| gate-windows | 135083317155 | 1000001128 | **failure** | 26m52s (19:47:23Z–20:14:15Z) | Gate (19:47:53Z–19:56:04Z, 8m11s); every later step success or skipped |
| gate-macos | 135083317097 | 1000001132 | success | 27m26s (19:57:25Z–20:24:51Z; created 19:47:21Z) | none |
| window-linux | 135083316575 | 1000001123 | success | 1m49s (19:47:23Z–19:49:12Z) | none |
| window-probe (macos-15) | 135083317051 | 1000001133 | success | 48s (19:57:23Z–19:58:11Z) | none |
| window-probe (windows-2025) | 135083317168 | 1000001127 | **failure** | 1m59s (19:47:23Z–19:49:22Z) | Window probe (19:49:14Z–19:49:16Z); Build kiln success |

gate-windows steps (job listing): Gate failure (19:47:53Z–19:56:04Z); Gate report success; Core crates success (19:56:04Z–20:13:40Z, 17m36s); Workspace lib and T-073 tests success (8s); Test list success (1s); kiln-workspace test targets success (12s); Full test output (gate failed) success, ran (6s); Disk, Upload reports success; Post Cache skipped.

Artefacts (6 at the end): `gate-linux-44383948281` 32319 B, `gate-windows-44383948281` 24664 B, `gate-macos-44383948281` 32019 B, `window-linux-44383948281` 577 B, `window-probe-macos-15-44383948281` 704 B, `window-probe-windows-2025-44383948281` 344 B. The three gate artefacts were fetched with `download_workflow_run_artifact` and curl (size equal to `size_in_bytes`, `testzip()` clean); the window artefacts were not downloaded. Scratch directory removed after the record. Job logs read (logs URL + curl): the three gate jobs (image block, status lines).

## gate-linux (job 135083317213, ubuntu-24.04)
- Image (job log): `Image: ubuntu-24.04`, `Version: 20261111.127.1`. Artefact `gate-linux-44383948281` (id 14466856429, 32319 B; fetched, size equal, `testzip()` clean): `core-crates.txt`, `gate-report.json`, `test-list.txt`; no `test-output.txt` (gate green).
- Gate report: `passed: true`, `failed_step: null`, 634.73 s.
| step | status | seconds | detail |
|---|---|---|---|
| fmt | pass | 0.89 | formatted |
| clippy | pass | 9.64 | no warnings (all targets, -D warnings) |
| layers | pass | 0.45 | 11 crates, 14 internal edges, layer table holds |
| ui-commands | pass | 0.03 | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | pass | 0.07 | reviews: 64 of 76 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 719 tracked file(s) clean (0 not UTF-8, skipped) |
| schema | pass | 4.53 | 30 commands; audit ok; docs/commands.json current |
| scorecard | pass | 151.22 | 64 items: 47 verified, 0 implemented, 0 failing, 17 planned. |
| test | pass | 465.59 | `1212 passed, 0 failed, 0 ignored in 142 suites` |
| no-skip | pass | 2.32 | self-test ok (17 known findings); 208 files scanned, 0 allowlisted skip(s), 47 platform test(s) registered (8 pending), today = 2026-11-18 |
| fuzz, budgets | empty | 0.0 | no targets registered yet |
- `TEST LIST` status line (job log): `TEST LIST: exit status 0`, then `TEST LIST: the status is recorded, not used; the gate step decides the colour`.
- No failing test. `core-crates.txt`: `==== kiln-text/kiln-syntax/kiln-search/xtask: exit status 0` (all four); sum of its `test result:` lines 511 passed, 0 failed, 0 ignored.
- `acceptance_t092` (xtask, `Running tests/acceptance_t092.rs`): `running 5 tests`, `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.62s`. `guard_t092`: `running 2 tests`, `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.23s`.

## gate-windows (job 135083317155, windows-2025) — the subject
- Runner 1000001128. Image (job log): `Image: windows-2025-vs2026`, `Version: 20261103.298.1`; `cargo 1.97.0 (c980f4866 2026-06-30)`. Artefact `gate-windows-44383948281` (id 14467003153, 24664 B; fetched, size equal, `testzip()` clean): `core-crates.txt` 42254 B, `gate-report.json` 15524 B, `test-list.txt` 10832 B, `test-output.txt` 5023 B, `windows-tests.txt` 54205 B, `workspace-tests.txt` 8382 B.
- Job-log status lines: `CORE CRATES: exit status per package: kiln-text=0 kiln-syntax=0 kiln-search=0 xtask=0 (reported, not required)`; `WORKSPACE TESTS: exit status 0 (reported, not required)`; `TEST LIST: exit status 101` and `TEST LIST: the status is recorded, not used; the gate step decides the colour`; `TEST OUTPUT: exit status 101`.
- Gate report (`gate-report.json`, ANSI stripped): `passed: false`, `failed_step: clippy`, 466.55 s.
| step | status | seconds | detail |
|---|---|---|---|
| fmt | pass | 2.50 | formatted |
| clippy | **fail** | 64.18 | kept text 60 lines; first `error` line `error[E0433]: cannot find `unix` in `os`` at `apps\kiln-cli\tests\guard_t038_cli.rs:26:14`; then `error[E0425]: cannot find value `ELOOP` in this scope` (`guard_t038_cli.rs:96:18`), `error[E0599]: no method named `is_fifo` found for struct `std::fs::FileType`` (`guard_t038_cli.rs:126:57`); ends `error: could not compile `kiln-cli` (test "guard_t038_cli") due to 3 previous errors` |
| layers | pass | 11.02 | 11 crates, 14 internal edges, layer table holds |
| ui-commands | pass | 0.05 | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | pass | 0.19 | reviews: 64 of 76 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 719 tracked file(s) clean (0 not UTF-8, skipped) |
| schema | pass | 35.91 | 30 commands; audit ok; docs/commands.json current |
| scorecard | **fail** | 345.58 | first line `windows: 64 items; 165 tests ran; 8 registered absent here; page not compared (computed on Linux)`, then `50 scorecard problems:`; first problem `building test executables failed on windows (); their tests are not run`; the next listed is `cmd.registry: lists test `kiln_commands::registry::tests::rejects_params_that_violate_schema`, which does not exist on windows` (the rest are `<item>: lists test `<name>`, which does not exist on windows`) |
| test | **fail** | 1.13 | `0 passed, 0 failed, 0 ignored in 0 suites`; kept text 27 lines; first `error` line `error[E0433]: cannot find `unix` in `os`` at `apps\kiln\tests\acceptance_t055.rs:1317:18`; ends `error: could not compile `kiln` (test "window_t065") due to 8 previous errors`, `... (test "script_t055") due to 1 previous error`, `... (test "acceptance_t055") due to 2 previous errors` |
| no-skip | pass | 5.98 | self-test ok (17 known findings); 208 files scanned, 0 allowlisted skip(s), 47 platform test(s) registered (8 pending), today = 2026-11-18 |
| fuzz, budgets | empty | 0.0 | no targets registered yet |
- (e) Failing steps of the gate and the first target each names: **clippy** names `kiln-cli` test `guard_t038_cli` (the gate stopped reporting at clippy: `failed_step: clippy`); **scorecard** names no target in its first problem (`building test executables failed on windows ()`); its first listed test is `kiln_commands::registry::tests::rejects_params_that_violate_schema`; **test** first names `apps\kiln\tests\acceptance_t055.rs:1317:18`, and its closing lines name `kiln` tests `window_t065`, `script_t055`, `acceptance_t055`. No `kiln-workspace` target is named by any of the three.
- "Test list": `test-list.txt` (10832 B) holds only compile errors: `could not compile` lines for `kiln` tests `window_t065` (8 errors), `acceptance_t065` (3), `script_t055` (1), `acceptance_t055` (2); no binary and no test name. "Full test output (gate failed)": ran; `test-output.txt` 5023 B, 81 lines, 0 lines containing `FAILED`; its `could not compile` lines: `acceptance_t065` (3 errors), `script_t055` (1), `acceptance_t055` (2).

### (b) Step "Workspace lib and T-073 tests (reported, not required)" (`workspace-tests.txt`)
Binaries: library `running 45 tests`, `acceptance_t073` `running 1 test`, `guard_t073` `running 10 tests`; 56 tests, 0 failed; `WORKSPACE TESTS: exit status 0`. I cannot tell from the file which library tests are new, so every test line of the library is given, verbatim (`T-074-round-1.md` lists 43 library tests; this run lists 45: the two names `disk::tests::leftover_at_temp_name_is_never_touched_on_every_platform` and `disk::tests::failed_save_after_writability_check_leaves_target_and_listing_unchanged_on_every_platform` do not occur in T-074's list of 43):
```
running 45 tests
test disk::tests::errors_are_root_independent ... ok
test disk::tests::failed_rename_removes_own_temp_and_keeps_target ... ok
test disk::tests::check_location_inside_on_every_platform ... ok
test disk::tests::held_root_check_is_inside_only_for_a_read_identity_equal_to_the_held_one ... ok
test disk::tests::leftover_at_temp_name_is_never_touched_on_every_platform ... ok
test disk::tests::check_location_outside_on_every_platform ... ok
test disk::tests::failed_save_after_writability_check_leaves_target_and_listing_unchanged_on_every_platform ... ok
test disk::tests::all_temp_names_taken_fails_without_changes_then_recovers ... ok
test disk::tests::new_rejects_missing_root_and_file_root_without_naming_them ... ok
test disk::tests::longest_file_names_can_be_saved ... ok
test disk::tests::link_at_temp_name_is_not_followed_on_every_platform ... ok
test disk::tests::root_given_through_a_link_on_every_platform ... ok
test disk::tests::root_removed_on_every_platform ... ok
test disk::tests::write_read_and_errors ... ok
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
test disk::tests::root_replaced_by_a_link_on_every_platform ... ok
test document::tests::disk_save_onto_path_replaced_by_directory_fails_and_changes_nothing ... ok
test disk::tests::matches_memory_model_on_random_operations ... ok
```
Result lines, verbatim:
```
test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.88s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
```
No failing test in this step.

### Step "kiln-workspace test targets (reported, not required)" (`windows-tests.txt`, ANSI stripped)

Every `==== <target>: exit status N` line, verbatim (15 targets):
```
==== acceptance_t002: exit status 0
==== acceptance_t002_r2: exit status 0
==== acceptance_t008: exit status 101
==== acceptance_t015_tools: exit status 0
==== acceptance_t019_workspace: exit status 101
==== acceptance_t036_workspace: exit status 0
==== acceptance_t073: exit status 0
==== confinement: exit status 0
==== guard_t008: exit status 101
==== guard_t036_removal_workspace: exit status 0
==== guard_t036_workspace: exit status 0
==== guard_t073: exit status 0
==== guard_t074: exit status 0
==== project: exit status 101
==== tools_contract: exit status 0
```
Summary line (job log), verbatim:
```
WINDOWS TESTS: exit status per target: acceptance_t002=0 acceptance_t002_r2=0 acceptance_t008=101 acceptance_t015_tools=0 acceptance_t019_workspace=101 acceptance_t036_workspace=0 acceptance_t073=0 confinement=0 guard_t008=101 guard_t036_removal_workspace=0 guard_t036_workspace=0 guard_t073=0 guard_t074=0 project=101 tools_contract=0 (reported, not required)
```
Every target's `running N tests` and `test result:` lines, verbatim, in file order (a target that does not compile has neither):
```
-- acceptance_t002
running 15 tests
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.54s
-- acceptance_t002_r2
running 8 tests
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
-- acceptance_t008
running 7 tests
test result: FAILED. 5 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
-- acceptance_t015_tools
running 3 tests
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.52s
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
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
-- guard_t008
running 1 test
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
-- guard_t036_removal_workspace
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
-- guard_t036_workspace
running 2 tests
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
-- guard_t073
running 10 tests
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
-- guard_t074
running 15 tests
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.54s
-- project
error: could not compile `kiln-workspace` (test "project") due to 3 previous errors
-- tools_contract
running 5 tests
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```


#### Targets with a non-zero status
**`acceptance_t008`** (exit 101): compiles; `running 7 tests`, `test result: FAILED. 5 passed; 2 failed`. Failing tests: `open_through_escaping_symlinks_is_refused`, `save_through_escaping_symlinks_touches_nothing`. The file's whole block, verbatim:
```
running 7 tests
test root_removed_and_replaced_by_link_is_refused ... ok
test open_through_escaping_symlinks_is_refused ... FAILED
test link_swapped_in_after_open_is_refused_at_save ... ok
test root_removed_and_replaced_by_new_directory_is_refused ... ok
test root_swapped_between_requests_is_refused ... ok
test outside_root_wins_over_not_found ... ok
test save_through_escaping_symlinks_touches_nothing ... FAILED

successes:

successes:
    link_swapped_in_after_open_is_refused_at_save
    outside_root_wins_over_not_found
    root_removed_and_replaced_by_link_is_refused
    root_removed_and_replaced_by_new_directory_is_refused
    root_swapped_between_requests_is_refused

failures:

---- open_through_escaping_symlinks_is_refused stdout ----

thread 'open_through_escaping_symlinks_is_refused' (6152) panicked at crates\kiln-workspace\tests\acceptance_t008.rs:243:42:
called `Result::unwrap()` on an `Err` value: Os { code: 2, kind: NotFound, message: "The system cannot find the file specified." }
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- save_through_escaping_symlinks_touches_nothing stdout ----

thread 'save_through_escaping_symlinks_touches_nothing' (7740) panicked at crates\kiln-workspace\tests\acceptance_t008.rs:243:42:
called `Result::unwrap()` on an `Err` value: Os { code: 2, kind: NotFound, message: "The system cannot find the file specified." }


failures:
    open_through_escaping_symlinks_is_refused
    save_through_escaping_symlinks_touches_nothing

test result: FAILED. 5 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

error: test failed, to rerun pass `-p kiln-workspace --test acceptance_t008`
error: 1 target failed:
    `-p kiln-workspace --test acceptance_t008`
```
**`guard_t008`** (exit 101): compiles; `running 1 test`, `test result: FAILED. 0 passed; 1 failed`. Failing test `symlinks_inside_root_still_work`; the file's whole block, verbatim:
```
running 1 test
test symlinks_inside_root_still_work ... FAILED

successes:

successes:

failures:

---- symlinks_inside_root_still_work stdout ----

thread 'symlinks_inside_root_still_work' (7876) panicked at crates\kiln-workspace\tests\guard_t008.rs:102:9:
assertion `left == right` failed: link root/ain changed
  left: "C:\\Users\\runneradmin\\AppData\\Local\\Temp\\k8g-9780-in-root\\root\\real\\t.txt"
 right: "\\\\?\\C:\\Users\\runneradmin\\AppData\\Local\\Temp\\k8g-9780-in-root\\root\\real\\t.txt"
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    symlinks_inside_root_still_work

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

error: test failed, to rerun pass `-p kiln-workspace --test guard_t008`
error: 1 target failed:
    `-p kiln-workspace --test guard_t008`
```
**`acceptance_t019_workspace`**: does not compile (`due to 5 previous errors`). First three `error` lines, with the `-->` line of each:
```
error[E0433]: cannot find `unix` in `os`
  --> crates\kiln-workspace\tests\acceptance_t019_workspace.rs:19:14
error[E0433]: cannot find `unix` in `os`
  --> crates\kiln-workspace\tests\acceptance_t019_workspace.rs:20:14
error[E0599]: no method named `is_fifo` found for struct `FileType` in the current scope
   --> crates\kiln-workspace\tests\acceptance_t019_workspace.rs:107:61
```
**`project`**: does not compile (`due to 3 previous errors`). The three `error` lines, with the `-->` line of each:
```
error[E0433]: cannot find `unix` in `os`
  --> crates\kiln-workspace\tests\project.rs:10:14
error[E0433]: cannot find `unix` in `os`
  --> crates\kiln-workspace\tests\project.rs:11:14
error[E0599]: no method named `is_fifo` found for struct `FileType` in the current scope
  --> crates\kiln-workspace\tests\project.rs:74:65
```

### (c) `MEASURED:` lines
`grep -c "MEASURED:" windows-tests.txt` gives 178: 2 inside the `guard_t073` block and 176 inside the `guard_t074` block (awk over the line ranges of the two `Running` blocks). The first `MEASURED:` line is line 345; the failing tests' blocks (`acceptance_t008` lines 80-120, `guard_t008` lines 267-292) contain 0 of them, so no `MEASURED:` line stands under a failing test. `workspace-tests.txt` holds 2 (the same two of `guard_t073`). Not listed here.

### (d) `xtask` on Windows (`core-crates.txt`)
`==== kiln-text: exit status 0`, `==== kiln-syntax: exit status 0`, `==== kiln-search: exit status 0`, `==== xtask: exit status 0`. Totals (sum of `test result:` lines, `awk '{p+=$4; f+=$6; i+=$8}'`): `xtask` 317 passed, 0 failed, 0 ignored; all four packages 503 passed, 0 failed, 0 ignored. No failing test.
`acceptance_t092`: `running 5 tests` ... `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.05s`. `guard_t092`: `running 2 tests` ... `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.42s`.

## gate-macos (job 135083317097, macos-15) — runner assigned (`runner_id` 1000001132)
- Image (job log): `Image: macos-15-arm64`, `Version: 20261104.0514.1`; macOS 15.8.2. Artefact `gate-macos-44383948281` (id 14469515062, 32019 B): `core-crates.txt`, `gate-report.json`, `test-list.txt`; no `test-output.txt` (gate green). Job log: `TEST LIST: exit status 0`.
- Gate report: `passed: true`, `failed_step: null`, 1070.59 s.
| step | status | seconds | detail |
|---|---|---|---|
| fmt | pass | 1.38 | formatted |
| clippy | pass | 13.88 | no warnings (all targets, -D warnings) |
| layers | pass | 1.65 | 11 crates, 14 internal edges, layer table holds |
| ui-commands | pass | 0.03 | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | pass | 0.20 | reviews: 64 of 76 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 719 tracked file(s) clean (0 not UTF-8, skipped) |
| schema | pass | 6.66 | 30 commands; audit ok; docs/commands.json current |
| scorecard | pass | 210.29 | macos: 64 items; 216 tests ran; 3 registered absent here; page not compared (computed on Linux) |
| test | pass | 832.38 | `1195 passed, 0 failed, 0 ignored in 142 suites` |
| no-skip | pass | 4.13 | self-test ok (17 known findings); 208 files scanned, 0 allowlisted skip(s), 47 platform test(s) registered (8 pending), today = 2026-11-18 |
| fuzz, budgets | empty | 0.0 | no targets registered yet |
- No failing test. `core-crates.txt`: all four `==== <pkg>: exit status 0`; sum of `test result:` lines 506 passed, 0 failed, 0 ignored. `acceptance_t092` (xtask): `running 5 tests`, `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.79s`. `guard_t092`: `running 2 tests`, `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.49s`.

## Test names listed per `kiln-workspace` test binary (`test-list.txt`)
Command: a script (`count.py`, scratch directory, removed) counting, per `Running <target> (target/debug/deps/<name>-<hash>)` line of the ANSI-stripped file, the lines ending `: test` that follow it before the next `Running` line.
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
window-linux success; window-probe (macos-15) success (runner assigned, `runner_id` 1000001133); window-probe (windows-2025) failure (step Window probe failed, Build kiln succeeded). Artefact contents not read.

## Compared with `T-074-round-1.md` (run 44358453526 @ 720f1d5; `gate-windows`, `kiln-workspace` targets)
Status = `==== <target>: exit status`; passed = from the `test result:` line. "T-074" values are those that record states; "T-092" values are from this run.
| target | T-074 status | T-074 passed | T-092 status | T-092 passed / failed |
|---|---|---|---|---|
| acceptance_t002 | 101 (compiles, fails) | 14 (1 failed) | 0 | 15 / 0 |
| acceptance_t002_r2 | 101 (does not compile) | none | 0 | 8 / 0 |
| acceptance_t008 | 101 (does not compile) | none | 101 (compiles, fails) | 5 / 2 |
| acceptance_t015_tools | 101 (compiles, fails) | 2 (1 failed) | 0 | 3 / 0 |
| acceptance_t019_workspace | 101 (does not compile) | none | 101 (does not compile) | none |
| acceptance_t036_workspace | 101 (does not compile) | none | 0 | 0 / 0 (`running 0 tests`) |
| acceptance_t073 | 0 | 1 | 0 | 1 / 0 |
| confinement | 101 (does not compile) | none | 0 | 4 / 0 |
| guard_t008 | 101 (does not compile) | none | 101 (compiles, fails) | 0 / 1 |
| guard_t036_removal_workspace | 101 (does not compile) | none | 0 | 0 / 0 (`running 0 tests`) |
| guard_t036_workspace | 101 (does not compile) | none | 0 | 2 / 0 |
| guard_t073 | 0 | 10 | 0 | 10 / 0 |
| guard_t074 | 0 | 15 | 0 | 15 / 0 |
| project | 101 (does not compile) | none | 101 (does not compile) | none |
| tools_contract | 0 | not stated in that record | 0 | 5 / 0 |
Other differences, from the two records: library tests 43 then, 45 now; `MEASURED:` lines in `windows-tests.txt` 178 then, 178 now (176 `guard_t074` + 2 `guard_t073` in both); `xtask` on Windows 310 passed then, 317 now; gate failing steps clippy, scorecard, test then and now (first clippy target `guard_t037_cli` then, `guard_t038_cli` now); scorecard problems 60 then, 50 now.

## Not read
Logs of all jobs except the three gate jobs' image blocks and status lines; the `window-*` artefact contents; `gate-windows` `core-crates.txt` beyond its `test result:`/`exit status` lines and the `acceptance_t092`/`guard_t092` blocks; `gate-linux` and `gate-macos` `core-crates.txt` likewise; the `MEASURED:` lines (counted, not listed); the 45 library tests' individual verdict beyond the quoted lines (each `... ok`); whether the brief's "13 of 15 targets compile and pass" matches the file (the file shows 11 targets with exit status 0 and 4 with 101; no judgement made); `test-output.txt` beyond its `could not compile` lines; the registered platform tests (`xtask/platform-tests.toml` at the commit) per platform.
