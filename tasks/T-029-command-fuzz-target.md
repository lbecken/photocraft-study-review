---
id: T-029
title: "Fuzz every registered command with schema-guided and random params in the gate, so that a panic or an internal error in any command fails the gate with a reproducer"
tier: high              # ADR-0002: gate path (xtask/fuzz.toml); approval row must contain `gate paths`
state: draft
owner_role: planner
scorecard_items: []     # gate capability; no product item
depends_on: [T-028]     # the fuzz runner
touches:
  - xtask/fuzz.toml                               # GATE PATH: register commands-random-params
  - crates/kiln-control/examples/fuzz_commands.rs # the fuzz program (not a gate path)
  - docs/development.md                           # one paragraph: the target, how to reproduce a failure
budget_tokens: 850000   # high (v1.1)
iteration: 2
found_by: planner
origin_task: null
---

# T-029 Command fuzz target

## Goal
Iteration 0 could not test the panic path: no test can make a real command panic, and AC-6 of
T-015 AC-8 leaves it to reading. After this task the gate's `fuzz` step runs a program on every
change. It feeds every command registered in `standard_registry()`, found at run time, with
generated params, in random short sessions over a fixed set of awkward files. It checks two
things:
- every request gets a response, and no command panics or returns an `internal` error;
- the detector itself works: each run starts with a self-test against two deliberately broken
  commands, so the target cannot pass vacuously.

Commands added later (T-018 … T-027) are fuzzed automatically, because the program discovers
them. A failure prints a reproducer: seed, case, step, command, params.

**Gate path:** `xtask/fuzz.toml`; the approval row must contain `gate paths`.

Reading list: `tasks/T-028-*.md` (fuzz entry keys and output protocol); `crates/kiln-commands/src/registry.rs`
(`Registry::register`, `describe`, `execute`; `Context::with_files`); `crates/kiln-control/src/lib.rs`
(`Host::in_memory`, `handle_line`; panics become `internal`/`panic`); `docs/commands.json`
(schemars output: `$defs`, `$ref`, `oneOf`, `enum`, `minimum`, `required`,
`additionalProperties: false`).

## Interface

### Registration
```toml
[[target]]
name = "commands-random-params"
description = "every registered command, schema-guided and random params, short sessions; no panic, no internal error"
command = ["cargo", "run", "--locked", "-q", "-p", "kiln-control", "--example", "fuzz_commands", "--", "--seed", "1", "--cases", "3000"]
timeout_s = 300
min_cases = 3000
```

### Program `fuzz_commands [--seed N] [--cases N] [--self-test-only]`
- A deterministic PRNG (splitmix64; no new dependency), seeded by `--seed`. The same seed and
  build give the same output.
- **Fixture files** (every case starts from them): `a.txt` = `hello\n`; `crlf.rs` = a CRLF Rust
  file; `bom.rs` (UTF-8 BOM); `u16.txt` (UTF-16LE with BOM); `bad.bin` = `\xff\xfe\xfd` +
  random bytes; `empty`; `d/e/f.rs`; `Cargo.toml` + `src/lib.rs` (a tiny crate); `.gitignore` =
  `*.log\n`; `x.log`.
- **A case**: a fresh `Context::with_files(fixtures)`, which starts with `NoTools`, so nothing
  can spawn. With probability 0.8, first 1–3 `doc.open` of random fixture paths. Then 1–8 steps,
  each a uniformly chosen registered name with params that are, at 70%, generated **from that
  command's params schema** and, at 30%, a random JSON value.
  - Schema generation handles `type` (object, integer, string, boolean, array, null),
    `required`/optional properties, `enum`, `oneOf`/`anyOf`, `$ref` into `$defs`, and
    `minimum`/`maximum`.
  - Integers are drawn from {0, 1, 2, 3, the open doc ids, current `len_chars` ± 1, 2^32,
    2^63 − 1, a random value}.
  - Strings are drawn from fixture paths, `../x`, `""`, `d`, a 300-char name, regex fragments
    (`(`, `a*`, `[^\r\n]`), text with `\r\n`, `é`, `😀`, NUL, and random text.
  - Each step is `Registry::execute` inside `catch_unwind`.
- **The same case is replayed through the protocol**: each step as a request line to
  `Host::in_memory(fixtures)`, plus at 5% a random byte line.
- **A failure** is any of:
  - a panic in `execute` (caught);
  - `Err(CommandError::Internal { .. })`;
  - a protocol response with `"code":"internal"`;
  - a non-blank request line without exactly one response line.
  The first failure is reported as `first_failure` (JSON text: `seed`, `case`, `step`,
  `command`, `params`, `what`).
