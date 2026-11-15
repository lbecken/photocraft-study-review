---
id: T-080
title: "List one level of a directory with project.tree, so that a truncated tree does not hide later entries"
tier: medium            # provisional: a new parameter of a verified command (ws.open-project)
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-057]
touches:                # provisional, fixed at specification
  - crates/kiln-commands/src/project.rs
  - docs/protocol/project.md
  - docs/commands.json
budget_tokens: 800000   # medium (v1.4)
iteration: 2
found_by: planner       # run R-262 (the spec of T-057), measured
origin_task: T-019
---

# T-080 List one level of a directory with project.tree

This is a draft filed from the spec of T-057 (run R-262; D-181). Interface, Pinned and acceptance criteria are
written by a planner, and the spec is checked before dispatch.

## Goal
`project.tree` lists a whole subtree up to `limit` entries. With a directory `a/` of 50,000 files and a file
`z.txt` beside it, the root listing returns only entries of `a` and `truncated: true`; `z.txt` is cut off, and the
project panel (T-057) can show only what the read listed, with a marker. After this task a caller can ask for the
entries directly inside one directory, so that a panel can show every level it expands.

## Evidence in the record
- `tasks/T-057-ui-project-panel.md`, "Spec check R-262": `project.tree` on 50,200 entries took 0.50 s in a debug
  build and returned 50,000 entries with `truncated: true`; with a 50,000-file directory `a/` plus `z.txt` the root
  listing shows only `a`.

## Open questions for the specification
- A new parameter (a depth) or a new command? Either changes the published schema of a verified item.

## Log
### director
- 2026-11-15 filed as a draft (D-181).
