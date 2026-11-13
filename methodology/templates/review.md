---
task: T-NNN
round: 1                 # file name reviews/T-NNN-rN.md
run: R-NNN
tier: medium             # medium | high
arm: independent         # independent (high; medium even id) | implementer (medium odd id)
reviewed_sha: <full 40-hex head of task/T-NNN after join>
previous_sha: null       # round ≥ 2: the sha of the last verdict (review only the delta)
verdict: request-changes # approve | request-changes
findings: {defect: 0, spec_gap: 0, test_gap: 0, nit: 0}
mutants: {introduced: 0, equivalent: 0, killed: 0, killed_by_acceptance: 0, survived: 0}
follow_ups: 0            # findings sent to a follow-up task (round ≥ 2, or spec gaps)
---

# Review T-NNN round N

## What I ran
Exact commands and result lines (gate, focused tests, CLI or control calls, toy repositories).

## Acceptance criteria
| AC | Behaviour observed (how) | Test(s) that cover it | Meets spec? |
|---|---|---|---|
| AC-1 | | | yes / no / partly |

## Checklist (yes / no / n/a, with a note when not yes)
- Every AC met through the public interface; each scorecard item has a CLI/control test.
- No inert path; no vacuous test; acceptance tests unchanged by the implementer; none skipped.
- Diff inside `touches`; no hidden CHARTER §3 contract change; layers respected; tier right (§4).
- Determinism in L0–L4; invalid params rejected; no panic on hostile input.
- High: boundary probed adversarially — what I tried: …

## Mutation spot-check (≥ 10 in round 1; round ≥ 2: in the delta)
| # | Bug seeded (path:line, change) | Killed by (test) or survived | Equivalent? |
|---|---|---|---|

## Findings
| # | Class | Where (path:line) | What is wrong | Evidence |
|---|---|---|---|---|
| 1 | defect / spec gap / test gap / nit | | | |

Round 1: any defect or test gap ⇒ `request-changes`. Round 2 (final): only a defect, anywhere.
Round 3: only confirms the fix of a round-2 defect.

## Follow-ups
Round ≥ 2: test gaps, spec gaps and nits outside the delta; any round: spec gaps for the director.

## Not checked
What I could not run or judge, and why.
