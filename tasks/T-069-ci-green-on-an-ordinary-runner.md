---
id: T-069
title: "Make the gate pass in CI again: the confinement tests that change identity pass, or fail with their reason, whatever the identity and location of the checkout"
tier: low               # ADR-0002: tests and documentation only, no behaviour change; scorecard_items empty (the tests stay tied to ws.path-confinement, which is not claimed). See ADR-0009 Q-7: a reviewer reads the diff anyway
state: in_progress
owner_role: planner
scorecard_items: []
depends_on: []
touches:                # provisional, fixed at specification; one role per path (D-153): all implementer
  - crates/kiln-workspace/tests/acceptance_t036_workspace.rs
  - crates/kiln-workspace/tests/confinement.rs            # only if its identity helper is the same fault
  - apps/kiln-cli/tests/acceptance_t036_cli.rs
  - apps/kiln-cli/tests/guard_t036_removal_cli.rs
  - apps/kiln-cli/tests/acceptance_t037_cli.rs            # the two T-052 tests that need a reachable checkout
  - apps/kiln-cli/tests/guard_t037_cli.rs
  - docs/development.md                                   # one paragraph: what these tests need from the machine
  - incidents/INC-001-ci-red-since-t036.md                # implementer writes it with the text the director gives below (D-175): `incidents/` is not a ledger path
budget_tokens: 150000   # low (v1.3)
iteration: 2
found_by: planner       # R-255 (ADR-0009, Context "Kiln today"): CI red on main since run 257
origin_task: T-036
---

# T-069 Make the gate pass in CI again

Draft by the planner of ADR-0009 (run R-255). It **absorbs the deferred draft T-052** (same tests, same
cause family); if the director agrees, T-052 is abandoned in favour of this task (its evidence is cited
below). Interface, Pinned and acceptance criteria are completed at specification.

## Goal
Every run of `ci.yml` on main since run 257 (`47135bc`, 2026-11-14 04:34Z, the first push after T-036
merged at `1a808c9`) to run 353 failed (VERIFIED, `actions_list` of `ci.yml`, R-255): the `scorecard`
step reports three `ws.path-confinement` tests as failing on the hosted runner
(`kiln_cli::acceptance_t036_cli::unreadable_root_is_refused_at_start`,
`kiln_workspace::acceptance_t036_workspace::unreadable_root_is_refused_by_new`,
`kiln_cli::guard_t036_removal_cli::root_made_unreadable_after_start_stays_protected_through_control`;
job logs of runs 257 and 352). They re-run themselves as uid 65534 from uid 0
(`acceptance_t036_workspace.rs:7,38,47,78`); the runner's user is not uid 0 (KNOWN). In a checkout that
the other identity cannot reach, these and two T-037 tests fail as well (R-255 probe c2: five failures
below `/tmp/claude-0`, mode 0700; `evaluations/iter-2.md` NEW-3, T-052).

After this task, each of these tests checks what it checks now, both when the test process is uid 0 and
when it is an ordinary user, and in a checkout anywhere: as an ordinary user, a root the user cannot
read is made directly (the user's own directory with its read permission removed), so no identity change
is needed; as uid 0, the identity change stays. Where a test's precondition cannot be met (uid 0 and the
other identity cannot reach the checkout), it fails with a message that names the precondition and the
path that blocks it, never passes. `docs/development.md` says what the tests need. The gate is green on
the next CI run of main.

Not a gate path; no product code changes. The director opens `incidents/INC-001-*.md` for the red
CI before this task is specified (METHODOLOGY §8), and closes it with the first green run.

## Evidence in the record
- R-255 probe e5 and d3 (ADR-0009): 97 failures and 3 successes among the last 100 completed runs; last
  success run 256 (`805ebcb`).
- `evaluations/iter-2.md` NEW-3; T-052's evidence (director, 2026-11-14).
- The scorecard step prints only the failing test names, not their panic messages (job log of run 352),
  so the cause on the runner is INFERRED from the code; the implementer reproduces both identities here
  (uid 0 directly; an ordinary identity with `setpriv`/`runuser` or the DAC capabilities dropped, as the
  evaluator did, `evaluations/iter-2-cut-1.md`).

## Open questions for the specification
- Does any other test assume uid 0 (`grep -rn "65534\|\.uid(" --include=*.rs crates apps`: 7 files)?
  Only those that fail on an ordinary identity are in scope.
