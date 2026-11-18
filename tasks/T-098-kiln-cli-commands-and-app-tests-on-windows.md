---
id: T-098
title: "Make every remaining test target of kiln-commands, kiln-cli and kiln compile and lint clean on Windows under T-092's sorting rule, so that the Windows gate builds and runs every test"
tier: high              # provisional: gate paths (xtask/platform-tests.toml, xtask/tests/); the confinement tests through the CLI and control protocol change what they assert on Windows
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-092]     # T-092's rule and registry; one registry at a time (§5.2). T-076 and T-077 follow this task (D-269 ruling 1)
touches:                # provisional, fixed at specification; one role per path (D-153)
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
  - apps/kiln/tests/acceptance_t065.rs
  - apps/kiln/tests/script_t055.rs
  - apps/kiln/tests/window_t065.rs
  - crates/kiln-commands/src/doc.rs                 # its #[cfg(test)] module only
  - crates/kiln-commands/tests/acceptance_t037_commands.rs
  - xtask/platform-tests.toml                       # GATE PATH
  - xtask/tests/acceptance_t098.rs                  # GATE PATH: test author (registry equality)
  - xtask/tests/guard_t098.rs                       # GATE PATH: test author (the portable rows carry no platform cfg)
budget_tokens: 1100000  # high (v1.5); estimate about 1,000,000 (near the ceiling: see T-092 Q-6)
iteration: 2
found_by: planner       # cut from T-092 at its specification (2026-11-18)
origin_task: T-092
---

# T-098 The kiln-commands, kiln-cli and kiln tests compile on Windows

Draft cut from T-092 at its specification (`tasks/T-092-mark-product-tests-for-windows.md`, "Cut"). Stays whole (ruling 6,
D-269): no widening of T-072's general step. Specified in full after T-092 merges. T-076 (narrowed to its three `/dev/full`
counterparts) and T-077 run after this task (ruling 1, D-269).

## Order note (D-273)
T-092's listing files moved to T-100, which also edits `xtask/platform-tests.toml`: T-098 and T-100 are not in flight
together (§5.2); the director orders them. T-100's paired helper for a name a listing must skip (`back\slash` on Unix,
an unpaired-surrogate name on Windows, ruling Q-A B) is the precedent if any file of this task makes a name holding `\`.

## Facts from T-092's probe (run 44355322518, `tasks/ci-runs/T-092-probe-1.md`)
- Exactly these 17 targets do not compile on Windows (record lines 195-259), with the first errors T-092's inventory lists.
- Every other target of these packages runs and passes on Windows except 4 of `kiln-cli` that are T-099's
  (`acceptance_t004_r2`, `acceptance_t004_r3`, `guard_t015_cli`, `guard_t032_cli`): `kiln-commands` 16 targets, its doc
  tests, `kiln-cli`'s bin unit tests and 19 test targets, `kiln`'s bin and 9 test targets all exit 0.
- `kiln` given an argument with an unpaired surrogate (`OsString::from_wide`), in `--root d <bad>`, `<bad>`, `script s.jsonl
  --root d <bad>`, `script s.jsonl --root <bad>`: `status Some(2), stdout 0 bytes, stderr 24 lines, first "kiln: argument N
  is not valid UTF-8", panicked false` (P-E0..E3). So the two non-UTF-8 argument tests take that Windows form, portable.
- Links can be made on the runner (`symlink_file`, `symlink_dir`, junctions), and every reparse point on the walk answers
  `OutsideRoot` for read, write, `is_file`, `list_dir`, `check_location` and `Document::open` (176 `guard_t074` lines).
- A fake tool must be an executable: with only `cargo.cmd`/`rustfmt.bat` in a child's `PATH`, `Command::new` ran the real
  tools; a `rustc`-built `cargo.exe` ran (P-D1..D4).

