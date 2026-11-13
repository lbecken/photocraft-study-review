# Kiln control protocol (JSON lines)

Kiln can be driven without a UI. A client sends **requests**, one JSON object per line; the
host answers each non-blank request line with exactly one **response** line. Every request is
a named command of the command registry (`kiln-cli commands --schema` lists them with their
params and result schemas, and `docs/commands.json` holds the same description).

The same protocol is spoken by:

- `kiln-cli run <script>`: requests from a file, responses on stdout;
- `kiln-cli control`: requests on stdin, responses on stdout, interactively;
- the library type `kiln_control::Host` (`handle_line`).

## Framing

- UTF-8 text; each request is one line terminated by `\n` (a final line without `\n` is still
  read; a `\r` before the `\n` is ignored as JSON whitespace).
- A line that is empty or contains only white space gets **no** response.
- Every other line gets exactly one response line, compact JSON terminated by `\n`.
- Responses come in request order. The host never stops or panics because of a request; a
  bad request gets an error response and the next request is processed normally.

## Request

```json
{"id": "anything-or-42", "command": "doc.open", "params": {"path": "src/main.rs"}}
```

| Key | Required | Type | Meaning |
|---|---|---|---|
| `command` | yes | string | the registry command name, e.g. `doc.insert` |
| `params` | no | object | the command's params; defaults to `{}` |
| `id` | no | string or integer | echoed in the response; `null` when absent |

Any other top-level key is an error (`invalid_request`). Integer ids are any JSON integer
(negative allowed) that fits `i64` or `u64`; `1.5`, `1.0`, `null`, `true`, arrays and objects are
not valid ids.

## Response

Object keys are sorted lexicographically (by Unicode code point) at every level, there is no
white space between tokens, and the line ends with `\n`.

Success:

```json
{"id":1,"ok":true,"result":{"bytes_written":12,"dirty":false,"doc":1}}
```

Failure:

```json
{"error":{"code":"invalid_params","details":["\"offset\" is a required property"],"message":"invalid params for `doc.insert`"},"id":"x","ok":false}
```

`error.details` (a list of strings) is present only for `invalid_params`.

### Error codes

| Code | When | `id` in the response |
|---|---|---|
| `invalid_json` | the line is not valid UTF-8, or not valid JSON | `null` |
| `invalid_request` | not a JSON object; `command` missing or not a string; `params` not an object; `id` not a string/integer; an unknown top-level key | the request's `id` if it was a valid id, else `null` |
| `unknown_command` | no command has this name | echoed |
| `invalid_params` | `params` does not match the command's params schema | echoed |
| `rejected` | the command ran and refused (e.g. offset out of range, no such document, file not found) | echoed |
| `internal` | a bug in Kiln (a result that violates its schema, a panicking command) | echoed |

A failed request changes nothing and is not recorded in the session log.

## Determinism

The responses depend only on the request lines and the initial files: no clock, randomness,
thread timing, host path or environment text appears in them. Error messages name files by
their root-relative path only, never by the absolute root. Running the same script on two
copies of the same directory, at different locations, prints byte-identical output and leaves
byte-identical files. `kiln-cli control` produces the same response bytes regardless of how the
input is timed.

## The session log as a script

Every successful request (read-only ones included) is recorded in the session log as its
command name and params. `kiln-cli run … --log <file>` (and `Host::log_script()`) writes the
log as a script: one request per logged command, numbered from 1,

```json
{"command":"doc.open","id":1,"params":{"path":"a.txt"}}
```

with sorted keys, each line ending in `\n`. Running that script on a fresh copy of the initial
directory reproduces the session's files, and its own log is byte-identical to the script.

## `kiln-cli`

```text
kiln-cli run <script> [--root <dir> | --memory] [--log <file>]
kiln-cli control [--root <dir>]
kiln-cli exec <name> <json-params> [--text <initial-text>] [--root <dir> | --memory]
kiln-cli commands [--schema | --audit]
```

