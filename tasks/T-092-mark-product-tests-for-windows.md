---
id: T-092
title: "Make every product test target compile and run on Windows: mark what tests a facility Windows lacks, fix what only assumes Unix"
tier: high              # provisional: xtask/platform-tests.toml and xtask/tests/ are gate paths
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-073]     # until kiln-workspace compiles for Windows no product test target is checked there
touches:                # provisional, fixed at specification
  - xtask/platform-tests.toml
budget_tokens: 1100000  # high (v1.5)
iteration: 2
found_by: planner       # spec check R-321 of T-079 (D-233, ruling Q-4 B)
origin_task: T-079
---

# T-092 The product tests on Windows

This is a draft split from T-079 by ruling Q-4 B (D-233). It is specified in full after T-073 merges.

## Goal
T-079 as first written held two halves; the macOS half stays there. This is the Windows half: every product test
target compiles for Windows; a test of a facility Windows lacks is registered with its counterpart or reason; a
test that only assumes Unix gets a portable fix; no Linux assertion changes.

## Evidence in the record
- Spec check R-321 in `tasks/T-079-mark-product-tests-by-platform.md`, probe 2 (with a stand-in for the 6 product
  lines of `disk.rs`, never committed): 225 errors in 27 test files and modules for
  `x86_64-pc-windows-msvc` — `kiln-cli` tests 11 files and 93 errors, `kiln` tests 4 and 14, `kiln-commands` 2 and 7,
  `kiln-workspace` 10 and 111; `xtask` and the test files of T-057 to T-059: 0.
- Probe 1 (the branch as it is): the check stops at `kiln-workspace` (lib 6 errors, lib test 34), so nothing of
  this task can be shown before T-073.

## Note from T-079 (director, D-242, ruling x5)
The four non-UTF-8 entries are `linux` with a reason that claims macOS only. On Windows a file name can hold an
unpaired surrogate (`reviews/T-079-r1.md`, finding 1): this task decides whether those tests get a Windows
counterpart with such a name.

## Note from T-073 (director, D-249, ruling z3)
T-073 registers 15 tests of `crates/kiln-workspace/src/disk.rs`'s own test module as `unix` with
`counterpart = "pending"`, `task = "T-092"`, expiring 2026-12-31 (modes, inodes, uid, Unix symlinks, FIFOs). This
task writes their Windows counterparts or changes an entry to `none` with the reason; those about the walk and
a replaced root may move to T-074 by a ruling after T-073's review. `xtask/platform-tests.toml` is in `touches`.
Also: `xtask/tests/acceptance_t079.rs` checks T-079's 20 entries one by one; a task that changes one of those 20
lists that file for its test author.

## Note from T-073's CI round 2 (director, 2026-11-18)
Since `kiln-workspace` compiles for Windows, `acceptance_t002::disk_save_replaces_via_rename_without_leftovers`
runs there and fails (`tasks/ci-runs/T-073-round-2.md`, the scorecard's list): T-071 made it fail on Windows by
design, naming why, in place of two twins that did nothing. This task gives it its Windows form: the checks every
platform has stay unconditional, the mode and inode checks become a registered Unix part with a counterpart or a
reason. The other targets the Windows gate still names: `kiln-cli` `acceptance_t008_cli` and `acceptance_t019_cli`,
`kiln` `acceptance_t055`, `script_t055`, `acceptance_t065`, `window_t065`.

## Log
### director
- 2026-11-18 filed as a draft (D-233).
