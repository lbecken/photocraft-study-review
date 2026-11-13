---
id: T-011
title: "Let kiln-cli seed the in-memory file system from a manifest, so memory and disk sessions can be compared"
tier: medium            # ADR-0002: a CLI option that fills the in-memory VFS; reads one host file given on the command line (as `run` reads its script), writes nothing to disk, spawns nothing, no boundary or dependency
state: merged
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
Director rulings after review round 1 (2026-11-13, decision D-067; review `reviews/T-011-r1.md`, run R-119: code correct in every probe; one test gap, three spec gaps, two nits). In this arm the implementer owns the acceptance tests; each test change is its own commit on `task/T-011-tests` that says what the spec requires.
- u1 (test gap). A manifest that holds a file `d` and a file `dx/y` is accepted: a path is in conflict with another only when one is a directory prefix of the other at a `/` boundary. The acceptance test that claims to cover this must really seed both files and read both back; the unit test that has it the wrong way round is corrected. The reviewer's surviving mutant (a string-prefix conflict check) must be caught by an acceptance test.
- u2 (duplicate keys). A manifest in which any JSON object repeats a key is malformed and is refused like every other malformed manifest (same exit code and refusal prefix; the reason text is free). `{"files":[…],"files":[]}` must not seed an empty file system. One acceptance test.
- u3 (`exec` order). For `exec`, invalid params JSON is reported before the manifest is read, as it already is with `--root`. This is now stated in `docs/protocol/cli-memory-seed.md` and held by one acceptance test.
- u4 (order of checks inside a manifest). Not observable through the CLI because the reason text is free; it stays a unit-test matter. No change.
- u5 (nits). The AC-1 generator makes sets of 1–5 files as the spec says; the "second session starts from the manifest" check asserts what its comment claims.

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
- 2026-11-13T13:10Z · R-114 · ready · did: restated ACs: (1) a fixed-seed generated script
  gives byte-identical stdout and exit code with `--memory-seed m.json` and with `--root d`
  on an equal tree; (2) the reopened text+encoding of every memory file, naively encoded,
  equals the disk file after the disk run; (3) `exec … --memory-seed` = `exec … --root`;
  (4) every bad manifest → exit 2, empty stdout, prefix `kiln-cli: bad --memory-seed
  <manifest>: `, no request run; conflicts/twice → usage error; (5) `--memory` unchanged.
  Step 0 (spec vs merged code, T-015/T-032/T-016/T-013): no mismatch that blocks;
  `RelPath::new`, `Host::in_memory(&[(&str,&[u8])])`, `Context::with_files`, `doc.info`
  `encoding`/`line_ending`, kind `decode_error` exist as named; T-032 reasons touch no CLI
  usage error; gate has 11 steps incl. `hygiene`; `docs/protocol/` did not exist (created).
  Drift only: exec's usage line now wraps to two lines in `--help`.
  Tests first on `task/T-011-tests`: commit `76c7c83d720b60311d4ca2ad94c817f209ea29dd`
  at 2026-11-13T12:57:42Z (amend, same minute, of `73e8fa88cfd5b15c46644ea33314080b5b898053`
  at 12:57:34Z to drop trailers that AGENTS.md rule 8 forbids; no code existed yet). 9
  acceptance tests in `apps/kiln-cli/tests/acceptance_t011.rs` all failed on base by
  assertion (unknown option `--memory-seed`; AC-1/AC-2 after their generator self-checks
  passed on the disk runs); 3 guards in `guard_t011.rs` passed on base; `ws.vfs` tests list
  += AC-1, AC-2. No test changed after that commit. Then on `task/T-011`:
  `ff5cc7f29acd36da616d50102874822af87af2f5` (option, manifest parser, usage),
  `ca2be9fbde0eca13b67945b2bed37310d34ca01b` (unit tests incl. extended `parse_opts`
  test — disclosed per Pinned (f), only additions; `tests/memory_seed.rs`),
  `24254895558b1498d56f40944f37ded70a91977f` (`docs/protocol/cli-memory-seed.md`).
  Self-check: throwaway merge of the tests branch → 9/9 acceptance + 3/3 guards pass
  (branch deleted); 3 hand mutants (prefix check off, duplicate check off, long hex
  refused) each killed by acceptance tests. · gate attempts: 1 (last: green, 11 steps:
  fmt, clippy, layers, ui-commands, hygiene, schema, scorecard, test 531 passed, no-skip,
  fuzz EMPTY, budgets EMPTY) · not run: gate on the joined branch (the join does it);
  Windows paths · missing context: none · next: director join.
  QUESTION: Pinned (a) puts "root/manifest check" before "requests"; for `exec` I kept the
  existing order (`<json-params>` that are not JSON are reported before the root, and now
  the manifest, is checked), so `exec x '{' --memory-seed bad.json` reports `invalid_json`.
  Untested by the acceptance tests; documented. Should the manifest come first for `exec`?
  QUESTION: I read `<manifest>` in `kiln-cli: bad --memory-seed <manifest>: ` as the argument
  as given (like `<reason>`, a placeholder); the acceptance tests pin that reading.
  QUESTION: "the same for `--memory`" implemented as ``options `--memory` and `--memory-seed`
  cannot be combined``; tests assert only both option names and "cannot be combined".
