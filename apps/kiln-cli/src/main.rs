//! # kiln-cli — headless application (thin binary)
//!
//! Everything the UI can do, the CLI can do (CHARTER §3 decision 1): it exposes the command
//! registry of `kiln-commands` and the JSON-lines control protocol of `kiln-control`
//! (`docs/control-protocol.md`).
//!
//! ```text
//! kiln-cli commands              list command names and docs
//! kiln-cli commands --schema     print every command with params/result schemas and examples
//! kiln-cli commands --audit      check every command's examples against its schemas
//! kiln-cli exec <name> <json> [--text <initial buffer text>] [--root <dir> | --memory]
//! kiln-cli run <script> [--root <dir> | --memory] [--log <file>]
//! kiln-cli control [--root <dir>]
//! ```
//!
//! `--root` (default: the current directory) is the root of the disk file system the commands
//! act on; paths inside commands are relative to it. `--memory` selects an empty in-memory
//! file system instead (for tests). `<script>` and `--log <file>` are host paths, resolved
//! against the current directory, not against `--root`.
//!
//! Exit codes: `commands`/`exec`: 0 success, 1 audit problems or internal error, 2 usage or
//! command error. `run`: 0 every response ok, 3 at least one failed request, 2 usage error,
//! unreadable script, bad root or unwritable log (nothing on stdout). `control`: 0 at end of
//! input, 2 usage error or bad root, 1 if reading stdin or writing stdout fails.

use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use kiln_commands::{Context, Registry, standard_registry};
use kiln_control::Host;
use kiln_workspace::DiskFs;
use serde_json::{Value, json};

const USAGE: &str = "usage:
  kiln-cli commands [--schema | --audit]
  kiln-cli exec <name> <json-params> [--text <initial-text>] [--root <dir> | --memory]
  kiln-cli run <script> [--root <dir> | --memory] [--log <file>]
  kiln-cli control [--root <dir>]";

/// Result of one CLI invocation, separated from process I/O so it can be unit-tested.
#[derive(Debug, Default, PartialEq, Eq)]
struct Outcome {
    code: u8,
    stdout: String,
    stderr: String,
}

impl Outcome {
    fn ok(stdout: String) -> Self {
        Self {
            code: 0,
            stdout,
            stderr: String::new(),
        }
    }
    fn err(code: u8, stderr: String) -> Self {
        Self {
            code,
            stdout: String::new(),
            stderr,
        }
    }
    fn usage() -> Self {
        Self::err(2, format!("{USAGE}\n"))
    }
}

fn pretty(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string())
}

/// Where commands find their files.
#[derive(Debug, PartialEq, Eq)]
enum Fs {
    Disk(PathBuf),
    Memory,
}

/// Options after the positional arguments of a subcommand.
#[derive(Debug, Default)]
struct Opts {
    root: Option<String>,
    memory: bool,
    log: Option<String>,
    text: Option<String>,
}

impl Opts {
    fn fs(&self) -> Fs {
        if self.memory {
            Fs::Memory
        } else {
            Fs::Disk(PathBuf::from(self.root.as_deref().unwrap_or(".")))
        }
    }
}

/// Split `args` into positional arguments and options: `--memory`, or `--flag value` for the
/// other flags; `allowed` names the flags this subcommand takes, options may come before,
/// between or after positionals. Each flag at most once; `--root` and `--memory` exclude each
/// other. None on a usage error.
fn parse_opts<'a>(args: &[&'a str], allowed: &[&str]) -> Option<(Vec<&'a str>, Opts)> {
    let mut opts = Opts::default();
    let mut positional = Vec::new();
    let mut rest = args;
    while let [arg, tail @ ..] = rest {
        rest = tail;
        if !arg.starts_with("--") {
            positional.push(*arg);
            continue;
        }
        if !allowed.contains(arg) {
            return None;
        }
        if *arg == "--memory" {
            if opts.memory {
                return None;
            }
            opts.memory = true;
            continue;
        }
        let [value, tail @ ..] = rest else {
            return None;
        };
        rest = tail;
        let slot = match *arg {
            "--root" => &mut opts.root,
            "--log" => &mut opts.log,
            "--text" => &mut opts.text,
            _ => return None,
        };
        if slot.replace((*value).to_owned()).is_some() {
            return None;
        }
    }
    if opts.memory && opts.root.is_some() {
        return None;
    }
    Some((positional, opts))
}

