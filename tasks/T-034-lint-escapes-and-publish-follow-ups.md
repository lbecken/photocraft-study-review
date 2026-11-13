---
id: T-034
title: "Treat lint escapes correctly at the queue, and finish the publish helper's T-013 follow-ups: hand commands that work, exit codes stated once, a stop line for failing transitions, the owed tests, the dead ceiling-parser arm"
tier: high              # ADR-0002 "any gate path": xtask/src, xtask/tests; the approval row must contain `gate paths`
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-013]     # merged; T-005 transitively
touches:
  - xtask/src/precheck.rs        # GATE PATH: new-member lints (1a), ceiling parser (6)
  - xtask/src/queue.rs           # GATE PATH: lint-attribute listing lines and summary (1b)
  - xtask/src/publish.rs         # GATE PATH: hand commands (2), stop line (4), owed unit test (5)
  - xtask/tests/acceptance_t034.rs   # GATE PATH: test author
  - xtask/tests/guard_t034.rs        # GATE PATH: test author (guards for behaviour main already has)
  - docs/development.md          # "Merge queue" lint rules; "Publishing a ledger input" exit codes and "After a stop" rows (not a gate path)
budget_tokens: 850000
iteration: 2
found_by: reviewer
origin_task: T-013        # items (2)–(6): T-013 r2 follow-ups (D-053); item (1): T-005 r3 (D-033)
---

# T-034 Lint escapes at the queue; the publish helper's follow-ups

## Goal
Part C of R-109's T-009 (split approved, R-112 brief). After this task:
(1) a new workspace member that omits the workspace lints is a gate change; a lint
`allow`/`expect` attribute added in non-test source is listed for the reviewer; one added in a
test file is neither (T-005 r3, D-033; T-009 Decisions D-3);
(2) every `publish: commit failed:` line gives a hand command that works, also when `git add`
was what failed (T-013 r2 follow-up 2);
(3) the publish exit codes are stated once and correctly, and a failed content commit in step 4
is pinned at exit 1 (follow-up 3);
(4) a failing transition or state edit in publish steps 3, 6 or 8 ends with publish's own stop
line instead of a generic error (follow-up 4);
(5) the publish paths T-013 left untested have tests (follow-up 1);
(6) the dead `_` separator check in the ceiling parser is removed or explained (follow-up 5).

Item mapping from R-109's T-009: (3)→(1), (8)→(2), (9)→(3), (10)→(4), (11)→(5), (12)→(6).

**Gate paths.** Every `xtask/` path in `touches` is a gate path, so the task is tier high
(ADR-0002); the approval row must begin `Approve T-034` and contain `gate paths`. No dependency
change (the `syn` dependency xtask needs for attribute detection is already there). Per T-005
D-3, it must pass both main's gate and its own. If another `xtask/src` file must change, stop
and report.

**Dispatch.** Never in flight with T-009 (shares `precheck.rs`, `queue.rs`, `publish.rs`,
`docs/development.md`), T-033 (`docs/development.md`) or T-028 (`docs/development.md`). No
logical dependency on T-009 or T-033; if T-009 merges first, its publish exit-3 line is one of
the exits (3) states.

Reading list: `docs/decisions.md` D-026, D-030 (R10), D-033, D-047 (rulings j–q), D-053;
`reviews/T-005-r3.md` (lint findings); `reviews/T-013-r2.md` findings 1–5 and "Follow-ups";
`tasks/T-013-process-tooling-v11.md` Interface (5) and rulings j–q;
`xtask/src/{precheck,queue,publish,metrics}.rs`; `xtask/tests/acceptance_t013_publish.rs`
(the publish toy repository); `docs/development.md` "Merge queue", "Publishing a ledger input"
and "After a stop".

## Interface
Existing commands keep their names, options, refusal formats, record formats and precondition
order unless a change is listed here.

