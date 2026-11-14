# State

_Last updated: 2026-11-14 03:02 UTC_

## Now
Iteration 2, first stage (ADR-0005), under methodology **v1.2** (D-093). Merged in iteration 2: T-039, T-042, T-019
(project tree), T-020 (syntax). Scorecard on main: 64 items: 28 verified, 5 implemented, 0 failing, 31 planned.
The first stage needs 39 verified (60%); T-021 and T-023 carry the remaining eight items of the plan, and the
evaluator must confirm all thirteen new ones.

In flight (see the newest addendum in `handoffs/2026-11-13-iteration-2-start.md` for heads and next steps):
- **T-021** search: in rework after review round 1 (rulings q1 to q6, D-116; cap D-115).
- **T-036** root-identity gap (high): implementer and independent test author building.

Specified and waiting: T-023 (after T-021 merges), T-037 (after T-036), T-038 (after the feature tasks), tooling
T-033, T-043, T-041. Drafted outside the first stage: T-044 and the deferred drafts. Not yet filed: one follow-up
task on deterministic limits (parse work, search work, `.gitignore` cost, the size of a highlight reply).

## Resume
Read `docs/CHARTER.md`, `methodology/METHODOLOGY.md`, the last rows of `docs/decisions.md`, the newest file in
`handoffs/` (its last addendum first), then `tasks/` (state in front matter, role logs at the bottom), `metrics/`
and `retrospectives/`. Worktrees live in `.worktrees/` (not pushed): after any restart run `uptime -s` and
`git status` in each before touching it. The container restarted twice on 2026-11-14 (02:11Z, 02:59Z).
