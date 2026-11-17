---
id: T-086
title: "Find where exit status 5 of a window run came from, seen once in 127 runs"
tier: low               # provisional: an investigation; it changes nothing until it finds a cause
state: draft
owner_role: planner
scorecard_items: []
depends_on: []
touches:                # provisional, fixed at specification
  - docs/protocol/ui.md
budget_tokens: 150000   # low (v1.4)
iteration: 2
found_by: evaluator     # evaluations/iter-2-cut-2.md, Q1 (run R-302)
origin_task: T-065
---

# T-086 An exit status 5 seen once

This is a draft filed from the evaluation at cut 2 (run R-302; D-215), question Q1: the director's answer is
that a one-off is filed, so that a second sighting has a place to go.

## Goal
One window run of 127 (`kiln --script` under `xvfb-run`, screen 1280x800) exited 5 where `kiln script` exits 3 for
the same script, with identical stdout and files and an empty stderr of `kiln`; 40 reruns exited 3. Exit status 5
is not in `docs/protocol/ui.md`'s table. This task finds whether `kiln`, the window system or `xvfb-run` produced
it (the evaluator kept `kiln`'s stderr, not `xvfb-run`'s), and documents or removes it.

## Evidence in the record
- `evaluations/iter-2-cut-2.md`, "Questions for the director", Q1; driver `evaluations/iter-2-cut-2-drivers/rc5.sh`.
- An earlier sighting (found by the retrospective of cut 2, D-219): the test author's run R-264 of T-070 and
  ruling m2 of D-182 record an exit status 5 of a window run under `xvfb-run`, with `xvfb-run` as the likely
  cause. This task starts from that record.

## Open questions for the specification
- Does the CI job `window-linux` (which runs the window procedure on every code push) ever show it? Its record
  is the cheapest place to watch.

## Log
### director
- 2026-11-16 filed as a draft (D-215).
