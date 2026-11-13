---
iteration: 0
run: R-050
main_sha: d3978dbfea7c69446be96866353b31da916efc9f
interfaces: [cli, control]
confirmed_items: [cli.exec, cli.run, cli.schema, cmd.log-replay, cmd.registry, cmd.schema, control.errors, control.protocol, control.session, io.bom, io.crlf, io.encoding, io.save-atomic, io.utf8, text.delete, text.insert, undo.grouping, undo.redo, undo.roundtrip, undo.undo, ws.dirty-tracking, ws.documents]
rejected_items: []
defects_filed: [T-010, T-011]
not_evaluated: [text.lines, text.positions, ws.vfs]
---

# Evaluation, iteration 0

Evaluated 25 of 25 `implemented` items: 22 confirmed, 0 rejected, 3 not confirmed because no
public interface exposes the behaviour (see "Not evaluated"). No UI exists in this
iteration, so `ui-snapshot` was not used.

## Setup
- Clean clone at `d3978dbf` in the director's scratch checkout. `cargo build -p kiln-cli`
  (debug, 18 s) for behaviour; `cargo build -p kiln-cli --release` only for timings.
- `cargo xtask scorecard` → `PASS scorecard: 63 items: 0 verified, 25 implemented, 0 failing,
  38 planned.`
- Sources of truth used: `kiln-cli --help`, `kiln-cli commands [--schema|--audit]`,
  `docs/control-protocol.md`, `docs/commands.json`, `docs/scorecard.md`,
  `scorecard/items/*.toml`. No source, tests, `tasks/` (other than the IDs I was told),
  `reviews/` or `docs/decisions.md` were read.
- Experiments in fresh directories under the scratchpad `eval-work/` with small Python
  drivers: `k.py` (writes a JSON-lines script, runs `kiln-cli run <script> --root <dir>`,
  parses responses), `ctl.py` (drives `kiln-cli control` one line at a time), `model.py`
  (random edits against a naive Python string model), `perf.py` / `long.py` (timings). Every
  invocation had a `timeout`. Fixtures were generated: Rust sources with CRLF, BOM, `é`, `😀`,
  `中`, no trailing newline, empty, BOM-only, UTF-16 LE/BE with BOM, invalid UTF-8 (lone
  `\xff`, overlong, encoded surrogate, truncated sequence), NUL bytes, 4 KiB random bytes, an
  ELF header, a 50 MB generated Rust file (52,435,350 bytes, 3,025,002 lines).
- Python `str` indices are Unicode scalar values, which is what the protocol's offsets count,
  so the naive model needs no conversion.

## Items

### cli.exec
- Claimed: `kiln-cli exec <name> <json>` prints the JSON result, or a structured error with exit 2.
- Did: `exec buffer.insert '{"offset":0,"text":"fn main() {}"}'`; `--text 'héllo'` with
  offset 2; offset 99 on `abc`; `"offset":"0"`; `'{offset:0}'`; `nosuch.cmd`; `doc.open` on a
  real file; `doc.text {"doc":1}`; `[]` as params; missing params; extra argument; `--text`
  twice; bad `--root`.
- Observed: `{"len_chars":12}` / `{"len_chars":6}` rc 0; errors on stderr as
  `{"code":"rejected","kind":"out_of_range",…}`, `invalid_params/schema` with details,
  `invalid_json/syntax`, `unknown_command`, `rejected/no_such_document`, all rc 2 with empty
  stdout; usage errors rc 2 with usage text.
- Verdict: **confirmed** (interface: cli `exec`).
- Inert check: by design every document command other than `doc.open`/`doc.list` fails
  under `exec` (`no_such_document`), because each `exec` is a fresh session. Documented in
  `--help` and the protocol doc, so not misleading, but `exec doc.insert|save|undo…` can
  never succeed.

### cli.run
- Claimed: one response per request; same script on same files → byte-identical output.
- Did: a 33-line hostile script (31 non-blank lines incl. blank lines, a `\r`-terminated
  line, a final line without `\n`) run on two copies of a directory at different absolute
  paths (`rA`, `sub/deeper/rB`).
- Observed: 31 responses for 31 non-blank lines, both runs rc 3, stdout byte-identical,
  resulting files byte-identical. Also: a recorded log replayed twice gives identical output
  (see cmd.log-replay).
