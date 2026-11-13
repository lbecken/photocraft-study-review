# Kiln methodology

Version **v1.2** · adopted by D-091 from retrospective 1 (`retrospectives/iter-1.md`, *retro-1*;
retrospective 0 is *retro-0*) · changes only at a retrospective (§14), recorded in `CHANGELOG.md`.

Binding inputs: `docs/CHARTER.md` (claims C1–C8, architecture §3, names and tiers §4),
`docs/adr/`, `docs/decisions.md`; where this file and the charter disagree, the charter wins.
Evidence: tasks (`T-004`), decisions (`D-031`), runs (`R-015`), retro-0 and retro-1 numbers; study
references (`E07-018`, `F1`) point to `/home/user/photocraft-study/deliverables/`.

Every rule names its check: **mechanical** (a command fails), **director** (at a named moment)
or **unchecked** (practice diverges exactly there: E02 §4, retro-0). **(T-033)** / **(T-041)**
marks tooling as it will be once that task merges; *Interim* says what the director does until then.

## 1. Principles

| # | Principle | Why (iteration 0; study) | Claim |
|---|---|---|---|
| P1 | Contracts and checks exist before agents fan out; a contract change is its own task. | Study: planned schema never built while fan-out ran (E04-003, D10). Retro-0: 0 contract changes yet, so untested. | C1 |
| P2 | Only the queue moves `main`, and only on a green gate over the rebased result. | Retro-0: 6/6 first-pass, 0 red-main minutes. Study: 11 of 21 red-main incidents were merge skew (E07-003). | C2 |
| P3 | Whoever builds does not certify: review and evaluation come from other agents; acceptance tests are written from the spec before or apart from the code, and on high tier and one arm of the medium experiment (§6) by a different agent. | Retro-0: review found 7 defects, independent tests 1 (T-004, D-029), at similar cost. Study: vacuous tests (E07-018). | C3 |
| P4 | Status is computed from behaviour tests plus an independent evaluator, never declared. | Retro-0: the evaluator withheld 3 of 25 `implemented` items (T-010, T-011). Study: "625/625 live" (E07-013). | C4 |
| P5 | Review depth follows a tier set by the boundary the change crosses (ADR-0002). | Retro-0: 5/5 tasks high, 7/7 defects in boundary tasks (T-002, T-004, T-005). Study: review never blocked a merge (F12). | C5 |
| P6 | Every rule names its check; the director's own steps included. | Retro-0: 14 director errors, 0 caught mechanically at the time. | all |
| P7 | Budgets gate: a task over budget merges only on a recorded extension. | Retro-0: T-004 1.19×, T-005 2.39×, neither stopped nor extended (C6 refuted). Study: 0/102 budgets enforced. | C6 |
| P8 | The repository is the only memory; commit early. | Retro-0: work survived a restart and a usage stop (D-022, D-024); only uncommitted work (8–25 min) was lost. | C7 |
| P9 | "Correct" is defined by constructed oracles and checked through the CLI or control protocol. | Retro-0: 3 items verifiable only through a Rust API stayed unconfirmed (T-010, T-011). | C8 |
| P10 | Stop and report instead of pushing through. | Retro-0: stops on D-016 (→ D-017) and on T-003's contradictory spec (D-020) each cost one short run. | C6, C7 |

## 2. Roles

Every role but the director runs as a **separate agent session with no shared memory**, given its
brief (`methodology/roles/<role>.md`), the task ID (or `iter-N`) and a run ID `R-NNN`. No run
holds two roles on one task. Checked: director, from the dispatch record (§2.1).

