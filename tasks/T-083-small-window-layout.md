---
id: T-083
title: "At a small window with many tabs the status bar is painted and a Documents click still switches"
tier: medium            # provisional: behaviour of UI modules over existing commands
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-059, T-082]
touches:                # provisional, fixed at specification
  - crates/kiln-ui/src/shell.rs
  - crates/kiln-ui/src/panels/tabs.rs
  - docs/protocol/ui-status.md
  - docs/protocol/ui-tabs.md
budget_tokens: 800000   # medium (v1.4)
iteration: 2
found_by: planner       # spec check R-295 of T-059 (D-206)
origin_task: T-059
---

# T-083 The layout at a small window with many tabs

This is a draft filed from the spec check of T-059 (run R-295; D-206). Interface, Pinned and acceptance criteria
are written by a planner, and the spec is checked before dispatch.

## Goal
At `--size 800x320` with 300 open documents the tab strip takes the height the status bar needs, and a click on a
`Documents` entry does not change `Title`. After this task the bar's four nodes each paint their value at that
size, and a `Documents` click switches as at the default size, or the document states exactly what a small window
shows.

## Evidence in the record
- Spec check R-295 in `tasks/T-059-ui-status-bar.md`: at 800x320 with 300 documents the bar's four nodes exist, are
  not `offscreen`, and none has a row.
- The same run's side finding: at that size a `Documents` click did not change `Title` (cause not investigated;
  T-058's criteria click `Documents` at the default size only).

## Open questions for the specification
- Is the `Documents` finding a defect of T-058 (the click is refused, or lands on another node) or the script's
  target being out of view? Reproduce first; if it is a defect at any size a test can reach, it is filed as such.
- Which panel gives way when the window is small: a cap on the strip's height, or a minimum for the editor and
  the bar?

## Log
### director
- 2026-11-16 filed as a draft (D-206).
