//! What a frame shows, computed from what the toolkit produced for it (ADR-0008 decision 2):
//! the accessibility tree egui emitted (`PlatformOutput::accesskit_update`) and the text shapes
//! it painted (`FullOutput::shapes`). Nothing here reads a description the UI writes of itself.
//!
//! - Nodes are the accessibility nodes reachable from the root, in the toolkit's child order.
//!   Text-run nodes (role `text_run`, egui's per-row pieces of a text widget) are not reported
//!   as nodes: their text is the parent's `value` and `rows`, and the parent's `selection` is
//!   computed from them.
//! - A node's rectangle is its bounds, or, without bounds, the union of its children's.
//! - Each painted glyph whose centre lies inside its clip rectangle and the window belongs to
//!   the deepest node whose visible part (its rectangle cut by the window and every
//!   `scroll_view` above it) contains that centre (the last one in tree order among
//!   equals; the root when none does). A node's `rows` are its glyphs grouped by painted row,
//!   top to bottom, each row left to right in runs of one colour.

use std::collections::HashMap;

use egui::accesskit::{self, NodeId, Role, Toggled, TreeUpdate};
use egui::epaint::{ClippedShape, Color32, Shape, TextShape};
use egui::{Pos2, Rect};
use serde_json::{Map, Value, json};

/// A painted character: where it is and the colour it was painted with.
#[derive(Clone, Debug)]
pub(crate) struct Glyph {
    pub(crate) chr: char,
    /// Left and right edge.
    pub(crate) x0: f32,
    pub(crate) x1: f32,
    /// Vertical middle of its painted row.
    pub(crate) y: f32,
    pub(crate) color: [u8; 3],
}

/// One painted row of a node.
#[derive(Clone, Debug)]
pub(crate) struct Row {
    top: f32,
    bottom: f32,
    pub(crate) glyphs: Vec<Glyph>,
}

/// One node of the observed tree.
#[derive(Debug)]
pub(crate) struct Node {
    pub(crate) id: NodeId,
    pub(crate) role: Role,
    pub(crate) label: Option<String>,
    pub(crate) value: Option<String>,
    pub(crate) focused: bool,
    pub(crate) disabled: bool,
    pub(crate) read_only: bool,
    pub(crate) offscreen: bool,
    pub(crate) selected: Option<bool>,
    pub(crate) toggled: Option<bool>,
    pub(crate) selection: Option<(usize, usize)>,
    pub(crate) rows: Vec<Row>,
    pub(crate) rect: Option<Rect>,
    /// The visible part: `rect` cut by the window and every `scroll_view` above the node
    /// (`None` without a rectangle or when nothing of it is visible).
    pub(crate) visible: Option<Rect>,
    pub(crate) parent: Option<usize>,
    pub(crate) children: Vec<usize>,
    depth: usize,
}

impl Node {
    /// Whether typed text goes into this node: a text-input role, neither read-only nor
    /// disabled.
    pub(crate) fn takes_text(&self) -> bool {
        is_text_input(self.role) && !self.read_only && !self.disabled
    }
}

/// The observed tree of one frame; `nodes[0]` is the root.
#[derive(Debug, Default)]
pub(crate) struct Observation {
    pub(crate) nodes: Vec<Node>,
}

impl Observation {
    /// Build the observation of a frame of size `size` from the toolkit's output.
    pub(crate) fn new(update: Option<&TreeUpdate>, shapes: &[ClippedShape], size: Rect) -> Self {
        let Some(update) = update else {
            return Observation::default();
        };
        let by_id: HashMap<NodeId, &accesskit::Node> =
            update.nodes.iter().map(|(id, n)| (*id, n)).collect();
        let Some(root) = update.tree.as_ref().map(|t| t.root) else {
            return Observation::default();
        };
        let mut obs = Observation::default();
        let mut visiting = Vec::new();
        obs.add(&by_id, root, None, 0, update.focus, &mut visiting);
        obs.compute_rects(size);
        obs.compute_offscreen(size);
        obs.attribute_glyphs(shapes, size);
        obs
    }

