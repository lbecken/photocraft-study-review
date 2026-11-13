---
id: T-022
title: "Search all project files with results grouped by file with line and column, and replace across files from a stored preview, each file's change one undo step in its document"
tier: medium            # ADR-0002: reads project files through the existing Vfs read path (no new disk code), writes nothing to disk; replacements go into open documents
state: draft
owner_role: planner
scorecard_items: [ws.find-in-files, ws.replace-in-files]
depends_on: [T-019, T-021]   # project walk; Query
touches:
  - crates/kiln-workspace/src/find.rs        # slot from T-015
  - crates/kiln-workspace/tests/find.rs      # implementer's own integration tests
  - crates/kiln-commands/src/find.rs         # find.* + FindState (slot from T-015)
  - docs/protocol/find.md                    # new
  - docs/commands.json                       # generated
budget_tokens: 500000   # medium (v1.1, provisional)
iteration: 1
found_by: planner
origin_task: null
---

# T-022 Find and replace in files

## Goal
After this task `find.in-files` runs a T-021 query over every file of the project tree
(T-019's walk, kinds `file` only) and returns the matches grouped by file, each with char
offsets, a 0-based line and column, and the line's text. `find.replace-preview` computes and
stores the replacements a query would make. `find.replace-apply` applies a preview to documents:
it opens files that are not open, then makes each file's replacements one undo step in its
document. It refuses, changing nothing, if any affected file changed since the preview. Nothing
is written to disk; the user saves with `doc.save`.

No gate path. Arm: **even** (independent test author).

Reading list: `tasks/T-019-*.md` (walk rules), `tasks/T-021-*.md` (Query, matching rules),
`tasks/T-018-*.md` (`apply_edits`), `tasks/T-015-*.md` (`FindState`, `Parts`, `Docs`),
`crates/kiln-commands/src/reason.rs` (reasons `stale_preview`, `no_such_preview`,
`invalid_pattern`, declared by T-032), `tasks/T-017-*.md` rulings s1/s2.

## Interface

### Text of a file
The current text of the open document if the path is open, otherwise the file decoded as
`doc.open` would decode it. A file that fails to decode is skipped with reason `decode_error`.
A text that contains a NUL char is skipped as `binary`.

### Commands
Query params as in T-021 (`query`, `regex?`, `case_sensitive?`, `whole_word?`), plus
`path?` (subtree, as for `project.tree`) and `limit?` (1..=100000, default 10000).

| Command | Mutating | Extra params | Result |
|---|---|---|---|
| `find.in-files` | no | — | `{files:[{path, matches:[Match]}], total, truncated, skipped:[{path, reason}]}` |
| `find.replace-preview` | yes | `replacement` | `{preview, files:[{path, replacements:[Match + {new}]}], total}` |
| `find.replace-apply` | yes | `preview`; `paths?` list (default all files of the preview) | `{files:[{path, doc, replaced, opened}], replaced}` |

`Match` = `{start, end, line, col, line_text}`. `line`/`col` are 0-based and give the
position of `start` (chars). `line_text` is the first 200 chars of that line without its line
break.

- Files appear in project-tree order. A file without matches is not listed. `total` counts all
  matches. Only the first `limit` matches are listed; once a file reaches the limit it is cut
  there, and later files are dropped, with `truncated: true`. `skipped` lists every file
  skipped, in tree order.
- `find.replace-preview` stores, under a new id (1, 2, 3, … per session, never reused), for
  each file with matches: its text at preview time and its replacement edits. The replacement
  texts come from T-021's `CompiledQuery::replacements` per file (linear in the file's text
  plus its matches; one `replacement_for` call per match would be O(text × matches)), which
  returns `Result<Vec<String>, TemplateError>`. A template naming a group that does not exist
  is refused before the walk, as T-021 refuses it before matching (D-044): `rejected`/
  `invalid_pattern`, message `invalid replacement: <TemplateError Display>`, nothing stored,
  no id used. So the `Err` never arises per file. There is no limit on a preview: it holds
  every match. Its result lists `new`, the replacement text of each match.
- `find.replace-apply` first checks, for every file selected by `paths`, that its current text
  equals the preview's snapshot. A path not in the preview → `rejected`/`invalid_path`. Any
  difference → `rejected`/`stale_preview`, and nothing changes. Then, in tree order, it opens
  each file that is not open (as `doc.open`, so the next ids are used, `opened: true`) and
  applies that file's edits with one `Buffer::apply_edits`. On success the preview is removed;
  a later use of its id → `rejected`/`no_such_preview`.
