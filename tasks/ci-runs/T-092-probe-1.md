# CI run 44355322518 (platforms.yml) on ci/candidate @ b57a74ef2a9773a02a284564d880bc853cec6ae7 — T-092 probe 1

Record by hand; method as `tasks/ci-runs/README.md`. A planner's probe for task T-092 (title: "T-092 planner probe (never merged): every product test target on Windows, one cargo test each; MEASURED lines ..."); base per the commit message: main bcca328 merged with task/T-074 (ed02d3e) and task/T-072 (3fcba00). Only the job `probe-t092-windows` is recorded in detail. Every value comes from this run's job listing, the artefact `probe-t092-windows-44355322518` (file `probe-tests.txt`) or the job log. No judgement of results is made here.
**Complete: the run is `completed` (conclusion failure); `probe-t092-windows` finished (success) at 17:18:27Z, before this record was started.**

## Run and job
- run id 44355322518, run_number 38, attempt 1, event push; created 2026-11-18T17:08:47Z; updated 2026-11-18T17:53:13Z; run conclusion **failure**.
- job `probe-t092-windows`: id 134981757369, runner label windows-2025, runner assigned (runner_id 1000001050); conclusion **success**; wall 9m37s (started 17:08:50Z, completed 17:18:27Z). Step "Product test targets (probe, reported)": success, 17:09:17Z–17:17:59Z (8m42s). Other steps success (the post-cache step finished 17:18:00Z–17:18:23Z; its log has `##[warning]Failed to save:` after `tar.exe: target/tmp/probe_t092/a/b\342\236\263.rs: File removed before we read it`).
- Image (job log): `Image: windows-2025-vs2026`, `Version: 20261103.298.1`; runner 2.338.2; `cargo 1.97.0 (c980f4866 2026-06-30)`.
- Artefact: `probe-t092-windows-44355322518`, id 14452042423, 30,579 B (fetched size equal; `testzip()` clean); one file `probe-tests.txt` (271,835 B, 4,764 lines after ANSI strip).
- The final summary line `PROBE TESTS: exit status per target:` is not in `probe-tests.txt` (grep finds none); it is in the job log (line 782 of the log with timestamps stripped).

## Other jobs of the run (job listing)
gate-linux failure; gate-windows failure; gate-macos failure (runner assigned, 1000001055); window-linux success; window-probe (windows-2025) failure; window-probe (macos-15) success (runner assigned, 1000001049).

## Totals (commands run on the ANSI-stripped, CR-stripped file)
- Table: `grep -E '^==== .*: exit status' probe.clean` gives 96 targets; `awk '{print $NF}' | sort | uniq -c`: 64 with status 0, 32 with status 101.
- Of the 32 non-zero: **26 do not compile** (block contains `error: could not compile`), **6 compile and fail** (no such line; each has a `test … FAILED` line).
- So: **64 targets exit 0; 26 do not compile; 6 compile and fail.**
- `grep -c 'MEASURED:' probe-tests.txt` = **207** (also 207 lines starting `MEASURED:`). Per target (script `awk` over the blocks): `kiln-workspace --test guard_t073` 2, `kiln-workspace --test guard_t074` 176, `kiln-workspace --test probe_t092` 25, `kiln --test probe_t092` 4; no other target has a `MEASURED:` line.

