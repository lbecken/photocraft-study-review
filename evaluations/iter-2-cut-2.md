---
iteration: 2
run: R-302                # booked to iter-2; cut 2 of the second stage
main_sha: 20f63f499905803ccfe9ee2578964429d8bede09
interfaces: [cli, control, ui-script, window]
confirmed_items: [ui.project-panel, ui.tabs, ui.status-bar, ui.window, ui.editor-view, ui.dispatch-only, cmd.schema, control.errors, ws.path-confinement, cli.run, text.positions, undo.roundtrip, search.literal, syntax.highlight]
rejected_items: []
defects_filed: [T-085, T-083, T-053]   # task numbers assigned by the director (D-215): NEW-1 = T-085, NEW-2 = T-083, NEW-3 = T-053; Q1 = T-086
not_evaluated: []         # no scorecard item left unevaluated; macOS and Windows not evaluated (below)
---

# Evaluation, iteration 2, cut 2 (second stage)

The three `implemented` UI items are confirmed on Linux/X11: `ui.project-panel`, `ui.tabs`,
`ui.status-bar`, each by ADR-0008's O1–O5 through `kiln script`, against `project.tree`/`doc.list`/
`cursor.get`/`doc.info` in the same session, my own naive models, the files on disk, the session log
replayed with `kiln-cli run`, and idle frames. The real window was started under Xvfb with the two
unpacked libraries and driven both by scripts and by real X input (XTest). The regression sample
(`ui.editor-view`, `ui.dispatch-only`, `ui.window`, and eight non-UI items) holds; 0 rejected.
3 findings (NEW-1 … NEW-3), 1 question, inert parameter values named under T-068.

Comparisons (all counts from the `out-*.txt` files named below):
- headless, seeded random sessions against naive models: tabs 120 sessions / 3,600 steps, 0
  mismatches; project panel 200 sessions / 8,000 steps, 0; status bar 270 sessions / 10,400 steps at
  six sizes: 0 mismatches at 640×400, 1024×768, 1280×800 and 3000×2000, while at 300×200 and
  400×300 the bar is never painted (NEW-2). In all: 590 sessions, 22,000 steps;
- real window vs headless: 397 scripts, 73,477 response lines (screens 1280×800, 1920×1080,
  1024×768, 640×400, 400×300), 0 stdout differences, 0 file differences, 1 exit-code difference
  that did not repeat in 40 reruns (Q1).

## Setup
- Clean clone at `20f63f49` in `/home/user/kiln-eval-cut2/clone` (`git rev-parse HEAD` →
  `20f63f499905803ccfe9ee2578964429d8bede09`, `git status` clean). Every directory above it mode
  0755, owner root (`stat`): reachable by other user identities: yes. Identity uid 0. Machine boot
  (`uptime -s`) 2026-11-15 21:11:33; no restart during the run.
- Ledger records left out of the clone: none (the clone holds `tasks/`, `reviews/`, `handoffs/`,
  `metrics/`, `retrospectives/`, `docs/decisions.md`; I did not open them, `tasks/` only listed).
  Fixture data read: none from the corpus.
- No `CARGO_*` profile variable set (`env | grep CARGO` → only `CARGO_HTTP_CAINFO`); the root
  `Cargo.toml` has `[profile.dev]` tables, so built as is: `cargo build -p kiln -p kiln-cli` →
  `Finished dev profile [unoptimized + debuginfo] target(s) in 37.95s` (`work/build.log`);
  `target/debug/kiln` 77,979,312 bytes, `kiln-cli` 31,796,648 bytes. Disk before: 26 GB free. No
  release build was made.
- `cargo xtask scorecard` (uid 0, in the clone): exit 0,
  `PASS  scorecard: 64 items: 44 verified, 3 implemented, 0 failing, 17 planned.` in 2m39s
  (`work/scorecard-run.log`); the file it wrote is byte-identical to the committed
  `docs/scorecard.md` (`cmp` of `work/scorecard-generated.md` and `work/scorecard-committed.md`).
  `target/` afterwards 2.8 GB.
- Real window: `libxcb-xkb1 1.15-1ubuntu2` and `libxkbcommon-x11-0 1.6.0-1build1` fetched with
  `apt-get download` and unpacked with `dpkg-deb -x` into `work/xlibs/root` exactly as the brief
  says; nothing installed. Xvfb, xvfb-run, ImageMagick `convert` are on the machine; real input with
  the cut-1 ctypes XTest client (`drivers/xprobe.py`, copied), screenshots from `Xvfb -fbdir`
  (`drivers/plainwin.sh`, copied). Xvfb itself creates its socket and lock under `/tmp/.X11-unix`
  and `/tmp/.X<n>-lock` (as `xvfb-run` does); after the four `kill -9` runs of `displaykill.sh` the
  lock files of displays 91–94 are left there (I removed nothing outside the work directory).
  Disclosure: early on I wrote two three-line probe scripts (`dl.jsonl`, `di.jsonl`, `doc.list`/
  `doc.info` field checks) into the session's scratch directory outside `work/`; nothing else was
  written outside `work/` and the clone's `target/` (besides Xvfb's own files above and the
  `scorecard` regeneration of `docs/scorecard.md` in the clone, byte-identical).
