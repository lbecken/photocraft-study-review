---
id: ADR-0008             # file docs/adr/0008-ui-observation.md
title: "Tests and the evaluator observe the UI through a headless script mode of the desktop binary that answers control requests and UI requests and reports what the frame shows"
status: accepted
date: 2026-11-14
task: T-055              # the high-tier task that carries it; T-056 is its first consumer
charter_section: "§3 decision 1 (headless core; everything the UI can do, the CLI can do), §3 L5 row; §4 roles (evaluator)"
approval: D-147
supersedes: null
superseded_by: null
---

# ADR-0008 How UI behaviour is observed

Written by the planner of the second stage (run R-225, booked to `iter-2`), at main
`ec08d22c6c12639c31022a221820c23a1aeeba32`. Labels: **VERIFIED** (a command was run and printed
it; the commands are in "Evidence"), **INFERRED** (reasoning from verified facts),
**PROPOSED** (this decision).

## Context

Facts:
- 23 of 64 scorecard items are `planned`; 8 are `ui.*` (`docs/scorecard.md`, generated). No UI
  code exists: `crates/kiln-ui/src/lib.rs` is a comment; `apps/kiln/src/main.rs` prints that the
  UI is not built and exits 1 (read at this main).
- D-145 orders the stage: first "the interface through which tests and the evaluator observe UI
  behaviour", and "each [UI module] counted only when the evaluator observes the behaviour
  through the observation interface". D-140 states the failure this guards against: PhotoCraft's
  parity count measured wiring, not behaviour.
- METHODOLOGY §9 and the evaluator brief give the evaluator "the UI snapshot mechanism";
  `methodology/templates/evaluation.md` lists the interface `ui-snapshot`. No such mechanism
  exists in `xtask/`, `apps/`, `crates/` or `docs/` (`grep -rln "ui-snapshot\|ui_snapshot"` finds
  only the template; VERIFIED). Retro-2s1, "Recommendation on stage 2", names this as the first
  thing to settle and says it is an ADR, not a task detail.
- METHODOLOGY §6 asks every scorecard item to have an acceptance test "through the CLI or control
  protocol"; a UI item has neither today.
- The charter names the toolkit: L5 `kiln-ui`, "egui user interface" (CHARTER §3). The
  `ui-commands` gate step limits what `kiln-ui` may name from lower crates
  (`xtask/ui-allowlist.toml`: `Registry`, `standard_registry`, `Context`, `CommandError`);
  `kiln_control::Host`, which speaks the control protocol and owns a session, is not listed.
  `xtask/layers.toml` already forbids egui, eframe, winit, egui_kittest and the window-system
  crates in layers 0–4, the testkit and headless apps.
- The machine that runs the gate and the evaluator here has 4 cores, no GPU and no display
  (`DISPLAY` and `WAYLAND_DISPLAY` unset). CI runs the gate on `ubuntu-24.04`
  (`.github/workflows/ci.yml:18`); what that runner has installed is not visible from here.

### Evidence: the feasibility probe (VERIFIED, R-225)

Made in a scratch directory outside the repository with an isolated cargo home, every command
under `timeout`; nothing was installed on the system; the scratch builds were deleted afterwards.
Toolchain 1.97.0 (`rust-toolchain.toml`).