## Goal (draft)
After T-092, 17 test targets still do not compile for Windows (VERIFIED by T-092's inventory: Windows clippy with the
stand-in on T-074's and T-072's trees, 115 errors): `kiln-cli` 11 (94 errors), `kiln` 4 (14), `kiln-commands` 2 (7: its lib
test, i.e. the test module of `doc.rs`, and `acceptance_t037_commands`). After this task every one compiles and is
clippy-clean with `-D warnings` for Windows, each test sorted by T-092's rule (portable, or a registered Unix part with its
counterpart or reason), and every test of these files passes on Windows or is registered absent there. Because nothing
then fails to compile, the gate's own `test` step and "Full test output (gate failed)" step run every target on Windows:
this task's CI evidence needs no workflow edit (T-092 Q-6), and its round lists the failures left for T-099.

## Preliminary sorting (from facility tags, `scripts/classify.py` in `/home/user/kiln-plan-T092-tools/`; each test is read at specification)
- Identity and mode tests through the CLI (`uid`/`gid`/`Command::uid`, modes `0311`/`0000`, `lchown`): `acceptance_t036_cli`
  (1), `acceptance_t037_cli` (3), `guard_t036_removal_cli` (3), `guard_t037_cli` (`reopen_in_a_memory_session_is_unchanged`
  mode only; `reopen_does_not_need_the_file` with links and a FIFO): Unix parts, counterparts where the CLI sentence has a
  Windows test (T-073's `guard_t073`, T-074's `disk.rs` `*_on_every_platform`), else `none`.
- FIFO and socket tests: `acceptance_t008_cli` (`fifo_and_socket_do_not_hang_run`, `fifo_does_not_hang_exec`, and the FIFO
  part of the swap tests), `guard_t019_cli` (2), `acceptance_t019_cli` (the FIFO entries of 4 tree tests): Unix parts.
- Links only: `acceptance_t023_cli::workspace_symbols_match_naive_model`, `guard_t036_cli::retargeted_root_link_is_refused_through_control`,
  `acceptance_t019_cli` (`limits_and_refusals`, `tree_under_a_path_applies_prefix_gitignores`), `acceptance_t055` (4 tests
  through `copy_dir` and `ac4_refusals_shown`), `script_t055::refused_opens_are_shown_with_the_kind_and_not_logged`,
  `doc.rs` (`reopen_on_disk_is_refused_when_the_location_is_outside_or_unknown`, `reopen_on_disk_does_not_need_the_file`):
  portable, Windows answer by T-074's documents.
- Arguments that are not UTF-8: `acceptance_t065::ac1_non_utf8_argument_is_a_usage_error`,
  `window_t065::an_argument_that_is_not_utf8_is_a_usage_error`: portable with a Windows form (an unpaired surrogate in
  UTF-16): VERIFIED by P-E (above).
- `guard_t038_cli` (`ELOOP`, a FIFO, link loops): the errno chosen by platform.
- **Constraint (`xtask/tests/guard_t079.rs` AC-2):** `guard_t038_cli.rs::disk_refusals_through_control`,
  `acceptance_t019_cli.rs::deep_trees_stop_at_the_depth_bound` and `::tree_order_and_counts` may carry no platform cfg
  attribute on the fn, in its body, its modules, the file or its declaration: only `cfg!(…)` expressions and twin helpers
  outside the body.

## Overlaps to settle at specification
- T-076 (draft; ruling x6, the `/dev/full` counterparts) edits `apps/kiln/tests/acceptance_t055.rs`; T-077 (draft) writes
  counterparts of no-display tests in `acceptance_t065.rs`, `window_t065.rs`: sequence them after this task (T-092 Q-1).

## Estimate
About 1,000,000 (17 files, about 45 tests to sort, the CLI's confinement tests read against T-073/T-074's documents, one or
two CI rounds).

## Log
### director
<!-- end director log -->
### planner
- 2026-11-18 · planner (T-092's specification run) · draft · did: filed from T-092's cut with the inventory figures · next: director
- 2026-11-18 · planner (T-092's finalization) · draft · did: added the probe's facts and rulings 1 and 6 (D-269) · next: specification after T-092 merges
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
