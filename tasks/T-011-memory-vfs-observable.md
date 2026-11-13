---
id: T-011
title: "Let kiln-cli seed the in-memory file system from a manifest, so memory and disk sessions can be compared"
tier: medium            # ADR-0002: a CLI option that fills the in-memory VFS; reads one host file given on the command line (as `run` reads its script), writes nothing to disk, spawns nothing, no boundary or dependency
state: draft
owner_role: planner
scorecard_items: [ws.vfs]
depends_on: [T-004]
touches:
  - apps/kiln-cli/src/main.rs           # option parsing, manifest reading, usage text
  - apps/kiln-cli/tests/memory_seed.rs  # implementer's own integration tests
  - docs/protocol/cli-memory-seed.md    # NEW: `--memory-seed` and the manifest (D-1); docs/control-protocol.md is not touched
budget_tokens: 500000   # medium (v1.1, provisional)
iteration: 1
found_by: evaluator
origin_task: T-002
---

# T-011 Let kiln-cli seed the in-memory file system from a manifest

## Goal
A user or the evaluator can run the same script against the in-memory file system and against a
disk root holding the same files, and compare responses and resulting file contents. Today
`--memory` gives an always-empty file system that nothing can fill, so every `doc.open` fails
with `not_found` and `ws.vfs` ("in-memory implementation for deterministic tests and a disk
implementation") could not be confirmed from outside (`evaluations/iter-0.md`, "Wired but inert"
and "Not evaluated"; R-050 on main `d3978db`; reproduced by the planner in R-055: `run` of one
`doc.open a.txt` with `--memory` → `rejected`/`not_found`, exit 3). The empty `--memory` stays as
it is (T-004 acceptance tests pin it); a new option seeds it.

**Experiment arm (METHODOLOGY §6): odd task number → the implementer writes the acceptance
tests**, through the CLI, in the same run: first on `task/T-011-tests` (committed, each failing
on base), then the code on `task/T-011`. The join is unchanged; the reviewer's mutation
spot-check is the only independent check.

Reading list: `evaluations/iter-0.md` (`ws.vfs`, "Wired but inert"); `scorecard/items/ws.vfs.toml`;
`apps/kiln-cli/src/main.rs` (`parse_opts`, `Fs::Memory`, `Host::in_memory`);
`crates/kiln-control/src/lib.rs` `Host::in_memory(files: &[(&str, &[u8])])`;
`crates/kiln-workspace/src/{memory,path}.rs` (`MemoryFs`, `RelPath` rules);
`docs/control-protocol.md` "`kiln-cli`", "Exit codes", "Paths".

## Interface

### CLI
```text
kiln-cli run <script> [--root <dir> | --memory | --memory-seed <manifest>] [--log <file>]
kiln-cli exec <name> <json-params> [--text <initial-text>] [--root <dir> | --memory | --memory-seed <manifest>]
```
`--memory-seed <manifest>` starts the session on an in-memory file system holding exactly the
manifest's files. It cannot be combined with `--root` or `--memory` (usage error, as `--root`
with `--memory` today: `options \`--root\` and \`--memory-seed\` cannot be combined`, and the same
for `--memory`). `<manifest>` is a host path resolved against the current directory, like
`<script>`. `control` is unchanged (it takes neither `--memory` nor `--memory-seed`).

### Manifest
A JSON object with exactly one key `files`, an array of objects with key `path` and exactly one
of `text` or `hex`:
```json
{"files": [
  {"path": "src/main.rs", "text": "fn main() {}\r\n"},
  {"path": "bom.txt",     "hex": "efbbbf6869"},
  {"path": "bad.txt",     "hex": "ff"}
]}
```
- `path`: a valid `RelPath` (the request-path rules of "Paths"); `text`: the file's bytes are
  the string's UTF-8 encoding; `hex`: an even number of hex digits, either case, giving the bytes.
