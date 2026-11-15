---
iteration: 2                # second stage of iteration 2 (ADR-0007 as amended by D-147), up to cut 1 (D-170)
run: R-254
methodology_in: v1.3
methodology_proposed: v1.4
main_sha: eefd9d0cfb58aeead65f59cf0a7150477d37870b   # D-170; main later gained four ledger commits (30917a5 D-171 … 60c20fa)
claims: {C1: insufficient, C2: supported, C3: insufficient, C4: supported,
         C5: insufficient, C6: refuted, C7: supported, C8: supported}
         # C2 supported for "main stays green", not for "the gate guards the window" (section 5);
         # C4 supported with one confirmation resting on evidence outside ADR-0008's interface (section 4);
         # C6 refuted as a gate (no latency budget enforced); C8 supported for UI behaviour, the observation layer itself needed a further oracle
---

# Retrospective, iteration 2, stage 2 to cut 1

## How to read this report

Labels as in retro-2s1. **VERIFIED**: I ran the command named beside the number and it printed it.
**INFERRED**: my reasoning from verified facts; the reasoning is given. **PROPOSED**: a change for
methodology v1.4, for the director to decide.

Commands, from the repository root (python3 standard library only, read-only):
- `M <section>` = `python3 -I retrospectives/scripts/iter2s2_metrics.py <section>`; sections
  `completeness tokens budgets joins queue reviews rework time lead gates estimates decisions acks speccheck`.
- `G` = `bash retrospectives/scripts/iter2s2_git.sh` (range `43e9162..eefd9d0`, D-143 to D-170; some
  lines from `9d5f6d6`, the D-147 commit).
- Stage-1 and iteration-1 comparison figures are retro-2s1's and retro-1's (their commands); I did not
  recompute them, because `iter2s1_metrics.py` sums every `iteration: 2` record and would now include
  this stage.
- Review classes and the director-error table are my readings of the review files, task files and
  decision rows (INFERRED classification of VERIFIED rows).

Scope: from the plan of the stage (R-225, started 2026-11-14T18:36:35Z; accepted by D-147, commit
`9d5f6d6` 19:06:27Z) to cut 1 (T-067 merged 2026-11-15T05:49:01Z; D-170 commit `eefd9d0` 05:54:40Z).
Runs R-223 to R-253 as the brief names them; R-223 (the stage-1 retrospective) and R-224 (the v1.3
edit) are shown separately where it matters. Three product tasks merged (T-055 `kiln script`, T-056
editor view, T-065 window) and one publish (T-067).

