//! The shell: the frame's layout and the parts of T-055. A text input labelled `Path`, a button
//! `Open`, a message line labelled `Message` and the list `Documents` of the open documents;
//! below them the current document's `Title` and the editor view (T-056), and the places of the
//! later panels (`crate::panels`): tabs and find above the editor, project and outline beside it,
//! status below it.
//!
//! The shell changes the session only by sending control requests through [`Session`]; what
//! it shows of the session comes from [`Session::docs`] and the editor view's reads.

use egui::accesskit::Role;
use egui::{Align, CentralPanel, Key, Layout, ScrollArea, TextEdit, Ui, UiBuilder};
use serde_json::json;

use crate::editor::EditorView;
use crate::panels::{self, PanelView};
use crate::session::Session;

/// The share of the height below the top rows that `Documents` takes.
const DOCUMENTS_SHARE: f32 = 0.25;

/// The shell's own state (what is typed, the last refusal).
#[derive(Default)]
pub(crate) struct Shell {
    path: String,
    message: String,
}

impl Shell {
    /// Draw one frame: the editor's input and reads first, then the layout.
    pub(crate) fn ui(&mut self, ui: &mut Ui, session: &mut Session, editor: &mut EditorView) {
        editor.update(ui.ctx(), session, &mut self.message);
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
                let path = self.path.clone();
                self.open(session, &path);
                // What the session now holds is drawn on the next frame as well.
                ui.ctx().request_repaint();
            }
            ui.label(self.message.as_str()).accessible_name("Message");
            ui.separator();
            let documents_height = (ui.available_height() * DOCUMENTS_SHARE).max(1.0);
            // A click on an entry makes its document current, as a click on its tab does.
            let mut switch = None;
            ui.allocate_ui(egui::vec2(ui.available_width(), documents_height), |ui| {
                scroll_view(ui, "kiln-shell-documents", |ui| {
                    list(ui, "Documents", |ui| {
                        for (doc, path) in session.docs() {
                            let entry = ui.label(path.as_str());
                            if entry.clicked() || entry.secondary_clicked() {
                                switch = Some(*doc);
                            }
                        }
                    });
                });
            });
            if let Some(doc) = switch {
                session.set_current(doc);
                ui.ctx().request_repaint();
            }
            ui.separator();
            self.document_area(ui, session, editor);
        });
    }

    /// Tabs and find above, status below, project and outline beside the editor.
    fn document_area(&mut self, ui: &mut Ui, session: &mut Session, editor: &mut EditorView) {
        let message = &mut self.message;
        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            panels::status::show(ui, &mut PanelView::new(session, message));
            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                panels::tabs::show(ui, &mut PanelView::new(session, message));
                panels::find::show(ui, &mut PanelView::new(session, message));
                editor.title(ui);
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    panels::outline::show(ui, &mut PanelView::new(session, message));
                    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                        panels::project::show(ui, &mut PanelView::new(session, message));
                        editor.view(ui);
                    });
                });
            });
        });
    }

    /// Ask the session to open `path` (a `doc.open` control request), as `Open` does: a refusal
    /// sets `Message` to begin with its kind, a success clears it.
    pub(crate) fn open(&mut self, session: &mut Session, path: &str) {
        self.message = match session.request("doc.open", json!({ "path": path })) {
            Ok(_) => String::new(),
            Err((kind, message)) => format!("{kind}: {message}"),
        };
    }
}

/// A vertical scroll area whose content is one accessibility node of role `scroll_view`, with
/// the visible part of the area (its clip rectangle, cut to the area's own width) as its bounds.
/// `kiln script` scrolls the nearest such node around a target and treats its bounds as the
/// visible part.
pub(crate) fn scroll_view(ui: &mut Ui, id_salt: &str, add: impl FnOnce(&mut Ui)) {
    scroll_view_with(ui, id_salt, None, add);
}

/// [`scroll_view`], scrolled to `offset` (points from the top) in this frame when given. Returns
/// the vertical offset after the frame and the visible part (screen coordinates).
pub(crate) fn scroll_view_with(
    ui: &mut Ui,
    id_salt: &str,
    offset: Option<f32>,
    add: impl FnOnce(&mut Ui),
) -> (f32, egui::Rect) {
    let mut area = ScrollArea::vertical().id_salt(id_salt).auto_shrink(false);
    if let Some(offset) = offset {
        area = area.vertical_scroll_offset(offset);
    }
    let output = area.show(ui, |ui| {
        ui.scope_builder(
            UiBuilder::new().accessibility_role(Role::ScrollView),
            |ui| {
                add(ui);
                // The visible part: the clip rectangle (the window cut by every scroll area
                // above) cut to the area's own horizontal extent, since for a vertical scroll
                // area the clip rectangle spans the parent's full width (T-057 F1).
                let clip = ui.clip_rect();
                let own = ui.max_rect().x_range();
                let visible =
                    egui::Rect::from_x_y_ranges(clip.x_range().intersection(own), clip.y_range());
                ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
                    node.set_bounds(accesskit_rect(visible));
                });
            },
        );
    });
    (output.state.clamped_offset().y, output.inner_rect)
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
            node.set_bounds(accesskit_rect(bounds));
        });
    });
}

/// A rectangle as the accessibility tree's.
pub(crate) fn accesskit_rect(r: egui::Rect) -> egui::accesskit::Rect {
    egui::accesskit::Rect {
        x0: r.min.x.into(),
        y0: r.min.y.into(),
        x1: r.max.x.into(),
        y1: r.max.y.into(),
    }
}
