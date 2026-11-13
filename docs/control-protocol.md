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
- Responses come in request order. A malformed or failing request gets an error response
  and the next request is processed normally. A command that panics (a bug) is reported as
  `internal` / `panic` and the session continues; such a command may have left its document
  partly changed. There is no request size limit: a huge line is read whole into memory.

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
{"error":{"code":"invalid_params","details":["\"offset\" is a required property"],"kind":"schema","message":"invalid params for `doc.insert`"},"id":"x","ok":false}
```

- `error.code` and `error.kind` are stable machine-readable values: match on these.
- `error.message` and `error.details` are for humans. They are not stable, and the message may
  contain text from the operating system (for example `No such file or directory (os error 2)`
  or `File name too long (os error 36)`), which differs between platforms.
- `error.details` (a list of strings) is present only for `invalid_params`.
- A value from the session that a message or detail echoes back (a command name, a path, a
  param value, a key) is cut to at most 200 characters: the first 199 followed by `…`. In
  addition, any run of more than 200 non-white-space characters in a message or detail is cut
  the same way. A valid `id` is always echoed in full.

### Error codes and kinds

| Code | Kind | When | `id` in the response |
|---|---|---|---|
| `invalid_json` | `not_utf8` | the line is not valid UTF-8 | `null` |
| `invalid_json` | `syntax` | the line is not valid JSON | `null` |
| `invalid_request` | `not_an_object` | the JSON is not an object | `null` |
| `invalid_request` | `bad_id` | `id` is not a string or an integer | `null` |
| `invalid_request` | `unknown_key` | a top-level key other than `id`, `command`, `params` | echoed if valid, else `null` |
| `invalid_request` | `missing_command` | no `command` | echoed if valid, else `null` |
| `invalid_request` | `command_not_string` | `command` is not a string | echoed if valid, else `null` |
| `invalid_request` | `params_not_object` | `params` is not an object | echoed if valid, else `null` |
| `unknown_command` | `unknown_command` | no command has this name | echoed |
| `invalid_params` | `schema` | `params` does not match the command's params schema | echoed |
| `rejected` | `invalid_path` | the path is not a valid root-relative path (see Paths) | echoed |
| `rejected` | `no_such_document` | no open document has this id | echoed |
| `rejected` | `unsaved_changes` | `doc.close` of a modified document without `discard` | echoed |
| `rejected` | `not_found` | the file (or, for a save, its directory) does not exist | echoed |
| `rejected` | `not_a_file` | the path names a directory | echoed |
| `rejected` | `io_error` | any other file-system failure (permissions, name too long, …) | echoed |
| `rejected` | `decode_error` | the file is not valid UTF-8 / UTF-16 text | echoed |
| `rejected` | `out_of_range` | an offset beyond the end of the text | echoed |
| `rejected` | `inverted_range` | a range whose start is after its end | echoed |
| `rejected` | `other` | any other refusal | echoed |
| `internal` | `panic` | a command panicked (a bug in Kiln) | echoed |
| `internal` | `result_schema` | a command's result violates its own schema (a bug in Kiln) | echoed |
| `internal` | `other` | any other internal error | echoed |

The `kind` of a `rejected` error is currently derived by `kiln-control` from the message the
lower layers produce; a refusal it does not recognise is reported as `other`, never dropped.
Further kinds may be added later; a client should treat an unknown `kind` by its `code`.

A request that fails validation, or a command that returns an error, is not recorded in the
session log; commands are written so that a refused request leaves the session unchanged.

## Paths

Paths in requests (`"path": "src/main.rs"`) are relative to the root (`--root`, or the
current directory). What is checked today:

- Refused (`rejected` / `invalid_path`) by a purely lexical check: an empty path, an absolute
  path (leading `/`), a `..` or `.` segment, an empty segment (`a//b`, a trailing `/`), a
  backslash, a NUL character.
- **Symbolic links are followed, including links that point outside the root.** A request path
  that passes through a symlink inside the root can open, and a save can replace, a file
  outside the root. Do not point `--root` at a directory whose symlinks you do not trust.
  Confining paths to the root is tracked as task T-008.
