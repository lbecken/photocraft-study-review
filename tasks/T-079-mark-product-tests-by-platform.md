---
id: T-079
title: "Make the gate pass on macOS: register the product tests of facilities macOS lacks, each with its sentence and counterpart, and fix the tests that assume Linux where macOS has the facility"
tier: high              # METHODOLOGY v1.5 §4: gate paths (xtask/platform-tests.toml, xtask/tests/); the approval row must contain `gate paths`. No dependency change, no product code.
state: queued
owner_role: planner
scorecard_items: []     # none claimed; the tests stay tied to their items
depends_on: [T-071]     # the registry's format and the no-skip rule; and both edit xtask/platform-tests.toml
touches:                # one role per path (D-153); all implementer except the last two. Scope of ruling Q-4 B (R-321); Q-4 A adds the list in Interface (W)
  - xtask/platform-tests.toml                              # GATE PATH: the 8 restored entries (Interface (2a), the patch's hunk) and the 12 of Interface (2)
  - xtask/src/queue.rs                                     # GATE PATH: its #[cfg(test)] module, exactly the patch's hunk (2 tests)
  - xtask/src/publish.rs                                   # GATE PATH: its #[cfg(test)] module, exactly the patch's hunk
  - xtask/src/worktree.rs                                  # GATE PATH: its #[cfg(test)] module, exactly the patch's hunk
  - xtask/tests/acceptance_t013_publish.rs                 # GATE PATH: exactly the patch's hunk
  - xtask/tests/guard_t005.rs                              # GATE PATH: exactly the patch's hunk
  - xtask/tests/acceptance_t005_r3.rs                      # GATE PATH: exactly the patch's hunk
  - xtask/tests/acceptance_t005_r4.rs                      # GATE PATH: exactly the patch's hunk
  - docs/development.md                                    # exactly the sentence "The registry has no entry yet: …" (lines 101–102 on e95e7c3), replaced (Pinned (i))
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
budget_tokens: 1100000  # high (v1.5); estimate about 850,000 (R-325), see Estimate
iteration: 2
found_by: planner       # ADR-0009 (R-255); split off T-071 by D-180 Q-1 B; re-specified R-321
origin_task: null
---

# T-079 Make the gate pass on macOS

Re-specified in full under METHODOLOGY v1.5 (§5.7: a draft written under an earlier version) in run R-321,
against `task/T-071` at `b7dd7d621f4a90b4eb7165f6118a0be88fb7dad1` (T-071 joined and green, not merged) read as
main. The Windows half of the R-285 spec is out of this scope (ruling Q-4 B, D-233: it is T-092). Re-checked in
run R-325 against main `e95e7c3` after T-071 merged without its eight registry entries (D-235): this task restores
them (Interface (2a)).

## Goal
On macOS the gate fails at `scorecard` and `test`: in run 44215218474 (`tasks/ci-runs/T-071-round-3.md`, the whole
artefact) the workspace has **1,101 passed, 16 failed**, and the 16 are exactly the product tests of Interface (1);
`xtask` passes there (273 passed, 0 failed). None of the 16 is a product fault: 12 test a facility macOS lacks and
4 assume Linux where macOS has the facility. After this task, on macOS, the 12 are registered platform tests that
do not exist there (each with its sentence and its counterpart or the reason there is none), the 4 pass, and
`gate-macos` is green; on Linux every test name of the base still exists and passes and no assertion changes.
Second part (D-235): T-071 merged the rule and the tool without the eight entries of `xtask`'s own tests and
without their cfg lines, so on main `xtask`'s tests again do not compile for Windows and five fail on macOS. This
task applies `tasks/T-079-restore-from-T-071.patch` (the reviewed diff of those eight paths; `reviews/T-071-r4.md`
has the reviewer's table) unchanged, so that `xtask` passes on all three again (T-071's AC-5 and AC-6, D-235).
Gate paths: `xtask/platform-tests.toml`, the seven `xtask` files of the patch, `xtask/tests/acceptance_t079.rs`,
`xtask/tests/guard_t079.rs`.

Reading list: METHODOLOGY v1.5 §5.7, §6, §8.1; `docs/development.md` "The gate" (`no-skip`, `scorecard` rows) and
"Platform tests" on main; `tasks/T-079-restore-from-T-071.patch`; `reviews/T-071-r4.md` (the eight);
`tasks/ci-runs/T-071-round-3.md` and `-round-4.md`.

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

