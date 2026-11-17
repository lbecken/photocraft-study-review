---
id: T-087
title: "Make the queue refuse a task whose tier is below the highest tier of its claimed items, and accept a review file in a low-tier task's diff"
tier: high              # ADR-0002 "a gate path": xtask/; fixed at specified (v1.5 §4); the approval row must contain `gate paths`
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-043]
touches:                # provisional, fixed at specification; one role per path (D-153)
  - xtask/src/precheck.rs                  # GATE PATH: implementer: `front-matter` (the tier rule) and `touches` (the review file)
  - xtask/tests/acceptance_t087.rs        # GATE PATH: test author
  - xtask/tests/guard_t087.rs             # GATE PATH: test author
  - xtask/platform-tests.toml             # GATE PATH: test author (its entries only, if Linux-only)
  - docs/development.md                   # implementer: "### Preconditions" (`front-matter`, `touches`)
budget_tokens: 1100000  # high (v1.5)
iteration: 2
found_by: task:T-043  # D-174 (1), D-179
origin_task: null
---

# T-087 Make the queue refuse a task whose tier is below the highest tier of its claimed items, and accept a review file in a low-tier task's diff

This is a draft filed by the re-specification of T-043 (run R-310, at main `1289bf1`), which split
the additions recorded for T-043 since D-098 into separate tasks (T-043 Q-6). Interface, Pinned
and acceptance criteria are written by a planner before dispatch, under methodology v1.5.

## Goal
Two rules the director applies by hand are checked by the queue. (1) v1.4 change 1 (D-174 (1)): a task's `tier` is at least the highest `tier` of the scorecard items it claims, read from main's `scorecard/items/<id>.toml`; a lower tier is refused at `front-matter` (recorded, T-043). (2) A low-tier task whose reviewer wrote a review file has that file in its diff; today the `touches` check refuses it (T-069, D-179).

## Evidence in the record
- D-158, D-162: T-056 re-tiered in flight (it claimed the high item `ui.dispatch-only`), two queue failures followed; v1.4 change 1 says the tool check is pending T-043.
- D-179: T-069 refused at `touches` for `reviews/T-069-r1.md` (a low-tier task with a reviewer's read); "For T-043's scope: a low-tier task with a reviewer's read needs the review file allowed".
- `xtask/src/precheck.rs` reads main's `scorecard_items` already (lines 309, 631).

## Open questions for the specification
- Which review files are allowed for a low tier: `reviews/T-NNN-rN.md` of the task only, or anything under `reviews/` (the `hygiene` step checks review front matter for medium and high only)?
- An item file that does not exist on main (a new item a task adds): its tier is read from the task head, or the check refuses?
- Platform: the director's tools run on Linux only (ADR-0009 (c), D-199). New test files are
  Linux-only with entries in `xtask/platform-tests.toml`, as T-043 Q-3 decides for its own, or
  portable after a probe on `ci/T-087`.

## Log
### director
<!-- end director log -->
### planner
- 2026-11-17 · R-310 · (new) · did: filed as a draft from T-043's split (T-043 Q-6). · gate attempts: 0 · missing context: none · next: director.
<!-- end planner log -->
