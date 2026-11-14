---
id: T-051
title: "Say in syntax.md and for the evaluator which corpus set the syntax.parse claim covers, and where the corpus can be read"
tier: low
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-020]
touches:                # provisional, fixed at specification
  - docs/protocol/syntax.md
  - methodology/templates/evaluation.md (director; ledger path)
budget_tokens: 150000
iteration: 2
found_by: evaluator      # evaluations/iter-2.md NEW-2 (runs R-221, R-222)
origin_task: T-020
---

# T-051 Say in syntax.md and for the evaluator which corpus set the syntax.parse claim covers, and where the corpus can be read

This is a draft filed from the evaluation of the first stage of iteration 2 (`evaluations/iter-2.md`,
runs R-221 and R-222; D-141). Interface, Pinned and acceptance criteria are written by a planner, and the
spec is checked before dispatch (METHODOLOGY v1.2).

## Goal
The scorecard item `syntax.parse` says every file of the pinned compiling corpus parses without error nodes. 35 corpus files compile; 2 of them (set `gap`, the edition-2024 `safe` qualifier) have one error node each, as `MANIFEST.toml` says, so the claim holds for set `valid` (33 files) only, and only the manifest says so. After this task `docs/protocol/syntax.md` states the three sets and what is claimed for each, and the evaluator's template says that the corpus under `crates/kiln-syntax/tests/corpus/` is fixture data an evaluator may read.

## Evidence in the record
- `evaluations/iter-2.md`, `syntax.parse` block and NEW-2: 45 files match the manifest (bytes, newline count, FNV-1a 64); `valid` 33 files without error nodes and rustc exit 0; `gap` 2 files with one error node each on the word `safe` and rustc exit 0; `invalid` 10 files with 1 to 2 error or missing nodes and rustc exit 1.
- Ruling of D-141: the corpus is fixture data, readable by the evaluator.

## Open questions for the specification
- `scorecard/items/` is add-only for the queue: can the item's title be narrowed, or is the statement made in `syntax.md` only?

## Log
### director
- 2026-11-14 filed as a draft (D-141).