fn bad_root(e: impl std::fmt::Display) -> Outcome {
    Outcome::err(2, format!("kiln-cli: bad --root: {e}\n"))
}

fn exec(registry: &Registry, name: &str, params: &str, opts: &Opts) -> Outcome {
    let params: Value = match serde_json::from_str(params) {
        Ok(v) => v,
        Err(e) => {
            return Outcome::err(
                2,
                format!(
                    "{}\n",
                    json!({"code": "invalid_json", "message": e.to_string()})
                ),
            );
        }
    };
    let text = opts.text.as_deref().unwrap_or("");
    let mut ctx = match opts.fs() {
        Fs::Memory => Context::with_text(text),
        Fs::Disk(root) => {
            let fs = match DiskFs::new(&root) {
                Ok(fs) => fs,
                Err(e) => return bad_root(e),
            };
            let mut ctx = Context::with_vfs(Box::new(fs));
            // The initial scratch text enters through the registry like any other change.
            if !text.is_empty()
                && let Err(e) = registry.execute(
                    &mut ctx,
                    "buffer.insert",
                    json!({"offset": 0, "text": text}),
                )
            {
                return Outcome::err(1, format!("{}\n", e.to_json()));
            }
            ctx
        }
    };
    match registry.execute(&mut ctx, name, params) {
        Ok(result) => Outcome::ok(format!("{result}\n")),
        Err(e) => Outcome::err(2, format!("{}\n", e.to_json())),
    }
}

fn host_for(fs: &Fs) -> Result<Host, Outcome> {
    match fs {
        Fs::Memory => Host::in_memory(&[]).map_err(|e| Outcome::err(1, format!("kiln-cli: {e}\n"))),
        Fs::Disk(root) => Host::on_disk(root).map_err(bad_root),
    }
}

/// `kiln-cli run`: every response is buffered so that a late failure (writing the log) leaves
/// stdout empty, as the exit-2 contract requires.
fn run_script(script: &str, opts: &Opts) -> Outcome {
    let mut host = match host_for(&opts.fs()) {
        Ok(h) => h,
        Err(o) => return o,
    };
    let bytes = match std::fs::read(script) {
        Ok(b) => b,
        Err(e) => {
            return Outcome::err(2, format!("kiln-cli: cannot read script `{script}`: {e}\n"));
        }
    };
    // Open (create/truncate) the log before running, so an unwritable log file is refused
    // before any command touches the files.
    let mut log_file = match &opts.log {
        None => None,
        Some(path) => match std::fs::File::create(path) {
            Ok(f) => Some((path, f)),
            Err(e) => {
                return Outcome::err(2, format!("kiln-cli: cannot write log `{path}`: {e}\n"));
            }
        },
    };
    let mut stdout = String::new();
    let mut failed = false;
    for line in bytes.split(|b| *b == b'\n') {
        let Some(response) = host.handle_line(line) else {
            continue;
        };
        failed |= !response_ok(&response);
        stdout.push_str(&response);
        stdout.push('\n');
    }
    if let Some((path, file)) = &mut log_file {
        let written = file
            .write_all(host.log_script().as_bytes())
            .and_then(|()| file.flush());
        if let Err(e) = written {
            return Outcome::err(2, format!("kiln-cli: cannot write log `{path}`: {e}\n"));
        }
    }
    Outcome {
        code: if failed { 3 } else { 0 },
        stdout,
        stderr: String::new(),
    }
}

fn response_ok(response: &str) -> bool {
    serde_json::from_str::<Value>(response).is_ok_and(|v| v["ok"] == Value::Bool(true))
}