    /// Add `id` and its descendants in pre-order (each node at most once).
    fn add(
        &mut self,
        by_id: &HashMap<NodeId, &accesskit::Node>,
        id: NodeId,
        parent: Option<usize>,
        depth: usize,
        focus: NodeId,
        visiting: &mut Vec<NodeId>,
    ) {
        let Some(node) = by_id.get(&id) else {
            return;
        };
        if node.role() == Role::TextRun || visiting.contains(&id) {
            return;
        }
        visiting.push(id);
        let index = self.nodes.len();
        let label = accessible_name(by_id, node);
        let value = node.value().map(str::to_owned);
        let selection = node
            .text_selection()
            .and_then(|sel| text_selection(by_id, node, sel));
        self.nodes.push(Node {
            id,
            role: node.role(),
            label,
            value,
            focused: id == focus && parent.is_some(),
            disabled: node.is_disabled(),
            read_only: node.is_read_only(),
            offscreen: false,
            visible: None,
            selected: node.is_selected(),
            toggled: match node.toggled() {
                Some(Toggled::True) => Some(true),
                Some(Toggled::False) => Some(false),
                Some(Toggled::Mixed) | None => None,
            },
            selection,
            rows: Vec::new(),
            rect: node.bounds().map(|b| {
                Rect::from_min_max(
                    Pos2::new(b.x0 as f32, b.y0 as f32),
                    Pos2::new(b.x1 as f32, b.y1 as f32),
                )
            }),
            parent,
            children: Vec::new(),
            depth,
        });
        if let Some(p) = parent {
            self.nodes[p].children.push(index);
        }
        for child in node.children() {
            self.add(by_id, *child, Some(index), depth + 1, focus, visiting);
        }
        visiting.pop();
    }

    /// Nodes without bounds get the union of their children's rectangles (the root: the
    /// window).
    fn compute_rects(&mut self, size: Rect) {
        for i in (0..self.nodes.len()).rev() {
            if self.nodes[i].rect.is_some() {
                continue;
            }
            let union = self.nodes[i]
                .children
                .iter()
                .filter_map(|c| self.nodes[*c].rect)
                .reduce(|a, b| a.union(b));
            self.nodes[i].rect = union;
        }
        if let Some(root) = self.nodes.first_mut() {
            root.rect = Some(size);
        }
    }

    /// The visible part of the window around node `i`: the window cut by the bounds of every
    /// `scroll_view` ancestor.
    fn visible_part(&self, i: usize, size: Rect) -> Rect {
        let mut visible = size;
        let mut at = self.nodes[i].parent;
        while let Some(p) = at {
            if self.nodes[p].role == Role::ScrollView
                && let Some(r) = self.nodes[p].rect
            {
                visible = visible.intersect(r);
            }
            at = self.nodes[p].parent;
        }
        visible
    }

    fn compute_offscreen(&mut self, size: Rect) {
        for i in 0..self.nodes.len() {
            let Some(rect) = self.nodes[i].rect else {
                continue;
            };
            let visible = self.visible_part(i, size);
            let shown = overlaps(rect, visible);
            self.nodes[i].offscreen = !shown;
            self.nodes[i].visible = shown.then(|| rect.intersect(visible));
        }
    }

    fn attribute_glyphs(&mut self, shapes: &[ClippedShape], size: Rect) {
        let mut painted = Vec::new();
        for clipped in shapes {
            let clip = clipped.clip_rect.intersect(size);
            collect_text(
                &clipped.shape,
                clip,
                clipped.transform_after_tessellation,
                &mut painted,
            );
        }
        // Painted rows of every node, in paint order.
        let mut per_node: Vec<Vec<Row>> = vec![Vec::new(); self.nodes.len()];
        for row in painted {
            let mut by_owner: Vec<(usize, Row)> = Vec::new();
            for glyph in row.glyphs {
                let owner = self.owner(Pos2::new((glyph.x0 + glyph.x1) / 2.0, glyph.y));
                match by_owner.iter_mut().find(|(o, _)| *o == owner) {
                    Some((_, r)) => r.glyphs.push(glyph),
                    None => by_owner.push((
                        owner,
                        Row {
                            top: row.top,
                            bottom: row.bottom,
                            glyphs: vec![glyph],
                        },
                    )),
                }
            }
            for (owner, r) in by_owner {
                per_node[owner].push(r);
            }
        }
        for (node, rows) in self.nodes.iter_mut().zip(per_node) {
            node.rows = merge_rows(rows);
        }
    }