| Role | Does | Never |
|---|---|---|
| director | Plans iterations; accepts specs; dispatches; makes rulings; joins; approves high tier as owner-delegate; runs the queue; records metrics; writes `STATE.md`, handoffs. Holds the charter's integrator role (O-1 closed). | Writes product code, tests or a spec it then accepts (T-012 broke this). |
| planner | Task specs: Pinned section, observable ACs, interface, `touches`, tier, budget; spec checks before dispatch (§5). | Writes code or tests. |
| implementer | Code and unit tests on `task/T-NNN`; on odd-numbered medium tasks also the acceptance tests (§6). | Edits acceptance tests on its own branch, or `scorecard/`; leaves `touches`. |
| test author | High and even-numbered medium tasks: acceptance tests from the spec alone on `task/T-NNN-tests`. | Reads the implementation; edits production code. |
| reviewer | Medium/high: reviews the joined branch (§7). | Rewrites the code or tests. |
| evaluator | End of iteration: exercises the product through public interfaces only (§9). | Reads source or tests to decide a verdict. |
| retrospective analyst | Scores C1–C8 from records; proposes versioned changes (§14). | Edits the methodology or records. |

### 2.1 Director rules

| Rule | Why | Check |
|---|---|---|
| A ruling that changes behaviour is written as a diff to the task file's Interface, ACs or Pinned section and committed on the task branch(es). It states observable behaviour, never a mechanism; a bound it states or asks a document to state (time, memory, size) cites the measurement it rests on. Each builder acknowledges a ruling in its log (accepts or questions it) before acting on it. | Retro-1: 14 ruling errors, 9 reached a builder, 9 caught by agents; 3 bounds without a measurement, 3 pinned mechanisms. | director, when recording the builder's run (acknowledgement in its log); corrected rulings counted at the retrospective |
| Builders' questions are answered to both builders at once, in the task file, before the join. | D-027 avoided a triage cycle; T-001's three gaps were found by three roles separately (D-013). | director |
| The approval row for a high-tier merge is the control the owner chose; the director writes it as the owner's delegate, verifies it is on `main`, then queues. | D-032: a failed command skipped the row and the next line queued T-004. D-091 keeps the row. | mechanical (queue `approval`) |
| Commit ledger files by explicit path; never `git add -A`. One command per line when a later step depends on an earlier one. | D-032 (a chained command queued after a failed step); the director's iteration-0 operating notes in `STATE.md`. | partly mechanical (queue `ledger`, `main-history`) |
| Never set or export `CARGO_TARGET_DIR`; run `cargo xtask …` from the checkout it is about. | D-019: the join measured main's binary; 15 false failures; triage R-015 cost 140,270 tokens. | mechanical (stale-binary guard, same repository only) |
| Every timestamp comes from the clock at the moment it happens. | 17 whole-minute runs, 2 impossible transition orders, D-032's failed line. | mechanical (`metrics` stamps `now`); any time typed by hand: `date -u +%Y-%m-%dT%H:%M:%SZ`, never memory |
| The dispatch message names the run id, and the model is the owner's D-006 choice; free disk ≥ 15 GB is checked first. A run is booked when it ends, before any later transition of its task is written (mechanical: `metrics add` refuses an end time earlier than the task's latest record). An agent stopped by a usage limit is resumed, and a resume is scheduled at every usage-limit stop; after a container restart the director runs `git status` in every worktree, then relaunches the agents fresh. | R-024 ran on another model and has no record (D-024); 6 GB free after 9 GB of scratch builds (D-022). Retro-1: agents were resumed (D-038; R-149, R-151); about 6.8 h of 14.76 h had no agent running; one stop had no check-in. | director; retro counts runs without a record and idle gaps |
| A budget overrun is decided in an explicit `Extend T-NNN to <N>` row (or split, re-spec, abandon), at the moment §11 names. | T-004, T-005 overran with no record. | mechanical (queue `budget`); director at the round-1 verdict (§11) |

## 3. Task lifecycle

One task = branch `task/T-NNN` = worktree `.worktrees/T-NNN` (`cargo xtask worktree new T-NNN`)
with its own `target/`; acceptance tests on `task/T-NNN-tests` (`worktree new T-NNN-tests`). Only
the director changes `state`, on `main`, with a `transition` record (§13) each time.

