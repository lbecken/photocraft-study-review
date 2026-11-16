---
id: T-054
title: "Make a debug build of the workspace about a third of its size, by line tables only, no debug info for dependencies and no incremental cache, with the same gate result and file and line still in panics and backtraces"
tier: high              # ADR-0002 "a gate path": the root Cargo.toml's [profile.*] tables are gate tables (xtask/src/precheck.rs `gate_tables`); no claimed item; fixed at specified (METHODOLOGY v1.4 §4)
state: queued            # director only, written with every transition; see METHODOLOGY §3
owner_role: planner
scorecard_items: []
depends_on: []          # nothing unmerged (T-070 merged at c7af15f, R-284); ordering only, see "Collisions"
touches:                # one role per path (D-153)
  - Cargo.toml                       # GATE PATH: implementer: a new [profile.dev] table and a new [profile.dev.package."*"] table only (Interface)
  - docs/development.md              # implementer: under "## The gate: `cargo xtask ci`", the paragraph beginning "Measured on the bootstrap machine" and its table (left as they are), and one new paragraph right after that table (D1–D5); nothing else, in particular not the "## CI" section (R-284)
  - xtask/tests/acceptance_t054.rs   # GATE PATH: test author
  - xtask/tests/guard_t054.rs        # GATE PATH: test author (file and line in a backtrace: holds on base and after)
budget_tokens: 1100000  # high (v1.4); a small change, expected far below the ceiling
iteration: 2
found_by: planner       # D-144 change 14 (retro-2s1 proposal 14); D-097, D-182 (the disk filled)
origin_task: null
---

# T-054 A lighter debug build profile

## Goal
The session's writable disk is about 40 GB. A worktree with the windowing crate built takes 12 GB
after the gate, and the base build a join makes in the tests worktree takes 7.9 GB. So two full
builds do not fit side by side, and the disk filled while three builders compiled (D-182). After
this task every debug and test build of the workspace is made with less debug info and no
incremental cache. That covers the gate's `clippy`, `schema`, `scorecard` and `test` steps, a
worktree build, the join and the queue. The gate gives **the same result** (same steps, same tests
passing). The build tree shrinks to about a third: measured 7.6 GB → 2.6 GB for
`cargo test --workspace --no-run` (Spec check R-270). A panic still names its file and line, and
`RUST_BACKTRACE=1` still shows file and line for the workspace's frames. Build time was not shown
to change by more than the noise of a shared machine (no time claim).

Gate path: the root `Cargo.toml`'s `[profile.*]` tables (`xtask/src/precheck.rs`, `gate_tables`:
`[workspace.lints]`, `[workspace.metadata]` and every `[profile.*]`), and `xtask/tests/`. The
approval row must begin `Approve T-054` and contain the words `gate paths`. It needs no
`dependency versions`: no dependency table changes, and `Cargo.lock` does not change
(`dependency_tables` does not include `[profile.*]`). Independent test author (high tier).

Reading list: `docs/development.md` ("The gate", "Task worktrees", "Merge queue", the gate-path
row); `xtask/src/precheck.rs` (`is_gate_path`, `gate_tables`, `dependency_tables`); `xtask/src/seed.rs`
(how a worktree is seeded from main's `target/`); the Cargo book, "Profiles" (`debug`,
`incremental`, profile overrides, and `test` inheriting from `dev`).

## Interface
The root `Cargo.toml` gains exactly these tables. Today it has no `[profile.*]` table (main
`78e223b`). Nothing else in the file changes:

```toml
# T-054 (D-182): smaller debug builds. Line tables keep file and line in panics and backtraces;
# dependencies carry no debug info; no incremental cache. Release builds are unchanged.
[profile.dev]
debug = "line-tables-only"
incremental = false

[profile.dev.package."*"]
debug = false
```

