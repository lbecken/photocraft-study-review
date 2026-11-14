//! T-055 acceptance tests: `kiln script <script> [--root <dir> | --memory] [--log <file>]
//! [--size <w>x<h>]` runs the Kiln UI without a window from a script of control requests and UI
//! requests (`tasks/T-055-ui-script-observation.md`, Interface, Pinned, AC-1 to AC-10).
//!
//! Every test drives the built `kiln` binary (`CARGO_BIN_EXE_kiln`) with temporary roots under
//! `CARGO_TARGET_TMPDIR`; every spawned process is killed after [`TIMEOUT`] (D-028 (8)). Nodes are
//! found by role, label or value, never by a whole snapshot, and checked against an independent
//! source: control requests in the same session, `kiln_control::Host` on a copy of the root
//! (the one Rust API the spec allows, as oracle), the session log replayed, or the files on disk.
//! No test pins a message text, the usage text, the set of roles or the command list (golden rule
//! 9; Pinned (f)). Responses are read with the small JSON reader below, so that the tests need no
//! dev-dependency beyond the `kiln-control` the spec names.
#![cfg(test)]

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use kiln_control::Host;

/// Longest any spawned process may run (a debug build of the toolkit, on a loaded machine).
const TIMEOUT: Duration = Duration::from_secs(180);

/// `src/a.rs` of Pinned (g).
const A_RS: &str = "fn main() {\n    let x = 1;\n}\n";

// ---------------------------------------------------------------------------------------------
// A small JSON reader (the responses are compact JSON written by serde_json).
// ---------------------------------------------------------------------------------------------

/// A parsed JSON value; numbers are kept as their text.
#[derive(Clone, Debug, PartialEq)]
enum J {
    Null,
    Bool(bool),
    Num(String),
    Str(String),
    Arr(Vec<J>),
    Obj(BTreeMap<String, J>),
}

impl J {
    fn get(&self, key: &str) -> Option<&J> {
        match self {
            J::Obj(m) => m.get(key),
            _ => None,
        }
    }

    /// `self[key]`, which must exist.
    fn at(&self, key: &str) -> &J {
        self.get(key)
            .unwrap_or_else(|| panic!("no key `{key}` in {}", self.show()))
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            J::Str(s) => Some(s),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            J::Bool(b) => Some(*b),
            _ => None,
        }
    }

    fn as_u64(&self) -> Option<u64> {
        match self {
            J::Num(n) => n.parse().ok(),
            _ => None,
        }
    }

    fn as_arr(&self) -> Option<&[J]> {
        match self {
            J::Arr(a) => Some(a),
            _ => None,
        }
    }

    /// A short rendering for assertion messages.
    fn show(&self) -> String {
        let s = format!("{self:?}");
        if s.chars().count() > 1500 {
            let cut: String = s.chars().take(1500).collect();
            format!("{cut}…")
        } else {
            s
        }
    }
}

fn parse_json(text: &str) -> J {
    let mut p = JsonParser { s: text, i: 0 };
    p.ws();
    let v = p.value();
    p.ws();
    assert_eq!(p.i, text.len(), "trailing text after JSON value: {text}");
    v
}

struct JsonParser<'a> {
    s: &'a str,
    i: usize,
}

impl JsonParser<'_> {
    fn peek(&self) -> char {
        self.s[self.i..]
            .chars()
            .next()
            .unwrap_or_else(|| panic!("unexpected end of JSON: {}", self.s))
    }

    fn bump(&mut self) -> char {
        let c = self.peek();
        self.i += c.len_utf8();
        c
    }

    fn ws(&mut self) {
        while self.i < self.s.len() && matches!(self.peek(), ' ' | '\t' | '\n' | '\r') {
            self.i += 1;
        }
    }

    fn expect(&mut self, c: char) {
        let got = self.bump();
        assert_eq!(
            got, c,
            "JSON: expected `{c}` at byte {} of {}",
            self.i, self.s
        );
    }

    fn word(&mut self, w: &str) {
        assert!(
            self.s[self.i..].starts_with(w),
            "JSON: expected `{w}` at byte {} of {}",
            self.i,
            self.s
        );
        self.i += w.len();
    }

    fn value(&mut self) -> J {
        match self.peek() {
            '{' => {
                self.bump();
                let mut m = BTreeMap::new();
                self.ws();
                if self.peek() == '}' {
                    self.bump();
                    return J::Obj(m);
                }
                loop {
                    self.ws();
                    let k = self.string();
                    self.ws();
                    self.expect(':');
                    self.ws();
                    let v = self.value();
                    assert!(m.insert(k.clone(), v).is_none(), "duplicate key `{k}`");
                    self.ws();
                    match self.bump() {
                        ',' => {}
                        '}' => return J::Obj(m),
                        c => panic!("JSON: unexpected `{c}` in object: {}", self.s),
                    }
                }
            }
            '[' => {
                self.bump();
                let mut a = Vec::new();
                self.ws();
                if self.peek() == ']' {
                    self.bump();
                    return J::Arr(a);
                }
                loop {
                    self.ws();
                    a.push(self.value());
                    self.ws();
                    match self.bump() {
                        ',' => {}
                        ']' => return J::Arr(a),
                        c => panic!("JSON: unexpected `{c}` in array: {}", self.s),
                    }
                }
            }
            '"' => J::Str(self.string()),
            't' => {
                self.word("true");
                J::Bool(true)
            }
            'f' => {
                self.word("false");
                J::Bool(false)
            }
            'n' => {
                self.word("null");
                J::Null
            }
            _ => {
                let start = self.i;
                while self.i < self.s.len()
                    && matches!(self.peek(), '-' | '+' | '.' | 'e' | 'E' | '0'..='9')
                {
                    self.i += 1;
                }
                assert!(
                    self.i > start,
                    "JSON: bad value at byte {start} of {}",
                    self.s
                );
                J::Num(self.s[start..self.i].to_owned())
            }
        }
    }

    fn hex4(&mut self) -> u32 {
        let h = &self.s[self.i..self.i + 4];
        self.i += 4;
        u32::from_str_radix(h, 16).unwrap_or_else(|_| panic!("JSON: bad \\u escape `{h}`"))
    }

    fn string(&mut self) -> String {
        self.expect('"');
        let mut out = String::new();
        loop {
            match self.bump() {
                '"' => return out,
                '\\' => match self.bump() {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    '/' => out.push('/'),
                    'b' => out.push('\u{8}'),
                    'f' => out.push('\u{c}'),
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    'u' => {
                        let hi = self.hex4();
                        let cp = if (0xD800..0xDC00).contains(&hi) {
                            self.word("\\u");
                            let lo = self.hex4();
                            assert!((0xDC00..0xE000).contains(&lo), "JSON: bad low surrogate");
                            0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                        } else {
                            hi
                        };
                        out.push(char::from_u32(cp).expect("JSON: escape is a char"));
                    }
                    c => panic!("JSON: bad escape `\\{c}`"),
                },
                c => out.push(c),
            }
        }
    }
}

/// `s` as a JSON string literal.
fn q(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

// ---------------------------------------------------------------------------------------------
// Processes, directories, fixtures.
// ---------------------------------------------------------------------------------------------

/// What a finished process left.
struct Out {
    code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl Out {
    fn stderr_text(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }
}

fn wait_with_timeout(child: &mut std::process::Child, what: &str) -> ExitStatus {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().expect("try_wait") {
            return status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{what} did not finish within {TIMEOUT:?}");
        }
        thread::sleep(Duration::from_millis(5));
    }
}

/// Run `cmd` to completion under the hang detector, stdin closed, stdout and stderr captured
/// (stdout into `/dev/full` instead when `stdout_full`).
fn run_cmd(mut cmd: Command, what: &str, stdout_full: bool) -> Out {
    cmd.stdin(Stdio::null()).stderr(Stdio::piped());
    if stdout_full {
        let full = fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .expect("open /dev/full (precondition of this test)");
        cmd.stdout(Stdio::from(full));
    } else {
        cmd.stdout(Stdio::piped());
    }
    let mut child = cmd.spawn().unwrap_or_else(|e| panic!("spawn {what}: {e}"));
    let out_reader = child.stdout.take().map(|mut o| {
        thread::spawn(move || {
            let mut b = Vec::new();
            let _ = o.read_to_end(&mut b);
            b
        })
    });
    let mut err = child.stderr.take().expect("stderr pipe");
    let err_reader = thread::spawn(move || {
        let mut b = Vec::new();
        let _ = err.read_to_end(&mut b);
        b
    });
    let status = wait_with_timeout(&mut child, what);
    let stdout = out_reader
        .map(|r| r.join().expect("stdout reader"))
        .unwrap_or_default();
    let stderr = err_reader.join().expect("stderr reader");
    let code = status
        .code()
        .unwrap_or_else(|| panic!("{what} ended by a signal ({status})"));
    Out {
        code,
        stdout,
        stderr,
    }
}

/// `kiln <args>` run in `cwd`.
fn kiln(cwd: &Path, args: &[&str]) -> Out {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kiln"));
    cmd.args(args).current_dir(cwd);
    run_cmd(cmd, &format!("kiln {}", args.join(" ")), false)
}

/// A fresh, empty directory for one test case.
fn case_dir(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("t055-acceptance")
        .join(name);
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).expect("create case dir");
    d
}