| State | Entry criteria | Work | Exit → next |
|---|---|---|---|
| `draft` | Director asks for a task, or an evaluator rejection, incident or review follow-up yields one. | Planner writes the spec. | Template complete → `specified`. |
| `specified` | Every section filled; Pinned answered; every scorecard item has an AC through CLI or control protocol. | Director checks tier (§4), `touches`, ACs, budget, `depends_on`; spec check and split where §5.7 asks. | Accept → `ready`; reject → `draft`. |
| `ready` | Accepted; every `depends_on` task `merged`. | — | Dispatched (§5) → `in_progress`. |
| `in_progress` | Worktree(s) exist; builders dispatched. | Loop (§10); questions logged by the first gate attempt and answered to both (§2.1). | Implementer reports gate green → `self_verified`. |
| `self_verified` | `cargo xtask ci` green in `.worktrees/T-NNN`; implementer log entry. | Low: nothing. Medium/high: acceptance tests committed; director joins (§6). | Low → `queued`. Join passes → `tests_joined`; acceptance fails → `in_progress`. |
| `tests_joined` | Join record: fail on base, pass joined, guards pass, gate green. | Review (§7). | `approve` → `reviewed`; `request-changes` → `in_progress`. |
| `reviewed` | Verdict `approve` in `reviews/`; high: approval row on `main`. | — | Director runs the queue → `queued`. |
| `queued` | Queue running for this task (one at a time). | `cargo xtask queue T-NNN`. | Green → `merged`; red → `in_progress` with the queue log. |
| `merged` | `main` fast-forwarded; `queue.jsonl` record `merged`. | Worktrees removed. | Evaluator confirms its items → `evaluated` (§9). |
| `evaluated` | Every item in `scorecard_items` confirmed, or none and the iteration closed. | — | Terminal. |
| `blocked` | A stop condition (§10) or unanswered question. | Director: ruling, split, extend, abandon. | Back to the state it left (recorded). |
| `abandoned` | Director decision with reason in the task log. | Worktrees removed. | Terminal. |

Rework rule: a medium/high task back in `in_progress` passes again through `self_verified →
tests_joined → reviewed`. Mechanical (`metrics transition` refuses other moves; retro-0: 7 skips).

## 4. Tiers (ADR-0002, CHARTER §4)

The tier is the highest that applies. Planner proposes, director confirms at `specified`.
- **high**: product code on the file system's persistence path (writes, renames, temp files,
  paths that reach disk) or that spawns processes; any gate path (the queue's list,
  `docs/development.md`); a security or confinement boundary, including a security property a
  document states; adding or changing a dependency (any `Cargo.lock` or dependency-table change,
  workspace-internal edges included); a change to a CHARTER §3 contract.
- **medium**: the default for a behaviour change that fits no high criterion. Adding a command
  is not a contract change; changing how the registry, schema validation or command log work is.
- **low**: documentation, tests or refactoring with no behaviour change. Scorecard items ⇒ ≥ medium.

| Tier | Acceptance tests | Review | Approval before `queued` |
|---|---|---|---|
| low | implementer's own; gate only | none | none |
| medium | by task number (§6): even → independent test author, odd → implementer | one full round: every AC through the public interface + mutation spot-check of ≥ 10 seeded bugs; later rounds per §7 | reviewer `approve` |
| high | independent test author | as medium + adversarial probing of the boundary | + director row `Approve T-NNN` (the owner's control, §2.1); ADR if a §3 contract changes |

Why: in iteration 0 every task was high, because `Cargo.lock` or "two layers" made almost any
feature high; tiers differed only by an approval row, so C5 could not be tested. Checked: director
at `specified`; mechanical in part: the queue refuses a non-high task that changes a gate path or
a dependency (`gate-path`, `dependencies`) and a high task without its row (`approval`).

## 5. Dispatch and parallelism

1. Dispatch only `ready` tasks. Mechanical (`worktree new` refuses). Why: T-001/T-002/T-005.
2. Two in-flight tasks (`in_progress` … `queued`) may not declare overlapping `touches` (equal, or
   one a directory prefix of the other). Checked: director at dispatch.
3. A CHARTER §3 contract change is its own high-tier task, merged before its consumers (P1).
   Checked: director at `specified`; reviewer flags a hidden one.
4. Cap: at most 3 builder agents compiling at once (CHARTER §4); a short test-author fix round
   counts as one. Checked: director (dispatch records). Why: 4 live runs at 02:18 (retro-0, I).
5. The dispatch message gives the role brief, task ID, run ID, worktree path, tier and who writes
   the acceptance tests. No token allotment: agents cannot observe their usage (O-7).
6. The task branch's diff stays inside `touches` plus the allowed paths. Mechanical (queue
   `touches`). The implementer branch adds no `acceptance_t*` file and nothing under `scorecard/`:
   director at join (`git diff --name-only main...task/T-NNN`).
