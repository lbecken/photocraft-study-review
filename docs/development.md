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
| tar/cp | not used | target seeding is done by xtask itself (plain file copies, mtimes kept) |

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

`new` prints the `cd` and `export CARGO_TARGET_DIR=<worktree>/target` to use. Never point two
checkouts at one target directory: cargo's mtime-based freshness can then reuse artifacts built
from the other tree (bootstrap-verification §7 defect 4).

**Seeding.** Unless `--no-seed` (which creates an empty `target/`), the new target directory is
seeded from main's `target/` with **copies** (never hard links, symlinks or a shared directory)
of the build artifacts of packages that are *not* workspace members:

- members are the package and target names `cargo metadata --no-deps` reports for main's
  checkout and the new one (`-` and `_` treated alike);
- in each profile directory (`target/debug`, `target/<triple>/debug`, …) only entries of
  `.fingerprint/`, `build/` and `deps/` named `<unit>-<16 hex hash>…` are considered, and an
  entry is copied only if `<unit>` is not a member; uplifted files (`target/debug/kiln-cli`, …),
  `incremental/`, `examples/` and anything unrecognised are never copied;
- mtimes are kept. A non-member whose sources live in the checkout (a path dependency outside
  the workspace) gets main's mtimes on its source files in the new checkout only where the
  content is byte-identical, so cargo judges it fresh exactly when it would in main's checkout;
  otherwise it is rebuilt.

So crates.io dependencies are reused, every workspace crate is rebuilt from the worktree's own
source, and a build in the worktree never writes into main's `target/` (or the reverse). A seed
costs roughly main's dependency artifacts (`du -sh .worktrees/T-NNN/target`), not the whole
`target/` — no workspace-crate files and no `incremental/` data. `worktree new` prints the
number of files and MB copied. Remove finished worktrees.

### Stale-binary guard

Cargo names a workspace crate's artifacts by its path relative to the workspace root, so two
checkouts sharing a target directory get the same artifact names, and mtime-based freshness can
serve one checkout's `xtask` in the other (T-005 join triage, D-019). Every `xtask` invocation
therefore first resolves the git working tree it was built from (the workspace above its
compile-time `CARGO_MANIFEST_DIR`) and the one it is run in (the cargo workspace above the
working directory), and refuses — exit 2, one line `xtask: stale xtask binary: built from <A>
but run in <B> …` — when they are **different working trees of the same repository** (same
common git directory), e.g. main's build served by `cargo xtask` in `.worktrees/T-NNN`. In its
own checkout it runs; in an unrelated repository (the toy repositories tests create) it runs; if
either side is not a git working tree it runs. The queue's run of main's xtask against a task
worktree needs no exception: main's build tree `.git/kiln-queue/main-xtask/src` is an export
inside the git directory, not a working tree, so the guard lets it run. No environment variable
or flag relaxes the guard (D-026 R8). Fix a refusal by rebuilding in the right checkout
(`touch xtask/src/main.rs` or `cargo clean -p xtask`); never export one checkout's
`CARGO_TARGET_DIR` while working in another.

## Merge queue: `cargo xtask queue T-NNN`

Run from the primary checkout, with `main` checked out and clean. In order:

1. takes `.git/kiln-queue.lock` (a lock whose process is gone is reclaimed); refuses (error,
   exit 2, no record) if `main` is not checked out, the working tree has changes outside
   `metrics/`, `.worktrees/T-NNN` is missing, dirty or not on `task/T-NNN`, or the branch has no
   commits beyond `main`;
2. runs the **preconditions** below; the first that fails is a refusal: exit 1, one stderr line
   `queue: refused: <check>: <reason>`, nothing changed and no record written;
3. builds main's xtask from `base_sha` (= `main`) in a private build tree
   `.git/kiln-queue/main-xtask/` (`src/` is a fresh export of `base_sha` through a private git
   index — not a worktree, never a task's tree — and `target/` is used for nothing else; a binary
   already built from the same sha is reused). If that build fails the queue stops with an
   error and writes no record (main itself is broken: an incident);
