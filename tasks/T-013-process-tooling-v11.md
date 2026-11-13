---
id: T-013
title: "Make the v1.1 process rules mechanical: clock-stamped and validated metrics, budget refusal in the queue, ready-only worktrees, corrected metric report, one-command publish, review and session-path gate steps"
tier: high              # ADR-0002 "any gate path": xtask/** (docs/development.md holds the list but is not itself one); approval row must contain `gate paths`
state: queued
owner_role: planner
scorecard_items: []     # tooling; no product scorecard item
depends_on: [T-005, T-014]   # and ADR-0003 (hygiene gate step) `accepted` on main before `ready` (Decisions D-2)
touches:
  - xtask/src                                  # GATE PATH: metrics, worktree, queue/precheck, join messages, gate steps, publish
  - xtask/tests/acceptance_t005_support.rs     # GATE PATH: test author's fixture change only (Decisions D-1)
  - docs/CHARTER.md                            # §4, the one line that lists the gate steps (Decisions D-2, ADR-0003)
  - docs/development.md                        # not a gate path (it documents the list); runbook text
  - reviews/T-003-r1.md                        # redaction of one session scratch path (line 39), nothing else
  - reviews/T-005-r1.md                        # redaction (line 15)
  - reviews/T-005-r2.md                        # redaction (line 18)
  - reviews/T-005-r3.md                        # redaction (line 17)
budget_tokens: 850000
iteration: 1
found_by: planner
origin_task: null
---

# T-013 Make the v1.1 process rules mechanical

## Goal
Methodology v1.1 (D-035, D-036) names eleven rules that are "interim: director by hand" until
this task lands (METHODOLOGY §2.1, §3 rework rule, §5.1, §7, §11, §12, §13, §16 row 3). In
iteration 0, 14 director errors were caught by no mechanical check (retro-0, "The director as a
failure source"). After this task:

1. `cargo xtask metrics add|transition` take the time from the clock, refuse a time earlier than
   the task's last record or in the future, refuse a transition that METHODOLOGY §3 does not
   allow, refuse `--tokens 0` unless the run's usage is declared unknown, print the task's
   cumulative tokens against its budget, and book planner, evaluator, retrospective-analyst and
   director runs to a pseudo-task `iter-N`.
2. `cargo xtask queue T-NNN` refuses a task whose recorded tokens exceed `budget_tokens` unless
   main's `docs/decisions.md` has a decision row `Extend T-NNN to <ceiling>` with a ceiling the
   tokens do not exceed (METHODOLOGY §11, P7; Decisions D-4).
3. `cargo xtask worktree new` refuses a task that is not `ready` on `main` (§5.1).
4. `cargo xtask metrics report` computes the v1.1 metric definitions (§13): acceptance-caught
   defects over every join, with unclassified failing joins reported as an error; tokens per
   merged task with and without `iter-N` overhead; known defects merged by decision; and a new
   `metrics join-defects` appends the director's classification of a failing join as a new
   record (Decisions D-3).
5. `cargo xtask publish T-NNN …` carries a ledger input that changes generated output (a new
   scorecard item, an evaluation file) to `main` as a low-tier publish task in one command
   (§8, D-034, D-036), so the director no longer writes and accepts such a spec by hand (T-012).
6. The gate gains one step `hygiene` with two checks (ADR-0003): every review of a medium/high
   iteration-≥1 task has the v1.1 `mutants` front matter, and no tracked file contains a session-specific
   scratch path, §12).
7. The v1.0 references listed in `methodology/CHANGELOG.md` v1.1 under "Known stale references,
   fixed by T-013" are corrected in `docs/development.md` and `xtask` messages.

**Gate paths.** `xtask/src` and `xtask/tests/acceptance_t005_support.rs` are gate paths (anything
under `xtask/`), so the task is tier high. `docs/development.md` is where the gate-path list is
documented; the queue does not treat it as a gate path. The director's approval row in `docs/decisions.md`
must begin `Approve T-013` and contain the words `gate paths`, or the queue refuses it (T-005
R3). The task changes no dependency: no `Cargo.toml`, `Cargo.lock`, lint table or build key.
Per T-005 D-3 the queue runs both main's gate and the task's own gate; both must pass.

The join's acceptance-test count across crates (T-004: `acc_total` 6 against `acc_fail_base` 13,
`metrics/tasks.jsonl` record at 2026-11-13T01:18:27Z) is **not** in this task: it stays in T-009
(6), which already pins it (T-009 AC-9). Reason: it is a defect of the `join` command's record
producer in `xtask/src/join.rs`, part of the queue-integrity set D-035 leaves with T-009; this
task only consumes join records. Putting it in one place avoids two implementations of the
per-package target identity.

Reading list: `methodology/METHODOLOGY.md` §2.1, §3, §5, §7, §8, §11, §12, §13, §16;
`methodology/CHANGELOG.md` v1.1 (changes 3, 5, 12, 13, 14, 15 and the stale-reference list);
`retrospectives/iter-0.md` "The director as a failure source", "Tooling and environment",
"Proposed changes" 3, 5, 11, 13, 14, 15; `docs/decisions.md` D-012, D-026 (threat model,
R7 table rows, R8 no bypass flag), D-030 R12, D-034, D-035, D-036; `docs/development.md`;
`methodology/templates/review.md`; `tasks/T-012-publish-iter0-evaluation.md` and
`tasks/T-014-publish-v11-entry-docs.md` (what a publish task looks like);
`xtask/src/{main,metrics,worktree,precheck,queue,join,gate,frontmatter,timefmt}.rs`.

## Interface
Every existing command keeps its name, options, output and exit codes unless a change is listed
here. Refusals below are **one stderr line, exit 1, nothing written or appended**; usage errors
(unknown option, missing value) stay exit 2 as today. `<now>` is the system clock in UTC,
second precision, formatted `YYYY-MM-DDTHH:MM:SSZ`. There is no option or environment variable
that replaces the clock or relaxes a refusal (D-026 R8).

### (1) `metrics add` and `metrics transition`

```text
cargo xtask metrics add --task <T-NNN|iter-N> --run R-NNN --role <role>
    (--tokens N | --tokens-unknown) --tool-calls N --gate-attempts N --outcome <outcome>
    --started-at <ts> [--ended-at <ts>] [--wall-s N] [--iteration N] [--tier T]
    [--missing-context S] [--note S]
cargo xtask metrics transition --task T-NNN --to <state> [--from <state>] [--at <ts>] [--iteration N]
cargo xtask metrics join-defects --task T-NNN --joined-sha <40-hex> --acc-defects N
```

Record fields stay the METHODOLOGY §13 names. Changes:

- **Time.** `add`: `--ended-at` defaults to `<now>`; `--started-at` stays required.
  `transition`: `--at` defaults to `<now>`. A record's **time** is `ended_at` for a run, `at` for
  a transition or join. Refused: a time later than `<now>`; a `started_at` later than `<now>` or
  later than `ended_at` (as today); a time earlier than the latest time among the records of the
  same `task` already in `metrics/tasks.jsonl` (equal is allowed).
- **Tokens.** Exactly one of `--tokens N` with `N ≥ 1`, or `--tokens-unknown`, which records
  `"tokens": null`. `--tokens 0` alone, both options together, or neither is refused.
