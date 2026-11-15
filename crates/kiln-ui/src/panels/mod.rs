//! The places of the later panels (T-056 Pinned (b)): the shell calls one `show` function per
//! panel, each in its own file, so that the module tasks T-057 to T-061 each change their own
//! file only. In this task every panel draws nothing.
//!
//! | File | Panel | Placed by the shell |
//! |---|---|---|
//! | `project.rs` | project tree (T-057) | beside the editor, on its left |
//! | `tabs.rs` | tabs (T-058) | above the editor |
//! | `status.rs` | status bar (T-059) | below the editor |
//! | `outline.rs` | outline and navigation (T-060) | beside the editor, on its right |
//! | `find.rs` | find bar (T-061) | above the editor, below the tabs |
//!
//! Each `show` gets the `Ui` of the rest of its region (a right-to-left layout for the outline,
//! bottom-up for the status bar, top-down or left-to-right for the others): what it allocates
//! there is taken from the editor's space.

use serde_json::Value;

use crate::session::Session;

pub(crate) mod find;
pub(crate) mod outline;
pub(crate) mod project;
pub(crate) mod status;
pub(crate) mod tabs;

/// What a panel may use: the one session, through which it sends control requests and reads
/// and sets the current document, and the shell's `Message` line.
///
/// - [`PanelView::request`] sends a control request; a successful one is logged and makes the
///   editor view re-read in the same frame (it counts as a request);
/// - [`PanelView::read`] sends a read-only control request (`mutating: false`): logged, but
///   not a request for the editor view's re-read. A panel sends reads only in the first frame
///   after a request or an input, as the editor view does (`docs/protocol/ui-editor.md`, E5);
/// - [`PanelView::current`] and [`PanelView::set_current`]: the current document (the
///   editor shows it; T-058 switches it by tab);
/// - [`PanelView::open_documents`]: the open documents `(id, path)` in id order;
/// - [`PanelView::set_message`]: what `Message` shows (a refusal begins with its kind).
#[allow(dead_code)] // the panels of T-057 to T-061 use it; in this task they draw nothing
pub(crate) struct PanelView<'a> {
    session: &'a mut Session,
    message: &'a mut String,
}

#[allow(dead_code)] // the panels of T-057 to T-061 use it; in this task they draw nothing
impl<'a> PanelView<'a> {
    pub(crate) fn new(session: &'a mut Session, message: &'a mut String) -> Self {
        PanelView { session, message }
    }

    /// Send the control request `command` with `params`; its result, or the refusal's kind
    /// and message.
    pub(crate) fn request(
        &mut self,
        command: &str,
        params: Value,
    ) -> Result<Value, (String, String)> {
        self.session.request(command, params)
    }

    /// Send the read-only control request `command` with `params`.
    pub(crate) fn read(&mut self, command: &str, params: Value) -> Result<Value, (String, String)> {
        self.session.read(command, params)
    }

    /// The current document's id, if a document is open.
    pub(crate) fn current(&self) -> Option<u64> {
        self.session.current()
    }

    /// Make the open document `doc` current (ignored if it is not open).
    pub(crate) fn set_current(&mut self, doc: u64) {
        self.session.set_current(doc);
    }

    /// The session's change counter (`Session::changes`): it changes with every successful
    /// request other than a read, and with every change of the current document.
    pub(crate) fn changes(&self) -> u64 {
        self.session.changes()
    }

    /// The open documents `(id, path)` in id order.
    pub(crate) fn open_documents(&self) -> &[(u64, String)] {
        self.session.docs()
    }

    /// Set what `Message` shows.
    pub(crate) fn set_message(&mut self, text: String) {
        *self.message = text;
    }
}
