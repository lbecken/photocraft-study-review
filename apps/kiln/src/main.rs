//! # kiln — desktop application (thin binary)
//!
//! ```text
//! kiln [--root <dir>] [--script <file>] [<path>…]
//! kiln script <script> [--root <dir> | --memory] [--log <file>] [--size <w>x<h>] [<path>…]
//! kiln --help | -h | script --help
//! ```
//!
//! Without a subcommand `kiln` opens the Kiln UI in a window on X11 (T-065,
//! [`kiln_ui::run_window`]) over the root, opens each `<path>` at start as `kiln script` does,
//! and runs until the window is closed (exit 0). With `--script <file>` the window takes its
//! input from the script, closes after the last line and prints the responses as `kiln script`
//! (exit 0, or 3 with an `ok:false` response). If the window system cannot be started: exit 2,
//! one line on stderr, nothing on stdout. If the window is closed before the script ends, the
//! responses of the lines that ran are printed, one stderr line says so, exit 2.
//!
//! `kiln script` runs the Kiln UI without a window from a script of control requests and UI
//! requests (`docs/protocol/ui.md`); the paths after the script are opened at start, in order,
//! as if each were opened through `Path`.
//!
//! Exit codes of `script`: 0 every response `ok:true`; 3 at least one `ok:false`; 2 usage
//! error, unreadable script, bad root, log file not writable (message on stderr, nothing on
//! stdout); 1 writing stdout failed. `--help`: 0 (1 if stdout cannot be written).

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use kiln_ui::{
    DEFAULT_SIZE, Files, SIZE_RANGE, Script, WindowError, parse_size, response_ok, run_window,
};

const USAGE: &str = "usage:
  kiln [--root <dir>] [--script <file>] [<path>…]
  kiln script <script> [--root <dir> | --memory] [--log <file>] [--size <w>x<h>] [<path>…]
  kiln --help | -h | script --help

  (none)  open the Kiln UI in a window on X11 over the root; each <path> is opened at
          start as for `script`. With --script <file>, the window takes its input from
          the script and closes after its last line; the responses are those of
          `script`, except the frame counts

  script  run the Kiln UI without a window: each line of <script> is a control request
          (answered as `kiln-cli control` answers it) or a UI request (an object with a
          key `ui`); one response line per request on stdout. Each <path> (relative to
          the root) is opened at start, in order, as if opened through the UI's `Path`

  --root <dir>      root of the files the session acts on (default: current directory)
  --memory          an empty in-memory file system instead
  --log <file>      after the script, write the successful control requests as a
                    replayable script (as `kiln-cli run --log`)
  --size <w>x<h>    the UI's size in points, each from 200 to 8192 (default 1280x800)

The UI requests are described in docs/protocol/ui.md, the control protocol in
docs/control-protocol.md.";

/// The longest stderr line, in characters (as `kiln-cli`'s messages).
const MAX_LINE: usize = 200;

/// What one invocation produced.
struct Outcome {
    code: u8,
    stdout: String,
    stderr: String,
}

impl Outcome {
    fn err(code: u8, stderr: String) -> Self {
        Outcome {
            code,
            stdout: String::new(),
            stderr,
        }
    }

    fn usage(reason: impl std::fmt::Display) -> Self {
        Self::err(2, format!("kiln: {reason}\n{USAGE}\n"))
    }
}

/// Options of `kiln script`.
#[derive(Default)]
struct Opts {
    root: Option<String>,
    memory: bool,
    log: Option<String>,
    size: Option<String>,
}

