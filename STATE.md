# State

_Last updated: 2026-11-16 02:51 UTC_

## Now
**Owner's instruction of 2026-11-16 (D-207): the `ci/…` branches can stay; finish cut 2 (T-059, evaluation B, report),
then pause. No new platform task is dispatched; T-054 (in review) is finished or stopped cleanly.**

Iteration 2, second stage (ADR-0007 as amended by D-147), under methodology **v1.4** (D-176).
Cut 1 is reached and evaluated (D-170). **The owner's instruction (D-171): run the retrospective, then go on to cut
2, and make the editor multi-platform (Linux, macOS, Windows).** Signed installers are out of scope (D-173); `ci/…`
branches may be pushed and are deleted after their task merges (D-185).

**PAUSED at cut 2 (D-207, D-216).** Nothing is in flight; no worktree exists. The evaluation at cut 2 is published
(`evaluations/iter-2-cut-2.md`, T-084, D-215): the three cut-2 items confirmed, none rejected. Waiting for the
owner's choice of what comes next: the platform track (T-071, T-079, T-072 to T-078), the 17 planned items, or
the follow-ups (T-082, T-083, T-085, T-086, T-066, T-068, T-053), and a retrospective of cut 2 (material: D-180
to D-216). Next free task number: T-087. Next run id: R-303. Next decision: D-217.

Merged since cut 1: T-069 (CI fix), T-057 (project panel), T-070 (`platforms.yml`: gate on three systems, the
Linux window procedure, window probes), T-058 (tabs), T-054 (lighter debug profile; the interim variables of
D-186 are gone, D-211), T-059 (status bar, `60cc322`, D-214). **Cut 2 is built.**

Platform track (paused by D-207): counted jobs `window-linux`, `window-probe (macos-15)` (D-202); not yet green
`gate-macos` (22 tests that assume Linux, D-199), `gate-windows` and `window-probe (windows-2025)` (does not
compile). Specified and waiting: T-071, T-079; drafts T-072 to T-078. Order when resumed: T-071, T-079, T-072,
T-073, T-074, T-075, T-076, T-077, T-078. The Windows verdict of T-054's backtrace guard is read on T-071's CI
round.

CI on main: `ci.yml` run 429 on `51ee7c1` (T-054's merge) green and the two counted platform jobs green there
(D-213); the runs on the push with T-059's merge (`20f63f4`) are read at the next addendum.

Scorecard on main: 64 items: 47 verified, 0 implemented, 0 failing, 17 planned. Merged in this stage: T-055
(`kiln script`, ADR-0008), T-056 (editor view), T-065 (the window), T-067 (evaluation at cut 1), T-069 (CI fix,
INC-001 closed in D-179), T-057 (project panel, `7e8a01c`, D-192). The product: `kiln [--root <dir>] [<path>…]`
opens a window with a project panel and one editor; Linux with X11 only until the platform tasks merge.

**Branches for the owner to delete (D-198; the director's deletion was refused with 403):** `ci/T-070`,
`ci/T-070-negative`, `ci/T-054` (pushed 23:36Z for ruling Q-4 A of T-054), `ci/T-058` (pushed 23:51Z), `ci/T-059` (pushed 01:36Z). The owner said on 2026-11-16 that they can stay for now (D-207).

Drafts: T-066, T-068, T-053, T-064, T-051, T-043, T-033, T-041 (last; owner told first), T-080, T-081, T-060 to
T-063, T-022, T-024 to T-031, T-044 to T-048, T-050, T-007, T-009, T-034. T-052 abandoned.

Methodology **v1.4** is in force (D-176): among its rules, a task's tier is at least the highest tier of the items
it claims and is fixed at `specified`; rulings live on main and the director commits only joins and scorecard ties
on a task branch; a `touches` path belongs to one role; **after every push the director reads the CI run of the
pushed commit and records it** (INC-001, closed in D-179).

## Resume
Read `docs/CHARTER.md`, `methodology/METHODOLOGY.md`, the last rows of `docs/decisions.md`, the newest file in
`handoffs/` (its last addendum first), then `tasks/` (state in front matter, role logs at the bottom), `metrics/`
and `retrospectives/`. Worktrees live in `.worktrees/` (not pushed): after any restart run `uptime -s` and
`git status` in each before touching it. The container restarted four times on 2026-11-14 (02:11Z, 02:59Z, 07:11Z, 18:15Z) and on 2026-11-15 at 16:09Z and 21:11Z.
