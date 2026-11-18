---
id: T-072
title: "Make the gate pass on macOS, and state in the documents where Kiln behaves differently there"
tier: high              # provisional: likely to touch crates/kiln-workspace/src/disk.rs (confinement) and paths.md; fixed at specification from T-070's first macOS run
state: draft
owner_role: planner
scorecard_items: []     # none claimed: the items exist and are verified on Linux; per-platform confirmation is the evaluator's (ADR-0009 Decision 3)
depends_on: [T-070, T-071]
touches:                # provisional, fixed at specification; each path one role (D-153)
  - crates/kiln-workspace/src/disk.rs   # implementer, only if T-070's run shows a product fault
  - docs/protocol/paths.md              # implementer: one sentence per macOS difference
  - docs/protocol/ui.md                 # implementer: the macOS key sentence (`command` is ⌘)
  - (test files named by T-070's run)   # test author; each path one role
budget_tokens: 1100000
iteration: 2
found_by: planner       # ADR-0009 (R-255)
origin_task: null
---

# T-072 Make the gate pass on macOS, and state in the documents where Kiln behaves differently there

Draft by the planner of ADR-0009 (run R-255); a stub in the style of T-066 and T-068. Interface, Pinned
and acceptance criteria are written at specification, after T-070's first runs have replaced the
estimates of ADR-0009 with measurements.

## Goal
Everything compiles for macOS (ADR-0009 probe a4, a5; VERIFIED with the C code stubbed); whether it passes
is unknown. After this task `gate-macos` is green on main, and each behaviour that differs on macOS is one
sentence with a test that runs there: candidates (INFERRED, ADR-0009 (b)): case-insensitive names (`A.rs`
and `a.rs` are one file and two documents), `PATH_MAX` 1,024, the `nobody` identity, Control and ⌘ in
the editor's keys. The confinement tests already compile on macOS; those that assume uid 0 are T-069's.

## Estimate
Unknown until T-070 runs: **about 600,000** if only tests and sentences change; up to the high ceiling
if `disk.rs` must change. Re-estimate at specification.

## Notes from T-070's CI rounds (director, 2026-11-15)
- Evidence to start from: `tasks/ci-runs/T-070-round-1.md` and `T-070-round-2.md` (`gate-macos`: the scorecard's 7
  problems; `xtask` 116 passed, 3 failed, names not printed; the first 200 lines of the full output with 11 failing
  tests named).
- **What the jobs do not show**: a failing test's panic message is in no job log (seen on the negative candidate,
  `tasks/ci-runs/T-070-negative.md`), the three failing `xtask` tests are not named, and the full output is cut at
  200 lines. This task's spec decides whether the report steps print failing tests with their messages (a change
  to `.github/workflows/platforms.yml`, a gate path) or the artefacts are read instead.
- **Follow-ups from T-070's review round 2** (`reviews/T-070-r2.md`, new findings 1 to 3; the first task that edits
  `.github/workflows/platforms.yml` or `xtask/tests/acceptance_t070.rs` takes them, this one or T-077): (1) the
  probe's time bound is tested as the text `120`, not as a bound (`sleep 2` in the watchdog or a dropped `kill -9`
  pass the gate), and no gate test covers "every recording step ends with exit 0"; (2) two assertions pin a layout
  no spec text pins (the probe reading the build's result through `steps.<id>.outcome`, and the literal `120`):
  loosen them when the workflow next changes; (3) `docs/development.md`: the macOS disk cell leaves out the
  record's inode columns although the sentence says "as printed".
- **From the re-specification of T-043 (D-223, ruling Q-8 B):** `incidents/` becomes a ledger path in this task
  (`xtask/ledger.toml`, the `paths-ignore` list of `platforms.yml` and the expectation in
  `xtask/tests/acceptance_t070.rs`, which checks the two against each other).
- **From v1.5 change 1 (D-219):** the evidence steps run every test binary (`--no-fail-fast`) and print failing
  tests with their messages; the full output is not cut at 200 lines.
- **From T-071's review round 2 (D-232, finding R2-4):** the `echo "==== cargo test --locked -p $p"` label of
  the core-crates step no longer matches the command, which has `--no-fail-fast` since T-071.

## Log
### director
<!-- end director log -->
### planner
- 2026-11-15 · R-255 · draft · did: filed from ADR-0009 · next: director
<!-- end planner log -->
