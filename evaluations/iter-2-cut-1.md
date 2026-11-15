---
iteration: 2
run: R-253                # booked to iter-2; cut 1 of the second stage
main_sha: a3e41e436be4c624fd5c80461645b1815b9460d2
interfaces: [cli, control, ui-script, window]
confirmed_items: [ui.editor-view, ui.dispatch-only, ui.window, cmd.schema, control.errors, ws.path-confinement, cli.run, text.positions, undo.roundtrip, search.literal, syntax.highlight]
rejected_items: []
defects_filed: [T-066, T-053, T-068]
not_evaluated: []
---

# Evaluation, iteration 2, cut 1 (second stage)

The three `implemented` UI items are confirmed: `ui.editor-view` (with NEW-1, a defect of the
snapshot's colour report next to combining marks; the window paints those rows correctly),
`ui.dispatch-only`, and `ui.window` (a real window was started under Xvfb with the two unpacked
libraries, driven by scripts and by real X input). The eight regression checks and the
`kiln-cli control` / `kiln script` byte check hold. 0 rejected. 4 findings (NEW-1 … NEW-4), 4
questions. `cargo xtask scorecard` passes in this clone. Comparisons: 18,400 scripted inputs in 460
seeded random sessions against a naive model and a `kiln-cli control` oracle (528 colour
mismatches, all in seed 5, all NEW-1; 0 others); 220 scripts run in the real window and headless,
46,360 response lines, 0 differences.

## Setup
- Clean clone at `a3e41e43` in `/home/user/kiln-eval-cut1/clone` (`git rev-parse HEAD` →
  `a3e41e436be4c624fd5c80461645b1815b9460d2`). Every directory above it is mode 0755, owner root
  (`stat`): reachable by other user identities: yes. Identity: uid 0. Machine boot
  (`uptime -s`): 2026-11-14 18:15:21; no restart during the run.
- `cargo build -p kiln -p kiln-cli` (debug): `Finished dev profile [unoptimized + debuginfo]
  target(s) in 59.66s` (`work/build.log`); `target/debug/kiln` 307 MB, `kiln-cli` 111 MB. No
  release build was made.
- `cargo xtask scorecard` (uid 0, in the clone): exit 0,
  `PASS  scorecard: 64 items: 41 verified, 3 implemented, 0 failing, 20 planned.`
  (`work/scorecard-run.log`). The file it wrote is byte-identical to the committed
  `docs/scorecard.md` (`cmp`; copies `work/scorecard-generated.md`, `work/scorecard-committed.md`).
  `target/` afterwards 9.0 GB (the gate's test binaries).
- Real window: `libxcb-xkb1 1.15-1ubuntu2` and `libxkbcommon-x11-0 1.6.0-1build1` fetched with
  `apt-get download` and unpacked with `dpkg-deb -x` into `work/xlibs/root` exactly as the brief
  says; nothing installed. Xvfb and xvfb-run are on the machine. Real input through XTest with my
  own ctypes client (`drivers/xprobe.py`, libX11 + libXtst already on the machine); screenshots
  from `Xvfb -fbdir` via `convert xwd:…` (`drivers/plainwin.sh`).
- Read: brief; `docs/scorecard.md`; the three item TOMLs; `docs/control-protocol.md`;
  `docs/protocol/ui.md`, `ui-editor.md`, `cursor.md`, `eol.md`, `doc-positions.md` (part);
  `docs/commands.json`; `kiln --help`, `kiln script --help`, `kiln-cli --help`,
  `kiln-cli commands --schema|--audit`; `methodology/templates/evaluation.md`; METHODOLOGY §9;
  ADR-0008; `evaluations/iter-2.md` and `evaluations/iter-2-drivers/examples2.py`. Fixture data:
  none from the corpus (not needed). **Disclosure:** a `grep -rn -i "mesa\|shader" docs/` I ran to
  find where the shader cache is documented also matched one line of `docs/decisions.md` (forbidden
  to me) and printed its first 300 characters (a D-165 row about the T-065 review). I did not use
  it for anything below.
- Drivers in `work/drivers/` (helpers `kl.py`; `work/env.sh` sets `KILN_BIN`, `KILN_CLI`,
  `KILN_WORK`, `KILN_CLONE`; window drivers also take `XLIBS`, `DRV`, `OUT`, `BASE`); every process
  under `timeout`. Fixtures under `work/fx/` and `work/win/`; outputs `work/out-*.txt`.

## Items

### ui.editor-view
- Claimed: "The editor view renders text with syntax colours, a cursor and scrolling"
  (`docs/protocol/ui-editor.md`).
- What I did:
  - O1/O2/O3, random: `drivers/fuzz.py <seed> <sessions> 40 <alphabet>`. Each session: a random
    document (LF, CRLF, CR or mixed endings, with or without a final break, empty, up to 20 lines,
    5 % with a 170- or 400-char line), path `a.rs` or `b.txt`; 40 random inputs from every row of the
    input table (typed text with and without a selection, `Enter`, `Backspace`, `Delete`, arrows
    with `ctrl`/`shift` as the table allows, `Home`/`End` ±`shift`, `click` with `at` ±`shift`,
    `ctrl+Z`, `ctrl+shift+Z`, `ctrl+Y`, `ctrl+S`), sent as UI requests after `focus` on `Editor`.
    After each input: `snapshot`, `doc.text`, `cursor.get`, `doc.info`, `syntax.highlight`. My naive
    model (line splitting, char/word/line movement, the mapping rule of cursor.md, the `Enter` rule)
    predicts the dispatched commands, text and selection; the predicted commands are run step by step
    in `kiln-cli control` on a copy (oracle; undo/redo/save state taken from it). Checked per input:
    UI `doc.text`/`cursor.get` = oracle; `Editor` `value` = `doc.text`, `selection` = `cursor.get`,
    `focused`; `Title` = path + ` — Kiln` or ` — modified — Kiln` per `doc.info.dirty`; rows = the
    non-empty lines, each a prefix of its line (equal when short); every non-blank painted char's
    colour = the table colour of its `syntax.highlight` kind (`.rs`) or `#dcdcdc` (`.txt`), blanks by
    the neighbour rule.
  - Edge cases (`drivers/edges.py`, `crjoin.py`, `combcol.py`, `scrollbar.py`, `misc.py`,
    `inert.py`): start paths (several, missing, a directory, `../x`, absolute, empty, `d/`, twice,
    `--memory`); the current-document rule through opens and closes; `--size` 17 values; a `.rs`
    of 1,048,576 and 1,048,577 chars; empty, `\n`, `\r\n`, `\r`, blanks-only and tabs-only
    documents; `Enter` in lf/crlf/cr/mixed/tie/none documents and the join case; 8 titles; plain
    paths (`a.txt`, `a.rs.bak`, `Makefile`, `a.rs~`, `rs`, `a.RS`, `dir.rs/b.txt`, `.rs`); a file
    with syntax errors and NUL/DEL chars; clicks on every column of a row with combining marks,
    CJK, emoji with modifier, ZWJ, `ﬁ`, tab; 200-line document scrolled by 3, 100, −1 steps, right,
    then 60 `ArrowDown`; keys outside the table; 150 open documents.
- What I observed:
  - Random: seed 1 ascii 4,000 inputs 0 mismatches; seed 2 ascii 4,000 / 0; seed 3 uni (adds `é
    中 😀 \t ß Ω`) 4,000 / 0; seed 4 uni 4,000 / 0; seed 5 comb (adds combining marks, no clicks)
    2,400 inputs, 528 mismatches, all `colour`, in 8 of 60 sessions (`out-fuzz-5-comb.txt`). With the
    colour check skipped for rows that begin with a combining mark (`drivers/fuzz_comb_filter.py 5
    60 40`): 10 colour mismatches remain in 2 sessions, on rows that contain a combining mark after
    a digit, a space or a `"` (`1́_fn … let ́xb1.5z…`, `ß"s"́1aΩ`) → NEW-1. Total 18,400 inputs, 460
    sessions; every other check 0 mismatches. A first trial (seed 1, 30 sessions) showed mismatches
    that were my generator's: it sent `ctrl+ArrowUp`, which the table leaves unspecified (the UI
    dispatched nothing for it); removed from the generator.
  - The ui-editor.md example reproduced: first row
    `[{"color":"#569cd6","text":"fn "},{"color":"#dcdcaa","text":"main"},{"color":"#dcdcdc","text":"() {"}]`,
    `doc.text` `fn main() {\n    ylet x = 1;\n}\n`, cursor 17/17 line 1 col 5, log
    `doc.open`, reads, `cursor.set {anchor:16,doc:1,head:16}`, `doc.insert {offset:16,text:"y"}`.
  - Current document: opens of `b.txt`, `a.rs` (doc 3), close 3 → `Title` `b.txt — Kiln` (highest
    id), close 1 → `b.txt`, close 2 → no `Title`, no `Editor`, `Documents` empty.
  - Start paths `a.rs missing.rs b.txt` → `Documents` `[a.rs, b.txt]`, `Message` empty, log only the
    two `doc.open`s; `a.rs missing.rs` → `Message` `not_found: cannot open missing.rs: not found`;
    `d`, `../x`, `/etc/passwd`, `a.rs/x`, `""`, `d/` → `not_a_file` / `invalid_path` / `not_found`
    messages, rc 0, log empty; `--memory a.rs` → `not_found`.
  - `--size`: 200x200 and 8192x8192 run; 199x800, 200x199, 8193x200, `1280X800`, `0x0`, `abc`,
    `-1x5`, `+1280x800`, `1280x800x`, ` 1280x800`, `1280x`, `x800`, `1280×800`,
    `99999999999999999999x200` → rc 2, `kiln: bad --size `…`: expected <w>x<h>, integers from 200 to
    8192`, nothing on stdout; `01280x800` accepted; `--size` twice → `option `--size` given twice`.
    At 1280x800 a row shows 161 chars and 34 rows; 200x200 24 chars, 3 rows.
  - Size limit: 1,048,576 chars `.rs` → coloured rows (keyword/function/plain), 8.96 s for the
    whole debug run; 1,048,577 → every run `#dcdcdc`, `syntax.highlight` `too_large`, `Message`
    empty, rc 3 (my own refused `syntax.highlight`).
  - Line endings: `Enter` inserted `\n` (lf, tie lf/crlf, none), `\r\n` (crlf, mixed with crlf
    dominant, tie cr/crlf), `\r` (cr); caret at the start of the new line. Join case `a\rb\rc\n`, caret
    5: `doc.insert {offset:5,text:"\r"}` then `cursor.set {anchor:7}`, text `a\rb\rc\r\n`, caret 7
    (line 3 col 0), `Message` empty; with a selection 5..6 the same through `doc.replace`.
  - Empty documents: `Editor` has no `rows`; `Enter`, `é`, `Backspace`×3, `Delete` dispatched
    `doc.insert "\n"`, `doc.insert "é"`, `doc.delete 1..2`, `doc.delete 0..1`, nothing for the
    third `Backspace` at 0 and nothing for `Delete` at the end. Blank-only row reported `#dcdcdc`.
  - Titles: `x — modified — Kiln.rs` → `x — modified — Kiln.rs — Kiln`, after typing
    `… — modified — Kiln`; plain colours for every non-`.rs` path listed; `.rs` itself coloured.
  - Syntax-error file: 3 rows, 0 colour mismatches against `syntax.highlight`.
  - Clicks: on every column 0–26 of `e◌́a◌̈x 中文字 😀👍🏽 ab‍cd ﬁ fi\tz` the head went to that column (0
    differences). Secondary click on `Editor`: nothing (no focus, no `cursor.set`); `ctrl`/`alt`
    click: like a plain click.
  - Scroll (200 lines): rows 0–33; 3 steps down → 20–53; 100 down → 167–199; 1 up → 160–193; right →
    unchanged; text and selection unchanged throughout; 60 `ArrowDown` → caret line 60, rows 27–60
    (caret line in view); settle counts 20, 20, 8, 3 frames. `PageDown` changed nothing (unspecified).
  - Keys outside the table (`Tab`, `Escape`, `PageUp`, `PageDown`, `F1`, `Space`, `A`, `ctrl+A/C/V/X/Q/W`,
    `alt+Z`, `ctrl+Home/End/ArrowUp/Backspace/Delete`, `ctrl+Enter`, `alt+ArrowRight`): no command, text
    unchanged; `shift+Enter` inserted a line ending like `Enter`; `command+Z` / `command+S` = `ctrl`.
  - Focus: no node focused in a fresh session or after an open; `focus`/`click` on `Editor` focus it.
  - Real window (same UI): screenshot `work/win/p1/s0.png` shows `src/a.rs` with `fn`/`let` painted
    `#569cd6`, `main` `#dcdcaa`, `1` `#b5cea8` (most frequent non-background pixel of each word).
- Verdict: **confirmed**, with NEW-1 filed. NEW-1 is a wrong colour *report* of the snapshot in
  rows that hold a combining mark without a preceding letter; the same row in the real window is
  painted with the right colours (`work/win/comb/zoom.png`: `◌́let x = 1;` with `let` blue, `1`
  green, `;` grey). If the director counts the snapshot's report as the behaviour, this item is
  rejected on NEW-1 instead.
- Inert-feature check: every input of the table changed text, selection, file or view as stated;
  see "Wired but inert" for `scroll` `left`/`right`.

### ui.dispatch-only
- Claimed: "The UI changes state only by dispatching registry commands" (ADR-0008 decision 5; ui.md;
  ui-editor.md "Frames, reads and the log").
- What I did: in each of the 460 random sessions above: (a) the mutating commands of the session's
  `--log` (by `mutating` in `docs/commands.json`) compared with the commands the input table
  predicts, in order; (b) the log replayed with `kiln-cli run` on a fresh copy of the initial files:
  exit code, files, and the replay's own log; (c) files of the UI session vs the oracle session;
  (d) the same script with `{"ui":"frames","count":30}` after every input: log byte-identical,
  each answer `{"frames":30,"settled":true}`, same files. Plus `edges.py settle`, `refuse`, `shell`,
  a control-only script, and refusals.
- What I observed:
  - (a)–(d): 0 mismatches in 460 sessions (18,400 inputs; 460 replays rc 0, files equal, replay
    log byte-identical to the UI log; 18,400 idle `frames` answers as stated). The only difference
    from my prediction: the UI writes `cursor.move` with an explicit `"extend":false` where the table
    names no `extend`; same effect (I normalised it).
  - After `doc.list` (successful): `frames 1` → `settled:false`, next → `true`; after a refused
    `doc.open` → `settled:true` at once; after `doc.open b.rs` the snapshot still showed `a.rs` until
    the next frame. Snapshot frame totals 3 → 4 → 7 → 9 → 609 matched the sum of reported frames.
  - 41 refused lines (40 UI refusals of all five kinds, 1 `bad_id`): no frame run (snapshot frames
    3 before and after), nothing logged, `doc.text` unchanged.
  - A script of control requests only, no start paths: stdout and `--log` byte-identical to
    `kiln-cli run` on the same script.
  - UI open of `""`, `src/missing.rs`, `d` refused with `Message` `invalid_path…`, `not_found…`,
    `not_a_file…`, not logged; successful UI open logged as `doc.open {"path":"src/a.rs"}`.
  - File access (strace, `work/win/strace/trace2.txt`): `kiln script` opened only the script, the
    root, `a.rs`, `.kiln-tmp-0` and the log; nothing under HOME.
- Verdict: **confirmed**.
- Inert-feature check: n/a (no controls of its own).

### ui.window
- Claimed: "The desktop app opens a window showing the open document" (ui.md "The window").
- What I did (ADR-0008 ruling Q-2: real window under Xvfb with the two libraries):
  - `drivers/wincmp.py`: fuzz scripts (with saves) run as
    `xvfb-run -a -s "-screen 0 <W>x<H>x24" kiln --root <copy> <path> --script s.jsonl` with
    `LD_LIBRARY_PATH` on the libraries, `MESA_SHADER_CACHE_DISABLE=true`, fresh `HOME`, and as
    `kiln script s.jsonl --root <copy> <path> [--size]`; compared stdout bytes, exit codes, files.
  - `drivers/misc.py` §5: start paths `a.rs missing.rs d b.txt d/c.rs`, `a.rs missing.rs`, none,
    with options before and after the paths. `drivers/plainwin.sh` + `xprobe.py`: plain window,
    XTest focus/click/type/keys/wheel, `WM_DELETE_WINDOW`; `winreplace.sh`: file replaced after
    opening; `wincases.sh`: failure modes, cache; `displaykill.sh`; `win/inp`: real input while a
    window script runs; `win/geo`: screen sizes; `strace` of a window run.
- What I observed:
  - Byte comparisons: 1280x800 window vs default `kiln script`: 100 scripts (`fz2-`), 23,060
    response lines, 0 stdout / 0 exit-code / 0 file differences; same without `--root` (cwd = the
    copy): 40 scripts, 7,030 lines, 0/0/0; 1024x768 screen vs `--size 1024x768`: 40, 7,030, 0/0/0;
    800x600 vs `--size 800x600`: 40, 9,240, 0/0/0. 800x600 vs the default size: 10 scripts, 2 stdout
    differ (as ui.md says), 0 rc / 0 file differences. Start-path scripts: window rc 3 = headless rc
    3, stdout equal, files equal, also with options after the paths. 12,000-frame script: 131.35 s
    in the window vs 8.57 s headless, stdout identical. Window stderr empty in all; nothing in HOME.
  - Plain window: one top-level window `Kiln` 1280×800 (XQueryTree); screenshot shows `Path`,
    `Open`, `Documents` entry `src/a.rs`, title `src/a.rs — Kiln` and the coloured text.
    Real input: after `XSetInputFocus` (no window manager under Xvfb), XTest click at the left edge
    of `let` put the caret there, `y` was inserted, title became `src/a.rs — modified — Kiln`,
    `ctrl+S` wrote `fn main() {\n    ylet x = 1;\n}\n` and the title lost `modified`; `ctrl+Q`
    did not close; close request → exit 0, stdout/stderr empty. Typed but unsaved `ZZ` then close →
    file unchanged `abc\n`, exit 0. Mouse wheel (button 5 ×5) scrolled the editor by 27 lines.
    Without `XSetInputFocus` the typed key did not arrive (Q4).
  - Replaced after opening, then `ctrl+S` in the window: by a directory → `Message`
    `not_a_file: cannot save a.txt: not a regular file`, title still modified; by a link to
    `/etc/hostname` → `outside_root: cannot save a.txt: resolves outside the root`, `/etc/hostname`
    unchanged (`vm\n`); by a link to `b.txt` inside → `b.txt` replaced with the text, link kept;
    removed → `a.txt` created again.
  - Real input during `--script` (12 × 600 frames; XTest focus, click, `ZZZ`, `ctrl+S`, `Return`,
    click on `Open`, click, `QQ`): stdout byte-identical to headless, file unchanged.
  - Failure modes (`out-wincases.txt`): no `DISPLAY`/`WAYLAND_DISPLAY` (with and without `--script`),
    `DISPLAY=:77` (nothing there), `DISPLAY=garbage`, only `WAYLAND_DISPLAY`, Xvfb without the two
    libraries → each rc 2, stdout empty, exactly one stderr line, e.g.
    `kiln: no window: Library libxkbcommon-x11.so could not be loaded.` (see Q1 for the others).
    Non-UTF-8 argument in 6 positions/forms → rc 2, `kiln: argument N is not valid UTF-8` + usage,
    stdout empty. Bad root before the window system; unreadable `--script` before the root.
  - Display killed after 3 s (6 runs: 2 plain, 4 with a 240,000-frame script): each rc 1, stdout
    empty, one stderr line `X connection to :7N broken (explicit kill or server shutdown).` (no
    panic message in these runs).
  - Screen sizes: window 800×600, 1024×768, 640×480, 300×200, 160×120 on screens of those sizes;
    1280×800 on 1920×1080. On 160×120 see NEW-4.
  - Files outside the root: with a fresh HOME and no `MESA_SHADER_CACHE_DISABLE`, 11 files under
    `HOME/.cache/mesa_shader_cache/` (strace shows nothing else written outside the root); with
    `MESA_SHADER_CACHE_DISABLE=true`: 0 files; `kiln script` under a display: 0 files (NEW-3).
- Verdict: **confirmed** (window started in this run; ADR-0008 Q-2 A).
- Inert-feature check: the window's own controls are the shell's and the editor's (above).

### Regression sample (items verified before this cut; `drivers/regress.py 7`, `out-regress.txt`)

### cmd.schema
- `kiln-cli commands --schema` byte-equal to `docs/commands.json`; `--audit` → `audit ok: 30
  commands, all examples validate`; 48/48 examples reproduced through `run` and valid against
  their result schemas (jsonschema 4.26.0), 57/57 refusal examples refuse with their kind.
  **Confirmed.**

### control.errors
- 20 hostile lines through `kiln-cli control` (bad ids `1.5`, `true`, 2^64; not JSON; `[1]`; `{}`;
  numeric command; unknown key; params array; `Doc.List`; lone surrogate escape; 100,000 `[`;
  `\xff\xfe`; blank lines; a 400-char path; missing params; doc 0): 18 responses for 18 non-blank
  lines, each with its documented code/kind (`io_error` message 200 chars), rc 0. The same lines
  through `kiln script` gave byte-identical stdout (rc 3 vs control's 0, as documented). 301 random
  control requests (68 refused): `kiln script` and `kiln-cli control` stdout identical, files
  identical. **Confirmed.**

### ws.path-confinement
- Symlink to a file outside → `outside_root`; symlinked directory outside → `outside_root`;
  `../…` → `invalid_path`; the outside file unchanged. **Confirmed.**

### cli.run
- The worked example of control-protocol.md: rc 3, stdout byte-equal to the document, file
  `hello world\n`, log byte-equal. **Confirmed.**

### text.positions
- `doc.position {char: i}` for i = 0…10 of `a\r\nbé😀\rc\n` against my model (byte, utf16, line,
  col): 0 mismatches; 2 (between `\r\n`) and 10 (past the end) refused. **Confirmed.**

### undo.roundtrip
- 30 seeded sessions of up to 25 random inserts/deletes (CR, CRLF, emoji) with cursor moves, 40
  undos → original text, 40 redos → final text: 0 mismatches. **Confirmed.**

### search.literal
- 40 random texts × 8 literal queries: matches and totals = Python's non-overlapping `find`:
  320 cases, 0 mismatches. **Confirmed.**

### syntax.highlight
- The published example reproduced; spans of a second file cover it exactly once in order; in
  the UI runs above 0 colour mismatches outside NEW-1 (the highlight agreed with the window's
  pixels). **Confirmed.**

## Wired but inert
- `scroll` with `direction` `left`/`right`: accepted (`ok`, `settled`) on `Editor` and on
  `Documents` (with a 405-char path open), but nothing changed in either; no area of this UI
  scrolls horizontally (ui-editor.md says the editor does not). Reported as inert parameter values.
- `Documents` entries (role `label`): a click (`f005.rs`, `f149.rs`) changed nothing and logged
  nothing; they are labels, not controls, and ui.md gives them no action (tabs are `ui.tabs`,
  planned). Listed so the director knows a user click there does nothing.
- `click` with `button: "secondary"` on `Open` (Path `a.rs`) and on `Editor`: nothing. Not specified.
- Every other control the snapshot showed had an effect: `Path` (typing, `Enter` opens), `Open`,
  `Editor`, the `Documents` `scroll_view`, the `scroll_bar` node (a click on it scrolled the list from
  `f000–f010` to `f015–f025`), `Message`/`Title` (display). Every documented refusal kind of ui.md
  occurred (each `not_actionable` case included); every documented limit I reached held (`count`
  600, `steps` 100, `--size` 200–8192, the 1,048,576-char colour limit, the 100 temporary names).

## Defects filed
Director's note, added when the file was published: the evaluator named its findings NEW-1 to NEW-4; the director added NEW-1 to the draft T-066 (the rows a snapshot reports for shaped text), NEW-2 to NEW-4 and the questions Q2 and Q3 to the draft T-053 (published texts), and filed the inert requests and Q1 as the draft T-068; the click on a `Documents` entry is noted in T-058. The answers to the questions are in D-169.

| Task | Item | Summary | Suspected origin task |
|---|---|---|---|
| NEW-1 | ui.editor-view | snapshot colours shifted by one char after a combining mark at the start of a row (and in 2 other rows with marks); the window paints them right | T-055 (snapshot), T-056 |
| NEW-2 | ui.editor-view | ui-editor.md: rows are "the lines consecutive", but an empty line has no row | T-056 (ui-editor.md) |
| NEW-3 | ui.window | ui.md names the shader cache under HOME but not how to turn it off | T-065 (ui.md) |
| NEW-4 | ui.window | ui.md: a smaller screen shows what `kiln script --size <w>x<h>` shows, but below 200 points no such `--size` exists | T-065 (ui.md) |

### NEW-1 — snapshot colours shifted next to combining marks
- Reproduction (`drivers/combcol.py`): `c.rs` with lines `let x = 1;`, `́let x = 1;`,
  `́ let x = 1;`, `̀̀̀ let x = 1;` …; `kiln script` with
  `{"id":"s","ui":"snapshot"}` and `syntax.highlight`. The row `◌́let x = 1;` is reported with `l`
  `#dcdcdc` (kind keyword → `#569cd6`), `x` `#569cd6` (plain), `1` `#dcdcdc` (number → `#b5cea8`),
  `;` `#b5cea8` (plain): every colour after the mark is one char late; row text correct. Same for
  `◌́ let x = 1;` and `◌̀◌̀◌̀ let x = 1;`. Not shifted in the same run: `a◌́ let`, `x ◌́ let`,
  `e◌́◌́ let`, `\t◌́let`, `e◌́let`, `◌̈◌́ let`, `// e◌́ let`, `let ◌́x`, `◌⃣ let`, `◌ि let`, `◌ั let`,
  `x◌́◌́◌́◌́ let` (`out-combcol.txt`: 6 of 20 lines reported wrong, 3 of them only by my model's
  treatment of zero-width chars, see below). Seed 5 of the fuzz: 528 wrong chars in 8 of 60
  sessions; 10 of them in 2 sessions on rows that contain a mark but do not begin with one
  (`1́_fn f() 😀letΩéy let ́xb1.5z9a中ßc_😀`: `5` reported plain, `z` number;
  `ß"s"́1aΩ0😀`: `1` plain, `a` number). I did not find the exact rule for those two.
- The real window paints the row correctly: `work/win/comb/zoom.png` (`◌́` drawn with a dotted
  circle, `let` blue, `1` green, `;` grey). So the toolkit draws an extra glyph and the snapshot's
  char-to-glyph colour mapping counts it.
- Expected: ui-editor.md "Every char of the painted rows that paints pixels has the colour of its
  `syntax.highlight` kind"; ui.md "Its colour is the colour it was painted with" and "a combining
  mark … that the toolkit shapes into more or fewer glyphs than chars changes nothing in the
  reported text" (the text is right; the colours are not).
- Note on my model: zero-width chars that paint nothing (`U+200D`, `U+200B`, `U+FEFF`) at a row
  start are reported in the colour of the next char that paints (keyword); that matches the
  documented neighbour rule, so I do not file it.

### NEW-2 — "the lines consecutive" vs empty lines
- Reproduction: `a.rs` = `fn a() {}\n\n   \n\t\tx\r\ny\rz\r\né́中文 😀 end\nlast`, snapshot →
  `Editor` rows `fn a() {}`, `   `, `\t\tx`, `y`, `z`, `é́中文 😀 end`, `last`: the empty line 1 has no
  row, so the rows' lines are 0, 2, 3, … (`drivers/probe.py`).
- Expected: ui-editor.md "one row for each document line in view that has at least one char in the
  visible part, the lines consecutive"; its Limits say "An empty line paints nothing, so it has no
  row". The first sentence is wider than the behaviour; a reader cannot map row r to a line from it.

### NEW-3 — the shader cache is named but not how to turn it off
- Reproduction (`drivers/wincases.sh`): window run with a fresh HOME and no
  `MESA_SHADER_CACHE_DISABLE` → 11 files in `HOME/.cache/mesa_shader_cache/`; with
  `MESA_SHADER_CACHE_DISABLE=true` → 0. strace: nothing else written outside the root.
- Expected: brief: "the documents name one cache and how to turn it off"; ui.md says only "the GL
  driver may write its shader cache under the home directory" (Limits) and "the window … writes no
  file of its own". Behaviour matches the sentence; the way to turn it off is not documented.

### NEW-4 — screens smaller than 200 points
- Reproduction (`work/win/geo`): `xvfb-run -a -s "-screen 0 160x120x24" kiln --script s.jsonl
  --root fix a.rs` → window 160×120 (XQueryTree), snapshot `Title` and `Editor` `offscreen`, a click
  on `Editor` → `not_actionable` `the target is offscreen`, rc 3; `kiln script s.jsonl --root fix
  --size 160x120 a.rs` → rc 2 `kiln: bad --size `160x120`: expected <w>x<h>, integers from 200 to
  8192`.
- Expected: ui.md "On a screen smaller than 1280×800 the window is smaller (the screen's size), …
  it is what `kiln script --size <w>x<h>` shows with the window's size". Wider than the behaviour
  for screens under 200 points in either dimension (no equivalent exists). Low impact.

### Known and already filed (observed, not filed again)
- T-025: `a\rb\nc\nd\n`, caret after the lone `\r`, `Enter` → `doc.insert "\n"` at 2, text
  `a\r\nb\nc\nd\n`, caret 3 (line 1 col 0): no line added (`out-edges-2.txt`).
- T-045: opening and showing a 1,048,576-char `.rs` took 8.96 s in a debug `kiln script` run; I did
  not time a keystroke there.
- T-066, T-048, T-053: not exercised / not checked (I did not compare the 1,110-action count).

## Not evaluated
- None of the three UI items. Not run, and why:
  - Release build: not needed for any check (the window's equality was measured on debug only;
    ui.md also claims release).
  - An ordinary (non-root) identity: permission cases were run as uid 0 with
    `CAP_DAC_OVERRIDE`/`CAP_DAC_READ_SEARCH` dropped (`capsh`, `drivers/edges.py save`): read-only
    file and read-only directory → `io_error … Permission denied (os error 13)` in `Message`.
  - A file replaced *during* a headless script: not possible (the script is read whole at start);
    done in the real window instead.
  - IME, drag, double click, real `Tab`/`PageUp` behaviour: unspecified by the documents.

## Questions for the director
- **Q1 (ui.window):** the no-display stderr line is cut at 200 chars before its reason and names
  the build machine's cargo registry: `kiln: no window: winit EventLoopError: os error at
  /root/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform_impl/linux/mod.rs:765:
  neither WAYLAND_DISPLAY nor WAYLAND_SOC…` (202 bytes). "One line on stderr" holds; is this text
  acceptable?
- **Q2 (ui.md targets):** `{"label":"Open","role":"button","value_contains":""}` → `no_target`
  (the button has no `value`). ui.md: "a node matches when every given field matches … `*_contains`
  as a substring". Should an empty substring match a node without a value?
- **Q3 (ui.md shell):** after 150 control `doc.open`s and one `frames`, the overflowing `Documents`
  area had no `scroll_bar` node; after 2–3 frames it had one. ui.md says the scroll bar "is a node of
  role `scroll_bar`". Is "only once the area has settled" the intended reading?
- **Q4 (ui.window):** under Xvfb with no window manager, XTest keys reached the window only after
  `XSetInputFocus`; an XTest click into the window moved the caret but did not give keyboard focus.
  Environment (no window manager) or a gap to state in ui.md?
