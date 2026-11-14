---
id: T-038
title: "Publish only the refusal kinds that a registered command can produce, tie each kind to the commands that report it with a mechanical check, and make every `when` text true of the binary"
tier: high              # ADR-0002: a CHARTER §3 contract change (decision 2: how the registry reports and describes refusals); needs an ADR (proposed ADR-0006)
state: draft
owner_role: planner
scorecard_items: [cmd.schema]   # existing item, strengthened (no new item)
depends_on: []          # T-032 merged (queue record 0ed8456e9c4eb87c7ff1ddf2527e3046cda2163e); the director accepts the ADR before `ready`
touches:
  - crates/kiln-commands/src/registry.rs        # implementer: refusal declarations, the execute check, describe(), audit()
  - crates/kiln-commands/src/reason.rs          # implementer: three `when` texts (not_found, not_a_file, out_of_range), its unit tests
  - crates/kiln-commands/src/lib.rs             # implementer: re-export of any new public type
  - crates/kiln-commands/src/buffer.rs          # implementer: declarations and refusal examples of buffer.*
  - crates/kiln-commands/src/cursor.rs          # implementer: same for cursor.*
  - crates/kiln-commands/src/doc.rs             # implementer: same for doc.*
  - crates/kiln-commands/src/eol.rs             # implementer: same for eol.*
  - crates/kiln-control/src/lib.rs              # implementer: only the test constant ERROR_KINDS, if Q-2 removes the `rejected`/`other` row
  - docs/control-protocol.md                    # implementer: the paragraph after the kinds table; the `rejected`/`other` row if Q-2
  - docs/protocol/doc-positions.md              # implementer: the parenthesis at :87–88 only
  - crates/kiln-commands/tests/acceptance_t032.rs      # implementer: merged pins this spec changes (see Pinned (f)); nothing else
  - apps/kiln-cli/tests/acceptance_t032_cli.rs         # implementer: same
  - crates/kiln-control/tests/acceptance_t032_control.rs  # implementer: same
  - apps/kiln-cli/tests/acceptance_t008_cli.rs         # implementer: the NOT_A_FILE_WHEN constant only
  - crates/kiln-commands/tests/reasons.rs              # implementer: the row count, only if Q-2
  - crates/kiln-commands/tests/acceptance_t038_commands.rs  # test author: AC-2, AC-3 (Rust API)
  - apps/kiln-cli/tests/acceptance_t038_cli.rs         # test author: AC-1, AC-4, AC-6, AC-7 through kiln-cli
  - apps/kiln-cli/tests/guard_t038_cli.rs              # test author: AC-5 (disk-only refusals through control)
  - scorecard/items/cmd.schema.toml                    # test author: append this task's test names
  # docs/commands.json changes: generated (`cargo xtask schema --write`); docs/scorecard.md: generated
budget_tokens: 1100000  # high (D-091 change 3)
iteration: 2
found_by: evaluator     # evaluations/iter-1.md NEW-2 and NEW-3 (run R-152)
origin_task: T-032
---

# T-038 Publish only the refusal kinds a command can produce, tied to the commands, and true `when` texts

## Goal
`kiln-cli commands --schema` and `docs/commands.json` publish `error_kinds`, which
`docs/control-protocol.md`:106 calls "the specified set; no other pair is produced". Today it
lists 23 `rejected` kinds, generated from every row of the `RejectReason` table
(`crates/kiln-commands/src/registry.rs`:604–607, `crates/kiln-commands/src/reason.rs`:61–122).
Twelve of them cannot occur in this build (VERIFIED, Pinned (g)): the eleven the evaluator
lists (`invalid_pattern`, `stale_preview`, `no_such_preview`, `tool_unavailable`,
`tool_not_found`, `tool_timeout`, `tool_failed`, `no_recording`, `format_error`,
`changed_on_disk`, `invalid_keymap`) and also `other` ("any other refusal"), which no code
raises. And three `when` texts do not match the binary: `not_a_file` claims "a file where a
directory is needed", which gives `not_found` (NEW-3); `not_found` omits that case; and
`out_of_range` says "an offset" although lines and columns use it too (noted in
`docs/protocol/doc-positions.md`:87–88).

