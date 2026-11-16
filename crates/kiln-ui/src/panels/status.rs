//! The status bar panel (T-059, `docs/protocol/ui-status.md`), placed by the shell below the
//! editor (see `super`; the shell gives it a bottom-up `Ui`).
//!
//! - With a current document: one row of four nodes of role `label`, with the accessible names
//!   `Line`, `Column`, `Encoding` and `Line ending`, each painting exactly its value on one row:
//!   `Ln <line + 1>` and `Col <col + 1>` of `cursor.get` (the selection's head; the column in
//!   chars), `doc.info`'s `encoding`, and `doc.info`'s `line_ending` followed by ` (mixed)` when
//!   `mixed_line_endings` is true. With no current document the panel shows nothing.
//! - The panel reads `cursor.get` and `doc.info` (read-only, [`PanelView::read`]) in the first
//!   frame after a change: when [`PanelView::changes`] or [`PanelView::current`] differs from what
//!   it last read for. It sends nothing on any other frame and nothing with no document open. A
//!   refused read leaves the bar empty (and `Message` unchanged).
//! - It is a display: it takes no focus and a click on it does nothing.
//!
//! What it shows between frames is kept in the toolkit's per-id memory.

use egui::{Id, Label, TextWrapMode, Ui};
use serde_json::{Value, json};

use super::PanelView;

/// The space between two of the bar's nodes (points).
const GAP: f32 = 16.0;

/// The four values the bar shows, in its order.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Shown {
    line: String,
    column: String,
    encoding: String,
    ending: String,
}

impl Shown {
    /// `(accessible name, painted text)` of each node, in order.
    fn nodes(&self) -> [(&'static str, &str); 4] {
        [
            ("Line", self.line.as_str()),
            ("Column", self.column.as_str()),
            ("Encoding", self.encoding.as_str()),
            ("Line ending", self.ending.as_str()),
        ]
    }
}

/// The panel's state between frames.
#[derive(Clone, Debug, Default)]
struct State {
    /// `(changes, current)` at the last read (`None`: never read).
    seen: Option<(u64, u64)>,
    /// What the last read gave (`None`: it was refused).
    shown: Option<Shown>,
}

/// The id under which the panel keeps its state.
fn state_id() -> Id {
    Id::unique("kiln-status-panel")
}

/// Draw the panel into `ui`: re-read after a change, then the row of four nodes.
pub(crate) fn show(ui: &mut Ui, view: &mut PanelView<'_>) {
    let Some(doc) = view.current() else {
        return;
    };
    let mut state: State = ui
        .ctx()
        .data_mut(|d| d.get_temp(state_id()))
        .unwrap_or_default();
    let key = (view.changes(), doc);
    if state.seen != Some(key) {
        state.seen = Some(key);
        state.shown = read(view, doc);
    }
    if let Some(shown) = &state.shown {
        ui.horizontal(|ui| {
            for (k, (name, text)) in shown.nodes().into_iter().enumerate() {
                if k > 0 {
                    ui.add_space(GAP);
                }
                let label = Label::new(text)
                    .wrap_mode(TextWrapMode::Extend)
                    .selectable(false);
                ui.add(label).accessible_name(name);
            }
        });
    }
    ui.ctx().data_mut(|d| d.insert_temp(state_id(), state));
}

/// Read `cursor.get` and `doc.info` of `doc` (read-only); `None` if either is refused.
fn read(view: &mut PanelView<'_>, doc: u64) -> Option<Shown> {
    let params = json!({ "doc": doc });
    let cursor = view.read("cursor.get", params.clone()).ok()?;
    let info = view.read("doc.info", params).ok()?;
    shown_of(&cursor, &info)
}

/// The bar's values from a `cursor.get` and a `doc.info` result.
fn shown_of(cursor: &Value, info: &Value) -> Option<Shown> {
    let line = cursor["line"].as_u64()?;
    let col = cursor["col"].as_u64()?;
    let encoding = info["encoding"].as_str()?;
    let ending = info["line_ending"].as_str()?;
    let mixed = info["mixed_line_endings"].as_bool()?;
    Some(Shown {
        line: format!("Ln {}", line + 1),
        column: format!("Col {}", col + 1),
        encoding: encoding.to_owned(),
        ending: if mixed {
            format!("{ending} (mixed)")
        } else {
            ending.to_owned()
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_one_based_and_mark_mixed_endings() {
        let cursor =
            json!({"doc": 1, "anchor": 0, "head": 17, "start": 0, "end": 17, "line": 1, "col": 5});
        let info = json!({"encoding": "utf-16le", "line_ending": "lf", "mixed_line_endings": true});
        let shown = shown_of(&cursor, &info).expect("values");
        assert_eq!(
            shown.nodes(),
            [
                ("Line", "Ln 2"),
                ("Column", "Col 6"),
                ("Encoding", "utf-16le"),
                ("Line ending", "lf (mixed)"),
            ]
        );
        let info = json!({"encoding": "utf-8", "line_ending": "none", "mixed_line_endings": false});
        let cursor = json!({"line": 0, "col": 0});
        let shown = shown_of(&cursor, &info).expect("values");
        assert_eq!(
            (
                shown.line.as_str(),
                shown.column.as_str(),
                shown.ending.as_str()
            ),
            ("Ln 1", "Col 1", "none")
        );
    }

    #[test]
    fn a_result_without_a_field_shows_nothing() {
        let info = json!({"encoding": "utf-8", "line_ending": "lf", "mixed_line_endings": false});
        assert!(shown_of(&json!({"line": 0}), &info).is_none());
        assert!(shown_of(&json!({"line": 0, "col": 0}), &json!({})).is_none());
    }
}
