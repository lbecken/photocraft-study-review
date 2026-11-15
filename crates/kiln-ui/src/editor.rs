//! The editor view (T-056, `docs/protocol/ui-editor.md`): the current document's text with
//! syntax colours, a caret and selection, and vertical scrolling; keys and clicks edit, move,
//! undo, redo and save it by dispatching registry commands through the [`Session`] only.
//!
//! What it shows it reads with read-only control requests (`doc.info`, `doc.text`,
//! `cursor.get`, `syntax.highlight` over the whole text), and only in a frame in which a request
//! (of any source, other than these reads) happened since its last read: on an idle frame it
//! sends nothing. In the frame in which it re-read it asks for one further frame. Scrolling needs
//! no read: the whole document was read.
//!
//! Layout: one row per document line (no wrap, no horizontal scroll), in the built-in monospace
//! font; a line is laid out only as far as just past the view's right edge.

use std::sync::Arc;
use std::time::Duration;

use egui::accesskit::{Action, Role, TextPosition, TextSelection};
use egui::text::{CCursor, LayoutJob, TextFormat};
use egui::{
    Color32, Context, Event, EventFilter, FontId, Galley, Id, Key, Modifiers, PointerButton, Pos2,
    Rect, Sense, Stroke, TextStyle, Ui, UiBuilder, Vec2, pos2, vec2,
};
use serde_json::{Value, json};

use crate::observe::{MIN_STEP, char_edges};
use crate::session::Session;
use crate::shell::{accesskit_rect, scroll_view_with};

/// The colour of each `syntax.highlight` kind (`docs/protocol/ui-editor.md`, T-056 Pinned, Q-8 A).
pub(crate) const KIND_COLOURS: &[(&str, Color32)] = &[
    ("plain", Color32::from_rgb(0xdc, 0xdc, 0xdc)),
    ("keyword", Color32::from_rgb(0x56, 0x9c, 0xd6)),
    ("type", Color32::from_rgb(0x4e, 0xc9, 0xb0)),
    ("function", Color32::from_rgb(0xdc, 0xdc, 0xaa)),
    ("string", Color32::from_rgb(0xce, 0x91, 0x78)),
    ("number", Color32::from_rgb(0xb5, 0xce, 0xa8)),
    ("comment", Color32::from_rgb(0x6a, 0x99, 0x55)),
];

/// The `plain` colour.
const PLAIN: Color32 = Color32::from_rgb(0xdc, 0xdc, 0xdc);
/// The background of selected text (a background only: text keeps its kind's colour).
const SELECTION: Color32 = Color32::from_rgb(0x26, 0x4f, 0x78);
/// Space left of the text, so that a caret at column 0 is visible.
const LEFT_MARGIN: f32 = 2.0;
/// The first number of chars of a line laid out; more are laid out while the row is narrower
/// than the view.
const FIRST_LAYOUT: usize = 256;

/// The colour of `kind` (`plain` for a kind the table does not name).
fn colour(kind: &str) -> Color32 {
    KIND_COLOURS
        .iter()
        .find(|(k, _)| *k == kind)
        .map_or(PLAIN, |(_, c)| *c)
}

/// What the view last read of the current document.
struct Doc {
    id: u64,
    path: String,
    text: String,
    chars: Vec<char>,
    /// Each line's content as `[start, end)` in chars (without its break).
    lines: Vec<(usize, usize)>,
    dirty: bool,
    line_ending: String,
    anchor: usize,
    head: usize,
    /// Highlight spans `(start, end, colour)` over the whole text (`None`: painted plain).
    spans: Option<Vec<(usize, usize, Color32)>>,
}