- **Self-test** (always first; alone with `--self-test-only`): on `standard_registry()` plus two
  commands registered by the program, it runs 200 cases.
  - `fuzz-selftest.panic` (`{n: integer 0..=5}`) panics when `n == 3`, through an operation
    the workspace lints allow (e.g. an out-of-bounds slice index; no `allow` attribute).
  - `fuzz-selftest.bad-result` returns output that violates its own result schema.
  `self_test` is `passed` iff both are detected as failures. The self-test's failures do not
  count in `failures`.
- **Last stdout line** (T-028 protocol): `{"cases", "failures", "self_test", "first_failure"?,
  "commands": {<name>: {"calls", "valid"}}}`. `valid` counts calls whose params passed schema
  validation. Exit 0 unless the program itself cannot run.
- The panic hook is silenced during the run so stderr stays readable. The real hook is
  restored before printing.

## Pinned
- (a) Failure classes as listed. Precedence within a step: panic → `internal` → protocol
  framing.
- (b) Files: none on disk; the fixtures are a `MemoryFs`.
- (c) Order: self-test → cases 0..N, each a registry pass, then a protocol pass.
- (d) Platform independent (MemoryFs only); `NoTools` guarantees no process is spawned. The
  reviewer checks that the program never calls `set_tools`.
- (e) Threat model (gate): it must not pass vacuously. The self-test proves detection; AC-3
  proves coverage. If a run finds a real panic in an existing command, this task does **not**
  fix it (outside `touches`). The implementer stops and reports with the reproducer, and the
  director files a defect task (`found_by: task:T-029`) and decides whether the target lands
  first (it would turn the gate red).
- (f) Snapshots: none.
- (g) Worked example: `--self-test-only --seed 1` prints a line with `"self_test":"passed"`.
  After temporarily making `fuzz-selftest.panic` never panic (a reviewer mutation), it prints
  `"self_test":"failed"`, and the gate step FAILs under T-028's protocol.
- (h) Scorecard: none.

## Acceptance criteria
- AC-1 (gate): `cargo xtask fuzz` passes on the task branch. Its detail shows ≥ 3000 cases and
  0 failures. The step takes < 120 s warm on the bootstrap machine (measured and recorded in
  the log). Oracle: the gate step.
- AC-2 (self-test): `--self-test-only` reports `passed`. With the environment variable
  `KILN_FUZZ_SELFTEST_BROKEN=1` (read only by this program, documented as test-only), the two
  self-test commands behave correctly, and the program reports `failed`. Oracle: exact value.
- AC-3 (coverage): in the output line, every name in `kiln-cli commands --schema` appears with
  `calls ≥ 50` and `valid ≥ 10`. Oracle: exact property against the public command list.
- AC-4 (determinism and reproducer): two runs with `--seed 7 --cases 300` print identical last
  lines. A reproducer printed for a seeded failure (the self-test's) names a case and step that,
  re-run alone, fail the same way. Oracle: differential.
- AC-5 (protocol pass): random byte lines (invalid UTF-8, huge, `{}`) never yield `internal`,
  and every non-blank line gets one response. Oracle: framing rule of `docs/control-protocol.md`.

## Scorecard items and intended test names
None. Tests: `crates/kiln-control/tests/acceptance_t029.rs` (`self_test_detects_broken_commands`,
`every_command_is_fuzzed_with_valid_params`, `fuzz_is_deterministic_and_reproducible`) and
`xtask/tests/acceptance_t029.rs` (`fuzz_target_is_registered_and_passes`).

## Out of scope and implementer notes
- Coverage-guided fuzzing (`cargo-fuzz` is not installed and would be a new tool); fuzzing
  `DiskFs` or real tools; checking T-007's read-only property, whose own audit does that.
- If the env-var switch in AC-2 is judged a test hook in product-side code, use a second example
  file instead. That needs a ruling: the path is not in `touches` (Q in your log).

## Questions and rulings

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13 · R-056 · (new) · did: specified the command fuzz target as an example in
  kiln-control. It sits there because only a layer above kiln-commands may use the Host, and an
  example in kiln-commands depending on kiln-control would be an upward dev-dependency.
  Commands are discovered at run time, the generator is schema-guided, there is a self-test
  against two broken commands, a coverage floor, and a reproducer. Tier high: gate path
  `xtask/fuzz.toml`. A real panic found while landing it is a stop condition, not a fix. ·
  gate attempts: 0 · missing context: none · next: director.
- 2026-11-13 · R-060 · draft · did: the panic-path reference now points to T-015 (it absorbed T-030's reasons). No other change; the fuzz program also proves T-015's `InternalReason::Panic` mapping end to end. · gate attempts: 0 · missing context: none · next: director.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
