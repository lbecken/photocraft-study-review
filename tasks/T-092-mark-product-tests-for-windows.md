---
id: T-092
title: "Make every test target of kiln-workspace except the two listing targets (T-100, D-273) compile, lint clean and pass on Windows: each test either asserts every platform's specified behaviour or is registered as a Unix test with its counterpart or reason; the two pending disk.rs entries get their Windows counterparts"
tier: high              # METHODOLOGY v1.5 §4: gate paths (xtask/platform-tests.toml, xtask/tests/); it changes what confinement and persistence tests assert (the save and the walk of disk.rs); the approval row must say `gate paths`. No product code, no dependency change, no document change.
state: queued
owner_role: planner
scorecard_items: []     # none claimed: the ws.*/io.* items are verified on Linux; per-platform confirmation is the evaluator's (ADR-0009 Decision 3)
depends_on: [T-073, T-074, T-072]   # ruling 7 (D-269). T-073 merged (8bf4c3a); T-072 merged (D-268; its general reported Windows step, this task's evidence, is on main); T-074 approved by review, not merged when this was finalized (shares disk.rs's test module, xtask/platform-tests.toml, acceptance_t002.rs): a fresh spec check by another planner follows once T-074 is on main (§5.7).
touches:                # one role per path (D-153). All implementer except the last two (test author). Cut D-273: acceptance_t019_workspace.rs and project.rs moved to T-100.
  - crates/kiln-workspace/tests/acceptance_t002.rs          # implementer: the one test disk_save_replaces_via_rename_without_leftovers (Interface row W1)
  - crates/kiln-workspace/tests/acceptance_t002_r2.rs       # implementer
  - crates/kiln-workspace/tests/acceptance_t008.rs          # implementer
  - crates/kiln-workspace/tests/acceptance_t015_tools.rs    # implementer
  - crates/kiln-workspace/tests/acceptance_t036_workspace.rs   # implementer
  - crates/kiln-workspace/tests/confinement.rs              # implementer
  - crates/kiln-workspace/tests/guard_t008.rs               # implementer
  - crates/kiln-workspace/tests/guard_t036_removal_workspace.rs   # implementer
  - crates/kiln-workspace/tests/guard_t036_workspace.rs     # implementer
  - crates/kiln-workspace/src/disk.rs                       # implementer: its #[cfg(test)] module only (two new tests, Interface row W40-W41)
  - xtask/platform-tests.toml                               # GATE PATH: implementer: exactly the entries of Interface (2)
  - xtask/tests/acceptance_t092.rs                          # GATE PATH: test author: AC-1 (fails on base)
  - xtask/tests/guard_t092.rs                               # GATE PATH: test author: AC-2 (passes on base)
budget_tokens: 1100000  # high (v1.5); estimate about 930,000 after the cut (D-273; see Estimate)
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

