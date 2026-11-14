---
id: ADR-0007             # file docs/adr/0007-second-stage-order.md
title: "The second stage of iteration 2: the observation interface and a window first, then one UI module per verified feature, then the feature drafts with their UI, as the budget allows"
status: accepted
date: 2026-11-14
task: null               # a plan, carried by the tasks it orders
charter_section: "§5 iterations (iteration 2, stage 2), as staged by ADR-0005"
approval: D-147
supersedes: null
superseded_by: null
---

# ADR-0007 Order of the second stage

Written by the planner of the second stage (run R-225, booked to `iter-2`) at main
`ec08d22c6c12639c31022a221820c23a1aeeba32`, on the director's direction D-145. Labels as in
the retrospectives: VERIFIED (a command or a cited record), INFERRED (reasoning given),
PROPOSED (this decision).

## Context

- **Where the product stands.** `docs/scorecard.md`: `64 items: 41 verified, 0 implemented, 0
  failing, 23 planned.` (D-142). The 23 planned items (VERIFIED, `scorecard/items/*.toml` with
  `tests = []`): `ui.window`, `ui.editor-view`, `ui.dispatch-only`, `ui.project-panel`,
  `ui.tabs`, `ui.status-bar`, `ui.search-panel`, `ui.diagnostics-panel`; `ws.find-in-files`,
  `ws.replace-in-files` (T-022); `diag.tool-fake`, `diag.cargo-check`, `diag.positions` (T-024);
  `edit.indent`, `edit.comment-toggle` (T-025); `fmt.rustfmt`, `fmt.idempotent`, `fmt.error`
  (T-026); `io.external-change` (T-027); `text.rope` (T-028); `cmd.palette`, `keymap.default`,
  `keymap.custom` (T-031). No draft claimed a `ui.*` item before this plan. T-025 claimed
  `edit.newline`, which is not an item; corrected in its file by this run (D-145).
- **The direction (D-145).** (1) T-052, T-051; T-043 with the checks of D-144 change 12; T-033
  and T-041 placed by this ADR, T-041 last, the owner told first. (2) A high-tier UI foundation
  with its own ADR on UI observation (ADR-0008). (3) Thin UI modules over verified commands
  (project tree, search, outline and navigation, status), one task each, each counted only when
  the evaluator observes the behaviour. (4) The feature drafts that need new commands (T-022,
  T-024, T-025, T-026, T-027, T-028, T-031), each followed by its UI module, as the budget allows.
  Reason: the owner's aim is a working desktop product, 41 verified items have no visible
  surface, and the weekly usage limit (D-143) makes it likely that not all of (4) fits.
- **What the record says about cost** (retro-2s1 "Metrics", VERIFIED there by `S tokens`):
  building and review per merged task, median high **810,888**, medium **686,472**, low 84,466
  and 124,444 (n=2; their mean **104,455** is used here); spec checks **2,286,780 for 7 tasks**
  (**326,683** each); the two evaluation runs of stage 1 **404,836**. Stage 1 cost 9,201,078 in
  all. Ceilings v1.3 (D-144 change 2): high 1,100,000, medium 800,000, low 150,000. The stage
  gives no data on UI work; estimates for UI tasks below are the medians of non-UI tasks
  (retro-2s1, "Recommendation on stage 2": "estimates from it are not estimates for UI work").
- **What the probe found** (ADR-0008, VERIFIED): egui runs headless with scripted input and
  text read-back; the toolkit adds about 1 GB and about 54 s to a debug build of the UI crates,
  728 MB with less debug info; no real window can start here without two extra libraries.
- **What serialises work** (VERIFIED from `touches` in the task files): `docs/development.md` is
  in the `touches` of T-052, T-043, T-033, T-041, T-028 and the profile task T-054; the root
  `Cargo.toml` in T-054, T-055 and T-056; `crates/kiln-ui/src/` in T-055 and T-056; `xtask/src/main.rs`
  in T-033 and T-043. METHODOLOGY §5.2 forbids overlapping `touches` in flight. Disk limited
  dispatch in stage 1 more than the cap of three builders (retro-2s1 section 8).