- Read: the brief; `docs/scorecard.md`; `scorecard/items/ui.{project-panel,tabs,status-bar,window}.toml`;
  `docs/control-protocol.md`; `docs/protocol/ui.md`, `ui-editor.md`, `ui-project.md`, `ui-tabs.md`,
  `ui-status.md`, `project.md`, `eol.md`; `docs/commands.json` (names, `mutating`, result schemas);
  the four help outputs, `kiln-cli commands [--schema|--audit]`; `methodology/templates/evaluation.md`;
  METHODOLOGY §9; ADR-0008; `evaluations/iter-2-cut-1.md` and its drivers (`kl.py`, `xprobe.py`,
  `plainwin.sh`, `wincases.sh`, `displaykill.sh`, `regress.py`, `wincmp.py` read; the first six
  copied to `work/drivers/` and adapted, marked in their text). `AGENTS.md` was shown to me by the
  tool environment when I first read a file of the clone; I did not use it. `tasks/` only listed
  (file names). Corpus fixture data: not used.
- Drivers in `work/drivers/` (helpers `kl.py`, `k2.py`; `work/env.sh` sets `KILN_BIN`, `KILN_CLI`,
  `KILN_WORK`, `KILN_CLONE`, `XLIBS`, `DRV`); every process under `timeout`. Fixtures under
  `work/fx/`, window runs under `work/win/`; outputs `work/out-*.txt`.

## Items

### ui.project-panel
- Claimed: "The project tree panel opens a file on click" (`docs/protocol/ui-project.md` P1–P7).
- What I did:
  - Random (`drivers/panel_fuzz.py <seed> 100 40`, seeds 31 and 32): each session a random tree of
    3–18 entries from a hostile name pool (`a`, `a b`, `a b c`, `a.rs`, `ab`, `a-b`, `B`, `.hidden`,
    `é` as e + U+0301, `x‍y` with U+200D, `ﬁ`, `中文`, `😀`, `שלום`, `1`, `_`, `k.log`), a root
    `.gitignore` `*.log`, a root `target/`, a `.git/`, in half of them a link `up -> ..`, in 30 % a
    FIFO. 40 steps each: clicks on `Project` and on random shown entries with random
    `button: secondary`, `modifiers`, `at`. My model lists each directory with `os.listdir` sorted
    by UTF-8 bytes, skipping `.git`, root `target`, `*.log`, and keeps the expanded set (a collapse
    forgets the descendants, a `Project` collapse forgets all). Checked per step: the shown
    `tree_item`s (label, value, toggled, in order) and the marker; `Project` `toggled`; the `tree`
    present iff expanded; `doc.list` and `Title` after a file click; `Message` after a click on the
    link (`outside_root`) or the FIFO (`not_a_file`); every visible item's row = its value (Hebrew
    names excepted, ui.md limit). Per session: the log's `project.tree`/`doc.open` requests = the
    model's (one `project.tree {}` per root expand, one `{"path":…}` per directory expand, none per
    collapse, one `doc.open` per file click); the log replayed with `kiln-cli run` on a fresh copy →
    same files; the same script with `{"ui":"frames","count":20}` after every action → byte-identical
    log, every idle answer `{"frames":20,"settled":true}`.
  - Edge cases (`drivers/panel_edges.py`, `out-panel-*.txt`): the example of ui-project.md; an
    empty root, a root holding only an ignored `.gitignore`/`x`, a root of only `target/` and
    `.git/`; 32 hostile root names (adds a line break, a 244-char name, RTL `שלום.txt`, `مرحبا.txt`,
    `٣٣ ab.txt`, a name that is only U+200D, `ﬁle.txt`, `😀👍🏽.txt`, `Open`, `Editor`, ` lead`,
    `trail `, `UPPER`/`upper`, `#`, `%41`, `~`, U+00A0, `z̈`) and a click on every file (scrolling
    first when `offscreen`); links (to a file and a directory outside, to a file and a directory
    inside, dangling, `..`) and a FIFO; collapse/expand memory; click variants, `focus`, `scroll`;
    labels equal to shell labels; a 130-level deep tree expanded to the leaf; truncated listings
    (a root with `a/b/` of 50,001 files: root, then `a`, then `a/b` expanded).
  - Changes on disk between an expand and a click (`drivers/panel_race.py`): the script expands,
    saves a sentinel document by control requests and runs 6,000 idle frames; a watcher thread
    changes the tree as soon as the sentinel changes on disk (0.04–0.07 s later); a control
    `project.tree` after the frames shows the change happened before the click.
  - Refusals with the DAC capabilities dropped (`capsh --drop=cap_dac_override,cap_dac_read_search`,
    `drivers/refused.py`).