## Table of targets: every `==== <package> <target>: exit status N` line, verbatim (96)
```
==== kiln-workspace --lib: exit status 0
==== kiln-workspace --doc: exit status 0
==== kiln-workspace --test acceptance_t002: exit status 101
==== kiln-workspace --test acceptance_t002_r2: exit status 101
==== kiln-workspace --test acceptance_t008: exit status 101
==== kiln-workspace --test acceptance_t015_tools: exit status 101
==== kiln-workspace --test acceptance_t019_workspace: exit status 101
==== kiln-workspace --test acceptance_t036_workspace: exit status 101
==== kiln-workspace --test acceptance_t073: exit status 0
==== kiln-workspace --test confinement: exit status 101
==== kiln-workspace --test guard_t008: exit status 101
==== kiln-workspace --test guard_t036_removal_workspace: exit status 101
==== kiln-workspace --test guard_t036_workspace: exit status 101
==== kiln-workspace --test guard_t073: exit status 0
==== kiln-workspace --test guard_t074: exit status 0
==== kiln-workspace --test probe_t092: exit status 0
==== kiln-workspace --test project: exit status 101
==== kiln-workspace --test tools_contract: exit status 0
==== kiln-commands --lib: exit status 101
==== kiln-commands --doc: exit status 0
==== kiln-commands --test acceptance_t003: exit status 0
==== kiln-commands --test acceptance_t003_registry: exit status 0
==== kiln-commands --test acceptance_t010_api: exit status 0
==== kiln-commands --test acceptance_t015: exit status 0
==== kiln-commands --test acceptance_t015_layers: exit status 0
==== kiln-commands --test acceptance_t032: exit status 0
==== kiln-commands --test acceptance_t032_json: exit status 0
==== kiln-commands --test acceptance_t037_commands: exit status 101
==== kiln-commands --test acceptance_t038_commands: exit status 0
==== kiln-commands --test guard_t003: exit status 0
==== kiln-commands --test guard_t004: exit status 0
==== kiln-commands --test guard_t006: exit status 0
==== kiln-commands --test guard_t015: exit status 0
==== kiln-commands --test guard_t032: exit status 0
==== kiln-commands --test guard_t038_commands: exit status 0
==== kiln-commands --test positions: exit status 0
==== kiln-commands --test reasons: exit status 0
==== kiln-commands --test scaffold: exit status 0
==== kiln-control --lib: exit status 0
==== kiln-control --doc: exit status 0
==== kiln-control --test acceptance_t004: exit status 0
==== kiln-control --test acceptance_t032_control: exit status 0
==== kiln-ui --lib: exit status 0
==== kiln-ui --doc: exit status 0
==== kiln-testkit --lib: exit status 0
==== kiln-testkit --doc: exit status 0
==== kiln-cli --bins: exit status 0
==== kiln-cli --test acceptance_t003_cli: exit status 0
==== kiln-cli --test acceptance_t004: exit status 0
==== kiln-cli --test acceptance_t004_r2: exit status 101
==== kiln-cli --test acceptance_t004_r3: exit status 101
==== kiln-cli --test acceptance_t008_cli: exit status 101
==== kiln-cli --test acceptance_t010: exit status 0
==== kiln-cli --test acceptance_t011: exit status 0
==== kiln-cli --test acceptance_t018: exit status 0
==== kiln-cli --test acceptance_t019_cli: exit status 101
==== kiln-cli --test acceptance_t020_cli: exit status 0
==== kiln-cli --test acceptance_t021_cli: exit status 0
==== kiln-cli --test acceptance_t023_cli: exit status 101
==== kiln-cli --test acceptance_t032_cli: exit status 0
==== kiln-cli --test acceptance_t036_cli: exit status 101
==== kiln-cli --test acceptance_t037_cli: exit status 101
==== kiln-cli --test acceptance_t037_removal_cli: exit status 101
==== kiln-cli --test acceptance_t038_cli: exit status 0
==== kiln-cli --test acceptance_t039_cli: exit status 0
==== kiln-cli --test cli: exit status 0
==== kiln-cli --test guard_t001: exit status 0
==== kiln-cli --test guard_t004: exit status 0
==== kiln-cli --test guard_t010: exit status 0
==== kiln-cli --test guard_t011: exit status 0
==== kiln-cli --test guard_t015_cli: exit status 101
==== kiln-cli --test guard_t018: exit status 0
==== kiln-cli --test guard_t019_cli: exit status 101
==== kiln-cli --test guard_t032_cli: exit status 101
==== kiln-cli --test guard_t036_cli: exit status 101
==== kiln-cli --test guard_t036_removal_cli: exit status 101
==== kiln-cli --test guard_t037_cli: exit status 101
==== kiln-cli --test guard_t038_cli: exit status 101
==== kiln-cli --test guard_t039_cli: exit status 0
==== kiln-cli --test memory_seed: exit status 0
==== kiln-cli --test run: exit status 0
==== kiln --bins: exit status 0
==== kiln --test acceptance_t055: exit status 101
==== kiln --test acceptance_t056: exit status 0
==== kiln --test acceptance_t057: exit status 0
==== kiln --test acceptance_t058: exit status 0
==== kiln --test acceptance_t059: exit status 0
==== kiln --test acceptance_t065: exit status 101
==== kiln --test editor_t056: exit status 0
==== kiln --test guard_t055: exit status 0
==== kiln --test probe_t092: exit status 0
==== kiln --test project_t057: exit status 0
==== kiln --test script_t055: exit status 101
==== kiln --test status_t059: exit status 0
==== kiln --test tabs_t058: exit status 0
==== kiln --test window_t065: exit status 101
```

