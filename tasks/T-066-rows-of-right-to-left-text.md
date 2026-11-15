---
id: T-066
title: "Report the rows of text laid out right to left as the chars each row paints"
tier: high              # provisional: it changes the observation interface every UI item is evaluated through (ADR-0008)
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-056]
touches:                # provisional, fixed at specification
  - crates/kiln-ui/src/observe.rs
  - docs/protocol/ui.md
budget_tokens: 1100000  # high (v1.3)
iteration: 2
found_by: implementer   # run R-242 of T-056, under ruling z1 (D-159), after the reviewer's finding 1 of reviews/T-056-r1.md
origin_task: T-055
---

# T-066 Report the rows of text laid out right to left as the chars each row paints

This is a draft filed so that the limit T-056 states is not lost (D-160). Interface, Pinned and acceptance
criteria are written by a planner, and the spec is checked before dispatch.

## Goal
A snapshot reports, for every node that shows text, the chars each painted row holds, in reading order. T-056
made that true for ligatures, combining marks, joiners and shaped digits. It is not yet true for text the toolkit
lays out right to left: there the glyphs are in visual order and the toolkit adds glyphs at the end, so which chars
count as visible is decided from wrong positions, and when such text runs onto several rows a row's text can be
wrong. After this task the rule holds for such text too, or the document says precisely which texts it does not
hold for and why that cannot be changed.

## Evidence in the record
- `docs/protocol/ui.md`, the limit added by T-056 (run R-242), with its measurement.
- T-056 implementer log R-242: a `Title` of a document named `٣٣ ab ٣٣ ab … fin fi.txt` lost its last two chars at
  1280×800; at `--size 200x400` the `Documents` entry and `Title` each showed one reversed and one duplicated row;
  one such `Title` had 103 glyphs for 53 chars.
- `reviews/T-056-r1.md`, finding 1: the three causes in the toolkit's text shaping.

## Open questions for the specification
- Does the editor (monospace, one line per row, cut at the right edge) show the problem at all, or only nodes whose
  text wraps?
- Is right-to-left text in scope for the product (CHARTER), or is the honest answer a stated limit?

## Note from T-056's review round 2 (director, D-161)
The limit stated in `docs/protocol/ui.md` is slightly narrower than what the reviewer showed: chars that a
right-to-left row loses can turn up in another node's rows (`xt` in the `Documents` list, `ln` in a container).
The specification states the full extent or removes it.

## Log
### director
- 2026-11-15 filed as a draft (D-160).
