//! `kiln script`: the Kiln UI run without a window from a script of control requests and UI
//! requests (`docs/protocol/ui.md`, ADR-0008, T-055).
//!
//! The runner owns one egui context and one [`App`] (the shell, the editor view and the
//! session), and draws every frame with [`crate::draw_frame`]. Frames are run with simulated
//! time only: frame `n` (from 0) has time `n / 60` s; no clock is read. After every action and
//! before the first request (after the start paths, [`Script::start`]), frames are run until
//! the UI asks for no further frame (egui's repaint delay is "never"), at most [`SETTLE_CAP`].

use std::path::Path;
use std::time::Duration;

use egui::accesskit::{Action, ActionRequest, NodeId, TreeId, TreeUpdate};
use egui::epaint::ClippedShape;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, ViewportId, ViewportInfo};
use kiln_control::Host;
use serde_json::{Map, Value, json};

use crate::json::{canonical, cap};
use crate::observe::{Observation, role_name};
use crate::{App, apply_style, draw_frame};

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
    app: App,
    /// Whether the start paths were opened and the first frames run.
    started: bool,
    screen: Rect,
    frames: u64,
    last_update: Option<TreeUpdate>,
    last_shapes: Vec<ClippedShape>,
    observation: Option<Observation>,
}

/// A UI request that is refused: its kind and a message.
type Refusal = (&'static str, String);

/// What a script line needs before its response is known (T-065: the window runs the frames of
/// a [`Pending`] itself, `kiln script` runs them at once).
pub(crate) enum Step {
    /// The response line, `None` for a blank line.
    Done(Option<String>),
    /// An action: its frames are to be run, then [`Script::finish`] gives the response.
    Frames(Pending),
}

/// The frames an action (or the start) still has to run, and what the ones run so far said.
pub(crate) struct Pending {
    /// The request's `id`; `None` for the start, which gets no response.
    id: Option<Value>,
    /// The input of the frames still to run before settling (one entry per frame).
    input: std::collections::VecDeque<Vec<Event>>,
    /// `frames`: exactly the frames of `input`, no settling.
    exact: bool,
    run: u64,
    wants_more: bool,
}

impl Pending {
    /// One frame per entry of `input` (at least one), then frames without input while the UI
    /// asks for one.
    fn settle(id: Option<Value>, input: Vec<Vec<Event>>) -> Pending {
        let mut input: std::collections::VecDeque<_> = input.into();
        if input.is_empty() {
            input.push_back(Vec::new());
        }
        Pending {
            id,
            input,
            exact: false,
            run: 0,
            wants_more: false,
        }
    }

    /// Exactly `count` frames without input.
    fn exactly(id: Value, count: u64) -> Pending {
        Pending {
            id: Some(id),
            input: (0..count).map(|_| Vec::new()).collect(),
            exact: true,
            run: 0,
            wants_more: false,
        }
    }

    /// The input of the next frame to run, or `None` when the action is complete. `go_on(run)`
    /// says whether settling may run one more frame after `run` frames (for `kiln script`:
    /// fewer than [`SETTLE_CAP`]).
    pub(crate) fn next_input(&mut self, go_on: impl FnOnce(u64) -> bool) -> Option<Vec<Event>> {
        if let Some(events) = self.input.pop_front() {
            return Some(events);
        }
        (!self.exact && self.wants_more && go_on(self.run)).then(Vec::new)
    }

    /// Record that one frame ran; `wants_more`: the UI asked for a further frame.
    pub(crate) fn after_frame(&mut self, wants_more: bool) {
        self.run += 1;
        self.wants_more = wants_more;
    }
}

impl Script {
    /// A session over `files` with a UI of `size` (width, height) points; no frame has run
    /// yet ([`Script::start`] runs the first ones). Fails, with the host's message (naming a
    /// root as given), when the root cannot be used.
    pub fn new(files: Files<'_>, size: (u32, u32)) -> Result<Script, String> {
        let host = match files {
            Files::Disk(root) => Host::on_disk(root),
            Files::Memory => Host::in_memory(&[]),
        }
        .map_err(|e| e.to_string())?;
        Ok(Script::with_host(host, size))
    }

    /// A session over `host` (T-065: the window checks the root itself, before the window
    /// system).
    pub(crate) fn with_host(host: Host, size: (u32, u32)) -> Script {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        apply_style(&ctx);
        Script {
            ctx,
            app: App::new(host),
            started: false,
            screen: Rect::from_min_size(Pos2::ZERO, egui::vec2(size.0 as f32, size.1 as f32)),
            frames: 0,
            last_update: None,
            last_shapes: Vec::new(),
            observation: None,
        }
    }

    /// Open the start paths in order, each as if it were typed into `Path` and opened (no
    /// response; a refused one sets `Message`), then run the frames before the first request
    /// until the UI settles. Runs once; the first request line runs it (without paths) if it has
    /// not run.
    pub fn start(&mut self, paths: &[&str]) {
        if let Some(mut pending) = self.begin_start(paths) {
            self.run_frames(&mut pending);
        }
    }

    /// The start without its frames: opens the paths and gives the frames to run before the
    /// first request; `None` if the start has already been made.
    pub(crate) fn begin_start(&mut self, paths: &[&str]) -> Option<Pending> {
        if self.started {
            return None;
        }
        self.started = true;
        for path in paths {
            self.app.open(path);
        }
        Some(Pending::settle(None, Vec::new()))
    }

    /// One script line in; its response line (without `"\n"`) out, `None` for a blank line.
    pub fn handle_line(&mut self, line: &[u8]) -> Option<String> {
        self.start(&[]);
        match self.begin_line(line) {
            Step::Done(response) => response,
            Step::Frames(mut pending) => {
                self.run_frames(&mut pending);
                self.finish(pending)
            }
        }
    }

    /// One script line, up to the frames it needs (the start must have been made).
    pub(crate) fn begin_line(&mut self, line: &[u8]) -> Step {
        let parsed = std::str::from_utf8(line)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(text).ok());
        match parsed {
            Some(Value::Object(request)) if request.contains_key("ui") && valid_id(&request) => {
                self.ui_request(&request)
            }
            // Control requests, and every line-level error, are the host's (same bytes). They
            // run no frame (D-149 n4): what they change is shown from the next frame on.
            _ => Step::Done(self.app.handle_line(line)),
        }
    }

    /// The response of an action whose frames have run (`None` for the start).
    pub(crate) fn finish(&self, pending: Pending) -> Option<String> {
        let id = pending.id?;
        let result = json!({"frames": pending.run, "settled": !pending.wants_more});
        Some(canonical(&json!({"id": id, "ok": true, "result": result})))
    }

    /// The session log as a script (the format of `kiln-cli run --log`).
    pub fn log_script(&self) -> String {
        self.app.log_script()
    }

    /// The number of frames run so far.
    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// The UI's state, for a window that draws the frames itself (T-065).
    pub(crate) fn app_mut(&mut self) -> &mut App {
        &mut self.app
    }

    fn ui_request(&mut self, request: &Map<String, Value>) -> Step {
        let id = request.get("id").cloned().unwrap_or(Value::Null);
        match self.ui_action(request) {
            Ok(UiStep::Answer(result)) => Step::Done(Some(canonical(
                &json!({"id": id, "ok": true, "result": result}),
            ))),
            Ok(UiStep::Settle(input)) => Step::Frames(Pending::settle(Some(id), input)),
            Ok(UiStep::Exactly(count)) => Step::Frames(Pending::exactly(id, count)),
            Err((kind, message)) => Step::Done(Some(canonical(&json!({
                "error": {"code": "ui", "kind": kind, "message": cap(&message)},
                "id": id,
                "ok": false,
            })))),
        }
    }

    fn ui_action(&mut self, request: &Map<String, Value>) -> Result<UiStep, Refusal> {
        let action = parse(request)?;
        let (screen, update, shapes) = (self.screen, self.last_update.as_ref(), &self.last_shapes);
        let observation: &Observation = self
            .observation
            .get_or_insert_with(|| Observation::new(update, shapes, screen));
        let input = match action {
            UiAction::Snapshot => {
                return Ok(UiStep::Answer(
                    json!({"frames": self.frames, "root": observation.to_json(0)}),
                ));
            }
            UiAction::Frames(count) => return Ok(UiStep::Exactly(count)),
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
                if observation.nodes[node].offscreen {
                    // A user cannot click what is not visible (D-149 n8): scroll first.
                    return Err(not_actionable("the target is offscreen"));
                }
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
        Ok(UiStep::Settle(input))
    }

    /// Run the frames of `pending` with simulated time, settling at most [`SETTLE_CAP`] frames.
    fn run_frames(&mut self, pending: &mut Pending) {
        while let Some(events) = pending.next_input(|run| run < SETTLE_CAP) {
            let wants_more = self.frame(events);
            pending.after_frame(wants_more);
        }
    }

    /// Run one frame with `events`; true if the UI asks for a further frame.
    fn frame(&mut self, events: Vec<Event>) -> bool {
        let input = headless_input(self.screen, self.frames, events);
        let app = &mut self.app;
        let mut output = self.ctx.run_ui(input, |ui| draw_frame(ui, app));
        // No renderer: the font atlas updates are not needed (nothing is drawn as pixels).
        output.textures_delta.clear();
        let wants_more = output
            .viewport_output
            .get(&ViewportId::ROOT)
            .is_some_and(|v| v.repaint_delay != Duration::MAX);
        self.record_frame(
            output.platform_output.accesskit_update,
            output.shapes,
            self.screen,
        );
        wants_more
    }

    /// Record what one frame produced: its accessibility update, its shapes and the window's
    /// rectangle in points. The next UI request observes it.
    pub(crate) fn record_frame(
        &mut self,
        update: Option<TreeUpdate>,
        shapes: Vec<ClippedShape>,
        screen: Rect,
    ) {
        self.frames += 1;
        self.screen = screen;
        self.last_update = update;
        self.last_shapes = shapes;
        self.observation = None;
    }
}

/// What a valid UI action needs.
enum UiStep {
    /// Answered without a frame (`snapshot`).
    Answer(Value),
    /// One frame per entry, then settling.
    Settle(Vec<Vec<Event>>),
    /// Exactly this many frames (`frames`).
    Exactly(u64),
}

/// The input of frame number `frame` (from 0) of `kiln script`: the window `screen`, simulated
/// time `frame / 60` s, and `events`.
pub(crate) fn headless_input(screen: Rect, frame: u64, events: Vec<Event>) -> RawInput {
    let mut viewports = egui::ViewportIdMap::default();
    viewports.insert(
        ViewportId::ROOT,
        ViewportInfo {
            native_pixels_per_point: Some(1.0),
            focused: Some(true),
            ..ViewportInfo::default()
        },
    );
    RawInput {
        viewport_id: ViewportId::ROOT,
        viewports,
        screen_rect: Some(screen),
        max_texture_side: Some(8192),
        time: Some(frame as f64 / 60.0),
        predicted_dt: 1.0 / 60.0,
        events,
        focused: true,
        ..RawInput::default()
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

/// Where a click on `node` lands: the centre of its visible part, or with `at = (r, c)` the left edge of char `c`
/// of its painted row `r` (just right of the last char when `c` is the row's length).
fn click_point(
    observation: &Observation,
    node: usize,
    at: Option<(usize, usize)>,
) -> Result<Pos2, Refusal> {
    let n = &observation.nodes[node];
    let Some((row, col)) = at else {
        // The centre of the visible part (a user clicks what is visible, D-150 w5).
        return Ok(n.visible.or(n.rect).map_or(Pos2::ZERO, |r| r.center()));
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

#[cfg(test)]
mod tests {
    use super::*;

    const LINES: &[&[u8]] = &[
        br#"{"id":1,"command":"doc.list"}"#,
        br#"{"id":2,"command":"buffer.insert","params":{"offset":0,"text":"fn main() {}"}}"#,
        br#"{"id":3,"command":"doc.open","params":{"path":"missing.rs"}}"#,
        b"",
        b"  \r",
        b"not json",
        b"\xff\xfe",
        b"[1]",
        br#"{"id":1.5,"ui":"snapshot"}"#,
        br#"{"id":[],"command":"doc.list"}"#,
        br#"{"id":"x","command":"nope"}"#,
        br#"{"id":"y","command":"buffer.insert","params":{}}"#,
        br#"{"id":4,"command":"buffer.text"}"#,
        br#"{"id":5,"command":"doc.list","extra":1}"#,
    ];

    #[test]
    fn control_lines_get_the_hosts_bytes_and_log() {
        let mut script = Script::new(Files::Memory, DEFAULT_SIZE).unwrap();
        let mut host = Host::in_memory(&[]).unwrap();
        for line in LINES {
            assert_eq!(script.handle_line(line), host.handle_line(line));
        }
        assert_eq!(script.log_script(), host.log_script());
    }

    #[test]
    fn only_ui_actions_run_frames() {
        let mut script = Script::new(Files::Memory, DEFAULT_SIZE).unwrap();
        assert_eq!(script.frames(), 0, "no frame before the start");
        script.start(&[]);
        let start = script.frames();
        assert!(start >= 1);
        script.handle_line(br#"{"command":"doc.open","params":{"path":"missing.rs"}}"#);
        script.handle_line(br#"{"command":"doc.list"}"#);
        script.handle_line(br#"{"ui":"snapshot"}"#);
        assert_eq!(script.frames(), start);
        script.handle_line(br#"{"ui":"frames","count":0}"#);
        script.handle_line(br#"{"ui":"type","text":"x"}"#);
        assert_eq!(script.frames(), start);
        script.handle_line(br#"{"ui":"frames","count":3}"#);
        assert_eq!(script.frames(), start + 3);
    }

    /// A session over an in-memory file system with `n` open documents, after one frame.
    fn with_documents(n: usize, size: (u32, u32)) -> Script {
        let names: Vec<String> = (0..n).map(|i| format!("f{i:02}.txt")).collect();
        let files: Vec<(&str, &[u8])> = names.iter().map(|p| (p.as_str(), &b"x\n"[..])).collect();
        let host = Host::in_memory(&files).unwrap();
        let mut script = Script::new(Files::Memory, size).unwrap();
        script.app = App::new(host);
        for name in &names {
            let line = format!("{{\"command\":\"doc.open\",\"params\":{{\"path\":\"{name}\"}}}}");
            assert!(response_ok(&script.handle_line(line.as_bytes()).unwrap()));
        }
        script.handle_line(br#"{"ui":"frames","count":1}"#);
        script
    }

    #[test]
    fn a_click_on_an_offscreen_target_is_refused_and_runs_no_frame() {
        let mut script = with_documents(40, (400, 300));
        let snapshot = |s: &mut Script| s.handle_line(br#"{"ui":"snapshot"}"#).unwrap();
        let before = snapshot(&mut script);
        assert!(before.contains(r#""offscreen":true"#));
        let frames = script.frames();
        let refused = script
            .handle_line(br#"{"ui":"click","target":{"value":"f39.txt"}}"#)
            .unwrap();
        assert!(refused.contains(r#""kind":"not_actionable""#), "{refused}");
        let refused = script
            .handle_line(br#"{"ui":"click","target":{"value":"f39.txt"},"at":{"row":0,"col":0}}"#)
            .unwrap();
        assert!(refused.contains(r#""kind":"not_actionable""#), "{refused}");
        assert_eq!(script.frames(), frames);
        assert_eq!(snapshot(&mut script), before);
        // A visible one is clicked.
        let ok = script
            .handle_line(br#"{"ui":"click","target":{"value":"f00.txt"}}"#)
            .unwrap();
        assert!(response_ok(&ok), "{ok}");
    }

    /// Every node of a snapshot response, in pre-order.
    fn all_nodes(snapshot: &str) -> Vec<Value> {
        fn walk(n: &Value, out: &mut Vec<Value>) {
            out.push(n.clone());
            for c in n["children"].as_array().into_iter().flatten() {
                walk(c, out);
            }
        }
        let v: Value = serde_json::from_str(snapshot).unwrap();
        let mut out = Vec::new();
        walk(&v["result"]["root"], &mut out);
        out
    }

    fn row_text(node: &Value) -> Vec<String> {
        node["rows"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|row| {
                row.as_array()
                    .unwrap()
                    .iter()
                    .map(|run| run["text"].as_str().unwrap().to_owned())
                    .collect()
            })
            .collect()
    }

    #[test]
    fn after_a_scroll_text_belongs_to_visible_nodes_only() {
        let mut script = with_documents(40, (400, 300));
        let scrolled = script
            .handle_line(
                br#"{"ui":"scroll","target":{"value":"f00.txt"},"direction":"down","steps":2}"#,
            )
            .unwrap();
        assert!(response_ok(&scrolled), "{scrolled}");
        let snapshot = script.handle_line(br#"{"ui":"snapshot"}"#).unwrap();
        let nodes = all_nodes(&snapshot);
        let path_label = nodes
            .iter()
            .find(|n| n["role"] == "label" && n["value"] == "Path")
            .unwrap();
        assert_eq!(row_text(path_label), ["Path"]);
        let documents = nodes.iter().find(|n| n["label"] == "Documents").unwrap();
        assert!(documents.get("rows").is_none(), "{documents}");
        for n in &nodes {
            if n.get("offscreen").is_some() {
                assert!(n.get("rows").is_none(), "{n}");
            }
        }
        let docs = documents["children"].as_array().unwrap();
        assert!(
            docs[0].get("offscreen").is_some(),
            "the first document scrolled out"
        );
        for d in docs.iter().filter(|d| d.get("rows").is_some()) {
            assert_eq!(row_text(d), [d["value"].as_str().unwrap()]);
        }
        assert!(docs.iter().any(|d| d.get("rows").is_some()));
    }

    #[test]
    fn a_click_lands_at_the_centre_of_the_visible_part() {
        let mut script = with_documents(40, (400, 300));
        script.handle_line(br#"{"ui":"snapshot"}"#).unwrap();
        let obs = script.observation.as_ref().unwrap();
        let list = obs
            .nodes
            .iter()
            .position(|n| n.label.as_deref() == Some("Documents"))
            .unwrap();
        let (rect, visible) = (
            obs.nodes[list].rect.unwrap(),
            obs.nodes[list].visible.unwrap(),
        );
        // The list overflows: the centre of its whole rectangle is below the window.
        assert!(rect.center().y > 300.0, "{rect:?}");
        let at = click_point(obs, list, None).unwrap();
        assert_eq!(at, visible.center());
        assert!(at.y < 300.0);
        for (i, n) in obs.nodes.iter().enumerate() {
            if let Some(v) = n.visible {
                assert!(v.contains(click_point(obs, i, None).unwrap()), "{i}");
            }
        }
    }

    #[test]
    fn size_values_are_checked() {
        assert_eq!(parse_size("1280x800"), Some((1280, 800)));
        assert_eq!(parse_size("200x8192"), Some((200, 8192)));
        for bad in [
            "199x800", "800x8193", "800", "x800", "800x", "+800x800", "8 x8", "1e3x800",
        ] {
            assert_eq!(parse_size(bad), None, "{bad}");
        }
    }
}
