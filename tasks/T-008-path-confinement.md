---
id: T-008
title: "Confine every disk path to the workspace root and refuse non-regular files at open and save, without blocking"
tier: high              # persistence/save path; security; L2 + L4 (kiln-workspace, kiln-control)
state: draft
owner_role: planner
scorecard_items: []     # no item covers this; see Q-4
depends_on: [T-004]
touches:
  - crates/kiln-workspace/src
  - crates/kiln-workspace/tests/confinement.rs   # implementer's own integration tests
  - crates/kiln-control/src/lib.rs               # kind mapping for the two new messages + the kinds list
  - docs/control-protocol.md                     # "Paths" section and the error-kinds table
budget_tokens: 700000
iteration: 1            # see Q-5
found_by: reviewer
origin_task: T-004
---

# T-008 Confine every disk path to the workspace root and refuse non-regular files at open and save, without blocking

## Goal
Today a request path that passes through a symlink inside the root can read and replace files
outside it. T-004 review r1 showed both: `doc.open` of a link to `/etc/passwd` succeeded, and a
save through a symlinked directory changed a file outside the root. Also, `doc.open` of a FIFO
hangs the session forever (T-004 r2, R2-SG2). After this task a `DiskFs` never reads, creates,
replaces or deletes anything whose location is outside its root, at open or at save, whatever
symlinks lie on the path or appear between open and save. Opening or saving anything that is
not a regular file (directory, FIFO, socket, device) is refused at once with a structured
error. `docs/control-protocol.md` states the guarantee, and it is true.

## Reading list
- `docs/decisions.md` D-014 (save rulings; Unix only), D-018, D-028 (1), D-031 (13), D-032.
- `reviews/T-004-r1.md` D-1 and SG-1 (the exploit), `reviews/T-004-r2.md` R2-SG2 (FIFO hang).
- `tasks/T-002-documents-file-io.md` Interface (`Vfs`, `VfsError`, `DiskFs`, `RelPath`,
  `Document::open/save`) and its out-of-scope note on symlinks.
- `crates/kiln-workspace/src/{disk,vfs}.rs`; `crates/kiln-control/src/lib.rs` (`rejected_kind`,
  `ERROR_KINDS`); `docs/control-protocol.md` "Paths" and "Error codes and kinds".
- Existing tests that must stay green unchanged: `crates/kiln-workspace/tests/acceptance_t002*.rs`
  (in-root symlink saves, decoy symlinks at temp names), `apps/kiln-cli/tests/acceptance_t004*.rs`.

## Definitions
- **Root.** The directory `DiskFs::new(root)` was given, fully resolved once at construction
  (the root path itself may be a symlink to a directory; its target is the root).
- **Location of a path `p`.** The file-system entry reached by resolving `<root>/p` one component
  at a time and following every symlink, in any component including the last. A relative link
  target is resolved against the link's directory, an absolute one as is. For a final component
  that does not exist, the location is the resolved parent directory plus that name.
- **Inside.** A location is inside the root if it is the root or below it. Only the final
  location decides: a chain of links that passes outside and comes back inside is inside.
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
- `NotFound`, `Io`: unchanged.

`Vfs` trait signatures and `DiskFs::new` are unchanged. `MemoryFs` is unchanged (it has no
symlinks or special files). Behaviour of `DiskFs`:

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
implementer).

Every check is made again on every call: what `doc.open` saw does not authorise a later save.

### Commands and control protocol (through the existing registry)
`doc.open` / `doc.save` report these as `rejected` with the existing message shapes
(`cannot open {VfsError}` / `cannot save {VfsError}`), so exactly:
- `cannot open <p>: resolves outside the root`, `cannot save <p>: resolves outside the root`
  → protocol `{"code":"rejected","kind":"outside_root",…}`;
- `cannot open <p>: not a regular file`, `cannot save <p>: not a regular file` →
  `{"code":"rejected","kind":"not_a_file",…}`.

`outside_root` is a new `(rejected, outside_root)` pair in the specified kind set (D-031 (10),
see Q-1). `not_a_file` keeps its name; its meaning widens to every non-regular file. A failed
`doc.save` leaves the document dirty and the session log unchanged (existing rule). `run`
exits 3 when such a request is refused. `exec doc.open` exits 2 and prints the error object
with that `kind` on stderr. No other exit code changes.

### `docs/control-protocol.md`
- "Paths": replace "Symbolic links are followed, including links that point outside the root …"
  with the confinement rule in the user's terms: lexical refusals (unchanged); a path whose
  location is outside the root is refused (`outside_root`) at open and at save; symlinks that
  stay inside are followed, and a save through one replaces its target and keeps the link;
  directories, FIFOs, sockets and devices are refused (`not_a_file`) without blocking; each
  request is checked again. List the residual limits: a hard link inside the root to a file
  outside cannot be detected; the file system changing *during* one request is not guarded
  against (Q-3).
