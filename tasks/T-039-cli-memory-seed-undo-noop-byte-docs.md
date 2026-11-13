---
id: T-039
title: "Document `--memory-seed` in control-protocol.md, what a `doc.undo` or `doc.redo` with nothing to do leaves unchanged, and that `byte` counts the decoded text, with tests that pin each"
tier: low               # ADR-0002: documentation and tests; two command descriptions (DOC strings) change, no behaviour
state: queued
owner_role: planner
scorecard_items: []     # low tier carries no scorecard item (METHODOLOGY §4)
depends_on: []          # T-011 merged (queue record df1eb20d05d9f12389c221bc8d65761d401dc61e); T-018 merged at 7a2678498df1cb80fa96f06947e919b3f396619d
touches:
  - docs/control-protocol.md                  # implementer: the `kiln-cli` section and its exit-code table only (D-091 change 18)
  - crates/kiln-commands/src/doc.rs           # implementer: the DOC strings of `doc.undo` and `doc.redo` only
  - docs/protocol/doc-positions.md            # implementer: one paragraph in "`doc.position`"
  - apps/kiln-cli/tests/acceptance_t039_cli.rs  # implementer: the document checks (fail on base)
  - apps/kiln-cli/tests/guard_t039_cli.rs     # implementer: the behaviour the documents state (passes on base)
  # docs/commands.json changes too (the two DOC strings): generated, `cargo xtask schema --write`
budget_tokens: 150000   # low (D-091 change 3)
iteration: 2
found_by: evaluator     # evaluations/iter-1.md NEW-4, Q1, Q3 (run R-152); answers D-090
origin_task: T-011      # NEW-4; Q1 concerns T-018's typing-group rule, Q3 T-010's positions
---

# T-039 `--memory-seed` in control-protocol.md; no-op undo and redo; `byte` counts decoded text

## Goal
Three documentation gaps, each pinned by a test:
- **(a) NEW-4.** `docs/control-protocol.md` "`kiln-cli`" lists `run … [--root <dir> |
  --memory]` and `exec … [--root <dir> | --memory]` (:157, :159), says `--memory` "cannot be
  combined with `--root`" (:173–174) and has no bad-manifest case in the exit-code table
  (:193, :195). `kiln-cli --help` and `docs/protocol/cli-memory-seed.md` have
  `--memory-seed <manifest>` for `run` and `exec`. After this task the protocol document says
  what the binary does, and a test compares its synopsis with `kiln-cli --help` so the two
  cannot drift apart again (D-091 change 18: a task that changes the CLI surface updates
  these sections).
- **(b) Q1.** No document says what a `doc.undo` or `doc.redo` with nothing to do leaves
  unchanged. D-090 accepts the behaviour; after this task the two commands' descriptions
  (printed by `kiln-cli commands`, generated into `docs/commands.json`) state it, and guard
  tests pin it.
- **(c) Q3.** `docs/protocol/doc-positions.md` says "`{"byte": n}`: a UTF-8 byte offset" (:46)
  but not of what. After this task it says that `byte` counts UTF-8 bytes of the document's
  decoded text, not of the file, with one example, pinned by a guard.

No behaviour changes. No gate path, no dependency. Tier low by ADR-0002.

Reading list: `evaluations/iter-1.md` NEW-4, Q1, Q3; `docs/decisions.md` D-090, D-091 (change
18); `docs/control-protocol.md` "`kiln-cli`" and "Exit codes"; `docs/protocol/cli-memory-seed.md`;
`apps/kiln-cli/src/main.rs`:40–61 (the usage text); `crates/kiln-commands/src/doc.rs` `Undo`,
`Redo`; `crates/kiln-text/src/history.rs` (`record`, `pop_undo`, `pop_redo`, `typing_open`);
`crates/kiln-text/src/buffer.rs`:338–375; `docs/protocol/doc-positions.md`; `docs/protocol/eol.md`
"Nothing to do changes nothing" (the model for (b)).

