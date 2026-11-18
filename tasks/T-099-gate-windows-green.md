---
id: T-099
title: "Make the Windows gate job green: fix or register the Windows failures of test targets that compile, pass the scorecard step on Windows, and propose counting gate-windows"
tier: high              # provisional: gate paths (xtask/platform-tests.toml) and the tests it changes; fixed at specification from the probe
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-098, T-100]     # T-100 added with D-273; after T-098 every target compiles on Windows, so the gate's test and scorecard steps run them all
touches: []             # fixed at specification; at least apps/kiln-cli/tests/{acceptance_t004_r2,acceptance_t004_r3,guard_t015_cli,guard_t032_cli}.rs (probe) and what T-098's CI round and the xtask.exe note add
budget_tokens: 1100000  # high (v1.5); estimate 500,000-700,000 (4 tests, the xtask.exe note, T-098's leftovers, the counting run)
iteration: 2
found_by: planner       # cut from T-092 at its specification (2026-11-18)
origin_task: T-092
---

# T-099 gate-windows green

Draft cut from T-092 at its specification (`tasks/T-092-mark-product-tests-for-windows.md`, "Cut").

## Goal (draft)
After T-092, T-100 (the two listing targets of `kiln-workspace`, cut from T-092 by D-273) and T-098 every test
target compiles on Windows. What may still fail there is **unchecked**: the targets that
compile today have never run on Windows, because the gate's `test` step stops at the first build errors (VERIFIED:
`tasks/ci-runs/T-072-round-3.md`, 0 `test result:` lines in `test-output.txt`): every target of `kiln-commands` but two,
`kiln-control`, `kiln-ui`, `kiln-testkit`, 23 of `kiln-cli`'s, 9 of `kiln`'s, and the doc tests. T-092's probe
(`probe-t092-windows`, one `cargo test` per target) measures them; T-098's CI round shows the rest. This task fixes or
registers each failure under T-092's sorting rule, makes the scorecard step pass on Windows (a listed test either runs and
passes there or is registered absent), and ends with `gate-windows` conclusion `success` on its rebased head and then on
main. **It proposes counting `gate-windows`** (METHODOLOGY §8.1: a job counts from its first green run on main; the
director's row names that run).

## Inventory from T-092's probe (run 44355322518, `tasks/ci-runs/T-092-probe-1.md` lines 128-158)
Of 96 targets run one by one on `windows-2025`, 6 compile and fail. Two are T-092's (`kiln-workspace` `acceptance_t002`,
`acceptance_t015_tools`). **The other four are this task's**, verbatim:
- `kiln-cli --test acceptance_t004_r2`: `test huge_values_are_echoed_truncated_to_200_chars ... FAILED`; panicked at
  `apps\kiln-cli\tests\acceptance_t004_r2.rs:48:56`: `spawn kiln-cli: Os { code: 206, kind: InvalidFilename, message: "The
  filename or extension is too long." }`; `4 passed; 1 failed`.
- `kiln-cli --test acceptance_t004_r3`: `test exec_errors_on_stderr_obey_size_caps ... FAILED`; panicked at
  `apps\kiln-cli\tests\acceptance_t004_r3.rs:58:31`: the same `spawn kiln-cli: Os { code: 206, … too long." }`; `4 passed;
  1 failed`.
- `kiln-cli --test guard_t015_cli`: `test cli_output_is_byte_identical_to_base ... FAILED`; panicked at
  `apps\kiln-cli\tests\guard_t015_cli.rs:132:9`: `session_run_files differs from the base recording (actual saved to
  …\target\tmp\guard_t015_cli\session_run_files.actual)`; `1 passed; 1 failed`.
- `kiln-cli --test guard_t032_cli`: `test protocol_bytes_are_unchanged ... FAILED`; panicked at
  `apps\kiln-cli\tests\guard_t032_cli.rs:134:9`: `session_run_files differs from the base recording (actual saved to …)`;
  `1 passed; 1 failed`.
Every other compiling target passes, the doc tests of all six libraries included (64 exit 0). Not shown by the record:
what in `session_run_files` differs (the `.actual` file was not uploaded), and how long the arguments of the two
os-error-206 tests are (the Windows command line holds at most 32,767 UTF-16 units: KNOWN, not measured). The first
step of the specification is to measure both (a probe or the first CI round). The 17 targets T-098 makes compile have not
run on Windows yet; their failures, if any, come from T-098's CI round. Overlap: `acceptance_t004_r2.rs` is also T-076's
(a `/dev/full` counterpart): the director orders T-076 and this task.

## Specification inputs
- The probe record `tasks/ci-runs/T-092-probe-1.md`, and T-098's last CI record.
- Pending entries allowed at the end: T-076's 3 and T-077's 5 (`no-skip` accepts them until 2026-12-31).

## Note from T-072's CI round 5 (director, D-265)
On Windows the gate's `test` step now ends its kept text with "failed to remove `xtask.exe`: Access is denied"
(`tasks/ci-runs/T-072-round-5.md`; first seen after T-096 merged). Not a test target. To be explained by this task:
probably a build that tries to replace the running `xtask.exe` (the gate runs under it), perhaps from the nested
`cargo test` of `xtask/tests/acceptance_t096.rs`. INFERRED, not shown.

## Log
### director
<!-- end director log -->
### planner
- 2026-11-18 · planner (T-092's specification run) · draft · did: filed from T-092's cut · next: director (after the probe round)
- 2026-11-18 · planner (T-092's finalization) · draft · did: added the probe's four failing tests with their messages; kept the director's note · next: specification after T-098
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
