//! T-059 acceptance tests: the status bar of `kiln script` (`tasks/T-059-ui-status-bar.md`,
//! Interface, Pinned, AC-1 to AC-5). Written by the implementer before the implementation, from
//! the spec alone (medium tier, odd task number: the implementer-first arm, METHODOLOGY §6), on
//! `task/T-059-tests`.
//!
//! Every test drives the built `kiln` binary (`CARGO_BIN_EXE_kiln`) through `kiln script` with
//! temporary roots under `CARGO_TARGET_TMPDIR`; every spawned process is killed after
//! [`TIMEOUT`] (D-028 (8)). The bar's four nodes are found by role and label: role `label`,
//! labels `Line`, `Column`, `Encoding`, `Line ending` (Interface). Their values are checked
//! against an independent source in the same session: the control reads `cursor.get` and
//! `doc.info` (and `doc.open`'s answer), the session log, the published `mutating` flags of
//! `docs/commands.json`, and `kiln_control::Host` (the one Rust oracle the spec allows).
//!
//! Every test that types or sends a key gives `Editor` focus first (`focus` or a `click` on it;
//! T-082: a tab click leaves the keyboard nowhere). Logs are compared only for frames run after
//! the last control request, or with the same control reads at the end of both sessions (a
//! script's read-only control request is a change, T-058 ruling h3).
//!
//! Not pinned (Pinned (f)): a whole snapshot, the bar's position, separators or decoration,
//! which reads the editor sends, how many reads a change causes, frame counts beyond the
//! 60-frame settle cap, message text beyond its leading kind, wall time.
//!
//! No Unix-only API (Platforms): no symbolic link, FIFO, mode bit or device.
#![cfg(test)]

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use kiln_control::Host;
use serde_json::{Value, json};

/// Longest any spawned process may run (a debug build of the toolkit, on a loaded machine).
const TIMEOUT: Duration = Duration::from_secs(300);

/// The name of the session root in every case directory.
const ROOT: &str = "t059root";

/// Frames run after a control request so that the views show its effect (the effect is shown
/// from the next frame on; a margin, not a pinned count).
const SETTLE: u64 = 10;

/// The longest settling the spec allows for an action (L8, `docs/protocol/ui.md`).
const SETTLE_CAP: u64 = 60;

/// The bar's four labels (Interface).
const BAR: [&str; 4] = ["Line", "Column", "Encoding", "Line ending"];

// ---------------------------------------------------------------------------------------------
// Processes, directories, files.
// ---------------------------------------------------------------------------------------------

