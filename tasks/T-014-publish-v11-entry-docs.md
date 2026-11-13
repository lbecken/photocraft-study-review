---
id: T-014
title: "Publish the v1.1 versions of AGENTS.md and the charter's tier and iteration lines"
tier: low
state: self_verified
owner_role: director
scorecard_items: []
depends_on: []
touches:
  - AGENTS.md
  - docs/CHARTER.md
budget_tokens: 150000
iteration: 1
found_by: planner
origin_task: null
---

# T-014 Publish the v1.1 versions of AGENTS.md and the charter's tier and iteration lines

## Goal
The agent entry point `AGENTS.md` and `docs/CHARTER.md` §4–§5 match methodology v1.1
(decision D-035, ADR-0002), which is already on `main` under `methodology/` and `docs/adr/`.

## Why this is a task
Neither file is a ledger path, so the director may not commit them to `main` directly; the
queue's main-history check would refuse every later run. The text was written by the
methodology author in run R-053/R-054; this task only carries it through the gate.
(Director-written spec for a mechanical change; see decision D-036.)

## Acceptance criteria
- AC-1: `AGENTS.md` and `docs/CHARTER.md` on the branch are byte-identical to the author's files.
- AC-2: the gate is green.
- AC-3: no other path changes.

## Out of scope
Stale references in `docs/development.md` and `xtask` messages (task T-013).

## Decisions
- None.

## Questions
- None open.

## Log
### director
- 2026-11-13: published by the director acting as implementer of a mechanical task.
