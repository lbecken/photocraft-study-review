---
id: T-047
title: "Bound what .gitignore files can cost a project walk, so that project.tree and nav.workspace-symbols stay within a stated limit on hostile projects"
tier: high              # ADR-0002 "a security or confinement boundary": a bound on reading needs a size query on Vfs (T-023 N-5); medium if the cap only counts what Vfs::read returns (see Log)
state: draft
owner_role: planner
scorecard_items: []     # no item is claimed or strengthened; see Open questions (ws.open-project)
depends_on: [T-019, T-023]   # both merged: the project walk, nav.workspace-symbols
touches:                # provisional, fixed at specification
  - crates/kiln-workspace/src/project.rs    # push_gitignore, parse_gitignore
  - crates/kiln-workspace/tests/project.rs  # implementer's own tests
  - docs/protocol/project.md                # "Guarantees and limits"
  - docs/protocol/nav.md                    # if the walk's reply or limits change
  - docs/commands.json                      # generated, if a description changes
budget_tokens: 1100000  # high (v1.2)
iteration: 2
found_by: reviewer      # reviews/T-019-r1.md, finding 3
origin_task: T-019
---

# T-047 Bound what .gitignore files can cost a project walk

This is a draft filed so that the promise of D-105 (ruling y7) is not lost. It lies outside the
first stage of iteration 2 (ADR-0005): the director decides when it is specified and dispatched.

## Goal
Today `project.tree` and `nav.workspace-symbols` read and compile every `.gitignore` on the way,
with no limit on its size or its number of patterns, and every matcher in force stays on a stack
along the depth. A hostile project can make one call take many seconds and hundreds of MB. After
this task what `.gitignore` files can cost one walk is bounded by a stated, deterministic rule,
and the document says what happens to a file beyond it. Interface, Pinned and acceptance
criteria are written after the spec check below.

## Evidence in the record
- `docs/protocol/project.md`, "Guarantees and limits": "No bound exists on the time or memory
  that `.gitignore` files can cost." Also: "A `.gitignore` whose patterns cannot be compiled
  contributes no patterns" (ruling y3), and "`limit` bounds the number of entries listed, not
  the work".
- `reviews/T-019-r1.md`, finding 3 (spec gap; debug build of `kiln-cli`): one 157 KB file of
  20,000 patterns, 0.60 s and 120 MB; 10 nested such files (1.7 MB on disk), 8.43 s and 393 MB;
  above about 50,000 patterns y3 applies "(1.28 s spent first)". Linear extrapolation, marked
  as inference: 128 nested files "about 100 s and several GB"; 1,000 sibling directories with
  one such file each "about 10 minutes". Boundary probes: at 100,000 lines, 2.64 s and 420 MB.
- `tasks/T-019-*.md`, implementer log R-168 (ruling y3 probe): `N` lines `*a<i>*b*[c-x]?d*e`, N = 1,000
  / 5,000 / 8,000 → `ok` (0.09 / 0.42 / 0.61 s); N = 12,000 / 20,000 (389 KB) → the build failed
  ("error building NFA") before y3 made such a file contribute no patterns.
- Ruling y7 in T-019 (D-105): "A bound is part of the follow-up task on deterministic limits that
  the director files (D-101)." D-107 accepted the unbounded cost with the merge.
- `docs/protocol/nav.md`: `nav.workspace-symbols` searches what the walk of `project.tree` with
  `limit` 1,000,000 lists; "An error while listing a directory or reading a `.gitignore` fails
  the whole command, as for `project.tree`."
- T-023 note N-5 (ruling D-108): `Vfs` has no size query; adding one is "a confinement-boundary
  change"; a file is read whole before its length is checked.

## Options the record names
- A size or pattern cap per `.gitignore`, a file beyond it treated as y3, contributing no
  patterns (`reviews/T-019-r1.md`, finding 3). Consequence: a project whose large `.gitignore` is
  real lists files that git ignores; a size cap checked before reading needs a size query on
  `Vfs` (N-5), one checked after reading does not bound the read.
- A documented limit sentence (same finding): taken by y7 as the present state; it bounds nothing.

## What the spec check must establish before this is specified
- Measurements on a release build through `kiln-cli` for the reviewer's three shapes (one large
  file, nested large files, many sibling files): time and peak memory against size and pattern
  count, so that a cap rests on a figure; and whether one cap per file bounds the nested and
  sibling cases or a cap per walk is also needed.
- The largest `.gitignore` in real Rust projects the spec can cite, so that a cap does not
  ignore ordinary files.
- Merged tests the change would touch: `crates/kiln-workspace/tests/project.rs`
  (`a_gitignore_that_cannot_be_compiled_contributes_no_patterns`, `an_ignored_gitignore_is_still_read`),
  `apps/kiln-cli/tests/acceptance_t019_cli.rs` and `guard_t019_cli.rs`, and
  `apps/kiln-cli/tests/acceptance_t023_cli.rs` for the workspace search.
- Document sentences it would change: `project.md` walk rule 3 and "Guarantees and limits";
  `nav.md`, "Finding symbols in the project".
- Drafted and in-flight tasks sharing its files: T-037 (in progress; adds a required method to
  `Vfs`, `vfs.rs`, `disk.rs`, `memory.rs`) and T-044 (`disk.rs`) if a size query is chosen;
  T-038 (`crates/kiln-commands/src/project.rs`, `nav.rs`); T-022 (uses the walk).

## Open questions for the director
- Must the bound cover the memory of reading a `.gitignore` (a `Vfs` size query, high tier), or
  only the patterns compiled from what is read (medium)?
- Is a file beyond the cap silent (as y3) or reported in the reply?
- Is the cost of many `.gitignore` files in one walk part of this task?

## Questions and rulings

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-14T07:20Z · R-212 · (new) · did: filed this draft from ruling y7 (D-105) and
  `reviews/T-019-r1.md` finding 3 as the director asked; no solution designed. Tier high,
  provisionally, by ADR-0002's confinement-boundary criterion: a bound on what reading costs
  needs a size query on `Vfs`, which T-023 N-5 calls a confinement-boundary change; a pattern cap
  on what the unchanged `Vfs::read` returns would be medium (as T-023's tier note says). The
  director confirms at `specified`. · gate attempts: 0 · not run: no cargo, no measurement ·
  missing context: none · next: director, when stage 1 has ended.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
