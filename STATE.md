# State

_Last updated: 2026-11-13 00:20 UTC (session 01)_

## Now
Iteration 0 — Foundations, nearly complete.

| Task | State | Notes |
|---|---|---|
| T-001 text core | merged | 21 acceptance tests; 1 review round |
| T-002 documents and file I/O | merged | 25 acceptance tests; 3 review rounds (3 save-path defects found by review) |
| T-003 document commands | merged | 23 acceptance tests; revert check 23/23; 1 review round |
| T-004 control protocol + `kiln-cli run` | in_progress | implementer R-026 and test author R-027 running |
| T-005 queue hardening | tests_joined | manual join (D-015); reviewer R-024 running (restarted after container restart) |

GitHub Actions on `main`: 27 runs, latest (`ad81ab4`, T-003 merged) green.

## Next
1. T-005: review → approval row → queue (last use of the bootstrap queue). After it merges, the hardened
   queue is in force: precheck refusals, main's gate, generated-file regeneration, wrong-checkout guard.
2. T-004: join → revert check → review → approval → queue. That closes the vertical slice.
3. Evaluator on the slice (fresh agent, public interfaces only) → `evaluations/`.
4. Retrospective 0: score claims C1–C8 from `metrics/`; methodology v1.0 → v1.1. Change candidates so far are
   in `docs/decisions.md` D-010, D-011, D-012, D-015, D-019, D-022 (guards vs acceptance tests; stub/logic
   commit split and mutation spot-check; generated files; join search path; stale-binary guard; scratch
   cleanup and disk monitoring; never `git stash`; real timestamps only; tier inflation — all five tasks high).
5. Follow-up tasks to write: T-006 (spec text + tests for the T-001 and T-003 spec-gap rulings),
   T-007 (replay verifies results; read-only commands change nothing but the log), budgets runner.

## Resume
Read `docs/CHARTER.md`, `methodology/METHODOLOGY.md`, `docs/decisions.md`, then `tasks/` (state in front
matter, role logs at the bottom) and `metrics/`. Worktrees live in `.worktrees/` (not pushed). Director
commands must never export `CARGO_TARGET_DIR` (see D-019). Run IDs used so far: R-001 … R-027.
