---
id: T-096
title: "The rebuild-from-base test of main's xtask builds with the repository's pinned toolchain"
tier: high              # gate paths: xtask/src/queue.rs and xtask/tests/ (METHODOLOGY §4)
state: ready
owner_role: planner
scorecard_items: []
depends_on: []
touches:                # one role per path (D-153)
  - xtask/src/queue.rs                  # GATE PATH: implementer: the test `tests::main_xtask_is_rebuilt_from_base_on_every_run` only (inside #[cfg(test)] mod tests)
  - xtask/tests/acceptance_t096.rs      # GATE PATH: test author (new file): AC-1
budget_tokens: 1100000  # high (v1.5); estimate about 280,000 (see Estimate)
iteration: 2
found_by: task:T-074    # found in T-074's early CI look (D-258)
origin_task: null       # the test dates from T-005 (D-026 R1); no task's defect in the usual sense: the runner image changed
---

# T-096 The rebuild-from-base test of main's xtask builds with the repository's pinned toolchain

## Goal

The unit test `queue::tests::main_xtask_is_rebuilt_from_base_on_every_run` (`xtask/src/queue.rs`,
line 1993 at main `62a4a52`) shows D-026 R1: main's `xtask` is rebuilt from base on every queue
run, and a poisoned cache (an overwritten binary, an overwritten member artefact, future mtimes,
an edited export) is not trusted. After this task the test passes or fails by what the queue
does, **not by which toolchain the machine running it has as its default**: its tiny workspace
carries the repository's own `rust-toolchain.toml`, as the export of main does in the real queue,
so the tiny build uses the pinned 1.97.0 on every machine.

Today the tiny workspace has no toolchain file and `build_main_xtask` (line 201) removes
`RUSTUP_TOOLCHAIN`, so the tiny build uses the machine's default toolchain. With Rust 1.100.0 as
the default, cargo lays `target/debug` out without `deps/` and the test fails at
`xtask/src/queue.rs:2038:43` with `Os { code: 2, kind: NotFound }`: seen in CI run 44307677646
(`gate-linux`, record `tasks/ci-runs/T-074-early-1.md`, section "gate-linux") and **reproduced by
this spec's probe** (Pinned (g), P-2).

Why now (D-258): main's own CI still gets the older runner image; when image `ubuntu-24.04`
`20261116.376.1` (Rust 1.100.0 installed beside the pinned one, D-258, read by R-363) reaches
main's Linux job, main's counted gate turns red, which is an incident (METHODOLOGY §8). This task
merges before T-072 and T-074 take their formal CI rounds; it shares no path with either
(Order).

**Gate paths:** both paths in `touches` are gate paths (`xtask/`; the list in
`docs/development.md`). The task is high tier and the approval row must say `gate paths`. No
dependency changes; no production code changes (the only `queue.rs` edit is inside
`#[cfg(test)] mod tests`).

