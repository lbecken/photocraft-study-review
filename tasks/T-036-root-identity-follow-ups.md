---
id: T-036
title: "Close the unreadable-root gap of T-008, make root-removal detection reliable, and pin the retargeted-link, vanished-root and `/.` link-target cases"
tier: high              # ADR-0002: a confinement/security boundary that docs/protocol/paths.md states (the root's identity between requests) and the persistence path (DiskFs construction, read, write_atomic)
state: draft
owner_role: planner
scorecard_items: [ws.path-confinement]   # existing item, strengthened (no new item)
depends_on: [T-008]     # merged at d4915606f9a789b42b99a1eea053554f1f9dd8a7
touches:
  - crates/kiln-workspace/src/disk.rs                          # implementer: DiskFs construction, the `/.` walk rule, module and field docs
  - crates/kiln-workspace/tests/confinement.rs                 # implementer's own integration tests (exists, T-008)
  - docs/protocol/paths.md                                     # implementer: "The root", "Location", "Residual limits"
  - crates/kiln-workspace/tests/acceptance_t036_workspace.rs   # test author: item 1 (Rust API), item 5 (N-1)
  - crates/kiln-workspace/tests/guard_t036_workspace.rs        # test author: items 3 and 4 (Rust API)
  - crates/kiln-workspace/tests/guard_t036_removal_workspace.rs  # test author: item 2 (Rust API), its own test binary
  - apps/kiln-cli/tests/acceptance_t036_cli.rs                 # test author: items 1 and 5 through kiln-cli, AC-6 (paths.md)
  - apps/kiln-cli/tests/guard_t036_cli.rs                      # test author: items 3 and 4 through control
  - apps/kiln-cli/tests/guard_t036_removal_cli.rs              # test author: item 2 through control, its own test binary
  - scorecard/items/ws.path-confinement.toml                   # test author: append this task's test names
  # docs/scorecard.md also changes: a generated path, regenerated (`cargo xtask scorecard`), exempt
budget_tokens: 850000   # high (v1.1)
iteration: 1
found_by: reviewer      # T-008 r2 (R-138): R2-D1's residual, TG-1, TG-3, TG-4, N-1
origin_task: T-008
---

# T-036 Close the unreadable-root gap of T-008, make root-removal detection reliable, and pin the retargeted-link, vanished-root and `/.` link-target cases

## Goal
T-008 merged (`d4915606f9a789b42b99a1eea053554f1f9dd8a7`, D-083) with one accepted gap (ruling
v11, D-081). If the process may traverse the root but not open it for reading, the session
does not hold the root open. A root that is removed and replaced by another directory between
two requests then goes undetected, and the session reads and writes in the replacement
(VERIFIED: `reviews/T-008-r2.md` probe (i), as user `nobody` with a root of mode 0311, inode
number reused 6 of 6; documented at `docs/protocol/paths.md`:97–101). After this task no
session reads or writes in a directory that replaced its root between two requests, whatever
the root's permissions (item 1). Ruling v12 gives this task four more items. Item 2: the
root-removal detection is reliable as the gate runs it (TG-1). Item 3: a root given through a
link that is later retargeted has an acceptance-level test (TG-3). Item 4: the vanished-root
refusal is pinned and tested (TG-4). Item 5: a link target ending in `/.` behaves as the
kernel does (N-1). TG-2 is recorded as accepted (Out of scope).

No gate path and no dependency in `touches`. If std alone cannot meet a criterion, stop and
report; do not add a crate.

Reading list: `tasks/T-008-path-confinement.md` Definitions, Interface (error table,
precedence), Pinned (d) and (e), `## Questions` rulings v4–v13; `docs/decisions.md` D-072,
D-074, D-076, D-081, D-083; `reviews/T-008-r2.md` (R2-D1, TG-1…TG-4, N-1, probes (a)–(j),
mutants M1, M3, M4, M5) and `reviews/T-008-r3.md`; `crates/kiln-workspace/src/disk.rs`
(`DiskFs`, `new`, `current_root`, `locate`, the `_root_handle` field and its comment);
`docs/protocol/paths.md`; `docs/control-protocol.md` "kiln-cli" and "Exit codes";
`crates/kiln-workspace/tests/acceptance_t008.rs` and `apps/kiln-cli/tests/acceptance_t008_cli.rs`
(fixtures, the root-removal tests, the `control` driver with per-response timeouts).

