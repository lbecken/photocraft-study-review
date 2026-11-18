# State

_Last updated: 2026-11-18 18:55 UTC_

## Now
Iteration 2, second stage (ADR-0007 as amended by D-147), under methodology **v1.5** (D-221). Cut 2 is built and
evaluated (`evaluations/iter-2-cut-2.md`, T-084, D-215). **The owner's instruction in force (D-218, 2026-11-17):
first the platform track (macOS, then Windows; no signed installers, D-173), then the 17 planned feature items.**
Follow-ups of cut 2 stay drafts unless a platform or feature task needs one. The `ci/…` branches stay (D-207);
`ci/candidate` is the one reused branch for CI evidence (a forced push there is allowed).

Scorecard on main: 64 items: 47 verified, 0 implemented, 0 failing, 17 planned.

**Platform track, state on 2026-11-18 18:55Z**
- Merged: T-070, T-071, T-079, T-073 (`8bf4c3a`), **T-096 (`aed3d88`, D-263), T-072 (`0deb173`, D-268: every gate
  job uploads its test list and prints failing blocks; one general Windows step reports `kiln-workspace`'s test
  targets), T-074 (`25054fe`, D-271: on Windows every reparse point on the walk is outside the root; saves go on
  past entries at temporary names)**.
- Counted jobs (a red run on main is an incident): the Linux gate (`ci.yml`), `window-linux`,
  `window-probe (macos-15)` (D-202), `gate-macos` (D-248). Not green yet: `gate-windows` (26 test targets do not
  compile on Windows, 6 fail: T-092, T-098, T-099), `window-probe (windows-2025)` (no OpenGL 2.0 on the hosted
  runner: T-077).
- **macOS runners come and go** (D-261, D-264, D-270): between 13:00Z and 16:40Z GitHub assigned none; since then
  `window-probe (macos-15)` on `ci/candidate` was twice cancelled with no runner while it ran on main. D-262's
  ruling (a job cancelled before any step has no result) was used for T-096 and, widened to evidence by runs, for
  T-074 (D-270). **To read: main's runs after T-074's merge (push at 18:50Z), the macOS probe first.**
- **In flight (planners):** a fresh spec check of T-092 (`kiln-workspace`'s test targets on Windows; high; about
  950,000; spec final, D-269); T-095 being specified in full (names and case in `path.rs`); T-093's spec being
  re-checked (macOS document sentences; low). The order among the three is set from their `touches`.
- Then (D-269): T-098 (the 17 test targets of `kiln-cli`, `kiln`, `kiln-commands`; about 1.0 M), T-099 (run-time
  failures and the scorecard step: makes `gate-windows` green and proposes counting it), T-076 (narrowed to three
  `/dev/full` counterparts) and T-077 (the window on Windows and macOS) after T-098, T-078 (unsigned artefacts).
  T-075 is absorbed. T-097 (toolchain bump readiness) and T-094 are drafts, not urgent.
- Pending registry entries on main expire 2026-12-31: 2 on T-092, 3 on T-076, 5 on T-077.
- After the track: a platform report to the owner, with the deferred tooling batch (T-043, T-087 to T-090; D-223:
  the owner decides), the stated Windows limits (cloud placeholders and deduplicated files are refused, D-254),
  the two uses of D-262's ruling, and the drafts T-091, T-094, T-097. Then the feature items, each draft
  re-specified in full under v1.5.

**Rules added by decisions since v1.5** (to go into the next methodology version): D-235 (a task that changes a
gate rule does not also use it; the spec check states what main's tool says about the candidate tree, and builder
and reviewer run main's `xtask` on it), D-249 (the spec check greps main's tests for every path the task changes),
D-229 (a ruling changed after a test was written to it goes to both builders with the test named), D-262 and D-270 (a counted job the host cancels before any step), D-266 and D-267 (the director does not rewrite a task branch; a join that needed a manual resolution cannot be replayed by the queue; every task file carries end-of-log markers).

**CI reading:** records come from uploaded artefacts by a fresh helper following `tasks/ci-runs/README.md`
(stored as `tasks/ci-runs/T-NNN-round-N.md`); small lookups by a light helper through the GitHub tools. The
nightly `platforms.yml` run on main (cron `17 3 * * *`) is read at each session start and handoff.

Ids: next task number T-100, next run R-385 (the T-092 spec check; R-386 the T-095 planner; R-387 the T-093 re-check), next decision D-272.

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
