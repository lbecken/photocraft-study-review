---
iteration: 1
run: R-153
methodology_in: v1.1
methodology_proposed: v1.2
main_sha: b3d9dd29787f0ef0e81881341917efb32ac3021d   # D-089, iteration closed; main later gained one handoff-only commit (b2ee623)
claims: {C1: insufficient, C2: supported, C3: insufficient, C4: supported,
         C5: insufficient, C6: refuted, C7: supported, C8: insufficient}
         # C4 and C8 revised after the iteration-1 evaluation (R-152, D-090); see "Evaluation of iteration 1"
---

# Retrospective, iteration 1

## How to read this report

Labels. **VERIFIED**: I ran the command named beside the number and it printed it.
**INFERRED**: my reasoning from verified facts; the reasoning is given. **PROPOSED**: a change
for methodology v1.2, for the director to decide.

Commands, run from the repository root (python3 standard library only, read-only):
- `S <section>` = `python3 -I retrospectives/scripts/iter1_metrics.py <section>`
  (sections: completeness, provenance, tokens, tiers, lead, queue, gates, joins, reviews, arms,
  rework, specchecks, rulings, decisions, concurrency, idle, scorecard, projection, budgets)
- `G` = `bash retrospectives/scripts/iter1_git.sh` (git history from `e1f1a2b`, the D-035 commit,
  to `b3d9dd2`, the D-089 close commit)
- `D` = `python3 -I retrospectives/scripts/iter1_director.py` (the director-error table below as
  data, with its tallies)
- `R` = `cargo xtask metrics report` (exits 1 by design; output quoted where used)

Definitions used throughout.
- **Iteration-1 merged tasks**: the twelve tasks with `iteration: 1` in front matter and a
  `merged` queue record: T-006, T-008, T-010, T-011, T-013, T-014, T-015, T-016, T-017, T-018,
  T-032, T-035. Decision D-089 (and the brief for this run) lists thirteen, adding T-012. The
  record contradicts that: `tasks/T-012-publish-iter0-evaluation.md` has `iteration: 0`, it
  merged at 02:56:07Z (`metrics/queue.jsonl`), before methodology v1.1 was adopted (D-035 commit
  `e1f1a2b` at 03:11:05Z), and retrospective 0 counts it as iteration-0 work. I report T-012
  separately and leave it out of every iteration-1 figure. VERIFIED (`S tokens`, `G`).
- **Hand-added runs**: R-067 (spec check for T-016 and T-017, 294,896 tokens) and R-069 (spec
  check for T-032 and re-check of T-015, 59,214 tokens) were never booked (D-055; handoff
  addendum 12:30Z). Where a figure says "+hand" I add them, R-067 split evenly between T-016 and
  T-017 and R-069 booked wholly to T-032; the record does not say how either run divides.
- **Builder-role runs**: runs with role implementer, test-author or reviewer (spec checks are
  booked as reviewer runs, D-055).
- **Wall time** of a task: first implementer or test-author run start to the queue's `merged`
  record. **Lead time**: METHODOLOGY §13 (`→ ready` transition to `merged` queue record).

Sample: 6 high, 3 medium, 3 low merged tasks. Every by-tier or by-arm figure describes a handful
of tasks, not a population.

## Data

| Item | Value | Command |
|---|---|---|
| Records in `metrics/tasks.jsonl` (all iterations) | 147 run, 183 transition, 44 join, 8 revert_check at the close (`b3d9dd2`); 0 `join_defects`; 19 queue records. After the close the file gained R-152 (the evaluator, booked to `iter-1`) and three T-040 transitions, so on today's main `S completeness` prints 148 run and 186 transition records; every build figure in this report excludes the evaluator and analyst runs (`S evaluation` lists them) | VERIFIED `S completeness`, `S joins`, `S evaluation` |
| Booked to iteration 1 | 94 run records (84 distinct run ids), 124 transitions; 28 join records and 5 revert checks after 03:00Z | VERIFIED `S completeness` |
| Run ids R-049…R-151 without a run record | 17: R-054, R-057, R-060, R-064, R-065, R-066, R-067, R-068, R-069, R-074, R-075, R-076, R-077, R-078, R-079, R-080, R-086 | VERIFIED `S completeness` |
| — of those, segments folded into another record | R-054 (into R-053), R-057 (R-055), R-060/R-066/R-074/R-077/R-079 (R-056), R-086 (R-085); R-064, R-065, R-068, R-076 have log entries in T-013/T-015 and no record (INFERRED: folded into R-070/R-071 "rounds 2–4" and R-072/R-073; the notes do not name them) | VERIFIED run notes (`S provenance`); `git grep R-064` etc. |
| — never booked | R-067, R-069 (354,110 tokens, counted by hand here) | D-055 |
| — lost with no figure at all | R-075 (T-013 review), R-078 (T-015 review), R-080 (T-016 test author), cut off by the container restart of D-045; tokens unknown | VERIFIED `reviews/T-013-r1.md:17`, `reviews/T-015-r1.md:17`, handoff line 74 |
| Iteration-1 run without any log entry | R-087 (T-015 rustfmt fix) | VERIFIED `S completeness` |
| `role: director` run records | **0** (all iterations), although §2.1 requires one per session and T-013 made it possible | VERIFIED `S completeness` |
| Runs with outcome `blocked` | 7: R-089, R-090, R-083 (usage limit at about 08:13Z; no decision row, handoff line 103), R-129, R-130, R-131 (usage limit at about 14:20Z, D-073), **R-122** (a stop condition: red gate in a merged test outside `touches`, D-069). The brief's statement that the blocked runs were interrupted by a usage limit (D-073) is true of 3, not 7. | VERIFIED `S completeness`; run notes |
| Runs split evenly over several task records | R-049 (3 tasks), R-051 (2), R-055 (8) | VERIFIED `S completeness` |
| Token figures that are differences of running totals | 9: R-063, R-070, R-071, R-112, R-129, R-130, R-131, R-149, R-151 | VERIFIED `S provenance` |
| Token figures that are a "last context size" from a transcript | 5: R-083, R-089, R-129, R-130, R-131 (the harness gave no figure for a run that ended on a usage limit) | VERIFIED `S provenance` |
| Start times entered by the director | 16 runs (R-098 … R-121); 24 later runs take times from the agent transcript | VERIFIED `S provenance` |
| Placeholder figures in records | R-056 `tool_calls: 1` (note says placeholder, still in the record); R-082 committed with `tokens: 200000` at 07:31:55Z and replaced by 243,114 at 07:32:28Z | VERIFIED `S provenance`; `G` ("commits that rewrite an earlier run record"); `git log -S` |
| Transitions that skip `self_verified` | 5 (T-015, T-013, T-016, T-032 ×2), **all before T-013 merged at 12:14:33Z; 0 after** | VERIFIED `S completeness` |
| Builder runs started before their task's `ready` | 4: R-058/R-059 (T-013, hand-entered times), R-072/R-073 (T-015, 06:26Z vs `ready` 06:34:20Z) | VERIFIED `S completeness` |
| Review files with findings and mutants in front matter | 20 of 20 | VERIFIED `S completeness` |
| Joins with failures but `acc_defects` null | 2 (T-013 06:27:09Z, 4 failures; 06:30:13Z, 2 failures); `acc_defects` is null on all 28 iteration-1 joins; `metrics join-defects` was never used | VERIFIED `S joins`; `R` ("unclassified join" lines) |
| Queue precondition refusals | 2 recorded only in decisions (T-015 D-050, T-032 D-058); none in `metrics/queue.jsonl` | VERIFIED `S queue`; D-050, D-058 |
| Usage-limit stops | 3: 04:15–06:10Z (D-038), about 08:13–11:10Z (handoff only), 14:20–16:10Z (D-073) | decisions; handoff lines 103, 106 |
| Container restarts | 3: about 07:05Z (D-045), 11:11Z (handoff line 103), 16:11:36Z (D-073) | decisions; handoff |
| `incidents/` | does not exist; no red-main incident recorded | VERIFIED `ls incidents` |

