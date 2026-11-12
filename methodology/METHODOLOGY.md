# Kiln methodology

Version **v1.0** · effective from the first run of `cargo xtask queue` (until then the bootstrap
exemption D-005 applies) · changes only at an iteration retrospective (§14), recorded in
`CHANGELOG.md`.

Binding inputs: `docs/CHARTER.md` (claims C1–C8, architecture §3, names in §4) and
`docs/decisions.md`. Where this file and the charter disagree, the charter wins and the
disagreement is a defect in this file. Study references (`E07-018`, `F1`, `D10`) point to the
PhotoCraft study in `/home/user/photocraft-study/deliverables/`.

Every rule below names how it is checked: **mechanical** (a command fails), **director** (the
director checks it at a named moment), or **unchecked** (stated only; watch for drift — the study
found that practice diverges exactly at unchecked rules, E02 §4). §16 lists them all.

## 1. Principles

| # | Principle | Why (study finding) | Claim |
|---|---|---|---|
| P1 | Contracts and checks exist before agents fan out; a contract change is its own task. | Planned validated schema and op-history never built while fan-out ran from day 1 (E04-003, D03, D10). | C1 |
| P2 | Only the queue moves `main`, and only on a green gate over the rebased result. | Individually green PRs broke main; 11 of 21 red-main incidents were merge skew (E07-003, F1). | C2 |
| P3 | Whoever builds does not certify: acceptance tests and review come from other agents. | Tests that asserted nothing; fixes merged without tests (E07-018, E07-015, F4). | C3 |
| P4 | Status is computed from behaviour tests plus an independent evaluator, never declared. | "625/625 live" measured wiring; 66 of 129 settings did nothing (E07-013, E07-014, F5, F6). | C4 |
| P5 | Review effort follows risk tier. | Review never blocked a merge; bots present, approval absent (E06-041, F12, D06). | C5 |
| P6 | Every rule names its check; a rule without one is labelled unchecked. | Stated-only rules are where practice diverged (E02 §4, errata #5). | all |
| P7 | Budgets gate: exceeding one stops work and is recorded. | 0 of 102 perf budgets enforced (errata #11, E04-051); effort estimates revised ~10× (E08-005). | C6 |
| P8 | The repository is the only memory: state, plans, decisions, logs and provenance are committed. | Hand-off via gitignored devlog and private `plan/` (E02-004, E01-005, F14); attribution lost (F13, E06-011). | C7 |
| P9 | "Correct" is defined by constructed oracles and checked through public interfaces. | The IDE has no reference binary (D10 §4); agents verifying through the control channel was rare (E04-024). | C8 |
| P10 | Stop and report instead of pushing through. | Out-of-scope edits produced duplicates and broken builds (E07-019, E06-050); contract changes hidden in feature PRs broke main (E07-003). | C6, C7 |

## 2. Roles

Builder roles run as **separate agent sessions with no shared memory**. Each receives its brief
(`methodology/roles/<role>.md`), the task ID and a run ID `R-NNN`; everything else it needs is in
the task file and the repository.

| Role | Does | Never |
|---|---|---|
| director | Plans iterations; accepts specs; dispatches runs; joins test branches; approves high tier as owner-delegate (`docs/decisions.md`); runs the queue; records metrics; writes `STATE.md` and handoffs. | Writes product code or tests; approves a spec it wrote. |
| planner | Writes task specs: observable acceptance criteria, interface, `touches`, tier, budget. | Writes code or tests. |
| implementer | Code and its own unit tests on `task/T-NNN`. | Edits acceptance tests or `scorecard/`; leaves `touches`. |
| test author | Acceptance tests from the spec alone on `task/T-NNN-tests`. | Reads the implementation; edits production code. |
| reviewer | Checks the joined branch against the spec; verdict with classified findings. | Rewrites the code. |
| evaluator | End of iteration: exercises the product through public interfaces only; confirms or rejects scorecard items; files defects. | Reads source or tests to decide a verdict. |
| retrospective analyst | Scores C1–C8 from metrics; proposes versioned methodology changes. | Edits the methodology directly. |

The charter's **integrator** role is held by the director in v1: integration is the join (§6) plus
`cargo xtask queue` (§8), so no separate integrator agent runs (open question O-1, §15).

No run may hold two builder roles on one task. Checked: director, at dispatch, from
`metrics/tasks.jsonl` (each run record names one role and one task).

## 3. Task lifecycle

One task = one branch `task/T-NNN` = one worktree `.worktrees/T-NNN` = one target directory,
created by `cargo xtask worktree new T-NNN`. Medium/high tasks also get `task/T-NNN-tests`
(`cargo xtask worktree new T-NNN-tests`, O-2). Only the director changes `state` in front matter,
on `main`, and appends a `transition` record (§13) each time.

| State | Entry criteria | Work | Exit → next |
|---|---|---|---|
| `draft` | Director asks the planner for a task (or the evaluator, an incident or a reviewer's spec gap yields a follow-up). | Planner writes the spec. | Spec complete per template → `specified`. |
| `specified` | All template sections filled; ACs observable. | Director checks tier, `touches`, ACs, budget, `depends_on`. | Director accepts → `ready`; rejects → `draft`. |
| `ready` | Accepted; every `depends_on` task is `merged`. | — | Dispatched (§5) → `in_progress`. |
| `in_progress` | Worktree(s) exist; implementer (and, medium/high, test author) dispatched. | Autonomous loop (§10). | Implementer reports gate green → `self_verified`. |
| `self_verified` | `cargo xtask ci` green in `.worktrees/T-NNN`; implementer log entry. | Low: nothing. Medium/high: wait for the test author's log entry; director joins (§6). | Low → `queued`. Medium/high: join passes → `tests_joined`; acceptance fails → `in_progress`. |
| `tests_joined` | Join recorded: fail on base, pass on joined branch, gate green. | Reviewer run (§7). | Approve (+ D-NNN for high) → `reviewed`; request-changes → `in_progress`. |
| `reviewed` | Verdict `approve` recorded in `reviews/`. | — | Director runs the queue → `queued`. |
| `queued` | Queue running for this task (one at a time). | `cargo xtask queue T-NNN`. | Green → `merged`; red → `in_progress` with the queue log attached. |
| `merged` | `main` fast-forwarded; `queue.jsonl` record `merged`. | Worktrees removed. | Evaluator confirms the task's items → `evaluated` (§9). |
| `evaluated` | Every item in `scorecard_items` confirmed in `evaluations/`, or the task has none and the iteration is closed. | — | Terminal. |
| `blocked` | A stop condition (§10) or unanswered spec question. | Director decides: amend spec, split, extend budget, abandon. | Back to the state it left (recorded). |
| `abandoned` | Director decision with reason in the task log. | Worktrees removed. | Terminal. |

Rework rule: any return to `in_progress` for a medium/high task passes again through
`self_verified → tests_joined → reviewed`; the re-join re-runs the acceptance tests and the
reviewer reviews the delta since its last verdict (O-3). Checked: director (transition records).

## 4. Tiers

The tier is the highest that applies (CHARTER §4). A task with any `scorecard_items` is at least
medium (a scorecard item is a feature). Planner proposes, director confirms at `specified`.

| Tier | Typical change | Required evidence before `queued` | Approves |
|---|---|---|---|
| low | docs, tests, internal refactor in one module | gate green; implementer's tests | nobody beyond the gate and queue |
| medium | feature in one crate | + acceptance tests fail on base and pass on the joined branch; reviewer `approve` | reviewer |
| high | §3 contracts, command schema, persistence/save path, CI and gate, dependencies, ≥2 layers | + director approval row `D-NNN` in `docs/decisions.md`; ADR in `docs/adr/` if a §3 contract changes | reviewer + director |

Checked: director at `specified` (tier vs `touches`: paths in two layers, `xtask/`, `.github/`,
`Cargo.toml`, `Cargo.lock`, `kiln-commands` schema ⇒ high) and before queueing (evidence present).

## 5. Dispatch and parallelism

1. Dispatch only `ready` tasks. Checked: director.
2. Two in-flight tasks (`in_progress` … `queued`) may not declare overlapping `touches`. Paths
   overlap if equal or one is a directory prefix of the other. Checked: director at dispatch.
3. A change to a CHARTER §3 contract is its own high-tier task, merged before the tasks that
   consume it (P1). Checked: director at `specified`; reviewer flags a hidden contract change.
4. The concurrency cap (in-flight tasks) is set in `STATE.md`; v1 starts at 3 and changes only at
   a retrospective. Checked: director.
5. Each dispatch message gives: the role brief, task ID, run ID, worktree path, the run's token
   allotment (all runs of a task sum to ≤ `budget_tokens`). Checked: director.
6. Before the join, the diff of `task/T-NNN` against its base stays inside `touches` plus the
   task file; after it, also the task's `acceptance_tNNN*` files, its scorecard items and
   `reviews/T-NNN-*`. Checked: director at join and before queueing
   (`git diff --name-only main...task/T-NNN`); target: mechanical in `xtask queue` (O-4).

## 6. Independent-test protocol (medium, high)

- The planner writes acceptance criteria as observable behaviour through a public interface
  (command + JSON params/result, CLI, control protocol, or a public Rust API declared in the spec).
- The test author works from the task file alone, on `task/T-NNN-tests` from the same base. Files
  are named `acceptance_tNNN*.rs` under a crate's or app's `tests/`; it may also set the `tests`
  list of the task's `scorecard/items/*.toml`. Nothing else.
- The implementer does not receive the acceptance tests before the first join, so the join
  measures what its own tests missed (C3). Neither side reads the other's branch. Checked:
  unchecked (both branches live in one repository).
- The implementer never edits `acceptance_t*` files or `scorecard/`. Checked: director at join
  (path filter on the implementer branch diff).
- Fail on base: the test author records each acceptance test's result on its own branch (base +
  tests). Each AC needs ≥1 test that fails by **assertion or command error**; a compile failure
  counts only as weak evidence and obliges the reviewer to do a revert check (O-5).
- Join (director): merge `task/T-NNN-tests` into `task/T-NNN`; run the acceptance tests, then
  `cargo xtask ci`; append a `join` record (§13). Acceptance failures go back to the implementer
  with the output; the implementer either fixes the code or files a spec question. A test that
  looks wrong is resolved by the director (with the planner if needed), never by editing it on
  the implementer branch.
- Low tier: the implementer writes the tests; gate only.

## 7. Review protocol

Low: no reviewer. Medium: one reviewer run on the joined branch. High: reviewer, then director
approval recorded as a `D-NNN` row; an ADR when a §3 contract changes.

The reviewer fills `templates/review.md` into `reviews/T-NNN-rN.md`: behaviour against each AC;
inert code paths (wired but no effect); vacuous tests (no observable assertion, early return,
mocks that make the test pass regardless); scope (`touches`, hidden contract change); determinism.
It runs the gate and may run the product headlessly. Verdict `approve` or `request-changes`.
Each finding is classed **defect** (behaviour wrong), **spec gap** (spec silent or wrong),
**test gap** (an AC or edge not tested), **nit** (style). Any defect or test gap ⇒
`request-changes`; spec gaps go to the director. Checked: director reads the verdict before
`reviewed`; finding counts are in front matter (mechanically countable).

## 8. Queue protocol

Only `cargo xtask queue T-NNN` moves `main`: rebase on `main`, full gate on the rebased result,
fast-forward only if green, append to `metrics/queue.jsonl`. One at a time. Checked: mechanical
(the queue refuses red); direct pushes to `main` are visible in history and are incidents.

On failure the task returns to `in_progress` with the queue log path in the task log.
If `main` is found red anyway (the GitHub Actions re-check, or a local run): stop dispatching and
queueing other work, open `incidents/INC-NNN-*.md`, land the fix or a revert through the queue,
write the post-mortem with a permanent fix that is a mechanical check or a justified "rule only".
Red-main minutes are measured from the incident front matter (§13).

## 9. Evaluator protocol

At the end of each iteration the director starts a fresh evaluator run. It receives the scorecard
(`cargo xtask scorecard` output), the CLI/control-protocol docs and the UI snapshot mechanism —
not source or tests. For each item with status `implemented` it tries the behaviour through a
public interface, records what it ran and observed, and confirms or rejects. It looks
specifically for features that are wired but do nothing. Output: `evaluations/iter-N.md`
(template); its `confirmed_items` are what `cargo xtask scorecard` reads to grant `verified`
(O-6). Each rejection becomes a `draft` task with `found_by: evaluator` and `origin_task`.
Nobody edits scorecard status by hand. Checked: mechanical (`scorecard --check` in the gate).

## 10. Autonomous loop and stop conditions (implementer; test author per its brief)

Loop: read the task file → restate the ACs in the log → plan → edit → build → test → inspect the
behaviour through the headless interface (`kiln-cli`, control protocol) → fix → repeat; finish with
`cargo xtask ci`. A **gate attempt** is one run of `cargo xtask ci`.

Stop and write a blocked report (a log entry starting `BLOCKED:`; run outcome `blocked`) when:
1. 3 consecutive gate attempts fail on the same failure (same step and same test or error);
2. the run's token allotment is exceeded;
3. a needed change lies outside `touches`;
4. a CHARTER §3 contract change is needed;
5. an acceptance test or the spec looks wrong (a `QUESTION:` line in the log).
Conditions 2–5 also bind the test author; its gate is expected to fail at its own acceptance
tests, so condition 1 applies to any other failure. Reviewer, evaluator and analyst stop at 2.
Checked: director from the log and run record (1–2 agent-reported, O-7); 3 by diff at join.

## 11. Budgets

`budget_tokens` is the total across all roles for the task. Starting values: low 150k, medium
400k, high 700k. The director records actual tokens, wall time, tool calls and gate attempts for
every run in `metrics/tasks.jsonl`. An overrun stops the run; the director decides (split,
re-spec, extend — extension recorded in the task log) and the overrun is counted. Values are
recalibrated only at a retrospective from the recorded distribution. Latency budgets are gate
steps (CHARTER §4); an unenforced budget is marked as such, not counted as held.

## 12. Continuity and provenance

- `STATE.md` (director): iteration, cap, in-flight tasks, next steps. `handoffs/` (director, one
  per session). `docs/decisions.md` (approvals, exceptions). `docs/adr/` (contract changes).
- Task file: state in front matter (director only); per-role log sections at the bottom. Each run
  appends to **its role's section** before it stops (separate sections keep the two branches
  mergeable at join). An agent's last act: log entry written, worktree building.
  Checked: director, before recording the run (no log entry ⇒ run marked incomplete).
- Commits carry `Task: T-NNN` and `Role: <role>` lines (`implementer`, `test-author`,
  `reviewer`; planner, evaluator and analyst files are committed by the director as `director`). Checked: director before queueing
  (`git log --format=%B`); target mechanical (O-4).
- Agents never write model names or session identifiers into code comments or docs; runs are
  identified by `R-NNN`. Checked: unchecked in v1 (a grep is possible but not specified).

## 13. Metrics

Records: `metrics/tasks.jsonl` (director via `cargo xtask metrics`), one JSON object per line:
`kind:"run"` {task, iteration, run, role, tier, started_at, ended_at, wall_s, tokens, tool_calls,
gate_attempts, outcome, missing_context}; `kind:"transition"` {task, iteration, at, from, to};
`kind:"join"` {task, at, acc_total, acc_fail_base, base_fail_kind, acc_fail_first_join,
acc_defects, impl_tests_green}. `metrics/queue.jsonl` (`xtask queue`): {task, attempt,
started_at, ended_at, base_sha, head_sha, result, failed_step}. Field names here are the
contract for `xtask metrics` (O-8).

| Metric | Definition | Source |
|---|---|---|
| Lead time | `at` of transition → `ready` until `ended_at` of the queue record with `result:"merged"` | tasks.jsonl transition; queue.jsonl |
| First-pass queue success | tasks whose `attempt:1` queue record is `merged` ÷ tasks queued | queue.jsonl |
| Gate attempts per task | Σ `gate_attempts` over the task's runs (queue runs counted separately) | tasks.jsonl run |
| Acceptance-caught defects | Σ `acc_defects` where `impl_tests_green` was true at first join (failures the director classed as implementation defects) | tasks.jsonl join |
| Review findings by class | Σ `findings.{defect,spec_gap,test_gap,nit}` by tier | `reviews/*.md` front matter |
| Escaped defects | tasks with `found_by` ∈ {evaluator, task, incident} per iteration of discovery, grouped by `origin_task` and its tier | task front matter |
| Rework ratio | tasks with `origin_task` set ÷ all tasks, per iteration | task front matter |
| Tokens per merged task | Σ run `tokens` of merged tasks ÷ merged tasks, by tier | tasks.jsonl; task state |
| Share verified | `verified` items ÷ all items | `cargo xtask scorecard` |
| Red-main minutes | Σ (`green_at` − `red_from`) in minutes | `incidents/*.md` front matter |
| Budget overruns | tasks with Σ run `tokens` > `budget_tokens`, count and share by tier | tasks.jsonl; task front matter |

Complete metrics (iteration 1 exit) means: every run has a run record and a log entry; every
state change has a transition record. Checked: director at iteration close.

## 14. Iterations and retrospective

Iterations and exit criteria are in CHARTER §5. At iteration end: evaluator run (§9); then a
fresh retrospective analyst run with `templates/retrospective.md`. It scores each claim C1–C8 as
supported / refuted / insufficient data, with the numbers and the command that produced them;
lists where the methodology was wrong (rules not followed, rules that did not help, missing
rules); proposes changes, each citing a metric or incident. The director accepts or rejects each
in `docs/decisions.md`, applies accepted ones, bumps the version (v1.0 → v1.1) and adds a
`CHANGELOG.md` entry. Names fixed by CHARTER §4 change only by ADR. Checked: director; any
methodology diff outside a retrospective commit is visible in history and is a process incident.

## 15. Open questions in v1 (interim handling stated above)

- O-1 Integrator role in CHARTER §4 has no separate agent; director holds it.
- O-2 Whether `cargo xtask worktree new` accepts `T-NNN-tests`.
- O-3 Re-review scope after a queue failure is not set by decision 5; v1 requires a delta review.
- O-4 `touches`, acceptance-path and commit-line checks are director-run until xtask has them.
- O-5 Fail-on-base for Rust-API tests that cannot compile on base.
- O-6 Location and format of evaluator confirmations read by `xtask scorecard`.
- O-7 Agents may not observe their own token use; the director enforces allotments.
- O-8 `xtask metrics` must use the field names in §13.

## 16. Rule → how it is checked

| Rule | § | Check |
|---|---|---|
| Gate `cargo xtask ci` green before `self_verified` and in the queue | 3, 8 | mechanical |
| Only `cargo xtask queue` moves `main`; never pushed red | 8 | mechanical (queue); director (history) |
| Scorecard status derived, never hand-edited | 9 | mechanical (`scorecard --check`) |
| Layers, determinism, skip ban, command schema | AGENTS.md | mechanical (gate steps) |
| Only `ready` tasks dispatched; `depends_on` merged | 5 | director |
| No overlapping `touches` in flight | 5 | director |
| Diff stays inside `touches` | 5 | director (scripted diff); target mechanical |
| Tier is the highest applicable; scorecard items ⇒ ≥ medium | 4 | director |
| Contract change is its own high task, ADR | 5, 4 | director; reviewer |
| ACs observable through a public interface | 6 | director at `specified` |
| Implementer never edits acceptance tests or `scorecard/` | 6 | director (path filter at join) |
| Test author never reads the implementation, and vice versa | 6 | **unchecked** |
| Acceptance tests fail on base, pass on joined branch | 6 | director (join record) |
| One builder role per run per task | 2 | director |
| Medium review verdict; findings classified | 7 | director |
| High approval row in `docs/decisions.md` | 7 | director |
| Stop conditions 1–2 | 10 | director (agent-reported) |
| Stop condition 3 (outside `touches`) | 10 | director (diff) |
| Token budget per task | 11 | director |
| Log entry before stopping; worktree building | 12 | director |
| `Task:` / `Role:` lines on commits | 12 | director; target mechanical |
| No model names or session IDs in code or docs | 12 | **unchecked** |
| Red main ⇒ incident with post-mortem | 8 | director |
| Methodology changes only at a retrospective | 14 | director (history) |
| Never weaken a test or gate to get green | AGENTS.md | reviewer; partly mechanical (`no-skip`) |
