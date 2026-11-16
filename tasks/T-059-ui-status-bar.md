---
id: T-059
title: "Show the current document's caret line and column, encoding and line-ending style in a status bar below the editor, re-read only after a change"
tier: medium            # ADR-0002 default (UI behaviour over existing commands; no high criterion); ≥ the claimed item's tier (`ui.status-bar`: low; items ⇒ medium); fixed at specified (METHODOLOGY v1.4 §4)
state: merged            # director only, written with every transition; see METHODOLOGY §3
owner_role: planner
scorecard_items: [ui.status-bar]
depends_on: [T-058]     # merged at 7acf7ce (`PanelView::changes`); spec check R-295 done
touches:                # one role per path (D-153); no gate path
  - crates/kiln-ui/src/panels/status.rs      # implementer: the panel (created empty by T-056)
  - apps/kiln/tests/acceptance_t059.rs       # implementer, on task/T-059-tests first (odd arm, METHODOLOGY §6)
  - apps/kiln/tests/status_t059.rs           # implementer: its own tests through the binary
  - docs/protocol/ui-status.md               # implementer: new; the sentences of Pinned (i)
budget_tokens: 800000   # medium (v1.4)
iteration: 2
found_by: planner       # ADR-0007 row 9 (run R-225); specified in run R-276
origin_task: null
---

# T-059 The status bar

## Goal
After this task the UI shows, below the editor, the current document's caret position (line and
column), its encoding and its line-ending style, each as a node a script can read. All of it
comes from the verified commands `cursor.get` and `doc.info`. The bar re-reads only in the frames
after a change, as the editor does, so idle frames still add nothing to the log.

Claims `ui.status-bar` ("The status bar shows line and column, encoding and line-ending style").
Tier medium, odd number, so the **implementer-first arm** applies: the implementer writes the
acceptance tests first on `task/T-059-tests` and runs the whole gate on a local merge before
reporting. No gate path; `ci/T-059` before the queue once the platform jobs count (ADR-0009). The
tests use no Unix-only API.

Reading list: `docs/adr/0008-ui-observation.md`; `docs/protocol/ui.md`;
`docs/protocol/ui-editor.md` ("The current document", "Frames, reads and the log");
`docs/protocol/ui-tabs.md` (T-058); `docs/protocol/cursor.md` (line and column in chars);
`docs/commands.json` for `cursor.get` and `doc.info`; `crates/kiln-ui/src/panels/mod.rs`;
`tasks/T-058-ui-tabs.md` Pinned (f).

## Interface

No new registry command, UI request kind, refusal kind or option.

### What the bar shows (nodes in a `kiln script` snapshot)
When there is a current document, four nodes of role `label`, each with an accessible name and a
`value` equal to its exact painted text (one row, so F2 holds):

| `label` | `value` | Source |
|---|---|---|
| `Line` | `Ln ` followed by the caret's line, 1-based, in decimal (`cursor.get` `line` + 1), e.g. `Ln 2` (R-295: a bare number could equal a generated document path, Pinned (f)) | `cursor.get` |
| `Column` | `Col ` followed by the caret's column, 1-based, in chars (`cursor.get` `col` + 1): a combining mark, a joiner or a tab each count as one char; e.g. `Col 6` | `cursor.get` |
| `Encoding` | exactly `doc.info`'s `encoding`: `utf-8`, `utf-8-bom`, `utf-16le` or `utf-16be` | `doc.info` |
| `Line ending` | exactly `doc.info`'s `line_ending` (`lf`, `crlf`, `cr`, `none`), followed by ` (mixed)` when `mixed_line_endings` is true | `doc.info` |

