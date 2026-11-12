# T-005 join triage (R-015, reviewer)

Branch `task/T-005` at `b03cf88` (join of `f65d7fe` + `62fb9bf`). This is a triage of the join
failures, not a round review (`reviews/T-005-r1.md` is still to be written).

**Summary.** 15 of the 16 failures are not caused by the implementation or the tests. The join
run never executed T-005's code. The `xtask` binary that the tests launch
(`CARGO_BIN_EXE_xtask` = `.worktrees/T-005/target/debug/xtask`) was main's xtask, and cargo
treated it as fresh. Once it is rebuilt from the joined sources, 15 tests pass and 1 fails. That
one is a test defect (AC-5). In the verdict table "no defect" means neither side deviates from
the spec; the failure was an artefact of the build.

## What I ran
All commands ran in `.worktrees/T-005` with
`CARGO_TARGET_DIR=/home/user/photocraft-study-review/.worktrees/T-005/target`.

| # | Command | Result |
|---|---|---|
| 1 | `cargo test -p xtask --test acceptance_t005 -- --nocapture --test-threads=4` (as found) | `0 passed; 16 failed`: the director's result reproduced |
| 2 | `grep -c -a 'main+task' target/debug/xtask` (also tried `front-matter` and `regenerated`) | `0` each. These strings are in `xtask/src/queue.rs` / `main.rs` on this branch |
| 3 | `cat target/debug/deps/xtask-c2793d37e30b396f.d` | lists 17 modules and no `policy.rs`, `precheck.rs` or `seed.rs`: the module list of base `73b00e0` |
| 4 | `stat` on the fingerprint and sources | `.fingerprint/xtask-c2793d37e30b396f/invoked.timestamp` is 23:30:33.878. All `xtask/src/*.rs` were last modified between 22:53:16 and 23:30:33.681 |
| 5 | primary checkout (main `0e21db0`; `git diff 73b00e0 0e21db0 -- xtask Cargo.lock .cargo` is empty): `CARGO_TARGET_DIR=<scratch> cargo build --offline --locked -p xtask --bin xtask`, then `sha256sum` | same fingerprint name `xtask-c2793d37e30b396f`. The binary is **byte-identical** to the stale one (`c45b33da…df13a` for both) |
| 6 | `touch xtask/src/main.rs`, then repeat #1 | `Compiling xtask`. Result: `15 passed; 1 failed` (`allows_acceptance_scorecard_and_review_paths_for_medium_only`) |
| 7 | `cargo test -p xtask` (unit tests) and `cargo test -p xtask --test guard_t005` | `77 passed`; `3 passed` |
| 8 | temporary edit of the one failing case (two variants, see RC-2), then repeat #1 filtered to that test, then `git checkout` of the file | `1 passed` for each variant |

Before #6 I copied the evidence for #3–#5 to a scratch directory outside the repository. I ran
nothing against the real repository with `queue`, `join` or `worktree`. `git status` is clean.
The only effect on the worktree's `target/` is that the xtask artifacts are now rebuilt from the
branch's own sources (see the correction for RC-1).

## 1. Root causes

### RC-1: the tested binary was main's xtask (cargo reused a stale build)
Explains 15 tests, all except `allows_acceptance_scorecard_and_review_paths_for_medium_only`.

**VERIFIED by #2, #3, #5 and #6:**
- The binary that run #1 executed was built from main's sources. Run #5 rebuilt it byte for byte
  from the primary checkout.
- Every failure output matches base behaviour:
  - Records have no `gate_xtask`, `gate_xtask_sha` or `regenerated` field.
  - Base's stdout format is printed (`queue … merged (base …, head …, gate 0.3s)`, base
    `xtask/src/main.rs:205`). The branch's format adds `, gate xtask … @ …`.
  - No refusal checks run.
  - Base's full `cp -a` seed copies the member's `.fingerprint`, `deps` and `incremental` files.
  - Base has no regeneration, so the poisoned case reports `failed_step:"rebase"`.
- After a rebuild, the same 15 tests pass unchanged.