- Verdict: **confirmed** (cli `run`).

### cli.schema
- Claimed: `kiln-cli commands --schema` prints every command with schemas and examples.
- Did: `commands --schema | diff - docs/commands.json`; `commands --audit`; reran all 17
  published examples myself (files + setup + params) through `run` / `exec --text`;
  validated 15 observed results of all 13 commands against the published `result_schema`
  with `jsonschema` 4.26.
- Observed: identical to `docs/commands.json`; `audit ok: 13 commands, all examples
  validate`; 17/17 examples reproduced exactly; 15/15 results valid.
- Verdict: **confirmed** (cli `commands`).

### cmd.schema
- Claimed: every command publishes params/result JSON Schemas and an executable example.
- Did/observed: as for cli.schema; each of the 13 commands has `params_schema`,
  `result_schema` and ≥1 example, and I executed every example independently of `--audit`.
- Verdict: **confirmed** (cli `commands`, `run`).

### cmd.registry
- Claimed: every state change goes through a named command whose params are schema-validated.
- Did: for each of the 13 commands, sent params with an unknown key `zzz` and with every
  declared property set to the string `"wrong"`; then read `doc.text` and `doc.info`.
  Also floats for integers (`1e2`, `2.0`), `1e20` for a doc id, negative offset.
- Observed: unknown key → `invalid_params` for all 13; wrong types → `invalid_params` for
  12 (`doc.open {"path":"wrong"}` is a valid string and gives `not_found`); document text and
  `can_undo` unchanged after every refusal. The only external entry points are commands; the
  file on disk changed only on `doc.save`.
- Verdict: **confirmed** (control/run). Note: for floats the error message reads
  `params … do not deserialize` (a second validation layer) but the code/kind is the same
  `invalid_params`/`schema`.

### cmd.log-replay
- Claimed: the command log replayed on the initial state reproduces the final document state.
- Did: 316-request random session over 3 files (CRLF Rust, BOM Rust, plain), with inserts,
  deletes, undo, redo, save, close, close-with-discard, reopen and `doc.text`; 82 succeeded,
  234 failed (mostly on closed ids). `run … --log log1.jsonl`; replayed `log1.jsonl` on a
  fresh copy with `--log log2.jsonl`.
- Observed: log has exactly the 82 successful requests, renumbered from 1; replay rc 0, all
  ok; the replay's results equal the original successful results (incl. every `doc.text`);
  `log2 == log1` byte for byte; all files byte-identical to the original session's files,
  no leftover temp files.
- Verdict: **confirmed** (cli `run --log`). Note: `control` has no `--log`, so an
  interactive session cannot be recorded.

### control.protocol
- Claimed: JSON-lines, one response per line, ids echoed.
- Did: the hostile script above; integer ids `-9223372036854775808` and
  `18446744073709551615` (echoed), `18446744073709551616`, `1.0`, `null`, `true`, `[1]`
  (`bad_id`, id `null`); a 5,000-char string id (echoed in full); via `control`: a request
  split across two writes (no answer until the newline), blank lines (no answer), final line
  without newline at EOF (answered, rc 0).
- Observed: as documented. Undocumented detail: duplicate keys `{"id":9,"id":10,…}` are
  accepted and the last one wins.
- Verdict: **confirmed** (run, control).

### control.errors
- Claimed: malformed/invalid requests get structured errors; the host never exits on bad input.
- Did: not-JSON, invalid UTF-8, a 50 MB line of random bytes, `[1,2]`, `"str"`, `{}`,
  params array, numeric command, unknown top-level key, wrong-case command, two objects on
  one line, a lone surrogate `\ud800`, 100,000 `[` (deep nesting), a 300-char path, a
  2,000-char-valued params object with 200 extra keys.
- Observed: each gets exactly one structured error with the documented code/kind (lone
  surrogate → `invalid_json/syntax`; deep nesting → `recursion limit exceeded`); messages
  capped at 200 chars ending in `…`; the session continued and the next request succeeded
  every time.
- Verdict: **confirmed** (run, control).

