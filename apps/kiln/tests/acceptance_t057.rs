//! T-057 acceptance tests: the project panel of `kiln script` (`tasks/T-057-ui-project-panel.md`,
//! Interface, Pinned, AC-1 to AC-10). Written by the implementer first, on `task/T-057-tests`,
//! before any implementation (odd-numbered medium task, METHODOLOGY §6 "Experiment arm").
//!
//! Every test drives the built `kiln` binary (`CARGO_BIN_EXE_kiln`) through `kiln script` with
//! temporary roots under `CARGO_TARGET_TMPDIR`; every spawned process (`kiln`, `git`) is killed
//! after [`TIMEOUT`] (D-028 (8)). The tests act with UI requests (`click`, `scroll`, `frames`),
//! find nodes by role and label (the toggle `Project`, the tree `Project files`, entries of role
//! `tree_item` by their path), and check what is shown against an independent source: a control
//! `project.tree` in the same session, the directory on disk, `git ls-files` of a fresh
//! repository where `docs/protocol/project.md` says the two agree, the session log, a naive model
//! of expanded directories, the generator's own tree, and `kiln_control::Host` replaying the log
//! (the one Rust API the spec allows, as oracle).
//!
//! Not pinned (golden rule 9, Pinned (f)): a whole snapshot, the panel's position or width, the
//! nesting or indentation of entries, any message text beyond its leading kind, the text after
//! `truncated`, which reads the editor view sends, the command list, `error_kinds`.
//!
//! Ruling Q-6 (D-181): these files use no Unix-only API. So AC-9 (a directory that cannot be
//! listed, mode 000) has no test here, and the cases of AC-5 that need a FIFO or a symbolic link
//! (and the `symlink` / `other` kinds in AC-1 and AC-3) have no fixture here: they cannot be made
//! without a Unix-only API. They go to T-079 with the platform-test registry (T-071).
#![cfg(test)]

use std::collections::{BTreeSet, HashMap};
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

/// The name of the session root in every case directory (AC-1 checks that it is never shown).
const ROOT: &str = "t057root";

/// `src/a.rs` of Pinned (g), lines short enough to be painted whole.
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
        .join("t057-acceptance")
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

fn mkdir(path: &Path) {
    fs::create_dir_all(path).expect("create fixture dir");
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

fn toggle_target() -> Value {
    json!({"role": "button", "label": "Project"})
}

fn tree_target() -> Value {
    json!({"role": "tree", "label": "Project files"})
}

fn item_target(path: &str) -> Value {
    json!({"role": "tree_item", "label": path})
}

fn click(id: &str, target: Value) -> String {
    ui(id, "click", json!({ "target": target }))
}

fn click_toggle(id: &str) -> String {
    click(id, toggle_target())
}

fn click_item(id: &str, path: &str) -> String {
    click(id, item_target(path))
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
    /// The response to the request with this id.
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
            "{}: request `{id}` refused: {r}",
            self.name
        );
        &r["result"]
    }

    /// An action that must have succeeded; its `result` (`frames`, `settled`).
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

    /// Every UI request of the script answered `ok:true` (no click on an offscreen or missing
    /// entry went unnoticed).
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

/// The panel's toggle: the one node of role `button` labelled `Project`.
fn the_toggle<'a>(root: &'a Value, what: &str) -> &'a Value {
    let found: Vec<&Value> = preorder(root)
        .into_iter()
        .filter(|n| role(n) == Some("button") && label(n) == Some("Project"))
        .collect();
    assert_eq!(
        found.len(),
        1,
        "{what}: expected exactly one node of role `button` labelled `Project` (the project \
         panel's toggle), found {}: {}",
        found.len(),
        short(root)
    );
    found[0]
}

/// The toggle's `toggled` state (it must have one).
fn toggle_state(root: &Value, what: &str) -> bool {
    let t = the_toggle(root, what);
    toggled(t).unwrap_or_else(|| panic!("{what}: the toggle has no `toggled`: {}", short(t)))
}

/// The node of role `tree` labelled `Project files`, if shown (at most one).
fn files_tree<'a>(root: &'a Value, what: &str) -> Option<&'a Value> {
    let found: Vec<&Value> = preorder(root)
        .into_iter()
        .filter(|n| role(n) == Some("tree") && label(n) == Some("Project files"))
        .collect();
    assert!(
        found.len() <= 1,
        "{what}: {} nodes of role `tree` labelled `Project files`",
        found.len()
    );
    found.first().copied()
}

/// The shown entries: the `tree_item` nodes below `Project files` in snapshot order. Panics if a
/// `tree_item` exists outside it.
fn shown<'a>(root: &'a Value, what: &str) -> Vec<&'a Value> {
    let everywhere = preorder(root)
        .into_iter()
        .filter(|n| role(n) == Some("tree_item"))
        .count();
    let items: Vec<&Value> = files_tree(root, what)
        .map(|t| {
            preorder(t)
                .into_iter()
                .skip(1)
                .filter(|n| role(n) == Some("tree_item"))
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(
        items.len(),
        everywhere,
        "{what}: a node of role `tree_item` lies outside `Project files`: {}",
        short(root)
    );
    items
}

fn item_label(n: &Value) -> String {
    label(n)
        .unwrap_or_else(|| panic!("a tree_item without a label: {}", short(n)))
        .to_owned()
}

fn shown_labels(root: &Value, what: &str) -> Vec<String> {
    shown(root, what).into_iter().map(item_label).collect()
}

/// The shown entry labelled `path`.
fn entry<'a>(root: &'a Value, path: &str, what: &str) -> &'a Value {
    shown(root, what)
        .into_iter()
        .find(|n| label(n) == Some(path))
        .unwrap_or_else(|| panic!("{what}: no shown entry labelled `{path}`: {}", short(root)))
}

/// The last segment of a root-relative path.
fn name_of(path: &str) -> &str {
    path.rsplit('/').next().expect("a segment")
}

/// The `Message` node's value.
fn message(root: &Value) -> String {
    let found = by_label(root, "Message");
    assert_eq!(found.len(), 1, "one node labelled Message: {}", short(root));
    value(found[0]).unwrap_or_default().to_owned()
}

/// The `Title` node's value.
fn title(root: &Value, what: &str) -> String {
    let found = by_label(root, "Title");
    assert_eq!(
        found.len(),
        1,
        "{what}: expected exactly one node labelled `Title`, found {}: {}",
        found.len(),
        short(root)
    );
    value(found[0]).unwrap_or_default().to_owned()
}

/// The one node labelled `Editor`, of role `multiline_text_input`.
fn editor<'a>(root: &'a Value, what: &str) -> &'a Value {
    let found = by_label(root, "Editor");
    assert_eq!(
        found.len(),
        1,
        "{what}: expected exactly one node labelled `Editor`, found {}: {}",
        found.len(),
        short(root)
    );
    assert_eq!(
        role(found[0]),
        Some("multiline_text_input"),
        "{what}: Editor role"
    );
    found[0]
}

