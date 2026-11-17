---
iteration: 2                # second stage of iteration 2 (ADR-0007 as amended by D-147), from cut 1 (D-170) to cut 2 (D-216)
run: R-304
methodology_in: v1.4
methodology_proposed: v1.5
main_sha: c45a17f9b6142f422f0ca2eaf8487da3975e22b4   # D-217, the brief's state; cut 2 is 995b176 (D-216), the publish T-084 merged at d600667
claims: {C1: insufficient, C2: supported, C3: insufficient, C4: supported,
         C5: insufficient, C6: refuted, C7: supported, C8: supported}
         # C2 supported for this period only (local gate and every CI run read on main green); INC-001 shows the
         #   earlier "supported" described the local gate, not CI (main red in CI for about 2,249 min);
         # C3 insufficient: one independent-arm task against two implementer-first ones, and no acceptance test caught
         #   a defect at a join in the period;
         # C4 supported with two caveats (the evaluator's clone held the ledger; three findings no review had);
         # C6 refuted as a gate (no latency budget enforced; budgets EMPTY in every gate run read)
---

# Retrospective, iteration 2, stage 2, cut 1 to cut 2

## How to read this report

Labels as in retro-2c1. **VERIFIED**: I ran the command named beside the number and it printed it.
**INFERRED**: my reasoning from verified facts; the reasoning is given. **PROPOSED**: a change for
methodology v1.5, for the director to decide.

Commands, from the repository root (python3 standard library only, read-only; every record is read
with `git show c45a17f:<path>`, so later writes by the running platform task do not move a figure):
- `M <section>` = `python3 -I retrospectives/scripts/iter2s2c2_metrics.py <section>`; sections
  `checks completeness tokens overhead helpers budgets joins queue reviews arms rework time lead gates decisions acks`.
- `G` = `bash retrospectives/scripts/iter2s2c2_git.sh` (range `30917a5^..c45a17f`, D-171 to D-217).
- `P` = `python3 -I retrospectives/scripts/iter2s2c2_touches.py` (`touches` overlap of the platform batch
  and of the feature drafts, at `c45a17f`).
- Earlier periods' figures are retro-2c1's and retro-2s1's (their commands); I did not recompute them.
- Review classes, defect origins and the director-error table are my readings of the review files,
  task files and decision rows (INFERRED classification of VERIFIED rows).

