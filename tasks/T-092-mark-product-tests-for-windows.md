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

## Log
### director
- 2026-11-18 filed as a draft (D-233).
