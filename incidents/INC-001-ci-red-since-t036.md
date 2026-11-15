---
id: INC-001
title: "CI on main red since the merge of T-036: three confinement tests fail on the hosted runner"
class: red-main
detected_at: 2026-11-15T16:37Z
detected_by: planner of ADR-0009 (run R-255) reading GitHub Actions; confirmed by the director (runs 353 to 357, job log of run 355)
red_from: 2026-11-14T04:34Z    # run 257, the first push after T-036 merged at 1a808c9
green_at: null                 # recorded in docs/decisions.md when the first green run on main is read
red_minutes: null              # computed with green_at
origin_task: T-036
fix_task: T-069
---

# INC-001 CI on main red since the merge of T-036

## What happened
Every run of `ci.yml` on main from run 257 (`47135bc`, the first push after T-036 merged at `1a808c9`) failed at
the gate's `scorecard` step: three tests of `ws.path-confinement` added by T-036 re-run themselves under another
identity starting from uid 0, and the hosted runner's user is not uid 0, so they fail there and the scorecard
page counts as stale. The local gate, which runs as uid 0, stayed green, and the queue merged eight further
tasks on it (T-023, T-037, T-038, T-049, T-055, T-056, T-065, T-067). Nobody read CI: the director's check after
a push compared commit ids only. Retrospectives and decision rows that say main was never red describe the local
gate, not CI.

## Timeline (UTC)
| Time | Event |
|---|---|
| 2026-11-14T04:33Z | T-036 merged by the queue at `1a808c9` (commit time 04:24:31Z; the queue's record commit `e42b5e5` at 04:33:18Z; local gate green) |
| 2026-11-14T04:34Z | first push with T-036 on main (`47135bc`, 04:34:26Z); its run 257 fails, and every later run fails the same way |
| 2026-11-15T16:37Z | detected by the planner of ADR-0009 while measuring CI times; confirmed by the director |
| 2026-11-15 | dispatching stopped except for the fix (T-069) |
| see docs/decisions.md | fix merged by the queue; first green run read |

## Cause
A test that needs a property of the machine (uid 0, a checkout another identity can reach) and reports a failure
when it is absent, combined with a gate that runs on one machine and a CI result nobody reads.

## Permanent fix
- T-069: the tests pass as an ordinary identity and as uid 0, in a checkout anywhere, or fail naming the
  precondition.
- Rule (methodology v1.4): after every push the director reads the CI run of the pushed commit and records its
  result; a red run is an incident at once. The queue's statement "main is green" means the local gate and CI.
- ADR-0009: the gate runs on three platforms in CI, so a difference between machines shows on the next push.
