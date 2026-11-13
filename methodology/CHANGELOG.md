# Methodology changelog

Each version lists what changed and the evidence behind it. From v1.1 on, every change cites a
metric (file, field, value) or an incident (`INC-NNN`) from this repository; v1.0 cites the
PhotoCraft study (`/home/user/photocraft-study/deliverables/`), because no Kiln data exists yet.

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