impl Doc {
    /// The line that holds offset `at`.
    fn line_of(&self, at: usize) -> usize {
        match self.lines.binary_search_by(|(start, _)| start.cmp(&at)) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        }
    }

    /// The colour the char at `at` is painted with.
    fn colour_at(&self, at: usize) -> Color32 {
        let Some(spans) = &self.spans else {
            return PLAIN;
        };
        match spans.binary_search_by(|(start, _, _)| start.cmp(&at)) {
            Ok(i) => spans[i].2,
            Err(0) => PLAIN,
            Err(i) => {
                let (_, end, c) = spans[i - 1];
                if at < end { c } else { PLAIN }
            }
        }
    }

    /// Where `cursor.move {by:"char", forward}` would put the head (a CRLF is one step).
    fn char_step(&self, forward: bool) -> usize {
        let (h, c) = (self.head, &self.chars);
        if forward {
            if h + 1 < c.len() && c[h] == '\r' && c[h + 1] == '\n' {
                h + 2
            } else {
                (h + 1).min(c.len())
            }
        } else if h >= 2 && c[h - 2] == '\r' && c[h - 1] == '\n' {
            h - 2
        } else {
            h.saturating_sub(1)
        }
    }

    /// `(start, end)` of the selection.
    fn selection(&self) -> (usize, usize) {
        (self.anchor.min(self.head), self.anchor.max(self.head))
    }

    /// The text `Enter` inserts: the document's line ending (`\n` for `none`).
    fn line_break(&self) -> &'static str {
        match self.line_ending.as_str() {
            "crlf" => "\r\n",
            "cr" => "\r",
            _ => "\n",
        }
    }
}

/// The lines of `chars`, split at `\r\n`, a lone `\r` and `\n` (`docs/protocol/doc-positions.md`),
/// as `[start, end)` content ranges.
fn split_lines(chars: &[char]) -> Vec<(usize, usize)> {
    let mut lines = Vec::new();
    let (mut start, mut i) = (0, 0);
    while i < chars.len() {
        match chars[i] {
            '\r' if chars.get(i + 1) == Some(&'\n') => {
                lines.push((start, i));
                i += 2;
                start = i;
            }
            '\r' | '\n' => {
                lines.push((start, i));
                i += 1;
                start = i;
            }
            _ => i += 1,
        }
    }
    lines.push((start, chars.len()));
    lines
}

/// Where the view's text was drawn in the last frame (screen coordinates).
#[derive(Clone, Copy)]
struct Geometry {
    /// Left edge of the text.
    left: f32,
    /// Top of line 0, the scroll offset applied.
    top: f32,
    row_height: f32,
    /// From `left` to the right edge of the visible part.
    width: f32,
}

/// The editor view's state.
#[derive(Default)]
pub(crate) struct EditorView {
    /// `Session::changes` at the last read (`None`: never read).
    seen: Option<u64>,
    /// Whether this frame re-read (it then asks for one further frame).
    reread: bool,
    doc: Option<Doc>,
    /// `(doc, head)` the view last scrolled for: when it changes, the head's line is brought
    /// into view.
    scrolled_for: Option<(u64, usize)>,
    /// The vertical scroll offset after the last frame.
    offset: f32,
    /// `(doc, largest offset)` of the scroll area in the last frame.
    last_max: Option<(u64, f32)>,
    geometry: Option<Geometry>,
}

/// The id of the editor node (`Editor`).
fn editor_id() -> Id {
    Id::unique("kiln-editor")
}

/// The id of the editor node's one text-run child, through which its selection is reported.
fn run_id() -> Id {
    editor_id().with("text")
}

impl EditorView {
    /// Start of a frame, before anything is drawn: handle the input addressed to the editor
    /// (each a command dispatched through `session`) and re-read the current document when a
    /// request happened since the last read, so that the frame shows the state after them. A
    /// refused command sets `message` to begin with its kind; a successful save clears it.
    pub(crate) fn update(&mut self, ctx: &Context, session: &mut Session, message: &mut String) {
        self.reread = false;
        if self.seen != Some(session.changes()) {
            self.refresh(session);
        }
        if self.doc.is_some() {
            if ctx.input(|i| i.has_accesskit_action_request(editor_id(), Action::Focus)) {
                ctx.memory_mut(|m| m.request_focus(editor_id()));
            }
            let events = ctx.input(|i| i.events.clone());
            for event in &events {
                if self.seen != Some(session.changes()) {
                    self.refresh(session);
                }
                self.input(ctx, event, session, message);
            }
            if self.seen != Some(session.changes()) {
                self.refresh(session);
            }
            if focused(ctx) {
                ctx.memory_mut(|m| {
                    m.set_focus_lock_filter(
                        editor_id(),
                        EventFilter {
                            tab: true,
                            horizontal_arrows: true,
                            vertical_arrows: true,
                            escape: false,
                        },
                    );
                });
            }
        }
        if self.reread {
            // Exactly one further frame (a zero delay would make egui run two).
            ctx.request_repaint_after(Duration::from_millis(1));
        }
    }

