---
id: T-048
title: "Cap the number of spans a syntax.highlight reply holds and report when it was cut"
tier: medium            # ADR-0002 default: in-memory behaviour of one command's params and result; adding a param is not a contract change
state: draft
owner_role: planner
scorecard_items: []     # no item is claimed; see Open questions (syntax.highlight)
depends_on: [T-020]     # merged: syntax.highlight
touches:                # provisional, fixed at specification
  - crates/kiln-commands/src/syntax.rs      # syntax.highlight params and result
  - docs/protocol/syntax.md                 # sections "syntax.highlight" and "Memory"
  - docs/commands.json                      # generated (`cargo xtask schema --write`)
budget_tokens: 700000   # medium (v1.2)
iteration: 2
found_by: reviewer      # reviews/T-020-r1.md, finding 1; moved out of T-020 by D-109 and ruling r1 (D-110)
origin_task: T-020
---

# T-048 Cap the number of spans a syntax.highlight reply holds

This is a draft filed so that the follow-up named in D-109 and ruling r1 (D-110) is not lost. It
lies outside the first stage of iteration 2 (ADR-0005): the director decides when it is
specified and dispatched.

## Goal
Today a whole-document `syntax.highlight` returns one span for each run of chars of one kind,
however many there are; `start` and `end` let a caller ask for a range, but nothing caps the
number of spans in a reply. After this task a reply holds at most a stated number of spans and
says when it was cut, as `search.find` and `search.replace` do with `limit` and `truncated`.
Interface, Pinned and acceptance criteria are written after the spec check below.

## Evidence in the record
- `docs/protocol/syntax.md`, "Memory": release build at the limit (1,048,576 chars; `doc.open`,
  `syntax.tree`, a `doc.replace` of one char, `syntax.tree`, `syntax.highlight`): 689 MiB for
  `a;` repeated and 2,073 MiB for `1;` repeated (maximum resident set size of the whole
  `kiln-cli run` process, two runs; "not a bound"); "Without the `syntax.highlight` call, the
  same script on `1;` repeated peaked at 566 MiB; with it, the `syntax.highlight` reply held
  1,048,575 spans."
- `tasks/T-020-*.md`, implementer log R-184: the outputs behind them (`1;` `6.94 s 2123024 KB`,
  1,048,575 spans; without the highlight call `2.95 s 579576 KB`; `a;` `3.95 s 705700 KB`, 1 span).
- `reviews/T-020-r1.md`, finding 1 (defect; debug build, D-103's script): `1;`×524,288
  `22.77 s 2129076 KB`, highlight 1,048,575 spans; the reviewer named "highlight results with
  many spans" as a case that takes more.
- D-109: "nothing is moved to a follow-up except a cap on the size of a highlight reply, which was
  never in the task". Ruling r1 in T-020: "A cap on the number of spans a `syntax.highlight`
  reply holds is not part of this task; it goes to the limits follow-up (D-101)."
- `docs/protocol/search.md`: `search.find` takes `limit` (1 to 100,000, default 10,000) and
  reports `total` and `truncated`; `search.replace` caps `ranges` the same way (T-021 ruling Q-2,
  D-101, taken after a replace-all reply pushed the process past 512 MiB).

## Options the record names
- A `limit` param and a `truncated` flag, as the search commands have (the precedent of T-021
  Q-2 (a)). Consequence: a caller that wants every span pages with `start`/`end`; the reply no
  longer covers the requested range exactly when cut, so the partition rule of `syntax.md` gains
  a case.
- Ranges only (`start`/`end`, as today): exists; it caps nothing, which is the finding.

## What the spec check must establish before this is specified
- Measurements on a release build through `kiln-cli`, D-103's script on `1;` and `a;` repeated
  at the limit: peak memory with a whole-document reply against a capped one, and how much of the
  2,073 MiB the reply itself accounts for (566 MiB without it is the record's only split).
- What a cut reply reports so that a caller can continue (for example where the last span ends),
  and how `truncated` interacts with the clipping of spans at range ends.
- Merged tests the change would touch: `apps/kiln-cli/tests/acceptance_t020_cli.rs`
  (`highlight_partitions_text`, `highlight_agrees_with_tokenizer`, `highlight_marks_fn_names`,
  and `deep_nesting_is_answered`, whose AC-10 expects spans covering `0..1000012`); the worked
  example of `syntax.md`; the `syntax.highlight` example the audit runs.
- Document sentences it would change: `syntax.md` "`syntax.highlight`" (result and the "cover
  the requested range exactly" sentence) and "Memory".
- Drafted tasks sharing its files: T-038 (`syntax.rs`, `syntax.md`); T-045 (`syntax.rs`,
  `syntax.md`). Not in flight with either.

## Open questions for the director
- The default and the maximum of `limit`, and whether a default may cut a whole-document reply
  that today is complete (a behaviour change for existing callers).
- Do the scorecard item `syntax.highlight`'s tests need re-tying after this change?
- `nav.outline` also replies with every item and "there is no `limit`" (`nav.md`): in scope here,
  or a separate draft?

## Questions and rulings

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-14T07:20Z · R-212 · (new) · did: filed this draft from D-109, ruling r1 (D-110) and
  `reviews/T-020-r1.md` finding 1 as the director asked; no solution designed. Tier medium:
  ADR-0002's default; one command's params and result in memory, no high criterion (T-021's
  `limit` on `search.replace` was medium on the same ground). · gate attempts: 0 · not run: no
  cargo, no measurement · missing context: none · next: director, when stage 1 has ended.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
