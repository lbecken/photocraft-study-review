# Director handoff — 2026-11-13 ~06:45 UTC

Written because the directing session's context is nearly full. Everything needed to continue is in the
repository; this note says where to look and what is in flight.

## Where the experiment stands
- **Iteration 0 is complete**: T-001…T-005 and T-012 merged; evaluation `evaluations/iter-0.md` (22 of 25
  items confirmed from outside); retrospective `retrospectives/iter-0.md`; decisions D-001…D-042 in
  `docs/decisions.md`.
- **Methodology v1.1 is in force** (`methodology/`, `docs/adr/0002-tier-by-boundary.md`, ADR-0003 accepted).
  Claims after iteration 0: C2, C4, C7 supported; C6 refuted; C1, C3, C5, C8 insufficient.
- **Iteration 1 has started.** Merged: T-006 (low), T-014 (publish of v1.1 entry docs). `main` is green
  (gate: 10 steps, 376 tests at the last check); scorecard 22 verified / 3 implemented / 38 planned.

## In flight (builder agents on the owner-specified model; cap: three compiling at once)
| Task | Role / run | State of the work |
|---|---|---|
| Merged in iteration 1 so far | T-006, T-012, T-014, T-013, T-015, T-016, T-032, T-010, T-011, T-017, T-008 | Eleven tasks. T-008 merged at `d4915606f9a789b42b99a1eea053554f1f9dd8a7` after three review rounds, with one accepted, documented gap (D-081, D-083); its worktrees are removed. GitHub CI: runs 174 to 179 all concluded with success (the T-017 merge push is run 175); the T-008 merge push (`a195dfa`) is run 185. Every run from 174 to 185 concluded with success when checked. |
| T-008 path confinement (high) | MERGED (see above) | Nothing further on this task. |
| Follow-up to T-008 (new task, high tier, `origin_task: T-008`) | a fresh planner, R-145, is drafting it in the primary checkout (one new file under `tasks/`) | When it reports: read the draft, rule on its options for the unreadable-root case, commit the file by path (before any queue run), book R-145 to `iter-1`. Then it needs a pre-dispatch spec check. |
| T-018 selection and anchors (medium, even) | in_progress: review r1 (R-144) request-changes (two documentation defects, one test gap, three spec gaps); rework by both builders, resumed: implementer R-146, test author R-147 | Rulings x1–x6 (D-084) under `## Questions and rulings`: `null` refused for optional parameters; the caret wart inside a batch kept and documented; a redo test; the `kiln-text` acceptance file renamed to `acceptance_t018_text.rs` so the join counts it. Branch head before the rework: `7c1526ef023da003b64d6ac9be329a81134bc7af` (includes the director's scorecard commit). Next: both report → verify commits → book → `cargo xtask join T-018` (expect 12 acceptance tests counted now; if the gate stops at `scorecard`, regenerate the page on the branch) → transitions → round 2 (delta, final; resume the R-144 reviewer) → a row beginning `Extend T-018 to <N>` (682,685 tokens booked before the rework, ceiling 500,000); medium tier needs no approval row → queue. |
| Specs T-021, T-022 (draft) | planner R-127 done and booked; rulings D-071 in both task files | Bounded search (`Regex::for_each_match`, exact `total`, pattern cap 1,000 chars, preview lists at most `limit`). Both need a pre-dispatch spec check after T-017 merges; the check measures search time at the cap and both halves of the 512 MiB memory tests. |
| Ready or waiting | T-020 (after T-008: both add a reason row), T-030 (not with T-008), T-035 (low), T-007, T-019, T-027 (high, spec checks) | |
| Tooling specs | T-009, T-033, T-034 (high, gate paths, not in flight together, need spec checks); T-028 depends on T-013 only | Lower priority than the product track. |

## Plan for the rest of iteration 1 (all specs are in `tasks/`, state `draft` unless noted)
- Tooling track: T-013 → T-009 (queue follow-ups, gate paths) → T-028 (fuzz and budgets runners) → T-029 (command fuzz target).
- Product track: T-015 → T-032 → then T-010 and T-008 (not together: both edit `reason.rs`) → T-007, T-011,
  T-030, T-018, T-020, T-019, T-027 → T-021, T-024, T-025, T-023 → T-022, T-026 → T-031.
- Pairs that share a hand-written file and must not be in flight together: T-007/T-010, T-007/T-011,
  T-007/T-024, T-011/T-024, T-007/T-027, T-008/T-010, T-008/T-024, T-008/T-030, T-010/T-027, T-024/T-030.
- Medium-tier experiment arm (D-035): even task number → independent test author; odd → the implementer
  writes the acceptance tests on `task/T-NNN-tests` first. High tier always has an independent test author.
- Every high-tier spec gets a spec check before dispatch (D-041); reports go under `tasks/spec-checks/`.
- After the features: evaluator for iteration 1 (clean clone, public interfaces only), retrospective 1
  (score C1–C8; compare the two arms), then iteration 2 (UI: the `ui.*` items).

## Held items
- Scorecard item file for T-008: `<scratchpad>/held/ws.path-confinement.toml` (may be lost with the scratch
  directory; T-008's test author creates it anyway, per its spec).
- Overhead runs not yet booked (to be booked to `iter-1` once T-013 lands): planner R-056/R-060/R-066
  (519,319 tokens), spec checks R-067 and R-069 (running).

## Operating rules for the director (each learned from an incident; see D-019…D-041)
- Run `cargo xtask …` from the main checkout; never export `CARGO_TARGET_DIR`. Agents build in their own
  worktree with cargo's default target directory.
- Timestamps from `date -u`. `set -e`, one dependent step per line, no heredoc inside an `&&` chain.
- Commit ledger files by explicit path. The queue refuses while any evidence file (`tasks/`, `metrics/`,
  `docs/decisions.md`, …) has uncommitted changes — planners writing drafts in the main checkout block it.
- Only ledger paths may be committed to `main` directly (`xtask/ledger.toml`): not `AGENTS.md`, not
  `docs/CHARTER.md`, not `reviews/`, not `docs/scorecard.md`. Anything else, and any ledger input that
  changes generated output, goes through the queue as a task (after T-013: `cargo xtask publish`).
- `main`'s history is append-only (D-040).
- A ruling that changes behaviour — including an answer to a builder's question — is written into the task
  spec and checked for consistency before builders get it; where the spec pins a line exactly, the pinned
  line wins over a general answer.
- Answer both builders' questions together before the join.
- Write and verify the approval row before queuing; gate-path tasks need `gate paths` in it; dependency
  version changes need `dependency versions`.
- Review: round 1 full, round 2 final unless the delta has a defect, no round 4.
- Every dispatch says: commit early and often; no `git stash`; clean up scratch builds.
- Agents interrupted by a container restart are relaunched fresh; agents stopped by a usage limit are resumed.
- Keep ≥ 15 GB free where possible; remove worktrees after merge and stale scratch directories.

## Run IDs
Used so far: R-001 … R-073.

## Addendum (~06:55 UTC)
- T-032 has had its spec-check edits applied (still `draft`); ADR-0004 still `proposed` — accept it, then make T-032 ready once T-015 merges. After ADR-0004 is accepted, publish the CHARTER §3 sentence as a low-tier task.
- Two spec edits still owed before their tasks are dispatched: T-008 line ~122 must drop its edit of a table cell in `docs/control-protocol.md` (it conflicts with T-032's rule that later tasks do not edit table cells — spec check T-032 finding 4); T-027 line ~72 still says "T-015's `VfsError` mapping" (it is T-032's now).
- T-015's AC-1 was corrected after dispatch (D-043); both builders were notified.
- ~07:00 UTC: T-016 and T-017 accepted (D-044) and `ready`. Still owed before dispatch of T-022: its line ~65 uses `replacement_for`, which now returns `Result`.
- ~07:05 UTC: disk fell to 2 GB free (each worktree's build directory is 3–6 GB). Policy from now on: after a join, delete the tests worktree's `target/`; keep at most four worktrees with build directories; `rm -rf target/debug/incremental` on main when needed. The upstream PhotoCraft clone in the old scratch directory was deleted (the study is complete; re-clone if needed).
- T-016 test author dispatched (R-080); its implementer follows when a compiling slot frees. T-017 is `ready`, not yet dispatched.
- 07:09 UTC: container restarted (D-045); T-013 reviewer, T-015 reviewer and T-016 test author relaunched fresh (R-081, R-082, R-083).

## Immediate next actions (as of 07:15 UTC; the directing session's context is ~88% full)
Three fresh agents are running: T-013 reviewer (R-081), T-015 reviewer (R-082), T-016 test author (R-083).
1. **When the T-013 review reports**: if approve → record reviewer metrics; write and verify the approval row in
   `docs/decisions.md` beginning `Approve T-013` and containing `gate paths`; set state `queued`; commit the
   ledger by path; `cargo xtask queue T-013`; mark merged, push, `cargo xtask worktree rm T-013` (and `--tests
   --force`). If request-changes → fresh implementer and/or test author for the listed defects only; round 2 is
   final (METHODOLOGY §7). After it merges: the tooling stamps times itself, `worktree new` requires `ready`,
   the queue enforces budgets (`Extend T-NNN to <N>` rows), `cargo xtask publish` exists, overhead runs are
   booked to `iter-1` (planner R-056/R-060/R-066: 519,319 tokens; spec checks R-062 286,583 — already booked
   to T-015 —, R-067 294,896, R-069 59,214; planner edits R-074 86,957, R-077/R-079 203,222).
2. **When the T-015 review reports**: same sequence (tier high, no gate paths; approval row `Approve T-015`).
   T-015's recorded tokens include the first spec check (287k) and may exceed 850k once the queue enforces
   budgets — add an explicit `Extend T-015 to <N>` row if so. After it merges: accept ADR-0004, make T-032
   `ready`, dispatch its implementer and test author; publish the CHARTER §3 sentence as a low-tier task.
3. **When the T-016 test author reports**: dispatch the T-016 implementer (worktree `.worktrees/T-016` exists),
   then join, review, approve (tier high: adds a dependency; no `dependency versions` wording needed), queue.
4. **T-017** is `ready`: dispatch implementer and test author when compiling slots and disk allow.
5. Then T-009 (after T-013), and the rest of the plan above. One spec edit still owed: T-022 line ~65
   (`replacement_for` now returns `Result`).
Disk: check `df -h /home/user` before every dispatch; delete a tests worktree's `target/` after its join.

## Addendum 2026-11-13 12:30Z (director)
- Rules added since the handoff was written: approval rows carry one decision and begin `Approve T-NNN` (D-058; check with grep before queuing); briefs give full 40-hex commit ids (D-050); a guard that only proves the current task left something alone is not a permanent test and tests do not pin message texts or whole documents that planned tasks change (D-043, D-052); pre-dispatch spec checks are booked to their task as reviewer runs; `iter-1` takes only director, planner, evaluator and retrospective runs (D-055).
- Still unbooked by hand: spec checks R-067 (T-016 and T-017, 294,896 tokens) and R-069 (T-032, 59,214 tokens) — count them in the iteration-1 evaluation.
- Owed: publish the ADR-0004 sentence in `docs/CHARTER.md` §3 as a low-tier task (not a ledger path); T-022 spec line about `replacement_for` returning `Result`.
- Run ids used: up to R-109. Helper for booking runs from agent transcripts: session scratchpad `book.sh` (not in the repository; the command is `cargo xtask metrics add …`).
- Third container restart at 11:11Z, after a usage-limit stop at about 08:13Z; agents were relaunched fresh. The usage window that began 11:10Z ends about 16:10Z.
- 13:55Z: run ids used up to R-127. Book a run with `cargo xtask metrics add` BEFORE writing the task's `merged` transition: the tool refuses a record whose end time is earlier than the task's latest record (R-123 was refused this way and a commit was made that named it without containing it; corrected in the next commit). Check the row count after every `metrics add`; do not pipe its output through `tail` or `cut` under `set -e`. Before citing a spec section in a message, grep that the text is in the file (D-068, D-069).
- 14:18Z: run ids used up to R-131. A background join's or queue's notification shows the exit code of the output filter, not of xtask: read `"result"` and the `xtask exit=` line before any transition (T-008's first join was `gate_failed` under an exit code 0). When a tests branch adds or changes a scorecard item, the join fails at `scorecard` until the director regenerates `docs/scorecard.md` on the task branch (T-011, T-008). Pushes have returned 503 several times today: retry with backoff and compare `git ls-remote origin refs/heads/main` with the local head. A built-in safety check blocks `rm -rf` on a variable path inside `sh -c`; use literal paths. A scratch directory `ulimit-probe-1358` (about 400 kB) was left in the session scratchpad. The usage window that began 11:10Z ends about 16:10Z.
- 16:16Z: usage limit reached at about 14:20Z, reset 16:10Z, container restarted 16:11:36Z (fourth restart; D-073). Run ids used up to R-134. After a restart: check every worktree with `git status` BEFORE touching it and never revert uncommitted work unseen — two agents had unfinished, uncommitted work and one was about to apply a mutation. Relaunch fresh agents and tell each exactly what it inherits. The new usage window began about 16:10Z.
- 16:16Z: a scheduled check-in is set for 2026-11-13T21:20:00Z (trigger `trig_01HKWRJfgWKPGDVxLf9DJxPL`, "Resume Kiln experiment after the 21:10 usage window"): it wakes this session to relaunch agents if a usage limit or restart stopped them. Replace it with a later one if the window changes; delete it when the experiment is finished. Between 14:20Z and 16:10Z no check-in existed and work resumed only because the owner wrote.
- 16:22Z: run ids used up to R-135. Three agents compiling (cap): R-132, R-134, R-135. Disk was 9.6 GB at 16:19Z: delete a tests worktree's `target/` (literal path) as soon as its join is recorded.
- 16:32Z: run ids used up to R-137; decisions up to D-077. Twelve director rulings or instructions have needed correcting so far (D-077 has the count); the recurring causes are a mechanism pinned where only behaviour should be, and a bound stated without a measurement. Scratch directories left in the session scratchpad: `ulimit-probe-1358`, `inode-probe-1630`.
- 16:42Z (time from the clock; the line was first stamped 16:44Z by hand): run ids used up to R-139; R-140 and R-141 are reserved as above. Decisions up to D-078. The R-134 reviewer's final harness figure is 134,645 tokens and 33 tool calls; the booked record says 133,953 and 32 (booked from an interim notification) — add a correction note to the next decision row. `cargo xtask metrics report` exits 1 while any task is over its ceiling without an extension; T-012 and T-014 merged before the budget rule and are permanently so, so that exit code is not a usable check (tooling follow-up).
- 16:54Z (time from the clock): run ids used up to R-142; R-143 is reserved for T-008 round 3. Decisions up to D-081. T-021 and T-022 can get their pre-dispatch spec check now that T-017 is merged (conditions in D-071 and D-077); T-021 also depends on T-018.
- 17:05Z (time from the clock): run ids used up to R-144; R-143 is reserved for T-008 round 3. GitHub CI is green through `e109d74` (run 174, the commit before the T-017 merge); the runs for the merge push and later had not completed when last checked at about 16:59Z — check again before calling the merge green on GitHub. Every ledger push starts a CI run of about three minutes (179 so far); the owner has been told.
- 17:21Z (time from the clock): run ids used up to R-147. Decisions up to D-084. Proposed to the owner, with no reply yet and taken as the default: close iteration 1 once T-018 has merged (evaluator, then retrospective, then methodology v1.2), and move the remaining drafted tasks to iteration 2; record it as a decision when taken. Rows in this table no longer carry start times: two were typed from memory today and were wrong.
- 17:24Z (time from the clock): the owner replied "Yes, if i dont reply, you decide the best course" (D-085): iteration 1 closes once T-018 has merged. Also in flight: T-035 (low tier, one charter sentence), implementer R-148; on its report the director checks AC-1 on the diff (`git diff main -- docs/CHARTER.md`: only item 2 of section 3, equal to the spec's after-block), books the run, then in_progress → self_verified → queued and `cargo xtask queue T-035` (low tier: no join, no review, no approval row).

## Addendum 2026-11-13 17:41Z (director) — supersedes the "In flight" rows for T-018, T-035 and the T-008 follow-up
- **Merged in iteration 1: twelve tasks.** T-035 merged at `2e54d25` (queue attempt 1, gate 11 steps, 638 tests); its worktree is removed.
- **T-018** is the last task of iteration 1. Rework re-joined green at `f8d2abb887df18c5971acb9edba52650b44c1dc7`. Review round 2 (R-149, `reviews/T-018-r2.md`, committed on the task branch as `398650a`): request-changes on one documentation sentence. Ruling x7 (D-087). State `in_progress`; implementer run R-150 is making the fix in `.worktrees/T-018`. Then: book R-150, `self_verified`, `cargo xtask join T-018`, `tests_joined`, round 3 by the same reviewer confirming only this fix (it must write the new full head as `reviewed_sha`), then a decisions row beginning `Extend T-018 to <N>` (756,045 tokens booked through R-149; ceiling 500,000), `reviewed`, `queued`, queue.
- **T-036** (follow-up to T-008, high tier) is drafted and committed, planner run R-145 booked. Its questions Q-1 to Q-3 are unanswered and it has had no spec check. It is iteration-2 work.
- **After T-018 merges** (D-085): record the close of iteration 1; evaluator (fresh, booked to `iter-1`, no source/tests/reviews) writes `evaluations/iter-1.md`, published with `cargo xtask publish`; then the retrospective analyst; then methodology v1.2; then the planner re-tags the remaining drafts to iteration 2.
- Runs are booked through R-149. `cargo xtask metrics transition` takes `--task T-NNN --to <state>` (a positional form writes nothing and prints an error).