fn editor_value<'a>(root: &'a Value, what: &str) -> &'a str {
    let ed = editor(root, what);
    value(ed).unwrap_or_else(|| panic!("{what}: Editor without a value: {}", short(ed)))
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

/// The nearest ancestor of role `scroll_view` of `target` (by identity) under `root`.
fn scroll_view_around<'a>(root: &'a Value, target: &Value) -> Option<&'a Value> {
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
    chain.pop();
    chain
        .into_iter()
        .rev()
        .find(|n| role(n) == Some("scroll_view"))
}

/// A name that begins with right-to-left text or an Arabic-Indic digit, whose rows `ui.md`
/// (Limits) does not guarantee (AC-3 (iii); T-066).
fn begins_rtl(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|c| ('\u{0590}'..='\u{08ff}').contains(&c))
}

// ---------------------------------------------------------------------------------------------
// `project.tree` results and the naive model of what is shown.
// ---------------------------------------------------------------------------------------------

/// One listed entry: root-relative path and kind.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    path: String,
    kind: String,
}

impl Entry {
    fn dir(&self) -> bool {
        self.kind == "dir"
    }
}

/// The entries of a `project.tree` result, in its order.
fn entries(result: &Value) -> Vec<Entry> {
    result["entries"]
        .as_array()
        .unwrap_or_else(|| panic!("project.tree result without entries: {}", short(result)))
        .iter()
        .map(|e| Entry {
            path: e["path"].as_str().expect("entry path").to_owned(),
            kind: e["kind"].as_str().expect("entry kind").to_owned(),
        })
        .collect()
}

/// The strict ancestors of a root-relative path (`a/b/c` → `a`, `a/b`).
fn ancestors(path: &str) -> Vec<&str> {
    path.match_indices('/').map(|(i, _)| &path[..i]).collect()
}

/// The entries of `all` one segment below `dir` (`None`: the root), in order.
fn children_of<'a>(all: &'a [Entry], dir: Option<&str>) -> Vec<&'a Entry> {
    all.iter()
        .filter(|e| ancestors(&e.path).last().copied() == dir)
        .collect()
}

/// What the model says is shown: an entry with its expanded state (directories only).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Want {
    path: String,
    dir: bool,
    expanded: bool,
}

/// The naive model (AC-2): the preorder of `all`, restricted to entries whose every ancestor
/// directory is expanded; nothing when the root is collapsed.
fn model_shown(all: &[Entry], root_open: bool, expanded: &BTreeSet<String>) -> Vec<Want> {
    if !root_open {
        return Vec::new();
    }
    all.iter()
        .filter(|e| ancestors(&e.path).iter().all(|a| expanded.contains(*a)))
        .map(|e| Want {
            path: e.path.clone(),
            dir: e.dir(),
            expanded: expanded.contains(&e.path),
        })
        .collect()
}

/// The shown entries equal `want` in order: labels, values (the name), and `toggled` (only on
/// directories, true when expanded).
fn check_shown(root: &Value, want: &[Want], what: &str) {
    let items = shown(root, what);
    let got: Vec<String> = items.iter().map(|n| item_label(n)).collect();
    let exp: Vec<String> = want.iter().map(|w| w.path.clone()).collect();
    assert_eq!(
        got, exp,
        "{what}: shown entries (labels, in snapshot order)"
    );
    for (n, w) in items.iter().zip(want) {
        assert_eq!(
            value(n),
            Some(name_of(&w.path)),
            "{what}: the value of `{}` is not its name",
            w.path
        );
        let t = w.dir.then_some(w.expanded);
        assert_eq!(
            toggled(n),
            t,
            "{what}: `toggled` of `{}` ({})",
            w.path,
            if w.dir {
                "a directory"
            } else {
                "not a directory"
            }
        );
    }
}

/// `project.tree {}` through `Host::on_disk` on `root` (an oracle outside the session, used to
/// plan the clicks before the session runs; the session's own control request is compared
/// with it).
fn host_tree(root: &Path) -> Vec<Entry> {
    let mut host = Host::on_disk(root).expect("Host::on_disk");
    let resp = host
        .handle_line(br#"{"id":1,"command":"project.tree"}"#)
        .expect("a response");
    let v: Value = serde_json::from_str(&resp).expect("JSON");
    assert_eq!(
        v["ok"],
        json!(true),
        "project.tree on {}: {resp}",
        root.display()
    );
    entries(&v["result"])
}

// ---------------------------------------------------------------------------------------------
// The log.
// ---------------------------------------------------------------------------------------------

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

/// Open documents as `(doc, path)`.
type Docs = Vec<(u64, String)>;

/// `(doc, path)` of a `doc.list` result.
fn doc_list(result: &Value) -> Docs {
    result["docs"]
        .as_array()
        .unwrap_or_else(|| panic!("doc.list without docs: {result}"))
        .iter()
        .map(|d| {
            (
                d["doc"].as_u64().expect("doc id"),
                d["path"].as_str().expect("doc path").to_owned(),
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// A deterministic generator (xorshift64*).
// ---------------------------------------------------------------------------------------------

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

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % (n as u64)).expect("fits")
    }
}

// ---------------------------------------------------------------------------------------------
// A script built step by step: a click on an entry that is `offscreen` is preceded by `scroll`
// requests, decided from a probe run of the script so far (a script is read whole at start, so
// the test cannot react within one run).
// ---------------------------------------------------------------------------------------------

struct Steps<'a> {
    dir: &'a Path,
    name: String,
    args: Vec<String>,
    lines: Vec<String>,
    scrolls: usize,
}

impl Steps<'_> {
    fn probe(&self) -> Sess {
        let mut lines = self.lines.clone();
        lines.push(snapshot("probe"));
        let args: Vec<&str> = self.args.iter().map(String::as_str).collect();
        let s = run(self.dir, &format!("{}-probe", self.name), &lines, &args);
        s.all_ui_ok();
        s
    }

    /// Append a click on the entry `path`, scrolling the panel first while it is `offscreen`:
    /// down when it lies after the first visible entry, else up; on `Project files` and on a
    /// visible entry in turn.
    fn click_entry(&mut self, id: &str, path: &str) {
        for attempt in 0..60 {
            let s = self.probe();
            let root = s.snap("probe");
            let what = format!("{} before `{id}`", self.name);
            let items = shown(root, &what);
            let at = items
                .iter()
                .position(|n| label(n) == Some(path))
                .unwrap_or_else(|| {
                    panic!("{what}: `{path}` is not shown before its click (the model shows it)")
                });
            if !offscreen(items[at]) {
                self.lines.push(click_item(id, path));
                return;
            }
            let first_visible = items.iter().position(|n| !offscreen(n));
            let direction = match first_visible {
                Some(v) if v < at => "down",
                _ => "up",
            };
            let target = match first_visible {
                Some(v) if attempt % 2 == 1 => item_target(&item_label(items[v])),
                _ => tree_target(),
            };
            self.lines.push(ui(
                &format!("{id}-scroll{attempt}"),
                "scroll",
                json!({"target": target, "direction": direction}),
            ));
            self.scrolls += 1;
        }
        panic!(
            "{}: `{path}` is still offscreen after 60 scroll steps",
            self.name
        );
    }
}

