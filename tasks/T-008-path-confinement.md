---
id: T-008
title: "Confine every disk path to the workspace root and refuse non-regular files at open and save, without blocking"
tier: high              # ADR-0002: persistence path (DiskFs read/write_atomic) and a confinement/security boundary the protocol document states
state: draft
owner_role: planner
scorecard_items: [ws.path-confinement]   # director-authorized item (Decisions D-4)
depends_on: [T-032]     # D-041: reason enum + generated kind list (T-015 transitively)
touches:
  - crates/kiln-workspace/src/vfs.rs           # R-060: narrowed from the directory (VfsError)
  - crates/kiln-workspace/src/disk.rs          # R-060: narrowed (DiskFs confinement)
  - crates/kiln-workspace/tests/confinement.rs   # implementer's own integration tests
  - crates/kiln-commands/src/reason.rs           # R-060: RejectReason::OutsideRoot + its description + the VfsError mapping row
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
hangs the session forever (T-004 r2, R2-SG2). After this task a `DiskFs` never reads, creates,
replaces or deletes anything whose location is outside its root, at open or at save, whatever
symlinks lie on the path or appear between open and save. Opening or saving anything that is
not a regular file (directory, FIFO, socket, device) is refused at once with a structured
error. `docs/protocol/paths.md` states the guarantee, and it is true; `docs/control-protocol.md`
no longer claims the opposite.

## Reading list
- `docs/decisions.md` D-014 (save rulings; Unix only), D-018, D-028 (1), D-031 (13), D-032.
- `reviews/T-004-r1.md` D-1 and SG-1 (the exploit), `reviews/T-004-r2.md` R2-SG2 (FIFO hang).
- `tasks/T-002-documents-file-io.md` Interface (`Vfs`, `VfsError`, `DiskFs`, `RelPath`,
  `Document::open/save`) and its out-of-scope note on symlinks.
- `crates/kiln-workspace/src/{disk,vfs}.rs`; `crates/kiln-commands/src/reason.rs` (T-032: reasons,
  descriptions, the exhaustive `VfsError` mapping); `docs/control-protocol.md` "Paths";
  `error_kinds` of the generated `docs/commands.json`.
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

`outside_root` is a new `(rejected, outside_root)` pair in the specified kind set (D-031 (10);
approved, Decisions D-1). `not_a_file` keeps its name; its meaning widens to every non-regular file. A failed
`doc.save` leaves the document dirty and the session log unchanged (existing rule). `run`
exits 3 when such a request is refused. `exec doc.open` exits 2 and prints the error object
with that `kind` on stderr. No other exit code changes.

### Documentation (Decisions D-6)
- New `docs/protocol/paths.md` states the confinement rule in the user's terms: lexical refusals (unchanged); a path whose
  location is outside the root is refused (`outside_root`) at open and at save; symlinks that
  stay inside are followed, and a save through one replaces its target and keeps the link;
  directories, FIFOs, sockets and devices are refused (`not_a_file`) without blocking; each
  request is checked again. List the residual limits: a hard link inside the root to a file
  outside cannot be detected; a path changed by another process *while* a request runs can
  slip through (accepted limitation, Decisions D-3).
- Error kinds (through T-032's mechanism): add `RejectReason::OutsideRoot` (kind
  `outside_root`, description "the path resolves outside the root") in
  `crates/kiln-commands/src/reason.rs`, map `VfsError::OutsideRoot` to it in the exhaustive
  `VfsError` mapping there, and reword `NotAFile`'s description to "the path names a directory
  or another non-regular file where a file is needed, or a file where a directory is needed" (spec check R-062, finding 23; T-019 uses it for `list_dir` of a file). Regenerate `docs/commands.json` (`cargo xtask schema --write`). Also reword the `not_a_file` "When" cell of the kept table in `docs/control-protocol.md` the same way (this task already touches that file). No list is edited by
  hand; `kiln-control` derives the kind from the reason and is not touched.
- `docs/control-protocol.md`, "Paths": the one bullet beginning "**Symbolic links are followed,
  including links that point outside the root.**" is replaced by one bullet: "A path whose
  location is outside the root is refused (`outside_root`); see `docs/protocol/paths.md`."
  Nothing else in "Paths" changes. This goes beyond D-6's "error-kind line only" (Q-1): left as
  is, the shared document would state a false security property after this task merges.

