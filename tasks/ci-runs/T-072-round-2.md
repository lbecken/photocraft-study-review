# CI run 44315616697 (platforms.yml) on ci/candidate @ d87f2e8e66dd3b6f14b689fada4f33d0341b8c68 — T-072 round 2

Record by hand; method as `tasks/ci-runs/README.md` (artefacts first; job logs for the image header, the cache step, the log-only lines and the disk step). Every value below comes from this run's job listing, its artefacts or its job logs. No judgement of results is made here; the reviewer compares them with the expectation written beforehand.
**Complete: the run had `status: completed` when first read (14:13Z); no waiting was needed. Nothing was still running at the end.**

## Run header
- id 44315616697, run_number 30, attempt 1, event push, workflow `platforms`, title "T-072: implementer log: gate attempt 3 after D-258"
- branch `ci/candidate`; commit `d87f2e8e66dd3b6f14b689fada4f33d0341b8c68` (equals the head asked for); created_at 2026-11-18T13:37:36Z; updated_at 2026-11-18T14:03:51Z
- status `completed`, run conclusion **failure** (cause by job: `gate-windows` failure; `window-probe (windows-2025)` failure; `window-probe (macos-15)` cancelled)

## Job table
Wall time = `completed_at` − `started_at` of the job (job listing).
| job | id | runner label | conclusion | wall | first failing step |
|---|---|---|---|---|---|
| gate-linux | 134840796984 | ubuntu-24.04 | success | 16m02s (13:37:39Z–13:53:41Z) | none |
| gate-macos | 134840796330 | macos-15 | success | 26m09s (13:37:41Z–14:03:50Z) | none |
| gate-windows | 134840796910 | windows-2025 | **failure** | 24m19s (13:37:39Z–14:01:58Z) | Gate (13:38:12Z–13:45:45Z); every later step success or skipped |
| window-linux | 134840797071 | ubuntu-24.04 | success | 52s (13:37:38Z–13:38:30Z) | none |
| window-probe (macos-15) | 134840797577 | macos-15 | **cancelled** | 15m01s (13:37:37Z–13:52:38Z) | no steps listed; `runner_id` 0, empty `runner_name`: no runner was assigned (job data, `get_workflow_job`) |
| window-probe (windows-2025) | 134840797202 | windows-2025 | **failure** | 2m17s (13:37:39Z–13:39:56Z) | Window probe (13:39:50Z–13:39:51Z); Build kiln success |

Artefacts (`list_workflow_run_artifacts`; the three gate artefacts fetched with `download_workflow_run_artifact` and curl, size equal to `size_in_bytes`, `testzip()` clean; the scratch directory is removed after this record):
`gate-linux-44315616697` 31,380 B (id 14433042731); `gate-macos-44315616697` 31,102 B (14433678388); `gate-windows-44315616697` 24,603 B (14433158381); `window-linux-44315616697` 577 B; `window-probe-windows-2025-44315616697` 344 B (the last two listed, not downloaded). **No `window-probe-macos-15-…` artefact exists** (5 artefacts in all).
Job logs (`get_job_logs`, then the logs URL fetched with curl) were read for the three gate jobs.

## Item 2: gate jobs, common facts