// =============================================================================================
// AC-1: the root's entries (oracle: a control `project.tree {}` in the same session).
// =============================================================================================

/// The root of Pinned (g) without the FIFO and the link (Q-6), plus a file named `Open` (P6).
fn ac1_fixture(root: &Path) {
    write(&root.join(".gitignore"), b"*.log\n");
    write(&root.join("a.log"), b"log\n");
    write(&root.join("target/t"), b"t\n");
    write(&root.join("src/a.rs"), b"fn main() {}\n");
    write(&root.join("dir with space/\u{e9}.txt"), b"e\n");
    write(&root.join("e\u{301}x.txt"), b"x\n");
    write(&root.join("\u{5e9}\u{5dc}\u{5d5}\u{5dd}.txt"), b"s\n");
    write(&root.join("Open"), b"not a button\n");
    mkdir(&root.join("empty"));
}

#[test]
fn ac1_click_on_project_lists_the_roots_entries() {
    let dir = case_dir("ac1");
    let root = dir.join(ROOT);
    ac1_fixture(&root);
    let script = vec![
        snapshot("s0"),
        click_toggle("c"),
        snapshot("s1"),
        ctl("pt", "project.tree", json!({})),
        click("open-label", json!({"label": "Open"})),
        click("open-role", json!({"role": "tree_item", "label": "Open"})),
        snapshot("s2"),
    ];
    let s = run(&dir, "s", &script, &["--root", ROOT, "src/a.rs"]);

    // Before the click: the toggle only, collapsed.
    let s0 = s.snap("s0");
    assert!(
        !toggle_state(s0, "before the click"),
        "before the click the toggle is `toggled: false`"
    );
    assert!(
        files_tree(s0, "before the click").is_none(),
        "before the click there is no `Project files`"
    );
    assert!(shown(s0, "before the click").is_empty());

    // After it: expanded, the root's entries as `project.tree {}` lists them.
    s.act("c");
    let s1 = s.snap("s1");
    assert!(
        toggle_state(s1, "after the click"),
        "after the click the toggle is `toggled: true`"
    );
    let all = entries(s.ok("pt"));
    let want: Vec<Want> = children_of(&all, None)
        .into_iter()
        .map(|e| Want {
            path: e.path.clone(),
            dir: e.dir(),
            expanded: false,
        })
        .collect();
    assert!(
        want.len() >= 6 && want.iter().any(|w| w.dir),
        "precondition: project.tree lists the fixture's root entries: {want:?}"
    );
    check_shown(s1, &want, "after the click");
    assert!(
        !shown_labels(s1, "s1")
            .iter()
            .any(|l| l == "a.log" || l == "target"),
        "ignored entries are not shown"
    );

    // Painted text of the entries (Pinned (f) F2), except for right-to-left names (ui.md Limits).
    for n in shown(s1, "s1") {
        let name = value(n).expect("value");
        if !begins_rtl(name) {
            assert_eq!(
                row_texts(n),
                [name.to_owned()],
                "the painted row of `{}` is not its name",
                item_label(n)
            );
        }
    }

    // `Project files` lies in a `scroll_view` with no shell node, no Title and no Editor; the
    // root's host path and name are shown nowhere.
    let tree = files_tree(s1, "s1").expect("Project files after the click");
    let sv = scroll_view_around(s1, tree)
        .unwrap_or_else(|| panic!("`Project files` is inside no scroll_view: {}", short(s1)));
    for n in preorder(sv) {
        if let Some(l) = label(n) {
            assert!(
                !["Path", "Open", "Message", "Documents", "Title", "Editor"].contains(&l)
                    || role(n) == Some("tree_item"),
                "the scroll_view around `Project files` contains `{l}` ({:?})",
                role(n)
            );
        }
    }
    title(s1, "precondition: a document is open");
    editor(s1, "precondition: a document is open");
    let host_root = fs::canonicalize(&root).expect("canonical root");
    for n in preorder(s1) {
        for text in [label(n), value(n)].into_iter().flatten() {
            assert!(
                !text.contains(ROOT) && !text.contains(&*host_root.to_string_lossy()),
                "a node shows the root's name or host path: {}",
                short(n)
            );
        }
    }

    // P6: with the panel expanded, a label alone can match an entry; `role` disambiguates.
    assert_eq!(
        s.r("open-label")["error"]["kind"],
        json!("ambiguous_target"),
        "{{\"label\":\"Open\"}} with a root file `Open` shown: {}",
        s.r("open-label")
    );
    s.act("open-role");
    let _ = fs::remove_dir_all(&dir);
}

// =============================================================================================
// AC-2: random expand and collapse (oracle: naive model plus the log).
// =============================================================================================

fn ac2_fixture(root: &Path) {
    for f in ["alpha/a1.txt", "alpha/a2.txt", "alpha/beta/b1.txt"] {
        write(&root.join(f), b"x\n");
    }
    for f in ["alpha/beta/gamma/g1.txt", "alpha/beta/gamma/g2.txt"] {
        write(&root.join(f), b"x\n");
    }
    mkdir(&root.join("alpha/empty"));
    for k in 1..=6 {
        write(&root.join(format!("delta/d{k}.txt")), b"x\n");
    }
    write(&root.join("delta/eps/e1.txt"), b"x\n");
    write(&root.join("delta/eps/zeta/z.txt"), b"x\n");
    for k in 1..=12 {
        write(&root.join(format!("omega/o{k:02}.txt")), b"x\n");
    }
    for k in 1..=10 {
        write(&root.join(format!("r{k:02}.txt")), b"x\n");
    }
}

