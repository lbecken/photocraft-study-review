# Paths and the root

Every `path` in a request names an entry below the root (`--root`, or the current directory): a
file for `doc.open` and for the file a `doc.save` writes, a directory for `project.tree`
(`docs/protocol/project.md`). This page states what Kiln guarantees about where
such a path may lead on disk (T-008). It applies to the on-disk file system; the in-memory one
(`--memory`, tests) has no symlinks or special files.

## The root

The root is resolved once, when the session starts: if `--root` is a symlink to a directory,
that directory is the root. A root that does not exist, or is not a directory, is refused as
before (the messages end in `: not found` and `: not a directory`). A root that Kiln cannot open
for reading is refused too, whatever its other permissions (for example a directory whose owner
may search it but not list it, mode `0311`, or one of mode `0000`): Kiln keeps the root open
while the session runs (below), and it does not start a session on a root it cannot keep open.
`kiln-cli` then exits 2 with a `bad --root` message on stderr, and no request runs.

The root is that directory, not its path. Kiln keeps it open while the session runs, only so
that no other directory can take its identity. Before each request that uses the file system
(`doc.open`, also of a path that is already open, `doc.save`, `project.tree` and
`nav.workspace-symbols`), Kiln resolves `--root` again, as it was given, and goes on only if it
still leads to that same directory. Once the root path stops leading to the root directory (the
root, or a directory above it, was replaced, for example by a symlink to another directory),
the session refuses every path with `outside_root`, at open and at save, including the save of
a document opened before and `doc.open` of a path that is already open; nothing is read or
written in the other directory. `project.tree` and `nav.workspace-symbols` are refused with
`outside_root` as well. The same refusal is given while the root path leads nowhere (the root
renamed or removed with nothing in its place). If the path leads to the root directory again
(moved back, or a link at that path pointed at it), the session works there again.

On Windows (T-073) Kiln holds the root open in the same way, and the identity it compares is
that of the held directory: the directory `--root` leads to is the root when it has the same
volume serial number and the same 64-bit file index as the held one. Measured on the
`windows-2025` runner in CI run 44268710587 (`tasks/ci-runs/T-073-round-1.md`):
- The held root itself can be renamed while a session runs; every call then answers
  `outside_root`, and once the root is moved back the session works there again, as on Linux and
  macOS (`crates/kiln-workspace/tests/guard_t073.rs::acw2_root_replaced_at_same_path_is_outside_root`;
  on macOS listed on the green `gate-macos` job of CI run 44358453526, `tasks/ci-runs/T-074-round-1.md`, "Addendum for T-093").
- **A difference from Linux and macOS:** while a session holds the root, Windows refuses to rename
  a directory above it (`PermissionDenied`, OS error 5), and the session goes on working on its
  root. On Linux and macOS that rename succeeds and the session then refuses every path with
  `outside_root` (`crates/kiln-workspace/tests/guard_t073.rs::acw2_root_parent_replaced_is_outside_root`,
  which passed on all three in that run; on macOS by name: listed on the green `gate-macos` job of
  CI run 44358453526, `tasks/ci-runs/T-074-round-1.md`, "Addendum for T-093", and its branch for Linux and macOS fails unless the rename succeeds).
- The held root can be removed while a session runs, empty or with a file in it; then a read, a
  save, a check whether a file exists, a listing and `check_location` each answer `outside_root`
  (a file is never reported as existing), also after a new directory takes its name, as on Linux
  and macOS. Measured on the runner in CI run 44290286460 (`tasks/ci-runs/T-074-probe-1.md`:
  `check_location`, `list_dir` and `read`) and in CI run 44307677646
  (`tasks/ci-runs/T-074-early-1.md`: all five calls, and for the root removed with a file the
  save of a document opened before, `guard_t074.rs::awa4_root_removed_while_held`; on macOS listed on
  the green `gate-macos` job of CI run 44358453526, `tasks/ci-runs/T-074-round-1.md:558`). Not run on Windows: a directory above the root
  replaced (Windows refused the rename).
