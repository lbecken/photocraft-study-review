---
id: T-078
title: "Build unsigned release binaries on the three platforms as workflow artefacts, and show that kiln script prints the same bytes on all three"
tier: high              # ADR-0002 "a gate path": .github/
state: draft
owner_role: planner
scorecard_items: []     # none claimed: the items exist and are verified on Linux; per-platform confirmation is the evaluator's (ADR-0009 Decision 3)
depends_on: [T-076]
touches:                # provisional, fixed at specification; each path one role (D-153)
  - .github/workflows/platforms.yml   # GATE PATH: implementer (release-build and compare jobs)
  - xtask/tests/acceptance_t078.rs    # GATE PATH: test author
  - docs/development.md               # implementer: where the artefacts are and what a user meets (Gatekeeper, SmartScreen)
  - docs/protocol/ui.md               # implementer: the cross-platform determinism sentence, with its measurement
budget_tokens: 1100000
iteration: 2
found_by: planner       # ADR-0009 (R-255)
origin_task: null
---

# T-078 Build unsigned release binaries on the three platforms as workflow artefacts, and show that kiln script prints the same bytes on all three

Draft by the planner of ADR-0009 (run R-255); a stub in the style of T-066 and T-068. Interface, Pinned
and acceptance criteria are written at specification, after T-070's first runs have replaced the
estimates of ADR-0009 with measurements.

## Goal
D-173: the platform work ends at built and tested binaries; no signing, no installer. After this task each
platform job of the nightly run (and a manual dispatch) builds `kiln` and `kiln-cli` in release mode and
keeps them as artefacts with a stated retention; and a fixed set of scripts run through `kiln script` and
`kiln-cli run` on the three platforms gives byte-identical output, compared in a final job (built-in fonts
should make layout platform-independent; whether float results agree on arm64 is not known, ADR-0009 (b)).
The documents say what a user meets with an unsigned binary (KNOWN: macOS quarantine and Gatekeeper,
Windows SmartScreen) and nothing works around it.

## Estimate
**About 700,000** (workflow and one gate test; the comparison may find real differences, which become
their own tasks).

## Log
### director
<!-- end director log -->
### planner
- 2026-11-15 · R-255 · draft · did: filed from ADR-0009 · next: director
<!-- end planner log -->
