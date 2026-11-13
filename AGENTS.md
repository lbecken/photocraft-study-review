# AGENTS.md — read this first

This repository is an experiment: AI agents build **Kiln**, a small desktop code editor for Rust
projects, under a versioned methodology, to test claims C1–C8 in `docs/CHARTER.md`. You are one
agent in one role on one task. You have no memory of earlier sessions; the repository is the
memory. The director (the orchestrating session) gave you a role, a task ID `T-NNN` and a run ID
`R-NNN`.

## Read, in this order
1. This file.
2. Your role brief: `methodology/roles/<role>.md`.
3. Your task file `tasks/T-NNN-*.md` and what its reading list names.
4. As needed: `docs/CHARTER.md` (binding architecture §3, names §4),
   `methodology/METHODOLOGY.md` (the full process), `docs/decisions.md`, `docs/adr/`.

## Workspace map (CHARTER §3)
A crate may depend only on crates in lower layers (`cargo xtask layers`).

| Layer | Path | Responsibility |
|---|---|---|
| L0 | `crates/kiln-text` | rope buffer, positions, edits, line endings, encoding, op-based undo |
| L1 | `crates/kiln-syntax` | tree-sitter Rust, incremental reparse, highlight, outline, brackets |
| L1 | `crates/kiln-search` | literal and regex search/replace |
| L2 | `crates/kiln-workspace` | VFS, documents, project tree, find-in-files, diagnostics, `cargo check`/`rustfmt` behind a trait |
| L3 | `crates/kiln-commands` | command registry: named commands, schema-validated JSON params/results, replay log |
| L4 | `crates/kiln-control` | headless host, JSON-lines control protocol |
| L5 | `crates/kiln-ui` | egui UI; only dispatches commands and reads state |
| — | `crates/kiln-testkit` | fixtures, pinned corpus, differential helpers (dev-dependency) |
| app | `apps/kiln`, `apps/kiln-cli` | thin binaries (desktop, headless) |
| tool | `xtask` | gate, checks, scorecard, worktree, queue, metrics |

Records: `tasks/`, `reviews/`, `evaluations/`, `incidents/`, `retrospectives/`, `metrics/`,
`scorecard/items/`, `handoffs/`, `STATE.md`.

## Golden rules
1. **State changes only through the command registry.** UI, CLI, control protocol and tests all
   dispatch named commands; the registry validates params against their schema.
2. **Respect the layers.** No upward dependency; L0–L4 have no UI or GPU dependency.
3. **Determinism.** No wall clock, randomness or thread timing in L0–L4 behaviour; external tools
   only through their trait, with the recorded-output fake in tests.
4. **Never weaken a test or a gate to get green.** No deleting or loosening assertions, no
   `#[ignore]`, no skip, no allowlist growth, no lowering a budget. If the test or gate is wrong,
   stop and say so.
5. **Stay inside `touches`.** Your task file lists the paths you may change. Everything else
   may be in another agent's hands right now.
6. **Only the queue moves `main`.** Never commit, merge or push to `main`; commit only to your
   task branch. `cargo xtask queue` is run by the director.
7. **Use the xtask commands as named.** If a `cargo xtask` subcommand you need does not exist or
   misbehaves, stop and report; do not substitute an ad-hoc script.
8. **Provenance.** Every builder commit carries `Task: T-NNN` and `Role: <role>` lines
   (`implementer`, `test-author`, `reviewer`). Never write model names or session identifiers
   (including session-scoped scratch paths) into code, docs or records.
9. **No brittle pins.** No test pins a message text, a whole document or a command list that a
   drafted task changes; that this task left something alone is the reviewer's diff check, not a
   guard (METHODOLOGY §6).

## Picking up a task
- Builders (implementer, test author, reviewer) work in the worktree the director names:
  `.worktrees/T-NNN` on `task/T-NNN`, or `.worktrees/T-NNN-tests` on `task/T-NNN-tests`, each
  with its own `target/` (created by `cargo xtask worktree new`). Work only there.
- Planner, evaluator and retrospective analyst write their files where their brief says; the
  director commits them.
- Read the task log first: earlier runs may have left questions, rulings or a blocked report.
- Acknowledge each ruling in your log, accepting it or questioning it, before you act on it
  (METHODOLOGY §2.1).
- Do not change the task's front matter; the director owns `state`.
- Log every question (`QUESTION:` line) by the end of your first gate attempt, then go on with
  the rest; the director answers all builders at once in the task file.

## Working environment (each rule cost real work in iteration 0)
- **Commit early**, at every working step: a restart and a usage-limit stop lost only
  uncommitted work (D-022, D-024).
- **Never `git stash`**: all worktrees share one stash ref, so a stash can reach another agent's tree (R-006).
- **Never set or export `CARGO_TARGET_DIR`**, whatever a tool prints; if it is set, `unset` it.
  Cargo then uses `<worktree>/target`. A shared target served a stale binary (D-019).
- **Clean up** scratch builds and toy repositories you made outside your worktree before you
  stop; 9 GB of them left 6 GB free disk (D-022).
- Every process a test spawns has a timeout (D-028 (8)).

## Definition of done (gate: `cargo xtask ci`)
Tiers (CHARTER §4, ADR-0002): **high** = persistence path, spawned processes, gate paths, a
security or confinement boundary, a dependency change, or a CHARTER §3 contract; **medium** =
the default for any other behaviour change; **low** = docs, tests or refactoring with no
behaviour change.

| Tier | Your run is done when |
|---|---|
| low | implementer: change and its tests committed; `cargo xtask ci` green in the worktree; log entry written |
| medium | implementer: as low, never adding `acceptance_t*` files or editing `scorecard/` on `task/T-NNN`. Acceptance tests: by the test author (even task number) or by the implementer on `task/T-NNN-tests` (odd), each criterion with a test that fails on base, results recorded. Reviewer: full round 1 incl. a mutation spot-check, delta round 2 final, review file committed |
| high | as medium, always with an independent test author; the reviewer also probes the boundary adversarially; the director writes the approval row and any ADR |

The task is done only when the queue has merged it and, for scorecard items, the evaluator has
confirmed them. Your run never declares that; it is computed (METHODOLOGY §3, §9).

## Stop conditions
Stop and write a blocked report (a log entry starting `BLOCKED:` with your diagnosis) instead
of pushing through when:
- 3 consecutive `cargo xtask ci` attempts fail on the same failure;
- you need to change a path outside `touches`;
- you need to change a CHARTER §3 contract;
- the spec, a ruling or an acceptance test looks wrong (a `QUESTION:` line in your log entry).
A clear blocked report is a good outcome: both stops in iteration 0 exposed a director error
(D-017, D-020).

## Before you stop, always
1. Append one entry to **your role's section** of the task file's Log (format in the task file):
   what you did, gate attempts and last result, what you did not run, missing context, next step.
2. Commit your work on your branch with the `Task:` and `Role:` lines.
3. Leave the worktree building.
Say plainly what you did not verify.