    /// The deepest node whose visible part contains `p` (last in tree order among equals; the
    /// root when none does).
    fn owner(&self, p: Pos2) -> usize {
        let mut best = 0;
        let mut best_depth = 0;
        for (i, node) in self.nodes.iter().enumerate().skip(1) {
            if node.depth >= best_depth && node.visible.is_some_and(|r| contains(r, p)) {
                best = i;
                best_depth = node.depth;
            }
        }
        best
    }

    /// The nearest node at or above `i` with role `scroll_view`.
    pub(crate) fn scroll_view_around(&self, i: usize) -> Option<usize> {
        let mut at = Some(i);
        while let Some(n) = at {
            if self.nodes[n].role == Role::ScrollView {
                return Some(n);
            }
            at = self.nodes[n].parent;
        }
        None
    }

    /// The focused node, if any.
    pub(crate) fn focused(&self) -> Option<usize> {
        self.nodes.iter().position(|n| n.focused)
    }

    /// The node `i` as the snapshot reports it.
    pub(crate) fn to_json(&self, i: usize) -> Value {
        let node = &self.nodes[i];
        let mut out = Map::new();
        out.insert("role".into(), Value::String(role_name(node.role)));
        if let Some(label) = &node.label {
            out.insert("label".into(), Value::String(label.clone()));
        }
        if let Some(value) = &node.value {
            out.insert("value".into(), Value::String(value.clone()));
        }
        for (key, on) in [
            ("focused", node.focused),
            ("disabled", node.disabled),
            ("offscreen", node.offscreen),
        ] {
            if on {
                out.insert(key.into(), Value::Bool(true));
            }
        }
        if let Some(selected) = node.selected {
            out.insert("selected".into(), Value::Bool(selected));
        }
        if let Some(toggled) = node.toggled {
            out.insert("toggled".into(), Value::Bool(toggled));
        }
        if let Some((anchor, head)) = node.selection {
            out.insert("selection".into(), json!({"anchor": anchor, "head": head}));
        }
        if !node.rows.is_empty() {
            let rows = node.rows.iter().map(row_json).collect();
            out.insert("rows".into(), Value::Array(rows));
        }
        let children = node.children.iter().map(|c| self.to_json(*c)).collect();
        out.insert("children".into(), Value::Array(children));
        Value::Object(out)
    }
}

/// A row as runs `{"text","color"}`, adjacent runs of one colour merged.
fn row_json(row: &Row) -> Value {
    let mut runs: Vec<(String, [u8; 3])> = Vec::new();
    for g in &row.glyphs {
        match runs.last_mut() {
            Some((text, color)) if *color == g.color => text.push(g.chr),
            _ => runs.push((g.chr.to_string(), g.color)),
        }
    }
    Value::Array(
        runs.into_iter()
            .map(|(text, [r, g, b])| json!({"text": text, "color": format!("#{r:02x}{g:02x}{b:02x}")}))
            .collect(),
    )
}

/// Painted rows of one node, merged when they share a line (a row whose vertical middle lies
/// inside another's span joins it), sorted top to bottom and each left to right.
fn merge_rows(rows: Vec<Row>) -> Vec<Row> {
    let mut merged: Vec<Row> = Vec::new();
    for row in rows {
        let middle = (row.top + row.bottom) / 2.0;
        match merged
            .iter_mut()
            .find(|m| m.top <= middle && middle < m.bottom)
        {
            Some(m) => m.glyphs.extend(row.glyphs),
            None => merged.push(row),
        }
    }
    merged.sort_by(|a, b| a.top.total_cmp(&b.top));
    for row in &mut merged {
        row.glyphs.sort_by(|a, b| a.x0.total_cmp(&b.x0));
    }
    merged
}

