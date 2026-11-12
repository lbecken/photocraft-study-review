# Bootstrap verification

Demonstrations of the iteration-0 tooling, run on 2026-11-12 in a throwaway repository
(`$DEMO` = a scratch directory: a copy of this working tree, `git init -b main`, one commit
`34aaabd`). Nothing here touched this repository's history. Output is real and trimmed: long
paths are replaced by `$DEMO`, repeated PASS lines are elided (`…`).

The five required demonstrations are 1–5. Section 6 covers the test join, and section 7 lists
the defects these runs found in the tooling (all fixed in this tree).

## 0. Cold gate

```
$ time cargo xtask ci          # fresh clone, empty target/
gate: 10 steps: fmt, clippy, layers, ui-commands, schema, scorecard, test, no-skip, fuzz, budgets
PASS  fmt             0.1s  formatted
PASS  clippy         10.0s  no warnings (all targets, -D warnings)
PASS  layers          0.1s  11 crates, 3 internal edges, layer table holds
PASS  ui-commands     0.0s  self-test ok; 1 kiln-ui file(s) scanned, 4 allowlisted path(s)
PASS  schema         16.0s  2 commands; audit ok; docs/commands.json current
PASS  scorecard      14.4s  62 items: 0 verified, 6 implemented, 0 failing, 56 planned.
PASS  test            0.5s  86 passed, 0 failed, 0 ignored in 20 suites
PASS  no-skip         0.1s  self-test ok; 31 files scanned, 0 allowlisted skip(s), today = 2026-11-12
EMPTY fuzz            0.0s  no targets registered yet (xtask/fuzz.toml)
EMPTY budgets         0.0s  no targets registered yet (xtask/budgets.toml)
GATE PASSED in 41.4s (10 steps; 2 step(s) EMPTY (nothing registered): fuzz, budgets)
real    0m51.305s
```

## 1. A passing task is merged and `main` fast-forwards

```
$ cargo xtask worktree new T-001
worktree: $DEMO/.worktrees/T-001
branch:   task/T-001
target:   $DEMO/.worktrees/T-001/target (seeded from main's target/)
use:
  cd $DEMO/.worktrees/T-001
  export CARGO_TARGET_DIR=$DEMO/.worktrees/T-001/target
# in the worktree: add test `insert_empty_text_is_noop` to kiln-text, commit (Task: T-001)
$ git rev-parse --short main
34aaabd
$ time cargo xtask queue T-001
gate: 10 steps: …
PASS  test            0.6s  87 passed, 0 failed, 0 ignored in 20 suites
…
GATE PASSED in 7.4s (10 steps; 2 step(s) EMPTY (nothing registered): fuzz, budgets)
queue T-001 attempt 1: merged (base 34aaabd1fd9b, head 631da5a4457b, gate 10.0s)
main fast-forwarded to 631da5a4457b7447a792129b10d0e7ad0e76595f (not pushed)
real    0m10.123s
$ echo $?
0
$ git log --oneline -2
631da5a kiln-text: test that inserting empty text is a no-op
34aaabd bootstrap snapshot (throwaway demo clone)
$ cat metrics/queue.jsonl
{"task":"T-001","attempt":1,"started_at":"2026-11-12T22:22:05Z","ended_at":"2026-11-12T22:22:15Z","base_sha":"34aaabd1fd9bd65cfe4c4fafe96e98ada88eb4e6","head_sha":"631da5a4457b7447a792129b10d0e7ad0e76595f","result":"merged","failed_step":null,"gate_seconds":10.0}
```

## 2. A task that breaks a test is rejected; `main` does not move

```
# T-002: in Buffer::insert, `self.len_chars += text.chars().count()` → `text.len()` (bytes)
$ git rev-parse --short main
f81f894
$ cargo xtask queue T-002
…
PASS  schema          1.1s  2 commands; audit ok; docs/commands.json current
FAIL  scorecard       3.8s  4 scorecard problems:
        - cmd.registry: test `kiln_commands::buffer::tests::insert_then_text_round_trip` fails
        - text.delete: test `kiln_text::buffer::tests::random_edits_match_naive_model` fails
        - text.insert: test `kiln_text::buffer::tests::random_edits_match_naive_model` fails
        - docs/scorecard.md is stale; run `cargo xtask scorecard` and commit it
GATE FAILED in 6.7s: scorecard; 4 later step(s) not run
queue T-002 attempt 2: gate_failed at scorecard (base f81f8940533c, head dc9858d24b06, gate 7.3s)
$ echo $?
1
$ git rev-parse --short main
f81f894
```

The scorecard step runs the tests that items list, so it catches the break before the `test`
step. (Attempt 1 of T-002 failed at `fmt` for a different reason — see section 7.)

## 3. A conflicting task is rejected; the rebase is aborted