| # | Question | Command (abridged) | Result |
|---|---|---|---|
| a | Toolkit and fetch | `cargo info egui`, `cargo info eframe`, `cargo info egui_kittest`; `cargo build` of three probe crates | egui, eframe, egui_kittest **0.37.0**, each `rust-version: 1.95`; all crates and their dependencies were downloaded from crates.io and built |
| b1 | Headless frames, scripted input, text read-back, egui alone | probe crate with `egui = { version = "=0.37.0", default-features = false, features = ["default_fonts"] }`; `Context::enable_accesskit()`; `Context::run_ui(RawInput { screen_rect: 800×600, time: Some(t), events, .. })` for each frame; a scripted pointer click on a button found by its label in the AccessKit tree, a click into a multiline text input, `Event::Text("// hi")`, `Key::Enter`; the last `accesskit_update` printed node by node | one line per node with role, label and value: `role=Button label=Some("Increment")`, `role=Label value=Some("clicks: 1")`, `role=MultilineTextInput value=Some("// hi\nfn main() {}\n")`; the same session run twice in one process: `deterministic across two sessions: true` |
| b2 | What was painted, with colour, and what is visible | same crate, 200 labels each a two-section `LayoutJob` (`"fn"` red, `" line_i"` blue) in a vertical `ScrollArea`, window 400×300; `FullOutput.shapes` filtered to `Shape::Text` whose galley rectangle meets its clip rectangle | `15 shapes, first "fn"#c80000 " line_0"#0000c8, last … " line_14"…`: the painted text, its colour per section and the visible subset are readable as text; two sessions equal |
| b3 | The toolkit's own test facility | probe crate with `egui_kittest = { version = "=0.37.0", default-features = false }`: `Harness::builder().with_size(800×600).build_ui_state(…)`, `get_by_label("Increment").click()`, `get_by_role(Role::MultilineTextInput).focus()` / `.type_text("// hi")`, `key_press(Key::Enter)`, `run()`, tree printed with `{:#?}` | works without a window; tree with role, label, value, focused, disabled, children; two sessions equal. The harness **refuses a frame in which a widget has no accessible name** (`1 widget(s) lack accessible name … MultilineTextInput`), fixed by labelling the input. Each `run()` settled within its default of 4 steps of 0.25 s simulated time |
| b4 | Scrolling | kittest: `get_by_label("line_3").scroll_down()` (an AccessKit `ScrollDown` action), then a `MouseWheel` event after `hover_at` | the action scrolled (first visible label `line_0` → `line_5`); mouse-wheel events, with `Line` and `Point` units and with and without touch phases, did **not** scroll in either probe (cause not found) |
| b5 | Determinism across processes | `./paint \| md5sum` twice, `./probe-kit \| md5sum` twice | identical digests (`49823b8a…` twice; `5c211b38…` twice) |
| b6 | Cost of a headless session | `/usr/bin/time` on the probes (debug) | 0.04 s (16 frames), 0.36 s (about 72 frames with 200 labels), 0.05 s (two harness sessions); about 15 MB resident |
| c1 | Build tree and time, standalone, debug, clean | `rm -rf target; /usr/bin/time cargo build; du -sh target; cargo tree -e normal` | egui: **23.15 s, 572 MB, 54 packages**; egui + egui_kittest: **23.72 s, 637 MB, 65 packages**; eframe (`glow`, `x11`, `default_fonts`, no default features): **37.69 s, 1.1 GB, 106 packages** |
| c2 | Added to this workspace | copy of the workspace (no `target/`); `kiln-ui` given `egui` and `eframe` as above and `egui_kittest` as a dev-dependency; `cargo metadata`; compare the locks; `cargo build -p kiln-ui -p kiln`; `cargo test -p kiln-ui --no-run` | `Cargo.lock` 122 → 331 entries, 183 new package names, **0 existing packages changed version** (so the queue's `dependency versions` approval is not needed as of this main); build **50.86 s**, then test build **3.46 s**, target **962 MB**; with `CARGO_PROFILE_DEV_DEBUG=line-tables-only` both together **36.20 s**, **728 MB** |
| c3 | Size of one debug binary that links the toolkit | `ls -l` | 118.7 MB (egui), 127.7 MB (egui + kittest), 195.9 MB (eframe) |
| c4 | Features that fail to build here | eframe with its default features, or with its `accesskit` feature | `wayland-sys` build script fails (no `wayland-client` pkg-config file); `cargo tree -i wayland-sys` shows `accesskit_winit` → `winit` default features. eframe's `glow` + `x11` build needs no system development package |
| d | A real window | eframe `run_native` app that draws 5 frames and closes: (1) as is; (2) `xvfb-run -a -s "-screen 0 1280x800x24"`; (3) as (2) with `libxkbcommon-x11-0` and `libxcb-xkb1` unpacked from `apt-get download` into scratch and put on `LD_LIBRARY_PATH` | (1) `Err`: `neither WAYLAND_DISPLAY nor WAYLAND_SOCKET nor DISPLAY is set`; (2) panic `Library libxkbcommon-x11.so could not be loaded` (Xvfb and Mesa GL are installed; these two libraries are not); (3) `run_native returned: Ok(())`, exit 0, 1.06 s |

What the probe did not establish: the same behaviour on the CI runner; text input methods (IME);
mouse-wheel scrolling; frame timing under load other than the probes' own; anything about
release builds; the cost of a real editor view (the probes draw at most 200 labels).

INFERRED from the probe:
1. egui can be driven entirely without a window, with simulated time, and the frame's result can
   be read back as text in two forms: the accessibility tree (roles, labels, values, focus,
   selection) and the painted text with its colours and visible extent. Neither depends on a
   display, a GPU or the wall clock; both were byte-identical across runs and processes.
2. The accessibility tree alone cannot show syntax colours or what part of a document is
   scrolled into view; the painted text can. An observation of `ui.editor-view` needs both.
3. A real window cannot start here as the machine is, and can with two runtime libraries and
   Xvfb. A gate test that opens a window would fail on this machine (as NEW-3's tests failed in a
   location they did not state) unless the environment is changed by the owner.
4. The toolkit's own harness (egui_kittest) is a Rust library for tests; it is not a public
   interface the evaluator can use without reading or writing code. A public interface has to be
   a mode of the product binary.

## Options

| Option | For | Against |
|---|---|---|
| A. **Headless script mode of the desktop binary** (`kiln script`): one script mixes control-protocol requests (answered exactly as `kiln-cli control` answers them, in the same session the UI uses) and UI requests (focus, click, type, key, scroll, run frames, snapshot); a snapshot reports the accessibility tree plus the painted text and colours of the nodes that show text | a public interface of the product, usable from a shell by the evaluator and from tests that drive the built binary (preferred by METHODOLOGY §6); the UI's effect can be checked against the registry in the same session (`doc.text`, `cursor.get`, `syntax.highlight`) and against the session log; deterministic (b5); no display needed (b1–b3) | the UI is driven by targets (role and label), not by pixels, so a layout fault that hides a control from a user but not from the tree is missed unless "offscreen" is reported; a second binary mode to maintain; snapshot shape becomes a published format |
| B. Tests only through egui_kittest in `kiln-ui`, no product interface | the toolkit's facility; least code | not a public interface: the evaluator would read or write Rust tests (forbidden by the brief); a test-only path can drift from what the binary does |
| C. Pixel screenshots of a real or software-rendered window, compared to references | sees exactly what a user sees, layout included | needs a display or a GPU or software renderer (d: not available as the machine is; `wgpu`/`snapshot` features add a renderer not probed); pixel comparisons are not text-readable or deterministic across machines; a reference image is a whole-document pin (golden rule 9) |
| D. The UI publishes its own view model (a description it writes of what it intends to show) | simple to read | the UI describing itself is the wiring measurement D-140 warns against: nothing checks that what is described is what is drawn |
| Keep the current contract | nothing to build | no UI item can be observed; the counting rule of D-145 cannot be applied; the 8 `ui.*` items stay `planned` |

## Decision (PROPOSED)

Option A.

1. **The interface.** The desktop binary `kiln` has a headless mode, `kiln script <script>
   [--root <dir> | --memory] [--log <file>] [--size <w>x<h>]`, that runs the same UI as the
   window, with no window, display or GPU. A script is JSON lines. A line that is an object with
   a key `ui` is a **UI request**; every other line is a **control request** and gets exactly the
   response `kiln-cli control` would give in the same session (same bytes). Both kinds act on one
   session: the UI's own state changes and the script's control requests go to the same
   documents, and both appear in the session log written by `--log`, in the format of
   `docs/control-protocol.md` "The session log as a script". The request and response formats,
   the UI request kinds and the refusals are specified in T-055 and published in
   `docs/protocol/ui.md`.
2. **What a snapshot reports.** The UI as the last frame left it: a tree of nodes, each with its
   accessibility role, label, value and states (focused, disabled, selected, toggled, offscreen;
   a text selection in chars for text inputs), and for every node that shows text, the text the
   frame painted inside it and inside the visible part of the window, row by row, each run with
   its colour. A snapshot is computed from what the toolkit produced for that frame (its
   accessibility output and its painted shapes), not from a description the UI code writes about
   itself.
3. **Input** is scripted by target, never by pixel: a UI request names a node by role and label
   (exact, or containing a text), and the runner delivers what a user's action would deliver to
   it: a pointer click at it, focus, typed text, a key with modifiers, a scroll step. A request
   whose target matches no node or several nodes is refused and changes nothing.
4. **Determinism.** The same script on the same initial files with the same build prints the same
   bytes and writes the same log, at any location and under any load: frame time advances by a
   fixed step per frame and is never read from a clock; the window size is fixed (`--size`,
   default stated in `docs/protocol/ui.md`); the fonts are those built into the binary. After
   each UI request the runner runs frames until the UI asks for no further frame, up to a
   stated cap, and says how many it ran.
5. **Dispatch only, observably.** Every change the UI makes to a document or a file is a request
   of the control protocol: it appears in the session log, and the log replayed with `kiln-cli run`
   on a copy of the initial files reproduces the files. The UI issues no request while nothing
   happens (frames on an idle UI add nothing to the log). This is the observable side of
   `ui.dispatch-only`; the `ui-commands` gate step stays its static side.
6. **The window.** `kiln [--root <dir>] [<path>…]` opens the window. Where no display is
   available it exits with a non-zero status and one stderr line saying so, without a panic.
   Whether a real window counts towards `ui.window` is the director's question Q-2 below; the
   evaluator can start one on this machine only under Xvfb with the two libraries of (d).

### What counts as "observed" for a UI scorecard item

A `ui.*` item is confirmed by the evaluator only if all of these hold, through `kiln script`
(and for `ui.window`, as Q-2 decides):
- (O1) **Act as a user.** The behaviour is triggered by UI requests (click, type, key, scroll)
  on targets the snapshot shows, not by control requests. Control requests may set up the
  session (open a file) and check it.
- (O2) **See the effect in the UI.** A snapshot after the action differs from the one before in
  the way the item's title says (a tab appears, a caret moves, the painted rows change).
- (O3) **Check it against an independent source.** The effect agrees with the registry in the
  same session (control requests such as `doc.text`, `doc.list`, `cursor.get`,
  `syntax.highlight`, `project.tree`), with the file on disk, or with a naive model; for colours,
  with `syntax.highlight` and the published colour of each highlight kind.
- (O4) **See it went through the registry.** For a change of state, the session log holds the
  command that made it, and nothing the UI did changed state outside the log.
- (O5) **Not inert.** Every control of the module the snapshot shows changes something
  observable when used, or is reported as wired but inert.
Not observation: a node existing (a control without its effect), the session log alone (wiring),
a snapshot the UI wrote about itself without (O3), or a Rust test the evaluator did not run
through the binary.

Builders' acceptance tests for a UI item follow the same shape: drive the built `kiln` binary
with a script, assert on nodes found by role and label (never on a whole snapshot, golden rule
9), and on an independent source as in (O3).

## Consequences

- **Tasks.** T-055 builds the interface and a minimal shell (high: new dependencies, a new
  write path for `--log`). T-056 (window and editor view, high: eframe) is its first consumer;
  every UI module task (T-057 onward) adds its observations to `docs/protocol/ui-<module>.md`
  and its tests to `apps/kiln/tests/`. ADR-0007 orders them.
- **The evaluator's brief** (`methodology/roles/evaluator.md`) and template change at the next
  methodology edit (director, ledger): "the UI snapshot mechanism" becomes "`kiln script` and
  `docs/protocol/ui.md`"; the template's interface name `ui-snapshot` becomes `ui-script`; the
  brief states (O1)–(O5) as the rule for `ui.*` items and that, where no display is available,
  `ui.window` is judged as Q-2 decides. Until then the director names these in the dispatch
  message.
- **Disk and build time.** The toolkit adds about 1 GB and about 54 s to a debug build of the
  UI crates (c2); debug binaries that link it are 119–196 MB each (c3). UI acceptance tests
  should drive the one `kiln` binary rather than link the toolkit into many test binaries.
  The lighter debug-info profile (T-054) reduces the same build to 728 MB (c2) and the
  workspace's test build from 6.3 to 2.5 GB (D-097).
- **Features.** eframe without `wayland` and without its `accesskit` feature (c4: both fail to
  build here); the accessibility output used by `kiln script` is egui's own and needs no
  platform adapter. A screen-reader adapter for the real window is out of scope for this stage.
- **CHARTER text to change:** none. §3 decision 1 ("everything the UI can do, the CLI can do")
  stays true: the UI's state changes are control requests. If Q-1 is answered A,
  `xtask/ui-allowlist.toml` gains one entry (`kiln_control::Host`), a gate-path change inside
  T-055.
- **Checks.** T-055's acceptance tests (determinism, pass-through equality with `kiln-cli run`,
  log replay, refusals); the existing `ui-commands` and `layers` gate steps; review against the
  binary for every sentence of `docs/protocol/ui.md`. Rule only: that a snapshot is computed from
  the toolkit's output and not from a self-description (reviewer, by reading; (O3) makes a
  self-description visible when it disagrees with the registry).

