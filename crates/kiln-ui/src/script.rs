//! `kiln script`: the Kiln UI run without a window from a script of control requests and UI
//! requests (`docs/protocol/ui.md`, ADR-0008, T-055).
//!
//! The runner owns one egui context, the shell and one session. Frames are run with simulated
//! time only: frame `n` (from 0) has time `n / 60` s; no clock is read. After every action, after
//! every successful control request and before the first request, frames are run until the UI
//! asks for no further frame (egui's repaint delay is "never"), at most [`SETTLE_CAP`].

use std::path::Path;
use std::time::Duration;

use egui::accesskit::{Action, ActionRequest, NodeId, TreeId, TreeUpdate};
use egui::epaint::ClippedShape;
use egui::{
    Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Theme, ViewportId, ViewportInfo,
};
use kiln_control::Host;
use serde_json::{Map, Value, json};

use crate::json::{canonical, cap};
use crate::observe::{Observation, role_name};
use crate::session::Session;
use crate::shell::Shell;

/// The most frames one action (or one control request, or the start) runs while settling.
pub const SETTLE_CAP: u64 = 60;
/// The most frames one `frames` request runs.
pub const MAX_FRAMES: u64 = 600;
/// The most steps one `scroll` request takes.
pub const MAX_SCROLL_STEPS: u64 = 100;
/// The smallest and largest width or height (`--size`), in points.
pub const SIZE_RANGE: std::ops::RangeInclusive<u32> = 200..=8192;
/// The default size `--size` (width, height) in points.
pub const DEFAULT_SIZE: (u32, u32) = (1280, 800);

/// Key names a `key` request accepts, with the key each names.
pub const KEY_NAMES: &[(&str, Key)] = &[
    ("A", Key::A),
    ("B", Key::B),
    ("C", Key::C),
    ("D", Key::D),
    ("E", Key::E),
    ("F", Key::F),
    ("G", Key::G),
    ("H", Key::H),
    ("I", Key::I),
    ("J", Key::J),
    ("K", Key::K),
    ("L", Key::L),
    ("M", Key::M),
    ("N", Key::N),
    ("O", Key::O),
    ("P", Key::P),
    ("Q", Key::Q),
    ("R", Key::R),
    ("S", Key::S),
    ("T", Key::T),
    ("U", Key::U),
    ("V", Key::V),
    ("W", Key::W),
    ("X", Key::X),
    ("Y", Key::Y),
    ("Z", Key::Z),
    ("0", Key::Num0),
    ("1", Key::Num1),
    ("2", Key::Num2),
    ("3", Key::Num3),
    ("4", Key::Num4),
    ("5", Key::Num5),
    ("6", Key::Num6),
    ("7", Key::Num7),
    ("8", Key::Num8),
    ("9", Key::Num9),
    ("Enter", Key::Enter),
    ("Tab", Key::Tab),
    ("Backspace", Key::Backspace),
    ("Delete", Key::Delete),
    ("Escape", Key::Escape),
    ("Space", Key::Space),
    ("ArrowLeft", Key::ArrowLeft),
    ("ArrowRight", Key::ArrowRight),
    ("ArrowUp", Key::ArrowUp),
    ("ArrowDown", Key::ArrowDown),
    ("Home", Key::Home),
    ("End", Key::End),
    ("PageUp", Key::PageUp),
    ("PageDown", Key::PageDown),
    ("F1", Key::F1),
    ("F2", Key::F2),
    ("F3", Key::F3),
    ("F4", Key::F4),
    ("F5", Key::F5),
    ("F6", Key::F6),
    ("F7", Key::F7),
    ("F8", Key::F8),
    ("F9", Key::F9),
    ("F10", Key::F10),
    ("F11", Key::F11),
    ("F12", Key::F12),
];

/// Where the session's files are.
#[derive(Debug, Clone, Copy)]
pub enum Files<'a> {
    /// The disk below this root (as `kiln-cli --root`).
    Disk(&'a Path),
    /// An empty in-memory file system (as `kiln-cli --memory`).
    Memory,
}

/// A `kiln script` session: the UI, its session and the frames run so far.
pub struct Script {
    ctx: egui::Context,
    shell: Shell,
    session: Session,
    screen: Rect,
    frames: u64,
    last_update: Option<TreeUpdate>,
    last_shapes: Vec<ClippedShape>,
    observation: Option<Observation>,
}