/// The painted rows of every text shape in `shape`, restricted to glyphs whose centre lies in
/// `clip`.
fn collect_text(
    shape: &Shape,
    clip: Rect,
    transform: egui::emath::TSTransform,
    out: &mut Vec<Row>,
) {
    match shape {
        Shape::Vec(shapes) => {
            for s in shapes {
                collect_text(s, clip, transform, out);
            }
        }
        Shape::Text(text) => text_rows(text, clip, transform, out),
        _ => {}
    }
}

fn text_rows(
    text: &TextShape,
    clip: Rect,
    transform: egui::emath::TSTransform,
    out: &mut Vec<Row>,
) {
    if text.opacity_factor <= 0.0 || text.angle != 0.0 {
        return;
    }
    for placed in &text.galley.rows {
        let origin = text.pos + placed.pos.to_vec2();
        let top = transform.mul_pos(origin).y;
        let bottom = transform.mul_pos(origin + egui::vec2(0.0, placed.size.y)).y;
        let colors = glyph_colors(text, placed);
        let mut glyphs = Vec::new();
        for (glyph, color) in placed.glyphs.iter().zip(colors) {
            let left = transform.mul_pos(origin + egui::vec2(glyph.pos.x, 0.0)).x;
            let right = transform
                .mul_pos(origin + egui::vec2(glyph.pos.x + glyph.advance_width, 0.0))
                .x;
            let centre = Pos2::new((left + right) / 2.0, (top + bottom) / 2.0);
            if !contains(clip, centre) || color.a() == 0 {
                continue;
            }
            let [r, g, b, _] = color.to_srgba_unmultiplied();
            glyphs.push(Glyph {
                chr: glyph.chr,
                x0: left,
                x1: right,
                y: centre.y,
                color: [r, g, b],
            });
        }
        if !glyphs.is_empty() {
            out.push(Row {
                top,
                bottom,
                glyphs,
            });
        }
    }
}

/// The colour each glyph of `row` was painted with, as the tessellator computes it from the
/// row's mesh (override and fallback colour, opacity). A glyph that paints no pixels (a space)
/// takes the colour of the glyph before it in the row, or, at the start of the row, of the
/// first glyph after it that paints pixels.
fn glyph_colors(text: &TextShape, row: &egui::epaint::text::PlacedRow) -> Vec<Color32> {
    let mesh = &row.visuals.mesh;
    let painted: Vec<Option<Color32>> = row
        .glyphs
        .iter()
        .map(|g| {
            if g.uv_rect.is_nothing() {
                return None;
            }
            let vertex = g.first_vertex as usize;
            let mut color = mesh.vertices.get(vertex)?.color;
            if let Some(over) = text.override_text_color {
                if row.visuals.glyph_vertex_range.contains(&vertex) {
                    color = over;
                }
            } else if color == Color32::PLACEHOLDER {
                color = text.fallback_color;
            }
            if g.is_color {
                color = Color32::from_white_alpha(color.a());
            }
            if text.opacity_factor < 1.0 {
                color = color.gamma_multiply(text.opacity_factor);
            }
            Some(color)
        })
        .collect();
    let default = painted
        .iter()
        .flatten()
        .next()
        .copied()
        .unwrap_or(text.override_text_color.unwrap_or(text.fallback_color));
    let mut previous = default;
    painted
        .into_iter()
        .map(|c| {
            let c = c.unwrap_or(previous);
            previous = c;
            c
        })
        .collect()
}

