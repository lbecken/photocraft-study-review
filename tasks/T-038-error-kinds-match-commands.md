---
id: T-038
title: "Publish only the refusal kinds that a registered command can produce, tie each kind to the commands that report it with a mechanical check, and make every `when` text true of the binary"
tier: high              # ADR-0002: a CHARTER §3 contract change (decision 2: how the registry reports and describes refusals); ADR-0006 accepted (D-128)
state: merged
owner_role: planner
scorecard_items: [cmd.schema, control.errors]   # existing items, strengthened (no new item); intended tests: "Scorecard" below
depends_on: [T-019, T-020, T-021, T-023, T-032, T-036, T-037, T-039]   # all merged except T-037 (in rework; branch task/T-037 read at 2f1d9f86ade442d3345e01f9814c27bfa3dca074): see "To re-check at dispatch"
touches:
  # implementer
  - crates/kiln-commands/src/registry.rs        # declarations, the execute check, describe(), audit(); its unit-test commands get a declaration (Pinned (f) 7)
  - crates/kiln-commands/src/reason.rs          # five `when` texts (Interface 4), the module docs; its unit test (Pinned (f) 5)
  - crates/kiln-commands/src/lib.rs             # re-export of any new public type; the crate docs on adding a command / a kind
  - crates/kiln-commands/src/buffer.rs          # declarations and refusal examples of buffer.*
  - crates/kiln-commands/src/cursor.rs          # same, cursor.*
  - crates/kiln-commands/src/doc.rs             # same, doc.*
  - crates/kiln-commands/src/eol.rs             # same, eol.*
  - crates/kiln-commands/src/project.rs         # same, project.tree
  - crates/kiln-commands/src/syntax.rs          # same, syntax.*; the module comment of D-113 (Interface 6)
  - crates/kiln-commands/src/search.rs          # same, search.*
  - crates/kiln-commands/src/nav.rs             # same, nav.*
  - crates/kiln-control/src/lib.rs              # the test constant ERROR_KINDS: its ("rejected", "other") row only (Q-2)
  - docs/control-protocol.md                    # the paragraph after the kinds table; the `rejected`/`other` row (Q-2)
  - docs/protocol/doc-positions.md              # the parenthesis at :94–95 only
  - docs/protocol/syntax.md                     # re-wrap of the paragraph holding :104 only (D-113)
  - docs/protocol/nav.md                        # re-wrap of the paragraph holding :115 only (D-126)
  - crates/kiln-commands/tests/acceptance_t032.rs          # merged pins of Pinned (f) only
  - apps/kiln-cli/tests/acceptance_t032_cli.rs             # same
  - crates/kiln-control/tests/acceptance_t032_control.rs   # same
  - apps/kiln-cli/tests/acceptance_t008_cli.rs             # same (NOT_A_FILE_WHEN and the outside_root `when` only)
  - crates/kiln-commands/tests/reasons.rs                  # same
  - docs/commands.json                                     # generated (`cargo xtask schema --write`), never edited by hand
  # test author (independent; task/T-038-tests)
  - apps/kiln-cli/tests/acceptance_t038_cli.rs             # AC-1, AC-4, AC-8
  - crates/kiln-commands/tests/acceptance_t038_commands.rs # AC-2, AC-3
  - apps/kiln-cli/tests/guard_t038_cli.rs                  # AC-5, AC-7
  - crates/kiln-commands/tests/guard_t038_commands.rs      # AC-6
  # NOT touched: scorecard/ (the director ties tests at the join), docs/scorecard.md (generated at the join),
  # xtask/ (no gate path), docs/CHARTER.md, docs/protocol/paths.md, docs/protocol/project.md, docs/protocol/search.md,
  # crates/kiln-workspace (no VFS change), every merged test file not listed above
budget_tokens: 1100000  # high (METHODOLOGY §11, v1.2)
iteration: 2
found_by: evaluator     # evaluations/iter-1.md NEW-2 and NEW-3 (run R-152)
origin_task: T-032
---

# T-038 Publish only the refusal kinds a command can produce, tied to the commands, and true `when` texts

## Goal
`kiln-cli commands --schema` and `docs/commands.json` publish `error_kinds`, which
`docs/control-protocol.md`:106 calls "the specified set; no other pair is produced". On main
(`e0f4daa`, T-023 merged at `1da69fd`) it lists all 24 rows of the `RejectReason` table
(`crates/kiln-commands/src/registry.rs`:604–607 publishes `RejectReason::ALL`,
`crates/kiln-commands/src/reason.rs`:61–124). Eleven of them have no command that can report
them (VERIFIED, Pinned (g)): `other`, `stale_preview`, `no_such_preview`, `tool_unavailable`,
`tool_not_found`, `tool_timeout`, `tool_failed`, `no_recording`, `format_error`,
`changed_on_disk`, `invalid_keymap`. (Since the draft, T-021 and T-023 made `invalid_pattern`
reachable and T-020 added `too_large`; `search.rs`:350 raises `RejectReason::Other` on a branch
that cannot be taken, see Q-10.) And five published `when` texts are not true of the binary
(Interface 4): `not_found`, `not_a_file`, `out_of_range`, `invalid_pattern`, `outside_root`.

After this task:
1. `error_kinds.rejected` lists exactly the kinds that some registered command declares, and each
   command entry lists the kinds it declares (`refusals`) with one executable refusal example per
   kind that the in-memory file system can produce (D-091 change 18; D-092 option B).
2. Both directions are checked mechanically: a command that reports a kind it does not declare
   gets `internal`/`other` from the registry, and `kiln-cli commands --audit` (the gate's `schema`
   step, `xtask/src/schema.rs`:67, unchanged) refuses a declared kind without a witness.
3. Every published `when` text is true of the binary, read with the convention of Interface 5.
4. `docs/control-protocol.md` says the same; the `rejected`/`other` row is gone (D-092 Q-2).
5. The three follow-ups of D-113 and D-126 are done (Interface 6).

No gate path (`xtask/` is not touched), no dependency, no VFS change. Tier high by ADR-0002: it
changes how the registry executes and describes commands (CHARTER §3 decision 2). ADR-0006
(accepted, D-128; text below, filed by the director as `docs/adr/0006-declared-refusal-kinds.md`) amends ADR-0004's Decision bullet "Reason variants may
exist before any command raises them. The generated list is the set of kinds the protocol may
produce."

Reading list: `evaluations/iter-1.md` "Wired but inert", NEW-2, NEW-3; `docs/decisions.md`
D-069, D-090, D-091 (changes 16, 18), D-092, D-095 (Q-4), D-101, D-113, D-126;
`docs/adr/0004-structured-refusal-reasons.md`; `crates/kiln-commands/src/reason.rs`,
`registry.rs` (`Command`, `Example`, `execute`, `describe`, `audit`); the `run` functions of
every command module; `docs/control-protocol.md` "Error codes and kinds";
`docs/protocol/paths.md` "The root" and "The rule"; `docs/protocol/project.md` (refusals);
`docs/protocol/search.md` "Refusals"; `docs/protocol/nav.md` "Errors";
`docs/protocol/cli-memory-seed.md` "Manifest"; the merged tests of Pinned (f).

## How a kind reaches a response today (VERIFIED by reading main `e0f4daa` and `task/T-037`)
A command refuses through `CommandError::rejected(reason, message)`. The reason is set
explicitly (`InvalidPath` in `doc.open` and `project.tree`, `UnsavedChanges` in `doc.close`,
`TooLarge`/`OutOfRange`/`InvertedRange` in `syntax.rs`:105–127 and :425–430, `OutOfRange` in
`search.rs`:308–314 and `nav.rs` goto-symbol, `InvalidPattern` in `search.rs`:62–78 and
`nav.rs` workspace-symbols, `NoSuchDocument` in `registry.rs`:235–240 for every command with a
`doc` param) or through an exhaustive `From` of a lower-layer error (`reason.rs`:142–194:
`VfsError`, `OpenError`, `EditError`, `PositionError`, `ToolError`). Rust cannot list from the
types which reasons a function returns, so the tie is declared and checked both ways:
- **reported ⊆ declared**: `Registry::execute` (`registry.rs`:533–570) sees every refusal; it
  already turns a result that violates its schema into `internal`/`result_schema` there.
- **declared ⇒ can occur**: only a run shows it. `audit` (`registry.rs`:622–665) already runs
  every example on a fresh in-memory context; refusal examples run the same way witness the
  kinds `MemoryFs` can produce. The others need `DiskFs` (`outside_root`; `io_error`; a file or
  directory changed between requests; a race inside one request) and are witnessed by tests.

### The kinds each command can report (VERIFIED: code read, every pair marked E or D probed except where noted)
E = the in-memory file system produces it (witnessed by a refusal example, audit);
D = only a session on disk produces it (witnessed by tests: C = through `kiln-cli control
--root`, deterministic, works as root; R = only when another process changes the tree during
the request, witnessed through a test VFS, AC-6).

| Command | E (refusal examples) | D (disk only) |
|---|---|---|
| `buffer.insert` | `out_of_range` | — |
| `buffer.text`, `doc.list` | — | — |
| `cursor.get`, `cursor.move`, `doc.info`, `doc.redo`, `doc.text`, `doc.undo`, `eol.counts`, `eol.normalize`, `nav.goto-line` | `no_such_document` | — |
| `cursor.set`, `doc.position` | `no_such_document`, `out_of_range`, `not_a_boundary` | — |
| `doc.close` | `no_such_document`, `unsaved_changes` | — |
| `doc.delete`, `doc.replace` | `no_such_document`, `out_of_range`, `inverted_range` | — |
| `doc.insert`, `doc.line` | `no_such_document`, `out_of_range` | — |
| `doc.open` | `invalid_path`, `not_found`, `not_a_file`, `decode_error` | `io_error` (C), `outside_root` (C) |
| `doc.save` | `no_such_document` | `not_found` (C), `not_a_file` (C), `io_error` (C), `outside_root` (C) |
| `project.tree` | `invalid_path`, `not_found`, `not_a_file` | `io_error` (C), `outside_root` (C) |
| `syntax.tree`, `nav.outline` | `no_such_document`, `too_large` | — |
| `syntax.highlight` | `no_such_document`, `too_large`, `out_of_range`, `inverted_range` | — |
| `syntax.match-bracket`, `nav.goto-symbol` | `no_such_document`, `too_large`, `out_of_range` | — |
| `search.find` | `no_such_document`, `invalid_pattern` | — |
| `search.replace` | `no_such_document`, `invalid_pattern`, `out_of_range` | — |
| `nav.workspace-symbols` | `invalid_pattern` | `io_error` (C), `outside_root` (C), `not_found` (R), `not_a_file` (R) |

30 commands; 56 E pairs; 12 D pairs (10 C, 2 R). `error_kinds.rejected` then holds 13 kinds, in
`RejectReason::ALL` order: `invalid_path`, `no_such_document`, `unsaved_changes`, `not_found`,
`not_a_file`, `io_error`, `decode_error`, `out_of_range`, `inverted_range`, `invalid_pattern`,
`not_a_boundary`, `outside_root`, `too_large`. Notes:
- `cursor.move` lists only `no_such_document`: its `set_selection` refusal (`cursor.rs`:285–287)
  is unreachable by the contract of `Buffer::moved` (never past either end, snaps out of a CRLF);
  INFERRED from that contract. `nav.goto-line` clamps (never `out_of_range`; probe).
