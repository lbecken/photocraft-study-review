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

## Log
### director
- 2026-11-14 filed as a draft (D-141).
