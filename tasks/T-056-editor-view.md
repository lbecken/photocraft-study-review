---
id: T-056
title: "Show the current document in an editor view with syntax colours, a caret and scrolling, and edit, undo and save it by dispatching commands only, observed through kiln script"
tier: high              # re-tiered by D-158: it claims `ui.dispatch-only`, a high-tier scorecard item; the spec check (R-238) had set medium for the absence of dependency and gate-path changes
state: tests_joined
owner_role: planner
scorecard_items: [ui.editor-view, ui.dispatch-only]
depends_on: [T-055]     # merged at e34ba1f (D-155); spec checks R-237 (main 197617c) and R-238 (main e0926ae)
touches:                # D-153: every path belongs to one role; the role is named first
  - crates/kiln-ui/src/                 # implementer · editor view, the shell's layout, panels/ (Pinned (b)), start paths of the runner
  - crates/kiln-ui/tests/               # implementer · own integration tests (each links the toolkit: keep few)
  - apps/kiln/src/                      # implementer · start paths of `kiln script`, usage text
  - apps/kiln/tests/editor_t056.rs      # implementer · own tests through the binary
  - apps/kiln/tests/script_t055.rs      # implementer · only the one case `["script","s.jsonl","t.jsonl"]` (Q-6 A); nothing else
  - docs/protocol/ui.md                 # implementer · Usage (start paths), the shell, the view's reads (Pinned (i))
  - docs/protocol/ui-editor.md          # implementer · new
  - apps/kiln/tests/acceptance_t056.rs  # test author
  - apps/kiln/tests/guard_t056.rs       # test author · optional (no guard is required, Pinned (f))
budget_tokens: 1100000 # high (v1.3), D-158
iteration: 2
found_by: planner
origin_task: null
---

# T-056 An editor view that shows and edits the current document by dispatching commands only

## Goal
After this task the Kiln UI that `kiln script` runs (T-055) shows, beside the shell, an **editor
view** of the current document: its text with syntax colours, a caret and selection, and
scrolling. Typing, deleting, Enter, caret movement, undo, redo and save work from keys and
pointer clicks, and each is a registry command dispatched through the session (ADR-0008
decision 5). `kiln script` also takes paths to open at start. Tests and the evaluator observe all
of it through `kiln script`; the desktop window that shows the same UI is T-065.

Tier medium (D-156): no dependency, no `Cargo.toml` or `Cargo.lock` change, no gate path, no new
file write of Kiln's own (documents are saved by `doc.save`). A change that needs any of these is
a `QUESTION:` (it would make the task high). Experiment arm: even number, so an independent test
author; one full review round with a mutation spot-check; no approval row.

Reading list: `docs/adr/0008-ui-observation.md` (with its rulings); `tasks/T-055-*.md` (rulings
n1–n11, w1–w7) and `docs/protocol/ui.md` as merged, in particular how `rows`, colours of blank
chars, `scroll_view` and `offscreen` are defined; `docs/protocol/cursor.md` (selection, mapping
rule, movement rule), `docs/protocol/syntax.md` (`syntax.highlight` kinds, size limit),
`docs/protocol/eol.md`, `docs/protocol/doc-positions.md`; `docs/control-protocol.md` (`doc.*`,
"The session log as a script": read-only requests are logged too); `docs/commands.json` (params,
results and `mutating` of the commands named below).

## Interface

### Command line
- `kiln script <script> [--root <dir> | --memory] [--log <file>] [--size <w>x<h>] [<path>…]`
  (T-055, extended; Q-6 A): the paths after the script are positional, in order, before or
  after the options. Each is opened as if it were typed into `Path` and opened (T-055: a refused
  one sets `Message` to begin with its kind, a successful one clears `Message`), after the log
  check and before the frames that run before the first request (Pinned (c)). Start paths get no
  response line and do not change the exit code; a successful one is logged as `doc.open` with
  that path, a refused one is not logged. With `--memory` every start path is refused
  (`not_found`). A second positional argument is therefore no longer a usage error.
- `kiln --help`, `kiln -h`, `kiln script --help`: usage on stdout, now with `[<path>…]`, exit 0.
- `kiln` without a subcommand is unchanged by this task (T-065 replaces it with the window).

### The current document
The editor view shows one document, the **current document**: the one most recently opened by a
successful `doc.open` from any source (a start path, the UI, a script's control request), a
`doc.open` that answers `already_open: true` included; if it is closed, the open document with
the highest id; with none open, no `Title` and no `Editor` node is shown. (T-058 adds switching
by tab; it amends this rule.)

### What the UI shows (nodes, for `kiln script` snapshots)
- A node labelled `Title` (role not pinned) whose `value` begins with the current document's
  path; the part of the value after the path contains the word `modified` exactly when
  `doc.info` reports the document `dirty`.
- A node of role `multiline_text_input`, labelled `Editor`, inside a node of role `scroll_view`
  (the T-055 helper, ruling n6) that contains no shell node (`Path`, `Open`, `Message`,
  `Documents`):
  - `value` is the current document's text exactly (`doc.text`, every char, `\r` included);
  - `selection` is its selection: anchor and head in chars, as `cursor.get` reports them;
  - `rows` are the painted rows by the rules of `docs/protocol/ui.md`: top to bottom, one row
    for each document line in view that has at least one char in the visible part, the lines
    consecutive; the runs' texts of a row together equal the line's chars from its first char
    up to the view's right edge, without its line break. A line with no chars (an empty line)
    paints nothing and has **no** row. Lines are not wrapped and the view does not scroll
    horizontally: a long line is cut at the right edge.
  - The editor is not focused by itself: a fresh session and an opened document leave every
    node unfocused (T-055 n2); a `click` on `Editor` or a `focus` on it focuses it.
- Colours (Q-8 A: this table is the published colour of each kind; the tests take the values
  from it): every char of the painted rows that paints pixels has the colour of its
  `syntax.highlight` kind in this table; a char that paints no pixels (in tests: a space or a
  tab) is reported with its neighbour's colour (`docs/protocol/ui.md`: the char before it in
  the row, at the start of a row the first one after it that paints pixels). This holds for a
  document whose path ends in `.rs`; every other document, and a `.rs` document that
  `syntax.highlight` refuses as `too_large` (longer than 1,048,576 chars), is painted in the
  `plain` colour.

  | kind | colour |
  |---|---|
  | `plain` | `#dcdcdc` |
  | `keyword` | `#569cd6` |
  | `type` | `#4ec9b0` |
  | `function` | `#dcdcaa` |
  | `string` | `#ce9178` |
  | `number` | `#b5cea8` |
  | `comment` | `#6a9955` |

  A selection does not change text colours (the toolkit's own text input paints selected text
  `#c0deff`, `reviews/T-055-r1.md` M5: the editor must not use that). A later task that
  changes a colour changes this table, `docs/protocol/ui-editor.md` and the tests together.
- The shell of T-055 (`Path`, `Open`, `Documents`, `Message`) stays, with what T-055's tests
  rely on (Pinned (f)).

### Input
With `Editor` focused; other nodes keep the toolkit's handling. "Dispatches" names the mutating
commands an input is recorded as in the log; the view's reads are not listed (below). Tests
check effects and replay, not which commands were sent.

