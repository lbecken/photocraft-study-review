# The project panel

T-057. The Kiln UI that `kiln script` runs (`docs/protocol/ui.md`) shows, beside the editor
(`docs/protocol/ui-editor.md`), a **project panel**: the project tree of the session root as
`project.tree` lists it (`docs/protocol/project.md`), whose directories expand and collapse and
whose other entries open as the current document on a click. Everything here is observed
through `kiln script`; the window of T-065 shows the same UI.

## What it shows (nodes)

- **The toggle**: a node of role `button` labelled `Project`, `toggled: false` while the panel is
  collapsed and `toggled: true` while it is expanded. Its one painted row is exactly `Project`
  (a marker, if any, is drawn as a shape).
- **When expanded**: a node of role `tree` labelled `Project files`, inside a node of role
  `scroll_view` that contains no shell node (`Path`, `Open`, `Message`, `Documents`), no `Title`
  and no `Editor`. Below it, in the snapshot's depth-first order, one node of role `tree_item`
  per shown entry:
  - `label`: the entry's root-relative path exactly as `project.tree` reports it;
  - `value`: the entry's name (its last path segment), which is also its painted text;
  - `toggled`: only on an entry of kind `dir`, `true` when expanded, `false` when collapsed;
    an entry of kind `file`, `symlink` or `other` has none.
  The shown entries are the root's entries and, after each expanded directory, its own shown
  entries, recursively: `project.tree`'s preorder restricted to what is shown. How the nodes are
  nested, indented or drawn is not part of this interface.
- **A truncated listing**: a node of role `label`, without `label`, whose `value` begins with
  `truncated` (below).
- **When collapsed**: the toggle only; no `tree`, no `tree_item`, no marker.
- The panel never shows the root's host path or name.

The panel's position and width are not part of this interface: scripts find its nodes by role
and label and `scroll` before clicking an `offscreen` entry.

## UI requests that act on it

All are the requests of `docs/protocol/ui.md`; the panel changes nothing in how they work.

| Request | Effect |
|---|---|
| `click` on `Project`, collapsed | expands it: one `project.tree {}` read |
| `click` on `Project`, expanded | collapses it; sends nothing and forgets every directory's expanded state |
| `click` on a `tree_item` with `toggled: false` | expands it: one `project.tree {"path": <label>}` read |
| `click` on a `tree_item` with `toggled: true` | collapses it; sends nothing; its descendants are no longer shown and their expanded state is forgotten |
| `click` on a `tree_item` without `toggled` | `doc.open {"path": <label>}` |
| `scroll` on `Project files` or on an entry | scrolls the panel's list |

A secondary click, a click with modifiers or with `at` on an entry does what a primary click
does. Keyboard navigation in the tree is not offered. No entry takes keyboard focus by itself.

P1. The panel starts collapsed and sends nothing until the user clicks it: a script of control
requests only writes the same log as `kiln-cli run`.

P2. Expanding the root or a directory sends one `project.tree` read (for the root without
`path`, for a directory with its path), and the entries shown under it are that read's entries
one level below it, in its order. Collapsing sends nothing.

Both reads use the default `limit`. The read is sent in the frame that handles the click, and
the new entries are shown from that frame or the next. Like every successful request the reads
are in the session log (`docs/control-protocol.md`); the panel sends no other read, none at
start, none after a script's control request, none after the UI's own opens or saves and none on
an idle frame.

P3. A click on an entry that is not a directory opens it with `doc.open` and makes it the
current document; a refusal leaves everything as it was and `Message` begins with its kind.

A successful open clears `Message`; an open that answers `already_open: true` also makes the
document current (`docs/protocol/ui-editor.md`, "The current document"), and the editor shows it
from the next frame. A FIFO or a device is refused (`not_a_file`) without blocking. A refused
read (`not_found`, `not_a_file`, `outside_root`, `io_error`, `invalid_path`) is not logged,
leaves the directory (or the root) collapsed and sets `Message` to begin with its kind, because
the user asked for it; the editor's own background reads never set `Message`. A successful
expand leaves `Message` as it was; only a successful open or save clears it.