/// `kiln-cli control`: answer each request line before reading the next, flushing every
/// response. Ends at end of input.
fn control(
    host: &mut Host,
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> std::io::Result<()> {
    let mut line = Vec::new();
    loop {
        line.clear();
        if input.read_until(b'\n', &mut line)? == 0 {
            return Ok(());
        }
        let request = line.strip_suffix(b"\n").unwrap_or(&line);
        if let Some(response) = host.handle_line(request) {
            output.write_all(response.as_bytes())?;
            output.write_all(b"\n")?;
            output.flush()?;
        }
    }
}

/// The subcommands that only produce an [`Outcome`] (everything but `control`).
fn run(args: &[String]) -> Outcome {
    let registry = match standard_registry() {
        Ok(r) => r,
        Err(e) => return Outcome::err(1, format!("registry failed to build: {e}\n")),
    };
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["commands"] => {
            let mut out = String::new();
            for name in registry.names() {
                let doc = registry.doc(name).unwrap_or_default();
                out.push_str(&format!("{name}\t{doc}\n"));
            }
            Outcome::ok(out)
        }
        ["commands", "--schema"] => Outcome::ok(format!("{}\n", pretty(&registry.describe()))),
        ["commands", "--audit"] => {
            let problems = registry.audit();
            if problems.is_empty() {
                Outcome::ok(format!(
                    "audit ok: {} commands, all examples validate\n",
                    registry.names().len()
                ))
            } else {
                Outcome::err(1, problems.iter().map(|p| format!("{p}\n")).collect())
            }
        }
        ["exec", rest @ ..] => match parse_opts(rest, &["--text", "--root", "--memory"]) {
            Some((pos, opts)) => match pos.as_slice() {
                [name, params] => exec(&registry, name, params, &opts),
                _ => Outcome::usage(),
            },
            None => Outcome::usage(),
        },
        ["run", rest @ ..] => match parse_opts(rest, &["--root", "--memory", "--log"]) {
            Some((pos, opts)) => match pos.as_slice() {
                [script] => run_script(script, &opts),
                _ => Outcome::usage(),
            },
            None => Outcome::usage(),
        },
        _ => Outcome::usage(),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("control") {
        let rest: Vec<&str> = args[1..].iter().map(String::as_str).collect();
        let opts = match parse_opts(&rest, &["--root"]) {
            Some((pos, opts)) if pos.is_empty() => opts,
            _ => return finish(Outcome::usage()),
        };
        let mut host = match host_for(&opts.fs()) {
            Ok(h) => h,
            Err(o) => return finish(o),
        };
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        return match control(&mut host, &mut stdin.lock(), &mut stdout.lock()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                let _ = writeln!(std::io::stderr(), "kiln-cli: control I/O error: {e}");
                ExitCode::from(1)
            }
        };
    }
    finish(run(&args))
}

