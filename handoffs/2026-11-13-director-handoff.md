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
| Merged in iteration 1 so far | T-006, T-012, T-014, T-013, T-015, T-016, T-032, T-010 | Queue enforces budgets (`Extend T-NNN to <N>` rows); transitions go in_progress → self_verified → tests_joined → reviewed → queued → merged. |
| T-017 regex (high) | reviewer R-117 running (round 1) | Joined green at `43fae90…` with the 100-capture-group cap (D-060). On approve: row beginning `Approve T-017` (dependency change), queue. |
| T-011 manifest seeding (medium, implementer-writes-tests arm) | reviewer R-123 running (round 2, final) | Re-joined green at `d0f9416…` after rulings u1–u5 (D-067). Medium: no approval row; check the budget line before queuing. |
| T-008 path confinement (high) | test author R-120 done (`a173c44…` on the tests branch); implementer R-122 running | Rulings v1, v2 (D-068). Then join, review (security boundary), approval row, queue. Adds a reason row: no other row-adding task in flight (T-020 waits). |
| Ready, not dispatched | T-018 (medium, even: implementer + independent test author) | Dispatch when a compile slot frees. Then T-020 (after T-008 merges), T-030 (not with T-008), T-035 (low, CHARTER sentence), T-007 (high, needs spec check), T-019, T-027 (high, after T-008; need spec checks). |
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
