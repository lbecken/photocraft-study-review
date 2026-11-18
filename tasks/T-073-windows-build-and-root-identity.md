---
id: T-073
title: "Make the workspace crate build and its root work on Windows: the root's identity and holding it open, the save's temporary file, every library and binary built and lint-clean for Windows"
tier: high              # METHODOLOGY v1.5 §4: the persistence path and the confinement boundary (disk.rs); a dependency edge (Cargo.toml target table, Cargo.lock); gate paths if Q-1 A (platforms.yml, xtask/tests) and the registry (xtask/platform-tests.toml)
state: queued
owner_role: planner
scorecard_items: []     # none claimed: ws.* items are verified on Linux; per-platform confirmation is the evaluator's
depends_on: [T-072, T-093]   # T-072: shares platforms.yml and acceptance_t070.rs under Q-1 A; T-093: shares docs/protocol/paths.md
touches:                # one role per path (D-153)
  - crates/kiln-workspace/src/disk.rs        # implementer: the product code of Interface (1)–(3) and its #[cfg(test)] module (Interface (5))
  - crates/kiln-workspace/Cargo.toml         # implementer: `[target.'cfg(windows)'.dependencies] same-file = "=1.0.6"`
  - Cargo.lock                               # generated: one line (`"same-file"` in kiln-workspace's dependency list)
  - xtask/platform-tests.toml                # GATE PATH: implementer: entries for the Unix-only tests of disk.rs's test module, and (by ruling) the test author's Windows tests
  - docs/protocol/paths.md                   # implementer: "The root" for Windows, and one sentence that the rest of the page is not yet stated for Windows (T-074)
  - docs/control-protocol.md                 # implementer: the save sentence (line 133 "(on Unix)")
  - docs/development.md                      # implementer: exactly the sentence of "Platform tests" that says what the registry lists (lines 101-105 on main), ruling c2 (D-250)
  - .github/workflows/platforms.yml          # GATE PATH: implementer, only with Q-1 A: one reported Windows step
  - xtask/tests/acceptance_t070.rs           # GATE PATH: implementer, only with Q-1 A: the expectation of that step
  - crates/kiln-workspace/tests/acceptance_t073.rs   # test author: Windows behaviour (Interface (4))
  - crates/kiln-workspace/tests/guard_t073.rs        # test author: the runtime tests of Interface (4), which pass on Unix base (ruling y1, D-247)
  - xtask/tests/acceptance_t079.rs                  # GATE PATH: test author: its two registry tests go from "exactly T-079's 20" to "T-079's 20 are present, each exactly" (ruling z1, D-249)
budget_tokens: 1100000  # high (v1.5); estimate about 1,050,000 (an Extend row is likely)
iteration: 2
found_by: planner       # ADR-0009 (R-255); re-specified R-336 (v1.5 §5.7)
origin_task: null
---

# T-073 Build `kiln-workspace` for Windows, with the root held and identified there