## Final summary line, verbatim (job log)
```
PROBE TESTS: exit status per target: kiln-workspace:--lib=0 kiln-workspace:--doc=0 kiln-workspace:--test=acceptance_t002=101 kiln-workspace:--test=acceptance_t002_r2=101 kiln-workspace:--test=acceptance_t008=101 kiln-workspace:--test=acceptance_t015_tools=101 kiln-workspace:--test=acceptance_t019_workspace=101 kiln-workspace:--test=acceptance_t036_workspace=101 kiln-workspace:--test=acceptance_t073=0 kiln-workspace:--test=confinement=101 kiln-workspace:--test=guard_t008=101 kiln-workspace:--test=guard_t036_removal_workspace=101 kiln-workspace:--test=guard_t036_workspace=101 kiln-workspace:--test=guard_t073=0 kiln-workspace:--test=guard_t074=0 kiln-workspace:--test=probe_t092=0 kiln-workspace:--test=project=101 kiln-workspace:--test=tools_contract=0 kiln-commands:--lib=101 kiln-commands:--doc=0 kiln-commands:--test=acceptance_t003=0 kiln-commands:--test=acceptance_t003_registry=0 kiln-commands:--test=acceptance_t010_api=0 kiln-commands:--test=acceptance_t015=0 kiln-commands:--test=acceptance_t015_layers=0 kiln-commands:--test=acceptance_t032=0 kiln-commands:--test=acceptance_t032_json=0 kiln-commands:--test=acceptance_t037_commands=101 kiln-commands:--test=acceptance_t038_commands=0 kiln-commands:--test=guard_t003=0 kiln-commands:--test=guard_t004=0 kiln-commands:--test=guard_t006=0 kiln-commands:--test=guard_t015=0 kiln-commands:--test=guard_t032=0 kiln-commands:--test=guard_t038_commands=0 kiln-commands:--test=positions=0 kiln-commands:--test=reasons=0 kiln-commands:--test=scaffold=0 kiln-control:--lib=0 kiln-control:--doc=0 kiln-control:--test=acceptance_t004=0 kiln-control:--test=acceptance_t032_control=0 kiln-ui:--lib=0 kiln-ui:--doc=0 kiln-testkit:--lib=0 kiln-testkit:--doc=0 kiln-cli:--bins=0 kiln-cli:--test=acceptance_t003_cli=0 kiln-cli:--test=acceptance_t004=0 kiln-cli:--test=acceptance_t004_r2=101 kiln-cli:--test=acceptance_t004_r3=101 kiln-cli:--test=acceptance_t008_cli=101 kiln-cli:--test=acceptance_t010=0 kiln-cli:--test=acceptance_t011=0 kiln-cli:--test=acceptance_t018=0 kiln-cli:--test=acceptance_t019_cli=101 kiln-cli:--test=acceptance_t020_cli=0 kiln-cli:--test=acceptance_t021_cli=0 kiln-cli:--test=acceptance_t023_cli=101 kiln-cli:--test=acceptance_t032_cli=0 kiln-cli:--test=acceptance_t036_cli=101 kiln-cli:--test=acceptance_t037_cli=101 kiln-cli:--test=acceptance_t037_removal_cli=101 kiln-cli:--test=acceptance_t038_cli=0 kiln-cli:--test=acceptance_t039_cli=0 kiln-cli:--test=cli=0 kiln-cli:--test=guard_t001=0 kiln-cli:--test=guard_t004=0 kiln-cli:--test=guard_t010=0 kiln-cli:--test=guard_t011=0 kiln-cli:--test=guard_t015_cli=101 kiln-cli:--test=guard_t018=0 kiln-cli:--test=guard_t019_cli=101 kiln-cli:--test=guard_t032_cli=101 kiln-cli:--test=guard_t036_cli=101 kiln-cli:--test=guard_t036_removal_cli=101 kiln-cli:--test=guard_t037_cli=101 kiln-cli:--test=guard_t038_cli=101 kiln-cli:--test=guard_t039_cli=0 kiln-cli:--test=memory_seed=0 kiln-cli:--test=run=0 kiln:--bins=0 kiln:--test=acceptance_t055=101 kiln:--test=acceptance_t056=0 kiln:--test=acceptance_t057=0 kiln:--test=acceptance_t058=0 kiln:--test=acceptance_t059=0 kiln:--test=acceptance_t065=101 kiln:--test=editor_t056=0 kiln:--test=guard_t055=0 kiln:--test=probe_t092=0 kiln:--test=project_t057=0 kiln:--test=script_t055=101 kiln:--test=status_t059=0 kiln:--test=tabs_t058=0 kiln:--test=window_t065=101
```

