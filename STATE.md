# State

_Last updated: 2026-11-14 08:45 UTC_

## Now
Iteration 2, first stage (ADR-0005), under methodology **v1.2** (D-093). Merged in iteration 2: T-039, T-042, T-019
(project tree), T-020 (syntax), T-021 (search), T-036 (root identity), T-023 (outline and navigation), T-037 (re-open
check). Scorecard on main: 64 items: 28 verified, 13 implemented, 0 failing, 23 planned. All thirteen items planned
for the first stage are implemented; the 60% line (39 of 64) is reached only if the evaluator confirms them (D-127).

In flight (see the newest addendum in `handoffs/2026-11-13-iteration-2-start.md` for heads and next steps):
- **T-038** error kinds match commands (high; ADR-0006, D-128, j1 in D-136): joined green on its first join at
  `9fd40ae2a87d4007c34719e2fb7dd65b8d3684aa`; review round 1 running (run R-218).

Then: the evaluation of the first stage (after T-038 merges; the brief is prepared), the retrospective, and the UI
tasks of the second stage. Tooling T-033, T-043, T-041 (in that order; T-041 last, owner told first) after the
evaluation. Drafted outside the first stage, each needing a spec check before dispatch: T-044, T-045 to T-048
(deterministic limits, D-131) and the deferred drafts.

## Resume
Read `docs/CHARTER.md`, `methodology/METHODOLOGY.md`, the last rows of `docs/decisions.md`, the newest file in
`handoffs/` (its last addendum first), then `tasks/` (state in front matter, role logs at the bottom), `metrics/`
and `retrospectives/`. Worktrees live in `.worktrees/` (not pushed): after any restart run `uptime -s` and
`git status` in each before touching it. The container restarted three times on 2026-11-14 (02:11Z, 02:59Z, 07:11Z).
