---
id: T-008
title: "Confine every disk path to the workspace root and refuse non-regular files at open and save, without blocking"
tier: high              # ADR-0002: persistence path (DiskFs read/write_atomic) and a confinement/security boundary the protocol document states
state: tests_joined
owner_role: planner
scorecard_items: [ws.path-confinement]   # director-authorized item (Decisions D-4)
depends_on: [T-032]     # D-041: reason enum + generated kind list (T-015 transitively); merged
touches:
  - crates/kiln-workspace/src/vfs.rs           # R-060: narrowed from the directory (VfsError)
  - crates/kiln-workspace/src/disk.rs          # R-060: narrowed (DiskFs confinement)
  - crates/kiln-workspace/tests/confinement.rs   # implementer's own integration tests
  - crates/kiln-workspace/tests/acceptance_t008.rs   # test author
  - crates/kiln-workspace/tests/guard_t008.rs        # test author (AC-5 guard, R-109)
  - apps/kiln-cli/tests/acceptance_t008_cli.rs       # test author
  - crates/kiln-commands/src/reason.rs           # R-060: RejectReason::OutsideRoot + its description + the VfsError mapping row
  - crates/kiln-commands/tests/acceptance_t032.rs    # IMPLEMENTER ONLY: the one `not_a_file` description row (D-061, R-112)
  - apps/kiln-cli/tests/acceptance_t032_cli.rs       # IMPLEMENTER ONLY: the one `not_a_file` description row (D-061, R-112)
  - crates/kiln-commands/tests/reasons.rs            # IMPLEMENTER ONLY: drop the description-equals-table-cell comparison (ruling v3, D-069)
  - docs/control-protocol.md                     # the one false "Paths" bullet only (D-037); no kind lines (R-060)
  # docs/commands.json also changes (error_kinds): a generated path, regenerated, exempt
  - docs/protocol/paths.md                       # NEW: the confinement rule in full (D-6)
budget_tokens: 850000   # high (v1.1)
iteration: 1
found_by: reviewer
origin_task: T-004
---

# T-008 Confine every disk path to the workspace root and refuse non-regular files at open and save, without blocking

## Goal
Today a request path that passes through a symlink inside the root can read and replace files
outside it. T-004 review r1 showed both: `doc.open` of a link to `/etc/passwd` succeeded, and a
save through a symlinked directory changed a file outside the root. Also, `doc.open` of a FIFO
hangs the session forever (T-004 r2, R2-SG2). Both were observed again on current code by the
T-032 reviewer (`reviews/T-032-r1.md`:177–180, handed to this task by D-051): (a) `kiln-cli exec
doc.open '{"path":"fifo"}' --root <root>` on a FIFO blocks until killed; (b) a symlink inside the
root to `/etc/passwd` opens. After this task a `DiskFs` never reads, creates,
replaces or deletes anything whose location is outside its root, at open or at save, whatever
symlinks lie on the path or appear between open and save. Opening or saving anything that is
not a regular file (directory, FIFO, socket, device) is refused at once with a structured
error, through `kiln-cli run`, `kiln-cli exec` and the Rust API alike. `docs/protocol/paths.md`
states the guarantee, and it is true; `docs/control-protocol.md` no longer claims the opposite.
No gate path and no dependency in `touches`.

## Reading list
- `docs/decisions.md` D-014 (save rulings; Unix only), D-018, D-028 (1), D-031 (13), D-032,
  D-051 (r5: the two observed problems), D-052 (ruling r6 and the `not_a_file` description).
- `tasks/spec-checks/T-008.md` (R-101) and the resolutions at the end of this file.
- `reviews/T-004-r1.md` D-1 and SG-1 (the exploit), `reviews/T-004-r2.md` R2-SG2 (FIFO hang),
  `reviews/T-032-r1.md`:170–180 (both reproduced on the T-032 code).
- `tasks/T-002-documents-file-io.md` Interface (`Vfs`, `VfsError`, `DiskFs`, `RelPath`,
  `Document::open/save`) and its out-of-scope note on symlinks.
- `crates/kiln-workspace/src/{disk,vfs}.rs`; `crates/kiln-commands/src/reason.rs` (T-032: the
  `reasons!` table, descriptions, the exhaustive `VfsError` mapping); `docs/control-protocol.md`
  "Paths"; `error_kinds` of the generated `docs/commands.json`.
- Existing tests that must stay green unchanged: `crates/kiln-workspace/tests/acceptance_t002*.rs`
  (in-root symlink saves, decoy symlinks at temp names), `apps/kiln-cli/tests/acceptance_t004*.rs`,
  `apps/kiln-cli/tests/guard_t015_cli.rs`, `apps/kiln-cli/tests/guard_t032_cli.rs`,
  `crates/kiln-commands/tests/{guard_t032,acceptance_t032_json}.rs`,
  `crates/kiln-control/tests/acceptance_t032_control.rs`; and
  `crates/kiln-commands/tests/acceptance_t032.rs`, `apps/kiln-cli/tests/acceptance_t032_cli.rs`
  except their one `not_a_file` row (see "Existing T-032 tests" below).

## Definitions
- **Root.** The directory `DiskFs::new(root)` was given, fully resolved once at construction
  (the root path itself may be a symlink to a directory; its target is the root).
- **Location of a path `p`.** The entry reached by resolving `<root>/p` one component at a time
  as the kernel does: every symlink in any component is followed, and `..` inside a link target
  is applied to the directory actually reached so far (physically, never by lexical
  normalisation). A relative target is resolved against the link's directory. An absolute
  target is resolved from `/` in the same way. If a component does not exist or is not a
  directory before the last one, the location is the part resolved so far plus the remaining
  components.
- **Inside.** A location is inside the root if it is the root or below it, compared component
  by component (a sibling `<root>x/` is not inside `<root>/`). Only the final location decides:
  a chain of links that passes outside and comes back inside is inside.
- **Regular file.** What `std::fs::FileType::is_file` reports after following symlinks.
  Directories, FIFOs, sockets, character and block devices are non-regular.

## Interface