That is option C of Q-1, measured as C6 below. `[profile.test]` inherits from `dev`, overrides
included (Cargo's profile rules; the measurements ran `cargo test`, so they show it).
`[profile.release]` and `[profile.bench]` are not written. Environment variables
(`CARGO_PROFILE_DEV_*`, `CARGO_INCREMENTAL`) still override the file for whoever sets them; no
tool of this repository sets them. `xtask` is a workspace member built with the dev profile, so it
gets the same settings.

What does not change: the gate's steps and their order, which tests exist and pass, the scorecard,
release builds, `Cargo.lock`, `rust-toolchain.toml`, any tool configuration file, and the queue's
preconditions (`gate-path` now applies to this task, as it does to any `[profile.*]` change).

### Rust API used by tests
None of the product. The tests are in `xtask/tests/` and read files and the test binary's own
backtrace with the standard library (`std::backtrace::Backtrace`) and `toml` (already an `xtask`
dependency). No new dependency.

## Pinned
- (a) Errors: n/a (no behaviour of the product changes).
- (b) Files: only the root `Cargo.toml` (the two tables above) and `docs/development.md` (the
  sentences of (i)). The first build after the merge rebuilds everything once, because every
  unit's fingerprint changes. The queue's worktree seeded from main's `target/` (`seed.rs`) then
  costs one full build, measured below at 62–117 s for `cargo test --workspace --no-run` on this
  machine under load. An existing `target/` keeps its old artifacts until `cargo clean`; the size
  figures are for an empty `target/`.
- (c) Order: n/a.
- (d) Platforms (ADR-0009): the settings are Cargo's own and apply on Linux, macOS and Windows.
  The figures are Linux only, on this machine. On macOS (`split-debuginfo` defaults to `unpacked`)
  and Windows (MSVC, `.pdb` files) sizes differ. `Cargo.toml` is a platform-sensitive path, so
  once T-070 counts, the rebased head goes to `ci/T-054` before the queue. The guard of AC-2 runs
  on all three there.
- (e) Threat model (gate path): the change can make the gate differ only through compiled
  artifacts. Debug assertions, overflow checks and `opt-level` stay at the dev defaults (not
  written), so no test's behaviour depends on the change. The reviewer checks the gate's test
  count is equal on base and branch (AC-1). No step reads debug info. The stale-binary guard and
  `testbins.rs` locate binaries by path and Cargo's JSON messages, not by debug info.
- (f) Tests that pin what this changes: none on main reads `[profile.*]`. `xtask/src/queue.rs`'s
  unit tests use their own manifest texts. Not to be pinned: wall times, sizes in a test (sizes
  are measured by people, AC-3), the text of a backtrace beyond file name and line.
- (g) Worked examples (measurements, Spec check R-270): see the table there. Backtrace with C6:
  `panicked at crates/kiln-commands/tests/probe_panic.rs:3:51:` and frame
  `3: unwrap<alloc::vec::Vec<u8, alloc::alloc::Global>, serde_json::error::Error>` /
  `at ./tests/probe_panic.rs:3:51` / `at ./tests/probe_panic.rs:2:18`. With `debug = false` for
  the workspace (D0): the `panicked at …:3:51` line stays, the `at` lines are gone. So
  `debug = false` for workspace crates is not acceptable, and line tables are.
- (h) Evaluator calls: n/a (no scorecard item).
- (i) Sentences for `docs/development.md`, worded for copying, with their evidence:
  - D1 "Debug and test builds carry line tables only for the workspace's crates and no debug
    info for dependencies, and keep no incremental cache (root `Cargo.toml`, `[profile.dev]`,
    T-054). A panic names its file and line, and `RUST_BACKTRACE=1` shows file and line for
    frames in the workspace's code. A debugger shows lines but not variables. Frames inside
    dependencies can lack file and line. Release builds are unchanged." Evidence: (g); AC-2.
  - D2 "Measured on the bootstrap machine (4 cores, Linux), from an empty `target/`:
    `cargo test --workspace --no-run --locked` leaves 2.6 GB with these settings, against 7.6 GB
    with Cargo's defaults (main `78e223b`, with the windowing crate)." Updated by R-284 to: "…leaves
    2.6 GB with these settings, against 7.8 GB with Cargo's defaults (main `cbb54a2`)." Evidence:
    R-284 (the tables written into the manifest of a copy of main `cbb54a2`, built without the
    interim variables: 2.6 GB, against 7.8 GB with no `[profile.*]` table); R-270 measured 2.6
    against 7.6 GB at `78e223b`. The implementer re-measures on the joined branch and copies its
    own figures if they differ (AC-3).
  - D5 (new, R-284): "Figures recorded before T-054 are records of their runs, made with Cargo's
    default debug info: the bootstrap table above, the CI table under "Runners, wall times and
    disk" (run 43752760283), and the 12 GB of a worktree after the gate (D-182). Both CI workflows
    already set `CARGO_INCREMENTAL=0`." Evidence: `docs/development.md` as it stands at
    `cbb54a2`; `.github/workflows/ci.yml` line 22, `platforms.yml` line 47. With D5, no sentence of
    the file becomes untrue: each figure in it names its run or its machine.
  - D3 "A rebuild after a one-line change is slower without the incremental cache: 14.8–23.2 s
    against 10.8–12.2 s for the same edit in `kiln-commands` (one measurement each, on a shared
    machine)." Evidence: R-270, rebuild runs. No time bound is stated.
  - D4 (replacing the cold-build row's date context, not its figures): "The gate's figures above
    are from the bootstrap; the build tree after a whole gate run in a worktree is <figure the
    implementer measures> (12 GB before T-054, D-182)." To be measured by the implementer (AC-3);
    it is not a guarantee.

## Acceptance criteria
- AC-1 (same gate result): `cargo xtask ci` is green on the joined branch. The implementer runs it,
  then the queue. Its `test` step reports the same number of passed tests as on the base (the
  implementer records both counts in its log; the reviewer compares them). Oracle: the gate's own
  report, compared with the base.
- AC-2 (file and line kept) — `guard_t054.rs`: a test captures a backtrace in its own body
  (`std::backtrace::Backtrace::force_capture()`) and asserts that its text names this test's
  source file with a line number. A second test catches a panic it raises, through
  `std::panic::catch_unwind` with a hook that records `PanicHookInfo::location()`, and asserts the
  location's file is this file and its line is the panicking line. Both pass on base (full debug
  info) and on the branch. With `debug = false` for the workspace the first one fails (measured
  as D0: no `at` lines), so it guards against a later, lighter setting. No wall time, no size.
  Oracle: the test's own `file!()` and `line!()`.
- AC-3 (smaller build tree) — measured, not tested: the implementer runs
  `cargo test --workspace --no-run --locked` from an empty `target/` in its worktree, records
  `du -sh target` before and after the change in its log, then deletes `target/`. The branch's
  figure is at most 40% of the base's (measured: 2.6 of 7.6 GB, 34%, at `78e223b`; 2.6 of 7.8 GB,
  33%, at `cbb54a2`, R-284). The implementer also records
  `du -sh target` after its last `cargo xtask ci` (D4). The reviewer repeats the first measurement
  once. Each measurement starts only with 14 GB free (`df -h /home/user`), one at a time.
  Oracle: `du`.
