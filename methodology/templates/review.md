---
task: T-NNN
round: 1                 # r1, r2, … ; file name reviews/T-NNN-rN.md
run: R-NNN
tier: medium             # medium | high
reviewed_sha: <head of task/T-NNN after join>
previous_sha: null       # for round ≥ 2: the sha of the last verdict (review the delta)
verdict: request-changes # approve | request-changes
findings: {defect: 0, spec_gap: 0, test_gap: 0, nit: 0}
---

# Review T-NNN round N

## What I ran
Exact commands and their result lines (gate, focused tests, headless CLI or control calls).

## Acceptance criteria
| AC | Behaviour observed (how) | Test(s) that cover it | Meets spec? |
|---|---|---|---|
| AC-1 | | | yes / no / partly |

## Checklist (answer each: yes / no / n/a, with a note when not yes)
- Every AC is met through the public interface, not only inside unit tests.
- No inert path: each new command, setting or branch has an observable effect that a test asserts.
- No vacuous test: each test asserts an effect; no early return; no mock that passes regardless.
- Acceptance tests unchanged by the implementer; none skipped or ignored.
- Diff inside `touches`; no hidden CHARTER §3 contract change; layers respected.
- Determinism: no wall clock, randomness or thread timing in L0–L4 behaviour.
- Errors: invalid params rejected by the registry; no panic on hostile input.
- If fail-on-base was by compile error only: revert check done (remove the core of the
  change, at least one acceptance test fails) — result: …

## Findings
| # | Class | Where (path:line) | What is wrong | Evidence |
|---|---|---|---|---|
| 1 | defect / spec gap / test gap / nit | | | |

Any defect or test gap ⇒ `request-changes`. Spec gaps also go to the director.

## Not checked
What I could not run or judge, and why.