/// A UI request that is refused: its kind and a message.
type Refusal = (&'static str, String);

impl Script {
    /// A session over `files` with a UI of `size` (width, height) points, after its first
    /// frames have run. Fails, with the host's message (naming a root as given), when the root
    /// cannot be used.
    pub fn new(files: Files<'_>, size: (u32, u32)) -> Result<Script, String> {
        let host = match files {
            Files::Disk(root) => Host::on_disk(root),
            Files::Memory => Host::in_memory(&[]),
        }
        .map_err(|e| e.to_string())?;
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        ctx.set_theme(Theme::Dark);
        ctx.all_styles_mut(|style| style.visuals.text_cursor.blink = false);
        let mut script = Script {
            ctx,
            shell: Shell::default(),
            session: Session::new(host),
            screen: Rect::from_min_size(Pos2::ZERO, egui::vec2(size.0 as f32, size.1 as f32)),
            frames: 0,
            last_update: None,
            last_shapes: Vec::new(),
            observation: None,
        };
        script.settle(Vec::new());
        Ok(script)
    }

    /// One script line in; its response line (without `"\n"`) out, `None` for a blank line.
    pub fn handle_line(&mut self, line: &[u8]) -> Option<String> {
        let parsed = std::str::from_utf8(line)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(text).ok());
        match parsed {
            Some(Value::Object(request)) if request.contains_key("ui") && valid_id(&request) => {
                Some(self.ui_request(&request))
            }
            // Control requests, and every line-level error, are the host's (same bytes).
            _ => {
                let response = self.session.handle_line(line)?;
                if response_ok(&response) {
                    self.settle(Vec::new());
                }
                Some(response)
            }
        }
    }

    /// The session log as a script (the format of `kiln-cli run --log`).
    pub fn log_script(&self) -> String {
        self.session.log_script()
    }

    /// The number of frames run so far.
    pub fn frames(&self) -> u64 {
        self.frames
    }

    fn ui_request(&mut self, request: &Map<String, Value>) -> String {
        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let response = match self.ui_action(request) {
            Ok(result) => json!({"id": id, "ok": true, "result": result}),
            Err((kind, message)) => json!({
                "error": {"code": "ui", "kind": kind, "message": cap(&message)},
                "id": id,
                "ok": false,
            }),
        };
        canonical(&response)
    }

    fn ui_action(&mut self, request: &Map<String, Value>) -> Result<Value, Refusal> {
        let action = parse(request)?;
        let (screen, update, shapes) = (self.screen, self.last_update.as_ref(), &self.last_shapes);
        let observation: &Observation = self
            .observation
            .get_or_insert_with(|| Observation::new(update, shapes, screen));
        let input = match action {
            UiAction::Snapshot => {
                return Ok(json!({"frames": self.frames, "root": observation.to_json(0)}));
            }
            UiAction::Frames(count) => {
                let mut settled = true;
                for _ in 0..count {
                    settled = !self.frame(Vec::new());
                }
                return Ok(json!({"frames": count, "settled": settled}));
            }
            UiAction::Focus(target) => {
                let node = find(observation, &target)?;
                if !observation.nodes[node].takes_text() {
                    return Err(not_actionable("the target does not take text input"));
                }
                vec![vec![action_request(
                    Action::Focus,
                    observation.nodes[node].id,
                )]]
            }
            UiAction::Type(text) => {
                if !observation
                    .focused()
                    .is_some_and(|n| observation.nodes[n].takes_text())
                {
                    return Err(not_actionable("no focused node takes text input"));
                }
                vec![vec![Event::Text(text)]]
            }
            UiAction::Key { key, modifiers } => vec![
                vec![
                    Event::ModifiersChanged(modifiers),
                    key_event(key, true, modifiers),
                ],
                vec![
                    key_event(key, false, modifiers),
                    Event::ModifiersChanged(Modifiers::NONE),
                ],
            ],
            UiAction::Click {
                target,
                button,
                modifiers,
                at,
            } => {
                let node = find(observation, &target)?;
                let pos = click_point(observation, node, at)?;
                let press = |pressed| Event::PointerButton {
                    pos,
                    button,
                    pressed,
                    modifiers,
                };
                vec![
                    vec![
                        Event::ModifiersChanged(modifiers),
                        Event::PointerMoved(pos),
                        press(true),
                    ],
                    vec![press(false), Event::ModifiersChanged(Modifiers::NONE)],
                ]
            }
            UiAction::Scroll {
                target,
                direction,
                steps,
            } => {
                let node = find(observation, &target)?;
                if observation.scroll_view_around(node).is_none() {
                    return Err(not_actionable("no scrollable area contains the target"));
                }
                let id = observation.nodes[node].id;
                vec![(0..steps).map(|_| action_request(direction, id)).collect()]
            }
        };
        let (frames, settled) = self.settle(input);
        Ok(json!({"frames": frames, "settled": settled}))
    }

    /// Run one frame per entry of `input` (at least one), then frames without input until the
    /// UI asks for no further frame or [`SETTLE_CAP`] frames ran. Returns (frames run, settled).
    fn settle(&mut self, input: Vec<Vec<Event>>) -> (u64, bool) {
        let mut input = input;
        if input.is_empty() {
            input.push(Vec::new());
        }
        let mut run = 0;
        let mut wants_more = false;
        for events in input {
            wants_more = self.frame(events);
            run += 1;
        }
        while wants_more && run < SETTLE_CAP {
            wants_more = self.frame(Vec::new());
            run += 1;
        }
        (run, !wants_more)
    }

    /// Run one frame with `events`; true if the UI asks for a further frame.
    fn frame(&mut self, events: Vec<Event>) -> bool {
        let mut viewports = egui::ViewportIdMap::default();
        viewports.insert(
            ViewportId::ROOT,
            ViewportInfo {
                native_pixels_per_point: Some(1.0),
                focused: Some(true),
                ..ViewportInfo::default()
            },
        );
        let input = RawInput {
            viewport_id: ViewportId::ROOT,
            viewports,
            screen_rect: Some(self.screen),
            max_texture_side: Some(8192),
            time: Some(self.frames as f64 / 60.0),
            predicted_dt: 1.0 / 60.0,
            events,
            focused: true,
            ..RawInput::default()
        };
        let shell = &mut self.shell;
        let session = &mut self.session;
        let mut output = self.ctx.run_ui(input, |ui| shell.ui(ui, session));
        // No renderer: the font atlas updates are not needed (nothing is drawn as pixels).
        output.textures_delta.clear();
        self.frames += 1;
        let wants_more = output
            .viewport_output
            .get(&ViewportId::ROOT)
            .is_some_and(|v| v.repaint_delay != Duration::MAX);
        self.last_update = output.platform_output.accesskit_update;
        self.last_shapes = output.shapes;
        self.observation = None;
        wants_more
    }
}