#[test]
fn ac2_expand_and_collapse_follow_the_naive_model() {
    let dir = case_dir("ac2");
    let root = dir.join(ROOT);
    ac2_fixture(&root);
    let all = host_tree(&root);
    let mut scrolls = 0;
    let mut collapses_with_descendants = 0;
    for seed in [0x0057_0201_u64, 0x0057_0202] {
        let mut rng = Rng(seed);
        let mut st = Steps {
            dir: &dir,
            name: format!("seed-{seed:x}"),
            args: ["--root", ROOT, "--size", "800x480"]
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
            lines: vec![ctl("pt", "project.tree", json!({})), snapshot("s-start")],
            scrolls: 0,
        };
        let mut root_open = false;
        let mut expanded: BTreeSet<String> = BTreeSet::new();
        let mut wants: Vec<(bool, Vec<Want>)> = Vec::new();
        let mut reads: Vec<Value> = Vec::new();
        for k in 0..16 {
            let id = format!("c{k}");
            let dirs: Vec<String> = model_shown(&all, root_open, &expanded)
                .into_iter()
                .filter(|w| w.dir)
                .map(|w| w.path)
                .collect();
            if !root_open || dirs.is_empty() || rng.below(8) == 0 {
                st.lines.push(click_toggle(&id));
                root_open = !root_open;
                if root_open {
                    reads.push(json!({}));
                } else {
                    expanded.clear();
                }
            } else {
                let d = dirs[rng.below(dirs.len())].clone();
                st.click_entry(&id, &d);
                if expanded.contains(&d) {
                    let below = format!("{d}/");
                    if expanded.iter().any(|p| p.starts_with(&below)) {
                        collapses_with_descendants += 1;
                    }
                    expanded.retain(|p| *p != d && !p.starts_with(&below));
                } else {
                    expanded.insert(d.clone());
                    reads.push(json!({ "path": d }));
                }
            }
            st.lines.push(snapshot(&format!("s{k}")));
            wants.push((root_open, model_shown(&all, root_open, &expanded)));
        }
        scrolls += st.scrolls;
        let args: Vec<&str> = st.args.iter().map(String::as_str).collect();
        let s = run(&dir, &st.name, &st.lines, &args);
        s.all_ui_ok();
        assert_eq!(
            entries(s.ok("pt")),
            all,
            "{}: project.tree in the session",
            s.name
        );
        assert!(!toggle_state(s.snap("s-start"), "start"));
        assert!(shown(s.snap("s-start"), "start").is_empty());
        for (k, (open, want)) in wants.iter().enumerate() {
            let what = format!("{} after step {k}", s.name);
            let snap = s.snap(&format!("s{k}"));
            assert_eq!(toggle_state(snap, &what), *open, "{what}: the toggle");
            check_shown(snap, want, &what);
        }
        // One `project.tree` read per expand click, with the params of P2; none per collapse;
        // nothing else (no document is open in this session).
        let extra = ui_logged(&s);
        assert_eq!(
            logged(&extra, "project.tree"),
            reads,
            "{}: the panel's project.tree reads",
            s.name
        );
        assert_eq!(
            extra.len(),
            reads.len(),
            "{}: the log holds requests other than the panel's reads: {extra:?}",
            s.name
        );
    }
    assert!(
        scrolls > 0,
        "precondition: some click needed a scroll first at 800x480"
    );
    assert!(
        collapses_with_descendants > 0,
        "precondition: some collapse forgot an expanded descendant"
    );
    let _ = fs::remove_dir_all(&dir);
}

// =============================================================================================
// AC-3: generated trees (oracle: project.tree, git, the generator's own tree).
// =============================================================================================

/// The stated alphabet (AC-3): pieces of names. ASCII letters and digits, precomposed `é`,
/// `e`+U+0301, a zero-width joiner, `ffi`, CJK, an emoji, Hebrew, Arabic, an Arabic-Indic digit.
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

/// A generated directory.
#[derive(Debug, Default)]
struct GenDir {
    files: Vec<String>,
    dirs: Vec<(String, GenDir)>,
    gitignore: Vec<String>,
}

/// A name from the alphabet: one to three pieces, never starting with the joiner.
fn gen_name(rng: &mut Rng) -> String {
    let n = 1 + rng.below(3);
    let mut s = String::new();
    for k in 0..n {
        if k > 0 {
            s.push_str(JOINS[rng.below(JOINS.len())]);
        }
        let piece = PIECES[rng.below(PIECES.len())];
        let piece = if k == 0 && piece == "\u{200d}" {
            "a"
        } else {
            piece
        };
        s.push_str(piece);
    }
    s
}

/// A key equal for two names that a file system could take for one: ignoring case and with
/// `e`+U+0301 composed (the only normalization-sensitive pair of the alphabet).
fn fold(name: &str) -> String {
    name.replace("e\u{301}", "\u{e9}").to_lowercase()
}

/// A generated directory at `depth` (the root is 0; entries of depth ≤ 4); `budget` bounds the
/// whole tree's entries.
fn gen_dir(rng: &mut Rng, depth: usize, budget: &mut usize) -> GenDir {
    let mut d = GenDir::default();
    let n = if depth == 0 {
        6 + rng.below(3)
    } else {
        rng.below(9)
    };
    let mut keys: BTreeSet<String> = BTreeSet::new();
    keys.insert(fold(".gitignore"));
    for _ in 0..n {
        if *budget == 0 {
            break;
        }
        let name = gen_name(rng);
        if !keys.insert(fold(&name)) {
            continue;
        }
        *budget -= 1;
        if depth < 3 && rng.below(3) == 0 {
            let sub = gen_dir(rng, depth + 1, budget);
            d.dirs.push((name, sub));
        } else {
            d.files.push(name);
        }
    }
    if d.files.len() + d.dirs.len() < 8 && rng.below(3) == 0 {
        d.gitignore = gen_patterns(rng, &d);
    }
    d
}

/// Every name in the subtree of `d` (files and directories).
fn subtree_names(d: &GenDir, out: &mut Vec<String>) {
    out.extend(d.files.iter().cloned());
    for (name, sub) in &d.dirs {
        out.push(name.clone());
        subtree_names(sub, out);
    }
}

/// One to three patterns of the forms `docs/protocol/project.md` guarantees (`*.x`, `/a`, `b/`,
/// `!c`, `**/d`, `e/**`), naming entries of `d`'s subtree.
fn gen_patterns(rng: &mut Rng, d: &GenDir) -> Vec<String> {
    let mut names = Vec::new();
    subtree_names(d, &mut names);
    if names.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for _ in 0..1 + rng.below(3) {
        match rng.below(6) {
            0 => {
                let with_ext: Vec<&String> = d.files.iter().filter(|f| f.contains('.')).collect();
                if let Some(f) = with_ext.get(rng.below(with_ext.len().max(1))) {
                    let ext = f.rsplit('.').next().expect("ext");
                    out.push(format!("*.{ext}"));
                    if rng.below(2) == 0 {
                        out.push(format!("!{f}"));
                    }
                }
            }
            1 => {
                let all: Vec<&String> = d
                    .files
                    .iter()
                    .chain(d.dirs.iter().map(|(n, _)| n))
                    .collect();
                if let Some(n) = all.get(rng.below(all.len().max(1))) {
                    out.push(format!("/{n}"));
                }
            }
            2 => {
                if let Some((n, _)) = d.dirs.get(rng.below(d.dirs.len().max(1))) {
                    out.push(format!("{n}/"));
                }
            }
            3 => {
                if let Some((n, _)) = d.dirs.get(rng.below(d.dirs.len().max(1))) {
                    out.push(format!("{n}/**"));
                }
            }
            4 => {
                let n = &names[rng.below(names.len())];
                out.push(format!("!{n}"));
            }
            _ => {
                let n = &names[rng.below(names.len())];
                out.push(format!("**/{n}"));
            }
        }
    }
    out
}

