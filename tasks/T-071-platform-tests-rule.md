---
id: T-071
title: "Make no-skip and the scorecard step follow the platform-test rule: a registry of tests that exist only on some platforms, each with its counterpart or the reason there is none; no helper that does nothing on one platform; every test target compiles on all three"
tier: high              # METHODOLOGY v1.4 §4: gate paths (xtask/); the approval row must contain `gate paths`. No dependency change.
state: draft
owner_role: planner
scorecard_items: []     # none claimed; the tests it marks stay tied to their items
depends_on: [T-070]     # its Windows evidence is T-070's gate-windows core-crates step; and both edit docs/development.md (§5.2)
touches:                # one role per path (D-153). All but the last two: implementer.
  - xtask/src/platforms.rs                  # GATE PATH: new: the registry's reader and rules (shared by no-skip and scorecard)
  - xtask/src/noskip.rs                     # GATE PATH
  - xtask/src/scorecard.rs                  # GATE PATH
  - xtask/src/main.rs                       # GATE PATH: only the `mod platforms;` line and the write-mode refusal of Interface (4)
  - xtask/platform-tests.toml               # GATE PATH: new: the registry with every entry (ruling Q-8 A, D-175)
  - xtask/fixtures/no-skip/                 # GATE PATH: the self-test cases of the new findings
  - xtask/src/hygiene.rs                    # GATE PATH: its #[cfg(test)] module only (Unix symlink calls, lines 540–541)
  - xtask/src/publish.rs                    # GATE PATH: its #[cfg(test)] module only (line 1340)
  - xtask/tests/acceptance_t005_r3.rs       # GATE PATH: the from_mode helper only (T-043 edits the `refused` helper: not in flight together)
  - xtask/tests/acceptance_t005_r4.rs       # GATE PATH: as above
  - xtask/tests/acceptance_t013_hygiene.rs  # GATE PATH
  - xtask/tests/guard_t005.rs               # GATE PATH
  - crates/kiln-workspace/src/disk.rs       # its #[cfg(test)] module only (from line 535); product code unchanged (T-073)
  - crates/kiln-commands/src/doc.rs         # its #[cfg(test)] module only (from line 1312)
  - crates/kiln-workspace/tests/acceptance_t002.rs        # the two inert helpers (lines 169–187) and the test that uses them
  - crates/kiln-workspace/tests/acceptance_t002_r2.rs
  - crates/kiln-workspace/tests/acceptance_t008.rs
  - crates/kiln-workspace/tests/acceptance_t019_workspace.rs
  - crates/kiln-workspace/tests/acceptance_t036_workspace.rs
  - crates/kiln-workspace/tests/confinement.rs
  - crates/kiln-workspace/tests/guard_t008.rs
  - crates/kiln-workspace/tests/guard_t036_removal_workspace.rs
  - crates/kiln-workspace/tests/guard_t036_workspace.rs
  - crates/kiln-workspace/tests/project.rs
  - crates/kiln-commands/tests/acceptance_t037_commands.rs
  - apps/kiln-cli/tests/acceptance_t008_cli.rs
  - apps/kiln-cli/tests/acceptance_t019_cli.rs
  - apps/kiln-cli/tests/acceptance_t023_cli.rs
  - apps/kiln-cli/tests/acceptance_t036_cli.rs
  - apps/kiln-cli/tests/acceptance_t037_cli.rs
  - apps/kiln-cli/tests/acceptance_t037_removal_cli.rs
  - apps/kiln-cli/tests/guard_t019_cli.rs
  - apps/kiln-cli/tests/guard_t036_cli.rs
  - apps/kiln-cli/tests/guard_t036_removal_cli.rs
  - apps/kiln-cli/tests/guard_t037_cli.rs
  - apps/kiln-cli/tests/guard_t038_cli.rs
  - apps/kiln/tests/acceptance_t055.rs
  - apps/kiln/tests/acceptance_t065.rs
  - apps/kiln/tests/script_t055.rs
  - apps/kiln/tests/window_t065.rs
  - docs/development.md                     # implementer: the `no-skip` and `scorecard` rows of "The gate" and a paragraph on the registry
  - xtask/tests/acceptance_t071.rs          # GATE PATH: test author
  - xtask/tests/guard_t071.rs               # GATE PATH: test author (today's no-skip findings that must stay)
