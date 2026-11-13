---
id: T-028
title: "Give the gate's fuzz and budgets steps a runner, and enforce the first two budgets: single-char edits in a 10 MB rope, and a 50 MB open-edit-save through the built CLI"
tier: high              # ADR-0002: gate paths (xtask/src, xtask/budgets.toml); approval row must contain `gate paths`
state: draft
owner_role: planner
scorecard_items: [text.rope]
depends_on: [T-013]          # merged; R-112: none of T-009/T-033/T-034 is needed (they do not touch registries.rs, gate.rs or budgets.toml); shares only docs/development.md with them: not in flight together
touches:
  - xtask/src/registries.rs                 # GATE PATH: the runners
  - xtask/src/gate.rs                       # GATE PATH: only if the step dispatch must change
  - xtask/budgets.toml                      # GATE PATH: register rope-edit-10mb and cli-roundtrip-50mb
  - crates/kiln-text/examples/budget_rope.rs   # measuring program 1 (product-side, not a gate path)
  - apps/kiln-cli/examples/budget_roundtrip.rs # measuring program 2: drives the built kiln-cli binary
  - docs/development.md                     # gate table rows `fuzz`, `budgets`; "Other registries"
budget_tokens: 850000   # high (v1.1)
iteration: 2
found_by: planner
origin_task: null
---

# T-028 Runners for the `fuzz` and `budgets` gate steps; the rope edit budget

## Goal
CHARTER §4 lists `fuzz` and `budgets` as gate steps. Today both report EMPTY, and any
registered target fails, because no runner exists (`xtask/src/registries.rs`). Claim C6
("budgets that gate stay honest") was refuted in iteration 0: no budget was enforced. After
this task each registered target is run by the gate and judged against a pinned output
protocol. The first budget is registered: p99 latency of single-char inserts and deletes at
scattered positions in a 10 MB buffer, which makes `text.rope` a gated claim. The command-fuzz
target is T-029, which registers itself in `xtask/fuzz.toml` once this runner exists.

**Gate paths:** `xtask/src/registries.rs`, `xtask/src/gate.rs` and `xtask/budgets.toml`. The
approval row must contain `gate paths`. xtask keeps depending on no workspace crate: targets
are programs it runs.

Reading list: CHARTER §4 (gate steps), METHODOLOGY §11 ("an unenforced one is not held");
`xtask/src/{registries,gate,util}.rs`; `xtask/fuzz.toml`, `xtask/budgets.toml`;
`docs/development.md` "The gate"; `xtask/tests/acceptance_t005_support.rs` (driving xtask in a
temporary root).

## Interface

### Registry entries (strict TOML; unknown keys or bad values are a FAIL naming the file and key)
`xtask/fuzz.toml`:
```toml
[[target]]
name = "commands-random-params"    # unique, [a-z0-9-]+
description = "…"
command = ["cargo", "run", "--locked", "-q", "-p", "…", "--example", "…", "--", "…"]  # argv, run from the repository root
timeout_s = 300                    # 1..=3600
min_cases = 2000                   # ≥ 1
```
`xtask/budgets.toml`:
```toml
[[target]]
name = "rope-edit-10mb"
description = "…"
command = ["cargo", "run", "--release", "--locked", "-q", "-p", "kiln-text", "--example", "budget_rope"]
timeout_s = 600
metric = "median_edit_us"
unit = "us"
max = 200                          # ≈ 5 × the iteration-0 measurement (director, R-060)
attempts = 3                       # 1..=5

[[target]]
name = "cli-roundtrip-50mb"
description = "kiln-cli run: open a 50 MB file, one edit, save; whole process wall time"
prepare = ["cargo", "build", "--release", "--locked", "-q", "-p", "kiln-cli"]   # optional key: run once before the attempts
command = ["cargo", "run", "--release", "--locked", "-q", "-p", "kiln-cli", "--example", "budget_roundtrip"]
timeout_s = 900
metric = "roundtrip_ms"
unit = "ms"
max = 1500                         # ≈ 5 × the iteration-0 measurement (director, R-060)
attempts = 3
```
`prepare` (optional, argv) runs once per target before the first attempt with the same
`timeout_s`; its failure is FAIL with reason `prepare`.

### Output protocol (what a target program prints)
The last non-empty stdout line must be one JSON object.
- Fuzz: `{"cases": n, "failures": f, "self_test": "passed"|"failed", "first_failure": "…"?}`.
  **PASS** iff exit 0, `self_test == "passed"`, `failures == 0` and `cases ≥ min_cases`.
- Budget: `{"metric": "<name>", "value": x, …}`, with `metric` equal to the entry's. A run
  **meets** the budget iff exit 0 and `value ≤ max`. The step runs the program up to `attempts`
  times and stops at the first run that meets it. **PASS** iff one did; otherwise FAIL listing
  every value. Noise from a loaded machine is absorbed by attempts, never by raising `max`.