- **iter-N.** `--task iter-N` (N a decimal number without leading zeros) books a run to
  iteration N's overhead: record `task: "iter-N"`, `iteration: N` (`--iteration`, if given, must
  equal N), `tier: null`. Roles allowed for `iter-N`: `director`, `planner`, `evaluator`,
  `retrospective-analyst`. Refused: a builder role (`implementer`, `test-author`, `reviewer`) or
  `integrator` on `iter-N`; and a `director`, `planner`, `evaluator` or `retrospective-analyst`
  run booked to a `T-NNN` (METHODOLOGY §13: these runs are overhead; T-012's false overrun).
  `transition` and `join-defects` on `iter-N` are refused.
- **Task file.** For `T-NNN`, exactly one `tasks/T-NNN-*.md` must exist in the working tree
  (it supplies tier, iteration and `budget_tokens` as today); otherwise refused.
- **Transitions.** The task's current state is the `to` of its latest `transition` record, or
  `draft` if it has none. `--from`, if given, must equal it. `(current, --to)` must be in this
  table (METHODOLOGY §3), with the tier taken from the task file's front matter:

  | from | allowed `to` |
  |---|---|
  | `draft` | `specified` |
  | `specified` | `ready`, `draft` |
  | `ready` | `in_progress` |
  | `in_progress` | `self_verified` |
  | `self_verified` | low: `queued`; medium/high: `tests_joined`, `in_progress` |
  | `tests_joined` | medium/high: `reviewed`, `in_progress` |
  | `reviewed` | medium/high: `queued` |
  | `queued` | `merged`, `in_progress` |
  | `merged` | `evaluated` |
  | any of the above except `merged` | also `blocked`, `abandoned` |
  | `blocked` | the `from` of the transition that entered `blocked`; or `abandoned` |
  | `evaluated`, `abandoned` | nothing (terminal) |

- **Budget line.** After a successful `add` for `T-NNN`, stdout ends with exactly:
  `T-NNN: <sum> tokens of budget <budget_tokens>` where `<sum>` is the sum of known `tokens`
  over every `run` record of the task (the new one included), then, only if `<sum>` exceeds the
  ceiling (`budget_tokens`, or the ceiling of the task's last `Extend` row in `docs/decisions.md`
  of the working tree), a line `T-NNN: over its ceiling <ceiling> by <sum - ceiling> tokens; the queue refuses it without a docs/decisions.md row "Extend T-NNN to <N>" with N ≥ <sum>`,
  then, only if some run has `tokens: null`, `T-NNN: <k> run(s) with unknown tokens not counted`.
  For `iter-N`: `iter-N: <sum> tokens of overhead`. Numbers are plain decimal integers.
- **join-defects** (Decisions D-3). Appends one record
  `{"kind":"join_defects","task":"T-NNN","at":"<now>","joined_sha":"<sha>","acc_defects":N}`;
  no existing record is ever edited. Refused: no `join` record of that task with that
  `joined_sha`, more than one, its own `acc_defects` already set, an earlier `join_defects`
  record for that join, `acc_fail_first_join` is 0, or `N > acc_fail_first_join`. A join's
  **classification** is its own `acc_defects` if set, else the `join_defects` record for it.
  The new kind is documented in `docs/development.md`; adding it to METHODOLOGY §13's record
  list is for retrospective 1 (methodology changes only there).
- Refusal lines begin `metrics: refused: ` and name the rule, for example
  `metrics: refused: T-007 is in_progress (since 2026-11-14T10:00:00Z); in_progress -> tests_joined is not allowed (METHODOLOGY §3)`.

### (2) Queue precondition `budget`
A new precondition after `approval` and before `main-history` (order otherwise unchanged):

| check | passes when |
|---|---|
| `budget` | the sum `S` of known `tokens` over the `kind:"run"` records of the task in **main's committed** `metrics/tasks.jsonl` is at most the task's **ceiling**: the ceiling `N` of the last (in table order) decision-table row of main's `docs/decisions.md` whose Decision cell begins with `Extend T-NNN to N` (`N` decimal digits only, followed by the cell's end or a character that is neither a digit nor an id character), else main's task-file `budget_tokens`. Rows are recognised exactly as for `approval` (D-026 R7: real Markdown table rows only); a cell `Extend T-NNN` without `to <digits>` is not an extension. A task file without `budget_tokens` fails |

Refusal: `queue: refused: budget: T-NNN has <S> recorded tokens, over its ceiling <ceiling> (<source>); a docs/decisions.md row "Extend T-NNN to <N>" with N ≥ <S> is needed`,
where `<source>` is `budget_tokens` or `Extend row`.
Runs with `tokens: null` count 0; when any exist, the queue's summary says how many.