### Rust (`kiln_workspace::…`)
```rust
pub enum VfsError {
    NotFound { path: RelPath },          // unchanged
    /// CHANGED meaning: the location is not a regular file (directory, FIFO, socket, device).
    NotAFile { path: RelPath },
    /// NEW: the location of `path` is outside the root.
    OutsideRoot { path: RelPath },
    Io { path: RelPath, kind: std::io::ErrorKind, message: String },   // unchanged
}
```
`Display` (exact; `{path}` is the `RelPath` as given, never a link target or host path):
- `OutsideRoot` → `{path}: resolves outside the root`
- `NotAFile` → `{path}: not a regular file` (was `{path}: is a directory, not a file`)
- `NotFound`, `Io`: unchanged. `VfsError::path()` returns the path for `OutsideRoot` too.

`Vfs` trait signatures and `DiskFs::new`'s signature are unchanged. `DiskFs::new` returns the
same `Ok`/`Err` values as today for every root (a dangling root symlink is still `NotFound` with
path `(root)`, a root that is or links to a regular file still `Io{kind: NotADirectory}`); only
the stored root is the resolved path. `MemoryFs` behaviour is unchanged (it has no symlinks or
special files); its `NotAFile` message changes with the `Display` above. Behaviour of `DiskFs`:

| Operation | Location outside root | Location not a regular file (existing entry) | Otherwise |
|---|---|---|---|
| `read(p)` | `Err(OutsideRoot{p})`; nothing outside is opened | `Err(NotAFile{p})`, returned without blocking; the entry is not opened for reading | as today |
| `write_atomic(p, …)` | `Err(OutsideRoot{p})`; nothing is created, changed or removed anywhere (no temp file inside or outside) | `Err(NotAFile{p})`; the entry stays as it was (same type, same content); no temp file left | as today: an in-root symlink is followed, its target replaced, the link kept |
| `is_file(p)` | `false` | `false` | `true` iff the location is a regular file |

Precedence, first match wins: `RelPath` lexical refusal (before the VFS, unchanged) →
`OutsideRoot` → `NotFound` → `NotAFile` → `Io`. A dangling link whose target lies outside is
`OutsideRoot`, not `NotFound`, so a caller learns nothing about what exists outside. A dangling
link inside the root is `NotFound` for `read` and creates its target for `write_atomic`, as
today. More than 40 symlink hops is `Io` with message `too many levels of symbolic links`, as
today (whether a loop that first leaves the root reports `OutsideRoot` or `Io` is left to the
implementer). Once resolution has passed outside the root and does not come back, any failure
to resolve further (missing entry, permission, not a directory) is `OutsideRoot`. Inside the
root, a missing or non-directory component is `NotFound`, as today.

Every check is made again on every call: what `doc.open` saw does not authorise a later save.

### Commands and control protocol (through the existing registry)
`doc.open` / `doc.save` report these as `rejected` with the existing message shapes
(`cannot open {VfsError}` / `cannot save {VfsError}`), so exactly:
- `cannot open <p>: resolves outside the root`, `cannot save <p>: resolves outside the root`
  → protocol `{"code":"rejected","kind":"outside_root",…}`;
- `cannot open <p>: not a regular file`, `cannot save <p>: not a regular file` →
  `{"code":"rejected","kind":"not_a_file",…}`.

`outside_root` is a new `(rejected, outside_root)` pair in the specified kind set (D-031 (10);
approved, Decisions D-1). `not_a_file` keeps its name; its meaning widens to every non-regular
file. A failed `doc.save` leaves the document dirty and the session log unchanged (existing
rule). `run` exits 3 when such a request is refused. `exec doc.open` exits 2 and prints the
error object with that `kind` on stderr. No other exit code changes.

### Documentation (Decisions D-6)
- New `docs/protocol/paths.md` states the confinement rule in the user's terms: lexical
  refusals (unchanged); a path whose location is outside the root is refused (`outside_root`)
  at open and at save; symlinks that stay inside are followed, and a save through one replaces
  its target and keeps the link; directories, FIFOs, sockets and devices are refused
  (`not_a_file`) without blocking; each request is checked again. List the residual limits: a
  hard link inside the root to a file outside cannot be detected; a path changed by another
  process *while* a request runs can slip through, and a FIFO swapped in during that window can
  make the request block (accepted limitation, Decisions D-3).