## Interface
### (a) `docs/control-protocol.md`, "`kiln-cli`" section
- The synopsis block becomes exactly:
  ```text
  kiln-cli run <script> [--root <dir> | --memory | --memory-seed <manifest>] [--log <file>]
  kiln-cli control [--root <dir>]
  kiln-cli exec <name> <json-params> [--text <initial-text>] [--root <dir> | --memory | --memory-seed <manifest>]
  kiln-cli commands [--schema | --audit]
  kiln-cli --help | -h | <subcommand> --help
  ```
  (the `run` and `exec` lines are those of `docs/protocol/cli-memory-seed.md`:11–12).
- The bullet "`--memory` (for tests) uses an empty in-memory file system instead; it cannot be
  combined with `--root`." is replaced by these three bullets, exactly:
  - "`--memory` (for tests) uses an empty in-memory file system instead."
  - "`--memory-seed <manifest>` (`run` and `exec`) uses an in-memory file system holding
    exactly the files of a JSON manifest, read once at start; nothing is written to disk. The
    manifest, its refusals and the order of checks are in `docs/protocol/cli-memory-seed.md`."
  - "`--root`, `--memory` and `--memory-seed` exclude each other: giving two of them is a usage
    error. `control` takes neither `--memory` nor `--memory-seed`."
- Exit-code table, column 2: the `run` cell becomes "usage error, unreadable script, root not
  a directory, bad `--memory-seed` manifest, log file not writable (message on stderr, nothing
  on stdout)"; the `exec` cell becomes "usage error, bad root, bad `--memory-seed` manifest,
  invalid JSON, command error". No other cell changes.
- Nothing else in the document changes.

### (b) Command descriptions (`crates/kiln-commands/src/doc.rs`)
- `doc.undo` DOC, exactly: "Undo the most recent undo step of a document. A successful undo
  ends the typing group. With nothing to undo (`undone: false`) nothing changes: not the text,
  the dirty flag, the selection or the redo history; no typing group is open then, because an
  open typing group is itself the most recent undo step."
- `doc.redo` DOC, exactly: "Redo the most recently undone step of a document. A successful
  redo ends the typing group. With nothing to redo (`redone: false`) nothing changes: not the
  text, the dirty flag, the selection, the undo history or an open typing group, so a one-char
  insert after it still joins the typing before it."
- `docs/commands.json` regenerated. Nothing else in `doc.rs` changes.

### (c) `docs/protocol/doc-positions.md`, "`doc.position`"
After the paragraph "All numbers are integers ≥ 0. Result: …" (:50–51), one paragraph, exactly:
"`byte` counts the UTF-8 bytes of the document's text as Kiln decoded it, not the bytes of the
file: a byte-order mark is not counted, and the text of a UTF-16 file is counted in UTF-8. For
example, a UTF-8 file with a BOM holding `héx` (file bytes `EF BB BF 68 C3 A9 78`) gives
`{"char": 2}` → `"byte": 3`, although the `x` is at offset 6 of the file; the same text in a
UTF-16 LE file (file bytes `FF FE 68 00 E9 00 78 00`) also gives `"byte": 3` (file offset 6).
A tool that reports offsets into the file must convert them."

## Pinned
- (a) Errors: n/a (no behaviour change; the refusals the documents name already exist).
- (b) Files: the three documents and `doc.rs` as above; `docs/commands.json` regenerated
  (only the two `doc` texts differ).
- (c) Order of checks: n/a.
- (d) Paths, platform: n/a beyond the tests (in-memory sessions through `--memory-seed`).
- (e) Threat model: n/a (no gate or security path).
- (f) Tests pinning what this changes: none found. No test reads the `kiln-cli` section, the
  exit-code table or the synopsis of `docs/control-protocol.md` (`grep` of `run <script>`,
  `cannot be combined with`, `root not a directory` in `crates/*/tests`, `apps/*/tests`,
  `xtask`); the tests that read the document read the kinds table, its paragraph and the Paths
  bullets, which do not change. No test pins the two DOC strings (`grep` "Undo the most
  recent undo step"); `kiln-cli commands` output is excluded from guards (ADR-0004
  "Consequences"). `acceptance_t010.rs`:1112–1128 requires only words in `doc-positions.md`
  that stay.
