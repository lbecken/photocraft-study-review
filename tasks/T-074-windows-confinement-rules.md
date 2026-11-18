---
id: T-074
title: "On Windows, refuse every reparse point on the walk as outside the root, state the Windows walk and root in the documents, and give the nine pending disk.rs tests their Windows counterparts"
tier: high              # METHODOLOGY v1.5 §4: the confinement boundary (disk.rs walk); gate paths (xtask/platform-tests.toml, .github/workflows/platforms.yml, xtask/tests/); the approval row must say `gate paths`. No dependency change.
state: draft
owner_role: planner
scorecard_items: []     # none claimed: the ws.* items are verified on Linux; per-platform confirmation is the evaluator's (ADR-0009 Decision 3)
depends_on: [T-073, T-072]   # T-073 merged (8bf4c3a); T-072: shares platforms.yml and acceptance_t070.rs (Q-2) — not merged when this spec was written
touches:                # one role per path (D-153); proposed with the cut of Q-1 (lexical names and case are T-095's)
  - crates/kiln-workspace/src/disk.rs        # implementer: the walk's refusal (Interface (1)), list kinds (2), and its #[cfg(test)] module: the Windows counterparts of Interface (4)
  - xtask/platform-tests.toml                # GATE PATH: implementer: exactly the 9 entries naming T-074 (Interface (4))
  - docs/protocol/paths.md                   # implementer: the Windows sentences of Pinned (i), "Location" to "Residual limits"
  - docs/protocol/project.md                 # implementer: one Windows sentence on the kind of a reparse point (Interface (2))
  - docs/control-protocol.md                 # implementer: exactly the sentence "How symlinks are followed there is not yet stated (T-074)."
  - .github/workflows/platforms.yml          # GATE PATH: implementer: one new reported step in gate-windows (Interface (5))
  - xtask/tests/acceptance_t070.rs           # GATE PATH: implementer: one new test fn for that step; no existing assertion changed
  - crates/kiln-workspace/tests/guard_t074.rs   # test author: the Windows runtime tests (Interface (6)); pass on Unix base
  - xtask/tests/acceptance_t074.rs           # GATE PATH: test author: the registry criterion AC-3 (fails on base everywhere)
budget_tokens: 1100000  # high (v1.5); estimate about 800,000 for this part (see Estimate; the uncut task about 1,300,000)
iteration: 2
found_by: planner       # ADR-0009 (R-255); re-specified under v1.5 (this run)
origin_task: null
---

# T-074 Windows: reparse points on the walk, the Windows root cases, the nine pending counterparts

Re-specified in full under v1.5 against main `72522f2` (code as at T-073's merge `8bf4c3a`: `git diff --stat 8bf4c3a
72522f2 -- crates apps xtask .github docs/protocol` is empty). **Proposed cut (Q-1):** this file keeps the walk and
reparse points (`disk.rs`); Windows-meaningful names (`:` and streams, drive prefixes, reserved device names,
trailing dot or space, `<>"|?*`) and case go to the new draft **T-095** (`tasks/T-095-windows-path-names-and-case.md`,
`path.rs`). Every Windows behaviour below that no run has shown is marked **unchecked** and is measured first by the
planner's probe (CI-REQUEST in the planner log; sentences are rewritten from its record before `specified`).

