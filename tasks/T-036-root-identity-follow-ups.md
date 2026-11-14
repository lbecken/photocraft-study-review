---
id: T-036
title: "Refuse a root Kiln cannot hold open, and pin root removal, a retargeted root link and a vanished root with reliable tests"
tier: high              # ADR-0002: a confinement/security boundary that docs/protocol/paths.md states (the root's identity between requests) and the persistence path (DiskFs construction, read, write_atomic)
state: queued
owner_role: planner
scorecard_items: [ws.path-confinement]   # existing item, strengthened (no new item)
depends_on: [T-008, T-019, T-039]   # all merged: T-008 d4915606f9a789b42b99a1eea053554f1f9dd8a7, T-019 3b1a95d865619dead945cbf1ed857afdfc8fe99e, T-039 c538ef8c28f973fcc55974844fc7ccaf550fb307 (D-092 order)
touches:
  - crates/kiln-workspace/src/disk.rs                          # implementer: DiskFs::new, the `_root_handle` field, module, field and `new` docs (not the walk)
  - crates/kiln-workspace/tests/confinement.rs                 # implementer's own integration tests (exists, T-008)
  - docs/protocol/paths.md                                     # implementer: "The root", "The rule", "Residual limits" (Pinned (i))
  - docs/control-protocol.md                                   # implementer: the `--root` bullet and the `run` and `control` cells of "Exit codes" only (D-092 Q-2; Pinned (i))
  - crates/kiln-workspace/tests/acceptance_t036_workspace.rs   # test author: AC-1 (Rust API)
  - crates/kiln-workspace/tests/guard_t036_workspace.rs        # test author: AC-3, AC-4 (Rust API)
  - crates/kiln-workspace/tests/guard_t036_removal_workspace.rs  # test author: AC-2 (Rust API), a test binary of its own
  - apps/kiln-cli/tests/acceptance_t036_cli.rs                 # test author: AC-1 through kiln-cli; AC-6 (documents)
  - apps/kiln-cli/tests/guard_t036_cli.rs                      # test author: AC-3, AC-4 through control
  - apps/kiln-cli/tests/guard_t036_removal_cli.rs              # test author: AC-2 through control, a test binary of its own
  # scorecard/ and docs/scorecard.md: the director's, at the join (METHODOLOGY §6); see "Scorecard items and intended test names"
budget_tokens: 1100000  # high (v1.2, METHODOLOGY §11)
iteration: 2
found_by: reviewer      # T-008 r2 (R-138): R2-D1's residual, TG-1, TG-3, TG-4 (N-1 moved to T-044, D-111)
origin_task: T-008
---

# T-036 Refuse a root Kiln cannot hold open, and pin root removal, a retargeted root link and a vanished root with reliable tests

## Goal
T-008 merged (`d4915606f9a789b42b99a1eea053554f1f9dd8a7`, D-083) with one accepted gap (ruling
v11, D-081). A root the process cannot open for reading is not held open, so a root that is
removed and replaced by another directory between two requests goes undetected, and the session
reads, lists and writes in the replacement (VERIFIED: `reviews/T-008-r2.md` probe (i), as user
`nobody` with a root of mode 0311, inode number reused 6 of 6; `evaluations/iter-1.md`
`ws.path-confinement`, root mode 0300 without DAC capabilities; this check's probe P2, modes 0311
and 0000, as uid 65534 and as uid 0 without DAC capabilities, where `project.tree` also listed
the replacement). It is documented at `docs/protocol/paths.md`:98–102.

After this task:
1. **Item 1.** `DiskFs::new` refuses a root it cannot hold open, so no session starts on it, and
   every root that construction accepts keeps its identity between requests (D-092 Q-1,
   option (b), and D-111 on Q-4: every root that cannot be opened for reading). `docs/protocol/paths.md` drops the residual limit, and
   `docs/control-protocol.md` says how such a `--root` is refused (D-092 Q-2).
2. **Item 2 (TG-1).** Root-removal detection is tested reliably as the gate runs the tests, in
   two new test binaries. T-008's four tests stay as they are (D-092 Q-3).
3. **Item 3 (TG-3).** A root given through a link that is later retargeted has guard tests through
   the Rust API and `control`.
4. **Item 4 (TG-4).** The refusal while the root path leads nowhere is pinned and tested.

Items 2–4 also cover `project.tree` / `Vfs::list_dir`, which T-019 added inside the boundary.
T-008 ruling v12 item (5), N-1 (a link target ending in `/.`), moved to T-044 (D-111). TG-2 is recorded as accepted (Out of scope).

No gate path and no dependency in `touches`. If std alone cannot meet a criterion, stop and
report; do not add a crate.

Reading list: `tasks/T-008-path-confinement.md` Definitions, Interface (error table,
precedence), Pinned (d) and (e), `## Questions` rulings v4–v13; `docs/decisions.md` D-074,
D-076, D-081, D-083, D-092; `reviews/T-008-r2.md` (R2-D1, TG-1, TG-3, TG-4, probes (a)–(j),
mutants M1, M4, M5) and `reviews/T-008-r3.md`; `reviews/T-019-r1.md` "Boundary probes" (Root,
Unreadable); `crates/kiln-workspace/src/disk.rs` at main
`d162413e14236111c9bb6d7c5503d29c1dd1cb87` (code identical to T-019's merge `3b1a95d`): module
docs :23–34, `DiskFs` fields :97–115, `new` :156–192, `current_root` :194–210, `locate` :212–216,
`locate_steps` :218–323, `list_dir` :467–517 (the root located under the label `(root)`
:468–473); `crates/kiln-workspace/src/vfs.rs` (`Vfs::list_dir` :104–110); `docs/protocol/paths.md`
(whole); `docs/protocol/project.md` "Guarantees and limits"; `docs/control-protocol.md` "`kiln-cli`"
and "Exit codes" (as T-039 left them); `apps/kiln-cli/src/main.rs` (`bad_root` :195–196,
`root_error` :356–370, exec :412–414, `host_for` :449–452, `run_script` :463–471,
`control_main` :612–623); `crates/kiln-control/src/lib.rs` (`on_disk` :93–96, `root_error`
:162–176); `crates/kiln-workspace/tests/acceptance_t008.rs` and
`apps/kiln-cli/tests/acceptance_t008_cli.rs` (fixtures, snapshots, the `control` driver with
per-response timeouts, the root-removal tests :753–842 and :669–754); `xtask/src/noskip.rs`:1–11.

## Item 1 as ruled (D-092 Q-1: option (b); D-111 on Q-4: option (A))
Construction refuses a root it cannot hold open, with the pinned error below; every root it
accepts is held open for the life of the session and so keeps its identity. The roots refused
are every root that cannot be opened for reading, a mode-0000 root included (D-111). The
draft's alternatives (keep and protect such a root, accept it and refuse every request, keep
the documented gap) are not specified further.

## Definitions
- **Root, location, inside, regular file**: as T-008 Definitions and `docs/protocol/paths.md`.
- **A root Kiln cannot hold** (D-111): at construction, the root (resolved as T-008
  says) is a directory that the calling process cannot open for reading. With mode bits, this is
  a directory on which the process's identity has no read permission, whatever its search
  permission: for its owner, modes 0311, 0300, 0100 and 0000 among others.
- **The root path leads nowhere**: resolving the root path as given fails, because the root or
  a directory above it was renamed or removed and nothing was put in its place.
- **Unprivileged identity U** (tests only): if the test process's effective uid is 0, U is uid
  65534 with gid 65534. Otherwise U is the test process's own identity. The effective uid is
  found without a new dependency: it is the owner of a file the test just created, as
  `disk.rs`:776–777 already does. A child spawned with `CommandExt::uid(65534)` and
  `gid(65534)` from uid 0 has no supplementary groups and no effective capabilities (VERIFIED,
  probe P5).

## Interface
`Vfs`, `VfsError`, `DirListing` and every public signature are unchanged. No new kind and no
`reason.rs` row. `kiln-control` and `kiln-cli` code are unchanged. Their existing `root_error`
arm for other `VfsError`s (`crates/kiln-control/src/lib.rs`:163–176,
`apps/kiln-cli/src/main.rs`:357–369) already prints an `Io` root error, and `bad_root`
(`main.rs`:195–196) exits 2 with nothing on stdout.

1. **Construction (item 1).** `DiskFs::new(root)`, in this order:
   - If `fs::metadata(root)` fails, the result is as on base: `NotFound` (path `(root)`) for a
     missing root, otherwise `Io { path: (root), kind: <the error's kind>, .. }`. For example a
     root below a directory the process cannot search gives `PermissionDenied`.
   - If the root is not a directory, the result is as on base: `Io { path: (root), kind:
     NotADirectory, .. }`.
   - **New:** if the root is a directory Kiln cannot hold, the result is `Err(VfsError::Io {
     path: (root), kind: PermissionDenied, message })` when the open fails for lack of
     permission, and the open's own error kind otherwise (for example when the process is out of
     file descriptors). `message` is not pinned. It names no host path. Nothing is created or
     changed anywhere.
   - **Every `DiskFs` that `new` returns holds its root directory open** for as long as it
     lives, and takes the root's identity (device, inode) from the directory it holds.
   - A dangling root link and a link to a regular file keep T-008's errors. Every root not named
     above gets the same `Ok`/`Err` as on base. D-092 Q-1 lifts T-008's Interface pin ("returns
     the same `Ok`/`Err` values as today for every root") for the roots Kiln cannot hold.
   - Through `kiln-cli`, `exec`, `run` and `control` with such a `--root`:
     - exit 2, with nothing on stdout;
     - no request is executed: for `run` the script is not read, for `control` stdin is not read
       (the host is built first, `main.rs`:463–471, :618–621, :412–414);
     - stderr is the existing `bad --root` message naming the root as given. Its form is
       documented (Pinned (i) C1), but no test pins its text.
   - **A root held at start whose mode changes later** keeps its identity and its protection
     (Interface 2; tested by AC-2 (iii)). Nothing else is pinned about the requests of such a
     session, and no document states what they answer. An implementer who wants to write such
     a sentence first runs the case (for example `chmod 0311` after start, then `doc.open` of a
     readable file and `project.tree {}`) and records the command and output in its log
     (METHODOLOGY §6).
