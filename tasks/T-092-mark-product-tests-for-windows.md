---
id: T-092
title: "Make every test target of kiln-workspace compile, lint clean and pass on Windows: each test either asserts every platform's specified behaviour or is registered as a Unix test with its counterpart or reason; the two pending disk.rs entries get their Windows counterparts"
tier: high              # METHODOLOGY v1.5 §4: gate paths (xtask/platform-tests.toml, xtask/tests/); it changes what confinement and persistence tests assert (the save and the walk of disk.rs); the approval row must say `gate paths`. No product code, no dependency change, no document change.
state: draft
owner_role: planner
scorecard_items: []     # none claimed: the ws.*/io.* items are verified on Linux; per-platform confirmation is the evaluator's (ADR-0009 Decision 3)
depends_on: [T-073, T-074, T-072]   # ruling 7 (D-269). T-073 merged (8bf4c3a); T-072 merged (D-268; its general reported Windows step, this task's evidence, is on main); T-074 approved by review, not merged when this was finalized (shares disk.rs's test module, xtask/platform-tests.toml, acceptance_t002.rs): a fresh spec check by another planner follows once T-074 is on main (§5.7).
touches:                # one role per path (D-153). All implementer except the last two (test author).
  - crates/kiln-workspace/tests/acceptance_t002.rs          # implementer: the one test disk_save_replaces_via_rename_without_leftovers (Interface row W1)
  - crates/kiln-workspace/tests/acceptance_t002_r2.rs       # implementer
  - crates/kiln-workspace/tests/acceptance_t008.rs          # implementer
  - crates/kiln-workspace/tests/acceptance_t015_tools.rs    # implementer
  - crates/kiln-workspace/tests/acceptance_t019_workspace.rs   # implementer
  - crates/kiln-workspace/tests/acceptance_t036_workspace.rs   # implementer
  - crates/kiln-workspace/tests/confinement.rs              # implementer
  - crates/kiln-workspace/tests/guard_t008.rs               # implementer
  - crates/kiln-workspace/tests/guard_t036_removal_workspace.rs   # implementer
  - crates/kiln-workspace/tests/guard_t036_workspace.rs     # implementer
  - crates/kiln-workspace/tests/project.rs                  # implementer
  - crates/kiln-workspace/src/disk.rs                       # implementer: its #[cfg(test)] module only (two new tests, Interface row W40-W41)
  - xtask/platform-tests.toml                               # GATE PATH: implementer: exactly the entries of Interface (2)
  - xtask/tests/acceptance_t092.rs                          # GATE PATH: test author: AC-1 (fails on base)
  - xtask/tests/guard_t092.rs                               # GATE PATH: test author: AC-2 (passes on base)
budget_tokens: 1100000  # high (v1.5); estimate about 950,000 (see Estimate)
iteration: 2
found_by: planner       # spec check R-321 of T-079 (D-233, ruling Q-4 B)
origin_task: T-079
---

# T-092 The kiln-workspace tests on Windows

**Finalized** on 2026-11-18 after the probe (run 44355322518, record `tasks/ci-runs/T-092-probe-1.md`, cited "PROBE") and
the director's rulings on Q-1 to Q-7 (D-269), against main `6a38a2b` (T-096 and T-072 merged) plus `task/T-074` at
`ed02d3e`: their code equals the tree first used (`git diff --stat proto/T-092 <main+T-074+prototype>` over `crates apps
xtask .github docs/protocol` is empty, in the planner's clone). First specified under v1.5 by a planner run on 2026-11-18 against main `bcca328` (= `2e1861d` plus a ledger commit:
`git diff --stat 2e1861d bcca328` shows `STATE.md`, `handoffs/2026-11-18.md` only), **with the trees of T-074 (`task/T-074`
at `ed02d3e`) and T-072 (`task/T-072` at `3fcba00`) as the base** (both merge before this task; the planner's clone merged
both into main without conflict, branch `plan-base` = `e538a17` in `/home/user/kiln-plan-T092`). This is the first of three
tasks (cut below); the draft's whole aim (`gate-windows` green) is reached by the third, T-099.

Labels: **VERIFIED** (a command run here, or a CI record quoted), **INFERRED**, **unchecked** (no record shows it; the
probe of "Probe" or this task's first CI round measures it).

## Goal
On Windows `kiln-workspace`'s library and its T-073/T-074 tests pass (VERIFIED: `T-074-early-2.md`, lib 43, `guard_t074`
15), but of its 15 integration-test targets 9 do not compile and 2 fail (VERIFIED, `T-072-round-3.md` and
`T-074-early-2.md`, the general step's status lines: `acceptance_t002_r2`, `acceptance_t008`, `acceptance_t019_workspace`,
`acceptance_t036_workspace`, `confinement`, `guard_t008`, `guard_t036_removal_workspace`, `guard_t036_workspace`, `project`
exit 101 with `error[E0433]: cannot find unix in os` first; `acceptance_t002` 14 passed 1 failed
(`disk_save_replaces_via_rename_without_leftovers`, panic at `acceptance_t002.rs:940:5`, "precondition: permission bits
and inode numbers are Unix metadata; …"); `acceptance_t015_tools` 2 passed 1 failed (`no_tools_never_runs`, panic at
`:131:5`, "precondition: the fake `cargo`/`rustfmt` of this test are shell scripts and need Unix")). Two Unix-only tests of
`disk.rs` wait on this task (`counterpart = "pending"`, `task = "T-092"`).

After this task, on Windows: **every `crates/kiln-workspace/tests/<t>.rs` target compiles, is clippy-clean with
`-D warnings`, and passes**; each test either asserts on every platform that platform's specified behaviour, or is a Unix
test listed in `xtask/platform-tests.toml` with its document sentence and the test that checks that sentence on Windows
(or `none` and why); the two pending entries name their Windows counterparts. Linux and macOS: no test name disappears
(`xtask/tests/guard_t079.rs`), no Linux assertion is weakened, the gate stays green.