### gate-linux (job 134840796984)
- Runner image (job log "Runner Image"): `Image: ubuntu-24.04`, `Version: 20261111.127.1`; runner version 2.338.2.
- Rust versions listed by the cache step ("Rust Versions"): `1.97.0 x86_64-unknown-linux-gnu 2d8144b7880597b6e6d3dfd63a9a9efae3f533d3` and `1.99.1 x86_64-unknown-linux-gnu 58cf8f9c35b355199727627d07ae34a4e2846d67`. The toolchain step printed `cargo 1.97.0 (c980f4866 2026-06-30)`, `clippy 0.1.97`, `rustfmt 1.9.0-stable`. Cache: `Cache hit … full match: true` (~358 MB).
- Steps (job listing):
| step | conclusion | start–end (Z) | duration |
|---|---|---|---|
| Set up job | success | 13:37:39–13:37:41 | 2s |
| Checkout | success | 13:37:41–13:37:46 | 5s |
| Install the pinned toolchain (rust-toolchain.toml) | success | 13:37:46–13:37:54 | 8s |
| Cache cargo registry and target | success | 13:37:54–13:38:00 | 6s |
| Gate | success | 13:38:00–13:48:34 | 10m34s |
| Gate report | success | 13:48:34–13:48:34 | 0s |
| Core crates (reported, not required) | success | 13:48:34–13:53:37 | 5m03s |
| Test list | success | 13:53:37–13:53:38 | 1s |
| Full test output (gate failed) | **skipped** | 13:53:38–13:53:38 | 0s |
| Disk | success | 13:53:38–13:53:38 | 0s |
| Upload reports | success | 13:53:38–13:53:39 | 1s |
| Post Cache, Post Checkout, Complete job | success, success, success | 13:53:39 | 0s each |
- Gate report (`gate-report.json`, 1,986 B): `passed: true`, `failed_step: null`, 630.4 s.
| step | status | seconds | detail |
|---|---|---|---|
| fmt | pass | 0.9 | formatted |
| clippy | pass | 9.7 | no warnings (all targets, -D warnings) |
| layers | pass | 0.4 | 11 crates, 14 internal edges, layer table holds |
| ui-commands | pass | 0.03 | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | pass | 0.06 | reviews: 59 of 71 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 694 tracked file(s) clean (0 not UTF-8, skipped) |
| schema | pass | 4.5 | 30 commands; audit ok; docs/commands.json current |
| scorecard | pass | 152.9 | 64 items: 47 verified, 0 implemented, 0 failing, 17 planned. |
| test | pass | 459.7 | 1171 passed, 0 failed, 0 ignored in 137 suites |
| no-skip | pass | 2.2 | self-test ok (17 known findings); 203 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (19 pending), today = 2026-11-18 |
| fuzz, budgets | empty | 0.0 | no targets registered yet (xtask/fuzz.toml, xtask/budgets.toml) |
- "Test list": 1 s. Log lines, verbatim: `TEST LIST: exit status 0` / the line after it: `TEST LIST: the status is recorded, not used; the gate step decides the colour`.
- `test-list.txt` (76,556 B, 1,567 lines): 136 binary lines (128 `Running`, 8 `Doc-tests`), 1,170 lines ending `: test` (test names). Commands: `grep -c Running`, `grep -c Doc-tests`, `grep -c ': test$'` on the ANSI-stripped file.
- "Full test output (gate failed)": **skipped** (gate green). So no `TEST OUTPUT:` line, nothing printed, no `test-output.txt`.
- Artefact files: `gate-report.json` 1,986 B; `core-crates.txt` 39,955 B; `test-list.txt` 76,556 B. Zip 31,380 B.
- Disk (job log): `Filesystem            Size  Used Avail Use% Mounted on`, `/dev/mapper/cow-root  145G   62G   83G  43% /`; `DISK: du -sh target` → `4.5G	target`.
- `core-crates.txt` labels: `==== cargo test --locked --no-fail-fast -p kiln-text|kiln-syntax|kiln-search|xtask`; each ends `exit status 0` (`==== kiln-text: exit status 0`, `kiln-syntax`, `kiln-search`, `xtask`). Totals by package (sum of `test result:` lines): kiln-text 78 passed, kiln-syntax 53, kiln-search 55, xtask 311; 0 failed in all.