- An empty `files` array is allowed (same as `--memory`).
- Refused before any request runs, exit 2, nothing on stdout, first stderr line
  `kiln-cli: bad --memory-seed <manifest>: <reason>` (the usage text is not printed): unreadable
  file; invalid JSON; a top-level or entry key other than those above; both or neither of
  `text`/`hex`; odd-length or non-hex `hex`; an invalid `path`; the same path twice; a path that
  is a strict prefix directory of another (`d` and `d/x`).

### Inspecting the files
No new command. The files a session leaves are observable through the protocol: after
`doc.close`, a new `doc.open` of the path reads it from the file system, so `doc.info`
(`encoding`, `line_ending`) and `doc.text` show what was saved. Text plus encoding determine the
bytes of a decodable file uniquely (UTF-8 with or without BOM, UTF-16 LE/BE with BOM), so equal
responses mean equal bytes.

### Documentation (Decisions D-1)
New `docs/protocol/cli-memory-seed.md` documents `--memory-seed`, the manifest format, its
refusals and the exit code 2 for a bad manifest. `docs/control-protocol.md` is not edited (no
new error kind); its `kiln-cli` usage block and exit-code table do not mention the option until
T-030 adds a pointer (see T-030). `kiln-cli --help` lists it.

## Pinned
- (a) Errors and precedence: option parsing (usage errors, exit 2, usage text) → root/manifest
  check (`bad --memory-seed`, exit 2, no usage text) → script read (`run`) → requests. Within the
  manifest: JSON → keys → per entry in array order: `path` validity, `text`/`hex`, duplicate,
  prefix conflict (checked after all entries). The `<reason>` text is free; the prefix is exact.
- (b) Files: reads the manifest (host file) once at start; writes nothing to disk (the in-memory
  file system is discarded at exit). Test files: `apps/kiln-cli/tests/acceptance_t011.rs` on
  `task/T-011-tests`, plus the item's `tests` list in `scorecard/items/ws.vfs.toml`.
- (c) Order of checks: as (a); requests run only after the whole manifest is valid.
- (d) Path base, platform, file types: manifest path against the current directory; file paths
  in the manifest are root-relative `RelPath`s inside the memory file system; directories are
  implied by paths; there are no symlinks or special files in memory.
- (e) Threat model: n/a (no disk write, no process, no gate path; the manifest is input the user
  chose, like the script). Hostile manifests (huge, deeply nested) get a structured exit-2 refusal,
  never a panic.
