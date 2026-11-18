---
id: T-074
title: "State and enforce on Windows what a path may lead to: reparse points, drive and verbatim prefixes, reserved names, streams and case"
tier: high              # ADR-0002: a security or confinement boundary
state: draft
owner_role: planner
scorecard_items: []     # none claimed: the items exist and are verified on Linux; per-platform confirmation is the evaluator's (ADR-0009 Decision 3)
depends_on: [T-073]
touches:                # provisional, fixed at specification; each path one role (D-153)
  - crates/kiln-workspace/src/disk.rs        # implementer
  - crates/kiln-workspace/src/path.rs        # implementer: lexical refusals for Windows-meaningful segments, if the spec chooses them
  - docs/protocol/paths.md                   # implementer: one sentence per Windows difference
  - crates/kiln-workspace/tests/acceptance_t074.rs  # test author
  - apps/kiln-cli/tests/acceptance_t074_cli.rs      # test author
budget_tokens: 1100000
iteration: 2
found_by: planner       # ADR-0009 (R-255)
origin_task: null
---

# T-074 State and enforce on Windows what a path may lead to: reparse points, drive and verbatim prefixes, reserved names, streams and case

Draft by the planner of ADR-0009 (run R-255); a stub in the style of T-066 and T-068. Interface, Pinned
and acceptance criteria are written at specification, after T-070's first runs have replaced the
estimates of ADR-0009 with measurements.

## Goal
`paths.md` promises Unix behaviour ("as the kernel does", "Unix only"). On Windows the walk would treat a
drive or `\\?\` prefix in a link target as "restart from `/`" (`disk.rs:291`), compare paths
case-sensitively on a case-insensitive file system (`disk.rs:236`), and pass segments Win32 gives a meaning:
`C:`, `f.rs:stream`, `CON`/`NUL`/`COM1` with any extension, a trailing dot or space (INFERRED, ADR-0009 (b)).
After this task the Windows behaviour is stated per case in `paths.md`, every statement has a test that
runs on Windows, and no path whose location is outside the root is read or written there. ADR-0009 Q-6
recommends refusing every reparse point on the walk as outside the root for this stage.

## Estimate
The most uncertain task: confinement tasks of iteration 2 cost 750,000–930,000 before their spec checks
(T-036, T-037; stage-1 high p75 866,379), and this one is verified only through CI: **about 1,300,000**
(an Extend row is likely). Spec check: fresh, about 330,000, recommended for a security boundary.

## Notes from T-073 (director, D-250)
- **Nine registry entries wait on this task** (ruling c8): tests of `crates/kiln-workspace/src/disk.rs`'s own test
  module registered `unix` with `counterpart = "pending"`, `task = "T-074"`, expiring 2026-12-31 — the walk, links
  and a replaced root (`symlink_at_temp_name_is_not_followed`,
  `replace_swaps_inode_keeps_permissions_and_follows_symlinks`, the four root-replaced/removed/retargeted tests,
  `link_target_with_trailing_slash_requires_a_directory`, both `check_location` tests). This task writes their
  Windows counterparts or changes an entry to `none` with the reason; `xtask/platform-tests.toml` is in `touches`.
- **Measured in T-073's CI round 1** (`tasks/ci-runs/T-073-round-1.md`): the held root can be renamed on Windows
  (calls then answer `OutsideRoot`); renaming its parent is refused (OS error 5). Not tested: removing a held
  root; the identity check after a replaced parent (Windows refuses the rename that would set it up).
- **Read, not run** (`reviews/T-073-r1.md`): the shared code reachable on Windows since T-073 — `..` removed by
  text against `canonicalize` on disk; junctions counted as links; `:` reaching alternate data streams; a
  drive-relative `C:x`; case and reserved names untreated; the save's new file takes the directory's access list
  and the temporary file is not owner-only (ruling c4); an error reading the root's identity answers "outside"
  on Windows where Unix goes on (finding 5).

## Log
### director
<!-- end director log -->
### planner
- 2026-11-15 · R-255 · draft · did: filed from ADR-0009 · next: director
<!-- end planner log -->
