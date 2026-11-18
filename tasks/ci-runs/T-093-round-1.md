# T-093 round 1: `platforms.yml` on `ci/candidate`

Run 44381094009 (number 42, attempt 1, event push), branch `ci/candidate`, head `0035507d201d44cef1a4ff2a03e45f1ab8d01740` ("T-093: implementer log, gate attempt 1 green"), created 2026-11-18T19:29:35Z. Tests under study: `apps/kiln-cli/tests/macos_t093.rs`: `tree_lists_names_differing_only_by_case_as_the_file_system_holds_them`, `reopen_with_other_case_is_a_new_document_where_case_folds`.

## Jobs (list_workflow_jobs)
| job | id | runner assigned | conclusion | wall |
|---|---|---|---|---|
| gate-linux | 135073223878 | yes (runner_id 1000001109) | success | 16 min 19 s (19:29:39Z–19:45:58Z) |
| gate-macos | 135073222182 | yes (runner_id 1000001120) | success | 29 min 47 s (19:39:40Z–20:09:27Z; job created 19:29:36Z) |
| gate-windows | 135073222255 | yes (runner_id 1000001107) | failure | 17 min 27 s (19:29:38Z–19:47:05Z) |
| window-linux | 135073222300 | yes | success | 59 s |
| window-probe (macos-15) | 135073222641 | yes (runner_id 1000001121) | success | 1 min 3 s |
| window-probe (windows-2025) | 135073221718 | yes | failure | 1 min 46 s |

## gate-linux
Image `ubuntu-24.04`, version `20261116.133.1` (job log, "Runner Image" block). Artefact `gate-linux-44381094009` (32041 bytes). Gate report: passed true, 626.5 s. Steps: fmt pass; clippy pass; layers pass; ui-commands pass; hygiene pass; schema pass; scorecard pass ("64 items: 47 verified, 0 implemented, 0 failing, 17 planned."); test pass (`1202 passed, 0 failed, 0 ignored in 141 suites`); no-skip pass; fuzz empty; budgets empty.

`test-list.txt` lines (verbatim), under the binary's line:
```
     Running tests/macos_t093.rs (target/debug/deps/macos_t093-3b6efd1800d9eec9)
reopen_with_other_case_is_a_new_document_where_case_folds: test
tree_lists_names_differing_only_by_case_as_the_file_system_holds_them: test

2 tests, 0 benchmarks
```
Both names present. The gate is green (0 failed in the gate's `test` step); no `test-output.txt` was uploaded, so a per-test verdict line is not available.

## gate-macos
Completed (read 2026-11-18 about 20:13Z, after the job finished). A runner was assigned (`runner_id` 1000001120). Conclusion `success`; started 19:39:40Z, completed 20:09:27Z (29 min 47 s). Image (job log, "Runner Image" block): `Image: macos-15-arm64`, `Version: 20261112.0539.3`; macOS 15.8.2 (24H232); runner 2.338.2. Artefact `gate-macos-44381094009` (id 14466098514, 31757 bytes; fetched, size equal, `testzip()` clean); files `gate-report.json` 2020 B, `core-crates.txt` 40687 B, `test-list.txt` 77785 B; no `test-output.txt` (gate green).

Gate report: `passed: true`, `failed_step: null`, 1118.47 s.
| step | status | seconds | detail |
|---|---|---|---|
| fmt | pass | 1.09 | formatted |
| clippy | pass | 15.70 | no warnings (all targets, -D warnings) |
| layers | pass | 2.24 | 11 crates, 14 internal edges, layer table holds |
| ui-commands | pass | 0.04 | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | pass | 0.21 | reviews: 64 of 76 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 717 tracked file(s) clean (0 not UTF-8, skipped) |
| schema | pass | 7.75 | 30 commands; audit ok; docs/commands.json current |
| scorecard | pass | 239.60 | macos: 64 items; 216 tests ran; 3 registered absent here; page not compared (computed on Linux) |
| test | pass | 848.07 | `1185 passed, 0 failed, 0 ignored in 141 suites` |
| no-skip | pass | 3.76 | self-test ok (17 known findings); 207 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (10 pending), today = 2026-11-18 |
| fuzz, budgets | empty | 0.0 | no targets registered yet |

Test counts: passed 1185, failed 0, ignored 0 (gate `test` step). The gate is green, so there is no failing test and no stdout block. (Linux's `test` step read `1202 passed, 0 failed, 0 ignored in 141 suites`; both 141 suites.) `core-crates.txt` of macOS: every `==== <pkg>: exit status` line is 0 (kiln-text, kiln-syntax, kiln-search, xtask); `grep -c macos_t093 core-crates.txt` gives 0.

`test-list.txt` (ANSI stripped), the two list lines verbatim with the `Running` line they stand under:
```
     Running tests/macos_t093.rs (target/debug/deps/macos_t093-b46dcee254bc077f)
reopen_with_other_case_is_a_new_document_where_case_folds: test
tree_lists_names_differing_only_by_case_as_the_file_system_holds_them: test

2 tests, 0 benchmarks
```
Both names are present, once each (`grep -n` for each name gives one line). No per-test verdict line (`... ok`) exists: no `test-output.txt`, and `macos_t093` is not in `core-crates.txt`.

## gate-windows
Artefact `gate-windows-44381094009`. Failing gate steps: clippy, scorecard, test (first failing step clippy; `E0599` no method `mode` in `guard_t037_cli`, and `E0433` `std::os::unix` in `acceptance_t055` for test). `macos_t093` appears nowhere in the files of the artefact (`grep -rn macos_t093` over `gate-report.json`, `core-crates.txt`, `test-list.txt`, `test-output.txt`, `windows-tests.txt`, `workspace-tests.txt`: no match).

## Window jobs
window-linux success; window-probe (macos-15) success; window-probe (windows-2025) failure.

## Not read
Logs of all jobs except the Linux job's image block; `window-*` artefact contents; `core-crates.txt` of Linux; logs of `gate-macos` except its "Runner Image" block; `core-crates.txt` of macOS beyond its `test result:` and `exit status` lines; `window-probe-*` artefact contents.
