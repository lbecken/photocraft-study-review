---
id: T-005
title: "Merge queue gates with main's xtask, refuses tasks without their lifecycle evidence, resolves generated-file conflicts, tolerates only ledger commits on main, and seeds worktrees without copying the whole target"
tier: high              # xtask/ (CI and gate)
state: queued
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
     Clarified (D-021): `join` runs guard tests itself — every guard must pass on base and on the joined branch; otherwise the result is `guard_failed`, exit non-zero, and the record carries `guard_total`, `guard_fail_base`, `guard_fail_join`.
  f. `xtask` refuses to run (exit 2, one line naming both paths) when the checkout it was built from differs from the workspace it is run in, except where the queue deliberately runs main's `xtask` against a task worktree.
- Director rulings after review r1 (2026-11-13, decision D-026) — part of the spec. Threat model: honest mistakes and corner-cutting by agents, not a malicious agent; every weakening of the gate must be either impossible or visible and approved, never silent.
  R1 (review F1). The queue rebuilds main's `xtask` crate from the sources at `base_sha` at the start of every run; it never trusts a previously built `xtask` binary. Cached dependency artifacts may be reused (accepted residual risk, documented).
  R2 (F2). A review's `reviewed_sha` must be a full 40-hex commit id that is an ancestor of, or equal to, the task head; commits after it may touch only review files, the task file, generated paths and acceptance/guard test files. Anything else is refused at `review` as stale.
  R3 (F3). Gate paths are: `xtask/**`, `.cargo/**`, `rust-toolchain.toml`, `clippy.toml`, `rustfmt.toml`, `.github/**`, any `build.rs`, and the root `Cargo.toml` when its `[workspace.lints]`, `[workspace.metadata]` or any `[profile.*]` table differs from main's. A task whose diff touches a gate path (a) must be tier high, else refusal `gate-path`; (b) runs both gates as already specified; (c) needs an approval row in main's `docs/decisions.md` whose Decision cell begins `Approve T-NNN` and contains the words `gate paths`, else refusal `approval`. The queue prints the gate paths the task changed.
  R4 (F4). A task whose diff changes `Cargo.lock` or the dependency tables of any `Cargo.toml` must be tier high, else refusal `dependencies`. The lock rule — no package already in main's lock changes version — is checked on every queue run, not only after a rebase conflict; a violation is refusal `dependencies`.
  R5 (F7). The queue is side-effect free on every non-merged outcome: after `gate_failed`, `ff_failed`, a conflict or a refusal, the task branch points at the commit it pointed at before the run and its worktree is clean.
  R6 (F8). Evidence (join records, review verdicts, approval rows, task front matter, policy files) is read from main's committed tree, never from uncommitted working-copy files. If `metrics/` has uncommitted changes when the queue starts, refusal `ledger` with a hint to commit the ledger first.
  R7 (F10). An approval row is a Markdown table row in `docs/decisions.md` outside any code block.
  R8 (F9). `KILN_XTASK_RUN_IN` must either be what makes the queue's main-gate run possible, with a test that fails if it is ignored, or be removed. No dead bypass variable.
  R9 (F5, F6). End-to-end tests are required for the lock version rule and for "regeneration changed a non-generated path".
  Clarifications (2026-11-13, D-027): R6 — `ledger` applies to uncommitted changes in evidence files (`metrics/tasks.jsonl`, `docs/decisions.md`, `tasks/`, `reviews/`, the policy files); `metrics/queue.jsonl` is the queue's own output log and is exempt. R2 — after `reviewed_sha` the only test files that may change are the task's own `acceptance_tNNN*.rs`, `guard_tNNN*.rs` and `acceptance_tNNN_support.rs`. R7 — a table row is a `|`-delimited line under a header separator line, outside code. R9 — a leaky regeneration has the same result kind as a failed regeneration. R4 — the lock rule compares registry and git packages only.