Scope: from D-171 (commit `30917a5`, 2026-11-15T16:15:17Z; the owner's instruction after cut 1) to D-216
(commit `995b176`, 2026-11-16T02:50:49Z; cut 2 reached), plus D-217 (CI read at the pause) and D-218 (the
owner's instruction that follows, for section 11). Runs R-254 to R-303 as the brief names them. Six tasks
merged: T-069 (CI fix, INC-001), T-057 (project panel), T-070 (`platforms.yml`), T-058 (tabs), T-054
(lighter debug profile), T-059 (status bar); one publish (T-084, the evaluation at cut 2).

**Drift.** `main` is now `a574041`, three ledger commits after `c45a17f` (`550e457` D-218,
`b1b9376` T-071 in progress with R-305/R-306, `a574041` state and handoff; `G`). Nothing counted here
uses them, except D-218's text in section 11.

## Data

| Check | Value | Command |
|---|---|---|
| Run records R-254..R-303 | **50**, no id missing: analyst 1, planner 17 (of which 8 are CI reading: 7 helpers booked as `planner` and the resumed planner R-294), implementer 11, test author 9, reviewer 11, evaluator 1; 46 `completed`, 4 `request_changes`, 0 blocked | VERIFIED `M completeness` |
| Runs with tokens unknown | **2**: R-267 (T-070 test author) and R-272 (T-057 implementer); both because the agent's context was compacted during the run and the transcript's running figure fell (D-183; R-272's note: 101,452 at its end against 338,211 at the end of R-265) | VERIFIED `M completeness`; D-183; the R-272 record's note |
| Runs without a log entry | **R-260** (T-069 reviewer, 2,121 tokens: one line of the review file fixed after the queue's `hygiene` failure) has no entry in the task's reviewer log; booked `completed` (§12: no entry ⇒ incomplete). R-282 and R-292 (CI helpers) are named in no task, review, decision or handoff file | VERIFIED `M completeness`, `M acks` |
| Runs with wall < 60 s | R-258 28 s (5,320), R-260 17 s (2,121): plausible. **R-292 15 s / 81,251 tokens, R-299 11 s / 76,281, R-301 19 s / 86,559, R-303 15 s / 89,076** (CI helpers, 4–6 tool calls): the booked times are unlikely to be the runs' whole duration; R-301's first booking was refused for an end time later than now (`0e678ce`), so at least one was typed | VERIFIED `M completeness`, `M helpers`; `git log -S'"run":"R-301"'` |
| Tier field | constant per task in every run record (T-054 high, T-057 medium, T-058 medium, T-059 medium, T-069 low, T-070 high): no re-tier in flight | VERIFIED `M completeness` |
| Transitions | T-057 11, T-058 11, T-059 8, T-069 6, T-070 13, T-054 11, T-084 6; every rework passes `self_verified → tests_joined → reviewed` | VERIFIED `M completeness` |
| Join records | **10** (T-057 2, T-070 3, T-058 2, T-054 2, T-059 1); 9 `joined`, 1 `refused_base_passes`; `acc_defects` null on all 10; no `join_defects` record in the period | VERIFIED `M joins` |
| Queue records | **8**: 7 merged, 1 `gate_failed` (T-069 at `hygiene`). One refusal is in a decision row only (T-069 at `touches`, D-179) | VERIFIED `M queue`; D-179 |
| Reviews | 10 files: T-069 r1 (low, a read of the diff), T-057 r1/r2, T-070 r1/r2, T-058 r1/r2, T-054 r1/r2, T-059 r1 (approve, no round 2); no confirmation record (no branch was rebuilt) | VERIFIED `M reviews` |
| Decision rows D-171..D-217 | 47, none missing; no D-NNN cited without a row | VERIFIED `M decisions` |
| Rulings acknowledged | 30 of 31 builder and reviewer runs by word match; the one without is R-260 (no entry). `M acks` prints "no" for R-286 because the first match is ruling i1's line, which names R-286; its entry reads "Rulings acknowledged and accepted" (`tasks/T-054-*.md`, implementer log) | VERIFIED `M acks`; reading |
| Methodology text changed | once: `aae0197` (v1.4 in force, D-176); `METHODOLOGY.md` 338 → 384 lines | VERIFIED `G` |
| Handoffs | `2026-11-15.md` (addenda 16:20 to 23:22 in this period) and `2026-11-16.md` (01:56, 02:51) | `ls handoffs`; `grep -n '^## ' handoffs/2026-11-1[56].md` |

The director's cost is not measured (METHODOLOGY §13). Time "with no run active" contains the director's
joins, rulings, CI pushes and queue runs.

## Metrics (METHODOLOGY §13)

| Metric | Value | Command |
|---|---|---|
| Lead time `ready` → merged | medium: T-057 **3.12 h**, T-058 **2.49 h**, T-059 **1.58 h** (median 2.49); high: T-070 4.50 h, T-054 2.60 h; low: T-069 1.24 h; publish T-084 0.33 h. Cut 1: high 3.02 / 3.21 / 2.64 h | VERIFIED `M lead`; retro-2c1 |
| First-pass queue success | **6 of 7** by record (T-069 failed at `hygiene` first); by events T-069 needed three attempts (refusal at `touches`, D-179; `gate_failed`; merged). Cut 1: 2 of 4 by record | VERIFIED `M queue`; D-179 |
| Gate attempts per task (Σ run `gate_attempts`) | T-057 4, T-058 4, T-059 2, T-069 2, T-070 3, T-054 2 (median 2.5). Cut 1: median 3 | VERIFIED `M gates` |
| Acceptance-caught defects | **0** at any join (every acceptance test failed on base and passed joined at every `joined` record). The one non-green join (T-070, 19:59:26) was a refusal because 3 tests passed on base (classified count 0, cause `test`/spec, D-189) | VERIFIED `M joins` |
| Review findings, round 1 (T-057, T-058, T-059, T-070, T-054) | defect **11**, spec gap 7, test gap 4, nit 7; `doc_sentences` 10. Round 2 (4 files): 0 defects, 1 test gap, 3 nits, `doc_sentences` 2. T-069's read: 1 nit | VERIFIED `M reviews` |
| — behaviour defects among round-1 defects | **2** (T-058 #1 the `modified` mark wraps; T-070 #1 the procedure's exit rule can be fooled); the other 9 are document sentences. Cut 1: 7 of 10 | INFERRED from the findings tables |
| Escaped defects (found after merge) | NEW-1 → T-085 (`found_by: evaluator`, `origin_task: T-055`); NEW-2 → T-083 (draft from T-059's spec check, `origin_task: T-059`); NEW-3 → T-053 (document sentences of T-057/T-058); the wrong "18.0 s for the expand" in `ui-project.md` (T-057 r2 follow-up, into T-081) | `evaluations/iter-2-cut-2.md`; front matter of T-081, T-083, T-085 |
| Rework ratio | task files first committed in the period with `origin_task`: T-069 (T-036), T-080 (T-019), T-081 (T-057), T-082 (T-058), T-083 (T-059), T-085 (T-055), T-086 (T-065) | front matter; `git log --diff-filter=A` |
| Review rounds per task | T-057 2, T-058 2, T-070 2, T-054 2, T-059 **1** (approve at round 1), T-069 1 (read) | VERIFIED `M reviews` |
| Acceptance kill rate | round 1: **76/97 = 78 %** (any test 91/97 = 94 %; 10 by a procedure, 6 survived); round 2: 26/31 = 84 %. Cut 1: 61 % and 85 % | VERIFIED `M reviews` |
| Known defects merged by decision | 0 behaviour defects open at merge. Limits stated and accepted: the bar not painted at 800x320 from 23 tabs (D-206), per-frame layout cost of large listings (D-188 r4), a line-break name painted on two rows (r6), focus after a tab click (h6, T-082), value collisions with bar values (k1, T-083), dependency frames without lines (D-208) | rows named |
| Tokens per merged task | medium **670,559 + 1 unknown**, **909,094**, **500,778** (median 670,559); high 809,362 + 1 unknown, 455,026; low 346,304. Cut 1 high median 997,800 | VERIFIED `M tokens` |
| Share verified | **47/64 = 73.4 %** on main (D-216); 3 new (`ui.project-panel`, `ui.tabs`, `ui.status-bar`) | `git show c45a17f:docs/scorecard.md` |
| Red-main minutes, per counted platform, from CI runs on main | **Linux (`ci.yml`)**: INC-001 red from 2026-11-14T04:34Z to run 366 at 2026-11-15T18:02:59Z = **2,249 min**, of which the 107 min from D-171 (16:15Z) to run 366 lie in this period; after it 0 red among the 16 runs whose conclusion is recorded (366, 377, 378, 388, 392, 405, 406, 421, 429, 431–436, 438) of runs 366–438. **`window-linux`, `window-probe (macos-15)`** (counted from D-202): 0 red on the 5 runs of `platforms.yml` on main that were read (D-202, D-210, D-213, D-217 ×2). `gate-macos`, `gate-windows`, `window-probe (windows-2025)`: tracking, red on every run | VERIFIED `M checks`; D-179, D-202, D-210, D-213, D-217; `grep -oE 'runs? [0-9]{3}'` over decisions and handoffs |
| Budget overruns | **2 of 6**: T-069 2.31× its low ceiling (346,304 of 150,000; Extend rows D-177 to 300,000 and D-178 to 350,000); T-058 1.14× its medium ceiling (909,094 of 800,000; D-196 to 1,100,000, written before round 1) | VERIFIED `M budgets`; `G` |
| Gate time (`gate_seconds`) | merged queue runs 397.4, 490.1, 803.1 (gate path, main + task `xtask`), 552.0, 984.2 (gate path), 580.1, 475.9; median **552.0 s**; with main's `xtask` only median 490.1 s. Cut 1 median 640.6 s | VERIFIED `M queue` |

## Further numbers

| Number | Value | Command |
|---|---|---|
| Tokens R-254..R-303 | **6,267,529** (2 runs unknown): cut-2 tasks 2,080,431; platform tasks 1,610,692; `iter-2` overhead **2,576,406 = 41.1 %**. R-254..R-302 = 6,178,453 and overhead 2,487,330, exactly D-216's figures; R-303 (89,076) came after D-216 | VERIFIED `M tokens`, `M overhead` |
| — overhead by kind | CI reading **952,269 (15.2 % of all)**; the retrospective to cut 1 410,954; spec checks 409,603 (7 runs); the evaluation 377,955; the platform plan 328,687; the methodology edit 96,938. Without R-254, R-255, R-257: 1,739,827 of 5,430,950 = 32.0 %. Cut 1: 33.8 % (26.5 % without R-223, R-224); stage 1 38.5 % | VERIFIED `M overhead` |
| — by role, task runs | T-057: implementer 338,211 (2, one unknown), reviewer 332,348 (2). T-058: test author 357,466 (2), implementer 275,059 (2), reviewer 276,569 (2). T-059: implementer 262,346 (1), reviewer 238,432 (1). T-070: test author 320,223 (5, one unknown), implementer 218,176 (3), reviewer 270,963 (2). T-054: test author 143,084, implementer 111,870, reviewer 200,072. T-069: implementer 185,749, reviewer 160,555 | VERIFIED `M tokens` |
| Per new verified item (3) | task tokens **693,477** (`ui.project-panel` 670,559, `ui.tabs` 909,094, `ui.status-bar` 500,778); all period runs 2,089,176 (platform work and INC-001 included). Cut 1: 1,025,216 and 1,548,523 | VERIFIED `M tokens` |
| Round-1 review cost | T-057 315,777; T-058 222,976; T-059 238,432; T-070 225,922; T-054 182,805; T-069 read 158,434 (brief asked under 80,000, D-178). Round 2: 16,571; 53,593; 45,041; 17,267 | VERIFIED `M reviews` |
| Post-round-1 rework | **333,305 = 9.0 %** of the six tasks' tokens (T-057 16,571 + R-272 unknown; T-058 125,055, 13.8 %; T-059 0; T-070 152,475, 18.8 %; T-054 37,083, 8.1 %; T-069 2,121). Cut 1 14.9 %, stage 1 8.5 % | VERIFIED `M rework` |
| All run records at `c45a17f` | 36,470,565 over 298 runs (2 unknown) = 30,203,036 to R-253 + 6,267,529 | VERIFIED `M tokens` |
| Builder concurrency | max **4** builder runs open at once: R-277, R-278 (T-070 rework) and R-279, R-280 (T-058) from 21:37:08 to 21:39:28 (140 s); cap 3 (§5.4). Hours at 0/1/2/3/4 builder or reviewer runs: 1.63 / 5.23 / 1.19 / 0.84 / 0.04 | VERIFIED `M checks`, `M time` |

## 1. Did each v1.4 change do what it was for?

| # (CHANGELOG v1.4) | Change | What happened in the period | Verdict |
|---|---|---|---|
| 1 | Tier ≥ items' tiers, fixed at `specified` | T-057 medium (item low), T-058 medium (item medium), T-059 medium (item low), T-054 and T-070 high (gate paths), T-069 low; every run record of a task carries one tier; no re-tier in flight. The tool check (T-043) does not exist | **Worked**, by the director alone (VERIFIED `M completeness`) |
| 2 | Rulings on main; on a branch the director commits only joins and scorecard ties | The only `Role: director` commits reaching main that touch `tasks/` are the publish tool's four T-084 commits; the other director commits are queue records and five scorecard ties (`G`). Queue refusals at `front-matter`: 0; rebuilt branches: 0; queue conflicts: 0 (`M queue`) | **Worked**: the failure class of D-162 did not recur (n = 6 merges) |
| 3 | One role per `touches` path | Every `touches` line of the six tasks names one role; two rulings added a path to one role (m9 `guard_t070.rs` to the test author, h5 `ui.md` to the implementer); 0 queue conflicts. Join check (T-033) not built | **Worked**, by hand |
| 4 | Rebuilt branch confirmed by tree identity (interim: reviewer's record) | No branch was rebuilt; no confirmation record | **Untested** |
| 5 | Text/colour items: differential run on a stated alphabet with shaped text | T-057 r1 (40 trees, stated alphabet, 3,078 entries), T-058 r1 (seeds 1000–1199, ligatures, combining marks, a joiner, right-to-left, CJK, emoji), T-059 r1 (stated alphabet with `e`+U+0301, U+200D, `ffi`, Hebrew, Arabic): 0 mismatches; the evaluator found no shaped-text defect. But D-174's caveat ("the next evaluation re-checks `ui.editor-view` on such text through the snapshot") was met only in part: the evaluator's files have no combining mark at a row's start, the class of cut 1's NEW-1, and colours of marks were excluded (`evaluations/iter-2-cut-2.md` "ui.editor-view") | **Done as written; found nothing**; the case that failed at cut 1 was not re-run (INFERRED from the listed lines) |
| 6 | A figure "to be measured by the implementer" must be measurable by it | T-070's Pinned (i) said "to be measured by the director" (wall times, disk): the director recorded them (D-187, 19:58Z) before the join (20:09Z) but not for the implementer, whose document said "not measured yet" → review defects #3 and #4. T-054's D3 was re-measured by the implementer and ruled to state its own figures (i1) | **Gap**: the rule has no case for a figure only the director can measure |
| 7 | An option called unmeasured is probed before ruling | Four unmeasured options were offered: T-057 Q-1 B, T-054 Q-2 B, T-070 Q-3 A, T-059 Q-5 A. None was probed; each time the director chose the measured option (D-181, D-186, D-180: "reason enough not to probe it", D-206). No ruling was reversed by a review in the period (h3's wording was corrected by the implementer, row 20 of section 7) | **Worked in effect; rule and practice differ** (the text says "probed before") |
| 8 | Window evidence in the gate where it can be | `window-linux` runs the T-065 procedure under Xvfb on every code push (T-070, `.github/scripts/window-procedure.sh`); counted from D-202 and green on every main run read. Not in the local gate (D-180 Q-3 B). T-057 r1 ran the procedure; T-058 r1 could not ("the machine lacks `libxkbcommon-x11.so`", without the unpack recipe); no task changed `window.rs` | **Worked** (on CI) |
| 9 | `killed_by_procedure` counted apart | Present in all 10 review files; T-057 r1 4, T-070 r1 6, r2 4 | **Worked** |
| 10 | A number in a decision row names its source | Every token figure I recomputed matches the records to the token (D-178 344,183; D-179 346,304; D-188 653,988; D-191 670,559; D-196 561,063; D-197 809,362; D-203 855,501; D-204 909,094; D-211 455,026; D-212 500,778; D-216 6,178,453, 2,487,330, 36,381,489; INC-001 2,249 min) (`M checks`, `M tokens`, `M overhead`). Two figures without a source were wrong: D-196 "a first review round has cost 200,000 to 260,000 on the last three UI tasks" (records: 314,479, 300,749, 315,777); D-191 r4 "18.0 s for the expand" (the reviewer's whole-script time, corrected in T-057 r2) | **Worked where a command was named**; the two misses named none |
| 11 | Interim rules written into the text | v1.4 holds them; STATE repeats them without contradiction | **Worked** |
| 12 | Evaluator clone without the ledger records it may not read | R-302's Setup: "Ledger records left out of the clone: none (the clone holds `tasks/`, `reviews/`, `handoffs/`, `metrics/`, `retrospectives/`, `docs/decisions.md`; I did not open them)". The brief forbade reading them but the clone was a plain clone | **Not applied** (VERIFIED `evaluations/iter-2-cut-2.md` "Setup"); whether `cargo xtask scorecard` runs without them was not tried |
| 13 | Queue refusals written to `queue.jsonl` (pending T-043) | T-069's refusal at `touches` is in D-179 only. D-174 (13) decided "T-043 is specified and dispatched in the next batch" and ADR-0009 Q-4 A put T-043 and T-066 in that batch; neither was dispatched and no row changes the decision (`grep -n 'T-043' docs/decisions.md`: last at D-179; D-216's list of follow-ups omits T-043) | **Not delivered; the decision was dropped without a row** |
| 14 | §8.1 Platforms and CI | ADR-0009's first tasks ran: T-069 (INC-001), T-070 (`platforms.yml`), T-054. Two jobs counted from D-202; pre-merge `ci/T-NNN` runs for T-070, T-054, T-058, T-059; claims from CI logs named runs (D-187, D-195, D-205, D-210). Detail in section 5 | **Worked**, at a reading cost of 952,269 tokens |
| 15 | A question the join can answer may be ruled after it | m9 (D-189) was ruled on the join's result and cites it; no builder question was left to the join | **Barely used** |
| 16 | Read CI after every push; red CI on main is an incident | INC-001 closed (D-179) with the first green run read. Afterwards 16 of the 73 `ci.yml` runs 366–438 have a recorded conclusion; D-179 relaxed the rule to "recorded at the next decision row or handoff addendum". The push with T-059's merge (`20f63f4`, run 433) was read at 03:03Z (D-217), after the evaluator was dispatched (01:52Z, R-302) and T-084 was queued (02:42Z) | **Worked for the incident; the "before the next dispatch or queue run" clause was not followed** (VERIFIED: rows, handoff 01:56 "CI still to read") |

## 2. The two arms on comparable tasks

T-058 (tabs, even, independent test author) against T-057 (project panel) and T-059 (status bar), both
odd, implementer-first. All three medium, UI panels over verified commands, one item each.

| Measure | T-057 (impl-first) | T-058 (independent) | T-059 (impl-first) | Command |
|---|---|---|---|---|
| Tokens | 670,559 + R-272 unknown | 909,094 | 500,778 | VERIFIED `M arms` |
| — by role | implementer 338,211 (+unknown), reviewer 332,348 | test author 357,466, implementer 275,059, reviewer 276,569 | implementer 262,346, reviewer 238,432 | VERIFIED `M arms` |
| — up to the first join | 338,211 | 561,063 | 262,346 | VERIFIED `M arms` |
| Joins / review rounds | 2 / 2 | 2 / 2 | 1 / 1 | VERIFIED `M arms` |
| Round-1 findings (d / sg / tg / nit) | 4 / 1 / 1 / 2 (all 4 defects document sentences) | 3 / 2 / 1 / 2 (1 behaviour defect) | 0 / 1 / 0 / 1 | VERIFIED `M reviews` |
| Round-1 live mutants; killed by acceptance tests | 18; **12** (67 %); 4 only by a procedure (2 outside the gate by ruling Q-6, 2 a real test gap) | 19; **18** (95 %) | 18; **18** (100 %) | VERIFIED `M arms` |
| Killed by the implementer's own tests | 12 of 18 (rows with "own" or "unit") | **19 of 19** (M17 only by them) | 13 of 18 (review: "not R9, R10, R14, R15, R19") | INFERRED count of the review tables (`grep -c` on the rows); `reviews/T-059-r1.md` |
| Acceptance-test defects and who found them | 2 (AC-3's generator could make 4 files; AC-2 needed 800x320): found by the implementer after its code, fixed in `1907f10` (18:36:23Z, 2 min after the first code commit) | 2 harness defects (right-to-left `Path` clearing; AC-7(c)'s order) found by the test author against its own stub before hand-over; **0 at the join** | 2 (`ac4c`, `ac4d` log checks): found by the implementer at its first scratch-merge run, fixed in `6132b10` (00:31:19Z); classified by the reviewer by re-running the first version (7 passed, 2 failed) | task logs; `git log` of the acceptance files (author dates) |
| Tests-first order | `eb04f67` 18:31:02Z before `df53890` 18:34:42Z | — | `f676bf2` 00:27:41Z before `a7ad19f` 00:30:29Z | VERIFIED `git log --format=%aI` |
| What the reviewer did extra | read the precondition commit and confirmed no assertion weakened | — | re-ran the first acceptance file against the code to classify the two changes | review files |
| Behaviour defect found later | NEW-3 (shared with T-058), the 18.0 s figure | NEW-3 | NEW-2 (at 300x300, 400x300) | evaluation |

What the record supports (INFERRED):
- The implementer-first arm was cheaper here by one agent: 500,778 and 670,559 against 909,094; the
  test author of T-058 alone spent 357,466, of which a scratch stub and 9 seeded bugs (D-196).
- On mutants the arms did not differ in the direction C3 predicts: the independent acceptance tests
  killed 18 of 19, the implementer's own tests 19 of 19; in T-059 the implementer's acceptance tests
  killed 18 of 18 and its own unit tests 13. No acceptance test of either arm caught a defect at a join.
- The one behaviour defect of the three (T-058 #1, a dirty document with a path wider than the strip)
  passed both the independent tests and the implementer's tests and was found by a reviewer's probe.
- In both implementer-first tasks the acceptance file was changed after the first code commit; the
  changes were fixture and log-check defects, not weakened assertions, by the reviewers' reading. In the
  independent arm the same kind of defect was caught by the test author's stub before the join.
- D-212's "the implementer's own tests missed 5 of the reviewer's 18 killed mutants and the acceptance
  tests it wrote first caught all 18" agrees with `reviews/T-059-r1.md`.

What a fair comparison would still need: tasks of matched size (T-059 has one panel file and no
switching logic; T-058 had close, discard and the `Documents` click); more than one task in the
independent arm; mutants seeded by the same reviewer with the same budget across arms; a mutant count
for the implementer's own tests in the front matter (today it is in table prose); and the
acceptance-test defects recorded as a number per task (today only in prose, D-212 "= 2").

## 3. Joins and queue

| Task | Event (UTC) | Result | Cause |
|---|---|---|---|
| T-069 | queue, before attempt 1 (time not recorded) | **refused at `touches`** (row only, D-179) | a review file on a low-tier branch is outside what the tool allows; added to `touches` on main |
| T-069 | queue 1, 17:45:55 | `gate_failed` at `hygiene` | the review file named the reviewer's scratch directory (R-260 fixed it) |
| T-069 | queue 2, 17:47:09 | merged, 397.4 s | — |
| T-057 | join 19:05:39 | joined, 11 acceptance tests all failing on base | — |
| T-070 | look at `e3b80dd`, about 18:56 (D-183 commit; no join record) | 10 of 11 pass | test fault (m8, D-183) |
| T-070 | join 19:59:26 | **`refused_base_passes`** (8 of 11 fail on base) | the spec gave the test author a deliverable of its own with acceptance tests (m9, D-189) |
| T-070 | join 20:09:16 | joined, 8 + 3 guards | — |
| T-057 | join 20:27:12 | joined, 12 | — |
| T-057 | queue 1, 21:16:33 | merged, 490.1 s | — |
| T-070 | join 21:46:12 | joined, 13 + 7 guards | — |
| T-058 | join 22:15:01 | joined, 12 | — |
| T-070 | queue 1, 22:30:38 | merged, 803.1 s (gate path) | — |
| T-058 | join 23:19:38 | joined, 14 | — |
| T-054 | join 23:35:46 | joined, 5 + 2 guards | — |
| T-058 | queue 1, 23:56:23 | merged, 552.0 s | — |
| T-054 | join 00:39:31 | joined, 7 + 2 guards | — |
| T-059 | join 00:55:43 | joined, 9 | — |
| T-054 | queue 1, 01:12:07 | merged, 984.2 s (gate path) | — |
| T-059 | queue 1, 01:41:50 | merged, 580.1 s | — |
| T-084 | queue 1, 02:42:18 | merged, 475.9 s | — |

VERIFIED `M joins`, `M queue`; causes from D-179, D-183, D-189.

**Green at the first attempt.** Joins: 4 of 5 tasks (T-070's first join refused); all 10 join records
had every acceptance test failing on base by assertion except the refused one; 0 acceptance failures
against an implementation. Queue: 6 of 7 by record, 5 of 6 product and platform tasks by events (T-069's
two failures). Against cut 1: joins 2 of 3 tasks green at the first join (T-055's failed, 2 of 10 records
not joined), queue 2 of 4 by record and 4 failures in all, every one from a director's ruling or commit
(retro-2c1 section 2). Here 1 of the 2 queue failures came from the spec (the `touches` refusal) and 1
from a reviewer (scratch path in the review file). VERIFIED counts; INFERRED attribution.

## 4. Reviews

| Review | Verdict | d | sg | tg | nit | doc | mutants in/eq/killed/by acc/by proc/surv | tokens |
|---|---|---|---|---|---|---|---|---|
| T-069 r1 (R-259, low read) | approve | 0 | 0 | 0 | 1 | 0 | 2/0/2/2/0/0 | 158,434 |
| T-057 r1 (R-269) | request-changes | 4 | 1 | 1 | 2 | 4 | 19/1/18/12/4/0 | 315,777 |
| T-057 r2 (R-275) | approve | 0 | 0 | 0 | 1 | 1 | 2/0/2/2/0/0 | 16,571 |
| T-070 r1 (R-274) | request-changes | 4 | 2 | 1 | 1 | 3 | 20/0/18/12/6/2 | 225,922 |
| T-070 r2 (R-281) | approve | 0 | 0 | 1 | 2 | 1 | 14/0/14/10/4/0 | 45,041 |
| T-058 r1 (R-283) | request-changes | 3 | 2 | 1 | 2 | 3 | 19/0/19/18/0/0 | 222,976 |
| T-058 r2 (R-290) | approve | 0 | 0 | 0 | 0 | 0 | 7/2/5/4/0/0 | 53,593 |
| T-054 r1 (R-291) | request-changes | 0 | 1 | 1 | 1 | 0 | 23/1/18/16/0/4 | 182,805 |
| T-054 r2 (R-298) | approve | 0 | 0 | 0 | 0 | 0 | 10/0/10/10/0/0 | 17,267 |
| T-059 r1 (R-300) | approve | 0 | 1 | 0 | 1 | 0 | 20/2/18/18/0/0 | 238,432 |

VERIFIED `M reviews`.

**Where each round-1 defect came from** (INFERRED from findings, the specs and the rulings):

| Defect | Kind | Origin |
|---|---|---|
| T-058 #1 `modified` mark wraps over a wide tab | behaviour | implementation (`tabs.rs` lays the mark out without a wrap mode); no test had a dirty long path (test gap #2) |
| T-070 #1 the procedure exits 0 when `kiln` prints a pass line | behaviour | the spec's exit rule ("exits 0 only if it printed `WINDOW CASES PASS` and `PLAIN PASS` and …", Interface (3)) carried into the wrapper, with R-251's lines echoing `kiln`'s stderr unindented |
| T-057 #2 where nested `truncated` markers go | document | implementation sentence; ruling v1 settled the behaviour "as built" without asking for the sentence |
| T-057 #3 the sentence of v3 missing | document | ruling v3 scheduled it "in the rework after review" (D-184), so round 1 had to count its absence |
| T-057 #4 the cost of a large listing hides the per-frame layout | document | the spec's Pinned figure (0.5 s for the read, `tasks/T-057-*.md:274`), copied |
| T-057 #5 "first 37 chars" | document | the implementer's own one-name measurement |
| T-058 #3 "a change is a request other than a read" | document | **the spec's Interface wording** (`tasks/T-058-*.md:119`), copied; corrected by h3 |
| T-058 #4 "`Discard` stays until closed or saved" | document | **the spec's Pinned T3** (`:242`), copied; corrected by h4 |
| T-070 #2 the Disk step "can neither hide a failure nor cause one" | document | implementation (the step lacked `set +e`) |
| T-070 #3 "not measured yet" | document | Pinned (i) "to be measured by the director"; the director's figures (D-187) were not given to the implementer |
| T-070 #4 "14 GB disk each" | document | **the spec's Pinned (d)**, marked KNOWN, not verified; the run showed 145G/150G/320Gi volumes |

Tally: behaviour 2 (implementation 1, spec wording 1); document 9 (spec 4, implementation 3, a ruling or
the director's step 2). None lies in L0–L4.

**Pattern A: a criterion worded as a list of forbidden (or required) things lets the unlisted through.**
- T-054 AC-4 named `release`, `bench` and three keys as absent; four non-equivalent mutants passed every
  gate test (`[profile.test] debug = true`, which undoes the task for the gate's `test` step;
  `split-debuginfo`, `codegen-units`, `build-override`). Cost: ruling j1, R-297 19,816 + R-298 17,267 =
  37,083 tokens, 27 min from round 1's end (00:31:01) to round 2's end (00:58:01), one extra join
  (VERIFIED `M rework`, `M reviews`; `reviews/T-054-r1.md`).
- T-070 finding 5: four pinned workflow properties outside the tests' enumerated checks (M9 to M12), and
  finding 1: the exit rule enumerated the pass lines and did not forbid FAIL lines. Cost: shares of R-278
  72,016 and R-277 35,418.
- What closed it: "the `[profile.*]` tables are exactly the Interface's" (j1) and "exits non-zero when any
  FAIL or NOT RUN verdict was printed" (g1): equality or a closed rule instead of a list. In both cases the
  reviewer's mutation spot-check found the gap; the spec checks (R-284, R-261) did not.
- Twice in the period, both in high-tier gate-path tasks; retro-2c1 names no case of this kind.

**Pattern B: a document sentence copied from the spec is wrong against the binary.**
- T-058 #3 and #4 (h3, h4), T-057 #4 (the read cost), T-070 #4 (runner disk): **4 of the 9** document
  defects. Cut 1 had the same kind: T-055 r1 #2 ("every interactive node has a non-empty label"), T-065 r1
  #5 and the 1280×800 sentence (retro-2c1 section 3).
- Cost: each a sentence in the rework and a ruling; T-058's rework (125,055 tokens) also carried the
  behaviour defect, so the share is not separable.
- Why the spec check did not catch them (INFERRED): v1.3 change 10 asks evidence for Pinned sentences
  "that state a figure, a limit or a list"; T3 and the change definition are behaviour statements, so no
  evidence was required. R-276 built a prototype of both panels and ran the merged tests against it
  (110 passed) but did not run the Pinned sentences against it (D-201: "the planner's prototype did not
  carry these two sentences into a check against the binary"). T-070's "14 GB" was marked KNOWN-not-
  verified and went into the document anyway.
- What would close it (PROPOSED, change 4): every Pinned (i) sentence carries the command on the
  prototype or the base binary that shows it, or is marked "unchecked" and is the reviewer's first check;
  a KNOWN-not-verified fact may not be copied into a document.

## 5. Platforms and CI as evidence

**What `platforms.yml` shows and does not show** (VERIFIED from `tasks/ci-runs/*.md` and `G`):
- Shown in job logs: each gate step's PASS/FAIL line and the `GATE PASSED|FAILED` line; the core-crates
  exit status per package; the disk lines; the window procedure's verdicts; the probe's exit status.
- Not shown in job logs:
  - names of passing tests (no line names `guard_t054` or `acceptance_t054` in any of the three gate
    logs of run 43775682214; `T-054-round-1.md:157`);
  - the panic message of a failing test on Linux (the negative candidate's sentence is in none of the
    400–800-line tails read; `T-070-negative.md:46`, `:157`);
  - anything after the first failing test binary in the gate's `test` step, which stops at the first
    failing suite (`T-070-round-1.md:270`; `xtask/src` has no `--no-fail-fast`, `G`);
  - anything after the first failing `xtask` binary in the core-crates step (`cargo test --locked -p "$p"`,
    no `--no-fail-fast`, `platforms.yml:98`; `T-054-round-1.md:82`: one `test result:` line for `xtask` on
    macOS, so the T-054 guards did not run there in that step);
  - more than 200 lines of the full test output (`grep … | head -200`, `platforms.yml:113`): D-187's
    "366 passed, 11 failed" was that prefix; the artefact holds 977 passed, 22 failed (R-285, D-199).
- The artefacts hold the full test output and the gate report: from them R-285 classified the 22 macOS
  failures (none a product fault by their panic messages) and R-294 read T-054's guard verdicts.

**What reading CI cost.** Eight runs booked to CI reading: **952,269 tokens, 15.2 % of the period**
(R-271 190,063; R-282 243,645; R-292 81,251; R-293 169,441; R-294 15,953; R-299 76,281; R-301 86,559; R-303
89,076). For the same run 43775682214 the helper reading job logs spent 169,441 and still could not name
the guard verdicts; the planner reading the artefacts spent 15,953 and could (10.6×) (VERIFIED `M helpers`;
D-205). INFERRED: the cost is in logs passing through an agent's context. It could be cut by having a tool
extract a fixed record from the artefacts (gate report JSON and the full test output) and by making the
evidence steps run every test binary; PROPOSED change 1.

**"A job counts from its first green run on main" in practice.** The first run of `platforms.yml` on main
(43770955835, D-202) was also the first green run of `window-linux` and `window-probe (macos-15)`, so both
counted at once; both stayed green on every main run read (D-210, D-213, D-217). The three gate and probe
jobs that do not build or pass are tracking; their red is recorded, and filed as T-071 to T-074, not as
incidents. The rule worked as written. Its consequence: once `gate-macos` turns green (T-072), every later
task touching a platform-sensitive path waits for a macOS gate run before its queue (21 min 20 s to 26 min
30 s per run in this period: D-187, D-202; INFERRED).

**Pre-merge `ci/T-NNN` runs.**

| Task | Run | Pushed → read | What it showed | Helper cost |
|---|---|---|---|---|
| T-070 round 1 | 43752760283 | 19:30:06 → 19:58 (R-271) | the workflow's own criteria AC-4, AC-5; macOS window opens | 190,063 |
| T-070 negative + round 2 | 43764233597, 43765532298 | 21:32:20 / 21:46:25 → 22:14 (R-282) | AC-6 as ruled g5 (test named, message not); AC-4 again | 243,645 |
| T-054 | 43775682214 | 23:36 → 00:04 (R-293, R-294) | guards pass on Linux and macOS (artefacts); Windows not compiled | 185,394 |
| T-058 | 43777023951 | 23:51 → 23:56 (R-292); queue 23:56:23 | the two counted jobs green; gates still running, not counted | 81,251 |
| T-059 | 43787215183 | 01:36 → 01:41 (R-301); queue 01:41:50 | the two counted jobs green | 86,559 |

VERIFIED from D-185, D-187, D-193, D-195, D-204, D-205, D-212, D-213 and `M helpers`. For the two cut-2
product tasks the pre-merge run added about 5–6 minutes before the queue and 81–87k tokens each and showed
nothing the local gate had not (INFERRED: both green; no counted job failed on any candidate). For T-054
it gave the only macOS evidence of its guard.

**Temporary branches.** Creating `ci/…` was allowed, deleting refused with HTTP 403 (D-198); D-185's
condition ("deleted when its task has merged") could not be met. Five branches stay (`ci/T-070`,
`ci/T-070-negative`, `ci/T-054`, `ci/T-058`, `ci/T-059`; STATE); the owner allowed them to stay (D-207).
Nothing runs on them unless pushed. The batch of section 11 will add one per task.

## 6. The evaluation at cut 2 against the reviews

R-302 (377,955 tokens) confirmed `ui.project-panel`, `ui.tabs`, `ui.status-bar` (the bar from 480x320 up),
`ui.window`, `ui.editor-view`, `ui.dispatch-only` and eight regression items; 0 rejected; 3 findings and 1
question (`evaluations/iter-2-cut-2.md`).

| Finding | Did a review see it? | Why the reviews did not (INFERRED) |
|---|---|---|
| NEW-1 a `click` right after a layout-changing control request is placed from the previous frame: `ok:true`, nothing happens (T-085) | No | The harnesses put a frame between a control request and a UI request by convention: T-059 r1's generator "Each control request is followed by exactly one frame"; T-058's test author "after other control requests the tests run `frames` 10 before looking". The case was excluded by the instrument's users, not tested. Origin suspected in T-055's runner |
| NEW-2 at 300x300 and 400x300 the bar's text is painted inside `Editor`'s rows; at 200x200/300x200 `Editor` offscreen (T-083) | Partly: T-059 r1 nit k2 saw no bar rows at 1280x300, 400x300, 200x300 and called the limit "incomplete, not wrong"; R-295 measured only 800x320 | The reviewer checked the bar's nodes' rows ("values only, rows not required" at 800x320 and 300x900), not the editor's rows; F2 ("rows equal the text each node shows") is asserted only at the default size |
| NEW-3 `ui.md` "The shell" narrower than the behaviour for `Message` (T-053) | Partly: T-058 r1 #5 widened the sentence for a refused close (h5) | `ui.md` was outside T-057's `touches`; the reviewer's document check (§7) reads sentences "naming the same command, option or kind", and a refused save or expand is not T-058's |
| Q1 exit status 5 once in 127 window runs (T-086) | Yes, earlier: T-070's test author saw `exit window=5` in 1 of 8 runs and inferred `xvfb-run`'s cleanup status (R-264 Q-b; ruling m2, D-182) | T-086 was filed "so that a second sighting has a place"; it does not cite the first |

Confirmed what reviews had shown: the panel, tabs and bar against `project.tree`, `doc.list`,
`cursor.get`, `doc.info` (590 sessions, 22,000 steps, 0 mismatches where painted); logs replayed with
`kiln-cli run`; window against headless (397 scripts, 73,477 lines, 0 output differences); the limits of
D-206 (22 painted, 23 not, at 800x320) reproduced. **Not run**: macOS and Windows (CI is their evidence);
a release build; an ordinary non-root identity (DAC capabilities dropped instead); keyboard navigation,
IME, drag, double click.

## 7. The director as a failure source

Every director error or correction I found in the period ("reached" = a builder, reviewer or evaluator
acted on it before it was corrected; costs from `M` unless said):

| # | Error or correction | Cause | Reached | Caught by | Measured cost |
|---|---|---|---|---|---|
| 1 | D-175 recorded the planner's inferred cause of INC-001 and put it in the incident text the implementer had to copy "exactly" | ruling (unmeasured cause) | implementer R-256 | implementer (QUESTION; D-177 c1) | R-258 5,320 |
| 2 | T-069 low tier with a reviewer's read, but the review file not in `touches` (the tool refuses it on a low branch) | spec accepted with an error | reviewer R-259 | tool (queue `touches`) | 1 refusal, recorded in D-179 only |
| 3 | T-069's transition `self_verified → merged` refused by `metrics` | bookkeeping | — | tool | a second commit |
| 4 | D-177's row committed at 17:35:58, after R-258 started (17:35:30); R-256 had passed the ceiling at 17:34:57 | order (§11) | — | not caught | none |
| 5 | T-069 estimated 120,000 + 30,000; spent 346,304 (2.31×); R-259 asked "under 80,000", spent 158,434, passing D-177's 300,000 | estimate | reviewer | director (D-178) | two Extend rows |
| 6 | T-070's spec gave the test author its own deliverable with acceptance tests | spec accepted with an error | test author R-264 | tool (join `refused_base_passes`) | R-273 10,738; 9.8 min |
| 7 | T-070 Interface (3): an enumerated pass rule for the procedure | spec | test author | reviewer R-274 #1 | share of R-278 72,016 |
| 8 | T-070 Pinned (d) "14 GB disk each" (KNOWN, not verified) | spec | implementer | reviewer #4 | share of R-277 35,418 |
| 9 | T-070 Pinned (i) "to be measured by the director": D-187's figures not given to the implementer before the review | step omitted | implementer | reviewer #3 | share of R-277 |
| 10 | T-070 AC-6 asked for the panic in Windows' `test-output.txt`, impossible before T-073 | spec | reviewer | reviewer #6 (g5) | a ruling |
| 11 | m1 fixed the core-crates step's form; the test's reader counted an `echo` (m8) | correction (test fault found by the director's merged look) | test author | director (D-183) | R-268 14,098 |
| 12 | D-182 disk full: three builders and a planner's prototype build in a ~40 GB allowance, 12 GB per UI build | operation (dispatch) | test author R-264 (3 failures in a gate attempt, a damaged transcript line), implementer R-265 (about 10 min waiting, a bus error) | test author | one unusable gate attempt; ~10 min |
| 13 | Four builder runs open 21:37:08–21:39:28 (cap 3, §5.4) | operation | — | not caught | none measured |
| 14 | D-188 r4 quoted "18.0 s for the expand" (the time of a whole script) into the document | ruling | implementer R-272 | reviewer R-275 | the wrong figure is on main (`ui-project.md`), into T-081 |
| 15 | D-192: "no ruling of the director corrected by a builder or reviewer" (row 14 contradicts it) | bookkeeping | — | not caught | — |
| 16 | T-057's Pinned cost sentence (0.5 s for the read; the per-frame layout not measured) | spec | implementer | reviewer #4 | share of R-272 (unknown) |
| 17 | v3 deferred its sentence to "the rework after review", so round 1 counted it as a defect | ruling timing | — | reviewer #3 | one finding |
| 18 | D-196: "a first review round has cost 200,000 to 260,000 on the last three UI tasks" (records 300,749–315,777; no source named) | bookkeeping | — | not caught | none (the conclusion holds) |
| 19 | T-058's Interface "other than a read" and Pinned T3 wrong against the binary (prototype not run on them) | spec accepted with an error | implementer | reviewer #3, #4 | share of 125,055 |
| 20 | D-201 h3 worded "the panels read again" (the project panel does not) | ruling | implementer R-288 | implementer (D-203) | none |
| 21 | T-054's dispatch named 9 GB free for a measured build (AC-3: 14 GB) | operation | implementer | director (i1, D-203) | none (measured at 20 GB) |
| 22 | T-054 AC-4 as a list of absent tables and keys | spec | test author R-287 | reviewer R-291 | 37,083; 27 min |
| 23 | D-205 disk full: the director's `scorecard --check` without the interim variables during the reviewer's 7.8 GB base build | operation | reviewer R-291 (re-ran a base run) | director | one re-run inside 182,805 |
| 24 | R-297's booking refused (end before a transition); the `… \| tail -1 && …` chain masked the exit status and the commit went on (§2.1 "one command per line") | bookkeeping | — | director (`b2eb13d`) | one commit |
| 25 | R-301's booking refused (end time later than now, typed by hand; §2.1 "never memory") | bookkeeping | — | director (`0e678ce`) | one commit |
| 26 | Run 433 (the push with T-059's merge) read after R-302's dispatch and T-084's queue | rule not followed (§8) | — | not caught | none (green) |
| 27 | T-043 and T-066 decided for "the next batch" (D-174 (13), D-175 Q-4 A); neither dispatched, no row changes it | decision not carried | — | not caught | `queue.jsonl` still lacks refusals |
| 28 | R-302's clone held every ledger record (v1.4 change 12) | step omitted | evaluator | evaluator (disclosed) | none |
| 29 | T-086 filed as a first sighting; R-264 Q-b and D-182 m2 record an earlier one with a cause | bookkeeping | — | not caught | a draft without its evidence |
| 30 | R-260 booked `completed` with no log entry (§12) | bookkeeping | — | not caught | — |
| 31 | D-187's macOS reading from a 200-line prefix ("366 passed, 11 failed"; "five no-display tests time out because a window opens or the binary hangs") | reading of partial evidence (stated as a prefix) | — | planner R-285 (artefact: 977/22; 5 assert that no window opens without `DISPLAY`) | none |
| 32 | D-185 assumed the `ci/` branches could be deleted; refused with 403 (D-198) | operation | — | tool | an owner question; 5 branches stay |

Tallies (VERIFIED by counting the table): **32** rows, of which 2 are corrections rather than errors (#11,
#31). **18 reached a builder, reviewer or evaluator** (#1, 2, 5, 6, 7, 8, 9, 10, 11, 12, 14, 16, 19, 20, 21,
22, 23, 28). Caught by: reviewer 9, director 6, tool 4, implementer 2, test author 1, planner 1, evaluator
1, nobody 8. By cause: spec accepted with an error 8 (#2, 6, 7, 8, 10, 16, 19, 22), ruling or ruling
timing 5 (#1, 11, 14, 17, 20), operation, order or step omitted 9 (#4, 9, 12, 13, 21, 23, 26, 28, 32),
bookkeeping 7 (#3, 15, 18, 24, 25, 29, 30), estimate 1 (#5), a decision not carried 1 (#27), a reading of
partial evidence 1 (#31).

Against earlier periods (INFERRED): cut 1 21 rows (9 reached) over 3 product tasks and a publish; stage 1
24 (10) over 10 tasks; iteration 1 40 (19) over 12. Here 32 (18) over 6 tasks and a publish, about 5.3 per
task against about 7 in cut 1. The kind moved: in cut 1, 4 of 4 queue failures and 3 of 7 behaviour defects
came from rulings or commits; here no queue failure came from a ruling or a director commit (the one
refusal came from the spec), and the largest group of costs is **spec sentences accepted with an error**
(8 rows, 4 of the 9 document defects, 1 of the 2 behaviour defects, the refused join). Rulings corrected by
an agent: cut 1 had two reversed by reviews with behaviour defects (Q-12 A, n11); here three were corrected
without a behaviour defect (D-175's cause by the implementer, h3's wording by the implementer, r4's figure by
the reviewer). Caveat: I count
more systematically than retro-2s1 did, and rows 24–30 come from recomputing records.

## 8. Cost

- Per task, role and item: Metrics and Further numbers (VERIFIED `M tokens`, `M arms`).
- Overhead booked to `iter-2`: 2,576,406 = 41.1 % of R-254..R-303 (32.0 % without the retrospective to cut
  1, the platform plan and the methodology edit). Its largest part is **CI reading, 952,269**, larger than
  the spec checks (409,603 over 7 runs, 58,515 per run against 432,716 over 3 in cut 1) and the evaluation
  (377,955) (VERIFIED `M overhead`). All seven spec checks were done by a resumed drafting planner (R-261, R-262, R-270, R-276, R-284,
  R-285, R-295; handoffs and rows) and cost 26,752–103,198 each.
- Budgets against spend (VERIFIED `M budgets`): T-057 81.7 % at round-1 end, 0.84× at merge (+ R-272
  unknown); T-058 98.0 % at round-1 end, 1.14× (Extend D-196 before round 1, by v1.4's rule, 82.6 % of the
  extension); T-059 0.63×; T-070 0.74× (+ R-267 unknown); T-054 0.41×; T-069 2.31× (two Extend rows; the
  second written after R-259 had passed the first).
- Runs with tokens unknown: R-267 and R-272 (context compaction, the transcript's running figure falls).
  The queue's `budget` check therefore used figures low by an unknown amount for T-057 and T-070. Neither
  task is near its ceiling by any plausible amount (INFERRED: R-272 had 22 tool calls in 653 s; R-278, a
  comparable rework, 72,016).
- D-216's "2,487,330 of planning, spec checks, the retrospective to cut 1, the methodology edit to v1.4, CI
  reading and this evaluation" is exact for R-254..R-302 (`M overhead`).
- Against cut 1 and stage 1 (INFERRED): per new item 693,477 task tokens against 1,025,216 (cut 1) and
  225,783 (stage 1, non-UI); medium median 670,559 against cut 1's high median 997,800; post-round-1
  rework 9.0 % against 14.9 %; overhead share 41.1 % against 33.8 %, driven by CI reading, which did not
  exist before.

## 9. Throughput and environment

| Number | Value | Command |
|---|---|---|
| D-171 → D-216 | **10.59 h** (16:15:17Z → 02:50:49Z) | VERIFIED `M time` |
| Time with no run active | **2.13 h (20 %)** by the records, in 17 gaps (six ≥ 10 min: 17:46–18:04 after T-069's merge, 19:03–19:20, 22:20–22:30, 00:58–01:11, 01:40–01:52, 02:25–02:50 the publish). The usage-limit stop is inside R-274's record (20:24:01–21:31:56), so it is not counted; with its 37 min counted: 2.74 h (26 %). Cut 1: 41 %; stage 1 22 % net of stops | VERIFIED `M time` |
| Usage-limit stops and restarts | one: work stopped about 20:35Z, reset 21:10Z, machine restarted 21:11:33Z, owner asked to continue 21:12Z; one uncommitted `metrics/tasks.jsonl` line recorded, R-274 resumed with a restart notice, nothing lost (D-190). After the pause: restart 2026-11-17T23:02:26Z with nothing in flight (D-218) | D-190, D-218 |
| Disk before T-054 | a UI worktree's `target/` 12 GB, a join's base build 7.9 GB (D-182, handoff 19:22); disk full at about 18:45Z (D-182) and 00:05Z (D-205); with the interim variables a task tree 4.8 GB, tests tree 3.5 GB (D-189) | rows |
| Disk after T-054 | empty-`target/` test build 7.8 → **2.6 GB**; `target/` after a whole gate 3.8 GB (R-286, re-measured by R-291); CI `gate-linux` `target/` 8.6G → 4.2G (D-205); free disk 22 GB at 01:52Z (handoff) | T-054 log; D-205; handoff 01:56 |
| Gate time before and after T-054 | queue, main's `xtask`: 397.4 s (T-069, defaults), 490.1 s (T-057) and 552.0 s (T-058) with the interim variables, then 580.1 s (T-059) and 475.9 s (T-084) with the profile; T-054's own 984.2 s (gate path, run twice). Implementer gates of T-054: 644.9 s from an empty `target/`, 431.5 s second. No reduction in gate time is visible; the profile reduced disk, not time (INFERRED; machine load varies) | VERIFIED `M queue`; T-054 log |
| Builder concurrency | max 4 (140 s), otherwise ≤ 3 | VERIFIED `M checks` |

INFERRED: the period ran two tracks side by side (platform and cut 2), which cut idle time from 41 % to
20 % and put four builders in flight once. The binding limits were disk until T-054 merged (two disk-full
events, one gate attempt lost, joins held for disk: D-184 "T-070's join follows when disk allows") and CI
wall time for the platform tasks (T-070 lead 4.50 h with two CI rounds of about 28 min push-to-read each).

## 10. Claims

| Claim | Verdict | Numbers that decide it | What would change the verdict |
|---|---|---|---|
| C1 contracts and checks first | **insufficient** | 7 spec-check runs, 409,603 tokens, 0 blocked builder runs; the checks found merged faults before dispatch (F1 scroll bounds in T-057; the `Ln`/`Col` collision and the small-window limit in T-059's R-295); they missed the own-deliverable tests (refused join), T-054's list criterion and four copied spec sentences; no unchecked arm | A comparison of checked and unchecked specs, or of checks that run every Pinned sentence against a prototype and checks that do not |
| C2 serial queue keeps main green | **supported for this period** | 7 merges; 6 of 7 first-attempt by record; Linux CI on main green on every run whose result is recorded after run 366; the two counted platform jobs green on every main run read. Against the earlier verdict: INC-001 (2,249 red minutes in CI while the local gate was green) shows retro-2c1's "supported" described the local gate only | A red counted job on main; or a platform-only failure merged green by the local gate |
| C3 another agent's tests catch what the implementer's miss | **insufficient** | 10 joins, 0 acceptance failures against an implementation; in the one independent-arm task the implementer's own tests killed 19 of 19 mutants and the independent tests 18 of 19; both behaviour defects of the period were found by reviewers; n = 1 against 2 | A seeded trial of both arms on matched tasks; an acceptance-caught defect no review saw |
| C4 status from behaviour tests plus evaluator | **supported** | 3 implemented → 3 confirmed; 11 regression items confirmed; 0 rejected; the evaluator found NEW-1 and NEW-2, which no review had. Caveats: the clone held the ledger (change 12 not applied); `ui.status-bar` is confirmed "from 480x320 up" while its document says each node paints its value on one row (NEW-2) | A confirmed item later found wrong in what its title claims |
| C5 risk tiers place review effort | **insufficient** | first medium reviews of stage 2: round 1 cost medium 315,777 / 222,976 / 238,432, high 225,922 / 182,805, low read 158,434; defects medium 7 (1 behaviour), high 4 (1 behaviour); effort did not follow tier | Several reviewed tasks per tier of comparable size |
| C6 budgets that gate | **refuted** as a gate | latency: `fuzz` and `budgets` EMPTY in every gate summary read (`T-054-round-1.md`, the implementers' logs); tokens: 2 overruns, each with an Extend row, one of them overrun again | An enforced latency budget (T-028) |
| C7 repository is the memory | **supported** | a usage-limit stop and a restart mid-review with nothing lost (D-190); the pause and the restart after it resumed from the repository (D-218); 49 of 50 runs with a log entry; every token figure of D-216 reproduces. Defects: R-260, two unknown-token runs, two booking slips, a decision dropped without a row (T-043, T-066) | A loss of state, or a resume that needed what only a session held |
| C8 constructed oracles define correct | **supported** | the reviewers' and the evaluator's naive models against `project.tree`, `git ls-files`, `doc.list`, `cursor.get`, `doc.info` (0 mismatches where painted), window against `kiln script`; the instrument itself was wrong again (NEW-1, a T-055 runner fault), and NEW-2 needed the editor's rows, not the bar's, as oracle | A UI item whose correct behaviour no oracle states |

## 11. The owner's new instruction (D-218): what the methodology lacks for what comes next

D-218: "continue with 1, and then 2": first the platform track (T-071, T-079, T-072 to T-078), then the 17
planned feature items; follow-ups stay drafts unless needed. Findings from the record only; not a design.

**(a) Tasks whose main evidence is a CI run on machines no agent can reach.**
- The builder's loop (§10) has no gate attempt on the target platform: a builder of T-072 to T-077 can
  only write a `CI-REQUEST` and wait for the director's record. The `CI-REQUEST` protocol, the round, and
  "the director pushes the exact candidate and records per job" exist only in T-070's spec ("What the
  director must push and read"), not in §8.1 (VERIFIED: `grep -c CI-REQUEST methodology/METHODOLOGY.md` = 0;
  `P`: no later platform spec mentions it).
- The stop rule "3 consecutive gate attempts fail on the same failure" (§10) does not say whether a CI
  round counts as a gate attempt. T-070 needed two rounds plus a negative run; ADR-0009 expected 2–3.
- The join's "fails on base" rule (§6) is met in these tasks by structural proxies: T-070's tests read the
  workflow as text, T-079's AC-1 to AC-4 scan source files with `syn`, and the behaviour (tests pass on
  macOS) is AC-6 "through CI". The methodology does not say that the join may be green while the main
  criterion is still unknown, nor what record the queue checks for it (ADR-0009 Decision 5's queue check
  is PROPOSED for T-033).
- What a CI record contains is task-log prose and `tasks/ci-runs/*.md` extractions written by helpers at
  81k–244k tokens each; §13 has no CI field; the logs omit what the platform tasks most need (failing
  tests' messages, every binary after the first failing one, everything after 200 lines) (section 5).
- Wall time: a macOS gate job ran 21–26 min; push-to-read was about 28 min for full rounds. Nine tasks with
  2–3 rounds each are hours of waiting the plan does not budget (INFERRED from the rows above).
- Platform claims by the evaluator (§9: "only from a CI run at the evaluated commit") and by reviewers
  ("reads the run's job logs") rest on the same logs; the reviewer of T-070 had "no Actions access here,
  so I read the record and the extraction, not raw job logs" (`reviews/T-070-r1.md`, Not checked): the
  rule asks reviewers for something the setup does not give them.

**(b) A batch of nine tasks that edit overlapping test files and workflow files.**
- 19 of the 36 pairs of T-071, T-079, T-072 to T-078 declare overlapping `touches`;
  `crates/kiln-workspace/src/disk.rs` is in five tasks, `xtask/platform-tests.toml` in three, T-079 and
  T-076 share 17 test files, T-079 and T-075 13, T-077 and T-078 share `platforms.yml` (VERIFIED `P`).
  §5.2 forbids such tasks in flight together, so the batch is in practice one serial chain; the
  methodology has no notion of a planned chain whose later specs go stale as each earlier task merges.
- §5.7 requires a spec check only for a spec written before its `depends_on` merged; the drafts T-072 to
  T-078 were written by R-255 before T-070 merged, so each needs one; but a spec can also go stale through
  a task that is not in its `depends_on` and edits the same files (T-059's check after T-058 found the
  `Ln`/`Col` collision and h3's rule, R-295). The rule does not name that case.
- Tier and gate paths: T-075 and T-076 are drafted low (ruling Q-8 A) while T-075 lists
  `xtask/platform-tests.toml`, a gate path, which makes a task high (§4); D-180 notes it is "corrected at
  its own specification" (VERIFIED `P`). The methodology has no rule for a registry edit that by design
  should not need a high review.
- `ci/T-NNN` branches cannot be deleted (D-198); the reuse rule is in a decision row and STATE only.

**(c) The feature items after it.**
- The 17 planned items are claimed by nine drafts: T-022 (2), T-024 (3), T-025 (2), T-026 (3), T-027 (1),
  T-028 (1), T-031 (3), T-062 (1), T-063 (1). All were last edited between 2026-11-13T21:26Z and
  2026-11-15T00:32Z, before v1.4, ADR-0009 and the three cut-2 panels (VERIFIED `P`).
- Stale against main by the record: budgets of older versions (T-022, T-025, T-026, T-031 500,000; T-024,
  T-027, T-028 850,000, "v1.1"), against today's 800,000 and 1,100,000 (§11 says they are restated at
  specification); T-024's Pinned (d) "Unix; tool from `PATH`" against ADR-0009's three platforms;
  T-062/T-063 add "one line in the shell" that T-058 changed and T-082/T-083 are drafted to change; six of
  the nine list `docs/commands.json` (generated), so they cannot be in flight together under §5.2; chains
  T-024 → T-026 → T-031 and T-022 → T-062, T-024 → T-063 (VERIFIED `P`).
- What the methodology does not say: whether a draft written under v1.2/v1.3 is re-specified in full or
  only spec-checked; that its tests must follow §8.1's platform-test rule from the start; and that each
  feature task touching `apps/` or `crates/kiln-workspace/` will need a pre-merge `ci/T-NNN` run on every
  counted platform, which after T-072 includes a macOS gate of about 25 minutes.

## 12. Where the methodology was wrong or silent, and proposed changes for v1.5

Rules not followed (with tasks): §5.4 builder cap (21:37–21:39, T-070 and T-058); §8 "read the CI run of
the pushed commit before the next dispatch or queue run" (run 433 after R-302 and T-084); §12 "no entry ⇒
run marked incomplete" (R-260); §2.1 "one command per line when a later step depends" and "never a time
from memory" (R-297, R-301); §11 "row before the next run" (D-177 28 s late); §9 / v1.4 change 12 (R-302's
clone); §2.1 "a number names its source" (D-196, r4); D-174 (13) and ADR-0009 Q-4 A (T-043, T-066 not
dispatched, no row); §5.7 "an unmeasured option is probed before ruling" (four times declined instead).

Rules that cost and changed little: pre-merge `ci/T-NNN` runs for UI tasks while only the two window jobs
count (T-058, T-059: about 5–6 min and 81–87k tokens each; nothing found). The rule is right for the
platform batch; its cost per product task is the helper read, not the run.

Gaps that let something through: Pinned behaviour sentences without evidence (pattern B); list-worded
criteria (pattern A); a test author's own deliverable with acceptance tests (m9); a figure only the
director can measure (T-070 #3); harness conventions that exclude a class of inputs (NEW-1: a frame after
every control request; NEW-2: rows checked only at the default size); CI evidence read from logs.

| # | Change (file, section) | Evidence | Expected effect and cost | What would show it was a mistake | Removes |
|---|---|---|---|---|---|
| 1 | **CI evidence from artefacts by a tool** (§8.1, §13): every evidence job runs all test binaries (`--no-fail-fast` in the gate's test step and in core-crates) and uploads the full output; the record of a CI run (run id, head, per job conclusion, failed step, tests passed/failed, failing tests with the first panic line) is extracted from the artefacts by a named `xtask` command and stored as a file; agents read that file, not job logs | 952,269 tokens of CI reading (15.2 %); R-293 169,441 vs R-294 15,953 on the same run; 366/11 prefix vs 977/22; guards not named in logs | CI reading cost falls by about an order of magnitude (R-294's ratio); reviewers and the evaluator get the record §9 asks them to cite; one high tool task | a failure visible in a log and absent from the artefacts | helper runs reading logs |
| 2 | **CI read before dispatch and queue, by scope** (§8): before any dispatch or queue run the director has the recorded conclusion of the newest run on main that changed a non-ledger path; ledger-only pushes are read in batch at the next addendum (until T-041) | 16 of 73 `ci.yml` runs recorded; run 433 read after R-302 and T-084 | rule and practice agree; nothing read that cannot be red | a red ledger-only push | "every push" |
| 3 | **Absence and limit criteria as equality** (§5.7, planner brief, task template): a criterion that something is absent, limited or the only form is written as equality to a stated set ("exactly the Interface's"); the spec check names one variant outside any list the criterion must reject | T-054 AC-4 (4 survivors, j1); T-070 #1 and #5 | the class goes; one line per criterion | equality criteria that pin what a drafted task changes (golden rule 9) and churn | — |
| 4 | **Every Pinned (i) sentence carries evidence** (§5.7, §6): not only figures, limits and lists; the spec check runs each against the planner's prototype or the base binary and gives the command; otherwise it is marked "unchecked" and is the reviewer's first check; a fact marked KNOWN-not-verified is not copied into a document | T-058 #3, #4 (h3, h4); T-057 #4; T-070 #4; D-201; cut 1 T-055 #2, T-065 #5 | copied-spec document defects (4 of 9 here) fall; spec-check time grows by one run of the prototype | the share of copied-spec defects does not fall over two stages | — |
| 5 | **A test author's own deliverable is tested by guards** (§6, planner brief) | D-189 refused join, m9 | one refused join class less; nil cost | — | — |
| 6 | **Figures only the director can measure** (§5.7): a Pinned figure "to be measured by the director" names the record that will hold it, and the director passes that record to the implementer before the join, or the reviewer writes the sentence | T-070 #3 (D-187 at 19:58, join 20:09, document "not measured yet") | one document defect class less | — | — |
| 7 | **Unmeasured options are not chosen without a probe** (§5.7, §2.1, replacing "probed before ruling") | four unmeasured options declined, none probed, none reversed | rule matches practice; nil cost | a declined option later shown better (T-083 may test Q-5 A) | "probed before the director rules" |
| 8 | **Harness conventions are stated and varied** (§7 round 1, reviewer brief): a UI review's differential run states the conventions its driver uses (frames after control requests, sizes, which rows are checked) and runs at least one variant without each (no frame after a control request; the minimum `--size`; rows of every node) | NEW-1 (T-059 r1 "Each control request is followed by exactly one frame"; T-058 test author "frames 10"), NEW-2 (rows "not required" at small sizes) | defects of the instrument and of small layouts found in review; cost one generator option | such variants find nothing in two stages | — |
| 9 | **Arm records** (§6, §13, review template): the review's `mutants` line adds `killed_by_own_tests`; the join or review front matter adds `acc_test_defects` with `found_by` (implementer, test author, reviewer) | T-058 own 19/19 vs acceptance 18/19, T-059 13/18, T-057 12/18 counted from table prose; acceptance-test defects only in prose (D-212) | the §6 arm comparison becomes computable; two fields | — | — |
| 10 | **Spec check after an overlapping merge** (§5.7): a spec is checked again when a task whose `touches` overlap its own merges after the spec was written, not only when its `depends_on` merges; for a planned chain (the platform batch), each spec is checked by the resumed drafter after its predecessor merges | R-295 after T-058 (30,344 tokens: `Ln`/`Col` collision, h3's rule); `P`: 19 of 36 overlapping pairs | stale specs caught before dispatch; one cheap resumed check per task (26,752–103,198 in this period) | checks that change nothing over a chain | — |
| 11 | **CI tasks in §8.1** (§8.1, §10, implementer and test-author briefs): the `CI-REQUEST` protocol; a CI round counts as one gate attempt for the stop rule; the join of a task whose main criterion is a CI run records that criterion as pending, and the queue requires its CI record (Decision 5's check) | T-070's spec carries the protocol alone; T-079 AC-6; ADR-0009 Decision 5 interim | the platform batch runs on written rules; a text change, the queue check with T-033 | — | the protocol held in one task's spec |
| 12 | **Disk at dispatch counts trees, not only free space** (§2.1): free disk ≥ the measured tree size of every build that will run at once plus 5 GB, the director's own cargo commands included | D-182, D-205 (two disk-full events, one gate attempt lost, a reviewer's re-run) | no disk-full event; nil cost (after T-054 a tree is about 4 GB) | a disk-full event with the rule followed | — |
| 13 | **Accepted decisions carried or re-decided** (§14, §2.1): a pause or stage-close row lists every accepted decision of the last retrospective and ADR not yet done, with where it went | T-043 and T-066 (D-174 (13), D-175 Q-4 A) dropped without a row; D-216's list omits T-043 | the record says what was decided and not done; one list | — | — |
| 14 | **Evaluator clone made by a stated command** (§9, evaluator brief): the director's setup names the command that leaves the ledger out (sparse checkout) and the evaluation's Setup reports it; if `cargo xtask scorecard` needs a ledger file, the methodology names it | R-302 "Ledger records left out: none" | change 12 applied; nil cost | `scorecard` fails without the ledger | — |
| 15 | **Feature drafts older than v1.4 are re-specified** (§5.7, planner brief): a draft last edited before the methodology version in force is written as a new spec (budget, platform statement per test file, pre-merge CI expected), not only spot-checked | the nine drafts of the 17 items (`P`); T-024 "(d) Unix"; budgets 500,000 / 850,000 | no spec dispatched with a pre-ADR-0009 platform line; one full spec run each, which §5.7 mostly requires anyway | — | — |
| 16 | **`ci/` branches are reused, not deleted** (§8.1): one `ci/T-NNN` per task, force-updated per round; the list is kept in STATE | D-198 (403), D-207 | rule matches what the session can do | the owner wants them deleted | D-185's deletion condition |

Not proposed: a new medium ceiling by arm (n = 1 in the independent arm); dropping the pre-merge runs of
UI tasks (they will matter once `gate-macos` counts); more confirmation tooling (no rebuild happened).

## Budget recalibration

Rule as before: p75 of the tier's builder and review tokens per merged task, rounded up to 50k.
- Medium (this period, n = 3): 500,778, 670,559 (+ unknown), 909,094 → linear p75 789,827 → **800,000**, the
  present ceiling (INFERRED from `M tokens`). Keep it; note that the one independent-arm task (909,094)
  needed D-196, and that 670,559 is low by R-272.
- High (pooled with cut 1, n = 5): 455,026, 809,362 (+ unknown), 887,544, 997,800, 1,190,305 → p75 997,800 →
  1,000,000. Keep 1,100,000 (two of the five are not UI tasks; n is small).
- Low (n = 1, T-069 346,304): no recalibration from one task whose cost was reproducing a machine
  property (D-178); a low task that needs a second identity's checkout and build should be estimated as
  such at specification.

## Where the director's record disagrees with the data

- **D-196**: "a first review round has cost 200,000 to 260,000 on the last three UI tasks"; the records give
  314,479 (T-056), 300,749 (T-065), 315,777 (T-057) (`M checks`).
- **D-188 r4 / `ui-project.md`**: "18.0 s for the expand" was the whole script's time (T-057 r2, D-191); the
  sentence is on main.
- **D-192**: "no ruling of the director corrected by a builder or reviewer" for T-057; r4's figure was
  corrected by the round-2 reviewer (D-191).
- **D-175**: the cause of INC-001 (corrected by D-177).
- **D-187**: "366 passed, 11 failed" and the reading of five no-display failures as timeouts came from a
  200-line prefix; the artefact gives 977/22 and five assertions that no window opens without `DISPLAY`
  (D-199).
- **D-174 (13) and D-175 Q-4 A**: T-043 and T-066 "in the next batch"; neither was dispatched, and D-216's
  list of what waits omits T-043.
- **T-086**: "so that a second sighting has a place"; R-264 Q-b and D-182 m2 record a first sighting with
  an inferred cause (`xvfb-run`'s status 5).
- **Handoff 2026-11-16 02:51**: "job logs name no test — artefacts do": a failing test's name is in the logs
  (the negative candidate's `t070_negative_candidate_fails_on_purpose ... FAILED`; macOS round 1's panics
  in its 200 lines); what the logs lack is passing tests, the negative's panic message, and everything after
  the first failing binary or 200 lines.
- **D-179**: "From this row on the CI run of each push is read and its result recorded at the next decision
  row or handoff addendum"; 16 of 73 runs have a recorded result.
- **R-260** booked `completed` with no log entry; **R-292, R-299, R-301, R-303** booked with 11–19 s walls.
- Figures that agree with the records to the token: D-178, D-179, D-188, D-191, D-196 (561,063), D-197,
  D-203, D-204, D-211, D-212, D-215, D-216 (6,178,453; 2,487,330; 36,381,489), and INC-001's 2,249 minutes
  (`M checks`, `M tokens`, `M overhead`).

## 13. Uncertainty

- **Samples**: three cut-2 tasks (two arms, n = 2 and 1), three platform tasks of three tiers; no claim has
  comparable events in number.
- **Classification**: behaviour versus document defects, defect origins, the director-error table (its
  causes and "reached"), and which reviews "partly" saw NEW-2 and NEW-3 are my readings.
- **Own-test kill counts** (19/19, 13/18, 12/18) are counted from the review tables' prose, not a field.
- **Helper wall times**: whether the 11–19 s walls of four CI helpers are their real duration is not
  established; only R-301's typed end time is shown by the record.
- **Unknown tokens**: R-267 and R-272; their size is not known, so T-057's and T-070's totals are lower
  bounds.
- **The usage-limit stop**: its start "about 20:35Z" is the director's statement (D-190); R-274's record
  spans it, so idle time is given with and without it.
- **CI**: I read no CI run; everything about `platforms.yml` results comes from the director's records and
  the helpers' extractions. Whether `ci.yml` runs 367–437 that are not recorded were all green is not
  established (D-217 states 431–436 and 438 green).
- **Change 12**: whether `cargo xtask scorecard` runs in a clone without `tasks/`, `reviews/`, `handoffs/`
  was not tried (no cargo here, by the brief).
- **ui.editor-view re-check**: that the evaluator's files contain no combining mark at a row's start is
  read from the lines quoted in the evaluation; its drivers were not read.
- **Would a fresh spec check have caught the copied sentences**: not observed; a resumed drafter did all
  seven checks of the period.
- **Model per run** (O-11): not visible; not used.
