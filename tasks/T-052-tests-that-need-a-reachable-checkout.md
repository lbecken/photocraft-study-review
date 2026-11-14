---
id: T-052
title: "Document, and say in the failure message, that five ws.path-confinement tests need a checkout another user can reach"
tier: low
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-036, T-037]
touches:                # provisional, fixed at specification
  - docs/development.md
  - crates/kiln-workspace/tests/ and apps/kiln-cli/tests/ (the precondition message of the five tests only)
budget_tokens: 150000
iteration: 2
found_by: evaluator      # evaluations/iter-2.md NEW-3 (run R-221; cause found by the director)
origin_task: T-036
---

# T-052 Document, and say in the failure message, that five ws.path-confinement tests need a checkout another user can reach

This is a draft filed from the evaluation of the first stage of iteration 2 (`evaluations/iter-2.md`,
runs R-221 and R-222; D-141). Interface, Pinned and acceptance criteria are written by a planner, and the
spec is checked before dispatch (METHODOLOGY v1.2).

## Goal
Five tests tied to `ws.path-confinement` re-run themselves under another identity. In a checkout below a directory that identity cannot traverse they fail with `precondition: cannot spawn the child run for mode 0311: Permission denied (os error 13)`, and `cargo xtask scorecard` then reports the item as failing (27 verified instead of 28). The tests do not skip, which is right; but the precondition is written nowhere and the message does not name it. After this task `docs/development.md` states the precondition and the message names the directory that blocks it.

## Evidence in the record
- `evaluations/iter-2.md`, NEW-3: `cargo xtask scorecard` exits 1 in a clean clone below `/tmp/claude-0` (mode 0700, owner root), also with the DAC capabilities dropped; the five tests are `acceptance_t036_cli::unreadable_root_is_refused_at_start`, `acceptance_t036_workspace::unreadable_root_is_refused_by_new`, `guard_t036_removal_cli::root_made_unreadable_after_start_stays_protected_through_control`, `acceptance_t037_cli::reopen_whose_walk_fails_inside_is_io_error`, `guard_t037_cli::reopen_does_not_need_the_file`.
- Director, 2026-11-14: `cargo test -p kiln-workspace --test acceptance_t036_workspace unreadable_root_is_refused_by_new` in that clone panics at `acceptance_t036_workspace.rs:217` with the message above; the primary checkout lies below directories of mode 0755 and the same tests pass there (gate of the T-038 queue run).
- What the tests name holds through the binary with the DAC capabilities dropped (the evaluator's `nodac.sh`, `confine2.py`).

## Open questions for the specification
- Should the tests place their child binary and fixtures in a directory they make reachable themselves, so the precondition goes away? That edits merged tests of a high-tier task and would raise the tier.

## Log
### director
- 2026-11-14 filed as a draft (D-141).
