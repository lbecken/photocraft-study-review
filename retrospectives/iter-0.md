---
iteration: 0
run: R-052
methodology_in: v1.0
methodology_proposed: v1.1
main_sha: 62c21aaf72940fbcd96b6298127fdb8e950ccbbe   # T-012 merged; later commits on main are STATE and this report
claims: {C1: insufficient, C2: supported, C3: insufficient, C4: supported,
         C5: insufficient, C6: refuted, C7: supported, C8: insufficient}
---

# Retrospective, iteration 0

Conventions. **M** = measured (a named command prints it). **I** = inferred (my reading of
records; the reasoning is given). Commands, run from the repository root:

- `S <section>` = `python3 -I retrospectives/scripts/iter0_metrics.py <section>`
  (sections: completeness, tokens, rounds, reviews, joins, lead, wall, budgets, concurrency,
  rework, missing)
- `G` = `bash retrospectives/scripts/iter0_git.sh`
- `R` = `cargo xtask metrics report`; `SC` = `cargo xtask scorecard`
- Findings and mutation counts not in front matter are quoted from the review body or
  `docs/decisions.md` (cited per number).

Sample: 5 feature tasks (T-001…T-005), all high tier; 1 low-tier publishing task (T-012);
1 bootstrap (T-000, exempt under D-005). Every by-tier comparison below has no medium tasks and
one low task. Treat all medians as descriptions of five tasks, not estimates of anything.

## Data