- (g) Worked examples, executed by this planner with the debug `kiln-cli` of main
  `73a8d13c48cc09e7ac9f238d4848c888341d9fb0` (no file this task concerns changed up to
  `d8c4a4199ca622f8a46dd09dcccb50b9f4bc4003`), `kiln-cli run s.jsonl --memory-seed m.json`
  with `a.txt` = `0123456789` (doc 1):
  - No-op redo: insert `a` at 10 → `len_chars` 11; `doc.redo` →
    `{"dirty":true,"len_chars":11,"redone":false}`; insert `b` at 11; `doc.info` →
    `can_redo:false, can_undo:true`; one `doc.undo` → `{"dirty":false,"len_chars":10,"undone":true}`,
    text `0123456789` (both inserts in one step). Without the redo, the same.
  - Successful redo ends the group: insert `x` at 0, undo, insert `a` at 10, undo, `doc.redo` →
    `redone:true`, insert `b` at 11, `doc.undo` → `len_chars` 11, text `0123456789a` (only `b`
    undone).
  - No-op undo: `cursor.set` 3..5; `doc.undo` → `{"dirty":false,"len_chars":10,"undone":false}`;
    `doc.info` `can_undo:false, can_redo:false`; `cursor.get` anchor 3, head 5. Then insert
    `xy` at 0, undo, `doc.undo` → `undone:false`; `doc.info` → `can_redo:true`; `doc.redo` →
    `redone:true`, text `xy0123456789` (the no-op undo kept the redo step).
  - No-op redo with a selection: `cursor.set` 2; `doc.redo` → `redone:false`; `cursor.get`
    anchor 2, head 2; `doc.info` unchanged.
  - Successful undo ends the group: insert `a` at 0, insert `z` at 5 (not adjacent: a new
    step), `doc.undo` → `{"dirty":true,"len_chars":11,"undone":true}`, insert `b` at 1 (it
    would continue the `a` step), `doc.undo` → `len_chars` 11, text `a0123456789` (only `b`
    undone).
  - That a no-op undo can only happen with no open typing group: VERIFIED by reading
    `history.rs`: `typing_open` becomes true only when `record` pushes a step (:75–88) and
    `pop_undo` clears it (:105–109), so an open group is always the top undo step.
  - `exec doc.list {} --memory --root .` → exit 2, ``kiln-cli: options `--root` and `--memory`
    cannot be combined`` (the third pair of the "exclude each other" bullet).
  - Bytes: `bom.txt` = `EF BB BF 68 C3 A9 78` → `encoding` `utf-8-bom`, `doc.text` `héx`;
    `{"char":2}` → `{"byte":3,"char":2,"col":2,"line":0,"utf16":2}`; `{"char":3}` → `byte` 4;
    `{"byte":3}` → `char` 2. `u16.txt` = `FF FE 68 00 E9 00 78 00` → `utf-16le`; `{"char":2}`
    → `byte` 3; `{"char":3}` → `byte` 4.
  - CLI: `run x.jsonl --memory --memory-seed m.json` → exit 2, ``kiln-cli: options `--memory`
    and `--memory-seed` cannot be combined``; `--root . --memory-seed` → exit 2, ``… `--root`
    and `--memory-seed` cannot be combined``; `control --memory-seed m.json` and `control
    --memory` → exit 2, ``kiln-cli: unknown option `--memory-seed` for `control` `` (and
    `--memory`); a manifest `{"files":[],"files":[]}` with `run` and with `exec doc.list {}` →
    exit 2, empty stdout, ``kiln-cli: bad --memory-seed bad.json: key `files` repeated in one
    object``. `kiln-cli --help` prints the `run` and `exec` usage with `--memory-seed
    <manifest>`.
