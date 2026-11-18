---
id: T-099
title: "Make the Windows gate job green: fix or register the Windows failures of test targets that compile, pass the scorecard step on Windows, and propose counting gate-windows"
tier: high              # provisional: gate paths (xtask/platform-tests.toml) and the tests it changes; fixed at specification from the probe
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-098]     # after T-098 every target compiles on Windows, so the gate's test and scorecard steps run them all
touches: []             # fixed at specification from the probe record (T-092's probe, commit b57a74e) and T-098's CI round
budget_tokens: 1100000  # high (v1.5); estimate 500,000-900,000, from the probe
iteration: 2
found_by: planner       # cut from T-092 at its specification (2026-11-18)
origin_task: T-092
---

# T-099 gate-windows green

Draft cut from T-092 at its specification (`tasks/T-092-mark-product-tests-for-windows.md`, "Cut").

## Goal (draft)
After T-092 and T-098 every test target compiles on Windows. What may still fail there is **unchecked**: the targets that
compile today have never run on Windows, because the gate's `test` step stops at the first build errors (VERIFIED:
`tasks/ci-runs/T-072-round-3.md`, 0 `test result:` lines in `test-output.txt`): every target of `kiln-commands` but two,
`kiln-control`, `kiln-ui`, `kiln-testkit`, 23 of `kiln-cli`'s, 9 of `kiln`'s, and the doc tests. T-092's probe
(`probe-t092-windows`, one `cargo test` per target) measures them; T-098's CI round shows the rest. This task fixes or
registers each failure under T-092's sorting rule, makes the scorecard step pass on Windows (a listed test either runs and
passes there or is registered absent), and ends with `gate-windows` conclusion `success` on its rebased head and then on
main. **It proposes counting `gate-windows`** (METHODOLOGY §8.1: a job counts from its first green run on main; the
director's row names that run).

## Specification inputs
- The probe record (to be written by the director under `tasks/ci-runs/`), and T-098's last CI record.
- Pending entries allowed at the end: T-076's 3 and T-077's 5 (`no-skip` accepts them until 2026-12-31).

## Log
### director
<!-- end director log -->
### planner
- 2026-11-18 · planner (T-092's specification run) · draft · did: filed from T-092's cut · next: director (after the probe round)
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
