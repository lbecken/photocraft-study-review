# Paths and the root

Every `path` in a request (`doc.open`, and the file a `doc.save` writes) names a file below the
root (`--root`, or the current directory). This page states what Kiln guarantees about where
such a path may lead on disk (T-008). It applies to the on-disk file system; the in-memory one
(`--memory`, tests) has no symlinks or special files.

## The root

The root is resolved once, when the session starts: if `--root` is a symlink to a directory,
that directory is the root. A root that does not exist, or is not a directory, is refused as
before (the messages end in `: not found` and `: not a directory`).

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

At open and at save (and whenever Kiln checks whether a file exists):

- **A path whose location is outside the root is refused** as `rejected` / `outside_root`, with
  the message `cannot open <path>: resolves outside the root` or
  `cannot save <path>: resolves outside the root`. Nothing outside the root is read, created,
  replaced or removed, and no temporary file is left anywhere. This holds for links to files or
  directories outside, absolute links, chains of links, links whose target uses `..` after a
  link to a directory, and dangling links that point outside: a dangling link to a missing file
  outside is `outside_root`, not `not_found`, so a refusal reveals nothing about what exists
  outside the root.
- **Symlinks whose location stays inside the root are followed.** A save through such a link
  replaces the file it leads to and keeps the link; a save through a dangling link inside the
  root creates its target, as a save of a new file does.
- **Directories, FIFOs, sockets and devices are refused** as `rejected` / `not_a_file`
  (`cannot open <path>: not a regular file`, `cannot save <path>: not a regular file`), without
  opening them, so a FIFO never makes a request wait. A refused save leaves the entry as it was.
- **Each request is checked again.** What `doc.open` saw does not authorise a later
  `doc.save`: if the file was replaced in between by a link that leads outside, or by a FIFO,
  the save is refused, and the document stays modified.

When several of these apply, the first in this order is reported: `invalid_path`,
`outside_root`, `not_found`, `not_a_file`, any other failure (`io_error`). A path that follows
more than 40 symlinks is `io_error` (`too many levels of symbolic links`).

Messages name the path as given in the request, never the root, a host path or a link target.
`doc.open` and `doc.save` refusals leave the session as it was (a failed `doc.save` keeps the
document dirty); in `kiln-cli run` the exit code is 3, and `kiln-cli exec` exits 2 with the
error object on stderr.

## Residual limits

- **Hard links.** A hard link inside the root to a file outside it is an ordinary file inside
  the root; it cannot be told apart by its path, and it is followed.
- **Changes during a request.** The check and the following open or rename are separate steps.
  A path changed by another process *while* a request runs (for example a directory swapped for
  a symlink between the check and the write) can slip through, and a FIFO swapped in during that
  window can make the request block. Changes made between two requests are always caught.
- Unix only.