    /// Read the current document with read-only requests.
    fn refresh(&mut self, session: &mut Session) {
        self.seen = Some(session.changes());
        self.reread = true;
        self.doc = session.current().and_then(|id| read_doc(session, id));
    }

    /// One input event: dispatched only if it is addressed to the editor.
    fn input(&mut self, ctx: &Context, event: &Event, session: &mut Session, message: &mut String) {
        match event {
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                modifiers,
            } => {
                let on_editor = ctx
                    .read_response(editor_id())
                    .is_some_and(|r| r.hovered() && r.interact_rect.contains(*pos));
                if on_editor {
                    ctx.memory_mut(|m| m.request_focus(editor_id()));
                    self.click(ctx, *pos, modifiers.shift, session, message);
                }
            }
            Event::Text(text) if focused(ctx) && !text.is_empty() => {
                self.insert(text, session, message);
            }
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } if focused(ctx) => self.key(*key, *modifiers, session, message),
            _ => {}
        }
    }

    /// A click at `pos`: the head to the char boundary nearest to it on its row.
    fn click(
        &self,
        ctx: &Context,
        pos: Pos2,
        shift: bool,
        session: &mut Session,
        message: &mut String,
    ) {
        let (Some(doc), Some(g)) = (&self.doc, self.geometry) else {
            return;
        };
        let row = ((pos.y - g.top) / g.row_height).floor();
        let line = if row < 0.0 {
            0
        } else {
            (row as usize).min(doc.lines.len() - 1)
        };
        let (start, end) = doc.lines[line];
        let col = if start == end {
            0
        } else {
            let font = monospace(ctx);
            let galley = line_galley(ctx, &font, doc, line, g.width);
            nearest_boundary(&galley, pos.x - g.left).min(end - start)
        };
        let head = start + col;
        let anchor = if shift { doc.anchor } else { head };
        let id = doc.id;
        send(
            session,
            message,
            "cursor.set",
            json!({"doc": id, "anchor": anchor, "head": head}),
        );
    }

    /// Typed text (or a line break): inserted at the caret, or replacing the selection, with the
    /// caret collapsed after it. Where the text ends with `\r` and the char after the selection
    /// is `\n`, the two form one break and the caret goes after the whole break (ruling x1).
    fn insert(&self, text: &str, session: &mut Session, message: &mut String) {
        let Some(doc) = &self.doc else {
            return;
        };
        let (id, (start, end)) = (doc.id, doc.selection());
        let joins_break = text.ends_with('\r') && doc.chars.get(end) == Some(&'\n');
        let caret = start + text.chars().count() + usize::from(joins_break);
        let done = if start == end {
            send(
                session,
                message,
                "doc.insert",
                json!({"doc": id, "offset": doc.head, "text": text}),
            )
        } else {
            send(
                session,
                message,
                "doc.replace",
                json!({"doc": id, "start": start, "end": end, "text": text}),
            )
        };
        // `doc.insert` alone leaves a caret after the text, except when the mapping rule's CRLF
        // snap puts it before a joined break; `doc.replace` leaves the new text selected.
        if done && (start != end || joins_break) {
            send(
                session,
                message,
                "cursor.set",
                json!({"doc": id, "anchor": caret}),
            );
        }
    }

    /// A key press (the Input table of T-056).
    fn key(&self, key: Key, m: Modifiers, session: &mut Session, message: &mut String) {
        let Some(doc) = &self.doc else {
            return;
        };
        let id = doc.id;
        let ctrl = m.ctrl || m.command;
        if m.alt {
            return;
        }
        if key == Key::Enter && !ctrl {
            self.insert(doc.line_break(), session, message);
            return;
        }
        let doc_only = json!({ "doc": id });
        match key {
            Key::Z if ctrl && m.shift => {
                send(session, message, "doc.redo", doc_only);
            }
            Key::Z if ctrl => {
                send(session, message, "doc.undo", doc_only);
            }
            Key::Y if ctrl => {
                send(session, message, "doc.redo", doc_only);
            }
            Key::S if ctrl => {
                if send(session, message, "doc.save", doc_only) {
                    message.clear();
                }
            }
            Key::ArrowLeft | Key::ArrowRight => {
                let by = if ctrl { "word" } else { "char" };
                let forward = key == Key::ArrowRight;
                send(
                    session,
                    message,
                    "cursor.move",
                    json!({"doc": id, "by": by, "forward": forward, "extend": m.shift}),
                );
            }
            _ if ctrl => {}
            Key::ArrowUp | Key::ArrowDown => {
                let forward = key == Key::ArrowDown;
                send(
                    session,
                    message,
                    "cursor.move",
                    json!({"doc": id, "by": "line", "forward": forward, "extend": m.shift}),
                );
            }
            Key::Home | Key::End => {
                let (start, end) = doc.lines[doc.line_of(doc.head)];
                let head = if key == Key::Home { start } else { end };
                let anchor = if m.shift { doc.anchor } else { head };
                send(
                    session,
                    message,
                    "cursor.set",
                    json!({"doc": id, "anchor": anchor, "head": head}),
                );
            }
            Key::Backspace | Key::Delete => {
                let (mut start, mut end) = doc.selection();
                if start == end {
                    if key == Key::Backspace {
                        start = doc.char_step(false);
                    } else {
                        end = doc.char_step(true);
                    }
                }
                if start < end {
                    send(
                        session,
                        message,
                        "doc.delete",
                        json!({"doc": id, "start": start, "end": end}),
                    );
                }
            }
            _ => {}
        }
    }

    /// The `Title` node: the current document's path, and `modified` after it when it is dirty.
    pub(crate) fn title(&self, ui: &mut Ui) {
        if let Some(doc) = &self.doc {
            let text = if doc.dirty {
                format!("{} — modified — Kiln", doc.path)
            } else {
                format!("{} — Kiln", doc.path)
            };
            ui.label(text).accessible_name("Title");
        }
    }

    /// The `Editor` node inside its scroll area, filling `ui`'s available space.
    pub(crate) fn view(&mut self, ui: &mut Ui) {
        let Some(doc) = &self.doc else {
            self.geometry = None;
            return;
        };
        let font = monospace(ui.ctx());
        let row_height = ui.fonts_mut(|f| f.row_height(&font));
        let view_height = ui.available_height();
        let content_height = doc.lines.len() as f32 * row_height;
        let max_offset = (content_height - view_height).max(0.0);
        // After a change of the head (or of the document), bring the head's line into view.
        let mut offset = None;
        let mut shift = 0.0;
        if self.scrolled_for != Some((doc.id, doc.head)) {
            let same_doc = self.scrolled_for.map(|(d, _)| d) == Some(doc.id);
            let mut at = if same_doc { self.offset } else { 0.0 };
            let top = doc.line_of(doc.head) as f32 * row_height;
            if top < at {
                at = top;
            } else if top + row_height > at + view_height {
                at = top + row_height - view_height;
            }
            let at = at.clamp(0.0, max_offset);
            // The scroll area knows last frame's maximum only (beyond it, egui animates the
            // offset back for dozens of frames): it goes as far as that allows, the text is
            // drawn shifted by the rest, and the next frame sets the whole offset.
            let allowed = match self.last_max {
                Some((d, last)) if d == doc.id && at > last => last,
                _ => at,
            };
            if allowed < at {
                shift = at - allowed;
                ui.ctx().request_repaint_after(Duration::from_millis(1));
            } else {
                self.scrolled_for = Some((doc.id, doc.head));
            }
            offset = Some(allowed);
        }
        self.last_max = Some((doc.id, max_offset));
        let focused = focused(ui.ctx());
        let mut geometry = None;
        let (scrolled, _) = scroll_view_with(ui, "kiln-editor-scroll", offset, |ui| {
            geometry = Some(paint(
                ui,
                doc,
                &font,
                row_height,
                content_height,
                shift,
                focused,
            ));
        });
        self.offset = scrolled;
        self.geometry = geometry;
    }
}

