# CI run 44350556911 (platforms.yml, negative candidate) on ci/candidate @ 0f8e5b91fcdf87d03152addc0c2035bdd760ce0a — T-072 negative

Record by hand; method as `tasks/ci-runs/README.md` and the model `tasks/ci-runs/T-070-negative.md`. Every value below comes from this run's job listing, its artefacts or its job logs. No judgement of results is made here. Times UTC. Wall = `completed_at` − `started_at` of the job (job listing).
The candidate is the tree of `T-072-round-5.md` (head 12753798265517fed14612380d396cc3912518a7) plus one deliberately failing test, `t072_negative_candidate_fails_on_purpose` in `crates/kiln-text/tests/negative_t072.rs` (pushed 2026-11-18T16:43:39Z). From the artefacts: its stdout has the lines `negative candidate output line 1` to `… 12`, then `     Running tests/heading_shaped_line_inside_a_block.rs (target/debug/deps/none-0000)`, then lines 14 to 25, then the panic `assertion `left == right` failed: T-072 negative candidate: this message must appear in the record` (`left: 2`, `right: 3`) at `negative_t072.rs:13:5`.

## Run header
- id 44350556911, run_number 37, attempt 1, event push, workflow `platforms`, title "CI negative candidate for T-072 (AC-8; not for merge): one deliberate…" (as listed)
- branch `ci/candidate`; commit `0f8e5b91fcdf87d03152addc0c2035bdd760ce0a` (equals the head asked for); created_at 2026-11-18T16:43:40Z
- `status: completed`, run conclusion **failure** (`updated_at` 2026-11-18T17:23:05Z). Cause by job: `gate-linux`, `gate-macos`, `gate-windows` failure and `window-probe (windows-2025)` failure; `window-linux` and `window-probe (macos-15)` success. The run was `in_progress` until 17:23:05Z (waited for: the macOS gate finished last); nothing was still running at the end.

## macOS runner question (first)
**A runner was assigned to both macOS jobs of this run**: `gate-macos` job 134964695395 has `runner_id` 1000001041, `runner_name` "GitHub Actions 1000001041"; `window-probe (macos-15)` job 134964695544 has `runner_id` 1000001046, `runner_name` "GitHub Actions 1000001046" (job listing / `get_workflow_job`); steps ran in both. The jobs started about 4.5 to 5.5 minutes after the run was created (gate-macos started 16:48:15Z, window-probe 16:49:21Z; run created 16:43:40Z).

## Job table
| job | id | runner label | runner assigned | conclusion | wall | first failing step |
|---|---|---|---|---|---|---|
| gate-linux | 134964695406 | ubuntu-24.04 | yes (1000001035) | **failure** | 20m54s (16:43:44Z–17:04:38Z) | Gate (16:44:17Z–16:51:41Z); every later step success or skipped (Post Cache skipped) |
| gate-macos | 134964695395 | macos-15 | yes (1000001041) | **failure** | 34m49s (16:48:15Z–17:23:04Z) | Gate (16:48:44Z–16:59:47Z); every later step success or skipped (Post Cache skipped) |
| gate-windows | 134964695336 | windows-2025 | yes (1000001037) | **failure** | 27m29s (16:43:43Z–17:11:12Z) | Gate (16:44:16Z–16:52:28Z); every later step success or skipped (Post Cache skipped) |
| window-linux | 134964694815 | ubuntu-24.04 | yes (1000001038) | success | 1m28s (16:43:44Z–16:45:12Z) | none |
| window-probe (macos-15) | 134964695544 | macos-15 | yes (1000001046) | success | 49s (16:49:21Z–16:50:10Z) | none |
| window-probe (windows-2025) | 134964695207 | windows-2025 | yes (1000001036) | **failure** | 2m16s (16:43:43Z–16:45:59Z) | Window probe (16:45:53Z–16:45:54Z); Build kiln success |

Artefacts (`list_workflow_run_artifacts`; the three gate artefacts and nothing else of them fetched with `download_workflow_run_artifact` and curl, size equal to `size_in_bytes`, `testzip()` clean; the scratch directory is removed after this record): `gate-windows-44350556911` 25,481 B (id 14452520719); `gate-linux-44350556911` 55,643 B (14450076605); `window-probe-macos-15-44350556911` 704 B (14449571330); `window-linux-44350556911` 577 B (14449165045); `window-probe-windows-2025-44350556911` 344 B (14447488551). `gate-macos-44350556911` 55,210 B (id 14453242461; not in the listing at the first read, about 17:11Z, because the job was still running; fetched at about 17:24Z). 6 artefacts in all. The window artefacts were not read.