## Targets with a non-zero status
### Compile and fail (6)
- `kiln-workspace --test acceptance_t002`
  test disk_save_replaces_via_rename_without_leftovers ... FAILED
  panicked line: thread 'disk_save_replaces_via_rename_without_leftovers' (8444) panicked at crates\kiln-workspace\tests\acceptance_t002.rs:940:5:
  next line: precondition: permission bits and inode numbers are Unix metadata; the Windows checks of atomic replace are not written yet (ADR-0009: T-073 and T-074 write the Windows behaviour and its tests)
  test result: FAILED. 14 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s
- `kiln-workspace --test acceptance_t015_tools`
  test no_tools_never_runs ... FAILED
  panicked line: thread 'no_tools_never_runs' (8592) panicked at crates\kiln-workspace\tests\acceptance_t015_tools.rs:131:5:
  next line: precondition: the fake `cargo`/`rustfmt` of this test are shell scripts and need Unix
  test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
- `kiln-cli --test acceptance_t004_r2`
  test huge_values_are_echoed_truncated_to_200_chars ... FAILED
  panicked line: thread 'huge_values_are_echoed_truncated_to_200_chars' (7576) panicked at apps\kiln-cli\tests\acceptance_t004_r2.rs:48:56:
  next line: spawn kiln-cli: Os { code: 206, kind: InvalidFilename, message: "The filename or extension is too long." }
  test result: FAILED. 4 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
- `kiln-cli --test acceptance_t004_r3`
  test exec_errors_on_stderr_obey_size_caps ... FAILED
  panicked line: thread 'exec_errors_on_stderr_obey_size_caps' (1100) panicked at apps\kiln-cli\tests\acceptance_t004_r3.rs:58:31:
  next line: spawn kiln-cli: Os { code: 206, kind: InvalidFilename, message: "The filename or extension is too long." }
  test result: FAILED. 4 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.44s
- `kiln-cli --test guard_t015_cli`
  test cli_output_is_byte_identical_to_base ... FAILED
  panicked line: thread 'cli_output_is_byte_identical_to_base' (4184) panicked at apps\kiln-cli\tests\guard_t015_cli.rs:132:9:
  next line: session_run_files differs from the base recording (actual saved to D:\a\photocraft-study-review\photocraft-study-review\target\tmp\guard_t015_cli\session_run_files.actual):
  test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.56s
- `kiln-cli --test guard_t032_cli`
  test protocol_bytes_are_unchanged ... FAILED
  panicked line: thread 'protocol_bytes_are_unchanged' (6268) panicked at apps\kiln-cli\tests\guard_t032_cli.rs:134:9:
  next line: session_run_files differs from the base recording (actual saved to D:\a\photocraft-study-review\photocraft-study-review\target\tmp\guard_t032_cli\session_run_files.actual):
  test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.69s

### Do not compile (26): first two `error` lines and the closing line of each
- `kiln-workspace --test acceptance_t002_r2`
      error[E0433]: cannot find `unix` in `os`
      error[E0433]: cannot find `unix` in `os`
      error: could not compile `kiln-workspace` (test "acceptance_t002_r2") due to 11 previous errors