- `--root <dir>` (default: the current directory) is the root of the disk file system the
  commands act on; paths in requests (`"path": "a.txt"`) are relative to it and may not leave
  it. Saves replace files atomically.
- `--memory` (for tests) uses an empty in-memory file system instead; it cannot be combined
  with `--root`.
- `<script>` and `--log <file>` are paths on the host, resolved against the current
  directory (not against `--root`). The log file is created (or truncated) before the first
  request runs and written after the last.
- Options may appear before or after the positional arguments; each at most once.
- `run` processes every request, also after failures, and writes all responses to stdout.
- `control` reads stdin until end of input; each response is written and flushed before the
  next request is read, so a client can wait for the answer before sending the next request.
- `exec` runs one command with its params given as a JSON argument, prints the result (not a
  protocol response) on stdout, or the error JSON on stderr. With `--text` the scratch buffer
  of the `buffer.*` commands starts with that text.

### Exit codes

| Subcommand | 0 | 1 | 2 | 3 |
|---|---|---|---|---|
| `run` | every response `ok:true` | — | usage error, unreadable script, root not a directory, log file not writable (message on stderr, nothing on stdout) | at least one response `ok:false` |
| `control` | end of input, whatever the responses | reading stdin or writing stdout failed | usage error, root not a directory | — |
| `exec` | success | internal error | usage error, invalid JSON, command error | — |
| `commands` | success | audit problems | usage error | — |

## Worked example

A directory `proj/` holds `a.txt` with the 6 bytes `hello\n`. The script `s.jsonl` opens the
file, inserts text, undoes and redoes the insert, sends one malformed request, and saves:

```text
{"id":1,"command":"doc.open","params":{"path":"a.txt"}}
{"id":2,"command":"doc.insert","params":{"doc":1,"offset":5,"text":" world"}}
{"id":3,"command":"doc.undo","params":{"doc":1}}
{"id":4,"command":"doc.redo","params":{"doc":1}}
{"id":"oops","command":"doc.insert","params":{"doc":1}}
{"id":5,"command":"doc.save","params":{"doc":1}}
```

```text
$ kiln-cli run s.jsonl --root proj --log l.jsonl
{"id":1,"ok":true,"result":{"already_open":false,"doc":1,"encoding":"utf-8","len_chars":6,"line_ending":"lf","mixed_line_endings":false,"path":"a.txt"}}
{"id":2,"ok":true,"result":{"dirty":true,"len_chars":12}}
{"id":3,"ok":true,"result":{"dirty":false,"len_chars":6,"undone":true}}
{"id":4,"ok":true,"result":{"dirty":true,"len_chars":12,"redone":true}}
{"error":{"code":"invalid_params","details":["\"offset\" is a required property","\"text\" is a required property"],"message":"invalid params for `doc.insert`"},"id":"oops","ok":false}
{"id":5,"ok":true,"result":{"bytes_written":12,"dirty":false,"doc":1}}
$ echo $?
3
$ cat proj/a.txt
hello world
$ cat l.jsonl
{"command":"doc.open","id":1,"params":{"path":"a.txt"}}
{"command":"doc.insert","id":2,"params":{"doc":1,"offset":5,"text":" world"}}
{"command":"doc.undo","id":3,"params":{"doc":1}}
{"command":"doc.redo","id":4,"params":{"doc":1}}
{"command":"doc.save","id":5,"params":{"doc":1}}
```

The exit code is 3 because one request failed; the failed request is not in the log. The same
session interactively:

```text
$ kiln-cli control --root proj
{"id":1,"command":"doc.open","params":{"path":"a.txt"}}
{"id":1,"ok":true,"result":{"already_open":false,"doc":1,"encoding":"utf-8","len_chars":12,"line_ending":"lf","mixed_line_endings":false,"path":"a.txt"}}
{"id":2,"command":"doc.text","params":{"doc":1}}
{"id":2,"ok":true,"result":{"len_chars":12,"text":"hello world\n"}}
```
