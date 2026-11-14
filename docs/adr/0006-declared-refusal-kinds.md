---
id: ADR-0006
title: "Each command declares the refusal kinds it can report; only declared kinds are published"
status: accepted
date: 2026-11-14
task: T-038
charter_section: "§3 decision 2"
approval: D-128
supersedes: null
superseded_by: null
---

# ADR-0006 Declared refusal kinds

## Context
- ADR-0004 made every refusal carry a `RejectReason` and generated `error_kinds` from the
  reason table: "Reason variants may exist before any command raises them. The generated list
  is the set of kinds the protocol may produce."
- `docs/control-protocol.md` calls the generated list "the specified set; no other pair is
  produced". The iteration-1 evaluation (NEW-2) found 11 published kinds no command could
  produce; on main at T-038's spec check (R-209) 11 of 24 still have no reachable raise site.
  NEW-3 found a `when` text wider than the binary; five texts are now wider or narrower.
- D-091 change 18 decided that the published specified set holds only kinds a registered
  command can produce, and left the means to T-038; D-092 chose option B.
- Rust cannot list the reasons a function returns, so the tie must be declared and checked.

## Options
| Option | For | Against |
|---|---|---|
| A: remove table rows until a task raises them | the list is exact; no new mechanism | every kind-adding task edits `reason.rs` again (the serialisation ADR-0004 removed); `From<&ToolError>` must go |
| B: commands declare their kinds; publish only declared kinds; check at execute; witness in the audit | exact list plus a per-command list; both directions mechanical; tasks declare in their own module | a declaration and refusal examples per command; disk-only pairs witnessed by tests, not the audit |
| C: a hand-set `reserved` flag per row | small | flags serialise tasks on `reason.rs`; nothing checks an unflagged row can occur |
| Keep the current contract | no work | the published specified set is false (NEW-2) |

## Decision
Option B.
- Every command of the standard registry declares the `RejectReason`s it can report, each
  marked *example* (the in-memory file system can produce it) or *disk* (only a session on
  disk can).
- `Registry::describe()` publishes, per command, `refusals` (its declared kinds) and
  `refusal_examples`; `error_kinds.rejected` is the set of kinds some command declares, in
  table order. A reason variant may exist before any command declares it; it is then not
  published.
- `Registry::execute` reports a refusal of an undeclared kind as `internal`/`other`.
- `Registry::audit` (the gate's `schema` step through `kiln-cli commands --audit`) refuses a
  command without a declaration, an *example* kind without a refusal example that produces it,
  a refusal example that does not produce its kind, and a refusal example of an undeclared or
  *disk* kind.
- ADR-0004's bullet "Reason variants may exist before any command raises them. The generated
  list is the set of kinds the protocol may produce." is replaced by: "Reason variants may
  exist before any command declares them. The generated list is the set of kinds some
  registered command declares; a declaration is checked at execute and witnessed in the
  audit."

## Consequences
- T-038 carries the change. Every later task that adds a command or makes a command report a
  new kind declares it in its own module and adds a refusal example (T-022, T-024 to T-027,
  T-031); `reason.rs` changes only when a new row is needed.
- `docs/control-protocol.md` states the rule (T-038 Interface 5); its `rejected`/`other` row
  is removed (D-092 Q-2).
- CHARTER §3 decision 2 text: unchanged ("listed in the command description generated from
  code" stays true).
- Checks: the `schema` gate step (audit and currency of `docs/commands.json`); T-038's
  acceptance tests; review for *disk* markings (rule only: no in-memory run can witness them).

## Verification
T-038 AC-1 to AC-6. Retrospective 2 counts: kind-adding tasks that edited `reason.rs` (expected:
only those adding a row), `internal`/`other` responses seen by the evaluator (expected 0), audit
failures at joins caused by missing declarations.

## Rollback
A new ADR; remove the declarations and the execute check, publish `RejectReason::ALL` again.
One task; no data or file format depends on declarations.
