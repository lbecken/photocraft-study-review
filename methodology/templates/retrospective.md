---
iteration: N
run: R-NNN
methodology_in: v1.1
methodology_proposed: v1.2
main_sha: <sha at iteration close>
claims: {C1: insufficient, C2: insufficient, C3: insufficient, C4: insufficient,
         C5: insufficient, C6: insufficient, C7: insufficient, C8: insufficient}
         # each: supported | refuted | insufficient
---

# Retrospective, iteration N

## Data
Record counts and completeness: tasks, runs, runs without a log entry or run record,
transitions missing, director runs missing. Every number below names its command or script.

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
| Review rounds per task; acceptance kill rate by round | | |
| Known defects merged by decision | | |
| Tokens per merged task by tier and arm (median, p75, max) | | |
| Director cost | | |
| Share of scorecard items verified | | |
| Red-main minutes | | |
| Budget overruns by tier | | |

## Experiment arm (METHODOLOGY §6)
| Measure | Independent (even) | Implementer-written (odd) | n each |
|---|---|---|---|
| Escaped defects / review findings / kill rate / tokens per merged task | | | |

## Claims
| Claim | Verdict | Numbers that decide it | What would change the verdict |
|---|---|---|---|
| C1 … C8 | | | |

## Where the methodology was wrong
Rules not followed (with task IDs), rules that cost effort without effect, gaps that let a
defect through, open questions O-n now answerable.

## Proposed changes
| # | Change (file, section) | Motivating metric or incident | Expected effect | Check | Removes |
|---|---|---|---|---|---|

## Budget recalibration
Proposed tier budgets from the recorded distribution, with the rule used.

## Uncertainty
Small samples, confounders, numbers that rest on director classification.