/// Split the arguments of `script` into positionals and options (before or after the
/// positional, each at most once). On a usage error, what was wrong.
fn parse_opts(args: &[String]) -> Result<(Vec<&str>, Opts), String> {
    let mut opts = Opts::default();
    let mut positional = Vec::new();
    let mut rest = args;
    while let [arg, tail @ ..] = rest {
        rest = tail;
        if !arg.starts_with("--") {
            positional.push(arg.as_str());
            continue;
        }
        if arg == "--memory" {
            if opts.memory {
                return Err("option `--memory` given twice".to_owned());
            }
            opts.memory = true;
            continue;
        }
        let slot = match arg.as_str() {
            "--root" => &mut opts.root,
            "--log" => &mut opts.log,
            "--size" => &mut opts.size,
            _ => return Err(format!("unknown option `{arg}` for `script`")),
        };
        let [value, tail @ ..] = rest else {
            return Err(format!("option `{arg}` needs a value"));
        };
        rest = tail;
        if slot.replace(value.clone()).is_some() {
            return Err(format!("option `{arg}` given twice"));
        }
    }
    if opts.memory && opts.root.is_some() {
        return Err("options `--root` and `--memory` cannot be combined".to_owned());
    }
    Ok((positional, opts))
}

/// `--help` or `-h` as the first argument, or in an option position of `script` or of the
/// window form (not as the value of an option).
fn wants_help(args: &[String]) -> bool {
    let is_help = |a: &str| a == "--help" || a == "-h";
    let (rest, valued): (&[String], &[&str]) = match args.first().map(String::as_str) {
        Some(first) if is_help(first) => return true,
        Some("script") => (&args[1..], &["--root", "--log", "--size"]),
        _ => (args, &["--root", "--script", "--log", "--size"]),
    };
    let mut takes_value = false;
    for a in rest {
        if takes_value {
            takes_value = false;
        } else if is_help(a) {
            return true;
        } else {
            takes_value = valued.contains(&a.as_str());
        }
    }
    false
}

/// Bare `kiln`: the window (T-065), in the order of checks usage → `--script` readable → root
/// → window system → start paths → first frame.
fn window(args: &[String]) -> Outcome {
    let mut root: Option<&str> = None;
    let mut script: Option<&str> = None;
    let mut paths = Vec::new();
    let mut rest = args;
    while let [arg, tail @ ..] = rest {
        rest = tail;
        if !arg.starts_with("--") {
            paths.push(arg.as_str());
            continue;
        }
        let slot = match arg.as_str() {
            "--root" => &mut root,
            "--script" => &mut script,
            "--log" | "--memory" | "--size" => {
                return Outcome::usage(format!("option `{arg}` is only for `kiln script`"));
            }
            _ => return Outcome::usage(format!("unknown option `{arg}`")),
        };
        let [value, tail @ ..] = rest else {
            return Outcome::usage(format!("option `{arg}` needs a value"));
        };
        rest = tail;
        if slot.replace(value.as_str()).is_some() {
            return Outcome::usage(format!("option `{arg}` given twice"));
        }
    }
    let bytes = match script {
        None => None,
        Some(path) => match std::fs::read(path) {
            Ok(b) => Some(b),
            Err(e) => {
                return Outcome::err(2, format!("kiln: cannot read script `{path}`: {e}\n"));
            }
        },
    };
    let root = root.unwrap_or(".");
    match run_window(Path::new(root), &paths, bytes.as_deref()) {
        Ok(None) => Outcome {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
        },
        Ok(Some(run)) => Outcome {
            code: match (run.completed, run.failed) {
                (false, _) => 2,
                (true, true) => 3,
                (true, false) => 0,
            },
            stdout: run.responses,
            stderr: if run.completed {
                String::new()
            } else {
                "kiln: the window was closed before the script ended\n".to_owned()
            },
        },
        Err(WindowError::BadRoot(e)) => Outcome::err(2, format!("kiln: bad --root: {e}\n")),
        Err(WindowError::NoWindow(e)) => Outcome::err(2, format!("kiln: no window: {e}\n")),
    }
}

