---
id: T-046
title: "Bound the work of a search deterministically, so that search.find and search.replace answer or refuse within that bound"
tier: high              # ADR-0002 "adding or changing a dependency": two of the means the record names need one (see Log); medium if only dependency-free means are chosen
state: draft
owner_role: planner
scorecard_items: []     # no item is claimed or strengthened; see Open questions (search.regex)
depends_on: [T-017, T-021]   # both merged: the regex contract, search.*
touches:                # provisional, fixed at specification
  - crates/kiln-search/src/regex.rs         # the regex contract of T-017
  - crates/kiln-search/src/query.rs         # the pattern checks of T-021
  - crates/kiln-commands/src/search.rs      # search.find, search.replace
  - docs/protocol/search.md                 # section "Limits"
  - docs/commands.json                      # generated (`cargo xtask schema --write`)
budget_tokens: 1100000  # high (v1.2)
iteration: 2
found_by: planner       # spec check R-158 of T-021 (D-101 Q-1)
origin_task: T-021
---

# T-046 Bound the work of a search deterministically

This is a draft filed so that the promise of D-101 (Q-1) is not lost. It lies outside the first
stage of iteration 2 (ADR-0005): the director decides when it is specified and dispatched.

## Goal
Today `search.find` and `search.replace` have no time bound: the caps of 1,000 pattern chars and
100 capture groups bound the pattern text and the capture slots, not the time a search takes,
and the slowest cases are short counted repeats. After this task a pattern that would take
unbounded work is refused, or the search stops, by a rule that gives the same outcome for the
same pattern and text every time (CHARTER §3 decision 4). Interface, Pinned and acceptance
criteria are written after the spec check below.

## Evidence in the record
- `docs/protocol/search.md`, "Limits": "These limits do not bound the time a search takes";
  release build, 1,000,000 chars (`ab` repeated 500,000 times): `[ab]{5000}[^ab]` (15 chars, no
  match) 41 to 42 s, `[ab]{3000}` (10 chars, 333 matches) 12 s, `([ab]*)` repeated 100 times
  (700 chars, one match) 5.6 to 5.8 s; "Kiln sets no time limit on a search" (elapsed time minus a
  `doc.open`-only script, two runs each, on a 4-core machine running other builds).
- `tasks/T-021-*.md`, implementer log R-173: the commands and outputs behind those figures
  (`41.65 s / 40.91 s`, `12.49 / 12.28 s`, `5.83 / 5.63 s`).
- `tasks/T-021-*.md`, spec check R-158, table T1 (debug build of a throwaway stub, spec evidence
  only): `[ab]{5000}[^ab]` 544 s, `[ab]{3000}` 172 s ("engine alone 170 s"), `([ab]*)`×100
  38.2 s ("engine alone 0.047 s"), `()`×100 on `x`×1,000,000 22.6 s ("engine alone 0.67 s");
  everything at the cap without counted repetition or many groups stayed under 0.25 s; the
  engine's compiled size "does not separate fast from slow".
- D-101 and ruling Q-1 in T-021: "No time bound is promised. ... A real bound (refusing large
  counted repeats, a whole-match-only search) is a follow-up task the director files".
- T-021 Q-1 option (a), consequence: "T-022 inherits it once per file".

## Options the record names (T-021, spec check R-158, Q-1)
- (b) Refuse counted repetitions above a bound, by a hand-written scanner in `kiln-search`
  ("fragile: nested `{n}`, classes, escapes"), medium; or by `regex-syntax` as a direct
  dependency ("high tier, approval row"). It would not cover `([ab]*)`×100 or `()`×100.
- (f) A whole-match-only primitive for `search.find` and reference-free templates: new public
  API whose empty-match iteration must equal `find_all`'s group 0; "it does not help the
  counted-repeat cases".
- (d) A step budget: "the `regex` crate has none; another engine is a dependency change and its
  own task".
- (c) A lower char cap: "pointless for time: the slowest probes are 10–15 chars".
- (e) A time budget: ruled out by CHARTER §3 decision 4 (T-021 ruling c3).
- The record's recommendation, if the owner wants a bound: "a separate task for (b)+(f)".

## What the spec check must establish before this is specified
- Measurements on a release build through `kiln-cli` on the texts of T1: the largest counted
  repeat that stays fast, and how nested and combined repeats (`(?:[ab]{100}){40}`) behave, so
  that any refusal bound rests on a figure.
- Whether `regex =1.13.1`, as pinned, exposes what a scanner or a refusal needs without
  `regex-syntax` as a direct dependency (it is already in the lock file as a transitive one).
- Merged tests the change would touch: `apps/kiln-cli/tests/acceptance_t021_cli.rs`
  (`regex_find_matches_naive_backtracker`, `invalid_patterns_are_reported`,
  `pattern_length_is_capped`, `search_memory_is_bounded` with `()`×100),
  `crates/kiln-search/tests/acceptance_t021_search.rs` (`for_each_match_equals_find_all`,
  `query_errors_in_order`), and T-017's tests in `crates/kiln-search/tests/`.
- Document sentences it would change: `search.md` "Limits" and "Refusals" (item 3), and the
  `invalid_pattern` `when` text that T-038 rewrites.
- Drafted tasks sharing its files: T-038 (`search.rs`, `reason.rs`); T-022 (find in files,
  `depends_on` T-021, reuses `Query`). Not in flight with T-038.

## Open questions for the director
- Which of (b), (f), (d) the owner wants; the tier follows from it (a dependency makes it high).
- Should (b) and (f) be one task, as the record recommends, or two (METHODOLOGY §5: a high-tier
  spec that bundles independent items is split)?
- A refused pattern: `invalid_pattern` with a new message, or a new kind?

## Questions and rulings

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-14T07:20Z · R-212 · (new) · did: filed this draft from D-101 (Q-1) and T-021's spec
  check as the director asked; no solution designed. Tier high by ADR-0002's dependency
  criterion, provisionally: `regex-syntax` as a direct dependency and another engine for a step
  budget are the dependency changes the record names; a hand-written scanner and a whole-match
  search alone would be medium. The director confirms at `specified`. · gate attempts: 0 ·
  not run: no cargo, no measurement · missing context: none · next: director, when stage 1 has
  ended.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