Reading list: this file; `xtask/src/queue.rs` lines 196-235 (`build_main_xtask`) and 1991-2088
(the test); `xtask/src/seed.rs` lines 260-326 (`prune_to_non_members`); `rust-toolchain.toml`;
`xtask/platform-tests.toml` lines 87-94 (the test's entry); `xtask/tests/guard_t079.rs` line
1461 and `xtask/tests/acceptance_t079.rs` lines 125-129 (the test's name and entry are pinned);
D-026, D-235, D-249, D-258 in `docs/decisions.md`; METHODOLOGY §6 and §8.1 ("Tests").

## Interface

No public surface changes: no command, CLI, control-protocol call, document or Rust item.

1. **The unit test** (implementer). `queue::tests::main_xtask_is_rebuilt_from_base_on_every_run`
   keeps its name, its place (`xtask/src/queue.rs`, `mod tests`), its `#[cfg(unix)]` attribute
   with its comment, and every assertion it has at main `62a4a52` (none removed, loosened or made
   conditional: AGENTS.md rule 4). One change: the tiny repository's committed tree (the files
   given to `portable_init`, so that the commit `build_main_xtask` exports carries it) gains
   `rust-toolchain.toml` whose bytes are **the bytes of this checkout's own
   `rust-toolchain.toml`**, read when the test runs (the file at `CARGO_MANIFEST_DIR/..`), never
   a channel typed into the test. The registry entry of the test (`xtask/platform-tests.toml`
   lines 87-94) stays as it is; its `reason` stays true.
2. **Optional, Q-3:** the test's reading of `target/debug/deps` (line 2038, `read_dir(&deps)
   .unwrap()`) may become an assertion whose message names the layout assumption (that the
   pinned toolchain's cargo puts member artefacts in `target/debug/deps`). This changes only the
   failure text; no test pins that text.
3. **The acceptance test** (test author, new file `xtask/tests/acceptance_t096.rs`): AC-1 below.

## Pinned

- (a) Error variants, payloads, precedence: n/a (no error surface changes). The test's failure
  when the toolchain file is missing or wrong is a panic; no test pins its text.
- (b) Layout of every file read or written: the test reads `<checkout>/rust-toolchain.toml`
  (TOML, at main: `[toolchain]` `channel = "1.97.0"`, `components = ["rustfmt", "clippy"]`,
  `profile = "minimal"`; command `cat rust-toolchain.toml`) and writes the same bytes as
  `rust-toolchain.toml` at the root of the tiny repository, committed on its `main`. The cargo
  layouts that matter, measured (P-1): cargo 1.97.0 writes `target/debug/{.fingerprint,build,
  deps,examples,incremental}` and `target/debug/deps/xtask-<16 hex>`; cargo 1.100.0 writes
  `target/debug/{build,examples,incremental}` with the member at
  `target/debug/build/xtask/<16 hex>/{fingerprint,out}` and **no `deps/` and no `.fingerprint/`**.
- (c) Order of checks: unchanged (the test's steps run in today's order).
- (d) Path base, platform, file types: the unit test stays `#[cfg(unix)]` (Linux, macOS), as
  registered. **Platform statement per test file:** `xtask/src/queue.rs`'s test: Linux and
  macOS, registered `unix`, unchanged. `xtask/tests/acceptance_t096.rs`: runs on Linux, macOS and
  Windows with **no platform `cfg` attribute and no registry entry**; on Unix it asserts AC-1, on
  Windows the counterpart of AC-1 (the unit test is not built there, Q-2 A); the platform is
  chosen inside the test by a `cfg!(…)` expression, which main's `no-skip` accepts without an
  entry (shown on the prototype, D-235 answers below). It needs `rustup` proxies on `PATH`
  (every CI job installs the toolchain with `rustup toolchain install`, `platforms.yml` lines
  66-71, 150-155, 251-256; `ci.yml` line 34); without `rustup` it fails naming why (METHODOLOGY
  §8.1: a test that cannot run fails there, naming why).
- (e) Threat model (gate path; D-026: honest mistakes and corner-cutting, not a malicious agent):
  the fix must not make the test easier to pass. With option (a) the test builds with the
  toolchain the queue actually uses for main's `xtask` (the export of main carries
  `rust-toolchain.toml`: VERIFIED, P-4), so it checks R1 under the production layout. What it no
  longer checks: the queue under a toolchain other than the pinned one, which the queue does not
  use while main carries a toolchain file. Residual, unchecked: a `rustup override` set for a
  directory above the queue's private build tree outranks the toolchain file (rustup's order:
  `RUSTUP_TOOLCHAIN`, `+toolchain`, directory override, toolchain file, default); not in scope.
- (f) Tests pinning a snapshot this changes: none changes. `xtask/tests/guard_t079.rs:1461`
  pins the test's name at its path (kept); `xtask/tests/acceptance_t079.rs:125-129` pins its
  registry entry (`unix`, counterpart `none`; kept); `xtask/src/provenance.rs:175`
  `include_str!("queue.rs")` only asserts an old variable name is absent (unaffected). Both
  `_t079` files pass on the prototype tree (run below).