2. **Root removed and replaced (item 2; T-008 v9 behaviour, unchanged).** This holds for every
   session that construction accepts. Suppose the root directory is removed between two
   requests and its path is replaced by a link to another directory, or by a new directory.
   Then:
   - `read`, `write_atomic` and `list_dir(Some(p))` of any path give `OutsideRoot` with that
     path; `is_file` is false;
   - `list_dir(None)` gives `OutsideRoot` with path `(root)` (`vfs.rs`:109);
   - through `control`, `doc.open`, `doc.save` (including the save of a document opened before)
     and `project.tree` with or without `path` give `rejected` / `outside_root`, and the earlier
     document stays dirty;
   - nothing is read, listed or written in the replacement or anywhere else.

   This holds whether or not the replacement got the removed root's inode number.
3. **Root given through a link that is retargeted (item 3; T-008 v8 behaviour, unchanged).**
   Take a `DiskFs` or session whose root was given as `rootlink`, a symlink to a directory.
   After `rootlink` is pointed at another directory between two requests, every request is
   refused as in Interface 2, and nothing is read, listed or written in either directory. After
   `rootlink` is pointed back at the original directory, requests work there again.
4. **The root path leads nowhere (item 4; newly pinned, base behaviour: T-008 r2 probe (h),
   T-019 r1 "Root renamed away").** This applies while the root, or a directory above it, has
   been renamed away or removed with nothing in its place. Every request is refused as in
   Interface 2: `read`, `write_atomic`, `list_dir` and the save of a document opened before give
   `OutsideRoot` / `outside_root`; `is_file` is false; the document stays dirty. Nothing is
   created anywhere: not the root path, not a file in the renamed-away directory, and no
   temporary file. Once the root path leads to the root directory again (renamed back),
   requests work there again.

### Documentation
The implementer copies the sentences of Pinned (i) exactly, makes the removals listed there,
and changes nothing else in `docs/protocol/paths.md` or `docs/control-protocol.md`. Line numbers
refer to main `d162413`. The `//!` module docs of `disk.rs` (:24–30), the `_root_handle` field
docs (:108–114) and the doc comment of `new` (:157, "Err if root is not a directory") say the
same as the documents: the root is always held open, and a root that cannot be held is refused.
`docs/protocol/project.md` is not edited (Pinned (f)).

