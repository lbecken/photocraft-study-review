---
id: T-026
title: "Format an open document with rustfmt through the tool runner as one undo step, keeping its line endings; a formatted document is left untouched and a rustfmt failure changes nothing"
tier: medium            # ADR-0002: no new spawning code (T-024's runner is the only spawner); in-memory edit of a document
state: draft
owner_role: planner
scorecard_items: [fmt.rustfmt, fmt.idempotent, fmt.error]
depends_on: [T-024]     # runners, --tools, recordings
touches:
  - crates/kiln-workspace/src/format.rs      # request building and output handling (slot from T-015)
  - crates/kiln-workspace/tests/format.rs    # implementer's own integration tests
  - crates/kiln-commands/src/fmt.rs          # fmt.document (slot from T-015)
  - apps/kiln-cli/tests/fixtures/t026        # recordings (test author)
  - docs/protocol/fmt.md                     # new
  - docs/commands.json                       # generated
budget_tokens: 500000   # medium (v1.1, provisional)
iteration: 1
found_by: planner
origin_task: null
---

# T-026 Format a document with rustfmt

## Goal
After this task `fmt.document {doc}` sends the document's current text to rustfmt through the
session's tool runner. If rustfmt succeeds, the output replaces the text as **one** undo step,
with the document's line ending kept: rustfmt turns CRLF stdin into LF, as VERIFIED below. An
already formatted document is left exactly as it is, with no undo step and its dirty flag
unchanged. A rustfmt failure is reported with rustfmt's message and changes nothing.

No gate path: the only spawning code is T-024's `ProcessRunner`, which this task does not edit.
Arm: **even** (independent test author).

Reading list: `tasks/T-024-*.md` (runners, `--tools`, recording format); `tasks/T-018-*.md`
(`apply_edits`, mapping); `tasks/T-015-*.md` (reasons `format_error`, tool kinds).

## Interface
`fmt.document {doc}` (mutating) → `{doc, changed, len_chars}`.
1. Request: `ToolRequest { tool: Rustfmt, args: ["--emit", "stdout", "--edition", "2024",
   "--config", "newline_style=Unix"], stdin: Some(<current text>) }`. A `ToolError` →
   `rejected` with T-015's reason (`tool_unavailable`, `tool_not_found`, `tool_timeout`,
   `tool_failed`, `no_recording`).
2. Exit status not 0 → `rejected`/`format_error`. The message is `rustfmt failed: ` followed by
   stderr with ANSI escapes removed, cut to 200 chars by the protocol cap. Nothing changes.
3. Output: stdout. If the document's dominant line ending (`doc.info.line_ending`) is CRLF or
   CR, every `\n` of the output is replaced by it. With LF or none, the output is used as is.
4. If the result equals the current text: `changed: false`, nothing changes (no undo step, and
   the typing group is not ended). Otherwise one `apply_edits` with the single edit replacing
   the range between the longest common char prefix and suffix of old and new text. Selection
   and anchors map by T-018's rule.

