//! The project tree panel (T-057, `docs/protocol/ui-project.md`), placed by the shell beside the
//! editor, on its left (see `super`).
//!
//! - A toggle button `Project` (`toggled` as the panel is expanded). It starts collapsed.
//! - When expanded, a tree `Project files` inside its own scroll area: one entry (role
//!   `tree_item`, label = the root-relative path, value and painted text = the name) per shown
//!   entry, in `project.tree`'s preorder restricted to what is shown; a directory has `toggled`
//!   (expanded or not), other kinds have none. A read that answered `truncated: true` adds a
//!   marker (role `label`, value beginning `truncated`) after the last entry shown below the
//!   directory it listed.
//! - A click on the toggle or on a directory that expands it sends one read, `project.tree {}`
//!   for the root or `project.tree {"path": <dir>}`, and shows that read's entries one segment
//!   below it, collapsed; a click that collapses sends nothing and forgets the expanded state of
//!   everything below. A click on any other entry sends `doc.open {"path": <path>}`. A refusal
//!   leaves the panel as it was and sets `Message` to begin with its kind; a successful open
//!   clears `Message`.
//!
//! Nothing is sent on any other frame: the panel reads only in the frame that handles a click
//! (no watching of the disk). What it shows between frames is kept here, in the toolkit's
//! per-id memory, never read from anywhere but those reads' results.

use std::collections::BTreeMap;

use egui::accesskit::Role;
use egui::epaint::PathShape;
use egui::{
    Align, Button, Id, Layout, Response, Sense, Stroke, TextStyle, TextWrapMode, Ui, UiBuilder,
    WidgetInfo, WidgetText, pos2, vec2,
};
use serde_json::{Value, json};

use super::PanelView;
use crate::shell::{accesskit_rect, scroll_view};

/// The panel's width when expanded, at most (points); never more than a quarter of the space
/// beside the editor.
const MAX_WIDTH: f32 = 240.0;
/// The panel's width when expanded, at least (points), unless the space is smaller.
const MIN_WIDTH: f32 = 120.0;
/// The indentation of one level of the tree (points).
const INDENT: f32 = 12.0;
/// The space before an entry's name, where a directory's arrow is drawn (points).
const ARROW: f32 = 12.0;
/// The marker after a truncated listing (its value begins with `truncated`).
const TRUNCATED: &str = "truncated listing";

/// One listed entry of a directory.
#[derive(Clone, Debug)]
struct Entry {
    /// Root-relative, as `project.tree` reported it.
    path: String,
    /// The last segment of `path`.
    name: String,
    dir: bool,
}

/// What one read listed one segment below the directory it was asked for.
#[derive(Clone, Debug, Default)]
struct Listing {
    entries: Vec<Entry>,
    truncated: bool,
}

/// The panel's state between frames: the root's listing while the panel is expanded, and the
/// listing of every expanded directory by its path.
#[derive(Clone, Debug, Default)]
struct State {
    root: Option<Listing>,
    dirs: BTreeMap<String, Listing>,
}

/// What a click asked for.
enum Clicked {
    Toggle,
    Entry(Entry),
}

/// The id under which the panel keeps its state.
fn state_id() -> Id {
    Id::unique("kiln-project-panel")
}

/// Draw the panel into `ui`; a click is handled in the frame that reports it.
pub(crate) fn show(ui: &mut Ui, view: &mut PanelView<'_>) {
    let mut state: State = ui
        .ctx()
        .data_mut(|d| d.get_temp(state_id()))
        .unwrap_or_default();
    let mut clicked = None;
    if state.root.is_some() {
        let width = (ui.available_width() * 0.25)
            .clamp(MIN_WIDTH, MAX_WIDTH)
            .min(ui.available_width());
        let size = vec2(width, ui.available_height());
        ui.allocate_ui_with_layout(size, Layout::top_down(Align::Min), |ui| {
            ui.set_min_size(size);
            if was_clicked(&toggle(ui, true)) {
                clicked = Some(Clicked::Toggle);
            }
            scroll_view(ui, "kiln-project-scroll", |ui| {
                tree(ui, |ui| {
                    if let Some(root) = &state.root {
                        listing(ui, &state, root, None, 0, &mut clicked);
                    }
                });
            });
        });
    } else {
        ui.with_layout(Layout::top_down(Align::Min), |ui| {
            if was_clicked(&toggle(ui, false)) {
                clicked = Some(Clicked::Toggle);
            }
        });
    }
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

/// The toggle: a button whose only text is `Project`, `toggled` as the panel is expanded.
fn toggle(ui: &mut Ui, expanded: bool) -> Response {
    ui.add(Button::new("Project").selected(expanded))
}

/// A node of role `tree` labelled `Project files` whose children are the nodes `add` makes.
fn tree(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    let builder = UiBuilder::new()
        .accessibility_role(Role::Tree)
        .accessibility_label("Project files");
    ui.scope_builder(builder, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        add(ui);
        let bounds = ui.min_rect();
        ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
            node.set_bounds(accesskit_rect(bounds));
        });
    });
}

