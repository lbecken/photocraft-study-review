---
id: T-037
title: "Check the root and the path's location when `doc.open` names a path that is already open, and say so in paths.md and in doc.open's description"
tier: high              # ADR-0002: a confinement boundary that docs/protocol/paths.md states ("at open … a path whose location is outside the root is refused")
state: draft
owner_role: planner
scorecard_items: [ws.path-confinement]   # existing item, strengthened (no new item)
depends_on: []          # T-008 merged at d4915606f9a789b42b99a1eea053554f1f9dd8a7; never in flight with T-036 (see Conflicts and dispatch)
touches:
  - crates/kiln-workspace/src/vfs.rs                         # implementer: the location-only check on the Vfs trait (see Interface, Rust API)
  - crates/kiln-workspace/src/disk.rs                        # implementer: DiskFs implements it with the existing root check and walk (shared with T-036)
  - crates/kiln-workspace/src/memory.rs                      # implementer: MemoryFs implements it (always inside)
  - crates/kiln-commands/src/doc.rs                          # implementer: doc.open's re-open branch, its DOC, the module docs and the `already_open` field doc
  - docs/protocol/paths.md                                   # implementer: the sentences named under Documentation (shared with T-036)
  - crates/kiln-commands/tests/acceptance_t037_commands.rs   # test author: AC-1, AC-2 through Registry::execute on a DiskFs context
  - apps/kiln-cli/tests/acceptance_t037_cli.rs               # test author: AC-1, AC-2, AC-4 through kiln-cli control; AC-5 (documents)
  - apps/kiln-cli/tests/guard_t037_cli.rs                    # test author: AC-3 (re-opens that keep working) through kiln-cli control and run
  - scorecard/items/ws.path-confinement.toml                 # test author: append this task's test names (shared with T-036)
  # docs/commands.json changes too (doc.open's DOC and the `already_open` field description): generated, `cargo xtask schema --write`
  # docs/scorecard.md changes too: generated, `cargo xtask scorecard`
budget_tokens: 1100000  # high (D-091 change 3)
iteration: 2
found_by: evaluator     # evaluations/iter-1.md NEW-1 (run R-152)
origin_task: T-008
---

# T-037 Check the root and the path's location when `doc.open` names a path that is already open

## Goal
Today `doc.open` of a path that is already open in the session returns the open document with
`already_open: true` and checks nothing. It does so even when the path now leads outside the
root, or the root has been replaced or has vanished (VERIFIED: `crates/kiln-commands/src/doc.rs`:172–178
looks the path up among the open documents and returns before any file-system call;
reproduced by the evaluator, `evaluations/iter-1.md` NEW-1, and by this planner's probes under
Pinned (g)). `docs/protocol/paths.md`:48–50 says "At open and at save … A path whose location
is outside the root is refused", and :19–20 says that once the root path stops leading to the
root "the session refuses every path with `outside_root`, at open and at save". The document
and the binary disagree.

After this task a re-open applies the same root-identity and location checks that a first open
and a save apply, and nothing more: a path whose location is now outside the root, or a session
whose root path no longer leads to its root, is refused with `outside_root`. A re-open still
does not require the file to exist, to be a regular file, to be readable or to decode, and it
reads nothing. `paths.md` and `doc.open`'s description say exactly this, and tests pin it.

No gate path and no dependency in `touches`. Tier high by ADR-0002: the confinement boundary
that `docs/protocol/paths.md` states.

Reading list: `evaluations/iter-1.md` "ws.path-confinement" and NEW-1; `docs/protocol/paths.md`
(whole); `tasks/T-008-path-confinement.md` Definitions, Interface (error table, precedence),
Pinned (c)–(e); `tasks/T-036-root-identity-follow-ups.md` "Conflicts and dispatch";
`crates/kiln-commands/src/doc.rs` (module docs, `Open`); `crates/kiln-workspace/src/vfs.rs`;
`crates/kiln-workspace/src/disk.rs` (`current_root`, `locate`, `write_atomic_with`, the `Vfs`
impl); `crates/kiln-workspace/src/memory.rs`; `apps/kiln-cli/tests/acceptance_t008_cli.rs`
(fixtures, the `control` driver with per-response timeouts, snapshots).

