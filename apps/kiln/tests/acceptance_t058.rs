//! T-058 acceptance tests: the tabs of `kiln script` (`tasks/T-058-ui-tabs.md`, Interface,
//! Pinned, AC-1 to AC-9). Written by the independent test author from the spec alone (even
//! task number, METHODOLOGY §6), on `task/T-058-tests`, before any implementation.
//!
//! Every test drives the built `kiln` binary (`CARGO_BIN_EXE_kiln`) through `kiln script` with
//! temporary roots under `CARGO_TARGET_TMPDIR`; every spawned process is killed after
//! [`TIMEOUT`] (D-028 (8)). The tests act with UI requests (`click`, `focus`, `type`, `key`,
//! `scroll`, `frames`), find nodes by role and label (or value): the tab list `Tabs` (role
//! `tab_list`), tabs (role `tab`, labelled with their path), `Close <path>` and `Discard <path>`
//! (role `button`), the `modified` mark (role `label`, value `modified`), `Documents` entries
//! (role `label`, value = path), `Title`, `Editor`, `Message`, `Path`, `Open`. They check what
//! is shown against an independent source: control requests in the same session (`doc.list`,
//! `doc.text`, `cursor.get`), the session log, the published `mutating` flags of
//! `docs/commands.json`, the files on disk, a naive model of the current-document rule and of
//! T3/T4 (AC-8), and `kiln_control::Host` (the one Rust API the spec allows, as oracle).
//!
//! Not pinned (golden rule 9, Pinned (f)): a whole snapshot, positions, the strip's height or
//! wrapping, frame counts beyond the 60-frame settle cap, message text beyond its leading kind,
//! which reads the editor sends, how many `doc.list` reads a change causes, the command list,
//! `error_kinds`, wall time. A click on an `offscreen` node is preceded by `scroll` requests on
//! that node (the scrollable area around it), decided from a run of the script so far (a script
//! is read whole at start, so a test cannot react within one run).
//!
//! No Unix-only API (Platforms): no symbolic link, FIFO, mode bit or device; generated names
//! follow T-057 AC-3's alphabet without a line break.
#![cfg(test)]

use std::collections::{BTreeMap, BTreeSet, HashMap};
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
const ROOT: &str = "t058root";

/// Frames run after a control request so that the views show its effect (the effect is shown
/// from the next frame on; a margin, not a pinned count).
const SETTLE: u64 = 10;

/// The longest settling the spec allows for an action (L8, `docs/protocol/ui.md`).
const SETTLE_CAP: u64 = 60;

const A_RS: &str = "fn main() {\n    let x = 1;\n}\n";

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
        .join("t058-acceptance")
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