fn finish(outcome: Outcome) -> ExitCode {
    // Write failures (closed pipe) cannot be reported anywhere useful; the exit code stands.
    let _ = std::io::stdout().write_all(outcome.stdout.as_bytes());
    let _ = std::io::stderr().write_all(outcome.stderr.as_bytes());
    ExitCode::from(outcome.code)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(args: &[&str]) -> Outcome {
        run(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn exec_insert_reports_new_length() {
        let out = call(&["exec", "buffer.insert", r#"{"offset":0,"text":"hi"}"#]);
        assert_eq!(out, Outcome::ok("{\"len_chars\":2}\n".into()));
    }

    #[test]
    fn exec_text_with_initial_text() {
        let out = call(&["exec", "buffer.text", "{}", "--text", "abc"]);
        assert_eq!(out.code, 0);
        let v: Value = serde_json::from_str(&out.stdout).unwrap();
        assert_eq!(v["text"], "abc");
    }

    #[test]
    fn exec_invalid_params_exits_2_with_structured_error() {
        let out = call(&["exec", "buffer.insert", r#"{"offset":"zero"}"#]);
        assert_eq!(out.code, 2);
        let v: Value = serde_json::from_str(&out.stderr).unwrap();
        assert_eq!(v["code"], "invalid_params");
    }

    #[test]
    fn exec_unknown_command_and_bad_json() {
        assert_eq!(call(&["exec", "no.such", "{}"]).code, 2);
        let out = call(&["exec", "buffer.text", "{"]);
        assert_eq!(out.code, 2);
        assert!(out.stderr.contains("invalid_json"));
    }

    #[test]
    fn schema_lists_every_command() {
        let out = call(&["commands", "--schema"]);
        let v: Value = serde_json::from_str(&out.stdout).unwrap();
        let names: Vec<&str> = v["commands"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, standard_registry().unwrap().names());
        assert!(names.contains(&"buffer.insert") && names.contains(&"buffer.text"));
    }

    #[test]
    fn audit_passes() {
        assert_eq!(call(&["commands", "--audit"]).code, 0);
    }

    #[test]
    fn options_parse_in_any_order_and_reject_conflicts() {
        let (pos, o) =
            parse_opts(&["--root", "d", "s", "--log", "l"], &["--root", "--log"]).unwrap();
        assert_eq!(pos, ["s"]);
        assert_eq!(
            (o.root.as_deref(), o.log.as_deref()),
            (Some("d"), Some("l"))
        );
        assert_eq!(o.fs(), Fs::Disk(PathBuf::from("d")));
        let (_, o) = parse_opts(&[], &["--root"]).unwrap();
        assert_eq!(o.fs(), Fs::Disk(PathBuf::from(".")));
        let all = ["--root", "--memory", "--log"];
        assert!(parse_opts(&["--root", "a", "--memory"], &all).is_none());
        assert!(parse_opts(&["--root", "a", "--root", "b"], &all).is_none());
        assert!(parse_opts(&["--memory", "--memory"], &all).is_none());
        assert!(parse_opts(&["--root"], &all).is_none());
        assert!(parse_opts(&["--text", "x"], &all).is_none());
    }

    #[test]
    fn exec_memory_and_root_options() {
        let out = call(&["exec", "buffer.text", "{}", "--memory", "--text", "abc"]);
        assert_eq!(out.code, 0);
        let out = call(&["exec", "doc.open", r#"{"path":"a.txt"}"#, "--memory"]);
        assert_eq!(out.code, 2);
        assert!(out.stderr.contains("rejected"), "{out:?}");
        assert_eq!(
            call(&["exec", "doc.list", "{}", "--root", "/no/such/kiln/dir"]).code,
            2
        );
        assert_eq!(
            call(&["exec", "doc.list", "{}", "--root", "a", "--memory"]).code,
            2
        );
        assert_eq!(call(&["exec", "doc.list"]).code, 2);
    }

    #[test]
    fn control_answers_and_flushes_each_line() {
        let mut host = Host::in_memory(&[("a.txt", b"x")]).unwrap();
        let input = b"{\"id\":1,\"command\":\"doc.open\",\"params\":{\"path\":\"a.txt\"}}\n\n  \nbad\n{\"command\":\"doc.list\"}";
        let mut out = Vec::new();
        control(&mut host, &mut &input[..], &mut out).unwrap();
        let out = String::from_utf8(out).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 3, "{out}");
        assert!(lines[0].starts_with(r#"{"id":1,"ok":true"#));
        assert!(lines[1].contains("invalid_json"));
        assert!(lines[2].contains(r#""ok":true"#));
        assert!(out.ends_with('\n'));
    }

    #[test]
    fn run_with_memory_and_unreadable_script() {
        let out = call(&["run", "/no/such/kiln/script.jsonl", "--memory"]);
        assert_eq!((out.code, out.stdout.as_str()), (2, ""));
        assert_eq!(call(&["run"]).code, 2);
        assert_eq!(call(&["run", "a", "b"]).code, 2);
    }

    #[test]
    fn usage_on_unknown_subcommand() {
        assert_eq!(call(&["frobnicate"]).code, 2);
    }
}