- What I observed:
  - Random: seed 31 100 sessions / 4,000 steps 0 mismatches; seed 32 100 / 4,000 / 0
    (`out-panel-fuzz-31.txt`, `-32.txt`).
  - Example reproduced: collapsed `Project` `toggled:false`, one row `Project`, no `tree`; after the
    click `.gitignore`, `b.txt`, `dir with space` (`toggled:false`), `src` (`toggled:false`); after
    `src`, `src/a.rs` (value `a.rs`) follows it; `Title` `src/a.rs — Kiln`; log `project.tree {}`,
    `project.tree {"path":"src"}`, `doc.open {"path":"src/a.rs"}`, then the views' reads.
  - Empty roots: the `tree` with no `tree_item`; one `project.tree {}` logged; collapse removes the
    tree.
  - Hostile names: the 32 root items equal `project.tree`'s one-level entries in order; value =
    last segment; rows = value except `new\nline.txt` → rows `new`, `line.txt` (as Limits say), the
    244-char name → one row of 22 `w` (cut at the panel edge, as Limits say), and items `offscreen`
    or partly outside the panel (no rows). After `scroll` 1–2 steps every item was clickable; all 30
    files opened with `doc.open` of their label exactly, `Title` beginning with it, `doc.text` equal
    to the file, `Message` empty.
  - Links: `l_in_file` opened; `l_in_dir` → `not_a_file: cannot open l_in_dir: not a regular file`;
    `l_out_file`, `l_out_dir`, `l_up` → `outside_root: … resolves outside the root`; `l_dangling` →
    `not_found`; `fifo` → `not_a_file` in 0.0 s (no blocking); none logged; links and the FIFO have
    no `toggled`.
  - Collapse: with `a`, `a/b`, `a/b/c` expanded, collapsing `a` shows `a`, `z.txt`; expanding `a`
    again shows `a/b` `toggled:false` (forgotten) and logs a fresh `project.tree {"path":"a"}`;
    collapsing `Project` then expanding shows `a` collapsed and logs `project.tree {}`; 100 idle
    frames `{"frames":100,"settled":true}`.
  - Variants: secondary click, `ctrl+shift` click, `at` col 2 and col 5 (= row length) on files →
    `doc.open`; `at` col 6 → `not_actionable` "`at.col` is beyond the row's length", row 1 →
    `not_actionable`; secondary and `at` on directories expand them; `focus` on a `tree_item`, on
    `Project files` and on `Project` → `not_actionable`; after a click on an entry no node is focused
    and `type` is refused `no focused node takes text input`; `key Delete` changes nothing; `scroll`
    on `Project files` and on an entry answer ok.
  - Labels: with the panel expanded, `{"label":"Open"}`, `Title`, `Message`, `Project`, `Path`,
    `Line`, `Documents` → `ambiguous_target` "2 nodes match the target" (P6); `{"label":"Editor"}`
    before any document is open matched only the file and opened it; with it open, 3 nodes match.
  - Deep: 130 expands (each after `scroll` 100 down) all ok, 131 items shown, the leaf at 131
    segments opened; 133 `project.tree` reads logged; `project.tree` of the 128-segment path lists
    its child.
  - Truncation (`out-panel-3.txt`, `out-panel-trunc-nested.txt`): root expand (read truncated, 50,000
    entries, 0.9 s) shows `0first.txt`, `a` and one label `truncated listing`, not `z.txt`;
    expanding `a` shows `a/b` then two markers (`a`'s, the root's), not `a/zz.txt`; expanding `a/b`
    (19.4 s for the script, the click answered `{"frames":17,"settled":true}`) shows 50,006 items:
    `0first.txt`, `a`, `a/b`, `a/b/f00000` … `a/b/f49999`, then three identical markers (P5).
  - Disk changes (`out-panel-race.txt`): a file removed after the expand → click → `Message`
    `not_found: cannot open v.txt: not found`, the stale item stays; a directory replaced by a file
    → click → `not_a_file: cannot list dd: not a directory`, stays collapsed; with `dd` expanded and
    then replaced, collapsing sends nothing and expanding again → the same `not_a_file`; a file
    replaced by a directory → `not_a_file: cannot open ff: not a regular file`; a file replaced by a
    link outside → `outside_root`; a removed expanded-later directory → `not_found: cannot list the
    project: dd: not found`; a file created after the expand appears after collapse/expand of
    `Project`. No refusal was logged.
  - DAC dropped, no unreadable directory: `noread.txt` click → `io_error: cannot open noread.txt:
    I/O error: Permission denied (os error 13)`; `ro.txt` opened, typed, `ctrl+S` →
    `io_error: cannot save ro.txt: …`, tab still `modified`, file unchanged. With an unreadable
    directory `locked/` anywhere in the tree, the root expand itself is refused
    (`io_error: cannot list the project: locked: … Permission denied`) and the panel cannot be
    expanded at all (as `project.md` says for a directory that cannot be read; see T-080 under
    "Known").
- Verdict: **confirmed** (Linux; macOS/Windows not evaluated). NEW-1 (a click right after a
  layout-changing control request misses) was found on `Project`.
- Inert-feature check: `Project` and every kind of `tree_item` changed something; the
  `truncated listing` label is a display (a click and a secondary click change nothing, `focus` is
  refused); `scroll` `left`/`right` on `Project files` is inert (below).

### ui.tabs
- Claimed: "Open documents appear as tabs; switching and closing tabs dispatch commands"
  (`docs/protocol/ui-tabs.md` T1–T7).
- What I did:
  - Random (`drivers/tabs_fuzz.py <seed> 40 30`, seeds 11, 12, 13): ten files (`.txt`/`.rs`, a
    space in a name, `dir/é.txt`, `dir/中文.txt`, CRLF, CR, mixed, UTF-16LE/BE, a BOM, an empty
    file). 30 steps per session from: open from the panel, control `doc.open` (also of an open
    path), click on a tab, click on a `Documents` entry, click on `Close <path>`, click on `Discard
    <path>`, typed text after `focus` on `Editor` (with and without a selection), arrow/Home/End keys
    with `shift`, a click on `Editor`, `ctrl+S`, `ctrl+Z`, and behind the UI's back control
    `doc.insert`, `doc.save`, `doc.close` (with and without `discard`), `nav.goto-line`. My model
    holds the open documents, the current one (rule 1–3 of ui-editor.md), the documents showing
    `Discard`, the `Message` prefix and the predicted mutating commands; dirty flags and carets for
    the predictions come from the previous step's `doc.list`/`cursor.get` (the script is rebuilt
    and run step by step). Checked per step: tabs in id order with labels = `doc.list` paths,
    `toggled` only on the model's current one, `modified` node after exactly the dirty ones, a
    `Close` button after each, `Discard` exactly where the model has it, tab rows = path,
    `tab_list` present iff a document is open, `Documents` values, `Message` prefix, `Title`, the
    four status-bar values against `cursor.get`/`doc.info`, `Editor` selection = `cursor.get`.
    Per session: mutating commands of the log (`doc.open`, `doc.close`, `doc.save`, `doc.insert`,
    `doc.replace`, `doc.undo`) = the model's, in order and with params; log replay on a fresh copy →
    same files; 30 idle frames after every action → byte-identical log.
  - Edge cases (`drivers/tabs_edges.py`, `out-tabs-edges-*.txt`, `out-tabs-many.txt`): 0, 1, 2
    documents and closing back to 0; the same file opened by the panel, by `Path`/`Open` and by a
    control request; a refused close followed by a switch, an edit, a UI save, an edit, a refused
    close, an undo back to clean, an edit, a refused close and `Discard`; control requests behind
    the UI (refused control close, control undo/redo/save/close with `discard`, control open);
    click variants on tabs, `Close`, `Discard` and `Documents` entries; a click on the current tab;
    paths with a line break, 244 chars, RTL, combining marks, emoji with modifier, a joiner, prefix
    names; 300 open documents (scroll steps 0–39, a sample of 13 tabs clicked after scrolling,
    timing).
  - Refused saves from the UI after the file was replaced by a directory / a link outside / a link
    inside / removed (`drivers/refused.py`, watcher thread as for the panel), then `Close`.