/// A validated UI request.
enum UiAction {
    Snapshot,
    Frames(u64),
    Focus(Target),
    Click {
        target: Target,
        button: PointerButton,
        modifiers: Modifiers,
        at: Option<(usize, usize)>,
    },
    Type(String),
    Key {
        key: Key,
        modifiers: Modifiers,
    },
    Scroll {
        target: Target,
        direction: Action,
        steps: u64,
    },
}

/// A target: every given field must match.
#[derive(Default)]
struct Target {
    role: Option<String>,
    label: Option<String>,
    label_contains: Option<String>,
    value: Option<String>,
    value_contains: Option<String>,
}

/// The keys each action allows besides `id` and `ui`; `None` for an unknown action.
fn action_keys(ui: &str) -> Option<&'static [&'static str]> {
    Some(match ui {
        "snapshot" => &[],
        "frames" => &["count"],
        "focus" => &["target"],
        "click" => &["target", "button", "modifiers", "at"],
        "type" => &["text"],
        "key" => &["key", "modifiers"],
        "scroll" => &["target", "direction", "steps"],
        _ => return None,
    })
}

fn bad(message: impl Into<String>) -> Refusal {
    ("bad_request", message.into())
}

fn not_actionable(message: &str) -> Refusal {
    ("not_actionable", message.to_owned())
}

