---
id: T-071
title: "Make no-skip and the scorecard step follow the platform-test rule: a registry of tests that exist only on some platforms, each with its counterpart or the reason there is none; no helper that does nothing on one platform; the gate's own tests compile and pass on all three"
tier: high              # METHODOLOGY v1.4 §4: gate paths (xtask/); the approval row must contain `gate paths`. No dependency change.
state: queued
owner_role: planner
scorecard_items: []     # none claimed; the tests it marks stay tied to their items
depends_on: [T-070]     # its Windows evidence is T-070's gate-windows core-crates step; and both edit docs/development.md (§5.2)
touches:                # one role per path (D-153). All but the last two: implementer. Product test files: T-079 (ruling Q-1 B, D-180)
  - .github/workflows/platforms.yml          # GATE PATH: implementer: `--no-fail-fast` on the core-crates step's per-package `cargo test`, in the three gate jobs; nothing else (ruling s11, D-226)
  - xtask/src/platforms.rs                  # GATE PATH: new: the registry's reader and rules (shared by no-skip and scorecard)
  - xtask/src/noskip.rs                     # GATE PATH
  - xtask/src/scorecard.rs                  # GATE PATH
  - xtask/src/main.rs                       # GATE PATH: only the `mod platforms;` line and the write-mode refusal of Interface (3)
  - xtask/platform-tests.toml               # GATE PATH: new: the registry with every entry (ruling Q-8 A, D-175)
  - xtask/fixtures/no-skip/                 # GATE PATH: the self-test cases of the new findings
  - xtask/src/hygiene.rs                    # GATE PATH: its #[cfg(test)] module only (Unix symlink calls, lines 540–541)
  - xtask/src/publish.rs                    # GATE PATH: its #[cfg(test)] module only (line 1340)
  - xtask/tests/acceptance_t005_r3.rs       # GATE PATH: the from_mode helper only (T-043 edits the `refused` helper: not in flight together)
  - xtask/tests/acceptance_t005_r4.rs       # GATE PATH: as above
  - xtask/tests/acceptance_t013_hygiene.rs  # GATE PATH
  - xtask/tests/guard_t005.rs               # GATE PATH
  - xtask/tests/acceptance_t005.rs          # GATE PATH: implementer: the one line that writes the outside dependency's path into a toy Cargo.toml (ruling s14, D-231)
  - xtask/src/queue.rs                     # GATE PATH: its #[cfg(test)] module only (from line 803): two tests fail on macOS (R-285)
  - xtask/src/worktree.rs                  # GATE PATH: its #[cfg(test)] module only (from line 228): one test fails on macOS (R-285)
  - xtask/src/join.rs                      # GATE PATH: its #[cfg(test)] module only: the toy repositories' git settings (ruling s13, D-228)
  - xtask/tests/acceptance_t013_publish.rs # GATE PATH: reads /proc (lines 942, 994); fails on macOS (R-285)
  - crates/kiln-workspace/tests/acceptance_t002.rs        # the two inert helpers (lines 169–187) and the test that uses them (line 924): this task's own rule fails on them, so it fixes them
  - docs/development.md                     # implementer: the `no-skip` and `scorecard` rows of "The gate" and a paragraph on the registry
  - xtask/tests/acceptance_t071.rs          # GATE PATH: test author
  - xtask/tests/acceptance_t070.rs          # GATE PATH: test author: the one expectation on the core-crates `cargo test` command, only if `--no-fail-fast` breaks it (ruling s11, D-226)
  - xtask/tests/guard_t071.rs               # GATE PATH: test author (today's no-skip findings that must stay)
budget_tokens: 1100000  # high (v1.4); estimate about 800,000 after the split (R-285)
iteration: 2
found_by: planner       # ADR-0009 (R-255)
origin_task: null
---

# T-071 The platform-test rule

## Goal
METHODOLOGY v1.4 §8.1 "Tests" replaces D-166 ruling b8 (a Unix-only test file accepted "since the gate
and CI run on Linux"): every test runs on every supported platform and asserts each platform's
specified behaviour; a test of a facility one platform lacks exists only where it has it, listed in a
platform-test registry with its sentence and its counterpart (or "no counterpart" and why); `no-skip`
fails an unlisted platform-conditional test, a missing counterpart and a helper that does nothing on
one platform; a test that cannot run on a platform fails there, naming why; the scorecard page is
computed on Linux. Today `no-skip` refuses *any* `cfg` other than `cfg(test)` on a test or test module
(`xtask/src/noskip.rs:9-10,81-90`), but does not see a `cfg` on a helper: `acceptance_t002.rs:176-187`
gives non-Unix `mode_and_inode` the body `None` and `set_mode` an empty body, and the test using them
checks the mode only `if let (Some(..), Some(..))` (line 924), so off Unix it passes without checking
(read on main 15555cc).

After this task:
1. the registry `xtask/platform-tests.toml` exists and holds every entry the current tests need;
2. `no-skip` applies the rule (Interface (2)); the scorecard step behaves per platform (Interface (3));
3. `xtask`'s own tests compile for Windows (today 4 errors in `xtask/src/hygiene.rs:540-541` and
   `xtask/src/publish.rs:1340-1341`, plus `acceptance_t005_r3.rs`, `_r4.rs`, `acceptance_t013_hygiene.rs`,
   `guard_t005.rs`; CI run 43765532298 `gate-windows`, R-255 probe a4) and pass on macOS and Windows
   (today 6 fail on macOS, Interface (4)); seen in the `core-crates` step of `platforms.yml`;
4. the product test files are **not** this task's: their marking and entries are T-079 (D-180 Q-1 B).
Gate paths: everything under `xtask/` in `touches`.

Reading list: METHODOLOGY v1.4 §8.1, §6; ADR-0009 Decision 4; `xtask/src/noskip.rs`, `scorecard.rs`,
`testbins.rs`; `xtask/skips.toml` (the format this registry imitates); `docs/development.md` "The gate"
(`no-skip`, `scorecard` rows) and "Scorecard"; ADR-0009 probe a4 and `tasks/T-070-*.md` Interface (1).

## Interface

### (1) The registry `xtask/platform-tests.toml`
TOML, parsed strictly (unknown keys, an empty `reason` or `sentence`, a malformed date are errors of
the `no-skip` step, as for `skips.toml`). Zero or more entries:
```toml
[[test]]
file = "crates/kiln-workspace/tests/acceptance_t008.rs"   # relative to the repository root, `/`-separated
item = "a_fifo_test"     # (illustrative name) module path in the file, then the fn or mod name
                                                           # (as skips.toml); "<file>" for an inner #![cfg(..)]
platforms = "unix"            # the cfg the item is behind: "unix" | "windows" | "linux" | "macos"
doc = "docs/protocol/paths.md"                             # the document whose sentence the item tests
quote = "Directories, FIFOs, sockets and devices are refused"   # a substring that occurs in `doc`
counterpart = "pending"       # "<file>::<item>" | "none" | "pending"
reason = "…"                  # required for "none" and "pending"
task = "T-075"                # required for "pending": the task that writes the counterpart
expires = "2027-01-31"        # required for "pending" (YYYY-MM-DD; today = HEAD's commit date, as no-skip)
```
Platform names: `unix` = Linux and macOS; `windows`; `linux`; `macos`. The cfg forms that mean them:
`cfg(unix)` or `cfg(not(windows))`; `cfg(windows)` or `cfg(not(unix))`; `cfg(target_os = "linux")`;
`cfg(target_os = "macos")`. Any other cfg on a test stays a `cfg-disabled` finding as today.

### (2) What `no-skip` checks (new findings; each a FAIL line naming `<file>:<line>` and the item)
- `platform-unlisted`: a test fn, a module containing tests, or a file's inner cfg, behind one of the
  cfg forms above, with no registry entry for that `file` and `item`.
