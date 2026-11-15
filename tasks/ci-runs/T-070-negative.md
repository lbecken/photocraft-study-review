# CI run 43764233597 (platforms.yml, negative candidate) on ci/T-070-negative @ 4bc231c3755d583707b8031bdfecb4d93fbb628f

Extracted from the runs API, the jobs API and job logs only (tools `actions_list`, `get_job_logs`). Times UTC as the API gives them. Wall = job completed_at - started_at (jobs API).

## Run header
- id: 43764233597 (run_number 2, attempt 1), event push, workflow `platforms`
- branch: ci/T-070-negative; commit 4bc231c3755d583707b8031bdfecb4d93fbb628f
- commit title: "T-070 AC-6: negative candidate, one test that fails on purpose (never merged)" (Task: T-070, Role: reviewer)
- created_at 2026-11-15T21:32:21Z (run_started_at same); updated_at 2026-11-15T21:58:13Z
- status completed, conclusion failure

## Job table
| job | id | conclusion | started | completed | wall | first failing step |
|---|---|---|---|---|---|---|
| gate-linux | 132965624371 | failure | 21:32:24Z | 21:48:02Z | 15m38s | Gate (21:32:43Z-21:40:11Z) |
| gate-macos | 132965624475 | failure | 21:32:24Z | 21:58:13Z | 25m49s | Gate (21:32:40Z-21:40:31Z) |
| gate-windows | 132965624442 | failure | 21:32:24Z | 21:41:07Z | 8m43s | Gate (21:33:00Z-21:36:39Z) |
| window-linux | 132965624083 | success | 21:32:23Z | 21:34:18Z | 1m55s | none |
| window-probe (macos-15) | 132965624433 | success | 21:32:24Z | 21:34:38Z | 2m14s | none |
| window-probe (windows-2025) | 132965624389 | failure | 21:32:24Z | 21:34:54Z | 2m30s | Build kiln (21:32:53Z-21:34:48Z) |

In all three gate jobs the steps after Gate (Gate report, Core crates, Full test output (gate failed), Disk, Upload reports) have conclusion success; "Post Cache cargo registry and target" is skipped in the three gate jobs and in window-probe (windows-2025).

## gate-linux (failure; Gate step failed)
Logs read: tail 400 lines, then tail 700 lines (job log lines are long; the second read was needed to reach the gate summary).

Gate summary lines (step "Gate"):
```
PASS  scorecard     149.3s  64 items: 44 verified, 0 implemented, 0 failing, 20 planned.
FAIL  test          234.3s  622 passed, 1 failed, 0 ignored in 81 suites
           Running tests/negative_t070.rs (target/debug/deps/negative_t070-b9d846510040e33f)
      error: test failed, to rerun pass `-p kiln-text --test negative_t070`
GATE FAILED in 438.9s: test
```
The other gate steps in that summary are PASS or EMPTY (fmt, clippy 33.8s, layers, ui-commands, hygiene, schema, scorecard, no-skip PASS; fuzz, budgets EMPTY).
Gate report JSON ("Gate report" step) test detail: `"622 passed, 1 failed, 0 ignored in 81 suites\ntest t070_negative_candidate_fails_on_purpose ... FAILED\n---- t070_negative_candidate_fails_on_purpose stdout ----` ... (the stdout section holds only `Running` lines, no panic message).

Step "Core crates (reported, not required)":
```
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
==== kiln-text: exit status 101
CORE CRATES: exit status per package: kiln-text=101 kiln-syntax=0 kiln-search=0 xtask=0 (reported, not required)
```
The kiln-text results before that FAILED line: 31, 21, 6, 3 passed (all ok).

Panic sentence ("the kestrel counted nine quartz lanterns at dawn"): does NOT appear in what I read (tail 400 and tail 700 lines of the job log; text search for "kestrel", "T070 NEG" and "panicked" in the 700-line tail found nothing). The test name appears: `t070_negative_candidate_fails_on_purpose` (step Gate report) and `negative_t070` (steps Gate, Gate report). The "Full test output (gate failed)" step prints through `head -200` of a grep and its printed portion in the tail has only `test result:` lines, then `TEST OUTPUT: exit status 101 (recorded; the gate step decides the colour)`.