/// Validate a UI request: `bad_request` first (keys, types, ranges, names), then
/// `unknown_action`.
fn parse(request: &Map<String, Value>) -> Result<UiAction, Refusal> {
    let Some(ui) = request.get("ui").and_then(Value::as_str) else {
        return Err(bad("`ui` must be a string"));
    };
    let allowed = action_keys(ui).unwrap_or(&[]);
    let mut unknown: Vec<&str> = request
        .keys()
        .map(String::as_str)
        .filter(|k| *k != "id" && *k != "ui" && !allowed.contains(k))
        .collect();
    if !unknown.is_empty() {
        unknown.sort_unstable();
        return Err(bad(format!("unknown key(s) `{}`", unknown.join("`, `"))));
    }
    let get = |key: &str| request.get(key);
    let action = match ui {
        "snapshot" => UiAction::Snapshot,
        "frames" => UiAction::Frames(integer(get("count"), "count", 1, MAX_FRAMES)?),
        "focus" => UiAction::Focus(target(get("target"))?),
        "click" => UiAction::Click {
            target: target(get("target"))?,
            button: match get("button") {
                None => PointerButton::Primary,
                Some(Value::String(b)) if b == "primary" => PointerButton::Primary,
                Some(Value::String(b)) if b == "secondary" => PointerButton::Secondary,
                Some(_) => return Err(bad("`button` must be \"primary\" or \"secondary\"")),
            },
            modifiers: modifiers(get("modifiers"))?,
            at: match get("at") {
                None => None,
                Some(at) => Some(position(at)?),
            },
        },
        "type" => match get("text") {
            Some(Value::String(text)) => UiAction::Type(text.clone()),
            Some(_) => return Err(bad("`text` must be a string")),
            None => return Err(bad("`text` is missing")),
        },
        "key" => UiAction::Key {
            key: match get("key") {
                Some(Value::String(name)) => KEY_NAMES
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, k)| *k)
                    .ok_or_else(|| bad(format!("unknown key name `{name}`")))?,
                Some(_) => return Err(bad("`key` must be a string")),
                None => return Err(bad("`key` is missing")),
            },
            modifiers: modifiers(get("modifiers"))?,
        },
        "scroll" => UiAction::Scroll {
            target: target(get("target"))?,
            direction: match get("direction") {
                Some(Value::String(d)) => match d.as_str() {
                    "up" => Action::ScrollUp,
                    "down" => Action::ScrollDown,
                    "left" => Action::ScrollLeft,
                    "right" => Action::ScrollRight,
                    _ => return Err(bad(format!("unknown direction `{d}`"))),
                },
                Some(_) => return Err(bad("`direction` must be a string")),
                None => return Err(bad("`direction` is missing")),
            },
            steps: match get("steps") {
                None => 1,
                some => integer(some, "steps", 1, MAX_SCROLL_STEPS)?,
            },
        },
        _ => return Err(("unknown_action", format!("unknown UI action `{ui}`"))),
    };
    Ok(action)
}

/// A required integer `key` in `min..=max`.
fn integer(value: Option<&Value>, key: &str, min: u64, max: u64) -> Result<u64, Refusal> {
    let Some(value) = value else {
        return Err(bad(format!("`{key}` is missing")));
    };
    if !(value.is_u64() || value.is_i64()) {
        return Err(bad(format!("`{key}` must be an integer")));
    }
    // A negative integer has no `u64` value: out of range like any other.
    match value.as_u64() {
        Some(n) if (min..=max).contains(&n) => Ok(n),
        _ => Err(bad(format!("`{key}` must be from {min} to {max}"))),
    }
}

fn target(value: Option<&Value>) -> Result<Target, Refusal> {
    let Some(value) = value else {
        return Err(bad("`target` is missing"));
    };
    let Value::Object(fields) = value else {
        return Err(bad("`target` must be an object"));
    };
    if fields.is_empty() {
        return Err(bad("`target` is empty"));
    }
    let mut t = Target::default();
    for (key, v) in fields {
        let slot = match key.as_str() {
            "role" => &mut t.role,
            "label" => &mut t.label,
            "label_contains" => &mut t.label_contains,
            "value" => &mut t.value,
            "value_contains" => &mut t.value_contains,
            other => return Err(bad(format!("unknown target key `{other}`"))),
        };
        let Value::String(s) = v else {
            return Err(bad(format!("target `{key}` must be a string")));
        };
        *slot = Some(s.clone());
    }
    Ok(t)
}

fn modifiers(value: Option<&Value>) -> Result<Modifiers, Refusal> {
    let mut m = Modifiers::NONE;
    let Some(value) = value else {
        return Ok(m);
    };
    let Value::Array(names) = value else {
        return Err(bad("`modifiers` must be an array"));
    };
    for name in names {
        match name.as_str() {
            // On Linux, egui's `command` is the Ctrl key: both names press it.
            Some("ctrl" | "command") => {
                m.ctrl = true;
                m.command = true;
            }
            Some("shift") => m.shift = true,
            Some("alt") => m.alt = true,
            Some(other) => return Err(bad(format!("unknown modifier `{other}`"))),
            None => return Err(bad("`modifiers` must hold strings")),
        }
    }
    Ok(m)
}