- `platform-mismatch`: an entry whose `platforms` is not the item's cfg.
- `platform-unused`: an entry that matches no item (as an unused skip).
- `platform-sentence`: `doc` does not exist or does not contain `quote`.
- `platform-counterpart`: `counterpart` names a file or item that does not exist as a test fn or a
  module containing tests, or one that is itself behind a cfg that does not cover every supported
  platform the entry's `platforms` leaves out (the counterpart of a `unix` item must exist on Windows).
- `platform-pending-expired`: a `pending` entry whose `expires` is before today; and a `pending` entry
  whose `task` file does not exist on main's tree of the scan (`tasks/T-NNN-*.md`).
- `inert-helper`: in a scanned file, a function that is not a test and is defined more than once under
  platform cfgs of the forms above, where one definition's body is trivial: no statement, or one
  expression that is a literal, `None`, `()`, `Ok(())`, `Default::default()`, or a tuple or struct of
  these. (A platform helper with real bodies on each platform is allowed.)
- `platform-statement`: inside a test fn, a statement or expression with a platform cfg attribute and no
  statement with the complementary cfg in the same block (so the test does something on one platform and
  nothing in its place on another). `use` items are exempt. The existing pattern of
  `acceptance_t015_tools.rs:130-133` (`#[cfg(not(unix))] panic!(…)` beside `#[cfg(unix)] {…}`) passes:
  each platform has a statement, and the one that cannot run fails, naming why.
