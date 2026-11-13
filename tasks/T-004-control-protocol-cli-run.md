---
id: T-004
title: "Drive a headless session with JSON-lines requests: `kiln-cli run <script>` and `kiln-cli control`, deterministically"
tier: high              # two layers (L4 + app); Cargo.toml/Cargo.lock; public CLI/protocol contract
state: tests_joined
owner_role: director
scorecard_items: [control.protocol, control.errors, control.session, cli.run]
depends_on: [T-003]
touches:
  - crates/kiln-control/src
  - crates/kiln-control/Cargo.toml
  - apps/kiln-cli/src
  - apps/kiln-cli/Cargo.toml
  - apps/kiln-cli/tests/run.rs
  - Cargo.toml
  - Cargo.lock
  - docs/control-protocol.md
budget_tokens: 700000
iteration: 0
found_by: planner
origin_task: null
---

# T-004 Drive a headless session with JSON-lines requests: `kiln-cli run <script>` and `kiln-cli control`, deterministically

## Goal
After this task a person or agent can drive Kiln without a UI: write a script of command
requests (one JSON object per line), run it with `kiln-cli run <script>` against a directory, and
get one JSON response per request; or talk to `kiln-cli control` over stdin/stdout with the same
protocol. Running the same script on the same initial files always prints byte-identical output,
and the session's command log can be written out as a script that reproduces the session.

## Reading list
- `docs/CHARTER.md` §3 decisions 1, 2, 4; §4.
- `tasks/T-003-document-commands.md` Interface (commands, `Context::with_vfs`, `LogEntry`,
  `Registry::replay`), as merged; `tasks/T-002-documents-file-io.md` (`DiskFs`, `RelPath`).
- `apps/kiln-cli/src/main.rs` (existing subcommands and exit codes), `apps/kiln-cli/tests/cli.rs`.

## Interface

### Protocol (JSON lines, UTF-8, `\n`-terminated)
Request: `{"id"?: string | integer, "command": string, "params"?: object}` — `params` defaults to
`{}`; any other top-level key is an error. Response, one line per request, compact JSON with
object keys sorted lexicographically at every level, no trailing spaces:
- success: `{"id":<id or null>,"ok":true,"result":<command result>}`
- failure: `{"error":{"code":…,"message":…[,"details":[…]]},"id":<id or null>,"ok":false}`

Error codes: `invalid_json` (line is not JSON or not UTF-8; `id` null), `invalid_request` (not an
object, missing/non-string `command`, non-object `params`, `id` not string/integer, unknown
top-level key; `id` echoed if it was a valid id, else null), and the registry's
`unknown_command`, `invalid_params`, `rejected`, `internal`. Lines that are empty or only
whitespace produce no response. The host never exits or panics because of a request.

### Rust (`kiln_control::…`)
```rust
pub struct Host { /* private */ }
#[derive(Debug)] pub struct HostError(pub String);   // + Display, Error
impl Host {
    pub fn in_memory(files: &[(&str, &[u8])]) -> Result<Host, HostError>;
    pub fn on_disk(root: &std::path::Path) -> Result<Host, HostError>;   // DiskFs at root
    /// One request line in; Some(response line without "\n") out, None for a blank line.
    pub fn handle_line(&mut self, line: &[u8]) -> Option<String>;
    pub fn log(&self) -> &[kiln_commands::LogEntry];
    /// The log as a script: one request per entry, `{"command":…,"id":<1-based index>,"params":…}`,
    /// keys sorted, each line "\n"-terminated.
    pub fn log_script(&self) -> String;
}
```

### CLI (`kiln-cli`)
```text
kiln-cli run <script> [--root <dir>] [--log <file>]
kiln-cli control [--root <dir>]
```
- `--root` (default: current directory) is the DiskFs root; script paths are relative to it.
- `run` reads `<script>` (a protocol request file), writes one response line per request to
  stdout, continues after failed requests, and with `--log` writes `log_script()` to `<file>`
  (a path on the host, not in the VFS) after the last request.
- `control` reads requests from stdin until EOF, writes each response line to stdout and flushes
  it before reading the next request.
- Exit codes: `run`: 0 every response `ok:true`; 3 at least one `ok:false`; 2 usage error,
  unreadable script, root not a directory, or unwritable `--log` file (message on stderr, nothing
  on stdout). `control`: 0 at EOF whatever the responses; 2 usage error / bad root.
