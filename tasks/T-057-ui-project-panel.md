---
id: T-057
title: "Show the project tree in a panel and open a file in the editor when its entry is clicked"
tier: medium            # ADR-0002 default: UI behaviour over existing commands; no dependency, gate path or persistence code of its own
state: draft
owner_role: planner
scorecard_items: [ui.project-panel]
depends_on: [T-056]
touches:                # provisional, fixed at specification
  - crates/kiln-ui/src/panels/project.rs   # the panel (created empty by T-056)
  - apps/kiln/tests/acceptance_t057.rs
  - docs/protocol/ui-project.md      # new
budget_tokens: 800000   # medium (v1.3)
iteration: 2
found_by: planner       # ADR-0007 (run R-225), D-145 step 3
origin_task: null
---

# T-057 Show the project tree in a panel and open a file in the editor when its entry is clicked

This is a draft filed with the plan of the second stage (ADR-0007, run R-225). Interface, Pinned and
acceptance criteria are written by a planner after its `depends_on` merge, so no separate spec check
is needed (METHODOLOGY v1.3 §5.7). Observation and counting follow ADR-0008: through `kiln script`,
(O1)–(O5).

## Goal
A panel labelled `Project` lists the root as `project.tree` lists it (entries in its preorder, directories expandable through `project.tree` with `path`); clicking a file entry opens it (`doc.open`) and makes it the current document; refusals (for example a file that vanished) show their kind in `Message`.

## Commands it shows
`project.tree` (`ws.open-project`, verified), `doc.open` (`control.session`, verified). It adds no registry command; if the specification finds one is needed, the new
command declares its refusal kinds with refusal examples (ADR-0006) and the task is re-planned.

## How the evaluator observes it (ADR-0008)
Snapshot: the panel's entries equal `project.tree`'s entries (paths and kinds) for the expanded directories; after a click on a file entry, `Title` and `Editor` show that file, `doc.list` lists it and the log holds the `doc.open`. Directory entries: a click expands or collapses and dispatches only read-only commands.

## Open questions for the specification
- How deep the panel lists by default (all of `project.tree`, or one level with expansion), and how it shows `project.tree`'s limits (`truncated`, depth bound) to the user.
- Whether a click on an already open file only makes it current (`doc.open` returns `already_open`).
- Tests find nodes by label and role and check them against the commands in the same session;
  none pins a whole snapshot, a message text or the shell's layout (golden rule 9).

## Log
### director
<!-- end director log -->
### planner
- 2026-11-14 · R-225 · (new) · did: filed as a draft for ADR-0007. · gate attempts: 0 · missing context: T-056's merged shell · next: director.
<!-- end planner log -->
