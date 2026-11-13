---
date: YYYY-MM-DD
session: sNN             # director session sequence number (not a platform session ID)
role: director
iteration: N
methodology: v1.1
main_sha: <sha>
main_green: true         # last gate / CI result on main
---

# Handoff YYYY-MM-DD sNN

File: `handoffs/YYYY-MM-DD-sNN.md`. Builder runs do not write handoffs; their record is the
task log. This note is for the next director session.

## Done this session
| Task | State change | Notes |
|---|---|---|

## In flight
| Task | State | Run(s) out | Waiting on |
|---|---|---|---|

## Blocked and questions
| Task | Blocker / question | Needs |
|---|---|---|

## Decisions recorded
D-NNN rows and ADRs added this session.

## Metrics hygiene
- Runs dispatched this session: n; run records written at dispatch: n; log entries present: n.
- Transitions recorded for every state change: yes / no (list gaps). Director run recorded: yes / no.
- `iter-N` runs not yet in `metrics/` (interim until T-013): run, role, tokens, start, end.
- Free disk at session end (`df -h`): … GB; worktrees and scratch builds removed: yes / no.

## Incidents
Open incidents and red-main status.

## Next concrete steps
1. …
2. …

## STATE.md
Updated: yes / no.