## Questions for the director

- **Q-1. How does the UI reach the session?** A: `kiln-ui` runs a `kiln_control::Host` (one
  allowlist entry): the UI's requests are control requests byte for byte, interleaved script
  requests need no second implementation, and the log is the control protocol's. B: allowlist
  `kiln_workspace::DiskFs` and build `Context` directly: the UI reads nothing more (documents
  are not readable through `Context`'s public API anyway), and the protocol framing, error caps
  and log format would be repeated in `kiln-ui`. C: `apps/kiln` builds the session and hands it
  in: no allowlist change, but `apps/kiln` is not scanned by `ui-commands`, so dispatch-only
  would rest on review. **Recommendation: A.**
- **Q-2. What counts for `ui.window`** ("The desktop app opens a window showing the open
  document")? A: gate tests through `kiln script` (the window's root view shows the document)
  plus the binary's no-display exit, and the evaluator additionally starts the real window under
  Xvfb with the two libraries of (d) and reports it; the item is confirmed only if the window
  started. B: headless only (the title's "opens a window" is then not observed). C: a gate test
  that opens a window under Xvfb (fails on this machine unless the owner adds the two libraries
  to the environment's setup; environment-dependent, as NEW-3). **Recommendation: A**, with the
  environment change for the evaluator's machine put to the owner (it installs two packages).

## Verification

T-055 AC-1 to AC-9; T-056's criteria for `ui.window`, `ui.editor-view` and `ui.dispatch-only`;
the next evaluation records, for each `ui.*` item, (O1)–(O5) with the script and the snapshot
excerpts. Retrospective of stage 2: count UI items confirmed and rejected, UI controls reported
inert, and evaluator findings that the snapshot could not show (a missed layout fault would be
one).

## Rollback

A new ADR. The script mode is one subcommand of `apps/kiln` and a module of `kiln-ui`; removing
it leaves the window. Without it no UI item can be observed, so rolling back also returns the
`ui.*` items to `planned` unless another observation interface replaces it. The allowlist entry
of Q-1 A is removed with it. No data or file format outside `docs/protocol/ui*.md` depends on
it.

## Rulings at acceptance (director, D-147)
- Q-1: A. The UI runs a `kiln_control::Host`; `xtask/ui-allowlist.toml` gains that one entry (a gate-path change,
  named in the approval row of T-055).
- Q-2: A. `ui.window` counts on headless gate tests plus a real window started under Xvfb by the reviewer and by the
  evaluator. Until the environment provides them, the reviewer and the evaluator unpack the two libraries the probe
  needed (`libxkbcommon-x11-0`, `libxcb-xkb1`) into their scratch directory, as the probe did; nothing is installed
  on the system. If a real window cannot be started in a run, `ui.window` is `not_evaluated` in that run.