- Anything else (non-zero exit, timeout (the process is killed), no JSON line, a missing or
  wrong-typed field, a wrong metric name) is FAIL. The detail shows the target name, the reason
  and the last 20 lines of stderr.
- An empty registry stays EMPTY, as today. Each target's detail line shows its result, e.g.
  `rope-edit-10mb: median_edit_us 31.0 ≤ 200 (attempt 1/3)` or `commands-random-params: 3000
  cases, 0 failures`.

### The measuring program (`crates/kiln-text/examples/budget_rope.rs`)
It uses only `kiln_text::Buffer`'s public API on `main` when this task lands.
- Build a buffer of exactly 10,000,000 chars, generated by a fixed 64-bit LCG (seed 1): lines of
  1–120 chars from `a–z`, space, `é`, `_`, with breaks cycling LF, CRLF, LF, CR.
- Do 200 warm-up operations, then 2,000 timed ones alternating `insert(p, "x")` and
  `delete(p..p+1)`. The positions come from the same LCG modulo the current length; a delete
  position that would split a CRLF moves one char left.
- Time each operation with `std::time::Instant`. Print
  `{"metric":"median_edit_us","value":<median>,"p99_us":…,"max_us":…,"ops":2000,"len_chars":…}`.
  This tooling is outside L0–L4 behaviour, so the clock is allowed here.

### The measuring program (`apps/kiln-cli/examples/budget_roundtrip.rs`)
- Generate, by the same LCG, a Rust-like file of exactly 52,428,800 bytes (LF lines of 1–100
  ASCII chars) in `<root>/target/budget-roundtrip/` (created fresh; nothing outside `target/`).
  Write the script `{"id":1,"command":"doc.open","params":{"path":"big.rs"}}`, then a
  `doc.insert` of `"x"` at offset 26,214,400, then `doc.save`.
- Run `<root>/target/release/kiln-cli run s.jsonl --root <dir>`, built by `prepare` (the
  example finds the root from `CARGO_MANIFEST_DIR`). It is spawned with a 300 s timeout, and the
  wall time is measured from spawn to exit.
- Check exit 0, three `ok:true` lines, and that the file is 52,428,801 bytes. Otherwise it exits
  1 with the reason on stderr.
- Print `{"metric":"roundtrip_ms","value":<ms>,"bytes":52428801}`. Delete the directory.

### Budget values (director, R-060)
Both maxima are about **five times** the iteration-0 evaluator's release-build measurements
(`evaluations/iter-0.md` "Measurements"): 0.04 ms median per edit command → 200 µs, and
0.28–0.34 s for open + edit + save of a 50 MB file → 1,500 ms. The purpose is to catch
**order-of-magnitude regressions on any machine** (sandbox, CI, a loaded 4-core box), not to tune
performance. A budget is never raised to get green (golden rule 4); a change needs a
retrospective decision.

## Pinned
- (a) FAIL reasons as listed; per target, the first reason wins in the order config → spawn →
  timeout → exit status → output parse → judgement. The step runs every target and reports all
  failures, not only the first.
- (b) Files read: the two registries (from the gate's root). Nothing written except the target's
  own build output under `<root>/target`.
- (c) Order: targets in file order. The fuzz step runs before budgets (CHARTER §4).
- (d) Linux; the command is run with the gate's environment unchanged (no `CARGO_TARGET_DIR`
  added), cwd = the gate's root.
- (e) Threat model (gate): a target must not be able to pass vacuously. A program printing a
  PASS line with non-zero exit, `self_test: "failed"`, `cases: 0`, `value` as a string, or the
  wrong `metric` fails. A program that hangs is killed at `timeout_s`, and the step fails rather
  than blocking the queue. A registry edit is a gate path (high, approval row). The reviewer
  probes each case.
- (f) Snapshots: the unit test `registries::tests::empty_is_reported_and_registered_without_runner_fails`
  pins "registered ⇒ FAIL". It changes: registered targets now run. The implementer lists it in
  the log as a golden-rule-4 disclosure. The EMPTY half stays. `docs/development.md` rows for
  `fuzz`/`budgets` change.
- (g) Worked example: the iteration-0 evaluator measured 1,000 single-char inserts in a 50 MB
  file through `kiln-cli control` (release) at median 0.04 ms, p99 0.09 ms, and open + edit +
  save at 0.28–0.34 s (`evaluations/iter-0.md` "Measurements"). Hence 200 µs and 1,500 ms. A
  buffer that copies its text per edit (a `String`) would need about 10 MB of memmove per edit,
  roughly 1 ms (INFERRED, not measured), so the rope budget still separates a rope from a flat
  string. The implementer records the
  measured median, p99 and max on the bootstrap machine in the log and in `docs/development.md`.
