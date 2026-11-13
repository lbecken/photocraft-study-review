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
| `hygiene` | two text checks (ADR-0003). `reviews`: every `reviews/T-NNN-rN.md` whose task file has tier medium/high and `iteration ≥ 1` has front matter `task`/`round` matching the file name, `tier` and `arm` matching the task (arm: `independent` for high and even-numbered medium, `implementer` for odd-numbered medium), and `mutants` (flow map `{…}` as in the template, or an indented block) with the five non-negative integers `introduced`, `equivalent`, `killed`, `killed_by_acceptance`, `survived`, `equivalent + killed + survived = introduced`, `killed_by_acceptance ≤ killed`, and `introduced ≥ 10` in round 1. `session-paths`: no file listed by `git ls-files` contains a match of `/tmp/claude-[0-9]+/[^/[:space:]]+/<UUID>` (non-UTF-8 files skipped); a match is reported as `<path>:<line>`, never the text. Each FAIL line names its check | never EMPTY: `session-paths` always runs; a review without a task file fails; no allowlist |
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
4. `cargo xtask schema --write` and commit `docs/commands.json`. Adding a command is medium
   tier unless a boundary of ADR-0002 applies; changing how the registry, schema validation or
   the command log work is a CHARTER §3 contract change, tier high (ADR-0002).

## Task worktrees

```sh
cargo xtask worktree new T-012            # .worktrees/T-012 on task/T-012 from main
cargo xtask worktree new T-012 --tests    # .worktrees/T-012-tests on task/T-012-tests
cargo xtask worktree new T-012-tests      # same as --tests (METHODOLOGY §3)
cargo xtask worktree list                 # task worktrees, ahead/behind main
cargo xtask worktree rm T-012 [--tests] [--force]   # deletes the branch only if merged
```

`new` first reads the task's state from **main's committed** `tasks/T-NNN-*.md` (never the
working copy) and refuses — exit 1, one stderr line, nothing created — unless it is `ready`
(METHODOLOGY §5.1): `worktree: refused: T-NNN is <state> on main; worktree new needs ready` or
`worktree: refused: no tasks/T-NNN-*.md on main`. The same holds for `--tests` and
`T-NNN-tests`. A main without any `tasks/` directory (repositories that do not track tasks) is
not checked.

`new` prints the `cd` line; cargo then uses `<worktree>/target` by itself. Never set or export
`CARGO_TARGET_DIR` (AGENTS.md, D-019): pointing two checkouts at one target directory lets
cargo's mtime-based freshness reuse artifacts built from the other tree (bootstrap-verification
§7 defect 4).

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
   exit 2, no record) if `main` is not checked out, the working tree has (non-evidence) changes outside
   `metrics/` (the `ledger` refusal of step 2 is checked first, right after `main`), `.worktrees/T-NNN` is missing, dirty or not on `task/T-NNN`, or the branch has no
   commits beyond `main`;
2. refuses at `ledger` (exit 1, one line, nothing changed) if the primary checkout has
   uncommitted changes — modified, staged or untracked — to an evidence file: anything under
   `metrics/` (no exemption: the queue commits its own `metrics/queue.jsonl`, D-030 R12),
   `docs/decisions.md`, `tasks/`, `reviews/`, `xtask/generated.toml` or `xtask/ledger.toml`.
   Evidence is read only from main's committed tree, so commit the ledger on `main` first
   (D-026 R6, D-027);
3. runs the **preconditions** below (`budget` sits between `approval` and `main-history`); the first that fails is a refusal: exit 1, one stderr line
   `queue: refused: <check>: <reason>`, nothing changed and no record written;