### gate-macos (job 134840796330)
- Runner image: `Image: macos-15-arm64`, `Version: 20261104.0514.1`; runner version 2.338.2.
- Rust versions listed by the cache step: `1.97.0 aarch64-apple-darwin 2d8144b7880597b6e6d3dfd63a9a9efae3f533d3` and `1.99.1 aarch64-apple-darwin 58cf8f9c35b355199727627d07ae34a4e2846d67`. Toolchain step printed `cargo 1.97.0 (c980f4866 2026-06-30)`. Cache: `Cache hit … full match: true` (~349 MB).
- Steps: see the "Item 5" section (full timings).
- Gate report (`gate-report.json`, 2,018 B): `passed: true`, `failed_step: null`, 1008.9 s.
| step | status | seconds | detail |
|---|---|---|---|
| fmt | pass | 1.0 | formatted |
| clippy | pass | 12.5 | no warnings (all targets, -D warnings) |
| layers | pass | 1.1 | 11 crates, 14 internal edges, layer table holds |
| ui-commands | pass | 0.03 | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | pass | 0.2 | reviews: 59 of 71 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 694 tracked file(s) clean (0 not UTF-8, skipped) |
| schema | pass | 6.1 | 30 commands; audit ok; docs/commands.json current |
| scorecard | pass | 194.3 | macos: 64 items; 216 tests ran; 3 registered absent here; page not compared (computed on Linux) |
| test | pass | 789.9 | 1154 passed, 0 failed, 0 ignored in 137 suites |
| no-skip | pass | 3.7 | self-test ok (17 known findings); 203 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (19 pending), today = 2026-11-18 |
| fuzz, budgets | empty | 0.0 | no targets registered yet |
- "Test list": 4 s. Log lines, verbatim: `TEST LIST: exit status 0` / the line after it: `TEST LIST: the status is recorded, not used; the gate step decides the colour`.
- `test-list.txt` (75,598 B, 1,550 lines): 136 binary lines (128 `Running`, 8 `Doc-tests`), 1,153 lines ending `: test`.
- "Full test output (gate failed)": **skipped**. No `TEST OUTPUT:` line, no `test-output.txt`.
- Artefact files: `gate-report.json` 2,018 B; `core-crates.txt` 39,721 B; `test-list.txt` 75,598 B. Zip 31,102 B.
- Disk (job log): `Filesystem      Size    Used   Avail Capacity iused ifree %iused  Mounted on`, `/dev/disk2s5   320Gi   260Gi    42Gi    87%    2.3M  436M    1%   /System/Volumes/Data`; `3.2G	target`.
- `core-crates.txt`: all four packages `exit status 0`. kiln-text 78, kiln-syntax 53, kiln-search 55, xtask 306 passed; 0 failed.

### gate-windows (job 134840796910)
- Runner image: `Image: windows-2025-vs2026`, `Version: 20261103.298.1`; runner version 2.338.2. Step shell: `C:\Program Files\Git\bin\bash.EXE --noprofile --norc -e -o pipefail {0}`.
- Rust versions listed by the cache step: `1.97.0 x86_64-pc-windows-msvc 2d8144b7880597b6e6d3dfd63a9a9efae3f533d3` and `1.99.1 x86_64-pc-windows-msvc 58cf8f9c35b355199727627d07ae34a4e2846d67`. Toolchain step printed `cargo 1.97.0 (c980f4866 2026-06-30)`. Cache: `No cache found.` (log, after "Restoring cache").
- Steps:
| step | conclusion | start–end (Z) | duration |
|---|---|---|---|
| Set up job | success | 13:37:41–13:37:43 | 2s |
| Checkout | success | 13:37:43–13:37:52 | 9s |
| Install the pinned toolchain (rust-toolchain.toml) | success | 13:37:52–13:38:05 | 13s |
| Cache cargo registry and target | success | 13:38:05–13:38:12 | 7s |
| Gate | **failure** | 13:38:12–13:45:45 | 7m33s |
| Gate report | success | 13:45:45–13:45:45 | 0s |
| Core crates (reported, not required) | success | 13:45:45–14:01:34 | 15m49s |
| Workspace lib and T-073 tests (reported, not required) | success | 14:01:34–14:01:41 | 7s |
| Test list | success | 14:01:41–14:01:42 | 1s |
| kiln-workspace test targets (reported, not required) | success | 14:01:42–14:01:48 | 6s |
| Full test output (gate failed) | success (**ran**) | 14:01:48–14:01:49 | 1s |
| Disk | success | 14:01:49–14:01:52 | 3s |
| Upload reports | success | 14:01:52–14:01:54 | 2s |
| Post Cache | skipped | 14:01:54 | 0s |
| Post Checkout, Complete job | success, success | 14:01:54–14:01:56 | 2s, 0s |
- Gate report (`gate-report.json`, 16,907 B): `passed: false`, `failed_step: clippy`, 429.3 s.
| step | status | seconds | detail |
|---|---|---|---|
| fmt | pass | 2.2 | formatted |
| clippy | **fail** | 59.7 | kept text (60 lines) ends `error: could not compile `kiln-cli` (test "acceptance_t023_cli") due to 1 previous error`; includes `error[E0433]: cannot find `unix` in `os`` at `apps\kiln-cli\tests\acceptance_t023_cli.rs:24:14` and `apps\kiln-cli\tests\guard_t019_cli.rs:12:14`, and `error[E0599]: no method named `is_fifo` found for struct `std::fs::FileType`` (`guard_t019_cli.rs:95:18`); `could not compile `kiln-cli` (test "guard_t019_cli") due to 2 previous errors` |
| layers | pass | 10.5 | 11 crates, 14 internal edges, layer table holds |
| ui-commands | pass | 0.05 | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | pass | 0.2 | reviews: 59 of 71 …; session-paths: 694 tracked file(s) clean |
| schema | pass | 33.7 | 30 commands; audit ok; docs/commands.json current |
| scorecard | **fail** | 316.8 | first line `windows: 64 items; 158 tests ran; 3 registered absent here; page not compared (computed on Linux)`, then `60 scorecard problems:` (first: `building test executables failed on windows (); their tests are not run`) |
| test | **fail** | 1.1 | `0 passed, 0 failed, 0 ignored in 0 suites`; kept text (27 lines) ends `could not compile `kiln` (test "script_t055") due to 1 previous error`, `could not compile `kiln` (test "acceptance_t055") due to 2 previous errors` |
| no-skip | pass | 5.0 | self-test ok (17 known findings); 203 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (19 pending), today = 2026-11-18 |
| fuzz, budgets | empty | 0.0 | no targets registered yet |
- "Test list": 1 s. Log lines, verbatim: `TEST LIST: exit status 101` / the line after it: `TEST LIST: the status is recorded, not used; the gate step decides the colour`.
- `test-list.txt` (10,832 B, 172 lines): compile errors only; **0 binary lines, 0 test names**. Same size as `test-output.txt` but the files are not byte-identical (their sorted lines have the same checksum, so only the order of blocks differs).
- "Full test output (gate failed)": **ran** (1 s). Log lines: `TEST OUTPUT: exit status 101` / `TEST OUTPUT: the status is recorded, not used; the gate step decides the colour`. The printer printed only `error[…]` lines and `error: could not compile` lines (log, verbatim): `error: could not compile `kiln` (test "window_t065") due to 8 previous errors`, `error: could not compile `kiln` (test "acceptance_t065") due to 3 previous errors`, `error: could not compile `kiln` (test "script_t055") due to 1 previous error`, `error: could not compile `kiln` (test "acceptance_t055") due to 2 previous errors`. The `error[…]` lines: `E0433 cannot find `unix` in `os``, `E0599 … `from_vec` found for struct `OsString``, `E0277 the size for values of type `[OsString]` cannot be known at compilation time`, `E0599 … `from_bytes` found for struct `OsStr``. No failing test name and no panic message (nothing compiled to run).
- Artefact files: `gate-report.json` 16,907 B; `core-crates.txt` 40,310 B; `test-output.txt` 10,832 B; `test-list.txt` 10,832 B; `workspace-tests.txt` 7,134 B; `windows-tests.txt` 62,809 B. Zip 24,603 B.
- Disk (job log): `Filesystem      Size  Used Avail Use% Mounted on`, `D:              150G  6.6G  144G   5% /d`; `3.5G	target`.
- `core-crates.txt`: all four packages `exit status 0`; kiln-text 78, kiln-syntax 53, kiln-search 55, xtask 303 passed; 0 failed.