- `doc.save`'s E column is only `no_such_document`: `MemoryFs` cannot change between requests
  through any command. (A manifest holding both `a` and `a/b` makes `MemoryFs` refuse a save with
  `not_found`/`not_a_file`, `memory.rs`:77–82; that is a state no disk can have, so it is not
  used as a witness.)
- The four VFS kinds of `nav.workspace-symbols` are those of its walk (`nav.rs` on main:
  `walk::tree` → `CommandError::rejected((&e).into(), …)`), which `docs/protocol/nav.md`
  "Errors" already publishes; read failures of `.rs` files are `skipped`, not refusals. With
  `path` omitted, the walk can give `not_found`/`not_a_file` only by a race (a `.gitignore` or
  directory removed or replaced between listing and reading); `project.tree` gives them
  deterministically for its `path`. See Q-5.
- `doc.open`'s `io_error`/`outside_root` include T-037's re-open (`task/T-037` doc.rs: the
  re-open branch maps `Vfs::check_location` errors). T-037 adds no kind: a re-open after the
  file was removed or replaced by a directory still answers `already_open: true` (probe).
- `search.replace` does not declare `other`: its one `RejectReason::Other` site (`search.rs`:350)
  is an `apply_edits` failure on matches the search produced, unreachable by `CompiledQuery`'s
  contract (sorted, disjoint, in bounds); INFERRED. See Q-10.
- Pairs that need an unprivileged user (permission denied on a file, directory or `.gitignore`)
  exist for `doc.open`, `doc.save`, `project.tree`, `nav.workspace-symbols` (`io_error`); each of
  these commands also has a C witness that works as root (link loop, deep path), so no test needs
  an unprivileged user.

## Interface
### 1. Generated description (`Registry::describe`, `kiln-cli commands --schema`, `docs/commands.json`)
- Each entry of `commands` gains `"refusals"`: the kinds the command declares, as strings, in
  `RejectReason::ALL` order, possibly empty; and `"refusal_examples"`: a list (possibly empty) of
  objects with exactly the keys `description` (string), `initial_text` (string), `files`,
  `setup`, `params` and `kind` (a string of `refusals`). `initial_text`, `setup` and `params` are
  as in `examples`. `files` (ruling Q-6, D-128) is a list of objects in the form of a
  `--memory-seed` manifest entry (`docs/protocol/cli-memory-seed.md` "Manifest"): `{"path",
  "text"}` or `{"path", "hex"}`, so a refusal example can hold bytes that are not UTF-8. The keys
  and order of the other members of a command entry, and of `examples`, are unchanged.
- `error_kinds.rejected` lists, in `RejectReason::ALL` order, exactly the kinds that occur in at
  least one command's `refusals`, each as `{"kind", "when"}`. `error_kinds.internal` is unchanged
  (`panic`, `result_schema`, `other`). The top-level keys and the keys of `error_kinds` are
  unchanged.
- A kind of the table that no command declares is not published anywhere in the description. It
  stays a variant of `RejectReason` with its description, so a later task that declares it
  publishes it without editing `reason.rs`.
- After this task the per-command `refusals` are the union of the two columns of the table in
  "The kinds each command can report", and the refusal examples cover exactly its E column (one
  or more per E pair; none for a D pair, Interface 3 (v)).

