# `kiln-cli --memory-seed`: an in-memory file system from a manifest

T-011. Lets a user or the evaluator run the same script against the in-memory file system and
against a disk root holding the same files, and compare the responses and the resulting files.
The general CLI and protocol are described in `docs/control-protocol.md`; this file covers only
the option (Decision D-1 of T-011).

## Usage

```text
kiln-cli run <script> [--root <dir> | --memory | --memory-seed <manifest>] [--log <file>]
kiln-cli exec <name> <json-params> [--text <initial-text>] [--root <dir> | --memory | --memory-seed <manifest>]
```

`--memory-seed <manifest>` starts the session on an in-memory file system holding exactly the
manifest's files. `<manifest>` is a host path resolved against the current directory, like
`<script>` and `--log`. The manifest is read once, at start; nothing is ever written to disk:
saves change the in-memory files only, and they are discarded when the process exits (the next
run starts from the manifest again). `--memory` (an empty in-memory file system) is unchanged;
an empty `files` array behaves the same. `control` takes neither `--memory` nor `--memory-seed`.

`--memory-seed` cannot be combined with `--root` or `--memory`, and may be given once. Each is a
usage error (exit 2, usage text on stderr), for example:

```text
kiln-cli: options `--root` and `--memory-seed` cannot be combined
kiln-cli: options `--memory` and `--memory-seed` cannot be combined
kiln-cli: option `--memory-seed` given twice
```

## Manifest

A JSON object with exactly one key, `files`: an array of objects with the key `path` and exactly
one of `text` or `hex`.

```json
{"files": [
  {"path": "src/main.rs", "text": "fn main() {}\r\n"},
  {"path": "bom.txt",     "hex": "efbbbf6869"},
  {"path": "bad.txt",     "hex": "ff"}
]}
```

- `path`: a root-relative path by the request-path rules of "Paths" in
  `docs/control-protocol.md` (non-empty, `/`-separated, no leading `/`, no empty, `.` or `..`
  segment, no `\`, no NUL). Directories are implied by the paths; there are no empty
  directories, symlinks or special files.
- `text`: the file's bytes are the string's UTF-8 encoding.
- `hex`: an even number of hex digits, either case (`"EFbb"`), giving the bytes; `""` is an
  empty file. Use it for a BOM, UTF-16 or bytes that are not valid UTF-8.

The files are then read like any others: `doc.open` detects the encoding and line endings
(`bad.txt` above is refused with `decode_error`, as the same file on disk is).

## Refusals

A bad manifest is refused before any request runs: exit 2, nothing on stdout, no usage text,
and a first stderr line

```text
kiln-cli: bad --memory-seed <manifest>: <reason>
```

where `<manifest>` is the argument as given and `<reason>` a free-text explanation (not a
stable contract). A `run` with a bad manifest therefore runs no request and does not create its
`--log` file. The manifest is refused when:

- the file cannot be read (missing, a directory, no permission);
- it is not valid JSON (including nesting deeper than the JSON parser allows);
- any JSON object in it repeats a key (keys compared after unescaping), for example
  `{"files":[…],"files":[]}` or an entry with `text` twice;
- the top level is not an object, has a key other than `files`, or lacks `files`; `files` is not
  an array; an entry is not an object or has a key other than `path`, `text`, `hex`;
- an entry's `path` is missing, not a string or not a valid path;
- an entry has both or neither of `text` and `hex`, a non-string value for one, or a `hex` with
  a non-hex character or an odd number of digits;
- two entries have the same path;
- a path is a strict prefix directory of another at a `/` boundary (`d` and `d/x`): a file
  cannot also be a directory. A string prefix alone is no conflict: `d` and `dx/y` are fine.

## Order of checks

1. Option parsing (usage errors: exit 2 with usage text).
2. The manifest (exit 2, `bad --memory-seed`), in this order: read, JSON (including repeated
   keys), keys (top level and every entry), then per entry in array order its `path`, its
   `text`/`hex` and duplicates, and last the prefix conflicts over all paths. Only the first
   problem is reported.
3. For `run`: reading the script, then opening `--log`.
4. Requests.

For `exec`, `<json-params>` that are not JSON are reported first (`invalid_json`, exit 2),
before the manifest is read, as they are before `--root` is checked: `exec doc.open '{'
--memory-seed missing.json` reports `invalid_json`, not `bad --memory-seed`.

## Exit codes

As for `--root` (`docs/control-protocol.md`, "Exit codes"), with one addition: exit 2 for a bad
manifest, for `run` and `exec`.

## Comparing memory and disk

```sh
kiln-cli run s.jsonl --memory-seed m.json > mem.out
kiln-cli run s.jsonl --root d > disk.out    # d holds the manifest's files
cmp mem.out disk.out
```

To inspect what a memory session saved, end the script with `doc.close`, a new `doc.open` of
the path, `doc.info` and `doc.text`: text plus `encoding` determine the bytes of a decodable
file uniquely (UTF-8 with or without BOM, UTF-16 LE/BE with BOM), so equal responses mean
equal files.
