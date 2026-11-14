---
id: T-059
title: "Show the caret's line and column, the document's encoding and its line-ending style in a status bar"
tier: medium            # ADR-0002 default: UI behaviour over existing commands; no dependency, gate path or persistence code of its own
state: draft
owner_role: planner
scorecard_items: [ui.status-bar]
depends_on: [T-056]
touches:                # provisional, fixed at specification
  - crates/kiln-ui/src/panels/status.rs   # the panel (created empty by T-056)
  - apps/kiln/tests/acceptance_t059.rs
  - docs/protocol/ui-status.md      # new
budget_tokens: 800000   # medium (v1.3)
iteration: 2
found_by: planner       # ADR-0007 (run R-225), D-145 step 3
origin_task: null
---

# T-059 Show the caret's line and column, the document's encoding and its line-ending style in a status bar

This is a draft filed with the plan of the second stage (ADR-0007, run R-225). Interface, Pinned and
acceptance criteria are written by a planner after its `depends_on` merge, so no separate spec check
is needed (METHODOLOGY v1.3 §5.7). Observation and counting follow ADR-0008: through `kiln script`,
(O1)–(O5).

## Goal
A node labelled `Status` shows the current document's caret line and column, its encoding and its line-ending style (and that the endings are mixed when `doc.info` says so), updated after every request that can change them.

## Commands it shows
`cursor.get` (`edit.selection`, verified), `doc.info` (encoding, `line_ending`, `mixed_line_endings`; `io.encoding`, `io.crlf`, `io.mixed-eol`, verified). It adds no registry command; if the specification finds one is needed, the new
command declares its refusal kinds with refusal examples (ADR-0006) and the task is re-planned.

## How the evaluator observes it (ADR-0008)
Snapshot after each caret movement or edit: the line and column shown equal `cursor.get` (with the stated base, 0- or 1-based), the encoding and line ending equal `doc.info`; for a UTF-16 file with BOM, a CRLF file and a mixed file the values differ as `doc.info` does.

## Open questions for the specification
- Display base of line and column (1-based for people, `cursor.get` is 0-based): state it in `docs/protocol/ui-status.md` so the oracle can convert.
- Column unit (chars, as `cursor.get`), or display columns (tabs)? Chars recommended (one oracle).
- Tests find nodes by label and role and check them against the commands in the same session;
  none pins a whole snapshot, a message text or the shell's layout (golden rule 9).

## Log
### director
<!-- end director log -->
### planner
- 2026-11-14 · R-225 · (new) · did: filed as a draft for ADR-0007. · gate attempts: 0 · missing context: T-056's merged shell · next: director.
<!-- end planner log -->
