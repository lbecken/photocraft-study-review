# CI run 44350543214 (platforms.yml) on ci/candidate @ 12753798265517fed14612380d396cc3912518a7 — T-072 round 5

Record by hand; method as `tasks/ci-runs/README.md` (artefacts first; job logs for the image header, the cache step, the log-only lines and the disk step). Every value below comes from this run's job listing, its artefacts or its job logs. No judgement of results is made here; the reviewer and the director compare them with the expectation written beforehand.
This is the candidate "T-072's reviewed head merged with main" (the tree the merge queue will test), pushed 2026-11-18T16:43:34Z.
**Complete: the run had `status: completed` at the end of reading (17:16:08Z); all six jobs finished and all six artefacts were fetched.**

## Run header
- id 44350543214, run_number 36, attempt 1, event push, workflow `platforms`, title "CI candidate (not for merge): T-072's reviewed head merged with main …" (as listed)
- branch `ci/candidate`; commit `12753798265517fed14612380d396cc3912518a7` (equals the head asked for); created_at 2026-11-18T16:43:36Z
- status `completed`, run conclusion **failure** (see "Status at the end").

## macOS runner question (first)
**A runner was assigned to the macOS jobs of this run** (unlike round 3): `gate-macos` job 134964644346 has `runner_id` 1000001044, `runner_name` "GitHub Actions 1000001044" (`get_workflow_job`), its steps ran (Set up job from 16:48:41Z); `window-probe (macos-15)` job 134964644462 has `runner_id` 1000001045, `runner_name` "GitHub Actions 1000001045", all steps ran. Both jobs were created at 16:43:36Z and started at 16:48:39Z and 16:48:44Z, i.e. about 5 minutes after creation.

## Job table
Wall time = `completed_at` − `started_at` of the job (job listing).
| job | id | runner label | runner assigned | conclusion | wall | first failing step |
|---|---|---|---|---|---|---|
| gate-linux | 134964644693 | ubuntu-24.04 | yes (1000001033) | success | 16m14s (16:43:39Z–16:59:53Z) | none |
| gate-macos | 134964644346 | macos-15 | yes (1000001044) | success | 27m29s (16:48:39Z–17:16:08Z) | none |
| gate-windows | 134964643950 | windows-2025 | yes (1000001029) | **failure** | 23m55s (16:43:40Z–17:07:35Z) | Gate (16:44:11Z–16:50:53Z); every later step success or skipped |
| window-linux | 134964644589 | ubuntu-24.04 | yes (1000001034) | success | 53s (16:43:39Z–16:44:32Z) | none |
| window-probe (macos-15) | 134964644462 | macos-15 | yes (1000001045) | success | 51s (16:48:44Z–16:49:35Z) | none |
| window-probe (windows-2025) | 134964644420 | windows-2025 | yes (1000001032) | **failure** | 2m03s (16:43:38Z–16:45:41Z) | Window probe (16:45:35Z–16:45:37Z); Build kiln success |

Artefacts (`list_workflow_run_artifacts`; fetched with `download_workflow_run_artifact` and curl, size equal to `size_in_bytes`, `testzip()` clean; the scratch directory is removed after this record):
`gate-windows-44350543214` 24,036 B (id 14452320214); `gate-linux-44350543214` 31,620 B (14450360460); `window-probe-macos-15-44350543214` 703 B (14449380582); `window-probe-windows-2025-44350543214` 344 B (14447673121); `window-linux-44350543214` 577 B (14446694985). `gate-macos-44350543214` 31,363 B (id 14453766316; not in the listing at the first read, about 17:11Z, because the job was still running; listed and fetched at about 17:22Z). 6 artefacts in all.
Job logs (`get_job_logs`, then the logs URL fetched with curl) were read for gate-linux and gate-windows (and gate-macos).

## Gate jobs