/// What a finished process left.
struct Out {
    code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
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

/// Run `cmd` under the hang detector, stdin closed, stdout and stderr captured.
fn run_cmd(mut cmd: Command, what: &str) -> Out {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap_or_else(|e| panic!("spawn {what}: {e}"));
    let mut out = child.stdout.take().expect("stdout pipe");
    let mut err = child.stderr.take().expect("stderr pipe");
    let out_reader = thread::spawn(move || {
        let mut b = Vec::new();
        let _ = out.read_to_end(&mut b);
        b
    });
    let err_reader = thread::spawn(move || {
        let mut b = Vec::new();
        let _ = err.read_to_end(&mut b);
        b
    });
    let status = wait_with_timeout(&mut child, what);
    let stdout = out_reader.join().expect("stdout reader");
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

/// `kiln <args>` in `cwd`.
fn kiln(cwd: &Path, args: &[&str]) -> Out {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kiln"));
    cmd.args(args).current_dir(cwd);
    run_cmd(cmd, &format!("kiln {}", args.join(" ")))
}

/// A fresh, empty directory for one test case.
fn case_dir(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("t059-acceptance")
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

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create copy dir");
    for e in fs::read_dir(from).expect("read_dir") {
        let e = e.expect("dir entry");
        let src = e.path();
        let dst = to.join(e.file_name());
        if src.is_dir() {
            copy_dir(&src, &dst);
        } else {
            fs::copy(&src, &dst).expect("copy file");
        }
    }
}

/// `text` as the bytes of a file in `encoding` (a `doc.info` encoding name; UTF-16 with its
/// byte-order mark, as `doc.open` detects it).
fn encode(text: &str, encoding: &str) -> Vec<u8> {
    match encoding {
        "utf-8" => text.as_bytes().to_vec(),
        "utf-8-bom" => {
            let mut b = vec![0xEF, 0xBB, 0xBF];
            b.extend_from_slice(text.as_bytes());
            b
        }
        "utf-16le" => {
            let mut b = vec![0xFF, 0xFE];
            for u in text.encode_utf16() {
                b.extend_from_slice(&u.to_le_bytes());
            }
            b
        }
        "utf-16be" => {
            let mut b = vec![0xFE, 0xFF];
            for u in text.encode_utf16() {
                b.extend_from_slice(&u.to_be_bytes());
            }
            b
        }
        other => panic!("unknown encoding {other} (a test bug)"),
    }
}

// ---------------------------------------------------------------------------------------------
// Script lines.
// ---------------------------------------------------------------------------------------------

/// A UI request `{"id":id,"ui":action,…fields}`.
fn ui(id: &str, action: &str, fields: Value) -> String {
    let mut m = match fields {
        Value::Object(m) => m,
        Value::Null => serde_json::Map::new(),
        other => panic!("ui fields must be an object: {other}"),
    };
    m.insert("id".into(), json!(id));
    m.insert("ui".into(), json!(action));
    Value::Object(m).to_string()
}

/// A control request with a string id.
fn ctl(id: &str, command: &str, params: Value) -> String {
    json!({"id": id, "command": command, "params": params}).to_string()
}

fn snapshot(id: &str) -> String {
    ui(id, "snapshot", Value::Null)
}

fn frames(id: &str, count: u64) -> String {
    ui(id, "frames", json!({"count": count}))
}

fn click(id: &str, target: &Value) -> String {
    ui(id, "click", json!({ "target": target }))
}

fn focus(id: &str, target: &Value) -> String {
    ui(id, "focus", json!({ "target": target }))
}

fn key(id: &str, name: &str, mods: &[&str]) -> String {
    if mods.is_empty() {
        ui(id, "key", json!({"key": name}))
    } else {
        ui(id, "key", json!({"key": name, "modifiers": mods}))
    }
}

fn type_text(id: &str, text: &str) -> String {
    ui(id, "type", json!({"text": text}))
}

fn open(id: &str, path: &str) -> String {
    ctl(id, "doc.open", json!({"path": path}))
}

fn cursor_get(id: &str, doc: u64) -> String {
    ctl(id, "cursor.get", json!({ "doc": doc }))
}

fn doc_info(id: &str, doc: u64) -> String {
    ctl(id, "doc.info", json!({ "doc": doc }))
}

fn editor_target() -> Value {
    json!({"label": "Editor"})
}

fn tab_target(path: &str) -> Value {
    json!({"role": "tab", "label": path})
}

fn close_target(path: &str) -> Value {
    json!({"role": "button", "label": format!("Close {path}")})
}

/// A `Documents` entry (`docs/protocol/ui-tabs.md`: role `label`, the path as `value`).
fn entry_target(path: &str) -> Value {
    json!({"role": "label", "value": path})
}

/// A node of the bar (Interface: role `label`, its accessible name).
fn bar_target(name: &str) -> Value {
    json!({"role": "label", "label": name})
}

// ---------------------------------------------------------------------------------------------
// Sessions.
// ---------------------------------------------------------------------------------------------

/// One `kiln script` run.
struct Sess {
    name: String,
    script: Vec<String>,
    resp: Vec<Value>,
    by_id: HashMap<String, usize>,
    log: String,
}

impl Sess {
    fn r(&self, id: &str) -> &Value {
        let i = self
            .by_id
            .get(id)
            .unwrap_or_else(|| panic!("{}: no request with id `{id}`", self.name));
        &self.resp[*i]
    }

    /// `result` of a request that must have succeeded.
    fn ok(&self, id: &str) -> &Value {
        let r = self.r(id);
        assert_eq!(
            r["ok"],
            json!(true),
            "{}: request `{id}` refused: {}",
            self.name,
            short(r)
        );
        &r["result"]
    }

    /// An action that must have succeeded and settled within the cap.
    fn settled(&self, id: &str) {
        let r = self.ok(id);
        assert_eq!(
            r["settled"],
            json!(true),
            "{}: action `{id}` did not settle within {SETTLE_CAP} frames: {}",
            self.name,
            short(r)
        );
    }

    /// The root of a snapshot that must have succeeded.
    fn snap(&self, id: &str) -> &Value {
        &self.ok(id)["root"]
    }

    /// The refusal kind of a request that must have been refused.
    fn refused_kind(&self, id: &str) -> String {
        let r = self.r(id);
        assert_eq!(
            r["ok"],
            json!(false),
            "{}: request `{id}` was expected to be refused: {}",
            self.name,
            short(r)
        );
        r["error"]["kind"].as_str().unwrap_or_default().to_owned()
    }

    /// Every UI request answered `ok:true`, and every action except `frames` settled.
    fn all_ui_ok(&self) {
        for (line, resp) in self.script.iter().zip(&self.resp) {
            let v: Value = serde_json::from_str(line).expect("script line");
            if v.get("ui").is_some() {
                assert_eq!(
                    resp["ok"],
                    json!(true),
                    "{}: UI request refused: {line} → {}",
                    self.name,
                    short(resp)
                );
                if v["ui"] != json!("snapshot") && v["ui"] != json!("frames") {
                    assert_eq!(
                        resp["result"]["settled"],
                        json!(true),
                        "{}: the action did not settle within {SETTLE_CAP} frames: {line} → {}",
                        self.name,
                        short(resp)
                    );
                }
            }
        }
    }

    fn log_entries(&self) -> Vec<Value> {
        self.log
            .lines()
            .map(|l| serde_json::from_str(l).expect("log line is JSON"))
            .collect()
    }
}

/// Run `kiln script <name>.jsonl <args…> --log <name>.log.jsonl` in `dir`. Panics unless
/// there is exactly one response line per script line, each with a unique id.
fn run(dir: &Path, name: &str, script: &[String], args: &[&str]) -> Sess {
    let file = format!("{name}.jsonl");
    let log = format!("{name}.log.jsonl");
    let mut bytes = Vec::new();
    for l in script {
        bytes.extend_from_slice(l.as_bytes());
        bytes.push(b'\n');
    }
    write(&dir.join(&file), &bytes);
    let _ = fs::remove_file(dir.join(&log));
    let mut all = vec!["script", file.as_str()];
    all.extend_from_slice(args);
    all.extend_from_slice(&["--log", log.as_str()]);
    let out = kiln(dir, &all);
    let stdout = String::from_utf8(out.stdout.clone()).expect("stdout is UTF-8");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        script.len(),
        "{name}: `kiln {}` printed {} response lines for {} request lines (exit {}, stderr: {})",
        all.join(" "),
        lines.len(),
        script.len(),
        out.code,
        String::from_utf8_lossy(&out.stderr)
    );
    let resp: Vec<Value> = lines
        .iter()
        .map(|l| serde_json::from_str(l).expect("response is JSON"))
        .collect();
    let mut by_id = HashMap::new();
    for (i, r) in resp.iter().enumerate() {
        if let Some(id) = r["id"].as_str() {
            assert!(
                by_id.insert(id.to_owned(), i).is_none(),
                "{name}: duplicate id `{id}` (a test bug)"
            );
        }
    }
    let log = fs::read_to_string(dir.join(&log)).unwrap_or_default();
    Sess {
        name: name.to_owned(),
        script: script.to_vec(),
        resp,
        by_id,
        log,
    }
}

fn root_args() -> [&'static str; 2] {
    ["--root", ROOT]
}

// ---------------------------------------------------------------------------------------------
// Snapshot nodes.
// ---------------------------------------------------------------------------------------------

fn children(n: &Value) -> &[Value] {
    n["children"].as_array().map_or(&[][..], Vec::as_slice)
}

/// `n` and its descendants in depth-first pre-order (the snapshot's order).
fn preorder(n: &Value) -> Vec<&Value> {
    let mut out = vec![n];
    for c in children(n) {
        out.extend(preorder(c));
    }
    out
}

fn role(n: &Value) -> Option<&str> {
    n["role"].as_str()
}

fn label(n: &Value) -> Option<&str> {
    n["label"].as_str()
}

fn value(n: &Value) -> Option<&str> {
    n["value"].as_str()
}

fn by_label<'a>(root: &'a Value, l: &str) -> Vec<&'a Value> {
    preorder(root)
        .into_iter()
        .filter(|n| label(n) == Some(l))
        .collect()
}

fn short(v: &Value) -> String {
    let s = v.to_string();
    if s.chars().count() > 1500 {
        let cut: String = s.chars().take(1500).collect();
        format!("{cut}…")
    } else {
        s
    }
}