### 2. The execute check (reported ⊆ declared)
When a command that has a declaration returns `CommandError::Rejected` whose reason it does not
declare, `Registry::execute` returns instead `CommandError::Internal { reason:
InternalReason::Other, message }`, the message naming the command and the kind (wording free).
The context is as the command left it (the command's own guarantee: unchanged on error) and
nothing is logged. Through the protocol this is `internal`/`other`; `kiln-cli exec` exits 1 and
`run` 3, as for every internal error. A declared refusal is returned unchanged, so no response of
the 30 commands changes (Pinned (g)). A command without a declaration (only commands defined in
test code) is not checked; Interface 3 (i) makes it an audit problem.

### 3. Declarations and the audit (declared ⇒ can occur)
- Every command of the standard registry declares its refusals; each declared kind is marked
  either *example* (witnessed by a refusal example of that command) or *disk* (cannot occur on
  `MemoryFs`; witnessed by tests). The disk pairs are exactly the D column above. How a command
  declares this (a trait item with a default meaning "no declaration", a method, a builder) is
  the implementer's choice; `Command` implementations in merged test files
  (`acceptance_t032.rs`, `acceptance_t032_json.rs`) must keep compiling unchanged and keep their
  behaviour.
- `Registry::audit` (so `kiln-cli commands --audit` and the gate's `schema` step) adds one
  problem line, naming the command and, where there is one, the kind, for each of:
  (i) a command with no declaration (an empty list is a declaration);
  (ii) a kind marked *example* for which the command has no refusal example of that kind;
  (iii) a refusal example whose params fail the params schema, whose `files` entry has an
  invalid path or a `hex` that is not an even number of hex digits, whose setup fails, or whose run does not return `rejected` with exactly its `kind` (a success,
  another kind, another code);
  (iv) a refusal example whose `kind` the command does not declare;
  (v) a refusal example whose `kind` is marked *disk*.
  Refusal examples run like examples: a fresh context, scratch buffer `initial_text`, `MemoryFs`
  holding `files`, then `setup` (each must succeed), then the command.
- A `too_large` refusal example (ruling Q-7, D-128) reaches 1,048,577 chars through its
  `setup`, not through a literal: for example `files` `[{"path": "g.rs", "text": "a"×32}]`, setup `doc.open`,
  three times `search.replace {"doc": 1, "query": "a", "replacement": "a"×32, "all": true,
  "limit": 1}` (32 → 1,024 → 32,768 → 1,048,576 chars), `doc.insert {"doc": 1, "offset": 0,
  "text": "b"}`, then the command. (This planner measured 0.6–0.7 s per such run with a debug
  `kiln-cli run`, Pinned (g); no time is promised. The implementer reports, as measured, the time
  these examples add to the gate's `schema` step: "Implementer's report" below.) The audit
  message lines and the success line of `commands --audit` (`audit ok: N commands, all examples
  validate`) are otherwise unchanged.
- The standard registry's audit is empty after this task.

### 4. `when` texts (`RejectReason::description`), after this task exactly
Five change; each replacement is worded to the probes of Pinned (g).
- `not_found`: "the path leads to nothing, or a component on its way that must be a directory is
  missing or is not a directory (a save creates a missing file, never a missing directory)"
- `not_a_file`: "the path leads to a directory, FIFO, socket or device where a regular file is
  needed, or to a file, FIFO, socket or device where a directory is needed"
- `out_of_range`: "an offset beyond the end of the text (in chars, bytes or UTF-16 code units), a
  line beyond the last line, a column beyond the end of its line, or an outline item index
  beyond the last item"
- `invalid_pattern`: "the search pattern is empty, longer than 1,000 chars, or in regex mode not
  accepted by the regex engine or with more than 100 capture groups; or a regex replacement
  refers to a capture group the pattern does not have"
- `outside_root`: "the path resolves outside the root, or the root path no longer leads to the
  directory the session started on"

Every other description (of the published, the unpublished and the internal kinds) is
unchanged; each other published one was checked against the binary (Pinned (g)): `invalid_path`,
`no_such_document`, `unsaved_changes`, `io_error`, `decode_error`, `inverted_range`,
`not_a_boundary`, `too_large`.

### 5. Documents (`docs/control-protocol.md`, `docs/protocol/doc-positions.md`)
- `docs/control-protocol.md`, the paragraph at :105–106 becomes exactly (any line wrapping):
  "The `kind` of a `rejected` or `internal` error comes from a structured reason set where the
  error is raised; no message text is consulted. The request-level rows of this table (codes
  `invalid_json`, `invalid_request`, `unknown_command`, `invalid_params`) and the
  `rejected`/`internal` kinds in `error_kinds` of `docs/commands.json` (`kiln-cli commands
  --schema`, generated from code) are the specified set; no other pair is produced.
  `error_kinds` lists only the `rejected` kinds that some command of this build can report, and
  each command's entry lists the kinds it can report as its `refusals`; a listed kind for which
  the command has no entry in `refusal_examples` can occur only in a session on disk (`--root`).
  A command that refuses with a kind it does not list reports `internal` / `other` instead. A
  'When' text says what a kind means; which commands report it, and for which params, is given
  by their `refusals` and descriptions. The `internal` kinds report bugs and can come from any
  command. The `rejected` and `internal` rows above are the kinds of iteration 0; kinds added
  since are listed only in `error_kinds`. Where a row's 'When' text differs from `error_kinds`,
  `error_kinds` governs; later tasks do not edit table cells."
  Evidence per sentence: Pinned (i).
- The row ``| `rejected` | `other` | any other refusal | echoed |`` is removed (D-092 Q-2): 22
  rows remain (10 request-level, 9 `rejected`, 3 `internal`). No other cell changes.
- `docs/protocol/doc-positions.md`:94–95: the parenthesis "(its published description in
  `error_kinds` still reads "an offset beyond the end of the text")" is removed, so the sentence
  reads "An offset, a line or a column beyond the end is refused with `out_of_range`." Nothing
  else in that file changes.

### 6. Follow-ups (D-113, D-126), exactly
- `crates/kiln-commands/src/syntax.rs`:11, the sentence "The limit bounds memory, not time."
  becomes "The limit bounds the length of the text a command accepts, not the time a parse takes
  or the memory it needs (`docs/protocol/syntax.md`)." (the wording of `docs/protocol/syntax.md`
  :134–135). No other line of the module comment changes.
- `docs/protocol/syntax.md`: the paragraph that holds line 104 (153 chars, "… `<` and `>` are not
  brackets. Brackets inside strings, character literals and comments are never matched; a") is
  re-wrapped so that no line of that paragraph exceeds 100 chars; its words and their order are
  unchanged (whitespace-squashed text equal before and after).
- `docs/protocol/nav.md`: the same for the paragraph that holds line 115 (122 chars, "walk of
  `project.tree` with `limit` 1,000,000 would list; it does not follow symbolic links, and files
  that `project.tree`").
- Only these two paragraphs are re-wrapped; other long lines in those files (syntax.md :43,
  :109; nav.md :17–19, :68) are out of scope.

## Pinned
- (a) Errors, payloads, precedence: no new kind or reason row; no message changes. The execute
  check applies after the command ran, to `Rejected` only, before the result-schema check of a
  successful result. `invalid_params`, `unknown_command` and the request-level kinds are
  untouched. The protocol error object is unchanged. Each command's own check order (documented
  in `docs/protocol/*.md`) is unchanged.
- (b) Files: `docs/commands.json` (generated) gains `refusals` and `refusal_examples` in every
  command entry, loses eleven `error_kinds.rejected` entries, and five `when` texts change. No
  runtime file is read or written. `docs/control-protocol.md`, `doc-positions.md`, `syntax.md`,
  `nav.md` as in Interfaces 5–6.
- (c) Order: execute: params schema → run → (refusal: declared? else `internal`/`other`) →
  (success: result schema) → log. Audit: the existing checks, then (i)–(v) per command in name
  order; within a command, (i), then (ii) by kind in `ALL` order, then (iii)–(v) per refusal
  example in declaration order.
- (d) Path base, platform: n/a for the registry; AC-5 and AC-7 use Unix symlinks and FIFOs as
  T-008's tests do; no test needs an unprivileged user.
- (e) Threat model: n/a in the security sense (no boundary). The risk is a stale contract: the
  checks of Interfaces 2 and 3 are the mitigation. Residual: a *disk* pair that a later change
  makes impossible, or an over-declared *disk* pair, is noticed only by review and that pair's
  tests (AC-5, AC-6).
- (f) Merged tests pinning what this changes (`grep`ed on main `e0f4daa`; none of these files is
  changed by `task/T-037`). The implementer replaces each pin below by the value this spec gives
  and changes nothing else in those files; each edit is logged under golden rule 4 with this
  item's number (precedent D-069: a merged test that contradicts the contract is restated by
  ruling, never loosened).
  1. `crates/kiln-commands/tests/acceptance_t032.rs`: `REJECTED` (:721–823): the texts of
     `not_found` (:740), `not_a_file` (:745), `out_of_range` (:760), `invalid_pattern` (:771)
     become those of Interface 4 (the eleven unpublished kinds stay: the test checks
     `RejectReason::ALL`); `describe_publishes_error_kinds_from_all` (:912–935): `rejected` is
     built from the declared kinds (the kinds of `ALL` that occur in some command's
     `refusals`), not from `ALL`.
  2. `apps/kiln-cli/tests/acceptance_t032_cli.rs`: `REJECTED` (:67–128): the entries `other`,
     `stale_preview`, `no_such_preview`, `tool_unavailable`, `tool_not_found`, `tool_timeout`,
     `tool_failed`, `no_recording`, `format_error`, `changed_on_disk`, `invalid_keymap` are
     removed and the four texts of item 1 change; `rows.len(), 23` (:223) becomes 22;
     `NEW_PARAGRAPH` (:247–255) becomes the paragraph of Interface 5.
  3. `crates/kiln-control/tests/acceptance_t032_control.rs`: `want_rejected` (:168–171) is built
     from the declared kinds, as in item 1; `rows.len(), 23` (:189) becomes 22; `checked, 13`
     (:203) becomes 12 (message "9 rejected and 3 internal rows").
  4. `apps/kiln-cli/tests/acceptance_t008_cli.rs`: `NOT_A_FILE_WHEN` (:31–32) and the
     `outside_root` text (:538–541) become those of Interface 4.
  5. `crates/kiln-commands/src/reason.rs` unit test
     `outside_root_is_appended_and_not_a_file_is_widened` (:361–373): the two descriptions become
     those of Interface 4.
  6. `crates/kiln-commands/tests/reasons.rs`: `describe_publishes_every_reason_in_all_order`
     (:175–187): `want` for `rejected` is built from the declared kinds, as in item 1 (the
     `internal` half and the `docs/commands.json` comparison stay); the table test (:205–238):
     13 rows becomes 12 (and its message "9 rejected and 3 internal rows").
  7. `crates/kiln-commands/src/registry.rs` unit tests: the test commands `Echo`, `Liar`,
     `Undocumented`, `NeedsSetup` (:684–781) each gain an empty declaration (no refusals), so
     `audit_reports_failing_setup_and_bad_example_paths` (:784–795, 2 problems) and
     `audit_catches_wrong_example_and_empty_registry` (:871–878, 1 problem) keep their
     assertions unchanged. No assertion of these tests changes.
  8. `crates/kiln-control/src/lib.rs` `ERROR_KINDS` (:48–72): the row `("rejected", "other")`
     (:68) is removed (it feeds `table_kinds_are_reasons_and_in_the_description`, :870–880,
     which would otherwise fail; `every_kind_is_documented` and `code_kind` keep passing).
  - Unaffected (checked): `acceptance_t010.rs`:1073–1100, `acceptance_t010_api.rs`:439–459,
    `positions.rs`:417–422 (the exact `not_a_boundary` entry, still published, keys unchanged);
    `acceptance_t018.rs`:1834–1840 (three kinds still published); `acceptance_t020_cli.rs`
    :1734–1741 and `syntax.rs`:719–725 (`too_large` still published); `acceptance_t020_cli.rs`
    AC-9 and `acceptance_t023_cli.rs::nav_commands_published_schema` (mutating flags, defaults,
    non-empty `examples` only); `acceptance_t021_cli.rs` (no read of `error_kinds`);
    `acceptance_t004_r3.rs`:516 (kinds ⊆ backticked tokens of the document; `other` stays as
    `internal`/`other`); `acceptance_t003_registry.rs`:105 and :170, `cli.rs`:33–41 (keys of
    `examples` entries, schemas; audit of the standard registry empty, which this task keeps);
    `nav.rs` unit tests `walk_errors_fail_the_command` (`io_error`, declared *disk*) and
    `skip_reasons_are_existing_kinds` (`RejectReason::ALL`, unchanged); `reason.rs`
    `PINNED_REJECTED` (names only); every guard pinning protocol bytes (`guard_t032_cli.rs`,
    `guard_t015_cli.rs`, `guard_t036_cli.rs`, …): no response of a registered command changes.
- (g) Worked examples, executed by this planner (R-209) with a debug `kiln-cli` built from a
  private merge of main `1a3dce3` with `task/T-023` at `1da69fd` (since merged unchanged) and
  `task/T-037` at `083cf0d` (product code equal to its head `2f1d9f8`; later commits change
  tests and the task file only). `commands` lists 30 commands; `commands --audit` → `audit ok:
  30 commands`; `commands --schema` equals that merge's `docs/commands.json` and lists the 24
  kinds of `ALL`. Memory probes: one `kiln-cli run s.jsonl --memory-seed m.json` per case;
  disk probes: `kiln-cli control --root` sessions, the tree changed between requests; run as
  root (uid 0). Result `code/kind` of the last request:
  - `doc.open`: `../x` → `rejected/invalid_path`; `missing.txt`, `f.txt/x` (f.txt a file),
    `nodir/x.txt` → `not_found`; `d` (a directory of the manifest) → `not_a_file`; hex `ff` →
    `decode_error` (`invalid utf-8 at byte offset 0`); hex `fffe00` → `decode_error` (`invalid
    utf-16le at byte offset 2`). On disk: a link to an outside file → `outside_root`; a two-link
    loop → `io_error` (`too many levels of symbolic links`); a 300-byte name → `io_error`; a FIFO
    and a directory → `not_a_file`; a link `slash -> a.txt/` → `not_found`. Re-open (T-037) of
    an open, edited `a.txt` after it was replaced by a link outside → `outside_root`, by a link
    loop → `io_error`; after it was removed, or replaced by a directory → `ok`,
    `already_open: true`; after the root was renamed and a new directory put in its place →
    `outside_root` (and a first open of another path too).
  - `doc.save` of `p/y.txt` (opened, edited): `p` replaced by a regular file → `not_found`;
    `p/y.txt` replaced by a directory → `not_a_file`, by a link outside → `outside_root`, by a
    link loop → `io_error`; removed → `ok` (the save creates it). `doc.save {doc: 9}` →
    `no_such_document`.
  - `project.tree` (memory): `{"path": "../x"}` → `invalid_path`; `missing` → `not_found`;
    `f.txt` (a file) → `not_a_file` (`cannot list f.txt: not a directory`); `f.txt/x` →
    `not_found`; `{"limit": 0}` → `invalid_params`. On disk: `path` a link to an outside
    directory → `outside_root`; a link loop → `io_error`; a FIFO → `not_a_file`; a 300-byte name
    → `io_error`; no `path` after the root was replaced → `outside_root`; no `path` with a chain
    of 18 nested 250-byte directory names (host path beyond 4,096 bytes) → `io_error`; a FIFO
    named `.gitignore` → `ok` (listed as `other`, not read).
  - `nav.workspace-symbols`: `query: ""` → `invalid_pattern`; `limit: 0` → `invalid_params`;
    after the root was replaced → `outside_root`; with the 18-level chain → `io_error`. The two R
    pairs were not produced (they need a race); INFERRED from `crates/kiln-workspace/src/project.rs`:70–99, :121–141, :214–219
    (`vfs.read` of a listed `.gitignore`, `list_dir` of a listed directory) and the `nav.rs`
    unit test `walk_errors_fail_the_command`, which shows a failing `.gitignore` read refusing
    the command with its kind.
  - Editing commands on `f.txt` = `abc\r\ndef\n` (9 chars, 3 lines): `doc.insert` offset 99,
    `doc.delete`/`doc.replace` 1..99 → `out_of_range`; 3..1 → `inverted_range`; `doc.line 3` →
    `out_of_range`; `doc.position` line 9, column 4 of the 3-char line 0, char 99 →
    `out_of_range`; char 4 (inside the CRLF) → `not_a_boundary`; `cursor.set` anchor 99 →
    `out_of_range`, head 4 → `not_a_boundary`; `cursor.move` back at 0 → `ok`; `cursor.get`,
    `eol.counts`, `syntax.tree`, `search.find`, `nav.outline`, `nav.goto-line`,
    `nav.goto-symbol` of doc 9 → `no_such_document`; `buffer.insert` offset 5 into the empty
    buffer → `out_of_range`; `doc.close` of a dirty document → `unsaved_changes`.
  - `syntax.*` on `a.rs` (25 chars): `highlight` start 99, end 99, `match-bracket` offset 99 →
    `out_of_range`; `highlight` 5..2 → `inverted_range`. Grown to 1,048,577 chars by the setup of
    Interface 3 (0.6–0.7 s per run): `syntax.tree`, `syntax.highlight`, `syntax.match-bracket`,
    `nav.outline`, `nav.goto-symbol` → `too_large`; at exactly 1,048,576 → `syntax.tree` `ok`.
  - `search.*`: `query: ""` → `invalid_pattern` (`empty search pattern`); 1,001 `a` →
    `invalid_pattern`; 1,000 `a` → `ok`; regex `(` → `invalid_pattern`; regex `(a)`×101 →
    `invalid_pattern` (`more than 100 capture groups`); regex `a{99999999}` → `invalid_pattern`
    (the engine's size limit); `limit: 0` → `invalid_params`; `search.replace` regex `(a)(b)`
    with `$3` → `invalid_pattern`, regex `(q)` (no match) with `$x` → `invalid_pattern`, literal
    `a` with `$3` → `ok`; `from: 99` → `out_of_range`; `from: 99, all: true` → `ok`.
  - `nav.goto-symbol` index 5 of a 1-item outline → `out_of_range`; `nav.goto-line` line 99
    col 99 → `ok` (clamped).
  - So the 13 kinds of Interface 1 occur, and no other `rejected` kind does: the eleven
    unpublished ones have no reachable raise site (`grep -n "RejectReason::"` over
    `crates/*/src` and `apps/*/src`: `Other` only at `search.rs`:350, Q-10; the only
    `ToolError` mapping is `reason.rs`:184–194 and no command calls a tool runner).
  - NOT executed: a socket or device (INFERRED from `disk.rs` `Found::Special`, as for the FIFO);
    `cursor.move` refusing (INFERRED unreachable); the R pairs (above); permission-denied pairs
    (root bypasses them).
- (h) Evaluator's calls (scorecard `cmd.schema`, `control.errors`): `kiln-cli commands --schema`:
  every command has `refusals` and `refusal_examples`; `error_kinds.rejected` is the union of
  the `refusals`; each refusal example run as written (manifest from `files`, script from
  `setup` and the request, `kiln-cli run s.jsonl --memory-seed m.json`) gives `rejected` with its
  `kind`; `kiln-cli commands --audit` exits 0. For `control.errors`: a few D pairs through
  `kiln-cli control --root` as in AC-5.
- (i) Sentences this task adds to `docs/` that state a guarantee or a limit (Interface 5; the
  `when` texts of Interface 4 are generated sentences with the evidence of (g)):
  - "`error_kinds` lists only the `rejected` kinds that some command of this build can report,
    and each command's entry lists the kinds it can report as its `refusals`" — evidence: AC-1
    (union equality), AC-3 (the audit refuses a declared kind without a witness), AC-4, AC-5,
    AC-6; (g).
  - "a listed kind for which the command has no entry in `refusal_examples` can occur only in a
    session on disk (`--root`)" — evidence: audit (ii) and (v) (AC-3): every *example* kind has
    a refusal example and no *disk* kind has one; the D column of the table, each pair probed
    on `MemoryFs` impossible (`memory.rs`: no links, no root, no outside changes) and on disk
    possible (g), AC-5, AC-6.
  - "A command that refuses with a kind it does not list reports `internal` / `other` instead" —
    evidence: AC-2.
  - "A 'When' text says what a kind means; which commands report it, and for which params, is
    given by their `refusals` and descriptions." — a reading rule, needed because
    `nav.workspace-symbols` declares `invalid_pattern` but has no 1,000-char cap and
    `nav.goto-line` clamps instead of `out_of_range` (g).

## Acceptance criteria
High tier: an independent test author writes every `acceptance_t038*`/`guard_t038*` file on
`task/T-038-tests`; the implementer adds none and edits no `scorecard/` file (METHODOLOGY §5.6,
§6). Every child process has a hard timeout of 10 s and every `control` response is awaited at
most 10 s (test hygiene, not a property of the product; no AC asserts a duration). No test
returns early or skips. Base = main after T-037 has merged (see "To re-check at dispatch"). **Acceptance** = fails on base; **guard** = passes on base and joined.

What a test may assert (golden rule 9; this task is about published texts): the **set** of
published kinds per command, `error_kinds.rejected` as the union of `refusals` in `ALL` order,
the kinds of refusal examples and that each runs to its `kind`, the audit's problem count per
registry and that a problem line names its command, response `code`/`kind` pairs, and the
structure of documents (a row present or absent). A test never asserts the wording of a `when`
text, of a message, of an audit line beyond the command name, or of a document sentence; the
reviewer checks wording on the diff and against the binary (METHODOLOGY §7). The merged pins
that already hold wording are restated by the implementer (Pinned (f)); no new test adds one.

- **AC-1 (acceptance; `apps/kiln-cli/tests/acceptance_t038_cli.rs::schema_publishes_each_commands_refusals`).**
  `kiln-cli commands --schema`: every command entry has `refusals`, a list of distinct strings
  each of which is a kind of `error_kinds.rejected`, in that list's order; for each command of
  the table in "The kinds each command can report" except `doc.save`, the set of `refusals`
  equals its row (E ∪ D); for `doc.save`, it contains its row (T-027 adds `changed_on_disk`);
  the kinds of `error_kinds.rejected` equal the union of all commands' `refusals`, in
  `RejectReason` order as published today (each published kind keeps its relative position);
  each `error_kinds` entry has exactly the keys `kind`, `when`; stdout equals
  `docs/commands.json`. Oracle: the table (exact sets). Fails on base (no `refusals`). The test
  does not list the commands of the registry or assert that a kind is absent.
- **AC-2 (acceptance; `crates/kiln-commands/tests/acceptance_t038_commands.rs::undeclared_refusal_is_internal`).**
  A registry with a test command declaring `[not_found]` whose run refuses with the kind its
  params name: `not_found` → `Rejected { reason: NotFound, .. }`; `decode_error` →
  `Internal { reason: Other, .. }`; in both cases the context's log is empty; its `describe()`
  lists `not_found` and no other kind under `error_kinds.rejected`. A second command with no
  declaration that refuses with `decode_error` → `Rejected { reason: DecodeError, .. }`
  (unchecked). Fails on base (no declaration API; the refusal passes).
- **AC-3 (acceptance; `acceptance_t038_commands.rs::audit_requires_a_witness_for_every_declared_kind`).**
  Test registries, one problem each, each `audit()` returning exactly one line that contains the
  command name: (i) a command with no declaration; (ii) a declared *example* kind without a
  refusal example; (iii) a refusal example that succeeds, one that refuses with another kind,
  one whose params fail the schema, one whose setup fails; (iv) a refusal example of an
  undeclared kind; (v) a refusal example of a kind marked *disk*. A *disk* kind without an
  example gives no problem. `standard_registry().audit()` is empty and `kiln-cli commands
  --audit` exits 0. Fails on base.
- **AC-4 (acceptance; `acceptance_t038_cli.rs::every_refusal_example_refuses_through_the_binary`).**
  For every refusal example in `commands --schema`: write a manifest from its `files` (copied as
  the manifest's `files`, ruling Q-6), a script of its `setup` then the request, and run
  `kiln-cli run s.jsonl --memory-seed m.json`; when `initial_text` is not empty (the example has
  no setup), run `kiln-cli exec <name> <params> --text <initial_text> --memory-seed m.json`
  instead. Every setup response is `ok:true`; the last is `rejected` with the example's `kind`.
  For each command of the table, the set of kinds of its refusal examples equals its E column.
  Oracle: the binary, independently of `audit`. Fails on base (no `refusal_examples`).
- **AC-5 (guard; `apps/kiln-cli/tests/guard_t038_cli.rs::disk_refusals_through_control`).**
  Through `kiln-cli control --root`, with fixtures as in T-008's and T-037's tests, the ten C
  pairs: `doc.open` of a link to an outside file → `outside_root`, of a two-link loop →
  `io_error`, a re-open after the open file was replaced by a link outside → `outside_root`;
  `doc.save` after the parent directory was replaced by a regular file → `not_found`, after the
  file was replaced by a directory → `not_a_file`, by a link outside → `outside_root`, by a link
  loop → `io_error`; `project.tree` with `path` a link to an outside directory →
  `outside_root`, a link loop → `io_error`; `nav.workspace-symbols` after the root was replaced
  by a new directory → `outside_root`, with a chain of nested directories whose host path
  exceeds 4,096 bytes → `io_error`. Each answer is `rejected` with that kind (never `internal`).
  Passes on base; it witnesses the C pairs and fails if the execute check rejects one.
- **AC-6 (guard; `crates/kiln-commands/tests/guard_t038_commands.rs::race_refusals_keep_their_kind`).**
  Through `standard_registry().execute` on a `Context::with_vfs` whose test VFS wraps a
  `MemoryFs` and fails `read` of `.gitignore` with `VfsError::NotFound` (as if removed after the
  listing) and, in a second context, `list_dir` of a listed directory with `VfsError::NotAFile`
  (as if replaced by a file): `nav.workspace-symbols {query: "a"}` → `rejected`/`not_found` and
  `rejected`/`not_a_file`; `project.tree {}` → the same. (The test VFS implements every `Vfs`
  method, including T-037's `check_location`.) Passes on base; witnesses the two R pairs.
- **AC-7 (guard; `guard_t038_cli.rs::situations_of_the_changed_when_texts`).** The situations
  the five new texts name give their kinds: `doc.open f.txt/x` and `nodir/x.txt` → `not_found`
  (memory); `project.tree {"path": "f.txt"}` (a file) → `not_a_file` (memory) and a FIFO
  → `not_a_file` (disk); `doc.position` with a column beyond its line, `doc.line` past the last
  line, `nav.goto-symbol` with an index beyond the outline → `out_of_range`; `search.find` with
  a 1,001-char query and with a regex of 101 groups, `search.replace` with `$3` for a 2-group
  regex → `invalid_pattern`; `project.tree {}` after the root was replaced → `outside_root`.
  Passes on base. The wording of the texts is the reviewer's check, and the restated merged
  pins of Pinned (f) 1, 2, 4, 5 hold it (ruling Q-9, D-128: this AC has no fail-on-base test,
  by design).
- **AC-8 (documents; acceptance; `acceptance_t038_cli.rs::protocol_docs_state_the_published_set`).**
  The table of "Error codes and kinds" in `docs/control-protocol.md` has no row with code
  `rejected` and kind `other`, and every `rejected` row's kind is published in `error_kinds`;
  `docs/protocol/doc-positions.md` no longer contains "still reads". Fails on base. (The
  paragraph's wording: the reviewer, and the restated `NEW_PARAGRAPH` pin, Pinned (f) 2.)
- **AC-9 (follow-ups; review only).** Interface 6: the reviewer checks on the diff that the
  `syntax.rs` sentence is the one given, that the two paragraphs' whitespace-squashed text is
  unchanged and that none of their lines exceeds 100 chars. No test: a test of "this task left
  the words alone" is a diff check (golden rule 9).

### Scorecard (the director ties these at the join; no builder edits `scorecard/`)
| Item | Intended tests |
|---|---|
| `cmd.schema` | `kiln_cli::acceptance_t038_cli::schema_publishes_each_commands_refusals`, `kiln_cli::acceptance_t038_cli::every_refusal_example_refuses_through_the_binary`, `kiln_commands::acceptance_t038_commands::audit_requires_a_witness_for_every_declared_kind` |
| `control.errors` | `kiln_commands::acceptance_t038_commands::undeclared_refusal_is_internal`, `kiln_cli::guard_t038_cli::disk_refusals_through_control`, `kiln_cli::acceptance_t038_cli::protocol_docs_state_the_published_set` |

## Out of scope and implementer notes
- Adding kinds, raising the eleven unpublished kinds, or any feature of T-022, T-024 to T-031.
- Message texts (documented as not stable; D-090 Q4). `when` texts of unpublished kinds: they
  describe planned features and are checked when their task declares them.
- The internal kinds, the request-level rows, the exit codes.
- `xtask/src/schema.rs` `structural_problems`: unchanged (the audit through `kiln-cli` does the
  check; editing it would make this a gate-path task).
- CHARTER §3 decision 2's sentence stays true ("listed in the command description generated
  from code"); if the director wants it to name the per-command tie, that is a low-tier task
  like T-035.
- Do not move declarations into `reason.rs`: under option B a task declares its kinds in its own
  module. `reason.rs` changes only for the five texts and its module docs.
- Update the docs that describe the old rule: `reason.rs` module docs (":6–8 publishes every
  reason"), `registry.rs` `describe` doc (:585–588), `lib.rs` crate docs (adding a command: also
  declare its refusals and give a refusal example per *example* kind; adding a kind: published
  once a command declares it).
- Implementer's own tests: unit tests in `registry.rs` and the command modules. Determinism:
  refusal examples run on `MemoryFs`, like examples.

### Implementer's report (in the log entry, not criteria)
- The time the refusal examples add to the gate's `schema` step (`cargo xtask schema`, which runs
  `kiln-cli commands --audit`): the step's elapsed time on the base and on the branch, same
  machine, two runs each, with the command; the `too_large` examples' share if measurable
  (ruling Q-7, D-128). A measured figure, not a bound; no document states it.
- Each merged pin edited under Pinned (f), by item number (golden rule 4).
- Any `docs/` sentence stating a guarantee or a limit that this spec lacks, with the command and
  output that show it (METHODOLOGY §6).

## Conflicts and dispatch
- Every command module, `registry.rs` and `reason.rs` are in `touches`: no other task that edits
  `crates/kiln-commands/src/` may be in flight with this one (METHODOLOGY §5.2). Today that is
  T-037 (`doc.rs`): T-038 is dispatched after T-037 has merged (D-092 order).
- `docs/control-protocol.md`: shared with T-024, T-030 and others drafted; this task changes only
  the paragraph after the kinds table and removes one row.
- Drafted tasks that add commands or kinds after this one (T-022, T-024 to T-027, T-031) must
  declare their commands' refusals and add refusal examples, or their refusals become
  `internal` and the audit fails at their join. The director adds one note to each of those
  specs when this merges. T-027 also restates the `doc.save` row of AC-1 only if it removes a
  kind (AC-1 checks `doc.save` as a superset).
- D-091 (bundling): see Q-3 and "Spec check R-209" finding 16.

## ADR-0006 (accepted, D-128; the director files it as `docs/adr/0006-declared-refusal-kinds.md`, this text)
```
---
id: ADR-0006
title: "Each command declares the refusal kinds it can report; only declared kinds are published"
status: accepted
date: 2026-11-14
task: T-038
charter_section: "§3 decision 2"
approval: D-128
supersedes: null
superseded_by: null
---

# ADR-0006 Declared refusal kinds

## Context
- ADR-0004 made every refusal carry a `RejectReason` and generated `error_kinds` from the
  reason table: "Reason variants may exist before any command raises them. The generated list
  is the set of kinds the protocol may produce."
- `docs/control-protocol.md` calls the generated list "the specified set; no other pair is
  produced". The iteration-1 evaluation (NEW-2) found 11 published kinds no command could
  produce; on main at T-038's spec check (R-209) 11 of 24 still have no reachable raise site.
  NEW-3 found a `when` text wider than the binary; five texts are now wider or narrower.
- D-091 change 18 decided that the published specified set holds only kinds a registered
  command can produce, and left the means to T-038; D-092 chose option B.
- Rust cannot list the reasons a function returns, so the tie must be declared and checked.

## Options
| Option | For | Against |
|---|---|---|
| A: remove table rows until a task raises them | the list is exact; no new mechanism | every kind-adding task edits `reason.rs` again (the serialisation ADR-0004 removed); `From<&ToolError>` must go |
| B: commands declare their kinds; publish only declared kinds; check at execute; witness in the audit | exact list plus a per-command list; both directions mechanical; tasks declare in their own module | a declaration and refusal examples per command; disk-only pairs witnessed by tests, not the audit |
| C: a hand-set `reserved` flag per row | small | flags serialise tasks on `reason.rs`; nothing checks an unflagged row can occur |
| Keep the current contract | no work | the published specified set is false (NEW-2) |

## Decision
Option B.
- Every command of the standard registry declares the `RejectReason`s it can report, each
  marked *example* (the in-memory file system can produce it) or *disk* (only a session on
  disk can).
- `Registry::describe()` publishes, per command, `refusals` (its declared kinds) and
  `refusal_examples`; `error_kinds.rejected` is the set of kinds some command declares, in
  table order. A reason variant may exist before any command declares it; it is then not
  published.
- `Registry::execute` reports a refusal of an undeclared kind as `internal`/`other`.
- `Registry::audit` (the gate's `schema` step through `kiln-cli commands --audit`) refuses a
  command without a declaration, an *example* kind without a refusal example that produces it,
  a refusal example that does not produce its kind, and a refusal example of an undeclared or
  *disk* kind.
- ADR-0004's bullet "Reason variants may exist before any command raises them. The generated
  list is the set of kinds the protocol may produce." is replaced by: "Reason variants may
  exist before any command declares them. The generated list is the set of kinds some
  registered command declares; a declaration is checked at execute and witnessed in the
  audit."

## Consequences
- T-038 carries the change. Every later task that adds a command or makes a command report a
  new kind declares it in its own module and adds a refusal example (T-022, T-024 to T-027,
  T-031); `reason.rs` changes only when a new row is needed.
- `docs/control-protocol.md` states the rule (T-038 Interface 5); its `rejected`/`other` row
  is removed (D-092 Q-2).
- CHARTER §3 decision 2 text: unchanged ("listed in the command description generated from
  code" stays true).
- Checks: the `schema` gate step (audit and currency of `docs/commands.json`); T-038's
  acceptance tests; review for *disk* markings (rule only: no in-memory run can witness them).

## Verification
T-038 AC-1 to AC-6. Retrospective 2 counts: kind-adding tasks that edited `reason.rs` (expected:
only those adding a row), `internal`/`other` responses seen by the evaluator (expected 0), audit
failures at joins caused by missing declarations.

## Rollback
A new ADR; remove the declarations and the execute check, publish `RejectReason::ALL` again.
One task; no data or file format depends on declarations.
```

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
- Q-5 (planner, R-209): which kinds does a command that reaches the VFS declare? (a) every kind
  its code can map a VFS error to on the shipped file systems, including the two that
  `nav.workspace-symbols` can give only when another process changes the tree during the request
  (`not_found`, `not_a_file`, R pairs), marked *disk* and witnessed by AC-6; (b) only kinds a
  deterministic session can produce, so such a race is reported as `internal`/`other` (a bug)
  although nothing is wrong in Kiln, and `docs/protocol/nav.md` "Errors" (which lists the four
  walk kinds) becomes wider than the published list. Consequences: (a) adds 2 pairs and one guard
  test; (b) needs a sentence in `nav.md` (outside `touches`) and publishes a bug kind for an
  outside change. **Recommended (a)**; ruled (a), D-128.
- Q-6 (planner, R-209): how does a refusal example hold bytes that are not UTF-8? `Example.files`
  is `Vec<(&'static str, &'static str)>` (UTF-8 text), so no example can give `decode_error`
  (both invalid UTF-8 and a UTF-16 BOM `FF FE` are not valid UTF-8). (a) Refusal examples
  publish `files` as `--memory-seed` manifest entries (`{"path","text"}` / `{"path","hex"}`),
  directly usable by AC-4 and the evaluator; `examples` are unchanged. (b) Keep the tuple form
  for refusal examples and mark `doc.open`/`decode_error` *disk*-like ("witnessed by tests"),
  weakening the mechanical witness of a pair `MemoryFs` produces deterministically. (c) A new
  byte form only for refusal examples' tuples. **Recommended (a)**; ruled (a),
  D-128.
- Q-7 (planner, R-209): how is `too_large` (a document of more than 1,048,576 chars) witnessed
  by a refusal example? (a) The example's `setup` grows a small file with `search.replace`
  (Interface 3; 0.6–0.7 s per example in debug, five examples); (b) mark `too_large` as
  witnessed by tests (it is not disk-only, so this needs a third marking); (c) a literal of
  1 MiB in `files` (each such example adds over 1 MiB to `docs/commands.json`, five times).
  Consequence of (a): the five examples depend on `search.replace`; a change there fails the
  audit loudly. **Recommended (a)**; ruled (a), D-128.
- Q-8 (planner, R-209): audit rule (v), a refusal example of a *disk* kind is a problem. With it,
  "a listed kind without a refusal example can occur only on disk" (Interface 5) is mechanical
  and a client can tell the disk-only kinds from the published shape; without it the marking
  is internal and that sentence must go. **Recommended: keep (v) and the sentence**; ruled so,
  D-128.
- Q-9 (planner, R-209): AC-7 (the changed `when` texts) has no test that fails on base: golden
  rule 9 forbids a test of the wording (the director's instruction for this task), and the
  situations already give their kinds on base. METHODOLOGY §6 asks for a failing test per AC.
  (a) Accept AC-7 as a guard; the wording is checked by the reviewer against the binary, and the
  merged pins restated under Pinned (f) carry the new texts; (b) an acceptance test that each of
  the five texts differs from its base text (fails on base, pins no new wording, but says
  nothing about truth). **Recommended (a)**; ruled (a), D-128.
- Q-10 (planner, R-209): `search.rs`:350 raises `RejectReason::Other` for an `apply_edits` failure
  that cannot happen. (a) Leave the code; `search.replace` does not declare `other`, so if it
  ever happened the execute check reports `internal`/`other` (correct: it would be a bug); (b)
  change the site to `CommandError::Internal { reason: InternalReason::Other, .. }` directly
  (same response, clearer code, one more edit in `search.rs`). **Recommended (a)** (no response
  changes either way); ruled (a), D-128.
### Rulings on the spec check R-209 (director, D-128)
- ADR-0006: accepted as proposed. The director files it as `docs/adr/0006-declared-refusal-kinds.md` with `status: accepted` and `approval: D-128` before dispatch; it amends ADR-0004's bullet as its Decision says.
- Q-5: yes. Each command that reads the file system declares all four file-system kinds, the two that only a race can produce included, and a guard test witnesses them where a test can.
- Q-6: yes. Refusal examples publish their `files` in the `--memory-seed` manifest form (text or hex), so that `decode_error` can be shown.
- Q-7: yes. A `too_large` example grows its document in its setup; no megabyte literal enters `docs/commands.json`. The time this adds to the gate's `schema` step is reported by the implementer as measured, not promised.
- Q-8: keep audit rule (v) and its sentence: a refusal example of a disk-only kind is a problem, so the absence of an example tells a client that the kind needs a session on disk.
- Q-9: accepted. The criterion on the changed `when` texts is checked by the reviewer on the diff and against the binary; no test asserts their wording (golden rule 9).
- Q-10: leave the unreachable raise site in `search.rs` as it is; it stays undeclared.
- Size: the task is kept whole (the grounds of D-092 Q-3 stand), with 30 commands to declare. The director watches the budget at the round-1 verdict as METHODOLOGY §11 says.
- On three criteria the planner flagged in R-211 (director, D-130; golden rule 9, as D-094 and D-114): AC-8's check that a phrase no longer appears in `docs/control-protocol.md` is not a test; the reviewer checks that removal on the diff. AC-1's comparison of `commands --schema` with `docs/commands.json` is already the gate's `schema` step and is not written as a test of this task. AC-3's check that each audit line names its command stays (it identifies the line; it asserts no wording).
- Dispatch check (director, D-135): on main `53c1355`, after T-037 merged at `f81092a7be14313834d9601cac000092a7c1bd7e`: `docs/commands.json` lists 30 commands and 24 `rejected` kinds; `FailingFs` in `crates/kiln-commands/src/nav.rs` implements `check_location` (line 678); ADR-0006 is filed as `docs/adr/0006-declared-refusal-kinds.md`; T-023 and T-037 are merged. The line numbers the spec cites were read before T-037's merge and are found by their text.
- Ruling j1 (director, D-136; the test author's question of R-216; answered to both builders). Interface 3 left the form of the declaration to the implementer, so the independent tests of AC-2's declared half and of AC-3's rules (ii) to (v) could not be written. The public items are now pinned as the implementer built them at `e78ec66` (signatures only; the director read them from the branch, the test author sees nothing else): in `kiln_commands`, `pub enum Witness { Example, Disk }`; `pub struct Refusal { pub reason: RejectReason, pub witness: Witness }` with `pub const fn example(reason: RejectReason) -> Self` and `pub const fn disk(reason: RejectReason) -> Self`; `pub enum SeedFile { Text { path: &'static str, text: &'static str }, Hex { path: &'static str, hex: &'static str } }` with `pub const fn text(path, text) -> Self` and `pub const fn hex(path, hex) -> Self`; `pub struct RefusalExample { pub description: &'static str, pub initial_text: &'static str, pub files: Vec<SeedFile>, pub setup: Vec<(&'static str, serde_json::Value)>, pub params: serde_json::Value, pub kind: RejectReason }`; and on the `Command` trait two provided items, `fn refusals() -> Option<Vec<Refusal>>` (default `None`: no declaration; `Some(vec![])` declares that the command never refuses) and `fn refusal_examples() -> Vec<RefusalExample>` (default empty). The test author adds the missing halves of AC-2 and AC-3 against these. The implementer keeps these signatures; if it must change one, it says so before the join.

- Rulings on review round 1 (director, D-137; `reviews/T-038-r1.md` at `4bef1e1062a87afbf4dcedcd15d03c32e7a793f0`; cap for the rework: 1,100,000 tokens for the task).
  - **j2 (finding 1, test gap; test author).** AC-3 gains one case, inside the existing test `audit_requires_a_witness_for_every_declared_kind` (no new test name): a registry holding one command that has **no declaration** and **one refusal example whose run returns `rejected` with exactly the example's kind**. `audit()` returns exactly two lines, and each contains the command's name (Interface 3: one line for rule (i), one for rule (iv)). The reviewer observed this on the joined head; with rule (iv) removed there is one line.
  - **j3 (finding 2, spec gap; director).** ADR-0004's last Decision bullet is replaced on main by the text ADR-0006 gives, with the old text kept in a parenthesis. ADR-0004 is not superseded as a whole, so `superseded_by` stays null.
  - **j4 (finding 4; accepted).** Registering a command whose declaration names the same kind twice is refused. It has no effect on a client and is unit-tested by the implementer; it stays.
  - **j5 (findings 3 and 5; follow-up, not this task).** The comment "13 rows" and the test name `describe_publishes_every_reason_in_all_order` in `crates/kiln-commands/tests/reasons.rs`, and the dated timing table in `docs/development.md` (its "warm" row is older than the refusal examples; the reviewer measured 3.4 to 4.0 s for `cargo xtask schema` alone), go to one low-tier task filed with the evaluation's findings.

## To re-check at dispatch
Read at the spec check: main `e0f4daa` (T-023 merged at `1da69fd3433474d8de74bae430a4d7d3d79e290f`,
the head the check read); `task/T-037` at `2f1d9f86ade442d3345e01f9814c27bfa3dca074` (product code
probed at `083cf0d`, identical in `crates/` and `apps/*/src`). At R-211 main is `ee58cd8` (ledger
commits only since `e0f4daa`); `task/T-037` has had main merged in (head `0946426`, moving) and is
applying ruling k6. What must hold on main once T-037 has merged (no head is named; read main):
1. T-023 is on main: `kiln-cli commands` lists the 30 commands of "The kinds each command can
   report", including the four `nav.*`; `docs/commands.json` is current.
2. `crates/kiln-commands/src/doc.rs`: the re-open branch maps `Vfs::check_location` errors through
   `OpenError::Vfs` to `outside_root`/`io_error` only, and a re-open of a removed file or of a
   directory still answers `already_open: true` (Pinned (g)); `doc.open`'s row is unchanged.
3. `Vfs::check_location` is on main (a required trait method, as on `task/T-037` `vfs.rs`:115),
   and `crates/kiln-commands/src/nav.rs`'s test VFS `FailingFs` implements it (T-037 ruling k6,
   D-128), so `cargo test -p kiln-commands` compiles on main. AC-6's test VFS implements every
   method the trait then requires, `check_location` included.
4. No other kind became reachable or unreachable: `grep -n "RejectReason::" crates/*/src
   apps/*/src` and the `From` impls in `reason.rs` match "The kinds each command can report"
   (T-037's k6 edit of `nav.rs` is test code and adds none).
5. The line numbers of Pinned (f) and Interfaces 5–6 (T-037 changes none of those files; it now
   edits `nav.rs` test code only, which is not a Pinned (f) item; any task merged in between may
   shift them).
6. ADR-0006 is filed by the director at `docs/adr/0006-declared-refusal-kinds.md` with `status:
   accepted`, `approval: D-128` (done or not, it is not a builder path).

## Spec check R-209
Checked the draft (R-154, written on `73a8d13`/`d8c4a41` with 20 commands) against main
`e0f4daa` and `task/T-037` at `2f1d9f8`, with probes of a private debug build (Pinned (g)). The
sections above are rewritten to today's code; the list records what the draft got wrong.
Blockers (would stop or mislead a builder):
1. Command set: 20 → 30 commands; the draft's table lacked `project.tree`, `syntax.*` (3),
   `search.*` (2), `nav.*` (4), and Interface 2 promised "today's 20 commands" keep their bytes.
2. Counts: "11 published, 12 unused rows" → 13 published (`invalid_pattern` is now raised by
   `search.*` and `nav.workspace-symbols`; `too_large` added and raised), 11 unpublished of 24.
   "32 refusal examples, six disk-session pairs" → 56 E pairs, 12 D pairs.
3. D pairs incomplete: `project.tree` (`outside_root`, `io_error`) and `nav.workspace-symbols`
   (`outside_root`, `io_error`, and the race-only `not_found`, `not_a_file`) were missing; a
   builder following the draft would have turned a probed disk refusal into `internal`.
4. `decode_error` cannot be witnessed by an `Example` (`files` are UTF-8 `&str`): the draft's
   AC-4 was unbuildable → Q-6.
5. `too_large` examples would need a > 1 MiB document; a literal would add several MiB to
   `docs/commands.json` → Q-7 (growth through `setup`, measured).
6. `when` texts: the draft changed three; five must change. Its new `not_a_file` text dropped
   "a file where a directory is needed", which T-019 made true (`project.tree` of a file →
   `not_a_file`, probed): it would have published a text narrower than the binary. The
   `invalid_pattern` text (D-101), the outline index of `out_of_range` (`nav.goto-symbol`) and
   the replaced root of `outside_root` were missing.
7. Pin list incomplete: `reasons.rs::describe_publishes_every_reason_in_all_order` (:175–187)
   pins `describe() == ALL` and fails under B (the draft called `reasons.rs` unaffected but for
   the row count); `acceptance_t032_control.rs` `checked == 13` (:203); the `registry.rs` unit
   tests whose audit counts break under audit rule (i); the `outside_root` text in
   `acceptance_t008_cli.rs` (:538–541) and `reason.rs`'s unit test.
8. Golden rule 9: the draft's ACs pinned wording (AC-6 `when` texts, AC-7 the paragraph, AC-2 a
   message), the absence of the eleven unpublished kinds (drafted tasks publish them) and the
   exact `doc.save` row (T-027 changes it).
9. v1.2 form: `scorecard/` was in `touches` (only the director edits it); `depends_on: []`; base
   `d8c4a41`; stale line references (`doc-positions.md`:87–88 is now :94–95; `reason.rs`,
   `registry.rs` ranges); no statement of the independent-test-author protocol.
10. The follow-ups of D-101 (text), D-113 and D-126 were only notes: `syntax.md`, `nav.md` and
    `project.rs`, `syntax.rs`, `search.rs`, `nav.rs` were not in `touches`, and no exact edit
    was given.
Ambiguities (resolved in the text or put as questions):
11. Whether a `when` text must hold for every declaring command (`nav.workspace-symbols` has no
    1,000-char cap; `nav.goto-line` clamps): a reading rule added to Interface 5.
12. Disk pairs had no published trace; Q-8 (audit (v)) makes "no refusal example ⇒ disk only".
13. `search.rs`:350's unreachable `RejectReason::Other`: Q-10.
14. AC-7 without a fail-on-base test under rule 9: Q-9.
Notes:
15. T-037's required `Vfs::check_location` vs `nav.rs` `FailingFs` ("To re-check" 2): a merge
    risk for T-037's queue run, not for this spec's content; reported to the director.
16. D-091 bundling. File overlap: the declarations (Interfaces 1–3) touch `registry.rs` and every
    command module; the texts (Interface 4) touch `reason.rs`; they share the generated
    `docs/commands.json`, `acceptance_t032_cli.rs`'s `REJECTED` constant (both edit it: the
    eleven removals and the four texts), the `NEW_PARAGRAPH` pin and the paragraph of
    Interface 5 (whose reading rule is about the texts and whose other sentences are about the
    declarations). The follow-ups (Interface 6) are independent of both but tiny (one comment in
    a module this task edits anyway, two re-wraps). So the overlap is smaller than D-092 states
    (one constant, one paragraph and a generated file, not "the same pinned tests") but real:
    split, the two would serialise on `acceptance_t032_cli.rs` and `docs/control-protocol.md`,
    the second would rebase the generated file, and the reviewer would judge the texts and the
    published set on two diffs although the texts are only meaningful for the published kinds.
    Kept together, the task is larger than the draft (30 commands, 56 refusal examples), which
    raises the risk of passing the 1,100,000 ceiling; the examples are mechanical. What I found
    does not change the grounds of D-092 Q-3; recommendation: keep one task, follow-ups
    included.
Verdict: not ready as drafted (10 blockers). Rewritten above; ready to dispatch once ADR-0006 and
Q-5 to Q-10 are ruled, T-037 has merged and "To re-check at dispatch" holds.
Finalised (R-211) on the rulings of D-128: ADR-0006 accepted, Q-5 to Q-10 as recommended, the
task kept whole. The PENDING marks are gone; "To re-check at dispatch" names what must hold on
main after T-037 merges. Ready to dispatch once that list holds.

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
- 2026-11-14T07:40Z · R-209 · draft · did: spec check (spec check: T-038) against main `e0f4daa`
  (T-023 merged at `1da69fd`, the head I was given drifted from `8ae1381`) and `task/T-037` at
  `2f1d9f8` (given `ec2cb85`; later commits change tests, `paths.md` and the task file only).
  Acknowledged the rulings of D-092 (Q-1 to Q-4, order) and the notes of D-101, D-113, D-126:
  accepted, all written into the body. Read the code of every command module, `registry.rs`,
  `reason.rs`, the VFS walk, `kiln-cli`, `xtask/src/schema.rs`, the protocol documents and every
  merged test that reads `error_kinds`, the reason table, a `when` text or the kinds table.
  Built a private debug `kiln-cli` of main + `task/T-023` + `task/T-037` and probed every pair
  of the per-command table that a session can produce, in memory and on disk (Pinned (g)); the
  scratch build was removed afterwards. Rewrote the spec to v1.2 and to today's code (10
  blockers, "Spec check R-209"), with ADR-0006's proposed text. Tier stays high (CHARTER §3
  decision 2). Pinned lines n/a: (d) for the registry, (e) in the security sense. Unsure: the
  two race-only pairs and `cursor.move`'s and `search.rs`:350's unreachable branches are
  INFERRED; run as root, so permission-denied pairs were not probed (each such command has a
  root-capable witness). · gate attempts: 0 · not run: cargo in the primary checkout or any
  worktree; the unit tests of the merge (T-037's `check_location` vs `nav.rs` `FailingFs` is
  from reading) · missing context: none · next: director rules on ADR-0006 and Q-5 to Q-10,
  re-checks the dispatch list once T-037 has merged.
  QUESTION: Q-5 (VFS kinds incl. race-only pairs), Q-6 (bytes in refusal examples), Q-7
  (`too_large` witness), Q-8 (audit rule (v)), Q-9 (AC-7 without a fail-on-base test), Q-10
  (`search.rs`:350) — see "Questions and rulings".
- 2026-11-14T07:30Z · R-211 · draft · did: finalised the spec on D-128 (a continuation of R-209;
  the first attempt was stopped by the usage limit before any edit and the machine restarted,
  D-129). Acknowledged the rulings: ADR-0006 accepted (filed by the director), Q-5 to Q-10 as
  recommended, task kept whole; accepted, all applied. Removed every PENDING mark; ADR section
  now `status: accepted`, `approval: D-128`; front-matter tier comment and Goal say accepted.
  Q-7: the planner's 0.6–0.7 s figure stays a measurement only; the time the refusal examples add
  to the `schema` step is an item of a new "Implementer's report" list, not a criterion; the AC
  timeouts are marked as test hygiene. Re-read as a builder: audit rule (iii) now names a bad
  `hex` (Q-6 makes it possible); no AC asserts the wording of a `when` text, a message, an audit
  line beyond the command name, a document sentence or the command list (AC-1's comparison of
  `commands --schema` with the generated `docs/commands.json` is the existing currency check, and
  AC-8 checks only that a row and the doc-positions parenthesis are gone). "To re-check at
  dispatch" rewritten: T-023 on main; T-037's k6 (`FailingFs` implements `check_location`) to
  hold on main after T-037 merges, by content, not by head (main `ee58cd8` read; only ledger
  commits since `e0f4daa`). · gate attempts: 0 · not run: cargo, any probe (no behaviour question
  was open) · missing context: none · next: director re-checks the dispatch list after T-037
  merges and dispatches.
<!-- end planner log -->
### implementer
- 2026-11-14T08:21Z · R-215 · ready · did: implemented on `task/T-038` from base `c82fa63`.
  Acknowledged the rulings: D-092 (option B, Q-2 row removal, Q-3 kept whole, Q-4 ADR wording),
  the notes D-101, D-113, D-126, D-128 (ADR-0006 accepted; Q-5 (a), Q-6 (a), Q-7 (a), Q-8 keep
  (v), Q-9 (a), Q-10 (a)), D-130 and the dispatch check D-135: all accepted; none contradicted the
  merged code. ACs in my words: every command declares its kinds and the description publishes
  them, per command and as a union (AC-1); an undeclared refusal becomes `internal`/`other`
  (AC-2); the audit witnesses every in-memory kind and refuses missing, wrong, undeclared and
  disk-kind examples (AC-3); every refusal example refuses with its kind through the binary
  (AC-4); disk and race pairs keep their kinds (AC-5, AC-6); the situations of the five texts
  give their kinds (AC-7); the documents state the set (AC-8); follow-ups (AC-9).
  Mechanism: `registry.rs` adds `Witness {Example, Disk}`, `Refusal {reason, witness}`
  (`Refusal::example`/`::disk`), `SeedFile {Text, Hex}` (manifest form, `SeedFile::text`/`::hex`)
  and `RefusalExample {description, initial_text, files, setup, params, kind}`, re-exported from
  `lib.rs`; `Command` gains two provided items, `fn refusals() -> Option<Vec<Refusal>>` (default
  `None` = no declaration, `Some(vec![])` = declares none) and `fn refusal_examples()` (default
  empty), so the `Command` impls of merged test files compile unchanged. Each command declares in
  its own module (shared helpers: `registry::no_such_document_example`,
  `syntax::too_large_example`, the Q-7 setup). `register` also refuses a kind declared twice (my
  addition; no spec line covers it). Execute: after the run, a `Rejected` of a kind the entry does
  not declare (entries without a declaration are unchecked) → `Internal{Other}` naming command and
  kind, before the result-schema check, nothing logged. `describe`: per command `refusals` (ALL
  order) and `refusal_examples`; `error_kinds.rejected` = ALL filtered by "some command
  declares". Audit: the existing loop, then per command in name order (i), (ii) by kind in ALL
  order, then per refusal example at most one line: (iv) undeclared, (v) disk, else (iii) params /
  files (path, hex) / setup / result; every line names the command.
  Table as DECLARED: equal to the spec's table for all 30 commands, no difference (code read per
  row; `cursor.move` `set_selection` refusal and `search.rs`:350 left undeclared as the notes
  say). Measured from `kiln-cli commands --schema`: 30 commands, 56 E pairs (57 refusal
  examples: `doc.open` has two `not_found`, `missing.txt` and `f.txt/x`), 12 D pairs;
  `error_kinds.rejected` = 13 kinds: invalid_path, no_such_document, unsaved_changes, not_found,
  not_a_file, io_error, decode_error, out_of_range, inverted_range, invalid_pattern,
  not_a_boundary, outside_root, too_large. Five `when` texts copied (checked by script equal to
  Interface 4 strings); variant doc comments of those five rows reworded to match (my reading of
  "the five texts"); module docs of `reason.rs`, `registry.rs` (`describe`, `audit`, `execute`)
  and `lib.rs` updated. `docs/control-protocol.md`: paragraph replaced (whitespace-squashed equal
  to Interface 5, re-wrapped at 100 without splitting a code span), the `rejected`/`other` row
  removed (22 rows). `doc-positions.md` parenthesis removed. Interface 6: `syntax.rs` sentence
  replaced (wrapped over three comment lines; no other line of the comment changed);
  `syntax.md` and `nav.md` paragraphs re-wrapped from the long line on (script asserted the
  squashed paragraph unchanged and every line ≤ 100).
  Own tests (`registry.rs` `refusal_tests`, 6): execute check (declared passes, undeclared →
  internal/other, log empty, invalid params first, undeclared command unchecked); describe
  publishes only declared kinds; manifest form and hex decoding; audit rules (i)–(v) one line each
  naming the command (13 variant commands), disk kind without example no problem; duplicate
  declaration refused; standard registry: all declare, union published, examples only of
  *example* kinds, audit empty. No wording asserted.
  Pin edits (golden rule 4, Pinned (f)): (1) `acceptance_t032.rs` `REJECTED` texts of
  not_found, not_a_file, out_of_range, invalid_pattern: old → new = the base texts → Interface 4;
  `describe_publishes_error_kinds_from_all`: `rejected` from `RejectReason::ALL` → ALL filtered by
  the commands' `refusals`. (2) `acceptance_t032_cli.rs` `REJECTED`: 11 entries removed (other,
  stale_preview, no_such_preview, tool_unavailable, tool_not_found, tool_timeout, tool_failed,
  no_recording, format_error, changed_on_disk, invalid_keymap), the same four texts changed;
  `rows.len(), 23` → 22; `NEW_PARAGRAPH` → Interface 5. (3) `acceptance_t032_control.rs`
  `want_rejected` from ALL → filtered by declared kinds; `rows.len(), 23` → 22 (message "23
  rows" → "22 rows"); `checked, 13` → 12 (message "10 rejected" → "9 rejected"). (4)
  `acceptance_t008_cli.rs` `NOT_A_FILE_WHEN` and the `outside_root` text → Interface 4. (5)
  `reason.rs` `outside_root_is_appended_and_not_a_file_is_widened`: both descriptions → Interface
  4. (6) `reasons.rs` `describe_publishes_every_reason_in_all_order`: `want` (rejected) from ALL →
  filtered by declared kinds; table test `13` → 12, message "10 rejected" → "9 rejected". (7)
  `registry.rs` test commands Echo, Liar, Undocumented, NeedsSetup: `refusals() -> Some(vec![])`
  added; no assertion changed. (8) `kiln-control/src/lib.rs` `ERROR_KINDS`: row
  `("rejected", "other")` removed. Comments in those files that name old counts were left as they
  were (nothing else changed).
  Evidence per published text (debug `kiln-cli` of this branch at `f18e996`, copied out; memory
  cases `kiln-cli run s.jsonl --memory-seed m.json`, disk cases `kiln-cli control --root`, run as
  root, fixtures under `/tmp/t038-impl-probe`, `/tmp/t038-impl-deep`, `/tmp/t038-impl-examples`,
  all removed): not_found — memory `doc.open` missing.txt, nodir/x.txt, f.txt/x (f.txt a file),
  `project.tree` missing → not_found; disk `doc.save` of an open, edited `p/y.txt`: file removed
  → ok (file recreated: True), `p` removed → not_found (directory created: False), `p` replaced
  by a file → not_found, link `slash -> file/` `doc.open` → not_found. not_a_file — memory
  `doc.open d` → not_a_file, `project.tree f.txt` → not_a_file; disk `doc.open` of a directory,
  FIFO, Unix socket, char device (mknod 1,3) → not_a_file each; `project.tree` path a file,
  FIFO, socket, device → not_a_file each; `doc.save` after the file was replaced by a directory →
  not_a_file. out_of_range — memory `doc.insert` offset 99, `doc.position` char 99, byte 99,
  utf16 99, line 9, line 0 col 4 (3-char line), `doc.line 3`, `nav.goto-symbol` index 5 of a
  1-item outline → out_of_range; `nav.goto-line` 99/99 → ok (clamped). invalid_pattern — memory
  `search.find` "" / 1,001 chars / regex `(` / regex `a{99999999}` / 101 groups → invalid_pattern;
  1,000 chars and 100 groups → ok; `search.replace` regex `(a)(b)` with `$3` → invalid_pattern,
  literal `a` with `$3` → ok; `nav.workspace-symbols` "" → invalid_pattern. outside_root — disk
  `doc.open` of a link outside, `project.tree` path a link to an outside dir, `doc.save` after
  the file became a link outside, re-open after the open file became a link outside →
  outside_root; after the root was renamed and a new directory put in its place: `doc.open b.rs`,
  re-open `a.txt`, `project.tree {}`, `nav.workspace-symbols` → outside_root. Other D pairs: link
  loop `doc.open`/`project.tree`/`doc.save`/re-open → io_error; 300-byte name `doc.open` and
  `project.tree` → io_error; 18 nested 250-byte directories (host path 4,542 bytes):
  `nav.workspace-symbols` and `project.tree {}` → io_error. All 57 refusal examples run as the
  evaluator would (manifest from `files`, script of `setup` + request, `run --memory-seed`): 57
  rejected with their kind, 0 failed. Interface 5: union and per-command sets from `--schema`
  (above); "no refusal example ⇒ disk only" = audit (ii)/(v) plus the D probes above; "undeclared
  ⇒ internal/other" = unit test (no registered command can show it through the binary).
  Guarantee sentences the spec lacks: none added.
  Schema step time (`/usr/bin/time cargo xtask schema`, this machine, another builder active):
  base `c82fa63` warm 0.48 s and 0.63 s (a first run that compiled xtask took 23.6 s, discarded);
  branch `e78ec66` warm 3.54 s, 3.49 s, 3.34 s; `kiln-cli commands --audit` alone 3.05 s
  (debug). The five `too_large` examples each took 0.62–0.72 s as separate `kiln-cli run`
  processes (about 3.3 s together, process start included), so they are most of the increase;
  their exact share inside the audit was not measured separately. In the gate the step read 5.2 s.
  · gate attempts: 2 (1: red clippy `manual_is_multiple_of` in my hex decoder, fixed; 2: green,
  `GATE PASSED in 551.6s (11 steps; 2 EMPTY: fuzz, budgets)`, test: 857 passed, 0 failed, 0
  ignored in 108 suites) · not run: the test author's `acceptance_t038*`/`guard_t038*` (not
  visible to me; their declaration API may need adapting to mine at the join: `Command::refusals`
  / `refusal_examples`, `Refusal::example`/`::disk`); permission-denied cases as an unprivileged
  user (no text this task changes names permissions); the two race pairs through a test VFS
  (declared *disk* per Q-5; only the existing `nav.rs` unit test shows a walk read failure
  keeping its kind); `cursor.move`'s and `search.rs`:350's branches (unreachable, INFERRED) ·
  missing context: none · next: director joins with `task/T-038-tests`; reviewer.
<!-- end implementer log -->
### test author
- 2026-11-14T08:09Z · R-216 · ready · did: independent tests on `task/T-038-tests` (base
  `c82fa63`) from the spec alone. Acknowledged the rulings before acting: D-092 (Q-1 option B,
  Q-2 row removed, Q-3 kept whole, Q-4, order), notes D-101/D-113/D-126, D-128 (ADR-0006
  accepted; Q-5 race pairs declared and witnessed by a guard → AC-6; Q-6 manifest-form `files` →
  AC-4 copies them as the manifest; Q-7 growth by setup; Q-8 audit (v); Q-9 AC-7 a guard with
  no wording test; Q-10 `search.rs`:350 undeclared), D-130 (AC-8's "phrase gone" check not
  written; AC-1's comparison with `docs/commands.json` not written; AC-3's "line names its
  command" kept), D-135 (dispatch facts): all accepted. Wrote
  `apps/kiln-cli/tests/acceptance_t038_cli.rs` (AC-1 `schema_publishes_each_commands_refusals`,
  AC-4 `every_refusal_example_refuses_through_the_binary`, AC-8
  `protocol_docs_state_the_published_set`), `crates/kiln-commands/tests/acceptance_t038_commands.rs`
  (AC-2 `undeclared_refusal_is_internal`, AC-3 `audit_requires_a_witness_for_every_declared_kind`;
  partial, see QUESTION), `apps/kiln-cli/tests/guard_t038_cli.rs` (AC-5
  `disk_refusals_through_control`, AC-7 `situations_of_the_changed_when_texts`),
  `crates/kiln-commands/tests/guard_t038_commands.rs` (AC-6 `race_refusals_keep_their_kind`).
  The spec's table is encoded once as data (`TABLE`, command → E, D) and self-checked against
  the spec's counts (30 commands, 56 E, 12 D, 13 kinds); order is checked against
  `RejectReason::ALL`. Base results: AC-1 assertion (no `refusals` in any entry); AC-4 assertion
  (no `refusal_examples`; 60 problems); AC-8 assertion (the `rejected`/`other` row is there);
  AC-2 assertion (`describe()` has no `refusals`; the unchecked `decode_error` part passes);
  AC-3 assertion (audit of a command without a declaration: 0 lines, want 1; the standard
  registry's audit is empty); guards AC-5 (11 cases, the 10 C pairs), AC-6 (4 cases, each with
  a nothing-fails control and a hit counter), AC-7 (memory and disk cases, each bound with its
  just-inside request ok) pass. Scratch prototype in `registry.rs` (removed with `git checkout`,
  never committed): a hard-coded declaration, `describe()` with `refusals`/`refusal_examples`
  (5 examples: exec path, `hex` decode_error, setup path, `too_large` by growth, one wrong kind),
  an execute check and audit rule (i): with it AC-1, AC-2, AC-3 and the guards pass and the AC-4
  runner passes the 4 sound examples. Kills: (1) publish undeclared `other` → AC-1, AC-2; (2)
  omit `not_a_boundary` from `cursor.set` → AC-1; (3) no execute check (undeclared refusal
  stays `rejected`) → NOT killed (needs AC-2's declared half, QUESTION); (4) check turns
  `outside_root`/`io_error` into `internal` → AC-5, AC-7; (5) check turns `not_found`/
  `not_a_file` into `internal` → AC-5, AC-6, AC-7; (6) audit rule (i) skipped → AC-3; (7) a
  refusal example producing another kind → AC-4. `cargo fmt --check` clean; `cargo clippy -p
  kiln-cli -p kiln-commands --tests -- -D warnings` clean on base (all four files compile on
  base); `cargo xtask no-skip` PASS. · gate attempts: 0 (not run: the whole gate, by the
  director's instruction on CPU) · not run: `cargo xtask ci`; the permission-denied pairs (not
  needed, Pinned (d)) · missing context: the declaration API (below) · next: director rules on
  the QUESTION; the declared halves of AC-2 and AC-3 are added on this branch once the API is
  named.
  QUESTION: AC-2 (a declared `not_found` passes, an undeclared `decode_error` becomes
  `Internal { reason: Other }`, `describe()` lists only `not_found`) and AC-3 rules (ii)–(v),
  "an empty list is a declaration" and "a *disk* kind without an example gives no problem"
  need a test command WITH a declaration and refusal examples, but Interface 3 leaves the API
  to the implementer ("a trait item …, a method, a builder"), so these cannot be written from
  the spec without guessing. Written now: only what needs no declaration (a command written
  with today's `Command` items is one without a declaration, as Interface 3 and Pinned (f) 7
  say). Proposed wording: Interface 3 names the API, e.g. `Command::refusals() ->
  Vec<Refusal>` with `Refusal::example(RejectReason)` / `Refusal::disk(RejectReason)` (default:
  `None` = no declaration) and `Command::refusal_examples() -> Vec<RefusalExample>` with the
  fields of Interface 1, or the director hands the test author the implementer's public
  signatures (only these) before the join. Prototype bug (3) shows the cost: without it no test
  of this task catches a missing execute check.
  Interpretations: an undeclared command publishes `refusals: []` and contributes no kind
  (Interface 1: "the kinds the command declares, … possibly empty"); a refusal example's
  `setup` has the `[name, params]` form of `examples` ("as in `examples`"); AC-3's `kiln-cli
  commands --audit` exits 0 is asserted in AC-4's test (the CLI file can run the binary; the
  commands file asserts `standard_registry().audit()` is empty); `run` of a refusal example
  exits 3 and `exec` exits 2 (the documented exit codes); D-130's "phrase" ruling read as
  covering AC-8's `doc-positions.md` check (the only phrase check of AC-8).
- 2026-11-14T08:16Z · R-217 · ready · did: acknowledged ruling j1 (D-136; read in the director's
  copy of this file): the public declaration API is pinned (`Witness`, `Refusal::{example,
  disk}`, `SeedFile::{text, hex}`, `RefusalExample`, provided `Command::refusals` default `None`
  and `Command::refusal_examples` default empty); my R-216 interpretations accepted. Accepted;
  acted on it. Rewrote `crates/kiln-commands/tests/acceptance_t038_commands.rs` on that API:
  AC-2 `undeclared_refusal_is_internal` now has the declared half (`t38.declared` declares
  `not_found`: `not_found` → `Rejected{NotFound}`, `decode_error` → `Internal{Other}`, nothing
  logged; `describe()` lists `["not_found"]` for it, `[]` for the undeclared `t38.alpha`, whose
  `decode_error` stays `Rejected`, and `error_kinds.rejected` = `[not_found]`; plus `Some(vec![])`
  refusing → `internal`, a declared *disk* `io_error` → `rejected` and published); AC-3
  `audit_requires_a_witness_for_every_declared_kind` now has, one registry of one test command
  each, exactly one line naming the command for (i), (ii), (iii) ×6 (succeeds, another declared
  kind, params fail the schema, setup fails, invalid `files` path, odd `hex`), (iv), (v); zero
  lines for a sound declaration, a *disk* kind without an example, `Some(vec![])`, a disk-only
  declaration; the two-command name-order case and the empty standard audit stay. Test commands'
  `examples()` keep the `Example` literal of `acceptance_t032.rs` (empty `files`/`setup`), which
  Interface 3 requires to keep compiling; `register` refuses an empty `examples()` on base, so an
  empty vector was not used. Base: this file no longer compiles (E0407 `refusals`/
  `refusal_examples` not trait members, E0432 `Refusal`/`RefusalExample`/`SeedFile`): weak
  evidence; its R-216 version (`667d77a`) failed by assertion. Every other T-038 file is
  unchanged and keeps its base result. Scratch stub of exactly the j1 signatures plus a
  prototype of the execute check, `describe()` and audit rules (i)–(v) in `registry.rs`/`lib.rs`
  (removed with `git checkout`, never committed): both commands-crate files pass; kills: no
  execute check (undeclared refusal stays `rejected`) → AC-2; rule (i) skipped → AC-3; (ii)
  skipped → AC-3; (iii) accepting any `rejected` kind → AC-3; (v) skipped → AC-3. Not
  distinguishable by count: a prototype that skips (iv) but runs the example still reports one
  line (via (iii)), which AC-3's "exactly one line" accepts. `rustfmt --check` on the four files
  clean; `cargo clippy -p kiln-commands -p kiln-cli --tests -- -D warnings` clean with the stub
  (stub-only lints `missing_docs` etc. allowed for the scratch code); `cargo xtask no-skip`
  PASS. · gate attempts: 0 · not run: `cargo xtask ci`; the CLI files against the stub (unchanged
  since R-216) · missing context: none · next: join.
- 2026-11-14T09:47Z · R-219 · rework · did: acknowledged ruling j2 (D-137, review round 1
  finding 1; read in the director's copy of this file): accepted (my (iv) case had a declaration,
  so rule (iii) alone gave its one line). Added inside
  `audit_requires_a_witness_for_every_declared_kind` one case: a registry of one command
  `t38.undeclared-with-example` with no declaration (`refusals()` = `None`) and one refusal
  example of `not_found` whose run refuses `not_found`; `audit()` → exactly 2 lines, each naming
  the command (rules (i) and (iv)). No other case or name changed. · gate attempts: 0 · ran:
  `cargo fmt --check` clean, `cargo xtask hygiene` PASS · not run: compiling or running the test
  (it needs the new API; the join compiles it) · missing context: none · next: join.
<!-- end test author log -->
### reviewer
- 2026-11-14T09:45Z · R-218 · tests_joined · review round 1 (high, independent) of
  `9fd40ae2a87d4007c34719e2fb7dd65b8d3684aa` against the primary copy of this file. Rulings
  D-092, D-128 (ADR-0006, Q-5 to Q-10), D-130, D-135 and j1 (D-136) acknowledged and accepted.
  did: `commands --schema` (equal to `docs/commands.json`; 30 commands, 13 kinds = the union
  of `refusals`, every set equal to the table); `--audit` ok; all 57 refusal examples run by my
  own script through `run --memory-seed` (57/57 their kind); about 116,000 fuzzed in-memory
  requests of all 30 commands (0 `internal`); about 150 `control --root` disk sessions (links,
  loops, FIFO/socket/device, long names and paths, replaced, removed or relinked roots,
  permission-denied with the DAC capabilities dropped via `capsh`), all 12 D pairs produced,
  the two R pairs by a real race (0 `internal`); the five `when` texts and the
  `control-protocol.md` paragraph against the binary (true, no wider or narrower case found);
  the Pinned (f) edits (exactly as listed); base vs head byte-identical on 16,320 fuzzed
  requests plus disk and CLI cases (base built from `git archive c82fa63` in scratch, `target/`
  deleted); schema step 0.03–0.06 s → 2.9–3.0 s for the audit, `cargo xtask schema`
  3.4–4.0 s; 15 mutants, 14 killed (all by T-038 acceptance tests), 1 survived (audit rule
  (iv) skipped: masked by (iii) through the execute check; test gap). Verdict request-changes
  (0 defects, 1 test gap, 1 spec gap: ADR-0004's bullet not replaced, 3 nits): `reviews/T-038-r1.md`.
  · gate attempts: 0 (not run: `cargo xtask ci`; ran `cargo test -p kiln-commands -p
  kiln-control -p kiln-cli` 360/360, `no-skip`, `schema`) · not run: a real unprivileged uid,
  `exec` exit 1 for internal, save onto a socket/device · missing context: none · next:
  director rules on finding 1 (add a no-declaration (iv) case, or rule (iv) redundant) and
  finding 2; round 2 on the delta.
- 2026-11-14T10:05Z · R-220 · rework · review round 2 (delta, final) of
  `82201ca5db605d31ccaa543c7688dcf9722eb488` since `9fd40ae`. Rulings j2 to j5 (D-137)
  acknowledged and accepted. did: the delta is only the j2 test case, the test author's log
  entry and my round-1 commit; `acceptance_t038_commands` passes on the head (2/2); my mutant M11
  (audit rule (iv) skipped) is now killed by the new case, and the tree was clean after the
  restore. Verdict approve (`reviews/T-038-r2.md`). · gate attempts: 0 · not run: the full gate
  (the re-join ran it) · missing context: none · next: director records the approval and queues.
<!-- end reviewer log -->