### gate-linux (job 134964644693)
- Runner image (job log "Runner Image"): `Image: ubuntu-24.04`, `Version: 20261111.127.1`; provisioner `Version: 20261103.629`.
- Rust versions listed by the cache step: `1.97.0 x86_64-unknown-linux-gnu 2d8144b7880597b6e6d3dfd63a9a9efae3f533d3` and `1.99.1 x86_64-unknown-linux-gnu 58cf8f9c35b355199727627d07ae34a4e2846d67`. Toolchain step printed `cargo 1.97.0 (c980f4866 2026-06-30)`, `clippy 0.1.97 (2d8144b788 2026-07-07)`, `rustfmt 1.9.0-stable (2d8144b788 2026-07-07)`. Cache: `Cache hit for: v0-rust-gate-linux-Linux-x64-f37a6dc5-6187dc6e`, `Cache Size: ~358 MB (375896019 B)`, `full match: true`.
- Steps (job listing):
| step | conclusion | start–end (Z) | duration |
|---|---|---|---|
| Set up job | success | 16:43:40–16:43:43 | 3s |
| Checkout | success | 16:43:43–16:43:48 | 5s |
| Install the pinned toolchain (rust-toolchain.toml) | success | 16:43:48–16:43:57 | 9s |
| Cache cargo registry and target | success | 16:43:57–16:44:07 | 10s |
| Gate | success | 16:44:07–16:54:36 | 10m29s |
| Gate report | success | 16:54:36 | 0s |
| Core crates (reported, not required) | success | 16:54:36–16:59:46 | 5m10s |
| Test list | success | 16:59:46–16:59:47 | 1s |
| Full test output (gate failed) | **skipped** | 16:59:47 | 0s |
| Disk | success | 16:59:47 | 0s |
| Upload reports | success | 16:59:47–16:59:49 | 2s |
| Post Cache, Post Checkout, Complete job | success, success, success | 16:59:49 | 0s each |
- Gate report (`gate-report.json`, 1,986 B): `passed: true`, `failed_step: null`, 624.7 s.
| step | status | seconds | detail |
|---|---|---|---|
| fmt | pass | 0.9 | formatted |
| clippy | pass | 9.7 | no warnings (all targets, -D warnings) |
| layers | pass | 0.5 | 11 crates, 14 internal edges, layer table holds |
| ui-commands | pass | 0.03 | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | pass | 0.06 | reviews: 62 of 74 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 706 tracked file(s) clean (0 not UTF-8, skipped) |
| schema | pass | 4.5 | 30 commands; audit ok; docs/commands.json current |
| scorecard | pass | 151.3 | 64 items: 47 verified, 0 implemented, 0 failing, 17 planned. |
| test | pass | 455.6 | 1176 passed, 0 failed, 0 ignored in 138 suites |
| no-skip | pass | 2.2 | self-test ok (17 known findings); 204 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (19 pending), today = 2026-11-18 |
| fuzz, budgets | empty | 0.0 | no targets registered yet (xtask/fuzz.toml, xtask/budgets.toml) |
- "Test list": 1 s. Log lines, verbatim: `TEST LIST: exit status 0` / `TEST LIST: the status is recorded, not used; the gate step decides the colour`.
- `test-list.txt` (77,010 B, 1,575 lines): 137 binary lines (129 `Running`, 8 `Doc-tests`), 1,175 lines ending `: test` (test names). Commands: `grep -c Running`, `grep -c Doc-tests`, `grep -c ': test$'`, `wc -l` on the ANSI-stripped file.
- "Full test output (gate failed)": **skipped** (gate green). No `TEST OUTPUT:` line, nothing printed, no `test-output.txt` in the artefact.
- Artefact files: `gate-report.json` 1,986 B; `core-crates.txt` 40,529 B; `test-list.txt` 77,010 B. Zip 31,620 B.
- Disk (job log): `Filesystem            Size  Used Avail Use% Mounted on`, `/dev/mapper/cow-root  145G   62G   83G  43% /`; `DISK: du -sh target` → `4.5G	target`.
- `core-crates.txt`: labels `==== cargo test --locked --no-fail-fast -p kiln-text|kiln-syntax|kiln-search|xtask`; each ends `exit status 0` (log line: `CORE CRATES: exit status per package: kiln-text=0 kiln-syntax=0 kiln-search=0 xtask=0 (reported, not required)`). kiln-text 78 passed, kiln-syntax 53, kiln-search 55, xtask 316; 0 failed in all.

