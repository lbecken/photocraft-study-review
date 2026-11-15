# State

_Last updated: 2026-11-15 05:25 UTC_

## Now
Iteration 2, second stage (ADR-0007 as amended by D-147), under methodology **v1.3** (D-146).
**Cut 1 is reached and evaluated (D-170): the director has stopped to report to the owner; nothing is in flight.**

Scorecard on main: 64 items: 44 verified, 0 implemented, 0 failing, 20 planned (68.8%). Merged in this stage:
T-055 (`kiln script`, the UI without a window, ADR-0008), T-056 (editor view), T-065 (the window), T-067 (the
evaluation at cut 1, `evaluations/iter-2-cut-1.md`, D-169). The product: `kiln [--root <dir>] [<path>…]` opens a
window that shows and edits one document; Linux with X11 only (D-166). No worktree exists.

Cost: second stage so far 4,645,569 tokens booked (tasks 3,075,649; overhead 1,569,920); all runs 30,203,036.

What comes next is the owner's choice (D-170):
- **Cut 2** (a project editor): T-057 project panel, T-058 tabs, T-059 status bar (drafts; each needs its spec
  check, by resuming the planner that drafted it: D-163), then evaluation B.
- **From the evaluations and reviews:** T-066 (what a snapshot reports for shaped text: right-to-left rows, colours
  after a combining mark), T-068 (UI requests that do nothing; the no-window line), T-053 (published texts).
- **Deferred corrections and tooling:** T-051, T-052, T-064; T-043, T-033 (with the notes of D-153 and D-162),
  T-054, T-041 (last; owner told first).
- **Not yet run:** a retrospective of the second stage so far (the material is in D-149 to D-170).
- Further drafts: T-060 to T-063 (UI modules), T-022, T-024 to T-031 (features), T-044 to T-048, T-050, T-007,
  T-009, T-034. Next free task number: T-069.

Rules added in this stage (D-153, D-158, D-162, D-163): a `touches` path belongs to one role; a task's tier is the
highest tier among the items it claims and is fixed at specification; the director commits nothing on a task
branch but joins and scorecard ties; a spec check is done by resuming the planner that drafted the spec.

## Resume
Read `docs/CHARTER.md`, `methodology/METHODOLOGY.md`, the last rows of `docs/decisions.md`, the newest file in
`handoffs/` (its last addendum first), then `tasks/` (state in front matter, role logs at the bottom), `metrics/`
and `retrospectives/`. Worktrees live in `.worktrees/` (not pushed): after any restart run `uptime -s` and
`git status` in each before touching it. The container restarted four times on 2026-11-14 (02:11Z, 02:59Z, 07:11Z, 18:15Z).