### (2a) The 8 restored entries (from the patch; `counterpart = "none"` for all, reason as in the patch)
| file :: item | platforms | doc / quote (occurs on main, R-325) |
|---|---|---|
| `xtask/src/queue.rs` :: `tests::basic_refusals_write_nothing` | linux | `docs/development.md` / "a lock whose process is gone is reclaimed" |
| `xtask/src/worktree.rs` :: `tests::create_seed_list_remove` | linux | same / "deletes the branch only if merged" |
| `xtask/src/publish.rs` :: `tests::a_failed_content_commit_in_step_four_exits_one` | unix | same / "a failed content commit in step 4 included" |
| `xtask/tests/acceptance_t013_publish.rs` :: `publish_refuses_while_the_queue_lock_is_held_and_releases_it_for_the_queue` | linux | same / "a lock held by a live process is refused" |
| `xtask/tests/guard_t005.rs` :: `worktree_build_does_not_touch_main_target` | unix | same / "never hard links, symlinks or a shared directory" |
| `xtask/tests/acceptance_t005_r3.rs` :: `tampered_cached_gate_binary_does_not_decide_the_gate` | linux | same / "so a binary left by an earlier run (or overwritten by task code during an earlier gate) is never trusted" |
| `xtask/tests/acceptance_t005_r4.rs` :: `planted_future_dated_xtask_does_not_decide_the_gate` | linux | same / same |
| `xtask/src/queue.rs` :: `tests::main_xtask_is_rebuilt_from_base_on_every_run` | unix | same / same |
The patch is applied as it is (`git apply`); nothing else in those seven `.rs` files changes. Its first hunk also
replaces the registry header's "No entry yet (D-235 u1) …" comment. The 12 entries of (2) follow the 8.

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
- (i) `docs/development.md`, "Platform tests": the sentence "The registry has no entry yet: the tests that need one
  are marked and listed by T-079 (D-235)." becomes, worded by the implementer: the registry lists the platform tests
  of `xtask` (the director's tools, Linux or Unix only) and of the product tests that test a facility macOS lacks;
  the Windows marking of the product tests is T-092's. **Unchecked** (the reviewer's first check: that each clause
  matches the registry at the head, e.g. `grep -c '^\[\[test\]\]' xtask/platform-tests.toml` = 20).
- (j) **Main's gate on the candidate tree** (D-235's rule). Main's tool is T-071's (`no-skip` reads the registry), so
  the queue's first gate (main's `xtask`) and the task's own gate are the same tool for every step this task touches:
  the task changes no file under `xtask/src/` outside test modules, no fixture, no `noskip.rs`/`platforms.rs`/
  `scorecard.rs`, no workflow. Measured (R-325, scratch export of main `e95e7c3` with the patch applied, main's own
  `cargo xtask no-skip`): `PASS  no-skip: self-test ok (17 known findings); 197 files scanned, 0 allowlisted skip(s),
  8 platform test(s) registered (0 pending), today = 2026-11-18` (exit 0); on main without the patch the same line
  with `0 platform test(s) registered`. **Before the join** the implementer runs, in a scratch export of main with the
  branch's diff applied, main's `cargo xtask ci` (or at least `no-skip` and `scorecard --check`) built from main's
  sources, and logs the `no-skip` detail line (it must say `20 platform test(s) registered`); the director's join
  and the queue then repeat what that showed.

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
- `gate-macos`'s `core-crates` step `==== xtask: exit status 0` (the 5 macOS-failing `xtask` tests absent by name:
  the 5 `linux` entries of (2a));
- `gate-windows` red at `kiln-workspace` (lib), as before (T-073), **and** its `core-crates` step
  `==== xtask: exit status 0`, every `xtask` binary run (21 on round 4), and the 8 restored tests absent by name — as
  `tasks/ci-runs/T-071-round-4.md` showed for the same tree (270 passed, 0 failed).
After the merge, the first green `gate-macos` run on main is recorded: from then on it counts (§8.1, ADR-0009 Q-1 A).