- Director rulings after review r2 (2026-11-13, decision D-030) — part of the spec; this is the last round of rulings for T-005 (anything further becomes a follow-up task):
  R10 (review F11). Gate paths also include: any `Cargo.toml` in which a `[lints]` table (or a `lints.workspace` setting) or a package `build` key is added, removed or changed relative to main; and any file a package's `build` key names. R3 (a)–(c) apply to them.
  R11 (F13). An existing registry or git package's version may change in `Cargo.lock` only when the task is tier high and its approval row (as in R3/R7) contains the words `dependency versions`; otherwise refusal `dependencies`. With that approval the change merges.
  R12 (F15, F8). The queue commits its own record: after every run that appends to `metrics/queue.jsonl` it makes a commit on main touching only that file (subject `queue: record T-NNN <result>`). The `ledger` refusal then covers all of `metrics/` with no exemption (this replaces the exemption in D-027), and the main-history check reads `merged` records from main's committed tree only. A refusal still writes nothing.
  R13 (F12, F14). Tests are required that fail when (a) the fresh-rebuild/wipe step of R1 is removed — including against a fake `xtask` binary with a modification time in the future — and (b) the lock version rule is not applied to the rebased tree.
  Accepted residual risks, to be listed in `docs/development.md`: cached dependency build artifacts can be tampered with; gated code runs with the director's privileges; acceptance tests edited after a review are not re-reviewed; an approval row is not tied to a commit.

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
- 2026-11-12T23:12Z · R-006 · ready · ACs in my words: the queue refuses (exit 1, one
  `queue: refused: <check>: …` line, nothing changed or written) a task whose main task file,
  front matter, state, changed paths, join record, newest review, approval row, policy files or
  main's post-merge history do not satisfy the table; gates with main's xtask built from
  `base_sha` (plus the task's own when it touches a gate path, prefixing `main:`/`task:`);
  resolves generated-only rebase conflicts by taking main's side and regenerating in file order
  (one commit, `regenerated` field); seeds targets with copies of non-member artifacts only. ·
  did: new `xtask/src/{policy,precheck,seed}.rs`, rewrote `queue.rs` run/gate (`GateRunner`
  gains `GateXtask` + `build_main_xtask`; fakes kept), `worktree.rs`/`join.rs` seed via
  `seed.rs`, `main.rs` prints refusals; `xtask/generated.toml`, `xtask/ledger.toml`;
  `docs/development.md` (worktrees, queue, join, toolchain row). Main's xtask is built in
  `.git/kiln-queue/main-xtask/` from a private-index export of `base_sha` (not the primary
  checkout, so a toy repo without a committed `Cargo.lock` cannot dirty main). Seeding keeps
  mtimes and, for in-checkout non-member path deps, copies the source checkout's mtimes onto
  byte-identical source files so they stay fresh (otherwise a fresh checkout's newer mtimes
  rebuild them). Exercised the built binary on toy scratch repos (not the real repo): AC-1
  both variants (`main:red`/`main+task`, `red`/`main`), AC-3 state and AC-2 touches refusals
  (exit 1), AC-14 (`regenerated:["docs/scorecard.md"]`, idempotent) also with a `Cargo.lock`
  entry, AC-10/11/12 (worktree runs its own source, main's target manifest unchanged, helper
  `fresh:true`, app rebuilt, only helper files seeded), AC-13 (task target manifest unchanged by
  `join --revert-check`, `result_kind: fails`). This task is the last one queued by the **old**
  queue (it runs the task branch's own gate); the reviewer should run the new queue against a
  scratch repo. · gate attempts: 1 (last: green) · not run: the new queue/worktree/join against
  the real repository (forbidden for this run); a real `Cargo.lock` conflict with registry
  packages (lock-version check is unit-tested only); seeding of the real 1.9 GB target (size
  not measured) · missing context: (1) `config` has no place in the check order — I put it
  after `state`, right before `touches`, the first check that needs it; (2) `front-matter`
  read literally ("identical to main's") refuses every real task, because the director changes
  `state` on main after the branch forks; I accept a head copy equal to main's **or** to the
  merge-base copy (i.e. the branch did not change it), and refuse a deletion; (3) whether
  regeneration runs only after a generated conflict (I did so) or on every queue run; (4)
  which `Cargo.lock` packages the version check covers — I compare packages with a `source`
  (registry/git), not path packages; (5) failure to build main's xtask has no specified
  result — it is an error (exit 2) with no record. · next: director join with the acceptance
  tests, reviewer.
- 2026-11-12T23:47Z · R-018 · in_progress (round 2, D-019 d–f; join triage read) · did: (d)
  `guard_tNNN*.rs` allowed exactly like `acceptance_tNNN*.rs` (`precheck.rs`); (e)
  `join::acceptance_targets` searches `crates/*/tests/`, `apps/*/tests/` and `xtask/tests/`
  (files directly in those dirs); join still counts only `acceptance_*` as evidence (guards
  pass on base by D-010 and run in the gate); (f) new `provenance.rs`, checked first in
  `main()`: compile-time `CARGO_MANIFEST_DIR` is compared with (1) the runtime
  `CARGO_MANIFEST_DIR` when `CARGO_PKG_NAME=xtask` (cargo run / cargo test set it), and (2) the
  checkout whose `target/` holds `current_exe()`; a mismatch exits 2 with one line naming
  both checkouts. The three cases: **queue main gate** — the queue sets
  `KILN_XTASK_RUN_IN=<canonical worktree>` on main's xtask (gate and `{xtask}` regeneration);
  it excuses only a run whose workspace (nearest `[workspace]` above cwd) is exactly that
  path, so a leaked/exported value excuses nothing elsewhere, and main's binary lives in
  `.git/kiln-queue/…`, not a checkout target; **tests in toy repos** — the inherited
  `CARGO_MANIFEST_DIR` (if not scrubbed) is the build's own and the exe sits in its own
  checkout's target, so both checks pass whatever the cwd; **accidental bypass** — only an
  exact-path match of a dedicated variable skips the checks. Unit tests: `provenance::tests`
  (4), `join::tests::acceptance_targets_include_xtask_tests`, `precheck::tests::allowed_paths`
  extended. Scratch proof (clones `p`, `w` under scratchpad/t005/prov): build in `p` with
  `CARGO_TARGET_DIR=w/target` (cargo does not rebuild: the RC-1 situation), then in `w`:
  `cargo xtask help` → exit 2 "built from …/p but cargo is running it for the workspace …/w";
  direct `w/target/debug/xtask help` → exit 2 "…sits in the target directory of …/w"; after
  `touch xtask/src/main.rs` it runs. Clone `q` (this branch as main): `cargo xtask queue
  T-900` ran main's guarded xtask through the full real gate in the task worktree with no
  guard refusal (its acceptance_t005 children ran; record `gate_xtask:"main"`); it stopped at
  `test` only on the known defective case. · gate attempts: 1 (last: red `test`: only
  `allows_acceptance_scorecard_and_review_paths_for_medium_only`, the triage's RC-2 test
  defect, not yet fixed on `task/T-005-tests`; fmt/clippy/layers/ui-commands/schema/scorecard
  passed; no-skip PASS and fuzz/budgets EMPTY run individually; 82 unit, 15/16 acceptance,
  guards pass) · not run: a green full gate (blocked by the acceptance test I may not edit) ·
  missing context: whether (e) "guard tests" should also be run/recorded by join — I only
  widened the search path · next: re-join with the test author's fix, then gate.
- 2026-11-12T23:52Z · R-018 (cont.) · in_progress · director ruling on (f) received mid-run;
  my first design differed (it compared cargo's runtime `CARGO_MANIFEST_DIR` and the exe's
  owning `target/`, which would also refuse a worktree's binary run in an unrelated toy repo),
  so I replaced it: `provenance.rs` now resolves the git working tree at the build's workspace
  and at the run workspace (nearest `[workspace]` above cwd) and refuses (exit 2, one line,
  both paths) only when they differ and share a common git dir. Cases: own checkout → runs;
  unrelated repo (toy repos) → runs; not a working tree on either side → runs; queue main
  gate → explicit `KILN_XTASK_RUN_IN=<canonical worktree>` set by the queue, honoured only for
  exactly that workspace (main's export in `.git/kiln-queue/` is not a working tree anyway);
  a leaked value excuses nothing else. Unit tests: 3 in `provenance::tests` with real git
  worktrees. Scratch proof (scratchpad/t005/prov2): primary `p` + `git worktree add`
  `.worktrees/T-901`, xtask built in `p` into the worktree's target: `cargo xtask help` and
  the direct binary in the worktree → exit 2 naming both; same binary in an unrelated repo →
  runs; with the exception for the worktree → runs; after `touch xtask/src/main.rs` → runs.
  Clone `q` as main: `cargo xtask queue T-900` ran main's guarded xtask through the real gate
  in the task worktree, no refusal, stopping only at the known test. · gate attempts: 2 (last:
  red `test`, only `allows_acceptance_scorecard_and_review_paths_for_medium_only` — triage
  RC-2 test defect; every other step passes) · not run: green gate (blocked on the test
  author's fix) · missing context: none beyond the ruling · next: re-join with the fixed test.
- 2026-11-12T23:58Z · R-023 · in_progress (round 3, D-021) · did: `join` now runs the
  regression guards (`guard_tNNN*.rs`, found in the same places as acceptance tests,
  including `xtask/tests`) itself, on base (tests worktree) and on the joined branch; the
  record gains `guard_total` (joined-branch guards, a non-compiling target counts as one),
  `guard_fail_base`, `guard_fail_join`; any guard failure gives `result:"guard_failed"`
  (exit 1), the merge is kept like `acceptance_failed`, and the gate is not run. Decided:
  precedence `refused_base_passes` > `acceptance_failed` > `guard_failed` > gate; a guard target
  that does not compile counts as a failure. Unit test
  `join::tests::guards_must_pass_on_base_and_joined_branch` (pass everywhere → joined and gate
  run once; broken on join → guard_failed, gate not run; failing on base → guard_failed).
  docs/development.md Join section updated. · gate attempts: 1 (last: green, 172 tests in 27
  suites; `acceptance_t005` 16/16, `acceptance_t005_r2` 4/4, guards pass) · not run: a real
  `cargo xtask join` on a scratch repo with a breaking guard beyond the r2 acceptance test,
  which does exactly that and passes · missing context: none · next: review.
- 2026-11-13T01:50Z · R-032 · in_progress (round 4, after review r1) · did: implemented D-026
  R1–R8 with the D-027 clarifications, one commit per ruling (8bf668c R8, c54957f R1, e4a7ef6
  R2, 2787cc1 R7, be74759 R3+R4, 2e1d657 R5, f4ab84a R6, 0c4727d R4/R9 refinement) plus unit
  tests per ruling (91 unit tests). R1: every run re-exports `base_sha` and prunes all
  workspace-member artifacts, uplifted binaries, `incremental/` and unknown files from the
  private target before `cargo build` (`seed::prune_to_non_members`); stamp-based reuse gone.
  R2: `reviewed_sha` must match `[0-9a-f]{40}` and be a commit ancestor of the head; after it
  only `reviews/T-NNN-*`, the task file, generated paths and the task's own
  `acceptance_tNNN*`/`guard_tNNN*` files in a tests dir; else `review` "stale review". R3:
  `gate-path` refusal for non-high tiers; approval cell must contain `gate paths`; gate paths
  printed on stderr and in the summary line. R4: `dependencies` refusal for non-high tiers on
  any `Cargo.lock` or changed dependency tables; lock rule before the rebase and again on the
  rebased tree. R5: drop guard aborts the rebase, resets and cleans on every non-merged outcome
  and on errors. R6: join records from `main:metrics/tasks.jsonl`; `ledger` refusal (first
  refusal, right after the on-main check) for uncommitted evidence files; `queue.jsonl` exempt
  and still read as is by `main-history`. R7: Decision cells only from header+delimiter tables
  outside fenced/indented code, block quotes and HTML comments; `Approve T-005-tests` no longer
  approves T-005. R8: `KILN_XTASK_RUN_IN` removed. R9: lock rule and leaky regeneration are
  real (`dependencies` refusal / `conflict` `regenerate:<path>`, branch put back); unit-tested
  only, end-to-end tests are the test author's. Checked from outside on a scratch toy repo:
  a poisoned cached main xtask is rebuilt (attempt 2 `gate_failed`, not merged), branch left at
  its head, untracked `metrics/tasks.jsonl` → `queue: refused: ledger: …` exit 1.
  Readings chosen (fail closed; director please confirm): (1) check order `ledger`,
  task-file, front-matter, state, config, touches, join, review, `gate-path`, `dependencies`,
  approval, main-history — gate-path/dependencies after review so existing AC-16 still sees
  `join`; (2) gate paths also match `rust-toolchain`, `.clippy.toml`, `.rustfmt.toml`, and
  `clippy.toml`/`rustfmt.toml`/`build.rs`/`rust-toolchain*` and `.cargo/` at any depth;
  `xtask/tests/**` counts as a gate path; (3) root `Cargo.toml` is a gate path if a gate table
  differs from main's **or** the merge base's; an unparsable manifest counts as differing;
  (4) dependency tables include `[target.*.…]`, `[workspace.dependencies]`, `[patch]`,
  `[replace]`; compared merge base vs head; (5) `gate paths` is a case-sensitive substring of
  the same Decision cell; (6) `reviewed_sha` lowercase only; (7) `ledger` covers all of
  `metrics/` except `queue.jsonl` (wider than `metrics/tasks.jsonl`); (8) review files are read
  from the task head commit (as the check table says), not main's tree; (9) pre-rebase lock
  rule refuses only versions the task's own diff changed, so a lock that is merely stale is left
  to rebase/regeneration and re-checked after the rebase; (10) a post-rebase lock violation is a
  `dependencies` refusal (no record, branch restored). Not covered: a crate's custom
  `build = "x.rs"` script path; tampering with cached registry-dependency artifacts (accepted
  residual risk, documented).
  QUESTION: `self_weakened_gate_fails_under_mains_gate` (AC-1) queues a **low** task changing
  `xtask/src/main.rs` and expects `gate_failed`/`main+task`; R3 (a) makes that refusal
  `gate-path` (observed: `queue: refused: gate-path: task/T-001 changes gate path(s)
  xtask/src/main.rs; such a task must be tier high (it is low)`). The test contradicts R3.
  QUESTION: `refuses_unaccepted_states`, `refuses_missing_or_failed_join`,
  `refuses_missing_rejected_or_stale_review`, `refuses_high_without_approval_row`,
  `refusals_change_nothing` plant join records in an uncommitted `metrics/tasks.jsonl`
  (`support::Repo::set_tasks_jsonl`/`with_evidence`); R6/D-027 refuse exactly that at
  `ledger` (observed in each). They need the fixture to commit the record on main.
  · gate attempts: 1 (last: red test — 167 passed, 6 failed: exactly the six above; fmt,
  clippy, layers, ui-commands, schema, scorecard green; 3 later steps not run) · not run: the
  gate's steps after `test`; the new queue on the real repository (forbidden); `cargo update
  --workspace` with registry dependencies (offline) · missing context: none · next: director
  rules on the two QUESTIONs (fixture/test update by the test author), then join with the R1–R9
  tests and re-gate.
- 2026-11-13T03:10Z · R-041 · in_progress (round 5, after review r2) · did: D-030 R10–R13,
  one commit per ruling (8304edf R10+F18, plus R11/R13/F16, a0f0dc6 R12; 93 unit tests). R10:
  any `Cargo.toml` whose `[lints]` or package `build` key the task changes, and any file a
  `build` key names (head or main), is a gate path. F18 (my choice): a manifest value counts as
  changed by the task only if the head differs from main's **and** from the merge base's.
  R11: version changes of main's registry/git packages need tier high plus an approval cell
  containing `dependency versions` (same cell, case-sensitive); then they merge, the
  rebased-tree check also lets them through; conflict regeneration of `Cargo.lock` stays
  strict. R12: after every appended record, `git commit -- metrics/queue.jsonl` on main,
  subject `queue: record T-NNN <result>` + `Task:`/`Role: director`; on a merge it sits on top of
  the fast-forwarded head (main = head_sha + record commit), otherwise it is main's only change;
  the branch is still put back (R5); refusals write nothing; the record commit is a ledger
  commit so the next `main-history` tolerates it (needs `metrics/` in ledger.toml); a repo
  with no git identity gets `kiln queue <queue@kiln.invalid>`; `ledger` covers all of
  `metrics/`; main-history reads committed records only. R13: unit tests kill both mutations
  (prune+bail removed with future-dated fakes; rebased-tree check disabled). Residual risks
  listed in docs verbatim. F17 left: the clean-main check refuses untracked drafts anyway.
  QUESTION (for the test author, R12-inherent): these assert main == merged head, main unmoved
  after a non-merge, `log -1` is the regeneration commit, or an uncommitted queue.jsonl:
  acceptance_t005 — gate_runs_mains_xtask_and_records_it, generated_only_conflict_is_regenerated_and_merged,
  invalid_policy_files_are_refused, main_history_tolerates_only_ledger_commits,
  mixed_or_failed_regeneration_conflict_is_rejected, refusals_change_nothing,
  refuses_front_matter_changes_on_task_branch, refuses_high_without_approval_row,
  refuses_missing_or_failed_join, refuses_missing_rejected_or_stale_review,
  refuses_unaccepted_states, self_weakened_gate_fails_under_mains_gate; acceptance_t005_r3 —
  approval_rows_in_code_blocks_or_prose_do_not_count, dependency_and_lock_changes_need_tier_high,
  gate_paths_need_tier_high_and_are_named, gate_paths_on_a_high_task_need_a_gate_paths_approval,
  lock_version_change_is_refused_with_or_without_a_conflict,
  regeneration_that_changes_a_non_generated_path_is_not_merged,
  review_sha_must_be_a_full_ancestor_and_only_evidence_may_follow,
  tampered_cached_gate_binary_does_not_decide_the_gate, uncommitted_evidence_does_not_count;
  guard_t005_r2 — front_matter_may_lag_or_match_mains_state. With a temporary, uncommitted copy
  whose `main_sha()` skips record commits, all pass except the three that check `log -1`, a
  commit count, or the old exemption (copy deleted).
  · gate attempts: 1 (last: red test only — the R12-inherent failures above; fmt, clippy,
  layers, ui-commands, schema, scorecard green; 3 later steps not run) · not run: real queue on
  the main repository; registry `cargo update` · missing context: none · next: test author
  updates the fixture for record commits; join; re-gate.
<!-- end implementer log -->
### test author
- 2026-11-12 23:30 · R-010 · in_progress · did: 16 acceptance tests in
  `xtask/tests/acceptance_t005.rs` and 3 regression guards in `xtask/tests/guard_t005.rs` (D-010),
  from the spec only. All drive `CARGO_BIN_EXE_xtask` in toy repos under `CARGO_TARGET_TMPDIR`
  (toy cargo workspace, toy `xtask` whose `ci --report` fails iff `RED` exists, `scorecard` =
  sorted summary of `items/` (fails on `items/poison.toml`), `schema --write` = constant; offline,
  child env stripped of `CARGO_*`/`GIT_*`). Every refusal assertion checks exit 1, exactly one
  `queue: refused: <check>: ` line, the named path/commit, and the AC-9 snapshot (main, task
  head, both `git status --porcelain`, `metrics/queue.jsonl` bytes). A check that passes is
  observed as a refusal at the next check in the spec's order (e.g. medium → `join`) or a merge.
  Criterion → tests → result on base (all fail by assertion; none by compile):
  AC-1 `self_weakened_gate_fails_under_mains_gate` (merged, no `gate_xtask`);
  AC-2 `gate_runs_mains_xtask_and_records_it` (merged instead of refusal `touches`);
  AC-3 `refuses_unaccepted_states` (17 refused states across tiers incl. low `reviewed`, medium/high
  `self_verified`; 5 accepted; merged instead of refusal);
  AC-4 `refuses_front_matter_changes_on_task_branch` (tier/touches/state/id/scorecard_items);
  AC-5 `refuses_paths_outside_touches`, `allows_acceptance_scorecard_and_review_paths_for_medium_only`
  (naive predicate `allowed()` cross-checked per case);
  AC-6 `refuses_missing_or_failed_join`; AC-7 `refuses_missing_rejected_or_stale_review`;
  AC-8 `refuses_high_without_approval_row`; AC-9 `refusals_change_nothing` (one refusal per check,
  plus `task-file`: absent, two files, id mismatch, bad tier, no touches, only on branch) and the
  snapshot inside every refusal assertion; AC-12 `seed_reuses_non_member_artifacts_only` (member
  `.fingerprint`/`deps`/`incremental` seeded); AC-14 `generated_only_conflict_is_regenerated_and_merged`
  (no `regenerated` field); AC-15 `mixed_or_failed_regeneration_conflict_is_rejected` (poisoned
  regeneration is `conflict`/`rebase`, not `regenerate:docs/scorecard.md`); AC-16
  `generated_paths_are_exempt_from_touches_by_mains_list`; AC-17 `main_history_tolerates_only_ledger_commits`
  (7 non-ledger commit kinds, each also followed by a ledger commit); AC-18
  `invalid_policy_files_are_refused` (missing / not TOML / unknown root key / empty, each file).
  Guards (pass on base, kept because base's full independent `cp -a` copy already has the
  property and the new selective seed must keep it; a hard-link, symlink or shared-dir seed fails
  them): AC-10 `seeded_worktree_builds_its_own_source`, AC-11 `worktree_build_does_not_touch_main_target`
  (manifests both ways; no symlink and no inode shared with main's target), AC-13
  `revert_check_leaves_task_target_untouched`.
  Edge cases: `crates/ab` vs `crates/a`, `docs/notes.md.bak` vs exact entry, trailing `/`, deletion,
  rename (no rename detection), acceptance file in `tests/sub/` or outside `tests/`, `t008` file on
  T-007, `xtask/tests/` acceptance file, review rounds r2 vs r10 (numeric), non-round review file,
  join record for another task / other kind / nonexistent sha, approval text not at cell start or in
  another column, ledger add-only deletion, mixed ledger+code commit, two-commit regeneration replay.
  Feasibility of AC-12 checked by hand (copy of target minus member files and `incremental/`:
  `outsidedep` fresh, member rebuilt). · gate attempts: 1 (last: red test — exactly the 16
  acceptance tests; guards and all else green) · not run: anything against the new queue (not
  implemented on this branch) · missing context: none outside the repository.
  QUESTION: the TOML layout of `xtask/generated.toml` / `xtask/ledger.toml` is not specified.
  Tests copy the files this package ships (`CARGO_MANIFEST_DIR`) into each toy repo (a stand-in is
  used only while they do not exist, i.e. on base), and AC-16/AC-18 edit them textually
  (replace `docs/commands.json`; prepend `unknown_key_for_test = true`). Proposed: name the schema
  in the spec (e.g. `[[file]] path, command` / `[[path]] path, mode`).
  QUESTION: `front-matter` says "identical to main's", but the director changes `state` on main
  after the branch is cut, so a literal reading refuses every real task. Proposed: "the task branch
  does not change the front matter relative to the merge base". Fixtures sidestep it (worktrees cut
  after main's task file is final).
  QUESTION: the allowed-path rule has no entry for `guard_tNNN*.rs` (D-010), so guard files are
  refused at `touches`; and `cargo xtask join` finds acceptance targets only under `crates/` and
  `apps/`, so this task's `xtask/tests/acceptance_t005.rs` (the spec's location) would be
  `no_acceptance_tests`. Proposed: add `guard_tNNN*.rs` to the rule and `xtask/tests/` to join's search.
  Also unspecified (chosen readings): position of `config` among the checks (AC-18 keeps all other
  checks passing, no merged record); "naming the commit" = contains a ≥7-char sha prefix; under
  `gate_xtask:"main"` `failed_step` is main's step unprefixed (`red`); AC-10's dependency "outside
  the workspace" is outside the repository at an absolute path (inside the repo its package id
  differs per checkout, so it could never be fresh); refusal exit 1 (base exits 2 on errors).
- 2026-11-12 23:50 · R-019 · round 2 · did: (1) fixed my defect in
  `allows_acceptance_scorecard_and_review_paths_for_medium_only`: `crates/a/tests/acceptance_t008.rs`
  lies inside T-007's `touches: [crates/a]`, so it is allowed; the case is now
  `crates/b/tests/acceptance_t008.rs` (another task's acceptance file outside `touches`) → refused.
  (2) fixed a fixture bug: the child-environment scrub removed a `CARGO_TARGET_DIR` the test
  itself had set whenever the parent shell exported one, so those builds used cargo's default
  target (same directory for the AC-10/11/12/13 cases, so their results stood; AC-14's private
  regeneration target and the new private xtask build did not). Explicitly set variables are now kept.
  (3) moved the toy-repo fixture into `xtask/tests/acceptance_t005_support.rs` (an allowed path; as
  its own target it has 0 tests), included by `#[path]` from the T-005 files. (4) D-019 tests, new
  file `xtask/tests/acceptance_t005_r2.rs`, all fail on base by assertion: (c)
  `config_is_checked_just_before_touches` (broken config + outside path → `config`; + draft →
  `state`; + changed front matter → `front-matter`); (d) `guard_files_are_allowed_paths_for_medium_only`
  (`guard_t007*.rs` directly in `crates|apps/*/tests` and `xtask/tests` allowed for medium;
  other task's number, `tests/sub/`, outside `tests/`, `.txt` refused; low refused); (e)
  `join_finds_acceptance_and_guard_tests_under_xtask_tests` (acceptance + guard in `xtask/tests`
  → `joined`, `base_fail_kind:"fails"`, so a guard passing on base is not "passes"; a guard that
  fails after the join keeps the result from `joined` — INFERRED: "finds" means found guards run);
  (f) `xtask_refuses_a_checkout_it_was_not_built_from`: builds a private copy of this checkout's
  xtask (tracked files copied, read only; persistent private target under `CARGO_TARGET_TMPDIR`,
  ~12 s cold), runs it in a second `git worktree` of that copy with changed xtask source → exit 2,
  one stderr line naming both checkouts (also from a subdirectory); legitimate: own checkout and its
  subdirectory → exit 0; an unrelated scratch repository (how every T-005 test drives xtask) with
  the private binary and with `CARGO_BIN_EXE_xtask` → exit 0. Guard `xtask/tests/guard_t005_r2.rs`
  `front_matter_may_lag_or_match_mains_state` (ruling (a); passes on base, which has no
  front-matter check): branch copy unchanged since merge base while main advanced `state` →
  merged; branch copy synced to main's → merged. Rulings (a)–(c) contradict none of my R-010 tests
  (AC-4 changes front matter against both main and merge base; AC-18 keeps every other check passing).
  · gate attempts: 1 (last: red test; the gate's `test` step runs `cargo test --workspace` without
  `--no-fail-fast`, so it stops at the first failing target, `acceptance_t005`, and its count omits
  the later targets; run separately: acceptance_t005 16/16 fail, acceptance_t005_r2 4/4 fail,
  guard_t005 3/3 pass, guard_t005_r2 1/1 pass, all else green) · not run: the queue exception of (f)
  with the real Kiln gate (too heavy for a test: it would run the whole Kiln gate on a copy); it is
  exercised only in toy repos (AC-1/2/14) · missing context: none.
  QUESTION (f): the ruling does not say how "the checkout it was built from differs" is told apart
  from a legitimate run in an unrelated repository. My tests assume: another checkout (worktree) of
  the same repository → refused; a different repository → allowed. Proposed wording: "refuses when
  the workspace it runs in is a different checkout of the same repository (same git common
  directory) than the one it was built from".
- 2026-11-13 · R-033 · round 3 · did: D-026 tests, new file `xtask/tests/acceptance_t005_r3.rs`
  (fresh test author; earlier files unchanged). Each test pairs attempted bypasses with the
  legitimate case. All 11 fail on base by assertion (base merges, or lacks `gate_xtask`):
  R1 `tampered_cached_gate_binary_does_not_decide_the_gate` (every built `xtask` ELF under the
  scratch dir, incl. worktree targets, `.git`, TMPDIR, XDG_CACHE_HOME, overwritten by a fake green
  gate after a first run; RED task → `gate_failed`/`red`/`main`, sha = base; green task still
  merges; base is fooled and merges RED); R2 `review_sha_must_be_a_full_ancestor_and_only_evidence_may_follow`
  (same-tree unrelated branch, 12/7-char ids, `HEAD`, branch name, non-ancestor, merge base, code
  after/mixed into review, non-acceptance `tests/helper.rs` → `review`; review/log/generated/
  acceptance/guard commits → merged); R3 `gate_paths_need_tier_high_and_are_named` (11 gate-path
  edits incl. `.cargo` runner, `build.rs`, root lints/metadata/profile → `gate-path` naming the path,
  low and medium; ordinary root `Cargo.toml` change → merged `main`),
  `gate_paths_on_a_high_task_need_a_gate_paths_approval` (no row / no words / words only in
  Reasoning → `approval`; with words → merged `main+task`, path printed); R4
  `dependency_and_lock_changes_need_tier_high` (version req, dev/build/target deps,
  `[workspace.dependencies]`, lock-only edit, medium → `dependencies`; non-dep manifest change and
  approved high new package → merged); R5 `non_merged_outcomes_leave_the_task_branch_where_it_was`
  (gate_failed, ff_failed via a gate that commits to main, regenerate-failure conflict, refusal:
  branch ref unchanged, worktree clean, no rebase in progress); R6 `uncommitted_evidence_does_not_count`
  (untracked join record, modified/untracked metrics → `ledger` with commit hint; working-copy
  approval row, front matter, policy file → not merged, no effect; committed → merged); R7
  `approval_rows_in_code_blocks_or_prose_do_not_count` (```/~~~/indented/unclosed fence, prose,
  lone pipe line → `approval`; table after a closed fence → merged); R8 `run_in_variable_lets_nothing_through`
  (5 values: RED still `gate_failed` under `main`, weakened gate → `gate-path`, outside path →
  `touches`); R9 `lock_version_change_is_refused_with_or_without_a_conflict`,
  `regeneration_that_changes_a_non_generated_path_is_not_merged`. Validated on base with a
  throwaway lenient copy (all setups run; tamper fools base; race yields `ff_failed`).
  · gate attempts: 1 (last: red test, stops at `acceptance_t005`; fmt/clippy/others pass; run
  separately: r3 0/11, guards 4/4 pass) · not run: anything against the new queue · missing
  context: none.
  QUESTION: R6 vs the queue's own `metrics/queue.jsonl` append and the R-010/R-019 fixture
  (`with_evidence`/`set_tasks_jsonl` leave join records uncommitted): read literally, the 2nd queue
  run in a repo, and many earlier tests (AC-3, 6–9, 14…), now meet `ledger`. My tests commit metrics
  after every run, so they hold either way. Director: exempt the queue's own record, or have it
  committed, and rule on updating the earlier tests.
  QUESTION: R2 "test files" read as acceptance/guard files only (`crates/a/tests/helper.rs` after
  review refused); alternative: any file under a `tests/` dir.
  QUESTION: R7 lone `|` line with no header/delimiter row is not a table row (refused); alternative:
  any pipe line outside fences.
  QUESTION: R9 result for regeneration touching a non-generated path is unspecified; I assert only
  not merged and no effect. Proposed: `conflict`, `failed_step:"regenerate:<path>"`.
  QUESTION: the R4 lock rule literally refuses a workspace member's version bump; untested. Propose
  scoping it to non-member packages or saying so.
  Not tested: R8 with a legitimate run plus a stray variable (refusing it would also be acceptable);
  R1 caches outside the scratch dir (e.g. under $HOME) are not tampered; R3 case of `gate paths`.
- 2026-11-13 · R-034 · round 3 continued · did: applied the director's answers to my R-033
  questions. (1) R6: the fixture's `set_tasks_jsonl` (and so `with_evidence`) now commits
  `metrics/tasks.jsonl` on main. That is a setup change only; no assertion changed. Tests that needed it:
  `refuses_unaccepted_states`, `refuses_missing_or_failed_join`,
  `refuses_missing_rejected_or_stale_review`, `refuses_high_without_approval_row`,
  `refusals_change_nothing`. `uncommitted_evidence_does_not_count` now refuses at `ledger` for
  uncommitted `metrics/tasks.jsonl` (new or modified), `docs/decisions.md`, a task file, a primary
  `reviews/` file and each policy file. It no longer tests non-evidence metrics files, and it checks
  that an uncommitted `metrics/queue.jsonl` does not block a merge. (2) R2: another task's
  `acceptance_t002.rs` / `acceptance_t002_support.rs` after review → stale; own
  `acceptance_t001_support.rs` allowed. (3) R7 unchanged. (4) R9 leak: requires `conflict` with
  `failed_step` `regenerate:…`. (5) R4/R9 lock test now uses a registry package (offline
  directory source replacing crates.io): a lock-only bump → `dependencies` before and after main's
  lock moves; an approved workspace-member version bump → merged. All 11 r3 tests still fail on base by
  assertion; acceptance_t005 16/16 and r2 4/4 still fail by assertion; guards 4/4 pass; clippy/fmt
  clean. · gate attempts: 0 this run (R-033: red only at test) · missing context: none.
- 2026-11-13 · R-037 · round 3 continued · did: `self_weakened_gate_fails_under_mains_gate`
  (AC-1) setup only. Under R3(a) a low task's `xtask/` change is refused at `gate-path` before
  any gate runs, so T-001 is now a high task with committed join/review evidence and an
  `Approve T-001 including its gate paths.` row. Assertions are unchanged (`gate_failed`,
  `main:red`, `main+task`). It still fails on base by assertion (base merges). No other older test
  has this shape. Clippy and fmt pass. · gate attempts: 0 · missing context: none.
<!-- end test author log -->
### reviewer
- 2026-11-13 · R-028 · tests_joined (main) · did: review round 1 of 055821a → `reviews/T-005-r1.md`,
  verdict request-changes (defect 2, spec gap 4, test gap 2, nit 2). Gate green (81.8 s); toy-repo
  queue experiments E1–E6; mutation spot-check 17/20 killed (survivors: KILN_XTASK_RUN_IN unset,
  Cargo.lock version check off, regeneration stray check off). Defects: cached main xtask can be
  overwritten by gated task code and is reused by stamp (a failing test merged); symbolic
  `reviewed_sha` (e.g. `task/T-NNN`) makes the stale-review rule vacuous. SPEC GAPS for the
  director: main's gate obeys the task's `.cargo/`, toolchain, clippy and xtask data files (both
  gates pass a neutralised test runner); un-conflicted `Cargo.lock` changes bypass `touches` and the
  version rule; gate_failed leaves the branch rebased so join/review evidence stops matching;
  join/main-history evidence comes from uncommitted `metrics/` files. · gate attempts: 1 (last:
  green) · not run: queue on the real repository, registry `cargo update` · tokens: unknown ·
  missing context: none · next: director rules on F3, F4, F7, F8; implementer fixes F1, F2, F5, F6.
- 2026-11-13 · R-038 · tests_joined (main) · did: review round 2 of 8914907..ac9627f →
  `reviews/T-005-r2.md`, verdict request-changes (spec gap 3, test gap 2, nit 3). Gate green
  (192 tests). Re-ran round-1 exploits in toy repos: F1, F2, F4, F6, F7, F9, F10 resolved;
  F3, F5, F8 partly. Mutations on the new code 17/22 killed (N12 equivalent). Test gaps: removing
  the main-xtask prune (plus its bail) lets a future-dated tampered binary merge, and no test
  fails; both after-rebase lock checks can be removed with no failing test. SPEC GAPS for the
  director: member `[lints]` and `build = "…"` weaken the gate silently in a low task (both
  merged); R4 leaves no route for any dependency version change; the main-history anchor still
  comes from the uncommitted `queue.jsonl` (a fake line let a direct code commit through). ·
  gate attempts: 1 (last: green) · not run: real-repo queue, registry `cargo update` · tokens:
  unknown · missing context: none · next: test author adds F12/F14 tests; director rules on
  F11, F13, F15.
<!-- end reviewer log -->