7. A spec written before all its `depends_on` merged gets one **spec check** (a planner run that
   reads it against `main` as it then is and lists what would stop a builder) before dispatch,
   whatever its tier; builders do not repeat it. A high-tier spec bundling independent items is
   split before dispatch. Checked: director at `ready`.

## 6. Acceptance-test protocol (medium, high)

- **Interface.** Every scorecard item has ≥ 1 AC and ≥ 1 acceptance test through the CLI or control
  protocol; Rust-API tests are additional (T-010, T-011). Checked: director at `specified`;
  evaluator `not_evaluated`.
- **Files.** `acceptance_tNNN*.rs` directly in a crate's, app's or `xtask/`'s `tests/`, on
  `task/T-NNN-tests` from the implementer's base. Tests meant to pass on base are regression guards
  in `guard_tNNN*.rs` (D-010); the join runs them on base and joined branch (D-021).
- **Scorecard.** Only the director edits `scorecard/`: the test author lists each item's tests in
  its log; the director ties them to the items on the task branch at the join, which regenerates
  `docs/scorecard.md` **(T-033)** (*Interim:* the director regenerates it). Checked: at join.
- **Fail on base.** Each AC has ≥ 1 test that fails on base by assertion or command error. Tests
  that drive the built binary or dispatch a command by name fail that way even for new API and are
  preferred. A compile-only failure is weak evidence, made up for by the reviewer's mutation
  spot-check (§7; D-011, O-5 closed). There is no revert check and no stub/logic commit split.
- **Self-check.** Before handing over, the test author runs its tests against a naive stub and ≥ 5
  seeded bugs and lists the kills in its log; a negative test first shows that the forbidden target
  exists and afterwards that it is unchanged (T-004 TG-1). Checked: reviewer (§7 counts).
- **Pins and documents.** No test pins a message text, a whole document or a command list that a
  drafted task changes; "this task left X alone" is a reviewer's diff check, not a guard. A
  `docs/protocol/` sentence stating a guarantee or a limit is written in the spec's Pinned section
  with its evidence and copied by the implementer; an implementer who has to write one the spec
  lacks records in its log the command and output that show it. A task that changes the CLI updates
  its synopsis and exit codes in `docs/control-protocol.md`; only kinds a registered command can
  produce are published (how: T-038). Checked: reviewer (§7).
- **Independence.** The implementer does not see independent tests before the first join; neither
  side reads the other's branch. Checked: unchecked (O-12).
- **Join** (director): `cargo xtask join T-NNN [--acc-defects N]` merges the tests branch, runs
  acceptance tests and guards, then the gate, and appends a `join` record. Failures go back to the
  implementer, who fixes the code or logs a `QUESTION:`. A test that looks wrong is settled by a
  ruling (§2.1), never by editing it on the implementer branch.
- **Experiment arm** (D-035 (16)). High and even-numbered medium tasks get an independent test
  author; on odd-numbered medium tasks the implementer writes the acceptance tests through the
  public interface in the same run, first on `task/T-NNN-tests` (committed, failing on base), then
  the code, and the reviewer's mutation spot-check is the only independent check.

## 7. Review protocol

Depth follows the tier (§4). The reviewer fills `templates/review.md` into `reviews/T-NNN-rN.md`.

