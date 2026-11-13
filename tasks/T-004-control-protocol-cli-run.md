---
id: T-004
title: "Drive a headless session with JSON-lines requests: `kiln-cli run <script>` and `kiln-cli control`, deterministically"
tier: high              # two layers (L4 + app); Cargo.toml/Cargo.lock; public CLI/protocol contract
state: queued
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
- Director rulings after review r1 (2026-11-13, decision D-028) — part of the spec:
  1. Symlinks: today request paths that pass through a symlink can leave the root. Confinement is a change to `kiln-workspace` and is out of this task's `touches`; it becomes task T-008. In this task, `docs/control-protocol.md` must say exactly what happens today (lexical `..` and absolute paths are refused; symlinks are followed, including out of the root; confinement is tracked as T-008) — the document may not claim a guarantee the code does not give.
  2. `kiln-cli --help`, `-h`, and `<subcommand> --help` print usage and exit 0.
  3. `run` exits 1 when it cannot write to stdout, the same as `control`.
  4. `exec` is one command in a fresh session: it can read files but an edit made with it is never saved. The usage text and the protocol document say so and point to `run` / `control`.
  5. Error responses carry a stable machine-readable `kind`; operating-system error text may appear only in the human-readable `message`.
  6. A value echoed back in an error message is truncated to at most 200 characters.
  7. The bad-root error names the path.
  8. Tests that spawn `kiln-cli` have a timeout.
- Director rulings after review r2 (2026-11-13, decision D-031) — part of the spec; last round of rulings for T-004:
  9. Ruling 6 is replaced by size caps that do not depend on where a value came from: in an error response every string — `message` and every string anywhere inside `details` — is at most 200 characters (truncated with a trailing `…`), and the whole error line is at most 4096 bytes; if it would be longer, `details` is replaced by `{"truncated":true}`. The same caps apply to error text on stderr. No comparison against earlier requests or the session log.
  10. The set of error kinds listed in `docs/control-protocol.md` is the specified set. Errors printed by `exec` carry a `kind` too.
  11. `docs/control-protocol.md` states that offsets are character offsets, and that `--help` exits 0 when its output can be written.
  12. A usage error names what was wrong (the unknown flag or the missing argument).
  13. A command that hangs on a non-regular file (for example a FIFO) is out of scope here and goes to T-008 with symlink confinement. The panic-to-`internal` path stays untested through the public interface (accepted; a command fuzzer is planned).

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
- 2026-11-13 · R-026 · in_progress · did: restated ACs: (1) `run` drives open/insert/undo/
  redo/save on a real file, exact line 5 and `hello world\n` on disk; (2) one response per
  non-blank line, ids echoed or null; (3) same script on two copies at different paths gives
  identical stdout and files, errors included; (4) `--log` writes the successful requests as a
  renumbered script whose replay reproduces files and log; (5) the listed malformed lines get
  the listed codes and the host continues, `run` exits 3; (6) `control` answers and flushes
  each line before reading the next, exit 0 at EOF; (7) responses are canonical sorted-key
  compact JSON with no root path; (8) usage/missing script/file root exit 2, `exec` unchanged;
  (9) `Host::in_memory` and disk give the same responses. Built `kiln_control::Host` (request
  validation, own canonical serializer that sorts keys regardless of `serde_json` map type,
  `catch_unwind` turning a panicking command into `internal`, `log_script`), `kiln-cli run` /
  `control`, and D-023: `exec` and `run` take `--root <dir>` (default `.`) or `--memory`
  (mutually exclusive); options may come in any order. Commits: API stubs (ee32218), Host
  logic (543cf55), CLI (c035d23), doc (`docs/control-protocol.md`). Tests: 10 unit tests in
  kiln-control, 4 new in kiln-cli `main.rs`, 7 in `apps/kiln-cli/tests/run.rs`.
  End-to-end by hand in a temp dir (`target/e2e`, `proj/a.txt` = `hello\n`, the AC-1 script
  plus one bad request): `kiln-cli run s.jsonl --root proj --log l.jsonl` → 6 lines, line 6
  `{"id":5,"ok":true,"result":{"bytes_written":12,"dirty":false,"doc":1}}`, exit 3; `od -c
  proj/a.txt` → `h e l l o   w o r l d \n`; `l.jsonl` = 5 renumbered requests (bad one
  absent); `kiln-cli control --root proj` piped open+text → `"text":"hello world\n"`, exit 0;
  `kiln-cli exec doc.open '{"path":"a.txt"}' --root proj` → `{…"doc":1,…"len_chars":12…}`,
  exit 0; no `.kiln-tmp-*` left in `proj`. Full transcript is the worked example in
  `docs/control-protocol.md`.
  Choices the spec left open: `<script>` and `--log` are host paths relative to the cwd, not
  to `--root` (I read "script paths are relative to it" as paths inside requests); blank =
  any Unicode white space; `exec --root` with `--text` seeds the scratch buffer through a
  `buffer.insert` command because `Context` has no constructor taking both a VFS and text;
  `control` takes only `--root` (D-023 names `exec` and `run` for `--memory`); `control` exits
  1 on a stdin/stdout I/O error (spec silent); the log file is created/truncated before the
  first request so an unwritable log is refused with no side effects; duplicate JSON keys in a
  request follow `serde_json` (last wins), not rejected. · gate attempts: 1 (last: green) ·
  not run: acceptance tests (not visible to me); no test of a panicking command (none exists
  in the registry); Windows/macOS · tokens: unknown · missing context: none · next: join with
  the test author's branch.
