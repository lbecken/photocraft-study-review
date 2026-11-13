# State

_Last updated: 2026-11-13 03:06 UTC (session 01)_

## Now
Iteration 0 — Foundations: all work merged; **retrospective 0 running** (analyst R-052, writes `retrospectives/iter-0.md`).

| Task | State | Notes |
|---|---|---|
| T-001 text core | merged | 21 acceptance tests; 1 review round |
| T-002 documents and file I/O | merged | 25 acceptance tests; 3 review rounds (3 save-path defects found by review) |
| T-003 document commands | merged | 23 acceptance tests; revert check 23/23; 1 review round |
| T-004 control protocol + CLI | merged | 32 acceptance tests; 3 review rounds; D-032 approval row written after the merge (slip) |
| T-005 queue hardening | merged | 41 acceptance tests; 3 review rounds, 5 implementation rounds; hardened queue now in force |
| T-012 publish iteration-0 evaluation | merged | first task through the hardened queue |

`main`: gate green, 376 tests; scorecard 22 verified / 3 implemented / 38 planned of 63.
Evaluation: `evaluations/iter-0.md` (22 confirmed, 0 rejected, 3 not observable from outside).

## Next
1. Read the retrospective; decide methodology v1.1 (director edits `methodology/` — a ledger path at a
   retrospective — with a CHANGELOG entry per change; record the decision in `docs/decisions.md`).
2. Iteration 1 planning. Drafts waiting: T-006 (low), T-007, T-008, T-009, T-030 (high), evaluator's T-010,
   T-011. Open questions: T-009 (record-commit signing, `history_anchor`, lint `allow` in test files); the
   evaluator's CRLF question (detected line ending changes nothing on save). The scorecard item
   `ws.path-confinement` is held at `<scratchpad>/held/ws.path-confinement.toml` and must land on T-008's
   branch with a regenerated scorecard (see D-034).
3. Then the remaining 38 scorecard items: syntax (tree-sitter), search, workspace tree and find-in-files,
   diagnostics, formatting, command palette/keymap, UI.

## Operating notes for the director (learned the hard way; see decisions D-019…D-034)
- Never export `CARGO_TARGET_DIR` in a director command; run `cargo xtask …` from the main checkout.
- Timestamps come from `date -u`, never from memory. One command per line when a later line depends on an
  earlier one succeeding (`set -e`), and no heredoc in the middle of an `&&` chain.
- Commit ledger files by explicit path (`git add tasks metrics docs/decisions.md …`), never `git add -A`.
- A ledger change that alters a generated file (new scorecard item, evaluation) must go through the queue.
- Write and verify the approval row before queuing; the hardened queue now enforces it.
- Agents interrupted by a container restart are relaunched fresh on the owner-specified builder model.
- Watch free disk (`df -h`); remove worktrees after merge; agents' scratch builds are not cleaned up for them.

## Resume
Read `docs/CHARTER.md`, `methodology/METHODOLOGY.md`, `docs/decisions.md`, then `tasks/` (state in front
matter, role logs at the bottom), `metrics/` and `retrospectives/`. Worktrees live in `.worktrees/` (not
pushed). Run IDs used so far: R-001 … R-052.
