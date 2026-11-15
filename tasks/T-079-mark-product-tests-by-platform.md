---
id: T-079
title: "Make every product test target compile on Windows and pass on macOS: register the tests of facilities a platform lacks, each with its sentence and counterpart, and fix the tests that assume Linux where the platform has the facility"
tier: high              # METHODOLOGY v1.4 §4: gate paths (xtask/platform-tests.toml, xtask/tests/); the approval row must contain `gate paths`. No dependency change, no product code.
state: specified
owner_role: planner
scorecard_items: []     # none claimed; the tests stay tied to their items
depends_on: [T-071]     # the registry's format and the no-skip rule it must satisfy
touches:                # one role per path (D-153); all implementer except the last two
  - xtask/platform-tests.toml                              # GATE PATH: entries for the product tests (the file exists after T-071)
  - crates/kiln-workspace/src/disk.rs                      # its #[cfg(test)] module only (from line 535); product code unchanged (T-073)
  - crates/kiln-commands/src/doc.rs                        # its #[cfg(test)] module only (from line 1311)
  - crates/kiln-workspace/tests/acceptance_t002_r2.rs
  - crates/kiln-workspace/tests/acceptance_t008.rs
  - crates/kiln-workspace/tests/acceptance_t019_workspace.rs
  - crates/kiln-workspace/tests/acceptance_t036_workspace.rs
  - crates/kiln-workspace/tests/confinement.rs
  - crates/kiln-workspace/tests/guard_t008.rs
  - crates/kiln-workspace/tests/guard_t036_removal_workspace.rs
  - crates/kiln-workspace/tests/guard_t036_workspace.rs
  - crates/kiln-workspace/tests/project.rs
  - crates/kiln-commands/tests/acceptance_t037_commands.rs
  - apps/kiln-cli/tests/acceptance_t004_r2.rs              # /dev/full (macOS failure, R-285)
  - apps/kiln-cli/tests/run.rs                             # /dev/full (macOS failure, R-285)
  - apps/kiln-cli/tests/acceptance_t008_cli.rs
  - apps/kiln-cli/tests/acceptance_t019_cli.rs
  - apps/kiln-cli/tests/acceptance_t023_cli.rs
  - apps/kiln-cli/tests/acceptance_t036_cli.rs
  - apps/kiln-cli/tests/acceptance_t037_cli.rs
  - apps/kiln-cli/tests/acceptance_t037_removal_cli.rs
  - apps/kiln-cli/tests/guard_t019_cli.rs
  - apps/kiln-cli/tests/guard_t036_cli.rs
  - apps/kiln-cli/tests/guard_t036_removal_cli.rs
  - apps/kiln-cli/tests/guard_t037_cli.rs
  - apps/kiln-cli/tests/guard_t038_cli.rs
  - apps/kiln/tests/acceptance_t055.rs
  - apps/kiln/tests/guard_t055.rs                          # no-display case (macOS failure, R-285)
  - apps/kiln/tests/acceptance_t065.rs
  - apps/kiln/tests/script_t055.rs
  - apps/kiln/tests/window_t065.rs
  - xtask/tests/acceptance_t079.rs                         # GATE PATH: test author
  - xtask/tests/guard_t079.rs                              # GATE PATH: test author (every Linux test name kept)
budget_tokens: 1100000  # high (v1.4); estimate about 850,000
iteration: 2
found_by: planner       # ADR-0009 (R-255); split off T-071 by D-180 Q-1 B
origin_task: null
---

# T-079 Make every product test target compile on Windows and pass on macOS