## Acceptance criteria
Gate tests by the independent test author (`xtask/tests/acceptance_t079.rs`), reading the repository's files:
- AC-1: the set of `(file, item, platforms, counterpart, task)` of `xtask/platform-tests.toml`'s entries equals
  exactly the 20 of Interface (2a) and (2): the 8 restored ones and the 12; fails on base (main has 0 entries). Must
  reject, among others: one of the 12 with `platforms = "unix"`, and a 21st entry (both outside the stated set).
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
Local, by the implementer and re-run by the reviewer (T-071's AC-5, D-235):
- AC-4: `cargo check -p xtask --all-targets --locked --target x86_64-pc-windows-msvc` at the head reports 0 errors
  (needs no C stub: `xtask` does not depend on `tree-sitter`). Fails on base: R-325 measured 7 `xtask/…: error` lines
  on main `e95e7c3` and exit 0 with the patch applied.
- Main's gate on the candidate tree (Pinned (j)): logged before the join.
Through CI (pending criterion above; the reviewer claims only what the record shows):
- AC-3: `gate-macos` green on the candidate, as stated.
- AC-5 (T-071's AC-6): `==== xtask: exit status 0` on `gate-macos` and on `gate-windows`, the 8 absent by name there
  as their `platforms` say. Fails on base (main: `xtask` does not compile for Windows; five fail on macOS, D-235).

## Out of scope and implementer notes
- Windows marking (Q-4 B: a new task after T-073, below), Windows counterparts (T-073 to T-076), the window on macOS/Windows (T-077),
  macOS documents (T-072).
- `acceptance_t002.rs` (T-071's), `acceptance_t015_tools.rs` (fails off Unix, naming why).
- Never mark a test that means the same on macOS; never change an expected value on Linux.
- Each CI round is a `CI-REQUEST` (§8.1) and counts as a gate attempt.

## Collisions and order (§5.7 re-check rule)
- T-071 merged (`0f8ea64`). In flight on `e95e7c3`: none of the drafts below.
- `docs/development.md` and `xtask/platform-tests.toml` are also in the `touches` of the drafts T-043 (also
  `queue.rs`, `publish.rs`, `acceptance_t005_r3/r4.rs`), T-089, T-090, T-091 (`docs/development.md` only) and T-092
  (`xtask/platform-tests.toml`): none of them is in flight with this task (§5.2); each is re-checked when this merges.
- T-072 (`disk.rs`, `docs/protocol/paths.md`, `ui.md`): shares `disk.rs` → after this task, re-checked when it merges.
- T-073, T-074 (`disk.rs`): after; T-075 (`crates/kiln-workspace/tests/`), T-076 (`apps/kiln-cli/tests/`, four
  `apps/kiln/tests/` files): directory overlaps → after; each re-checked when this merges.
- Cut-2 tests (`acceptance_t057/58/59.rs`, `project_t057.rs`, `tabs_t058.rs`, `status_t059.rs`) are not in `touches`
  and pass on macOS (round 3: none of them fails).

## Estimate
T-071: 1,215,738 tokens booked against the planner's estimate of about 800,000 (×1.5; its CI rounds and a reviewer
finding holes in a checker). This task: 12 markings with entries, 4 portable fixes, two scan/guard tests, 1–2 CI
rounds: about 550,000 by the stage's figures, about 800,000 with T-071's factor. The eight add little building (a
reviewed patch applied, `git apply --check` passes on `e95e7c3`) but a reviewer's check of 7 gate-path files against
`reviews/T-071-r4.md` and the main-gate run of Pinned (j): about **+50,000**. **About 850,000**, inside the high
ceiling 1,100,000. Arm: high → independent test author.

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

### Spec check R-325
Against main `e95e7c3` (T-071 merged at `0f8ea64` without its entries, D-235). Changed: `touches` gains the seven
`xtask` files of the patch (exactly its hunks) and one sentence of `docs/development.md`; Goal and Interface (2a) the
eight; AC-1 now 20 entries; AC-4 (T-071's AC-5) and AC-5 (T-071's AC-6) added, each failing on base; the pending CI
criterion gains `xtask` exit status 0 on macOS and Windows; Pinned (i) the replaced sentence, (j) main's gate on the
candidate tree; Collisions and Estimate. Evidence (scratch `/home/user/kiln-plan-T-079/`, an export of `e95e7c3` as
a git repository, deleted; `df -h /home/user`: 23 GB free): `git apply --check` of the patch passes; main's `cargo
xtask no-skip` passes on main + patch (8 registered) and on main (0); `cargo check -p xtask --all-targets --locked
--target x86_64-pc-windows-msvc`: exit 0 with the patch, 7 error lines without. The nine `doc`/`quote` pairs (the 8's
six quotes and the 12's three) occur in tracked `.md` files on main (whitespace-collapsed substring test). The 12 and
the 4 fixes are unchanged by T-071's merge (their files are not in the patch). Not run: the 12 (not written yet), CI.

- **Q-8 (planner, R-325). One patch or two commits?** A: the implementer applies the patch as its first commit
  (`git apply`), then the twelve — the reviewer then compares that commit with `park/T-071-reviewed` path by path.
  B: interleaved. **Recommendation: A** — the reviewed eight stay identifiable.

### Ruling on spec check R-325 (director, D-239)
- **Q-8: A.** The patch `tasks/T-079-restore-from-T-071.patch` is the implementer's first commit, unchanged
  (`git apply`), then the twelve entries and the four portable fixes in later commits, so the reviewer compares
  that first commit path by path with the head reviewed in T-071.
- Pinned (j) stands as written: before the join the implementer runs main's gate (built from main's sources) on
  the candidate tree and logs its `no-skip` line (`20 platform test(s) registered`).

### Rulings on the test author's report (director, D-241; told to both builders)
- **w1 (question Q-T1: the item of a non-UTF-8 entry when its test is split).** For each of the four non-UTF-8
  tests the registered item is exactly one of two names: the test's name as Interface (1) gives it (the test is
  not split and is marked as a whole), or that name followed by `_non_utf8` (the Linux-only part split out; the
  original name stays on the portable part, which the base-names guard already requires). No other name. A test
  is split exactly when it checks things that also hold on macOS; the implementer says which of the four it split
  and why.
- **w2 (the base-names guard).** `guard_t079.rs` pins the 1,121 Linux test names of main `ee9b130`. It stays for
  the platform batch (T-072 to T-078, T-092), where the risk is a test lost behind a cfg; a later task that
  renames or removes a test on purpose lists `xtask/tests/guard_t079.rs` in its `touches` for the test author.
  When the platform batch ends the director decides whether it is retired (noted in T-091).

### Rulings on review round 1 and CI round 1 (director, D-242; `reviews/T-079-r1.md` at `0be64ec9d98d717a73fc0bfc3008c553875a9ffb`; record `tasks/ci-runs/T-079-round-1.md`; told to both builders)
- **x1 (finding 3, defect; implementer).** `deep_trees_stop_at_the_depth_bound` takes its 2,100-deep chain down
  without recursion per level before the fixture is dropped, so that no step of the test needs a stack that grows
  with the depth. Proof on Linux: the test passes with `RUST_MIN_STACK=262144` (the reviewer's reproduction: it
  aborts there today). Look for the same in every other test that builds a deep chain (`guard_t038_cli`'s
  `deep_chain` and the `symbols-deep-chain` case) and say what each does at its end.
- **x2 (finding 6, test gap; implementer).** The case-folding probe of `tree_order_and_counts` cannot lie
  unnoticed: the test checks the probe's answer against the tree it materialized (both twins present on disk
  when the probe says "does not fold"; exactly one when it says "folds"), so a probe that answers "folds" on a
  case-sensitive file system fails the test on Linux.
- **x3 (finding 4, test gap; test author).** The base-names guard's scan also reads `cfg_attr`: a base test whose
  attributes hold a `cfg_attr(<predicate>, cfg(...))` (or `cfg_attr(<predicate>, ignore)`, already a `no-skip`
  finding) counts as conditional on that predicate, so the reviewer's mutant M15 fails the guard.
- **x4 (finding 5, spec gap of T-071's tool).** `cfg_attr(<predicate>, cfg(...))` on a test passes `no-skip`: to
  T-091 (noted there).
- **x5 (finding 1; implementer; the spec's wording corrected here).** The reason of the four non-UTF-8 entries says
  only what is true: macOS refuses a file name that is not valid UTF-8, so the fixture cannot be made there;
  Windows is not claimed (its names can hold unpaired surrogates): T-092 decides the Windows side. `platforms`
  stays `linux`, counterpart `none`.
- **x6 (finding 2).** The eight `pending` entries need their tasks to carry the counterpart: T-076 and T-077 are
  noted on main (the counterpart tests named, `xtask/platform-tests.toml` in `touches`, tier high because the
  registry is a gate path). No change in this task.
- After the rework: merge the tests branch, a second join, CI round 2 on `ci/candidate`, review round 2.
- **x3 read as built (the test author's run R-333).** A `cfg_attr(<predicate>, ignore)` does not make a base
  name absent for the guard (an ignored test is still in cargo's list; it is `no-skip`'s finding): the
  parenthesis of x3 that said otherwise is dropped.

## Log
### director
<!-- end director log -->
- 2026-11-18 **CI round 1** (the implementer's `CI-REQUEST`; record `tasks/ci-runs/T-079-round-1.md`, from the six
  artefacts, run R-331): run 44245093360 on `ci/candidate` at the joined head
  `885f4f1557d4aa1196e79e4136a1f6e1f55da6c4`. `gate-linux` success; the counted jobs success. `xtask` exit status 0
  on all three platforms (Linux 286 passed, macOS 281, Windows 278; 23 binaries each); `acceptance_t079` 2 of 2 and
  `guard_t079` 6 of 6 on all three; the 8 `xtask` entries present or absent as registered. **`gate-macos` is not
  green, from one cause**: the test thread of `deep_trees_stop_at_the_depth_bound` (the 2,100-deep chain, one of
  the four portable fixes) overflows its stack and aborts the binary `acceptance_t019_cli`, leaving 5 of its 7
  tests without a verdict (`5 scorecard problems`). Of the 16 formerly failing macOS tests: 8 absent as registered,
  the 4 non-UTF-8 tests ok under their original names with the `_non_utf8` parts absent, 2 portable fixes ok, 2 in
  the aborted binary. The pending CI criterion: clauses 3 and 4 not met (the macOS gate and its scorecard list),
  the others met. Rework is ruled with the review's findings.

- 2026-11-18 **CI round 2: `gate-macos` is green** (record `tasks/ci-runs/T-079-round-2.md`, from the six artefacts, run
  R-335): run 44253737976 on `ci/candidate` at the joined head `6dd4cd09364703454881766b06c9d539ba43bab9`.
  `gate-macos` success: every gate step passes (`no-skip` with `20 platform test(s) registered (8 pending)`; the
  scorecard line `macos: 64 items; 216 tests ran; 3 registered absent here; page not compared (computed on Linux)`
  with nothing after it; the test step `1117 passed, 0 failed, 0 ignored in 133 suites`). `gate-linux` success;
  the counted jobs success; `xtask` exit status 0 on all three platforms (Linux 286 passed, macOS 281, Windows
  278). The two Windows jobs are red at `kiln-workspace`, as expected until T-073. The pending CI criterion:
  seven of eight clauses met; of clause 5 the "0 failed" half is met, and that the 12 registered product tests
  are absent by name on macOS cannot be read from this run (no full test output is uploaded when the gate
  passes) — it was read by name in round 1's record, on the same registry and cfg lines. The stack overflow of
  round 1 is gone, which fits the reviewer's reproduction of its cause.

### planner
- 2026-11-15 · R-285 · draft · did: full spec from the split of T-071 (D-180 Q-1 B) · not run: builds · next: director
- 2026-11-18 · R-321 · specified · did: re-specified in full under v1.5 against task/T-071 @ b7dd7d6; Windows check probed twice (scratch, deleted) · not run: CI · next: director (Q-4 to Q-8)
- 2026-11-18 · R-325 · specified · did: re-check after T-071's merge (D-235): the eight written in; main's no-skip on main + patch run (scratch, deleted) · not run: CI · next: director (Q-8)
<!-- end planner log -->
### implementer
- 2026-11-18 · R-327 · start (`uptime -s` 2026-11-18 04:43:09; worktree clean at `ee9b130`). Criteria in my words:
  the registry holds exactly 20 entries (the patch's 8, then 12 `linux` ones: 3 `/dev/full` pending T-076, 4
  non-UTF-8 `none`, 5 no-display pending T-077, all `expires = "2026-12-31"`); those 12 tests sit behind exactly
  `cfg(target_os = "linux")` with their only-used helpers; the 4 Linux-assuming tests get portable fixes with no
  cfg and no platform branch, every Linux assertion kept; one sentence of `docs/development.md` made true;
  `xtask` checks for Windows with 0 errors; main's tool on this tree prints `20 platform test(s) registered`.
- step (a): `git apply --check` then `git apply tasks/T-079-restore-from-T-071.patch` → commit `c61acc5` (the patch
  alone, 8 files, +98 −4). `cargo xtask no-skip` → `PASS  no-skip: self-test ok (17 known findings); 197 files
  scanned, 0 allowlisted skip(s), 8 platform test(s) registered (0 pending), today = 2026-11-18`.
- step (b), `/dev/full` (3) → `05ad9da`: `#[cfg(target_os = "linux")]` on `ac8_stdout_write_failure_exits_1`,
  `run_and_control_exit_1_when_stdout_cannot_be_written`, `run_exits_1_when_stdout_cannot_be_written`; no helper is
  used only by them (`run_cmd`, `output_with_timeout` have other callers). Entries `pending` T-076, `expires =
  "2026-12-31"`; `doc`/`quote`: `docs/protocol/ui.md` / "writing stdout failed" (exit-code table, line 45) for `kiln
  script`, `docs/control-protocol.md` / same (lines 213–214, the `run` and `control` rows) for the two `kiln-cli` tests
  (`grep -n "writing stdout failed" docs/protocol/ui.md docs/control-protocol.md`). `no-skip`: `… 11 platform test(s)
  registered (3 pending) …` PASS.
- step (b), no display (5) → `8ce9cd4`: the cfg on the three `acceptance_t065.rs` tests, `window_t065.rs`'s and
  `guard_t055.rs`'s test; the same cfg on what only they use: in `acceptance_t065.rs` `use std::net::{…}`,
  `UNUSED_DISPLAY`, the `Display::WaylandOnly`/`Unreachable` variants and their two match arms in `kiln_in`,
  `assert_display_unused`, `assert_no_window`; in `window_t065.rs` `assert_no_window`; in `guard_t055.rs` four `use`
  lines, `TIMEOUT`, `kiln_no_args`. No twin anywhere. Entries `pending` T-077, `docs/protocol/ui.md` / "If the window
  system cannot be started it exits 2" (line 67). `no-skip`: `… 16 platform test(s) registered (8 pending) …` PASS.
  Stand-in for macOS: `rustup target add aarch64-apple-darwin` → already installed; `cargo check --target
  aarch64-apple-darwin` first failed in `tree-sitter`'s C build (`cc: unrecognized option '-arch'`), so a probe-only
  stub C compiler (ADR-0009 a3: compiles an empty unit with `clang --target=arm64-apple-macos11`; scratch, never
  committed) as `CC_aarch64_apple_darwin`, `AR_aarch64_apple_darwin=llvm-ar`. Then `cargo check --workspace
  --all-targets --locked --target aarch64-apple-darwin` and `cargo clippy … -- -D warnings`: exit 0, no warning.
  Sanity check of the stand-in: with the cfg removed from `window_t065.rs`'s `assert_no_window`, clippy for that
  target says `error: function \`assert_no_window\` is never used` (cfg restored, not committed).
- step (c), the four portable fixes (no cfg, no platform branch in the test fns; each test run green on Linux):
  - `disk::tests::failed_rename_removes_own_temp_and_keeps_target` → `1695c0c`. Old: `assert_eq!(used,
    Some(dir.join(".kiln-tmp-1")));` New: `assert_eq!(used, Some(dir.canonicalize().unwrap().join(".kiln-tmp-1")));`
    Same value on Linux (`TMPDIR` unset, `readlink -f /tmp` = `/tmp`, `fresh_dir` is under `std::env::temp_dir()`);
    `DiskFs::new` resolves the root with `fs::canonicalize` (`disk.rs:179`). Still an exact equality.
  - `guard_t038_cli::disk_refusals_through_control` → `c63f846`. Old (in `Work::loop_links`): `Some(40), // ELOOP`.
    New: `Some(ELOOP)` with `#[cfg(target_os = "linux")] const ELOOP: i32 = 40;` and `#[cfg(target_os = "macos")]
    const ELOOP: i32 = 62;` at file level (62 is the number the macOS run printed: `(os error 62)`, Interface (1)).
    Linux: the same literal 40. Windows: the file does not compile there before T-092 anyway (`std::os::unix`).
  - `acceptance_t019_cli::deep_trees_stop_at_the_depth_bound` → `cf23269`. Old: `mkdir -p` of `chain(2100)` in
    `c2100` and `assert!(out.status.success(), "mkdir -p of the 2,100-deep chain")`. New: the chain built with short
    paths only (each level made at `c2100-parent`, the chain so far renamed into it, 2,099 times, then renamed to
    `c2100/d`), each step `.expect("a level of the 2,100-deep chain")`. The assertions on the tree are unchanged:
    `find d -mindepth 2099 -type d` → exactly 1 line ("the chain is 2,100 deep"), and the listing equals `expected`.
    The method is the one `guard_t038_cli.rs::deep_chain` uses; the spec's words are "one level at a time relative to
    the last directory": std has no `mkdirat`/per-thread `chdir`, so I chose renames (doubt D2 below).
  - `acceptance_t019_cli::tree_order_and_counts` → `0707a2c`. The only case-only twins in its trees are `s/t/Z` and
    `s/t/z` of the `dot_git` tree (random names are `[a-e0-9._-]` only, `STEM`/`EXT`; no hand tree has an upper-case
    name: `sed -n 825,965p … | grep '[A-Z]'`). Old: `let dot_git = tree_of(&[…]);` and `assert_eq!(got.entries,
    entries(&[… ("s/t/Z", "file"), ("s/t/_", "file"), ("s/t/z", "file") …]));` New: `let dot_git =
    as_held_in(tree_of(&[…]), &w.dir);` and `assert_eq!(got.entries, as_listed_in(entries(&[…same…]), &dot_git,
    &w.dir));` where both helpers return their input unchanged unless `folds_case(dir)` (a probe file `case-probe-Aa`
    found as `case-probe-aA`) — so on Linux every value is the old one; where case folds, the later twin (and what is
    below it) is not written and not expected. `check_against_model` unchanged. Not exercised on a folding file
    system here (`mount -t tmpfs -o casefold` refused: the kernel has no `/sys/fs/unicode`); the filter's logic
    checked on a scratch copy (keeps `s/t/Z`, drops `s/t/z`; drops `a/y` after `A`). Doubt D3 below.
- QUESTION (Q-9, blocking): the four non-UTF-8 tests. The spec says "the non-UTF-8 case only becomes a `linux` test
  (split out if the test checks other things)" and (3) "a split keeps the original name for the part that stays";
  AC-1 is an equality on `(file, item, …)` whose items the test author can only take from Interface (1)'s names; the
  dispatch says "a marked test keeps its body". All four check other things: `disk_list_dir_reports_entries_by_their_own_kind_and_skips_bad_names`
  (`acceptance_t019_workspace.rs:202`: kinds, links, FIFO, socket, `OutsideRoot`, loop → `Io`, plus `n\xff`, `d\xfe`,
  `b\s`, `skipped` 3), `tree_counts_skipped_names_on_disk` (`:408`: `bad\xff` and `back\slash`, `skipped` 2),
  `disk_list_dir_reports_own_kinds_sorted_and_skips_odd_names` (`project.rs:106`: kinds, links, FIFO, socket,
  `back\slash`, `n\xff`, `skipped` 2, and **`b.rs` beside `B.rs`**, a case-only twin that macOS folds, so a portable
  half fails there too; spec silent), `documented_differences_from_git` (`project.rs:357`: braces, trailing tab,
  nested `.git`, `back\slash`, `n\xff`, `skipped` 2, `ignored` 4). A: mark the four whole (body kept, original names
  registered; AC-1 holds; on macOS their other checks then do not run). B: split, the `linux` half keeps the
  original name and the whole body (registered), a new portable test under a new name carries the rest with
  `skipped` one less, plus a case fix in `project.rs:106`'s half (outside the spec; the new halves' macOS result
  unknown until CI). C: split, the portable half keeps the name: the 4 registered items get new names, which AC-1 (as
  the test author writes it from the spec) cannot contain. I have not chosen; the four are unmarked at this head.
- gate attempt 1 (`cargo xtask ci` at `0707a2c`, 16 entries): `GATE PASSED in 649.8s` (`test` 1122 passed, 0 failed;
  `no-skip` `… 16 platform test(s) registered (8 pending) …`).
- Ruling w1/w2 (D-241, main's task file) received mid-run; accepted: Q-9 is answered by it. All four are split,
  because each also checks things that hold on macOS (kinds, links, FIFO, socket, `OutsideRoot`, loop, `back\slash`;
  gitignore braces, trailing tab, nested `.git`). → `7600934`: the original name keeps the portable part (the
  non-UTF-8 lines removed, `skipped` one or two less: 3 → 1, 2 → 1, 2 → 1, 2 → 1); a new `<name>_non_utf8` test,
  `#[cfg(target_os = "linux")]`, holds the base body **whole** (checked: each copy equals the fn on `ee9b130` but for its
  name and, in `project.rs`, the fixed fixture tag `kinds`/`s6` → `kinds-nu8`/`s6-nu8`, since `work(name)` is keyed
  by name and pid only and the two would collide in parallel), so every Linux assertion of the base still runs on
  Linux. `use std::ffi::OsStr` and `OsStrExt` in both files get the same cfg (only the `_non_utf8` tests use them;
  the stand-in reports `unused import` without it). In `project.rs`'s portable
  `disk_list_dir_reports_own_kinds_sorted_and_skips_odd_names` `B.rs` is `C.rs` (and its expected entry): `B.rs` beside
  `b.rs` cannot exist on a case-folding file system; `C.rs` still sorts before `b.rs` by bytes and after it by a
  case-blind order; the `B.rs`/`b.rs` pair stays in the `_non_utf8` copy (doubt D4). Entries `none`, reason "no file
  name on macOS or Windows can be invalid Unicode", `docs/protocol/project.md` / "not valid UTF-8 or contain"
  (line 26). `no-skip`: `… 20 platform test(s) registered (8 pending) …` PASS; the eight tests pass on Linux.
- step (d) → `1fa1891`: "The registry has no entry yet: …" replaced by: the registry lists the tests of `xtask`'s
  queue, worktree and publish tools (Linux or Unix only, no counterpart) and the product tests of a facility macOS
  lacks (`/dev/full`, non-UTF-8 names, X11 without a display), each Linux only; the Windows marking is T-092's.
  Evidence: `grep -c '^\[\[test\]\]' xtask/platform-tests.toml` = 20; per `file`: 8 under `xtask/` (`queue.rs` 2,
  `worktree.rs`, `publish.rs`, `acceptance_t013_publish.rs`, `guard_t005.rs`, `acceptance_t005_r3/r4.rs`), all
  `counterpart = "none"`; `platforms`: 17 `linux`, 3 `unix` (the 3 are `xtask`'s); the 12 product entries all `linux`;
  `ls tasks | grep T-092` → `T-092-mark-product-tests-for-windows.md`.
- Main's gate on this tree (Pinned (j)): main had moved to `b0238ed` (6 ledger/metrics commits after `ee9b130`, only
  `tasks/`, `metrics/`, `handoffs/`: `git diff --stat ee9b130 main`); `git archive main | tar -x` into a scratch dir,
  one `cargo build -p xtask --locked` there, that binary run in the worktree: `PASS  no-skip: self-test ok (17 known
  findings); 197 files scanned, 0 allowlisted skip(s), 20 platform test(s) registered (8 pending), today =
  2026-11-18`; `PASS  hygiene: reviews: 55 of 67 …, ok; session-paths: 675 tracked file(s) clean …`; `PASS  scorecard:
  64 items: 47 verified, 0 implemented, 0 failing, 17 planned.`; `PASS  layers: 11 crates, 14 internal edges, layer
  table holds`; `PASS  ui-commands: self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s)`; `PASS  schema:
  30 commands; audit ok; docs/commands.json current`.
- AC-4: `cargo check -p xtask --all-targets --locked --target x86_64-pc-windows-msvc` → exit 0, 0 `error` lines.
  macOS stand-in (stub C compiler as above): `cargo clippy --workspace --all-targets --locked --target
  aarch64-apple-darwin -- -D warnings` → exit 0, no warning, after the split too.
- Doubts: D2 the deep chain is built by renames, not "relative to the last directory" (same effect: no call names a
  long path). D3 `tree_order_and_counts`' case filter has not run on a folding file system; the spec named only the
  model, and the literal list after it (`s/t/Z` and `s/t/z`) also had to pass through the filter or the test could
  not pass on macOS. D4 `C.rs` for `B.rs` in a portable part (spec silent). D5 the CI record showed only each
  failing test's first panic: a fixed or split test may fail later on macOS for another reason (e.g. the
  `symbols-deep-chain` case of `disk_refusals_through_control` on macOS's shorter `PATH_MAX`); only CI shows it.
- gate attempt 2 (`cargo xtask ci` at `c71c8e7`, the committed state): `GATE PASSED in 549.2s` (`test` 1126
  passed, 0 failed, 0 ignored in 131 suites = 1122 + the 4 `_non_utf8`; `hygiene` PASS; `no-skip` `… 20 platform
  test(s) registered (8 pending) …`). 2 gate attempts, both green; 0 CI rounds yet. Scratch (`/home/user/kiln-impl-T-079/`)
  deleted. Not run: macOS, Windows (the director's CI). No information outside the repository was needed but the
  director's mid-run rulings w1, w2. Next: director (CI round 1).
- CI-REQUEST: c71c8e74782397bfa55a3135346d681561e9b31b
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
