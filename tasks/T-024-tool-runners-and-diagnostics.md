---
id: T-024
title: "Run cargo and rustfmt behind the tool trait with a recording and a replaying fake, and turn `cargo check --message-format=json` into diagnostics mapped to document positions that follow later edits"
tier: high              # ADR-0002: introduces the spawn path; CLI opt-in flag on the session's trust boundary (running cargo executes project build scripts)
state: draft
owner_role: planner
scorecard_items: [diag.tool-fake, diag.cargo-check, diag.positions]
depends_on: [T-015, T-018, T-032]   # tool contract + slots; anchors; tool kinds
touches:
  - crates/kiln-workspace/src/tools.rs          # ProcessRunner, RecordingRunner, ReplayRunner (contract from T-015 unchanged)
  - crates/kiln-workspace/src/diagnostics.rs    # cargo JSON parsing and position mapping (slot from T-015)
  - crates/kiln-workspace/tests/tools.rs        # implementer's own integration tests
  - crates/kiln-commands/src/tool.rs            # tool.versions (slot from T-015)
  - crates/kiln-commands/src/diag.rs            # diag.check, diag.list + DiagState (slot from T-015)
  - crates/kiln-control/src/lib.rs              # Host::set_tools only
  - apps/kiln-cli/src/main.rs                   # --tools option, usage text
  - apps/kiln-cli/tests/fixtures/t024           # fixture crates and recordings (test author)
  - docs/protocol/tool.md                       # new: runners, --tools, recording format
  - docs/protocol/diag.md                       # new
  - docs/control-protocol.md                    # ONE line in the `kiln-cli` section: `--tools` (director, R-060 answer 4)
  - docs/commands.json                          # generated
budget_tokens: 850000   # high (v1.1)
iteration: 2
found_by: planner
origin_task: null
---

# T-024 Tool runners and cargo-check diagnostics

## Goal
CHARTER §3 decision 4 requires external tools behind a trait with a recorded-output fake. T-015
defined the trait (`ToolRunner`) and the default `NoTools`. This task adds three runners:
- a real runner, the **only** code in Kiln that spawns a process;
- a recording runner, which runs the real tool and appends each request and its output to a
  file;
- a replaying runner, which answers from such a file and never spawns.

`kiln-cli` selects a runner with `--tools`. On top, `diag.check` runs `cargo check
--message-format=json` and stores the diagnostics. Their positions are char offsets and 0-based
line/col in the project's files. For documents that are open and unmodified at check time, the
positions follow later edits (T-018 anchors). `diag.list` reports them. `tool.versions` shows the
tool versions; it is the smallest observable use of a runner. Tests use the replaying runner.
Exactly one marked test per tool spawns the real one.

No gate path. Reading list: CHARTER §3 decision 4; `tasks/T-015-*.md` (tool contract, reasons,
`Parts`, `DiagState`); `tasks/T-018-*.md` (anchors); `tasks/T-022-*.md` "Text of a file";
`apps/kiln-cli/src/main.rs` options; `docs/control-protocol.md` "kiln-cli" (this task adds one
line there for `--tools`; the full text is in `docs/protocol/tool.md`).

## Interface

### Runners (`kiln_workspace::tools`)
```rust
pub struct ProcessRunner { /* root, limits */ }
impl ProcessRunner { pub fn new(root: &std::path::Path) -> ProcessRunner; }
pub struct RecordingRunner { /* inner ProcessRunner, file */ }
impl RecordingRunner { pub fn create(root: &std::path::Path, file: &std::path::Path) -> std::io::Result<RecordingRunner>; }
pub struct ReplayRunner { /* entries, cursors */ }
impl ReplayRunner { pub fn load(file: &std::path::Path) -> Result<ReplayRunner, String>; pub fn from_jsonl(text: &str) -> Result<ReplayRunner, String>; }
// all three: impl ToolRunner (+ Debug)
```
`ProcessRunner::run`:
- Runs `cargo` or `rustfmt`, found on `PATH`, with exactly `request.args`. Not found →
  `ToolError::NotFound`.
- The working directory is `root`.
- The environment is cleared and then given only `PATH`, `HOME`, `CARGO_HOME`, `RUSTUP_HOME`,
  `RUSTUP_TOOLCHAIN` and `TMPDIR` (each only if set), plus `CARGO_TERM_COLOR=never`.
  `CARGO_TARGET_DIR` is never passed.
- stdin is `request.stdin` (then closed), or closed at once. stdout and stderr are read
  concurrently.
- Limits: cargo 600 s, rustfmt 30 s. On expiry the process is killed and reaped →
  `ToolError::Timeout`.