## Pinned
- (a) Errors, payloads, precedence: `VfsError::{OutsideRoot, NotAFile}` with exact `Display`;
  precedence lexical → `OutsideRoot` → `NotFound` → `NotAFile` → `Io`; protocol pairs
  `(rejected, outside_root)` (new, approved D-1) and `(rejected, not_a_file)` (widened); message
  shapes `cannot open|save <p>: …`; `run` exit 3, `exec` exit 2 (Interface).
- (b) Files: reads and writes only below the resolved root; temporary files `.kiln-tmp-*` next
  to the target as today (T-002); no file is created on a refused save. Test files:
  `crates/kiln-workspace/tests/acceptance_t008.rs` and `apps/kiln-cli/tests/acceptance_t008_cli.rs`
  (distinct target names, so the join counts and runs both even before T-009 (6) lands), plus
  the item file (see Scorecard items).
- (c) Order of checks: per call, location resolution → outside test → existence → file type →
  I/O; re-done on every call (no cache from `doc.open`).
- (d) Path base, platform, file types: `RelPath` relative to the `DiskFs` root as resolved at
  construction; Unix only (D-014 (5)); regular files only, every other type refused; symlinks
  followed inside the root.
- (e) Threat model: an honest user or a hostile repository (links planted in the project) with
  no concurrent attacker: every request whose final location is outside the root is refused,
  whatever links lie on the path or were swapped between requests. Accepted (D-3): a change by
  another process *during* one call; hard links to outside files; `MemoryFs` (no links). The
  reviewer probes adversarially (symlink chains, absolute links, dangling links, loops, links
  swapped between requests, long names, FIFOs, sockets, devices).
- (f) Tests pinning a snapshot this changes: the gate's `schema` step on `docs/commands.json` (kept
  green by regenerating it); `acceptance_t002*.rs` and
  `acceptance_t004*.rs` stay unchanged and green (in-root symlinks and decoy temp names). The
  `not_a_file` description and the "Paths" bullet of `docs/control-protocol.md` change by design.
- (g) Worked examples: on today's `main` the reviewer's exploit succeeds (evaluation iter-0
  "Hostile use": `out.txt -> ../outside/secret.txt` opened and a save replaced the outside file;
  FIFO `doc.open` hung until `timeout 10`), so AC-1…AC-3 and AC-6 fail on base by assertion or
  timeout. Expected outputs are the exact strings in the Interface; not executed on new code.
- (h) Scorecard item `ws.path-confinement`: the evaluator runs AC-3's script through
  `kiln-cli run --root <dir>` (refusals `outside_root`, the outside file unchanged) and AC-6's
  FIFO/socket/directory script under a timeout (`not_a_file`, no hang).

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
  of the root and its other "Paths" bullets are unchanged; the generated
  `error_kinds` of the generated `docs/commands.json` lists `outside_root` and the widened `not_a_file`;
  `docs/protocol/paths.md` exists with the rule; the `schema` step stays green. The
  reviewer checks `docs/protocol/paths.md` against AC-1…AC-8.