- Error kinds (through T-032's mechanism): add the row
  `OutsideRoot = "outside_root" => "the path resolves outside the root"` with a `///` doc line,
  appended as the last row of the `RejectReason` table in `crates/kiln-commands/src/reason.rs`
  (the row's position fixes the order of `error_kinds`); map `VfsError::OutsideRoot` to it in
  the exhaustive `From<&VfsError>` there; and reword the `NotAFile` row's description to
  exactly "the path names a directory or another non-regular file where a file is needed, or a
  file where a directory is needed" (spec check R-062, finding 23; T-019 uses it for `list_dir`
  of a file). Regenerate `docs/commands.json` (`cargo xtask schema --write`). No table cell of
  `docs/control-protocol.md` is edited (T-032: later tasks do not edit that document's table
  cells; `error_kinds` governs where a cell differs). No list is edited by hand; `kiln-control`
  derives the kind from the reason and is not touched.
- `docs/control-protocol.md`, "Paths": the one bullet beginning "**Symbolic links are followed,
  including links that point outside the root.**" (today 4 lines) is replaced by one bullet:
  "A path whose location is outside the root is refused (`outside_root`); see
  `docs/protocol/paths.md`." Nothing else in "Paths" changes. This goes beyond D-6's "error-kind
  line only" (Q-1, allowed by D-037): left as is, the shared document would state a false
  security property after this task merges.

### Existing T-032 tests (D-052)
Two T-032 acceptance tests compare the `not_a_file` description word for word with today's
text "the path names a directory":
- `crates/kiln-commands/tests/acceptance_t032.rs::reasons_have_pinned_names_order_and_descriptions`
  (the `RejectReason::NotAFile` row of its `REJECTED` table);
- `apps/kiln-cli/tests/acceptance_t032_cli.rs::schema_dump_publishes_the_generated_kind_list`
  (the `("not_a_file", …)` row of its `REJECTED` table).

**T-008's implementer** (D-061) changes exactly the description string of those two rows to
the new text above, on `task/T-008`, in the same commit as the reworded `NotAFile` row, and
nothing else in either file; the change is disclosed in the implementer log as an exact text
change under golden rule 4 (the assertion stays word for word). The implementer's gate is
green before the join, as for any task. The test author does not touch these two files; it
asserts the new description independently in its own file (AC-9,
`documents_state_the_confinement_rule`). Reason (D-061): an exception that makes a red gate
"expected" erodes what the gate means, and the text is pinned by this spec, so who types it
does not affect the tests' independence. The directory cases of
`guard_t032.rs`, `acceptance_t032.rs` and `acceptance_t032_json.rs` no longer pin the
directory message (ruling r6), and `guard_t032_cli.rs` no longer hashes
`docs/control-protocol.md` (r6), so no other T-032 test changes.

## Pinned
- (a) Errors, payloads, precedence: `VfsError::{OutsideRoot, NotAFile}` with exact `Display`;
  precedence lexical → `OutsideRoot` → `NotFound` → `NotAFile` → `Io`; protocol pairs
  `(rejected, outside_root)` (new, approved D-1) and `(rejected, not_a_file)` (widened); message
  shapes `cannot open|save <p>: …`; `run` exit 3, `exec` exit 2 (Interface).
- (b) Files: reads and writes only below the resolved root; temporary files `.kiln-tmp-*` next
  to the target as today (T-002); no file is created on a refused save. Test files:
  `crates/kiln-workspace/tests/acceptance_t008.rs`, `crates/kiln-workspace/tests/guard_t008.rs`
  and `apps/kiln-cli/tests/acceptance_t008_cli.rs` (distinct target names, so the join counts
  and runs them all even before T-009 (6) lands), the two description rows of "Existing T-032
  tests", plus the item file (see Scorecard items).
- (c) Order of checks: per call, location resolution → outside test → existence → file type →
  I/O; re-done on every call (no cache from `doc.open`).
- (d) Path base, platform, file types: `RelPath` relative to the `DiskFs` root as resolved at
  construction; Unix only (D-014 (5)); regular files only, every other type refused; symlinks
  followed inside the root.
- (e) Threat model: an honest user or a hostile repository (links planted in the project) with
  no concurrent attacker: every request whose final location is outside the root is refused,
  whatever links lie on the path or were swapped between requests. Accepted (D-3): a change by
  another process *during* one call (including a FIFO swapped in during the call, which can
  block it); hard links to outside files; `MemoryFs` (no links). The reviewer probes
  adversarially (symlink chains, absolute links, links whose target contains `..` after a
  directory link, sibling directories sharing the root's name as a prefix, dangling links,
  loops, links swapped between requests, long names, FIFOs, sockets, devices).
- (f) Tests pinning a snapshot this changes: the gate's `schema` step on `docs/commands.json`
  (regenerated); the two `not_a_file` description rows named in "Existing T-032 tests" (edited
  by the implementer, D-061). The directory cases of `guard_t032.rs`, `acceptance_t032.rs` and
  `acceptance_t032_json.rs` assert reason and kind only (T-032 r6, VERIFIED on main: the
  `UNPINNED` constant at `guard_t032.rs`:47–49, `acceptance_t032.rs`:58–60,
  `acceptance_t032_json.rs`:50–52); `guard_t032_cli.rs` has no document hash (r6: no
  `protocol_doc_is_unchanged_outside_the_kinds_paragraph` on main). `acceptance_t032_cli.rs::protocol_doc_states_kinds_come_from_reasons`
  reads only the paragraph after the kinds table, not "Paths". `acceptance_t002*.rs`,
  `acceptance_t004*.rs`, `guard_t015_cli.rs` and `guard_t032_cli.rs` stay unchanged and green
  (their sessions name no directory, symlink or special file; spec check finding 4).
- (g) Worked examples: on today's `main` the reviewer's exploit succeeds (evaluation iter-0
  "Hostile use": `out.txt -> ../outside/secret.txt` opened and a save replaced the outside file;
  FIFO `doc.open` hung until `timeout 10`; T-032 r1 again: `exec doc.open` of a FIFO blocks, a
  link to `/etc/passwd` opens), so AC-1…AC-4 and AC-6…AC-8 fail on base by assertion or
  timeout. Resolution examples, checked by the R-101 spec check in a scratch directory: with
  `root/s -> ../outside/d` and `root/t -> s/../secret.txt`, lexical normalisation gives
  `root/secret.txt` but the kernel reads `outside/secret.txt` (so `t` is outside); with a sibling
  `work/rootx/secret.txt` and `root/px -> ../rootx/secret.txt`, a string-prefix test says inside,
  the component-wise rule says outside. Expected outputs are the exact strings in the
  Interface; not executed on new code.
- (h) Scorecard item `ws.path-confinement`: the evaluator runs AC-3's script through
  `kiln-cli run --root <dir>` (refusals `outside_root`, the outside file unchanged) and AC-6's
  FIFO/socket/directory script and its `exec` call under a timeout (`not_a_file`, no hang).

## Acceptance criteria
Every case uses real entries created by the test in a fresh temporary directory `work/` that
holds the root `work/root/` and an outside directory `work/outside/` with
`work/outside/secret.txt` = `b"s3cr3t-content\n"`. **Each escape test first asserts that its
outside target exists and has the expected bytes** (so the test can fail; the T-004 r1 escape
tests could not). "Snapshot" means every entry under `work/`, recursively, symlinks not
followed: type, link target, bytes. Oracle: exact value unless stated otherwise.

In `acceptance_t008.rs`, errors are asserted by `e.to_string()` (exact `Display`) and
`e.path()`, plus the snapshot, never by matching a variant, so the file compiles on base and
fails there by assertion (`OutsideRoot` does not exist on base). Variant matches belong in the
implementer's unit tests. Every AC below except AC-5 is an **acceptance** test (fails on base);
AC-5 is a **guard** (passes on base by design). No test asserts the number or the order of
entries in `error_kinds`, nor the bytes of a whole document (D-043, D-051, D-052).

- AC-1 (open escapes are refused): fixtures also `work/outside/d/` (a directory) and
  `work/rootx/secret.txt` (bytes `b"s3cr3t-content\n"`); links in the root `rel -> ../outside/secret.txt`,
  `abs -> <absolute path of work/outside/secret.txt>`, `dlink -> ../outside` (a directory),
  `chain -> rel`, `s -> ../outside/d`, `t -> s/../secret.txt` and `px -> ../rootx/secret.txt`:
  `DiskFs::read` of `rel`, `abs`, `dlink/secret.txt`, `chain`, `t`, `px` each → an error whose
  `Display` is `<path>: resolves outside the root` and whose `path()` is that path; `is_file`
  false for each; `Document::open` of `rel` → an error whose `Display` is
  `cannot open rel: resolves outside the root`; the work snapshot is unchanged.
- AC-2 (save escapes are refused and touch nothing): `write_atomic` with `b"PWNED"` to `rel`,
  `abs`, `dlink/secret.txt`, `dlink/new.txt`, `t`, `px` and to a dangling
  `gone -> ../outside/none.txt` each → `<path>: resolves outside the root`; the work snapshot is
  byte-identical before and after (so `secret.txt` and `rootx/secret.txt` are unchanged,
  `outside/new.txt` and `outside/none.txt` do not exist, and no `.kiln-tmp-*` exists anywhere
  under `work/`).
- AC-3 (the reviewer's exploit through the CLI): `kiln-cli run s.jsonl --root work/root` with
  `doc.open {"path":"dlink/secret.txt"}`, `doc.insert {"doc":1,"offset":0,"text":"PWNED "}`,
  `doc.save {"doc":1}`, `doc.open {"path":"abs"}`, `doc.list {}` → line 1 is
  `rejected`/`outside_root` with message `cannot open dlink/secret.txt: resolves outside the root`;
  lines 2–3 are `rejected`/`no_such_document`; line 4 is `outside_root` with message
  `cannot open abs: resolves outside the root`; line 5
  `{"id":…,"ok":true,"result":{"docs":[]}}`; exit code 3; no stdout line contains the
  absolute work path or `s3cr3t-content`; the work snapshot is unchanged. Running the same
  script on a second copy of `work/` at another absolute path gives byte-identical stdout
  (differential).
- AC-4 (a link that becomes an escape between open and save): (a) open `a.txt` (a regular file
  in the root), insert text, then replace `a.txt` by a symlink to `../outside/secret.txt`;
  `Document::save` (and, through `kiln-cli control`, `doc.save`, where the test changes the
  file system only after the `doc.insert` response arrived) → `Display`/message
  `cannot save a.txt: resolves outside the root` / kind `outside_root`; `secret.txt` unchanged;
  `a.txt` is still that symlink; the document is still dirty (`is_dirty()` / `doc.info`
  `dirty:true`); no `.kiln-tmp-*` anywhere. (b) The same with directory component `sub` of an
  open `sub/f.txt` replaced by a symlink to `../outside` that holds a real `f.txt` →
  `outside_root`, that outside `f.txt` unchanged. (c) The same with `a.txt` replaced by a
  symlink to `real/t.txt` inside the root, where `real/t.txt` holds exactly the bytes `a.txt`
  had when it was opened → save succeeds, `real/t.txt` holds the document's text, `a.txt` is
  still the symlink. (The equal bytes keep (c) valid after T-027, whose unforced save refuses a
  file that changed on disk since it was read.)
- AC-5 (symlinks that stay inside keep working; a **guard**, `guard_t008.rs::symlinks_inside_root_still_work`):
  with `real/t.txt` = `b"target\n"`, links `in -> real/t.txt`, `din -> real`,
  `ain -> <absolute path of work/root/real/t.txt>`, `in2 -> in`, a dangling
  `ghost -> real/new.txt`, an excursion `back -> ../outside/back2` with
  `work/outside/back2 -> ../root/real/t.txt`, and also for a `DiskFs` whose root is given as
  `work/rootlink` where `rootlink -> root`: `read` of `in`, `din/t.txt`, `ain`, `in2`, `back`
  gives `b"target\n"`; `write_atomic` through each replaces `real/t.txt`'s bytes and every link
  keeps its target; `write_atomic("ghost", b"g")` creates `real/new.txt` = `b"g"` and `ghost`
  stays a link. `DiskFs::new` of a dangling symlink → `NotFound` with path `(root)`, of a
  symlink to a regular file → `Io` with kind `NotADirectory` (as today). Every existing
  `acceptance_t002*` test still passes unchanged.
- AC-6 (non-regular files on open, without blocking; observed problem (a)): a FIFO `p` (made
  with the `mkfifo` utility under a timeout, no writer), a Unix socket `s` (bound with
  `std::os::unix::net::UnixListener` at a short path in the same file system as `work/` —
  `work/` under `std::env::temp_dir()` with a short unique name — and renamed to `root/s` if
  needed; a socket path is limited to 108 bytes), a directory `d` and a link `lp -> p`: `read`
  of each → `<path>: not a regular file` within 5 seconds (the test runs the call under a
  timeout so a hang fails it, never blocks the suite); `is_file` false.
  `DiskFs::new("/dev")` then `read("null")` → `null: not a regular file` and `is_file("null")`
  false (read-only device case: never write under `/dev`). Through `kiln-cli run` with
  `doc.open p`, `doc.open s`, `doc.open d`, `doc.list`, under a 10-second timeout: four
  response lines, the first three `rejected`/`not_a_file` with messages
  `cannot open p: not a regular file` (and `s`, `d`), the fourth `ok:true`; exit 3. And
  `kiln-cli exec doc.open '{"path":"p"}' --root work/root` under a 10-second timeout exits 2
  with stderr exactly
  `{"code":"rejected","kind":"not_a_file","message":"cannot open p: not a regular file"}` and a
  newline, stdout empty (the T-032 r1 reproduction itself).
- AC-7 (non-regular files on save): `write_atomic` onto the FIFO `p`, the socket `s` and the
  directory `d` → `<path>: not a regular file`; afterwards `p` is still a FIFO, `s` still a
  socket, `d` still a directory with its content; no `.kiln-tmp-*` left. And: open regular
  `f.txt`, edit, replace `f.txt` by a FIFO, save → `cannot save f.txt: not a regular file`,
  `f.txt` still a FIFO, document dirty.
- AC-8 (precedence and messages): `read` and `write_atomic` of `gone` (dangling, outside) →
  `resolves outside the root`; `read` of a dangling `ghost2 -> real/none.txt` (inside) →
  `ghost2: not found`; `up -> ..` → `read` `up: resolves outside the root` (not `not a regular
  file`); `self -> .` → `read` `self: not a regular file`; `po -> ../outside/fifo` (a FIFO made
  outside) → `read` `po: resolves outside the root` within 5 seconds; `dn -> /dev/null` →
  `read` `dn: resolves outside the root` and `is_file` false;
  `doc.open {"path":"../outside/secret.txt"}` still → `invalid_path` (lexical first); the
  `Display` of each error returned in AC-1…AC-7 is exactly the string in the Interface and
  contains neither the absolute work path nor the text `outside` or `rootx`.
- AC-9 (contract documents; `kiln_cli::acceptance_t008_cli::documents_state_the_confinement_rule`):
  `kiln-cli commands --schema` has, in `error_kinds.rejected`, an entry `outside_root` with
  `when` "the path resolves outside the root" and the entry `not_a_file` with `when` equal to
  the new description (each looked up by kind, not by position); `docs/control-protocol.md`
  contains the new bullet of the Interface (whitespace-insensitive) and no longer contains
  "Symbolic links are followed, including links that point outside the root"; its other
  "Paths" bullets are present unchanged (each bullet found by its text, not by line number);
  `docs/protocol/paths.md` exists and contains `outside_root` and `not_a_file`. The reviewer
  checks `docs/protocol/paths.md` against AC-1…AC-8.

## Scorecard items and intended test names
Item `ws.path-confinement`. The item file is **not** on `main` (R-051 wrote it in a checkout but
it was never committed; committing it as a ledger commit would make `docs/scorecard.md` stale on
`main`, D-034). The test author creates `scorecard/items/ws.path-confinement.toml` on
`task/T-008-tests` (an allowed path for this task's own item) with `id = "ws.path-confinement"`,
`title = "Disk paths are confined to the workspace root at open and save; non-regular files are refused without blocking"`,
`crate = "kiln-workspace"`, `tier = "high"` and the `tests` below, and regenerates
`docs/scorecard.md` (a generated path) on that branch. Proposed claim: the six
`kiln_workspace::acceptance_t008` tests, the guard
`kiln_workspace::guard_t008::symlinks_inside_root_still_work` (listed separately), and
`kiln_cli::acceptance_t008_cli::{reviewer_exploit_is_refused_end_to_end, fifo_and_socket_do_not_hang_run, fifo_does_not_hang_exec}`.
Tests: `crates/kiln-workspace/tests/acceptance_t008.rs`
(`kiln_workspace::acceptance_t008::{open_through_escaping_symlinks_is_refused,
save_through_escaping_symlinks_touches_nothing, link_swapped_in_after_open_is_refused_at_save,
non_regular_files_are_refused_without_blocking, save_onto_non_regular_files_leaves_them_alone,
outside_root_wins_over_not_found}`), `crates/kiln-workspace/tests/guard_t008.rs`
(`kiln_workspace::guard_t008::symlinks_inside_root_still_work`) and
`apps/kiln-cli/tests/acceptance_t008_cli.rs` (`kiln_cli::acceptance_t008_cli::{reviewer_exploit_is_refused_end_to_end,
fifo_and_socket_do_not_hang_run, fifo_does_not_hang_exec, swap_between_requests_is_refused_through_control,
documents_state_the_confinement_rule}`).

## Out of scope
- Atomic (race-free) confinement against a process that changes the file system *during* a
  single call; it would need `openat`-style system calls and a new dependency, which the
  director declined (Decisions D-3). It is documented as a limitation, including the possible
  block on a FIFO swapped in during the call.
- Hard links (undetectable by path); Windows (D-014 (5)); symlinks in `MemoryFs`.
- Structured reasons and the generated kind list: T-032 (merged). This task only adds the
  `OutsideRoot` row, its mapping row and the reworded `NotAFile` description.
- Project tree, find-in-files and any future directory walk. They must apply the same rule when
  they are built; that is for their tasks. T-019 gives `list_dir` of a file its own message
  (the `Display` `{path}: not a regular file` misreads there; spec check finding 19).
- Any change to `Cargo.toml`, `Cargo.lock`, `xtask/`, `.cargo/`, `.github/`, lint tables or
  build keys. This task touches **no gate path and no dependency**. If std alone cannot meet a
  criterion, stop and report; do not add a crate.

## Implementer notes
- Your tests: unit tests in `crates/kiln-workspace/src`, integration tests in
  `crates/kiln-workspace/tests/confinement.rs`, unit tests of the mapping in `reason.rs`.
  Variant matches (`VfsError::OutsideRoot { .. }`) go here, not in the acceptance file.
- No `kiln-control` test changes: kinds come from reasons (T-032). Acceptance and guard files of
  T-002/T-004/T-015/T-032 are not edited by you, except the two T-032 description rows, which
  are yours (see "Existing T-032 tests", D-061); your gate is green before the join.
- Determinism: no host path or link target in any error text; no timing dependence except the
  tests' hang timeouts (no thread that times out a blocking read: CHARTER §3 decision 4).
- Specified: the outcomes, errors, messages and precedence above, and that nothing outside is
  touched. Left to the implementer: how locations are resolved and compared, and how a
  non-regular file is detected without opening it.
- Where an older planner log entry disagrees with the sections above (for example R-049's
  "`kiln-control` changes to classify messages"), the sections above govern.

## Decisions
Director's answers to the planner's questions of R-049 (2026-11-13, run R-051).
- D-1 (was Q-1): the new kind `outside_root` (`rejected`) is approved.
- D-2 (was Q-2): symlinks whose location resolves inside the root are followed (save replaces
  the target and keeps the link).
- D-3 (was Q-3): the remaining race within one call is accepted and documented as a limitation
  in `docs/protocol/paths.md` and the `DiskFs` module docs; no new dependency.
- D-4 (was Q-4): scorecard item `ws.path-confinement` is created (director-authorized) and this
  task claims it. Because the task now has a scorecard item, the evaluator must confirm it.
- D-5 (was Q-5): no; the iteration-0 evaluator runs against today's `main`. This task is
  iteration 1.
- D-6 (director R-057, all product tasks): a task documents its behaviour in its own new file
  under `docs/protocol/` and touches `docs/control-protocol.md` only to add an error-kind line;
  two tasks that add kinds before T-030 may conflict on rebase in that list, and the director
  resolves it by hand (so that list is not counted as an overlap at dispatch).
- Dispatch note (R-109, replaces R-057's): never in flight with T-010 (`reason.rs`; in progress
  now) or T-020 (`reason.rs`, its size-limit row), nor with T-024 or T-030
  (`docs/control-protocol.md`). T-019 and T-027 depend on it. It may run beside T-007, T-009
  and T-011.
- Director (2026-11-13, D-037): T-008 may replace the symlink bullet in the "Paths" section of `docs/control-protocol.md`, which today states that symlinks are followed; that statement is this task's to correct. `docs/control-protocol.md` is in `touches` for that bullet and for error-kind lines only.
- Director (2026-11-13, D-052): the `not_a_file` description stays pinned word for word in
  T-032's two tests; T-008's spec names them (Interface, "Existing T-032 tests").
- Director (2026-11-13, D-061, reverses R-109's proposal): the implementer, not the test
  author, updates the two rows and keeps its gate green before the join; no "expected red".
  The test author asserts the new description in `acceptance_t008_cli.rs` (AC-9).

## Questions
Director rulings on the test author's two questions (2026-11-13, decision D-068; run R-120), given to both builders; the test author's tests already follow these readings:
- v1 (AC-4, AC-7 texts). `Document::save` returns the `VfsError`, whose text is `<path>: …`; the words `cannot save ` are added by the `doc.save` command. Where AC-4 and AC-7 quote `cannot save a.txt: …` as the text of the Rust API error, read: the command's message is `cannot save ` followed by the API error's text. `document.rs` is not changed for this.
- v2 (AC-8). The director's edit after re-check R-115 forbade the text `outside` in error texts, which the pinned message `resolves outside the root` itself contains. AC-8 reads: apart from the pinned phrase `resolves outside the root`, no error text contains the absolute work path, the fixture directory names `outside` or `rootx`, or a link's target.
- v3 (implementer's blocker, R-122; decision D-069). `crates/kiln-commands/tests/reasons.rs::table_rows_are_published_kinds_with_their_when_text` requires every description to equal its table cell in `docs/control-protocol.md`, while that document says "Where a row's 'When' text differs from `error_kinds`, `error_kinds` governs; later tasks do not edit table cells." The test contradicts the document it reads. No table cell is edited. `reasons.rs` (the T-032 implementer's own test file) joins `touches`: the implementer removes the description-equals-cell comparison, including the `invalid_path` special case, and keeps the checks that every `rejected`/`internal` table row is a published kind with a non-empty description and that the table has its 13 rows. The exact description texts stay pinned by the T-032 acceptance tests. Logged under golden rule 4; the gate is green before the join.
Director rulings after review round 1 (2026-11-13, decision D-072; review `reviews/T-008-r1.md`, run R-128: two defects, one spec gap, one nit; both original problems gone in the reviewer's probes). Checked against Definitions, Pinned (d) and (e) and the Interface error table above before the builders got them:
- v4 (D-1, defect; implementer and test author). By Definitions the root is the directory reached when `DiskFs::new` resolved its argument, and Pinned (e) refuses every request whose final location is outside it, "whatever links lie on the path or were swapped between requests"; D-3 accepts only a change during one call. The code keeps the root as a path and lets the kernel resolve it again on every call, so when the root, or a directory above it, is replaced by a symlink between two requests, files outside the root are opened and saved (reproduced by the reviewer through `control`). This is fixed in code; the guarantee is not narrowed. Observable, and nothing else is pinned: after the root path, or a parent directory of it, has been replaced between two requests of one session by a symlink to another directory, a request whose file then lies outside the root as resolved at construction is refused with the existing `OutsideRoot` (`{path}: resolves outside the root`, kind `outside_root`) at open and at save, and the outside files are byte-identical afterwards. How the root is kept (re-validating it on each call, or holding the directory itself) is the implementer's choice. A root given through a symlink at construction keeps working (AC-1). What a session reports when its root has only been renamed or removed, with nothing put in its place, is not pinned beyond this: nothing outside is read or written. The test author adds acceptance tests for the root swap and the parent-directory swap through the Rust API and through `control`, each asserting first that the outside file is reachable through the swapped path (no vacuous pass); they must fail on the current joined head.
- v5 (D-2, defect; implementer). `docs/protocol/paths.md` must not claim more than the code keeps: "a FIFO never makes a request wait" becomes a statement about a FIFO present when the request is checked, with a pointer to the limits section, and the unqualified sentences about nothing outside being read and no temporary file being left are qualified the same way. "Changes made between two requests are always caught" stays only if v4 makes it true for the root as well.
- v6 (SG-1, spec gap). The sentence "so a caller learns nothing about what exists outside" (Interface, Precedence) is wider than "only the final location decides" allows: a chain that leaves the root and returns shows whether the outside directory it passes through exists. It is read as: a dangling link whose walk ends outside is `OutsideRoot`, not `NotFound`. `docs/protocol/paths.md` states it in that narrower form. No code change.
- v7 (N-1, nit; implementer). A link target that ends in `/` requires a directory, as the kernel does (Definitions: "as the kernel does"); the implementer fixes it with a unit test. Inside the root only; no confinement impact.
- v8 (corrects v4; decision D-074; question of the test author, run R-133). v4 said the mechanism was the implementer's choice, "re-validating it on each call, or holding the directory itself". The second option contradicts v4's own observable ("refused … at open and at save"): a session that held the original directory would, after the root path is replaced, look for files in the moved-aside directory, report `not found` for `secret.txt` and save `f.txt` successfully there. The words "or holding the directory itself" are struck. Pinned: once the path given at construction no longer resolves to the directory recorded at construction (it was replaced, or a directory above it was), every request of that session is refused with `OutsideRoot` (`{path}: resolves outside the root`), at open and at save, including the save of a document opened before the change, and nothing is written anywhere. What happens when the root is only renamed or removed with nothing put in its place stays unpinned beyond: nothing outside is read or written. Read by the director, not yet joined: the test author's tests (`bc1d62c0ac0f6308832e92343aea53706aa55dfc`) assert exactly this, and the implementer's `current_root` (`a2b12ea4f6dc20d0e84b1e77e69a7ec90bcbc145`) re-resolves the stored path on each call and compares device and inode with those recorded at construction.
- v9 (decision D-076; report of the implementer, run R-132). v8's text does not forbid an open handle, but the director's message to the implementer did ("do not introduce a held directory handle"); that instruction is withdrawn. Comparing device and inode identifies a directory only while it exists: when the root directory is removed, its inode number can be given to the next directory created (on the file system the gate runs on a director's probe saw this three times out of three), so a root removed and replaced by a link to a new directory could pass the comparison. Allowed, therefore: the session may hold the root directory open for as long as it lives, solely so that its inode cannot be reused; no path is resolved through that handle, and every request still resolves the path given at construction as v8 pins. Added to the pinned observable of v4 and v8: after the root directory has been removed and its path replaced by a link to another directory (or by a new directory), every request is refused with `OutsideRoot` at open and at save and nothing is written anywhere. The test author adds acceptance tests for this case through the Rust API and through `control`; they assert the refusal whether or not the inode number was in fact reused, and must fail on the bare tests branch by assertion. Known limit, stated in the code: a root that cannot be opened for reading is not held, and reuse is then not excluded; `docs/protocol/paths.md` must say so, and the round-2 reviewer judges whether that is acceptable or construction should refuse such a root.
Director rulings after review round 2 (2026-11-13, decision D-081; review `reviews/T-008-r2.md`, run R-138: the round-1 defect D-1 is resolved in the reviewer's own probes; one defect in the delta, documentation only; four test gaps and one nit as follow-ups). The director read `docs/protocol/paths.md` lines 14–21 and 86–92 and the module documentation of `disk.rs` before ruling:
- v10 (R2-D1, defect; implementer; documentation and comments only). v9 required `docs/protocol/paths.md` to state the limit for a root that cannot be opened for reading; it does not, and three of its sentences are unconditional where the code is not: that the root is kept open, that nothing is read or written in the other directory, and that changes made between two requests are always caught. The document and the `//!` module documentation of `disk.rs` must say what the code keeps: the root is held open only when it can be opened for reading; when it cannot, a root that is removed and replaced by another directory between two requests may go undetected, and the session may then read and write in the replacement directory (the reviewer reproduced this as an unprivileged user with a root of mode 0311: the inode number was reused in six runs of six). The three sentences are qualified by that limit. No code, test or behaviour changes.
- v11 (risk accepted by the director as owner-delegate, for audit). The unreadable-root case is NOT closed in this task. This case is inside the threat model: Pinned (e) covers changes made between two requests, which is why v4 had the readable-root case fixed in code and did not narrow the guarantee. It is accepted here as a known gap for two reasons only: round 2 is final and round 3 may only confirm the fix of a round-2 defect (METHODOLOGY §7), so new code could not be reviewed within this task; and the precondition is unusual (a root the process may traverse but not open for reading). The reviewer adds that refusing such a root at construction would change what `DiskFs::new` is pinned to accept. Until the follow-up task of v12 merges, the guarantee of this task is the documented one (v10), not the unconditional one.
- v12 (follow-ups; a new high-tier task with `origin_task: T-008`, drafted by the planner). (1) Close the unreadable-root case or refuse such a root, with tests run as an unprivileged user. (2) TG-1: the four root-removal tests catch a session that does not hold its root in 10 of 10 runs alone but the API target in only 6 of 10 runs as the gate runs it (in parallel); make them reliable. (3) TG-3: an acceptance test for a root given through a link that is later retargeted (v8 pins the refusal; only a unit test covers it). (4) TG-4: pin and test the refusal while the root path leads nowhere, which paths.md now documents. (5) N-1: a link target ending in `/.` is followed to a file where the kernel refuses. (6) TG-2 (comparing only the inode across devices) is recorded as accepted: it needs a second mount to test.
- v13 (process). Round 3 confirms the documentation fix of v10 and nothing else; there is no round 4.

- Q-1 (planner, R-057): D-6 limits edits of `docs/control-protocol.md` to error-kind lines, but
  its "Paths" section has a bullet saying symlinks are followed out of the root, which becomes a
  false security claim when T-008 merges. Specified: T-008 replaces exactly that bullet by a
  one-line pointer to `docs/protocol/paths.md`. Allow, or should the bullet stay until T-030?
  Answered by D-037 (allowed).

## Resolutions of spec check R-101
`tasks/spec-checks/T-008.md`: 6 blockers, 9 ambiguities, 7 notes. Resolved in R-109.
- 1 (blocker, T-032 tests pin the old directory `Display`): resolved on main by ruling r6 /
  D-052; VERIFIED, the directory cases use `UNPINNED` (`guard_t032.rs`:47–49,
  `acceptance_t032.rs`:58–60, `acceptance_t032_json.rs`:50–52); Pinned (f) says so.
- 2 (blocker, two T-032 tests pin the `not_a_file` description): resolved the other way per
  D-052; the two tests and files are named, in `touches`, the new text is exact, and the edit is
  the implementer's (D-061, reversing R-109's assignment to the test author; Interface,
  "Existing T-032 tests").
- 3 (blocker, document hash guard): resolved on main by r6 / D-052; VERIFIED,
  `protocol_doc_is_unchanged_outside_the_kinds_paragraph` is gone from `guard_t032_cli.rs`.
- 4 (note, no other collision): accepted; restated in Pinned (f) and the reading list.
- 5 (blocker, AC-3 contradicts itself): applied; outside bytes `s3cr3t-content\n`.
- 6 (blocker, AC-5 passes on base): applied; AC-5 is `guard_t008.rs`, six acceptance tests.
- 7 (ambiguity, AC-9 untested): applied as `documents_state_the_confinement_rule`, with kinds
  looked up by name and bullets by text (no count, order or whole-document bytes).
- 8 (ambiguity, socket path length): applied in AC-6.
- 9 (ambiguity, compile on base): applied; acceptance assertions by `Display` and `path()`.
- 10 (blocker, lexical and prefix confinement): applied; `s`, `t`, `px`, `rootx` in AC-1/AC-2,
  `up`, `self`, `po`, `dn` in AC-8; "component by component" added to Definitions.
- 11 (ambiguity, `exec` reproduction): applied in AC-6 and as `fifo_does_not_hang_exec`.
- 12 (ambiguity, location gaps): applied; Definitions replaced, the outside-failure rule added.
- 13 (ambiguity, stat-then-open hang): first option applied (documented limit, D-3).
- 14 (ambiguity, `MemoryFs` message): applied in the Interface.
- 15 (ambiguity, `DiskFs::new` errors): applied in the Interface and the AC-5 guard.
- 16 (note, names exist): re-checked on main; unchanged.
- 17 (note, row position): applied (last row of `RejectReason`, with a `///` line).
- 18 (note, out-and-back chain untested): applied as the `back` case of the AC-5 guard.
- 19 (note, T-019's message): recorded in Out of scope; T-019's own spec is not edited here.
- 20 (note, touches and stale dispatch note): applied; dispatch note rewritten for today's states.
- 21 (note, consistency): accepted; no edit.
- 22 (ambiguity, stale decision text): applied (D-3 text, Q-1 answered, Interface governs).
- Not from R-101 (R-109): AC-4 (c) now makes the in-root target hold the opened bytes, because
  T-027 (draft, depends on T-008) refuses an unforced save of a file that changed on disk and
  would otherwise have to edit this acceptance test.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · tokens (if known) · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13T02:45Z · R-049 · (new) · did: specified confinement by final resolved location
  (open, save, `is_file`), re-checked on every call; new `VfsError::OutsideRoot` and kind
  `outside_root`; `NotAFile` widened to all non-regular files, refused without blocking;
  precedence and exact messages; the protocol-document rewrite. ACs use real outside files
  whose presence each test asserts first, the reviewer's exploit end to end, swaps between open
  and save (Rust API and `control`), FIFO/socket/directory, and `/dev/null` read-only (no
  write under `/dev`; `/dev/zero` avoided, it would exhaust memory on base). Read `disk.rs`:
  base renames the temp file over a FIFO or socket and reads FIFOs blocking, so AC-6/AC-7 fail
  on base by assertion or timeout. `VfsError` matches in `kiln-control` and `kiln-cli` have
  catch-all arms, so the new variant forces no file outside `touches`. `kiln-control` changes
  only to classify the two new messages (T-007 replaces message matching). Tier high:
  save path, security, L2 + L4. No gate path, no dependency. · gate attempts: 0 · not run:
  gate (no code) · missing context: none · next: director answers Q-1…Q-5.
- 2026-11-13T03:05Z · R-051 · draft · did: applied the director's answers as Decisions D-1…D-5;
  wrote `scorecard/items/ws.path-confinement.toml` (tier high, crate kiln-workspace,
  `tests = []`) and claimed it; the race is now a stated limitation. I did not regenerate
  `docs/scorecard.md` (not in this run's paths): a new item makes it stale, so the item should
  reach `main` on a task branch where `docs/scorecard.md` is regenerated (D-012), not as a
  ledger commit. · gate attempts: 0 · missing context: none · next: director review.
- 2026-11-13T03:50Z · R-055 · draft · did: brought the spec to v1.1: tier high under ADR-0002
  (criteria: product code on the persistence path, `DiskFs::read`/`write_atomic`, and a
  confinement boundary that `docs/control-protocol.md` states), budget 850k; added Pinned with
  the threat model; renamed the CLI test file to `acceptance_t008_cli.rs` so the join does not
  meet two `acceptance_t008` targets (T-009 (6) not yet merged); found that
  `scorecard/items/ws.path-confinement.toml` is not on `main` (`ls scorecard/items`) and moved
  its creation to the tests branch with a pinned title. AC-3 and AC-6 are the CLI criteria for
  the item. Arm n/a (high). · gate attempts: 0 · not run: the exploit on new code · missing
  context: none · next: director review.
- 2026-11-13T03:56Z · R-057 · draft · did: applied D-6: the confinement text moves to new
  `docs/protocol/paths.md`; `docs/control-protocol.md` edits limited to the kind lines and the
  one false "Paths" bullet (Q-1); dispatch note rewritten. · gate attempts: 0 · missing
  context: none · next: director answers Q-1.
- 2026-11-13 · R-060 · draft · did: applied the director's contracts-first restructuring: depends on T-015 (not T-004); `outside_root` is added as a `RejectReason` variant with its mapping row in `crates/kiln-commands/src/reason.rs` and the generated kind list is regenerated, so `crates/kiln-control/src/lib.rs` and the kind lines of `docs/control-protocol.md` left `touches`; `crates/kiln-workspace/src` narrowed to `vfs.rs` and `disk.rs` so T-015/T-019/T-022/T-024/T-027 files stay disjoint. Older text that mentions `rejected_kind`, `ERROR_KINDS` or message mapping is superseded by this entry. Shares `reason.rs` with T-010 and T-027: never in flight with them. · gate attempts: 0 · missing context: none · next: director.
- 2026-11-13 · R-066 · draft · did: retargeted to T-032 (D-041): `depends_on` T-015 → T-032 (needs the reason mechanism); the generated list is `error_kinds` in `docs/commands.json`; the `NotAFile` description is widened for T-019 (finding 23). · gate attempts: 0 · missing context: none · next: director.
- 2026-11-13T12:37Z · R-109 · draft · did: applied spec check R-101 against main `e614fee`
  (T-015, T-032, T-016, T-013 merged). Findings 1 and 3 recorded as resolved by r6 / D-052
  after reading the T-032 test files on main (`UNPINNED` constants; no document-hash guard in
  `guard_t032_cli.rs`). Finding 2 resolved per D-052: the two description rows, their files in
  `touches`, the exact new text, assigned to the test author; I added the consequence D-052
  left open (the implementer's pre-join gate is red in exactly those two tests). Applied
  blockers 5, 6, 10, ambiguities 7–9, 11–15, 22 and notes 17, 18, 20 as proposed; finding 13
  by its first option. Added the observed problems (a) FIFO hang through `run`, `exec` and the
  API and (b) an in-root link to an outside file (AC-1 `abs`, AC-3 line 4) to the Goal and ACs.
  One change not from R-101: AC-4 (c) uses equal bytes so T-027 need not edit it. Tier high
  unchanged (persistence path and confinement boundary, ADR-0002). Pinned lines n/a: none. ·
  gate attempts: 0 · not run: no cargo (director's instruction); the new resolution examples
  are R-101's scratch results, not re-run · missing context: none · next: director; ready for
  a re-check.
  QUESTION: D-052 gives the two `not_a_file` description rows to the test author, so the
  implementer's `cargo xtask ci` is red in those two tests until the join. Specified as an
  expected, logged red that does not count toward the three-red stop. Confirm, or let the
  implementer change the two rows instead.
- 2026-11-13T12:47Z · R-112 · draft · did: applied D-061: the two T-032 `not_a_file` description rows are
  the implementer's (touches comments, "Existing T-032 tests", Pinned (f), Implementer notes,
  Decisions, resolution of finding 2); the "expected red" gate is removed and the test author
  asserts the description only in AC-9. Reason: an expected-red exception erodes what the gate
  means, and the spec pins the text, so who types it does not affect independence. R-109's
  QUESTION is answered by D-061. · gate attempts: 0 · not run: no cargo · missing context:
  none · next: director; ready for a re-check.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
