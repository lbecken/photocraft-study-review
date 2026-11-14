---
id: T-058
title: "Show every open document as a tab, switch the current document by clicking a tab, and close a tab through doc.close"
tier: medium            # ADR-0002 default: UI behaviour over existing commands; no dependency, gate path or persistence code of its own
state: draft
owner_role: planner
scorecard_items: [ui.tabs]
depends_on: [T-056]
touches:                # provisional, fixed at specification
  - crates/kiln-ui/src/panels/tabs.rs   # the panel (created empty by T-056)
  - apps/kiln/tests/acceptance_t058.rs
  - docs/protocol/ui-tabs.md      # new
budget_tokens: 800000   # medium (v1.3)
iteration: 2
found_by: planner       # ADR-0007 (run R-225), D-145 step 3
origin_task: null
---

# T-058 Show every open document as a tab, switch the current document by clicking a tab, and close a tab through doc.close

This is a draft filed with the plan of the second stage (ADR-0007, run R-225). Interface, Pinned and
acceptance criteria are written by a planner after its `depends_on` merge, so no separate spec check
is needed (METHODOLOGY v1.3 §5.7). Observation and counting follow ADR-0008: through `kiln script`,
(O1)–(O5).

## Goal
A node labelled `Tabs` has one tab per open document (`doc.list` order), its value the path and `modified` when dirty, the current document's tab `selected`; clicking a tab makes that document current; each tab's close button closes it with `doc.close`; closing a dirty document is refused (`unsaved_changes`) and asks the user, and only a confirmed close sends `discard: true`.

## Commands it shows
`doc.list`, `doc.close` with its `unsaved_changes` refusal and `discard` (`ws.documents`, `ws.dirty-tracking`, verified), `doc.info`. It adds no registry command; if the specification finds one is needed, the new
command declares its refusal kinds with refusal examples (ADR-0006) and the task is re-planned.

## How the evaluator observes it (ADR-0008)
Snapshot: tabs equal `doc.list` (paths, dirty marks); after a tab click, `Editor` shows that document's `doc.text`; after a close, `doc.list` no longer lists it and the log holds `doc.close`; a refused close leaves the document open and `Message` begins with `unsaved_changes`.

## Open questions for the specification
- How the user confirms discarding a dirty document (a confirm button node), and what the current document becomes after a close (T-056's rule: the highest open id).
- Tests find nodes by label and role and check them against the commands in the same session;
  none pins a whole snapshot, a message text or the shell's layout (golden rule 9).

## Log
### director
<!-- end director log -->
### planner
- 2026-11-14 · R-225 · (new) · did: filed as a draft for ADR-0007. · gate attempts: 0 · missing context: T-056's merged shell · next: director.
<!-- end planner log -->