- AC-4 (the change is exactly the two tables) — `acceptance_t054.rs`: the test parses the root
  `Cargo.toml` with `toml` and asserts: `profile.dev.debug` is `"line-tables-only"`,
  `profile.dev.incremental` is `false`, and `profile.dev.package."*".debug` is `false`. It also
  asserts that `profile.release` and `profile.bench` are absent, and that `profile.dev` sets no
  `opt-level`, `debug-assertions` or `overflow-checks`. Fails on base: no `[profile.dev]` table.
  Oracle: the manifest. (A test may read this one manifest; no drafted task changes the profile
  tables. If one does, it changes this test in the same task.)

## Out of scope and implementer notes
- Out: `split-debuginfo` (not measured; on Linux the default `off` is what was measured); a
  separate gate-only profile or `CARGO_INCREMENTAL=0` set by `xtask` (an `xtask/src` change:
  Q-2 B); `[profile.release]`; build caches (`sccache`); a shared `target/` between worktrees
  (forbidden, D-019); `cargo clean` of existing worktrees (the director's, after the merge).
- Do not set `CARGO_TARGET_DIR` (AGENTS.md). Measure in the worktree.
- **The interim environment (D-186) is not used by this task's builders.** `CARGO_PROFILE_DEV_DEBUG`
  and `CARGO_INCREMENTAL` override the manifest, so a build made with them measures the
  variables, not the tables. The implementer, the test author and the reviewer build, test, run
  the gate and measure with both unset (`env -u CARGO_PROFILE_DEV_DEBUG -u CARGO_INCREMENTAL …`).
  The director's dispatch message says so, and the director's join and queue run of this task run
  without them too. After the merge the director drops the interim variables for every later build
  (a decision row ends D-186's interim). The workflows' own `CARGO_INCREMENTAL=0` stays: it agrees
  with the manifest.
- Disk: one measurement at a time; delete each `target/` you measure.

### Collisions and order
- **T-070** (merged at `c7af15f`; this item kept for the record): edited `docs/development.md`'s "CI" section and adds `.gitattributes`, a
  workflow and `xtask/tests/acceptance_t070.rs`. It shares `docs/development.md` with this task
  (§5.2 compares paths, not sections), so the two are **not in flight together**: T-054 is
  dispatched after T-070 merges, or `docs/development.md` leaves this task's `touches` (Q-3).
- **T-064**, **T-052**, **T-071**, **T-077**, **T-078** (drafts) also list `docs/development.md`:
  one at a time with T-054.
- **T-065** (merged) edited the root `Cargo.toml`'s dependency table. **T-073** (draft) adds a
  dependency in `crates/kiln-workspace/Cargo.toml` and `Cargo.lock`, not the root manifest, so it
  does not overlap by path. Any later task that edits the root `Cargo.toml` (a
  `[workspace.dependencies]` entry) overlaps by path and is not in flight with T-054.
- The first queue run after T-054 merges rebuilds main's `target/` from scratch. Queue the next
  task only with the disk rule (14 GB free) in mind. In CI, the cache action's key covers the
  manifests, so the first run after the merge also builds cold (one longer run; nothing to change).
- **T-058** (in review): touches `crates/kiln-ui/`, `apps/kiln/tests/` and two protocol documents.
  There is no shared path, so it may be in flight beside T-054. Its join and queue run are built
  under the interim variables until T-054 merges.
- **T-071** (draft, platform-test rule): its `touches` list `xtask/src/` files and
  `docs/development.md`. It shares `docs/development.md`, so it is **not in flight with T-054**;
  either order works.

## Questions and rulings
Questions for the director, before dispatch:
- **Q-1. Which setting.** All options were measured from an empty `target/` with
  `cargo test --workspace --no-run --locked` (R-270):
  - A: `debug = "line-tables-only"` only. 4.5 GB.
  - B: A plus `incremental = false`. 3.5 GB.
  - C: B plus no debug info for dependencies. 2.6 GB. This is the Interface.
  - Panic file and line and the workspace's backtrace lines are kept in A, B and C (C shown
    directly; A shares C's workspace setting). The cost of B and C is a slower rebuild after a
    small edit (D3) and, for C, dependency frames that can lack lines.
  - **Recommendation: C.** Disk is the binding limit (D-182), and the extra rebuild time is
    seconds.
- **Q-2. Incremental for builders, off for the gate only?**
  - A: off for every debug build (as specified; measured).
  - B: on in the manifest, and the gate, join and queue set `CARGO_INCREMENTAL=0`. This changes
    `xtask/src` and is unmeasured: the probe it needs is a gate run in a worktree under B,
    comparing `target/` with A's.
  - **Recommendation: A.**
- **Q-3. Waiting for T-070.**
  - A: dispatch after T-070 merges (as specified).
  - B: drop `docs/development.md` from `touches`, put D1–D3 in a comment above the tables in
    `Cargo.toml`, and dispatch now. The figures would then live in a gate path, and the
    development document would say nothing.
  - **Recommendation: A**, unless the disk blocks T-070's own join first. In that case B, with
    D1–D3 moved into `docs/development.md` by the next task that edits it.

### Spec check R-270
Planner run R-270 (booked to `iter-2`), at main `78e223b371366d1dcec20c2c8b2025b026e1336c` (ledger
commits since R-262; nothing merged). The stub (R-225) had touches, a goal and open questions.

Changed against the stub:
- The setting is chosen and measured (Q-1 C). The stub's two candidates are both measured, plus
  incremental off.
- `touches` gained the two test files (test author). `[profile.test]` is not needed (inherits).
- The figures and the order after T-070 are new.
- Budget kept at the high ceiling.
- The stub's question about the queue's seeding is answered in Pinned (b).

Evidence (a copy of the workspace under the session's scratchpad, without `target/`, `.git`,
`.worktrees`; settings passed as `cargo --config` or environment variables, the manifest
unchanged; each `target/` deleted before the next build; `df -h /home/user` showed 15 GB, then
14 GB free before every build; other builders were using the machine, so the times are noisy):

| Run | Setting | `target/` after `cargo test --workspace --no-run --locked` | wall |
|---|---|---|---|
| B0 | Cargo defaults (today) | **7.6 GB** (`deps` 6.1 G, `incremental` 1.4 G, `build` 180 M) | 87.9 s |
| C1 | `debug = "line-tables-only"` | 4.5 GB (`deps` 3.3 G, `incremental` 982 M) | 87.9 s; 118.4 s on a second run |
| C3 | C1 + `incremental = false` | 3.5 GB (`deps` 3.3 G) | 62.0 s; 102.6 s |
| C2 | dependencies `debug = false` only | 4.3 GB | 94.1 s |
| C5 | `incremental = false` only | 6.2 GB | 153.0 s |
| **C6** | C3 + dependencies `debug = false` (**Interface**) | **2.6 GB** | 117.0 s |

- Rebuild after appending a comment to `crates/kiln-commands/src/doc.rs`, then
  `cargo test --workspace --no-run`: C1 (incremental on) 12.21 s and 10.78 s; C3 (off) 23.20 s and
  14.78 s.
- Panic and backtrace (`RUST_BACKTRACE=1`, a test that unwraps a failing `serde_json::from_str`):
  - B0, C3 and C6 print `panicked at …/probe_panic.rs:3:51:` and frames with
    `at ./tests/probe_panic.rs:3:51`. With line tables a generic frame is named
    `unwrap<…>` instead of `<core::result::Result<…>>::unwrap`.
  - D0 (`debug = false` for everything) keeps the `panicked at` line and loses every `at` line.
  - A panic inside `ropey` (`Rope::char` out of range) under C3 and C6 named
    `ropey-1.6.1/src/rope.rs:803:13` in both, because that function was compiled into the calling
    test. That frames compiled inside a dependency lose their lines under C6 is Cargo's documented
    effect, not shown by this probe (D1 says "can").
- Not measured: a whole `cargo xtask ci` run (it needs a git checkout, and a baseline gate run
  needs about 12 GB, D-182, beside a review using 12 GB). The implementer measures it under the
  disk rule (AC-3, D4). Also not measured: `split-debuginfo`, macOS, Windows, release builds, and
  Q-2 B.
- Gate-path and dependency rules read in `xtask/src/precheck.rs`: `is_gate_path` (lines 88–94),
  `gate_tables` (lines 98–110: `profile`), `dependency_tables` (lines 189–231: no `profile`).

### Rulings on spec R-270 (director, D-186)
- **Q-1: C.** Line tables only for the workspace's crates, no debug info for dependencies, no incremental cache
  (measured: the test build goes from 7.6 GB to 2.6 GB; a panic still names file and line, and a backtrace still
  shows the workspace's frames with their lines).
- **Q-2: A.** Incremental compilation is off for every debug build; setting it only for the gate through `xtask`
  was not measured and is not taken.
- **Q-3: A.** The task is dispatched after T-070 has merged (both edit `docs/development.md`).
- **Until then, an interim that changes no file:** the director runs joins and queue runs, and builders build,
  with `CARGO_PROFILE_DEV_DEBUG=line-tables-only` and `CARGO_INCREMENTAL=0` in the environment (the planner's
  second-last row: 3.5 GB against 7.6 GB). CI keeps Cargo's defaults until this task merges; the profile changes no
  test result.

### Spec check R-284
Planner run R-284 (booked to `iter-2`, resumed), at main `cbb54a2628afbd368087e82c5d7925eaa5b08625`
(T-070 merged at `c7af15f`, T-057 at `7e8a01c`).

1. `docs/development.md`: the passage named by line numbers is re-anchored by its heading and
   text. T-070 added "## CI" with "Runners, wall times and disk", whose `du -sh target` figures
   (`gate-linux` 8.6G, `gate-macos` 4.4G, `gate-windows` 2.5G) are introduced as what "Run
   43752760283 … showed". They are records of a named run and stay true. So does the bootstrap
   table ("Measured on the bootstrap machine"). So does "A seed costs roughly main's dependency
   artifacts" under "Task worktrees": it gives no figure. No sentence of the file becomes untrue.
   New sentence D5 says the earlier figures predate T-054, in this task's own paragraph, so
   neither "## CI" nor any other task's section is edited. `touches` names exactly that paragraph.
2. Workflows (gate paths, not in `touches`; nothing to change):
   - `ci.yml` (job env, line 22) and `platforms.yml` (workflow env, line 47) set
     `CARGO_INCREMENTAL: "0"`, which agrees with the manifest. Neither sets
     `CARGO_PROFILE_*` or a debug level.
   - `window-linux` and `window-probe` run `cargo build -p kiln --locked` and use
     `target/debug/kiln`. The dev profile's directory stays `debug`, so
     `xtask/tests/acceptance_t070.rs` `ac1_window_jobs_build_the_profile_whose_binary_they_run`
     (which reads the workflow for `--release`/`--profile` and env keys) and its env check of
     `CARGO_TERM_COLOR`/`CARGO_INCREMENTAL` (line 1655) read only the workflow and stay green. No
     T-070 test reads the root `Cargo.toml` (`grep` for `profile`, `debug`, `CARGO_`).
   - The cache action's key covers the manifests: one cold run after the merge.
   - D-186's line "CI keeps Cargo's defaults until this task merges" is true for debug info only:
     CI already builds without the incremental cache.
3. Platforms (`platforms.yml` runs the gate on macOS and Windows, and `xtask`'s tests also in the
   core-crates step, on Windows the only test evidence until T-073):
   - `acceptance_t054.rs` (AC-4) reads one file with `std::fs` and `toml`: no Unix-only API. With
     `.gitattributes` `* -text` the checkout is byte-identical on all three. Runs on all three.
   - `guard_t054.rs` AC-2: the panic-location test uses only `std::panic` (the location is
     compiled in, independent of debug info: shown in R-270 D0). Runs on all three.
   - `guard_t054.rs` AC-2: the backtrace test uses `std::backtrace`, no Unix-only API. Whether a
     backtrace names file and line with line tables on macOS (debug info left in object files,
     `split-debuginfo = "unpacked"` by default there) and on Windows (MSVC, a `.pdb`) was **not
     measured** (no such machine here). This test must not add a failure of its own on those runners
     (Q-4).
4. The interim environment: builders of this task build without it (implementer notes); the
   director ends it by a decision row after the merge.
5. Other changes on main:
   - `xtask/src/precheck.rs` `gate_tables` still includes `profile` (lines 96–110 at `cbb54a2`),
     so tier high and the approval words `gate paths` stand.
   - The merged T-057 tests made the default test build larger. Re-measured, one build at a time
     (20 GB free before each, in a copy under `/home/user`, deleted after):
     - `cargo test --workspace --no-run --locked` with no `[profile.*]` table and the interim
       variables unset: **7.8 GB, 125.4 s**;
     - with the two tables of the Interface written into the copy's `Cargo.toml`: **2.6 GB,
       88.0 s**.
   - That confirms the tables themselves, not only the `--config` form R-270 used. D2 and AC-3's
     figures are updated.

New question:
- **Q-4. The backtrace test on macOS and Windows.**
  - A: before the queue, the director pushes the candidate to `ci/T-054` (already required:
    `Cargo.toml` and `xtask/` are platform-sensitive) and records, per platform, whether
    `guard_t054` passed. A platform where a backtrace lacks file and line is a platform difference:
    the test asserts that platform's behaviour, D1 gets one sentence for it (ADR-0009 Decision 4),
    and that is a ruling before the queue.
  - B: drop the backtrace assertion and keep only the panic-location test (weaker: it would not
    catch a later `debug = false`).
  - **Recommendation: A.**

Not checked: macOS and Windows (Q-4); a whole `cargo xtask ci` under the new profile (still the
implementer's, AC-3/D4); the rebuild-time figures of D3 on current main (not re-measured: they are
worded as one measurement each).

### Rulings on spec check R-284 (director, D-200)
- **Q-4: A.** Before the queue the director pushes the joined candidate to `ci/T-054` and records per platform
  whether `guard_t054` passed. A platform whose backtrace lacks file and line becomes a stated difference by a
  ruling before the queue.
- Builders, reviewer, join and queue of this task run with `CARGO_PROFILE_DEV_DEBUG` and `CARGO_INCREMENTAL`
  unset; the director ends the interim of D-186 with a decision row after the merge.
- Not in flight with T-071 (`docs/development.md`); beside T-058 is allowed.

### Ruling on the implementer's report (director, D-203)
- **i1.** Sentence D3 of the paragraph in `docs/development.md` states the implementer's own figures of run R-286
  (rebuilds 14.4 to 24.2 s without the incremental cache against 7.3 to 9.6 s with it, two runs each), not the
  planner's figures of R-270: a figure in a document comes from a command of the run that writes it. The spec's
  D3 wording "one measurement each" is read as this.
- The 14 GB free before a measured build (AC-3) is the rule; the dispatch's 9 GB was the director's slip.

### Rulings on review round 1 (director, D-208; `reviews/T-054-r1.md` at `500da2a05c384cf05c0da7c39f5c5ef416d725c6`)
- **j1 (findings 1 and 2: a spec gap, closed here; test author).** AC-4 is read as: **the `[profile.*]` tables of
  the root manifest are exactly the Interface's.** There is no other profile table (`test`, `release`, `bench` or
  any other name); `[profile.dev]` holds only the keys `debug`, `incremental` and `package`; `package` holds only
  `"*"`; `package."*"` holds only `debug`; there is no `build-override`. `acceptance_t054.rs` asserts this; each
  of the reviewer's four surviving mutants (a `[profile.test]` table with `debug = true`, `split-debuginfo`,
  `codegen-units`, `build-override`) fails a test. The tests still fail on base (no `[profile.dev]` there). No
  change to `Cargo.toml` or the document.
- **j2 (finding 3, nit).** No change: with line tables an inlined frame loses its module path; file and line stay.
- Known consequence accepted with the task (the reviewer's runs): non-generic dependency frames in a backtrace
  have no file and line, and a debugger shows lines but no locals. A developer who needs them overrides the
  profile for a run (`--config profile.dev.debug=true`).

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
- 2026-11-16 **CI round 1 (ruling Q-4 A)**: run 43775682214 of `platforms.yml` on `ci/T-054` at the joined head
  `213df3e048dd436f34e2eb6d4652eabc471817cc` (pushed 2026-11-15T23:36Z; extraction `tasks/ci-runs/T-054-round-1.md`,
  the job logs by a helper, the test verdicts from the uploaded artefacts by the planner, run R-294). **Linux and
  macOS: `guard_t054` 2 passed (`ac2_backtrace_names_this_file_and_line ... ok`,
  `ac2_panic_location_is_this_file_and_line ... ok`) and `acceptance_t054` 5 passed.** Windows: no verdict, because
  no `xtask` test binary compiles there until T-071 (`could not compile xtask (test "guard_t005")`); the Windows
  verdict of the backtrace guard is read on T-071's CI round (noted there), and a difference found then is ruled
  then. Jobs: `gate-linux` success (`GATE PASSED in 462.9s`, 1025 tests; `du -sh target` 4.2G against 8.6G in
  run 43765532298); `window-linux` and `window-probe (macos-15)` success (the two counted jobs); the macOS and
  Windows gates red as on main; macOS totals 1,003 passed and 22 failed, the same 22 names as before the change.

### planner
- 2026-11-14 · R-225 · (new) · did: filed as a draft for ADR-0007 (D-144 change 14). · gate attempts: 0 · missing context: the exact setting D-097 measured · next: director.
- 2026-11-15 · R-270 · draft · did: measured six settings in a scratch copy, one at a time under the 14 GB rule; specified option C (line tables, no dependency debug info, no incremental: 7.6 → 2.6 GB for the test build); tier high (gate path `[profile.*]`), approval row `Approve T-054 … gate paths`, no `dependency versions`; 4 criteria, two of them tests in `xtask/tests/` (one fails on base, one guard); ordered after T-070 (shared `docs/development.md`). · gate attempts: 0 (scratch builds only) · not run: a whole gate run, macOS, Windows · missing context: none · next: director (Q-1 to Q-3).
- 2026-11-15 · R-284 · draft · did: spec check against main `cbb54a2` (see "Spec check R-284"): `docs/development.md` passage re-anchored, D5 added so no figure of the file becomes untrue; workflows need no change (`CARGO_INCREMENTAL=0` already, `target/debug` kept, `acceptance_t070` stays green); per-test platform statement, Q-4 for the backtrace test; builders build without the interim variables; re-measured 7.8 → 2.6 GB with the tables in the manifest. · gate attempts: 0 · not run: macOS, Windows, a whole gate · missing context: none · next: director (Q-4).
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
