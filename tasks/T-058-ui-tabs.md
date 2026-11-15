---
id: T-058
title: "Show every open document as a tab; make a document current by clicking its tab or its Documents entry; close it from its tab, discarding unsaved changes only on a second, explicit click"
tier: medium            # ADR-0002 default (UI behaviour over existing commands; no dependency, gate path, persistence or process code); = the claimed item's tier (`ui.tabs`: medium); fixed at specified (METHODOLOGY v1.4 §4)
state: queued            # director only, written with every transition; see METHODOLOGY §3
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
- 2026-11-15 **Pre-merge platform run** (ADR-0009 Decision 5; D-204): run 43777023951 of `platforms.yml` on `ci/T-058` at
  `043ed2b07fc61b56b059b5db72761e1ed4c9e739` (the branch rebased onto main `20eb15a` in a throwaway worktree). The two
  counted jobs are green: `window-linux` success (`WINDOW CASES PASS`, `AC-3 PASS`, `PLAIN PASS: the start path
  changes the screen`) and `window-probe (macos-15)` success. `window-probe (windows-2025)` fails at "Build kiln" as
  on main; the three gate jobs were still running when read (23:56Z) and are not counted (D-202).

### planner
- 2026-11-14 · R-225 · (new) · did: filed as a draft for ADR-0007. · gate attempts: 0 · missing context: T-056's merged shell · next: director.
- 2026-11-15 · R-276 · draft · did: specified the tabs against main `c288f7f` with a prototype build in scratch (tabs, status bar, `Documents` click; all 110 merged `kiln` tests green); tier medium (item medium), independent arm (even); 9 criteria, AC-8 on generated sessions; the current-document rule gains one clause, and the four merged tests that pin the rest need no change; ordered before T-059 (it adds `PanelView::changes`). · gate attempts: 0 (scratch builds only) · not run: macOS, Windows, the window · missing context: none · next: director (Q-1 to Q-4).
<!-- end planner log -->
### implementer
- 2026-11-15 · R-279 · in_progress · did: read main's copy of this file; rulings Q-1 A, Q-2 A, Q-3 A, Q-4 A (D-194) and the build flags (D-186) accepted and followed. Criteria restated: AC-1 the tabs equal `doc.list` in order after every open/close, one `toggled` tab, the one `Title` names, no `Tabs` with nothing open; AC-2 a tab click shows its document (Title, Editor value and selection as the registry has them) and adds only `mutating: false` entries, one naming it, nothing for the current tab; AC-3 `Close` of a clean document sends `doc.close {"doc"}`, the highest id becomes current only if the closed one was current, `Message` empty; AC-4 `Close` of a dirty document is refused (`unsaved_changes`), `Discard` closes it with `discard: true`, a save instead removes `Discard` and a later `Close` keeps the edit on disk; AC-5 `modified` follows exactly the dirty tabs; AC-6 a `Documents` click is a switch; AC-7 control-only scripts log as `Host`, idle frames log nothing, a control edit of a non-current document brings a `doc.list` read and `modified`; AC-8 generated sessions equal a naive model and replay; AC-9 300 documents all reachable by scrolling `Tabs`. Built: `panels/tabs.rs` (a `tab_list` `Tabs` in its own `scroll_view`, wrapped; a tab is a clickable rect with `WidgetInfo::selected(Role::Tab, …)`, label = path, path painted with `TextWrapMode::Extend`; a `modified` label; a close button with a drawn cross; a `Discard <path>` button after an `unsaved_changes` refusal, dropped when the document closes or a `doc.list` read reports it clean; `doc.list` read when `PanelView::changes()` differs from the last read and a document is open; a click handled in its frame, then a repaint); `PanelView::changes` in `mod.rs`; in `shell.rs` a `Documents` entry's (already click-sensing, selectable) label response now calls `Session::set_current` on a primary or secondary click, nodes unchanged. `docs/protocol/ui-tabs.md` (new) and "The current document" of `ui-editor.md` rewritten as rules 1–4. Own tests `apps/kiln/tests/tabs_t058.rs`, 11 tests through `kiln script` (AC-1 to AC-9 each, AC-8 with 40 seeds × 30 steps and a `Host` replay, plus any-button clicks on Close/Discard/entries, focus, a long path, scroll logging nothing). · decisions within the spec: (1) the strip's height follows its content, up to max(5 rows, one scroll step + a row): with 3 rows, a `scroll` step (100 points, egui's fixed distance for a scroll action) was taller than the strip, so 24 of 300 tabs were never in view by scrolling (my AC-9 test caught it); now all 300 are; (2) the tab list reads `open_documents()` for labels/order and the last `doc.list` read only for `dirty`; (3) Close/Discard/Documents accept a secondary click and modifiers like tabs (the spec says so for tabs; a `Documents` entry is "the same control"). · deviations: T6 in `ui-tabs.md` gives my measurement, not the spec's 5.9 s (the prototype's): 300 control opens + `frames 600`, `/usr/bin/time … kiln script idle.jsonl --root root` in three runs: 5.56 s, 5.73 s, 5.38 s (35 MB), 4-core Linux (`nproc` 4), debug build with the D-186 flags; the log had 304 lines (300 opens + one frame's reads). The "with `at`" clause is narrowed to tabs: `at` on `Close` is refused `not_actionable` (it paints no text), as for any node without rows. · checked by hand (Linux, scratch root, deleted): Pinned (g) session — tab order and `toggled`, rows `["new","line.txt"]`, switch log `doc.info`/`doc.text`/`cursor.get`/`syntax.highlight` of doc 1 + `doc.list`, `modified`, refusal `unsaved_changes: …`, `Discard`, `{"discard":true,"doc":1}` in the log, `שלום.txt` current after, `Message` cleared, 300 idle frames nothing; T7: a click on the `new\nline.txt` tab made it current (`Title` `new\nline.txt — Kiln`) in 4 frames; 300 documents: 36 tabs visible at first with 3 rows, `click` on `f299.txt` `not_actionable`. Observation for AC-4 (no question; merged behaviour): any click outside the editor, `Close` included, takes the editor's focus (egui `surrender_focus_on` clicks), so `ctrl+S` after a refused close needs a click or `focus` on `Editor` first; my test focuses it. · commands: `cargo test -p kiln --locked --no-fail-fast` → all targets green (26, 28, 12, 7, 14, 2, 5, 9, 9 own, 7 — before the last two own tests were added; 11 own later); `cargo xtask clippy` PASS after attempt 1. · gate attempts: 3 (1 red clippy `type_complexity` in my test file, fixed; 2 stopped by my shell's 600 s limit after `scorecard` passed, no result; 3 green on `9fcb913`: 11 steps, `test` 1010 passed in 122 suites, fuzz and budgets EMPTY, 399.7 s). · not run: macOS, Windows (CI after T-070), the window (`kiln --script`), a release build; my tests were not run on base (not required of the implementer; each needs a `tab` node or a `Documents` switch, which base lacks). · missing context: none · next: director (join with the acceptance tests).
- 2026-11-15 · R-288 · in_progress (review round 1, request-changes) · **Rework after review round 1.** Read `reviews/T-058-r1.md` and main's "Rulings on review round 1" (D-201); h1, h3, h4, h5, h6, h8 accepted (h2 and h7 are the test author's). Scratch root under `/home/user/kiln-impl-T-058/r288` (deleted after). · h1: `tabs.rs` lays the `modified` mark out with `TextWrapMode::Extend`, so it is never wrapped into a wide tab's row. Probe (root `a.txt`, `b.txt`, `d/`+`x`×180+`.t`; three control opens, `frames 2`, click the wide tab, `focus` `Editor`, `type "Q"`, snapshot): before, tab row 177 chars starting `xxx`, mark rows `["d/xxxxx","modified"]`; after, tab row 184 chars = the whole path, mark rows `["modified"]`; same at `--size 3000x700`; at `--size 800x320` the tab row is a 117-char prefix (cut at the window's edge) and the mark one row `modified`. New own test `the_mark_of_a_wide_dirty_tab_is_one_row` (both sizes); it failed with the wrap mode removed (tab row ≠ path), passes with it. · h3: `ui-tabs.md` now says a change is any successful request from outside the UI's own reads, a script's read-only control request included, or a change of the current document; the editor view and the tabs read again in the first frame after one (the tabs in the click's own frame after a `Documents` click). Probe: three opens, `frames 5`, then `buffer.text` / `doc.list` / `project.tree` each followed by `frames 5`: each added `doc.info`, `doc.text`, `cursor.get`, `doc.list` after it; then `frames 300`: log unchanged; then a `Documents` click: `doc.list` logged before the editor's `doc.info`/`doc.text`/`cursor.get`. No product change. · h4: T3 reads "It stays until the document is closed or is no longer dirty (a save, or an undo back to clean, by the UI or by a control request)." Probe: four refused closes after `type "Q"`, each followed by one of `focus Editor` + `ctrl+Z`, control `doc.undo`, control `doc.save`, `focus Editor` + `ctrl+S`: `Discard` and `modified` gone after each; final `doc.list` all clean; saved files hold `Qa`/`Qc`. · h5: `ui.md`'s `Message` bullet adds "and after a refused close from a tab (`docs/protocol/ui-tabs.md`)" and `unsaved_changes` as an example; nothing else in that file. Evidence: the same probe's snapshots after each refused close show `Message` beginning `unsaved_changes:`. · h6: accepted; the focus sentence unchanged. · h8: the long-path limit now says a path wider than the window is cut at the window's edge (row = chars whose centre is inside, label exact, how many depends on the chars, as `ui-project.md` says for the panel) and that the mark after such a tab stays one row (h1 probe at 800x320). · commands: `cargo test -p kiln --locked --test tabs_t058 --test acceptance_t058`: 12 and 12 passed. · gate attempts: 1 (green on `bcaf05d`: 11 steps, `test` 1023 passed in 123 suites, fuzz and budgets EMPTY, 552.6 s). · not run: macOS, Windows, the window, a release build. · missing context: none · next: director (round 2).
<!-- end implementer log -->
### test author
- 2026-11-15 · R-280 · in_progress · did: wrote `apps/kiln/tests/acceptance_t058.rs` from the spec alone (12 tests, all through `kiln script`, temporary roots, every process under a 300 s timeout, no Unix-only API); rulings acknowledged and accepted: Q-1 A (`Documents` entries clickable: AC-6, AC-8), Q-2 A (highest open id: AC-3, AC-8), Q-3 A (`Discard <path>` after an `unsaved_changes` refusal: AC-4, AC-8), Q-4 A (tab label = path, no value, paints its path: `check_tabs`), D-186 build variables used for every cargo command. · gate attempts: 0 (`cargo xtask ci` not run, as dispatched; run instead: `cargo clippy -p kiln --tests` clean, `cargo fmt --check` clean, `cargo xtask hygiene` PASS, `cargo test -p kiln --test acceptance_t058`: 0 passed, 12 failed on base) · not run: macOS, Windows, the window (`kiln --script`), the line-break path (the reviewer's recorded procedure, Platforms) · missing context: none · next: director (join).
  - Tests per item `ui.tabs` (every test below; all through the CLI), criterion → test → result on base (`c6f8c55` + these tests):
    - AC-1 → `ac1_tabs_follow_random_opens_and_closes` (4 seeds × 16 steps: control `doc.open`, `Path`/`Open`, project-panel clicks, control `doc.close`) → assertion: no `Tabs` while a document is open (seed 1 step 0).
    - AC-2 → `ac2_click_on_a_tab_switches_with_reads_only` (Title, `Editor` value = `doc.text`, `selection` = `cursor.get`, added log entries all `mutating: false` and one naming the document, by comparing sessions that differ by one click; a click on the current tab leaves the log equal) → command error: `click` on the tab answers `no_target`.
    - AC-3 → `ac3_close_clean_documents_from_their_tabs` (current re-opened so that the highest id differs from the neighbour; `Message` set by a refused UI open before each close) → command error: `click` on `Close c.txt` answers `no_target`.
    - AC-4 → `ac4_refused_close_then_discard`, `ac4_refused_close_then_save_then_close` (second session: `ctrl+S`, `Discard` gone, close without discard, file = `doc.text` after the edit) → assertion: no `Tabs` (both).
    - AC-5 → `ac5_modified_marks_follow_doc_list` (4 seeds × 24 steps: tab clicks, UI typing, `ctrl+Z`, `ctrl+S`, control `doc.insert`/`doc.undo` of non-current documents) → assertion: no `Tabs`.
    - AC-6 → `ac6_click_on_a_documents_entry_switches` → assertion: after the click on the `Documents` entry `src/a.rs`, `Title` still begins with `c.txt` (the spec's stated base failure).
    - AC-7 → `ac7a_control_requests_only_log_as_host` (responses and `--log` equal `Host`'s; a second session with one more frame shows the tabs and a `doc.list` read, so the equality is not vacuous) → assertion: no `Tabs` in the second session; `ac7b_idle_frames_after_tabs_add_nothing` → command error: `click` on the tab `a.txt` answers `no_target`; `ac7c_a_control_edit_is_read_then_idle` → assertion: no `Tabs`.
    - AC-8 → `ac8_generated_sessions_follow_the_naive_model` (40 seeds × 30 steps over T-057 AC-3's alphabet; opens by control and by `Path`/`Open`, tab clicks, `Documents` clicks, closes, discards, typed text, `ctrl+S`, `scroll` of `Tabs`; model in the test file; replay through `Host`; seed, step, open count and file names in every message) → assertion: no `Tabs` (seed 1 step 0).
    - AC-9 → `ac9_every_tab_of_300_documents_can_be_reached_and_clicked` (one survey run scrolling `Tabs` step by step, then every tab clicked at a position where it is not `offscreen`, `Title` checked after each click; a click on an `offscreen` tab is `not_actionable`; every action `settled`) → assertion: no `Tabs` in the survey's first snapshot.
  - Self-check: a scratch stub of the panel (its own `doc.list` read on `changes()`, a wrapped `tab_list` in a scroll area of at most 110 points, a drawn close cross, a `Discard` button after `unsaved_changes`) plus `PanelView::changes` and a clickable `Documents` entry, in a copy of the worktree outside the repository, never committed: all 12 tests passed (AC-9 included). Seeded bugs and the tests that killed them: B1 a switch sent as `doc.open` (mutating) → `ac2`; B2 `Close` always discards → `ac3`, both `ac4`, `ac7b`, `ac8`; B3 `Discard` kept after a save → `ac4_refused_close_then_save_then_close`, `ac8`; B4 `modified` after the current tab whenever any document is dirty → `ac5`, `ac7a`, `ac7b`, `ac7c`, `ac8`; B5 `doc.list` read on every frame → `ac2` (current-tab click logs), `ac7b`, `ac7c`; B6 `toggled` by position instead of id → `ac1`, `ac3`, `ac7a`, `ac7b`, `ac8`; B7 `Message` not cleared after a successful close → `ac3`, `ac4_refused_close_then_discard`; B8 `Documents` entries inert → `ac6`, `ac8`; B9 `doc.list` re-read only when the number of documents changes → both `ac4`, `ac5`, `ac7b`, `ac7c`, `ac8`. No bug survived; AC-9 killed none of these (it checks reachability and switching only, which every bug kept).
  - Harness defects found against the stub and fixed before handing over: `Path` cleared with `End` + `Backspace` failed for right-to-left text (the toolkit's caret), now one `Backspace` and one `Delete` per char wherever the caret is; AC-7(c) had a control request just before the idle frames (a control request is itself a change), now last in both sessions.
  - Assumptions where the spec left room (none changes a criterion; each is a reading the director may overrule):
    - "After it in the depth-first order and before the next tab": for the last tab the nodes up to the end of the snapshot (a later panel that shows a node of role `label` with value `modified` after the tabs would be counted for the last tab). `Close`/`Discard` labels are also required to occur nowhere else.
    - "Painting no text" for `Close <path>`: no `rows`, or only empty rows. Rows are compared only for nodes that are not `offscreen` and for paths without a line break that do not begin with right-to-left text or `٣` (AC-8's rule, applied to every test); `Discard` rows under the same rule for its path.
    - `doc.close` params are compared exactly as Interface writes them: `{"doc": id}` for `Close`, `{"doc": id, "discard": true}` for `Discard`.
    - "The path `Title` begins with": the longest open path that is a prefix of `Title` (no suffix pinned).
    - "Names the new document": a logged read whose params have `doc` equal to the new id.
    - AC-7 (a) as written passes on base (no frame, no read); the test adds a second session with one frame that must show the tabs and a `doc.list` read, so that it fails on base and is not vacuous.
    - "The frames until the UI settles" (AC-7 (c)): `frames` 60 (the settle cap), not a count of frames; after other control requests the tests run `frames` 10 before looking (the effect is shown from the next frame on).
    - AC-9: the strip is scrolled with `scroll` on `Tabs` one step at a time, `down` (or `right` if a `down` step changes nothing), at most 60 steps, after a 100-step scroll back to the start; if a click itself moves the strip, the run is repeated with the scroll back before every click.
    - The `Editor` is focused with `focus` before typed text and `ctrl+S` (L4: nothing takes focus by itself, and the spec does not say where focus is after a tab click).
- 2026-11-15 · R-289 · review round 1 request-changes · did: read `reviews/T-058-r1.md` (at `20fb713`) and the rulings D-201 on main, all accepted (h2, h4, h7 mine; h3: a script's read-only control request is a change, checked that no test of mine assumes otherwise: every session that compares logs or idle frames ends its control reads after its last frames or puts them in both sessions, AC-7(c) since R-280; h6: the tests already focus the editor before `ctrl+S`, no change); no product code or implementer branch read beyond the review. Added `ac5_a_wide_dirty_path_keeps_its_row_and_its_mark` (h2: paths `d/` + 120…240 `x` + `.t` in steps of 4 and the reviewer's 180, each dirty by a control `doc.insert`, the 180 one also by typing after a click on its tab; the tab paints one row, the start of its path, the same row as while clean; its `modified` node is not `offscreen` and paints exactly one row `modified`; that the lengths straddle the strip is found from the clean snapshots: at least one row whole and one cut, no width pinned) and `ac4_refused_close_then_undo_removes_discard` (h4: after the refused close, `ctrl+Z` in the focused editor, and in a second session a control `doc.undo`, make `doc.list` clean and remove `Discard b.txt`; the file is untouched). h7: each tab's group now ends with the `Tabs` node's subtree (the snapshot gives it), so the last tab's group takes in nothing outside the strip; `Close`/`Discard` labels are still required to occur nowhere else. · gate attempts: 0 (`cargo xtask ci` not run, as dispatched; `cargo clippy -p kiln --tests` clean, `cargo fmt --check` clean, `cargo xtask hygiene` PASS; `cargo test -p kiln --test acceptance_t058` on base: 0 passed, 14 failed, both new tests by the assertion that `Tabs` is missing while documents are open) · stub check (scratch copy under `/home/user/kiln-tests-T-058/`, deleted): the R-280 stub rebuilt with the tab text and `Discard` on one row and the mark `Label::extend()`: 14 passed. Fault of finding 1 imitated by letting the mark wrap (`Label::wrap()`): only the new h2 test failed, the 156-char path's `modified` reporting `["a.txt","modified"]` (a foreign first row, as the reviewer's `["d/xxxxx","modified"]`); the reviewed head itself was not run. `Discard` kept while the document is open: the new h4 test and `ac4_refused_close_then_save_then_close` failed. · not run: the reviewed head `a56ba4e`, macOS, Windows, the window · missing context: none · next: director.
  - Assumption added: in the wide-path test the `modified` node must not be `offscreen` with one long tab open (the strip shows at least the tab's row and the next; the review measured a strip of at least 5 rows), so that its rows are compared rather than skipped.
<!-- end test author log -->
### reviewer
- 2026-11-15 · R-283 · tests_joined · did: round 1 full review of `a56ba4e88ebc6c438209b31dbe96dd0839437b0b` (`reviews/T-058-r1.md`); rulings Q-1 A, Q-2 A, Q-3 A, Q-4 A (D-194) and the build flags (D-186) read on main and accepted. Every AC and Pinned item checked through `kiln script`; my own seeded differential run (seeds 1000–1199, 200 sessions × 40 steps, generated names over a stated alphabet with ligatures, combining marks, a joiner, right-to-left text, CJK and emoji): 0 mismatches; the line-break path by hand on Linux: as T7 says; 19 mutants, 19 killed, 18 by `acceptance_t058`, M17 only by the implementer's tests; the four merged rule tests pass unchanged; no merged test file in the diff. Verdict request-changes: 3 defects (the `modified` mark takes a wide dirty tab's text, against F2; two `ui-tabs.md` sentences false against the binary: what counts as a change, when `Discard` goes), 1 test gap, 2 spec gaps (ui.md's `Message` sentence; focus after a refused close), 2 nits; `doc_sentences` 3. · gate attempts: 0 (focused `kiln` test files only; `cargo xtask hygiene` on the review commit) · not run: the window (no `libxkbcommon-x11` here), macOS, Windows, a release build · missing context: none · next: director (findings 1–4 to the implementer; 5, 6 for a ruling).
- 2026-11-15 · R-290 · tests_joined (second join) · did: round 2, final, the delta `20fb713`..`1f895dfd4627fba813e66589b61a6818aade9001` (`reviews/T-058-r2.md`); rulings h1–h8 (D-201) read on main and accepted. All eight round-1 findings confirmed by my own runs: 1 fixed (78 wide-dirty-path cases at the default size, 800x320 and 3000x700, plus 867 marks in a crowded strip, all `["modified"]`; the tab's row unchanged); with the h1 fix reverted, the new `ac5_a_wide_dirty_path_keeps_its_row_and_its_mark` fails; 2, 3, 4, 5, 7, 8 fixed; 6 ruled (h6, T-082). Every sentence the rework added or changed in `ui-tabs.md` and `ui.md` is true against the binary, including the h3 wording and the `Documents`-click exception (log order `doc.list` before the editor's reads; one read); `doc_sentences` 0. 7 mutants: 4 killed by `acceptance_t058`, 1 by merged `editor_t056`, 2 equivalent, 0 survived. Delta inside `touches`, each path by its role; no merged test edited; no Unix-only API. Differential re-run: seeds 1000–1059 × 40 steps, 0 mismatches. Verdict approve. · gate attempts: 0 (focused `kiln` test files; full `kiln`+`kiln-ui` for unkilled mutants; `cargo xtask hygiene` on the review commit) · not run: the window (no `libxkbcommon-x11`), macOS, Windows, a release build · missing context: none · next: director.
<!-- end reviewer log -->