**Why cargo kept it.** This part is INFERRED, but the timestamps support it.
- Cargo's artifact hash for a workspace member depends on its path relative to the workspace
  root, not on the checkout. So the primary checkout and `.worktrees/T-005` produce the same
  artifact name (#5).
- At 23:30:33.878 a build of main's xtask was invoked with this worktree's target. It was
  probably `cargo xtask join T-005` run from the primary checkout while
  `CARGO_TARGET_DIR=.worktrees/T-005/target` was exported. The join record in
  `metrics/tasks.jsonl` (`0e21db0`) says `"at":"2026-11-12T23:30:37Z"`,
  `result:"no_acceptance_tests"`, and the binary was written at 23:30:37.058.
- That build overwrote the implementer's build of the same name.
- Cargo decides freshness by mtime, against the files listed in the dep-info. The rebase in this
  worktree had written every source before 23:30:33.681, which is older than the 23:30:33.878
  invocation. So every later `cargo test`, and every `cargo xtask …`, in this worktree reused
  main's binary.

**Consequence.** The join record's numbers, and any `cargo xtask ci` run in this worktree after
23:30:37, measured main's xtask, not T-005's. The test author's fail-on-base run happened in
`.worktrees/T-005-tests` with its own target (log R-010), so it is not affected.

**Correction (process, for the director):**
- Rebuild before measuring: `cargo clean -p xtask`, or `touch xtask/src/main.rs` (done here).
- Never run `cargo xtask` from one checkout with `CARGO_TARGET_DIR` set to another checkout's
  target.
- Re-run the join measurement and rewrite the join record. The current record is
  `no_acceptance_tests`, with `acc_total:0`.
- PROPOSED: a join or gate procedure that builds the binary under test in a target directory used
  by nothing else, the way `build_main_xtask` in `xtask/src/queue.rs:156` already does for main's
  xtask.

### RC-2: one test's fixture contradicts the allowed-path rule
Explains 1 test, `allows_acceptance_scorecard_and_review_paths_for_medium_only` (AC-5).

VERIFIED: the failure is at `xtask/tests/acceptance_t005.rs:863`, the test's own model check,
before xtask is run (`model: crates/a/tests/acceptance_t008.rs  left: true right: false`).
- The test gives the medium task T-007 `touches: ["crates/a"]` (line 845) and expects
  `crates/a/tests/acceptance_t008.rs` to be refused (line 857).
- The spec allows it: "a path `p` is allowed if, for some entry `t` of main's `touches`,
  `p == t` or `p` starts with `t` followed by `/`".
- The test's own predicate `allowed()` (line 524) and the implementation (`Allowed::allows`,
  `xtask/src/precheck.rs:121`) both say allowed.

