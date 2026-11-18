---
id: T-100
title: "Make the two listing test targets of kiln-workspace (acceptance_t019_workspace, project) compile, lint clean and pass on Windows under T-092's sorting rule, with a paired helper for the name a listing must skip"
tier: high              # provisional: gate path (xtask/platform-tests.toml, xtask/tests/); the tests of the confinement boundary's listings change what they assert on Windows
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-092, T-095]   # T-092: its sorting rule, and one registry at a time (§5.2); T-095: what names mean on Windows (the Windows clause of `skipped`, project.md:26, :99)
touches:                # provisional, fixed at specification; one role per path (D-153)
  - crates/kiln-workspace/tests/acceptance_t019_workspace.rs   # implementer
  - crates/kiln-workspace/tests/project.rs                     # implementer
  - xtask/platform-tests.toml                                  # GATE PATH: implementer
  - xtask/tests/acceptance_t100.rs                             # GATE PATH: test author (registry equality; fails on base)
  - xtask/tests/guard_t100.rs                                  # GATE PATH: test author (the portable rows carry no platform cfg)
budget_tokens: 1100000  # high (v1.5); estimate about 550,000
iteration: 2
found_by: planner       # cut from T-092 at its spec check (D-273)
origin_task: T-092
---

# T-100 The listing tests on Windows

Draft cut from T-092 by the director's ruling on its spec check (D-273; `tasks/T-092-mark-product-tests-for-windows.md`,
"Spec check (fresh planner)", M3 and "After the rulings").

## Goal (draft)
After T-092, `acceptance_t019_workspace` and `project` are the two `kiln-workspace` test targets of the general Windows
step that still exit 101: they do not compile there (5 and 3 errors on main `32b597d`: the `std::os::unix` imports and
`FileType::is_fifo`/`is_socket`; VERIFIED with the Windows clippy stand-in of `reviews/T-073-r1.md`, T-092 spec check
point 3). After this task both compile, are clippy-clean with `-D warnings` and exit 0 on Windows, so the step shows 15
of 15; no Linux test name disappears (`xtask/tests/guard_t079.rs`) and no Linux assertion is weakened.

## The rows moved from T-092 (its Interface (1), sorted by its rule)
- W23 `acceptance_t019_workspace.rs::disk_list_dir_reports_entries_by_their_own_kind_and_skips_bad_names`: split; the
  original P/w (links inside answer `outside_root` on Windows), `_unix` holds FIFO and socket, counterpart `none`.
- W36-W38 `project.rs::disk_list_dir_reports_own_kinds_sorted_and_skips_odd_names`,
  `disk_list_dir_errors_follow_t008_precedence`, `symlinks_and_fifos_are_listed_not_followed`: split each, originals P/w,
  `_unix` → `none`.
- W39 `project.rs::a_prefix_directory_outside_the_root_refuses_the_path`: P/w.
- W42, W43: new `#[cfg(windows)]` tests `disk_list_dir_reports_entries_by_their_own_kind_and_skips_bad_names_not_unicode`
  and `tree_counts_skipped_names_on_disk_not_unicode` (T-092 ruling 3, D-269; PROBE P-A1-A6 of
  `tasks/ci-runs/T-092-probe-1.md:442-447`: such a name is made, skipped and counted), entries `windows`/`none`.
- Names holding `\` (T-092 spec check M3; ruling Q-A B, D-273): `acceptance_t019_workspace.rs:221` (W23), `:514`
  (`tree_counts_skipped_names_on_disk`), `project.rs:121` (W36), `:425` (`documented_differences_from_git`). On Windows
  `\` separates components, so these fixtures cannot be made (INFERRED; never run). Ruled: a paired fixture helper makes
  the name the listing must skip, `back\slash` on Unix and the unpaired-surrogate name of P-A on Windows, each with a
  real body (`no-skip`'s `inert-helper` rule); the four tests stay portable, **no new registry entry** for them.
- Registry: 6 new entries (W23`_unix`, W36-W38`_unix` `unix`/`none`; W42, W43 `windows`/`none`), and the four T-079
  non-UTF-8 entries of these files keep `platforms = "linux"`, `counterpart = "none"` (pinned by
  `xtask/tests/acceptance_t079.rs`; a `_unix` item does not end in `_non_utf8`, so ruling w1 is not touched) with their
  `reason` naming W42/W43 instead of "the Windows side is T-092's".
- Unchecked until this task's first CI round, as in T-092's Pinned (d): link fixtures beyond T-074's measured practice
  (`PathBuf::push` targets, directory links removed with `remove_dir`).

## Specification inputs
- T-092's sorting rule and its spec check (points 1-4, M2-M4); T-095's sentences for `skipped` on Windows once merged; the
  registry as T-092 leaves it; T-092's first CI record (what its P/w forms showed on Windows).

## Estimate
About 550,000 of 1,100,000: implementer about 150,000 (2 files, 7 rows, the paired helper, 6 entries and 4 reasons),
test author about 150,000 (registry equality and a portable-rows guard, the shape of T-092's), reviewer about 200,000,
rework about 50,000. The fixed part of a high task here is about 450,000-500,000 (T-092 "Cut"); booked costs of the
nearest tasks: T-073 860,977, T-074 920,909, T-079 846,099 (`metrics/tasks.jsonl`).

## Order
After T-092 (registry) and T-095 (depends_on). It shares `xtask/platform-tests.toml` with T-098, so the two are not in
flight together (§5.2): the director orders them. T-099 needs this task too (every `kiln-workspace` target compiles).

## Note from T-092's review (director, D-281)
`crates/kiln-workspace/tests/acceptance_t008.rs:331-333`: a comment states as fact that Windows reads `..` in a
link target by its text before following the link; when it was written that was an inference from the root-path
probe. CI round 2 of T-092 then measured the fixture (`root/t` does not reach; `tasks/ci-runs/T-092-round-2.md`):
the comment may cite that run. A small edit for whichever task next touches the file.

## Log
### director
<!-- end director log -->
### planner
- 2026-11-18 · planner (T-092's spec check, resumed for D-273) · draft · did: filed from the cut with the rows moved,
  ruling Q-A B, the estimate · next: specification after T-092 and T-095 merge
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