### gate-windows (job 134964643950)
- Runner image: `Image: windows-2025-vs2026`, `Version: 20261103.298.1`; provisioner `Version: 20261103.629`. Step shell: `C:\Program Files\Git\bin\bash.EXE --noprofile --norc -e -o pipefail {0}`.
- Rust versions listed by the cache step: `1.97.0 x86_64-pc-windows-msvc 2d8144b7880597b6e6d3dfd63a9a9efae3f533d3` and `1.99.1 x86_64-pc-windows-msvc 58cf8f9c35b355199727627d07ae34a4e2846d67`. Toolchain step printed `cargo 1.97.0 (c980f4866 2026-06-30)`, `clippy 0.1.97 (2d8144b788 2026-07-07)`, `rustfmt 1.9.0-stable (2d8144b788 2026-07-07)`. Cache: `No cache found.`
- Steps:
| step | conclusion | start–end (Z) | duration |
|---|---|---|---|
| Set up job | success | 16:43:42–16:43:43 | 1s |
| Checkout | success | 16:43:43–16:43:51 | 8s |
| Install the pinned toolchain (rust-toolchain.toml) | success | 16:43:51–16:44:05 | 14s |
| Cache cargo registry and target | success | 16:44:05–16:44:11 | 6s |
| Gate | **failure** | 16:44:11–16:50:53 | 6m42s |
| Gate report | success | 16:50:53 | 0s |
| Core crates (reported, not required) | success | 16:50:53–17:07:08 | 16m15s |
| Workspace lib and T-073 tests (reported, not required) | success | 17:07:08–17:07:14 | 6s |
| Test list | success | 17:07:14–17:07:20 | 6s |
| kiln-workspace test targets (reported, not required) | success | 17:07:20–17:07:27 | 7s |
| Full test output (gate failed) | success (**ran**) | 17:07:27–17:07:28 | 1s |
| Disk | success | 17:07:28–17:07:31 | 3s |
| Upload reports | success | 17:07:31–17:07:32 | 1s |
| Post Cache | skipped | 17:07:32 | 0s |
| Post Checkout, Complete job | success, success | 17:07:32–17:07:34 | 2s, 0s |
- Gate report (`gate-report.json`, 17,130 B): `passed: false`, `failed_step: clippy`, 379.9 s.
| step | status | seconds | detail |
|---|---|---|---|
| fmt | pass | 2.1 | formatted |
| clippy | **fail** | 55.3 | kept text (60 lines) ends `error: could not compile `kiln-cli` (test "acceptance_t037_cli") due to 16 previous errors` / `warning: build failed, waiting for other jobs to finish...`; first error `error[E0599]: no method named `mode` found for struct `std::fs::Metadata`` at `apps\kiln-cli\tests\acceptance_t037_cli.rs:153:15` (`uid` at :154 follows) |
| layers | pass | 9.8 | 11 crates, 14 internal edges, layer table holds |
| ui-commands | pass | 0.04 | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | pass | 0.2 | reviews: 62 of 74 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 706 tracked file(s) clean (0 not UTF-8, skipped) |
| schema | pass | 31.1 | 30 commands; audit ok; docs/commands.json current |
| scorecard | **fail** | 271.0 | first line `windows: 64 items; 158 tests ran; 3 registered absent here; page not compared (computed on Linux)`, then `60 scorecard problems:` (first: `building test executables failed on windows (); their tests are not run`); kept text 62 lines |
| test | **fail** | 5.9 | `0 passed, 0 failed, 0 ignored in 0 suites`; kept text (27 lines) contains `error[E0433]: cannot find `unix` in `os`` at `apps\kiln\tests\acceptance_t055.rs:1317:18` and ends `error: failed to remove file `<workspace>\target\debug\xtask.exe`` / `Caused by:` / `  Access is denied. (os error 5)` (workspace path shortened here) |
| no-skip | pass | 4.5 | self-test ok (17 known findings); 204 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (19 pending), today = 2026-11-18 |
| fuzz, budgets | empty | 0.0 | no targets registered yet (xtask/fuzz.toml, xtask/budgets.toml) |
- "Test list": 6 s. Log lines, verbatim: `TEST LIST: exit status 101` / `TEST LIST: the status is recorded, not used; the gate step decides the colour`.
- `test-list.txt` (5,023 B, 81 lines): compile errors only; **0 binary lines, 0 test names**; 3 `could not compile` lines (`acceptance_t065` 3 errors, `script_t055` 1, `acceptance_t055` 2).
- "Full test output (gate failed)": **ran** (1 s). Log lines, verbatim: `TEST OUTPUT: exit status 101` / `TEST OUTPUT: the status is recorded, not used; the gate step decides the colour`. The printer printed only `error[…]` lines and `error: could not compile` lines: `E0433 cannot find `unix` in `os``, `E0599 … `from_bytes` found for struct `OsStr``, `E0599 … `from_vec` found for struct `OsString``, `E0277 the size for values of type `[OsString]` cannot be known at compilation time`; the four `could not compile` lines, verbatim: `error: could not compile `kiln` (test "window_t065") due to 8 previous errors`, `error: could not compile `kiln` (test "acceptance_t065") due to 3 previous errors`, `error: could not compile `kiln` (test "script_t055") due to 1 previous error`, `error: could not compile `kiln` (test "acceptance_t055") due to 2 previous errors`. `test-output.txt` (10,832 B, 172 lines): 0 lines `Running`, `test result:` or `FAILED`; no failing test name and no panic message in it (nothing compiled to run).
- Artefact files: `gate-report.json` 17,130 B; `core-crates.txt` 40,889 B; `test-output.txt` 10,832 B; `test-list.txt` 5,023 B; `workspace-tests.txt` 7,134 B; `windows-tests.txt` 62,810 B. Zip 24,036 B.
- Disk (job log): `Filesystem      Size  Used Avail Use% Mounted on`, `D:              150G  6.6G  144G   5% /d`; `DISK: du -sh target` → `3.5G	target`.
- `core-crates.txt`: all four packages `exit status 0` (log: `CORE CRATES: exit status per package: kiln-text=0 kiln-syntax=0 kiln-search=0 xtask=0 (reported, not required)`); kiln-text 78, kiln-syntax 53, kiln-search 55, xtask 308 passed; 0 failed.