- `kiln-workspace --test acceptance_t008`
      error[E0433]: cannot find `unix` in `os`
      error[E0433]: cannot find `unix` in `os`
      error: could not compile `kiln-workspace` (test "acceptance_t008") due to 20 previous errors
- `kiln-workspace --test acceptance_t019_workspace`
      error[E0433]: cannot find `unix` in `os`
      error[E0433]: cannot find `unix` in `os`
      error: could not compile `kiln-workspace` (test "acceptance_t019_workspace") due to 5 previous errors
- `kiln-workspace --test acceptance_t036_workspace`
      error[E0433]: cannot find `unix` in `os`
      error[E0433]: cannot find `unix` in `os`
      error: could not compile `kiln-workspace` (test "acceptance_t036_workspace") due to 16 previous errors
- `kiln-workspace --test confinement`
      error[E0433]: cannot find `unix` in `os`
      error[E0433]: cannot find `unix` in `os`
      error: could not compile `kiln-workspace` (test "confinement") due to 24 previous errors
- `kiln-workspace --test guard_t008`
      error[E0433]: cannot find `unix` in `os`
      error: could not compile `kiln-workspace` (test "guard_t008") due to 1 previous error
- `kiln-workspace --test guard_t036_removal_workspace`
      error[E0433]: cannot find `unix` in `os`
      error[E0599]: no method named `dev` found for struct `Metadata` in the current scope
      error: could not compile `kiln-workspace` (test "guard_t036_removal_workspace") due to 3 previous errors
- `kiln-workspace --test guard_t036_workspace`
      error[E0433]: cannot find `unix` in `os`
      error: could not compile `kiln-workspace` (test "guard_t036_workspace") due to 1 previous error
- `kiln-workspace --test project`
      error[E0433]: cannot find `unix` in `os`
      error[E0433]: cannot find `unix` in `os`
      error: could not compile `kiln-workspace` (test "project") due to 3 previous errors
- `kiln-commands --lib`
      error[E0433]: cannot find `unix` in `os`
      error[E0433]: cannot find `unix` in `os`
      error: could not compile `kiln-commands` (lib test) due to 4 previous errors
- `kiln-commands --test acceptance_t037_commands`
      error[E0433]: cannot find `unix` in `os`
      error[E0599]: no method named `dev` found for struct `Metadata` in the current scope
      error: could not compile `kiln-commands` (test "acceptance_t037_commands") due to 3 previous errors
- `kiln-cli --test acceptance_t008_cli`
      error[E0433]: cannot find `unix` in `os`
      error[E0433]: cannot find `unix` in `os`
      error: could not compile `kiln-cli` (test "acceptance_t008_cli") due to 15 previous errors
- `kiln-cli --test acceptance_t019_cli`
      error[E0433]: cannot find `unix` in `os`
      error[E0599]: no method named `is_fifo` found for struct `FileType` in the current scope
      error: could not compile `kiln-cli` (test "acceptance_t019_cli") due to 3 previous errors
- `kiln-cli --test acceptance_t023_cli`
      error[E0433]: cannot find `unix` in `os`
      error: could not compile `kiln-cli` (test "acceptance_t023_cli") due to 1 previous error
- `kiln-cli --test acceptance_t036_cli`
      error[E0433]: cannot find `unix` in `os`
      error[E0433]: cannot find `unix` in `os`
      error: could not compile `kiln-cli` (test "acceptance_t036_cli") due to 15 previous errors
- `kiln-cli --test acceptance_t037_cli`
      error[E0433]: cannot find `unix` in `os`
      error[E0433]: cannot find `unix` in `os`
      error: could not compile `kiln-cli` (test "acceptance_t037_cli") due to 16 previous errors
- `kiln-cli --test acceptance_t037_removal_cli`
      error[E0433]: cannot find `unix` in `os`
      error[E0599]: no associated function or constant named `from_mode` found for struct `Permissions` in the current scope
      error: could not compile `kiln-cli` (test "acceptance_t037_removal_cli") due to 5 previous errors
- `kiln-cli --test guard_t019_cli`
      error[E0433]: cannot find `unix` in `os`
      error[E0599]: no method named `is_fifo` found for struct `FileType` in the current scope
      error: could not compile `kiln-cli` (test "guard_t019_cli") due to 2 previous errors
