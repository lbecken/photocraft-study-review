# `kiln script`: the Kiln UI without a window, driven by a script

T-055, ADR-0008. Runs the Kiln UI with no window from a script of control requests and UI
requests, and reports what each frame shows. Tests and the evaluator observe UI behaviour
through it. The control protocol itself is described in `docs/control-protocol.md`; this file
covers the UI requests and the mode.

`kiln script` needs no display, window system or GPU.

## Usage

```text
kiln script <script> [--root <dir> | --memory] [--log <file>] [--size <w>x<h>] [<path>…]
kiln --help | -h | script --help
```

- `<script>` and `--log <file>` are paths on the host, relative to the current directory.
  Request paths are relative to the root, as in the control protocol.
- `--root <dir>` (default: the current directory) and `--memory` (an empty in-memory file
  system) as for `kiln-cli run`; they exclude each other.
- `--size <w>x<h>`: the UI's size in points, integers 200 to 8192 each; default `1280x800`.
- `<path>…` (T-056): `kiln script` opens the paths given after the script at start, in order,
  as if each were opened through `Path`. A refused one sets `Message` to begin with its kind, a
  successful one clears `Message`; start paths get no response line and do not change the exit
  code; a successful one is logged as `doc.open` with that path, in the order of the command
  line and before the script's first request, a refused one is not logged. These `doc.open`s
  are input of the session: the log replayed with `kiln-cli run` reproduces the files with them
  included.
  With `--memory` every start path is refused (`not_found`). Start paths are read only through
  `doc.open`.
- Options may come before or after the positional arguments, each at most once.
- `--help`, `-h` (alone or after `script`): the usage on stdout, exit 0.
- `kiln` with no subcommand still only says that the desktop window is not built and exits 1
  (the window is T-065).

Order of checks at start: usage (options, `--size` range) → script readable → root (or
`--memory`) → log file writable → start paths in order → frames until settled → requests. The conventions are those of `kiln-cli run`: a usage
error prints what was wrong and the usage on stderr; a bad root is refused before any request
runs with a `kiln: bad --root:` message naming the root as given; in every such case nothing is
written on stdout.

| Exit code | When |
|---|---|
| 0 | every response `ok:true` |
| 1 | writing stdout failed |
| 2 | usage error, unreadable script, bad root, log file not writable (message on stderr, nothing on stdout) |
| 3 | at least one response `ok:false` |

Files: the script is read once, whole, at start. The `--log` file is created (or truncated)
before the first request and written after the last, in the format of "The session log as a
script" (`docs/control-protocol.md`). The root is touched only through control requests (the
UI's included). No other file is written; nothing is read from or written to the user's
configuration or home directory. No display variable is read in script mode. Linux is what the
gate and the evidence of T-055 cover.

## Script lines

Framing as in `docs/control-protocol.md` "Framing": one request per line, blank lines get no
response, one compact response line with sorted keys per other line, in order; the session
continues after any failure. A line that does not parse, is not an object, or has an invalid
`id` gets the control protocol's error for it.

- An object **with** a key `ui` is a **UI request**. Allowed keys: `id`, `ui` and those of its
  action below; any other key is refused (`bad_request`).
- Every other object is a **control request**.

A control request in a script gets the response line `kiln-cli control` would give it at the
same point of the same session. The UI and the script's control requests act on one session.

Every change the UI makes to documents or files is a control request: it is in the session log,
and running the log with `kiln-cli run` on a copy of the initial files gives the same files.
The editor reads what it shows with read-only control requests, which are in the session log;
it sends them only in the first frame after a request or an input, never on a frame that
follows neither. So `frames` on an idle UI adds nothing to the session log
(`docs/protocol/ui-editor.md`, "Frames, reads and the log").

## UI requests

**Targets.** `target` is an object with one or more of `role`, `label`, `label_contains`,
`value`, `value_contains` (strings); a node matches when every given field matches (`role`,
`label`, `value` exactly; `*_contains` as a substring). Offscreen nodes match too. The nodes are
those of the last frame's snapshot.

| `ui` | Other keys | Does |
|---|---|---|
| `snapshot` | — | reports the last frame (below); runs no frame |
| `frames` | `count` (integer 1–600) | runs exactly `count` frames with no input |
| `focus` | `target` | gives the target keyboard focus |
| `click` | `target`; `button` (`"primary"` default, `"secondary"`); `modifiers` (array of `"ctrl"`, `"shift"`, `"alt"`, `"command"`; default none); `at` (optional `{"row":r,"col":c}`, integers ≥ 0) | a pointer press and release at the target, as a user's click: at the centre of its visible part, or with `at`, at the left edge of char `c` of the target's painted row `r` (its `rows`), or just right of the row's last char when `c` equals the row's length |
| `type` | `text` (string) | the text as typed input to the focused node |
| `key` | `key` (a name from the table below); `modifiers` as for `click` | one key press and release |
| `scroll` | `target`; `direction` (`"up"`, `"down"`, `"left"`, `"right"`); `steps` (integer 1–100, default 1) | scrolls the scrollable area that contains the target by `steps` steps |

