---
id: T-095
title: "On Windows, refuse lexically the path segments Windows gives a meaning (streams and drive prefixes, reserved device names, trailing dot or space, <>\"|?* and control characters), and state how case and short names behave"
tier: high              # METHODOLOGY v1.5 §4: the confinement boundary (the lexical check of every request path); gate paths (.github/workflows/platforms.yml, xtask/tests/acceptance_t070.rs)
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-074]     # cut from T-074 (its Q-1); shares docs/protocol/paths.md, platforms.yml, acceptance_t070.rs
touches:                # provisional, fixed at specification; one role per path (D-153)
  - crates/kiln-workspace/src/path.rs        # implementer: the Windows lexical refusals in `RelPath::new` and its unit tests
  - docs/protocol/paths.md                   # implementer: "Lexical refusals" for Windows; the case and short-name limits; T-074's "T-095's" clause removed
  - docs/control-protocol.md                 # implementer: a Windows sentence after the lexical bullet, which stays verbatim (acceptance_t008_cli.rs:560-563)
  - docs/protocol/project.md                 # implementer, if names refused on Windows are counted in `skipped` (to be decided at specification)
  - .github/workflows/platforms.yml          # GATE PATH: implementer: the guard file added to T-074's reported step
  - xtask/tests/acceptance_t070.rs           # GATE PATH: implementer: that step's expectation
  - crates/kiln-workspace/tests/guard_t095.rs   # test author: Windows runtime tests (pass on Unix base)
  - crates/kiln-workspace/tests/acceptance_t095.rs  # test author: what fails on base on every platform, if any (to be decided at specification)
budget_tokens: 1100000  # high (v1.5); estimate about 500,000
iteration: 2
found_by: planner       # cut from T-074 at its re-specification (ADR-0009 (b), R-255)
origin_task: null
---

# T-095 Windows: names Windows gives a meaning, and case

A draft cut from T-074 (its Q-1, proposed): T-074 keeps the walk and reparse points (`disk.rs`); this task takes the
lexical side (`path.rs`). Specified in full after T-074 merges and after T-074's probe record
(`tasks/ci-runs/T-074-probe-1.md`) has shown what the current code does with these names on the runner.

## Goal (draft)
`RelPath::new` (`crates/kiln-workspace/src/path.rs:29-57`) refuses the same forms on every platform: empty, leading `/`,
`.`/`..`/empty segments, `\`, NUL. On Windows a segment can carry a meaning the walk does not see: `f.rs:s` names an
alternate data stream of `f.rs`; `C:x` or `C:` makes `Path::join` drive-relative (the final inside check should refuse
it; INFERRED); `CON`, `NUL`, `COM1`… (any extension), a trailing dot or space, `<>"|?*` and control characters 1–31 are
special to Win32 (with the `\\?\` paths the walk uses, possibly ordinary names; INFERRED). Case: NTFS folds case, so
`A.rs` and `a.rs` are one file and two documents ("already open" is by path text), and 8.3 short names alias long ones.
After this task each such form has one stated Windows behaviour in `paths.md` (refused as `invalid_path` before the file
system is consulted, or stated as a limit), tested on the runner.

## Questions for the specification (to the director)
- Q-1: Refuse on Windows only (Linux and macOS unchanged; a per-platform sentence) or on every platform (portable
  projects; but a Linux file named `a:b` becomes unopenable and is then omitted from listings as `skipped`). Planner's
  leaning: Windows only.
- Q-2: Case and short names: state as limits (two documents for one file), or refuse a second open of the same file
  under another spelling (that is `kiln-commands`' open-document table: another crate, a contract question).
- Q-3: The set refused, written as equality ("exactly …"), with the reserved-name list fixed from the probe record (for
  example whether `COM¹` and `CONIN$` count on the runner).

## Estimate
About 500,000: a short product change, unit tests in `path.rs` (they run on Windows in T-073's `--lib` step), a guard file
by an independent test author, three or four document places, one more test target in T-074's step, 1–2 CI rounds.

## Log
### director
<!-- end director log -->
### planner
- 2026-11-18 · draft · did: cut from T-074 at its re-specification under v1.5 · next: director (T-074 Q-1)
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
