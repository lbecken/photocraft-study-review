---
id: T-042
title: "State the two v1.2 builder rules in AGENTS.md and the two stages of iteration 2 in CHARTER §5"
tier: low               # ADR-0002: documentation only; neither file is a gate path (precheck.rs is_gate_path)
state: queued
owner_role: planner
scorecard_items: []     # low tier carries no scorecard item (METHODOLOGY §4)
depends_on: []          # nothing unmerged; no spec check is required by METHODOLOGY §5.7
touches:
  - AGENTS.md           # implementer: the two sentences of Interface (1) only
  - docs/CHARTER.md     # implementer: the iteration-2 row of §5 only, Interface (2)
budget_tokens: 150000   # low (METHODOLOGY §11, v1.2)
iteration: 2
found_by: planner       # D-091 ("rules in docs/development.md and AGENTS.md follow through a low-tier task"), D-093, ADR-0005 Consequences
origin_task: null
---

# T-042 AGENTS.md and CHARTER §5 follow methodology v1.2

## Goal
After this task a builder who reads only `AGENTS.md` (as every builder does first) meets the two
v1.2 builder rules that today live only in the role briefs and the methodology, and
`docs/CHARTER.md` §5 says what ADR-0005 decided for iteration 2. Both are follow-through of
accepted decisions; nothing new is decided here.
- The rules: a builder acknowledges each ruling in its log before acting on it (v1.2 change 4,
  METHODOLOGY §2.1); no test pins a message text, a whole document or a command list that a
  drafted task changes (v1.2 change 6, METHODOLOGY §6). D-093 lists both as missing from
  `AGENTS.md`. Evidence: 14 ruling errors in iteration 1, 9 of which reached a builder; six
  incidents of tests pinning what later tasks change (D-043, D-046 twice, D-052, D-069, T-010 r1
  nit 3) (retro-1; CHANGELOG v1.2 rows 4 and 6).
- The charter: ADR-0005 (accepted by D-091 change 16) stages iteration 2 and names the CHARTER §5
  row as the text to change through a low-tier task, because `docs/CHARTER.md` is not a ledger
  path (ADR-0005 Consequences; retro-1 change 15 deferred).

**Tier.** Low: documentation with no behaviour change. Neither path is a gate path:
`precheck.rs` `is_gate_path` covers `xtask/`, `.github/`, `.cargo` directories, build scripts and
tool configuration files, and the queue's `gate-path` check would refuse a low task otherwise.
Changing CHARTER §5 is allowed here because ADR-0005 decided it (CHARTER: "change only by ADR"
binds §3; §5 is changed as ADR-0005 says). No gate path, no dependency, no approval row.

The five `docs/development.md` passages that D-093 assigned to this task are not in it: by the
director's request to R-160 they belong to the tasks whose tools change them (four to T-033, the
CI one to T-041), so each passage changes with the behaviour it describes (Q-1).

Reading list: `AGENTS.md`; `docs/CHARTER.md` §5; `docs/adr/0005-iteration-2-in-two-stages.md`;
`methodology/METHODOLOGY.md` §2.1 (rulings), §6 (Pins and documents); `docs/decisions.md` D-091,
D-093, D-094.

## Interface
Copy these sentences exactly (METHODOLOGY §6: guarantee and rule sentences come from the spec).

### (1) `AGENTS.md`
- In "## Golden rules", after rule 8, add rule 9 as its own numbered item:

  `9. **No brittle pins.** No test pins a message text, a whole document or a command list that a drafted task changes; that this task left something alone is the reviewer's diff check, not a guard (METHODOLOGY §6).`

- In "## Picking up a task", directly after the bullet that begins `- Read the task log first:`,
  add the bullet:

  `- Acknowledge each ruling in your log, accepting it or questioning it, before you act on it (METHODOLOGY §2.1).`

Line breaks inside a sentence may follow the file's wrapping at about 98 columns; the words and
punctuation are as above. Nothing else in `AGENTS.md` changes.

### (2) `docs/CHARTER.md` §5
Replace the table row that begins `| 2 Product |` with this row (one line, exactly as in the
block):

```text
| 2 Product | in two stages (ADR-0005): stage 1 reaches the 60% line that iteration 1 missed, with the tasks ADR-0005 lists; stage 2 is the UI, end-to-end flows, budgets and hardening, drafted after stage 1 | stage 1: ≥60% scorecard items verified, read from `cargo xtask scorecard` after stage 1's evaluation, before any stage-2 task is dispatched; stage 2: every item verified; evaluator sign-off; budgets enforced |
```