## Item 3: gate-windows, T-073 step and the 14-target step
"Workspace lib and T-073 tests (reported, not required)" (`workspace-tests.txt`; log line `WORKSPACE TESTS: exit status 0 (reported, not required)`): lib `36 passed; 0 failed` (includes `disk::tests::held_root_check_is_inside_only_for_a_read_identity_equal_to_the_held_one ... ok`), `acceptance_t073` `1 passed; 0 failed`, `guard_t073` `10 passed; 0 failed`: 47 passed, 0 failed. Two `MEASURED:` lines, verbatim:
- `MEASURED: acw2_root_parent_replaced_is_outside_root: windows: rename of p while the root is held: refused: kind PermissionDenied, raw OS error Some(5) (Access is denied. (os error 5))`
- `MEASURED: acw2_root_replaced_at_same_path_is_outside_root: windows: rename of root while the root is held: ok`

"kiln-workspace test targets (reported, not required)" (`windows-tests.txt`), every status line verbatim:
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
==== project: exit status 101
==== tools_contract: exit status 0
```
Summary line (job log, verbatim): `WINDOWS TESTS: exit status per target: acceptance_t002=101 acceptance_t002_r2=101 acceptance_t008=101 acceptance_t015_tools=101 acceptance_t019_workspace=101 acceptance_t036_workspace=101 acceptance_t073=0 confinement=101 guard_t008=101 guard_t036_removal_workspace=101 guard_t036_workspace=101 guard_t073=0 project=101 tools_contract=0 (reported, not required)`.

Non-zero targets:
- `acceptance_t002` (compiled, `running 15 tests`, `test result: FAILED. 14 passed; 1 failed`): `test disk_save_replaces_via_rename_without_leftovers ... FAILED`; `thread 'disk_save_replaces_via_rename_without_leftovers' (6084) panicked at crates\kiln-workspace\tests\acceptance_t002.rs:940:5:` / `precondition: permission bits and inode numbers are Unix metadata; the Windows checks of atomic replace are not written yet (ADR-0009: T-073 and T-074 write the Windows behaviour and its tests)`.
- `acceptance_t015_tools` (compiled, `running 3 tests`, `test result: FAILED. 2 passed; 1 failed`): `test no_tools_never_runs ... FAILED`; `thread 'no_tools_never_runs' (2952) panicked at crates\kiln-workspace\tests\acceptance_t015_tools.rs:131:5:` / `precondition: the fake `cargo`/`rustfmt` of this test are shell scripts and need Unix`.
- Compile errors (no test ran; first error line of each, then the closing line):
  - `acceptance_t002_r2`: `error[E0433]: cannot find `unix` in `os`` at `crates\kiln-workspace\tests\acceptance_t002_r2.rs:16:18`; `error: could not compile `kiln-workspace` (test "acceptance_t002_r2") due to 11 previous errors`.
  - `acceptance_t008`: same error at `acceptance_t008.rs:20:14`; `… (test "acceptance_t008") due to 20 previous errors`.
  - `acceptance_t019_workspace`: at `:19:14`; `… due to 5 previous errors`.
  - `acceptance_t036_workspace`: at `:20:14`; `… due to 16 previous errors`.
  - `confinement`: at `:9:14`; `… due to 24 previous errors`.
  - `guard_t008`: at `:11:14`; `… due to 1 previous error`.
  - `guard_t036_removal_workspace`: at `:26:14`; `… due to 3 previous errors`.
  - `guard_t036_workspace`: at `:17:14`; `… due to 1 previous error`.
  - `project`: at `:10:14`; `… due to 3 previous errors`.
  Each first error is `error[E0433]: cannot find `unix` in `os`` (path as `crates\kiln-workspace\tests\<target>.rs`).
- Exit 0: `acceptance_t073` (1 passed), `guard_t073` (10 passed), `tools_contract` (5 passed).

## Item 4: `xtask` tests (`core-crates.txt`, block `==== cargo test --locked --no-fail-fast -p xtask`)
Commands: ANSI-stripped file; `awk` over the xtask block summing `test result:` lines (fields 4 and 6) and printing the last `Running` line before each `test result:`.
| platform | `==== xtask: exit status` | binaries | whole package | `acceptance_t072` | `guard_t072` | failing tests |
|---|---|---|---|---|---|---|
| Linux | 0 | 25 | 311 passed, 0 failed | `17 passed; 0 failed`, 0.45 s | `7 passed; 0 failed`, 125.19 s | none |
| macOS | 0 | 25 | 306 passed, 0 failed | `17 passed; 0 failed`, 1.37 s | `7 passed; 0 failed`, 135.01 s | none |
| Windows | 0 | 25 | 303 passed, 0 failed | `17 passed; 0 failed`, 1.65 s | `7 passed; 0 failed`, 127.81 s | none |
(xtask unit tests, `src/main.rs`: Linux 138 passed, macOS 136 passed, Windows 134 passed, all 0 failed.)

`queue::tests::main_xtask_is_rebuilt_from_base_on_every_run`:
| platform | image version (job log) | result |
|---|---|---|
| Linux | `ubuntu-24.04` 20261111.127.1 | **passed**: `test queue::tests::main_xtask_is_rebuilt_from_base_on_every_run ... ok` (`core-crates.txt` line 409) |
| macOS | `macos-15-arm64` 20261104.0514.1 | **passed**: `test queue::tests::main_xtask_is_rebuilt_from_base_on_every_run ... ok` (line 410) |
| Windows | `windows-2025-vs2026` 20261103.298.1 | **absent**: no line with that name in `core-crates.txt`; the xtask unit-test binary ran (`Running unittests src\main.rs`, 134 passed; other `queue::tests::…` lines are present) |
No failure message exists for it on any platform. The Linux image is 20261111.127.1, i.e. not 20261116.376.1 or later.

## Item 5: gate-macos in detail
Job 134840796330, runner label `macos-15`, image `macos-15-arm64` 20261104.0514.1; `status: completed`, `conclusion: success`; created 13:37:36Z, started 13:37:41Z, completed 14:03:50Z, wall **26m09s**. All steps completed; none in progress or pending. Artefact `gate-macos-44315616697` exists (31,102 B, created 14:03:46Z).
| step | conclusion | start | end | duration |
|---|---|---|---|---|
| Set up job | success | 13:37:42Z | 13:37:43Z | 1s |
| Checkout | success | 13:37:43Z | 13:37:45Z | 2s |
| Install the pinned toolchain (rust-toolchain.toml) | success | 13:37:45Z | 13:37:53Z | 8s |
| Cache cargo registry and target | success | 13:37:53Z | 13:38:05Z | 12s |
| Gate | success | 13:38:05Z | 13:55:01Z | 16m56s (report 1008.9 s) |
| Gate report | success | 13:55:01Z | 13:55:01Z | 0s |
| Core crates (reported, not required) | success | 13:55:01Z | 14:03:40Z | 8m39s |
| Test list | success | 14:03:40Z | 14:03:44Z | 4s |
| Full test output (gate failed) | skipped | 14:03:44Z | 14:03:44Z | 0s |
| Disk | success | 14:03:44Z | 14:03:44Z | 0s |
| Upload reports | success | 14:03:44Z | 14:03:46Z | 2s |
| Post Cache cargo registry and target | success | 14:03:46Z | 14:03:46Z | 0s |
| Post Checkout | success | 14:03:46Z | 14:03:47Z | 1s |
| Complete job | success | 14:03:47Z | 14:03:48Z | 1s |
The other macOS job of this run, `window-probe (macos-15)`, was cancelled after 15m01s with no runner assigned (see the job table); its cause is not shown in anything read.

## Item 6a: Figures for "Runners, wall times and disk"
| job | runner label, image | total wall | Gate step | Test list | Full test output | disk line (`df`) | `target/` |
|---|---|---|---|---|---|---|---|
| gate-linux | ubuntu-24.04, ubuntu-24.04 20261111.127.1 | 16m02s | 10m34s (report 630.4 s) | 1s | skipped | `/dev/mapper/cow-root  145G   62G   83G  43% /` | 4.5G |
| gate-macos | macos-15, macos-15-arm64 20261104.0514.1 | 26m09s | 16m56s (report 1008.9 s) | 4s | skipped | `/dev/disk2s5   320Gi   260Gi    42Gi    87%    2.3M  436M    1%   /System/Volumes/Data` | 3.2G |
| gate-windows | windows-2025, windows-2025-vs2026 20261103.298.1 | 24m19s | 7m33s (report 429.3 s) | 1s | 1s (ran, nothing compiled) | `D:              150G  6.6G  144G   5% /d` | 3.5G |

## Compared with round 1 (`T-072-round-1.md`, run 44297420849 on d6f780562e35f42c071ffe864a91ff02ff29881f)
| item | round 1 | round 2 |
|---|---|---|
| Run / head | 44297420849, run_number 27, d6f7805 | 44315616697, run_number 30, d87f2e8 |
| Run state | in_progress when the waits ended (gate-macos unfinished); later listing (13:15Z `updated_at`) shows completed `failure` | completed, `failure` |
| gate-linux | failure (`test`: 1135 passed, 2 failed), 32m50s | success (`test`: 1171 passed, 0 failed, 137 suites), 16m02s |
| gate-macos | not finished (Full test output cancelled, Disk never completed, no artefact, no log) | success, 26m09s, completed, artefact present (31,102 B) |
| gate-windows | failure, `failed_step: clippy`, 16m30s | failure, `failed_step: clippy`, 24m19s; clippy/scorecard/test FAIL as before, other steps PASS or EMPTY |
| window-linux | success 1m09s | success 52s |
| window-probe (macos-15) | success 1m00s | **cancelled** 15m01s, no runner assigned, no artefact |
| window-probe (windows-2025) | failure 2m21s (Window probe failed, Build kiln success) | failure 2m17s (same step failed, Build kiln success) |
| Linux image | ubuntu-24.04 20261111.370.1 | ubuntu-24.04 20261111.127.1 |
| macOS image | not read | macos-15-arm64 20261104.0514.1 |
| Windows image | windows-2025-vs2026 20261103.298.1 | windows-2025-vs2026 20261103.298.1 (same) |
| Rust versions (cache step) | not recorded | 1.97.0 and 1.99.1 on all three gate jobs (listed above) |
| Cache | not recorded | Linux and macOS full-match hit; Windows `No cache found.` |
| Linux Gate step / report | 13m59s / 834.8 s | 10m34s / 630.4 s |
| macOS Gate step / report | 17m40s (failed) / not read | 16m56s / 1008.9 s |
| Windows Gate step / report | 4m42s / 266.3 s | 7m33s / 429.3 s |
| Test list step | Linux 1s, Windows 1s, macOS 4s | Linux 1s, Windows 1s, macOS 4s |
| `TEST LIST: exit status` line | Linux/Windows: `TEST LIST: exit status 0 (recorded; the gate step decides the colour)` / `… 101 (recorded; …)`; macOS not read | Linux `TEST LIST: exit status 0`, macOS `TEST LIST: exit status 0`, Windows `TEST LIST: exit status 101`; each followed by `TEST LIST: the status is recorded, not used; the gate step decides the colour` (line text differs from round 1) |
| `test-list.txt` Linux | 76,556 B, 1,567 lines, 136 binaries, 1,170 names | 76,556 B, 1,567 lines, 136 binaries (128+8), 1,170 names (same figures) |
| `test-list.txt` macOS | not read | 75,598 B, 1,550 lines, 136 binaries (128+8), 1,153 names |
| `test-list.txt` Windows | 10,832 B, 172 lines, 0 / 0 | 10,832 B, 172 lines, 0 / 0 (same) |
| Full test output step | Linux ran 11m28s (exit 101), Windows ran 1s, macOS cancelled | Linux skipped, macOS skipped, Windows ran 1s (`TEST OUTPUT: exit status 101`, same four `could not compile` lines) |
| `test-output.txt` | Linux and Windows present | Windows only (10,832 B); none on Linux or macOS |
| Artefact sizes (zip) | gate-linux 55,429; gate-windows 25,204; no gate-macos | gate-linux 31,380; gate-macos 31,102; gate-windows 24,603 |
| `xtask` whole package | Linux 309 passed, 2 failed (exit 101); Windows 301 passed, 2 failed (exit 101); macOS not read | Linux 311/0, macOS 306/0, Windows 303/0; all exit 0 |
| `acceptance_t072` | Linux, Windows `15 passed; 2 failed` | Linux, macOS, Windows `17 passed; 0 failed` |
| `guard_t072` | Linux 7/0 (125.26 s), Windows 7/0 (127.33 s) | Linux 7/0 (125.19 s), macOS 7/0 (135.01 s), Windows 7/0 (127.81 s) |
| Failing tests (xtask) | `ac1_the_list_step_runs_the_list_records_its_status_and_prints_the_binaries`, `docs_ci_table_shows_a_green_gate_macos_with_the_inode_columns` (Linux, Windows) | none on any platform |
| `queue::tests::main_xtask_is_rebuilt_from_base_on_every_run` | not recorded in round 1 | Linux passed (image 20261111.127.1), macOS passed, Windows absent |
| Windows 14 targets | 14 status lines | the same 14 names, order and statuses (acceptance_t002_r2, acceptance_t008, acceptance_t019_workspace, acceptance_t036_workspace, confinement, guard_t008, guard_t036_removal_workspace, guard_t036_workspace, project: 101 compile errors; acceptance_t002 and acceptance_t015_tools: 101 with a test failure; acceptance_t073, guard_t073, tools_contract: 0) |
| Windows T-073 step | lib 36, acceptance_t073 1, guard_t073 10; 47 passed | same: 36 + 1 + 10 = 47 passed, 0 failed |
| Windows clippy kept text | ends `kiln-cli` `guard_t036_removal_cli` (17 errors) | ends `kiln-cli` `acceptance_t023_cli` (1 error); also `guard_t019_cli` (2 errors) |
| Windows df / target | `D:  220G  3.7G  217G   2% /d`, 3.5G | `D:  150G  6.6G  144G   5% /d`, 3.5G |
| Linux df / target | `/dev/root  145G  65G  80G  45% /`, 4.5G | `/dev/mapper/cow-root  145G  62G  83G  43% /`, 4.5G |
| macOS df / target | not read | `/dev/disk2s5 320Gi 260Gi 42Gi 87% 2.3M 436M 1% /System/Volumes/Data`, 3.2G |
| Linux gate `test` suites | 1135 passed, 2 failed in 121 suites | 1171 passed, 0 failed in 137 suites |
| Scorecard line Linux | 64 items: 47 verified, 0 implemented, 0 failing, 17 planned. | the same |
| no-skip | 203 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (19 pending) | the same |

## Not read
- The contents of `window-linux-44315616697` (577 B) and `window-probe-windows-2025-44315616697` (344 B): listed, not downloaded (so the Windows probe's `PROBE: exit status` line is not read).
- `window-probe (macos-15)`: no steps, no runner, no artefact, no log read; the reason for the cancellation at 13:52:38Z is not shown by the job data (not determinable).
- Job logs of the two window jobs and of the ubuntu/macOS Core-crates steps beyond the lines quoted; the full text of clippy's, scorecard's and test's failing output on Windows beyond what the gate report keeps (60/62/27 lines).
- `xtask/platform-tests.toml` at d87f2e8 and the source of `main_xtask_is_rebuilt_from_base_on_every_run` were not read, so why the test is absent on Windows is not determinable from this run's files.
- Why the Linux gate counts 1,171 passed while `test-list.txt` has 1,170 names, and why macOS counts 1,154 against 1,153 names: not explained by any file read.

## Re-run of the failed and cancelled jobs (attempt 2)
Added after the record above. The director queued a re-run at about 14:17Z because `window-probe (macos-15)` had been cancelled with no runner assigned. Every value comes from the run and job listings (read 15:15Z), `get_workflow_job` and the artefact `window-probe-windows-2025-44315616697` (attempt 2's copy). The sections above describe attempt 1 and are unchanged.

Run 44315616697 now: `run_attempt` 2, `run_started_at` 2026-11-18T14:16:07Z, `updated_at` 14:36:19Z, `status` completed, `conclusion` **failure**. Cause by job: `gate-windows` failure and `window-probe (windows-2025)` failure, as in attempt 1; `window-probe (macos-15)` cancelled again.

Jobs of attempt 2 (wall = `completed_at` − `started_at`):
| job | id | conclusion | wall | note |
|---|---|---|---|---|
| gate-windows | 134864491321 | **failure** | 20m06s (14:16:12Z–14:36:18Z) | Gate failure 14:16:43Z–14:23:01Z (6m18s); Gate report, Core crates (14:23:01Z–14:35:58Z, 12m57s), Workspace lib and T-073 tests, Test list, kiln-workspace test targets, Full test output (ran, 1s), Disk, Upload reports all success; Post Cache skipped |
| window-probe (windows-2025) | 134864491773 | **failure** | 2m28s (14:16:12Z–14:18:40Z) | Build kiln success 14:16:45Z–14:18:34Z; Window probe failure 14:18:34Z–14:18:35Z; Upload the probe's record success |
| window-probe (macos-15) | 134864492248 | **cancelled** | 15m01s (14:16:10Z–14:31:11Z) | no `steps` in the job data; `runner_id` 0, `runner_name` empty, labels `macos-15`: no runner was assigned; cancelled again |
The other three jobs (gate-linux, gate-macos, window-linux) were not re-run: the listing shows them under `run_attempt` 2 but with their attempt-1 times and conclusions (success, success, success; job ids 134864603888, 134864495407, 134864493987).

`window-probe (macos-15)` in attempt 2: steps none; the probe did not run; **no `window-probe-macos-15-…` artefact exists** (artefact listing of the run at 15:15Z: 7 artefacts, none of that name); its exit status line therefore does not exist (the probe's `PROBE:` line is in no file or log read). The job log was not read.

Attempt 2's `window-probe (windows-2025)` artefact (`window-probe-windows-2025-44315616697`, id 14435961107, 344 B, created 14:18:36Z; attempt 1's copy is id 14431189662, created 13:39:52Z, not read), `window-probe.txt` verbatim:
```
PROBE OS: windows-2025 MINGW64_NT-10.0-26100 runnervmm7ud1 3.6.10-5a1665c8.x86_64 2026-09-28 10:40 UTC x86_64 Msys
PROBE: exit status 2
PROBE: stdout 0 bytes, 0 lines
--- stdout
--- stderr
kiln: no window: egui_glow: OpenGL: egui_glow requires opengl 2.0+.
--- end
```
Attempt 2 also uploaded a second `gate-windows-44315616697` artefact (id 14436771907, 24,736 B, created 14:36:15Z; attempt 1's: id 14433158381, 24,603 B). Its contents were **not read**; the gate result of attempt 2 beyond the job's steps above (failed step Gate) is not read.