## gate-linux (job 134964695406): conclusion failure
- Runner image (job log): `Image: ubuntu-24.04`, `Version: 20261111.127.1`. Cache: `Cache hit for: v0-rust-gate-linux-Linux-x64-f37a6dc5-6187dc6e`.
- Steps (job listing):
| step | conclusion | start–end (Z) | duration |
|---|---|---|---|
| Gate | **failure** | 16:44:17–16:51:41 | 7m24s |
| Gate report | success | 16:51:41–16:51:42 | 1s |
| Core crates (reported, not required) | success | 16:51:42–16:56:55 | 5m13s |
| Test list | success | 16:56:55–16:56:56 | 1s |
| Full test output (gate failed) | success (**ran**) | 16:56:56–17:04:34 | 7m38s |
| Disk | success | 17:04:34 | 0s |
| Upload reports | success | 17:04:34–17:04:36 | 2s |
- Gate report (`gate-report.json`, 5,840 B): `passed: false`, **`failed_step: fmt`**, 440.6 s. Steps that failed: `fmt` and `test`.
| step | status | seconds | detail |
|---|---|---|---|
| fmt | **fail** | 0.9 | ``run `cargo fmt --all`:`` then two `Diff in …/crates/kiln-text/tests/negative_t072.rs:6:` and `…:13:` hunks (rustfmt layout only; the second hunk's text contains the assert line with `"T-072 negative candidate: this message must appear in the record"`) |
| clippy | pass | 8.7 | no warnings (all targets, -D warnings) |
| layers | pass | 0.4 | 11 crates, 14 internal edges, layer table holds |
| ui-commands | pass | 0.03 | self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s) |
| hygiene | pass | 0.06 | reviews: 62 of 74 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 707 tracked file(s) clean (0 not UTF-8, skipped) |
| schema | pass | 4.4 | 30 commands; audit ok; docs/commands.json current |
| scorecard | pass | 151.1 | 64 items: 47 verified, 0 implemented, 0 failing, 17 planned. |
| test | **fail** | 272.8 | **counts: `678 passed, 1 failed, 0 ignored in 87 suites`**; kept text (28 lines): `test t072_negative_candidate_fails_on_purpose ... FAILED`, `---- t072_negative_candidate_fails_on_purpose stdout ----`, then 25 `     Running tests/…` lines of other binaries (the first is `guard_t032.rs`), the last `     Running tests/negative_t072.rs (target/debug/deps/negative_t072-e8d965f05b80fe8e)`, and `error: test failed, to rerun pass `-p kiln-text --test negative_t072``; no `negative candidate output` line and no panic message in the kept text |
| no-skip | pass | 2.2 | self-test ok (17 known findings); 205 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (19 pending), today = 2026-11-18 |
| fuzz, budgets | empty | 0.0 | no targets registered yet |
- "Core crates (reported, not required)": log line `CORE CRATES: exit status per package: kiln-text=101 kiln-syntax=0 kiln-search=0 xtask=0 (reported, not required)`.
- "Test list" step: 1 s. Status lines, verbatim: `TEST LIST: exit status 0` / `TEST LIST: the status is recorded, not used; the gate step decides the colour`. `test-list.txt` (77,179 B, 1,579 lines): 138 binary lines (130 `Running`, 8 `Doc-tests`), 1,176 lines ending `: test`; it holds `     Running tests/negative_t072.rs (target/debug/deps/negative_t072-e8d965f05b80fe8e)` and `t072_negative_candidate_fails_on_purpose: test`.
- Artefact files: `gate-report.json` 5,840 B; `core-crates.txt` 42,299 B; `test-output.txt` 98,518 B; `test-list.txt` 77,179 B. Zip 55,643 B.
- Disk (job log): `/dev/mapper/cow-root  145G   62G   83G  43% /`; `4.6G	target`.

