# Development

How to build Kiln, run the gate, and use the task tooling (`cargo xtask …`). Names follow
CHARTER §4; the process around them is in `methodology/METHODOLOGY.md`.

## Toolchain and tools

| Tool | Version | Source |
|---|---|---|
| rustc / cargo | 1.97.0 | pinned by `rust-toolchain.toml` (rustup installs it on first use) |
| rustfmt | 1.9.0-stable | toolchain component |
| clippy | 0.1.97 | toolchain component |
| git | ≥ 2.38 (developed with 2.43.0) | system; `worktree`, `rebase`, `merge --ff-only` |
| cp | GNU coreutils (`cp -a --reflink=auto`) | system; seeds worktree target dirs |

No cargo plugins are required or installed: `cargo-nextest`, `cargo-deny` and `cargo-fuzz`
were absent on the bootstrap machine and nothing was installed. The gate uses `cargo test`
and runs test executables directly. A task that adds a tool records its exact install command
here.

## Layout

```
crates/kiln-text        L0  buffer (char-indexed insert/delete today)
crates/kiln-syntax      L1  empty, documented
crates/kiln-search      L1  empty, documented
crates/kiln-workspace   L2  empty, documented
crates/kiln-commands    L3  command registry, schema validation, buffer.* commands
crates/kiln-control     L4  empty, documented
crates/kiln-ui          L5  empty, documented
crates/kiln-testkit     —   empty, documented (dev-dependency only)
apps/kiln-cli           app headless CLI over the registry
apps/kiln               app desktop placeholder (exits 1: UI not built)
xtask                   tool gate, checks, scorecard, worktrees, queue, join, metrics
scorecard/items/*.toml  one file per feature item (status is derived, never stored)
metrics/*.jsonl         queue and task records (METHODOLOGY §13)
```

Workspace lints (`Cargo.toml`): `unsafe_code`, `clippy::{unwrap_used, expect_used, panic, todo,
unimplemented, dbg_macro}` denied, `missing_docs` warned (an error under the gate's
`-D warnings`). `clippy.toml` allows unwrap/expect/panic/dbg inside `#[test]` code only.
Helpers in integration-test files are not test code: return `Result` from them.

## Build and run

```sh
cargo build --workspace
cargo run -p kiln-cli -- commands                 # names and docs
cargo run -p kiln-cli -- commands --schema        # JSON: schemas + examples (= docs/commands.json)
cargo run -p kiln-cli -- exec buffer.insert '{"offset":0,"text":"fn main() {}"}'
cargo run -p kiln-cli -- exec buffer.text '{}' --text 'let x = 1;'
```

`exec` prints the result JSON on stdout (exit 0) or a structured error
`{"code": "invalid_params" | "unknown_command" | "rejected" | "internal" | "invalid_json", …}` on
stderr (exit 2). Each invocation starts from a fresh context whose scratch buffer is `--text`.

## The gate: `cargo xtask ci`

One command, identical locally, in the queue and in GitHub Actions. Steps run in CHARTER §4
order, each prints `PASS`/`FAIL`/`EMPTY`, its time and a detail line; the gate stops at the
first failure (`--keep-going` runs all; `--report <file>` writes a JSON report, used by the
queue). Every step can also run alone: `cargo xtask <step>`.

