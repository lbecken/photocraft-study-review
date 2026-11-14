---
id: T-061
title: "Find and replace in the current document from a find bar: list and step through matches, replace one or all"
tier: medium            # ADR-0002 default: UI behaviour over existing commands; no dependency, gate path or persistence code of its own
state: draft
owner_role: planner
scorecard_items: []   # no scorecard item exists for this UI; see Q-3 of ADR-0007
depends_on: [T-056]
touches:                # provisional, fixed at specification
  - crates/kiln-ui/src/panels/find.rs   # the panel (created empty by T-056)
  - apps/kiln/tests/acceptance_t061.rs
  - docs/protocol/ui-find.md      # new
budget_tokens: 800000   # medium (v1.3)
iteration: 2
found_by: planner       # ADR-0007 (run R-225), D-145 step 3
origin_task: null
---

# T-061 Find and replace in the current document from a find bar: list and step through matches, replace one or all

This is a draft filed with the plan of the second stage (ADR-0007, run R-225). Interface, Pinned and
acceptance criteria are written by a planner after its `depends_on` merge, so no separate spec check
is needed (METHODOLOGY v1.3 §5.7). Observation and counting follow ADR-0008: through `kiln script`,
(O1)–(O5).

## Goal
A find bar (query field, case, whole-word and regex toggles, next/previous, a replacement field, replace and replace-all buttons) dispatches `search.find` and `search.replace`; the editor selects the current match and keeps it in view; an invalid pattern shows `invalid_pattern` in `Message`.

## Commands it shows
`search.find`, `search.replace` (`search.literal`, `search.regex`, `search.whole-word`, `search.replace`, all verified). It adds no registry command; if the specification finds one is needed, the new
command declares its refusal kinds with refusal examples (ADR-0006) and the task is re-planned.

## How the evaluator observes it (ADR-0008)
Snapshot: the match count and the selected match equal `search.find`'s ranges; after replace-all, `doc.text` equals `search.replace`'s result in a second session and one `ctrl+Z` undoes it; toggles change the result as the commands' params do (inert-toggle check, ADR-0008 (O5)).

## Open questions for the specification
- No scorecard item covers this UI. ADR-0007 Q-3 as for T-060.
- `search.find`'s `limit` and `truncated`: how the bar shows a capped result.
- Tests find nodes by label and role and check them against the commands in the same session;
  none pins a whole snapshot, a message text or the shell's layout (golden rule 9).

## Log
### director
<!-- end director log -->
### planner
- 2026-11-14 · R-225 · (new) · did: filed as a draft for ADR-0007. · gate attempts: 0 · missing context: T-056's merged shell · next: director.
<!-- end planner log -->
