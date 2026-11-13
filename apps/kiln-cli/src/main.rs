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
//! kiln-cli exec <name> <json> [--text <initial buffer text>] [--root <dir> | --memory | --memory-seed <manifest>]
//! kiln-cli run <script> [--root <dir> | --memory | --memory-seed <manifest>] [--log <file>]
//! kiln-cli control [--root <dir>]
//! kiln-cli --help | -h | <subcommand> --help
//! ```
//!
//! `--root` (default: the current directory) is the root of the disk file system the commands
//! act on; paths inside commands are relative to it. `--memory` selects an empty in-memory
//! file system instead (for tests); `--memory-seed <manifest>` an in-memory file system holding
//! exactly the files of a JSON manifest (`docs/protocol/cli-memory-seed.md`). `<script>`,
//! `<manifest>` and `--log <file>` are host paths, resolved against the current directory, not
//! against `--root`. `exec` is one command in a fresh
//! session: it can read files, but an edit made with it is never saved (use `run`/`control`).
//!
//! Exit codes: `--help`: 0. `commands`/`exec`: 0 success, 1 audit problems or internal error, 2 usage or
//! command error. `run`: 0 every response ok, 3 at least one failed request, 2 usage error,
//! unreadable script, bad root, bad manifest or unwritable log (nothing on stdout), 1 if writing stdout fails. `control`: 0 at end of
//! input, 2 usage error or bad root, 1 if reading stdin or writing stdout fails.

use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use kiln_commands::{Context, Registry, standard_registry};
use kiln_control::{Host, cap_error, cap_text, command_error_json};
use kiln_workspace::{DiskFs, RelPath, VfsError};
use serde_json::{Value, json};

const USAGE: &str = "usage:
  kiln-cli commands [--schema | --audit]
  kiln-cli exec <name> <json-params> [--text <initial-text>]
               [--root <dir> | --memory | --memory-seed <manifest>]
  kiln-cli run <script> [--root <dir> | --memory | --memory-seed <manifest>] [--log <file>]
  kiln-cli control [--root <dir>]
  kiln-cli --help | -h | <subcommand> --help

  run      run a script of JSON-lines requests, one response line per request
  control  read JSON-lines requests from stdin, answer each on stdout
  exec     run ONE command in a fresh session: it can read files, but an edit made
           with it is never saved; to edit and save files use `run` or `control`
  commands list the commands (--schema: params/result schemas and examples)

  --root <dir>  root of the files the commands act on (default: current directory)
  --memory      an empty in-memory file system instead (for tests)
  --memory-seed <manifest>
                an in-memory file system holding exactly the files of a JSON manifest
                (`files`: entries of `path` and one of `text` or `hex`);
                see docs/protocol/cli-memory-seed.md; nothing is written to disk
  --log <file>  after the script, write the successful requests as a replayable script

The protocol is described in docs/control-protocol.md.";

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
    /// A usage error: what was wrong, then the usage text (D-031 (12)).
    fn usage(reason: impl std::fmt::Display) -> Self {
        Self::err(2, format!("kiln-cli: {reason}\n{USAGE}\n"))
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
    /// An in-memory file system seeded from the manifest at this host path.
    MemorySeed(PathBuf),
}

/// Options after the positional arguments of a subcommand.
#[derive(Debug, Default)]
struct Opts {
    root: Option<String>,
    memory: bool,
    memory_seed: Option<String>,
    log: Option<String>,
    text: Option<String>,
}

impl Opts {
    fn fs(&self) -> Fs {
        if self.memory {
            Fs::Memory
        } else if let Some(manifest) = &self.memory_seed {
            Fs::MemorySeed(PathBuf::from(manifest))
        } else {
            Fs::Disk(PathBuf::from(self.root.as_deref().unwrap_or(".")))
        }
    }
}

/// Split `args` (of subcommand `sub`) into positional arguments and options: `--memory`, or
/// `--flag value` for the other flags; `allowed` names the flags `sub` takes, options may come
/// before, between or after positionals. Each flag at most once; `--root`, `--memory` and
/// `--memory-seed` exclude each other. On a usage error, what was wrong.
fn parse_opts<'a>(
    sub: &str,
    args: &[&'a str],
    allowed: &[&str],
) -> Result<(Vec<&'a str>, Opts), String> {
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
            return Err(format!("unknown option `{arg}` for `{sub}`"));
        }
        if *arg == "--memory" {
            if opts.memory {
                return Err("option `--memory` given twice".to_owned());
            }
            opts.memory = true;
            continue;
        }
        let [value, tail @ ..] = rest else {
            return Err(format!("option `{arg}` needs a value"));
        };
        rest = tail;
        let slot = match *arg {
            "--root" => &mut opts.root,
            "--memory-seed" => &mut opts.memory_seed,
            "--log" => &mut opts.log,
            "--text" => &mut opts.text,
            _ => return Err(format!("unknown option `{arg}` for `{sub}`")),
        };
        if slot.replace((*value).to_owned()).is_some() {
            return Err(format!("option `{arg}` given twice"));
        }
    }
    if opts.memory && opts.root.is_some() {
        return Err("options `--root` and `--memory` cannot be combined".to_owned());
    }
    if opts.memory_seed.is_some() {
        if opts.root.is_some() {
            return Err("options `--root` and `--memory-seed` cannot be combined".to_owned());
        }
        if opts.memory {
            return Err("options `--memory` and `--memory-seed` cannot be combined".to_owned());
        }
    }
    Ok((positional, opts))
}

