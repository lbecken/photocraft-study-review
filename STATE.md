# State

_Last updated: 2026-11-18 22:10 UTC (paused)_

## Paused (D-283)
**The owner paused the work at 2026-11-18 22:09Z until the weekly usage limit resets (Saturday 2026-11-21, 5 PM
in the owner's time zone). Nothing runs and nothing is scheduled; resume when the owner says so.** To resume:
1. `uptime -s`, `git status` in the primary checkout, `git worktree list`. If `.worktrees/T-095` and
   `.worktrees/T-095-tests` are gone (the machine was replaced), re-create them from main `607fed20b7dd`
   (`git worktree add -q .worktrees/T-095 -b task/T-095 607fed20b7dd`, the same for `-tests`) and restore the
   commits with `git am < tasks/T-095-wip-implementer.patch` and `git am < tasks/T-095-wip-tests.patch` (heads
   must come out as trees equal to `88753e3` and `2720fcd`; compare with `git diff --stat` against the patch).
2. T-095 is `in_progress`, both builders were stopped mid-work (runs R-405, R-406 booked with what they had done;
   their agent ids may no longer be resumable): dispatch fresh builders (Opus 5.5) that continue from the branch
   as it is, each told what its predecessor had committed and that nothing was verified (no gate, no checks).
3. Read main's `ci.yml` and `platforms.yml` runs for the pushes of 21:32Z and 21:59Z on 2026-11-18 (T-093's and
   T-092's merges) and the newest nightly run: the counted jobs; a macOS job without a runner is re-run.
4. Have T-098 specified in full (a planner; the brief is in the transcript and in the handoff's last addendum),
   then its fresh spec check; go on with the platform track as listed below.
The patch files `tasks/T-095-wip-*.patch` are removed by the director when T-095 has merged.

## Now
Iteration 2, second stage (ADR-0007 as amended by D-147), under methodology **v1.5** (D-221). Cut 2 is built and
evaluated (`evaluations/iter-2-cut-2.md`, T-084, D-215). **The owner's instruction in force (D-218, 2026-11-17):
first the platform track (macOS, then Windows; no signed installers, D-173), then the 17 planned feature items.**
Follow-ups of cut 2 stay drafts unless a platform or feature task needs one. The `ci/…` branches stay (D-207);
`ci/candidate` is the one reused branch for CI evidence (a forced push there is allowed).

Scorecard on main: 64 items: 47 verified, 0 implemented, 0 failing, 17 planned.

**Platform track, state on 2026-11-18 22:05Z**
- Merged: T-070, T-071, T-079, and on 2026-11-18 six tasks: T-073 (`8bf4c3a`), T-096 (`aed3d88`), T-072 (`0deb173`),
  T-074 (`25054fe`), **T-093 (`7e7e082`, D-280: the macOS sentences of the protocol documents, each with its test
  or "unmeasured") and T-092 (`fded2a4`, D-282: 13 of the 15 test targets of `kiln-workspace` compile and pass on
  Windows)**.
- Counted jobs (a red run on main is an incident): the Linux gate (`ci.yml`), `window-linux`,
  `window-probe (macos-15)`, `gate-macos`. Not green yet: `gate-windows` (17 test targets of `kiln-cli`, `kiln`,
  `kiln-commands` and the two listing targets of `kiln-workspace` do not compile; then run-time failures and the
  scorecard step: T-098, T-100, T-099), `window-probe (windows-2025)` (no OpenGL 2.0 on the hosted runner: T-077).
- **macOS runners come and go** (D-261, D-264, D-270, INC-002 closed in D-276): a macOS job is sometimes cancelled
  after 15 minutes with no runner assigned. On a candidate: re-run, and D-262 as widened by D-270 says when an
  approval may go on. On main: re-run until it has a result, with an incident file open meanwhile.
- **In flight:** **T-095** (Windows names and case in `RelPath::new`; high; estimate 580,000; D-272, D-275):
  implementer R-405 (`.worktrees/T-095`), independent test author R-406 (`.worktrees/T-095-tests`). When the test
  author reports: an early CI look of the tests branch merged with main (ruling Q-C) fills the measured sentences
  before the join. Then join, CI round, review (a fresh reviewer), a row beginning `Approve T-095`, queue. The
  planner of T-092 is specifying **T-098** in full (R-407), with a cut if it is above about 900,000.
- **To read:** main's `ci.yml` and `platforms.yml` runs for the pushes that carried T-093's merge (21:32Z) and
  T-092's merge (21:59Z): the counted jobs, and on Windows the general step (13 of 15 expected).
- Then: T-098 (after a fresh spec check; beside T-095 if it shares no file), T-100 (after T-095), T-099 (makes
  `gate-windows` green and proposes counting it), T-076 (three `/dev/full` counterparts) and T-077 (the window on
  Windows and macOS) after T-098, T-078 (unsigned artefacts). T-075 is absorbed. T-097, T-094, T-091 are drafts.
- Pending registry entries on main expire 2026-12-31: 3 on T-076, 5 on T-077.
- After the track: a platform report to the owner, with the deferred tooling batch (T-043, T-087 to T-090; D-223:
  the owner decides), the Windows choices (D-254: reparse points refused, cloud placeholders with them; D-272: the
  names refused on Windows, reserved names allowed, case as a stated limit), the two uses of D-262's ruling, and
  the drafts T-091, T-094, T-097. Then the feature items, each draft re-specified in full under v1.5.

**Rules added by decisions since v1.5** (to go into the next methodology version): D-235 (a task that changes a
gate rule does not also use it; the spec check states what main's tool says about the candidate tree, and builder
and reviewer run main's `xtask` on it), D-249 (the spec check greps main's tests for every path the task changes),
D-229 (a ruling changed after a test was written to it goes to both builders with the test named), D-262 and D-270 (a counted job the host cancels before any step), D-266 and D-267 (the director does not rewrite a task branch; a join that needed a manual resolution cannot be replayed by the queue; every task file carries end-of-log markers), D-279 (every file a role writes is in `touches`, the review file of a low-tier task included), D-275 a1 (a guard that pins other tasks' entries or files is not written: that is the reviewer's diff check), D-277 b1 (a fixture's precondition is measured per platform; at least one fixture must be a real case).

**CI reading:** records come from uploaded artefacts by a fresh helper following `tasks/ci-runs/README.md`
(stored as `tasks/ci-runs/T-NNN-round-N.md`); small lookups by a light helper through the GitHub tools. The
nightly `platforms.yml` run on main (cron `17 3 * * *`) is read at each session start and handoff.

Ids: next task number T-101, next run R-408, next decision D-284.

Drafts: T-074 to T-078, T-091, T-092, T-094 (platform); T-043, T-087 to T-090 (tooling, deferred); T-082, T-083,
T-085, T-086, T-066, T-068, T-053, T-064, T-080, T-081, T-051, T-033, T-041 (last; the owner is told first),
T-060 to T-063, T-022, T-024 to T-031, T-044 to T-048, T-050, T-007, T-009, T-034. T-052 abandoned.
Specified and waiting: T-072, T-093, T-043.

Accepted work not done (v1.5 change 13): the tooling batch above; the retrospective material collected since
cut 2 (D-235, D-249, D-229, six restarts, booking slips, T-071's cost of 1,286,681 against 800,000, T-079 as the
first clean v1.5 task, T-073: a task for another platform has almost only guards on Linux).

## Resume
Read `docs/CHARTER.md`, `methodology/METHODOLOGY.md`, the last rows of `docs/decisions.md`, the newest file in
`handoffs/` (its last addendum first), then `tasks/` (state in front matter, role logs at the bottom), `metrics/`
and `retrospectives/`. Worktrees live in `.worktrees/` (not pushed): after any restart run `uptime -s` and
`git status` in each before touching it. The container restarted four times on 2026-11-14 (02:11Z, 02:59Z, 07:11Z, 18:15Z) and on 2026-11-15 at 16:09Z and 21:11Z.
