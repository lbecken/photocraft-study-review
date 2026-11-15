//! The tabs panel (T-058, `docs/protocol/ui-tabs.md`), placed by the shell above the editor (see
//! `super`).
//!
//! - With at least one document open: a list of role `tab_list` labelled `Tabs`, inside its own
//!   scroll area of at most [`MAX_ROWS`] rows (and more than one scroll step). With none open
//!   the panel shows nothing.
//! - One tab per open document, in id order: a clickable node of role `tab` whose label is the
//!   document's path and whose painted text is the path; no value; `toggled` (selected) exactly
//!   for the current document.
//! - After each tab: a label `modified` when the last `doc.list` read reported the document
//!   dirty; a button `Close <path>` whose mark is a drawn cross (no text); and, after a close of
//!   that document was refused with `unsaved_changes` and until it is closed or a `doc.list` read
//!   reports it clean, a button `Discard <path>`.
//! - A click on a tab makes its document current ([`PanelView::set_current`]: no request); a
//!   click on `Close <path>` sends `doc.close {"doc": id}`, on `Discard <path>`
//!   `doc.close {"doc": id, "discard": true}`. A success clears `Message`; a refusal sets it to
//!   begin with its kind.
//!
//! The panel reads `doc.list` (read-only) in the first frame after a change
//! ([`PanelView::changes`] differs from what it last saw) while a document is open, and nothing
//! on any other frame. What it shows between frames is kept in the toolkit's per-id memory.

use std::collections::BTreeSet;

use egui::accesskit::Role;
use egui::{
    Button, Id, Label, Response, RichText, Sense, Stroke, TextStyle, TextWrapMode, Ui, UiBuilder,
    WidgetInfo, WidgetText, pos2, vec2,
};
use serde_json::{Value, json};

use super::PanelView;
use crate::shell::{accesskit_rect, scroll_view};

/// The strip's height, at most, in rows of tabs; it scrolls beyond that.
const MAX_ROWS: f32 = 5.0;
/// How far one `scroll` step moves a scroll area (points; the toolkit's distance for a scroll
/// action). The strip, when it scrolls, is taller than this by a row, so that a step down or up
/// never skips a row of tabs.
const SCROLL_STEP: f32 = 100.0;
/// The space left and right of a tab's path (points).
const PAD: f32 = 6.0;
/// The text of the node that follows the tab of a dirty document.
const MODIFIED: &str = "modified";

/// The panel's state between frames.
#[derive(Clone, Debug, Default)]
struct State {
    /// [`PanelView::changes`] at the last `doc.list` read (`None`: never read).
    seen: Option<u64>,
    /// The documents the last `doc.list` read reported dirty.
    dirty: BTreeSet<u64>,
    /// The documents whose close was refused with `unsaved_changes` (they show `Discard`).
    discard: BTreeSet<u64>,
    /// The height of the list's content in the last frame (points).
    content: Option<f32>,
}

/// What a click asked for.
enum Clicked {
    Tab(u64),
    Close(u64),
    Discard(u64),
}

/// The id under which the panel keeps its state.
fn state_id() -> Id {
    Id::unique("kiln-tabs-panel")
}

/// Draw the panel into `ui`; a click is handled in the frame that reports it.
pub(crate) fn show(ui: &mut Ui, view: &mut PanelView<'_>) {
    if view.open_documents().is_empty() {
        return;
    }
    let mut state: State = ui
        .ctx()
        .data_mut(|d| d.get_temp(state_id()))
        .unwrap_or_default();
    if state.seen != Some(view.changes()) {
        refresh(&mut state, view);
    }
    let open = view.open_documents().to_vec();
    state.discard.retain(|d| open.iter().any(|(id, _)| id == d));
    let current = view.current();

    let row = ui.spacing().interact_size.y + ui.spacing().item_spacing.y;
    let max = (row * MAX_ROWS).max(SCROLL_STEP + row);
    let height = state.content.unwrap_or(row).clamp(row, max);
    let mut clicked = None;
    let mut content = height;
    ui.allocate_ui(vec2(ui.available_width(), height), |ui| {
        ui.set_min_height(height);
        scroll_view(ui, "kiln-tabs-scroll", |ui| {
            content = tab_list(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (id, path) in &open {
                        let selected = current == Some(*id);
                        if was_clicked(&tab(ui, *id, path, selected)) {
                            clicked = Some(Clicked::Tab(*id));
                        }
                        if state.dirty.contains(id) {
                            ui.add(Label::new(RichText::new(MODIFIED).weak()).selectable(false));
                        }
                        if was_clicked(&close_button(ui, *id, path)) {
                            clicked = Some(Clicked::Close(*id));
                        }
                        if state.discard.contains(id) {
                            let discard = Button::new(format!("Discard {path}"))
                                .wrap_mode(TextWrapMode::Extend);
                            if was_clicked(&ui.add(discard)) {
                                clicked = Some(Clicked::Discard(*id));
                            }
                        }
                        ui.add_space(PAD);
                    }
                });
            });
        });
    });
    // The strip follows its content's height (up to `MAX_ROWS`) from the next frame.
    let wanted = content.clamp(row, max);
    if (wanted - height).abs() > 0.5 {
        ui.ctx().request_repaint();
    }
    state.content = Some(content);
    if let Some(click) = clicked {
        handle(&mut state, click, view);
        // What the click changed is drawn on the next frame.
        ui.ctx().request_repaint();
    }
    ui.ctx().data_mut(|d| d.insert_temp(state_id(), state));
}