Nothing else in `docs/CHARTER.md` changes; in
particular §4's gate line, which `acceptance_t013_hygiene.rs` reads, stays as it is.

## Pinned
- (a) **Errors, payloads, precedence.** n/a: no behaviour.
- (b) **Files.** `AGENTS.md` and `docs/CHARTER.md`, edited as Interface says; no other file. No
  test file: the sentences are checked by the director on the diff (AC-1 to AC-3), not by a test,
  because v1.2 §6 forbids a test that pins the text of a document another task may change, and
  both documents change again at retrospectives (D-094 used the same check for T-039).
- (c) **Order of checks.** n/a.
- (d) **Paths and platform.** n/a.
- (e) **Threat model.** n/a: not a gate or security task.
- (f) **Tests pinning a snapshot this changes.** None of `AGENTS.md`;
  `xtask/tests/acceptance_t013_hygiene.rs` reads CHARTER §4's gate line only, which this task
  leaves alone (`grep -rn 'AGENTS.md\|CHARTER.md' --include=*.rs crates apps xtask` on main
  `b53d370f4a5f22eeffd34969b49f34170e0edf57` finds only that reader).
- (g) **Worked examples.** n/a.
- (h) **Scorecard items.** None.
- (i) **Guarantee sentences in `docs/protocol/`.** None.

## Acceptance criteria
Named commands, run by the director on the task branch before queueing (low tier: no review).
- AC-1: `git diff --name-only main...task/T-042` prints exactly `AGENTS.md` and `docs/CHARTER.md`.
- AC-2: in `.worktrees/T-042`, `tr -s ' \n' ' ' < AGENTS.md | grep -cF '<sentence>'` prints 1 for
  each of the two sentences of Interface (1); rule 9 follows rule 8 under "## Golden rules", and
  the acknowledgement bullet directly follows the `Read the task log first:` bullet.
- AC-3: in `.worktrees/T-042`, `grep -cF '| 2 Product | in two stages (ADR-0005):' docs/CHARTER.md`
  prints 1, `grep -c '^| 2 Product | UI, end-to-end flows' docs/CHARTER.md` prints 0, and
  `git diff main...task/T-042 -- docs/CHARTER.md` shows one removed and one added line, both in §5.
- AC-4: `cargo xtask ci` green in `.worktrees/T-042`.

## Out of scope and implementer notes
- `docs/development.md`: T-033 and T-041 (Q-1). The role briefs and `methodology/`: already v1.2
  (D-093); the director edits `methodology/` only at a retrospective.
- `docs/adr/0005-*.md` front matter (`task: null`, "not yet filed"): a ledger path the director
  updates on `main` with this task's id when it is accepted.
- Rewording or renumbering anything else in `AGENTS.md`; any other CHARTER section.
- No test file (Pinned (b)). Never set or export `CARGO_TARGET_DIR` (D-019).

## Questions and rulings
- Q-1 (planner, R-160): D-093 says this low-tier task carries five `docs/development.md` passages;
  the director's request moves four to T-033 and one to T-041. A decision row should record the
  change so that D-093 and the task files agree. Recommended as requested: a passage that
  describes a tool changes in the task that changes the tool, so the reviewer checks both against
  the same binary.
### Rulings (director, D-098)
- Q-1: recorded in D-098: of the five `docs/development.md` passages D-093 assigned to this task, four are in T-033 and one in T-041; this task holds only `AGENTS.md` and `docs/CHARTER.md` §5.
- ADR-0005's `task:` field is set to T-042 by the director when this task is made ready.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13T21:40Z · R-160 · (new) · did: drafted from D-091, D-093 and ADR-0005; wrote the
  exact sentences (two for `AGENTS.md`, one charter row). Tier low: documentation only, and
  neither path is a gate path by `precheck.rs` `is_gate_path` (checked, as the director asked,
  because METHODOLOGY §4 names `docs/development.md` as where the gate-path list lives, not as a
  gate path). Pinned (a), (c), (d), (e), (g) n/a: no behaviour; (h), (i) n/a. ACs are named
  commands, not tests, under v1.2 §6 (D-094 precedent). · gate attempts: 0 · not run: no cargo ·
  missing context: none · next: director (Q-1); no spec check required.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