## Options

| Option | For | Against |
|---|---|---|
| A. D-145's order, made concrete below | a usable application after the second UI task; each later step adds visible behaviour; what is cut at a budget stop is the tail | order and UI start are confounded (below); about 4.4 M tokens before the first UI item can be evaluated |
| B. The analyst's order (retro-2s1): small fixes and tooling, then the UI foundation, then the feature drafts in parallel, UI panels each after its feature | more items per token early (the feature drafts carry 15 items) | at a budget stop the product may still have no window; the owner's aim is a working desktop product (D-145) |
| C. One UI shell over every verified command first | one task | serialises every later UI change on one crate and cannot be counted per item (retro-2s1 assessment of D-140) |

## Decision (PROPOSED)

Option A. The stage dispatches in this order; tasks on one line may be in flight together
(no overlapping `touches`, at most three builders, disk as METHODOLOGY §2.1 checks it).

Estimates: *build* is the median of the tier for building and review runs (charged to the task);
*plan* is the planning still needed before dispatch (a specification of a draft or a spec check,
booked to `iter-2`), at the spec-check median 326,683 as a proxy (no median exists for writing a
spec); 0 where the spec is complete and needs no check. Cumulative includes the evaluation runs
at the cut lines.

| # | Task | Tier | Items it claims | Depends on (merged first) | May run with | Build | Plan | Cumulative |
|---|---|---|---|---|---|---|---|---|
| 1 | T-051 syntax.parse claim and corpus access | low | — | T-020 ✓ | T-052 | 104,455 | 326,683 | 431,138 |
| 2 | T-052 tests that need a reachable checkout | low | — | T-036 ✓, T-037 ✓ | T-051 | 104,455 | 326,683 | 862,276 |
| 3 | T-054 lighter debug build profile | high (gate path) | — | — (after T-052: shared `docs/development.md`) | T-051 | 810,888 | 326,683 | 1,999,847 |
| 4 | T-043 queue refusal records + D-144 change 12 checks | high (gate path) | — | — (after T-054: shared `docs/development.md`) | T-055, T-056 | 810,888 | 326,683 | 3,137,418 |
| 5 | T-055 UI script mode and shell (ADR-0008) | high (dependency) | — | — (after T-054: shared `Cargo.toml`) | T-043 | 810,888 | 0 (spec written, R-225) | 3,948,306 |
| 6 | T-056 window and editor view | high (dependency) | `ui.window`, `ui.editor-view`, `ui.dispatch-only` | T-055 | T-043 | 810,888 | 326,683 (spec check) | 5,085,877 |
| — | evaluation A (the three items; ADR-0008 (O1)–(O5)) | — | — | T-056 | — | 404,836 | — | **5,490,713** |
| 7 | T-057 project panel | medium (odd: implementer-first arm) | `ui.project-panel` | T-056 | T-058, T-059 | 686,472 | 326,683 | 6,503,868 |
| 8 | T-058 tabs | medium (even: independent) | `ui.tabs` | T-056 | T-057, T-059 | 686,472 | 326,683 | 7,517,023 |
| 9 | T-059 status bar | medium (odd) | `ui.status-bar` | T-056 | T-057, T-058 | 686,472 | 326,683 | 8,530,178 |
| — | evaluation B (the three items) | — | — | T-057–T-059 | — | 404,836 | — | **8,935,014** |
| 10 | T-060 outline and navigation panel | medium (even) | none unless Q-3 adds one | T-056 | T-061 | 686,472 | 326,683 | 9,948,169 |
| 11 | T-061 find bar | medium (odd) | none unless Q-3 adds one | T-056 | T-060 | 686,472 | 326,683 | 10,961,324 |
| 12 | T-028 gate runners, fuzz, budgets | high (gate path) | `text.rope` | T-013 ✓ | 13–15 | 810,888 | 326,683 | 12,098,895 |
| 13 | T-022 find and replace in files | medium | `ws.find-in-files`, `ws.replace-in-files` | T-019 ✓, T-021 ✓ | 12 | 686,472 | 326,683 | 13,112,050 |
| 14 | T-062 search panel | medium (even) | `ui.search-panel` | T-056, T-022 | 12, 15 | 686,472 | 326,683 | 14,125,205 |
| 15 | T-024 tool runners and diagnostics | high (spawns processes) | `diag.tool-fake`, `diag.cargo-check`, `diag.positions` | T-015 ✓, T-018 ✓, T-032 ✓ | 12–14 | 810,888 | 326,683 | 15,262,776 |
| 16 | T-063 diagnostics panel | medium (odd) | `ui.diagnostics-panel` | T-056, T-024 | 17 | 686,472 | 326,683 | 16,275,931 |
| 17 | T-025 newline, indent, comment | medium (odd) | `edit.indent`, `edit.comment-toggle` | T-018 ✓ | 16, 18 | 686,472 | 326,683 | 17,289,086 |
| 18 | T-026 rustfmt | medium (even) | `fmt.rustfmt`, `fmt.idempotent`, `fmt.error` | T-024 | 17 | 686,472 | 326,683 | 18,302,241 |
| 19 | T-027 external change before save | high (persistence path) | `io.external-change` | T-032 ✓, T-008 ✓ | 17, 18 | 810,888 | 326,683 | 19,439,812 |
| 20 | T-031 palette and keymap | medium (odd) | `cmd.palette`, `keymap.default`, `keymap.custom` | T-018 ✓, T-023 ✓, T-025, T-026 | — | 686,472 | 326,683 | 20,452,967 |
| 21 | T-033 join counts and scorecard | high (gate path) | — | T-005 ✓, T-013 ✓ (after T-043: shared `xtask/src/main.rs`) | — | 810,888 | 326,683 | 21,590,538 |
| 22 | T-041 no CI run for ledger-only pushes | high (gate path) | — | — (last; the owner is told first, D-098) | — | 810,888 | 0 (specified) | 22,401,426 |
| — | evaluation C (all items implemented since B) | — | — | — | — | 404,836 | — | **22,806,262** |