## Pinned
- (a) **Errors, payloads, precedence.** No new variant or kind. A root Kiln cannot hold gives
  `Io { kind: PermissionDenied, path: (root) }` (or the open's own kind, Interface 1), after
  `NotFound` and `NotADirectory`. Interfaces 2–4 give `OutsideRoot` before anything else, as
  T-008 (c). `list_dir(None)` names `(root)`. CLI: exit 2, with nothing on stdout, for a refused
  root; refused requests give `run` exit 3 and `exec` exit 2, as T-008.
- (b) **Files.** As T-008 (b). No file is created on a refused construction or a refused request.
  Test files are those in `touches`. All six stems are distinct across the workspace (checked
  against every `tests/` directory at `d162413`: no `*_t036*` file exists). AC-2's tests are in
  two binaries of their own (`guard_t036_removal_*`).
- (c) **Order of checks.** Construction: `fs::metadata` (`NotFound`, other errors), then
  directory (`NotADirectory`), then canonicalisation, then the open that holds the root (this
  refusal). Per call, T-008 (c) applies unchanged: root identity, then location, then outside,
  then existence, then type, then I/O.
- (d) **Path base, platform, file types.** As T-008 (d), Unix only. The unprivileged tests rely
  on POSIX permission semantics for the owner bits of a directory: mode 0311 means search
  without read, and 0000 means neither. The fixture `work/` must lie in a directory U can
  traverse, and U must be able to execute the binary under test. On this machine,
  `std::env::temp_dir()` is `/tmp` (mode 1777, `TMPDIR` unset), and `/`, `/home`, `/home/user`,
  the checkout, `.worktrees/` and `target/` are all 0755 (VERIFIED, `stat`). CI runs as a
  non-root user (`.github/workflows/ci.yml`:18, `ubuntu-24.04`), so U is the test's own
  identity there.
- (e) **Threat model**: see "Threat model" below.
- (f) **Tests and documents that pin what this task changes.** None found.
  - T-008: the four root-removal tests stay unchanged and green (`acceptance_t008.rs`:753–842,
    `acceptance_t008_cli.rs`:669–754). `guard_t008.rs` pins `DiskFs::new` errors only for a
    dangling link and a link to a file. `acceptance_t002.rs`:1040–1042 asserts `is_err` only.
    `acceptance_t004_r2.rs::bad_root_error_names_the_path` and `main.rs`
    `bad_root_message_names_the_root` use missing roots and file roots only.
    `acceptance_t008_cli.rs::documents_state_the_confinement_rule` reads the "Paths" bullets of
    `docs/control-protocol.md`, which this task does not edit.
  - T-019: no T-019 test or guard creates a directory without read permission or changes a
    directory's mode. `grep -n 'from_mode\|set_permissions\|uid'` finds nothing in
    `acceptance_t019_workspace.rs`, `tests/project.rs`, `acceptance_t019_cli.rs` or
    `guard_t019_cli.rs`. Its root-replacement tests (`acceptance_t019_workspace.rs`:300–318,
    `tests/project.rs`:183–190) assert `OutsideRoot` with `(root)` for readable roots, which
    stays true. T-019's review found the unreadable root out of its scope ("belongs to T-036",
    `reviews/T-019-r1.md`:219–220).
  - Documents read by tests:
    - `acceptance_t039_cli.rs::protocol_synopsis_matches_help` compares only the synopsis block,
      which is unchanged.
    - `acceptance_t032_cli.rs` `table_rows` (:146–161) and `kiln-commands/tests/reasons.rs`
      (:209–239) parse only the error-kinds table; the exit-code table is not parsed.
    - `acceptance_t004_r3.rs` `documented_kinds` collects a superset of inline-code tokens; new
      tokens only enlarge it.
  - No existing test creates a root its own user cannot read. `grep -rn 'from_mode\|set_permissions'`
    over every crate, app and xtask finds only files and 0755 directories. So the new refusal
    cannot break a test on CI's non-root identity.
  - `docs/protocol/project.md`: its sentence that "nothing outside the root is listed or read"
    (:72–74) is untrue on base for an unreadable root that is removed and replaced (probe P2:
    `project.tree` listed the replacement). It becomes true with this task without an edit. Its
    sentence that a directory "that cannot be read … makes the whole command fail with
    `io_error`" (:110–111) stays true for subdirectories (VERIFIED, `reviews/T-019-r1.md`:154–159).
    For a root made unreadable after start it is INFERRED, not probed (the same `read_dir`
    failure as P1's 0311 root); the reviewer runs that case. The case it covered for a root
    unreadable at start can no longer arise. This task changes no sentence of `project.md`.
- (g) **Worked examples and measurements**, run by this planner (R-181) on the debug `kiln-cli`
  built from main `d162413` in a private clone. Scripts are in the planner's scratch directory;
  fixtures are under it or under one fixed `/tmp/t036-r181/` directory, which was removed
  afterwards. "capsh" means uid 0 under `capsh --drop=cap_dac_override,cap_dac_read_search`
  (checked: `CapEff 000001fffefffff9`) on a root-owned fixture. "65534" means `setpriv
  --reuid=65534 --regid=65534 --clear-groups` (checked: `Groups:` empty, `CapEff 0`) on a
  fixture chowned to 65534. Both gave the same results.
  - **P1, base, root modes.** Fixture: `root/f.txt` 0644, `exec doc.open f.txt`, `exec
    project.tree {}`, `run doc.list`.
    - 0711: all ok.
    - 0311, 0300, 0100: session starts; `doc.open` ok; `project.tree` → `io_error` ("cannot
      list the project: (root): I/O error …"); `run` exit 0.
    - 0000: session starts; `doc.open` → `io_error` (Permission denied), exit 2 with a JSON error
      object on stderr; `run` exit 0.
    - 0644 (read, no search): `doc.open` → `io_error`; `project.tree` lists `f.txt`.
    - On base every one of these roots is accepted.
  - **P2, base, remove and replace through `control`.** Steps: `doc.open f.txt`, `doc.insert`,
    then `rm -r root`; then make up to 64 new directories until one has the old (dev, ino); then
    `root -> cN`, or `mv cN root`; then `secret.txt` in it.
    - 0711: the number was not reused (handle held). `doc.open secret.txt`, `doc.save`, and
      `project.tree {}` (`… (root): resolves outside the root`) all gave `outside_root`; nothing
      written.
    - 0311 (link and dir): the number was reused at once. `doc.open secret.txt` ok, `doc.save`
      ok (`f.txt` written into the replacement), `project.tree` listed the replacement.
    - 0000 (link and dir): reused. `doc.open secret.txt` ok and `project.tree` ok, both in the
      replacement. The 0000 case is the reason for Q-4 (ruled (A), D-111).
  - P3 (kernel and Kiln behaviour of `/.` link targets) moved to T-044 with item 5.
  - **P4, inode reuse under `/tmp`** (`inode_reuse.py`, fixture as T-008's, root removed with
    `rmtree`, then directories made one at a time in its parent):
    - Not held, quiet: the first new directory got the number in 500 of 500 trials (machine load
      average 4.6–8.7: other agents were running).
    - Not held, with a thread creating and removing directories in another `/tmp` directory:
      79/100 at the first new directory, 83/100 within 64.
    - Not held, with the same churn in the same directory: 64/100 at the first, 96/100 within 64.
    - Held open (as the head will be): 0/50 within 64 and 0/50 within 1,000. The cost was
      7.2 ms per trial for 64 (fixture included).
  - **P5, std's `CommandExt::uid(65534).gid(65534)` from uid 0** (rustc 1.97.0, the pinned
    toolchain). A parent started with groups 10, 20, 27 gave a child with `Uid 65534`,
    `Gid 65534`, `Groups:` empty, `CapEff 0000000000000000`.
  - **P6, bad roots on base.** `exec doc.list {}`, `run s.jsonl` and `control` with `--root
    nope` each gave exit 2, 0 bytes on stdout, and one stderr line `kiln-cli: bad --root: root
    `nope`: not found`, with no usage text. `control --root filelink` gave `…: not a directory`.
    The current `--root` sentence calls this a "usage error", which by the document's own
    definition prints the usage text (`docs/control-protocol.md`:166–168, `main.rs`:86–89).
  - Not executed: anything on new code. The refusal of 0000 roots, the
    held-handle identity and the `bad --root` text for `PermissionDenied` are INFERRED from
    `root_error` (`main.rs`:357–369) and `VfsError`'s `Display` (`vfs.rs`:54), which would give
    ``root `<r>`: I/O error: Permission denied (os error 13)`` if the message is the OS error's
    text.
- (h) **Scorecard item `ws.path-confinement`, the evaluator's calls.**
  - As an unprivileged user, or as uid 0 under `capsh --drop=cap_dac_override,cap_dac_read_search`
    on a root-owned directory (the iteration-1 evaluator's method), run `kiln-cli exec doc.open
    '{"path":"f.txt"}' --root <dir>` with `<dir>` of mode 0311, and again of mode 0000
    (D-111). Each gives exit 2, empty stdout, and stderr naming `<dir>`. With the directory at
    mode 0711 it succeeds.
  - Then run a `kiln-cli control --root rootlink` session, retarget `rootlink` after
    `doc.insert`, and get `outside_root` at open, at save and for `project.tree`, with the work
    tree unchanged.
- (i) **Sentences this task adds to, changes in or removes from `docs/`**, worded for the
  implementer to copy.

  `docs/protocol/paths.md`, "The root":
  - **S1 (added after :13, ending the first paragraph):** "A root that Kiln cannot open for
    reading is refused too, whatever its other permissions (for example a directory whose owner
    may search it but not list it, mode `0311`, or one of mode `0000`): Kiln keeps the root open
    while the session runs (below), and it does not start a session on a root it cannot keep
    open. `kiln-cli` then exits 2 with a `bad --root` message on stderr, and no request runs."
    Evidence: AC-1 (fails on base). Probe P1 shows these roots are accepted on base. Probe P6
    and `main.rs`:195–196, :412–414, :449–452, :463–471 show a refused root gives exit 2, an
    empty stdout and no request.
  - **R1 (:15–17), replaced by S2.** Removed: "When the root can be opened for reading, Kiln
    keeps it open while the session runs, only so that no other directory can take its
    identity; when it cannot (see Residual limits), it is not kept open."
    **S2:** "Kiln keeps it open while the session runs, only so that no other directory can take
    its identity." Evidence: Interface 1 (every accepted root is held), AC-1 and AC-2.
  - **R2 (:22–23), removed:** ", except in the case of an unreadable root described under
    Residual limits". The sentence then ends "… nothing is read or written in the other
    directory." Evidence: AC-2 (including case (iii)) and AC-3.
  - **S3 (added right after that sentence):** "`project.tree` is refused with `outside_root` as
    well." Evidence: probe P2 (0711), `reviews/T-019-r1.md`:125–128, and the `project.tree`
    steps of AC-2, AC-3 and AC-4.
  - Kept unchanged, and newly pinned by AC-4: "The same refusal is given while the root path
    leads nowhere (the root renamed or removed with nothing in its place)." Also kept verbatim,
    because T-037 adds a clause to it: "the session refuses every path with `outside_root`, at
    open and at save, including the save of a document opened before".

  `docs/protocol/paths.md`, "The rule":
  - **R3 (:53–55), removed:** ", or an unreadable root is replaced between requests". The
    sentence then begins "Unless the file system is changed while the request runs (see
    Residual limits), nothing outside the root is read, created, replaced or removed, and no
    temporary file is left anywhere." Evidence: AC-2 and AC-3.
  - **R4 (:73–74), removed:** " (for a root that cannot be opened for reading, see Residual
    limits)". The bullet then ends "… the save is refused, and the document stays modified."
    Evidence: AC-2 (the earlier document's save).

  `docs/protocol/paths.md`, "Residual limits":
  - **R5 (:96–97), removed:** ", except in the case of an unreadable root below". The sentence
    becomes "Changes made between two requests, to the path or to the root and the directories
    above it, are always caught." Evidence: AC-2 to AC-4 and Interface 1. Both device and inode
    are compared (`disk.rs`:205), so a replacement on another device is caught too (TG-2 is a
    test gap, not a behaviour gap).
  - **R6 (:98–102), removed whole:** "- **A root that cannot be opened for reading.** When the
    process may traverse the root but has no read permission on it, Kiln cannot keep it open. A
    root that is then removed and replaced by another directory between two requests may go
    undetected (the new directory can get the removed root's identity), and the session may
    then read and write in the replacement directory."

  `docs/control-protocol.md`, "`kiln-cli`" (as T-039 left it):
  - **C1 (:171–172).** Removed: "A root that does not exist or is not a directory is a usage
    error whose message names the path." Replaced by: "A root that does not exist, is not a
    directory, or cannot be opened for reading (see `docs/protocol/paths.md`, The root) is
    refused before any request runs: a `kiln-cli: bad --root:` message naming the root as given
    is written on stderr, nothing on stdout, and the exit code is 2; the usage text is not
    printed." Evidence: probe P6 for the missing and not-a-directory roots (one stderr line, 0
    stdout bytes, exit 2, no usage text, for `exec`, `run` and `control`); AC-1 for the third
    case. The old sentence's "usage error" was already untrue of the binary.

  `docs/control-protocol.md`, "Exit codes":
  - **C2 (`run` row, column 2).** "root not a directory" is replaced by "bad root (see
    `--root`)". The cell becomes "usage error, unreadable script, bad root (see `--root`), bad
    `--memory-seed` manifest, log file not writable (message on stderr, nothing on stdout)".
  - **C3 (`control` row, column 2).** "usage error, root not a directory" becomes "usage
    error, bad root (see `--root`)".
  - The `exec` row already says "bad root" and is unchanged.
  - Evidence: probe P6 (a missing root already exits 2 for `run` and `control`, so the old cells
    were incomplete) and AC-1.

## Threat model
Kept from T-008 (e): an honest user or a hostile repository, with no concurrent attacker inside
one call. Added here: another process, under the session's user or another one, removes,
renames or replaces the root or a directory above it between two requests.
- **Covered.**
  - Every root that construction accepts keeps its identity between requests: removal and
    replacement by a link or a new directory, a retargeted root link, an ancestor swap (T-008),
    and a vanished root. In all of these nothing is read, listed or written outside the root
    directory.
  - A root Kiln cannot hold is refused before any request runs. The refusal rests on the open
    itself, so it also covers roots made unreadable by an ACL or an LSM policy (not
    tested).
  - A root that was held at start stays protected if its mode changes later.
- **Not covered (accepted, documented).**
  - Changes during one call (D-3 of T-008).
  - Hard links.
  - A root swapped or made unreadable while `DiskFs::new` itself runs (the same class as D-3).
  - Network file systems and macOS (not tested).
- **Reviewer probes adversarially:**
  - an unreadable root through `exec`, `run` and `control`, at modes 0311, 0300, 0100, 0000 and
    0644;
  - a root unreadable only through its group or other bits while the owner differs;
  - a root reached through a link whose target cannot be read;
  - a root made unreadable after start, then removed and replaced;
  - a root below an ancestor that cannot be searched;
  - repeated remove-and-replace cycles;
  - `project.tree` in each of these states;
  - an ACL denial if the machine allows it.

## Acceptance criteria
**Acceptance-test protocol (high tier, METHODOLOGY §6).** An independent test author writes
every `acceptance_t036*` and `guard_t036*` file in `touches`, on `task/T-036-tests`, from this
file alone. The implementer adds none of them and does not edit `scorecard/`. The test author
lists the final test names in its log, and the director ties them to the item at the join.
Before handing over, the test author runs its tests against a naive stub and against at least 5
seeded bugs, including the mutant M of AC-2, and lists the kills in its log.

**Fixtures.** As in T-008: a fresh `work/` per test, holding `work/root/`, `work/outside/` and
`work/outside/secret.txt` = `b"s3cr3t-content\n"`. Each test first asserts through `std::fs`
that the files a refusal must protect are reachable through the changed path, so it cannot pass
vacuously. "Snapshot" means every entry under `work/`, recursively, links not followed: type,
link target and bytes.

**How errors are asserted.** In the Rust API, by variant (`matches!`, every variant exists on
base) and by `path()`. Through the CLI, by `ok`, `code`, `kind`, exit code and empty stdout. No
test pins a message text: each `message`, `Display` and stderr text is checked only to name no
part of the host work path and, where stated, to name the root as given. No test asserts a count
of `error_kinds`, the command list or a whole document.

**Processes and time.** Every child process (`kiln-cli`, or the test's own binary) has a hard
timeout of 10 s, after which it is killed and the test fails. Every `control` response is awaited
for at most 10 s. These timeouts only detect hangs: no test asserts a duration. FIFOs, sockets
and devices are not used.

**No skip.** Where the identity differs (uid 0 or not), both branches run the whole scenario. No
test function has a `return` inside a conditional or loop (`xtask/src/noskip.rs`:5–7). A
child-run test is written as `if <child> { … } else { … }`, not with an early `return`.

Base = main `d162413` (T-008 and T-019 merged). **Acceptance** = fails on base; **guard** =
passes on base and must keep passing.

- **AC-1 (item 1; acceptance; `acceptance_t036_cli.rs::unreadable_root_is_refused_at_start`
  and `acceptance_t036_workspace.rs::unreadable_root_is_refused_by_new`).**
  - *How the process under test becomes unprivileged.* The gate runs as uid 0, which bypasses
    the permission checks. The process under test is therefore always a child run as U, on a
    fixture handed to U:
    - If the test's effective uid is 0, the fixture's directories and files are given to U with
      `std::os::unix::fs::chown`, and the child is spawned with
      `std::os::unix::process::CommandExt::{uid, gid}`.
    - Otherwise the child runs as the test's own identity, and no chown is needed.
    - `work/` lies in `std::env::temp_dir()` with mode 0755 (Pinned (d)).
    - `run`'s script is a file in `work/` owned by U. Do not use stdin or `/dev/stdin`: as 65534
      the pipe is not readable (probe P1).
  - *Controls, run first in the same test, as the same identity.* They make the case
    non-vacuous on every machine. If a control fails, the test fails with a message that names
    the precondition.
    - Positive control: a readable root `work/open` (mode 0711) holding `f.txt` (0644). As U,
      `exec doc.open '{"path":"f.txt"}' --root work/open` succeeds. This shows that U can run
      the binary and reach the fixture.
    - Negative control: `work/open/locked.txt` (mode 0000, owned by U). As U, `exec doc.open
      '{"path":"locked.txt"}' --root work/open` fails (exit 2, empty stdout). This shows that U
      cannot override permission checks.
  - *Cases.* `work/root` holds `f.txt` (0644, readable by U) and is owned by U, with mode (a)
    0311 and (b) 0000 (D-111). For each mode, as U:
    - `kiln-cli exec doc.open '{"path":"f.txt"}' --root work/root` exits 2, stdout is empty, and
      stderr names `work/root` and is not a JSON object. A refused request would print a JSON
      error object.
    - `kiln-cli run work/s.jsonl --root work/root`, with the one request `doc.open f.txt`, exits
      2 with empty stdout.
    - `kiln-cli control --root work/root`, with that one request on stdin, exits 2 and writes no
      line on stdout.
    - The `work/` snapshot is unchanged.
    - On base: (a) succeeds (exit 0, a response); (b) gives `exec` exit 2 with a JSON error
      object, `run` exit 3 and `control` exit 0 with a response. So the test fails on base by
      assertion in every case.
  - *Rust API variant.* The in-process API cannot run as U under a uid-0 gate. The test
    therefore re-runs its own test binary (`std::env::current_exe()`, `--exact <its name>`, an
    environment variable marking the child run) as U, with the same controls and timeout.
    - In the child run it asserts, for each mode, `DiskFs::new(work/root)` →
      `Err(VfsError::Io { kind: ErrorKind::PermissionDenied, .. })` with `path()` `(root)` and a
      `Display` containing no part of the work path. It also asserts that
      `DiskFs::new(work/open)` is `Ok`.
    - The parent asserts that the child run passed and that it actually ran those assertions,
      for example through a marker file the child writes after them.
  - Oracle: the exact values above.
- **AC-2 (item 2, TG-1; guards with a measured detection requirement;
  `guard_t036_removal_workspace.rs` and `guard_t036_removal_cli.rs`, each a test binary of its
  own).**
  - *Cases.*
    - Through the Rust API (one `DiskFs` and one `Document` kept across the change) and through
      `control` (one child, the change made after the `doc.insert` response):
      - (i) the root removed and replaced by a link to a new directory;
      - (ii) the root removed and replaced by a new directory at the same path.
    - (iii) Through `control` only, as U with AC-1's identity rules and controls: the root has
      mode 0711 when the session starts, then `chmod 0311`, then (i). This shows that a root
      readable at start stays protected after its mode changes.
  - *Expected in every trial of every case.*
    - Through `std::fs`, `secret.txt` in the replacement is readable through the root path.
    - Then, Rust API:
      - `read("secret.txt")` → `OutsideRoot` with that path;
      - `is_file("secret.txt")` → false;
      - `write_atomic("new.txt", …)` → `OutsideRoot`;
      - `list_dir(None)` → `OutsideRoot` with path `(root)`;
      - `Document::save` of the earlier document → `OutsideRoot`, and the document stays dirty.
    - Through `control`:
      - `doc.open secret.txt`, `doc.save` of document 1 and `project.tree {}` → `rejected` /
        `outside_root`;
      - `doc.info` of document 1 → `dirty: true`.
    - The replacement's snapshot and the `work/` snapshot are unchanged, and no `f.txt`, no
      `new.txt` and no `.kiln-tmp-*` appear in the replacement.
  - *Making detection reliable, required.*
    - **Isolation.** Every test in each of the two files holds one file-level lock for its whole
      body (a `static Mutex<()>`, poison-tolerant: `lock().unwrap_or_else(|e| e.into_inner())`).
      No two remove-and-replace windows of one binary then overlap. The other test binaries of
      `cargo test --workspace` run before or after this one, not beside it.
    - **Trials.** Each test runs up to K ≥ 5 trials (the test author chooses K and records it).
      Each trial uses a fresh `work/` and a new session.
    - **Reuse search.** In each trial, after the removal, the test creates directories
      `work/c0`, `work/c1`, … one at a time. It stops when one has the removed root's (device,
      inode), or when B ≥ 64 exist (the test author chooses B and records it). The replacement
      is that directory, or the last one. In case (i) the root path becomes a link to it; in
      case (ii) it is renamed to the root path.
    - **Early stop.** A trial whose replacement got the removed root's number ends the loop with
      `break`, never `return`. Otherwise the next trial runs, up to K. On the head the handle is
      held, so the number is never free and all K trials run: probe P4 measured 0/50 within 64
      and 0/50 within 1,000, at 7.2 ms per trial of 64.
    - **Report.** Each trial writes one line to stderr: the case, the trial number, the old and
      new (device, inode), and whether the number was reused. Every assertion message includes
      that line.
    - **No branch on reuse.** The refusals are asserted in every trial, and no branch passes
      without asserting. On the head they hold whatever the file system does, because the
      session holds its root. So no assertion depends on inode reuse, and the gate cannot flake
      on it.
  - *Detection requirement, measured by the reviewer.*
    - M is the head with the root no longer kept: construction still opens the root and still
      refuses one it cannot open, but drops the handle before returning (T-008 r2 mutant M1 in
      the new code).
    - **Head.** Both targets pass in 20 of 20 runs, run as the gate runs them: `cargo test -p
      kiln-workspace --test guard_t036_removal_workspace` and `cargo test -p kiln-cli --test
      guard_t036_removal_cli`, with the default test threads.
    - **Against M, per run.** 20 runs of each target, with `-- --nocapture`. A run in which any
      trial reports reuse and the test still passes is a test gap.
    - **Against M, per test.** Each test fails in at least 19 of the 20 runs. A run that passes
      must show "reused: no" for each of its K trials. The reviewer records each such run with
      the load average and the other processes then running.
    - **Against M, full suite.** One `cargo test --workspace --locked` run fails in these tests.
    - The reviewer records the counts, the host file system and anything else running on the
      machine.
    - The test author records the same measurement against a local mutant (10 runs per target)
      in its log before handing over.
    - Expected from probe P4 (INFERRED, assuming independent trials): at the worst measured
      miss rate of 17% per trial, the chance that all 5 trials miss is about 1.4·10⁻⁴. Base
      figures (T-008 r2, one trial per test, run in parallel): 6 of 10 runs for the API target.
- **AC-3 (item 3, TG-3; guard; `guard_t036_workspace.rs::retargeted_root_link_is_refused` and
  `guard_t036_cli.rs::retargeted_root_link_is_refused_through_control`).**
  - *Fixture.* `work/rootlink -> root`, with `root/f.txt`. `work/other/` holds `secret.txt` and
    its own `f.txt`. The session is given `work/rootlink`.
  - *Steps.*
    1. Open `f.txt` and edit it.
    2. Point `rootlink` at `other`, and assert through `std::fs` that `rootlink/secret.txt` is
       readable.
    3. Expect:
       - `read` / `doc.open` of `secret.txt` → `OutsideRoot` / `outside_root`;
       - `is_file` → false;
       - `write_atomic("new.txt")` → `OutsideRoot`;
       - `list_dir(None)` → `OutsideRoot` with `(root)`, and `project.tree {}` → `outside_root`;
       - the save of the earlier document → `OutsideRoot` / `outside_root`, still dirty;
       - the `work/` snapshot is unchanged: `root/f.txt` and `other/f.txt` keep their bytes, and
         there is no `new.txt` and no `.kiln-tmp-*`.
    4. Point `rootlink` back at `root`. The save of the earlier document now succeeds and writes
       `root/f.txt`. The bytes on disk still equal those read at open, so this holds after T-027
       too.
  - It kills T-008 r2 mutant M4 (the canonical path cached as `given`, now `disk.rs`:180) at
    acceptance level; the reviewer checks this.
- **AC-4 (item 4, TG-4; guard; `guard_t036_workspace.rs::vanished_root_is_refused` and
  `guard_t036_cli.rs::vanished_root_is_refused_through_control`).**
  - *Cases*, with nothing put in place in any of them:
    - (i) `root` renamed to `root.away`;
    - (ii) the root at `work/p/root`, with `p` renamed to `p.away`;
    - (iii) `root` removed with `remove_dir_all`.
  - *After the change:*
    - `read` / `doc.open` of `f.txt` and of `missing.txt` → `OutsideRoot` / `outside_root`;
    - `is_file("f.txt")` → false;
    - `write_atomic("new.txt", …)` → `OutsideRoot`;
    - `list_dir(None)` → `OutsideRoot` with `(root)`, and `project.tree {}` → `outside_root`;
    - the save of the document opened before → `OutsideRoot` / `outside_root`, still dirty
      (`doc.info` `dirty: true`);
    - the `work/` snapshot is unchanged: no `work/root` created, the renamed-away `f.txt` keeps
      its bytes, and there is no `new.txt` and no `.kiln-tmp-*` anywhere.
  - *Moved back.* Cases (i) and (ii) are then renamed back, and the save succeeds into
    `root/f.txt`.
  - It kills mutant M5 (a failed re-resolution taken as success, now at `disk.rs`:203) at
    acceptance level.
- **AC-5**: moved to T-044 with item 5 (D-111).
- **AC-6 (documents; acceptance, fails on base;
  `acceptance_t036_cli.rs::documents_drop_the_unreadable_root_limit`).** Each check is made on
  the text with whitespace squashed.
  - `docs/protocol/paths.md` contains none of these texts, all present on base: "Kiln cannot
    keep it open" (R6), "except in the case of an unreadable root" (R2, R5) and "an unreadable
    root is replaced between requests" (R3).
  - `docs/control-protocol.md` contains neither "is a usage error whose message names the path"
    (C1) nor "root not a directory" (C2, C3), both present on base.
  - The test checks absence only. The reviewer checks the added sentences S1–S3 and C1–C3
    sentence by sentence against the binary.

## Scorecard items and intended test names
| Item | Tests (intended; the test author lists the final names in its log) |
|---|---|
| ws.path-confinement | `kiln_cli::acceptance_t036_cli::unreadable_root_is_refused_at_start` (AC-1), `kiln_workspace::acceptance_t036_workspace::unreadable_root_is_refused_by_new` (AC-1, Rust API), `kiln_cli::guard_t036_removal_cli::root_removed_and_replaced_by_link_is_refused_through_control`, `kiln_cli::guard_t036_removal_cli::root_removed_and_replaced_by_new_directory_is_refused_through_control`, `kiln_cli::guard_t036_removal_cli::root_made_unreadable_after_start_stays_protected_through_control` (AC-2), `kiln_workspace::guard_t036_removal_workspace::root_removed_and_replaced_by_link_is_refused`, `kiln_workspace::guard_t036_removal_workspace::root_removed_and_replaced_by_new_directory_is_refused` (AC-2, Rust API), `kiln_cli::guard_t036_cli::retargeted_root_link_is_refused_through_control`, `kiln_cli::guard_t036_cli::vanished_root_is_refused_through_control` (AC-3, AC-4), `kiln_workspace::guard_t036_workspace::retargeted_root_link_is_refused`, `kiln_workspace::guard_t036_workspace::vanished_root_is_refused` (AC-3, AC-4, Rust API) |

AC-6 (documents) is not tied to the item. The director ties the tests at the join and
regenerates `docs/scorecard.md` (METHODOLOGY §6); the item's status is derived, and the
evaluator confirms it again.

## Out of scope and implementer notes
- **TG-2 (accepted, no test).** The root identity compares device and inode, but no test
  replaces the root by a directory on another device that has the same inode number. Building
  that needs a second mount (a bind mount or another file system), which the gate cannot create
  without privileges and tools it does not have. T-008 r2 mutant M3 (inode-only comparison)
  therefore survives. It is recorded here as accepted, with low practical risk per the reviewer.
  The implementer must not weaken the comparison.
- **T-008 ruling v12 (5), N-1 (`/.` link targets)**: moved to T-044 (D-111). The walk
  (`locate_steps`) is not changed by this task.
- Races during one call (D-3); hard links; Windows; `MemoryFs`.
- **A root with read but no search permission** (mode 0644 directory). It can be opened, so it
  is held and accepted as today. Its `doc.open` requests fail with `io_error`, while
  `project.tree` lists it (probe P1). This is unchanged and not pinned.
- `docs/protocol/project.md` (Pinned (f)). The `kiln-control` and `kiln-cli` sources: their
  existing `root_error` arm prints the new refusal.
- T-008's acceptance and guard files are not edited or removed. Its four probabilistic
  root-removal tests stay as they are (D-092 Q-3).
- Specified: the outcomes above. Left to the implementer: how the root is kept (std offers an
  open directory handle). Tests do not pin `/proc` paths, file-descriptor details or inode
  numbers as assertions. Inode numbers are reported only (AC-2).
- Implementer's own tests: unit tests in `disk.rs` and integration tests in `confinement.rs`.
  Variant matches on new detail belong there. Unprivileged cases there follow AC-1's identity
  rules (a child run as U). Determinism: no clock or thread timing in `DiskFs`, and no new
  dependency.

## Item 5
Item 5 (link targets ending in `/.`, T-008 r2 N-1) moved to `tasks/T-044-dot-in-link-targets.md` (D-111).

## Conflicts and dispatch
- **T-019** (merged `3b1a95d`) and **T-039** (merged `c538ef8`) come first by D-092. This file is
  checked against both (Pinned (f)).
- **T-037** (draft; `disk.rs`, `paths.md`, `scorecard/items/ws.path-confinement.toml`) runs
  after this task and is never in flight with it.
  - S2 keeps verbatim the phrase T-037 extends ("including the save of a document opened
    before").
  - T-037's Threat model names "the residual limits T-036 is about (an unreadable root)"; its
    pre-dispatch check after this merge removes that.
  - Neither task's tests use the other's branch.
- **T-038** (draft; edits the paragraph after the kinds table of `docs/control-protocol.md`):
  same file, different paragraph. Never in flight together (§5.2). T-038 runs after the feature
  tasks (D-092).
- **T-024** and **T-030** (drafts outside the first stage of iteration 2) also edit
  `docs/control-protocol.md` (T-024: one `--tools` line in the `kiln-cli` section). They are not
  dispatched beside this task.
- **T-027** (draft) shares no file in `touches`. Its unforced-save check returns `OutsideRoot`
  unchanged, and AC-3 and AC-4 keep the on-disk bytes equal to those read, so neither order of
  merge breaks the other.
- **T-044** (item 5, draft) shares `disk.rs`, `paths.md` and `confinement.rs`. It runs after
  this task, in either order with T-037, never in flight with either.
- `vfs.rs` and `reason.rs` are not touched.
- T-033 (draft) changes how the join keys test targets. The six stems here are unique either
  way.

## Questions and rulings
- Q-1 (planner, R-145): choose item 1's option, (a), (b), (c) or (d). Recommended: (b). The
  spec above is written for (b). Under (a), Interface 1 and AC-1 change as stated in "Item 1".
- Q-2 (planner, R-145): `docs/control-protocol.md` says that a root which does not exist or
  is not a directory is a usage error, and the exit-code table's 2 column names "root not a
  directory" for `run` and `control`. Under (b) an unreadable root also exits 2. The sentences
  become incomplete but not false. Editing them would make this task share that document with
  T-024 and T-030. Proposed: leave it, and let `paths.md` state the refusal. Or allow one
  `--root` bullet edit.
- Q-3 (planner, R-145): T-008's four root-removal tests stay probabilistic. AC-2 adds
  reliable versions in their own binaries instead of editing merged acceptance files. Should
  the director prefer the T-008 tests themselves be made reliable, `acceptance_t008.rs` and
  `acceptance_t008_cli.rs` join `touches` for the test author only. Then the edit must only
  add serialisation or the reuse search, never loosen an assertion (golden rule 4).
- Ruling on Q-1 (director, D-092): option (b). A root the session may search but not read is refused at construction as the spec states; every root that construction accepts is protected.
- Ruling on Q-2 (director, D-092): the task edits the `--root` sentences and the exit-code table of `docs/control-protocol.md` so that they are complete for an unreadable root (D-091 change 18: a task that changes the CLI surface updates the CLI sections it changes). `docs/control-protocol.md` joins `touches`. T-039 edits the same sections and merges first.
- Ruling on Q-3 (director, D-092): as proposed. T-008's four root-removal tests are left as they are; the reliable versions are new tests in their own binaries.
- Order (director, D-092): after T-019 (shared `disk.rs`) and after T-039; before T-037. The planner brings the file to iteration 2 and to the v1.2 ceiling when it applies these rulings; a pre-dispatch check follows (D-091 change 7).
- Ruling on Q-4 of the spec check R-181 (director, D-111): (A). Construction refuses every root that cannot be opened for reading, a mode-0000 root included; any other failure to open the root is refused with its own error kind. The ruling on Q-1 in D-092 named only roots that can be searched but not read, which left the mode-0000 root accepted and unprotected; the spec check showed that by a probe (such a root was removed and replaced and the session read, listed and wrote in the replacement).
- Split (director, D-111): yes. This task keeps items 1 to 4 (the root's identity between requests). Item 5 (link targets ending in `/.`) becomes its own task, T-044, filed by the planner from the section "Split-off item 5"; it runs after this task and after T-037 and is never in flight with either (shared `disk.rs`, `docs/protocol/paths.md`, `confinement.rs`).
- Ruling on AC-6 (director, D-114; AGENTS.md golden rule 9). AC-6 is not a test: a test that asserts that a document does not contain given words pins document text that T-037, T-044 and later tasks edit, and would fail any later sentence that legitimately uses those words. The implementer removes and replaces the sentences exactly as Pinned (i) lists them (R1 to R6, S1 to S3, C1 to C3), and the reviewer checks on the diff that each listed removal was made and runs each added sentence against the binary. `acceptance_t036_cli.rs` therefore holds AC-1 only, and the test `documents_drop_the_unreadable_root_limit` is not written.
- Ruling on the test author's question of R-190 (director, D-117; answered to both builders). AC-4 asks, through `control`, for `doc.open f.txt` to be refused after the root has vanished, but `f.txt` is already open as document 1 at that point, and `doc.open` of a path that is already open answers `already_open: true` without looking at the disk. That behaviour is the evaluator's finding NEW-1 and belongs to T-037, whose spec changes it; `kiln-commands` is outside this task's `touches`. The test author's reading is confirmed: in AC-4 the `control` test opens a file of the root that is not already open (and a missing one), and the Rust-API test keeps `read("f.txt")`. The implementer has nothing to change for this.
### Rulings on review round 1 (R-192, `reviews/T-036-r1.md`; director, D-119). The implementer acknowledges each before acting.
- p1 (finding 1, test gap). The implementer's test `root_that_cannot_be_opened_for_reading_is_refused_by_new` in `crates/kiln-workspace/tests/confinement.rs` must leave nothing behind when it fails: a guard that restores the modes and removes its fixture on every exit path, and the child is waited for after it is killed. The reviewer's run against mutants left one directory in `/tmp` per failing run. No assertion changes.
- p2 (finding 2, nit). The sentences that say a bad root exits 2 with a `bad --root` message (S1 in `docs/protocol/paths.md`, C1 in `docs/control-protocol.md`) are true except that `exec` checks its params before the root, an order `docs/protocol/cli-memory-seed.md` already states. C1 gains a short pointer to that order; the implementer runs the case first (`exec` with params that are not JSON and an unreadable root) and writes only what it saw. S1 is left as it is.
- p3 (finding 3, nit). No change: that `new` does not re-check that the handle it holds is a directory could matter only if the root were swapped while `new` runs, which the threat model of this spec excludes, and later requests are refused in that case. Recorded as accepted.

### Spec check R-181 (planner, 2026-11-13, against main `d162413e14236111c9bb6d7c5503d29c1dd1cb87`; code identical to T-019's merge `3b1a95d`)
The check is recorded here, as the director asked, rather than in `tasks/spec-checks/`.

**Applied.**
- D-092 Q-1 (b): Item 1, Interface 1 and AC-1 now read as decided; the option table is gone.
- Q-2: `docs/control-protocol.md` joins `touches`, with sentences C1–C3 pinned.
- Q-3: T-008's tests stay; AC-2 has two binaries of its own.
- Order: `depends_on` gains T-019 and T-039 (both merged).
- v1.2:
  - budget 1,100,000;
  - `scorecard/` out of `touches`, replaced by a table of intended names;
  - every added, changed and removed `docs/` sentence written out with its evidence (Pinned
    (i));
  - no message, document, command-list or `error_kinds` pins;
  - no time assertions;
  - the independent-test-author protocol stated.

**Blockers found in the R-145 text** (each fixed above, or turned into Q-4):
1. Interface 1 and Definitions refused only "search but not read" roots and kept "every other
   root" as on base. Roots of mode 0000 or 0200, and roots whose open fails for another reason,
   were then accepted and not held. P2 shows a 0000 root removed and replaced: the session read,
   listed and wrote in the replacement. This contradicts the second half of the D-092 ruling.
   The draft's Documentation also allowed a residual limit for "other failures to open"
   → Q-4, spec written for (A).
2. Message texts were pinned throughout (`cannot save …: resolves outside the root`, `dot: not
   found`, the `bad --root` prefix, a `Display` "beginning with `(root): `"), against v1.2 and
   golden rule 9. Fixed: variant and `path()`, `ok`/`code`/`kind`, exit codes, and negative
   host-path checks.
3. AC-2's "20 of 20 detections against M" with one trial per test cannot be met on a loaded
   machine. P4: 83/100 within 64 directories with churn in another `/tmp` directory. Fixed: K ≥
   5 fresh-session trials, a reuse search of B ≥ 64 directories, a per-trial report, and a
   requirement split into a deterministic part (any trial with reuse must fail) and a rate part
   (≥ 19/20, with passing runs explained by their reports). The gate never depends on reuse.
4. D-092 Q-2 was not applied: `docs/control-protocol.md` was outside `touches` and its
   sentences unpinned. The sentence to edit also calls a bad root a "usage error", which the
   binary contradicts: no usage text is printed (P6). Fixed by C1–C3.
5. The sentences to add and remove in `paths.md` were described, not worded, and the removals
   were given by stale line numbers. Fixed by Pinned (i): S1–S3 and R1–R6 with exact text and
   evidence.
6. `scorecard/items/ws.path-confinement.toml` was in `touches` for the test author (v1.1).
   Removed; the director ties the tests at the join.
7. `budget_tokens: 850000` (v1.1). Now 1,100,000.
8. References stale after T-019 would have misled a builder:
   - `disk.rs`:155–160/:181 (now `new` :162–192, `metadata` :170, `absolute` :180) and :262–263
     (trailing slash, now :277–280);
   - the euid trick :707–709 (now :776–777);
   - `locate` (now a one-line wrapper of `locate_steps`, :214–216; `current_root` is called from
     `locate_steps` :225 and so also guards `list_dir`, :472);
   - `paths.md` :97–101/:14–16/:20–22/:52–55/:69–73/:95–96 (now :98–102, :15–17, :22–23,
     :53–55, :72–74, :96–97).
   All updated.
9. Items 2–4 ignored `Vfs::list_dir` / `project.tree`, which T-019 added inside the boundary.
   `paths.md` says "every path", and `project.md`:72–74 says nothing outside is "listed or
   read", which is untrue on base for an unreadable root that is replaced (P2). Fixed:
   `list_dir(None)` → `(root)`, and `project.tree` in AC-2 to AC-4 (guards; T-019 r1's
   root-replaced probes show the base behaviour), with S3 stating it. `project.md` needs no
   edit (Pinned (f)).
10. Interface 1 said a root readable at start "keeps working" after its mode changes. That is
    wider than the binary: `project.tree` of a root later made 0311 is `io_error` (INFERRED from
    P1). R-186: no longer stated; Interface 1 asks for a recorded probe before any document says it.
11. Item 5 missed the `.`-after-a-missing-entry case: a save through `real/new.txt/.` creates
    `real/new.txt` on base, where the kernel refuses (P3, code reading). It also missed that
    `project.tree {"path":"dot"}` changes from `not_a_file` to `not_found`. Fixed in the
    split-off text, now T-044.
12. Five items bundled in one high-tier spec, against D-091. Item 5 is split off (see that
    section).

**Notes (would not stop a builder, fixed or recorded).**
- No-skip lint: AC-1's child run must not `return` inside a conditional (`noskip.rs`:5–7);
  stated.
- `run`'s script must be a file owned by U, because stdin as `/dev/stdin` is unreadable to 65534
  (P1 artifact); stated.
- The draft's out-of-scope note on 0644 roots said requests fail with `io_error`. `project.tree`
  in fact lists such a root (P1); corrected.
- std's `CommandExt::uid` dropping groups and capabilities was INFERRED in R-145. It is now
  VERIFIED (P5).
- T-019's tests and guards pin nothing this task changes. T-019's spec kept its tests off the
  unreadable root, and this is verified by `grep` (Pinned (f)).
- T-037 must drop its "residual limits T-036 is about" bullet after this merges (Conflicts).
- The check found no other drafted task whose tests read the `kiln-cli` section or the exit-code
  table of `docs/control-protocol.md` beyond the synopsis block.

**Split: yes, two tasks** (items 1–4 here; item 5 split off), ruled in D-111; item 5 is now
T-044, which carries the reasons and the cost of each choice.

**Verdict (R-181):** ready to dispatch once Q-4 is ruled and the split filed. Both were done in
D-111 and applied in R-186.

- Q-4 (planner, R-181): which roots does construction refuse?
  - *Why it is open.* D-092 Q-1 says "a root the session may search but not read is refused at
    construction …; every root that construction accepts is protected". The two halves differ
    for directories Kiln can neither read nor search (owner mode 0000 or 0200), and for a root
    whose open fails for another reason (for example no file descriptor left). On base these are
    accepted and not held.
  - *Evidence.* P2, as uid 65534 and as uid 0 without DAC capabilities: a 0000 root removed and
    replaced by a directory that got its inode number. `doc.open secret.txt` read the
    replacement, `project.tree` listed it, and a save wrote in it.
  - (A) **Refuse every root that cannot be opened for reading**: `PermissionDenied` for lack of
    permission, the open's own kind otherwise.
    - The second half of the ruling holds literally: every accepted root is held.
    - The residual-limit bullet goes entirely, and "always caught" (R5) becomes true without
      exception.
    - A 0000 root no longer starts a session; such a session could do nothing anyway (every
      request `io_error`, P1).
    - The refusal rests on the open itself, so ACL and LSM denials are covered too.
    - AC-1 gains case (b).
  - (B) **Refuse only roots that can be searched but not read** (the R-145 definition).
    - 0000 and 0200 roots and other open failures stay accepted and not held, so the second half
      of the ruling is untrue for them.
    - `paths.md` keeps a residual-limit bullet, worded for exactly those roots. S1, C1 and C2/C3
      then say "may search but not list", and R5 keeps a narrowed exception.
    - AC-1 drops case (b), and AC-6 keeps the bullet's new title out of its absence checks.
  - **Recommendation: (A).** It is the only reading under which both halves of D-092 Q-1 are
    true. It is no harder to build (the refusal is the failed open) and no harder to test (one
    more mode in AC-1). Ruled (A) in D-111 and applied in R-186.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · tokens (if known) · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13T17:26Z · R-145 · (new) · did: drafted the follow-up of T-008 ruling v12 from
  main `a195dfa` (T-008 merged at `d491560`). Read the T-008 task file (Definitions,
  Interface, Pinned (d)/(e), v4–v13), D-072/D-074/D-076/D-081/D-083, `reviews/T-008-r2.md`,
  `reviews/T-008-r3.md`, `disk.rs`, `paths.md`, the CLI and `kiln-control` root-error paths,
  the gate's test step (`cargo test --workspace --locked`, `xtask/src/gate.rs`:110) and
  no-skip rules (`xtask/src/noskip.rs`:1–11: no conditional `return`, so AC-1 runs both
  identity branches in full).
  - Item 1: four options with consequences. Recommended (b), refusal at construction, because
    its test is deterministic, it needs std only, and it extends an existing
    `Io{PermissionDenied}` root error. Written for (b); the delta for (a) is stated.
  - Unprivileged tests: the child (`kiln-cli`, or the test binary re-run with `--exact`) runs
    as uid/gid 65534 through std's `CommandExt::{uid,gid}` when the gate is uid 0, on a
    fixture chowned to it. Positive and negative controls prove the identity has no override
    and can reach the fixture. On a machine where it cannot drop privileges or lacks the
    precondition, the test fails, naming it. It never skips.
  - TG-1 becomes a reviewer-measured 20/20 detection requirement against a mutant, with
    isolation and a reuse search.
  - TG-3, TG-4 and the moved-back cases are guards (base already behaves so, r2 probes (e),
    (h)). N-1 and item 1 are acceptance.
  - Probes executed in a throwaway `/tmp` directory, as root and as 65534 via `setpriv`, and
    removed afterwards: `/.` link targets, 0311 and 0000 permission controls, inode reuse 6/6
    in a quiet directory and 1/5 in a busy one.
  - Tier high (ADR-0002: confinement boundary stated in `paths.md`; persistence path). No
    gate path, no dependency.
  - Pinned lines n/a: none.
  - Unsure: std dropping supplementary groups on `uid()` is INFERRED (no std source here; the
    controls catch it). The reliability means of AC-2 are not measured on new code.
  - · gate attempts: 0 · not run: cargo (director's instruction), any test on new code · missing
  context: none · next: director answers Q-1…Q-3, then a spec check.
  QUESTION: Q-1 (option for item 1), Q-2 (`docs/control-protocol.md` root sentences), Q-3
  (leave T-008's four tests as they are) — see "Questions and rulings".
- 2026-11-13T23:46Z · R-181 · draft · did: spec check against main `d162413` (T-019 merged at
  `3b1a95d`, T-039 at `c538ef8`; code identical to `3b1a95d`). Rulings D-092 Q-1 (b), Q-2 and
  Q-3 acknowledged and accepted, and applied to the spec text. Brought the file to v1.2: budget
  1,100,000; `scorecard/` out of `touches` (intended-names table); Pinned (i) S1–S3, R1–R6, C1–C3
  with exact text and evidence; no message, document or `error_kinds` pins; no time assertions;
  independent-test-author protocol.
  - Blockers: 12, listed under "Spec check R-181", each fixed or turned into Q-4. The main ones:
    - a 0000 root is accepted and not held under the draft's (b) (probe P2);
    - message pins;
    - a 20/20 detection target that one trial per test cannot meet under load (P4: 83/100);
    - `docs/control-protocol.md` not in `touches`, and its "usage error" untrue (P6);
    - `project.tree`/`list_dir` missing from items 2–4;
    - item 5 missing the `real/new.txt/.` save case.
  - Split: item 5 (N-1) split off by D-091 (the reasons and the cost are under "Split-off item
    5"); the director files it.
  - Tier stays high (ADR-0002: confinement boundary, persistence path); no gate path, no
    dependency. Pinned lines n/a: none.
  - Probes, on the debug `kiln-cli` of a private clone (built once, `target/` deleted after):
    P1 root modes; P2 remove-and-replace sessions, as uid 0 without DAC capabilities (`capsh`)
    and as uid 65534 (`setpriv`, binary and fixture copied to one fixed `/tmp/t036-r181/`,
    removed after); P3 kernel `/.` behaviour and Kiln on base; P4 inode reuse (Python, under
    `/tmp`); P5 std `CommandExt::uid` from uid 0, compiled with `rustc`; P6 bad-root output.
  - Unsure: the 0000-root refusal and the `bad --root` text for `PermissionDenied` are INFERRED
    (no new code). P4's independence across trials is assumed. A root made 0311 after start was
    not probed (INFERRED from P1).
  - spec check: booked to iter-2. · gate attempts: 0 · not run: `cargo test` (any target), any
    mutant run, `cargo xtask` · missing context: none · next: director rules Q-4 and files the
    split-off item-5 task, then dispatch.
  QUESTION: Q-4 (which roots construction refuses; recommended (A), every root that cannot be
  opened for reading) — see "Questions and rulings".
- 2026-11-13T23:58Z · R-186 · draft · did: applied D-111. Acknowledged and accepted Q-4 (A) and
  the split.
  - PENDING marks and the (B) deltas removed; the spec reads as decided for (A).
  - Item 5 removed from Goal, Interface, Pinned (P3), ACs and Out of scope, leaving one line per
    place that points to T-044. Filed `tasks/T-044-dot-in-link-targets.md` from the former
    "Split-off item 5" section; `touches` and the test-name table here had no item-5 entries
    left.
  - Blocker 10: Interface 1 no longer states what a root made unreadable after start answers.
    Only its protection is pinned (tested by AC-2 (iii)). Any document sentence about the rest
    needs a probe recorded in the implementer's log first. Pinned (f) now marks the `project.md`
    case for such a root as INFERRED, for the reviewer to run.
  - Re-read the whole spec as a builder: no contradiction found between sections; the title and
    front matter cover items 1–4 only.
  - · gate attempts: 0 · not run: cargo, git · missing context: D-111 is not yet in
  `docs/decisions.md` (the rulings are read from this file) · next: director moves the task to
  `specified`/`ready`.
<!-- end planner log -->
### implementer
- 2026-11-14T03:11Z · R-189 · ready · did: item 1 (code and documents) and my own integration
  test; items 2–4 left to the test author (not seen).
  - Rulings acknowledged, all accepted: D-092 Q-1 (b) as widened by D-111 Q-4 (A) (refuse every
    root that cannot be opened for reading, `PermissionDenied` for lack of permission, the
    open's own kind otherwise); D-092 Q-2 (`docs/control-protocol.md` `--root` bullet and the
    `run`/`control` exit-code cells edited, C1–C3); D-092 Q-3 (T-008's four tests untouched);
    D-111 split (item 5 is T-044: walk not changed); D-114 (AC-6 is no test; removals checked on
    the diff). No ruling, Interface line or Pinned sentence contradicted the merged code.
  - Criteria in my words: `DiskFs::new` keeps its `NotFound`/`NotADirectory` results, then, after
    canonicalising, opens the root for reading; a failed open is `Io { path: (root), kind: <the
    open's kind> }` and nothing is created; every `DiskFs` holds that handle and takes (dev, ino)
    from it. `kiln-cli` then exits 2, stdout empty, `bad --root` on stderr, no request run.
    Removal/replacement, retargeted link and vanished root stay refused on `read`,
    `write_atomic`, `list_dir`.
  - Code (`disk.rs`): `_root_handle` is now `fs::File` (was `Option`); `new` maps a failed
    `File::open` or `fstat` of the held handle to `io_error((root), e)`; identity from the held
    handle only (the fallback to the path's metadata is gone). Order kept as Pinned (c):
    metadata, directory, canonicalise, open. Module, field and `new` docs say the root is always
    held and a root that cannot be held is refused. `locate`/`locate_steps` untouched; no
    `unsafe`, no dependency.
  - Test (`tests/confinement.rs::root_that_cannot_be_opened_for_reading_is_refused_by_new`): re-runs
    its own binary (`--exact`, env marker) as U (uid/gid 65534 via `CommandExt` when euid is 0, on
    a chowned fixture; own identity otherwise), 10 s hard timeout, marker file checked by the
    parent. Child: positive control (0711 root accepted, `f.txt` read), negative control (0000
    file → `Io{PermissionDenied}`), then modes 0311, 0300, 0200, 0100, 0000 directly and through a
    root symlink → `Io{PermissionDenied}`, `path()` `(root)`, `Display` without the work path, work
    snapshot unchanged; then a root held at 0711, made 0311, removed and replaced by a link to a
    new directory (64 dirs, no inode reuse asserted) → `read`/`write_atomic`/`list_dir(None)`/save
    `OutsideRoot`, `is_file` false, document dirty, snapshot unchanged. No message text asserted.
    Against base `disk.rs` it fails ("mode 311 accepted"); restored after.
  - gate attempts: 3 (1: red clippy, `unwrap_used` in my test helpers, moved into a `cfg(test)`
    module; 2: lost, the container restarted at 02:59Z while it ran; 3: green, "GATE PASSED in
    588.6s (11 steps; 2 step(s) EMPTY …)", test step "759 passed, 0 failed, 0 ignored in 93
    suites"). `cargo xtask no-skip` PASS separately.
  - Evidence for each document sentence: on the debug `kiln-cli` of this branch (built before
    the restart from `3c9b3c5`, the code commit; later commits change tests and docs only), the
    binary and fixture copied to a fixed `/tmp/t036-impl-probe/` (and `-sess/`, `-run/`), removed
    afterwards. Identities: `setpriv --reuid=65534 --regid=65534 --clear-groups` (`Groups:` empty,
    `CapEff 0`) on a fixture chowned to 65534, and `capsh
    --drop=cap_dac_override,cap_dac_read_search` (uid 0, `CapEff 000001fffefffff9`) on a root-owned
    fixture; both gave identical results.
    - S1, C1, C2, C3: `exec doc.open '{"path":"f.txt"}' --root root`, `run s.jsonl --root root`,
      `control --root root` (one `doc.open` on stdin). Modes 0311, 0300, 0100, 0000: each rc=2,
      0 stdout bytes, one stderr line ``kiln-cli: bad --root: root `root`: I/O error: Permission
      denied (os error 13)``, no usage text. 0711: rc=0 for all three. 0644 (read, no search):
      accepted as on base (exec rc=2 with a JSON `io_error` object, run rc=3, control rc=0).
      Negative control (0000 file in a 0711 root) gave `io_error` for both identities. Plain uid 0
      with capabilities: a 0000 root is accepted (capabilities bypass the bits). Base cases
      unchanged: `--root nope` → ``bad --root: root `nope`: not found`` rc=2; a file root → `…:
      not a directory` rc=2. "No request runs": `run no-such-script.jsonl --root root` (0311, as
      65534) gives the `bad --root` line, not a script error, rc=2; `control` with `not json` on
      stdin gives 0 stdout bytes, rc=2.
    - S2 and R1–R6 (held root; removals): as 65534, `control --root root` at 0711, `doc.open
      f.txt`, `doc.insert`, `chmod 0311 root`, then root removed and 64 new dirs made: no inode
      reuse (old (65024,835942), last new (65024,837331)); root → link to `c63` holding
      `secret.txt` (readable through the root path via std): `doc.open secret.txt`, `doc.save`
      and `project.tree {}` all `rejected`/`outside_root`, `doc.info` `dirty: true`, the
      replacement held only `secret.txt` afterwards.
    - S3 (`project.tree` refused): the same session; also as uid 0, `control --root rootlink`
      with `rootlink` retargeted to `other` (`doc.open secret.txt`, `doc.save`, `project.tree {}`
      → `outside_root`; pointed back, the save wrote `root/f.txt`, `other/` unchanged), and the
      root renamed to `root.away` and later removed (`doc.save`, `project.tree {}` →
      `outside_root`, no `root` created; renamed back, the save succeeded).
    - No sentence was added about what a root made unreadable after start answers. Observed only
      (recorded, not documented): after `chmod 0311` a held root gave `doc.open g.txt` ok,
      `project.tree {}` → `io_error` ("cannot list the project: (root): I/O error: Permission
      denied"), `doc.save` ok.
  - Pinned (i): S1, S2, S3, C1, C2, C3 copied word for word; R1–R6 removed as listed. The two
    edited "The root" paragraphs were re-wrapped (words unchanged); nothing else edited in either
    document. `docs/protocol/project.md` not edited.
  - not run: the test author's tests (not seen); the reviewer's 20-run detection measurement
    and mutant M; an ACL or LSM denial; macOS/CI non-root identity (my test's own-identity branch
    is covered by code reading only, the gate is uid 0); a release build.
  - missing context: none. Interruption: the container restart killed gate attempt 2; probe
    outputs above were observed before it, the gate was re-run in full after it.
  - next: director joins `task/T-036-tests`.
  QUESTION: AC-4 through `control` expects `doc.open` of `f.txt` → `outside_root` after the root
  vanishes, but if `f.txt` is the document opened before, `doc.open` answers `ok` with
  `already_open: true` without touching the disk (`crates/kiln-commands/src/doc.rs`:136,
  documented; observed on this branch and the same code on base). `paths.md` "The root" keeps
  (Pinned, verbatim) "the session refuses every path with `outside_root`, at open …", which is
  then wider than the binary for an already-open path. I changed neither (outside `touches` /
  kept verbatim); does AC-4 mean a path not already open, and should a later task narrow the
  sentence?
- 2026-11-14T04:07Z · R-195 · review r1 request-changes · did: rework on `cbcfdbb` (my work,
  the test author's tests, the scorecard tie, review r1).
  - Rulings acknowledged, all accepted (D-119, read in the primary checkout's copy of this file,
    "Rulings on review round 1"; this branch's copy does not carry them): p1 (finding 1, test
    gap) the fixture must go on every exit path and the child is waited for after a kill, no
    assertion change; p2 (finding 2, nit) C1 gains a pointer to `exec`'s params-before-root
    order, written only from a probe, S1 untouched; p3 (finding 3) no change, accepted.
  - p1 (`tests/confinement.rs`, `mod t036`): a `Fixture` guard whose `Drop` makes every
    directory under the fixture mode 0755 again (links not followed) and removes it; the parent
    holds it from creation, so success, a failed assertion and a panic all clean up. After
    `kill()` on the 10 s hang detector the child is now `wait()`ed. No assertion changed.
    Proof: temporary one-line mutant in `disk.rs` (`File::open(&resolved)` falling back to
    `File::open("/")`, so unreadable roots are accepted); `cargo test -p kiln-workspace --test
    confinement root_that` FAILED (child: "mode 311 accepted", leaving its root at mode 0311;
    parent: "the unprivileged child run failed"); then `ls -d /tmp/k8i-*t036*` → "No such file or
    directory". `disk.rs` restored with `git checkout --`; `git status` clean.
  - p2 probe (debug `kiln-cli` rebuilt from this branch, as uid 65534 via `setpriv
    --clear-groups`, fixture under `/tmp/t036-impl-p2/`, root 0311 owned by 65534, removed
    after): `exec doc.open '{' --root root` → rc 2, 0 stdout bytes, stderr
    `{"code":"invalid_json","kind":"syntax",…}` and no `bad --root` line; `exec doc.open
    '{"path":"f.txt"}' --root root` → rc 2, 0 stdout bytes, ``kiln-cli: bad --root: root `root`:
    I/O error: Permission denied (os error 13)``. Added after C1, which is otherwise unchanged:
    "For `exec`, params that are not JSON are reported first (`invalid_json`), before the root is
    checked (order of checks: `docs/protocol/cli-memory-seed.md`)."
  - gate attempts: 1 (green: "GATE PASSED in 361.1s (11 steps; 2 step(s) EMPTY …)", test step
    "770 passed, 0 failed, 0 ignored in 99 suites").
  - not run: the test's own-identity branch on a non-root machine (a failed child there could
    leave 0000/0311 directories; the guard unlocks them as the owner, by code reading only);
    the 10 s hang path (not provoked). Seen, not mine: `/tmp/k8i-<pid>-{corner,esc,fifo,in,swap}`
    from T-008's tests in earlier runs, which remove nothing by design.
  - missing context: D-119 is not on this branch (read from the primary checkout). · next:
    review round 2.
<!-- end implementer log -->
### test author
- 2026-11-14T03:09Z · R-190 · ready · did: wrote the six files in `touches` from this spec
  alone, on `task/T-036-tests` (base `138be41`). The container was restarted at 02:59Z, mid-run.
  The work committed before it survived. The only uncommitted edits were rustfmt reflows in three
  files; I read them, found them complete and committed them. No fixture was left in `/tmp`.
  Every check below was re-run after the restart.
  - Rulings acknowledged and accepted before acting on them:
    - D-092 Q-1 (b): refusal at construction.
    - D-092 Q-2: no test reads the docs.
    - D-092 Q-3: T-008's and T-019's test files untouched; AC-2 in two new binaries.
    - D-111 Q-4 (A): AC-1 covers 0311 and 0000.
    - D-111 split: no item-5 test.
    - D-114: AC-6 is not a test; `documents_drop_the_unreadable_root_limit` is not written, and
      `acceptance_t036_cli.rs` holds AC-1 only.
  - Tests for `ws.path-confinement`, and their result on base (euid 0, so U = 65534):
    - AC-1 `kiln_cli::acceptance_t036_cli::unreadable_root_is_refused_at_start`: FAILS on base
      by assertion in all six cases, after both controls passed.
      - 0311: `exec` exit 0 with a result; `run` exit 0; `control` exit 0 with a response.
      - 0000: `exec` exit 2 but a JSON `io_error` object on stderr; `run` exit 3; `control`
        exit 0 with a response.
      - This is exactly the base the spec predicts.
    - AC-1 `kiln_workspace::acceptance_t036_workspace::unreadable_root_is_refused_by_new`: FAILS
      on base by assertion in the child run as U, for both modes: `DiskFs::new` returned `Ok`,
      with `_root_handle: None`.
    - AC-2 `kiln_workspace::guard_t036_removal_workspace::{root_removed_and_replaced_by_link_is_refused,
      root_removed_and_replaced_by_new_directory_is_refused}`: pass on base.
    - AC-2 `kiln_cli::guard_t036_removal_cli::{root_removed_and_replaced_by_link_is_refused_through_control,
      root_removed_and_replaced_by_new_directory_is_refused_through_control,
      root_made_unreadable_after_start_stays_protected_through_control}`: pass on base.
    - AC-3 `kiln_workspace::guard_t036_workspace::retargeted_root_link_is_refused` and
      `kiln_cli::guard_t036_cli::retargeted_root_link_is_refused_through_control`: pass on base.
    - AC-4 `kiln_workspace::guard_t036_workspace::vanished_root_is_refused` and
      `kiln_cli::guard_t036_cli::vanished_root_is_refused_through_control`: pass on base, cases
      (i), (ii) and (iii) in one test each.
  - AC-2 parameters: K = 5, B = 64.
    - Lock: a `static Mutex<()>` per file, held by every test (poison-tolerant).
    - Fresh `work/` and a fresh session per trial; one stderr line per trial; `break` on reuse.
    - Refusals are asserted in every trial. Beyond the spec's list, the Rust API test also checks
      `list_dir(Some(secret.txt))`, which Interface 2 names.
  - 20 runs of each removal binary on base (`cargo test -p … --test … -- --nocapture`, default
    threads, load average about 4):
    - `guard_t036_removal_workspace` 20/20 passed and `guard_t036_removal_cli` 20/20 passed.
    - 0 of 20 runs of either binary reported reuse (0 of 200 and 0 of 300 trials).
    - On base, as uid 0, every root of these tests is held (modes 0755, and 0711 at start for
      (iii)), so base behaves as the fixed code. No unheld root was exercised there.
  - Self-check, in a throwaway copy at `/tmp/t036-r190-selfcheck` (0755, so 65534 could reach
    it; removed afterwards). All 11 tests were run against each version:
    - plausible fix (open the root, refuse on failure, hold it): all 11 pass;
    - B1 (refusal with `NotFound`): killed by AC-1 Rust;
    - B2 (refuse only if owner-search is set, so 0000 is accepted): killed by AC-1 Rust and CLI;
    - B3 (refusal with path `root`, not `(root)`): killed by AC-1 Rust;
    - B4 (T-008 M4, `given` = canonical path): killed by AC-3 Rust and CLI;
    - B5 (T-008 M5, failed re-resolution taken as success): killed by AC-4 Rust and CLI;
    - B6 (no identity comparison): killed by AC-2 (5 tests) and AC-3 (2 tests);
    - B7 (refusal message names the host path): killed by AC-1 Rust.
    - B1, B3 and B7 survive the CLI test by design, since no stderr text is pinned (golden rule
      9).
  - Mutant M (open, refuse, then drop the handle), 10 runs per removal target with
    `--nocapture`:
    - every test failed in 10/10 runs;
    - reuse was found in 20 of 23 workspace trials and 30 of 32 CLI trials, always by the first
      or second trial, case (iii) included;
    - no run passed, and no reused trial passed.
  - `cargo fmt --check` clean.
  - `cargo clippy -p kiln-workspace -p kiln-cli --tests -- -D warnings` clean.
  - `cargo xtask no-skip` PASS.
  · gate attempts: 0 · not run: `cargo xtask ci` and the full workspace suite (brief: own targets
  only; on this branch the gate should fail only in the two acceptance tests); the non-root
  identity branch (this machine is uid 0; that branch is written but not executed) · missing
  context: none · next: director joins with `task/T-036`.
  QUESTION: AC-4 lists "`read` / `doc.open` of `f.txt` … → `outside_root`" through `control`.
  But `f.txt` is document 1 there, and `doc.open` of an open document returns it with
  `already_open: true` without reading the disk (`crates/kiln-commands/src/doc.rs`:136, the
  registry's contract). On base, `doc.open f.txt` answered `ok` after the root was renamed away
  (and after the AC-3 retarget). So that step cannot hold on base or head while `kiln-commands`
  is outside `touches`. I did not send it. Instead `vanished_root_is_refused_through_control`
  opens `g.txt`, an existing file in the root that is not open (Interface 4: "every request is
  refused"), plus `missing.txt`. The Rust test keeps `read("f.txt")`. Proposed wording: "through
  `control`, `doc.open` of an existing file that is not open (`g.txt`) and of `missing.txt` →
  `outside_root`; through the Rust API, `read` of `f.txt` and `missing.txt` → `OutsideRoot`".
<!-- end test author log -->
### reviewer
- 2026-11-14T03:58Z · R-192 · tests_joined · did: round 1 (full, high) of head
  `29c1f401e6a266db6012feb7cfb25f1bc096fd11`, verdict request-changes (`reviews/T-036-r1.md`,
  commit `8c323b0`). Rulings acknowledged and accepted: D-092 Q-1 to Q-3, D-111 Q-4 (A) and the
  split, D-114, D-117 (read from the primary checkout's copy; this copy lacks D-117).
  - Every AC checked through `kiln-cli` as uid 65534 (`setpriv`) and as uid 0 without DAC caps
    (`capsh`): 7 unreadable modes refused, held roots protected after chmod, removal,
    replacement (link, directory, file, bind mount, lazy unmount, cycles); nothing written.
  - Mutants 15: 14 killed (all by this task's acceptance/guard files), 1 equivalent (M7), 0
    survived. Mutant M: every removal test failed in 20/20 runs, reuse in every trial; head
    20/20 passes per binary, 0 reuse.
  - Documents: S1–S3, C1–C3 verbatim, R1–R6 removed exactly; all sentences true except NEW-1
    (listed for T-037).
  - Findings: 1 test gap (`confinement.rs` T-036 test leaves its `/tmp` fixture on failure),
    2 nits.
  · gate attempts: 0 (ran `cargo test --workspace` on head: 770 passed; `no-skip` PASS; `hygiene`
  PASS) · not run: `cargo xtask ci`, ACL/LSM denial, EMFILE on the root open, the non-root
  identity branch, TG-2 · missing context: none · next: implementer fixes finding 1; round 2.
- 2026-11-14T04:20Z · R-199 · tests_joined · did: round 2 (final) of the delta from
  `29c1f40` to `5b2bffb677b6f8ad89b38ca27cfc47dbbb3243ef`; verdict approve, merge recommended
  (`reviews/T-036-r2.md`, commit `66842ee`). Rulings D-119 p1–p3 acknowledged and accepted
  (read from the primary checkout's copy).
  - The delta is the test guard, the wait after kill and the C1 pointer sentence; no `src`
    change, no assertion changed.
  - Finding 1 resolved: M2, M5, a hanging child and an aborting child each made the test fail,
    and each left 0 `/tmp/k8i-*-t036-*` and no process; so did the precondition panic.
  - Finding 2 resolved: the sentence matches the binary as 65534 (`invalid_json` before the
    root check for `exec`; `bad --root` for valid JSON, `run` and `control`).
  - Finding 3 accepted by p3.
  · gate attempts: 0 (`hygiene` PASS) · not run: `cargo xtask ci`, full suite, non-root branch ·
  missing context: none · next: director records the approval.
<!-- end reviewer log -->