- (h) Scorecard items: none.

## Acceptance criteria
Every child process has a hard timeout of 10 s. No test returns early or skips. No test
compares a whole document. Base = main `d8c4a4199ca622f8a46dd09dcccb50b9f4bc4003`.

- **AC-1 (fails on base; `acceptance_t039_cli.rs::protocol_synopsis_matches_help`).** For each
  of `run`, `exec`, `control` and `commands`: the set of tokens beginning with `--` on its line
  of the synopsis block under "## `kiln-cli`" equals the set in its usage entry of `kiln-cli
  --help` (the line beginning `kiln-cli <subcommand>` and its continuation lines). Oracle: the
  binary's own usage text. Today `run` and `exec` lack `--memory-seed` in the document.
- **AC-2 (fails on base; `acceptance_t039_cli.rs::protocol_states_memory_seed`).** The
  document, whitespace-squashed, contains the three bullets of Interface (a) and the two
  exit-code cells exactly, does not contain "it cannot be combined with `--root`.", and its
  `control` row of the exit-code table does not mention `--memory-seed`.
- **AC-3 (guard; `guard_t039_cli.rs::memory_seed_conflicts_and_bad_manifest`).** The CLI cases
  of Pinned (g): exit codes, empty stdout and the first stderr line as shown. Pins what AC-2's
  sentences state.
- **AC-4 (fails on base; `acceptance_t039_cli.rs::undo_and_redo_describe_the_no_op`).**
  `kiln-cli commands` prints `doc.undo` and `doc.redo` with the DOC texts of Interface (b)
  exactly; `kiln-cli commands --schema` equals `docs/commands.json`.
- **AC-5 (guard; `guard_t039_cli.rs::noop_undo_and_redo_change_nothing`).** Through `kiln-cli
  run --memory-seed`, the five sessions of Pinned (g) (no-op redo, successful redo, no-op undo,
  no-op redo with a selection, successful undo), every response asserted exactly. Oracle:
  exact values, computed beforehand from the grouping rule of `crates/kiln-text/src/lib.rs`
  "Undo grouping" and checked by this planner's probes.
- **AC-6 (fails on base; `acceptance_t039_cli.rs::doc_positions_counts_decoded_bytes`).**
  `docs/protocol/doc-positions.md`, whitespace-squashed, contains the paragraph of Interface
  (c) exactly.
- **AC-7 (guard; `guard_t039_cli.rs::byte_offsets_ignore_bom_and_encoding`).** The byte cases of
  Pinned (g) through `kiln-cli run --memory-seed` with `hex` entries, every response exactly.

## Out of scope and implementer notes
- Any behaviour change, including what a no-op undo or redo does to the session log (both are
  logged as successful mutating requests, `registry.rs`:318–322; unchanged and not restated).
- The `--root` bullet and the "root not a directory" cells (T-036's, if its unreadable-root
  refusal is accepted: D-091 change 18 makes it update them); `--tools` (T-024); T-030's
  sentences.
- `docs/protocol/cli-memory-seed.md`, `docs/protocol/cursor.md`, `docs/protocol/eol.md`:
  unchanged.
- The `doc.undo`/`doc.redo` result field docs ("nothing to undo, nothing changed") stay.
- Low tier: the implementer writes these tests and runs the gate; no independent test author,
  no review. The file names follow the `acceptance_`/`guard_` convention so the join tool keys
  them uniquely; the director decides whether the join's "no `acceptance_t*` on the implementer
  branch" check applies to a low-tier task.

## Conflicts and dispatch
- `docs/control-protocol.md`: also edited by T-038 (the paragraph after the kinds table),
  T-036 (the `--root` bullet and exit cells, under D-091 change 18), T-024 (`--tools`, stage 2)
  and T-030 (stage 2). Never in flight with any of them. AC-1 makes every later task that
  changes `kiln-cli --help` also update the synopsis line, which is D-091 change 18's rule:
  T-024's "one line in the `kiln-cli` section" becomes the synopsis lines plus that line, and
  T-030's sentence pointing to `docs/protocol/` may become unnecessary (director's note to
  those drafts).
