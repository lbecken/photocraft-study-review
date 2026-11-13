---
id: ADR-0004             # file docs/adr/0004-structured-refusal-reasons.md
title: "Every command refusal carries a structured reason; protocol kinds come from reasons; the kind list is generated from code"
status: proposed
date: 2026-11-13
task: T-015
charter_section: "§3 decision 2"
approval: null           # director: the D-NNN row that accepts it
supersedes: null
superseded_by: null
---

# ADR-0004 Structured refusal reasons and a generated kind list

## Context
- CHARTER §3 decision 2 makes the command registry the one interface for every state change.
  Its error type `CommandError` is part of that interface for every caller: UI, CLI, control
  protocol and tests.
- `CommandError::Rejected` carries only a message. `kiln-control` derives the documented protocol
  `kind` by matching message text (`rejected_kind`, `crates/kiln-control/src/lib.rs`). A
  reworded message silently becomes `other`. The T-004 reviews and D-031 (9)–(10) recorded this.
- The documented kinds are a hand-written table in `docs/control-protocol.md`. Iteration 1
  plans 13 new kinds: 11 for features (T-015) plus `outside_root` (T-008) and `not_a_boundary`
  (T-010). With a hand-written table, every task that adds one edits the same file, so those
  tasks cannot be in flight together (METHODOLOGY §5.2; director rulings D-2, R-056).
- Fact: on `main`, 23 `(code, kind)` pairs are documented, and the 10 `rejected` kinds come
  from message matching (`docs/control-protocol.md`, "Error codes and kinds").

## Options
| Option | For | Against |
|---|---|---|
| A: reason enum on `Rejected`/`Internal`, kinds from it, list generated from code | kind cannot drift from code; a new kind is one enum variant plus a regenerated file | changes the registry's error type (every refusal site, one ADR); a generated path and its gate check |
| B: keep messages, generate only the list | smaller change | kinds still depend on message text |
| Keep the current contract | no work | silent `other` on rewording; every kind-adding task serialises on one document |

## Decision
- `CommandError::Rejected { reason: RejectReason, message }` and
  `CommandError::Internal { reason: InternalReason, message }`. The reason is set where the
  refusal is raised.
- Mappings from lower-layer error enums to reasons are exhaustive and have no catch-all arm.
- The protocol `kind` of a `rejected`/`internal` error is `reason.as_str()`. No message text is
  consulted.
- `docs/protocol/error-kinds.md` is generated from code (request-level pairs plus every reason
  with its description). It is a generated path in `xtask/generated.toml`, and a test in the
  gate's `test` step fails when it is stale.
- A task that adds a kind adds a reason variant with a description and regenerates the file. It
  does not edit `docs/control-protocol.md`.

## Consequences
- T-015 carries the change; T-007, T-008, T-010, T-030 and every iteration-1 feature task follow
  it. T-008 adds `OutsideRoot` and T-010 adds `NotABoundary` through this mechanism.
- Two tasks that each add a variant can still conflict on rebase in `reason.rs` (one enum), so
  they are not run in flight together. The generated file regenerates itself.
- CHARTER §3 decision 2, to append: "Refusals carry a structured reason; the protocol's error
  kinds are derived from it and listed in a file generated from code."
- Checks: the currency test (`kiln-control`); the queue regenerates the file on conflict; the
  reviewer checks that no catch-all arm exists (rule only: a lint cannot see intent).

## Verification
T-015 AC-1 to AC-4: reasons at their sources, kinds independent of message text, byte-identical
protocol output on base sessions, generated list current and complete.

## Rollback
Restore message-based kinds by a new ADR. Cost: one task that removes `reason` from
`CommandError`, which compiles only once every refusal site is edited back, and one that
restores the hand-written table. No data or file format depends on reasons.