## Goal
On Windows the walk runs the Unix code (`disk.rs:272-375`): it follows what `std` calls a symlink (file and directory
symlinks and junctions, the name-surrogate reparse tags) "as the kernel does", turning an absolute target's drive or
`\\?\` prefix into "restart from `/`" (`disk.rs:300-302`, `:335-337`), and it does not see other reparse points at all
(read in `reviews/T-073-r1.md`, not run). `docs/protocol/paths.md:50-51` says the rules below "The root" are not yet
stated for Windows, and nine Unix-only tests of `disk.rs` wait for Windows counterparts (`xtask/platform-tests.toml:
238-380`, `task = "T-074"`, expiring 2026-12-31). After this task, on Windows: **every reparse point the walk meets is
refused as outside the root** (ADR-0009 ruling Q-6 B, D-175), the documents state the Windows walk, the Windows forms
of the root cases, and each of the nine entries names its Windows counterpart; every such sentence has a test that
runs on the `windows-2025` runner. Linux and macOS behaviour is unchanged.

Gate paths in `touches`: `xtask/platform-tests.toml`, `.github/workflows/platforms.yml`, `xtask/tests/acceptance_t070.rs`,
`xtask/tests/acceptance_t074.rs`.

Reading list: ADR-0009 (Decision 4, ruling Q-6 B); `docs/protocol/paths.md` (all), `docs/protocol/project.md` "kind";
`docs/control-protocol.md` "Paths"; `crates/kiln-workspace/src/disk.rs` lines 1-610; `xtask/platform-tests.toml:222-380`;
`tasks/T-073-*.md` (rulings y1, c1-c8), `reviews/T-073-r1.md` "The Windows code"; `tasks/ci-runs/T-073-round-2.md`; the
probe record (`tasks/ci-runs/T-074-probe-1.md`, once written); METHODOLOGY v1.5 §5.7, §6, §8.1.

## Notes from T-073 (director, D-250), kept; checked in this run
- Nine registry entries wait on this task (ruling c8): verified, `xtask/platform-tests.toml:238-380`, each `unix`,
  `pending`, `task = "T-074"`, `expires = "2026-12-31"`; Interface (4) disposes of each.
- Measured in T-073's CI round 1: the held root can be renamed on Windows (calls then answer `OutsideRoot`); renaming its
  parent is refused (OS error 5). Not tested: removing a held root; the identity check after a replaced parent.
  Verified against `tasks/ci-runs/T-073-round-1.md` and `-round-2.md`; the removal is probe F.
- Read, not run (`reviews/T-073-r1.md`): `..` removed by text against `canonicalize` on disk — **possibly wrong for
  Windows**: `canonicalize` opens a non-verbatim path, which Win32 normalises by text first, so both may agree (INFERRED;
  probe K decides); junctions counted as links (probe B); `:` reaching streams and drive-relative `C:x`, case and reserved
  names (T-095; probe I); the save's access list (Q-7); an identity read error answering "outside" (Interface (3), probe J).
- Wrong or stale in the draft: the line references (S-1); the draft's `touches` lacked the registry its own note puts in
  `touches`; "device names reach devices" (ADR-0009 (b)) is doubtful for the walk, whose paths are `\\?\` verbatim paths
  from `canonicalize` (`reviews/T-073-r1.md`: with that prefix `CON` is an ordinary name; INFERRED, probe I).

## Interface (behaviour on Windows; Linux and macOS unchanged)
### (1) The walk refuses every reparse point
- On Windows, each entry the walk reaches below the root — every component of the request path and of nothing else,
  the last one included — is examined without being followed (`fs::symlink_metadata`). If its attributes carry the
  reparse-point attribute (`FILE_ATTRIBUTE_REPARSE_POINT`, `0x400`: file and directory symlinks, junctions, and every
  other reparse point), the call is refused with `OutsideRoot` before anything there or beyond it is opened, created,
  replaced or removed, whatever the reparse point leads to (inside the root, outside, nowhere). Exactly: on Windows the
  walk follows no link, and its location is the root joined with the request's segments.
- This applies to `read`, `write_atomic`, `is_file` (false), `list_dir` (a reparse point as the listed directory, or on
  the way to it) and `check_location`, and so to `doc.open` (also of a path already open), `doc.save`, `project.tree`
  and `nav.workspace-symbols`. Precedence and messages as now: `invalid_path`, `outside_root`, `not_found`,
  `not_a_file`, `io_error`; a component that does not exist ends the walk as now (`not_found`), so a reparse point
  after a missing component is never reached.
- The save's temporary file: a reparse point at a candidate name `.kiln-tmp-N` is never followed or changed; the save
  uses the next free name (`create_new`, as now; **unchecked**, probe H).
- Not on the walk, unchanged: the root (`--root` given as a symlink or junction resolves, at start, to the directory it
  leads to, which is the root, as on Unix; **unchecked**, probe E) and the directories above it.
### (2) Kinds in a listing
On Windows a directory entry whose own attributes carry the reparse-point attribute is listed with kind `symlink` (never
`file` or `dir`), so `project.tree` lists it and never descends into it (`docs/protocol/project.md:57,79`). For symlinks and
junctions this is what `std` reports already (**unchecked**, probe B3/C7); for other reparse tags it is read, not run (no
such entry can be made on the runner by a test; Q-4).
### (3) The root cases on Windows (each sentence follows the probe record; Pinned (i))
- `--root` given as a junction: its target is the root; the junction retargeted while a session runs → every call
  `outside_root`; pointed back → the session works again (probe E).
- The root renamed away and a junction to another directory put at its path → `outside_root`, nothing read or written
  in the other directory (probe G).
- The held root removed (empty, and with a file in it) while a session runs: what Windows allows, and what the session
  answers then and after a new directory is made at the path (probe F).
- The list right of the held root denied while a session runs: Windows answers `outside_root` to every call (T-073 r1
  finding 5, read not run), where Unix goes on; once the right is restored the session works again (probe J).
- `..` after a link in the `--root` text: whether Windows resolves it by text (probe K); stated only if it differs from
  Unix.
### (4) The nine pending registry entries (`disk.rs::tests::…`, all `platforms = "unix"`; the Unix tests stay as they are, names and `#[cfg(unix)]` unchanged)
Each becomes `counterpart = "<file>::<item>"` (no `reason`, `task`, `expires`), naming a test that runs on Windows:

| # | Entry (`tests::`) | Windows counterpart |
|---|---|---|
| 2 | `symlink_at_temp_name_is_not_followed` | new test of `disk.rs`'s module: a junction (and, if probe A1 shows a file symlink can be made, a file symlink) at `.kiln-tmp-0` is not followed or changed; the save uses the next name |
| 8 | `replace_swaps_inode_keeps_permissions_and_follows_symlinks` | new test: a save through a file symlink or a junction is `outside_root`; link, its target and the directory unchanged (the inode part has its Windows form in `guard_t073.rs::acw2_save_replaces_target_by_rename_and_leaves_no_temp`) |
| 9 | `root_replaced_by_a_symlink_between_requests_is_outside` | new test: Interface (3), second bullet |
| 10 | `parent_replaced_by_a_symlink_between_requests_is_outside` | `crates/kiln-workspace/tests/guard_t073.rs::acw2_root_parent_replaced_is_outside_root` (exists, no platform cfg; on Windows it shows the parent's rename refused, measured in run 44268710587) |
| 11 | `root_removed_and_replaced_by_a_link_to_a_new_directory_is_outside` | new test: Interface (3), third bullet |
| 12 | `root_given_through_a_link_that_is_retargeted` | new test: Interface (3), first bullet |
| 13 | `link_target_with_trailing_slash_requires_a_directory` | the new test of #8 or #14 that shows a directory link refused (on Windows no link is followed, so a target's trailing `/` has no Windows form) |
| 14 | `check_location_is_ok_for_every_inside_location` | new test: `check_location` is `Ok` for every inside kind Windows has (file, directory, nothing, missing parent) and `outside_root` for a junction or symlink at any component |
| 15 | `check_location_refuses_outside_loops_and_a_replaced_root` | new test: a junction to outside, a junction to inside, a junction loop and a replaced root are each `outside_root` |

The new tests are the implementer's, in `disk.rs`'s `#[cfg(test)]` module, **with no platform cfg on the test fn**: each
asserts the Unix behaviour on Unix and Interface (1)/(3) on Windows, chosen with `cfg!(windows)`, both branches asserting
(METHODOLOGY §8.1). A helper that makes a link may be defined per platform (Unix `symlink`; Windows a junction, made with
`std::os::windows::fs::symlink_dir` if probe A2 shows it works without a privilege the runner lacks, else `cmd /C mklink
/J` under a timeout), each form doing its work (`no-skip`'s `inert-helper` rule). They run on Windows in T-073's existing
reported step (`--lib`).
### (5) CI evidence
One new step in `gate-windows`, after the gate and before the upload, reported and never required (`set +e` … `exit 0`,
`if: always()`), separate from T-073's step: `cargo test --locked --no-fail-fast -p kiln-workspace --test guard_t074 --
--show-output` into a `.txt` file the upload includes. Its expectation is one new test fn in `xtask/tests/acceptance_t070.rs`
of the form of `t073_gate_windows_reports_the_workspace_lib_and_t073_tests` (`:1210-1277`). T-073's step and its test are
not changed (main's `acceptance_t070` then still passes: measured, Spec check S-4).
### (6) Tests (test author)
- `crates/kiln-workspace/tests/guard_t074.rs`: through the Rust API (`DiskFs`, `Vfs`, `Document`), no platform cfg in the
  file (ruling Q-2 of T-073, D-245): for each kind the runner can make (file symlink, directory symlink, junction; inside,
  outside, dangling, relative and absolute target; at the last and at a middle component), `read`, `write_atomic`,
  `is_file`, `list_dir`, `check_location` and a `Document` save answer Interface (1) on Windows and today's behaviour on
  Unix; listing kinds (2); the root cases (3). A test of a case the runner cannot set up fails naming why (§8.1), never
  passes; the outcome of each root case is printed as one `MEASURED:` line as in `guard_t073.rs`.
- `xtask/tests/acceptance_t074.rs`: AC-3 (reads `xtask/platform-tests.toml` as text, as `acceptance_t079.rs` does).

## Pinned
- (a) Errors: no new kind; a reparse point is `VfsError::OutsideRoot` (`rejected` / `outside_root`, message `cannot open
  <path>: resolves outside the root` / `cannot save …`, as now). Precedence as now.
- (b) Files: as now (`.kiln-tmp-N`). Registry entry layout as `xtask/platform-tests.toml:9-18`.
- (c) Order: lexical check, root check, walk (the reparse test at each component, before following or descending), type,
  open — as now, with the new test inside the walk.
- (d) Platform: Windows x86_64 MSVC (CI `windows-2025`), NTFS (probe I9 records the volume's file system). Unix: no line
  used on Linux or macOS changes except by an added `#[cfg(unix)]`, as T-073 (its Q-3 A).
- (e) Threat model: a hostile project directory (cloned repository, archive) may hold symlinks, junctions or other reparse
  points; the attacker cannot change the tree during a request (the residual limit stays). Refusing every reparse point
  is fail-safe: nothing beyond one is opened. Not defended (unchanged): hard links; changes during a request; the root
  and the directories above it being reparse points (resolved at start); the ReFS identity limit (T-094). Usability cost,
  accepted by Q-6 B: a project whose files are reparse points (KNOWN, not measured: cloud placeholder files of a synced
  folder, deduplicated files) cannot be opened (Q-6).
- (f) Tests that pin what this task changes: none on Linux (no Linux behaviour changes). `xtask/tests/guard_t079.rs` pins
  every Linux test name of the base (1,121; `:515-516`), among them all 22 of `disk.rs` (`:1241-1262`): none renamed,
  removed or given another cfg, and `failed_rename_removes_own_temp_and_keeps_target` keeps no platform cfg (`:506-511`).
  The 8 entries quoting `paths.md` (`platform-tests.toml:310-376`) need their quoted sentences kept word for word.
- (g) Worked examples (probe record to be read; expectations from reading): `junction j -> <root>/d`, file `<root>/d/x`:
  base `read j/x` → `OutsideRoot` already (the junction's `\??\` target becomes "restart from `/`", r1), after → `OutsideRoot`
  by (1); relative file symlink `sr -> f.txt`: base `read sr` → `Ok` (followed), after → `OutsideRoot` — the case that
  fails on base on Windows (needs probe A6: a file symlink can be made on the runner).
- (h) Evaluator: none (no item claimed).
- (i) Document sentences (worded as the implementer copies them; each **unchecked** until the probe record, then to be
  shown by `guard_t074.rs` on CI):
  - `paths.md`, replacing `:50-51`: "On Windows (T-074) the rules below hold with these differences, each shown by
    `crates/kiln-workspace/tests/guard_t074.rs` on the `windows-2025` runner: no link is followed. Every entry on the way
    that is a reparse point — a symbolic link to a file or a directory, a junction, or any other reparse point — makes
    the path `outside_root`, wherever it leads, and nothing beyond it is read or written. Names that Windows gives a
    meaning and case are T-095's." (T-095's clause goes when T-095 merges.)
  - `paths.md` "The root", after the T-073 bullets: the probe's sentences for (3) — removal, junction as `--root`,
    replaced by a junction, list right denied (each with the run that measured it).
  - `paths.md` "Residual limits": "On Windows a project whose files are reparse points (for example cloud placeholder
    files of a synced folder) cannot be opened: they are refused as `outside_root` (read, not measured)." `- Unix only.`
    stays for T-093.
  - `project.md` (after `:79-80`): "On Windows every entry that is a reparse point is listed with kind `symlink` and never
    descended (T-074)."
  - `control-protocol.md:141`: "How symlinks are followed there is not yet stated (T-074)." → "On Windows no link is
    followed: a save through a symbolic link or junction is refused (`outside_root`), see `docs/protocol/paths.md`." The
    bullet's first sentence ("… (on Unix); through a symlink, the link's target is replaced and the link is kept.") stays
    verbatim (`apps/kiln-cli/tests/acceptance_t008_cli.rs:560-566`).
- (j) **Main's gate on the candidate tree (D-235)**: see Spec check S-3.

## Acceptance criteria
Local (Linux; implementer, re-run by the reviewer):
- AC-1: every existing `kiln-workspace` test passes on Linux and macOS unchanged (the 28 of `ws.path-confinement` among
  them); every changed product line is under `#[cfg(windows)]` or adds `#[cfg(unix)]` to an unchanged line (reviewer's
  hunk table, as T-073 Q-3 A); the Linux and macOS lib IR equals base's after normalising (reviewer, as T-073 r1).
- AC-2: `cargo clippy -p kiln-workspace --lib --profile test --locked --target x86_64-pc-windows-msvc -- -D warnings` and
  `cargo clippy --workspace --lib --bins --locked --target x86_64-pc-windows-msvc -- -D warnings` clean (no C compiler
  needed for `kiln-workspace`: measured in this run; a stub as T-073 for the workspace).
- AC-3 (`acceptance_t074.rs`, fails on base everywhere by assertion): for each of the nine (file, item) pairs of Interface
  (4) the registry holds exactly one entry, with `platforms = "unix"`, `counterpart` not `pending`, and no `task` or
  `expires`; for #10 the counterpart is exactly `crates/kiln-workspace/tests/guard_t073.rs::acw2_root_parent_replaced_is_outside_root`;
  and no entry has `task = "T-074"`. Variant it must reject: one of the nine left `pending` with `task = "T-095"`.
- AC-4: `cargo xtask no-skip` on the candidate prints `35 platform test(s) registered (10 pending)` (19 − 9; the new tests
  carry no platform cfg, so no entry is added). Variant to reject: a counterpart naming a `#[cfg(unix)]` test (no-skip
  `platform-counterpart`).
Through CI (pending criterion, §8.1; the join records it as pending):
- AC-W1: the new step's output names every `guard_t074` test `ok`, with its `MEASURED:` lines; T-073's step names every
  lib test `ok`, among them each new counterpart of (4).
- AC-W2: `gate-windows` steps fmt, layers, ui-commands, hygiene, schema, no-skip PASS; clippy, scorecard, test FAIL only
  at test targets of other crates or files (T-092's), named in the record, none of `kiln-workspace`'s lib, `guard_t073`,
  `guard_t074`.

### The pending CI criterion (v1.5 §8.1), for the approval row
The record `tasks/ci-runs/T-074-round-N.md` (from the artefacts) of the rebased head on `ci/candidate` shows:
`gate-linux`, `gate-macos`, `window-linux`, `window-probe (macos-15)` green; AC-W1 and AC-W2 as stated; no-skip `35 …
(10 pending)` on all three; `window-probe (windows-2025)` as before (exit 2, no OpenGL). **Two CI rounds expected.**

## Out of scope and implementer notes
- Names Windows gives a meaning, and case (T-095); the 128-bit identity (T-094); the access list of a saved file (ruling
  c4 of T-073; Q-7); product test files outside `disk.rs`'s module (T-092); following links on Windows (ADR-0009 Q-6 A,
  a later decision).
- The test author's file has no platform cfg; if a platform-only test is unavoidable it names it in its report and the
  implementer adds the entry before the join (T-073 ruling Q-2, D-245).
- No implementation hint beyond the attribute named in (1); `std::os::windows::fs::MetadataExt::file_attributes` is std.

## Collisions and order
- `platforms.yml`, `acceptance_t070.rs`: **T-072** (specified, in re-check) → not in flight together (§5.2); Q-2.
- `docs/protocol/paths.md`: T-093 (specified, low; dispatched after T-074), T-094, T-095 (drafts) → after this task; each
  re-checks the sentences of (i). `docs/protocol/project.md`: T-093 → after. `disk.rs`: T-092, T-094, T-075 (drafts) →
  after. `xtask/platform-tests.toml`: T-092, T-095 → after.

## Estimate
T-073 (same file, platform and shape: about 60 product lines, a test module, an independent Windows test file, 2 CI
rounds) booked 860,977 (D-252). This part: about 20 product lines, seven new portable unit tests that make junctions,
nine registry entries, five document places, one workflow step and its expectation, a guard file and an xtask test, 2 CI
rounds: **about 800,000** (high ceiling 1,100,000). The uncut task (with T-095's names and case): about 1,300,000, over the
ceiling (Q-1). Arm: high → independent test author.

## Questions and rulings
Questions are in the planner's report to the director (Q-1 to Q-8) and below.
- **Q-1 (cut).** A: cut as above (T-074 the walk, T-095 names and case). B: one task, an `Extend` row. **Rec. A.**
- **Q-2 (order with T-072).** Both need `platforms.yml` and `acceptance_t070.rs`. A: T-072 first (specified, ~700,000),
  T-074 after it merges and re-checks (5). B: T-074 first. C: T-074 without a workflow step (independent tests not run
  on Windows until T-092). **Rec. A.**
- **Q-3 (who writes the counterparts).** A: the implementer, in `disk.rs`'s module (registry green on its branch; run by
  T-073's step). B: the test author, the implementer naming them after the join (one more commit and gate run).
  **Rec. A.**
- **Q-4 (list kind of other reparse points).** A: `symlink` (2). B: as `std` reports (a cloud-file directory would be
  `dir`, and `project.tree` would then be refused when it descends). **Rec. A.**
- **Q-5 (CLI test file).** The draft had `apps/kiln-cli/tests/acceptance_t074_cli.rs`. A: dropped (the CLI maps
  `OutsideRoot` the same on every platform; `kiln-cli`'s tests are T-076's on Windows). B: kept, a second step target.
  **Rec. A.**
- **Q-6 (cost of Q-6 B).** Reparse points that are not links (cloud placeholders, deduplicated files) are refused too.
  A: accept and state (Pinned (i)). B: refuse only name surrogates (links, junctions) and follow the others as files.
  **Rec. A**, as Q-6 B says, with the owner told.
- **Q-7 (access list, T-073 c4).** Not a location rule. A: out of scope; a draft for it when the owner asks. B: here
  (copy the security descriptor). **Rec. A.**
- **Q-8 (fail on base on Windows).** Windows runtime tests pass on Unix base (guards, as T-073 y1); which fail on Windows
  base is shown by the probe record (`read sr` followed on base). A: that record is the base evidence. B: one CI run of
  the tests branch at base. **Rec. A.**

### Spec check (planner, this run)
- **S-1 Blockers found and resolved in this spec:** the draft's `touches` lacked `xtask/platform-tests.toml` (its own
  notes require it) and any workflow path (without one no independent test runs on Windows before T-092); added, with
  Q-2. The draft's line numbers (`disk.rs:291`, `:236`) are T-073-stale: the prefix-to-`/` step is `:300-302`, `:335-337`,
  the inside test `:280`.
- **S-2 Shared with tasks in flight:** T-072 (`platforms.yml`, `acceptance_t070.rs`): overlap, Q-2. No trait, type or
  public signature changes (the `Vfs` trait and `VfsError` unchanged); no Rust-API criterion needs a new signature.
- **S-3 Main's tool on the candidate (D-235).** The task changes no file under `xtask/src/` and no fixture, so main's tool
  is the candidate's: `no-skip` reads the candidate registry and sources — PASS if each new counterpart test has no
  platform cfg and each per-platform helper does work (`35 … (10 pending)`); a counterpart naming a test the tool cannot
  find, or a quoted `paths.md` sentence of `platform-tests.toml:310-376` reworded, fails it. `hygiene`: PASS (no session
  path; review files as usual). `scorecard --check`: PASS (no item or listed test changes; the page is computed on Linux,
  where nothing changes). `layers`: PASS (no manifest or lock change; `Cargo.lock` is not in `touches`). `ui-commands`:
  PASS (no `kiln-ui` file). `schema`: PASS (no command or `docs/commands.json` change). No gate rule changes, so none is
  used by the tree. The builder runs main's six steps on the candidate before the join and logs their lines.
- **S-4 Main's tests that name a path in `touches` (D-249; `grep -rlE '<path>' --include=*.rs crates apps xtask` on
  `72522f2`):** `disk.rs` → `xtask/tests/guard_t079.rs` (base names; passes if no `disk.rs` test is renamed, removed or
  re-cfg'd: Pinned (f)). `paths.md` → `apps/kiln-cli/tests/acceptance_t008_cli.rs:576-578` (contains `outside_root`,
  `not_a_file`: passes); `disk.rs`, `vfs.rs` name it in comments. `control-protocol.md` → `acceptance_t008_cli.rs:549-570`
  pins the lexical bullet and the save bullet's first sentence verbatim (kept: passes); the other 15 files read other
  sections (error kinds, synopsis), not the sentence changed. `platform-tests.toml` → `xtask/tests/acceptance_t079.rs`
  (T-079's 20 entries only, none of the nine: passes); `acceptance_t071.rs`, `guard_t071.rs` write toy registries in
  scratch trees (not the file); the other hits are comments. `platforms.yml` → `acceptance_t070.rs`: **measured** — main's
  `acceptance_t070` (14) and `guard_t070` (7) pass on main plus a separate reported step in `gate-windows` (scratch
  clone, `cargo test -p xtask --locked --test acceptance_t070 --test guard_t070`: `14 passed`, `7 passed`); a step that
  edits T-073's command instead would fail `t073_gate_windows_reports_…` (it matches the command text). `acceptance_t070.rs`
  → `guard_t079.rs` pins its 14 names (add only), `guard_t070.rs` (comment). `project.md` → five files, all comments or
  `project.rs`'s own docs. New files: no main test names them.
- **S-5 Criteria of absence:** (1) "the walk follows no link" — variant to reject: a junction to a directory inside the
  root followed. AC-3 — variant: an entry moved to `T-095`. Interface (2) — variant: a junction listed `dir`.
- **S-6 Unmeasured, not chosen without the probe:** whether a file symlink and a directory symlink can be made on the
  runner (A1, A2, A6); what `std` reports for each kind (B); base behaviour (C); the root cases (E, F, G, J, K); the
  temporary name (H). The probe (planner log) measures all of them in one round.

## Log
### director
<!-- end director log -->
### planner
- 2026-11-15 · R-255 · draft · did: filed from ADR-0009 · next: director
- 2026-11-18 · re-specification under v1.5 (main `72522f2`; scratch clone `/home/user/kiln-plan-T074`, branch `probe`) ·
  did: read the methodology, T-073's spec, reviews and CI records, ADR-0009, the code and registry; cut proposed (Q-1,
  T-095 drafted); measured: `cargo clippy -p kiln-workspace --test probe_t074 --locked --target x86_64-pc-windows-msvc --
  -D warnings` clean with no C compiler stub; main's `acceptance_t070` 14 passed and `guard_t070` 7 passed with a separate
  reported step added to `gate-windows` (branch `proto`, not committed); the probe file passes on Linux (`PROBE: not
  windows`). Probe commit `de8106fe621e26ec907b6e1a2a3c068438888992` (parent `72522f2`): a temporary job `probe-t074` on
  `windows-2025` runs `crates/kiln-workspace/tests/probe_t074.rs` and uploads `probe-t074.txt`; the gate jobs on that commit
  are expected red at `no-skip` (the probe file is not registered), which does not matter. One read-only `git log` was run
  in the primary checkout by mistake; nothing else. Tier high (confinement boundary; gate paths). Pinned (h) n/a (no
  item). Not run: anything on Windows. `target/` of the clone deleted at the end; the clone is kept for the push.
  CI-REQUEST: de8106fe621e26ec907b6e1a2a3c068438888992 /home/user/kiln-plan-T074 · next: director (rulings Q-1 to Q-8;
  probe round; then this planner rewrites (3), (i) and (g) from the record)
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