- `crates/kiln-commands/src/doc.rs`: also T-037 (doc.open) and T-038 (declarations); not in
  flight together. `docs/protocol/doc-positions.md`: also T-038 (the parenthesis at :87–88).
- Proposed order: this task first (smallest, no code behaviour), then T-037, then T-038. T-036
  may run beside this task only if it does not edit `docs/control-protocol.md`; under D-091
  change 18 it does, so after it.
- `docs/commands.json` is generated: regenerate on conflict.

## Questions and rulings
- Q-1 (planner, R-154): where (b) is stated. Recommended: the two DOC strings, because they are
  the commands' description every client reads (`kiln-cli commands`, `docs/commands.json`).
  Alternative: a new page `docs/protocol/undo.md`, which touches no code file and so does not
  meet T-037 and T-038 in `doc.rs`, but is one more page a client must find.
- Ruling on Q-1 (director, D-092): the two DOC strings, as recommended. T-039 is dispatched first of the tasks that share `doc.rs`.
- Ruling z1 (director, D-094; METHODOLOGY v1.2 §6, no test pins a message text or a document that a drafted task changes). AC-2, AC-6 and the first half of AC-4 are not tests: no test asserts that `docs/control-protocol.md`, `docs/protocol/doc-positions.md` or the printed description of `doc.undo`/`doc.redo` contains a given sentence (T-036 edits the same CLI section and exit-code table; T-037 and T-038 edit command descriptions). The implementer copies the sentences of Interface (a), (b) and (c) into the documents and the two DOC strings, and the director compares them with the Interface on the diff before queueing (low tier has no reviewer). The second half of AC-4 is already the gate's `schema` step and needs no test. AC-1 stays (it compares option sets with `kiln-cli --help`, not wording). AC-3, AC-5 and AC-7 stay as guards of behaviour, with one limit: a guard asserts exit codes, whether stdout is empty, and for a refusal its `code` and `kind`; it never asserts the text of a message or of stderr. `acceptance_t039_cli.rs` therefore holds AC-1 only.
- Ruling z2 (director, D-095; found by the director reading the branch's sentences against the binary before queueing). Interface (a)'s second bullet says of `--memory-seed`: "nothing is written to disk". That is wider than the binary: `kiln-cli run s.jsonl --memory-seed m.json --log out.log` creates `out.log` on disk (run by the director on main's debug binary, 2026-11-13). The clause becomes: "read once at start; saves change the in-memory files only." (the wording `docs/protocol/cli-memory-seed.md` already uses). The implementer changes that clause in `docs/control-protocol.md` and nothing else.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13T18:29Z · R-154 · (new) · did: drafted from `evaluations/iter-1.md` NEW-4, Q1, Q3,
  D-090 and D-091 change 18 on main `d8c4a4199ca622f8a46dd09dcccb50b9f4bc4003`. Read the
  `kiln-cli` section, `cli-memory-seed.md`, the usage text in `main.rs`, `doc.rs` `Undo`/`Redo`,
  `history.rs`, `buffer.rs` undo/redo, `doc-positions.md`, `eol.md`. Probed with `kiln-cli run
  --memory-seed`: no-op redo keeps the typing group open, a successful redo ends it, a no-op
  undo changes nothing (redo kept, selection kept), BOM and UTF-16 byte offsets, and the
  option conflicts and bad-manifest exits, and that a successful undo ends the group (Pinned
  (g)). That a no-op undo cannot meet an open group is from reading `history.rs`. Tier low by ADR-0002 (documentation and tests;
  two DOC strings). Pinned lines n/a: (a), (c), (d), (e), (h), with reasons. · gate attempts: 0
  · not run: cargo (director's instruction) · missing context: none · next: director answers
  Q-1 and orders this task before T-037 and T-038.
  QUESTION: Q-1 (DOC strings or a new page) — see "Questions and rulings".
<!-- end planner log -->
### implementer
- 2026-11-13T21:35Z · R-159 · in_progress (primary checkout; worktree copy said ready, same
  rulings) · ruling z1: ACCEPTED, and Q-1's ruling (D-092) accepted: AC-2, AC-6 and AC-4's
  first half are no tests; AC-4's second half is the gate's `schema` step; guards assert exit
  codes, empty stdout and exact `run` responses, never message or stderr text (so AC-3 drops
  its "first stderr line"). Restated: AC-1 the `--` option sets of `run`, `exec`, `control`,
  `commands` in the synopsis equal those of `kiln-cli --help`; AC-3 the three option pairs,
  `control --memory`/`--memory-seed` and a bad manifest with `run` and `exec` exit 2 with empty
  stdout; AC-5 the five undo/redo sessions of Pinned (g) (plus the same session without the
  redo); AC-7 BOM and UTF-16 LE byte offsets. · did: wrote
  `acceptance_t039_cli.rs::protocol_synopsis_matches_help` (fails on base: `run` lacks
  `--memory-seed`) and `guard_t039_cli.rs::{memory_seed_conflicts_and_bad_manifest,
  noop_undo_and_redo_change_nothing, byte_offsets_ignore_bom_and_encoding}` (pass on base and
  after; with `pop_redo` temporarily made to end the group even on an empty stack,
  `noop_undo_and_redo_change_nothing` fails, then restored); copied Interface (a), (b), (c)
  into `docs/control-protocol.md`, `doc.rs` and `docs/protocol/doc-positions.md`;
  `cargo xtask schema --write` (only the two `doc` texts of `docs/commands.json` differ).
  Checked by script (whitespace-squashed) that the five sentences of (a), both DOC strings as
  printed by `kiln-cli commands` and the paragraph of (c) equal the spec's. Ran every Pinned (g)
  case through the debug `kiln-cli` on base `5615387be82037cb0d2b7ffe271401bec0c480af`: all
  responses and exits as the spec states; no Interface sentence found untrue of the binary. ·
  gate attempts: 2 (1 red `fmt` on my test files, fixed with `cargo fmt --all`; last green,
  11 steps, 694 tests passed) · not run: release build; a no-op redo after a non-typing edit
  (not a Pinned case); that `--memory-seed` reads the manifest "once" (taken from
  `cli-memory-seed.md`, not probed) · missing context: none · next: director compares the
  diff with Interface (a), (b), (c) and queues; tasks/ is outside `touches`, only this log
  entry changed there.
- 2026-11-13T21:37Z · R-161 · in_progress · ruling z2: ACCEPTED. My R-159 check missed that
  `--log` writes to disk under `--memory-seed`; the copied clause "nothing is written to disk"
  was wider than the binary. · did: replaced that clause in the `--memory-seed` bullet of
  `docs/control-protocol.md` with "read once at start; saves change the in-memory files only."
  and nothing else (two lines re-wrapped). Probed on the debug `kiln-cli` of this branch,
  manifest `a.txt`=`0123456789`, cwd with no `a.txt`: `run s.jsonl --memory-seed m.json` with
  open, insert `x`@0, `doc.save` (`bytes_written` 11), `doc.text` → exit 0 and no `a.txt` on disk;
  `doc.close` then `doc.open` in the same session reads `x0123456789` (the save changed the
  in-memory file); the same run with `--log out.log` creates `out.log` holding the 4 requests.
  Not changed (outside this ruling and `touches`): `kiln-cli --help` and
  `docs/protocol/cli-memory-seed.md` also say "nothing is written to disk". · gate attempts: 0
  (by instruction: `cargo fmt --all --check` ok, `cargo xtask hygiene` PASS,
  `cargo test -p kiln-cli --test acceptance_t039_cli --test guard_t039_cli` 4 passed) · not run:
  `cargo xtask ci` (the queue runs it) · missing context: none · next: director re-checks the
  bullet and queues.
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