Gate paths in `touches`: `xtask/platform-tests.toml`, `xtask/tests/acceptance_t092.rs`, `xtask/tests/guard_t092.rs`. No
workflow edit: T-072's general reported step (`kiln-workspace test targets (reported, not required)`, D-254 Q-5 A) and
T-073's step report every target of this task.

Reading list: ADR-0009 Decision 4; METHODOLOGY v1.5 §5.7, §6, §8.1; `xtask/src/platforms.rs` (the six cfg forms,
`unpaired_statements`, `inert_helpers`, `apply`); `xtask/platform-tests.toml` (after T-074); `tasks/ci-runs/T-072-round-3.md`
"gate-windows, T-073 step and the 14-target step"; `tasks/ci-runs/T-074-early-2.md` lines 170-500; `tasks/ci-runs/T-074-probe-1.md`
(links on the runner); `tasks/T-074-windows-confinement-rules.md` Interface (1)-(7); `docs/protocol/paths.md`,
`docs/protocol/project.md:57-90`, `docs/control-protocol.md:133-141`; `crates/kiln-workspace/tests/guard_t073.rs`
(`acw2_*`: the Windows forms of the root and the save); `xtask/tests/guard_t079.rs` (both guards).

## The inventory (by runs)
**Command** (planner's clone, tree `plan-base`; stub C compiler `/home/user/kiln-plan-T092-tools/stub-cc` writing an empty
object for every `-o`/`-Fo` output, `AR_x86_64_pc_windows_msvc=llvm-lib`, as `reviews/T-073-r1.md` "Windows stand-ins"; the
stub changes no Rust diagnostic, nothing is linked):
`cargo clippy --workspace --all-targets --locked --keep-going --target x86_64-pc-windows-msvc --message-format=json`,
summarised per target by `scripts/summarise.py` (kept beside the logs in `/home/user/kiln-plan-T092-tools/`). Exit 101.
VERIFIED: **26 targets do not compile, 199 errors**, all in test code; every library and binary compiles; one more target
compiles with warnings that `-D warnings` makes errors.

| Package | Target (errors) | First errors: the Unix-only API |
|---|---|---|
| kiln-workspace (9 targets, 84) | `acceptance_t002_r2` (11), `acceptance_t008` (20), `acceptance_t019_workspace` (5), `acceptance_t036_workspace` (16), `confinement` (24), `guard_t008` (1), `guard_t036_removal_workspace` (3), `guard_t036_workspace` (1), `project` (3) | `std::os::unix` (E0433) in every one; `Permissions::from_mode`/`mode` (t002_r2, t036, confinement); `FileType::is_fifo`/`is_socket`/`is_char_device` (t008, t019, confinement, project); `Metadata::ino`/`dev` (t002_r2, t008, confinement, guard_t036_removal); `Metadata::uid`/`gid`, `Command::uid` (t036, confinement) |
| kiln-workspace (warnings) | `acceptance_t015_tools`: 3 `dead_code` (`TIMEOUT`, `wait_with_timeout`, `MARKER_ENV`: used only by the Unix branch) | — |
| kiln-commands (2, 7) | lib test (`doc.rs` test module, 4), `acceptance_t037_commands` (3) | `unix` (`PermissionsExt`, `symlink`), `from_mode`, `dev`/`ino` |
| kiln-cli (11, 94) | `acceptance_t008_cli` (15), `acceptance_t019_cli` (3), `acceptance_t023_cli` (1), `acceptance_t036_cli` (15), `acceptance_t037_cli` (16), `acceptance_t037_removal_cli` (5), `guard_t019_cli` (2), `guard_t036_cli` (1), `guard_t036_removal_cli` (17), `guard_t037_cli` (16), `guard_t038_cli` (3) | `unix`, `dev`/`ino`, `is_fifo`/`is_socket`, `uid`/`gid`/`Command::uid`, `from_mode`/`mode`, `ELOOP` (guard_t038_cli) |
| kiln (4, 14) | `acceptance_t055` (2), `acceptance_t065` (3), `script_t055` (1), `window_t065` (8) | `unix::fs::symlink`; `OsStr::from_bytes`, `OsString::from_vec` (arguments not UTF-8) |

Consistent with CI (VERIFIED): the 9 `kiln-workspace` targets are exactly the 9 `exit status 101` compile failures of
`T-074-early-2.md` lines 193-202; CI's clippy kept text names `kiln-cli` targets (`acceptance_t019_cli` round 1,
`acceptance_t008_cli` T-073 round 2, `guard_t037_cli` T-072 round 3) and the test step names the 4 `kiln` targets, because
cargo stops at the first failing targets: in `T-072-round-3.md` the full test output has 4 `could not compile` lines and
0 `test result:` lines while 9 more targets are broken. Targets that compile but have never run on Windows: every target
of `kiln-commands` (16), `kiln-control` (2 + lib), `kiln-ui` (lib), `kiln-testkit` (lib), `kiln-cli` (23 + bin), `kiln`
(9 + bin) and the doc tests; the probe (below) ran each: 4 fail, all in `kiln-cli` (T-099), the rest pass. The draft's figures
(225 errors in 27 files, R-321 probe 2) predate T-073 and T-074: `kiln-workspace` fell from 111 to 84 (the `disk.rs` test
module now compiles), `kiln-cli` 93 → 94.

**This task's share:** the 9 non-compiling targets, `acceptance_t015_tools` (warnings and one failing test), one test of
`acceptance_t002`, and the two pending `disk.rs` entries. `kiln-commands`, `kiln-cli`, `kiln` are T-098's; the failures of
targets that compile today are T-099's (Cut).

