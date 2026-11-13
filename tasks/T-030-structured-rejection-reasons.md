---
id: T-030
title: "Measure the error-size cap without the echoed id, pin the exact 4096-byte boundary with a test, and put the protocol document's misplaced and stale sentences right"
tier: medium            # ADR-0002 (R-060 re-tier): a behaviour change in kiln-control's cap decision; no persistence, process, gate path, security boundary, dependency or §3 contract (the reason contract moved to T-015)
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-032]     # D-041: T-032 rewrites kiln-control's kind derivation and the error-kinds sentences first
touches:
  - crates/kiln-control/src/lib.rs            # cap decision only
  - crates/kiln-control/tests/caps.rs         # implementer's own integration tests
  - docs/control-protocol.md                  # "Response" caps text; the integer-id sentence; stale sentences (see Interface)
budget_tokens: 500000   # medium (v1.1, provisional)
iteration: 2
found_by: reviewer
origin_task: T-004
---

# T-030 Error cap without the echoed id; exact boundary; document fixes

## Goal
D-032 approved T-004 with three follow-ups (`reviews/T-004-r3.md` R3-SG1, R3-N1, R3-N2, R3-N3):
1. **A long `id` must not count toward the truncation decision.** Today the 4096-byte check
   measures the whole response line, id included. A 5000-character id therefore replaces even
   small `details` by `{"truncated":true}`, contrary to the document ("A valid `id` is always
   echoed in full; it is not counted against the caps").
2. **A test at the exact 4096-byte boundary.** No test sits on it; mutation C04 survived.
3. **One misplaced sentence.** "Integer ids are any JSON integer …" sits in the
   character-offset paragraph and belongs to the id rules of "Request".

Restructured by the director (R-060): part (d) of the former T-030 (structured reasons, the
generated kind list, the ADR) moved to T-032 (D-041), which lands first. That leaves this task with
behaviour in `kiln-control`'s cap only, so it is medium by ADR-0002. Arm: **even** (independent
test author). No gate path.

Reading list: `docs/decisions.md` D-031 (9)–(10), D-032; `reviews/T-004-r3.md`;
`crates/kiln-control/src/lib.rs` (`cap_error`, `command_error_json`, the response writer);
`docs/control-protocol.md` "Request", "Response".

## Interface
- **Cap rule (exact).** For a host response, decide whether to replace `details` by
  `{"truncated":true}` from the byte length of the canonical response line in which the echoed
  id is replaced by `null` (`"id":null`). The id itself is still echoed in full. For
  `command_error_json` (no id; used by `kiln-cli exec`), measure the canonical error object, as
  today. A measured length of exactly 4096 bytes keeps `details`; 4097 truncates. The
  200-character string cap is unchanged and applies first, as today.
- **Every other response byte stays the same.** That covers every request without a long id,
  and every `kiln-cli exec` error object.
