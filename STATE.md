# State

_Last updated: 2026-11-15 02:00 UTC_

## Now
Iteration 2, **second stage** (ADR-0007, accepted with a changed order in D-147), under methodology **v1.3**
(D-146). The first stage is closed (D-142): scorecard on main 64 items: 41 verified, 0 implemented, 0 failing,
23 planned. Retrospective of the first stage: `retrospectives/iter-2-stage-1.md` (decided in D-144).

Order of the stage: T-055 (merged) → T-056 (editor view) → T-065 (window; spec check once T-056 has merged) →
evaluation A, limited (cut 1: a desktop editor for one file) → T-057, T-058, T-059 →
evaluation B (cut 2: a project editor) → T-060, T-061, the feature drafts each followed by its UI module → T-033 →
T-041 (last; owner told first). Deferred until what they remove starts to cost: T-051, T-052, T-054, T-043, T-064.

In flight:
- **T-055** (`kiln script`) merged at `e34ba1f` (D-155) and **T-056** (editor view) merged at `30435d1` (D-162).
  Scorecard on main: 64 items: 41 verified, 2 implemented (`ui.editor-view`, `ui.dispatch-only`), 0 failing,
  21 planned. No worktree exists.
- **T-065** window (high; `ui.window`; spec check R-246, rulings D-163): implementer run R-247 in
  `.worktrees/T-065`, independent test author run R-248 in `.worktrees/T-065-tests`, both from main
  `025cc78900d63c90cb04ab6794b45d5bab8c157b`. It adds the windowing dependency. After its merge: evaluation A
  (cut 1: a desktop editor for one file), where the director stops and reports to the owner.

Rules added in this stage (D-153, D-158, D-162): a `touches` path belongs to one role; a task's tier is the highest
tier among the items it claims; the director commits nothing on a task branch but joins and scorecard ties.

The planner's estimate for the whole stage is about 22.8 M tokens; about 5.5 M to cut 1 and 8.9 M to cut 2 (with
the deferred corrections counted). The weekly usage limit was reached once (D-143). Next free task number: T-067.

## Resume
Read `docs/CHARTER.md`, `methodology/METHODOLOGY.md`, the last rows of `docs/decisions.md`, the newest file in
`handoffs/` (its last addendum first), then `tasks/` (state in front matter, role logs at the bottom), `metrics/`
and `retrospectives/`. Worktrees live in `.worktrees/` (not pushed): after any restart run `uptime -s` and
`git status` in each before touching it. The container restarted four times on 2026-11-14 (02:11Z, 02:59Z, 07:11Z, 18:15Z).
