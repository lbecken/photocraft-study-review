---
id: ADR-0002             # file docs/adr/0002-tier-by-boundary.md
title: "Risk tier by the boundary a change crosses; review depth by tier"
status: accepted
date: 2026-11-13
task: null               # adopted at retrospective 0, not carried by a task; tooling in T-013
charter_section: "§4 risk tiers"
approval: D-035
supersedes: null
superseded_by: null
---

# ADR-0002 Risk tier by the boundary a change crosses; review depth by tier

## Context
CHARTER §4 set the tiers as low (docs, tests, internal refactor in one module), medium (feature
in one crate) and high (§3 contracts, command schema, persistence/save path, CI and gate,
dependencies, anything in two or more layers). Iteration-0 facts (`retrospectives/iter-0.md`,
§Tiering, §Where defects were found, §Cost and convergence of review):
- 5 of 5 feature tasks were high, each correctly under the rule as written: `Cargo.lock` or
  "two layers" alone makes almost any feature high (T-001 through `Cargo.lock`, T-004 through
  two layers). There were 0 medium tasks, so claim C5 had no contrast.
- High and medium differed only by the director's approval row (METHODOLOGY v1.0 §4, §7). The
  row followed the reviewer's verdict every time and added no finding; review depth was the
  same for every tier.
- 7 of 7 defects found in review were in the tasks that cross the operating-system or trust
  boundary: T-002 (file system side effects), T-004 (paths, processes, a security claim in a
  document), T-005 (gate). T-001 and T-003 (in-memory L0/L3) had one review round and 0 defects.
- 6 of the 7 defects came from the reviewer using the product adversarially; all 12 test gaps
  came from mutation spot-checks. Revert checks (2) caught nothing.
These are five tasks of headless L0–L4 work; the clustering is a description, not an estimate.

## Options
| Option | For | Against |
|---|---|---|
| A Tier by boundary, review depth by tier | discriminates on the evidence (T-001, T-003 become medium); makes C5 testable | lets in-memory defects get lighter review; untested on medium tasks |
| B Keep the rule, add review depth only | one change | everything stays high; still no contrast for C5 |
| Keep the current contract | no change | C5 stays untestable; "two layers" keeps every feature high |

## Decision
The tier is the highest that applies:
- **high**: product code on the file system's persistence path or that spawns processes; a gate
  path (as the queue defines them); a security or confinement boundary; adding or changing a
  dependency (any `Cargo.lock` or dependency-table change); a change to a CHARTER §3 contract.
- **medium**: the default for a behaviour change that fits no high criterion; typically a
  feature inside one crate that claims scorecard items. Scorecard items ⇒ at least medium.
  Adding a command is not a contract change; changing how the registry, schema validation or
  the command log work is.
- **low**: documentation, tests, or refactoring with no behaviour change.

Review depth: low — gate only; medium — one full review round with an acceptance-criteria check
through the public interface and a mutation spot-check of at least ten seeded bugs, no approval
row; high — as medium plus adversarial probing of the boundary and the director's approval row.
Later rounds follow the stopping rule of METHODOLOGY §7.

## Consequences
- CHARTER §4 "Risk tiers" sentence replaced by the rule above, citing this ADR. Removed criteria:
  "anything in two or more layers" and the standalone "command schema" (see the medium rule).
- METHODOLOGY §4 and §7, AGENTS.md definition of done, planner and reviewer briefs, task and
  review templates follow it. Budgets per tier: high 850k, medium 500k (provisional), low 150k.
- Medium tasks run the D-035 (16) experiment arm (METHODOLOGY §6).
- Checks: the queue's `gate-path` and `dependencies` preconditions make a non-high task that
  changes a gate path or a dependency mechanically refusable, and `approval` refuses a high task
  without its row. Persistence, processes and security boundaries are judged by the director at
  `specified` and by the reviewer (rule only: they cannot be read off paths reliably).
- Any dependency change is high, workspace-internal edges included, matching the queue
  (the retrospective had proposed medium for internal edges; the director chose consistency).

## Verification (how retrospective 1 judges it)
- Tier mix: share of tasks per tier; the rule discriminates if medium holds at least ~2 in 5.
- C5: review findings by class, escaped defects by origin tier, and tokens per merged task, by
  tier. Supported if escaped defects per task are not higher in medium than in high while medium
  costs less; refuted if medium tasks show more escaped defects than high tasks after lighter
  review. With fewer than ~5 tasks per tier the verdict stays "insufficient".
- Tier disputes: tasks re-tiered after `specified`, and defects in medium tasks that a boundary
  criterion should have caught.

## Rollback
Restore the v1.0 sentence in CHARTER §4 by a new ADR superseding this one; tasks already merged
keep their recorded tier. Cost: one methodology change at a retrospective; no code depends on it
beyond the queue's existing preconditions.
