# CI run 44268710587 (platforms.yml) on ci/candidate @ 786c592b21740de950ee0bd9576a9f07ec7884c8 — T-073 round 1

Record by run R-344 (planner of ADR-0009); method as `tasks/ci-runs/README.md` (artefacts first).

## Run header
- id 44268710587, run_number 22, attempt 1, event push, workflow `platforms`, title "Join task/T-073-tests into task/T-073"
- branch `ci/candidate`; commit `786c592b21740de950ee0bd9576a9f07ec7884c8`; created_at 2026-11-18T08:38:13Z
- status at 09:08Z: every job completed (`gate-macos` uploaded its reports at 09:07:52Z and was in post-job cleanup); the
  run's conclusion is failure because of `gate-windows` and `window-probe (windows-2025)`

## Job table
| job | id | conclusion | wall | first failing step |
|---|---|---|---|---|
| gate-linux | 134679587059 | **success** | 11m34s (08:38:16Z–08:49:50Z) | none |
| gate-macos | 134679587105 | **success** (every step through "Upload reports" success; read before the job's own completion time was set) | about 29m35s (08:38:17Z–09:07:52Z, upload) | none |
| gate-windows | 134679587067 | failure | 17m48s (08:38:16Z–08:56:04Z) | Gate (08:38:50Z–08:44:56Z) |
| window-linux | 134679587079 | **success** | 1m51s (08:38:16Z–08:40:07Z) | none |
| window-probe (macos-15) | 134679587108 | **success** | 2m34s (08:38:16Z–08:40:50Z) | none |
| window-probe (windows-2025) | 134679586754 | failure | 2m21s (08:38:15Z–08:40:36Z) | **Window probe** (08:40:27Z–08:40:30Z); "Build kiln" **success** (08:38:43Z–08:40:27Z) |

Artefacts: `gate-windows-…` 16,269 B (`gate-report.json`, `core-crates.txt`, `test-output.txt`, `workspace-tests.txt`),
`gate-linux-…` 10,304 B, `window-linux-…` 577 B, `window-probe-macos-15-…` 703 B, `window-probe-windows-2025-…` 344 B;
`gate-macos-…` 10,299 B (`gate-report.json`, `core-crates.txt`). Fetched with `download_workflow_run_artifact` into
`/home/user/kiln-plan-T-073-artefact/` (deleted after this record), sizes equal to `size_in_bytes`, `testzip()` clean.

Counted window jobs: `window-linux` — `WINDOW CASES PASS`, `AC-3: exit=2 stdout_bytes=0 stderr_lines=1 panicked=0 (pass: 2
0 1 0)`, `AC-3 PASS`, `PLAIN PASS: the start path changes the screen`; `window-probe (macos-15)` — `PROBE: exit status
0`, `PROBE: stdout     2174 bytes, 3 lines`.

## gate-linux
Gate report `passed: true`: every step PASS (fuzz, budgets EMPTY); scorecard "64 items: 47 verified, 0 implemented, 0
failing, 17 planned."; test "1146 passed, 0 failed, 0 ignored in 135 suites"; no-skip "self-test ok (17 known findings);
201 files scanned, 0 allowlisted skip(s), **35 platform test(s) registered (23 pending)**, today = 2026-11-18".

## gate-windows, step by step (`gate-report.json`, ANSI stripped)
`passed: false`, `failed_step: clippy`, 347.5 s.
| step | status | detail (first line) and what the failures name |
|---|---|---|
| fmt | PASS | formatted |
| clippy | **FAIL** | compiler output; errors only in `apps\kiln-cli\tests\acceptance_t019_cli.rs` (`:23:14` E0433 `cannot find unix in os`; `:407:26`, `:425:61` E0599 `is_fifo`), then `error: could not compile kiln-cli (test "acceptance_t019_cli") due to 3 previous errors` — a product **test** target (T-092); clippy stops at the first failing crate, so later test targets are not reached |
| layers | PASS | 11 crates, 14 internal edges, layer table holds |
| ui-commands | PASS | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | PASS | reviews: 57 of 69 …; session-paths: 684 tracked file(s) clean |
| schema | **PASS** | 30 commands; audit ok; docs/commands.json current (round 2 of T-079: FAIL) |
| scorecard | **FAIL** | first line `windows: 64 items; 158 tests ran; 3 registered absent here; page not compared (computed on Linux)`, then `60 scorecard problems:` — `building test executables failed on windows (); their tests are not run` and lines `… lists test …, which does not exist on windows` (e.g. `cmd.registry` … `kiln_commands::registry::tests::…`) |
| test | **FAIL** | "0 passed, 0 failed, 0 ignored in 0 suites"; errors named: `apps\kiln\tests\acceptance_t055.rs` (E0433 `unix`), `could not compile kiln (test "script_t055") due to 1 previous error`, `… (test "acceptance_t055") due to 2 previous errors` — product **test** targets (T-092) |
| no-skip | PASS | self-test ok (17 known findings); 201 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (23 pending), today = 2026-11-18 |
| fuzz, budgets | EMPTY | no targets registered yet |
`test-output.txt` (the full-test-output step; 172 lines): `could not compile kiln (test "window_t065") due to 8 previous
errors`, `… (test "acceptance_t065") due to 3 …`, `… (test "script_t055") due to 1 …`, `… (test "acceptance_t055") due to 2 …`
(E0433 `unix` ×5, E0277 `[OsString]` ×6, `OsStr::from_bytes` ×2, `OsString::from_vec` ×1) — all in `apps\kiln\tests\`. **No error
in a library or binary, and none in `kiln-workspace`** (the gate's `schema` step built and ran `kiln-cli`; `window-probe`
built `kiln`).

## The reported step "Workspace lib and T-073 tests" (`workspace-tests.txt`)
Compiled: `Compiling kiln-workspace …`, `Finished test profile … in 4.55s`; three binaries, all `ok`:
- `Running unittests src\lib.rs` — `running 35 tests` — `test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.75s`
- `Running tests\acceptance_t073.rs` — `running 1 test` — `test result: ok. 1 passed; 0 failed …`:
  `acq4_same_file_is_declared_only_as_windows_dependency_at_exact_version ... ok`
- `Running tests\guard_t073.rs` — `running 10 tests` — `test result: ok. 10 passed; 0 failed …`: `acw2_missing_root_is_refused`,
  `acw2_file_as_root_is_refused`, `acw2_existing_root_is_accepted_and_used`, `acw2_save_creates_temp_exclusively`,
  `acw2_root_parent_replaced_is_outside_root`, `acw2_save_of_new_file_creates_it_and_leaves_no_temp`,
  `acw2_root_replaced_at_same_path_is_outside_root`, `acw2_save_keeps_writable_target_writable`,
  `acw2_save_of_read_only_target_is_refused_and_changes_nothing`, `acw2_save_replaces_target_by_rename_and_leaves_no_temp`,
  each `ok`.
The 35 lib tests: `disk::tests::errors_are_root_independent` ok; `disk::tests::failed_rename_removes_own_temp_and_keeps_target` ok; `disk::tests::new_rejects_missing_root_and_file_root_without_naming_them` ok; `disk::tests::write_read_and_errors` ok; `document::tests::dirty_follows_content` ok; `document::tests::disk_save_onto_path_replaced_by_directory_fails_and_changes_nothing` ok; `document::tests::edits_keep_encoding_and_inserted_newline_stays_lf` ok; `document::tests::failed_save_keeps_file_and_dirty_state` ok; `document::tests::open_errors` ok; `document::tests::unedited_round_trip_is_byte_exact_for_each_encoding` ok; `document::tests::works_through_trait_objects` ok; `encoding::tests::decode_error_display` ok; `encoding::tests::detects_boms_and_strips_exactly_one` ok; `disk::tests::longest_file_names_can_be_saved` ok; `encoding::tests::exact_error_offsets` ok; `encoding::tests::names` ok; `encoding::tests::utf16_with_surrogate_pair` ok; `memory::tests::check_location_is_always_ok_and_changes_nothing` ok; `memory::tests::directories_are_implicit` ok; `memory::tests::failing_writes_change_nothing` ok; `memory::tests::list_dir_lists_implied_directories_in_byte_order` ok; `memory::tests::sibling_with_common_prefix_is_not_a_child` ok; `memory::tests::write_into_missing_parent_is_not_found_and_creates_nothing` ok; `encoding::tests::encode_decode_inverse_on_generated_strings` ok; `memory::tests::write_then_read_round_trips_and_creates_top_level_files` ok; `path::tests::accepts_valid_paths_verbatim` ok; `path::tests::parent_and_segments` ok; `path::tests::rejects_invalid_paths` ok; `project::tests::limit_edges` ok; `project::tests::bom_and_invalid_lines` ok; `project::tests::negation_and_deeper_files` ok; `project::tests::under_uses_prefix_gitignore_and_is_not_itself_subject` ok; `project::tests::worked_example` ok; `disk::tests::all_temp_names_taken_fails_without_changes_then_recovers` ok; `disk::tests::matches_memory_model_on_random_operations` ok.
`MEASURED:` lines, exactly as printed:
```
MEASURED: acw2_root_parent_replaced_is_outside_root: windows: rename of p while the root is held: refused: kind PermissionDenied, raw OS error Some(5) (Access is denied. (os error 5))
MEASURED: acw2_root_replaced_at_same_path_is_outside_root: windows: rename of root while the root is held: ok
```
No test failed (no failure message to quote).

## window-probe (windows-2025) (`window-probe.txt`)
"Build kiln" **success** (08:38:43Z–08:40:27Z); the "Window probe" step failed (it exits with `kiln`'s status):
```
PROBE OS: windows-2025 MINGW64_NT-10.0-26100 runnervmm7ud1 3.6.10-5a1665c8.x86_64 2026-09-28 10:40 UTC x86_64 Msys
PROBE: exit status 2
PROBE: stdout 0 bytes, 0 lines
--- stderr
kiln: no window: egui_glow: OpenGL: egui_glow requires opengl 2.0+.
```
That is the specified no-window exit (status 2, one stderr line): the runner gives no OpenGL 2.0+ (ADR-0009 (b) had
inferred this; now shown).

## xtask (`core-crates.txt`)
Linux `==== xtask: exit status 0`, 23 binaries, 287 passed, 0 failed; Windows `==== xtask: exit status 0`, 23 binaries,
279 passed, 0 failed. Core crates exit status 0 on both. `no-skip` registered: 35 (23 pending) on Linux and Windows.

## gate-macos (counted, D-248)
Gate report `passed: true`, 1,178.3 s: every step PASS (fuzz, budgets EMPTY); scorecard `macos: 64 items; 216 tests ran;
3 registered absent here; page not compared (computed on Linux)` (no problem after it); test "1129 passed, 0 failed, 0
ignored in 135 suites"; no-skip "… 35 platform test(s) registered (23 pending) …". `xtask`: `==== xtask: exit status 0`,
23 binaries, 282 passed, 0 failed. No full test output (the gate passed).

## Against T-073's pending CI criterion
| Clause | Verdict | Quote |
|---|---|---|
| `gate-linux` green | **met** | `passed: true`; test "1146 passed, 0 failed, 0 ignored in 135 suites" |
| `gate-macos` green | **met** | `passed: true`; test "1129 passed, 0 failed …" |
| `window-linux`, `window-probe (macos-15)` green | **met** | `WINDOW CASES PASS`, `AC-3 PASS`, `PLAIN PASS`; `PROBE: exit status 0` |
| `gate-windows`: fmt, layers, ui-commands, hygiene, **schema**, no-skip PASS | **met** | the six steps PASS; schema "30 commands; audit ok; docs/commands.json current" |
| clippy, scorecard, test FAIL only with errors in test targets other than `kiln-workspace`'s lib test and `acceptance_t073`, named | **met** (as far as printed) | clippy: `kiln-cli (test "acceptance_t019_cli")`; test: `kiln (test "script_t055")`, `kiln (test "acceptance_t055")`; full output: `kiln` tests `window_t065`, `acceptance_t065`, `script_t055`, `acceptance_t055`; scorecard: "building test executables failed" and listed tests "which does not exist on windows". Cargo stops at the first failing targets, so not every failing test target is named |
| core-crates `xtask` exit status 0 | **met** | `==== xtask: exit status 0` (279 / 0) |
| the new step: every `acceptance_t073` and `kiln-workspace` lib test `ok` | **met** | lib 35/0, `acceptance_t073` 1/0, `guard_t073` 10/0, each `ok` by name |
| `window-probe (windows-2025)`: "Build kiln" success and a `PROBE:` exit-status line | **met** | Build kiln success; `PROBE: exit status 2` (`kiln: no window: egui_glow: OpenGL: egui_glow requires opengl 2.0+.`) |

## Differences from `tasks/ci-runs/T-079-round-2.md`
- Windows: `kiln-workspace` and every library and binary now build (round 2: `could not compile kiln-workspace (lib) due to
  6 previous errors`); schema FAIL → **PASS**; scorecard "18 tests ran … 199 scorecard problems" → "158 tests ran … 60
  scorecard problems"; clippy and test now fail at product test targets of `kiln-cli` and `kiln` (T-092); "Build kiln" in
  `window-probe` failure → success, the probe now runs: exit status 2, no OpenGL 2.0+; new reported step 46/0; `xtask`
  278 → 279 passed; no-skip 20 (8 pending) → 35 (23 pending) registered.
- macOS: green as before; test 1117 → 1129 passed in 135 suites; `xtask` 281 → 282; no-skip 35 registered.
- Linux: green as before; test 1134 → 1146 passed in 135 suites; `xtask` 286 → 287; no-skip 35 registered.

## Not read
- Every failing Windows test target (cargo stops building after the first failures in clippy and test; the full output
  names only `kiln`'s four).
- macOS and Linux product test names (no `test-output.txt` on a green gate; T-072 changes that).
- Job logs (all artefacts uploaded).