fn write(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    fs::write(path, bytes).expect("write fixture file");
}

/// The small project the tests use: `<dir>/proj` with `src/a.rs` (Pinned (g)) and a few more
/// files (CRLF, non-ASCII). Returns the root.
fn fixture(dir: &Path) -> PathBuf {
    let root = dir.join("proj");
    write(&root.join("src/a.rs"), A_RS.as_bytes());
    write(
        &root.join("src/lib.rs"),
        b"pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
    );
    write(&root.join("b.txt"), b"hello\n");
    write(&root.join("notes/c.md"), b"# Notes\r\nline two\r\n");
    write(
        &root.join("u.txt"),
        "h\u{e9}llo w\u{f6}rld \u{1F600}\n".as_bytes(),
    );
    root
}

/// Every entry under `root`, by `/`-separated relative path: file bytes, `<dir>` or the target
/// of a symlink. Symlinks are not followed.
fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        let mut entries: Vec<_> = fs::read_dir(dir)
            .expect("read_dir")
            .map(|e| e.expect("dir entry").path())
            .collect();
        entries.sort();
        for p in entries {
            let rel = p
                .strip_prefix(base)
                .expect("under base")
                .to_string_lossy()
                .replace('\\', "/");
            let meta = fs::symlink_metadata(&p).expect("symlink_metadata");
            if meta.file_type().is_symlink() {
                let target = fs::read_link(&p).expect("read_link");
                out.insert(rel, format!("-> {}", target.display()).into_bytes());
            } else if meta.is_dir() {
                out.insert(rel, b"<dir>".to_vec());
                walk(base, &p, out);
            } else {
                out.insert(rel, fs::read(&p).expect("read file"));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

/// Copy `from` to `to` (files and directories; symlinks recreated as they are).
fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create copy dir");
    for e in fs::read_dir(from).expect("read_dir") {
        let e = e.expect("dir entry");
        let src = e.path();
        let dst = to.join(e.file_name());
        let meta = fs::symlink_metadata(&src).expect("symlink_metadata");
        if meta.file_type().is_symlink() {
            std::os::unix::fs::symlink(fs::read_link(&src).expect("read_link"), &dst)
                .expect("symlink");
        } else if meta.is_dir() {
            copy_dir(&src, &dst);
        } else {
            fs::copy(&src, &dst).expect("copy file");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Sessions.
// ---------------------------------------------------------------------------------------------

/// One `kiln script` run.
struct Session {
    out: Out,
    /// Stdout split into lines (each without its `\n`).
    lines: Vec<String>,
    /// The `--log` file, or empty when it was not written.
    log: String,
}

impl Session {
    /// The parsed responses; panics unless there is exactly one per non-blank request line.
    fn responses(&self, requests: usize) -> Vec<J> {
        assert_eq!(
            self.lines.len(),
            requests,
            "kiln script printed {} response lines for {requests} request lines (exit {}, \
             stderr: {})",
            self.lines.len(),
            self.out.code,
            self.out.stderr_text()
        );
        self.lines.iter().map(|l| parse_json(l)).collect()
    }
}

/// Script lines as bytes, each followed by `\n`.
fn script_bytes(lines: &[Vec<u8>]) -> Vec<u8> {
    let mut b = Vec::new();
    for l in lines {
        b.extend_from_slice(l);
        b.push(b'\n');
    }
    b
}

fn as_bytes(lines: &[String]) -> Vec<Vec<u8>> {
    lines.iter().map(|l| l.as_bytes().to_vec()).collect()
}

/// Write `lines` to `<dir>/<name>.jsonl` and run `kiln script <name>.jsonl <fs_args…> --log
/// <name>.log.jsonl` in `dir`.
fn run_script_bytes(dir: &Path, name: &str, lines: &[Vec<u8>], fs_args: &[&str]) -> Session {
    let script = format!("{name}.jsonl");
    let log = format!("{name}.log.jsonl");
    write(&dir.join(&script), &script_bytes(lines));
    let _ = fs::remove_file(dir.join(&log));
    let mut args = vec!["script", script.as_str()];
    args.extend_from_slice(fs_args);
    args.extend_from_slice(&["--log", log.as_str()]);
    let out = kiln(dir, &args);
    session_from(out, &dir.join(&log))
}

fn session_from(out: Out, log: &Path) -> Session {
    let text = String::from_utf8(out.stdout.clone()).expect("stdout is UTF-8");
    let lines = text.lines().map(str::to_owned).collect();
    let log = fs::read_to_string(log).unwrap_or_default();
    Session { out, lines, log }
}

/// [`run_script_bytes`] over `--root proj`.
fn run_script(dir: &Path, name: &str, lines: &[String]) -> Session {
    run_script_bytes(dir, name, &as_bytes(lines), &["--root", "proj"])
}

/// Feed `lines` to `host` as `kiln-cli run` does; the response lines.
fn host_feed(host: &mut Host, lines: &[Vec<u8>]) -> Vec<String> {
    let bytes = script_bytes(lines);
    bytes
        .split(|b| *b == b'\n')
        .filter_map(|l| host.handle_line(l))
        .collect()
}

/// The exit code `kiln-cli run` documents for these responses.
fn run_exit_code(responses: &[String]) -> i32 {
    if responses
        .iter()
        .all(|r| parse_json(r).get("ok") == Some(&J::Bool(true)))
    {
        0
    } else {
        3
    }
}

/// The entries of a log script, parsed.
fn log_entries(log: &str) -> Vec<J> {
    log.lines().map(parse_json).collect()
}

/// The `params.path` of every `doc.open` in a log.
fn logged_opens(log: &str) -> Vec<String> {
    log_entries(log)
        .iter()
        .filter(|e| e.at("command").as_str() == Some("doc.open"))
        .map(|e| {
            e.at("params")
                .at("path")
                .as_str()
                .expect("path is a string")
                .to_owned()
        })
        .collect()
}

/// Replay a log script with `Host` on `root`; every request must succeed again.
fn replay(root: &Path, log: &str) -> Host {
    let mut host = Host::on_disk(root).expect("Host::on_disk on the replay copy");
    let lines: Vec<Vec<u8>> = log.lines().map(|l| l.as_bytes().to_vec()).collect();
    for (line, resp) in lines.iter().zip(host_feed(&mut host, &lines)) {
        assert_eq!(
            parse_json(&resp).get("ok"),
            Some(&J::Bool(true)),
            "replaying the log: {} answered {resp}",
            String::from_utf8_lossy(line)
        );
    }
    host
}

/// `result.docs` of a `doc.list` response as (doc, path) pairs.
fn doc_list(resp: &J) -> Vec<(u64, String)> {
    assert_eq!(
        resp.get("ok"),
        Some(&J::Bool(true)),
        "doc.list: {}",
        resp.show()
    );
    resp.at("result")
        .at("docs")
        .as_arr()
        .expect("docs is an array")
        .iter()
        .map(|d| {
            (
                d.at("doc").as_u64().expect("doc id"),
                d.at("path").as_str().expect("path").to_owned(),
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Snapshots and the shell.
// ---------------------------------------------------------------------------------------------

/// A `snapshot` response: (`result.root`, `result.frames`); checks AC-10 on it.
fn snap(resp: &J) -> (&J, u64) {
    assert_eq!(
        resp.get("ok"),
        Some(&J::Bool(true)),
        "snapshot refused: {}",
        resp.show()
    );
    let result = resp.at("result");
    let frames = result
        .at("frames")
        .as_u64()
        .expect("snapshot result.frames is an unsigned integer");
    let root = result.at("root");
    check_names(root);
    (root, frames)
}

fn children(n: &J) -> &[J] {
    n.at("children")
        .as_arr()
        .unwrap_or_else(|| panic!("node without a children array: {}", n.show()))
}

fn role(n: &J) -> Option<&str> {
    n.get("role").and_then(J::as_str)
}

fn label(n: &J) -> Option<&str> {
    n.get("label").and_then(J::as_str)
}

fn value(n: &J) -> Option<&str> {
    n.get("value").and_then(J::as_str)
}

fn all_nodes(root: &J) -> Vec<&J> {
    let mut out = vec![root];
    let mut i = 0;
    while i < out.len() {
        let n = out[i];
        out.extend(children(n).iter());
        i += 1;
    }
    out
}

/// The one node with this label.
fn by_label<'a>(root: &'a J, l: &str) -> &'a J {
    let found: Vec<&J> = all_nodes(root)
        .into_iter()
        .filter(|n| label(n) == Some(l))
        .collect();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one node labelled `{l}`, found {}: {}",
        found.len(),
        root.show()
    );
    found[0]
}

/// AC-10: every `button` and `text_input` has a non-empty `label`.
fn check_names(root: &J) {
    for n in all_nodes(root) {
        if matches!(role(n), Some("button" | "text_input")) {
            assert!(
                label(n).is_some_and(|l| !l.is_empty()),
                "AC-10: a {} node without a non-empty label: {}",
                role(n).unwrap_or_default(),
                n.show()
            );
        }
    }
}

/// The shell nodes of Interface "The shell", checked for their roles.
struct Shell<'a> {
    path: &'a J,
    open: &'a J,
    documents: &'a J,
    message: &'a J,
}

fn shell(root: &J) -> Shell<'_> {
    let path = by_label(root, "Path");
    assert_eq!(role(path), Some("text_input"), "Path: {}", path.show());
    let open = by_label(root, "Open");
    assert_eq!(role(open), Some("button"), "Open: {}", open.show());
    Shell {
        path,
        open,
        documents: by_label(root, "Documents"),
        message: by_label(root, "Message"),
    }
}

/// The `value`s of the children of `Documents`, in order.
fn documents(root: &J) -> Vec<String> {
    children(shell(root).documents)
        .iter()
        .map(|c| {
            value(c)
                .unwrap_or_else(|| panic!("a Documents child without a value: {}", c.show()))
                .to_owned()
        })
        .collect()
}

fn message_value(root: &J) -> String {
    value(shell(root).message).unwrap_or_default().to_owned()
}

/// An action response: `ok:true`, `result.frames` in 1..=60 (`count` for `frames`), and
/// `settled` false only when the cap of 60 was reached. Returns the frame count.
fn action_ok(resp: &J, frames_count: Option<u64>) -> u64 {
    assert_eq!(
        resp.get("ok"),
        Some(&J::Bool(true)),
        "UI action refused: {}",
        resp.show()
    );
    let result = resp.at("result");
    let n = result
        .at("frames")
        .as_u64()
        .expect("result.frames is an unsigned integer");
    let settled = result
        .at("settled")
        .as_bool()
        .expect("result.settled is a boolean");
    match frames_count {
        Some(count) => assert_eq!(n, count, "`frames` runs exactly `count` frames"),
        None => {
            assert!((1..=60).contains(&n), "an action ran {n} frames (1 to 60)");
            assert!(
                settled || n == 60,
                "not settled after {n} frames, below the cap of 60"
            );
        }
    }
    n
}

/// A UI refusal of `kind`.
fn ui_refusal(resp: &J, kind: &str, what: &str) {
    assert_eq!(
        resp.get("ok"),
        Some(&J::Bool(false)),
        "{what}: {}",
        resp.show()
    );
    let e = resp.at("error");
    assert_eq!(e.at("code").as_str(), Some("ui"), "{what}: {}", resp.show());
    assert_eq!(e.at("kind").as_str(), Some(kind), "{what}: {}", resp.show());
    let message = e.at("message").as_str().expect("error.message is a string");
    assert!(
        message.chars().count() <= 200,
        "{what}: message longer than 200 chars"
    );
}

fn focus_path() -> String {
    r#"{"id":"focus","ui":"focus","target":{"label":"Path"}}"#.to_owned()
}

fn type_text(id: &str, text: &str) -> String {
    format!(r#"{{"id":{},"text":{},"ui":"type"}}"#, q(id), q(text))
}

fn click_open() -> String {
    r#"{"id":"click","target":{"label":"Open"},"ui":"click"}"#.to_owned()
}

/// `{"ui":"frames","count":1}`: a control request's effect is shown from the next frame on
/// (D-149 n4), so a script that wants to see it in a snapshot runs a frame first.
fn one_frame(id: &str) -> String {
    format!(r#"{{"count":1,"id":{},"ui":"frames"}}"#, q(id))
}

fn snapshot(id: &str) -> String {
    format!(r#"{{"id":{},"ui":"snapshot"}}"#, q(id))
}

fn control(id: u64, command: &str, params: &str) -> String {
    format!(
        r#"{{"command":{},"id":{id},"params":{params}}}"#,
        q(command)
    )
}

fn open(id: u64, path: &str) -> String {
    control(id, "doc.open", &format!("{{\"path\":{}}}", q(path)))
}

// ---------------------------------------------------------------------------------------------
// AC-1: control pass-through (oracle: kiln_control::Host, differential).
// ---------------------------------------------------------------------------------------------

/// Run `lines` through `kiln script` on `<dir>/proj` and through `Host` on a copy of it made
/// before the run; stdout, exit code, `--log` and the files afterwards must agree.
fn assert_pass_through(dir: &Path, name: &str, lines: &[Vec<u8>]) {
    let root = dir.join("proj");
    let oracle_root = dir.join(format!("{name}-oracle"));
    let _ = fs::remove_dir_all(&oracle_root);
    copy_dir(&root, &oracle_root);
    let mut host = Host::on_disk(&oracle_root).expect("Host::on_disk on the copy");
    let expected = host_feed(&mut host, lines);
    let sess = run_script_bytes(dir, name, lines, &["--root", "proj"]);
    let mut expected_stdout = String::new();
    for r in &expected {
        expected_stdout.push_str(r);
        expected_stdout.push('\n');
    }
    let got = String::from_utf8_lossy(&sess.out.stdout);
    if got != expected_stdout {
        let first = sess
            .lines
            .iter()
            .zip(&expected)
            .position(|(a, b)| a != b)
            .unwrap_or(sess.lines.len().min(expected.len()));
        panic!(
            "{name}: kiln script stdout differs from Host at response {first} \
             ({} vs {} lines; exit {}, stderr {}):\n  kiln: {:?}\n  Host: {:?}",
            sess.lines.len(),
            expected.len(),
            sess.out.code,
            sess.out.stderr_text(),
            sess.lines.get(first),
            expected.get(first)
        );
    }
    assert_eq!(
        sess.out.code,
        run_exit_code(&expected),
        "{name}: exit code (stderr {})",
        sess.out.stderr_text()
    );
    assert_eq!(
        sess.log,
        host.log_script(),
        "{name}: --log vs Host::log_script()"
    );
    assert_eq!(
        tree(&root),
        tree(&oracle_root),
        "{name}: files after the run"
    );
}

/// A deterministic generator (xorshift64*).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[usize::try_from(self.below(xs.len() as u64)).expect("index")]
    }

    fn flag(&mut self) -> bool {
        self.below(2) == 1
    }
}

/// One random control line (never with a `ui` key): `doc.*`, `cursor.*`, `search.*`,
/// `syntax.*`, `project.tree`, or a malformed line.
fn random_control_line(rng: &mut Rng, k: u64) -> Vec<u8> {
    const PATHS: [&str; 11] = [
        "src/a.rs",
        "src/lib.rs",
        "b.txt",
        "notes/c.md",
        "u.txt",
        "src/missing.rs",
        "src",
        "../x",
        "",
        "/abs",
        "a//b",
    ];
    const TEXTS: [&str; 6] = ["x", "// \u{fc}\n", "\r\n", "\u{1F600}", "", "{ ( ["];
    const QUERIES: [&str; 6] = ["a", "fn", "\u{e9}", "x+", "[", "line"];
    const MALFORMED: [&[u8]; 16] = [
        b"{\"id\":",
        b"[1,2]",
        b"\"just a string\"",
        b"{\"id\":1.5,\"command\":\"doc.list\"}",
        b"{\"command\":\"doc.list\",\"extra\":1}",
        b"{\"id\":\"m\"}",
        b"{\"command\":5}",
        b"{\"command\":\"no.such-command\"}",
        b"{\"command\":\"doc.list\",\"params\":[]}",
        b"{\"command\":\"doc.open\",\"params\":{\"path\":\"\xff\xfe\"}}",
        b"",
        b"   \t",
        b"{\"command\":\"doc.list\"}\r",
        b"{\"id\":true,\"command\":\"doc.list\"}",
        b"null",
        b"{\"command\":\"doc.insert\",\"params\":{\"doc\":1}}",
    ];
    let id = match rng.below(4) {
        0 => String::new(),
        1 => format!("\"id\":{k},"),
        2 => format!("\"id\":\"r{k}\","),
        _ => format!("\"id\":-{k},"),
    };
    let doc = rng.below(4);
    let off = rng.below(40);
    let (command, params) = match rng.below(26) {
        0..=3 => ("doc.open", format!("{{\"path\":{}}}", q(rng.pick(&PATHS)))),
        4 => ("doc.list", "{}".to_owned()),
        5 => ("doc.text", format!("{{\"doc\":{doc}}}")),
        6 => ("doc.info", format!("{{\"doc\":{doc}}}")),
        7 => (
            "doc.insert",
            format!(
                "{{\"doc\":{doc},\"offset\":{off},\"text\":{}}}",
                q(rng.pick(&TEXTS))
            ),
        ),
        8 => {
            let end = if rng.below(5) == 0 {
                off.saturating_sub(1)
            } else {
                off + rng.below(5)
            };
            (
                "doc.delete",
                format!("{{\"doc\":{doc},\"end\":{end},\"start\":{off}}}"),
            )
        }
        9 => ("doc.undo", format!("{{\"doc\":{doc}}}")),
        10 => ("doc.redo", format!("{{\"doc\":{doc}}}")),
        11 => ("doc.save", format!("{{\"doc\":{doc}}}")),
        12 => (
            "doc.close",
            format!("{{\"discard\":{},\"doc\":{doc}}}", rng.flag()),
        ),
        13 => ("cursor.get", format!("{{\"doc\":{doc}}}")),
        14 => (
            "cursor.set",
            format!(
                "{{\"anchor\":{off},\"doc\":{doc},\"head\":{}}}",
                rng.below(40)
            ),
        ),
        15 => (
            "cursor.move",
            format!(
                "{{\"by\":{},\"doc\":{doc},\"extend\":{},\"forward\":{}}}",
                q(rng.pick(&["char", "word", "line", "bogus"])),
                rng.flag(),
                rng.flag()
            ),
        ),
        16 => (
            "search.find",
            format!(
                "{{\"doc\":{doc},\"query\":{},\"regex\":{}}}",
                q(rng.pick(&QUERIES)),
                rng.flag()
            ),
        ),
        17 => (
            "search.replace",
            format!(
                "{{\"all\":{},\"doc\":{doc},\"query\":{},\"replacement\":{}}}",
                rng.flag(),
                q(rng.pick(&QUERIES)),
                q(rng.pick(&TEXTS))
            ),
        ),
        18 => ("syntax.highlight", format!("{{\"doc\":{doc}}}")),
        19 => (
            "syntax.match-bracket",
            format!("{{\"doc\":{doc},\"offset\":{off}}}"),
        ),
        20 => ("syntax.tree", format!("{{\"doc\":{doc}}}")),
        21 => (
            "project.tree",
            rng.pick(&[
                "{}",
                "{\"path\":\"src\"}",
                "{\"limit\":2}",
                "{\"path\":\"../x\"}",
            ])
            .to_string(),
        ),
        _ => return rng.pick(&MALFORMED).to_vec(),
    };
    format!("{{{id}\"command\":{},\"params\":{params}}}", q(command)).into_bytes()
}

#[test]
fn ac1_control_pass_through() {
    let dir = case_dir("ac1");
    fixture(&dir);

    // The worked example of Pinned (g).
    let g = [
        r#"{"id":1,"command":"doc.list"}"#.to_owned(),
        r#"{"id":2,"command":"doc.open","params":{"path":"src/a.rs"}}"#.to_owned(),
        r#"{"id":3,"command":"doc.open","params":{"path":"src/missing.rs"}}"#.to_owned(),
        r#"{"id":4,"command":"doc.list"}"#.to_owned(),
    ];
    assert_pass_through(&dir, "pinned-g", &as_bytes(&g));

    // The worked example of docs/control-protocol.md (a.txt = "hello\n"; it saves).
    write(&dir.join("proj/a.txt"), b"hello\n");
    let w = [
        r#"{"id":1,"command":"doc.open","params":{"path":"a.txt"}}"#,
        r#"{"id":2,"command":"doc.insert","params":{"doc":1,"offset":5,"text":" world"}}"#,
        r#"{"id":3,"command":"doc.undo","params":{"doc":1}}"#,
        r#"{"id":4,"command":"doc.redo","params":{"doc":1}}"#,
        r#"{"id":"oops","command":"doc.insert","params":{"doc":1}}"#,
        r#"{"id":5,"command":"doc.save","params":{"doc":1}}"#,
    ];
    let w: Vec<String> = w.iter().map(|s| (*s).to_owned()).collect();
    assert_pass_through(&dir, "protocol-example", &as_bytes(&w));
    assert_eq!(
        fs::read(dir.join("proj/a.txt")).expect("read a.txt"),
        b"hello world\n",
        "the worked example saves `hello world\\n`"
    );

    // Random sequences, malformed lines included.
    for seed in [0x0055_0001_u64, 0x0055_0002, 0x0055_0003] {
        let mut rng = Rng(seed);
        let lines: Vec<Vec<u8>> = (1..=90).map(|k| random_control_line(&mut rng, k)).collect();
        assert_pass_through(&dir, &format!("random-{seed:x}"), &lines);
    }

    // A huge line is read whole.
    let big = "y".repeat(300_000);
    let huge = vec![
        open(1, "b.txt").into_bytes(),
        control(
            2,
            "doc.insert",
            &format!("{{\"doc\":1,\"offset\":0,\"text\":{}}}", q(&big)),
        )
        .into_bytes(),
        control(3, "doc.info", "{\"doc\":1}").into_bytes(),
    ];
    assert_pass_through(&dir, "huge-line", &huge);

    // `--memory`: an empty in-memory file system.
    let mem: Vec<Vec<u8>> = [
        r#"{"id":1,"command":"doc.list"}"#,
        r#"{"id":2,"command":"doc.open","params":{"path":"a.txt"}}"#,
        r#"{"id":3,"command":"project.tree"}"#,
    ]
    .iter()
    .map(|s| s.as_bytes().to_vec())
    .collect();
    let mut host = Host::in_memory(&[]).expect("Host::in_memory");
    let expected = host_feed(&mut host, &mem);
    let sess = run_script_bytes(&dir, "memory", &mem, &["--memory"]);
    assert_eq!(
        sess.lines, expected,
        "--memory: stdout vs Host::in_memory(&[])"
    );
    assert_eq!(
        sess.out.code,
        run_exit_code(&expected),
        "--memory: exit code"
    );
    assert_eq!(sess.log, host.log_script(), "--memory: --log");
    let _ = fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------------------------
// AC-2: the shell is shown (oracle: doc.list in the same session).
// ---------------------------------------------------------------------------------------------

/// The script of AC-2 (also AC-9's).
fn ac2_script() -> Vec<String> {
    vec![
        snapshot("s0"),
        open(1, "src/a.rs"),
        open(2, "b.txt"),
        one_frame("frame"),
        snapshot("s1"),
        control(3, "doc.list", "{}"),
    ]
}

/// The checks of AC-2 on the responses of [`ac2_script`].
fn check_ac2(r: &[J]) {
    let (s0, _) = snap(&r[0]);
    let sh = shell(s0);
    assert!(
        children(sh.documents).is_empty(),
        "a fresh session lists documents: {}",
        sh.documents.show()
    );
    assert_eq!(message_value(s0), "", "Message of a fresh session");
    assert_eq!(r[1].get("ok"), Some(&J::Bool(true)), "{}", r[1].show());
    assert_eq!(r[2].get("ok"), Some(&J::Bool(true)), "{}", r[2].show());
    action_ok(&r[3], Some(1));
    let (s1, _) = snap(&r[4]);
    let listed: Vec<String> = doc_list(&r[5]).into_iter().map(|(_, p)| p).collect();
    assert_eq!(listed, ["src/a.rs", "b.txt"], "doc.list in the session");
    assert_eq!(documents(s1), listed, "Documents vs doc.list");
}

#[test]
fn ac2_shell_shown() {
    let dir = case_dir("ac2");
    fixture(&dir);
    let script = ac2_script();
    let sess = run_script(&dir, "s", &script);
    let r = sess.responses(script.len());
    check_ac2(&r);
    assert_eq!(sess.out.code, 0, "every response ok:true");
    let _ = fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------------------------
// AC-3: the UI dispatches (oracles: registry state, log replay).
// ---------------------------------------------------------------------------------------------

fn ui_open_and_replay(name: &str, confirm: &str) {
    let dir = case_dir(name);
    let root = fixture(&dir);
    let initial = dir.join("initial");
    copy_dir(&root, &initial);
    let script = vec![
        snapshot("s0"),
        focus_path(),
        type_text("type", "src/a.rs"),
        snapshot("s1"),
        confirm.to_owned(),
        snapshot("s2"),
        control(1, "doc.list", "{}"),
        control(
            2,
            "doc.insert",
            r#"{"doc":1,"offset":0,"text":"// via ui\n"}"#,
        ),
        control(3, "doc.save", r#"{"doc":1}"#),
        control(4, "doc.list", "{}"),
    ];
    let sess = run_script(&dir, "s", &script);
    let r = sess.responses(script.len());
    let (s0, _) = snap(&r[0]);
    assert!(documents(s0).is_empty(), "{name}: no document before");
    action_ok(&r[1], None);
    action_ok(&r[2], None);
    let (s1, _) = snap(&r[3]);
    let path = shell(s1).path;
    assert_eq!(
        value(path),
        Some("src/a.rs"),
        "{name}: typed into Path: {}",
        path.show()
    );
    assert_eq!(
        path.get("focused"),
        Some(&J::Bool(true)),
        "{name}: Path focused"
    );
    let sel = path.at("selection");
    assert_eq!(
        (sel.at("anchor").as_u64(), sel.at("head").as_u64()),
        (Some(8), Some(8)),
        "{name}: caret after the 8 typed chars"
    );
    action_ok(&r[4], None);
    let (s2, _) = snap(&r[5]);
    assert_eq!(
        documents(s2),
        ["src/a.rs"],
        "{name}: Documents after the open"
    );
    assert_eq!(
        doc_list(&r[6]),
        [(1, "src/a.rs".to_owned())],
        "{name}: doc.list after the open"
    );
    for (i, resp) in r.iter().enumerate().skip(7) {
        assert_eq!(
            resp.get("ok"),
            Some(&J::Bool(true)),
            "{name}: line {i}: {}",
            resp.show()
        );
    }
    assert_eq!(sess.out.code, 0, "{name}: exit code");
    assert_eq!(
        logged_opens(&sess.log),
        ["src/a.rs"],
        "{name}: the log holds the UI's doc.open: {}",
        sess.log
    );
    let on_disk = fs::read_to_string(root.join("src/a.rs")).expect("read src/a.rs");
    assert_eq!(
        on_disk,
        format!("// via ui\n{A_RS}"),
        "{name}: saved through doc 1"
    );

    // Replay the log on a copy of the initial root.
    let host = &mut replay(&initial, &sess.log);
    let after = host
        .handle_line(br#"{"id":1,"command":"doc.list"}"#)
        .expect("a response");
    assert_eq!(
        doc_list(&parse_json(&after)),
        doc_list(&r[9]),
        "{name}: doc.list after the replay vs the session's last doc.list"
    );
    assert_eq!(
        tree(&initial),
        tree(&root),
        "{name}: files after the replay"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn ac3_ui_dispatches_on_click() {
    ui_open_and_replay("ac3-click", &click_open());
}

#[test]
fn ac3_ui_dispatches_on_enter() {
    ui_open_and_replay("ac3-enter", r#"{"id":"enter","key":"Enter","ui":"key"}"#);
}

// ---------------------------------------------------------------------------------------------
// AC-4: refusals shown (oracle: the same doc.open as a control request in a second session).
// ---------------------------------------------------------------------------------------------

#[test]
fn ac4_refusals_shown() {
    let cases = [
        ("missing", "src/missing.rs", "not_found"),
        ("directory", "src", "not_a_file"),
        ("symlink", "out/secret.txt", "outside_root"),
        ("dotdot", "../x", "invalid_path"),
    ];
    for (name, path, kind) in cases {
        let dir = case_dir(&format!("ac4-{name}"));
        let root = fixture(&dir);
        let outside = dir.join("outside");
        write(&outside.join("secret.txt"), b"secret\n");
        write(&dir.join("x"), b"beside the root\n");
        std::os::unix::fs::symlink(&outside, root.join("out")).expect("symlink out");
        // The targets exist (so a refusal is the check, not a missing file).
        assert!(outside.join("secret.txt").is_file() && dir.join("x").is_file());
        let before = tree(&dir);

        // Oracle: a second session.
        let mut host = Host::on_disk(&root).expect("Host::on_disk");
        let control_resp = parse_json(
            &host
                .handle_line(open(1, path).as_bytes())
                .expect("a response"),
        );
        assert_eq!(
            control_resp.get("ok"),
            Some(&J::Bool(false)),
            "{name}: control doc.open"
        );
        let oracle_kind = control_resp.at("error").at("kind").as_str().expect("kind");
        assert_eq!(
            oracle_kind, kind,
            "{name}: the kind a control doc.open reports"
        );

        let script = vec![
            open(1, "b.txt"),
            one_frame("frame"),
            snapshot("s0"),
            focus_path(),
            type_text("type", path),
            click_open(),
            snapshot("s1"),
            control(2, "doc.list", "{}"),
        ];
        let sess = run_script(&dir, "s", &script);
        let mut r = sess.responses(script.len());
        action_ok(&r[1], Some(1));
        // Index as before the frame line: r[0] the open, r[1] the snapshot.
        r.remove(1);
        let (s0, _) = snap(&r[1]);
        assert_eq!(documents(s0), ["b.txt"], "{name}: Documents before");
        assert_eq!(message_value(s0), "", "{name}: Message before");
        action_ok(&r[2], None);
        action_ok(&r[3], None);
        action_ok(&r[4], None);
        let (s1, _) = snap(&r[5]);
        assert_eq!(documents(s1), documents(s0), "{name}: Documents unchanged");
        let msg = message_value(s1);
        assert!(
            msg.starts_with(oracle_kind),
            "{name}: Message `{msg}` does not begin with `{oracle_kind}`"
        );
        assert_eq!(
            doc_list(&r[6]),
            [(1, "b.txt".to_owned())],
            "{name}: doc.list after the refused open"
        );
        assert_eq!(
            logged_opens(&sess.log),
            ["b.txt"],
            "{name}: log: {}",
            sess.log
        );
        assert_eq!(sess.out.code, 0, "{name}: every response ok:true");
        // Nothing outside the scripts' own files changed.
        let mut after = tree(&dir);
        after.retain(|k, _| !k.starts_with("s."));
        let mut before = before.clone();
        before.retain(|k, _| !k.starts_with("s."));
        assert_eq!(after, before, "{name}: files unchanged");
        let _ = fs::remove_dir_all(&dir);
    }
}

// ---------------------------------------------------------------------------------------------
// AC-5: idle (oracle: log comparison).
// ---------------------------------------------------------------------------------------------

fn idle_case(name: &str, prefix: &[String]) {
    let dir = case_dir(name);
    fixture(&dir);
    let idle = r#"{"count":600,"id":"idle","ui":"frames"}"#.to_owned();
    let mut with = prefix.to_vec();
    with.extend([
        one_frame("frame"),
        snapshot("s0"),
        idle,
        snapshot("s1"),
        control(9, "doc.list", "{}"),
    ]);
    let mut without = prefix.to_vec();
    without.extend([
        one_frame("frame"),
        snapshot("s0"),
        snapshot("s1"),
        control(9, "doc.list", "{}"),
    ]);

    let a = run_script(&dir, "with", &with);
    let ra = a.responses(with.len());
    fixture(&dir); // the same initial files for the second run
    let b = run_script(&dir, "without", &without);
    let rb = b.responses(without.len());
    let n = prefix.len() + 1;
    let (s0, f0) = snap(&ra[n]);
    assert_eq!(action_ok(&ra[n + 1], Some(600)), 600);
    let (s1, f1) = snap(&ra[n + 2]);
    assert_eq!(s1, s0, "{name}: the snapshot after 600 idle frames differs");
    assert_eq!(f1 - f0, 600, "{name}: frames counted by the snapshots");
    assert!(!a.log.is_empty(), "{name}: the session log is empty");
    assert_eq!(a.log, b.log, "{name}: idle frames added to the log");
    assert_eq!(
        doc_list(&ra[n + 3]),
        doc_list(&rb[n + 2]),
        "{name}: doc.list"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn ac5_idle_frames_add_nothing() {
    idle_case("ac5-fresh", &[open(1, "b.txt")]);
    idle_case(
        "ac5-after-ui",
        &[focus_path(), type_text("type", "src/a.rs"), click_open()],
    );
}

// ---------------------------------------------------------------------------------------------
// AC-6: determinism (oracle: byte equality).
// ---------------------------------------------------------------------------------------------

/// A script using every UI action and control requests.
fn ac6_script() -> Vec<String> {
    vec![
        snapshot("s0"),
        open(1, "src/a.rs"),
        r#"{"count":3,"id":"frames","ui":"frames"}"#.to_owned(),
        focus_path(),
        type_text("t1", "b.tx"),
        r#"{"id":"k1","key":"T","modifiers":["shift"],"ui":"key"}"#.to_owned(),
        r#"{"id":"k2","key":"Backspace","ui":"key"}"#.to_owned(),
        type_text("t2", "t"),
        snapshot("s1"),
        r#"{"id":"k3","key":"Enter","ui":"key"}"#.to_owned(),
        type_text("t3", "u.txt"),
        click_open(),
        r#"{"button":"secondary","id":"c2","modifiers":["ctrl","alt"],"target":{"label":"Open"},"ui":"click"}"#.to_owned(),
        r#"{"at":{"col":0,"row":0},"id":"c3","target":{"label":"Open"},"ui":"click"}"#.to_owned(),
        r#"{"direction":"down","id":"sc","steps":3,"target":{"label":"Documents"},"ui":"scroll"}"#.to_owned(),
        r#"{"id":"k4","key":"Tab","ui":"key"}"#.to_owned(),
        r#"{"id":"k5","key":"ArrowLeft","modifiers":["shift"],"ui":"key"}"#.to_owned(),
        r#"{"id":"bad","target":{"label":"no such node"},"ui":"click"}"#.to_owned(),
        control(2, "doc.list", "{}"),
        control(3, "doc.text", r#"{"doc":1}"#),
        control(4, "syntax.highlight", r#"{"doc":1}"#),
        snapshot("s2"),
    ]
}

/// The run's precondition: one response per line, the snapshots and the control requests
/// answered, the log not empty.
fn check_ac6_run(sess: &Session, what: &str) {
    let script = ac6_script();
    let r = sess.responses(script.len());
    for i in [0, 8, 21] {
        snap(&r[i]);
    }
    for i in [1, 18, 19, 20] {
        assert_eq!(
            r[i].get("ok"),
            Some(&J::Bool(true)),
            "{what}: {}",
            r[i].show()
        );
    }
    action_ok(&r[2], Some(3));
    action_ok(&r[3], None);
    assert!(!sess.log.is_empty(), "{what}: empty log");
}

#[test]
fn ac6_determinism() {
    let base = case_dir("ac6");
    let script = ac6_script();
    let roots = [
        base.join("a"),
        base.join("bb/located/elsewhere/deeper"),
        base.join("c"),
        base.join("dd/x"),
        base.join("e/eee"),
    ];
    for r in &roots {
        fs::create_dir_all(r).expect("create");
        fixture(r);
    }
    let first = run_script(&roots[0], "s", &script);
    check_ac6_run(&first, "run 1");
    let second = run_script(&roots[1], "s", &script);
    check_ac6_run(&second, "run 2 (another location)");

    // Three more runs at once, while busy threads of this process load the machine.
    let stop = Arc::new(AtomicBool::new(false));
    let burners: Vec<_> = (0..2)
        .map(|_| {
            let stop = Arc::clone(&stop);
            thread::spawn(move || {
                let mut x = 1_u64;
                while !stop.load(Ordering::Relaxed) {
                    x = x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                }
                x
            })
        })
        .collect();
    let loaded: Vec<_> = roots[2..]
        .iter()
        .map(|r| {
            let r = r.clone();
            let script = script.clone();
            thread::spawn(move || run_script(&r, "s", &script))
        })
        .collect();
    let loaded: Vec<Session> = loaded
        .into_iter()
        .map(|h| h.join().expect("loaded run"))
        .collect();
    stop.store(true, Ordering::Relaxed);
    for b in burners {
        let _ = b.join();
    }
    for (i, run) in std::iter::once(&second).chain(&loaded).enumerate() {
        let what = format!("run {}", i + 2);
        check_ac6_run(run, &what);
        assert_eq!(
            run.out.stdout, first.out.stdout,
            "{what}: stdout differs from run 1"
        );
        assert_eq!(run.log, first.log, "{what}: log differs from run 1");
        assert_eq!(run.out.code, first.out.code, "{what}: exit code");
        assert_eq!(
            tree(&roots[i + 1].join("proj")),
            tree(&roots[0].join("proj"))
        );
    }
    let _ = fs::remove_dir_all(&base);
}

/// Interface "Responses" (S6): the snapshot's `frames` is the session's total, so between two
/// snapshots it grows by exactly the frames the actions between them reported; a refused request
/// and a control request run none; frames run before the first request.
#[test]
fn ac6_frame_counts_add_up() {
    let dir = case_dir("ac6-frames");
    fixture(&dir);
    let script = ac6_script();
    let sess = run_script(&dir, "s", &script);
    let r = sess.responses(script.len());
    let (_, mut last) = snap(&r[0]);
    assert!(last >= 1, "no frame ran before the first request");
    let mut sum = 0;
    for (line, resp) in script.iter().zip(&r).skip(1) {
        if line.contains("\"ui\":\"snapshot\"") {
            let (_, total) = snap(resp);
            assert_eq!(total - last, sum, "frames between snapshots, up to {line}");
            last = total;
            sum = 0;
        } else if line.contains("\"ui\":") && resp.get("ok") == Some(&J::Bool(true)) {
            let count = line.contains("\"ui\":\"frames\"").then_some(3);
            sum += action_ok(resp, count);
        }
    }
    let _ = fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------------------------
// AC-7: UI refusals (oracle: exact kinds; snapshot and log equality).
// ---------------------------------------------------------------------------------------------

#[test]
fn ac7_ui_refusals() {
    let dir = case_dir("ac7");
    fixture(&dir);
    let setup = vec![
        open(1, "src/a.rs"),
        open(2, "b.txt"),
        one_frame("frame"),
        snapshot("s0"),
    ];

    // The setup alone: the state the refusals must leave, and Open's painted rows.
    let base = run_script(&dir, "base", &setup);
    let rb = base.responses(setup.len());
    let (b0, _) = snap(&rb[setup.len() - 1]);
    assert!(
        all_nodes(b0).iter().all(|n| n.get("focused").is_none()),
        "precondition of `type` with nothing focused: a node is focused in a fresh session"
    );
    let rows = shell(b0)
        .open
        .get("rows")
        .and_then(J::as_arr)
        .unwrap_or_else(|| panic!("precondition: Open shows text, so it has `rows`"))
        .to_vec();
    assert!(!rows.is_empty(), "precondition: Open has a painted row");
    let row0_len: usize = rows[0]
        .as_arr()
        .expect("a row is an array of runs")
        .iter()
        .map(|run| run.at("text").as_str().expect("run text").chars().count())
        .sum();
    let long_label = "n".repeat(1000);

    let refusals: Vec<(String, &str)> = vec![
        (r#"{"bogus":1,"id":"r1","ui":"snapshot"}"#.to_owned(), "bad_request"),
        (r#"{"id":"r2","ui":"click"}"#.to_owned(), "bad_request"),
        (r#"{"count":0,"id":"r3","ui":"frames"}"#.to_owned(), "bad_request"),
        (r#"{"count":601,"id":"r4","ui":"frames"}"#.to_owned(), "bad_request"),
        (r#"{"id":"r5","key":"A","modifiers":["hyper"],"ui":"key"}"#.to_owned(), "bad_request"),
        (r#"{"id":"r6","target":{},"ui":"click"}"#.to_owned(), "bad_request"),
        (r#"{"id":"r7","key":"NoSuchKey","ui":"key"}"#.to_owned(), "bad_request"),
        (r#"{"count":"5","id":"r8","ui":"frames"}"#.to_owned(), "bad_request"),
        (r#"{"id":"r9","target":{"label":"Open"},"ui":"snapshot"}"#.to_owned(), "bad_request"),
        (r#"{"id":"r10","ui":"dance"}"#.to_owned(), "unknown_action"),
        (r#"{"id":"r11","target":{"label":"No such node here"},"ui":"click"}"#.to_owned(), "no_target"),
        (format!(r#"{{"id":"r12","target":{{"label":{}}},"ui":"click"}}"#, q(&long_label)), "no_target"),
        (r#"{"id":"r13","target":{"value_contains":"."},"ui":"click"}"#.to_owned(), "ambiguous_target"),
        (type_text("r14", "x"), "not_actionable"),
        (
            format!(r#"{{"at":{{"col":0,"row":{}}},"id":"r15","target":{{"label":"Open"}},"ui":"click"}}"#, rows.len()),
            "not_actionable",
        ),
        (
            format!(r#"{{"at":{{"col":{},"row":0}},"id":"r16","target":{{"label":"Open"}},"ui":"click"}}"#, row0_len + 1),
            "not_actionable",
        ),
        // Precedence (Pinned (a)): two faults, the first kind in the order wins.
        (r#"{"bogus":1,"id":"p1","ui":"dance"}"#.to_owned(), "bad_request"),
        (r#"{"count":3,"id":"p2","target":{"label":"No such node here"},"ui":"click"}"#.to_owned(), "bad_request"),
        (r#"{"id":"p3","modifiers":["hyper"],"target":{"label":"No such node here"},"ui":"click"}"#.to_owned(), "bad_request"),
        (r#"{"at":{"col":0,"row":99},"id":"p4","target":{"label":"No such node here"},"ui":"click"}"#.to_owned(), "no_target"),
        (r#"{"at":{"col":0,"row":99},"id":"p5","target":{"value_contains":"."},"ui":"click"}"#.to_owned(), "ambiguous_target"),
    ];
    let mut script = setup.clone();
    script.extend(refusals.iter().map(|(l, _)| l.clone()));
    script.push(snapshot("s1"));
    fixture(&dir);
    let sess = run_script(&dir, "s", &script);
    let r = sess.responses(script.len());
    let (s0, f0) = snap(&r[setup.len() - 1]);
    // The ambiguous target matches two nodes (the two documents).
    let dotted = all_nodes(s0)
        .iter()
        .filter(|n| value(n).is_some_and(|v| v.contains('.')))
        .count();
    assert!(
        dotted >= 2,
        "precondition: two nodes with a `.` in their value"
    );
    for ((line, kind), resp) in refusals.iter().zip(&r[setup.len()..]) {
        let id = parse_json(line).at("id").clone();
        assert_eq!(resp.get("id"), Some(&id), "id echoed for {line}");
        ui_refusal(resp, kind, line);
    }
    let (s1, f1) = snap(&r[script.len() - 1]);
    assert_eq!(s1, s0, "a refused UI request changed the snapshot");
    assert_eq!(f1, f0, "a refused UI request ran frames");
    assert_eq!(sess.log, base.log, "a refused UI request changed the log");
    assert!(!sess.log.is_empty(), "the setup's opens are logged");
    assert_eq!(sess.out.code, 3, "exit code with refusals");
    let _ = fs::remove_dir_all(&dir);
    click_offscreen_is_refused();
}

/// D-149 n8: a `click` whose target is `offscreen` is refused `not_actionable` and changes
/// nothing. 100 open documents in a 200×200 window overflow `Documents` (AC-2: one child per
/// open document, each showing its 20-char path), so some document node lies outside the
/// visible part; it is found by its `offscreen` state, not by position.
fn click_offscreen_is_refused() {
    let dir = case_dir("ac7-offscreen");
    let root = dir.join("proj");
    let paths: Vec<String> = (0..100)
        .map(|i| format!("docs/document-{i:03}.txt"))
        .collect();
    for p in &paths {
        write(&root.join(p), b"x\n");
    }
    let mut setup: Vec<String> = paths.iter().zip(1..).map(|(p, id)| open(id, p)).collect();
    setup.push(one_frame("frame"));
    setup.push(snapshot("b0"));
    let size = ["--root", "proj", "--size", "200x200"];
    let base = run_script_bytes(&dir, "base", &as_bytes(&setup), &size);
    let rb = base.responses(setup.len());
    let (b0, _) = snap(&rb[setup.len() - 1]);
    assert_eq!(
        documents(b0),
        paths,
        "precondition: Documents lists the 100 documents"
    );
    let nodes = all_nodes(b0);
    let target = nodes
        .iter()
        .filter(|n| n.get("offscreen") == Some(&J::Bool(true)))
        .filter_map(|n| value(n))
        .find(|v| nodes.iter().filter(|m| value(m) == Some(*v)).count() == 1)
        .unwrap_or_else(|| {
            panic!(
                "precondition: no offscreen node with a unique value among 100 open documents \
                 in a 200x200 window"
            )
        })
        .to_owned();
    let click = format!(
        r#"{{"id":"off","target":{{"value":{}}},"ui":"click"}}"#,
        q(&target)
    );
    let mut script = setup.clone();
    script.push(click.clone());
    script.push(snapshot("b1"));
    let sess = run_script_bytes(&dir, "s", &as_bytes(&script), &size);
    let r = sess.responses(script.len());
    let (s0, f0) = snap(&r[setup.len() - 1]);
    ui_refusal(&r[setup.len()], "not_actionable", &click);
    let (s1, f1) = snap(&r[setup.len() + 1]);
    assert_eq!(
        s1, s0,
        "a refused click on an offscreen node changed the snapshot"
    );
    assert_eq!(f1, f0, "a refused click on an offscreen node ran frames");
    assert_eq!(
        sess.log, base.log,
        "a refused click on an offscreen node changed the log"
    );
    assert_eq!(sess.out.code, 3, "exit code with the refusal");
    let _ = fs::remove_dir_all(&dir);
}

/// D-149 n4: a control request runs no frame and a snapshot runs none; what a control request
/// changed is shown from the next frame on (oracle: `doc.list` in the same session).
#[test]
fn ac6_control_effect_shown_from_the_next_frame() {
    let dir = case_dir("ac6-next-frame");
    fixture(&dir);
    let script = vec![
        snapshot("s0"),
        open(1, "src/a.rs"),
        snapshot("s1"),
        control(2, "doc.list", "{}"),
        one_frame("frame"),
        snapshot("s2"),
    ];
    let sess = run_script(&dir, "s", &script);
    let r = sess.responses(script.len());
    let (s0, f0) = snap(&r[0]);
    assert!(documents(s0).is_empty(), "no document in a fresh session");
    assert_eq!(r[1].get("ok"), Some(&J::Bool(true)), "{}", r[1].show());
    let (s1, f1) = snap(&r[2]);
    assert_eq!(f1, f0, "a control request or a snapshot ran a frame");
    assert_eq!(
        s1, s0,
        "the snapshot before any frame shows the control request's effect"
    );
    assert_eq!(doc_list(&r[3]), [(1, "src/a.rs".to_owned())], "doc.list");
    action_ok(&r[4], Some(1));
    let (s2, f2) = snap(&r[5]);
    assert_eq!(f2, f0 + 1, "frames after one frame");
    assert_eq!(
        documents(s2),
        ["src/a.rs"],
        "Documents after the next frame"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// Pinned (a): line-level errors come before anything else, as the control protocol's error
/// for the line (oracle: `Host`'s response to the same line, byte for byte).
#[test]
fn ac7_line_level_errors_first() {
    let dir = case_dir("ac7-lines");
    fixture(&dir);
    let bad: Vec<Vec<u8>> = vec![
        br#"{"bogus":1,"id":1.5,"ui":"dance"}"#.to_vec(),
        br#"{"id":null,"ui":"snapshot"}"#.to_vec(),
        br#"{"id":[1],"ui":"frames","count":0}"#.to_vec(),
        br#"["ui","snapshot"]"#.to_vec(),
        br#""ui""#.to_vec(),
        br#"{"ui":"snapshot""#.to_vec(),
        b"{\"ui\":\"snapshot\",\"x\":\"\xff\"}".to_vec(),
    ];
    let mut lines = vec![snapshot("s0").into_bytes()];
    lines.extend(bad.iter().cloned());
    lines.push(snapshot("s1").into_bytes());
    let sess = run_script_bytes(&dir, "s", &lines, &["--root", "proj"]);
    let r = sess.responses(lines.len());
    let mut host = Host::in_memory(&[]).expect("Host::in_memory");
    for (i, line) in bad.iter().enumerate() {
        let expected = host.handle_line(line).expect("a response");
        assert_eq!(
            sess.lines[i + 1],
            expected,
            "line {}: {}",
            i + 1,
            String::from_utf8_lossy(line)
        );
    }
    let (s0, f0) = snap(&r[0]);
    let (s1, f1) = snap(&r[lines.len() - 1]);
    assert_eq!((s1, f1), (s0, f0), "a line-level error changed the UI");
    assert_eq!(sess.out.code, 3, "exit code");
    let _ = fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------------------------
// AC-8: command line (oracle: exit codes and empty stdout).
// ---------------------------------------------------------------------------------------------

fn assert_exit(dir: &Path, args: &[&str], code: i32) -> Out {
    let out = kiln(dir, args);
    assert_eq!(
        out.code,
        code,
        "kiln {}: exit code (stderr: {})",
        args.join(" "),
        out.stderr_text()
    );
    out
}

fn assert_refused(dir: &Path, args: &[&str]) -> Out {
    let out = assert_exit(dir, args, 2);
    assert!(
        out.stdout.is_empty(),
        "kiln {}: stdout not empty",
        args.join(" ")
    );
    assert!(
        !out.stderr.is_empty(),
        "kiln {}: no message on stderr",
        args.join(" ")
    );
    out
}

#[test]
fn ac8_usage_errors() {
    let dir = case_dir("ac8-usage");
    fixture(&dir);
    write(
        &dir.join("s.jsonl"),
        b"{\"id\":1,\"command\":\"doc.list\"}\n",
    );
    let cases: [&[&str]; 18] = [
        &["script", "s.jsonl", "--root", "proj", "--bogus"],
        &["script", "--bogus", "s.jsonl", "--root", "proj"],
        &["script", "s.jsonl", "--root", "proj", "--memory"],
        &["script", "--memory", "s.jsonl", "--root", "proj"],
        &["script", "s.jsonl", "--root", "proj", "--size", "199x800"],
        &["script", "s.jsonl", "--root", "proj", "--size", "800x199"],
        &["script", "s.jsonl", "--root", "proj", "--size", "8193x800"],
        &["script", "s.jsonl", "--root", "proj", "--size", "800x8193"],
        &["script", "s.jsonl", "--root", "proj", "--size", "1280"],
        &["script", "s.jsonl", "--root", "proj", "--size", "wxh"],
        &[
            "script",
            "s.jsonl",
            "--root",
            "proj",
            "--size",
            "1280x800x2",
        ],
        &["script", "s.jsonl", "--root", "proj", "--size", "-1x800"],
        &[
            "script",
            "s.jsonl",
            "--root",
            "proj",
            "--size",
            "1280.5x800",
        ],
        &["script", "s.jsonl", "--root", "proj", "--size"],
        &["script"],
        &["script", "--root", "proj"],
        &["script", "s.jsonl", "--root", "proj", "--root", "proj"],
        &[
            "script", "s.jsonl", "--root", "proj", "--size", "300x300", "--size", "300x300",
        ],
    ];
    for args in cases {
        assert_refused(&dir, args);
    }
    assert_refused(
        &dir,
        &["script", "s.jsonl", "--log", "a.jsonl", "--log", "b.jsonl"],
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn ac8_unreadable_script_bad_root_unwritable_log() {
    let dir = case_dir("ac8-files");
    let root = fixture(&dir);
    write(
        &dir.join("s.jsonl"),
        b"{\"id\":1,\"command\":\"doc.open\",\"params\":{\"path\":\"b.txt\"}}\n\
          {\"id\":2,\"command\":\"doc.insert\",\"params\":{\"doc\":1,\"offset\":0,\"text\":\"X\"}}\n\
          {\"id\":3,\"command\":\"doc.save\",\"params\":{\"doc\":1}}\n",
    );
    let before = tree(&root);
    assert_refused(&dir, &["script", "no-such-script.jsonl", "--root", "proj"]);
    assert_refused(&dir, &["script", "proj", "--root", "proj"]);
    for bad in ["no-such-dir", "proj/b.txt"] {
        let out = assert_refused(&dir, &["script", "s.jsonl", "--root", bad]);
        assert!(
            out.stderr_text().contains(bad),
            "the bad root is named as given: {}",
            out.stderr_text()
        );
    }
    assert_refused(
        &dir,
        &[
            "script",
            "s.jsonl",
            "--root",
            "proj",
            "--log",
            "no-dir/l.jsonl",
        ],
    );
    assert_refused(
        &dir,
        &["script", "s.jsonl", "--root", "proj", "--log", "proj/src"],
    );
    assert_eq!(tree(&root), before, "a refused start ran a request");
    // The log is written after the last request: a log that cannot be written is exit 2 with
    // nothing on stdout, as for `kiln-cli run`.
    assert_refused(
        &dir,
        &["script", "s.jsonl", "--root", "proj", "--log", "/dev/full"],
    );
    let _ = fs::remove_dir_all(&dir);
}

/// Pinned (c): usage → script readable → root → log writable → requests. A refusal before the
/// log check leaves an existing log file as it was.
#[test]
fn ac8_order_of_checks_keeps_the_log() {
    let dir = case_dir("ac8-order");
    fixture(&dir);
    write(
        &dir.join("s.jsonl"),
        b"{\"id\":1,\"command\":\"doc.list\"}\n",
    );
    let keep = b"keep me\n";
    let cases: [&[&str]; 4] = [
        &[
            "script",
            "s.jsonl",
            "--root",
            "proj",
            "--size",
            "1x1",
            "--log",
            "old.jsonl",
        ],
        &[
            "script",
            "s.jsonl",
            "--root",
            "proj",
            "--memory",
            "--log",
            "old.jsonl",
        ],
        &[
            "script",
            "missing.jsonl",
            "--root",
            "proj",
            "--log",
            "old.jsonl",
        ],
        &[
            "script",
            "s.jsonl",
            "--root",
            "no-such-dir",
            "--log",
            "old.jsonl",
        ],
    ];
    for args in cases {
        write(&dir.join("old.jsonl"), keep);
        assert_refused(&dir, args);
        assert_eq!(
            fs::read(dir.join("old.jsonl")).expect("old log"),
            keep,
            "kiln {}: touched the log",
            args.join(" ")
        );
    }
    // And a good start truncates it.
    let out = assert_exit(
        &dir,
        &["script", "s.jsonl", "--root", "proj", "--log", "old.jsonl"],
        0,
    );
    assert!(!out.stdout.is_empty());
    let log = fs::read_to_string(dir.join("old.jsonl")).expect("log");
    assert_eq!(
        logged_commands(&log),
        ["doc.list"],
        "log after a good run: {log}"
    );
    let _ = fs::remove_dir_all(&dir);
}

fn logged_commands(log: &str) -> Vec<String> {
    log_entries(log)
        .iter()
        .map(|e| e.at("command").as_str().expect("command").to_owned())
        .collect()
}

#[test]
fn ac8_help() {
    let dir = case_dir("ac8-help");
    for args in [&["--help"][..], &["-h"], &["script", "--help"]] {
        let out = assert_exit(&dir, args, 0);
        assert!(
            !out.stdout.is_empty(),
            "kiln {}: no usage on stdout",
            args.join(" ")
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// Options before or after the positional argument; `--size` bounds 200 and 8192 accepted; the
/// default root is the current directory; `--memory` runs.
#[test]
fn ac8_accepted_command_lines() {
    let dir = case_dir("ac8-ok");
    fixture(&dir);
    write(
        &dir.join("s.jsonl"),
        b"{\"id\":1,\"command\":\"doc.open\",\"params\":{\"path\":\"b.txt\"}}\n",
    );
    let reference = assert_exit(&dir, &["script", "s.jsonl", "--root", "proj"], 0);
    let line = String::from_utf8(reference.stdout.clone()).expect("utf-8");
    assert_eq!(
        parse_json(line.trim_end()).get("ok"),
        Some(&J::Bool(true)),
        "{line}"
    );
    let same: [&[&str]; 4] = [
        &["script", "--root", "proj", "s.jsonl"],
        &["script", "--size", "200x200", "s.jsonl", "--root", "proj"],
        &["script", "s.jsonl", "--size", "8192x8192", "--root", "proj"],
        &[
            "script", "--log", "l.jsonl", "s.jsonl", "--size", "1280x800", "--root", "proj",
        ],
    ];
    for args in same {
        let out = assert_exit(&dir, args, 0);
        assert_eq!(out.stdout, reference.stdout, "kiln {}", args.join(" "));
    }
    let in_root = assert_exit(&dir.join("proj"), &["script", "../s.jsonl"], 0);
    assert_eq!(
        in_root.stdout, reference.stdout,
        "default root: the current directory"
    );
    let mem = assert_exit(&dir, &["script", "s.jsonl", "--memory"], 3);
    assert_eq!(
        String::from_utf8_lossy(&mem.stdout).lines().count(),
        1,
        "--memory"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn ac8_stdout_write_failure_exits_1() {
    let dir = case_dir("ac8-full");
    fixture(&dir);
    write(
        &dir.join("s.jsonl"),
        b"{\"id\":1,\"command\":\"doc.list\"}\n",
    );
    // The same run succeeds when stdout can be written, so the 1 below is the write failure.
    let ok = assert_exit(&dir, &["script", "s.jsonl", "--root", "proj"], 0);
    assert!(!ok.stdout.is_empty(), "the response line");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kiln"));
    cmd.args(["script", "s.jsonl", "--root", "proj"])
        .current_dir(&dir);
    let out = run_cmd(cmd, "kiln script > /dev/full", true);
    assert_eq!(
        out.code,
        1,
        "stdout on /dev/full (stderr: {})",
        out.stderr_text()
    );
    let _ = fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------------------------
// AC-9: no display (oracle: equality).
// ---------------------------------------------------------------------------------------------

#[test]
fn ac9_no_display() {
    let dir = case_dir("ac9");
    fixture(&dir);
    let script = ac2_script();
    write(&dir.join("s.jsonl"), &script_bytes(&as_bytes(&script)));
    let home = dir.join("home");
    fs::create_dir_all(home.join(".config")).expect("home");
    let run = |display: bool| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kiln"));
        cmd.args(["script", "s.jsonl", "--root", "proj"])
            .current_dir(&dir)
            .env("HOME", &home)
            .env("XDG_CONFIG_HOME", home.join(".config"));
        if display {
            cmd.env("DISPLAY", ":4242")
                .env("WAYLAND_DISPLAY", "kiln-t055-no-such-wayland");
        } else {
            cmd.env_remove("DISPLAY").env_remove("WAYLAND_DISPLAY");
        }
        run_cmd(cmd, "kiln script (display variables)", false)
    };
    let without = run(false);
    let sess = session_from(without, &dir.join("none.log"));
    check_ac2(&sess.responses(script.len()));
    assert_eq!(sess.out.code, 0, "exit code without a display");
    let with = run(true);
    assert_eq!(
        with.stdout, sess.out.stdout,
        "stdout with unusable display variables"
    );
    assert_eq!(
        with.code, sess.out.code,
        "exit code with unusable display variables"
    );
    // Pinned (b): nothing is written to the user's home or configuration directory.
    assert_eq!(
        tree(&home),
        BTreeMap::from([(".config".to_owned(), b"<dir>".to_vec())]),
        "the home directory changed"
    );
    let _ = fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------------------------
// AC-10: accessible names (oracle: tree walk; also checked on every snapshot above by `snap`).
// ---------------------------------------------------------------------------------------------

#[test]
fn ac10_accessible_names() {
    let dir = case_dir("ac10");
    fixture(&dir);
    let script = vec![
        snapshot("fresh"),
        open(1, "src/a.rs"),
        open(2, "notes/c.md"),
        one_frame("frame"),
        snapshot("two-docs"),
        focus_path(),
        type_text("type", "u.txt"),
        snapshot("typed"),
        click_open(),
        snapshot("opened"),
        type_text("type2", "/missing"),
        click_open(),
        snapshot("refused"),
    ];
    let sess = run_script(&dir, "s", &script);
    let r = sess.responses(script.len());
    for (line, resp) in script.iter().zip(&r) {
        if line.contains("\"ui\":\"snapshot\"") {
            let (root, _) = snap(resp);
            let nodes = all_nodes(root);
            for wanted in ["button", "text_input"] {
                assert!(
                    nodes.iter().any(|n| role(n) == Some(wanted)),
                    "{line}: no {wanted} node to check"
                );
            }
        }
    }
    let _ = fs::remove_dir_all(&dir);
}
