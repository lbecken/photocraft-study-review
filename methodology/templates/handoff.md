---
date: YYYY-MM-DD         # one handoff file per day
role: director
iteration: N
methodology: v1.5
main_sha: <sha>
main_green: true         # last gate / CI result on main
---

# Handoff YYYY-MM-DD

File: `handoffs/YYYY-MM-DD-<slug>.md`, one per day. Builder runs do not write handoffs; their record is
the task log. Later the same day, add a section `## Addendum <UTC time, from the clock>` naming
the stop or restart before it and what it supersedes. This note is for the next director session.

## Done
| Task | State change | Notes |
|---|---|---|

## In flight
| Task | State | Run(s) out | Waiting on |
|---|---|---|---|

## Blocked and questions
| Task | Blocker / question | Needs |
|---|---|---|

## Decisions recorded
D-NNN rows and ADRs added.

## Metrics hygiene
- Runs dispatched: n; run records booked when the run ended: n; log entries present: n.
- Transitions recorded, and the task file's `state` line written, for every state change: yes / no
  (list gaps).
- Runs not yet in `metrics/`: run, role, tokens, start, end. Spec checks are booked to `iter-N`.
- Usage-limit stops: time, resume scheduled (yes / no). Restarts: `uptime -s`, `git status` run in
  every worktree (yes / no). Stopped agents resumed with a restart notice (yes / no).
- Newest CI run on main that changed a non-ledger path read before each dispatch or queue run: yes / no
  (run ids; red ⇒ incident). Ledger-only pushes since the last addendum, read in a batch: run ids, conclusions.
- Accepted decisions not yet done (at a pause or stage close): each, and where it went.
- Free disk at the end (`df -h`): … GB; worktrees and scratch builds removed: yes / no.

## Incidents
Open incidents and red-main status.

## Next concrete steps
1. …
2. …

## STATE.md
Updated: yes / no.
