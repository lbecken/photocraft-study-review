# CI run 43752760283 (platforms.yml, run 1) on ci/T-070 @ cd1121b9e69a949bddd0d91afce5b96887747894

Extracted from job logs and the jobs API only. Timestamps trimmed. Wall = job completed_at - started_at.

## Job table
| job | id | conclusion | failed step | started | completed | wall |
|---|---|---|---|---|---|---|
| gate-linux | 132930159718 | success | - | 19:30:10Z | 19:45:44Z | 15m34s |
| window-linux | 132930159618 | success | - | 19:30:10Z | 19:32:23Z | 2m13s |
| window-probe (macos-15) | 132930159717 | success | - | 19:30:12Z | 19:32:38Z | 2m26s |
| window-probe (windows-2025) | 132930159658 | failure | Build kiln (19:30:38Z-19:32:13Z) | 19:30:10Z | 19:32:18Z | 2m08s |
| gate-windows | 132930159360 | failure | Gate (19:30:39Z-19:34:20Z) | 19:30:10Z | 19:39:25Z | 9m15s |
| gate-macos | 132930159628 | failure | Gate (19:30:21Z-19:36:27Z) | 19:30:10Z | 19:51:30Z | 21m20s |

gate-macos was still running at first look (step "Full test output (gate failed)"); it later finished: that step ran 19:39:59Z-19:51:25Z.
Its log (HTTP 404 while running) became readable only after completion.

## Artefacts
gate-linux-43752760283 8822 B; gate-macos-43752760283 29358 B; gate-windows-43752760283 8377 B; window-linux-43752760283 577 B; window-probe-macos-15-43752760283 677 B; window-probe-windows-2025-43752760283 292 B. Not downloaded.

## gate-linux (success)
```
cargo 1.97.0 (c980f4866 2026-06-30)
clippy 0.1.97 (2d8144b788 2026-07-07)
rustfmt 1.9.0-stable (2d8144b788 2026-07-07)
PASS  fmt             0.7s  formatted
PASS  clippy         39.0s  no warnings (all targets, -D warnings)
PASS  layers          1.6s  11 crates, 14 internal edges, layer table holds
PASS  ui-commands     0.0s  self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s)
PASS  hygiene         0.1s  reviews: 42 of 54 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 592 tracked file(s) clean (0 not UTF-8, skipped)
PASS  schema         22.0s  30 commands; audit ok; docs/commands.json current
PASS  scorecard     188.2s  64 items: 44 verified, 0 implemented, 0 failing, 20 planned.
PASS  test          394.2s  990 passed, 0 failed, 0 ignored in 120 suites
PASS  no-skip         1.7s  self-test ok; 185 files scanned, 0 allowlisted skip(s), today = 2026-11-15
EMPTY fuzz            0.0s  no targets registered yet (xtask/fuzz.toml)
EMPTY budgets         0.0s  no targets registered yet (xtask/budgets.toml)
GATE PASSED in 647.6s (11 steps; 2 step(s) EMPTY (nothing registered): fuzz, budgets)
CORE CRATES: exit status per package: kiln-text=0 kiln-syntax=0 kiln-search=0 xtask=0 (reported, not required)
DISK: df -h of the workspace's volume
Filesystem      Size  Used Avail Use% Mounted on
/dev/root       145G   69G   76G  48% /
DISK: du -sh target
8.6G	target
```
Report JSON: "passed": true, "failed_step": null, "seconds": 647.569021122; all details equal the PASS lines above.
Core crates test result lines (all `ok`, 0 failed): kiln-text 31,21,6,3,17,0 passed; kiln-syntax 22,9,6,5,11,0; kiln-search 17,19,3,5,11,0; xtask 119,16,4,11,6,0,2,5,9,11,5,4,0,11,3,1 (per binary, in order). "Full test output (gate failed)" step: skipped.

