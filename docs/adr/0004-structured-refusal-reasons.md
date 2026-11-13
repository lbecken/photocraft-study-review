---
id: ADR-0004             # file docs/adr/0004-structured-refusal-reasons.md
title: "Every command refusal carries a structured reason; protocol kinds come from reasons; the kind list is generated from code"
status: proposed
date: 2026-11-13
task: T-032
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
- `CommandError::Rejected` carries only a message. `kiln-control` derives the protocol `kind`
  by matching message text (`rejected_kind`, `crates/kiln-control/src/lib.rs`). A reworded
  message silently becomes `other` (T-004 reviews; D-031 (9)–(10)).
- Iteration 1 plans 13 new kinds: 11 for feature tasks, `outside_root` (T-008) and
  `not_a_boundary` (T-010). With a hand-written list, every task that adds one edits the same
  document (director rulings D-2, R-056).
- Fact: on `main`, 23 `(code, kind)` pairs are documented in `docs/control-protocol.md`, and the
  10 `rejected` kinds come from message matching.

## Options
| Option | For | Against |
|---|---|---|
| A: reason enum on `Rejected`/`Internal`; kinds from it; the list generated into `docs/commands.json` by `Registry::describe()` | a kind cannot drift from code; a new kind is one variant plus a regeneration the gate already checks; no new generated path, no `xtask/` change | changes the registry's error type at every refusal site |
| A′: as A, but a new generated file `docs/protocol/error-kinds.md` | a separate human-readable page | a new `xtask/generated.toml` entry breaks the T-005 toy-repository tests and the pinned list in `xtask/src/policy.rs` (spec check R-062, blockers 1–2); a gate path |
| B: keep messages, generate the list only | small | kinds still depend on message text |
| Keep the current contract | no work | silent `other` on rewording; kind-adding tasks serialise on one document |

## Decision
Option A.
- `CommandError::Rejected { reason: RejectReason, message }` and
  `CommandError::Internal { reason: InternalReason, message }`. The reason is set where the
  refusal is raised.
- Lower-layer error enums map to reasons through exhaustive `From` impls without a catch-all
  arm.
- The protocol `kind` is `reason.as_str()`. No message text is consulted.
- `Registry::describe()` publishes `error_kinds` (`rejected` and `internal`, each kind with its
  description), so it lands in the generated `docs/commands.json`, which the gate's `schema`
  step compares byte for byte.
- `docs/control-protocol.md` keeps its table of request-level rows and iteration-0 kinds; kinds
  added later are listed only in `error_kinds`. A task that adds a kind adds one variant with a
  description and regenerates `docs/commands.json`.
- Reason variants may exist before any command raises them. The generated list is the set of
  kinds the protocol may produce.

## Consequences
- T-032 carries the change and applies the CHARTER sentence (`docs/CHARTER.md` in its
  `touches`). Hence it depends on T-013, which holds that file. T-007, T-008, T-010, T-030 and
  the feature tasks that raise a refusal (T-018 onward, except T-025) follow it.
- No iteration-0 test or fixture changes: no `xtask/` path; the T-004 test that reads kinds from
  `docs/control-protocol.md` keeps passing because the table is kept;
  `acceptance_t003_registry.rs` (`docs/commands.json == describe()`) holds after regeneration.
- `reason.rs` is one enum. T-008, T-010 and T-027 each add a variant or mapping there, so they
  are not run in flight together.
- CHARTER §3 decision 2, appended: "Refusals carry a structured reason; the protocol's error
  kinds are derived from it and listed in the command description generated from code."
- Checks: the `schema` gate step (currency); T-032's acceptance tests; the reviewer, for the
  absence of catch-all arms (rule only: a lint cannot see intent).

## Verification
- T-032 AC-1 to AC-4: reasons at their sources, kinds independent of message text, the
  generated list equal to the reasons and consistent with the table, protocol bytes unchanged.
- Retrospective 1 counts:
  - tasks that added a kind (expected 13), and how many edited `docs/control-protocol.md` for
    it (expected 0);
  - `other` kinds seen in evaluator sessions (expected 0);
  - queue conflicts on `reason.rs` and on `docs/commands.json`, and whether regeneration
    resolved them.

## Rollback
Restore message-based kinds by a new ADR. Cost: one task that removes `reason` from
`CommandError`, which compiles only once every refusal site is edited back; `error_kinds` is
dropped from `describe()`. No data or file format depends on reasons.