## Definitions
- **Root, location, inside, the walk**: as `docs/protocol/paths.md` "The root" and "Location",
  and T-008 Definitions.
- **Open path**: the path of a document open in the session. A request path *is already open*
  iff, after the lexical check, it is equal as a string to the path of an open document. This
  is how the code keys it today (VERIFIED: `doc.rs`:172–176 compares `RelPath` values, which
  hold the path exactly as given, `crates/kiln-workspace/src/path.rs`:7–8). So the key is the
  path text, not the file: in a root holding `l -> a.txt`, opening `a.txt` and then `l` gives
  two documents (VERIFIED by probe, Pinned (g)). This task does not change the key.
- **Re-open**: a `doc.open` whose path is already open.
- **Location check**: the root-identity check followed by the walk of the path to its
  location, as `write_atomic` performs them before it looks at what is at the location
  (`disk.rs` `locate`). Its outcome is one of: the location is inside the root (whatever is or
  is not there); `OutsideRoot` (the root path no longer leads to the root directory, or the
  location is outside, or the walk failed once outside); or an I/O failure of the walk inside
  the root (for example more than 40 symlinks, or a directory on the way that cannot be
  searched).

## Options (the director's question Q-1)
What each option answers to a re-open of `a.txt` (open, edited) after the change named in the
first column. "ok" means `already_open: true` with the open document unchanged. "Reads" says
whether the re-open reads the file.

| After the open, outside Kiln | O1 location check only (recommended) | O2 full open check, result discarded | O3 key by location | O4 document only |
|---|---|---|---|---|
| `a.txt` deleted | ok | `not_found` | open as new path? (see below) | ok |
| `a.txt` replaced by a directory | ok | `not_a_file` | as O2 | ok |
| `a.txt` replaced by a FIFO | ok, without blocking | `not_a_file`, without blocking | as O2 | ok |
| `a.txt` replaced by a link to `b.txt` inside | ok (a later save writes `b.txt`, as paths.md already says for links) | ok, after reading `b.txt` | a different file: a new document? | ok |
| `a.txt` no longer decodes | ok | `decode_error` | as O2 | ok |
| `a.txt` replaced by a link outside the root | `outside_root` | `outside_root` | `outside_root` | ok |
| root renamed away, replaced by a link, or removed | `outside_root` | `outside_root` | `outside_root` | ok |
| a link loop at `a.txt` | `io_error` | `io_error` | `io_error` | ok |
| Reads the file | never | every re-open, whole file | yes | never |