Notes on the order:
- **Step 1 (rows 1–4).** As D-145. T-054 is placed in step 1 because D-144 change 14 asks for it
  before the UI's dependencies arrive; it is small and makes three UI worktrees fit the disk
  (6.3 GB → 2.5 GB per test build, D-097; the UI crates 962 → 728 MB, ADR-0008 c2). T-043 runs
  alongside T-055 and T-056 (no shared path), so it costs tokens but not wall time on the UI
  path.
- **Step 2 (rows 5–6)** is the foundation, split in two (ADR-0008; the planner brief splits a
  high-tier bundle): the observation interface with a minimal shell, then the window and the
  editor view. The two specs are written (`tasks/T-055-*.md`, `tasks/T-056-*.md`).
- **Step 3 (rows 7–11).** One module per task, each in its own source file, which T-056 creates
  empty (T-056 Pinned (b)), so that three can be in flight. The three with scorecard items come
  first. D-145 names "search" among the modules over verified commands: the verified search
  commands are in-document (`search.find`, `search.replace`), which no `ui.*` item names, so
  that module is the find bar T-061; `ui.search-panel` names find-in-files, which needs T-022,
  and is row 14. "Outline and navigation" (T-060) likewise has no item. Both wait for Q-3.
- **Step 4 (rows 12–20)** follows D-145's list. T-028 comes first in it: it is the only route to
  an enforced budget (C6 refuted as a gate, retro-2s1 section 9) and touches nothing of the UI.
  Each feature is followed by its UI module where a `ui.*` item exists (T-062 after T-022, T-063
  after T-024). The other features get UI work without an item, filed when the feature merges
  and numbered then: Enter, Tab and comment keys bound to `edit.*` after T-025 (replacing T-056's
  Enter, Q-4 of T-056); a Format action after T-026; a save-conflict prompt after T-027; the
  palette and the keymap in the UI after T-031.
