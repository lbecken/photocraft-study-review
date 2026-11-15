# State

_Last updated: 2026-11-15 21:40 UTC_

## Now
Iteration 2, second stage (ADR-0007 as amended by D-147), under methodology **v1.4** (D-176).
Cut 1 is reached and evaluated (D-170). **The owner's instruction (D-171): run the retrospective, then go on to cut
2, and make the editor multi-platform (Linux, macOS, Windows).** Signed installers are out of scope (D-173); `ci/…`
branches may be pushed and are deleted after their task merges (D-185).

In flight:
- **T-070** (CI jobs for macOS and Windows; high; `in_progress`): review round 1 was request-changes (D-193,
  rulings g1 to g7 in the task file). Rework running: implementer R-277 (`.worktrees/T-070`), test author R-278
  (`.worktrees/T-070-tests`). Then: merge the tests branch, re-join, push the joined head to `ci/T-070` (CI round
  2, AC-4 and AC-5), read the run on `ci/T-070-negative` (`4bc231c`, pushed 21:32:20Z; AC-6 per g5), review round
  2, a row beginning `Approve T-070` with the words `gate paths`, queue; after the merge read the first
  `platforms.yml` runs on main and delete `ci/T-070`, `ci/T-070-negative` and the local `scratch/T-070-negative`.
- **T-058** (tabs, cut 2; medium; `in_progress`; spec R-276, rulings D-194): implementer R-279 (`.worktrees/T-058`),
  independent test author R-280 (`.worktrees/T-058-tests`).
- **T-059** (status bar, cut 2; medium; `specified`): depends on T-058; spec check after T-058 merges.
- **T-054** (lighter build profile; high): specified (D-186), dispatched after T-070 merges. Interim: builds run
  with `CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0`.
Next (ADR-0009 Q-4): T-071 and T-079 after T-070; T-066; T-043; T-072 to T-076; T-077, T-078. After T-057, T-058
and T-059: evaluation B, then **stop and report to the owner at cut 2**. Next free task number: T-082.

CI on main: run 392 of `ci.yml` on `c288f7f` (the push with T-057's merge) green (read 21:35Z); the runs of later
ledger pushes are read at the next addendum.

Scorecard on main: 64 items: 44 verified, 1 implemented, 0 failing, 19 planned. Merged in this stage: T-055
(`kiln script`, ADR-0008), T-056 (editor view), T-065 (the window), T-067 (evaluation at cut 1), T-069 (CI fix,
INC-001 closed in D-179), T-057 (project panel, `7e8a01c`, D-192). The product: `kiln [--root <dir>] [<path>…]`
opens a window with a project panel and one editor; Linux with X11 only until the platform tasks merge.

**Branches for the owner to delete (D-198; the director's deletion was refused with 403):** `ci/T-070`,
`ci/T-070-negative`.

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