4. builds main's xtask from `base_sha` (= `main`) in a private build tree
   `.git/kiln-queue/main-xtask/` (`src/` is a fresh export of `base_sha` through a private git
   index — not a worktree, never a task's tree — and `target/` is used for nothing else). Every
   run rebuilds the xtask crate from that fresh export: before `cargo build` the queue removes
   from the private `target/` every artifact of a workspace member of `base_sha`, all uplifted
   binaries, `incremental/` data and unknown files, so a binary left by an earlier run (or
   overwritten by task code during an earlier gate) is never trusted (D-026 R1). Compiled
   registry dependencies are kept and may be reused — an accepted residual risk: task code that
   rewrites a dependency's artifact in that directory is not detected. If the build fails the
   queue stops with an error and writes no record (main itself is broken: an incident);
5. rebases `task/T-NNN` onto `main` in `.worktrees/T-NNN` (generated-file conflicts: below;
   any other conflict: `git rebase --abort`, record `conflict`, `failed_step: "rebase"`);
6. runs **main's gate**: main's xtask binary `ci --report <file>` with the rebased worktree as
   working directory and `CARGO_TARGET_DIR=.worktrees/T-NNN/target`. If the task changes a gate
   path (below; the queue prints them: a stderr line `queue: T-NNN changes gate paths: …` and
   `gate paths changed: …` in the summary), the task's own rebased gate
   (`cargo run -p xtask -- ci --report …`) runs afterwards too; both must pass, there is no
   override (D-3). Before the gate, the lock rule is applied once more to the rebased tree: if
   its `Cargo.lock` changes the version of a package of main's lock, the run is refused at
   `dependencies` and the branch is put back;
7. fast-forwards `main` only if green (never pushes). On every other outcome — `gate_failed`,
   `ff_failed`, `conflict`, a refusal after the rebase, or an error — `task/T-NNN` is put back
   on the commit it pointed at before the run (`git rebase --abort`, `git reset --hard`,
   `git clean -d --force`; ignored files such as `target/` stay) and its worktree is clean, so
   the join and review evidence still names ancestors of the head and a re-queue needs no
   re-join or re-review (D-026 R5). Then it appends to `metrics/queue.jsonl` and **commits that
   record on main** (D-030 R12): one commit touching only `metrics/queue.jsonl`, subject
   `queue: record T-NNN <result>` (`Task:`/`Role: director` lines). After a merge it sits on top
   of the fast-forwarded head, so main is then `head_sha` plus the record commit; after any
   other outcome it is the only change to main (the task branch is still put back, R5). A
   refusal writes and commits nothing. The record commit is a ledger commit (`metrics/` is a
   ledger path), so the next `main-history` check tolerates it:

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
| `join` | medium/high: main's committed `metrics/tasks.jsonl` (never the working copy) has a `kind:"join"`, `result:"joined"` record of the task whose `joined_sha` is the task head or an ancestor of it. Read only from this record — the rebase flattens the join merge (D-6) |
| `review` | medium/high: on the task head, `reviews/T-NNN-r<N>.md` with the highest N has `verdict: approve` and a `reviewed_sha` that is a full 40-hex lowercase commit id (no branch name, `HEAD`, abbreviation or other revision expression) naming a commit that is the head or an ancestor of it; every commit after `reviewed_sha` changes only `reviews/T-NNN-*`, the task file, generated paths and the task's acceptance/guard test files (allowed-path rule below); anything else is a stale review (D-026 R2) |
| `gate-path` | a task whose changed paths include a **gate path** is tier high (D-026 R3). Gate paths: anything under `xtask/` or `.github/`; any path with a `.cargo` directory segment; any file named `build.rs`, `rust-toolchain.toml`, `rust-toolchain`, `clippy.toml`, `.clippy.toml`, `rustfmt.toml` or `.rustfmt.toml` (at any depth); any `Cargo.toml` whose `[lints]` table (including `lints.workspace`) or package `build` key the task adds, removes or changes; any file that a package `build` key names in main's tree or at the task head (a build script need not be called `build.rs`, D-030 R10); and the root `Cargo.toml` when the task changes its `[workspace.lints]`, `[workspace.metadata]` or any `[profile.*]` table. "The task changes" a value relative to main when the head's value differs from main's **and** from the merge base's, so a value main changed after the branch was made is not blamed on the task (the rebase brings main's); an unparsable manifest at the head always counts |
| `dependencies` | a task whose changed paths include a `Cargo.lock` or a `Cargo.toml` whose dependency tables differ between the merge base and the head (`[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`, their `[target.*]` forms, `[workspace.dependencies]`, `[patch.*]`, `[replace]`; a new manifest with any of them counts) is tier high; and for every changed `Cargo.lock`, no registry or git package of main's lock has a version at the task head that the task's own diff changed (head differs from main's and from the merge base's; path packages are not compared, D-027). A version that is only stale because main moved is replaced by the rebase or regeneration, and the rule runs again on the rebased tree before the gate (D-026 R4). A version change is allowed only for a tier-high task whose approval row (as for `approval`) contains the words `dependency versions` (D-030 R11); then it merges. Regeneration of a conflicted `Cargo.lock` never changes such versions (`regenerate:Cargo.lock`) |
| `approval` | high: main's `docs/decisions.md` has a table row whose Decision cell begins with `Approve T-NNN` (D-2) followed by the cell's end or a character that cannot continue an id (so `Approve T-005-tests` does not approve T-005); for a task that changes gate paths, that same cell must also contain the words `gate paths` (D-026 R3). Only real Markdown table rows count: a header row with a `Decision` cell, then a delimiter row, then rows starting with `|` after at most three spaces — never lines in fenced code blocks, indented code, block quotes or HTML comments (D-026 R7) |
| `budget` | the sum `S` of known `tokens` over the task's `kind:"run"` records in main's **committed** `metrics/tasks.jsonl` is at most the task's ceiling: the `N` of the last (in table order) decision-table row of main's `docs/decisions.md` whose Decision cell begins with `Extend T-NNN to N` (decimal digits, then the cell's end or a character that is neither a digit nor an id character; rows recognised as for `approval`; `Extend T-NNN` without `to <digits>` is not an extension), else main's `budget_tokens` (a task file without it fails). Refusal: `queue: refused: budget: T-NNN has <S> recorded tokens, over its ceiling <ceiling> (<budget_tokens\|Extend row>); a docs/decisions.md row "Extend T-NNN to <N>" with N ≥ <S> is needed`. Runs with `tokens: null` count 0 and the summary line says how many (METHODOLOGY §11) |
| `main-history` | every commit on main's first-parent history after the newest `head_sha` of a `merged` record in main's **committed** `metrics/queue.jsonl` (D-030 R12) changes only ledger paths, respecting their mode; trivially true before the first merge |

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

`metrics/queue.jsonl` is written and committed by the queue itself (above); every other
evidence file must be committed on `main` by the director before a queue run (`ledger` above).
If committing the record fails (e.g. no git identity can be found), the queue reports an error
and the uncommitted record makes the next run stop at `ledger` until it is committed by hand.

### Accepted residual risks

The queue defends against honest mistakes and corner-cutting, not a malicious agent (D-026).
These risks are accepted (D-030) and not defended against:

- cached dependency build artifacts can be tampered with;
- gated code runs with the director's privileges;
- acceptance tests edited after a review are not re-reviewed;
- an approval row is not tied to a commit.

This queue was introduced by T-005. T-005 itself was the last task queued by the old queue,
which ran the task branch's own gate and checked none of the above.

## Join: `cargo xtask join T-NNN` (and the optional `--revert-check`)

Acceptance tests live in `crates/*/tests/acceptance_tNNN*.rs`, `apps/*/tests/…` or
`xtask/tests/…` (directly in that `tests/` directory) on `task/T-NNN-tests`. Regression guards
(`guard_tNNN*.rs`, D-010) live in the same places; they are meant to pass on base, so join does
not count them as acceptance evidence, but it runs them itself on base and on the joined
branch: every guard must pass in both places, otherwise the record's `result` is
`guard_failed` (exit 1, the merge is kept, the gate is not run). The record carries
`guard_total`, `guard_fail_base` and `guard_fail_join` (D-021). `join`:

1. builds and runs them in `.worktrees/T-NNN-tests` (base + tests) → `base_fail_kind`:
   `fails` (all compile and fail), `does_not_compile` (weak evidence: join prints
   ``join: base evidence is `<kind>` (weak); the reviewer's mutation spot-check must cover it
   (METHODOLOGY §6)``), `mixed`, or `passes` (refused: the merge is undone, nothing proven);
2. runs the implementer branch's `cargo test --workspace` → `impl_tests_green`;
3. merges `task/T-NNN-tests` into `task/T-NNN` (`--no-ff`, `Task:`/`Role: director` lines);
4. runs the acceptance tests on the joined branch → `acc_fail_first_join`; if all pass, the gate;
5. appends a `join` record to `metrics/tasks.jsonl` (`--acc-defects N` records the director's
   classification; otherwise `null`, classified later with `metrics join-defects`). Exit 0
   only for `result: "joined"`.

`cargo xtask join --revert-check T-NNN [--revert <sha>]…` reverts the implementation commits
(default: non-merge commits on `task/T-NNN` not on `main` or `task/T-NNN-tests`) in a temporary
worktree `.worktrees/T-NNN-revert-check` with a private target seeded from the task worktree's
target exactly like `worktree new` (copies of non-member artifacts only; the task's target is
only read, so a later build there is never served the reverted tree), keeps the tests, and records a `revert_check` record with
`result_kind`. Exit 0 only for `fails`. It is an optional diagnostic: the method does not
require a revert check or a stub/logic commit split (v1.1); weak base evidence is covered by the
reviewer's mutation spot-check (METHODOLOGY §6). `--revert <sha>` limits the revert to the
given commits.

Note: the queue's rebase flattens the join merge, so after queueing `task/T-NNN-tests` no longer
looks merged (`worktree list` shows it ahead); remove it with `worktree rm --tests --force`. For
the same reason the queue's `join` check reads the `join` record, not history.

## Metrics: `cargo xtask metrics`

Field names are the contract of METHODOLOGY §13. `<now>` is the system clock (UTC, seconds);
there is no option or variable that replaces it or relaxes a refusal (D-026 R8). Refusals are
one stderr line `metrics: refused: …`, exit 1, nothing appended; usage errors (unknown option,
missing value, a value that is not a number or a timestamp) exit 2.

```sh
cargo xtask metrics add --task T-012 --run R-001 --role implementer --tokens 12000 \
  --tool-calls 9 --gate-attempts 1 --outcome completed --started-at 2026-11-12T22:00:00Z \
  [--ended-at <ts>] [--tier medium] [--iteration 1] [--wall-s N] [--missing-context "…"] [--note "…"]
cargo xtask metrics add --task iter-1 --run R-002 --role evaluator --tokens-unknown ...
cargo xtask metrics transition --task T-012 --to ready [--from specified] [--at <ts>]
cargo xtask metrics join-defects --task T-012 --joined-sha <40-hex> --acc-defects 1
cargo xtask metrics report
```

Rules, checked in this order (first failure wins): options → task id and task file / `iter-N`
role rules → tokens → time → append.

- **Task.** `T-NNN` needs exactly one `tasks/T-NNN-*.md` in the working tree (tier, iteration
  and `budget_tokens` come from it; `--tier`/`--iteration` override). `iter-N` (N without
  leading zeros) books iteration N's overhead: `task:"iter-N"`, `iteration: N` (`--iteration`,
  if given, must equal N), `tier: null` (`--tier` is refused). Only `director`, `planner`,
  `evaluator` and `retrospective-analyst` runs go to `iter-N`, and those roles never go to a
  `T-NNN`. `transition` and `join-defects` refuse `iter-N`.
- **Tokens.** Exactly one of `--tokens N` (N ≥ 1) or `--tokens-unknown` (`"tokens": null`).
- **Time.** `--ended-at` (add) and `--at` (transition) default to `<now>`. A record's time is
  `ended_at` for a run, `at` for any other record. Refused: a time later than `<now>`, a
  `started_at` later than `<now>` or than `ended_at`, and a time earlier than the latest time
  among the task's records in `metrics/tasks.jsonl` (equal is allowed).
- **Transitions.** The current state is the `to` of the task's latest transition (`draft` if
  none); `--from`, if given, must equal it. Allowed (METHODOLOGY §3): draft → specified;
  specified → ready, draft; ready → in_progress; in_progress → self_verified; self_verified →
  queued (low) or tests_joined, in_progress (medium/high); tests_joined → reviewed, in_progress
  (medium/high); reviewed → queued (medium/high); queued → merged, in_progress; merged →
  evaluated; every state but merged, evaluated and abandoned → blocked, abandoned; blocked → the
  state it was entered from, or abandoned; evaluated and abandoned are terminal.
- **Budget line.** After `add` for a task, stdout ends with `T-NNN: <sum> tokens of budget
  <budget_tokens>` (sum of known tokens of all its runs), then, if the sum exceeds the ceiling
  (`budget_tokens`, or the task's last `Extend T-NNN to <N>` row in the working tree's
  `docs/decisions.md`), `T-NNN: over its ceiling <ceiling> by <d> tokens; the queue refuses it
  without a docs/decisions.md row "Extend T-NNN to <N>" with N ≥ <sum>`, then, if some run has
  unknown tokens, `T-NNN: <k> run(s) with unknown tokens not counted`. For `iter-N`:
  `iter-N: <sum> tokens of overhead`.
- **`join-defects`** appends `{"kind":"join_defects","task":…,"at":"<now>","joined_sha":…,
  "acc_defects":N}`, the director's classification of a failing join; no record is ever edited
  (T-013 D-3). Refused: no `join` record of the task with that `joined_sha`, more than one, its
  own `acc_defects` already set, an earlier `join_defects` record for it, `acc_fail_first_join`
  0, or N above it. A join's **classification** is its own `acc_defects`, else that record.
  (The new record kind joins METHODOLOGY §13's list at retrospective 1.)

`wall_s` defaults to `ended_at − started_at`. Aliases: `--seconds` = `--wall-s`, `--tool-uses`
= `--tool-calls`. `report` prints tokens by tier (`iter-N` runs as tier `overhead`, never in a
task's total), first-pass queue success, tokens per merged task (merged = a `merged` record in
`metrics/queue.jsonl`) by tier and by arm without overhead and per iteration with `iter-N`
overhead (unknown-token runs excluded and counted on the line), lead time, gate attempts per
task, first-join base evidence, acceptance-caught defects (Σ classification over **every** join
with `impl_tests_green: true`, by tier and arm), one `unclassified join: …` line per failing
join of an iteration-≥1 task without a classification (the report is printed in full and exits
1), known defects merged by decision (`found_by: reviewer` tasks whose `origin_task` is merged,
as `T-NNN (from T-MMM)`), and budget overruns with `extend to: <N>|none` and
`within ceiling: yes|no`.

## Publishing a ledger input: `cargo xtask publish`

```sh
cargo xtask publish T-NNN --title "<title>" --iteration N <path>...
```

An evaluation file or a new scorecard item is a ledger path, but it changes the generated
`docs/scorecard.md`, which reaches `main` only through the queue (D-034, T-012). `publish` does
the whole low-tier publish task in one command, from the primary checkout with `main` checked
out. Inputs: new `scorecard/items/<id>.toml` files (absent from main) and new or modified
`evaluations/*.md` files of the working tree. Publish takes the queue's lock
(`<git-common-dir>/kiln-queue.lock`) before step 1 and holds it through step 7, so it never runs
beside a queue or another publish; a lock held by a live process is refused (`publish: refused:
another queue or publish run holds <lock path>`, exit 1, nothing changed). The lock is released
just before step 8, where the queue takes it itself; a queue that starts in that gap makes step
8 fail with its usual line, and nothing is damaged. Steps (each failure stops):

1. checks, nothing changed (`publish: refused: <reason>`, exit 1): `main` checked out; a well
   formed `T-NNN` with no task file on main or in the working tree, no `task/T-NNN` branch, no
   `.worktrees/T-NNN` and no record of `T-NNN` in `metrics/tasks.jsonl` (`publish: refused:
   T-NNN already has records in metrics/tasks.jsonl`: a number publish or anything else has used
   is never reused); a title with a letter or digit (`publish: refused: the title has no letter
   or digit`); at least one input, each of an allowed kind and different from main;
   apart from the inputs, the queue's working-tree rules (`ledger`, no other dirty path); the
   command main's `xtask/generated.toml` lists for `docs/scorecard.md` (`{xtask}` = main's
   xtask, built as the queue builds it), run in a private export of main plus the inputs
   (`.git/kiln-publish/`), must succeed (else its own error) and must change
   `docs/scorecard.md` (else `publish: refused: the inputs change no generated file; commit them
   on main as a ledger commit`). The primary checkout's `docs/scorecard.md` is not written;
2. writes `tasks/T-NNN-publish-<slug>.md` (tier low, `state: ready`, `owner_role: director`,
   `touches` = the inputs, `budget_tokens: 150000`, the three ACs of T-012), records draft →
   specified → ready and commits the file and `metrics/tasks.jsonl` on main (`publish: T-NNN
   ready`, `Task:`/`Role: director`, the repository's commit configuration, signing included).
   If the write or a transition fails before the commit, the task file is removed and
   `metrics/tasks.jsonl` restored, so main is as it was found (the failure's own message and
   exit code). A failed commit, a failing `git add` included (in steps 2, 6 and 8 alike): exit 2
   and one line `publish: commit failed: <git error>; commit by hand: git commit …`;
3. creates the worktree as `worktree new` does; ready → in_progress and `state: in_progress`,
   committed on main at once (`publish: T-NNN in progress`, the task file and
   `metrics/tasks.jsonl`), so main's ledger is committed wherever the command stops and the
   task file's `state:` equals the last recorded transition;
4. copies the inputs into it, regenerates `docs/scorecard.md` there with that same command,
   and commits exactly those paths on `task/T-NNN` (anything else: stop, exit 1);
5. runs the gate there; red: exit 1 and `publish: gate failed in .worktrees/T-NNN at <step>; …`
   (the task stays in_progress, the inputs stay in the primary checkout);
6. in_progress → self_verified → queued, `state: queued`, commit `publish: T-NNN queued`.
   Main's history is append-only (D-040): no tool resets, amends or rewrites a commit on it, so
   a successful publish adds six first-parent commits — `ready`, `in progress`, `queued`, the
   content commit, the queue's record commit and `merged`; after a red gate `ready` and
   `in progress` stay;
7. compares every input with its branch copy, then removes each from the primary checkout (new
   files deleted, modified files restored), printing `publish: <path> moved to task/T-NNN`; if
   any input differs, none is removed;
8. runs `cargo xtask queue T-NNN` (output and exit code passed through). Merged: queued →
   merged, `state: merged`, commit `publish: T-NNN merged`, `worktree rm T-NNN`. Otherwise:
   `publish: queue did not merge T-NNN (exit c); …` and exit `c`.

A stop in step 3, 4 or 7 that has no line of its own prints `publish: stopped at step <n>:
<error>; T-NNN is <state>` (exit 1; 2 for a failed content commit in step 4).

The helper records no `run`. The director still reserves the task number and title, has the
input written by its author, books the session's own run to `iter-N`, acts on a stop message
(re-run the gate, re-queue, or abandon), and files any defect tasks as separate ledger commits.

### After a stop

Publish is never resumed and a task number it has used is not reused: the director finishes or
abandons the task by hand with the ordinary commands. Below, *ledger commit "<s>"* means
`git add -- tasks/T-NNN-<slug>.md metrics/tasks.jsonl` and then
`git commit -m "<s>" -m "Task: T-NNN" -m "Role: director" -- tasks/T-NNN-<slug>.md metrics/tasks.jsonl`
on main; *set state X* means editing the task file's front-matter line to `state: X`.

| Stopped at | Left behind on main | Finish by hand | Abandon |
|---|---|---|---|
| 1 (refused) | nothing | fix the cause, re-run `publish` | — |
| 2, before the commit | nothing (task file removed, ledger restored) | fix the cause, re-run `publish` with the same number | — |
| 2, commit failed | task file and two transitions (`ready`) uncommitted | repair (signing, index lock), ledger commit "publish: T-NNN ready"; then the row for 3 | `metrics transition --task T-NNN --to abandoned`, set state abandoned, ledger commit "publish: T-NNN abandoned" |
| 3 | `ready` committed; maybe a worktree | `worktree new T-NNN` if missing; `metrics transition --task T-NNN --to in_progress`; set state in_progress; ledger commit "publish: T-NNN in progress"; then the row for 4 | as for 2, then `worktree rm T-NNN --force` if a worktree exists |
| 4 | `in_progress` committed; worktree | in `.worktrees/T-NNN`: copy the inputs, run main's `docs/scorecard.md` command (`cargo xtask scorecard`), commit exactly the inputs and `docs/scorecard.md` (`Task: T-NNN`, `Role: director`); then the row for 5 | `metrics transition --task T-NNN --to abandoned`, set state abandoned, ledger commit "publish: T-NNN abandoned", `worktree rm T-NNN --force` |
| 5 (gate red) | `in_progress` committed; content commit on `task/T-NNN` | fix the input (in the worktree and the primary checkout alike), commit it on the branch, `cargo xtask ci` there; green: `metrics transition --task T-NNN --to self_verified`, then `--to queued`; set state queued; ledger commit "publish: T-NNN queued"; then the row for 7 | as for 4 |
| 6, commit failed | `in_progress` committed; `queued` uncommitted | repair, ledger commit "publish: T-NNN queued"; then the row for 7 | as for 4 (after the ledger commit) |
| 7 | `queued` committed; inputs still in the primary checkout | make each input equal `git show task/T-NNN:<path>` (or fix the branch and re-run its gate), then remove it (`rm` a new file, `git checkout HEAD -- <path>` a modified one); `cargo xtask queue T-NNN`; then the row for 8 | as for 4 |
| 8, queue refused | `queued` committed; inputs on `task/T-NNN` | act on the queue's refusal, `cargo xtask queue T-NNN`; merged: `metrics transition --task T-NNN --to merged`, set state merged, ledger commit "publish: T-NNN merged", `worktree rm T-NNN` | as for 4 |
| 8, merged commit failed | merged by the queue; `merged` uncommitted | repair, ledger commit "publish: T-NNN merged", `worktree rm T-NNN` | — (merged is final) |

After an abandon the inputs are still the director's: publish them again under a new task number.

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