/// Check that `pos` holds exactly the positional arguments `names` (for messages).
fn positionals<'a>(pos: &[&'a str], names: &[&str]) -> Result<Vec<&'a str>, String> {
    if let Some(missing) = names.get(pos.len()) {
        return Err(format!("missing argument {missing}"));
    }
    if let Some(extra) = pos.get(names.len()) {
        return Err(format!("unexpected argument `{extra}`"));
    }
    Ok(pos.to_vec())
}

fn bad_root(e: impl std::fmt::Display) -> Outcome {
    Outcome::err(2, format!("kiln-cli: bad --root: {e}\n"))
}

fn bad_manifest(manifest: &std::path::Path, reason: impl std::fmt::Display) -> Outcome {
    Outcome::err(
        2,
        format!(
            "kiln-cli: bad --memory-seed {}: {reason}\n",
            manifest.display()
        ),
    )
}

/// The files of a `--memory-seed` manifest (`docs/protocol/cli-memory-seed.md`), read once
/// from the host file `manifest`. Checks in this order (T-011 Pinned (a)): readable, JSON, keys
/// (top level and every entry), then per entry in array order the `path`, `text`/`hex` and
/// duplicates, and last prefix conflicts. On a refusal, the reason.
fn read_manifest(manifest: &std::path::Path) -> Result<Vec<(RelPath, Vec<u8>)>, String> {
    let bytes = std::fs::read(manifest).map_err(|e| format!("cannot read: {e}"))?;
    parse_manifest(&bytes)
}

/// [`read_manifest`] after the read.
fn parse_manifest(bytes: &[u8]) -> Result<Vec<(RelPath, Vec<u8>)>, String> {
    // serde_json bounds nesting depth itself (an error, not a stack overflow).
    let value: Value = serde_json::from_slice(bytes).map_err(|e| format!("invalid JSON: {e}"))?;
    // serde_json keeps the last of repeated keys; a repeat makes the manifest malformed (u2).
    repeated_key(bytes)?;
    let Value::Object(top) = value else {
        return Err("not a JSON object".to_owned());
    };
    if let Some(key) = top.keys().find(|k| k.as_str() != "files") {
        return Err(format!("unknown key `{key}` (only `files`)"));
    }
    let Some(files) = top.get("files") else {
        return Err("missing key `files`".to_owned());
    };
    let Value::Array(entries) = files else {
        return Err("`files` is not an array".to_owned());
    };
    let mut objects = Vec::with_capacity(entries.len());
    for (i, entry) in entries.iter().enumerate() {
        let Value::Object(entry) = entry else {
            return Err(format!("files[{i}] is not an object"));
        };
        if let Some(key) = entry
            .keys()
            .find(|k| !["path", "text", "hex"].contains(&k.as_str()))
        {
            return Err(format!(
                "files[{i}]: unknown key `{key}` (only `path`, `text`, `hex`)"
            ));
        }
        objects.push(entry);
    }
    let mut out = Vec::with_capacity(objects.len());
    let mut seen = std::collections::BTreeSet::new();
    for (i, entry) in objects.into_iter().enumerate() {
        let path = match entry.get("path") {
            None => return Err(format!("files[{i}]: missing `path`")),
            Some(Value::String(p)) => RelPath::new(p).map_err(|e| format!("files[{i}]: {e}"))?,
            Some(_) => return Err(format!("files[{i}]: `path` is not a string")),
        };
        let content = match (entry.get("text"), entry.get("hex")) {
            (Some(Value::String(text)), None) => text.as_bytes().to_vec(),
            (None, Some(Value::String(hex))) => {
                decode_hex(hex).map_err(|e| format!("files[{i}]: `hex` {e}"))?
            }
            (Some(_), None) => return Err(format!("files[{i}]: `text` is not a string")),
            (None, Some(_)) => return Err(format!("files[{i}]: `hex` is not a string")),
            _ => return Err(format!("files[{i}]: needs exactly one of `text` and `hex`")),
        };
        if !seen.insert(path.as_str().to_owned()) {
            return Err(format!("files[{i}]: path `{path}` given twice"));
        }
        out.push((path, content));
    }
    // A file cannot also be a directory: no path may be a strict prefix directory of another.
    // In sorted order, the first path at or after "p/" starts with "p/" iff any path does.
    for p in &seen {
        let dir = format!("{p}/");
        if let Some(below) = seen.range(dir.clone()..).next()
            && below.starts_with(&dir)
        {
            return Err(format!("path `{p}` is a file and a directory (`{below}`)"));
        }
    }
    Ok(out)
}