### gate-windows, T-073 step and the 14-target step
"Workspace lib and T-073 tests (reported, not required)" (`workspace-tests.txt`; log line `WORKSPACE TESTS: exit status 0 (reported, not required)`): lib `36 passed; 0 failed`, `acceptance_t073` `1 passed; 0 failed`, `guard_t073` `10 passed; 0 failed`: 47 passed, 0 failed. Two `MEASURED:` lines, verbatim:
- `MEASURED: acw2_root_parent_replaced_is_outside_root: windows: rename of p while the root is held: refused: kind PermissionDenied, raw OS error Some(5) (Access is denied. (os error 5))`
- `MEASURED: acw2_root_replaced_at_same_path_is_outside_root: windows: rename of root while the root is held: ok`

"kiln-workspace test targets (reported, not required)" (`windows-tests.txt`; 14 `==== cargo test … --test <target>` blocks), every status line verbatim:
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

Non-zero targets (`windows-tests.txt`):
- `acceptance_t002` (`test result: FAILED. 14 passed; 1 failed`): `test disk_save_replaces_via_rename_without_leftovers ... FAILED`; `thread 'disk_save_replaces_via_rename_without_leftovers' (4468) panicked at crates\kiln-workspace\tests\acceptance_t002.rs:940:5:` / `precondition: permission bits and inode numbers are Unix metadata; the Windows checks of atomic replace are not written yet (ADR-0009: T-073 and T-074 write the Windows behaviour and its tests)`.
- `acceptance_t015_tools` (`test result: FAILED. 2 passed; 1 failed`): `test no_tools_never_runs ... FAILED`; `thread 'no_tools_never_runs' (10224) panicked at crates\kiln-workspace\tests\acceptance_t015_tools.rs:131:5:` / `precondition: the fake `cargo`/`rustfmt` of this test are shell scripts and need Unix`.
- Compile errors (no test ran), closing lines: `acceptance_t002_r2` 11 errors; `acceptance_t008` 20; `acceptance_t019_workspace` 5; `acceptance_t036_workspace` 16; `confinement` 24; `guard_t008` 1; `guard_t036_removal_workspace` 3; `guard_t036_workspace` 1; `project` 3 (each `error: could not compile `kiln-workspace` (test "<target>") due to N previous error(s)`).
- Exit 0: `acceptance_t073` (1 passed), `guard_t073` (10 passed), `tools_contract` (5 passed).

