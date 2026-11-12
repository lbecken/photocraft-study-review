# Owner-delegate decision log

The human owner chose "I decide, you audit": the director approves high-risk items on the
owner's behalf and records each one here for later review.

| # | Date | Decision | Reasoning | Reversible? |
|---|---|---|---|---|
| D-001 | 2026-11-12 | Product is Kiln, a desktop code editor slice for Rust (owner's choice among three options). | Tests the IDE-transfer claims of the study directly. | yes |
| D-002 | 2026-11-12 | Scope "medium": about 40–70 tasks over 3 iterations (owner's choice). | Enough parallel work to exercise the queue and independent testing. | yes |
| D-003 | 2026-11-12 | Integration by local serial merge queue to `main`, GitHub Actions re-checks `main`; no pull requests (owner's choice). | Real queue semantics without PR noise. | yes |
| D-004 | 2026-11-12 | Architecture in `docs/CHARTER.md` §3 adopted as the initial binding contract. | Contracts before fan-out (claim C1); mirrors the verified PhotoCraft mechanisms and fixes two planned-but-unbuilt ones (validated command schema, op-based undo). | by ADR |
| D-005 | 2026-11-12 | Iteration 0 bootstrap is built directly on `main` by one supervised agent, before the gate and queue exist. | The queue cannot gate its own creation; the exemption ends when `cargo xtask queue` first runs. | n/a |
| D-006 | 2026-11-12 | Builder agents run on the model the owner specified for this experiment; the director plans, reviews and integrates. | Owner's instruction. | yes |
| D-007 | 2026-11-12 | Methodology v1.0 accepted with its interim answers to open questions O-1…O-8: the director acts as integrator; test authors get their own worktree; after a queue failure only the delta since the last verdict is re-reviewed; "does not compile on base" is weak evidence and a revert check is required; token budgets are enforced by the director after each run (agents cannot see their own usage) with gate-attempt count as the in-run proxy; task front matter gains `iteration`, `found_by`, `origin_task`; concurrency cap 3. | Each is a gap the methodology author found in the director's brief; all are cheap to revisit at retrospective 0. | yes |
