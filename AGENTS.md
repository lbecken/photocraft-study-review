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
8. **Provenance.** Every commit message carries `Task: T-NNN` and `Role: <role>` lines. Never
   write model names or session identifiers into code comments or docs.

## Picking up a task
- Builders (implementer, test author, reviewer) work in the worktree the director names:
  `.worktrees/T-NNN` on `task/T-NNN`, or `.worktrees/T-NNN-tests` on `task/T-NNN-tests`, each
  with its own target directory (created by `cargo xtask worktree new`). Work only there.
- Planner, evaluator and retrospective analyst write their files where their brief says; the
  director commits them.
- Read the task log first: earlier runs may have left questions, failures or a blocked report.
- Do not change the task's front matter; the director owns `state`.

## Definition of done (gate: `cargo xtask ci`)
| Tier | Your run is done when |
|---|---|
| low | implementer: change and its tests committed; `cargo xtask ci` green in the worktree; log entry written |
| medium | implementer: as low, never touching `acceptance_t*` or `scorecard/`. Test author: acceptance tests committed, each criterion has a test that fails on base, results recorded. Reviewer: review file with verdict committed |
| high | as medium; the task also needs a director approval row in `docs/decisions.md` and, if a CHARTER §3 contract changes, an ADR — the director handles both |

The task is done only when the queue has merged it and, for scorecard items, the evaluator has
confirmed them. Your run never declares that; it is computed (METHODOLOGY §3, §9).

## Stop conditions
Stop and write a blocked report (a log entry starting `BLOCKED:` with your diagnosis) instead
of pushing through when:
- 3 consecutive `cargo xtask ci` attempts fail on the same failure;
- you reach the token allotment the director gave you;
- you need to change a path outside `touches`;
- you need to change a CHARTER §3 contract;
- the spec or an acceptance test looks wrong (a `QUESTION:` line in your log entry).
A clear blocked report is a good outcome.

## Before you stop, always
1. Append one entry to **your role's section** of the task file's Log (format in the task file):
   what you did, gate attempts and last result, what you did not run, missing context, next step.
2. Commit your work on your branch with the `Task:` and `Role:` lines.
3. Leave the worktree building.
Say plainly what you did not verify.
