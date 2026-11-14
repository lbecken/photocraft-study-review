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
  path that is open in the session (the same path text) reads nothing. It is refused with
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
- Unix only.