- Output that is not UTF-8 → `ToolError::Failed { message: "output is not UTF-8" }`.

Recording file: JSON lines, one per `run` in call order, sorted keys, written and flushed
after each call. Its first line is created (truncated) by `create`.
`{"request":{"args":[…],"stdin":null|"…","tool":"cargo"|"rustfmt"},"output":{"status":0|null,"stderr":"…","stdout":"…"}}`,
or instead of `output`, `"error":{"kind":"not_found"|"timeout"|"failed","message":"…","seconds":n?}`.
`ReplayRunner::run` answers the n-th call with a given request with the n-th recorded entry for
an equal request, or with the last one once they run out. It re-creates errors as recorded. No
equal request recorded → `ToolError::NoRecording`. A malformed file → `load` error naming the
line number.

### `kiln-cli --tools <mode>` (on `run`, `control`, `exec`): spawning is opt-in (director, R-060)
- **Default `none`, always**, with `--root` and with `--memory`. Without an explicit
  `--tools real` or `--tools record:<file>`, no command starts a process. Every tool command
  then returns `{"code":"rejected","kind":"tool_unavailable","message":"external tools are off for
  this session (start with --tools real to run them)"}` (T-015's `NoTools` Display; with
  `--tools replay:` the recorded result instead).
- `real`: the `ProcessRunner`. `record:<file>`: the `RecordingRunner`, which also runs the real
  tools and so is an opt-in too. `replay:<file>`: the `ReplayRunner`, which never spawns.
- `<file>` is a host path resolved against the current directory, like `--log`.
- `real` or `record:` with `--memory` is a usage error (exit 2). So is an unloadable replay
  file (the message names the line), and so is an unknown mode.