- **`docs/control-protocol.md`:**
  - the integer-id sentence moves to the id rules of "Request";
  - the "Response" caps text says the id is replaced by `null` for the measurement;
  - two stale sentences are corrected, if their tasks are on `main` when this task starts (the
    director's D-2 left them here). First, "a command that returns an error is not recorded"
    gains T-007's rule (c) exception (`internal`/`result_schema` and the serialization case).
    Second, the `kiln-cli` section gains one sentence pointing to `docs/protocol/` for options
    and subcommands documented there.
  - Nothing else changes.

## Pinned
- (a) Errors: no new code or kind. The cap decision is the only change.
- (b) Files: `docs/control-protocol.md` (the sentences above); no runtime file.
- (c) Order: string caps (200 chars) → measure the id-null line → truncate `details` if > 4096.
- (d) n/a: no paths.
- (e) Threat model: n/a. The cap is a size bound, not a security boundary. The id is
  client-chosen and echoed by design (R3-SG1).
- (f) Tests pinning a snapshot: every `acceptance_t004*` test stays unchanged and green. The
  T-004 reviewer's example (5000-character id + `doc.insert {"doc":[…]}`) changes by design: it
  now keeps `details`.
- (g) Worked example (from R3-N1, recomputed by the test, not by hand): the request
  `{"id":"<5000 × a>","command":"doc.insert","params":{"doc":[1]}}`. Today: `details` =
  `{"truncated":true}` in a 5118-byte line. After: the full `details` list. The `error` object
  equals the one for the same request with `"id":1`.
- (h) Scorecard: none. The test author may add AC-1's test to `control.errors`.

## Acceptance criteria
- AC-1 (id not counted): through `Host::handle_line` and through `kiln-cli control`, send the
  same failing request with `"id":1` and with a 5000-character string id. The two `error`
  objects are identical, and neither has `{"truncated":true}`, since the `id:1` line is under
  4096 bytes. Oracle: differential.
- AC-2 (exact boundary, response): build requests whose id-null response length steps through
  4094…4098 bytes (the test finds them by growing one param string a char at a time). `details`
  is kept exactly while the measured length is ≤ 4096, and the same holds with a long id added.
  Oracle: exact value.
- AC-3 (exact boundary, `exec`): a hand-built `CommandError::InvalidParams` whose capped error
  object from `command_error_json` is exactly 4096 bytes keeps `details`; with one byte more,
  `"details":{"truncated":true}`. Oracle: exact value.
- AC-4 (no other change): every T-004 acceptance test and the protocol document's worked
  example give byte-identical output. Oracle: regression.
- AC-5 (document): the reviewer checks the moved sentence, the caps text against AC-1–AC-3,
  and that only the listed sentences changed (`git diff`).

## Scorecard items and intended test names
None. `crates/kiln-control/tests/acceptance_t030.rs`: `echoed_id_does_not_count_toward_cap`,
`response_cap_boundary_is_exactly_4096_bytes`, `exec_cap_boundary_is_exactly_4096_bytes`,
`other_responses_are_unchanged`.

## Out of scope and implementer notes
- Structured reasons, the generated kind list, kind derivation: T-032. New kinds: the tasks
  that need them.
- Your tests: `crates/kiln-control/tests/caps.rs`, unit tests in `lib.rs`.

## Decisions
- D-1 (director, R-051): created by splitting T-007.
- D-2 (director, R-057): kind list generated from code; stale sentences corrected here.
- R-060 (director): contracts first. Part (d) moved to T-015, then to T-032 (D-041); this task keeps part (e) and the
  D-2 sentence fixes, re-tiered by ADR-0002.

## Questions
None open.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · tokens (if known) · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13T03:05Z · R-051 · (new) · did: wrote T-030 from T-007's former parts (d) and (e),
  per the director's split: `RejectReason`/`InternalReason` with a source table (including
  `OutsideRoot` from T-008 and the panic path), kinds from reasons only, byte-identical protocol
  output, and the D-032 cap rule (id measured as null; exact 4096 boundary via the public
  `command_error_json`). Read `kiln-control`: today `Internal` kinds (`panic`, `result_schema`)
  are also derived from message text, so the internal reasons are in scope too. Depends on T-007
  (shared files, final `internal` path) and T-008 (`OutsideRoot`). Tier high: `CommandError`
  contract, L3 + L4. No gate path, no dependency. · gate attempts: 0 · missing context: none ·
  next: director review.
- 2026-11-13T03:50Z · R-055 · draft · did: brought the spec to v1.1: tier high under ADR-0002
  (criterion: it changes the registry's error contract `CommandError`, i.e. how the registry
  works, a CHARTER §3 contract change; an ADR is due); budget 850k; Pinned; `depends_on` +T-010
  with two reason rows and `NotABoundary` for its position refusals; renamed the control test
  target to `acceptance_t030_control.rs`. Arm n/a (high). · gate attempts: 0 · missing context:
  none · next: director review.
- 2026-11-13T03:56Z · R-057 · draft · did: applied D-2: generated `docs/protocol/error-kinds.md`
  with a currency test and a `generated.toml` entry (gate path, so `gate paths` in the
  approval), table in `docs/control-protocol.md` replaced by a pointer, and the stale sentences
  other tasks may not edit assigned here; AC-10; dispatch note revised. · gate attempts: 0 ·
  missing context: none · next: director review, ADR.
- 2026-11-13 · R-060 · draft · did: shrank to part (e) per the director's restructuring. Part (d) (reasons, generated kind list, ADR, the `xtask/generated.toml` entry) moved to T-015. Re-tiered medium by ADR-0002: no high criterion is left (no gate path, no §3 contract, no boundary); the number is even, so there is an independent test author. `depends_on` [T-015] (shared `crates/kiln-control/src/lib.rs` and `docs/control-protocol.md`). Kept D-2's stale-sentence fixes, since they are document-only and sit in the same file. The file name keeps its old slug (no renumbering). · gate attempts: 0 · missing context: none · next: director.
- 2026-11-13 · R-066 · draft · did: D-041: T-032 (not T-015) rewrites `kiln-control`'s kind derivation and the protocol document's sentences, so `depends_on` is T-032. The stale-sentence item for the "This table is the specified set" sentence is done by T-032. · gate attempts: 0 · missing context: none · next: director.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
