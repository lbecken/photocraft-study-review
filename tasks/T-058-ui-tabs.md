---
id: T-058
title: "Show every open document as a tab; make a document current by clicking its tab or its Documents entry; close it from its tab, discarding unsaved changes only on a second, explicit click"
tier: medium            # ADR-0002 default (UI behaviour over existing commands; no dependency, gate path, persistence or process code); = the claimed item's tier (`ui.tabs`: medium); fixed at specified (METHODOLOGY v1.4 §4)
state: tests_joined            # director only, written with every transition; see METHODOLOGY §3
owner_role: planner
scorecard_items: [ui.tabs]
depends_on: []          # T-056, T-057, T-065 merged; nothing unmerged
touches:                # one role per path (D-153); no gate path
  - crates/kiln-ui/src/panels/tabs.rs        # implementer: the panel (created empty by T-056)
  - crates/kiln-ui/src/panels/mod.rs         # implementer: one method, `PanelView::changes` (Interface); nothing else
  - crates/kiln-ui/src/shell.rs              # implementer: `Documents` entries made clickable (Q-1 A); nothing else
  - apps/kiln/tests/acceptance_t058.rs       # test author (even task number: independent)
  - apps/kiln/tests/tabs_t058.rs             # implementer: its own tests through the binary
  - docs/protocol/ui-tabs.md                 # implementer: new; the sentences of Pinned (i)
  - docs/protocol/ui-editor.md               # implementer: the section "The current document" only
  - docs/protocol/ui.md                      # implementer: the one sentence on when `Message` begins with a kind (ruling h5, D-201); nothing else
budget_tokens: 800000   # medium (v1.4)
iteration: 2
found_by: planner       # ADR-0007 row 8 (run R-225); specified in run R-276; the Documents note: evaluator (D-169)
origin_task: null
---

# T-058 Tabs

## Goal
After this task the UI shows one tab per open document above the editor. A click on a tab makes
its document current: the editor, `Title` and (after T-059) the status bar show it. A click on
the tab's close button closes the document with `doc.close`. A document with unsaved changes is
not closed by that click: it is refused (`unsaved_changes`), and a `Discard` button appears that
closes it with `discard: true` when clicked. Entries of the shell's `Documents` list become the
same control as tabs: a click makes that document current. Today a click there does nothing
(`evaluations/iter-2-cut-1.md`, "Wired but inert"; D-169). The current-document rule of T-056
gains one clause (switching), and none of its other clauses change.

Claims `ui.tabs` ("Open documents appear as tabs; switching and closing tabs dispatch commands").
Tier medium, even number, so an **independent test author** writes the acceptance tests from this
spec. No gate path. `apps/` is a platform-sensitive path (ADR-0009 Decision 5): `ci/T-058` before
the queue once the platform jobs count. The tests use no Unix-only API (Platforms).

