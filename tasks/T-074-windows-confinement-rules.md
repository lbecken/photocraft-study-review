---
id: T-074
title: "On Windows, refuse every reparse point on the walk as outside the root, keep saves working past links at temporary names, state the Windows walk and root in the documents, and give the nine pending disk.rs tests their Windows counterparts"
tier: high              # METHODOLOGY v1.5 §4: the confinement boundary (the disk.rs walk) and the persistence path (the temporary file); gate paths (xtask/platform-tests.toml, xtask/tests/); the approval row must say `gate paths`. No dependency change.
state: in_progress
owner_role: planner
scorecard_items: []     # none claimed: the ws.* items are verified on Linux; per-platform confirmation is the evaluator's (ADR-0009 Decision 3)
depends_on: [T-073]     # merged (8bf4c3a). T-072 runs beside this task (D-254); its merge is needed before this task's CI round, not before dispatch (pending CI criterion)
touches:                # one role per path (D-153); cut by D-254 ruling 1 (names and case are T-095's)
  - crates/kiln-workspace/src/disk.rs        # implementer: Interface (1)-(3) under #[cfg(windows)], and its #[cfg(test)] module: the counterparts of Interface (5)
  - xtask/platform-tests.toml                # GATE PATH: implementer: exactly the 9 entries naming T-074 (Interface (5))
  - docs/protocol/paths.md                   # implementer: the sentences of Pinned (i) for paths.md
  - docs/protocol/project.md                 # implementer: one Windows sentence on the kind of a reparse point
  - docs/control-protocol.md                 # implementer: exactly the sentence "How symlinks are followed there is not yet stated (T-074)."
  - crates/kiln-workspace/tests/guard_t074.rs   # test author: the Windows runtime tests (Interface (6)); pass on Unix base
  - xtask/tests/acceptance_t074.rs           # GATE PATH: test author: AC-3, the registry criterion (fails on base everywhere)
  - crates/kiln-workspace/tests/acceptance_t002.rs  # test author: the one test `save_through_symlink_updates_target` only (D-256 Q-5; Interface (7))
budget_tokens: 1100000  # high (v1.5); estimate about 880,000 (D-256; see Estimate)
iteration: 2
found_by: planner       # ADR-0009 (R-255); re-specified under v1.5
origin_task: null
---

# T-074 Windows: reparse points on the walk, links at temporary names, the root cases, the nine counterparts

