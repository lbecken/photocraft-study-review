---
id: T-079
title: "Make the gate pass on macOS: register the product tests of facilities macOS lacks, each with its sentence and counterpart, and fix the tests that assume Linux where macOS has the facility"
tier: high              # METHODOLOGY v1.5 §4: gate paths (xtask/platform-tests.toml, xtask/tests/); the approval row must contain `gate paths`. No dependency change, no product code.
state: specified
owner_role: planner
scorecard_items: []     # none claimed; the tests stay tied to their items
depends_on: [T-071]     # the registry's format and the no-skip rule; and both edit xtask/platform-tests.toml
touches:                # one role per path (D-153); all implementer except the last two. Scope of ruling Q-4 B (R-321); Q-4 A adds the list in Interface (W)
  - xtask/platform-tests.toml                              # GATE PATH: the 12 entries of Interface (2)
  - crates/kiln-workspace/src/disk.rs                      # its #[cfg(test)] module only (from line 535): failed_rename_removes_own_temp_and_keeps_target; product code unchanged (T-073)
  - crates/kiln-workspace/tests/acceptance_t019_workspace.rs
  - crates/kiln-workspace/tests/project.rs
  - apps/kiln-cli/tests/acceptance_t004_r2.rs
  - apps/kiln-cli/tests/run.rs
  - apps/kiln-cli/tests/acceptance_t019_cli.rs
  - apps/kiln-cli/tests/guard_t038_cli.rs
  - apps/kiln/tests/acceptance_t055.rs
  - apps/kiln/tests/acceptance_t065.rs
  - apps/kiln/tests/window_t065.rs
  - apps/kiln/tests/guard_t055.rs
  - xtask/tests/acceptance_t079.rs                         # GATE PATH: test author
  - xtask/tests/guard_t079.rs                              # GATE PATH: test author (every Linux test name kept)
budget_tokens: 1100000  # high (v1.5); estimate about 800,000 for Q-4 B, see Estimate
iteration: 2
found_by: planner       # ADR-0009 (R-255); split off T-071 by D-180 Q-1 B; re-specified R-321
origin_task: null
---

# T-079 Make the gate pass on macOS

Re-specified in full under METHODOLOGY v1.5 (§5.7: a draft written under an earlier version) in run R-321,
against `task/T-071` at `b7dd7d621f4a90b4eb7165f6118a0be88fb7dad1` (T-071 joined and green, not merged) read as
main. The Windows half of the R-285 spec is out of this scope unless ruling Q-4 says A (Interface (W)): the probe
of R-321 shows it cannot be evidenced before T-073.

## Goal
On macOS the gate fails at `scorecard` and `test`: in run 44215218474 (`tasks/ci-runs/T-071-round-3.md`, the whole
artefact) the workspace has **1,101 passed, 16 failed**, and the 16 are exactly the product tests of Interface (1);
`xtask` passes there (273 passed, 0 failed). None of the 16 is a product fault: 12 test a facility macOS lacks and
4 assume Linux where macOS has the facility. After this task, on macOS, the 12 are registered platform tests that
do not exist there (each with its sentence and its counterpart or the reason there is none), the 4 pass, and
`gate-macos` is green; on Linux every test name of the base still exists and passes and no assertion changes.
Gate paths: `xtask/platform-tests.toml`, `xtask/tests/acceptance_t079.rs`, `xtask/tests/guard_t079.rs`.

Reading list: METHODOLOGY v1.5 §5.7, §6, §8.1; `docs/development.md` "The gate" (`no-skip`, `scorecard` rows) and
"Platform tests" on `task/T-071`; `xtask/platform-tests.toml` there (8 entries); `tasks/ci-runs/T-071-round-3.md`.

## Interface

