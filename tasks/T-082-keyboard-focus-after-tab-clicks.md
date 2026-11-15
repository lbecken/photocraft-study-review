---
id: T-082
title: "After a click on a tab, a close button or a Documents entry the editor has the keyboard again"
tier: medium            # provisional: behaviour of UI modules over existing commands
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-058]
touches:                # provisional, fixed at specification
  - crates/kiln-ui/src/panels/tabs.rs
  - crates/kiln-ui/src/shell.rs
  - docs/protocol/ui-tabs.md
budget_tokens: 800000   # medium (v1.4)
iteration: 2
found_by: reviewer      # reviews/T-058-r1.md finding 6 (run R-283); the implementer's report of R-279
origin_task: T-058
---

# T-082 The editor has the keyboard again after a tab click

This is a draft filed from the review of T-058 (run R-283; D-201). Interface, Pinned and acceptance criteria are
written by a planner, and the spec is checked before dispatch.

## Goal
A click on a tab, on a tab's close or `Discard` button, or on a `Documents` entry takes the keyboard focus away
from the editor and gives it to nothing, so typing and `ctrl+S` do nothing until the editor is clicked. After this
task the editor of the current document has the focus after such a click, when a document is open.

## Evidence in the record
- `reviews/T-058-r1.md`, finding 6: after a refused close `ctrl+S` saves nothing until `Editor` is clicked or
  focused; `docs/protocol/ui-tabs.md` documents it (T-058).
- The implementer's report of run R-279, deviation 5.

## Open questions for the specification
- Does the same hold after a click in the project panel (T-057) and on `Open` (T-055)? One rule for all of them.
- Which merged tests give the editor focus by hand and stay valid either way?

## Log
### director
- 2026-11-15 filed as a draft (D-201).
