---
id: T-085
title: "A click sent right after a control request that moves the layout hits its target, or the document says a frame is needed first"
tier: medium            # provisional: behaviour of the script runner (T-055), which every UI test uses
state: draft
owner_role: planner
scorecard_items: []
depends_on: []
touches:                # provisional, fixed at specification
  - crates/kiln-ui/src/script.rs
  - docs/protocol/ui.md
budget_tokens: 800000   # medium (v1.4)
iteration: 2
found_by: evaluator     # evaluations/iter-2-cut-2.md, NEW-1 (run R-302)
origin_task: T-055
---

# T-085 A click right after a layout-changing control request

This is a draft filed from the evaluation at cut 2 (run R-302; D-215). Interface, Pinned and acceptance criteria
are written by a planner, and the spec is checked before dispatch.

## Goal
A `click` whose target moved because of the control request just before it is placed from the previous frame's
layout: it answers `ok:true` and nothing happens. After this task the press is placed in the frame it is delivered
in, or `docs/protocol/ui.md` states that a click right after a control request may miss and that a frame is to be
run first, and the answer gives a sign.

## Evidence in the record
- `evaluations/iter-2-cut-2.md`, "NEW-1": after a control `doc.open` with no frame in between, a click on `Project`
  leaves it collapsed (`toggled:false`, no `project.tree` in the log); with one frame in between the same click
  expands it. Driver `evaluations/iter-2-cut-2-drivers/tabs_edges.py` (`stale`).

## Open questions for the specification
- Which merged tests send a UI request right after a control request without a frame, and do any of them pass
  only because the layout did not move? T-066 and T-068 edit the same files: order.
- A real user never sees the stale layout, so this is a fault of the test instrument, not of the window; say so.

## Log
### director
- 2026-11-16 filed as a draft (D-215).
