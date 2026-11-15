# Methodology changelog

Each version lists what changed and the evidence behind it. From v1.1 on, every change cites a
metric (file, field, value) or an incident (`INC-NNN`) from this repository; v1.0 cites the
PhotoCraft study (`/home/user/photocraft-study/deliverables/`), because no Kiln data exists yet.

## v1.4 — 2026-11-15

Adopted by D-174 from the retrospective of iteration 2 to cut 1 of stage 2
(`retrospectives/iter-2-stage-2-cut-1.md`, "retro-2c1", run R-254; numbers as in its section 13,
commands in `retrospectives/scripts/iter2s2_metrics.py` and `iter2s2_git.sh`), and by D-175
(INC-001, ADR-0009 with its rulings at acceptance). Tooling: T-033 (3, 4), T-043 (1, 13), T-071 (14).

| # | What changed (METHODOLOGY § unless named) | Decision | Evidence |
|---|---|---|---|
| 1 | A task's tier is at least the highest tier of its items, fixed at `specified`, unchanged in flight; tool check pending T-043 (§4, task template, planner brief). | D-174 (1), D-158, D-162 | T-056 re-tiered at the join; two queue failures followed (D-162). |
| 2 | Rulings live on `main`; on a task branch the director commits only joins and scorecard ties, never front matter; replaces "committed on the task branch(es)" (§2.1, task template, reviewer and implementer briefs). | D-174 (2), D-162 | Front-matter commits on T-056's branch: one refusal, one conflict (D-162). |
| 3 | One role per `touches` path; the owner adds a line the other role needs on a ruling; join check pending T-033 (§5.6, task template, planner, implementer and test-author briefs). | D-174 (3), D-153 | D-153, D-154: 2 queue failures, 18,525 tokens. |
| 4 | A rebuilt branch is confirmed by a tree-identity check by the tool (T-033); until then by the reviewer's confirmation record, which is not a review round (§7, §13, reviewer brief). | D-174 (4) | R-235, R-236, R-245: 23,935 tokens for three confirmations filed as round 3. |
| 5 | Text or colour items: the round-1 differential run uses generated text from a stated alphabet with ligatures, combining marks, joiners and right-to-left text (§6, reviewer brief, review template). | D-174 (5) | T-055's row-text defect passed two rounds with no shaped input; NEW-1. |
| 6 | A figure "to be measured by the implementer" must be measurable by it; otherwise the reviewer measures it (§5.7, planner and reviewer briefs). | D-174 (6) | T-056's QUESTION, ruling y2, round-1 defect #3. |
| 7 | An option a spec check calls unmeasured is probed before the ruling; a resumed drafting planner (preferred for cost) checks facts against main and does not choose alone between its draft and an alternative (§5.7, planner brief). | D-174 (7), D-163 | Q-12 A reversed by b1 (R-246); 34,769 against 366,707 tokens (D-163). |
| 8 | Window evidence in the gate where it can be (Linux procedure under Xvfb in CI on every code push); otherwise the item is procedure-only and the procedure is rerun when the window code changes (§6, reviewer brief). | D-174 (8), ADR-0009 Q-3 | No gate test of `ui.window` opens a window; 7 of 17 T-065 r1 mutants only by a real window. |
| 9 | Mutant kills by gate tests (`killed_by_acceptance`) and by a manual procedure (`killed_by_procedure`) are counted apart (§7, §13, review template, reviewer brief). | D-174 (9) | T-065 r1: 13 = 4 + 9, misread in D-165 and D-167. |
| 10 | A number in a decision row names the command or record it comes from (§2.1). | D-174 (10) | Eight figures in rows and a handoff contradicted by the records (retro-2c1 §6). |
| 11 | The rules of D-153, D-158/D-162 and D-163 are in the text, each once (changes 1, 2, 3, 7). | D-174 (11) | None of them was in `methodology/`; STATE worded D-158 against D-163. |
| 12 | The evaluator's clone leaves out the ledger records it may not read, as far as `cargo xtask scorecard` still runs there (§9, evaluator brief, evaluation template). | D-174 (12) | R-253 printed part of D-165. |
| 13 | Queue refusals are written to `queue.jsonl`, pending T-043 (was marked T-033) (§8, §16). | D-174 (13) | 2 refusals in rows only (D-154, D-162). |
| 14 | New §8.1 Platforms and CI: what "supported" means, CI evidence from hosted runners, who reads and records a CI run, counted jobs from their first green run on main, the `ci/T-NNN` run for platform-sensitive paths, claims from CI logs, the test rule replacing D-166 b8 (T-071) (§8.1, §16, reviewer, evaluator, implementer and test-author briefs, evaluation template). | D-174 (14), D-175, ADR-0009 | ADR-0009 probe a0–e5; 31 Unix-only test files (retro-2c1 §12). |
| 15 | A question the join can answer may be ruled after the join's result, and the ruling cites it (§2.1, task template). | D-174 (15) | T-055 joined 3 min before its rulings; the join's failure was useful evidence. |
| 16 | After every push the director reads and records the CI run; "main is green" means the local gate and CI; red CI on main is an incident at once; red-main minutes are counted from CI runs (§8, §13, analyst brief, retrospective, incident and handoff templates). | D-175 | INC-001: CI on main red since run 257 (ADR-0009 e5; runs 353–357 confirmed, D-175) while the record said 0 red-main minutes. |