/// The entries of `list` at `depth`, each expanded directory followed by its own, then the
/// marker if the read that made `list` was truncated.
fn listing(
    ui: &mut Ui,
    state: &State,
    list: &Listing,
    owner: Option<&str>,
    depth: usize,
    clicked: &mut Option<Clicked>,
) {
    for entry in &list.entries {
        let below = entry.dir.then(|| state.dirs.get(&entry.path)).flatten();
        let response = entry_row(ui, entry, depth, entry.dir.then_some(below.is_some()));
        if was_clicked(&response) {
            *clicked = Some(Clicked::Entry(entry.clone()));
        }
        if let Some(below) = below {
            listing(ui, state, below, Some(&entry.path), depth + 1, clicked);
        }
    }
    if list.truncated {
        marker(ui, owner, depth);
    }
}

/// One row: its rectangle (the full width, the text's height at least) and the laid-out name,
/// painted at `depth` after the arrow's space.
fn row(
    ui: &mut Ui,
    depth: usize,
    text: &str,
) -> (egui::Rect, egui::Pos2, std::sync::Arc<egui::Galley>) {
    let galley = WidgetText::from(text).into_galley(
        ui,
        Some(TextWrapMode::Extend),
        f32::INFINITY,
        TextStyle::Body,
    );
    let height = galley.size().y.max(ui.spacing().interact_size.y);
    let (_, rect) = ui.allocate_space(vec2(ui.available_width(), height));
    let left = rect.left() + depth as f32 * INDENT + ARROW;
    let top = rect.center().y - galley.size().y / 2.0;
    (rect, pos2(left, top), galley)
}

/// An entry: a clickable row of role `tree_item` (label = path, value = name; `toggled` for a
/// directory: `Some(expanded)`).
fn entry_row(ui: &mut Ui, entry: &Entry, depth: usize, toggled: Option<bool>) -> Response {
    let (rect, at, galley) = row(ui, depth, &entry.name);
    let id = Id::unique("kiln-project-entry").with(entry.path.as_str());
    let response = ui.interact(rect, id, Sense::click());
    response.widget_info(|| match toggled {
        Some(expanded) => WidgetInfo::selected(Role::TreeItem, true, expanded, &entry.name),
        None => WidgetInfo::labeled(Role::TreeItem, true, &entry.name),
    });
    ui.ctx().accesskit_node_builder(id, |node| {
        node.set_label(entry.path.as_str());
        node.set_value(entry.name.as_str());
    });
    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        if response.hovered() {
            ui.painter()
                .rect_filled(rect, visuals.corner_radius, visuals.weak_bg_fill);
        }
        if let Some(expanded) = toggled {
            arrow(ui, at, galley.size().y, expanded, visuals.text_color());
        }
        ui.painter().galley(at, galley, visuals.text_color());
    }
    response
}

/// A directory's arrow, drawn as a shape (never as text, so that the row's text is the name):
/// pointing right when collapsed, down when expanded, in the space just left of `at`.
fn arrow(ui: &Ui, at: egui::Pos2, height: f32, expanded: bool, color: egui::Color32) {
    let c = pos2(at.x - ARROW / 2.0, at.y + height / 2.0);
    let r = 3.5;
    let points = if expanded {
        vec![
            pos2(c.x - r, c.y - r / 2.0),
            pos2(c.x + r, c.y - r / 2.0),
            pos2(c.x, c.y + r),
        ]
    } else {
        vec![
            pos2(c.x - r / 2.0, c.y - r),
            pos2(c.x + r, c.y),
            pos2(c.x - r / 2.0, c.y + r),
        ]
    };
    ui.painter()
        .add(PathShape::convex_polygon(points, color, Stroke::NONE));
}

/// The marker after a truncated listing of `owner` (`None`: the root): a node of role `label`
/// whose value is its text.
fn marker(ui: &mut Ui, owner: Option<&str>, depth: usize) {
    let (rect, at, galley) = row(ui, depth, TRUNCATED);
    let id = Id::unique("kiln-project-truncated").with(owner);
    let response = ui.interact(rect, id, Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, TRUNCATED));
    if ui.is_rect_visible(rect) {
        let color = ui.visuals().weak_text_color();
        ui.painter().galley(at, galley, color);
    }
}