- **T-033 is placed after the stage's product work, before T-041.** It is not worth its cost
  before the UI foundation: what it automates has an interim that worked in stage 1 (0 joins
  failed at `scorecard` in 16, the director regenerating the page; retro-2s1 section 1 row 8),
  its cost is that of a high task (about 1.14 M with its re-specification, since D-144 change 8
  widened its scope), and it touches `xtask/src/main.rs` with T-043, so it cannot run beside
  T-043. Its value is in measurement (countable `acc_defects`, O-8; the spec-check role), which
  the director can keep by the interim rule. If the director wants step 4's nine joins counted by
  the tool, row 21 moves to just before row 12 (Q-5).
- **T-041 last**, as D-098 and D-145 say; the owner is told before it is dispatched.

### Cut line: what is usable if the budget ends after N tasks

| Cut | After | Estimated cumulative | Items verified if every claim is confirmed | What the owner has |
|---|---|---|---|---|
| 0 | row 4 | 3,137,418 | 41 | no UI; tooling and documents corrected |
| **1** | row 6 + evaluation A | **5,490,713** | 44 | **a desktop editor for one file at a time**: open from the command line or the path field, syntax colours, caret, selection, scrolling, typing, delete, undo, redo, save, refusals shown |
| **2** | row 9 + evaluation B | **8,935,014** | 47 | **a usable project editor**: the project tree opens files, several documents as tabs with switching and closing, a status bar with line, column, encoding and line endings |
| 3 | row 11 | 10,961,324 (+ evaluation) | 47 (49 if Q-3 adds two items) | + outline, go-to-symbol, go-to-line, workspace symbols, find and replace in the file |
| 4 | row 20 + evaluation C | 20,857,803 | 64 | every planned item claimed: find in files, diagnostics, formatting, external-change check, palette, keymap, an enforced latency budget |
| 5 | row 22 + evaluation C | 22,806,262 | 64 | + the two tooling tasks |

INFERRED: at stage 1's total cost (9.2 M) the stage reaches cut 2; the whole plan is about 2.5
times stage 1 and will not fit if the budget resembles stage 1's. Cut 1 is the smallest
"usable application"; cut 2 is the one this plan aims at first. An evaluation run at each cut
line means a budget stop leaves the items built so far verified, not only implemented (Q-6).

### Planned items no task of the stage covers

- With the whole plan: **none** of the 23 (each is claimed by one row above).
- Not covered **if the budget ends at cut 2**: 17 items, `ws.find-in-files`,
  `ws.replace-in-files`, `diag.tool-fake`, `diag.cargo-check`, `diag.positions`, `edit.indent`,
  `edit.comment-toggle`, `fmt.rustfmt`, `fmt.idempotent`, `fmt.error`, `io.external-change`,
  `text.rope`, `cmd.palette`, `keymap.default`, `keymap.custom`, `ui.search-panel`,
  `ui.diagnostics-panel`.
- UI that no item covers, so no evaluation can count it (Q-3): outline and navigation (T-060),
  the find bar (T-061), and the UI parts of T-025, T-026, T-027 and T-031. `ws.replace-in-files`
  is claimed through the CLI by T-022; no `ui.*` item names replace in files.
- Not ordered in this stage (D-145 names none of them; they stay drafts for the director):
  T-007, T-009, T-029, T-030, T-034, T-044, T-045 to T-048, T-050, T-053. Of these, T-053 and
  T-050 correct published texts and would fit beside step 3 if the director wants them.

### Limit: order and UI start are confounded (the analyst's objection, retro-2s1)

This stage changes the order (surface before depth, D-140) and starts the first L5 work and the
toolkit dependency at the same time. A result, good or bad, cannot be put down to the order
alone: UI tasks may cost more or less than non-UI tasks for reasons of their own (a new toolkit,
a new observation interface, snapshot-based tests), and the estimates above are non-UI medians.
What the stage can show: whether UI items observed by the evaluator through ADR-0008 are
confirmed or rejected, how many UI controls are reported inert (ADR-0008 (O5)), and the cost per
UI item; it cannot show that surface-first is better or worse than depth-first. Recorded in the
uncertainty register of the stage's retrospective.