- 2026-11-13 · R-035 · in_progress (review r1 request-changes, D-028) · did: (1) rewrote
  `docs/control-protocol.md` "Paths": lexical refusals listed, symlinks followed including
  out of the root, confinement tracked as T-008; also removed other over-claims (determinism
  now says OS error text in `message` can differ between platforms; "never panics" → panics
  reported as `internal`/`panic`, document may be partly changed; log replay reproduces files
  only if the same requests succeed again; no request size limit stated). (2) `--help`, `-h`,
  `<subcommand> --help|-h` print usage on stdout, exit 0 (not when `-h` is the value of
  `--root`/`--log`/`--text`). (3) stdout write failure → exit 1 with a stderr message, for
  every Outcome-based subcommand (run, exec, commands), not only `run`. (4) usage text and doc
  say `exec` is one command in a fresh session, edits never saved, use `run`/`control`.
  (5) every error object now has `kind` (23 `(code, kind)` pairs, table in the doc; a unit test
  checks the doc lists them all). `rejected` kinds are derived in kiln-control from the
  lower-layer message (anchored on the known request/document path), falling back to `other`.
  (6) echo clipping to ≤200 chars (199 + `…`) in `message` and `details`: request strings and
  keys in raw/JSON/Rust-escaped form, the value a schema detail points at, the open document's
  path, and as a backstop any >200-char run of non-white-space. (7) bad-root errors name the
  path (`kiln-cli: bad --root: root `<p>`: not found|not a directory`; `HostError` too).
  (8) every spawn in `apps/kiln-cli/tests/run.rs` has a 60 s timeout (try_wait loop + kill;
  control test reads lines through a channel). No new public API. Re-ran the worked example
  in a temp dir: stdout, `l.jsonl`, `a.txt` (`hello world\n`), exit 3 and the control
  transcript match the document (stdout lines compared with `diff`). Commit ac24a4c.
  · gate attempts: 1 (last: green, 278 tests) · not run: the test author's new D-028 tests
  (not visible); a panicking command (SG-2, still untestable); non-Linux; permission-denied
  `io_error` on disk (sandbox runs as root; covered via ENAMETOOLONG instead) · missing
  context: the kind vocabulary and what counts as an "echoed value" were not specified, I chose
  them as above · next: join and review.
  Concern for the reviewer: `kind` for `rejected` is parsed from message text because
  `CommandError::Rejected` carries no structured reason (kiln-commands is outside touches); a
  later change to a lower-layer message would silently turn its kind into `other`. A
  structured reason in kiln-commands would be the robust fix (new task). Residual clipping gap:
  a >200-char value containing white space that is echoed by a message of a later request and
  is neither in that request nor the open document's path is not clipped.