- Error table: add the `outside_root` row; reword the `not_a_file` "When" cell. Every other row
  is unchanged.

## Acceptance criteria
Every case uses real entries created by the test in a fresh temporary directory `work/` that
holds the root `work/root/` and an outside directory `work/outside/` with
`work/outside/secret.txt` = `b"secret\n"`. **Each escape test first asserts that its outside
target exists and has the expected bytes** (so the test can fail; the T-004 r1 escape tests
could not). "Snapshot" means every entry under `work/`, recursively, symlinks not followed:
type, link target, bytes. Oracle: exact value unless stated otherwise.

- AC-1 (open escapes are refused): with links in the root `rel -> ../outside/secret.txt`,
  `abs -> <absolute path of work/outside/secret.txt>`, `dlink -> ../outside` (a directory) and
  `chain -> rel`: `DiskFs::read` of `rel`, `abs`, `dlink/secret.txt`, `chain` each →
  `Err(OutsideRoot{path})` with that path; `is_file` false for each; `Document::open` →
  `Err(OpenError::Vfs(OutsideRoot{…}))`; the work snapshot is unchanged.
- AC-2 (save escapes are refused and touch nothing): `write_atomic` with `b"PWNED"` to `rel`,
  `abs`, `dlink/secret.txt`, `dlink/new.txt` and to a dangling `gone -> ../outside/none.txt`
  each → `Err(OutsideRoot{path})`; the work snapshot is byte-identical before and after (so
  `secret.txt` is unchanged, `outside/new.txt` and `outside/none.txt` do not exist, and no
  `.kiln-tmp-*` exists anywhere under `work/`).