Re-specified in full in run R-336 against main `7b86bdf` (v1.5). The first Windows product task: after it, the Windows
jobs get past compilation of every library and binary. It does **not** decide how a path is walked on Windows (links,
junctions and other reparse points, drive and `\\?\` prefixes, reserved names, streams, case): that is T-074's
(ADR-0009 ruling Q-6 B: reparse points refused). It does not make the product test files compile for Windows: T-092.

## Goal
On Windows `kiln-workspace` does not compile: 6 errors in `crates/kiln-workspace/src/disk.rs` (lib) and 34 in its lib
test — reproduced in R-336 on `7b86bdf` (`cargo check -p kiln-workspace --all-targets --locked --target
x86_64-pc-windows-msvc`, C compiler stubbed): `disk.rs:86` (`std::os::unix` import), `:189`, `:213` (`dev()`, `ino()`
of the root), `:361` (`OpenOptions::mode`). So nothing above it builds there: `gate-windows` fails at clippy, schema,
scorecard and test, and `window-probe (windows-2025)` at "Build kiln" (`tasks/ci-runs/T-079-round-2.md`). After this
task: every library and binary of the workspace builds and is clippy-clean for Windows; on Windows the root is held
open and identified as on Unix in effect (Interface (1)); a save writes and replaces as specified there (Interface (3));
the Linux and macOS code paths are unchanged.

Reading list: ADR-0009 (Decision 4, rulings Q-6, probe a6); `docs/protocol/paths.md` "The root";
`docs/control-protocol.md` save paragraph; `crates/kiln-workspace/src/disk.rs` lines 1–535; `tasks/T-074-*.md`,
`tasks/T-092-*.md` (what is theirs); `docs/development.md` "Platform tests"; METHODOLOGY v1.5 §5.7, §6, §8.1.

## Interface (behaviour on Windows; Unix unchanged)
### (1) The root
- `DiskFs::new(root)` on Windows: the root is resolved as now (`fs::canonicalize`), then the directory is **held open
  for the life of the `DiskFs`** and its **identity** is that of the held directory: two handles are the same
  directory when they name the same file on the same volume (`same_file::Handle` equality: volume serial number and
  file index). A root that cannot be opened is refused as now (`io_error`, path `(root)`).
- Every call that today re-checks the root (`current_root`) re-resolves the given path and goes on only if the
  directory it reaches now is the held one; otherwise `outside_root`, as on Unix.
- What a held root means on Windows for "the root path replaced": whether a held directory can be renamed or removed
  by another process is **to be measured by the test author on the runner** (AC-W2, CI); the sentences of
  `paths.md` "The root" follow the measurement.
### (2) Not decided here (T-074)
The walk below the root runs the existing code on Windows, unchanged; until T-074 the documents say the path rules of
`paths.md` below "The root" are not yet stated for Windows. No Windows-specific refusal is added here.
### (3) The save
- The temporary file is created exclusively (`create_new`), with no mode on Windows (the `0600`/target-mode sentences
  are Unix's); the target's permissions are copied as now (`set_permissions` with the target's `Permissions`, which on
  Windows carries the read-only attribute); then renamed over the target (`fs::rename`). A target the process cannot
  open for writing is refused as now.
- Directory sync after the rename: best effort as now (on Windows a directory cannot be opened this way; the failure
  is ignored, as the code already does).
### (4) Tests (test author)
`acceptance_t073.rs`: Windows-only tests (`#[cfg(windows)]`, registered `windows`, counterparts the existing Unix tests
of the same sentence, e.g. `acceptance_t036_workspace`, `guard_t036_workspace`, `acceptance_t002`): a root that exists
is accepted and held; a session's root replaced by another directory at the same path is refused with `outside_root`
(or, if Windows refuses the replacement while the root is held, the test records that the replacement fails — AC-W2);
a save of a read-only target is refused and changes nothing; a save replaces the target and leaves no `.kiln-tmp-*`.
### (5) `disk.rs`'s own test module
Its 34 Windows errors are tests of Unix facilities (modes, FIFOs, `uid`, inodes, symlinks made with Unix calls). Each
such test gets `#[cfg(unix)]` and an entry (`platforms = "unix"`, counterpart `pending` T-092 or a named Windows test
of (4), reason the facility); its Unix-only helpers the same cfg without a twin. So `cargo test -p kiln-workspace
--lib` builds on Windows. No assertion changes on Linux or macOS.

## Pinned
- (a) Errors: as now (`VfsError` kinds unchanged); no new kind.
- (b) Files: as now (`.kiln-tmp-N` beside the target).
- (c) Order of checks: as now.
- (d) Platform: Windows x86_64 MSVC (CI `windows-2025`).
- (e) Threat model: a confinement boundary. Until T-074 the Windows walk is not specified and the Windows jobs are
  tracking, not counted (§8.1); no Windows build is released (T-078 after T-074).
