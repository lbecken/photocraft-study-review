//! The desktop window (T-065): the Kiln UI in a window on X11, drawn by [`crate::draw_frame`]
//! with the style of [`crate::apply_style`], and `--script` run against the real window.
//!
//! Without a script the window shows the UI and takes the real keyboard and pointer until the
//! user closes it. With a script, a [`Driver`] runs the script's lines exactly as `kiln script`
//! does ([`Script`]'s parsing, targets, input and observation), except that the frames are the
//! window's: the input of each frame is added to the window system's input
//! (`eframe::App::raw_input_hook`), and what the frame produced (its accessibility update and
//! shapes) is taken by an egui plugin's `output_hook`. Time is the window's real time (ruling
//! Q-12 A).

use std::cell::{Cell, RefCell};
use std::panic::AssertUnwindSafe;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use egui::accesskit::TreeUpdate;
use egui::epaint::ClippedShape;
use egui::{Event, Rect, ViewportCommand, ViewportId};
use kiln_control::Host;

use crate::script::{DEFAULT_SIZE, Pending, SETTLE_CAP, Script, Step, response_ok};
use crate::{apply_style, draw_frame};

/// Why the window did not run.
#[derive(Debug)]
pub enum WindowError {
    /// The root cannot be used: the host's message, which names the root as given.
    BadRoot(String),
    /// The window system cannot be started (no display, a display that cannot be reached, or
    /// one whose window system fails to start): what failed, on one line.
    NoWindow(String),
}

/// What a window run with a script produced.
#[derive(Debug)]
pub struct ScriptRun {
    /// The response lines, each followed by `"\n"`, in the order of the script's lines.
    pub responses: String,
    /// Whether at least one response is `ok:false`.
    pub failed: bool,
    /// Whether every line of the script ran; `false` when the window was closed first (the
    /// responses are then those of the lines that ran).
    pub completed: bool,
}

/// What one frame of the window produced, as the plugin took it.
#[derive(Default)]
struct Capture {
    /// Passes since the frame began (egui may run two passes in one frame).
    passes: u32,
    update: Option<TreeUpdate>,
    shapes: Vec<ClippedShape>,
    /// The smallest repaint delay of the frame's passes (`Duration::MAX`: none asked).
    repaint: Option<Duration>,
    screen: Option<Rect>,
    /// While true, the plugin asks for the next frame after every pass.
    active: bool,
}

/// What one completed frame left for the driver.
struct Frame {
    update: Option<TreeUpdate>,
    shapes: Vec<ClippedShape>,
    wants_more: bool,
    screen: Rect,
}

impl Capture {
    /// The frame since the last call, if one ran; resets for the next.
    fn take(&mut self) -> Option<Frame> {
        if self.passes == 0 {
            return None;
        }
        self.passes = 0;
        let repaint = self.repaint.take().unwrap_or(Duration::MAX);
        Some(Frame {
            update: self.update.take(),
            shapes: std::mem::take(&mut self.shapes),
            wants_more: repaint != Duration::MAX,
            screen: self.screen.take().unwrap_or(Rect::NOTHING),
        })
    }
}

/// The egui plugin that takes each pass's output for the driver.
struct Observe(Arc<Mutex<Capture>>);

impl egui::Plugin for Observe {
    fn debug_name(&self) -> &'static str {
        "kiln-script"
    }

    fn output_hook(&mut self, ctx: &egui::Context, output: &mut egui::FullOutput) {
        let mut capture = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        capture.passes += 1;
        // As `FullOutput::append` merges the passes of one frame: the last pass's update and
        // shapes, the smallest repaint delay.
        capture.update = output.platform_output.accesskit_update.clone();
        capture.shapes = output.shapes.clone();
        let delay = output
            .viewport_output
            .get(&ViewportId::ROOT)
            .map_or(Duration::MAX, |v| v.repaint_delay);
        capture.repaint = Some(capture.repaint.map_or(delay, |d| d.min(delay)));
        capture.screen = Some(ctx.viewport_rect());
        if capture.active {
            // A non-zero delay: the window draws the next frame soon, and the next frame's own
            // repaint delay is not touched (a zero-delay request would carry over into it).
            ctx.request_repaint_after(Duration::from_millis(1));
        }
    }
}

/// What the driver asks of the coming frame.
#[derive(Debug, PartialEq)]
enum Next {
    /// Run the frame with these events added to its input.
    Input(Vec<Event>),
    /// The input given before has not reached a frame yet (the window system keeps it): add
    /// nothing.
    Wait,
    /// The script has ended: close the window.
    Close,
}

