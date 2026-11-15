---
id: T-081
title: "Make a frame of the project panel cost what its visible rows cost, not what all shown entries cost"
tier: medium            # provisional: behaviour of a UI module; it changes what a snapshot reports for rows out of view
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-057]
touches:                # provisional, fixed at specification
  - crates/kiln-ui/src/panels/project.rs
  - docs/protocol/ui-project.md
budget_tokens: 800000   # medium (v1.4)
iteration: 2
found_by: reviewer      # reviews/T-057-r1.md finding 4 (run R-269)
origin_task: T-057
---

# T-081 Make a frame of the project panel cost what its visible rows cost

This is a draft filed from the review of T-057 (run R-269; D-188). Interface, Pinned and acceptance criteria are
written by a planner, and the spec is checked before dispatch.

## Goal
The project panel lays out every shown entry on every frame, so the time of a frame grows with the number of
entries shown. After this task a frame costs what the rows in view cost, and the document says what a snapshot
reports for entries out of view.

## Evidence in the record
- `reviews/T-057-r1.md`, finding 4: debug build, 50,000 entries shown: the expand took 18.0 s, each later frame
  0.76 s, 600 idle frames 475.9 s.
- `docs/protocol/ui-project.md`, the limit T-057 states (ruling r4).

## Open questions for the specification
- Today entries out of view are nodes with `offscreen: true`. If they are not laid out, are they still reported
  (from the panel's own list) or absent? Tests of T-057 that find entries before scrolling to them depend on it.
- The same question holds for `Documents` (T-055) and the editor's rows (T-056, which already paints only the
  rows in view).

## Note from T-057's review round 2 (director, D-191)
The limit sentence in `docs/protocol/ui-project.md` quotes "18.0 s for the expand"; the reviewer says that figure
was the time of a whole script that started the session and expanded both the root and the large directory. This
task re-measures and corrects the sentence.

## Log
### director
- 2026-11-15 filed as a draft (D-188).
