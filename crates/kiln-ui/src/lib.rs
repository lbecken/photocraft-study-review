//! # kiln-ui — layer L5
//!
//! Responsibility (CHARTER §3): the egui desktop user interface. It may only dispatch
//! commands and read state; it never calls mutating APIs of lower crates. `cargo xtask
//! ui-commands` enforces this with the explicit import allowlist in `xtask/ui-allowlist.toml`.
//!
//! May depend on: L0–L4.
//!
//! The UI reaches the session through one [`kiln_control::Host`] (ADR-0008, ruling Q-1 A):
//! every change it makes is a control request, so it appears in the session log.
//!
//! What exists: the shell of T-055 (a `Path` input, an `Open` button, a `Message` line and the
//! list of open `Documents`), the editor view of T-056 (the current document's `Title` and the
//! `Editor` with syntax colours, caret, selection and scrolling; `docs/protocol/ui-editor.md`),
//! the empty places of the later panels, and [`Script`], the runner of `kiln script`, which
//! drives the UI without a window from control requests and UI requests and reports what each
//! frame shows (`docs/protocol/ui.md`). The window is T-065.
//!
//! ## The frame function (shared with T-065)
//!
//! Every frame of the UI is drawn by one function, [`draw_frame`]:
//!
//! ```text
//! pub fn draw_frame(ui: &mut egui::Ui, app: &mut App)
//! ```
//!
//! `ui` is the frame's root `Ui` (as `egui::Context::run_ui` passes it) and [`App`] the UI's
//! state: the shell, the editor view and the session. `kiln script` calls it for every frame
//! ([`Script`]); the window of T-065 calls the same function, so both show the same UI. An
//! [`App`] is made from a [`kiln_control::Host`] with [`App::new`]; [`App::open`] opens a path
//! as `Path` and `Open` do (the start paths); [`App::handle_line`] sends one control request
//! line; [`App::log_script`] is the session log. [`apply_style`] sets the theme and style the
//! script mode uses (dark, no caret blink).

mod editor;
mod json;
mod observe;
mod panels;
mod script;
mod session;
mod shell;

pub use script::{
    DEFAULT_SIZE, Files, KEY_NAMES, MAX_FRAMES, MAX_SCROLL_STEPS, SETTLE_CAP, SIZE_RANGE, Script,
    parse_size, response_ok,
};

use editor::EditorView;
use session::Session;
use shell::Shell;

/// The UI's state: the shell, the editor view and the one session they share.
pub struct App {
    shell: Shell,
    editor: EditorView,
    session: Session,
}

impl App {
    /// A UI over the session of `host`, with nothing open.
    pub fn new(host: kiln_control::Host) -> App {
        App {
            shell: Shell::default(),
            editor: EditorView::default(),
            session: Session::new(host),
        }
    }

    /// Open `path` as if it were typed into `Path` and opened (a `doc.open` control request):
    /// a refusal sets `Message` to begin with its kind, a success clears `Message` and makes the
    /// document current. `Path` itself is left as it is.
    pub fn open(&mut self, path: &str) {
        self.shell.open(&mut self.session, path);
    }

    /// One control request line in, the host's response out (`None` for a blank line), exactly
    /// as [`kiln_control::Host::handle_line`].
    pub fn handle_line(&mut self, line: &[u8]) -> Option<String> {
        self.session.handle_line(line)
    }

    /// The session log as a script ([`kiln_control::Host::log_script`]).
    pub fn log_script(&self) -> String {
        self.session.log_script()
    }
}

/// Draw one frame of the Kiln UI into the frame's root `ui` (see the crate documentation).
pub fn draw_frame(ui: &mut egui::Ui, app: &mut App) {
    let App {
        shell,
        editor,
        session,
    } = app;
    shell.ui(ui, session, editor);
}

/// The theme and style of the script mode: dark, no caret blink (T-055).
pub fn apply_style(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    ctx.all_styles_mut(|style| style.visuals.text_cursor.blink = false);
}
