---
id: T-053
title: "Correct published texts that are stale or cut: the regex refusal message, the two shapes of files in commands.json, and four stale sentences"
tier: medium
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-021, T-038, T-039]
touches:                # provisional, fixed at specification
  - crates/kiln-commands/src/search.rs
  - crates/kiln-commands/tests/reasons.rs
  - docs/commands.json
  - docs/development.md
  - docs/protocol/cli-memory-seed.md
  - apps/kiln-cli/src/ (the `--help` text)
budget_tokens: 700000
iteration: 2
found_by: evaluator      # evaluations/iter-2.md Q2, Q6, NEW-4; reviews of T-038 (j5, D-137) and T-039 (D-096)
origin_task: T-038
---

# T-053 Correct published texts that are stale or cut: the regex refusal message, the two shapes of files in commands.json, and four stale sentences

This is a draft filed from the evaluation of the first stage of iteration 2 (`evaluations/iter-2.md`,
runs R-221 and R-222; D-141). Interface, Pinned and acceptance criteria are written by a planner, and the
spec is checked before dispatch (METHODOLOGY v1.2).

## Goal
Several published texts are stale, cut or uneven, each found by an evaluator or a reviewer and none of them a wrong behaviour. This draft collects them so they are not lost; the planner splits it if the parts do not share a tier (D-091: bundles are split).

## Evidence in the record
- `evaluations/iter-2.md`, Q2: for a long refused regex the 200-char message cap keeps the echo of the pattern and cuts the engine's reason.
- `evaluations/iter-2.md`, Q6: in `docs/commands.json`, `examples[].files` are `[path, text]` pairs and refusal examples' `files` are `{path, text}` objects (ruling j1, D-136, pinned the second form).
- `evaluations/iter-2.md`, NEW-4: `nav.workspace-symbols` publishes `not_found` and `not_a_file`; they are kinds that need a change on disk during the request (the reviewer of T-038 produced them 618 and 422 times by toggling a directory while requests ran, `reviews/T-038-r1.md`). Nothing published tells a client that a kind is of that sort.
- D-137 j5: the comment "13 rows" and the test name `describe_publishes_every_reason_in_all_order` in `crates/kiln-commands/tests/reasons.rs`; the dated timing table in `docs/development.md` (`cargo xtask schema` alone now takes 3.4 to 4.0 s).
- D-096: `kiln-cli --help` and `docs/protocol/cli-memory-seed.md` say "nothing is written to disk", which is too wide because of `--log`.

## Open questions for the specification
- Q6 changes a published shape: which form stays, and is that a protocol change (tier)?
- NEW-4: publish the witness sort (example or disk) of each declared kind, or say it in `docs/control-protocol.md` only?

## Note from T-056's review round 2 (director, D-161)
`docs/protocol/ui-editor.md`, the settle sentence, says "1,110 actions", the count of the first 22 acceptance
tests; with all 28 the reviewer counted 1,135 (the figures themselves are unchanged: largest 20 frames).

## Note from T-065's review round 2 (director, D-167)
`docs/protocol/ui.md`, section "The window": "the clock is not read" is wrong as worded (the toolkit reads the
clock and the window then overwrites the time; "the UI's time does not come from the clock" is what holds); and
the sentence on a lost display says a panic message came in "one of four runs" where the reviewer saw it in 8 of 20
("in some runs").

## Note from the evaluation at cut 1 (director, D-169)
`evaluations/iter-2-cut-1.md`: NEW-2 `docs/protocol/ui-editor.md` calls the rows' lines "consecutive" although an
empty line has no row; NEW-3 `docs/protocol/ui.md` names the shader cache under `HOME` but not how to turn it off
(`MESA_SHADER_CACHE_DISABLE=true`: 11 files without it, none with it); NEW-4 the sentence on smaller screens has no
equivalent below the smallest `--size` (`--size 160x120` is refused while a window on a 160x120 screen opens);
Q2 a target `value_contains: ""` does not match a node without a value (say so); Q3 the `scroll_bar` node of an
overflowing list appears only once the list has settled (say so).

## Note from the evaluation at cut 2 (director, D-215)
`evaluations/iter-2-cut-2.md`, "NEW-3" (this task is its number): `docs/protocol/ui.md`, "The shell", says when
`Message` gets a refusal's kind and when it is cleared more narrowly than the binary and the module documents do:
it also gets one from refused UI saves and refused panel expands, and successful saves and tab closes clear it.
The cut-1 findings named in this task were seen again (the shader-cache sentence, "the lines consecutive").

## Log
### director
- 2026-11-14 filed as a draft (D-141).
