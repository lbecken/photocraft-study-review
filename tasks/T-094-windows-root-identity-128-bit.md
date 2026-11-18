---
id: T-094
title: "Identify the root on Windows by the 128-bit file id where the 64-bit index is not unique"
tier: high              # provisional: the confinement boundary (disk.rs)
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-073]
touches:                # provisional, fixed at specification
  - crates/kiln-workspace/src/disk.rs
  - docs/protocol/paths.md
budget_tokens: 1100000  # high (v1.5)
iteration: 2
found_by: reviewer      # reviews/T-073-r1.md finding 3 (run R-343); read, not run
origin_task: T-073
---

# T-094 The root's identity on file systems without a unique 64-bit index

This is a draft filed from the review of T-073 (run R-343; D-250, ruling c3). It is specified in full before any
dispatch.

## Goal
T-073 identifies the root on Windows by the volume serial number and the 64-bit file index (`same_file::Handle`).
ReFS documents that index as not unique, so two directories could compare equal there, which is the unsafe
direction for the root check. After this task the identity is the 128-bit file id where the system offers it, or
the limit stays stated and the task is closed with the evidence that it cannot be reached.

## Evidence in the record
- `reviews/T-073-r1.md`, finding 3 (read from the crate's source and the platform's documentation; no run on
  ReFS, SMB or FAT).
- `docs/protocol/paths.md`, the limit sentence T-073 writes (ruling c3).

## Open questions for the specification
- Can a hosted runner offer a ReFS volume (a Dev Drive) for a test, or is this evidence only by reading?
- Does `same-file` gain the wider id, or does Kiln call the system itself (a new unsafe-free dependency)?

## Log
### director
- 2026-11-18 filed as a draft (D-250).