- (f) Tests pinning: none on Linux change; `disk.rs` test module per (5).
- (g) **Probe (R-336, scratch `/home/user/kiln-plan-T-073/`, export of `7b86bdf`, deleted; `df -h /home/user`: 22 GB
  free; C compiler stubbed as ADR-0009 probe a3):** a prototype of (1) and (3) — the `std::os::unix` import and the
  `(dev, ino)` field under `#[cfg(unix)]`, a `#[cfg(windows)] root_handle: same_file::Handle` taken with
  `Handle::from_path(&resolved)` and compared in `current_root`, `.mode(mode)` only under `#[cfg(unix)]`; 51 diff lines
  — gives: `cargo clippy -p kiln-workspace --lib --locked -- -D warnings` clean for `x86_64-pc-windows-msvc`,
  `x86_64-unknown-linux-gnu` and `aarch64-apple-darwin`; **`cargo clippy --workspace --lib --bins --locked --target
  x86_64-pc-windows-msvc -- -D warnings`: clean** (every library and both binaries); `cargo check -p kiln-workspace --lib
  --tests --target x86_64-pc-windows-msvc`: the lib test still has 28 errors (the test module, (5)) and integration
  tests fail to build (T-092's; the run had no `--keep-going`, so they were not all counted); on Linux `cargo test -p kiln-workspace --locked --no-fail-fast`: every binary
  `ok`, 127 tests, 0 failed. `Cargo.lock`: one added line, `"same-file",` in `kiln-workspace`'s list, no package or
  version changed (`diff`), with the dependency as `[target.'cfg(windows)'.dependencies]` (then `cargo tree -p
  kiln-workspace -e normal --depth 1` on Linux does not list `same-file`). Whether the prototype *behaves* on Windows
  was not run (no Windows here).
- (h) Evaluator: none.
- (i) Document sentences: `paths.md` "The root" on Windows (held, identity by volume and file index; what a held root
  allows: **to be measured by the test author on CI**, AC-W2); "the rules below are not yet stated for Windows (T-074)"
  (no evidence needed: a scope statement); `control-protocol.md:133` "(on Unix)" → the Windows save sentence, its parts
  each **unchecked** until AC-W3's record (no mode; read-only refused; replace by rename).
- (j) **Main's gate on the candidate tree** (D-235): the task changes no `xtask/src/` file and no fixture; main's tool
  reads the registry the candidate extends (main has 20 entries, all valid). `no-skip` must accept the new entries:
  shown before the join by the implementer with main's `cargo xtask no-skip` (built from main) on main plus the diff,
  logged with its detail line (`<20+n> platform test(s) registered`). With Q-1 A the workflow edit is read only by
  `acceptance_t070.rs` from the candidate tree.

## Acceptance criteria
Local (implementer, re-run by the reviewer; R-336's commands, C compiler stubbed where `kiln-syntax` is built):
- AC-1: `cargo check -p kiln-workspace --lib --locked --target x86_64-pc-windows-msvc` → 0 errors (base: 6).
- AC-2: `cargo clippy --workspace --lib --bins --locked --target x86_64-pc-windows-msvc -- -D warnings` → clean (base:
  fails at `kiln-workspace`, 6 errors).
- AC-3: `cargo check -p kiln-workspace --lib --profile test --target x86_64-pc-windows-msvc` (the lib test target only) → 0 errors
  (base: 34; 28 with the prototype of (g)); the integration-test targets are T-092's (not required).
- AC-4: the set of `Cargo.lock` lines changed equals exactly {`+ "same-file",` in `kiln-workspace`'s list}; must reject a
  `same-file` version change.
Gate (local, Linux), guards of unchanged behaviour (hold on base, §6): every existing `kiln-workspace` test passes on
Linux (`ws.path-confinement`'s 28 and the rest); the guard `guard_t073.rs` is not needed — the existing tests are the
guard, and the reviewer checks that every changed line of product code is under `#[cfg(windows)]` or adds a
`#[cfg(unix)]` to an unchanged line (Q-3).
Through CI (pending criterion; the join records it as pending, §8.1 "CI tasks"):
- AC-W1: `window-probe (windows-2025)`: "Build kiln" succeeds; the probe runs `kiln --root <dir> --script …` and
  records its exit status (0 if a window opened; 2 with one line if the runner gives no window — either is a result;
  the window is T-077's).
- AC-W2 (Q-1 A): the Windows step running `cargo test --locked --no-fail-fast -p kiln-workspace --lib --test
  acceptance_t073` shows every test `ok`, by name, in the artefact; this also measures (i)'s held-root sentence.
- AC-W3: `gate-windows`'s `schema` step PASS (it builds and runs `kiln-cli`; base: FAIL "kiln-cli commands --audit
  failed"); `clippy` still FAIL (test targets of other crates, T-092); `scorecard` and `test` still FAIL (test
  executables, T-092).

### The pending CI criterion (v1.5 §8.1), for the approval row
The record (`tasks/ci-runs/T-073-round-N.md`, from the artefacts) of the rebased head on `ci/candidate` shows:
`gate-linux`, `gate-macos`, `window-linux`, `window-probe (macos-15)` green (counted jobs); `gate-windows`: fmt,
layers, ui-commands, hygiene, **schema**, no-skip PASS; clippy, scorecard, test FAIL, each only with errors in test
targets other than `kiln-workspace`'s lib test and `acceptance_t073` (named in the record); the core-crates step
`xtask` exit status 0 as now; with Q-1 A, the new step's `test-output` naming every `acceptance_t073` and
`kiln-workspace` lib test `ok`; `window-probe (windows-2025)`: "Build kiln" success and a `PROBE:` exit-status line.
**Two to three CI rounds expected** (the held-root measurement may change (1)'s sentences).

## Out of scope and implementer notes
- The walk on Windows (T-074); product test files outside `disk.rs`'s module (T-092); the window (T-077).
- No change to a Unix code path: a line used on Linux or macOS is either unchanged or gains only `#[cfg(unix)]`.
- The test author's registry entries: listed in its log; the director adds them to `xtask/platform-tests.toml` at the
  join (one role per path, D-153; Q-2).

## Collisions and order
- `docs/protocol/paths.md`: T-093 → after it. `platforms.yml`, `acceptance_t070.rs` (Q-1 A): T-072 → after it.
  `crates/kiln-workspace/src/disk.rs`: T-074 (after), T-092 (after; T-092 drops its `disk.rs` module part, done here),
  T-075 (draft, after). `xtask/platform-tests.toml`: T-092, T-089, T-090, T-043 (drafts) → not in flight together.
- Recommended order: T-072 ‖ T-093, then T-073, then T-074, then T-092.

## Estimate
T-071 1,286,681 (a gate tool, 4 CI rounds); T-079 846,099 (marking and fixes, 2 CI rounds). This task: product code
in the confinement module (about 60 lines, prototype), 34 test-module markings, a Windows test file by an independent
test author, 2–3 CI rounds: **about 1,050,000**, close to the high ceiling 1,100,000; an `Extend` row is likely. Arm:
high → independent test author.

## Questions and rulings
- **Q-1 (planner, R-336). Windows test evidence before T-092.** The gate's `test` step on Windows cannot build until
  T-092 (other crates' test files). A: this task adds to `gate-windows` one reported step (`cargo test --locked
  --no-fail-fast -p kiln-workspace --lib --test acceptance_t073`, recorded, not required) and its expectation in
  `acceptance_t070.rs`; after T-072. B: no Windows test runs; evidence is AC-1–AC-4 (local) and AC-W1/AC-W3.
  **Recommendation: A** — §8.1 takes Windows evidence only from CI, and (1)'s held-root sentence needs a measurement
  there. (Measured here only that the lib test target can be made to build: probe (g).)
- **Q-2 (planner, R-336). Registry lines for the test author's tests.** A: the test author lists them in its log, the
  director adds them at the join (D-153's "owner adds a line"). B: the test author owns `xtask/platform-tests.toml` and
  the implementer lists its entries. **Recommendation: A** — the implementer has most entries ((5)).
- **Q-3 (planner, R-336). Showing the Unix paths unchanged.** A: the existing tests plus the reviewer's line-by-line
  check that every product change is under a cfg (no new guard). B: a guard test that scans `disk.rs` for unguarded
  changes against a pinned copy (brittle, golden rule 9). **Recommendation: A.**
- **Q-4 (planner, R-336). `same-file` as a Windows-only dependency.** A: `[target.'cfg(windows)'.dependencies]` (Linux
  and macOS builds unchanged; measured: the same one-line lock change, clippy clean on Windows). B: an unconditional
  dependency. **Recommendation: A.** Either way no package version changes, so the approval row needs no `dependency
  versions` (it needs the high-tier approval for a dependency change, and `gate paths` with Q-1 A or the registry).

### Rulings on spec R-336 (director, D-245)
- **Q-1: A, in this task and now.** The one reported Windows step (`cargo test --locked --no-fail-fast -p
  kiln-workspace --lib --test acceptance_t073`, recorded, not required) and its expectation in
  `acceptance_t070.rs` are this task's; it does not wait for T-072. T-072 is dispatched after this task merges
  and its spec is re-checked then (both edit `platforms.yml`).
- **Q-2: neither A nor B as written.** The registry is the implementer's path and the director commits no content
  on a task branch. The test author's file uses no platform cfg: what differs by platform is decided at run time
  with `cfg!`, as in T-071 and T-079. If a platform-only test is unavoidable, the test author names it in its
  report and the implementer adds the entry before the join.
- **Q-3: A.** The Unix paths are shown unchanged by the existing tests and by the reviewer's line-by-line check
  that every product change is under a cfg.
- **Q-4: A.** `same-file` is a Windows-only dependency (`[target.'cfg(windows)'.dependencies]`); the lock gains one
  line and no package version changes. The approval row names the dependency change and `gate paths`.
- Order: this task now (the Windows critical path: T-073, T-074, T-092, T-075 to T-078); T-072 and T-093 after it,
  where their paths are free (T-093 shares `docs/protocol/paths.md` with this task and with T-074).

### Rulings on the test author's report (director, D-247; told to both builders)
- **y1 (question 1: nothing fails on base on Linux).** The ten runtime tests of Interface (4) assert, on Unix,
  behaviour the spec keeps unchanged, so they pass on base there: they are guards. They move to
  `crates/kiln-workspace/tests/guard_t073.rs` (in the test author's `touches` now), unchanged. On Windows they are
  the runtime evidence of AC-W2. `acceptance_t073.rs` holds what fails on base on every platform and can be read
  there: the crate's manifest declares `same-file` exactly as ruling Q-4 A says — under a Windows-only target
  table, with the version `=1.0.6`, and nowhere else in the manifest (read as text; no new dev-dependency, since
  the lock may gain exactly one line). One variant it must reject: the dependency in `[dependencies]`.
- **y2 (question 2: where the measurement shows).** The reported Windows step runs both files and prints the
  passing tests' output: `cargo test --locked --no-fail-fast -p kiln-workspace --lib --test acceptance_t073 --test
  guard_t073 -- --show-output` (implementer; the expectation in `acceptance_t070.rs` follows). The `MEASURED:`
  lines of the two root-rename tests are then in the record, and the director reads them for the Pinned
  sentence on a held root.
- Not tested and noted for T-074: removing (not renaming) a held root; and, if Windows refuses both renames, the
  identity check has no Windows test in this task.

### Rulings on the implementer's block (director, D-249; told to both builders)
- **z1 (the block: main's `acceptance_t079` fails on any tree that adds a registry entry).** T-079's two tests
  pinned the whole registry (`exactly the twenty`), which was right for T-079's diff and makes every later task
  that adds an entry fail main's tests. They change, in this task, by the test author
  (`xtask/tests/acceptance_t079.rs` joins `touches`): **each of T-079's 20 (file, item) pairs has exactly one
  entry, with exactly T-079's fields; the pending entries among those 20 are exactly T-079's 8 with their task and
  expiry; entries for other (file, item) pairs are not these tests' matter.** What they still reject: one of the
  20 missing, changed (platforms, counterpart, task, expiry) or entered twice. What guards the rest stays where it
  is: `no-skip` checks every entry against a cfg'd item, and the base-names guard keeps every Linux test.
  The implementer's gate attempt 1 (1108 passed, 2 failed: exactly these two tests) counts as its gate evidence;
  the whole gate runs at the join on the merged head.
- **z2 (the implementer's second question).** Confirmed: the save bullet of `docs/control-protocol.md` keeps
  "(on Unix)" word for word (`apps/kiln-cli/tests/acceptance_t008_cli.rs` pins it) and the Windows sentence is
  appended after it.
- **z3 (the 15 pending entries).** They stay as written (`task = "T-092"`), by Interface (5). The reviewer says
  for each whether its counterpart is T-092's (a Windows form of the same check), T-074's (the walk and
  replaced-root cases) or `none` (a facility Windows lacks, such as mode bits); the director moves the task
  names by a ruling after the review, and T-092 and T-074 are noted to carry them.

### Rulings on review round 1 and CI round 1 (director, D-250; `reviews/T-073-r1.md` at `8bbe10aa7c7fc3c6c32abda3d0ff6169ec3fb679`; record `tasks/ci-runs/T-073-round-1.md`; all for the implementer)
- **c1 (finding 1, defect).** `docs/protocol/paths.md` and `docs/control-protocol.md` state what run 44268710587
  measured, naming the record, in place of "to be measured" and "unchecked": on Windows the held root itself can
  be renamed, and every call then answers `OutsideRoot`; renaming a directory above the root is refused by
  Windows while the root is held (`PermissionDenied`, OS error 5) — a difference from Linux and macOS, stated as
  one. The three parts of the save sentence (no mode; a read-only target refused; replaced by rename) are stated
  as shown by the guards passing on Windows in that run. What was only read, not run, is marked so.
- **c2 (finding 2).** `docs/development.md` joins `touches` for the one sentence of "Platform tests" that says
  what the registry lists: it is reworded so that it stays true as entries are added (it names the file and the
  kinds of reason, not a count or a list of crates), and it is true of this tree's 35 entries.
- **c3 (finding 3, read not run).** `paths.md` states the limit: on Windows the root's identity is the volume
  serial number and the 64-bit file index; on a file system that does not keep that index unique (ReFS is
  documented so) two directories can compare equal. Marked as read, not measured. The 128-bit identity is T-094
  (draft).
- **c4 (finding 4, read not run).** The save sentence for Windows also says: the new file gets the access list
  its directory gives it, not the target's own, and the temporary file is not owner-only while it is written.
  Marked as read, not measured; what follows for confinement is noted in T-074.
- **c5 (finding 5, test gap).** The decision "is this still the root" for Windows goes through one small function
  that takes the result of reading the identity and the held identity and answers inside only when the read
  succeeded and the two are equal; it is written without a platform cfg (generic over the identity type) and has
  a unit test that runs on every platform, in which a failed read answers "outside": the reviewer's mutant W3
  then fails on Linux. The Windows code calls it; the Unix lines stay as they are.
- **c6, c7 (nits).** The doc comments that say "(device, inode)" name the platform; the module doc says the whole
  attribute word is copied on Windows and a read-only target is refused before that.
- **c8 (ruling z3 completed, from the reviewer's table).** Of the 15 entries: 9 get `task = "T-074"`
  (`symlink_at_temp_name_is_not_followed`, `replace_swaps_inode_keeps_permissions_and_follows_symlinks`, the four
  root-replaced/removed/retargeted tests, `link_target_with_trailing_slash_requires_a_directory`, both
  `check_location` tests); 3 become `counterpart = "none"` with the reason that Windows has no mode bits
  (`temp_file_is_created_owner_only_and_gets_target_mode_before_rename`,
  `save_creates_temp_owner_only_for_an_existing_target`, `new_file_gets_default_mode`); of the 3 that are a Windows
  form of the same check, `read_only_target_is_refused_unless_root` names its counterpart now
  (`crates/kiln-workspace/tests/guard_t073.rs::acw2_save_of_read_only_target_is_refused_and_changes_nothing`, if the
  tool accepts it as covering Windows: say what `no-skip` answers) and the other two stay `pending` on T-092.
- After the rework: a second join, CI round 2 on `ci/candidate` (the new unit test and the registry on three
  platforms), review round 2.

## Log
### director
<!-- end director log -->
- 2026-11-18 **CI round 1: the pending CI criterion is met; the first Windows runtime evidence of the project** (record
  `tasks/ci-runs/T-073-round-1.md`, from the six artefacts, run R-344): run 44268710587 on `ci/candidate` at the
  joined head `786c592b21740de950ee0bd9576a9f07ec7884c8`. `gate-linux`, `gate-macos` and the two counted window jobs
  success. On Windows: `kiln-workspace` compiles; **the reported step passes 46 of 46** (lib 35, `acceptance_t073` 1,
  `guard_t073` 10); the gate's fmt, layers, ui-commands, hygiene, schema (for the first time) and no-skip
  (`35 platform test(s) registered (23 pending)`) PASS; clippy, scorecard and test FAIL only at other crates' test
  targets (T-092: `kiln-cli` `acceptance_t019_cli`; `kiln` `script_t055`, `acceptance_t055`, `window_t065`,
  `acceptance_t065`), with no error in any library or binary; the scorecard line went from 18 tests ran and 199
  problems to 158 tests ran and 60 problems. **Measured on Windows** (the guards' `MEASURED:` lines): renaming
  the root while it is held succeeds (the identity check then answers "outside", as the guard asserts);
  renaming the root's parent is refused, `PermissionDenied`, raw OS error 5. `window-probe (windows-2025)`: "Build
  kiln" succeeds for the first time; the probe exits 2 with `kiln: no window: egui_glow: OpenGL: egui_glow
  requires opengl 2.0+.` — the hosted Windows runner offers no OpenGL 2.0, which is T-077's matter (noted there).

- 2026-11-18 **CI round 2: the pending CI criterion is met on the final candidate** (record
  `tasks/ci-runs/T-073-round-2.md`, made by a fresh helper from `tasks/ci-runs/README.md`, run R-347): run 44279298877
  on `ci/candidate` at the joined head `5858927aacfedc2d3973ee115a41687a9637d80b`. The four counted jobs success
  (`gate-linux` 1147 passed, `gate-macos` 1130 passed, 0 failed). Windows: the reported step passes 47 of 47 (lib
  36 with the new unit test `ok`, `acceptance_t073` 1, `guard_t073` 10), the same two `MEASURED:` lines as round 1;
  `no-skip` prints `35 … (19 pending)` on all three platforms; fmt, layers, ui-commands, hygiene, schema and
  no-skip PASS; clippy, scorecard and test FAIL at test targets of other crates only (no library, no binary, not
  `kiln-workspace`'s lib). The scorecard's list names one run-time failure on Windows,
  `kiln_workspace::acceptance_t002::disk_save_replaces_via_rename_without_leftovers`: the test T-071 left red on
  Windows by design (it fails there naming why, until the product test files are done for Windows): T-092's,
  noted there. The probe: `kiln` builds, exit status 2, no OpenGL 2.0 on the runner (T-077).

### planner
- 2026-11-15 · R-255 · draft · did: filed from ADR-0009 · next: director
- 2026-11-18 · R-336 · draft · did: re-specified in full under v1.5; prototype probed (scratch, deleted) · not run: CI, Windows runtime · next: director
<!-- end planner log -->
### implementer
- 2026-11-18 · R-338 · in_progress · criteria restated: AC-1 the lib of `kiln-workspace` checks for
  `x86_64-pc-windows-msvc` with 0 errors (base 6, measured again here); AC-2 every library and binary of the
  workspace is clippy-clean with `-D warnings` for that target; AC-3 the lib test target of `kiln-workspace`
  (`--lib --profile test`) checks there with 0 errors (base 34); AC-4 `Cargo.lock` changes by exactly the line
  `+ "same-file",` in `kiln-workspace`'s list, no version; guards: every existing `kiln-workspace` test passes on
  Linux, every product change is under `#[cfg(windows)]` or is an added `#[cfg(unix)]` on an unchanged line;
  AC-W1–W3 come from the CI record (pending). Rulings D-245 acknowledged and accepted: Q-1 A (one reported
  Windows step in `platforms.yml` and its expectation in `acceptance_t070.rs`, now), Q-2 (registry mine; the test
  author uses no platform cfg; I add an entry only for a platform-only test it names), Q-3 A, Q-4 A. C compiler
  stub for cross checks in scratch (`clang --target=<t> -c` of an empty unit, `AR_<t>` = `llvm-lib`/`llvm-ar`),
  as ADR-0009 probe a3 · next: product code
- 2026-11-18 · R-338 · blocked · did: product code (`disk.rs`: the `std::os::unix` import, `root_id` and
  `_root_handle` and their initialisers and the `(dev, ino)` comparison each gain `#[cfg(unix)]` on unchanged lines;
  `#[cfg(windows)] root_handle: same_file::Handle` from `Handle::from_path(&resolved)` and compared in
  `current_root`; `create_temp` unchanged under `#[cfg(unix)]` plus a `#[cfg(windows)]` twin without `.mode`;
  module docs gain an "On Windows" paragraph and line 1 no longer says "Unix only"); `same-file = "=1.0.6"` under
  `[target.'cfg(windows)'.dependencies]`; 15 tests and 10 helpers of the test module under `#[cfg(unix)]` (names
  unchanged) with 15 registry entries (`unix`, `pending` T-092, 2026-12-31); `paths.md` "The root" Windows
  paragraph (held-root behaviour marked unchecked) and the T-074 scope sentence; the Windows save sentence in
  `control-protocol.md`; the reported Windows step and `t073_gate_windows_reports_the_workspace_lib_and_t073_tests`
  in `acceptance_t070.rs`, changed after ruling y2 (D-247, accepted) to `--test acceptance_t073 --test guard_t073 --
  --show-output` (fails on the D-245 form and on base). Local criteria (C compiler stub in scratch): AC-1 0 errors;
  AC-2 clean; AC-3 0 errors (28 before the test-module step); AC-4 `+ "same-file",` only; macOS
  `check --workspace --all-targets` and `clippy --all-targets -D warnings` for aarch64-apple-darwin exit 0. Main's
  tool (`ffaace9`, export, one build) on this tree: no-skip PASS `35 platform test(s) registered (23 pending)`,
  hygiene, scorecard --check, layers, ui-commands, schema PASS. Gate attempts: 1, FAILED at `test`: 1108 passed, 2
  failed, both in `xtask/tests/acceptance_t079.rs` (`ac1_registry_entries_are_exactly_the_twenty_of_interface_2_and_2a`,
  `ac1_pending_entries_name_their_task_and_expire_on_2026_12_31`): they pin the registry to exactly T-079's 20 entries
  and 8 pending ones, so the 15 entries Interface (5) requires fail them by construction. no-skip, fuzz, budgets not
  run by the gate after the failure (no-skip PASS when run alone).
  BLOCKED: QUESTION: Interface (5) needs registry entries, and main's `acceptance_t079.rs` (not in `touches`, an
  `acceptance_t*` file I may not edit) asserts the registry holds exactly its 20. Who changes those two tests, and
  how (for example to "the 20 of T-079 are present", as the implementer brief asks for things that grow)?
  QUESTION: `control-protocol.md`: `apps/kiln-cli/tests/acceptance_t008_cli.rs:566` pins the save bullet with
  "(on Unix)" verbatim, so I kept it and appended the Windows sentence rather than replacing "(on Unix)"; confirm.
  Note: the 15 counterparts are `pending` T-092 as Interface (5) allows; the symlink-walk ones may belong to T-074,
  and the read-only and replaced-root ones could name tests of `guard_t073.rs` after the join.
  Not run: anything on Windows; no CI-REQUEST, because the gate is not green · next: director ruling on the two
  questions, then gate attempt 2 and CI-REQUEST
<!-- end implementer log -->
### test author
- 2026-11-18 · R-339 · started · base `7de171b` · rulings acknowledged: Q-1 A accepted (this file is what the
  reported Windows step runs: the Windows runtime evidence of AC-W2); Q-2 accepted — it overrides Interface (4)'s
  "`#[cfg(windows)]`, registered `windows`": the file has no platform cfg, every test runs on Linux, macOS and Windows,
  the expectation is chosen at run time with `cfg!(windows)` (never an early return), so no registry entry is needed;
  Q-3 A, Q-4 A accepted (no test of mine depends on them).
- QUESTION (fail on base): the only criterion this file serves is AC-W2 (CI, pending). Interface (4) asks for tests
  of Windows behaviour whose Unix counterparts already hold on Linux and macOS ("Unix unchanged"), so on Linux base
  (branch `task/T-073-tests` = `7de171b` + this file) every test passes by design; on Windows base the file does not
  build (the crate's lib has 6 errors, `disk.rs:86,189,213,361`), so there "fails on base" is `compile` only. The
  spec assigns no criterion to this file that fails on base on Linux (AC-1–AC-4 are the implementer's local commands;
  `guard_t073.rs` is ruled unnecessary). Proposed: the join records AC-W2's base evidence as `compile` on
  `x86_64-pc-windows-msvc` (AC-1's 6 errors) and accepts that `acceptance_t073.rs` passes on Linux base, as the
  pending CI criterion of §8.1; or the file is renamed `guard_t073_windows.rs` (a path outside `touches`, so a
  ruling). I added no test that fails on base by construction (for example by reading the manifest), since the spec
  does not assign one.
- 2026-11-18 · R-339 · tests written · `crates/kiln-workspace/tests/acceptance_t073.rs`, 10 tests, no platform cfg
  (Q-2); the expectation differs only by `cfg!(windows)` in a branch or match guard, both sides asserting (no early
  return). Criterion → tests → result on Linux base (`cargo test -p kiln-workspace --test acceptance_t073`, as uid 0
  and, on a scratch copy, as uid 65534):
  - AC-W2 / Interface (1) root accepted and used: `acw2_existing_root_is_accepted_and_used` — pass (guard on Unix);
    refused as now: `acw2_missing_root_is_refused` (`NotFound (root)`), `acw2_file_as_root_is_refused`
    (`Io NotADirectory (root)`) — pass.
  - AC-W2 / Interface (1) identity, root path replaced by another directory: `acw2_root_replaced_at_same_path_is_outside_root`
    (`root` renamed away, new `root` made), `acw2_root_parent_replaced_is_outside_root` (`p` renamed away, new `p/root`) —
    pass. Unix: the rename must succeed, then `read`, `is_file`, `write_atomic`, `list_dir`, `check_location` and the
    save of a document opened before are `OutsideRoot`, nothing changes, and after moving back the save works. Windows:
    either the same, or (spec's other outcome) the rename is refused — then a control directory nobody holds was
    renamed in the same place first, and the session must still read and save on its root. The outcome is printed as
    `MEASURED: <test>: <platform>: rename of <dir> while the root is held: ok | refused: kind …, raw OS error …`.
  - AC-W2 / Interface (3) save: `acw2_save_replaces_target_by_rename_and_leaves_no_temp` (a hard link to the old
    target keeps the old bytes), `acw2_save_of_new_file_creates_it_and_leaves_no_temp`, `acw2_save_creates_temp_exclusively`
    (decoys at `.kiln-tmp-0`/`-1` untouched), `acw2_save_keeps_writable_target_writable`,
    `acw2_save_of_read_only_target_is_refused_and_changes_nothing` (where the read-only attribute stops writes —
    Windows always, Unix unless uid 0 — `Io PermissionDenied` naming `f.txt`, nothing changed, document dirty; as uid 0
    the save completes and keeps read-only) — pass.
  - Fail on base: none on Linux or macOS (by design, see the QUESTION above); on Windows base: `compile` (the lib's 6
    errors; `cargo check -p kiln-workspace --test acceptance_t073 --locked --target x86_64-pc-windows-msvc` → 6 `error[`).
  Self-check (Linux, scratch copy of `disk.rs`, never committed; killed by, as uid 0 / as uid 65534): M1 target
  written in place instead of renamed → `…replaces_target_by_rename…` / same; M2 temp copied and left behind → 8 tests /
  7; M3 root identity check always true → both `…replaced…` tests / same; M4 temp created with truncate instead of
  `create_new` → `…creates_temp_exclusively` / same; M5 write-protection check removed → survives as uid 0 (expected:
  permission bits do not apply) / `…read_only_target…`; M6 target permissions not copied → `…read_only_target…` /
  survives (only observable when the read-only target is writable to the process; Unix modes are covered by
  `acceptance_t002_r2`); M7 refusal of the target with the wrong kind (`NotFound`) → survives as uid 0 / `…read_only…`;
  M9 target never replaced (temp removed) → 8 / 7. Every mutant is killed on at least one identity.
  Windows type check: on base the lib does not build there, so on a scratch copy with the lib's Unix lines stubbed
  (not the product, not committed) `cargo clippy -p kiln-workspace --test acceptance_t073 --locked --target
  x86_64-pc-windows-msvc -- -D warnings` is clean; `cargo check … --target aarch64-apple-darwin` clean. Windows
  expectations: not run (no Windows here).
- QUESTION (measurement channel, for the implementer's step): the held-root measurement of AC-W2 is printed on the
  test's captured stdout, which libtest shows for a passing test only with `-- --show-output` (in the "successes"
  block, after the result lines). Writing it to stderr directly was tried and dropped: the workflow's `2>&1` merge
  split a `test <name> ... ok` line on Linux. Proposed: the reported Windows step runs `cargo test --locked
  --no-fail-fast -p kiln-workspace --lib --test acceptance_t073 -- --show-output`, so the artefact carries the
  `MEASURED:` lines; otherwise the measurement can be read only from which branch passed, which the result line does
  not show.
- Registry: none needed (no platform cfg in the file; Q-2). Not covered: the root removed (not renamed) while held —
  Interface (4) names only the replacement; if Windows refuses both renames, no test of this file reaches the
  identity comparison on Windows (M3 would survive there), which the `MEASURED:` lines will show.
- Gate on the committed state: `cargo clippy -p kiln-workspace --tests -- -D warnings` clean, `cargo fmt --check`
  clean, `cargo xtask no-skip` PASS (20 registered, unchanged), `cargo xtask hygiene` PASS. `cargo xtask ci` not run
  (my branch changes one test file; the steps above are its share). next: director (two QUESTIONs).
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