## The sorting rule (every test of the files in `touches`)
Applied in this order; the first that fits decides. A *facility* is what the test makes or reads.
1. **Portable (P).** The test uses only facilities Windows has, even if spelt with a Unix-only API: links
   (`std::os::windows::fs::symlink_file`/`symlink_dir`; VERIFIED makeable on `windows-2025`: `T-074-probe-1.md` A1, A2),
   renames, hard links (`std::fs::hard_link`; VERIFIED by `guard_t073::acw2_save_replaces_target_by_rename_and_leaves_no_temp`
   passing there), the read-only attribute (`Permissions::set_readonly`/`readonly`, std on every platform). It keeps its
   name and carries no platform cfg attribute on the fn, its modules or its file. It asserts each platform's specified
   behaviour; where a document states a Windows difference (T-074: on Windows every reparse point on the walk answers
   `outside_root`; `paths.md` after T-074) the expected value is chosen in the test with `cfg!(windows)` (the form T-074
   used in `acceptance_t002.rs`, accepted by main's `no-skip` without an entry, D-256 Q-5). Helpers that differ by platform
   are twins with a real body on each (`#[cfg(unix)]`/`#[cfg(windows)]`; `no-skip`'s `inert-helper` rule). Statements behind
   platform cfgs inside a body cover the three platforms together (`platform-statement` rule).
2. **Unix part, registered (U).** The test's point is a facility Windows does not have: FIFOs, Unix-domain sockets,
   character devices (`/dev/null`); permission mode bits and the umask; user and group ids (`uid`, `gid`, `chown`,
   `Command::uid`, switching identity to make a root unreadable); inode numbers as the thing asserted (inode reuse);
   non-UTF-8 byte names. That part is behind `#[cfg(unix)]` (`#[cfg(target_os = "linux")]` where macOS lacks it too) and
   listed with the sentence it tests and its counterpart: the test that checks the same sentence on Windows if one exists
   in the tree after T-074 or this task writes it, else `none` with the reason. Never `pending` (no later task owns these).
   - If the whole test is such a part, the cfg goes on the test and the entry names it.
   - If the test also checks things every platform has, it is **split**: the original name keeps the portable part (rule 1)
     and a new test `<name>_unix` holds the Unix part (T-079's `_non_utf8` precedent, ruling w1), whose entry may name the
     original as its counterpart when the original checks the same sentence on Windows.
   - If every test of a file is U with one sentence and one counterpart, the file carries `#![cfg(unix)]` and one entry
     `item = "<file>"`.
3. **Identity used as evidence, not as the point.** stable Rust exposes no file identity on Windows (VERIFIED:
   `rustc +1.97.0 --target x86_64-pc-windows-msvc` on a use of `std::os::windows::fs::MetadataExt::{volume_serial_number,
   file_index, number_of_links}`: `error[E0658]: use of unstable library feature windows_by_handle`, three times;
   `file_attributes()` compiles). Where an inode only shows "replaced by a rename, not written in place", the test shows
   it on every platform with a hard link to the old file (as `guard_t073` does) and keeps the inode check as a Unix
   statement; where an inode is only in a message (`note`), the message drops it on Windows; where a snapshot's inode
   field guards decoys, see Q-2.

Two builders sort a test the same way by reading what it makes (the fixture) and what it asserts: the table below is
this rule applied by the planner from the sources (facility tags from `scripts/classify.py`, then each test read); a
builder who would sort a row otherwise logs a `QUESTION:` and does not change the row silently.

Against `xtask/tests/guard_t079.rs` (1,121 Linux names, `BASE_COUNT`): P keeps the name and builds on Linux; U keeps it
(`unix` and `target_os = "linux"` hold on Linux); a split keeps the original name and adds one; `#![cfg(unix)]` on a file
keeps every name. No row renames, removes or moves a test, so `guard_t079.rs` is not in `touches`. Its AC-2 guard forbids
platform cfg attributes in four named tests; one is in a file of this task's crate
(`disk.rs::tests::failed_rename_removes_own_temp_and_keeps_target`) and no row touches it.

## Interface
### (1) The tests (base: T-074's tree)
P = portable, name kept, no entry. P/w = portable with the Windows expectation chosen by `cfg!(windows)` (T-074's
documented Windows walk). U = Unix test, entry. S = split (original P, new `<name>_unix` U).

| Row | File :: test | Facility | Treatment | Counterpart (U) |
|---|---|---|---|---|
| W1 | `acceptance_t002.rs` :: `disk_save_replaces_via_rename_without_leftovers` | mode, inode | P: the `#[cfg(windows)] panic!` goes; on every platform a hard link to the old file shows the rename; Unix statements keep mode and inode; Windows statements check the read-only attribute unchanged (both kinds in each block, so the block covers the three platforms) | — |
| W2-W9 | `acceptance_t002_r2.rs` :: `save_leaves_regular_files_at_temp_names_untouched`, `save_never_writes_through_symlinks_at_temp_names`, `save_ignores_self_pointing_and_dangling_symlinks_at_temp_names`, `documents_named_like_temp_files_do_not_clobber_each_other`, `file_with_255_byte_name_opens_edits_and_saves`, `not_found_names_the_requested_path`, `save_onto_directory_is_not_a_file` (P); `save_through_symlink_with_decoys_changes_only_the_link_target` (P/w: refused on Windows, as `acceptance_t002::save_through_symlink_updates_target`) | links; the `support` module's `fresh_dir`, `restore_perms`, `snapshot` use modes | P; the `support` module becomes twins (Q-2 for the snapshot's identity field) | — |
| W10 | `acceptance_t002_r2.rs` :: `saved_file_is_never_more_readable_than_the_target` | mode bits | U | `none`: mode bits (as the `disk.rs` entries `temp_file_is_created_owner_only_…`) |
| W11 | `acceptance_t002_r2.rs` :: `save_to_unwritable_target_is_permission_denied_and_untouched` | mode bits | U | `crates/kiln-workspace/tests/guard_t073.rs::acw2_save_of_read_only_target_is_refused_and_changes_nothing` |
| W12-W14 | `acceptance_t008.rs` :: `open_through_escaping_symlinks_is_refused`, `save_through_escaping_symlinks_touches_nothing`, `link_swapped_in_after_open_is_refused_at_save` | links (outside) | P (the answer is `outside_root` on every platform); `snapshot` twin without the FIFO/socket branches on Windows | — |
| W15, W16 | `acceptance_t008.rs` :: `non_regular_files_are_refused_without_blocking`, `save_onto_non_regular_files_leaves_them_alone` | FIFO, socket, `/dev/null` | U | `none`: Windows has no FIFOs or Unix-domain sockets that std can make, and reserved device names are ordinary files under the walk (`T-074-probe-1.md` I1-I6, T-095's matter) |
| W17 | `acceptance_t008.rs` :: `outside_root_wins_over_not_found` | links; FIFO outside, link to `/dev/null` | S | `_unix` → `crates/kiln-workspace/tests/acceptance_t008.rs::outside_root_wins_over_not_found` |
| W18 | `acceptance_t008.rs` :: `root_swapped_between_requests_is_refused` | rename of the held root, link | P/w (VERIFIED possible: "rename of root while the root is held: ok", `T-072-round-3.md` MEASURED; PROBE `awa4_root_replaced_by_a_junction: windows: rename of the held root: Ok(())`) | — |
| W19 | `acceptance_t008.rs` :: `parent_swapped_between_requests_is_refused` | rename of the held root's parent | U: the fixture cannot be made on Windows (VERIFIED: "rename of p while the root is held: refused: kind PermissionDenied … os error 5", same record) | `crates/kiln-workspace/tests/guard_t073.rs::acw2_root_parent_replaced_is_outside_root` |
| W20, W21 | `acceptance_t008.rs` :: `root_removed_and_replaced_by_link_is_refused`, `root_removed_and_replaced_by_new_directory_is_refused` | removal of the held root; inode in the note only | P/w (VERIFIED possible: PROBE `awa4_root_removed_while_held: windows: remove_dir_all of the held root with a file: Ok(())`, then every call `OutsideRoot`, also with a new directory or a junction at its name) | — |
| W22 | `acceptance_t015_tools.rs` :: `no_tools_never_runs` | shell-script fakes | P (ruling 4, decided by PROBE P-D): the Windows branch builds the fakes `cargo.exe` and `rustfmt.exe` with `rustc` (P-D3 `status Some(0)`, 212 ms) and shows them working first by their marker (P-D4 `cargo with cargo.exe and cargo.cmd on PATH: status Some(0)`, marker `Ok("check")`). Script fakes must not be used for that first check: with only `cargo.cmd` or `rustfmt.bat` in the child's `PATH`, `Command::new` ran **the real tools** (P-D1 stdout `cargo 1.97.0 …`, P-D2 `rustfmt 1.9.0-stable …`, no marker): Windows' lookup is not limited to the child's `PATH`. The three helpers are then used on every platform, so the target is clippy-clean | — |
| W23 | `acceptance_t019_workspace.rs` :: `disk_list_dir_reports_entries_by_their_own_kind_and_skips_bad_names` | links; FIFO, socket | S: the original lists files, directories and links with the kinds `project.md` states per platform after T-074 | `_unix` → `none` (as W15) |
| W24 | `acceptance_t036_workspace.rs` :: `unreadable_root_is_refused_by_new` | mode 0311/0000, uid switch | U | `none`: modes and identities; on Windows unreadability is an access list, out of scope (ADR-0009 (b), `control-protocol.md:139` "Read and not measured") |
| W25 | `confinement.rs` :: `escapes_are_refused_at_read_write_and_is_file` | links (outside) | P | — |
| W26 | `confinement.rs` :: `links_inside_and_excursions_back_inside_are_followed` | links (inside) | P/w: on Windows each answers `outside_root` | — |
| W27 | `confinement.rs` :: `non_regular_files_are_refused_without_blocking` | FIFO, socket, `/dev/null` | U | `none` (as W15) |
| W28 | `confinement.rs` :: `swaps_between_open_and_save_are_checked_again` | links, renames; a FIFO swapped in | S | `_unix` → `crates/kiln-workspace/tests/confinement.rs::swaps_between_open_and_save_are_checked_again` |
| W29 | `confinement.rs` :: `walk_corner_cases` | link loops, chains, absolute links | P/w | — |
| W30 | `confinement.rs` :: `root_that_cannot_be_opened_for_reading_is_refused_by_new` | modes, uid, chown | U | `none` (as W24) |
| W31 | `guard_t008.rs` :: `symlinks_inside_root_still_work` | links (inside) | P/w | — |
| W32, W33 | `guard_t036_removal_workspace.rs` (both tests) | inode reuse as the point (`trial`) | U, file-level `#![cfg(unix)]`, one entry `<file>` | `crates/kiln-workspace/src/disk.rs::tests::root_removed_on_every_platform` |
| W34 | `guard_t036_workspace.rs` :: `retargeted_root_link_is_refused` | directory link | P (twin link helper) | — |
| W35 | `guard_t036_workspace.rs` :: `vanished_root_is_refused` | three cases: root renamed, **root's parent renamed**, root removed | S: the original keeps `RenameRoot` and `Remove` (both VERIFIED possible while held: rows W18, W20); `vanished_root_is_refused_unix` holds `RenameParent`, whose fixture cannot be made on Windows (as W19) | `_unix` → `crates/kiln-workspace/tests/guard_t073.rs::acw2_root_parent_replaced_is_outside_root` |
| W36-W38 | `project.rs` :: `disk_list_dir_reports_own_kinds_sorted_and_skips_odd_names`, `disk_list_dir_errors_follow_t008_precedence`, `symlinks_and_fifos_are_listed_not_followed` | links; FIFOs | S each | `_unix` → `none` (as W15) |
| W39 | `project.rs` :: `a_prefix_directory_outside_the_root_refuses_the_path` | links | P/w | — |
| W40 | `disk.rs` :: new `tests::leftover_at_temp_name_is_never_touched_on_every_platform` | read-only leftover at `.kiln-tmp-0` (`set_readonly`) | new P test; counterpart of `tests::leftover_at_temp_name_is_never_touched`. Windows behaviour VERIFIED at base (PROBE P-B1..B8: both saves `Ok`, `a` = `newer`, leftover bytes and `readonly` `true` unchanged, listing `[".kiln-tmp-0", "a"]`) | — |
| W41 | `disk.rs` :: new `tests::failed_save_after_writability_check_leaves_target_and_listing_unchanged_on_every_platform` | the three injected failures; snapshot of listing, bytes and read-only attribute | new P test; counterpart of `tests::failed_save_after_writability_check_leaves_target_and_listing_unchanged`. Case 1 VERIFIED at base on Windows (PROBE P-C1 `Err(Io { path: RelPath("f"), kind: AlreadyExists, message: "all 100 temporary file names are taken" })`, P-C2 listing unchanged, P-C3 bytes unchanged, P-C4 `readonly` `false`); cases 2-3 are injected and platform-free | — |
| W42 | `acceptance_t019_workspace.rs` :: new `disk_list_dir_reports_entries_by_their_own_kind_and_skips_bad_names_not_unicode` (`#[cfg(windows)]`) | a name with an unpaired surrogate (`OsStringExt::from_wide`) | new Windows test (ruling 3, decided by PROBE P-A); asserts `list_dir` omits it and counts it in `skipped` | `none`: Linux holds such names as bytes (its `_non_utf8` tests), macOS refuses them |
| W43 | `acceptance_t019_workspace.rs` :: new `tree_counts_skipped_names_on_disk_not_unicode` (`#[cfg(windows)]`) | the same name in a subdirectory | new Windows test (ruling 3); asserts `project::tree`'s `skipped` | `none` (as W42) |

PROBE P-A (Windows, at base): `create "b\u{d800}.rs": Ok(())`; read_dir lists it; read back `Ok([98])`; `list_dir(None):
Ok(DirListing { entries: [DirEntry { name: "ok.rs", kind: File }], skipped: 1 })`; `project::tree: Ok(ProjectTree { entries:
[… "ok.rs" …], ignored: 0, skipped: 1, too_deep: 0, truncated: false })`. So W42 and W43 pass on base on Windows: they
are the implementer's tests of existing behaviour, not acceptance tests.

Every other test of these files is unchanged in body (their only Windows obstacle was a file-level import).
**Counts** (rows): P 29 (W1; W2-W9 8; W12-W14 3; W17, W23, W28, W35, W36-W38 originals 7; W18, W20, W21; W22; W25, W26,
W29, W31, W34, W39); U on existing tests 10 (W10, W11, W15, W16, W19, W24, W27, W30, W32, W33); new `_unix` tests 7 (W17,
W23, W28, W35, W36-W38); new counterparts 2 (W40, W41); new Windows tests 2 (W42, W43).

### (2) The registry (`xtask/platform-tests.toml`)
After this task, the entries whose `file` is one of the 11 test files of `touches` are exactly 18 new ones,
`(file, item, platforms, counterpart)`: 16 with `platforms = "unix"` — W10, W11, W15, W16, W17`_unix`, W19, W23`_unix`, W24,
W27, W28`_unix`, W30, (W32+W33 as `<file>`), W35`_unix`, W36`_unix`, W37`_unix`, W38`_unix` — and 2 with `platforms =
"windows"` — W42, W43 — counterparts as in the table; **plus** the four T-079 non-UTF-8 entries of
`acceptance_t019_workspace.rs` and `project.rs`, whose `platforms` and `counterpart` stay exactly as T-079 wrote them
(`linux`, `none`; pinned by `xtask/tests/acceptance_t079.rs`), with `reason` no longer saying "the Windows side is T-092's"
but naming W42/W43 as the Windows form (ruling 3). And the two `disk.rs`
entries `tests::leftover_at_temp_name_is_never_touched` and
`tests::failed_save_after_writability_check_leaves_target_and_listing_unchanged` name W40 and W41
(`counterpart = "crates/kiln-workspace/src/disk.rs::tests::<W40|W41 name>"`, no `reason`, `task`, `expires`). Every `doc`/`quote`
is an existing sentence (`paths.md:112` "Directories, FIFOs, sockets and devices are refused"; `project.md:80` "FIFOs,
sockets and devices are listed with kind `other`"; `paths.md:13-15` for the unreadable root; the `decisions.md` and
`paths.md` sentences the `disk.rs` entries already quote); this task adds no document sentence. Other entries are not
changed. After this task no entry has `task = "T-092"`; pending entries: 8 (T-076's 3, T-077's 5).

## Pinned
- (a) Error variants, payloads, precedence: n/a: no product code changes; the tests assert the variants T-073/T-074
  specified (e.g. `VfsError::OutsideRoot { path }` for every reparse point on the Windows walk).
- (b) Files read or written: the 13 files of `touches` and the two `xtask/tests` files. Fixtures stay under
  `CARGO_TARGET_TMPDIR` as today.
- (c) Order: the implementer first commits the Windows check of its tree with the stand-in (AC-4), then asks for a CI round
  (`CI-REQUEST`). What the rows assume of Windows is VERIFIED by records (each row names it: PROBE, `T-072-round-3.md`,
  `T-074-probe-1.md`); what stays **unchecked** is only whether each rewritten P/w test passes there as written, which that
  first CI round measures (AC-3). A row whose Windows run disagrees with the table is a `QUESTION:`, never a silent cfg.
- (d) Platform assumptions: links on Windows are made with `symlink_dir` when the target, resolved from the link's
  directory, is an existing directory when the link is made, else `symlink_file`; junctions are not used by these tests
  (T-074's guards cover them). Paths are compared as `/`-joined relative strings, as the files do now.
- (e) Threat model: the tests in `touches` are the evidence for the confinement boundary and the save. The risk is a test
  that passes on Windows without checking: excluded mechanically by `no-skip` for cfg'd tests, statements and helpers,
  and by AC-2 for the P rows; the reviewer checks each P/w row's Windows branch asserts the documented answer (not merely
  "no panic") and each U row's counterpart checks the same sentence.
- (f) Tests pinning what this task changes: `xtask/tests/guard_t079.rs` (names: kept, see the rule; prototype run below);
  `xtask/tests/acceptance_t079.rs` (the four non-UTF-8 entries' `platforms`/`counterpart`: kept); `xtask/tests/acceptance_t074.rs`
  (its nine entries and "no `task = "T-074"`": untouched). None needs to change, so none is in `touches`.
- (g) Worked examples executed: the prototype forms below (main's `no-skip` on a file-level `#![cfg(unix)]` with and
  without its entry; a twin link helper compiled for Windows); the Windows behaviour W22, W40-W43 rest on: PROBE P-A to P-D,
  quoted in their rows.
- (h) Scorecard items: none claimed. The evaluator's call for a Windows confirmation is the CI record (ADR-0009 Decision 3).
- (i) Document sentences: none added or changed (n/a).

## Acceptance criteria
- **AC-1 (registry; test author, `xtask/tests/acceptance_t092.rs`).** Parsing `xtask/platform-tests.toml` with `toml` (any
  spelling of the array of tables), the set of `(file, item, platforms, counterpart)` of entries whose `file` is one of the
  11 test files of `touches` equals exactly Interface (2)'s 18 plus T-079's 4; the two `disk.rs`
  entries have exactly the W40/W41 counterparts and no `reason`, `task`, `expires`; no entry has `task = "T-092"`. Fails on
  base (the 18 are absent; the two are `pending`). Must reject: an entry `confinement.rs::walk_corner_cases` with
  `platforms = "unix"` (a P row registered); W19 with `counterpart = "none"`.
- **AC-2 (guard; test author, `xtask/tests/guard_t092.rs`).** Each P row's test (Interface (1), the 29 names) exists as exactly one `#[test]` fn and has no platform cfg attribute on the fn, its inline modules, the file
  (`#![cfg]`) or the file's `mod` declaration (the reading of `guard_t079.rs` AC-2, without its in-body clause: P rows may
  hold paired platform statements, which `no-skip` checks). Passes on base (none of these has such a cfg today). Must
  reject `#[cfg(unix)]` on `guard_t008.rs::symlinks_inside_root_still_work`.
- **AC-3 (pending CI criterion; record `tasks/ci-runs/T-092-round-N.md`).** On the head rebased on a main holding T-072 and
  T-074, in `gate-windows`: the step "kiln-workspace test targets (reported, not required)" prints exactly one line
  `==== <t>: exit status 0` per file `crates/kiln-workspace/tests/<t>.rs` of the tree (15 at T-074's tree) and no other
  status line; "Workspace lib and T-073 tests" prints `WORKSPACE TESTS: exit status 0` with `test
  disk::tests::<W40 name> ... ok` and `… <W41 name> ... ok`, and the general step's block for `acceptance_t019_workspace`
  has `test <W42 name> ... ok` and `test <W43 name> ... ok`. Base (VERIFIED, `T-074-early-2.md:173-202`, and PROBE's
  table lines 26-41): 10 of 15 exit 101.
  Must reject any `exit status 101`. `gate-windows`' clippy, scorecard and test steps still fail, and every `could not
  compile` line in their kept text and in `test-output.txt` names a package other than `kiln-workspace`. `gate-linux`,
  `window-linux` success; `gate-macos` success (or D-262's rule for a job the host cancels before any step, which this
  diff can bear on only through the test files it builds: the reviewer states it).
- **AC-4 (procedure; implementer first, reviewer again).** With the stand-in of "The inventory":
  `cargo clippy -p kiln-workspace --all-targets --locked --target x86_64-pc-windows-msvc -- -D warnings` exits 0 (base:
  exit 101, the 9 targets and the 3 warnings above); the same for `aarch64-apple-darwin` exits 0; on the host `cargo xtask
  ci` is green with main's tool (D-235).
- Guards that must keep passing (no change): `xtask/tests/guard_t079.rs` (both), `acceptance_t079.rs`, `acceptance_t074.rs`.

## Main's tool and main's tests on the planned tree (D-235, D-249), by runs
Run twice: first on `plan-base` (main `bcca328` + T-074 + T-072); again at finalization on main `6a38a2b` (T-072 and T-096
merged) + `task/T-074` `ed02d3e` + the prototype commit (branch `check2` = `84e57c1` in the planner's clone; code identical
to the first tree), with the same results below except `hygiene`'s file count (`716 tracked file(s) clean`).
Prototype (planner's clone, branch `proto/T-092` = `fb13257`; never merged): `guard_t036_removal_workspace.rs` given
`#![cfg(unix)]` and one entry `<file>` → `disk.rs::tests::root_removed_on_every_platform`; `guard_t036_workspace.rs` given
twin `symlink` helpers (`symlink_dir` on Windows). `xtask/src` of the base equals main's (`git diff --stat main plan-base --
xtask/src` empty), so `cargo xtask` there is main's tool. VERIFIED:
- `cargo xtask no-skip` → `PASS no-skip: self-test ok (17 known findings); 206 files scanned, 0 allowlisted skip(s), 36
  platform test(s) registered (10 pending), today = 2026-11-18`; without the entry → `FAIL no-skip: 1 skip problems: …
  guard_t036_removal_workspace.rs:22: <file>: platform-unlisted: …`.
- `hygiene`, `layers`, `ui-commands`, `schema`: PASS (`schema: 30 commands; audit ok; docs/commands.json current`).
- `cargo xtask scorecard --check` (host, builds and runs the listed tests) → `PASS scorecard: 64 items: 47 verified, 0
  implemented, 0 failing, 17 planned.`, exit 0 (first tree only; not repeated, the code being identical).
- `cargo clippy --target x86_64-pc-windows-msvc -p kiln-workspace --test guard_t036_workspace --test
  guard_t036_removal_workspace -- -D warnings` → `Finished` (no warning); the same on the host → `Finished`.
- `cargo test -p xtask --test acceptance_t079 --test acceptance_t074 --test guard_t079 --test guard_t071` → 2, 2, 6, 6
  passed, 0 failed.
- D-249 grep (`grep -rln <path> --include=*.rs xtask crates apps` for each path of `touches`): `guard_t079.rs` names every
  file (Linux names; kept); `acceptance_t079.rs` names `acceptance_t019_workspace.rs` and `project.rs` (their four
  entries' `platforms`/`counterpart`; kept); `acceptance_t074.rs` and `guard_t074.rs` name `disk.rs` (the nine entries; a
  test file); `guard_t071.rs` and `acceptance_t074.rs` read the registry (parse and the nine; kept);
  `acceptance_t005_r2.rs`, `acceptance_t005_r3.rs`, `acceptance_t071.rs`, `xtask/src/precheck.rs` name
  `acceptance_t002.rs`/`acceptance_t008.rs`/`guard_t008.rs` only as file names in scratch fixtures (`crates/a/…`,
  `crates/b/…`, a toy text), not these files; on main now `xtask/tests/acceptance_t072.rs:1578` names
  `crates/kiln-workspace/tests` only inside its scratch bench tree (`b.tree`). No main test pins a path this task changes in a way it breaks.
This task changes no gate rule (`xtask/src` untouched), so the D-235 rule ("does not also use it") does not bite.

## The pending CI criterion: which step reports what
- This task: T-072's general step (`windows-tests.txt`) and T-073's step (`workspace-tests.txt`) report every target it
  changes. No other step is needed.
- `kiln-commands`, `kiln-cli`, `kiln` have no per-target step. None is needed **if T-098 makes all their remaining
  targets compile in one task**: then nothing fails to compile on Windows, and the gate's own `test` step and "Full test
  output (gate failed)" run every target (cargo stops at build errors: VERIFIED above). If T-098 were split, its first
  part would have no Windows run of its files unless the general step were widened to other packages — a workflow edit
  whose test file `xtask/tests/acceptance_t072.rs` pins the step (`ac9_*`: exactly one writer of `windows-tests.txt`, one
  `cargo test` with `-p kiln-workspace`; `ac1_*`: the upload list and the step order), so that file would join that task's
  `touches` for its test author (Q-6).

## Probe (run; one round)
Commit `b57a74ef2a9773a02a284564d880bc853cec6ae7` (branch `probe/T-092` in the planner's clone; never merged); run
44355322518, job `probe-t092-windows` success in 9m37s, record `tasks/ci-runs/T-092-probe-1.md`: 96 targets, 64 exit 0,
26 do not compile, 6 compile and fail; 207 `MEASURED:` lines. Against the expectation written before it ran:
- P-0, the 26: **met** — exactly the 26 of the inventory (record lines 160-259).
- P-0, the rest: `kiln-workspace` as `T-074-early-2.md` (the same two failures, same messages) — **met**. The never-run
  targets: 4 fail, all `kiln-cli` (`acceptance_t004_r2`, `acceptance_t004_r3`: `spawn kiln-cli: … code: 206 … The filename
  or extension is too long.`; `guard_t015_cli`, `guard_t032_cli`: `session_run_files differs from the base recording`);
  every other target, the doc tests of all 6 libraries included, exits 0. They are T-099's inventory.
- P-A (unpaired-surrogate name made and skipped): **met** (W42, W43).
- P-B (read-only leftover at `.kiln-tmp-0`): **met** (W40).
- P-C (all 100 names taken): **met** (W41).
- P-D: **not met in one point**: `cargo.cmd`/`rustfmt.bat` alone were not "not found": `Command::new` ran the real
  `cargo`/`rustfmt` (P-D1, P-D2). The `rustc`-built `cargo.exe` ran and wrote the marker: **met** (W22).
- P-E (`kiln` with an unpaired-surrogate argument): **met** — `status Some(2), stdout 0 bytes, stderr 24 lines, first
  "kiln: argument N is not valid UTF-8", panicked false` in all four forms (T-098).

## Cut (the draft's aim in three tasks)
| Task | Scope | Measurable end | touches (dirs) | Tier | Estimate | Order |
|---|---|---|---|---|---|---|
| **T-092** (this) | `kiln-workspace`: 9 targets that do not compile, `acceptance_t015_tools`, one test of `acceptance_t002`, the 2 pending `disk.rs` entries | AC-3: 15 of 15 `exit status 0` (base 5 of 15) | `crates/kiln-workspace/tests/` (11 files), `disk.rs` test module, registry, `xtask/tests/` (2) | high | 950,000 | after T-072, T-074 |
| **T-098** (`tasks/T-098-kiln-cli-commands-and-app-tests-on-windows.md`) | `kiln-commands` (2), `kiln-cli` (11), `kiln` (4): every remaining target compiles under the same rule | `gate-windows` clippy step PASS; the test step builds; every test of its files passes on Windows or is registered absent | `apps/kiln-cli/tests/` (11), `apps/kiln/tests/` (4), `crates/kiln-commands/src/doc.rs` test module, `crates/kiln-commands/tests/acceptance_t037_commands.rs`, registry, `xtask/tests/` (2) | high | 1,000,000 | after T-092 (registry) |
| **T-099** (`tasks/T-099-gate-windows-green.md`) | the Windows failures of targets that already compile (probe, T-098's round) and the scorecard step | `gate-windows` conclusion success on the rebased head, then on main; **this task proposes counting `gate-windows`** (§8.1: it counts from its first green run on main) | `apps/kiln-cli/tests/` (4 files of the probe) and what the `xtask.exe` note needs | high (provisional) | 500,000-700,000 | after T-098 |

Each merges with the Windows gate strictly nearer to green: T-092 26 → 17 targets that do not compile and 2 → 0 known
failing tests; T-098 17 → 0 and clippy green; T-099 the remaining failures → `gate-windows` success.
Drafts (ruling 1, D-269): T-075 is absorbed by T-092 and T-098; T-076 is narrowed to ruling x6's three `/dev/full`
counterparts and runs after T-098 (shared `apps/kiln/tests/acceptance_t055.rs`; it also shares
`apps/kiln-cli/tests/acceptance_t004_r2.rs` with T-099, so the director orders those two); T-077 runs after T-098 (shared
`acceptance_t065.rs`, `window_t065.rs`); T-095 (after T-074) shares only the registry, and only if it adds an entry.

**Should T-092 itself be cut?** Not recommended. A clean line exists: (a) the save — `acceptance_t002.rs`,
`acceptance_t002_r2.rs`, `acceptance_t015_tools.rs`, `disk.rs` (W1-W11, W22, W40, W41; 7 entries incl. the two
`disk.rs` ones; end: those three targets exit 0 and `WORKSPACE TESTS` shows W40, W41 ok) and (b) the walk and listings —
the other 8 files (W12-W21, W23-W39, W42, W43; 13 entries; end: those 8 targets exit 0). But both parts edit the
registry and need their own `xtask/tests` acceptance file, so they run one after the other, each high with an independent
test author, a review of at least one round and a CI round: the fixed part of a high task here is about 450,000-500,000
(T-079: test author 211,253, reviewer 360,143 of 846,099; T-074: 265,811 and 348,575 of 897,911; `metrics/tasks.jsonl`),
so the two would cost about 1,300,000-1,400,000 against about 950,000 whole. A line by treatment (only portable rewrites
vs Unix parts) cuts across files (most files hold both kinds), so it is not clean. The risk of the whole task is an
overrun of the estimate, which the §11 `Extend` row covers; the reviewer's load is the larger part, and it is
proportional to rows, not to tasks.

## Estimate
About 950,000 of 1,100,000: implementer about 380,000 (11 files, 43 rows, a Windows check per commit, one or two CI rounds),
test author about 150,000 (two `xtask` test files from the tables), reviewer about 320,000 (each P/w row's Windows branch
against T-074's documents, each counterpart's sentence, the stand-in, the CI record), rework about 100,000. Calibration:
T-079 (the macOS half, high, 20 entries and 4 portable fixes) booked 846,099; T-074 897,911 (`metrics/tasks.jsonl`, summed
by task).

## Out of scope and implementer notes
- Product code (`crates/kiln-workspace/src/` outside `disk.rs`'s test module), documents, the workflow, `xtask/src`.
- `kiln-commands`, `kiln-cli`, `kiln` tests (T-098); failures in targets that already compile outside this crate (T-099).
- No test is renamed, removed or moved; no assertion is loosened on Linux or macOS; `#[ignore]`, skips and early returns
  are not used (`no-skip`).
- Every process a test spawns keeps its timeout (D-028 (8)); W22's `rustc` too.

## Questions and rulings
Questions of the planner's first run (Q-1 to Q-7) and the director's rulings (D-269), as the sections above now state them:
- **Q-1** → ruling 1: T-075 absorbed by T-092 and T-098 (the director marks it); T-076 narrowed to its three `/dev/full`
  counterparts; T-076 and T-077 run after T-098. (Cut.)
- **Q-2** → ruling 2: the snapshot of `acceptance_t002_r2.rs` records identity (inode) on Unix only; on Windows it records
  kind, bytes, link target and the read-only attribute; the reviewer checks this is the only difference between the twins.
  `same-file` handles are not used. (Rows W2-W9.)
- **Q-3** → ruling 3 by the probe: P-A met, so W42 and W43 are written, entries `windows`/`none`; the four T-079 entries
  keep `linux`/`none` with corrected reasons. (Interface (2).)
- **Q-4** → ruling 4 by the probe: P-D4 met, so W22 is P with `rustc`-built executable fakes. (Row W22.)
- **Q-5** → ruling 5: `<name>_unix` on the Unix part, the original name on the portable part. (Rule 2.)
- **Q-6** → ruling 6: T-098 stays whole; T-072's step is not widened. (Pending CI criterion.)
- **Q-7** → ruling 7: `depends_on` T-073, T-074, T-072; a fresh spec check by another planner after T-074 is on main.
Open: none of the planner's. The spec check should re-read Interface (1) against T-074's tree as merged (any review-round
change to `disk.rs`'s test module or the registry), and confirm W35's split (found at finalization: its `RenameParent`
case renames the held root's parent, which Windows refuses).

## Log
### director
- 2026-11-18 filed as a draft (D-233).
<!-- end director log -->
### planner
- 2026-11-18 · planner (specification run; booked to iter-2) · draft · did: re-specified the draft in full under v1.5 against
  main `bcca328` with T-074's and T-072's trees as base; inventory by a Windows clippy with the stand-in (26 targets, 199
  errors; kiln-workspace 9 targets, 84); sorting rule and 41 rows; cut into T-092, T-098, T-099 (drafts written); prototype
  of the forms run with main's tool; probe commit `b57a74e` prepared (CI-REQUEST below). Tier high: gate paths
  (`xtask/platform-tests.toml`, `xtask/tests/`) and the tests of the confinement boundary and the save change what they
  assert on Windows. Pinned (a), (h), (i) n/a: no product code, no item, no sentence. Unsure: every row marked unchecked
  (W18, W20, W21, W35 runtime; W22, W40, W41 by the probe); Q-2 and Q-3. Not merged when written: T-072, T-074 (spec
  check needed). Draft corrections: depends_on (Q-7); the draft's list of targets the gate names is the part cargo
  printed before stopping, not the set (26, above); its figures predate T-073; its title is the whole cut, not this task.
  CI-REQUEST: b57a74ef2a9773a02a284564d880bc853cec6ae7 /home/user/kiln-plan-T092 (branch `probe/T-092`; expectation in
  "Probe") · next: director (probe round, Q-1 to Q-7)
- 2026-11-18 · planner (finalization, same run resumed; booked to iter-2) · draft · did: read the probe record in full and
  compared it with the expectation (all met except P-D's script fakes, which ran the real tools rather than failing to
  start); applied rulings 1-7 (D-269); rows W22, W40, W41 settled, W42-W43 added, W35 split (found by reading the test:
  it renames the held root's parent), counts now 43 rows, 18 entries; re-ran main's tool and the registry tests on main
  `6a38a2b` + `task/T-074` + the prototype (all PASS, as listed); answered the cut question (no cut: two sequential high
  tasks would cost about 1.3-1.4M against about 0.95M); updated T-098 and T-099 with the probe's facts; deleted the clone's
  `target/`. Unchecked: whether each rewritten P/w test passes on Windows as written (first CI round, AC-3) · next: director
  (spec check after T-074 merges)
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
