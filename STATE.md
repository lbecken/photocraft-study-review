# State

_Last updated: 2026-11-14 10:30 UTC_

## Now
Iteration 2, first stage (ADR-0005), under methodology **v1.2** (D-093). Every product task of the stage is merged:
T-039, T-042, T-019 (project tree), T-020 (syntax), T-021 (search), T-036 (root identity), T-023 (outline and
navigation), T-037 (re-open check), T-038 (declared refusal kinds, `76422d0`, D-139). Scorecard on main: 64 items:
28 verified, 13 implemented, 0 failing, 23 planned. The 60% line (39 of 64) is reached only if the evaluator confirms
at least 11 of the 13 implemented items and no verified item is rejected.

In flight: the **evaluation of the first stage** (run R-221, fresh evaluator, clean clone of `0704b24`, public
interfaces only, booked to `iter-2`). No worktree exists.

Then: publish `evaluations/iter-2.md` with `cargo xtask publish` (next free task number T-049; file its findings and
the follow-up of j5, D-137), the retrospective of the stage, decisions on the methodology, and the UI tasks of the
second stage (the owner's remark on PhotoCraft's order, surface first and depth later, is an input: D-140). Tooling
T-033, T-043, T-041 (in that order; T-041 last, owner told first). Drafts needing a spec check before dispatch:
T-044, T-045 to T-048 and the deferred drafts (six of them carry a note from T-038 on declaring refusals).

## Resume
Read `docs/CHARTER.md`, `methodology/METHODOLOGY.md`, the last rows of `docs/decisions.md`, the newest file in
`handoffs/` (its last addendum first), then `tasks/` (state in front matter, role logs at the bottom), `metrics/`
and `retrospectives/`. Worktrees live in `.worktrees/` (not pushed): after any restart run `uptime -s` and
`git status` in each before touching it. The container restarted three times on 2026-11-14 (02:11Z, 02:59Z, 07:11Z).