## gate-macos step by step
Job 134964644346, runner label `macos-15`; **runner assigned** (`runner_id` 1000001044). `status: completed`, `conclusion: success`; created 16:43:36Z, started 16:48:39Z, completed 17:16:08Z, wall **27m29s**.
- Runner image (job log): `Image: macos-15-arm64`, `Version: 20261112.0539.3`; provisioner `Version: 20261103.629`; runner version 2.338.2.
- Rust versions listed by the cache step: `1.97.0 aarch64-apple-darwin 2d8144b7880597b6e6d3dfd63a9a9efae3f533d3` and `1.99.1 aarch64-apple-darwin 58cf8f9c35b355199727627d07ae34a4e2846d67`. Toolchain step printed `cargo 1.97.0 (c980f4866 2026-06-30)`, `clippy 0.1.97 (2d8144b788 2026-07-07)`, `rustfmt 1.9.0-stable (2d8144b788 2026-07-07)`. Cache: `Cache hit for: v0-rust-gate-macos-Darwin-arm64-03a9c7df-6187dc6e`, `Cache Size: ~349 MB (366033134 B)`, `full match: true`.
- Steps (job listing):
| step | conclusion | start–end (Z) | duration |
|---|---|---|---|
| Set up job | success | 16:48:41–16:48:44 | 3s |
| Checkout | success | 16:48:44–16:48:47 | 3s |
| Install the pinned toolchain (rust-toolchain.toml) | success | 16:48:47–16:48:57 | 10s |
| Cache cargo registry and target | success | 16:48:57–16:49:18 | 21s |
| Gate | success | 16:49:18–17:06:53 | 17m35s |
| Gate report | success | 17:06:53 | 0s |
| Core crates (reported, not required) | success | 17:06:53–17:15:58 | 9m05s |
| Test list | success | 17:15:58–17:16:01 | 3s |
| Full test output (gate failed) | **skipped** | 17:16:01 | 0s |
| Disk | success | 17:16:01–17:16:02 | 1s |
| Upload reports | success | 17:16:02–17:16:03 | 1s |
| Post Cache, Post Checkout, Complete job | success, success, success | 17:16:03–17:16:06 | 1s, 0s, 2s |
- Gate report (`gate-report.json`, 2,024 B): `passed: true`, `failed_step: null`, 1043.3 s.
| step | status | seconds | detail |
|---|---|---|---|
| fmt | pass | 1.0 | formatted |
| clippy | pass | 12.6 | no warnings (all targets, -D warnings) |
| layers | pass | 1.4 | 11 crates, 14 internal edges, layer table holds |
| ui-commands | pass | 0.03 | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | pass | 0.3 | reviews: 62 of 74 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 706 tracked file(s) clean (0 not UTF-8, skipped) |
| schema | pass | 7.2 | 30 commands; audit ok; docs/commands.json current |
| scorecard | pass | 244.6 | macos: 64 items; 216 tests ran; 3 registered absent here; page not compared (computed on Linux) |
| test | pass | 773.4 | 1159 passed, 0 failed, 0 ignored in 138 suites |
| no-skip | pass | 2.9 | self-test ok (17 known findings); 204 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (19 pending), today = 2026-11-18 |
| fuzz, budgets | empty | 0.0 | no targets registered yet (xtask/fuzz.toml, xtask/budgets.toml) |
- "Test list": 3 s. Log lines, verbatim: `TEST LIST: exit status 0` / `TEST LIST: the status is recorded, not used; the gate step decides the colour`.
- `test-list.txt` (76,052 B, 1,558 lines): 137 binary lines (129 `Running`, 8 `Doc-tests`), 1,158 lines ending `: test` (test names). Same commands as for Linux.
- "Full test output (gate failed)": **skipped** (gate green); no `test-output.txt`.
- Artefact files: `gate-report.json` 2,024 B; `core-crates.txt` 40,296 B; `test-list.txt` 76,052 B. Zip 31,363 B (artefact id 14453766316, created 17:16:03Z).
- Disk (job log): `Filesystem      Size    Used   Avail Capacity iused ifree %iused  Mounted on`, `/dev/disk2s5   320Gi   262Gi    40Gi    87%    2.3M  420M    1%   /System/Volumes/Data`; `DISK: du -sh target` → `3.2G	target`.
- `core-crates.txt`: all four packages `exit status 0` (log: `CORE CRATES: exit status per package: kiln-text=0 kiln-syntax=0 kiln-search=0 xtask=0 (reported, not required)`); kiln-text 78 passed, kiln-syntax 53, kiln-search 55, xtask 311; 0 failed.