Re-specified in full under v1.5 against main `72522f2` (code as at T-073's merge `8bf4c3a`: `git diff --stat 8bf4c3a
72522f2 -- crates apps xtask .github docs/protocol` is empty), with the rulings of D-254 and the planner's probe (run
44290286460 at `de8106fe621e26ec907b6e1a2a3c068438888992`, record `tasks/ci-runs/T-074-probe-1.md`; cited below as
"PROBE <id>", every such line as printed there on `windows-2025`, at base). Cut (D-254 ruling 1): this task is the walk and
reparse points in `disk.rs`; names Windows gives a meaning, streams, case and short names are **T-095**'s
(`tasks/T-095-windows-path-names-and-case.md`), which holds what the probe showed about them.

## Goal
On Windows the walk runs the Unix code (`disk.rs:272-375`): it follows what `std` reports as a symlink (file and
directory symlinks and junctions: PROBE B1, each `is_symlink=true`), turning an absolute target's `\\?\`/drive prefix into
"restart from `/`" (`disk.rs:300-302`, `:335-337`). Measured at base: a relative file link inside the root is followed
(PROBE C1 `read sr: Ok([102])`, C2 `check_location sr: Ok(())`); junctions and absolute links answer `OutsideRoot` by that
accident, also when they lead inside (PROBE C1, C3 `read j/x (file exists in d): Err(OutsideRoot …)`); and a save in a
directory with a junction and a file link at `.kiln-tmp-0`/`.kiln-tmp-1` fails (PROBE H2 `write_atomic f.txt: Err(Io { …
kind: PermissionDenied, message: "Access is denied. (os error 5)" })`, H3 target unchanged, `out` empty), where on Unix
a leftover at a temporary name only moves the save to the next name. `docs/protocol/paths.md:50-51` says the rules below
"The root" are not yet stated for Windows, and nine Unix-only tests of `disk.rs` wait for Windows counterparts
(`xtask/platform-tests.toml:238-380`, `task = "T-074"`, expiring 2026-12-31).

After this task, on Windows: **every reparse point the walk meets is refused as outside the root** (ADR-0009 ruling Q-6
B, D-175; D-254 ruling 6); a reparse point at a temporary name makes the save take the next name; the documents state
the Windows walk and the measured Windows root cases; each of the nine entries names a counterpart that runs on Windows.
Linux and macOS behaviour is unchanged.

Gate paths in `touches`: `xtask/platform-tests.toml`, `xtask/tests/acceptance_t074.rs`. (D-254 ruling 2: no workflow
edit; T-072's general reported Windows step reports this task's test file.)

Reading list: ADR-0009 (Decision 4, ruling Q-6 B); `tasks/ci-runs/T-074-probe-1.md` (all); `docs/protocol/paths.md`
(all), `docs/protocol/project.md:57,79-80`; `docs/control-protocol.md` "Paths"; `crates/kiln-workspace/src/disk.rs` lines
1-610; `xtask/platform-tests.toml:222-380`; `tasks/T-073-*.md` (rulings Q-2, y1, c1-c8); `reviews/T-073-r1.md` "The Windows
code"; `tasks/T-072-gate-green-on-macos.md` Interface (6), AC-9; METHODOLOGY v1.5 §5.7, §6, §8.1.

## Notes from T-073 (director, D-250), checked
- Nine registry entries wait on this task: verified (`xtask/platform-tests.toml:238-380`); Interface (5) disposes of each.
- Measured in T-073's CI round 1 (held root renamable, parent not, OS error 5): verified against both T-073 records.
  "Not tested: removing a held root": now measured (PROBE F1-F8), Interface (4).
- Read, not run, in `reviews/T-073-r1.md`, now measured: junctions counted as links (PROBE B1, C7); `..` in the root text
  "removed by text against `canonicalize` on disk" was **wrong for Windows**: both resolve it by text there (PROBE K2
  `canonicalize(link/../b): Ok("\\\\?\\…\\k\\b")`, `absolute(…)`: `…\k\b`), so the root is the `b` beside the junction
  (K4 `read f.txt: Ok("lexical")`), not, as on Unix, the one beside its target; Interface (4). Streams, `C:x`, reserved
  names, case: T-095 (PROBE I). The save's access list: out of scope (D-254 ruling 7). An identity read error answering
  "outside" (finding 5): **not measurable on the runner** — denying the list right with `icacls` changed nothing there
  (PROBE J2 `read f.txt: Ok(1)`, J3 `DiskFs::new(root) now: Ok(())`; INFERRED: the runner's user bypasses it); no
  sentence here, the T-073 text stays.
- Stale in the draft: line references (`disk.rs:291`, `:236` → `:300-302`/`:335-337`, `:280`); the draft's `touches` lacked
  the registry; "device names reach devices" (ADR-0009 (b)) is false for the walk: PROBE I1-I3 (T-095).

## Interface (behaviour on Windows; Linux and macOS unchanged)
### (1) The walk refuses every reparse point
- Each entry the walk reaches below the root — every component of the request path, the last one included — is
  examined without being followed (`fs::symlink_metadata`). If its attributes carry the reparse-point attribute
  (`FILE_ATTRIBUTE_REPARSE_POINT`, `0x400`), the call is refused with `OutsideRoot` before anything there or beyond it is
  opened, created, replaced or removed, whatever the reparse point is (file or directory symlink, junction, any other
  reparse point) and wherever it leads (inside, outside, nowhere). Exactly: on Windows the walk follows no link, and the
  location is the root joined with the request's segments. (The attribute is set on all six links the probe made and on
  no plain file or directory: PROBE B1 `attrs=0x2420`/`0x2410 reparse=true` against `0x2020`/`0x2010 reparse=false`.)
- For `read`, `write_atomic`, `is_file` (false), `list_dir` (a reparse point as the listed directory or on the way to
  it) and `check_location`, hence `doc.open` (also of a path already open), `doc.save`, `project.tree`,
  `nav.workspace-symbols`. Precedence and messages as now (`invalid_path`, `outside_root`, `not_found`, `not_a_file`,
  `io_error`); a missing component ends the walk as now (`not_found`).
- Not on the walk, unchanged: `--root` and the directories above it (a junction given as `--root` leads to the root,
  PROBE E2-E3).
### (2) Kinds in a listing
A directory entry whose own attributes carry the reparse-point attribute is listed with kind `symlink`, never `file` or
`dir`, so `project.tree` never descends into it (D-254 ruling 4). For links and junctions that is base behaviour (PROBE
C7 `j:Symlink`, `jo:Symlink`, `sd:Symlink`, `sf:Symlink`, `so:Symlink`, `sr:Symlink`); for other reparse tags it is
**unchecked** (no test on the runner can make one).
### (3) A reparse point at a temporary name (decided from PROBE H: fixed, not stated)
When creating `.kiln-tmp-N` fails and an entry exists at that name (`symlink_metadata` succeeds, nothing followed or
opened), the save takes the next name, as it does for `AlreadyExists`; the entry and whatever it leads to are not
changed. A failure with nothing at the name is returned as now. Reason for a fix rather than a stated refusal: on Unix
"a leftover … only makes the next save use the next name" (`disk.rs:11-13`), and a project holding a junction at
`.kiln-tmp-0` (a hostile or merely odd repository) would otherwise make every save in that directory fail on Windows
(PROBE H2). Which of the two links in H refused the create is not determinable from the record (the loop stops at the
first error); the tests cover the pair exactly as H planted it and each kind alone (D-256 Q-2). Only the Windows
`create_temp` (`disk.rs:431-449`) changes. Exactly (D-256 Q-1, Q-3):
- The create opens with `std::os::windows::fs::OpenOptionsExt::custom_flags(0x0020_0000)` (`FILE_FLAG_OPEN_REPARSE_POINT`)
  in addition to `write(true)` and `create_new(true)`; no other flag.
- For each name `.kiln-tmp-0` … `.kiln-tmp-99` in order: the create succeeds → that name is used; it fails with
  `AlreadyExists` → next name; it fails otherwise and `fs::symlink_metadata(<name>)` is `Ok` → next name; it fails
  otherwise and `symlink_metadata` fails (whatever its kind, `NotFound` included) → the create's own error is returned
  (`VfsError::Io` with the create's kind and message, as now).
- When no name was free (every one of the 100 skipped by the two rules above), the save fails as on Unix:
  `VfsError::Io` with kind `AlreadyExists` and message "all 100 temporary file names are taken" (`disk.rs:445-448`),
  whatever the individual creates answered. No test is required for the mixed case (D-256 Q-3); the existing no-cfg
  `all_temp_names_taken_fails_without_changes_then_recovers` (`disk.rs:774-803`) keeps covering plain files on Windows.
### (4) The root cases on Windows (measured at base; unchanged by this task; stated in paths.md)
- `--root` a junction: its target is the root (PROBE E2, E3); the junction retargeted while a session runs → every call
  `outside_root`, nothing read in the other directory (E5); pointed back → the session works again (E7).
- The root renamed away and a junction to another directory put at its path → `outside_root` for read and save, the
  other directory unchanged (PROBE G1-G3); a junction at the path to the held root → works (G5).
- The held root removed while a session runs, empty (`remove_dir`) or with a file in it (`remove_dir_all`): Windows
  allows it (PROBE F1, F7), every call then answers `outside_root` (F3, F8), also after a new directory takes the name
  (F4, F5).
- `..` in the `--root` text is resolved by its text before any link is followed (PROBE K2-K4): a difference from Unix.
- Unprobed, expected the same and **unchecked until this task's first CI round** (spec check F-6, F-7): a directory
  symbolic link as `--root` (as a junction, first bullet); the held root removed and a junction to a new directory put at
  its name (every call `outside_root`). The first CI round must show both in `guard_t074` (and #11, #12 of (5)). If it
  shows otherwise: a call that is not `outside_root` after the root was replaced is a confinement defect — the builder
  logs `QUESTION:` and stops, the director rules; a symbolic link as `--root` that does not behave as a junction makes
  the reviewer narrow the `paths.md` root sentence of Pinned (i) to junctions and state the symbolic-link case as what was
  measured. No document sentence rests on the removed-then-junction case.
### (5) The nine pending registry entries (`disk.rs::tests::…`, `platforms = "unix"`; the Unix tests stay as they are)
Each becomes `counterpart = "<file>::<item>"` (no `reason`, `task`, `expires`). The new tests are the implementer's
(D-254 ruling 3), in `disk.rs`'s `#[cfg(test)]` module, **with no platform cfg on the test fn**: each asserts the Unix
behaviour on Unix and this Interface on Windows, chosen with `cfg!(windows)`, both branches asserting (§8.1). A link
helper may be defined per platform (Unix `symlink`; Windows `std::os::windows::fs::symlink_file`/`symlink_dir`, which
work on the runner, PROBE A1, A2, A5, A6, and a junction by `cmd /C mklink /J` under a timeout, PROBE A3), each form doing
its work (`no-skip`'s `inert-helper` rule). They run on Windows in T-073's reported step (`-p kiln-workspace --lib`).
The new tests' names are fixed here (the registry cites them and AC-3 checks them; D-256): the implementer uses exactly
the names in the table, all in `crates/kiln-workspace/src/disk.rs`'s module `tests`.

| # | Entry (`tests::`) | Counterpart (exact item) | What it asserts on Windows |
|---|---|---|---|
| 2 | `symlink_at_temp_name_is_not_followed` | `tests::link_at_temp_name_is_not_followed_on_every_platform` | Interface (3), four fixtures each in a fresh directory: (a) H's pair (junction at `.kiln-tmp-0` to an outside directory, file symlink at `.kiln-tmp-1` to a missing outside file); (b) a junction alone at `.kiln-tmp-0`; (c) a file symlink alone at `.kiln-tmp-0` to an existing outside file; (d) a dangling file symlink alone at `.kiln-tmp-0` to a missing outside path. Each: the save `Ok`, the target holds the new bytes, the links still links with their targets, the outside unchanged and the missing path still missing, no `.kiln-tmp-*` left besides the planted ones |
| 8 | `replace_swaps_inode_keeps_permissions_and_follows_symlinks` | `tests::save_through_a_link_on_every_platform` | a save through a file symlink (relative and absolute), a directory symlink and a junction on the way is `OutsideRoot`; link, target and directory unchanged |
| 9 | `root_replaced_by_a_symlink_between_requests_is_outside` | `tests::root_replaced_by_a_link_on_every_platform` | Interface (4), second bullet |
| 10 | `parent_replaced_by_a_symlink_between_requests_is_outside` | `crates/kiln-workspace/tests/guard_t073.rs::acw2_root_parent_replaced_is_outside_root` (exists, no platform cfg; the parent's rename is refused on Windows, run 44268710587) | — (exists) |
| 11 | `root_removed_and_replaced_by_a_link_to_a_new_directory_is_outside` | `tests::root_removed_on_every_platform` | Interface (4), third bullet (a new directory at the name: PROBE F4, F5), and a junction to a new directory at the name (unprobed: Interface (4), last paragraph) |
| 12 | `root_given_through_a_link_that_is_retargeted` | `tests::root_given_through_a_link_on_every_platform` | Interface (4), first bullet, once with a junction and once with a directory symbolic link as `--root` (D-256 Q-4) |
| 13 | `link_target_with_trailing_slash_requires_a_directory` | `tests::save_through_a_link_on_every_platform` (the test of #8; on Windows no link is followed, so a target's trailing `/` has no Windows form) | — |
| 14 | `check_location_is_ok_for_every_inside_location` | `tests::check_location_inside_on_every_platform` | `check_location` `Ok` for a file, a directory, nothing and a missing parent; `OutsideRoot` for a link or junction at a middle and at the last component |
| 15 | `check_location_refuses_outside_loops_and_a_replaced_root` | `tests::check_location_outside_on_every_platform` | a junction to outside, a junction to inside, a junction loop and a replaced root each `OutsideRoot` |

Registry text (the implementer's; exact): in each of the nine entries, keep the lines `file`, `item`, `platforms`, `doc`,
`quote` byte for byte and replace the four lines `counterpart = "pending"`, `reason = …`, `task = "T-074"`,
`expires = "2026-12-31"` by the single line `counterpart = "<file>::<item>"` with `<file>` =
`crates/kiln-workspace/src/disk.rs` and `<item>` from the table (#10: the `guard_t073.rs` path in full). Example (#2):
`counterpart = "crates/kiln-workspace/src/disk.rs::tests::link_at_temp_name_is_not_followed_on_every_platform"`. No
other line of the registry changes and no entry is added.

### (6) Tests (test author)
- `crates/kiln-workspace/tests/guard_t074.rs`: Rust API (`DiskFs`, `Vfs`, `Document`); no platform cfg in the file (T-073
  ruling Q-2, D-245); every fixture below `CARGO_TARGET_TMPDIR`. For each kind the runner can make (file symlink relative
  and absolute, directory symlink, junction; leading inside, outside, nowhere; at a middle and at the last component):
  `read`, `write_atomic`, `is_file`, `list_dir`, `check_location` and a `Document` save answer Interface (1)-(2) on Windows
  and today's behaviour on Unix; Interface (3) with the four fixtures (a)-(d) of (5) #2 (H's pair, the junction alone,
  the file link alone, the dangling file link to outside alone); the root cases of (4) — a junction as `--root` and
  retargeted, a **directory symbolic link as `--root`** and retargeted (D-256 Q-4), replaced, removed (also with a junction
  to a new directory at the name), `..` in the root text — each printing one `MEASURED:` line as in `guard_t073.rs`
  (`MEASURED: <case>: <what was observed>`), and after a removal or replacement every one of `read`, `write_atomic`,
  `is_file` (false), `list_dir`, `check_location` (spec check F-5). A case the runner cannot set up fails naming why, never passes (§8.1). It must not need a module file
  in a subdirectory of `tests/` (T-072's step runs each `tests/*.rs` as one target).
- `xtask/tests/acceptance_t074.rs`: AC-3, reading `xtask/platform-tests.toml` as text as `acceptance_t079.rs` does.
### (7) `acceptance_t002::save_through_symlink_updates_target` (test author; D-256 Q-5)
This existing test has no platform cfg, makes a relative file link inside the root on every platform
(`acceptance_t002.rs:184-191`, Windows: `symlink_file`) and asserts the save goes through it; it passes on Windows at base
(`tasks/ci-runs/T-073-round-2.md:52`: of `io.save-atomic`'s three tests only `disk_save_replaces_via_rename_without_leftovers`
fails there) and would fail after Interface (1). Form, chosen by a run of main's `no-skip` (spec check, D-256 Q-5): the
name and the fixture lines stay; the body after `let path = rp("d/link.txt");` becomes `if cfg!(windows) { … } else {
<today's body, unchanged> }`, the Windows branch asserting: `Document::open(&disk, &path)` is
`Err(OpenError::Vfs(VfsError::OutsideRoot { .. }))`; `disk.write_atomic(&path, …)` is
`Err(VfsError::OutsideRoot { path })`; `d/target.txt` still holds `target\n`; `d/link.txt` is still a symbolic link; the
listing of `d` equals the one before. No early `return` (main's `no-skip` calls that a skip: run, FAIL `early-return`), no
platform cfg added, no registry entry (main's `no-skip` on that form: PASS, `35 … (19 pending)` on base + this change).
No other line of `acceptance_t002.rs` changes: **T-092 later rewrites the rename test in this file**
(`disk_save_replaces_via_rename_without_leftovers`, its `#[cfg(windows)] panic!` at `:939-942`) and re-checks this test
when it is specified.

## Pinned
- (a) Errors: no new kind; a reparse point is `VfsError::OutsideRoot` (`rejected` / `outside_root`, messages as now).
- (b) Files: `.kiln-tmp-N` as now (Interface (3) changes only which names count as taken). Registry layout as
  `xtask/platform-tests.toml:9-18`; the nine entries' exact text in Interface (5).
- (b2) The Windows temporary create (D-256 Q-1): `custom_flags(0x0020_0000)` (`FILE_FLAG_OPEN_REPARSE_POINT`) with
  `create_new(true)` makes the create act on the name itself: an existing link or junction at the name is never followed,
  so its target is neither created nor opened, and the create fails (the name exists) — then Interface (3)'s rules apply.
  This rests on the Win32 documentation of `CreateFileW` (read, not run here) and is **unchecked until this task's first CI
  round**, where (5) #2 fixtures (c) and (d) and `guard_t074`'s Interface (3) cases show it (the outside file unchanged, the
  missing outside path still missing). If the round shows a target created or changed, it is a confinement defect: the
  builder logs `QUESTION:` and stops; no document sentence about temporary names stands until it is shown.
- (c) Order: lexical check, root check, walk (the reparse test at each component, before following or descending), type,
  open, as now.
- (d) Platform: Windows x86_64 MSVC (CI `windows-2025`; the probe's volume was `D:`, PROBE B4, I9; its file system was not
  read: I9 `Error 3`). Unix: every product change is under `#[cfg(windows)]` or adds `#[cfg(unix)]` to an unchanged line
  (comments, doc comments and the `#[cfg(test)]` module excepted: Pinned (i) rewrites `:94-96`, Interface (5) adds tests
  with no platform cfg; spec check C-5).
- (e) Threat model: a hostile project directory (cloned repository, archive) may hold links, junctions or other reparse
  points, also at `.kiln-tmp-N`; the attacker does not change the tree during a request (residual limit unchanged).
  Refusing every reparse point is fail-safe. Not defended (unchanged): hard links; changes during a request; reparse
  points at or above the root (resolved at start); the ReFS identity limit (T-094). Accepted cost (D-254 ruling 6): files
  that are reparse points without being links are refused too.
- (f) Tests pinning what this task changes: none on Linux. `xtask/tests/guard_t079.rs` pins all 1,121 Linux test names of
  its base (`:515-516`), among them the 22 of `disk.rs` (`:1241-1262`): none renamed, removed or re-cfg'd, and
  `failed_rename_removes_own_temp_and_keeps_target` keeps no platform cfg (`:506-511`). The 7 entries quoting `paths.md`
  (`platform-tests.toml:310-376`; `grep -n 'doc = "docs/protocol/paths.md"' xtask/platform-tests.toml` → 7 entries and the
  template comment `:13`; spec check C-1) keep their quoted sentences word for word. On Windows (spec check F-11):
  `crates/kiln-workspace/tests/acceptance_t002.rs::save_through_symlink_updates_target` (no platform cfg, a relative file
  link made by `symlink_file`, `:188-191`, `:945-973`) passes at base and would fail after Interface (1): it gets the
  Windows branch of Interface (7) (D-256 Q-5). Main's tests on that path (D-249; `grep -rn "acceptance_t002" --include=*.rs
  crates apps xtask`): `xtask/tests/guard_t079.rs:1291-1305` lists its test names, among them this one (`:1301`), and asks
  only that they still exist — the name stays: `cargo test -p xtask --test guard_t079` on the changed file in the spec
  check's clone → 4 passed, 0 failed; `xtask/tests/acceptance_t005_r2.rs:164` names `xtask/tests/acceptance_t002.rs`, another
  file; `acceptance_t002_r2.rs` is another file; `scorecard/items/io.save-atomic.toml:5` lists the name (unchanged; the
  scorecard is computed on Linux, where the test's body is today's).
- (g) Worked examples, executed on the runner at base (PROBE lines) against the model "Windows follows no link; a taken
  name is skipped": `read sr` (relative file link to `f.txt`): base `Ok([102])` (C1), after `OutsideRoot`; `read j/x`
  (junction to `d` inside, `d/x` exists): base `OutsideRoot` (C3), after `OutsideRoot`; `write_atomic f.txt` with links at
  `.kiln-tmp-0`/`-1`: base `Err(Io … PermissionDenied … os error 5)` (H2), after `Ok`, written through `.kiln-tmp-2`, links
  and `out` unchanged.
- (h) Evaluator: none (no item claimed).
- (i) Document sentences, worded to be copied. "Measured" ones cite run 44290286460 (base) and are shown again, on the
  task's head, by `guard_t074.rs` in the CI record (the reviewer checks that before the sentence stands); "unchecked" ones
  are the reviewer's first check and stay marked so in the document.
  - `paths.md`, replacing `:50-51`: "On Windows (T-074) the rules below hold with this difference: no link is followed.
    Every entry on the way that is a reparse point — a symbolic link to a file or a directory, a junction, or any other
    reparse point — makes the path `outside_root`, wherever it leads, and nothing beyond it is read or written
    (`crates/kiln-workspace/tests/guard_t074.rs` on the `windows-2025` runner; reparse points other than links and
    junctions: unchecked, the runner cannot make one). Names that Windows gives a meaning, and case, are not yet stated
    for Windows (T-095)." Evidence: measured for links and junctions (PROBE B1 attributes; C1/C3 base behaviour), the
    rest unchecked.
  - `paths.md` "The root", replacing T-073's "Not run on Windows: removing a held root, and …" with: "Measured on the
    runner (CI run 44290286460, `tasks/ci-runs/T-074-probe-1.md`): the held root can be removed while a session runs, and
    every call then answers `outside_root`, also after a new directory takes its name, as on Linux and macOS. Not run on
    Windows: a directory above the root replaced (Windows refused the rename)." Evidence: PROBE F1-F8 for removal and
    for `check_location` (F3, F5), `list_dir` (F5) and `read` (F8) after it; "every call" (a save, `is_file`) only by
    `guard_t074.rs` on the head (spec check C-3).
  - `paths.md` "The root", a Windows bullet: "**A difference from Linux and macOS:** a `..` in `--root` is resolved by its
    text before any link is followed: with `link` a junction, `--root link\..\b` is the `b` beside `link`, not the one
    beside the directory `link` leads to." Evidence: PROBE K2, K4. Unix side (spec check, Linux, scratch): with `k/link ->
    a/real` a symbolic link, `readlink -f k/link/../b` → `…/k/a/b` and its `f.txt` reads `physical` (the `b` beside the
    target), `readlink -f` standing in for `fs::canonicalize` (both `realpath` semantics; INFERRED equivalence, not run
    through `DiskFs`); macOS: unchecked.
  - `paths.md` "The root", a Windows bullet: "A junction or symbolic link given as `--root` leads to the root; if it is
    pointed elsewhere while a session runs, every call answers `outside_root`, and pointed back the session works there
    again, as on Linux and macOS." Evidence: for a junction PROBE E2-E7 (after a retarget only `read` was run, E5),
    G1-G5; **a symbolic link as `--root`: unchecked at base** (the probe made junctions only, E1, E4, E6, G2, G4), shown on
    the head by `guard_t074.rs` (D-256 Q-4); the reviewer checks that `MEASURED:` line before the sentence stands, else
    narrows it (Interface (4), last paragraph; spec check C-4).
  - `paths.md` "Residual limits", a new bullet: "On Windows every
    reparse point on the way is refused, so files that are reparse points without being links — for example cloud
    placeholder files of a synced folder or deduplicated files — cannot be opened or saved; they answer `outside_root`
    (unchecked: the runner cannot make them)." `- Unix only.` stays (T-093 replaces it).
  - `paths.md`, a last section **"For the platform report"**, one line for the owner: "On Windows Kiln follows no link or
    junction inside a project and refuses every reparse point as outside the root, including cloud placeholder and
    deduplicated files (those two unchecked on the hosted runner)."
  - `project.md`, after `:79-80`: "On Windows every entry that is a reparse point is listed with kind `symlink` and never
    descended (T-074; for links and junctions shown by `guard_t074.rs`, for other reparse points unchecked)." Evidence:
    PROBE C7 (links, junctions).
  - `control-protocol.md:141`: "How symlinks are followed there is not yet stated (T-074)." → "On Windows no link is
    followed: a save through a symbolic link or a junction is refused (`outside_root`), and a link at a temporary name
    makes the save use the next name; see `docs/protocol/paths.md`." Evidence: PROBE C4, H2 (base) and `guard_t074.rs`
    (head). The bullet's first sentence ("… (on Unix); through a symlink, the link's target is replaced and the link is
    kept.") stays verbatim (`apps/kiln-cli/tests/acceptance_t008_cli.rs:563-574`, the bullet at `:566-568`).
  - `disk.rs` module docs (`:94-96`): "The walk below the root … is not decided here (T-074)" → the Windows rule of (1)
    and (3), naming T-095 for names and case.

## Acceptance criteria
Local (Linux; implementer, re-run by the reviewer):
- AC-1: every existing `kiln-workspace` test passes on Linux and macOS (the 28 of `ws.path-confinement` among them); every
  changed product line is under `#[cfg(windows)]` or adds `#[cfg(unix)]` to an unchanged line, comments, doc comments and
  the `#[cfg(test)]` module excepted (reviewer's hunk table, T-073 Q-3 A; spec check C-5); the Linux and macOS lib IR
  equals base's after normalising (reviewer, as `reviews/T-073-r1.md`).
- AC-2: `cargo clippy -p kiln-workspace --lib --profile test --locked --target x86_64-pc-windows-msvc -- -D warnings`,
  `cargo clippy -p kiln-workspace --test guard_t074 --locked --target x86_64-pc-windows-msvc -- -D warnings` and `cargo
  clippy --workspace --lib --bins --locked --target x86_64-pc-windows-msvc -- -D warnings` clean (`kiln-workspace` needs no
  C compiler stub: a test target of it was clippy-checked this way in this run; the workspace form needs the stub of
  `reviews/T-073-r1.md`). The `--test guard_t074` form is run by the test author before handover and by the reviewer
  after the join, not by the implementer, who does not see that file before the join (§6; spec check C-6).
- AC-3 (`xtask/tests/acceptance_t074.rs`; fails on base on every platform by assertion): for each of the nine (file, item)
  pairs of Interface (5) the registry holds exactly one entry, with `platforms = "unix"`, `counterpart` equal to exactly
  the string of Interface (5)'s "Registry text" for that row (#10:
  `crates/kiln-workspace/tests/guard_t073.rs::acw2_root_parent_replaced_is_outside_root`), and no `reason`, `task` or
  `expires`; no entry has `task = "T-074"`. Variants it must reject: one of the nine left `pending` with `task = "T-095"`;
  #14 naming `crates/kiln-workspace/src/disk.rs::tests::check_location_outside_on_every_platform` (another row's test).
- AC-4: `cargo xtask no-skip` on the candidate prints `35 platform test(s) registered (10 pending)` (19 − 9; the new tests
  carry no platform cfg, so no entry is added). Variant to reject: a counterpart naming a `#[cfg(unix)]` test.
Through CI (pending criterion, §8.1; fail-on-base evidence on Windows is the probe record, D-254 ruling 8):
- AC-W1 (Interface (1)): in `guard_t074`, each case "a link or junction on the way is `outside_root`, nothing read or
  written" `ok` on Windows. Fails on base for a relative file link inside the root: PROBE C1 `read sr: Ok([102])`, C2
  `check_location sr: Ok(())` (the junction and absolute-link cases already pass at base: C1, C3, C4, C5, C6).
- AC-W2 (Interface (2)): `list_dir` kinds of links and junctions `symlink` on Windows. Holds at base (PROBE C7): a guard.
- AC-W3 (Interface (3), Pinned (b2)): in `guard_t074` and in (5) #2, the four fixtures (a) H's pair, (b) a junction alone at
  `.kiln-tmp-0`, (c) a file symlink alone there to an existing outside file, (d) a dangling file symlink alone there to a
  missing outside path: each save `Ok`, the target holds the new bytes, each link still a link with its target, the
  outside file unchanged, the missing path still missing, no temporary file left. (a) is the fail-on-base evidence; (b)-(d)
  are unmeasured at base. Fails on base: PROBE H2 (`PermissionDenied`, os error 5), H3 — **for the
  pair only** (junction at `.kiln-tmp-0` and file link at `.kiln-tmp-1` together, as H1 planted them); for each entry
  alone base behaviour is not measured (which one refused is not determinable, Interface (3)) (spec check C-8, Q-2).
- AC-W4 (Interface (4)): the root cases of Interface (6) `ok` with their `MEASURED:` lines, every call answering as stated.
  Hold at base for a junction (PROBE E2-E7, F1-F8, G1-G5, K2-K4): guards for the sentences of Pinned (i); the directory
  symbolic link as `--root` and the removed-then-junction case are unmeasured at base (Interface (4), last paragraph).
- AC-W5 (Interface (5)): each of the seven counterpart tests of `disk.rs`, by the names of Interface (5), `ok` on Windows
  in T-073's step.
- AC-W6 (Interface (7)): `acceptance_t002::save_through_symlink_updates_target` `ok` on Windows in T-072's step (and on
  Linux and macOS as today). Fails on base on Windows: at base the open through the link succeeds (the test passed there,
  `T-073-round-2.md:52`), so the Windows branch's first assertion fails.

### The pending CI criterion (v1.5 §8.1), for the approval row
The record `tasks/ci-runs/T-074-round-N.md`, from the artefacts of the task's head **rebased on a main that holds T-072**
and pushed to `ci/candidate`, shows: `gate-linux`, `gate-macos`, `window-linux`, `window-probe (macos-15)` green; no-skip
`35 platform test(s) registered (10 pending)` on all three; in `gate-windows`: fmt, layers, ui-commands, hygiene,
schema, no-skip PASS; clippy, scorecard, test FAIL only at test targets T-092 owns, named in the record, none of
`kiln-workspace`'s lib, `guard_t073`, `guard_t074`; **T-073's step** ("Workspace lib and T-073 tests", `workspace-tests.txt`,
`-p kiln-workspace --lib --test acceptance_t073 --test guard_t073`) names every lib test `ok`, the seven new counterparts
among them (AC-W5); **T-072's general step** (its Interface (6), file `windows-tests.txt`) shows `==== guard_t074: exit status
0` and every `guard_t074` test `ok` with its `MEASURED:` lines (AC-W1-W4); the core-crates step's `xtask` block exit status
0 with `acceptance_t074`'s tests `ok` (AC-3 on Windows); `window-probe (windows-2025)` as before (exit 2, no OpenGL).
Counts to expect (spec check C-7, from `tasks/ci-runs/T-073-round-2.md` plus this task): T-073's step `running 43 tests`
/ `43 passed; 0 failed` for the lib (36 at base + the 7 counterparts, named in the implementer's log),
`acceptance_t073` 1 and `guard_t073` 10 passed; core-crates `==== xtask: exit status 0` with 279 + n passed, 0 failed
(n = `acceptance_t074`'s tests, from the test author's log); `guard_t074` every test `ok`, 0 failed, the count from the
test author's log. T-072's step reports every `crates/kiln-workspace/tests/*.rs` target: the record lists each one's
exit status; a non-zero status is allowed only where it was non-zero at base for the same tests
(`acceptance_t002`: its `#[cfg(windows)]` precondition panic in `disk_save_replaces_via_rename_without_leftovers`,
`acceptance_t002.rs:939-942`, T-092's), and no test that passed on Windows at base may fail there;
`acceptance_t002::save_through_symlink_updates_target` `ok` (AC-W6). The seven counterparts `ok` in T-073's step by
the names of Interface (5) (AC-W5; spec check F-12: their fixtures are under the system temp directory on `C:`, where no
link was made by the probe — unchecked until this round; if a link or junction cannot be made there, the test fails
naming why, never passes, and the implementer moves that test's fixtures to a directory under the workspace's
`target/` on the work volume; no document sentence rests on the fixture's volume).
**Two CI rounds expected.** If T-072 has not merged when this task's head is ready, `guard_t074` is not run on Windows and
the criterion cannot be met: the director waits for T-072 (it does not edit the workflow here).

## Out of scope and implementer notes
- Names Windows gives a meaning, streams, case, short names (T-095); the 128-bit identity (T-094); the saved file's access
  list (D-254 ruling 7); product test files outside `disk.rs`'s module (T-092), except the one test of Interface (7)
  (D-256 Q-5); a CLI test file (D-254 ruling 5);
  following links on Windows (ADR-0009 Q-6 A, a later decision); an identity-read failure on Windows (not measurable on
  the runner, PROBE J).
- The test author's file has no platform cfg; a platform-only test it cannot avoid is named in its report and the
  implementer adds the entry before the join (D-245).
- `std::os::windows::fs::MetadataExt::file_attributes` is std; no dependency changes (`Cargo.toml`, `Cargo.lock` not in
  `touches`).

## Collisions and order
- T-072 (in flight, D-254): no shared path (it touches `platforms.yml`, `xtask/ledger.toml`, `docs/development.md`,
  `xtask/tests/acceptance_t072.rs`, `guard_t072.rs`); this task's CI round needs it merged (above).
- `docs/protocol/paths.md`, `project.md`: T-093 (specified, low; dispatched after this task), T-094, T-095 (drafts) →
  after; each re-checks the sentences of (i). `disk.rs`: T-092, T-094, T-075 (drafts) → after. `xtask/platform-tests.toml`:
  T-092, T-095 → after. `crates/kiln-workspace/tests/acceptance_t002.rs`: T-092 (draft; rewrites the rename test there)
  → after, and re-checks `save_through_symlink_updates_target` (Interface (7)).

## Estimate
T-073 (same file and platform, about 60 product lines, a test module, an independent Windows test file, 2 CI rounds)
booked 860,977 (D-252). This task: about 30 product lines (the reparse test, the list kind, the temporary-name rule), seven
portable unit tests that make links and junctions, nine registry entries, six document places, a guard file and an xtask
test, one test of `acceptance_t002.rs`, no workflow edit, 2 CI rounds: **about 880,000** (D-256; ceiling 1,100,000;
spec check §6). Arm: high → independent test author.

## Questions and rulings
### Rulings on the planner's questions (director, D-254)
1 cut accepted (T-095 after); 2 T-072 adds the general Windows step, this task gives up `platforms.yml` and
`acceptance_t070.rs` and runs beside T-072, CI read on a head rebased on a main holding T-072; 3 counterparts by the
implementer in `disk.rs`'s module; 4 other reparse points listed `symlink`; 5 no CLI test file; 6 every reparse point
refused, a stated limit, "unchecked" where the runner cannot show it, a line under "For the platform report"; 7 access
list out of scope; 8 the probe record is the Windows fail-on-base evidence.

### Rulings on the fresh spec check (director, D-256), written in
Q-1 yes: Pinned (b2), Interface (3) "Exactly". Q-2 yes: Interface (5) #2 fixtures (a)-(d), (6), AC-W3. Q-3 yes:
Interface (3) "Exactly" (no test). Q-4 yes: (5) #12, (6), AC-W4, Pinned (i) root sentence. Q-5 yes: `touches`,
Interface (7), AC-W6, Pinned (f), Collisions; form chosen by run (Interface (7)). Estimate 880,000.
Each finding closed (spec check below): F-1 → C-8 + Q-2 (AC-W3 (a)); F-2 → Q-1 (Pinned (b2), unchecked until CI round 1,
what happens otherwise stated); F-3 → Q-3; F-4 → no change (Unix is path-based too; the window is the stated residual
limit, Pinned (e)); F-5 → C-3 + Interface (6) (every call after removal/replacement), the reviewer checks before the
sentence stands; F-6 → C-4 + Q-4; F-7 → (5) #11 + Interface (4) last paragraph (unchecked until CI round 1; a
non-`outside_root` answer is a defect, no sentence rests on it); F-8 → C-5; F-9 → C-6; F-10 → C-7; F-11 → Q-5;
F-12 → pending criterion (unchecked until CI round 1; fixture move if a link cannot be made; no sentence rests on it).
Agreed between the builders by this text (re-read from each side): the seven counterpart names and the registry's exact
text (Interface (5); AC-3 checks the strings); error variants `VfsError::OutsideRoot`, `VfsError::Io` with
`AlreadyExists` and its message, `OpenError::Vfs(VfsError::OutsideRoot { .. })`; the `MEASURED:` line form; the
`acceptance_t002` form. Left free on purpose: `guard_t074`'s and `acceptance_t074`'s test names (cited by no other
file; the CI record takes them from the test author's log).

### Spec check (planner, after the probe and D-254)
- **S-1 T-072's general step and this task's files** (`tasks/T-072-gate-green-on-macos.md` Interface (6), AC-9): it runs,
  for each `crates/kiln-workspace/tests/*.rs` in name order, `cargo test --locked --no-fail-fast -p kiln-workspace --test
  <t> -- --show-output`, so `guard_t074.rs` is run, one target per invocation (another file's compile errors do not stop
  it), with its `MEASURED:` lines. The `disk.rs` counterparts run in T-073's `--lib` step; `xtask/tests/acceptance_t074.rs`
  runs in the core-crates step (`cargo test -p xtask`). Nothing is missing for this task. Two notes for T-072's builders,
  not blockers: AC-9 as written does not check `-- --show-output` (only Interface (6) says it; without it the `MEASURED:`
  lines of a passing test are not in the artefact); and a `tests/<dir>/mod.rs` file would not be a target (this task's
  file must not need one, Interface (6)).
- **S-2 Shared with tasks in flight:** T-072 — no path; the workflow it changes is read by this task's criterion only. No
  trait, type or public signature changes (`Vfs`, `VfsError`, `EntryKind` unchanged); no criterion is tested through a new
  Rust signature.
- **S-3 Main's tool on the candidate (D-235).** No file under `xtask/src/`, no fixture: main's tool (also after T-072,
  which changes `xtask/ledger.toml` and no rule) is the candidate's. `no-skip`: PASS if each new counterpart has no
  platform cfg on its fn, each per-platform helper and `#[cfg]` statement has its twin doing work, and the 7 quoted
  `paths.md` sentences stay (`35 … (10 pending)`); a counterpart naming a missing or Unix-only test fails it. `hygiene`:
  PASS (no session path in documents or code; review files as usual). `scorecard --check`: PASS (no item, no listed test
  changes; the page is computed on Linux). `layers`: PASS (no manifest or lock change). `ui-commands`: PASS (no `kiln-ui`
  file). `schema`: PASS (no command, `docs/commands.json` unchanged). No gate rule changes, so the tree uses none.
  The implementer runs main's six steps (built from `git archive main` in scratch) on the candidate before the join and
  logs their lines; the reviewer repeats it.
- **S-4 Main's tests that name a path in `touches` (D-249; `grep -rlE '<path>' --include=*.rs crates apps xtask` on
  `72522f2`):** `disk.rs` → `xtask/tests/guard_t079.rs` (base names: passes under Pinned (f)). `paths.md` →
  `apps/kiln-cli/tests/acceptance_t008_cli.rs:576-578` (contains `outside_root`, `not_a_file`: passes); `disk.rs`, `vfs.rs`
  name it in comments. `project.md` → `crates/kiln-ui/src/panels/project.rs`, `crates/kiln-commands/src/project.rs`,
  `crates/kiln-workspace/src/project.rs`, `apps/kiln/tests/acceptance_t057.rs`, `apps/kiln/tests/project_t057.rs`: doc
  references, no sentence read. `control-protocol.md` → `acceptance_t008_cli.rs:549-574` pins the lexical bullet and the save
  bullet's first sentence verbatim (kept: passes) and that the doc does not claim symlinks are followed out of the root
  (the new sentence does not say so: passes); the other 16 files read other sections (kinds, synopsis); no test contains
  "How symlinks are followed" (grep: none). `platform-tests.toml` → `xtask/tests/acceptance_t079.rs` (T-079's 20 only, none
  of the nine: passes); `acceptance_t071.rs`, `guard_t071.rs` write toy registries in scratch trees; other hits are
  comments. New files (`guard_t074.rs`, `acceptance_t074.rs`): no main test names them; `guard_t079` only requires base
  names to stay. After T-072: its `acceptance_t072.rs` (AC-9) runs the step's script in a scratch tree with fake test
  files, not the real `tests/` list (its spec), so a new `guard_t074.rs` does not change it; to be confirmed against
  T-072's merged tests by the fresh spec check.
- **S-5 Criteria of absence or limit:** Interface (1) "the walk follows no link" — variant to reject: a relative file link
  inside the root followed (base, PROBE C1). AC-3 — variant: an entry moved to `T-095`. Interface (2) — variant: a
  junction listed `dir`. Interface (3) — variant: the save refused with `PermissionDenied` (base, H2).
- **S-6 Nothing left unmeasured that a choice hangs on:** the probe answered A-K; no second probe. Unchecked and marked:
  reparse points other than links and junctions (cannot be made on the runner); an identity-read failure (PROBE J shows
  no effect on the runner, so no sentence); the volume's file system (PROBE I9 not read).
- **S-7 Not checked:** T-072's branch (dispatched minutes ago; not read); anything at run time with a prototype of
  Interface (1)-(3) (none built: the choices rest on base measurements and reading).

### Spec check (fresh planner)
Read: `methodology/roles/planner.md`; METHODOLOGY §3-§8.1; this spec; `tasks/ci-runs/T-074-probe-1.md` in full;
`disk.rs` 1-660, 740-803; `docs/protocol/paths.md`, `project.md`, `docs/control-protocol.md` "Paths"; the registry;
`xtask/src/platforms.rs:883-1030` (the registry rules); D-235, D-249, D-250, D-252 to D-254; T-095; T-072 Interface (6),
AC-9, the pending criterion and its rulings; `tasks/ci-runs/T-073-round-2.md`; `reviews/T-073-r1.md` (stand-ins). Main at
`9603d97`; the only drift from the spec's `72522f2` is `docs/decisions.md` (command in the log). Scratch clone
`/home/user/kiln-check-T074` (its `target/` deleted at the end); helper scripts in `/home/user/kiln-check-T074-scripts/`.

#### 1. Sentences resting on the probe
Checked every "PROBE" citation against the record line by line. Agree: Goal (C1 `read sr: Ok([102])`, C2, C3, H2, H3
`out listing:` empty), Notes (K2, K4, J2, J3), Interface (1) (B1: six links `0x2420`/`0x2410 reparse=true`, `f.txt`
`0x2020`, `d` `0x2010`), (2) (C7), (4) G1-G5, K2-K4, F1/F7 removal `Ok`, (5) A1, A2, A3, A5, A6, (g) C1, C3, H2. Mismatches:
- **F-1 (Interface (3), AC-W3: fail-on-base evidence covers only the pair).** H planted both entries at once
  (`.kiln-tmp-0` junction, `.kiln-tmp-1` file link, H1) and got one error (H2). The spec is right that which entry refused
  is not determinable (`create_temp` returns on the first error that is not `AlreadyExists`, `disk.rs:440-442`: either
  `-0` gave os error 5, or `-0` gave `AlreadyExists` and `-1` gave os error 5). Consequence the spec does not draw: for
  "a junction at `.kiln-tmp-0`" alone and "a file symlink there" alone, base behaviour is **unmeasured**; one of the two
  (at least) fails on base, the other may already pass. AC-W3's "Fails on base: PROBE H2" is true only of the pair.
  Corrected in AC-W3 (C-8); Q-2 asks for the pair as a case.
- **F-2 (Interface (3): can the entry be followed by the create itself? Not measured, not pinned).** The only call on the
  way that could follow the entry is the `create_new` open (`disk.rs:435-438`). The Unix test it replaces checks a
  *dangling* link to the outside and that its target is not created (`disk.rs:752-758`: `!outside.join("nonexistent")
  .exists()`); H's file link `-1 -> out/t` was dangling and `out` stayed empty (H3), but `-1` may never have been tried
  (F-1). Whether std's `create_new` on Windows opens with `FILE_FLAG_OPEN_REPARSE_POINT` (so a link at the name answers
  "exists" and is never followed) is **unchecked here** (no std source in this toolchain: `ls
  ~/.rustup/toolchains/1.97.0-x86_64-unknown-linux-gnu/lib/rustlib/` has no `src`; INFERRED from memory of std, not
  verified). If it follows, a dangling file link at a free-looking name makes the save create its temporary file outside
  the root and then rename the link over the target — no error, so Interface (3)'s "on failure, look" never runs. The
  spec states the outcome ("the entry and whatever it leads to are not changed") without the mechanism or a measurement.
  Q-1.
- **F-3 (Interface (3): bound and exhaustion).** "Next name" is bounded: the loop is `0..TEMP_ATTEMPTS` (100,
  `disk.rs:112`, `:433`), so there is no unbounded retry. Not stated: what a save answers when every one of the 100 names
  holds an entry and some of them failed with an error other than `AlreadyExists` (today's final error is
  `AlreadyExists`, "all 100 temporary file names are taken", `:445-448`, pinned on Unix and Windows by
  `all_temp_names_taken_fails_without_changes_then_recovers`, `:774-803`, no platform cfg). Also not stated: what
  happens if `symlink_metadata` of the name itself fails (neither "an entry exists" nor "nothing there"). The window
  between the failed create and the `symlink_metadata` look is harmless both ways (an entry appearing skips one name; one
  disappearing returns the create's own error; the failed create changed nothing). Q-3.
- **F-4 (Interface (1): which call detects, the window, and the Unix premise).** Detection: `fs::symlink_metadata` of each
  candidate by path (Interface (1)), the attribute read with `MetadataExt::file_attributes`; the later `fs::read`
  (`:533`), the writability open (`:497-500`) and the rename are by path and follow. That is the **same shape as the Unix
  side**: Unix does not use directory handles or `O_NOFOLLOW` in this file (`grep -n "NOFOLLOW\|openat\|custom_flags"
  crates/kiln-workspace/src/disk.rs` → nothing); it `lstat`s by path (`:317`) and opens by path, with `O_EXCL` only on the
  temporary file (`:412-416`). So the check-to-use window is the accepted residual limit on both platforms (`paths.md`
  "Changes during a request", Pinned (e)); the spec does not need a handle-based Windows walk to be "as Unix". Tight
  enough for two implementers: yes for the walk (an arm before the symlink arm in `locate_steps`, refusing
  `OutsideRoot`; my prototype is 5 lines, see §3) and for the listing (the entry's own attributes; `DirEntry::metadata`
  on Windows comes from the directory read, no open). The one place where two builds can differ is F-2.
- **F-5 (Interface (4) / Pinned (i): "every call" wider than the record).** E5 measured `read` only; F3/F5 `check_location`
  and `list_dir`, F8 `read`; G3 `read` and `write_atomic`. No save after removal (F) and nothing but `read` after a
  retarget (E5) was run. The sentences say "every call"; that is right only once `guard_t074` shows it on the head, as
  Pinned (i) already requires. Corrected the evidence lines (C-3).
- **F-6 (Pinned (i): "A junction or symbolic link given as `--root`").** E and G made junctions only (E1, E4, E6, G2, G4);
  no directory symbolic link was given as `--root` on the runner. The symbolic-link half is unmeasured; marked unchecked
  (C-4); Q-4 asks for it in the tests (the registry quote of #12 is about a *symlink* as `--root`,
  `platform-tests.toml:343`).
- **F-7 (Interface (5) #11).** The counterpart is "Interface (4), third bullet": a removed root and a new *directory* at its
  name (F4, F5). The Unix test is a root removed and replaced by a *link* to a new directory; on Windows that case (removed,
  then a junction at the name) was not probed (G is rename-away, not removal). Not a blocker: the quote ("only so that
  no other directory can take its identity") is met by F's case; noted for the test author.
- No other mismatch: the root cases (junction as root E2/E3/E5/E7; replaced G1-G5; removed F1-F8; `..` K2-K4) say what the
  record says, apart from F-5/F-6.

#### 2. Acceptance criteria
| AC | Testable / oracle | Fails on base | Verified where |
|---|---|---|---|
| AC-1 | yes (tests, hunk table, IR) | n/a (a guard of Unix) | local |
| AC-2 | yes (three commands) | n/a (a gate) | local |
| AC-3 | yes, equality per pair + "no entry has `task = "T-074"`"; variant named | yes on every platform: base holds 9 entries `pending`/`task = "T-074"` (`grep -c 'task = "T-074"' xtask/platform-tests.toml` → 9) | local + CI (core-crates `xtask`) |
| AC-4 | yes, exact line; variant named and **run** (§3: rejected) | yes: base prints `35 platform test(s) registered (19 pending)` (run, §3) | local + CI |
| AC-W1 | yes | relative file link: PROBE C1/C2; the other kinds hold at base (C1-C6) | CI only |
| AC-W2 | yes | holds at base (C7): a guard | CI only |
| AC-W3 | yes | the pair only (F-1, C-8) | CI only |
| AC-W4 | yes | holds at base: guards | CI only |
| AC-W5 | yes | the new tests do not exist on base | CI only |
- **F-8 (AC-1 and Pinned (d) contradict Pinned (i)).** "Every changed product line is under `#[cfg(windows)]` or adds
  `#[cfg(unix)]` to an unchanged line" cannot hold: Pinned (i) itself rewrites the module docs `disk.rs:94-96`, and the
  seven counterparts are new lines in the `#[cfg(test)]` module with no platform cfg (Interface (5)). Corrected (C-5):
  comments, doc comments and the `#[cfg(test)]` module are excepted; the IR clause is what proves Unix unchanged.
- **F-9 (AC-2: one command is not the implementer's).** `cargo clippy … --test guard_t074` needs the test author's file,
  which the implementer may not see before the join (§6 Independence); listed under "Local (implementer …)". Corrected
  (C-6): the test author runs it before handover, the reviewer after the join. The other two forms I ran on a prototype
  (§3): clean.
- **F-10 (pending criterion: counts missing).** It names the steps and files but not the counts to expect. From
  `tasks/ci-runs/T-073-round-2.md` ("Running unittests src\lib.rs — running 36 tests", acceptance_t073 1, guard_t073 10;
  core-crates `==== xtask: exit status 0` 279/0) and this task's +7 lib tests, the record should show: T-073's step lib
  **43 passed; 0 failed**, `acceptance_t073` 1, `guard_t073` 10; core-crates `xtask` **279 + n passed, 0 failed** where n is
  `acceptance_t074`'s test count from the test author's log; `guard_t074` all `ok` with the count from the test author's
  log; the seven counterpart names from the implementer's log. Also unstated: T-072's step reports **every**
  `crates/kiln-workspace/tests/*.rs` target, and some are red on Windows at base (`acceptance_t002`: the
  `#[cfg(windows)] panic!` at `acceptance_t002.rs:939-942`, the test `disk_save_replaces_via_rename_without_leftovers`
  that `T-073-round-2.md` lists as failing in the scorecard step); the criterion must say which exit statuses other than
  0 are expected and which tests in them may fail (see F-11). Added to the criterion (C-7); the `acceptance_t002` part
  waits on Q-5.
- **Criteria of absence or limit.** AC-3 ("no entry has `task = "T-074"`"; variant `task = "T-095"`), AC-4 (exact count;
  variant a `#[cfg(unix)]` counterpart — rejected by main's tool, §3), Interface (1) (variant: a relative link followed,
  C1), (2) (a junction listed `dir`), (3) (the save refused, H2). Missing a variant for Interface (3)'s "nothing that
  already exists is opened or changed": **a dangling file link at `.kiln-tmp-0` to a path outside — the outside path must
  still not exist** (the Unix test's own case, `disk.rs:752-758`); Q-2.
- Windows-only criteria (AC-W1 to AC-W5, and AC-3/AC-4 on Windows) are verified only by CI; the criterion names the steps
  (T-073's "Workspace lib and T-073 tests", T-072's general step, core-crates), the test files and now the counts (C-7).
- Not testable on the runner, and so unstated as a criterion (fine, but say so): Interface (3)'s "a failure with nothing at
  the name is returned as now" on Windows (no way found to make the create fail with nothing at the name); the reviewer
  reads it in the code.

#### 3. The nine registry entries and main's `no-skip` (run)
The nine are exactly those with `task = "T-074"` (`grep -c 'task = "T-074"' xtask/platform-tests.toml` → 9;
`platform-tests.toml:237-380`), each `platforms = "unix"`, and Interface (5) disposes of each once (table rows 2, 8-15). Main's
rule (`xtask/src/platforms.rs:985-1006`): a named counterpart must be a test fn (or module with tests) that exists on every
platform of the entry's complement; nothing forbids two entries naming one counterpart (#8 and #13) and nothing compares
what the counterpart asserts with the quote.
- Baseline, scratch clone at `9603d97`: `cargo xtask no-skip` → `PASS … 201 files scanned, 0 allowlisted skip(s), 35
  platform test(s) registered (19 pending)`.
- Mock-up of the planned registry (`/home/user/kiln-check-T074-scripts/mock_t074.py`: the nine entries rewritten to
  `counterpart = "<file>::<item>"` with `reason`/`task`/`expires` removed, #10 naming `guard_t073.rs::acw2_root_parent_
  replaced_is_outside_root`, #8 and #13 one test; seven stub tests with no platform cfg in `disk.rs`'s module choosing
  their branch by `cfg!(windows)`, and a `#[cfg(unix)]`/`#[cfg(windows)]` link helper each doing work) → `PASS … 35 platform
  test(s) registered (10 pending)`: **AC-4's line confirmed.**
- Variant: #14's counterpart naming `tests::new_file_gets_default_mode` (`#[cfg(unix)]`) → `FAIL … platform-counterpart:
  entry 34: counterpart … exists on linux, macos only, not on every one of windows`: **AC-4's variant is rejected.**
- Variant: the Windows twin of the helper with body `true` → `FAIL … inert-helper`: Interface (5)'s helper rule holds.
- `guard_t073.rs::acw2_root_parent_replaced_is_outside_root` (#10): exists, no platform cfg (`guard_t073.rs:486`; the file
  has `#![cfg(test)]` only, `:27`), so the registry accepts it (also in the mock-up run).
- Counterparts against their quotes (no tool checks this; for the reviewer): #8 quotes "through a symlink, the link's target
  is replaced and the link is kept" and its Windows counterpart asserts the opposite (refusal); acceptable only because
  `control-protocol.md` gains the Windows sentence (Pinned (i)). #13's quote ("A link whose target ends in `/` must lead to
  a directory") has no Windows form at all; naming #8's test is allowed by the tool, `none` with that reason would be the
  plainer record (note, no question: the director's D-250 gave all nine to this task). #12 quotes a *symlink* as `--root`
  while Interface (4) tests a junction (F-6, Q-4). #11: F-7.
- Prototype of the prescribed code shape (`/home/user/kiln-check-T074-scripts/proto_t074.py`: Interface (1) as an arm
  `#[cfg(windows)] Ok(meta) if is_reparse_point(&meta) => return Err(OutsideRoot)` before the symlink arm; (2) a
  `#[cfg(windows)]` re-binding of `kind` from `entry.metadata()`'s attributes; (3) `Err(_) if fs::symlink_metadata(&temp)
  .is_ok() => continue` in the Windows `create_temp`, plus `custom_flags(0x0020_0000)` (Q-1 option A); one counterpart
  shape with `symlink_file`/`symlink_dir` helpers): 21 added product lines with comments, after `cargo fmt` (`git diff -U0`
  counted above the test module). Results: `cargo clippy -p
  kiln-workspace --lib --locked --target x86_64-pc-windows-msvc -- -D warnings` → `Finished`, 0 warnings, **no C compiler
  stub**; the same with `--profile test` → `Finished`, 0 warnings; `cargo clippy --workspace --lib --bins --locked --target
  x86_64-pc-windows-msvc -- -D warnings` with the stub of `reviews/T-073-r1.md` (it must handle MSVC's `-Fo<path>` as
  well as `-o`; with `-o` only, `tree-sitter-rust`'s build fails) → `Finished`, 0 warnings; Linux `cargo test -p
  kiln-workspace --lib --locked` → 52 passed (51 at base + 1 shape test). Not linked or run for Windows.

#### 4. D-235 and D-249 (run)
- **D-235 (S-3): confirmed.** On the prototype + mock-up tree, main's tool (no file under `xtask/src` changed, so the
  clone's `cargo xtask` is main's): `fmt` PASS, `layers` PASS (11 crates, 14 edges), `ui-commands` PASS, `hygiene` PASS
  (692 tracked files clean), `schema` PASS (30 commands, `docs/commands.json` current), `scorecard` PASS (64 items: 47
  verified), `no-skip` PASS `35 … (10 pending)`. `cargo test --locked -p xtask --no-fail-fast` → 287 passed, 0 failed
  (summed over the `test result:` lines). `clippy` and `test` of the whole gate not run (the Linux lib tests and the three
  Windows clippy forms of §3 were).
- **D-249 (S-4), `grep -rlE '<path>' --include=*.rs crates apps xtask` on `9603d97`:** `disk.rs` → `xtask/tests/guard_t079.rs`
  only (22 names at `:1241-1262` and the no-cfg check `:505-511`; `every_linux_test_name_of_the_base_still_exists`,
  `:381`, wants a superset, so new names pass — confirmed by the `cargo test -p xtask` run). `platform-tests.toml` →
  `acceptance_t079.rs` (T-079's 20 only, passes), `acceptance_t071.rs`/`guard_t071.rs` (toy registries), the rest
  comments or the tool itself (`xtask/src/noskip.rs`, `platforms.rs`: §3). `paths.md` → `acceptance_t008_cli.rs:576-578`
  (contains `outside_root`, `not_a_file`: passes), comments in `disk.rs`, `vfs.rs`. `project.md` → five files, doc
  references only. `control-protocol.md` → 17 files (16 besides `acceptance_t008_cli.rs`, not 15: C-2), of which only
  `acceptance_t008_cli.rs:549-574` reads "Paths": the lexical bullet and the save bullet's first sentence verbatim
  (`:563-574`, the save bullet `:566-568`), and the absence of "Symbolic links are followed, including links that point
  outside the root" (`:557-560`); the planned sentence keeps both and says neither. No test contains "How symlinks are
  followed", "not yet stated", "Not run on Windows" or "Unix only" as a pin (grep: only a comment in
  `acceptance_t002.rs:169`). `guard_t074`, `acceptance_t074`: no hits. All of S-4 confirmed, with C-2's two numbers.
- **F-11 (blocker-level, missed by S-4 and Pinned (f): a main test pins on Windows what this task changes).**
  `grep -rln "symlink_file\|symlink_dir\|mklink" --include=*.rs crates apps xtask` → among product tests only
  `crates/kiln-workspace/tests/acceptance_t002.rs`. Its `save_through_symlink_updates_target` (`:945-973`) has **no
  platform cfg**, its helper's Windows twin makes a relative file link (`:188-191`, `symlink_file("target.txt", …)`), and
  it asserts the save goes through the link and keeps it. On Windows at base it **passes**: the scorecard step of T-073's
  round 2 ran `io.save-atomic`'s three tests and named only `disk_save_replaces_via_rename_without_leftovers` as failing
  (`tasks/ci-runs/T-073-round-2.md:52`), consistent with PROBE C1 `read sr: Ok`. After Interface (1) it fails on Windows
  (`Document::open` → `OutsideRoot`, `unwrap` panics). The test is listed by the scorecard item `io.save-atomic`
  (`scorecard/items/io.save-atomic.toml:5`) and by `guard_t079.rs:1301` (name only). T-072's general step runs
  `acceptance_t002` on Windows (it is a `crates/kiln-workspace/tests/*.rs` file), so the record will show a test that
  passed at base now failing, inside a target that is already red (`:939-942`). §8.1: after this task the test asserts on
  Windows the opposite of the specified behaviour. `acceptance_t002.rs` is not in `touches` (T-092's per Out of scope).
  Q-5.
- **T-072's planned tests (its spec only).** `acceptance_t072.rs` AC-9 runs the step's script with a fake `cargo` in a
  scratch tree of three test files, and must reject a fixed `--test` list (T-072 AC-9): it names no path of this task.
  `guard_t072.rs` G-1 to G-4 read the workflow only. Interface (6) runs every `crates/kiln-workspace/tests/*.rs`; its
  ruling (added 11:20Z) carries `--show-output`, on which AC-W1-W4's `MEASURED:` lines depend: confirmed in T-072's
  rulings, still unchecked against T-072's branch (not read). Nothing in T-072's spec pins a path of T-074.

#### 5. Pinned: commands run
| Sentence | Command (this check) | Result |
|---|---|---|
| "`git diff --stat 8bf4c3a 72522f2 -- crates apps xtask .github docs/protocol` is empty" | the same, in the clone | empty: holds |
| Goal: nine entries `task = "T-074"`, expiring 2026-12-31, `:238-380` | `grep -c 'task = "T-074"'`; read `:237-380` | 9, all `2026-12-31`: holds |
| (f) "1,121 Linux test names … 22 of `disk.rs` (`:1241-1262`)", `:506-511`, `:515-516` | `grep -n`/`grep -c "kiln-workspace/src/disk.rs::" xtask/tests/guard_t079.rs` | 22 at `:1241-1262`; `BASE_COUNT: usize = 1121` at `:516`: holds |
| (f) "The 8 entries quoting `paths.md`" | `grep -n 'doc = "docs/protocol/paths.md"' xtask/platform-tests.toml` | **7** (8 lines, one the template comment `:13`): corrected (C-1) |
| (f) "Tests pinning what this task changes: none on Linux" | `grep -rln "symlink_file\|symlink_dir\|mklink" --include=*.rs crates apps xtask` | true on Linux; **false on Windows** (`acceptance_t002`, F-11) |
| AC-1 "the 28 of `ws.path-confinement`" | count of `tests` in `scorecard/items/ws.path-confinement.toml` | 28: holds |
| AC-4 `35 … (10 pending)` (19 − 9) | main's `no-skip` on the mock-up (§3) | holds; base 19 pending |
| AC-2 "`kiln-workspace` needs no C compiler stub" | §3 clippy runs without `CC_*` set | holds for `--lib` and `--lib --profile test` |
| Interface (5) "run on Windows in T-073's reported step (`-p kiln-workspace --lib`)" | `grep -n "acceptance_t073" .github/workflows/platforms.yml` | `:199`, `--lib --test acceptance_t073 --test guard_t073 -- --show-output`: holds |
| Pinned (i) control-protocol: "`acceptance_t008_cli.rs:560-566`" | `grep -n "A save replaces the file\|lost a Paths bullet" apps/kiln-cli/tests/acceptance_t008_cli.rs` | bullet at `:566-568` in the loop `:563-574`: corrected (C-2) |
| S-4 "`:549-570`", "the other 15 files" | `sed -n 540,585p …`; `git grep -lE 'control-protocol.md' 72522f2 -- '*.rs' \| wc -l` | `:549-574`; 17 files, so 16 others: corrected (C-2) |
| Pinned (i) K sentence, Unix side "unchecked by a command here" | `readlink -f k/link/../b` in a scratch tree | physical `a/b`: evidence added (macOS unchecked) |
| Pinned (i) F sentence "every call", root sentence "junction or symbolic link" | the probe record | narrower than the sentence: evidence lines corrected (C-3, C-4) |
| Estimate "T-073 … booked 860,977 (D-252)" | `grep -n "^| D-252" docs/decisions.md` | holds |
Every other Pinned sentence for a document carries PROBE lines, "`guard_t074.rs` on the head" or "unchecked"; none was
found without one after C-3/C-4. Line references checked and holding: `disk.rs:11-13`, `:94-96`, `:272-375`, `:300-302`,
`:335-337`, `:431-449`; `paths.md:50-51`; `project.md:57`, `:79-80`; `control-protocol.md:141`; `platform-tests.toml:9-18`.
- **F-12 (fixture location of the seven counterparts).** The probe ran in `CARGO_TARGET_TMPDIR` on `D:` (B6, I9); the
  counterparts live in the lib's test module, where `CARGO_TARGET_TMPDIR` is not set (cargo sets it for integration tests
  only), so they use `fresh_dir` under `std::env::temp_dir()` (`disk.rs:620-626`), on the runner's `C:` volume (8.3 names
  enabled there, I9). The 36 lib tests already pass there (T-073-round-2), so the root handling is shown; `symlink_file`,
  `symlink_dir` and `mklink /J` on `C:` are unprobed (INFERRED to work: the privilege is per process, not per volume).
  Note for the implementer; first CI round shows it.

#### 6. Scope, roles, estimate
- **Cut with T-095:** clean. One sentence to watch: Interface (1)'s "the location is the root joined with the request's
  segments" is not literally true for a segment `C:x` (`Path::join` replaces the root, PROBE I10); the walk still ends
  outside (I1-I3 `OutsideRoot`), and the lexical refusal is T-095's. No change asked; T-095's specification should
  restate it. Nothing of T-095 needs to move here or the reverse.
- **One role per path:** kept (implementer: `disk.rs` incl. its test module, the registry, three documents; test author:
  `guard_t074.rs`, `acceptance_t074.rs`). Q-5 A would add `acceptance_t002.rs` for the test author.
- **Estimate.** T-073 (860,977) had about 125 changed `disk.rs` lines, 1,046 lines of new test files, 160 registry lines,
  a dependency, 2 CI rounds (`git diff --stat 7c44427b16db 8bf4c3a -- crates apps xtask docs .github Cargo.lock`). T-074:
  fewer product lines (prototype: 21), but seven portable unit tests that make links and junctions whose Windows branches
  the implementer can only type-check (their first run is CI), a guard over a larger matrix (four link kinds × three
  destinations × two positions × six operations, plus four root cases), the registry, six document places, and F-11.
  780,000 is plausible only if the first CI round is clean. **My estimate: about 880,000** (≈800,000 with two clean CI
  rounds; ≈1,000,000 with a third round); the 1,100,000 ceiling holds.

#### Corrections made (plain errors)
- C-1: Pinned (f), S-3: "8 entries/sentences quoting `paths.md`" → 7.
- C-2: Pinned (i) `acceptance_t008_cli.rs:560-566` → `:563-574` (bullet `:566-568`); S-4 `:549-570` → `:549-574`, "the
  other 15 files" → 16. In T-095: `acceptance_t008_cli.rs:560-563` → `:563-574`, lexical bullet `:564-565`.
- C-3: Pinned (i) F sentence: evidence narrowed to the calls the probe ran.
- C-4: Pinned (i) root sentence: the symbolic-link half marked unchecked.
- C-5: Pinned (d), AC-1: comments, doc comments and the `#[cfg(test)]` module excepted from "every changed product line".
- C-6: AC-2: the `--test guard_t074` clippy form is the test author's and the reviewer's.
- C-7: pending criterion: expected counts and the rule for other targets in T-072's step.
- C-8: AC-W3: fail-on-base evidence is the pair only.
- (also) Pinned (f): the Windows pin of F-11 recorded; Pinned (i) K sentence: the Unix side's command added.

#### Questions for the director (design choices; not changed here)
- **Q-1 (F-2) How does the Windows create avoid following a link at a temporary name?** A: pin it: the Windows
  `create_temp` opens with `OpenOptionsExt::custom_flags(0x0020_0000)` (`FILE_FLAG_OPEN_REPARSE_POINT`, std, no
  dependency; my prototype is clippy-clean for Windows), so a link there is never followed whatever std does. B: a second
  probe of base `create_new` on a dangling link alone, then decide (one CI round and a record, about 60,000-80,000
  tokens, and a day's latency). C: rely on std as it is (unchecked). **Recommendation: A** (a line and a test case, about
  5,000 tokens), with Q-2's case as its test.
- **Q-2 (F-1) Cases for Interface (3).** Add to counterpart #2 and `guard_t074`: H's pair exactly (junction at
  `.kiln-tmp-0`, file link at `.kiln-tmp-1`: the only fail-on-base evidence), and a dangling file link to an outside path
  alone at `.kiln-tmp-0` (the outside path must still not exist afterwards: the Unix test's case). **Recommendation: yes**;
  about 10,000 tokens.
- **Q-3 (F-3) Exhaustion and a failing look.** State in Interface (3): when every one of the 100 names holds an entry the
  save fails `AlreadyExists` ("all 100 temporary file names are taken") as on Unix, whatever the creates answered; if
  `symlink_metadata` of the name fails with anything but `NotFound`, the create's own error is returned. **Recommendation:
  state it, no new Windows test** (100 junctions cost 100 `mklink` spawns; the existing no-cfg
  `all_temp_names_taken_fails_without_changes_then_recovers` covers plain files on Windows); the reviewer reads the code.
  Nil cost.
- **Q-4 (F-6) A symbolic link as `--root`.** A: add a directory symbolic link given as `--root` (made by `symlink_dir`, PROBE
  A2) to `guard_t074`'s root cases and to counterpart #12, so the sentence is measured. B: narrow the sentence to
  junctions. **Recommendation: A** (about 5,000 tokens; #12's quote is about a symlink).
- **Q-5 (F-11) `acceptance_t002::save_through_symlink_updates_target` on Windows.** A: add
  `crates/kiln-workspace/tests/acceptance_t002.rs` to `touches` for the test author: that test's Windows branch
  (`cfg!(windows)`) expects `outside_root` at open and save with link and target unchanged; the Unix branch, the name
  (`guard_t079.rs:1301`) and `io.save-atomic`'s list unchanged; T-092 (draft, after this task) then starts from it. About
  30,000-40,000 tokens. B: leave it to T-092 and have the record name it as the one test expected to turn from pass to
  fail on Windows; no cost now, but for that interval a test asserts on Windows the opposite of the specified behaviour
  (§8.1) and `io.save-atomic` gains a Windows failure. **Recommendation: A.**

#### Verdict
**Ready after rulings** on Q-1 to Q-5 (Q-5 is the one that would otherwise surface as an unexplained Windows failure in
the CI round; Q-1 decides whether Interface (3) holds by construction). No rework of the design: the walk rule, the
listing kind, the root cases, the registry plan and the gate-tool side (D-235) check out by run.

## Log
### director
<!-- end director log -->
### planner
- 2026-11-15 · R-255 · draft · did: filed from ADR-0009 · next: director
- 2026-11-18 · re-specification under v1.5 (main `72522f2`; scratch clone `/home/user/kiln-plan-T074`) · did: read the
  methodology, T-073's spec, reviews and CI records, ADR-0009, the code and registry; proposed the cut (T-095 drafted);
  measured: `cargo clippy -p kiln-workspace --test probe_t074 --locked --target x86_64-pc-windows-msvc -- -D warnings`
  clean with no C compiler stub; main's `acceptance_t070` 14 passed and `guard_t070` 7 passed with a separate reported step
  added to `gate-windows` (not committed); probe commit `de8106fe621e26ec907b6e1a2a3c068438888992`. One read-only `git log`
  was run in the primary checkout by mistake. CI-REQUEST: de8106fe621e26ec907b6e1a2a3c068438888992 /home/user/kiln-plan-T074
  · next: director
- 2026-11-18 · resumed with D-254 and the probe record (run 44290286460, read in full) · did: finalised the spec: `touches`
  reduced to seven paths (no workflow, no `acceptance_t070.rs`); Interface (3) added from PROBE H (a fix: a taken name is
  skipped; expectation `.kiln-tmp-2` was wrong, the save failed with os error 5); Interface (4) from PROBE E, F, G, K (F and
  K each get a `paths.md` sentence; K corrects T-073 r1's reading); PROBE J showed nothing measurable (my expectation
  `OutsideRoot` was wrong), no sentence; each AC names its PROBE lines; S-1 checked T-072's Interface (6) against this
  task's files; names, streams, case and short names moved to T-095 as measured base behaviour. Tier high (confinement
  boundary, persistence path, gate paths); Pinned (h) n/a. No second probe. Not run: any prototype of the change; anything
  on Windows myself · next: fresh spec check, director
- 2026-11-18 · spec check: fresh planner (high tier, confinement boundary), against main `9603d97` (drift from `72522f2`:
  `git diff --stat 72522f2 9603d97 -- crates apps xtask .github docs` → `docs/decisions.md` only); scratch clone
  `/home/user/kiln-check-T074` · did: section "Spec check (fresh planner)" (findings F-1 to F-12, corrections C-1 to C-8, questions
  Q-1 to Q-5; T-095: one line reference); ran main's `no-skip` on a mock-up of the planned registry and three variants, main's seven gate steps and
  `cargo test -p xtask` on a prototype tree, Windows clippy of a prototype of Interface (1)-(3) · verdict: ready after
  rulings · not run: anything on Windows; T-072's branch (not read) · state left `draft` · next: director
- 2026-11-18 · D-256 rulings written in (Q-1 to Q-5, estimate 880,000; F-1 to F-12 each closed, list under "Rulings on the
  fresh spec check") · Q-5 form by run in the clone: an `if cfg!(windows) { …; return; }` form → main's `no-skip` FAIL
  `early-return`; `if cfg!(windows) { … } else { today's body }` → PASS `35 … (19 pending)`, Linux `cargo test -p
  kiln-workspace --test acceptance_t002 save_through_symlink` 1 passed, `cargo clippy -p kiln-workspace --test
  acceptance_t002 -- -D warnings` clean for Linux and `x86_64-pc-windows-msvc`, `cargo test -p xtask --test guard_t079`
  passed · fixed the seven counterpart names and the registry text · clone's `target/` deleted · state left `draft`
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