### control.session
- Claimed: a headless session can open, edit and save a file through the control protocol.
- Did: `ctl.py` session on a BOM + CRLF Rust file without trailing newline: rename
  `count_words` → `tally` with two `doc.replace` at offsets computed from `doc.text`, type
  `\r\n// done` char by char, `doc.save`, then compile the saved file with `rustc`.
- Observed: BOM kept, 15 CRLF and 0 bare LF, `rustc` rc 0, program prints `2`; 5 undo steps
  bring the text back to the opened text.
- Verdict: **confirmed** (control).

### io.utf8
- Claimed: unedited UTF-8 files open and save byte-for-byte unchanged.
- Did: opened and saved without editing 12 files (LF, CRLF, mixed, CR-only, no trailing
  newline, empty, BOM, BOM-only, UTF-16 LE/BE, NUL bytes, BOM-less UTF-16-looking bytes).
- Observed: 12/12 byte-identical after save.
- Verdict: **confirmed** (run).

### io.bom
- Claimed: UTF-8 BOM detected on open and preserved on save.
- Did: open `\xef\xbb\xbffn main() {}\n`, edit, save; BOM-only file, edit, save.
- Observed: `encoding: "utf-8-bom"`, `len_chars` 13 (BOM not in the text), saved bytes start
  with exactly one BOM; BOM-only → `x` gives `\xef\xbb\xbfx`. A `U+FEFF` the user inserts at
  offset 0 is written after the BOM (two BOMs on disk) and survives reopen as text, which is
  consistent.
- Verdict: **confirmed** (run).

### io.crlf
- Claimed: detect LF or CRLF on open and write the same line endings on save.
- Did: open CRLF, LF, mixed and CR-only files; save unedited (byte-identical); in a CRLF file
  insert `    x();\n` and `    y();\r\n` and save.
- Observed: `line_ending` `crlf`/`lf`/`cr`/`none` and `mixed_line_endings` correct; existing
  line endings are written verbatim; the inserted `\n` is written as `\n`, so the file
  becomes mixed (`doc.info` then says `mixed_line_endings: true`).
- Verdict: **confirmed**, with a spec ambiguity for the director: the item can be read as
  "line breaks a user adds follow the detected style". The product implements "verbatim"
  (`doc.save` doc says so), so the detected `line_ending` is informational only: it changes
  nothing that is written. If the intended meaning is the stronger one, this is a defect;
  `io.mixed-eol` (planned) may be the place to decide.

### io.encoding
- Claimed: UTF-16 LE/BE with BOM open correctly; undecodable bytes reported, never replaced.
- Did: UTF-16 LE `h é 😀 \r\n` and BE `h é 😀 \n` (surrogate pair inside), edit (`😀`, `中`)
  and save; invalid UTF-8 variants; UTF-16 with an odd trailing byte and a lone surrogate.
- Observed: `utf-16le` 5 chars / `utf-16be` 4 chars, edits saved as correct UTF-16 with BOM.
  Every invalid file → `rejected/decode_error` with the exact byte offset of the first bad
  byte (e.g. 13 for `fn a() {}\n// \xff`); no replacement characters anywhere. UTF-16
  without BOM is opened as UTF-8 (not claimed).
- Verdict: **confirmed** (run).

### io.save-atomic
- Claimed: save via temporary file + rename; a failed save leaves the original intact.
- Did: `strace` of a save; save with the directory made immutable (`chattr +i`, since I run
  as root); save after the directory was deleted; save after the path became a directory;
  pre-existing user files named `.kiln-tmp-0` / `.kiln-tmp-1` next to the target.
- Observed: `openat .kiln-tmp-0 O_CREAT|O_EXCL 0600` → `write` → `fsync` → `rename` →
  `fsync(dir)`. Failed saves: `io_error` / `not_found` / `not_a_file`, original file intact,
  document stays dirty, no temp left. Pre-existing `.kiln-tmp-*` files untouched. Mode bits
  (0751, 0444) preserved.
- Verdict: **confirmed** (control). Notes: the rename breaks hard links (the other name
  keeps the old content) and a read-only (0444) file is overwritten without complaint; both
  are undocumented.