- **A difference from Linux and macOS:** a `..` in `--root` is resolved by its text before any
  link is followed: with `link` a junction, `--root link\..\b` is the `b` beside `link`, not the
  one beside the directory `link` leads to (CI runs 44290286460 and 44307677646,
  `guard_t074.rs::awa4_dotdot_after_a_junction_in_the_root_text`). On macOS, with `link` a symbolic
  link, it is the `b` beside the directory `link` leads to (the same test's branch for Linux and
  macOS, listed on the green `gate-macos` job of CI run 44358453526, `tasks/ci-runs/T-074-round-1.md:558`).
- A junction or symbolic link given as `--root` leads to the root; if it is pointed elsewhere
  while a session runs, every call answers `outside_root`, and pointed back the session works
  there again, as on Linux and macOS (CI run 44307677646, `tasks/ci-runs/T-074-early-1.md`:
  `guard_t074.rs::awa4_root_given_through_a_junction_and_retargeted` and
  `::awa4_root_given_through_a_directory_link_and_retargeted`; on macOS, where both plant a
  symbolic link, both are listed on the green `gate-macos` job of CI run 44358453526, `tasks/ci-runs/T-074-round-1.md:558`).
- A limit, read and not measured: on a file system that does not keep the 64-bit file index
  unique (ReFS is documented so), two different directories can compare equal, so a directory
  that replaced the root could be taken for it (a 128-bit identity is T-094's).

On Windows (T-074) the rules below hold with this difference: no link is followed. Every entry
on the way that is a reparse point — a symbolic link to a file or a directory, a junction, or any
other reparse point — makes the path `outside_root`, wherever it leads, and nothing beyond it is
read or written (`crates/kiln-workspace/tests/guard_t074.rs`, its four `awa1_*` tests, on the `windows-2025` runner in
CI run 44307677646, `tasks/ci-runs/T-074-early-1.md`, for file links relative and absolute,
directory links and junctions leading inside, outside and nowhere; reparse points other than
links and junctions: unchecked, the runner cannot make one). Names
that Windows gives a meaning, and case, are not yet stated for Windows (T-095).

## Lexical refusals (unchanged)

Before the file system is consulted, a path is refused as `rejected` / `invalid_path` if it is
empty, absolute (leading `/`), has a `..` or `.` segment or an empty segment (`a//b`, a
trailing `/`), or contains a backslash or a NUL character. This check comes first:
`doc.open {"path":"../x"}` is `invalid_path`, never anything below.

## Location

The *location* of a path is the entry reached by walking it from the root one component at a
time, as the kernel does: every symbolic link on the way is followed, a relative link target is
taken from the link's directory, an absolute one from `/`, and a `..` in a link target goes to
the parent of the directory actually reached (never by shortening the text). If a component
before the last does not exist or is not a directory, the walk stops there.

The location is *inside* the root if it is the root or below it, compared component by
component: a sibling directory `<root>x` is not inside `<root>`. Only the final location
counts: a chain of links that leaves the root and comes back inside is inside.

## The rule

At open and at save (and whenever Kiln checks whether a file exists), except as the last item
below says for a path that is already open:

- **A path whose location is outside the root is refused** as `rejected` / `outside_root`, with
  the message `cannot open <path>: resolves outside the root` or
  `cannot save <path>: resolves outside the root`. Unless the file system is changed while
  the request runs (see Residual limits), nothing outside the root is read, created, replaced
  or removed, and no temporary file is left anywhere. This holds for links to files or
  directories outside, absolute links, chains of links, links whose target uses `..` after a
  link to a directory, and dangling links that point outside. A dangling link whose walk ends
  outside the root is `outside_root`, not `not_found`. (Only the final location counts, so a
  chain that leaves the root and comes back can show, by `not_found` versus `outside_root`
  (for a path that is already open, by `already_open` versus `outside_root`), whether a
  directory it passes through outside exists.)
- **Symlinks whose location stays inside the root are followed.** A save through such a link
  replaces the file it leads to and keeps the link; a save through a dangling link inside the
  root creates its target, as a save of a new file does.
- **Directories, FIFOs, sockets and devices are refused** as `rejected` / `not_a_file`
  (`cannot open <path>: not a regular file`, `cannot save <path>: not a regular file`), without
  opening them, so a FIFO present when the request is checked never makes it wait (a FIFO
  swapped in while the request runs can; see Residual limits). A refused save leaves the entry
  as it was.
- **Each request is checked again.** What `doc.open` saw does not authorise a later
  `doc.save`: if the file, a directory on its path, the root or a directory above the root
  was replaced in between by a link that leads outside, or the file by a FIFO, the save is
  refused, and the document stays modified.
- **A path that is already open is checked again, for where it leads only.** A `doc.open` of a
  path that is open in the session (the same path text) reads nothing. On macOS, on the runner's
  volume, which folds case, `doc.open` of `A.rs` while `a.rs` is open opens a second document of
  the same file (`already_open: false`), and a save through it changes what `a.rs` holds
  (`apps/kiln-cli/tests/macos_t093.rs::reopen_with_other_case_is_a_new_document_where_case_folds`,
  listed on the green `gate-macos` job of CI run 44381094009, `tasks/ci-runs/T-093-round-1.md`; the job keeps no per-test output on a green run, so nothing beyond what the test asserts is shown); on a case-sensitive macOS volume: unmeasured on macOS. It is refused with
  `outside_root` when the root path no longer leads to the root (also when it can no longer be
  followed, for example because a directory above the root cannot be searched, which refuses a
  first open, a save and `project.tree` the same way) or the path's location is now outside the
  root, and with `io_error` when its location cannot be determined inside the root (for example
  more than 40 symlinks, or a directory on the way that cannot be searched). Otherwise it
  returns the open document unchanged with `already_open: true`, even if the file has since
  been deleted, replaced by a directory, a FIFO or a link that stays inside the root, made
  unreadable, or no longer decodes. A refused re-open leaves the document open and unchanged.

When several of these apply, the first in this order is reported: `invalid_path`,
`outside_root`, `not_found`, `not_a_file`, any other failure (`io_error`). For a path that is
already open, only `invalid_path`, `outside_root` and `io_error` can be reported, in this
order. A path that follows more than 40 symlinks is `io_error` (`too many levels of symbolic
links`). A link whose target ends in `/` must lead to a directory, as the kernel requires; a
file reached that way is `not_found`.

Messages name the path as given in the request, never the root, a host path or a link target.
`doc.open` and `doc.save` refusals leave the session as it was (a failed `doc.save` keeps the
document dirty); in `kiln-cli run` the exit code is 3, and `kiln-cli exec` exits 2 with the
error object on stderr.

## Residual limits

- **Hard links.** A hard link inside the root to a file outside it is an ordinary file inside
  the root; it cannot be told apart by its path, and it is followed.
- **Changes during a request.** The check of the root, the walk of the path and the following
  open or rename are separate steps. A path changed by another process *while* a request runs
  (for example the root, a directory or the file swapped for a symlink between the check and
  the write) can slip through, so a file outside can be read or written and a temporary file
  can be left behind, and a FIFO swapped in during that window can make the request block.
  Changes made between two requests, to the path or to the root and the directories above it,
  are always caught: the next request checks the path again (`doc.open` of a path that is
  already open checks only where it leads, see The rule).
- **Platforms.** The rules of this page hold on Linux and macOS as written, with the macOS case
  difference stated in The rule (a path already open is recognised by its text). On macOS: the tests
  of the scorecard item `ws.path-confinement` (`scorecard/items/ws.path-confinement.toml`) ran and
  passed on the green `gate-macos` job of CI run 44358453526 (`tasks/ci-runs/T-074-round-1.md`,
  scorecard step `macos: 64 items; 216 tests ran; 3 registered absent here`, the 3 being
  `ui.window`'s); the bound of 40 symlinks and its message
  (`crates/kiln-workspace/tests/confinement.rs::walk_corner_cases`) and a root given through a
  symlink (`crates/kiln-workspace/src/disk.rs`, `disk::tests::failed_rename_removes_own_temp_and_keeps_target`
  and `disk::tests::root_given_through_a_link_that_is_retargeted`) by name, listed on that job
  (`tasks/ci-runs/T-074-round-1.md`, "Addendum for T-093"). On Windows they hold with the differences stated in The root and The rule (T-073,
  T-074); names and case on Windows are not yet stated (T-095).
- On Windows every reparse point on the way is refused, so files that are reparse points without
  being links — for example cloud placeholder files of a synced folder or deduplicated files —
  cannot be opened or saved; they answer `outside_root` (unchecked: the runner cannot make them).

## For the platform report

On Windows Kiln follows no link or junction inside a project and refuses every reparse point as
outside the root, including cloud placeholder and deduplicated files (those two unchecked on the
hosted runner).