/// A click of any button, with any modifiers (the panel distinguishes none).
fn was_clicked(response: &Response) -> bool {
    response.clicked() || response.secondary_clicked()
}

/// A node of role `tab_list` labelled `Tabs` whose children are the nodes `add` makes; the
/// height of what `add` laid out.
fn tab_list(ui: &mut Ui, add: impl FnOnce(&mut Ui)) -> f32 {
    let builder = UiBuilder::new()
        .accessibility_role(Role::TabList)
        .accessibility_label("Tabs");
    ui.scope_builder(builder, |ui| {
        add(ui);
        let bounds = ui.min_rect();
        ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
            node.set_bounds(accesskit_rect(bounds));
        });
        bounds.height()
    })
    .inner
}

/// A tab: a clickable rectangle of role `tab` (label = path, no value, `toggled` = `selected`)
/// in which the path is painted on one row (two for a path with a line break).
fn tab(ui: &mut Ui, doc: u64, path: &str, selected: bool) -> Response {
    let galley = WidgetText::from(path).into_galley(
        ui,
        Some(TextWrapMode::Extend),
        f32::INFINITY,
        TextStyle::Body,
    );
    let size = vec2(
        galley.size().x + 2.0 * PAD,
        galley.size().y.max(ui.spacing().interact_size.y),
    );
    let (_, rect) = ui.allocate_space(size);
    let id = Id::unique("kiln-tab").with(doc);
    let response = ui.interact(rect, id, Sense::click());
    response.widget_info(|| WidgetInfo::selected(Role::Tab, true, selected, path));
    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact_selectable(&response, selected);
        if selected || response.hovered() {
            ui.painter()
                .rect_filled(rect, visuals.corner_radius, visuals.weak_bg_fill);
        }
        let at = pos2(rect.left() + PAD, rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(at, galley, visuals.text_color());
    }
    response
}

/// A tab's close button: a square of role `button` labelled `Close <path>`, its mark a drawn
/// cross (so that it paints no text).
fn close_button(ui: &mut Ui, doc: u64, path: &str) -> Response {
    let side = ui.spacing().interact_size.y;
    let (_, rect) = ui.allocate_space(vec2(side, side));
    let id = Id::unique("kiln-tab-close").with(doc);
    let response = ui.interact(rect, id, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, format!("Close {path}")));
    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        if response.hovered() {
            ui.painter()
                .rect_filled(rect, visuals.corner_radius, visuals.weak_bg_fill);
        }
        let c = rect.center();
        let r = side / 5.0;
        let stroke = Stroke::new(1.5, visuals.fg_stroke.color);
        ui.painter()
            .line_segment([pos2(c.x - r, c.y - r), pos2(c.x + r, c.y + r)], stroke);
        ui.painter()
            .line_segment([pos2(c.x - r, c.y + r), pos2(c.x + r, c.y - r)], stroke);
    }
    response
}

/// Read `doc.list` (read-only): which documents are dirty. A refused read is not shown.
fn refresh(state: &mut State, view: &mut PanelView<'_>) {
    state.seen = Some(view.changes());
    if let Ok(result) = view.read("doc.list", json!({})) {
        state.dirty = dirty_of(&result);
        let dirty = &state.dirty;
        state.discard.retain(|d| dirty.contains(d));
    }
}

/// The ids a `doc.list` result reports dirty.
fn dirty_of(result: &Value) -> BTreeSet<u64> {
    result["docs"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter(|d| d["dirty"] == Value::Bool(true))
        .filter_map(|d| d["doc"].as_u64())
        .collect()
}

/// Handle a click: switch (no request), close or close discarding.
fn handle(state: &mut State, click: Clicked, view: &mut PanelView<'_>) {
    let (doc, params) = match click {
        Clicked::Tab(doc) => {
            view.set_current(doc);
            return;
        }
        Clicked::Close(doc) => (doc, json!({ "doc": doc })),
        Clicked::Discard(doc) => (doc, json!({ "doc": doc, "discard": true })),
    };
    match view.request("doc.close", params) {
        Ok(_) => {
            state.discard.remove(&doc);
            view.set_message(String::new());
        }
        Err((kind, message)) => {
            if kind == "unsaved_changes" {
                state.discard.insert(doc);
            }
            view.set_message(format!("{kind}: {message}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirty_ids_come_from_the_list() {
        let r = json!({"docs": [
            {"doc": 1, "path": "a", "dirty": true},
            {"doc": 2, "path": "b", "dirty": false},
            {"doc": 5, "path": "c", "dirty": true},
        ]});
        assert_eq!(dirty_of(&r).into_iter().collect::<Vec<_>>(), [1, 5]);
        assert!(dirty_of(&json!({})).is_empty());
    }
}