/// `kiln script`, in the order of checks of `docs/protocol/ui.md`: usage, script readable,
/// root, log writable, start paths, the first frames, requests. Responses are buffered so that a late failure (writing the
/// log) leaves stdout empty.
fn script(args: &[String]) -> Outcome {
    let (positional, opts) = match parse_opts(args) {
        Ok(parsed) => parsed,
        Err(reason) => return Outcome::usage(reason),
    };
    let (script_path, start_paths) = match positional.as_slice() {
        [] => return Outcome::usage("missing argument <script>"),
        [script, paths @ ..] => (*script, paths),
    };
    let size = match &opts.size {
        None => DEFAULT_SIZE,
        Some(text) => match parse_size(text) {
            Some(size) => size,
            None => {
                return Outcome::usage(format!(
                    "bad --size `{text}`: expected <w>x<h>, integers from {} to {}",
                    SIZE_RANGE.start(),
                    SIZE_RANGE.end()
                ));
            }
        },
    };
    let bytes = match std::fs::read(script_path) {
        Ok(b) => b,
        Err(e) => {
            return Outcome::err(
                2,
                format!("kiln: cannot read script `{script_path}`: {e}\n"),
            );
        }
    };
    let root = opts.root.as_deref().unwrap_or(".");
    let files = if opts.memory {
        Files::Memory
    } else {
        Files::Disk(Path::new(root))
    };
    // The host checks the root before any frame or request runs.
    let mut session = match Script::new(files, size) {
        Ok(s) => s,
        Err(e) if opts.memory => return Outcome::err(1, format!("kiln: {e}\n")),
        Err(e) => return Outcome::err(2, format!("kiln: bad --root: {e}\n")),
    };
    let mut log_file = match &opts.log {
        None => None,
        Some(path) => match std::fs::File::create(path) {
            Ok(f) => Some((path, f)),
            Err(e) => return Outcome::err(2, format!("kiln: cannot write log `{path}`: {e}\n")),
        },
    };
    // Start paths get no response line and do not change the exit code.
    session.start(start_paths);
    let mut stdout = String::new();
    let mut failed = false;
    for line in bytes.split(|b| *b == b'\n') {
        let Some(response) = session.handle_line(line) else {
            continue;
        };
        failed |= !response_ok(&response);
        stdout.push_str(&response);
        stdout.push('\n');
    }
    if let Some((path, file)) = &mut log_file {
        let written = file
            .write_all(session.log_script().as_bytes())
            .and_then(|()| file.flush());
        if let Err(e) = written {
            return Outcome::err(2, format!("kiln: cannot write log `{path}`: {e}\n"));
        }
    }
    Outcome {
        code: if failed { 3 } else { 0 },
        stdout,
        stderr: String::new(),
    }
}

/// Every stderr line cut to [`MAX_LINE`] characters.
fn bound_stderr(stderr: &str) -> String {
    let mut out = String::with_capacity(stderr.len());
    for line in stderr.split_inclusive('\n') {
        let (body, end) = match line.strip_suffix('\n') {
            Some(body) => (body, "\n"),
            None => (line, ""),
        };
        if body.chars().count() > MAX_LINE {
            out.extend(body.chars().take(MAX_LINE - 1));
            out.push('…');
        } else {
            out.push_str(body);
        }
        out.push_str(end);
    }
    out
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let outcome = if wants_help(&args) {
        Outcome {
            code: 0,
            stdout: format!("{USAGE}\n"),
            stderr: String::new(),
        }
    } else if args.first().map(String::as_str) == Some("script") {
        script(&args[1..])
    } else {
        window(&args)
    };
    let mut out = std::io::stdout().lock();
    let written = out
        .write_all(outcome.stdout.as_bytes())
        .and_then(|()| out.flush());
    let (code, stderr) = match written {
        Ok(()) => (outcome.code, outcome.stderr),
        Err(e) => (
            1,
            format!("{}kiln: cannot write to stdout: {e}\n", outcome.stderr),
        ),
    };
    let _ = std::io::stderr().write_all(bound_stderr(&stderr).as_bytes());
    ExitCode::from(code)
}