- `kiln-cli --test guard_t036_cli`
      error[E0433]: cannot find `unix` in `os`
      error: could not compile `kiln-cli` (test "guard_t036_cli") due to 1 previous error
- `kiln-cli --test guard_t036_removal_cli`
      error[E0433]: cannot find `unix` in `os`
      error[E0433]: cannot find `unix` in `os`
      error: could not compile `kiln-cli` (test "guard_t036_removal_cli") due to 17 previous errors
- `kiln-cli --test guard_t037_cli`
      error[E0433]: cannot find `unix` in `os`
      error[E0433]: cannot find `unix` in `os`
      error: could not compile `kiln-cli` (test "guard_t037_cli") due to 16 previous errors
- `kiln-cli --test guard_t038_cli`
      error[E0433]: cannot find `unix` in `os`
      error[E0425]: cannot find value `ELOOP` in this scope
      error: could not compile `kiln-cli` (test "guard_t038_cli") due to 3 previous errors
- `kiln --test acceptance_t055`
      error[E0433]: cannot find `unix` in `os`
      error[E0433]: cannot find `unix` in `os`
      error: could not compile `kiln` (test "acceptance_t055") due to 2 previous errors
- `kiln --test acceptance_t065`
      error[E0433]: cannot find `unix` in `os`
      error[E0599]: no associated function or constant named `from_bytes` found for struct `OsStr` in the current scope
      error: could not compile `kiln` (test "acceptance_t065") due to 3 previous errors
- `kiln --test script_t055`
      error[E0433]: cannot find `unix` in `os`
      error: could not compile `kiln` (test "script_t055") due to 1 previous error
- `kiln --test window_t065`
      error[E0433]: cannot find `unix` in `os`
      error[E0599]: no associated function or constant named `from_vec` found for struct `OsString` in the current scope
      error: could not compile `kiln` (test "window_t065") due to 8 previous errors