- Existing subcommands (`commands`, `exec`) and their exit codes are unchanged.

`docs/control-protocol.md` documents all of the above with one worked example (the evaluator
receives this file, METHODOLOGY §9).

## Acceptance criteria
- AC-1 (session, control.session): in a fresh directory holding `a.txt` = `"hello\n"`, `kiln-cli
  run s.jsonl --root <dir>` with the script
  `{"id":1,"command":"doc.open","params":{"path":"a.txt"}}` /
  `{"id":2,"command":"doc.insert","params":{"doc":1,"offset":5,"text":" world"}}` /
  `{"id":3,"command":"doc.undo","params":{"doc":1}}` /
  `{"id":4,"command":"doc.redo","params":{"doc":1}}` /
  `{"id":5,"command":"doc.save","params":{"doc":1}}` exits 0, prints exactly 5 lines, line 5 is
  `{"id":5,"ok":true,"result":{"bytes_written":12,"dirty":false,"doc":1}}`, and `a.txt` on disk
  is `"hello world\n"`. Oracle: exact value.
- AC-2 (one response per request, ids echoed, control.protocol): for scripts mixing string ids,
  integer ids, missing ids and blank lines, the number of output lines equals the number of
  non-blank input lines and the i-th response's `id` equals the i-th request's id (null when
  absent). Oracle: exact value.
- AC-3 (byte-identical replay): the same script run twice on two fresh copies of the same initial
  directory — at two different absolute paths — prints byte-identical stdout and leaves
  byte-identical files. Holds for scripts that include failing requests and error messages.
  Oracle: differential (run vs run).
- AC-4 (log as script): after `run s.jsonl --log l.jsonl`, running `l.jsonl` on a fresh copy of
  the initial directory leaves byte-identical files, and its own `--log` output is byte-identical
  to `l.jsonl`; `l.jsonl` contains exactly the successful requests of `s.jsonl` in order. Oracle:
  round-trip + idempotence.
- AC-5 (errors, control.errors): each of these lines yields exactly one `ok:false` response with
  the stated code, and the following valid request still succeeds: `not json` → `invalid_json`;
  `[1,2]` → `invalid_request`; `{"id":7}` → `invalid_request`, id 7; `{"id":1.5,"command":"doc.list"}`
  → `invalid_request`, id null; `{"command":"doc.list","params":[]}` → `invalid_request`;
  `{"command":"doc.list","extra":1}` → `invalid_request`; `{"command":"no.such"}` →
  `unknown_command`; `{"command":"doc.insert","params":{"doc":1}}` → `invalid_params`;
  a line of invalid UTF-8 bytes → `invalid_json`. `run` then exits 3. Oracle: exact value.
- AC-6 (control mode): `kiln-cli control --root <dir>` with requests written one at a time to
  stdin returns each response before the next request is written (the test waits for the
  response line before sending the next request), and exits 0 at EOF even after errors.
  Oracle: exact value.
- AC-7 (output format): every response line parses as JSON, re-serializing it with sorted keys
  and no whitespace gives the identical line, and no response contains the absolute root path.
  Oracle: idempotence + exact value.
- AC-8 (CLI errors): missing script file → exit 2, empty stdout; `--root` naming a file → exit 2;
  unknown subcommand → exit 2; `kiln-cli exec buffer.insert …` behaves as before. Oracle: exact value.
- AC-9 (library): `Host::in_memory(&[("a.txt", b"x")])` + `handle_line` gives the same response
  strings as `run` on a directory with the same file, for the AC-1 script. Oracle: differential
  (memory vs disk).

## Scorecard items and intended test names
Tests in `apps/kiln-cli/tests/acceptance_t004.rs` (`kiln_cli::acceptance_t004::<fn>`) and
`crates/kiln-control/tests/acceptance_t004.rs` (`kiln_control::acceptance_t004::<fn>`).

| Item | Tests |
|---|---|
| control.session | `kiln_cli::acceptance_t004::open_edit_undo_redo_save_through_run`, `kiln_control::acceptance_t004::memory_and_disk_hosts_give_same_responses` |
| control.protocol | `kiln_control::acceptance_t004::one_response_per_request_with_ids_echoed`, `kiln_cli::acceptance_t004::control_mode_answers_each_line_before_next` |
| control.errors | `kiln_control::acceptance_t004::malformed_requests_get_structured_errors_and_host_continues`, `kiln_cli::acceptance_t004::run_exits_3_when_a_request_fails` |