/// The accessible name: the node's label, else the names of the nodes it is labelled by.
fn accessible_name(
    by_id: &HashMap<NodeId, &accesskit::Node>,
    node: &accesskit::Node,
) -> Option<String> {
    if let Some(label) = node.label().filter(|l| !l.is_empty()) {
        return Some(label.to_owned());
    }
    let names: Vec<&str> = node
        .labelled_by()
        .iter()
        .filter_map(|id| by_id.get(id))
        .filter_map(|n| n.label().or_else(|| n.value()))
        .filter(|s| !s.is_empty())
        .collect();
    (!names.is_empty()).then(|| names.join(" "))
}

/// A text selection in chars of the node's text, from positions in its text-run children.
fn text_selection(
    by_id: &HashMap<NodeId, &accesskit::Node>,
    node: &accesskit::Node,
    sel: &accesskit::TextSelection,
) -> Option<(usize, usize)> {
    let mut offsets = HashMap::new();
    let mut at = 0usize;
    for child in node.children() {
        let Some(run) = by_id.get(child).filter(|n| n.role() == Role::TextRun) else {
            continue;
        };
        offsets.insert(*child, at);
        at += run.character_lengths().len();
    }
    let offset = |p: &accesskit::TextPosition| offsets.get(&p.node).map(|o| o + p.character_index);
    Some((offset(&sel.anchor)?, offset(&sel.focus)?))
}

/// Roles whose nodes take typed text.
fn is_text_input(role: Role) -> bool {
    matches!(
        role,
        Role::TextInput
            | Role::MultilineTextInput
            | Role::SearchInput
            | Role::PasswordInput
            | Role::EmailInput
            | Role::NumberInput
            | Role::PhoneNumberInput
            | Role::UrlInput
            | Role::EditableComboBox
    )
}

/// The role in snake_case (`TextInput` → `text_input`).
pub(crate) fn role_name(role: Role) -> String {
    let camel = format!("{role:?}");
    let mut out = String::with_capacity(camel.len() + 4);
    for (i, c) in camel.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// `p` inside `r` (left and top edges included, right and bottom excluded).
fn contains(r: Rect, p: Pos2) -> bool {
    r.min.x <= p.x && p.x < r.max.x && r.min.y <= p.y && p.y < r.max.y
}

/// Whether some part of `a` lies in `b` (a rectangle of zero width or height counts when it
/// lies on or inside `b`).
fn overlaps(a: Rect, b: Rect) -> bool {
    let w = a.max.x.min(b.max.x) - a.min.x.max(b.min.x);
    let h = a.max.y.min(b.max.y) - a.min.y.max(b.min.y);
    let wide = w > 0.0 || (w == 0.0 && a.width() == 0.0);
    let tall = h > 0.0 || (h == 0.0 && a.height() == 0.0);
    wide && tall
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_names_are_snake_case() {
        assert_eq!(role_name(Role::TextInput), "text_input");
        assert_eq!(role_name(Role::Button), "button");
        assert_eq!(role_name(Role::MultilineTextInput), "multiline_text_input");
    }

    #[test]
    fn overlap_counts_degenerate_rects_inside() {
        let window = Rect::from_min_max(Pos2::ZERO, Pos2::new(100.0, 100.0));
        let empty_line = Rect::from_min_max(Pos2::new(10.0, 10.0), Pos2::new(10.0, 20.0));
        let below = Rect::from_min_max(Pos2::new(0.0, 100.0), Pos2::new(10.0, 120.0));
        assert!(overlaps(empty_line, window));
        assert!(!overlaps(below, window));
    }

    #[test]
    fn rows_merge_runs_of_one_colour() {
        let g = |chr, x0: f32, color| Glyph {
            chr,
            x0,
            x1: x0 + 1.0,
            y: 0.5,
            color,
        };
        let row = Row {
            top: 0.0,
            bottom: 1.0,
            glyphs: vec![
                g('f', 0.0, [200, 0, 0]),
                g('n', 1.0, [200, 0, 0]),
                g(' ', 2.0, [0, 0, 200]),
            ],
        };
        assert_eq!(
            row_json(&row),
            json!([{"text": "fn", "color": "#c80000"}, {"text": " ", "color": "#0000c8"}])
        );
    }
}