**Key names:** `A`–`Z`, `0`–`9`, `Enter`, `Tab`, `Backspace`, `Delete`, `Escape`, `Space`,
`ArrowLeft`, `ArrowRight`, `ArrowUp`, `ArrowDown`, `Home`, `End`, `PageUp`, `PageDown`,
`F1`–`F12`. `"ctrl"` and `"command"` both press the Ctrl key (on Linux the toolkit's `command`
modifier is Ctrl). A key request delivers the key only, no typed text: use `type` for text.

How input is delivered: `click` presses at the point in one frame (after moving the pointer
there) and releases in the next; the pointer then stays where it was released. `key` presses in
one frame and releases in the next. `type` and `focus` are one frame of input. `scroll` asks the
toolkit's accessibility scroll action of the target `steps` times in one frame; a step is the
toolkit's (100 points).

**Responses.** An action (`frames`, `focus`, `click`, `type`, `key`, `scroll`) answers
`{"id":…,"ok":true,"result":{"frames":<n>,"settled":<bool>}}`. After an action the runner runs
frames until the UI asks for no further frame, at most 60, and reports how many it ran and
whether it settled (`frames` runs exactly `count`). `n` counts the frames that carried the input
too. Before the first request of a script (after the start paths) the runner runs frames the
same way.

A control request runs no frame and a `snapshot` runs none: what a control request changed is
shown from the next frame on (run `{"ui":"frames","count":1}` or any action before the snapshot),
and between two snapshots the `frames` total grows by exactly the frames the UI requests between
them reported.

Each frame advances the UI's time by exactly 1/60 s.

### Snapshot

`snapshot` answers
`{"id":…,"ok":true,"result":{"frames":<total frames run in the session>,"root":<node>}}`. A node
is an object with:

- `role`: the accessibility role the toolkit reports for it, in snake_case (`button`, `label`,
  `text_input`, `window`, …);
- `label`, `value`: strings, each present only when the node has one;
- `focused`, `disabled`, `offscreen`: `true`, present only when true (`offscreen`: no part of the
  node lies in the visible part of the window);
- `selected`, `toggled`: booleans, present only when the node has that state;
- `selection`: `{"anchor":a,"head":h}` in chars of `value`, present for a text input that has a
  caret or selection;
- `rows`: for a node that shows text: the text the frame painted inside the node and inside the
  visible part of the window, one array per painted row from top to bottom, each row a list of
  runs `{"text":…,"color":"#rrggbb"}` from left to right, adjacent runs of the same colour merged.
  A row's text is the chars of the text the node shows that the row paints, in reading order,
  taken from the text the toolkit laid out and not from the positions of its glyphs: a
  ligature, a combining mark or a joiner that the toolkit shapes into more or fewer glyphs than
  chars changes nothing in the reported text, and a `click`'s `at` column counts those chars;
- `children`: the child nodes in the toolkit's order (always present, possibly empty).

Nodes that the frame does not show at all (hidden) are left out. The snapshot is computed from
what the toolkit produced for the frame (its accessibility output and painted shapes), never from
a description the UI code writes about itself.

How the fields are read from the toolkit's output:

- `label` is the node's accessible name: its own label, or else the text of the nodes it is
  labelled by (a visible label next to an input).
- The toolkit's text-run nodes (role `text_run`, the per-row pieces of a text widget) are not
  reported as nodes: their text is the parent's `value` and `rows`, and the parent's `selection`
  is computed from them.
- A node's rectangle is its bounds, or, for a node without bounds, the union of its children's.
  Every scrollable area is reported as a node of role `scroll_view` whose bounds are the
  visible part of the area; the visible part of the window around a node is the window cut by every `scroll_view`
  above it.
- A painted character counts when its centre lies inside its clip rectangle and the window; it
  belongs to the deepest node whose visible part contains its centre (the root when none
  does), the visible part of a node being its rectangle cut by the window and by every
  `scroll_view` above it. So a node marked `offscreen` has no `rows`, and a scrolled list never
  takes the text of a node outside it.
  Its colour is the colour it was painted with; a character that paints no pixels (a space)
  takes the colour of the character before it in the painted row (at the start of a row, of the
  first one after it that paints pixels), so the colour reported for a space is its neighbour's,
  not necessarily the one its text was given.

