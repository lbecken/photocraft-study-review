---
id: T-090
title: "Make the queue refuse a task whose join left a CI criterion pending until a CI record shows the counted jobs green on its rebased head"
tier: high              # ADR-0002 "a gate path": xtask/; fixed at specified (v1.5 §4); the approval row must contain `gate paths`
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-089]
touches:                # provisional, fixed at specification; one role per path (D-153)
  - xtask/src/precheck.rs                  # GATE PATH: implementer: a new check `ci` before `approval`
  - xtask/src/join.rs                      # GATE PATH: implementer, only if the pending criterion is written by the join (T-033 shares this file)
  - xtask/tests/acceptance_t090.rs        # GATE PATH: test author
  - xtask/platform-tests.toml             # GATE PATH: test author (its entries only, if Linux-only)
  - docs/development.md                   # implementer: "### Preconditions", "## CI"
budget_tokens: 1100000  # high (v1.5)
iteration: 2
found_by: task:T-043  # v1.5 change 11, D-219 (11), ADR-0009 Decision 5
origin_task: null
---

# T-090 Make the queue refuse a task whose join left a CI criterion pending until a CI record shows the counted jobs green on its rebased head

This is a draft filed by the re-specification of T-043 (run R-310, at main `1289bf1`), which split
the additions recorded for T-043 since D-098 into separate tasks (T-043 Q-6). Interface, Pinned
and acceptance criteria are written by a planner before dispatch, under methodology v1.5.

## Goal
v1.5 change 11: a CI criterion pending at the join is today written into the approval or queue decision row and checked by the director. After this task the queue refuses (a recorded refusal, T-043) a task with a pending CI criterion until a CI record (T-089's file) names the task's rebased head and shows every counted job green, as ADR-0009 Decision 5 proposed for the queue.

## Evidence in the record
- v1.5 change 11, D-219 (11): "the pending CI criterion is written in the approval or queue decision row until T-043 makes the queue check it".
- ADR-0009 Decision 5 ("Interim: rule only … Later, PROPOSED: a queue precondition reading the recorded result").
- T-079 AC-6 and T-070's spec: the `CI-REQUEST` protocol.

## Open questions for the specification
- Where the pending criterion is recorded (a join record field, T-033's tool; a field in the task's front matter; the decision row as today) and which jobs count (decision rows, D-202, not the workflow file).
- The rebased head the queue would fast-forward to is known only after the rebase: the check runs after it, as `dependencies` does?
- Platform: the director's tools run on Linux only (ADR-0009 (c), D-199). New test files are
  Linux-only with entries in `xtask/platform-tests.toml`, as T-043 Q-3 decides for its own, or
  portable after a probe on `ci/T-090`.

## Log
### director
<!-- end director log -->
### planner
- 2026-11-17 · R-310 · (new) · did: filed as a draft from T-043's split (T-043 Q-6). · gate attempts: 0 · missing context: none · next: director.
<!-- end planner log -->