### text.insert
- Claimed: insert at a char position; out-of-range rejected, buffer unchanged.
- Did: inserts before/after `é`/`😀`; offsets 46 and 10000 on a 40-char text, `-1`; 5 seeds ×
  150 random edits (inserts of 0–4 chars from `a é 😀 \n \r\n \r 中`) compared with a naive
  model after every edit.
- Observed: all 750 texts and lengths match the model; out-of-range → `out_of_range`, text
  unchanged; negative → `invalid_params`.
- Verdict: **confirmed** (run).

### text.delete
- Claimed: delete a char range, return removed text; inverted / out-of-range rejected.
- Did: `start 5 end 3`, `end 1000`, empty range, and the random sessions above (deletes and
  replaces).
- Observed: `inverted_range`, `out_of_range`, `removed` equals the model's slice every time.
- Verdict: **confirmed** (run).

### undo.undo
- Claimed: undo reverts the most recent edit exactly.
- Did: 30 multi-char inserts/deletes/replaces on a CRLF text, then 31 undos with `doc.text`
  after each.
- Observed: each undo restores exactly the previous state; the 31st returns `undone: false`.
- Verdict: **confirmed** (run). Minor: a `doc.replace` with identical text (`b` → `b`)
  creates an undo step whose undo changes nothing (`can_undo` stays true).

### undo.redo
- Claimed: redo reapplies; a new edit after undo clears redo.
- Observed: after undo-all, redo-all (123–132 steps per seed) gives the final text exactly;
  `can_redo` false after a new edit following undo (case "undo then typing").
- Verdict: **confirmed** (run).

### undo.roundtrip
- Claimed: any edit sequence then undoing every step restores the original.
- Observed: 5/5 random 150-edit sessions restored the opened text exactly, `dirty: false`,
  `can_undo: false`.
- Verdict: **confirmed** (run).

### undo.grouping
- Claimed: consecutive typed chars at adjacent positions group into one undo step.
- Did: 12 cases on `0123456789`.
- Observed: `abc` typed at the end or in the middle → one undo; `😀é` → one; `a ␠ b ⇥` → one;
  gap (a@10, b@5), backwards (a@10, b@10), multi-char insert, `\n`, `\r\n`, an intervening
  delete, and `doc.save` each break the group; a rejected insert inside a run does not.
- Verdict: **confirmed** (run).

### ws.dirty-tracking
- Claimed: modified after edit, clean after save or full undo.
- Observed: dirty after edit; clean after save; undo past the save point → dirty; save
  after undo writes the undone text and is clean; redo → dirty, undo → clean. Dirty is by
  content: inserting `xy` and deleting it again by hand also gives clean.
- Verdict: **confirmed** (control).

### ws.documents
- Claimed: several documents open at once, each with its own buffer, history, dirty flag.
- Observed: 3 documents; edits/undo on one leave the others' text, `can_undo` and `dirty`
  unchanged; `doc.list` sorted with correct flags; dirty close refused (`unsaved_changes`),
  `discard: true` closes; reopen gets a new id (4) and empty history; reopening an open path
  returns the edited document with `already_open: true`; a failed `doc.open` uses no id.
- Verdict: **confirmed** (control). Note: the same file opened through a symlink name is a
  second, independent document (two buffers on one file; last save wins).

## Wired but inert
- `--memory` (run, exec): the in-memory file system is always empty and nothing can create
  a file in it, so every file request fails with `not_found`. Filed **T-011**.
- `doc.info.line_ending` / `doc.open.line_ending`: correct, but has no effect on what save
  writes (see io.crlf). Spec question, no task filed.
- `exec` with any document command except `doc.open`/`doc.list`: can never succeed
  (documented).
- `doc.replace` with identical text: a no-op undo step (minor, no task).
- `buffer.insert` / `buffer.text`: a scratch buffer with no save, undo or path; works but has
  no use beyond demonstrating the registry.

## Hostile use and robustness (not scorecard items)
- Symlinks: the documentation is **true**. `out.txt -> ../outside/secret.txt` opens, and a
  save replaced the outside file (`PWNED SECRET`), link kept. `outdir/secret.txt` through a
  directory link also opens. Dangling link → `not_found`; link loop → `io_error` (os error
  40). Covered by T-008 (not duplicated).
