---
id: T-NNN
title: "<imperative, behaviour-level title>"
tier: medium            # low | medium | high — highest applicable (METHODOLOGY §4)
state: draft            # director only; see METHODOLOGY §3
owner_role: planner     # role expected to act next
scorecard_items: []     # e.g. [text-undo-redo]; non-empty ⇒ tier ≥ medium
depends_on: []          # task IDs that must be merged first
touches:                # paths the implementer may change (files or directories)
  - crates/kiln-xxx/src/yyy.rs
budget_tokens: 400000   # total across all roles; low 150k, medium 400k, high 700k (v1)
# methodology v1 extensions (needed by METHODOLOGY §13 metrics):
iteration: 1
found_by: planner       # planner | evaluator | reviewer | queue | incident | task:T-NNN
origin_task: null       # the task whose defect this fixes, if any
---

# T-NNN <title>

## Goal
What a user or caller can do after this task that they cannot do now. One paragraph.

## Reading list
Only what this task needs: CHARTER sections, ADRs, files.

## Interface
The public surface the tests and the evaluator use. For a command: id, params and result
(JSON Schema or a typed example), error cases. For a library: the public Rust items, exact
signatures. State "unchanged" if the task only changes behaviour behind an existing interface.

## Acceptance criteria
Each is observable behaviour through the interface above, with its oracle.
- AC-1: Given …, when <command/CLI call>, then <observable result>. Oracle: <naive model / round-trip / corpus / exact value>.
- AC-2: …

## Out of scope
What a reasonable implementer might do but must not, and which task covers it.

## Implementer notes
Unit-test expectations, determinism concerns, performance budget if any.

## Questions
Planner and director only (builders put `QUESTION:` lines in their log section; the director
copies them here with the answer). Q-1 (role, R-NNN): question → answer (director, date).

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · tokens (if known) · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