## Consequences

- Drafts filed with this ADR: T-054 (profile), T-057 to T-063 (UI modules). Specs written:
  T-055, T-056. T-025's claim corrected. Next free task number: T-064.
- D-146 files a low-tier task after this numbering (the charter's state list, `docs/development.md`
  and `AGENTS.md` brought to v1.3). It touches `docs/development.md`, so it joins the chain of
  step 1 (T-052 → T-054 → T-043) and cannot be in flight with any of them; at the low-tier
  estimate it adds about 431,138 (build plus planning) to every cumulative figure from its place
  on.
- Every UI task's acceptance tests drive the built `kiln` binary through `kiln script`
  (ADR-0008) and live in `apps/kiln/tests/`; every UI scorecard item is confirmed only by
  (O1)–(O5).
- The evaluator's brief and template change as ADR-0008 says, before evaluation A.
- CHARTER §5 text: none changed; the stage-2 exit criterion stays "every item verified;
  evaluator sign-off; budgets enforced", which the plan reaches only at cut 4.
- A stage-scope change gets a row (D-144 change 15): if the budget ends before row 22, the
  director's row lists the rows not done.

## Questions for the director

- **Q-1, Q-2**: in ADR-0008 (how the UI reaches the session; what counts for `ui.window`).
- **Q-3. UI without an item (T-060, T-061, and the UI of T-025 to T-031).** A: add items through
  a publish task before each is specified (for example `ui.outline-panel`, `ui.find-bar`), so the
  counting rule applies; the scorecard grows past 64. B: build them without items (they count
  for the owner's use, not for the scorecard). C: leave them out of the stage. **Recommendation:
  A for T-060 and T-061** (both are basic editor use and D-145 names them), B for the smaller UI
  parts of step 4.
- **Q-4**: in T-056 (Enter before T-025).
- **Q-5. T-033's place.** A: row 21, after the product work (recommended, reasons above). B:
  before step 4 (row 12), so the step's joins are counted by the tool. C: before the UI
  foundation, as ADR-0005 had it in stage 1. **Recommendation: A.**
- **Q-6. Evaluation runs at the cut lines.** A: an evaluation after row 6 and after row 9 (and
  at the end), so that a budget stop leaves verified items and the counting rule is tried early
  on three items. B: one evaluation at the end (cheaper by about 0.8 M if no stop comes).
  **Recommendation: A.**

## Verification

At the stage's close: which rows merged, at what cost against the estimate; the scorecard
figure against the cut table; for each `ui.*` item, the evaluator's (O1)–(O5) record. The
retrospective compares cost per UI item with cost per non-UI item and names the confounding
above.

## Rollback

A decision row can reorder, cut or add rows; tasks already merged stay. If ADR-0008 is
rejected, rows 5 onward are re-planned, since no UI item could be counted.

## Amendment at acceptance (director, D-147)
Accepted with a changed dispatch order, for the usage budget (D-143: the weekly limit was reached once):
1. T-055, then T-056 (after its spec check), then **evaluation A**, limited to the three UI items T-056 claims and a
   small regression sample. This is cut 1: a desktop editor for one file.
2. T-057, T-058, T-059, then **evaluation B** (cut 2: a project editor).
3. The corrections and tooling this ADR places first (T-051, T-052, T-054, T-043, and T-064 for the texts outside the
   ledger) are dispatched when what they remove starts to cost: T-054 when disk limits the number of worktrees with
   the UI toolkit built; T-043 at the next queue refusal or record error of the kinds it prevents; T-051, T-052 and
   T-064 before the evaluation that would otherwise need their interim rulings again. None is dropped.
4. The rest in this ADR's order (T-060, T-061; the feature drafts each followed by its UI module; T-033 after the
   product work; T-041 last, the owner told first).
T-055 no longer waits for T-054; whichever of the two is second rebases on the other's change of the root `Cargo.toml`.
Answers to this ADR's questions: Q-3 (UI without an item) is decided when T-060 is specified; Q-5 A; Q-6 A with
evaluation A limited as above.