/// Handle a click: expand (one read), collapse (nothing sent) or open (`doc.open`).
fn handle(state: &mut State, click: Clicked, view: &mut PanelView<'_>) {
    match click {
        Clicked::Toggle if state.root.is_some() => *state = State::default(),
        Clicked::Toggle => {
            if let Some(list) = read(view, None) {
                state.root = Some(list);
            }
        }
        Clicked::Entry(entry) if entry.dir => {
            let expanded = state.dirs.contains_key(&entry.path);
            if expanded {
                collapse(&mut state.dirs, &entry.path);
            } else if let Some(list) = read(view, Some(&entry.path)) {
                state.dirs.insert(entry.path, list);
            }
        }
        Clicked::Entry(entry) => match view.request("doc.open", json!({ "path": entry.path })) {
            Ok(_) => view.set_message(String::new()),
            Err((kind, message)) => view.set_message(format!("{kind}: {message}")),
        },
    }
}

/// Forget the listing of `dir` and of every directory below it.
fn collapse(dirs: &mut BTreeMap<String, Listing>, dir: &str) {
    let below = format!("{dir}/");
    dirs.retain(|path, _| path != dir && !path.starts_with(&below));
}

/// Read the directory `dir` (the root for `None`) with one `project.tree` of the default limit;
/// its entries one segment below it. A refusal sets `Message` and gives `None`.
fn read(view: &mut PanelView<'_>, dir: Option<&str>) -> Option<Listing> {
    let params = match dir {
        Some(path) => json!({ "path": path }),
        None => json!({}),
    };
    match view.read("project.tree", params) {
        Ok(result) => Some(listing_of(&result, dir)),
        Err((kind, message)) => {
            view.set_message(format!("{kind}: {message}"));
            None
        }
    }
}

/// The entries of a `project.tree` result one segment below `dir`, in the result's order.
fn listing_of(result: &Value, dir: Option<&str>) -> Listing {
    let prefix = dir.map(|d| format!("{d}/")).unwrap_or_default();
    let entries = result["entries"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|e| {
            let path = e["path"].as_str()?;
            let name = path.strip_prefix(prefix.as_str())?;
            if name.is_empty() || name.contains('/') {
                return None;
            }
            Some(Entry {
                path: path.to_owned(),
                name: name.to_owned(),
                dir: e["kind"] == "dir",
            })
        })
        .collect();
    Listing {
        entries,
        truncated: result["truncated"] == Value::Bool(true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(entries: &[(&str, &str)], truncated: bool) -> Value {
        let entries: Vec<Value> = entries
            .iter()
            .map(|(kind, path)| json!({"kind": kind, "path": path}))
            .collect();
        json!({"entries": entries, "truncated": truncated})
    }

    #[test]
    fn a_listing_holds_the_entries_one_segment_below() {
        let r = result(
            &[
                ("dir", "src"),
                ("file", "src/a.rs"),
                ("dir", "src/sub"),
                ("file", "src/sub/b.rs"),
                ("symlink", "src/l"),
            ],
            false,
        );
        let paths = |l: &Listing| -> Vec<(String, String, bool)> {
            l.entries
                .iter()
                .map(|e| (e.path.clone(), e.name.clone(), e.dir))
                .collect()
        };
        assert_eq!(
            paths(&listing_of(&r, None)),
            [("src".into(), "src".into(), true)]
        );
        assert_eq!(
            paths(&listing_of(&r, Some("src"))),
            [
                ("src/a.rs".into(), "a.rs".into(), false),
                ("src/sub".into(), "sub".into(), true),
                ("src/l".into(), "l".into(), false),
            ]
        );
        // A name that only begins like the directory is not below it.
        let r = result(&[("file", "srcx/a"), ("file", "src/b")], true);
        let l = listing_of(&r, Some("src"));
        assert_eq!(paths(&l), [("src/b".into(), "b".into(), false)]);
        assert!(l.truncated);
    }

    #[test]
    fn collapsing_forgets_everything_below() {
        let mut dirs = BTreeMap::new();
        for d in ["a", "a/b", "a/b/c", "a b", "ab", "z"] {
            dirs.insert(d.to_owned(), Listing::default());
        }
        collapse(&mut dirs, "a");
        let left: Vec<&str> = dirs.keys().map(String::as_str).collect();
        assert_eq!(left, ["a b", "ab", "z"]);
    }
}
