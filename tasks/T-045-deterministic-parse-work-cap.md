---
id: T-045
title: "Bound the work of every parse by a deterministic count, so that syntax.* and the nav.* commands that parse answer or refuse within that bound"
tier: medium            # ADR-0002 default (in-memory behaviour, no high criterion as the record stands); high if the spec check finds the cap needs a dependency change (see Log)
state: draft
owner_role: planner
scorecard_items: []     # no item is claimed or strengthened; see Open questions (syntax.parse, syntax.outline)
depends_on: [T-016, T-020, T-023]   # all merged: the parse contract, syntax.*, nav.*
touches:                # provisional, fixed at specification
  - crates/kiln-syntax/src/tree.rs          # the parse and update entry points that T-016 defined
  - crates/kiln-commands/src/syntax.rs      # syntax.* refusal on the cap
  - crates/kiln-commands/src/nav.rs         # nav.outline, nav.goto-symbol, nav.workspace-symbols
  - docs/protocol/syntax.md                 # section "Time"
  - docs/protocol/nav.md                    # section "Memory and time"
  - docs/commands.json                      # generated (`cargo xtask schema --write`)
budget_tokens: 700000   # medium (v1.2)
iteration: 2
found_by: planner       # spec check R-157 of T-020 (D-097 Q-3)
origin_task: T-020
---

# T-045 Bound the work of every parse by a deterministic count

This is a draft filed so that the promise of D-097 (Q-3) is not lost. It lies outside the first
stage of iteration 2 (ADR-0005): the director decides when it is specified and dispatched.

## Goal
Today no time bound exists for any `syntax.*` command or for the `nav.*` commands that parse
(`nav.outline`, `nav.goto-symbol`, `nav.workspace-symbols`). The size limit of 1,048,576 chars
bounds only the length of the text, and for some texts parse time grows with the square of the
length. After this task a caller would get, for every parse, either the answer or a refusal
after a fixed amount of parser work, counted so that the same text and the same calls always
give the same outcome (CHARTER §3 decision 4: no wall-clock dependence in L0–L4). Interface,
Pinned and acceptance criteria are written after the spec check below.

## Evidence in the record
- `docs/protocol/syntax.md`, "Time": "No time bound exists for any `syntax.*` command"; release
  build, `syntax.tree` after `doc.open`, `#!` repeated: 1.10 s at 16,384 chars, 4.49 s at
  32,768, 17.00 s at 65,536 (one run each, on a 4-core machine running other builds); "a call
  after an edit that leaves the tree with errors parses the text twice".
- `tasks/T-020-*.md`, implementer log R-167: the commands and outputs behind those figures
  (`1.10 s 14336 KB`, `4.49 s 18508 KB`, `17.00 s 26820 KB`).
- `tasks/T-020-*.md`, Pinned (e) and planner log R-157: in a debug spike outside the
  repository, `#!` repeated took 0.49 s for 8,192 chars, 1.88 s for 16,384, 7.67 s for 32,768
  and 30.3 s for 65,536; 1,048,576 `"` took 36.9 s; "at the limit it would take hours
  (extrapolated, not run)". Spec evidence only, not text for any document (ruling Q-3).
- `docs/protocol/nav.md`, "Memory and time": "No time bound exists for any `nav.*` command that
  parses"; `nav.workspace-symbols` over one file of `#!` repeated (65,536 chars) took 23 s
  (release, two runs; T-023 implementer log R-194: `23.09 s`, `23.13 s`).
- `reviews/T-023-r1.md`, finding 7: memory of a parsed file at the limit, debug build:
  `a;`×524,288 `352476 KB`, `1;`×524,288 `401672 KB`; ruled m5 (D-124), "other texts can need more".
- D-097, ruling Q-3 in T-020: "A deterministic work cap is a follow-up task, filed by the
  director; this task does not attempt it." Repeated as out of scope in T-020 (Out of scope) and
  T-023 (Out of scope, line 432).
- T-016 (Interface, notes): `ParseError::NoTree` cannot occur "With no timeout or cancellation
  flag set"; "Set no timeout or cancellation flag." `docs/protocol/syntax.md`, "Errors", and
  `nav.md`, "Errors", state that no tree is "not possible with the pinned versions".

## Options the record names
- A deterministic work cap (D-097 Q-3): the only means the record names; it names no mechanism.
  Consequence: a parse can stop without a tree, so the T-016 statement above and the "not
  possible" sentences of `syntax.md` and `nav.md` change.
- A work limit counted in parser steps through the parser's progress callback: named in the
  filing request of R-212, not found in the record. Consequence, as the request states it: the
  tree API that T-016 defined changes.
- A time budget: ruled out by CHARTER §3 decision 4 (as for search, T-021 Q-1 (e)).

## What the spec check must establish before this is specified
- Whether the pinned tree-sitter (=0.27.1) offers a way to stop a parse after a counted amount
  of work without a version or feature change (if not, the tier is high: dependency change).
- Measurements on a release build through `kiln-cli`: the count a cap would allow against the
  pinned compiling corpus, this workspace's largest Rust file, AC-10's deep-nesting text of T-020
  (1,000,012 chars, must stay answered) and `#!` repeated at several lengths.
- Whether `SyntaxTree::update`'s second, full parse counts against the same cap.
- Merged tests the change would touch: `crates/kiln-syntax/tests/acceptance_t016.rs` and
  `tests/tree.rs` (corpus and workspace sources parse without errors), `apps/kiln-cli/tests/
  acceptance_t020_cli.rs` (`deep_nesting_is_answered`, `corpus_parses_through_protocol`,
  `refusals_leave_the_cache_unchanged`), `apps/kiln-cli/tests/acceptance_t023_cli.rs`.
- Document sentences it would change: `syntax.md` "Time" and "Errors"; `nav.md` "Memory and time",
  "Errors", and the `skipped` reasons of `nav.workspace-symbols`.
- Drafted tasks sharing its files: T-038 (`syntax.rs`, `nav.rs`, `reason.rs`, `syntax.md`,
  `nav.md`); T-048 (`syntax.rs`, `syntax.md`). Not in flight with either.

## Open questions for the director
- Is the kiln-syntax parse API that T-016 defined a CHARTER §3 contract (then this is its own
  high-tier task, METHODOLOGY §5.3) or a crate API (medium)?
- A refusal of `syntax.*` and nav's `skipped` list: a new refusal kind, or an existing one?
- Should the cache rule of T-020 (a refusal keeps the cache) and `mode` hold unchanged?
- Does a capped file in `nav.workspace-symbols` go to `skipped`, as `too_large` does?

## Questions and rulings

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-14T07:20Z · R-212 · (new) · did: filed this draft from D-097 (Q-3), T-020 and T-023
  as the director asked; no solution designed. Tier medium: ADR-0002's default for a behaviour
  change; no persistence path, process, gate path or confinement boundary; high if the spec check
  finds a dependency change or rules the T-016 API a §3 contract. · gate attempts: 0 · not run:
  no cargo, no measurement · missing context: the progress-callback means is not in the record
  (only "a deterministic work cap") · next: director, when stage 1 has ended.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
