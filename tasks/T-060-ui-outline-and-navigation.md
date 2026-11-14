---
id: T-060
title: "Show the current document's outline in a panel, select an item's name on click, and offer go-to-line and a workspace symbol search"
tier: medium            # ADR-0002 default: UI behaviour over existing commands; no dependency, gate path or persistence code of its own
state: draft
owner_role: planner
scorecard_items: []   # no scorecard item exists for this UI; see Q-3 of ADR-0007
depends_on: [T-056]
touches:                # provisional, fixed at specification
  - crates/kiln-ui/src/panels/outline.rs   # the panel (created empty by T-056)
  - apps/kiln/tests/acceptance_t060.rs
  - docs/protocol/ui-outline.md      # new
budget_tokens: 800000   # medium (v1.3)
iteration: 2
found_by: planner       # ADR-0007 (run R-225), D-145 step 3
origin_task: null
---

# T-060 Show the current document's outline in a panel, select an item's name on click, and offer go-to-line and a workspace symbol search

This is a draft filed with the plan of the second stage (ADR-0007, run R-225). Interface, Pinned and
acceptance criteria are written by a planner after its `depends_on` merge, so no separate spec check
is needed (METHODOLOGY v1.3 §5.7). Observation and counting follow ADR-0008: through `kiln script`,
(O1)–(O5).

## Goal
A panel labelled `Outline` lists `nav.outline`'s items for the current document; clicking one dispatches `nav.goto-symbol` and the editor shows its name selected and in view; a go-to-line field moves the caret with `nav.goto-line`; a symbol search field lists `nav.workspace-symbols` results and opens the chosen one.

## Commands it shows
`nav.outline` (`syntax.outline`), `nav.goto-symbol`, `nav.goto-line`, `nav.workspace-symbols` (`nav.goto-symbol`, `nav.goto-line`, `nav.workspace-symbol`, all verified). It adds no registry command; if the specification finds one is needed, the new
command declares its refusal kinds with refusal examples (ADR-0006) and the task is re-planned.

## How the evaluator observes it (ADR-0008)
Snapshot: outline entries equal `nav.outline` (names, kinds, order); after a click, `Editor` `selection` equals `cursor.get` and covers the item's name; go-to-line clamps as `nav.goto-line` does; workspace results equal `nav.workspace-symbols`.

## Open questions for the specification
- No scorecard item covers this UI (the eight `ui.*` items do not). ADR-0007 Q-3: add an item such as `ui.outline-panel` (by a publish task) so the evaluator's observation counts, or build it without an item, or leave it out of the stage.
- Split go-to-line and the workspace symbol search into their own task if the spec grows past one panel.
- Tests find nodes by label and role and check them against the commands in the same session;
  none pins a whole snapshot, a message text or the shell's layout (golden rule 9).

## Log
### director
<!-- end director log -->
### planner
- 2026-11-14 · R-225 · (new) · did: filed as a draft for ADR-0007. · gate attempts: 0 · missing context: T-056's merged shell · next: director.
<!-- end planner log -->