After this task, on Windows (cut D-273): **every `crates/kiln-workspace/tests/<t>.rs` target except
`acceptance_t019_workspace` and `project` compiles, is clippy-clean with `-D warnings`, and passes** (13 of 15; the two
listing targets are T-100's, `tasks/T-100-listing-tests-on-windows.md`, and stay as they are: see "The end after the
cut"); each test either asserts on every platform that platform's specified behaviour, or is a Unix
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

**This task's share** (after the cut, D-273: 7 of the 9; `acceptance_t019_workspace` and `project` are T-100's): the 9 non-compiling targets, `acceptance_t015_tools` (warnings and one failing test), one test of
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
   - A helper, constant or `use` that only a U part needs carries `#[cfg(unix)]` itself (one definition, not a twin),
     or it is dead code on Windows, an error under `-D warnings` (spec check M4).
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
documented Windows walk). U = Unix test, entry. S = split (original P or P/w by rule 1, new `<name>_unix` U; the
originals of W17, W23, W28, W36, W37, W38 are P/w, W35's is P: spec check M2).

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
| W23 **(T-100, D-273)** | `acceptance_t019_workspace.rs` :: `disk_list_dir_reports_entries_by_their_own_kind_and_skips_bad_names` | links; FIFO, socket | S: the original lists files, directories and links with the kinds `project.md` states per platform after T-074 | `_unix` → `none` (as W15) |
| W24 | `acceptance_t036_workspace.rs` :: `unreadable_root_is_refused_by_new` | mode 0311/0000, uid switch | U, file-level `#![cfg(unix)]`, one entry `<file>` (the file's only test; rule 2, third bullet; spec check M1) | `none`: modes and identities; on Windows unreadability is an access list, out of scope (ADR-0009 (b), `control-protocol.md:139` "Read and not measured") |
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
| W36-W38 **(T-100, D-273)** | `project.rs` :: `disk_list_dir_reports_own_kinds_sorted_and_skips_odd_names`, `disk_list_dir_errors_follow_t008_precedence`, `symlinks_and_fifos_are_listed_not_followed` | links; FIFOs | S each | `_unix` → `none` (as W15) |
| W39 **(T-100, D-273)** | `project.rs` :: `a_prefix_directory_outside_the_root_refuses_the_path` | links | P/w | — |
| W40 | `disk.rs` :: new `tests::leftover_at_temp_name_is_never_touched_on_every_platform` | read-only leftover at `.kiln-tmp-0` (`set_readonly`) | new P test; counterpart of `tests::leftover_at_temp_name_is_never_touched`. Windows behaviour VERIFIED at base (PROBE P-B1..B8: both saves `Ok`, `a` = `newer`, leftover bytes and `readonly` `true` unchanged, listing `[".kiln-tmp-0", "a"]`) | — |
| W41 | `disk.rs` :: new `tests::failed_save_after_writability_check_leaves_target_and_listing_unchanged_on_every_platform` | the three failures (case 1 by the public API, cases 2-3 injected); snapshot of listing, bytes and read-only attribute | new P test; counterpart of `tests::failed_save_after_writability_check_leaves_target_and_listing_unchanged`. Case 1 VERIFIED at base on Windows (PROBE P-C1 `Err(Io { path: RelPath("f"), kind: AlreadyExists, message: "all 100 temporary file names are taken" })`, P-C2 listing unchanged, P-C3 bytes unchanged, P-C4 `readonly` `false`); cases 2-3 are injected and platform-free | — |
| W42 **(T-100, D-273)** | `acceptance_t019_workspace.rs` :: new `disk_list_dir_reports_entries_by_their_own_kind_and_skips_bad_names_not_unicode` (`#[cfg(windows)]`) | a name with an unpaired surrogate (`OsStringExt::from_wide`) | new Windows test (ruling 3, decided by PROBE P-A); asserts `list_dir` omits it and counts it in `skipped` | `none`: Linux holds such names as bytes (its `_non_utf8` tests), macOS refuses them |
| W43 **(T-100, D-273)** | `acceptance_t019_workspace.rs` :: new `tree_counts_skipped_names_on_disk_not_unicode` (`#[cfg(windows)]`) | the same name in a subdirectory | new Windows test (ruling 3); asserts `project::tree`'s `skipped` | `none` (as W42) |

PROBE P-A (Windows, at base): `create "b\u{d800}.rs": Ok(())`; read_dir lists it; read back `Ok([98])`; `list_dir(None):
Ok(DirListing { entries: [DirEntry { name: "ok.rs", kind: File }], skipped: 1 })`; `project::tree: Ok(ProjectTree { entries:
[… "ok.rs" …], ignored: 0, skipped: 1, too_deep: 0, truncated: false })`. So W42 and W43 pass on base on Windows: they
are the implementer's tests of existing behaviour, not acceptance tests.

**Cut (D-273):** rows W23, W36, W37, W38, W39, W42, W43 (the files `acceptance_t019_workspace.rs`, `project.rs`) and the
`\`-name tests of spec check M3 move to T-100; they stay in the table above for reference only and are **not** this
task's. Every other test of the 9 files left is unchanged in body (their only Windows obstacle was a file-level import).
**Counts after the cut** (36 rows): P 24 (W1; W2-W9 8; W12-W14 3; W17, W28, W35 originals 3; W18, W20, W21; W22; W25,
W26, W29, W31, W34); U on existing tests 10 (W10, W11, W15, W16, W19, W24, W27, W30, W32, W33); new `_unix` tests 3 (W17,
W28, W35); new counterparts 2 (W40, W41). (Before the cut: 43 rows, P 29, `_unix` 7, Windows tests 2.)

### (2) The registry (`xtask/platform-tests.toml`)
After this task (cut D-273), the entries whose `file` is one of the 9 test files of `touches` are exactly 12 new ones,
`(file, item, platforms, counterpart)`, all `platforms = "unix"` — W10, W11, W15, W16, W17`_unix`, W19, (W24 as
`<file>`, spec check M1), W27, W28`_unix`, W30, (W32+W33 as `<file>`), W35`_unix` — counterparts as in the table. The
entries of W23`_unix`, W36-W38`_unix`, W42, W43 and the four T-079 non-UTF-8 entries' `reason`s are T-100's; this task
does not touch any entry whose `file` is `acceptance_t019_workspace.rs` or `project.rs`. And the two `disk.rs`
entries `tests::leftover_at_temp_name_is_never_touched` and
`tests::failed_save_after_writability_check_leaves_target_and_listing_unchanged` name W40 and W41
(`counterpart = "crates/kiln-workspace/src/disk.rs::tests::<W40|W41 name>"`, no `reason`, `task`, `expires`). Every `doc`/`quote`
is an existing sentence (`paths.md:112` "Directories, FIFOs, sockets and devices are refused"; `paths.md:13-15` for the unreadable root; the `decisions.md` and
`paths.md` sentences the `disk.rs` entries already quote); this task adds no document sentence. Other entries are not
changed. After this task no entry has `task = "T-092"`; pending entries: 8 (T-076's 3, T-077's 5).

## Pinned
- (a) Error variants, payloads, precedence: n/a: no product code changes; the tests assert the variants T-073/T-074
  specified (e.g. `VfsError::OutsideRoot { path }` for every reparse point on the Windows walk).
- (b) Files read or written: the 11 files of `touches` (after the cut) and the two `xtask/tests` files. Fixtures stay under
  `CARGO_TARGET_TMPDIR` as today.
- (c) Order: the implementer first commits the Windows check of its tree with the stand-in (AC-4), then asks for a CI round
  (`CI-REQUEST`). What the rows assume of Windows is VERIFIED by records (each row names it: PROBE, `T-072-round-3.md`,
  `T-074-probe-1.md`); what stays **unchecked** is only whether each rewritten P/w test passes there as written, which that
  first CI round measures (AC-3). A row whose Windows run disagrees with the table is a `QUESTION:`, never a silent cfg.
- (d) Platform assumptions: links on Windows are made with `symlink_dir` when the target, resolved from the link's
  directory, is an existing directory when the link is made, else `symlink_file`; junctions are not used by these tests
  (T-074's guards cover them). Paths are compared as `/`-joined relative strings, as the files do now. Ruling Q-B
  (D-273): relative link targets are built with `PathBuf::push` (as `guard_t074.rs:195-207`) and a directory link is
  removed with `fs::remove_dir` (as `guard_t074.rs:240-251`, from `T-074-probe-1.md` E4): T-074's measured practice.
  **Unchecked until CI round 1:** that a relative target with `..` after a directory link (`acceptance_t008.rs:262`,
  `confinement.rs:138,201`) reaches on Windows the file the fixture's precondition expects, and that `canonicalize`
  through `.`/`..` links (`acceptance_t008.rs:557-558`) gives the paths it expects. If the round shows otherwise, the
  implementer logs a `QUESTION:` naming the row and the measured value; the precondition is then chosen with
  `cfg!(windows)` like an expected value only after the director's ruling, never by a silent cfg or a dropped check.
- (e) Threat model: the tests in `touches` are the evidence for the confinement boundary and the save. The risk is a test
  that passes on Windows without checking: excluded mechanically by `no-skip` for cfg'd tests, statements and helpers,
  and by AC-2 for the P rows; the reviewer checks each P/w row's Windows branch asserts the documented answer (not merely
  "no panic") and each U row's counterpart checks the same sentence.
- (f) Tests pinning what this task changes: `xtask/tests/guard_t079.rs` (names: kept, see the rule; prototype run below);
  `xtask/tests/acceptance_t079.rs` (pins only entries of the two files moved to T-100: not touched by this task); `xtask/tests/acceptance_t074.rs`
  (its nine entries and "no `task = "T-074"`": untouched). None needs to change, so none is in `touches`.
- (g) Worked examples executed: the prototype forms below (main's `no-skip` on a file-level `#![cfg(unix)]` with and
  without its entry; a twin link helper compiled for Windows); the Windows behaviour W22, W40, W41 rest on: PROBE P-B to P-D,
  quoted in their rows.
- (h) Scorecard items: none claimed. The evaluator's call for a Windows confirmation is the CI record (ADR-0009 Decision 3).
- (i) Document sentences: none added or changed (n/a).

## Acceptance criteria
- **AC-1 (registry; test author, `xtask/tests/acceptance_t092.rs`).** Parsing `xtask/platform-tests.toml` with `toml` (any
  spelling of the array of tables), the set of `(file, item, platforms, counterpart)` of entries whose `file` is one of the
  9 test files of `touches` equals exactly Interface (2)'s 12; the two `disk.rs`
  entries have exactly the W40/W41 counterparts and no `reason`, `task`, `expires`; no entry has `task = "T-092"`. Fails on
  base (the 12 are absent; the two are `pending`). Must reject: an entry `confinement.rs::walk_corner_cases` with
  `platforms = "unix"` (a P row registered); W19 with `counterpart = "none"`; W41's entry naming W40's test.
- **AC-2 (guard; test author, `xtask/tests/guard_t092.rs`).** Each P row's test (Interface (1), the 24 names after the cut) exists as exactly one `#[test]` fn and has no platform cfg attribute on the fn, its inline modules, the file
  (`#![cfg]`) or the file's `mod` declaration (the reading of `guard_t079.rs` AC-2, without its in-body clause: P rows may
  hold paired platform statements, which `no-skip` checks). Passes on base (none of these has such a cfg today). Must
  reject `#[cfg(unix)]` on `guard_t008.rs::symlinks_inside_root_still_work`.
- **AC-3 (pending CI criterion; record `tasks/ci-runs/T-092-round-N.md`; restated after the cut, D-273).** On the head
  rebased on a main holding T-072 and T-074, in `gate-windows`, the step "kiln-workspace test targets (reported, not
  required)" (`windows-tests.txt`) prints exactly these 15 status lines and no other: `==== <t>: exit status 0` for each of
  the 13 targets `acceptance_t002`, `acceptance_t002_r2`, `acceptance_t008`, `acceptance_t015_tools`, `acceptance_t036_workspace`, `acceptance_t073`, `confinement`, `guard_t008`, `guard_t036_removal_workspace`, `guard_t036_workspace`, `guard_t073`, `guard_t074`, `tools_contract`, each with `test result: ok.` and exactly this many passed: 15, 8, 7, 3, 0, 1, 4, 1, 0, 2, 10, 15, 5
  (71 in all; the two 0s are the Unix-only files of W24 and W32/W33, `running 0 tests`, by design); and `==== <t>: exit
  status 101` for exactly `acceptance_t019_workspace` and `project`, each whose block holds `error: could not compile
  `kiln-workspace` (test "<t>")` (base: due to 5 and to 3 previous errors, the `std::os::unix` imports and
  `is_fifo`/`is_socket`; T-092 does not touch these files, so the same) and no `test result:` line. The step "Workspace lib
  and T-073 tests" prints a line beginning `WORKSPACE TESTS: exit status 0` and `test result: ok. 45 passed` for the library
  (43 + W40, W41) with `test disk::tests::<W40 name> ... ok` and `test disk::tests::<W41 name> ... ok`. Base (VERIFIED,
  `T-074-round-1.md:178-192`, `T-074-early-2.md:175-189`, PROBE's table lines 26-41 without `probe_t092`): 11 of 15 exit
  101, 4 exit 0. Must reject: any `exit status 101` among the 13; a third `exit status 101`; a listing target with `exit
  status 0` reported as this task's (it would mean T-100's files changed: a scope finding); a count other than the one
  given. `gate-windows`' clippy, scorecard and test steps still fail; every `could not compile` line in their kept text
  and in `test-output.txt` names a package other than `kiln-workspace` or one of the two listing targets.
  `gate-linux`, `window-linux` success; `gate-macos` success (or D-262's rule for a job the host cancels before any step,
  which this diff can bear on only through the test files it builds: the reviewer states it).