/// The number of entries the generator made (`.gitignore` files included).
fn count(d: &GenDir) -> usize {
    d.files.len()
        + usize::from(!d.gitignore.is_empty())
        + d.dirs.iter().map(|(_, sub)| 1 + count(sub)).sum::<usize>()
}

fn materialize(d: &GenDir, at: &Path, counter: &mut usize) {
    mkdir(at);
    for f in &d.files {
        *counter += 1;
        write(&at.join(f), format!("file {counter}\n").as_bytes());
    }
    if !d.gitignore.is_empty() {
        let mut text = d.gitignore.join("\n");
        text.push('\n');
        write(&at.join(".gitignore"), text.as_bytes());
    }
    for (name, sub) in &d.dirs {
        materialize(sub, &at.join(name), counter);
    }
}

/// `git` with no global or system configuration, under the hang detector.
fn git(args: &[&str], dir: &Path) -> Vec<u8> {
    let mut cmd = Command::new("git");
    cmd.args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_CEILING_DIRECTORIES");
    let out = run_cmd(cmd, &format!("git {}", args.join(" ")));
    assert_eq!(
        out.code,
        0,
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

/// The paths `git ls-files --cached --others --exclude-standard` lists in a fresh repository of
/// a copy of `root` (`project.md`'s stated agreement).
fn git_files(root: &Path, repo: &Path) -> BTreeSet<String> {
    let _ = fs::remove_dir_all(repo);
    copy_dir(root, repo);
    git(&["init", "-q"], repo);
    let out = git(
        &[
            "-c",
            "core.excludesFile=",
            "-c",
            "core.precomposeUnicode=false",
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
        repo,
    );
    let set = out
        .split(|b| *b == 0)
        .filter(|p| !p.is_empty())
        .map(|p| String::from_utf8(p.to_vec()).expect("git path is UTF-8"))
        .collect();
    let _ = fs::remove_dir_all(repo);
    set
}

#[test]
fn ac3_generated_trees_fully_expanded() {
    let dir = case_dir("ac3");
    let mut rows_checked = 0;
    let mut rows_skipped = 0;
    let mut ignored_somewhere = false;
    for k in 0..40u64 {
        let seed = 0x0057_0300 + k;
        let mut rng = Rng(seed * 0x9e37_79b9 + 1);
        let mut budget = 70;
        let generated = gen_dir(&mut rng, 0, &mut budget);
        let root = dir.join(ROOT);
        let _ = fs::remove_dir_all(&root);
        let mut counter = 0;
        materialize(&generated, &root, &mut counter);
        let planned = host_tree(&root);
        let dirs: Vec<&Entry> = planned.iter().filter(|e| e.dir()).collect();
        let files: Vec<&Entry> = planned.iter().filter(|e| e.kind == "file").collect();
        let what = format!(
            "seed {seed:#x} ({} generated, {} listed: {} dirs, {} files)",
            count(&generated),
            planned.len(),
            dirs.len(),
            files.len()
        );
        assert!(
            files.len() >= 5,
            "{what}: precondition: the generator made at least 5 listed files"
        );
        let mut script = vec![click_toggle("c-root")];
        for (i, d) in dirs.iter().enumerate() {
            script.push(click_item(&format!("c-d{i}"), &d.path));
        }
        script.push(snapshot("s-all"));
        script.push(ctl("pt", "project.tree", json!({})));
        let mut chosen: Vec<String> = Vec::new();
        while chosen.len() < 5 {
            let f = files[rng.below(files.len())].path.clone();
            if !chosen.contains(&f) {
                chosen.push(f);
            }
        }
        for (i, f) in chosen.iter().enumerate() {
            script.push(click_item(&format!("o{i}"), f));
            script.push(snapshot(&format!("so{i}")));
            script.push(ctl(&format!("l{i}"), "doc.list", json!({})));
            script.push(ctl(&format!("t{i}"), "doc.text", json!({"doc": i + 1})));
        }
        let s = run(
            &dir,
            &format!("seed-{seed:x}"),
            &script,
            &["--root", ROOT, "--size", "1280x8192"],
        );
        s.all_ui_ok();
        let all = entries(s.ok("pt"));
        assert_eq!(all, planned, "{what}: project.tree in the session");
        if s.ok("pt")["ignored"].as_u64().expect("ignored") > 0 {
            ignored_somewhere = true;
        }

        // (i) every entry shown, in project.tree's order, directories expanded.
        let snap = s.snap("s-all");
        let want: Vec<Want> = all
            .iter()
            .map(|e| Want {
                path: e.path.clone(),
                dir: e.dir(),
                expanded: e.dir(),
            })
            .collect();
        check_shown(snap, &want, &what);

        // (ii) files and links equal git's list.
        let listed: BTreeSet<String> = all
            .iter()
            .filter(|e| e.kind == "file" || e.kind == "symlink")
            .map(|e| e.path.clone())
            .collect();
        let git_set = git_files(&root, &dir.join("git-copy"));
        assert_eq!(
            listed, git_set,
            "{what}: file entries vs git ls-files; .gitignore files: {:?}",
            generated.gitignore
        );

        // (iii) painted rows, except for names beginning with right-to-left text.
        for n in shown(snap, &what) {
            let name = value(n).expect("value");
            if begins_rtl(name) {
                rows_skipped += 1;
                continue;
            }
            rows_checked += 1;
            assert_eq!(
                row_texts(n),
                [name.to_owned()],
                "{what}: the painted row of `{}`",
                item_label(n)
            );
        }

        // (iv) clicks on 5 files open them and make each current.
        for (i, f) in chosen.iter().enumerate() {
            let w = format!("{what}: after the click on `{f}`");
            s.act(&format!("o{i}"));
            let list = doc_list(s.ok(&format!("l{i}")));
            assert_eq!(
                list.last(),
                Some(&(i as u64 + 1, f.clone())),
                "{w}: doc.list {list:?}"
            );
            let so = s.snap(&format!("so{i}"));
            let t = title(so, &w);
            assert!(t.starts_with(f.as_str()), "{w}: Title `{t}`");
            assert_eq!(
                editor_value(so, &w),
                s.text(&format!("t{i}")),
                "{w}: Editor value vs doc.text"
            );
            assert_eq!(message(so), "", "{w}: Message");
        }
        let _ = fs::remove_dir_all(&root);
    }
    assert!(
        rows_checked > 100 && rows_skipped > 0,
        "precondition: rows checked {rows_checked}, skipped {rows_skipped}"
    );
    assert!(
        ignored_somewhere,
        "precondition: some .gitignore pattern ignored an entry"
    );
    let _ = fs::remove_dir_all(&dir);
}

// =============================================================================================
// AC-4: a click on a file opens it (oracle: registry state, replay).
// =============================================================================================

#[test]
fn ac4_click_on_a_file_opens_it_and_makes_it_current() {
    let dir = case_dir("ac4");
    let root = dir.join(ROOT);
    write(&root.join("src/a.rs"), A_RS.as_bytes());
    write(&root.join("b.txt"), b"b line\nsecond\n");
    write(&root.join("docs/c.md"), b"# c\n");
    let initial = dir.join("initial");
    copy_dir(&root, &initial);
    let script = vec![
        snapshot("s0"),
        click_toggle("c0"),
        click_item("c1", "src"),
        click_item("o1", "src/a.rs"),
        snapshot("s1"),
        ctl("l1", "doc.list", json!({})),
        ctl("t1", "doc.text", json!({"doc": 1})),
        click_item("o2", "b.txt"),
        snapshot("s2"),
        ctl("l2", "doc.list", json!({})),
        ctl("t2", "doc.text", json!({"doc": 2})),
        click_item("o3", "src/a.rs"),
        snapshot("s3"),
        ctl("l3", "doc.list", json!({})),
        click_item("o4", "b.txt"),
        snapshot("s4"),
        ctl("l4", "doc.list", json!({})),
    ];
    // A refused start path sets `Message`, so that its clearing is observed.
    let s = run(&dir, "s", &script, &["--root", ROOT, "missing.rs"]);
    s.all_ui_ok();
    assert!(
        message(s.snap("s0")).starts_with("not_found"),
        "precondition: Message shows the refused start path"
    );
    let a = (1, "src/a.rs".to_owned());
    let b = (2, "b.txt".to_owned());
    let steps: [(&str, &str, &str, Docs); 4] = [
        ("s1", "l1", "src/a.rs", vec![a.clone()]),
        ("s2", "l2", "b.txt", vec![a.clone(), b.clone()]),
        ("s3", "l3", "src/a.rs", vec![a.clone(), b.clone()]),
        ("s4", "l4", "b.txt", vec![a.clone(), b.clone()]),
    ];
    for (snap, list, path, docs) in &steps {
        let what = format!("after the click on `{path}` ({snap})");
        assert_eq!(doc_list(s.ok(list)), *docs, "{what}: doc.list");
        let root_node = s.snap(snap);
        let t = title(root_node, &what);
        assert!(t.starts_with(path), "{what}: Title `{t}`");
        assert_eq!(message(root_node), "", "{what}: Message is cleared");
    }
    assert_eq!(editor_value(s.snap("s1"), "s1"), s.text("t1"));
    assert_eq!(s.text("t1"), A_RS);
    assert_eq!(editor_value(s.snap("s2"), "s2"), s.text("t2"));
    assert_eq!(editor_value(s.snap("s3"), "s3"), s.text("t1"));

    // Each click logged its `doc.open`, with the entry's label as `path`.
    let extra = ui_logged(&s);
    let opens = logged(&extra, "doc.open");
    assert_eq!(
        opens,
        [
            json!({"path": "src/a.rs"}),
            json!({"path": "b.txt"}),
            json!({"path": "src/a.rs"}),
            json!({"path": "b.txt"}),
        ],
        "the UI's doc.open requests"
    );

    // The log replayed on a copy of the initial root gives the same doc.list.
    let copy = dir.join("replay");
    copy_dir(&initial, &copy);
    let mut host = replay(&copy, &s.log_entries());
    let list = host_req(&mut host, "doc.list", json!({}));
    assert_eq!(
        doc_list(&list["result"]),
        doc_list(s.ok("l4")),
        "doc.list after replaying the log"
    );
    let _ = fs::remove_dir_all(&dir);
}

// =============================================================================================
// AC-5: refusals (oracle: a control doc.open of the same path in a second session).
// =============================================================================================

#[test]
fn ac5_a_refused_open_changes_nothing_and_shows_its_kind() {
    let dir = case_dir("ac5");
    let root = dir.join(ROOT);
    write(&root.join("a.rs"), A_RS.as_bytes());
    write(&root.join("bad.txt"), b"ok\xff\xfe\n");
    // The oracle: a control doc.open of the same path in a second session.
    let o = run(
        &dir,
        "oracle",
        &[ctl("o", "doc.open", json!({"path": "bad.txt"}))],
        &root_args(),
    );
    let r = o.r("o");
    assert_eq!(
        r["ok"],
        json!(false),
        "precondition: bad.txt is refused: {r}"
    );
    let kind = r["error"]["kind"].as_str().expect("kind").to_owned();
    assert_eq!(kind, "decode_error", "precondition: invalid UTF-8");

    let script = vec![
        click_toggle("c0"),
        click_item("oa", "a.rs"),
        ctl("l0", "doc.list", json!({})),
        snapshot("s0"),
        click_item("ob", "bad.txt"),
        snapshot("s1"),
        ctl("l1", "doc.list", json!({})),
    ];
    let s = run(&dir, "s", &script, &root_args());
    s.all_ui_ok();
    assert_eq!(
        s.act("ob")["settled"],
        json!(true),
        "the refused open is answered within the action's settling"
    );
    assert_eq!(doc_list(s.ok("l0")), [(1, "a.rs".to_owned())]);
    assert_eq!(
        doc_list(s.ok("l1")),
        doc_list(s.ok("l0")),
        "doc.list after the refused open"
    );
    let s1 = s.snap("s1");
    let m = message(s1);
    assert!(
        m.starts_with(&kind),
        "Message `{m}` does not begin with `{kind}`"
    );
    assert!(
        message(s.snap("s0")).is_empty(),
        "precondition: Message empty before"
    );
    let t = title(s1, "after the refusal");
    assert!(t.starts_with("a.rs"), "the current document stays: `{t}`");
    // The entry stays shown; nothing else changed in the panel.
    assert_eq!(
        shown_labels(s1, "after the refusal"),
        shown_labels(s.snap("s0"), "before the refusal")
    );
    let opens = logged(&s.log_entries(), "doc.open");
    assert_eq!(
        opens,
        [json!({"path": "a.rs"})],
        "no doc.open of the refused path is in the log"
    );
    let _ = fs::remove_dir_all(&dir);
}

// =============================================================================================
// AC-6: idle frames and reads (oracle: log comparison).
// =============================================================================================

/// (a) A script of control requests only, with the panel present, writes `Host::log_script()`.
#[test]
fn ac6a_control_requests_only_log_as_host() {
    let dir = case_dir("ac6a");
    let root = dir.join(ROOT);
    write(&root.join("src/a.rs"), A_RS.as_bytes());
    write(&root.join("b.txt"), b"b\n");
    let oracle = dir.join("oracle");
    copy_dir(&root, &oracle);
    let controls = vec![
        ctl("1", "project.tree", json!({})),
        ctl("2", "doc.open", json!({"path": "src/a.rs"})),
        ctl(
            "3",
            "doc.insert",
            json!({"doc": 1, "offset": 0, "text": "// x\n"}),
        ),
        ctl("4", "project.tree", json!({"path": "src"})),
        ctl("5", "doc.open", json!({"path": "missing.rs"})),
        ctl("6", "doc.save", json!({"doc": 1})),
        ctl("7", "doc.list", json!({})),
    ];
    let mut host = Host::on_disk(&oracle).expect("Host::on_disk");
    let expected: Vec<String> = controls
        .iter()
        .map(|l| host.handle_line(l.as_bytes()).expect("response"))
        .collect();
    let mut script = controls.clone();
    script.push(snapshot("s"));
    let s = run(&dir, "s", &script, &root_args());
    // The panel is present (collapsed) in the session.
    let snap = s.snap("s");
    assert!(
        !toggle_state(snap, "a control-only script"),
        "the panel is collapsed"
    );
    for (i, e) in expected.iter().enumerate() {
        let got: Value = s.resp[i].clone();
        let want: Value = serde_json::from_str(e).expect("JSON");
        assert_eq!(got, want, "response {i}");
    }
    assert_eq!(s.log, host.log_script(), "--log vs Host::log_script()");
    let _ = fs::remove_dir_all(&dir);
}

/// (b) 600 frames after an expanded panel and an opened file add nothing to the log.
#[test]
fn ac6b_idle_frames_after_the_panel_add_nothing() {
    let dir = case_dir("ac6b");
    let root = dir.join(ROOT);
    write(&root.join("src/a.rs"), A_RS.as_bytes());
    let prefix = vec![
        click_toggle("c0"),
        click_item("c1", "src"),
        click_item("o", "src/a.rs"),
    ];
    let mut with = prefix.clone();
    with.push(frames("idle", 600));
    with.push(snapshot("s"));
    let mut without = prefix;
    without.push(snapshot("s"));
    let a = run(&dir, "with", &with, &root_args());
    let b = run(&dir, "without", &without, &root_args());
    a.all_ui_ok();
    b.all_ui_ok();
    let extra = ui_logged(&b);
    assert_eq!(
        logged(&extra, "project.tree"),
        [json!({}), json!({"path": "src"})],
        "precondition: the panel read the root and src"
    );
    assert_eq!(
        logged(&extra, "doc.open"),
        [json!({"path": "src/a.rs"})],
        "precondition: the click opened the file"
    );
    assert!(title(a.snap("s"), "with").starts_with("src/a.rs"));
    assert_eq!(a.log, b.log, "600 idle frames added to the log");
    let _ = fs::remove_dir_all(&dir);
}

/// (c) Expand, collapse, expand: two `project.tree {}` reads and nothing else.
#[test]
fn ac6c_collapse_sends_nothing_expand_reads_again() {
    let dir = case_dir("ac6c");
    let root = dir.join(ROOT);
    write(&root.join("src/a.rs"), A_RS.as_bytes());
    write(&root.join("b.txt"), b"b\n");
    let script = vec![
        click_toggle("c1"),
        snapshot("s1"),
        frames("f1", 100),
        click_toggle("c2"),
        snapshot("s2"),
        frames("f2", 100),
        click_toggle("c3"),
        snapshot("s3"),
        frames("f3", 100),
    ];
    let s = run(&dir, "s", &script, &root_args());
    s.all_ui_ok();
    assert!(toggle_state(s.snap("s1"), "s1"));
    assert!(!toggle_state(s.snap("s2"), "s2"));
    assert!(shown(s.snap("s2"), "s2").is_empty(), "collapsed: no entry");
    assert!(
        files_tree(s.snap("s2"), "s2").is_none(),
        "collapsed: no Project files"
    );
    assert!(toggle_state(s.snap("s3"), "s3"));
    assert_eq!(
        shown_labels(s.snap("s3"), "s3"),
        ["b.txt", "src"],
        "after the second expand"
    );
    let log = s.log_entries();
    assert_eq!(
        log.iter()
            .map(|e| (e["command"].clone(), params_of(e)))
            .collect::<Vec<_>>(),
        [
            (json!("project.tree"), json!({})),
            (json!("project.tree"), json!({}))
        ],
        "the log of expand, collapse, expand"
    );
    let _ = fs::remove_dir_all(&dir);
}

// =============================================================================================
// AC-7: fresh reads, no watching (oracle: project.tree in the same session).
// =============================================================================================

#[test]
fn ac7_each_expand_reads_afresh_and_shows_that_read() {
    let dir = case_dir("ac7");
    let root = dir.join(ROOT);
    for f in ["src/a.rs", "src/b.rs", "src/sub/c.rs", "README.md", "z.txt"] {
        write(&root.join(f), b"x\n");
    }
    let script = vec![
        click_toggle("e0"),
        ctl("p0", "project.tree", json!({})),
        snapshot("s0"),
        frames("f0", 600),
        click_item("e1", "src"),
        ctl("p1", "project.tree", json!({"path": "src"})),
        snapshot("s1"),
        frames("f1", 600),
        click_item("c1", "src"),
        frames("f2", 600),
        snapshot("sc"),
        click_item("e2", "src"),
        ctl("p2", "project.tree", json!({"path": "src"})),
        snapshot("s2"),
        frames("f3", 600),
    ];
    let s = run(&dir, "s", &script, &root_args());
    s.all_ui_ok();
    // Each expand logs its own read, right before the control request that follows it, and
    // nothing else is logged, however many frames run.
    let log: Vec<(Value, Value)> = s
        .log_entries()
        .iter()
        .map(|e| (e["command"].clone(), params_of(e)))
        .collect();
    let t = |p: Value| (json!("project.tree"), p);
    assert_eq!(
        log,
        [
            t(json!({})),
            t(json!({})),
            t(json!({"path": "src"})),
            t(json!({"path": "src"})),
            t(json!({"path": "src"})),
            t(json!({"path": "src"})),
        ],
        "the log: one read per expand, each followed by the script's control request"
    );
    // The shown children equal those of the control response that follows each expand.
    let p0 = entries(s.ok("p0"));
    let roots: Vec<String> = children_of(&p0, None)
        .iter()
        .map(|e| e.path.clone())
        .collect();
    assert_eq!(shown_labels(s.snap("s0"), "s0"), roots, "root entries");
    for (snap, p) in [("s1", "p1"), ("s2", "p2")] {
        let listed = entries(s.ok(p));
        let kids: Vec<String> = children_of(&listed, Some("src"))
            .iter()
            .map(|e| e.path.clone())
            .collect();
        assert_eq!(kids, ["src/a.rs", "src/b.rs", "src/sub"], "precondition");
        let labels = shown_labels(s.snap(snap), snap);
        let at = labels
            .iter()
            .position(|l| l == "src")
            .expect("src is shown");
        assert_eq!(
            labels[at + 1..at + 1 + kids.len()],
            kids[..],
            "{snap}: the children shown after `src`"
        );
        assert_eq!(
            labels.len(),
            roots.len() + kids.len(),
            "{snap}: shown entries {labels:?}"
        );
        assert_eq!(toggled(entry(s.snap(snap), "src", snap)), Some(true));
    }
    assert_eq!(shown_labels(s.snap("sc"), "sc"), roots, "collapsed src");
    assert_eq!(toggled(entry(s.snap("sc"), "src", "sc")), Some(false));
    let _ = fs::remove_dir_all(&dir);
}

// =============================================================================================
// AC-8: a truncated listing (oracle: project.tree's rule 6).
// =============================================================================================

/// The marker nodes in `Project files`: role `label`, no `label`, value beginning `truncated`.
fn markers<'a>(root: &'a Value, what: &str) -> Vec<&'a Value> {
    files_tree(root, what)
        .map(|t| {
            preorder(t)
                .into_iter()
                .filter(|n| {
                    role(n) == Some("label")
                        && label(n).is_none()
                        && value(n).is_some_and(|v| v.starts_with("truncated"))
                })
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn ac8_a_truncated_root_listing_ends_with_a_marker() {
    let dir = case_dir("ac8");
    let root = dir.join(ROOT);
    mkdir(&root.join("a"));
    for k in 0..50_001 {
        fs::write(root.join("a").join(format!("f{k:05}")), b"").expect("write a/f…");
    }
    write(&root.join("z.txt"), b"z\n");
    let script = vec![
        click_toggle("c"),
        snapshot("s"),
        ctl("pt", "project.tree", json!({})),
    ];
    let s = run(&dir, "s", &script, &root_args());
    s.all_ui_ok();
    let pt = s.ok("pt");
    assert_eq!(pt["truncated"], json!(true), "precondition: truncated");
    let listed = entries(pt);
    let roots: Vec<String> = children_of(&listed, None)
        .iter()
        .map(|e| e.path.clone())
        .collect();
    assert_eq!(
        roots,
        ["a"],
        "precondition: no root entry after `a` is listed"
    );
    let snap = s.snap("s");
    assert_eq!(shown_labels(snap, "s"), ["a"], "the shown root entries");
    assert_eq!(toggled(entry(snap, "a", "s")), Some(false));
    // In snapshot order, the marker follows `a`, and nothing shown follows the marker.
    let tree = files_tree(snap, "s").expect("Project files");
    let order: Vec<&Value> = preorder(tree)
        .into_iter()
        .filter(|n| {
            role(n) == Some("tree_item")
                || (role(n) == Some("label")
                    && label(n).is_none()
                    && value(n).is_some_and(|v| v.starts_with("truncated")))
        })
        .collect();
    assert_eq!(order.len(), 2, "`a` and one marker: {}", short(tree));
    assert_eq!(label(order[0]), Some("a"));
    assert_eq!(role(order[1]), Some("label"), "the marker follows `a`");
    assert!(
        preorder(snap)
            .iter()
            .all(|n| label(n) != Some("z.txt") && value(n) != Some("z.txt")),
        "z.txt is not shown"
    );
    let _ = fs::remove_dir_all(&dir);

    // A small tree has no marker.
    let dir = case_dir("ac8-small");
    let root = dir.join(ROOT);
    write(&root.join("a/f"), b"");
    write(&root.join("z.txt"), b"z\n");
    let script = vec![
        click_toggle("c"),
        click_item("ca", "a"),
        snapshot("s"),
        ctl("pt", "project.tree", json!({})),
    ];
    let s = run(&dir, "s", &script, &root_args());
    s.all_ui_ok();
    assert_eq!(s.ok("pt")["truncated"], json!(false));
    assert_eq!(shown_labels(s.snap("s"), "small"), ["a", "a/f", "z.txt"]);
    assert!(
        markers(s.snap("s"), "small").is_empty(),
        "a small tree shows no `truncated` marker"
    );
    let _ = fs::remove_dir_all(&dir);
}

// =============================================================================================
// AC-10: F1, the shell fix (oracle: snapshot walk, doc.line).
// =============================================================================================

#[test]
fn ac10_panel_text_belongs_to_panel_nodes_and_rows_equal_text() {
    let dir = case_dir("ac10");
    let root = dir.join(ROOT);
    write(&root.join("src/a.rs"), A_RS.as_bytes());
    write(&root.join("notes.txt"), b"n\n");
    write(&root.join("docs/guide.md"), b"g\n");
    let script = vec![
        click_toggle("c0"),
        click_item("c1", "src"),
        click_item("c2", "docs"),
        snapshot("s"),
        ctl("l0", "doc.line", json!({"doc": 1, "line": 0})),
        ctl("l1", "doc.line", json!({"doc": 1, "line": 1})),
        ctl("l2", "doc.line", json!({"doc": 1, "line": 2})),
    ];
    let s = run(&dir, "s", &script, &["--root", ROOT, "src/a.rs"]);
    s.all_ui_ok();
    let snap = s.snap("s");
    // The toggle paints exactly `Project`, each entry exactly its name.
    assert_eq!(
        row_texts(the_toggle(snap, "s")),
        ["Project"],
        "the toggle's painted row"
    );
    let items = shown(snap, "s");
    assert_eq!(items.len(), 5, "precondition: five entries shown");
    for n in &items {
        assert_eq!(
            row_texts(n),
            [value(n).expect("value").to_owned()],
            "the painted row of `{}`",
            item_label(n)
        );
    }
    // No scroll_view that holds `Editor`, and no node around the panel's entries, takes text.
    let ed = editor(snap, "s");
    let mut at = scroll_view_around(snap, ed);
    assert!(at.is_some(), "Editor lies inside a scroll_view");
    while let Some(sv) = at {
        assert!(
            sv.get("rows").is_none(),
            "a scroll_view holding Editor has rows: {:?}",
            row_texts(sv)
        );
        at = scroll_view_around(snap, sv);
    }
    let tree = files_tree(snap, "s").expect("Project files");
    assert!(tree.get("rows").is_none(), "`Project files` has rows");
    // F2: every node with rows other than Editor has one row, its value or else its label.
    for n in preorder(snap) {
        if label(n) == Some("Editor") || n.get("rows").is_none() {
            continue;
        }
        let shows = value(n)
            .or_else(|| label(n))
            .unwrap_or_else(|| panic!("a node without text has rows: {}", short(n)));
        assert_eq!(
            row_texts(n),
            [shows.to_owned()],
            "the rows of a {} node are not the text it shows",
            n["role"]
        );
    }
    // The Editor's rows are still the document's lines from line 0.
    let lines: Vec<String> = ["l0", "l1", "l2"]
        .iter()
        .map(|id| s.ok(id)["text"].as_str().expect("line text").to_owned())
        .collect();
    assert_eq!(
        lines,
        ["fn main() {", "    let x = 1;", "}"],
        "precondition"
    );
    assert_eq!(row_texts(ed), lines, "the Editor's rows");
    let _ = fs::remove_dir_all(&dir);
}