/// Whether the editor has keyboard focus.
fn focused(ctx: &Context) -> bool {
    ctx.memory(|m| m.has_focus(editor_id()))
}

/// The built-in monospace font of the style.
fn monospace(ctx: &Context) -> FontId {
    ctx.global_style().text_styles[&TextStyle::Monospace].clone()
}

/// Send a mutating command of the UI's own; a refusal sets `message` to begin with its kind.
/// True if it succeeded.
fn send(session: &mut Session, message: &mut String, command: &str, params: Value) -> bool {
    match session.request(command, params) {
        Ok(_) => true,
        Err((kind, text)) => {
            *message = format!("{kind}: {text}");
            false
        }
    }
}

/// Read document `id` with read-only requests (`None` if one is refused).
fn read_doc(session: &mut Session, id: u64) -> Option<Doc> {
    let doc = json!({ "doc": id });
    let info = session.read("doc.info", doc.clone()).ok()?;
    let text = session.read("doc.text", doc.clone()).ok()?;
    let selection = session.read("cursor.get", doc.clone()).ok()?;
    let path = info["path"].as_str()?.to_owned();
    // `.rs` documents only; a refusal (`too_large`) leaves them plain.
    let spans = if path.ends_with(".rs") {
        session
            .read("syntax.highlight", doc)
            .ok()
            .and_then(|r| spans(&r))
    } else {
        None
    };
    let text = text["text"].as_str()?.to_owned();
    let chars: Vec<char> = text.chars().collect();
    let number = |v: &Value| v.as_u64().and_then(|n| usize::try_from(n).ok());
    Some(Doc {
        id,
        path,
        lines: split_lines(&chars),
        chars,
        text,
        dirty: info["dirty"].as_bool().unwrap_or(false),
        line_ending: info["line_ending"].as_str().unwrap_or("none").to_owned(),
        anchor: number(&selection["anchor"])?,
        head: number(&selection["head"])?,
        spans,
    })
}