## `xtask` tests (`core-crates.txt`, block `==== cargo test --locked --no-fail-fast -p xtask`)
Commands: ANSI-stripped file; `awk '/-p xtask$/{x=1} x&&/^test result/{p+=$4;f+=$6}'` (totals); `awk '/-p xtask$/{x=1} x&&/Running/{r=$0} x&&/^test result/{print r" => "$0}'` (per binary); `grep -nE 'g1_probe|main_xtask_is_rebuilt|FAILED|panicked'`.
| platform | `==== xtask: exit status` | binaries (`Running` / `test result:` lines) | whole package | `acceptance_t072` | `guard_t072` | `acceptance_t096` | unit-test binary (`src/main.rs`) | failing tests |
|---|---|---|---|---|---|---|---|---|
| Linux | 0 | 26 / 26 | 316 passed, 0 failed | `20 passed; 0 failed`, finished in 0.47s | `8 passed; 0 failed`, finished in 125.19s | `1 passed; 0 failed`, finished in 8.83s | `138 passed; 0 failed`, 3.00s | none |
| macOS | 0 | 26 / 26 | 311 passed, 0 failed | `20 passed; 0 failed`, finished in 1.28s | `8 passed; 0 failed`, finished in 135.15s | `1 passed; 0 failed`, finished in 13.80s | `136 passed; 0 failed`, 22.26s | none |
| Windows | 0 | 26 / 26 | 308 passed, 0 failed | `20 passed; 0 failed`, finished in 2.34s | `8 passed; 0 failed`, finished in 127.91s | `1 passed; 0 failed`, finished in 20.51s | `134 passed; 0 failed`, 93.11s | none |

`guard_t072::g1_probe_with_a_kiln_that_never_exits_ends_within_180_s_and_records_the_kill` (verbatim lines in `core-crates.txt`):
| platform | lines | binary time |
|---|---|---|
| Linux | `test g1_probe_with_a_kiln_that_never_exits_ends_within_180_s_and_records_the_kill has been running for over 60 seconds` (line 766) / `test g1_probe_with_a_kiln_that_never_exits_ends_within_180_s_and_records_the_kill ... ok` (line 767) | `guard_t072` binary `finished in 125.19s` (no per-test time in the file) |
| Windows | the same two lines (lines 773, 774) | `guard_t072` binary `finished in 127.91s` |
| macOS | `test g1_probe_with_a_kiln_that_never_exits_ends_within_180_s_and_records_the_kill has been running for over 60 seconds` (line 762) / `test g1_probe_with_a_kiln_that_never_exits_ends_within_180_s_and_records_the_kill ... ok` (line 763) | `guard_t072` binary `finished in 135.15s` |
`grep -nE '^test .* FAILED$|panicked'` finds no line in the Linux, Windows and macOS `core-crates.txt`. The `acceptance_t072` binary lists 20 tests on Linux (`running 20 tests`, line 668) and on Windows (line 676) and macOS (line 664); `guard_t072` lists 8 (`running 8 tests`, lines 758 and 765).

`queue::tests::main_xtask_is_rebuilt_from_base_on_every_run`: Linux `ubuntu-24.04` 20261111.127.1: `test queue::tests::main_xtask_is_rebuilt_from_base_on_every_run ... ok` (`core-crates.txt` line 409; 20 `^test queue::tests::` lines in all). Windows `windows-2025-vs2026` 20261103.298.1: **absent** (no line with that name; the unit-test binary ran, 134 passed; 18 `queue::tests::…` lines are present). macOS `macos-15-arm64` 20261112.0539.3: `test queue::tests::main_xtask_is_rebuilt_from_base_on_every_run ... ok` (line 409; 19 `^test queue::tests::` lines in all).