### (3) `worktree new`
`worktree new T-NNN`, `worktree new T-NNN --tests` and `worktree new T-NNN-tests` first read
the task's state from **main's committed tree** (`tasks/T-NNN-*.md` on branch `main`, never the
working copy). Refused unless it is `ready`:
`worktree: refused: T-NNN is <state> on main; worktree new needs ready` or
`worktree: refused: no tasks/T-NNN-*.md on main`. When main's tree has no `tasks/` directory at
all (repositories that do not track tasks, such as the seeding guards' toy repositories), no
state is checked. `worktree new` no longer prints an `export CARGO_TARGET_DIR=…` line (item 7).

### (4) `metrics report`
Keeps today's sections and adds or changes these (METHODOLOGY §13 definitions):
- **Arm** of a task: `independent` for high, and for medium with an even task number;
  `implementer` for medium with an odd number; none for low.
- **Acceptance-caught defects**: Σ of the classification ((1) join-defects) over every `join` record with
  `impl_tests_green: true` (not the first join only), total and by tier and arm.
- **Unclassified joins**: every `join` record of a task whose task file has `iteration ≥ 1`
  with `acc_fail_first_join > 0` and no classification (own `acc_defects` null and no
  `join_defects` record for it), one line each
  (`unclassified join: T-NNN at <at> joined_sha <sha>: <k> failure(s), acc_defects missing`).
  If there is any, the report is still printed in full and the command exits 1.
- **Tokens per merged task** (merged = has a `merged` record in `metrics/queue.jsonl`): Σ known
  run tokens of merged tasks ÷ number of merged tasks, by tier and arm (*without overhead*); and
  per iteration N, (Σ known tokens of merged tasks of iteration N + Σ known tokens of `iter-N`
  runs) ÷ merged tasks of iteration N (*with overhead*). Runs with `tokens: null` are excluded
  and counted on the same line.
- **Known defects merged by decision**: task files with `found_by: reviewer` and an
  `origin_task` that has a `merged` queue record, listed as `T-NNN (from T-MMM)` with a count,
  under that heading. This counts follow-up tasks filed from a review of a merged task; it does
  not separate defect from spec-gap follow-ups (see log, uncertainty).
- **Budget overruns**: each task over `budget_tokens`, with `extend to: <N>|none` (its last
  `Extend T-NNN to <N>` row in the working tree's `docs/decisions.md`) and `within ceiling: yes|no`.
- `iter-N` runs appear in tokens by tier as tier `overhead`, never in a task's total.

### (5) `cargo xtask publish`
```text
cargo xtask publish T-NNN --title "<title>" --iteration N <path>...
```
Run in the primary checkout with `main` checked out. Inputs are paths in the working tree that
are either a **new** `scorecard/items/<id>.toml` (absent from main's tree) or a new or modified
`evaluations/*.md`. Steps, in this order; each step's failure stops the command:

1. **Checks** (nothing changes; refusal `publish: refused: <reason>`, exit 1): `main` checked
   out; `T-NNN` well formed; no `tasks/T-NNN-*.md` on main or in the working tree, no branch
   `task/T-NNN`, no `.worktrees/T-NNN`; at least one input; every input is of an allowed kind and
   differs from main's tree; apart from the inputs, the primary checkout passes the checks the
   queue applies to it before its preconditions (step 1's working-tree rule and `ledger`);
   regenerating `docs/scorecard.md` from the working tree with the inputs present would change
   it (else: `publish: refused: the inputs change no generated file; commit them on main as a ledger commit`),
   and the scorecard accepts the inputs (else the scorecard's own error, prefixed
   `publish: refused: `). The primary checkout's `docs/scorecard.md` is not written.
2. **Task file on main.** Writes `tasks/T-NNN-<slug>.md` (`<slug>` = `publish-` + the title in
   lowercase with every run of non-alphanumerics replaced by `-`, trimmed of `-`, cut to 40
   characters) with front matter exactly:
   `id: T-NNN`, `title: "<title>"`, `tier: low`, `state: ready`, `owner_role: director`,
   `scorecard_items: []`, `depends_on: []`, `touches:` the inputs in byte order,
   `budget_tokens: 150000`, `iteration: N`, `found_by: planner`, `origin_task: null`; and a body
   with the template's section headings, a fixed Goal ("publish <inputs> and the regenerated
   `docs/scorecard.md`"), the three ACs of T-012 (inputs byte-identical; scorecard regenerated
   and gate green; no other path), and a director log line `- <now> · created by cargo xtask publish`.
   Records transitions `draft → specified` and `specified → ready` at `<now>` (rules of (1)).
   Commits exactly that task file and `metrics/tasks.jsonl` on `main`, subject
   `publish: T-NNN ready`, body lines `Task: T-NNN` and `Role: director`, with the repository's
   commit configuration (signing included, no bypass). If the commit fails: exit 2 and one line
   `publish: commit failed: <git error>; commit by hand: git commit -m "publish: T-NNN ready" -m "Task: T-NNN" -m "Role: director" -- tasks/T-NNN-<slug>.md metrics/tasks.jsonl`, then nothing else.
3. **Branch.** Creates the worktree as `worktree new T-NNN` does (now allowed: `ready`);
   transition `ready → in_progress`.
4. **Content.** Copies each input byte for byte into the worktree, regenerates
   `docs/scorecard.md` there with the `scorecard` command, and commits on `task/T-NNN` exactly the
   inputs and `docs/scorecard.md`, subject `T-NNN: publish <inputs>; regenerate docs/scorecard.md`,
   lines `Task: T-NNN` / `Role: director`. If the commit would contain any other path: stop,
   exit 1.
5. **Gate.** `ci` in the worktree. Red: exit 1, line
   `publish: gate failed in .worktrees/T-NNN at <step>; T-NNN stays in_progress; fix the input and re-run the gate there, or abandon (metrics transition --to abandoned; worktree rm T-NNN --force)`.
   The primary checkout's inputs are untouched.
6. **Queue state.** Transitions `in_progress → self_verified → queued`; sets `state: queued` in
   main's task file; commits that file and `metrics/tasks.jsonl` on main (`publish: T-NNN queued`,
   same lines and failure rule as step 2).
7. **Hand-over.** For each input, checks that the primary checkout's bytes equal the committed
   branch copy, then removes it from the primary checkout (an untracked file is deleted, a
   modified tracked file restored to main's version), printing `publish: <path> moved to task/T-NNN`.
   This is what lets the queue's working-tree check pass and its fast-forward bring the file back.
8. **Queue.** Runs the queue for `T-NNN` exactly as `cargo xtask queue T-NNN` (its output passes
   through). Exit 0: transition `queued → merged`, `state: merged` in main's task file, ledger
   commit `publish: T-NNN merged`, `worktree rm T-NNN`; exit 0. Any other exit code `c`: one line
   `publish: queue did not merge T-NNN (exit c); the task stays queued; the inputs are on task/T-NNN (git show task/T-NNN:<path>)`
   and exit `c`.

The helper records no `run`. What the director still does by hand: reserve a free task number
and give the title and iteration; have the input written by its author (the helper never writes
an evaluation or an item); book the session's own run to `iter-N`; act on any stop message
(fix and re-run the gate, re-queue, or abandon); file the evaluator's defect tasks as separate
ledger commits; record the session in `STATE.md` and the handoff.

### (6) Gate step `hygiene` (Decisions D-2, ADR-0003)
One step, run by `cargo xtask ci` and alone as `cargo xtask hygiene`, placed after
`ui-commands` and before `schema` (cheap first). It runs both checks below and fails if either
finds a problem; its detail line names the check (`reviews:` or `session-paths:`) of each
problem. The CHARTER §4 gate line gains `hygiene` in that position (this task edits that one
line of `docs/CHARTER.md`, as ADR-0003 decides).

- Check `reviews`: for every file `reviews/T-NNN-rN.md` (other names under `reviews/` are ignored)
  whose task file `tasks/T-NNN-*.md` has tier `medium` or `high` and `iteration ≥ 1`, the front
  matter must parse and have: `task: T-NNN` and `round: N` matching the file name; `tier` equal
  to the task file's tier; `arm` equal to the task's arm ((4)); `mutants` with the five keys
  `introduced`, `equivalent`, `killed`, `killed_by_acceptance`, `survived`, each a non-negative
  integer, with `equivalent + killed + survived = introduced` and
  `killed_by_acceptance ≤ killed`; and in round 1, `introduced ≥ 10`. A review without a task
  file fails. FAIL detail: one `reviews/<file>: <problem>` per problem. Iteration-0 reviews
  (v1.0 template) are not checked. (The step is never EMPTY: the second check always runs.)
- Check `session-paths`: no file listed by `git ls-files` in the checkout being gated contains a match
  of the regular expression
  `/tmp/claude-[0-9]+/[^/[:space:]]+/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}`.
  Files that are not valid UTF-8 are skipped. FAIL detail: `<path>:<line>` per match, never the
  matched text. Before this step can pass on `main`, the four existing matches are redacted
  (Pinned (f)).

### (7) Stale v1.0 references
Each of these says what v1.1 says instead (the reviewer checks the text):
- `docs/development.md` "Adding a command" step 4 (line 136–137, "Command schema changes are
  high tier"): adding a command is medium unless a boundary applies; changing how the registry,
  schema validation or the command log work is a CHARTER §3 contract change, high (ADR-0002).
- `:144` "(METHODOLOGY O-2)": cite METHODOLOGY §3 instead (O-2 is closed).
- `:149` and `xtask/src/main.rs:180`: no `export CARGO_TARGET_DIR` is printed or recommended;
  `worktree new` prints the `cd` line and that cargo uses `<worktree>/target` by itself; the text
  says never to set `CARGO_TARGET_DIR` (AGENTS.md, D-019).
- `:326–352`, `xtask/src/join.rs:1` and `xtask/src/main.rs:294`: the revert check and the
  stub/logic commit split are not part of the method; weak base evidence (`does_not_compile`) is
  covered by the reviewer's mutation spot-check (METHODOLOGY §6). The join's stderr line for weak
  evidence becomes
  `join: base evidence is \`<kind>\` (weak); the reviewer's mutation spot-check must cover it (METHODOLOGY §6)`.
  The `join --revert-check` subcommand itself stays (guard `guard_t005::revert_check_leaves_task_target_untouched`
  pins it) and is described as an optional diagnostic.
- `:360` "Timestamps are always passed in": the Metrics section documents (1)–(4).
- `docs/development.md` also documents (2), (3), (5), (6) in its queue, worktree, gate and a new
  "Publishing a ledger input" section.

## Pinned
- (a) **Errors, payloads, precedence.** Exit codes: refusal 1, usage error 2, publish commit
  failure 2, publish otherwise the queue's code. `metrics add` checks in this order, first
  failure wins: options parse (usage, exit 2) → task id form and task file / `iter-N` role rules
  → tokens rule → time rules → append. `transition`: id → task file → `--from` matches current →
  table → time → append. Queue: `budget` sits between `approval` and `main-history`; all earlier
  preconditions keep their order. `worktree new`: state check before any git or file change.
  `publish`: steps 1–8 in order.
- (b) **Files.** `metrics/tasks.jsonl`: one compact JSON object per line, appended (only
  nothing is rewritten; `join-defects` appends a `kind:"join_defects"` record); `run.tokens` may be `null`, `run.tier` is
  `null` for `iter-N`. `metrics/queue.jsonl`, `docs/decisions.md`, task files and reviews are
  read only, except the publish task file it writes. Publish writes `tasks/T-NNN-<slug>.md`, the
  worktree's copies of the inputs and `docs/scorecard.md`, and the queue's files as the queue
  does. The four redacted reviews change only on the named line: the session-specific prefix up
  to and including `scratchpad/` is replaced by `<scratch>/`.
- (c) **Order of checks.** As (a) and Interface (5) steps 1–8.
- (d) **Paths and platform.** Every path relative to the repository root of the primary checkout
  (publish, queue) or of the checkout being gated (gate steps). Linux only, git ≥ 2.38, as today.
  Main's state for `worktree new` and the budget data for `queue` come from main's committed
  tree, never from a working copy (D-026 R6).
- (e) **Threat model.** As D-026: the checks catch the director's honest slips and corner-cutting,
  not a malicious agent. Accepted: an `--at` (or omitted default) recorded while the clock is
  wrong; a director who writes a false `Extend` row; a run booked with invented tokens; a task
  directory deleted from main to skip the state check. Not accepted: any flag or environment
  variable that turns a refusal off, or a clock override. The `hygiene` step's reviews check verifies form and
  arithmetic, not that the mutants were really run.
- (f) **Tests pinning behaviour this changes.** (1) `acceptance_t005_support.rs` `Repo::add_task`
  / `Repo::worktree` commit toy task files in states `reviewed` (29 calls), `self_verified` (39)
  and `draft` (2) and then call `worktree new` (87 `add_task`/`worktree` call sites; command:
  `grep -on 'add_task("T-[0-9]*", "[a-z]*", "[a-z_]*"' xtask/tests/acceptance_t005*.rs | awk -F'"' '{print $6}' | sort | uniq -c`).
  Rule (3) refuses all of them. Ruling D-1: T-013's test author updates the shared fixture
  `xtask/tests/acceptance_t005_support.rs` so toy tasks are `ready` before a worktree is made; no
  assertion in any T-005 test changes. If a T-005 test then fails for another reason (for
  example by counting main's commits), the test author stops with a `QUESTION:`. (2) The
  seeding guards (`guard_t005.rs` `seed_repo`, `acceptance_t005.rs` `build::repo`) have no
  `tasks/` directory: covered by the no-`tasks/` rule in (3). (3) `xtask/src/gate.rs`
  `step_order_matches_charter` (implementer's unit test) changes with the new step; say so in the
  log. (4) the session-paths check fails on today's `main` on exactly four lines (command:
  `git grep -nE '<the regex of (6)>'` → `reviews/T-003-r1.md:39`, `reviews/T-005-r1.md:15`,
  `reviews/T-005-r2.md:18`, `reviews/T-005-r3.md:17`); these are redacted in this task (in
  `touches`). No allowlist (golden rule 4: no allowlist growth). (5) No existing test calls
  `metrics add` or `transition` through the binary (grep `"metrics"` in `xtask/tests`), so (1)
  breaks no acceptance test; unit tests in `metrics.rs` that pass fixed times stay valid if they
  call the record builders with an injected clock.
- (g) **Worked examples** (executed against the rules, not code): a task whose last transition is
  `in_progress` at 10:00:00Z: `transition --to tests_joined` → refused (table); `--to
  self_verified --at 09:59:59Z` → refused (earlier); `--to self_verified` at 10:05:00Z → appended
  with `at` 10:05:00Z. A medium task `T-020` with runs 300,000 + 250,000 tokens and budget 500,000:
  `add` prints `T-020: 550000 tokens of budget 500000` and the over-budget line `over budget by
  50000 tokens`; the queue refuses `budget` until a row `| D-0xx | … | Extend T-020 to 600000 … | … |`
  is on main; a later row `Extend T-020 to 520000` makes it refuse again (last row wins). Join record `acc_fail_first_join: 2, acc_defects: null` on an iteration-1 task:
  report exits 1 with one `unclassified join` line; `join-defects … --acc-defects 3` refused
  (3 > 2); `--acc-defects 1` sets it, and the report's acceptance-caught total grows by 1.
  Session path: the line `see /tmp/claude-0/x/` + a UUID + `/scratchpad/m.py` matches; the regex
  text in this file does not (it contains `[0-9a-f]{8}`, not eight hex digits).
- (h) **Scorecard items.** None.

## Acceptance criteria
In throw-away git repositories built by the tests (as T-005's are), with `HOME` and
`GIT_CONFIG_GLOBAL` isolated, through the `xtask` binary only. Time-dependent assertions bracket
`<now>` between clock readings taken by the test before and after the call. Oracle: exact value
(exit code, the one stderr line, appended bytes, `git log`/`git show`) unless stated.

- AC-1 (clock): `metrics add` without `--ended-at` and `transition` without `--at` record a time
  within the test's bracket; `--at` one second earlier than the task's latest record → refused,
  nothing appended; equal → appended; one hour after `<now>` → refused.
- AC-2 (transitions): from a fresh medium task, the path draft → specified → ready → in_progress
  → self_verified → tests_joined → reviewed → queued → merged → evaluated is accepted with
  default times. Refused, each with an unchanged file: in_progress → tests_joined;
  self_verified → queued on a medium task (accepted on a low task); `--from ready` when the
  current state is `in_progress`; any transition out of `evaluated`; blocked → `ready` when it
  was entered from `in_progress` (blocked → `in_progress` accepted); `transition --task iter-1`.
- AC-3 (tokens): `--tokens 0` → refused; `--tokens-unknown` → record has `"tokens":null`;
  both → refused; neither → refused; `--tokens 1` → accepted.
- AC-4 (budget line): runs 300000 and 250000 on a task with `budget_tokens: 500000` → stdout of
  the second `add` ends with the two lines of Interface (1) with 550000 and 50000; a third run
  with `--tokens-unknown` adds the unknown-runs line with 1.
- AC-5 (iter-N): `add --task iter-1 --role evaluator …` → record `task:"iter-1"`,
  `iteration:1`, `tier:null`, and stdout `iter-1: <n> tokens of overhead`; `--role implementer`
  on `iter-1` → refused; `--role planner` on `T-001` → refused; `--iteration 2` with `iter-1` →
  refused.
- AC-6 (queue budget): a reviewed, joined, approved high toy task whose committed run records
  exceed its `budget_tokens` → `queue: refused: budget: …` exactly as Interface (2), main and the
  branch unchanged, no record. After a committed row `Extend T-NNN to <N>` with `N` ≥ the
  recorded sum, it merges; with `N` below the sum, it is refused naming that ceiling; a later row
  with a lower `N` overrides an earlier higher one. Rows `Extend T-NNN` (no ceiling),
  `Extend T-NNN-tests to …`, and an `Extend` line inside a fenced code block do not count. Uncommitted run records in the working copy are not read (the `ledger` refusal
  comes first, as today). A task within budget is unaffected.
- AC-7 (worktree ready): a task committed on main as `specified` → `worktree new` refused with
  the exact line and no `.worktrees/T-NNN`, no branch; after main's file says `ready`, it is
  created; a `ready` state only in the uncommitted working copy is refused; `--tests` and
  `T-NNN-tests` forms behave the same; a repository with no `tasks/` on main creates the
  worktree.
- AC-8 (report): with fixture records (two joins with `impl_tests_green:true` and `acc_defects`
  1 and 2, one with `impl_tests_green:false` and 5; an iteration-1 task with an unclassified
  failing join; an iteration-0 task with one): the acceptance-caught total is 3 by tier and arm,
  exactly one `unclassified join` line (the iteration-1 one), exit 1; after `join-defects` sets
  it, exit 0. Tokens per merged task without and with overhead equal hand-computed values for a
  fixture with two merged tasks (one medium odd, one high) and two `iter-1` runs. Known defects
  merged by decision lists exactly the `found_by: reviewer` tasks whose origin is merged.
  `join-defects` appends exactly one `join_defects` record and changes no other byte of the
  file (the old lines are a byte prefix of the new file); its refusals leave the file
  byte-identical.
- AC-9 (publish, success): a toy repository with a gate that runs `scorecard --check`, an
  untracked new `scorecard/items/x.y.toml` and a modified `evaluations/iter-0.md`: `publish
  T-050 --title "Publish x" --iteration 1 <both>` exits 0; main's first-parent log shows, in
  order, `publish: T-050 ready`, `publish: T-050 queued`, the task's content commit (inputs +
  `docs/scorecard.md` only, byte-identical inputs), `queue: record T-050 merged`,
  `publish: T-050 merged`; every one of these commits carries `Task: T-050` and `Role: director`;
  the transitions recorded for T-050 are draft→specified→ready→in_progress→self_verified→queued→merged
  in non-decreasing time; the task file's front matter is exactly Interface (5) step 2 with
  `state: merged`; `.worktrees/T-050` no longer exists; `scorecard --check` passes on main.
- AC-10 (publish, refusals and stops): each leaves main, branches, worktrees and the inputs
  unchanged and prints the exact line: an input that changes no generated file; an input under
  `docs/`; an existing `tasks/T-050-*.md`; another uncommitted change in `tasks/`; a malformed
  item. A gate that fails at step 5 → exit 1, the step-5 line, T-050 `in_progress` on main, the
  primary checkout's inputs still present. A commit that fails at step 2 (signing program that
  exits non-zero) → exit 2 and the step-2 line; the printed command, run after the test repairs
  signing, commits exactly those two paths.
- AC-11 (`hygiene`, reviews check): on a toy repository with an iteration-1 medium task T-021 (odd →
  `implementer`): a round-1 review with `mutants: {introduced: 12, equivalent: 1, killed: 10,
  killed_by_acceptance: 9, survived: 1}` and `arm: implementer` → PASS; each single fault → FAIL
  naming the file: no `mutants`; a missing key; `introduced: 9` in round 1 (accepted in round 2);
  sums that do not add up; `killed_by_acceptance > killed`; `arm: independent`; `tier: high`;
  `round: 2` in `T-021-r1.md`. An iteration-0 task's review without `mutants` → not checked; a
  low task's review → not checked. On this repository's `main` after redaction: PASS or EMPTY.
- AC-12 (`hygiene`, session-paths check): a tracked file containing a session-specific path (built at run
  time by the test) → FAIL with `<path>:<line>` and without the matched text; the same text in an
  untracked file → PASS; a regex-like text such as the one in this task file → PASS; a non-UTF-8
  tracked file → skipped. On this repository's tree after redaction → PASS. `cargo xtask ci`
  lists `hygiene` between `ui-commands` and `schema`, and `docs/CHARTER.md` §4's gate line names
  it in the same place.
- AC-13 (stale references): `worktree new` output contains no `CARGO_TARGET_DIR`; a join with
  weak base evidence prints the Interface (7) line and no `revert-check`; `docs/development.md`
  contains none of `export CARGO_TARGET_DIR`, `O-2)`, `Command schema changes are`, `Timestamps
  are always passed in`, and its Join section does not require a revert check (reviewer reads
  the rest). Oracle: exact value and reading.
- AC-14 (no regression): every T-005 acceptance and guard test passes (unmodified, or with only
  the fixture change D-1 allows: no assertion in any T-005 test changes); main's gate and the
  task's own gate are green.

## Out of scope and implementer notes
- Join acceptance counts across crates: T-009 (6). Record-commit signing, merged-record binding,
  lint escapes, untracked drafts: T-009.
- The concurrency cap in `worktree new` (retro change 13's second half): builder agents
  compiling are not visible to the repository; stays a director check (§5.4).
- The dispatch record at run start (§2.1) and the model-matches field: not a §13 field; stays a
  director check.
- Editing `methodology/` (only at a retrospective): the "(T-013)" / *Interim* marks in
  METHODOLOGY become stale once this merges; the director notes that for retrospective 1.
- Changing `docs/CHARTER.md` beyond the gate-step line, `AGENTS.md`, any product crate, `Cargo.*`, `.github/`.
- Your tests: unit tests in `xtask/src`; toy repositories via the existing helpers. Inject the
  clock into record builders for unit tests; the binary always uses the system clock. Tests that
  need a session-specific path must build the string at run time so no tracked file contains one.
- Never set or export `CARGO_TARGET_DIR` in commands you run by hand (D-019); the queue and the
  publish helper set it only for the child processes they run, as the queue does today.
- Specified: commands, options, refusal texts, record changes, order of steps and checks.
  Left to you: module layout, how the scorecard's would-be output is computed without writing,
  how table rows are shared between `approval` and `budget`.

## Order with T-009 (both touch `xtask/src` and `docs/development.md`)
T-013 lands **first**; T-009 depends on it (`depends_on: [T-005, T-013]` there). Their `touches`
cannot be made disjoint: both change `queue.rs`/`precheck.rs` (a precondition each), `join.rs`
(T-013 messages, T-009 counts), `main.rs` and `docs/development.md`, so they may not be in
flight together (METHODOLOGY §5.2). Why this order: T-013 replaces eleven interim director steps
that every iteration-1 task depends on (times, transitions, `iter-N` booking, `ready` at dispatch,
budget), including T-009's own records and budget check; T-009's items are accepted residual
risks or have a hand workaround until then (the director recounts a cross-crate join by hand and
records it in a `--note`; untracked drafts are moved aside). The publish helper passes the
queue's exit code through unchanged, so T-009's new exit code 3 needs no change in T-013's code.
Both tracks are disjoint from every product task (no `xtask/` in their `touches`), so T-013 can
run in parallel with any one product task.

## Decisions
Director's answers to the planner's questions of R-055 (run R-057).
- D-1 (was Q-1): allowed — T-013's test author updates the shared fixture
  `xtask/tests/acceptance_t005_support.rs` so toy tasks are `ready` before a worktree is made; no
  assertion in any T-005 test changes. The file is in `touches`.
- D-2 (was Q-2): one new gate step named `hygiene` carries both checks (review front matter of
  medium/high tasks; no tracked file contains a session-specific scratch path). The gate's step
  list is in CHARTER §4, so this needs an ADR: `docs/adr/0003-hygiene-gate-step.md` (drafted by
  the planner, R-057). T-013 depends on it being accepted; its `touches` include
  `docs/CHARTER.md` for the one line that lists the gate steps.
- D-3 (was Q-3): `join-defects` appends a new record kind to `metrics/tasks.jsonl`; records are
  never edited in place.
- D-4 (was Q-4): an `Extend T-NNN` row must name the new token ceiling, e.g.
  `Extend T-NNN to 1200000`; the queue compares against that number.
- Director answers before the join (2026-11-13, decision D-039), given to both builders at once — part of the spec:
  a. `publish` leaves main's ledger committed wherever it stops: the `ready → in_progress` transition is committed before the gate runs; after a red gate the task is `in_progress` in main's committed tree and the working tree is clean.
  b. Publish slug: trailing `-` trimmed after the 40-character cut. Publish refusals: exit 1, exactly one stderr line starting `publish: refused: ` naming the offending path, nothing changed.
  c. `--tier` on an `iter-N` record is refused; `join-defects` has no time rule; a task file without `budget_tokens` prints `T-NNN: <sum> tokens of budget none`; the review front-matter check accepts `mutants` as a flow map or a block map.
  d. `worktree new` refuses a task with no task file on main as well as one that is not `ready`; the T-005 negative case that needs such a worktree is set up in the fixture with plain `git worktree add` (setup only).
  e. The over-budget line is the Interface (1) text; the wording in Pinned (g) is struck.
  f. `metrics report` formats: a line starting `acceptance-caught defects: <total>`; per-tier and per-arm integers on lines naming the tier or arm; the with-overhead figure on a line containing `overhead`; `T-NNN (from T-MMM)` lines; `extend to: <N>|none` and `within ceiling: yes|no` on a line naming the task.
  g. `publish` regenerates `docs/scorecard.md` with the command main's `xtask/generated.toml` lists for that path, run in the task worktree.
  h. (replaced by D-040) `main`'s history is append-only: no tool resets, amends or rewrites a commit on `main`. A successful publish adds exactly six commits to main's first-parent history — the five of AC-9 plus `publish: T-NNN in progress` after the `ready` commit and before the gate; after a red gate the `ready` and `in progress` commits both remain.
  i. `hygiene` is never EMPTY.
- Director rulings after review round 1 (2026-11-13, decision D-047), given to both builders at once — part of the spec. Each was checked against the code it constrains (`precheck.rs` extension parser, `queue.rs` `Lock::acquire`, `publish.rs` steps):
  j. (finding 9) The ceiling of an `Extend T-NNN to <N>` row is a run of ASCII digits. A digit run directly followed by `,`, `.` or `_` and another digit (`1,200,000`, `5000.5`, `1_200_000`) is malformed: the row is **not** an extension (as `Extend T-NNN` with no ceiling). One parser serves the queue's `budget` precondition and `metrics report`; no new message.
  k. (findings 1, 10) Publish step 1 also refuses when the working tree's `metrics/tasks.jsonl` holds any record whose `task` is `T-NNN`: `publish: refused: T-NNN already has records in metrics/tasks.jsonl`. If anything in step 2 fails before its commit is attempted (write, transition), publish removes the task file it wrote and restores `metrics/tasks.jsonl` to the bytes it had when the step began, so main is as it was found; exit code and message of the failure are kept.
  l. (finding 2) In steps 2, 6 and 8 a failing `git add` is a commit failure: exit 2 and the pinned `publish: commit failed: <git error>; commit by hand: …` line.
  m. (findings 3, 11) Publish takes the queue's lock (`<git-common-dir>/kiln-queue.lock`, `Lock::acquire`) before step 1 touches the queue's private build tree, and holds it through step 7. Lock held by a live process: `publish: refused: another queue or publish run holds <lock path>`, exit 1, nothing changed (build tree included). Publish releases the lock immediately before step 8, and the queue takes it itself as today. Accepted: another queue may start in that gap; then step 8's existing line (`publish: queue did not merge T-NNN (exit c); the task stays queued; …`) applies and nothing is damaged.
  n. (finding 12) Publish is never resumed and a task number it has used is not reused. Wherever it stops after step 2's commit, every ledger change it made on main is committed (D-039a) and main's task file `state:` equals the last recorded transition's `to` (so a stop in steps 3–5 leaves `state: in_progress` committed, not `ready`). A stop in steps 3, 4 or 7 that has no pinned line prints one line `publish: stopped at step <n>: <error>; T-NNN is <state>`. `docs/development.md` "Publishing a ledger input" gains a table: for each stopping step, the state left behind and the by-hand commands that finish it (the director's ordinary `queue` / `metrics transition` / task-file `state:` / ledger commit) or abandon it.
  o. (findings 4, 5) `session-paths` checks a tracked symlink's target text as it checks a file's content. No xtask command panics on a malformed `task` value in a record: the arm of a task id that does not end in an ASCII digit is `none` (reported under neither arm).
  p. (findings 6–8, test author) Tests are owed for: publish with a dirty path outside the inputs and ledger paths; an input changed between steps 4 and 7; publish with no input path; and each of j–o. Finding 13 (slug of a title with no alphanumerics): step 1 refuses, `publish: refused: the title has no letter or digit`.
  q. (test author's question, R-090; answered to both builders at once) A stop in step 3, 4 or 7 exits 1 (as step 4's "any other path" stop already does); exit 2 stays reserved for usage errors and commit failures. The text of the step-1 "no input path" refusal is not pinned beyond the `publish: refused: ` prefix. The undo of ruling k's second sentence is covered by the implementer's unit test only: step 1 now refuses the one case that reached it through the command line.

## Questions and rulings
None open.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13T03:50Z · R-055 · (new) · did: specified the v1.1 tooling from CHANGELOG v1.1
  changes 3, 5, 12 (as reworded by D-036), 13, 14, 15 and the stale-reference list. Read
  `metrics.rs` (times passed in, `tokens` required u64, no state table), `worktree.rs` (no state
  check), `main.rs:180` (prints the export line), `main.rs:294` and `join.rs:1` (revert check),
  `gate.rs` (step list and its order unit test). Found that rule (3) breaks 87 toy-task
  `worktree new` calls in T-005's acceptance tests (Q-1) and that `session-paths` fails today on
  four review lines (redaction in `touches`, no allowlist). Kept the cross-crate join count in
  T-009. Tier high, criterion: gate paths (`xtask/src`; `docs/development.md` documents the list but is not one); no dependency.
  Pinned (h) n/a: no scorecard item. Uncertain: "known defects merged by decision" is computed
  from `found_by: reviewer` follow-ups, which also counts spec-gap follow-ups (T-006, T-007 would
  count beside T-008, T-009, T-030) — no field distinguishes them; the scope (7 parts) may exceed
  850k (T-005 took 1.67 M for a comparable xtask change; retro-0 `S tokens`), so splitting off
  (5) is an option. · gate attempts: 0 · not run: gate, any xtask command · missing context:
  none · next: director answers Q-1…Q-4.
- 2026-11-13T03:56Z · R-057 · draft · did: applied D-1…D-4: fixture change in `touches` with
  the director's wording; single `hygiene` step and the CHARTER §4 line in `touches`, dependency
  on ADR-0003 (drafted as `docs/adr/0003-hygiene-gate-step.md`, status proposed); `join_defects`
  append-only record; `Extend T-NNN to <N>` ceilings, last row wins. · gate attempts: 0 ·
  missing context: none · next: director accepts ADR-0003.
<!-- end planner log -->
### implementer
- 2026-11-13T04:05Z · R-058 · in_progress (main) · did: restated the criteria before coding —
  (1) metrics add/transition stamp `<now>`, refuse future, earlier-than-task-latest, illegal
  §3 transitions, bad token options and role/`iter-N` mismatches, print the budget line(s);
  `join-defects` appends a classification record; (2) queue refuses at `budget` between
  `approval` and `main-history` from main's committed runs and Extend rows; (3) `worktree new`
  needs `ready` in main's committed task file (no `tasks/` on main: no check), prints no
  CARGO_TARGET_DIR; (4) report: arm, acceptance-caught over all green joins by tier/arm,
  unclassified iteration-≥1 joins exit 1, tokens per merged task with/without `iter-N`,
  known defects merged by decision, overruns with Extend; (5) `publish` in 8 steps; (6) gate
  step `hygiene` (reviews + session-paths) after ui-commands, CHARTER §4 line, four
  redactions; (7) stale texts. · gate attempts: 0 · next: implement.
  QUESTION: (Q-a) my gate cannot be green before the join: rule (3) refuses the T-005 toy
  tasks (`reviewed`/`self_verified`) and D-1 gives the fixture fix to the test author, while
  the brief forbids me to edit `acceptance_t*` files. I will report the gate with the T-005
  acceptance failures as the predicted D-1 failures and nothing else red.
  QUESTION: (Q-b) publish step 5 red: the `ready → in_progress` transition (step 3) is
  appended to main's working-copy `metrics/tasks.jsonl` but no step commits it before step 6;
  I read AC-10's "T-050 in_progress on main" as the metrics state in the primary checkout
  (uncommitted), and add no commit. Should step 3 or the failure path commit it?
  QUESTION: (Q-c) slug: I cut the title part (after `publish-`) to 40 characters after the
  trim, literally in the order written (so a cut can leave a trailing `-`); ASCII
  alphanumerics only.
  QUESTION: (Q-d) publish refusal texts not given by the spec: I use `publish: refused: ledger:
  <the queue's ledger reason>` and `publish: refused: the main working tree is dirty: <paths>`
  for step 1's working-tree rules.
  QUESTION: (Q-e) not specified, chosen: `--tier` on `iter-N` is refused (tier must be null);
  `join-defects` applies no time rule (its `at` is `<now>`); a task file without
  `budget_tokens` prints `T-NNN: <sum> tokens of budget none` and no over line; the review
  check accepts `mutants` as a flow map `{…}` (template form) or an indented block map.
- 2026-11-13T06:40Z · R-058 · in_progress (main) · did: (6) `xtask/src/hygiene.rs` + gate
  step after `ui-commands` (unit test `step_order_matches_charter` extended: hygiene directly
  after ui-commands, schema directly after hygiene), CHARTER §4 line, the four redactions
  (only the named lines; `git grep -nE` of the pattern now finds nothing); (3) ready-on-main
  check in `worktree::create` (queue/join unit-test fixtures use a `#[cfg(test)]`-only
  `create_unchecked_for_tests`, so the binary has no bypass; queue unit fixture task files got
  `budget_tokens: 1000`); (2) `budget` precondition + `precheck::extension` shared with the
  report and `add`; (1)/(4) metrics rewritten (injected clock, §3 table, iter-N, tokens,
  budget lines, `join-defects`, report sections, report exit 1); (5) `xtask/src/publish.rs`;
  (7) messages and `docs/development.md`. Exercised through the binary in toy repositories:
  metrics refusals/budget lines/report exit codes; publish success (main's first-parent log
  ready, queued, content, `queue: record T-050 merged`, merged; transitions in order; worktree
  removed), refusals (docs/ input, unchanged input, malformed item, dirty `tasks/`, existing
  task file), step-2 signing failure (exit 2, printed command then commits exactly the two
  paths) and step-5 red gate. · gate attempts: 1 (last: red `test` only — 15 failures, every
  one `worktree: refused: T-001 is self_verified|reviewed on main` from the T-005 fixture,
  the D-1 change I may not make; all other steps PASS, hygiene PASS on this tree). With a
  trial fixture change in a throw-away clone (not committed: toy task committed `ready`, worktree
  made, then the real state committed) all T-005 suites pass except three, each still a
  `worktree new` refusal: `refusals_change_nothing` makes a worktree for T-010 that has **no
  task file** on main (a `tasks/` directory exists) — that is beyond "toy tasks are ready";
  `join_finds_acceptance_and_guard_tests_under_xtask_tests` makes `T-001-tests` when the task
  is `tests_joined`; `refuses_front_matter_changes_on_task_branch` depends on the branch's copy
  of the task file (fixture design). · not run: the acceptance tests of the parallel test
  author; publish on the real repository (forbidden; the real `scorecard` in the export builds
  every test binary into `.git/kiln-publish/target`, a cold build); a concurrent queue during
  publish (publish does not hold the queue lock across steps 1–7). · missing context: none ·
  next: join; director answers Q-a…Q-e.
  QUESTION: (Q-f) `refusals_change_nothing` (T-005) needs a worktree for a task with no task
  file on main; with rule (3) that cannot be done through `worktree new` without changing the
  test beyond D-1 — the test author will meet it.
- 2026-11-13T06:30Z · R-063 · in_progress · did: applied the round-2 rulings. Q-b: step 3
  commits `publish: T-NNN in_progress` (`metrics/tasks.jsonl` only) before the gate; a red
  step 5 leaves main committed with the task in_progress. To keep ruling (4)'s exactly-five
  history on success, step 6 folds that interim commit into `publish: T-NNN queued` with a
  `git reset --soft` to the `ready` commit, only while main still equals the interim commit
  (reviewer: judge whether this is acceptable; the alternative is six commits). Q-c: trailing
  `-` trimmed after the cut. Q-d: scorecard and no-change refusals now name the inputs (the
  no-change text gains a ` (<paths>)` suffix). Ruling (3): the regeneration command is the
  `docs/scorecard.md` entry of main's committed `xtask/generated.toml` (`{xtask}` = main's
  xtask as the queue builds it), run in the task worktree in step 4; step 1's pre-check runs
  the same command in the private export (nothing in the checkout changes before step 2).
  Ruling (2): report prints `acceptance-caught defects: N (…)` then `  tier <t>: n` / `  arm
  <a>: n` lines, `  iteration N with overhead: <avg> (…)`. Unit test added for the command
  lookup; toy runs re-done through the binary (five commits after success; step-5 stop leaves
  `ready`, `in_progress` committed and only the input untracked). · gate attempts: 2 (last:
  red `test` only, the same 15 T-005 `worktree: refused` failures; all else PASS). · not run:
  acceptance tests; publish on the real repository. · missing context: none · next: join.
- 2026-11-13T06:45Z · R-064 · in_progress · did: D-040 — removed the `reset --soft`; the
  step-3 commit's subject is `publish: T-NNN in progress`; a success adds six first-parent
  commits. `publish::run_with` takes the gate and the queue as parameters (the binary passes
  `CargoGate` and the child `cargo xtask queue`), so unit tests run the whole publish
  in-process on a toy repository: the six subjects in order, every reflog value of main since
  the pre-publish tip an ancestor of the final head, clean checkout, worktree removed; and a
  red gate leaves `ready` + `in progress` committed, the in_progress transition in main's
  committed metrics, only the inputs dirty. development.md updated. · gate attempts: 3 (last:
  red `test` only, the same 15 T-005 fixture failures; all else PASS). · not run: acceptance
  tests. · missing context: none · next: join.
- 2026-11-13T07:10Z · R-070 · in_progress (joined) · did: restored the Interface-pinned publish
  lines: step-5 gate line exactly as written (no "(committed on main)"), the no-change refusal
  exactly, and the malformed-input refusal as `publish: refused: <the scorecard's own error>`
  (I had prefixed the input names; the Interface pins "the scorecard's own error, prefixed
  `publish: refused: `"). Other publish lines (step-2 commit failure, step 7, step 8) checked
  against the Interface: unchanged. · gate attempts: 4 (last: red `test`, 404 passed, 1
  failed: `publish_refusals_change_nothing`). · not run: — · missing context: none · next:
  director rules on the question below.
  QUESTION: (Q-g) `acceptance_t013_publish.rs::publish_refusals_change_nothing`, malformed-item
  case (around line 503): earlier cases leave `scorecard/items/x.y.toml` (untracked) and
  `evaluations/iter-0.md` (modified) in the primary checkout, then publish is called with
  `scorecard/items/bad.toml` as the only input. Interface (5) step 1 says that *apart from the
  inputs* the checkout must pass the queue's working-tree rules, and the queue refuses any
  dirty path outside `metrics/`, so the binary refuses with `publish: refused: the main working
  tree is dirty apart from the inputs: evaluations/iter-0.md, scorecard/items/x.y.toml` before
  it runs the scorecard; the test expects the scorecard's own error. I read the test as
  wrong (it should remove or pass the two files first) and have not changed the order of the
  checks. If instead uncommitted files of input kinds that are not inputs are to be
  tolerated, that is a spec change.
<!-- end implementer log -->
### test author
- 2026-11-13T04:05Z · R-059 · ready (worktree copy; in_progress on main) · did: read spec,
  fixture and xtask sources; questions logged before writing tests · gate attempts: 0 · not
  run: gate · missing context: see questions · next: write tests on the assumptions below.
  QUESTION: Pinned (g) says the over-budget line is "over budget by 50000 tokens"; Interface (1)
  and AC-4 give `T-NNN: over its ceiling <ceiling> by <sum - ceiling> tokens; the queue refuses
  it without …`. Tests assert the Interface (1) text exactly. Proposed: strike the (g) wording.
  QUESTION: Interface (4) pins no line format for "acceptance-caught defects by tier and arm",
  "tokens per merged task" (without/with overhead) or "budget overruns". Tests assert: a line
  starting `acceptance-caught defects: <total>` (today's prefix); the hand-computed integers
  (no decimals) appear on report lines that name the tier/arm, or `overhead` for the
  with-overhead value; `T-NNN (from T-MMM)` lines; `extend to: <N>|none` and
  `within ceiling: yes|no` on a line naming the task. Proposed: pin exactly these.
  QUESTION: AC-10 asks for "the exact line" but Interface (5) pins only `publish: refused:
  <reason>` for an input under `docs/`, an existing task file, another uncommitted change in
  `tasks/` and a malformed item. Tests assert exit 1, exactly one stderr line starting
  `publish: refused: `, naming the offending path (for the malformed item: containing the toy
  scorecard's own error text), and nothing changed. Proposed: that is the testable wording.
  QUESTION: Interface (5) steps 1 and 4 say "regenerating docs/scorecard.md" / "the `scorecard`
  command". Tests assume the checkout's own `cargo xtask scorecard` (as the queue's
  `xtask/generated.toml` regeneration runs it), so a toy repository's toy scorecard decides;
  an implementation that calls the tool's built-in scorecard module on the toy tree fails AC-9.
  Proposed: "the command `xtask/generated.toml` lists for `docs/scorecard.md`".
  QUESTION: AC-9 "main's first-parent log shows, in order" — tests assert the five subjects are
  exactly the first-parent commits after main's pre-publish tip, in that order (no other commit
  between). Proposed: say "exactly these five".
  QUESTION: AC-11 says "PASS or EMPTY" on this repository's main; Interface (6) says the step is
  never EMPTY. Tests require PASS (run on a clone of the tested commit, a toy repository).
- 2026-11-13T05:40Z · R-059 · in_progress · did: wrote `xtask/tests/acceptance_t013_{metrics,
  report,queue,publish,hygiene,docs}.rs` (25 tests) and helper `acceptance_t013_support.rs`;
  changed the shared fixture `acceptance_t005_support.rs` only as D-1 allows (`Repo::worktree`
  commits a one-file toy task as `ready` on main for the call, restores it and fast-forwards
  the branch; a task with no or several task files on main gets a plain `git worktree add`,
  per the director's ruling to the implementer); no T-005 assertion changed, all 41 T-005
  tests pass on base. Applied the director's rulings (1)–(6) and the implementer's (red gate
  leaves `in_progress` committed and the tree clean; `--tier` on iter-N refused; `budget
  none`; block-map `mutants`). Criterion → tests → base result (all assertion or command
  error exit 2, none compile-only): AC-1 `add_and_transition_record_the_clock_time_by_default`,
  `times_before_the_tasks_latest_record_or_after_now_are_refused`; AC-2
  `a_medium_task_walks_the_whole_path_with_default_times`,
  `transitions_outside_the_table_are_refused_and_change_nothing`,
  `every_state_pair_follows_the_table` (naive model, 3 tiers × 20 states × 12); AC-3
  `tokens_must_be_positive_or_declared_unknown`; AC-4
  `add_prints_the_tasks_tokens_against_its_budget`; AC-5 `overhead_runs_are_booked_to_iter_n`,
  `a_task_run_needs_exactly_one_task_file`; AC-6 `queue_refuses_a_task_over_its_ceiling_until_an_extend_row_covers_it`,
  `budget_sits_after_approval_counts_unknown_runs_as_zero_and_needs_budget_tokens`; AC-7 (+AC-13
  output) `worktree_new_needs_the_task_ready_on_main`,
  `worktree_new_checks_no_state_without_a_tasks_directory_on_main` (fails on base only on the
  `CARGO_TARGET_DIR` line); AC-8 `report_computes_the_v11_metrics`,
  `join_defects_appends_one_record_and_classifies_the_join`; AC-9
  `publish_carries_a_ledger_input_to_main_in_one_command`; AC-10 `publish_refusals_change_nothing`,
  `publish_stops_when_the_gate_fails_and_keeps_the_inputs`,
  `a_failed_ready_commit_stops_publish_and_prints_the_commit_command`; AC-11/12
  `reviews_check_accepts_v11_front_matter_and_names_each_faulty_file`,
  `session_paths_check_flags_tracked_files_by_path_and_line_only`,
  `this_repositorys_tree_passes_hygiene`, `ci_and_charter_list_hygiene_between_ui_commands_and_schema`;
  AC-13 `a_join_with_weak_base_evidence_points_to_the_mutation_spot_check`,
  `development_md_has_no_stale_v10_references`; AC-14: the T-005 suites themselves.
  Self-check: scratch stub (worktree ready check, metrics add/transition/join-defects, queue
  `budget`, `hygiene`, gate step, join line; no publish, report or docs — those tests fail on
  it for being unimplemented) passed every test it covered and all T-005 tests; seeded bugs
  killed 14/14: B1 worktree reads working copy (worktree_new_needs…); B2 equal time refused
  (times_before…, +3); B3 `--tokens 0` accepted (tokens_must…); B4 blocked ignores entered
  state (every_state_pair…, transitions_outside…); B5 budget `>=` (queue_refuses…); B6 fenced
  Extend counts (queue_refuses…); B7 first Extend row wins (queue_refuses…, add_prints…); B8 no
  `killed_by_acceptance ≤ killed` (reviews_check…); B9 session path text printed
  (session_paths…); B10 arm parity swapped (reviews_check…); B11 join-defects N ≤ f+1
  (join_defects…); B12 iter-N tier "" (overhead_runs…); B13 run time = started_at
  (times_before…); B14 unknown-runs line dropped for one run (add_prints…). The stub found one
  test bug (untracked case re-tracked by `git add -A`), fixed. Clippy `-D warnings` clean with
  the stub. Not tested: `worktree new` with two task files on main (unspecified); the queue
  summary's count of unknown-token runs (no format); report's "excluded runs" count on the
  tokens-per-merged-task line (no format); the T-005 fixture's own git/cargo calls have no
  timeout (reused as is; every xtask call in T-013 tests has one); gate.rs
  `step_order_matches_charter` is the implementer's (Pinned (f)(3)). · gate attempts: 1 (last:
  red test, only in acceptance_t013_* targets; fmt, clippy, layers, ui-commands, schema,
  scorecard, no-skip PASS) · not run: gate after the last ruling edits (tests re-run directly:
  41 T-005 pass, 25 T-013 fail on base) · missing context: none · next: join.
- 2026-11-13T06:10Z · R-065 · in_progress · did: applied D-040 (main append-only): AC-9 test
  now requires exactly six first-parent commits (ready, in progress, queued, content, queue
  record, merged) and that the pre-publish tip and every reflog value of main are ancestors of
  the final head; the in-progress commit touches only T-050's ledger files; the red-gate test
  requires main's first-parent log to be exactly ready, in progress. Fails on base (exit 2). ·
  gate attempts: 0 this run (clippy clean, publish tests run directly) · missing context: none ·
  next: join.
- 2026-11-13T06:40Z · R-068 · in_progress · did: removed three `#[cfg(test)]` lines that my
  lint fix (R-059, a script adding `cfg(test)` before every top-level `fn`) had wrongly put into
  the toy source string `PUBLISH_TOY_MAIN`, and its unused `Path` import; built the toy `xtask`
  as a binary with `-D warnings` and ran its `scorecard`, `scorecard --check` and `ci --report`
  by hand (pass, red, malformed). My stub had no `publish`, so it never built this toy: a
  self-check gap. · gate attempts: 0 this run (clippy clean) · missing context: none · next:
  re-join.
- 2026-11-13T07:10Z · R-071 · in_progress · did: the implementer's Q-g is right. In
  `publish_refusals_change_nothing` several cases left non-input files dirty from earlier cases:
  the docs/ case left the evaluation modified; the modified-item and malformed-item cases left
  the new item and the evaluation dirty. Interface (5) step 1 says that, apart from the inputs,
  the primary checkout must pass the queue's working-tree check, so those cases hit that
  refusal first. Now each case starts from a clean checkout (`checkout -- .`, `clean -fd`,
  status asserted empty) and dirties only its own files. No assertion weakened. · gate
  attempts: 0 (clippy clean; fails on base, exit 2) · missing context: none · next: re-join.
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