- (f) Tests pinning a snapshot this changes: `apps/kiln-cli/tests/acceptance_t004.rs`
  `run_memory_flag_uses_empty_in_memory_fs`, the `exec --memory` cases and `cli_usage_errors_exit_2`
  (kept green: `--memory` is unchanged); `acceptance_t004_r2.rs` `help_flags_print_usage_and_exit_0`
  (checks `usage` appears; the new line keeps it). The `parse_opts` unit tests in `main.rs` that
  list the allowed options change with the code (implementer's, disclose).
- (g) Worked examples, executed by the planner on today's `main` (R-055): `run` with
  `doc.open a.txt` and `--memory` → `{"error":{"code":"rejected","kind":"not_found","message":"cannot open a.txt: not found"},"id":null,"ok":false}`, exit 3 (the base an
  AC fails on). With the manifest above and a disk root holding the same three files, the
  expected `doc.open` results are the disk results (for example `bad.txt` →
  `decode_error` at byte offset 0, as R-055 observed on disk for a one-byte `\xff` file).
- (h) `ws.vfs`, evaluator's call: `kiln-cli run s.jsonl --memory-seed m.json` and
  `kiln-cli run s.jsonl --root d` with `d` holding the manifest's files; compare stdout byte for
  byte and then compare `d`'s files with the reopened texts (AC-1, AC-2).

## Acceptance criteria
All through the `kiln-cli` binary. Each test writes a manifest and an equal disk directory from
the same data.
- AC-1 (memory and disk agree, `ws.vfs`): for fixed-seed generated file sets (≥ 20 sets of 1–5
  files: UTF-8 with and without BOM, UTF-16 LE/BE with BOM, CRLF/LF/CR, nested directories, and
  one undecodable file) and a fixed-seed script per set that opens, edits, saves, undoes, closes
  with and without `discard`, and finally closes, reopens and reads (`doc.info`, `doc.text`) every
  file, `run --memory-seed` and `run --root` print byte-identical stdout and the same exit code.
  Oracle: differential (memory vs disk).
- AC-2 (saved files can be inspected and equal the disk files): in AC-1's runs, the reopened
  `doc.text` and `encoding` of each file in the memory run, encoded by the test's naive encoder,
  equal the bytes of that file in the disk directory after the disk run. Oracle: byte comparison.
- AC-3 (exec): `exec doc.open '{"path":"a.txt"}' --memory-seed m.json` with `a.txt` in the
  manifest → exit 0 and the same JSON as with `--root` on an equal directory. Oracle: exact value.
- AC-4 (bad manifests): each refusal case of the Interface → exit 2, empty stdout, first stderr
  line beginning `kiln-cli: bad --memory-seed <manifest>: `; a `run` whose script would change a
  file therefore changes nothing. `--memory-seed` with `--root` or `--memory`, or twice → usage
  error, exit 2. Oracle: exact value.
- AC-5 (no regression): `--memory` alone still gives an empty file system (the T-004 tests pass
  unchanged). Oracle: regression.

## Out of scope and implementer notes
- Writing the in-memory files out to disk (`--memory-dump`): a disk write; not needed for AC-2.
- Seeding from a directory (`--memory-from <dir>`): a directory walk on disk is a confinement
  question (T-008's rule for future walks) and would make this task high.
- `control` with an in-memory file system; path confinement (T-008).
- Any change outside `apps/kiln-cli` and the document (if `kiln-control` or `kiln-workspace`
  needs one, stop and report: `Host::in_memory` already takes files).
- Your tests: unit tests in `main.rs`, integration tests in `apps/kiln-cli/tests/memory_seed.rs`;
  your acceptance tests in `apps/kiln-cli/tests/acceptance_t011.rs` on the tests branch first.
  Determinism: generated data from fixed seeds; every spawned process has a timeout.
- Dispatch (R-057): only `apps/kiln-cli/src/main.rs` overlaps another task here (T-007). T-011
  may run in parallel with T-006, T-008, T-009, T-010, T-013 and T-030.

## Decisions
- D-1 (director R-057, all product tasks): a task documents its behaviour in its own new file
  under `docs/protocol/` and touches `docs/control-protocol.md` only to add an error-kind line.
  T-011 adds no kind, so it does not touch that file.

## Questions and rulings
None open.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13T03:50Z · R-055 · draft · did: turned the evaluator's defect report into a spec:
  chose seeding over dropping `--memory` (T-004 tests pin the empty option; the evaluator needs a
  comparison), and a JSON manifest with `text`/`hex` over `--memory-from <dir>` (a directory
  walk would be a confinement boundary, tier high) and over base64 (a new dependency, tier high).
  Inspection by close-and-reopen instead of a new command or a disk dump (no write). Reproduced
  the inert `--memory` with the built `kiln-cli`. Tier medium under ADR-0002 (criterion: CLI
  behaviour with no disk write, process, gate path, security boundary or dependency); budget
  500k; arm implementer (odd number). Pinned (e) n/a. · gate attempts: 0 · not run: new code ·
  missing context: none · next: director review.
- 2026-11-13T03:56Z · R-057 · draft · did: applied D-1 (documentation in new
  `docs/protocol/cli-memory-seed.md`; `docs/control-protocol.md` out of `touches`); dispatch
  note rewritten. · gate attempts: 0 · missing context: none · next: director review.
- 2026-11-13 · R-060 · draft · did: checked against the contracts-first restructuring: T-011 adds no error kind and T-015 renders the kind list from a `kiln-control` binary, not from `kiln-cli`, so `depends_on` stays T-004 and T-011 can run before T-015. Still never in flight with T-007 or T-024 (both edit `apps/kiln-cli/src/main.rs`). · gate attempts: 0 · missing context: none · next: director.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