- AC-3 (the reviewer's exploit through the CLI): `kiln-cli run s.jsonl --root work/root` with
  `doc.open {"path":"dlink/secret.txt"}`, `doc.insert {"doc":1,"offset":0,"text":"PWNED "}`,
  `doc.save {"doc":1}`, `doc.open {"path":"abs"}`, `doc.list {}` → line 1 is
  `rejected`/`outside_root` with message `cannot open dlink/secret.txt: resolves outside the root`;
  lines 2–3 are `rejected`/`no_such_document`; line 4 is `outside_root`; line 5
  `{"id":…,"ok":true,"result":{"docs":[]}}`; exit code 3; no stdout line contains the
  absolute work path or the text `secret`; the work snapshot is unchanged. Running the same
  script on a second copy of `work/` at another absolute path gives byte-identical stdout
  (differential).
- AC-4 (a link that becomes an escape between open and save): (a) open `a.txt` (a regular file
  in the root), insert text, then replace `a.txt` by a symlink to `../outside/secret.txt`;
  `Document::save` (and, through `kiln-cli control`, `doc.save`, where the test changes the
  file system only after the `doc.insert` response arrived) → `OutsideRoot` / `outside_root`
  with message `cannot save a.txt: resolves outside the root`; `secret.txt` unchanged; `a.txt`
  is still that symlink; the document is still dirty (`is_dirty()` / `doc.info` `dirty:true`);
  no `.kiln-tmp-*` anywhere. (b) The same with directory component `sub` of an open `sub/f.txt`
  replaced by a symlink to `../outside` that holds a real `f.txt` → `OutsideRoot`, that outside
  `f.txt` unchanged. (c) The same with `a.txt` replaced by a symlink to `real/t.txt` inside the
  root → save succeeds, `real/t.txt` holds the document's text, `a.txt` is still the symlink.
- AC-5 (symlinks that stay inside keep working): with `real/t.txt` = `b"target\n"`, links
  `in -> real/t.txt`, `din -> real`, `ain -> <absolute path of work/root/real/t.txt>`,
  `in2 -> in`, a dangling `ghost -> real/new.txt`, and also for a `DiskFs` whose root is given
  as `work/rootlink` where `rootlink -> root`: `read` of `in`, `din/t.txt`, `ain`, `in2` gives
  `b"target\n"`; `write_atomic` through each replaces `real/t.txt`'s bytes and every link keeps
  its target; `write_atomic("ghost", b"g")` creates `real/new.txt` = `b"g"` and `ghost` stays a
  link. Every existing `acceptance_t002*` test still passes unchanged.
- AC-6 (non-regular files on open, without blocking): a FIFO `p` (made with the `mkfifo`
  utility, no writer), a Unix socket `s` (bound with `std::os::unix::net::UnixListener`, short
  path), a directory `d` and a link `lp -> p`: `read` of each → `Err(NotAFile{path})` within 5
  seconds (the test runs the call under a timeout so a hang fails it, never blocks the suite);
  `is_file` false. `DiskFs::new("/dev")` then `read("null")` → `Err(NotAFile)` and
  `is_file("null")` false (read-only device case: never write under `/dev`). Through
  `kiln-cli run` with `doc.open p`, `doc.open s`, `doc.open d`, `doc.list`, under a 10-second
  timeout: four response lines, the first three `rejected`/`not_a_file` with messages
  `cannot open p: not a regular file` (and `s`, `d`), the fourth `ok:true`; exit 3.
- AC-7 (non-regular files on save): `write_atomic` onto the FIFO `p`, the socket `s` and the
  directory `d` → `Err(NotAFile{path})`; afterwards `p` is still a FIFO, `s` still a socket,
  `d` still a directory with its content; no `.kiln-tmp-*` left. And: open regular `f.txt`,
  edit, replace `f.txt` by a FIFO, save → `NotAFile`, `f.txt` still a FIFO, document dirty.
- AC-8 (precedence and messages): `read` and `write_atomic` of `gone` (dangling, outside) →
  `OutsideRoot`, of a dangling `ghost2 -> real/none.txt` (inside) via `read` → `NotFound`;
  `doc.open {"path":"../outside/secret.txt"}` still → `invalid_path` (lexical first); the
  `Display` of each `VfsError` returned in AC-1…AC-7 is exactly the string in the Interface and
  contains neither the absolute work path nor any link target.
- AC-9 (contract document): `docs/control-protocol.md` no longer says symlinks are followed out
  of the root; it lists `(rejected, outside_root)` and the widened `not_a_file`; every pair the
  host can produce is listed (the existing `kiln-control` documentation test stays green). The
  reviewer checks the "Paths" text against AC-1…AC-8.

## Scorecard items and intended test names
No item exists for confinement (Q-4). Tests (no item): `crates/kiln-workspace/tests/acceptance_t008.rs`
(`kiln_workspace::acceptance_t008::{open_through_escaping_symlinks_is_refused,
save_through_escaping_symlinks_touches_nothing, link_swapped_in_after_open_is_refused_at_save,
symlinks_inside_root_still_work, non_regular_files_are_refused_without_blocking,
save_onto_non_regular_files_leaves_them_alone, outside_root_wins_over_not_found}`) and
`apps/kiln-cli/tests/acceptance_t008.rs` (`kiln_cli::acceptance_t008::{reviewer_exploit_is_refused_end_to_end,
fifo_and_socket_do_not_hang_run, swap_between_requests_is_refused_through_control}`).

## Out of scope
- Atomic (race-free) confinement against a process that changes the file system *during* a
  single call; it needs `openat`-style system calls and a new dependency (Q-3).
- Hard links (undetectable by path); Windows (D-014 (5)); symlinks in `MemoryFs`.
- Structured reasons in `CommandError` instead of message matching in `kiln-control`: T-007.
  Here `kiln-control`'s existing message mapping only gains the two new message shapes.
- Project tree, find-in-files and any future directory walk. They must apply the same rule when
  they are built; that is for their tasks.
- Any change to `Cargo.toml`, `Cargo.lock`, `xtask/`, `.cargo/`, `.github/`, lint tables or
  build keys. This task touches **no gate path and no dependency**. If std alone cannot meet a
  criterion, stop and report; do not add a crate.

## Implementer notes
- Your tests: unit tests in `crates/kiln-workspace/src`, integration tests in
  `crates/kiln-workspace/tests/confinement.rs`, `kiln-control` unit tests in its `lib.rs`.
- Existing unit tests of `rejected_kind` that pin the old directory message change with it;
  say so in your log (golden rule 4 disclosure). Acceptance and guard files of T-002/T-004 are
  not edited.
- Determinism: no host path or link target in any error text; no timing dependence except the
  tests' hang timeouts.
- Specified: the outcomes, errors, messages and precedence above, and that nothing outside is
  touched. Left to the implementer: how locations are resolved and compared, and how a
  non-regular file is detected without opening it.

## Questions
- Q-1 (planner, R-049): a new kind `outside_root` adds a pair to the set D-031 (10) made
  specified. Approve it, or choose `invalid_path` (it reads as a lexical error) or `io_error`.
- Q-2 (planner, R-049): symlinks whose location stays inside the root are still followed, and
  a save through them replaces the target and keeps the link. Existing T-002 acceptance tests
  and the `io.save-atomic` item rely on this. The alternative is to refuse every symlink.
- Q-3 (planner, R-049): with std only, the check and the use inside one call are not atomic, so
  a concurrent process can still race them. This matches D-026's threat model (honest mistakes,
  untrusted repo contents, no active attacker). Accept it as a documented residual risk, or
  rule that race-free confinement is required: that needs a new dependency (gate path:
  `dependencies`, tier high, an approval row that says so).
- Q-4 (planner, R-049): no scorecard item covers path confinement or non-regular files. I
  checked all 63 items: `io.save-atomic` claims only the in-root symlink save, and `ws.vfs`
  claims only the VFS model. I did not create one. Should there be one (for example
  `ws.path-confinement`, crate `kiln-workspace`, tier high) for the evaluator to confirm?
- Q-5 (planner, R-049): T-004 r1 (SG-1) recommended confinement before the iteration-0
  evaluator runs on the slice. Is this task iteration 0 (evaluator waits for it) or 1?

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
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