### Refusals

`{"error":{"code":"ui","kind":<kind>,"message":…},"id":…,"ok":false}`, checked in this order:

| `kind` | When |
|---|---|
| `bad_request` | a missing, unknown or wrongly typed key, a number out of range, an unknown key or modifier name, an empty target |
| `unknown_action` | `ui` is not one of the table |
| `no_target` | no node matches |
| `ambiguous_target` | more than one node matches |
| `not_actionable` | the action cannot apply: `focus` or `type` with no node that takes text input, `click` on an `offscreen` target (scroll first), `scroll` with no scrollable area around the target, `at` on a target without `rows` or beyond its painted rows or a row's length |

Line-level errors come first, exactly as in the control protocol (`invalid_json` / `not_utf8`,
`syntax`; `invalid_request` / `not_an_object`, `bad_id`); then the order above. Control requests
keep their kinds (`error_kinds` in `docs/commands.json`). A refused UI request runs no frame and
changes nothing. A UI refusal and a refused control request leave the session and the UI
unchanged and are not logged. `message` is for humans (at most 200 characters, as in the control
protocol); match on `code` and `kind`. `focus` takes a node that takes text input (a text-input
role, neither read-only nor disabled); `type` needs the focused node to be one.

UI requests add no registry command.

## The shell

What the UI shows of T-055 (T-056 adds the editor view, `docs/protocol/ui-editor.md`;
T-065 the window; later tasks add panels):

- a text input labelled `Path` (role `text_input`);
- a button labelled `Open`;
- a node labelled `Documents` (role `list`, inside a `scroll_view`) whose children are one node
  per open document, in id order, each with the document's path as its `value`;
- a node labelled `Message` whose `value` is empty or absent, or, after a refused open by the
  UI, begins with the refusal's `kind` (e.g. `not_found`) followed by free text.

Clicking `Open` (or `Enter` in `Path`) asks the session to open the path in `Path` (a `doc.open`
control request); `Path` keeps its text. A successful open clears `Message`. Every node of role `button` or `text_input` has a non-empty `label`. The scroll bar the
toolkit adds to an overflowing scroll area is a node of role `scroll_bar` without a label;
scripts scroll with `scroll` on the `scroll_view` (or a node inside it), not through the bar.

## Determinism

The output and the log depend only on the script, the initial files and the build: the same
script on two copies of the same directory, at different locations, prints the same bytes and
writes the same log. No clock is read; each frame advances the UI's time by exactly 1/60 s.

## Limits

- A script line is read whole into memory (as the control protocol).
- A FIFO given as the script or as `--log` blocks until its other end is opened, as with
  `kiln-cli run` (the script is read whole at start; the log file is opened before the first
  request).
- Mouse-wheel input is not offered; `scroll` uses the accessibility scroll action.
- Text that the toolkit lays out right to left (a text that begins with an Arabic-Indic digit
  such as `٣`, or with an Arabic or Hebrew letter): its rows report its chars in reading order,
  but which of them count as painted inside the visible part is decided from glyph positions
  that do not match the chars, so a row can lack chars at its end or keep chars beyond a cut,
  and when the toolkit lays such text out into several rows (in a narrow node), the rows' text
  can be wrong. Measured: `Title` of a document named `٣٣ ab ٣٣ ab … fin fi.txt` (1280×800) reports
  its row without the last two chars; at `--size 200x400` the `Documents` entry and `Title`
  report a reversed and a duplicated row.
- `frames` and `scroll` are capped (600, 100) and each action's settling is capped at 60
  frames, so one request runs a bounded number of frames.
- The runner spawns no process, opens no network connection and reads no clock for behaviour.
  A script is trusted as much as a `kiln-cli run` script and no more: every file the UI reads
  or writes goes through a control request (confinement and refusals as
  `docs/protocol/paths.md`).

## Example

A root `proj/` with `src/a.rs`:

```text
{"id":"f","ui":"focus","target":{"label":"Path"}}
{"id":"t","ui":"type","text":"src/a.rs"}
{"id":"c","ui":"click","target":{"label":"Open"}}
{"id":"s","ui":"snapshot"}
{"id":1,"command":"doc.list"}
```

Each of `f`, `t`, `c` answers `ok:true`; the snapshot has, under the node labelled `Documents`,
one child with `value` `src/a.rs`; `doc.list` lists `src/a.rs` as doc 1, and the log holds a
`doc.open` with `{"path":"src/a.rs"}`. Typing `src/missing.rs` instead leaves `Documents` without
children and gives `Message` a value beginning `not_found`.