**Round 1 is a full review**: each AC observed through the public interface; inert paths; vacuous
tests; scope and hidden contract changes; determinism; and, against the built binary, every
sentence the task adds or changes in `docs/` and every sentence in `docs/protocol/`,
`docs/control-protocol.md` or the generated `error_kinds` naming the same command, option or kind.
Plus a **mutation spot-check**: seed ≥ 10 bugs one at a time, run the tests, discard each edit,
record `mutants: {introduced, equivalent, killed, killed_by_acceptance, survived}` in front matter;
a surviving non-equivalent mutant is a test gap. High adds **adversarial probing** of the boundary
(symlinks, long names, hostile repositories, poisoned caches, symbolic shas).

Findings are **defect** (behaviour wrong), **spec gap**, **test gap** or **nit**. Round 1: any
defect or test gap ⇒ `request-changes`; spec gaps go to the director for a ruling (§2.1).

**Stopping rule.** Round 2 reviews the delta since `previous_sha`, confirms round-1 findings
resolved, and is final. A defect found anywhere still blocks (`request-changes`); test gaps, spec
gaps and nits outside the delta go to the Follow-ups list with `approve`, and the director makes
follow-up tasks with `origin_task`. Round 3 only confirms the fix of a round-2 defect; there is no
round 4: the director approves with recorded follow-ups, splits or abandons. After a queue failure,
review the delta (O-3). Checked: gate `hygiene` (front matter); director.

## 8. Queue protocol

Only `cargo xtask queue T-NNN` moves `main`: rebase on `main`, main's gate on the rebased result,
fast-forward only if green, append and commit `metrics/queue.jsonl`. One at a time. Preconditions
are in `docs/development.md`; a refusal is written to `metrics/queue.jsonl` **(T-033)** (*Interim:*
the director records it in a decision row). GitHub Actions runs the gate on pushes to `main` except
those that change only ledger paths **(T-041)**; *Interim:* every push. Checked: mechanical.

A ledger input that changes generated output (a scorecard item, an evaluation) reaches `main`
only as a low-tier publish task through the queue (`cargo xtask publish`; D-034).

If `main` is found red: stop dispatching and queueing, open `incidents/INC-NNN-*.md`, land a fix
or revert through the queue, and name a permanent fix (a mechanical check or a justified "rule only").

## 9. Evaluator protocol

At the end of each iteration a fresh evaluator run (booked to `iter-N`) receives the scorecard
(`cargo xtask scorecard`), the CLI and control-protocol docs and the UI snapshot mechanism — not
source, tests, logs or reviews. For each `implemented` item it tries the behaviour through a
public interface, records what it ran and saw, and confirms or rejects; it hunts for features
wired but inert. Output `evaluations/iter-N.md`; `confirmed_items` is what `scorecard` reads to
grant `verified`. Each rejection, inert feature or wrong behaviour becomes a `draft` task
(`found_by: evaluator`, `origin_task`) or a question to the director; iteration 0 left 2 unfiled.
An item no interface exposes is `not_evaluated` with a filed task (T-010, T-011). Checked:
mechanical (`scorecard --check`); director (nothing unfiled).

## 10. Autonomous loop and stop conditions

Loop: read the task file → restate the ACs in the log → plan → edit → build → test → observe
through `kiln-cli` or the control protocol → fix → repeat; commit at each working step; finish
with `cargo xtask ci` (one run = one **gate attempt**). `QUESTION:` lines are logged by the end of
the first gate attempt; work continues on the rest.

Stop and write a blocked report (a log entry starting `BLOCKED:`; run outcome `blocked`) when:
1. 3 consecutive gate attempts fail on the same failure (same step and same test or error);
2. a needed change lies outside `touches`;
3. a CHARTER §3 contract change is needed;
4. an acceptance test, a ruling or the spec looks wrong or contradictory.
Conditions 2–4 bind the test author too. Why: both iteration-0 stops (D-017, D-020) cost one
short run and exposed a director error. Checked: director (1 agent-reported); 2 mechanical
(queue `touches`). There is no token stop: agents cannot see their usage (O-7).

## 11. Budgets

