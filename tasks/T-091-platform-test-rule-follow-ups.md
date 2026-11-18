---
id: T-091
title: "Close the follow-ups of the platform-test rule: a lone helper with a platform statement, enclosing cfgs two levels up under test, two messages"
tier: high              # provisional: xtask/ is a gate path
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-071]
touches:                # provisional, fixed at specification
  - xtask/src/platforms.rs
  - xtask/src/noskip.rs
  - xtask/tests/acceptance_t091.rs
  - xtask/tests/guard_t071.rs
  - docs/development.md
budget_tokens: 1100000  # high (v1.5)
iteration: 2
found_by: reviewer      # reviews/T-071-r2.md, findings R2-2, R2-3, R2-5, R2-6, R2-7 (run R-317)
origin_task: T-071
---

# T-091 Follow-ups of the platform-test rule

This is a draft filed from the second review of T-071 (run R-317; D-232). It is re-specified in full before any
dispatch (v1.5 §5.7).

## Goal
The rule of T-071 has one hole left by its rulings and two untested cases:
- **R2-2 (spec gap).** A single helper (no twin) that holds a `#[cfg(unix)]` statement and is called by a test
  passes with no finding: ruling s4 inspects test fns at any depth, not the helpers they call, and the limits of
  s8 do not list it. Either the rule covers helper fns in test code, or the document lists the limit.
- **R2-3 (test gap).** No gate test puts a platform cfg two levels above an item (`mod` in `mod`, `impl` in
  `mod`): two mutants survive (only the innermost `mod`'s cfg counts; an `impl`'s cfg ignores the enclosing
  `mod`). The code is right today.
- **R2-5 (nit).** For an item that is `cfg-disabled` by an empty intersection, the mismatch message says "the
  item has no platform cfg (it runs on every platform)".
- **R2-6 (nit).** `guard_t071`'s assertion that accepts `cfg-disabled` for a conditional `ignore` is wider than
  "s1 clarified" needs.
- **R2-7 (nit).** Twins inside a `#[cfg(feature = …)]` module are not reported.
- **R4-1 (test gap; `reviews/T-071-r4.md`, run R-324).** Nothing notices a clean fixture file the self-test stops
  reading (`platform-clean.rs`; true of `clean.rs` before this task too).
- **From T-079 (D-241, ruling w2).** `xtask/tests/guard_t079.rs` pins every Linux test name of main `ee9b130`
  as a constant; when the platform batch ends, decide whether it is retired or replaced by a tool that
  regenerates the list.
- **From T-079's review (D-242, ruling x4; `reviews/T-079-r1.md` finding 5).** `#[cfg_attr(<predicate>,
  cfg(...))]` on a test takes it out of the test list and passes `no-skip` with no finding.
- **From T-079's review round 2 (D-243; `reviews/T-079-r2.md`).** (1) If both case-folding helpers of
  `tree_order_and_counts` filter whatever the probe says, the test still passes on Linux: one assertion closes
  it. (2) The `cfg_attr` scan of `guard_t079.rs` has no self-test: undoing it passes the gate. (3) Two paths can
  still reach the recursive drop of the 2,100-deep chain: a panic while the chain is built, and a take-down that
  fails part-way while the test is already panicking.

## Follow-ups from T-072 (director, D-265; `reviews/T-072-r2.md`)
- The failure printer prints the first 20 lines of a failing test's block: a panic message after more than 20
  lines of output is in the artefact, not in the job log (`tasks/ci-runs/T-072-negative.md`). Print the panic lines
  of a failing block wherever they fall.
- No test has the `error: N targets failed` line inside an open block (mutant M23 survives).
- Since ruling u5 widened G-1 to 180 s, a probe limit of 150 s passes the guard (M21): pin the limit by the step's
  text or by the measured time minus the limit.

## Follow-up from INC-002 (director, D-274)
A CI record should mark a job the host cancelled before any step with no runner (`runner_id` 0) as
"no result (no runner)" by rule, so that it is neither read as red nor missed (`incidents/INC-002-…md`).

## Log
### director
- 2026-11-18 filed as a draft (D-232).
