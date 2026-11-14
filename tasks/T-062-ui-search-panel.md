---
id: T-062
title: "Run find-in-files from a search panel, list the results grouped by file, and open a result at its position on click"
tier: medium            # ADR-0002 default: UI behaviour over existing commands; no dependency, gate path or persistence code of its own
state: draft
owner_role: planner
scorecard_items: [ui.search-panel]
depends_on: [T-056, T-022]
touches:                # provisional, fixed at specification
  - crates/kiln-ui/src/panels/search_panel.rs   # the panel (new; plus one line in the shell)
  - apps/kiln/tests/acceptance_t062.rs
  - docs/protocol/ui-search-panel.md      # new
budget_tokens: 800000   # medium (v1.3)
iteration: 2
found_by: planner       # ADR-0007 (run R-225), D-145 step 4
origin_task: null
---

# T-062 Run find-in-files from a search panel, list the results grouped by file, and open a result at its position on click

This is a draft filed with the plan of the second stage (ADR-0007, run R-225). Interface, Pinned and
acceptance criteria are written by a planner after its `depends_on` merge, so no separate spec check
is needed (METHODOLOGY v1.3 §5.7). Observation and counting follow ADR-0008: through `kiln script`,
(O1)–(O5).

## Goal
A panel labelled `Search` takes a query and options, dispatches T-022's find-in-files command, lists the results grouped by file with line and column, and opens a result's file with the caret at its position on click.

## Commands it shows
none yet: find in files is T-022's command (`ws.find-in-files`, `ws.replace-in-files`, planned); `doc.open`, `nav.goto-line` (verified). It adds no registry command; if the specification finds one is needed, the new
command declares its refusal kinds with refusal examples (ADR-0006) and the task is re-planned.

## How the evaluator observes it (ADR-0008)
Snapshot: the listed results equal the command's result in a second session (files, lines, columns, order); after a click, `Title`, `Editor` and `cursor.get` show the file and position; replace across files only if T-022's preview command is shown here (decide at specification).

## Open questions for the specification
- Whether replace-in-files (`ws.replace-in-files`) gets its UI here or in a separate task (the item `ui.search-panel` names find only).
- The source file name: `panels/search_panel.rs` is not among the files T-056 creates; this task adds it and one line to the shell, so it cannot be in flight with another task that edits the shell.
- Tests find nodes by label and role and check them against the commands in the same session;
  none pins a whole snapshot, a message text or the shell's layout (golden rule 9).

## Log
### director
<!-- end director log -->
### planner
- 2026-11-14 · R-225 · (new) · did: filed as a draft for ADR-0007. · gate attempts: 0 · missing context: T-056's merged shell · next: director.
<!-- end planner log -->