/// The spans of a `syntax.highlight` result as `(start, end, colour)`.
fn spans(result: &Value) -> Option<Vec<(usize, usize, Color32)>> {
    result["spans"]
        .as_array()?
        .iter()
        .map(|s| {
            let n = |k: &str| s[k].as_u64().and_then(|n| usize::try_from(n).ok());
            Some((n("start")?, n("end")?, colour(s["kind"].as_str()?)))
        })
        .collect()
}

/// The layout of `line`'s chars, each in its kind's colour, as far as just past `width`.
fn line_galley(ctx: &Context, font: &FontId, doc: &Doc, line: usize, width: f32) -> Arc<Galley> {
    let (start, end) = doc.lines[line];
    let mut count = (end - start).min(FIRST_LAYOUT);
    loop {
        let job = line_job(doc, start, start + count, font);
        let galley = ctx.fonts_mut(|f| f.layout_job(job));
        if start + count >= end || galley.size().x > width {
            return galley;
        }
        count = (count * 4).min(end - start);
    }
}

/// The char boundary of a one-row `galley` nearest to `x` (galley-relative): the left edges of
/// its chars as the observation reports them (`observe::char_edges`), and the end of its last
/// char; the lower one on a tie.
fn nearest_boundary(galley: &Galley, x: f32) -> usize {
    let Some(placed) = galley.rows.first() else {
        return 0;
    };
    let n = galley.job.text.chars().count();
    let edges = char_edges(&placed.row, n);
    let x = x - placed.pos.x;
    let last_left = edges.last().map_or(f32::NEG_INFINITY, |(l, _)| *l);
    let end = edges
        .iter()
        .map(|(_, r)| *r)
        .fold(last_left + MIN_STEP, f32::max);
    let mut best = (0, f32::INFINITY);
    for (k, b) in edges.iter().map(|(l, _)| *l).chain([end]).enumerate() {
        let d = (b - x).abs();
        if d < best.1 {
            best = (k, d);
        }
    }
    best.0
}

/// A layout job of the chars `[start, stop)`, one section per run of one colour.
fn line_job(doc: &Doc, start: usize, stop: usize, font: &FontId) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mut run = String::new();
    let mut run_colour = None;
    for at in start..stop {
        let c = doc.colour_at(at);
        if run_colour != Some(c) {
            if let Some(prev) = run_colour {
                job.append(&run, 0.0, format(font, prev));
                run.clear();
            }
            run_colour = Some(c);
        }
        run.push(doc.chars[at]);
    }
    if let Some(prev) = run_colour {
        job.append(&run, 0.0, format(font, prev));
    }
    job.break_on_newline = false;
    job
}

fn format(font: &FontId, color: Color32) -> TextFormat {
    TextFormat {
        font_id: font.clone(),
        color,
        ..TextFormat::default()
    }
}