- The caret is the selection's head (`cursor.get` `head`).
- With no current document, none of the four nodes exists.
- The values are of the current document (T-058's rule), read after the change that made it
  current.
- Not shown: dirty state (`Title` and the tab show it), selection size, language or syntax mode.
  No command reports a language: the editor colours a document by its path's suffix, which is the
  UI's rule, not a command's result.
- Where the bar sits: below the editor, at the bottom of the document area, under the tab strip,
  `Title`, the project panel and the editor (`panels/mod.rs`; the shell draws it first, bottom
  up). Its layout is not specified for tests. The bar is one row tall.
- Measured (R-295, prototype on main `c772117`): at the default size with 300 open documents, the
  four nodes are on screen and each paints exactly its value. At `--size 800x320` with 300 open
  documents, the four nodes exist and are not `offscreen`, but **none has a row**: no char of the
  bar is reported as painted in it. Q-5 decides whether that must be fixed here.

### UI requests that act on it
None: the bar is a display. It takes no focus, and a click on it does nothing (stated as
behaviour in `ui-status.md`, so the evaluator's inert check (O5) does not report it).

### Control requests the bar sends, and when
- **Reads**: read-only `cursor.get {"doc": <current>}` and `doc.info {"doc": <current>}` in the
  first frame after a change, in the words of `docs/protocol/ui-tabs.md` (T-058 ruling h3,
  D-201). A **change** is any successful request that reaches the session from outside the UI's
  own reads (the editor view's and the panels' read-only requests), a script's read-only control
  request (`doc.list`, `buffer.text`, `project.tree`, …) included, or a change of the current
  document (a tab or `Documents` click). The bar reads again in the first frame after one; how
  many reads one action causes is not part of this interface. The bar detects a change by
  `PanelView::changes()` or `PanelView::current()` differing from what it last read for. Both
  reads are logged.
- **Nothing** on an idle frame, on a scroll or a click that sends no request, or with no document
  open. A script of control requests only writes `Host::log_script()` (L7).
- A refused read (it cannot happen for an open document) leaves the bar empty and `Message`
  unchanged.
- The bar does not share the editor's reads: the log holds the editor's `cursor.get` and
  `doc.info` and the bar's own (measured: two more read-only requests per change, E2). Q-3.

### Long values and special text
Values are short (at most a decimal number or an encoding name plus ` (mixed)`). The document's
text is never shown in the bar, so right-to-left text, line breaks and shaped text in the document
affect only the `Column` count, which is in chars as `cursor.get` reports it.

### Platforms (ADR-0009)
No platform difference: the values are the commands' own. Test files use no Unix-only API.
Evidence for macOS and Windows comes from CI once T-070 has merged.

### Rust items this builds on (on main `c772117`, T-058 merged; not used by tests)
```rust
pub(crate) fn show(ui: &mut egui::Ui, view: &mut PanelView<'_>);    // panels/status.rs, kept
impl<'a> PanelView<'a> {
    pub(crate) fn read(&mut self, command: &str, params: serde_json::Value) -> Result<serde_json::Value, (String, String)>;
    pub(crate) fn current(&self) -> Option<u64>;
    pub(crate) fn changes(&self) -> u64;   // merged, panels/mod.rs:78; counts every successful request through the session (`Session::handle_line`), a script's read-only ones included, and every change of the current document; not the panels' and the editor's `read`s (session.rs:44–66, 84–91)
}
```
The shell gives `show` a bottom-up `Ui` (`panels/mod.rs`). `changes()`'s doc comment says "other
than a read"; the code counts a script's read-only control requests too, as h3 says (R-295).

### Rust API used by tests
None. All criteria are tested through the built `kiln` binary, with `kiln_control::Host` as the
only Rust oracle (T-055's signatures).

## Pinned
- (a) Errors: n/a. The bar sends reads only, and they cannot be refused for an open document; if
  one is, the bar shows nothing.
- (b) Files: none. Source: `status.rs` only.
- (c) Order: in the frame after a change the editor reads first (it runs before the layout) and the
  bar reads during the layout. Both show the state after the change from that frame on, within the
  action's settling.
- (d) Line and column as `cursor.get` gives them, plus one. Linux evidence until T-070.
- (e) Threat model: n/a (two read-only requests per change).
- (f) Facts the merged tests rely on, which this task keeps:
  - F2: the four nodes each paint exactly their value (one row) wherever they have a row.
  - **No target collision (R-295).** Merged tests target `Documents` entries as
    `{"role":"label","value":<path>}` (`acceptance_t058.rs:255`, `tabs_t058.rs:199`). The bar's
    nodes are also of role `label`. `acceptance_t058.rs`'s generator (`PIECES`, line 2309) can make
    a root file named `7` and so a document path `7`, which a bare line or column number could
    equal: then that Documents click would be `ambiguous_target`. Hence the `Ln `/`Col ` prefixes.
    No path the merged generators make can equal `Ln …`, `Col …`, an encoding name, or a line-ending
    value: `PIECES` and `JOINS` contain no `L`, `C`, `u`, `t`, `l`, `o`, `c`, and `T-057`'s
    generator is the same alphabet. The bar adds no node labelled `Close …`, `Discard …`, `Tabs`,
    `Project`, `Project files`, `Path`, `Open`, `Documents`, `Editor`, `Title` or `Message`. It
    adds no node with value `modified`, and none inside the `Tabs` subtree, so `acceptance_t058`'s
    group bounds (each tab's group ends with the `Tabs` subtree) and its "nowhere outside the
    strip" checks are unaffected. `{"value_contains":"."}` (expected ambiguous) gains no match.
  - L1–L8: L4 the bar takes no focus; L5 none of the labels `Line`, `Column`, `Encoding`,
    `Line ending` is `Path`, `Open`, `Documents` or `Message`, or contains `Pat` or `Ope`; L7; L8
    (settling within 60 frames: measured 3–5 with the bar, E2).
  - T-057's layout-dependent tests (`ac2_…` at 800x320, `ac3_…` at 1280x8192): passed with the bar
    and the tab strip both present (E1). `acceptance_t057.rs` is not in `touches`. A failure is a
    `QUESTION:`.
  - T-058's tests (`acceptance_t058.rs`, `tabs_t058.rs`, on main) and T-057's layout-dependent
    tests: with the prototype bar on main `c772117`, every target of `apps/kiln` passed, including
    `acceptance_t058` (14) with `ac5_a_wide_dirty_path_keeps_its_row_and_its_mark` and `ac9`'s 300
    documents, `tabs_t058` (12), `acceptance_t057` (12) with `ac2`/`ac3`: 158 tests in all, 0 failed
    (R-295, E3). `acceptance_t058.rs` and `acceptance_t057.rs` are not in `touches`.
  Not to be pinned: a whole snapshot, the bar's position, its separators or decoration, which
  reads the editor sends.
- (g) Worked examples, executed with `kiln script` on a scratch build of main `c288f7f` plus the
  prototype (E2):
  - after six control opens, the last `שלום.txt` (`h\n`): `Line` `1`, `Column` `1`, `Encoding`
    `utf-8`, `Line ending` `lf`;
  - `mixed.txt` (`x\ny\r\n`) answers `doc.open` with `line_ending: "lf"` and
    `mixed_line_endings: true`, so `Line ending` is `lf (mixed)`;
  - `bom.txt` answers `encoding: "utf-8-bom"`;
  - `b.txt` (`b\r\nc\r\n`) answers `line_ending: "crlf"`;
  - in `src/a.rs` (`fn main() {\n    let x = 1;\n}\n`), a click `at {row:1, col:4}` on `Editor` and
    `type "z"` give `cursor.get` line 1, col 5 (head 17, T-056 (g)), and the bar shows `Line`
    `Ln 2`, `Column` `Col 6` (the prototype showed `2` and `6`, without the prefixes added by
    R-295).
- (h) Evaluator calls for `ui.status-bar` (ADR-0008 (O1)–(O5)): `kiln script` with documents of
  each encoding and line-ending kind (including mixed and none), caret moves by keys and clicks;
  snapshots before and after, the four values against `cursor.get` and `doc.info` in the same
  session; `--log` for the bar's reads (read-only, none on idle frames).
- (i) Sentences for `docs/protocol/ui-status.md`, worded for copying, with their evidence:
  - S1 "The status bar shows, for the current document, the caret's line and column (as
    `Ln <n>` and `Col <n>`, both counted from 1; the column in chars, as `cursor.get` counts it), the encoding as `doc.info`
    names it, and the line ending as `doc.info` names it, followed by ` (mixed)` when the line
    endings are mixed." Evidence: E2; AC-1, AC-2.
  - S2 "With no document open the bar shows nothing." Evidence: AC-5.
  - S3 "The bar reads `cursor.get` and `doc.info` in the frame after a change and never on an idle
    frame; its reads are in the session log." Evidence: E2 log; AC-4.
  - S4 "The bar is a display: a click on it does nothing." Evidence: AC-5.

## Acceptance criteria
All drive the built `kiln` binary (`kiln script`); nodes found by role and label. Each fails on
base, where no node labelled `Line` exists (an assertion on its absence, or `no_target`). The
implementer commits them failing on `task/T-059-tests` first.
- AC-1 (`ui.status-bar`, encoding and line ending): for generated documents of each encoding
  (`utf-8`, `utf-8-bom`, `utf-16le`, `utf-16be`) × each line-ending kind (`lf`, `crlf`, `cr`, no
  break, mixed), opened one after another, the snapshot after each open shows `Encoding` and
  `Line ending` equal to that document's `doc.info` (with ` (mixed)` exactly when
  `mixed_line_endings`). After a control `eol.normalize` the value follows `doc.info`. Oracle:
  `doc.info` in the same session.
- AC-2 (`ui.status-bar`, line and column, generated, D-174 change 5): for at least 40 seeds, a
  document whose text is drawn from a stated alphabet (ASCII, spaces and tabs, `é` and
  `e`+U+0301, a zero-width joiner, `ffi`, `漢字`, `😀`, `שלום`, `سلام`, `٣`, LF, CRLF and CR
  breaks), and 40 random caret steps through the UI (arrow keys with and without `ctrl` and
  `shift`, `Home`, `End`, clicks `at` a random visible row and column, typed text, `Enter`,
  `Backspace`). After every step `Line` = `Ln ` + (`cursor.get` `line` + 1) and `Column` =
  `Col ` + (`col` + 1).
  Oracle: `cursor.get` in the same session.
- AC-3 (switching, T-058's switching clause): with two documents of different encodings and line
  endings and different carets open, switching by a tab click, by a `Documents` click and by a
  control `doc.open` (`already_open`) shows the newly current document's values. After a refused
  close (`Close <path>` of a dirty document, `unsaved_changes`) the four values are unchanged.
  After closing the current document (a clean one, by `Close <path>`) they are those of the open
  document with the highest id. Oracle: `cursor.get` and `doc.info` of that document;
  `docs/protocol/ui-editor.md`'s rule.
- AC-4 (reads and idle): (a) a script of control requests only writes `Host::log_script()`;
  (b) 600 frames with a document open, run after the last control request, add nothing to the log;
  (c) after a control `cursor.set` on the current document, the frames until the UI settles add
  read-only requests only, and the bar's `Line` and `Column` follow the new head; (d) after a
  script's read-only control request (e.g. `doc.list`), the frames that follow may add read-only
  requests and 600 further frames add nothing. No read count is pinned. Oracle: log comparison;
  `cursor.get`.
- AC-5 (no document; display only): a fresh session and a session after the last document is
  closed have none of the four nodes. A `click` on `Line` changes no node's value, adds nothing to
  the log, and settles. Oracle: snapshot, log.

## Out of scope and implementer notes
- Out: dirty state, selection size, a language or syntax mode, the document's length, a clickable
  bar (go-to-line from the bar: T-060's), configuring the bar.
- Test notes (R-295): a click on a tab leaves the keyboard nowhere (draft T-082), so every test
  that types or sends a key to the editor first gives it focus with `focus` on `Editor` or a
  `click` on it; none relies on focus after a tab click. Tests compare logs only for frames run
  after their last control request, or put the same control reads in both sessions (a script's
  read-only control request is a change, h3). The tests use no Unix-only API.
- Order of delivery (odd arm): `apps/kiln/tests/acceptance_t059.rs` is committed first, failing
  on base, on `task/T-059-tests`; then `status.rs`, `status_t059.rs` and `ui-status.md` on
  `task/T-059`; then the implementer runs the whole gate (`no-skip` included) on a local scratch
  merge of the two branches before reporting. All four paths belong to the implementer.
- Do not edit `mod.rs` (if more than `changes()` is needed, `QUESTION:`), `observe.rs`
  (T-066), `script.rs`, `window.rs` (T-068), `ui.md`, `editor.rs`, `shell.rs`, or merged tests.

### Collisions and order
- **T-058**: merged at `7acf7ce`.
- **T-054** (`Cargo.toml`, `docs/development.md`, `xtask/tests/`), **T-071** (`xtask/` only),
  **T-066** (`observe.rs`, `ui.md`), **T-068** (`script.rs`, `window.rs`, `ui.md`): no shared
  path; may be in flight beside T-059.
- **T-082** (draft: `tabs.rs`, `shell.rs`, `ui-tabs.md`): no shared path unless Q-5 A adds
  `shell.rs`, in which case they are not in flight together.
- Platforms: `apps/` is platform-sensitive (ADR-0009 Decision 5). The director pushes the rebased
  head to `ci/T-059` and queues only when the counted jobs are green: `window-linux` and
  `window-probe (macos-15)` count since D-202.
- **T-060, T-061**: no shared path. **T-062, T-063**: they edit `shell.rs`, which T-059 does not
  touch; no conflict by path.

## Questions and rulings
Questions for the director, before dispatch:
- **Q-1. Base of line and column.**
  - A: 1-based for people (as specified).
  - B: 0-based, as `cursor.get` reports.
  - **Recommendation: A.**
- **Q-2. What the bar shows.**
  - A: exactly the item's four (as specified).
  - B: also the dirty state and the selection size. More nodes to keep in step with the commands,
    and none of it is claimed by the item.
  - **Recommendation: A.**
- **Q-3. Its own reads or the editor's.**
  - A: the bar reads `cursor.get` and `doc.info` itself (measured: all merged tests pass, two more
    reads per change, E1, E2).
  - B: the editor's last read is handed to panels through `PanelView`. This edits `editor.rs`,
    `mod.rs` and `shell.rs`, is unmeasured, and would collide with T-058's paths.
  - **Recommendation: A.**
- **Q-4. Order with T-058.**
  - A: T-058 first, adding `PanelView::changes` (as specified).
  - B: T-059 first, adding it itself; then T-058 depends on T-059.
  - **Recommendation: A** (the tabs carry the medium item and the switching a user needs).

- **Q-5 (R-295). The bar at a small window with many tabs.** At `--size 800x320` with 300 open
  documents the four nodes have no rows (measured, the bar not shown as painted).
  - A: require that each node has exactly one row equal to its value at that size too, and allow
    `crates/kiln-ui/src/shell.rs` in `touches` (implementer: keeping the area above the bar from
    covering it). **Unmeasured**: no fix was probed (the scratch disk was below the 8 GB stop line
    after one build). It needs a probe first: a prototype that clips the area above the bar, then
    `acceptance_t057`/`acceptance_t058`/`tabs_t058` at 800x320.
  - B: state it as a limit in `ui-status.md`, with the measurement, and file a follow-up that
    probes A (measured; no `shell.rs` change, so no collision with T-082).
  - **Recommendation: B**, with the follow-up dispatched after T-082 (both would edit `shell.rs`).
    If B is ruled, `ui-status.md` gets S5: "In a small window with many tabs the bar can be
    covered: at 800x320 with 300 open documents its values are not painted (R-295)."

### Rulings (director, D-194)
- **Q-1: A.** Line and column are 1-based.
- **Q-2: A.** The item's four fields only.
- **Q-3: A.** The bar makes its own reads.
- **Q-4: A.** T-058 first; this spec gets its spec check against main after T-058 merges, then `ready`.

### Spec check R-276
Planner run R-276 (booked to `iter-2`), at main `c288f7f0926ca31e6b740640eaf36c5f7c520098`.
`depends_on` T-058 was not merged when this was written, so this spec needs a spec check after
T-058 merges. That check pins `PanelView::changes` as merged and reads T-058's acceptance tests
for assumptions about the space below the editor.

Changed against the stub:
- Tier medium (item low; items ⇒ medium); arm implementer-first (odd); budget 800,000 (v1.4).
- `depends_on` [T-056] → [T-058].
- `touches` named, one role each, with the implementer's own test file added.
- The stub's questions answered: base (Q-1, 1-based) and unit (chars, as `cursor.get`).

Evidence (the same scratch build as T-058's spec check, with both prototypes present; deleted
after this run):
- E1: with a prototype `status.rs` (four `label` nodes with accessible names `Line`, `Column`,
  `Encoding`, `Line ending`, reading `cursor.get` and `doc.info` when `(changes(), current())`
  differs), `cargo test -p kiln --locked --no-fail-fast` passed all 110 tests. That includes T-057's
  two layout-dependent tests and `z1_rows_equal_the_text_each_node_shows` (F2).
- E2: the session of T-058's (g):
  - the bar's values after the opens, after the switch to `src/a.rs` and after the click and type,
    as quoted in (g);
  - actions settled in 3 to 5 frames;
  - the log shows the bar's `cursor.get` and `doc.info` after the editor's reads in each changed
    frame, and nothing during 300 idle frames;
  - `cursor.get` of doc 6 at the end, `line 0, col 0`, matched `Line` `1`, `Column` `1`.

Not checked: T-058's final shape (not merged); macOS and Windows; a release build; the window.

### Spec check R-295
Planner run R-295 (booked to `iter-2`, resumed), at main `c77211720b2f89bc2998acc337ba3a77c95d2a3c`
(T-058 merged at `7acf7ce`).

What changed:
- The read rule is worded as h3 (D-201). AC-4 now names no read count and adds case (d) for a
  script's read-only control request.
- `changes()` is pinned as merged, and its doc comment's "other than a read" is noted as narrower
  than the code (`Session::handle_line` counts a script's read-only requests).
- The values of `Line` and `Column` gain the prefixes `Ln ` and `Col `. Reading
  `acceptance_t058.rs` showed that a bare number can collide with a generated document path in a
  Documents-click target of role `label`. The prototype tests passed, but only by the seeds' luck:
  a root file `7` is possible.
- AC-3 covers the switching clause (tab and `Documents` clicks), a refused close, and the close of
  the current document. AC-5 covers the last close.
- Focus and log-comparison test notes added; the arm's order of delivery stated; `ci/T-059` and
  the two counted jobs (D-202).
- Q-5 is new (a small window).

Measured (one scratch tree under `/home/user/kiln-plan-T-059/`, `CARGO_PROFILE_DEV_DEBUG=line-tables-only
CARGO_INCREMENTAL=0`; `df -h /home/user` showed 8.0 GB free before the build and 6.2 GB after; the
tree was deleted after the runs (7.7 GB free), and no second build was made, being under the 8 GB
stop line):
- E3: the R-276 bar prototype on current main; `cargo test -p kiln --locked --no-fail-fast` passed
  every target: 0, 26, 28, 12, 14, 7, 14, 2, 5, 9, 12, 7 (158 passed, 0 failed), among them
  `acceptance_t058` (incl. ac5 and ac9) and `tabs_t058`.
- E4: 300 control opens, `frames 2`, then snapshot, a `Documents` click on `f010.txt`, snapshot, a
  control `doc.close` of doc 300, `frames 2`, snapshot:
  - at the default size, `Line`, `Column`, `Encoding`, `Line ending` on screen with rows `1`, `1`,
    `utf-8`, `lf`. `Title` followed the `Documents` click (`f010.txt — Kiln`) and stayed after
    closing a non-current document;
  - at `--size 800x320` the four nodes exist, are not `offscreen` and have no rows (Q-5). There the
    `Documents` click did not change `Title` (`f299.txt` stayed): at that size with 300 tabs the
    click did not reach the entry. Not investigated (no second build): a matter for T-058's
    follow-ups, and T-059's criteria do not click `Documents` at that size.

Not checked: a fix for Q-5 A; macOS and Windows; the window; a release build; why the
`Documents` click had no effect at 800x320 with 300 tabs (reported to the director).

Estimate: about 690,000 tokens, below the medium ceiling of 800,000. That is the stage's medium
median of 686,472 (retro-2s1). The task is smaller than T-058 (909,094 with an independent test
author): one panel file, display only, no close or switching logic, and in the implementer-first
arm one fewer agent. The risk is Q-5 A, if ruled, which adds a layout change and a re-run of the
layout-dependent tests.

### Rulings on spec check R-295 (director, D-206)
- **Q-5: B.** That nothing of the bar is painted at `--size 800x320` with 300 tabs is stated as a limit in
  `docs/protocol/ui-status.md` with the measurement; T-083 (draft) takes the layout at small sizes.
- The values `Ln <n>` and `Col <n>` (a bare number could make a merged test's `{"role":"label","value":…}` click
  ambiguous) are accepted; ruling Q-1 A (1-based) stands.
- Before the queue the rebased head goes to `ci/T-059`; `window-linux` and `window-probe (macos-15)` must be green.

### Rulings on review round 1 (director, D-212; `reviews/T-059-r1.md` at `f0ab0260e2a6bc1e886ca3dc87829237349967fe`: approve)
- **k1 (finding 1, spec gap).** A document whose path equals a value the bar is showing (`Ln 1`, `Col 1`, an
  encoding name, a line-ending value) makes the `Documents` target `{"role":"label","value":<path>}` ambiguous.
  Not changed in this task (the review approves; no rework round): T-083 states the limit in `ui-status.md` or
  gives the bar's nodes a role of their own. Noted there.
- **k2 (finding 2, nit).** The limit's paragraph is incomplete, not wrong (a window lower than about 305 points
  hides the bar even with one tab): T-083.
- The two changes to the acceptance file after its first commit (`b935ad9`) are test defects (the reviewer re-ran
  the first version: 7 passed, 2 failed; no assertion weakened): acceptance-test defects of this task = 2.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
- 2026-11-16 **Pre-merge platform run** (ADR-0009 Decision 5; D-212): run 43787215183 of `platforms.yml` on `ci/T-059` at
  `6f32d1093ab530015e7e02b8e8e54a03e669093d` (the branch rebased onto main `51ee7c1`, which holds T-054's profile).
  The two counted jobs are green: `window-linux` success, `window-probe (macos-15)` success.
  `window-probe (windows-2025)` fails at "Build kiln" as on main; the three gate jobs were running or red when read
  (01:41Z) and are not counted (D-202).

### planner
- 2026-11-14 · R-225 · (new) · did: filed as a draft for ADR-0007. · gate attempts: 0 · missing context: T-056's merged shell · next: director.
- 2026-11-15 · R-276 · draft · did: specified the status bar against main `c288f7f` with a prototype (all 110 merged `kiln` tests green); tier medium (item low ⇒ medium), implementer-first arm (odd); 5 criteria, AC-2 on generated text; reads after a change only; depends on T-058 (`PanelView::changes`), so not in flight with it; spec check after T-058 merges. · gate attempts: 0 (scratch builds only) · not run: macOS, Windows, the window · missing context: T-058 as merged · next: director (Q-1 to Q-4).
- 2026-11-16 · R-295 · draft · did: spec check against main `c772117` (T-058 merged): read rule in h3's words, `changes()` pinned, `Ln `/`Col ` prefixes after a target-collision finding, AC-3/AC-4/AC-5 extended, focus and delivery notes, Q-5 (bar unpainted at 800x320 with 300 tabs). Prototype re-run: 158 `kiln` tests green. · gate attempts: 0 · not run: a Q-5 fix (disk), macOS, Windows · missing context: none · next: director (Q-5).
<!-- end planner log -->
### implementer
- 2026-11-16 · R-296 · in_progress · did: implementer-first arm (odd, medium). Criteria in my words: AC-1 for every encoding × line-ending kind (incl. mixed, none) the bar's `Encoding` is `doc.info`'s encoding and `Line ending` its `line_ending` plus ` (mixed)` exactly when mixed, also after `eol.normalize`; AC-2 over 40 seeds × 40 UI caret steps on generated text, `Line`/`Column` are `Ln `/`Col ` + `cursor.get` line/col + 1; AC-3 after a tab click, a `Documents` click, an `already_open` `doc.open`, a refused close and the close of the current document the bar shows the (rule-)current document's values; AC-4 control-only scripts log as `Host`, idle frames add nothing, a `cursor.set` and a script read cause read-only requests only; AC-5 no document, no nodes; a click on the bar does nothing. Rulings D-194 (Q-1..Q-4 A) and D-206 (Q-5 B, `Ln <n>`/`Col <n>`) acknowledged and followed.
  - Step A (`task/T-059-tests`, 284b1fc): `apps/kiln/tests/acceptance_t059.rs`, 9 tests, run on base (`cargo test -p kiln --locked --test acceptance_t059 --no-fail-fast`, 0 passed, 9 failed), each by assertion, message `expected exactly one node labelled `Line` (the status bar), found 0`: AC-1 `ac1_encoding_and_line_ending_follow_doc_info` (seed 1, after the first open); AC-2 `ac2_line_and_column_follow_cursor_get_over_generated_steps` (40 of 40 seeds failed at step 0; the message names seed, step, request, document and alphabet); AC-3 `ac3_switching_shows_the_current_documents_values` ("after the opens"); AC-4 `ac4a_control_requests_only_log_as_host` (the `Host` equality passed, then the framed session's bar check), `ac4b_idle_frames_add_nothing`, `ac4c_a_control_cursor_set_is_read_then_idle`, `ac4d_a_read_only_control_request_then_idle`; AC-5 `ac5_no_document_no_bar` (the fresh-session checks passed, then "with a.txt open"), `ac5_a_click_on_the_bar_does_nothing` ("before the clicks"). AC-2's alphabet: ASCII letters and a digit, space, tab, `é`, `e`+U+0301, U+200D, `ffi`, `漢字`, `😀`, `שלום`, `سلام`, `٣`; breaks LF, CRLF, CR; steps: arrows with none/`ctrl`/`shift`/both, `Home`/`End` (± `shift`), clicks `at` a row and column drawn at random and reduced to the rows the snapshot before shows (re-run when a click was beyond them), typed pieces, `Enter`, `Backspace`.
  - Acceptance file changed after step A, once (b935ad9), for two defects in the tests' own log checks, found at the first run on the scratch merge (7 passed, 2 failed): (1) `ac4c` asserted that the first `cursor.get {"doc":1}` after the second `cursor.set` was the script's last request, but the views read `cursor.get` too (`left: 21`); it now checks that the log's last entry is that request. (2) `ac4d` located the script's `doc.list` by content and found the tabs panel's earlier identical `doc.list` read, so the window included the typed `doc.insert`; it now finds it by position, after the log of the same script cut before the `doc.list`. No assertion was weakened; both tests still fail on base (mutant M0 below). For the director to classify.
  - Step B (`task/T-059`): `status.rs` (27d5f2f): four `Label`s (`selectable(false)`, one row, `accessible_name` `Line`/`Column`/`Encoding`/`Line ending`) in a `ui.horizontal` row of the shell's bottom-up `Ui`; reads `cursor.get` and `doc.info` via `PanelView::read` when `(changes(), current())` differs from the last read's; nothing with no current document; a refused read shows nothing. Two unit tests (`shown_of`). `apps/kiln/tests/status_t059.rs`, 4 tests through the binary: Pinned (g)'s worked examples (`Ln 2`/`Col 6` after the click `at {row:1,col:4}` and `type "z"`; `lf (mixed)`, `utf-8-bom`, `crlf`, `none`, the Hebrew-named file), columns at every valid head of a UTF-16 LE text with a tab, `e`+U+0301, U+200D, an emoji, `ffi`, CJK, Arabic, a CRLF and a CR against a count computed in the test, numeric paths `1` and `7` staying one `Documents` target (Pinned (f)), `Enter`/`ctrl+Z`/`ctrl+S` and an empty file. `docs/protocol/ui-status.md` (6b5685f): S1–S5 copied from Pinned (i) / Q-5 B, the node table, reads, limits.
  - Self-check (seeded bugs in `status.rs`, each built and run against `acceptance_t059` and `status_t059`): M0 `show` draws nothing (= base): all 9 acceptance + 4 own killed; M1 line 0-based: AC-1, AC-2, AC-3, AC-4a–d (7) + 4 own; M2 column 0-based: same 7 + 4 own; M3 no ` (mixed)`: AC-1, AC-4a, AC-4b, AC-4d + 2 own; M4 key on `current()` only (no re-read after edits): AC-1, AC-2, AC-4c, AC-4d + 3 own; M5 read on every frame: AC-4b, AC-4c, AC-4d, AC-5 click; M6 read `doc 0` when none is open: no test (equivalent: the read is refused, refused requests are not logged, nothing is drawn); M6b keep drawing the last values with no document: `ac5_no_document_no_bar`; M7 swap encoding and line ending: AC-1, AC-3, AC-4a, AC-4b, AC-4d + 4 own; M8 read the lowest-id document: AC-1, AC-3, AC-4a, AC-4b + 3 own; M9 a click on the bar sets `Message`: `ac5_a_click_on_the_bar_does_nothing` only; M10 `request` instead of `read` (a change every frame): AC-2, AC-3, AC-4b–d, AC-5 click; M11 column from `head`: AC-2, AC-3, AC-4a–c + 4 own. 12 of 13 killed, 1 equivalent.
  - Evidence for `ui-status.md` (debug build of the scratch merge, `kiln script --root r --log …`, scripts run from outside the repository): S1 — AC-1, AC-2, `status_t059`; selection head — AC-3 (`a.txt` anchor 0, head 7 → `Ln 2`/`Col 3`), AC-4c (anchor 1, head 13); switching and close of the current document — AC-3; S2 — AC-5 and a script `doc.open f000.txt`, `frames 5`, `doc.close {"doc":1}`, `frames 60`: log entries after the close `[]`, the frames `settled: true`; S3 — AC-4a–d, and the log of `doc.open`, `frames 5`, tab click, `focus Editor`, `key ArrowDown`, click `Line`: after `cursor.move` the log reads `doc.info, doc.text, cursor.get, cursor.get, doc.info, doc.list` (the editor's reads, the bar's two, the tabs'); scroll — 300 opens, `frames 5`, then `scroll` `Tabs` down 3, `Documents` down 3, `Tabs` up 1 vs. the same without: all ok, logs equal (306 entries); no file — the same session's root: bytes and mtimes of all 300 files unchanged, logged commands `cursor.get, cursor.move, doc.info, doc.list, doc.open, doc.text`; S4 and focus — AC-5 and a script `focus Editor`, snapshot, click `Encoding`, snapshot, `focus` `Line ending`: focused `['Editor']` before, `[]` after, `focus` refused `not_actionable`; layout ("below … one row tall") — from `shell.rs:81-82` (status drawn first in the bottom-up layout) and each node painting one row, not from a geometric check.
  - Q-5 B measurement (S5): N control opens of `f000.txt`…, `frames 2`, snapshot; the bar's four nodes exist, with values, not `offscreen`, in every case. At `--size 800x320` rows `['Ln 1']`… with N = 1, 5, 10, 20, 22 and no rows with N = 23, 24, 25, 28, 30, 32, 35, 38, 40, 45, 50, 100, 200, 300. At 1280x800 rows with every N measured (1 … 300). Not measured: other sizes or path lengths, a release build, the window.
  - Scratch merge: branch `scratch/T-059-join` in `.worktrees/T-059` (task/T-059 + task/T-059-tests), never committed to either task branch, deleted afterwards. · gate attempts: 2 on the scratch merge (attempt 1 at 260aeb9: red `clippy`, `cloned_ref_to_slice_refs` in my `status_t059.rs:128`, fixed in 4c8ab89; attempt 2 at d566998 = task/T-059 4c8ab89 + task/T-059-tests b935ad9: green in 10m16s, fmt, clippy, layers, ui-commands, hygiene, schema, scorecard (64 items: 44 verified), test 1060 passed 0 failed 0 ignored in 127 suites (`acceptance_t057`, `acceptance_t058`, `tabs_t058` among them), no-skip; fuzz and budgets EMPTY) · not run: macOS, Windows (CI after T-070), the window, a release build; a refused bar read (cannot happen for an open document; covered only by the unit test `a_result_without_a_field_shows_nothing`) · missing context: none · next: director (join; `ci/T-059`).
  - Note (not a defect of this task): the editor and the bar both read `cursor.get` and `doc.info` after every change, as Q-3 A ruled (two extra reads per change).
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
- 2026-11-16T01:40Z · R-300 · tests_joined (main; `in_progress` on the branch) · did: round 1 full review of `975dd48` (implementer-first arm); rulings D-194 (Q-1..Q-4 A) and D-206 (Q-5 B; `Ln <n>`/`Col <n>`) read on main and accepted. `cargo test -p kiln` all targets green (171, merged T-055..T-058 tests unchanged); `284b1fc`'s acceptance file re-run against the code: `ac4c`/`ac4d` fail as logged, so `b935ad9` is two test defects, no weakening (both still fail on base and on mutant R19). 20 mutants of `status.rs`: 18 killed, all by acceptance tests, 2 equivalent (R8, R18), 0 survived. Differential run with my own driver: seeds 1–200 × 40 steps (8,000) at 1280x800, 1001–1050 at 800x320 and 2001–2050 at 300x900 (2,000 each), 0 mismatches; idle seeds 1–40, logs equal; negative controls flagged. Every sentence of `ui-status.md` run against the binary: none wrong. Findings: 1 spec gap (a document path equal to a bar value makes a `Documents` value target ambiguous), 1 nit (Limits: a low window hides the bar with one tab too; for T-083). Verdict approve (`reviews/T-059-r1.md`). · gate attempts: 0 (focused targets only; `cargo xtask hygiene` on the committed state) · not run: macOS, Windows, CI, the window, a release build, the whole gate · missing context: none · next: director (finding 1; `ci/T-059`).
<!-- end reviewer log -->
