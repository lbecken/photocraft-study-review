---
id: T-073
title: "Make the workspace build on Windows: the root's identity, holding it open, and saving, with the Windows sentences of the save and the root"
tier: high              # ADR-0002: persistence path and confinement boundary (disk.rs), a dependency edge (same-file); approval row needs neither `dependency versions` (probe a6: no version changes) nor `gate paths`
state: draft
owner_role: planner
scorecard_items: []     # none claimed: the items exist and are verified on Linux; per-platform confirmation is the evaluator's (ADR-0009 Decision 3)
depends_on: [T-070, T-071]
touches:                # provisional, fixed at specification; each path one role (D-153)
  - crates/kiln-workspace/src/disk.rs        # implementer
  - crates/kiln-workspace/Cargo.toml         # implementer: `same-file` (already in the lock at 1.0.6, probe a6)
  - Cargo.lock                               # generated
  - docs/protocol/paths.md                   # implementer: "The root" for Windows
  - docs/control-protocol.md                 # implementer: the save sentence ("on Unix", line 133)
  - crates/kiln-workspace/tests/acceptance_t073.rs  # test author
budget_tokens: 1100000
iteration: 2
found_by: planner       # ADR-0009 (R-255)
origin_task: null
---

# T-073 Make the workspace build on Windows: the root's identity, holding it open, and saving, with the Windows sentences of the save and the root

Draft by the planner of ADR-0009 (run R-255); a stub in the style of T-066 and T-068. Interface, Pinned
and acceptance criteria are written at specification, after T-070's first runs have replaced the
estimates of ADR-0009 with measurements.

## Goal
On Windows `kiln-workspace` does not compile: 6 errors, all in `disk.rs` (`:86` import, `:189`/`:213`
`dev()`/`ino()`, `:361` the temporary file's mode; probe a3, VERIFIED). After this task it builds on all
three, and on Windows: the root is identified by what `same-file` compares (probe a6: compiles for
`x86_64-pc-windows-msvc`, no new package); a root directory can be held open (KNOWN: `std` cannot open a
directory on Windows without the backup-semantics flag, so as written every root would be refused); a
save writes the temporary file and replaces the target, and the documents say what differs (no mode
bits: the read-only attribute is kept; a target another process holds open cannot be replaced and the
save fails with `io_error`, the target unchanged; directory sync does nothing). The Unix behaviour is
unchanged (the existing tests pass on Linux and macOS). The confinement walk on Windows is T-074's.

## Open questions for the specification
- Is a root that is held open still renamable or removable on Windows? The `paths.md` sentences on a
  replaced root depend on it; measured on the runner by the test author.
- `same-file` as a direct dependency (an edge, no version) versus Windows calls through `windows-sys`
  (already in the lock through the toolkit; but `unsafe_code` is denied workspace-wide).

## Estimate
High; **about 1,100,000** (code in the confinement module, verified only through CI runs).

## Log
### director
<!-- end director log -->
### planner
- 2026-11-15 · R-255 · draft · did: filed from ADR-0009 · next: director
<!-- end planner log -->
