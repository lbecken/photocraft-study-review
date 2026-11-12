---
id: ADR-NNNN             # file docs/adr/NNNN-slug.md, numbered, never deleted
title: "<decision in a few words>"
status: proposed         # proposed | accepted | rejected | superseded
date: YYYY-MM-DD
task: T-NNN              # the high-tier task that carries the change
charter_section: "§3 decision N"   # which binding contract this changes
approval: D-NNN          # row in docs/decisions.md
supersedes: null
superseded_by: null
---

# ADR-NNNN <title>

## Context
The forces: which contract, why it no longer fits, evidence (task logs, metrics, incidents).
Facts separate from opinion.

## Options
| Option | For | Against |
|---|---|---|
| A | | |
| B | | |
| Keep the current contract | | |

## Decision
The rule agents follow from now on, stated so it can be checked.

## Consequences
- Tasks and crates affected; consumer tasks that must follow this one.
- CHARTER §3 text to change (exact).
- Check that enforces it (xtask step, test name), or "rule only, because …".

## Verification
Tests or oracles that show the new contract holds.

## Rollback
How to undo it and what that costs.
