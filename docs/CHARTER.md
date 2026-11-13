# Charter

Owner: the repository owner (human). Director and owner-delegate: the orchestrating session.
Builders: AI coding agents working under `methodology/`.

## 1. What this experiment must answer

The PhotoCraft study found a method that produces a lot of software quickly, and found where
it was hollow. This experiment asks whether a corrected method keeps the speed and closes the
gaps. The specific claims under test (study references in brackets):

| # | Claim under test | PhotoCraft weakness it answers |
|---|---|---|
| C1 | Build contracts and checks first, fan agents out second | fan-out from day 1; architecture plan items never built [D03, D10] |
| C2 | A serial merge queue keeps main green at agent speed | individually-green PRs colliding on main [D07, E07-003] |
| C3 | Tests written by a different agent than the implementer catch defects the implementer's own tests miss | tests asserting nothing, fixes merged without tests [E07-015, E07-018] |
| C4 | Feature status computed from behaviour tests, plus an independent evaluator, prevents over-claimed completion | "625/625" measured wiring; inert settings [E07-013, E07-014] |
| C5 | Risk tiers put review effort where defects are expensive | review never blocked a merge; bots present, approval absent [D06] |
| C6 | Budgets that gate (latency, token) stay honest | 0 of 102 perf budgets enforced [errata #11] |
| C7 | State, plans, decisions and provenance kept in the repo make the work resumable and auditable | private plan/ and devlog; attribution banned [D03, D05] |
| C8 | An editor's "correct" can be defined by constructed oracles (compiler, parser corpus, round-trip and differential properties) | the IDE has no reference binary [D10] |

A claim is supported or refuted by numbers in `metrics/`, not by impression.

## 2. The product: Kiln

A small desktop code editor for Rust projects. It must be usable: open a project, edit,
search, see syntax colours and compiler diagnostics, format, save.

Target feature surface (the scorecard in `scorecard/` is the authoritative list, about 50 items):
text buffer and editing; undo/redo; file open/save with encodings and line endings;
multi-document workspace and project tree; syntax highlighting, outline and bracket matching
for Rust; search and replace in file and in project; go-to-line and go-to-symbol;
`cargo check` diagnostics; `rustfmt` formatting; command palette; keymap; headless CLI and
control protocol; desktop UI.

Out of scope: plugins, LSP server integration, debugger, languages other than Rust, remote
editing, collaborative editing, VCS integration.

"Done" for the product means: every scorecard item is `verified` (its named behaviour tests
pass and the independent evaluator confirmed it through a public interface), the gate is
green, and the enforced budgets hold.

## 3. Architecture decisions (binding; change only by ADR)

Layers — a crate may depend only on crates in lower layers. Enforced by `cargo xtask layers`.

| Layer | Crate | Responsibility |
|---|---|---|
| L0 | `kiln-text` | rope buffer, positions (byte / char / UTF-16 / line-col), edits, line endings, encoding, operation-based undo/redo |
| L1 | `kiln-syntax` | tree-sitter integration (Rust), incremental reparse, highlight spans, outline, bracket matching |
| L1 | `kiln-search` | literal and regex search and replace over a buffer |
| L2 | `kiln-workspace` | virtual file system (memory + disk), open documents, project tree, find-in-files, diagnostics store, external tools (`cargo check`, `rustfmt`) |
| L3 | `kiln-commands` | the command registry: every state change goes through a named command with schema-validated JSON params and results; command log for replay |
| L4 | `kiln-control` | headless host and JSON-lines control protocol |
| L5 | `kiln-ui` | egui user interface; may only dispatch commands and read state |
| — | `kiln-testkit` | fixtures, pinned corpus, differential-test helpers (dev-dependency of any layer) |
| app | `apps/kiln` (desktop), `apps/kiln-cli` (headless) | thin binaries |
| tool | `xtask` | gate, architecture checks, scorecard, worktree, queue, metrics |

Decisions:
1. **Headless core.** L0–L4 have no UI or GPU dependency. Everything the UI can do, the CLI can do.
2. **One command registry.** Commands declare typed params/results (serde + JSON Schema); the
   registry rejects invalid params. UI, CLI, control protocol and tests all go through it.
   (PhotoCraft planned a validated schema and never built it.) Refusals carry a structured
   reason; the protocol's error kinds are derived from it and listed in the command
   description generated from code (ADR-0004).
3. **Operation-based undo**, not whole-document snapshots.
4. **Determinism.** No wall-clock, randomness or thread-timing dependence in L0–L4 behaviour;
   external tools are behind a trait with a recorded-output fake.
5. **Constructed oracles** define correct: differential tests against a naive model (text,
   search), round-trip and idempotence properties (I/O, format, undo), "compiles ⇒ parses
   without error nodes" on a pinned corpus, incremental == from-scratch for reparse and index.
6. **Shared-file hygiene.** Registries and trackers are one-file-per-item directories
   (`scorecard/items/*.toml`, `tasks/*.md`, one module per command group) so parallel agents
   do not edit the same file.

## 4. Operating interface (names every document and tool must use)

- Gate: `cargo xtask ci` — identical locally and in GitHub Actions. Steps, cheap first: fmt,
  clippy `-D warnings`, `layers`, `ui-commands`, `hygiene` (review front matter, no session-specific paths), `schema`, `scorecard --check`, tests,
  `no-skip` (an ignored or skipped test fails the gate unless listed with an expiry),
  command fuzz smoke, enforced budgets.
- `cargo xtask worktree new T-NNN` — creates `.worktrees/T-NNN` on branch `task/T-NNN` with
  its own target directory.
- `cargo xtask queue T-NNN` — the merge queue: rebase the task branch on `main`, run the full
  gate on the result, fast-forward `main` only if green, append the result to
  `metrics/queue.jsonl`. One at a time. `main` is never pushed red.
- `cargo xtask scorecard [--check]` — derives each item's status from its named tests;
  status is never edited by hand.
- `cargo xtask metrics …` — appends per-task records to `metrics/tasks.jsonl`.
- Task files: `tasks/T-NNN-slug.md` with front matter `id, title, tier (low|medium|high),
  state, owner_role, scorecard_items, depends_on, touches (paths), budget_tokens, iteration,
  found_by, origin_task` (the last two only on defect/follow-up tasks).
  States: `draft → specified → ready → in_progress → self_verified → tests_joined → reviewed
  → queued → merged → evaluated` (or `blocked`, `abandoned`).
- Risk tiers (ADR-0002, by the boundary a change crosses; the highest that applies): **high**
  (touches the file system's persistence path, spawns processes, changes a gate path, a security
  or confinement boundary, adds or changes a dependency, or changes a §3 contract), **medium**
  (the default for any other behaviour change; adding a command is not a contract change),
  **low** (documentation, tests, or refactoring with no behaviour change).
- Roles: director (orchestrator; owner-delegate; also acts as integrator by running the
  queue), planner, implementer, test author, reviewer, evaluator, retrospective analyst.
  At most 3 builder agents compile at once; a short test-author fix round counts as one
  (4-core machine; recalibrate at retrospectives).

## 5. Iterations

| Iteration | Purpose | Exit criteria |
|---|---|---|
| 0 Foundations | workspace, gate, architecture checks, queue, CI, scorecard machinery, one vertical slice (open → edit → save, driven headlessly) | gate green on main; queue has merged ≥3 tasks; slice passes evaluator |
| 1 Fan-out | parallel feature work across L0–L4; independent test authors on high-tier tasks and on one arm of the medium-tier experiment (ADR-0002, D-035) | ≥60% scorecard items verified; metrics complete for every task |
| 2 Product | UI, end-to-end flows, budgets, hardening | every item verified; evaluator sign-off; budgets enforced |

Each iteration ends with a retrospective (`retrospectives/`) that scores claims C1–C8 from
the metrics and proposes methodology changes. The methodology version is bumped only then.