- **AC-4 (procedure; implementer first, reviewer again).** With the stand-in of "The inventory":
  `cargo clippy -p kiln-workspace --lib --tests --locked --keep-going --target x86_64-pc-windows-msvc -- -D warnings` exits
  101 with `could not compile` for exactly `acceptance_t019_workspace` and `project` (T-100's) and for no other target;
  `--lib --test <t>` for each of the 13 targets of AC-3 exits 0 (base: exit 101, the 9 targets and the 3 warnings above); the same for `aarch64-apple-darwin` exits 0 (stand-in as
  `reviews/T-073-r1.md:58`: the stub with `--target=arm64-apple-macos11`, `AR_aarch64_apple_darwin=llvm-ar`); on the host `cargo xtask
  ci` is green with main's tool (D-235).
- Guards that must keep passing (no change): `xtask/tests/guard_t079.rs` (both), `acceptance_t079.rs`, `acceptance_t074.rs`.
- **The end after the cut (D-273).** 13 of the 15 targets exit 0 as AC-3 lists; `acceptance_t019_workspace` and `project`
  still do not compile on Windows. Making them merely compile is not cheap without changing what they assert: their
  errors are the `FileTypeExt`/`UnixListener` imports and the FIFO/socket fixtures inside W23, W36-W38 (`acceptance_t019_
  workspace.rs:19-20,107,120,296`, `project.rs` imports), so compiling means putting those parts behind `#[cfg(unix)]`,
  i.e. doing T-100's splits and registering them; and once compiled, the `\` names (spec check M3) would make
  `tree_counts_skipped_names_on_disk` and `documented_differences_from_git` fail. So they stay as they are; T-100 owns them.

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
| **T-092** (this; cut D-273) | `kiln-workspace`: 7 of the 9 targets that do not compile, `acceptance_t015_tools`, one test of `acceptance_t002`, the 2 pending `disk.rs` entries | AC-3: 13 of 15 `exit status 0`, the two listing targets still 101 (base 4 of 15 at 0) | `crates/kiln-workspace/tests/` (9 files), `disk.rs` test module, registry, `xtask/tests/` (2) | high | 930,000 | after T-072, T-074 |
| **T-100** (`tasks/T-100-listing-tests-on-windows.md`, D-273) | `acceptance_t019_workspace.rs`, `project.rs`: rows W23, W36-W39, W42, W43, the two `\`-name tests (Q-A ruled B: a paired helper) | those 2 targets exit 0 in the general step (15 of 15 with T-092) | the 2 files, registry, `xtask/tests/` (2) | high | 550,000 | after T-092 (registry) and T-095 (names on Windows) |
| **T-098** (`tasks/T-098-kiln-cli-commands-and-app-tests-on-windows.md`) | `kiln-commands` (2), `kiln-cli` (11), `kiln` (4): every remaining target compiles under the same rule | `gate-windows` clippy step PASS; the test step builds; every test of its files passes on Windows or is registered absent | `apps/kiln-cli/tests/` (11), `apps/kiln/tests/` (4), `crates/kiln-commands/src/doc.rs` test module, `crates/kiln-commands/tests/acceptance_t037_commands.rs`, registry, `xtask/tests/` (2) | high | 1,000,000 | after T-092 (registry) |
| **T-099** (`tasks/T-099-gate-windows-green.md`) | the Windows failures of targets that already compile (probe, T-098's round) and the scorecard step | `gate-windows` conclusion success on the rebased head, then on main; **this task proposes counting `gate-windows`** (§8.1: it counts from its first green run on main) | `apps/kiln-cli/tests/` (4 files of the probe) and what the `xtask.exe` note needs | high (provisional) | 500,000-700,000 | after T-098 |

Each merges with the Windows gate strictly nearer to green (after the cut, D-273: T-092 26 → 19 targets that do not
compile and 2 → 0 known failing tests; T-100 19 → 17; then T-098 17 → 0). As first written: T-092 26 → 17 targets that do not compile and 2 → 0 known
failing tests; T-098 17 → 0 and clippy green; T-099 the remaining failures → `gate-windows` success.
Drafts (ruling 1, D-269): T-075 is absorbed by T-092 and T-098; T-076 is narrowed to ruling x6's three `/dev/full`
counterparts and runs after T-098 (shared `apps/kiln/tests/acceptance_t055.rs`; it also shares
`apps/kiln-cli/tests/acceptance_t004_r2.rs` with T-099, so the director orders those two); T-077 runs after T-098 (shared
`acceptance_t065.rs`, `window_t065.rs`); T-095 (after T-074) shares only the registry, and only if it adds an entry.

**Should T-092 itself be cut?** (Superseded: D-273 cut the listings off as T-100; the paragraph is kept as the first
answer.) Not recommended. A clean line exists: (a) the save — `acceptance_t002.rs`,
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
**After the cut (D-273): about 930,000 of 1,100,000**, by role against the booked costs of the three nearest high tasks
(`metrics/tasks.jsonl`, summed by task and role; implementer / test author / reviewer): T-073 293,262 / 215,523 / 352,192
(860,977); T-074 283,525 / 265,811 / 371,573 (920,909); T-079 274,703 / 211,253 / 360,143 (846,099).
- implementer about 320,000: 9 files, 36 rows (24 portable rewrites, 10 registered, 3 splits, 2 new `disk.rs` tests),
  12 entries, the stand-in per commit, one or two CI rounds; above the three (275,000-293,000) for the larger row count;
- test author about 200,000: two `xtask` files of the shape of T-079's (211,253);
- reviewer about 350,000: each P/w Windows branch, each counterpart's sentence, the stand-in, the CI record; at the low end
  of the three (352,000-372,000) since the listings are gone;
- rework about 60,000 (a second CI round if Pinned (d)'s unchecked forms differ on Windows).
Before the cut the spec check put the whole at about 1,080,000 (D-273: cut).

## Out of scope and implementer notes
- Product code (`crates/kiln-workspace/src/` outside `disk.rs`'s test module), documents, the workflow, `xtask/src`.
- `kiln-commands`, `kiln-cli`, `kiln` tests (T-098); failures in targets that already compile outside this crate (T-099).
- No test is renamed, removed or moved; no assertion is loosened on Linux or macOS; `#[ignore]`, skips and early returns
  are not used (`no-skip`).
- Every process a test spawns keeps its timeout (D-028 (8)); W22's `rustc` too.
- W22 (spec check, point 2): `rustc` is run from the test's `PATH`; if it cannot be started the test fails naming why
  (no fallback to `.cmd`/`.bat` fakes); the child's `PATH` is joined with `std::env::join_paths` (`;` on Windows); the
  `.exe` fakes append to the marker, as the shell fakes do.
- Ruling Q-B (D-273; Pinned (d)): on Windows a directory link is removed with
  `fs::remove_dir`, not `remove_file` (`guard_t074.rs:240-251`, from `T-074-probe-1.md` E4), and relative link targets
  are built with `PathBuf::push` as `guard_t074.rs:195-207` does.

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

## Spec check (fresh planner)
Run on 2026-11-18 against main `32b597d` (= `4eb349d` plus a ledger commit: `git diff --stat 4eb349d 32b597d` shows
`STATE.md`, `handoffs/2026-11-18.md` only), in the clone `/home/user/kiln-check-T092` (the planner's clone was only
read: its branches fetched as remote `plan`). Labels as above (VERIFIED: a command run here, or a record quoted).
Stand-in for Windows as `reviews/T-073-r1.md`: `CC_x86_64_pc_windows_msvc=<stub writing an empty object for every
-o/-Fo>`, `AR_x86_64_pc_windows_msvc=llvm-lib` (kept in `/home/user/kiln-check-T092-tools/`); nothing is linked.

### 1. The table against main
**Base.** `git diff --stat main plan/plan-base -- crates apps xtask .github docs/protocol Cargo.lock` is **empty**
(VERIFIED): the tree the table was written on is main's code tree; T-074's merge changed nothing under those paths
relative to it (its `disk.rs` test module, `acceptance_t002.rs` and registry were already in `plan-base`). Registry on
main: 35 entries, 10 pending (2 `T-092`, 3 `T-076`, 5 `T-077`) (VERIFIED: `tomllib` over the file), as D-271 says.

**Names.** Every existing test of rows W1-W39 exists under that name in that file on main, once (VERIFIED: `grep -A3
'#\[test\]'` per file; 71 tests in the 11 files: 15, 10, 10, 3, 7, 1, 6, 1, 2, 2, 14). W40-W43 do not exist (new).
Every named counterpart exists on main with no platform cfg, so it runs on Windows: `guard_t073.rs::acw2_save_of_read_only_
target_is_refused_and_changes_nothing` (:658), `guard_t073.rs::acw2_root_parent_replaced_is_outside_root` (:486; on
Windows it asserts the refused rename and a working session, :435-464), `disk.rs::tests::root_removed_on_every_platform`
(:1970). The `_unix` counterparts (W17, W28) name originals that this task keeps portable (implementer). No row
touches `disk.rs::tests::failed_rename_removes_own_temp_and_keeps_target` (`guard_t079` AC-2): VERIFIED.

**Mismatches** (M1-M6; M1, M2, M4, M5 corrected in the text above by this check, M3 is question Q-A below):
- **M1 (W24, corrected).** `acceptance_t036_workspace.rs` holds one test, and it is U with one sentence and counterpart
  `none`: the sorting rule's third bullet gives the file-level form (`#![cfg(unix)]`, `item = "<file>"`), the table gave
  the test-level form. Measured: `#[cfg(unix)]` on the test (and on its two `use std::os::unix` lines) leaves **15
  errors** for Windows in the file's helpers (`uid`, `gid`, `mode`, `from_mode`, `lchown`, `Command::uid`); with the
  file-level form the target compiles for Windows and main's `no-skip` passes with an entry `<file>` (sketch, point 3).
  The two builders would have written different `item`s, which AC-1 compares exactly. Row W24 and Interface (2) now say
  `<file>`.
- **M2 (split originals, corrected).** The legend said "S = split (original P)". Each original keeps a link that stays
  inside or a loop, whose Windows answer is `outside_root` (T-074: every reparse point on the walk), so each is **P/w**:
  W17 (`ghost2 -> real/none.txt` is `not found` on Unix, `acceptance_t008.rs:593-600`; `up`, `self`), W23 and W36 (the
  listing through `l`/`to_dir`, "A symlink on the way that stays inside is followed"), W28 (`c.txt -> real/t.txt` is
  saved through on Unix, `confinement.rs:345-352`), W37 (`loop1` is `Io` on Unix, `project.rs:231`), W38 (`disk_tree(…,
  Some("self"))` lists through `self`, `project.rs:297-302`). W35's original has no link (P). AC-2's 29 names are
  unchanged (P/w carries no cfg attribute).
- **M3 (two tests missing; names holding `\`).** Four tests make a file whose name contains `\`:
  `acceptance_t019_workspace.rs:221` (W23's original, `root/b\\s`), `:514` (`tree_counts_skipped_names_on_disk`, **not
  in the table**), `project.rs:121` (W36's original, `back\\slash`), `:425` (`documented_differences_from_git`, **not in
  the table**). On Windows `\` separates components, so `fs::write("…/sub/back\\slash")` writes into a directory
  `sub\back` that does not exist and the `unwrap()` panics (INFERRED from Win32 path rules; no record: these targets
  have never compiled on Windows). So "Every other test of these files is unchanged in body" is wrong for two tests,
  AC-3 ("15 of 15 exit 0") cannot be met for `acceptance_t019_workspace` and `project` as specified, and the sorting
  rule has no line for a name holding `\` (rule 2 lists non-UTF-8 byte names only). A design choice: Q-A.
- **M4 (helpers of U parts, corrected).** Rule 1 says twins for helpers that differ; it did not say what happens to a
  helper or import used only by a U part (`mkfifo`, `socket_at`, `set_mode`, the `uid` helpers): on Windows it is dead
  code, an error under `-D warnings`. One sentence added to rule 2: such items carry `#[cfg(unix)]` themselves (one
  definition, not a twin: `no-skip`'s `inert-helper` rule reads only functions defined more than once, `xtask/src/
  platforms.rs:360-375`), as `acceptance_t019_workspace.rs:14-18` already does for its Linux-only helpers.
- **M5 (W41 wording, corrected).** "the three injected failures": case 1 (all names taken) goes through the public API,
  cases 2-3 are injected (`disk.rs:1023-1061`).
- **M6 (registry text, note).** AC-1 pins `(file, item, platforms, counterpart)`; every `item` and `counterpart` the
  builders must agree on is now given exactly (W24 by M1). `doc`/`quote`/`reason` per entry are not given row by row
  (only the three shared sentences): not a blocker, since AC-1 does not compare them and `no-skip` refuses a quote not in
  its document; the reviewer checks each counterpart's sentence (Pinned (e)).

### 2. Sentences that rest on the probe or a CI record
Checked against `tasks/ci-runs/T-092-probe-1.md` (cited `probe:<line>`), `T-074-round-1.md`, `T-074-early-2.md`,
`T-072-round-3.md`, `T-074-probe-1.md`.
- **Hold as cited (VERIFIED):** Goal's two failures and their panic lines (probe:129-138); W18/W19's MEASURED lines
  (`T-072-round-3.md:109-110`; probe:264-265, 364); W20/W21 (probe:358-363: `remove_dir`/`remove_dir_all` of the held
  root `Ok(())`, then every call `OutsideRoot`, with a new directory or a junction at the name); W22's P-D1 to P-D4
  (probe:460-466); W40's P-B1-B8 (probe:448-455; `[110, 101, 119, 101, 114]` is `newer`); W41's P-C1-C4 (probe:456-459);
  P-A (probe:442-447); P-E (probe:467-470); 96 targets, 64/26/6, 207 `MEASURED:` (probe:17-20); links makeable
  (`T-074-probe-1.md:45-50`).
- **Wrong figure (corrected):** AC-3's "Base … 10 of 15 exit 101" and the Cut table's "base 5 of 15": on the tree now on
  main 11 of 15 targets exit 101 and 4 exit 0 (`acceptance_t073`, `guard_t073`, `guard_t074`, `tools_contract`;
  `T-074-round-1.md:178-192`, the same in `T-074-early-2.md:175-189` and probe:26-41 without `probe_t092`).
- **Wider than its record (labelled here):** W20's fixture puts a *directory symlink* at the removed root's name
  (`acceptance_t008.rs:756-758`); the probe measured a new directory and a junction there, not a directory symlink:
  INFERRED the same answer (every reparse point is `OutsideRoot`). W43's name sits in a subdirectory; P-A6 measured it in
  the root: INFERRED the same (`list_dir` applies one name check at every level). P-D measured `cargo.exe` only; the
  `rustfmt.exe` fake is INFERRED to behave alike. W22's "Windows' lookup is not limited to the child's `PATH`": the
  record shows the real tools ran (P-D1, P-D2); why (std appends `.exe` only, then searches other places) is INFERRED.
- **`no_tools_never_runs` (W22), what the row leaves to chance** (each now an implementer note, below, "spec check"):
  (a) `rustc` is looked up on the *test's* `PATH`: P-D3 shows it found under `cargo test` in a job that ran `rustup
  toolchain install` (`platforms.yml:193`, the same install line as the step jobs); if it is not found the spawn fails,
  and the test must then fail naming why (§8.1), never fall back to script fakes (P-D1/P-D2: they would run the real
  tools and the marker check would fail for the wrong reason); (b) build time 212 ms per fake (P-D3), under the
  file's 60 s `TIMEOUT` through `wait_with_timeout` (D-028 (8)); MSVC `rustc` also writes a `.pdb` beside the `.exe`,
  harmless; (c) the fakes go in `fresh_dir("fake-path")/bin` under `CARGO_TARGET_TMPDIR`, as now; (d) the parent's
  `PATH` is built with `":"` today (`acceptance_t015_tools.rs:151`): Windows separates with `;`, so the Windows branch
  needs `std::env::join_paths`; (e) the probe's fake *overwrote* its marker (`fs::write`), the test counts two lines
  (`:165`): the fake must append.
- **The two non-Unicode tests (W42, W43):** P-A5/P-A6 hold; they pass on base on Windows (implementer's tests of
  existing behaviour), so no acceptance criterion rests on them, as the row says. Their entries (`windows`, `none`) passed
  main's `no-skip` in the sketch of point 3 (W42).
- **W35's split:** confirmed by reading `guard_t036_workspace.rs:209-275`: `RenameParent` renames `p` while `p/root` is
  held, which Windows refuses (`T-072-round-3.md:109`); `RenameRoot` with its move back and save ran on Windows inside
  `acw2_root_replaced_at_same_path_is_outside_root` (guard_t073 10 of 10, `T-074-round-1.md:167`); `Remove` probe:361-362.
- **The two `disk.rs` counterparts (W40, W41):** names, behaviour and the entries' new text fit main's registry (the
  pending text is at `xtask/platform-tests.toml`, entries `tests::leftover_at_temp_name_is_never_touched` and
  `tests::failed_save_after_writability_check_leaves_target_and_listing_unchanged`); W40 written as a sketch passed on
  Linux and type-checks for Windows (point 3). W41's case 1 overlaps the existing portable
  `all_temp_names_taken_fails_without_changes_then_recovers` (`disk.rs:830`); its own point is the read-only attribute in
  the snapshot, which on Windows stays `false` (P-C4): weak but the only Windows form of "mode unchanged".

### 3. Type-check for Windows (stand-in), on main and on sketches
- **Main** (`main` = `32b597d`): `cargo clippy -p kiln-workspace --all-targets --locked --keep-going --target
  x86_64-pc-windows-msvc --message-format=short -- -D warnings` → exit 101; `could not compile` for exactly **10
  targets**: the 9 of the inventory (`acceptance_t002_r2` 11, `acceptance_t008` 20, `acceptance_t019_workspace` 5,
  `acceptance_t036_workspace` 16, `confinement` 24, `guard_t008` 1, `guard_t036_removal_workspace` 3,
  `guard_t036_workspace` 1, `project` 3: 84 errors) plus `acceptance_t015_tools` (3, the `dead_code` warnings made errors).
  The library, its test module and the other 5 targets compile. VERIFIED, as "The inventory" says.
- **The prototype** (`plan/proto/T-092` = `fb13257`, checked out in this clone): `cargo clippy -p kiln-workspace --test
  guard_t036_workspace --test guard_t036_removal_workspace --locked --target x86_64-pc-windows-msvc -- -D warnings` →
  `Finished`, no warning; main's `cargo xtask no-skip` → `PASS no-skip: … 206 files scanned, 0 allowlisted skip(s), 36
  platform test(s) registered (10 pending)`. VERIFIED. It shows two forms only (a file-level `#![cfg(unix)]` with its entry;
  a twin link helper); it does not show W35's split, which it predates, nor that the twin's runtime works (see Q-B: the
  test still removes the directory link with `fs::remove_file`, `guard_t036_workspace.rs:164,197`).
- **My sketch of three rows** (branch `chk-sketch` in this clone, never pushed; registry edited as Interface (2) says):
  (i) W24 file-level `#![cfg(unix)]` + entry `<file>` (`counterpart = "none"`); (ii) W40 written in `disk.rs`'s test
  module with `Permissions::set_readonly`/`readonly`, and the pending entry turned into `counterpart =
  "crates/kiln-workspace/src/disk.rs::tests::leftover_at_temp_name_is_never_touched_on_every_platform"` with `reason`,
  `task`, `expires` removed; (iii) W42 as a `#[cfg(windows)]` test (`OsStringExt::from_wide` with `0xD800`) + entry
  `platforms = "windows"`, `counterpart = "none"`. Results: Windows clippy of `--lib --tests -D warnings`: the target
  `acceptance_t036_workspace` and the library's test module now compile; `acceptance_t019_workspace` still has exactly
  its 5 base errors (lines 19, 20, 107, 120, 296), none in the sketch (from line 559); main's `no-skip` → `PASS … 37
  platform test(s) registered (9 pending)`; host `cargo test -p kiln-workspace --lib leftover_at_temp_name` → 2 passed;
  `hygiene`, `layers`, `ui-commands`, `schema` PASS; `cargo test -p xtask --test acceptance_t079 --test acceptance_t074
  --test guard_t079 --test guard_t071` → 2, 2, 6, 6 passed. The W24 test-level form for contrast: 15 errors (M1).

### 4. Acceptance criteria
- **AC-1** testable (TOML, `std::fs`), an equality over a stated set, fails on base on every platform (VERIFIED by the
  registry dump of point 1: none of the 18 exists; both `disk.rs` entries are `pending`/`T-092`). Two outside-the-list
  variants are named. One more it must reject, for the `disk.rs` half: W41's entry naming W40's test (both are
  `disk.rs::tests::…_on_every_platform`). The item of W24 is `<file>` (M1). If Q-A is ruled A, the set grows by Q-A's
  entries; under B it is unchanged.
- **AC-2** testable (`syn`, as `guard_t079` AC-2), passes on base (none of the 29 has a platform cfg attribute on the fn,
  its modules, the file or its `mod` line: read for each file at point 1; the only in-body cfgs today are statements in
  W1, which AC-2 does not cover). It is the test author's guard of the implementer's work, not of its own (D-189: fine).
  It does not cover W40/W41 (new); AC-3's Windows lines and the reviewer do.
- **AC-3** names the step and the line form exactly as the record has them (`==== <t>: exit status N`,
  `T-074-round-1.md:178-192`). Corrections: base is 11 of 15 at 101 (point 2); the `WORKSPACE TESTS` line carries a suffix
  on main (`WORKSPACE TESTS: exit status 0 (reported, not required)`, `T-074-round-1.md:67`): AC-3 now says "a line
  beginning". **Counts to expect** were missing; from the table (INFERRED: what each target holds on Windows after the
  task), the general step's `test result: ok.` per target: `acceptance_t002` 15, `acceptance_t002_r2` 8,
  `acceptance_t008` 7, `acceptance_t015_tools` 3, `acceptance_t019_workspace` 7 (5 + W42 + W43), `acceptance_t036_workspace`
  0 (file-level, M1), `acceptance_t073` 1, `confinement` 4, `guard_t008` 1, `guard_t036_removal_workspace` 0,
  `guard_t036_workspace` 2, `guard_t073` 10, `guard_t074` 15, `project` 12, `tools_contract` 5: **90**; and in
  `workspace-tests.txt` the library 45 (43 + W40, W41). A target with 0 tests exits 0 by design (its file is Unix-only),
  not vacuously. Under Q-A A: `acceptance_t019_workspace` 6.
- **"15 of 15 exit 0" for the two targets that compile today:** `acceptance_t002` exit 0 means all 15 pass on Windows:
  W1's Windows statements and T-074's `cfg!(windows)` branch of `save_through_symlink_updates_target` (base 14/1,
  probe:129-133). `acceptance_t015_tools` exit 0 means 3 of 3: W22's `rustc`-built fakes ran, left their marker, and the
  child copy under `NoTools` left none (base 2/1, probe:134-138). Neither can be shown before CI: the stand-in links
  nothing (AC-4 shows only that they compile). Also: AC-3 is unreachable for `acceptance_t019_workspace` and `project`
  until Q-A is ruled (M3).
- **AC-4** testable as a procedure; base fails (point 3: exit 101, 10 targets). The macOS command lacked its stand-in:
  as `reviews/T-073-r1.md:58`, `CC_aarch64_apple_darwin=<the same stub with --target=arm64-apple-macos11>`,
  `AR_aarch64_apple_darwin=llvm-ar` (added to AC-4). Run on main with it: `cargo clippy -p kiln-workspace --all-targets
  --locked --target aarch64-apple-darwin -- -D warnings` → `Finished` (exit 0), so AC-4's macOS half holds at base and
  must stay so. Rule 3's claim re-run: `rustc +1.97.0 --crate-type lib --emit=metadata --target x86_64-pc-windows-msvc`
  on a use of `volume_serial_number`, `file_index`, `number_of_links`, `file_attributes` → 3 × `error[E0658]: use of
  unstable library feature windows_by_handle` (VERIFIED).

### 5. Roles and paths
One role per path holds (D-153): implementer — the 11 test files, `disk.rs`'s test module, the registry; test author
— `xtask/tests/acceptance_t092.rs` (AC-1) and `guard_t092.rs` (AC-2). Every existing test file is the implementer's:
its rewrite *is* the deliverable, and no criterion can be written from the spec by another role that fails on base
through the rewritten tests' behaviour (they pass on Linux base and are compile errors on Windows base). The test
author's share is therefore structural: the registry equality (fails on base) and the portable-rows guard (passes on
base). This is T-079's arm (implementer: the product test files and the registry; test author: `acceptance_t079.rs`,
`guard_t079.rs`), which booked test author 211,253 (`metrics/tasks.jsonl`). The other arm (the test author rewrites the
tests, the implementer writes the registry and two guards) would leave the implementer a registry to write from a
table and make the test author's work a deliverable that by D-189 only guards may test: no gain. **Keep the arm.** What
neither role's tests show — that each P/w Windows branch asserts the documented answer and each counterpart checks the
same sentence — stays the reviewer's (Pinned (e)) and the CI round's (AC-3). For the join: the implementer *edits* six
earlier `acceptance_t*` files (`acceptance_t002`, `_t002_r2`, `_t008`, `_t015_tools`, `_t019_workspace`, `_t036_workspace`);
§5.6 forbids *adding* one; the director's `git diff --name-only` check at the join should expect these six as edits.

### 6. D-235 and D-249, by runs on main
- **Main's tool on a planned tree** (my sketch, point 3: three rows and three registry edits on main `32b597d`; `xtask/src`
  is main's): `cargo xtask no-skip` → `PASS no-skip: self-test ok (17 known findings); 206 files scanned, 0 allowlisted
  skip(s), 37 platform test(s) registered (9 pending), today = 2026-11-18`; `hygiene` → `PASS … session-paths: 716
  tracked file(s) clean`; `layers` → `PASS layers: 11 crates, 14 internal edges, layer table holds`; `ui-commands` →
  `PASS ui-commands: self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s)`; `schema` → `PASS schema: 30
  commands; audit ok; docs/commands.json current`; `scorecard --check` → `PASS scorecard: 64 items: 47 verified, 0
  implemented, 0 failing, 17 planned.`, exit 0. This task changes no gate rule, so D-235's "does not also use it" does
  not bite.
- **D-249 grep** (`git grep -l -F <file name> main -- '*.rs'` for each path of `touches`): as the spec's list, plus two
  hits it did not name, both harmless: `xtask/tests/acceptance_t005.rs:340,368` names `acceptance_t008.rs` only as a
  scratch fixture path (`crates/b/tests/…`); `crates/kiln-ui/src/panels/mod.rs:7` names a different `project.rs` (the UI
  panel). Readers of the registry on main: `xtask/src/{noskip,platforms,scorecard}.rs` (the tool), `guard_t071.rs`,
  `acceptance_t074.rs`, `acceptance_t079.rs` (the real file), `acceptance_t071.rs` (toy registries it writes). What
  each says about the candidate: `acceptance_t079` — the four T-079 entries keep `platforms`/`counterpart`, `reason` is
  not pinned (its header, :18): passes; the new `_unix` items do not end in `_non_utf8`, so none "belongs" to a T-079
  test by ruling w1 (`belongs_to`, :200-205): passes; `acceptance_t074` — the nine untouched: passes; `guard_t079` —
  every base name kept: passes (sketch run: 6 of 6); `scorecard` — W42/W43 are registered absent on Linux, no item lists
  them: passes (run above). `acceptance_t092`/`guard_t092` are named by nothing yet.
- **End-of-log markers (D-266/D-267):** this file has all five (`<!-- end director log -->`, planner, implementer, test
  author, reviewer); so do T-098 and T-099 (`grep -o '<!-- end [a-z ]* log -->'`).

### 7. Scope, estimate, order
- **Estimate.** By role on the calibration tasks (`metrics/tasks.jsonl`, summed): T-073 implementer 293,262, test author
  215,523, reviewer 352,192 (860,977); T-074 283,525 / 265,811 / 371,573 (920,909; the spec's 897,911 predates its last
  review addenda); T-079 274,703 / 211,253 / 360,143 (846,099). The spec's test author 150,000 is below all three, its
  reviewer 320,000 below all three, though this task has more rows (43, plus M3's two) and more Windows branches to read
  than any of them. Mine: implementer about 400,000, test author about 200,000, reviewer about 380,000, rework about
  100,000: **about 1,080,000** of 1,100,000. 950,000 is not believable as a central figure; the ceiling is likely to be
  reached if Q-B's fixture forms cost a second CI round.
- **Cut, if one is needed:** not the spec's save/walk line but the listings: `acceptance_t019_workspace.rs` and
  `project.rs` (W23, W36-W39, W42, W43, M3's two tests; 6 entries and T-079's four reasons) as a second task after
  T-095 (which writes the Windows clause of `skipped`, `project.md:26`), because Q-A and T-095's sentence bear on
  exactly those two files. Cost as the spec computed for any two-part cut (both edit the registry, so serial: about
  1.3-1.4 M together). Recommended only if Q-A is ruled A or the first CI round shows several rows disagreeing.
- **Order.** Shared paths (VERIFIED from the `touches` lists): T-093 (`docs/protocol/paths.md`, `project.md`,
  `apps/kiln-cli/tests/macos_t093.rs`) and T-095 (`path.rs`, `paths.md`, `project.md`, `control-protocol.md`,
  `guard_t095.rs`) share **no path** with T-092, so §5.2 lets T-092 run beside either; T-098 shares
  `xtask/platform-tests.toml` and its `depends_on` is T-092: after. Semantic contacts: T-092's entries quote
  `paths.md:112`, `project.md:80`, `paths.md:13-14` and keep T-079's `project.md` "not valid UTF-8 or contain", in the
  documents T-093 and T-095 edit; a reworded quote fails main's `no-skip` in the later task's queue run (mechanical, one
  queue attempt). T-095 appends to `project.md:26` (its Interface, "appended"), keeping the quote. So: T-092 now, beside
  T-093/T-095; whichever of them merges after T-092 re-checks the registry quotes in its documents at its spec check;
  T-098 after T-092; T-099 after T-098.

### Questions for the director
- **Q-A (M3: names holding `\`).** Two tests outside the table and two originals make such a name, which Windows cannot
  hold. (A) Add the facility to rule 2: `tree_counts_skipped_names_on_disk` becomes U (counterpart W43), and
  `documented_differences_from_git` is split, `_unix` holding `odd/back\slash` (counterpart W43); W23's and W36's
  `\` lines move into their `_unix` halves; 2 more entries (20 new), 45 rows. (B) Make the four portable with a twin
  fixture helper that makes a name the listing must skip: `back\slash` on Unix, the unpaired-surrogate name of P-A on
  Windows (P-A5/P-A6 show it skipped and counted); no new entry, AC-1 unchanged, and the Windows run then asserts
  `skipped` in four existing tests. **Recommendation: B**: it keeps more Windows checking, adds no entry, and the helper
  is a twin with a real body on each platform (`no-skip`'s `inert-helper` rule holds). Cost: about 20,000-30,000
  (implementer) and a row each for the reviewer; A costs about the same plus two entries and AC-1's set. Either way
  W42/W43 (ruling 3) stay as ruled.
- **Q-B (fixture forms on Windows that no record shows).** The P rows' fixtures use `/`-separated relative link targets
  (`acceptance_t008.rs:257-264`, `confinement.rs:133-202`), `..` after a directory link inside a target with a
  precondition that the link reaches its file (`acceptance_t008.rs:262,265-266`; `confinement.rs:138,201`), `fs::remove_file`
  of a directory link (`guard_t036_workspace.rs:164,197`; the prototype keeps it), and `canonicalize` through `.`/`..`
  links (`acceptance_t008.rs:557-558`). T-074's guard avoided the first and third (targets by `PathBuf::push`,
  `guard_t074.rs:195-207`; directory links removed with `remove_dir`, `:240-251`, from `T-074-probe-1.md` E4); the second
  and fourth are unmeasured. (A) Dispatch as is; the first CI round measures; each disagreement is a `QUESTION:` (likely
  several rows, a second CI round, about 100,000-200,000). (B) A one-job probe of exactly these four forms before
  dispatch (about 40,000-60,000 and one CI run of about 10 minutes). (C) Pin now T-074's measured practice (the implementer
  note added under "Out of scope", marked as this question's) and let a fixture precondition that differs on Windows be
  chosen with `cfg!(windows)` like an expected value, leaving only `..`-after-a-link and `canonicalize` to the first
  round. **Recommendation: C**, with B if the director wants the first CI round to be the last. If the ruling is A, the
  implementer note goes.
- **Q-C (estimate).** Record about 1,080,000 (point 7) instead of 950,000, keep the task whole, and name the listings cut
  in advance as the fallback. Cost of the recommendation: none now; it moves the `Extend` decision earlier.

### After the rulings (D-273), the same day
Rulings applied: the cut (the two listing files to T-100), Q-A B in T-100, Q-B in Pinned (d) ("unchecked until CI round
1", with what follows a disagreement), Q-C by the cut and the new estimate. Re-run for what the cut changes, in the clone
(branch `chk-cut` from main `32b597d`, never pushed): a registry mock-up holding only T-092-side entries (W24 `<file>`,
W32/W33 `<file>`, W40's counterpart; no entry of the moved files changed) with those three forms in the tree → main's
`cargo xtask no-skip` → `PASS no-skip: self-test ok (17 known findings); 206 files scanned, 0 allowlisted skip(s), 37
platform test(s) registered (9 pending), today = 2026-11-18`; `cargo test -p xtask --test acceptance_t079 --test
acceptance_t074 --test guard_t079 --test guard_t071` → 2, 2, 6, 6 passed. D-249 greps for the reduced `touches` (`git
grep -l -F <name> main -- '*.rs'`): as point 6 minus `acceptance_t079.rs` (it names only the two moved files), so no main
test pins an entry this task now changes beyond `acceptance_t074` (nine untouched) and `guard_t079` (names kept).
Expected counts: AC-3 (71 in the general step's 13 targets, library 45; the two listing targets 101 at compile).
**Roles, confirmed for the reduced `touches`:** implementer — the 9 existing test files and `disk.rs`'s test module (the
rewritten tests are the deliverable) and the registry; test author — `xtask/tests/acceptance_t092.rs` (AC-1: the 12
entries and the two `disk.rs` counterparts; fails on base, as T-074's `acceptance_t074.rs`) and `xtask/tests/guard_t092.rs`.
One difference from the ruling's wording: `guard_t092.rs` as specified (AC-2) is a *source* guard (the 24 portable tests
exist once and carry no platform cfg attribute; `syn`, as `guard_t079` AC-2), not a runtime guard. A runtime guard of the
rewritten tests' behaviour would re-run what the implementer's own tests assert; their Windows behaviour is AC-3's (CI)
and the reviewer's. Kept as is; the director may ask for a runtime guard, which would need its own criterion.
**Verdict after the rulings: ready to dispatch.** Nothing open; the unchecked Windows fixture forms are stated in Pinned (d)
with their handling, and AC-3 names every line and count to expect.

### Verdict
**Ready after rulings** (Q-A, Q-B; Q-C is bookkeeping). Not "needs rework": the table's names, counterparts and
registry arithmetic hold on main (point 1), the probe sentences hold or are labelled (point 2), main's tool and tests
pass on a planned tree (point 6). Corrected in the text by this check: W24's form (M1), the split legend (M2), rule 2's
sentence on helpers of U parts (M4), W41's wording (M5), AC-3's base figure and the Cut table's (point 2), AC-3's
`WORKSPACE TESTS` line and expected counts (point 4), AC-4's macOS stand-in, the W22 and directory-link implementer
notes (point 2; the latter pending Q-B).

### Rulings on the test author's questions (director, D-275; sent to both builders)
- **a1.** The guard "each of the 33 other base entries is unchanged" is removed before the join: it is a
  left-alone pin (D-043), and T-076, T-077 and T-098 would each have to edit it to change their own entries (the
  D-249 kind of block). That no entry of another task changed is the reviewer's diff check, as ruling t1 of T-072.
- **a2.** The two source-side tests (`ac1_unix_only_tests_of_the_nine_files_are_exactly_the_listed_ones`,
  `ac1_w40_and_w41_exist_once_in_disk_tests_without_platform_cfg`) count under AC-1.
- **a3 (implementer's question, D-276).** W14 (`link_swapped_in_after_open_is_refused_at_save`) is P/w: its part (c)
  saves through a link that stays inside the root, which is `outside_root` on Windows since T-074; the expected
  answer of that part is chosen with `cfg!(windows)`, with no cfg attribute on the test. Accepted.

### Rulings on review round 1 and CI round 1 (director, D-277; sent to both builders)
- **b1 (CI round 1, the case Pinned (d) left unchecked).** In run 44383948281 on Windows the fixture precondition
  `assert_reaches` of `acceptance_t008.rs` (reading `root/<link>` through the kernel reaches the outside content)
  answered `NotFound` in W12 and W13 (`tasks/ci-runs/T-092-round-1.md`): at least one link form of the fixture is
  not an escape on Windows. The precondition becomes per fixture and per platform: on Unix it stays strict; on
  Windows each fixture link is measured first and a `MEASURED:` line says for each whether the kernel reaches the
  outside content; a fixture that reaches is asserted as before; one that does not reach is still asserted to be
  refused by Kiln (`outside_root`, since every reparse point on the walk is refused) with nothing outside changed;
  and at least one fixture of each test must reach on Windows, or the test fails naming the forms tried (a test of
  escapes with no real escape shows nothing). If no form reaches, that is a `QUESTION:` before any further change.
- **b2 (review finding 1).** `guard_t008::symlinks_inside_root_still_work`: the check that the links are kept must
  not depend on the verbatim prefix and takes no platform cfg: compare with `read_link` taken right after the
  fixture is made, as the reviewer proposes. The same pattern is looked for in the other rewritten files.

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
- 2026-11-18 · planner (fresh spec check; booked to iter-2) · draft · spec check: read the spec against main `32b597d`
  (code tree equal to the spec's `plan-base`); ran Windows and macOS clippy stand-ins on main, the prototype and a
  three-row sketch, main's `no-skip`, `hygiene`, `layers`, `ui-commands`, `schema`, `scorecard --check` and the
  registry tests on the sketch (all PASS). Found: W24's form (M1), the split originals are P/w (M2), two tests outside
  the table and two originals make names holding `\` that Windows cannot hold (M3, Q-A), dead helpers of U parts (M4),
  wrong base figures in AC-3 and the Cut table, missing expected counts and macOS stand-in, fixture forms no record
  shows (Q-B), an optimistic estimate (Q-C, about 1,080,000). Corrections made in the text; design points as Q-A to
  Q-C. Tier high confirmed (gate paths; confinement and persistence tests). Verdict: ready after rulings. Clone
  `target/` deleted · next: director (rulings Q-A to Q-C)
- 2026-11-18 · planner (spec check resumed for rulings D-273; booked to iter-2) · draft · spec check: applied the cut
  (rows W23, W36-W39, W42, W43 and the `\`-name tests to T-100, draft written), Q-B into Pinned (d), the end restated
  (13 of 15 targets exit 0, counts in AC-3; the two listing targets stay non-compiling), estimate about 930,000 by role;
  re-ran `no-skip` on a mock-up and the D-249 greps; T-098 and T-099 order notes. Verdict: ready to dispatch · next: director
<!-- end planner log -->
### implementer
- 2026-11-18 · implementer (R-391) · in_progress · criteria in my words: (1) on Windows the 13 targets of AC-3
  compile, are clippy-clean under `-D warnings` and pass with the counts AC-3 gives (lib 45 with W40, W41); the two
  listing targets stay as they are; (2) each test of the 9 files is P (no platform cfg on fn, module, file; Windows
  expectation by `cfg!(windows)` where a document states it), or U behind `#[cfg(unix)]` with exactly the 12 registry
  entries of Interface (2), or split keeping the name on the portable part and `_unix` on the Unix part; (3) the two
  pending `disk.rs` entries name W40, W41, which are new portable tests; (4) no Linux name lost or assertion weakened;
  main's tool and main's registry tests pass; the gate is green. Log as I go, file by file in table order.
  - W1 (`acceptance_t002.rs`): the `#[cfg(windows)] panic!` removed; a hard link `hold/first` (outside `d`, made before
    the listings are taken) keeps the old bytes after the first save, on every platform; before the unedited second save
    a hard link `hold/second` is taken and written through afterwards, `d/f.txt` unchanged (a different file); Unix
    statements keep mode and inode as before; paired `#[cfg(windows)]` blocks check the read-only attribute unchanged
    (new helper `readonly`, Windows only, one definition). Linux: 15 passed (as base); Windows stand-in clippy
    `--lib --test acceptance_t002 -D warnings` → `Finished`.
  - W2-W11 (`acceptance_t002_r2.rs`): `support` twins (`#[cfg(unix)]`/`#[cfg(windows)]`, real body each):
    `restore_perms` (Windows clears the read-only attribute), `set_mode` (Windows: no write bit sets read-only),
    `mode_of` and the type `Mode` (bits on Unix, read-only attribute on Windows), `make_link` (Unix: `symlink` with the
    same target string as before; Windows: target by `PathBuf::push`, `symlink_dir` if it resolves to a directory, else
    `symlink_file`, Pinned (d)); `Entry::File`'s `ino` is `#[cfg(unix)]` (ruling 2: the only difference of the
    snapshots). `mode`, `bits_do_not_stop_writes` and the `unix` import: `#[cfg(unix)]`, used only by W10/W11. W5's
    Windows branch (`cfg!(windows)`): open and `write_atomic` through `work/l.txt` are `OutsideRoot`, snapshot unchanged.
    W10, W11: `#[cfg(unix)]`, entries added now (registry kept in step with each file so `no-skip` stays green). Linux:
    fixtures and assertions the same (same link targets, same modes, same snapshot fields); 10 passed as base. Stand-in
    clippy Windows and macOS `-D warnings`: `Finished`. main's `no-skip`: PASS, 37 registered (10 pending).
  - W12-W21 (`acceptance_t008.rs`): twin `make_link` (Unix: `symlink` verbatim as before; Windows: Pinned (d), Q-B)
    and twin `describe` (Windows without the FIFO/socket branches) under one `snapshot`, whose keys are now
    `/`-joined components on every platform (on Linux the same strings; on Windows `assert_no_temp`'s `rsplit('/')`
    would otherwise see `\`-joined keys and check nothing); twin `identity_note` (W20/W21: the inode is only in the
    message; Windows names the kinds). `mkfifo`, `socket_at`, `MemoryFs` and the Unix imports: `#[cfg(unix)]`.
    W15, W16, W19: `#[cfg(unix)]`, entries added. W17 split: the original keeps every link that Windows can make
    (`gone`, `ghost2`, `up`, `self`, `dlink`) with the same assertions on Unix and `cfg!(windows)` → `outside_root` for
    `ghost2` and `self`; `outside_root_wins_over_not_found_unix` holds the FIFO `po` and the device `dn` with their
    preconditions and assertions unchanged (its own `Work` `precunix`), entry → the original. W18, W20, W21: same
    expected values on every platform (each answer is `outside_root` already), so no `cfg!(windows)` was needed.
    W14: part (c) (`c.txt` → `real/t.txt`, a link that stays inside) is followed on Unix; on Windows T-074 makes it
    `outside_root`, so I chose that expectation with `cfg!(windows)` (save refused, document dirty, target unchanged):
    see QUESTION 1. Linux: 11 passed (base 10 + the new `_unix`); stand-in clippy Windows and macOS: `Finished`;
    expected on Windows 7. main's `no-skip`: PASS, 41 registered (10 pending).
  - QUESTION 1 (W14): the table sorts `link_swapped_in_after_open_is_refused_at_save` as P ("the answer is
    `outside_root` on every platform"), but its part (c) saves through a link that stays inside, which T-074 refuses
    on Windows. I wrote it as P/w (`cfg!(windows)` on that part's expectation; no cfg attribute, so AC-2 holds).
    Recommendation: accept P/w for W14 (the same treatment M2 gave the split originals).
  - W22 (`acceptance_t015_tools.rs`): the `#[cfg(not(unix))] panic!` and the `#[cfg(unix)]` block go; the body runs on
    every platform with a twin `make_fake` (Unix: the same shell script text as before, mode 0755; Windows: a Rust
    source compiled by `rustc` from the test's `PATH` to `<tool>.exe`, appending `argv` to the marker, through
    `wait_with_timeout` (60 s), failing with "precondition: building the fake … failed" if `rustc` fails; a spawn that
    cannot start now panics naming the command). The child's `PATH` is `std::env::join_paths([bin, split_paths(PATH)…])`
    (on Unix the same `bin:PATH` string). `TIMEOUT`, `wait_with_timeout`, `MARKER_ENV` are now used on every platform.
    The generated Windows source was compiled and run on Linux in scratch (`rustc --edition=2021`; one marker line per
    start). Linux: 3 passed; stand-in clippy Windows and macOS: `Finished`.
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
