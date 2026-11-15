---
id: T-069
title: "Make the gate pass in CI again: the confinement tests that change identity pass, or fail with their reason, whatever the identity and location of the checkout"
tier: low               # ADR-0002: tests and documentation only, no behaviour change; scorecard_items empty (the tests stay tied to ws.path-confinement, which is not claimed). See ADR-0009 Q-7: a reviewer reads the diff anyway
state: self_verified
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
  - reviews/T-069-r1.md                                  # reviewer: the read of the diff asked for by ADR-0009 Q-7 (a low-tier branch may not otherwise carry a review file; D-179)
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

## Ruling on the implementer's question of R-256 (director, D-177)
- **c1.** The cause stated in D-175 and in the incident text above was an inference and was wrong. Reproduced by
  the implementer as uid 1000: the three tests panic in their own setup, where the fixture snapshot reads a control
  file of mode 0000 that only uid 0 can read; the identity change is not what fails. The incident file states the
  reproduced cause (the implementer corrects three sentences of it, run R-258); the text above stays as the record
  of what was first ruled.

## Log
### director
<!-- end director log -->
### planner
- 2026-11-15 · R-255 · draft · did: filed from ADR-0009's probe (CI red since run 257) · next: director
<!-- end planner log -->
### implementer
- 2026-11-15 · R-256 · ready · rulings of D-175 acknowledged and accepted (tier low with a
  reviewer's read; oracle pinned: assertions unchanged, only how the unreadable root is reached;
  other uid-0 tests in scope only if they fail as an ordinary identity; T-052 abandoned; INC-001
  text copied byte for byte from the ruling's block).
  ACs restated: (1) as uid 0 and as an ordinary user, in a checkout both can reach, the five tests
  (and `confinement::root_that_cannot_be_opened_for_reading_is_refused_by_new`, same helper) pass
  and each fails under a mutant of `disk.rs` that removes the refusal it guards; (2) as uid 0 in
  a checkout below a root-owned 0700 directory, each identity-changing test fails with a
  `precondition:` message naming that directory; (3) CI green after the merge (director).
  **Cause, reproduced.** Ordinary identity here: `setpriv --reuid 1000 --regid 1000
  --clear-groups --inh-caps=-all --bounding-set=-all` with a copy of the 1.97.0 toolchain and the
  cargo registry, in a clone of this branch owned by uid 1000 under `/home/user/kiln-t069-scratch/`
  with its own `target/` (this worktree and `/root` are not writable/reachable for uid 1000; the
  scratch directory is deleted at the end). On base `e9d4b4d`: `cargo xtask scorecard --check`
  as uid 1000 FAILS with exactly the CI result: the three tests named in the Goal fail and
  `docs/scorecard.md is stale`; `cargo test --workspace --locked --no-fail-fast` as uid 1000: 975
  ok, 3 FAILED, the same three, and no other test. The panic in each is not the identity change
  (as an ordinary user the tests already ran as their own identity) but the parent's fixture
  snapshot: `fs::read(..).unwrap()` of the mode-0000 control file `open/locked.txt`, which only
  uid 0 can read (`acceptance_t036_workspace.rs:167`, `acceptance_t036_cli.rs:172`,
  `guard_t036_removal_cli.rs:192` at base: `PermissionDenied`). As uid 0 on base, all six files
  pass; with the scratch checkout's top directory made root-owned 0700, as uid 0 six tests fail
  (the five + the confinement one) with `precondition: cannot spawn …: Permission denied`, which
  names no path.
  **Changed** (tests only, `touches`): the three T-036 files: the snapshot reads a file it gets
  `PermissionDenied` on with the owner's read bit added for that read and the mode restored at
  once (`read_bytes`), so both identities compare the same bytes; all five files and
  `confinement.rs`: before a child runs as 65534 (uid 0 only), `assert_reachable` checks that
  65534 can search every directory down to the binary it runs and to the fixture (mode bits),
  and fails with `precondition: … <path> (mode 0700, owner uid 0, …) does not let uid 65534
  search it …`. No assertion changed; no skip, `#[ignore]` or `cfg`. `docs/development.md`: one
  paragraph under CI. `incidents/INC-001-ci-red-since-t036.md`: created with the ruling's text.
  **Results at `3f5cff6`** (test change): uid 0, worktree, the six files: 1+6+1+3+3+2 tests ok.
  uid 1000: `cargo test --workspace --locked --no-fail-fast` 978 ok, 0 failed; `cargo xtask ci
  --keep-going` GATE PASSED (scorecard PASS 64 items, 44 verified, 0 failing; test PASS 979
  passed). AC-2, uid 0, checkout below root 0700: the same six tests fail, each with the
  precondition message naming `/home/user/kiln-t069-scratch (mode 0700, owner uid 0, group gid
  0)`; the other tests in those binaries pass.
  **Tests outside the five that fail as an ordinary identity:** none (only the three T-036 tests
  failed as uid 1000 on base; the two T-037 tests and `confinement` failed only in the
  unreachable checkout as uid 0).
  **Mutants** of `crates/kiln-workspace/src/disk.rs`, each applied, run (the six files) and
  reverted with `git checkout -- crates/kiln-workspace/src/disk.rs`, under both identities with
  identical results; `git status` clean after each set: M1 (`DiskFs::new` holds `/` when the root
  cannot be opened: refusal at start removed) → `unreadable_root_is_refused_by_new`,
  `unreadable_root_is_refused_at_start`, `confinement::root_that_cannot…` FAIL; M2 (held-root
  identity not compared in `current_root`) → `root_made_unreadable_after_start_stays_protected…`
  FAIL (with the other two guard_t036 cases, `reopen_after_the_root_moved_is_refused`,
  `confinement::root_that_cannot…`); M3 (`check_location` accepts a walk failing with
  `PermissionDenied`) → `reopen_whose_walk_fails_inside_is_io_error` FAIL in its "as U" cases
  AC-4 (ii) and (iii); M4 (`check_location` opens the file) → `reopen_does_not_need_the_file`
  FAIL in "as U: a.txt made mode 0000".
  **Gate:** attempt 1, uid 0, worktree, at `4c96de1`: GATE PASSED (979 passed; scorecard 44
  verified, 0 failing). Ordinary identity: `cargo xtask ci --keep-going` as uid 1000 at
  `3f5cff6` (above) and again at the log commit (last line of this entry).
  QUESTION: the incident text (copied as ruled) says the tests fail because they "re-run
  themselves under another identity starting from uid 0"; the reproduced mechanism is that their
  parent reads a mode-0000 control file only uid 0 can read (the identity branch for an ordinary
  user already existed). Should the record of D-175 or INC-001 be corrected? Not changed here.
  · not run: the CI runner itself (the director reads the run after the merge); ACLs or
  filesystems other than ext4-like mode bits; TMPDIR set to an unreachable directory as uid 0 is
  covered by the fixture check but was not exercised. · missing context: none. · next: reviewer
  (diff read and the AC-1 mutant), director.
  Last line: at `85ea36a`, as uid 1000, `cargo xtask ci --keep-going`: GATE PASSED (scorecard PASS,
  64 items, 44 verified, 0 failing; test PASS, 979 passed, 0 failed); gate attempt 2, uid 0,
  worktree, at the commit of this line: see the commit after it (PASS expected; reported to
  the director). Scratch checkout `/home/user/kiln-t069-scratch/` deleted.
- 2026-11-15 · R-258 · ready · ruling c1 (D-177) acknowledged and accepted: INC-001 now states
  the cause as reproduced in R-256 ("What happened", "Cause", first bullet of "Permanent fix";
  the rest as ruled). Figures checked against my R-256 record: on base as uid 1000, 975 tests
  ok and the same three FAILED, each a PermissionDenied unwrap reading `open/locked.txt`; kept
  as given. Correction to R-256: gate attempt 2 (uid 0, worktree) at `e727698` PASSED (979
  tests passed; scorecard 64 items, 0 failing). This run: `cargo xtask hygiene` only; the whole
  gate not re-run (change limited to the incident file and this log). · next: director.
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
- 2026-11-15 · R-259 · round 1 · approve · rulings D-175 (branch) and c1 / D-177 (main's task
  file) acknowledged and accepted. INC-001 states the cause I reproduced, and D-177 records the
  corrected one. Read of the diff `e9d4b4d..673dbe0`:
  - Only additions, plus the `snapshot` line in the three T-036 files. No assertion changed,
    no skip, `#[ignore]` or `cfg` added.
  - `read_bytes` restores the mode before it unwraps and runs only while no request is in
    flight.
  - `assert_reachable` runs as uid 0 only and can add failures, never passes.
  Ran the six test files:
  - uid 0 in the worktree: all pass.
  - Review clone with its own target, test binaries run as uid 0 and as uid 1000
    (`setpriv --reuid 1000 --regid 1000 --clear-groups --inh-caps=-all --bounding-set=-all`).
    Base as 1000: the three T-036 tests FAIL with PermissionDenied at their snapshot
    (:167/:172/:192). Head as 1000 and as 0: all pass.
  - Head as 0, clone dir chmod 0700 root: the six identity-changing tests fail with
    `precondition:` naming that directory.
  Mutants under both identities, identical results:
  - M1 (disk.rs `DiskFs::new` refusal removed): killed by the three at-start tests.
  - M2 (`current_root` identity check removed): killed by guard_t036 (iii).
  Restored, `git status` clean. Docs paragraph and INC-001 checked (commit ids, the eight
  merges, cause). Finding 1 is a nit: canonical-path and ACL limits of `assert_reachable`;
  it fails rather than passes. Follow-ups: confinement leaves `k8i-*` fixtures in `temp_dir`;
  machines that still fail (Windows, rootless uid 0, macOS root). Review in
  `reviews/T-069-r1.md`. · not run: CI runs; whole gate as uid 1000 · next: director.
<!-- end reviewer log -->
