---
id: T-071
title: "Make no-skip and the scorecard step follow the platform-test rule of ADR-0009, and make the gate's own tests run on all three platforms"
tier: high              # ADR-0002 "a gate path": xtask/; the approval row must contain `gate paths`
state: draft
owner_role: planner
scorecard_items: []     # none claimed: the items exist and are verified on Linux; per-platform confirmation is the evaluator's (ADR-0009 Decision 3)
depends_on: [T-070]
touches:                # provisional, fixed at specification; each path one role (D-153)
  - xtask/src/noskip.rs                 # GATE PATH: implementer
  - xtask/src/scorecard.rs              # GATE PATH: implementer
  - xtask/platform-tests.toml           # GATE PATH: implementer (new registry, empty or with the entries this task needs)
  - xtask/fixtures/no-skip/             # GATE PATH: implementer (self-test cases for the new findings)
  - xtask/src/hygiene.rs                # GATE PATH: implementer (its test module uses std::os::unix, probe a4)
  - xtask/src/publish.rs                # GATE PATH: implementer (test module, probe a4)
  - xtask/tests/acceptance_t005_r3.rs   # GATE PATH: owner of the file at specification (one role per path)
  - xtask/tests/acceptance_t005_r4.rs   # GATE PATH: as above
  - xtask/tests/acceptance_t013_hygiene.rs  # GATE PATH: as above
  - xtask/tests/guard_t005.rs           # GATE PATH: as above
  - xtask/tests/acceptance_t071.rs      # GATE PATH: test author
  - docs/development.md                 # implementer: the no-skip and scorecard rows
budget_tokens: 1100000
iteration: 2
found_by: planner       # ADR-0009 (R-255)
origin_task: null
---

# T-071 Make no-skip and the scorecard step follow the platform-test rule of ADR-0009, and make the gate's own tests run on all three platforms

Draft by the planner of ADR-0009 (run R-255); a stub in the style of T-066 and T-068. Interface, Pinned
and acceptance criteria are written at specification, after T-070's first runs have replaced the
estimates of ADR-0009 with measurements.

## Goal
D-166 ruling b8 accepted Unix-only test files "since the gate and CI run on Linux". ADR-0009 Decision 4
replaces it by a rule. After this task, `no-skip` fails: a test or test module behind a platform `cfg`
that is not listed in `xtask/platform-tests.toml` with the document sentence it covers and its
counterpart on the other platforms (or "no counterpart" and a reason); a listed entry whose counterpart
does not exist; and a helper used by tests that does nothing (or returns a placeholder) on one platform
where it acts on another (today `acceptance_t002.rs:176-187`, R-255 probe (c)). A test that cannot run on
a platform fails there with a message (as `acceptance_t015_tools.rs:130`), never passes. The scorecard
page is computed on Linux; on macOS and Windows the scorecard step fails when a listed test built there
fails or is missing without a registry entry, and does not compare the page. The gate's own tests
(`xtask/`) compile and pass on all three (6 files fail to compile on Windows today, probe a4).

## Open questions for the specification
- How does `no-skip` tell a "helper that does nothing" from an ordinary platform helper? It may have to
  stay partly a review rule; the spec says which part is mechanical.
- Registry fields and whether entries expire (skips do; platform facts do not).

## Estimate
High; xtask changes of the first stage cost 700,000–930,000 (stage-1 high p75 866,379): **about 1,000,000**
(three platforms to verify through CI). Spec check by the drafting planner about 35,000.

## Log
### director
<!-- end director log -->
### planner
- 2026-11-15 · R-255 · draft · did: filed from ADR-0009 · next: director
<!-- end planner log -->
