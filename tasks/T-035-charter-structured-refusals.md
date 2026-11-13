---
id: T-035
title: "State in docs/CHARTER.md §3 that refusals carry structured reasons (ADR-0004)"
tier: low               # ADR-0002: documentation only; the contract itself changed in T-032 (high) under the accepted ADR-0004
state: merged
owner_role: planner
scorecard_items: []
depends_on: [T-032]     # merged: the behaviour the sentence states is on main
touches:
  - docs/CHARTER.md              # §3 decision 2 only: one appended sentence (not a gate path)
budget_tokens: 150000   # low (v1.1)
iteration: 1
found_by: planner
origin_task: T-032
---

# T-035 CHARTER §3 decision 2 records structured refusal reasons

## Goal
ADR-0004 (accepted, D-050) changed `CommandError`, a CHARTER §3 contract, and T-032 built it
(merged, D-057/D-058). ADR-0004 "Consequences" says the director publishes the CHARTER §3
decision 2 sentence separately, as a low-tier task (D-042). After this task CHARTER §3 decision
2 says what the registry's refusals are, so a reader of the binding architecture learns it
without reading the ADR. No behaviour, code, test or generated file changes. No gate path.

Reading list: `docs/adr/0004-structured-refusal-reasons.md` ("Decision", "Consequences");
`docs/CHARTER.md` §3 "Decisions", item 2; `docs/decisions.md` D-042, D-050.

## Interface
`docs/CHARTER.md`, §3 "Decisions", item 2. Today it reads (3-space continuation indent):
```
2. **One command registry.** Commands declare typed params/results (serde + JSON Schema); the
   registry rejects invalid params. UI, CLI, control protocol and tests all go through it.
   (PhotoCraft planned a validated schema and never built it.)
```
After this task it reads exactly:
```
2. **One command registry.** Commands declare typed params/results (serde + JSON Schema); the
   registry rejects invalid params. UI, CLI, control protocol and tests all go through it.
   (PhotoCraft planned a validated schema and never built it.) Refusals carry a structured
   reason; the protocol's error kinds are derived from it and listed in the command
   description generated from code (ADR-0004).
```
The sentence is ADR-0004's own wording ("Consequences": "CHARTER §3 decision 2, appended: …"),
with the reference `(ADR-0004)` added before the full stop, as CHARTER §4 cites ADR-0002.
Nothing else in `docs/CHARTER.md` changes.

## Pinned
- (a) Errors: n/a (no behaviour). · (b) Files: `docs/CHARTER.md` only, the lines above; no
  generated file changes (`docs/scorecard.md` and `docs/commands.json` do not read the charter).
- (c) Order of checks: n/a. · (d) Paths: n/a (one Markdown file).
- (e) Threat model: n/a (not a gate or security path; `docs/CHARTER.md` is not on the gate-path
  list in `docs/development.md`).
- (f) Tests pinning this file: `xtask/tests/acceptance_t013_hygiene.rs` reads CHARTER §4's gate
  line only ("Steps, cheap first:" … "enforced budgets"); §3 is not read by any test (grep
  `CHARTER.md` in `crates/`, `apps/`, `xtask/`). It stays green.
- (g) Worked example: the "after" block above is the whole diff (3 lines become 5).
- (h) Scorecard items: none.

## Acceptance criteria
- AC-1 (checked on the diff by the reviewer — for this low-tier task the director — no test):
  `git diff main -- docs/CHARTER.md` changes only item 2 of §3 "Decisions", and the item equals
  the "after" block of the Interface (line breaks may differ; words and punctuation exact); no
  other path changes; `cargo xtask ci` is green.

No acceptance or guard test file: a test that pins charter text would only repeat this diff
check (D-052: a check that the current task changed only some text belongs to the review, not
to a permanent test).

## Out of scope and implementer notes
- Any other charter edit, including §4 and the layer table; any ADR or `docs/control-protocol.md`
  edit; code.
- Commit with `Task: T-035` and `Role: implementer`.

## Questions and rulings
None open.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13T12:53Z · R-112 · (new) · did: specified the CHARTER §3 decision 2 sentence owed
  since ADR-0004 was accepted (D-042, D-050): ADR-0004's wording, appended to item 2, with
  `(ADR-0004)`. Tier low: documentation only; the §3 contract change itself was T-032's (high)
  under the accepted ADR, so this edit records an existing contract and changes no behaviour
  (ADR-0002's high criterion is the contract change, already made). Pinned (a), (c), (d), (e),
  (h) n/a (no behaviour, no boundary, no item). One AC, checked on the diff; no test, by D-052's
  reasoning. · gate attempts: 0 · not run: no cargo · missing context: none · next: director.
  QUESTION: keep `(ADR-0004)` in the sentence (my choice, like §4's ADR-0002 citation), or use
  ADR-0004's words with no reference?
<!-- end planner log -->
### implementer
- 2026-11-13T17:29Z · R-148 · ready · did: checked CHARTER §3 item 2 on base
  73d35f55018275c0b937dc1f1e0bd35491d4b3cc (merge base; main has since moved to
  af61bda078ed4b9df565ca4ddb3974b7188bd6a8, ledger-only commits) equals the "Today it reads" block; appended the
  ADR-0004 sentence so item 2 equals the "after" block (3 lines become 5, 3-space indent);
  `git diff main -- docs/CHARTER.md` (and main...HEAD) shows only item 2 changed; `git status` shows no other path
  · gate attempts: 1 (last: green; fmt, clippy, layers, ui-commands, hygiene, schema, scorecard,
  test 638 passed, no-skip; fuzz, budgets EMPTY) · not run: nothing beyond the gate (no
  behaviour to drive) · missing context: none · next: director review (AC-1 on the diff).
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
