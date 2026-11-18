---
id: T-075
title: "Make the existing kiln-workspace and kiln-commands tests compile and run on Windows under the platform-test rule"
tier: low               # ADR-0002: tests only; behaviour is specified by T-073 and T-074
state: draft
owner_role: planner
scorecard_items: []     # none claimed: the items exist and are verified on Linux; per-platform confirmation is the evaluator's (ADR-0009 Decision 3)
depends_on: [T-071, T-074]
touches:                # provisional, fixed at specification; each path one role (D-153)
  - crates/kiln-workspace/tests/            # implementer: the 9 files that fail on Windows (probe a4), one role
  - crates/kiln-workspace/src/disk.rs       # implementer: its #[cfg(test)] module only (from line 535)
  - crates/kiln-commands/src/doc.rs         # implementer: its #[cfg(test)] module only (from line 1312)
  - crates/kiln-commands/tests/acceptance_t037_commands.rs
  - xtask/platform-tests.toml               # GATE PATH: entries for tests of facilities Windows lacks (FIFOs, modes) -> makes this high unless T-071 lists them; decided at specification
budget_tokens: 150000
iteration: 2
found_by: planner       # ADR-0009 (R-255)
origin_task: null
---

# T-075 Make the existing kiln-workspace and kiln-commands tests compile and run on Windows under the platform-test rule

Draft by the planner of ADR-0009 (run R-255); a stub in the style of T-066 and T-068. Interface, Pinned
and acceptance criteria are written at specification, after T-070's first runs have replaced the
estimates of ADR-0009 with measurements.

> **Absorbed (director, D-269, 2026-11-18):** the work of this draft is done by T-092 (the `kiln-workspace` test
> targets on Windows) and T-098 (the other crates' targets), as the planner of T-092 found from the inventory. The
> draft is kept as a record and is not specified.

## Goal
On Windows 12 test files or test modules of these crates do not compile (9 integration-test files of `kiln-workspace`, `acceptance_t037_commands.rs`, and the test modules of `disk.rs` and `doc.rs`) (probe a4: FIFOs, modes, `uid`/`gid`,
`dev`/`ino`, non-UTF-8 names). After this task each compiles everywhere; each test asserts every
platform's specified behaviour or is a listed platform test with its counterpart (ADR-0009 Decision 4).
No assertion is weakened on Linux or macOS (the reviewer of Q-7's kind compares the Linux test list and
outcomes before and after).

## Estimate
Low by rule but large: **about 350,000**, over the low ceiling; the director either sets an Extend row at
specification or splits it by file group. If it must edit `xtask/platform-tests.toml` it is high.

## Log
### director
<!-- end director log -->
### planner
- 2026-11-15 · R-255 · draft · did: filed from ADR-0009 · next: director
<!-- end planner log -->