Step "Disk":
```
Filesystem            Size  Used Avail Use% Mounted on
/dev/mapper/cow-root  145G   66G   79G  46% /
DISK: du -sh target
8.7G	target
```

## gate-macos (failure; Gate step failed)
Logs read: tail 400 lines, then tail 800 lines (the gate summary precedes the 400-line window).

Gate summary lines (step "Gate"):
```
FAIL  scorecard     350.7s  7 scorecard problems:
FAIL  test            4.5s  25 passed, 1 failed, 0 ignored in 2 suites
GATE FAILED in 451.5s: scorecard, test
```
(fmt, clippy 53.4s, layers, ui-commands, hygiene, schema, no-skip PASS; fuzz, budgets EMPTY.) Gate report JSON: `"failed_step": "scorecard"`, `"seconds": 451.506876333`. The seven scorecard problems are listed in the JSON detail (same six test names as round 1 plus "docs/scorecard.md is stale; run `cargo xtask scorecard` and commit it"); the test step detail names `ac8_stdout_write_failure_exits_1`.

Step "Core crates (reported, not required)":
```
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
==== kiln-text: exit status 101
test result: FAILED. 116 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 23.22s
==== xtask: exit status 101
CORE CRATES: exit status per package: kiln-text=101 kiln-syntax=0 kiln-search=0 xtask=101 (reported, not required)
```
Panic sentence: does NOT appear in what I read (tail 400 and tail 800 lines; text search for "kestrel", "T070", "negative_t070" in the 800-line tail found nothing). The "Full test output (gate failed)" step (21:44:26Z-21:58:08Z) prints a `head -200` prefix whose failing tests are the same /dev/full, no-display timeout, link-loop and depth-bound tests seen in round 1; no kiln-text failure panic line is in that prefix.

Step "Disk":
```
Filesystem      Size    Used   Avail Capacity iused ifree %iused  Mounted on
/dev/disk2s5   320Gi   262Gi    40Gi    87%    2.3M  423M    1%   /System/Volumes/Data
DISK: du -sh target
4.4G	target
```

## gate-windows (failure; Gate step failed)
Logs read: tail 400 lines (this reached back to the Cache step and covered the whole Gate step output).

Gate summary lines (step "Gate"):
```
FAIL  clippy         22.2s  note: found an item that was configured out
FAIL  schema         44.1s  kiln-cli commands --audit failed:
FAIL  scorecard     101.9s  error: building test executables failed ():
FAIL  test            9.3s  0 passed, 0 failed, 0 ignored in 0 suites
GATE FAILED in 193.2s: clippy, schema, scorecard, test
```
(fmt, layers, ui-commands, hygiene, no-skip PASS; fuzz, budgets EMPTY.) Compile errors as printed in the Gate step details: clippy `error: could not compile `kiln-workspace` (lib) due to 6 previous errors`; scorecard `could not compile `kiln-workspace` (lib test) due to 34 previous errors`; test `error: could not compile `xtask` (bin "xtask" test) due to 4 previous errors`. These match the round-1 pattern (std::os::unix items in kiln-workspace and xtask).

Step "Core crates (reported, not required)":
```
==== kiln-text: exit status 101
==== kiln-syntax: exit status 0
==== kiln-search: exit status 0
==== xtask: exit status 101
CORE CRATES: exit status per package: kiln-text=101 kiln-syntax=0 kiln-search=0 xtask=101 (reported, not required)
```
Note: kiln-text=101 here; the log I read shows no `test result:` line for kiln-text in this job, so I cannot tell from it whether 101 comes from the panic or from a build failure of the dependency (kiln-text is not shown to depend on kiln-workspace in what I read). Not determined.

Panic sentence: does NOT appear in what I read (tail 400 lines; text search for "kestrel", "T070", "negative_t070", "panicked" found no match except the grep command text itself).

