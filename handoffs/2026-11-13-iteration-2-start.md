# Director handoff: start of iteration 2 (written 2026-11-13 21:46Z; supersedes `2026-11-13-director-handoff.md` for everything in flight)

## Where the experiment stands
- Iteration 1 is closed (D-089, corrected by D-091): twelve tasks merged; evaluation published (`evaluations/iter-1.md`,
  T-040): 28 of 64 scorecard items verified. Retrospective: `retrospectives/iter-1.md`; decisions on its 18 changes: D-091.
- Methodology **v1.2** is in force (D-093). ADR-0005 stages iteration 2: first stage = T-019, T-020, T-021, T-023 (13 items,
  to reach 60%), T-036, T-037, T-038, T-039, the tooling tasks; second stage = UI tasks, not yet drafted.
- Merged in iteration 2 so far: T-039 at `c538ef8c28f973fcc55974844fc7ccaf550fb307`.

## In flight
| Task | Who | State |
|---|---|---|
| T-019 project tree (high) | implementer R-163 in `.worktrees/T-019`, independent test author R-164 in `.worktrees/T-019-tests` | in_progress since 21:44:58Z. Spec checked (R-156, R-162), rulings Q-1 to Q-8 in the task file. Next: book each run when it ends, answer questions to both builders, regenerate the scorecard page on the branch, `cargo xtask join T-019`, review round 1 (v1.2 §7: every related document sentence against the binary; no revert check), approval row at merge (adds the `ignore` crate). |
| T-020 syntax (medium, even: independent test author) | spec check R-157 done; the same planner is finalising the spec on rulings Q-1 to Q-6 (run R-165) | Then: specified, ready, two worktrees, dispatch when 15 GB are free. A follow-up task for a deterministic parse work cap is to be filed (D-097). |
| T-021 search (medium, odd: implementer writes acceptance tests first) | spec check R-158 running; it measures search time at the pattern cap and the memory tests | Rule on its questions, finalise, dispatch as disk allows. |
| Tooling specs T-033, T-041, T-042, T-043 | planner R-160 writing in `tasks/` | Read, rule, commit. T-043 includes: the queue must not refuse for a dirty ledger file it does not read (three refusals today). |

## Waiting, in order
T-023 after T-019 merges (one spec check first). T-036 after T-019 and T-039 (rulings in its file; needs the planner to apply
them and one check), then T-037. T-038 after the four feature tasks. Deferred drafts: T-007, T-009, T-022, T-024 to T-031, T-034.

## Things the next session must know
- **Disk**: a task worktree's build takes about 6.3 GB; check `df -h /home/user` before each dispatch (15 GB free). Remove a
  worktree as soon as its task merges. Main's `target/` holds stale artifacts and can be cleaned when nothing is running.
- **The queue and publish refuse when any ledger file is dirty**, including files planners are writing in the primary
  checkout. Until T-043: commit such files by path as "WIP" first, then start the queue at once.
- No commit on main while a queue or publish runs; no `metrics` write while a join runs.
- `cargo xtask metrics transition --task T-NNN --to <state>`; book a run before writing later transitions of its task.
- Spec checks and planner runs are booked to `iter-2` as role `planner` (note beginning `spec check:` for checks).
- Before queueing any task, read every sentence it adds to `docs/` against the binary (D-095: one was wider than the binary).
- Left open: `kiln-cli --help` and `docs/protocol/cli-memory-seed.md` say "nothing is written to disk", which is too wide
  because of `--log` (D-096); T-036's file needs the planner to apply D-092's rulings.
- A check-in is scheduled for 02:15Z on 2026-11-14 (the usage window began about 21:10Z).
- Last run id used: R-165.

