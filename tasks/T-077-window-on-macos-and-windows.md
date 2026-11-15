---
id: T-077
title: "Open the window on macOS and Windows, build the windowing crate with each platform's features, and run the window procedure in CI where the runner allows"
tier: high              # ADR-0002: dependency tables change (per-target features of eframe)
state: draft
owner_role: planner
scorecard_items: []     # none claimed: the items exist and are verified on Linux; per-platform confirmation is the evaluator's (ADR-0009 Decision 3)
depends_on: [T-070, T-068, T-076]
touches:                # provisional, fixed at specification; each path one role (D-153)
  - Cargo.toml                       # implementer: eframe features per target (x11 on Linux only)
  - crates/kiln-ui/Cargo.toml         # implementer
  - Cargo.lock                        # generated
  - crates/kiln-ui/src/window.rs      # implementer (after T-068, which edits it)
  - .github/workflows/platforms.yml   # GATE PATH: implementer: the window procedure on macOS/Windows if T-070's probe allows
  - docs/protocol/ui.md               # implementer: "The window" per platform; the `ctrl`/`command` sentence
  - docs/protocol/ui-editor.md        # implementer: keys on macOS
budget_tokens: 1100000
iteration: 2
found_by: planner       # ADR-0009 (R-255)
origin_task: null
---

# T-077 Open the window on macOS and Windows, build the windowing crate with each platform's features, and run the window procedure in CI where the runner allows

Draft by the planner of ADR-0009 (run R-255); a stub in the style of T-066 and T-068. Interface, Pinned
and acceptance criteria are written at specification, after T-070's first runs have replaced the
estimates of ADR-0009 with measurements.

## Goal
The toolkit compiles for macOS and Windows with today's features (probe a2, a4), but no window has been
opened there. After this task `kiln` opens a window on macOS and Windows, the no-window exit (status 2,
one line) holds there, and where T-070's probe showed a usable session the window procedure of T-065
runs in CI on that platform; elsewhere the documents say "built, not observed" (ADR-0009 Decision 2, 6,
Q-5). If OpenGL is not available on a runner, the renderer question goes to the owner (Q-5 B).

## Estimate
Comparable to T-065 (887,544, D-168): **about 1,100,000**; more if a renderer change is decided.

## Note from T-070's review (director, D-193)
`xtask/tests/acceptance_t070.rs` pins the `window-probe` job this task changes: the file goes into this task's
`touches` (test author) and each changed assertion is named.

## Log
### director
<!-- end director log -->
### planner
- 2026-11-15 · R-255 · draft · did: filed from ADR-0009 · next: director
<!-- end planner log -->