- `Host::in_memory`/`on_disk` keep `NoTools`; only an explicit `set_tools` installs a runner.
- `docs/protocol/tool.md` and the one new line in `docs/control-protocol.md` say: "`--tools
  real` runs `cargo` and `rustfmt` in the project root; `cargo check` executes the project's
  build scripts and procedural macros, so use it only on projects you trust."
- `kiln_control::Host` gains `kiln_control::Host` gains
`pub fn set_tools(&mut self, tools: Box<dyn ToolRunner>)`.

### Commands
| Command | Mutating | Params | Result |
|---|---|---|---|
| `tool.versions` | no | `{}` | `{cargo, rustfmt}`: the trimmed stdout of `cargo --version` and `rustfmt --version` |
| `diag.check` | yes | `{}` | `{success, counts:{error, warning, other}, diagnostics:[Diag]}` |
| `diag.list` | no | `{path?}` | `{diagnostics:[Diag]}`, the stored ones, positions as of now; empty before any check |

`Diag` = `{id, level, message, code, path, start, end, line, col, end_line, end_col, doc,
stale}`. `code` and `path` may be null; positions may be null (see rules).

`diag.check` rules (exact):
1. The request is `ToolRequest { tool: Cargo, args: ["check", "--message-format=json",
   "--workspace", "--all-targets"], stdin: None }`. A `ToolError` → `rejected` with its
   T-015 reason. Exit status non-zero and no `error`-level message parsed → `rejected`/
   `tool_failed`, with the first 200 chars of stderr in the message.
2. Each stdout line that is a JSON object with `"reason":"compiler-message"` yields one candidate
   from its `message`: `level` and `message` as given, `code` = `code.code` or null. The span
   is the first `spans[]` entry with `is_primary: true` whose `file_name` is a valid RelPath
   naming an existing project file. With no such span: `path`, positions and `doc` are null.
   Other lines are ignored.
3. Position: `line = line_start − 1`, `col = column_start − 1`, `end_line = line_end − 1`,
   `end_col = column_end − 1` (cargo's columns are 1-based chars). `start`/`end` are the char
   offsets of those line/cols in the file's text at check time, per T-022's text rule. A
   line/col beyond the text is clamped to it and `stale` set. If the text cannot be read or
   decoded, `start`/`end` are null.
4. Candidates equal in every field but `id` are merged; `--all-targets` reports a lib
   diagnostic twice. The rest are sorted by path (null last, then bytes), `start`, `end`, level
   (`error`, `warning`, then others by name) and message. `id` = 1, 2, … in that order,
   renumbered per check.
5. `doc`: the id of the open document with that path if its text equals the disk text at check
   time. Its `start`/`end` are then anchored in that buffer with T-018 anchors. If the
   document is open but modified, `doc` is null and `stale: true`.
6. `success` = exit status 0. `counts` count levels `error`, `warning`, and the rest.
7. The new set replaces the stored one; the old anchors are removed.

`diag.list`: anchored diagnostics report their current anchor offsets, and line/col computed
from the current text. If their document was closed, they report check-time positions with
`doc: null, stale: true`. With `path`, only that path's are listed.

No new error kind (T-032 added the tool kinds). `docs/protocol/tool.md` and `diag.md` document
all of the above, including the warning in (e).

## Pinned
- (a) `ToolError` → reason: T-032's `From<&ToolError>`. `diag.check` precedence: runner error →
  `tool_failed` (rule 1) → parse (never fails; malformed lines are ignored).
- (b) Files: the recording file (written only in `record:` mode, appended per call); the
  replay file (read once at start); project files (read, as T-022). Cargo itself writes
  `<root>/target`, which T-019's walk skips.
- (c) Order: as in rules 1–7.
- (d) Unix; tool from `PATH`; cwd = the `DiskFs` root. Offsets are chars.
- (e) Threat model: `cargo check` runs the project's build scripts and proc macros. A hostile
  repository therefore runs code as the user, which is inherent to the tool. `docs/protocol/diag.md`
  says so, and nothing spawns without the explicit `--tools real`/`record:` flag (default
  `none`). The reviewer checks that only `ProcessRunner`
  spawns. They also check that no request field can choose the program, args or environment:
  args are fixed by the commands. A tool cannot outlive its limit: it is killed and reaped,
  with no zombie and no stuck reader thread. A huge or non-UTF-8 output is bounded or refused.
  Recording files contain file text (rustfmt stdin), which the docs mention.
- (f) Snapshots: `kiln-cli` usage text gains `--tools`. Any test pinning the usage text (in
  `apps/kiln-cli/tests/acceptance_t004*.rs` or `cli.rs`) is listed by the implementer in the
  log before changing anything. If one pins it exactly, stop and report: that file is not in
  `touches`.
- (g) Worked example (VERIFIED with real cargo 1.97.0 on a scratch crate). `src/lib.rs` =
  `pub fn f() -> u8 {\r\n    let x: u8 = "s";\r\n    x\r\n}\r\n`, checked with `--all-targets`,
  gives two identical `E0308` messages. Each has a primary span `line_start 2, column_start
  17, line_end 2, column_end 20` (and `byte_start 36`, which counts CRLF bytes). After rule 4
  there is one `Diag`: `level "error"`, `message "mismatched types"`, `code "E0308"`, `path
  "src/lib.rs"`, `line 1, col 16, end_line 1, end_col 19`, `start 36`, `end 39`. Line 1 starts
  at char 20, because `pub fn f() -> u8 {\r\n` is 20 chars, and the text is ASCII. If the
  file is open and clean, then after `doc.insert {offset:0, text:"// x\r\n"}` (6 chars),
  `diag.list` gives `start 42, end 45, line 2, col 16`.
- (h) Evaluator calls (real tools only with `--tools real`): `diag.tool-fake` → `kiln-cli run s.jsonl --root fx --tools replay:rec.jsonl`
  with `PATH` set to an empty directory gives the same output twice and spawns nothing.
  `--tools record:` with the real tools produces a file that replays to identical responses.
  `diag.cargo-check` → `diag.check` on a fixture crate, compared with `cargo check
  --message-format=json` run by hand. `diag.positions` → `doc.open`, `diag.check`, `doc.insert`,
  `diag.list`.

## Acceptance criteria
- AC-1 (`diag.tool-fake`, CLI): with `--tools replay:<committed recording>` and `PATH` pointing
  to an empty directory, a script running `tool.versions`, `diag.check` and `diag.list` gives
  byte-identical stdout on two runs, and the recorded results. A request not in the recording →
  `rejected`/`no_recording`. With no `--tools` option, and with `--tools none`, on `--root` and on
  `--memory`: `tool_unavailable` with the exact pinned message, and nothing is spawned (an
  executable `cargo` script on a private `PATH` that writes a marker file is never run). Oracle: exact value +
  differential.
- AC-2 (`diag.tool-fake`, the one marked real-tool test): `#[test] fn real_cargo_and_rustfmt_record_and_replay`.
  It copies a fixture crate and the repository's `rust-toolchain.toml` to a temp dir, then
  runs a script with `--tools record:r.jsonl` (real cargo, timeout 300 s) and with
  `--tools replay:r.jsonl`. The responses are byte-identical. The recording's `stdout` equals
  what `cargo check --message-format=json --workspace --all-targets`, run directly by the test,
  prints in the same dir. Oracle: the real tool.
- AC-3 (`diag.cargo-check`, CLI): for ≥ 4 committed fixture recordings produced by real cargo
  (errors with codes, warnings, a note-only message, a span in a non-project file, a duplicate
  from `--all-targets`, a non-ASCII and a CRLF source), `diag.check` equals a naive model of
  rules 1–7 applied to the recorded JSON. For every single-line primary span, the file text's
  slice `[start, end)` equals `spans[].text[0].text` between `highlight_start` and
  `highlight_end` (1-based chars). Oracle: the compiler's JSON (its own highlighted text).
- AC-4 (`diag.cargo-check`, failure): a recording of cargo exiting 101 with no JSON message (a
  broken `Cargo.toml`) → `rejected`/`tool_failed`, and the stored set is unchanged. A recorded
  `not_found` → `tool_not_found`; a recorded `timeout` → `tool_timeout`. Oracle: exact value.
- AC-5 (`diag.positions`, control): for clean open documents, ≥ 500 fixed-seed random
  `doc.insert`/`doc.delete`/`doc.undo` calls after `diag.check`. After each, `diag.list` gives
  each diagnostic's `start`/`end` equal to a naive model of T-018's mapping rule applied to the
  check-time offsets. `line`/`col` equal a naive conversion of the current text. A document
  modified before the check gives `doc: null, stale: true`. Closing the document gives
  check-time positions with `stale: true`. The worked example holds. Oracle: naive model.
- AC-6 (process hygiene, Rust): `ProcessRunner` with a 1 s limit on a test program that sleeps
  returns `Timeout` within 3 s, and the child is gone. `CARGO_TARGET_DIR` set in the test's
  environment is absent in the child's (checked with a recorded `env`-printing helper script
  placed on a private `PATH` as `cargo`). Oracle: exact value.