## Addendum 2026-11-13 23:26Z (director) — supersedes the "In flight" table above
- **Merged in iteration 2:** T-039, T-042, T-019 (at `3b1a95d865619dead945cbf1ed857afdfc8fe99e`; `ws.open-project` is implemented, to be confirmed by the evaluator).
- **T-020** (syntax, medium, independent tests): joined green on its first join at `c0c12fc30d473ff1d0f5b71c8ce9d4a0b66e0c12`, state `tests_joined`; review round 1 (run R-179) is running in `.worktrees/T-020`. Then: book the run, rule on findings, rework if any, re-join, round 2, queue (medium tier: no approval row).
- **T-021** (search, medium, implementer wrote the tests first): built (runs R-173, R-177), state `self_verified`; tests merged and scorecard tied on the branch at `46b93d64a018268b60528146cf16f55a36f10555`; `cargo xtask join T-021` is running. Then: `tests_joined`, a fresh reviewer for round 1 (tell it about the second tests commit `46bc135` made after code existed).
- **Spec checks running:** T-023 (R-180; dispatch only after T-020 and T-021 merge) and T-036 (R-181; applies D-092's rulings; dispatch after T-020 and T-021 have left their worktrees, since disk is short; T-037 follows it).
- **Waiting:** T-038 (after the feature tasks), tooling T-033, T-043, T-041 (in that order; T-041 last and the owner is told before it is queued). To file: one follow-up task for deterministic limits on parse work, search work and `.gitignore` cost (D-097, D-101, D-105).
- **Join procedure used under v1.2 (until T-033):** in the task worktree, `git merge --no-ff task/T-NNN-tests` with the message "Join task/T-NNN-tests into task/T-NNN" and trailers `Task:` / `Role: director`; add the test names to the item files under `scorecard/items/`; `cargo xtask scorecard`; commit both (`Role: director`); then `cargo xtask join T-NNN` from the primary checkout. All three joins so far passed on the first attempt this way.
- **Disk** stays the limit: a join needs about 4 GB in the tests worktree for the base build plus the gate's build. Delete `target/` of a worktree whose agent has finished, and of the tests worktree after each join. `cargo xtask publish` leaves 4 GB under `.git/kiln-publish/target`.
- Last run id used: R-181. The 02:15Z check-in is still scheduled.

## Addendum 2026-11-14 02:14Z (director) — after the usage-limit stop and restart (D-112)
- **Merged in iteration 2:** T-039, T-042, T-019. Scorecard on main: 28 verified, 1 implemented.
- **T-020:** review round 1 was request-changes (D-110; cap row D-109: `Extend T-020 to 880000`). State `in_progress`. Test author's rework done (run R-185, head `71417863bbdb1356f47bc985f3d3c5c489a9b1f5` on `task/T-020-tests`; one new test `refusals_leave_the_cache_unchanged`, to be tied to `syntax.parse`). Implementer's rework (run R-184) is committed at `ef16572` on `task/T-020`; the agent was resumed after the restart to run the gate and write its log entry. Then: book R-184, merge the tests branch into `task/T-020` again, add the new test to `scorecard/items/syntax.parse.toml`, regenerate, commit, `self_verified`, join, round 2 by the same reviewer (delta `c0c12fc..<new head>`), queue.
- **T-021:** joined green on the second join at `9dcd59d41b4763a2b9eedc19239fbe867dfc6d52` (the first failed the `no-skip` step, D-108). State `tests_joined`. Review round 1 (run R-187) is running in `.worktrees/T-021`.
- **Specs final, not yet ready:** T-023 (make ready after T-020 and T-021 merge; check its "To re-check at dispatch" list on main) and T-036 (items 1 to 4; make ready when a slot and disk are free). T-044 is a new draft (split from T-036), outside the first stage.
- **Then:** T-037 (pre-dispatch check after T-036 merges), T-038 (after the feature tasks), tooling T-033, T-043, T-041; the follow-up on deterministic limits is still to be filed (parse work, search work, `.gitignore` cost, the size of a highlight reply).
- Runs are booked through R-186 (R-184 and R-187 are running). Check-in scheduled for 07:15Z.