Consequences:
- **O1** (the director's leaning). The re-open is refused exactly when a save of the same
  document would be refused for confinement, so "already open" can no longer vouch for a path
  that leads outside. It needs one new capability on the `Vfs` trait (the location check
  without existence, type or read), because the trait today offers only `read`, `write_atomic`
  and `is_file`, and `is_file` answers `false` for "outside" and for "missing" alike
  (`vfs.rs`:64–71). `MemoryFs` has no root identity and no links, so a re-open there is
  unchanged.
- **O2** needs no trait change (call `read` and discard). But a document whose file was deleted
  or replaced by a directory can no longer be re-opened, although it is still open and holds
  unsaved edits. Every re-open reads the whole file, and the result still describes the old
  document, not what was read.
- **O3** changes what "already open" means: two paths to one file would share a document, ids
  would follow the file, and a deleted file has no location to key on. That is a feature with
  its own design (and T-019 and T-027 interact with it); out of scope.
- **O4** only edits `paths.md` and `doc.open`'s description ("a re-open is not checked"). It
  is the cheapest, and nothing is read or written either way (the evaluator's "Impact"). But it
  keeps a refusal-free answer for a path that now leads outside the root, and it narrows a
  confinement sentence instead of making the binary meet it.

**Recommendation: O1.** It matches the director's leaning, its behaviour is a strict subset of
what a save already does (no new rule to learn), and it reads nothing. The rest of this file
specifies O1. Under O4, Interface 1 to 3 disappear, AC-1, AC-2 and AC-4 become guards that pin
today's answers, and AC-5 pins the "not checked" sentences instead.

## Interface
`doc.open` params and result are unchanged. No new kind and no `reason.rs` row: the re-open
uses the kinds `outside_root` and `io_error`, which `doc.open` already reports.

1. **Re-open on a disk session.** A `doc.open` whose path is already open, in a session on the
   disk file system (`kiln-cli run`, `exec` or `control` with `--root`, or the default root):
   - performs the location check of the path and nothing else; it opens, reads, creates or
     changes nothing;
   - is refused with `rejected` / `outside_root`, message `cannot open <path>: resolves outside
     the root`, if the outcome is `OutsideRoot`;
   - is refused with `rejected` / `io_error`, message beginning `cannot open <path>: I/O
     error: `, if the walk failed inside the root (the rest of the message is not pinned);
   - otherwise returns `{"already_open": true, "doc": <id>, …}` exactly as today: the open
     document's id, path, encoding and its current text's line endings and length. Whether
     a file exists at the location, what type it has, and whether it can be read or decoded
     do not matter.
2. **A refused re-open leaves the session as it was**: the document stays open with the same
   id, text, dirty flag, undo/redo history and selection; `doc.list` is unchanged; nothing is
   added to the session log (as for every refused request). Once the path leads inside the
   root again (for example the root moved back), the re-open answers `already_open: true`
   again.
3. **Re-open on an in-memory session** (`--memory`, `--memory-seed`, `Context::new`,
   `Host::in_memory`): unchanged, always `already_open: true`.
4. **First open** (a path that is not open): unchanged in every respect.
5. **Rust API.** `Vfs` gains one required method that performs the location check and returns
   `Ok(())` or the `VfsError` (`OutsideRoot` or `Io`). Its name, and whether it is exposed
   beyond `Vfs`, are the implementer's choice; it must not open, read, create or remove
   anything. `DiskFs` implements it with the existing root check and walk; `MemoryFs` returns
   `Ok(())`. No other public signature changes. (The acceptance tests do not call it: they go
   through `doc.open`.)

### Order of checks for `doc.open`
`invalid_params` (schema) → `invalid_path` (lexical) → is the path already open?
- yes → location check (`outside_root`, then `io_error`) → `already_open: true`;
- no → as today (paths.md "The rule": `outside_root`, `not_found`, `not_a_file`, `io_error`,
  then `decode_error`).

### Documentation
- `doc.open` DOC (`crates/kiln-commands/src/doc.rs`:135–136), after this task exactly:
  "Open a file as a document and return its id. Opening a path that is already open returns
  the existing document unchanged, with `already_open: true`, without reading the file again;
  the path is still checked first, and one that now resolves outside the root is refused with
  `outside_root`."
- The `already_open` field doc (`doc.rs`:117) says the same in one sentence; the module docs'
  "Ids" ruling (`doc.rs`:8–13) gains the sentence that a re-open is refused like any open when
  the path now resolves outside the root, and then changes nothing. `docs/commands.json` is
  regenerated.
- `docs/protocol/paths.md`:
  - "The root" (:17–22): "the session refuses every path with `outside_root`, at open and at
    save, including the save of a document opened before" gains "and the re-open of a path
    that is already open".
  - "The rule": a new bullet after "Each request is checked again" (:69–73): **"A path that is
    already open is checked again, for its location only.** A `doc.open` of a path that is
    open in the session (the same path text) is refused with `outside_root` when the root path
    no longer leads to the root or the path's location is now outside the root, and with
    `io_error` when the walk fails inside the root. Otherwise it returns the open document
    unchanged with `already_open: true` and reads nothing, so it succeeds even if the file
    has since been deleted, replaced by a directory, a FIFO or a link inside the root, or no
    longer decodes. A refused re-open leaves the document open and unchanged."
  - The precedence sentence (:75–76) gains: "For a path that is already open, only
    `invalid_path`, `outside_root` and `io_error` can be reported, in this order."
  - Nothing else in `paths.md` changes. Line numbers refer to main
    `73a8d13c48cc09e7ac9f238d4848c888341d9fb0`; if T-036 merged first, apply the same sentences
    to its text.
- `docs/control-protocol.md` is not edited: its Paths bullet ("A path whose location is outside
  the root is refused (`outside_root`); see `docs/protocol/paths.md`") stays true.

## Pinned
- (a) Errors, payloads, precedence: no new variant or kind. Re-open: `outside_root` (`cannot
  open <path>: resolves outside the root`), then `io_error` (`cannot open <path>: I/O error:
  …`), as in "Order of checks". Error object as every protocol error; `exec` exits 2 and `run`
  exits 3 on a refusal, as today.
- (b) Files: nothing on disk is opened, read, created, renamed or removed by a re-open, refused
  or not. `docs/commands.json` changes only in doc.open's `doc` text and the `already_open`
  description.
- (c) Order of checks: above. The re-open never calls `read`; the location check runs once per
  re-open; nothing is cached between requests (paths.md "Each request is checked again").
- (d) Path base, platform, file types: as T-008 (d). Unix only. The tests use symlinks,
  directories, a FIFO (created as `acceptance_t008_cli.rs`:122–130 does, by running `mkfifo`
  under the test's timeout), and renames and removals of the root. No socket or device is
  needed.
- (e) Threat model: see below.
- (f) Tests pinning a snapshot this changes: none. `grep already_open` over every `tests/`
  directory finds re-opens only in sessions whose files do not change between requests
  (`guard_t032_cli.rs`, `guard_t015_cli.rs`, `acceptance_t003.rs`, `guard_t006.rs`,
  `guard_t015.rs`, `acceptance_t004.rs`, `acceptance_t011.rs`); their responses stay
  byte-identical. No test pins doc.open's DOC text (`grep` for "already open returns the
  existing" finds only `doc.rs`).
- (g) Worked examples, executed by this planner with the debug `kiln-cli` of main
  `73a8d13c48cc09e7ac9f238d4848c888341d9fb0` (its `commands --schema` equals the committed
  `docs/commands.json`), through `kiln-cli control --root root` in a throwaway directory holding
  `root/a.txt` = `alpha\n`, `root/b.txt`, `root/sub/c.txt` and `outside/secret.txt`. Each case:
  `doc.open a.txt` → doc 1; `doc.insert` `E` at 0; the change below; `doc.open a.txt`;
  `doc.info`; `doc.save`.
  - On base, the second `doc.open` answered `{"already_open":true,"doc":1,…,"len_chars":7,…}`
    after every change: `a.txt` deleted, replaced by a directory, by a FIFO, by a link to
    `b.txt`, by a link to `../outside/secret.txt`, by a dangling link to
    `../outside/none.txt`, by undecodable bytes; the root renamed away; the root renamed away
    and replaced by a link to `outside`. The save then gave: deleted → ok (file recreated);
    directory → `not_a_file`; FIFO → `not_a_file`; link to `b.txt` → ok; link outside,
    dangling outside, root renamed, root replaced → `outside_root` (`cannot save a.txt:
    resolves outside the root`); undecodable → ok. `outside/secret.txt` was unchanged in every
    case.
  - `sub/c.txt` open, then `sub` replaced by a link to `../outside`: re-open of `sub/c.txt` →
    `already_open: true`; a first open of `sub/secret.txt` in the same state → `outside_root`.
  - Root holding `l -> a.txt`: `doc.open a.txt` → doc 1, `doc.open l` → doc 2
    (`already_open: false`), `doc.open a.txt` → doc 1 `already_open: true`.
  - First opens on base (from `kiln-cli run`): a link loop → `io_error` `cannot open loop1: I/O
    error: too many levels of symbolic links`; a link to an outside file → `outside_root`.
  - Under O1 the answers become those of the O1 column above. NOT executed (no new code):
    INFERRED from the walk's documented outcomes (`disk.rs` module docs, `locate`) and from the
    save answers above, which run the same walk.
- (h) Scorecard item `ws.path-confinement`. The evaluator runs `kiln-cli control --root root`,
  opens and edits `a.txt`, replaces `a.txt` by a link to an outside file, sends `doc.open
  {"path":"a.txt"}` and gets `rejected` / `outside_root`; then renames the root away and gets
  the same; then moves it back and gets `already_open: true` with the edited text.

## Threat model
Kept from T-008 (e) and T-036: an honest user or a hostile repository; another process may
change the tree between two requests, not during one.
- **Covered.** Between two requests, the file, a directory on its path, the root or a
  directory above the root is replaced so that the open path leads outside or the root path
  leads elsewhere or nowhere: the re-open is refused, nothing is read anywhere, and the
  document stays as it was.
- **Not covered (accepted, documented).** Changes during one request (T-008 D-3); hard links;
  the residual limits T-036 is about (an unreadable root); `MemoryFs` (no links, no root).
- **What a re-open reveals.** As for a first open: whether the location is inside or outside
  (and, through `io_error`, that the walk failed inside). Nothing about the content of an
  outside file, and no host path in any message.
- **Reviewer probes adversarially:** re-open after each change of the Options table, through
  `run`, `exec` (fresh session: a re-open cannot occur there, which the reviewer confirms) and
  `control`; chains that leave and come back; a parent directory swapped for a link outside;
  the root renamed away and moved back; a FIFO at the path (no blocking); a re-open
  interleaved with saves of the same document.

## Acceptance criteria
Fixtures as in `acceptance_t008_cli.rs`: a fresh `work/` per test with `work/root/`,
`work/outside/` and `work/outside/secret.txt` = `b"s3cr3t-content\n"`; `work/root/a.txt` =
`b"alpha\n"`, `work/root/b.txt` = `b"bravo\n"`. Each test first asserts through `std::fs` that
the change it makes is in place (for example that `root/a.txt` now reads `s3cr3t-content`),
so it cannot pass vacuously. "Snapshot" means every entry under `work/`, recursively, links
not followed: type, link target, bytes. Every child process has a hard timeout of 10 s and
every `control` response is awaited at most 10 s. No test asserts a count of `error_kinds`,
a whole document's bytes, or an unpinned message text. No test returns early or skips.

Base = main `d8c4a4199ca622f8a46dd09dcccb50b9f4bc4003` (no file this task concerns differs from
`73a8d13c48cc09e7ac9f238d4848c888341d9fb0`, where the probes ran). **Acceptance** = fails on base;
**guard** = passes on base.

In each control session below, the first requests are `doc.open a.txt` (doc 1) and
`doc.insert {"doc":1,"offset":0,"text":"E"}`, then the change, then the requests named.

- **AC-1 (acceptance; `acceptance_t037_cli.rs::reopen_of_a_path_now_outside_is_refused`).**
  Changes, each in its own session: (i) `root/a.txt` replaced by a link to
  `../outside/secret.txt`; (ii) by a dangling link to `../outside/none.txt`; (iii) `root/sub/c.txt`
  opened instead of `a.txt` and `root/sub` replaced by a link to `../outside` holding `c.txt`.
  Then `doc.open` of the open path → `{"code":"rejected","kind":"outside_root","message":"cannot
  open <path>: resolves outside the root"}`; then `doc.info {"doc":1}` → `dirty: true`,
  `len_chars` 7 (or that of the edited `c.txt`); `doc.text` → the edited text; `doc.list` →
  the one document. The snapshot of `work/outside` is unchanged and `outside/none.txt` does
  not exist. Oracle: exact values. Fails on base (`already_open: true`).
- **AC-2 (acceptance; `acceptance_t037_cli.rs::reopen_after_the_root_changed_is_refused` and
  `acceptance_t037_commands.rs::reopen_after_the_root_changed_is_refused`).** Changes: (i)
  `root` renamed to `root.away`; (ii) then a link `root -> outside` created; (iii) `root`
  removed with `remove_dir_all` after (i) was undone. After each: re-open of `a.txt` →
  `outside_root` with the exact message; the document unchanged as in AC-1. After (i) is undone
  (renamed back): re-open → `already_open: true`, `doc` 1, `len_chars` 7. The Rust-API variant
  builds a `Context::with_vfs(Box::new(DiskFs::new(root)))`, dispatches by name through
  `standard_registry().execute`, and asserts the same JSON values and `CommandError` fields
  (`reason` `OutsideRoot`, message). Fails on base.
- **AC-3 (guard; `guard_t037_cli.rs::reopen_does_not_need_the_file`).** Changes, each in its own
  session: `root/a.txt` deleted; replaced by a directory; replaced by a FIFO; replaced by a link
  to `b.txt`; overwritten with the bytes `FF FE FF`. Re-open → `{"already_open":true,"doc":1,
  "encoding":"utf-8","len_chars":7,"line_ending":"lf","mixed_line_endings":false,"path":"a.txt"}`
  (exact). For the FIFO the response arrives within the 10 s limit and the FIFO is still a FIFO
  afterwards. For the link to `b.txt`, `b.txt` still reads `bravo\n` after the re-open. Also:
  in `kiln-cli run --memory-seed` with `a.txt`, open, insert, re-open → `already_open: true`
  unchanged. Passes on base and must keep passing.
- **AC-4 (acceptance; `acceptance_t037_cli.rs::reopen_through_a_link_loop_is_io_error`).** `root/a.txt` replaced by
  `a.txt -> loop2`, `loop2 -> a.txt`. Re-open → `rejected` / `io_error`, message beginning
  `cannot open a.txt: I/O error: `; document unchanged. Fails on base.
- **AC-5 (documents; acceptance; `acceptance_t037_cli.rs::documents_state_the_reopen_rule`).**
  `kiln-cli commands` prints doc.open's DOC exactly as in Documentation (whitespace-insensitive);
  `docs/protocol/paths.md`, whitespace-squashed, contains "A path that is already open is
  checked again, for its location only." and "For a path that is already open, only
  `invalid_path`, `outside_root` and `io_error` can be reported, in this order."; and
  `docs/commands.json` equals `kiln-cli commands --schema`. Fails on base. The reviewer checks
  the rest of the Documentation list sentence by sentence.

Scorecard: the test author appends the AC-1, AC-2 (both) and AC-4 test names, and AC-3 as a
guard, to `scorecard/items/ws.path-confinement.toml` and regenerates `docs/scorecard.md`.

## Out of scope and implementer notes
- What "already open" is keyed on (O3) and anything about two paths to one file.
- Re-checking anything but the location on a re-open (O2): existence, type, readability,
  decoding, or whether the file changed on disk (T-027's subject).
- T-036's subjects: the unreadable root, root-removal detection reliability, `/.` link targets.
- `docs/control-protocol.md`; `kiln-control` and `kiln-cli` sources (they report the refusal
  through the existing paths).
- Implementer's own tests: unit tests in `disk.rs`, `memory.rs` and `doc.rs` (variant matches
  belong there). `crates/kiln-workspace/tests/confinement.rs` is T-036's and is not in this
  task's `touches`. Determinism: no clock or thread timing; the location check opens nothing.
- Do not add an example to doc.open for this (examples run on `MemoryFs`, where the re-open is
  unchanged).

## Conflicts and dispatch
- **T-036** (draft) shares `crates/kiln-workspace/src/disk.rs`, `docs/protocol/paths.md` and
  `scorecard/items/ws.path-confinement.toml`. Never in flight together (METHODOLOGY §5.2).
  Recommended order: T-036 first if the director answers its Q-1 to Q-3 before this task is
  `ready`, because it rewrites "The root" paragraph that Documentation above adds a clause to;
  otherwise this task first, and T-036 rebases onto the new sentences. Either merge order keeps
  both tasks' tests valid: this task changes only the re-open branch, which no T-036 test uses.
- **T-038** and **T-039** (drafted in the same run) both edit `crates/kiln-commands/src/doc.rs`;
  not in flight with either. Proposed order: T-039, then this task, then T-038 (T-038 then
  declares doc.open's refusals as they are after this task; the kinds do not change).
- **T-027** (draft) edits `doc.rs` (`Save`) and `reason.rs`; not in flight together because of
  `doc.rs`.
- `docs/commands.json` and `docs/scorecard.md` are generated: regenerate on conflict.
- D-091 change 16 puts T-036 and T-037 to T-039 in the first stage of iteration 2, so all four
  are candidates for dispatch at the same time; the shared files above decide their order.

## Questions and rulings
- Q-1 (planner, R-154): choose O1, O2, O3 or O4 (Options). Recommended O1; the file is written
  for O1, with the O4 delta stated under the table.
- Q-2 (planner, R-154): under O1, a walk that fails inside the root on a re-open (a link loop, a
  directory that cannot be searched) is refused with `io_error` (AC-4), because the location
  cannot then be shown to be inside. The alternative is to answer `already_open: true` and
  refuse only `outside_root`, which matches "does not require the file to be readable" more
  literally. Recommended: `io_error`, as written.
- Ruling on Q-1 (director, D-092): O1. Naming a path that is already open applies the check of the root and the check of the path's location, and reads nothing; a path that now leads outside the root, or a session whose root was replaced or removed, is refused `outside_root`; a file that was deleted, replaced by a directory or another non-regular file, or made undecodable is still answered `already_open: true`.
- Ruling on Q-2 (director, D-092): `io_error`, as written. A location that cannot be shown to be inside the root is not answered as open.
- Order (director, D-092): after T-036 (shared `disk.rs`, `docs/protocol/paths.md`, the scorecard item file) and after T-039 (shared `doc.rs`); never in flight with either. A pre-dispatch check against main follows T-036's merge (D-091 change 7).

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13T18:29Z · R-154 · (new) · did: drafted from `evaluations/iter-1.md` NEW-1 and D-090
  on main `73a8d13c48cc09e7ac9f238d4848c888341d9fb0`. Read `doc.rs` (`Open`), `vfs.rs`,
  `path.rs`, `disk.rs` (`current_root`, `locate`, `write_atomic_with`, `Vfs` impl),
  `memory.rs`, `paths.md`, T-008 and T-036. Established that "already open" is keyed on the
  path text (code and probe). Probed the debug `kiln-cli` through `control` for nine changes
  after an open (Pinned (g)); every re-open answered `already_open: true` on base. Wrote four
  options with their answers per change; recommended O1 (location check only), which needs one
  new `Vfs` method because `is_file` cannot tell "outside" from "missing". Tier high by
  ADR-0002 (confinement boundary stated in `paths.md`); no gate path, no dependency, no §3
  contract (the `Vfs` trait is an L2 API, not a CHARTER §3 decision). Pinned lines n/a: none.
  Unsure: the O1 answers are INFERRED from the walk's code and the save answers, not executed.
  The FIFO fixture method is left to the test author, as T-008's tests do it. · gate attempts:
  0 · not run: cargo (director's instruction); no new code exists to run · missing context:
  none · next: director answers Q-1 and Q-2 and orders this task against T-036.
  QUESTION: Q-1 (option), Q-2 (`io_error` on a failed walk) — see "Questions and rulings".
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