## Item 1: the unreadable root. Options for the director (Q-1)
"Unreadable root" means a directory that the session's identity may search (`x`) but not read
(`r`). Each option lists one consequence, then what it changes in `DiskFs::new`'s pinned
contract (T-008 Interface: "`DiskFs::new` returns the same `Ok`/`Err` values as today for every
root") and in `docs/protocol/paths.md`.

| Option | Behaviour | Consequence | `DiskFs::new` contract | `docs/protocol/paths.md` |
|---|---|---|---|---|
| (a) | Such a root keeps working and is protected like a readable one: after it is removed and replaced, every request is `outside_root`. | std names no way to keep a directory without read permission (reviewer's note: Linux `O_PATH`, which std does not name and macOS lacks), and the acceptance test can only show the refusal when the freed inode number is in fact reused. | unchanged | drop the unreadable-root bullet of "Residual limits" and the qualifications at :14–16, :20–22, :52–55, :69–73, :95–96 |
| (b) | Construction refuses such a root with a pinned error; every root that construction accepts is protected. | A project whose root its user cannot list cannot be opened at all (`kiln-cli` exits 2 with `bad --root`). Such a session already cannot list its root, and T-019's `project.tree` lists it. | one stated exception: an unreadable directory root is `Err(Io{kind: PermissionDenied})` | "The root" says such a root is refused at start; the "Residual limits" bullet and the same qualifications go |
| (c) | Construction accepts it; every later request of that session is refused. | The session starts but can do nothing, and the refusal is either a misleading `outside_root` for an in-root file or a new kind (a `reason.rs` row; shares the file with T-020 and T-027 and needs a kind approval, D-031 (10)). | unchanged | new paragraph for the refusal; the bullet goes |
| (d) | Keep the documented gap. | Contradicts ruling v12 (1); listed only for completeness. | unchanged | unchanged |

**Recommendation: (b).** It is the only option whose acceptance test is deterministic: the
refusal does not depend on the file system reusing an inode number. It needs only std and
behaves the same on every Unix. It extends an error class that already exists: a root under a
parent the process cannot search is already `Io{kind: PermissionDenied}` with path `(root)`
today (VERIFIED by reading `disk.rs`:155–160, :181: `fs::metadata` fails and the error is not
`NotFound`). And it leaves `docs/protocol/paths.md` with one sentence about it instead of a
residual limit. The rest of this file specifies (b). If the director chooses (a), Interface (1)
becomes "construction unchanged; after removal and replacement every request of such a session
is `outside_root`, at open and at save, and nothing is written anywhere", and AC-1 is replaced
by that case run as an unprivileged user. AC-1's fixture, identity rules and controls stay as
written, and the test then joins item 2's reliability requirement.

## Definitions
- **Root, location, inside, regular file**: as T-008 Definitions.
- **Unreadable root**: at construction, the root (resolved as T-008 says) is a directory the
  calling process may search but may not open for reading.
- **The root path leads nowhere**: resolving the root path as given fails, because the root or
  a directory above it was renamed or removed and nothing was put in its place.
- **Unprivileged identity U** (tests only): if the test process's effective uid is 0, U is uid
  65534 with gid 65534. Otherwise U is the test process's own identity. The effective uid is
  found without a new dependency: it is the owner of a file the test just created, as
  `disk.rs`:707–709 already does.

## Interface
`Vfs` and `VfsError` are unchanged, and so is every public signature. No new kind and no
`reason.rs` row. `kiln-control` and `kiln-cli` code are unchanged. Their existing `root_error`
arm for other `VfsError`s (`crates/kiln-control/src/lib.rs`:163–176, `apps/kiln-cli/src/main.rs`:357–369)
already prints an `Io` root error.

1. **Unreadable root (item 1, option (b)).** `DiskFs::new(root)`, where the root is an
   unreadable root, returns `Err(VfsError::Io { path: (root), kind: PermissionDenied, message })`.
   Its `Display` begins with `(root): ` and contains no host path. `message` is not pinned.
   Nothing is created or changed anywhere. Precedence at construction: `NotFound` (path
   `(root)`) → `Io{NotADirectory}` → this refusal. A dangling root link and a link to a
   regular file keep T-008's errors. Every other root gets the same `Ok`/`Err` as on base.
   Through `kiln-cli`: `exec`, `run` and `control` with such a `--root` exit 2. stdout is
   empty and no request is executed. The first stderr line begins with
   ``kiln-cli: bad --root: root `<root as given>`: ``, and the rest of the line is not pinned.
   A root that was readable when the session started keeps working if its mode changes
   later, and it is protected (Interface 2).
2. **Root removed and replaced (item 2; T-008 v9 behaviour, unchanged).** This holds for every
   session that construction accepts. Suppose the root directory is removed between two
   requests and its path is replaced by a link to another directory, or by a new directory.
   Then every request is refused with `OutsideRoot` (`{path}: resolves outside the root`,
   kind `outside_root`) at open and at save, including the save of a document opened before.
   Nothing is written anywhere. This holds whether or not the replacement got the removed
   root's inode number.
3. **Root given through a link that is retargeted (item 3; T-008 v8 behaviour, unchanged).**
   Take a `DiskFs` or session whose root was given as `rootlink`, a symlink to a directory.
   After `rootlink` is pointed at another directory between two requests, every request is
   `OutsideRoot` at open and at save, including the save of an earlier document, and nothing
   is read or written in either directory. After `rootlink` is pointed back at the original
   directory, requests work again there.
4. **The root path leads nowhere (item 4; newly pinned, base behaviour as observed by T-008 r2
   probe (h)).** This applies when the root, or a directory above it, has been renamed away or
   removed with nothing in its place. While that lasts, `read` of any path is `OutsideRoot`,
   `is_file` is `false`, and `write_atomic` of any path is `OutsideRoot`. A save of a document
   opened before is refused with `cannot save <p>: resolves outside the root` and the document
   stays dirty. Nothing is created anywhere: not the root path, not a file in the renamed-away
   directory, and no temporary file. Once the root path leads to the root directory again
   (renamed back), requests work there again. `doc.open`/`doc.save` through `control` give
   kind `outside_root` with the same messages.
5. **Link target with `.` after a non-directory (item 5, N-1; Definitions "as the kernel
   does").** Take a symlink whose target has a `.` component right after a component that
   leads to something other than a directory, for example `real/t.txt/.`. The walk fails
   there, as the kernel's does ("Not a directory"). Inside the root this is `NotFound` for
   `read` and `write_atomic`, and `is_file` is `false`. The target file is unchanged and
   nothing is created. If the walk is outside the root at that point, the result is
   `OutsideRoot` (T-008 precedence). A `.` after a directory changes nothing (`real/./t.txt`
   reads `real/t.txt`; `real/.` is the directory `real`, hence `NotAFile`). `paths.md`'s
   trailing-slash sentence widens to: a link whose target ends in `/` or `/.`, or has `.`
   after a non-directory, must lead to a directory; a file reached that way is `not_found`.

### Documentation
`docs/protocol/paths.md` (implementer) must state, and claim no more than the code keeps:
- "The root": a root the session cannot open for reading is refused at start (`kiln-cli`
  exits 2, `bad --root`). The qualifications at :14–16, :20–22, :52–55, :69–73 and :95–96
  ("when it can be opened for reading", "except in the case of an unreadable root…") go, and
  the "A root that cannot be opened for reading" bullet of "Residual limits" goes. They go
  only if no session can be constructed any longer without the root being held. If the code
  still accepts a root it does not hold in some other case (for example running out of file
  descriptors at construction, which T-008 r3 notes), that case stays a residual limit,
  worded for that case.
- The vanished-root sentence (:22–23) is kept and now describes pinned behaviour (Interface 4).
- The trailing-slash sentence widened as in Interface 5.
- The `//!` module docs and the `_root_handle` field docs of `disk.rs` say the same as the
  document.
- `docs/control-protocol.md` is not edited (Q-2).

## Pinned
- (a) Errors, payloads, precedence: no new variant or kind. Unreadable root:
  `Io{kind: PermissionDenied, path: (root)}`, after `NotFound` and `NotADirectory`. Interfaces
  2–4: `OutsideRoot` before anything else, as T-008 (c). Interface 5: `NotFound` inside, and
  `OutsideRoot` once the walk is outside. CLI: exit 2 and the `bad --root` prefix; `run`
  exit 3 and `exec` exit 2 for refused requests, as T-008.
- (b) Files: as T-008 (b). No file is created on a refused construction or save. Test files
  are those in `touches`. All six names are distinct across crates (the join keys targets by
  file name). Item 2's tests live in their own two binaries (`guard_t036_removal_*`).
- (c) Order of checks: construction checks existence, then directory, then readability. Per
  call, T-008 (c) applies unchanged: root identity → location → outside → existence → type →
  I/O.
- (d) Path base, platform, file types: as T-008 (d). Unix only. The unprivileged tests rely
  on POSIX permission semantics for the owner bits of a directory (mode 0311: search without
  read).
- (e) Threat model: see "Threat model" below.
- (f) Tests pinning a snapshot this changes: none found. `grep` of every `tests/` directory
  and `src/` for modes without read permission on a directory finds none. Every existing test
  runs as uid 0 in the gate, where an unreadable root cannot arise. The T-008 tests stay
  unchanged and green, including its four probabilistic root-removal tests
  (`acceptance_t008.rs`:753–842, `acceptance_t008_cli.rs`:669–754). `guard_t008.rs` pins
  `DiskFs::new` errors only for a dangling link and a link to a file, which do not change.
  `acceptance_t002.rs`:1040–1042 asserts `is_err` only.
- (g) Worked examples, run by this planner in a fresh `mktemp -d /tmp/…` directory (as root,
  then as uid/gid 65534 with `setpriv --reuid=65534 --regid=65534 --clear-groups`), removed
  afterwards:
  - `ln -s real/t.txt/. dot; cat dot` → `Not a directory`; `echo X > dot` → `Directory
    nonexistent`, `real/t.txt` unchanged. `dots -> real/t.txt/./` → `Not a directory`;
    `dd -> real/.` → `Is a directory`; `mid -> real/./t.txt` → `target`;
    `od -> ../outside/secret.txt/.` → `Not a directory`. On base, `exec doc.open dot` →
    ok (T-008 r2, N-1), so AC-5 fails on base. `dots` is already `not_found` on base
    (VERIFIED by reading `disk.rs`:262–263, which pushes a directory requirement for a target
    ending in `/`).
  - As 65534, with the root of mode 0311 owned by 65534: listing the root → `Permission
    denied`. `cat root/f.txt` (0644) → content. A file of mode 0000 owned by 65534 →
    `Permission denied`. `/proc/self/status`: `Groups:` empty, `CapEff: 0000000000000000`.
    As root, listing the same root succeeds, so the gate's own identity cannot see the gap.
  - Under the session's scratch directory (mode 0700, root-owned), 65534 could not even reach
    the fixture. That is why AC-1 requires a fixture parent U can traverse, and a positive
    control.
  - Inode reuse, `mkdir r; rm -r r; mkdir fresh` in a small fresh directory under `/tmp`: the
    same inode number 6 of 6 times. In a busy directory, 1 of 5. Reuse depends on what else
    allocates inodes at that moment, which is TG-1's cause.
  - On base, `DiskFs::new` of a 0311 root as U returns `Ok` (the reviewer's probe (i) ran a
    whole session), so AC-1 fails on base by assertion.
  - Not executed: anything on the new code; std's behaviour with `CommandExt::uid`/`gid`
    (INFERRED: std drops supplementary groups when it sets the uid; AC-1's controls catch it
    if not).
- (h) Scorecard item `ws.path-confinement`. As an unprivileged user, the evaluator runs
  `kiln-cli exec doc.open '{"path":"f.txt"}' --root <dir of mode 0311>` and gets exit 2, empty
  stdout and stderr ``kiln-cli: bad --root: root `<dir>`: …``. The same call with the
  directory at mode 0711 succeeds. The evaluator also runs a `kiln-cli control --root
  rootlink` session, retargets `rootlink` after `doc.insert`, and gets `outside_root` at open
  and at save, with the work tree unchanged.

## Threat model
Kept from T-008 (e): an honest user or a hostile repository, with no concurrent attacker
inside one call. Added here: another process, under the session's user or another one,
removes, renames or replaces the root or a directory above it between two requests.
- **Covered.** Every root that construction accepts keeps its identity between requests:
  removal plus replacement by a link or a new directory, a retargeted root link, an ancestor
  swap (T-008), and a vanished root. In all of these nothing is read or written outside the
  root directory. An unreadable root is refused before any request runs.
- **Not covered (accepted, documented).**
  - Changes during one call (D-3 of T-008).
  - Hard links.
  - A replacement on another device that has the same inode number (TG-2; Out of scope).
  - A root made unreadable or swapped while `DiskFs::new` itself runs: same class as D-3.
  - Other failures to open the root at construction, which no std-only test can provoke.
    `paths.md` states them if the code still accepts such a root.
  - An identity with capabilities that bypass permission checks: that is root for this
    purpose, and the gap cannot arise.
  - Roots unreadable through ACLs or an LSM policy instead of mode bits. These are not tested;
    the reviewer probes one if the machine allows it.
  - Network file systems and macOS (not tested).
- **Reviewer probes adversarially:** an unreadable root through `exec`, `run` and `control`;
  a root unreadable only through its group or other bits while the owner differs; a root made
  unreadable after start and then removed and replaced; a root reached through a link whose
  target is unreadable; repeated remove-and-replace races; `.` and `..` after files in link
  targets, inside and outside the root.

## Acceptance criteria
Fixtures as in T-008: a fresh `work/` per test with `work/root/`, `work/outside/`, and
`work/outside/secret.txt` = `b"s3cr3t-content\n"`. Each test first asserts that the files it
expects a refusal to protect are reachable through the changed path with `std::fs`, so the test
cannot pass vacuously. "Snapshot" means every entry under `work/`, recursively, links not
followed: type, link target, bytes. Errors are asserted by `to_string()` and `path()`, never by
matching a variant, except `VfsError::Io { kind, .. }` in AC-1, which exists on base. Every
child process (`kiln-cli`, or the test's own binary) has a hard timeout of 10 s, after which it
is killed and the test fails. Every `control` response is awaited at most 10 s. FIFOs, sockets
and devices are not used. No test asserts a count of `error_kinds`, a whole document's bytes,
or a message text that this task does not pin. No test returns early or skips: where the
identity differs (uid 0 or not), both branches run the whole scenario.

Base = `main` with T-008 merged. **Acceptance** = fails on base; **guard** = passes on base.

- **AC-1 (item 1; acceptance; `acceptance_t036_cli.rs::unreadable_root_is_refused_at_start`
  and `acceptance_t036_workspace.rs::unreadable_root_is_refused_by_new`).**
  - *How the process under test becomes unprivileged.* The gate runs as uid 0, which bypasses
    the permission checks. The process under test is therefore always a child run as U, and
    the fixture is handed to U:
    - If the test's effective uid is 0, the fixture's directories and files are given to U
      with `std::os::unix::fs::chown`. The child is spawned with
      `std::os::unix::process::CommandExt::{uid, gid}`.
    - Otherwise the child runs as the test's own identity, and no chown is needed.
    - `work/` lies in a directory U can traverse, for example `std::env::temp_dir()` when it is
      `/tmp`, and has mode 0755.
  - *Controls, in the same test, same identity, before the case.* These controls make the
    case non-vacuous on every machine.
    - Positive control: a readable root `work/open` (mode 0711) holding `f.txt` (0644). As U,
      `exec doc.open '{"path":"f.txt"}' --root work/open` must succeed. This shows U can run
      the binary and reach the fixture.
    - Negative control: `work/open/locked.txt` (mode 0000, owned by U). As U, `exec doc.open
      '{"path":"locked.txt"}'` must fail (exit 2, empty stdout). This shows U has no override
      of permission checks.
  - *Case.* `work/root` (mode 0311, owned by U) holds `f.txt` (0644, readable by U).
    - `kiln-cli exec doc.open '{"path":"f.txt"}' --root work/root` as U: exit 2, stdout
      empty, first stderr line begins with ``kiln-cli: bad --root: root `work/root`: `` (the
      root as given).
    - `kiln-cli run s.jsonl --root work/root` with `doc.open f.txt`: exit 2, stdout empty.
    - `kiln-cli control --root work/root` with one request on stdin: exit 2, no response line
      on stdout.
    - The `work/` snapshot is unchanged.
    - On base all three succeed or answer, so the test fails by assertion.
  - *Rust API variant.* The in-process API cannot run as U under a uid-0 gate, so the test
    re-runs its own test binary (`std::env::current_exe()`, `--exact <its name>`, an
    environment variable marking the child run) as U, with the same controls and timeout.
    - In the child run it asserts `DiskFs::new(work/root)` → `Err(VfsError::Io { kind:
      PermissionDenied, .. })`, `path()` `(root)`, and a `Display` that begins with `(root): `
      and contains no part of the work path. It also asserts that `DiskFs::new(work/open)` is
      `Ok`.
    - The parent asserts that the child run passed and that it actually ran those
      assertions, for example through a marker file the child writes after them.
  - Oracle: exact values above.
- **AC-2 (item 2, TG-1; guards with a measured detection requirement;
  `guard_t036_removal_workspace.rs` and `guard_t036_removal_cli.rs`).**
  - *Cases.* Rust API (one `DiskFs` kept across the change) and `control` (one child, change
    after the `doc.insert` response):
    - (i) the root removed and replaced by a link to a new directory;
    - (ii) the root removed and replaced by a new directory at the same path;
    - (iii) through `control` only, as U with the AC-1 identity and controls: a root of mode
      0711 when the session starts, then chmod 0311, then (i). This shows a root readable at
      start stays protected after its mode changes.
  - *Expected in each case.* T-008's v9 expectations: `secret.txt` in the replacement is
    readable through the root path first, then `outside_root` at `read`/`doc.open`,
    `is_file` false, `outside_root` at `write_atomic`, and `cannot save f.txt: resolves
    outside the root` for the earlier document, which stays dirty. The replacement and
    `work/` snapshots are unchanged, and no `f.txt` is created in the replacement.
  - *Making detection reliable* is the test author's choice, but two properties are required:
    - no two tests of the same binary run their remove-and-replace window at the same time
      (for example every test of the file holds one file-level lock, poison-tolerant);
    - the replacement is likely to receive the removed root's inode number when nothing holds
      it. For example: after the removal, create directories until one has the removed
      root's (device, inode) or a bound the test author chooses and records is reached; use
      that one, or the last one. The refusal is asserted either way, with no branch that
      passes without asserting.
    - Each failure message states the old and new (device, inode) and whether the number was
      reused.
  - *Detection requirement, measured by the reviewer.* M is the head with the root's
    reuse protection removed: T-008 r2 mutant M1, or its equivalent in the new code.
    - Against M, each test of both files fails in 20 of 20 runs of its target, run as the
      gate runs it: `cargo test -p kiln-workspace --test guard_t036_removal_workspace` and
      `cargo test -p kiln-cli --test guard_t036_removal_cli`, default test threads.
    - Against M, one `cargo test --workspace --locked` run fails in every one of them.
    - Against the head, both targets pass in 20 of 20 runs.
    - The reviewer records the counts, the host file system and anything else running on the
      machine. The test author records the same measurement against a local mutant before
      the join.
    - Base (T-008 r2): 6 of 10 runs for the API target as the gate runs it, 10/10 for each
      test alone.
- **AC-3 (item 3, TG-3; guard; `guard_t036_workspace.rs::retargeted_root_link_is_refused` and
  `guard_t036_cli.rs::retargeted_root_link_is_refused_through_control`).**
  - *Fixture.* `work/rootlink -> root` with `root/f.txt`, and `work/other/` holding
    `secret.txt` and its own `f.txt`. The session is given `work/rootlink`.
  - *Steps.*
    1. Open `f.txt` and edit it.
    2. Point `rootlink` at `other` and assert through `std::fs` that `rootlink/secret.txt` is
       readable.
    3. Then: `read`/`doc.open secret.txt` → `secret.txt: resolves outside the root` / kind
       `outside_root`; `is_file` false; `write_atomic("new.txt")` → `OutsideRoot`; the save
       of the earlier document → `cannot save f.txt: resolves outside the root`, still dirty.
       The `work/` snapshot is unchanged: `root/f.txt` and `other/f.txt` keep their bytes and
       there is no `new.txt` and no `.kiln-tmp-*`.
    4. Point `rootlink` back at `root`. The save of the earlier document now succeeds and
       writes `root/f.txt`. The bytes on disk still equal those read at open, so this holds
       after T-027 too.
  - Kills T-008 r2 mutant M4 at acceptance level (the reviewer checks).
- **AC-4 (item 4, TG-4; guard; `guard_t036_workspace.rs::vanished_root_is_refused` and
  `guard_t036_cli.rs::vanished_root_is_refused_through_control`).**
  - *Cases.*
    - (i) `root` renamed to `root.away`;
    - (ii) the root at `work/p/root` with `p` renamed to `p.away`;
    - (iii) `root` removed with `remove_dir_all`. Nothing is put in its place in any case.
  - *After the change.*
    - `read`/`doc.open` of `f.txt` and of `missing.txt` → `<p>: resolves outside the root` /
      `outside_root`;
    - `is_file("f.txt")` false;
    - `write_atomic("new.txt", …)` → `OutsideRoot`;
    - the save of the document opened before → `cannot save f.txt: resolves outside the
      root`, still dirty (`doc.info` `dirty:true`);
    - the snapshot of `work/` is unchanged: no `work/root` created, the renamed-away `f.txt`
      keeps its bytes, and no `new.txt` and no `.kiln-tmp-*` anywhere.
  - *Moved back.* Then (i) and (ii) are renamed back, and the save succeeds into `root/f.txt`.
  - Kills mutant M5 (failed re-resolution taken as success) at acceptance level.
- **AC-5 (item 5, N-1; acceptance; `acceptance_t036_workspace.rs::dot_after_a_file_in_a_link_target_is_not_found`
  and `acceptance_t036_cli.rs::dot_after_a_file_in_a_link_target_is_not_found_through_exec`).**
  - *Fixture.* `real/t.txt` = `b"target\n"`; links `dot -> real/t.txt/.`,
    `dots -> real/t.txt/./`, `dd -> real/.`, `mid -> real/./t.txt`,
    `od -> ../outside/secret.txt/.`.
  - *Rust API.*
    - `read("dot")` → `dot: not found`; `write_atomic("dot", b"X")` → `dot: not found`;
      `is_file("dot")` false; `read("dots")` → `dots: not found`.
    - `read("dd")` → `dd: not a regular file`; `read("mid")` → `b"target\n"`;
      `read("od")` → `od: resolves outside the root`.
    - Afterwards `real/t.txt` is still `b"target\n"` and the snapshot is otherwise unchanged
      (no `.kiln-tmp-*`).
  - *CLI.* `kiln-cli exec doc.open '{"path":"dot"}' --root work/root` → exit 2, stdout empty,
    stderr one JSON object with `kind` `not_found` and `message` `cannot open dot: not found`.
  - Fails on base (`dot` is read and written through).
- **AC-6 (documents; acceptance, fails on base;
  `acceptance_t036_cli.rs::paths_doc_states_no_unreadable_root_limit`).**
  - `docs/protocol/paths.md` contains neither the sentence "Kiln cannot keep it open" nor the
    phrase "except in the case of an unreadable root". Both are looked up whitespace-
    insensitively.
  - It contains `bad --root`.
  - The reviewer checks the rest of the document sentence by sentence against the code
    (Documentation above).
  - Both phrases are in `paths.md` on base (:21–22, :96, :98), so this fails there. Under
    Documentation's exception (another accepted-but-unheld case), the residual bullet for
    that case must not reuse either phrase.

Scorecard: the test author appends the AC-1, AC-3, AC-4 and AC-5 test names, and the two
AC-2 files' tests listed as guards, to `scorecard/items/ws.path-confinement.toml`. Then the
test author regenerates `docs/scorecard.md`. The item's status is derived, so the evaluator
confirms it again.

## Out of scope and implementer notes
- **TG-2 (accepted, no test).** The root identity compares device and inode, but no test
  replaces the root by a directory on another device that has the same inode number. Building
  that needs a second mount (a bind mount or another file system), which the gate cannot
  create without privileges and tools it does not have. T-008 r2 mutant M3 (inode-only
  comparison) therefore survives. It is recorded here as accepted, with low practical risk
  per the reviewer. The implementer must not weaken the comparison.
- Races during one call (D-3); hard links; Windows; `MemoryFs`.
- A root with read but no search permission (mode 0644 directory). Construction succeeds and
  requests fail with `io_error` today. Unchanged and not pinned.
- `docs/control-protocol.md` (Q-2). The `kiln-control` and `kiln-cli` sources: their
  existing `root_error` arm prints the new refusal.
- T-008's acceptance and guard files are not edited or removed. Its four probabilistic
  root-removal tests stay as they are (Q-3).
- Specified: the outcomes above. Left to the implementer: how the root is kept, how
  readability is established at construction, and how the walk handles `.`. Do not pin
  `/proc` paths or file-descriptor details in tests.
- Implementer's own tests: unit tests in `disk.rs`, integration tests in `confinement.rs`.
  Variant matches belong there. Unprivileged cases there follow AC-1's identity rules (a
  child run as U). Determinism: no clock or thread timing in `DiskFs`, and no new
  dependency.

## Conflicts and dispatch
- Never in flight with **T-019** (draft, depends on T-008). Both edit `crates/kiln-workspace/src/disk.rs`,
  and T-019's `list_dir` of the root is affected by the unreadable-root decision.
- **T-027** (draft, depends on T-008) shares no file in `touches`: `document.rs`, `doc.rs`
  and a `reason.rs` mapping row, none of which this task touches. It may run beside this task.
  Its unforced-save check returns `OutsideRoot` unchanged (T-027 Interface), and AC-3 and
  AC-4 keep the on-disk bytes equal to those read, so neither order of merge breaks the other.
- `vfs.rs` and `reason.rs` are not touched, so there is no conflict with T-010 (merged),
  T-020 or T-027 on `reason.rs`.
- `docs/protocol/paths.md` is edited by no other planned task.
- `docs/scorecard.md` is generated (regenerate on conflict).
- T-033 (draft) changes how the join keys test targets. The file names here are unique
  either way.

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
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
