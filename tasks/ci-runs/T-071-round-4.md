# CI run 44220309842 (platforms.yml) on ci/candidate @ b7dd7d621f4a90b4eb7165f6118a0be88fb7dad1 — T-071 round 4

Record by run R-322 (planner of ADR-0009); method as `tasks/ci-runs/T-071-round-3.md` (artefacts first).

## Run header
- id 44220309842, run_number 17, attempt 1, event push, workflow `platforms`, title "T-071: implementer log (R-319)"
- branch `ci/candidate`; commit `b7dd7d621f4a90b4eb7165f6118a0be88fb7dad1`; created_at 2026-11-18T02:02:32Z
- status completed, **conclusion failure**; updated_at 2026-11-18T02:27:08Z

## Job table
| job | id | conclusion | wall | first failing step |
|---|---|---|---|---|
| gate-linux | 134520221898 | **success** | 11m40s (02:02:35Z–02:14:15Z) | none |
| gate-macos | 134520222133 | failure | 24m33s (02:02:35Z–02:27:08Z) | Gate (02:02:47Z–02:09:27Z); Upload reports success |
| gate-windows | 134520222357 | failure | 14m22s (02:02:35Z–02:16:57Z) | Gate (02:03:08Z–02:05:37Z) |
| window-linux | 134520222218 | **success** | 1m47s (02:02:35Z–02:04:22Z) | none |
| window-probe (macos-15) | 134520222132 | **success** | 1m58s (02:02:36Z–02:04:34Z) | none |
| window-probe (windows-2025) | 134520222111 | failure | 2m19s (02:02:35Z–02:04:54Z) | Build kiln (02:03:09Z–02:04:50Z) |

Artefacts (6, all uploaded): `gate-macos-…` 33,500 B, `gate-windows-…` 17,334 B, `gate-linux-…` 10,064 B,
`window-probe-macos-15-…` 705 B, `window-linux-…` 577 B, `window-probe-windows-2025-…` 292 B; fetched with
`download_workflow_run_artifact` into `/home/user/kiln-plan-T-071-artefact/` (deleted after this record), each zip's size
equal to `size_in_bytes`, `testzip()` clean; read as data.

Counted jobs' verdicts: `window-linux` — six `CASE` lines (ac2 0/0, ac2x3 3/3, dblclick 0/0, scroll100 0/0, ctrlq 0/0,
noroot 0/0), `WINDOW CASES PASS`, `AC-3 PASS`, `PLAIN PASS: the start path changes the screen`; `window-probe (macos-15)` —
`PROBE: exit status 0`, `PROBE: stdout     2174 bytes, 3 lines`. (`window-probe (windows-2025)`: `PROBE: not run (cargo
build -p kiln: failure)`.)

## `xtask` per platform (`core-crates.txt`, `cargo test --locked --no-fail-fast -p xtask`)
| | Linux | macOS | Windows |
|---|---|---|---|
| exit status line | `==== xtask: exit status 0` | `==== xtask: exit status 0` | **`==== xtask: exit status 0`** |
| test binaries run (`Running` + `test result:`) | 21 | 21 | 21 |
| passed / failed | 278 / 0 | 273 / 0 | **270 / 0** |
Windows verdicts: `test seed_reuses_non_member_artifacts_only ... ok` (`acceptance_t005`: 16 passed) and
`test seeded_worktree_builds_its_own_source ... ok` (`guard_t005`: 2 passed). No failing `xtask` test on any platform.

## Named suites (passed / failed, identical on Linux, macOS and Windows)
`guard_t054` 2/0 (`ac2_panic_location_is_this_file_and_line`, `ac2_backtrace_names_this_file_and_line` both `ok`);
`acceptance_t071` 28/0; `guard_t071` 6/0; `acceptance_t070` 13/0; `guard_t070` 7/0.

## Workspace and the scorecard's first line
- macOS (`test-output.txt`, whole output): 131 `test result:` lines, **1,101 passed, 16 failed**; the failing set is exactly
  R-285's 16 product tests (none new, none gone). Gate: `scorecard` and `test` FAIL, all other steps PASS/EMPTY.
- `scorecard` first line, macOS: `macos: 64 items; 219 tests ran; 0 registered absent here; page not compared (computed on
  Linux)`, then `6 scorecard problems:`. Windows: `windows: 64 items; 18 tests ran; 0 registered absent here; page not
  compared (computed on Linux)`, then `202 scorecard problems:` (Windows gate red at clippy, schema, scorecard, test:
  `kiln-workspace` does not compile, T-073). Linux: `64 items: 47 verified, 0 implemented, 0 failing, 17 planned.`
- `no-skip` on all three: `self-test ok (17 known findings); 197 files scanned, 0 allowlisted skip(s), 8 platform test(s)
  registered (0 pending), today = 2026-11-18`. Linux gate: all PASS, `1122 passed, 0 failed, 0 ignored in 131 suites`.

## Against T-071's AC-6 (scorecard clause as ruling s10 reads it)
| Clause | Verdict | Quote |
|---|---|---|
| `xtask` exit status 0 on gate-macos | **met** | `==== xtask: exit status 0` (273 / 0) |
| `xtask` exit status 0 on gate-windows | **met** | `==== xtask: exit status 0` (270 / 0) |
| gate-linux green | **met** | job success; `passed: true` |
| macOS gate red until T-079 (expected) | met | scorecard, test FAIL; the 16 product tests |
| Windows gate red until T-073 (expected) | met | clippy, schema, scorecard, test FAIL (`kiln-workspace`) |
| scorecard off Linux: platform line first, page not compared, whether it passes or reports problems | **met** | the two first lines quoted above, each followed by its problem list |

## Differences from round 3 (run 44215218474)
- Windows: `acceptance_t005::seed_reuses_non_member_artifacts_only` FAILED → ok; `guard_t005::seeded_worktree_builds_its_own_source`
  FAILED → ok (`xtask` 268/2 exit 101 → 270/0 exit 0). No other verdict changed (by the per-binary totals: every other binary has the same count as in round 3 and 0 failed).
- Linux, macOS: no test changed verdict (`xtask` 278/0 and 273/0 as before; macOS workspace 1,101/16, the same 16).

## Not read
- Job logs (all artefacts uploaded).
