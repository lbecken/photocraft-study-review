---
iteration: 1
run: R-NNN                # booked to iter-N
main_sha: <sha evaluated>
interfaces: [cli, control, ui-snapshot]
confirmed_items: []      # scorecard item IDs confirmed; read by `cargo xtask scorecard`
rejected_items: []       # item IDs whose behaviour did not hold
defects_filed: []        # task IDs created as draft
not_evaluated: []        # item IDs not reached, with reason below
---

# Evaluation, iteration N

## Setup
How the product was built and started (commands), fixture projects used.

## Items
One block per scorecard item with status `implemented`.

### <item-id>
- Claimed behaviour (from the scorecard): …
- What I did (exact CLI / control-protocol requests, UI snapshot steps): …
- What I observed (output excerpt, snapshot path): …
- Verdict: confirmed / rejected. If rejected: filed T-NNN.
- Inert-feature check: did the setting/command change anything observable? yes / no.

## Wired but inert
Commands, settings or UI controls that exist but had no observable effect, including ones not
on the scorecard.

## Defects filed
| Task | Item | Summary | Suspected origin task |
|---|---|---|---|

## Not evaluated
Items I could not reach and why (missing interface, crash, time), with the task filed for each.

## Questions for the director
Observed behaviour I cannot classify as a defect or not. Every observation is either a filed
task, a covering task named, or a question here.
