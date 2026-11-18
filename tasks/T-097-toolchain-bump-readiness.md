---
id: T-097
title: "Make the tools ready for a toolchain whose cargo lays out target/ differently"
tier: high              # gate paths under xtask/src (provisional)
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-096]
touches: []             # fixed at specification
budget_tokens: 1100000
iteration: 2
found_by: planner       # T-096's specification (R-364), ruling Q-4 (D-259)
origin_task: T-096
---

# T-097 Make the tools ready for a toolchain whose cargo lays out target/ differently

Draft stub (director, D-259). Written in full at specification, under the methodology version then in force.

## Goal
T-096's probe showed that cargo 1.100.0 puts nothing in `target/debug/deps` (a member's output goes to
`target/debug/build/<package>/<hash>/out`). The repository pins 1.97.0, so nothing is wrong today. Listed by the
planner of T-096 as assuming the old layout or typing the channel: the pruning in `xtask/src/seed.rs` and
`xtask/src/publish.rs`, and three test files that type the channel (named in T-096's spec, "Out of scope and
implementer notes"). This task makes a toolchain bump a change of one file and has the tools fail naming the cause
where a layout assumption breaks. Not urgent: it is needed before the pinned channel moves.

## Log
### director
<!-- end director log -->
### planner
<!-- end planner log -->