| Step | What it checks | Cannot pass vacuously because |
|---|---|---|
| `fmt` | `cargo fmt --all -- --check` | — |
| `clippy` | `cargo clippy --workspace --all-targets --locked -- -D warnings` | — |
| `layers` | `cargo metadata` against `xtask/layers.toml`: no upward or same-layer dependency (normal, build or dev), testkit only as dev-dependency, apps not depended on, xtask depends on no workspace crate, no UI/GPU crate (`headless_forbidden`) in the normal/build closure of L0–L4, the testkit and headless apps | fails on a workspace crate missing from the table, a table entry that is not a crate, an empty workspace |
| `ui-commands` | every path in `crates/kiln-ui/**/*.rs` that names a lower crate (`kiln_text::…`) must match `xtask/ui-allowlist.toml` (whole-segment prefix) | runs a self-test first: `xtask/fixtures/ui-commands/violation.rs` must yield exactly its 6 known violations and `clean.rs` none; fails if `kiln-ui/src/lib.rs` is missing |
| `schema` | `kiln-cli commands --audit` (each example's params validate, the example runs on a fresh context and returns exactly its stated result, which validates against the result schema), then unique names, docs, both schemas, ≥1 example per command, and `docs/commands.json` byte-equal to the dump | zero commands fails |
| `scorecard` | `scorecard --check` (below) | an empty `scorecard/items` fails; a listed test that does not exist fails |
| `test` | `cargo test --workspace --locked` (unit, integration, doc tests) | zero passed tests fails |
| `no-skip` | no `#[ignore]`, `#[cfg_attr(…, ignore)]`, `skip*` call/macro or conditional `return` in a test, `cfg` other than `cfg(test)` on a test or test module, or ```` ```ignore ```` doc test — unless listed in `xtask/skips.toml` with reason and unexpired `expires` (today = HEAD's commit date) | self-test on `xtask/fixtures/no-skip/` (9 known findings, clean file 0); unused or expired allowlist entries fail |
| `fuzz` | `xtask/fuzz.toml` | EMPTY while nothing is registered; FAILS for any registered target until a runner exists |
| `budgets` | `xtask/budgets.toml` | same as `fuzz` |

EMPTY is listed in the final line (`GATE PASSED … 2 step(s) EMPTY (nothing registered): fuzz,
budgets`); it is never reported as a pass of that step.

Measured on the bootstrap machine (4 cores, 15 GB RAM, no GPU), `time cargo xtask ci`:

| Situation | Wall time |
|---|---|
| cold (fresh clone, empty `target/`, includes building xtask) | 51.3 s (gate 41.4 s: clippy 10.0 s, schema 16.0 s, scorecard 14.4 s) |
| warm, nothing changed | 1.4 s |
| in the queue, worktree seeded from main's `target/`, one crate changed | 6–11 s |

## Scorecard: `cargo xtask scorecard [--check]`

An item is `scorecard/items/<id>.toml`:

```toml
id = "text.insert"                 # equals the file stem; dotted lowercase
title = "Insert text at a char position; …"   # one line, behaviour-level
crate = "kiln-text"                # owning workspace crate
tier = "medium"                    # low | medium | high
tests = [                          # qualified names, see below
    "kiln_text::buffer::tests::insert_into_empty_buffer",
]
```

Unknown fields are errors, so a hand-written `status` is rejected. Status is derived:

- `planned` — no tests, or a listed test does not exist;
- `failing` — all exist, one fails or is ignored;
- `implemented` — all exist and pass (they are run by the scorecard step itself);
- `verified` — implemented, and the evaluation file with the highest `iteration` among
  `evaluations/*.md` that mentions the item lists it in `confirmed_items` (front matter of
  `methodology/templates/evaluation.md`) and contains a `### <item-id>` block. A later
  `rejected_items` entry withdraws it; at equal iteration, rejection wins.

`cargo xtask scorecard` writes `docs/scorecard.md`; `--check` (in the gate) fails if it is stale,
if an item is malformed, lists a missing/failing/ignored test, or an evaluation names an unknown
item or confirms one without a record block.

Test names: `<crate_ident>::<path>` for unit tests of a library (and of a binary named like its
package), `<crate_ident>::<target>::<path>` for integration tests and other binaries — e.g.
`kiln_cli::tests::exec_invalid_params_exits_2_with_structured_error` (unit test in
`apps/kiln-cli/src/main.rs`) and `kiln_cli::cli::exec_insert_through_the_binary`
(`apps/kiln-cli/tests/cli.rs`). `cargo test -p <crate> -- --list` shows the `<path>` part.

Adding an item: create the file with `tests = []` (state `planned`), run
`cargo xtask scorecard`, commit both. The test author fills `tests` (METHODOLOGY §6).

## Adding a command

1. In the command group's module in `crates/kiln-commands/src/` (one module per group, e.g.
   `buffer.rs`), define params (`#[derive(Deserialize, JsonSchema)]`,
   `#[serde(deny_unknown_fields)]`) and result (`#[derive(Serialize, JsonSchema)]`) types with
   doc comments (they become schema descriptions).
2. Implement `Command`: `NAME` (`group.verb`), `DOC`, `examples()` (≥1; each states the initial
   buffer text, params and the exact result), `run` (leave the context unchanged on error).
3. Register it in the module's `register`; a new group module is registered in
   `standard_registry()`.
4. `cargo xtask schema --write` and commit `docs/commands.json`. Command schema changes are
   high tier (CHARTER §4).

## Task worktrees

```sh
cargo xtask worktree new T-012            # .worktrees/T-012 on task/T-012 from main
cargo xtask worktree new T-012 --tests    # .worktrees/T-012-tests on task/T-012-tests
cargo xtask worktree new T-012-tests      # same as --tests (METHODOLOGY O-2)
cargo xtask worktree list                 # task worktrees, ahead/behind main
cargo xtask worktree rm T-012 [--tests] [--force]   # deletes the branch only if merged
```

`new` prints the `cd` and `export CARGO_TARGET_DIR=<worktree>/target` to use. The target dir
is seeded with a copy of main's `target/` (`--no-seed` to skip): crates.io dependencies are
reused, workspace crates rebuild because the fresh checkout is newer. Each seeded copy costs the
full size of `target/` (1.6–1.9 GB here; this filesystem has no reflinks) — remove finished
worktrees. Never point two checkouts at one target directory: cargo's mtime-based freshness
can then reuse artifacts built from the other tree (observed during bootstrap).

## Merge queue: `cargo xtask queue T-NNN`

Run from the primary checkout, with `main` checked out and clean. It: takes
`.git/kiln-queue.lock` (a lock whose process is gone is reclaimed); refuses if the working tree
has changes outside `metrics/`; rebases `task/T-NNN` onto `main` in `.worktrees/T-NNN` (conflict:
`git rebase --abort`, record `conflict`, exit 1); runs `cargo xtask ci` there with
`CARGO_TARGET_DIR=.worktrees/T-NNN/target`; fast-forwards `main` only if green; appends to
`metrics/queue.jsonl`:

```json
{"task":"T-001","attempt":1,"started_at":"…Z","ended_at":"…Z","base_sha":"…","head_sha":"…",
 "result":"merged|gate_failed|conflict|ff_failed","failed_step":null,"gate_seconds":10.0}
```

It never pushes. Refusals (lock held, dirty main, missing worktree, nothing to merge) write no
record. The gate that runs is the task branch's own (rebased) xtask: a task that edits `xtask/`
changes its own gate, which is why `xtask/` is high tier. The queue does not check tier, join
or review evidence; the director does before queueing (METHODOLOGY §4).

`metrics/` is exempt from the clean check because the queue appends there; the director commits
metrics records.

## Join: `cargo xtask join T-NNN` and `--revert-check`

Acceptance tests live in `crates/*/tests/acceptance_tNNN*.rs` or `apps/*/tests/…` on
`task/T-NNN-tests`. `join`:

1. builds and runs them in `.worktrees/T-NNN-tests` (base + tests) → `base_fail_kind`:
   `fails` (all compile and fail), `does_not_compile` (weak evidence), `mixed`, or `passes`
   (refused: the merge is undone, nothing proven);
2. runs the implementer branch's `cargo test --workspace` → `impl_tests_green`;
3. merges `task/T-NNN-tests` into `task/T-NNN` (`--no-ff`, `Task:`/`Role: director` lines);
4. runs the acceptance tests on the joined branch → `acc_fail_first_join`; if all pass, the gate;
5. appends a `join` record to `metrics/tasks.jsonl` (`--acc-defects N` records the director's
   classification; otherwise `null`). Exit 0 only for `result: "joined"`.

`cargo xtask join --revert-check T-NNN [--revert <sha>]…` reverts the implementation commits
(default: non-merge commits on `task/T-NNN` not on `main` or `task/T-NNN-tests`) in a temporary
worktree with a private seeded target, keeps the tests, and records a `revert_check` record with
`result_kind`. Exit 0 only for `fails`. When the tests call a new API, revert only the logic
commits (`--revert <sha>`) so the API stub remains and the tests compile — split such work into
a stub commit and a logic commit.

Note: the queue's rebase flattens the join merge, so after queueing `task/T-NNN-tests` no longer
looks merged (`worktree list` shows it ahead); remove it with `worktree rm --tests --force`.

## Metrics: `cargo xtask metrics`

Field names are the contract of METHODOLOGY §13. Timestamps are always passed in
(`YYYY-MM-DDTHH:MM:SSZ` or with an offset; stored as UTC `Z`).

```sh
cargo xtask metrics add --task T-012 --run R-001 --role implementer --tokens 12000 \
  --tool-calls 9 --gate-attempts 1 --outcome completed \
  --started-at 2026-11-12T22:00:00Z --ended-at 2026-11-12T22:20:00Z \
  [--tier medium] [--iteration 0] [--wall-s N] [--missing-context "…"] [--note "…"]
cargo xtask metrics transition --task T-012 --from specified --to ready --at 2026-11-12T21:50:00Z
cargo xtask metrics report
```

`--tier` and `--iteration` default from `tasks/T-NNN-*.md` front matter; `wall_s` defaults to
`ended_at − started_at`. Aliases: `--seconds` = `--wall-s`, `--tool-uses` = `--tool-calls`.
Roles: director, planner, implementer, test-author, reviewer, integrator, evaluator,
retrospective-analyst. `report` prints tokens by tier, first-pass queue success, tokens per
merged task by tier, lead time (first `→ ready` transition to the first `merged` queue record),
gate attempts per task, first-join base evidence, acceptance-caught defects and budget overruns.

## Other registries

- `xtask/layers.toml` — the layer table (a CHARTER §3 contract: change by ADR).
- `xtask/ui-allowlist.toml` — paths kiln-ui may name; each entry needs a reason.
- `xtask/skips.toml` — allowed skips: `file`, `item` (module path + name), optional `kind`,
  `reason`, `expires`.
- `xtask/fuzz.toml`, `xtask/budgets.toml` — registered targets (none yet).

## CI

`.github/workflows/ci.yml` runs `cargo xtask ci` on every push to `main` (and on manual
dispatch), Linux, `permissions: contents: read`, actions pinned by commit sha
(`actions/checkout` v7.0.1, `Swatinem/rust-cache` v2.9.3). There is no `concurrency` group,
so no run on `main` is cancelled or replaced.
