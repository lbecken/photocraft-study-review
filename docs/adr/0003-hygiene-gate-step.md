---
id: ADR-0003             # file docs/adr/0003-hygiene-gate-step.md
title: "A `hygiene` gate step checks review front matter and session-specific paths"
status: accepted
date: 2026-11-13
task: T-013
charter_section: "§4 gate steps"
approval: null           # director: the D-NNN row that accepts it
supersedes: null
superseded_by: null
---

# ADR-0003 A `hygiene` gate step checks review front matter and session-specific paths

## Context
CHARTER §4 lists the gate's steps, and the charter changes only by ADR (METHODOLOGY §14).
Methodology v1.1 adds two rules whose check is "mechanical once T-013 lands" and is a director
check until then:
- §7: every round-1 review of a medium or high task seeds at least 10 mutants and records
  `mutants: {introduced, equivalent, killed, killed_by_acceptance, survived}` in its front
  matter, so that retrospective 1 can compute the acceptance kill rate by tier and arm (§13).
  In iteration 0 the T-005 reviews lacked these counts (retro-0 proposed change 8;
  `methodology/CHANGELOG.md` v1.1 change 8). Without a check, the metric depends on each
  reviewer remembering the template.
- §12: no session identifiers, including session-specific scratch paths, in code, docs or
  records. Four tracked review files contain one today (`git grep -nE` with the pattern in
  T-013 Interface (6): `reviews/T-003-r1.md:39`, `reviews/T-005-r1.md:15`, `reviews/T-005-r2.md:18`,
  `reviews/T-005-r3.md:17`); the rule was unchecked (retro-0 change 15).
Both checks read only text files of the checkout, take well under a second and need no build,
so they belong early in the gate.

## Options
| Option | For | Against |
|---|---|---|
| A One new step `hygiene` carrying both checks | one charter change; one name to learn; both are cheap text checks on records | one step mixes two concerns (each problem line names its check) |
| B Two new steps (`reviews`, `session-paths`) | one concern per step | two charter entries for two small checks |
| C Fold into existing steps (`scorecard --check`, `no-skip`) | no charter change | hides unrelated checks inside steps whose names say otherwise; the charter's list stays nominally true but misleads |
| Keep the current contract | nothing changes | both rules stay unchecked; §7's metric can be incomplete at retrospective 1 |

## Decision
The gate (`cargo xtask ci`) gains one step, `hygiene`, run after `ui-commands` and before
`schema`, also runnable alone as `cargo xtask hygiene`. It fails when either check finds a
problem:
- **reviews**: every `reviews/T-NNN-rN.md` of a task with tier `medium` or `high` and
  `iteration ≥ 1` has the v1.1 review front matter (`task` and `round` matching the file name,
  `tier` and `arm` matching the task, and `mutants` with five non-negative integers that add up,
  with at least 10 introduced in round 1);
- **session-paths**: no tracked file contains a session-specific scratch path (the regular
  expression is fixed in T-013 Interface (6)).
The exact rules are T-013's spec. There is no allowlist for either check.

## Consequences
- Task T-013 implements the step, redacts the four existing session paths, and changes
  CHARTER §4's gate line from
  "Steps, cheap first: fmt, clippy `-D warnings`, `layers`, `ui-commands`, `schema`, …" to
  "Steps, cheap first: fmt, clippy `-D warnings`, `layers`, `ui-commands`, `hygiene` (review
  front matter, no session-specific paths), `schema`, …" (rest unchanged). `docs/CHARTER.md` is
  not a ledger path, so the change reaches `main` through T-013 and the queue.
- Reviewers of medium/high tasks of iteration ≥ 1 must fill the `mutants` front matter or the
  task's gate fails in the queue; iteration-0 reviews are not checked.
- Agents must not write session-specific scratch paths into tracked files; tests that need one
  build the string at run time.
- METHODOLOGY §7 and §12 name T-013's check; the "(T-013)" marks become stale and are updated at
  retrospective 1.
- Check that enforces it: the `hygiene` step itself; T-013 AC-11, AC-12.

## Verification (how retrospective 1 judges it)
- Every medium/high review of iteration 1 has complete `mutants` counts (`reviews/` front
  matter), so the acceptance kill rate can be computed for every such task.
- `git grep` with the session-path pattern over `main` finds nothing at iteration 1's close.
- Cost: the step's time in gate reports (expected well under 1 s); the number of queue runs it
  failed, and whether each failure was a real omission.

## Rollback
A new ADR superseding this one removes `hygiene` from CHARTER §4, and a high-tier task removes
the step from `xtask`. Cost: one task; the two rules return to director checks.
