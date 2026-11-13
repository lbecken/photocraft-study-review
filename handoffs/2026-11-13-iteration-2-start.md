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