**What the token figures of resumed agents mean (INFERRED).** For a resumed agent the booked
figure is the harness's running total at the end minus the total at the previous booking. Two
observations show that this running total is not the number of tokens processed: a resumed
reviewer with a 187,576-token context was booked at 24,820 for round 2 (R-149) and 7,890 for
round 3 (R-151); and for usage-limit stops the director substituted "the last context size".
A model call re-reads its whole context, so if the running total were consumption, every resumed
round would cost at least the context it inherits. The differences therefore approximate context
growth, not cost. Consequences: (a) the cheap round-2 and round-3 reviews by a resumed reviewer
(16k–25k) and the expensive ones by a fresh reviewer (134k–210k, `S reviews`) are not comparable;
(b) every task total that contains a delta (T-013, T-017, T-018, T-008) is a lower bound by an
unknown amount; (c) the five "last context size" figures are neither consumption nor a
difference and should be treated as order-of-magnitude.

**Completeness verdict (VERIFIED from the rows above).** By §13 ("every run has a record and a
log entry, every state change a transition") iteration-1 metrics are **not complete**: 2 runs
unbooked, 3 runs lost without a figure, 4 log-only run ids, 1 run without a log entry, 0 director
runs, 2 unclassified failing joins. They are much better than iteration 0's on the points T-013
made mechanical: 0 skipped transitions and 0 whole-minute pairs after T-013 merged (1 before:
R-059).

## Metrics (METHODOLOGY §13)

| Metric | Value | Command |
|---|---|---|
| Lead time `ready`→merged | high: median **4.80 h**, p75 7.53, max 9.80 (n=6); medium: 1.00, 2.79, 4.58 (n=3); low: 0.12, 0.20, 0.27 (n=3). `R` prints median 1.28 h, max 9.80 over n=18 (it mixes iterations). | VERIFIED `S lead`, `R` |
| Wall time, first builder dispatch → merged | high median **4.24 h** (1.21–8.38); medium 0.99 h (0.89–1.61); low 0.18 h. T-013's 8.38 h and T-016's 5.2 h contain the 04:15–06:10 and 08:13–11:10 stops. | VERIFIED `S lead` |
| First-pass queue success | **11/12** by record (T-018 attempt 1: rebase conflict); **9/12** if the two unrecorded precondition refusals count. `R` prints 17/18 over both iterations. | VERIFIED `S queue`, `R`; D-050, D-058 |
| Queue gate time | median **199 s** per merged run (36–237 s); iteration 0 was 7–38 s | VERIFIED `S queue` |
| Gate attempts per task | median **3** (0–7, sum 41, n=12); `R` prints mean 3.2 over all tasks | VERIFIED `S gates`, `R` |
| Acceptance-caught defects | **records: 0** (all 28 iteration-1 joins have `acc_defects: null`); `R` prints 2, both iteration-0 joins. Run note of R-070: "director classification acc_defects=1" for T-013's second join (an implementation that drifted from a pinned output line); never written to a record. | VERIFIED `S joins`, `R`, `metrics/tasks.jsonl` R-070 note |
| Review findings by class and tier | high, 13 reviews: defect 11, spec gap 13, test gap 13, nit 14. medium, 7 reviews: defect 3, spec gap 6, test gap 3, nit 6 | VERIFIED `S reviews` |
| Escaped defects | **0 behaviour defects; 4 documentation findings** escaped every in-iteration check and were found by the evaluator (NEW-1…NEW-4, filed as T-037, T-038, T-039; D-090). By origin (my attribution, see "Evaluation of iteration 1"): T-008 (high, independent) 1, T-032 (high, independent) 2, T-011 (medium, implementer-written) 1, the last caused by a director rule (D-037). Escapes from iteration 0 merged now: T-010 (origin T-001) and T-011 (origin T-002) | `evaluations/iter-1.md`; D-090; `S evaluation` |
| Rework ratio | 12 of 30 iteration-1 task files carry `origin_task` (40 %); 7 were filed 02:37–02:53Z from iteration-0 reviews and evaluation; 5 were first committed during iteration 1 (T-032, T-033, T-034, T-035, T-036), 3 of them from iteration-1 tasks (T-034←T-013, T-035←T-032, T-036←T-008) | VERIFIED `G` (last section) |
| Review rounds per task | T-016 1; T-010, T-011, T-013, T-015, T-032 2; T-008, T-017, T-018 3 | VERIFIED `S reviews` |
| Acceptance kill rate | round 1: 128/158 = **81 %**; round 2: 50/67 = **75 %** (iteration 0: 87 % and 47 %); high 117/156, medium 61/69 over all rounds | VERIFIED `S reviews` |
| Known defects merged by decision | **1** defect: T-008's unreadable-root replacement (D-081 v11, D-083), owed to T-036. Plus 2 documented limits not called defects: T-016 parse memory (D-056 → T-020), T-017 result memory and large-pattern search time (D-079 → T-021); T-013 follow-ups, none a defect (D-053 → T-034). `R` prints 9 because it counts every task with an `origin_task`, which is the rework ratio, not this metric. | decisions; `R` |
| Tokens per merged task by tier | high: median **930,777** booked / 1,004,501 +hand, p75 1,442,812, max 1,790,657 (n=6); medium: median **520,658**, p75 684,480, max 848,301 (n=3); low: median 255,816, max 261,060 (n=3; two of them planner bookings, see Budgets) | VERIFIED `S tiers` |
| Tokens per merged task with overhead | (10,631,091 booked + 354,110 hand) / 12 = **915,433**; `R` prints 863,803 (no hand runs) | VERIFIED `S tokens`, `R` |
| Tokens per merged task by arm (medium) | independent: 520,658 and 848,301; implementer-written: 431,922. `R`'s arm lines (independent 908,069, n=13) mix high and low tasks and both iterations. | VERIFIED `S arms`, `R` |
| Director cost | **not recorded** (0 director runs) | VERIFIED `S completeness` |
| Share of scorecard items verified | **22/64 = 34.4 %** on main at the close (6 implemented, 36 planned, 0 failing); the evaluation confirms all 28, so once its publication (T-040) merges: **28/64 = 43.8 %** | VERIFIED `S scorecard`; `evaluations/iter-1.md` front matter (`S evaluation`: 28 confirmed, 0 rejected, 0 not evaluated) |
| Red-main minutes | **0** recorded (no incident). GitHub Actions: the handoff says every run from 174 to 185 concluded with success when checked; not re-checked here | handoff line 18; `ls incidents` |
| Budget overruns | booked: 7 of 12 (high 3/6: T-008 1.82×, T-013 2.11×, T-015 1.32×; medium 2/3: T-010 1.04×, T-018 1.70×; low 2/3: T-006 1.71×, T-014 1.74×). +hand: 8 of 12 (T-017 884,915 > 850,000). Extend rows: 5 (T-015, T-013, T-010, T-008, T-018); without one: T-006, T-014, and T-017 once its spec check is counted. | VERIFIED `S tokens`, `R` |

Further numbers:

| Number | Value | Command |
|---|---|---|
| Iteration-1 tokens | 10,631,091 booked + 354,110 hand = **10,985,201**; plus 3 lost runs and the director, unknown | VERIFIED `S tokens` |
| — by role | planner 2,347,030; implementer 2,745,148; test author 2,565,831; reviewer 2,973,082 (of which spec checks 558,527 booked + 354,110 hand) | VERIFIED `S tokens` |
| — by where booked | merged tasks 8,911,082; `iter-1` planner overhead 1,454,555; planner shares booked to draft tasks 265,454 | VERIFIED `S tokens` |
| Post-round-1 rework | **2,194,642** tokens = 24.6 % of merged-task tokens (iteration 0: 30.5 %); T-013 790,202 (44 % of the task), T-008 543,785 (35 %), T-015 287,842, T-017 251,948, T-018 165,616 | VERIFIED `S rework` |
| Review tokens by round | r1 1,720,532 (9 reviews); r2 641,505 (8) + R-106 addendum 14,844; r3 29,628 (3) | VERIFIED `S reviews` |
| Mutants seeded | r1 172 (14 equivalent, 148 killed, 10 survived); r2 70 (3, 64, 3) | VERIFIED `S reviews` |

## The §6 medium-tier arms

Assignment, VERIFIED from the task files: T-010 "even task number → independent test author"
(`tasks/T-010-expose-lines-and-positions.md:35`), T-018 "Arm: even (independent test author)"
(`tasks/T-018-selection-anchors-batch-eol.md:39`), T-011 "odd task number → the implementer
writes the acceptance tests" (`tasks/T-011-memory-vfs-observable.md:31`). The director's belief
is correct. T-011's tests came first: commit `76c7c83` "acceptance tests and guards … (written
before the code)" at 12:57:34Z, and the R-114 note says all 9 failed on the bare branch by
assertion; the four rework test commits precede the rework implementation commit (R-121, R-123
notes). VERIFIED (`git log -- 'apps/kiln-cli/tests/acceptance_t011*'`).