/// Every entry under `root` by `/`-separated relative path: file bytes or `<dir>`.
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
            if p.is_dir() {
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

fn focus(id: &str, target: Value) -> String {
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

fn scroll(id: &str, target: &Value, direction: &str, steps: u64) -> String {
    ui(
        id,
        "scroll",
        json!({"target": target, "direction": direction, "steps": steps}),
    )
}

fn open(id: &str, path: &str) -> String {
    ctl(id, "doc.open", json!({"path": path}))
}

fn tabs_target() -> Value {
    json!({"role": "tab_list", "label": "Tabs"})
}

fn tab_target(path: &str) -> Value {
    json!({"role": "tab", "label": path})
}

fn close_target(path: &str) -> Value {
    json!({"role": "button", "label": format!("Close {path}")})
}

fn discard_target(path: &str) -> Value {
    json!({"role": "button", "label": format!("Discard {path}")})
}

/// A `Documents` entry (Interface: role `label`, the path as `value`).
fn entry_target(path: &str) -> Value {
    json!({"role": "label", "value": path})
}

fn editor_target() -> Value {
    json!({"label": "Editor"})
}

/// The lines of an open through `Path` and `Open`: focus `Path`, delete what it holds (`prev`:
/// one `Backspace` and one `Delete` per char, wherever the toolkit left the caret; surplus keys
/// on an empty side do nothing), type `path`, click `Open`.
fn ui_open_lines(id: &str, prev: &str, path: &str) -> Vec<String> {
    let mut out = vec![focus(&format!("{id}-fp"), json!({"label": "Path"}))];
    for k in 0..prev.chars().count() {
        out.push(key(&format!("{id}-bs{k}"), "Backspace", &[]));
        out.push(key(&format!("{id}-del{k}"), "Delete", &[]));
    }
    out.push(type_text(&format!("{id}-tp"), path));
    out.push(click(&format!("{id}-open"), &json!({"label": "Open"})));
    out
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

    /// `result` of a control request that must have succeeded.
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

    /// An action that must have succeeded and settled within the cap; its `result`.
    fn act(&self, id: &str) -> &Value {
        let r = self.r(id);
        assert_eq!(
            r["ok"],
            json!(true),
            "{}: UI action `{id}` refused: {}",
            self.name,
            short(r)
        );
        let n = r["result"]["frames"].as_u64().expect("frames");
        assert!(n >= 1, "{}: action `{id}` ran no frame", self.name);
        &r["result"]
    }

    /// The root of a snapshot that must have succeeded.
    fn snap(&self, id: &str) -> &Value {
        let r = self.r(id);
        assert_eq!(
            r["ok"],
            json!(true),
            "{}: snapshot `{id}` refused: {}",
            self.name,
            short(r)
        );
        &r["result"]["root"]
    }

    fn text(&self, id: &str) -> String {
        self.ok(id)["text"]
            .as_str()
            .expect("doc.text text")
            .to_owned()
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

    /// Every UI request of the script answered `ok:true`, and every action except `frames`
    /// settled.
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

    /// Every UI request of one step (id `step` or beginning `step-`) answered `ok:true`.
    fn step_ok(&self, step: &str, what: &str) {
        let dash = format!("{step}-");
        for (line, resp) in self.script.iter().zip(&self.resp) {
            let v: Value = serde_json::from_str(line).expect("script line");
            let id = v["id"].as_str().unwrap_or_default();
            if v.get("ui").is_some() && (id == step || id.starts_with(&dash)) {
                assert_eq!(
                    resp["ok"],
                    json!(true),
                    "{what}: UI request refused: {line} → {}",
                    short(resp)
                );
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
// A script with clicks that scroll first when their target is `offscreen`.
// ---------------------------------------------------------------------------------------------

/// A line of a [`Plan`]: a plain line, or a click whose target may need scrolling first.
enum Line {
    Raw(String),
    Click { id: String, target: Value },
}

/// A script whose clicks are preceded by a snapshot `pre-<id>` and by the scroll requests found
/// necessary in earlier runs: [`Plan::run`] runs it, and while a click was refused
/// `not_actionable` with its target `offscreen` in `pre-<id>`, adds one scroll step on the
/// target (down when a visible node of its area comes before it, else up; sideways when a
/// vertical step changed nothing) and runs again. The scrolls are not steps of the test.
struct Plan {
    dir: PathBuf,
    name: String,
    args: Vec<String>,
    lines: Vec<Line>,
    scrolls: HashMap<String, Vec<(String, Vec<bool>)>>,
}

impl Plan {
    fn new(dir: &Path, name: &str, args: &[&str]) -> Self {
        Plan {
            dir: dir.to_path_buf(),
            name: name.to_owned(),
            args: args.iter().map(|a| (*a).to_owned()).collect(),
            lines: Vec::new(),
            scrolls: HashMap::new(),
        }
    }

    fn raw(&mut self, line: String) {
        self.lines.push(Line::Raw(line));
    }

    fn raws(&mut self, lines: Vec<String>) {
        for l in lines {
            self.raw(l);
        }
    }

    fn click(&mut self, id: &str, target: Value) {
        self.lines.push(Line::Click {
            id: id.to_owned(),
            target,
        });
    }

    fn script(&self) -> Vec<String> {
        let mut out = Vec::new();
        for l in &self.lines {
            match l {
                Line::Raw(s) => out.push(s.clone()),
                Line::Click { id, target } => {
                    for (k, (dir, _)) in self.scrolls.get(id).into_iter().flatten().enumerate() {
                        out.push(scroll(&format!("{id}-scroll{k}"), target, dir, 1));
                    }
                    out.push(snapshot(&format!("pre-{id}")));
                    out.push(click(id, target));
                }
            }
        }
        out
    }

    fn run(&mut self) -> Sess {
        let args: Vec<String> = self.args.clone();
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        for _attempt in 0..200 {
            let s = run(&self.dir, &self.name, &self.script(), &args);
            let mut fix: Option<(String, String, Vec<bool>)> = None;
            for l in &self.lines {
                let Line::Click { id, target } = l else {
                    continue;
                };
                let r = s.r(id);
                if r["ok"] == json!(true) {
                    continue;
                }
                if r["error"]["kind"] != json!("not_actionable") {
                    break;
                }
                let pre = s.snap(&format!("pre-{id}"));
                let tried = self.scrolls.get(id).cloned().unwrap_or_default();
                fix = scroll_towards(pre, target, &tried).map(|(dir, sig)| (id.clone(), dir, sig));
                break;
            }
            match fix {
                None => return s,
                Some((id, dir, sig)) => self.scrolls.entry(id).or_default().push((dir, sig)),
            }
        }
        panic!("{}: too many scroll steps needed (a test bug)", self.name);
    }
}

/// Whether `n` matches a UI target (every given field equal).
fn matches(n: &Value, target: &Value) -> bool {
    ["role", "label", "value"]
        .iter()
        .all(|k| match target.get(*k) {
            Some(want) => n.get(*k) == Some(want),
            None => true,
        })
}

/// The scroll step that brings `target` towards view, with the area's visibility signature
/// before it; `None` if the target is not one `offscreen` node inside a `scroll_view`.
fn scroll_towards(
    root: &Value,
    target: &Value,
    tried: &[(String, Vec<bool>)],
) -> Option<(String, Vec<bool>)> {
    let found: Vec<&Value> = preorder(root)
        .into_iter()
        .filter(|n| matches(n, target))
        .collect();
    if found.len() != 1 || !offscreen(found[0]) {
        return None;
    }
    let sv = scroll_view_around(root, found[0])?;
    let nodes = preorder(sv);
    let sig: Vec<bool> = nodes.iter().map(|n| offscreen(n)).collect();
    let at = nodes.iter().position(|n| std::ptr::eq(*n, found[0]))?;
    let first_visible = nodes
        .iter()
        .skip(1)
        .position(|n| !offscreen(n))
        .map(|i| i + 1);
    let forward = first_visible.is_some_and(|v| v < at);
    let stuck_vertical = tried
        .last()
        .is_some_and(|(d, before)| (d == "down" || d == "up") && *before == sig);
    let stuck_sideways = tried
        .last()
        .is_some_and(|(d, before)| (d == "right" || d == "left") && *before == sig);
    assert!(
        !stuck_sideways,
        "scrolling changed nothing around the offscreen target {target} (a test assumption)"
    );
    let dir = match (stuck_vertical, forward) {
        (false, true) => "down",
        (false, false) => "up",
        (true, true) => "right",
        (true, false) => "left",
    };
    Some((dir.to_owned(), sig))
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

fn toggled(n: &Value) -> Option<bool> {
    n.get("toggled")
        .map(|t| t.as_bool().expect("toggled is a boolean"))
}

fn offscreen(n: &Value) -> bool {
    n.get("offscreen") == Some(&json!(true))
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

/// The chain of ancestors of `target` (by identity) under `root`, `root` first, `target` last.
fn chain_to<'a>(root: &'a Value, target: &Value) -> Vec<&'a Value> {
    fn path_to<'a>(n: &'a Value, target: &Value, out: &mut Vec<&'a Value>) -> bool {
        out.push(n);
        if std::ptr::eq(n, target) {
            return true;
        }
        for c in children(n) {
            if path_to(c, target, out) {
                return true;
            }
        }
        out.pop();
        false
    }
    let mut chain = Vec::new();
    assert!(path_to(root, target, &mut chain), "the node is in the tree");
    chain
}

/// The nearest ancestor of role `scroll_view` of `target`.
fn scroll_view_around<'a>(root: &'a Value, target: &Value) -> Option<&'a Value> {
    let mut chain = chain_to(root, target);
    chain.pop();
    chain
        .into_iter()
        .rev()
        .find(|n| role(n) == Some("scroll_view"))
}

/// A name whose rows `ui.md` (Limits) does not guarantee: it begins with right-to-left text or
/// an Arabic-Indic digit (AC-8), or it holds a line break.
fn rows_unspecified(path: &str) -> bool {
    path.contains('\n')
        || path
            .chars()
            .next()
            .is_some_and(|c| ('\u{0590}'..='\u{08ff}').contains(&c))
}

/// The `Message` node's value.
fn message(root: &Value) -> String {
    let found = by_label(root, "Message");
    assert_eq!(found.len(), 1, "one node labelled Message: {}", short(root));
    value(found[0]).unwrap_or_default().to_owned()
}

/// The value of the node labelled `Path`.
fn path_value(root: &Value) -> String {
    let found = by_label(root, "Path");
    let input: Vec<&&Value> = found
        .iter()
        .filter(|n| role(n) == Some("text_input"))
        .collect();
    assert_eq!(input.len(), 1, "one text input labelled Path");
    value(input[0]).unwrap_or_default().to_owned()
}

/// The `Title` node's value, if there is one (at most one).
fn title(root: &Value, what: &str) -> Option<String> {
    let found = by_label(root, "Title");
    assert!(
        found.len() <= 1,
        "{what}: {} nodes labelled `Title`",
        found.len()
    );
    found
        .first()
        .map(|n| value(n).unwrap_or_default().to_owned())
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

fn editor_value<'a>(root: &'a Value, what: &str) -> &'a str {
    let ed = editor(root, what);
    value(ed).unwrap_or_else(|| panic!("{what}: Editor without a value: {}", short(ed)))
}

/// `selection` of `Editor` as `(anchor, head)`.
fn editor_sel(root: &Value, what: &str) -> (u64, u64) {
    let ed = editor(root, what);
    let s = &ed["selection"];
    assert!(
        s.is_object(),
        "{what}: Editor without a selection: {}",
        short(ed)
    );
    (
        s["anchor"].as_u64().expect("anchor"),
        s["head"].as_u64().expect("head"),
    )
}

/// The open path `Title` begins with: the longest of `paths` that is a prefix of its value.
fn title_path(title: &str, paths: &[String]) -> Option<String> {
    paths
        .iter()
        .filter(|p| title.starts_with(p.as_str()))
        .max_by_key(|p| p.len())
        .cloned()
}

/// The current document as the UI shows it: `None` when there is neither `Title` nor `Editor`;
/// else the open path `Title` begins with (exactly one `Title` and one `Editor`).
fn shown_current(root: &Value, paths: &[String], what: &str) -> Option<String> {
    match title(root, what) {
        None => {
            assert!(
                by_label(root, "Editor").is_empty(),
                "{what}: an Editor without a Title: {}",
                short(root)
            );
            None
        }
        Some(t) => {
            editor(root, what);
            Some(title_path(&t, paths).unwrap_or_else(|| {
                panic!("{what}: Title `{t}` begins with no open path of {paths:?}")
            }))
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The tabs as the snapshot shows them.
// ---------------------------------------------------------------------------------------------

/// The node of role `tab_list` labelled `Tabs`, if shown (at most one).
fn tab_list<'a>(root: &'a Value, what: &str) -> Option<&'a Value> {
    let found: Vec<&Value> = preorder(root)
        .into_iter()
        .filter(|n| role(n) == Some("tab_list") && label(n) == Some("Tabs"))
        .collect();
    assert!(
        found.len() <= 1,
        "{what}: {} nodes of role `tab_list` labelled `Tabs`",
        found.len()
    );
    found.first().copied()
}

/// The tab nodes below `Tabs`, in snapshot order. Panics if a node of role `tab` lies outside
/// `Tabs`, or if `Tabs` is missing while `need` is set.
fn tab_nodes<'a>(root: &'a Value, need: bool, what: &str) -> Vec<&'a Value> {
    let everywhere = preorder(root)
        .into_iter()
        .filter(|n| role(n) == Some("tab"))
        .count();
    let list = tab_list(root, what);
    if need {
        assert!(
            list.is_some(),
            "{what}: no node of role `tab_list` labelled `Tabs` while documents are open: {}",
            short(root)
        );
    }
    let tabs: Vec<&Value> = list
        .map(|t| {
            preorder(t)
                .into_iter()
                .skip(1)
                .filter(|n| role(n) == Some("tab"))
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(
        tabs.len(),
        everywhere,
        "{what}: a node of role `tab` lies outside `Tabs`: {}",
        short(root)
    );
    tabs
}

fn tab_label(n: &Value) -> String {
    label(n)
        .unwrap_or_else(|| panic!("a tab without a label: {}", short(n)))
        .to_owned()
}

fn tab_labels(root: &Value, need: bool, what: &str) -> Vec<String> {
    tab_nodes(root, need, what)
        .into_iter()
        .map(tab_label)
        .collect()
}

/// What one tab should show.
#[derive(Clone, Debug)]
struct WantTab {
    path: String,
    current: bool,
    dirty: bool,
    discard: bool,
}

/// The nodes after `tabs[i]` in depth-first order and before the next tab (for the last tab,
/// to the end of the snapshot).
fn segment<'a>(all: &[&'a Value], tabs: &[&Value], i: usize) -> Vec<&'a Value> {
    let at = |t: &Value| {
        all.iter()
            .position(|n| std::ptr::eq(*n, t))
            .expect("tab in preorder")
    };
    let from = at(tabs[i]) + 1;
    let to = tabs.get(i + 1).map_or(all.len(), |t| at(t));
    all[from..to].to_vec()
}

/// The tabs equal `want` (Interface "What the UI shows"): `Tabs` exactly when a document is
/// open, inside a `scroll_view` without shell, `Title`, `Editor` or project-panel nodes; one
/// tab per document in order, label = path, no value, `toggled` = current; after each tab and
/// before the next one `modified` node exactly when dirty, one `Close <path>` button, and one
/// `Discard <path>` button exactly when `discard`. Rows are compared where they are specified
/// (`check_rows`) and the node is not `offscreen`.
fn check_tabs(root: &Value, want: &[WantTab], check_rows: bool, what: &str) {
    if want.is_empty() {
        assert!(
            tab_list(root, what).is_none(),
            "{what}: `Tabs` is shown with no document open: {}",
            short(root)
        );
        assert!(
            tab_nodes(root, false, what).is_empty(),
            "{what}: a tab is shown with no document open"
        );
        return;
    }
    let tabs = tab_nodes(root, true, what);
    let list = tab_list(root, what).expect("checked above");
    let sv = scroll_view_around(root, list).unwrap_or_else(|| {
        panic!(
            "{what}: `Tabs` is inside no node of role `scroll_view`: {}",
            short(root)
        )
    });
    for n in preorder(sv) {
        if let Some(l) = label(n) {
            assert!(
                ![
                    "Path",
                    "Open",
                    "Message",
                    "Documents",
                    "Title",
                    "Editor",
                    "Project",
                    "Project files"
                ]
                .contains(&l),
                "{what}: the scroll_view around Tabs contains the node `{l}`"
            );
        }
        assert!(
            !matches!(role(n), Some("tree" | "tree_item")),
            "{what}: the scroll_view around Tabs contains a project-panel node"
        );
    }
    let got: Vec<String> = tabs.iter().map(|n| tab_label(n)).collect();
    let exp: Vec<String> = want.iter().map(|w| w.path.clone()).collect();
    assert_eq!(got, exp, "{what}: tab labels below Tabs, in order");
    let all = preorder(root);
    for (i, w) in want.iter().enumerate() {
        let t = tabs[i];
        let p = &w.path;
        assert_eq!(value(t), None, "{what}: the tab `{p}` has a value");
        assert_eq!(
            toggled(t),
            Some(w.current),
            "{what}: `toggled` of the tab `{p}` (current: {})",
            w.current
        );
        if check_rows && !offscreen(t) && !rows_unspecified(p) {
            assert_eq!(
                row_texts(t),
                std::slice::from_ref(p),
                "{what}: the tab `{p}` paints its path"
            );
        }
        let seg = segment(&all, &tabs, i);
        let marks: Vec<&&Value> = seg
            .iter()
            .filter(|n| role(n) == Some("label") && value(n) == Some("modified"))
            .collect();
        assert_eq!(
            marks.len(),
            usize::from(w.dirty),
            "{what}: `modified` nodes after the tab `{p}` (dirty: {})",
            w.dirty
        );
        if check_rows {
            for m in &marks {
                if !offscreen(m) {
                    assert_eq!(row_texts(m), ["modified"], "{what}: `modified` paints");
                }
            }
        }
        for (kind, want_n) in [("Close", 1), ("Discard", usize::from(w.discard))] {
            let l = format!("{kind} {p}");
            let everywhere: Vec<&Value> = all
                .iter()
                .copied()
                .filter(|n| label(n) == Some(l.as_str()))
                .collect();
            let here: Vec<&&Value> = seg
                .iter()
                .filter(|n| role(n) == Some("button") && label(n) == Some(l.as_str()))
                .collect();
            assert_eq!(
                (here.len(), everywhere.len()),
                (want_n, want_n),
                "{what}: nodes labelled `{l}` (after its tab, and anywhere)"
            );
            if check_rows {
                for b in here {
                    if offscreen(b) {
                        continue;
                    }
                    let rows = row_texts(b);
                    if kind == "Close" {
                        assert!(
                            rows.iter().all(String::is_empty),
                            "{what}: `{l}` paints text {rows:?}"
                        );
                    } else if !rows_unspecified(p) {
                        assert_eq!(
                            rows,
                            std::slice::from_ref(&l),
                            "{what}: `{l}` paints its label"
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The registry and the log.
// ---------------------------------------------------------------------------------------------

/// One entry of `doc.list`.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Listed {
    doc: u64,
    path: String,
    dirty: bool,
}

fn doc_list(result: &Value) -> Vec<Listed> {
    result["docs"]
        .as_array()
        .unwrap_or_else(|| panic!("doc.list without docs: {result}"))
        .iter()
        .map(|d| Listed {
            doc: d["doc"].as_u64().expect("doc id"),
            path: d["path"].as_str().expect("doc path").to_owned(),
            dirty: d["dirty"].as_bool().expect("dirty"),
        })
        .collect()
}

fn paths_of(list: &[Listed]) -> Vec<String> {
    list.iter().map(|d| d.path.clone()).collect()
}

fn params_of(e: &Value) -> Value {
    e.get("params").cloned().unwrap_or_else(|| json!({}))
}

/// The control requests of the script that succeeded, as `(command, params)`, in order.
fn script_controls(s: &Sess) -> Vec<(String, Value)> {
    s.script
        .iter()
        .zip(&s.resp)
        .filter_map(|(line, resp)| {
            let v: Value = serde_json::from_str(line).ok()?;
            if v.get("ui").is_some() || resp["ok"] != json!(true) {
                return None;
            }
            Some((v["command"].as_str()?.to_owned(), params_of(&v)))
        })
        .collect()
}

/// The logged requests the script did not send: the log minus the script's successful control
/// requests, matched in order.
fn ui_logged(s: &Sess) -> Vec<Value> {
    let log = s.log_entries();
    let mine = script_controls(s);
    let mut k = 0;
    let mut rest = Vec::new();
    for e in log {
        let c = e["command"].as_str().unwrap_or_default();
        if k < mine.len() && mine[k].0 == c && mine[k].1 == params_of(&e) {
            k += 1;
        } else {
            rest.push(e);
        }
    }
    assert_eq!(
        k,
        mine.len(),
        "{}: the log lacks the script's control request {:?}",
        s.name,
        mine.get(k)
    );
    rest
}

/// The params of the logged requests of `command` among `log`.
fn logged(log: &[Value], command: &str) -> Vec<Value> {
    log.iter()
        .filter(|e| e["command"] == json!(command))
        .map(params_of)
        .collect()
}

/// The log entries `after` added to `before`, where `after`'s script is `before`'s without its
/// last `before_tail` control reads (which ran no frame), plus more requests and `after_tail`
/// control reads at its end (which run no frame either): the entries between the two tails.
fn added_by(before: &Sess, before_tail: usize, after: &Sess, after_tail: usize) -> Vec<Value> {
    let a = before.log_entries();
    let b = after.log_entries();
    let core = &a[..a.len() - before_tail];
    assert!(
        b.len() >= core.len() + after_tail && b[..core.len()] == *core,
        "{}: the log does not extend that of {} (the sessions are not deterministic)",
        after.name,
        before.name
    );
    let mine = script_controls(after);
    let tail = &b[b.len() - after_tail..];
    for (e, (c, p)) in tail.iter().zip(&mine[mine.len() - after_tail..]) {
        assert!(
            e["command"] == json!(c) && params_of(e) == *p,
            "{}: the log does not end with the script's reads",
            after.name
        );
    }
    b[core.len()..b.len() - after_tail].to_vec()
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

/// A switch's log entries (AC-2, AC-6): at least one, all of read-only commands, at least one
/// naming `doc`.
fn check_switch_log(added: &[Value], doc: u64, what: &str) {
    let flags = mutating_flags();
    assert!(
        !added.is_empty(),
        "{what}: the switch added nothing to the log (the views did not re-read)"
    );
    for e in added {
        let c = e["command"].as_str().unwrap_or_default();
        let m = *flags
            .get(c)
            .unwrap_or_else(|| panic!("{what}: logged command `{c}` is not in commands.json"));
        assert!(!m, "{what}: the switch logged the mutating command {e}");
    }
    assert!(
        added.iter().any(|e| params_of(e)["doc"] == json!(doc)),
        "{what}: no logged read names the new document {doc}: {added:?}"
    );
}

/// Replay `log` with `Host` on `root`; every request must succeed again.
fn replay(root: &Path, log: &[Value]) -> Host {
    let mut host = Host::on_disk(root).expect("Host::on_disk on the replay copy");
    for e in log {
        let line = e.to_string();
        let resp = host
            .handle_line(line.as_bytes())
            .expect("a response per line");
        let v: Value = serde_json::from_str(&resp).expect("JSON");
        assert_eq!(
            v["ok"],
            json!(true),
            "replaying the log: {line} answered {resp}"
        );
    }
    host
}

fn host_req(host: &mut Host, command: &str, params: Value) -> Value {
    let line = json!({"command": command, "params": params}).to_string();
    let resp = host.handle_line(line.as_bytes()).expect("response");
    serde_json::from_str(&resp).expect("JSON")
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

/// The want of [`check_tabs`] from `doc.list` and the current path, nothing discarding.
fn want_from_list(list: &[Listed], current: Option<&str>) -> Vec<WantTab> {
    list.iter()
        .map(|d| WantTab {
            path: d.path.clone(),
            current: current == Some(d.path.as_str()),
            dirty: d.dirty,
            discard: false,
        })
        .collect()
}

// =============================================================================================
// AC-1: tabs equal the documents (oracle: doc.list and Title in the same session).
// =============================================================================================

const AC1_FILES: [&str; 6] = [
    "a.rs",
    "b.txt",
    "dir x/c.txt",
    "dir x/\u{e9}.txt",
    "z9.md",
    "\u{6f22}\u{5b57}.txt",
];

fn ac1_root(dir: &Path) {
    for (i, f) in AC1_FILES.iter().enumerate() {
        write(&dir.join(ROOT).join(f), format!("file {i}\n").as_bytes());
    }
}

/// After each step of random opens (control `doc.open`, `Path`/`Open`, project-panel clicks)
/// and control closes, the tabs equal `doc.list` in order, exactly one is `toggled: true` and
/// it is the document `Title` begins with; no `Tabs` with nothing open.
#[test]
fn ac1_tabs_follow_random_opens_and_closes() {
    for seed in 1..=4_u64 {
        let dir = case_dir(&format!("ac1-{seed}"));
        ac1_root(&dir);
        let mut rng = Rng::new(seed);
        let mut plan = Plan::new(&dir, "s", &root_args());
        plan.raw(snapshot("s-start"));
        // Set up the project panel: expanded, `dir x` expanded (not steps).
        plan.click("panel", json!({"role": "button", "label": "Project"}));
        plan.click("panel-dir", json!({"role": "tree_item", "label": "dir x"}));
        let mut open_ids: Vec<(u64, String)> = Vec::new();
        let mut next_id = 1;
        let mut path_text = String::new();
        let mut steps: Vec<String> = Vec::new();
        let mut ui_opens: Vec<(usize, String)> = Vec::new();
        for k in 0..16 {
            let choice = if open_ids.is_empty() {
                rng.below(3)
            } else {
                rng.below(5)
            };
            let id = format!("k{k}");
            match choice {
                0..=2 => {
                    let p = (*rng.pick(&AC1_FILES)).to_owned();
                    match choice {
                        0 => {
                            plan.raw(open(&id, &p));
                            plan.raw(frames(&format!("{id}-f"), SETTLE));
                            steps.push(format!("control doc.open `{p}`"));
                        }
                        1 => {
                            plan.raws(ui_open_lines(&id, &path_text, &p));
                            path_text.clone_from(&p);
                            ui_opens.push((k, p.clone()));
                            steps.push(format!("Path/Open `{p}`"));
                        }
                        _ => {
                            plan.click(&id, json!({"role": "tree_item", "label": p}));
                            steps.push(format!("project-panel click `{p}`"));
                        }
                    }
                    if !open_ids.iter().any(|(_, q)| *q == p) {
                        open_ids.push((next_id, p));
                        next_id += 1;
                    }
                }
                _ => {
                    let i = rng.below(open_ids.len());
                    let (doc, p) = open_ids.remove(i);
                    plan.raw(ctl(&id, "doc.close", json!({"doc": doc})));
                    plan.raw(frames(&format!("{id}-f"), SETTLE));
                    steps.push(format!("control doc.close {doc} (`{p}`)"));
                }
            }
            plan.raw(snapshot(&format!("s{k}")));
            plan.raw(ctl(&format!("l{k}"), "doc.list", json!({})));
        }
        let s = plan.run();
        check_tabs(s.snap("s-start"), &[], true, &format!("seed {seed} start"));
        for (k, step) in steps.iter().enumerate() {
            let what = format!("seed {seed} step {k} ({step})");
            s.step_ok(&format!("k{k}"), &what);
            let list = doc_list(s.ok(&format!("l{k}")));
            let root = s.snap(&format!("s{k}"));
            let paths = paths_of(&list);
            let labels = tab_labels(root, !list.is_empty(), &what);
            assert_eq!(labels, paths, "{what}: tab labels vs doc.list paths");
            let cur = shown_current(root, &paths, &what);
            let on: Vec<String> = tab_nodes(root, !list.is_empty(), &what)
                .into_iter()
                .filter(|t| toggled(t) == Some(true))
                .map(tab_label)
                .collect();
            if list.is_empty() {
                assert!(cur.is_none(), "{what}: a Title with nothing open");
            } else {
                assert_eq!(
                    on.len(),
                    1,
                    "{what}: tabs with toggled: true: {on:?} (exactly one)"
                );
                assert_eq!(
                    Some(on[0].clone()),
                    cur,
                    "{what}: the toggled tab vs the path Title begins with"
                );
            }
            check_tabs(root, &want_from_list(&list, cur.as_deref()), true, &what);
        }
        for (k, p) in &ui_opens {
            assert_eq!(
                path_value(s.snap(&format!("s{k}"))),
                *p,
                "seed {seed} step {k}: Path holds the typed path (test harness)"
            );
        }
        s.all_ui_ok();
        let _ = fs::remove_dir_all(&dir);
    }
}

// =============================================================================================
// AC-2: switching (oracle: registry state, the published `mutating` flags).
// =============================================================================================

fn ac2_root(dir: &Path) {
    let r = dir.join(ROOT);
    write(&r.join("src/a.rs"), A_RS.as_bytes());
    write(&r.join("b.txt"), b"b line\nsecond\n");
    write(&r.join("c.txt"), b"c\n");
}

/// Three documents with distinct selections; `c.txt` (doc 3) current.
fn ac2_prefix() -> Vec<String> {
    vec![
        open("o1", "src/a.rs"),
        open("o2", "b.txt"),
        open("o3", "c.txt"),
        ctl(
            "c1",
            "cursor.set",
            json!({"doc": 1, "anchor": 3, "head": 7}),
        ),
        ctl(
            "c2",
            "cursor.set",
            json!({"doc": 2, "anchor": 9, "head": 2}),
        ),
        frames("f0", SETTLE),
        snapshot("s0"),
    ]
}

/// The lines that check a switch to `doc` (`k` numbers them).
fn ac2_after(k: usize, doc: u64) -> Vec<String> {
    vec![
        snapshot(&format!("s{k}")),
        ctl(&format!("t{k}"), "doc.text", json!({"doc": doc})),
        ctl(&format!("g{k}"), "cursor.get", json!({"doc": doc})),
    ]
}

/// A click on a tab makes `Title` begin with its path, `Editor` show its `doc.text` and its
/// `cursor.get` selection; the log entries it adds are all read-only and one names the new
/// document; a click on the current tab adds nothing.
#[test]
fn ac2_click_on_a_tab_switches_with_reads_only() {
    let dir = case_dir("ac2");
    ac2_root(&dir);
    let paths: Vec<String> = ["src/a.rs", "b.txt", "c.txt"]
        .iter()
        .map(|p| (*p).to_owned())
        .collect();
    // Session k = the prefix and the first k switches; each switch is checked against the
    // session before it.
    let switches: [(u64, &str); 3] = [(1, "src/a.rs"), (2, "b.txt"), (3, "c.txt")];
    let mut sessions = vec![run(&dir, "p0", &ac2_prefix(), &root_args())];
    let mut script = ac2_prefix();
    for (k, (doc, p)) in switches.iter().enumerate() {
        let mut next = script.clone();
        next.push(click(&format!("x{k}"), &tab_target(p)));
        next.extend(ac2_after(k + 1, *doc));
        // The script of the following session goes on from here without the control reads, so
        // that its log extends this one's.
        script.push(click(&format!("x{k}"), &tab_target(p)));
        sessions.push(run(&dir, &format!("p{}", k + 1), &next, &root_args()));
    }
    let base = &sessions[0];
    assert_eq!(
        shown_current(base.snap("s0"), &paths, "before any switch"),
        Some("c.txt".to_owned()),
        "precondition: c.txt (the last opened) is current"
    );
    for (k, (doc, p)) in switches.iter().enumerate() {
        let s = &sessions[k + 1];
        let what = format!("after the click on the tab `{p}`");
        s.act(&format!("x{k}"));
        let root = s.snap(&format!("s{}", k + 1));
        assert_eq!(
            shown_current(root, &paths, &what).as_deref(),
            Some(*p),
            "{what}: Title"
        );
        assert_eq!(
            editor_value(root, &what),
            s.text(&format!("t{}", k + 1)),
            "{what}: Editor vs doc.text"
        );
        let g = s.ok(&format!("g{}", k + 1));
        assert_eq!(
            editor_sel(root, &what),
            (
                g["anchor"].as_u64().expect("anchor"),
                g["head"].as_u64().expect("head")
            ),
            "{what}: Editor's selection vs cursor.get"
        );
        let want: Vec<WantTab> = paths
            .iter()
            .map(|q| WantTab {
                path: q.clone(),
                current: q == p,
                dirty: false,
                discard: false,
            })
            .collect();
        check_tabs(root, &want, true, &what);
        let added = added_by(&sessions[k], if k == 0 { 0 } else { 2 }, s, 2);
        check_switch_log(&added, *doc, &what);
    }
    // The selections were kept per document (the oracle saw the two set ones).
    assert_eq!(sessions[1].ok("g1")["head"], json!(7));
    assert_eq!(sessions[2].ok("g2")["head"], json!(2));

    // A click on the current tab adds nothing to the log and changes nothing shown.
    let mut again = script.clone();
    again.push(snapshot("before"));
    again.push(click("same", &tab_target("c.txt")));
    again.push(frames("idle", SETTLE));
    again.push(snapshot("after"));
    let mut without = script.clone();
    without.push(snapshot("before"));
    let a = run(&dir, "same", &again, &root_args());
    let b = run(&dir, "same-without", &without, &root_args());
    a.act("same");
    assert_eq!(
        shown_current(a.snap("after"), &paths, "after a click on the current tab").as_deref(),
        Some("c.txt")
    );
    assert_eq!(
        a.log, b.log,
        "a click on the current tab added to the session log"
    );
    let _ = fs::remove_dir_all(&dir);
}

// =============================================================================================
// AC-3: closing (oracle: doc.list, Title, log).
// =============================================================================================

/// A refused UI open of `missing.rs` (sets `Message`), or with `first` the lines that type it.
fn refused_open(id: &str, first: bool) -> Vec<String> {
    let mut out = Vec::new();
    if first {
        out.push(focus(&format!("{id}-fp"), json!({"label": "Path"})));
        out.push(type_text(&format!("{id}-tp"), "missing.rs"));
    }
    out.push(click(id, &json!({"label": "Open"})));
    out.push(snapshot(&format!("{id}-s")));
    out
}

/// A click on `Close <path>` of a clean document closes it (`doc.list`, its tab, the log's
/// `doc.close {"doc": id}`); closing the current one makes the highest open id current, closing
/// another leaves the current one; `Message` is empty afterwards.
#[test]
fn ac3_close_clean_documents_from_their_tabs() {
    let dir = case_dir("ac3");
    let r = dir.join(ROOT);
    for p in ["a.txt", "b.txt", "c.txt", "d.txt"] {
        write(&r.join(p), format!("{p}\n").as_bytes());
    }
    let mut plan = Plan::new(&dir, "s", &root_args());
    plan.raws(vec![
        open("o1", "a.txt"),
        open("o2", "b.txt"),
        open("o3", "c.txt"),
        open("o4", "d.txt"),
        // Re-open b.txt: it is current, and its neighbours differ from the highest id.
        open("o2b", "b.txt"),
        frames("f0", SETTLE),
        snapshot("s0"),
    ]);
    // (path closed, current after, open after)
    let closes: [(&str, u64, &str); 4] = [
        ("c.txt", 3, "b.txt"),
        ("b.txt", 2, "d.txt"),
        ("a.txt", 1, "d.txt"),
        ("d.txt", 4, ""),
    ];
    for (k, (p, _, _)) in closes.iter().enumerate() {
        plan.raws(refused_open(&format!("m{k}"), k == 0));
        plan.click(&format!("x{k}"), close_target(p));
        plan.raw(snapshot(&format!("s{}", k + 1)));
        plan.raw(ctl(&format!("l{}", k + 1), "doc.list", json!({})));
    }
    let s = plan.run();
    let all: Vec<String> = ["a.txt", "b.txt", "c.txt", "d.txt"]
        .iter()
        .map(|p| (*p).to_owned())
        .collect();
    assert_eq!(
        shown_current(s.snap("s0"), &all, "s0").as_deref(),
        Some("b.txt"),
        "precondition: b.txt is current"
    );
    let mut open_now = all.clone();
    for (k, (p, _, cur)) in closes.iter().enumerate() {
        let what = format!("after the click on `Close {p}`");
        assert!(
            message(s.snap(&format!("m{k}-s"))).starts_with("not_found"),
            "precondition: Message shows the refused open before `{what}`"
        );
        s.act(&format!("x{k}"));
        open_now.retain(|q| q != p);
        let root = s.snap(&format!("s{}", k + 1));
        let list = doc_list(s.ok(&format!("l{}", k + 1)));
        assert_eq!(paths_of(&list), open_now, "{what}: doc.list");
        assert_eq!(message(root), "", "{what}: Message is cleared");
        let shown = shown_current(root, &open_now, &what);
        let want_cur = (!cur.is_empty()).then(|| (*cur).to_owned());
        assert_eq!(shown, want_cur, "{what}: the current document");
        check_tabs(
            root,
            &want_from_list(&list, want_cur.as_deref()),
            true,
            &what,
        );
    }
    let closed: Vec<Value> = closes.iter().map(|(_, d, _)| json!({"doc": d})).collect();
    assert_eq!(
        logged(&ui_logged(&s), "doc.close"),
        closed,
        "the UI's doc.close requests"
    );
    let _ = fs::remove_dir_all(&dir);
}

// =============================================================================================
// AC-4: unsaved changes (oracle: doc.list, log, file bytes).
// =============================================================================================

const AC4_A: &[u8] = b"alpha\n";
const AC4_B: &[u8] = b"beta\n";

fn ac4_root(dir: &Path) -> PathBuf {
    let r = dir.join(ROOT);
    write(&r.join("a.txt"), AC4_A);
    write(&r.join("b.txt"), AC4_B);
    r
}

/// Open a.txt and b.txt (current), edit b.txt from the editor, click `Close b.txt`.
fn ac4_prefix() -> Vec<String> {
    vec![
        open("o1", "a.txt"),
        open("o2", "b.txt"),
        frames("f0", SETTLE),
        snapshot("s0"),
        focus("fe", editor_target()),
        type_text("tz", "z"),
        snapshot("s1"),
        click("x1", &close_target("b.txt")),
        snapshot("s2"),
        ctl("l2", "doc.list", json!({})),
        ctl("t2", "doc.text", json!({"doc": 2})),
    ]
}

/// The want of the two tabs: b.txt dirty/discard as given, `current` = the current path.
fn ac4_want(current: &str, b_dirty: bool, b_discard: bool) -> Vec<WantTab> {
    vec![
        WantTab {
            path: "a.txt".into(),
            current: current == "a.txt",
            dirty: false,
            discard: false,
        },
        WantTab {
            path: "b.txt".into(),
            current: current == "b.txt",
            dirty: b_dirty,
            discard: b_discard,
        },
    ]
}

/// The refused close shared by both sessions: b.txt stays open and dirty, `Message` begins
/// with `unsaved_changes`, `Discard b.txt` appears (and was not there before).
fn ac4_check_refused(s: &Sess) {
    check_tabs(
        s.snap("s0"),
        &ac4_want("b.txt", false, false),
        true,
        "before the edit",
    );
    check_tabs(
        s.snap("s1"),
        &ac4_want("b.txt", true, false),
        true,
        "after the edit, before the close",
    );
    s.act("x1");
    let root = s.snap("s2");
    let what = "after the refused close";
    assert_eq!(
        doc_list(s.ok("l2")),
        [
            Listed {
                doc: 1,
                path: "a.txt".into(),
                dirty: false
            },
            Listed {
                doc: 2,
                path: "b.txt".into(),
                dirty: true
            }
        ],
        "{what}: doc.list"
    );
    assert!(
        message(root).starts_with("unsaved_changes"),
        "{what}: Message `{}`",
        message(root)
    );
    check_tabs(root, &ac4_want("b.txt", true, true), true, what);
}

/// After an edit, `Close` is refused and `Discard` appears; `Discard` closes with
/// `discard: true` and leaves the file as it was.
#[test]
fn ac4_refused_close_then_discard() {
    let dir = case_dir("ac4-discard");
    let r = ac4_root(&dir);
    let mut script = ac4_prefix();
    script.extend([
        click("x2", &discard_target("b.txt")),
        snapshot("s3"),
        ctl("l3", "doc.list", json!({})),
    ]);
    let s = run(&dir, "s", &script, &root_args());
    ac4_check_refused(&s);
    s.act("x2");
    let root = s.snap("s3");
    let what = "after the click on `Discard b.txt`";
    assert_eq!(
        paths_of(&doc_list(s.ok("l3"))),
        ["a.txt"],
        "{what}: doc.list"
    );
    assert_eq!(message(root), "", "{what}: Message is cleared");
    check_tabs(root, &ac4_want("a.txt", false, false)[..1], true, what);
    assert_eq!(
        logged(&ui_logged(&s), "doc.close"),
        [json!({"doc": 2, "discard": true})],
        "the UI's doc.close requests: none for the refused close, one discarding"
    );
    assert_eq!(
        fs::read(r.join("b.txt")).expect("b.txt"),
        AC4_B,
        "{what}: the file"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// After the refused close, `ctrl+S` makes `Discard` disappear, and a later `Close` closes
/// without discarding; the file holds the edit.
#[test]
fn ac4_refused_close_then_save_then_close() {
    let dir = case_dir("ac4-save");
    let r = ac4_root(&dir);
    let mut script = ac4_prefix();
    script.extend([
        focus("fe2", editor_target()),
        key("save", "S", &["ctrl"]),
        snapshot("s3"),
        ctl("l3", "doc.list", json!({})),
        click("x2", &close_target("b.txt")),
        snapshot("s4"),
        ctl("l4", "doc.list", json!({})),
    ]);
    let s = run(&dir, "s", &script, &root_args());
    ac4_check_refused(&s);
    s.act("save");
    let what = "after ctrl+S";
    assert!(
        !doc_list(s.ok("l3"))[1].dirty,
        "{what}: b.txt is clean (precondition)"
    );
    check_tabs(s.snap("s3"), &ac4_want("b.txt", false, false), true, what);
    s.act("x2");
    let what = "after the second click on `Close b.txt`";
    assert_eq!(
        paths_of(&doc_list(s.ok("l4"))),
        ["a.txt"],
        "{what}: doc.list"
    );
    check_tabs(
        s.snap("s4"),
        &ac4_want("a.txt", false, false)[..1],
        true,
        what,
    );
    assert_eq!(
        logged(&ui_logged(&s), "doc.close"),
        [json!({"doc": 2})],
        "the UI's doc.close requests: one, not discarding"
    );
    let edited = s.text("t2");
    assert_ne!(
        edited.as_bytes(),
        AC4_B,
        "precondition: the edit changed the text"
    );
    assert_eq!(
        fs::read(r.join("b.txt")).expect("b.txt"),
        edited.as_bytes(),
        "{what}: the file holds the edit"
    );
    let _ = fs::remove_dir_all(&dir);
}

// =============================================================================================
// AC-5: dirty marks (oracle: doc.list).
// =============================================================================================

/// After random UI edits, undos and saves of the current document and control edits and undos
/// of other documents, a `modified` node follows exactly the tabs `doc.list` reports dirty.
#[test]
fn ac5_modified_marks_follow_doc_list() {
    let files = ["a.txt", "b.txt", "sub/c.txt"];
    for seed in 1..=4_u64 {
        let dir = case_dir(&format!("ac5-{seed}"));
        for f in files {
            write(&dir.join(ROOT).join(f), format!("{f} text\n").as_bytes());
        }
        let mut rng = Rng::new(100 + seed);
        let mut plan = Plan::new(&dir, "s", &root_args());
        for (i, f) in files.iter().enumerate() {
            plan.raw(open(&format!("o{i}"), f));
        }
        plan.raw(frames("f0", SETTLE));
        let mut current: u64 = 3;
        let mut steps = Vec::new();
        for k in 0..24 {
            let id = format!("k{k}");
            let other: Vec<u64> = (1..=3).filter(|d| *d != current).collect();
            match rng.below(6) {
                0 => {
                    let d = (rng.below(3) + 1) as u64;
                    plan.click(&id, tab_target(files[(d - 1) as usize]));
                    current = d;
                    steps.push(format!("click tab {d}"));
                }
                1 => {
                    plan.raw(focus(&format!("{id}-fe"), editor_target()));
                    plan.raw(type_text(&id, rng.pick(&["x", "yz", "\u{e9}"])));
                    steps.push(format!("type into {current}"));
                }
                2 => {
                    plan.raw(focus(&format!("{id}-fe"), editor_target()));
                    plan.raw(key(&id, "Z", &["ctrl"]));
                    steps.push(format!("ctrl+Z in {current}"));
                }
                3 => {
                    plan.raw(focus(&format!("{id}-fe"), editor_target()));
                    plan.raw(key(&id, "S", &["ctrl"]));
                    steps.push(format!("ctrl+S in {current}"));
                }
                4 => {
                    let d = *rng.pick(&other);
                    plan.raw(ctl(
                        &id,
                        "doc.insert",
                        json!({"doc": d, "offset": 0, "text": "q"}),
                    ));
                    plan.raw(frames(&format!("{id}-f"), SETTLE));
                    steps.push(format!("control doc.insert into {d}"));
                }
                _ => {
                    let d = *rng.pick(&other);
                    plan.raw(ctl(&id, "doc.undo", json!({"doc": d})));
                    plan.raw(frames(&format!("{id}-f"), SETTLE));
                    steps.push(format!("control doc.undo of {d}"));
                }
            }
            plan.raw(snapshot(&format!("s{k}")));
            plan.raw(ctl(&format!("l{k}"), "doc.list", json!({})));
        }
        let s = plan.run();
        let mut saw_dirty = false;
        let mut saw_clean_after_dirty = false;
        for (k, step) in steps.iter().enumerate() {
            let what = format!("seed {seed} step {k} ({step})");
            let list = doc_list(s.ok(&format!("l{k}")));
            let root = s.snap(&format!("s{k}"));
            let paths = paths_of(&list);
            let cur = shown_current(root, &paths, &what);
            check_tabs(root, &want_from_list(&list, cur.as_deref()), true, &what);
            let dirty = list.iter().any(|d| d.dirty);
            saw_clean_after_dirty |= saw_dirty && list.iter().any(|d| !d.dirty);
            saw_dirty |= dirty;
        }
        assert!(
            saw_dirty && saw_clean_after_dirty,
            "seed {seed}: the steps made no document dirty and clean again (a test bug)"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}

// =============================================================================================
// AC-6: `Documents` entries (oracle: as AC-2).
// =============================================================================================

/// A click on a `Documents` entry makes its document current (`Title`, `Editor`) and is logged
/// like a tab switch (read-only requests only, one naming the document).
#[test]
fn ac6_click_on_a_documents_entry_switches() {
    let dir = case_dir("ac6");
    ac2_root(&dir);
    let paths: Vec<String> = ["src/a.rs", "b.txt", "c.txt"]
        .iter()
        .map(|p| (*p).to_owned())
        .collect();
    let switches: [(u64, &str); 2] = [(1, "src/a.rs"), (2, "b.txt")];
    let mut sessions = vec![run(&dir, "p0", &ac2_prefix(), &root_args())];
    let mut script = ac2_prefix();
    for (k, (doc, p)) in switches.iter().enumerate() {
        let mut next = script.clone();
        next.push(click(&format!("x{k}"), &entry_target(p)));
        next.extend(ac2_after(k + 1, *doc));
        script.push(click(&format!("x{k}"), &entry_target(p)));
        sessions.push(run(&dir, &format!("p{}", k + 1), &next, &root_args()));
    }
    assert_eq!(
        shown_current(sessions[0].snap("s0"), &paths, "before any click").as_deref(),
        Some("c.txt"),
        "precondition: c.txt is current"
    );
    for (k, (doc, p)) in switches.iter().enumerate() {
        let s = &sessions[k + 1];
        let what = format!("after the click on the Documents entry `{p}`");
        s.act(&format!("x{k}"));
        let root = s.snap(&format!("s{}", k + 1));
        assert_eq!(
            shown_current(root, &paths, &what).as_deref(),
            Some(*p),
            "{what}: Title"
        );
        assert_eq!(
            editor_value(root, &what),
            s.text(&format!("t{}", k + 1)),
            "{what}: Editor vs doc.text"
        );
        let g = s.ok(&format!("g{}", k + 1));
        assert_eq!(
            editor_sel(root, &what),
            (
                g["anchor"].as_u64().expect("anchor"),
                g["head"].as_u64().expect("head")
            ),
            "{what}: Editor's selection vs cursor.get"
        );
        let added = added_by(&sessions[k], if k == 0 { 0 } else { 2 }, s, 2);
        check_switch_log(&added, *doc, &what);
    }
    let _ = fs::remove_dir_all(&dir);
}

// =============================================================================================
// AC-7: idle and reads (oracle: log comparison, doc.list).
// =============================================================================================

fn ac7_controls() -> Vec<String> {
    vec![
        open("1", "a.txt"),
        open("2", "b.txt"),
        ctl(
            "3",
            "doc.insert",
            json!({"doc": 1, "offset": 0, "text": "x"}),
        ),
        open("4", "c.txt"),
        ctl("5", "doc.close", json!({"doc": 2})),
        ctl("6", "doc.close", json!({"doc": 9})),
        ctl("7", "doc.list", json!({})),
    ]
}

fn ac7_root(dir: &Path) -> PathBuf {
    let r = dir.join(ROOT);
    for p in ["a.txt", "b.txt", "c.txt"] {
        write(&r.join(p), format!("{p}\n").as_bytes());
    }
    r
}

/// (a) A script of control requests only writes `Host::log_script()` and answers as `Host`;
/// the same requests followed by one frame show their tabs and read `doc.list` (so the panel
/// is present and the equality is not vacuous).
#[test]
fn ac7a_control_requests_only_log_as_host() {
    let dir = case_dir("ac7a");
    let r = ac7_root(&dir);
    let oracle = dir.join("oracle");
    copy_dir(&r, &oracle);
    let controls = ac7_controls();
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
    let list = doc_list(t.ok("7"));
    let root = t.snap("s");
    let cur = shown_current(root, &paths_of(&list), "after one frame");
    check_tabs(
        root,
        &want_from_list(&list, cur.as_deref()),
        true,
        "the control requests and one frame",
    );
    assert!(
        !logged(&ui_logged(&t), "doc.list").is_empty(),
        "with a frame, the panel reads doc.list"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// (b) 600 frames after tabs exist (after switches, an edit and closes from the UI) add nothing
/// to the log.
#[test]
fn ac7b_idle_frames_after_tabs_add_nothing() {
    let dir = case_dir("ac7b");
    ac7_root(&dir);
    let prefix = vec![
        open("o1", "a.txt"),
        open("o2", "b.txt"),
        open("o3", "c.txt"),
        frames("f0", SETTLE),
        click("t1", &tab_target("a.txt")),
        focus("fe", editor_target()),
        type_text("ty", "y"),
        click("x1", &close_target("a.txt")),
        click("x2", &close_target("b.txt")),
        click("t3", &entry_target("c.txt")),
    ];
    let mut with = prefix.clone();
    with.push(frames("idle", 600));
    with.push(snapshot("s"));
    with.push(ctl("l", "doc.list", json!({})));
    let mut without = prefix;
    without.push(snapshot("s"));
    without.push(ctl("l", "doc.list", json!({})));
    let a = run(&dir, "with", &with, &root_args());
    let b = run(&dir, "without", &without, &root_args());
    b.all_ui_ok();
    let list = doc_list(b.ok("l"));
    assert_eq!(
        paths_of(&list),
        ["a.txt", "c.txt"],
        "precondition: b.txt closed"
    );
    let root = b.snap("s");
    let cur = shown_current(root, &paths_of(&list), "without");
    let mut want = want_from_list(&list, cur.as_deref());
    want[0].discard = true;
    check_tabs(root, &want, true, "before the idle frames");
    assert!(
        !logged(&ui_logged(&b), "doc.list").is_empty(),
        "precondition: the panel read doc.list"
    );
    a.all_ui_ok();
    assert_eq!(a.log, b.log, "600 idle frames added to the log");
    let _ = fs::remove_dir_all(&dir);
}

/// (c) After a control `doc.insert` on a non-current document, the frames until the UI settles
/// add a `doc.list` read and the tab's `modified` node appears; 600 further frames add nothing.
#[test]
fn ac7c_a_control_edit_is_read_then_idle() {
    let dir = case_dir("ac7c");
    ac7_root(&dir);
    let prefix = vec![
        open("o1", "a.txt"),
        open("o2", "b.txt"),
        frames("f0", SETTLE),
        snapshot("s0"),
        ctl(
            "ins",
            "doc.insert",
            json!({"doc": 1, "offset": 0, "text": "x"}),
        ),
        frames("f1", SETTLE_CAP),
        snapshot("s1"),
    ];
    // `l1` comes last in both sessions: a control request is itself a change.
    let mut with = prefix.clone();
    with.push(frames("idle", 600));
    with.push(ctl("l1", "doc.list", json!({})));
    let mut without = prefix;
    without.push(ctl("l1", "doc.list", json!({})));
    let a = run(&dir, "with", &with, &root_args());
    let b = run(&dir, "without", &without, &root_args());
    let list = doc_list(b.ok("l1"));
    assert!(list[0].dirty && !list[1].dirty, "precondition: doc.list");
    check_tabs(
        b.snap("s0"),
        &want_from_list(&list, Some("b.txt"))
            .into_iter()
            .map(|mut w| {
                w.dirty = false;
                w
            })
            .collect::<Vec<_>>(),
        true,
        "before the control edit",
    );
    check_tabs(
        b.snap("s1"),
        &want_from_list(&list, Some("b.txt")),
        true,
        "after the control edit",
    );
    // The log after the `doc.insert` and before the script's own last `doc.list` (`l1`, which
    // runs no frame): the reads of the settling frames, among them `doc.list`.
    let log = b.log_entries();
    let at = log
        .iter()
        .position(|e| e["command"] == json!("doc.insert"))
        .expect("the doc.insert is logged");
    let last = log.last().expect("a log");
    assert_eq!(last["command"], json!("doc.list"), "the log ends with `l1`");
    let settling = &log[at + 1..log.len() - 1];
    assert!(
        settling.iter().any(|e| e["command"] == json!("doc.list")),
        "the frames after the control edit logged no doc.list read: {settling:?}"
    );
    assert_eq!(
        a.log, b.log,
        "600 frames after the settling added to the log"
    );
    let _ = fs::remove_dir_all(&dir);
}

// =============================================================================================
// AC-8: generated sessions against a naive model (oracle: model, doc.list, replay).
// =============================================================================================

/// T-057 AC-3's alphabet: pieces of names (ASCII, precomposed `é`, `e`+U+0301, a zero-width
/// joiner, `ffi`, CJK, an emoji, Hebrew, Arabic, an Arabic-Indic digit).
const PIECES: [&str; 14] = [
    "a",
    "b",
    "Q",
    "7",
    "k9",
    "ffi",
    "\u{e9}",
    "e\u{301}",
    "\u{200d}",
    "\u{6f22}\u{5b57}",
    "\u{1f600}",
    "\u{5e9}\u{5dc}\u{5d5}\u{5dd}",
    "\u{633}\u{644}\u{627}\u{645}",
    "\u{663}",
];

/// What may join two pieces: nothing, an inner space, `-`, `_`, `.` (never trailing).
const JOINS: [&str; 5] = ["", " ", "-", "_", "."];

/// A name: one to three pieces, never starting with the joiner.
fn gen_name(rng: &mut Rng) -> String {
    let n = 1 + rng.below(3);
    let mut s = String::new();
    for k in 0..n {
        if k > 0 {
            s.push_str(JOINS[rng.below(JOINS.len())]);
        }
        let piece = PIECES[rng.below(PIECES.len())];
        s.push_str(if k == 0 && piece == "\u{200d}" {
            "a"
        } else {
            piece
        });
    }
    s
}

/// A key equal for two names a file system could take for one (case, `e`+U+0301 vs `é`).
fn fold(name: &str) -> String {
    name.replace("e\u{301}", "\u{e9}").to_lowercase()
}

/// A random root: four files at the root and two in one directory, all distinct by [`fold`].
fn gen_root(rng: &mut Rng) -> Vec<String> {
    let mut top = BTreeSet::new();
    let dir_name = gen_name(rng);
    top.insert(fold(&dir_name));
    let mut out = Vec::new();
    while out.len() < 4 {
        let n = gen_name(rng);
        if top.insert(fold(&n)) {
            out.push(n);
        }
    }
    let mut inner = BTreeSet::new();
    while out.len() < 6 {
        let n = gen_name(rng);
        if inner.insert(fold(&n)) {
            out.push(format!("{dir_name}/{n}"));
        }
    }
    out
}

/// One document of the model.
#[derive(Clone, Debug)]
struct MDoc {
    id: u64,
    path: String,
    dirty: bool,
    discard: bool,
}

/// The naive model: the current-document rule (1–4) and T3, T4 of the spec.
#[derive(Clone, Debug, Default)]
struct Model {
    docs: Vec<MDoc>,
    next_id: u64,
    current: Option<u64>,
}

impl Model {
    fn new() -> Self {
        Model {
            next_id: 1,
            ..Model::default()
        }
    }

    fn find(&self, id: u64) -> &MDoc {
        self.docs
            .iter()
            .find(|d| d.id == id)
            .expect("open in the model")
    }

    fn find_mut(&mut self, id: u64) -> &mut MDoc {
        self.docs
            .iter_mut()
            .find(|d| d.id == id)
            .expect("open in the model")
    }

    /// Rule 1: an open (a re-open included) makes the document current.
    fn open(&mut self, path: &str) {
        if let Some(d) = self.docs.iter().find(|d| d.path == path) {
            self.current = Some(d.id);
        } else {
            let id = self.next_id;
            self.next_id += 1;
            self.docs.push(MDoc {
                id,
                path: path.to_owned(),
                dirty: false,
                discard: false,
            });
            self.current = Some(id);
        }
    }

    /// Rule 2.
    fn switch(&mut self, id: u64) {
        self.current = Some(id);
    }

    /// Rule 3: closing removes it; if it was current, the highest open id is current.
    fn remove(&mut self, id: u64) {
        self.docs.retain(|d| d.id != id);
        if self.current == Some(id) {
            self.current = self.docs.iter().map(|d| d.id).max();
        }
    }

    /// T3: `Close` of a dirty document is refused and shows `Discard`; of a clean one closes.
    fn close_click(&mut self, id: u64) {
        if self.find(id).dirty {
            self.find_mut(id).discard = true;
        } else {
            self.remove(id);
        }
    }

    fn typed(&mut self) {
        let c = self.current.expect("a current document");
        self.find_mut(c).dirty = true;
    }

    /// A save makes the document clean, which ends its `Discard` (T3).
    fn saved(&mut self) {
        let c = self.current.expect("a current document");
        let d = self.find_mut(c);
        d.dirty = false;
        d.discard = false;
    }

    fn want(&self) -> Vec<WantTab> {
        self.docs
            .iter()
            .map(|d| WantTab {
                path: d.path.clone(),
                current: self.current == Some(d.id),
                dirty: d.dirty,
                discard: d.discard,
            })
            .collect()
    }

    fn listed(&self) -> Vec<Listed> {
        self.docs
            .iter()
            .map(|d| Listed {
                doc: d.id,
                path: d.path.clone(),
                dirty: d.dirty,
            })
            .collect()
    }

    fn current_path(&self) -> Option<String> {
        self.current.map(|c| self.find(c).path.clone())
    }
}

/// One generated session of AC-8: the plan, the step descriptions and the model after each.
fn ac8_plan(
    dir: &Path,
    files: &[String],
    rng: &mut Rng,
    n: usize,
) -> (Plan, Vec<String>, Vec<Model>) {
    let mut plan = Plan::new(dir, "s", &root_args());
    plan.raw(frames("f0", SETTLE));
    let mut m = Model::new();
    let mut steps = Vec::new();
    let mut models = Vec::new();
    let mut path_text = String::new();
    for k in 0..n {
        let id = format!("k{k}");
        let mut kind = rng.below(31);
        if m.docs.is_empty() {
            kind %= 6; // only opens
        }
        let discards: Vec<u64> = m.docs.iter().filter(|d| d.discard).map(|d| d.id).collect();
        if (24..27).contains(&kind) && discards.is_empty() {
            kind = 13; // a close instead
        }
        let pick_doc = |rng: &mut Rng, m: &Model| m.docs[rng.below(m.docs.len())].clone();
        let desc = match kind {
            0..=2 => {
                let p = rng.pick(files).clone();
                plan.raw(open(&id, &p));
                plan.raw(frames(&format!("{id}-f"), SETTLE));
                m.open(&p);
                format!("control doc.open `{p}`")
            }
            3..=5 => {
                let p = rng.pick(files).clone();
                plan.raws(ui_open_lines(&id, &path_text, &p));
                path_text.clone_from(&p);
                m.open(&p);
                format!("Path/Open `{p}`")
            }
            6..=10 => {
                let d = pick_doc(rng, &m);
                plan.click(&id, tab_target(&d.path));
                m.switch(d.id);
                format!("click tab `{}`", d.path)
            }
            11..=12 => {
                let d = pick_doc(rng, &m);
                plan.click(&id, entry_target(&d.path));
                m.switch(d.id);
                format!("click Documents entry `{}`", d.path)
            }
            13..=16 => {
                let d = pick_doc(rng, &m);
                plan.click(&id, close_target(&d.path));
                m.close_click(d.id);
                format!("click `Close {}` (dirty: {})", d.path, d.dirty)
            }
            17..=21 => {
                let t = *rng.pick(&["x", "yz", "\u{e9}", " q", "\u{6f22}"]);
                plan.raw(focus(&format!("{id}-fe"), editor_target()));
                plan.raw(type_text(&id, t));
                m.typed();
                format!("type {t:?}")
            }
            22..=23 => {
                plan.raw(focus(&format!("{id}-fe"), editor_target()));
                plan.raw(key(&id, "S", &["ctrl"]));
                m.saved();
                "ctrl+S".to_owned()
            }
            24..=26 => {
                let d = *rng.pick(&discards);
                let p = m.find(d).path.clone();
                plan.click(&id, discard_target(&p));
                m.remove(d);
                format!("click `Discard {p}`")
            }
            _ => {
                let dir = *rng.pick(&["down", "up"]);
                let steps_n = 1 + rng.below(2) as u64;
                plan.raw(scroll(&id, &tabs_target(), dir, steps_n));
                format!("scroll Tabs {dir} {steps_n}")
            }
        };
        plan.raw(snapshot(&format!("s{k}")));
        plan.raw(ctl(&format!("l{k}"), "doc.list", json!({})));
        steps.push(desc);
        models.push(m.clone());
    }
    (plan, steps, models)
}

/// For 40 seeds: a random root from the alphabet and 30 random steps; after every step the
/// snapshot equals the naive model (tabs, `toggled`, `modified`, `Discard`, the current
/// document) and `doc.list` equals the model's; the final log replayed with `Host` on a copy of
/// the root gives the same `doc.list` and files.
#[test]
fn ac8_generated_sessions_follow_the_naive_model() {
    const SEEDS: u64 = 40;
    const STEPS: usize = 30;
    for seed in 1..=SEEDS {
        let dir = case_dir(&format!("ac8-{seed}"));
        let mut rng = Rng::new(1000 + seed);
        let files = gen_root(&mut rng);
        let root = dir.join(ROOT);
        for (i, f) in files.iter().enumerate() {
            write(&root.join(f), format!("text {i}\n").as_bytes());
        }
        let initial = dir.join("initial");
        copy_dir(&root, &initial);
        let (mut plan, steps, models) = ac8_plan(&dir, &files, &mut rng, STEPS);
        plan.raw(ctl("end-list", "doc.list", json!({})));
        let s = plan.run();
        for (k, (step, m)) in steps.iter().zip(&models).enumerate() {
            let what = format!(
                "seed {seed}, step {k} of {STEPS} ({step}; {} open; files {files:?})",
                m.docs.len()
            );
            s.step_ok(&format!("k{k}"), &what);
            assert_eq!(
                doc_list(s.ok(&format!("l{k}"))),
                m.listed(),
                "{what}: doc.list vs the model"
            );
            let snap = s.snap(&format!("s{k}"));
            let paths: Vec<String> = m.docs.iter().map(|d| d.path.clone()).collect();
            assert_eq!(
                shown_current(snap, &paths, &what),
                m.current_path(),
                "{what}: the current document (Title) vs the model"
            );
            check_tabs(snap, &m.want(), true, &what);
        }
        s.all_ui_ok();
        let copy = dir.join("replay");
        copy_dir(&initial, &copy);
        let mut host = replay(&copy, &s.log_entries());
        let list = host_req(&mut host, "doc.list", json!({}));
        assert_eq!(
            doc_list(&list["result"]),
            doc_list(s.ok("end-list")),
            "seed {seed}: doc.list after replaying the log"
        );
        assert_eq!(
            tree(&copy),
            tree(&root),
            "seed {seed}: files after replaying the log"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}

// =============================================================================================
// AC-9: many documents (oracle: Title, settled).
// =============================================================================================

const MANY: usize = 300;

fn many_name(i: usize) -> String {
    format!("f{i:03}.txt")
}

fn many_opens() -> Vec<String> {
    let mut out: Vec<String> = (0..MANY)
        .map(|i| open(&format!("o{i}"), &many_name(i)))
        .collect();
    out.push(frames("f0", SETTLE));
    out
}

/// The scroll position (steps from the start) at which each tab is first not `offscreen`, from
/// a run that scrolls `Tabs` one step at a time in `direction`; `None` for a tab never visible.
fn survey(dir: &Path, direction: &str, max: usize) -> (Sess, Vec<Option<usize>>) {
    let back = if direction == "right" { "left" } else { "up" };
    let mut script = many_opens();
    // From the start of the strip, whatever an open did to its scrolling.
    script.push(scroll("sc0", &tabs_target(), back, 100));
    script.push(snapshot("v0"));
    for i in 1..=max {
        script.push(scroll(&format!("sc{i}"), &tabs_target(), direction, 1));
        script.push(snapshot(&format!("v{i}")));
    }
    let s = run(dir, &format!("survey-{direction}"), &script, &root_args());
    let names: Vec<String> = (0..MANY).map(many_name).collect();
    let mut first: Vec<Option<usize>> = vec![None; MANY];
    for i in 0..=max {
        let root = s.snap(&format!("v{i}"));
        let what = format!("survey {direction}, after {i} steps");
        let tabs = tab_nodes(root, true, &what);
        let labels: Vec<String> = tabs.iter().map(|t| tab_label(t)).collect();
        assert_eq!(labels, names, "{what}: one tab per document, in order");
        for (j, t) in tabs.iter().enumerate() {
            if first[j].is_none() && !offscreen(t) {
                first[j] = Some(i);
            }
        }
    }
    (s, first)
}

/// With 300 open documents at the default size, every tab is reached by scrolling `Tabs` until
/// it is not `offscreen`, and its click makes it current; a click on an `offscreen` tab is
/// `not_actionable`; every action settles within 60 frames.
#[test]
fn ac9_every_tab_of_300_documents_can_be_reached_and_clicked() {
    let dir = case_dir("ac9");
    let r = dir.join(ROOT);
    for i in 0..MANY {
        write(&r.join(many_name(i)), format!("{i}\n").as_bytes());
    }
    let names: Vec<String> = (0..MANY).map(many_name).collect();
    const MAX: usize = 60;
    let (mut s, mut first) = survey(&dir, "down", MAX);
    if first.iter().any(Option::is_none) && s.r("sc1")["ok"] == json!(true) {
        let unchanged = {
            let a: Vec<bool> = tab_nodes(s.snap("v0"), true, "v0")
                .iter()
                .map(|t| offscreen(t))
                .collect();
            let b: Vec<bool> = tab_nodes(s.snap("v1"), true, "v1")
                .iter()
                .map(|t| offscreen(t))
                .collect();
            a == b
        };
        if unchanged {
            (s, first) = survey(&dir, "right", MAX);
        }
    }
    s.all_ui_ok();
    let direction = if s.name.ends_with("right") {
        "right"
    } else {
        "down"
    };
    let back = if direction == "right" { "left" } else { "up" };
    let unreached: Vec<&String> = names
        .iter()
        .zip(&first)
        .filter(|(_, f)| f.is_none())
        .map(|(n, _)| n)
        .collect();
    assert!(
        unreached.is_empty(),
        "tabs never reached by scrolling Tabs {MAX} steps {direction}: {unreached:?}"
    );
    let first: Vec<usize> = first.into_iter().map(|f| f.expect("reached")).collect();
    let hidden = first
        .iter()
        .position(|f| *f > 0)
        .expect("with 300 documents some tab starts offscreen (Interface)");

    // Clicks: for each scroll position, scroll back to the start and forward to it, then click
    // every tab first visible there. `reset_each` scrolls before every click instead (needed
    // only if a click itself scrolls the strip).
    let plan_clicks = |reset_each: bool| -> Vec<String> {
        let mut script = many_opens();
        script.push(scroll("h-back", &tabs_target(), back, 100));
        script.push(click("hidden", &tab_target(&names[hidden])));
        let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for (j, f) in first.iter().enumerate() {
            groups.entry(*f).or_default().push(j);
        }
        let position = |script: &mut Vec<String>, tag: &str, at: usize| {
            script.push(scroll(&format!("{tag}-back"), &tabs_target(), back, 100));
            for i in 0..at {
                script.push(scroll(
                    &format!("{tag}-fwd{i}"),
                    &tabs_target(),
                    direction,
                    1,
                ));
            }
        };
        for (at, tabs) in &groups {
            if !reset_each {
                position(&mut script, &format!("g{at}"), *at);
            }
            for j in tabs {
                if reset_each {
                    position(&mut script, &format!("t{j}"), *at);
                }
                script.push(click(&format!("c{j}"), &tab_target(&names[*j])));
                script.push(snapshot(&format!("s{j}")));
            }
        }
        script
    };
    let mut c = run(&dir, "clicks", &plan_clicks(false), &root_args());
    if (0..MANY).any(|j| c.r(&format!("c{j}"))["ok"] != json!(true)) {
        c = run(&dir, "clicks-reset", &plan_clicks(true), &root_args());
    }
    assert_eq!(
        c.refused_kind("hidden"),
        "not_actionable",
        "a click on the offscreen tab `{}`",
        names[hidden]
    );
    for (line, resp) in c.script.iter().zip(&c.resp) {
        let v: Value = serde_json::from_str(line).expect("script line");
        if v.get("ui").is_some() && v["id"] != json!("hidden") && v["ui"] != json!("snapshot") {
            assert_eq!(resp["ok"], json!(true), "{line} → {}", short(resp));
            if v["ui"] != json!("frames") {
                assert_eq!(
                    resp["result"]["settled"],
                    json!(true),
                    "{line} did not settle within {SETTLE_CAP} frames"
                );
            }
        }
    }
    for (j, n) in names.iter().enumerate() {
        let what = format!("after the click on the tab `{n}`");
        let root = c.snap(&format!("s{j}"));
        assert_eq!(
            shown_current(root, &names, &what).as_deref(),
            Some(n.as_str()),
            "{what}: Title"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}