**Drift.** The brief names head `eefd9d0`; main is now `60c20fa`, four ledger commits later (`30917a5`
D-171, `93ad8ed` D-172, `e32c1ed` D-173 "owner: skip signed installers for now", `60c20fa` the
director's commit of this run's and R-255's work in progress; `git log --oneline eefd9d0..HEAD`).
Nothing counted here changes with them; D-171 is used for section 12; D-173 came after my start and is
not used. Files that another run (R-255) adds under `docs/adr/0009-*` and `tasks/T-069…` after my
start are not read.

## Data

| Check | Value | Command |
|---|---|---|
| Run records R-223..R-253 | **31**, no id missing: analyst 1, planner 5, implementer 7, test author 8, reviewer 9, evaluator 1; 28 completed, 3 request_changes, 0 blocked; all `iteration: 2`; `missing_context` none on all | VERIFIED `M completeness` |
| Runs without a log entry on main | **R-235** (T-055 reviewer, 12,020 tokens, the "first confirmation" of D-153): its id appears in no task, review, evaluation or decision file. D-154 says its record lies on `backup/T-055-linear1`, "not used". §12: no entry ⇒ run marked incomplete; it is booked `completed` | VERIFIED `M completeness`, `M acks` |
| Runs with wall < 60 s | R-236 47 s, R-245 46 s (confirmations; plausible); no start = end | VERIFIED `M completeness` |
| Tier field of run records | R-240 (T-056 test author) `medium`, all other T-056 runs `high`. D-158 says "the two runs already booked carry tier medium"; R-239 was booked `high` in the re-tiering commit `99e9677` | VERIFIED `M completeness`; `git log -S'"run":"R-239"'` |
| Transitions | T-055 20, T-056 11, T-065 11, T-067 6; every product task passes `self_verified → tests_joined → reviewed` after each rework. One join without its transitions: T-056's third join (01:20:37) ran while main's state was `queued`; D-162 says so | VERIFIED `M completeness` |
| Join records | **10** (T-055 5, T-056 3, T-065 2); 2 `acceptance_failed`, 8 `joined`; `acc_defects` null on all 10 join records; the two failing joins each have a separate `join_defects` record (counts 1 and 0); the causes are in D-149 and D-151 only | VERIFIED `M joins` |
| Queue records | 6: 4 merged, 2 conflicts (T-055, T-056). **Two refusals are in decision rows only** (D-154 T-055 at `join`; D-162 T-056 at `front-matter`): T-043 is not merged | VERIFIED `M queue`; D-154, D-162 |
| Reviews | 8 files: 3 tasks × 2 rounds + 2 "round 3" confirmations (T-055-r3, T-056-r3); each with `findings`, `doc_sentences`, `mutants`. T-055-r3 carries round 2's mutant counts over (not re-run) | VERIFIED `M reviews` |
| Decision rows D-143..D-172 | 30, none missing; no D-NNN cited in tasks, reviews, evaluations, ADRs, handoffs or STATE without a row | VERIFIED `M decisions` |
| Builder and reviewer runs that acknowledge rulings | **23 of 24** by word match; the one without is R-235, which has no entry | VERIFIED `M acks` |
| Handoffs | `handoffs/2026-11-14.md` (5 addenda, 19:10–23:45Z) and `handoffs/2026-11-15.md` (6 addenda, 01:55–16:20Z): one file per day, as v1.3 change 6 says | `ls handoffs`; `grep -n '^## ' handoffs/2026-11-1[45].md` |
| Methodology text changed in the stage | none after `40947e8` (v1.3 in force) | VERIFIED `G` |

The director's cost is not measured (METHODOLOGY §13); it is not zero. Time "with no run active"
contains the director's joins, rulings and queue runs.

## Metrics (METHODOLOGY §13)

| Metric | Value | Command |
|---|---|---|
| Lead time `ready` → merged | T-055 **3.02 h**, T-056 **3.21 h**, T-065 **2.64 h** (all high; median 3.02); T-067 0.53 h. Stage 1 high median 2.07 h (retro-2s1) | VERIFIED `M lead` |
| First-pass queue success | **2/4** by record (T-065, T-067); T-055 and T-056 each needed three attempts (a conflict and a refusal each); 8 queue attempts for 4 merges. Stage 1: 10/10 by record, 7/10 with refusals | VERIFIED `M queue`; D-153, D-154, D-162 |
| Queue gate time | 1,074.9 s (T-055, gate run with main's and the task's `xtask`, a gate-path change), 548.3, 560.7, 720.4 (T-067); median **640.6 s**. Stage 1 median 486.2 s, max 574.2 s | VERIFIED `M queue` |
| Gate attempts per task | T-055 5, T-056 3, T-065 2 (median 3; stage 1 median 3) | VERIFIED `M gates` |
| Acceptance-caught defects | **1** (T-055's first join, `join_defects` count 1, cause `implementation` with its origin in the spec, D-149); 1 test defect (T-055's third join, count 0, cause `test`, D-151). T-056 and T-065: 0 acceptance failures at any join | VERIFIED `M joins` |
| Review findings, round 1 (3 reviews) | defect **10**, spec gap 7, test gap 11, nit 3; `doc_sentences` 3 (field). Round 2 (3): 0 defects, 2 test gaps (T-055), 5 nits; `doc_sentences` 4. Round 3 (2 confirmations): nothing | VERIFIED `M reviews` |
| — behaviour defects among round-1 defects | **7** (T-055 1, T-056 2, T-065 4); 3 are document sentences. Stage 1: 0 of 5 | INFERRED from the findings tables |
| Escaped defects (found after the task merged) | From T-055 (high, independent): the row-text defect (found in T-056 by its implementer and reviewer; T-066 `origin_task: T-055`, `found_by: implementer`, which is not a §13 value; `task:T-056` would be); NEW-1 (evaluator; added to T-066); the inert `scroll` left/right (evaluator, T-068 `origin_task: T-055`). From T-056 and T-065: NEW-2 to NEW-4 (document sentences, into T-053) | `evaluations/iter-2-cut-1.md`; front matter of T-066, T-068 (`G`-style loop over `tasks/`) |
| Rework ratio | task files first committed in the stage with `origin_task`: T-065 (a split, not a defect), T-066, T-068 | front matter, `git log --diff-filter=A` |
| Review rounds per task | 2 each; plus a confirmation record for T-055 and T-056 filed as `r3` (D-153 calls this a use of round 3 the methodology does not provide) | VERIFIED `M reviews` |
| Acceptance kill rate | round 1: 34/56 = **61 %** (any test 44/56); round 2: 11/13 = 85 %. Stage 1: 87 % and 54 %. T-065's `killed_by_acceptance` 13 counts 9 kills by the manual window procedure, which the gate does not run (section 5); with gate tests only, round 1 is 25/56 = 45 % (INFERRED from the T-065 r1 mutant table) | VERIFIED `M reviews`; `reviews/T-065-r1.md` |
| Known defects merged by decision | 0 behaviour defects open at merge. Limits stated and accepted: right-to-left rows (T-056, into T-066), a display that goes away (exit 1), screens smaller than the window (T-065); a keystroke on a 1,048,576-char highlighted document takes 4.8 s in a debug build, no bound promised (D-161) | D-161, D-167 |
| Tokens per merged task | T-055 997,800; T-056 1,190,305; T-065 887,544 (high; median **997,800**); T-067 none. Stage 1 high median 810,888 | VERIFIED `M tokens` |
| Share verified | **44/64 = 68.8 %** on main (D-170); 3 new | `git show eefd9d0:docs/scorecard.md` |
| Red-main minutes | 0 recorded (no `incidents/`) | `ls incidents` (absent) |
| Budget overruns | **1 of 3**: T-056 1.082× its ceiling (1,190,305 of 1,100,000), inside the cap of D-159 (1,300,000), the row written before the rework | VERIFIED `M budgets`; `G` (D-159 commit 00:32:44 < R-242/R-243 start 00:33:18) |
| Gate time per stage (v1.3 change 14) | queue median 640.6 s against 486.2 s; implementer gate runs 515.2–758.9 s (T-055 639, 551, 551.6; T-056 758.9, 515.2, 545.9; T-065 703.5, 572.1) | VERIFIED `M queue`; task logs (`grep -n 'gate attempts' tasks/T-05[56]*.md tasks/T-065-window.md`) |

## Further numbers

| Number | Value | Command |
|---|---|---|
| Tokens R-223..R-253 | **4,645,569** (= D-170's figure); product tasks 3,075,649; `iter-2` overhead **1,569,920 = 33.8 %**: R-223 retrospective 279,971, R-224 methodology edit 179,361, R-225 plan 351,751, spec checks 432,716 (R-237 366,707, R-238 31,240, R-246 34,769), R-253 evaluation 326,121. Without R-223 and R-224: overhead 1,110,588 of 4,186,237 = 26.5 %. Stage 1: 38.5 % | VERIFIED `M tokens` |
| — by role (product tasks) | T-055: implementer 356,057 (3 runs), test author 329,386 (4), reviewer 312,357 (4); T-056: 429,742 (2), 404,915 (2), 355,648 (3); T-065: 320,372 (2), 192,777 (2), 374,395 (2) | VERIFIED `M tokens` |
| — by role (all 31 runs) | planner 963,828; implementer 1,106,171; test author 927,078; reviewer 1,042,400; evaluator 326,121; analyst 279,971 | VERIFIED `M tokens` |
| Per new verified item (3) | product tokens **1,025,216**; with overhead from R-225 1,395,412; all 31 runs 1,548,523. Stage 1: 225,783 (item tasks only) and 707,775 (all); iteration 1 about 1.83 M per item claimed | VERIFIED `M tokens`; retro-2s1 |
| — per item by task | T-056 (2 items) 595,153 each; T-065 (1) 887,544; T-055 claims none (the observation interface) | INFERRED division of `M tokens` |
| Review rounds' cost | round 1 884,219 (268,991; 314,479; 300,749); round 2 134,246; confirmations 23,935 (R-235, R-236, R-245) | VERIFIED `M tokens`, `M reviews` |
| Post-round-1 rework | **457,405 = 14.9 %** of product tokens (T-055 119,421, 12.0 %; T-056 151,895, 12.8 %; T-065 186,089, 21.0 %). Stage 1 8.5 %, iteration 1 24.6 %. Before round 1, T-055 also had 51,408 of rework after its failed first join (R-228, R-229) | VERIFIED `M rework` |
| All run records of the experiment | 30,203,036 tokens, 248 runs (= D-170) | VERIFIED `M tokens` |
| Builder concurrency | max **2** builder runs open at once (cap 3); hours at 0/1/2 builders 4.00 / 3.83 / 1.43 | VERIFIED `M time` |

## 1. Did each v1.3 change do what it was for?

| # (CHANGELOG) | Change | What happened in the stage | Verdict |
|---|---|---|---|
| 8 | Every join that is not green is classified (count and cause) | Both non-green joins were classified at once: T-055 join 1 (19:50:00) → `join_defects` 1 at 19:53:09, cause `implementation` (origin in the spec) in D-149; join 3 (20:57:28) → `join_defects` 0 at 20:59:29, cause `test` in D-151. The `join` records keep `acc_defects: null`; the cause exists only in decision rows (interim until T-033) | **Worked**; the cause is not machine-readable (VERIFIED `M joins`) |
| 9 | One seeded differential run by the reviewer in round 1 where a reference tool exists | No item of the stage has an external reference tool (git, rustc); the reviewers made differential runs anyway against internal references: T-055 r1 against `kiln-cli run`/`control` (120 scripts, 3,934 lines, 0 differences); T-056 r1 against the reviewer's own model of the Input table (420 sessions, 16,800 steps; it found 5,833 misaligned row steps = the row-text defect, and killed surviving mutant M17); T-065 r1 window against `kiln script` (70 scripts, 2,170 lines; it found findings 1–3) and base against head (100 scripts, 0 differences). **These runs found or generalised 4 of the 7 behaviour defects** (T-056 #1, first seen by its implementer; T-065 #1–#3) | **Worked, beyond its wording**; the rule as written ("reference tool already on the machine") would not have required any of them (reviews; INFERRED count) |
| 11 | `doc_sentences` in review front matter | Filled on 8 of 8 reviews. Round 1: 3 of 31 findings by the field (stage 1: 21 of 40 by reading). By my reading the reviewers counted narrowly: T-055 r1 #10 and #12, T-056 r1 #8, T-065 r1 #9 are also about document sentences, which would make 7 of 31. Either way the share fell: the stage's defects are behaviour | **Worked as a field; definition ambiguous** (VERIFIED `M reviews`; INFERRED re-reading) |
| 3 | Spec check reads branches in flight; pins Rust signatures of criteria tested through a Rust API | No task was in flight during any spec check (R-237: "No task is in flight"; tasks ran one after another, max 2 builders). Both specs pin `kiln_control::Host`'s signatures and state "Rust API used by tests: none"; no builder asked for an API (no D-136 kind of question). The checks listed collisions with drafts instead (B12 T-057–T-063 panel files; B14 T-066) | **Untested** for in-flight branches; no API miss (task files) |
| 10 | Pinned figures, limits and lists carry evidence at the spec check, or "to be measured by the implementer" | Figures with evidence held: lock 156 → 326 packages with no version change (R-237 E-a, R-246 E-g; confirmed by T-065 r1 `lockcmp.py`), exits without a display. A figure "to be measured by the implementer" failed: T-056's settle count was to be measured "on the acceptance scripts", which the implementer may not see (its QUESTION, `tasks/T-056-editor-view.md:858`); its own figure (focus 3) was false on the acceptance scripts (focus 13), T-056 r1 defect #3. A pinned sentence without its condition failed: "a desktop window of 1280×800 points" (T-065 r1 #9, smaller screens) | **Worked where evidence was given; "to be measured by the implementer" can be unmeasurable under independence** (reviews; task files) |
| 1 | No row at 75 % of a ceiling | T-055 stood at 79.9 % at round 1 with no row and ended at 90.7 %; T-065 63.8 % → 80.7 %. Nothing was lost by dropping the row | **Worked** (VERIFIED `M budgets`) |
| 1 (kept part) | Row before a run past the ceiling (the one cap row, D-159) | T-056 stood at 1,038,410 (94.4 %) after round 1. D-159 (commit 00:32:44) extended it to 1,300,000 34 s before the rework runs started (00:33:18), chose extend over split with a reason, and estimated rework plus round 2 at 220,000; the actual was **151,895**, ending at 91.6 % of the cap and 108.2 % of the ceiling. The row recorded a choice that was made, and the queue's `budget` check needed it | **Worked**: one row, written in time, with a cap that held (VERIFIED `M budgets`, `G`) |
| 2 | Ceilings 1.1 M / 800k / 150k | All three tasks high; one over (T-056). The medium ceiling was never applied: T-056 was specified medium and re-tiered to high at its join (D-158) at 90.5 % of 800,000 | high ceiling tight for UI tasks (n=3; median 997,800) (VERIFIED `M budgets`, `M tokens`) |
| 4 | A ruling that changes `touches`, `depends_on` or state is a front-matter edit in the same commit | Done in each ruling commit (`9d27ad0` D-149, `1ed8d14` D-150, `99e9677` D-158, `88aa454` D-159, `b49dc24` D-165: `state` or `tier`/`budget_tokens` changed in the same commit). But the D-159 instruction to give **the branch's** copy main's front matter caused one refusal and one conflict (D-162) | **Followed on main; harmful on the branch** (VERIFIED `git show` of those commits; D-162) |
| 5 | Resume stopped agents after a stop or restart, with a restart notice | No usage-limit stop and no restart between D-147 and D-170 (evaluation setup: boot 18:15:21, no restart during R-253; next boot 16:09:46 on 11-15 with nothing in flight, D-172) | **Untested** in the stage |
| 6 | One handoff per day with timestamped addenda | Two files, 5 and 6 addenda; each names state, next step, runs booked, free disk | **Worked** (`handoffs/`) |
| 7 | Odd arm: implementer runs the whole gate on a local merge of its tests | No odd-arm task (T-055 high, T-056 even and later high, T-065 high) | **Untested** |
| 12 | Pending T-043 (D-NNN rows, start = end, state line) | T-043 not dispatched. No missing D-NNN row and no start = end run in the stage; one state line left at `queued` during a re-join (D-162). D-147 deferred T-043 "at the next queue refusal or record error of the kinds it prevents"; **two refusals came (D-154, D-162) and T-043 stayed a draft** | Not delivered; its trigger was met and not acted on (VERIFIED `M queue`; `grep state: tasks/T-043-*.md`) |
| 13 | Evaluator in a clone others can reach; named fixture data | R-253 worked in a clean clone under 0755 directories, `scorecard` passed there. But the clone held `docs/decisions.md`, and a `grep` over `docs/` printed 300 chars of D-165 to the evaluator, which it disclosed | **Worked**; the clone exposes records the evaluator may not read (evaluation "Setup") |
| 14 | Gate time per stage | Reported above; queue median 486 → 641 s | **Worked** |
| 15 | A stage closes with a row on its ADR list | The stage is not closed; D-170 lists what was deferred (T-051, T-052, T-053, T-064, T-043, T-033, T-054, T-041, T-066, T-068) | Not yet applicable |

## 2. Joins

| Task | Join (UTC) | Acceptance tests | Failing at the join | Classification | Cause |
|---|---|---|---|---|---|
| T-055 | 19:50:00 | 18 | 1 (`ac6_frame_counts_add_up`) | 1 defect, `implementation`, origin in the spec (D-149) | the implementer ran unreported frames after each successful control request |
| T-055 | 20:11:20 | 19 | 0, gate green | — | rework on n4, n8 |
| T-055 | 20:57:28 | 26 | 1 (`w3d_scrolled_list_reports_only_visible_text`) | 0 defects, `test` (D-151) | the test assumed `Open` visible at 200×200 |
| T-055 | 21:03:47 | 26 | 0 | — | test fixed (w7) |
| T-055 | 21:29:26 | 26 | 0 | — | rebuilt branch after the queue conflict and refusal (D-154) |
| T-056 | 23:25:41 | 22 | 0 | — | first join green |
| T-056 | 00:53:52 | 28 | 0 | — | after round-1 rework |
| T-056 | 01:20:37 | 28 | 0 | — | rebuilt branch after front-matter refusal and conflict (D-162) |
| T-065 | 02:23:41 | 6 | 0 | — | first join green |
| T-065 | 03:43:03 | 7 | 0 | — | after round-1 rework |

Every acceptance test failed on base at every join (`acc_fail_base = acc_total`, kind `fails`); guards 2
(T-055), none in T-056 and T-065. Two of the ten joins exist only because the director rebuilt a
branch. VERIFIED `M joins`.

**What D-149's join caught (VERIFIED, task file and D-149).** The spec's Interface said frames run after
actions and frame totals add up; its AC-2 expected a snapshot right after control `doc.open`s to show
them, without saying when a control request's effect is shown. The implementer resolved the
contradiction by running unreported frames after every successful control request, wrote a unit test
asserting that (`frames_run_after_successful_control_requests_only`), and **asked about it as its
QUESTION (1)** in its log (`tasks/T-055-ui-script-observation.md:470-472`). The independent test
`ac6_frame_counts_add_up` took the Interface at its word and found 21 frames between two snapshots
against 20 reported. The ruling n4 went against the implementer's choice.

**Would a review have caught it? (INFERRED.)** Very likely: the question was in the log before the
join, and §2.1 requires the director to answer builders' questions before the join; the director
joined at 19:50:00 and ruled at 19:53:00 (`9d27ad0`). The round-1 reviewer checked exactly this
property through the binary after n4 ("every snapshot's total equals the start frames plus the sum of
reported frames", `reviews/T-055-r1.md:71-74`). So the test caught a deviation the implementer had
disclosed and its own test pinned, not a defect hidden from it.

**What it changes for C3 (INFERRED).** It is one event in which a different agent's test caught that an
implementation diverged from the spec while the implementer's own test asserted the divergence. It is
not evidence that independent tests catch what nobody saw. **It is also not the first such event in the
experiment, as D-149 states:** D-029 (iteration 0) records that "at the T-004 round-2 join an
independent acceptance test caught an implementation defect the implementer's own 278-test green gate
had missed", and the T-004 join record of 2026-11-13T01:57:08Z has `acc_defects: 1` with that note
(VERIFIED `M joins`, the list "with acc_fail_first_join > 0"; `grep '^| D-029 ' docs/decisions.md`).
Against it, in this stage: T-056's first join passed 22 of 22 and T-065's 6 of 6, and the reviews of
those same heads found 2 and 4 behaviour defects. No behaviour defect that a review found was caught by
an acceptance test at a join.

## 3. Reviews

| Review | Verdict | d | sg | tg | nit | doc_sent. | mutants in/eq/killed/by acc/surv | tokens |
|---|---|---|---|---|---|---|---|---|
| T-055 r1 (R-230) | request-changes | 2 | 3 | 6 | 1 | 1 | 20/0/13/9/7 | 268,991 |
| T-055 r2 (R-234) | approve | 0 | 0 | 2 | 1 | 0 | 7/0/5/5/2 | 24,841 |
| T-055 r3 (R-236) | approve (confirmation) | 0 | 0 | 0 | 0 | 0 | carried over | 6,505 |
| T-056 r1 (R-241) | request-changes | 3 | 1 | 3 | 1 | 1 | 20/1/16/12/3 | 314,479 |
| T-056 r2 (R-244) | approve | 0 | 0 | 0 | 2 | 2 | 3/0/3/3/0 | 35,759 |
| T-056 r3 (R-245) | approve (confirmation) | 0 | 0 | 0 | 0 | 0 | none | 5,410 |
| T-065 r1 (R-249) | request-changes | 5 | 3 | 2 | 1 | 1 | 18/1/15/13/2 | 300,749 |
| T-065 r2 (R-252) | approve | 0 | 0 | 0 | 2 | 2 | 3/0/3/3/0 | 73,646 |

VERIFIED `M reviews`. R-235 (12,020, the unused first confirmation) has no review file on main.

**Mutants by what killed them (round 1; from each review's table, INFERRED counting).**
- T-055: 20 seeded; 9 killed by acceptance tests, 4 only by the implementer's tests (M8, M14, M15, M16),
  7 survived (M2, M3, M5, M6, M7, M12, M13) → 6 test gaps. Round 2: 5 of the 7 killed by new
  acceptance tests; M2 (`settled` always true) and M5 (colour channels swapped) survived and went to
  T-056's spec (D-152); T-056's acceptance tests now compare colours with `syntax.highlight` (AC-2).
- T-056: 20 seeded, 1 equivalent (M8); 12 by acceptance, 4 only by the implementer's tests (M4, M11,
  M14, M18), 3 survived (M12, M16, M17), one of which (M17) the reviewer's differential run killed. Round
  2: the 3 killed by new acceptance tests.
- T-065: 18 seeded, 1 likely equivalent (M15); by the gate's acceptance tests 4 (M01–M04); by the
  implementer's unit tests 6 (M09–M12, M16, M17; 4 of them also by the window procedure); **only by a
  run in a real window 5** (M05, M06, M07, M13, M18), and 2 survived everything (M08, M14) until the
  reviewer's own window checks. Round 2: the 3 re-seeded (M08, M08b, M14) killed only by the window
  procedure `ac2.sh`.

**Behaviour defects found by reviews and where each came from (INFERRED from findings and rulings).**

| # | Defect | Found by | Origin |
|---|---|---|---|
| T-055 r1 #1 | painted text given to a node by its unclipped rectangle: after a scroll `Path` loses its rows to the list | reviewer R-230 (probe through the binary; M7 survived) | **director's ruling n11** (D-149), which accepted the implementer's proposed rule ("rectangle") as published (D-150) |
| T-056 r1 #1 | a row's text is not the painted chars in reading order (ligatures, combining marks, joiners, Arabic-Indic digits); click columns shifted | the T-056 implementer saw `modifeid` (y5, D-158); reviewer R-241 reproduced and generalised it with its seeded differential (5,833 misaligned steps in 168 of 180 sessions) | **code merged by T-055** (`observe.rs` `merge_rows` sorted glyphs by `x0`; `text_rows` reported `glyph.chr`) |
| T-056 r1 #2 | Enter at the end of a full view does not settle in 60 frames (146 of 5,002 actions) | reviewer R-241 (5,000-key run) | implementation (T-056 `editor.rs` scroll-to-caret) |
| T-065 r1 #1 | the toolkit's quit shortcut closes the window mid-`--script` | reviewer R-249 (window vs headless comparison) | implementation (toolkit default not disabled); the spec left closing during `--script` unspecified (D-163) |
| T-065 r1 #2 | a timed double click differs between window and headless | reviewer R-249 | **director's ruling Q-12 A** (D-163; the spec check R-246 had recommended A and called B "unmeasured") |
| T-065 r1 #3 | a 100-step scroll in a release build hits the settle cap; 22 of 80 lines differ | reviewer R-249 (release build in scratch) | **ruling Q-12 A** |
| T-065 r1 #4 | a non-UTF-8 argument panics | reviewer R-249 | code merged by T-055 (`kiln script` panicked on main already, D-165 b3) and repeated in T-065 |

Document defects: T-055 r1 #2 (a sentence from the spec, "every interactive node has a non-empty
label"); T-056 r1 #3 (the settle figure, see section 1 row 10); T-065 r1 #5 (Pinned (i)'s sentence made
false by #1–#3). So **3 of 7 behaviour defects trace to a director's ruling, 2 to code merged by T-055,
2 to the implementation of the task under review**; none is in L0–L4 (every finding's path is under
`crates/kiln-ui/`, `apps/kiln/` or `docs/`).

**The T-055 defect that T-056's review found after two T-055 rounds passed it.** What finds it: a
property check that the text a snapshot reports for a node equals the text the node shows (`rows` vs
`value`), run over **generated text that includes the classes the shaper treats specially**
(ligature pairs such as `fi`, combining marks, joiners, right-to-left digits). The oracle was in the
snapshot itself (`value`); what was missing was the input. Why the earlier rounds did not (VERIFIED by
grep, reasoning INFERRED):
- T-055's shell shows only ASCII labels and the paths it is given; no `Title` with "modified".
- At T-055's merge (`e34ba1f`) neither `acceptance_t055.rs` nor `script_t055.rs` nor the `kiln-ui` unit
  tests contain a string literal with a non-ASCII char (the one hit is an ellipsis in a message format)
  (`git show e34ba1f:<file> | grep -cP '"[^"]*[^\x00-\x7F][^"]*"'`).
- Neither T-055 review mentions any non-ASCII input (`grep -nP '[^\x00-\x7F]' reviews/T-055-r*.md`
  finds only punctuation and arrows); its differential run compared control lines with `kiln-cli`,
  where the UI's text reading plays no part.
- T-056's spec check R-237 typed `a\tb c é λ ж 中 😀` into `Path` and saw the row equal the text
  (E-f): precomposed chars and no ligature pair, so it passed.
The same pattern repeated once more: NEW-1 (colours one char late after a combining mark at the start
of a row) passed both T-056 rounds, whose colour check used the reviewer's 14 files; the evaluator found
it with a seeded fuzz whose alphabet added combining marks (seed 5: 528 mismatches in 8 of 60 sessions).

## 4. The evaluation at cut 1 against the reviews

R-253 (326,121 tokens) confirmed `ui.editor-view`, `ui.dispatch-only`, `ui.window` and 8 regression
items; 0 rejected; 4 findings, 3 inert request kinds, 4 questions (`evaluations/iter-2-cut-1.md`).

| Finding | Did a review see it? | Why the evaluator found it (INFERRED) |
|---|---|---|
| NEW-1 snapshot colours one char late after a combining mark at a row's start | No (T-056 r1/r2: 445 rows, 26,916 chars, 0 colour mismatches on the reviewer's 14 files) | a seeded fuzz that crossed colour checks with an alphabet of combining marks |
| NEW-2 "the lines consecutive" vs empty lines with no row | No; the spec check R-237 had found the fact (B3) and the Limits state it | reading the sentence against a probe with an empty line |
| NEW-3 shader cache named, not how to turn it off | Partly: T-065 r1 used `MESA_SHADER_CACHE_DISABLE=true` and checked the sentence true | the evaluator's brief required "the documents name one cache and how to turn it off" |
| NEW-4 screens under 200 points | No: b6 measured 1024×768, 800×600, 640×480 | a 160×120 screen |
| Inert: `scroll` left/right on `Editor` and `Documents`; secondary click; clicks on `Documents` entries | No: T-065 r1 scrolled left/right but compared window with headless (equal), not effect | (O5) asks for every control's effect |
| Q1 the no-window line cut before its reason and naming the build machine's registry path | Seen, not flagged: T-065 r1 AC-1 row quotes `kiln: no window: winit EventLoopError: os error at …` "cut at 120 chars" | read as a user would |

Confirmed what reviews had shown: window and `kiln script` byte-equal (reviews 98 scripts / 2,750
lines; evaluator 220 / 46,360 lines, three screen sizes); `ctrl`+`Q` no longer closes; real input
ignored during `--script`; display killed → exit 1; non-UTF-8 → usage error; the Input table against a
model (reviewer 420 sessions; evaluator 460 sessions with a `kiln-cli control` oracle) and replayed logs.
**Not run** by the evaluator: a release build (T-065 r1's defect #3 appeared only there); an ordinary
non-root identity (capabilities dropped instead); IME, drag, double click; T-066, T-048, T-053.

**Was ADR-0008's rule for "observed" applied as written? (INFERRED.)**
- (O3) "for colours, with `syntax.highlight` and the published colour of each highlight kind": for the
  rows of NEW-1 the snapshot disagrees with `syntax.highlight` (528 chars). The evaluator confirmed
  `ui.editor-view` on **screenshots of the real window** (pixels from `Xvfb -fbdir`), which ADR-0008
  rejected as an observation interface (option C), and said that if the snapshot's report counts as the
  behaviour the item is rejected; D-169 took the confirmation. So the confirmation rests on evidence
  outside the agreed interface, by a director's choice the ADR does not provide for.
- (O5) "every control … changes something observable when used, or is reported as wired but inert":
  the three inert kinds were reported and filed (T-068, T-058), and the items were confirmed. Read
  literally, reporting satisfies (O5); the ADR does not say whether an inert control inside an item's
  title blocks the item. Here the inert values (`left`/`right`) are outside what the titles claim.
- `ui.window` (Q-2 A): the window was started under Xvfb in the run, so it is confirmed as ruled.

## 5. The gate and the window

- The 7 tests that `scorecard/items/ui.window.toml` names are all command-line and no-display cases
  (`ac1_no_display_*`, `ac1_usage_errors_exit_2`, `ac1_unreadable_script_exits_2`,
  `ac1_non_utf8_argument_is_a_usage_error`, …): **no gate test opens a window** (VERIFIED `cat
  scorecard/items/ui.window.toml`). The item's title, "opens a window showing the open document", is
  observed only by the reviewer's and the evaluator's procedure under Xvfb (ADR-0008 Q-2 A).
- Of T-065 r1's 17 non-equivalent mutants, 10 are killed by tests the gate runs (4 acceptance, 6 unit)
  and **7 only by a run in a real window** (5 by the procedure, 2 by the reviewer's own checks); in round
  2 all 3 re-seeded mutants only by the procedure (section 3). D-165 and D-167 say "9 of 18 … could be
  killed only by a run in a real window"; the review's table gives 7: four of the nine the procedure
  killed (M11, M12, M16, M17) are also killed by unit tests in the gate.
- What it means (INFERRED): "only the queue moves main, on a green gate" still holds, and main stayed
  green; but for `ui.window` a green gate shows the command line and the shared frame function, not the
  window. A regression of the kinds M05–M08, M13, M14, M18 (panic hook, `catch_unwind`, start paths of
  either form, script input, default root, closing after the last line) would merge green. The window
  procedure (`ac2.sh` in the T-065 test-author log, six cases) exists only as a log entry; nothing reruns
  it at a later task unless a reviewer of a later task chooses to. CI runs `ubuntu-24.04`
  (`.github/workflows/ci.yml:18`) and its state is not visible here.

## 6. The director as a failure source

Every director error or correction I found in the stage (sources: decision rows, task files, records;
"reached" = a builder acted on it before it was corrected; costs from `M` unless said):

| # | Error or correction | Cause | Reached | Caught by | Measured cost |
|---|---|---|---|---|---|
| 1 | T-055 made ready (no spec check, allowed by §5.7) with its Interface and AC-2 in conflict over when a control request's effect is shown (D-149: "origin in the spec") | spec accepted with an error | implementer R-226, test author R-227 | acceptance test at join 1 (and the implementer's QUESTION 1) | one failing join; R-228 + R-229 51,408 (shared with n8 and document sentences) |
| 2 | Join of T-055 at 19:50:00 before the implementer's eight questions were answered (rulings 19:53:00); §2.1 says before the join | step omitted | — | not caught (found here) | inside #1 |
| 3 | n1 (D-148): both roles edit `apps/kiln/Cargo.toml` and `Cargo.lock` | ruling | both builders | tool: queue conflict at rebase (D-153) | with #4: 2 queue failures, 1 extra join, R-235 + R-236 18,525, 25.2 min between queue attempts (21:24:09 → 21:49:19); D-154 says "about 50 minutes" |
| 4 | D-153: linear replay of the branch, which changed every commit id; the queue then refused at `join` (D-154) | operation | reviewer R-235 (confirmation of a branch then abandoned) | tool | inside #3 |
| 5 | n11 (D-149): the implementer's "rectangle" rule accepted and published | ruling | implementer R-228 | reviewer R-230 (defect #1) | share of T-055's post-round-1 119,421 |
| 6 | Spec sentence "every interactive node has a non-empty label" (T-055) | spec accepted with an error | implementer | reviewer R-230 (defect #2, spec gap 10) | share of the same rework |
| 7 | T-055's spec asked for a guard of behaviour main has, pinning exit status 1 that the drafted T-056 changes (golden rule 9) | spec accepted with an error | test author R-227 | reviewer R-230 (spec gap 9) | part of R-232 (w4) |
| 8 | T-056 tiered medium after the split although it claims `ui.dispatch-only`, a high item (R-238, D-156 Q-5) | spec accepted with an error | implementer R-239, test author R-240 (briefed medium) | director at the join (D-158) | the C5 comparison the split was for was lost; led to #9 |
| 9 | D-159: front-matter lines committed on the T-056 branch so that `hygiene` agrees with `tier: high` | operation | — | tool: refusal at `front-matter`, then a conflict at rebase (D-162) | 2 queue failures, 1 extra join, R-245 5,410, ≤ 21.4 min (state `queued` 01:18:29 → attempt 2 01:39:51) |
| 10 | T-056's third join run with main's state at `queued`, no transition written | bookkeeping | — | director (D-162) | — |
| 11 | Q-12 A (D-163): `--script` in the window in real time (the resumed spec check R-246 recommended it and called B unmeasured) | ruling | implementer R-247, test author R-248 | reviewer R-249 (defects #2, #3, #5), reversed by b1 | share of T-065's post-round-1 186,089; 66 min from round-1 end to round-2 end (03:17:45 → 04:23:41), shared |
| 12 | D-163 left closing the window during `--script` unspecified; the toolkit's quit shortcut then closed it (T-065 r1 #1) | ruling (gap left) | implementer | reviewer R-249 | inside #11 (INFERRED link) |
| 13 | Pinned "a desktop window of 1280×800 points" without its condition (T-065, spec gap 9 → b6) | spec accepted with an error | implementer | reviewer R-249 | inside #11 |
| 14 | D-158: "724,931 tokens booked" (the records: 723,931) | bookkeeping | — | director (D-159) | — |
| 15 | D-158: "the two runs already booked carry tier medium" (only R-240 does) | bookkeeping | — | not caught | — |
| 16 | D-162: T-056 "over 7 runs of builders and reviewer, two confirmations" (7 runs in all, one of them the only confirmation, R-245) | bookkeeping | — | not caught | — |
| 17 | D-165, D-167: "9 of 18 mutants … only by a run in a real window" (the review's table: 7, section 5) | bookkeeping | — | not caught | — |
| 18 | D-149: "the first acceptance test in the experiment to fail against an implementation for a behaviour reason" (D-029, T-004, iteration 0) | bookkeeping | — | not caught | — |
| 19 | R-235 booked `completed` without a log entry on main (§12) | bookkeeping | — | not caught | — |
| 20 | Handoff 2026-11-15 05:25: "four of this stage's five queue failures" (the records show four in all: 2 conflicts in `queue.jsonl`, 2 refusals in D-154, D-162) | bookkeeping | — | not caught | — |
| 21 | D-170 sets 4,645,569 against "the planner's estimate for cut 1, about 5.5 M including corrections that were deferred"; the estimate for the work actually done (ADR-0007 rows 5, 6 and evaluation A) is 2,353,295, so spend was **1.63×** it, not under it (section 7) | bookkeeping | — | not caught | — |

Tallies (VERIFIED by counting the table): **21** errors or corrections; **9 reached a builder** (#1, 3, 5,
6, 7, 8, 11, 12, 13). Caught by: reviewer 6, acceptance test 1, tool 3, director 3, nobody 8. By cause:
spec accepted with an error 5, ruling 4, operation or step omitted 3, bookkeeping 9. **All 4 queue
failures and 3 of the 7 behaviour defects** trace to the director's rulings or commits.

Against earlier stages: stage 1 24 (10 reached a builder) over 10 tasks; iteration 1 40 (19) over 12;
this stage 21 (9) over 3 product tasks and one publish. Per product task the rate rose (about 7 against
3.4 and 3.3, INFERRED division). Two caveats: the bookkeeping rows 14–21 come from my recomputing the
rows' figures against the records, which retro-2s1 did for fewer rows; and the UI tasks were the first
of their kind. Measured direct costs: two queue chains (#3–4, #9: 4 failed attempts, 2 extra joins,
23,935 tokens of confirmations, about 47 minutes between attempts) plus shares of three reworks.

**Rules added in the stage: in the methodology or not?** None of them is in `methodology/`, `AGENTS.md`
or `docs/development.md` (no commit there since `40947e8`; `grep -rn -i 'one role\|highest tier among\|
joins and scorecard\|planner that drafted'` finds nothing; VERIFIED `G`). They are in decision rows, in
STATE ("Rules added in this stage") and in task-file comments (`touches: # D-153: every path belongs to
one role`):
- D-153: a `touches` path belongs to one role — decisions, STATE, task front matter comments.
- D-158 / D-162: a task's tier is not below the tier of an item it claims, fixed at specification —
  decisions, STATE. STATE words it as "the highest tier among the items it claims", which would make
  T-065 medium; D-163 has it as the larger of the boundary tier and the items' tiers.
- D-162: the director commits nothing on a task branch but joins and scorecard ties — decisions, STATE,
  handoff 02:00. It contradicts §2.1's text ("committed on the task branch(es)").
- D-163: STATE lists "a spec check is done by resuming the planner that drafted the spec" as a rule; D-163
  itself records only the cost (34,769 against 366,707); the handoff 02:00 says "prefer it".

## 7. Spec checks and the split

| | R-237 (fresh, T-056 before the split) | R-238 (applying the split) | R-246 (drafter resumed, T-065) |
|---|---|---|---|
| Tokens | 366,707 | 31,240 | 34,769 |
| Wall | 1,408 s | 222 s | 303 s |
| Blockers found | 11 (B1–B11) | — | 4 (B12–B15) |
| Questions | 6 (Q-5 to Q-10) | — | 2 (Q-11, Q-12) |
| Misses that reached builders | T-056 tier medium (via R-238); the settle figure "to be measured" on unseeable scripts; z5 (Enter after a lone `\r`) | the medium tier | Q-12's recommendation A (2 behaviour defects); "1280×800" without its condition |

VERIFIED `M speccheck`; blockers from the task files. Per blocker: R-237 33,337; R-246 8,692 (INFERRED
division). The resumed check was ten times cheaper; it found fewer blockers in a smaller spec, and the
one recommendation it defended from its own draft ("A: real time, as specified") was the one the review
reversed. A fresh check is not shown to have caught that (no counterfactual).

**The split (D-156).** Cost: R-238 31,240 plus R-246 34,769 of planning; a second review chain, approval
row and queue run. Bought: the editor merged at 01:49 before the window was dispatched, so the window's
dependency could not hold it back; the window's spec check pinned the shared items (`App`, `draw_frame`,
`apply_style`). Not bought: the reason D-156 gave first, "the bulk of the behaviour gets a medium review
(C5 can be tested)", was lost when T-056 was re-tiered high (D-158). T-056 + T-065 cost 2,077,849
against ADR-0007's single-task estimate of 810,888 for the unsplit T-056 (2.56×); what the unsplit task
would have cost cannot be told.

**ADR-0007's estimates against spend** (VERIFIED `M estimates`): T-055 810,888 → 997,800 (1.23×);
T-056 (with the window) 810,888 → 2,077,849 (2.56×); spec check 326,683 → 432,716 (1.32×); evaluation A
404,836 → 326,121 (0.81×); together **2,353,295 → 3,834,486 (1.63×)**, plus the plan R-225 (351,751),
which the estimate did not contain. ADR-0007's cut-1 figure of 5,490,713 includes 3,137,418 for rows 1–4
(T-051, T-052, T-054, T-043), which D-147 deferred; comparing 4.65 M with 5.5 M (D-170) mixes the two.
The estimates used non-UI medians, as the ADR said they would.

## 8. Cost

- Tokens per task, role and item: Metrics and Further numbers (VERIFIED `M tokens`).
- Overhead booked to `iter-2`: 1,569,920 = 33.8 % (26.5 % without R-223, R-224); stage 1 38.5 %. Spec
  checks fell from 2,286,780 (14 runs, stage 1) to 432,716 (3 runs).
- Budgets: T-055 90.7 %, T-056 108.2 % (cap 91.6 %), T-065 80.7 % of 1,100,000. The UI tasks' median
  (997,800) is above stage 1's high median (810,888) and near the ceiling.
- Against stage 1 and iteration 1 (INFERRED from the rows above and retro-2s1): per new verified item
  1.03 M of task tokens against 0.23 M, and 1.55 M against 0.71 M with all runs; post-round-1 rework
  14.9 % against 8.5 % (iteration 1 24.6 %); round-1 review tokens per task about equal (268,991–314,479
  against 229,836–326,832). The UI items are dearer per item; T-055 carries the observation interface
  and claims no item, and the toolkit is new, so this is not a cost of the order alone (ADR-0007's
  confound).

## 9. Throughput and environment

| Number | Value | Command |
|---|---|---|
| D-147 → T-067 merged | **10.71 h** (19:06:27Z → 05:49:01Z); from R-225's start 11.21 h | VERIFIED `M time` |
| Time with no run active | **4.37 h of 10.71 h (41 %)** in 22 gaps; stage 1 22 % net of stops. Twelve gaps ≥ 10 min, nine of them 17–22 min; most follow the end of a builder pair or a review (join with its gate, rulings, dispatch); 21:27–21:48 and 01:18–01:39 are the two branch rebuilds; 05:07–05:49 the publish | VERIFIED `M time` |
| Usage-limit stops and restarts in the stage | **none** between D-147 and D-170 (R-253: boot 18:15:21, no restart). Before the stage, D-143: stop about 12:05Z, restart 18:15:21Z, about 6.17 h lost, R-223 resumed (retro-2s1). After cut 1: the director stopped at 05:54Z to report (D-170); restart 16:09:46Z with nothing in flight; the owner answered about 16:10Z (D-172) | D-143, D-172; `evaluations/iter-2-cut-1.md` "Setup" |
| Builder concurrency | max 2 of 3; the stage was one serial chain (T-055 → T-056 → T-065) | VERIFIED `M time` |
| Disk | free at each handoff addendum: 21, 18, 28, 28, 17, 28, 16, 27 GB; a T-055 worktree's `target/` 9.9 GB after the gate; a T-065 worktree 12 GB ("a worktree with the windowing crate built is 12 GB", handoff 02:45); the evaluator's clone 9.0 GB; now 23 GB available | `grep -on '[0-9.]\+ GB free' handoffs/2026-11-1[45].md`; task logs; `df -h /home/user` |
| Gate time per run | queue 1,074.9 / 548.3 / 560.7 / 720.4 s (T-055 ran the gate twice, a gate path); implementer gates 515–759 s; stage 1 queue 241 → 574 s | VERIFIED `M queue`; task logs |
| The windowing dependency | `Cargo.lock` 122 → 156 (T-055, egui) → 326 packages (T-065, eframe; `G`); clean debug build of `-p kiln` 56.42 s / 55.47 s with `target/` 1.8 GB and a 304–306 MB `kiln` binary (R-237 E-a, R-246 E-g); release build of T-065's head 2 m 41 s, 643 MB, against the base's 1 m 46 s, 432 MB (T-065 r1). Queue gate before and after eframe: 548.3 s (T-056) and 560.7 s (T-065): no visible change in gate time; worktree size 9.9 → 12 GB | `G`; task files; reviews |

INFERRED: the binding limits on throughput were the serial chain of the plan (one task at a time, ADR-0007
as amended) and the director's work between runs (41 % of the window with no agent running), not the
builder cap and not disk. With three UI worktrees at about 12 GB each, cut 2's three tasks side by side
would need about 36 GB against 16–28 GB seen free: the trigger D-147 set for T-054 ("when disk limits
the number of worktrees with the UI toolkit built") would be met by cut 2's plan.

## 10. Claims

| Claim | Verdict | Numbers that decide it | What would change the verdict |
|---|---|---|---|
| C1 contracts and checks first | **insufficient** | 3 spec-check runs, 432,716 tokens, 15 blockers, 0 blocked builder runs; T-055 (no check) reached its first join with a spec contradiction; the resumed check endorsed the ruling (Q-12 A) that caused 2 defects; no unchecked arm to compare | A comparison of checked and unchecked (or fresh and resumed) specs on blocked runs and defects |
| C2 serial queue keeps main green | **supported** for main | 4 merges, 0 red-main minutes; 4 failed queue attempts, all from the director's rulings or commits, each refused or stopped before main moved. Caveat: for `ui.window` a green gate shows no window (section 5) | A red main; or a window regression merged green and found later |
| C3 another agent's tests catch what the implementer's miss | **insufficient** | 10 joins, 1 implementation deviation caught at a join, which the implementer had disclosed and its own test asserted; 0 of 7 behaviour defects found by reviews were caught by acceptance tests (first joins of T-056 and T-065 green 22/22, 6/6); round-1 kill rate by acceptance 61 % (45 % with gate tests only). All three tasks had independent test authors, so no arm comparison. The earlier event D-029 (iteration 0) stands | An acceptance-caught defect that neither the implementer nor a review saw; or a seeded trial of both arms |
| C4 status from behaviour tests plus evaluator | **supported** | 3 implemented → 3 confirmed, 8 regression items confirmed again, 0 rejected; the evaluator found NEW-1 and three inert request kinds that no review found. Caveats: `ui.editor-view` was confirmed on pixels outside ADR-0008's interface while the snapshot failed (O3) on 528 chars; `ui.window`'s gate tests show no window | A confirmed item later found wrong in the behaviour its title names |
| C5 risk tiers place review effort | **insufficient** | all three tasks high (T-056 re-tiered); round-1 review 268,991–314,479 tokens; 10 round-1 defects in high tasks; no medium or low reviewed task | Reviewed tasks of several tiers in one stage |
| C6 budgets that gate | **refuted** as a gate | latency: `xtask/budgets.toml` and `fuzz.toml` have no entry at `eefd9d0` (the steps are EMPTY); tokens: one overrun with a row before the rework and a cap that held | An enforced latency budget (T-028) |
| C7 repository is the memory | **supported** | 31 runs, 30 with a log entry; the post-cut-1 restart found nothing in flight and the work was resumed from the repository; every figure of D-170 agrees with the records. Defects of the record: R-235, rows 14–21 of section 6, rules kept outside the methodology | A loss of state, or a resume that needed what only a session held |
| C8 constructed oracles define correct | **supported** for UI behaviour | `kiln-cli control` as oracle (3,934 lines, 0 differences; 460 sessions, 0 command mismatches), `syntax.highlight` for colours, a naive model of the Input table, `kiln script` as oracle for the window (2,750 and 46,360 lines). They found the row-text defect, M17, T-065 #1–#3 and NEW-1. But the observation layer itself was wrong three times (row text, right-to-left rows, NEW-1), and the evaluator needed pixels to judge it | A UI item whose "correct" no oracle states; or a defect of the observation that no oracle can see |

## 11. Surface first (D-140, D-145) and cut 2

What cut 1 shows (INFERRED from the sections above):
- A thin surface over verified commands can be kept honest: every UI change of state is a logged
  registry command (460 evaluator sessions, logs replayed to the same files), and no defect the UI work
  found lies in L0–L4; all 10 round-1 defects were in `kiln-ui`, `apps/kiln` or documents.
- Building the surface first meant building the measuring instrument first: T-055's observation
  (`observe.rs`) carried two defects into later tasks (row text, NEW-1) and a stated limit (right-to-left
  rows, T-066). Every later UI item is judged through it.
- Wired but inert turned up three times, all in request parameters and labels the shell accepts without
  effect (`scroll` left/right, secondary click, clicks on `Documents` entries), filed as T-068 and noted in
  T-058; none in what the items' titles claim.

What it does not show: that surface first is better or worse than depth first. The order, the first L5
work, a new toolkit and a new observation interface came together (ADR-0007 "Limit"); cost per UI item
(about 1.03 M) cannot be put down to the order.

What the record says about cut 2 (T-057, T-058, T-059):
- Each needs a spec check (D-147); the drafts were written by R-225 before T-055 existed.
- T-057 and T-059 are odd: the first implementer-first UI tasks, so v1.3 change 7 will be tried there.
- All three read snapshots: T-066 (what a snapshot reports for shaped text, NEW-1 included) changes the
  instrument they are measured with, and T-068 the requests they accept; T-058 changes the
  current-document rule (D-169 noted the `Documents` click there).
- Disk: three worktrees with the toolkit built need about 36 GB (section 9).
- D-171 (platforms) touches the same window code (`eframe` features `x11` only) and the confinement code
  under every item (`crates/kiln-workspace/src/disk.rs`), so its order relative to cut 2 matters for
  rework (INFERRED; not designed here).

## 12. Linux, macOS and Windows (D-171): what the methodology assumes

From the record only (`G`, last sections; task files; reviews):
- **`no-skip` against `cfg`.** `xtask/src/noskip.rs:9-10` fails any test behind a `cfg` other than
  `cfg(test)`. D-108 (stage 1) refused a `#[cfg(unix)]` acceptance test on that rule; ruling b8 (D-166)
  accepted a test file "that compiles only on Unix … without a `cfg`, since the gate and CI run on
  Linux". At `eefd9d0`, **31 test files** use `std::os::unix` or `cfg(unix)` (`G`), and production code in
  `crates/kiln-workspace/src/disk.rs:86` imports `std::os::unix::fs::{MetadataExt, OpenOptionsExt}`
  without a `cfg`. On a non-Unix target these do not compile (INFERRED from the imports; nothing was
  built), and the methodology has no rule that lets a test be platform-specific.
- **Tests that need a Unix identity or file system.** 16 files set permission modes or use
  `PermissionsExt` (`G`); reviewers and the evaluator probe permissions as uid 0 with capabilities
  dropped (`setpriv`, `capsh`), symlinks, FIFOs, `/dev/full`, `/proc`, `strace` (T-055 r1, T-056 r1,
  T-065 r1, R-253).
- **The Xvfb procedure.** ADR-0008 Q-2 A and the window procedure: `apt-get download`, `dpkg-deb -x`,
  `xvfb-run`, `LD_LIBRARY_PATH`, XTest through `ctypes` on libX11, framebuffers from `Xvfb -fbdir`;
  eframe built with `x11` only (D-167). Every observation of `ui.window` is of this kind.
- **The gate on one machine.** CHARTER §4 says the gate is "identical locally and in GitHub Actions";
  the queue runs it locally and moves main on that result (§8); CI runs one job on `ubuntu-24.04`
  (`.github/workflows/ci.yml:18`), after the push, and its results are not visible to the agents here
  (retro-2s1; D-171: "no agent in this environment can run macOS or Windows").
- **Evidence only Linux gives.** The reviewer's adversarial probing (§4 high tier), the differential runs
  of change 9, the evaluator's "public interfaces" (§9) and the scorecard's `verified` are all produced on
  this one Linux machine.

What the methodology would have to say, and does not, for evidence that can come only from CI runs on
machines no agent can reach (findings, not a design):
1. **What a platform is in the scorecard**: whether an item is `verified` per platform or once, and what
   the evaluator confirms for a platform it cannot run (§9 knows only "through a public interface").
2. **Who reads a CI result and where it is recorded**: no metric, record or role covers a CI job's
   result; §13 has no field for it; `queue.jsonl` records only the local gate.
3. **Whether the queue waits for it**: §8 moves main on the local gate; a leg that fails only on another
   platform would land on main green by the local rule (PhotoCraft's three Windows-only failures reached
   main, D-171).
4. **What "red main" means** when main is green here and red on one CI leg, and who opens the incident.
5. **How `no-skip` treats platform `cfg`**: today a platform-only test is either a gate failure or, by
   b8, a file that does not compile elsewhere.
6. **What a reviewer or evaluator may claim** about behaviour on a platform it cannot run, and how a
   claim resting on a CI log differs from one observed through a public interface (the "observed" rule of
   ADR-0008 has no remote form).
7. **Tiering**: the CI workflow is a gate path (high, §4); platform legs would make every task that
   touches platform code depend on evidence the builder cannot produce in its own loop (§10's gate
   attempt is local).

## 13. Where the methodology was wrong or silent, and proposed changes for v1.4

Rules not followed: §2.1 "questions answered before the join" (T-055, #2); §2.1 "rulings committed on
the task branch(es)" (practice moved to main from T-055 on, reviewers read main's copy; D-162 now forbids
the branch); §12 "no entry ⇒ run marked incomplete" (R-235); D-147's own trigger for T-043 (two refusals,
no dispatch); §13 `found_by` values (T-066 `implementer`).

Rules that cost and changed little: confirmation records filed as "round 3" (23,935 tokens for three runs
whose content is a `git diff --stat` that a tool could print).

Gaps that let something through: no check that a task's tier is at least its items' tiers (D-158); no
check that a path is edited by one role (D-153); the queue cannot take a branch with resolved merges
(D-153, D-154); observation code reviewed without the input classes that break it (section 3); a figure
"to be measured by the implementer" on tests it cannot see (T-056); no gate evidence for the window
(section 5); `killed_by_acceptance` mixes gate tests and a manual procedure (T-065); an evaluator clone
that contains the records it may not read (R-253); no platform notion (section 12).

| # | Change (file, section) | Evidence | Expected effect and cost | What would show it was a mistake | Removes |
|---|---|---|---|---|---|
| 1 | **Tier floor from the items** (§4, task template, planner brief): a task's tier is the higher of its boundary tier and the highest tier of the items it claims, set at `specified`; `scorecard --check` (or the queue) refuses a task whose tier is below an item's | D-158 (re-tier at the join), D-162 (2 queue failures that followed) | no re-tier in flight; a small tool change (T-033's scope) | a task whose real risk is lower than an item's tier, over-reviewed at cost | the director's reading of item tiers |
| 2 | **Rulings live on main; the director commits only joins and scorecard ties on a task branch** (§2.1, replacing "committed on the task branch(es)"); builders and reviewers read rulings from main's task file | D-159 → D-162; T-055 r1, T-056 r1 read main's copy; handoff 02:00 | rule matches practice; refusals of this kind go to 0 | a builder acting on a stale branch copy of a ruling | the branch-ruling sentence |
| 3 | **One role per `touches` path** (§5, task template), checked at the join (`git diff --name-only` of both branches must not share a path) | D-153, D-154: 2 queue failures, 18,525 tokens, 25 min | the conflict class disappears; one check at the join | a task where a shared file cannot be avoided (then the owner adds the line on a ruling, as D-153 says) | — |
| 4 | **Confirmation of a rebuilt branch is a tool check, not a review round** (§7): the queue or the join accepts a head whose tree equals the approved head merged with main (`git diff --stat` empty) and records it; no reviewer run, no `rN` file (with T-033's branch-shape fix) | R-235, R-236, R-245 23,935 tokens; R-235 without a log entry; D-153 "a use of round 3 the methodology does not provide for" | three runs and two records fewer per rebuild | a rebuilt tree that differs in a way `diff --stat` cannot show | round 3 used as confirmation |
| 5 | **Generated inputs for observation and text code** (§7 round 1, reviewer brief): where an item reports text or colours, the round-1 differential or model run draws its text from a stated alphabet that includes ligature pairs, combining marks (also at a row's start), joiners and right-to-left digits, and the review records the alphabet | row-text defect passed T-055's two rounds with no non-ASCII input (section 3); found by T-056's 5,833 misaligned steps; NEW-1 found only by the evaluator's combining-mark seed | defects of the observation found in the task that writes it; cost one generator per item, reusable | such runs that find nothing in two stages | — |
| 6 | **A figure "to be measured by the implementer" must be measurable by it** (§5.7, planner brief): if it depends on the acceptance tests, the reviewer measures it in round 1 and the sentence is written then | T-056 QUESTION (`:858`), y2, round-1 defect #3 | one class of document defect less | — | — |
| 7 | **An unmeasured option is probed before it is ruled** (§2.1): when a spec check offers options and calls the stronger one "unmeasured", the director orders the probe (or rules the stronger option provisionally) before dispatch; a resumed drafter may do the check, but not alone decide between its own draft and an alternative | Q-12 A (R-246 resumed drafter: "B would need its own probe"); 2 behaviour defects, reversed by b1 | fewer rulings reversed by review; cost a probe (T-065's b1 fix was 82,028 implementer tokens) | probes that never change a ruling | — |
| 8 | **Window evidence in the gate or recorded as outside it** (§6, §9, scorecard): either a gate step that runs the window procedure under Xvfb (needs the owner to add the two libraries to the environment) or an item field saying which of its criteria only a manual procedure shows, so that the procedure is rerun at every task touching `window.rs` and at each evaluation | `ui.window`'s 7 gate tests open no window; 7 of 17 T-065 r1 mutants only by a real window | the window stops being guarded by memory; cost a few minutes of gate per run (a 12,000-frame window script took 131 s, R-253) | flaky window runs failing the gate | — |
| 9 | **Split `killed_by_acceptance`** (review template, §13): `killed_by_gate_acceptance` and `killed_by_procedure` | T-065 r1 13 = 4 + 9; D-165/D-167 misread it as "9 only by a window" | the kill rate means what the gate holds | — | — |
| 10 | **Numbers in decision rows name their source** (§2.1): a count or cost in a row is copied from a command's output named in the row (as every agent must); the analyst's script recounts them | rows 14–21 of section 6 (eight figures or claims in rows, a handoff and the run record that the records contradict) | record errors become visible at writing time; cost a command per row | — | — |
| 11 | **Write the stage's interim rules into the methodology** (§2.1, §4, §5): D-153, D-158/D-162, D-162's branch rule, and D-163's "resume the drafter for its spec check" with change 7's limit | none of the four is in `methodology/` (VERIFIED `G`); STATE states D-158's rule in a form that contradicts D-163 | agents read one text; STATE stops being a rule source | — | rules held in STATE |
| 12 | **Evaluator clone without ledger records** (§9, evaluator brief): the clone is made without `docs/decisions.md`, `tasks/`, `reviews/`, `handoffs/`, `STATE.md` (sparse checkout), which the evaluator may not read | R-253's disclosure (300 chars of D-165 printed) | the evaluator cannot read what it must not; negligible cost | a check the evaluator needs that reads one of them | — |
| 13 | **Act on T-043's trigger**: queue refusals are written to `queue.jsonl` before the next queue run (D-147's own rule) | 2 refusals in rows only (D-154, D-162); first-pass success cannot be computed from `queue.jsonl` | the queue metric becomes complete; one high task (T-043) | — | the decision-row interim for refusals |
| 14 | **Platform evidence** (§4, §6, §8, §9, §13, scorecard; with ADR-0009): the methodology states (a) the platform set an item is verified for, (b) a CI leg's result as a record (`ci.jsonl` or a queue field) and who reads it, (c) whether the queue waits for the legs, (d) red main per platform, (e) how `no-skip` treats a platform `cfg`, (f) what a reviewer or evaluator may claim from a CI log | section 12; D-166 b8; D-171; 31 Unix-only test files | the port has rules before its first task; cost one methodology section | — | b8's Linux-only acceptance |
| 15 | **Answer questions before the join, or drop the words** (§2.1): either the join refuses while the implementer's last entry has an unruled `QUESTION:` touching an AC, or the rule says "before rework is dispatched" | T-055 joined 3 min before its rulings; the join's failure was useful evidence | rule and practice agree; small | — | one of the two wordings |

Not proposed: changing the builder cap (max 2 used); restoring the 75 % row (its absence cost nothing);
loosening round-1 request-changes (all three rounds 1 had behaviour defects).

## Budget recalibration

Rule as before: p75 of the tier's builder and review tokens per merged task, rounded up to 50k. Stage 2
alone (high, n=3): 997,800, 1,190,305, 887,544 → p75 1,094,053 (INFERRED from `M tokens`; linear p75 of
three values) → 1,100,000, the present ceiling. I propose no change: n=3, all UI tasks of one kind, and one
of the three needed the cap of D-159. Medium and low have no data in this stage.

## Where the director's record disagrees with the data

- **D-149** calls T-055's failing join "the first acceptance test in the experiment to fail against an
  implementation for a behaviour reason"; D-029 and the T-004 join record (2026-11-13T01:57:08Z,
  `acc_defects: 1`) record one in iteration 0. The test also caught a deviation the implementer had
  asked about in its log (section 2).
- **D-165 and D-167**: "9 of 18 mutants … only by a run in a real window"; the review's table gives 7 of
  17 non-equivalent (section 5).
- **D-170**: spend 4,645,569 set against "about 5.5 M" for cut 1; the estimate for the work done is
  2,353,295 and spend was 1.63× it (section 7).
- **D-158**: "724,931" (723,931; D-159 corrects it) and "the two runs already booked carry tier medium"
  (only R-240; R-239 is `high`).
- **D-162**: "over 7 runs of builders and reviewer, two confirmations" (7 runs, one confirmation, R-245).
- **D-154**: "about 50 minutes" for ruling n1; the records show 25.2 min between the two queue attempts
  (the director's own time is not recorded, so the row may count more).
- **Handoff 2026-11-15 05:25**: "four of this stage's five queue failures"; the records show four failures
  in all (2 conflicts recorded, 2 refusals in rows), all four the director's.
- **D-172**: "about eleven hours after the stop at cut 1"; from the D-170 commit (05:54:40Z) to the
  restart (16:09:46Z) is 10.25 h, from the handoff's 05:25Z 10.75 h (minor).
- **STATE** lists D-163 as a rule ("a spec check is done by resuming the planner that drafted the
  spec"); D-163 states only the cost. STATE's wording of D-158 ("the highest tier among the items it
  claims") would make T-065 medium; D-163 says high is the larger.
- **R-235** is booked `completed` with no log entry on main.
- Figures that agree with the records to the token: D-149 557,980; D-150 878,379; D-151 931,025; D-154
  18,525; D-155 997,800 over 11 runs; D-159 1,038,410; D-160 1,149,136; D-161 1,184,895; D-162
  1,190,305; D-165 701,455; D-167/D-168 887,544; D-168 3,075,649; D-170 4,645,569, 1,569,920 and
  30,203,036 (`M tokens`, `M budgets`; INFERRED partial sums of `M tokens` rows for the intermediate ones).

## 14. Uncertainty

- **Samples**: three product tasks, all high, all with independent test authors and all UI; no arm or
  tier comparison is possible. C3 and C5 have no comparable events.
- **Classification**: behaviour versus document defects, the origins of defects, the director-error table
  (its causes, "reached", and the link of #12 to the ctrl+Q defect), and the evaluator-versus-review
  attribution are my readings.
- **Counting differences**: the bookkeeping rows of section 6 come from recomputing figures that
  retro-2s1 checked less systematically; the rise in errors per task may partly be my method.
- **Director time**: 41 % of the window had no agent running; how much of it was director work (joins,
  queue, rulings) and how much waiting is not recorded. The cost in wall time of #3–4 and #9 is the gap
  between queue attempts, not the director's effort.
- **Would a review have caught D-149's deviation**: inferred from the question in the log and the
  reviewer's later check; not observed.
- **Mutant equivalence**: T-056 M8 and T-065 M15 are argued equivalent by the reviewers, not proven.
- **Platforms**: that the Unix-only files do not compile elsewhere is read from their imports; nothing was
  built (the brief forbids cargo here). CI results on `ubuntu-24.04` are not visible.
- **The window's evidence**: whether the window procedure would pass at today's head has not been rerun
  since R-253.
- **The usage window**: no stop is recorded in the stage; the handoff's "usage window that began about
  18:15Z ends about 23:15Z" is a statement, not a measurement, and no stop followed it.
- **Model per run** (O-11): not visible; not used.