## When the tree on disk changes

P4. The panel does not watch the disk: what it shows is what the last read of each expanded
directory returned, until that directory is collapsed and expanded again.

A click on an entry that has since vanished or changed kind is answered by `doc.open` or
`project.tree` as they answer it (for example `not_found`); the refusal shows in `Message` and
the stale entry stays until the next expand. A file that the UI's own `doc.save` creates appears
after the next expand of its directory.

## Truncated listings

P5. When the read that listed a directory was truncated, a node whose value begins with
`truncated` comes after the last entry shown below that directory (after the descendants of its
last entry, when that entry is an expanded directory), and entries of that directory after the
cut are not shown.

Which entries are shown is then what that read listed (`project.tree`'s rule 6: preorder up to
`limit`). Nested truncated listings show one marker each, innermost first: with the root, `a`
and `a/b` expanded and all three reads truncated, the markers of `a/b`, `a` and the root follow
one another after the last entry shown below `a/b`. The markers are identical; a script tells
them apart only by position.

## Labels, targets and paths

P6. An entry's label is its root-relative path; with the panel expanded, a target given by
label alone can also match an entry (a file named `Open` at the root matches
`{"label":"Open"}`): give `role` as well.

P7. Paths are shown as `project.tree` reports them, with `/` on every platform.

Entries are in `project.tree`'s byte order, with no case folding; labels are used unchanged as
the `path` of `doc.open` and `project.tree`. The panel adds no platform difference; any
difference in what is listed is `project.tree`'s (`docs/protocol/project.md`). The evidence is
the Linux gate; for macOS and Windows it comes from CI once the platform jobs run.

## Files and confinement

The panel reads and writes no file itself: everything goes through `project.tree` and
`doc.open`, confined to the root (`docs/protocol/paths.md`). Links are listed and never followed
by `project.tree`; special files are never opened by it and are refused by `doc.open` without
blocking. It reads only on a user's click.

## Limits

- Every shown entry is laid out on every frame, so the time of a frame grows with the number of
  entries shown. Measured by the reviewer (`reviews/T-057-r1.md`, finding 4) in a debug build,
  with a directory of 50,000 entries shown: 18.0 s for the expand, then 0.76 s for each later
  frame (475.9 s for 600 idle frames). The read itself is one `project.tree` of up to 50,000
  entries. No time bound is stated.
- The rows of a name that begins with right-to-left text can lack chars (`docs/protocol/ui.md`,
  Limits); its `label` and `value` are exact.
- A name wider than the panel is cut at the panel's right edge: its row then holds only the
  chars whose centre lies inside the panel; its `label` and `value` are exact. How many chars
  remain depends on the chars, since their widths differ.
- A name that contains a line break is painted on two rows (`new` and `line` for `new\nline`),
  so its rows are not one row equal to its `value`; its `label` and `value` are exact and a
  click opens it.

## Example

A root `proj/` with `.gitignore` (`*.log`), `a.log`, `target/t`, `src/a.rs`, `dir with space/é.txt`
and `b.txt`, and `kiln script s.jsonl --root proj`:

```text
{"id":"c","ui":"click","target":{"role":"button","label":"Project"}}
{"id":"d","ui":"click","target":{"role":"tree_item","label":"src"}}
{"id":"o","ui":"click","target":{"role":"tree_item","label":"src/a.rs"}}
{"id":"s","ui":"snapshot"}
```

After `c` the shown entries are, by label, `.gitignore`, `b.txt`, `dir with space` (`toggled:
false`) and `src` (`toggled: false`), not `a.log`, `target` or `dir with space/é.txt`. After
`d`, `src` is `toggled: true` and `src/a.rs` (value `a.rs`) follows it. After `o` the snapshot
`s` shows `Title` beginning with `src/a.rs`. The log holds `project.tree {}`,
`project.tree {"path":"src"}`, `doc.open {"path":"src/a.rs"}`, then the editor's reads.