No new error kind. `docs/protocol/fmt.md` documents the command, the fixed rustfmt arguments
(the project's `rustfmt.toml`, found from the root as cwd, applies) and the line-ending rule.

## Pinned
- (a) Precedence: schema → `no_such_document` → runner error → `format_error`.
- (b) Files: none written; rustfmt reads its config from the project root (cwd); recordings per
  T-024.
- (c) Order: rules 1–4.
- (d) Offsets chars; edition fixed at 2024 (Out of scope).
- (e) Threat model: n/a beyond T-024 (fixed args; stdin is document text).
- (f) Snapshots: none.
- (g) Worked examples (VERIFIED with rustfmt 1.9.0 of toolchain 1.97.0, `--emit stdout
  --edition 2024`):
  - stdin `fn  main(){let x=1;}` → stdout `fn main() {\n    let x = 1;\n}\n`, exit 0.
  - stdin `fn a() {}\r\nfn  b(){}\r\n` → stdout `fn a() {}\nfn b() {}\n`. So the CRLF document
    becomes `fn a() {}\r\nfn b() {}\r\n` after rule 3. The common prefix is 14 chars
    (`fn a() {}\r\nfn `) and the common suffix 4 (`{}\r\n`), so the one edit replaces chars
    `14..18` (` b()`) with `b() `.
  - stdin `fn main( {` → exit 1, stderr starting `error: this file contains an unclosed
    delimiter` → `rejected`/`format_error`.
- (h) Evaluator calls: `kiln-cli run --root <dir> --tools real` (real tools; opt-in, T-024) or `--tools replay:<rec>` with
  `doc.open`, `fmt.document`, `doc.text`, `doc.undo`, `doc.info`.

## Acceptance criteria
- AC-1 (`fmt.rustfmt`, CLI, replay): with committed recordings made by real rustfmt for ≥ 6
  documents (LF, CRLF, CR-only, mixed, non-ASCII, BOM file), `fmt.document` gives `doc.text`
  equal to the recorded stdout after rule 3. One `doc.undo` restores the exact original, and
  `doc.redo` re-applies. A caret placed before the first changed char stays where it was.
  Oracle: the recorded tool output + round trip.
- AC-2 (`fmt.idempotent`, CLI, replay): on a document whose recorded rustfmt output equals its
  text after rule 3, `fmt.document` gives `changed: false`, and `doc.info` (`dirty`,
  `can_undo`, `can_redo`) and `cursor.get` are unchanged. Oracle: exact value.
- AC-3 (`fmt.idempotent`, the one marked real-rustfmt test of this task):
  `#[test] fn real_rustfmt_is_idempotent_on_corpus`. With `--tools real` (30 s limit per call),
  every `valid` corpus file of T-016 is formatted. A second `fmt.document` gives
  `changed: false`, and the first result equals `rustfmt --emit stdout --edition 2024 --config
  newline_style=Unix` run directly by the test (after rule 3). Oracle: the real tool,
  idempotence.
- AC-4 (`fmt.error`, CLI, replay): recordings of a syntax error (exit 1) and of a timeout give
  `rejected`/`format_error` and `rejected`/`tool_timeout`. The message starts with `rustfmt
  failed: error:`. `doc.text`, `doc.info` and `cursor.get` are unchanged. With no `--tools` option
  (the default `none`) → `tool_unavailable`. Oracle: exact value.
- AC-5 (worked examples): each holds through replayed recordings of exactly those inputs.
  Oracle: exact value.

## Scorecard items and intended test names
| Item | Tests |
|---|---|
| fmt.rustfmt | `kiln_cli::acceptance_t026::format_replaces_text_as_one_undo_step` |
| fmt.idempotent | `kiln_cli::acceptance_t026::formatted_document_is_untouched`, `kiln_cli::acceptance_t026::real_rustfmt_is_idempotent_on_corpus` |
| fmt.error | `kiln_cli::acceptance_t026::rustfmt_failure_changes_nothing` |

## Out of scope and implementer notes
- Detecting the edition from `Cargo.toml`; formatting a range; format on save; `cargo fmt`.
- The no-oracle part: whether rustfmt's output is "good" is rustfmt's business. This task's
  oracle is rustfmt's own output and its idempotence.
- Your tests: `crates/kiln-workspace/tests/format.rs` (with a test `ToolRunner`, never real
  rustfmt), unit tests.

## Questions and rulings

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13 · R-056 · (new) · did: specified formatting through T-024's runner. Ran real
  rustfmt: it converts CRLF stdin to LF, so rule 3 restores the document's dominant line
  ending, and a minimal-range edit keeps the caret. Tier medium: the process is spawned only by
  T-024's runner, which this task does not touch, so no ADR-0002 high criterion applies (the
  director may judge "spawns processes" to cover callers too; then high). Arm even. Pinned (e),
  (f) n/a. · gate attempts: 0 · missing context: none · next: director.
- 2026-11-13 · R-060 · draft · did: tier medium confirmed (R-060 answer 7: callers through the runner trait are medium). Real tools only with the explicit `--tools real` (T-024's opt-in default `none`). · gate attempts: 0 · missing context: none · next: director.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