- 2026-11-13 · R-039 · in_progress (join r2: 26/27, `huge_values_are_echoed_truncated_to_200_chars`
  failed) · did: cause: clipping lived in the host's error builder and used only the current
  request (and the open document's path), so `kiln-cli exec`, which builds its stderr error
  without the host, echoed 100000 chars, and a long value with spaces echoed by a later request
  was missed. Now two public functions in kiln-control, `clip_error(&mut error, sources)` and
  `clip_text(text, sources)` (API committed with stub bodies first, 8436026, then logic), are
  applied at the exits only: `Host::handle_line` clips every error response before it is
  serialized, with sources = the line (parsed, or raw if not JSON) plus the params of every
  entry of the session log; `kiln-cli`'s `finish` passes all stderr through `bound_stderr`
  (JSON error lines → `clip_error`, other lines → `clip_text`, sources = argv as given and
  parsed as JSON); `control` now returns its outcome to `finish` too. Producers no longer clip.
  Text of ≤200 chars is returned untouched; the template around a clipped value is kept.
  Unit tests: kiln-control (later-request echo of a 300+-char path with spaces via a failed
  save; values without spaces, with spaces, with escaped quotes in message and details; template
  kept); kiln-cli (exec message + details, run and control responses alike, CLI text errors for
  script path and `--root` on run and exec), all checking the decoded strings. Doc paragraph on
  clipping rewritten. · gate attempts: 1 (last: green, 292 tests) · not run: non-Linux ·
  missing context: none · next: review. Note: on an error with a >200-char message, clipping
  scans the whole session log (cost proportional to the log, only on such errors); an echo that
  is a transformed or partial copy of an input (other than JSON/Rust escaping or a schema
  pointer's compact JSON) is caught only by the >200 non-white-space-run backstop.
- 2026-11-13 · R-043 · in_progress (review r2 request-changes, D-031) · did: replaced the
  echo matching (sources, session-log scan, forms, run backstop; the R-039 `clip_error` /
  `clip_text` API is removed) by plain caps: `kiln_control::cap_error` cuts every string in
  `message` and anywhere inside `details` to 200 chars (199 + `…`, on `char`s, so no
  multi-byte split; operates on `Value`, so JSON stays valid), then replaces `details` by
  `{"truncated":true}` if the compact line exceeds 4096 bytes; `cap_text` caps each stderr
  text line to 200 chars. Applied in `Host::handle_line` (every error response) and in kiln-cli
  `bound_stderr` (all stderr). New API committed as stubs first (48519c7), logic in fe0a570.
  `exec` errors now come from `kiln_control::command_error_json` (same object as a protocol
  error, with `kind`, no `id`; params-not-JSON gets `invalid_json`/`syntax`). Usage errors print
  `kiln-cli: <what was wrong>` (unknown option, option twice, missing value, `--root` with
  `--memory`, missing/unexpected argument, unknown/missing subcommand) before the usage text.
  Doc: caps section, kinds table declared the specified set, `exec` error object, character
  offsets (verified `a\r\né😀` → `len_chars` 5), `--help` exit 0 only when stdout is
  writable (verified `>/dev/full` → 1), usage-error format. R2-D1 case by hand: `exec doc.list`
  with 2000 short strings → 358 bytes on stderr (was 16 112). Unit tests: `cap_string` on
  2/3/4-byte chars and a cut right after a multi-byte char; nested details; line-size
  replacement (ASCII and 4-byte); `cap_text` per line; every Host error line capped (incl.
  arrays/objects of short strings, 300 unknown keys, invalid JSON with é); `command_error_json`
  kind; CLI: exec/run/control/text errors capped with values without/with spaces, escapes and
  multi-byte; usage reasons for 11 cases. · gate attempts: 1 (last: green, 294 tests) · not
  run: the reviewer's long-session timing (the per-error log scan is gone, so I expect it fixed
  but did not measure); non-Linux · missing context: none · next: join and review. Note: a
  valid `id` is echoed in full and not capped, so a multi-KB string id can still make an error
  line exceed 4096 bytes (documented); keys inside `details` objects are not capped (none
  occur today).
<!-- end implementer log -->
### test author
- 2026-11-13 · R-027 · in_progress · did: wrote `apps/kiln-cli/tests/acceptance_t004.rs` (13 tests,
  binary as subprocess on real temp dirs, compile on base, all fail by assertion: base exits 2
  with usage), `crates/kiln-control/tests/acceptance_t004.rs` (6 tests on the declared `Host` API,
  `compile` on base; uses only std + kiln_control with a small in-file JSON parser because the
  crate's dependencies are not fixed by the spec), `apps/kiln-cli/tests/guard_t004.rs` (2
  regression guards for `exec`/`commands`, pass on base by design); set `tests` of
  control.session, control.protocol, control.errors, cli.run. Criterion → tests → base:
  AC-1 → cli `open_edit_undo_redo_save_through_run` (exact line 5, bytes on disk) → fail assertion;
  D-023 → cli `run_root_defaults_to_current_directory`, `run_memory_flag_uses_empty_in_memory_fs`,
  `exec_opens_real_files_under_root` → fail assertion;
  AC-2 → cli `run_one_response_per_request_with_ids_echoed`, control
  `one_response_per_request_with_ids_echoed` → assertion / compile;
  AC-3 → cli `same_script_gives_byte_identical_output` (two roots of different depth, 7 failing
  requests incl. path errors) → assertion;
  AC-4 → cli `log_script_replays_and_is_idempotent` (exact log text, replay files, log
  idempotence, results equal), `log_of_all_failing_script_is_empty`; control
  `log_script_lists_successful_requests_with_1_based_ids` → assertion / compile;
  AC-5 → cli `run_exits_3_when_a_request_fails` (all 9 listed lines + 8 implied edge lines, each
  followed by a request that must succeed with unchanged state), `run_survives_hostile_lines`;
  control `malformed_requests_get_structured_errors_and_host_continues` → assertion / compile;
  AC-6 → cli `control_mode_answers_each_line_before_next` (reader thread, waits ≤60 s per
  response before writing the next; EOF → exit 0, file saved) → assertion;
  AC-7 → cli `run_output_is_canonical_json` (+ checks inside AC-3 test), control
  `responses_are_canonical_json` → assertion / compile;
  AC-8 → cli `cli_usage_errors_exit_2` (12 error invocations, each exit 2, empty stdout, stderr
  non-empty, plus positive controls that exit 0 so the test fails on base), guard
  `exec_buffer_commands_behave_as_before`, `commands_subcommands_exit_codes_unchanged` → pass
  (guards);
  AC-9 → control `memory_and_disk_hosts_give_same_responses`, `host_constructors_reject_bad_roots`
  → compile. Edge cases: blank / whitespace-only / lone-CR lines, string/negative/zero/2^53+1/
  empty/escaped ids, duplicate ids, explicit `"id":null`/bool/array/object ids (→ invalid_request,
  id null), `params: null`, non-string `command`, trailing garbage after JSON, invalid UTF-8 inside a
  string, 100k-deep nesting, 1 MiB command name, NUL bytes, out-of-range offset (`rejected`),
  `../` and absolute paths, missing file, CRLF file, non-ASCII and control characters in text.
  The kiln-control file was compile-checked against a local stub of the declared API (not kept).
  QUESTION: AC-9 says `Host::in_memory(&[("a.txt", b"x")])` with the AC-1 script, but AC-1's
  insert at offset 5 is out of range for "x"; I test the literal wording (memory = disk, request 2
  fails on both) and also the evident intent with "hello\n". Proposed wording: use
  `b"hello\n"`. QUESTION: "unwritable `--log` … nothing on stdout" — the log is written after the
  last request, so an implementation must check the log path before running (or buffer stdout); I
  test the spec literally (exit 2, empty stdout, no log file). QUESTION: D-023 does not say how
  `--root`/`--memory` combine with `exec --text` or with each other, or flag order for `exec`; I
  only test `exec <name> <json> --root <dir>`, `--memory`, and the cwd default. Assumptions
  tested: explicit `"id":null` is `invalid_request` (spec: id must be string/integer);
  read-only successful requests are logged (T-003 log); `log_script` params keys sorted at every
  level; params omitted are logged as `{}`. · gate attempts: 1 (last: red clippy, scorecard,
  test — all only the kiln-control acceptance file not compiling, as expected for `compile`) ·
  not run: nothing else · missing context: none · next: join.
- 2026-11-13 · R-029 · join round 1 · did: fixed my defect in control
  `one_response_per_request_with_ids_echoed` (7 non-blank requests, not 8; last index is the
  `doc.insert`); dropped the literal `b"x"` AC-9 variant (director ruling); added cli
  `exec_text_combines_with_root_and_memory` (D-023 ruling on `exec --text`) → fails on base by
  assertion. Fixed test passes against a throwaway id-echoing stub (not kept); control file is
  still `compile` on base. `--log` test keeps exit 2 as in the Interface. · gate attempts: 0 ·
  not run: gate (only test files changed) · missing context: none · next: re-join.
- 2026-11-13 · R-030 · join round 2 · did: fixed `clippy::err_expect` in control
  `host_constructors_reject_bad_roots` (`let Err(e) = … else { panic! }`; `expect_err` would need
  `Host: Debug`, which the spec does not declare). Ran `cargo clippy --all-targets -- -D warnings`
  on a throwaway package (stub `Host` + copies of all three T-004 test files, workspace lints and
  clippy.toml): clean; a planted `.err().expect()` probe was caught, so the check reaches the
  file. · gate attempts: 0 · not run: full gate (base cannot compile the control file) · missing
  context: none · next: re-join.
- 2026-11-13 · R-036 · review r1 (D-028) · did: escape cases now name real files outside the
  root: `mixed_script` uses `../outside.txt` and `sub/../../outside.txt` with a real
  `../outside.txt` seeded next to every root, and AC-3/AC-4/AC-7 tests assert both refusals and
  unchanged outside bytes; new cli `run_refuses_paths_outside_root_and_writes_nothing_there`
  (`..`, `sub/../..`, `./..`, absolute path to an existing file, each followed by insert + save;
  outside bytes unchanged, no new files beside the root) → fail assertion on base; control
  `responses_are_canonical_json` now uses a real `../up.txt` and a real absolute file. Every
  spawned kiln-cli in my files now has a 120 s timeout (poll + kill). New
  `apps/kiln-cli/tests/acceptance_t004_r2.rs` (all 6 fail on base by assertion):
  `help_flags_print_usage_and_exit_0` (r2), `run_and_control_exit_1_when_stdout_cannot_be_written`
  (r3, stdout = /dev/full, with a writable-stdout control), `exec_is_one_command_session_and_usage_says_so`
  (r4), `error_responses_carry_a_stable_kind` (r5: every `ok:false` has `error.kind`, a lowercase
  token, identical across two roots and two runs), `huge_values_are_echoed_truncated_to_200_chars`
  (r6: 100k-char command name, path, path segment, param value, top-level key, param key,
  truncated JSON; plus `exec`), `bad_root_error_names_the_path` (r7: file / missing root for run,
  control, exec). New guard `crates/kiln-commands/tests/guard_t004.rs`
  `lexical_dotdot_and_absolute_paths_are_refused_on_disk` (DiskFs + registry; passes on base by
  design; symlinks not tested, T-008). Lint: clippy `--all-targets -D warnings` clean for kiln-cli
  and kiln-commands, and for the kiln-control file on a throwaway stub package (not kept).
  QUESTION: r4/r5 give no exact wording or `kind` values; I test only that the usage text
  mentions saving and names run/control, and that `kind` is a stable lowercase token on every
  error (including protocol errors) — proposed: list the `kind` values in
  `docs/control-protocol.md`. Assumed: `--help` output goes to stdout. · gate attempts: 1 (last:
  red clippy, scorecard, test — only the kiln-control acceptance file not compiling, as before) ·
  not run: nothing else · missing context: none · next: re-join.
- 2026-11-13 · R-044 · review r2 (D-031) · did: new `apps/kiln-cli/tests/acceptance_t004_r3.rs`
  (5 tests, all fail on base by assertion): `run_and_control_error_responses_obey_size_caps` and
  `exec_errors_on_stderr_obey_size_caps` (r9: huge string, huge wrong-type string, 15k-element
  array of short strings with spaces, 100-deep nested object, 5000 unknown param keys, huge
  multi-byte string, huge (multi-byte) command name, 5000 unknown top-level keys, huge array as
  params, huge path; every error line ≤ 4096 bytes, JSON, every string incl. keys ≤ 200 chars;
  truncated unknown-command message ends with `…`; small values `zero` / `no.such.cmd` still
  reported intact), `usage_errors_on_stderr_obey_size_caps_and_name_the_problem` (r9 stderr caps
  for a 10k-char flag, still names it), `usage_error_names_the_offending_flag_or_missing_argument`
  (r12), `error_kinds_are_documented_and_pinned_to_situations` (r10: 9 situations × 2 value
  variants on two roots; each kind is in the documented set, equal across variants and roots, maps
  to one `code`; the four `rejected` situations have four distinct kinds; `exec` errors carry the
  same kind as `run` for 5 situations). The r2 truncation test does not contradict the new caps
  and is unchanged. Lint: clippy `--all-targets -D warnings` clean for kiln-cli.
  QUESTION: r10 does not fix the layout of the kinds list or the kind names, so I read every
  inline-code token in `docs/control-protocol.md` as the documented set (a superset) and pin
  kinds by consistency, not by literal name; proposed: a fenced list or table of kinds the tests
  can read exactly. Assumed: the four `rejected` situations are distinct kinds; a missing-argument
  message contains one of missing/required/requires/expected/expects/needs. · gate attempts: 1
  (last: red clippy, scorecard, test — only the kiln-control acceptance file not compiling, as
  before) · not run: nothing else · missing context: none · next: re-join.
- 2026-11-13 · R-045 · join (r3 tests) · did: fixed my over-assumptions in
  `error_kinds_are_documented_and_pinned_to_situations`: the "invalid request" pair mixed two
  documented situations; it is now "request not an object" (`42`, a JSON string) and "request
  without command" (`{"id":7}`, `{"params":{}}`). Each remaining pair now uses one situation only:
  malformed JSON is two syntax errors (no truncated-input variant), and path refused is two `..`
  paths (no absolute-path variant). Removed the assertion that the four `rejected` situations have
  distinct kinds (the spec does not say so). Each pair is still checked for self-consistency,
  documentation, the same result on both roots, one `code` per kind, and an `exec`-vs-`run`
  match. Clippy clean; fails on base by assertion. · gate attempts: 0 · not run: gate (only this test
  changed) · missing context: none · next: re-join.
<!-- end test author log -->
### reviewer
- 2026-11-13 · R-031 · tests_joined (review r1) · did: reviewed 76051fb as a CLI user: hand slice
  (open/insert/delete/replace/undo/redo/save) via `run --root --log`, `control`, `exec --root`;
  cross-root determinism with 20 failing requests; log replay round-trip; 52 MB hostile script;
  control flush/EOF/EPIPE; seeded panic → `internal`; doc worked example reproduced byte for
  byte. 25 mutants: 23 killed (22 by acceptance), 1 equivalent, 1 survived (catch_unwind).
  Verdict request-changes: D-1 doc says request paths cannot leave the root, but symlinks are
  followed out of it for open and save; TG-1 `../`/absolute-path acceptance lines pass with the
  `..` check removed (M06 wrote outside the root). Review: `reviews/T-004-r1.md`. · gate
  attempts: 1 (last: green) · not run: Windows/macOS, non-root permission errors, revert check
  (director did it) · missing context: none · next: implementer fixes D-1 (doc), test author
  fixes TG-1; director rules on spec gaps.
  QUESTION: SG-1 symlink confinement (deferred by T-002) is user-facing now — schedule before
  the evaluator? SG-2 how to test the panic → `internal` path through the declared API?
  SG-3 should `run` exit 1 when stdout cannot be written (as `control` does)? SG-4 `exec` cannot
  persist edits; intended? SG-5 OS `strerror` text in responses.
- 2026-11-13 · R-040 · review r2 (delta 79da139..51f83e0) · did: re-read docs/control-protocol.md
  end to end against the binary; produced 20 of 23 kinds through run/control (incl. disk changed
  mid-session); attacked clipping (escapes, emoji, combining marks, arrays/objects, log echoes,
  exec stderr); --help on every subcommand; >/dev/full for every subcommand; final
  open-edit-undo-redo-save flow from the doc alone + log replay. Round-1: D-1, TG-1, SG-3, SG-4,
  SG-5, N-2, N-3 resolved; SG-1 ruled (T-008); N-1, N-4 partly; SG-2 open. 20 delta mutants: 18
  killed (8 by acceptance), survivors N08 (panic kind) and N19 (path anchoring). Verdict
  request-changes: R2-D1 `False schema does not allow [..]` details echo arrays/objects of short
  space-containing strings unclipped (16 KB via exec). Review: `reviews/T-004-r2.md`. · gate
  attempts: 1 (last: green) · not run: permission-denied io_error (runs as root), release timing,
  Windows/macOS · missing context: none · next: implementer fixes R2-D1 and R2-N1 doc row.
  QUESTION: R2-SG1 freeze the 23-kind table in the spec so acceptance tests can pin it?
  R2-SG2 add "refuse FIFOs/non-regular files" (doc.open hangs) to T-008? SG-2 panic path test?
<!-- end reviewer log -->