Step "Full test output (gate failed)" (21:40:54Z-21:41:02Z): its filtered output (grep for `test result:`, `panicked at`, `---- ... stdout ----`) printed no lines; the only output is `TEST OUTPUT: exit status 101 (recorded; the gate step decides the colour)`. So that step shows neither compile errors nor the sentence. Compile errors appear in the Gate step details instead: first error line in clippy detail `error[E0599]: no method named `dev` found for struct `std::fs::Metadata` in the current scope`; error count lines `could not compile `kiln-workspace` (lib) due to 6 previous errors` (clippy and schema), 34 (scorecard), `xtask` 4 (test).

Step "Disk":
```
Filesystem      Size  Used Avail Use% Mounted on
D:              150G  5.5G  145G   4% /d
DISK: du -sh target
2.5G	target
```

## window-linux (success)
Logs read: tail 200 lines. Step "Window procedure (T-065) under Xvfb" conclusion success; "Installed X and GL packages (for the record)" success.
```
WINDOW CASES PASS
AC-3: exit=2 stdout_bytes=0 stderr_lines=1 panicked=0 (pass: 2 0 1 0)
  stderr: kiln: no window: Library libxkbcommon-x11.so could not be loaded.
AC-3 PASS
missing script: exit=2 stdout_bytes=0 (pass: 2 0; 124 = a window opened)
plain none: drawn_and_stable=yes still_running=yes
plain none2: drawn_and_stable=yes still_running=yes
plain apath: drawn_and_stable=yes still_running=yes
  two runs without a path: same screen
PLAIN PASS: the start path changes the screen
```
Cases printed: ac2, ac2x3 (exit window=3 headless=3), dblclick, scroll100, ctrlq, noroot; each with `files EQUAL`.

## window-probe (macos-15) (success)
Logs read: tail 120 lines. Step "Window probe" success (21:33:56Z-21:34:00Z).
```
PROBE: exit status 0
PROBE: stdout     1864 bytes, 3 lines
--- stdout
...(3 JSON lines: s1 snapshot, {"id":1,"ok":true,"result":{"docs":[]}}, s2 snapshot)
--- stderr
--- end
```
Also `PROBE OS: macos-15 Darwin ... 24.6.0 ... arm64`, `ProductVersion: 15.8.2`, `BuildVersion: 24H232`. stderr is empty.

## window-probe (windows-2025) (failure at Build kiln)
Logs read: tail 120 lines. Build kiln failed with `##[error]Process completed with exit code 101.` after `error: could not compile `kiln-workspace` (lib) due to 6 previous errors` (last error visible: `error[E0599]: no method named `mode` found for mutable reference `&mut OpenOptions`` at crates\kiln-workspace\src\disk.rs:361:14). The "Window probe" step ran (conclusion success, 21:34:48Z-21:34:48Z) and recorded:
```
PROBE OS: windows-2025 MINGW64_NT-10.0-26100 runnervmm7ud1 3.6.10-5a1665c8.x86_64 2026-09-28 10:40 UTC x86_64 Msys
PROBE: not run (cargo build -p kiln: failure)
```
No probe stdout or stderr (probe not run).

## Not read
- The panic sentence "T070 NEGATIVE CANDIDATE: the kestrel counted nine quartz lanterns at dawn" is not in any log text I read, for any of the three gate jobs. Reads: gate-linux tail 400 and tail 700 lines; gate-macos tail 400 and tail 800; gate-windows tail 400. In every job the test-stdout panic message is filtered out of the printed parts (the gate step keeps only the test-name header and `Running` lines; the full-output step prints `head -200` of a filtered grep). The artifacts (core-crates.txt, test-output.txt per gate job) were not downloaded.
- gate-windows: why kiln-text exited 101 (no kiln-text `test result:` line in what I read).
- gate-windows: no compile errors in the "Full test output (gate failed)" step (filtered output is empty); counts come from the Gate step details.
- gate-macos: the full names of the 3 failing xtask tests (not printed).
- Log start portions of steps before the Gate summary (toolchain, cache) were not read.