### (1) Lint escapes (T-009 D-3)
- (a) **Gate path.** Already on main (D-030 R10, `precheck.rs` `manifest_gate_keys`; unit test
  `queue.rs` `member_lints_and_custom_build_scripts_are_gate_paths`): a member `Cargo.toml`
  whose `[lints]` table or package `build` key differs from both main's and the merge base's
  copy is a gate path, so an existing member that drops `[lints] workspace = true` is refused
  below tier high. **New:** a member `Cargo.toml` that is new at the task head (absent on the
  merge base) and has no `[lints]` table with exactly `workspace = true` is a gate path too
  (today an absent `[lints]` compares equal to an absent file). Nothing that is a gate path
  today stops being one.
- (b) **Listed, not refused.** A `.rs` file outside every test directory in which the task
  **adds** a lint-suppressing attribute (`allow` or `expect`, at item or crate level, `#[…]` or
  `#![…]`, including inside `cfg_attr(…)`) is not a gate path. The queue prints, for each added
  attribute, one stderr line `queue: T-NNN adds a lint attribute in non-test source: <path>:<line>: <attribute text>`
  and lists them in its summary as `lint attributes added: <path>:<line>, …`. It does not refuse.
  The record format is unchanged.
- (c) **Test files.** An attribute added in a `.rs` file under a `tests/` directory directly
  below a package root (`crates/*/tests/**`, `apps/*/tests/**`, `xtask/tests/**`) is neither a
  gate change nor listed.
- "Added" compares the merge base with the task head: attributes that exist on the merge base do
  not count; nor do moved lines whose attribute text is unchanged (how moves are detected is left
  to the implementer, as long as an added attribute is never missed).

### (2) Publish hand commands that work (follow-up 2)
When a ledger commit of publish fails (steps 2, 3, 6 and 8; a failing `git add` included,
ruling D-047 (l)), the one stderr line becomes
`publish: commit failed: <git error>; stage by hand: git add -- <paths>; commit by hand: git commit -m "<subject>" -m "Task: T-NNN" -m "Role: director" -- <paths>`,
where `<paths>` is the same list as today, in the same order, in both commands. Today the line
has no `stage by hand` clause, so after a failed `git add` the printed `git commit -- <paths>`
names untracked files and fails with `pathspec … did not match` (R-097 toy p3). Exit code (2)
and everything else are unchanged. The line still starts with `publish: commit failed: ` and
still ends with `; commit by hand: git commit -m … -- <paths>`, so T-013's two acceptance tests
that check exactly that prefix and suffix (`acceptance_t013_publish.rs`:660, :909) stay green
unmodified.

### (3) Publish exit codes, stated once (follow-up 3)
`docs/development.md` and this task state: refusal 1; usage error 2; a failed **ledger** commit
(steps 2, 3, 6, 8) 2 with the line of (2); a stop in step 3, 4 or 7 — a failed content commit
on `task/T-NNN` in step 4 included — 1 with `publish: stopped at step <n>: <error>; T-NNN is <state>`
(ruling D-047 (q)); a stop of (4) 1; the gate's red line 1; step 8 the queue's code (and, once
T-009 is merged, its exit-3 line). This is today's behaviour (R-097 confirmed it); T-013's
Pinned (a) wording "publish commit failure 2" is superseded by this sentence, and T-013's
merged task file is not edited.

