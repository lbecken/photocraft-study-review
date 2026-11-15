# The tabs

T-058. The Kiln UI that `kiln script` runs (`docs/protocol/ui.md`) shows, above the editor
(`docs/protocol/ui-editor.md`), one **tab** per open document. A click on a tab makes its
document current; a tab's close button closes it with `doc.close`, and a document with unsaved
changes is closed only by a second, explicit click on `Discard`. The entries of the shell's
`Documents` list switch documents like tabs. Everything here is observed through `kiln script`;
the window of T-065 shows the same UI.

## What it shows (nodes)

- **The tab list**, present exactly when at least one document is open: a node of role
  `tab_list` labelled `Tabs`, inside a node of role `scroll_view` that contains no shell node
  (`Path`, `Open`, `Message`, `Documents`), no `Title`, no `Editor` and no project-panel node.
- **One tab per open document**, in id order (the order of `doc.list`), each a node of role
  `tab`:
  - `label`: the document's path exactly as `doc.list` reports it;
  - no `value`;
  - its painted text is its path;
  - `toggled: true` for the current document's tab, `toggled: false` for every other.
- **With each tab**, after it in the snapshot's depth-first order and before the next tab:
  - when `doc.list` reports the document `dirty`: a node of role `label` with `value`
    `modified`, painting exactly `modified`;
  - always: a node of role `button` labelled `Close <path>`, painting no text (its mark is drawn
    as a shape);
  - only after a close of that document was refused with `unsaved_changes`, and until the
    document is closed or `doc.list` reports it clean: a node of role `button` labelled
    `Discard <path>`, painting exactly its label.
- `Documents` keeps its nodes (role `list`, entries of role `label` with the path as `value`);
  its entries are now clickable.

How tabs wrap, their size and the height of the strip are not part of this interface. The strip
is at most a few rows tall and scrolls (`scroll` on `Tabs` or on any tab), so with many
documents some tabs are `offscreen`: scripts find tabs by role and label and `scroll` before
clicking an `offscreen` one.

T1. Every open document has a tab, in id order, labelled with its path; the current document's
tab is reported `toggled: true`.

## UI requests that act on it

All are the requests of `docs/protocol/ui.md`; the tabs change nothing in how they work.

| Request | Effect |
|---|---|
| `click` on a `tab` | its document becomes current; on the current tab, nothing changes |
| `click` on a `Documents` entry (`{"role":"label","value":<path>}`) | the same as a click on that document's tab |
| `click` on `Close <path>` | `doc.close {"doc": <id>}` |
| `click` on `Discard <path>` | `doc.close {"doc": <id>, "discard": true}` |
| `scroll` on `Tabs` or on a tab | scrolls the strip |

A secondary click or a click with modifiers on a tab, a close or discard button or a
`Documents` entry, and a click with `at` on a tab, does what a primary click does. No tab, close or discard button takes keyboard
focus by itself. As with any click outside the editor, a click on one of them leaves the editor
unfocused.

T2. A click on a tab, or on a document's entry in `Documents`, makes that document current. It
sends no request that changes anything; the views then re-read the document with read-only
requests, which are in the session log.

T3. A tab's close button closes its document with `doc.close`. A document with unsaved changes
is not closed: `Message` begins with `unsaved_changes` and a `Discard` button appears, which
closes it with `discard: true`. It stays until the document is closed or saved.

A successful close from a tab, with or without `discard`, clears `Message` and removes the tab;
when the closed document was current, the open document with the highest id becomes current
(`docs/protocol/ui-editor.md`, "The current document"). Any refusal of `doc.close` changes
nothing else and sets `Message` to begin with its kind. There is no dialog and no confirmation
beyond the click on `Discard`.

T4. A node with the value `modified` follows the tab of every document `doc.list` reports as
dirty.

## Frames, reads and the log

T5. The tabs read `doc.list` in the frames after a change and never on an idle frame.

- A **change** is a successful request of any source other than a read, or a change of the
  current document (a tab or `Documents` click). In the first frame after one, while a document
  is open, the tabs send one read-only `doc.list`; how many reads one action causes is not part
  of this interface. With no document open they send nothing.
- On an idle frame (no change since the last read) and on a `scroll` they send nothing. A script
  of control requests only runs no frame after its first request, so its `--log` stays
  `Host::log_script()`.
- `doc.close` (with or without `discard`) is sent only on a click on `Close` or `Discard`, in
  the frame that handles the click. A switch is recorded in the log only by the views' read-only
  requests (all `mutating: false` in `docs/commands.json`).
- The click's request or switch happens in the frame that handles it; the views show its effect
  from that frame or the next, within the action's settling. A refused read is not shown.

The tabs read and write no file: closing goes through `doc.close`, which never touches the disk.
Paths are shown as `doc.list` reports them, with `/` on every platform; the evidence is the
Linux gate, and for macOS and Windows it comes from CI once the platform jobs run.

## Limits

- T6. Every tab is laid out on every frame, so the time of a frame grows with the number of open
  documents: with 300 open documents, a script that opens them and runs 600 idle frames took
  5.4 to 5.7 s (three runs) in a debug build on a 4-core Linux machine.
- With 300 open documents at the default size, most tabs are `offscreen`; a `click` on an
  `offscreen` tab is refused `not_actionable`, and every tab can be clicked after scrolling
  `Tabs`. The strip is taller than one `scroll` step, so a step skips no row of tabs.
- A long path is painted whole on one row: tabs do not shorten paths.
- T7. A path that contains a line break is painted on two rows; its label is exact and a click
  on it works.
- A path that begins with right-to-left text has an exact `label`; its rows fall under
  `docs/protocol/ui.md`'s Limits.
- Not offered: reordering, pinning or dragging tabs; keyboard shortcuts for switching or
  closing; middle-click close; closing several tabs at once.
