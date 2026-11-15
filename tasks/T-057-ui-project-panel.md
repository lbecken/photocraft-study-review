---
id: T-057
title: "Show the project tree in a panel beside the editor: expand and collapse directories, and open a file as the current document by clicking it"
tier: medium            # ADR-0002 default (UI behaviour over existing commands; no dependency, gate path, persistence or process code); ≥ the claimed item's tier (`ui.project-panel`: low); fixed at specified (METHODOLOGY v1.4 §4)
state: in_progress            # director only, written with every transition; see METHODOLOGY §3
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
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
