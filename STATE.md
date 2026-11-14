# State

_Last updated: 2026-11-14 06:05 UTC_

## Now
Iteration 2, first stage (ADR-0005), under methodology **v1.2** (D-093). Merged in iteration 2: T-039, T-042, T-019
(project tree), T-020 (syntax), T-021 (search), T-036 (root identity), T-023 (outline and navigation). Scorecard on
main: 64 items: 28 verified, 13 implemented, 0 failing, 23 planned. All thirteen items planned for the first stage are implemented; the
60% line (39 of 64) is reached only if the evaluator confirms them (D-127).

In flight (see the newest addendum in `handoffs/2026-11-13-iteration-2-start.md` for heads and next steps):
- **T-037** re-open check (high): in rework after review round 1 (rulings k1 to k5, D-125); the test author's part is
  done, the implementer's is running.
- **T-038** error kinds (high): pre-dispatch spec check running; dispatched after T-037 merges.

Then: the evaluation of the first stage (after T-037 and T-038 merge), the retrospective, and the UI tasks of the
second stage. Tooling T-033, T-043, T-041 (in that order; T-041 last, owner told first) can run beside the evaluation.
Drafted outside the first stage: T-044 and the deferred drafts. Not yet filed: one follow-up task on deterministic
limits (parse work, search work, `.gitignore` cost, the size of a highlight reply).

## Resume
Read `docs/CHARTER.md`, `methodology/METHODOLOGY.md`, the last rows of `docs/decisions.md`, the newest file in
`handoffs/` (its last addendum first), then `tasks/` (state in front matter, role logs at the bottom), `metrics/`
and `retrospectives/`. Worktrees live in `.worktrees/` (not pushed): after any restart run `uptime -s` and
`git status` in each before touching it. The container restarted twice on 2026-11-14 (02:11Z, 02:59Z).
