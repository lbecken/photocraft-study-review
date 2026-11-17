---
id: T-088
title: "Make metrics transition write the task file's state line, refuse a run booked with start equal to end, and check that every cited decision has a row and every review counts own-test kills"
tier: high              # ADR-0002 "a gate path": xtask/; fixed at specified (v1.5 §4); the approval row must contain `gate paths`
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-043, T-087]
touches:                # provisional, fixed at specification; one role per path (D-153)
  - xtask/src/metrics.rs                   # GATE PATH: implementer: `transition` writes `state:`; `add` refuses start = end
  - xtask/src/hygiene.rs                   # GATE PATH: implementer: cited `D-NNN` rows; `killed_by_own_tests` in review front matter
  - xtask/src/main.rs                      # GATE PATH: implementer: usage text, only if options change
  - xtask/tests/acceptance_t088.rs        # GATE PATH: test author
  - xtask/tests/guard_t088.rs             # GATE PATH: test author
  - xtask/platform-tests.toml             # GATE PATH: test author (its entries only, if Linux-only)
  - docs/development.md                   # implementer: "## Metrics" and the `hygiene` row of "## The gate"
budget_tokens: 1100000  # high (v1.5)
iteration: 2
found_by: task:T-043  # D-144 change 12, D-146, v1.5 change 9
origin_task: null
---

# T-088 Make metrics transition write the task file's state line, refuse a run booked with start equal to end, and check that every cited decision has a row and every review counts own-test kills

This is a draft filed by the re-specification of T-043 (run R-310, at main `1289bf1`), which split
the additions recorded for T-043 since D-098 into separate tasks (T-043 Q-6). Interface, Pinned
and acceptance criteria are written by a planner before dispatch, under methodology v1.5.

## Goal
Four record checks decided in D-144 change 12 and v1.5 change 9 that no tool makes: (1) `cargo xtask metrics transition --task T-NNN --to <state>` also rewrites the `state:` line of `tasks/T-NNN-*.md` on main, so the director stops editing it by hand (D-139, D-146; `worktree new` reads that line); (2) `metrics add` refuses a run whose `started_at` equals its `ended_at` with more than 10,000 tokens (R-180); (3) the gate's `hygiene` step fails when a `D-NNN` cited in `tasks/`, `reviews/` or `docs/` has no row in `docs/decisions.md` (D-104); (4) `hygiene` requires `killed_by_own_tests` in a round-1 review's `mutants` (v1.5 change 9, D-212).

## Evidence in the record
- D-144 change 12, accepted for T-043's scope: "every cited D-NNN has a row; a run with start equal to end is refused; `metrics transition` writes the task file's state line".
- D-146: "the director writes the state line until T-043".
- v1.5 change 9 / D-219 (9): `killed_by_own_tests` in the review template now, in the tools with T-043.
- `metrics add` already refuses an end time later than now and one earlier than the task's latest record (`xtask/src/metrics.rs` lines 344–357, 506–513); nothing to build for those (R-297, R-301).

## Open questions for the specification
- (1): the state line's comment after `state:` (the template's `# director only …`) is kept; is the commit the director's (the tool writes, the director commits) or the tool's?
- (3): citations inside code blocks and in quoted upstream text count? Rows cited before they exist in the same commit?
- (4): existing review files without the field (rounds before v1.5) are exempt by date or by round?
- Platform: the director's tools run on Linux only (ADR-0009 (c), D-199). New test files are
  Linux-only with entries in `xtask/platform-tests.toml`, as T-043 Q-3 decides for its own, or
  portable after a probe on `ci/T-088`.

## Log
### director
<!-- end director log -->
### planner
- 2026-11-17 · R-310 · (new) · did: filed as a draft from T-043's split (T-043 Q-6). · gate attempts: 0 · missing context: none · next: director.
<!-- end planner log -->
