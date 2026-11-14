---
id: T-063
title: "List diagnostics in a panel and move the caret to a diagnostic when it is selected"
tier: medium            # ADR-0002 default: UI behaviour over existing commands; no dependency, gate path or persistence code of its own
state: draft
owner_role: planner
scorecard_items: [ui.diagnostics-panel]
depends_on: [T-056, T-024]
touches:                # provisional, fixed at specification
  - crates/kiln-ui/src/panels/diagnostics.rs   # the panel (new; plus one line in the shell)
  - apps/kiln/tests/acceptance_t063.rs
  - docs/protocol/ui-diagnostics.md      # new
budget_tokens: 800000   # medium (v1.3)
iteration: 2
found_by: planner       # ADR-0007 (run R-225), D-145 step 4
origin_task: null
---

# T-063 List diagnostics in a panel and move the caret to a diagnostic when it is selected

This is a draft filed with the plan of the second stage (ADR-0007, run R-225). Interface, Pinned and
acceptance criteria are written by a planner after its `depends_on` merge, so no separate spec check
is needed (METHODOLOGY v1.3 §5.7). Observation and counting follow ADR-0008: through `kiln script`,
(O1)–(O5).

## Goal
A panel labelled `Diagnostics` lists the diagnostics T-024's commands report (severity, file, position, message), runs the check on request (only where the session allows tools: T-024's opt-in), and on selecting one opens its file and puts the caret at its position, following later edits as `diag.positions` does.

## Commands it shows
none yet: diagnostics are T-024's commands (`diag.cargo-check`, `diag.positions`, `diag.tool-fake`, planned); `doc.open`, `cursor.set` (verified). It adds no registry command; if the specification finds one is needed, the new
command declares its refusal kinds with refusal examples (ADR-0006) and the task is re-planned.

## How the evaluator observes it (ADR-0008)
Snapshot: the listed diagnostics equal the command's result (with the recorded-output fake, deterministic); after a selection, `cursor.get` equals the diagnostic's mapped position; after an edit above it, the selection still lands where `diag.positions` maps it.

## Open questions for the specification
- How the UI session enables tools (T-024 makes running cargo an opt-in flag on the session's trust boundary): a `kiln` option, and whether `kiln script` gets the fake for tests.
- As for T-062: the panel file is not among T-056's, so this task edits the shell once.
- Tests find nodes by label and role and check them against the commands in the same session;
  none pins a whole snapshot, a message text or the shell's layout (golden rule 9).

## Log
### director
<!-- end director log -->
### planner
- 2026-11-14 · R-225 · (new) · did: filed as a draft for ADR-0007. · gate attempts: 0 · missing context: T-056's merged shell · next: director.
<!-- end planner log -->