## Scorecard items and intended test names
| Item | Tests |
|---|---|
| diag.tool-fake | `kiln_cli::acceptance_t024::replay_is_deterministic_and_spawns_nothing`, `kiln_cli::acceptance_t024::real_cargo_and_rustfmt_record_and_replay` |
| diag.cargo-check | `kiln_cli::acceptance_t024::check_matches_compiler_json`, `kiln_cli::acceptance_t024::check_failures_are_reported` |
| diag.positions | `kiln_cli::acceptance_t024::diagnostics_follow_edits` |

## Out of scope and implementer notes
- `rustfmt` behaviour and `fmt.*` (T-026; it uses this task's runners). Checking unsaved
  buffers. Clippy. Watching. Windows.
- Determinism: the timeout is the only clock use, and it sits inside `ProcessRunner` at the
  external boundary. Replayed sessions are clock-free.
- Your tests: `crates/kiln-workspace/tests/tools.rs` (the helper-program tests use scripts on a
  private `PATH`, never real cargo), unit tests.

## Questions and rulings

## Note from T-038 (director, D-139) — to re-check at dispatch
T-038 merged at `76422d0ed4dbcecb4366dd019f23c282f160cea6` (ADR-0006). Every command this task adds, and every command it makes
report a new refusal kind, declares that kind in its own module (`refusals()`) and gives a refusal example for each
kind that can be shown without a disk (`refusal_examples()`); otherwise the refusal is reported as `internal` and the
audit fails at the join. A message text, the command list and `error_kinds` are not pinned by a test (golden rule 9).
The spec check before dispatch adds the matching acceptance criterion.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13 · R-056 · (new) · did: combined the runners and the diagnostics into one task.
  The fake is meaningless without its first consumer, and the high review then covers both the
  process boundary and the parsing. Tier high, by ADR-0002 "spawns processes". VERIFIED with
  real cargo in a scratch crate (removed): with CRLF, `byte_start` counts CRLF bytes, columns
  are 1-based chars, and `--all-targets` duplicates lib messages. So the positions use
  line/col, and rule 4 merges duplicates. `--tools` is documented in `docs/protocol/tool.md`
  under the R-056 ruling (later R-060: one line in the CLI section is allowed).
  Pinned: none n/a. · gate attempts: 0 · missing context: none · next: director.
- 2026-11-13 · R-060 · draft · did: applied answers 4, 7 and 9. Spawning is opt-in: the default is `--tools none` everywhere, and only `real`/`record:` run tools, with the pinned `tool_unavailable` result otherwise and AC-1 checking that nothing spawns. Added one line to `docs/control-protocol.md` with the build-script warning. Stays high: this task introduces the spawn path. · gate attempts: 0 · missing context: none · next: director.
- 2026-11-13 · R-066 · draft · did: tool kinds and the `ToolError` mapping are T-032's (D-041): `depends_on` gains T-032. · gate attempts: 0 · missing context: none · next: director.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
