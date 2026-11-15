---
id: T-057
title: "Show the project tree in a panel beside the editor: expand and collapse directories, and open a file as the current document by clicking it"
tier: medium            # ADR-0002 default (UI behaviour over existing commands; no dependency, gate path, persistence or process code); ≥ the claimed item's tier (`ui.project-panel`: low); fixed at specified (METHODOLOGY v1.4 §4)
state: queued                 # director only, written with every transition; see METHODOLOGY §3
owner_role: planner
scorecard_items: [ui.project-panel]
depends_on: []          # T-055, T-056, T-065 merged (main 15555cc); nothing unmerged
touches:                # one role per path (D-153); no gate path
  - crates/kiln-ui/src/panels/project.rs   # implementer: the panel (created empty by T-056)
  - crates/kiln-ui/src/shell.rs            # implementer: the `scroll_view` helper's reported bounds only (Pinned (f) F1); nothing else in the file
  - apps/kiln/tests/acceptance_t057.rs     # implementer, on task/T-057-tests first (odd arm, METHODOLOGY §6)
  - apps/kiln/tests/project_t057.rs        # implementer: its own tests through the binary
  - docs/protocol/ui-project.md            # implementer: new; the sentences of Pinned (i)
budget_tokens: 800000   # medium (v1.4)
iteration: 2
found_by: planner       # ADR-0007 row 7 (run R-225); specified in run R-262
origin_task: null
---

# T-057 The project panel

## Goal
After this task the Kiln UI (the window and `kiln script`) has a project panel beside the editor.
It starts collapsed. A click on its toggle lists the root's entries as `project.tree` lists them.
A click on a directory expands or collapses it. A click on a file, link or special entry asks the
session to open it (`doc.open`), which makes it the current document (T-056's rule). The panel
reads only when the user expands something, so frames on an idle UI still add nothing to the log.
It also fixes a fault in the merged observation that a side panel exposes: a vertical scroll
area's reported bounds span its parent's full width (F1).

This claims `ui.project-panel` ("The project tree panel opens a file on click"). It is tier medium,
odd-numbered, so the **implementer-first arm** applies: the implementer writes the acceptance
tests first on `task/T-057-tests` and runs the whole gate on a local merge before reporting
(METHODOLOGY §6). No gate path. `apps/` is a platform-sensitive path (ADR-0009 Decision 5): once
the platform jobs exist (T-070), the rebased head goes to `ci/T-057` before the queue.

Reading list: `docs/adr/0008-ui-observation.md` ("What counts as observed"); `docs/protocol/ui.md`
(targets, `scroll`, snapshot rules, Limits); `docs/protocol/ui-editor.md` (the current document;
"Frames, reads and the log"); `docs/protocol/project.md` (the whole document);
`crates/kiln-ui/src/panels/mod.rs`, `panels/project.rs`, `shell.rs`; this file's Pinned (f)
(the facts the merged tests rely on).

## Interface

No new registry command, no new UI request kind, no new refusal kind, no new command-line option.

### What the panel shows (nodes in a `kiln script` snapshot)
- **The toggle**: a node of role `button` labelled `Project`, with `toggled: false` when collapsed
  and `toggled: true` when expanded. Its one painted row is exactly `Project` (F2: any arrow or
  marker is drawn as a shape, not as text in the button).
- **When expanded**, a node of role `tree` labelled `Project files`, inside a node of role
  `scroll_view` that contains no shell node (`Path`, `Open`, `Message`, `Documents`), no `Title`
  and no `Editor`. Below it, in the snapshot's depth-first order, one node of role `tree_item`
  per **shown entry**:
  - `label`: the entry's root-relative path exactly as `project.tree` reports it (`/` between
    segments on every platform);
  - `value`: the entry's name (its last path segment), which is also its painted text;
  - `toggled`: present only on a `dir` entry: `true` when expanded, `false` when collapsed. An
    entry of kind `file`, `symlink` or `other` has no `toggled`. That is how a directory is told
    from the rest;
  - the shown entries are, in order: the root's entries (expanded root), and after each expanded
    directory its own shown entries, recursively. So the order is `project.tree`'s preorder
    restricted to what is shown. How the nodes are nested, indented or drawn is not specified.
- **Truncated listing**: when the read that listed a directory answered `truncated: true`, one
  node of role `label` with no `label` whose `value` begins with `truncated` follows that
  directory's last shown entry (Q-1 A). Which entries are shown is then what that read listed
  (`project.tree`'s rule 6: preorder up to `limit`, so later entries of the directory can be
  missing). For the root, the marker follows the root's last shown entry.
- **When collapsed**: the toggle only. No `tree`, no `tree_item`, no marker.
- The panel never shows the root's host path or name.

### Where it sits
The shell calls `panels::project::show` "beside the editor, on its left" (`panels/mod.rs`). Tests
assume no position, size or width: they find nodes by role and label and use `scroll` before
clicking an `offscreen` entry. The panel's width is the implementer's. It must keep every merged
test green, F1 to F2 and L1 to L8 below included (evidence: a 180-point prototype did, with F1
fixed, Spec check R-262 E2).

### UI requests that act on it
All are existing requests (`docs/protocol/ui.md`); nothing here changes how they work.
- `click` on `Project`: collapsed → expanded (reads, below); expanded → collapsed (sends nothing,
  forgets every directory's expanded state).
- `click` on a `tree_item` with `toggled`: collapsed → expanded (reads); expanded → collapsed
  (sends nothing; its descendants are no longer shown and their expanded state is forgotten).
- `click` on a `tree_item` without `toggled`: sends `doc.open {"path": <label>}`. On success
  the document is the current one (T-056: an open that answers `already_open: true` makes it
  current too), so `Title` and `Editor` show it, and `Message` is cleared. On a refusal nothing
  else changes, and `Message` begins with the refusal's kind (`not_found`, `not_a_file`,
  `outside_root`, `io_error`, `decode_error`, …, as `doc.open` publishes). A FIFO or a device
  is refused (`not_a_file`) without blocking.
- `scroll` on `Project files` or on any of its entries scrolls the panel's list. A secondary
  click, a click with modifiers or with `at` on an entry does what a primary click does: the
  panel distinguishes no modifier. Keyboard navigation in the tree is out of scope.
- No entry takes keyboard focus by itself (L4).

### Control requests the panel sends, and when
- **Reads**, read-only and logged like every successful request (`docs/control-protocol.md`): in
  the frame that handles a click that expands, exactly one `project.tree` read. For the root it
  is `project.tree {}`; for a directory it is `project.tree {"path": <its label>}`. Both use the
  default `limit`. The directory's shown entries are the read's entries one segment below it
  (for the root, the entries without `/`), in the read's order, all collapsed.