4. rebases `task/T-NNN` onto `main` in `.worktrees/T-NNN` (generated-file conflicts: below;
   any other conflict: `git rebase --abort`, record `conflict`, `failed_step: "rebase"`);
5. runs **main's gate**: main's xtask binary `ci --report <file>` with the rebased worktree as
   working directory and `CARGO_TARGET_DIR=.worktrees/T-NNN/target`. If the task changes a gate
   path (anything under `xtask/` or `.cargo/`, `clippy.toml`, `rust-toolchain.toml`), the task's
   own rebased gate (`cargo run -p xtask -- ci --report …`) runs afterwards too; both must pass,
   there is no override (D-3);
6. fast-forwards `main` only if green (never pushes) and appends to `metrics/queue.jsonl`:

```json
{"task":"T-001","attempt":1,"started_at":"…Z","ended_at":"…Z","base_sha":"…","head_sha":"…",
 "result":"merged|gate_failed|conflict|ff_failed","failed_step":null,"gate_seconds":10.0,
 "gate_xtask":"main|main+task","gate_xtask_sha":"<base_sha>","regenerated":[]}
```

Under `"main+task"` a `gate_failed` record's `failed_step` is prefixed `main:` or `task:`.
The queue works on any repository with an `xtask` package whose `ci --report <file>` writes
JSON `{"passed": bool, "failed_step": string|null, …}`.

### Preconditions (in this order; all read before any rebase)

| check | passes when |
|---|---|
| `task-file` | exactly one `tasks/T-NNN-*.md` in **main's** tree; its front matter parses with `id: T-NNN`, `tier` low/medium/high and a `touches` list |
| `front-matter` | the task branch did not change that file's front matter: its copy at the task head has front matter identical to main's, or identical to the copy at the merge base (main's `state` moves after the branch is made); deleting the file is refused. Builders append to the log body only |
| `state` | main's `state` is `queued`, or `reviewed` (medium/high), or `self_verified` (low) |
| `config` | main's `xtask/generated.toml` and `xtask/ledger.toml` exist and are valid (below) |
| `touches` | every path of `git diff --name-only --no-renames <merge-base> <task head>` is allowed (below) |
| `join` | medium/high: the primary checkout's `metrics/tasks.jsonl` has a `kind:"join"`, `result:"joined"` record of the task whose `joined_sha` is the task head or an ancestor of it. Read only from this record — the rebase flattens the join merge (D-6) |
| `review` | medium/high: on the task head, `reviews/T-NNN-r<N>.md` with the highest N has `verdict: approve`, a `reviewed_sha` that is the head or an ancestor of it, and every commit after `reviewed_sha` changes only `reviews/T-NNN-*` and the task file |
| `approval` | high: main's `docs/decisions.md` has a table row whose Decision cell begins with `Approve T-NNN` (D-2) |
| `main-history` | every commit on main's first-parent history after the newest `head_sha` of a `merged` record in `metrics/queue.jsonl` changes only ledger paths, respecting their mode; trivially true before the first merge |

**Allowed paths** (`touches` check): `p` is allowed if `p == t` or `p` starts with `t/` for an
entry `t` of main's `touches` (a trailing `/` on `t` is ignored; `crates/a` allows
`crates/a/src/x.rs`, not `crates/ab/x.rs`); or `p` is the task file; or `p` is a generated path;
or, for medium/high only, `p` is an `acceptance_tNNN*.rs` or `guard_tNNN*.rs` file directly in a
`crates/*/tests/`, `apps/*/tests/` or `xtask/tests/` directory, `scorecard/items/<id>.toml` for an `<id>` in the
task's `scorecard_items`, or starts with `reviews/T-NNN-`. Deletions count as changes.

### Policy files (read from main's tree, never from the task branch)