- What I observed:
  - Random: seed 11 40 sessions / 1,200 steps / 6,604 response lines, 0 mismatches; seed 12 40 /
    1,200 / 6,649, 0; seed 13 40 / 1,200 / 6,622, 0 (`out-tabs-fuzz-1{1,2,3}.txt`). (Two trial
    seeds with an earlier generator are not counted: their only mismatches were my generator's.)
  - 0 → 1 → 2 → 0: no `tab_list`, no `Title`, no `Editor`, no bar with no document; after the
    close of the current `b.txt` `a.txt` became current (highest id).
  - Same file twice: panel `a.txt`, `b.txt`, `a.txt` → two tabs, `a.txt` current; `Path` `b.txt` +
    `Open` → `b.txt` current; control `doc.open a.txt` → `already_open:true`, `a.txt` current after a
    frame; five `doc.open` logged, one tab per path.
  - Refused close sequence: `Close a.txt` (dirty) → `Message` `unsaved_changes: document 1 has
    unsaved changes; save it or close with discard`, `Discard a.txt` appears; tab click switches
    with `Discard` and `Message` kept; typed `Y`, `ctrl+S` → `Discard` gone, `Message` cleared,
    file `XYa\n`; typed `Z`, `Close` → `Discard` again; `ctrl+Z` back to clean → `Discard` gone,
    `Message` kept; typed `W` → `Discard` does not come back; `Close` → back; `Discard` → closed,
    `Message` cleared. Mutating log exactly the commands of ui-tabs.md's table.
  - Behind the UI: a control `doc.close` of a dirty document is refused and shows neither
    `Discard` nor a `Message` (ui.md names `Message` only for a refused close from a tab); control
    undo to clean removes `modified`, redo brings it back; control save removes `modified` and
    `Discard`; control `doc.close` with `discard` removes the tab and its `Discard`; control opens do
    not clear `Message` (only UI opens, saves and closes do).
  - Variants: secondary click, `alt+shift` click, `at` on a tab, secondary and `at` on a
    `Documents` entry → switch; secondary on `Close c.txt` → closed; `ctrl` on `Close` of a dirty
    document → refused with `Discard`; `at` on `Discard` → closed; `at` on `Close` → `not_actionable`
    "the target shows no text" (it paints none, as stated); `focus` on a tab or `Close` →
    `not_actionable`. A click on the current tab sends nothing (log unchanged between two control
    markers); a `Documents` click sends the views' reads only.
  - Names: every tab's row equals its path except `new\nline.txt` (two rows, T7) and the 244-char
    path (cut at the window edge, Limits), `modified` after the wide dirty tab paints exactly
    `modified`; every tab click made its document current.
  - 300 documents: 72 tabs visible at scroll 0, scroll steps overlap (step 1 shows `f048`–`f119`
    after `f000`–`f071`), all 300 tabs visible at some step from 0 to 39, the 13 sampled tabs
    clicked after scrolling became current, a click on `f299.txt` without scrolling →
    `not_actionable` "the target is offscreen"; bar painted at every step. 300 control opens + 600
    idle frames: 5.44 s (debug), matching T6's 5.4–5.7 s.
  - Refused UI saves (`out-refused.txt`): replaced by a directory → `not_a_file: cannot save a.txt:
    not a regular file`, tab stays `modified`, `Close` → `Discard`; replaced by a link outside →
    `outside_root: cannot save a.txt: resolves outside the root`, outside file unchanged; replaced
    by a link inside → saved through the link (`in.txt` = `Za\n`), `Close` closes; removed → saved
    (file re-created), `Close` closes. No refused save logged.
- Verdict: **confirmed** (Linux).
- Inert-feature check: tabs, `Close`, `Discard`, `Documents` entries, `scroll` down/up on `Tabs`
  all had effects; `scroll` `left`/`right` on `Tabs` is inert (a 244-char tab is cut at the window
  edge and stays cut).

### ui.status-bar
- Claimed: "The status bar shows line and column, encoding and line-ending style"
  (`docs/protocol/ui-status.md`).
- What I did:
  - Random (`drivers/status_fuzz.py <seed> <n> <steps> [--size]`): two documents per session of
    generated text (from `abc xyz`, `é`, a lone U+0301, U+200D, `ﬁ`, `中文`, `😀`, `👍🏽`, `ש`, `ל`,
    `م`, `٣`, tab, `_`, `(`, `1`; LF, CRLF, CR, mixed, no break, empty), each in a random encoding
    (`utf-8`, `utf-8-bom`, `utf-16le`, `utf-16be` with BOM) and path (`.txt`/`.rs`), opened as start
    paths. Steps: keys (arrows with `shift`/`ctrl`, Home/End, Backspace, Delete, Enter), typed
    text, `ctrl+S`, clicks with `at` (with `shift`), tab switches, and control `nav.goto-line`,
    `eol.normalize`, `doc.insert` of `\r`/`\n`/`\r\n`/`x` at 0. After each step: `snapshot`,
    `cursor.get`, `doc.info`, `doc.text` of the current document. Checked: `Line`/`Column`/
    `Encoding`/`Line ending` values = `Ln line+1`, `Col col+1`, `encoding`, `line_ending` +
    ` (mixed)`; each painted on exactly one row equal to its value; `cursor.get` line/col = my naive
    line split of `doc.text` at the head; `doc.info` line ending and mixed = my naive count (eol.md:
    dominant, ties LF, CRLF, CR); encoding = the one I wrote; `Editor` value/selection; the
    current tab. Per session: log replay → same files; 20 idle frames after every successful
    action → byte-identical log.
  - Edge cases (`drivers/status_edges.py`, `out-status-edges.txt`): no document; eight files
    (mixed, combining/joiner/ligature/tab/emoji, UTF-16LE CRLF, UTF-16BE CR, BOM without break,
    empty, RTL, a tie); the example of ui-status.md; a selection; clicks on every column of
    `e◌́‍ﬁ\t😀x`; a control `cursor.set` read before and after one frame; closing the current
    document; an `already_open` control open; click, `focus`, `scroll` on the bar.
  - The Limits sentence: 800×320 with 1, 5, 21, 22, 23, 24, 30 documents (`out-status-800x320.txt`);
    a sweep of sizes 200×200 … 1280×800 with 1–3 documents (`drivers/smallsize.py`).
- What I observed:
  - Random: seed 21, default size, 100 sessions / 4,000 steps: 0 mismatches (194 clicks with `at`
    refused `not_actionable`, as ui.md states for an empty editor or a row beyond the painted ones;
    my driver first counted 23 of them as mismatches, `out-status-fuzz-21.txt`); seed 22, `--size
    640x400`, 50 / 2,000: 0 (76 such refusals); seed 24, `--size 3000x2000`, 30 / 1,200: 0; seed
    23, `--size 300x200`, 50 / 2,000: the bar's four values always equal `cursor.get`/`doc.info`
    but are never painted (8,000 `rows` mismatches), and in 9 of the 50 sessions 15 actions answered
    `{"frames":60,"settled":false}` (the idle `frames 20` after them also unsettled) → NEW-2; seed 25, `--size
    1024x768`, 20 / 600: 0; seed 26, `--size 400x300`, 20 / 600: values right, never painted (2,400
    `rows` mismatches), 2 unsettled → NEW-2.
  - Example: `ab\nhello world\r\nx\ry`, ArrowDown, 5 × ArrowRight → `Ln 2`, `Col 6`, `utf-8`,
    `lf (mixed)`. Selection (shift+Left, shift+Up): anchor 8, head 2 → `Ln 1`, `Col 3` (the head).
    `e◌́‍ﬁ\t😀x` End → `Col 8`; clicks at columns 0–7 → `Col 1`–`Col 8` = `cursor.get` col + 1.
    UTF-16LE CRLF → `utf-16le`/`crlf`; UTF-16BE CR → `utf-16be`/`cr`; BOM → `utf-8-bom`/`none`; empty →
    `none`; tie `a\r\nb\nc\rd` → `lf (mixed)`. A control `cursor.set` is shown after one frame
    (before it the snapshot still shows the old column, as ui.md says). Closing the current
    document shows the highest id's values; an `already_open` open shows that document's.
  - Click (plain, secondary+`ctrl`, `at`) on bar nodes: nothing changes and nothing is logged;
    `focus` → `not_actionable`; `scroll` on `Line` → `not_actionable` "no scrollable area contains
    the target".
  - 800×320: painted with 1, 5, 21, 22 documents, not painted with 23, 24, 30, values present and
    not `offscreen` — exactly the Limits sentence (T-083).
  - Opened from the panel, `End` pressed (`drivers/sizelimit.py`, `out-sizelimit.txt`): a `.rs` of
    1,048,576 chars → bar `Ln 1 Col 16 utf-8 lf`, keyword/comment/number colours, 8.4 s for the whole
    debug run; 1,048,577 chars → same bar, every run `#dcdcdc`, `syntax.highlight` `too_large`,
    `Message` empty, 0.4 s; a 200,000-char line → `Col 200001`; a `.rs` with syntax errors →
    `Col 15`, coloured. Each bar value = `cursor.get`/`doc.info`, painted.
- Verdict: **confirmed** at the default size and the other sizes ≥ 480×320 I ran (Linux). At small
  sizes the bar is covered or painted into the editor even with one document: NEW-2.
- Inert-feature check: every value followed the registry; the bar is a display (documented).

### ui.window
- Claimed: "The desktop app opens a window showing the open document" (ui.md "The window").
- What I did (ADR-0008 ruling Q-2: a real window under Xvfb with the two libraries):
  - `drivers/wincmp2.py`: the random sessions above (tabs, panel, status) run as
    `xvfb-run -a -s "-screen 0 <W>x<H>x24" kiln --root <copy> <paths> --script s.jsonl` with
    `LD_LIBRARY_PATH` on the libraries, `MESA_SHADER_CACHE_DISABLE=true`, a fresh `HOME`, and as
    `kiln script s.jsonl --root <copy> <paths> [--size]`; compared stdout bytes, exit codes, files.
  - Plain window without `--script` driven by real X input (`drivers/plainwin.sh` + `xprobe.py`,
    screenshots `work/win/p1/o/s*.png`).
  - Failure modes (`drivers/wincases.sh`, `out-wincases.txt`), display killed (`drivers/displaykill.sh`,
    `out-displaykill.txt`), the cache under HOME.
- What I observed:
  - Window vs headless: tabs sessions, 1280×800: 127 scripts, 21,019 response lines, 0 stdout / 0
    file differences, 1 exit-code difference (window 5, headless 3, stdout identical,
    `fx/script-tz11-2-29500-291`); the same script 40 more times in the window: rc 3 each time,
    stdout digest `5177581d` each time, both stderrs empty (`out-rc5.txt`) → Q1. Tabs without
    `--root` (cwd = the copy): 20 scripts, 3,350 lines, 0/0/0. Status sessions 1280×800: 100 scripts,
    23,253 lines, 0/0/0. Panel sessions 1280×800: 60, 7,200, 0/0/0. Status sessions on a 640×400
    screen vs `--size 640x400`: 30, 6,994, 0/0/0; 1024×768 vs `--size 1024x768`: 20, 3,511, 0/0/0;
    400×300 vs `--size 400x300`: 20, 3,497, 0/0/0 (so the window reports NEW-2's overlap too; the
    framebuffer `work/win/small/o/s400.png` shows `Ln 1  Col 1  utf-8  lf` drawn across the row
    `line03`); a 1920×1080 screen (window 1280×800) vs the default: 20, 4,653, 0/0/0. (The 300×200
    comparison found no unique session pairs in `fx/` because I generated seed 23 twice; not run.)
    Window stderr empty and nothing under HOME in every run. Total 397 scripts, 73,477 lines.
  - Plain window (`kiln --root fix src/a.rs`, no `--script`): one window `Kiln`, screenshot shows
    `Path`, `Open`, the `Documents` entry, the tab `src/a.rs` with its close mark, `Title`, the
    `Project` button, the coloured text and the bar `Ln 1 Col 1 utf-8 lf`. Real input (XTest after
    `XSetInputFocus`): click on `Project` → `b.txt` and `▸src` listed; click on `b.txt` → second tab,
    `Documents` gains `b.txt`; click in the text and `Z` → `beeZ`, tab `modified`, title
    `b.txt — modified — Kiln`, bar `Col 5`; click on b.txt's close mark → `Message`
    `unsaved_changes: …` and `Discard b.txt`; click on the `src/a.rs` tab → it is current, `Discard`
    stays; click on `Discard b.txt` → tab gone, `b.txt` on disk still `bee`; click before `let`, `Q`,
    `ctrl+S` → `src/a.rs` on disk `fn main() {\n    Qlet x = 1;\n}\n`, bar `Ln 2 Col 6`; click on
    `src` → expanded with `a.rs`; click on the close mark of `src/a.rs` → no tab, no editor, no bar;
    close request → exit 0, stdout and stderr empty.
  - Failure modes: no `DISPLAY`/`WAYLAND_DISPLAY` (with and without `--script`), `DISPLAY=:77`,
    `DISPLAY=garbage`, only `WAYLAND_DISPLAY`, Xvfb without the two libraries → rc 2, stdout empty,
    one stderr line each (`kiln: no window: Library libxkbcommon-x11.so could not be loaded.`; the
    others the 200-char winit line of cut-1's Q1). Non-UTF-8 argument in six positions/forms → rc
    2, `kiln: argument N is not valid UTF-8` + usage, stdout empty. `--size`, `--memory`, `--log`
    without `script` → rc 2 `option … is only for kiln script`; `--root` twice → rc 2; bad root →
    rc 2 `kiln: bad --root: root \`nowhere\`: not found` (no display needed); unreadable `--script`
    → rc 2 before the root. Window with start paths `missing.rs src/a.rs nothere` → rc 0, `doc.list`
    `src/a.rs` only.
  - Display killed after 3 s (2 plain, 2 with a 240,000-frame script): rc 1, stdout empty, the one
    line `X connection to :9N broken (explicit kill or server shutdown).`, as ui.md says.
  - Cache: a window run with a fresh HOME and no `MESA_SHADER_CACHE_DISABLE` wrote 11 files under
    `HOME/.cache/mesa_shader_cache/`; with it set, 0; `kiln script` under a display, 0. ui.md still
    names the cache but not how to turn it off (cut-1 NEW-3, now in T-053).
- Verdict: **confirmed** (window started in this run; ADR-0008 Q-2 A).
- Inert-feature check: the window's controls are those above; all acted under real input.

### Regression sample

### ui.editor-view
- `drivers/editor_regress.py`, `out-editor-regress.txt`: `c.rs` and `c.txt` with
  `let é́ = "ﬁﬂ";`, `fn ẍ() { 1 }`, `// á ﬁ ‍ z`, `struct Ş;`: rows = the lines; a click with `at`
  on every column (53 per file) → `cursor.get` head = line start + column, `Editor` selection =
  `cursor.get`; 8 × shift+ArrowRight → selection 0..k in both; `ﬁé` typed over the selection →
  `doc.text` and `Editor` value equal, head 3; colours of every non-blank char that is not a mark
  or joiner = the table colour of its `syntax.highlight` kind (`.rs`) or `#dcdcdc` (`.txt`): 0
  mismatches. **Confirmed.**

### ui.dispatch-only
- In the 590 random sessions above (tabs, panel, status): the log replayed with `kiln-cli run` on
  a fresh copy gave the same files in every session; idle frames after every action left every log
  byte-identical; the mutating commands of the tabs sessions were exactly the model's. Refused
  UI requests ran no frame (status sessions: the idle check is only valid after successful
  actions, see the driver's comment). **Confirmed.**

### ui.window (regression)
- The headless-vs-window comparisons above use scripts that drive the project panel, the tabs and
  the status bar. **Confirmed** (above).

### cmd.schema
- `kiln-cli commands --schema` byte-equal to `docs/commands.json`; `--audit` → `audit ok: 30
  commands, all examples validate`; 48/48 examples reproduced through `run` and valid against
  their result schemas (jsonschema 4.26.0), 57/57 refusal examples refuse with their kind
  (`drivers/regress.py 302`, `out-regress.txt`). **Confirmed.**

### control.errors
- 20 hostile lines through `kiln-cli control`: 18 responses for 18 non-blank lines, each with its
  documented code/kind (`io_error` message 200 chars), rc 0; the same lines through `kiln script`
  gave byte-identical stdout (rc 3 vs 0, as documented). **Confirmed.**

### ws.path-confinement
- A link to a file outside → `outside_root`; a path through a linked directory outside →
  `outside_root`; `../…` → `invalid_path`; the outside file unchanged. Also through the panel and the
  UI save above. **Confirmed.**

### cli.run
- The worked example of control-protocol.md: rc 3, stdout byte-equal to the document, file
  `hello world\n`, log byte-equal. **Confirmed.**

### text.positions
- `doc.position {char: i}` for i = 0…10 of `a\r\nbé😀\rc\n` against my model: 0 mismatches; the
  offset between `\r\n` and the one past the end refused. **Confirmed.**

### undo.roundtrip
- 30 seeded sessions of up to 25 random inserts/deletes with cursor moves, 40 undos → original, 40
  redos → final: 0 mismatches. **Confirmed.**

### search.literal
- 40 random texts × 8 literal queries against Python's non-overlapping `find`: 320 cases, 0
  mismatches. **Confirmed.**

### syntax.highlight
- The published example reproduced; a second file's spans cover it exactly once in order; the
  editor colours above agreed with it. **Confirmed.**

### kiln-cli control vs kiln script
- 301 random control requests (79 refused): stdout byte-identical, files identical; and the 18
  hostile lines above.

## Wired but inert
- `scroll` with `direction` `left`/`right` on `Tabs` and on `Project files` (`out-inert.txt`):
  answered `{"frames":3,"settled":true}`, nothing changed, although a 244-char tab and a 244-char
  entry are cut at the edge. Same as cut 1 for `Editor` and `Documents`; covered by T-068 (UI
  requests that do nothing).
- `click` on a status-bar node, on `Title`, on the `truncated listing` marker: nothing; these are
  displays, documented as such (ui-status.md "None. The bar is a display"; ui-project.md for the
  marker by role `label`). Listed, not filed.
- `click` on `Project` right after a control request that changed the layout: `ok:true`, nothing
  happens → NEW-1 (not an inert control: the same click one frame later works).
- Every other control the snapshot showed had an effect: `Project`, every `tree_item` kind, every
  tab, `Close`, `Discard`, `Documents` entries, `scroll` up/down on `Tabs` and `Project files`.
  Every documented refusal I reached occurred (`not_actionable` for `at` beyond a row or rows, for an
  offscreen tab or entry, for `focus` on non-inputs, for `scroll` outside a scroll area;
  `ambiguous_target`; `no_target`; the panel's `not_found`, `not_a_file`, `outside_root`,
  `io_error`; the tabs' `unsaved_changes`), and every documented limit I measured held (T6 timing,
  the 800×320 counts, the line-break and wide-name rows).

## Defects filed

| Task | Item | Summary | Suspected origin task |
|---|---|---|---|
| T-085 (NEW-1) | ui.project-panel (ui.md UI requests) | a `click` right after a control request that moves the layout lands where the target was in the previous frame: `ok:true`, nothing happens | T-055 (runner), ui.md |
| T-083 (NEW-2) | ui.status-bar, ui.editor-view | at `--size` 300×300 / 400×300 the bar's text is painted inside `Editor` and the bar paints nothing; at 200×200 and 300×200 `Editor` is offscreen and actions can run 60 frames unsettled — with 1 document | T-059 (layout), possibly within T-083 |
| T-053 (NEW-3) | ui.tabs / ui.project-panel (ui.md "The shell") | ui.md says when `Message` gets a kind and when it is cleared more narrowly than the behaviour and the module documents | T-057, T-058 (ui.md not updated) |

### NEW-1 — a click right after a layout-changing control request misses
- Reproduction (`drivers/tabs_edges.py stale`, `fx/pd1`): root with `a.txt`, `b.txt`; script
  `{"id":"o","command":"doc.open","params":{"path":"a.txt"}}`,
  `{"id":"p","ui":"click","target":{"role":"button","label":"Project"}}`, `{"id":"s","ui":"snapshot"}`,
  then the same click and snapshot again. Observed (`out-tabs-stale.txt`): `p` →
  `{"frames":4,"settled":true}`, `Project` still `toggled:false`, no `project.tree` in the log; the
  second click → `toggled:true`, log `project.tree {}`. With `{"ui":"frames","count":1}` between the
  open and the click, the first click expands (`toggled: True`). The same happened in my first
  race script (an open of `sentinel` then a click on `Project`). A tab click right after a control
  `doc.open` still hit its tab in my runs (the strip did not move under it).
- Expected: ui.md "`click` … a pointer press and release at the target, as a user's click: at the
  centre of its visible part"; "The nodes are those of the last frame's snapshot"; "what a control
  request changed is shown from the next frame on". The press is placed from the previous frame's
  layout and delivered in a frame whose layout the control request changed (the tab strip and bar
  appear above `Project`), so it hits something else or nothing, and the answer gives no sign. A
  user never sees that stale layout. Either the runner places the press in the frame it is
  delivered in, or ui.md states that a click right after a control request may miss and that a
  frame should be run first.

### NEW-2 — small sizes: the bar painted into the editor, the editor offscreen
- Reproduction (`drivers/smallsize.py`, `out-smallsize.txt`): `a.txt` of 40 lines `line00`…,
  `kiln script s.jsonl --size 400x300 a.txt` with `{"id":"s","ui":"snapshot"}`: `Editor` rows end
  with `[{"color":"#dcdcdc","text":"line03"}]`, `[{"color":"#8c8c8c","text":" 1utf-8lf"}]`; the four
  bar nodes have values `Ln 1`, `Col 1`, `utf-8`, `lf`, are not `offscreen`, and have no `rows`.
  Same at 300×300, with 1, 2 and 3 documents. At 200×200 and 300×200 `Editor` is `offscreen` (no
  rows; a click on it would be refused), the bar is not painted, with 1, 2 or 3 documents; at
  480×320 and larger everything is painted in place. In `status_fuzz.py 23 50 40 --size 300x200`
  the bar was never painted in 2,000 steps, and 15 actions (`Enter`, `focus`, a tab click) answered
  `{"frames":60,"settled":false}` (in 9 of 50 sessions); later idle frames settled (600 more frames
  after one of them: `settled:true`).
- In the real window on a 400×300 screen the same overlap is drawn (`work/win/small/o/s400.png`:
  the bar's `Ln 1 Col 1 utf-8 lf` across the editor row `line03`), and `kiln --script` printed the
  same bytes as `kiln script --size 400x300` for 20 scripts.
- Expected: ui-status.md "four nodes of role `label` … each … a `value` equal to the text it paints
  on one row", "at the bottom of the document area, below … the editor; it is one row tall";
  ui-editor.md "the runs' texts of a row together equal the line's chars"; ui.md `--size` "integers
  200 to 8192 each". The only stated limit is for many tabs (800×320 from 23 documents, T-083). The
  bar text attributed to `Editor` also makes the editor's rows wrong. If T-083 already covers every
  small size, this is evidence for it rather than a new task (director).

### NEW-3 — ui.md's sentences on `Message` are narrower than the behaviour
- Observed: `Message` begins with a kind after a refused UI save (`io_error: cannot save ro.txt…`,
  `not_a_file: cannot save a.txt…`, `outside_root: cannot save a.txt…`) and after a refused panel
  expand (`not_a_file: cannot list dd: not a directory`, `io_error: cannot list the project: …`,
  `not_found: cannot list the project: dd: not found`); it is cleared by a successful UI save and by
  a successful close from a tab, and is not cleared by a control `doc.open`.
- Expected (as written): ui.md "The shell": "`Message` whose `value` is empty or absent, or, after a
  refused open by the UI and after a refused close from a tab …, begins with the refusal's `kind`";
  "A successful open clears `Message`". ui-editor.md, ui-project.md and ui-tabs.md each state their
  own cases (all observed as they state them), so the behaviour follows the module documents; the
  shell's sentence was not widened when T-057 and T-058 added cases. Low impact.

### Known and already filed (observed, not filed again)
- T-080 (`project.tree` reads one level per request): the root expand reads the whole tree, so one
  unreadable directory anywhere (`locked/`, DAC dropped) refuses the root expand and the panel
  cannot be opened at all; a 50,001-file subdirectory truncates the root listing.
- T-081 (every shown entry laid out on every frame): the nested truncation case with 50,000 shown
  entries (timing in "Items").
- T-082 (no keyboard focus after a click on a tab/close/`Documents`): not separately measured: my
  scripts `focus` `Editor` before typing; after a click on a panel entry `type` was refused
  `no focused node takes text input` (documented: a click outside the editor leaves it unfocused).
- T-083 (small window with many tabs): the 800×320 counts reproduced exactly (22 painted, 23 not).
- T-068 (UI requests that do nothing): `scroll` `left`/`right` above.
- T-053 (inexact sentences of "The window"; cut-1 NEW-2/NEW-3/NEW-4): ui-editor.md still says "the
  lines consecutive" (line 34); ui.md still names the shader cache without saying how to turn it off.
- T-066 (rows of right-to-left text): RTL names in the panel and tabs reported rows as stated; not
  re-measured.
- T-045, T-048, T-025: not exercised in this run.

## Not evaluated
- macOS and Windows: not evaluated by this run (Linux with X11 only, as the brief says); the CI
  platform jobs are their evidence. Every item above is Linux only.
- Not run, and why: a release build (no check needed it; ui.md's "debug and release builds"
  sentence was checked on debug only); an ordinary non-root identity (permission cases were run as
  uid 0 with `CAP_DAC_OVERRIDE`/`CAP_DAC_READ_SEARCH` dropped); keyboard navigation, IME, drag,
  double click (not offered or not specified).

## Questions for the director
- **Q1 (ui.window):** one window run of 127 (`fx/script-tz11-2-29500-291/s.jsonl`, 1280×800)
  exited 5 where `kiln script` exits 3, with byte-identical stdout, identical files and empty
  stderr; 40 reruns of the same script exited 3. Exit 5 is not in ui.md's table. I could not tell
  whether `kiln`, the window system or `xvfb-run` produced it (my driver kept the window's stderr,
  empty, but not `xvfb-run`'s). Is a one-off of this kind to be filed?
