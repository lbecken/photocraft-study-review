---
id: INC-NNN              # file incidents/INC-NNN-slug.md
title: "<what broke>"
class: red-main          # red-main | escaped-defect | process | data-loss
detected_at: YYYY-MM-DDTHH:MMZ
detected_by: <GitHub Actions run on main / local gate / evaluator / role>
red_from: YYYY-MM-DDTHH:MMZ    # completion of the first red run on main (red-main only)
green_at: YYYY-MM-DDTHH:MMZ    # completion of the next green run on main
red_minutes: 0
origin_task: T-NNN       # task whose merge introduced it, if known
fix_task: T-NNN
---

# INC-NNN <title>

## What happened
Facts only, with shas, task IDs and queue records. No blame.

## Timeline (UTC)
| Time | Event |
|---|---|
| | origin task merged by the queue |
| | detected |
| | dispatching stopped |
| | fix or revert merged by the queue |

## Why the gate and queue did not stop it
Which step should have caught it and why it did not: not in the gate, nondeterminism,
environment difference between local and GitHub Actions, a direct push, an unchecked rule.

## Immediate fix
Fix or revert task and what it changed.

## Permanent fix
| Type | Description | Task |
|---|---|---|
| Mechanical check / test / gate step | | |
| Rule only — why no check is possible | | |

## Methodology impact
Proposed change for the next retrospective, if any (cite this incident).