/// Each painted row's text (its runs joined).
fn row_texts(n: &Value) -> Vec<String> {
    n["rows"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    row.as_array()
                        .expect("row is an array")
                        .iter()
                        .map(|run| run["text"].as_str().expect("run text"))
                        .collect::<String>()
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The `Message` node's value.
fn message(root: &Value) -> String {
    let found = by_label(root, "Message");
    assert_eq!(found.len(), 1, "one node labelled Message: {}", short(root));
    value(found[0]).unwrap_or_default().to_owned()
}

/// The one node labelled `Editor`.
fn editor<'a>(root: &'a Value, what: &str) -> &'a Value {
    let found = by_label(root, "Editor");
    assert_eq!(
        found.len(),
        1,
        "{what}: expected exactly one node labelled `Editor`, found {}: {}",
        found.len(),
        short(root)
    );
    found[0]
}

/// What a snapshot's node list says without layout: `(role, label, value)` of every node in
/// pre-order.
fn values_of(root: &Value) -> Vec<(Option<String>, Option<String>, Option<String>)> {
    preorder(root)
        .into_iter()
        .map(|n| {
            (
                role(n).map(str::to_owned),
                label(n).map(str::to_owned),
                value(n).map(str::to_owned),
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// The bar as the snapshot shows it.
// ---------------------------------------------------------------------------------------------

/// The four values of the bar.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Bar {
    line: String,
    column: String,
    encoding: String,
    ending: String,
}

/// The bar of a snapshot at the default size: each of the four labels names exactly one node,
/// of role `label`, with a value, not focused, that paints exactly its value on one row (F2;
/// at the default size with few documents the bar is on screen, Interface "Measured").
fn bar(root: &Value, what: &str) -> Bar {
    let mut vals = Vec::new();
    for name in BAR {
        let found = by_label(root, name);
        assert_eq!(
            found.len(),
            1,
            "{what}: expected exactly one node labelled `{name}` (the status bar), found {}: {}",
            found.len(),
            short(root)
        );
        let n = found[0];
        assert_eq!(
            role(n),
            Some("label"),
            "{what}: `{name}` is not of role label: {}",
            short(n)
        );
        let v = value(n)
            .unwrap_or_else(|| panic!("{what}: `{name}` has no value: {}", short(n)))
            .to_owned();
        assert_ne!(
            n.get("focused"),
            Some(&json!(true)),
            "{what}: `{name}` is focused (L4)"
        );
        assert_eq!(
            row_texts(n),
            vec![v.clone()],
            "{what}: `{name}` does not paint exactly its value on one row (F2): {}",
            short(n)
        );
        vals.push(v);
    }
    Bar {
        line: vals[0].clone(),
        column: vals[1].clone(),
        encoding: vals[2].clone(),
        ending: vals[3].clone(),
    }
}

/// None of the four labels names any node.
fn no_bar(root: &Value, what: &str) {
    for name in BAR {
        let found = by_label(root, name);
        assert!(
            found.is_empty(),
            "{what}: a node labelled `{name}` exists with no document open: {}",
            short(found[0])
        );
    }
}

/// `Line`'s value for a `cursor.get` result: `Ln ` and the line plus one.
fn want_line(cursor: &Value) -> String {
    format!(
        "Ln {}",
        cursor["line"].as_u64().expect("cursor.get line") + 1
    )
}

/// `Column`'s value for a `cursor.get` result: `Col ` and the column plus one.
fn want_column(cursor: &Value) -> String {
    format!(
        "Col {}",
        cursor["col"].as_u64().expect("cursor.get col") + 1
    )
}

/// `Line ending`'s value for a `doc.info` (or `doc.open`) result.
fn want_ending(info: &Value) -> String {
    let le = info["line_ending"].as_str().expect("line_ending");
    if info["mixed_line_endings"] == json!(true) {
        format!("{le} (mixed)")
    } else {
        le.to_owned()
    }
}

/// The bar a document's `cursor.get` and `doc.info` results call for.
fn want_bar(cursor: &Value, info: &Value) -> Bar {
    Bar {
        line: want_line(cursor),
        column: want_column(cursor),
        encoding: info["encoding"].as_str().expect("encoding").to_owned(),
        ending: want_ending(info),
    }
}

// ---------------------------------------------------------------------------------------------
// The registry and the log.
// ---------------------------------------------------------------------------------------------

fn params_of(e: &Value) -> Value {
    e.get("params").cloned().unwrap_or_else(|| json!({}))
}

/// `mutating` of every command, from `docs/commands.json`.
fn mutating_flags() -> HashMap<String, bool> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/commands.json");
    let v: Value = serde_json::from_str(&fs::read_to_string(&p).expect("read docs/commands.json"))
        .expect("commands.json is JSON");
    v["commands"]
        .as_array()
        .expect("commands")
        .iter()
        .map(|c| {
            (
                c["name"].as_str().expect("name").to_owned(),
                c["mutating"].as_bool().expect("mutating"),
            )
        })
        .collect()
}

/// Every entry of `entries` is a read-only command (`mutating: false` in `commands.json`).
fn all_read_only(entries: &[Value], what: &str) {
    let flags = mutating_flags();
    for e in entries {
        let c = e["command"].as_str().unwrap_or_default();
        let m = *flags
            .get(c)
            .unwrap_or_else(|| panic!("{what}: logged command `{c}` is not in commands.json"));
        assert!(!m, "{what}: the frames logged the mutating command {e}");
    }
}

/// The index of the one log entry equal to `command` with `params`, after `from`.
fn log_index(log: &[Value], from: usize, command: &str, params: &Value, what: &str) -> usize {
    log.iter()
        .enumerate()
        .skip(from)
        .find(|(_, e)| e["command"] == json!(command) && params_of(e) == *params)
        .map(|(i, _)| i)
        .unwrap_or_else(|| panic!("{what}: `{command}` {params} is not in the log after {from}"))
}

// ---------------------------------------------------------------------------------------------
// A deterministic generator (xorshift64*).
// ---------------------------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % (n as u64)).expect("fits")
    }

    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

// =============================================================================================
// AC-1: encoding and line ending of each kind (oracle: doc.info in the same session).
// =============================================================================================

const ENCODINGS: [&str; 4] = ["utf-8", "utf-8-bom", "utf-16le", "utf-16be"];
const ENDINGS: [&str; 5] = ["lf", "crlf", "cr", "none", "mixed"];

/// Words of the generated lines (ASCII, an accented letter, CJK; never empty, so that a CR
/// break never meets a following LF).
const WORDS: [&str; 6] = ["ab", "x", "Kiln", "\u{e9}t\u{e9}", "\u{6f22}", "q 9"];

/// A text of the line-ending kind `kind`: non-empty lines joined by breaks of that kind (one to
/// four breaks; none for `none`; for `mixed`, two to five breaks with at least one LF and one
/// CRLF, the rest random, in a random order). After the last break comes a line or nothing.
fn gen_text(rng: &mut Rng, kind: &str) -> String {
    let mut breaks: Vec<&str> = match kind {
        "lf" => vec!["\n"; 1 + rng.below(4)],
        "crlf" => vec!["\r\n"; 1 + rng.below(4)],
        "cr" => vec!["\r"; 1 + rng.below(4)],
        "none" => vec![],
        "mixed" => {
            let mut b = vec!["\n", "\r\n"];
            for _ in 0..rng.below(4) {
                b.push(*rng.pick(&["\n", "\r\n", "\r"]));
            }
            for i in (1..b.len()).rev() {
                let j = rng.below(i + 1);
                b.swap(i, j);
            }
            b
        }
        other => panic!("unknown kind {other}"),
    };
    let line = |rng: &mut Rng| {
        let mut s = String::new();
        for w in 0..1 + rng.below(3) {
            if w > 0 {
                s.push(' ');
            }
            s.push_str(rng.pick(&WORDS));
        }
        s
    };
    let mut s = line(rng);
    let last = breaks.len();
    for (k, b) in breaks.drain(..).enumerate() {
        s.push_str(b);
        if k + 1 < last || rng.below(2) == 0 {
            s.push_str(&line(rng));
        }
    }
    s
}

/// For seeds 1 and 2: one document per encoding × line-ending kind, opened one after another by
/// control requests in a random order (a mixed one last); after each open the bar shows that
/// document's `doc.info` encoding and line ending (with ` (mixed)` exactly when mixed), and its
/// caret. Then two control `eol.normalize`s on the current document: the bar follows `doc.info`.
#[test]
fn ac1_encoding_and_line_ending_follow_doc_info() {
    for seed in [1_u64, 2] {
        let dir = case_dir(&format!("ac1-{seed}"));
        let root = dir.join(ROOT);
        let mut rng = Rng::new(seed);
        let mut docs = Vec::new();
        for enc in ENCODINGS {
            for kind in ENDINGS {
                let path = format!("{enc}-{kind}.txt");
                let text = gen_text(&mut rng, kind);
                write(&root.join(&path), &encode(&text, enc));
                docs.push((path, enc, kind, text));
            }
        }
        for i in (1..docs.len()).rev() {
            let j = rng.below(i + 1);
            docs.swap(i, j);
        }
        let last_mixed = docs
            .iter()
            .rposition(|d| d.2 == "mixed")
            .expect("a mixed document");
        let n = docs.len();
        docs.swap(last_mixed, n - 1);

        let mut script = Vec::new();
        for (k, (path, ..)) in docs.iter().enumerate() {
            let id = k as u64 + 1;
            script.push(open(&format!("o{k}"), path));
            script.push(frames(&format!("f{k}"), SETTLE));
            script.push(snapshot(&format!("s{k}")));
            script.push(doc_info(&format!("i{k}"), id));
            script.push(cursor_get(&format!("c{k}"), id));
        }
        let cur = n as u64;
        // Normalise to the dominant ending (no longer mixed), then to another kind.
        let to = ["cr", "crlf", "lf"];
        script.push(ctl("n1", "eol.normalize", json!({"doc": cur})));
        script.push(frames("fn1", SETTLE));
        script.push(snapshot("sn1"));
        script.push(doc_info("in1", cur));
        script.push(cursor_get("cn1", cur));
        let target = to[rng.below(3)];
        script.push(ctl(
            "n2",
            "eol.normalize",
            json!({"doc": cur, "to": target}),
        ));
        script.push(frames("fn2", SETTLE));
        script.push(snapshot("sn2"));
        script.push(doc_info("in2", cur));
        script.push(cursor_get("cn2", cur));

        let s = run(&dir, "s", &script, &root_args());
        for (k, (path, enc, kind, text)) in docs.iter().enumerate() {
            let what = format!("seed {seed}, after opening {path} ({enc}, {kind}, text {text:?})");
            let opened = s.ok(&format!("o{k}"));
            let info = s.ok(&format!("i{k}"));
            assert_eq!(
                info["encoding"],
                json!(enc),
                "{what}: precondition, doc.info encoding"
            );
            assert_eq!(
                info["mixed_line_endings"],
                json!(*kind == "mixed"),
                "{what}: precondition, doc.info mixed_line_endings"
            );
            assert_eq!(
                (
                    opened["line_ending"].clone(),
                    opened["mixed_line_endings"].clone()
                ),
                (
                    info["line_ending"].clone(),
                    info["mixed_line_endings"].clone()
                ),
                "{what}: precondition, doc.open and doc.info agree"
            );
            let shown = bar(s.snap(&format!("s{k}")), &what);
            assert_eq!(
                shown,
                want_bar(s.ok(&format!("c{k}")), info),
                "{what}: the bar against doc.info and cursor.get"
            );
        }
        for (n_id, what) in [
            ("n1", "eol.normalize (dominant)"),
            ("n2", "eol.normalize to"),
        ] {
            let normalized = s.ok(n_id);
            let info = s.ok(&format!("i{n_id}"));
            let what = format!("seed {seed}, after {what} on doc {cur}");
            assert_eq!(
                normalized["mixed_line_endings"],
                json!(false),
                "{what}: precondition"
            );
            let shown = bar(s.snap(&format!("s{n_id}")), &what);
            assert_eq!(
                shown,
                want_bar(s.ok(&format!("c{n_id}")), info),
                "{what}: the bar against doc.info"
            );
            assert_eq!(shown.ending, want_ending(normalized), "{what}");
        }
        assert_eq!(
            s.ok("in2")["line_ending"],
            json!(target),
            "seed {seed}: precondition, normalised to {target}"
        );
        s.all_ui_ok();
        let _ = fs::remove_dir_all(&dir);
    }
}

// =============================================================================================
// AC-2: line and column after random caret steps (oracle: cursor.get in the same session).
// =============================================================================================

/// AC-2's alphabet, as the spec states it: ASCII letters and a digit, a space, a tab, a
/// precomposed `é`, `e`+U+0301, a zero-width joiner, `ffi`, `漢字`, `😀`, `שלום`, `سلام`, an
/// Arabic-Indic digit `٣`; documents also hold LF, CRLF and CR breaks.
const ALPHABET: [&str; 15] = [
    "a",
    "Zq",
    "7",
    " ",
    "\t",
    "\u{e9}",
    "e\u{301}",
    "\u{200d}",
    "ffi",
    "\u{6f22}\u{5b57}",
    "\u{1f600}",
    "\u{5e9}\u{5dc}\u{5d5}\u{5dd}",
    "\u{633}\u{644}\u{627}\u{645}",
    "\u{663}",
    "b",
];

/// The breaks of AC-2's documents.
const BREAKS: [&str; 3] = ["\n", "\r\n", "\r"];

/// How the alphabet is described in a failure message.
const ALPHABET_TEXT: &str =
    "ASCII, space, tab, é, e+U+0301, U+200D, ffi, 漢字, 😀, שלום, سلام, ٣; LF, CRLF, CR";

/// Seeds and steps of AC-2 (at least 40 each, D-174 change 5).
const SEEDS: u64 = 40;
const STEPS: usize = 40;

/// A generated document: two to six lines of zero to five pieces, joined by random breaks.
fn gen_doc(rng: &mut Rng) -> String {
    let lines = 2 + rng.below(5);
    let mut s = String::new();
    for k in 0..lines {
        for _ in 0..rng.below(6) {
            s.push_str(rng.pick(&ALPHABET));
        }
        if k + 1 < lines {
            s.push_str(rng.pick(&BREAKS));
        }
    }
    s
}

/// One caret step of AC-2.
#[derive(Clone, Debug)]
enum Step {
    Key(&'static str, Vec<&'static str>),
    Type(String),
    /// A click on `Editor` at a random row and column: the raw draws, reduced to the rows the
    /// snapshot before the step shows.
    Click(usize, usize),
}

fn gen_steps(rng: &mut Rng) -> Vec<Step> {
    const MODS: [&[&str]; 4] = [&[], &["ctrl"], &["shift"], &["ctrl", "shift"]];
    let mut out = Vec::new();
    for _ in 0..STEPS {
        let step = match rng.below(10) {
            0..=2 => {
                let k = *rng.pick(&["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"]);
                Step::Key(k, rng.pick(&MODS).to_vec())
            }
            3 => {
                let k = *rng.pick(&["Home", "End"]);
                let m: &[&str] = if rng.below(2) == 0 { &[] } else { &["shift"] };
                Step::Key(k, m.to_vec())
            }
            4 | 5 => Step::Click(rng.below(1000), rng.below(1000)),
            6 | 7 => {
                let mut t = String::new();
                for _ in 0..1 + rng.below(2) {
                    t.push_str(rng.pick(&ALPHABET));
                }
                Step::Type(t)
            }
            8 => Step::Key("Enter", vec![]),
            _ => Step::Key("Backspace", vec![]),
        };
        out.push(step);
    }
    out
}

/// The lines of a text split at LF, CRLF and CR, in chars.
fn lines_of(text: &str) -> Vec<usize> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut len = 0;
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\r' => {
                out.push(len);
                len = 0;
                if chars.get(i + 1) == Some(&'\n') {
                    i += 1;
                }
            }
            '\n' => {
                out.push(len);
                len = 0;
            }
            _ => len += 1,
        }
        i += 1;
    }
    out.push(len);
    out
}