fn position(value: &Value) -> Result<(usize, usize), Refusal> {
    let Value::Object(fields) = value else {
        return Err(bad("`at` must be an object"));
    };
    if let Some(other) = fields.keys().find(|k| *k != "row" && *k != "col") {
        return Err(bad(format!("unknown key `{other}` in `at`")));
    }
    let row = integer(fields.get("row"), "row", 0, u64::MAX)?;
    let col = integer(fields.get("col"), "col", 0, u64::MAX)?;
    Ok((
        usize::try_from(row).unwrap_or(usize::MAX),
        usize::try_from(col).unwrap_or(usize::MAX),
    ))
}

/// The one node `target` matches: `no_target` or `ambiguous_target` otherwise.
fn find(observation: &Observation, target: &Target) -> Result<usize, Refusal> {
    let matches = |s: &Option<String>, want: &Option<String>, exact: bool| match want {
        None => true,
        Some(w) => s.as_deref().is_some_and(|s| {
            if exact {
                s == w
            } else {
                s.contains(w.as_str())
            }
        }),
    };
    let found: Vec<usize> = observation
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| {
            target.role.as_ref().is_none_or(|r| role_name(n.role) == *r)
                && matches(&n.label, &target.label, true)
                && matches(&n.label, &target.label_contains, false)
                && matches(&n.value, &target.value, true)
                && matches(&n.value, &target.value_contains, false)
        })
        .map(|(i, _)| i)
        .collect();
    match found.as_slice() {
        [] => Err(("no_target", "no node matches the target".to_owned())),
        [one] => Ok(*one),
        more => Err((
            "ambiguous_target",
            format!("{} nodes match the target", more.len()),
        )),
    }
}

/// Where a click on `node` lands: its centre, or with `at = (r, c)` the left edge of char `c`
/// of its painted row `r` (just right of the last char when `c` is the row's length).
fn click_point(
    observation: &Observation,
    node: usize,
    at: Option<(usize, usize)>,
) -> Result<Pos2, Refusal> {
    let n = &observation.nodes[node];
    let Some((row, col)) = at else {
        return Ok(n.rect.map_or(Pos2::ZERO, |r| r.center()));
    };
    if n.rows.is_empty() {
        return Err(not_actionable("the target shows no text"));
    }
    let Some(glyphs) = n.rows.get(row).map(|r| &r.glyphs) else {
        return Err(not_actionable(
            "`at.row` is beyond the target's painted rows",
        ));
    };
    match (glyphs.get(col), glyphs.last()) {
        (Some(g), _) => Ok(Pos2::new(g.x0, g.y)),
        (None, Some(last)) if col == glyphs.len() => Ok(Pos2::new(last.x1 + 1.0, last.y)),
        _ => Err(not_actionable("`at.col` is beyond the row's length")),
    }
}

fn action_request(action: Action, target: NodeId) -> Event {
    Event::AccessKitActionRequest(ActionRequest {
        action,
        target_tree: TreeId::ROOT,
        target_node: target,
        data: None,
    })
}

fn key_event(key: Key, pressed: bool, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers,
    }
}

/// `id` absent, a string, or an integer (the control protocol's rule; otherwise the line is
/// the host's `bad_id`).
fn valid_id(request: &Map<String, Value>) -> bool {
    match request.get("id") {
        None | Some(Value::String(_)) => true,
        Some(Value::Number(n)) => n.is_i64() || n.is_u64(),
        Some(_) => false,
    }
}

/// Whether a response line reports `"ok":true`.
pub fn response_ok(response: &str) -> bool {
    serde_json::from_str::<Value>(response).is_ok_and(|v| v["ok"] == Value::Bool(true))
}

/// Parse `--size`'s value `<w>x<h>`: integers in [`SIZE_RANGE`]. `None` if malformed or out
/// of range.
pub fn parse_size(text: &str) -> Option<(u32, u32)> {
    let (w, h) = text.split_once('x')?;
    let number = |s: &str| {
        if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        s.parse::<u32>().ok().filter(|n| SIZE_RANGE.contains(n))
    };
    Some((number(w)?, number(h)?))
}