- Pin the oracle: on both identities, the refusal is the same `io_error`/`PermissionDenied` with path
  `(root)` as today (the tests' assertions do not change, only how the unreadable root is made).

## Acceptance (sketch)
- AC-1: run as uid 0 here and as an ordinary identity here, in a checkout reachable by both: the five
  tests pass, and each fails when the refusal of an unreadable root is removed from `disk.rs` (a mutant
  the reviewer of Q-7 seeds).
- AC-2: in a checkout below a directory of mode 0700 owned by root, run as uid 0: the identity-changing
  tests fail with the precondition message naming that directory.
- AC-3: the next `ci.yml` run on main after the merge is green (read by the director, recorded).

## Estimate
Low tier figures: 84,466–124,444 per low task (`retrospectives/iter-2-stage-1.md` Metrics); about
120,000 here, plus a reviewer's read of the diff (about 30,000, ADR-0009 Q-7). Spec check by the planner
that drafted it: about 35,000 (D-163).

## Rulings at specification (director, D-175)
- **Tier low, with a reviewer's read of the diff** (ADR-0009 Q-7 A): the task changes tests of a high-tier item
  and no product code; the reviewer seeds the mutant of AC-1 and reads CI's result after the merge.
- **Oracle pinned:** on both identities the assertions of the five tests do not change; only how the unreadable
  root is made differs (an ordinary user removes its own read permission from a directory it owns; uid 0 changes
  identity as today). A test whose precondition cannot be met fails with a message that names the precondition
  and the path that blocks it; it never passes and is never skipped.
- **Other tests that assume uid 0** are in scope only if they fail as an ordinary identity; the implementer runs
  the whole gate as an ordinary identity here and lists every test that fails there.
- **T-052 is abandoned in favour of this task** (ADR-0009 Q-9).
- **The incident file.** The implementer creates `incidents/INC-001-ci-red-since-t036.md` with exactly this
  content (the director fills `green_at` in a decision row when the first green run is read, since the file
  cannot change on main outside the queue):

```
---
id: INC-001
title: "CI on main red since the merge of T-036: three confinement tests fail on the hosted runner"
class: red-main
detected_at: 2026-11-15T16:37Z
detected_by: planner of ADR-0009 (run R-255) reading GitHub Actions; confirmed by the director (runs 353 to 357, job log of run 355)
red_from: 2026-11-14T04:34Z    # run 257, the first push after T-036 merged at 1a808c9
green_at: null                 # recorded in docs/decisions.md when the first green run on main is read
red_minutes: null              # computed with green_at
origin_task: T-036
fix_task: T-069
---

# INC-001 CI on main red since the merge of T-036

## What happened
Every run of `ci.yml` on main from run 257 (`47135bc`, the first push after T-036 merged at `1a808c9`) failed at
the gate's `scorecard` step: three tests of `ws.path-confinement` added by T-036 re-run themselves under another
identity starting from uid 0, and the hosted runner's user is not uid 0, so they fail there and the scorecard
page counts as stale. The local gate, which runs as uid 0, stayed green, and the queue merged eight further
tasks on it (T-023, T-037, T-038, T-049, T-055, T-056, T-065, T-067). Nobody read CI: the director's check after
a push compared commit ids only. Retrospectives and decision rows that say main was never red describe the local
gate, not CI.

## Timeline (UTC)
| Time | Event |
|---|---|
| 2026-11-14T04:33Z | T-036 merged by the queue at `1a808c9` (commit time 04:24:31Z; the queue's record commit `e42b5e5` at 04:33:18Z; local gate green) |
| 2026-11-14T04:34Z | first push with T-036 on main (`47135bc`, 04:34:26Z); its run 257 fails, and every later run fails the same way |
| 2026-11-15T16:37Z | detected by the planner of ADR-0009 while measuring CI times; confirmed by the director |
| 2026-11-15 | dispatching stopped except for the fix (T-069) |
| see docs/decisions.md | fix merged by the queue; first green run read |

## Cause
A test that needs a property of the machine (uid 0, a checkout another identity can reach) and reports a failure
when it is absent, combined with a gate that runs on one machine and a CI result nobody reads.

## Permanent fix
- T-069: the tests pass as an ordinary identity and as uid 0, in a checkout anywhere, or fail naming the
  precondition.
- Rule (methodology v1.4): after every push the director reads the CI run of the pushed commit and records its
  result; a red run is an incident at once. The queue's statement "main is green" means the local gate and CI.
- ADR-0009: the gate runs on three platforms in CI, so a difference between machines shows on the next push.
```

## Log
### director
<!-- end director log -->
### planner
- 2026-11-15 · R-255 · draft · did: filed from ADR-0009's probe (CI red since run 257) · next: director
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
