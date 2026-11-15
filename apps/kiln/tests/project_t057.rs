//! T-057, the implementer's own tests of the project panel (`docs/protocol/ui-project.md`),
//! beside the acceptance tests: the shell fix F1 (a `scroll_view`'s reported bounds are its own
//! visible part), clicks with another button, modifiers or `at`, the root's collapse forgetting
//! every directory, focus, and the reads' params. Every test drives the built `kiln` binary
//! through `kiln script` with temporary roots; every spawned process is killed after
//! [`TIMEOUT`]. No Unix-only API (ruling Q-6). Nothing here pins a whole snapshot, a message text,
//! a position or a width.
#![cfg(test)]

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

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
        .join("t057-own")
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

fn params_of(e: &Value) -> Value {
    e.get("params").cloned().unwrap_or_else(|| json!({}))
}

/// `(command, params)` of every logged request.
fn log_of(s: &Sess) -> Vec<(String, Value)> {
    s.log_entries()
        .iter()
        .map(|e| {
            (
                e["command"].as_str().unwrap_or_default().to_owned(),
                params_of(e),
            )
        })
        .collect()
}

/// Every node with rows other than `Editor` shows one row, its value or else its label (F2), and
/// no `scroll_view` that holds `Editor` has rows.
fn check_rows_belong(snap: &Value, what: &str) {
    for n in preorder(snap) {
        if label(n) == Some("Editor") || n.get("rows").is_none() {
            continue;
        }
        let shows = value(n)
            .or_else(|| label(n))
            .unwrap_or_else(|| panic!("{what}: a node without text has rows: {}", short(n)));
        assert_eq!(
            row_texts(n),
            [shows.to_owned()],
            "{what}: the rows of a {} node",
            n["role"]
        );
    }
    let ed = editor(snap, what);
    let mut at = scroll_view_around(snap, ed);
    assert!(at.is_some(), "{what}: Editor lies inside a scroll_view");
    while let Some(sv) = at {
        assert!(
            sv.get("rows").is_none(),
            "{what}: a scroll_view holding Editor has rows {:?}",
            row_texts(sv)
        );
        at = scroll_view_around(snap, sv);
    }
}