/// Refuse `json` (already parsed as valid JSON, so well-formed and at most serde_json's
/// nesting depth) if any object in it repeats a key; keys are compared after unescaping.
fn repeated_key(json: &[u8]) -> Result<(), String> {
    /// One open container: an object (its keys so far, whether a key comes next) or an array.
    enum Open {
        Object(std::collections::BTreeSet<String>, bool),
        Array,
    }
    let mut stack: Vec<Open> = Vec::new();
    let mut i = 0;
    while i < json.len() {
        match json[i] {
            b'{' => stack.push(Open::Object(std::collections::BTreeSet::new(), true)),
            b'[' => stack.push(Open::Array),
            b'}' | b']' => {
                stack.pop();
            }
            b',' => {
                if let Some(Open::Object(_, key_next)) = stack.last_mut() {
                    *key_next = true;
                }
            }
            b'"' => {
                let start = i;
                i += 1;
                while json[i] != b'"' {
                    // Skip the escaped character (`\"` must not end the string).
                    i += if json[i] == b'\\' { 2 } else { 1 };
                }
                if let Some(Open::Object(keys, key_next @ true)) = stack.last_mut() {
                    *key_next = false;
                    let key: String = serde_json::from_slice(&json[start..=i])
                        .map_err(|e| format!("invalid JSON: {e}"))?;
                    if !keys.insert(key.clone()) {
                        return Err(format!("key `{key}` repeated in one object"));
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    Ok(())
}

/// Bytes from an even number of hex digits (either case); on failure, what was wrong.
fn decode_hex(hex: &str) -> Result<Vec<u8>, String> {
    if let Some((i, c)) = hex.char_indices().find(|(_, c)| !c.is_ascii_hexdigit()) {
        return Err(format!("has a non-hex character {c:?} at byte {i}"));
    }
    let digits = hex.as_bytes();
    if digits.len() % 2 == 1 {
        return Err("has an odd number of digits".to_owned());
    }
    // Every byte is an ASCII hex digit now, so `to_digit` cannot fail.
    let digit = |b: u8| char::from(b).to_digit(16).map_or(0, |d| d as u8);
    Ok(digits
        .chunks_exact(2)
        .map(|pair| digit(pair[0]) << 4 | digit(pair[1]))
        .collect())
}

/// The seeded files as [`Host::in_memory`] / [`Context::with_files`] take them.
fn as_file_refs(files: &[(RelPath, Vec<u8>)]) -> Vec<(&str, &[u8])> {
    files
        .iter()
        .map(|(p, b)| (p.as_str(), b.as_slice()))
        .collect()
}

/// "root `<root>`: <reason>", as `kiln_control::Host::on_disk` reports it.
fn root_error(root: &std::path::Path, e: &VfsError) -> String {
    let reason = match e {
        VfsError::NotFound { .. } => "not found".to_owned(),
        VfsError::Io {
            kind: std::io::ErrorKind::NotADirectory,
            ..
        } => "not a directory".to_owned(),
        other => {
            let text = other.to_string();
            text.strip_prefix("(root): ").unwrap_or(&text).to_owned()
        }
    };
    format!("root `{}`: {reason}", root.display())
}

/// Help is asked for: `--help`/`-h` as the first argument, or in an option position of a
/// subcommand (not as the value of `--root`, `--memory-seed`, `--log` or `--text`).
fn wants_help(args: &[String]) -> bool {
    let is_help = |a: &str| a == "--help" || a == "-h";
    let Some(first) = args.first() else {
        return false;
    };
    if is_help(first) {
        return true;
    }
    if !["commands", "exec", "run", "control"].contains(&first.as_str()) {
        return false;
    }
    let mut takes_value = false;
    for a in &args[1..] {
        if takes_value {
            takes_value = false;
        } else if is_help(a) {
            return true;
        } else {
            takes_value = matches!(a.as_str(), "--root" | "--memory-seed" | "--log" | "--text");
        }
    }
    false
}

fn exec(registry: &Registry, name: &str, params: &str, opts: &Opts) -> Outcome {
    let params: Value = match serde_json::from_str(params) {
        Ok(v) => v,
        Err(e) => {
            let error = json!({"code": "invalid_json", "kind": "syntax",
                "message": format!("params are not JSON: {e}")});
            return Outcome::err(2, format!("{error}\n"));
        }
    };
    let text = opts.text.as_deref().unwrap_or("");
    let mut ctx = match opts.fs() {
        Fs::Memory => Context::with_text(text),
        fs @ (Fs::Disk(_) | Fs::MemorySeed(_)) => {
            let mut ctx = match fs {
                Fs::Disk(root) => match DiskFs::new(&root) {
                    Ok(fs) => Context::with_vfs(Box::new(fs)),
                    Err(e) => return bad_root(root_error(&root, &e)),
                },
                Fs::MemorySeed(manifest) => {
                    let files = match read_manifest(&manifest) {
                        Ok(files) => files,
                        Err(reason) => return bad_manifest(&manifest, reason),
                    };
                    match Context::with_files(&as_file_refs(&files)) {
                        Ok(ctx) => ctx,
                        Err(e) => return Outcome::err(1, format!("kiln-cli: {e}\n")),
                    }
                }
                Fs::Memory => unreachable!("handled above"),
            };
            // The initial scratch text enters through the registry like any other change.
            if !text.is_empty()
                && let Err(e) = registry.execute(
                    &mut ctx,
                    "buffer.insert",
                    json!({"offset": 0, "text": text}),
                )
            {
                let error = command_error_json("buffer.insert", &json!({}), &e);
                return Outcome::err(1, format!("{error}\n"));
            }
            ctx
        }
    };
    match registry.execute(&mut ctx, name, params.clone()) {
        Ok(result) => Outcome::ok(format!("{result}\n")),
        // The same error object (with `kind`) as a protocol response, without `id`.
        Err(e) => Outcome::err(2, format!("{}\n", command_error_json(name, &params, &e))),
    }
}

fn host_for(fs: &Fs) -> Result<Host, Outcome> {
    match fs {
        Fs::Memory => Host::in_memory(&[]).map_err(|e| Outcome::err(1, format!("kiln-cli: {e}\n"))),
        Fs::Disk(root) => Host::on_disk(root).map_err(bad_root),
        Fs::MemorySeed(manifest) => {
            let files = read_manifest(manifest).map_err(|reason| bad_manifest(manifest, reason))?;
            Host::in_memory(&as_file_refs(&files))
                .map_err(|e| Outcome::err(1, format!("kiln-cli: {e}\n")))
        }
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
    if wants_help(args) {
        return Outcome::ok(format!("{USAGE}\n"));
    }
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
        ["commands", arg, ..] if arg.starts_with("--") => {
            Outcome::usage(format!("unknown option `{arg}` for `commands`"))
        }
        ["commands", arg, ..] => Outcome::usage(format!("unexpected argument `{arg}`")),
        ["exec", rest @ ..] => {
            let allowed = ["--text", "--root", "--memory", "--memory-seed"];
            let parsed = parse_opts("exec", rest, &allowed).and_then(|(pos, opts)| {
                Ok((positionals(&pos, &["<name>", "<json-params>"])?, opts))
            });
            match parsed {
                Ok((pos, opts)) => exec(&registry, pos[0], pos[1], &opts),
                Err(reason) => Outcome::usage(reason),
            }
        }
        ["run", rest @ ..] => {
            let parsed = parse_opts(
                "run",
                rest,
                &["--root", "--memory", "--memory-seed", "--log"],
            )
            .and_then(|(pos, opts)| Ok((positionals(&pos, &["<script>"])?, opts)));
            match parsed {
                Ok((pos, opts)) => run_script(pos[0], &opts),
                Err(reason) => Outcome::usage(reason),
            }
        }
        [] => Outcome::usage("missing subcommand"),
        [sub, ..] => Outcome::usage(format!("unknown subcommand `{sub}`")),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let outcome = if args.first().map(String::as_str) == Some("control") && !wants_help(&args) {
        control_main(&args[1..])
    } else {
        run(&args)
    };
    finish(outcome)
}

/// `kiln-cli control`: responses stream to stdout as they are produced (each already
/// capped by the host); the returned outcome carries only the exit code and stderr.
fn control_main(rest: &[String]) -> Outcome {
    let rest: Vec<&str> = rest.iter().map(String::as_str).collect();
    let opts = match parse_opts("control", &rest, &["--root"])
        .and_then(|(pos, opts)| positionals(&pos, &[]).map(|_| opts))
    {
        Ok(opts) => opts,
        Err(reason) => return Outcome::usage(reason),
    };
    let mut host = match host_for(&opts.fs()) {
        Ok(h) => h,
        Err(o) => return o,
    };
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    match control(&mut host, &mut stdin.lock(), &mut stdout.lock()) {
        Ok(()) => Outcome::ok(String::new()),
        Err(e) => Outcome::err(1, format!("kiln-cli: control I/O error: {e}\n")),
    }
}

/// Everything the process writes to stderr passes through here (D-031 (9)): a line that is a
/// structured error object gets [`cap_error`] (strings ≤ 200 chars, line ≤ 4096 bytes), any
/// other line [`cap_text`] (≤ 200 chars). Protocol responses on stdout are capped by the host.
fn bound_stderr(stderr: &str) -> String {
    let mut out = String::with_capacity(stderr.len().min(4096));
    for line in stderr.split_inclusive('\n') {
        let (body, end) = match line.strip_suffix('\n') {
            Some(body) => (body, "\n"),
            None => (line, ""),
        };
        match serde_json::from_str::<Value>(body) {
            Ok(mut error @ Value::Object(_)) if error.get("code").is_some() => {
                cap_error(&mut error);
                out.push_str(&error.to_string());
            }
            _ => out.push_str(&cap_text(body)),
        }
        out.push_str(end);
    }
    out
}

fn finish(outcome: Outcome) -> ExitCode {
    let mut stdout = std::io::stdout().lock();
    let written = stdout
        .write_all(outcome.stdout.as_bytes())
        .and_then(|()| stdout.flush());
    let (code, stderr) = match written {
        Ok(()) => (outcome.code, outcome.stderr),
        // D-028 (3): output that did not reach stdout is a failure (exit 1), whatever the
        // responses were.
        Err(e) => (
            1,
            format!("{}kiln-cli: cannot write to stdout: {e}\n", outcome.stderr),
        ),
    };
    let _ = std::io::stderr().write_all(bound_stderr(&stderr).as_bytes());
    ExitCode::from(code)
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
        let (pos, o) = parse_opts(
            "run",
            &["--root", "d", "s", "--log", "l"],
            &["--root", "--log"],
        )
        .unwrap();
        assert_eq!(pos, ["s"]);
        assert_eq!(
            (o.root.as_deref(), o.log.as_deref()),
            (Some("d"), Some("l"))
        );
        assert_eq!(o.fs(), Fs::Disk(PathBuf::from("d")));
        let (_, o) = parse_opts("run", &[], &["--root"]).unwrap();
        assert_eq!(o.fs(), Fs::Disk(PathBuf::from(".")));
        let all = ["--root", "--memory", "--log"];
        let err = |args: &[&str]| parse_opts("run", args, &all).err().unwrap();
        assert_eq!(
            err(&["--root", "a", "--memory"]),
            "options `--root` and `--memory` cannot be combined"
        );
        assert_eq!(
            err(&["--root", "a", "--root", "b"]),
            "option `--root` given twice"
        );
        assert_eq!(
            err(&["--memory", "--memory"]),
            "option `--memory` given twice"
        );
        assert_eq!(err(&["--root"]), "option `--root` needs a value");
        assert_eq!(err(&["--text", "x"]), "unknown option `--text` for `run`");
        assert_eq!(err(&["--root=x"]), "unknown option `--root=x` for `run`");
        // T-011: `--memory-seed <manifest>` excludes `--root` and `--memory`.
        let all = ["--root", "--memory", "--memory-seed", "--log"];
        let seed_err = |args: &[&str]| parse_opts("run", args, &all).err().unwrap();
        for args in [
            &["--memory-seed", "m", "--root", "d"][..],
            &["--root", "d", "--memory-seed", "m"],
        ] {
            assert_eq!(
                seed_err(args),
                "options `--root` and `--memory-seed` cannot be combined"
            );
        }
        for args in [
            &["--memory-seed", "m", "--memory"][..],
            &["--memory", "--memory-seed", "m"],
        ] {
            assert_eq!(
                seed_err(args),
                "options `--memory` and `--memory-seed` cannot be combined"
            );
        }
        assert_eq!(
            seed_err(&["--memory-seed", "m", "--memory-seed", "n"]),
            "option `--memory-seed` given twice"
        );
        assert_eq!(
            seed_err(&["--memory-seed"]),
            "option `--memory-seed` needs a value"
        );
        let (_, o) = parse_opts("run", &["--memory-seed", "m.json", "--log", "l"], &all).unwrap();
        assert_eq!(o.fs(), Fs::MemorySeed(PathBuf::from("m.json")));
        let control_err = |args: &[&str]| parse_opts("control", args, &["--root"]).err().unwrap();
        assert_eq!(
            control_err(&["--memory-seed", "m"]),
            "unknown option `--memory-seed` for `control`"
        );
    }

    #[test]
    fn usage_errors_name_what_was_wrong() {
        for (args, reason) in [
            (&[][..], "missing subcommand"),
            (&["frobnicate"], "unknown subcommand `frobnicate`"),
            (&["run"], "missing argument <script>"),
            (&["run", "a", "b"], "unexpected argument `b`"),
            (
                &["run", "s", "--root=x"],
                "unknown option `--root=x` for `run`",
            ),
            (&["exec"], "missing argument <name>"),
            (&["exec", "doc.list"], "missing argument <json-params>"),
            (&["exec", "doc.list", "{}", "x"], "unexpected argument `x`"),
            (
                &["exec", "doc.list", "{}", "--log", "l"],
                "unknown option `--log` for `exec`",
            ),
            (
                &["commands", "--frob"],
                "unknown option `--frob` for `commands`",
            ),
            (&["commands", "x"], "unexpected argument `x`"),
        ] {
            let out = call(args);
            assert_eq!(out.code, 2, "{args:?}");
            assert!(out.stdout.is_empty());
            let first = out.stderr.lines().next().unwrap();
            assert_eq!(first, format!("kiln-cli: {reason}"), "{args:?}");
            assert!(out.stderr.contains("usage:"), "{args:?}");
        }
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
    fn help_is_recognised_only_in_option_positions() {
        let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(wants_help(&v(&["--help"])));
        assert!(wants_help(&v(&["-h", "run"])));
        assert!(wants_help(&v(&["run", "s", "--help"])));
        assert!(wants_help(&v(&["exec", "-h"])));
        assert!(!wants_help(&v(&["run", "s", "--root", "-h"])));
        assert!(!wants_help(&v(&[
            "exec",
            "buffer.text",
            "{}",
            "--text",
            "--help"
        ])));
        assert!(!wants_help(&v(&["frobnicate", "--help"])));
        assert!(!wants_help(&v(&[])));
        let out = call(&["exec", "--help"]);
        assert_eq!((out.code, out.stdout.starts_with("usage:")), (0, true));
        assert!(out.stdout.contains("never saved"));
        let out = call(&["exec", "buffer.text", "{}", "--text", "--help", "--memory"]);
        assert_eq!(out.code, 0);
        assert!(out.stdout.contains("--help"), "{out:?}");
    }

    #[test]
    fn bad_root_message_names_the_root() {
        let out = call(&["exec", "doc.list", "{}", "--root", "/no/such/kiln/dir"]);
        assert_eq!(out.code, 2);
        assert_eq!(
            out.stderr,
            "kiln-cli: bad --root: root `/no/such/kiln/dir`: not found\n"
        );
        let out = call(&["run", "s.jsonl", "--root", "/no/such/kiln/dir"]);
        assert_eq!(
            out.stderr,
            "kiln-cli: bad --root: root `/no/such/kiln/dir`: not found\n"
        );
    }

    /// Long values without and with white space, escaped characters and multi-byte ones.
    fn long_values() -> [String; 4] {
        [
            "x".repeat(5000),
            "x y ".repeat(1250),
            "\"x\" ".repeat(1000),
            "é😀 ".repeat(1000),
        ]
    }

    /// Every string inside `v`, at any depth.
    fn strings(v: &Value, out: &mut Vec<String>) {
        match v {
            Value::String(s) => out.push(s.clone()),
            Value::Array(a) => a.iter().for_each(|x| strings(x, out)),
            Value::Object(m) => m.values().for_each(|x| strings(x, out)),
            _ => {}
        }
    }

    /// The D-031 caps for one error line (a bare error object or a response).
    fn assert_capped(line: &str) {
        assert!(line.len() <= 4096, "{} bytes", line.len());
        let v: Value = serde_json::from_str(line).unwrap();
        let error = if v.get("error").is_some() {
            &v["error"]
        } else {
            &v
        };
        assert!(
            error["code"].is_string() && error["kind"].is_string(),
            "{line:.300}"
        );
        let mut all = Vec::new();
        strings(&error["message"], &mut all);
        strings(&error["details"], &mut all);
        for t in all {
            assert!(t.chars().count() <= 200, "{t:.300}");
        }
    }

    /// What `kiln-cli` would print on stderr for `args`.
    fn stderr_of(args: &[String]) -> String {
        bound_stderr(&run(args).stderr)
    }

    fn strs(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn exec_errors_are_capped_and_carry_kind() {
        let short_strings = Value::from(vec!["a b"; 2000]).to_string();
        for value in long_values() {
            let j = Value::String(value.clone()).to_string();
            let cases = [
                // message: an unknown command name
                vec![
                    "exec".to_owned(),
                    value.clone(),
                    "{}".into(),
                    "--memory".into(),
                ],
                // message: an invalid path
                vec![
                    "exec".into(),
                    "doc.open".into(),
                    format!(r#"{{"path":{}}}"#, Value::String(format!("../{value}"))),
                    "--memory".into(),
                ],
                // details: a string where an integer is expected
                vec![
                    "exec".into(),
                    "doc.insert".into(),
                    format!(r#"{{"doc":{j},"offset":0,"text":"x"}}"#),
                    "--memory".into(),
                ],
                // details: an array of it
                vec![
                    "exec".into(),
                    "doc.insert".into(),
                    format!(r#"{{"doc":[{j},{j}],"offset":0,"text":"x"}}"#),
                    "--memory".into(),
                ],
                // R2-D1: an array of short strings to a command without params
                vec![
                    "exec".into(),
                    "doc.list".into(),
                    format!(r#"{{"x":{short_strings}}}"#),
                    "--memory".into(),
                ],
                vec![
                    "exec".into(),
                    "buffer.text".into(),
                    format!(r#"{{"x":{{"y":{short_strings}}}}}"#),
                    "--memory".into(),
                ],
                // params that are not JSON
                vec![
                    "exec".into(),
                    "doc.list".into(),
                    format!("{{{value}"),
                    "--memory".into(),
                ],
            ];
            for args in cases {
                let err = stderr_of(&args);
                assert!(
                    err.ends_with('\n') && err.matches('\n').count() == 1,
                    "{err:.300}"
                );
                assert_capped(err.trim_end());
            }
        }
        // A short error is printed in full, now with its kind.
        assert_eq!(
            stderr_of(&strs(&["exec", "no.such", "{}"])),
            "{\"code\":\"unknown_command\",\"kind\":\"unknown_command\",\"message\":\"unknown command `no.such`\"}\n"
        );
        let err = stderr_of(&strs(&[
            "exec",
            "doc.open",
            r#"{"path":"nope"}"#,
            "--memory",
        ]));
        let v: Value = serde_json::from_str(&err).unwrap();
        assert_eq!(
            (v["code"].as_str(), v["kind"].as_str()),
            (Some("rejected"), Some("not_found"))
        );
    }

    #[test]
    fn run_and_control_responses_are_capped() {
        let dir = std::env::temp_dir().join(format!("kiln-cli-unit-cap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let short_strings = Value::from(vec!["a b"; 2000]).to_string();
        for (i, value) in long_values().iter().enumerate() {
            let j = Value::String(value.clone()).to_string();
            let script = [
                format!(r#"{{"command":{j}}}"#),
                format!(r#"{{"command":"doc.open","params":{{"path":{j}}}}}"#),
                format!(
                    r#"{{"command":"doc.insert","params":{{"doc":{j},"offset":0,"text":"x"}}}}"#
                ),
                format!(r#"{{"command":"doc.list",{j}:1}}"#),
                format!(r#"{{"command":"doc.list","params":{{"x":{short_strings}}}}}"#),
            ]
            .join("\n");
            let path = dir.join(format!("s{i}.jsonl"));
            std::fs::write(&path, &script).unwrap();
            let out = call(&["run", path.to_str().unwrap(), "--memory"]);
            assert_eq!(out.code, 3);
            let mut host = Host::in_memory(&[]).unwrap();
            let mut controlled = Vec::new();
            control(&mut host, &mut script.as_bytes(), &mut controlled).unwrap();
            assert_eq!(
                String::from_utf8(controlled).unwrap(),
                out.stdout,
                "run = control"
            );
            assert_eq!(out.stdout.lines().count(), 5);
            for line in out.stdout.lines() {
                assert_capped(line);
            }
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn cli_text_errors_are_capped() {
        for value in long_values() {
            for args in [
                vec![
                    "run".to_owned(),
                    format!("/no/such/{value}.jsonl"),
                    "--memory".into(),
                ],
                vec![
                    "run".into(),
                    "s.jsonl".into(),
                    "--root".into(),
                    format!("/no/{value}"),
                ],
                vec![
                    "exec".into(),
                    "doc.list".into(),
                    "{}".into(),
                    "--root".into(),
                    format!("/no/{value}"),
                ],
                vec!["run".into(), "s.jsonl".into(), format!("--{value}")],
                vec![value.clone()],
            ] {
                let err = stderr_of(&args);
                assert!(err.starts_with("kiln-cli: "), "{err:.300}");
                for line in err.lines() {
                    assert!(line.chars().count() <= 200, "{line:.300}");
                }
                assert!(err.lines().next().unwrap().ends_with('…'), "{err:.300}");
            }
        }
        // Short arguments are named in full.
        assert_eq!(
            stderr_of(&strs(&["run", "s.jsonl", "--root", "/no/such/kiln"])),
            "kiln-cli: bad --root: root `/no/such/kiln`: not found\n"
        );
    }

    fn manifest_err(json: &str) -> String {
        parse_manifest(json.as_bytes()).unwrap_err()
    }

    #[test]
    fn manifest_parses_text_and_hex_in_order() {
        let files = parse_manifest(
            br#"{"files":[{"path":"src/main.rs","text":"fn main() {}\r\n"},
                {"path":"bom.txt","hex":"efBBbf6869"},{"path":"e","hex":""},{"path":"dx/y","text":""},
                {"path":"d","text":"\u00e9"}]}"#,
        )
        .unwrap();
        let got: Vec<(&str, &[u8])> = as_file_refs(&files);
        assert_eq!(
            got,
            [
                ("src/main.rs", &b"fn main() {}\r\n"[..]),
                ("bom.txt", b"\xef\xbb\xbfhi"),
                ("e", b""),
                // `d` is a string prefix of `dx/y`, not a directory prefix: no conflict (u1).
                ("dx/y", b""),
                ("d", "é".as_bytes()),
            ]
        );
        assert!(parse_manifest(br#"{"files":[]}"#).unwrap().is_empty());
    }

    #[test]
    fn manifest_refusals_name_the_problem() {
        for (json, want) in [
            ("", "invalid JSON"),
            ("[]", "not a JSON object"),
            ("{}", "missing key `files`"),
            (r#"{"files":[],"x":1}"#, "unknown key `x`"),
            (r#"{"files":{}}"#, "`files` is not an array"),
            (r#"{"files":[1]}"#, "files[0] is not an object"),
            (
                r#"{"files":[{"path":"a","text":"","mode":1}]}"#,
                "files[0]: unknown key `mode`",
            ),
            (r#"{"files":[{"text":""}]}"#, "files[0]: missing `path`"),
            (
                r#"{"files":[{"path":1,"text":""}]}"#,
                "files[0]: `path` is not a string",
            ),
            (
                r#"{"files":[{"path":"a/../b","text":""}]}"#,
                "files[0]: invalid relative path",
            ),
            (
                r#"{"files":[{"path":"a"}]}"#,
                "exactly one of `text` and `hex`",
            ),
            (
                r#"{"files":[{"path":"a","text":"","hex":""}]}"#,
                "exactly one of `text` and `hex`",
            ),
            (
                r#"{"files":[{"path":"a","text":1}]}"#,
                "`text` is not a string",
            ),
            (
                r#"{"files":[{"path":"a","hex":1}]}"#,
                "`hex` is not a string",
            ),
            (r#"{"files":[{"path":"a","hex":"abc"}]}"#, "odd number"),
            (r#"{"files":[{"path":"a","hex":"0g"}]}"#, "non-hex"),
            (r#"{"files":[{"path":"a","hex":"é1"}]}"#, "non-hex"),
            (
                r#"{"files":[{"path":"a","text":""},{"path":"a","hex":""}]}"#,
                "files[1]: path `a` given twice",
            ),
            (
                r#"{"files":[{"path":"d/e/f","text":""},{"path":"d","text":""}]}"#,
                "path `d` is a file and a directory (`d/e/f`)",
            ),
        ] {
            let got = manifest_err(json);
            assert!(got.contains(want), "{json}: {got}");
        }
        for json in [
            r#"{"files":[{"path":"a","text":""}],"files":[]}"#,
            r#"{"files":[{"path":"a","text":"x","text":"y"}]}"#,
            r#"{"files":[{"path":"a","p\u0061th":"b","text":""}]}"#,
            r#"{"files":[{"path":"a","text":""},{"hex":"","path":"b","hex":""}]}"#,
        ] {
            let got = manifest_err(json);
            assert!(got.contains("repeated in one object"), "{json}: {got}");
        }
        // The same key in different objects, and key-like strings in values, are fine.
        assert_eq!(
            parse_manifest(
                br#"{"files":[{"path":"a","text":"\"path\":"},{"path":"b,{\"","hex":""}]}"#
            )
            .unwrap()
            .len(),
            2
        );
        // Keys are checked for every entry before any entry's path (Pinned (a)).
        let got = manifest_err(r#"{"files":[{"path":"/x","text":""},{"path":"a","oops":1}]}"#);
        assert!(got.starts_with("files[1]: unknown key"), "{got}");
        // Prefix conflicts come after every entry is checked.
        let got = manifest_err(
            r#"{"files":[{"path":"d","text":""},{"path":"d/x","text":""},{"path":"q","hex":"1"}]}"#,
        );
        assert!(got.starts_with("files[2]: `hex`"), "{got}");
        // Deep nesting is an error, not a stack overflow.
        let deep = format!("{}{}", "[".repeat(200_000), "]".repeat(200_000));
        assert!(manifest_err(&deep).starts_with("invalid JSON"));
    }

    #[test]
    fn decode_hex_cases() {
        assert_eq!(decode_hex("00ffFFa0").unwrap(), [0, 255, 255, 160]);
        assert_eq!(decode_hex("").unwrap(), Vec::<u8>::new());
        assert!(decode_hex("0").is_err() && decode_hex("+1").is_err() && decode_hex(" 1").is_err());
    }

    #[test]
    fn memory_seed_through_run_and_exec() {
        let dir = std::env::temp_dir().join(format!("kiln-cli-unit-seed-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let m = dir.join("m.json");
        std::fs::write(&m, r#"{"files":[{"path":"a.txt","text":"hi\n"}]}"#).unwrap();
        let m = m.to_str().unwrap();
        let out = call(&[
            "exec",
            "doc.open",
            r#"{"path":"a.txt"}"#,
            "--memory-seed",
            m,
        ]);
        assert_eq!(out.code, 0, "{out:?}");
        let out = call(&[
            "exec",
            "buffer.text",
            "{}",
            "--memory-seed",
            m,
            "--text",
            "abc",
        ]);
        assert_eq!(
            out,
            Outcome::ok("{\"len_chars\":3,\"text\":\"abc\"}\n".into())
        );
        let missing = dir.join("missing.json");
        let missing = missing.to_str().unwrap();
        for args in [
            vec!["exec", "doc.list", "{}", "--memory-seed", missing],
            vec!["run", "/no/such/script", "--memory-seed", missing],
        ] {
            let out = call(&args);
            assert_eq!((out.code, out.stdout.as_str()), (2, ""), "{args:?}");
            let prefix = format!("kiln-cli: bad --memory-seed {missing}: cannot read: ");
            assert!(out.stderr.starts_with(&prefix), "{out:?}");
            assert!(!out.stderr.contains("usage:"));
        }
        // A manifest path that looks like `--help` is a value, not a help request.
        assert!(!wants_help(&strs(&["run", "s", "--memory-seed", "--help"])));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn usage_on_unknown_subcommand() {
        assert_eq!(call(&["frobnicate"]).code, 2);
    }
}
