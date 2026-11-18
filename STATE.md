# State

_Last updated: 2026-11-18 10:50 UTC_

## Now
Iteration 2, second stage (ADR-0007 as amended by D-147), under methodology **v1.5** (D-221). Cut 2 is built and
evaluated (`evaluations/iter-2-cut-2.md`, T-084, D-215). **The owner's instruction in force (D-218, 2026-11-17):
first the platform track (macOS, then Windows; no signed installers, D-173), then the 17 planned feature items.**
Follow-ups of cut 2 stay drafts unless a platform or feature task needs one. The `ci/…` branches stay (D-207);
`ci/candidate` is the one reused branch for CI evidence (a forced push there is allowed).

Scorecard on main: 64 items: 47 verified, 0 implemented, 0 failing, 17 planned.

**Platform track, state on 2026-11-18 10:50Z**
- Merged: T-070 (`platforms.yml`), T-071 (`0f8ea64`, the platform-test rule), T-079 (`bc8d097`, registry and
  portable fixes: macOS gate green), **T-073 (`8bf4c3a`, D-252: `kiln-workspace` and `kiln` build for Windows; the
  root held and identified there; 47 of 47 file-system tests pass on the Windows runner)**.
- Counted jobs (a red run on main is an incident): the Linux gate (`ci.yml`), `window-linux`,
  `window-probe (macos-15)` (D-202), `gate-macos` (D-248). Not green yet: `gate-windows` (other crates' test
  targets: T-092), `window-probe (windows-2025)` (no OpenGL 2.0 on the hosted runner: T-077).
- **In flight:** a planner specifies T-074 in full under v1.5 (Windows confinement rules; high; carries 9 pending
  registry entries); another re-checks T-072's spec (CI evidence) against the tree after T-073. Both are then
  dispatched side by side (D-245). CI on the push with T-073's merge is still to be read.
- Order after that (D-245): T-092 (product tests on Windows) with T-093 (macOS document sentences) beside it,
  then T-075, T-076 (becomes high tier), T-077, T-078 (unsigned artefacts only).
- Pending registry entries expire 2026-12-31: 9 on T-074, 2 on T-092, 3 on T-076, 5 on T-077.
- After the track: a platform report to the owner, with the deferred tooling batch (T-043, T-087 to T-090, about
  4 to 5 M tokens; D-223: the owner decides) and the drafts T-091, T-094. Then the feature items, each draft
  re-specified in full under v1.5.

**Rules added by decisions since v1.5** (to go into the next methodology version): D-235 (a task that changes a
gate rule does not also use it; the spec check states what main's tool says about the candidate tree, and builder
and reviewer run main's `xtask` on it), D-249 (the spec check greps main's tests for every path the task changes),
D-229 (a ruling changed after a test was written to it goes to both builders with the test named).

**CI reading:** records come from uploaded artefacts by a fresh helper following `tasks/ci-runs/README.md`
(stored as `tasks/ci-runs/T-NNN-round-N.md`); small lookups by a light helper through the GitHub tools. The
nightly `platforms.yml` run on main (cron `17 3 * * *`) is read at each session start and handoff.

Ids: next task number T-095, next run R-348, next decision D-253.

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
