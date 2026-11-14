//! The shell: what the Kiln UI shows in this task (T-055). A text input labelled `Path`, a
//! button `Open`, a message line labelled `Message` and the list `Documents` of the open
//! documents. T-056 adds the window and the editor view around it.
//!
//! The shell changes the session only by sending control requests through [`Session`]; what
//! it shows of the session comes from [`Session::docs`].

use egui::accesskit::Role;
use egui::{CentralPanel, Key, ScrollArea, TextEdit, Ui, UiBuilder};
use serde_json::{Value, json};

use crate::json::canonical;
use crate::session::Session;

/// The shell's own state (what is typed, the last refusal).
#[derive(Default)]
pub(crate) struct Shell {
    path: String,
    message: String,
}

impl Shell {
    /// Draw one frame of the shell.
    pub(crate) fn ui(&mut self, ui: &mut Ui, session: &mut Session) {
        CentralPanel::default().show(ui, |ui| {
            let mut open = false;
            ui.horizontal(|ui| {
                let label = ui.label("Path");
                let edit = ui
                    .add(TextEdit::singleline(&mut self.path).id_salt("kiln-shell-path"))
                    .labelled_by(label.id);
                if edit.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    open = true;
                }
                if ui.button("Open").clicked() {
                    open = true;
                }
            });
            if open {
                self.open(session);
                // What the session now holds is drawn on the next frame as well.
                ui.ctx().request_repaint();
            }
            ui.label(self.message.as_str()).accessible_name("Message");
            ui.separator();
            scroll_view(ui, "kiln-shell-documents", |ui| {
                list(ui, "Documents", |ui| {
                    for (_, path) in session.docs() {
                        ui.label(path.as_str());
                    }
                });
            });
        });
    }

    /// Ask the session to open the path in `Path` (a `doc.open` control request).
    fn open(&mut self, session: &mut Session) {
        let request = canonical(&json!({"command": "doc.open", "params": {"path": self.path}}));
        let Some(response) = session.handle_line(request.as_bytes()) else {
            return;
        };
        let response: Value = serde_json::from_str(&response).unwrap_or(Value::Null);
        self.message = if response.get("ok") == Some(&Value::Bool(true)) {
            String::new()
        } else {
            let error = &response["error"];
            let kind = error["kind"].as_str().unwrap_or("internal");
            let message = error["message"].as_str().unwrap_or("");
            format!("{kind}: {message}")
        };
    }
}

/// A vertical scroll area whose content is one accessibility node of role `scroll_view`, with
/// the visible part of the area (its clip rectangle) as its bounds. `kiln script` scrolls the
/// nearest such node around a target and treats its bounds as the visible part.
pub(crate) fn scroll_view(ui: &mut Ui, id_salt: &str, add: impl FnOnce(&mut Ui)) {
    ScrollArea::vertical()
        .id_salt(id_salt)
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.scope_builder(
                UiBuilder::new().accessibility_role(Role::ScrollView),
                |ui| {
                    add(ui);
                    let clip = ui.clip_rect();
                    ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
                        node.set_bounds(rect(clip));
                    });
                },
            );
        });
}

/// A labelled accessibility node of role `list` whose children are the nodes `add` makes.
fn list(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui)) {
    let builder = UiBuilder::new()
        .accessibility_role(Role::List)
        .accessibility_label(label);
    ui.scope_builder(builder, |ui| {
        add(ui);
        let bounds = ui.min_rect();
        ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
            node.set_bounds(rect(bounds));
        });
    });
}

fn rect(r: egui::Rect) -> egui::accesskit::Rect {
    egui::accesskit::Rect {
        x0: r.min.x.into(),
        y0: r.min.y.into(),
        x1: r.max.x.into(),
        y1: r.max.y.into(),
    }
}