/// A script run against frames that someone else runs (the window): the same [`Script`] as
/// `kiln script`, fed one frame at a time.
struct Driver {
    script: Script,
    lines: Vec<Vec<u8>>,
    next_line: usize,
    /// The action (or the start) whose frames are running.
    current: Option<Pending>,
    /// Input was handed to a frame that has not run yet.
    handed: bool,
    responses: String,
    failed: bool,
    done: bool,
}

impl Driver {
    fn new(script: Script, bytes: &[u8]) -> Driver {
        Driver {
            script,
            lines: bytes.split(|b| *b == b'\n').map(<[u8]>::to_vec).collect(),
            next_line: 0,
            current: None,
            handed: false,
            responses: String::new(),
            failed: false,
            done: false,
        }
    }

    /// Open the start paths; the frames before the first request follow.
    fn start(&mut self, paths: &[&str]) {
        self.current = self.script.begin_start(paths);
    }

    /// Called before every frame of the window with what the previous frame produced, if a
    /// frame ran since the last call; says what the coming frame gets.
    fn before_frame(&mut self, ran: Option<Frame>) -> Next {
        if let Some(frame) = ran {
            self.script
                .record_frame(frame.update, frame.shapes, frame.screen);
            if let Some(pending) = self.current.as_mut() {
                pending.after_frame(frame.wants_more);
            }
            self.handed = false;
        } else if self.handed {
            return Next::Wait;
        }
        loop {
            if let Some(pending) = self.current.as_mut() {
                if let Some(events) = pending.next_input(|run| run < SETTLE_CAP) {
                    self.handed = true;
                    return Next::Input(events);
                }
                if let Some(pending) = self.current.take()
                    && let Some(response) = self.script.finish(pending)
                {
                    self.push(response);
                }
            }
            let Some(line) = self.lines.get(self.next_line) else {
                self.done = true;
                return Next::Close;
            };
            self.next_line += 1;
            match self.script.begin_line(line) {
                Step::Done(Some(response)) => self.push(response),
                Step::Done(None) => {}
                Step::Frames(pending) => self.current = Some(pending),
            }
        }
    }

    fn push(&mut self, response: String) {
        self.failed |= !response_ok(&response);
        self.responses.push_str(&response);
        self.responses.push('\n');
    }

    fn result(self) -> ScriptRun {
        ScriptRun {
            responses: self.responses,
            failed: self.failed,
            completed: self.done,
        }
    }
}

/// The `eframe` application of the window. Without a script the driver only holds the UI and
/// its session (its start paths opened as with a script); its lines are empty and it is never
/// asked for a frame's input.
struct WindowApp<'a> {
    driver: &'a mut Driver,
    capture: Arc<Mutex<Capture>>,
    scripted: bool,
    closing: bool,
    /// The script's input handed to the frame about to run.
    handed: Vec<Event>,
}

impl eframe::App for WindowApp<'_> {
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        if !self.scripted {
            return;
        }
        let next = if self.closing {
            Next::Close
        } else {
            let ran = self
                .capture
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .take();
            self.driver.before_frame(ran)
        };
        // Ruling b1: under a script the window's time is the script's, as in `kiln script`
        // (frame `n` from 0 at `n / 60` s), never the clock, on every frame to the last: what
        // the toolkit times (a double click, a scroll animation, settling) does not depend on
        // the display's frame rate.
        raw_input.time = Some(self.driver.script.frames() as f64 / 60.0);
        raw_input.predicted_dt = 1.0 / 60.0;
        // The input is the script's only, as in `kiln script`: the window system's events
        // (pointer, keys, focus changes) are dropped and the window counts as focused. A close
        // request is not an event here (it is in the viewport's info) and still closes.
        raw_input.focused = true;
        if let Some(info) = raw_input.viewports.get_mut(&ViewportId::ROOT) {
            info.focused = Some(true);
        }
        match next {
            Next::Input(events) => {
                raw_input.events.clone_from(&events);
                self.handed = events;
            }
            // The frame given input before has not run; eframe keeps that input with whatever
            // came since: give it the script's input again, and nothing else.
            Next::Wait => raw_input.events.clone_from(&self.handed),
            Next::Close => {
                raw_input.events.clear();
                self.closing = true;
                self.capture
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .active = false;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        draw_frame(ui, self.driver.script.app_mut());
    }
}

