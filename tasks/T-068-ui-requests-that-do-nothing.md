---
id: T-068
title: "Refuse UI requests that can do nothing, and make the no-window line state its reason"
tier: medium            # provisional: behaviour of the script mode's refusals and one message; no dependency, no gate path
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-065]
touches:                # provisional, fixed at specification; each path one role
  - crates/kiln-ui/src/script.rs
  - crates/kiln-ui/src/window.rs
  - docs/protocol/ui.md
budget_tokens: 800000   # medium (v1.3)
iteration: 2
found_by: evaluator     # evaluations/iter-2-cut-1.md, "Wired but inert" and Q1 (run R-253)
origin_task: T-055
---

# T-068 Refuse UI requests that can do nothing, and make the no-window line state its reason

This is a draft filed from the evaluation at cut 1 (`evaluations/iter-2-cut-1.md`, run R-253; D-169). Interface,
Pinned and acceptance criteria are written by a planner, and the spec is checked before dispatch.

## Goal
Three UI requests are accepted and answer ok while nothing can happen: a `scroll` with `direction` `left` or
`right` on a target that cannot scroll sideways (`Editor`, `Documents`), and a secondary click on `Open` or on
`Editor`. After this task a request that cannot act on its target is refused as the other requests that cannot act
are, or the document says what it does. And the line printed when no window can be had is cut at 200 chars before
it states the reason and carries a path of the build machine; after this task it states the reason and no such path.

## Evidence in the record
- `evaluations/iter-2-cut-1.md`, "Wired but inert": `scroll` left and right change nothing on `Editor` or
  `Documents`; a secondary click on `Open` or `Editor` does nothing; the documents do not specify either.
- `evaluations/iter-2-cut-1.md`, Q1: the no-display line is cut at 200 chars before the reason and contains
  `/root/.cargo/registry/...`.

## Open questions for the specification
- Refuse (`not_actionable`) or document as no-ops? A refusal changes what `kiln script` answers for scripts that
  send such requests today.
- T-057 to T-063 add panels that may scroll sideways or take a secondary click: the rule must not forbid them.

## Log
### director
- 2026-11-15 filed as a draft (D-169).
