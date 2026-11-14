---
id: T-054
title: "Make a debug build of the workspace take less disk and time with a lighter debug-info setting, keeping file and line in test backtraces"
tier: high              # ADR-0002 "a gate path": the root Cargo.toml's [profile.*] tables are gate tables (docs/development.md, gate-path row); the approval row must contain `gate paths`
state: draft
owner_role: planner
scorecard_items: []
depends_on: []
touches:                # provisional, fixed at specification
  - Cargo.toml          # GATE PATH: [profile.dev] (and [profile.test] if needed) only
  - docs/development.md # the disk and build-time figures
budget_tokens: 1100000  # high (v1.3); a small change, expected far below the ceiling
iteration: 2
found_by: planner       # D-144 change 14 (retro-2s1 proposal 14); D-097 has the measurement
origin_task: null
---

# T-054 A lighter debug build profile, before the UI's dependencies arrive

This is a draft filed with the plan of the second stage (ADR-0007, run R-225). Interface, Pinned and
acceptance criteria are written by a planner (METHODOLOGY v1.3).

## Goal
Disk, not the builder cap, limited dispatch in the first stage: a task worktree's build takes
about 6.3 GB, feature tasks were dispatched "as the disk allows (15 GB free before each dispatch)",
and one join waited for disk (D-097, D-105; retro-2s1 section 8). The UI adds about 1 GB and about
54 s to a debug build of the UI crates (ADR-0008 c2). After this task a debug build (the gate's
`clippy` and `test` steps, a worktree, the queue) takes markedly less disk, with test failures
still reporting file and line. It is dispatched before T-055 so that the UI's dependencies are
first built under it (D-144 change 14).

## Evidence in the record
- D-097: "a lighter debug-info setting was measured (2.5 GB for the workspace's test build)" against
  about 6.3 GB per worktree build; the setting itself is not named in the row.
- ADR-0008 c2 (R-225): the UI crates' debug build in a copy of the workspace, 962 MB and 54.32 s
  with the default debug info; 728 MB and 36.20 s with `CARGO_PROFILE_DEV_DEBUG=line-tables-only`.
- Gate time grew from 241 s to 574 s over the first stage (retro-2s1 section 8); retro change 14
  asks for gate time to be reported per stage.

## Open questions for the specification
- Which setting: `debug = "line-tables-only"` for the whole dev profile, or full debug info for
  workspace members and less for dependencies (`[profile.dev.package."*"]`)? Measure both on the
  workspace's test build (disk and `cargo xtask ci` wall time) and keep the one that keeps file
  and line in a failing test's panic message.
- Does any test or tool rely on full debug info (a backtrace assertion, the stale-binary guard)?
- The change invalidates every existing `target/` once; the queue seeds worktrees from main's
  `target/`: say what the first queue run after the merge costs.

## Log
### director
<!-- end director log -->
### planner
- 2026-11-14 · R-225 · (new) · did: filed as a draft for ADR-0007 (D-144 change 14). · gate attempts: 0 · missing context: the exact setting D-097 measured · next: director.
<!-- end planner log -->