/// Open the Kiln UI in a window on X11 over the files below `root`, open `paths` in order as
/// `kiln script`'s start paths, and run until the window is closed; with `script` (the bytes
/// of a script file), run its lines against the window and close it after the last one.
///
/// Order of checks: the root (before anything of the window system), then the window system,
/// then the start paths, then the first frame. Returns `Ok(None)` after a window without a
/// script was closed, `Ok(Some(run))` after a script, and [`WindowError`] when the root cannot
/// be used or the window system cannot be started. A panic while the window system starts (a
/// library it loads at run time is missing) is caught and reported as
/// [`WindowError::NoWindow`] without the panic message being printed; once the window has
/// started, a panic is not caught.
pub fn run_window(
    root: &Path,
    paths: &[&str],
    script: Option<&[u8]>,
) -> Result<Option<ScriptRun>, WindowError> {
    let host = Host::on_disk(root).map_err(|e| WindowError::BadRoot(e.to_string()))?;
    let scripted = script.is_some();
    let mut driver = Driver::new(
        Script::with_host(host, DEFAULT_SIZE),
        script.unwrap_or_default(),
    );
    let capture = Arc::new(Mutex::new(Capture {
        active: true,
        ..Capture::default()
    }));
    let started = Cell::new(false);
    // While the window system starts, a panic is silent (it is reported as one line); the
    // default hook is put back as soon as the window has started.
    let saved_hook = RefCell::new(Some(std::panic::take_hook()));
    std::panic::set_hook(Box::new(|_| {}));
    let restore_hook = || {
        if let Some(hook) = saved_hook.borrow_mut().take() {
            std::panic::set_hook(hook);
        }
    };
    let (width, height) = DEFAULT_SIZE;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Kiln")
            .with_inner_size([width as f32, height as f32]),
        ..eframe::NativeOptions::default()
    };
    let creator_capture = Arc::clone(&capture);
    let driver_ref = &mut driver;
    let creator: eframe::AppCreator<'_> = Box::new(|cc| {
        started.set(true);
        restore_hook();
        apply_style(&cc.egui_ctx);
        // Ruling b2: no key closes the window (egui's quit shortcut, ctrl+Q, is not obeyed);
        // the window closes on the window system's close request.
        cc.egui_ctx.options_mut(|o| o.quit_shortcuts.clear());
        if scripted {
            // eframe asks for a repaint while it sets up (it adds a font provider), so the first
            // frame would count as "asked for immediately" and get a time step of 0 instead of
            // `kiln script`'s 1/60 s; two passes without UI consume that request (measured: the
            // start then settles in the frames of `kiln script`).
            for _ in 0..2 {
                let _ = cc.egui_ctx.run_logic(&egui::RawInput::default(), |_| {});
            }
            cc.egui_ctx.enable_accesskit();
            cc.egui_ctx.add_plugin(Observe(creator_capture));
        }
        // Ruling b4: the start paths are opened by the same code with and without a script.
        driver_ref.start(paths);
        Ok(Box::new(WindowApp {
            driver: driver_ref,
            capture: Arc::clone(&capture),
            scripted,
            closing: false,
            handed: Vec::new(),
        }))
    });
    let ran = std::panic::catch_unwind(AssertUnwindSafe(|| {
        eframe::run_native("Kiln", options, creator)
    }));
    restore_hook();
    match ran {
        Ok(Ok(())) => Ok(scripted.then(|| driver.result())),
        // Every error of `run_native` is one of starting the window system or its renderer.
        Ok(Err(e)) => Err(WindowError::NoWindow(one_line(&e.to_string()))),
        Err(payload) if !started.get() => Err(WindowError::NoWindow(one_line(&panic_text(
            payload.as_ref(),
        )))),
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// The text of a panic's payload.
fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_owned()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "the window system panicked".to_owned()
    }
}

