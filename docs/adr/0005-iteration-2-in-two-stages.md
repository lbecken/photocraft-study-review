---
id: ADR-0005             # file docs/adr/0005-iteration-2-in-two-stages.md
title: "Iteration 2 runs in two stages: first the 60 % line, then the UI"
status: accepted
date: 2026-11-13
task: T-042              # the CHARTER §5 text follows through this low-tier task
charter_section: "§5 iterations (iteration 2)"
approval: D-091          # retro-1 change 16
supersedes: null
superseded_by: null
---

# ADR-0005 Iteration 2 in two stages

## Context
- CHARTER §5 sets the exit criterion of iteration 1 at 60 % of scorecard items verified, and
  gives iteration 2 the purpose "UI, end-to-end flows, budgets, hardening" with the exit
  criterion "every item verified".
- The criterion was missed. Iteration 1 closed with 22 of 64 items verified (D-089). After the
  iteration-1 evaluation (D-090) and its publication (T-040, D-091) the scorecard on main reads
  64 items, 28 verified, 0 implemented, 36 planned: 43.8 %. 60 % of 64 is 38.4, so 39 items are
  needed; the shortfall is 11 items (retro-1, "Charter exit criterion").
- The twelve tasks merged in iteration 1 claimed 6 scorecard items; the four tasks that carried
  items took 30.5 % of the 10,985,201 tokens booked (retro-1; D-091).
- Eighteen tasks are drafted. Retro-1 estimates that all of them would cost 14,210,274 tokens at
  iteration-1 medians and about 16.9–19.4 M with iteration 1's overhead factor, 1.5–1.8 times all
  of iteration 1. None of the 8 `ui.*` items is claimed by any draft (retro-1, "Recommendation on
  the scope of iteration 2").

## Options
| Option | For | Against |
|---|---|---|
| A. Dispatch all eighteen drafts, as planned | Charter order unchanged | Retro-1's estimate of 16.9–19.4 M tokens; the 60 % line is reached only somewhere inside that; no UI item is claimed |
| B. Two stages: first the smallest set that reaches 60 %, then the UI tasks, drafted after the first stage | An iteration sized to what was measured; the 60 % line is checked before more work is dispatched | The drafts outside the first stage wait; the UI work starts later |
| Keep the current plan and criterion | No document changes | The criterion that was missed is not met again before the next one is attempted |

## Decision
Option B (D-091, retro-1 change 16).

**Stage 1** reaches the charter's 60 % line. It dispatches:
- the item tasks T-019 (1 item: `ws.open-project`), T-020 (4: `syntax.parse`,
  `syntax.incremental`, `syntax.highlight`, `syntax.brackets`), T-021 (4: `search.literal`,
  `search.regex`, `search.whole-word`, `search.replace`) and T-023 (4: `syntax.outline`,
  `nav.goto-line`, `nav.goto-symbol`, `nav.workspace-symbol`): 13 items, counted from the
  `scorecard_items` of each task's front matter, all `planned` on main today;
- T-036, which closes the risk accepted at T-008's merge (D-081, D-083), and the evaluator's
  findings T-037, T-038 and T-039 (D-090);
- the tooling T-033, cut down to the tool parts of v1.2 changes 1, 8 and 9 (D-091), and T-041
  (no CI run for a push that changes only ledger paths).

Arithmetic, from the current scorecard (64 items, 28 verified): 28 + 13 = 41 of 64 = 64.1 %,
two items above the 39 needed. The line is met only if the evaluator confirms all 13 and no
verified item is withdrawn. T-036 and T-037 strengthen the existing item `ws.path-confinement`
and add none.

**Stage 2** is the UI tasks, drafted after stage 1. T-007, T-009, T-022, T-024 to T-031 and T-034
stay drafted and are not dispatched in stage 1.

The check: the director dispatches no task outside the stage-1 list until stage 1 ends (director,
at dispatch); the 60 % line is read from `cargo xtask scorecard` after stage 1's evaluation.

## Consequences
- Given up for stage 1: find and replace in files (T-022), tool runners and diagnostics (T-024),
  newline, indent and comment (T-025), rustfmt (T-026), external change before save (T-027), gate
  runners, fuzz and budgets (T-028), the command fuzz target (T-029), structured rejection reasons
  (T-030), the command palette and keymap (T-031), merge-queue follow-ups (T-009), command-layer
  integrity (T-007), lint escapes and publish follow-ups (T-034). Their items stay `planned`.
- Cost, as retro-1 estimates it at iteration-1 medians: T-019 plus the three medium tasks
  1,004,501 + 3 × 520,658 = 2.57 M tokens (2.95 M at means); with T-036, about 1.0–1.15 M more, a
  total of 3.6–4.1 M, about a third of iteration 1. That estimate does not include T-037, T-038,
  T-039, T-033 or T-041, which were filed or scoped after it.
- T-023 depends on T-019 (front matter), so it is dispatched after T-019 merges. The medium-tier
  experiment continues: T-020 is even, T-021 and T-023 are odd (retro-1).
- CHARTER §5 text to change: the iteration-2 row states the two stages and that stage 1 must reach
  the 60 % line first. `docs/CHARTER.md` is not a ledger path, so the text follows through a
  low-tier task through the queue (D-091; retro-1 change 15 was deferred).
- Check: director at dispatch (rule only, because no tool reads a stage list).

## Verification
After stage 1's evaluation, `cargo xtask scorecard` prints at least 39 of 64 items verified.

## Rollback
A later decision row can return any stage-2 draft to dispatch, or end stage 1 early; the cost
is that the 60 % line is then not checked before that work runs.
