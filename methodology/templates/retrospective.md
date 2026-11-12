---
iteration: N
run: R-NNN
methodology_in: v1.0
methodology_proposed: v1.1
main_sha: <sha at iteration close>
claims: {C1: insufficient, C2: insufficient, C3: insufficient, C4: insufficient,
         C5: insufficient, C6: insufficient, C7: insufficient, C8: insufficient}
         # each: supported | refuted | insufficient
---

# Retrospective, iteration N

## Data
Record counts and completeness: tasks, runs, runs without a log entry or run record,
transitions missing. Every number below names the command or script that produced it.

## Metrics (METHODOLOGY §13)
| Metric | Value | Command |
|---|---|---|
| Lead time (median, p75) by tier | | |
| First-pass queue success | | |
| Gate attempts per task (median) | | |
| Acceptance-caught defects | | |
| Review findings by class and tier | | |
| Escaped defects (by origin task, tier) | | |
| Rework ratio | | |
| Tokens per merged task by tier (median, p75, max) | | |
| Share of scorecard items verified | | |
| Red-main minutes | | |
| Budget overruns by tier | | |

## Claims
| Claim | Verdict | Numbers that decide it | What would change the verdict |
|---|---|---|---|
| C1 … C8 | | | |

## Where the methodology was wrong
Rules not followed (with task IDs), rules that cost effort without effect, gaps that let a
defect through, open questions O-n now answerable.

## Proposed changes
| # | Change (file, section) | Motivating metric or incident | Expected effect | Check |
|---|---|---|---|---|

## Budget recalibration
Proposed tier budgets from the recorded distribution, with the rule used.

## Uncertainty
Small samples, confounders, numbers that rest on director classification.
