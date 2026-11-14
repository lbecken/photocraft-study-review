---
id: T-050
title: "State every difference from git in .gitignore matching that a pattern can reach, and the depth rule for a deep path"
tier: medium
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-019]
touches:                # provisional, fixed at specification
  - docs/protocol/project.md
  - crates/kiln-workspace/src/ (the ignore matcher, only if the spec chooses to change behaviour)
budget_tokens: 700000
iteration: 2
found_by: evaluator      # evaluations/iter-2.md NEW-1 and Q1 (run R-221)
origin_task: T-019
---

# T-050 State every difference from git in .gitignore matching that a pattern can reach, and the depth rule for a deep path

This is a draft filed from the evaluation of the first stage of iteration 2 (`evaluations/iter-2.md`,
runs R-221 and R-222; D-141). Interface, Pinned and acceptance criteria are written by a planner, and the
spec is checked before dispatch (METHODOLOGY v1.2).

## Goal
`docs/protocol/project.md` lists the differences between `project.tree`'s `.gitignore` handling and git. The evaluator found pattern forms, all outside the forms the document guarantees, where the binary and git differ and the list is silent. After this task either the list names them or the binary agrees with git on them, and the depth rule says what happens when the `path` given to `project.tree` is itself 128 or more segments deep.

## Evidence in the record
- `evaluations/iter-2.md`, NEW-1: a bracket class matches `/` (`*[!a]*a` ignores `x/a`, which git lists; also `[/]`); a lone `[` ignores a file named `[`, which git lists; unbalanced braces (`*}`, `{*`) and empty alternatives (`b*{}`, `{x,}`) differ from git. 450 random trees within the guaranteed forms agreed with `git ls-files` (seeds 1 to 3).
- `evaluations/iter-2.md`, Q1: a `path` of 128 or more segments is itself descended, so entries of 132 segments are listed.
- Known and filed apart: the cost of `.gitignore` files, T-047.

## Open questions for the specification
- Document or change? A change of matching is a behaviour change of a verified item (`ws.open-project`) and raises the tier question.
- Q1: is the depth bound counted from the root or from `path`? State the one the binary has, or change it.

## Log
### director
- 2026-11-14 filed as a draft (D-141).