- FIFO: `doc.open` of a FIFO hangs (killed by `timeout 10`, no response). Saving a document
  whose file was replaced by a FIFO silently replaces the FIFO with a regular file.
  Device files through symlinks (`/dev/zero`, `/dev/urandom`) are read until memory runs out
  (under `ulimit -v 3000000`: `io_error … out of memory` after 0.9 s / 4.9 s; without a limit
  it would exhaust the machine). Covered by T-008.
- Directory where a file is expected → `not_a_file`. Lexical path checks (`/abs`, `.`,
  `..`, `a//b`, trailing `/`, backslash, NUL) all `invalid_path` as documented.
- Binary garbage files → `decode_error`; a valid-UTF-8 ELF header opens as text.
- Enormous requests: a single 300 MB `doc.insert` line succeeded in 2.0 s (release) with a
  peak RSS of 1.9 GB (≈6× the payload); documented as "no request size limit".
- Two sessions on one file: B saves, then A saves → B's change is lost with no warning
  (planned item `io.external-change`).
- End of input with a dirty document: `control` exits 0 and the unsaved edit is silently
  dropped.
- `run --log /dev/full`: requests ran (the file on disk was changed) but stdout is empty and
  rc 2 — documented ("nothing on stdout"), but the caller loses all responses after side
  effects happened.
- Memory grows with session length (the session log is kept): 49 MB peak after 50,000
  commands on a 13-byte file.

## Measurements
Release build unless noted; `perf.py`, `long.py` in the scratch `eval-work/`.

| What | Result |
|---|---|
| open 50 MB Rust file (`doc.open`) | 0.13–0.17 s (debug 0.89 s) |
| one insert in the middle of it | 0.1–0.2 ms |
| save it (52,435,360 bytes) | 0.15–0.17 s (debug 0.19 s) |
| open + edit + save total | 0.28–0.34 s (debug 1.09 s) |
| `doc.text` of it (55 MB response line) | 0.38 s |
| `doc.info` of it (computes `len_lines` 3,025,002) | 48 ms |
| 1000 single-char inserts at scattered offsets in it | median 0.04 ms, p99 0.09 ms, max 38 ms |
| peak RSS with the 50 MB file open and edited | 220 MB |
| long `control` session, 50,000 commands (insert/undo/info) | 0.040 ms/command, median 0.037 ms, p99 ≤ 0.072 ms, flat over the session |
| same, debug, 20,000 commands | 0.079 ms/command |

## Defects filed
| Task | Item | Summary | Suspected origin task |
|---|---|---|---|
| T-010 | text.lines, text.positions | No command exposes line text, line starts or position conversion; the items cannot be observed from outside | T-001 (from test names `kiln_text::acceptance_t001::*`) |
| T-011 | ws.vfs | `--memory` file system is always empty and cannot be seeded; memory/disk agreement cannot be checked from outside | T-002 (from test names `kiln_workspace::acceptance_t002::*`) |

Not filed (already covered): symlink escape, FIFO hang, device files → T-008.

## Not evaluated
- `text.lines`: only the line count is observable (`doc.info.len_lines`, which agreed with
  a naive count on 31 random LF/CRLF/CR texts). Line text and line start offsets have no
  command. Not confirmed; T-010.
- `text.positions`: no command takes or returns byte, UTF-16 or line/column positions. Not
  observable at all. Not confirmed; T-010.
- `ws.vfs`: the disk implementation is exercised by everything above, but the in-memory one
  can only be observed empty (its responses equal those of an empty disk root for a 7-line
  script). Agreement on reads and writes of real files cannot be checked. Not confirmed;
  T-011.

## Documentation checks
- True as written: symlink paragraph; temp-file-and-rename save (`.kiln-tmp-<n>`, link kept);
  framing (blank lines, `\r`, last line without `\n`); id rules (`null`, `1.0`, `true` →
  `bad_id`); error caps (200-char messages); determinism across root locations; exit codes
  for run/control/exec/commands I tried, including `/dev/full` on stdout → 1.
- Imprecise: the doc says a bad `--root` "is a usage error", and that a usage error prints the
  usage text after the message; for a bad root only the one-line message is printed (rc 2).
  The exit-code table lists "root not a directory" separately, so this is wording, not
  behaviour.
- Spec ambiguity: `io.crlf` (see the item).
