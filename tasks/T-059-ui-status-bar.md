---
id: T-059
title: "Show the current document's caret line and column, encoding and line-ending style in a status bar below the editor, re-read only after a change"
tier: medium            # ADR-0002 default (UI behaviour over existing commands; no high criterion); ≥ the claimed item's tier (`ui.status-bar`: low; items ⇒ medium); fixed at specified (METHODOLOGY v1.4 §4)
state: specified            # director only, written with every transition; see METHODOLOGY §3
owner_role: planner
scorecard_items: [ui.status-bar]
depends_on: [T-058]     # `PanelView::changes`, added by T-058; not merged when this spec was written: spec check required
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
| `Line` | the caret's line, 1-based, in decimal (`cursor.get` `line` + 1) | `cursor.get` |
| `Column` | the caret's column, 1-based, in chars (`cursor.get` `col` + 1): a combining mark, a joiner or a tab each count as one char | `cursor.get` |
| `Encoding` | exactly `doc.info`'s `encoding`: `utf-8`, `utf-8-bom`, `utf-16le` or `utf-16be` | `doc.info` |
| `Line ending` | exactly `doc.info`'s `line_ending` (`lf`, `crlf`, `cr`, `none`), followed by ` (mixed)` when `mixed_line_endings` is true | `doc.info` |

- The caret is the selection's head (`cursor.get` `head`).
- With no current document, none of the four nodes exists.
- The values are of the current document (T-058's rule), read after the change that made it
  current.
- Not shown: dirty state (`Title` and the tab show it), selection size, language or syntax mode.
  No command reports a language: the editor colours a document by its path's suffix, which is the
  UI's rule, not a command's result.
- Where the bar sits ("below the editor", `panels/mod.rs`) and its layout are not specified for
  tests. The bar is one row tall.

### UI requests that act on it
None: the bar is a display. It takes no focus, and a click on it does nothing (stated as
behaviour in `ui-status.md`, so the evaluator's inert check (O5) does not report it).

### Control requests the bar sends, and when
- **Reads**: read-only `cursor.get {"doc": <current>}` and `doc.info {"doc": <current>}` in the
  first frame after a change. A change is a successful request of any source other than a read,
  or a change of the current document: `PanelView::changes()` (T-058) or `PanelView::current()`
  differs from what the bar last read for. Both reads are logged.
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

### Rust items this builds on (on main `c288f7f`, plus T-058's method; not used by tests)
```rust
pub(crate) fn show(ui: &mut egui::Ui, view: &mut PanelView<'_>);    // panels/status.rs, kept
impl<'a> PanelView<'a> {
    pub(crate) fn read(&mut self, command: &str, params: serde_json::Value) -> Result<serde_json::Value, (String, String)>;
    pub(crate) fn current(&self) -> Option<u64>;
    pub(crate) fn changes(&self) -> u64;                              // added by T-058
}
```
The shell gives `show` a bottom-up `Ui` (`panels/mod.rs`). The spec check after T-058 merges pins
`changes()` as merged.

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
  - F2: the four nodes each paint exactly their value. No new node carries a value equal to a
    `Documents` path, so no merged `{"value":…}` target changes.
  - L1–L8: L4 the bar takes no focus; L5 none of the labels `Line`, `Column`, `Encoding`,
    `Line ending` is `Path`, `Open`, `Documents` or `Message`, or contains `Pat` or `Ope`; L7; L8
    (settling within 60 frames: measured 3–5 with the bar, E2).
  - T-057's layout-dependent tests (`ac2_…` at 800x320, `ac3_…` at 1280x8192): passed with the bar
    and the tab strip both present (E1). `acceptance_t057.rs` is not in `touches`. A failure is a
    `QUESTION:`.
  - T-058's acceptance tests: not on main yet. The spec check after T-058 merges lists what they
    assume about the space below the editor.
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
    `type "z"` give `cursor.get` line 1, col 5 (head 17, T-056 (g)), and the bar shows `Line` `2`,
    `Column` `6`.
- (h) Evaluator calls for `ui.status-bar` (ADR-0008 (O1)–(O5)): `kiln script` with documents of
  each encoding and line-ending kind (including mixed and none), caret moves by keys and clicks;
  snapshots before and after, the four values against `cursor.get` and `doc.info` in the same
  session; `--log` for the bar's reads (read-only, none on idle frames).
- (i) Sentences for `docs/protocol/ui-status.md`, worded for copying, with their evidence:
  - S1 "The status bar shows, for the current document, the caret's line and column (both
    counted from 1; the column in chars, as `cursor.get` counts it), the encoding as `doc.info`
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
  `Backspace`). After every step `Line` = `cursor.get` `line` + 1 and `Column` = `col` + 1.
  Oracle: `cursor.get` in the same session.
- AC-3 (switching): with two documents open, switching by a tab click (T-058), by a `Documents`
  click and by a control `doc.open` (`already_open`) shows the newly current document's values.
  Oracle: `cursor.get` and `doc.info` of that document.
- AC-4 (reads and idle): (a) a script of control requests only writes `Host::log_script()`;
  (b) 600 frames with a document open add nothing to the log; (c) after a control `cursor.set` on
  the current document, the frames until the UI settles add read-only requests only, and the
  bar's `Line` and `Column` follow the new head. Oracle: log comparison; `cursor.get`.
- AC-5 (no document; display only): a fresh session and a session after the last document is
  closed have none of the four nodes. A `click` on `Line` changes no node's value, adds nothing to
  the log, and settles. Oracle: snapshot, log.

## Out of scope and implementer notes
- Out: dirty state, selection size, a language or syntax mode, the document's length, a clickable
  bar (go-to-line from the bar: T-060's), configuring the bar.
- Do not edit `mod.rs` (T-058 adds `changes()`; if more is needed, `QUESTION:`), `observe.rs`
  (T-066), `script.rs`, `window.rs` (T-068), `ui.md`, `editor.rs`, `shell.rs`, or merged tests.

### Collisions and order
- **T-058**: must merge first (`PanelView::changes`). Not in flight together: T-059's spec builds
  on a method T-058's branch adds.
- **T-066, T-068, T-071, T-054**: no shared path; may be in flight beside T-059.
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

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-14 · R-225 · (new) · did: filed as a draft for ADR-0007. · gate attempts: 0 · missing context: T-056's merged shell · next: director.
- 2026-11-15 · R-276 · draft · did: specified the status bar against main `c288f7f` with a prototype (all 110 merged `kiln` tests green); tier medium (item low ⇒ medium), implementer-first arm (odd); 5 criteria, AC-2 on generated text; reads after a change only; depends on T-058 (`PanelView::changes`), so not in flight with it; spec check after T-058 merges. · gate attempts: 0 (scratch builds only) · not run: macOS, Windows, the window · missing context: T-058 as merged · next: director (Q-1 to Q-4).
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