| Item | Value | Source |
|---|---|---|
| Records in `metrics/tasks.jsonl` | 57 run, 59 transition, 16 join, 3 revert_check | M, `S completeness` |
| Queue records | 6, all `attempt:1`, all `merged` | M, `metrics/queue.jsonl` |
| Run IDs issued (R-001…R-051) | 51; 50 have a run record | M, `S completeness` |
| Run without a run record | **R-024** (interrupted T-005 review, ran on a model other than the owner's, D-024) | M, `S completeness`; D-024 |
| Runs without a role log entry in a task file | R-001, R-002 (bootstrap, no task file; exempt), **R-015** (T-005 join triage: only `reviews/T-005-join-triage.md`), R-024 | M, grep of run IDs in `tasks/` (command in §Data notes) |
| Runs with unknown tokens recorded as 0 | 1 (**R-022**, T-003 test author, container restart D-022) | M, `S completeness` |
| Runs whose tokens are a director-computed delta of a resumed agent | 24 of 57 | M, `S completeness` (`note` contains "delta") |
| Runs split evenly over several task records | R-003 (5 tasks), R-049 (3), R-051 (2) | M, `S completeness` |
| Run records with both timestamps on a whole minute (hand-entered, I) | 17 of 57 | M, `S completeness` |
| Transitions on a :x0:00 / :x5:00 mark | 12 of 59 | M, `S completeness` |
| Transitions that leave a state before entering it | 2 (T-001 `in_progress` entered 23:10 left 23:00; T-002 `ready` entered 23:05 left 23:00) | M, `S completeness` |
| Runs that start before their task's `ready` transition | T-001 R-004/R-005, T-005 R-006 start 22:53, `ready` at 23:05; the planner R-003 "ends" 22:55 | M, `S lead`; run records in `metrics/tasks.jsonl` |
| Self-contradicting delta runs | R-023 is "the delta over R-018" yet runs 23:49:57–23:53:57 inside R-018 (23:46–23:57) | M, `S concurrency` + notes |
| Rework transitions that skip `self_verified` (§3 rework rule) | 7 (`in_progress → tests_joined`: T-002 ×2, T-004 ×2, T-005 ×3) | M, `S completeness` |
| Round reviews with finding counts in front matter | 11 of 11 | M, `S reviews` |
| Round reviews without mutant counts in front matter | 3 (T-005 r1–r3; counts only in body / D-026, D-033) | M, `S completeness` |
| Count mismatch | T-005 r2 commit `ec17870` says 4 spec gaps, front matter says 3 | M, `git log -1 ec17870` vs `reviews/T-005-r2.md` |
| Join records written by hand | 3 (T-005, D-015) | M, `note` fields, `S joins` |
| Join records with `acc_total ≠ acc_fail_base` | 4 (T-001 refusal; T-004 ×3 = the two-crate bug, D-025) | M, `S joins` |
| Joins with failures but `acc_defects: null` | 2 (T-004 01:18:27, 02:19:40) | M, `S joins` |
| Director's own token use, wall time, gate runs | **not recorded at all** | M (absence): no `role:"director"` run record |
| Environment incidents | 2 (D-022 restart, D-024 usage limit), in `docs/decisions.md` only; `incidents/` does not exist (no red main) | M, `ls incidents` |

Data notes. Log-entry check: `for r in R-0{01..51}; do grep -l -- "$r" tasks/*.md; done`.
Token figures for 24 resumed runs are differences the director computed between cumulative
counters; they cannot be re-derived from the repository. Wall times of R-026/R-027 include the
45-minute outage (D-024), and the T-005 review chain includes a container restart (D-022).

**Completeness verdict (M).** By the §13 definition ("every run has a run record and a log
entry; every state change has a transition record") iteration-0 metrics are **not complete**:
1 run without a record, 1 run without a log entry, 7 rework passes without the required
transitions, and timestamps that are partly reconstructed. Lead times and concurrency below rest
on those timestamps and are therefore low-confidence.

## Metrics (METHODOLOGY §13)

| Metric | Value | Command |
|---|---|---|
| Lead time ready→merged | high: median **0.59 h**, p75 2.19 h, max 3.58 h (n=5: 0.16, 0.55, 0.59, 2.19, 3.58); low: 0.01 h (n=1, T-012, director-performed). `ready` stamps are hand-entered and T-001's precedes nothing real (its runs started 12 min earlier). | M, `S lead`; `R` prints median 0.57 h over n=6 |
| First-pass queue success | **6/6** (100 %); queue gate 6.9–38.4 s | M, `R`; `metrics/queue.jsonl` |
| Gate attempts per task (Σ over runs) | median **7** (T-001 2, T-002 7, T-003 3, T-004 7, T-005 9); `R` prints mean 2.4 because it averages over all 12 task IDs incl. planner-only drafts | M, `S rounds`; `R` |
| Acceptance-caught defects | **tool: 0** (+1 unclassified). Records: `acc_defects` sums to 2 (T-005 23:53:49, T-004 01:57:08). My classification: **1** implementation defect (T-004, D-029); the T-005 one is a disagreement over an ambiguous director ruling (D-021), i.e. a spec gap. The tool reads only the first join record per task, which for T-005 is the invalid `no_acceptance_tests` record and for T-004 an unclassified test defect. | M, `R`, `S joins`; I for the reclassification |
| Review findings by class (high; no medium/low reviews) | 11 rounds: defect **7**, spec gap **36**, test gap **12**, nit **35** | M, `S reviews` |
| Escaped defects | **2**, both found by the evaluator in iteration 0: T-010 (origin T-001, high), T-011 (origin T-002, high) | M, task front matter |
| Rework ratio | iteration-0 task files: 2/8 = **25 %** (T-010, T-011); all task files: 7/13 = 54 % (follow-ups T-006…T-009, T-030 carry `iteration: 1`) | M, task front matter |
| Tokens per merged task | high: median **580,825**, p75 830,064, max 1,672,352 (n=5; T-003 understated by the unknown R-022); low: 159,025 (n=1, T-012 — this is the evaluator's run booked to T-012, not implementation cost) | M, `S tokens`; `R` prints the high mean 783,203 |
| Share of scorecard items verified | **22/63 = 34.9 %** (3 implemented-not-verified, 38 planned) | M, `SC` |
| Red-main minutes | **0**: no incident; GitHub Actions on `main` shows 57 runs, 57 `success` | M, GitHub Actions run list (read via the GitHub API tool at R-052) |
| Budget overruns | high: **2/5** merged (T-004 1.19×, T-005 2.39×); low: 1/1 (T-012 1.06×, an artefact of booking the evaluator to it) | M, `S budgets`; `R` prints 3/13 |

Further numbers used below:

| Number | Value | Command |
|---|---|---|
| Feature-task tokens by role | implementer 1,374,307 (35.1 %), test author 1,093,358 (27.9 %), reviewer 1,221,024 (31.2 %, incl. triage R-015 140,270), planner 227,325 (5.8 %); sum 3,916,014 | M, `S tokens` |
| All recorded run tokens | 4,900,715 (bootstrap 505,549; iteration-1 drafts 320,127; evaluator 159,025) | M, `S tokens` |
| Agent wall time per feature task (Σ `wall_s`, runs overlap) | T-001 0.44 h, T-002 0.64 h, T-003 0.36 h, T-004 3.10 h, T-005 2.91 h | M, `S wall` |
| Iteration span | first run → last merge 5.05 h; first feature run → last feature merge 4.03 h (both include the 45-min outage) | M, `S wall` |
| Mutants seeded / killed (all rounds) | T-001 14/14; T-002 31/22; T-003 29/27; T-004 58/52; T-005 56/44 (r1 17/20 D-026, r2 17/22 review body l.90, r3 10/14 by difference from D-033) | M, `S reviews`; D-026, D-033 |
| Round-1 mutants killed by acceptance tests alone | 73/84 = 87 % (T-001 14/14, T-002 13/16, T-003 24/29, T-004 22/25; T-005 not split) | M, `S reviews` |
| Later-round mutants killed by acceptance tests alone | 21/45 = 47 % (T-002 r2 6/12, T-004 r2 8/20, r3 7/13) | M, `S reviews` |
| Revert checks | T-003 23/23 fail, T-004 14/14 fail (as required); 1 `revert_conflict` | M, `S joins` |

## Where defects were found

Counting rule: a **defect** is behaviour that contradicts the spec or a stated guarantee; a
**test gap** is an AC or edge no test detects (here always shown by a surviving mutant); a
**spec gap** is a question the spec did not answer. Implementer and test-author spec gaps are
the `missing_context` items of their run records (semicolon-separated); reviewer spec gaps are
front-matter counts. Overlap is noted, not removed.

| Found by | Defects | Test gaps | Spec gaps | Token cost (feature tasks) | Evidence |
|---|---|---|---|---|---|
| Implementer's own tests | **1** recorded (T-001 window-count underflow, caught by its differential test). Implementer also *disclosed without fixing* T-002's 246–255-byte name failure (later reviewer D-2) and the temp-file clobber (later SG-1). Other own-test catches are not recorded: no field exists. | — | 26 items (R-004 2, R-007 3, R-016 1, R-020 3, R-026 4, R-035 2, R-006 5, R-032 6) | 1,374,307 | `tasks/T-001-text-core.md:325`; `tasks/T-002-documents-file-io.md` R-007 "not run"; `S missing` |
| Independent acceptance tests at join | **1** (T-004 r2 join: echo truncation did not reach `exec`, D-029) + 1 recorded but reclassified as ruling ambiguity (T-005, D-021) | — (they *are* tests) | 22 items (R-005 4, R-008 3, R-027 3, R-036 1, R-044 1, R-010 4, R-019 1, R-033 5); T-003 unknown (R-022) | 1,093,358 (+R-022 unknown) | `S joins`; D-021, D-029 |
| — of which test-side defects found at join | 6 failing tests that were the test's fault (T-004 miscount D-025; T-005 AC-5 fixture; T-005 r3 1 and r4 2 contradicted by rulings; T-004 r3 1 over-assumption) | | | follow-up test runs 23,391 (R-029, R-030, R-037, R-045, R-046) | join `note`s; task logs R-045 |
| Reviewer — own probing (driving the product, toy repos, exploits) | **6** (T-002 D-1 symlinked temp, D-2 long names; T-004 D-1 doc claims confinement, R2-D1 caps; T-005 F1 poisoned cache, F2 symbolic `reviewed_sha`) + 2 exploits classed spec gap (T-005 F3 `.cargo/config.toml`, F4 low-tier `Cargo.lock`) | 0 | (in the 36) | | `reviews/*` findings tables |
| Reviewer — inspection (code reading) | **1** (T-002 D-3 permissions applied after write) | 0 | (in the 36) | | `reviews/T-002-r1.md:108` |
| Reviewer — mutation spot-check | 0 | **12** (all 12 test gaps rest on a surviving mutant; incl. T-004 TG-1: path-escape acceptance tests that could not fail) | — | | `reviews/*` TG rows |
| Reviewer, all methods | **7** | **12** | **36** (≥6 duplicate test-author questions: T-001 SG-1..3 were found by all three roles, D-013; T-002 SG-5..7 = test-author Q1–Q3) | 1,221,024 (r1 801,638; later rounds 279,116; triage 140,270) | `S reviews` |
| Evaluator | **2** filed (T-010 items unobservable through any public interface; T-011 `--memory` always empty — wired but inert); 0 rejections; 2 unfiled observations (dirty document silently dropped at end of `control` input; `line_ending` has no effect on save) | — | 1 (io.crlf) | 159,025 for 25 items | `evaluations/iter-0.md` §Wired but inert, §Defects filed |
| Director | 0 product defects; 3 tooling defects noticed (join cannot see `xtask/tests` D-015; join `acc_total` wrong across crates D-025; ledger inputs that change a generated file cannot be committed D-034) | — | — | not recorded | D-015, D-025, D-034 |
| Gate at join (mechanical) | 0 product defects; 5 joins `gate_failed` on stale generated file or lint (D-012, `2b2f81b`) | — | — | — | `S joins` |
| Queue | 0 (6/6 first pass) | — | — | 113 s of gate time | `metrics/queue.jsonl` |

**C3 vs C5 against this table (I).** Review found 7 defects for 1.22 M tokens (≈174 k per
defect); independent tests found 1 for 1.09 M tokens. Six of the seven review defects came
from the reviewer *using* the system adversarially (symlinks, long names, poisoned caches,
symbolic shas); none came from reading the acceptance tests. All 12 test gaps came from
mutation. The defect evidence in this iteration therefore favours review depth (C5's
mechanism) over independent authorship (C3's), on n=5 high-tier tasks.

**What independent test authorship bought, plainly.**
- Measured: 1 implementation defect at join (T-004), in code implementing a director ruling
  (D-028 (6)) that was replaced one round later (D-031); 1 behaviour change on T-005 from an
  ambiguous ruling (D-021).
- Measured: strong evidence. Round-1 acceptance tests alone killed 87 % of seeded mutants;
  T-004/T-005 tests failed on base by assertion; revert checks failed 23/23 and 14/14. This is
  the opposite of PhotoCraft's vacuous tests and is what lets the scorecard derive status (C4).
- Measured: 22 spec questions raised in parallel with implementation; on T-005 the director
  answered them to both sides before the join (D-027), which avoided a triage cycle by the
  director's account.
- Not supported by the records: that a test author found anything a reviewer would not have
  (no counterfactual); that the independence rule held (unchecked, §16); that it saved
  implementer rework. Against it: 6 test-side join failures, and one set of tests that could not
  fail (T-004 TG-1), caught only by the reviewer's mutation.
- Cost: 27.9 % of feature-task tokens (≈ 1.09 M per caught implementation defect).

## Cost and convergence of review

| Task | Rounds | Tokens per round | Defects | Test gaps | Spec gaps | Post-r1 rework tokens (all roles) |
|---|---|---|---|---|---|---|
| T-001 | 1 | 127 k | 0 | 0 | 3 | 0 |
| T-002 | 3 | 138 k / 56 k / 9 k | 3 / 0 / 0 | 3 / 2 / 0 | 7 / 1 / 0 | 140,477 |
| T-003 | 1 | 161 k | 0 | 0 | 6 | 0 |
| T-004 | 3 | 179 k / 81 k / 22 k | 1 / 1 / 0 | 1 / 0 / 0 | 5 / 3 / 1 | 330,217 |
| T-005 | 3 (+ triage 140 k) | 196 k / 68 k / 43 k | 2 / 0 / 0 | 2 / 2 / 2 | 4 / 3 / 3 | 725,687 |

M: `S reviews`; post-r1 rework: `S rework`.
Round medians: r1 161 k, r2 68 k, r3 22 k tokens. Post-r1 rework = 1,196,381 tokens,
**30.5 %** of feature-task tokens.

**Where later-round findings came from (I, from the findings' "Where" columns).** Rounds 2–3
produced 18 non-nit findings (1 defect, 8 spec gaps, 6 test gaps across T-002/T-004/T-005; plus
r3 T-004 1 spec gap). **All 18 are in code or text written to implement round-1 rulings**; 0 are
new defects in the originally submitted code. Four trace to a ruling that was itself wrong:
T-004 R2-D1 (D-028 (6) → D-031), T-005 F13 (R4 made every version change unmergeable) and F15
(the D-027 exemption), and outside review the D-016 wording the implementer stopped on (D-017).

**Stopping rule the data suggests (I).** A round after round 1 is a check of the fixes, not a
fresh review. Stop at round 2 unless round 2 finds a **defect in the delta**; turn remaining
spec and test gaps into a follow-up task (D-030 and D-033 did this by ad-hoc ruling). Applied
to iteration 0 it would have removed T-005 r3 (43 k reviewer + ≈146 k implementer/test runs
R-041, R-042, R-046) and T-002 r3, and kept T-004 r3 only because r2 found a defect.

## The director as a failure source

| # | Error | Consequence | Caught mechanically? | Check now / proposed |
|---|---|---|---|---|
| 1 | Ran `cargo xtask join` with `CARGO_TARGET_DIR` pointing at the T-005 worktree (D-019) | Join measured main's binary; 15 false failures; triage run R-015 (140,270 tokens); T-005 back to `in_progress` | No; found by the tests failing and a reviewer triage | Now: wrong-checkout refusal (T-005, D-019 (f)). Proposed: none further |
| 2 | D-016 ruling unimplementable without reading the umask → D-017 | Implementer round R-016 (14,624) stopped; review round 3 | No; implementer stop condition 5 | Proposed change 2 (ruling check) |
| 3 | D-027 exemption of `queue.jsonl` → hole F15 | Fake `merged` line let a direct code commit through; one more round | No; reviewer r2 | Change 2 |
| 4 | D-026 R4 lock rule → no version change ever mergeable (F13) | Rework in round 5 | No; reviewer r2 | Change 2 |
| 5 | D-028 (6) echo-truncation ruling → D-031 | Defect R2-D1, 20× slower sessions, T-004 round 3 (≈88 k tokens R-043/044/045/047) | No; reviewer r2 (and partly the acceptance test, D-029) | Change 2 |
| 6 | Approval row written after the merge (D-032) | Evidence integrity only; queue merged T-004 without its row | No (bootstrap queue) | Now: hardened queue refuses a high task without its row (T-005) |
| 7 | Resumed a stopped review on a different model (R-024, D-024) | Run discarded, no run record; D-006 violated | No; noticed from a failure notice | Proposed change 11 (dispatch record states the model matched) |
| 8 | Guessed timestamps (D-032's failed command; 17 whole-minute runs; 2 impossible transition orders) | Lead time and concurrency unreliable; a shell command failed and the next line queued | No: `metrics add/transition` accept any `--at` | Change 3 |
| 9 | Accepted a self-contradicting spec (T-003: add commands, keep a test that pins the command list, `apps/` out of scope) | Blocked run R-020 (147,422 tokens, most of it useful work), D-020 | No; implementer stop condition 3 | Change 9 (planner checklist: snapshot tests a change will break) |
| 10 | Accepted a wrong worked example (T-004 AC-9, offset 5 in a one-byte file) | Both builders raised it; ruling D-025 | No; both builders | Change 9 (examples executed) |
| 11 | Did not stop T-004/T-005 at their budget (crossed at R-040 and R-018) and recorded no extension | §11 not followed; C6 refuted | No | Change 5 |
| 12 | Metrics bookkeeping: evaluator run booked to T-012 (false overrun); `acc_defects` left null on 2 joins; 7 rework passes without `self_verified`; T-005 joins hand-written | Wrong report lines (`R` overruns 3/13, acc-caught 0) | No | Changes 3, 14 |
| 13 | Wrote T-012's spec, accepted it and committed its work as `Role: implementer` (`d58c1b0`) | §2 "never approves a spec it wrote" not followed; provenance misattributed | No | Change 12 (a ledger-input publishing path that needs no task) |
| 14 | Concurrency: records show 4 builder runs live 02:18:30–02:20:40 (R-041…R-044); cap is 3 (I: timestamps partly hand-entered) | Possible contribution to the usage-limit stop | No | Change 13 |

Of 14 director errors, **0** were caught by a mechanical check at the time; 2 are now
mechanical (rows 1, 6, via T-005). Three were caught by a builder's stop condition (rows 2, 9,
and 10 by questions), four by reviewers (rows 3–5, partly 1).

## Spec quality

| Task | Missing-context items, first-round runs | After rulings | Later review rounds / join failures traced to an under-specified interface (I) |
|---|---|---|---|
| T-001 | 6 (2 + 4, same two gaps seen by both builders) | 0 | 0 rounds; 0 |
| T-002 | 6 | 1 (D-016) | 2 rounds: the save path's side effects, permissions and platform were unspecified (r1 SG-1, SG-2, SG-4) |
| T-003 | 3 (+ R-022 unknown) | 0 | 0 rounds; 1 blocked run (spec contradiction, D-020) |
| T-004 | 7 | 4 (no kind values, no structured reason, kinds layout) | 2 rounds (error kinds and caps unspecified, D-028/D-031); 2 join failures (wrong example; over-assumed kind distinctness) |
| T-005 | 9 | 12 (R-019 1, R-033 5, R-032 6) | 2 rounds (threat model and gate-path list unstated until D-026); 4 join failures (D-021 ambiguity; R3 contradicted old setup; 2 in round 4) |

M: `S missing`; I: attribution of rounds. Total 48 items, 31 in first
rounds, 17 created by the director's own rulings. The planner should be required to pin down:
(a) every error variant, its payload fields and the precedence when several apply (T-001,
T-002, T-004); (b) the layout of every file the task reads or writes (T-005 TOML); (c) the
order of checks (T-005); (d) path base, platform and file-type assumptions (T-002, T-004);
(e) the threat model for gate or security tasks (T-005, stated only at D-026); (f) existing
tests that pin a snapshot the task will change (T-003); (g) every worked example, executed
against a naive model (T-004 AC-9); (h) for each scorecard item, the public command the
evaluator will use (T-010).

## Tiering

M: 5/5 feature tasks high (task front matter). Each met a v1.0 high criterion (T-001 "L0
public API" and `Cargo.lock`; T-002 save path; T-003 command schema; T-004 two layers; T-005
`xtask/`), so "everything high" was **correct under the rule as written**. The rule did not
discriminate: `Cargo.lock` or "two layers" alone makes almost any feature high.

Two observations (I):
1. **High and medium differ only by the director approval row and a possible ADR** (§4, §7).
   Review depth is not tied to tier, so even a perfect tier split would not test C5.
2. Defects and later rounds concentrated where the task crosses the OS or trust boundary:
   T-002 (filesystem side effects), T-004 (paths, processes, a security claim in a public
   document), T-005 (gate). T-001 and T-003 (in-memory L0/L3) had 1 round and 0 defects.

A rule that would have discriminated: **high** = filesystem/OS/process boundary (persistence,
paths, spawning), gate/queue/CI paths, a security property stated in docs, a new external
dependency, or a change to the *shape* of a CHARTER §3 contract; **medium** = everything else
with scorecard items, including a lockfile change from workspace-internal edges only and new
commands registered under the existing schema. Under it T-001 and T-003 are medium, T-002,
T-004, T-005 high. The CHARTER §4 tier names and definitions change only by ADR.

## Tooling and environment

| Manual step / incident | Occurrences | Removed by |
|---|---|---|
| Regenerate `docs/scorecard.md` after a join | 4 (T-001…T-004) + 1 in T-012 | partly T-005 (conflict-only regeneration); fully T-009 (7) |
| Hand-written join records (join could not see `xtask/tests`) | 3 (T-005) | T-005 round 2 (D-015 (e)) |
| `acc_total` wrong when acceptance files sit in two crates | 3 records (T-004) | T-009 (6) |
| Approval row checked by eye | 5 | T-005 (mechanical) |
| Tier vs `touches`, `Task:`/`Role:` lines, implementer not editing `acceptance_t*` | every task | partly T-005 (gate paths, dependencies need high); rest unchecked (T-005 r1 list) |
| Untracked drafts block the `ledger` check | reported F17/F24 | T-009 (5) |
| Ledger input that changes a generated file (evaluation, scorecard item) | 2 (D-034; planner an hour earlier) | T-009 (7) for `docs/scorecard.md` |
| Hand-entered run/transition timestamps and token deltas | 57 runs, 59 transitions | nothing (change 3) |
| Stale binary from a shared target dir | 1 (D-019) | T-005 round 2 guard |
| Worktree seeding copying 1.6 GB per worktree; 9 GB of scratch builds; free disk 6 GB | D-009, D-022 | seeding: T-005; scratch cleanup and disk monitoring: nothing |
| `git stash` in a worktree (shared stash ref) | 1 (R-006) | nothing (rule in briefs only) |
| Container restart; usage-limit stop (45 min) | 1 + 1; 8–25 min of uncommitted work repeated | "commit early" and "relaunch fresh" in dispatch (D-024); not checked |

## Throughput

| Measure | Value | Basis |
|---|---|---|
| Tokens per merged feature task | median 580,825; range 356,965–1,672,352 | M, `S tokens` |
| Tokens per verified item, product tasks only (T-001…T-004: 2,243,662 / 18 items) | ≈ 125 k | M, `S tokens`; `evaluations/iter-0.md` confirmed_items |
| Tokens per verified item, all recorded runs (4,900,715 / 22) | ≈ 223 k | M |
| Merged tasks per hour of iteration span | 6 / 5.05 h ≈ 1.2 | M, `S wall` |
| Recorded tokens per hour of span | ≈ 0.97 M (incl. 45-min outage) | M |
| Director tokens | unknown (not recorded) | — |

Caveats: R-022 counted as 0; 24 deltas; the outage and the restart are inside the wall times;
director work is excluded from every token figure; all 5 tasks are headless L0–L4 work, while
the remaining items include syntax (tree-sitter), search, diagnostics, formatting and the egui
UI, for which there is no sample.

**Projection for the 38 planned items** (+3 implemented items covered by T-010/T-011):
- Low, ≈ 4.7 M tokens: product-task rate (125 k/item), one review round, no tooling tasks.
- Middle, ≈ 8.5 M: all-in rate of iteration 0 (223 k/item), i.e. the same mix of tooling,
  follow-ups, evaluation and planning.
- High, ≈ 17 M: T-004's rate (830 k / 4 items ≈ 208 k/item) times the iteration's all-in /
  product ratio (4.90 M / 2.24 M = 2.18).
- Plus the 7 drafted follow-ups (budgets 3.75 M; T-006…T-011, T-030), plus unmeasured director
  tokens. Wall time at 1.2 merged tasks/h: about 25–35 h of operation for 30–40 further tasks,
  with a usage-limit stop roughly every 5 h at the measured rate (one in iteration 0).

**Is the medium scope (40–70 tasks) realistic? (I)** In task count, yes: ≈ 4 items per feature
task gives 10–13 feature tasks; at the measured 1.4 follow-ups per feature task (7 from 5) that
is 24–31 more tasks plus the 7 drafted, **44–51 tasks in all**, inside 40–70. In cost, only at
the low-to-middle end and only if review converges: three T-005-like tasks (1.67 M each)
would consume the middle scenario's margin. The upper end of 70 tasks is not supported by
anything measured.

## Claims

| Claim | Verdict | Numbers that decide it | What would change the verdict |
|---|---|---|---|
| C1 contracts and checks first | **insufficient** | Gate, queue, scorecard and validated schema existed before feature work (T-000; `cmd.schema` verified); 0 ADRs, 0 contract changes, 0 escaped defects traced to a contract. But there was no fan-out to compare against, and the checks built first were themselves breakable until T-005 (4 exploits, D-026). | Iteration 1 fan-out: escaped defects or rework traced to contract changes ≈ 0 with ≥3 tasks in flight → supported; contract-change rework → refuted. |
| C2 serial queue keeps main green at agent speed | **supported** (weak) | 6/6 first-pass; 57/57 Actions runs green; 0 red-main minutes; queue adds 7–38 s. Contention was low: no queue failure or conflict to observe. | Any red main, or queue wait dominating lead time, in iteration 1. |
| C3 different-author tests catch what implementer tests miss | **insufficient** | Existence shown once (T-004, D-029; tool reports 0); reviewer caught 7 at similar cost; 1 director-classified case in n=5. | ≥3 acceptance-caught implementation defects across ≥10 medium/high tasks, classified by someone other than the director → supported; 0 → refuted. |
| C4 derived status + evaluator prevent over-claiming | **supported** | 0 hand edits possible (`scorecard --check` in gate); evaluator withheld 3 of 25 `implemented` items (12 %) that tests claimed but no public interface showed; reviewer caught one over-claiming document (T-004 D-1). | An evaluator-confirmed item later found inert or wrong. |
| C5 risk tiers put review effort where defects are expensive | **insufficient** | 5/5 high, so no contrast; high and medium differ only by an approval row. Defects did cluster in boundary-crossing tasks (7/7 in T-002/T-004/T-005). | A tier split with different review depth and ≥5 tasks per tier. |
| C6 budgets that gate stay honest | **refuted** (as operated) | Token budgets did not gate: T-004 1.19×, T-005 2.39×, no stop, no extension record. 0 latency budgets enforced (budgets step empty). The overruns *were* recorded and reported honestly. | Overruns stopped or extended on record, mechanically, in iteration 1. |
| C7 repo state makes work resumable and auditable | **supported** | Work survived a restart and a usage-limit stop (D-022, D-024): only uncommitted work (8–25 min) was lost; 50/51 runs have records, 49/51 log entries; this retrospective was produced from the repo alone. Against: hand-entered timestamps, unrecorded director work. | A resume that fails for lack of a record, or audits that cannot reproduce a number. |
| C8 constructed oracles define correct | **insufficient** | Differential and round-trip tests exist (T-001, T-002) and killed every seeded mutant aimed at them; 0 escaped correctness defects on those items; the oracle-vs-not escaped-defect comparison (director classification) was not done. | That comparison over iteration 1 with ≥10 items each side. |

Confounders for every verdict: n=5, one tier, a director who also classifies `acc_defects`, a
tooling task (T-005) that is 43 % of feature tokens, and reconstructed timestamps.

## Where the methodology was wrong

**Rules not followed (M unless marked).**
- §11 budget stop: T-004, T-005.
- §3 rework passes through `self_verified`: 7 transitions skip it (T-002, T-004, T-005).
- §7 "any test gap ⇒ request-changes": T-005 r3 approved with 2 test gaps (by D-030's ruling).
- §7/§4 approval row before the queue: T-004 (D-032).
- §2 director never approves a spec it wrote: T-012.
- §12 every run has a record and a log entry: R-024, R-015.
- §5.1 dispatch only `ready` tasks: T-001, T-002, T-005 runs start before `ready` (I: may be
  timestamp error).
- §5.4 cap 3: 4 live runs at 02:18:30–02:20:40 (I: timestamps).
- §12 `Task:`/`Role:` lines: 67 of 164 commits on main lack them, all director commits; only 6
  director commits carry `Role: director` (`G`). The rule is ambiguous about director commits.
- §12 no session identifiers in docs (unchecked): 4 review files contain a session-scoped
  scratch path (`G`, last section). Commit trailers name two different models across builder
  commits (25 builder commits without trailer, 25 with one variant, 42 with another; `G`), so
  D-006 compliance cannot be checked from the repository.
- D-006 owner's model for builders: R-024 (D-024).
- §13 `found_by` vocabulary: T-012 has `found_by: evaluator` but is not a defect.

**Rules that cost effort and caught nothing.**
- Revert check: 2 performed, both failed as required, caught nothing; the stub/logic commit
  split it needs was not asked of implementers (D-011). Mutation spot-checks found all 12 test
  gaps.
- Director approval row: it followed the reviewer's verdict every time, added no finding, and
  was once forgotten without effect (D-032). It is cheap and now mechanical; its value is audit.
- Queue preconditions caught nothing in iteration 0 (all evidence was present), but they are
  cheap (seconds) and are the defence D-032 shows is needed.

**Gaps no rule addressed.** Director operator errors and wrong rulings (14 above); a
stopping rule for review; environment (disk, restarts, usage limits, model on resume);
acceptance tests that cannot fail (no self-check for test authors); §6 allows ACs through a
public *Rust API* while §9 lets the evaluator use only CLI/control, which left 3 items
unverifiable (T-010, T-011); ledger inputs that change generated files (D-034); regression
guards had no category (D-010).

**Unchecked rules that drifted.** Test-author/implementer independence (no evidence either
way); no session identifiers (drifted, above); `Task:`/`Role:` lines on director commits.

**Open questions now answerable.**
- O-1 integrator: director as integrator works but is the main failure source (14 errors);
  answer: keep the role, make the join and records mechanical, do not add an agent.
- O-2: answered yes (`worktree new T-NNN-tests` exists; `task/T-00N-tests` branches).
- O-3: unanswered (no queue failure).
- O-4: partly answered: T-005 made `touches`, state, join, review, approval, gate paths and
  dependencies mechanical; implementer-edits-acceptance, commit lines and tier-vs-`touches`
  beyond gate paths remain director checks.
- O-5: answered: compile-only fail-on-base occurred on T-001/T-002; mutation spot-check
  (D-011) was the effective substitute.
- O-6: answered: `evaluations/iter-N.md` `confirmed_items`, read by `scorecard` (22 verified).
- O-7: answered: agents cannot see tokens and director enforcement after each run did not
  happen; in-run gating by tokens is not workable as written.
- O-8: partly: names match, but `acc_defects` is read from the first join only and
  `acc_total` is wrong across crates.

## Proposed changes (methodology v1.1)

Ordered by expected benefit. "Removes" says which rule the change lets the director delete.

| # | Change (file, section): old → new | Motivating metric or incident | Expected effect | Check | Removes |
|---|---|---|---|---|---|
| 1 | METHODOLOGY §7: no stopping rule → "Round 2 reviews only the delta and is final unless it finds a **defect in the delta**; remaining spec/test gaps become a follow-up task with `origin_task`. Round 3 only for a delta defect." | Post-r1 rework 1.196 M tokens (30.5 %); 18 later-round findings, 0 new defects in submitted code; D-030/D-033 ad-hoc "last round" | −20–30 % tokens on high tasks | rounds per task ≤ 2 except recorded delta defects (`S reviews`) | the ad-hoc "last round" rulings; "any test gap ⇒ request-changes" applies to round 1 only |
| 2 | METHODOLOGY §3/§7 + reviewer brief: rulings are free text in `decisions.md` → "A ruling that changes behaviour is written as a diff to the task's Interface/ACs, committed before re-dispatch, and checked by the reviewer (or planner) for implementability before builders get it; the ruling cites that commit." | 3 rulings replaced (D-017, D-030, D-031); 4 later findings trace to wrong rulings; 17 missing-context items created by rulings | fewer ruling-caused rounds | count of replaced rulings per iteration (target 0); ruling rows cite a task-file commit | — |
| 3 | METHODOLOGY §13 + `xtask metrics`: hand-entered times → "`metrics add/transition` stamp `now` by default, refuse an `--at` earlier than the task's last record, refuse transitions not in §3 (e.g. `in_progress→tests_joined`), refuse `--tokens 0` without `--tokens-unknown`, and record director runs (`role: director`) per session" | 17 whole-minute runs, 2 impossible orders, 7 skipped transitions, R-022, D-032's failed shell line, no director tokens | trustworthy lead time; director cost visible | `S completeness` problems = 0 | director check "transition records" (becomes mechanical) |
| 4 | CHARTER §4 (by ADR) + METHODOLOGY §4/§7: tier by `Cargo.lock`/layers → the boundary rule in §Tiering above; **and tie review depth to tier**: medium = AC check + ≥10-mutant spot-check; high = + adversarial probing of the boundary + approval row | 5/5 high; 7/7 defects in boundary tasks; high vs medium differ only by a row | a testable C5; ≈ 2 of 5 tasks medium | by-tier findings and tokens at retrospective 1 | "`Cargo.lock` ⇒ high", "≥2 layers ⇒ high" |
| 5 | METHODOLOGY §11 + `xtask queue`: director stops runs over budget → "`metrics add` prints the task's cumulative tokens against budget; `queue` refuses a task over `budget_tokens` without an `Extend T-NNN` row" ; budgets per §Budget recalibration | T-004 1.19×, T-005 2.39×, no stop or record (C6 refuted) | C6 testable; overruns become decisions | `R` overruns each have an Extend row | stop condition 2 for agents (they cannot see tokens, O-7) |
| 6 | METHODOLOGY §6 + planner brief: ACs may use a public Rust API → "every scorecard item has ≥1 acceptance test through CLI or control protocol; Rust-API tests are additional" | 3 of 25 items unverifiable (T-010, T-011) | evaluator `not_evaluated` = 0 | `evaluations/iter-1.md` `not_evaluated` | — |
| 7 | Test-author brief: no self-check → "before handing over, run the tests against a naive stub plus ≥5 seeded bugs and list kills in the log; a negative test must show the forbidden target exists and is unchanged" | T-004 TG-1 tests could not fail; T-001's author did this voluntarily (14/14 later) | acceptance kill rate ≥ 80 % in every round (now 47 % in later rounds) | review front matter `killed_by_acceptance` | — |
| 8 | METHODOLOGY §6/§7: revert check for compile-only fail-on-base → "reviewer mutation spot-check on every medium/high review; `mutants: {introduced, killed, killed_by_acceptance}` required in front matter" | revert checks 2/2 caught nothing; mutation found 12/12 test gaps; T-005 reviews lack counts | one evidence rule instead of two | review front matter lint in `scorecard --check` or a gate step | revert check; the stub/logic commit split |
| 9 | Planner brief + `templates/task.md`: add a "Pinned" section listing (a)–(h) from §Spec quality, each answered or marked "not applicable" | 31 first-round missing-context items; D-020; D-025 example; T-005 threat model only at D-026 | first-round missing-context ≤ 2 per task | `S missing` | — |
| 10 | METHODOLOGY §3: questions arrive anytime → "both builders log questions within their first gate attempt; the director answers both at once before the join" (D-027 pattern) | T-001's 3 gaps found by 3 roles separately; D-027 avoided a triage cycle | fewer join failures from ambiguity (6 test-side now) | join failures traced to ambiguity per task | — |
| 11 | `docs/development.md` director runbook + dispatch record: "dispatch writes a run record at start (run, role, task, model-matches-owner yes/no); free disk ≥ 15 GB checked before dispatch; scratch builds removed at run end; a stopped agent is relaunched, never resumed" | D-022 (6 GB free), D-024 (model on resume), R-024 without record | no unrecorded runs | `S completeness` | — |
| 12 | METHODOLOGY §8/§12: evaluation and other ledger inputs that change generated files need a director-written low task (T-012) → "the queue regenerates `docs/scorecard.md` on every run (T-009 (7)); the director commits ledger inputs directly" | 5 manual regenerations; D-034; T-012's self-approved spec | no director-as-implementer tasks | count of tasks with `owner_role: director` and no builder run | D-012 manual regeneration |
| 13 | `xtask worktree new`: dispatch checks by eye → "refuses unless the task is `ready` and in-flight tasks < cap" | 3 tasks run before `ready`; 4 live runs (I) | §5.1/§5.4 mechanical | refusal tests | two director checks in §16 |
| 14 | METHODOLOGY §13 + `xtask metrics`: (a) acceptance-caught defects over **every** join with `impl_tests_green`, `acc_defects` required when `acc_fail_first_join > 0`; (b) planner/evaluator runs booked to `iter-N`, not a task; (c) new metric "known defects merged by decision" (T-008, T-009, T-030 origins); (d) rework ratio by iteration of discovery | `R` says 0 caught vs 2 records; T-012 false overrun; deferred defects invisible | report matches records | `R` vs `S joins` | — |
| 15 | METHODOLOGY §12: clarify that `Task:`/`Role:` lines bind builder commits; director commits carry `Role: director` only when they record a role's output; add the session-path grep to the gate (`no-skip`-style step) | 67/164 commits without lines; 4 files with session paths | rule matches practice; drift checked | `G` | the unchecked label on §12's identifier rule |

**Considered and rejected.**
- Drop independent test authors for medium tier: one caught defect is not enough to keep them
  on defect evidence, but their tests are what the scorecard derives status from (C4) and what
  mutation measured at 87 %; rejected until retrospective 1 has medium-tier data.
- A separate integrator agent (O-1): the integrator failures were bookkeeping slips; mechanising
  them (changes 3, 11, 13) is cheaper than another agent.
- Longer or second reviewers on high tasks: later rounds already showed diminishing returns.
- Per-run token allotments enforced in-run: agents cannot observe tokens (O-7).
- Raising the concurrency cap: the usage limit, not the cap, stopped work (D-024).

## Budget recalibration

Rule: budget = p75 of the recorded total tokens of merged tasks of that tier, rounded up to
50 k; with fewer than 3 tasks in a tier, keep the old value or use the re-tiered tasks named.
- **High: 850 k** (p75 of 476 k, 581 k, 357 k, 830 k, 1,672 k = 830,064; `S tokens`). With
  change 1 in force, T-005's 1.67 M would be ≈ 1.2–1.3 M (I), still an overrun to decide.
- **Medium: 500 k, provisional**: no medium task exists; T-001 (476 k) and T-003 (357 k + an
  unknown R-022) are the tasks change 4 would re-tier, so the max rounded up.
- **Low: 150 k, unchanged**: the one low record (T-012, 159 k) is the evaluator's run, not the
  task's cost.

## Uncertainty

- n=5 feature tasks, one tier, one tooling task carrying 43 % of feature tokens.
- `acc_defects` is director-classified; I reclassified the T-005 one (D-021) as a ruling
  ambiguity, so acceptance-caught defects are 1 (mine), 2 (records) or 0 (tool).
- Reviewer finding classes are the reviewer's own; two demonstrated exploits on T-005 are
  "spec gaps" by that classification.
- Token figures: R-022 unknown (recorded 0); 24 deltas computed by the director; even splits
  of three multi-task runs; director tokens absent.
- Timestamps: 17 runs and 12 transitions look hand-entered; lead time, concurrency and
  "before ready" findings are low-confidence.
- Wall times include a 45-minute outage and a container restart.
- The model actually used by each builder run cannot be determined from the repository.
- The projection extrapolates headless L0–L4 work to UI, syntax and tool-integration items for
  which there is no sample.
- I could not determine: what the implementers' own tests caught beyond the one logged case;
  whether test authors read implementation branches; the director's cost; whether the four
  concurrent runs were real.