- **Opens**: `doc.open {"path": <label>}` on a click on a non-directory entry. It counts as a
  request, so the editor view re-reads as T-056 specifies.
- **Nothing else**: no read on start, none after a script's control request, none after the
  UI's own opens or saves, none on an idle frame (T-056's definition: no request and no input
  since the last read). A script of control requests only therefore still writes
  `Host::log_script()` (L7).
- A refused read is not logged (control protocol). The directory (or the root) stays collapsed.
  `Message` begins with the kind (`not_found`, `not_a_file`, `outside_root`, `io_error`,
  `invalid_path`), because the user asked for it. This differs from the editor's own background
  reads, which never set `Message`.

### When the tree on disk changes
There is no watching (Q-3 A). What is shown is what the last read of each expanded directory
returned. It is brought up to date only when the user collapses and expands that directory (or
the root) again, which sends a fresh read. A click on an entry that has since vanished or changed
kind is answered by `doc.open` or `project.tree` as they answer it (for example `not_found`). That
refusal shows in `Message` and the stale entry stays until the next expand. A file the UI's own
`doc.save` creates (a save creates a missing file, `docs/protocol/paths.md`) appears after the
next expand of its directory.

### Platforms (ADR-0009)
The panel adds no platform difference. It shows `project.tree`'s paths and kinds verbatim, with
`/` as the separator and in `project.tree`'s byte order with no case folding, on every platform.
Any difference in what is listed is `project.tree`'s and is stated (if at all) in
`docs/protocol/project.md` by the platform tasks (T-074, T-075), not here. Evidence on Linux is the
gate. For macOS and Windows it comes from the CI jobs once T-070 has merged (`ci/T-057`, read
through the director's record). Until then the panel is checked on Linux only. Entry kinds that a
platform lacks (FIFOs on Windows; symbolic links where the test cannot create them) are covered by
tests that follow the platform-test rule (T-071's registry if merged at dispatch, ADR-0009
Decision 4).

### Rust items the panel builds on (on main `15555cc`; not used by tests)
```rust
// crates/kiln-ui/src/panels/project.rs (the one function the shell calls; signature kept)
pub(crate) fn show(ui: &mut egui::Ui, view: &mut PanelView<'_>);

// crates/kiln-ui/src/panels/mod.rs (unchanged by this task)
pub(crate) struct PanelView<'a> { /* session: &'a mut Session, message: &'a mut String */ }
impl<'a> PanelView<'a> {
    pub(crate) fn new(session: &'a mut Session, message: &'a mut String) -> Self;
    pub(crate) fn request(&mut self, command: &str, params: serde_json::Value)
        -> Result<serde_json::Value, (String, String)>;   // (kind, message) on a refusal
    pub(crate) fn read(&mut self, command: &str, params: serde_json::Value)
        -> Result<serde_json::Value, (String, String)>;
    pub(crate) fn current(&self) -> Option<u64>;
    pub(crate) fn set_current(&mut self, doc: u64);
    pub(crate) fn open_documents(&self) -> &[(u64, String)];
    pub(crate) fn set_message(&mut self, text: String);
}

// crates/kiln-ui/src/shell.rs (F1 changes the bounds it reports, not the signatures)
pub(crate) fn scroll_view(ui: &mut egui::Ui, id_salt: &str, add: impl FnOnce(&mut egui::Ui));
pub(crate) fn scroll_view_with(ui: &mut egui::Ui, id_salt: &str, offset: Option<f32>,
    add: impl FnOnce(&mut egui::Ui)) -> (f32, egui::Rect);
```
`show` gets no state of its own: the panel keeps what it shows between frames itself, inside
`project.rs` (the prototype used the toolkit's per-id memory). `PanelView` is not edited: a panel
that needs more asks (`QUESTION:`), since T-058 to T-061 share it.

### Rust API used by tests
None. All criteria are tested through the built `kiln` binary. `kiln_control::Host` is the only
Rust API a test may use, as an oracle, with T-055's pinned signatures (`on_disk`, `in_memory`,
`handle_line`, `log_script`; unchanged on main). `apps/kiln/Cargo.toml` already has
`kiln-control` and `serde_json` as dev-dependencies and is not edited. A test that needs a
random-number generator writes its own (as `acceptance_t056.rs` does); no new dependency.

## Pinned
- (a) Errors: no new kind. Refusals come from `doc.open` and `project.tree` as published
  (`docs/commands.json`: `doc.open` `invalid_path`, `not_found`, `not_a_file`, `io_error`,
  `decode_error`, `outside_root`; `project.tree` `invalid_path`, `outside_root`, `not_found`,
  `not_a_file`, `io_error`). Their precedence is the commands' own. A refusal leaves the session
  and the panel as they were (a directory stays collapsed) and sets `Message` to begin with the
  kind. Exit codes of `kiln script` are unchanged: no UI-initiated request gets a response line.
- (b) Files: none read or written by the panel itself. Everything goes through `project.tree`
  and `doc.open`. Source: the panel stays in `panels/project.rs`. The only other code edit is F1
  in `shell.rs`.
- (c) Order: in a frame that handles a click on an expandable node, the read is sent in that
  frame and the new entries are shown from that frame or the next (within the action's
  settling). A click on a file sends `doc.open` in the frame that handles it. The editor shows
  the new current document from the next frame, as for `Open`.
- (d) Paths: root-relative, as `project.tree` gives them. They are used unchanged as `doc.open`'s
  and `project.tree`'s `path`. Platforms: see Interface; the evidence is Linux (gate) until T-070.
- (e) Threat model: the panel reaches only what the two commands reach (confined to the root,
  `docs/protocol/paths.md`; links listed, never followed by `project.tree`; special files never
  opened by it, and refused by `doc.open` without blocking). It reads only on a user's click. A
  hostile tree costs one `project.tree` per expand: at most `limit` (50,000) entries listed, with
  the time bound `project.md` says does not exist for `.gitignore` files. Measured: 50,200
  entries, 0.50 s for the read and 0.67 s for the whole expanding script, debug build, about
  108 MB resident (E3).
- (f) Facts the merged tests rely on, which this task keeps (none is a layout this task's tests
  may assume):
  - **F1 (fixed by this task)**: on main, a `scroll_view`'s reported bounds are the toolkit's clip
    rectangle of the scroll area's content. For a vertical scroll area that rectangle spans the
    parent's full width, not the area's. With a panel beside the editor, a panel char's centre
    then lies in the editor's `scroll_view`, which is deeper, so the char is attributed to it.
    Measured: `z1_rows_equal_the_text_each_node_shows` (`acceptance_t056.rs:2813`) fails with the
    `Project` row reported in the editor's `scroll_view` (E1). After this task a `scroll_view`'s
    bounds are its visible part: the area's own horizontal extent cut by the window and by every
    `scroll_view` above it. That is what `docs/protocol/ui.md` already says ("whose bounds are
    the visible part of the area"), so no sentence changes. Scrolling, `offscreen` and the rows
    of `Documents` and `Editor` are unchanged (all 93 tests of `apps/kiln` and 21 of `kiln-ui`
    passed with the prototype fix, E2).
  - **F2**: every node with `rows` other than `Editor` has exactly one painted row, equal to its
    `value`, else its `label` (`z1_rows_equal_the_text_each_node_shows`). The toggle paints
    exactly `Project`; an entry paints exactly its name.
  - **L1–L8** as listed in `tasks/T-056-editor-view.md` Pinned (f). In particular: L4, nothing is
    focused until a script focuses or clicks; L5, no new node is labelled `Path`, `Open`,
    `Documents` or `Message` and no new label contains `Pat` or `Ope` in a collapsed panel (an
    expanded panel shows paths as labels, so a project with a file named `Open` would match
    `{"label":"Open"}`: scripts add `role`; documented, (i) P6); L7, start collapsed, no read
    before an input; L8, settles within 60 frames (measured 4 to 15, E3).
  - Merged tests target `{"value":"docs/document-000.txt"}`, `{"value":"f00.txt"}` (the
    `Documents` entries) and `{"value_contains":"."}` (expecting `ambiguous_target`). An entry's
    `value` is its name, not its path (Q-5 A), and the panel is collapsed in those sessions, so
    none of them changes.
  Not to be pinned by this task's tests (golden rule 9): a whole snapshot, the panel's position
  or width, the nesting or indentation of entries, any message text beyond its leading kind, the
  text after `truncated`, which reads the editor view sends, the command list, `error_kinds`.
- (g) Worked examples, executed with the `kiln` binary built from main `15555cc` (with the
  throw-away prototype panel, Spec check R-262; the control responses are main's code). Root
  `proj/`: `.gitignore` (`*.log`), `a.log`, `target/t`, `src/a.rs` (`fn main() {}\n`),
  `dir with space/é.txt`, `e`+U+0301+`x.txt`, `שלום.txt`, `link.rs -> src/a.rs`, a FIFO `fifo`:
  ```text
  {"id":1,"command":"project.tree"}
  → {"id":1,"ok":true,"result":{"entries":[{"kind":"file","path":".gitignore"},{"kind":"dir","path":"dir with space"},{"kind":"file","path":"dir with space/é.txt"},{"kind":"file","path":"éx.txt"},{"kind":"other","path":"fifo"},{"kind":"symlink","path":"link.rs"},{"kind":"dir","path":"src"},{"kind":"file","path":"src/a.rs"},{"kind":"file","path":"שלום.txt"}],"ignored":2,"skipped":0,"too_deep":0,"truncated":false}}
  {"id":2,"command":"project.tree","params":{"path":"src"}}
  → {"id":2,"ok":true,"result":{"entries":[{"kind":"file","path":"src/a.rs"}],"ignored":0,"skipped":0,"too_deep":0,"truncated":false}}
  {"id":3,"command":"doc.open","params":{"path":"fifo"}}       → error rejected / not_a_file
  {"id":4,"command":"project.tree","params":{"path":"gone"}}   → error rejected / not_found
  {"id":5,"command":"project.tree","params":{"path":"src/a.rs"}} → error rejected / not_a_file
  {"id":6,"command":"doc.open","params":{"path":"link.rs"}}    → ok, doc 1, path "link.rs"
  ```
  So after `click` on `Project`, the shown entries are, by label: `.gitignore`, `dir with space`
  (toggled false), `éx.txt`, `fifo`, `link.rs`, `src` (toggled false), `שלום.txt`. Not `a.log`,
  `target` or `dir with space/é.txt`. After `click {"role":"tree_item","label":"src"}`, `src`
  is `toggled: true` and `src/a.rs` (value `a.rs`) follows it. `click` on `src/a.rs` makes doc 1
  current. `click` on `fifo` gives `Message` `not_a_file…` at once. 600 idle frames add nothing.
  The prototype's log was `project.tree {}`, `project.tree {"path":"src"}`, `doc.open src/a.rs`,
  then the editor's reads, `doc.open link.rs`, the editor's reads (E3).
- (h) Evaluator calls for `ui.project-panel` (ADR-0008 (O1)–(O5)): `kiln script --root d` with
  `click` on `Project`, on directories and on files (scrolling first where `offscreen`).
  Snapshots before and after. Checked against `project.tree` in the same session, the directory
  on disk, and `git ls-files --cached --others --exclude-standard` in a fresh repository where
  `project.md` says the two agree. After a file click: `doc.list`, `Title`, `Editor` value equal
  to `doc.text`, and the `doc.open` in `--log`. Inert check: every entry and the toggle change
  something (O5). Where a display exists, the same through `kiln --script` (T-065).
- (i) Sentences for `docs/protocol/ui-project.md`, worded for copying, each with its evidence:
  - P1 "The panel starts collapsed and sends nothing until the user clicks it: a script of
    control requests only writes the same log as `kiln-cli run`." Evidence: L7 tests stay green
    (E2); AC-6.
  - P2 "Expanding the root or a directory sends one `project.tree` read (for the root without
    `path`, for a directory with its path), and the entries shown under it are that read's
    entries one level below it, in its order. Collapsing sends nothing." Evidence: E3 log; AC-2,
    AC-3.
  - P3 "A click on an entry that is not a directory opens it with `doc.open` and makes it the
    current document; a refusal leaves everything as it was and `Message` begins with its kind."
    Evidence: E3 (`fifo` → `not_a_file`, `link.rs` opened); AC-4, AC-5.
  - P4 "The panel does not watch the disk: what it shows is what the last read of each expanded
    directory returned, until that directory is collapsed and expanded again." Evidence: AC-7
    (each expand logs a fresh read, and the shown entries are that read's).
  - P5 "When the read that listed a directory was truncated, a node whose value begins with
    `truncated` follows its last shown entry, and entries of that directory after the cut are not
    shown." Evidence (E4): a root with `a/` holding 50,000 files and `z.txt` lists 50,000
    entries, `truncated: true`, and no root entry after `a`. AC-8.
  - P6 "An entry's label is its root-relative path; with the panel expanded, a target given by
    label alone can also match an entry (a file named `Open` at the root matches
    `{"label":"Open"}`): give `role` as well." Evidence: Interface; AC-1.
  - P7 "Paths are shown as `project.tree` reports them, with `/` on every platform." Evidence:
    Linux gate (AC-1). macOS and Windows: CI once T-070 merges (a sentence stays general only if
    tested on all three, ADR-0009 Decision 4).
  - Limits, copied: "Expanding a large directory costs one `project.tree` of up to 50,000
    entries (0.5 s for 50,200 entries in a debug build, measured once on a 4-core Linux machine;
    no time bound is stated)." "The rows of a name that begins with right-to-left text can lack
    chars (`docs/protocol/ui.md`, Limits); its `label` and `value` are exact."

## Acceptance criteria
All tests drive the built `kiln` binary (`kiln script`) with temporary roots, every process
under a timeout. Nodes are found by role and label (or value), never by a whole snapshot or a
position. "Shown entries" means the `tree_item` nodes below `Project files` in snapshot order.
Each AC fails on base, because no node labelled `Project` exists there (`no_target` at the first
`click`). The odd arm's implementer commits them failing on `task/T-057-tests` first.
- AC-1 (`ui.project-panel`, root): fresh session, `click` `Project`. The toggle becomes
  `toggled: true`. The shown entries' labels equal, in order, the entries of a control
  `project.tree {}` in the same session whose path has no `/`. Each `dir` entry has
  `toggled: false` and no other has `toggled`. Each `value` is the label's last segment. Before
  the click there is no `tree_item` and the toggle is `toggled: false`. Oracle: `project.tree`
  (differential, same session).
- AC-2 (directories): for random sequences of expand and collapse clicks (scrolling first when
  `offscreen`), the shown entries after each step equal the naive model: the preorder of a
  control `project.tree {}`, restricted to entries whose every ancestor directory is expanded in
  the model (collapse forgets descendants). The log holds exactly one `project.tree` read per
  expand click, with the params of P2, and none per collapse. Oracle: naive model plus log.
- AC-3 (generated trees, D-174 change 5): for at least 40 seeds, a random tree (depth ≤ 4, ≤ 8
  entries per directory, some empty directories, some `.gitignore` files with patterns only of
  `project.md`'s guaranteed forms) with names drawn from a stated alphabet: ASCII letters and
  digits, inner spaces, `-`, `_`, `.` (not trailing), precomposed `é`, `e`+U+0301, a zero-width
  joiner, `ffi`, CJK `漢字`, emoji `😀`, Hebrew `שלום`, Arabic `سلام`, Arabic-Indic `٣`. The
  generator never makes two names in one directory that are equal ignoring case or under
  Unicode normalization (so the tree is the same on every platform's file system), and never
  uses `\`, `/`, NUL, `:*?"<>|` or a trailing space or dot. Expand everything (root, then every
  directory as it appears). Then (i) the shown labels equal `project.tree {}`'s entries in
  order; (ii) the `file` and `symlink` entries equal `git ls-files --cached --others
  --exclude-standard` of a fresh repository of the same tree with no global or system
  configuration (`project.md`'s stated agreement); (iii) labels and values are compared exactly
  for every name, while rows are compared only for names that begin with neither right-to-left
  text nor an Arabic-Indic digit (`ui.md` Limits; T-066); (iv) clicks on 5 random files open
  them (oracle as AC-4). Seed and counts are printed on failure. Oracle: `project.tree`, git,
  the generator's own tree.
- AC-4 (`ui.project-panel`, open on click): a `click` on a file entry gives a `doc.list` with that
  path, a `Title` value that begins with it, an `Editor` value equal to `doc.text` of it, an empty
  `Message`, and a `doc.open {"path": <label>}` in `--log`. Clicking an already open file
  makes it current again (`Title`) without a new document. Clicking a second file then the first
  switches `Title` both times. Replaying the `--log` with `kiln_control::Host` on a copy of the
  root gives the same `doc.list`. Oracle: registry state, replay.
- AC-5 (refusals): clicking a FIFO (`not_a_file`, answered within the action's settling), a link
  to a directory (`not_a_file`), a link outside the root (`outside_root`) and a file with invalid
  UTF-8 (`decode_error`) each leave `doc.list` unchanged. `Message`'s value begins with the kind a
  control `doc.open` of the same path reports in a second session, and no `doc.open` of it is in
  the log. Unix-only cases follow the platform-test rule (Interface, Platforms). Oracle:
  differential.
- AC-6 (idle and reads): (a) a script of control requests only, run with the panel present,
  writes `Host::log_script()` (as T-055 AC-1). (b) 600 frames after an expanded panel and an
  opened file add nothing to the log. (c) After `click` `Project`, collapse and expand again,
  the log holds two `project.tree {}` reads and nothing else from the panel. Oracle: log
  comparison.
- AC-7 (fresh reads, no watching): in one session, expand `Project`, expand a directory,
  collapse and expand it again, and send a control `project.tree` of the same path right after
  each expand. Each expand logs its own read, and the shown children equal the children in the
  control response that follows it. No read is logged between expands, however many frames run.
  A disk change by another process during a session cannot be scripted deterministically: a
  script is read whole at start, and no Kiln command creates or removes a file a panel would
  list. So the change itself is checked by the reviewer once, as a recorded procedure, not a gate
  test: in the window (or `kiln script` started on a tree, then a second run), create and remove
  files from a shell between a collapse and an expand. Oracle: `project.tree` (differential);
  procedure.
- AC-8 (truncated): a root with `a/` holding 50,001 files and a file `z.txt`. After `click`
  `Project`, the shown root entries are exactly `a`, followed by one node of role `label` whose
  value begins with `truncated`, and `z.txt` is not shown. In a small tree no such node exists.
  Oracle: `project.tree`'s rule 6 (a control `project.tree {}` answers `truncated: true` with no
  root entry after `a`). (Directory creation of 50,001 empty files: the test cleans up.)
- AC-9 (refused reads): a root holding a directory that cannot be listed (mode 000). A control
  `project.tree {}` then fails with `io_error` (`project.md`: an unreadable directory fails the
  whole command). `click` `Project` leaves the toggle `toggled: false` with no `tree_item`.
  `Message` begins with the kind the control request reports, and no `project.tree` read is in the
  log. Where the identity is exempt from mode bits (uid 0 with the DAC capabilities), the control
  request succeeds, and the test then asserts the panel shows the entries, never passing without
  a check. Windows has no mode bit: the case follows the platform-test rule. Oracle: the control
  `project.tree` of the same session (differential).
- AC-10 (F1, the shell fix): with the panel expanded at the default size, every char of a painted
  row inside the panel is attributed to a panel node (no `scroll_view` holding `Editor` has
  `rows`). With the panel expanded, F2 holds: every node with `rows` other than `Editor` has one
  row equal to its value or label. The rows of `Editor` still equal the document's lines from
  line 0 for a file of short lines. Oracle: snapshot walk, `doc.line`. (On base the first part
  cannot run: AC-1's `no_target`.)

## Out of scope and implementer notes
- Out: keyboard navigation in the tree; showing kinds (link, special file) other than directory;
  showing `ignored`, `skipped`, `too_deep`; a depth or one-level parameter for `project.tree` (a
  change of a verified command: Q-1 C); creating, renaming, deleting files; drag and drop;
  watching the file system; remembering the panel's state across sessions; context menus; the
  panel's width or resizing; icons. Making `Documents` entries clickable (T-058, D-169).
- Do not edit `panels/mod.rs` (`PanelView` is shared with T-058 to T-061), `observe.rs` (T-066),
  `script.rs` or `window.rs` (T-068), `docs/protocol/ui.md` (T-066, T-068), or `ui-editor.md`.
- The implementer's own tests (`project_t057.rs`) and unit tests in `project.rs` are free. The
  odd arm's acceptance tests on the tests branch come first.
- Determinism: no clock and no randomness outside a test's seeded generator; the panel keeps its
  state in its own file.

### Collisions with other tasks
- **T-058 (tabs)**: no shared file. Both use `PanelView` and the current-document rule. T-057
  makes a document current only through `doc.open`; T-058 amends the rule (switching by tab). If
  T-058 merges first, AC-4's "current" is read through `Title` as T-058 leaves it. The spec check
  of whichever is second re-checks it.
- **T-059 (status)**, **T-061 (find)**: no shared file.
- **T-060 (outline, right of the editor)**: needs F1 too. Run T-060 after T-057, or put F1 into
  whichever is first (here). Not in flight together if T-060's `touches` take `shell.rs`.
- **T-062, T-063**: their stubs add a line to `shell.rs`, so they cannot be in flight with T-057.
- **T-066** (`observe.rs`, `ui.md`): no shared file. T-066 changes how rows of shaped and
  right-to-left text are reported. AC-3 (iii) already leaves those rows out. After T-066 a later
  task may tighten it.
- **T-068** (`script.rs`, `window.rs`, `ui.md`): no shared file. If T-068 makes requests that can
  do nothing refused, its spec decides whether a modifier or `at` on an entry is "nothing". This
  spec says they act as a plain click.
- **T-070 to T-078** (platforms): no shared file. ADR-0009 Decision 5 applies (`ci/T-057`).

## Questions and rulings
Questions for the director, before dispatch:
- **Q-1. A truncated listing.**
  - A: show what the read listed plus a `truncated` marker (as specified; measured E4: a large
    early directory hides later root entries).
  - B: read with `limit` 1,000,000. This is **unmeasured** (time and memory at 1,000,000 entries
    not probed), so it is not recommended without a probe.
  - C: file a follow-up adding a one-level listing to `project.tree` (a change of the verified
    `ws.open-project` command, medium, its own task).
  - **Recommendation: A now, and C as a draft.**
- **Q-2. Start collapsed or expanded.**
  - A: collapsed (as specified).
  - B: expanded at start. This reads before the first request, which breaks L7 and T-055 AC-1's
    log equality (merged tests would change) and adds entries to every merged test's snapshots.
  - **Recommendation: A.**
- **Q-3. Refreshing.**
  - A: re-read only on a user's expand (as specified).
  - B: re-read every expanded directory after every successful request. A keystroke would then
    cost a full read (E3: 0.5 s for 50,200 entries in debug), and the log would grow with reads.
  - C: file watching (a new mechanism, platform-dependent, nondeterministic for `kiln script`).
  - **Recommendation: A.**
- **Q-4. Where F1 is fixed.**
  - A: in this task (`shell.rs`, the helper's bounds only; measured necessary, E1, and sufficient
    for the merged tests, E2).
  - B: a separate task before T-057 and T-060.
  - **Recommendation: A** (one function, and this task is its first consumer).
- **Q-5. An entry's label and value.**
  - A: label = path, value = name (as specified; keeps F2, and the `Documents` values unique).
  - B: value = path. Then a merged target `{"value": <path>}` becomes ambiguous once a panel is
    expanded on that path, and F2 fails (painted name ≠ value).
  - **Recommendation: A.**

### Spec check R-262
Planner run R-262 (booked to `iter-2`), at main `15555cce3bfafdd8a579127ff49c3acd7f717ba6`. The
stub (R-225) was written before T-055, T-056 and T-065 existed. Every `depends_on` task has
merged, so this is the specification, written as a spec check would check it (METHODOLOGY v1.4
§5.7).

Changed against the stub:
- Tier medium confirmed by the v1.4 rule (claimed item `ui.project-panel` is low; items ⇒ ≥
  medium). Arm: odd, implementer-first. Budget 800,000 (v1.4).
- `depends_on` [T-056] → [] (merged). `touches` narrowed to named files, one role each, with
  `crates/kiln-ui/src/shell.rs` added for F1 (found by the probe, E1). Document `ui-project.md`
  (the stub's name kept).
- The stub's "directories expandable through `project.tree` with `path`" is kept; "how deep the
  panel lists by default" is answered: one level, collapsed at start (Q-2); limits are answered
  by the `truncated` marker (Q-1); "a click on an already open file" is answered by T-056's rule
  (`already_open` makes it current).
- New: the node vocabulary (toggle `Project`, `Project files`, `tree_item` with label = path,
  value = name, `toggled` for directories), the read policy, staleness (no watching), platforms,
  F1, F2, the generated-input criterion, and the collisions.

Evidence (all in a scratch copy of main under the session's scratchpad, `cargo` with
`CARGO_PROFILE_DEV_DEBUG=line-tables-only`, nothing built in the repository; the copy is deleted
after this run):
- E1: a throw-away prototype panel in `panels/project.rs` (a `Project` button with `toggled`, a
  `tree` of click-sensing labels set to role `tree_item`, `label` = path, `value` = name), with
  `cargo test -p kiln --locked`. `acceptance_t056.rs` `z1_rows_equal_the_text_each_node_shows`
  FAILED: `s1: a node without text has rows: {… "role":"scroll_view","rows":[[{"color":"#d7d7d7","text":"Project"}]]}`.
  Debug output: the `Project` button at x 8–65, while the editor's `ui.clip_rect()` was
  `[[0 0] - [1280 800]]` and its available rect started at x 196. The `scroll_view` bounds come
  from the content's clip rectangle, which spans the parent's width. The same failure occurred
  with the panel in a fixed 180-point region.
- E2: with `shell.rs`'s helper reporting the clip rectangle cut to the content's own horizontal
  extent (`max_rect` x range), `cargo test -p kiln --locked --no-fail-fast` passed every target
  (0, 26, 28, 7, 14, 2, 9, 7 tests: 93 passed, 0 failed), and `cargo test -p kiln-ui --locked`
  passed 21. Build of `kiln` and its tests: 45.96 s; target 1.5 GB.
- E3: the prototype through `kiln script --root proj --log l.jsonl` on (g)'s root: before any
  click, `Project` is `toggled: false` with rows `Project`. `click` `Project` settled in 4 frames
  and showed the seven root entries of (g), with `toggled` on the two directories. Expand `src`
  took 4 frames; open `src/a.rs` 5; `fifo` 4 (refused, no hang); `link.rs` 4 (opened). 600 idle
  frames added nothing. Labels and values were exact for `éx.txt` (combining mark) and `שלום.txt`.
  The log was as in (g). 50,200-entry root (200 directories of 250 files): control
  `project.tree {}` took 0.50 s wall and 107,844 KB, returning 50,000 entries, `truncated: true`;
  the expanding script took 0.67 s, `"frames":15,"settled":true`.
- E4: root `a/` with 50,000 files plus `z.txt`: `project.tree {}` returned 50,000 entries,
  `truncated: true`, last `a/f49998`, root entries `[a]` only.
- E5: the merged tests' targets (`grep` over `apps/kiln/tests/*.rs`): `"value":"f00.txt"`,
  `"value":"docs/document-000.txt"`, `"value_contains":"."` ×3, `"label_contains":"Pat"`,
  `"label_contains":"Ope"`; these constrain labels and values (F2, L5, Q-5).

Not checked: the refusal branch of AC-9 on this machine (the gate runs as uid 0 with the DAC capabilities, so a mode-000 directory is readable here and AC-9 takes its success branch; the refusal path is covered by the reviewer with the capabilities dropped, as the evaluator did in D-169); macOS and Windows (no CI job yet, T-070); a release build; the window (`kiln
--script` under Xvfb was not run); time at `limit` 1,000,000 (Q-1 B); whether T-058's spec
changes the current-document rule in a way AC-4 must follow (T-058 is still a draft).

### Rulings on spec R-262 (director, D-181)
- **Q-1: A.** A truncated read is shown as what it listed plus a `truncated` marker; a one-level listing in
  `project.tree` is filed as a draft (T-080). Reading with a larger `limit` is not taken: it was not measured.
- **Q-2: A.** The panel starts collapsed.
- **Q-3: A.** What is shown is re-read only when the user expands; no watching of the disk.
- **Q-4: A.** The fault F1 (a vertical scroll area reports bounds as wide as its parent) is fixed in this task, in
  `crates/kiln-ui/src/shell.rs`, with a test that fails before the fix.
- **Q-5: A.** An entry's `label` is its path and its `value` is its name.
- **Q-6 (director).** The test files of this task use no Unix-only API (permission modes, links made through
  `std::os::unix`): the rule for platform-specific tests (T-071) is not merged, and these files will be compiled
  on macOS and Windows once T-070 has merged. AC-9's refusal branch (a directory that cannot be read) is therefore
  not an acceptance test in this task: the reviewer shows it once as a recorded procedure with the DAC
  capabilities dropped, and the test is added by the task that marks platform tests (T-079). Fixtures that need a
  link or a FIFO are made only where the test can do so without a Unix-only API; otherwise they go the same way.
- Arm: the task number is odd, so the implementer writes the acceptance tests first on `task/T-057-tests`,
  committed before any code, and runs the whole gate on a local merge of that branch before reporting (§6).

### Rulings on the implementer's questions of R-265 (director, D-184)
- **v1 (question 1).** The `truncated` marker of a sub-directory comes after everything shown below that
  directory, as built. The reviewer checks the sub-directory case, which no acceptance test covers.
- **v2 (question 2).** The FIFO and link cases of AC-5 and the `symlink` and `other` kinds of AC-1 and AC-3 go to
  T-079 with AC-9: they need fixtures a test cannot make without a Unix-only API (Q-6). The reviewer shows them
  once as a recorded procedure.
- **v3 (question 3).** A successful expand leaves `Message` as it was; only a successful open or save clears it.
  `docs/protocol/ui-project.md` says so in one sentence (the implementer adds it in the rework after review).
- The sentence "a file the UI's `doc.save` creates appears after the next expand" is unverified: no script can make
  a save create a file. It is removed from the document unless the reviewer finds a way to show it.

### Rulings on review round 1 (director, D-188; `reviews/T-057-r1.md` at `27d9875c41bf0967f44a655aca299d5e10d6cca7`)
- **r1 (finding 1, test gap).** An acceptance test for ruling v1 is added to `acceptance_t057.rs` on the tests
  branch: with a truncated listing inside an expanded sub-directory, that directory's `truncated` marker comes
  after everything shown below it, and a nested truncated directory's marker comes before its parent's. The
  fixture needs no Unix-only API (the reviewer used 50,001 empty files, as AC-8 does).
- **r2 (finding 2).** P5 in `docs/protocol/ui-project.md` says where nested markers go: each after the last entry
  shown below its directory, the innermost first.
- **r3 (finding 3).** The sentence of v3 is added: a successful expand leaves `Message` as it was.
- **r4 (finding 4).** The sentence on the cost of a large directory states what was measured, as a limit: every
  shown entry is laid out on every frame, so the time of a frame grows with the number of entries shown; the
  reviewer measured, in a debug build with 50,000 entries shown, 18.0 s for the expand, 0.76 s for each later
  frame and 475.9 s for 600 idle frames. Laying out only the rows in view changes what a snapshot reports for the
  rows out of view and is not done in this task; it is filed as draft T-081.
- **r5 (finding 5).** "shows its first 37 chars" is replaced by what holds: a name wider than the panel is cut at
  the panel's edge, and how many chars remain depends on the chars.
- **r6 (finding 6, spec gap).** A file name that contains a line break is painted on two rows, so "one row equal
  to the value" does not hold for it; its label and value are exact and a click opens it. Stated as a limit.
- **r7 (findings 7 and 8).** The long comment line in `shell.rs` is wrapped. The two tests that depend on the
  layout (AC-2 needs a scroll at 800x320; AC-3 clicks every entry at 1280x8192 without scrolling) stay as they are
  and are named in T-058's and T-059's notes: a task that changes the shell's layout re-checks them.
- The sentence "a file the UI's `doc.save` creates appears after the next expand" stays: the reviewer showed it.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-14 · R-225 · (new) · did: filed as a draft for ADR-0007. · gate attempts: 0 · missing context: T-056's merged shell · next: director.
- 2026-11-15 · R-262 · draft · did: specified the panel against main `15555cc` (see "Spec check R-262"): tier medium (v1.4 §4: item low, items ⇒ medium; no high criterion), odd arm; `touches` five named files, each one role; found and specified F1 (a merged fault in the `scroll_view` bounds that any side panel exposes) with a probe build in scratch; 10 criteria, AC-3 on generated trees from a stated alphabet; Pinned (i) P1–P7 with evidence; five questions. · gate attempts: 0 (scratch builds only, outside the repository) · not run: macOS, Windows, release build, the real window · missing context: T-058's final current-document rule · next: director (Q-1 to Q-5).
<!-- end planner log -->
### implementer
- 2026-11-15 · R-265 · ready · did: odd arm, implementer-first. Rulings acknowledged before acting: Q-1 A, Q-2 A, Q-3 A, Q-4 A, Q-5 A accepted as written; Q-6 accepted: no Unix-only API in either test file, so AC-9 has no test, and the FIFO, link-to-a-directory and link-outside cases of AC-5 (and the `symlink`/`other` kinds in AC-1/AC-3 fixtures) have none either (no portable way to make them). Criteria in my words: AC-1 one click on `Project` shows the root's entries of `project.tree {}`, dirs `toggled:false`, value = name; AC-2 random expand/collapse equals the naive preorder model, one read per expand, none per collapse; AC-3 40 generated trees fully expanded equal `project.tree`, git, exact labels/values, rows except RTL-first names, 5 opens; AC-4 a file click opens and makes current, re-click switches without a new doc, replay agrees; AC-5 a refused open changes nothing, `Message` begins with the oracle's kind; AC-6 control-only log = `Host`, idle frames add nothing, collapse/expand logs two root reads; AC-7 each expand logs a fresh read and shows it; AC-8 truncated root: `a` then one marker, no `z.txt`; AC-9 (not a test, Q-6); AC-10 panel text belongs to panel nodes, F2, editor rows from line 0.
  (1) Tests first on `task/T-057-tests`: `apps/kiln/tests/acceptance_t057.rs`, commit `2ec56826e4f57d2f66941d3c60634ceaacd84e04` (2026-11-15T18:31:02Z), before the first implementation commit `47236156528b2814daef711aaa3066b53e6a6854` (18:34:42Z). On base all 11 fail at the first use of the toggle (`no_target` for `{"role":"button","label":"Project"}`, or "found 0" for the snapshot checks of AC-1/AC-6a): assertion, not compile. Tests by criterion (scorecard `ui.project-panel`: AC-1 and AC-4 are the item's): AC-1 `ac1_click_on_project_lists_the_roots_entries`; AC-2 `ac2_expand_and_collapse_follow_the_naive_model`; AC-3 `ac3_generated_trees_fully_expanded`; AC-4 `ac4_click_on_a_file_opens_it_and_makes_it_current`; AC-5 `ac5_a_refused_open_changes_nothing_and_shows_its_kind` (decode_error only); AC-6 `ac6a_control_requests_only_log_as_host`, `ac6b_idle_frames_after_the_panel_add_nothing`, `ac6c_collapse_sends_nothing_expand_reads_again`; AC-7 `ac7_each_expand_reads_afresh_and_shows_that_read`; AC-8 `ac8_a_truncated_root_listing_ends_with_a_marker`; AC-10 `ac10_panel_text_belongs_to_panel_nodes_and_rows_equal_text`.
  (2) Change to the acceptance tests after code (commit `fdd0d97ba6d0b5f11bd34658d170a3ad85ea63a6`, 18:36:23Z, its message says what was wrong): AC-3's generator could list fewer than the 5 files the criterion clicks (seed 0x570317 listed 4), so it now makes five root files without `.` that no pattern names; AC-2 at 800x480 needed no scroll, so its "scroll first" precondition failed, now 800x320. No assertion about the panel changed.
  (3) Code: `panels/project.rs` (state in the toolkit's per-id memory; reads only in the frame that handles a click; `doc.open` through `PanelView::request`; refusals to `Message`; directories' arrows drawn as shapes; width min(240, a quarter of the space), 120 at least). F1 in `shell.rs`: the `scroll_view` bounds are the clip rectangle cut to the area's own horizontal extent (`max_rect` x range). Own test `f1_text_beside_the_editor_belongs_to_the_panel` failed before the fix (`1280x800 collapsed: the toggle's row`, left `[]`: the row went to the editor's scroll_view) and passes after; with the panel and before the fix the merged `z1_rows_equal_the_text_each_node_shows` failed too (E1 reproduced). Own tests `apps/kiln/tests/project_t057.rs` (5) and two unit tests in `project.rs`.
  (4) Checked against the built binary (scratch, deleted): Pinned (g) with a real FIFO and link: entries, `toggled`, `fifo` → `Message` `not_a_file…` in 4 frames, `link.rs` opened, 600 idle frames, the log exactly as (g). AC-9's refusal branch by hand with `setpriv --bounding-set=-dac_override,-dac_read_search --inh-caps=-dac_override,-dac_read_search` on a root holding a mode-000 dir: toggle stays `toggled:false`, no `tree_item`, `Message` begins `io_error`, log empty, the control `project.tree` `io_error`. The document's example was run as written. Sentence I added beyond Pinned (i) (limit): "A name wider than the panel is cut …": evidence `kiln script` with a root file of 84 chars at 1280x800 → its row holds the first 37 chars, label/value exact. Not checked: "A file that the UI's own `doc.save` creates appears after the next expand" (copied from Interface; no script can make a save create a file), the window (`kiln --script`), macOS, Windows.
  (5) Self-check, mutants of `project.rs`/`shell.rs` against the acceptance tests on the scratch join: explicit `limit` in the root read (killed by ac2, ac6b, ac6c, ac7); collapse keeps descendants (ac2); value = path (ac2, ac3, ac10); no `toggled` on dirs (ac1, ac2, ac3, ac7, ac8); refused open leaves `Message` (ac5); successful open keeps `Message` (ac4); no marker (ac8); entries reversed (ac1, ac2, ac3, ac6c, ac7, ac8); F1 reverted (ac10); root collapse rewritten (equivalent, survived). Two more mutants (a read every frame, a directory that opens) did not build: the disk filled (linker bus error), see below.
  gate attempts: 3 (last: green). (a) Scratch merge `scratch/T-057-join` (task/T-057 + task/T-057-tests, local, never pushed, deleted): attempt 1 red at `fmt` (my `shell.rs` line), fixed on `task/T-057` by rustfmt; attempt 2 green, 560 s, `test` 997 passed, `no-skip` 186 files, 0 skips. (b) `task/T-057` without the acceptance file: attempt 3 green, 392 s, 986 passed. The gate waited about 10 min: free disk fell to 0.5–0.8 GB while other builders ran (a mutant's link died with a bus error then; I deleted my `target/debug/incremental`); it ran once 23 GB were free.
  QUESTION: P5 for a sub-directory whose last child is itself expanded: I put the marker after everything shown below the directory (after its last descendant), reading "its last shown entry" that way. Only the root case is tested.
  QUESTION: Q-6 also removes AC-5's FIFO and link cases from the gate (only `decode_error` is tested); should they go to T-079 with AC-9's refusal branch?
  QUESTION: after a refused read, a later successful expand leaves `Message` as it is (the spec says nothing about clearing it on a successful read). Keep, or clear it as a successful open does?
  Note: both test files start with `#![cfg(test)]` as `acceptance_t056.rs` does (no-skip allows exactly `cfg(test)`); no other `cfg`.
  · missing context: none · next: reviewer (mutation spot-check; AC-7's recorded disk-change procedure; AC-9's refusal with capabilities dropped; the window).
- 2026-11-15 · R-272 · rework after review round 1 · did: rulings acknowledged before acting: v1 (marker after everything shown below its directory, as built), v2 (FIFO/link cases to T-079), v3 (a successful expand leaves `Message` as it was) of D-184; r1–r7 of D-188 accepted as written; the `doc.save` sentence stays. r1: `ac8b_nested_truncated_listings_put_each_marker_after_its_directory` on `task/T-057-tests` (commit `ff73a475580d5b87acc13805ae50af4721dd6435`): root `a/b/c/` with 50,001 empty files plus `a/z.txt`, `z.txt`, so the reads of the root, `a` and `a/b` are all truncated (asserted) while each directory shows one child; shown order after expanding root and `a`: `a`, `a/b`, marker, marker; after `a/b` too: `a`, `a/b`, `a/b/c`, marker ×3; after collapsing `a/b`: `a`, `a/b`, marker ×2; cleans up. It differs from the reviewer's fixture (`a/b/` of 50,001 files) so that only 3 entries are shown (3.9 s instead of tens of seconds per run, see r4) while `a`'s last shown entry is still an expanded directory. Mutants on the scratch join, against all 12 acceptance tests: M10 (marker only when `owner.is_none()`) killed by ac8b alone; M18 (a sub-directory's marker drawn right after its last entry, before that entry's descendants) killed by ac8b alone; the other 11 pass under both. r2: P5 says the marker comes after the last entry shown below its directory, nested markers innermost first, identical, told apart by position. r3: v3's sentence added after the refused-read paragraph. r4: the Limits item states the per-frame layout cost with the reviewer's measurement (18.0 s, 0.76 s/frame, 475.9 s for 600 idle frames, debug, cited to `reviews/T-057-r1.md` finding 4); panel unchanged (T-081). r5: the 84→37 figure is replaced by "how many chars remain depends on the chars". r6: checked against the built binary (`kiln script`, root file `new\nline`): rows `["new"], ["line"]`, label and value `new\nline`, a click opens it (`Title` `new\nline — Kiln`); stated as a limit. r7: `shell.rs` doc comment wrapped to 100 cols; AC-2/AC-3 untouched. · gate attempts: 1 (last: green) on the scratch join of `task/T-057` + `task/T-057-tests` (local, deleted): 458 s, `test` 998 passed, 0 failed; `no-skip` 186 files, 0 skips. · not run: gate on `task/T-057` alone (only docs and a comment changed there since its last green gate); release build; the window. · missing context: none · next: reviewer round 2.
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
- 2026-11-15 · R-269 · tests_joined (head 4d6e464) · round 1 · did: rulings acknowledged from main's copy and accepted: Q-1 A to Q-5 A, Q-6 (D-181); v1, v2, v3 and the `doc.save` sentence (D-184). Full review `reviews/T-057-r1.md`, verdict `request-changes`. Tests-first order confirmed (`2ec5682` before `4723615`); `fdd0d97` changes fixtures and preconditions only, no panel assertion weaker. Differential run: 40 generated trees (seeds 0x5EED7000–0x5EED7018 at 900x500 with scrolling, 0x5EED8000–0x5EED800E at 1280x8192), 3,078 entries, 791 directories expanded by clicks, vs `project.tree`, the disk, `git ls-files`, replay: 0 mismatches; 12 random sessions (0x5EED9100–0x5EED910B, 480 steps) vs a naive model: 0 mismatches. Procedures: mode-000 refusal with DAC capabilities dropped, a refused sub-directory read (4,101-byte host path), FIFO/links/device, v1 nested markers, AC-7 disk change within one session, and the `doc.save`-creates sentence shown (it can stay). Window: R-251 procedure all PASS, my window script byte-equal to `kiln script`. F1 at ten sizes. 19 mutants: 12 killed by acceptance tests, 2 by own gate tests, 4 only by procedures, 1 equivalent, 0 survived. Findings: test gap (v1 sub-directory marker not in the gate, M10/M18); four document sentences (P5 placement, v3 sentence missing, the large-directory limit leaves out a per-frame cost of 0.76 s at 50,000 shown entries, the 84→37 chars figure); spec gap (a name with a line break paints two rows, F2); two nits. · gate attempts: 0 of `cargo xtask ci` (ran `cargo test -p kiln -p kiln-ui`, `ui-commands`, `layers`, `no-skip`, `schema`, `scorecard --check`, `hygiene`) · not run: release build, macOS, Windows, a real window manager · missing context: none · next: implementer (findings 1–5, 7, 8), director (finding 6)
<!-- end reviewer log -->