Reading list: `docs/adr/0008-ui-observation.md` ("What counts as observed"); `docs/protocol/ui.md`
(targets, `scroll`, snapshot rules, Limits); `docs/protocol/ui-editor.md` ("The current document",
"Frames, reads and the log"); `docs/protocol/ui-project.md` (a panel's conventions);
`crates/kiln-ui/src/panels/mod.rs`, `panels/project.rs` (idioms), `shell.rs`, `session.rs`; this
file's Pinned (f).

## Interface

No new registry command, UI request kind, refusal kind or command-line option.

### What the UI shows (nodes in a `kiln script` snapshot)
- **The tab list**, present exactly when at least one document is open: a node of role
  `tab_list` labelled `Tabs`, inside a node of role `scroll_view` that contains no shell node
  (`Path`, `Open`, `Message`, `Documents`), no `Title`, no `Editor` and no project-panel node.
- **One tab per open document**, in id order (the order of `doc.list`), each a node of role
  `tab`:
  - `label`: the document's path exactly as `doc.list` reports it;
  - no `value`;
  - its painted text is its path (one row; see Limits for a path with a line break);
  - `toggled: true` for the current document's tab, `toggled: false` for every other. The toolkit
    reports a tab's selected state as `toggled` in the snapshot (measured, E3).
- **With each tab**, after it in the snapshot's depth-first order and before the next tab:
  - when `doc.list` reports the document `dirty`: one node of role `label` with `value` `modified`,
    painting exactly `modified`;
  - always: a node of role `button` labelled `Close <path>`, painting no text (its mark is drawn
    as a shape, so F2 holds);
  - only after a close of that document was refused with `unsaved_changes`, and until the
    document is closed or `doc.list` reports it clean: a node of role `button` labelled
    `Discard <path>`, painting exactly its label.
- How tabs wrap, their size and the height of the strip are not specified. The strip is at most a
  few rows tall and scrolls (`scroll` on `Tabs` or on any tab), so with many documents some tabs
  are `offscreen`. Tests scroll before clicking an `offscreen` tab and assume no position.
- `Documents` (the shell's list) keeps its nodes exactly (role `list`, entries of role `label`
  with the path as `value`). An entry is now clickable (Q-1 A).

### UI requests that act on it
- `click` on a `tab`: its document becomes current (no mutating request; Requests below). A click
  on the current tab changes nothing.
- `click` on a `Documents` entry (`{"role":"label","value":<path>}` inside `Documents`): the same
  as a click on that document's tab.
- `click` on `Close <path>`: sends `doc.close {"doc": <id>}`.
  - Success: the tab is gone, `Message` is cleared, and if the document was current, the current
    document is the open one with the highest id (unchanged rule).
  - Refusal `unsaved_changes`: nothing closes, `Message` begins with `unsaved_changes`, and the
    `Discard <path>` button appears.
  - Any other refusal: `Message` begins with its kind.
- `click` on `Discard <path>`: sends `doc.close {"doc": <id>, "discard": true}`, with the same
  success rule. There is no dialog and no confirmation beyond this second, explicit click.
- `scroll` on `Tabs` or a tab scrolls the strip. A modifier, a secondary click or `at` on a tab
  acts as a plain click.
- No tab, close or discard button takes keyboard focus by itself (L4).

### The current document after this task (`docs/protocol/ui-editor.md`, rewritten section)
1. A successful `doc.open` from any source (a start path, `Path`/`Open`, the project panel, a
   script's control request), `already_open: true` included, makes that document current
   (unchanged).
2. **New:** a click on a document's tab, or on its `Documents` entry, makes it current.
3. When the current document is closed (from its tab or by any `doc.close`), the open document
   with the highest id becomes current (unchanged).
4. With no document open there is no current document, no `Title`, no `Editor` and no `Tabs`
   (unchanged, plus `Tabs`).
Merged tests that pin rules 1, 3 and 4 keep passing unchanged, so none is edited and none is in
`touches`:
- `apps/kiln/tests/editor_t056.rs` `the_current_document_follows_opens_and_closes` (rules 1, 3, 4);
- `apps/kiln/tests/acceptance_t056.rs` `ac1_current_document_rule` and
  `ac1_start_paths_in_order_last_success_is_current` (rule 1);
- `apps/kiln/tests/acceptance_t057.rs` `ac4_click_on_a_file_opens_it_and_makes_it_current` (rule 1).
Evidence: all 110 tests of `apps/kiln` passed with a prototype of this task (E2).

### Control requests the panel sends, and when
- `doc.close` (and `doc.close` with `discard: true`): on those clicks only, in the frame that
  handles the click.
- **No mutating request on a switch.** A switch is made through the session's current document
  (`PanelView::set_current`). The views that show the current document then re-read it with
  read-only requests. Those reads are logged: the editor's `doc.info`, `doc.text`, `cursor.get`,
  `syntax.highlight`, and this panel's `doc.list`. That is how "switching dispatches commands" is
  observed (in the log, all `mutating: false`).
- **Reads**: a read-only `doc.list` in the first frame after a **change** (the prototype sometimes
  read twice for one close, E3; the count is not pinned), and only when a document is open. A change is a successful request of any source other than a read, or a change
  of the current document; it is exactly what `Session::changes` counts. This is the editor's rule
  (`docs/protocol/ui-editor.md`, E5).
- **Nothing** on an idle frame (no change since the last read), on a scroll, or with no document
  open. A script of control requests only runs no frame after its first request, so it writes
  `Host::log_script()` (L7).

### Many documents, long names, special names
- Every open document has a tab; none is hidden or merged. With 300 documents at the default size
  the strip showed 62 tabs and 238 `offscreen`. A click on an `offscreen` tab is `not_actionable`,
  and the tab can be clicked after scrolling. Actions settled within 14 frames, and 600 idle frames
  took 5.9 s with the session's 300 opens (debug build, E4). Every tab is laid out on every frame,
  like the project panel's entries (T-057 r4); the per-frame cost grows with the open documents.
  That is a stated limit (Pinned (i) T6), not a bound.
- A long path is painted whole on one row: tabs do not shorten paths. Whether it is cut at the
  window's edge depends on the chars (`ui-project.md`, as r5).
- A path containing a line break (allowed on Linux) is painted on two rows, so the tab has two
  rows. Its `label` is exact and a click works (E3). Stated as a limit, as T-057 r6.
- A path that begins with right-to-left text: its `label` is exact; its rows fall under
  `docs/protocol/ui.md`'s Limits (T-066).

### Platforms (ADR-0009)
No platform difference: tabs show `doc.list`'s paths, with `/` on every platform. The test files
use no Unix-only API (no FIFO, no symbolic link, no mode bit, no name a platform forbids;
generated names follow T-057 AC-3's alphabet without a line break). The line-break case is
checked by the reviewer as a recorded procedure on Linux. Evidence for macOS and Windows comes
from CI once T-070 has merged.

### Rust items this builds on (main `c288f7f`; not used by tests)
```rust
// panels/tabs.rs — signature kept
pub(crate) fn show(ui: &mut egui::Ui, view: &mut PanelView<'_>);
// panels/mod.rs — existing, unchanged
impl<'a> PanelView<'a> {
    pub(crate) fn request(&mut self, command: &str, params: serde_json::Value) -> Result<serde_json::Value, (String, String)>;
    pub(crate) fn read(&mut self, command: &str, params: serde_json::Value) -> Result<serde_json::Value, (String, String)>;
    pub(crate) fn current(&self) -> Option<u64>;
    pub(crate) fn set_current(&mut self, doc: u64);       // counts as a change (Session::set_current)
    pub(crate) fn open_documents(&self) -> &[(u64, String)];
    pub(crate) fn set_message(&mut self, text: String);
}
// panels/mod.rs — added by this task (T-059 uses it)
impl<'a> PanelView<'a> {
    /// The session's change counter (`Session::changes`): it changes with every successful
    /// request other than a read, and with every change of the current document.
    pub(crate) fn changes(&self) -> u64;
}
// session.rs — existing, used by the shell for a `Documents` click; not edited
impl Session { pub(crate) fn set_current(&mut self, doc: u64); pub(crate) fn changes(&self) -> u64; }
```
### Rust API used by tests
None. All criteria are tested through the built `kiln` binary. `kiln_control::Host` is the only
Rust API a test may use, as an oracle, with T-055's pinned signatures. `apps/kiln/Cargo.toml` is
not edited.

## Pinned
- (a) Errors: refusals of `doc.close` as published: `no_such_document` (it cannot be reached from
  a tab, since a tab exists only for an open document) and `unsaved_changes`. A refusal changes
  nothing else and sets `Message` to begin with its kind. A successful close from the UI clears
  `Message`, as a successful UI open or save does. A refused read is not shown. Exit codes
  unchanged.
- (b) Files: none read or written by the panel; closing goes through `doc.close`. Source: the panel
  is in `tabs.rs`; the other edits are one method in `mod.rs` and the `Documents` click in
  `shell.rs`.
- (c) Order: in the frame that handles a click, the click's request (or switch) happens; views show
  its effect from that frame or the next, within the action's settling. In the frame after a change
  the editor reads first (it runs before the layout) and the panels read during the layout.
- (d) Paths as `doc.list` reports them; Linux evidence until T-070 (Platforms).
- (e) Threat model: n/a beyond the commands' own (no file access of the panel's own; `doc.close`
  never touches disk). A hostile number of documents costs layout per frame (T6).
- (f) Facts the merged tests rely on, which this task keeps:
  - **F1–F2** (T-057): `scroll_view` bounds are the area's visible part; every node with `rows`
    other than `Editor` has exactly one row, equal to its `value`, else its `label`. That is why a
    tab has a label and no value and paints its path, why `modified` is a separate node with that
    value, and why the close mark is a shape.
  - **No value collision**: merged tests target `{"value":"f00.txt"}`, `{"value":"docs/document-000.txt"}`
    (`Documents` entries) and expect `{"value_contains":"."}` to be ambiguous. Tabs carry no
    value, so none changes.
  - **L1–L8** (T-056 Pinned (f)): L4 nothing focused by itself; L5 no new label `Path`, `Open`,
    `Documents`, `Message` and none containing `Pat` or `Ope` in the T-055 sessions (tab labels are
    their paths, `Close …`, `Discard …`: none of the T-055 sessions opens such a path, E2); L7;
    L8 settles within 60 frames (measured up to 14, E4).
  - **T-057's layout-dependent tests** (D-188): `ac2_expand_and_collapse_follow_the_naive_model`
    (some click needs a scroll at `--size 800x320`) and `ac3_generated_trees_fully_expanded`
    (every entry clicked without scrolling at `1280x8192`) passed with the tab strip and a status
    bar both present (E2). `acceptance_t057.rs` is therefore not in `touches`. If the
    implementation's strip is taller than the prototype's and either test fails, that is a
    `QUESTION:` (stop condition 2), not an edit.
  Not to be pinned (golden rule 9): a whole snapshot, the strip's height or wrapping, positions,
  message text beyond its leading kind, the text of `Close`/`Discard` beyond the labels above,
  which reads the editor sends.
- (g) Worked examples, executed with `kiln script` on a scratch build of main `c288f7f` plus the
  throw-away prototype (E3); the control responses are main's. Root with `src/a.rs`,
  `b.txt` (CRLF), `mixed.txt` (`x\ny\r\n`), `bom.txt` (BOM), `new\nline.txt`, `שלום.txt`; six
  control `doc.open`s, then `frames 1`:
  - the tabs, in order: labels `src/a.rs`, `b.txt`, `mixed.txt`, `bom.txt`, `new\nline.txt`
    (rows `["new","line.txt"]`), `שלום.txt` (`toggled: true`: the last opened);
  - `click {"role":"tab","label":"src/a.rs"}` (4 frames): its tab `toggled: true`, `Title`
    `src/a.rs — Kiln`; the log gained `doc.info`, `doc.text`, `cursor.get`, `syntax.highlight`
    of doc 1 and `doc.list`, all read-only;
  - after a click into `Editor` and `type "z"`: `modified` follows `src/a.rs`'s tab;
  - `click {"label":"Close src/a.rs"}`: `Message`
    `unsaved_changes: document 1 has unsaved changes; save it or close with discard`, the
    `Discard src/a.rs` button shown, `doc.list` unchanged;
  - `click {"label":"Discard src/a.rs"}`: the log has
    `{"command":"doc.close","params":{"discard":true,"doc":1}}`, and `שלום.txt` (id 6, the highest)
    is current;
  - 300 idle frames: nothing logged.
- (h) Evaluator calls for `ui.tabs` (ADR-0008 (O1)–(O5)): `kiln script --root d` with documents
  opened by `Open`, the project panel and control requests; tab clicks, `Documents` clicks, close
  and discard clicks; snapshots before and after; checked against `doc.list`, `Title`, `Editor`
  value against `doc.text`, and `--log` (each switch shows read-only requests only, each close a
  `doc.close`). Inert check (O5): every tab, close and discard button and every `Documents` entry
  changes something. Where a display exists, the same through `kiln --script`.
- (i) Sentences for `docs/protocol/ui-tabs.md` (and the rewritten "The current document" of
  `ui-editor.md`), worded for copying, with their evidence:
  - T1 "Every open document has a tab, in id order, labelled with its path; the current
    document's tab is reported `toggled: true`." Evidence: E3; AC-1.
  - T2 "A click on a tab, or on a document's entry in `Documents`, makes that document current. It
    sends no request that changes anything; the views then re-read the document with read-only
    requests, which are in the session log." Evidence: E3 log; AC-2, AC-6.
  - T3 "A tab's close button closes its document with `doc.close`. A document with unsaved changes
    is not closed: `Message` begins with `unsaved_changes` and a `Discard` button appears, which
    closes it with `discard: true`. It stays until the document is closed or saved." Evidence: E3;
    AC-3, AC-4.
  - T4 "A node with the value `modified` follows the tab of every document `doc.list` reports as
    dirty." Evidence: AC-5.
  - T5 "The tabs read `doc.list` in the frames after a change and never on an idle frame." Evidence: E3 log; AC-7.
  - T6 (limit) "Every tab is laid out on every frame, so the time of a frame grows with the number
    of open documents: with 300 open documents, a script that opens them and runs 600 idle frames
    took 5.9 s in a debug build on a 4-core Linux machine." Evidence: E4.
  - T7 (limit) "A path that contains a line break is painted on two rows; its label is exact and a
    click on it works." Evidence: E3.
  - The rewritten current-document rule (1–4 above). Evidence: AC-1, AC-2, AC-3 and the four merged
    tests named above.

## Acceptance criteria
All tests drive the built `kiln` binary (`kiln script`) with temporary roots, processes under a
timeout. Nodes are found by role and label (or value), never by a whole snapshot or a position.
Every AC fails on base, where no node of role `tab` exists (`no_target`, or an assertion on the
absent `Tabs` node), except AC-6, which fails because a `Documents` click leaves `Title` unchanged
on base.
- AC-1 (`ui.tabs`, tabs equal the documents): after each step of random sequences of opens
  (control `doc.open`, `Path`/`Open`, project-panel clicks) and closes (control `doc.close`), the
  tab labels below `Tabs` equal `doc.list`'s paths in order. Exactly one tab is `toggled: true`,
  and its label is the path `Title` begins with. No `Tabs` exists with nothing open. Oracle:
  `doc.list` and `Title` in the same session.
- AC-2 (`ui.tabs`, switching dispatches commands): a click on a tab makes `Title` begin with its
  path and `Editor`'s value equal that document's `doc.text`. `Editor`'s `selection` equals that
  document's `cursor.get` (each document keeps its selection). The log entries added by the
  switch are all of commands `mutating: false` in `docs/commands.json`, and at least one names the
  new document. A click on the current tab adds nothing to the log. Oracle: registry state; the
  published `mutating` flags.
- AC-3 (`ui.tabs`, closing): a click on `Close <path>` of a clean document removes it from
  `doc.list` and its tab. The log gains `doc.close {"doc": <id>}`. Closing the current document
  makes the highest open id current; closing another leaves the current one. `Message` is empty
  afterwards. Oracle: `doc.list`, `Title`, log.
- AC-4 (unsaved changes): after an edit, a click on `Close <path>` leaves the document in
  `doc.list` and dirty; `Message` begins with `unsaved_changes`; `Discard <path>` exists; the log
  has no `doc.close` for it. A click on `Discard <path>` closes it, and the log gains
  `doc.close` with `discard: true`. In a second session, after the refused close, `ctrl+S` makes
  `Discard <path>` disappear and a later `Close <path>` closes it without discarding (the file on
  disk holds the edit). Oracle: `doc.list`, log, file bytes.
- AC-5 (dirty marks): after random edits, undos and saves from the UI and edits by control
  requests to non-current documents, a `modified` node follows exactly the tabs whose documents
  `doc.list` reports dirty. Oracle: `doc.list`.
- AC-6 (`Documents` entries): a click on a `Documents` entry makes its document current (`Title`,
  `Editor`) and is logged like a tab switch (read-only requests only). Oracle: as AC-2.
- AC-7 (idle and reads): (a) a script of control requests only writes `Host::log_script()`;
  (b) 600 frames after tabs exist add nothing to the log; (c) after a successful control request
  that changes a document's dirty state (e.g. `doc.insert` on a non-current document), the frames
  until the UI settles add a `doc.list` read and the tab's `modified` node appears, and 600
  further frames add nothing. How many reads a change causes is not pinned: the prototype logged
  two `doc.list` reads after one discarding close (E3). Oracle: log comparison; `doc.list`.
- AC-8 (generated sessions, D-174 change 5): for at least 40 seeds, a random root (names from
  T-057 AC-3's alphabet: ASCII, inner spaces, `é` and `e`+U+0301, a joiner, `ffi`, `漢字`, `😀`,
  `שלום`, `سلام`, `٣`; no line break, no case-only or normalization collisions, nothing a platform
  forbids) and 30 random steps (opens, tab clicks, `Documents` clicks, closes, discards, typed
  text, saves, scrolls of `Tabs`). After every step the snapshot equals a naive model of the
  current-document rule (1–4) and of T3 and T4 (tab labels, `toggled`, `modified`, `Discard`).
  The final `--log` replayed with `kiln_control::Host` on a copy of the root gives the same
  `doc.list` and files. Labels are compared exactly; rows only for names beginning with neither
  right-to-left text nor `٣` (`ui.md` Limits). Seed and counts printed on failure. Oracle: naive
  model, `doc.list`, replay.
- AC-9 (many documents): with 300 open documents at the default size, every tab can be reached by
  scrolling `Tabs` until it is not `offscreen`, and its click makes it current (`Title`). A click
  on an `offscreen` tab is `not_actionable`. Every action settles within 60 frames. No wall time is
  asserted. Oracle: `Title`, `settled`.

## Out of scope and implementer notes
- Out: reordering, pinning or drag of tabs; a keyboard shortcut for switching or closing
  (`ctrl+W`, `ctrl+Tab`: T-031's keymap); middle-click close; a confirmation dialog; closing
  several tabs; shortening or eliding paths; laying out only the tabs in view (T-081 is about the
  same question for the project panel).
- Do not edit `observe.rs` (T-066), `script.rs` or `window.rs` (T-068), `docs/protocol/ui.md`
  (T-066, T-068), `editor.rs`, `session.rs`, or any merged test file.
- `PanelView::changes` is the only addition to `mod.rs`: T-059 builds on it.

### Collisions and order
- **T-059** (status bar): uses `PanelView::changes`, which this task adds, so T-059 depends on
  T-058 and is **not in flight with it**. T-058 goes first: it has the medium item and the larger
  share of a usable application (switching documents). T-059's spec gets its spec check after
  T-058 merges.
- **T-066** (`observe.rs`, `ui.md`), **T-068** (`script.rs`, `window.rs`, `ui.md`), **T-071**
  (`xtask/`), **T-054** (root `Cargo.toml`, `docs/development.md`): no shared path; may be in
  flight beside T-058. T-066 changes how rows of shaped and right-to-left text are reported; AC-8
  already leaves those rows out.
- **T-060, T-061** (`outline.rs`, `find.rs`): no shared path, but they use `PanelView`; if one is
  specified before T-058 merges, its spec check re-reads `mod.rs`. **T-062, T-063** edit
  `shell.rs`: not in flight with T-058.

## Questions and rulings
Questions for the director, before dispatch:
- **Q-1. The `Documents` list.**
  - A: its entries become clickable and make a document current, like tabs (probed: all 110
    merged tests pass, and the click makes the entry current, E5).
  - B: it stays a list of labels, and `ui-tabs.md` says so (no code change in `shell.rs`; the
    evaluator's "inert" note becomes documented behaviour).
  - C: remove it (tabs replace it). Many merged T-055 tests use it (L1, L6, the
    `{"value":…}` targets), so they would have to be rewritten.
  - **Recommendation: A.**
- **Q-2. Which document becomes current when the current one is closed.**
  - A: the highest open id (unchanged; pinned by merged tests).
  - B: the neighbouring tab. This changes the asserted rule in `editor_t056.rs` (the "highest"
    snapshot) and is unmeasured.
  - **Recommendation: A.**
- **Q-3. Unsaved changes on close.**
  - A: a refused close shows a `Discard` button (probed, E3).
  - B: close always discards. Silent data loss: not recommended.
  - C: refuse only; the user must save or undo first.
  - **Recommendation: A.**
- **Q-4. What a tab paints.**
  - A: its path, with label = path and no value (F2 holds; no value collides with `Documents`).
  - B: its name, with value = name. Then the merged target `{"value":"f00.txt"}`
    (`script_t055.rs:439`) matches both the `Documents` entry and the tab and becomes
    `ambiguous_target` whenever `f00.txt` is open, by the definition of a target
    (`docs/protocol/ui.md`).
  - **Recommendation: A.**

### Rulings before dispatch (director, D-194)
- **Q-1: A.** `Documents` entries become clickable and make a document current.
- **Q-2: A.** Closing the current document makes the highest open id current. The neighbouring tab is the more
  usual behaviour; it stays out because it is unmeasured and changes a merged assertion (noted for the next
  retrospective's product list, not a task yet).
- **Q-3: A.** A refused close shows a `Discard <path>` button.
- **Q-4: A.** A tab paints its path; label = path, no value.
- Builds use `CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0` (D-186) until T-054 merges.

### Spec check R-276
Planner run R-276 (booked to `iter-2`), at main `c288f7f0926ca31e6b740640eaf36c5f7c520098` (T-057
merged at `7e8a01c`). The stub (R-225) was written before T-056 to T-065.

Changed against the stub:
- Tier medium (item `ui.tabs` medium); arm independent (even); budget 800,000 (v1.4).
- `depends_on` [T-056] → [] (merged).
- `touches` named and given one role each: `mod.rs` (one method), `shell.rs` (the `Documents`
  click, from the D-169 note) and `ui-editor.md` (the rule) added.
- The stub's questions answered:
  - confirming a discard: a second, explicit `Discard` click (Q-3);
  - what becomes current after a close: unchanged (Q-2);
  - the `Documents` note: entries become clickable (Q-1).

Evidence (a scratch copy of main under the session's scratchpad; `cargo` with
`CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0`; one tree, 20 GB free before the
build; deleted after this run):
- E1: the prototype was a `tabs.rs` (a wrapped `tab_list` in a `scroll_view`, tabs as clickable
  rects with `WidgetInfo::selected(Role::Tab, …)` and the path as label, a `modified` label node,
  a close button with a drawn cross, a `Discard` button after an `unsaved_changes` refusal,
  `doc.list` read when `changes()` differs), a `status.rs` (T-059's prototype) and
  `PanelView::changes()`. `cargo test -p kiln --locked --no-run` built in 41.96 s.
- E2: `cargo test -p kiln --locked --no-fail-fast` with the prototypes passed every target:
  0, 26, 28, 12, 7, 14, 2, 5, 9 and 7 tests (110 passed, 0 failed). That includes T-057's two
  layout-dependent tests and the four tests pinning the current-document rule.
- E3: the session of (g), outputs as quoted there. Also observed: a tab's selected state is
  reported as `toggled`; the line-break path's tab has rows `["new","line.txt"]` and an exact
  label; after `Discard`, the prototype left `Message` showing the refusal, so this spec states
  that a successful close clears it.
- E4: 300 documents opened by control requests at the default size: 62 tabs visible and 238
  `offscreen`. `click` on `f299.txt` gave `not_actionable` ("the target is offscreen"). `click` on
  `f000.txt` took 14 frames; `scroll` on `Tabs` 2 steps took 14; `click` on `f150.txt` took 4, and
  it became current. The whole script took 6.12 s and 35,412 KB. The same 300 opens plus 600 idle
  frames took 5.88 s.
- E5: `shell.rs` changed so that a `Documents` entry is a clickable label calling
  `Session::set_current`: all 110 tests passed again, and `click {"role":"label","value":"src/a.rs"}`
  made `Title` `src/a.rs — Kiln` in 4 frames.
- E6: the merged tests' targets and rule tests found by `grep` in `apps/kiln/tests/*.rs` (Pinned (f)).

Not checked: macOS and Windows (CI after T-070); a release build; the real window
(`kiln --script` under Xvfb); more than 300 documents.

### Rulings on review round 1 (director, D-201; `reviews/T-058-r1.md` at `20fb71378f2da53b8b009caf96b0e22538a73c33`; told to both builders)
- **h1 (finding 1, defect; implementer).** The `modified` mark paints exactly `modified` on one row whatever the
  strip's width and the path's length, and the tab's own rows hold its whole path. No wrap of the mark.
- **h2 (finding 2, test gap; test author).** `acceptance_t058.rs` gains a case with a dirty document whose path is
  wider than the strip at the default size: the rows of the tab and of its `modified` node are compared. It fails
  at the reviewed head `a56ba4e`. It also gains the assertion of h4 (an undo back to clean removes `Discard`).
- **h3 (finding 3, document sentence; implementer; the spec's wording corrected here).** The binary's rule stays
  (`Session::changes`): **a change is any successful request that reaches the session from outside the panels'
  own reads, a script's read-only control request included**; the panels read again on the next frame after one.
  `ui-tabs.md` states it so; the spec's Interface and T5 sentence "any source other than a read" is read as this.
  No product change. Idle frames still send nothing (AC-7 b).
- **h4 (finding 4, document sentence; implementer; the spec's T3 corrected here).** `Discard <path>` stays until
  the document is closed or is no longer dirty (a save, or an undo back to clean, by the UI or by a control
  request).
- **h5 (finding 5; implementer).** `docs/protocol/ui.md` joins `touches` for one sentence: `Message` begins with a
  refusal's kind after a refused open by the UI **and after a refused close from a tab**. Nothing else in that
  file (T-066 and T-068 edit it later and are not in flight).
- **h6 (finding 6, spec gap).** AC-4's "`ctrl+S` after the refused close" is read as "after the editor has focus
  again"; the tests give it focus first, and the document's sentence stays. That a click on a tab, a close button
  or a `Documents` entry leaves the keyboard nowhere is a product weakness, not this task's: T-082 (draft).
- **h7 (finding 7, nit; test author).** The last tab's group must not take in nodes outside the tab strip: bound
  it by the `Tabs` node if a snapshot gives the means, else by the first node that is not a tab, a `modified`
  mark, a `Close` or a `Discard` button. Say which was possible.
- **h8 (finding 8, nit; implementer).** The sentence "painted whole on one row" gets the spec's caveat back (cut at
  the window's edge).

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-14 · R-225 · (new) · did: filed as a draft for ADR-0007. · gate attempts: 0 · missing context: T-056's merged shell · next: director.
- 2026-11-15 · R-276 · draft · did: specified the tabs against main `c288f7f` with a prototype build in scratch (tabs, status bar, `Documents` click; all 110 merged `kiln` tests green); tier medium (item medium), independent arm (even); 9 criteria, AC-8 on generated sessions; the current-document rule gains one clause, and the four merged tests that pin the rest need no change; ordered before T-059 (it adds `PanelView::changes`). · gate attempts: 0 (scratch builds only) · not run: macOS, Windows, the window · missing context: none · next: director (Q-1 to Q-4).
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
