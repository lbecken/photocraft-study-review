# State

_Last updated: 2026-11-15 21:15 UTC_

## Now
Iteration 2, second stage (ADR-0007 as amended by D-147), under methodology **v1.3** (D-146).
Cut 1 is reached and evaluated (D-170). **The owner's instruction (D-171): run the retrospective, then go on to cut
2, and make the editor multi-platform (Linux, macOS, Windows).**

Decided since: the retrospective to cut 1 (D-174), ADR-0009 on platforms (D-175; signed installers out of scope,
D-173), methodology v1.4 (D-176). **INC-001 (CI on main red since T-036) is closed: T-069 merged, CI run 366 green
at 2026-11-15T18:02:59Z (D-179).**

In flight (after the usage limit of about 20:35Z and the restart of 21:11Z, D-190):
- **T-057** (project panel, cut 2; medium): rework after review round 1 merged; second join green at `6335016`
  (12 acceptance tests); review round 2 running (run R-275, the round-1 reviewer resumed).
- **T-070** (CI jobs for macOS and Windows; high): CI round 1 on `ci/T-070` recorded (D-187: Linux gate, Linux
  window procedure and the macOS window probe green; Windows red as expected; macOS gate red with a first list of
  failing tests); local join green at `bbbb0e5` (8 acceptance tests, 3 guards); review round 1 running (run R-274,
  resumed after the limit); it prepares the negative candidate for AC-6, which the director pushes to
  `ci/T-070-negative` (the owner allows `ci/…` branches, D-185).
- **T-054** (lighter build profile; high): specified (D-186), dispatched after T-070 merges. Interim: builds run
  with `CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0`.
Next (ADR-0009 Q-4): T-071 and T-079 after T-070; T-058, T-059; T-066; T-043; T-072 to T-076; T-077, T-078. Next
free task number: T-082. CI on main: run 388 on `1072708` green (read 21:13Z).

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

Methodology **v1.4** is in force (D-176): among its rules, a task's tier is at least the highest tier of the items
it claims and is fixed at `specified`; rulings live on main and the director commits only joins and scorecard ties
on a task branch; a `touches` path belongs to one role; **after every push the director reads the CI run of the
pushed commit and records it** (INC-001, closed in D-179).

## Resume
Read `docs/CHARTER.md`, `methodology/METHODOLOGY.md`, the last rows of `docs/decisions.md`, the newest file in
`handoffs/` (its last addendum first), then `tasks/` (state in front matter, role logs at the bottom), `metrics/`
and `retrospectives/`. Worktrees live in `.worktrees/` (not pushed): after any restart run `uptime -s` and
`git status` in each before touching it. The container restarted four times on 2026-11-14 (02:11Z, 02:59Z, 07:11Z, 18:15Z) and on 2026-11-15 at 16:09Z and 21:11Z.