- A save replaces the file by writing a temporary file `.kiln-tmp-<n>` next to it and renaming
  it over the target (on Unix); through a symlink, the link's target is replaced and the link is
  kept.

## Determinism

The responses depend only on the request lines and the initial files: Kiln adds no clock,
randomness or thread timing to them, and Kiln's own error messages name files by their
root-relative path, never by the absolute root (a path you put in a request yourself is echoed
as given). Operating-system error text in `message` is the exception: it is the same from run
to run on one platform but can differ between platforms. So on one platform, running the same
script on two copies of the same directory, at different locations, prints byte-identical
output and leaves byte-identical files. `kiln-cli control` produces the same response bytes
regardless of how the input is timed.

## The session log as a script

Every successful request (read-only ones included) is recorded in the session log as its
command name and params. `kiln-cli run … --log <file>` (and `Host::log_script()`) writes the
log as a script: one request per logged command, numbered from 1,

```json
{"command":"doc.open","id":1,"params":{"path":"a.txt"}}
```

with sorted keys, each line ending in `\n`. Running that script on a fresh copy of the initial
directory repeats the session's successful commands, so it reproduces the session's files and
its own log is byte-identical to the script, provided nothing else changes the files and the
same requests succeed again (an `io_error` from a full disk, say, may not repeat).

Document ids are per session: the first opened document is `1`, the next `2`, and so on; a
failed `doc.open` does not use up an id. A script names documents by these predicted ids.

## `kiln-cli`

```text
kiln-cli run <script> [--root <dir> | --memory] [--log <file>]
kiln-cli control [--root <dir>]
kiln-cli exec <name> <json-params> [--text <initial-text>] [--root <dir> | --memory]
kiln-cli commands [--schema | --audit]
kiln-cli --help | -h | <subcommand> --help
```

- `--help` or `-h` (alone, or after a subcommand) prints the usage text on stdout and exits 0.
- `--root <dir>` (default: the current directory) is the root of the disk file system the
  commands act on; paths in requests are relative to it (see Paths for what is and is not
  refused). A root that does not exist or is not a directory is a usage error whose message
  names the path.
- `--memory` (for tests) uses an empty in-memory file system instead; it cannot be combined
  with `--root`.
- `<script>` and `--log <file>` are paths on the host, resolved against the current
  directory (not against `--root`). The log file is created (or truncated) before the first
  request runs and written after the last.
- Options may appear before or after the positional arguments; each at most once.
- `run` processes every request, also after failures, and writes all responses to stdout.
- `control` reads stdin until end of input; each response is written and flushed before the
  next request is read, so a client can wait for the answer before sending the next request.
- `exec` runs **one command in a fresh session**, with its params given as a JSON argument,
  prints the result (not a protocol response) on stdout, or the error JSON on stderr. It can
  read files (`doc.open`, `doc.text`, …), but an edit made with it is never saved: the next
  `exec` is a new session in which no document is open. To edit and save files, use `run` or
  `control`. With `--text` the scratch buffer of the `buffer.*` commands starts with that text.

### Exit codes

| Subcommand | 0 | 1 | 2 | 3 |
|---|---|---|---|---|
| `run` | every response `ok:true` | writing stdout failed | usage error, unreadable script, root not a directory, log file not writable (message on stderr, nothing on stdout) | at least one response `ok:false` |
| `control` | end of input, whatever the responses | reading stdin or writing stdout failed | usage error, root not a directory | — |
| `exec` | success | internal error, or writing stdout failed | usage error, bad root, invalid JSON, command error | — |
| `commands` | success | audit problems, or writing stdout failed | usage error | — |
| `--help`, `-h` | always | — | — | — |

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
{"error":{"code":"invalid_params","details":["\"offset\" is a required property","\"text\" is a required property"],"kind":"schema","message":"invalid params for `doc.insert`"},"id":"oops","ok":false}
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