```
# T-003 and T-004 both rewrite line 1 of README.md; T-004 is queued first.
$ cargo xtask queue T-004 | tail -2
queue T-004 attempt 1: merged (base f81f8940533c, head 341d62ea0d76, gate 10.6s)
main fast-forwarded to 341d62ea0d76de8df2516ac3bacbf8c584ab754d (not pushed)
$ cargo xtask queue T-003
queue: rebase of task/T-003 onto main conflicts; aborted:
error: could not apply b22e876... docs: retitle README (T-003)
…
queue T-003 attempt 1: conflict at rebase (base 341d62ea0d76, head b22e8762d206, gate 0.0s)
$ echo $?
1
$ git rev-parse --short main
341d62e
$ (cd .worktrees/T-003 && git status --short && git log --oneline -1)
b22e876 docs: retitle README (T-003)
$ tail -1 metrics/queue.jsonl
{"task":"T-003","attempt":1,"started_at":"2026-11-12T22:23:51Z","ended_at":"2026-11-12T22:23:51Z","base_sha":"341d62ea0d76de8df2516ac3bacbf8c584ab754d","head_sha":"b22e8762d20672d532125fa8b9c21ef021778af5","result":"conflict","failed_step":"rebase","gate_seconds":0.0}
```

## 4. An upward dependency fails `layers`

```
# T-005: crates/kiln-text/Cargo.toml gains `kiln-control = { path = "../kiln-control" }`
$ git diff --stat
 Cargo.lock                  | 3 +++
 crates/kiln-text/Cargo.toml | 3 +++
$ cargo xtask layers
FAIL  layers: 1 layer violations:
  - `kiln-text` has a dependency on `kiln-control`: upward dependency (L0 -> L4)
$ echo $?
1
$ cargo xtask queue T-005 | grep -E "FAIL|GATE|queue"
FAIL  layers          0.1s  1 layer violations:
GATE FAILED in 1.6s: layers; 7 later step(s) not run
queue T-005 attempt 1: gate_failed at layers (base 341d62ea0d76, head 918aa34d6d90, gate 1.7s)
$ git rev-parse --short main
341d62e
```

## 5. An `#[ignore]` fails `no-skip`

```
# T-006: adds `#[test] #[ignore = "flaky on CI"] fn huge_insert_is_fast()` to kiln-text
$ cargo xtask no-skip
FAIL  no-skip: 1 skip problems:
  - crates/kiln-text/src/buffer.rs:188: `tests::huge_insert_is_fast` is a skipped test (ignore); remove the skip or list it in xtask/skips.toml with a reason and expiry
$ echo $?
1
$ cargo xtask queue T-006 | grep -E "PASS  test|FAIL|GATE|queue|- "
PASS  test            0.5s  87 passed, 0 failed, 1 ignored in 20 suites
FAIL  no-skip         0.1s  1 skip problems:
        - crates/kiln-text/src/buffer.rs:188: `tests::huge_insert_is_fast` is a skipped test (ignore); …
GATE FAILED in 7.4s: no-skip; 2 later step(s) not run
queue T-006 attempt 1: gate_failed at no-skip (base 341d62ea0d76, head f3e0d6feba79, gate 7.4s)
$ git rev-parse --short main
341d62e
```

`cargo test` alone passes here (1 ignored); only `no-skip` rejects it.

## 6. Test join and revert check (real cargo)

### 6a. Base evidence `fails`; acceptance failure goes back to the implementer; revert check

```
$ cargo xtask worktree new T-012 && cargo xtask worktree new T-012 --tests
# test author: crates/kiln-text/tests/acceptance_t012.rs asserts the out-of-bounds message
#   contains "valid: 0..=2" (compiles on base, fails there)
# implementer's first edit did not apply (nothing committed)
$ cargo xtask join T-012
{ "kind": "join", "task": "T-012", "at": "2026-11-12T22:30:03Z", "acc_total": 1,
  "acc_fail_base": 1, "base_fail_kind": "fails", "acc_fail_first_join": 1,
  "acc_defects": null, "impl_tests_green": true, "gate_green": null, …,
  "result": "acceptance_failed" }
$ echo $?
1
$ cargo xtask queue T-012 | tail -1
queue T-012 attempt 1: gate_failed at test (base 2b3de1809249, head 27f2dc6a2199, gate 6.4s)
# implementer fixes the message and commits; director re-joins
$ cargo xtask join T-012
GATE PASSED in 2.3s (10 steps; 2 step(s) EMPTY (nothing registered): fuzz, budgets)
{ "kind": "join", "task": "T-012", "at": "2026-11-12T22:30:41Z", "acc_total": 1,
  "acc_fail_base": 1, "base_fail_kind": "fails", "acc_fail_first_join": 0,
  "acc_defects": null, "impl_tests_green": true, "gate_green": true, …, "result": "joined" }
$ cargo xtask join --revert-check T-012
{ "kind": "revert_check", "task": "T-012", "at": "2026-11-12T22:30:50Z",
  "reverted": ["ac62289ca1a833f32a8639ff1ab0067a03550711"],
  "acc_total": 1, "acc_fail": 1, "result_kind": "fails" }
