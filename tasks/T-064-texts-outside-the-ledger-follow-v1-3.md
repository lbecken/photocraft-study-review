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

## Note from the edit to v1.4 (director, D-176)
Further sentences outside the ledger that disagree since v1.4 (run R-257): `docs/CHARTER.md` lines 80 (the gate
"identical locally and in GitHub Actions" names no platforms), 82 (`no-skip` without the platform-test registry),
97 (tiers without the floor from claimed items); `docs/development.md` lines 75 (`no-skip` and `cfg`), 537 to 540
(CI is one Linux job), 71 (the `hygiene` reviews check: mutant keys, confirmation records counted as rounds), 265
to 266 (front-matter edits on a branch); `AGENTS.md` lines 44, 66, 99 (one role per path; rulings read from main;
the platform test rule). `crates/kiln-workspace/src/disk.rs` line 1 and `docs/protocol/paths.md` line 114 ("Unix
only") are product documents and go with T-072 to T-074.

## Log
### director
- 2026-11-14 filed as a draft (D-146, D-147).
