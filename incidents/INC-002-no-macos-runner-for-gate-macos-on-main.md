---
id: INC-002
title: "A counted job on main has no result: GitHub assigned no macOS runner to gate-macos"
class: process           # not red-main: the job did not fail, it never ran
detected_at: 2026-11-18T19:20Z
detected_by: the director, from the record of main's `platforms.yml` run after T-074's merge (helper run R-388)
platform: macos
red_from: 2026-11-18T19:06Z    # `gate-macos` of run 44373961129 cancelled after 15 min 01 s with no runner (created about 18:51Z)
green_at: null                 # the next completed `gate-macos` on main; a re-run of the run's failed and cancelled jobs was queued at 19:20Z
red_minutes: 0                 # no red minute: no result
origin_task: null        # not a task's defect
fix_task: null
---

# INC-002 A counted job on main has no result: GitHub assigned no macOS runner to gate-macos

## What happened
The push of 18:50Z on 2026-11-18 carried T-074's merge (`25054fe`, pushed head `4eb349d`). Its `platforms.yml` run
44373961129 (record: `tasks/ci-runs/T-074-round-1.md`, "Main after the merge"): `window-probe (macos-15)` success
(a runner was assigned, `PROBE: exit status 0`), `window-linux` success, the Linux gate success in `ci.yml` run
44373961102 (1200 passed); **`gate-macos` cancelled after 15 minutes 01 second with no runner assigned and no step
run**. `gate-macos` has been a counted job since D-248. It did not fail; it has no result for this commit.

The same has happened on the branch `ci/candidate` since about 13:00Z (D-261, D-264, D-270): macOS jobs are
cancelled after 15 minutes with `runner_id` 0, for `window-probe (macos-15)` in ten runs and for `gate-macos` in
three, while other macOS jobs of the same hours ran. GitHub's status page showed no incident at 16:20Z. The cause is
not known from inside this repository (VERIFIED: the cancellations and their times, from the job data; not known:
why). The account's Actions usage and limits are the owner's to see.

What is known about the tree on macOS: the CI candidate `720f1d5`, whose code paths equal the merged tree
(D-271), had `gate-macos` green (run 44358453526, 1183 passed, 0 failed); the previous push to main (T-072's merge)
had `gate-macos` running with its gate step green when last read (run 40).

## Timeline (UTC)
| Time | Event |
|---|---|
| 13:05 | first `window-probe (macos-15)` cancelled with no runner, on `ci/candidate` (run 44307677646) |
| 15:31 | first `gate-macos` cancelled with no runner, on `ci/candidate` (run 44333697372) |
| 16:48 | macOS runners assigned again for some runs (D-264) |
| 18:49 | T-074 merged by the queue; pushed 18:50 |
| 19:06 | `gate-macos` of main's run 44373961129 cancelled, no runner |
| 19:20 | detected; a re-run of that run's failed and cancelled jobs queued |

## Why the gate and queue did not stop it
Nothing was wrong with the tree: the local gate and the queue's gates passed, and the candidate's macOS gate was
green. The hosted macOS job is evidence the project does not control. D-262 and D-270 ruled how an approval treats
a counted job that the host cancels before any step; there was no rule for the same on main.

## Immediate fix
Re-run until a runner is assigned; record the result here (`green_at`). Dispatching is not stopped: no counted job
is red, and the candidate's result covers the same code.

## Permanent fix
| Type | Description | Task |
|---|---|---|
| Rule only — the host's capacity cannot be checked from the gate | A counted job cancelled before any step with no runner assigned has no result. On main it is re-run until it has one, and an incident file is open meanwhile; on a candidate, D-262 as widened by D-270 says when an approval may go on. To enter the next methodology version (§8.1). | next methodology version |
| Mechanical check | The CI record could mark such a job "no result (no runner)" by its `runner_id` instead of a helper finding it by hand each time. | T-091 (note added) |
| For the owner | Whether the account's Actions usage or a concurrency limit explains the missing runners; fewer candidate runs in parallel on `ci/candidate` would lower the load on macOS runners in any case. | owner |