AC-5 ("for a medium task `crates/a/tests/acceptance_t007_x.rs` … are allowed, while …
`crates/a/tests/acceptance_t008.rs` are refused") holds only if that medium task's `touches` do
not cover `crates/a`. The test reused the `touches: [crates/a]` of AC-5's previous clause.

Because the model check fails first, the rest of this test never reached xtask on the joined
branch. With a temporary edit (run #8) all of its other cases pass against the implementation:
- expectation flipped to `true`: passes;
- path changed to `crates/b/tests/acceptance_t008.rs` with expectation `false`: passes. This
  variant includes the 3 low-tier refusals.

## 2. Verdict per test
Verdict counts: implementation defect 0 · test defect 1 · spec gap 0 deciding a verdict ·
no defect (RC-1) 15.

| # | Test | AC | Verdict | Deciding spec sentence | Minimal correction |
|---|---|---|---|---|---|
| 1 | `self_weakened_gate_fails_under_mains_gate` | AC-1 | no defect (RC-1) | "→ queue result `gate_failed`, `failed_step` starts with `main:`, `gate_xtask:"main+task"`" | rebuild; passes |
| 2 | `gate_runs_mains_xtask_and_records_it` | AC-2 | no defect (RC-1) | "a task that touches only non-gate paths and whose branch contains an xtask change is refused at `touches`" | rebuild; passes |
| 3 | `refuses_unaccepted_states` | AC-3 | no defect (RC-1) | "for each state other than the accepted ones … queue → refusal `state`; nothing written" | rebuild; passes |
| 4 | `refuses_front_matter_changes_on_task_branch` | AC-4 | no defect (RC-1) | "a task branch that changes its task file's `tier`, `touches` or `state` → refusal `front-matter`" | rebuild; passes. It relies on fixtures where main's copy equals the merge-base copy (see ruling R-a) |
| 5 | `refuses_paths_outside_touches` | AC-5 | no defect (RC-1) | "a change to a path outside `touches` → refusal `touches` naming that path" | rebuild; passes |
| 6 | `allows_acceptance_scorecard_and_review_paths_for_medium_only` | AC-5 | **test defect** (RC-2) | "a path `p` is allowed if, for some entry `t` of main's `touches`, `p == t` or `p` starts with `t` followed by `/`" | test: replace `("crates/a/tests/acceptance_t008.rs", false)` with `("crates/b/tests/acceptance_t008.rs", false)`, or give T-007 touches that do not cover `crates/a`. Spec (planner, optional): state that the medium-task example in AC-5 assumes `touches` that do not cover `crates/a` |
| 7 | `refuses_missing_or_failed_join` | AC-6 | no defect (RC-1) | "medium task with no join record, or only `result:"acceptance_failed"`, or a `joined` record whose `joined_sha` is not an ancestor of the head → refusal `join`" | rebuild; passes |
| 8 | `refuses_missing_rejected_or_stale_review` | AC-7 | no defect (RC-1) | "missing review file, highest round `request-changes` …, `reviewed_sha` not an ancestor, or a code change committed after `reviewed_sha` → refusal `review`" | rebuild; passes |
| 9 | `refuses_high_without_approval_row` | AC-8 | no defect (RC-1) | "high task without an `Approve T-NNN` row in main's `docs/decisions.md` → refusal `approval`; with it, passes" | rebuild; passes |
| 10 | `refusals_change_nothing` | AC-9 (and `task-file` from the check table) | no defect (RC-1) | "`git rev-parse main`, the task branch head, `git status --porcelain` of both checkouts and `metrics/queue.jsonl` are byte-equal before and after" | rebuild; passes |
| 11 | `seed_reuses_non_member_artifacts_only` | AC-12 | no defect (RC-1) | "the seeded target contains no file whose name contains the member crate's name and no `incremental` directory" | rebuild; passes |
| 12 | `generated_only_conflict_is_regenerated_and_merged` | AC-14 | no defect (RC-1) | "→ `merged`, `regenerated:["docs/scorecard.md"]`" | rebuild; passes |
| 13 | `mixed_or_failed_regeneration_conflict_is_rejected` | AC-15 | no defect (RC-1) | "A failing regeneration command → `conflict`, `failed_step:"regenerate:<path>"`" | rebuild; passes |
| 14 | `generated_paths_are_exempt_from_touches_by_mains_list` | AC-16 | no defect (RC-1) | "a task branch that adds a path to its own `xtask/generated.toml` gains no exemption (main's file is read) and is refused at `touches`" | rebuild; passes. It depends on the implementation's TOML layout (ruling R-b) |
| 15 | `main_history_tolerates_only_ledger_commits` | AC-17 | no defect (RC-1) | "a direct main commit that changes `src/` (or `xtask/`, or `Cargo.lock`) → refusal `main-history` naming the commit" | rebuild; passes |
| 16 | `invalid_policy_files_are_refused` | AC-18 | no defect (RC-1) | "missing or invalid `xtask/generated.toml` / `xtask/ledger.toml` on main → refusal `config`" | rebuild; passes. It depends on the implementation's TOML layout (R-b); it does not depend on where `config` sits in the order (R-c) |

Guards (pass, run #7): `seeded_worktree_builds_its_own_source` (AC-10),
`worktree_build_does_not_touch_main_target` (AC-11), `revert_check_leaves_task_target_untouched`
(AC-13).

**Where I am unsure.**
- The exact process that wrote main's xtask into this target at 23:30:33.878 is inferred from
  timestamps and the join record; I did not see the director's shell. The finding itself does
  not depend on it: the binary is main's and is byte-identical to a fresh build of main.
- "15 pass" comes from one run on this machine (5.4 s). I did not test for flakiness.

### Rulings the director needs
None of these decides a verdict above, but the passing tests rely on a reading of each.
- **R-a (front-matter).** The table says "identical to main's". The implementation accepts a head
  copy equal to main's copy **or** to the merge-base copy (`xtask/src/precheck.rs:291–321`). The
  acceptance tests never exercise a main that has diverged; the implementer's unit test does
  (`xtask/src/queue.rs:974`ff., "The director moved main's state after the branch was made").
  Both authors propose "the branch does not change the front matter relative to the merge base".
  Rule on it, and amend the spec text.
- **R-b (policy-file schema).** The schema is unspecified. The implementation uses
  `[[generated]] path, command` and `[[ledger]] path, mode` (see `precheck.rs` unit-test
  constants). The tests copy the shipped files into each toy repository, so AC-16 and AC-18 test
  only edits made to the implementation's own layout. This is a small loss of independence; I
  found no deviation. Name the schema in the spec.
- **R-c (`config` position).** The implementation checks it after `state` and before `touches`.
  The spec does not place it in the order. Fix its position.
- **R-d (guard files and join search path).** These are already accepted into round 2 by D-015.
  Note that this branch itself contains `xtask/tests/guard_t005.rs`, which the new `touches`
  rule would refuse.

## 3. Not tested by either side (acceptance level)
- The gate-path exception for `.cargo/`, `clippy.toml` and `rust-toolchain.toml`. AC-1 exercises
  only `xtask/`. The acceptance tests mention `.cargo/config.toml` and `rust-toolchain.toml` only
  as fixture files.
- `failed_step` prefixed `task:`: main's gate passes and the task's own gate fails under
  `main+task`. No acceptance test reaches this case.
- The director's `Cargo.lock` rule: no package already in main's lock may change version. The
  implementer covers it with unit tests only. Its scope is unspecified: the implementer compares
  only packages that have a `source`.
- A failure to build main's xtask: no specified result. The implementation exits 2 and writes
  no record.
- An extra refusal that is not in the spec: `main-history` also refuses when the last `merged`
  head is not an ancestor of main (`precheck.rs:497`ff.). This is reasonable, but it should be
  stated in the spec.
- `worktree new --no-seed` producing an empty target: asserted only implicitly, as far as I can
  see.
- **The hazard that caused RC-1.** Cargo's checkout-independent artifact hash plus mtime-based
  freshness lets a build from one checkout masquerade as fresh in another checkout's target. The
  seeding rule ("copy only non-member artifacts") prevents this for `worktree new`. Nothing
  prevents it for a shell that points `CARGO_TARGET_DIR` at a worktree, and no test or guard
  covers it. If this is recurring, it is a methodology concern for retrospective 0 (one target
  per source tree, enforced by the tooling, not by habit).