/// F1: with the panel beside the editor, the toggle's text is attributed to the toggle, not to
/// the editor's `scroll_view` (on main it was: the editor's bounds spanned the full width),
/// collapsed and expanded, at the default size and at a small one.
#[test]
fn f1_text_beside_the_editor_belongs_to_the_panel() {
    let dir = case_dir("f1");
    let root = dir.join(ROOT);
    write(&root.join("src/a.rs"), A_RS.as_bytes());
    write(&root.join("b.txt"), b"b\n");
    for size in ["1280x800", "640x400"] {
        let script = vec![
            snapshot("collapsed"),
            click_toggle("c0"),
            click_item("c1", "src"),
            snapshot("expanded"),
        ];
        let s = run(
            &dir,
            &format!("s-{size}"),
            &script,
            &["--root", ROOT, "--size", size, "src/a.rs"],
        );
        s.all_ui_ok();
        for id in ["collapsed", "expanded"] {
            let what = format!("{size} {id}");
            let snap = s.snap(id);
            assert_eq!(
                row_texts(the_toggle(snap, &what)),
                ["Project"],
                "{what}: the toggle's row"
            );
            check_rows_belong(snap, &what);
        }
        let expanded = s.snap("expanded");
        for n in shown(expanded, size) {
            assert_eq!(
                row_texts(n),
                [value(n).expect("value").to_owned()],
                "{size}: the row of `{}`",
                item_label(n)
            );
        }
        assert_eq!(
            row_texts(editor(expanded, size)),
            ["fn main() {", "    let x = 1;", "}"],
            "{size}: the editor's rows"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// A secondary click, a click with modifiers and a click with `at` do what a primary click does,
/// on the toggle, on a directory and on a file.
#[test]
fn other_buttons_modifiers_and_at_act_as_a_click() {
    let dir = case_dir("buttons");
    let root = dir.join(ROOT);
    write(&root.join("src/a.rs"), A_RS.as_bytes());
    write(&root.join("b.txt"), b"b\n");
    let variants: [(&str, Value); 4] = [
        ("secondary", json!({"button": "secondary"})),
        ("ctrl", json!({"modifiers": ["ctrl"]})),
        ("shift-alt", json!({"modifiers": ["shift", "alt"]})),
        ("at", json!({"at": {"row": 0, "col": 1}})),
    ];
    for (name, extra) in variants {
        let with = |id: &str, target: Value| {
            let mut fields = extra.clone();
            fields["target"] = target;
            ui(id, "click", fields)
        };
        // `at` needs painted rows: the toggle and every entry have one.
        let script = vec![
            with("t", toggle_target()),
            snapshot("s1"),
            with("d", item_target("src")),
            snapshot("s2"),
            with("f", item_target("src/a.rs")),
            snapshot("s3"),
            with("d2", item_target("src")),
            snapshot("s4"),
            ctl("l", "doc.list", json!({})),
        ];
        let s = run(&dir, name, &script, &root_args());
        s.all_ui_ok();
        assert!(toggle_state(s.snap("s1"), name), "{name}: expanded");
        assert_eq!(shown_labels(s.snap("s1"), name), ["b.txt", "src"]);
        assert_eq!(
            shown_labels(s.snap("s2"), name),
            ["b.txt", "src", "src/a.rs"],
            "{name}: src expanded"
        );
        assert!(
            title(s.snap("s3"), name).starts_with("src/a.rs"),
            "{name}: the file opened"
        );
        assert_eq!(
            shown_labels(s.snap("s4"), name),
            ["b.txt", "src"],
            "{name}: src collapsed"
        );
        assert_eq!(s.ok("l")["docs"][0]["path"], json!("src/a.rs"));
        assert_eq!(
            log_of(&s)
                .into_iter()
                .filter(|(c, _)| c == "project.tree" || c == "doc.open")
                .collect::<Vec<_>>(),
            [
                ("project.tree".to_owned(), json!({})),
                ("project.tree".to_owned(), json!({"path": "src"})),
                ("doc.open".to_owned(), json!({"path": "src/a.rs"})),
            ],
            "{name}: the panel's requests"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// Collapsing the root forgets every directory's expanded state: expanding it again reads the
/// root only and shows every directory collapsed.
#[test]
fn collapsing_the_root_forgets_every_directory() {
    let dir = case_dir("root-collapse");
    let root = dir.join(ROOT);
    write(&root.join("src/sub/a.rs"), A_RS.as_bytes());
    write(&root.join("z.txt"), b"z\n");
    let script = vec![
        click_toggle("e0"),
        click_item("e1", "src"),
        click_item("e2", "src/sub"),
        snapshot("s1"),
        click_toggle("c0"),
        snapshot("s2"),
        click_toggle("e3"),
        snapshot("s3"),
        click_item("e4", "src"),
        snapshot("s4"),
    ];
    let s = run(&dir, "s", &script, &root_args());
    s.all_ui_ok();
    assert_eq!(
        shown_labels(s.snap("s1"), "s1"),
        ["src", "src/sub", "src/sub/a.rs", "z.txt"]
    );
    assert!(shown(s.snap("s2"), "s2").is_empty());
    assert_eq!(shown_labels(s.snap("s3"), "s3"), ["src", "z.txt"]);
    assert_eq!(toggled(entry(s.snap("s3"), "src", "s3")), Some(false));
    assert_eq!(
        shown_labels(s.snap("s4"), "s4"),
        ["src", "src/sub", "z.txt"]
    );
    assert_eq!(toggled(entry(s.snap("s4"), "src/sub", "s4")), Some(false));
    let t = |p: Value| ("project.tree".to_owned(), p);
    assert_eq!(
        log_of(&s),
        [
            t(json!({})),
            t(json!({"path": "src"})),
            t(json!({"path": "src/sub"})),
            t(json!({})),
            t(json!({"path": "src"})),
        ]
    );
    let _ = fs::remove_dir_all(&dir);
}

/// L4: nothing is focused by itself, with the panel collapsed or expanded; an entry the user
/// did not click takes no focus.
#[test]
fn no_entry_takes_focus_by_itself() {
    let dir = case_dir("focus");
    let root = dir.join(ROOT);
    write(&root.join("src/a.rs"), A_RS.as_bytes());
    write(&root.join("b.txt"), b"b\n");
    let script = vec![
        snapshot("s0"),
        click_toggle("e0"),
        frames("f", 30),
        snapshot("s1"),
        click_item("e1", "src"),
        snapshot("s2"),
    ];
    let s = run(&dir, "s", &script, &["--root", ROOT, "src/a.rs"]);
    s.all_ui_ok();
    let focused = |id: &str| -> Vec<String> {
        preorder(s.snap(id))
            .into_iter()
            .filter(|n| n.get("focused").is_some())
            .map(|n| format!("{:?} {:?}", role(n), label(n)))
            .collect()
    };
    assert!(focused("s0").is_empty(), "s0: {:?}", focused("s0"));
    for id in ["s1", "s2"] {
        let snap = s.snap(id);
        for n in shown(snap, id) {
            if label(n) != Some("src") {
                assert!(
                    n.get("focused").is_none(),
                    "{id}: `{}` took focus without a click",
                    item_label(n)
                );
            }
        }
        assert!(
            editor(snap, id).get("focused").is_none(),
            "{id}: the editor took focus from a panel click"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// Paths with spaces and non-ASCII names are sent unchanged as `path` (Pinned (d)), and the
/// opened document's path equals the entry's label.
#[test]
fn labels_are_sent_unchanged_as_paths() {
    let dir = case_dir("paths");
    let root = dir.join(ROOT);
    write(&root.join("dir with space/\u{e9}.txt"), b"e\n");
    write(&root.join("e\u{301}x.txt"), b"x\n");
    let script = vec![
        click_toggle("e0"),
        click_item("e1", "dir with space"),
        click_item("o1", "dir with space/\u{e9}.txt"),
        click_item("o2", "e\u{301}x.txt"),
        ctl("l", "doc.list", json!({})),
        snapshot("s"),
    ];
    let s = run(&dir, "s", &script, &root_args());
    s.all_ui_ok();
    assert_eq!(
        s.ok("l")["docs"],
        json!([
            {"dirty": false, "doc": 1, "path": "dir with space/\u{e9}.txt"},
            {"dirty": false, "doc": 2, "path": "e\u{301}x.txt"},
        ])
    );
    assert_eq!(
        value(entry(s.snap("s"), "dir with space/\u{e9}.txt", "s")),
        Some("\u{e9}.txt")
    );
    assert_eq!(
        log_of(&s)
            .into_iter()
            .filter(|(c, _)| c == "project.tree" || c == "doc.open")
            .collect::<Vec<_>>(),
        [
            ("project.tree".to_owned(), json!({})),
            ("project.tree".to_owned(), json!({"path": "dir with space"})),
            (
                "doc.open".to_owned(),
                json!({"path": "dir with space/\u{e9}.txt"})
            ),
            ("doc.open".to_owned(), json!({"path": "e\u{301}x.txt"})),
        ],
        "the panel's requests (the editor's reads aside)"
    );
    let _ = fs::remove_dir_all(&dir);
}