- `xtask/generated.toml` — `[[generated]]` entries `path` + `command` (argv; `{xtask}` = main's
  built xtask), in order: `Cargo.lock` ← `cargo update --workspace`, `docs/commands.json` ←
  `{xtask} schema --write`, `docs/scorecard.md` ← `{xtask} scorecard` (D-1).
- `xtask/ledger.toml` — `[[ledger]]` entries `path` + `mode` (`any` or `add-only`): the paths
  the director may commit to on `main` directly (D-4): `tasks/`, `metrics/`,
  `docs/decisions.md`, `docs/adr/`, `STATE.md`, `handoffs/`, `retrospectives/`, `evaluations/`,
  `methodology/` (`any`; "only at a retrospective" stays a director check) and
  `scorecard/items/` (`add-only`: new files only). Code, `xtask/`, `.github/` and `Cargo.*`
  reach `main` only through the queue.

Both are parsed strictly: unknown keys, an empty list, an empty command, a malformed or
duplicate path, a path in both files, or a ledger entry under `xtask/`, `.github/` or `Cargo.*`
are errors (refusal `config`). Changing either file is an `xtask/` change, so it goes through
the queue like code and takes effect from the next run after it is merged.

### Generated-file conflicts

Tasks still regenerate generated files locally so their own gate passes. When the rebase stops
on a conflict and **every** conflicted path is a generated path, the queue takes main's side for
those paths (`git checkout HEAD -- <path>`, or `git rm` if main's side deleted it), continues the
rebase and repeats for later commits. After the rebase it runs every entry's command in file
order in the worktree (`CARGO_TARGET_DIR=.worktrees/T-NNN/target`); for `Cargo.lock` it first
restores main's (`base_sha`) lock file and afterwards fails if a registry or git package of
main's lock changed version (path packages are not compared). If any generated file changed it
commits them as one commit (`queue: regenerate generated files`, `Task: T-NNN`,
`Role: director`); the gate then runs on the result. `regenerated` lists the generated paths
that were conflicted or changed. A failing command, a version change in the lock, or a command
that changes a non-generated path gives `conflict` with `failed_step: "regenerate:<path>"`; the
task branch is reset to its pre-queue head and `main` is unchanged. Regeneration runs only after
such a conflict; a rebase without conflicts keeps the task's generated files as committed (its
gate checks them).

`metrics/` is exempt from the clean check because the queue appends there; the director commits
metrics records (a ledger path).

This queue was introduced by T-005. T-005 itself was the last task queued by the old queue,
which ran the task branch's own gate and checked none of the above.

## Join: `cargo xtask join T-NNN` and `--revert-check`

Acceptance tests live in `crates/*/tests/acceptance_tNNN*.rs`, `apps/*/tests/…` or
`xtask/tests/…` (directly in that `tests/` directory) on `task/T-NNN-tests`. Regression guards
(`guard_tNNN*.rs`, D-010) live in the same places; they are meant to pass on base, so join does
not count them as acceptance evidence, but it runs them itself on base and on the joined
branch: every guard must pass in both places, otherwise the record's `result` is
`guard_failed` (exit 1, the merge is kept, the gate is not run). The record carries
`guard_total`, `guard_fail_base` and `guard_fail_join` (D-021). `join`:

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
worktree `.worktrees/T-NNN-revert-check` with a private target seeded from the task worktree's
target exactly like `worktree new` (copies of non-member artifacts only; the task's target is
only read, so a later build there is never served the reverted tree), keeps the tests, and records a `revert_check` record with
`result_kind`. Exit 0 only for `fails`. When the tests call a new API, revert only the logic
commits (`--revert <sha>`) so the API stub remains and the tests compile — split such work into
a stub commit and a logic commit.

Note: the queue's rebase flattens the join merge, so after queueing `task/T-NNN-tests` no longer
looks merged (`worktree list` shows it ahead); remove it with `worktree rm --tests --force`. For
the same reason the queue's `join` check reads the `join` record, not history.

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