## Window jobs
- `window-linux` (job 134964644589): success, 53s; all steps success (Build kiln 16:43:59Z–16:44:05Z; Window procedure (T-065) under Xvfb 16:44:13Z–16:44:29Z). `window-procedure.txt` (1,207 B) read; it contains six `CASE …` blocks (ac2, ac2x3, dblclick, scroll100, ctrlq, noroot; each `exit window=N headless=N`, `lines … EQUAL`, `files EQUAL`), `WINDOW CASES PASS`, `AC-3: exit=2 stdout_bytes=0 stderr_lines=1 panicked=0 (pass: 2 0 1 0)` with stderr `kiln: no window: Library libxkbcommon-x11.so could not be loaded.`, `AC-3 PASS`, `missing script: exit=2 stdout_bytes=0 (pass: 2 0; 124 = a window opened)`, three `plain …: drawn_and_stable=yes still_running=yes` lines, `two runs without a path: same screen`, `PLAIN PASS: the start path changes the screen`.
- `window-probe (windows-2025)` (job 134964644420): failure, 2m03s; Build kiln success (16:44:07Z–16:45:35Z), Window probe failure (16:45:35Z–16:45:37Z), Upload the probe's record success. `window-probe.txt` (265 B), verbatim:
```
PROBE OS: windows-2025 MINGW64_NT-10.0-26100 runnervmm7ud1 3.6.10-5a1665c8.x86_64 2026-09-28 10:40 UTC x86_64 Msys
PROBE: exit status 2
PROBE: stdout 0 bytes, 0 lines
--- stdout
--- stderr
kiln: no window: egui_glow: OpenGL: egui_glow requires opengl 2.0+.
--- end
```
- `window-probe (macos-15)` (job 134964644462): **runner assigned** (1000001045); success, 51s; all steps success (Build kiln 16:49:12Z–16:49:28Z, Window probe 16:49:28Z–16:49:30Z). `window-probe.txt` (2,549 B): `PROBE OS: macos-15 Darwin … 24.6.0 … arm64`, `ProductName: macOS`, `ProductVersion: 15.8.2`, `BuildVersion: 24H232`, `PROBE: exit status 0`, `PROBE: stdout     2174 bytes, 3 lines`, stdout of 3 JSON lines (a snapshot with `"frames":3`, `{"id":1,"ok":true,"result":{"docs":[]}}`, a second snapshot), stderr empty.