/// The script of one AC-2 session: open, focus `Editor`, then per step its action, a snapshot
/// `s<k>` and `cursor.get` `c<k>`. `at` holds each click's chosen `(row, col)` (`None`: a click
/// without `at`).
fn ac2_script(steps: &[Step], at: &HashMap<usize, Option<(usize, usize)>>) -> Vec<String> {
    let mut script = vec![
        open("o", "g.txt"),
        frames("f", SETTLE),
        focus("fe", &editor_target()),
        snapshot("s-init"),
    ];
    for (k, step) in steps.iter().enumerate() {
        let id = format!("a{k}");
        script.push(match step {
            Step::Key(name, mods) => key(&id, name, mods),
            Step::Type(t) => type_text(&id, t),
            Step::Click(..) => match at.get(&k).copied().flatten() {
                Some((r, c)) => ui(
                    &id,
                    "click",
                    json!({"target": editor_target(), "at": {"row": r, "col": c}}),
                ),
                None => click(&id, &editor_target()),
            },
        });
        script.push(snapshot(&format!("s{k}")));
        script.push(cursor_get(&format!("c{k}"), 1));
    }
    script
}

/// One seed of AC-2: runs the session (again while a click's `at` was beyond the rows shown
/// before it, choosing from those rows), then checks the bar after every step.
fn ac2_seed(seed: u64) {
    let dir = case_dir(&format!("ac2-{seed}"));
    let mut rng = Rng::new(seed);
    let text = gen_doc(&mut rng);
    write(&dir.join(ROOT).join("g.txt"), text.as_bytes());
    let steps = gen_steps(&mut rng);
    let initial = lines_of(&text);
    // First guess: reduce each click's draws to the initial document's lines.
    let mut at: HashMap<usize, Option<(usize, usize)>> = HashMap::new();
    for (k, step) in steps.iter().enumerate() {
        if let Step::Click(r, c) = step {
            let row = r % initial.len();
            at.insert(k, Some((row, c % (initial[row] + 1))));
        }
    }
    let mut runs = 0;
    let s = loop {
        runs += 1;
        assert!(runs <= 60, "seed {seed}: too many runs (a test bug)");
        let script = ac2_script(&steps, &at);
        let s = run(&dir, "s", &script, &root_args());
        let mut fix = None;
        for (k, step) in steps.iter().enumerate() {
            let Step::Click(r, c) = step else { continue };
            let resp = s.r(&format!("a{k}"));
            if resp["ok"] == json!(true) {
                continue;
            }
            assert_eq!(
                resp["error"]["kind"],
                json!("not_actionable"),
                "seed {seed}, step {k}: the click was refused: {}",
                short(resp)
            );
            let before = if k == 0 {
                "s-init".to_owned()
            } else {
                format!("s{}", k - 1)
            };
            let rows = row_texts(editor(s.snap(&before), &format!("seed {seed}, {before}")));
            let choice = if rows.is_empty() {
                None
            } else {
                let row = r % rows.len();
                Some((row, c % (rows[row].chars().count() + 1)))
            };
            assert_ne!(
                at.get(&k).copied().flatten(),
                choice,
                "seed {seed}, step {k}: a click within the rows shown was refused: {}",
                short(resp)
            );
            fix = Some((k, choice));
            break;
        }
        match fix {
            Some((k, choice)) => {
                at.insert(k, choice);
            }
            None => break s,
        }
    };
    let script = ac2_script(&steps, &at);
    s.ok("fe");
    for (k, step) in steps.iter().enumerate() {
        let line = &script[4 + 3 * k];
        let what = format!(
            "seed {seed}, step {k} ({step:?}, request {line}); document {text:?}; alphabet: {ALPHABET_TEXT}"
        );
        let resp = s.r(&format!("a{k}"));
        assert_eq!(resp["ok"], json!(true), "{what}: refused: {}", short(resp));
        assert_eq!(
            resp["result"]["settled"],
            json!(true),
            "{what}: did not settle within {SETTLE_CAP} frames"
        );
        let cursor = s.ok(&format!("c{k}"));
        let root = s.snap(&format!("s{k}"));
        let shown = bar(root, &what);
        assert_eq!(
            (shown.line.as_str(), shown.column.as_str()),
            (want_line(cursor).as_str(), want_column(cursor).as_str()),
            "{what}: Line and Column against cursor.get {cursor}"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// For 40 seeds, a generated document and 40 random caret steps through the UI (arrow keys with
/// and without `ctrl` and `shift`, `Home`, `End`, clicks `at` a random shown row and column,
/// typed text, `Enter`, `Backspace`); after every step `Line` is `Ln ` + (`line` + 1) and
/// `Column` is `Col ` + (`col` + 1) of `cursor.get`. A failure names the seed, the step and the
/// alphabet. The seeds run on a few threads (each its own session).
#[test]
fn ac2_line_and_column_follow_cursor_get_over_generated_steps() {
    let workers = 3;
    let failures: Vec<String> = thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|w| {
                scope.spawn(move || {
                    let mut failed = Vec::new();
                    for seed in (1..=SEEDS).filter(|s| s % workers == w) {
                        let r = std::panic::catch_unwind(|| ac2_seed(seed));
                        if let Err(e) = r {
                            let msg = e
                                .downcast_ref::<String>()
                                .cloned()
                                .or_else(|| e.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                                .unwrap_or_default();
                            failed.push(format!("seed {seed}: {msg}"));
                        }
                    }
                    failed
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("worker"))
            .collect()
    });
    assert!(
        failures.is_empty(),
        "{} of {SEEDS} seeds failed (alphabet: {ALPHABET_TEXT}):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

// =============================================================================================
// AC-3: switching documents (oracle: cursor.get and doc.info of the document; the current rule).
// =============================================================================================

/// Three documents with different encodings, line endings and carets (the caret a selection
/// whose head is not its anchor in `a.txt`).
fn ac3_root(dir: &Path) {
    let r = dir.join(ROOT);
    write(&r.join("a.txt"), &encode("abc\r\ndef\r\nghi\r\n", "utf-8"));
    write(&r.join("b.txt"), &encode("xy\nz\nwvu\n", "utf-16le"));
    write(&r.join("c.txt"), &encode("p\rqq\rrrr", "utf-8-bom"));
}

/// A snapshot `s<k>` followed by the oracle reads `g<k>`/`i<k>` of `doc`.
fn ac3_check(k: usize, doc: u64) -> Vec<String> {
    vec![
        snapshot(&format!("s{k}")),
        cursor_get(&format!("g{k}"), doc),
        doc_info(&format!("i{k}"), doc),
    ]
}

/// Switching by a tab click, by a `Documents` click and by a control `doc.open`
/// (`already_open`) shows the newly current document's values; a refused close of the dirty
/// current document leaves the four values unchanged; closing the current clean document shows
/// the open document with the highest id.
#[test]
fn ac3_switching_shows_the_current_documents_values() {
    let dir = case_dir("ac3");
    ac3_root(&dir);
    let mut script = vec![
        open("o1", "a.txt"),
        open("o2", "b.txt"),
        open("o3", "c.txt"),
        ctl(
            "k1",
            "cursor.set",
            json!({"doc": 1, "anchor": 0, "head": 7}),
        ),
        ctl("k2", "cursor.set", json!({"doc": 2, "anchor": 1})),
        ctl("k3", "cursor.set", json!({"doc": 3, "anchor": 8})),
        frames("f0", SETTLE),
    ];
    // (k, doc expected current) after each action.
    let mut expect: Vec<(usize, u64, &str)> = Vec::new();
    script.extend(ac3_check(0, 3));
    expect.push((0, 3, "after the opens (c.txt current)"));
    script.push(click("t1", &tab_target("a.txt")));
    script.extend(ac3_check(1, 1));
    expect.push((1, 1, "after a click on the tab of a.txt"));
    script.push(click("e2", &entry_target("b.txt")));
    script.extend(ac3_check(2, 2));
    expect.push((2, 2, "after a click on the Documents entry of b.txt"));
    script.push(open("o4", "a.txt"));
    script.push(frames("f4", SETTLE));
    script.extend(ac3_check(3, 1));
    expect.push((3, 1, "after a control doc.open of a.txt (already_open)"));
    script.push(ctl(
        "ins",
        "doc.insert",
        json!({"doc": 1, "offset": 15, "text": "!"}),
    ));
    script.push(frames("f5", SETTLE));
    script.extend(ac3_check(4, 1));
    expect.push((4, 1, "after a control edit of a.txt after its caret"));
    script.push(click("x5", &close_target("a.txt")));
    script.extend(ac3_check(5, 1));
    expect.push((5, 1, "after the refused close of a.txt"));
    script.push(click("t6", &tab_target("b.txt")));
    script.extend(ac3_check(6, 2));
    expect.push((6, 2, "after a click on the tab of b.txt"));
    script.push(click("x7", &close_target("b.txt")));
    script.extend(ac3_check(7, 3));
    expect.push((
        7,
        3,
        "after closing b.txt (current, clean): c.txt has the highest id",
    ));
    script.push(ctl("l", "doc.list", json!({})));

    let s = run(&dir, "s", &script, &root_args());
    s.all_ui_ok();
    assert_eq!(s.ok("o4")["already_open"], json!(true), "precondition");
    assert_eq!(s.ok("o4")["doc"], json!(1), "precondition");
    let mut shown = Vec::new();
    for (k, doc, what) in &expect {
        let cursor = s.ok(&format!("g{k}"));
        let info = s.ok(&format!("i{k}"));
        let b = bar(s.snap(&format!("s{k}")), what);
        assert_eq!(
            b,
            want_bar(cursor, info),
            "{what}: the bar against cursor.get and doc.info of doc {doc}"
        );
        shown.push(b);
    }
    // The three documents' values differ in each of the four fields, so a stale bar fails.
    for (i, j) in [(0, 1), (1, 2), (0, 2)] {
        let (x, y) = (&shown[i], &shown[j]);
        assert!(
            x.line != y.line || x.column != y.column,
            "precondition: carets differ"
        );
        assert_ne!(x.encoding, y.encoding, "precondition: encodings differ");
        assert_ne!(x.ending, y.ending, "precondition: line endings differ");
    }
    // The refused close.
    assert!(
        message(s.snap("s5")).starts_with("unsaved_changes"),
        "precondition: the close of the dirty a.txt was refused: {}",
        message(s.snap("s5"))
    );
    assert_eq!(shown[5], shown[4], "the refused close changed the bar");
    let docs: Vec<u64> = s.ok("l")["docs"]
        .as_array()
        .expect("docs")
        .iter()
        .map(|d| d["doc"].as_u64().expect("doc"))
        .collect();
    assert_eq!(
        docs,
        [1, 3],
        "precondition: b.txt was closed, a.txt was not"
    );
    let _ = fs::remove_dir_all(&dir);
}

// =============================================================================================
// AC-4: reads and idle frames (oracle: log comparison, Host::log_script, cursor.get).
// =============================================================================================

fn ac4_root(dir: &Path) -> PathBuf {
    let r = dir.join(ROOT);
    write(
        &r.join("a.txt"),
        &encode("one\r\ntwo\r\nthree\r\n", "utf-8"),
    );
    write(&r.join("b.txt"), &encode("x\ny\r\nz", "utf-16be"));
    r
}

/// (a) A script of control requests only answers as `Host` and writes `Host::log_script()`; the
/// same requests and frames show the bar (so the panel is there and the equality not vacuous).
#[test]
fn ac4a_control_requests_only_log_as_host() {
    let dir = case_dir("ac4a");
    let r = ac4_root(&dir);
    let oracle = dir.join("oracle");
    copy_dir(&r, &oracle);
    let controls = vec![
        open("1", "a.txt"),
        open("2", "b.txt"),
        ctl("3", "cursor.set", json!({"doc": 2, "anchor": 5})),
        ctl("4", "eol.normalize", json!({"doc": 1, "to": "lf"})),
        ctl("5", "doc.list", json!({})),
        cursor_get("6", 2),
        doc_info("7", 2),
    ];
    let mut host = Host::on_disk(&oracle).expect("Host::on_disk");
    let expected: Vec<String> = controls
        .iter()
        .map(|l| host.handle_line(l.as_bytes()).expect("response"))
        .collect();
    let s = run(&dir, "s", &controls, &root_args());
    for (i, e) in expected.iter().enumerate() {
        let want: Value = serde_json::from_str(e).expect("JSON");
        assert_eq!(s.resp[i], want, "response {i}");
    }
    assert_eq!(s.log, host.log_script(), "--log vs Host::log_script()");

    let mut framed = controls.clone();
    framed.push(frames("f", SETTLE));
    framed.push(snapshot("s"));
    let t = run(&dir, "framed", &framed, &root_args());
    assert_eq!(
        bar(t.snap("s"), "the control requests and frames"),
        want_bar(t.ok("6"), t.ok("7")),
        "the bar of the current document b.txt"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// (b) With a document open, 600 frames after UI actions (typing, keys, a tab and a `Documents`
/// click) and after the last control request add nothing to the log.
#[test]
fn ac4b_idle_frames_add_nothing() {
    let dir = case_dir("ac4b");
    ac4_root(&dir);
    let prefix = vec![
        open("o1", "a.txt"),
        open("o2", "b.txt"),
        frames("f0", SETTLE),
        focus("fe", &editor_target()),
        type_text("ty", "q"),
        key("k1", "ArrowDown", &[]),
        click("t1", &tab_target("a.txt")),
        click("ce", &editor_target()),
        key("k2", "End", &[]),
        click("e2", &entry_target("b.txt")),
    ];
    let tail = |v: &mut Vec<String>| {
        v.push(snapshot("s"));
        v.push(cursor_get("g", 2));
        v.push(doc_info("i", 2));
    };
    let mut with = prefix.clone();
    with.push(frames("idle", 600));
    tail(&mut with);
    let mut without = prefix;
    tail(&mut without);
    let a = run(&dir, "with", &with, &root_args());
    let b = run(&dir, "without", &without, &root_args());
    b.all_ui_ok();
    a.all_ui_ok();
    assert_eq!(
        bar(b.snap("s"), "before the idle frames"),
        want_bar(b.ok("g"), b.ok("i")),
        "the bar of b.txt"
    );
    assert_eq!(
        bar(a.snap("s"), "after the idle frames"),
        want_bar(a.ok("g"), a.ok("i")),
        "the bar of b.txt after 600 frames"
    );
    assert_eq!(a.log, b.log, "600 idle frames added to the log");
    let _ = fs::remove_dir_all(&dir);
}

/// (c) After a control `cursor.set` on the current document (a caret, then a selection whose
/// head is not its anchor), the frames until the UI settles add read-only requests only, and
/// `Line` and `Column` follow the new head; 600 further frames add nothing.
#[test]
fn ac4c_a_control_cursor_set_is_read_then_idle() {
    let dir = case_dir("ac4c");
    ac4_root(&dir);
    let prefix = vec![
        open("o1", "a.txt"),
        frames("f0", SETTLE),
        snapshot("s0"),
        ctl("k1", "cursor.set", json!({"doc": 1, "anchor": 7})),
        frames("f1", SETTLE_CAP),
        snapshot("s1"),
        ctl(
            "k2",
            "cursor.set",
            json!({"doc": 1, "anchor": 1, "head": 13}),
        ),
        frames("f2", SETTLE_CAP),
        snapshot("s2"),
    ];
    let mut with = prefix.clone();
    with.push(frames("idle", 600));
    with.push(cursor_get("g", 1));
    let mut without = prefix;
    without.push(cursor_get("g", 1));
    let a = run(&dir, "with", &with, &root_args());
    let b = run(&dir, "without", &without, &root_args());
    let first = bar(b.snap("s0"), "before the cursor.set");
    assert_eq!(
        (first.line.as_str(), first.column.as_str()),
        ("Ln 1", "Col 1")
    );
    for (k, snap) in [("k1", "s1"), ("k2", "s2")] {
        let set = b.ok(k);
        let shown = bar(b.snap(snap), &format!("after {k}"));
        assert_eq!(
            (shown.line.clone(), shown.column.clone()),
            (want_line(set), want_column(set)),
            "after {k}: Line and Column against the head {set}"
        );
    }
    let last = b.ok("g");
    assert_eq!(last["head"], json!(13), "precondition: the head");
    assert_eq!(last["anchor"], json!(1), "precondition: the anchor");
    // The log between each cursor.set and the next script request: read-only requests only,
    // among them a cursor.get of the document.
    let log = b.log_entries();
    let i1 = log_index(&log, 0, "cursor.set", &json!({"doc": 1, "anchor": 7}), "k1");
    let i2 = log_index(
        &log,
        i1 + 1,
        "cursor.set",
        &json!({"doc": 1, "anchor": 1, "head": 13}),
        "k2",
    );
    assert_eq!(
        log.len() - 1,
        log_index(&log, i2 + 1, "cursor.get", &json!({"doc": 1}), "g")
    );
    for (from, to, what) in [(i1, i2, "after k1"), (i2, log.len() - 1, "after k2")] {
        let window = &log[from + 1..to];
        all_read_only(window, what);
        assert!(
            window
                .iter()
                .any(|e| e["command"] == json!("cursor.get") && params_of(e) == json!({"doc": 1})),
            "{what}: the frames read no cursor.get of doc 1: {window:?}"
        );
    }
    assert_eq!(
        a.log, b.log,
        "600 frames after the settling added to the log"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// (d) After a script's read-only control request (`doc.list`), the frames that follow add
/// read-only requests only, and 600 further frames add nothing; the bar still shows the
/// current document.
#[test]
fn ac4d_a_read_only_control_request_then_idle() {
    let dir = case_dir("ac4d");
    ac4_root(&dir);
    let prefix = vec![
        open("o1", "b.txt"),
        frames("f0", SETTLE),
        focus("fe", &editor_target()),
        type_text("ty", "w"),
        key("k", "ArrowRight", &["shift"]),
        ctl("l", "doc.list", json!({})),
        frames("f1", SETTLE),
        snapshot("s"),
    ];
    let mut with = prefix.clone();
    with.push(frames("idle", 600));
    with.push(cursor_get("g", 1));
    with.push(doc_info("i", 1));
    let mut without = prefix;
    without.push(cursor_get("g", 1));
    without.push(doc_info("i", 1));
    let a = run(&dir, "with", &with, &root_args());
    let b = run(&dir, "without", &without, &root_args());
    b.all_ui_ok();
    assert_eq!(
        bar(b.snap("s"), "after doc.list"),
        want_bar(b.ok("g"), b.ok("i")),
        "the bar of b.txt"
    );
    let log = b.log_entries();
    let at = log_index(&log, 0, "doc.list", &json!({}), "the script's doc.list");
    all_read_only(&log[at + 1..], "after the script's doc.list");
    assert_eq!(
        a.log, b.log,
        "600 frames after the settling added to the log"
    );
    let _ = fs::remove_dir_all(&dir);
}

// =============================================================================================
// AC-5: no document, display only (oracle: snapshot, log).
// =============================================================================================

/// A fresh session has none of the four nodes; with a document open it has them; after the last
/// document is closed (from its tab, or by control `doc.close`s) it has none again.
#[test]
fn ac5_no_document_no_bar() {
    let dir = case_dir("ac5a");
    ac4_root(&dir);
    let script = vec![
        snapshot("s0"),
        frames("f0", SETTLE),
        snapshot("s1"),
        open("o1", "a.txt"),
        frames("f1", SETTLE),
        snapshot("s2"),
        click("x1", &close_target("a.txt")),
        snapshot("s3"),
        open("o2", "a.txt"),
        open("o3", "b.txt"),
        frames("f2", SETTLE),
        snapshot("s4"),
        ctl("c2", "doc.close", json!({"doc": 3})),
        frames("f3", SETTLE),
        snapshot("s5"),
        ctl("c1", "doc.close", json!({"doc": 2})),
        frames("f4", SETTLE),
        snapshot("s6"),
        cursor_get("g", 2),
    ];
    let s = run(&dir, "s", &script, &root_args());
    s.all_ui_ok();
    no_bar(s.snap("s0"), "a fresh session");
    no_bar(s.snap("s1"), "a fresh session after frames");
    bar(s.snap("s2"), "with a.txt open");
    no_bar(s.snap("s3"), "after closing a.txt from its tab");
    bar(s.snap("s4"), "with two documents open");
    bar(s.snap("s5"), "with one document open");
    no_bar(s.snap("s6"), "after closing the last document by doc.close");
    assert_eq!(s.refused_kind("g"), "no_such_document", "precondition");
    let _ = fs::remove_dir_all(&dir);
}

/// A click on each of the bar's nodes (primary, secondary, with `at`, with a modifier) changes
/// no node's value, adds nothing to the log and settles; the bar takes no focus.
#[test]
fn ac5_a_click_on_the_bar_does_nothing() {
    let dir = case_dir("ac5b");
    ac4_root(&dir);
    let prefix = vec![
        open("o1", "a.txt"),
        open("o2", "b.txt"),
        frames("f0", SETTLE),
        focus("fe", &editor_target()),
        type_text("ty", "k"),
        key("kr", "ArrowRight", &["shift"]),
        snapshot("pre"),
    ];
    let mut with = prefix.clone();
    with.push(click("c1", &bar_target("Line")));
    with.push(snapshot("p1"));
    with.push(ui(
        "c2",
        "click",
        json!({"target": bar_target("Column"), "button": "secondary"}),
    ));
    with.push(ui(
        "c3",
        "click",
        json!({"target": bar_target("Encoding"), "at": {"row": 0, "col": 0}}),
    ));
    with.push(ui(
        "c4",
        "click",
        json!({"target": bar_target("Line ending"), "modifiers": ["shift"]}),
    ));
    with.push(focus("fl", &bar_target("Line")));
    with.push(snapshot("p2"));
    let mut without = prefix;
    without.push(snapshot("p2"));
    let a = run(&dir, "with", &with, &root_args());
    let b = run(&dir, "without", &without, &root_args());
    let pre = a.snap("pre");
    let before = bar(pre, "before the clicks");
    for id in ["c1", "c2", "c3", "c4"] {
        a.settled(id);
    }
    assert_eq!(
        a.refused_kind("fl"),
        "not_actionable",
        "focus on Line: the bar takes no focus"
    );
    for snap in ["p1", "p2"] {
        let root = a.snap(snap);
        assert_eq!(bar(root, snap), before, "{snap}: the bar changed");
        assert_eq!(
            values_of(root),
            values_of(pre),
            "{snap}: a click on the bar changed a node's value"
        );
        for name in BAR {
            for n in by_label(root, name) {
                assert_ne!(
                    n.get("focused"),
                    Some(&json!(true)),
                    "{snap}: {name} focused"
                );
            }
        }
    }
    assert_eq!(a.log, b.log, "the clicks on the bar added to the log");
    let _ = fs::remove_dir_all(&dir);
}
