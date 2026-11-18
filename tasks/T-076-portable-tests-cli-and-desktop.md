---
id: T-076
title: "Make the kiln-cli and kiln tests compile and run on Windows under the platform-test rule"
tier: low               # ADR-0002: tests only
state: draft
owner_role: planner
scorecard_items: []     # none claimed: the items exist and are verified on Linux; per-platform confirmation is the evaluator's (ADR-0009 Decision 3)
depends_on: [T-071, T-074]
touches:                # provisional, fixed at specification; each path one role (D-153)
  - apps/kiln-cli/tests/    # implementer: the 11 files that fail on Windows (probe a4)
  - apps/kiln/tests/acceptance_t055.rs
  - apps/kiln/tests/acceptance_t065.rs   # non-UTF-8 arguments: on Windows arguments are UTF-16 (a different invalid form)
  - apps/kiln/tests/script_t055.rs
  - apps/kiln/tests/window_t065.rs
budget_tokens: 150000
iteration: 2
found_by: planner       # ADR-0009 (R-255)
origin_task: null
---

# T-076 Make the kiln-cli and kiln tests compile and run on Windows under the platform-test rule

Draft by the planner of ADR-0009 (run R-255); a stub in the style of T-066 and T-068. Interface, Pinned
and acceptance criteria are written at specification, after T-070's first runs have replaced the
estimates of ADR-0009 with measurements.

## Goal
15 test files of the two apps do not compile on Windows (probe a4). After this task they compile and run
on all three under the rule of ADR-0009 Decision 4, and the items whose tests live there (b2:
`ws.open-project`, `nav.*`, `syntax.outline`, `control.errors`, `ui.window`, part of `ws.path-confinement`)
have their tests run on Windows.

Tests of facilities Windows lacks (FIFOs in `acceptance_t008_cli.rs`, `guard_t019_cli.rs`, …) need entries in
`xtask/platform-tests.toml`, a gate path: either T-071 adds them, or this task is high (ADR-0009 Q-8).

## Estimate
**About 300,000**, over the low ceiling; Extend row or split at specification.

## Note from T-079 (director, D-242, ruling x6)
T-079 registers three tests as Linux-only with `counterpart = "pending"`, `task = "T-076"`, expiring 2026-12-31:
the `/dev/full` tests (`apps/kiln/tests/acceptance_t055.rs` `ac8_stdout_write_failure_exits_1`,
`apps/kiln-cli/tests/acceptance_t004_r2.rs` `run_and_control_exit_1_when_stdout_cannot_be_written`,
`apps/kiln-cli/tests/run.rs` `run_exits_1_when_stdout_cannot_be_written`). **This task writes their counterparts**
(a stdout that cannot be written, made in a way every platform has, for example a closed pipe) and edits the
three registry entries to name them: `xtask/platform-tests.toml` joins `touches`, and the tier is **high**, not
low, because the registry is a gate path. The specification writes this in.

## Log
### director
<!-- end director log -->
### planner
- 2026-11-15 · R-255 · draft · did: filed from ADR-0009 · next: director
<!-- end planner log -->