/// `text` on one line: line breaks and runs of white space become one space.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::{Files, headless_input};

    /// Run `bytes` through a [`Driver`] whose frames are run here as `kiln script` runs them
    /// (simulated time, the same input), with the window's plugin taking the output. Returns
    /// the driver's result and the frames run.
    fn drive(bytes: &[u8], paths: &[&str], files: Files<'_>) -> ScriptRun {
        let host = match files {
            Files::Disk(root) => Host::on_disk(root),
            Files::Memory => Host::in_memory(&[]),
        }
        .unwrap();
        let mut driver = Driver::new(Script::with_host(host, DEFAULT_SIZE), bytes);
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        apply_style(&ctx);
        let capture = Arc::new(Mutex::new(Capture {
            active: true,
            ..Capture::default()
        }));
        ctx.add_plugin(Observe(Arc::clone(&capture)));
        driver.start(paths);
        let screen = Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0));
        let mut frame = 0;
        loop {
            let ran = capture.lock().unwrap().take();
            let events = match driver.before_frame(ran) {
                Next::Input(events) => events,
                Next::Wait => panic!("every frame here runs"),
                Next::Close => break,
            };
            let input = headless_input(screen, frame, events);
            let app = driver.script.app_mut();
            let mut output = ctx.run_ui(input, |ui| draw_frame(ui, app));
            output.textures_delta.clear();
            frame += 1;
            assert!(frame < 100_000, "the driver ends");
        }
        driver.result()
    }

    fn headless(bytes: &[u8], paths: &[&str], files: Files<'_>) -> (String, bool) {
        let mut script = Script::new(files, DEFAULT_SIZE).unwrap();
        script.start(paths);
        let mut out = String::new();
        let mut failed = false;
        for line in bytes.split(|b| *b == b'\n') {
            if let Some(r) = script.handle_line(line) {
                failed |= !response_ok(&r);
                out.push_str(&r);
                out.push('\n');
            }
        }
        (out, failed)
    }

    const SCRIPT: &str = concat!(
        r#"{"id":"s0","ui":"snapshot"}"#,
        "\n",
        r#"{"id":"f","ui":"focus","target":{"label":"Path"}}"#,
        "\n\n",
        r#"{"id":"t","ui":"type","text":"b.txt"}"#,
        "\n",
        r#"{"id":"k","ui":"key","key":"Enter"}"#,
        "\n",
        r#"{"id":1,"command":"doc.list"}"#,
        "\n",
        r#"{"id":"c","ui":"click","target":{"label":"Editor"},"at":{"row":0,"col":2}}"#,
        "\n",
        r#"{"id":"y","ui":"type","text":"Z"}"#,
        "\n",
        r#"{"id":"n","ui":"frames","count":3}"#,
        "\n",
        r#"{"id":"r","ui":"scroll","target":{"label":"Editor"},"direction":"down","steps":2}"#,
        "\n",
        r#"{"id":"bad","ui":"click","target":{"label":"nothing"}}"#,
        "\n",
        "not json\n",
        r#"{"id":"s1","ui":"snapshot"}"#,
        "\n",
        r#"{"id":2,"command":"doc.text","params":{"doc":2}}"#,
    );

    #[test]
    fn the_driver_answers_as_kiln_script_with_the_same_frames() {
        let dir = std::env::temp_dir().join(format!("kiln-window-unit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let long: String = (0..200).map(|i| format!("line {i}\n")).collect();
        std::fs::write(dir.join("a.rs"), "fn main() {}\n").unwrap();
        std::fs::write(dir.join("b.txt"), &long).unwrap();
        let paths = ["a.rs", "missing.rs"];
        let run = drive(SCRIPT.as_bytes(), &paths, Files::Disk(&dir));
        // The same files again for the headless run (nothing was saved, but be sure).
        std::fs::write(dir.join("b.txt"), &long).unwrap();
        let (expected, failed) = headless(SCRIPT.as_bytes(), &paths, Files::Disk(&dir));
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(run.completed);
        assert_eq!(run.failed, failed);
        assert!(failed, "the script has refusals");
        assert_eq!(run.responses, expected);
        assert_eq!(expected.lines().count(), 13);
        assert!(
            expected.contains(r#""value":"b.txt — modified — Kiln""#),
            "{expected}"
        );
    }

    #[test]
    fn input_waits_for_a_frame_that_did_not_run() {
        let host = Host::in_memory(&[]).unwrap();
        let mut driver = Driver::new(
            Script::with_host(host, DEFAULT_SIZE),
            br#"{"id":1,"ui":"frames","count":2}"#,
        );
        driver.start(&[]);
        assert_eq!(driver.before_frame(None), Next::Input(Vec::new()));
        // No frame ran (a hidden window): the input given is still on its way.
        assert_eq!(driver.before_frame(None), Next::Wait);
        assert!(driver.responses.is_empty());
        let frame = || Frame {
            update: None,
            shapes: Vec::new(),
            wants_more: false,
            screen: Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0)),
        };
        // The start settled; the request's two frames follow.
        assert_eq!(driver.before_frame(Some(frame())), Next::Input(Vec::new()));
        assert_eq!(driver.before_frame(Some(frame())), Next::Input(Vec::new()));
        assert_eq!(driver.before_frame(Some(frame())), Next::Close);
        let run = driver.result();
        assert!(run.completed && !run.failed);
        assert_eq!(
            run.responses,
            "{\"id\":1,\"ok\":true,\"result\":{\"frames\":2,\"settled\":true}}\n"
        );
    }

    #[test]
    fn errors_are_one_line() {
        assert_eq!(one_line("a\n  b\tc\r\n"), "a b c");
        assert_eq!(panic_text(&"x"), "x");
        assert_eq!(panic_text(&String::from("y")), "y");
    }
}