After this task:
1. `error_kinds.rejected` lists exactly the kinds that some registered command can report,
   and each command's entry lists the kinds it can report (`refusals`). A client reading the
   generated description knows which kinds can occur and from which command (D-091 change 18:
   the published specified set contains only kinds a registered command can produce).
2. That statement is checked mechanically: a command that reports a kind it does not list is
   turned into an internal error by the registry, and `commands --audit` (the gate's `schema`
   step, `xtask/src/schema.rs`:68) refuses a listed kind that has no executed witness, except
   the pairs that only a disk session can produce, which are declared as such.
3. Every published `when` text is true of the binary.
4. `docs/control-protocol.md` says the same.

No gate path, no dependency. Tier high by ADR-0002: it changes how the registry works and what
CHARTER §3 decision 2's "command description generated from code" contains. The director
writes the ADR (proposed ADR-0006; ADR-0005 is D-091's re-scoping) amending ADR-0004's
Decision bullet "Reason variants may exist before any command raises them. The generated list
is the set of kinds the protocol may produce."

Reading list: `evaluations/iter-1.md` "Wired but inert", NEW-2, NEW-3; `docs/decisions.md`
D-069, D-090, D-091 (changes 16 and 18); `docs/adr/0004-structured-refusal-reasons.md`;
`crates/kiln-commands/src/reason.rs`; `crates/kiln-commands/src/registry.rs` (`Command`,
`Example`, `execute`, `describe`, `audit`); the `run` functions of `buffer.rs`, `cursor.rs`,
`doc.rs`, `eol.rs`; `docs/control-protocol.md` "Error codes and kinds";
`docs/protocol/doc-positions.md` "Refusals"; the merged tests named in Pinned (f).

## How a kind is tied to a command today (VERIFIED by reading)
A command refuses through `CommandError::rejected(reason, message)`. The reason is set in two
ways: explicitly (`InvalidPath` in doc.open, `doc.rs`:171; `UnsavedChanges` in doc.close,
:320–321; `NoSuchDocument` in `Context::doc`/`doc_mut`, `registry.rs`:236–237, used by every
command with a `doc` param), or through an exhaustive `From` of a lower-layer error
(`reason.rs`:140–192: `VfsError`, `OpenError`, `EditError`, `PositionError`, `ToolError`).
Rust gives no way to list, from the types, which reasons a function can return. So a tie must
be declared, and a declaration needs a check in each direction:
- **reported ⊆ declared**: the registry sees every refusal in `execute` (`registry.rs`:533–575),
  the place where it already turns a result that violates its schema into `internal` /
  `result_schema`. The same place can check the reason against the command's declaration.
- **declared ⇒ can occur**: only a run shows that. The registry already runs every example on
  a fresh in-memory context in `audit` (`registry.rs`:622–660), which the gate's `schema` step
  calls. Refusal examples run the same way would witness each declared kind. An in-memory
  context cannot produce `outside_root` or `io_error`, nor a save that fails because the file
  changed between requests (`MemoryFs`, `memory.rs`: no links, no root, no outside changes), so
  those pairs are declared as needing a disk session and are witnessed by tests instead.

## Options (the director's question Q-1)
D-091 change 18 fixed the result (the published set holds only producible kinds) and left the
means open. Every option needs the tie above to make the statement mechanical; they differ in
what happens to the twelve rows.

| | A: remove the rows until their task adds them | B: keep the rows, publish only declared kinds (recommended) | C: keep the rows with a hand-set `reserved` flag |
|---|---|---|---|
| Published `error_kinds.rejected` | the table, which then holds only producible kinds | the kinds some command declares, in table order | the rows without the flag |
| Parallel work | each kind-adding task edits `reason.rs` again: in stage 1 (D-091 change 16) T-021 gains it beside T-020; in stage 2 T-022, T-024, T-026, T-027 and T-031 all do, so none of them may be in flight together (METHODOLOGY §5.2) | a task declares its kinds on its own commands, in its own module; `reason.rs` is untouched unless the task needs a new row (T-020's `TooLarge`) | each feature task flips its flag in `reason.rs`: the same serialisation as A |
| Clients | the list is exact | the list is exact; each command also says which kinds it reports | exact only while the flags are right |
| Mechanical check | reported ⊆ table holds by the enum; table ⇒ can occur needs the same witnesses as B | both directions as in "How a kind is tied" | a flagged kind reported → can be refused at `execute`; an unflagged kind that cannot occur is not caught unless witnesses are added, which is B |
| What else changes | ADR-0004's "variants may exist before any command raises them" is reversed; `From<&ToolError>` must go until T-024 (it has no catch-all and maps to removed variants) | ADR-0004's last sentence is narrowed to the declared kinds | as B, plus a flag per row |
| Merged pins broken | the eleven kinds in the T-032 pin lists | the same, plus `describe() == ALL` (two tests) | as B |

**Recommendation: B.** It gives the client the exact list and more (the per-command
`refusals`), it is the only option whose check works in both directions without a shared file,
and it keeps the property ADR-0004 bought: kind-adding tasks do not meet in `reason.rs`. Its
cost over A is two more merged assertions to restate. The rest of this file specifies B.
Under A: Interface 1 publishes `RejectReason::ALL` as today; the twelve rows and the
`From<&ToolError>` impl are removed; Interfaces 2–4 stay (they are what makes "every row can
occur" mechanical); and the drafted tasks that raise those kinds gain `reason.rs` in `touches`.

## Interface
### 1. Generated description (`Registry::describe`, `kiln-cli commands --schema`, `docs/commands.json`)
- Each entry of `commands` gains `"refusals"`: the kinds that command can report, as a list of
  strings in `RejectReason::ALL` order, possibly empty. It also gains `"refusal_examples"`: a
  list of objects with exactly the keys `description`, `initial_text`, `files`, `setup`,
  `params` (as for `examples`) and `kind` (instead of `result`). The keys and order of the
  other members of a command entry are unchanged.
- `error_kinds.rejected` lists, in `RejectReason::ALL` order, exactly the kinds that occur in
  at least one command's `refusals`, each as `{"kind", "when"}` (keys unchanged).
  `error_kinds.internal` is unchanged (`panic`, `result_schema`, `other`: the registry and host
  can report them for any command, only through a bug). The top-level keys and the keys of
  `error_kinds` are unchanged.
- The kinds in the table that no command declares are not published anywhere in the
  description. They remain variants of `RejectReason` with their descriptions, so a later task
  that declares one publishes it without editing `reason.rs`.
- Pinned per command, after this task (from reading each `run` and probing, Pinned (g)):

| Command | `refusals` |
|---|---|
| `buffer.insert` | `out_of_range` |
| `buffer.text`, `doc.list` | (none) |
| `cursor.get`, `cursor.move`, `doc.info`, `doc.redo`, `doc.text`, `doc.undo`, `eol.counts`, `eol.normalize` | `no_such_document` |
| `cursor.set`, `doc.position` | `no_such_document`, `out_of_range`, `not_a_boundary` |
| `doc.close` | `no_such_document`, `unsaved_changes` |
| `doc.delete`, `doc.replace` | `no_such_document`, `out_of_range`, `inverted_range` |
| `doc.insert`, `doc.line` | `no_such_document`, `out_of_range` |
| `doc.open` | `invalid_path`, `not_found`, `not_a_file`, `io_error`, `decode_error`, `outside_root` |
| `doc.save` | `no_such_document`, `not_found`, `not_a_file`, `io_error`, `outside_root` |

  So `error_kinds.rejected` holds 11 kinds: `invalid_path`, `no_such_document`,
  `unsaved_changes`, `not_found`, `not_a_file`, `io_error`, `decode_error`, `out_of_range`,
  `inverted_range`, `not_a_boundary`, `outside_root`, in that order (`ALL` order).
  `cursor.move` lists only `no_such_document`: its `set_selection` refusal branch
  (`cursor.rs`:285–287) is unreachable by the contract of `Buffer::moved`
  (`crates/kiln-text/src/buffer.rs`:448–456, "never past either end", snaps out of a CRLF);
  INFERRED from that contract. If it were ever taken, Interface 2 reports it as a bug.
  T-037 does not change doc.open's list (its re-open uses `outside_root` and `io_error`).

### 2. The execute check (reported ⊆ declared)
When a command of a registry returns `CommandError::Rejected` whose reason is not in that
command's declared refusals, `Registry::execute` returns instead `CommandError::Internal {
reason: InternalReason::Other, message }` where the message names the command and the kind
(for example ``command `t.x` refused with `decode_error`, which it does not declare``; the
wording is not pinned beyond containing both names). The context is unchanged (the command's
own guarantee) and nothing is logged. Through the protocol this is `internal` / `other`, and
`kiln-cli exec` exits 1, as for every internal error. A declared refusal is returned unchanged,
so every response of today's 20 commands keeps its bytes.

### 3. Declarations and the audit (declared ⇒ can occur)
- Every command declares its refusals. Each declared kind is marked as witnessed either by a
  refusal example of that command (the default) or as needing a disk session. Today's
  disk-session pairs are exactly: `doc.open` with `io_error` and `outside_root`; `doc.save`
  with `not_found`, `not_a_file`, `io_error` and `outside_root`. How a command declares this
  (a trait item, a method with a default for commands outside the standard registry, a
  builder) is the implementer's choice; commands defined in merged test files must keep
  compiling and keep their current behaviour, so a command that declares nothing is not
  checked by Interface 2, and it is an audit problem (below).
- `Registry::audit` (and so `kiln-cli commands --audit` and the gate's `schema` step) adds one
  problem line, naming the command and, where there is one, the kind, for each of:
  (i) a command that declares no refusal list (an empty list is a declaration);
  (ii) a declared kind marked as witnessed by an example, for which the command has no refusal
  example of that kind;
  (iii) a refusal example whose params fail the params schema, whose setup fails, or whose run
  does not return `rejected` with exactly its `kind` (a success, another kind, another code);
  (iv) a refusal example whose `kind` the command does not declare.
  Refusal examples run like examples: a fresh context, scratch buffer `initial_text`,
  `MemoryFs` holding `files`, then `setup`.
- The standard registry's audit is empty after this task: today it needs 32 refusal examples
  (the pairs of the table above minus the six disk-session pairs).

### 4. `when` texts (`RejectReason::description`), after this task exactly
- `not_found`: "the path leads to nothing, or a component on its way that must be a directory
  is missing or is not a directory (a save creates a missing file, never a missing directory)"
- `not_a_file`: "the path leads to a directory, FIFO, socket or device where a regular file is
  needed"
- `out_of_range`: "an offset beyond the end of the text (in chars, bytes or UTF-16 code units),
  a line beyond the last line, or a column beyond the end of its line"
- Every other description, of the published and the unpublished kinds and of the internal
  kinds, is unchanged. Each published one was checked against the binary (Pinned (g)).

### 5. Documents
- `docs/control-protocol.md`, the paragraph at :105–106 becomes exactly (any line wrapping):
  "The `kind` of a `rejected` or `internal` error comes from a structured reason set where the
  error is raised; no message text is consulted. The request-level rows of this table (codes
  `invalid_json`, `invalid_request`, `unknown_command`, `invalid_params`) and the
  `rejected`/`internal` kinds in `error_kinds` of `docs/commands.json` (`kiln-cli commands
  --schema`, generated from code) are the specified set; no other pair is produced.
  `error_kinds` lists only the `rejected` kinds that some command of this build can report,
  and each command's entry lists those it can report as its `refusals`; the `internal` kinds
  report bugs and can come from any command. The `rejected` and `internal` rows above are the
  kinds of iteration 0; kinds added since are listed only in `error_kinds`. Where a row's
  'When' text differs from `error_kinds`, `error_kinds` governs; later tasks do not edit table
  cells."
- Under Q-2 as recommended, the row ``| `rejected` | `other` | any other refusal | echoed |``
  is removed from the table (22 rows remain: 10 request-level, 9 `rejected`, 3 `internal`).
- `docs/protocol/doc-positions.md`:87–88: the parenthesis "(its published description in
  `error_kinds` still reads "an offset beyond the end of the text")" is removed; nothing else
  in that file changes.

## Pinned
- (a) Errors, payloads, precedence: no new kind or reason row. The execute check (Interface 2)
  applies after the command ran and before the result-schema check of a successful result;
  it concerns only `Rejected`. `invalid_params`, `unknown_command` and the request-level
  kinds are untouched. The protocol error object is unchanged.
- (b) Files: `docs/commands.json` (generated) gains `refusals` and `refusal_examples` in every
  command entry and loses twelve `error_kinds.rejected` entries; three `when` texts change.
  No runtime file is read or written.
- (c) Order: execute: params schema → run → (refusal: declared? else internal) → (success:
  result schema) → log. Audit: the existing checks, then (i)–(iv) per command in name order.
- (d) Path base, platform: n/a for the registry; AC-5 uses Unix symlinks as T-008's tests do.
- (e) Threat model: n/a in the security sense (no boundary). The risk is a stale contract: the
  checks of Interfaces 2 and 3 are the mitigation; the residual is a disk-session pair that a
  later change makes impossible, which only review and that pair's tests can notice.
- (f) Tests pinning what this changes (all `grep`ed on main `d8c4a4199ca622f8a46dd09dcccb50b9f4bc4003`).
  The implementer replaces each pin below by the value this spec gives, and changes nothing
  else in those files (precedent D-069: a merged test that contradicts the contract is edited
  by ruling, never loosened):
  - `crates/kiln-commands/tests/acceptance_t032.rs`: `REJECTED` (:721–823: the three texts;
    the twelve unpublished kinds stay, since the test checks `RejectReason::ALL`, which keeps
    them); `describe_publishes_error_kinds_from_all` (:913–934: `error_kinds.rejected` is no
    longer `ALL` but the declared kinds).
  - `apps/kiln-cli/tests/acceptance_t032_cli.rs`: `REJECTED` (:67–128: it checks the published
    list, so the twelve leave it and the three texts change); the 23-row count (:223) if Q-2;
    `NEW_PARAGRAPH` (:247–255).
  - `crates/kiln-control/tests/acceptance_t032_control.rs`: the equality with `ALL`
    (:168–177) and the 23-row count (:189) if Q-2.
  - `apps/kiln-cli/tests/acceptance_t008_cli.rs`: `NOT_A_FILE_WHEN` (:31–32).
  - `crates/kiln-commands/src/reason.rs` unit test `outside_root_is_appended_and_not_a_file_is_widened`
    (:357–369).
  - If Q-2: `crates/kiln-commands/tests/reasons.rs` (13 rows, :235) and
    `crates/kiln-control/src/lib.rs` `ERROR_KINDS` (:68).
  - Unaffected (checked): `acceptance_t010.rs` (exact `not_a_boundary` entry, keys unchanged),
    `positions.rs`:417–421, `reasons.rs` `error_kinds` helper (2 keys per entry),
    `acceptance_t018.rs`:1834–1840, `acceptance_t004_r3.rs` (kinds ⊆ document tokens),
    `kiln-control` `table_kinds_are_reasons_and_in_the_description` (unless Q-2 keeps the
    `other` row: then it fails, because `rejected/other` is no longer published, which is why
    Q-2 recommends removing the row).
  - Every guard pinning protocol bytes (`guard_t032_cli.rs`, `guard_t015_cli.rs`, …) stays
    green unchanged: no refusal of today's commands changes.
- (g) Worked examples, executed by this planner with the debug `kiln-cli` of main
  `73a8d13c48cc09e7ac9f238d4848c888341d9fb0` (its `commands --schema` equals the committed
  `docs/commands.json`; no file this task concerns changed up to `d8c4a4199ca622f8a46dd09dcccb50b9f4bc4003`),
  one `kiln-cli run --root root` script in a throwaway directory plus a `control` session:
  - `doc.open`: `../x` → `invalid_path`; `missing.txt`, `f.txt/x` (f.txt a file), `nodir/x.txt`,
    a link `slashlink -> f.txt/` → `not_found` (`cannot open f.txt/x: not found`); a directory
    and a FIFO → `not_a_file`; a link to an outside file → `outside_root`; a two-link loop →
    `io_error … too many levels of symbolic links`; a 300-byte name → `io_error`; `FF` →
    `decode_error … invalid utf-8 at byte offset 0`; `FF FE 00` → `decode_error … invalid
    utf-16le at byte offset 2`.
  - `doc.insert` offset 99 → `out_of_range`; `doc.delete`/`doc.replace` 3..1 → `inverted_range`;
    `doc.delete` 1..99 → `out_of_range`; `doc.line` 4 of 4 lines → `out_of_range`;
    `doc.position` char 10 / byte 14 / utf16 11 / line 9 / column 4 of a 3-char line →
    `out_of_range`; byte 5, utf16 6, char 2 (inside CRLF) → `not_a_boundary`; `cursor.set`
    anchor 99 and head 99 → `out_of_range`, anchor 2 → `not_a_boundary`; `cursor.move` back at
    0 → ok; `cursor.move`/`eol.normalize` of doc 9 → `no_such_document`; `buffer.insert` 5 into
    an empty buffer → `out_of_range`; `doc.close` of a dirty document → `unsaved_changes`.
  - `control`: `p/y.txt` opened and edited, then `p` replaced by a regular file → `doc.save`
    → `not_found`; then `p/y.txt` made a directory → `not_a_file`. (`outside_root` at save:
    T-037's probes; `io_error` at save: evaluator, `evaluations/iter-1.md` ws.path-confinement.)
  - So the kinds produced are the eleven of Interface 1. `RejectReason::Other` and the eleven
    planned kinds have no raise site (`grep RejectReason::` over `crates/*/src` and
    `apps/*/src`; the only `ToolError` mapping is `reason.rs`:182–192, and no registered
    command calls a tool runner).
  - NOT executed: `doc.replace` out of range (INFERRED: the same `EditError` as `doc.delete`);
    a save through a link loop (INFERRED `io_error` from the walk, `disk.rs` `locate`).
- (h) Scorecard item `cmd.schema`: the evaluator runs `kiln-cli commands --schema` and checks
  that every command has `refusals` and `refusal_examples`, that `error_kinds.rejected` is the
  union of the `refusals`, and runs a few refusal examples through `kiln-cli run
  --memory-seed`; `kiln-cli commands --audit` exits 0.

## Acceptance criteria
Every child process has a hard timeout of 10 s; every `control` response is awaited at most
10 s. No test returns early or skips. Base = main `d8c4a4199ca622f8a46dd09dcccb50b9f4bc4003`.
**Acceptance** = fails on base; **guard** = passes on base.

- **AC-1 (acceptance; `acceptance_t038_cli.rs::schema_publishes_refusals_and_only_producible_kinds`).**
  `kiln-cli commands --schema`: every command entry's `refusals` equals the table of
  Interface 1; `error_kinds.rejected` kinds equal the eleven, in that order, and equal the
  union of all `refusals`; none of the twelve unpublished kinds occurs anywhere in the output
  as a `kind` value; `error_kinds.internal` is `panic`, `result_schema`, `other` with today's
  texts; the stdout bytes equal `docs/commands.json`. Oracle: exact values. Fails on base.
- **AC-2 (acceptance; `acceptance_t038_commands.rs::undeclared_refusal_is_internal`).** A
  registry with a test command declaring `[not_found]` that refuses with the kind its params
  name: `not_found` → `Rejected { reason: NotFound, .. }` with the command's message;
  `decode_error` → `Internal { reason: Other, .. }` whose message contains the command name and
  `decode_error`; in both cases the context's log is empty and `describe()` lists only
  `not_found` under `error_kinds.rejected`. Fails on base (no declaration, the refusal passes).
- **AC-3 (acceptance; `acceptance_t038_commands.rs::audit_requires_witnesses`).** Test
  registries, one problem each: a command with no declaration (i); a declared example-witnessed
  kind without a refusal example (ii); a refusal example that succeeds, one that refuses with
  another kind, one whose params fail the schema (iii); a refusal example of an undeclared kind
  (iv). Each `audit()` has exactly one line, which contains the command name (and the kind,
  where there is one). A disk-session pair without an example gives no problem.
  `standard_registry().audit()` is empty and `kiln-cli commands --audit` exits 0. Fails on base.
- **AC-4 (acceptance; `acceptance_t038_cli.rs::every_refusal_example_refuses_through_the_binary`).**
  For every refusal example in `commands --schema`: write a manifest from its `files`, a script
  of its `setup` then the request, and run `kiln-cli run s.jsonl --memory-seed m.json` (or,
  when `initial_text` is not empty, `kiln-cli exec <name> <params> --text <initial_text>`
  after a setup-free example). Every setup response is `ok:true` and the last is `rejected`
  with the example's `kind`. The set of (command, kind) pairs covered equals the table of
  Interface 1 minus the six disk-session pairs (32 pairs). Oracle: the binary, independently of
  `audit`. Fails on base.
- **AC-5 (guard; `guard_t038_cli.rs::disk_session_refusals`).** Through `kiln-cli control
  --root` with T-008 fixtures: `doc.open` of a link to an outside file → `outside_root`, of a
  two-link loop → `io_error`; `doc.save` after the parent directory was replaced by a regular
  file → `not_found`, after the file was replaced by a directory → `not_a_file`, after it was
  replaced by a link outside → `outside_root`, after it was replaced by a link loop →
  `io_error`. Passes on base; it witnesses the six disk-session pairs.
- **AC-6 (acceptance; `acceptance_t038_cli.rs::when_texts_match_the_binary`).** The three
  texts of Interface 4 exactly, in the schema output; and the cases they describe occur:
  `doc.open f.txt/x` → `not_found`; a directory → `not_a_file`; `doc.line` past the last line
  and `doc.position` with a column past its line → `out_of_range` (through `kiln-cli run
  --memory-seed`). Fails on base (texts).
- **AC-7 (documents; acceptance; `acceptance_t038_cli.rs::protocol_docs_state_the_published_set`).**
  The paragraph of Interface 5 exactly (whitespace-squashed, located as
  `acceptance_t032_cli.rs` locates it); under Q-2 the table has no `rejected`/`other` row;
  `docs/protocol/doc-positions.md` no longer contains "still reads". Fails on base.

Scorecard: the test author appends the AC-1, AC-3, AC-4 test names and AC-5 as a guard to
`scorecard/items/cmd.schema.toml` and regenerates `docs/scorecard.md`.

## Out of scope and implementer notes
- Adding kinds, raising the planned kinds, or any feature of T-020 to T-031.
- Message texts (documented as not stable; D-090 Q4).
- The `when` texts of unpublished kinds: they describe planned features and are checked when
  their task declares them.
- The internal kinds and the request-level rows.
- CHARTER §3 decision 2's sentence stays true ("listed in the command description generated
  from code"); if the director wants it to name the per-command tie, that is a low-tier task
  like T-035.
- Implementer's own tests: unit tests in `registry.rs` and the command modules. Determinism:
  refusal examples run on `MemoryFs`, like examples.
- Do not move refusal declarations into `reason.rs`: the point of B is that a task declares its
  kinds in its own module.

## Conflicts and dispatch
- METHODOLOGY §5.3: a §3 contract change merges before its consumers. In stage 1 (D-091
  change 16), T-019, T-020, T-021 and T-023 add commands that refuse. If they merge after this
  task, each declares its commands' refusals and adds refusal examples (their specs need one
  director note saying so; otherwise their refusals become `internal` and their acceptance
  tests fail at the join, loudly). If one merges before, this task declares that task's
  commands too and its `touches` grow by that module. Recommended: this task early in stage 1.
- `crates/kiln-commands/src/doc.rs`: shared with T-037 and T-039 (and T-027): never in flight
  together. Proposed order T-039, T-037, T-038.
- `docs/control-protocol.md`: shared with T-039 (CLI section), T-036 (CLI section, under D-091
  change 18), T-024 and T-030: never in flight together; this task changes only the paragraph
  after the kinds table (and the row under Q-2).
- `docs/protocol/doc-positions.md`: shared with T-039 (a different paragraph).
- `crates/kiln-commands/src/reason.rs`: shared with T-020 (a new row) and T-027 (a mapping).
- D-091 also says a high-tier spec that bundles independent items is split before dispatch.
  NEW-2 (Interfaces 1–3) and NEW-3 (Interface 4) touch the same rows, the same generated
  description and the same merged pins, so they are one change here; see Q-3.

## Questions and rulings
- Q-1 (planner, R-154): option A, B or C (Options). Recommended B; written for B, with the A
  delta stated.
- Q-2 (planner, R-154): `rejected`/`other` has no raise site, so under D-091 change 18 it is
  not published, yet the iteration-0 table lists it and four tests require every table row to
  be published. Recommended: remove that one row (a row, not a cell, so "later tasks do not
  edit table cells" is kept) and restate the row counts. Alternative: keep the row and restate
  those four tests as "every table row is a reason", leaving a table row outside the specified
  set.
- Q-3 (planner, R-154): split or not. If split, a medium task takes Interface 4, AC-6 and the
  doc-positions.md parenthesis (pins: the three texts in `acceptance_t032.rs`,
  `acceptance_t032_cli.rs`, `acceptance_t008_cli.rs`, `reason.rs`), and this task keeps the
  rest; the two then both edit `reason.rs` and the same pin lists, so they run one after the
  other. Recommended: keep together.
- Q-4 (planner, R-154): the ADR (proposed ADR-0006) amends ADR-0004's Decision: "The
  generated list is the set of kinds the protocol may produce" becomes "the generated list is
  the set of kinds some registered command declares; a declaration is checked at execute and
  witnessed in the audit".
- Ruling on Q-1 (director, D-092): option B. Each command declares the refusal kinds it can report; the generated description publishes only declared kinds and gives each command its list; a refusal of an undeclared kind is reported as `internal`; the audit witnesses every declared kind.
- Ruling on Q-2 (director, D-092): remove the `rejected`/`other` row from the protocol table and restate the row counts.
- Ruling on Q-3 (director, D-092): keep together. The `when` texts and the published set are one generated table and the same pinned tests; they are not independent items in the sense of D-091.
- Ruling on Q-4 (director, D-092): the ADR wording is accepted as the proposal to be checked; the ADR is numbered ADR-0006 (ADR-0005 re-scopes iteration 2) and is accepted or not at the pre-dispatch check.
- Order (director, D-092): dispatched after the stage-1 feature tasks (T-019, T-020, T-021, T-023) have merged, so that it declares the refusals of every command registered by then; those four tasks are not changed for it. Its spec is checked against main at that point (D-091 change 7).
- Note from T-021's spec check (director, D-101): the `when` text of `invalid_pattern` must also cover what T-021 makes the binary do — a pattern longer than the cap and a replacement template that cannot be expanded. T-021 leaves `reason.rs` alone, so this text joins the list this task makes true. Also from T-019 (D-095 Q-4): after T-019, `project.tree` produces `not_a_file` for a file where a directory is needed.
- Note from T-020's review round 2 (director, D-113): two follow-ups ride with this task, since it edits `crates/kiln-commands/src/syntax.rs`: the module comment there that says the size limit "bounds memory, not time" is brought in line with `docs/protocol/syntax.md` (the limit bounds the length of the text, not time or memory), and the 153-char line in `docs/protocol/syntax.md` (the sentence about a bracket the parser inserted) is re-wrapped without changing a word.
- Note from T-023's review round 2 (director, D-126): the reworded sentence N8 in `docs/protocol/nav.md` (about line 115) is 122 chars long; it is re-wrapped with this task without changing a word. T-023 adds four commands (`nav.outline`, `nav.goto-line`, `nav.goto-symbol`, `nav.workspace-symbols`) whose refusals this task declares with the others.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13T18:29Z · R-154 · (new) · did: drafted from `evaluations/iter-1.md` NEW-2, NEW-3,
  D-090 and D-091 change 18 on main `d8c4a4199ca622f8a46dd09dcccb50b9f4bc4003` (probes on the
  binary of `73a8d13c48cc09e7ac9f238d4848c888341d9fb0`; nothing concerned changed between).
  Read `reason.rs`, `registry.rs` (`execute`, `describe`, `audit`), every command's `run`,
  ADR-0004, D-069, and every test that reads `error_kinds` or the kinds table. Probed every
  published `when` text through `run` and `control` (Pinned (g)): eleven kinds occur; twelve,
  including `other`, which the evaluator did not list, have no raise site. Derived the
  per-command refusal table from the raise sites. Compared three options; recommended B
  (publish only declared kinds; per-command `refusals`; execute check; refusal examples in the
  audit; six disk-session pairs witnessed by a guard). Tier high by ADR-0002 (registry, CHARTER
  §3 decision 2), with an ADR. Pinned lines n/a: (d) partly, (e) in the security sense. Unsure:
  `cursor.move`'s refusal branch is unreachable only by `Buffer::moved`'s documented contract;
  two worked examples are INFERRED (listed in (g)). · gate attempts: 0 · not run: cargo
  (director's instruction); no new code exists · missing context: none · next: director
  answers Q-1 to Q-4 and writes the ADR.
  QUESTION: Q-1 (option), Q-2 (`rejected`/`other` row), Q-3 (split), Q-4 (ADR wording) — see
  "Questions and rulings".
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