## Goal
The rule of METHODOLOGY v1.4 §8.1 "Tests" (tool: T-071) is applied to the product's test files. Today:
- **Windows:** 25 product test files and two test modules (`disk.rs`, `doc.rs`) do not compile for
  `x86_64-pc-windows-msvc` (R-261 re-check on 15555cc: 238 errors in 33 files with the C code stubbed and
  `disk.rs`'s 6 product lines stood in; 6 of the 33 are `xtask`'s, T-071's). CI cannot show this yet: in run
  43765532298 `gate-windows` stops at `kiln-workspace` (lib), whose 6 product errors are T-073's.
- **macOS:** of 22 failing tests in the full test output of `gate-macos` (run 43765532298, artefact
  `test-output.txt`, read in R-285: 977 passed, 22 failed), 16 are product tests, and **none is a
  product fault**: each assumes Linux (table in Interface (1)). 6 are `xtask`'s (T-071).
After this task, with T-071 merged: every product test target compiles for Windows once the 6 product
lines of `disk.rs` are set aside (measured locally, AC-5); on macOS the 16 tests either pass or are
registered platform tests whose counterpart is named; no assertion changes on Linux, where every test
name still exists and passes. Gate paths: `xtask/platform-tests.toml`, `xtask/tests/acceptance_t079.rs`,
`xtask/tests/guard_t079.rs`.

Reading list: METHODOLOGY v1.4 §8.1, §6; `tasks/T-071-platform-tests-rule.md` Interface (1)–(3) (the
registry and the findings); `tasks/ci-runs/T-070-round-2.md`; ADR-0009 (b) and probe a4.

## Interface

### (1) The 16 macOS failures and what becomes of each
| Test (file:line of the panic) | Printed cause | Kind | Treatment |
|---|---|---|---|
| `ac8_stdout_write_failure_exits_1` (`apps/kiln/tests/acceptance_t055.rs:329`) | `open /dev/full … NotFound` | facility absent (`/dev/full` is Linux's) | `#[cfg(target_os = "linux")]`, entry `linux`, counterpart pending T-076: the same sentence through a stdout whose reader has closed (a portable way to make a write fail) |
| `run_and_control_exit_1_when_stdout_cannot_be_written` (`apps/kiln-cli/tests/acceptance_t004_r2.rs:124`) | `this test needs /dev/full` | as above | as above |
| `run_exits_1_when_stdout_cannot_be_written` (`apps/kiln-cli/tests/run.rs:396`) | `NotFound` | as above | as above |
| `tree_counts_skipped_names_on_disk` (`crates/kiln-workspace/tests/acceptance_t019_workspace.rs:412`) | `Os { code: 92 … "Illegal byte sequence" }` | facility absent: the macOS file system refuses names that are not UTF-8 (and Windows names are UTF-16) | the non-UTF-8 case only becomes a `linux` test (split out if the test checks other things); entry `linux`, counterpart `none`, reason "no file name on macOS or Windows can be invalid Unicode" |
| `disk_list_dir_reports_entries_by_their_own_kind_and_skips_bad_names` (`acceptance_t019_workspace.rs:219`) | same | same | same |
| `documented_differences_from_git` (`crates/kiln-workspace/tests/project.rs:374`) | same | same | same |
| `disk_list_dir_reports_own_kinds_sorted_and_skips_odd_names` (`project.rs:120`) | same | same | same |
| `ac1_no_display_every_window_form`, `ac1_no_display_exits_2_with_one_line`, `ac1_wayland_only_or_unreachable_display_exits_2` (`apps/kiln/tests/acceptance_t065.rs:65`) | `did not finish within 120s (a window opened, or a hang)` | the specified behaviour is X11's: without `DISPLAY` there is no window on Linux; on macOS a window opens (the `window-probe (macos-15)` job opened one, run 43765532298) | `linux` tests; entries `linux`, counterpart pending T-077 (what "no window" means on macOS and Windows) |
| `without_a_display_the_window_exits_2_with_one_line` (`apps/kiln/tests/window_t065.rs:68`) | `did not finish within 60s` | same | same |
| `kiln_without_subcommand_and_display_exits_nonzero_with_a_message` (`apps/kiln/tests/guard_t055.rs:55`) | `kiln did not finish within 60s` | same | same |
| `disk_refusals_through_control` (`apps/kiln-cli/tests/guard_t038_cli.rs:86`) | `precondition: root/l1 is a link loop: … (os error 62)` against `Some(40)` | test assumes Linux's `ELOOP` number | portable: the expected number is the platform's (chosen in the test by platform; no `cfg` on the test) |
| `deep_trees_stop_at_the_depth_bound` (`apps/kiln-cli/tests/acceptance_t019_cli.rs:1573`) | `mkdir -p of the 2,100-deep chain` | the fixture uses `mkdir -p` beyond the host's path limit (GNU succeeds, macOS's does not) | portable: build the chain one level at a time relative to the last directory; the assertion stays |
| `tree_order_and_counts` (`acceptance_t019_cli.rs:1018`) | the model lists `s/t/z` beside `s/t/Z`; Kiln lists one | the model assumes a case-sensitive file system (macOS's default is not) | portable: the generator does not make names that differ only by case on a file system that folds case (the test detects it), or the model folds them; the assertion against the model stays |
| `disk::tests::failed_rename_removes_own_temp_and_keeps_target` (`crates/kiln-workspace/src/disk.rs:780`) | left `/private/var/…/.kiln-tmp-1`, right `/var/…/.kiln-tmp-1` | the test compares a path below the temp dir, which is a link on macOS | portable: compare canonical paths |
All 16 VERIFIED as printed in the artefact (R-285); the "Kind" column is INFERRED from the message and
the code at the line.

### (2) Windows
In the 27 files and modules of `touches` that do not compile on Windows (all but `acceptance_t004_r2.rs`,
`run.rs`, `guard_t055.rs`), every test that cannot compile or mean anything on Windows (FIFOs, mode bits,
uid/gid, inodes, `mkfifo`, non-UTF-8 bytes in names or arguments, shell-script fakes) gets `#[cfg(unix)]`,
its Unix-only helpers get `#[cfg(unix)]` with no Windows twin (T-071's `inert-helper` rule), and an entry
with `doc`/`quote` of the sentence it tests, `counterpart = "pending"`, the counterpart's future
`file::item` (ruling Q-3 A, D-180), `task` T-075 (`kiln-workspace`, `kiln-commands`) or T-076 (`kiln-cli`,
`kiln`) — or T-073/T-074 where the sentence is about the root or confinement on Windows — and `expires =
"2026-12-31"` (D-180 Q-2 A). A test that compiles on Windows stays unmarked. How many tests are marked is
**to be measured by the implementer** (R-261 found 31 test fns and 116 helpers named directly by errors;
tests reaching those helpers are more).

### (3) Unchanged
No assertion is changed, removed or loosened on Linux; every Linux test name of the base still exists
(a split keeps the original name for the part that stays). No product code; no `docs/` sentence (the
macOS and Windows sentences these tests imply go to T-072, T-074, T-077).

## Pinned
- (a)–(c): n/a (tests and a registry only); the findings are T-071's.
- (d) Platforms: Linux x86_64, macOS arm64, Windows x86_64 (§8.1).
- (e) Threat model: an entry must not let a weakened test pass: the reviewer reads every entry against
  its test, and the guard keeps every Linux test name.
- (f) Tests pinning what this changes: the scorecard items list test names (`scorecard/items/*.toml`);
  a split must keep the listed name on Linux (guard).
- (g) Worked examples: the table of Interface (1), from the artefact.
- (h) Evaluator: none (no item).
- (i) None.

### What the director must push and read
After the join: push the joined head to `ci/T-079` (D-185; per-task branch reused if it exists, D-198),
read run, record per job (§8.1). **Expected 2 rounds.** Expected result: `gate-linux` green; `gate-macos`
**green** if nothing beyond the 22 failures exists (T-071 + T-079 cover all 22; the artefact is the whole
output, so no unseen failure is expected); `gate-windows` red at `kiln-workspace` (lib) until T-073.

## Acceptance criteria
Gate tests by the independent test author (`xtask/tests/acceptance_t079.rs`), reading the repository's
own files with `syn` or as text — each fails on base (main with T-071):
- AC-1: no product test file or test module names `std::os::unix` (or a `std::os::unix` method such as
  `from_mode`, `ino`, `uid`, `is_fifo`) outside an item behind a `unix` or `linux` cfg. Oracle: a scan;
  on base it finds the 27 files and modules of Interface (2).
- AC-2: no test outside an item behind `cfg(target_os = "linux")` names `/dev/full`. On base: 3 files.
- AC-3: no test compares `raw_os_error()` with an integer literal (a platform-chosen value is allowed).
  On base: `guard_t038_cli.rs:86`.
- AC-4: the registry has an entry with `platforms = "linux"` for each of the 12 registered tests of
  Interface (1), and every entry's counterpart names a test or carries a reason. On base: none.
- Guard (`guard_t079.rs`): the Linux test list (`cargo test --workspace -- --list` names) contains every
  name of the base's list (passes on base and after).
Local, by the implementer, re-run by the reviewer (R-255 probe method; scratch, nothing installed except
`rustup target add`):
- AC-5: `cargo check --workspace --all-targets --locked --target x86_64-pc-windows-msvc`, C compiler stubbed,
  `disk.rs`'s 6 product lines stood in (never committed): **0 errors in test code** (238 on 15555cc).
Through CI (the director's record; the reviewer claims only what it shows, §8.1):
- AC-6: on `ci/T-079`, the full test output of `gate-macos` has none of the 16 tests failing, and the 12
  registered ones absent; the scorecard step's platform line reports them as registered absent.

## Out of scope and implementer notes
- Windows counterparts (T-073 to T-076); the window on macOS/Windows (T-077); documents (T-072, T-074).
- `xtask`'s tests and the tool (T-071). `acceptance_t002.rs` and `acceptance_t015_tools.rs` (T-071 handles the
  first; the second already fails off Unix, naming why).
- Never mark a test that compiles and means the same on Windows; never change an expected value on Linux.

## Collisions and order
- After T-071 (depends_on). T-043 does not share these paths.
- **T-072** (macOS): its residual scope after this task is the macOS document sentences and anything
  `ci/T-079` still shows; it shares no path if it touches only `docs/` — order T-079 then T-072 (Q-1).
- **T-073** edits `disk.rs` (product code): same file as this task's test module → not in flight together;
  T-079 first (T-073's tests then compile).
- **T-074, T-075, T-076** edit these test files' Windows cases → after this task.
- **T-058/T-059** (cut 2): their test files (`acceptance_t058.rs`, `tabs_t058.rs` on `task/T-058`) use no
  Unix-only API (R-285 grep); if one fails on macOS in `ci/T-079`'s run, it is added here by a ruling, or
  fixed in its own task if still in flight.
- T-054: no shared path (this task does not touch `docs/development.md`) — may be in flight together.

## Estimate
30 paths of mostly mechanical marking with per-test judgement, 4 portable fixes, entries, a test author
with four scan tests, two CI rounds: **about 850,000** of the high ceiling 1,100,000 (T-070: 809,362, D-197;
stage-1 high p75 866,379). Arm: high → independent test author.

## Questions and rulings
- **Q-1 (planner, R-285). T-072 after this task.** A: T-072 shrinks to the macOS document sentences (non-UTF-8
  names impossible, a window opens without `DISPLAY`, case folding) and what `ci/T-079` still shows. B: T-072
  takes the four portable fixes of Interface (1) instead. **Recommendation: A** — the fixes are in files this
  task already holds; B puts the same files in two tasks.
- **Q-2 (planner, R-285). The no-display tests' counterpart.** A: pending T-077. B: `none` (macOS always has a
  window system). **Recommendation: A** — "without a window system, exit 2" still has a macOS and a Windows
  meaning (no session, a headless runner) for T-077 to state.
- **Q-3 (planner, R-285). AC-1 to AC-3 as scans.** They check the repository, not Kiln's behaviour. A: keep
  them (they fail on base and are mechanical). B: rely on AC-5 and AC-6 only. **Recommendation: A** — AC-5 and
  AC-6 are not gate tests; without the scans a later Unix-only test passes the local gate.

### Spec check R-285
Written against main `cbb54a2628afbd368087e82c5d7925eaa5b08625`. Evidence: the `gate-macos` artefact of run
43765532298 (whole test output, downloaded with the Actions tools, unpacked under `/home/user/kiln-plan-T-071/`,
deleted); `tasks/ci-runs/T-070-round-2.md` for Windows; R-261's Windows check for the 33 files. Files added
beyond R-261's list: `acceptance_t004_r2.rs`, `run.rs`, `guard_t055.rs` (macOS only). Not run: no build.

### Rulings on spec R-285 (director, D-199)
- **Q-1: A.** T-072 shrinks to the macOS document sentences, the report-step limits noted in its file, and whatever
  `ci/T-079` still shows red on macOS; the portable fixes stay here.
- **Q-2: A.** The no-display tests' counterpart is pending T-077.
- **Q-3: A.** The repository scans stay as acceptance tests.
- Order: T-054, T-071, then this task (after T-071's registry exists), then T-072, T-073.

## Log
### director
<!-- end director log -->
### planner
- 2026-11-15 · R-285 · draft · did: full spec from the split of T-071 (D-180 Q-1 B) · not run: builds · next: director
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
