---
id: T-005
title: "Merge queue gates with main's xtask, refuses tasks without their lifecycle evidence, resolves generated-file conflicts, tolerates only ledger commits on main, and seeds worktrees without copying the whole target"
tier: high              # xtask/ (CI and gate)
state: in_progress
owner_role: director
scorecard_items: []
depends_on: []
touches:
  - xtask/src
  - xtask/generated.toml
  - xtask/ledger.toml
  - docs/development.md
budget_tokens: 700000
iteration: 0
found_by: planner
origin_task: null
---

# T-005 Merge queue gates with main's xtask, refuses tasks without their lifecycle evidence, resolves generated-file conflicts, tolerates only ledger commits on main, and seeds worktrees without copying the whole target

## Goal
After this task `cargo xtask queue T-NNN` can no longer be passed by a task that weakens its own
gate, and it mechanically refuses a task whose recorded state, tier evidence or changed paths do
not satisfy METHODOLOGY §3–§7 (closing O-4 for `touches` and evidence). `cargo xtask worktree
new` (and the revert check's private target) stop copying all of `target/` — about 1.9 GB on
this machine (`du -sh target` in the primary checkout, 2026-11-12: 1.9 G, of which
`debug/incremental` 409 M and workspace-crate files in `debug/deps` 634 M by
`du -ch target/debug/deps/*kiln* target/debug/deps/xtask*`) — while never letting two source
trees share build artifacts. It also resolves rebase conflicts that lie only in generated files
by regenerating them, and accepts on `main` only queue merges plus director commits to ledger
paths. Sources: D-009; the bootstrap agent's report warnings (a) the queue runs the task branch's
own `xtask`, (b) it checks no tier, join record or `touches`, (d) each seeded worktree copies the
whole `target/` and once filled the disk, (e) the rebase flattens the join merge, so join
evidence must come from the metrics record (D-6); `docs/bootstrap-verification.md` §7 defects
4 and 5.

## Reading list
- `docs/decisions.md` D-005, D-007, D-009; `methodology/METHODOLOGY.md` §3–§8, §13, O-4.
- `docs/development.md` sections "Task worktrees", "Merge queue", "Join".
- `docs/bootstrap-verification.md` §6–§7.
- `xtask/src/{queue,worktree,join,git,frontmatter,main}.rs`; `methodology/templates/{task,review}.md`.

## Interface
Command lines are unchanged:
```text
cargo xtask queue T-NNN
cargo xtask worktree new T-NNN [--tests] [--no-seed]
cargo xtask join --revert-check T-NNN [--revert <sha>]...
```

### Queue preconditions (checked before any rebase, in this order)
Each failure is a **refusal**: exit code 1, one stderr line starting exactly
`queue: refused: <check>: ` followed by the reason, `main`, the task branch and worktree
unchanged, and no record written to `metrics/queue.jsonl` (as for today's refusals).
`<check>` is one of:

| check | Passes when |
|---|---|
| `task-file` | exactly one `tasks/T-NNN-*.md` exists in **main's** tree and its front matter parses with `id: T-NNN`, a valid `tier` and a `touches` list |
| `front-matter` | the task branch's copy of that file has front matter identical to main's (builders may append to the log body only) |
| `state` | main's `state` is `queued`, or `reviewed` (tier medium/high), or `self_verified` (tier low) |
| `touches` | every path in `git diff --name-only --no-renames <merge-base main task/T-NNN> task/T-NNN` is allowed (below) |
| `join` | tier medium/high: `metrics/tasks.jsonl` in the primary checkout has a record `kind:"join"`, `task:"T-NNN"`, `result:"joined"` whose `joined_sha` is an ancestor of, or equal to, the task head |
| `review` | tier medium/high: on the task head, the `reviews/T-NNN-r<N>.md` with the highest N has `verdict: approve`, its `reviewed_sha` is an ancestor of or equal to the task head, and every commit after `reviewed_sha` changes only `reviews/T-NNN-*` and the task file |
| `approval` | tier high: main's `docs/decisions.md` has a table row whose Decision cell begins with `Approve T-NNN` (D-2) |
| `main-history` | every commit on main's first-parent history after the newest `head_sha` of a `merged` record in `metrics/queue.jsonl` changes only ledger paths, respecting each entry's mode (below); passes trivially when no `merged` record exists yet |

Allowed paths for `touches`: a path `p` is allowed if, for some entry `t` of main's `touches`,
`p == t` or `p` starts with `t` followed by `/` (a trailing `/` on `t` is ignored); or `p` is the
task file itself; or, for tier medium/high only, `p` is a file named `acceptance_tNNN*.rs` (NNN =
the task's number) directly in some `crates/*/tests/`, `apps/*/tests/` or `xtask/tests/`
directory, or `scorecard/items/<id>.toml` for an `<id>` in the task's `scorecard_items`, or
starts with `reviews/T-NNN-`; or `p` is a generated path (below) — generated paths are exempt.

The join evidence is read only from the `join` record in `metrics/tasks.jsonl`, never from
merge commits in history: the queue's rebase flattens the join merge (D-6).

### Data files (read from **main's** tree, never from the task branch)
- `xtask/generated.toml` — the generated-file policy (D-1): an ordered list of entries, each with
  `path` (a repository path) and `command` (argv; the token `{xtask}` means main's built xtask
  binary) that regenerates it in a checkout. Initial content, in this order:
  `Cargo.lock` ← `cargo update --workspace` (from main's lock: re-resolves workspace members
  only, does not bump other packages); `docs/commands.json` ← `{xtask} schema --write`;
  `docs/scorecard.md` ← `{xtask} scorecard`.
- `xtask/ledger.toml` — the ledger paths the director may commit to on `main` directly (D-4),
  each with `mode` `any` or `add-only` (new files only; no modification or deletion):
  `tasks/`, `metrics/`, `docs/decisions.md`, `docs/adr/`, `STATE.md`, `handoffs/`,
  `retrospectives/`, `evaluations/`, `methodology/` (mode `any`; "only at a retrospective"
  stays a director check, noted in the file) and `scorecard/items/` (mode `add-only`). Code,
  `xtask/`, `.github/`, `Cargo.*` are never ledger paths.
- Both are parsed strictly (unknown keys, empty lists, a path in both files, or a ledger entry
  under `xtask/`, `.github/` or `Cargo.*` are errors); the queue refuses with check `config` if
  either is missing or invalid on main.

### Generated-file conflicts
Tasks still regenerate generated files locally so their own gate passes. When the rebase stops
on a conflict and **every** conflicted path is a generated path, the queue takes main's side for
those paths, continues the rebase, and repeats for later commits; after the rebase it runs every
entry's command (in file order) in the worktree and, if any generated file changed, commits them
as one commit with message lines `queue: regenerate generated files`, `Task: T-NNN`,
`Role: director`. The gate then runs on the result. A conflict touching any other path is a
`conflict` result as today. The queue record gains `regenerated`: the list of generated paths
that were conflicted or changed (empty list otherwise). If a regeneration command fails, the
result is `conflict` with `failed_step` `regenerate:<path>` and main is unchanged. Until this
task lands the director does all of this by hand.

### Gate provenance
- The gate is run by an `xtask` binary built from **main's commit at `base_sha`** (not from the
  task branch), executed with the rebased task worktree as working directory and the worktree's
  target directory as `CARGO_TARGET_DIR`.
- **Gate-path exception.** If any changed path (as in `touches` above) lies under `xtask/`,
  `.cargo/`, or is `clippy.toml` or `rust-toolchain.toml`, the queue runs main's gate and then
  the task's own (rebased) gate; both must pass. There is no override (D-3).
- `metrics/queue.jsonl` records gain two fields (METHODOLOGY §13 fields unchanged):
  `gate_xtask` = `"main"` or `"main+task"`, and `gate_xtask_sha` = the commit main's xtask was
  built from. For a `gate_failed` result under `"main+task"`, `failed_step` is prefixed `main:` or
  `task:` to say which gate failed.
- The queue works on any repository with an `xtask` package whose `ci --report <file>` writes JSON
  `{"passed": bool, "failed_step": string|null, …}` — the existing report contract.

### Target seeding
`worktree new` (without `--no-seed`) and the revert check seed the new target directory with a
**copy** (never hard links, symlinks or a shared directory) of only those build artifacts in the
source target that belong to packages that are **not** workspace members; nothing of a workspace
member (and no `incremental/` data) is copied. `--no-seed` keeps creating an empty target.

## Acceptance criteria
Tests run the `xtask` binary (`env!("CARGO_BIN_EXE_xtask")`) against scratch git repositories
created under `env!("CARGO_TARGET_TMPDIR")`. A scratch repository may be a tiny cargo workspace
with its own toy `xtask` package implementing `ci --report` (no registry dependencies, so it builds
offline in seconds).
- AC-1 (self-weakened gate is caught): scratch repo whose main's toy `xtask ci` fails when a file
  `RED` exists; a low-tier task (`touches: [RED, xtask]`, state `self_verified`) adds `RED` and
  changes its own toy xtask to always pass → queue result `gate_failed`, `failed_step` starts
  with `main:`, `gate_xtask:"main+task"`, main unchanged. The same task without the xtask change
  but with `touches: [RED]` → `gate_failed`, `gate_xtask:"main"`. Oracle: exact value.
- AC-2 (main's gate is what runs): a task that touches only non-gate paths and whose branch
  contains an xtask change is refused at `touches` (it cannot reach the gate); a task whose
  change is green under main's gate is `merged` with `gate_xtask:"main"` and `gate_xtask_sha`
  equal to `base_sha`. Oracle: exact value.
- AC-3 (state): for each state other than the accepted ones (`draft`, `specified`, `ready`,
  `in_progress`, `self_verified` for medium/high, `tests_joined`, `merged`, `blocked`), queue →
  refusal `state`; nothing written. A low task in `self_verified` and a medium task in `reviewed`
  (with evidence) pass this check. Oracle: exact value.
- AC-4 (front matter): a task branch that changes its task file's `tier`, `touches` or `state`
  → refusal `front-matter`; appending to the log body only passes. Oracle: exact value.
- AC-5 (touches): a change to a path outside `touches` → refusal `touches` naming that path;
  `touches: [crates/a]` allows `crates/a/src/x.rs` but not `crates/ab/x.rs`; a deleted file
  outside `touches` is refused too; for a medium task `crates/a/tests/acceptance_t007_x.rs`,
  `scorecard/items/<its item>.toml` and `reviews/T-007-r1.md` are allowed, while
  `scorecard/items/<other item>.toml` and `crates/a/tests/acceptance_t008.rs` are refused; for a
  low task an `acceptance_t007.rs` file outside `touches` is refused. Oracle: exact value
  (naive model: the allowed-path rule above as a predicate).
- AC-6 (join evidence): medium task with no join record, or only `result:"acceptance_failed"`, or a
  `joined` record whose `joined_sha` is not an ancestor of the head → refusal `join`. Oracle:
  exact value.
- AC-7 (review evidence): missing review file, highest round `request-changes` (even if an earlier
  round approved), `reviewed_sha` not an ancestor, or a code change committed after
  `reviewed_sha` → refusal `review`; a later commit touching only the review file and the task
  log passes. Oracle: exact value.
- AC-8 (approval): high task without an `Approve T-NNN` row in main's `docs/decisions.md` →
  refusal `approval`; with it, passes. Oracle: exact value.
- AC-9 (no side effects of refusal): for every refusal above, `git rev-parse main`, the task
  branch head, `git status --porcelain` of both checkouts and `metrics/queue.jsonl` are byte-equal
  before and after. Oracle: exact value.
- AC-10 (seed is fresh, never stale): build a scratch workspace with one member crate and one
  path dependency outside the workspace in the primary checkout; `worktree new T-001`; change the
  member crate's behaviour on the task branch and build/run in the worktree → the new behaviour is
  observed; run on main → the old behaviour; then change main's crate, rebuild main, and rebuild
  the worktree → the worktree still observes its own source. Oracle: exact value (program output).
- AC-11 (isolation): a build in the seeded worktree leaves every file under the primary
  checkout's `target/` unchanged (manifest of relative path, size and content hash before ==
  after), and vice versa. Oracle: exact value.
- AC-12 (seed is useful and small): the first build in a seeded worktree reports the non-member
  package as fresh (cargo `--message-format=json` `compiler-artifact` with `"fresh":true`) and
  rebuilds the member; the seeded target contains no file whose name contains the member crate's
  name and no `incremental` directory. Oracle: exact value.
- AC-13 (revert check isolation): `join --revert-check` leaves the task worktree's target
  byte-identical (manifest as in AC-11), so a following build there is not served the reverted
  tree (bootstrap-verification §7 defect 4). Oracle: exact value.
- AC-14 (generated conflict resolved): two tasks each change `docs/scorecard.md` (toy generated
  file whose toy command writes a deterministic summary of `items/`) and their own `items/` file;
  queue the first (merged), then the second → `merged`, `regenerated:["docs/scorecard.md"]`, and
  main's `docs/scorecard.md` equals what the toy command produces on main's final tree. Oracle:
  idempotence (regenerating on the merged main changes nothing) + exact value.
- AC-15 (other conflicts still rejected): a conflict on `docs/scorecard.md` and on a non-generated
  file in the same rebase → `conflict`, `failed_step:"rebase"`, main and task branch unchanged.
  A failing regeneration command → `conflict`, `failed_step:"regenerate:<path>"`. Oracle: exact value.
- AC-16 (generated paths exempt from touches): a task with `touches: [src/a]` that also changes
  `Cargo.lock` and `docs/commands.json` passes the `touches` check; a task branch that adds a path
  to its own `xtask/generated.toml` gains no exemption (main's file is read) and is refused at
  `touches`. Oracle: exact value.
- AC-17 (ledger history): after a merged task, director commits on main that change only
  `tasks/`, `metrics/`, `STATE.md` and add a new `scorecard/items/x.toml` pass `main-history`; a
  direct main commit that changes `src/` (or `xtask/`, or `Cargo.lock`) → refusal `main-history`
  naming the commit; a direct commit modifying an existing `scorecard/items/*.toml` → refusal
  `main-history`. Oracle: exact value.
- AC-18 (config): missing or invalid `xtask/generated.toml` / `xtask/ledger.toml` on main →
  refusal `config`. Oracle: exact value.

## Intended test names
No scorecard items (tooling). Tests in `xtask/tests/acceptance_t005.rs`
(`xtask::acceptance_t005::<fn>`): `self_weakened_gate_fails_under_mains_gate`,
`gate_runs_mains_xtask_and_records_it`, `refuses_unaccepted_states`,
`refuses_front_matter_changes_on_task_branch`, `refuses_paths_outside_touches`,
`allows_acceptance_scorecard_and_review_paths_for_medium_only`, `refuses_missing_or_failed_join`,
`refuses_missing_rejected_or_stale_review`, `refuses_high_without_approval_row`,
`refusals_change_nothing`, `seeded_worktree_builds_its_own_source`,
`worktree_build_does_not_touch_main_target`, `seed_reuses_non_member_artifacts_only`,
`revert_check_leaves_task_target_untouched`, `generated_only_conflict_is_regenerated_and_merged`,
`mixed_or_failed_regeneration_conflict_is_rejected`, `generated_paths_are_exempt_from_touches_by_mains_list`,
`main_history_tolerates_only_ledger_commits`, `invalid_policy_files_are_refused`.

## Out of scope
- Checking `Task:` / `Role:` commit lines, the "implementer never edits acceptance files" split
  per branch, token budgets — later tooling tasks (O-4 remainder).
- The queue updating task `state` itself; pushing; GitHub Actions changes (`.github/` untouched).
- A budgets or fuzz runner (separate later task, T-001 D-2).
- Known limitation, not addressed here (bootstrap report warning (c)): the `ui-commands` step
  inspects import paths in `kiln-ui`, not method calls, so a UI calling a lower crate through a
  value it was handed is not caught. Belongs to the task that builds `kiln-ui`.
- New dependencies for `xtask` (`xtask/Cargo.toml` and `Cargo.lock` are outside `touches`).
- `METHODOLOGY.md` text (changes only at a retrospective, §14).

## Implementer notes
- This task is itself queued by the **old** queue (it runs the task branch's own gate). That is the
  last time; say so in your log. The reviewer should run the new queue against a scratch repo.
- Build main's xtask from a clean checkout of `base_sha` (the primary checkout is on `main` and
  clean when the queue runs); its build directory must not be any worktree's target and must not
  be shared with a different source tree.
- Update `docs/development.md` ("Task worktrees", "Merge queue", "Join", the Toolchain table if `cp`
  options change) so that it describes the new behaviour exactly, including the refusal checks,
  the two data files, the generated-conflict procedure and the ledger rule.
- The two data files are new and therefore not on main when this task is queued; the old queue
  does not read them. After merge, the first run of the new queue reads them from main.
- Keep the existing unit tests' fake-gate design; real-cargo tests must stay fast (tiny scratch
  workspaces, no registry dependencies) because they run inside every gate.

## Decisions
Director's answers to the planner's questions of R-003 (2026-11-12).
- D-1 (was Q-1): generated-file policy. A data file (`xtask/generated.toml`) lists generated
  paths — `docs/scorecard.md`, `docs/commands.json`, `Cargo.lock` — with the command that
  regenerates each. Tasks still regenerate locally to pass the gate; when a queue rebase conflicts
  only on listed paths the queue regenerates them and continues; listed paths are exempt from the
  `touches` check. Until T-005 lands, the director does this by hand.
- D-2 (was Q-2, accepted as proposed): approval rows begin `Approve T-NNN`; no sha binding.
- D-3 (was Q-3, accepted as proposed): gate-path tasks pass both main's and their own gate; no
  override.
- D-4 (was Q-4): the director may commit directly to `main` only on ledger paths — `tasks/`,
  `metrics/`, `docs/decisions.md`, `docs/adr/`, `scorecard/items/` (adding items), `STATE.md`,
  `handoffs/`, `retrospectives/`, `evaluations/`, and `methodology/` at a retrospective — listed
  in a data file (`xtask/ledger.toml`) the queue reads, so its clean-main and history checks
  tolerate ledger commits and nothing else. Code, `xtask/`, `.github/` and `Cargo.*` reach `main`
  only through the queue.
- D-5 (was Q-5): the "warnings" were in the bootstrap agent's report: (a) the queue runs the
  task's own xtask; (b) no tier/join/`touches` checks; (c) `ui-commands` checks import paths, not
  method calls — out of scope, listed as a known limitation; (d) full `target/` copies (1.6 GB)
  filled the disk; (e) the rebase flattens the join merge, so join evidence comes from the
  metrics record.
- D-6 (from D-5 (e)): the `join` check reads only `metrics/tasks.jsonl`, never history.
- Director (2026-11-12): `Cargo.lock` regeneration = take `main`'s lock file, then `cargo update --workspace`; the queue must verify that no package already present in `main`'s lock changed version, and fail otherwise.
- Director rulings after join triage (2026-11-12, decision D-019) — part of the spec:
  a. `front-matter` passes when the branch's copy of the task front matter equals main's or is unchanged since the merge base.
  b. The shipped layout of `xtask/generated.toml` and `xtask/ledger.toml` is the specified schema.
  c. The `config` check runs just before `touches`.
  d. `guard_tNNN*.rs` test files are allowed paths, like `acceptance_tNNN*.rs`.
  e. `cargo xtask join` also finds acceptance and guard tests under `xtask/tests`.
  f. `xtask` refuses to run (exit 2, one line naming both paths) when the checkout it was built from differs from the workspace it is run in, except where the queue deliberately runs main's `xtask` against a task worktree.

## Questions
None open.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · tokens (if known) · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-12 · R-003 · (new) · did: specified seven named queue refusal checks (task file, front
  matter, state, touches with the allowed-path rule, join, review, approval) read from main,
  gating by main's xtask with a both-gates rule for gate-path tasks, two new queue record fields,
  and seeding by copying only non-member artifacts; ACs drive the binary on tiny scratch repos.
  Measured `target/` composition with `du` (commands in Goal). Tier high: `xtask/`. Rejected
  hard-link seeding (cargo rewrites some files in place, so a link could mutate main's target)
  and any shared target (bootstrap §7 defect 4). Unsure: generated `docs/scorecard.md` (Q-1),
  approval row format (Q-2), no-override rule (Q-3), director commits on main (Q-4), the cited
  "Warnings" sections (Q-5). · gate attempts: 0 · not run: gate (no code) · missing context: the
  cited "Warnings" sections · next: director review.
- 2026-11-12 · R-003 · draft · did: applied the director's answers (D-1…D-6): added the
  generated-file policy (`xtask/generated.toml`, conflict regeneration, `touches` exemption,
  `regenerated` record field) and the ledger rule (`xtask/ledger.toml`, `main-history` check
  with add-only `scorecard/items/`), both read from main; `config` refusal; ACs 14–18; the
  `ui-commands` limitation recorded as out of scope; touches gain the two data files. Chose
  `cargo update --workspace` from main's lock as the `Cargo.lock` regeneration (minimal
  re-resolution) — the director should confirm it at acceptance. · gate attempts: 0 · next:
  director review.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