- (g) Worked examples, executed (scratch directory `/home/user/kiln-plan-T096/`, a clone of main
  `62a4a52`; a scratch `RUSTUP_HOME` and `CARGO_HOME`; the machine's own default untouched:
  `rustup default` there still prints `stable-x86_64-unknown-linux-gnu (default)` = 1.97.0):
  - **P-0 toolchain download allowed:** `RUSTUP_HOME=<scratch>/rustup rustup toolchain install
    1.100.0 --profile minimal` → `1.100.0-x86_64-unknown-linux-gnu installed - rustc 1.100.0
    (0ef1bc386 2026-11-09)`; `cargo --version` → `cargo 1.100.0 (3acc79c0c 2026-11-05)`.
  - **P-1 the tiny workspace with each toolchain** (the test's three files; `env -u
    RUSTUP_TOOLCHAIN CARGO_TARGET_DIR=<w>/target cargo build --quiet --package xtask`, then
    `find target -maxdepth 3`): with default 1.100.0, exit 0, `target/debug` holds
    `.cargo-artifact-lock .cargo-build-lock .cargo-lock build/xtask examples incremental xtask
    xtask.d`, and the member is at `build/xtask/1dbd46f60fc59203/out/xtask`; **no `deps`**. With
    `+1.97.0`: `target/debug` holds `.fingerprint/xtask-9fdb320c82cd348f build deps/xtask-9fdb…
    examples incremental xtask xtask.d`. So "the default cargo of the new toolchain lays out
    `target/` without `debug/deps`" is **VERIFIED for cargo 1.100.0** (D-258 had it INFERRED).
    That the runner image's default is 1.100.0 rests on R-363's reading (D-258), not re-checked.
    Whether 1.99.1 has the old layout: unchecked (not installed).
  - **P-2 the test at base with default 1.100.0** (scratch clone, `cargo test --offline -q -p
    xtask --bin xtask queue::tests::main_xtask_is_rebuilt_from_base_on_every_run`; the test
    binary itself is built by 1.97.0 through the clone's toolchain file): `panicked at
    xtask/src/queue.rs:2038:43: called Result::unwrap() on an Err value: Os { code: 2, kind:
    NotFound, message: "No such file or directory" }`, `test result: FAILED. 0 passed; 1 failed`
    — **the CI failure, reproduced**. Same clone, `rustup default 1.97.0`: `test result: ok. 1
    passed`.
  - **P-3 option (a) prototype** (Interface (1): eight lines reading the checkout's file and one
    tuple added to `portable_init`'s list; patch kept at `/home/user/kiln-plan-T096/option-a.patch`
    until the scratch is removed): default 1.100.0 → `test result: ok. 1 passed`. With a fake
    default toolchain (P-5) → `ok. 1 passed`; base with the same fake default → `panicked at
    xtask/src/queue.rs:2019:48: … list the workspace members of main at <sha> cargo metadata
    failed in …/main-xtask/src: fake default toolchain`, `FAILED. 0 passed; 1 failed`.
  - **P-3b option (b) prototype** (the `deps` loop replaced by a walk of the whole profile that
    overwrites every file named `xtask` or `xtask-<…>` without a dot other than the uplifted
    binary, with a non-empty assertion): default 1.100.0 → poisons
    `target/debug/build/xtask/1dbd46f60fc59203/out/xtask` and `ok. 1 passed`. So (b) works too;
    R1 holds under the new layout, because `prune_to_non_members` (`seed.rs:301-324`) finds no
    `.fingerprint` in `debug/` and removes all of it (read in the code; consistent with P-3b).
  - **P-4 the real queue under default 1.100.0:** an export of main `62a4a52` (`git archive`,
    the same files as `export_tree`), `env -u RUSTUP_TOOLCHAIN rustup show active-toolchain` →
    `1.97.0-x86_64-unknown-linux-gnu (overridden by '<export>/rust-toolchain.toml')`; `cargo
    build --offline --quiet --locked --package xtask` → exit 0, `target/debug` has `deps`. So
    the claim "the real queue is not affected" is **VERIFIED** for the build of main's `xtask`;
    `seed::member_names` runs in the same export, so it is pinned too.
  - **P-5 the acceptance test's oracle, prototyped** (`/home/user/kiln-plan-T096/proto/
    acceptance_t096.rs`, not a deliverable): a scratch `RUSTUP_HOME` with
    `toolchains/1.97.0-x86_64-unknown-linux-gnu` a symlink to the real pinned toolchain (found by
    `rustc --print sysroot` in the checkout), `toolchains/fake/bin/{cargo,rustc}` shell scripts
    that print `fake default toolchain` and exit 97, `settings.toml` with `default_toolchain =
    "fake"`; then `cargo test --offline -q -p xtask --bin xtask <the test> -- --exact` in the
    checkout with that `RUSTUP_HOME`, `RUSTUP_AUTO_INSTALL=0` and a scratch `CARGO_TARGET_DIR`.
    Prototype tree: `test result: ok. 1 passed … finished in 24.23s`. Base with the prototype
    test: `FAILED. 0 passed; 1 failed … finished in 24.28s`, the inner panic at
    `queue.rs:2019:48` naming `fake default toolchain`. Fail on base by assertion, no network.
- (h) Per scorecard item: n/a (no scorecard item; `scorecard_items: []`).
- (i) Document sentences: none. The task adds or changes no sentence in `docs/`; the sentence the
  registry entry quotes (`docs/development.md`: "so a binary left by an earlier run (or
  overwritten by task code during an earlier gate) is never trusted") stays true and unchanged.

### Options weighed (the director's request)

| Option | What changes | What the test then shows of D-026 R1 | Verdict |
|---|---|---|---|
| (a) tiny workspace carries the repository's `rust-toolchain.toml` | ~10 lines in the test | R1 under the toolchain and target layout the queue really uses for main's `xtask` (P-4); unchanged assertions, unchanged poison targets. If a later toolchain bump brings the new layout, this test fails loudly at the `deps` read: the right alarm, since `seed.rs` assumes that layout too (Notes) | **chosen**: smallest, test-only, deterministic on every machine (P-3) |
| (b) the test stops depending on the layout | the poison loop walks the whole profile | R1 under whatever the machine's default is, i.e. a toolchain the queue does not use for main's `xtask`; the pass still depends on the default being installed and working (a broken or absent default fails it, P-5's fake shows that); the layout alarm is lost | works (P-3b), not chosen; could be added later on top of (a) when a bump needs it |
| (c) `build_main_xtask` pins the toolchain | production code in the queue | nothing new in production (the export already pins, P-4); for an export without a toolchain file it would have to invent one (whose? the task's? the caller's?): a change to the queue's behaviour on a gate path that the defect does not need, and a wider review | rejected |

## Acceptance criteria

- **AC-1 (test author, `xtask/tests/acceptance_t096.rs`):** Given this checkout and a scratch
  `RUSTUP_HOME` in which the pinned toolchain is installed under its own name and the **default
  toolchain cannot build** (its `cargo` and `rustc` exit non-zero with a marker text), when
  `cargo test --offline --locked -p xtask --bin xtask
  queue::tests::main_xtask_is_rebuilt_from_base_on_every_run -- --exact` runs in the checkout
  with that `RUSTUP_HOME`, then it exits 0 and its result line reports exactly 1 passed and 0
  failed. Before that main assertion the test shows that its setup is effective (METHODOLOGY §6
  self-check: the forbidden target exists): with that `RUSTUP_HOME`, a `cargo --version` run
  with `RUSTUP_TOOLCHAIN` removed in a directory that has no toolchain file above it fails and
  prints the marker, and the same in the checkout prints the pinned version read from
  `rust-toolchain.toml`. Oracle: exit status and the counts of the result line; the marker; the
  pinned channel from the repository's file. **On base it fails by assertion** (P-5: exit 101,
  the inner panic names the marker). Platform: Linux and macOS (Unix branch). On Windows
  (`cfg!(windows)`, Q-2 A) the counterpart: `cargo test --offline --locked -p xtask --bin xtask
  -- --list` exits 0 and its list contains no line for
  `queue::tests::main_xtask_is_rebuilt_from_base_on_every_run` (the registry's `unix`), and the
  test says so in its output. The test removes its scratch directories; every process it spawns
  has a timeout (AGENTS.md); it sets `CARGO_TARGET_DIR` only for the processes it spawns, to a
  scratch directory, never for itself.
- **AC-2 (implementer, the unit test itself, `xtask/src/queue.rs`):** with the machine's own
  default toolchain and, where it can be had, with 1.100.0 as the default of a scratch
  `RUSTUP_HOME` (P-0 shows the download works here), `cargo test -p xtask --bin xtask
  queue::tests::main_xtask_is_rebuilt_from_base_on_every_run` passes; the implementer logs both
  runs with their result lines. The reviewer repeats the 1.100.0 run. (The implementer owns this
  file; the independent check from outside is AC-1.)
- **Equality (absence):** the diff of `xtask/src/queue.rs` between base and head is exactly
  inside the body of `tests::main_xtask_is_rebuilt_from_base_on_every_run` (reviewer's diff
  check, not a guard). A variant outside it that must be rejected: any change to
  `build_main_xtask`, `export_tree` or `seed.rs`.
- **Guards:** none new. `guard_t079` and `acceptance_t079` must still pass (they pin the test's
  name and entry).

## Out of scope and implementer notes

- Do not change `build_main_xtask`, `seed.rs`, the registry, `platforms.yml` or any document.
- **Note for a later draft (not scope here):** what else in `xtask/` assumes the pinned
  toolchain's layout or the default toolchain (`grep -n '"deps"\|\.fingerprint\|RUSTUP_TOOLCHAIN\|
  rust-toolchain' xtask/src/*.rs xtask/tests/*.rs`):
  1. `seed::prune_to_non_members` (`seed.rs:265-326`) and `seed::seed`/its walk of profile
     directories (`seed.rs:131-150`, 232-252) know only `<profile>/{.fingerprint,build,deps}`.
     Under the 1.100.0 layout prune finds no profile and removes all of `target/debug`
     (safe for R1, but no dependency artefact is reused: every queue run and `publish` rebuild
     all dependencies), and a seed copies nothing (new worktrees build from scratch). Harmless
     while the pin is 1.97.0; matters at the first toolchain bump to a cargo with the new layout.
  2. `publish.rs:545-546` uses the same prune on its private target (same consequence).
  3. Typed channels in toy repositories: `"1.97.0"` in `xtask/tests/acceptance_t005_support.rs:287`,
     `acceptance_t005.rs:704`, `guard_t005.rs:112` (go stale at a bump; `acceptance_t005_r3.rs:529`
     reads the repository's file instead).
  4. Every production cargo call with `RUSTUP_TOOLCHAIN` removed (`queue.rs:128`, 227, 490;
     `publish.rs:394`; `seed.rs:49`) runs in a checkout or export that carries
     `rust-toolchain.toml`, so it is pinned (P-4 for the build of main's `xtask`; the others by
     reading). No other `xtask/src` unit test runs real cargo in a toy repository without a
     toolchain file (`grep -n '"Cargo.toml"' xtask/src/*.rs`: the toy manifests at
     `queue.rs:1235-1255` and `provenance.rs:129-156` are never built).
  Recommended: one draft "toolchain bump readiness" covering 1-3 (Q-4).
- Determinism: the acceptance test needs no network (`--offline`; the dependencies are in the
  cargo registry after the gate's own build). Its cost, measured on the prototype: about 24 s
  for the nested test build and run on this machine; target size not measured.

## Spec-check answers (D-235, D-249), by runs

Run on a scratch clone of main `62a4a52` with the prototype (P-3 patch plus P-5 test file)
committed there; main's `xtask` built from clean main in that clone before the patch (`cargo
build --offline -q -p xtask`, binary copied aside) and run from the clone's root:

| main's step | output on the planned tree |
|---|---|
| `no-skip` | `PASS no-skip: self-test ok (17 known findings); 202 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (19 pending), today = 2026-11-18` (base: 201 files, same otherwise: the new file is scanned and passes without a registry entry) |
| `hygiene` | `PASS hygiene: reviews: 59 of 71 review file(s) checked (medium/high, iteration >= 1), ok; session-paths: 696 tracked file(s) clean (0 not UTF-8, skipped)` |
| `layers` | `PASS layers: 11 crates, 14 internal edges, layer table holds` |
| `ui-commands` | `PASS ui-commands: self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s)` |
| `schema` | `PASS schema: 30 commands; audit ok; docs/commands.json current` |
| `scorecard --check` | **not run**: it builds the whole workspace's tests (4-5 GB, the worktrees' `target/` sizes) and the disk had 5.9 GB free with builders running (D-182, D-205). By reading: no file under `scorecard/` or `docs/scorecard.md` names `xtask`, `queue.rs` or `acceptance_t096` (`grep -rn`), so no item's tests change. **The implementer runs it** with main's tool before the join (D-235 rule). |

D-249 (main's tests that name `xtask/src/queue.rs` or the test's name; `grep -rn
'queue\.rs\|main_xtask_is_rebuilt' --include=*.rs crates apps xtask`):
`xtask/tests/guard_t079.rs:1450-1469` (20 names of `queue.rs` tests, among them this one at
1461; the guard asks that base names still exist: kept), `xtask/tests/acceptance_t079.rs:92`,
125-129 (registry entries of `queue.rs` tests: unchanged), `xtask/src/provenance.rs:175`
(absence of an old variable name in `queue.rs`: unaffected). Run on the planned tree: `cargo test
-p xtask --test guard_t079 --test acceptance_t079` → `ok. 2 passed` and `ok. 6 passed`;
`cargo test -p xtask --bin xtask provenance` → `ok. 4 passed`.

In-flight branches (read-only `git grep` on `task/T-072` `d87f2e8`, `task/T-072-tests`
`8967249`, `task/T-074` `22eea4e`, `task/T-074-tests` `5799fcc`): none of their test files
names `queue.rs`, the test, or `acceptance_t096`.

## Pending CI criterion

The join records a pending CI criterion (METHODOLOGY §8.1): on the joined head pushed to
`ci/candidate`, `gate-linux` success, with `queue::tests::main_xtask_is_rebuilt_from_base_on_every_run`
and `acceptance_t096` passing in its record; `gate-macos` success (both run there, Unix branch);
on Windows the `xtask` binaries' exit status 0 as on main (D-243), with `acceptance_t096`
passing (Windows branch). **Which runner image the Linux job gets is not controllable**: the
director records the image version from the job's "Set up job" log (records are otherwise from
artefacts). If it is `20261116.376.1` or later, the run also shows the fix on the real new image;
if it is older, the evidence for the new default is P-2/P-3 (local, 1.100.0) plus AC-1 (a default
that cannot build, on every runner), and the task is not held for the image.

## Estimate

About **280,000 tokens** for all roles (ceiling 1,100,000): implementer about 70,000 (one test
edit, AC-2's two runs, main's tool on the tree, the gate); test author about 80,000 (one file,
self-check against ≥ 5 seeded bugs, e.g. no toolchain file, a typed other channel, the file
written but not committed, the file read from a wrong path, an empty file); reviewer about
90,000 (round 1 with ≥ 10 mutants, each costing one nested run of about 24 s; the 1.100.0 run;
adversarial: a broken default, an absent default); CI record about 40,000.

## Order

T-096 merges **before** T-072 and T-074 take their formal CI rounds (D-258). Shared paths: none.
T-072's `touches`: `.github/workflows/platforms.yml`, `xtask/ledger.toml`, `docs/development.md`,
`xtask/tests/acceptance_t072.rs`, `xtask/tests/guard_t072.rs`. T-074's: `crates/kiln-workspace/src/disk.rs`,
`xtask/platform-tests.toml`, `docs/protocol/paths.md`, `docs/protocol/project.md`,
`docs/control-protocol.md`, `crates/kiln-workspace/tests/guard_t074.rs`,
`xtask/tests/acceptance_t074.rs`, `crates/kiln-workspace/tests/acceptance_t002.rs` (front matter
of both task files at main `62a4a52`). T-096's two paths are neither equal to nor a directory
prefix of any of them (§5.2). Under Q-2 B (a `cfg` attribute and a registry entry) T-096 would
share `xtask/platform-tests.toml` with T-074 and could not run beside it: one more reason for
Q-2 A. After T-096 merges, both rework branches rebase onto it before their CI rounds; their
spec checks need no re-run for T-096 (no overlapping `touches`, §5.7).

## Questions and rulings

- **Q-1 (planner):** the fix. A: option (a), the tiny workspace carries the repository's
  `rust-toolchain.toml` (recommended: P-3; test-only; the test then checks R1 under the queue's
  real toolchain). B: option (b), layout-independent poisoning (works, P-3b, but tests a toolchain
  the queue does not use and depends on the default). C: option (c), pin in `build_main_xtask`
  (production change the defect does not need).
- **Q-2 (planner):** the acceptance test on Windows, where the unit test does not exist. A: one
  test with a `cfg!(unix)` branch (AC-1) and a Windows branch (the unit test is absent from the
  binary's `--list`); no registry entry; main's `no-skip` passes on the prototype (recommended).
  B: a `#[cfg(unix)]` attribute and a registry entry: needs `xtask/platform-tests.toml`, held by
  T-074, so T-096 would wait for T-074 (against D-258's order).
- **Q-3 (planner):** the `deps` read in the test gets an assertion message naming the layout
  assumption (Interface (2)). A: allowed, the implementer's wording (recommended: a future bump
  then fails with a message that points at the layout, not at `NotFound`). B: leave the line.
- **Q-4 (planner):** the layout and typed-channel findings (Out of scope, note items 1-3). A:
  file one draft "toolchain bump readiness" now, to be specified before any toolchain bump
  (recommended). B: a note in T-091 only.
- **Q-5 (planner):** an environment note for the director, not about the spec: the first probe
  command (`rustup toolchain install 1.100.0` with a scratch `RUSTUP_HOME` but the real
  `CARGO_HOME`) let rustup **self-update the machine's `rustup` binary** from 1.28.2 to 1.29.3
  (`/root/.cargo/bin/rustup`). The machine's toolchains and default are unchanged (`rustup
  default`: `stable-x86_64-unknown-linux-gnu (default)`; `cargo --version` in the checkout:
  `cargo 1.97.0 (c980f4866 2026-06-30)`). Later probe commands used a scratch `CARGO_HOME` with
  self-update disabled. A reviewer or test author who repeats P-0 should set both homes to
  scratch and run `rustup set auto-self-update disable` there first.

### Rulings (director, D-259)
- **Q-1: (a).** The test's tiny workspace carries this repository's `rust-toolchain.toml`, copied at run time.
- **Q-2: yes.** In `xtask/tests/acceptance_t096.rs` a `cfg!(unix)` branch (both branches assert), not a registry
  entry: `xtask/platform-tests.toml` belongs to T-074 while it is in flight. Main's `no-skip` must pass on the tree.
- **Q-3: yes.** An assertion message may name the layout assumption.
- **Q-4: yes.** What else assumes the old `target/` layout or types the channel is drafted as T-097 (toolchain
  bump readiness); nothing of it is in this task.
- **Q-5: noted.** The planner's install let `rustup` update itself on this machine (1.28.2 to 1.29.3); toolchains and
  the default are unchanged (D-259).
- The implementer runs `scorecard --check` of main's tool on the tree (the planner could not, for disk).
- No separate fresh spec check: the task changes one test, and the probe reproduced both the failure and the fix.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-18T13:55Z · R-NNN (run id not given in the dispatch) · draft (new) · did: specified
  T-096 from D-258 and the CI record of run 44307677646; probed in `/home/user/kiln-plan-T096/`
  (scratch clone of main `62a4a52`, scratch `RUSTUP_HOME`/`CARGO_HOME`): 1.100.0 downloaded; the
  tiny workspace's layout under 1.100.0 and 1.97.0 (P-1); the CI failure reproduced with 1.100.0
  as default (P-2); options (a) and (b) prototyped, both pass (P-3, P-3b); the real queue's
  export pinned under default 1.100.0 (P-4); AC-1's oracle prototyped with a fake default
  toolchain, fail on base and pass on the prototype (P-5); main's `no-skip`, `hygiene`, `layers`,
  `ui-commands`, `schema` pass on the planned tree; the D-249 tests pass. Tier high: the
  criterion is "any gate path" (`xtask/src/queue.rs`, `xtask/tests/`). Pinned "n/a": (a), (h),
  (i) (no error surface, no scorecard item, no document sentence). Unsure: whether 1.99.1
  already has the new layout (not installed); the nested build's disk size (not measured); the
  Windows branch's cost on the hosted runner (a second build of `xtask`'s test binary, not
  measured) · gate attempts: 0 · not run: `scorecard --check` with main's tool (disk: 5.9 GB
  free with builders running; the implementer runs it), the whole `xtask` test suite on the
  prototype · missing context: the run id · next: director rules Q-1 to Q-5; spec check not
  required by §5.7 (no `depends_on`), the director may still ask for one (gate path).
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