Removed: rulings "committed on the task branch(es)"; D-166 ruling b8 (Unix-only test files accepted);
`killed_by_acceptance` as a count that mixed gate tests and manual procedures; "until T-052 merges"
(T-052 abandoned for T-069, D-175).

## v1.3 — 2026-11-14

Adopted by D-144 from the retrospective of iteration 2, stage 1 (`retrospectives/iter-2-stage-1.md`,
"retro-2s1", run R-223). Numbers follow its section 12, which D-144 decides one by one; every
change's source is D-144 and retro-2s1 (evidence column, commands there). Change 12 is tooling in
T-043's scope and is stated as pending it; the cause of change 8 is recorded by T-033.

| # | What changed (METHODOLOGY § unless named) | Evidence (retro-2s1) |
|---|---|---|
| 1 | The `Extend` row at 75 % of a ceiling is dropped; the queue's `budget` check and the row before a run past the ceiling stay (§11, §2.1, §16). | 4 rows, none capping or moving work; 89–98 % of spend before the verdict. |
| 2 | Ceilings high 1,100,000; medium 800,000 (provisional); low 150,000; a draft with an older budget is restated when specified (§11, planner brief, task template). | Pooled p75: 1,076,226; 781,214; 124,444. |
| 3 | A spec check reads the branches in flight for shared traits, types and documents and pins the Rust signatures of criteria tested through a Rust API (§5.7, planner brief). | D-117, D-128/D-132, D-136. |
| 4 | A ruling that changes `touches`, `depends_on` or a state is a front-matter edit in the same commit; the `state` line is written with every transition until `metrics transition` does it (§2.1, §3, task and handoff templates). | D-134, D-139 (two queue refusals). |
| 5 | After a usage-limit stop or a restart, stopped agents are resumed with a restart notice (re-read their uncommitted work first), not relaunched fresh (§2.1, §16, handoff template). | 4 of 4 restarts resumed, nothing committed lost (D-112, D-116, D-129, D-143). |
| 6 | One handoff file per day with timestamped addenda; session start and end times dropped (§12, handoff template). | One file, six addenda; session times never written. |
| 7 | Implementer-first arm: the implementer runs the whole gate, `no-skip` included, on a local merge of its tests branch before reporting (§6, implementer brief). | D-108 (T-021's first join failed at `no-skip`). |
| 8 | Every join that is not green is classified: acceptance-defect count and a cause (§6, §13, §15 O-8). | T-021's record `acc_defects: null` against D-108; 0 of 16 filled. |
| 9 | Where a reference tool is on the machine (git, rustc), one seeded differential run of the built binary by the reviewer in round 1, in every tier, recorded with seed and counts (§6, reviewer brief, review template; D-146 corrects D-144, which gave the high tier to the test author, who has no implementation to run). | NEW-1: 12 hand fixtures against 600 fuzzed patterns. |
| 10 | Pinned sentences stating figures, limits or lists carry their evidence at the spec check (§5.7, planner brief). | 3 of 5 round-1 defects were pinned sentences; D-131. |
| 11 | Review front matter counts `doc_sentences` (§7, §13, review template, reviewer brief). | 21 of 40 round-1 findings, counted by reading. |
| 12 | Pending T-043: every cited D-NNN has a row; `metrics add` refuses a run with start equal to end; `metrics transition` writes the task file's `state` line (§3, §13, §16). | D-104 (14 citations, no row); R-180. |
| 13 | The evaluator may read named fixture data (e.g. `crates/kiln-syntax/tests/corpus/`) and works in a checkout other identities can reach (§9, evaluator brief, evaluation template). | D-141 (R-222); NEW-3. |
| 14 | Gate time (`gate_seconds`) is reported per stage (§13, retrospective template). | Queue gate 241 s → 574 s. |
| 15 | A stage closes only with a decision row listing what of its ADR list was not done and where it went (§14). | D-142 against ADR-0005 (T-033, T-041). |

Also from D-144: the task state `evaluated` is dropped, the published evaluation's
`confirmed_items` being the record, and `merged` is terminal (§3; no task ever entered it); spec
checks are written into the task file, as practice was (§2, §5.7, planner brief, task template).

Removed: the 75 % `Extend` row; "relaunch fresh" after a restart; the handoff's session number and
start and end times; state `evaluated`; output under `tasks/spec-checks/`; the interim "the director
passes `--acc-defects` at every failing join" (replaced by change 8's interim).

## v1.2 — 2026-11-13

Adopted by D-091 from retrospective 1 (`retrospectives/iter-1.md`, "retro-1"). Changes keep the
numbers of retro-1's proposal table, which D-091 decides one by one; the evidence figures are
retro-1's (commands there). Tooling for 1, 8, 9 and 10 is T-033, cut down to those parts; for 14
it is T-041. Until they merge, the rules state an interim step.

| # | What changed (METHODOLOGY § unless named) | Evidence (retro-1) | Claim |
|---|---|---|---|
| 1 | Planner runs and spec checks are booked to `iter-N`; a task's budget covers building and review only; until the tool has a role for it, a spec check is booked as role `planner` with a note beginning `spec check:` (§11, §13, planner brief, handoff template). | 354,110 tokens never booked (D-055); spec checks pushed T-015 and T-008 over budget (286,583; 271,944); T-017's overrun hidden; T-006 and T-014 overruns were planner bookings. | C6 |
| 2 | When round 1 is `request-changes` at 75 % or more of the ceiling, the director writes an `Extend T-NNN to <N>` row before any rework run: N caps the rework, the row names the findings sent to follow-up, and rework that would pass N stops and is split (§11, §2.1). | 5 Extend rows at 100.2–102.3 % of spend, none changing anything; post-round-1 rework 2,194,642 tokens (24.6 %). | C6 |
| 3 | Ceilings: high 1,100,000; medium 700,000 (provisional, three tasks); low 150,000 (§11, task template, planner brief). | p75 of builder and review runs: high 1,073,747, medium 665,024, low 116,752; 7 of 12 over the v1.1 ceilings. | C6 |
| 4 | A ruling states observable behaviour, never a mechanism; a bound cites its measurement; each builder acknowledges a ruling in its log before acting on it (§2.1, all builder briefs, task template). | 14 ruling errors, 9 reached a builder, 9 caught by agents; 3 unmeasured bounds, 3 pinned mechanisms. | C7 |
| 5 | A `docs/protocol/` sentence stating a guarantee or a limit is written in the spec's Pinned section with its evidence, and the implementer copies it; an implementer who has to write one the spec lacks records in its log the command and output that show it (D-091's addition) (§6, planner and implementer briefs, task template (i)). | 6 of 14 review defects and 3 of 3 round-2 defects were documentation sentences; at least 3 of the 6 came from text written for a ruling. | C4, C5 |
| 6 | No test pins a message text, a whole document or a command list that a drafted task changes; "this task left X alone" is a reviewer's diff check (§6, builder briefs). | D-043, D-046 (two guards), D-052, D-069 (blocked run, 190,104 tokens), T-010 r1 nit 3. | C1 |
| 7 | A spec written before its `depends_on` merged gets one spec check before dispatch, whatever its tier; builders do not repeat it (§5.7, §3, planner brief). Also accepted (D-091, from retro-1's budget section): a high-tier spec that bundles independent items is split before dispatch. | T-010, unchecked: both builders stopped, 226,395 tokens for 2 mismatches; checks found 16 blockers for 912,637; T-013 bundled seven items (2.11×). | C1, C6 |
| 8 | The director alone edits `scorecard/`, at the join; the test author lists test names in its log; the join regenerates `docs/scorecard.md` (T-033) (§6, test-author brief). | 2 joins failed at `scorecard` (T-011, T-008); three documents disagreed (D-075). | C2 |
| 9 | The join counts acceptance targets by path and refuses a failing record without `--acc-defects`; queue precondition refusals are written to `metrics/queue.jsonl` (T-033) (§6, §8, §13). | T-018 counted 6 of 12; 28 joins with `acc_defects` null, 2 failing joins unclassified; 2 refusals unrecorded. | C3 |
| 10 | The revert check leaves the briefs and the methodology now, the `join --revert-check` command with T-033 (§6, §16, reviewer brief). | 5 run in iteration 1, 0 defects. | C3, C5 |
| 12 | Resume after a usage-limit stop and schedule a resume at every such stop; after a container restart, `git status` in every worktree and relaunch fresh (§2.1, handoff template). | Rule and practice disagreed (D-038; handoff line 62); 6.8 h of 14.76 h idle; one stop with no check-in. | C7 |
| 13 | "One `role: director` run per session" and the "Director cost" metric are dropped; the director's cost is stated as not measured; the handoff records each director session's start and end time (§2.1, §12, §13, handoff and retrospective templates, analyst brief). | 0 director records in two iterations; O-10. | C7 |
| 14 | No CI run for a push that changes only ledger paths, as a task through the queue because `.github/` is a gate path (T-041) (§8). | About 128 CI runs (handoff; D-091 notes the run number had reached 203); 15 handoff/STATE-only commits. | C2 |
| 16 | Iteration 2 is staged by ADR-0005: a first stage to the 60 % line, then the UI tasks (§14). | Exit criterion missed: 28 of 64 after the evaluation, 11 short. | C4 |
| 17 | Review round 1 checks against the built binary every sentence the task adds or changes in `docs/` and every sentence elsewhere in `docs/protocol/`, `docs/control-protocol.md` and the generated `error_kinds` naming the same command, option or kind (§7, reviewer brief). | The evaluator found 4 mismatches no review found (D-090). | C4 |
| 18 | A task that changes the CLI updates its synopsis and exit codes in `docs/control-protocol.md` (D-037's restriction lifted for those sections); only kinds a registered command can produce are published, how decided in T-038's spec (§6). | NEW-4 (T-011 told not to touch the file); NEW-2 (11 kinds of unbuilt features published). | C4 |

Rejected and deferred: 11 (the queue writes the high-tier approval row from the review) is
rejected, because the approval row is the control the owner chose and the director signs it as
the owner's delegate; removing it is the owner's choice. 15 (ADR text for `docs/CHARTER.md`
committed as ledger) is deferred, because the charter is not a ledger path for the queue's check
of main's history and one occurrence does not justify the tool change; revisit at the second.

Removed: the revert check; "one `role: director` run per session" and the "Director cost"
metric; "relaunched fresh, never resumed"; the check of a ruling by a planner or reviewer run and
the citing of its commit (planner and reviewer briefs' "checking a ruling", task template); the
queue-time Extend interim; booking planner runs and spec checks to tasks; the test author's edit
of scorecard items; interims for tooling T-013 delivered; open questions O-10 and O-13
(answered); the duplicated low-tier line of §6 (§4 states it). Rules in `docs/development.md` and
`AGENTS.md` follow through a low-tier task (D-091).

## v1.1 — 2026-11-13

Adopted by D-035 from retrospective 0 (`retrospectives/iter-0.md`, "retro-0"); changes 1–15 are
its proposals, 16 is the director's. Numbers are retro-0's (commands there). Tooling for 3, 5,
12, 13, 14 is task T-013; until it lands the rules state an interim step.

| # | What changed (METHODOLOGY § unless named) | Evidence | Claim |
|---|---|---|---|
| 1 | Review stopping rule: round 2 reviews the delta and is final; a defect anywhere blocks; gaps and nits outside the delta go to follow-up; round 3 only confirms a round-2 fix; no round 4 (§7, reviewer brief, review template; director's R-054 resolution). | Post-r1 rework 1,196,381 tokens (30.5 %); 18 later-round findings, 0 in submitted code; ad-hoc "last round" D-030, D-033. | C5, C6 |
| 2 | Rulings are task-file diffs checked by a planner or reviewer before builders get them; the decision row cites the commit (§2.1, planner and reviewer briefs). | 3 rulings replaced (D-017, D-030, D-031); 17 of 48 missing-context items created by rulings. | C7 |
| 3 | `metrics` stamps times, refuses invalid transitions and `--tokens 0`, records director runs (§13, §2.1, §3 rework rule). | 17 whole-minute runs, 2 impossible orders, 7 skipped `self_verified`, R-022, D-032, no director record. | C7 |
| 4 | Tier by boundary; review depth tied to tier (§4, §7, CHARTER §4 by ADR-0002). | 5/5 tasks high; 7/7 review defects in boundary tasks; tiers differed only by an approval row. | C5 |
| 5 | Budgets high 850k / medium 500k (provisional) / low 150k; overrun merges only with an `Extend T-NNN` row; token stop for agents removed (§11, §10). | T-004 1.19×, T-005 2.39×, no stop, no record; agents cannot see tokens (O-7). | C6 |
| 6 | Every scorecard item has an AC and a test through the CLI or control protocol (§6, planner brief, task template). | 3 of 25 items unverifiable (T-010, T-011). | C4, C8 |
| 7 | Test-author self-check: naive stub, ≥ 5 seeded bugs with kills logged, negative tests show the target exists and is unchanged (§6, test-author brief). | T-004 TG-1 tests could not fail; later-round acceptance kill rate 21/45. | C3 |
| 8 | Reviewer mutation spot-check (≥ 10) with `mutants` front matter replaces the revert check and the stub/logic commit split (§6, §7). | Revert checks 2/2 caught nothing; mutation found 12/12 test gaps; T-005 reviews lacked counts. | C3, C5 |
| 9 | "Pinned" section in every spec, items (a)–(h) (task template, planner brief, §3 `specified`). | 31 first-round missing-context items; D-020, D-025, threat model only at D-026. | C1, C7 |
| 10 | Questions logged by the first gate attempt; answered to both builders at once before the join (§10, §2.1, AGENTS.md). | D-027 avoided a triage cycle; T-001 gaps found by three roles separately (D-013); 6 test-side join failures. | C3 |
| 11 | Dispatch record at start (model matches D-006), free disk ≥ 15 GB, scratch cleanup, relaunch not resume (§2.1, AGENTS.md, briefs). | R-024 (D-024) without record on another model; 6 GB free after 9 GB scratch (D-022). | C7 |
| 12 | Withdrawn as worded (it left `docs/scorecard.md` stale on `main`): a ledger input that changes generated output goes through the queue as a low publish task, as T-012; T-013 adds a one-command helper (§8). | 5 manual regenerations (D-012); D-034; T-012's self-approved spec. | C2, C7 |
| 13 | `worktree new` refuses a task not `ready`; cap restated as 3 builder agents compiling at once (§5, CHARTER §4). | 3 tasks ran before `ready`; 4 live runs (inferred). | C2, C6 |
| 14 | Metric definitions: acceptance-caught defects over every join, `acc_defects` required on failures; `iter-N` booking for planner/evaluator/analyst runs; "known defects merged by decision"; rework by iteration of discovery; `found_by` names a defect's finder; tokens per task with and without `iter-N` overhead; T-013 owns the mutants lint, session-path check and `iter-N` booking (§13). | Report showed 0 caught vs 2 recorded; T-012 false overrun; T-008/T-009/T-030 deferred defects invisible. | C3, C6 |
| 15 | `Task:`/`Role:` lines bind builder commits and director commits of a role's output; session-path grep targeted for the gate (§12, AGENTS.md). | 67/164 commits without lines, all director's; 4 review files with a session path. | C7 |
| 16 | Experiment arm: odd-numbered medium tasks have implementer-written acceptance tests (committed on `task/T-NNN-tests` first, failing on base) plus the reviewer's spot-check; even-numbered medium and all high tasks keep an independent test author; P3 and CHARTER §5 restated (§6, briefs). | Independent tests: 1 defect for 1.09 M tokens; review 7 for 1.22 M (D-035). | C3, C5 |

Added: a director rules section (§2.1) from retro-0 "The director as a failure source" (14
errors, 0 caught mechanically). Removed: "`Cargo.lock` ⇒ high", "≥ 2 layers ⇒ high", the revert
check and stub/logic split, per-run token allotments and the token stop condition, "any test gap
⇒ request-changes" after round 1, open questions O-1, O-2, O-5, O-6, O-7 (answered), O-9.

Known stale references, fixed by T-013 (not edited in v1.1): `docs/development.md:137` (command
schema ⇒ high), `:144` (O-2, closed), `:149` and `xtask/src/main.rs:180` (export
`CARGO_TARGET_DIR`), `:326-352`, `xtask/src/join.rs:1` and `xtask/src/main.rs:294` (revert
check), `:360` (timestamps passed by hand); draft tasks T-007–T-030 carry v1.0 budgets.

Considered and rejected (retro-0):
- Drop independent test authors for medium tier: one caught defect is too little to keep them on
  defect evidence, but their tests feed the scorecard (C4) and killed 87 % of round-1 mutants;
  wait for medium-tier data (D-035 replaced this with arm 16).
- A separate integrator agent (O-1): the failures were bookkeeping slips; mechanising them
  (changes 3, 11, 13) is cheaper than another agent.
- Longer or second reviewers on high tasks: later rounds already showed diminishing returns.
- Per-run token allotments enforced in-run: agents cannot observe tokens (O-7).
- Raising the concurrency cap: the usage limit, not the cap, stopped work (D-024).

## v1.0 — 2026-11-12

Initial version, written before iteration 0's first queued task. All rules are untested
proposals; the study's own verification lists its process proposals as having no effect data
(`VERIFICATION.md` §5). Starting values (tier budgets 150k / 400k / 700k tokens, concurrency cap
3, stop after 3 consecutive failed gate attempts) are director choices to be recalibrated.

| Principle | Study finding it answers | Measured by (claim → metric, METHODOLOGY §13) |
|---|---|---|
| P1 Contracts and checks before fan-out; contract change is its own task | Planned validated command schema and op-history never built (E04-003); fan-out from day 1 (D03, D10); a field added in one PR broke a concurrent one (E07-003) | C1 → rework ratio, escaped defects traced to contract changes, count of ADRs after iteration 0 |
| P2 Only the queue moves `main` | Semantic merge skew, 11 of 21 traced red-main incidents, no mechanical queue (E07-003, E05-052, F1) | C2 → first-pass queue success, red-main minutes, lead time |
| P3 Builder does not certify | Vacuous tests (E07-018, F4); fixes merged without tests (E07-015) | C3 → acceptance-caught defects; review findings of class test gap |
| P4 Status derived from tests plus evaluator | "625/625 live" measured wiring (E07-013, F5); 66 of 129 settings inert (E07-014, F6) | C4 → share verified; evaluator rejections of `implemented` items; escaped defects |
| P5 Review by risk tier | Review never blocked a merge; ~4 explicit approvals in ~22 sampled PRs (E06-041, F12, D06) | C5 → review findings by class and tier; escaped defects by origin tier; tokens per merged task by tier |
| P6 Every rule names its check | Stated-only rules are where practice diverged (E02 §4, errata #5); gates that could not fail (F11) | all claims → METHODOLOGY §16; retrospective lists drift at unchecked rules |
| P7 Budgets gate | 0 of 102 perf budgets enforced (errata #11, E04-051); effort estimate revised ~10× (E08-005) | C6 → budget overruns, tokens per merged task, enforced latency budgets |
| P8 Repository is the memory | Gitignored devlog and `plan/` (E02-004, E01-005, F14); session trailers on few commits (E06-011, F13); informal decision log (E02-025) | C7 → runs with log entry and run record, transitions recorded, runs reporting missing context |
| P9 Constructed oracles, public interfaces | No reference binary for an IDE (D10 §4); agents rarely verified through the control channel (E04-024) | C8 → escaped defects on items whose tests use an oracle vs not (director classification in v1) |
| P10 Stop and report | Out-of-scope edits caused duplicates and broken builds (E07-019, E06-050); hidden contract changes (E07-003) | C6, C7 → blocked runs by stop condition, budget overruns, out-of-`touches` diffs found at join |

Known gaps carried as open questions O-1 … O-8 (METHODOLOGY §15).