$ cargo xtask queue T-012 | tail -2
queue T-012 attempt 2: merged (base 2b3de1809249, head ac62289ca1a8, gate 1.8s)
main fast-forwarded to ac62289ca1a833f32a8639ff1ab0067a03550711 (not pushed)
```

### 6b. Tests that cannot compile on base are recorded as weak evidence

```
# T-010: acceptance test calls the new `Buffer::clear()`; implementation in two commits
#   (API stub returning "", then the logic)
$ cargo xtask join T-010
GATE PASSED in 3.9s (…)
{ "kind": "join", "task": "T-010", "acc_total": 1, "acc_fail_base": 1,
  "base_fail_kind": "does_not_compile", "acc_fail_first_join": 0, "impl_tests_green": true,
  "gate_green": true, …, "result": "joined" }
join: base evidence is `does_not_compile` (weak); the reviewer must run `cargo xtask join --revert-check T-010`
$ cargo xtask join --revert-check T-010                     # reverts stub + logic
{ …, "reverted": ["316fa1fd…", "0e8dbef9…"], "acc_total": 0, "acc_fail": 0,
  "result_kind": "does_not_compile" }
revert check: acceptance tests do not fail by assertion with the implementation reverted (does_not_compile)
$ echo $?
1
$ cargo xtask join --revert-check T-010 --revert 316fa1fd95263eaae873ade008e39f528e8a7b4d   # logic only
{ …, "reverted": ["316fa1fd95263eaae873ade008e39f528e8a7b4d"], "acc_total": 1, "acc_fail": 1,
  "result_kind": "fails" }
$ echo $?
0
```

### 6c. Tests that pass on base are refused

```
# T-009: acceptance test for Buffer::append — which main already had (T-007 had merged it)
$ cargo xtask join T-009
xtask: acceptance tests pass on base, so they prove nothing; join undone. Passing on base: acceptance_t009::ac1_append_adds_text_at_the_end
$ echo $?
2
```

### 6d. Metrics

```
$ cargo xtask metrics add --task T-012 --run R-001 --role implementer --tier medium --iteration 0 --tokens 12000 --tool-calls 9 --gate-attempts 1 --outcome completed --started-at 2026-11-12T22:00:00Z --ended-at 2026-11-12T22:20:00Z
{"kind":"run","task":"T-012","iteration":0,"run":"R-001","role":"implementer","tier":"medium","started_at":"2026-11-12T22:00:00Z","ended_at":"2026-11-12T22:20:00Z","wall_s":1200,"tokens":12000,"tool_calls":9,"gate_attempts":1,"outcome":"completed","missing_context":"none"}
$ cargo xtask metrics transition --task T-012 --from specified --to ready --at 2026-11-12T21:50:00Z --iteration 0
{"kind":"transition","task":"T-012","iteration":0,"at":"2026-11-12T21:50:00Z","from":"specified","to":"ready"}
$ cargo xtask metrics report
runs: 1 over 1 tasks
tokens by tier (all runs):
  medium       12000 tokens, 1 task(s)
first-pass queue success: 5/12 tasks (42%); queue runs 14; merged tasks 6
tokens per merged task by tier:
  unknown          0 (n=6)
lead time (ready -> merged): no data; 6 merged task(s) lack a ready transition
…
```

(The report ran before T-012 attempt 2 merged, hence no lead time yet; demo tasks have no task
files, hence tier `unknown`.)

### Lint probe

```
# a non-test `pub fn probe(s: &str) -> usize { s.parse::<usize>().unwrap() }` in kiln-text
$ cargo xtask clippy
FAIL  clippy: … error: used `unwrap()` on a `Result` value
```

## 7. Defects found by these runs (fixed in this tree)

| # | Found in | Defect | Fix |
|---|---|---|---|
| 1 | demo 2, first try | The queue refused a clean main as dirty: `git status --porcelain` output was trimmed, eating the leading space of ` M metrics/queue.jsonl`, so the `metrics/` exemption missed it. Unit tests had only untracked files. | `dirty_paths` reads untrimmed output; test covers a modified tracked `metrics/` file. |
| 2 | demo 2, attempt 1 | The fix for #1 was committed directly to the demo's `main`, unformatted; the next queue run failed at `fmt` on main's own content. A direct commit made main red — the case the queue exists to prevent (D-005). | Formatted; later fixes in the demo went through the queue (T-008, T-011). |
| 3 | T-007 | `join --revert-check` on a task whose tests branch had no commits reported `result_kind: "fails"` with 0 tests — a vacuous pass. | An empty run is `no_acceptance_tests`, never evidence; revert-check refuses a tests branch with no commits. Unit-tested. |
| 4 | T-010 | The revert check built the reverted tree into the task worktree's target dir; afterwards cargo considered the task's artifacts fresh (mtime-based) and ran the stub build, so queue attempt 1 failed at `test` on correct code. | The revert check seeds and uses a private target dir; never share a target dir between checkouts (`docs/development.md`). |
| 5 | T-012 setup | Seeding every worktree with a full copy of `target/` (1.6 GB each, no reflinks here) filled the scratch disk after ~14 worktrees. | Documented; `worktree rm` after merge; `--no-seed` available. |
