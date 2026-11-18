# T-093 round 1: `platforms.yml` on `ci/candidate`

Run 44381094009 (number 42, attempt 1, event push), branch `ci/candidate`, head `0035507d201d44cef1a4ff2a03e45f1ab8d01740` ("T-093: implementer log, gate attempt 1 green"), created 2026-11-18T19:29:35Z. Tests under study: `apps/kiln-cli/tests/macos_t093.rs`: `tree_lists_names_differing_only_by_case_as_the_file_system_holds_them`, `reopen_with_other_case_is_a_new_document_where_case_folds`.

## Jobs (list_workflow_jobs)
| job | id | runner assigned | conclusion | wall |
|---|---|---|---|---|
| gate-linux | 135073223878 | yes (runner_id 1000001109) | success | 16 min 19 s (19:29:39Z–19:45:58Z) |
| gate-macos | 135073222182 | yes (runner_id 1000001120) | not complete (status `in_progress`) | not complete (started 19:39:40Z) |
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
A runner was assigned (`runner_id` 1000001120). The job was still `in_progress` at the last read: steps Set up job, Checkout, toolchain, Cache succeeded; `Gate` completed `success` (19:40:14Z–19:59:02Z) and `Gate report` `success`; `Core crates (reported, not required)` was `in_progress` since 19:59:02Z; `Test list`, `Full test output`, `Disk` and `Upload reports` still `pending`. No `gate-macos-44381094009` artefact existed yet, so the image version, the gate report's step results and counts, the `test-list.txt` lines of the two tests and any per-test verdict are **not read** (waiting limit reached).

## gate-windows
Artefact `gate-windows-44381094009`. Failing gate steps: clippy, scorecard, test (first failing step clippy; `E0599` no method `mode` in `guard_t037_cli`, and `E0433` `std::os::unix` in `acceptance_t055` for test). `macos_t093` appears nowhere in the files of the artefact (`grep -rn macos_t093` over `gate-report.json`, `core-crates.txt`, `test-list.txt`, `test-output.txt`, `windows-tests.txt`, `workspace-tests.txt`: no match).

## Window jobs
window-linux success; window-probe (macos-15) success; window-probe (windows-2025) failure.

## Not read
Logs of all jobs except the Linux job's image block; `window-*` artefact contents; `core-crates.txt` of Linux; the `gate-macos` artefact (job unfinished at the end of the waiting time).
