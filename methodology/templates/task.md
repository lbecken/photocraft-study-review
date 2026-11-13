---
id: T-NNN
title: "<imperative, behaviour-level title>"
tier: medium            # low | medium | high — boundary rule, METHODOLOGY §4 / ADR-0002
state: draft            # director only; see METHODOLOGY §3
owner_role: planner     # role expected to act next
scorecard_items: []     # e.g. [text-undo-redo]; non-empty ⇒ tier ≥ medium
depends_on: []          # task IDs that must be merged first
touches:                # exact paths the task may change, incl. test dirs; mark gate paths
  - crates/kiln-xxx/src/yyy.rs
budget_tokens: 500000   # all runs booked to the task: low 150k, medium 500k, high 850k (v1.1)
iteration: 1
found_by: planner       # planner | evaluator | reviewer | incident | task:T-NNN (who found the defect)
origin_task: null       # the task whose defect this fixes, if any
---

# T-NNN <title>

## Goal
What a caller can do after this task that they cannot do now. Name any gate paths in `touches`.
Reading list: only what this task needs (CHARTER sections, ADRs, files).

## Interface
Command id, params, result (JSON Schema or typed example), CLI/control call, public Rust items.

## Pinned
Answer each, or "n/a" with a reason (checked at `specified`, METHODOLOGY §3):
- (a) error variants, payload fields, precedence: … · (b) layout of every file read or written: …
- (c) order of checks: … · (d) path base, platform, file-type assumptions: …
- (e) threat model (gate/security): … · (f) tests pinning a snapshot this changes (in `touches`): …
- (g) worked examples, each executed against a naive model (command and output):
- (h) per scorecard item, the CLI or control-protocol call the evaluator will use:

## Acceptance criteria
Observable behaviour through the interface, with its oracle; ≥ 1 per scorecard item via CLI/control.
- AC-1: Given …, when <command/CLI call>, then <observable result>. Oracle: <naive model / round-trip / corpus / exact value>.

## Out of scope and implementer notes
What a reasonable implementer might do but must not, and which task covers it. Unit-test
expectations, determinism, performance budget if any.

## Questions and rulings
Builders log `QUESTION:` lines; the director answers all at once here: Q-n (role, R-NNN) → answer
(D-NNN, commit). A ruling that changes behaviour edits the sections above in the same commit.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

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