`budget_tokens` covers building and review runs only; planner runs and spec checks go to `iter-N`
(§13). Ceilings: **high 1,100,000, medium 700,000 (provisional), low 150,000** (retro-1: p75 of the
tier, rounded up to 50k). The queue refuses a task over its ceiling without an `Extend T-NNN to
<N>` row (mechanical, `budget`). When round 1 is `request-changes` and the task has spent 75 % or
more of its ceiling, the director writes, before any rework run, a row beginning `Extend T-NNN to
<N>` that sets N as the cap for the rework and says which findings go to a follow-up; rework that
would pass N stops and is split. In every other case (low tier, an overrun inside round 1, a
run lost to a stop) the director writes the row before dispatching the next run of a task that
has reached its ceiling. Checked: director at the verdict and at dispatch. Latency budgets are gate steps
(CHARTER §4); an unenforced one is not "held". Recalibrate only at a retrospective.

## 12. Continuity and provenance

- `STATE.md`; `handoffs/` (one per director session, start and end time); `docs/decisions.md`; ADRs.
- Task file: state in front matter (director only); each run appends to **its role's log
  section** before it stops. Last act: log entry written, work committed, worktree building.
  Checked: director before recording the run (no entry ⇒ run marked incomplete).
- Builder commits carry `Task: T-NNN` and `Role: implementer|test-author|reviewer`; director
  commits that record another role's output carry `Task: T-NNN|iter-N` and `Role: director`;
  other director commits need none (67/164 lacked them). Checked: director before queueing.
- No model names or session identifiers in code, docs or records. Mechanical for session paths
  (gate `hygiene`); the director greps the diff for the rest (4 review files had them).

## 13. Metrics

Records in `metrics/tasks.jsonl` (via `cargo xtask metrics`), one JSON object per line:
`kind:"run"` {task, iteration, run, role, tier, started_at, ended_at, wall_s, tokens, tool_calls,
gate_attempts, outcome, missing_context}; `kind:"transition"` {task, iteration, at, from, to};
`kind:"join"` {task, at, acc_total, acc_fail_base, base_fail_kind, acc_fail_first_join,
acc_defects, impl_tests_green}. `metrics/queue.jsonl` (queue): {task, attempt, started_at,
ended_at, base_sha, head_sha, result, failed_step}. These names are the tool's contract (O-8).

`metrics add/transition` stamp `now`, refuse an `--at` earlier than the task's last record or a
transition not in §3 or `--tokens 0` without `--tokens-unknown`, and book planner, evaluator and
analyst runs to `iter-N`; a spec check gets its own role **(T-033)** (*Interim:* role `planner`,
note beginning `spec check:`). The join counts acceptance targets by path and refuses a failing
record without `--acc-defects` **(T-033)** (*Interim:* the director passes it at every failing
join). The director's cost is not measured. `found_by`: who found a defect (`evaluator`,
`reviewer`, `incident`, `task:T-NNN`); planned work is `planner`.