/// Paint the visible lines of `doc` in the scroll area's content `ui` and report the editor
/// node; returns where the text was drawn.
fn paint(
    ui: &mut Ui,
    doc: &Doc,
    font: &FontId,
    row_height: f32,
    content_height: f32,
    shift: f32,
    focused: bool,
) -> Geometry {
    let origin = ui.max_rect().min - vec2(0.0, shift);
    let rect = Rect::from_min_size(
        origin,
        vec2(ui.available_width(), content_height.max(row_height)),
    );
    let builder = UiBuilder::new()
        .scope_id(editor_id())
        .max_rect(rect)
        .sense(Sense::click())
        .accessibility_role(Role::MultilineTextInput)
        .accessibility_label("Editor");
    ui.scope_builder(builder, |ui| {
        ui.expand_to_include_rect(rect);
        let clip = ui.clip_rect();
        let left = rect.left() + LEFT_MARGIN;
        let width = clip.right() - left;
        let first = ((clip.top() - rect.top()) / row_height).floor().max(0.0) as usize;
        let last = (((clip.bottom() - rect.top()) / row_height).ceil().max(0.0) as usize)
            .min(doc.lines.len());
        let (sel_start, sel_end) = doc.selection();
        let painter = ui.painter().clone();
        for line in first..last {
            let (start, end) = doc.lines[line];
            let y = rect.top() + line as f32 * row_height;
            let galley = line_galley(ui.ctx(), font, doc, line, width);
            let laid_out = galley.text().chars().count();
            let x_of = |at: usize| {
                let col = at.saturating_sub(start).min(laid_out);
                left + galley.pos_from_cursor(CCursor::new(col)).min.x
            };
            // The selection, as a background: past the end of a line that has a break, a
            // little more, so that a selected break is visible.
            let has_break = line + 1 < doc.lines.len();
            if sel_start < sel_end && sel_start <= end && sel_end > start {
                let x0 = x_of(sel_start.max(start));
                let x1 = if sel_end > end && has_break {
                    x_of(end) + row_height / 3.0
                } else {
                    x_of(sel_end.min(end))
                };
                painter.rect_filled(
                    Rect::from_min_max(pos2(x0, y), pos2(x1, y + row_height)),
                    0.0,
                    SELECTION,
                );
            }
            painter.galley(pos2(left, y), galley.clone(), PLAIN);
            if focused && doc.line_of(doc.head) == line {
                let x = x_of(doc.head);
                painter.vline(x, y..=y + row_height, Stroke::new(1.0, PLAIN));
            }
        }
        // The node: its value is the text, its selection the document's (in chars, through
        // one text-run child that holds every char).
        let run = run_id();
        ui.interact(
            Rect::from_min_size(rect.min, Vec2::ZERO),
            run,
            Sense::hover(),
        );
        let text_position = |at: usize| TextPosition {
            node: run.accesskit_id(),
            character_index: at,
        };
        ui.ctx().accesskit_node_builder(editor_id(), |node| {
            node.set_bounds(accesskit_rect(rect));
            node.set_value(doc.text.clone());
            node.set_text_selection(TextSelection {
                anchor: text_position(doc.anchor),
                focus: text_position(doc.head),
            });
        });
        ui.ctx().accesskit_node_builder(run, |node| {
            node.set_role(Role::TextRun);
            node.set_value(doc.text.clone());
            node.set_character_lengths(
                doc.chars
                    .iter()
                    .map(|c| c.len_utf8() as u8)
                    .collect::<Vec<u8>>(),
            );
        });
        Geometry {
            left,
            top: rect.top(),
            row_height,
            width,
        }
    })
    .inner
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_split_at_every_break() {
        let chars: Vec<char> = "a\r\nbé\rc\n".chars().collect();
        assert_eq!(split_lines(&chars), [(0, 1), (3, 5), (6, 7), (8, 8)]);
        assert_eq!(split_lines(&[]), [(0, 0)]);
    }

    #[test]
    fn kinds_have_the_published_colours() {
        assert_eq!(colour("keyword").to_hex(), "#569cd6ff");
        assert_eq!(colour("comment").to_hex(), "#6a9955ff");
        assert_eq!(colour("nonsense"), PLAIN);
        assert_eq!(KIND_COLOURS.len(), 7);
    }
}
