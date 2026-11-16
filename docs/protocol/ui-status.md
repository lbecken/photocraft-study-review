# The status bar

T-059. The Kiln UI that `kiln script` runs (`docs/protocol/ui.md`) shows, below the editor
(`docs/protocol/ui-editor.md`), a status bar for the current document ("The current document"
in `docs/protocol/ui-editor.md`). Everything here is observed through `kiln script`; the window
of T-065 shows the same UI.

The status bar shows, for the current document, the caret's line and column (as `Ln <n>` and
`Col <n>`, both counted from 1; the column in chars, as `cursor.get` counts it), the encoding as
`doc.info` names it, and the line ending as `doc.info` names it, followed by ` (mixed)` when the
line endings are mixed.

## What it shows (nodes)

With a current document, four nodes of role `label`, each with an accessible name (its `label`)
and a `value` equal to the text it paints on one row:

| `label` | `value` | Example |
|---|---|---|
| `Line` | `Ln ` and `cursor.get`'s `line` plus one, in decimal | `Ln 2` |
| `Column` | `Col ` and `cursor.get`'s `col` plus one, in decimal | `Col 6` |
| `Encoding` | `doc.info`'s `encoding`: `utf-8`, `utf-8-bom`, `utf-16le` or `utf-16be` | `utf-8` |
| `Line ending` | `doc.info`'s `line_ending` (`lf`, `crlf`, `cr`, `none`), followed by ` (mixed)` when `mixed_line_endings` is true | `lf (mixed)` |

- The caret is the selection's head (`cursor.get`'s `head`): with a selection, `Line` and
  `Column` are those of its head, not of its anchor.
- The column counts chars (Unicode scalar values) from the start of the line: a tab, a
  combining mark, a zero-width joiner and each char of a ligature count as one each, and an
  emoji as one (two UTF-16 units). The document's text is never shown in the bar, so
  right-to-left or shaped text changes only the count.
- The values are those of the current document, read after the change that made it current: a
  click on a tab or on a `Documents` entry, a `doc.open` (also one that answers
  `already_open: true`), and the close of the current document (the open document with the
  highest id becomes current) each show the newly current document's values.
- The bar shows nothing else: no dirty state (`Title` and the tab show it), no selection size, no
  language or syntax mode.
- With no document open the bar shows nothing.

Where the bar sits: at the bottom of the document area, below the tab strip, `Title`, the project
panel and the editor; it is one row tall. Its layout, separators and spacing are not part of
this interface: scripts find its nodes by role and label.

## UI requests that act on it

None. The bar is a display: a click on it does nothing. A click on one of its nodes, with either
button, with modifiers or with `at`, changes no node's value and sends no request. It takes no
focus: `focus` on one of its nodes is refused `not_actionable`, as for any node that takes no
text input. As with any click outside the editor, a click on it leaves the editor unfocused.

## Frames, reads and the log

The bar reads `cursor.get` and `doc.info` in the frame after a change and never on an idle
frame; its reads are in the session log.

- A **change** is what it is for the tabs (`docs/protocol/ui-tabs.md`, "Frames, reads and the
  log"): any successful request that reaches the session from outside the UI's own reads (the
  editor view's and the panels' read-only requests), a script's read-only control request
  (`doc.list`, `buffer.text`, `project.tree`, …) included, or a change of the current document (a
  tab or `Documents` click). The bar then sends `cursor.get` and `doc.info` of the current
  document, both read-only (`mutating: false` in `docs/commands.json`), in the first frame after
  the change; the editor view sends its own reads as well, and how many reads one action causes
  is not part of this interface.
- On an idle frame (no change since the bar's last read), on a `scroll`, on a click that sends no
  request, and with no document open it sends nothing. A script of control requests only runs
  no frame after its first request, so its `--log` stays `Host::log_script()`.
- The values show the state after the change from the frame of the read on, within the
  action's settling.

The bar reads and writes no file: its two requests are read-only and touch no file. The values
are the commands' own, with no platform difference; the evidence is the Linux gate, and for
macOS and Windows it comes from CI once the platform jobs run.

## Limits

- In a small window with many tabs the bar can be covered: at 800x320 with 300 open documents
  its values are not painted (R-295). There the four nodes exist with their values and are not
  `offscreen`, but none of them has `rows`. Measured (T-059, a debug build on Linux; documents
  `f000.txt`, `f001.txt`, … opened by control requests, then two frames): at `--size 800x320`
  the four values are painted with 1, 5, 10, 20 and 22 open documents and not painted with 23,
  24, 25, 28, 30, 32, 35, 38, 40, 45, 50, 100, 200 and 300; at the default size they are painted
  with every count measured, up to 300. Other sizes and path lengths were not measured.