| Input | Dispatches | Effect |
|---|---|---|
| typed text, empty selection | `doc.insert` at the head | the text is inserted at the caret; the caret is after it |
| typed text over a selection | `doc.replace` of `start..end`, then `cursor.set {anchor: start + n}` | the text replaces the selection; the caret is after it, collapsed (`doc.replace` alone leaves the selection over the new text: Pinned (g)) |
| `Enter` | as typed text, the text being the line ending of `doc.info` `line_ending` read when the key is handled: `lf` → `\n`, `crlf` → `\r\n`, `cr` → `\r`, `none` → `\n` | a CRLF document stays CRLF (Q-4 A) |
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

Not specified (tests do not use them): `Tab`, `Escape`, `PageUp`, `PageDown`, `alt` with any
key, `ctrl` with `ArrowUp`, `ArrowDown`, `Home`, `End`, `Backspace` or `Delete`, typed text that
contains a line break or another control char, drag, double click as a separate gesture.

After any successful request, from the UI or from the script, that changes the head of the
current document, the next frame scrolls the view vertically so that the head's line is in view.
A command the UI dispatches that the session refuses changes nothing and sets `Message` to begin
with its kind; a successful UI open or save clears `Message`.

### Frames, reads and the log (Q-9 A, Q-10 A)
- The view reads what it shows with **read-only** control requests (`mutating: false` in
  `docs/commands.json`). Like every successful request they are in the session log
  (`docs/control-protocol.md`), and replaying them changes nothing.
