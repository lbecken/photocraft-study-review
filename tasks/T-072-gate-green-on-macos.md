---
id: T-072
title: "Make the gate pass on macOS, and state in the documents where Kiln behaves differently there"
tier: high              # provisional: likely to touch crates/kiln-workspace/src/disk.rs (confinement) and paths.md; fixed at specification from T-070's first macOS run
state: draft
owner_role: planner
scorecard_items: []     # none claimed: the items exist and are verified on Linux; per-platform confirmation is the evaluator's (ADR-0009 Decision 3)
depends_on: [T-070, T-071]
touches:                # provisional, fixed at specification; each path one role (D-153)
  - crates/kiln-workspace/src/disk.rs   # implementer, only if T-070's run shows a product fault
  - docs/protocol/paths.md              # implementer: one sentence per macOS difference
  - docs/protocol/ui.md                 # implementer: the macOS key sentence (`command` is ⌘)
  - (test files named by T-070's run)   # test author; each path one role
budget_tokens: 1100000
iteration: 2
found_by: planner       # ADR-0009 (R-255)
origin_task: null
---

# T-072 Make the gate pass on macOS, and state in the documents where Kiln behaves differently there

Draft by the planner of ADR-0009 (run R-255); a stub in the style of T-066 and T-068. Interface, Pinned
and acceptance criteria are written at specification, after T-070's first runs have replaced the
estimates of ADR-0009 with measurements.

## Goal
Everything compiles for macOS (ADR-0009 probe a4, a5; VERIFIED with the C code stubbed); whether it passes
is unknown. After this task `gate-macos` is green on main, and each behaviour that differs on macOS is one
sentence with a test that runs there: candidates (INFERRED, ADR-0009 (b)): case-insensitive names (`A.rs`
and `a.rs` are one file and two documents), `PATH_MAX` 1,024, the `nobody` identity, Control and ⌘ in
the editor's keys. The confinement tests already compile on macOS; those that assume uid 0 are T-069's.

## Estimate
Unknown until T-070 runs: **about 600,000** if only tests and sentences change; up to the high ceiling
if `disk.rs` must change. Re-estimate at specification.

## Log
### director
<!-- end director log -->
### planner
- 2026-11-15 · R-255 · draft · did: filed from ADR-0009 · next: director
<!-- end planner log -->