budget_tokens: 1100000  # high (v1.4); estimate above the ceiling, see Estimate and Q-1
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
3. **every test target of the workspace compiles for `x86_64-pc-windows-msvc`** once `kiln-workspace`'s
   6 product-code errors are set aside (they are T-073's): today 238 errors in 33 files, all test code
   (R-261 re-check on 15555cc; ADR-0009 probe a4 method); after this task 0 in test code;
4. `xtask`'s own tests compile and pass on Windows and macOS (seen in T-070's CI core-crates step).
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

### (4) The entries (ruling Q-8 A: this task writes all of them)
For each of the 33 files and test modules that do not compile for Windows today (list in `touches`; R-261
`win-items.txt`: 238 errors, 31 test fns and 116 helpers or `use` lines named directly), and for
`acceptance_t002.rs` (inert helpers):
- a test that cannot compile or cannot mean anything off Unix gets `#[cfg(unix)]` (or the file `#![cfg(unix)]`
  only if every test in it is of that kind), its Unix-only helpers get `#[cfg(unix)]` without an off-Unix
  twin, and the test gets an entry with `doc`/`quote` of the sentence it tests and
  `counterpart = "pending"`, `task` = `T-075` (`kiln-workspace`, `kiln-commands`) or `T-076` (`kiln-cli`,
  `kiln`), a `reason` naming the facility (FIFO, mode bits, uid/gid, inode, non-UTF-8 bytes in a name or
  argument, `mkfifo`, shell-script fakes), and `expires` per ruling Q-2;
- or `counterpart = "none"` where Windows has no such facility and no sentence of `docs/protocol/` will
  state a Windows behaviour for it — expected to be rare; each one is named in the implementer's log;
- a test that compiles off Unix stays unmarked; no assertion is changed, removed or loosened on Linux or
  macOS (golden rule 4);
- `xtask`'s six files and modules become portable instead (the gate must run its own tests where it runs):
  a symlink is made with the platform's call; a mode is set with the platform's means; `dev`/`ino` checks
  use a portable identity or are Unix-only entries with a counterpart in the same file.
How many tests end up registered is **to be measured by the implementer** (the count of entries, by
`task`), and stated in the log and in the `no-skip` detail line.

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
- AC-5: in a scratch copy of the task head with `disk.rs`'s 6 product lines replaced by the probe's
  stand-in (ADR-0009 `logs/probe-shim.diff`, never committed) and the C compiler stubbed,
  `cargo check --workspace --all-targets --locked --target x86_64-pc-windows-msvc` reports **0 errors**
  (today 238); the same for `aarch64-apple-darwin` without the stand-in (today 0, must stay 0); and the
  Linux test list (`cargo test --workspace -- --list`) is unchanged, name for name, from main's.
Observed only through CI (the director's record; §8.1):
- AC-6: on the candidate `ci/T-071`, T-070's `gate-macos` scorecard step shows the platform detail line
  with 0 problems from absent tests, and `gate-windows`'s core-crates step shows `xtask`'s tests passing.
  (The Windows gate stays red until T-073; that is expected, T-070 "Expected to FAIL".)

## Out of scope and implementer notes
- Writing Windows counterparts (T-073 to T-076); product code (`disk.rs` above line 535 is T-073's).
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
- T-057–T-059: new files `apps/kiln/tests/acceptance_t05N.rs`, not in this `touches`; their tests must
  compile on Windows from the start (no Unix-only API without an entry). If one merges before T-071
  with a Unix-only test, it is added to T-071's `touches` by a ruling.
- T-066, T-068 (kiln-ui): no shared path.
- T-072–T-078 depend on this task.

## Estimate
xtask gate changes of this experiment: T-033/T-043-class tool work 700,000–930,000 (stage-1 high p75
866,379); plus mechanical marking of 34 files with per-test judgement and the Windows check (AC-5). **About
1,300,000**: above the high ceiling 1,100,000 (Q-1).

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

## Log
### director
<!-- end director log -->
### planner
- 2026-11-15 · R-255 · draft · did: filed from ADR-0009 · next: director
- 2026-11-15 · R-261 · draft · did: full spec against 15555cc (above) · not run: hosted runners · next: director (Q-1–Q-4, specified)
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