The existing six kinds, `skips.toml`, its expiry rule and the self-test stay; the self-test gains one
fixture case per new kind (the fixture's finding count is stated in the detail line, as today "9 known
findings"). The detail line adds `<n> platform test(s) registered (<p> pending)`.

### (3) The `scorecard` step per platform
- On Linux: as today, with one addition: a listed test absent because its registry entry's `platforms`
  excludes Linux is "registered absent here", not `Missing`; the page is computed and compared.
- On macOS and Windows: every listed test that is built on this platform is run; a failing or ignored
  one is a problem, as today; a listed test absent without a registry entry excluding this platform is a
  problem (`… does not exist on <platform>`); a registered absent one is counted. The page is **not**
  compared and not written. The detail line names the platform, e.g. `windows: 64 items; 410 tests ran;
  37 registered absent here; page not compared (computed on Linux)`.
- `cargo xtask scorecard` (write mode) on macOS or Windows exits 1 with one line
  `scorecard: refused: docs/scorecard.md is computed on Linux` and writes nothing.

### (4) Entries written by this task (the rest are T-079's, D-180 Q-1 B)
Only `xtask`'s own tests. Facts from the full test output of CI run 43765532298 (`gate-macos` artefact
`test-output.txt` and `core-crates.txt`, downloaded in R-285; the task file's director log and
`tasks/ci-runs/T-070-round-2.md` had only the printed prefix):
| Test (file) | macOS failure as printed | Kind | What this task does |
|---|---|---|---|
| `queue::tests::basic_refusals_write_nothing` (`xtask/src/queue.rs:2154`) | "another queue run holds …/kiln-queue.lock (999999999 T-999)" | the queue's lock liveness reads `/proc/<pid>` and keeps any lock where `/proc` is absent (`queue.rs:313-314`) | register `linux`, `counterpart = "none"`, reason "the queue, worktree and publish tools run on the director's Linux machine only (ADR-0009 (c))" — or make portable (Q-5) |
| `queue::tests::lock_rule_is_applied_to_the_rebased_tree` (`queue.rs:931`) | `sed: 1: "Cargo.lock": invalid command code C` | the fixture uses GNU `sed -i` | make portable (no `sed -i`) |
| `worktree::tests::create_seed_list_remove` (`worktree.rs:305`) | `assertion failed: list(root).unwrap().contains("task/T-007")` | temp dir under `/var` → `/private/var` link, INFERRED | as the queue row (Q-5) |
| `acceptance_t005_r3::tampered_cached_gate_binary_does_not_decide_the_gate` (`:336`) | "nothing to tamper with" | the queue's private build tree layout, INFERRED | as the queue row |
| `acceptance_t005_r4::planted_future_dated_xtask_does_not_decide_the_gate` (`:635`) | "round 1: nothing to plant over" | as above | as the queue row |
| `acceptance_t013_publish::publish_refuses_while_the_queue_lock_is_held_and_releases_it_for_the_queue` (`:941`) | `assertion failed: Path::new("/proc")…exists()` | `/proc` | as the queue row |
Windows compile errors (`hygiene.rs`, `publish.rs` test modules; `acceptance_t005_r3/r4`,
`acceptance_t013_hygiene`, `guard_t005`): made portable where the platform has the facility (a symlink by
the platform's call; read-only by `Permissions::set_readonly`), else registered with the queue rows' reason.
How many entries this writes is **to be measured by the implementer**; expected under 15.

### (5) `docs/development.md`
The `no-skip` row states the rule and the new findings; the `scorecard` row states (3); a short
paragraph "Platform tests" states the registry's fields and that only a high-tier task changes it
(`xtask/` is a gate path).

## Pinned
- (a) Findings: the seven new kinds above, reported in scan order after the existing kinds; precedence
  within one item: `platform-unlisted` before the others. Registry parse errors stop the step (FAIL, one
  line naming the entry).
- (b) Files read: `xtask/platform-tests.toml`, `xtask/skips.toml`, the scanned `.rs` files (`crates`,
  `apps`, `xtask`, minus `xtask/fixtures`, as `SCANNED`/`EXCLUDED`, `noskip.rs:471-473`), `tasks/` names,
  each `doc`; written: none (the scorecard page on Linux only, as today).
- (c) Order: `no-skip`'s self-test, then the scan, then `skips.toml`, then the registry rules.
- (d) Platform: the scorecard step's platform is the host it runs on; the rule's platform set is the
  three of §8.1.
- (e) Threat model: a gate step; a builder must not be able to pass a weakened test by registering it:
  every entry needs a sentence that exists and a counterpart that exists or a dated `pending` with a task
  file; the reviewer reads every entry (they are few dozen, to be measured).
- (f) Tests pinning what this changes: `xtask/src/noskip.rs` unit tests and the self-test count "9 known
  findings" (`noskip.rs:492`, `docs/development.md:75`), the scorecard unit tests; the test author lists
  any acceptance test of T-005/T-013 that asserts the `no-skip` detail line.
- (g) Worked example (VERIFIED by reading, main 15555cc): `acceptance_t002.rs` defines `mode_and_inode`
  under `#[cfg(unix)]` (line 170) and `#[cfg(not(unix))]` with body `None` (176–179) → `inert-helper`;
  `set_mode` likewise with `{}` (186–187) → `inert-helper`; `make_symlink` under `unix` and `windows` with
  real bodies (189–196) → no finding. `acceptance_t015_tools.rs:130-133` → no finding.
- (h) Evaluator: none (no item).
- (i) No `docs/protocol/` sentence changes; the `docs/development.md` sentences of (5), evidence AC-1 to AC-4.

## Acceptance criteria
Local, gate tests by the independent test author (`xtask/tests/acceptance_t071.rs`), each driving the built
`xtask no-skip` / `xtask scorecard --check` on toy repositories (as the T-005/T-013 tests do):
- AC-1: for each new kind, a toy repository that has exactly that fault gets exactly one FAIL line of
  that kind naming the file and line; the same repository with the fault corrected passes. Oracle: exact
  kind and location.
- AC-2: a correct registry (one `unix` test with a Windows counterpart; one `pending` with a task file and
  a future date; one `none` with a reason) passes; the detail line counts them.
- AC-3: `inert-helper`: the `acceptance_t002.rs` pattern (twins with `None` / `{}`) fails; twins with real
  bodies pass; a single `#[cfg(unix)]` helper without twin passes.
- AC-4: scorecard on Linux with a listed test registered `windows`-only: not `Missing`, page compared;
  the same toy on a host reported as non-Linux cannot be run locally — see AC-6.
Local, by the implementer and re-run by the reviewer (measurable here, R-255 probe method):
- AC-5: `cargo check -p xtask --all-targets --locked --target x86_64-pc-windows-msvc` in a scratch copy of
  the task head reports 0 errors (today 4 + the four test files; `xtask` needs no C stub: it does not
  depend on `tree-sitter`); the Linux test list of `xtask` (`cargo test -p xtask -- --list`) equals main's
  name for name; `no-skip` on the task head passes on Linux.
Observed only through CI (the director's record; §8.1; `ci/T-071`, D-185; reuse rule D-198):
- AC-6: on the candidate, the `core-crates` step shows `xtask` exit status 0 on `gate-macos` and on
  `gate-windows`; `gate-linux` green. The macOS gate stays red until T-079 (product tests) and Windows
  until T-073, as `tasks/T-070-*.md` "Expected to FAIL" says; the scorecard step on macOS prints the
  platform detail line of Interface (3).

## Out of scope and implementer notes
- The product test files (T-079); Windows counterparts (T-073 to T-076); product code.
- Changing an assertion of a marked test, or marking a test that compiles on Windows.
- A new dependency (none needed: `syn` already parses attributes; `toml` reads the registry).
- `skips.toml` entries for platform reasons: the registry is the only place for them.

## What stays for T-072 to T-078
- T-072: macOS failures found by T-070's runs; any `macos` entry they need (written by T-072, high).
- T-073, T-074: the Windows behaviour and its new tests; they replace `pending` counterparts where their
  tests cover a registered sentence (editing the registry: both are high).
- T-075, T-076 (low, by ruling Q-8 A): write the remaining `pending` counterparts as tests. Note: turning a
  `pending` into a test name edits `xtask/platform-tests.toml`, a gate path — so either the entry is
  written now with the counterpart's future name and `pending` is read as "named, not yet existing" (Q-3
  A), or T-075/T-076 become high. 
- T-077: `window_t065.rs` and `acceptance_t065.rs` counterparts on macOS/Windows.

## Collisions and order
- T-070 first (depends_on; `docs/development.md` shared).
- T-043 lists `xtask/tests/acceptance_t005_r3.rs`, `_r4.rs` and `docs/development.md` → not in flight
  together; order T-071 then T-043, or the reverse (Q-4).
- T-064, T-041: `docs/development.md` → not in flight together.
- T-054 (`Cargo.toml`, `docs/development.md` lines 82–88 and a paragraph): shares `docs/development.md`
  → **not in flight together** under §5.2 (equal path), although the passages differ (Q-6).
- T-057 (merged) and T-058/T-059: their test files (`apps/kiln/tests/acceptance_t057.rs`,
  `project_t057.rs`; on `task/T-058`: `acceptance_t058.rs`, `tabs_t058.rs`) use no Unix-only API
  (`grep` for `std::os::unix`, `cfg(unix)`, `/dev/full`, `raw_os_error`, `from_bytes`: 0 hits, R-285) and are
  not in this `touches`. If T-058 merges while T-071 is in flight, nothing changes for T-071's branch;
  T-071's queue run rebases onto it and its `no-skip` scans T-058's files: they pass unless they add a
  platform cfg (they have none). Whether they pass on macOS is T-079's or T-058's concern, not this task's.
- T-079 depends on this task; T-072–T-078 follow.
- T-066, T-068 (kiln-ui): no shared path.
- T-072–T-078 depend on this task.

## Estimate
After the split (D-180 Q-1 B): the tool, the registry, `xtask`'s own 9 test files and modules and `acceptance_t002.rs`.
xtask tool tasks: T-070 809,362 booked (D-197), stage-1 high p75 866,379. **About 800,000** of the high
ceiling 1,100,000; arm: high → independent test author.

## Questions and rulings
- **Q-1 (planner, R-261). Size.** A: one task, an `Extend T-071 to 1400000` row at dispatch. B: split:
  T-071 the tool, the registry format and `xtask`'s own tests (about 800,000); a new high task for the
  marking and entries of the 28 product test files (about 500,000), which keeps ruling Q-8 A's intent
  (entries written by a high task before T-075/T-076). **Recommendation: B** — each fits its ceiling, and
  the tool's review is not diluted by 28 mechanical files; the cost is one more spec check and join.
- **Q-2 (planner, R-261). Expiry of `pending`.** A: a date about six weeks out (2026-12-31). B: no expiry,
  only the `task` file. **Recommendation: A** — skips expire for the same reason; the date is moved by a
  ruling if T-075/T-076 slip.
- **Q-3 (planner, R-261). How a `pending` becomes a counterpart without a gate-path edit by a low task.**
  A: the entry names the counterpart's future `file::item` now with `pending`; `no-skip` treats a pending
  entry whose named counterpart exists as satisfied, and the stale `pending` word is removed by the next
  high task touching the registry. B: T-075 and T-076 become high. C: move the registry out of `xtask/`.
  **Recommendation: A** — unmeasured cost is nil; it needs the counterpart names fixed now, which the
  implementer can do per test.
- **Q-4 (planner, R-261). T-043 order.** A: T-071 before T-043 (ADR-0009 Q-4 order). B: T-043 first (it
  is a director-tool fix the record has needed twice, D-174, D-179). **Recommendation: B** if T-043 is
  ready before T-070 merges, else A — they only conflict by path, and the earlier ready one goes first.

### Spec check R-261
Written to a full spec from the R-255 stub against main `15555cce3bfafdd8a579127ff49c3acd7f717ba6` (v1.4 §8.1).
- The 33 files re-measured on 15555cc with the R-255 method (scratch `r261/ws`, `cargo check
  --all-targets --target x86_64-pc-windows-msvc`, stub C compiler, `disk.rs` stand-in; 768 MB, deleted):
  **238 errors in 33 files** (R-255: 198; T-069 added Unix code to six of them), all in test code; the
  direct names: 31 test fns and 116 helpers or `use` lines (`win-items.txt`, approximate: an error is
  attributed to the nearest preceding `fn`). How many tests depend on those helpers is for the
  implementer to measure.
- Added `acceptance_t002.rs` (compiles on Windows but holds the inert-helper pattern) — 34 files.
- `xtask/tests/acceptance_t005_r3.rs`/`_r4.rs` are also in T-043's `touches` (its `refused` helper) →
  Q-4. `docs/development.md` is shared with T-070, T-041, T-043, T-064.
- `noskip.rs` facts read: cfg rule lines 9–10, 81–90; scan dirs 471–473; self-test count 492; detail line.
- Not checked: anything on a hosted runner; the number of registered tests.

### Rulings on spec check R-261 (director, D-180)
- **Q-1: B, split** (METHODOLOGY: bundles are split). T-071 keeps the tool, the registry file and `xtask`'s own
  files; the marking of the product test files becomes its own task (T-079), written by the planner when T-070 has
  merged; ruling Q-8 A of ADR-0009 (T-075 and T-076 stay low) holds across the two.
- **Q-2: A.** `pending` entries expire on 2026-12-31.
- **Q-3: A.** A `pending` entry names its future counterpart now.
- **Q-4: B.** T-043 and T-071 go in the order they become ready.
- The stub of T-075 still lists `xtask/platform-tests.toml`; it is corrected at its own specification.

### Spec check R-285
Against main `cbb54a2628afbd368087e82c5d7925eaa5b08625` (T-070 merged at `c7af15f`, D-197).
- **Split applied** (D-180 Q-1 B): the 28 product test files and the two product test modules left `touches`
  and Interface (4); they are T-079's. Goal items 3–4, AC-5, AC-6, Estimate rewritten accordingly.
- **New evidence: macOS.** The `gate-macos` artefact of run 43765532298 (downloaded with the Actions tools;
  zip unpacked under `/home/user/kiln-plan-T-071/`, deleted) has the whole test output: **977 passed, 22
  failed** over 121 `test result` lines (the record's 366/11 was a printed prefix). Of the 22, six are
  `xtask`'s (table in Interface (4)); none is a product fault: each is a Linux assumption of a test or of
  the director's tools. Added to `touches`: `xtask/src/queue.rs` and `xtask/src/worktree.rs` (test modules),
  `xtask/tests/acceptance_t013_publish.rs`. The other 16 are T-079's (its spec lists them).
- **New evidence: Windows.** Run 43765532298 confirms the xtask compile errors in `hygiene.rs:541` and
  `publish.rs:1341` (`T-070-round-2.md`); the four xtask test files' errors are hidden there behind the bin
  test's failure; R-261's check named them.
- Line references re-read on cbb54a2: `hygiene.rs:540-541`, `publish.rs:1340-1341`, test modules at
  `queue.rs:803`, `worktree.rs:228`, `hygiene.rs:426`, `publish.rs:753`; `noskip.rs:9-10, 81-90, 471, 492`;
  `acceptance_t002.rs:169-187, 924`; `docs/development.md:75` — all as stated.
- **Collisions:** T-054 shares `docs/development.md` → not in flight together (§5.2) — Q-6. T-058's files
  are clean (above). T-043: unchanged (D-180 Q-4 B).
- Not run: no build (nothing in this check needed one).

- **Q-5 (planner, R-285). The director's tools on macOS and Windows.** A: their tests that assume Linux
  (`/proc`, the queue's build tree) are registered `linux` with `counterpart = "none"`, since the queue,
  `worktree` and `publish` run only on the director's machine. B: make the tools portable (liveness without
  `/proc`, path canonicalisation). **Recommendation: A** — no platform but Linux runs them; B is real work
  (`kill(pid, 0)` needs `unsafe` or a new dependency) for no user.
- **Q-6 (planner, R-285). T-054 beside T-071.** A: sequence them (T-054 first: it is small and ready to
  specify). B: T-071 drops `docs/development.md` and T-079 carries its two rows, which leaves the `no-skip`
  row wrong on main between the merges. **Recommendation: A.**

### Rulings on spec check R-285 (director, D-199)
- **Q-5: A.** The director's tools' Linux-only tests (queue, `worktree`, `publish`) are registered as `linux` with
  no counterpart and the reason: they run on the director's machine only.
- **Q-6: A.** T-054 first, then this task; they are not in flight together (`docs/development.md` in both).
- The whole test output of `gate-macos` in run 43765532298 (the uploaded artefact, read in R-285) is 977 passed and
  22 failed, none a product fault by the panic messages; the "366 passed, 11 failed" in the director's records of
  T-070 was the printed prefix. The 6 `xtask` failures are this task's, the 16 product-test failures T-079's.

### Note from T-054 (director, D-205)
When `xtask`'s tests compile on Windows (AC-5, AC-6), this task's CI round on `ci/T-071` is the first run that
gives `guard_t054::ac2_backtrace_names_this_file_and_line` a verdict on Windows (ruling Q-4 A of T-054: a backtrace
with line tables only must name file and line). The director reads it from the artefact and records it; if it
fails there, it is ruled as a stated platform difference before this task is queued.

### Rulings on the test author's questions (director, D-220; the planner's recommendations of run R-308, with p2 and p4 decided otherwise; told to both builders)
- **p1 (question 1).** A finding about a registry entry (`platform-mismatch`, `platform-sentence`,
  `platform-counterpart`, `platform-pending-expired`, `platform-unused`) names exactly
  `xtask/platform-tests.toml:<line of that entry's [[test]] header>` and the entry's `file::item`.
  `platform-unlisted`, `inert-helper` and `platform-statement` name the `.rs` file and line.
- **p2 (question 2).** No new field. A `pending` entry names the task that will write the counterpart (`task`)
  and its `expires` date; the counterpart's name is not known before it is written. Ruling Q-3 A's sentence "a
  pending entry names its future counterpart now" is replaced by this. When the counterpart exists, the entry is
  edited to name it; a `pending` entry is satisfied exactly while `expires` is today or later and the `task` file
  exists, and is `platform-pending-expired` otherwise.
- **p3 (question 3).** Each platform cfg form means a set: `unix` = {Linux, macOS}, `windows` = {Windows},
  `linux` = {Linux}, `macos` = {macOS} (with the `not(...)` forms of Interface (1) as written). A block that holds
  platform-cfg statements is in order exactly when the union of their sets is all three platforms; `use` items stay
  exempt.
- **p4 (question 4).** AC-1 means exactly one FAIL line of that kind for the fixture, as written. That no fixture
  of the self-test yields a line of a second kind is the reviewer's check, not an acceptance test.
- **p5 (question 5).** Exactly the six cfg forms of Interface (1) mean a platform. `target_os = "windows"`,
  `any(...)`, `all(...)` and every other cfg on a test stay `cfg-disabled`, registered or not.
- **p6 (question 6).** An entry for an existing item that has no platform cfg is `platform-mismatch` (no cfg means
  all three platforms). `platform-unused` is exactly: no such `file::item` exists.
- **p7 (question 7).** "main's tree" is the root being scanned: the working tree the step runs in
  (`<root>/tasks/T-NNN-*.md`), never a branch named `main`. In the self-test the root is `xtask/fixtures/no-skip/`:
  every `file`, `doc`, counterpart and `tasks/` name in a fixture resolves inside it.
- Test author: add a guard for p5 (the two forms stay `cfg-disabled`) and cases for p3 (a `linux` statement
  complemented by `macos` and `windows` statements; one left uncomplemented) and p6; nothing else changes.

### Rulings on the implementer's report (director, D-222)
- **q1 (r3/r4 scope).** Accepted: in `xtask/tests/acceptance_t005_r3.rs` and `acceptance_t005_r4.rs` the task may
  edit the `from_mode` helper **and the cfg lines that registering the two tests of Interface (4) needs** (the test
  fn, `FAKE_GATE`, and in r4 `type Env`, `fn record` and the `PathBuf` import, used by that test only). Nothing near
  the `refused` helper. The `touches` comment's "the from_mode helper only" is read so.
- **q2 (AC-5).** "The test list equals main's name for name" means: no test name of main is lost on Linux; unit
  tests added by this task are allowed.
- The implementer's choices to be checked by the reviewer, not ruled here: `unix` rather than `linux` for the two
  tests that only fail to compile on Windows; whitespace runs in a quote match count as one space; a missing
  registry file is an empty registry; off Linux a failed test build is reported and the step goes on;
  `acceptance_t002::disk_save_replaces_via_rename_without_leftovers` fails on Windows by design, naming why, until
  the product does it there.

### Rulings on review round 1 and CI round 1 (director, D-226; `reviews/T-071-r1.md` at `5bf01d8717a2d17088b72d38a758ef8b29540874`; record `tasks/ci-runs/T-071-round-1.md`; told to both builders)
Threat model as before (D-026): honest mistakes and corner-cutting, not a determined evader. A hole that a plain
way of writing a test falls into is closed; a contrived one is stated as a limit of the tool.
- **s1 (finding 1; implementer).** A `cfg_attr` that makes `test` or `ignore` conditional
  (`#[cfg_attr(<any predicate>, test)]`, `#[cfg_attr(<any>, ignore)]`) is a `cfg-disabled` finding.
- **s2 (finding 2; implementer).** A helper's platform cfg is the combination of its own and its enclosing
  `mod` and `impl` items' cfgs: twins whose cfg sits on the enclosing item are twins.
- **s3 (finding 3; implementer).** A statement counts as a complement only if its platform cfg is its only cfg.
- **s4 (finding 4; implementer).** Statements in blocks at any depth inside a test fn are inspected, a fn nested in
  it included. The unit test that asserts otherwise is changed.
- **s5 (finding 5; implementer).** An item's platforms are the intersection of its own cfg and its enclosing
  items' cfgs; an empty intersection is a `cfg-disabled` finding, registered or not.
- **s6 (finding 6; implementer).** The trivial bodies of the spec's list count in their path-qualified forms too
  (`Option::None`, `Result::Ok(())`, `core::`/`std::` prefixes).
- **s7 (finding 7; implementer).** An empty block or empty statement is not a complement.
- **s8 (finding 8, spec gap).** Not closed; **stated as limits** in the "Platform tests" paragraph of
  `docs/development.md`, each with a toy case the reviewer ran: a runtime `if cfg!(…)` branch that is not an
  early return; `cfg` inside a macro call; twins written with `not(any(…))` or any form outside the six of p5; a
  body that does nothing in a form outside the spec's list. `guard_t071` keeps pinning the runtime branch.
- **s9 (finding 9; implementer).** `doc` is exactly: a repository-relative path, with `/`, of a tracked `.md` file
  under `docs/`. Anything else (the registry, a test file, an absolute path, a file outside `docs/`) is
  `platform-sentence`.
- **s10 (findings 10, 15; the spec's text corrected here).** Pinned (a): eight new kinds. AC-6's scorecard clause
  reads: off Linux the step's output names the platform and says the page was not compared **whether it passes
  or reports problems** (when it reports problems, that line comes first). Implementer.
- **s11 (finding 16; AC-6's Windows clause; implementer).** (i) The toy repositories of `xtask`'s tests are made
  independent of the machine's git settings where they are created (line-ending conversion off, and whatever
  else the helper leaves to the machine), in the helpers this task may edit; if a needed helper is outside
  `touches`, name it in a QUESTION line. (ii) Comparisons of printed paths read `\` as `/`. (iii) A test of the
  director's tools (queue, join, publish, worktree) that still cannot run on Windows for a stated reason is
  registered (`unix` or `linux`, no counterpart, ruling Q-5 A); every other `xtask` test passes on Windows.
  (iv) So that one CI round shows every failing binary, the core-crates step's per-package `cargo test` gets
  `--no-fail-fast` in the three gate jobs of `.github/workflows/platforms.yml` (in `touches` for this one flag);
  if `xtask/tests/acceptance_t070.rs` pins the command so that the flag breaks it, the test author adapts that
  one expectation (it is in the test author's `touches` for that).
- **s12 (test gaps; test author).** Tests that kill M9 (the `use` exemption widened to any item), M13 (a missing
  registry file must not make the step pass empty: with platform tests present and no registry the step fails
  with `platform-unlisted` lines) and M21 (methods in `impl` blocks are helpers); and a test for each of s1 to
  s7, s9 and s10. The nit (a `skips.toml` entry of a platform kind) is not changed.
- The pending CI criterion stays open: AC-6 and the Windows verdict of `guard_t054`'s backtrace test are read on
  CI round 2 after the re-join.
- **s1 clarified (the test author's question 8; told to the implementer in flight).** The existing kind
  `cfg-attr-ignore` keeps reporting `#[cfg_attr(<predicate>, ignore)]`; s1 adds only that
  `#[cfg_attr(<any predicate>, test)]` is `cfg-disabled`. The empty statement of s7 is `#[cfg(windows)] ();`
  (`#[cfg(windows)] ;` is not valid Rust).
- **s13 (the implementer's question in run R-313; D-228).** `xtask/src/join.rs` joins `touches` for its
  `#[cfg(test)]` module only: its toy repositories get the same four settings as those of `queue.rs`, `publish.rs`
  and `worktree.rs` (`core.autocrlf=false`, `core.eol=lf`, `core.safecrlf=false`, `core.longpaths=true`), so the
  three `join::tests` that still fail under `core.autocrlf=true` pass. `xtask/src/git.rs` stays untouched.
- **s14 (CI round 3; D-231; implementer).** The two remaining Windows failures have one cause: a path is written
  unescaped into a TOML string of a toy `Cargo.toml` (`xtask/tests/acceptance_t005.rs:711`,
  `seed_reuses_non_member_artifacts_only`; the same line in `xtask/tests/guard_t005.rs`,
  `seeded_worktree_builds_its_own_source`). The path is written so that TOML reads it on every platform (with `/`,
  or as a literal string). `xtask/tests/acceptance_t005.rs` joins `touches` for that one line.

### Rulings on review round 2 (director, D-232; `reviews/T-071-r2.md` at `418b96f011ed22f343a776098848a54419f5881d`)
- **R2-1** is ruling s14 (the implementer's run R-319). After it: a join, a CI round on the new head, and the
  reviewer's confirmation of the delta (the two lines) as round 3; nothing else is reworked.
- **R2-2, R2-3, R2-5, R2-6, R2-7** (a spec gap, a test gap, three nits) go to T-091 (draft); **R2-4** (the echo
  label in `platforms.yml`) to T-072.

### Rulings after the queue's refusal at main's gate (director, D-235)
The queue ran main's gate (the old tool) on the rebased tree first, as it does for every task that changes a gate
path, and it failed at `main:no-skip`: the old self-test reads `xtask/fixtures/no-skip/violations.rs` and
`clean.rs`, which this task extended. The old scan would also call the eight registered tests `cfg-disabled`. A
task cannot both change the rule and use it in the tree: main's tool judges the tree. So the task is cut in two.
- **u1 (this task keeps the rule and the tool; implementer).** `violations.rs` and `clean.rs` go back to main's
  bytes; the cases of the new kinds move to new fixture files that the new self-test reads beside them (the old
  self-test reads only the two old files). The eight entries of `xtask/platform-tests.toml` and the platform cfg
  attributes on those eight test fns (and nothing else) leave the branch: the registry file stays with its
  header and no entry. Everything else stays as reviewed at `b7dd7d6`.
- **u2 (the proof before the join; implementer).** On the reworked head, main's tool passes: run the primary
  checkout's `target/debug/xtask no-skip` (built from main) with the worktree as working directory, then
  `cargo xtask ci` (the task's own gate).
- **u3 (the usage moves to T-079).** The eight entries and their cfg lines, exactly as reviewed here (branch
  `park/T-071-reviewed`, head `8cb8464b9825ef2467ccc1ee14f8b95e40bb81b1`), are added by T-079 together with its
  twelve, after this task has merged and main's tool knows the rule. **AC-5 (the Windows check of `xtask`'s
  tests) and AC-6 (the CI clauses) move to T-079** with them; CI rounds 3 and 4 stay on record as the evidence
  that the combined tree works. This task's remaining criteria: AC-1 to AC-4 and the guards.
- **u4 (review).** Round 4 confirms the delta: the fixture cases are the same cases in new files (the self-test
  still knows 17 findings, one per new kind), the two old fixture files equal main's, and the tree differs from
  the reviewed one only by u1.

## Log
### director
<!-- end director log -->
- 2026-11-18 **CI round 1** (the pending CI criterion AC-6; record `tasks/ci-runs/T-071-round-1.md`, written from the
  artefacts in run R-312): run 44201932507 of `platforms.yml` on `ci/candidate` at the joined head
  `f7d511352027bdd8b0e48f207c3aac9ad5bbca85`. `gate-linux` success (`xtask` 261 passed; the 7 registered tests ran);
  `window-linux` and `window-probe (macos-15)` success (the counted jobs). **macOS: `xtask` exit status 0** (256
  passed, 0 failed; by count the five `linux` entries are absent); the `gate-macos` artefact was not uploaded
  (`ETIMEDOUT`), so no test names. **Windows: the `xtask` tests compile for the first time; the unit-test binary
  ran with 108 passed and 22 failed** and cargo stopped there, so no integration binary ran: 15 queue tests whose
  `touches` refusal lists files as changed, 4 merge or rebase conflicts in toy repositories, 1 CRLF content, 1
  path separator in a message, 1 file Windows cannot execute. AC-6: clause "`xtask` exit status 0 on
  `gate-windows`" not met; clause "the scorecard step on macOS prints the platform detail line" not met as printed
  (the step printed `6 scorecard problems: …`; the page was not compared). The Windows verdict of `guard_t054`'s
  backtrace test stays pending (its binary did not run). Rework is ruled with the review's findings.

- 2026-11-18 **CI round 3** (round 2, at `88f38ca`, is superseded and not read; record `tasks/ci-runs/T-071-round-3.md`,
  from the six artefacts, run R-318): run 44215218474 on `ci/candidate` at the joined head
  `d00e21d5901ce6c35124f985ed3bd3638044c96d`. `gate-linux` success; the counted jobs `window-linux` and
  `window-probe (macos-15)` success. **All 21 `xtask` test binaries ran on each platform**: Linux 278 passed, macOS
  273 passed, Windows 268 passed and 2 failed (ruling s14). None of round 1's 22 Windows failures recurs. The 8
  registered tests are present or absent exactly as their entries say. Off Linux the scorecard step prints the
  platform line first (s10). `acceptance_t071` 28 of 28, `guard_t071` 6 of 6, `acceptance_t070` 13 of 13 and
  `guard_t070` 7 of 7 on all three. macOS workspace: 1,101 passed, 16 failed, exactly the 16 product tests of
  D-199 (T-079's). AC-6: every clause met except "`xtask` exit status 0 on `gate-windows`" (the 2 failures).

- 2026-11-18 **CI round 4: the pending CI criterion is met** (record `tasks/ci-runs/T-071-round-4.md`, from the six
  artefacts, run R-322): run 44220309842 on `ci/candidate` at `b7dd7d621f4a90b4eb7165f6118a0be88fb7dad1` (the joined
  and reviewed head). `xtask` exit status 0 on all three platforms, 21 test binaries each: Linux 278 passed, macOS
  273, **Windows 270 passed and 0 failed** (`seed_reuses_non_member_artifacts_only ... ok`,
  `seeded_worktree_builds_its_own_source ... ok`). `gate-linux` success; `window-linux` and
  `window-probe (macos-15)` success. AC-6 met in every clause (the scorecard clause as s10 words it). The macOS
  and Windows gates stay red for the product tests (T-079) and `kiln-workspace` (T-073), as the spec expects.

### planner
- 2026-11-15 · R-255 · draft · did: filed from ADR-0009 · next: director
- 2026-11-15 · R-261 · draft · did: full spec against 15555cc (above) · not run: hosted runners · next: director (Q-1–Q-4, specified)
- 2026-11-15 · R-285 · draft · did: spec check against cbb54a2; split applied; macOS artefact read (22 failures classified) · not run: builds · next: director (Q-5, Q-6)
<!-- end planner log -->
### implementer
- 2026-11-17 · R-305 · implementer · head after this entry's commit; tool built on d419ad7 · gate: attempt 1 FAIL (clippy `useless_format`, mine), attempt 2 stopped by me mid-run (the working tree changed under it for D-220), attempt 3 PASS (`cargo xtask ci`, 458.6 s, 1083 passed, no-skip `self-test ok (17 known findings); 195 files scanned, 0 allowlisted skip(s), 7 platform test(s) registered (0 pending)`).
  - ACs restated: AC-1 each new kind found exactly once at its location, gone when fixed; AC-2 a correct registry passes and is counted; AC-3 twins with a do-nothing body fail, real twins and lone helpers pass; AC-4 Linux: a test registered off Linux is "registered absent here", page compared; AC-5 `xtask` checks for Windows with 0 errors, Linux test list keeps every name of main's, `no-skip` passes; AC-6 CI only.
  - Built: `xtask/src/platforms.rs` (cfg forms → platform sets; strict registry parser, one error line per faulty entry; the rules; inert-helper and statement analysis; scorecard name prefixes). `noskip.rs`: platform-form cfgs on tests are platform items, not `cfg-disabled`; self-test over the fixture registry `xtask/fixtures/no-skip/platform-tests.toml` + `platform-doc.md` (9 skips + one line of each of the 8 platform kinds = 17); order self-test, scan, `skips.toml`, registry; a malformed registry stops the step (FAIL, its lines). `scorecard.rs`: `TestState::RegisteredAbsent`, `run_on(host)`; off Linux: tests built there run, a failed build is a problem (not a stop), detail `<platform>: n items; m tests ran; k registered absent here; page not compared (computed on Linux)`, nothing written. `main.rs`: `mod platforms;` and the write-mode refusal. Registry `xtask/platform-tests.toml`: 7 entries, all `counterpart = "none"` (ruling Q-5 A).
  - Tests fixed: `hygiene.rs` and `acceptance_t013_hygiene.rs` symlinks by the platform's call (twins with real bodies); `queue.rs` `lock_rule_is_applied_to_the_rebased_tree` without `sed -i`; registered `linux`: `queue.rs` `tests::basic_refusals_write_nothing`, `worktree.rs` `tests::create_seed_list_remove`, `acceptance_t013_publish.rs` lock test (+ its only helper `tree_snap`), r3 tamper and r4 planted tests; registered `unix`: `publish.rs` step-4 hook test (exec bit), `guard_t005.rs` `worktree_build_does_not_touch_main_target` (+ helpers `inodes`, `shared_entries`; dev/ino are Unix-only in stable std). `acceptance_t002.rs`: the two inert twins removed; the mode/inode checks are `#[cfg(unix)]` and now unconditional there (no `if let Some`), and on Windows the test runs every other check and then panics naming why (§8.1, the `acceptance_t015_tools` pattern). No Linux assertion changed.
  - Decisions within the spec: `unix` (not `linux`) for the two tests that fail only to compile on Windows and pass on macOS (R-285 lists 6 macOS failures, not these); quote matching collapses whitespace runs (documented); a missing registry file is an empty registry (documented; deleting it makes every platform item unlisted); `platform-statement` also groups match arms; impl-method twins keyed by their impl block; Linux detail line unchanged unless k > 0 (then `… k registered absent here.`).
  - D-220 acknowledged and applied (accepted): p1 entry findings now name `xtask/platform-tests.toml:<header line>` and `file::item` (I had named the item's `.rs` line); p2 one pending line, satisfied while `expires` ≥ today and the task file exists; p3 one finding per block whose union is not all three (I had one per uncovered statement); p4 self-test requires exactly one line per kind; p5 already so (`target_os = "windows"`, `any`, `all` → `cfg-disabled`; unit test `cfg_forms`); p6 an entry for an existing test without platform cfg → `platform-mismatch`, `platform-unused` only when no such test fn/module/file exists; p7 `tasks/` and `doc` under the scanned root (self-test: the fixture dir).
  - Commands: `cargo check -p xtask --all-targets --locked --target x86_64-pc-windows-msvc` on a `git archive` copy of the head: 0 errors (base 550e457 with `--keep-going`: 13 errors in hygiene.rs, publish.rs, r3, r4, t013_hygiene, guard_t005). `cargo clippy -p xtask --all-targets --locked --target {x86_64-pc-windows-msvc,aarch64-apple-darwin} -- -D warnings`: 0. `cargo clippy -p kiln-workspace --test acceptance_t002 --target aarch64-apple-darwin -- -D warnings`: 0. `cargo test -p xtask -- --list --format terse` main vs head: 0 names only in main, 14 only in head (my new unit tests). Toy trees driven through `target/debug/xtask no-skip --today …` and `scorecard [--check]` (every kind, the pass detail with `(1 pending)`, a malformed entry, a `windows`-registered listed test: `1 registered absent here`, page written and compared; without the entry: `does not exist`).
  - QUESTION (scope): `touches` gives r3/r4 "the from_mode helper only", but Interface (4) says to register those two tests (`as the queue row`), which needs `#[cfg(target_os = "linux")]` on the test fn and on `FAKE_GATE`, and in r4 on `type Env`, `fn record` and the `PathBuf` import, which only that test uses (else `dead_code` under `-D warnings` on macOS/Windows). Done in its own commit e866a54 + 5 lines in r4 in a later one; nothing near the `refused` helper. Revert if the ruling is otherwise.
  - QUESTION (AC-5): "the Linux test list equals main's name for name": main's list is kept whole, 14 unit tests are added. I read it as "no name lost".
  - Not checked: anything on macOS or Windows hosts. Windows symlink tests need the runner's symlink privilege and assume std keeps `/` in a symlink target text (not verified). `acceptance_t002` does not build for Windows here because `kiln-workspace`'s `disk.rs` does not (product, T-073); its Windows branch is verified only by reading. The macOS/Windows sentences of `docs/development.md` (scorecard row, refusal) are checked by unit tests (`run_on` with `Platform::Macos`/`Windows`) and by reading `main.rs`, not run on those hosts.
  - Expected on `ci/T-071`: gate-linux green. gate-macos `core-crates` `xtask`: every `xtask` test runs except the 5 `linux` entries; exit 0 expected (INFERRED: the 6 R-285 failures are those 5 plus the `sed -i` one, now portable); scorecard step `macos: 64 items; <m> tests ran; 0 registered absent here; page not compared (computed on Linux)`, failing on the product failures (T-079). gate-windows: `xtask` tests compile; the 7 entries are absent; all other `xtask` tests run there for the first time (many spawn `git`, `sh` scripts, `cargo`), so failures not visible from Linux are possible and are new evidence, not marked by this task (out of scope: marking a test that compiles on Windows). `acceptance_t002::disk_save_replaces_via_rename_without_leftovers` fails on Windows by design, naming why.
  - next: director (QUESTIONs above; join)
- 2026-11-18 · R-313 · implementer · Rework after review round 1 (D-226 s1–s12 read on main, accepted; review `reviews/T-071-r1.md` and record `tasks/ci-runs/T-071-round-1.md` read)
  - s1: `#[cfg_attr(<pred>, …, test)]` is a test (`is_test_attr`) and its cfg is `cfg-disabled` (`platforms::item_cfg`); `cfg_attr(<pred>, ignore)` was already a finding (`cfg-attr-ignore`, one of the existing kinds) and stays that kind, so the self-test count and today's guards stay (read as: "is a finding"; say if it must be `cfg-disabled` instead). Unit: `noskip::tests::cfg_attr_test_is_cfg_disabled`, `platforms::tests::cfg_attr_test_is_a_cfg_that_is_not_a_platform_form`. Commit 0e52bdf.
  - s2: a helper's set is `env ∩ own` (enclosing `mod`/`impl`); line = its own platform cfg, else its name; impl methods keyed `<impl Type[ as Trait]>::name`. Unit: `twins_by_enclosing_mod_or_impl_cfg` (mod twins, impl twins, twins in one impl, real twins). 0e52bdf.
  - s3: a complement counts only when its platform cfgs are its only cfgs (`platform_attr` via `item_cfg`). s4: fns nested in a test are inspected (the unit assertion that said otherwise is reversed). s7: an empty block, `()`, a stray `;` counts for no platform. Unit: `statements_need_their_complement`. 0e52bdf.
  - s5: item set = `env ∩ own`; empty with a platform cfg → `cfg-disabled` (registered or not). Unit: `enclosing_cfgs_narrow_an_item_and_an_empty_one_is_cfg_disabled`. 0e52bdf.
  - s6: `None`, `Ok(())`, `Default::default()` also with `Option::`/`Result::`/`option::`/`result::`/`default::` and `std::`/`core::` prefixes. Unit: `trivial_bodies`. 0e52bdf.
  - s9: `doc` must start `docs/`, end `.md`, have no `\`, empty, `.` or `..` component, and be listed by `git ls-files --error-unmatch` in the scanned root; else `platform-sentence`. The fixture doc moved to `xtask/fixtures/no-skip/docs/platform-doc.md` (p7: inside the fixture root). Unit: `docs_are_tracked_markdown_under_docs`. 0e52bdf.
  - s10: off Linux the step's detail starts with the platform line, also on FAIL (then the problem list). Unit: `off_linux_the_page_is_neither_compared_nor_written`. `docs/development.md` scorecard row says so. 4f39e3f.
  - s11 (iv): `--no-fail-fast` added to the three `cargo test --locked -p "$p"` lines of `platforms.yml` (the `echo "==== cargo test --locked -p $p"` lines left as they are: "that flag only"); `docs/development.md`'s core-crates sentence quoted the command and now names the flag. `acceptance_t070` + `guard_t070`: 13 + 7 passed. 4f39e3f.
<!-- end implementer log -->
### test author
- 2026-11-17 · R-306 · tests written · did: `xtask/tests/acceptance_t071.rs` (16 tests) and
  `xtask/tests/guard_t071.rs` (6 guards), from the spec on main (b1b9376) alone; every test drives the built
  `xtask no-skip` / `xtask scorecard [--check]` on toy repositories under `CARGO_TARGET_TMPDIR` (no-skip toys
  carry a copy of `xtask/fixtures/no-skip/`, since the self-test runs first, Pinned (c)). Rulings acknowledged
  and accepted: D-180 Q-1 B, Q-2 A, Q-3 A (but see QUESTION 2), Q-4 B; D-199 Q-5 A, Q-6 A; note D-205 (nothing
  for the test author). Pinned (f): no acceptance test of T-005/T-013 asserts the `no-skip` detail line
  (`grep` for `allowlisted`, `files scanned`, `known findings`, `self-test ok` in `xtask/tests/`: 0 hits).
  - Criterion → tests → result on base (all by assertion; none fails only by compile):
    AC-1 → `ac1_platform_unlisted_names_file_line_and_item` (fn, module, inner `#![cfg]`, test in a module;
    no `cfg-disabled` for a platform cfg), `ac1_platform_mismatch_names_file_line_and_item`,
    `ac1_platform_mismatch_each_cfg_form_means_exactly_its_platform` (6 forms × 4 names),
    `ac1_platform_unused_names_the_entry`, `ac1_platform_sentence_missing_doc_or_quote`,
    `ac1_platform_counterpart_must_exist_and_cover_the_left_out_platforms` (missing file/item, a helper, a
    module without tests, a `unix` counterpart of a `unix` item, a `windows` counterpart of a `linux` item;
    accepted: `windows`, `not(unix)`, no cfg, module, `<file>::tests::x`),
    `ac1_platform_pending_expired_date_and_task_file` (day before, today, later; task file missing/added;
    HEAD's date without `--today`), `ac1_inert_helper_names_file_line_and_item`,
    `ac1_platform_statement_names_file_line_and_item`, `ac1_platform_statement_same_block_and_use_exemption`
    — each fails on base: no line of the new kind (base reports `cfg-disabled` or passes).
    AC-2 → `ac2_correct_registry_passes_and_the_detail_line_counts_it` (4 entries/1 pending, then 6/2),
    `ac2_malformed_registry_stops_the_step` — base: the correct registry fails (`cfg-disabled`).
    AC-3 → `ac3_t002_pattern_fails_real_twins_and_a_single_helper_pass`,
    `ac3_each_trivial_body_form_and_each_platform_pair` — base: no `inert-helper` line.
    AC-4 → `ac4_scorecard_registered_absent_test_is_not_missing_and_the_page_is_linux_only`,
    `ac4_scorecard_write_mode_writes_on_linux_and_is_refused_elsewhere` — base (Linux): `lists test
    toy_a::t::win_only, which does not exist`, exit 1. On macOS/Windows they assert Interface (3)'s behaviour
    there (decided at run time with `cfg!`): only CI can show it (AC-6).
    AC-5, AC-6: implementer/reviewer and CI; not tested here.
  - Guards (pass on base; must pass after): `guard_the_six_existing_kinds_are_still_reported_with_file_line_and_item`,
    `guard_skips_toml_still_allowlists_until_expiry_and_refuses_unused_entries`,
    `guard_other_cfgs_stay_cfg_disabled_even_when_registered` (`feature`, `target_os = "freebsd"`,
    `all(unix, feature)`), `guard_the_self_test_still_runs_before_the_scan`,
    `guard_runtime_cfg_and_cfg_test_are_not_findings`,
    `guard_scorecard_listed_test_absent_without_entry_is_a_problem_everywhere`.
  - Self-check: a stub of Interface (1)–(3) in a scratch copy (deleted) passes all 22. Mutants, each killed by:
    unlisted not reported → ac1_platform_unlisted; `not(windows)` not read as unix → ac1 forms, ac1 statement
    block, ac2 correct, ac3 forms; module cfg ignored / inner file cfg ignored → ac1_platform_unlisted, ac2
    correct; line off by one → ac1_platform_unlisted; mismatch unchecked → both ac1 mismatch tests; unused
    unchecked → ac1_platform_unused; quote unchecked → ac1_platform_sentence; counterpart checked by file only
    (a test that does not exist accepted) → ac1_platform_counterpart; counterpart's own cfg unchecked →
    ac1_platform_counterpart; `expires <= today` expired → ac1_platform_pending; task file unchecked →
    ac1_platform_pending; empty body not trivial → both ac3; feature twins counted → ac3 forms; trivial Unix
    side ignored → ac3 forms; single trivial helper reported → ac3 t002; complement searched in enclosing
    blocks → ac1 statement block; any other platform statement counts as complement → ac1 statement block;
    `use` not exempt → ac1 statement block; empty reason accepted / `none` without reason accepted → ac2
    malformed; pending count 0 → ac2 correct; `unix` not covering Linux → ac1 counterpart, ac2 correct, ac4;
    registered absent test never Missing whatever its platforms → ac4 registered-absent; page not compared on
    Linux → ac4 registered-absent; any cfg containing `unix` / any `target_os` read as a platform →
    guard_other_cfgs. Accepted forms (pass): entry findings at the registry entry's line instead of the item's;
    `inert-helper` at the real twin's line; another line format that names kind, `<file>:<line>` and item.
  - QUESTION 1: Interface (2) "a FAIL line naming `<file>:<line>` and the item": for findings about an entry
    (`platform-mismatch`, `-sentence`, `-counterpart`, `-pending-expired`, `-unused`) is `<file>:<line>` the
    item's place or the entry's place in `xtask/platform-tests.toml`? The tests accept both (for `-unused`, the
    entry's `file` at any line or the entry's lines). Proposal: "the item's `<file>:<line>`;
    `platform-unused`: `xtask/platform-tests.toml:<line of the entry>`".
  - QUESTION 2: ruling Q-3 A says a `pending` entry names its future counterpart, but Interface (1) has
    `counterpart = "pending"` and no field for that name. Tests use Interface (1)'s form; "a pending entry whose
    named counterpart exists is satisfied" is untested. Proposal: state the field (e.g. `future = "<file>::<item>"`).
  - QUESTION 3: `platform-statement`: Interface (1) has no complementary form for a `linux` or `macos`
    statement (no `not(target_os = …)`). Untested; please state what complements them (or that they always fail).
  - QUESTION 4: AC-1 says "exactly one FAIL line of that kind"; the tests do not assert that no line of another
    kind appears (except: no `cfg-disabled` for a platform cfg). Is "exactly one FAIL line" in all meant?
  - QUESTION 5: "Any other cfg on a test stays `cfg-disabled`": guarded for `feature`, `target_os = "freebsd"`,
    `all(unix, feature)`; not for `target_os = "windows"` or `any(unix, windows)`, which an implementation could
    reasonably read as platforms. Confirm they stay `cfg-disabled`, and the tests can add them.
  - QUESTION 6: an entry for an existing test with no cfg: `platform-unused` or `platform-mismatch`? Untested.
  - QUESTION 7: "task file … on main's tree of the scan": the working tree or the `main` branch? Toys make
    both the same; untested which. Same for whether the self-test fixtures may name files outside
    `xtask/fixtures/no-skip/` (a doc, a task file): toys copy any file a fixture names in `doc = "…"` /
    `task = "…"` from the repository; proposal: the fixture is self-contained.
  - Assumptions where the spec left room: the kind name appears in the FAIL line (as `(cfg-disabled)` today);
    findings exit 1, a malformed registry exits 1 or 2 and names `platform-tests.toml` only for the errors
    Interface (1) lists (unknown key, empty `reason`, empty quote, malformed date) and for non-TOML; a missing
    required field or an unknown `platforms` value is checked only to fail; `--today` overrides HEAD's date
    for `pending` as for skips; on Linux an item whose only absent tests are registered elsewhere is not
    `planned` (its status is not otherwise pinned); the non-Linux detail line contains `macos` / `windows`
    (case-insensitive); "struct of these" tested with named fields only.
  - Run: clippy `-p xtask --tests -D warnings` clean (also with the stub in place), `cargo fmt --check` ok,
    acceptance 16/16 fail on base, guards 6/6 pass, `cargo xtask no-skip` and `cargo xtask hygiene` PASS on this
    branch; the stub's rule finds the two inert helpers of `acceptance_t002.rs` at 176 and 186 (Pinned (g)).
    Not run: `cargo xtask ci` (the whole gate); macOS and Windows (CI, `ci/T-071`).
  - next: director (join; QUESTIONS 1–7)
- 2026-11-17 · R-309 · tests revised · did: rulings p1–p7 (D-220) acknowledged and accepted. p1: every
  finding about an entry is now asserted to name exactly `xtask/platform-tests.toml:<[[test]] header line>`
  and `<file>::<item>` (mismatch, both forms tests, unused, sentence, counterpart, pending); unlisted,
  inert-helper and statement keep the `.rs` place. p3: `ac1_platform_statement_same_block_and_use_exemption`
  rewritten to the union rule (linux+macos+windows passes; linux+windows, two Unix statements, and a
  complement in the enclosing block each fail, naming the faulty one-line block's line); one line per block
  or per statement both accepted. p5: `guard_other_cfgs_stay_cfg_disabled_registered_or_not` (was
  `…_even_when_registered`) adds `target_os = "windows"` and `any(unix, windows)`, each registered and not.
  p6: `ac1_platform_mismatch_names_file_line_and_item` adds an entry for `everywhere` (no cfg) → exactly one
  `platform-mismatch`, no `platform-unused`. p2, p4, p7: no change (the toys commit on `main` for HEAD's date;
  the task file is in the working tree the step scans). Base: acceptance 16/16 fail by assertion, guards
  6/6 pass. New mutants on a scratch stub (deleted), each killed: complement searched in enclosing blocks,
  pairwise unix/windows complement, any two different platforms → statement test; entry finding at the
  item's place, at the entry's last line, or naming the item without its file → mismatch (both), unused,
  sentence, counterpart, pending tests; no-cfg entry treated as unused, or accepted → mismatch test;
  `target_os = "windows"` or `any(..)` read as a platform → guard_other_cfgs. Accepted: one statement
  finding per block. clippy `-D warnings`, `fmt --check`, `no-skip`, `hygiene`: clean. Not run: `cargo
  xtask ci`; macOS, Windows. · next: director (join)
<!-- end test author log -->
### reviewer
- 2026-11-18 · R-311 · review round 1 · reviewed f7d511352027bdd8b0e48f207c3aac9ad5bbca85 · verdict: request-changes (`reviews/T-071-r1.md`). Rulings D-180, D-199, D-205, D-220 p1–p7, D-222 q1, q2 read on main and accepted. Findings: 7 defects (AC-6 Windows clause not met per the CI record, run 44201932507: 22 xtask unit tests fail on Windows, no integration binary ran; `cfg_attr(unix, test)` unseen; `mod`/`impl` twins not `inert-helper`; a complement disabled by another cfg counts; nested fns in a test not inspected; a test behind `windows` inside a `unix` mod passes with entries; `Option::None`/`Result::Ok(())` not trivial), 5 spec gaps (AC-6's macOS detail-line clause cannot hold while the step fails; empty complement; runtime `cfg!`, single helpers, macro bodies, other forms; `doc` may be any file; "seven" kinds in Pinned (a)), 3 test gaps (mutants M9, M13, M21 survive), 1 nit. Mutants 23: 18 killed by acceptance, 17 by own tests (overlap), 2 by own tests only, 3 survived. AC-5 run: Windows and macOS check 0 errors; no xtask test name of base lost (225 → 261). Machine restarted at 00:19Z during the run; earlier results kept were written down before it. Not run: macOS, Windows, the whole `cargo xtask ci`; CI record read at the end (R-312, main 14130e6): agrees on Linux and macOS xtask exit 0, disagrees on Windows and the macOS detail line; `guard_t054` Windows verdict still pending. · next: director
<!-- end reviewer log -->