### (4) A failing transition or state edit in steps 3, 6, 8 (follow-up 4)
If, in publish step 3, 6 or 8, a `metrics/tasks.jsonl` transition or the edit of the task
file's `state:` fails (for example a transition refused by the time rules of `metrics`), publish
prints exactly one line of its own,
`publish: stopped at step <n>: <error>; T-NNN is <state>`, where `<state>` is the task's last
transition in the working tree's `metrics/tasks.jsonl` (as for the existing stop line), and
exits 1. Whatever that step already appended stays in the working tree, uncommitted; nothing
is reset. Today such a failure ends with a generic error and no stop line (R-097 finding 4).
`docs/development.md` "After a stop" gains one row per step ("3/6/8, a transition or the state
edit failed": left behind, finish by hand: complete the step's remaining transitions with
`metrics transition`, set the state, ledger commit with the step's subject).

### (5) Owed publish tests (follow-up 1)
No behaviour change beyond (2): the step-2 undo with an existing `metrics/tasks.jsonl` gets a
unit test (it cannot be reached through the command line: ruling D-047 (q)); the ledger commits
of steps 3 and 8 get acceptance tests of their failure line (AC-3).

### (6) Ceiling parser (follow-up 5)
`precheck.rs` `extension_of`: the `b'_'` arm of the separator check is unreachable (a digit run
followed by `_` is already rejected by the id-character test). Remove it or say so in a comment.
No behaviour change; T-013's ruling-j tests stay green unmodified.

## Pinned
- (a) **Errors and precedence.** Queue precondition order unchanged; (1a) is part of
  `gate-path`. Queue exit codes unchanged (listing lines never refuse). Publish exit codes: (3).
  The exact lines are in (1b), (2), (4).
- (b) **Files.** As T-005/T-013; no new file or record field. Tests:
  `xtask/tests/acceptance_t034.rs`, `xtask/tests/guard_t034.rs`.
- (c) **Order of checks.** Within `gate-path`, (1a) is one more reason a manifest is a gate
  path; listing (1b) happens only for a run that is not refused before the merge.
- (d) **Paths and platform.** Repository-relative paths; test directories as in (1c); Linux,
  git ≥ 2.38.
- (e) **Threat model.** D-026: honest mistakes and corner-cutting. Accepted: a lint attribute
  added in non-test source merges (it is only listed, D-3).
- (f) **Tests pinning behaviour this changes.** Every T-005 and T-013 acceptance and guard test
  must pass unmodified: the two publish tests at `acceptance_t013_publish.rs`:660 and :909 check
  only the prefix and suffix (2) keeps; `publish_refuses_a_checkout_dirty_outside_the_inputs_and_ledger_paths`
  is untouched by this task. Unit tests in `xtask/src` that pin the old publish lines change
  with the code (disclose in the log).
- (g) **Worked examples.** (2): step 2 with `.git/index.lock` present on a repository whose
  `metrics/tasks.jsonl` is tracked → `publish: commit failed: fatal: Unable to create '…/.git/index.lock': File exists.; …; stage by hand: git add -- tasks/T-050-publish-publish-x.md metrics/tasks.jsonl; commit by hand: git commit -m "publish: T-050 ready" -m "Task: T-050" -m "Role: director" -- tasks/T-050-publish-publish-x.md metrics/tasks.jsonl`
  (title `Publish x`; the git error and file name as R-097 toy p3 recorded them; not
  executed). (1a): a new `crates/z/Cargo.toml` with `[package] name = "z"` only, on a
  medium task → refused at `gate-path` naming `crates/z/Cargo.toml` (today: not a gate path,
  read from `manifest_gate_keys`).
- (h) **Scorecard items.** None.

## Acceptance criteria
In throw-away git repositories that the tests build (as T-005's and T-013's acceptance tests
do, the publish cases in T-013's publish toy repository), with `GIT_CONFIG_GLOBAL`/`HOME`
isolated; all through the `xtask` binary. Oracle: exact value unless stated. Each AC says
whether it is an **acceptance** test (`acceptance_t034.rs`, fails on base) or a **guard**
(`guard_t034.rs`, passes on base). No test pins a whole document's bytes or a count of rows,
records or kinds that another task changes (D-043, D-051, D-052).

- AC-1 (lint attributes): acceptance — a medium-tier task that adds a member crate whose
  `Cargo.toml` has no `[lints]` table → refused at `gate-path` naming that `Cargo.toml`; the same
  on a high task with a `gate paths` approval merges; a medium task that adds
  `#![allow(clippy::unwrap_used)]` to `crates/a/src/lib.rs`, `#[cfg_attr(test, allow(dead_code))]`
  and `#[expect(unused)]` to another source file → merges, and stderr has exactly one listing
  line per attribute with the right `<path>:<line>`; the summary lists them. Guard — a medium
  task that removes `[lints] workspace = true` from an existing member → refused at
  `gate-path` naming it (D-030 R10, already on main); a medium task that adds
  `#![allow(dead_code)]` to `crates/a/tests/support.rs` → merges with no listing line (no
  listing exists on base, so this half passes there); editing other lines of a file whose
  `#[allow]` already exists on the merge base → no listing line.
- AC-2 (step-4 content commit; guard): a hook that rejects commits on `task/*` branches makes
  step 4's content commit fail → exit 1, one line starting `publish: stopped at step 4: `, and
  no line starting `publish: commit failed: ` (ruling D-047 (q), already on main).
- AC-3 (publish ledger-commit failures; acceptance): a `commit-msg` hook on main that rejects
  exactly one subject (`publish: T-050 ready`, then in separate runs `publish: T-050 in
  progress`, `publish: T-050 queued`, `publish: T-050 merged`; the queue's own record commit
  uses `--no-verify` and is not affected) → exit 2 and one stderr line in the form of (2) with
  that subject and the step's paths; nothing of a later step happens. And with
  `.git/index.lock` present at step 2 (as T-013's test does): the line of (2), and after the
  test removes the lock, running the printed `stage by hand` command and then the `commit by
  hand` command (each through `sh -c` from the repository root) creates the
  `publish: T-050 ready` commit with exactly the two paths.
- AC-4 (stop line for a failing transition; acceptance): the toy gate (step 5) appends to the
  primary checkout's `metrics/tasks.jsonl` a well-formed T-050 `run` record whose `ended_at`
  is one day after now (a `run`, so the last transition stays `in_progress`), so step 6's
  first transition is refused by the time rules (a time earlier than the task's latest
  record) → exit 1; stderr has exactly one line that starts with `publish: stopped at step 6: `,
  and that line ends with `; T-050 is in_progress` (the `<error>` text is not pinned); no
  stderr line starts with `xtask: ` or `metrics: `; main's committed task file says
  `state: in_progress`.
- AC-5 (no regression; guard by nature): every T-005 and T-013 acceptance and guard test
  passes unmodified (T-013's ruling-j tests cover (6)); main's gate and the task's own gate are
  both green.

## Out of scope and implementer notes
- Queue binding, record-commit failure, untracked drafts, approval near-miss, publish's exit-3
  line: T-009. Join counts and scorecard at join: T-033.
- Whether a new member *with* exactly `[lints] workspace = true` (and no `build` key) should
  stop being a gate change (today it is one, because its `[lints]` differs from an absent
  file): not changed here (Questions).
- Any product crate, `Cargo.toml`, `Cargo.lock`, `.cargo/`, `.github/`, `methodology/`, and
  T-013's task file.
- Your tests: unit tests in `xtask/src` (toy-repository helpers in `git.rs` `testrepo`),
  including (5)'s step-2 undo with a committed `metrics/tasks.jsonl` whose bytes must be
  restored (R-097 mutant N6 must be killed). Acceptance and guard files of T-005 and T-013 are
  not edited.
- Specified: the rules, messages and exit codes above. Left to you: how attributes are detected
  (`syn` is available), how moves are recognised.
- Never set or export `CARGO_TARGET_DIR` in commands you run by hand (D-019).

## Questions and rulings
- Q-1 (planner, R-109, moved from T-009 Q-2): should a new member with exactly
  `[lints] workspace = true` (and no `build` key) stop being a gate change? Not specified; it
  would relax a gate rule.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13T12:51Z · R-112 · (new) · did: created from part C of R-109's T-009 (items (3),
  (8)–(12), AC-7, AC-12–14; split approved by the director); added a (2) worked example and
  excluded a `metrics:` line in AC-4 (base may print the metrics refusal itself). Tier high:
  gate paths (`xtask/src/{precheck,queue,publish}.rs`, `xtask/tests`). Pinned (h) n/a. Not in
  flight with T-009, T-033, T-028. · gate attempts: 0 · not run: no cargo; worked examples not
  executed · missing context: none · next: director answers Q-1; spec check.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