- 2026-11-13T13:35Z · R-121 · review r1 request-changes, rulings u1–u5 (D-067) · did: test
  commits on `task/T-011-tests`, each saying what the ruling requires:
  u1 `9645f083cb0ebd7ab30dcfdd0e0128b6b27f6d14` (13:28:44Z; `ac4_valid…` now seeds `c/x`,
  `c/y`, `dx/y`, `d` and reads `d` and `dx/y` back; the old manifest had no file `d`);
  u5 `1d728f5c410f997f7d8ef876b46748e811f48d23` (13:30:11Z; generator 1–5 files incl. the
  undecodable one, asserts `1..=5`; second session runs a read-only script and asserts the
  manifest's texts); u3 `905bcb47d3803be5cf374374c2b0f94e753c21be` (13:30:24Z;
  `ac4_exec_reports_invalid_params_json_before_the_manifest`); u2
  `30faadd33f09a0da9702f54e7a4a9b77cb3facbe` (13:30:40Z; `ac4_repeated_json_keys_are_refused`).
  All new/changed tests fail on base by assertion (unknown option); in a throwaway merge with
  the pre-u2 code only the u2 test failed. Reviewer's M1 (string-prefix check,
  `starts_with(p)`) re-seeded: killed by `ac4_valid_manifest_edge_cases_are_accepted`;
  reverted. Then on `task/T-011`: `2627f4024191c3d51e608abf26a0b6e6464fd496` (13:31:41Z):
  u2 implemented as a key scan over the already-validated JSON (keys unescaped, compared per
  object; no new dependency), unit cases added; unit test u1 corrected (`d` + `dx/y`); doc
  states u1, u2 and u3. u4: nothing. Throwaway merge of the tests branch: 11/11 acceptance,
  3/3 guards pass (branch deleted). · gate attempts: 1 (last: green, 11 steps, test 543
  passed; fuzz, budgets EMPTY). The gate ran the OLD joined copy of the acceptance tests on
  this branch; the four new/changed tests run only after the director's re-join. · not run:
  gate on the re-joined branch · missing context: none · next: director re-join.
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
- 2026-11-13T13:24Z · R-119 · tests_joined · did: round-1 review of
  `47a4c23151423ea70ec9df1faff78b6e7b50be22` (`reviews/T-011-r1.md`): acceptance/guard files
  unchanged since `76c7c83` (both branches); CLI probes incl. the doc's memory-vs-disk example;
  19 mutants (1 equivalent, 17 killed, 15 by acceptance, 1 survived: string-prefix conflict
  check, test gap). Verdict request-changes (1 test gap); 3 spec gaps for the director
  (duplicate JSON keys, `exec` params-vs-manifest order, within-manifest order unobservable),
  2 nits. · gate attempts: 0 (ran `kiln-cli` package tests per mutant and `cargo xtask
  hygiene`) · not run: full gate, permission-denied manifest (uid 0), Windows · missing
  context: none · next: implementer fixes finding 1; director rules on findings 2–4.
<!-- end reviewer log -->