## window-linux (success)
```
CASE ac2: exit window=0 headless=0
  first snapshot Title src/b.rs and Message not_found: True
  lines 12 12 | without counts: EQUAL | with counts: EQUAL
  files EQUAL
CASE ac2x3: exit window=3 headless=3
  lines 13 13 | without counts: EQUAL | with counts: EQUAL
  files EQUAL
CASE dblclick: exit window=0 headless=0
  lines 7 7 | without counts: EQUAL | with counts: EQUAL
  files EQUAL
CASE scroll100: exit window=0 headless=0
  lines 4 4 | without counts: EQUAL | with counts: EQUAL
  files EQUAL
CASE ctrlq: exit window=0 headless=0
  lines 9 9 | without counts: EQUAL | with counts: EQUAL
  files EQUAL
CASE noroot: exit window=0 headless=0
  lines 12 12 | without counts: EQUAL | with counts: EQUAL
  files EQUAL
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
Package record:
```
COMMAND xvfb-run: /usr/bin/xvfb-run
COMMAND Xvfb: /usr/bin/Xvfb
dpkg-query: no packages found matching libxkbcommon-x11-0
dpkg-query: no packages found matching libxcb-xkb1
dpkg-query: no packages found matching libegl-mesa0
PACKAGE libgl1-mesa-dri 25.2.8-0ubuntu0.24.04.4 ii 
PACKAGE libglx-mesa0 25.2.8-0ubuntu0.24.04.4 ii 
PACKAGE mesa-libgallium 25.2.8-0ubuntu0.24.04.4 ii 
PACKAGE xvfb 2:21.1.12-1ubuntu1.9 ii 
dpkg-query: exit status 1
APT LISTS: 58 files in /var/lib/apt/lists
	libGLX_mesa.so.0 (libc6,x86-64) => /lib/x86_64-linux-gnu/libGLX_mesa.so.0
```
Reading: libxkbcommon-x11-0 and libxcb-xkb1 are reported "no packages found"; xvfb 2:21.1.12-1ubuntu1.9 installed (ii); Mesa packages libgl1-mesa-dri, libglx-mesa0, mesa-libgallium 25.2.8-0ubuntu0.24.04.4 ii; libegl-mesa0 not found; only libGLX_mesa.so.0 appears in ldconfig (no LDCONFIG: none line). AC-3 stderr line names libxkbcommon-x11.so as not loadable.

## window-probe (macos-15) (success)
```
PROBE OS: macos-15 Darwin ... 24.6.0 ... arm64 (sw_vers: macOS 15.8.2, build 24H232)
PROBE: exit status 0
PROBE: stdout     1864 bytes, 3 lines
```
stdout first 10 lines (3 lines in all; each is one JSON line, shortened here):
```
{"id":"s1","ok":true,"result":{"frames":3,"root":{... "role":"window"}}}
{"id":1,"ok":true,"result":{"docs":[]}}
{"id":"s2","ok":true,"result":{"frames":3,"root":{... "role":"window"}}}
```
Snapshot content: window > pane > containers with a label "Path", a text_input labelled "Path" (value ""), a button "Open", a label "Message", a list labelled "Documents". stderr: empty (`--- stderr` followed directly by `--- end`).

## window-probe (windows-2025) (failure at Build kiln)
```
PROBE OS: windows-2025 MINGW64_NT-10.0-26100 runnervmm7ud1 3.6.10-5a1665c8.x86_64 2026-09-28 10:40 UTC x86_64 Msys
PROBE: not run (cargo build -p kiln: failure)
```
No probe stdout/stderr (probe not run). First 20 error lines of Build kiln:
```
error[E0433]: cannot find `unix` in `os`
  --> crates\kiln-workspace\src\disk.rs:86:14
   |
86 | use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
   |              ^^^^ could not find `unix` in `os`
   |
note: found an item that was configured out
  --> /rustc/2d8144b7880597b6e6d3dfd63a9a9efae3f533d3/library\std\src\os\mod.rs:29:4
   |
   = note: the item is gated here
note: found an item that was configured out
  --> /rustc/2d8144b7880597b6e6d3dfd63a9a9efae3f533d3/library\std\src\os\mod.rs:84:40
   |
   = note: the item is gated here

error[E0599]: no method named `dev` found for struct `Metadata` in the current scope
   --> crates\kiln-workspace\src\disk.rs:189:36
    |
189 |                     root_id: (held.dev(), held.ino()),
    |                                    ^^^ method not found in `Metadata`