| cli.run | `kiln_cli::acceptance_t004::same_script_gives_byte_identical_output`, `kiln_cli::acceptance_t004::log_script_replays_and_is_idempotent` |

Also (no item): `kiln_control::acceptance_t004::responses_are_canonical_json`, `kiln_cli::acceptance_t004::cli_usage_errors_exit_2`.

## Out of scope
- New commands (anything not in the registry after T-003); sessions over sockets; concurrency;
  request batching; a request size limit — later tasks.
- An in-memory `run` mode from the CLI (tests use real temp directories).
- The UI (`apps/kiln`, `kiln-ui`).
- Changes to `kiln-commands`, `kiln-workspace`, `kiln-text`, `apps/kiln-cli/tests/cli.rs`.

## Implementer notes
- `kiln-control` depends on `kiln-commands` and `kiln-workspace`; `kiln-cli` gains `kiln-control`.
  Add `kiln-control` (and `kiln-workspace` if T-003 did not) to `[workspace.dependencies]`.
- Your own tests go in `apps/kiln-cli/tests/run.rs` and in `crates/kiln-control/src` unit tests.
- Sorted keys: check that the workspace's `serde_json` has no `preserve_order` feature enabled
  (it is not today); do not rely on insertion order.
- Determinism: no clock, randomness, thread timing, host paths or environment-dependent text in
  responses; `control` must not depend on stdin timing for its output.

## Decisions
Director's answers to the planner's questions of R-003 (2026-11-12).
- D-1 (was Q-1): the planner added `scorecard/items/cli.run.toml` (director-authorized, `tests =
  []`, tier high, crate `kiln-cli`); this task claims it with the AC-3/AC-4 tests.
- D-2 (was Q-2, accepted as proposed): `run` exits 3 when the script ran and at least one request
  failed; 2 stays usage / unreadable input.
- D-3 (was Q-3, accepted as proposed): `kiln-cli control` (stdin/stdout) is in scope.
- `Cargo.lock` is a generated file under T-005's policy; until T-005 lands the director
  regenerates it on a rebase conflict.
- Director ruling (2026-11-13, decision D-023), part of the spec: `kiln-cli exec` and `kiln-cli run` must be able to act on real files. Both take `--root <dir>` (default: the current directory) and run commands against a disk workspace rooted there; an explicit `--memory` flag selects the empty in-memory file system for tests. Review of T-003 found that `exec` currently starts from an empty in-memory file system on every call, so no document can be opened from the CLI.
- Director rulings at the join (2026-11-13, decision D-025), part of the spec: the AC-9 example file contains `hello\n` (the original `x` made its own offset out of range); an unwritable `--log` path is detected before the script runs (empty stdout, non-zero exit); `exec --text` seeds the legacy single buffer and may be combined with `--root` or `--memory`; `<script>` and `--log` paths are relative to the current directory, not to `--root`; `control` has no `--memory`; a `control` I/O error exits 1; for duplicate JSON keys in a request the last one wins.

## Questions
None open.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · tokens (if known) · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-12 · R-003 · (new) · did: specified the JSON-lines request/response format with
  canonical (sorted-key) output, error codes, `kiln_control::Host`, `kiln-cli run` and
  `kiln-cli control` with exit codes, and log-as-script; claimed `control.protocol`,
  `control.errors`, `control.session`. Oracles: exact values, run-vs-run and memory-vs-disk
  differentials, log replay round-trip + idempotence. Tier high: L4 + app, `Cargo.toml`/
  `Cargo.lock`, a public protocol. Unsure: missing `cli.run` item (Q-1), exit code 3 (Q-2),
  `control` mode in scope (Q-3). · gate attempts: 0 · not run: gate (no code) · missing context:
  none · next: director review.
- 2026-11-12 · R-003 · draft · did: applied the director's answers (D-1…D-3): created
  `scorecard/items/cli.run.toml` and regenerated `docs/scorecard.md` (`cargo xtask scorecard`);
  added `cli.run` to `scorecard_items` with its two tests; exit code 3 and `control` mode
  confirmed. · gate attempts: 0 · next: director review.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
