---
id: T-064
title: "Make docs/CHARTER.md, docs/development.md and AGENTS.md say what methodology v1.3 says"
tier: low
state: draft
owner_role: planner
scorecard_items: []     # low tier carries no scorecard item (METHODOLOGY §4)
depends_on: []
touches:                # provisional, fixed at specification
  - docs/CHARTER.md
  - docs/development.md
  - AGENTS.md
budget_tokens: 150000   # low (v1.3)
iteration: 2
found_by: planner       # the editor of v1.3, run R-224 (D-146)
origin_task: null
---

# T-064 Make the texts outside the ledger say what methodology v1.3 says

This is a draft filed so that the list of D-146 is not lost (the same kind of task as T-042 was for v1.2).

## Goal
Three files the director may not edit directly still state rules v1.3 changed. After this task they agree with
`methodology/METHODOLOGY.md` v1.3.

## Evidence in the record (run R-224, D-146)
- `docs/CHARTER.md` lines 95 to 96: the state list ends "→ queued → merged → evaluated"; v1.3 drops `evaluated`
  (the charter wins where the two disagree, so until this task the state exists and is not used).
- `docs/development.md` lines 407 to 409 (the transition list with `merged → evaluated`), 354 to 355 and 416 to 421
  (`join-defects`: no cause; refuses when no acceptance test failed at the first join), 400 to 402 (start equal to
  end is accepted; the tool change is T-043's), 71 (the review fields lack `doc_sentences`).
- `AGENTS.md` line 89: the medium definition of done lacks the implementer-first gate run on a local merge.

## Open questions for the specification
- Does the charter edit need an ADR (METHODOLOGY on charter changes), or is it a wording change within D-144?
- `xtask` still accepts `merged → evaluated`: leave it (harmless) or remove it with T-043?

## Log
### director
- 2026-11-14 filed as a draft (D-146, D-147).