## Scorecard items and intended test names
Item `ws.path-confinement`. The item file is **not** on `main` (R-051 wrote it in a checkout but
it was never committed; committing it as a ledger commit would make `docs/scorecard.md` stale on
`main`, D-034). The test author creates `scorecard/items/ws.path-confinement.toml` on
`task/T-008-tests` (an allowed path for this task's own item) with `id = "ws.path-confinement"`,
`title = "Disk paths are confined to the workspace root at open and save; non-regular files are refused without blocking"`,
`crate = "kiln-workspace"`, `tier = "high"` and the `tests` below, and regenerates
`docs/scorecard.md` (a generated path) on that branch. Proposed claim: the seven
`kiln_workspace::acceptance_t008` tests below plus `kiln_cli::acceptance_t008_cli::reviewer_exploit_is_refused_end_to_end`
and `kiln_cli::acceptance_t008_cli::fifo_and_socket_do_not_hang_run`. Tests: `crates/kiln-workspace/tests/acceptance_t008.rs`
(`kiln_workspace::acceptance_t008::{open_through_escaping_symlinks_is_refused,
save_through_escaping_symlinks_touches_nothing, link_swapped_in_after_open_is_refused_at_save,
symlinks_inside_root_still_work, non_regular_files_are_refused_without_blocking,
save_onto_non_regular_files_leaves_them_alone, outside_root_wins_over_not_found}`) and
`apps/kiln-cli/tests/acceptance_t008_cli.rs` (`kiln_cli::acceptance_t008_cli::{reviewer_exploit_is_refused_end_to_end,
fifo_and_socket_do_not_hang_run, swap_between_requests_is_refused_through_control}`).

## Out of scope
- Atomic (race-free) confinement against a process that changes the file system *during* a
  single call; it would need `openat`-style system calls and a new dependency, which the
  director declined (Decisions D-3). It is documented as a limitation.
- Hard links (undetectable by path); Windows (D-014 (5)); symlinks in `MemoryFs`.
- Structured reasons and the generated kind list: T-032. This task only adds the
  `OutsideRoot` reason and its mapping row.
- Project tree, find-in-files and any future directory walk. They must apply the same rule when
  they are built; that is for their tasks.
- Any change to `Cargo.toml`, `Cargo.lock`, `xtask/`, `.cargo/`, `.github/`, lint tables or
  build keys. This task touches **no gate path and no dependency**. If std alone cannot meet a
  criterion, stop and report; do not add a crate.

## Implementer notes
- Your tests: unit tests in `crates/kiln-workspace/src`, integration tests in
  `crates/kiln-workspace/tests/confinement.rs`, unit tests of the mapping in `reason.rs`.
- No `kiln-control` test changes: kinds come from reasons (T-032). Acceptance and guard files of T-002/T-004 are
  not edited.
- Determinism: no host path or link target in any error text; no timing dependence except the
  tests' hang timeouts.
- Specified: the outcomes, errors, messages and precedence above, and that nothing outside is
  touched. Left to the implementer: how locations are resolved and compared, and how a
  non-regular file is detected without opening it.

## Decisions
Director's answers to the planner's questions of R-049 (2026-11-13, run R-051).
- D-1 (was Q-1): the new kind `outside_root` (`rejected`) is approved.
- D-2 (was Q-2): symlinks whose location resolves inside the root are followed (save replaces
  the target and keeps the link).
- D-3 (was Q-3): the remaining race within one call is accepted and documented as a limitation
  in `docs/control-protocol.md` and the `DiskFs` docs; no new dependency.
- D-4 (was Q-4): scorecard item `ws.path-confinement` is created (director-authorized) and this
  task claims it. Because the task now has a scorecard item, the evaluator must confirm it.
- D-5 (was Q-5): no; the iteration-0 evaluator runs against today's `main`. This task is
  iteration 1.
- D-6 (director R-057, all product tasks): a task documents its behaviour in its own new file
  under `docs/protocol/` and touches `docs/control-protocol.md` only to add an error-kind line;
  two tasks that add kinds before T-030 may conflict on rebase in that list, and the director
  resolves it by hand (so that list is not counted as an overlap at dispatch).
- Dispatch note (R-057, replaces R-055's): `crates/kiln-control/src/lib.rs` overlaps T-010 and
  T-030, so T-008 is never in flight with them. It may run in parallel with T-006, T-007,
  T-009, T-011, T-013.
- Director (2026-11-13, D-037): T-008 may replace the symlink bullet in the "Paths" section of `docs/control-protocol.md`, which today states that symlinks are followed; that statement is this task's to correct. `docs/control-protocol.md` is in `touches` for that bullet and for error-kind lines only.

## Questions
- Q-1 (planner, R-057): D-6 limits edits of `docs/control-protocol.md` to error-kind lines, but
  its "Paths" section has a bullet saying symlinks are followed out of the root, which becomes a
  false security claim when T-008 merges. Specified: T-008 replaces exactly that bullet by a
  one-line pointer to `docs/protocol/paths.md`. Allow, or should the bullet stay until T-030?

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
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
