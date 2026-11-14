---
iteration: 2                # first stage of iteration 2 (ADR-0005)
run: R-223
methodology_in: v1.2
methodology_proposed: v1.3
main_sha: e7d6ef9ef9a8b5118343ae528f5e1051058031f0   # D-142, stage closed; main later gained one ledger commit (43e9162, D-143)
claims: {C1: insufficient, C2: supported, C3: insufficient, C4: supported,
         C5: insufficient, C6: refuted, C7: supported, C8: supported}
         # C6 refuted as a gate (latency: no budget enforced); C8 supported for the non-UI items only
---

# Retrospective, iteration 2, stage 1

## How to read this report

Labels as in retro-1. **VERIFIED**: I ran the command named beside the number and it printed it.
**INFERRED**: my reasoning from verified facts; the reasoning is given. **PROPOSED**: a change for
methodology v1.3, for the director to decide.

Commands, from the repository root (python3 standard library only, read-only):
- `S <section>` = `python3 -I retrospectives/scripts/iter2s1_metrics.py <section>`; sections
  `completeness tokens budgets extends lead queue joins reviews rework arms time time2 acks
  scorecard decisions ceilings gates`.
- `G` = `bash retrospectives/scripts/iter2s1_git.sh` (range `1c0946d..e7d6ef9`, v1.2 in force to
  the stage close; some lines use `b3d9dd2..e7d6ef9`, from D-089).
- Review classes by kind (document sentence or not) are my reading of each review's findings
  table (`reviews/T-NNN-r1.md`, section "Findings"); they are INFERRED classifications of VERIFIED
  rows.

Scope: from v1.2 coming into force (commit `1c0946d`, 2026-11-13T21:24:05Z, D-093) to the stage
close (`e7d6ef9`, D-142; last merge T-049 at 12:03:39Z). Records booked `iteration: 2` include two
planner runs that began before v1.2 (R-154 at 18:17Z, which crossed the D-092 stop). Drift: main is
now `43e9162` (one ledger commit, D-143); nothing I count changes with it (`G`, first lines).

Ten tasks merged (D-142): seven product tasks with a review (T-019, T-020, T-021, T-023, T-036,
T-037, T-038), two low-tier documentation tasks (T-039, T-042) and the publication T-049. The brief
says "nine product tasks"; two of the nine are documentation-only (T-039, T-042). VERIFIED `S queue`.

## Data

| Check | Value | Command |
|---|---|---|
| Run records booked to iteration 2 | **67**: planner 17, implementer 24, test author 10, reviewer 14, evaluator 2; outcomes 60 completed, 7 request_changes, 0 blocked | VERIFIED `S completeness` |
| Run ids R-154..R-222 without any record | **R-197** (mentioned nowhere in `tasks/`, `reviews/`, `docs/`, `handoffs/`, `STATE.md`: an id skipped or a run never booked; cannot tell) | VERIFIED `S completeness`; `grep -rn R-197` |
| Run ids in range booked elsewhere | R-155 (the v1.2 editor) to `iter-1` as role `director`, 271,820 tokens (D-093 says so) | VERIFIED `S completeness` |
| Run records with an impossible wall time | **R-180** (T-023 spec check): `started_at` = `ended_at` minute, `wall_s` 3, 291,166 tokens. The 23:26Z handoff addendum lists R-180 as running, so it started before 23:26Z. Not corrected in the record | VERIFIED `S completeness`; `handoffs/2026-11-13-iteration-2-start.md` addendum 23:26Z |
| Runs whose id appears in no task, review or evaluation file | 0 | VERIFIED `S completeness` |
| Builder and reviewer runs whose log entry accepts or acknowledges the rulings | **48 of 48** (word match on "acknowledg/accept"; R-183's entry was written by the director from the agent's report, the entry says so) | VERIFIED `S acks`; `tasks/T-021-*.md:781` |
| Merged task without any run record | T-049 (a publish made by `cargo xtask publish`; no agent ran) | VERIFIED `S completeness` |
| Transitions | 99 in iteration 2; every product task passes `self_verified → tests_joined → reviewed`; **0 transitions to `evaluated`**, although D-141 confirmed all 13 items; all ten task files say `merged` | VERIFIED `S completeness` |
| Join records | 16, **`acc_defects` null on all 16**; 0 `revert_check` records | VERIFIED `S completeness`, `S joins` |
| Queue records | 10, all `merged` at attempt 1; **0 refusal records** (three refusals are in decision rows only: D-097, D-134, D-139; T-043 not merged) | VERIFIED `S queue` |
| Reviews | 14 (7 tasks × 2 rounds), each with `findings` and `mutants` front matter | VERIFIED `S reviews` |
| Decision rows D-089..D-143 | 54 rows; **D-104 has no row** although it is cited 14 times in `tasks/` and `reviews/` (a T-020 ruling, `tasks/T-020-*.md:465`) | VERIFIED `S decisions` |
| Handoffs | one file for the whole stage (`handoffs/2026-11-13-iteration-2-start.md`, written 21:46Z, six addenda, the last at 10:30Z); no front matter, no `started_at`/`ended_at` of director sessions as the v1.2 template asks; no addendum for the close at 12:03Z (STATE.md carries it) | VERIFIED `wc -l handoffs/*`; `grep -n '^## ' handoffs/2026-11-13-iteration-2-start.md`; `methodology/templates/handoff.md` |
| Spec-check files | 0 new files under `tasks/spec-checks/` (5, all iteration 1); the 14 stage spec checks wrote into the task files, while the planner brief says "in a file under `tasks/spec-checks/`" | VERIFIED `ls tasks/spec-checks`; `methodology/roles/planner.md:55-57` |
| Session paths in tracked files | 0 | VERIFIED `G` |

Metrics built on `acc_defects` are incomplete (null everywhere), as in iteration 1. Times of R-180
are wrong. The director's cost is not measured (METHODOLOGY §13); it is not zero.

## Metrics (METHODOLOGY §13)