### (1) The 16 macOS failures and what becomes of each
Panic locations as printed in the full macOS output of run 43765532298 (R-285) and, for the 10 the log prefix showed, again in runs 44201932507 and 44215218474 (`tasks/ci-runs/T-071-round-1.md`, `-round-3.md`); test fns re-read on `b7dd7d6`: `acceptance_t055.rs:2098`, `acceptance_t004_r2.rs:123`, `run.rs:391`, `acceptance_t019_workspace.rs:202, 408`, `project.rs:106, 357`, `acceptance_t065.rs:274, 299, 335`, `window_t065.rs:109`, `guard_t055.rs:70`, `guard_t038_cli.rs:440` (its `Some(40)` at 88), `acceptance_t019_cli.rs:1026, 1501`, `disk.rs:755` (its `.kiln-tmp-1` assertion at 780).
| Test (file:line of the panic) | Printed cause | Kind | Treatment |
|---|---|---|---|
| `ac8_stdout_write_failure_exits_1` (`apps/kiln/tests/acceptance_t055.rs:329`) | `open /dev/full … NotFound` | facility absent (`/dev/full` is Linux's) | `#[cfg(target_os = "linux")]`, entry `linux`, counterpart pending T-076: the same sentence through a stdout whose reader has closed (a portable way to make a write fail) |
| `run_and_control_exit_1_when_stdout_cannot_be_written` (`apps/kiln-cli/tests/acceptance_t004_r2.rs:124`) | `this test needs /dev/full` | as above | as above |
| `run_exits_1_when_stdout_cannot_be_written` (`apps/kiln-cli/tests/run.rs:396`) | `NotFound` | as above | as above |
| `tree_counts_skipped_names_on_disk` (`crates/kiln-workspace/tests/acceptance_t019_workspace.rs:412`) | `Os { code: 92 … "Illegal byte sequence" }` | facility absent: the macOS file system refuses names that are not UTF-8 (and Windows names are UTF-16) | the non-UTF-8 case only becomes a `linux` test (split out if the test checks other things); entry `linux`, counterpart `none`, reason "no file name on macOS or Windows can be invalid Unicode" |
| `disk_list_dir_reports_entries_by_their_own_kind_and_skips_bad_names` (`acceptance_t019_workspace.rs:219`) | same | same | same |
| `documented_differences_from_git` (`crates/kiln-workspace/tests/project.rs:374`) | same | same | same |
| `disk_list_dir_reports_own_kinds_sorted_and_skips_odd_names` (`project.rs:120`) | same | same | same |
| `ac1_no_display_every_window_form`, `ac1_no_display_exits_2_with_one_line`, `ac1_wayland_only_or_unreachable_display_exits_2` (`apps/kiln/tests/acceptance_t065.rs:65`) | `did not finish within 120s (a window opened, or a hang)` | the specified behaviour is X11's: without `DISPLAY` there is no window on Linux; on macOS a window opens (the `window-probe (macos-15)` job opened one, run 43765532298) | `linux` tests; entries `linux`, counterpart pending T-077 (what "no window" means on macOS and Windows) |
| `without_a_display_the_window_exits_2_with_one_line` (`apps/kiln/tests/window_t065.rs:68`) | `did not finish within 60s` | same | same |
| `kiln_without_subcommand_and_display_exits_nonzero_with_a_message` (`apps/kiln/tests/guard_t055.rs:55`) | `kiln did not finish within 60s` | same | same |
| `disk_refusals_through_control` (`apps/kiln-cli/tests/guard_t038_cli.rs:86`) | `precondition: root/l1 is a link loop: … (os error 62)` against `Some(40)` | test assumes Linux's `ELOOP` number | portable: the expected number is the platform's (chosen in the test by platform; no `cfg` on the test) |
| `deep_trees_stop_at_the_depth_bound` (`apps/kiln-cli/tests/acceptance_t019_cli.rs:1573`) | `mkdir -p of the 2,100-deep chain` | the fixture uses `mkdir -p` beyond the host's path limit (GNU succeeds, macOS's does not) | portable: build the chain one level at a time relative to the last directory; the assertion stays |
| `tree_order_and_counts` (`acceptance_t019_cli.rs:1018`) | the model lists `s/t/z` beside `s/t/Z`; Kiln lists one | the model assumes a case-sensitive file system (macOS's default is not) | portable: the generator does not make names that differ only by case on a file system that folds case (the test detects it), or the model folds them; the assertion against the model stays |
| `disk::tests::failed_rename_removes_own_temp_and_keeps_target` (`crates/kiln-workspace/src/disk.rs:780`) | left `/private/var/…/.kiln-tmp-1`, right `/var/…/.kiln-tmp-1` | the test compares a path below the temp dir, which is a link on macOS | portable: compare canonical paths |
All 16 VERIFIED as printed in the artefact (R-285); the "Kind" column is INFERRED from the message and
the code at the line.


### (2) The 12 entries (T-071's format exactly)
Appended after the 8 entries of T-071; fields exactly `file`, `item`, `platforms`, `doc`, `quote`, `counterpart`,
`reason`, and for `pending` `task` and `expires` (no other key: an unknown key is a parse error of `no-skip`).
`platforms = "linux"` for all 12 (the test exists only on Linux; `unix` would keep it on macOS, where it fails).
- `/dev/full` (3): `counterpart = "pending"`, `task = "T-076"`, `expires` per ruling Q-6, reason naming
  `/dev/full`. `doc`/`quote`: the stdout-failure exit of `docs/control-protocol.md` (e.g. the `run` row, line 213 on
  `b7dd7d6`: "writing stdout failed"; `grep -n "writing stdout failed" docs/control-protocol.md`) and, for
  `ac8_stdout_write_failure_exits_1`, the sentence of `docs/protocol/ui.md` it tests (the implementer quotes it).
- non-UTF-8 names (4): `counterpart = "none"`, reason "no file name on macOS or Windows can be invalid Unicode";
  `doc = "docs/protocol/project.md"`, `quote` e.g. "not valid UTF-8 or contain" (line 26 on `b7dd7d6`).
- no display (5): `counterpart = "pending"`, `task = "T-077"` (ruling Q-2 A, D-199), `expires` per Q-6;
  `doc = "docs/protocol/ui.md"`, `quote` e.g. "If the window system cannot be started it exits 2" (line 67).
The counterpart tasks exist: `tasks/T-076-portable-tests-cli-and-desktop.md`, `tasks/T-077-window-on-macos-and-windows.md`
(`ls tasks/`, R-321). Where a registered test needs a helper used by no other test, the helper gets the same
`cfg(target_os = "linux")` and no twin (a twin with a trivial body is `inert-helper`; an unguarded helper used only
by a Linux test is dead code on macOS and fails clippy's `-D warnings` there).

### (3) Unchanged
No assertion is changed, removed or loosened on Linux; every Linux test name of the base still exists (a split keeps
the original name for the part that stays). No product code; no `docs/` sentence (the macOS sentences go to T-072).

### (W) The Windows half — only if ruling Q-4 is A
In the 27 files and test modules that do not compile for Windows (R-321 probe 2, below), every test that cannot
compile or mean anything on Windows gets `#[cfg(unix)]`, its Unix-only helpers the same cfg and no twin, and an
entry `platforms = "unix"`, `counterpart = "pending"`, `task` T-075 / T-076 / T-073 / T-074, `expires` per Q-6.
Adds to `touches` the other 15 files of the probe's list (Pinned (g)). Evidence: only AC-W (local); no CI run can
show it before T-073, because `kiln-workspace` does not compile there and no test target that depends on it is built.

## Pinned
- (a)–(c): n/a (tests and a registry only); the findings are T-071's `no-skip` kinds.
- (d) Platforms: Linux x86_64, macOS arm64, Windows x86_64 (§8.1).
- (e) Threat model: an entry must not let a weakened test pass: the reviewer reads every entry against its test;
  the guard keeps every Linux test name; `no-skip` refuses an entry whose item's cfg is not `linux`.
- (f) Tests pinning what this changes: scorecard items list test names; a split keeps the listed name on Linux.
- (g) Probe (R-321, scratch `/home/user/kiln-plan-T-079/`, deleted; `git archive b7dd7d6`; stub C compiler as
  ADR-0009 probe a3; `df -h /home/user`: 15 GB free before):
  - probe 1, the branch as it is: `cargo check --workspace --all-targets --locked --keep-going --target
    x86_64-pc-windows-msvc` → exit 101, 29.7 s; **only** `could not compile kiln-workspace (lib) due to 6 previous
    errors` and `… (lib test) due to 34 previous errors`; all 34 error lines in `crates/kiln-workspace/src/disk.rs`;
    no test target of a crate above `kiln-workspace` is checked at all.
  - probe 2, with ADR-0009's probe-only stand-in for `disk.rs`'s 6 product lines (never committed): **225 errors in
    27 files**, all test code: `apps/kiln-cli/tests/` 11 files (93), `apps/kiln/tests/` 4 (14), `crates/kiln-commands`
    2 (7: `src/doc.rs` 4, `tests/acceptance_t037_commands.rs` 3), `crates/kiln-workspace` 10 (111: `src/disk.rs` test
    module 20, `confinement.rs` 24, `acceptance_t008.rs` 20, `acceptance_t036_workspace.rs` 16, `acceptance_t002_r2.rs`
    11, `acceptance_t019_workspace.rs` 9, `project.rs` 6, `guard_t036_removal_workspace.rs` 3, `guard_t008.rs` 1,
    `guard_t036_workspace.rs` 1); **0 in `xtask`** (T-071) and 0 in T-057/T-058/T-059's files. R-261 counted 238 in 33
    files; the 6 `xtask` files are now portable, and T-069/T-071 changed the rest.
- (h) Evaluator: none (no item).
- (i) None.

### The pending CI criterion (v1.5 §8.1), for the approval row
The CI record (`tasks/ci-runs/T-079-round-N.md`, from the artefacts) of the rebased head on `ci/candidate` shows:
- `gate-linux` green; `window-linux` and `window-probe (macos-15)` success (counted jobs);
- **`gate-macos` green**: every gate step PASS or EMPTY; the `scorecard` step's first line
  `macos: 64 items; <m> tests ran; <k> registered absent here; page not compared (computed on Linux)` with no problem
  after it — the 6 problems of round 3 are cleared: `guard_t038_cli::disk_refusals_through_control` (portable fix),
  `acceptance_t065::ac1_no_display_exits_2_with_one_line`, `…_every_window_form`, `…_wayland_only_or_unreachable_display_exits_2`
  (registered `linux`: "registered absent here", which `docs/development.md`'s `scorecard` row says is not a problem),
  `acceptance_t019_cli::tree_order_and_counts`, `…::deep_trees_stop_at_the_depth_bound` (portable fixes); the full test
  output 0 failed, and none of the 12 registered names in it;
- `gate-windows` red at `kiln-workspace` (lib), as before (T-073); its record shows the `core-crates` step unchanged
  (`xtask` and the three core crates) — T-079 has no part to show there under Q-4 B.
After the merge, the first green `gate-macos` run on main is recorded: from then on it counts (§8.1, ADR-0009 Q-1 A).

## Acceptance criteria
Gate tests by the independent test author (`xtask/tests/acceptance_t079.rs`), reading the repository's files:
- AC-1: the set of entries of `xtask/platform-tests.toml` equals exactly T-071's 8 plus the 12 `(file, item,
  platforms = "linux")` of Interface (1)/(2), each with the `counterpart` and `task` stated there; fails on base (8
  entries). Must reject, among others: one of the 12 with `platforms = "unix"` (outside the stated set).
- AC-2: for the 4 portable fixes, the set of platform-cfg'd items in their test fns equals ∅ (the four tests have no
  `cfg` and no platform branch of their own); passes on base, so it is a guard of intent, written in `guard_t079.rs`.
  Must reject a `#[cfg(target_os = "linux")]` on `tree_order_and_counts`.
- Guard (`guard_t079.rs`): the set of Linux test names (`cargo test --workspace -- --list`, names) of the head
  contains every name of the base's set.
Not duplicated: that each registered item is behind exactly `cfg(target_os = "linux")` and has an existing sentence
is `no-skip`'s (`platform-mismatch`, `platform-sentence`, `platform-unlisted`), which the gate runs; AC-1 does not
re-check it. R-285's scans AC-1 to AC-3 (`std::os::unix`, `/dev/full`, raw errno literals) are dropped: the first is
the Windows half (Q-4), the second follows from AC-1 + `no-skip`, the third is shown only by macOS CI (on Linux the
literal 40 is right).
Through CI (pending criterion above; the reviewer claims only what the record shows):
- AC-3: `gate-macos` green on the candidate, as stated.
- AC-W (only with Q-4 A; local, implementer and reviewer): probe 2's command on the head reports 0 errors in test
  code (225 on `b7dd7d6`).

## Out of scope and implementer notes
- Windows marking (Q-4 B: a new task after T-073, below), Windows counterparts (T-073 to T-076), the window on macOS/Windows (T-077),
  macOS documents (T-072).
- `acceptance_t002.rs` (T-071's), `acceptance_t015_tools.rs` (fails off Unix, naming why).
- Never mark a test that means the same on macOS; never change an expected value on Linux.
- Each CI round is a `CI-REQUEST` (§8.1) and counts as a gate attempt.

## Collisions and order (§5.7 re-check rule)
- After T-071 (both edit `xtask/platform-tests.toml`). In flight now: only T-071.
- T-091 (draft; `xtask/src/platforms.rs`, `noskip.rs`, `xtask/tests/acceptance_t091.rs`, `guard_t071.rs`,
  `docs/development.md`): no shared path, may be in flight together; if T-091 changes what `no-skip` reports, this
  spec is re-checked when it merges.
- T-072 (`disk.rs`, `docs/protocol/paths.md`, `ui.md`): shares `disk.rs` → after this task, re-checked when it merges.
- T-073, T-074 (`disk.rs`): after; T-075 (`crates/kiln-workspace/tests/`), T-076 (`apps/kiln-cli/tests/`, four
  `apps/kiln/tests/` files): directory overlaps → after; each re-checked when this merges.
- Cut-2 tests (`acceptance_t057/58/59.rs`, `project_t057.rs`, `tabs_t058.rs`, `status_t059.rs`) are not in `touches`
  and pass on macOS (round 3: none of them fails).

## Estimate
T-071: 1,215,738 tokens booked against the planner's estimate of about 800,000 (×1.5; its CI rounds and a reviewer
finding holes in a checker). This task under Q-4 B: 12 markings with entries, 4 portable fixes, two scan/guard tests,
1–2 CI rounds; about 550,000 by the stage's figures, **about 800,000** with T-071's overrun factor, inside the high
ceiling 1,100,000. Under Q-4 A add the 27-file marking: about 1,300,000, over the ceiling. Arm: high → independent
test author.

## Questions and rulings
- **Q-1 (planner, R-285). T-072 after this task.** A: T-072 shrinks to the macOS document sentences (non-UTF-8
  names impossible, a window opens without `DISPLAY`, case folding) and what `ci/T-079` still shows. B: T-072
  takes the four portable fixes of Interface (1) instead. **Recommendation: A** — the fixes are in files this
  task already holds; B puts the same files in two tasks.
- **Q-2 (planner, R-285). The no-display tests' counterpart.** A: pending T-077. B: `none` (macOS always has a
  window system). **Recommendation: A** — "without a window system, exit 2" still has a macOS and a Windows
  meaning (no session, a headless runner) for T-077 to state.
- **Q-3 (planner, R-285). AC-1 to AC-3 as scans.** They check the repository, not Kiln's behaviour. A: keep
  them (they fail on base and are mechanical). B: rely on AC-5 and AC-6 only. **Recommendation: A** — AC-5 and
  AC-6 are not gate tests; without the scans a later Unix-only test passes the local gate.

### Spec check R-285
Written against main `cbb54a2628afbd368087e82c5d7925eaa5b08625`. Evidence: the `gate-macos` artefact of run
43765532298 (whole test output, downloaded with the Actions tools, unpacked under `/home/user/kiln-plan-T-071/`,
deleted); `tasks/ci-runs/T-070-round-2.md` for Windows; R-261's Windows check for the 33 files. Files added
beyond R-261's list: `acceptance_t004_r2.rs`, `run.rs`, `guard_t055.rs` (macOS only). Not run: no build.

### Rulings on spec R-285 (director, D-199)
- **Q-1: A.** T-072 shrinks to the macOS document sentences, the report-step limits noted in its file, and whatever
  `ci/T-079` still shows red on macOS; the portable fixes stay here.
- **Q-2: A.** The no-display tests' counterpart is pending T-077.
- **Q-3: A.** The repository scans stay as acceptance tests.
- Order: T-054, T-071, then this task (after T-071's registry exists), then T-072, T-073.

### Spec check R-321
Against `task/T-071` @ `b7dd7d6` (as main), methodology v1.5. Changed:
- **Re-specified in full** (§5.7: written under v1.4). Scope narrowed to macOS (Q-4, recommendation B); title,
  Goal, `touches` (33 → 14 paths), criteria rewritten.
- **Counts re-measured:** "238 errors in 33 files" is now 225 in 27 (probe 2) and unmeasurable without the stand-in
  (probe 1); the 12 facility tests and 4 portable fixes re-read on the branch (test fn lines in Interface (1)).
- **Registry:** no `future` field (T-071 p2); `pending` names `task` and `expires`; `doc` a tracked `.md` under
  `docs/` (s9); counterpart tasks T-076, T-077 exist.
- **Criteria as equalities** with an outside variant each; R-285's scans dropped where `no-skip` or CI already shows
  the case (stated under Acceptance criteria).
- **Pending CI criterion** written for the approval row; each of round 3's 6 scorecard problems mapped to its fix.
- Not run: any CI; a macOS build (not possible here).

### Questions (planner, R-321)
- **Q-4. The Windows half.** A: keep it in T-079, evidenced only by AC-W, a local check with a probe-only stand-in
  (probe 2 measured it: 225 errors in 27 files on `b7dd7d6`); no CI can show it before T-073 (probe 1); the marking
  may be reshaped by T-073/T-074, which change the same tests' Windows meaning. B: T-079 is macOS only; the Windows
  marking becomes a new high task (the next free number, T-092 on main 3c62974) after T-073 (when Windows CI can compile the tests and show it), absorbing
  the registry work T-075/T-076 cannot do as low tasks. **Recommendation: B** (measured: probe 1 shows CI cannot
  evidence it; A would also take the task over the ceiling).
- **Q-5. AC-2 as a guard.** It passes on base. A: keep it in `guard_t079.rs` (§6: a criterion that holds on base is a
  guard). B: drop it (CI shows the fixes). **Recommendation: A** — cheap, and it stops a "fix" by `cfg`.
- **Q-6. Expiry of `pending`.** The 8 `pending` entries (3 → T-076, 5 → T-077) expire 2026-12-31 (D-180 Q-2 A).
  The order T-079, T-072, T-073, T-074, T-075, T-076, T-077 is five high tasks before T-076/T-077 at about 1–3 days
  each by this stage's record. A: keep 2026-12-31 and move it by a ruling if they slip. B: 2027-01-31.
  **Recommendation: A** — an expiry is meant to press; a slip is a decision row.
- **Q-7. Dispatch.** T-079 can be dispatched when T-071 merges, if the merge equals `b7dd7d6` in the paths above
  (else a short re-check). **Recommendation:** dispatch then.

### Rulings on spec check R-321 (director, D-233)
- **Q-4: B.** T-079 is the macOS half only ("the macOS gate green"). The Windows marking of the product tests
  becomes T-092 (draft), after T-073, when Windows CI can compile those tests: before that nothing of it can be
  shown (the planner's probe 1: the Windows check stops at `kiln-workspace`'s 40 errors and checks no test target
  above it).
- **Q-5: A.** AC-2 passes on base and is a guard.
- **Q-6: A.** `pending` entries expire on 2026-12-31; the date moves by a ruling if T-076 or T-077 slip.
- **Q-7: A.** Dispatch when T-071 has merged, if the merge equals `b7dd7d6` in this task's paths.

### Ruling from T-071's queue refusal (director, D-235; changes this spec before dispatch)
T-071 merges without the eight registry entries of `xtask`'s own tests and without their cfg lines (main's old
tool rejects a tree that uses the rule it does not know yet). **This task adds them**, exactly as reviewed on
branch `park/T-071-reviewed` (`8cb8464b9825ef2467ccc1ee14f8b95e40bb81b1`: `xtask/platform-tests.toml` and the cfg
lines in `xtask/src/queue.rs`, `publish.rs`, `worktree.rs` test modules, `xtask/tests/acceptance_t013_publish.rs`,
`guard_t005.rs`, `acceptance_t005_r3.rs`, `acceptance_t005_r4.rs`), beside its own twelve. Those paths join
`touches` (implementer), and T-071's AC-5 (`cargo check -p xtask --all-targets --locked --target
x86_64-pc-windows-msvc`: 0 errors) and AC-6 (`xtask` exit status 0 on `gate-macos` and `gate-windows`) join this
task's criteria and its pending CI criterion. The planner re-checks this spec against main after T-071 merges
(v1.5 change 10) and writes these in.

## Log
### director
<!-- end director log -->
### planner
- 2026-11-15 · R-285 · draft · did: full spec from the split of T-071 (D-180 Q-1 B) · not run: builds · next: director
- 2026-11-18 · R-321 · specified · did: re-specified in full under v1.5 against task/T-071 @ b7dd7d6; Windows check probed twice (scratch, deleted) · not run: CI · next: director (Q-4 to Q-8)
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
