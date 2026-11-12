# State

_Last updated: 2026-11-12 23:10 UTC (session 01)_

## Now
Iteration 0 — Foundations. Bootstrap done (D-005 exemption ended in spirit; queue not yet used on the real repo).
Wave 1 dispatched: T-001 implementer (R-004), T-001 test author (R-005), T-005 implementer (R-006).

## Next
1. When a slot frees: T-005 test author, then T-002 implementer + test author.
2. Per task: join (`cargo xtask join`, plus `--revert-check`), reviewer, director approval row in `docs/decisions.md`, then `cargo xtask queue`.
   Until T-005 lands, the director resolves `Cargo.lock` / `docs/scorecard.md` rebase conflicts by regenerating them.
3. T-003 after T-001 + T-002 merge; T-004 after T-003. Then evaluator on the slice, retrospective 0.

## Resume
Read `docs/CHARTER.md`, `methodology/METHODOLOGY.md`, `docs/decisions.md`, then `tasks/` (state in front
matter, role logs at the bottom) and `metrics/`. Worktrees live in `.worktrees/` (not pushed): if the
container was replaced, in-flight task branches are lost and those tasks restart from `ready`.
Run IDs used so far: R-001 … R-006.