- It sends them only in the first frame after a successful request of any source (the script's
  control requests, the UI's own commands, start paths) other than its own reads, or after an
  input that changes what is in view (a `scroll`, a click, a key). **Idle** means no request and
  no input since the view's last read; on an idle frame it sends nothing, so frames on an idle
  UI add nothing to the log. A script of control requests only runs no read (it runs no frame
  after the first request, T-055 n4) and its `--log` stays `Host::log_script()` (T-055 AC-1).
- In the frame in which it re-read after a script's control request, the view asks for one
  further frame: `{"ui":"frames","count":1}` right after a successful control request answers
  `settled: false`; the following `{"ui":"frames","count":1}` answers `settled: true` unless
  something else asks for frames.
- A read the session refuses (for example `syntax.highlight` → `too_large`) is not shown in
  `Message`. No UI request of its own gets a response line or changes the exit code.

### Rust API used by tests
None. All criteria are tested through the built `kiln` binary; `kiln_control::Host` is the only
Rust API a test may use, as an oracle, with the signatures pinned in T-055 (unchanged on main
`197617c`, `crates/kiln-control/src/lib.rs` lines 85, 92, 104, 150):
```rust
impl kiln_control::Host {
    pub fn on_disk(root: &std::path::Path) -> Result<kiln_control::Host, kiln_control::HostError>;
    pub fn in_memory(files: &[(&str, &[u8])]) -> Result<kiln_control::Host, kiln_control::HostError>;
    pub fn handle_line(&mut self, line: &[u8]) -> Option<String>;
    pub fn log_script(&self) -> String;
}
```
`apps/kiln/Cargo.toml` already has `kiln-control` and `serde_json` as dev-dependencies (T-055);
neither role edits it.

### Shared with T-065 (Rust items inside `kiln-ui`, not used by tests)
`kiln script` draws every frame with one function of `kiln-ui` that takes the frame's
`&mut egui::Ui` and the UI's state (shell, editor view, session); T-065's window calls the same
function, so that the window and `kiln script` show the same UI. Its name and signature are the
implementer's and are documented in `crates/kiln-ui/src/lib.rs`; T-065's spec check pins them.
The script mode keeps its theme and style (dark theme, no caret blink, T-055).

## Pinned
- (a) Errors: no new refusal kind, no new registry command, no new UI request kind. A refused
  command the UI dispatches leaves the session as it was and sets `Message` to begin with the
  kind (`doc.save` → `io_error` when all 100 temporary names `.kiln-tmp-0` … `.kiln-tmp-99` in
  the file's directory exist: executed, (g)). Precedence is the commands' own. Exit codes of
  `kiln script` unchanged (T-055); start paths do not change them.
- (b) Files: as T-055 (`--log`); start paths are read only through `doc.open`; documents are
  saved only by `doc.save` (ADR-0008 decision 5). Source layout, so that the module tasks T-057
  to T-061 can run in parallel: the shell places each later panel by calling one function in its
  own file, and this task creates those files drawing nothing:
  `crates/kiln-ui/src/panels/project.rs`, `tabs.rs`, `status.rs`, `outline.rs`, `find.rs`, each
  with `pub(crate) fn show(ui: &mut egui::Ui, view: &mut PanelView<'_>)`, and `panels/mod.rs`
  with the type `PanelView`, through which a panel sends control requests through the one
  session and reads and sets the current document (its fields are the implementer's, documented
  in `panels/mod.rs`; T-057 to T-061 re-check them). Where the shell puts them is for the
  source, not for tests: project and outline beside the editor, tabs and find above it, status
  below it. A module task then touches its own file, its tests and its document only.
- (c) Order of `kiln script`: usage → script readable → root (or `--memory`) → log writable →
  start paths in order → frames until settled (T-055) → requests. On an input the command is
  dispatched in the frame that handles it; the view shows the session's state after it from the
  same or the next frame, which the action's settling includes.
- (d) Offsets in chars, as the commands; rows are document lines (no wrap, no horizontal
  scroll). Linux, as T-055. Fonts built in (the editor in the built-in monospace font).
- (e) Threat model: the editor adds no file access: the files it reads and writes are those of
  commands, confined to the root (`docs/protocol/paths.md`). No process, no network. A
  document's text is drawn, never interpreted.
- (f) Tests that pin what this changes. The T-055 tests on main stay in the gate and must keep
  passing. With the editor added they rely on these facts, which the shell keeps (none of them
  is a layout T-056's own tests may assume):
  - L1 at `--size 200x200` with 100 open documents, `Path` is visible and shows what was typed
    into it (`w3d_scrolled_list_reports_only_visible_text`);
  - L2 at the default size, `Open` has a painted row (`ac7_ui_refusals`, preconditions);
  - L3 `Path` and `Open` are inside no `scroll_view` (`script_t055.rs` expects `scroll` on
    `Open` to be `not_actionable`);
  - L4 nothing is focused until a script focuses or clicks (`ac7_ui_refusals`: `type` first is
    `not_actionable`);
  - L5 no new node is labelled `Path`, `Open`, `Documents` or `Message`, and no new label
    contains `Pat` or `Ope` (`w3b_target_by_label_contains`);
  - L6 at the default size, 100 open documents overflow `Documents` and some stay visible after
    1 and after 4 scroll steps (`w3e_more_steps_scroll_further`);
  - L7 a script of control requests only writes `Host::log_script()` as its log
    (`ac1_control_pass_through`; "Frames, reads and the log");
  - L8 every action settles within 60 frames (`action_ok` in both files).
  `script_t055.rs:599` asserts exit 2 for `kiln script s.jsonl t.jsonl`, which start paths make a
  valid command line: the implementer changes that one case (Q-6 A). `guard_t055.rs` (bare
  `kiln`) is untouched: this task does not change bare `kiln`. Not to be pinned by this task's
  tests (golden rule 9): a whole snapshot, the usage text, message texts, the layout of panels,
  the key table beyond the rows above (T-025, T-031 change bindings), which read requests the
  view sends, the Dispatches column, the command list, `error_kinds`.
- (g) Worked examples, executed with the `kiln-cli` binary built from main `197617c` (R-237;
  commands and outputs in "Spec check R-237", E-e):
  - `src/a.rs` = `fn main() {\n    let x = 1;\n}\n` (29 chars). `syntax.highlight
    {doc:1,start:0,end:11}` → `keyword 0..2, plain 2..3, function 3..7, plain 7..11`. The space
    at 2 paints no pixels and takes the colour of `n`, so the first painted row is
    `[{"text":"fn ","color":"#569cd6"},{"text":"main","color":"#dcdcaa"},{"text":"() {","color":"#dcdcdc"}]`
    (chars 0..11; the space at 9 takes the colour of `)`).
  - A click `at {row:1,col:4}` puts the caret at char 16 (`doc.line {line:1}` → `start 12`,
    text `    let x = 1;`); `type` `"y"` → `doc.text` `fn main() {\n    ylet x = 1;\n}\n`,
    `cursor.get` anchor 17, head 17, line 1, col 5 (executed as `cursor.set {anchor:16}`,
    `doc.insert {offset:16,text:"y"}`). `Backspace` → the original text, head 16.
  - Typing over a selection: with anchor 13, head 18 in that text, `doc.replace
    {start:13,end:18,text:"Z"}` leaves `cursor.get` anchor 13, head 14 (and with anchor 18,
    head 13: anchor 14, head 13): the selection covers the new text, so the UI follows with
    `cursor.set {anchor:14}`.
  - `a\r\nb` (`doc.open` → `line_ending` `crlf`), caret 3: `cursor.move {by:"char",
    forward:false}` → head 1, so `Backspace` deletes `1..3` (`removed` `"\r\n"`, text `ab`).
  - `x\r\ny` (`crlf`), caret 1, `Enter` → `doc.insert {offset:1,text:"\r\n"}` → `x\r\n\r\ny`.
  - `abc` (no break): `doc.info` `line_ending` is `none`, so `Enter` inserts `\n`. After the
    `Backspace` above, `c1.txt` holds `ab`, and a second `doc.open` of it answers
    `already_open: true`, doc 2, `line_ending` `none`; `doc.undo` then gives `dirty false`,
    `can_redo true`, `line_ending` `crlf`.
  - A refused save: `src/a.rs` with the 100 files `src/.kiln-tmp-0` … `src/.kiln-tmp-99`
    present: `doc.open`, `doc.insert {offset:0,text:"x"}`, `doc.save` → `rejected` /
    `io_error`; `doc.info` → `dirty true`; the file is unchanged.
  - The UI parts follow from the Input table and `docs/protocol/cursor.md`; the implementer
    re-executes them through the binary and logs the output.
- (h) Evaluator calls (ADR-0008 (O1)–(O5)):
  - `ui.editor-view`: `kiln script s.jsonl --root d src/a.rs`; snapshots before and after
    typing, arrows, a click `at`, a click without `at` after a scroll, scroll steps; colours
    against `syntax.highlight` and the table; caret by typing a char and reading `doc.text` and
    `cursor.get`.
  - `ui.dispatch-only`: the `--log` of an editing session replayed with `kiln-cli run` on a copy
    of the root gives the same files, texts and selections; every logged request the script did
    not send is a mutating command of the Input table or read-only; frames on an idle editor add
    nothing to the log.
- (i) Sentences for `docs/protocol/ui-editor.md` (E5 also in `ui.md`), worded for copying, with
  evidence:
  - E1 "The editor changes a document only by commands: every keystroke and click that edits,
    moves the caret, undoes, redoes or saves is a request in the session log." Evidence: AC-7.
  - E2 "Enter inserts the document's line ending as `doc.info` reports it when the key is
    pressed (`\n` for a document without a line break)." Evidence: AC-4; (g).
  - E3 the colour table and its rule above. Evidence: AC-2.
  - E4 "After a request that moves the caret, from the UI or from a script, the caret's line is
    in view from the next frame on." Evidence: AC-5.
  - E5 "The editor reads what it shows with read-only control requests, which are in the
    session log; it sends them only in the first frame after a request or an input, never on a
    frame that follows neither." Evidence: AC-7, AC-8.
  - `ui.md` Usage: "`kiln script` opens the paths given after the script at start, in order, as
    if each were opened through `Path`." Evidence: AC-1.
  - Limits, copied: "Lines are not wrapped and the view does not scroll horizontally: a long
    line is cut at the right edge." "An empty line paints nothing, so it has no row." "The view
    re-reads the document after every request, so the cost of a keystroke grows with the
    document's length; no time bound is stated." (No measurement exists; a bound would need
    one, METHODOLOGY §2.1.) The largest settle count of an editor action in the acceptance
    scripts is **to be measured by the implementer** (as T-055 S6).

## Acceptance criteria
Through the built `kiln` binary (`kiln script`), temporary roots, every process under a timeout;
nodes found by role and label (`Title`, `Editor`, `Message`), never a whole snapshot; no layout
is assumed (T-055 w7): rows are matched to lines by their text, so fixtures whose lines in view
are non-empty and pairwise distinct are used where rows are mapped to lines. Every criterion
fails on base: base `kiln script` shows no `Title` or `Editor` node and refuses start paths
(exit 2).
- AC-1 (`ui.editor-view`, the document shown): `kiln script s.jsonl --root d src/a.rs` (start
  path), and separately a script that opens `src/a.rs` through `Path` and `Open`, and one that
  opens it by a control `doc.open` and runs one frame: a snapshot shows `Title` beginning with
  `src/a.rs` whose remainder lacks `modified`, and `Editor` whose `value` equals `doc.text` and
  whose rows' texts equal the document's lines from line 0. Start paths `a.rs b.rs missing.rs`:
  current is `b.rs`, `Message` begins with `not_found`, the log holds the two `doc.open`s, the
  exit code is that of the script's own responses. A second open of `a.rs` makes it current; a
  control `doc.close` of the current one makes the highest open id current; with none open there
  is no `Editor`. Oracle: `doc.text`, `doc.line`, `doc.list` in the same session.
- AC-2 (`ui.editor-view`, colours; D-152 follow-up): for at least 20 Rust files (keywords,
  types, functions, strings, numbers, comments, blanks inside and at the start of lines, CRLF
  and non-ASCII text such as `é`, `λ`, `ж`), every char of every painted row of `Editor` that is
  not a space or a tab has the table's colour of the `syntax.highlight` kind of its offset, and
  every space or tab the colour its neighbour gets by the rule above; at least one row per file
  is checked after a `scroll` too. A `.txt` document and a `.rs` document of 1,048,577 chars are
  painted all `plain`. Oracle: `syntax.highlight` over the rows' char ranges in the same session
  and the table (Q-8 A). Kills a swap of colour channels (T-055 M5).
- AC-3 (`ui.editor-view`, caret and pointer; D-152 follow-up): a `click` `at {row, col}` on
  `Editor` for random rows and columns of the painted rows gives `cursor.get` head = that line's
  `start` + `col`; typing a char there inserts it at that offset (`doc.text`); `Editor`
  `selection` equals `cursor.get` after each step; `shift`-click keeps the old anchor. **Centre
  of the visible part:** on a document of 300 distinct non-empty lines, `scroll` `Editor` down
  until line 0 is out of view, snapshot S, then `click` `Editor` without `at`: the head is on a
  line that has a row in S (a click at the centre of the whole rectangle lands outside the
  window or far below the rows). Oracle: `cursor.get`, `doc.text`, `doc.line`.
- AC-4 (`ui.editor-view`, keys): seeded random sessions of typed text (no control chars),
  `Enter`, `Backspace`, `Delete`, `ArrowLeft`/`ArrowRight` with and without `ctrl` and `shift`,
  `ArrowUp`/`ArrowDown` with and without `shift`, `Home`, `End` (with and without `shift`), on
  LF, CRLF, CR-only, no-break and mixed documents: after every step `Editor` value equals
  `doc.text` and `selection` equals `cursor.get`; the final text and selection equal a naive
  model of the Input table using the rules of `docs/protocol/cursor.md` (mapping rule with the
  CRLF snap, movement rule); in an LF-only and a CRLF-only document `eol.counts` still reports
  one kind only after any session. `Enter` is pressed only where the caret's line has no space
  or tab before the caret, so that the later `edit.newline` (T-025: line ending plus the
  indentation before the caret) gives the same result. Oracle: naive model; `eol.counts`.
- AC-5 (`ui.editor-view`, scrolling and caret in view): a 300-line document of distinct lines:
  the first snapshot's rows are lines `0..k` (k ≥ 1); `scroll` `down` n steps on `Editor` makes
  the first row a later line, keeps the rows consecutive lines, and leaves `doc.text` and
  `cursor.get` unchanged; more steps scroll at least as far as fewer; `ArrowDown` from line 0 to
  line 200 leaves line 200 among the rows; a control `cursor.set` to line 250 and one frame
  leave line 250 among the rows. Oracle: `doc.line`, `cursor.get`.
- AC-6 (undo, redo, save): after edits, `ctrl+Z`, `ctrl+shift+Z` and `ctrl+Y` leave `Editor`
  value and `Title` equal to what `doc.text` and `doc.info` (`dirty`) report, and equal to what a
  second session reports after the same logged requests; `ctrl+S` writes the file, whose bytes
  equal those a second session writes from the same log, and `Title` loses `modified`. A save
  refused because the 100 temporary names exist (Pinned (g)) leaves the file unchanged, `Title`
  with `modified` and `Message` beginning with the kind a control `doc.save` reports in a second
  session (`io_error`). Oracle: a second session (differential); file bytes.
- AC-7 (`ui.dispatch-only`): for every session of AC-3, AC-4 and AC-6, running its `--log` with
  `kiln_control::Host` on a copy of the initial root gives the same `doc.text`, `cursor.get` and
  `doc.info` (`dirty`, `can_undo`, `can_redo`) for every document and the same files; every
  logged request that the script did not send is either `mutating: false` in
  `docs/commands.json` or one of the mutating commands of the Input table; on an idle editor
  (after an action, or one frame after the last control request), 600 frames add nothing to the
  log (compared with the same script without them). The `ui-commands` gate step passes on the
  branch. Oracle: replay (differential); `mutating` flags; log comparison; gate step.
- AC-8 (frames report whether the UI still asks for a frame; Q-10 A, D-152 follow-up, T-055
  M2): with a document open and the editor idle, a control `doc.insert` followed by
  `{"ui":"frames","count":1}` answers `settled: false`; repeating `{"ui":"frames","count":1}`
  answers `settled: true` within 60 repetitions, and once it has, one more such request answers
  `settled: true` and leaves the snapshot's `root` unchanged. On base the first answer is
  `settled: true` (R-237 E-f). Oracle: the `settled` flags against snapshot changes.

## Out of scope and implementer notes
- Out: the desktop window, starting bare `kiln`, the windowing dependency (T-065); tabs and
  switching documents (T-058), project tree (T-057), status bar (T-059), outline and navigation
  (T-060), find bar (T-061), smart indent and comment toggling (T-025), the palette and keymap
  (T-031), formatting (T-026), diagnostics (T-024), the external-change prompt (T-027), mouse
  wheel, drag-select, clipboard, IME, line numbers, bracket-match display, a screen-reader
  adapter, themes and font settings, performance budgets.
- Dependencies: none new (`kiln-ui` already has `egui`, `kiln-control`, `serde_json`). The
  queue refuses a medium task that changes `Cargo.lock` or a dependency table.
- Disk: test through the one `kiln` binary; each integration-test binary in
  `crates/kiln-ui/tests/` links the toolkit again.
- The view may ask for only the visible part of the highlight (`syntax.highlight` takes
  `start`, `end`); what it asks is the implementer's choice, the painted colours are not.

## Questions and rulings
Questions for the director, before dispatch:
- **Q-2 (ADR-0008). What counts for `ui.window`.** A: AC-1, AC-2 in the gate plus AC-3 by the
  reviewer and the evaluator under Xvfb; the item is confirmed only if AC-3 held for the
  evaluator. B: headless only (AC-1, AC-2; "opens a window" not observed). C: AC-3 as a gate test
  (fails on this machine unless the owner adds `libxkbcommon-x11-0` and `libxcb-xkb1` to the
  environment's setup, and its state on the CI runner is unknown). **Recommendation: A**, and
  put the two packages to the owner for the evaluator's machine.
- **Q-4. Enter before T-025.** A: Enter inserts the document's line ending (as specified),
  replaced by `edit.newline` (line ending plus indentation) in the UI module that follows T-025.
  B: Enter inserts `\n` (as `doc.insert` would; a CRLF file becomes mixed, the iteration-0
  finding). **Recommendation: A.**

### Rulings (director, D-147)
- **Q-2: A.** `ui.window` counts on the headless gate tests plus a real window under Xvfb, started by the reviewer in
  round 1 and by the evaluator; the two libraries are unpacked into scratch as in the probe (ADR-0008, rulings).
- **Q-4: A.** Enter inserts the document's line ending.
- This spec gets its spec check when T-055 has merged (it depends on T-055's interface).

### Follow-ups from T-055's review (director, D-152) — for this task's spec check
- A test that a `frames` request reports `settled: false` when the UI still asks for a frame (T-055 mutant M2
  survived: `settled` always true).
- Colours compared with an independent source: the painted colour of each run against `syntax.highlight` and the
  published colour of each highlight kind (T-055 mutant M5 survived: two colour channels swapped; T-055 could not pin
  a colour value).
- A click landing at the centre of a target's visible part, observed through something this task's UI shows (in
  T-055 it is covered only by a unit test).
- `apps/kiln/tests/guard_t055.rs` asserts a non-zero exit status for bare `kiln` without a display (T-055 w4); if
  this task changes what bare `kiln` does without a display beyond the exit status, the guard goes into `touches`.
- Scroll areas are reported with the helper of T-055 (role `scroll_view`, ruling n6); painted text belongs to the
  deepest node whose visible part contains it (w1).

### Spec check R-237
Planner run R-237 (`iter-2`), against main `197617ced8ce8beca1e008d0838d01096ebca2d5` (main moved
to `ca2a27c` during the run: `STATE.md` and `handoffs/` only, no effect). No task is in flight
(no `task/T-05x`/`T-06x` branch except `task/T-055-tests`, merged). Scratch work in a scratch
directory outside the repository with its own cargo home and a copy of the workspace; peak
3.3 GB; builds deleted afterwards. Nothing was built inside the repository.

**What changed in this file, and why**
1. Front matter: `touches` given one role per path (D-153); added `apps/kiln/tests/script_t055.rs`
   (implementer, one case: blocker B4); `docs/protocol/ui.md` scope widened to Usage and start
   (its lines 24–25 become false); `apps/kiln/Cargo.toml` stays out (no role needs it);
   `guard_t056.rs` made optional; `depends_on` note: T-055 merged.
2. Goal: approval row needs neither `gate paths` nor `dependency versions` (measured, E-a).
3. Command line: synopsis `kiln [--root] [--script] [<path>…]`; usage errors, the order of
   checks and exit 2 for every start failure; `script` as first argument; closing saves nothing;
   `--script` responses equal except frame counts (real time); stdout after closing, stderr
   separate (B10); `kiln script` start paths: when they are opened, logged, and that they give no
   response line and do not change the exit code (B4).
4. Current document: `already_open: true` counts; T-058 amends the rule.
5. `Title`: `modified` is looked for after the path (B7).
6. `Editor`: role, label, inside a `scroll_view` without shell nodes; `value` with every `\r`;
   not focused by itself (B8, L4); `rows` per `ui.md`'s rules, an empty line has no row (B3);
   no horizontal scroll.
7. Colours: the blank-char rule of `ui.md` added to the table's rule (B1); selection colour note.
8. Input table: typing over a selection collapses the caret by a `cursor.set` (B9); Enter for
   `none` and `cr` (B2); no-op Backspace/Delete dispatch nothing; Home/End without `shift`
   collapse; click to the nearest boundary, click count ignored; a list of unspecified inputs;
   scroll-to-caret after any change of the head from any source (needed by T-060, T-061); which
   successes clear `Message`.
9. New subsection "Frames, reads and the log" (B6) and the `settled:false` rule (Q-10).
10. Rust API: `Host` signatures restated with their lines on main.
11. Pinned (a) refused-save example replaced (B5); (b) GL cache measured, `PanelView` and `show`
    named for T-057–T-061; (c) both orders of checks; (d) features and window size with
    evidence; (e) display connection and GL cache; (f) the eight facts L1–L8 the T-055 tests
    rely on, the guard and the implementer test that change; (g) re-executed on main with the
    two corrections (row with the blank rule, `none`), the selection mapping, the refused save;
    (h) updated; (i) E5, E6 and limits added, settle count to be measured.
12. Acceptance criteria rewritten: each names its observation and why it fails on base; the
    three D-152 follow-ups are AC-10 (settled), AC-4 (colours against the published table),
    AC-5 (click at the centre of the visible part); AC-2 adds the order root → window and
    `WAYLAND_SOCKET`; AC-3 compares without frame counts; AC-6 limits `Enter` to unindented
    lines (T-025 collision) and adds CR-only and no-break documents; AC-7 adds caret-in-view
    from a control request; AC-8 uses the refused save that a script can provoke; AC-9 compares
    selections and `doc.info` after replay and classifies the logged requests.
13. Out of scope: Wayland, the close prompt; dependency and disk notes from measurements.

**Blockers found (each fixed above unless a question is named)**
- B1 Pinned (g)'s painted row had a separate `" "` run in `plain`; by `docs/protocol/ui.md`
  ("a character that paints no pixels (a space) takes the colour of the character before it")
  the space after `fn` is reported `#569cd6`. A test author following the draft would have
  written a test the implementation must fail.
- B2 "on a text with no break `doc.info` reports `lf`": executed, it reports `none`; the Enter
  rule had no answer for `none` and `cr`.
- B3 "one row per document line in view": `observe.rs` pushes a row only for glyphs painted
  (`text_rows`, `if !glyphs.is_empty()`), so an empty line has no row; mapping rows to lines
  by index would fail on any fixture with an empty line.
- B4 `kiln script … [<path>…]` contradicts `apps/kiln/tests/script_t055.rs:599` (on main, in the
  gate), which expects exit 2 for `kiln script s.jsonl t.jsonl`; the file was not in `touches`
  (Q-6).
- B5 AC-8's "a save whose directory was removed": a `kiln script` cannot remove a directory
  between two requests (the script is read whole first, T-055 Pinned (b)), so the criterion
  could not be tested; replaced by the occupied temporary names (`io_error`, executed).
- B6 Read-only requests are in the session log (`docs/control-protocol.md`, "Every successful
  request (read-only ones included)"; `Host` has no unlogged path, `crates/kiln-control/src/lib.rs`).
  The draft's "re-reads after any request" plus "600 idle frames add nothing" was
  contradictory for a `frames` request right after a control request, and a view that read
  outside frames would break T-055 AC-1 (L7) (Q-9).
- B7 A path containing `modified` (e.g. `modified.rs`) made `Title`'s rule false.
- B8 The T-055 tests on main constrain the shell: L1–L8 in Pinned (f); in particular an editor
  that takes focus when a document opens fails `ac7_ui_refusals` (L4), and a top row moved into
  a scroll area fails `script_t055.rs` (L3).
- B9 Typing over a selection with `doc.replace` alone leaves the new text selected (executed),
  not the caret after it as the draft's Effect column said.
- B10 `xvfb-run` on this machine runs the command as `"$@" 2>&1` (`/usr/bin/xvfb-run` line 184;
  checked: a command's stderr arrived on stdout), so AC-3's stdout comparison needs stderr
  redirected inside the wrapped command; the recipe does that.
- B11 `kiln --bogus` exits 1 on main (bare path); the draft did not say what the window binary
  does with a usage error.

**Questions for the director** (Q-2 and Q-4 stand as ruled)
- **Q-5. Split.** The task bundles two parts that can be built and verified apart.
  A: split into (i) T-056 *editor view* — `Title`, `Editor`, colours, input, scrolling, undo and
  save, the view's reads, start paths of `kiln script`, the panel files — claiming
  `ui.editor-view` and `ui.dispatch-only`; no dependency, no gate path, no new write path, so
  **medium** by ADR-0002 (budget 800,000; even number: independent test author); ACs 1 (without
  the window), 4–10; and (ii) a new task (next free number, T-065) *window* — eframe, `kiln`'s
  start, the no-display exit, `--script` in the window, claiming `ui.window`; **high** (dependency
  change; budget 1,100,000); ACs 1 (start paths in the window), 2, 3; `depends_on` (i), since both
  edit `crates/kiln-ui/src/`, `apps/kiln/src/` and `ui.md` and the window shows the editor view.
  B: keep one high task. **Recommendation: A**: the bulk of the behaviour gets a medium review
  (C5 can be tested), the high-tier approval, the dependency and the reviewer's Xvfb run cover
  only the small window part, and a failure of the window part (a CI runner without X libraries,
  Q-7) does not hold back the editor. Cost: one more spec and review cycle; evaluation A waits for
  both.
- **Q-6. Start paths of `kiln script`.** A: positional after the script (as specified; the
  implementer changes the one case of its own T-055 test, `script_t055.rs:599`). B: a repeatable
  `--open <path>` (breaks T-055's "each option at most once"). C: no start paths in script mode
  (the window's start paths are then observed only under Xvfb). **Recommendation: A.**
- **Q-7. A display that exists but whose window system cannot start** (Xvfb without the two
  libraries: winit panics, exit 101, E-b c). A: `kiln` exits 2 with one stderr line and no panic
  message, as for no display (the implementer catches it; the reviewer checks it under Xvfb
  without the libraries; not a gate test). B: accept the toolkit's panic and state it as a limit
  in `ui.md`. **Recommendation: A** (a user on a machine without the libraries gets a message,
  not a backtrace).
- **Q-8. How the colour of a kind is published.** A: the table in this spec and in
  `docs/protocol/ui-editor.md`; the tests carry the seven values copied from the spec (no drafted
  task changes them; themes are out of scope), the reviewer checks the document's table against
  the spec. B: the tests parse the table out of `ui-editor.md` at run time (checks the document
  too, but a test then depends on a document's formatting). **Recommendation: A.**
- **Q-9. The view's reads in the log.** A: they are logged, as every successful request is; the
  idle rule is "no request and no input since the view's last read" (Interface, E5).
  B: an unlogged read path in `kiln_control::Host` (a change of the L4 interface and of the
  log's meaning: its own task before this one). **Recommendation: A**; replay is unaffected
  (reads change nothing) and AC-9 checks that every extra logged request is read-only or a
  command of the Input table.
- **Q-10. A `settled: false` that a test can provoke** (D-152 follow-up). A: pin that the view
  asks for one further frame in the frame in which it re-read after a script's control request
  (AC-10; kills T-055's M2). B: test only that after `settled: true` a further frame changes
  nothing (M2 survives). **Recommendation: A.** On main the shell answers `settled: true` there
  (E-f), so the rule is new behaviour, stated as such.

**Shared with drafted tasks (each re-checks at its spec check)**
- T-057 (project panel): `panels/project.rs`, `PanelView`, the current-document rule (a click
  that opens an already open file makes it current through `already_open: true`), L5 (no label
  containing `Pat` or `Ope`), `scroll_view` for a long tree.
- T-058 (tabs): changes the current-document rule (a tab click without `doc.open`): its
  `touches` must take the rule's place (`docs/protocol/ui-editor.md` and the shell state, today
  only `panels/tabs.rs`); `modified` as in `Title`; after a close the highest open id.
- T-059 (status bar): its reads fall under E5 (first frame after a request only) and are logged.
- T-060 (outline, navigation): E4 covers `nav.goto-*` (any change of the head scrolls); a
  go-to-line field is a `text_input` and needs a label (T-055 AC-10) not containing `Pat`/`Ope`.
- T-061 (find bar): AC-4 requires that nothing but the highlight kind sets a text colour: match
  highlights must be backgrounds, or T-061 amends E3 and this task's tests by a ruling.
- T-062, T-063: add `panels/search_panel.rs`, `panels/diagnostics.rs` and one line in the shell
  file (`crates/kiln-ui/src/…`, named by T-056's implementer); they cannot be in flight together.
- T-025 / the UI module after it (Q-4 A): `Enter` becomes `edit.newline`; AC-6 presses Enter only
  where that gives the same text (no indentation before the caret); E2 changes with it.
  `edit.newline` with `none` inserts `\n` (T-025 (g)), as here.
- T-031 (keymap): the Input table is hard-coded here; a default keymap must keep these rows'
  effects or change T-056's tests by a ruling.
- T-054 (debug profile): both edit the root `Cargo.toml` (`[profile.*]` there,
  `[workspace.dependencies]` here); the second rebases; its disk figures change with eframe
  (E-a).
- T-064: `docs/development.md:33` ("desktop placeholder (exits 1: UI not built)") becomes false;
  this task does not touch that file.

**Evidence** (`$S` = the scratch directory; `K` = its `kiln` build of main `197617c` plus
eframe; `C` = its `kiln-cli`)
- E-a Dependency. `Cargo.toml` `[workspace.dependencies]` `eframe = { version = "=0.37.0",
  default-features = false, features = ["default_fonts", "glow", "x11"] }`, `kiln-ui`
  `eframe.workspace = true`; `cargo metadata` then a set comparison of the two locks (Python
  `tomllib`): `main 156 new 326 added 170 removed 0`, `removed (version changes): []` (14 names
  gain a second version beside main's, e.g. `thiserror`, `toml_edit`, `windows-sys`). `cargo
  tree -p kiln -e normal -i tokio|wgpu|renderdoc-sys` → nothing to print; `-i wayland-sys`,
  `-i accesskit_winit` → no such package. `cargo tree -e features -i egui`: features
  `default_fonts`, `bytemuck`. `/usr/bin/time cargo build -p kiln --locked` (clean): `Finished …
  in 56.42s`, `du -sh target` 1.8G, `kiln` 304,066,160 bytes. `cargo test -p kiln --locked
  --test acceptance_t055 --test guard_t055 --test script_t055` → `26 passed`, `2 passed`,
  `9 passed`. `cargo xtask layers` → `PASS layers: 11 crates, 14 internal edges`; `cargo xtask
  ui-commands` → `PASS`.
- E-b The window. A scratch-only function in the copy's `kiln-ui` ran the T-055 shell with
  `eframe::run_ui_native` (inner size 1280×800, dark theme, no blink) for a few frames:
  (a) `env -u DISPLAY -u WAYLAND_DISPLAY K probe-window proj` → exit 2 by the probe's own
  mapping, 0 bytes stdout, one line `kiln: no window: winit EventLoopError: os error at …`
  (`run_native` returned `Err`; no panic); (b) `DISPLAY=:99` (nothing listening) → the same;
  (c) `xvfb-run -a -s "-screen 0 1280x800x24" K probe-window proj` → **panic**
  `Library libxkbcommon-x11.so could not be loaded.` (xkbcommon-dl, from winit's
  `EventLoop::new`), exit 101; (d) with the libraries: `apt-get download libxkbcommon-x11-0
  libxcb-xkb1` (Ubuntu 24.04: `libxkbcommon-x11-0 1.6.0-1build1`, `libxcb-xkb1 1.15-1ubuntu2`,
  46.8 kB) in an empty directory, `dpkg-deb -x` each into `$S/xlibs/root`, then
  `env -u WAYLAND_DISPLAY LD_LIBRARY_PATH=$S/xlibs/root/usr/lib/x86_64-linux-gnu xvfb-run -a -s
  "-screen 0 1280x800x24" K probe-window proj` → exit 0, 0.61 s and 0.25 s in two runs;
  `pixels_per_point 1`, `content_rect [[0 0] - [1280 800]]` (also with a 1600×1000 screen).
  With an egui plugin whose `output_hook` built the same `Observation` as `kiln script` from
  the frame's accessibility update and shapes, the real window's root equalled the root of `K
  script h.jsonl --root proj` (`doc.open src/a.rs`, `frames 5`, `snapshot`): `equal: True`,
  14 nodes each.
  **Recipe for the reviewer and the evaluator** (nothing installed on the system):
  ```sh
  D=<scratch>/xlibs; mkdir -p "$D/debs" && (cd "$D/debs" && apt-get download libxkbcommon-x11-0 libxcb-xkb1) && for f in "$D"/debs/*.deb; do dpkg-deb -x "$f" "$D/root"; done
  env -u WAYLAND_DISPLAY LD_LIBRARY_PATH="$D/root/usr/lib/x86_64-linux-gnu" MESA_SHADER_CACHE_DISABLE=true timeout 120 xvfb-run -a -s "-screen 0 1280x800x24" sh -c 'exec target/debug/kiln --root d src/a.rs --script s.jsonl 2>stderr.txt' > stdout.txt; echo "exit=$?"
  ```
- E-c Features that do not build: the same with `features = [… "x11", "accesskit"]`: `cargo
  build -p kiln` → `error: failed to run custom build command for wayland-sys v0.31.11` (no
  `wayland-client.pc`), as ADR-0008 c4.
- E-d Home directory: the window run of E-b (d) with `HOME` a fresh scratch directory created
  `.cache/mesa_shader_cache/…`; with `MESA_SHADER_CACHE_DISABLE=true` the directory stayed empty
  (`find | wc -l` → 1).
- E-e Control parts of (g): `C run g.jsonl --root r --log l.jsonl` (28 requests) → among others
  `{"id":2,…"spans":[{"end":2,"kind":"keyword","start":0},{"end":3,"kind":"plain","start":2},{"end":7,"kind":"function","start":3},{"end":11,"kind":"plain","start":7}]}`,
  `{"id":3,…"end":26,"ending":"lf","line":1,"start":12,"text":"    let x = 1;"}`,
  `{"id":6,…"anchor":17,"col":5,…"head":17,"line":1,…}`,
  `{"id":10,…"anchor":13,…"head":14,…}` after `doc.replace {start:13,end:18,text:"Z"}` on anchor
  13/head 18, `{"id":13,…"anchor":14,…"head":13,…}` for the reversed selection,
  `{"id":16,…"anchor":1,…"head":1,…}`, `{"id":17,…"removed":"\r\n"}`, `{"id":18,…"text":"ab"}`,
  `{"id":22,…"text":"x\r\n\r\ny"}`, `{"id":24,…"line_ending":"none",…}` (text `abc`),
  `{"id":25,…"already_open":true,"doc":2,…"line_ending":"none",…}`, `{"id":26,…"dirty":false,…"undone":true}`,
  `{"id":27,…"can_redo":true,…"line_ending":"crlf",…}`; exit 0; the log has 28 lines, 12 of them
  read-only (`doc.text`, `cursor.get`, `syntax.highlight`, `doc.info`, `doc.line`). Refused save:
  `src/.kiln-tmp-0` … `-99` created, `C run s3.jsonl --root r3` →
  `{"error":{"code":"rejected","kind":"io_error","message":"cannot save src/a.rs: I/O error: all 100 temporary file names are taken"},"id":3,"ok":false}`,
  then `doc.info` `"dirty":true`; the file still `fn main() {}`; exit 3. Directory removed
  between requests (`kiln-cli control` driven by a script): `doc.save` → `rejected` /
  `not_found`.
- E-f Rows and settling on main's script mode (`K script t.jsonl --root r`): `type`
  `"a\tb c é λ ж 中 😀"` into `Path` → its row text is exactly that string (tab and non-ASCII
  chars are reported); a control `doc.open` then `{"ui":"frames","count":1}` →
  `{"frames":1,"settled":true}` (so AC-10 fails on base).

**Not checked**
- The CI runner (`ubuntu-24.04`): whether winit there finds `libX11` and returns an error
  without `DISPLAY` (here it does, E-b a); AC-2 runs in the gate there.
- The editor itself: no implementation exists; settle counts, the cost of a keystroke, whether
  the toolkit's accessibility scroll action reaches a custom editor node, and how `selection`
  is reported for a custom widget (`observe.rs` computes it from `text_run` children) are the
  implementer's to show.
- A real window driven by a script (the probe drew frames without input); the X pointer's
  initial position under Xvfb could give a hover colour that headless runs do not have
  (none seen for the shell).
- Release builds; Wayland.

### Rulings on spec check R-237 (director, D-156)
- **Q-5: A, split.** T-056 becomes the **editor view** only: it claims `ui.editor-view` and `ui.dispatch-only`, is
  observed through `kiln script`, adds no dependency, tier medium, budget 800,000, independent test author (even
  task number). The **window** (the windowing dependency, starting `kiln`, the exit without a display, `--script`
  in the window; `ui.window`) becomes T-065, high tier, 1,100,000, depending on T-056; it gets its own spec check
  when T-056 has merged. The planner moves the window parts into `tasks/T-065-*.md` and leaves no window criterion
  here.
- **Q-6: A.** Start paths of `kiln script` are positional paths after the script; the implementer changes the one
  case in `apps/kiln/tests/script_t055.rs` that expects exit 2 for a second positional argument (it is in the
  implementer's `touches`).
- **Q-7: A** (goes with T-065). A display that is present but cannot give a window ends with exit status 2 and one
  line on stderr, without a panic; checked by the reviewer under Xvfb without the two libraries.
- **Q-8: A.** The colour of each highlight kind is published as a table in this spec's Pinned section and in
  `docs/protocol/ui-editor.md`; tests take the values from the spec. A later task that changes a colour changes the
  table, the document and those tests together.
- **Q-9: A.** The view's reads are control requests and are logged; "idle" means no request and no input since the
  view's last read, and then frames add nothing to the log.
- **Q-10: A.** After a script's control request the view re-reads on the next frame and asks for one more frame,
  so a `frames` request with `count` 1 right after it answers `settled: false`; AC-10 tests it.
- Blockers B1 to B11 are accepted as the spec check fixed them.

### Spec check R-238
Planner run R-238 (`iter-2`, spec check), against main `e0926ae` (D-156). Applied the split of
Q-5 A; the sections above are kept as the record (they describe the spec before the split, so
their AC numbers and touches are the old ones).
- **File:** `tasks/T-056-window-and-editor-view.md` became `tasks/T-056-editor-view.md` (id
  unchanged); the window is `tasks/T-065-window.md`.
- **Front matter:** title; tier medium; items `ui.editor-view`, `ui.dispatch-only`; budget
  800,000; `touches` lost `Cargo.toml`, `Cargo.lock`, `crates/kiln-ui/Cargo.toml` (no
  dependency); every remaining path has one role.
- **Moved to T-065:** the `kiln` window command line, `--script` for the window, the exit
  without a display (Q-7 A), eframe and its features, window size and style, the GL cache
  (Pinned (b), (e)), E6 and the Xvfb limits, old AC-2 and AC-3, the evaluator's window call,
  the recipe and evidence E-a to E-d (copied there), blockers B10 and B11, the Q-2 ruling
  (copied there as the item's rule).
- **Kept here:** start paths of `kiln script` (Q-6 A, folded into Interface), the current
  document, `Title`, `Editor`, colours (Q-8 A folded in), input, the view's reads and idle
  (Q-9 A folded in), `settled: false` (Q-10 A folded in), the panel files, L1–L8.
- **New:** "Shared with T-065": the one frame function that both `kiln script` and the window
  call (named by this task's implementer, pinned at T-065's spec check).
- **Criteria renumbered:** old AC-1 → AC-1 (without the window), AC-4 → AC-2, AC-5 → AC-3,
  AC-6 → AC-4, AC-7 → AC-5, AC-8 → AC-6, AC-9 → AC-7, AC-10 → AC-8. Old AC-2 and AC-3 are
  T-065's AC-1 and AC-2. Nothing was dropped.
- **Collisions, updated:** T-057 to T-061 depend on T-056 (panel files, `PanelView`, current
  document, L5) as listed in R-237; against T-065 they collide only if T-065's `touches` stays
  the prefix `crates/kiln-ui/src/` (T-065's spec check narrows it). T-062, T-063: as R-237.
  T-025, T-031: against T-056 (Enter, the Input table). T-054 and T-064 now collide with T-065,
  not with T-056 (root `Cargo.toml`; `docs/development.md:33` "exits 1" stays true after T-056).
  T-065 and T-056 share `crates/kiln-ui/src/`, `apps/kiln/src/` and `docs/protocol/ui.md`:
  sequential (`depends_on`).

### Rulings on the test author's questions of R-240 (director, D-157; told to both builders)
- **x1 (T1).** After Enter the caret is at the start of the line that follows the break Enter made. Where the
  inserted line ending and the char after it form one line break together (a `cr` document, Enter typed with a
  `\n` right after the selection's end), the caret is after that whole break; the UI sends no `cursor.set` that
  the session would refuse, and `Message` shows no refusal. The acceptance tests avoid the case; the implementer
  covers it with a test of its own and the reviewer checks it.
- **x2 (T2).** The `doc.open` of each start path is sent by the invocation: it is in the session log, in the order
  of the command line, before the script's first request, and counts as input of the session for `ui.dispatch-only`
  (the log replayed with `kiln-cli run` reproduces the files with them included).
- **x3 (T3).** In a row that holds only blanks (spaces, tabs), the blanks are reported in the colour of `plain`.

### Rulings on the implementer's questions of R-239 and re-tiering (director, D-158)
- **Tier: high.** The task claims `ui.dispatch-only`, whose scorecard tier is high; a task's tier is not below the
  tier of an item it claims. Consequences: an approval row before the queue; the reviewer's round 1 includes the
  high-tier probing; budget 1,100,000. The independent test author is already in place.
- **y1 (question 1).** A start path is opened as if it had been opened through `Path`, but its text is not put
  into the `Path` field. Confirmed.
- **y2 (question 2).** The settle figure in the "Limits" sentence is re-measured on the acceptance scripts by the
  implementer after the join (it may then read the acceptance tests), and the sentence carries the measured figure.
- **y3 (question 3).** "A script of control requests only runs no read" holds for an invocation without start
  paths; with a start path the frames before the first request read. The narrowed sentence stands.
- **y4 (deviation).** `Title`'s value is the path followed by ` — Kiln`, or by ` — modified — Kiln` when the
  document is dirty, so that it never equals a `Documents` entry. Accepted; `docs/protocol/ui-editor.md` states it,
  and the spec's sentence on `Title` reads so from here on.
- **y5 (defect seen in merged code).** The implementer saw a snapshot report the painted text of `Title` with two
  letters exchanged (`modifeid` for `modified`), in the row-merging code of T-055's observation. A row's text must
  be the chars as painted, in reading order. It is fixed in this task (the file is in the implementer's
  `touches`), after the reviewer has reproduced it; the test author adds a test that, for every node that shows
  text in proportional type (`Title`, `Path`, `Open`, `Message`, `Documents`' children), the text of its rows
  equals the text it shows.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-14 · R-225 · (new) · did: specified the window and the editor view, split from T-055 (the
  observation interface) as the brief asks. Tier high: eframe is a dependency change (no gate
  path). Items `ui.window`, `ui.editor-view`, `ui.dispatch-only`, each with criteria through
  `kiln script`. AC-3 cannot run where the gate runs (no display; ADR-0008 d) and is stated as a
  reviewer's and evaluator's check, not a gate test (Q-2). The colour table is a contract this
  task sets (no drafted task changes it). Pinned (b) fixes one file per later panel so that
  T-057 to T-061 can be in flight together. (g)'s control parts executed with `kiln-cli` at
  `ec08d22`; the UI parts derived from the Input table and `docs/protocol/cursor.md`. T-055 was
  not merged when this was written: spec check required. · gate attempts: 0 · missing context:
  T-055's merged shell and `docs/protocol/ui.md` · next: director (Q-2, Q-4).
- 2026-11-14 · R-237 · draft · spec check: did: read the spec against main `197617c` (T-055
  merged), its tests, guards, `docs/protocol/ui.md`, `observe.rs`, `script.rs`, `shell.rs`,
  `session.rs`, `apps/kiln/src/main.rs`, the protocol documents and D-145 to D-155; no task in
  flight. Rulings Q-2 A and Q-4 A (D-147) and the follow-ups of D-152 acknowledged and accepted;
  the three follow-ups are AC-10, AC-4, AC-5; the guard of w4 needs no change (exit 2 is
  non-zero). Found 11 blockers (B1–B11, all fixed in the text, B4 and B6 with questions Q-6 and
  Q-9) and wrote Q-5 (split, recommended), Q-6 to Q-10. Measured in scratch: eframe builds with
  main's toolchain and changes no version in the lock; no display → `Err` without panic; Xvfb
  without the two libraries → panic (Q-7); with them the window starts and its observation
  equals `kiln script`'s; the GL driver writes a cache under `$HOME` unless disabled; the
  control parts of (g) re-executed (two corrections). Tier stays high (dependency change);
  under Q-5 A the editor part would be medium. Pinned lines: none n/a. Unsure: the CI runner,
  and how a custom editor widget reports `selection` and takes the scroll action (Not checked).
  · gate attempts: 0 (no build in the repository) · not run: any build of an editor (none
  exists); the CI runner · missing context: none · next: director (Q-5 to Q-10; then
  `specified`).
- 2026-11-14 · R-238 · draft · spec check: did: applied D-156 (Q-5 to Q-10, all A,
  acknowledged and accepted): split the window off into T-065, renamed this file
  (`T-056-editor-view.md`), folded Q-6, Q-8, Q-9 and Q-10 into the spec text, renumbered the
  criteria (8), set tier medium (no dependency, no gate path, no own file write; the criterion
  that decided high, the eframe dependency, moved to T-065) and budget 800,000; added the frame
  function shared with T-065. Pinned (e) and (h) narrowed to the editor. · gate attempts: 0 ·
  not run: no build (none needed) · missing context: none · next: director (`specified`).
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