- (h) Evaluator call (`text.rope`): `cargo xtask budgets` prints `PASS budgets` with both
  measured values. That is accepted as the public check for this item (director, R-060 answer 3).
  The evaluator may also run either example itself.

## Acceptance criteria
In `xtask/tests/`, driving the xtask binary in temporary roots with hand-written registries
whose commands are small shell scripts (`sh -c 'printf …'`).
- AC-1 (fuzz judgement): each of these gives FAIL with the pinned reason: exit 0 with a
  exit 1 with a passing line; `failures: 1`; `self_test: "failed"`; `cases` below
  `min_cases`; no JSON line; a 10 s sleep with `timeout_s = 1` (killed; the step returns within
  5 s). A correct line with exit 0 gives PASS. An empty registry gives EMPTY. Oracle: exact
  value.
- AC-2 (budget judgement): `value ≤ max` → PASS; `value > max` on all 3 attempts → FAIL
  listing 3 values; a script that fails twice then meets the budget → PASS on attempt 3; a
  wrong `metric` → FAIL; `"value":"12"` → FAIL. Oracle: exact value.
- AC-3 (config strictness): an unknown key, a duplicate name, `attempts = 0`, `timeout_s = 0`,
  or a missing `command` gives FAIL naming the file and the key. Oracle: exact value.
- AC-4 (`text.rope`, gate CLI): in the repository, `cargo xtask budgets` runs `rope-edit-10mb`
  and `cli-roundtrip-50mb` and passes. Its detail shows both metrics and the attempts. The example's output line has
  `len_chars` ≥ 10,000,000 and `ops: 2000`. Oracle: the enforced budget.
- AC-5 (budget discriminates): the example run against a test-only flat-`String` buffer
  exceeds the registered `max`, or at least its p99 is more than 3× the rope's in the same run.
  This is a test in `crates/kiln-text/tests/` that times both with the same LCG; it is not a
  gate target. Oracle: differential.

## Scorecard items and intended test names
| Item | Tests |
|---|---|
| text.rope | `xtask::acceptance_t028::rope_budget_is_registered_and_passes`, `kiln_text::acceptance_t028::rope_beats_flat_string` |

Also `xtask::acceptance_t028::{fuzz_judgement, budget_judgement, registry_config_is_strict}`.

## Out of scope and implementer notes
- The command-fuzz program and its registration (T-029); further budgets; trend tracking.
- Gate time: the release build of `kiln-text` + `ropey` adds a few seconds cold. Record the
  measured `cargo xtask ci` wall time before and after in `docs/development.md`.
- Your tests: unit tests in `xtask/src/registries.rs`.

## Questions and rulings

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13 · R-056 · (new) · did: specified one runner for both steps, with a strict output
  protocol and a self-test field for fuzz, and the rope budget as its first target. The budget
  is measured at the Buffer level, in a release example in kiln-text, so xtask still depends on
  no crate. The `max` of 250 µs is PROPOSED from the evaluator's measured protocol p99 (0.09 ms
  at 50 MB); the director decides (Q-4). Tier high: gate paths. Depends on T-009 and T-013
  because they own `xtask/src` as a directory in their touches. Unsure: whether
  `cargo xtask budgets` counts as the "CLI" for the per-item AC rule (Q-4). · gate attempts: 0
  · missing context: none · next: director.
- 2026-11-13 · R-060 · draft · did: applied answer 3. Added the second target `cli-roundtrip-50mb` (built CLI, 50 MB open + one edit + save), with an optional `prepare` key so the release binary is built first. Both maxima are about 5× the iteration-0 measurements: median 0.04 ms → 200 µs (metric changed to `median_edit_us`), 0.3 s → 1,500 ms. The purpose (order-of-magnitude regressions on any machine) is stated in the spec. Fixed an AC-1 slip (a correct line with exit 0 was listed among the FAIL cases). Touches gain `apps/kiln-cli/examples/budget_roundtrip.rs`, which is disjoint from T-007/T-011/T-024 (`src/main.rs`). · gate attempts: 0 · missing context: none · next: director.
- 2026-11-13T12:51Z · R-112 · draft · did: `depends_on` [T-009, T-013] → [T-013]. T-009 was split (T-009 queue, T-033 join, T-034 lint and publish); none of the three changes `xtask/src/registries.rs`, `xtask/src/gate.rs` or `xtask/budgets.toml`, and T-028 uses nothing they add, so it needs none of them; T-013 (merged) gave the gate its current steps. The only shared path is `docs/development.md`: a dispatch constraint (never in flight with T-009, T-033, T-034), not a dependency. T-029 (`depends_on: [T-028]`) is unaffected. · gate attempts: 0 · missing context: none · next: director.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