```
Full error set (6 errors, all in crates\kiln-workspace\src\disk.rs): E0433 line 86 (std::os::unix); E0599 `dev` 189 and 213; `ino` 189 and 213; `mode` on OpenOptions 361. Final lines:
```
Some errors have detailed explanations: E0433, E0599.
For more information about an error, try `rustc --explain E0433`.
error: could not compile `kiln-workspace` (lib) due to 6 previous errors
warning: build failed, waiting for other jobs to finish...
##[error]Process completed with exit code 101.
```

## gate-windows (Gate failed; steps clippy, schema, scorecard, test)
```
cargo 1.97.0 (c980f4866 2026-06-30)
clippy 0.1.97 (2d8144b788 2026-07-07)
rustfmt 1.9.0-stable (2d8144b788 2026-07-07)
gate: 11 steps: fmt, clippy, layers, ui-commands, hygiene, schema, scorecard, test, no-skip, fuzz, budgets
PASS  fmt             2.0s  formatted
FAIL  clippy         33.7s  note: found an item that was configured out
PASS  layers         10.9s  11 crates, 14 internal edges, layer table holds
PASS  ui-commands     0.0s  self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s)
PASS  hygiene         0.2s  reviews: 42 of 54 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 592 tracked file(s) clean (0 not UTF-8, skipped)
FAIL  schema         42.4s  kiln-cli commands --audit failed:
FAIL  scorecard      99.9s  error: building test executables failed ():
FAIL  test            2.9s  0 passed, 0 failed, 0 ignored in 0 suites
PASS  no-skip         2.6s  self-test ok; 185 files scanned, 0 allowlisted skip(s), today = 2026-11-15
EMPTY fuzz            0.0s  no targets registered yet (xtask/fuzz.toml)
EMPTY budgets         0.0s  no targets registered yet (xtask/budgets.toml)
GATE FAILED in 194.7s: clippy, schema, scorecard, test
```
Report JSON: "passed": false, "failed_step": "clippy", "seconds": 194.7002608. Step details (the failing ones are compiler output with ANSI codes; the report keeps a tail of it):

Gate step: first 30 lines of the clippy failure output as printed (lines 229-258 of the job log region):
```
FAIL  clippy         33.7s  note: found an item that was configured out
        --> /rustc/2d8144b7880597b6e6d3dfd63a9a9efae3f533d3/library\std\src\os\mod.rs:29:4
         |
         = note: the item is gated here
      note: found an item that was configured out
        --> /rustc/2d8144b7880597b6e6d3dfd63a9a9efae3f533d3/library\std\src\os\mod.rs:84:40
         |
         = note: the item is gated here
      
      error[E0599]: no method named `dev` found for struct `std::fs::Metadata` in the current scope
         --> crates\kiln-workspace\src\disk.rs:189:36
          |
      189 |                     root_id: (held.dev(), held.ino()),
          |                                    ^^^ method not found in `std::fs::Metadata`
      
      error[E0599]: no method named `ino` found for struct `std::fs::Metadata` in the current scope
         --> crates\kiln-workspace\src\disk.rs:189:48
          |
      189 |                     root_id: (held.dev(), held.ino()),
          |                                                ^^^
          |
      help: there is a method `into` with a similar name
          |
      189 |                     root_id: (held.dev(), held.into()),
          |                                                  +
      
      error[E0599]: no method named `dev` found for struct `std::fs::Metadata` in the current scope
         --> crates\kiln-workspace\src\disk.rs:213:35
          |
      213 |         if meta.is_dir() && (meta.dev(), meta.ino()) == self.root_id {
```
Failure details per step as printed: clippy: `error[E0599]` (no method dev/ino/mode on std::fs types) in crates\kiln-workspace\src\disk.rs; schema: "kiln-cli commands --audit failed:" then the same kiln-workspace compile errors ("could not compile `kiln-workspace` (lib) due to 6 previous errors"); scorecard: "error: building test executables failed ():" with `Permissions::from_mode` / `is_fifo` not found, "could not compile `kiln-workspace` (lib test) due to 34 previous errors", then "error: failed to remove file `...\target\debug\xtask.exe` Caused by: Access is denied. (os error 5)"; test: "0 passed, 0 failed, 0 ignored in 0 suites", errors in xtask\src\hygiene.rs:541 (std::os::unix) and xtask\src\publish.rs:1341 (from_mode), "could not compile `xtask` (bin \"xtask\" test) due to 4 previous errors".

Compile errors per file, counted only from the error lines present in the printed step details (details are excerpts; "previous errors" totals in the log are 6 (lib), 34 (lib test), 4 (xtask test)):
| step | file | `error[` with ` -->` seen |
|---|---|---|
| clippy | crates\kiln-workspace\src\disk.rs | 5 (lines 189, 189, 213, 213, 361; the step excerpt omits an E0433 error; its "Some errors have detailed explanations: E0433, E0599" line names that code) |
| schema | crates\kiln-workspace\src\disk.rs | 2 (213, 361 visible) |
| scorecard | crates\kiln-workspace\src\disk.rs | 3 visible (1248 from_mode, 1283 is_fifo, 1353 is_fifo) of 34 reported |
| test | xtask\src\hygiene.rs | 1 (541) |
| test | xtask\src\publish.rs | 1 (1341) |
In window-probe's Build kiln (full output, not an excerpt): disk.rs 6 errors.

Core crates:
```
==== cargo test --locked -p kiln-text
==== kiln-text: exit status 0
==== cargo test --locked -p kiln-syntax
==== kiln-syntax: exit status 0
==== cargo test --locked -p kiln-search
==== kiln-search: exit status 0
==== cargo test --locked -p xtask
==== xtask: exit status 101
CORE CRATES: exit status per package: kiln-text=0 kiln-syntax=0 kiln-search=0 xtask=101 (reported, not required)
```
xtask: only "==== xtask: exit status 101"; the filtered log shows no `test result:` line or error line for xtask (build of xtask test failed, see above; the log does not print it). kiln-text/kiln-syntax/kiln-search: all test result ok (31,21,6,3,17,0 / 22,9,6,5,11,0 / 17,19,3,5,11,0 passed, 0 failed).
```
DISK: df -h of the workspace's volume
Filesystem      Size  Used Avail Use% Mounted on
D:              150G  5.5G  145G   4% /d
DISK: du -sh target
2.5G	target
```
Toolchain: lines above (cargo 1.97.0 / clippy 0.1.97 / rustfmt 1.9.0-stable). "Full test output" printed: `TEST OUTPUT: exit status 101` and no test result lines (nothing compiled).

## gate-macos (Gate failed: steps scorecard and test)
```
cargo 1.97.0 (c980f4866 2026-06-30)
clippy 0.1.97 (2d8144b788 2026-07-07)
rustfmt 1.9.0-stable (2d8144b788 2026-07-07)
gate: 11 steps: fmt, clippy, layers, ui-commands, hygiene, schema, scorecard, test, no-skip, fuzz, budgets
PASS  fmt             0.7s  formatted
PASS  clippy         34.6s  no warnings (all targets, -D warnings)
PASS  layers          1.7s  11 crates, 14 internal edges, layer table holds
PASS  ui-commands     0.0s  self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s)
PASS  hygiene         0.4s  reviews: 42 of 54 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 592 tracked file(s) clean (0 not UTF-8, skipped)
PASS  schema         21.6s  30 commands; audit ok; docs/commands.json current
FAIL  scorecard     292.1s  7 scorecard problems:
        - control.errors: test `kiln_cli::guard_t038_cli::disk_refusals_through_control` fails
        - ui.window: test `kiln::acceptance_t065::ac1_no_display_exits_2_with_one_line` fails
        - ui.window: test `kiln::acceptance_t065::ac1_no_display_every_window_form` fails
        - ui.window: test `kiln::acceptance_t065::ac1_wayland_only_or_unreachable_display_exits_2` fails
        - ws.open-project: test `kiln_cli::acceptance_t019_cli::tree_order_and_counts` fails
        - ws.open-project: test `kiln_cli::acceptance_t019_cli::deep_trees_stop_at_the_depth_bound` fails
        - docs/scorecard.md is stale; run `cargo xtask scorecard` and commit it
FAIL  test            2.4s  25 passed, 1 failed, 0 ignored in 2 suites
      test ac8_stdout_write_failure_exits_1 ... FAILED
      ---- ac8_stdout_write_failure_exits_1 stdout ----
         Compiling kiln-testkit v0.0.0 (/Users/runner/work/photocraft-study-review/photocraft-study-review/crates/kiln-testkit)
          Finished `test` profile [unoptimized + debuginfo] target(s) in 0.55s
           Running unittests src/main.rs (target/debug/deps/kiln-c7027f4c8a754026)
           Running tests/acceptance_t055.rs (target/debug/deps/acceptance_t055-8741fed8ef727973)
      error: test failed, to rerun pass `-p kiln --test acceptance_t055`
PASS  no-skip         1.5s  self-test ok; 185 files scanned, 0 allowlisted skip(s), today = 2026-11-15
EMPTY fuzz            0.0s  no targets registered yet (xtask/fuzz.toml)
EMPTY budgets         0.0s  no targets registered yet (xtask/budgets.toml)
GATE FAILED in 355.1s: scorecard, test
```
Report JSON: "failed_step": "scorecard". Clippy, schema, layers, hygiene etc. PASS on macOS.
Gate "test" step detail: first failing test aborted the run (stops at first failing suite): `ac8_stdout_write_failure_exits_1` with panic
```
thread 'ac8_stdout_write_failure_exits_1' panicked at apps/kiln/tests/acceptance_t055.rs:329:14:
open /dev/full (precondition of this test): Os { code: 2, kind: NotFound, message: "No such file or directory" }
```
Core crates:
```
==== cargo test --locked -p kiln-text
==== kiln-text: exit status 0
==== cargo test --locked -p kiln-syntax
==== kiln-syntax: exit status 0
==== cargo test --locked -p kiln-search
==== kiln-search: exit status 0
==== cargo test --locked -p xtask
test result: FAILED. 116 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.40s
==== xtask: exit status 101
CORE CRATES: exit status per package: kiln-text=0 kiln-syntax=0 kiln-search=0 xtask=101 (reported, not required)
```
xtask: `test result: FAILED. 116 passed; 3 failed` (names of the 3 are not in the log: the step prints only `test result` lines). kiln-text, kiln-syntax, kiln-search all ok.

### Full workspace test output (step "Full test output (gate failed)")
The step prints `grep -E -A1 ... | head -200`. The printed block is exactly 200 lines (cap reached), so the list below is a PREFIX of the run: 46 test binaries printed, then cut. The complete list is in artefact gate-macos-43752760283 (test-output.txt), not read.
Printed binaries: 366 passed, 11 failed (sum of the `test result:` lines in the printed block).

Failing tests seen (binary: test), grouped by first panic line:

Group A, `open /dev/full` / `/dev/full` missing (macOS has no /dev/full): 3 tests
- kiln acceptance_t055: ac8_stdout_write_failure_exits_1
- kiln-cli acceptance_t004_r2: run_and_control_exit_1_when_stdout_cannot_be_written
- kiln-cli run: run_exits_1_when_stdout_cannot_be_written
Panic messages:
```
thread 'ac8_stdout_write_failure_exits_1' (97225) panicked at apps/kiln/tests/acceptance_t055.rs:329:14:
open /dev/full (precondition of this test): Os { code: 2, kind: NotFound, message: "No such file or directory" }
thread 'run_and_control_exit_1_when_stdout_cannot_be_written' (114168) panicked at apps/kiln-cli/tests/acceptance_t004_r2.rs:124:5:
this test needs /dev/full
thread 'run_exits_1_when_stdout_cannot_be_written' (126105) panicked at apps/kiln-cli/tests/run.rs:396:62:
called `Result::unwrap()` on an `Err` value: Os { code: 2, kind: NotFound, message: "No such file or directory" }
```

Group B, `kiln` with no display did not finish (window opened or hang), timeout: 5 tests
- kiln acceptance_t065: ac1_no_display_exits_2_with_one_line; ac1_no_display_every_window_form; ac1_wayland_only_or_unreachable_display_exits_2 (binary 121.35s, 4 passed 3 failed)
- kiln guard_t055: kiln_without_subcommand_and_display_exits_nonzero_with_a_message (60.10s)
- kiln window_t065: without_a_display_the_window_exits_2_with_one_line (60.23s)
```
thread 'ac1_no_display_exits_2_with_one_line' (98060) panicked at apps/kiln/tests/acceptance_t065.rs:65:13:
kiln --root d src/a.rs did not finish within 120s (a window opened, or a hang)
thread 'ac1_no_display_every_window_form' panicked at apps/kiln/tests/acceptance_t065.rs:65:13:
kiln  did not finish within 120s (a window opened, or a hang)
thread 'ac1_wayland_only_or_unreachable_display_exits_2' panicked at apps/kiln/tests/acceptance_t065.rs:65:13:
kiln --root d src/a.rs did not finish within 120s (a window opened, or a hang)
thread 'kiln_without_subcommand_and_display_exits_nonzero_with_a_message' (106548) panicked at apps/kiln/tests/guard_t055.rs:55:13:
kiln did not finish within 60s
thread 'without_a_display_the_window_exits_2_with_one_line' (110325) panicked at apps/kiln/tests/window_t065.rs:68:13:
kiln ["--root", "d", "src/a.rs"] did not finish within 60s
```
No test binary was reported killed; these are test-side timeouts.

Group C, symlink loop precondition ("Too many levels of symbolic links (os error 62)"): 1 test (3 failed cases inside)
- kiln-cli guard_t038_cli: disk_refusals_through_control (failed cases: open-link-loop, save-link-loop, tree-link-loop)
```
thread 'disk_refusals_through_control' (125916) panicked at apps/kiln-cli/tests/guard_t038_cli.rs:86:9:
assertion `left == right` failed: precondition: root/l1 is a link loop: Too many levels of symbolic links (os error 62)
(also root/p/y.txt, root/l1) ... guard_t038_cli.rs:262:5:
failed cases (each one's message is above): ["open-link-loop", "save-link-loop", "tree-link-loop"]
```

Group D, kiln-cli acceptance_t019_cli (binary: 5 passed, 2 failed, 11.53s): 2 tests
- deep_trees_stop_at_the_depth_bound: `acceptance_t019_cli.rs:1573:5: mkdir -p of the 2,100-deep chain`
- tree_order_and_counts: `acceptance_t019_cli.rs:1018:5: assertion `left == right` failed: tree dot-git: entries: {".git": Dir, ".git/HEAD": File([114, 101, 102, 10]), "s": Dir, "s/.git": File([103, 105, 116, 100, ...` (printed line cut at that point by the log's -A1 filter; the rest not in the log)

Total failing tests visible in the printed prefix: 11 (3 + 5 + 1 + 2). Failing in the xtask core-crates run: 3 (names unprinted). Possible overlap/other failures beyond the 200-line cap: unknown.
Scorecard step problem list (gate step, 7): the five scorecard items map to Groups B, C, D; plus `docs/scorecard.md is stale; run `cargo xtask scorecard` and commit it`:
```
        - control.errors: test `kiln_cli::guard_t038_cli::disk_refusals_through_control` fails
        - ui.window: test `kiln::acceptance_t065::ac1_no_display_exits_2_with_one_line` fails
        - ui.window: test `kiln::acceptance_t065::ac1_no_display_every_window_form` fails
        - ui.window: test `kiln::acceptance_t065::ac1_wayland_only_or_unreachable_display_exits_2` fails
        - ws.open-project: test `kiln_cli::acceptance_t019_cli::tree_order_and_counts` fails
        - ws.open-project: test `kiln_cli::acceptance_t019_cli::deep_trees_stop_at_the_depth_bound` fails
        - docs/scorecard.md is stale; run `cargo xtask scorecard` and commit it
```
Disk and toolchain:
```
DISK: df -h of the workspace's volume
Filesystem      Size    Used   Avail Capacity iused ifree %iused  Mounted on
/dev/disk2s5   320Gi   262Gi    40Gi    87%    2.3M  419M    1%   /System/Volumes/Data
DISK: du -sh target
4.4G	target
```
