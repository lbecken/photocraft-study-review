# The editor view

T-056. The Kiln UI that `kiln script` runs (`docs/protocol/ui.md`) shows, beside the shell, an
**editor view** of the current document: its text with syntax colours, a caret and selection,
and scrolling. Typing, deleting, `Enter`, caret movement, undo, redo and save work from keys and
pointer clicks, and each is a registry command dispatched through the session (ADR-0008
decision 5). Everything here is observed through `kiln script`; the desktop window that shows
the same UI is T-065.

## The current document

The editor view shows one document, the **current document**: the one most recently opened by a
successful `doc.open` from any source (a start path, the UI, a script's control request), a
`doc.open` that answers `already_open: true` included; if it is closed, the open document with
the highest id; with none open, no `Title` and no `Editor` node is shown.

## What it shows (nodes)

- A node labelled `Title` whose `value` begins with the current document's path; the part of
  the value after the path contains the word `modified` exactly when `doc.info` reports the
  document `dirty`.
- A node of role `multiline_text_input`, labelled `Editor`, inside a node of role `scroll_view`
  that contains no shell node (`Path`, `Open`, `Message`, `Documents`):
  - `value` is the current document's text exactly (`doc.text`, every char, `\r` included);
  - `selection` is its selection: anchor and head in chars, as `cursor.get` reports them;
  - `rows` are the painted rows by the rules of `docs/protocol/ui.md`: top to bottom, one row
    for each document line in view that has at least one char in the visible part, the lines
    consecutive; the runs' texts of a row together equal the line's chars from its first char
    up to the view's right edge, without its line break.
  - The editor is not focused by itself: a fresh session and an opened document leave every
    node unfocused; a `click` on `Editor` or a `focus` on it focuses it.

Offsets are in chars, as in the commands; rows are document lines. The editor is drawn in the
built-in monospace font.

## Colours

Every char of the painted rows that paints pixels has the colour of its `syntax.highlight` kind
in this table; a char that paints no pixels (a space or a tab) is reported with its neighbour's
colour (`docs/protocol/ui.md`: the char before it in the row, at the start of a row the first
one after it that paints pixels). This holds for a document whose path ends in `.rs`; every
other document, and a `.rs` document that `syntax.highlight` refuses as `too_large` (longer than
1,048,576 chars), is painted in the `plain` colour.

| kind | colour |
|---|---|
| `plain` | `#dcdcdc` |
| `keyword` | `#569cd6` |
| `type` | `#4ec9b0` |
| `function` | `#dcdcaa` |
| `string` | `#ce9178` |
| `number` | `#b5cea8` |
| `comment` | `#6a9955` |

In a row that holds only blanks (spaces, tabs), the blanks are reported in the colour of
`plain`. A selection does not change text colours. A later task that changes a colour changes this
table, the spec of T-056 and its tests together.

## Input

With `Editor` focused; other nodes keep the toolkit's handling. "Dispatches" names the mutating
commands an input is recorded as in the log; the view's reads are not listed (below).

| Input | Dispatches | Effect |
|---|---|---|
| typed text, empty selection | `doc.insert` at the head | the text is inserted at the caret; the caret is after it |
| typed text over a selection | `doc.replace` of `start..end`, then `cursor.set {anchor: start + n}` | the text replaces the selection; the caret is after it, collapsed |
| `Enter` | as typed text, the text being the line ending of `doc.info` `line_ending` read when the key is handled: `lf` → `\n`, `crlf` → `\r\n`, `cr` → `\r`, `none` → `\n` | a CRLF document stays CRLF |
| `Backspace` | `doc.delete` | the selection; with an empty selection the range from where `cursor.move {by:"char", forward:false}` would put the head to the head (a CRLF goes whole); at offset 0 nothing is dispatched |
| `Delete` | `doc.delete` | the selection; with an empty selection the range from the head to where `cursor.move {by:"char", forward:true}` would put it; at the end nothing is dispatched |
| `ArrowLeft` / `ArrowRight` | `cursor.move {by:"char"}` | with `ctrl`: `by:"word"`; with `shift`: `extend:true`; both combine |
| `ArrowUp` / `ArrowDown` | `cursor.move {by:"line"}` | with `shift`: `extend:true` |
| `Home` / `End` | `cursor.set` | head to `start` / `end` of its line as `doc.line` reports it; without `shift` `anchor` = head; with `shift` the anchor stays |
| pointer click in a row | `cursor.set` | the head goes to the char boundary nearest the press point on that row (a press at the left edge of a char puts it before that char; just right of the row's last char, after it); without `shift` collapsed, with `shift` the anchor stays. A click is handled the same whatever the toolkit's click count |
| `ctrl`+`Z` | `doc.undo` | |
| `ctrl`+`shift`+`Z`, `ctrl`+`Y` | `doc.redo` | |
| `ctrl`+`S` | `doc.save` | |
| `scroll` on `Editor` | — (reads only) | the view scrolls; text and selection unchanged |

Not specified: `Tab`, `Escape`, `PageUp`, `PageDown`, `alt` with any key, `ctrl` with
`ArrowUp`, `ArrowDown`, `Home`, `End`, `Backspace` or `Delete`, typed text that contains a line
break or another control char, drag, double click as a separate gesture.

The editor changes a document only by commands: every keystroke and click that edits, moves the
caret, undoes, redoes or saves is a request in the session log.

Enter inserts the document's line ending as `doc.info` reports it when the key is pressed (`\n`
for a document without a line break). After Enter the caret is at the start of the line that follows the break
Enter made. Where the inserted line ending and the char after it form one line break together
(a `cr` document, Enter typed with a `\n` right after the selection's end), the caret is after
that whole break (the UI then follows the edit with a `cursor.set`; it sends none that the
session would refuse, and `Message` shows no refusal).

After a request that moves the caret, from the UI or from a script, the caret's line is in view
from the next frame on.

A command the UI dispatches that the session refuses changes nothing and sets `Message` to begin
with its kind (for example `doc.save` → `io_error` when all 100 temporary names `.kiln-tmp-0` …
`.kiln-tmp-99` in the file's directory exist); a successful UI open or save clears `Message`.

## Frames, reads and the log

The editor reads what it shows with read-only control requests, which are in the session log;
it sends them only in the first frame after a request or an input, never on a frame that
follows neither.

- Like every successful request, the reads are in the session log
  (`docs/control-protocol.md`), and replaying them changes nothing. Which reads it sends is not
  part of this interface.
- **Idle** means no request and no input since the view's last read; on an idle frame it sends
  nothing, so frames on an idle UI add nothing to the log. A script of control requests only,
  without start paths, runs no read (it runs no frame after the first request) and its `--log`
  stays `Host::log_script()`.
- In the frame in which it re-read after a script's control request, the view asks for one
  further frame: `{"ui":"frames","count":1}` right after a successful control request answers
  `settled: false`; the following `{"ui":"frames","count":1}` answers `settled: true` unless
  something else asks for frames.
- A read the session refuses (for example `syntax.highlight` → `too_large`) is not shown in
  `Message`. No UI request of its own gets a response line or changes the exit code.

The editor adds no file access: the files it reads and writes are those of commands, confined to
the root (`docs/protocol/paths.md`). No process, no network. A document's text is drawn, never
interpreted.

## Limits

- Lines are not wrapped and the view does not scroll horizontally: a long line is cut at the
  right edge.
- An empty line paints nothing, so it has no row.
- The view re-reads the document after every request, so the cost of a keystroke grows with the
  document's length; no time bound is stated.
- The largest settle count of an editor action measured in the implementer's test scripts of
  T-056 (`apps/kiln/tests/editor_t056.rs`) was 20 frames, for a `scroll` of 3 or more steps on
  `Editor`; there a `click` on `Editor` settled within 14 frames, `key` within 4, `type` and
  `focus` within 3. Every action's
  settling stays capped at 60 frames (`docs/protocol/ui.md`).

## Example

A root `proj/` with `src/a.rs` = `fn main() {\n    let x = 1;\n}\n` and
`kiln script s.jsonl --root proj src/a.rs`:

```text
{"id":"s","ui":"snapshot"}
{"id":"c","ui":"click","target":{"label":"Editor"},"at":{"row":1,"col":4}}
{"id":"t","ui":"type","text":"y"}
{"id":1,"command":"doc.text","params":{"doc":1}}
{"id":2,"command":"cursor.get","params":{"doc":1}}
```

The snapshot `s` has `Title` beginning with `src/a.rs` and `Editor` with that text as its
`value`; its first row is
`[{"text":"fn ","color":"#569cd6"},{"text":"main","color":"#dcdcaa"},{"text":"() {","color":"#dcdcdc"}]`
(the space after `fn` takes the colour of `n`). The click puts the caret at char 16 (line 1
starts at 12); typing `y` gives `doc.text` `fn main() {\n    ylet x = 1;\n}\n` and
`cursor.get` anchor 17, head 17, line 1, col 5. The log holds `doc.open`, the view's reads,
`cursor.set {anchor:16,head:16}` and `doc.insert {offset:16,text:"y"}`.