| Measure | T-010 (independent) | T-018 (independent) | T-011 (implementer-first) | Command |
|---|---|---|---|---|
| Tokens (booked) | 520,658 | 848,301 | 431,922 | `S arms` |
| — test writing | test author 185,830 | test author 306,170 | inside implementer 227,541 | `S arms` |
| — review | 149,402 (2 rounds) | 220,286 (3 rounds) | 165,470 (2 rounds) | `S arms` |
| Defects found at join | 0 (2 joins, 0 first-join failures) | 0 (3 joins, 0) | 0 (3 joins, 0) | `S joins` |
| Review defects | 0 | 3 (all documentation sentences) | 0 | `S reviews` |
| Review spec gaps / test gaps / nits | 0 / 1 / 2 | 3 / 1 / 1 | 3 / 1 / 3 | `S reviews` |
| Round-1 acceptance kill rate | 11/12 = 92 % | 18/20 = 90 % | 15/18 = 83 % | `S arms` |
| Mutants surviving, all rounds | 1 | 1 | 1 | `S arms` |
| Escaped findings (evaluator) | 0 | 0 | 1 documentation omission (NEW-4), caused by director rule D-037, not by the tests | `evaluations/iter-1.md`; `tasks/T-011-memory-vfs-observable.md:12` |

**Confounders (INFERRED).** T-018 is the largest of the three (kiln-text selection and batch
edits plus two command groups, two scorecard items, 11 rulings) and its three review defects are
all sentences in `docs/protocol/eol.md`, not code; T-010's cost includes a 226,395-token stop of
both builders on a stale spec (D-059), which has nothing to do with the arm; T-011 is a CLI
option over an existing file system. Parity assignment did not balance the work.

**Verdict.** n=2 against n=1. The sample supports **no conclusion** about either arm: no arm
caught a defect at join, each left exactly one surviving mutant, the kill rates differ by one or
two mutants, and the cost difference is explained by task size and an unrelated spec stop. The
one thing measured cleanly is that the implementer-first protocol was followed (tests committed
and failing before code), and that its tests killed 15 of 18 live round-1 mutants.

## Where defects were found, and at what cost

Counting rule (as retrospective 0): a **defect** is behaviour or a stated guarantee that
contradicts the spec or the code; a **spec gap** is a question the spec did not answer; a
**test gap** is a stated behaviour no test detects (here always a surviving mutant); a **nit** is
the rest. Review classes are the reviewers' own front-matter counts; the split of defects by
kind is mine, from each review's findings table.