| Metric | Value | Command |
|---|---|---|
| Lead time `ready` → merged | high median **2.07 h** (1.64, 1.76, 2.38, 3.39; n=4); medium median **4.96 h** (2.10, 4.96, 5.75; n=3); low 0.13–0.36 h (n=3). Iteration 1: high 4.80, medium 1.00 (retro-1). T-020's 4.96 h and T-021's 5.75 h contain the 23:54–02:10 stop | VERIFIED `S lead` |
| First-pass queue success | **10/10** by record; **7/10** if the three precondition refusals count (D-097 T-039, D-134 T-037, D-139 T-038), all three caused by the director's working tree or records | VERIFIED `S queue`; decisions |
| Queue gate time | median **486 s** (241 s for T-019 → 574 s for T-023; 2.4×); iteration 1 median 199 s | VERIFIED `S queue` |
| Gate attempts per task | median **3** (0–5, sum 27, n=9; T-049 has no run); iteration 1 median 3 (retro-1) | VERIFIED `S gates` |
| Acceptance-caught defects | **0 by record** (`acc_defects` null on 16/16 joins), and **0 in fact**: `acc_fail_first_join` is 0 on all 16 joins, so no acceptance test failed against an implementation at any join | VERIFIED `S joins` |
| Review findings by class and tier (round 1) | high (4 reviews): defect 2, spec gap 4, test gap 4, nit 10. medium (3): defect 3, spec gap 7, test gap 6, nit 4. Round 2 (7 reviews): 4 nits, nothing else | VERIFIED `S reviews` |
| Escaped defects (evaluator, by origin) | **0 behaviour defects**; 4 findings, all documents or preconditions: NEW-1 (T-019, high, independent), NEW-2 (T-020 medium independent, and T-016), NEW-3 (T-036/T-037, high), NEW-4 (T-023, medium, implementer-written) | `evaluations/iter-2.md` "Defects filed"; D-141 |
| Rework ratio | 10 task files first committed in the stage carry `origin_task` (T-043, T-044, T-045–T-048, T-050–T-053); 8 of them name a stage-1 task as origin (T-045, T-048, T-051 ← T-020; T-047, T-050 ← T-019; T-046 ← T-021; T-052 ← T-036; T-053 ← T-038) | VERIFIED `G` (last sections) |
| Review rounds per task | 2 for each of the 7; no round 3 | VERIFIED `S reviews` |
| Acceptance kill rate | round 1: 130/149 = **87 %** (any test: 140/149); round 2: 13/24 = 54 % (any test 24/24). Iteration 1: 81 % and 75 % | VERIFIED `S reviews` |
| Known defects merged by decision | **0** defects. Documented limits accepted at merge: no bound on what `.gitignore` files cost (T-019, D-107 → T-047); no time bound on parse and no cap on highlight spans (T-020 → T-045, T-048); no time bound on search (T-021 → T-046) | decisions D-107, D-113, D-118; `G` |
| Tokens per merged task by tier | high median **810,888**, p75 866,379, max 932,643 (n=4); medium median **686,472**, p75 749,633, max 812,794 (n=3); low 84,466 and 124,444 (T-049 none) | VERIFIED `S tokens` |
| Tokens per merged task with overhead | 9,201,078 / 10 = **920,108**; per product task 1,314,440; without overhead 565,617. Iteration 1: 915,433 with overhead | VERIFIED `S tokens` |
| Share of scorecard items verified | **41/64 = 64.1 %** on main after T-049 (0 implemented, 0 failing, 23 planned) | VERIFIED `S scorecard` |
| Red-main minutes | **0** recorded (no `incidents/` file in the stage) | `ls incidents` (directory absent) |
| Budget overruns | **1 of 10**: T-020 1.16× its ceiling (812,794 of 700,000), with an Extend row (D-109) written before the rework | VERIFIED `S budgets`, `S extends` |

Further numbers:

| Number | Value | Command |
|---|---|---|
| Iteration-2 tokens booked | **9,201,078** (67 runs); stage tasks 5,656,173; `iter-2` overhead **3,544,905 = 38.5 %**: spec checks 2,286,780 (14 runs), other planner 853,289 (R-154, R-160, R-212), evaluator 404,836 (R-221, R-222). Not included: R-155 (271,820, booked to `iter-1`), this analysis, the director | VERIFIED `S tokens` |
| — by role | planner 3,140,069; implementer 2,193,124; test author 1,305,631; reviewer 2,157,418; evaluator 404,836 | VERIFIED `S tokens` |
| Spec checks by task | T-019 277,127; T-020 335,094; T-021 300,611; T-023 321,909; T-036 385,953; T-037 269,512; T-038 396,574 (= 42 % of the seven product tasks' 5,447,263) | VERIFIED `S tokens` |
| Tokens per new verified item | item tasks only: 2,935,175 / 13 = **225,783**; with their four spec checks 320,763; all iteration-2 tokens / 13 = **707,775**. Iteration 1: 10,985,201 tokens for 6 items claimed (retro-1) ≈ 1.83 M per item (INFERRED division) | VERIFIED `S tokens` |
| Post-round-1 rework | **461,408 = 8.5 %** of product-task tokens (iteration 1: 24.6 %); T-037 160,588 (19.0 %, of which 19,089 for ruling k6/k7, R-210 and R-213), the others 18,409–74,183 (2.0–9.1 %) | VERIFIED `S rework` |
| Review tokens by round | r1 **1,949,250** (7 reviews, 229,836–326,832); r2 **208,168** (7 reviews, 7,359–61,441) | VERIFIED `S reviews` |
| Mutants seeded | r1 151 (2 equivalent, 140 killed, 9 survived); r2 24 (0, 24, 0) | VERIFIED `S reviews` |
| ADR-0005 cost estimate vs spend | estimate 3.6–4.1 M for T-019, T-020, T-021, T-023, T-036 at iteration-1 medians; booked **3,670,329** on those five task records, plus 1,620,694 of their spec checks now booked to `iter-2` (INFERRED sum of `S tokens` rows) | VERIFIED `S tokens`; ADR-0005 "Consequences" |

## 1. Did each v1.2 change do what it was for?

| # (CHANGELOG) | Change | What happened in the stage | Verdict |
|---|---|---|---|
| 1 | Planner runs and spec checks booked to `iter-N`; budget covers building and review | All 17 planner runs are booked to `iter-2`; 14 carry a note beginning `spec check:`. No task record holds a planner run. The cost moved, it did not shrink: spec checks are 2,286,780 tokens, 42 % of what the seven product tasks cost. One booking error: R-180's times (wall 3 s) | **Worked** (VERIFIED `S tokens`, `S completeness`) |
| 2 | Row before rework at ≥ 75 % of the ceiling | Four rows, each committed before the first rework run: D-109 T-020 23:45:32 < R-185/R-184 23:46:08; D-115 T-021 03:01:31 < R-191 03:02:02; D-123 T-023 05:17:53 < R-204 05:18:10; D-137 T-038 09:46:41 < R-219 09:46:54. At the verdict the tasks stood at 105.5 %, 89.1 %, 85.7 % and 83.1 % of the ceiling. Rework afterwards cost 74,183, 62,451, 58,445 and 18,409 tokens; every task ended 7–15 % under its N (92.4 %, 88.0 %, 86.6 %, 84.8 % of N). Every row says "nothing is moved to a follow-up" (D-109 moves only an out-of-task cap). **No row changed what was done.** T-020 had passed its ceiling before round 1 ended, so the row recorded an overrun already spent. T-037 (62 % at the verdict) had the largest rework, 160,588, and needed no row | **Followed, no effect** (VERIFIED `S extends`, `G`) |
| 3 | Ceilings 1.1 M / 700k / 150k | 1 task of 10 over its ceiling (T-020, medium, 1.16×); medium tasks spent 94–116 % of 700,000, high tasks 67–85 % of 1,100,000 | Medium ceiling tight, high loose (VERIFIED `S budgets`) |
| 4 | Rulings state behaviour; bounds cite a measurement; builders acknowledge | 48/48 builder and reviewer runs record acceptance or acknowledgement. Ruling errors fell from 14 (iteration 1) to 3 by my table (below). Two rulings still stated something without the evidence in hand: y1 (D-100 → D-102: "reads directories and `.gitignore` files and nothing else") and the memory sentence of D-103 (→ D-110: "largest peak" read as a bound). Both were caught by an agent acting on the new rule (the implementer's `strace` check; the reviewer's sentence check) | **Worked in part** (VERIFIED `S acks`; decisions) |
| 5 | Guarantee sentences from Pinned; implementer logs evidence for others | Implementers probed pinned sentences against the binary and found 4 untrue or unproducible before review: T-019 S1 (D-100) and its restatement (D-102), T-020's memory script (D-103), T-021's 700/800 chars (D-106). But 3 of the 5 review defects were sentences the spec or the director had pinned and the implementer copied (T-020 #1 from D-103; T-023 #1, N8 written by the spec check; T-019 #2, the depth wording ruled by y4). Copying moves the error to the spec; it does not remove it | **Partly**: the implementer's probe catches; the copy rule itself does not (INFERRED from review findings and decisions) |
| 6 / golden rule 9 | No pins of what a drafted task changes | Applied before dispatch four times: D-094 (T-039: three word-for-word ACs), D-114 (T-036: an AC testing absence of words), D-120 (T-037 spec check removed tests of document and message text), D-130 (T-038: three criteria). One violation reached review: T-021's own unit tests pin message texts (r1 nit 6), against its Pinned (f) | **Worked** (decisions; `reviews/T-021-r1.md`) |
| 7 | Pre-dispatch spec check; split bundled high specs | 7 checks (14 runs), **65 blockers** by the decision rows' counts (T-019 8, T-020 6, T-021 9, T-023 12, T-036 12, T-037 8, T-038 10): about 35,000 tokens per blocker. Two blockers made a spec unbuildable as written (T-019, D-095); the checks measured three false bounds (T-020 parse time and memory, D-097; T-021 search time and reply memory, D-101). No builder run ended `blocked` (iteration 1: T-010's unchecked spec stopped both builders, 226,395). T-036 was split (T-044, D-111). **Misses that reached builders:** T-036/T-037 interaction (D-117, both builders asked); the spec check wrote the false N8 (T-023 defect, D-124); the Rust API left open (D-136, a test author could not write 5 tests, R-217 30,281); the T-037/T-023 shared trait (found by another task's check, D-128) | **Worked, at 42 % of build cost** (VERIFIED `S tokens`; decisions) |
| 8 | Director alone edits `scorecard/`, at the join | 0 joins failed at `scorecard` (iteration 1: 2). But in a clone under a 0700 directory the scorecard computes 27 verified and `ws.path-confinement` failing (NEW-3): the generated status depends on the checkout's location | **Worked** for its purpose (VERIFIED `S joins`; `evaluations/iter-2.md` NEW-3) |
| 9 | Join counts by path, refuses a failing record without `--acc-defects`; refusals recorded (T-033) | T-033 not merged; interim: the director passes `--acc-defects` at every failing join. The interim rule never fired, because no join had a failing acceptance test; T-021's first join failed at `no-skip` because of an acceptance-test defect (D-108) and its record has `acc_defects: null`. Refusals: 3, in decision rows, 0 in `queue.jsonl` | **Not delivered**; interim covers only part (VERIFIED `S joins`, `S queue`) |
| 10 | No revert check | 0 revert records; no review or decision asks for one | **Removed without loss visible** (VERIFIED `S completeness`) |
| 12 | Resume after a usage-limit stop; after a restart, `git status` everywhere and relaunch fresh | Three stops in the stage (D-092, D-112, D-129) and one after it (D-143), each with a resume and a rescheduled check-in. Four container restarts (02:11:36, 02:59:31, 07:11:37, 18:15:21). After **every** restart the director resumed the agents instead of relaunching them (D-112, D-116, D-129, D-143), with a restart notice, and gave the reason in D-112. Nothing committed was lost; lost: one gate attempt and one final report (D-116) | Stops: **followed**. Restarts: **not followed 4 of 4**, deliberately (VERIFIED decisions) |
| 13 | Director cost not measured; handoff records each director session's start and end | No director run record (correct). No handoff carries session start/end; the template's front matter is not used | **Not followed** (VERIFIED `handoffs/`) |
| 14 | No CI run for ledger-only pushes (T-041) | Not merged; 165 of 262 commits in the stage change only ledger paths (`G`); every push still starts CI (not countable here) | **Not delivered** (VERIFIED `G`) |
| 16 | Stage iteration 2 (ADR-0005) | 41/64 verified after the evaluation, as the ADR's arithmetic said. ADR-0005 listed T-033 and T-041 in stage 1; neither ran, and D-142 closed the stage without a row that removes them (D-127 says only that they "need not precede the evaluation") | **Worked** for the line; the stage list was not kept and not amended (VERIFIED `S scorecard`; ADR-0005 "Decision"; D-127, D-142) |
| 17 | Reviewer checks document sentences against the binary | Found **all 5 round-1 defects** (all are document or published-description sentences) and 6 document spec gaps and 10 document nits. The evaluator still found 4 documentation findings (section 5) | **Worked**; the largest single source of findings (INFERRED classification of VERIFIED rows) |
| 18 | CLI changes update `docs/control-protocol.md`; only producible kinds published | T-036 edited the `--root` sentences and exit codes (D-092, D-121); T-038 cut `error_kinds.rejected` from 24 to 13 declared kinds (D-138); the evaluator saw 0 `internal` in 1,634 hostile requests and produced every declared kind except `nav.workspace-symbols`' `not_found`/`not_a_file`, which need a race (NEW-4) | **Worked** (D-138, D-141) |

## 2. The medium-tier arms

Assignment by number (ADR-0005): T-020 even (independent test author); T-021 and T-023 odd
(implementer writes acceptance tests first). Order, VERIFIED from history: T-021 tests `f7a77b8`
22:26:15Z before the first code commit `9044413` (D-106), one test-fix commit `46bc135` after a trial
join; T-023 tests `6a60a75` 04:09:01Z and `e8423ff` 04:10:15Z before code (D-122) (`G`).

| Measure | T-020 (independent) | T-021 (implementer-first) | T-023 (implementer-first) | Command |
|---|---|---|---|---|
| Tokens (task) | 812,794 | 686,472 | 658,423 | `S arms` |
| — implementer | 261,046 | 345,003 (incl. tests) | 332,512 (incl. tests) | `S arms` |
| — test author | 289,855 | — | — | `S arms` |
| — reviewer (2 rounds) | 261,893 | 341,469 | 325,911 | `S arms` |
| Spec check (`iter-2`) | 335,094 | 300,611 | 321,909 | `S tokens` |
| Joins; first join | 2; green | 3; **gate_failed** at `no-skip` (an acceptance test under `#[cfg(unix)]`, D-108) | 2; green | `S joins` |
| Acceptance failures at any join | 0 | 0 | 0 | `S joins` |
| Round-1 findings d / sg / tg / nit | 1 / 2 / 2 / 2 | 1 / 2 / 1 / 2 | 1 / 3 / 3 / 0 | `S reviews` |
| Round-1 defect | document sentence (memory, D-103 wording) | published defaults `null` in the schema | document sentence (N8, pinned by the spec check) | review findings |
| Round-1 kill rate by acceptance | 14/18 = 78 % | 22/23 = 96 % | 34/38 = 89 % | `S reviews` |
| Found later (evaluator) | NEW-2 (item title; corpus owned by T-016) | none | NEW-4 (two published refusals need a race) | `evaluations/iter-2.md` |

With iteration 1 (retro-1; same script, `S arms`): independent T-010 520,658 and T-018 848,301;
implementer-first T-011 431,922. Pooled, n=3 per arm: tokens independent 520,658 / 812,794 /
848,301, implementer-first 431,922 / 658,423 / 686,472; round-1 kill rate 43/50 = 86 % against
71/79 = 90 %; review defects, all rounds, 4 (T-018 3, T-020 1) against 2 (T-021 1, T-023 1); first
joins failing 0/3 against 2/3 (T-011 at `scorecard`, a stale page; T-021 at `no-skip`). VERIFIED
`S arms`.

**What the sample can show (INFERRED).** (1) The implementer-first protocol was followed in both
odd tasks. (2) The implementer-first arm was cheaper in all three pairs by task total, by
about 70–84k of implementer tokens against 290k for a separate test author in this stage; the
tasks differ in content, so this is not a controlled difference. (3) The implementer-first arm has a
mechanical weakness the record shows once: the implementer's gate runs on the code branch without
the acceptance tests, so a test that the gate refuses is first seen at the join (D-108). T-021's and
T-023's implementers ran a trial join before reporting (commit `46bc135` "found on the throwaway
join"; D-122 "all ten pass on a trial join"), which is not a rule. **What it cannot show:** whether
independent tests catch defects the implementer's own miss (C3). No acceptance test in either arm
failed against an implementation at any of the 16 joins, so there is no event to compare; every
defect in both arms was a document sentence found by the reviewer. n=3 per arm with parity-assigned
work is not a sample for kill-rate differences of 4 points.

## 3. Joins

16 joins over 7 tasks (2, 2, 3, 2, 2, 3, 2). First joins: 6 green, 1 `gate_failed` (T-021,
`no-skip`, an acceptance-test defect the director classified as such in D-108; the record says
`acc_defects: null`). Every join had `acc_fail_base` = `acc_total` (every acceptance test failed
on base); `base_fail_kind` `fails` 9, `mixed` 7, no compile-only kind. `acc_fail_first_join` = 0 on
all 16: **the acceptance tests caught no implementation defect at any join.** Re-joins: 8 after
rework, one more for T-037 after main was merged into the branch (ruling k6, D-128); T-037's join
at 06:06 was on the old base and had to be repeated (D-129). Guards: T-019 2, T-036 9, T-037 2,
T-038 3; none failed. VERIFIED `S joins`.

What the acceptance tests did do (VERIFIED `S reviews`): they killed 130 of 149 live round-1 mutants
and 13 of 24 round-2 mutants, i.e. they work as a regression net. Test authors also raised spec
problems: T-036's two builders found the T-037 interaction independently (D-117); T-038's test
author found that the spec left the declaration API open, and a prototype that let an undeclared
refusal through passed every test it could write (D-136).

## 4. Reviews

Findings per task and round, from front matter (VERIFIED `S reviews`), with my split of each class
into document sentence (D) or not (INFERRED from the findings tables):

| Review | Verdict | defect | spec gap | test gap | nit | mutants in/eq/killed/by acc/surv | tokens |
|---|---|---|---|---|---|---|---|
| T-019 r1 | request-changes | 2 (2 D) | 2 (0 D) | 0 | 2 (1 D) | 24/0/24/21/0 | 239,923 |
| T-019 r2 | approve | 0 | 0 | 0 | 0 | 1/0/1/0/0 | 25,755 |
| T-020 r1 | request-changes | 1 (1 D) | 2 (1 D) | 2 | 2 (2 D) | 18/0/16/14/2 | 229,836 |
| T-020 r2 | approve | 0 | 0 | 0 | 2 (2 D) | 5/0/5/5/0 | 32,057 |
| T-021 r1 | request-changes | 1 (1 D, the generated schema) | 2 (2 D) | 1 | 2 (1 D) | 24/1/22/22/1 | 306,349 |
| T-021 r2 | approve | 0 | 0 | 0 | 0 | 7/0/7/1/0 | 35,120 |
| T-023 r1 | request-changes | 1 (1 D) | 3 (2 D) | 3 | 0 | 38/0/35/34/3 | 295,837 |
| T-023 r2 | approve | 0 | 0 | 0 | 1 (1 D) | 3/0/3/3/0 | 30,074 |
| T-036 r1 | request-changes | 0 | 0 | 1 | 2 (1 D) | 15/1/14/14/0 | 271,118 |
| T-036 r2 | approve | 0 | 0 | 0 | 0 | 4/0/4/0/0 | 16,362 |
| T-037 r1 | request-changes | 0 | 1 (0 D) | 2 | 3 (3 D) | 17/0/15/11/2 | 279,355 |
| T-037 r2 | approve | 0 | 0 | 0 | 1 (1 D) | 3/0/3/3/0 | 61,441 |
| T-038 r1 | request-changes | 0 | 1 (1 D, ADR-0004) | 1 | 3 (2 D) | 15/0/14/14/1 | 326,832 |
| T-038 r2 | approve | 0 | 0 | 0 | 0 | 1/0/1/1/0 | 7,359 |

Totals round 1: defect 5 (**5 D**), spec gap 11 (6 D), test gap 10, nit 14 (10 D): **21 of 40
round-1 findings are document sentences**, and **0 defects are wrong command behaviour**. Round 2:
4 nits, all document wrapping or wording. Cost: round 1 1,949,250 tokens, 389,850 per defect,
129,950 per defect or test gap; round 2 208,168.

**Round 1 request-changes on all seven: too strict, or real gaps? (INFERRED.)** Four of the seven
were request-changes on a defect, each an untrue sentence about the binary (real by the rule that
documents are claims, and three of them came from text the director or the spec check wrote). Three
were request-changes on test gaps alone: T-037 (two surviving mutants: re-open of an unedited
document, selection and redo after a refusal), T-038 (audit rule (iv) removable with every test
passing) and T-036 (the implementer's test left its fixture in `/tmp` on failure, a hygiene gap,
not a surviving mutant). Nine of the ten test gaps are surviving mutants that change an observable
answer. The rework they forced was small: 41,177 (T-036), 160,588 (T-037, of which about 19k was
ruling k6/k7, not the gaps) and 18,409 (T-038); round 2 cost 7k–61k. The rule is not too strict on
this evidence; one of ten blocking test gaps (T-036) would have done as well as a follow-up. The
uniform request-changes is a sign of where findings come from (documents and untested branches),
not of reviewers inventing work.

## 5. The evaluation against the reviews

The evaluator (R-221, R-222; 404,836 tokens) confirmed all 41 items, found 0 behaviour defects and
4 findings (`evaluations/iter-2.md`; D-141). Against the reviews:

| Finding | Did a review see it? | Why the evaluator found it (INFERRED) |
|---|---|---|
| NEW-1 `.gitignore` forms where Kiln differs from git, not in the list | Partly: T-019 r1 finding 1 found three other differences (trailing Unicode whitespace, POSIX classes), and the list was reworded "found so far (this list is not complete)" (rulings y6) | Method: the review compared 12 hand fixtures with `git ls-files` (`reviews/T-019-r1.md:76`); the evaluator ran a seeded fuzz of 600 patterns against git and found 8 differing patterns of 4 kinds |
| NEW-2 `syntax.parse` title wider than the claim (2 compiling `gap` files have error nodes) | No; T-037 r1 nit 6 did flag a scorecard title of another item | Item titles are the director's file; the T-020 review checked behaviour against the spec, which names the `valid` set |
| NEW-3 five `ws.path-confinement` tests fail in a checkout under a 0700 directory | No | Environment: every review ran in the primary checkout under 0755 directories; the evaluator ran in a fresh clone under the scratch directory |
| NEW-4 `nav.workspace-symbols` publishes `not_found`/`not_a_file` the evaluator could not produce | The T-038 reviewer produced both by toggling a directory during requests (`reviews/T-038-r1.md:72-78`) but did not flag that nothing published says they need a race | The evaluator reads only published text, so a refusal it cannot reach is a finding; a reviewer who reaches it by a race sees no defect |

Also: the evaluator could not evaluate `syntax.parse` until the director ruled that the corpus under
`crates/` is readable fixture data (the brief forbade `crates/`, a director brief error; R-222 20,610
tokens). **Not run** by the evaluator: requests raced against disk changes; an ordinary non-root
identity (permission cases ran as uid 0 without the DAC capabilities, because the binary lay under
a 0700 directory); `syn` as an outline oracle; the release-build figures of `nav.md` and the 41–42 s
search figure; T-044 and T-047 cases; `ui-snapshot` (no UI). Cost: 9,874 tokens per item checked,
101,209 per finding (INFERRED division of `S tokens`).

## 6. The director as a failure source

Every director error or correction I found in the stage, with its source (decision rows, task
files, records; VERIFIED by reading each row; "reached" = an agent acted on it before it was
corrected):

| # | Error or correction | Cause | Reached | Caught by | Measured cost |
|---|---|---|---|---|---|
| 1 | D-093: ADR-0005 file name from the director's brief broke the convention | brief | editor R-155 | director, before commit | — |
| 2 | D-092 ruling Q-1 (T-036) protected only roots searchable but not readable; a mode-0000 root stayed accepted and unprotected (D-111) | ruling | planner R-181 | spec check | inside R-181 |
| 3 | D-095: T-039 made ready with "nothing is written to disk", false with `--log` | spec accepted unchecked | implementer R-159 | director, reading the branch | R-161 6,650 |
| 4 | D-097: queue refused T-039, a planner's uncommitted edit made the ledger dirty (third such refusal) | operation | — | tool | one queue run |
| 5 | D-099: T-042 spec accepted with two defects (AC-1 did not exempt the log; a grep lacked `-e`) | spec accepted | implementer R-166 | director at queue | none |
| 6 | D-100 ruling y1 restated S1 without evidence; untrue too (`readlink` calls), D-102 y5 | ruling | implementer R-168 | implementer (`strace`) | R-171 2,517 |
| 7 | D-103: the director worded the memory sentence; "largest peak" read as a bound, untrue (T-020 r1 defect, D-110) | ruling | implementer R-172 | reviewer R-179 | share of rework 74,183 |
| 8 | D-104 cited 14 times, no row in `docs/decisions.md` | bookkeeping | — | not caught (found here) | — |
| 9 | D-106: R-173 first booked with typed times | bookkeeping | — | director, before commit | — |
| 10 | D-117: T-036 AC-4 expected a re-open to be refused (T-037's open finding); spec made ready with it | spec accepted | implementer R-189, test author R-190 | both builders | not separable |
| 11 | D-122: the dispatch message named the command by the item id (`nav.workspace-symbol`) | brief | implementer R-194 | implementer | R-200 3,794 (with one more fix) |
| 12 | D-124: N8, pinned by the spec check, contradicted Q-3; spec made ready with it (T-023 r1 defect) | spec accepted | implementer R-194 | reviewer R-203 | share of rework 58,445 |
| 13 | D-128: T-037 and T-023 in flight together over a shared trait and document; the overlap rule cannot see it | dispatch | implementer (rework) | planner R-209, by reading | R-210 12,489 + R-213 6,600 + a re-join |
| 14 | D-129: T-037 re-joined on its old base after k6 was known; repeated | operation | — | director | one join |
| 15 | D-134: ruling k6 widened `touches` in the rulings section only; queue refused | step omitted | — | tool | one queue run |
| 16 | D-136: T-038 spec (checked) left the declaration API open; made ready | spec accepted | test author R-216 | test author | R-217 30,281 |
| 17 | D-137 j3: ADR-0004's bullet not replaced when ADR-0006 was accepted (D-128) | step omitted | reviewer R-218 | reviewer | a spec-gap finding |
| 18 | D-139: transitions recorded but the task file's state line not changed; queue refused | step omitted | — | tool | one queue run |
| 19 | D-141: the evaluator's brief forbade `crates/`, so `syntax.parse` could not be evaluated | brief | evaluator R-221 | evaluator | R-222 20,610 |
| 20 | D-131: the planner corrected the director's brief for T-045–T-048 in five places (figures called measured that were debug builds or extrapolations; a mechanism from a report) | brief | planner R-212 | planner | inside R-212 |
| 21 | R-180 booked with start = end (wall 3 s; it ran before 23:26Z) | bookkeeping | — | not caught | — |
| 22 | T-021 first join classified as an acceptance-test defect (D-108) but recorded with `acc_defects: null` | bookkeeping | — | not caught | — |
| 23 | Handoff kept as one file with addenda; no director session start/end (v1.2 change 13) | step omitted | — | not caught | — |
| 24 | D-142 closed the stage without T-033 and T-041, which ADR-0005 lists in stage 1, and without a row amending the ADR | step omitted | — | not caught | — |

Not counted as errors, but rule deviations the director chose and explained: relaunch-after-restart
not followed after four restarts (D-112, D-116, D-129, D-143); T-042's implementer told not to run
the full gate because of disk (D-099).

Tallies (VERIFIED by counting the table): **24** errors or corrections; **10 reached a builder**
(implementer, test author or reviewer: #3, 5, 6, 7, 10, 11, 12, 13, 16, 17) and 4 more reached a
planner, editor or evaluator (#1, 2, 19, 20). Caught by: an agent 11; the director 5 (#1, 3, 5, 9,
14); a tool 3 (#4, 15, 18); nobody 5 (#8, 21, 22, 23, 24). By cause: ruling 3, spec accepted with an
error 5, brief 4, dispatch 1, operation or step omitted 6, bookkeeping 5 (my classification).

Against iteration 1 (40, 19 reached a builder, 12 tasks; ruling errors 14, 9 reached a builder):
the stage has 24 and 10 over 10 tasks; ruling errors fell to 3, 2 of which reached a builder. The
largest class is now **specs the director made ready with an error in them (5)**, four of which a
spec check had read (#10, 12, 16; #3 and #5 had no check, being low tier). Measured direct costs
(#3, 6, 11, 13, 16, 19): about 83,000 tokens plus shares of two reworks, three queue refusals and
two extra joins (INFERRED sum of the costs column).

## 7. Cost

- Per task, role and item: see Metrics and "Further numbers" (VERIFIED `S tokens`).
- Overhead booked to `iter-2`: **3,544,905 = 38.5 %** of 9,201,078. Spec checks alone 2,286,780,
  about 326,700 per checked task; the three other planner runs 853,289 (drafting T-037–T-039, the
  v1.2 tooling specs, the limit follow-ups). Iteration 1's comparable `iter-1` overhead was 1,454,555
  of 10,985,201 (13.2 %), when spec checks were booked to tasks (retro-1); the shares are not
  comparable for that reason.
- Budget against spend: no stage budget was set. ADR-0005 estimated 3.6–4.1 M for the four item
  tasks plus T-036 at iteration-1 medians; the five task records hold 3,670,329, inside the estimate,
  but their spec checks add 1,620,694 that the estimate did not include (it was made on iteration-1
  medians whose tasks carried some spec checks) (VERIFIED sums of `S tokens` rows).
- Against iteration 1: fewer tokens (9.20 M against 10.99 M) for more items (13 new verified against
  6 claimed), less rework after round 1 (8.5 % against 24.6 %), one budget overrun against 7 of 12.

## 8. Throughput and environment

| Number | Value | Command |
|---|---|---|
| v1.2 in force → stage close | 14.66 h (21:24:05Z → 12:03:39Z) | `S time2` |
| Usage-limit stops inside it | D-112 23:54–02:10 (2.27 h), D-129 about 06:16–07:10 (0.90 h): 3.17 h; available 11.49 h. Before it: D-092 18:31–21:12 (2.68 h). After the close: D-143 12:05Z, limit reset 16:00Z, machine restarted 18:15:21Z, resume about 18:15Z: **6.17 h** lost wall time, no work lost (D-143; coordinator's message) | `S time2`; decisions |
| Container restarts | 02:11:36, 02:59:31 (not tied to a limit), 07:11:37, 18:15:21 | D-112, D-116, D-129, D-143 |
| Time with no run of any role active, net of the stops | **2.50 h of 11.49 h (22 %)**; iteration 1: 6.8 h of 14.76 h (46 %, retro-1). Gaps ≥ 10 min: 07:26–07:41, 07:47–07:59, 08:21–08:42, 09:48–10:07, 10:08–10:22, 11:36–12:03 (director work between runs: re-joins, rulings, queue) | `S time`, `S time2` |
| Builder concurrency | max **3** builder runs open at once (cap held); hours at 0/1/2/3 builders over the whole span: 2.98 / 6.70 / 2.13 / 0.90; max 5 runs of any role | `S time` |
| Long runs | R-154 10,902 s and R-184 9,293 s each span a usage stop (23,655 tokens for R-184's 2.6 h); R-218 3,778 s (a review with 116,000 fuzzed requests); R-221 4,254 s | `S time` |
| Gate time | queue gate 241 s → 574 s over the stage; `schema` step about 0.5 s → 3.5 s with T-038 (D-138); the `fuzz` and `budgets` steps are EMPTY ("2 EMPTY: fuzz, budgets", `tasks/T-038-*.md`) | `S queue`; `grep EMPTY` |
| Disk | a worktree build about 6.3 GB; 12 GB free at D-097, so feature tasks were dispatched "as the disk allows (15 GB free before each dispatch) rather than three at once"; 23 GB after clearing 6.7 GB of stale artifacts (D-098); T-020's join waited for disk (9.9 GB free, D-105); `publish` leaves a 4.0 GB build directory (D-101). Now: 23 G available (`df -h /home/user`) | decisions; `df` |

INFERRED: the binding limit on throughput in this stage was disk and the serial dependency chain
(T-019 → T-023; T-036 → T-037 → T-038), not the cap of three builders: three builders were open
together for only 0.90 h. The usage limit cost 3.17 h inside the stage and 6.17 h after it.

## 9. Claims

| Claim | Verdict | Numbers that decide it | What would change the verdict |
|---|---|---|---|
| C1 contracts and checks first | **insufficient** | Pre-dispatch spec checks: 65 blockers, 0 blocked builder runs, 42 % of build cost; one contract change (ADR-0006) as its own high task, merged before its consumers, with declaration notes added to six drafts (D-139). But four spec-check misses reached builders (section 1, row 7), and the stage has no arm without checks to compare | A stage where unchecked specs (or cheaper checks) are compared with checked ones on blocked runs and rework |
| C2 serial queue keeps main green | **supported** | 10/10 first attempts merged; 0 red-main minutes; 3 precondition refusals, all director-caused; gate time 2.4× longer | A red main, or queue time becoming the throughput limit |
| C3 tests by another agent catch what the implementer's miss | **insufficient** | 16 joins in both arms, 0 acceptance failures against an implementation; all 5 defects were document sentences found by reviewers; independent test authors found two spec problems (D-117, D-136); kill rates 86 % vs 90 % pooled, n=3 per arm. Over iterations 1 and stage 1, 44 joins carry no recorded acceptance-caught defect | An acceptance-caught defect at a join, by arm; or a seeded-defect trial (the same spec built twice, tests from each arm run against both) |
| C4 status from behaviour tests plus evaluator | **supported** | 13 implemented → 13 confirmed, 28 re-confirmed, 0 rejected; 0 inert features; the 4 findings are claims wider than the binary or preconditions, filed as T-050–T-053. Caveat: the computed status depends on the checkout's location (NEW-3) | An evaluator rejection of an item the scorecard computed `verified`, or a confirmed item later found inert |
| C5 risk tiers place review effort | **insufficient** | Round-1 review tokens per task: high 239,923–326,832, medium 229,836–306,349, about equal; defects high 2, medium 3, all documents; 0 behaviour defects in either tier, so "where defects are expensive" has no event | Behaviour defects by tier, or an escaped defect from a lower tier |
| C6 budgets that gate | **refuted** as a gate | Latency: the gate's `budgets` step is EMPTY, 0 latency budgets enforced. Tokens: one overrun (T-020), recorded honestly before rework; the four 75 % rows changed nothing (section 1, row 2) | An enforced latency budget (T-028), or a token row that stops or splits work |
| C7 repository is the memory | **supported** | 3 usage stops and 4 restarts, every resume from the repository with nothing committed lost; 48/48 builder runs logged. Defects of the record: D-104 missing, R-180's times, `acc_defects` null, handoff without session times | A loss of committed state or a resume that needed information not in the repository |
| C8 constructed oracles define correct | **supported** for the 13 non-UI items | Evaluator: 450 trees against `git ls-files`, 8,000 search/replace checks against a model, 840 third-party files, 6,870 incremental against fresh parses, 450 outlines, 3,600 goto-line checks, rustc on 45 corpus files: 0 mismatches attributable to Kiln; NEW-1 shows the oracle (git) also finds where the documents are short. Untested for the UI | A UI item whose "correct" cannot be stated by such an oracle, or a mismatch the oracles missed |

## 10. The charter's exit criterion

ADR-0005 / CHARTER §5 stage 1: at least 60 % of items verified, read from `cargo xtask scorecard`
after stage 1's evaluation, before any stage-2 task is dispatched. `docs/scorecard.md` on main:
`64 items: 41 verified, 0 implemented, 0 failing, 23 planned.` → 64.1 %, two items over the 39
needed. **Met.** VERIFIED `S scorecard`. No stage-2 task has been dispatched (all 24 non-merged task
files are `draft`, `G`). Caveat (INFERRED): the same command in the evaluator's clone printed 27
verified with `ws.path-confinement` failing (before publication); in such a location the
published figure would read 40 of 64, still over the line, so the margin holds by one item.

Iteration 2's final criterion (every item verified; evaluator sign-off; budgets enforced) is far:
23 items planned, none started; 8 are `ui.*` and no UI code exists (`crates/kiln-ui/src/lib.rs`
and `apps/kiln/src/main.rs`, 23 lines together, `wc -l`); the budgets step is empty.

## 11. Where the methodology was wrong or silent

Rules not followed:
- **Relaunch fresh after a container restart** (§2.1): not followed after any of the four restarts
  (D-112, D-116, D-129, D-143); agents were resumed with a notice to trust nothing outside their
  worktree and to read uncommitted edits first. No loss was traced to the resume. The rule is wrong
  for this environment, or at least untested against the practice that replaced it.
- **Handoff per director session with start and end** (§12, template): one file with six addenda.
- **Spec-check output under `tasks/spec-checks/`** (planner brief): all 14 wrote into task files.
- **ADR-0005's stage-1 list**: T-033 and T-041 not run, the stage closed without amending it (D-142).
- **`merged → evaluated`** (§3): no transition written after the evaluation confirmed the items.

Rules that cost and changed nothing:
- **The 75 % Extend row** (§11): four rows, each on time, none capping or moving work; 89–98 % of
  each task's spend came before the round-1 verdict, so the moment the rule picks is after the money.
- **`--acc-defects` at a failing join (interim)**: never triggered; the one failing join (T-021) is
  of a kind the interim wording does not cover.

Gaps that let something through or that no rule addresses:
- **Two tasks in flight that meet in a shared trait or document** (D-128, D-132): §5.2 compares
  `touches` only. A spec check of a third task found it by reading.
- **A ruling that widens `touches`** (D-134) and **a state change recorded in metrics but not in the
  front matter** (D-139): two places, one read by the tool.
- **The implementer-first arm's gate never sees the acceptance tests before the join** (D-108).
- **A criterion tested through a Rust API without the API in the Interface** (D-136).
- **Pinned text is trusted by the copy rule**: 3 of 5 defects were sentences the spec or director
  pinned (section 1, row 5). The spec check writes sentences and nobody but the reviewer runs them.
- **Differential depth**: where an external reference exists (git), hand fixtures (T-019 review)
  missed what a 600-pattern fuzz found (NEW-1).
- **Location-dependent tests** (NEW-3) and **evaluator access to fixture data** (D-141): silent.
- **Decision ids**: nothing checks that a cited D-NNN exists (D-104).
- **Run times**: `metrics add` accepted a run with `started_at` = `ended_at` and 291,166 tokens (R-180).
- **Disk** limited dispatch more than the builder cap did (D-097, D-105), and gate time grew 2.4×;
  no rule or metric covers either.
- **Review classes**: 21 of 40 round-1 findings are document sentences, and the review front matter
  has no field for them, so this is counted by reading.

Open questions (§15) the data answers: **O-3** (re-review after a queue failure): no queue failure
in the stage, still open. **O-8**: `acc_defects` filled on 0 of 16 joins; T-033 is still the answer
and still unmerged. **O-12**: independence still unchecked; the odd-arm order held in T-021 and T-023
(history). **O-11**: unchanged.

## 12. Proposed changes for v1.3

| # | Change (file, section) | Evidence | Expected effect and cost | What would show it was a mistake | Removes |
|---|---|---|---|---|---|
| 1 | **Drop the 75 % row** (METHODOLOGY §11, §2.1). Keep the queue's `budget` check and the row before dispatching a run of a task at or over its ceiling | 4 rows, rework 18,409–74,183, every task 7–15 % under N; T-020 was at 105.5 % before the verdict (`S extends`) | One director step and four rows less per stage; no change in spend | A post-round-1 rework over 150,000 tokens that no row caught before it ran | the 75 % rule |
| 2 | **Ceilings**: high 1,100,000 (pooled p75 1,076,226), medium **800,000** (pooled p75 781,214, n=6), low 150,000 (pooled p75 124,444); restate the drafts' v1.1 budgets (500k/850k in T-022, T-024–T-031) at the next spec check | `S ceilings`; draft front matter | Medium tasks stop overrunning a ceiling that 3 of 6 exceeded or touched; no effect on work | Medium tasks in stage 2 finishing well under 700k, or overruns of 800k | — |
| 3 | **Spec check reads the in-flight branches** (§5.7, planner brief): list every shared trait, type, document and command list a ready task meets with any task in flight, and pin the Rust signatures of any criterion tested through a Rust API | D-117, D-128/D-132, D-136 (four misses reaching builders; R-210, R-213, R-217 49,370 tokens) | Fewer late rulings; cost a few thousand tokens per check | A stage-2 conflict of this kind that a check had the branches to see | — |
| 4 | **Two-place rulings become one** (§2.1): a ruling that changes `touches`, `depends_on` or state is a front-matter diff in the same commit; until T-043, the director runs `cargo xtask queue --dry-run` or reads the front matter before queueing | D-134, D-139 (two refusals, both director) | Refusals of this kind go to 0 | A refusal of the same kind | — |
| 5 | **Restart rule follows practice** (§2.1, handoff template): after a container restart, `uptime -s` and `git status` in every worktree, then resume each agent with a restart notice telling it to trust nothing outside its committed worktree and to read its uncommitted edits first | D-112, D-116, D-129, D-143: 4 of 4 resumed, nothing committed lost, one gate attempt and one report lost | Rule and practice agree (retro-1 found them apart too) | Work lost or a wrong state trusted after a resumed restart | "relaunch fresh" |
| 6 | **Handoff**: one file per day with timestamped addenda, each naming the stop or restart before it; drop "director session start and end" | One file, six addenda; the session times were never written; the director's cost is not measured anyway | Rule matches the record | A resume that needed a session boundary the file did not give | session start/end |
| 7 | **Odd arm: a trial join before the implementer reports** (§6, implementer brief): the implementer merges its tests branch into a scratch branch and runs the gate there | D-108 (T-021 first join `gate_failed` at `no-skip`); T-021 and T-023 did this unasked (`46bc135`; D-122) | The join stops being the first gate run of the tests; one gate run (about 5–9 min) per odd task | A first join failing for a reason the trial join would have shown | — |
| 8 | **Classify every join that is not green**, not only those with acceptance failures (§13; T-033's scope): `--acc-defects` and a cause (`implementation`, `test`, `environment`) on any record whose `result` is not `joined` | T-021's record (null) against D-108's classification; 0/16 filled | `acc_defects` becomes countable (O-8) | Classifications that cannot be made at join time | the interim wording |
| 9 | **Differential runs where an external reference exists** (§6 self-check, §7): for an item with a reference tool (git for ignore rules, rustc for the corpus), the test author or reviewer runs a seeded differential fuzz and logs seed, count and differences | NEW-1: 12 hand fixtures (T-019 r1) against 600 fuzzed patterns (8 differences found) | Lists of "differences" become complete or stated as sampled; cost one script per such item | Fuzz runs that find nothing the hand cases did not | — |
| 10 | **Pinned sentences are probed before acceptance** (§3 `specified`, planner brief): every Pinned sentence that states a figure, a limit or a list carries the command and output that show it at spec-check time, or is marked "to be measured by the implementer" | 3 of 5 review defects were pinned sentences (T-019 #2, T-020 #1, T-023 #1); D-131 (figures called measured that were not) | Fewer defects of class D at review; cost inside the spec check | Review defects in pinned sentences that carried evidence | — |
| 11 | **Review front matter counts document sentences** (`templates/review.md`, §13): `doc_sentences: N` among the findings | 21 of 40 round-1 findings, counted here by reading | The largest finding source becomes a metric | — | reading-based classification |
| 12 | **Mechanical record checks** (gate `hygiene` or `metrics`): every `D-NNN` cited in `tasks/`, `reviews/`, `docs/` has a row; `metrics add` refuses `started_at` = `ended_at` with tokens over 10,000 | D-104 (14 citations, no row); R-180 | Two silent record errors become impossible | False refusals | — |
| 13 | **Evaluator brief names readable fixture data and runs in a checkout other identities can reach** (§9, evaluator brief), until T-051 and T-052 merge | D-141 (R-222, 20,610 tokens); NEW-3 | No evaluator ruling needed for fixture data; the scorecard reads the same in the evaluator's clone | — | — |
| 14 | **Track gate time and disk per worktree** (§13): queue `gate_seconds` reported per stage; the debug-info profile change measured in D-097 (2.5 GB instead of 6.3 GB per build) drafted as a high-tier gate-path task before the UI adds egui | gate 241 → 574 s; disk limited dispatch (D-097, D-105) | Concurrency up to the cap; cost one high-tier task | Disk not binding in stage 2 without it | — |
| 15 | **Stage scope changes get a row** (§14 / ADR-0005 practice): a stage closes only after a row lists what of its ADR list was not done and where it went | D-142 vs ADR-0005 (T-033, T-041) | The ADR and the record agree | — | — |

Not proposed: loosening round-1 request-changes for test gaps (section 4: 9 of 10 were surviving
mutants; the extra rounds cost 208,168 in all); changing the builder cap (it was not binding).

### Recommendation on stage 2

The 23 planned items (VERIFIED `S scorecard`; drafts' `scorecard_items` by `grep`): `ui.*` 8 (no
draft); T-022 `ws.find-in-files`, `ws.replace-in-files` (medium, depends on T-019, T-021); T-024
three `diag.*` (high); T-025 `edit.indent`, `edit.comment-toggle` (medium; it also claims
`edit.newline`, which **is not a scorecard item**, `ls scorecard/items`); T-026 three `fmt.*`
(medium, after T-024); T-027 `io.external-change` (high); T-028 `text.rope` and the budgets step
(high); T-031 `cmd.palette`, `keymap.*` (medium, after T-023, T-025, T-026). The other drafts carry
no item: tooling T-033, T-043, T-041; limits T-044–T-048; evaluator findings T-050–T-053; deferred
T-007, T-009, T-029, T-030, T-034.

Recommended order (PROPOSED; INFERRED from dependencies and the stage's costs):
1. First, the cheap items that the stage showed cost director errors or evaluator rulings: T-052 and
   T-051 (low), then T-043 and T-033 (refusal records, join classification; changes 4 and 8), before
   the next joins. T-041 last, as D-098 says (owner told first).
2. A UI foundation task (high: the egui dependency; window, dispatch-only, and the means by which an
   evaluator observes UI behaviour, which does not exist yet), with an ADR for that observation
   interface. Proposal 14 before or with it, since egui will add build size.
3. In parallel as disk allows: T-028 (the only route to C6), T-024, T-022, T-025 (fix its items),
   T-027. Then T-026, then T-031.
4. UI panels each after the feature they show: editor view, tabs, status bar, project panel after
   step 2; search panel after T-022; diagnostics panel after T-024.

Cost, at this stage's medians (INFERRED): the seven feature drafts, 3 high × 810,888 + 4 medium ×
686,472 = 5,178,552, plus seven spec checks at 326,683 = 2,286,781: about **7.5 M** before any UI
task, follow-up, tooling or evaluation; stage 1 cost 9.2 M in all.

**D-140** (a thin UI shell over every verified command first, then depth; a UI item counted only
when the evaluator observes behaviour). What the record supports:
- The architecture makes a shell cheap to keep honest: the UI may only dispatch registry commands
  (AGENTS.md golden rule 1; gate step `ui-commands`); all 30 commands are verified through the CLI,
  and the evaluator confirmed every parameter changes a result ("Wired but inert", none found). A
  shell over them cannot fake a behaviour the CLI lacks.
- The failure D-140 guards against, PhotoCraft's parity count measuring wiring, is what C4's
  evaluator confirmation already prevented here for 41 items; counting a UI item only on observed
  behaviour extends that rule rather than inventing one.
- It tests a real question (surface-first under a behaviour scorecard) that this experiment's order
  never tested.

What speaks against it, or must be settled first:
- **No observation interface exists.** The evaluator brief names a "UI snapshot mechanism"
  (`methodology/roles/evaluator.md:16`), but no `ui-snapshot` code exists (`grep -rn ui-snapshot
  xtask/src` finds nothing; VERIFIED) and three evaluations did not use it; "the evaluator observes
  the behaviour" has no defined public interface yet. Without one, the
  counting rule cannot be applied; this is an ADR, not a task detail.
- **One crate, one shell**: a shell "over every verified command" is one change to `crates/kiln-ui`
  that every later UI item edits; §5.2 forbids overlapping `touches`, so UI items serialise behind it
  (CHARTER decision 6 asks for one module per group). Throughput in this stage was already limited by
  serial chains and disk.
- Two of the eight UI items (search panel, diagnostics panel) need features not built (T-022, T-024),
  so "every verified command" covers at most six UI items first.
- **Confounding**: it changes the order and introduces the first L5 work and the egui dependency at
  once; a result could not be attributed to the order. The stage gives no data on UI cost; estimates
  from it are not estimates for UI work.
- C6 still has no enforced budget; T-028 is independent of the UI and should not wait for it.

Assessment (INFERRED): D-140 is consistent with the record if its first task is the observation
interface (step 2 above) and the shell is cut so that each UI item has its own module; as a single
shell over everything first, it would serialise the stage and could not be counted.

## Budget recalibration

Rule as retro-1: p75 of the tier's builder and review tokens per merged task, rounded up to 50k.
Stage only: high 866,379 (n=4) → 900,000; medium 749,633 (n=3) → 750,000; low 114,450 (n=2) →
150,000. Pooled with iteration 1: high 1,076,226 (n=10) → 1,100,000; medium 781,214 (n=6) →
800,000; low 124,444 (n=5) → 150,000. VERIFIED `S ceilings`. I propose the pooled figures (change 2):
the stage alone has n ≤ 4 per tier and no task of the UI kind.

## Where the director's record disagrees with the data

- **D-104 does not exist** in `docs/decisions.md`; T-020's file and review cite it 14 times (`S decisions`).
- **R-180's run record** has `started_at` = `ended_at` (wall 3 s) for 291,166 tokens; the 23:26Z
  handoff addendum lists it as running (`S completeness`).
- **T-021's first join** is recorded with `acc_defects: null`, although D-108 classified it as one
  acceptance-test defect (`S joins`).
- **D-142 closes the stage** that ADR-0005 defines as including T-033 and T-041; neither ran and no
  row amends the ADR. The record says "ten tasks merged in it"; that is right, but the stage's own
  list was not completed.
- **No task reached `evaluated`**, although D-141 confirmed all 13 items (`S completeness`).
- **The handoff** does not carry the director-session start and end times that v1.2 change 13 put
  in the template (`handoffs/`).
- **Restarts**: §2.1 says relaunch fresh; the record shows four resumes. D-112 explains the choice;
  the methodology was not changed to match.
- The **cost and percentage figures** in D-105, D-107, D-109, D-114, D-115, D-119, D-121, D-123,
  D-127, D-133, D-137 and D-139 agree with `metrics/tasks.jsonl` to the token (`S extends`, `S tokens`).
- The brief's "nine product tasks" counts two documentation-only low-tier tasks (T-039, T-042).

## 13. Uncertainty

- **Samples**: 7 reviewed tasks, n=3 per medium arm pooled across iterations; 0 acceptance-caught
  defects, so C3 and C5 have no events.
- **Classification**: the document-sentence split of findings (21 of 40), the director-error table
  (24, its causes and "reached") and the evaluator-vs-review attribution are my readings.
- **Spec-check blockers** (65) are the decision rows' counts; I did not recount them in the task files.
- **Idle time** uses booked start and end times; R-180's are wrong, and the director's own work
  between runs (joins, queue, rulings) is not recorded, so "no run active" is not "nothing done".
- **The usage-limit windows** are from decision rows (D-092, D-112, D-129) and, for D-143 and the
  18:15Z restart, from the coordinator's message and D-143; the reset times are stated, not measured.
- **R-197**: I could not tell whether a run happened.
- **CI runs** on GitHub Actions are not visible from here; red-main 0 rests on the absence of incidents.
- **What the evaluator did not run** (races, non-root identity, `syn`, release figures) is also
  unverified by any review in this stage except races (T-038 r1) and an unprivileged uid (T-036 r1).
- **Model per run** (O-11): not visible; not used.