- Refusals use existing kinds only (T-032's reason table, `outside_root` from T-008):
  `invalid_pattern` (T-021's messages, including `invalid pattern: the pattern has more than
  100 capture groups`, T-017 s1, and `invalid replacement: …`), `invalid_path`, `not_found`,
  `not_a_file`, `outside_root`, `io_error` (from the walk), `stale_preview`,
  `no_such_preview`. No new kind.

`docs/protocol/find.md` documents all of this.

## Pinned
- (a) Precedence: schema → query (`invalid_pattern`: the pattern, then, for
  `find.replace-preview`, the replacement template) → walk errors (T-019's) → (apply only)
  preview exists → paths valid → staleness of every selected file → apply.
- (b) Files: read only, through `Vfs::read`; nothing written. The preview is held in memory in
  `FindState`, never on disk.
- (c) `find.replace-apply` checks every selected file before changing anything (all or
  nothing). Opening a file that the preview names but that was deleted since → `stale_preview`.
- (d) Offsets are chars of each file's text; paths are RelPaths; tree order as T-019.
- (e) Threat model: n/a beyond T-019's walk, which is reused unchanged; no new disk access path.
  A replacement never reaches disk without an explicit `doc.save`.
- (f) Snapshots: none.
- (g) Worked example (executed against a naive Python model): files `a.rs` = `let foo = foo_x;\n`,
  `b/c.rs` = `x\r\nfoo\r\n`, `bin.dat` = `foo\0`. `find.in-files {query:"foo", whole_word:true}`
  → files `a.rs` (match `4..7`, line 0, col 4, `line_text` `let foo = foo_x;`) and `b/c.rs`
  (match `3..6`, line 1, col 0, `line_text` `foo`); `total: 2`; `skipped: [{path:"bin.dat",
  reason:"binary"}]`. `find.replace-preview {…, replacement:"bar"}` → `preview: 1`.
  `find.replace-apply {preview:1}` → docs 1 (`a.rs`) and 2 (`b/c.rs`), each `replaced: 1`,
  `opened: true`. `doc.text {doc:1}` = `let bar = foo_x;\n`. `doc.undo {doc:1}` restores it.
  The files on disk are unchanged.
- (h) Evaluator calls: `kiln-cli run --root <dir>` with `find.in-files`; with
  `find.replace-preview`, `find.replace-apply`, `doc.undo`, `doc.save`.

## Acceptance criteria
- AC-1 (`ws.find-in-files`, CLI): for ≥ 50 fixed-seed random projects (files with LF, CRLF,
  CR, `é`, `😀`, `.gitignore`, a non-UTF-8 file, a NUL file) and random queries, the result
  equals a naive model. The model takes the files from `project.tree`, reads the bytes from
  disk, applies T-021's rules per file (naive literal scan, or naive regex subset) and computes
  line/col by a naive split on `\r\n|\r|\n`. Oracle: naive model.
- AC-2 (`ws.find-in-files`, open documents win): after `doc.insert` into an open file,
  `find.in-files` reports matches against the edited text, not the disk text. Oracle: exact
  value.
- AC-3 (`ws.replace-in-files`, CLI): for random projects, queries and templates,
  `replace-preview` then `replace-apply`, then `doc.text` of each document, equals the naive
  per-file replacement. One `doc.undo` per document restores its original text. After `doc.save`
  of every document, the files on disk equal the naive result. Files never opened by the apply
  are byte-identical on disk. Oracle: naive model + round trip.
- AC-4 (`ws.replace-in-files`, stale): after `replace-preview`, (i) editing one of its files in
  an open document, or (ii) changing one file on disk (not open), makes `replace-apply`
  `rejected`/`stale_preview`. No document is opened or changed, and `doc.list` is unchanged.
  `paths` naming only the unchanged files succeeds. Oracle: exact value.
- AC-5 (`ws.replace-in-files`, preview ids): a second apply of the same preview →
  `no_such_preview`; ids increase from 1. The worked example holds exactly. A
  `find.replace-preview` with regex `(\w)(\d)` and replacement `$3` → `rejected`/`invalid_pattern`
  with message `invalid replacement: no capture group "3"`, and the next successful preview
  still gets the next unused id (no id consumed by the refusal). Oracle: exact value.

## Scorecard items and intended test names
| Item | Tests |
|---|---|
| ws.find-in-files | `kiln_cli::acceptance_t022::find_in_files_matches_naive_model`, `kiln_cli::acceptance_t022::open_documents_are_searched_as_edited` |
| ws.replace-in-files | `kiln_cli::acceptance_t022::replace_in_files_matches_naive_model_and_undoes_per_file`, `kiln_cli::acceptance_t022::stale_preview_changes_nothing`, `kiln_cli::acceptance_t022::preview_ids_and_worked_example` |

## Out of scope and implementer notes
- Include/exclude globs; saving from the apply; streaming results; parallel search.
- The item file says `tier = "high"` for `ws.replace-in-files`; the director ruled medium
  (R-060 answer 5). The test author sets `tier = "medium"` in
  `scorecard/items/ws.replace-in-files.toml` on `task/T-022-tests` (an allowed path for this
  task's item) and regenerates `docs/scorecard.md`.
- Your tests: `crates/kiln-workspace/tests/find.rs`, unit tests.

## Questions and rulings

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13 · R-056 · (new) · did: specified find and replace in files on T-019's walk and
  T-021's query. Replace goes into documents (one undo step each, never saved implicitly),
  through a stored preview with an all-or-nothing staleness check. Tier medium: it writes no
  file and reuses T-019's reads, so no ADR-0002 high criterion applies, although the item file
  says high (Q-6). Arm even. The worked example was executed against a naive model. Pinned (e),
  (f) n/a beyond reuse. · gate attempts: 0 · missing context: none · next: director.
- 2026-11-13 · R-060 · draft · did: item tier ruled medium; the test author changes the item file on the tests branch. · gate attempts: 0 · missing context: none · next: director.
- 2026-11-13 · R-066 · draft · did: `stale_preview`/`no_such_preview` are T-032's (D-041); T-032's description of `stale_preview` now covers a removed file (finding 25). Transitively after T-032 via T-021. · gate attempts: 0 · missing context: none · next: director.
- 2026-11-13T12:55Z · R-112 · draft · did: `replacement_for` returns `Result` (T-017 `expand`,
  T-021), which the preview text ignored: the template is now refused before the walk with
  T-021's `invalid_pattern` message (D-044), no preview id used (AC-5); replacement texts come
  from T-021's new linear `replacements` (T-017 implementer's cost note); the s1 capture-group
  message is listed; kinds are T-032's, not T-015's. · gate attempts: 0 · not run: no cargo ·
  missing context: none · next: director.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