### Linux: the job log of the step "Full test output (gate failed)", complete
From its first printed line (after the step's command header) to the `TEST OUTPUT: exit status 101` line (165 lines; the timestamp prefix of each log line and ANSI escapes are removed; the line after it in the log is `TEST OUTPUT: the status is recorded, not used; the gate step decides the colour`). The step's command prints, per the script shown in the log, "each failing test's `---- <name> stdout ----` block with the 20 lines after it" (the awk uses `n = 20` after the heading line). The printed block of the failing test is the lines between `     Running tests/negative_t072.rs (…)` and `error: test failed, to rerun pass …`.
```
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.69s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.57s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.95s
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 26.15s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 13.34s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.15s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.78s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.51s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.89s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.98s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.15s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.41s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.38s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 21.26s
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 24.29s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.64s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.45s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.20s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.91s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.47s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 79 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.93s
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.89s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.86s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.92s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 32.54s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 22.98s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.73s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.86s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 52.16s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.06s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.36s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.72s
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.50s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/negative_t072.rs (target/debug/deps/negative_t072-e8d965f05b80fe8e)
---- t072_negative_candidate_fails_on_purpose stdout ----
negative candidate output line 1
negative candidate output line 2
negative candidate output line 3
negative candidate output line 4
negative candidate output line 5
negative candidate output line 6
negative candidate output line 7
negative candidate output line 8
negative candidate output line 9
negative candidate output line 10
negative candidate output line 11
negative candidate output line 12
     Running tests/heading_shaped_line_inside_a_block.rs (target/debug/deps/none-0000)
negative candidate output line 14
negative candidate output line 15
negative candidate output line 16
negative candidate output line 17
negative candidate output line 18
negative candidate output line 19
negative candidate output line 20
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
error: test failed, to rerun pass `-p kiln-text --test negative_t072`
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.39s
test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
test result: ok. 51 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.92s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 138 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.02s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.62s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.55s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.48s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.22s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.54s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.55s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.25s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.12s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.89s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.87s
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.52s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.89s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.65s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.41s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 125.21s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.53s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
error: 1 target failed:
    `-p kiln-text --test negative_t072`
TEST OUTPUT: exit status 101
```

### Linux: `test-output.txt`, the failing test's complete stdout block (artefact, lines 1203–1234 of the ANSI-stripped file, from the `----` line to the `note: run with` line)
```
---- t072_negative_candidate_fails_on_purpose stdout ----
negative candidate output line 1
negative candidate output line 2
negative candidate output line 3
negative candidate output line 4
negative candidate output line 5
negative candidate output line 6
negative candidate output line 7
negative candidate output line 8
negative candidate output line 9
negative candidate output line 10
negative candidate output line 11
negative candidate output line 12
     Running tests/heading_shaped_line_inside_a_block.rs (target/debug/deps/none-0000)
negative candidate output line 14
negative candidate output line 15
negative candidate output line 16
negative candidate output line 17
negative candidate output line 18
negative candidate output line 19
negative candidate output line 20
negative candidate output line 21
negative candidate output line 22
negative candidate output line 23
negative candidate output line 24
negative candidate output line 25

thread 't072_negative_candidate_fails_on_purpose' (83642) panicked at crates/kiln-text/tests/negative_t072.rs:13:5:
assertion `left == right` failed: T-072 negative candidate: this message must appear in the record
  left: 2
 right: 3
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

### Linux: `core-crates.txt`, block `==== cargo test --locked --no-fail-fast -p kiln-text`
`==== kiln-text: exit status 101` (last line of the block); kiln-text totals 78 passed, 1 failed. The failing test's lines (the block `     Running tests/negative_t072.rs (target/debug/deps/negative_t072-8a05108293acb7e2)` … `error: test failed, to rerun pass `-p kiln-text --test negative_t072``):
```
     Running tests/negative_t072.rs (target/debug/deps/negative_t072-8a05108293acb7e2)

running 1 test
test t072_negative_candidate_fails_on_purpose ... FAILED

failures:

---- t072_negative_candidate_fails_on_purpose stdout ----
negative candidate output line 1
negative candidate output line 2
negative candidate output line 3
negative candidate output line 4
negative candidate output line 5
negative candidate output line 6
negative candidate output line 7
negative candidate output line 8
negative candidate output line 9
negative candidate output line 10
negative candidate output line 11
negative candidate output line 12
     Running tests/heading_shaped_line_inside_a_block.rs (target/debug/deps/none-0000)
negative candidate output line 14
negative candidate output line 15
negative candidate output line 16
negative candidate output line 17
negative candidate output line 18
negative candidate output line 19
negative candidate output line 20
negative candidate output line 21
negative candidate output line 22
negative candidate output line 23
negative candidate output line 24
negative candidate output line 25

thread 't072_negative_candidate_fails_on_purpose' (20757) panicked at crates/kiln-text/tests/negative_t072.rs:13:5:
assertion `left == right` failed: T-072 negative candidate: this message must appear in the record
  left: 2
 right: 3
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    t072_negative_candidate_fails_on_purpose

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p kiln-text --test negative_t072`
```
Other packages: `==== kiln-syntax: exit status 0`, `==== kiln-search: exit status 0`, `==== xtask: exit status 0`.

## gate-windows (job 134964695336): conclusion failure
- Runner image (job log): `Image: windows-2025-vs2026`, `Version: 20261103.298.1`. Cache: `No cache found.`
- Steps: Gate **failure** 16:44:16–16:52:28 (8m12s); Gate report success; Core crates (reported, not required) success 16:52:28–17:10:40 (18m12s); Workspace lib and T-073 tests success 7s; Test list success 17:10:47–17:10:54 (7s); kiln-workspace test targets success 8s; Full test output (gate failed) success (**ran**) 17:11:02–17:11:03 (1s); Disk success; Upload reports success; Post Cache skipped.
- Gate report (`gate-report.json`, 17,306 B): `passed: false`, **`failed_step: fmt`**, 468.4 s. Steps that failed: `fmt` (2.3 s; the same `negative_t072.rs` diff hunks as on Linux, with the path shown as `\\?\D:\a\…\crates\kiln-text\tests\negative_t072.rs`, including the assert line with the message), `clippy` (63.4 s), `scorecard` (347.1 s; first line `windows: 64 items; 158 tests ran; 3 registered absent here; page not compared (computed on Linux)`), `test` (1.1 s; `0 passed, 0 failed, 0 ignored in 0 suites`; kept text 27 lines of `unix`/`os` compile errors, ending `error: could not compile `kiln` (test "acceptance_t055") due to 2 previous errors`). Passed: layers, ui-commands, hygiene, schema, no-skip. The `test` step's kept text has no `t072_negative` name and no message.
- "Core crates" step: `core-crates.txt` (42,834 B) has the failing test (see below); kiln-text 78 passed, 1 failed.
- "Test list" step: 7 s. Status lines, verbatim: `TEST LIST: exit status 101` / `TEST LIST: the status is recorded, not used; the gate step decides the colour`. `test-list.txt` (9,029 B, 143 lines): compile errors only; **0 binary lines, 0 test names**; 3 `could not compile` lines (`window_t065` 8 errors, `script_t055` 1, `acceptance_t055` 2); no `t072_negative` name.
- Artefact files: `gate-report.json` 17,306 B; `core-crates.txt` 42,834 B; `test-output.txt` 10,832 B; `test-list.txt` 9,029 B; `workspace-tests.txt` 7,134 B; `windows-tests.txt` 62,809 B. Zip 25,481 B.
- Disk (job log): `D:              150G  6.6G  144G   5% /d`; `3.5G	target`.

### Windows: the job log of the step "Full test output (gate failed)", complete
From its first printed line to the `TEST OUTPUT: exit status 101` line (19 lines; timestamps and ANSI escapes removed; the next log line is `TEST OUTPUT: the status is recorded, not used; the gate step decides the colour`). The step printed only compile-error lines: the workspace did not compile its test targets, so no test of `kiln-text` ran in this step and there is no `---- … stdout ----` block.
```
error[E0433]: cannot find `unix` in `os`
error[E0433]: cannot find `unix` in `os`
error[E0599]: no associated function or constant named `from_vec` found for struct `OsString` in the current scope
error[E0277]: the size for values of type `[OsString]` cannot be known at compilation time
error[E0277]: the size for values of type `[OsString]` cannot be known at compilation time
error[E0277]: the size for values of type `[OsString]` cannot be known at compilation time
error[E0277]: the size for values of type `[OsString]` cannot be known at compilation time
error[E0599]: no associated function or constant named `from_bytes` found for struct `OsStr` in the current scope
error[E0277]: the size for values of type `[OsString]` cannot be known at compilation time
error[E0277]: the size for values of type `[OsString]` cannot be known at compilation time
error[E0599]: no associated function or constant named `from_bytes` found for struct `OsStr` in the current scope
error[E0433]: cannot find `unix` in `os`
error: could not compile `kiln` (test "window_t065") due to 8 previous errors
error: could not compile `kiln` (test "acceptance_t065") due to 3 previous errors
error[E0433]: cannot find `unix` in `os`
error[E0433]: cannot find `unix` in `os`
error: could not compile `kiln` (test "script_t055") due to 1 previous error
error: could not compile `kiln` (test "acceptance_t055") due to 2 previous errors
TEST OUTPUT: exit status 101
```

### Windows: `test-output.txt` (10,832 B, 172 lines)
Four `error: could not compile` lines (`window_t065` 8, `acceptance_t065` 3, `script_t055` 1, `acceptance_t055` 2); 0 lines `Running`, `test result:` or `FAILED`; no `t072_negative` name and no message. **The failing test's stdout block is not in this artefact.**

### Windows: `core-crates.txt`, block `==== cargo test --locked --no-fail-fast -p kiln-text`
`==== kiln-text: exit status 101`; the failing test's lines in the artefact (this is where the block appears on Windows):
```
     Running tests\negative_t072.rs (target\debug\deps\negative_t072-8e589ee09183a195.exe)

running 1 test
test t072_negative_candidate_fails_on_purpose ... FAILED

failures:

---- t072_negative_candidate_fails_on_purpose stdout ----
negative candidate output line 1
negative candidate output line 2
negative candidate output line 3
negative candidate output line 4
negative candidate output line 5
negative candidate output line 6
negative candidate output line 7
negative candidate output line 8
negative candidate output line 9
negative candidate output line 10
negative candidate output line 11
negative candidate output line 12
     Running tests/heading_shaped_line_inside_a_block.rs (target/debug/deps/none-0000)
negative candidate output line 14
negative candidate output line 15
negative candidate output line 16
negative candidate output line 17
negative candidate output line 18
negative candidate output line 19
negative candidate output line 20
negative candidate output line 21
negative candidate output line 22
negative candidate output line 23
negative candidate output line 24
negative candidate output line 25

thread 't072_negative_candidate_fails_on_purpose' (1116) panicked at crates\kiln-text\tests\negative_t072.rs:13:5:
assertion `left == right` failed: T-072 negative candidate: this message must appear in the record
  left: 2
 right: 3
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    t072_negative_candidate_fails_on_purpose

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p kiln-text --test negative_t072`
```
Other packages: `==== kiln-syntax: exit status 0`, `==== kiln-search: exit status 0`, `==== xtask: exit status 0`.

## gate-macos (job 134964695395)
- Runner image (job log): `Image: macos-15-arm64`, `Version: 20261104.0514.1`; Rust versions listed by the cache step `1.97.0 aarch64-apple-darwin 2d8144b7880597b6e6d3dfd63a9a9efae3f533d3` and `1.99.1 aarch64-apple-darwin 58cf8f9c35b355199727627d07ae34a4e2846d67`; `Cache hit for: v0-rust-gate-macos-Darwin-arm64-03a9c7df-6187dc6e`.
- Steps (job listing): Gate **failure** 16:48:44–16:59:47 (11m03s); Gate report success; Core crates (reported, not required) success 16:59:47–17:09:37 (9m50s); Test list success 17:09:37–17:09:42 (5s); Full test output (gate failed) success (**ran**) 17:09:42–17:22:55 (13m13s); Disk success; Upload reports success; Post Cache skipped.
- Gate report (`gate-report.json`, 5,883 B): `passed: false`, **`failed_step: fmt`**, 653.9 s. Steps that failed: `fmt` (1.1 s; the same `negative_t072.rs` diff hunks as on Linux, including the assert line with the message) and `test` (399.6 s): **counts `670 passed, 1 failed, 0 ignored in 87 suites`**; kept text (28 lines) names `t072_negative_candidate_fails_on_purpose` (`test … ... FAILED`, `---- … stdout ----`) followed by `     Running …` lines and `error: test failed…`; no `negative candidate output` line and no panic message in the kept text. Passed: clippy, layers, ui-commands, hygiene (62 of 74 review files; 707 tracked files), schema, scorecard (`macos: 64 items; 216 tests ran; 3 registered absent here; page not compared (computed on Linux)`), no-skip (205 files).
- "Core crates" log line: `CORE CRATES: exit status per package: kiln-text=101 kiln-syntax=0 kiln-search=0 xtask=0 (reported, not required)`.
- "Test list" step: 5 s. Status lines, verbatim: `TEST LIST: exit status 0` / `TEST LIST: the status is recorded, not used; the gate step decides the colour`. `test-list.txt` (76,221 B, 1,562 lines): 138 binary lines (130 `Running`, 8 `Doc-tests`), 1,159 lines ending `: test`; it holds `t072_negative_candidate_fails_on_purpose: test`.
- Artefact files: `gate-report.json` 5,883 B; `core-crates.txt` 42,130 B; `test-output.txt` 97,612 B; `test-list.txt` 76,221 B. Zip 55,210 B (artefact id 14453242461, created 17:22:59Z).
- Disk (job log): `/dev/disk2s5   320Gi   261Gi    41Gi    87%    2.3M  433M    1%   /System/Volumes/Data`; `3.2G	target`.

### macOS: the job log of the step "Full test output (gate failed)", complete
From its first printed line to the `TEST OUTPUT: exit status 101` line (165 lines; timestamps and ANSI escapes removed; the next log line is `TEST OUTPUT: the status is recorded, not used; the gate step decides the colour`). The printed block of the failing test is the lines between `     Running tests/negative_t072.rs (…)` and `error: test failed, to rerun pass …`.
```
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.84s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.73s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 21.12s
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 33.47s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 13.14s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.72s
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.80s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.43s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.34s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.40s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.10s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.11s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.81s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.63s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.61s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.56s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.81s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 14.42s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 32.53s
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 30.16s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.69s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.44s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.37s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.18s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.38s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.91s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.14s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.34s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.65s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.55s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.56s
test result: ok. 79 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.64s
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.30s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.09s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.22s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 48.43s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 35.45s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.85s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.02s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 82.17s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.47s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.42s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.74s
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.30s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/negative_t072.rs (target/debug/deps/negative_t072-bd78b11f179dee1c)
---- t072_negative_candidate_fails_on_purpose stdout ----
negative candidate output line 1
negative candidate output line 2
negative candidate output line 3
negative candidate output line 4
negative candidate output line 5
negative candidate output line 6
negative candidate output line 7
negative candidate output line 8
negative candidate output line 9
negative candidate output line 10
negative candidate output line 11
negative candidate output line 12
     Running tests/heading_shaped_line_inside_a_block.rs (target/debug/deps/none-0000)
negative candidate output line 14
negative candidate output line 15
negative candidate output line 16
negative candidate output line 17
negative candidate output line 18
negative candidate output line 19
negative candidate output line 20
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
error: test failed, to rerun pass `-p kiln-text --test negative_t072`
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.74s
test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.52s
test result: ok. 51 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.77s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.38s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.03s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.13s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 136 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 25.45s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.05s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.33s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 31.22s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.87s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.88s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.34s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 55.74s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.70s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.77s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.87s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.75s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.96s
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.23s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 13.27s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.63s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.34s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.75s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 135.52s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.42s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
error: 1 target failed:
    `-p kiln-text --test negative_t072`
TEST OUTPUT: exit status 101
```

### macOS: `test-output.txt`, the failing test's complete stdout block (artefact, lines 1197–1228 of the ANSI-stripped file, from the `----` line to the `note: run with` line)
```
---- t072_negative_candidate_fails_on_purpose stdout ----
negative candidate output line 1
negative candidate output line 2
negative candidate output line 3
negative candidate output line 4
negative candidate output line 5
negative candidate output line 6
negative candidate output line 7
negative candidate output line 8
negative candidate output line 9
negative candidate output line 10
negative candidate output line 11
negative candidate output line 12
     Running tests/heading_shaped_line_inside_a_block.rs (target/debug/deps/none-0000)
negative candidate output line 14
negative candidate output line 15
negative candidate output line 16
negative candidate output line 17
negative candidate output line 18
negative candidate output line 19
negative candidate output line 20
negative candidate output line 21
negative candidate output line 22
negative candidate output line 23
negative candidate output line 24
negative candidate output line 25

thread 't072_negative_candidate_fails_on_purpose' (318866) panicked at crates/kiln-text/tests/negative_t072.rs:13:5:
assertion `left == right` failed: T-072 negative candidate: this message must appear in the record
  left: 2
 right: 3
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

### macOS: `core-crates.txt`, block `==== cargo test --locked --no-fail-fast -p kiln-text`
`==== kiln-text: exit status 101`; kiln-text totals 78 passed, 1 failed. The failing test's lines:
```
     Running tests/negative_t072.rs (target/debug/deps/negative_t072-eeeb203127ac5f93)

running 1 test
test t072_negative_candidate_fails_on_purpose ... FAILED

failures:

---- t072_negative_candidate_fails_on_purpose stdout ----
negative candidate output line 1
negative candidate output line 2
negative candidate output line 3
negative candidate output line 4
negative candidate output line 5
negative candidate output line 6
negative candidate output line 7
negative candidate output line 8
negative candidate output line 9
negative candidate output line 10
negative candidate output line 11
negative candidate output line 12
     Running tests/heading_shaped_line_inside_a_block.rs (target/debug/deps/none-0000)
negative candidate output line 14
negative candidate output line 15
negative candidate output line 16
negative candidate output line 17
negative candidate output line 18
negative candidate output line 19
negative candidate output line 20
negative candidate output line 21
negative candidate output line 22
negative candidate output line 23
negative candidate output line 24
negative candidate output line 25

thread 't072_negative_candidate_fails_on_purpose' (65107) panicked at crates/kiln-text/tests/negative_t072.rs:13:5:
assertion `left == right` failed: T-072 negative candidate: this message must appear in the record
  left: 2
 right: 3
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    t072_negative_candidate_fails_on_purpose

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p kiln-text --test negative_t072`
```

## Window jobs (listing only)
`window-linux`: success; `window-probe (macos-15)`: success (runner assigned); `window-probe (windows-2025)`: failure at step Window probe (Build kiln success). Their artefacts were not read.

## Does the record name the failing test and show its message?
(What each platform's record, as read above, contains. No judgement.)
| platform | names `t072_negative_candidate_fails_on_purpose` | shows "T-072 negative candidate: this message must appear in the record" | printed block of the "Full test output (gate failed)" step |
|---|---|---|---|
| Linux | yes: gate report `test` step, the step's printed block, `test-output.txt`, `core-crates.txt`, `test-list.txt` | yes in `test-output.txt` (full block with panic) and in `core-crates.txt`; also in the gate report's `fmt` diff text; **not** in the gate report's `test` kept text and **not** in the step's printed block | printed: the heading, lines 1–12, the heading-shaped `Running tests/heading_shaped_line_inside_a_block.rs (…)` line, lines 14–20, `test result: FAILED…`, `error: test failed…`. **Not printed: lines 21–25 and the panic message.** after the heading-shaped line: lines 14–20 (7 of the 12 lines 14–25) |
| Windows | yes: `core-crates.txt` (full block); not in the step's printed output, not in `test-output.txt`, not in the gate report's `test` text | yes in `core-crates.txt` (full block incl. all 25 lines and the panic message) and in the gate report's `fmt` diff text; not in the step's printed output | step printed compile-error lines only; no block |
| macOS | yes: gate report `test` step, the step's printed block, `test-output.txt`, `core-crates.txt`, `test-list.txt` | yes in `test-output.txt` (full block with panic) and in `core-crates.txt`; also in the gate report's `fmt` diff text; **not** in the gate report's `test` kept text and **not** in the step's printed block | printed: the heading, lines 1–12, the heading-shaped `Running tests/heading_shaped_line_inside_a_block.rs (…)` line, lines 14–20, `test result: FAILED…`, `error: test failed…`. **Not printed: lines 21–25 and the panic message.** |

## Not read
- Job log of `gate-macos` beyond the image header, cache, `CORE CRATES`/`TEST LIST` lines, disk lines and the "Full test output (gate failed)" step; the macOS `workspace`/other steps' output.
- The window artefacts and window job logs; job logs other than the "Full test output (gate failed)" steps, the image header, cache and disk lines.
- `workspace-tests.txt` and `windows-tests.txt` of the Windows artefact were listed but not read for this record (they are the same size as in the round-5 artefact for `workspace-tests.txt`, 7,134 B).
- Why the Linux gate's `test` step reports 87 suites and 678 passed (against 1,176 names in `test-list.txt`): not examined.