| Mechanism | Found | Cost | Evidence |
|---|---|---|---|
| Pre-dispatch spec check (5 runs) | **16 blockers**, 48 ambiguities, 55 notes in T-015 (6), T-008 (6), T-016 (2), T-017 (1), T-032 (1); one `rework` verdict that split T-015 into T-015 and T-032 | 912,637 tokens (558,527 booked, 354,110 hand): about 57k per blocker | VERIFIED `S specchecks`; D-041, D-044, D-052, D-065 |
| Builders' pre-code check (T-010, no spec check) | 2 spec-vs-main mismatches → rulings q1–q5 | 226,395 tokens (both builders, before any work product) | R-107/R-108 notes; D-059 |
| Test author (questions, self-check) | T-017: a pattern of 34,000 groups made a search allocate about 55 GB (D-060); T-008: ruling v4 contradicted its own observable (D-074); T-018: three undecided behaviours (w1–w4, D-078); T-008 v1, v2 readings (D-068) | not separable from the test runs | decisions |
| Implementer (stops and reports) | T-008: a merged T-032 test contradicting its own document (D-069, blocked run R-122, 190,104 tokens); inode reuse defeating the identity check (D-076); T-013: two contradictory answers (D-040) | R-122 190,104; the rest not separable | decisions; run notes |
| Join (acceptance tests) | **0 implementation defects by record**; 1 by the R-070 note (T-013). 4 joins `gate_failed`: T-011 and T-008 at the `scorecard` step (stale generated page), T-015 at `no-skip` and at `fmt` | not separable | VERIFIED `S joins`; D-075; T-015 log (R-076, R-087) |
| Review round 1 (9 reviews) | defect **11**, spec gap 16, test gap 10, nit 14. Defects by kind: 1 product boundary escape (T-008 D-1, root swapped between requests); 5 in the `publish`/hygiene tooling (T-013); 2 brittle guards in test files (T-015); 3 documentation sentences wider than the code (T-008 D-2, T-018 #1, #2) | 1,720,532 tokens; 156k per round-1 defect | VERIFIED `S reviews`; `reviews/*-r1.md` findings tables |
| Review round 2 (8 reviews) | defect **3**, spec gap 3, test gap 6, nit 6. All 3 defects are documentation sentences in the delta (T-008 R2-D1, T-017 #1, T-018 #1) | 641,505 + 14,844 tokens | VERIFIED `S reviews` |
| Review round 3 (3 reviews, confirm only) | 0 | 29,628 tokens | VERIFIED `S reviews` |
| Mutation spot-check (inside review) | every test gap above; 10 surviving mutants in round 1, 3 in round 2 | inside review cost | VERIFIED `S reviews` |
| Revert check (5 runs by reviewers; removed by v1.1 §6) | 0 defects; it exposed the join's file-stem miscount once (T-018 saw 6 of 12 targets, D-084 x6) | not separable | VERIFIED `S joins`; `reviews/T-018-r1.md` R-144 note |
| Queue | 0 product defects; 1 rebase conflict and 2 precondition refusals, all three caused by the director (D-089, D-050, D-058) | 12 merged runs, 1,948 s of gate time | VERIFIED `S queue` |
| CI (GitHub Actions on every push) | 0 failures reported | about 128 runs of about 3 minutes (INFERRED from run numbers 57 at retrospective 0 and 185 at the T-008 merge push; handoff lines 18, 112); not verifiable here | handoff |
| Iteration-1 evaluator (R-152, after the close) | 0 behaviour defects; 28/28 items confirmed; **4 documentation findings** no review, spec check or gate found (NEW-1…NEW-4); reproduced the accepted T-008 gap | 237,085 tokens: 59,271 per finding, 8,467 per item checked | VERIFIED `S evaluation`; D-090 |
| Iteration-0 evaluator follow-ups | T-010 and T-011 (filed in iteration 0), merged now: 3 items made observable through the CLI | 952,580 tokens (520,658 + 431,922) | VERIFIED `S tokens` |
| Director re-reading | see the director section: 11 of its own errors caught on re-reading | not recorded | `D` |

**Documentation sentences.** Five reviews found a documentation sentence wider than the code
and classed it a defect: T-008 r1, T-008 r2, T-017 r2, T-018 r1, T-018 r2 (6 sentences; D-081,
D-084 and D-087 count them as they happened). Three more reviews found a spec or test-comment
sentence wider than what it describes and classed it spec gap or nit: T-011 r1 #1 (a test
comment claiming a case its data lacks), T-013 r2 #3 (Pinned (a) contradicting ruling q),
T-016 r1 N1 (spec Goal "only adds packages"). None was narrower than the code. So **6 of the 14
review defects of the iteration are documentation sentences**, and **all 3 round-2 defects** are.
At least 3 of the 6 were written to carry out a director ruling: T-017 r2 (ruling s5), T-008 r2
(ruling v9 required a sentence that was not written; the director did not read the document
before the join, D-081), T-018 r2 (the wart that ruling x2 asked to be described). VERIFIED
(`reviews/*` findings tables; decisions).

**What this says (INFERRED).** Review, by adversarial use and by checking sentences against
behaviour, again found most of what was found: 14 defects against 0–1 at join. The one
security-relevant product defect (root swap, T-008) passed two spec checks, eleven independent
acceptance tests and the implementer's tests (D-072). Independent and implementer-written
acceptance tests both functioned as mutation-killing regression nets (81 % round 1), not as
defect finders.

## The director as a failure source

The table `D` lists 40 errors or corrections in iteration 1, each with its source. Tallies,
VERIFIED by `D`:

| By cause | n | By who caught it | n |
|---|---|---|---|
| ruling, answer or instruction with wrong content | 14 | an agent (builder, reviewer, planner, checker) | 13 |
| record or bookkeeping | 7 | the director on re-reading | 11 |
| required step omitted | 7 | a tool (queue or metrics refused) | 5 |
| brief or operator error | 6 | the owner | 1 |
| timestamp from memory | 4 | nobody (still in the record, or found here) | 7 |
| spec accepted that could not be built | 2 | not recorded | 3 |

19 of the 40 reached a builder before they were corrected. Of the 14 ruling errors, 9 reached a
builder; by sub-cause, 8 were not checked against existing text, tests or other answers, 3 stated
a bound without a measurement (s1, s5, the first draft of D-070), and 3 pinned a mechanism where
only behaviour should be pinned (v4, the "no held handle" message, the first wording of w1).
VERIFIED `D`.

Items the brief named, checked against the record:
- **D-077 "counts twelve and names two causes"**: D-077 calls itself the twelfth corrected
  ruling or instruction of the experiment and names one cause ("a bound stated without a
  measurement"); D-076 names the other ("pinned a mechanism"); the handoff (16:32Z) names both.
  The count runs across both iterations: D-031 is "third" (iteration 0), D-040 "fourth", D-044
  "sixth", D-068 "eighth" … D-077 "twelfth". The fifth and seventh are not identified in any
  row (VERIFIED `S decisions`, "ordinal counts"). My table has more because it also counts
  rulings corrected before they were committed and non-ruling errors.
- **Wrong timestamps**: 4 rows (D-045, D-055, commits `3976c53` and `b9350ee`); 16 runs carry a
  director-entered start time.
- **A metrics row with invented figures removed before commit**: **not found in the record**. What
  the record does show: R-082 was committed with `tokens: 200000` and replaced 33 seconds later
  (commits `ea1761c`, `70f652b`), and R-056 still carries a placeholder `tool_calls: 1`. VERIFIED `G`.
- **An approval row that did not begin with the required words**: D-057; the queue refused T-032
  once; D-058 restated it (VERIFIED `S decisions`).
- **A brief that caused a rebase conflict**: D-089, R-150's brief; queue attempt 1 `conflict` at
  `rebase` (VERIFIED `S queue`).
- **Briefs with abbreviated ids**: one recorded instance, D-050 (a 7-character `reviewed_sha`);
  the queue refused T-015 once.

**What this shows (INFERRED).** The mechanical checks T-005 and T-013 added caught every error
they were built for (approval wording, abbreviated sha, booking order, transition order) and
nothing else. The director's largest error class, wrong rulings, was caught by the agents who had
to act on them, at the price of a stop, a question or a review round. The v1.1 rule meant to
catch them first, a ruling "checked for implementability by a planner or reviewer run", was not
followed in that form: the director checked rulings himself (D-047, D-051, D-070 say so), and 0
of 31 decision rows that mention a ruling cite the commit that wrote it (5 cite a reviewed or
merged head, none the ruling commit; VERIFIED `S decisions`, read row by row). Iteration 0 had 14
director errors with 0 caught mechanically; iteration 1 has 40 with 5 caught mechanically, over
three times as many tasks dispatched.

## Spec quality

Rulings per task, counted by identifier in the task file's sections before `## Log` (VERIFIED
`S rulings`; T-015's are D-043, the AC-5 line and D-046 points 1–3; T-016's one ruling is the
pre-dispatch R-079 corpus ruling):

| Task | Tier | Post-dispatch rulings | Reversed or amended later | Drafted before a dependency merged? | Spec check |
|---|---|---|---|---|---|
| T-013 | high | 17 (a–q) | 1 (h → D-040) | no (T-005, T-014 merged) | none (dispatched before the rule, D-041) |
| T-008 | high | 13 (v1–v13) | 4 (v2 corrects D-065's edit, v8 corrects v4, v9 withdraws an instruction, D-061 reverses D-052) | yes (T-032) | R-101 + re-check R-115 |
| T-018 | medium | 11 (w1–w4, x1–x7) | 0 (x2 keeps a wart w2 did not foresee) | yes (T-010, T-015) | none |
| T-010 | medium | 9 (q1–q5, t1–t4) | 0 (q2 drops a spec sentence) | yes (T-032) | none |
| T-017 | high | 9 (s1–s9) | 2 (s4 corrects s1, s8 corrects s5) | no | R-067 |
| T-032 | high | 6 (r1–r6) | 0 | yes (T-015) | R-069 |
| T-011 | medium | 5 (u1–u5) | 0 | no | none |
| T-015 | high | 5 | 1 (D-046 point 2 amended, `2e1c206`) | effectively (T-013 held its paths, D-041) | R-062 (`rework`) + R-069 |
| T-016 | high | 0 (+1 pre-dispatch, R-079 replacing R-077) | 1 (R-077) | no | R-067 |
| T-006, T-014, T-035 | low | 0 | — | — | — |

Total: **75 post-dispatch rulings on 9 tasks, 9 reversed or amended** (8 post-dispatch plus
R-077). VERIFIED for the counts (`S rulings`); the reversal column is from reading each ruling.

**Specs drafted before their dependencies merged (INFERRED from the rows above):** four (T-008,
T-010, T-018, T-032). T-010, which had no check, stopped both builders at their pre-code check
(226,395 tokens, D-059). T-008 needed two checks (271,944 tokens booked), 6 blockers, and still
three pre-join rulings (v1–v3) and a blocked run (R-122). T-032's check found 1 blocker, a T-015
guard pinning output T-032 changes. T-018 had no blocking mismatch; its test author's three open
questions became w1–w4. The pattern: what goes stale is not the dependency's API but its
**tests** (guards and acceptance tests pinning messages, documents and command lists).

**What the pre-dispatch checks found for their cost (INFERRED).** 912,637 tokens for 16 blockers.
D-041, D-044 and D-052 describe each blocker as something that would otherwise have stopped a
builder or failed a join; D-059 shows what one such stop cost on T-010 (226,395 for two
mismatches). On that comparison the checks paid for themselves roughly once over. What they
missed: the root swap (T-008, found in review), the 34,000-group memory abort (T-017, test
author), the unmeasured time bound (T-017, review), unbounded parse memory (T-016, review), and
a brittle guard the checker flagged itself minutes after its verdict (D-043).

## Tiering and budgets

Were the v1.1 ceilings right? **No, as predictors**: 7 of 12 merged tasks exceeded them as booked,
8 of 12 with the unbooked check counted (`S tokens`). By tier:

| Task | Ceiling | Booked | × | Builder + review only | What the excess went on |
|---|---|---|---|---|---|
| T-013 high | 850k | 1,790,657 | 2.11 | 1,751,746 | seven tooling items in one spec (D-054); post-r1 rework 790,202; usage-limit stops R-089/R-090 (354,339 as context sizes) and continuations R-091/R-092 (225,977); one review lost to a restart (R-075, unknown) |
| T-008 high | 850k | 1,549,053 | 1.82 | 1,152,495 | two spec checks 271,944; planner shares 124,614; post-r1 rework 543,785 after the root-swap defect and three corrected rulings (v4, v8, v9); a usage-limit stop (R-130/R-131) |
| T-015 high | 850k | 1,124,087 | 1.32 | 837,504 | spec check R-062 286,583; post-r1 rework 287,842 on two brittle guards (tests, not code) |
| T-017 high | 850k | 737,467 (884,915 +hand) | 0.87 (1.04) | 737,467 | post-r1 rework 251,948 on two corrected director rulings (s4, s8) |
| T-016, T-032 high | 850k | 693,067; 625,593 | 0.82; 0.74 | same | within |
| T-018 medium | 500k | 848,301 | 1.70 | 848,301 | rework after r1 (x1–x6) and r2 (x7, a documentation sentence): 165,616 after r1; three builder roles each about 300k |
| T-010 medium | 500k | 520,658 | 1.04 | 481,747 | the stale-spec stop, 226,395 |
| T-011 medium | 500k | 431,922 | 0.86 | 393,011 | within |
| T-006 low | 150k | 255,816 | 1.71 | 131,202 | planner shares 124,614 booked to it |
| T-014 low | 150k | 261,060 | 1.74 | 0 | the whole record is methodology authoring (R-053), booked to it as planner |
| T-035 low | 150k | 73,401 | 0.49 | 73,401 | within |

VERIFIED `S tokens`, `S budgets`, `S rework`; causes from D-049, D-054, D-066, D-082, D-088 and
run notes.

Every Extend row raised the ceiling to just above what had already been spent: 1,150,000 for
1,124,087; 1,800,000 for 1,790,657; 525,000 for 520,658; 1,560,000 for 1,549,053; 850,000 for
848,301 (100.2–102.3 %), each committed 2–14 minutes before the queue run (VERIFIED `S tokens`,
`G` "Extend rows vs queue merge record"). No task was stopped, split or abandoned on budget.
Excluding planner shares and spec checks, only T-013 (2.06×), T-018 (1.70×) and T-008 (1.36×)
exceed their ceilings, and all three overran in rework after review.

## Tooling and environment

| Defect or incident | Occurrences and cost | Evidence |
|---|---|---|
| Join identifies acceptance targets by file stem: two `acceptance_t018.rs` files in different crates counted once | T-018 join recorded 6 of 12 tests; the revert check saw 6; fixed by renaming a file (x6) | D-084; `S joins` (16:56:49Z, `acc 6`) |
| Generated `docs/scorecard.md` stale at join when the tests branch changes a scorecard item; three documents disagree on who edits `scorecard/` | 2 joins `gate_failed` at `scorecard` (T-011, T-008); director regenerated the page on the branch | D-075; handoff line 105; `S joins` |
| `reviewed_sha` must be full 40-hex | queue refused T-015 once (the tool was right; the brief was wrong) | D-050 |
| Approval row must begin `Approve T-NNN` | queue refused T-032 once (the tool was right) | D-058 |
| Queue precondition refusals not written to `metrics/queue.jsonl` | 2 refusals invisible to the first-pass metric | `S queue` |
| `metrics report` exits 1 for ever, because T-012 and T-014 overran before the rule existed | the exit code cannot be used as a check | handoff line 110; `R` |
| `metrics add` refuses a record that ends before the task's latest record | R-123 refused after the `merged` transition; a commit named the run without containing it | handoff line 104 |
| `metrics transition` positional form writes nothing | — | handoff line 121 |
| `metrics join-defects` | never used; the two failing T-013 joins stay unclassified. The brief calls it read-only; `docs/development.md:417` says it appends a `join_defects` record, so I did not run it | `S joins`; `docs/development.md` |
| Test author cannot run the full gate on the tests branch | T-015 join failed at `fmt`; fixed by R-087 (902 tokens) | R-087 note |
| Disk | "full gate not run for lack of disk" in R-081, R-135, R-140, R-147; disk fell to 2 GB before the D-045 restart | run notes; D-045 |
| Usage-limit stops | 3 stops; **about 6.8 h with no agent running** of the 14.76 h from the D-035 commit to the last merge: 08:13–11:12 (2.99 h) and 14:19–16:13 (1.90 h) are measured gaps in the run records; 04:15–06:10 (1.92 h, D-038) is hidden inside R-056/R-058/R-059, whose wall times include it | VERIFIED `S idle`; D-038 |
| Container restarts | 3; 3 runs lost with no figure (R-075, R-078, R-080); a half-applied mutation left in a worktree (D-045) | D-045, D-073 |
| Cost of restarts and stops | blocked runs 668,277 tokens as booked (context sizes and differences; usage-limit stops only); fresh continuations R-091, R-092, R-093, R-132, R-133, R-134: 744,089 tokens, which include new work. The share that was repeated work is not recorded. | VERIFIED `S budgets` |
| CI | every push to `main` starts a run of about 3 minutes; about 128 runs in the iteration (INFERRED, handoff run numbers); 15 commits in the iteration touched only `handoffs/` or `STATE.md`; the push count itself is not recorded | `G`; handoff lines 18, 112 |

## Throughput and concurrency

| Measure | Value | Command |
|---|---|---|
| Span | 14.76 h from the D-035 commit (03:11:05Z) to T-018's merge (17:56:51Z); 15.54 h from the first iteration-1 run (R-049, 02:24:37Z) | VERIFIED `S idle` |
| Merged tasks per hour | **0.81** (12 / 14.76 h); iteration 0: 1.2. Without the 6.8 h of usage-limit stops: 12 / 7.95 h ≈ 1.5 (INFERRED arithmetic) | VERIFIED `S idle` |
| Tokens per hour of span | 10,985,201 / 14.76 h ≈ 0.74 M | VERIFIED `S tokens`, `S idle` |
| Builder-role runs live, time-weighted mean (03:52–17:51Z) | **1.32**; none live 43 % of the time | VERIFIED `S concurrency` |
| Maximum live | builder roles: 4 (at 11 run starts, the first at 06:26:05Z; reviewers and a spec check included); implementer and test author only: 4 at one instant (06:30:00Z, hand-entered times); all roles incl. planners: 5 at 12:49:40Z | VERIFIED `S concurrency` |
| Sum of run wall time | builder roles 18.45 h over 75 runs | VERIFIED `S concurrency` |

INFERRED: the "fan-out" iteration ran on average 1.3 builder agents. The cap of three was not the
constraint; the usage limit was (nearly half the span idle), followed by serial dependencies
(T-013 → T-015 → T-032 → T-008/T-010 → T-018) and review rounds.

## Charter exit criterion

The charter requires at least 60 % of scorecard items verified at the end of iteration 1. Main
has 64 items: 22 verified (34.4 %), 6 implemented, 36 planned (VERIFIED `S scorecard`). 60 %
needs **39**; the shortfall is **17 items**. If the evaluation confirms all 6 implemented items,
28 of 64 (43.8 %), still **11 short**.

Why, from the data:
- The twelve merged tasks claimed **6 items** (VERIFIED `S scorecard`): 3 were iteration-0 items
  the evaluator could not observe (text.lines, text.positions, ws.vfs: T-010, T-011), 1 is a new
  item created by T-008 (ws.path-confinement), and **2 moved from planned** (edit.selection,
  io.mixed-eol: T-018). Planned went from 38 to 36.
- Tokens by kind of work (VERIFIED `S tiers`, of 10,985,201): tasks with scorecard items
  3,349,934 (30.5 %); contract and library tasks with no item (T-015, T-016, T-017, T-032)
  3,534,324 (32.2 %); process, tooling and documentation (T-006, T-013, T-014, T-035) 2,380,934
  (21.7 %); planner overhead 1,454,555 (13.2 %) plus planner shares on drafts 265,454 (2.4 %).
- Time: about 46 % of the span had no agent running (usage limits); the product track was
  serial behind T-013 and T-015 (handoff "Plan for the rest of iteration 1").
- Cost per item (INFERRED): 10.99 M tokens for 2 planned items moved and 1 new item, or 1.83 M
  per item the evaluator can now confirm (6); iteration 0 spent about 223k per verified item.

After the evaluation (R-152) and the publication of its file (T-040), the scorecard reads 28 of
64 (43.8 %): the shortfall against the criterion is **11 items**, exactly the maximum computed
above, because the evaluator confirmed every implemented item.

INFERRED: the iteration bought contracts (structured refusals, tree-sitter, regex, the scaffold)
and process tooling, which the drafted feature tasks need, and very few items. The exit criterion
was set for feature fan-out that did not happen inside the iteration.

## Claims

| Claim | Verdict | Numbers that decide it | What would change it |
|---|---|---|---|
| C1 contracts and checks first | **insufficient** | Contracts did go first: T-015, T-032 (ADR-0004), T-016, T-017 merged before their consumers; the T-015 reviewer merged five branches shaped like the parallel tasks with no conflict (D-046). But fan-out was 1.32 agents on average (`S concurrency`), and the consumer rework that occurred came from **tests attached to the contracts** (guards and message pins: D-043, D-046, D-052, D-059, D-061, D-069; one blocked run of 190,104 tokens) rather than from contract shape. Contract tasks cost 32.2 % of tokens for 0 items. | Iteration 2 with ≥ 3 consumers in flight: consumer rework traced to contract shape ≈ 0 → supported; contract changes forced after fan-out → refuted. |
| C2 serial queue keeps main green at agent speed | **supported** (weak) | 12/12 merged; 11/12 first attempt by record (the 1 failure a director-made conflict); 0 incidents; CI success when checked (handoff, not re-verified). Gate time grew to a median 199 s (`S queue`). Queue waiting never dominated lead time (merges at 12:14, 12:22, 12:26 back to back). | A red main, or queue wait > 25 % of lead time with ≥ 3 tasks waiting. |
| C3 different-author tests catch defects implementer tests miss | **insufficient** (close to refuted) | Iteration 1: **0** acceptance-caught implementation defects by record, **1** by run note (T-013, R-070) across 8 tasks with independent tests and 2,565,831 test-author tokens; review found 14 defects. Iterations 0+1: 2 by note across 13 such tasks. Retrospective 0's test: ≥ 3 across ≥ 10 → supported; 0 → refuted. Neither holds. Medium arm: no difference measurable (n=2 vs 1). | One more iteration with 0 acceptance-caught defects across ≥ 5 independent-test tasks → refuted. |
| C4 derived status + evaluator prevent over-claiming | **supported** | Status is derived (gate `scorecard --check`); the evaluator confirmed 6 of 6 newly implemented items and 22 of 22 regression items through `kiln-cli` only, 0 rejected, 0 not evaluated (`S evaluation`). Over-claiming moved from status to **documents**: 6 sentences wider than the code caught by review, and 4 more (2 of them published kinds for features that do not exist, "wired but inert" in the evaluator's terms) caught only by the evaluator. Neither status nor tests caught any of the 10. | An evaluator-confirmed item later found inert or wrong. |
| C5 risk tiers put review effort where defects are expensive | **insufficient** | Medium tier now exists: 3 tasks, 7 reviews, 3 defects (all documentation sentences, T-018); high: 6 tasks, 13 reviews, 11 defects, including the only boundary escape (T-008) and all 5 tooling defects (T-013) (`S reviews`). Retrospective 0's test needs ≥ 5 tasks per tier; medium has 3. Round-1 review cost is similar across tiers (medium 133k–188k, high 177k–262k). | ≥ 5 medium tasks with ≈ 0 code defects at review and lower review cost than high. |
| C6 budgets that gate stay honest | **refuted** (as a gate) | Token budgets were enforced mechanically from 12:14Z, and every overrun queued after that had an Extend row (T-010, T-008, T-018), which meets retrospective 0's wording. But all 5 Extend rows set the ceiling at 100.2–102.3 % of what was already spent, none stopped, split or re-specified a task; T-006 and T-014 overran with no row; T-017's overrun is hidden by an unbooked spec check; latency budgets: still 0 enforced (T-028 is a draft). | A budget decision taken before the spend that changes what is built (a split, a follow-up instead of rework), recorded and mechanically checked. |
| C7 repo state makes work resumable and auditable | **supported** | 3 usage-limit stops and 3 container restarts; every interrupted task was resumed from the repository and merged; uncommitted work survived on disk (D-073). Audit: this report reproduces its numbers from the repository. Against: 2 unbooked runs, 3 runs lost without figures, 0 director runs, 9 delta and 5 context-size token figures. | A resume that fails for lack of a record; or a number the repository cannot reproduce. |
| C8 constructed oracles define correct | **insufficient** | Naive models and exhaustive checks were used by test authors and reviewers (T-017 20,000 pairs; T-018 19,683 cases; T-010 490 checks; T-016 corpus and `syn`); they found 0 code defects and showed 3 documentation sentences wrong (T-018). The evaluation found 0 behaviour defects in any item, oracle-backed or not, so there is no contrast to measure. | Escaped behaviour defects in items without an oracle and none in items with one, over ≥ 10 items each. |

## Where the methodology was wrong

**Rules not followed** (VERIFIED unless marked):
- §2.1 one `role: director` run per session: 0 records in two iterations.
- §2.1 dispatch writes a run record at start, with "model matches": records are written after the
  run ends (R-123 was booked after its task merged), and the record has no model field.
- §2.1 a ruling is checked by a planner or reviewer run, and its row cites the commit: checked by
  the director alone (D-047, D-051, D-070); 0 of 31 ruling rows cite the ruling commit.
- §2.1 "a stopped or interrupted agent is relaunched fresh, never resumed": agents stopped by
  the usage limit at 04:15Z were resumed (D-038), builders were resumed after rulings (R-107,
  R-108), reviewers were resumed for rounds 2 and 3 (R-149, R-151); the handoff's own rule says
  "agents stopped by a usage limit are resumed" (line 62). Rule and practice disagree.
- §2.1 every timestamp from the clock: 16 director-entered start times; 4 corrected handoff times.
- §5.1 dispatch only `ready` tasks: R-072/R-073 (T-015), R-058/R-059 (T-013, hand times).
- §5.4 at most 3 builder agents: 4 builder-role runs live at 11 run starts (reviewers counted;
  `S concurrency`); 4 implementer/test-author runs at one hand-entered instant.
- §6 the revert check is gone: 5 revert checks run by reviewers.
- §11 interim Extend rows: T-006, T-014 overran without one.
- §12 every run has a record and a log entry: see Data.
- §13 `acc_defects` required when `acc_fail_first_join > 0`: T-013's two joins.
- §3 rework passes through `self_verified`: 5 skips, all before T-013 merged.
- CHARTER §5 exit criterion: the iteration was closed without meeting it (D-089, under D-085).

**Rules that cost effort and caught nothing:**
- Approval rows for high tier: 7 rows (D-048, D-053, D-056, D-057/D-058, D-079, D-083); each
  followed the reviewer's `approve`; one caused a queue refusal (D-058). Same in iteration 0.
- Extend rows: 5, each written minutes before the queue run at 100.2–102.3 % of the spend.
- Revert checks: 5 run, 0 defects (one exposed a tool miscount).
- The publish-through-the-queue path for documents that are not ledger paths: T-035 cost 73,401
  tokens and a 204 s queue run to append one sentence that ADR-0004 had already accepted (D-086).

**Missing rules (failures no rule addressed):**
- Documentation sentences stating guarantees are not tied to tests or measurements: 6 review
  defects, all 3 round-2 defects.
- Tests that pin messages, documents or command lists owned by later tasks: 6 incidents (D-043,
  D-046 ×2, D-052, D-069, T-010 r1 nit 3); written down only in a handoff addendum (12:30Z).
- Resource bounds: specs taking a dependency that processes untrusted input had a time line and
  no memory line (D-056, D-060, D-070).
- Usage limits: no rule schedules a resume; one stop had no check-in (handoff 16:16Z).
- Queue refusals are not recorded.

**Open questions now answerable:**
- O-3 (re-review after a queue failure): one queue failure (T-018 attempt 1, a task-file conflict);
  re-queued 40 s later without re-review. Answer: a conflict confined to ledger text needs none.
- O-8: still open; `acc_defects` was never filled in iteration 1 and `join-defects` never used.
- O-10 (director cost): not observed in two iterations; treat as not measurable.
- O-12 (independence): unchecked for test authors; for the implementer-first arm, the order
  "tests committed before code" is checkable from commits and held (T-011).
- O-13 (real concurrency): answered within the limits of the times: mean 1.32 builder-role runs.

## Proposed changes (methodology v1.2)

All PROPOSED. Each names the metric or incident behind it and what it removes or replaces.

| # | Change (file, section) | Motivating metric or incident | Expected effect | Check | Removes or replaces |
|---|---|---|---|---|---|
| 1 | §11, §13 and `xtask metrics`: book every planner run and every pre-dispatch spec check to `iter-N` (allow `role reviewer` with a spec-check flag there); a task's budget covers only its builder and review runs | 354,110 tokens never booked (D-055); spec checks pushed T-015 and T-008 over budget (286,583; 271,944); T-017's overrun hidden; T-006 and T-014 "overruns" are planner bookings | budgets measure building; no hand counts | `S completeness` finds no unbooked spec check | D-055's booking rule; the handoff's list of unbooked runs; even splits of planner runs over tasks |
| 2 | §11: the budget decision moves to the round-1 verdict. When round 1 is `request-changes` and the task's tokens are at or above 75 % of its ceiling, the director decides in a decision row, before any rework run, what the rework covers (defects only, or all findings) and what goes to a follow-up | 5 Extend rows at 100.2–102.3 % of spend, none changing anything; post-r1 rework 2,194,642 tokens (24.6 %); T-013, T-018, T-008 overran only in rework | a budget that can change what is built | decision row timestamp before the first post-r1 run (`S rework`) | the queue-time Extend row ritual (the queue keeps only a check that the decision row exists) |
| 3 | Budget recalibration (§11): see the section below | 7 of 12 over ceiling | fewer meaningless overruns | `S budgets` | — |
| 4 | §2.1 rulings: a ruling states observable behaviour, never a mechanism; any bound it states or asks a document to state (time, memory, size) cites the measurement it rests on; each builder acknowledges a ruling (accept or question) in its log before acting on it | 14 ruling errors, 9 reached a builder, 9 caught by agents; 3 unmeasured bounds (s1, s5, D-070 draft), 3 mechanisms (v4, the handle message, w1 draft); the planner/reviewer check was never run as written | ruling errors caught before work, by the people who caught them anyway | count of corrected rulings per iteration (`D`) | the unfollowed "checked by a planner or reviewer run" and "row cites the commit" |
| 5 | §6 and the planner brief: a sentence in `docs/protocol/` that states a guarantee or a limit is written in the spec (Pinned) with its evidence, and the implementer copies it; implementers do not write new guarantee sentences | 6 of 14 review defects, 3 of 3 round-2 defects were documentation sentences; 3 of 6 came from text written for a ruling | fewer round-2 rounds (each cost a rework run and a review) | review defects of kind "documentation" per iteration | the implementer's freedom to word guarantees; the reviewer's need to audit every new sentence from scratch |
| 6 | §6: a test may not pin a message text, a whole document or a command list that a drafted task changes; "this task left X alone" is a reviewer's diff check, not a permanent guard | D-043, D-046 (two guards), D-052, D-069 (blocked run, 190,104 tokens), T-010 r1 nit 3 | no later task blocked by an earlier task's guard | grep of guard files at review | the handoff-only operating rule; ad-hoc guard-rewrite rulings |
| 7 | §4/§5: a spec whose `depends_on` had not merged when it was written gets one pre-dispatch check run, whatever its tier, replacing the builders' parallel pre-code checks | T-010 (no check): both builders stopped, 226,395 tokens for 2 mismatches; checks found 16 blockers for 912,637 | one check run instead of two stopped builders | spec-check files per dispatched task | duplicated pre-code stops |
| 8 | §5, `docs/development.md`: one owner of `scorecard/` (the director, at join), stated in one place; the join regenerates `docs/scorecard.md` on the task branch | 2 joins failed at `scorecard` (T-011, T-008); three documents disagreed (D-075) | no manual regeneration | joins `gate_failed` at `scorecard` = 0 | the conflicting text in `docs/development.md` and specs; the manual regeneration step |
| 9 | `xtask join`/`queue` (T-033 scope): identify acceptance targets by path; refuse to write a failing join record without `--acc-defects`; write queue precondition refusals to `metrics/queue.jsonl` | T-018 counted 6 of 12; 28 joins with `acc_defects` null, 2 failing joins unclassified; 2 refusals unrecorded | acceptance-caught defects measurable (C3) | `S joins`, `S queue` | `metrics join-defects` and the director's later classification; renaming rulings like x6 |
| 10 | `docs/development.md`, reviewer brief: delete `join --revert-check` | 5 run in iteration 1, 0 defects; v1.1 §6 already removed it | less review time | — | the revert check |
| 11 | §4 approval row: the queue reads the final review's `approve` plus a `gate_paths`/`dependency_versions` field in its front matter; the decision row is written by the tool from it | 7 approval rows in iteration 1 (5 in iteration 0) all followed the review verdict; 1 refusal from wording (D-058) | one fewer director step that can fail | queue refusal tests | the hand-written `Approve T-NNN` row |
| 12 | §2.1 interruptions: resume after a usage-limit stop, relaunch fresh after a container restart (with `git status` first); schedule a resume at every usage-limit stop | rule and practice disagree (D-038, handoff line 62); 6.8 h of 14.76 h idle; one stop with no check-in (handoff 16:16Z) | less idle time; a rule that matches practice | idle gaps in `S idle` | the contradictory "never resumed" clause |
| 13 | §2.1 and §13: drop "one `role: director` run per session" and the "Director cost" metric; the handoff records each director session's start and end time | 0 director records in two iterations; O-10 | honest metrics table | — | an unfollowed, unmeasurable rule |
| 14 | CI configuration (gate path, high tier): run GitHub Actions on the queue's merge commits, not on ledger-only pushes | about 128 CI runs of about 3 minutes (handoff); every merge already passed main's gate in the queue; 15 handoff/STATE-only commits | fewer CI runs at no loss of checking | CI run count per merged task | CI runs on ledger pushes |
| 15 | §8 ledger paths: text that an accepted ADR adds to `docs/CHARTER.md` is committed with the ADR as ledger | T-035: 73,401 tokens and a queue run for one sentence (D-086) | — | — | low-tier publish tasks for ADR text |
| 16 | CHARTER §5 (by ADR) and the iteration-2 plan: iteration 2 must first meet iteration 1's 60 % line, with the scope below | exit criterion missed by 11 items after the evaluation (28/64) | an iteration sized to what was measured | `S scorecard` | — |
| 17 | §7 round 1 and the reviewer brief: the reviewer checks every sentence the task adds or changes in `docs/`, **and every sentence elsewhere (other `docs/protocol/` pages, `docs/control-protocol.md`, the generated `error_kinds`) that names the same command, option or kind**, against the built binary, not against the code or spec | the evaluator found 4 such mismatches after 3 review rounds of T-008, 2 of T-032 and 2 of T-011 had not (D-090); reviews compared code with task files and the task's own documents | fewer documentation findings at evaluation | evaluator findings of kind "document vs binary" per iteration | — (replaces part of the reviewer's code-reading with binary checks it already runs for ACs) |
| 18 | D-037 rule ("a task touches `docs/control-protocol.md` only for error-kind lines") and T-032's design of `error_kinds`: a task that changes the CLI surface updates the CLI synopsis and exit-code table it changes; a kind is published only when a registered command can produce it | NEW-4 (T-011 was told not to touch the file, `tasks/T-011-memory-vfs-observable.md:12`); NEW-2 (11 kinds of unbuilt features published as the "specified set") | no stale synopsis, no inert kinds | evaluator findings | the D-037 restriction for CLI sections; reserved kinds in the published set |

## Budget recalibration

Rule (unchanged from v1.1): p75 of the tier's recorded totals for merged tasks, rounded up to
50k, computed on builder and review runs only (change 1). VERIFIED `S budgets`:
- **High: 1,100,000** (p75 1,073,747 of 625,593; 693,067; 737,467; 837,504; 1,152,495; 1,751,746).
- **Medium: 700,000** (p75 665,024 of 393,011; 481,747; 848,301; n=3, provisional).
- **Low: 150,000** (p75 116,752 of 73,401 and 131,202; T-014 has no builder runs).

INFERRED: raising the ceilings records reality; it does not lower cost. The three large overruns
were rework after review (change 2) and one bundled tooling task (T-013: seven items in one
spec). A high-tier spec that bundles independent items should be split at `specified`.

## Recommendation on the scope of iteration 2

The eighteen drafts (VERIFIED `S projection`, `S scorecard`): 10 high (T-007, T-009, T-019,
T-024, T-027, T-028, T-029, T-033, T-034, T-036) and 8 medium (T-020, T-021, T-022, T-023,
T-025, T-026, T-030, T-031); their budgets sum to 12,500,000.
- At iteration-1 medians (+hand): 10 × 1,004,501 + 8 × 520,658 = **14,210,274** tokens; at means
  16,259,073. With iteration 1's overhead factor (total 10,985,201 ÷ merged-task 9,265,192 =
  1.19): about **16.9–19.4 M**, i.e. 1.5–1.8 × all of iteration 1.
- At iteration 1's rate (0.74 M tokens per hour of span, usage stops included) that is about
  23–26 hours, three or four more usage-limit stops at the observed frequency (INFERRED).
- None of the 8 `ui.*` items (the charter's purpose for iteration 2) is claimed by any draft;
  28 of the 36 planned items are.

PROPOSED scope, smallest that meets the 60 % line: T-019 (high, ws.open-project), T-020 (medium,
4 syntax items), T-021 (medium, 4 search items), T-023 (medium, 4 outline and navigation items;
needs T-019): 13 items. With the 6 implemented items confirmed, 22 + 6 + 13 = 41 of 64 (64 %).
Cost at iteration-1 medians: 1,004,501 + 3 × 520,658 = **2.57 M** (2.95 M at means). Add T-036
(high; it closes the risk accepted in D-081/D-083): about 1.0–1.15 M. Total **3.6–4.1 M**, about a
third of iteration 1. Defer T-009, T-028, T-029, T-034 and T-007/T-030 until an incident needs
them; cut T-033 down to change 9. Then draft the UI tasks.

All dependencies of the proposed tasks are merged (T-019: T-015, T-008; T-020: T-015, T-016,
T-032; T-021: T-015, T-017, T-018, T-032; T-023: T-016, T-018, T-019, T-032; VERIFIED in front
matter). T-020 (even) and T-023, T-021 (odd) continue the §6 arms.

## Uncertainty

- Samples: 6 high, 3 medium, 3 low; arms 2 against 1. Every by-tier and by-arm statement is a
  description, not an estimate.
- Token figures: 9 differences of running totals and 5 context sizes (see Data); 3 lost runs with
  no figure; director tokens absent; the split of R-067 and R-069 between tasks is my convention.
  Totals containing deltas are lower bounds of unknown size.
- Times: 16 director-entered starts; R-056 spans five segments with idle gaps; the 04:15–06:10
  stop is taken from D-038, not from records.
- CI run counts come from the handoff's run numbers, not from GitHub (not reachable here).
- Classification: review classes are the reviewers'; the split of defects by kind, the reversal
  column of the rulings table and the director-error table are mine (`D` lists every row and its
  source). The "acc_defects = 1" for T-013 rests on a run note, not a record.
- I could not establish: who caught 3 of the director's errors (D-041 report path, D-045
  timestamps, the D-046 amendment); which errors are D-077's fifth and seventh; how many tokens
  the restarts repeated; whether test authors read implementation branches (unchecked, O-12);
  which model each run used (O-11); the actual number of pushes to `main`.
- Evaluation: the attribution of NEW-1…NEW-4 to origin tasks is mine (git history of the
  sentences, below); the evaluator attributed NEW-2 and NEW-3 to T-035 "by its file name; not
  read", which the history contradicts. T-037, T-038 and T-039 had no task files when I read the
  repository, so `found_by`/`origin_task` could not be checked.
- Drift: after the head I was given (`b3d9dd2`), main gained a handoff addendum (`b2ee623`), the
  evaluation decision D-090 and R-152's record (`13981d2`), and a director commit of this run's
  in-progress scripts (`73a8d13`); D-089 itself is in `b3d9dd2`. `iter-1` overhead now includes
  R-152 (237,085 tokens) and will include this run, R-153, once booked; neither is in any build
  figure here. All git ranges are pinned to `b3d9dd2`.

## Evaluation of iteration 1

Source: `evaluations/iter-1.md` (R-152; untracked in the working tree when read, being published
as T-040), its drivers in `evaluations/iter-1-drivers/`, and D-090. Run record: R-152, evaluator,
`iter-1`, 237,085 tokens, 66 tool calls, 17:57:48–18:14:22Z (994 s). VERIFIED `S evaluation`.

**Result (VERIFIED from the file's front matter).** 28 items confirmed: the 6 that were
`implemented` (edit.selection, io.mixed-eol, text.lines, text.positions, ws.path-confinement,
ws.vfs) and, as regression checks, the 22 verified in iteration 0; 0 rejected; 0 not evaluated;
defects filed T-037, T-038, T-039. Interfaces: CLI and control protocol only. With capabilities
dropped it reproduced the gap accepted at T-008's merge (a removed and recreated unreadable root
received the same inode and a save went into the replacement; D-081, D-083, owed to T-036).
Once T-040 merges the scorecard reads 28/64 = 43.8 %, 11 items short of the 60 % criterion.

**Findings, and where they came from.** All four are documents saying more or less than the
binary; none breaks a scorecard claim. Origins are mine, from git history (VERIFIED commands in
brackets):

| Finding | What | Origin of the wrong text | Checks it passed |
|---|---|---|---|
| NEW-1 (T-037) | `doc.open` of an already-open path is not checked at all; `docs/protocol/paths.md` says every path is checked at open, and the root text says every request is refused once the root is replaced | the sentence is T-008's (high, independent); the unchecked branch is T-003's `already_open` path (`git log -S already_open -- crates/kiln-commands/src` → `b406cac`, iteration 0) | 2 spec checks, 11 + 8 independent acceptance tests, 3 review rounds of T-008 (one of them probing every root swap) |
| NEW-2 (T-038) | the generated `error_kinds`, which `docs/control-protocol.md` calls "the specified set", lists 11 `rejected` kinds of features that do not exist (`tool_unavailable`, `changed_on_disk`, …) | T-032 (high): `git log -S tool_unavailable -- crates` → `2975342` "T-032: structured reasons …; error_kinds in describe()"; declared by design in the T-032 spec | T-032 spec check R-069, 2 review rounds and an addendum; the `schema` gate step (it checks that the list is current, not that it is true) |
| NEW-3 (T-038) | `not_a_file` description claims "a file where a directory is needed"; the binary and `paths.md` give `not_found` | the planner's T-032 spec text (`git log -S "a file where a directory is needed"` → `8b6adfb`, 06:32Z), pinned word for word by director decisions D-052 and D-061 and by T-008's spec | 2 spec checks, T-032 and T-008 reviews (5 rounds in all) |
| NEW-4 (T-039) | `docs/control-protocol.md` CLI synopsis and exit-code table omit `--memory-seed` | T-011 (medium, implementer-written tests) was told not to edit that file: "docs/control-protocol.md is not touched" (`tasks/T-011-memory-vfs-observable.md:12`), under the director's D-037 rule | 2 review rounds of T-011 |

The evaluator attributes NEW-2 and NEW-3 to T-035 "by its file name; not read"; the history above
contradicts that (T-035 changed one charter sentence, D-086).

**What it found that review and the gate did not (INFERRED).** Every review compared code with
its task file and with the documents the task itself changed; the spec checks compared specs with
code on main; the gate checks that generated files are current. None compared one document with
another, or a document's general sentence with the binary's behaviour in a case the task did not
change. The evaluator did exactly that, from the outside, and found 4 mismatches in 994 seconds.
Three of the four sit in text that director rulings or rules pinned or restricted (NEW-3 pinned by
D-052/D-061, NEW-4 caused by D-037, NEW-2 by the T-032 design the director accepted with
ADR-0004). This extends the review finding above: 6 + 4 = 10 documentation sentences wider or
narrower than the binary in one iteration, against 6 code defects (T-008 D-1 and T-013's five).

**Cost (INFERRED arithmetic from `S evaluation` and `S reviews`).** The evaluator spent 59,271
tokens per finding and 8,467 per item checked. Review round 1 spent 156,412 per defect
(1,720,532 / 11) and 33,736 per finding of any class (51); round 2 213,835 per defect (641,505 / 3).
Per finding the evaluator is in the same range as review; it is the only mechanism of the
iteration that found document-to-document and document-to-binary mismatches outside a task's own
diff, at about one seventh of a round-1 review's total cost.

**Answers to the evaluator's questions** are in D-090 (Q1 accepted as behaviour and to be
documented by T-039; Q2 enough for the item; Q3 intended, T-039 adds a sentence, T-024 must
convert; Q4 no task). I have not checked them further.

**Effect on the claims.** C4 moves from insufficient to **supported** (28/28 confirmed, 0 inert
items; the over-claiming found was in documents, and it took the evaluator to find 4 of it). C8
stays **insufficient**: 0 behaviour defects escaped anywhere, so oracle-backed and other items
cannot be contrasted. C3 and C5 are unchanged: no implementation defect escaped either arm or
either tier; the escaped documentation findings come from high tasks (3) and one medium task (1,
caused by a director rule, not by its tests). Proposed changes 17 and 18 follow from this section.