| Metric | Definition | Source |
|---|---|---|
| Lead time | `→ ready` transition until the `merged` queue record | tasks.jsonl; queue.jsonl |
| First-pass queue success | tasks whose `attempt:1` is `merged` ÷ tasks queued | queue.jsonl |
| Gate attempts per task | Σ `gate_attempts` over the task's runs | tasks.jsonl run |
| Acceptance-caught defects | Σ `acc_defects` over **every** join with `impl_tests_green`; `acc_defects` is required whenever `acc_fail_first_join > 0`; by arm (§6) | tasks.jsonl join |
| Review findings by class | Σ `findings` by tier and arm | `reviews/*.md` front matter |
| Review rounds per task | highest N of `reviews/T-NNN-rN.md`, by tier | `reviews/` |
| Acceptance kill rate | `killed_by_acceptance` ÷ (`introduced` − `equivalent`), by round, tier, arm | review front matter |
| Escaped defects | tasks with `found_by` ∈ {evaluator, incident, task:T-NNN} and `origin_task`, by origin tier and arm | task front matter |
| Known defects merged by decision | defect findings or filed defects a task was approved with and sent to follow-up (as T-008, T-009, T-030) | review Follow-ups; decision rows |
| Rework ratio | tasks with `origin_task` ÷ all tasks, by iteration of discovery (the follow-up file's first commit) | task files; git |
| Tokens per merged task | Σ run `tokens` of merged tasks ÷ merged tasks, by tier and arm; reported without and with `iter-N` overhead (iteration-0 comparable) | tasks.jsonl |
| Share verified | `verified` items ÷ all items | `cargo xtask scorecard` |
| Red-main minutes | Σ (`green_at` − `red_from`) | `incidents/*.md` |
| Budget overruns | tasks over `budget_tokens`, by tier; each with or without an `Extend` row | tasks.jsonl; decisions |

Complete metrics: every run has a record and a log entry, every state change a transition.
Checked: director at close; analyst recounts.

## 14. Iterations and retrospective

Iterations and exit criteria: CHARTER §5 and ADR-0005. At iteration end: evaluator (§9), then a
fresh retrospective analyst with `templates/retrospective.md`. It scores C1–C8 as supported /
refuted / insufficient with numbers and the commands that produced them; compares the §6 arms;
lists rules not followed, rules that cost and caught nothing, and missing rules; proposes changes,
each citing a metric or incident and naming what it removes. The director decides each in
`docs/decisions.md`, applies them, bumps the version, adds a `CHANGELOG.md` entry. CHARTER §4
changes only by ADR. Checked: director (history; an edit outside a retrospective is an incident).

## 15. Open questions

Closed by retro-0: O-1 director stays integrator; O-2 yes (`worktree new T-NNN-tests`); O-5
mutation spot-check; O-6 `evaluations/iter-N.md` `confirmed_items`; O-7 no in-run token gating;
O-9 publish tasks (§8). By retro-1: O-10 director cost not measured; O-13 mean 1.32 builder runs.
- O-3 Re-review after a queue failure: retro-1 says a ledger-text conflict (T-018) needs none; open.
- O-4 Implementer-edits-acceptance, commit lines, tier vs `touches` beyond gate paths: director.
- O-8 `acc_defects` was never filled in iteration 1 (retro-1); T-033 requires it at a failing join.
- O-11 Model per run: the dispatch record is self-reported; the repository cannot show it.
- O-12 Independence: unchecked for test authors; the implementer-first order held (T-011, retro-1).

## 16. Rule → how it is checked

| Rule | § | Check |
|---|---|---|
| Gate green before `self_verified` and in the queue; only the queue moves `main`; diff inside `touches`; gate paths and dependencies ⇒ high; high has its row; over budget ⇒ `Extend` row | 3–5, 8, 11 | mechanical (queue) |
| Scorecard derived; layers, determinism, skip ban, command schema, mutants lint, session paths; valid transitions, clock times, `iter-N` booking, `ready` at dispatch | 3, 5, 7, 9, 12, 13, AGENTS.md | mechanical (gate; `metrics`; `worktree new`) |
| Spec-check role; join counts by path; failing join classified; queue refusals recorded; page regenerated at join; `join --revert-check` removed; no CI run for a ledger-only push | 6, 8, 13 | mechanical (T-033, T-041); interim director (nobody runs the revert check) |
| Tier by boundary; Pinned answered; CLI/control AC per item; no overlapping `touches`; contract change own task; spec check; bundled high spec split | 3–6 | director at `specified`, `ready` and dispatch; reviewer |
| Rulings state behaviour and measured bounds, acknowledged by builders; questions answered to both; dispatch record, disk; resume or relaunch; `Extend` row at a round-1 `request-changes` at ≥ 75 % | 2.1, 11 | director |
| Guarantee sentences from Pinned; no pins of what a drafted task changes; documents checked against the binary; never weaken a test or gate to get green | 6, 7, AGENTS.md | reviewer; evaluator; partly mechanical (`no-skip`) |
| Test author and implementer do not read each other's branch | 6 | **unchecked** |
| Fail on base and guards; implementer branch adds no acceptance tests or `scorecard/` edits, the director ties items; self-check kills logged; stopping rule; builder cap | 5–7 | mechanical (join); reviewer; director (at join) |
| Stop conditions; log entry; commit lines; evaluator files every observation; red main ⇒ incident; methodology only at a retrospective | 8–10, 12, 14 | director (stops agent-reported) |
