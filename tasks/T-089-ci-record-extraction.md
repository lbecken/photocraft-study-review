---
id: T-089
title: "Turn a directory of downloaded CI gate artefacts into the CI record file under tasks/ci-runs/ with one xtask command"
tier: high              # ADR-0002 "a gate path": xtask/; fixed at specified (v1.5 §4); the approval row must contain `gate paths`
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-072]
touches:                # provisional, fixed at specification; one role per path (D-153)
  - xtask/src/cirecord.rs                 # GATE PATH: implementer (new)
  - xtask/src/main.rs                      # GATE PATH: implementer: the subcommand and its usage line
  - xtask/tests/acceptance_t089.rs        # GATE PATH: test author
  - xtask/tests/fixtures/t089/            # GATE PATH: test author: artefact files as T-072's workflow uploads them
  - xtask/platform-tests.toml             # GATE PATH: test author (its entries only, if any)
  - docs/development.md                   # implementer: "## CI", the CI-record paragraph
budget_tokens: 1100000  # high (v1.5)
iteration: 2
found_by: task:T-043  # v1.5 change 1, D-219 (1)
origin_task: null
---

# T-089 Turn a directory of downloaded CI gate artefacts into the CI record file under tasks/ci-runs/ with one xtask command

This is a draft filed by the re-specification of T-043 (run R-310, at main `1289bf1`), which split
the additions recorded for T-043 since D-098 into separate tasks (T-043 Q-6). Interface, Pinned
and acceptance criteria are written by a planner before dispatch, under methodology v1.5.

## Goal
v1.5 change 1: agents read CI evidence from a record file, not from job logs. `xtask` cannot reach GitHub; the director (or a helper with the Actions tools) downloads a run's artefacts into a directory, and `cargo xtask ci-record <dir> --task T-NNN --round <n> --run <id> --sha <head>` writes `tasks/ci-runs/T-NNN-round-<n>.md`: per job and platform, the gate's result and failed steps from `gate-report.json`, every failing test with its message from the test output, the counts, the disk line, and the artefacts it did not find. Deterministic: the same directory gives the same file.

## Evidence in the record
- retro-2c2: 952,269 tokens (15.2 %) spent reading CI; a 200-line prefix read as 366/11 where the artefact held 977/22 (D-199).
- `tasks/ci-runs/T-054-round-1.md` lines 79–157: names of failing tests not readable from logs; the uploaded `test-output.txt` holds them.
- D-219 (1): the workflow part (every test binary run, failing tests printed with messages) is T-072's, the extraction tool T-043's (moved here by T-043 Q-6).

## Open questions for the specification
- The artefact layout is fixed by T-072: specify after T-072 merges (its upload names and file formats).
- The record's format: the existing hand-written `tasks/ci-runs/*.md` as the model, or a new fixed layout (and JSON beside it for T-090)?
- Platform: the director's tools run on Linux only (ADR-0009 (c), D-199). New test files are
  Linux-only with entries in `xtask/platform-tests.toml`, as T-043 Q-3 decides for its own, or
  portable after a probe on `ci/T-089`.

## Log
### director
<!-- end director log -->
### planner
- 2026-11-17 · R-310 · (new) · did: filed as a draft from T-043's split (T-043 Q-6). · gate attempts: 0 · missing context: none · next: director.
<!-- end planner log -->