## Compared with round 3 (`T-072-round-3.md`, run 44333697372 on bff0d7a38562d3ef1a0d6cb83fff781673b5040a)
| item | round 3 | round 5 |
|---|---|---|
| Run / head | 44333697372, run_number 33, bff0d7a | 44350543214, run_number 36, 1275379 |
| macOS runner | none assigned (`runner_id` 0, cancelled after 15m01s, no steps) | assigned for both macOS jobs; steps ran |
| gate-macos | cancelled 15m01s, no artefact | success 27m29s; `test` 1159 passed, 0 failed, 138 suites; gate report 1043.3 s |
| window-probe (macos-15) | cancelled 15m01s, no runner | success 51s, `PROBE: exit status 0` |
| gate-linux | success 21m59s; `test` 1175 passed, 137 suites | success 16m14s; `test` 1176 passed, 0 failed, 138 suites |
| gate-windows | failure 27m07s; `failed_step: clippy` | failure 23m55s; `failed_step: clippy` |
| window-linux | success 1m04s | success 53s |
| window-probe (windows-2025) | failure 2m12s; `PROBE: exit status 2` | failure 2m03s; `PROBE: exit status 2`, same stderr text |
| Linux image | ubuntu-24.04 20261111.370.1 | ubuntu-24.04 20261111.127.1 |
| Windows image | windows-2025-vs2026 20261103.298.1 | the same |
| Rust versions (cache step) | 1.97.0 and 1.99.1 | the same |
| Cache | Linux full match, Windows none | the same |
| Linux Gate step / report | 14m51s / 886.5 s | 10m29s / 624.7 s |
| Windows Gate step / report | 8m23s / 478.9 s | 6m42s / 379.9 s |
| Linux `test-list.txt` | 76,803 B, 1,571 lines, 136 binaries (128+8), 1,174 names | 77,010 B, 1,575 lines, 137 binaries (129+8), 1,175 names |
| Windows `test-list.txt` | 9,109 B, 142 lines, 0 / 0 | 5,023 B, 81 lines, 0 / 0 |
| `TEST LIST: exit status` | Linux 0, Windows 101 | Linux 0, Windows 101 |
| Windows Full test output | ran 2s, `TEST OUTPUT: exit status 101`; `test-output.txt` 10,832 B | ran 1s, `TEST OUTPUT: exit status 101`; `test-output.txt` 10,832 B |
| `xtask` whole package | Linux 315, Windows 307; 0 failed; macOS not run | Linux 316, Windows 308; 0 failed; macOS 311, all 0 failed |
| `acceptance_t072` | `20 passed; 0 failed` on Linux and Windows | `20 passed; 0 failed` on Linux and Windows |
| `guard_t072` | Linux 8/0 (125.24 s), Windows 8/0 (127.99 s) | Linux 8/0 (125.19 s), Windows 8/0 (127.91 s) |
| `g1_probe_…` name | `…within_135_s…`, `ok` on Linux and Windows | `…within_180_s…`, `ok` on Linux and Windows |
| Failing tests (xtask) | none | none on Linux and Windows |
| `queue::tests::main_xtask_is_rebuilt_from_base_on_every_run` | Linux ok, Windows absent, macOS not run | Linux ok, Windows absent, macOS ok |
| Windows T-073 step | lib 36, t073 1, guard 10; 47 passed | the same |
| Windows 14 targets | 14 status lines | the same 14 names and statuses; failures of `acceptance_t002` and `acceptance_t015_tools` with the same messages (thread ids differ) |
| Windows clippy kept text | ends `kiln-cli` `guard_t037_cli` (16 errors) | ends `kiln-cli` `acceptance_t037_cli` (16 errors) |
| Windows scorecard first line | `windows: 64 items; 158 tests ran; 3 registered absent here …`, `60 scorecard problems:` | the same |
| Windows `test` step kept text | ends `could not compile `kiln` (test "acceptance_t055") due to 2 previous errors` | contains the `acceptance_t055.rs:1317:18` `unix` error and ends `failed to remove file …\target\debug\xtask.exe` / `Access is denied. (os error 5)` |
| hygiene (Linux) | 60 of 72 review files; 695 tracked files | 62 of 74 review files; 706 tracked files |
| Linux df / target | `/dev/root  145G  65G  80G  45% /`, 4.5G | `/dev/mapper/cow-root  145G   62G   83G  43% /`, 4.5G |
| Windows df / target | `D:  150G  6.6G  144G   5% /d`, 3.5G | the same |

## Status at the end
Run `status: completed`, run conclusion **failure** (`updated_at` 2026-11-18T17:16:08Z). Cause by job: `gate-windows` failure and `window-probe (windows-2025)` failure; the four other jobs success. Nothing was still running at the end; the run was still `in_progress` at about 17:11Z (only `gate-macos`, in its "Core crates" step, was unfinished then) and was waited for until it completed at 17:16:08Z.

## Not read
- Job log of `gate-macos` beyond the image header, Rust versions, cache, `TEST LIST`, `CORE CRATES` and disk lines (not read in full); job logs of the window jobs.
- Job logs of the window jobs; the ubuntu and Windows steps' own output beyond the lines quoted; the full text of clippy's, scorecard's and test's failing output on Windows beyond what the gate report keeps (60/62/27 lines).
- `xtask/platform-tests.toml` at 1275379 and the source of `main_xtask_is_rebuilt_from_base_on_every_run` and of `guard_t072.rs`: not read, so why the first is absent on Windows is not determinable from this run's files.
- Why the Linux gate counts 1,176 passed while `test-list.txt` has 1,175 names: not explained by any file read.
- Per-test results other than those named above (xtask `core-crates.txt` lines were counted, not read test by test).
