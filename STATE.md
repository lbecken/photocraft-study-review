# State

_Last updated: 2026-11-18 16:50 UTC_

## Now
Iteration 2, second stage (ADR-0007 as amended by D-147), under methodology **v1.5** (D-221). Cut 2 is built and
evaluated (`evaluations/iter-2-cut-2.md`, T-084, D-215). **The owner's instruction in force (D-218, 2026-11-17):
first the platform track (macOS, then Windows; no signed installers, D-173), then the 17 planned feature items.**
Follow-ups of cut 2 stay drafts unless a platform or feature task needs one. The `ci/…` branches stay (D-207);
`ci/candidate` is the one reused branch for CI evidence (a forced push there is allowed).

Scorecard on main: 64 items: 47 verified, 0 implemented, 0 failing, 17 planned.

**Platform track, state on 2026-11-18 16:50Z**
- Merged: T-070, T-071, T-079, T-073 (`8bf4c3a`, D-252), **T-096 (`aed3d88`, D-263: a tool test no longer depends on
  the runner's default Rust; found on a candidate, D-258)**.
- Counted jobs (a red run on main is an incident): the Linux gate (`ci.yml`), `window-linux`,
  `window-probe (macos-15)` (D-202), `gate-macos` (D-248). Not green yet: `gate-windows` (other crates' test
  targets: T-092), `window-probe (windows-2025)` (no OpenGL 2.0 on the hosted runner: T-077).
- **GitHub assigns no macOS runner to this repository** (D-261): `window-probe (macos-15)` since about 13:00Z,
  `gate-macos` since about 15:16Z; jobs are cancelled after 15 minutes with no runner; no public incident; the
  owner is told. D-262's ruling (a job cancelled before any step has no result; the clause counts as met only if
  the diff cannot bear on the job, its last run on main was green, and a re-run was tried) was used once, for T-096.
- **T-072** (CI evidence; high): 4 joins, review rounds 1 and 2 (approve at `0dbeac1`, `reviews/T-072-r2.md`);
  823,527 tokens of 1,100,000. **Open: the CI criterion** — its head merged with main (`1275379`) and the negative
  candidate of AC-8 (`0f8e5b9`) went to `ci/candidate` at 16:43Z: read Linux and Windows now; the macOS clauses
  need a runner (its diff bears on `gate-macos`, so D-262's ruling does not apply). Then a row `Approve T-072`
  with `gate paths`, `reviewed`, `queued`, queue.
- **T-074** (Windows confinement; high): 2 joins, review rounds 1 and 2 (approve at `1d62ae0`, addendum `ed02d3e`);
  897,911 of 1,100,000. Two early CI looks (`tasks/ci-runs/T-074-early-1.md`, `-early-2.md`) show its 15 guards
  passing on Windows (176 `MEASURED:` lines) and on macOS, the library 43 of 43. **Open: the formal CI round** on
  its head merged with a main that holds T-072 (D-254), macOS included. Then `Approve T-074`, queue. Worktrees
  `.worktrees/T-074`, `-tests` (build directories deleted for disk; a rebuild takes about 10 minutes).
- **T-092** (Windows gate green): a planner is writing the full specification and a cut (drafts T-098 on).
- After T-074: T-095 (names and case in `path.rs`; draft, to be specified against T-074's merged tree), T-093
  beside T-092, then T-075, T-076 (becomes high tier), T-077, T-078 (unsigned artefacts only). T-097 (toolchain
  bump readiness) is a draft, not urgent.
- Pending registry entries expire 2026-12-31: on main 9 on T-074, 2 on T-092, 3 on T-076, 5 on T-077 (T-074's
  branch settles its 9).
- After the track: a platform report to the owner, with the deferred tooling batch (T-043, T-087 to T-090; D-223:
  the owner decides), the stated Windows limits (cloud placeholders and deduplicated files are refused, D-254) and
  the drafts T-091, T-094, T-097. Then the feature items, each draft re-specified in full under v1.5.

**Rules added by decisions since v1.5** (to go into the next methodology version): D-235 (a task that changes a
gate rule does not also use it; the spec check states what main's tool says about the candidate tree, and builder
and reviewer run main's `xtask` on it), D-249 (the spec check greps main's tests for every path the task changes),
D-229 (a ruling changed after a test was written to it goes to both builders with the test named).

**CI reading:** records come from uploaded artefacts by a fresh helper following `tasks/ci-runs/README.md`
(stored as `tasks/ci-runs/T-NNN-round-N.md`); small lookups by a light helper through the GitHub tools. The
nightly `platforms.yml` run on main (cron `17 3 * * *`) is read at each session start and handoff.

Ids: next task number T-098 (the T-092 planner may take T-098 on), next run R-378 (the T-092 planner), next decision D-264.

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