## `MEASURED:` lines, every one, verbatim, in the file's order (207)
Blocks: lines 1-2 are from `guard_t073`, then 176 from `guard_t074`, then 25 from `kiln-workspace` `probe_t092`, then 4 from `kiln` `probe_t092` (per-target counts above).
```
MEASURED: acw2_root_parent_replaced_is_outside_root: windows: rename of p while the root is held: refused: kind PermissionDenied, raw OS error Some(5) (Access is denied. (os error 5))
MEASURED: acw2_root_replaced_at_same_path_is_outside_root: windows: rename of root while the root is held: ok
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
MEASURED: awa3_temp_name_held_by_a_dangling_file_link_to_outside: windows: fixture: FileAbs made at .kiln-tmp-0 -> D:\a\photocraft-study-review\photocraft-study-review\target\tmp\guard_t074\awa3_temp_name_held_by_a_dangling_file_link_to_outside-8188\out\missing (target exists: false)
MEASURED: awa3_temp_name_held_by_a_dangling_file_link_to_outside: windows: write_atomic(f.txt) beside the links: Ok(())
MEASURED: awa3_temp_name_held_by_a_dangling_file_link_to_outside: windows: Document save of f.txt beside the links: Ok(())
MEASURED: awa3_temp_name_held_by_a_dangling_junction: windows: fixture: Junction made at .kiln-tmp-0 -> D:\a\photocraft-study-review\photocraft-study-review\target\tmp\guard_t074\awa3_temp_name_held_by_a_dangling_junction-8188\out\gone (target exists: false)
MEASURED: awa3_temp_name_held_by_a_dangling_junction: windows: write_atomic(f.txt) beside the links: Ok(())
MEASURED: awa3_temp_name_held_by_a_dangling_junction: windows: Document save of f.txt beside the links: Ok(())
MEASURED: awa3_temp_name_held_by_a_file_link_to_an_outside_file: windows: fixture: FileAbs made at .kiln-tmp-0 -> D:\a\photocraft-study-review\photocraft-study-review\target\tmp\guard_t074\awa3_temp_name_held_by_a_file_link_to_an_outside_file-8188\out\t.txt (target exists: true)
MEASURED: awa3_temp_name_held_by_a_file_link_to_an_outside_file: windows: write_atomic(f.txt) beside the links: Ok(())
MEASURED: awa3_temp_name_held_by_a_file_link_to_an_outside_file: windows: Document save of f.txt beside the links: Ok(())
MEASURED: awa3_temp_name_held_by_a_junction: windows: fixture: Junction made at .kiln-tmp-0 -> D:\a\photocraft-study-review\photocraft-study-review\target\tmp\guard_t074\awa3_temp_name_held_by_a_junction-8188\out (target exists: true)
MEASURED: awa3_temp_name_held_by_a_junction: windows: write_atomic(f.txt) beside the links: Ok(())
MEASURED: awa3_temp_name_held_by_a_junction: windows: Document save of f.txt beside the links: Ok(())
MEASURED: awa3_temp_names_held_by_a_junction_and_a_dangling_file_link: windows: fixture: Junction made at .kiln-tmp-0 -> D:\a\photocraft-study-review\photocraft-study-review\target\tmp\guard_t074\awa3_temp_names_held_by_a_junction_and_a_dangling_file_link-8188\out (target exists: true)
MEASURED: awa3_temp_names_held_by_a_junction_and_a_dangling_file_link: windows: fixture: FileAbs made at .kiln-tmp-1 -> D:\a\photocraft-study-review\photocraft-study-review\target\tmp\guard_t074\awa3_temp_names_held_by_a_junction_and_a_dangling_file_link-8188\out\t (target exists: false)
MEASURED: awa3_temp_names_held_by_a_junction_and_a_dangling_file_link: windows: write_atomic(f.txt) beside the links: Ok(())
MEASURED: awa3_temp_names_held_by_a_junction_and_a_dangling_file_link: windows: Document save of f.txt beside the links: Ok(())
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
MEASURED: awa4_dotdot_after_a_junction_in_the_root_text: --root k/link/../b: windows: read f.txt: Ok
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
MEASURED: awa4_root_given_through_a_directory_link_and_retargeted: windows: DirSym as --root: read f.txt: Ok([114, 101, 97, 108, 10])
MEASURED: awa4_root_given_through_a_directory_link_and_retargeted: DirSym retargeted to another directory: windows: read OutsideRoot, write_atomic OutsideRoot, is_file false, list_dir OutsideRoot, check_location OutsideRoot, save OutsideRoot
MEASURED: awa4_root_given_through_a_directory_link_and_retargeted: DirSym pointed back: windows: read f.txt: Ok
MEASURED: awa4_root_removed_while_held: windows: remove_dir of the held empty root: Ok(())
MEASURED: awa4_root_removed_while_held: empty root removed: windows: read OutsideRoot, write_atomic OutsideRoot, is_file false, list_dir OutsideRoot, check_location OutsideRoot, save -
MEASURED: awa4_root_removed_while_held: empty root removed, a new directory at its name: windows: read OutsideRoot, write_atomic OutsideRoot, is_file false, list_dir OutsideRoot, check_location OutsideRoot, save -
MEASURED: awa4_root_removed_while_held: windows: remove_dir_all of the held root with a file: Ok(())
MEASURED: awa4_root_removed_while_held: root with a file removed: windows: read OutsideRoot, write_atomic OutsideRoot, is_file false, list_dir OutsideRoot, check_location OutsideRoot, save OutsideRoot
MEASURED: awa4_root_removed_while_held: root removed, a junction to a new directory at its name: windows: read OutsideRoot, write_atomic OutsideRoot, is_file false, list_dir OutsideRoot, check_location OutsideRoot, save OutsideRoot
MEASURED: awa4_root_replaced_by_a_junction: windows: rename of the held root: Ok(())
MEASURED: awa4_root_replaced_by_a_junction: a junction to another directory at the root's path: windows: read OutsideRoot, write_atomic OutsideRoot, is_file false, list_dir OutsideRoot, check_location OutsideRoot, save OutsideRoot
MEASURED: awa4_root_replaced_by_a_junction: a junction at the root's path to the held root: windows: read f.txt: Ok
MEASURED: awa4_root_given_through_a_junction_and_retargeted: windows: Junction as --root: read f.txt: Ok([114, 101, 97, 108, 10])
MEASURED: awa4_root_given_through_a_junction_and_retargeted: Junction retargeted to another directory: windows: read OutsideRoot, write_atomic OutsideRoot, is_file false, list_dir OutsideRoot, check_location OutsideRoot, save OutsideRoot
MEASURED: awa4_root_given_through_a_junction_and_retargeted: Junction pointed back: windows: read f.txt: Ok
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
MEASURED: P-A1 create "b\u{d800}.rs": Ok(())
MEASURED: P-A2 read_dir names: ["\"b\\u{d800}.rs\"", "\"ok.rs\""]
MEASURED: P-A3 read back by the same OsString: Ok([98])
MEASURED: P-A4 DiskFs::new: Ok(())
MEASURED: P-A5 list_dir(None): Ok(DirListing { entries: [DirEntry { name: "ok.rs", kind: File }], skipped: 1 })
MEASURED: P-A6 project::tree: Ok(ProjectTree { entries: [TreeEntry { path: RelPath("ok.rs"), kind: File }], ignored: 0, skipped: 1, too_deep: 0, truncated: false })
MEASURED: P-B1 set_readonly: Ok(())
MEASURED: P-B2 readonly before: Ok(true)
MEASURED: P-B3 first save: Ok(())
MEASURED: P-B4 second save: Ok(())
MEASURED: P-B5 a: Ok([110, 101, 119, 101, 114])
MEASURED: P-B6 leftover bytes: Ok("leftover from a crash, long")
MEASURED: P-B7 readonly after: Ok(true)
MEASURED: P-B8 listing: ["\".kiln-tmp-0\"", "\"a\""]
MEASURED: P-C1 save with all names taken: Err(Io { path: RelPath("f"), kind: AlreadyExists, message: "all 100 temporary file names are taken" })
MEASURED: P-C2 listing unchanged: true
MEASURED: P-C3 f: Ok([111, 114, 105, 103, 105, 110, 97, 108, 32, 99, 111, 110, 116, 101, 110, 116])
MEASURED: P-C4 f readonly: Ok(false)
MEASURED: P-D1 cargo with only cargo.cmd on PATH: status Some(0), stdout "cargo 1.97.0 (c980f4866 2026-06-30)", stderr "", 37.1548ms
MEASURED: P-D1m marker: Err(Os { code: 2, kind: NotFound, message: "The system cannot find the file specified." })
MEASURED: P-D2 rustfmt with only rustfmt.bat on PATH: status Some(0), stdout "rustfmt 1.9.0-stable (2d8144b788 2026-07-07)", stderr "", 586.3387ms
MEASURED: P-D2m marker: Err(Os { code: 2, kind: NotFound, message: "The system cannot find the file specified." })
MEASURED: P-D3 rustc builds the fake: status Some(0), stdout "", stderr "", 211.9272ms
MEASURED: P-D4 cargo with cargo.exe and cargo.cmd on PATH: status Some(0), stdout "", stderr "", 10.5245ms
MEASURED: P-D4m marker: Ok("check")
MEASURED: P-E0 kiln ["--root", "d", "caf\u{d800}.rs"]: status Some(2), stdout 0 bytes, stderr 24 lines, first "kiln: argument 3 is not valid UTF-8", panicked false
MEASURED: P-E1 kiln ["caf\u{d800}.rs"]: status Some(2), stdout 0 bytes, stderr 24 lines, first "kiln: argument 1 is not valid UTF-8", panicked false
MEASURED: P-E2 kiln ["script", "s.jsonl", "--root", "d", "caf\u{d800}.rs"]: status Some(2), stdout 0 bytes, stderr 24 lines, first "kiln: argument 5 is not valid UTF-8", panicked false
MEASURED: P-E3 kiln ["script", "s.jsonl", "--root", "caf\u{d800}.rs"]: status Some(2), stdout 0 bytes, stderr 24 lines, first "kiln: argument 4 is not valid UTF-8", panicked false
```

## Not read
- The rest of `probe-tests.txt` outside the lines named above (test lines of passing tests, `---- ` blocks of passing tests, compile errors beyond the first two per target); the job log beyond the header, the summary line and its tail.
- Other jobs' artefacts and logs of this run (not opened).
- Why the 26 targets do not compile is stated only by their first two `error` lines; the sources were not read.
