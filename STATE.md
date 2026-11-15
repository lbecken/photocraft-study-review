# State

_Last updated: 2026-11-15 23:22 UTC_

## Now
Iteration 2, second stage (ADR-0007 as amended by D-147), under methodology **v1.4** (D-176).
Cut 1 is reached and evaluated (D-170). **The owner's instruction (D-171): run the retrospective, then go on to cut
2, and make the editor multi-platform (Linux, macOS, Windows).** Signed installers are out of scope (D-173); `ci/…`
branches may be pushed and are deleted after their task merges (D-185).

In flight:
- **T-058** (tabs, cut 2; medium; ceiling extended to 1,100,000 by D-196): review round 1 was request-changes
  (D-201, rulings h1 to h8); both builders reworked; second join running at `1f895df`; then review round 2
  (resume the round-1 reviewer), queue.
- **T-054** (lighter build profile; high; D-200): test author done (`28164d8`, R-287); implementer R-286 running in
  `.worktrees/T-054`. Its builders, join and queue run with the interim variables UNSET. Before the queue the
  joined head goes to `ci/T-054` (ruling Q-4 A).
- **T-059** (status bar, cut 2; `specified`): spec check after T-058 merges.
- **T-071** (platform-test rule in the tools; high) and **T-079** (mark the product tests by platform; high):
  `specified` (D-199). Order: T-054, T-071, T-079, T-072, T-073. The macOS gate's 22 failing tests are all
  Linux assumptions of tests by the planner's reading (D-199).
After T-058 and T-059: evaluation B, then **stop and report to the owner at cut 2**. Next free task number: T-083.

Merged since the last note: **T-070** (`c7af15f`, D-197): `platforms.yml` is on main. Counted jobs (D-202):
`window-linux`, `window-probe (macos-15)`; not yet green: `gate-macos`, `gate-windows`, `window-probe (windows-2025)`.

CI on main: `ci.yml` runs 405 (`786b686`, T-070's merge) and 406 green (read 23:00Z); `platforms.yml` run
43770955835 read (D-202).

Scorecard on main: 64 items: 44 verified, 1 implemented, 0 failing, 19 planned. Merged in this stage: T-055
(`kiln script`, ADR-0008), T-056 (editor view), T-065 (the window), T-067 (evaluation at cut 1), T-069 (CI fix,
INC-001 closed in D-179), T-057 (project panel, `7e8a01c`, D-192). The product: `kiln [--root <dir>] [<path>…]`
opens a window with a project panel and one editor; Linux with X11 only until the platform tasks merge.

**Branches for the owner to delete (D-198; the director's deletion was refused with 403):** `ci/T-070`,
`ci/T-070-negative`, `ci/T-054` (pushed 23:36Z for ruling Q-4 A of T-054).

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
