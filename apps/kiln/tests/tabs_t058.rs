//! T-058, the implementer's own tests of the tabs (`docs/protocol/ui-tabs.md`) and of the
//! `Documents` click, beside the acceptance tests. Every test drives the built `kiln` binary
//! through `kiln script` with temporary roots; every spawned process is killed after
//! [`TIMEOUT`]. `kiln_control::Host` is used only as an oracle (a replay of the log). No
//! Unix-only API; nothing here pins a whole snapshot, a message text beyond its leading kind, a
//! position, a size or the strip's height.
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

const A_RS: &str = "fn main() {\n    let x = 1;\n}\n";

// ---------------------------------------------------------------------------------------------
// Processes, directories, files.
// ---------------------------------------------------------------------------------------------

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

fn kiln(cwd: &Path, args: &[&str]) -> Out {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kiln"));
    cmd.args(args).current_dir(cwd);
    run_cmd(cmd, &format!("kiln {}", args.join(" ")))
}

fn case_dir(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("t058-own")
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

/// Write `files` (root-relative path, contents) below `dir/ROOT`.
fn fixture(dir: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        write(&dir.join(ROOT).join(path), text.as_bytes());
    }
}

/// Copy the directory tree `from` to `to` (regular files and directories only).
fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create copy dir");
    for e in fs::read_dir(from).expect("read_dir") {
        let e = e.expect("dir entry");
        let target = to.join(e.file_name());
        if e.file_type().expect("file type").is_dir() {
            copy_tree(&e.path(), &target);
        } else {
            fs::copy(e.path(), &target).expect("copy file");
        }
    }
}

/// Every regular file below `dir`, by its path relative to `dir` (with `/`), and its bytes.
fn files_of(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![(dir.to_path_buf(), String::new())];
    while let Some((d, prefix)) = stack.pop() {
        for e in fs::read_dir(&d).expect("read_dir") {
            let e = e.expect("dir entry");
            let name = e.file_name().into_string().expect("UTF-8 name");
            let rel = format!("{prefix}{name}");
            if e.file_type().expect("file type").is_dir() {
                stack.push((e.path(), format!("{rel}/")));
            } else {
                out.insert(rel, fs::read(e.path()).expect("read file"));
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Script lines.
// ---------------------------------------------------------------------------------------------

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

fn ctl(id: &str, command: &str, params: Value) -> String {
    json!({"id": id, "command": command, "params": params}).to_string()
}

fn snapshot(id: &str) -> String {
    ui(id, "snapshot", Value::Null)
}

fn frames(id: &str, count: u64) -> String {
    ui(id, "frames", json!({ "count": count }))
}

fn click(id: &str, target: Value) -> String {
    ui(id, "click", json!({ "target": target }))
}

fn tab_target(path: &str) -> Value {
    json!({"role": "tab", "label": path})
}

fn click_tab(id: &str, path: &str) -> String {
    click(id, tab_target(path))
}

fn click_close(id: &str, path: &str) -> String {
    click(
        id,
        json!({"role": "button", "label": format!("Close {path}")}),
    )
}

fn click_discard(id: &str, path: &str) -> String {
    click(
        id,
        json!({"role": "button", "label": format!("Discard {path}")}),
    )
}

fn click_entry(id: &str, path: &str) -> String {
    click(id, json!({"role": "label", "value": path}))
}

fn open(id: &str, path: &str) -> String {
    ctl(id, "doc.open", json!({ "path": path }))
}

fn key(id: &str, name: &str, modifiers: &[&str]) -> String {
    ui(id, "key", json!({"key": name, "modifiers": modifiers}))
}

fn type_text(id: &str, text: &str) -> String {
    ui(id, "type", json!({ "text": text }))
}

// ---------------------------------------------------------------------------------------------
// Sessions.
// ---------------------------------------------------------------------------------------------

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

    fn snap(&self, id: &str) -> &Value {
        &self.ok(id)["root"]
    }

    /// Every UI request answered `ok:true`, each action settled.
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
                if v["ui"] != "snapshot" && v["ui"] != "frames" {
                    assert_eq!(
                        resp["result"]["settled"],
                        json!(true),
                        "{}: {line} did not settle: {}",
                        self.name,
                        short(resp)
                    );
                }
            }
        }
    }

    /// The session log's entries as `(command, params)`.
    fn log_entries(&self) -> Vec<(String, Value)> {
        log_of(&self.log)
    }

    /// The `doc.list` answer of the control request `id`, as `(doc, path, dirty)`.
    fn list(&self, id: &str) -> Vec<(u64, String, bool)> {
        self.ok(id)["docs"]
            .as_array()
            .expect("docs")
            .iter()
            .map(|d| {
                (
                    d["doc"].as_u64().expect("doc"),
                    d["path"].as_str().expect("path").to_owned(),
                    d["dirty"].as_bool().expect("dirty"),
                )
            })
            .collect()
    }
}

fn log_of(log: &str) -> Vec<(String, Value)> {
    log.lines()
        .map(|l| {
            let v: Value = serde_json::from_str(l).expect("log line is JSON");
            (
                v["command"].as_str().expect("command").to_owned(),
                v.get("params").cloned().unwrap_or_else(|| json!({})),
            )
        })
        .collect()
}

/// Run `kiln script <name>.jsonl --root ROOT <extra…> --log <name>.log.jsonl` in `dir`.
fn run(dir: &Path, name: &str, script: &[String], extra: &[&str]) -> Sess {
    let file = format!("{name}.jsonl");
    let log = format!("{name}.log.jsonl");
    let mut bytes = Vec::new();
    for l in script {
        bytes.extend_from_slice(l.as_bytes());
        bytes.push(b'\n');
    }
    write(&dir.join(&file), &bytes);
    let _ = fs::remove_file(dir.join(&log));
    let mut all = vec!["script", file.as_str(), "--root", ROOT];
    all.extend_from_slice(extra);
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

// ---------------------------------------------------------------------------------------------
// Snapshot nodes.
// ---------------------------------------------------------------------------------------------

fn children(n: &Value) -> &[Value] {
    n["children"].as_array().map_or(&[][..], Vec::as_slice)
}

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

fn short(v: &Value) -> String {
    let s = v.to_string();
    if s.chars().count() > 1500 {
        let head: String = s.chars().take(1500).collect();
        format!("{head}…")
    } else {
        s
    }
}

/// The painted rows of `n`, each as its runs' texts joined.
fn row_texts(n: &Value) -> Vec<String> {
    n["rows"].as_array().map_or_else(Vec::new, |rows| {
        rows.iter()
            .map(|row| {
                row.as_array()
                    .map(|runs| {
                        runs.iter()
                            .map(|r| r["text"].as_str().unwrap_or_default())
                            .collect::<String>()
                    })
                    .unwrap_or_default()
            })
            .collect()
    })
}

/// The `Tabs` node, if shown (exactly one).
fn tabs_node<'a>(root: &'a Value, what: &str) -> Option<&'a Value> {
    let found: Vec<&Value> = preorder(root)
        .into_iter()
        .filter(|n| role(n) == Some("tab_list") && label(n) == Some("Tabs"))
        .collect();
    assert!(found.len() <= 1, "{what}: {} `Tabs` nodes", found.len());
    found.first().copied()
}

/// What the strip shows, per tab in order: its label, `toggled`, whether a `modified` node
/// follows it and whether its `Discard` button does (before the next tab).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Shown {
    path: String,
    current: bool,
    modified: bool,
    discard: bool,
}

fn shown(root: &Value, what: &str) -> Vec<Shown> {
    let Some(tabs) = tabs_node(root, what) else {
        return Vec::new();
    };
    let mut out: Vec<Shown> = Vec::new();
    for n in preorder(tabs) {
        match role(n) {
            Some("tab") => {
                assert!(
                    value(n).is_none(),
                    "{what}: a tab has a value: {}",
                    short(n)
                );
                out.push(Shown {
                    path: label(n).expect("a tab has a label").to_owned(),
                    current: n["toggled"]
                        .as_bool()
                        .unwrap_or_else(|| panic!("{what}: a tab without toggled: {}", short(n))),
                    modified: false,
                    discard: false,
                });
            }
            Some("label") if value(n) == Some("modified") => {
                out.last_mut().expect("`modified` before any tab").modified = true;
            }
            Some("button") => {
                let last = out.last_mut().expect("a button before any tab");
                let l = label(n).unwrap_or_default();
                if l == format!("Discard {}", last.path) {
                    last.discard = true;
                } else {
                    assert_eq!(
                        l,
                        format!("Close {}", last.path),
                        "{what}: a button after a tab"
                    );
                }
            }
            _ => {}
        }
    }
    out
}

fn tab_labels(root: &Value, what: &str) -> Vec<String> {
    shown(root, what).into_iter().map(|t| t.path).collect()
}

fn title(root: &Value) -> Option<String> {
    preorder(root)
        .into_iter()
        .find(|n| label(n) == Some("Title"))
        .and_then(|n| value(n).map(str::to_owned))
}

fn message(root: &Value) -> String {
    preorder(root)
        .into_iter()
        .find(|n| label(n) == Some("Message"))
        .and_then(|n| value(n).map(str::to_owned))
        .unwrap_or_default()
}

fn editor(root: &Value) -> &Value {
    preorder(root)
        .into_iter()
        .find(|n| label(n) == Some("Editor"))
        .expect("an `Editor` node")
}

/// The published `mutating` flag of each command (`docs/commands.json`).
fn mutating_flags() -> HashMap<String, bool> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/commands.json");
    let v: Value = serde_json::from_slice(&fs::read(path).expect("docs/commands.json"))
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

/// The log entries the script's lines up to and including `id` had added (the session log
/// holds every successful request in order; the `--log` of a prefix of the script is its
/// prefix). Found by running `script[..=id]` as its own session.
fn log_upto(dir: &Path, name: &str, script: &[String], id: &str, extra: &[&str]) -> usize {
    let end = script
        .iter()
        .position(|l| serde_json::from_str::<Value>(l).expect("line")["id"] == json!(id))
        .expect("id in script");
    run(dir, name, &script[..=end], extra).log_entries().len()
}

// ---------------------------------------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------------------------------------

/// AC-1, T1: tabs follow opens from every source and closes, in id order; one `toggled` tab,
/// the one `Title` names; no `Tabs` with nothing open; the strip is in a `scroll_view` with no
/// shell node, `Title` or `Editor`; a tab paints its path.
#[test]
fn tabs_follow_opens_and_closes_from_every_source() {
    let dir = case_dir("follow");
    fixture(
        &dir,
        &[
            ("src/a.rs", A_RS),
            ("b.txt", "b\n"),
            ("c d.txt", "c\n"),
            ("z.txt", "z\n"),
        ],
    );
    let script = vec![
        snapshot("s0"),
        ui("fp", "focus", json!({"target": {"label": "Path"}})),
        type_text("tp", "b.txt"),
        click("co", json!({"label": "Open"})),
        snapshot("s1"),
        ctl("l1", "doc.list", json!({})),
        open("o2", "src/a.rs"),
        frames("f2", 1),
        snapshot("s2"),
        ctl("l2", "doc.list", json!({})),
        click("pt", json!({"role": "button", "label": "Project"})),
        click("pz", json!({"role": "tree_item", "label": "z.txt"})),
        snapshot("s3"),
        ctl("l3", "doc.list", json!({})),
        open("o4", "c d.txt"),
        open("o4b", "b.txt"),
        frames("f4", 1),
        snapshot("s4"),
        ctl("l4", "doc.list", json!({})),
        ctl("x5", "doc.close", json!({"doc": 1})),
        frames("f5", 1),
        snapshot("s5"),
        ctl("l5", "doc.list", json!({})),
        ctl("x6", "doc.close", json!({"doc": 3})),
        frames("f6", 1),
        snapshot("s6"),
        ctl("l6", "doc.list", json!({})),
        ctl("x7a", "doc.close", json!({"doc": 2})),
        ctl("x7b", "doc.close", json!({"doc": 4})),
        frames("f7", 2),
        snapshot("s7"),
    ];
    let s = run(&dir, "s", &script, &[]);
    s.all_ui_ok();
    assert!(tabs_node(s.snap("s0"), "s0").is_none());
    assert!(tabs_node(s.snap("s7"), "s7").is_none());
    assert!(s.snap("s7").to_string().find("\"tab\"").is_none());
    for (snap, list) in [
        ("s1", "l1"),
        ("s2", "l2"),
        ("s3", "l3"),
        ("s4", "l4"),
        ("s5", "l5"),
        ("s6", "l6"),
    ] {
        let root = s.snap(snap);
        let paths: Vec<String> = s.list(list).into_iter().map(|(_, p, _)| p).collect();
        assert_eq!(tab_labels(root, snap), paths, "{snap}: tabs vs doc.list");
        let current: Vec<Shown> = shown(root, snap)
            .into_iter()
            .filter(|t| t.current)
            .collect();
        assert_eq!(current.len(), 1, "{snap}: toggled tabs {current:?}");
        let t = title(root).expect("Title");
        assert!(
            t.starts_with(&current[0].path),
            "{snap}: Title {t:?} vs {current:?}"
        );
        for n in preorder(tabs_node(root, snap).expect("Tabs")) {
            if role(n) == Some("tab") {
                assert_eq!(row_texts(n), [label(n).unwrap()], "{snap}: tab rows");
            }
        }
    }
    // The opens' order: b.txt (UI), src/a.rs (control), z.txt (panel), c d.txt (control),
    // then b.txt again (already open: current again).
    assert_eq!(
        tab_labels(s.snap("s4"), "s4"),
        ["b.txt", "src/a.rs", "z.txt", "c d.txt"]
    );
    assert!(shown(s.snap("s4"), "s4")[0].current);
    // The strip's scroll view holds no shell node, no `Title`, no `Editor`.
    let root = s.snap("s4");
    let scroll = preorder(root)
        .into_iter()
        .filter(|n| role(n) == Some("scroll_view"))
        .find(|n| {
            preorder(n)
                .iter()
                .any(|m| role(m) == Some("tab_list") && label(m) == Some("Tabs"))
        })
        .expect("Tabs inside a scroll_view");
    for n in preorder(scroll) {
        let l = label(n).unwrap_or_default();
        assert!(
            ![
                "Path",
                "Open",
                "Message",
                "Documents",
                "Title",
                "Editor",
                "Project"
            ]
            .contains(&l),
            "the strip's scroll view holds {l:?}"
        );
        assert_ne!(role(n), Some("tree_item"));
    }
    let _ = fs::remove_dir_all(&dir);
}

/// AC-2, T2: a click on a tab shows that document (Title, Editor value and selection as the
/// registry reports them); the switch logs only read-only commands, one naming the document; a
/// click on the current tab adds nothing; a secondary click, modifiers and `at` act as a click;
/// a tab click focuses nothing.
#[test]
fn switching_shows_the_document_and_reads_only() {
    let dir = case_dir("switch");
    fixture(&dir, &[("src/a.rs", A_RS), ("b.txt", "one\r\ntwo\r\n")]);
    let script = vec![
        open("o1", "src/a.rs"),
        ctl(
            "cs1",
            "cursor.set",
            json!({"doc": 1, "anchor": 3, "head": 7}),
        ),
        open("o2", "b.txt"),
        ctl(
            "cs2",
            "cursor.set",
            json!({"doc": 2, "anchor": 5, "head": 5}),
        ),
        frames("f", 3),
        snapshot("s0"),
        click_tab("c1", "src/a.rs"),
        snapshot("s1"),
        frames("idle1", 50),
        click_tab("c1again", "src/a.rs"),
        frames("idle2", 50),
        ctl("t1", "doc.text", json!({"doc": 1})),
        ctl("g1", "cursor.get", json!({"doc": 1})),
        ui(
            "c2",
            "click",
            json!({"target": tab_target("b.txt"), "button": "secondary"}),
        ),
        snapshot("s2"),
        ui(
            "c3",
            "click",
            json!({"target": tab_target("src/a.rs"), "modifiers": ["ctrl", "shift"]}),
        ),
        snapshot("s3"),
        ui(
            "c4",
            "click",
            json!({"target": tab_target("b.txt"), "at": {"row": 0, "col": 1}}),
        ),
        snapshot("s4"),
        ctl("t2", "doc.text", json!({"doc": 2})),
        ctl("g2", "cursor.get", json!({"doc": 2})),
    ];
    let s = run(&dir, "s", &script, &[]);
    s.all_ui_ok();
    let check = |snap: &str, path: &str, text: &str, get: &str| {
        let root = s.snap(snap);
        assert!(title(root).expect("Title").starts_with(path), "{snap}");
        let current: Vec<String> = shown(root, snap)
            .into_iter()
            .filter(|t| t.current)
            .map(|t| t.path)
            .collect();
        assert_eq!(current, [path], "{snap}");
        let e = editor(root);
        assert_eq!(e["value"], s.ok(text)["text"], "{snap}: Editor value");
        let g = s.ok(get);
        assert_eq!(e["selection"]["anchor"], g["anchor"], "{snap}: anchor");
        assert_eq!(e["selection"]["head"], g["head"], "{snap}: head");
        assert!(
            preorder(root).iter().all(|n| n.get("focused").is_none()),
            "{snap}: something focused"
        );
    };
    check("s1", "src/a.rs", "t1", "g1");
    check("s3", "src/a.rs", "t1", "g1");
    check("s2", "b.txt", "t2", "g2");
    check("s4", "b.txt", "t2", "g2");
    assert_eq!(s.ok("g1")["anchor"], json!(3));
    assert_eq!(s.ok("g1")["head"], json!(7));

    // The entries the first switch added, and those of the click on the current tab.
    let flags = mutating_flags();
    let before = log_upto(&dir, "p0", &script, "s0", &[]);
    let after = log_upto(&dir, "p1", &script, "idle1", &[]);
    let again = log_upto(&dir, "p2", &script, "idle2", &[]);
    let entries = s.log_entries();
    let added = &entries[before..after];
    assert!(!added.is_empty(), "the switch logged nothing");
    for (command, _) in added {
        assert_eq!(
            flags.get(command),
            Some(&false),
            "the switch logged {command}"
        );
    }
    assert!(
        added.iter().any(|(_, p)| p["doc"] == json!(1)),
        "no entry names doc 1: {added:?}"
    );
    assert!(added.iter().any(|(c, _)| c == "doc.list"), "{added:?}");
    assert_eq!(again, after, "a click on the current tab logged something");
    let _ = fs::remove_dir_all(&dir);
}

/// AC-3: a close from a clean tab sends `doc.close {"doc": id}`; the current one's close makes
/// the highest open id current, another's leaves the current one; `Message` is cleared.
#[test]
fn closing_a_clean_tab() {
    let dir = case_dir("close");
    fixture(
        &dir,
        &[("a.txt", "a\n"), ("b.txt", "b\n"), ("c.txt", "c\n")],
    );
    let script = vec![
        open("o1", "a.txt"),
        open("o2", "b.txt"),
        open("o3", "c.txt"),
        open("o1b", "a.txt"),
        frames("f", 1),
        // A refusal in `Message` first, so that its clearing is seen.
        ui("fp", "focus", json!({"target": {"label": "Path"}})),
        type_text("tp", "missing.txt"),
        click("co", json!({"label": "Open"})),
        snapshot("s0"),
        click_close("x2", "b.txt"),
        snapshot("s1"),
        ctl("l1", "doc.list", json!({})),
        click_close("x1", "a.txt"),
        snapshot("s2"),
        ctl("l2", "doc.list", json!({})),
        click_close("x3", "c.txt"),
        snapshot("s3"),
        ctl("l3", "doc.list", json!({})),
    ];
    let s = run(&dir, "s", &script, &[]);
    s.all_ui_ok();
    assert!(message(s.snap("s0")).starts_with("not_found"), "s0");
    // b closed; a stays current.
    assert_eq!(tab_labels(s.snap("s1"), "s1"), ["a.txt", "c.txt"]);
    assert_eq!(s.list("l1").len(), 2);
    assert!(title(s.snap("s1")).unwrap().starts_with("a.txt"));
    assert_eq!(message(s.snap("s1")), "");
    // a (current) closed: c (id 3, the highest) is current.
    assert_eq!(tab_labels(s.snap("s2"), "s2"), ["c.txt"]);
    assert!(title(s.snap("s2")).unwrap().starts_with("c.txt"));
    assert!(shown(s.snap("s2"), "s2")[0].current);
    assert!(tabs_node(s.snap("s3"), "s3").is_none());
    assert!(title(s.snap("s3")).is_none());
    assert!(s.list("l3").is_empty());
    let closes: Vec<Value> = s
        .log_entries()
        .into_iter()
        .filter(|(c, _)| c == "doc.close")
        .map(|(_, p)| p)
        .collect();
    assert_eq!(
        closes,
        [json!({"doc": 2}), json!({"doc": 1}), json!({"doc": 3})]
    );
    let _ = fs::remove_dir_all(&dir);
}

/// AC-4, T3: a dirty document's close is refused (`unsaved_changes`), `Discard` appears and
/// closes it with `discard: true`; after a save instead, `Discard` goes and a close keeps the
/// edit on disk.
#[test]
fn unsaved_changes_need_a_second_click() {
    let dir = case_dir("discard");
    fixture(&dir, &[("a.txt", "abc\n"), ("b.txt", "b\n")]);
    let edit = |id: &str| -> Vec<String> {
        vec![
            click_tab(&format!("{id}t"), "a.txt"),
            ui(
                &format!("{id}c"),
                "click",
                json!({"target": {"label": "Editor"}, "at": {"row": 0, "col": 0}}),
            ),
            type_text(&format!("{id}y"), "Q"),
        ]
    };
    let mut script = vec![open("o1", "a.txt"), open("o2", "b.txt"), frames("f", 1)];
    script.extend(edit("e"));
    script.extend([
        click_close("x1", "a.txt"),
        snapshot("s1"),
        ctl("l1", "doc.list", json!({})),
        click_tab("sw", "b.txt"),
        snapshot("s2"),
        click_discard("d1", "a.txt"),
        snapshot("s3"),
        ctl("l3", "doc.list", json!({})),
    ]);
    let s = run(&dir, "s", &script, &[]);
    s.all_ui_ok();
    let a = |snap: &str| -> Option<Shown> {
        shown(s.snap(snap), snap)
            .into_iter()
            .find(|t| t.path == "a.txt")
    };
    assert!(message(s.snap("s1")).starts_with("unsaved_changes"));
    assert_eq!(
        a("s1"),
        Some(Shown {
            path: "a.txt".into(),
            current: true,
            modified: true,
            discard: true
        })
    );
    assert_eq!(s.list("l1")[0], (1, "a.txt".into(), true));
    // The Discard button paints its label; the close mark paints no text.
    let root = s.snap("s1");
    let discard = preorder(root)
        .into_iter()
        .find(|n| label(n) == Some("Discard a.txt"))
        .expect("Discard");
    assert_eq!(row_texts(discard), ["Discard a.txt"]);
    let close = preorder(root)
        .into_iter()
        .find(|n| label(n) == Some("Close a.txt"))
        .expect("Close");
    assert!(row_texts(close).is_empty(), "the close mark paints text");
    // It stays across a switch.
    assert!(a("s2").expect("a.txt").discard);
    assert!(
        s.log_entries()
            .iter()
            .all(|(c, p)| c != "doc.close" || p["discard"] == json!(true))
    );
    assert!(a("s3").is_none());
    assert_eq!(s.list("l3"), [(2, "b.txt".into(), false)]);
    assert!(
        s.log_entries()
            .contains(&("doc.close".into(), json!({"doc": 1, "discard": true})))
    );
    assert_eq!(fs::read(dir.join(ROOT).join("a.txt")).unwrap(), b"abc\n");

    // Second session: save instead.
    let dir2 = case_dir("discard-save");
    fixture(&dir2, &[("a.txt", "abc\n"), ("b.txt", "b\n")]);
    let mut script = vec![open("o1", "a.txt"), open("o2", "b.txt"), frames("f", 1)];
    script.extend(edit("e"));
    script.extend([
        click_close("x1", "a.txt"),
        snapshot("s1"),
        // A click outside the editor (here on `Close`) takes its focus, as any click does.
        ui("ef", "focus", json!({"target": {"label": "Editor"}})),
        key("save", "S", &["ctrl"]),
        snapshot("s2"),
        click_close("x2", "a.txt"),
        snapshot("s3"),
        ctl("l3", "doc.list", json!({})),
    ]);
    let s = run(&dir2, "s", &script, &[]);
    s.all_ui_ok();
    assert!(a_of(s.snap("s1")).discard);
    let saved = a_of(s.snap("s2"));
    assert!(!saved.discard && !saved.modified, "{saved:?}");
    assert!(shown(s.snap("s3"), "s3").iter().all(|t| t.path != "a.txt"));
    assert_eq!(s.list("l3"), [(2, "b.txt".into(), false)]);
    assert!(
        s.log_entries()
            .contains(&("doc.close".into(), json!({"doc": 1})))
    );
    assert!(
        !s.log_entries()
            .iter()
            .any(|(c, p)| c == "doc.close" && p.get("discard").is_some())
    );
    assert_eq!(fs::read(dir2.join(ROOT).join("a.txt")).unwrap(), b"Qabc\n");
    let _ = fs::remove_dir_all(&dir);
    let _ = fs::remove_dir_all(&dir2);
}

fn a_of(root: &Value) -> Shown {
    shown(root, "a.txt")
        .into_iter()
        .find(|t| t.path == "a.txt")
        .expect("a.txt's tab")
}

/// AC-5, AC-7 (c), T4, T5: `modified` follows dirty documents, also after a control edit of a
/// non-current one, an undo from the UI and a control save; the reads come after the change,
/// and idle frames add nothing.
#[test]
fn modified_follows_doc_list() {
    let dir = case_dir("dirty");
    fixture(
        &dir,
        &[("a.txt", "a\n"), ("b.txt", "b\n"), ("c.txt", "c\n")],
    );
    let script = vec![
        open("o1", "a.txt"),
        open("o2", "b.txt"),
        open("o3", "c.txt"),
        frames("f0", 5),
        frames("idle0", 600),
        ctl(
            "i1",
            "doc.insert",
            json!({"doc": 1, "offset": 0, "text": "x"}),
        ),
        frames("f1", 5),
        snapshot("s1"),
        ctl("l1", "doc.list", json!({})),
        frames("f1b", 5),
        frames("idle1", 600),
        ctl(
            "i2",
            "doc.insert",
            json!({"doc": 3, "offset": 0, "text": "y"}),
        ),
        frames("f2", 5),
        snapshot("s2"),
        ctl("l2", "doc.list", json!({})),
        click("ce", json!({"label": "Editor"})),
        key("undo", "Z", &["ctrl"]),
        snapshot("s3"),
        ctl("l3", "doc.list", json!({})),
        ctl("sv", "doc.save", json!({"doc": 1})),
        frames("f4", 5),
        snapshot("s4"),
        ctl("l4", "doc.list", json!({})),
    ];
    let s = run(&dir, "s", &script, &[]);
    s.all_ui_ok();
    for (snap, list) in [("s1", "l1"), ("s2", "l2"), ("s3", "l3"), ("s4", "l4")] {
        let marks: Vec<(String, bool)> = shown(s.snap(snap), snap)
            .into_iter()
            .map(|t| (t.path, t.modified))
            .collect();
        let dirty: Vec<(String, bool)> = s.list(list).into_iter().map(|(_, p, d)| (p, d)).collect();
        assert_eq!(marks, dirty, "{snap}");
    }
    assert!(s.list("l1")[0].2, "the control edit made a.txt dirty");
    assert!(!s.list("l3")[2].2, "the undo made c.txt clean");
    assert!(!s.list("l4")[0].2, "the save made a.txt clean");
    let modified = preorder(s.snap("s1"))
        .into_iter()
        .find(|n| value(n) == Some("modified"))
        .expect("modified");
    assert_eq!(row_texts(modified), ["modified"]);

    // The reads: idle frames add nothing; the frames after the control edit add `doc.list`.
    let n0 = log_upto(&dir, "p0", &script, "f0", &[]);
    let n0i = log_upto(&dir, "p0i", &script, "idle0", &[]);
    assert_eq!(n0, n0i, "600 idle frames logged something");
    let n1 = log_upto(&dir, "p1", &script, "f1", &[]);
    let entries = s.log_entries();
    let after_insert = &entries[n0 + 1..n1];
    assert!(
        after_insert.iter().any(|(c, _)| c == "doc.list"),
        "{after_insert:?}"
    );
    let n1b = log_upto(&dir, "p1b", &script, "f1b", &[]);
    let n1i = log_upto(&dir, "p1i", &script, "idle1", &[]);
    assert_eq!(n1i, n1b, "600 idle frames logged something");
    let _ = fs::remove_dir_all(&dir);
}

/// AC-6: a click on a `Documents` entry makes its document current, logged like a switch.
#[test]
fn a_documents_entry_switches() {
    let dir = case_dir("documents");
    fixture(&dir, &[("src/a.rs", A_RS), ("b.txt", "b\n")]);
    let script = vec![
        open("o1", "src/a.rs"),
        open("o2", "b.txt"),
        frames("f", 2),
        snapshot("s0"),
        click_entry("c1", "src/a.rs"),
        snapshot("s1"),
        ctl("t1", "doc.text", json!({"doc": 1})),
        frames("settle", 5),
        click_entry("c1b", "src/a.rs"),
        frames("idle", 50),
        ui(
            "c2",
            "click",
            json!({"target": {"role": "label", "value": "b.txt"}, "button": "secondary"}),
        ),
        snapshot("s2"),
    ];
    let s = run(&dir, "s", &script, &[]);
    s.all_ui_ok();
    assert!(title(s.snap("s0")).unwrap().starts_with("b.txt"));
    let root = s.snap("s1");
    assert!(title(root).unwrap().starts_with("src/a.rs"));
    assert_eq!(editor(root)["value"], s.ok("t1")["text"]);
    assert!(shown(root, "s1")[0].current);
    assert!(title(s.snap("s2")).unwrap().starts_with("b.txt"));
    // `Documents` keeps its nodes: a list of labels with the paths as values.
    let docs = preorder(root)
        .into_iter()
        .find(|n| role(n) == Some("list") && label(n) == Some("Documents"))
        .expect("Documents");
    let values: Vec<&str> = preorder(docs)
        .into_iter()
        .filter(|n| role(n) == Some("label"))
        .filter_map(value)
        .collect();
    assert_eq!(values, ["src/a.rs", "b.txt"]);

    let flags = mutating_flags();
    let before = log_upto(&dir, "p0", &script, "s0", &[]);
    let after = log_upto(&dir, "p1", &script, "s1", &[]);
    let entries = s.log_entries();
    let added = &entries[before..after];
    assert!(added.iter().any(|(_, p)| p["doc"] == json!(1)), "{added:?}");
    for (command, _) in added {
        assert_eq!(flags.get(command), Some(&false), "{command}");
    }
    let settled = log_upto(&dir, "p2", &script, "settle", &[]);
    let idle = log_upto(&dir, "p3", &script, "idle", &[]);
    assert_eq!(
        idle, settled,
        "a click on the current entry logged something"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// AC-7 (a): a script of control requests only writes `Host::log_script()`.
#[test]
fn control_requests_only_log_as_the_host() {
    let dir = case_dir("control-only");
    fixture(&dir, &[("a.txt", "a\n"), ("b.txt", "b\n")]);
    let oracle = dir.join("oracle");
    copy_tree(&dir.join(ROOT), &oracle);
    let script = vec![
        open("o1", "a.txt"),
        open("o2", "b.txt"),
        ctl(
            "i",
            "doc.insert",
            json!({"doc": 1, "offset": 0, "text": "x"}),
        ),
        ctl("x", "doc.close", json!({"doc": 2})),
        ctl("l", "doc.list", json!({})),
    ];
    let s = run(&dir, "s", &script, &[]);
    let mut host = Host::on_disk(&oracle).expect("Host::on_disk");
    for line in &script {
        host.handle_line(line.as_bytes());
    }
    assert_eq!(s.log, host.log_script());
    let _ = fs::remove_dir_all(&dir);
}

/// AC-9: with 300 documents every tab is reached by scrolling `Tabs`, and a click then makes it
/// current; a click on an `offscreen` tab is `not_actionable`; every action settles.
#[test]
fn many_documents_are_reached_by_scrolling() {
    let dir = case_dir("many");
    let names: Vec<String> = (0..300).map(|i| format!("f{i:03}.txt")).collect();
    let files: Vec<(&str, &str)> = names.iter().map(|n| (n.as_str(), "x\n")).collect();
    fixture(&dir, &files);
    let opens: Vec<String> = names
        .iter()
        .enumerate()
        .map(|(i, n)| open(&format!("o{i}"), n))
        .collect();
    // First: which tabs are visible after each step down from the top.
    let mut script = opens.clone();
    script.push(frames("f", 2));
    script.push(snapshot("s0"));
    let steps = 30;
    for k in 1..=steps {
        script.push(ui(
            &format!("d{k}"),
            "scroll",
            json!({"target": {"label": "Tabs"}, "direction": "down", "steps": 1}),
        ));
        script.push(snapshot(&format!("s{k}")));
    }
    let s = run(&dir, "scan", &script, &[]);
    s.all_ui_ok();
    let mut first_seen: BTreeMap<String, usize> = BTreeMap::new();
    for k in 0..=steps {
        let id = format!("s{k}");
        let root = s.snap(&id);
        let tabs = tabs_node(root, &id).expect("Tabs");
        let all: Vec<&Value> = preorder(tabs)
            .into_iter()
            .filter(|n| role(n) == Some("tab"))
            .collect();
        assert_eq!(all.len(), 300, "{id}: every document has a tab");
        for n in all {
            if n["offscreen"] != json!(true) {
                first_seen.entry(label(n).unwrap().to_owned()).or_insert(k);
            }
        }
    }
    assert_eq!(first_seen.len(), 300, "some tab was never in view");
    let hidden = first_seen
        .iter()
        .find(|(_, k)| **k > 0)
        .map(|(n, _)| n.clone())
        .expect("some tab starts offscreen");
    // Then: click a sample of tabs (0, 23, …, 299) after scrolling to them.
    let mut script = opens;
    script.push(frames("f", 2));
    script.push(click_tab("off", &hidden));
    for (i, name) in names.iter().enumerate().step_by(23) {
        let k = first_seen[name];
        script.push(ui(
            &format!("top{i}"),
            "scroll",
            json!({"target": {"label": "Tabs"}, "direction": "up", "steps": steps + 5}),
        ));
        if k > 0 {
            script.push(ui(
                &format!("dn{i}"),
                "scroll",
                json!({"target": {"label": "Tabs"}, "direction": "down", "steps": k}),
            ));
        }
        script.push(click_tab(&format!("c{i}"), name));
        script.push(snapshot(&format!("t{i}")));
    }
    let s = run(&dir, "click", &script, &[]);
    assert_eq!(s.r("off")["error"]["kind"], json!("not_actionable"));
    for (line, resp) in s.script.iter().zip(&s.resp).skip(302) {
        assert_eq!(resp["ok"], json!(true), "{line} → {}", short(resp));
        if resp["result"].get("settled").is_some() {
            assert_eq!(resp["result"]["settled"], json!(true), "{line}");
        }
    }
    for (i, name) in names.iter().enumerate().step_by(23) {
        let t = title(s.snap(&format!("t{i}"))).expect("Title");
        assert!(t.starts_with(name.as_str()), "{name}: Title {t:?}");
    }
    let _ = fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------------------------
// AC-8: generated sessions against a naive model.
// ---------------------------------------------------------------------------------------------

/// A small deterministic generator (an LCG; the seed is printed on failure).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Name pieces (T-057 AC-3's alphabet without a line break).
const PIECES: &[&str] = &[
    "a",
    "b",
    "Zq",
    "x y",
    "é",
    "e\u{301}x",
    "a\u{200d}b",
    "ffi",
    "漢字",
    "😀",
    "שלום",
    "سلام",
    "٣",
    "m-1",
];

/// The naive model: open documents `(id, path)`, the current one, dirty ones, `Discard` shown.
#[derive(Default)]
struct Model {
    next: u64,
    open: Vec<(u64, String)>,
    current: Option<u64>,
    dirty: BTreeSet<u64>,
    discard: BTreeSet<u64>,
}

impl Model {
    fn path(&self, id: u64) -> String {
        self.open.iter().find(|(d, _)| *d == id).unwrap().1.clone()
    }

    fn close(&mut self, id: u64) {
        self.open.retain(|(d, _)| *d != id);
        self.dirty.remove(&id);
        self.discard.remove(&id);
        if self.current == Some(id) {
            self.current = self.open.last().map(|(d, _)| *d);
        }
    }

    fn expected(&self) -> Vec<Shown> {
        self.open
            .iter()
            .map(|(id, path)| Shown {
                path: path.clone(),
                current: self.current == Some(*id),
                modified: self.dirty.contains(id),
                discard: self.discard.contains(id),
            })
            .collect()
    }
}

fn starts_ltr(name: &str) -> bool {
    !["שלום", "سلام", "٣"].iter().any(|p| name.starts_with(p))
}

/// After one step: its snapshot's id, the expected strip and the path `Title` begins with.
type Check = (String, Vec<Shown>, Option<String>);

/// One generated session: the root's names, the script, and a check after each step.
fn generated(seed: u64) -> (Vec<String>, Vec<String>, Vec<Check>) {
    let mut rng = Rng(seed);
    let mut names = BTreeSet::new();
    let count = 3 + rng.below(4);
    let mut keys = BTreeSet::new();
    while names.len() < count {
        let mut n = String::new();
        for _ in 0..1 + rng.below(2) {
            n.push_str(PIECES[rng.below(PIECES.len())]);
        }
        let n = format!("{n}.txt");
        let n = if rng.below(3) == 0 {
            format!("d/{n}")
        } else {
            n
        };
        // No case-only or normalization collisions (T-057 AC-3).
        if keys.insert(n.replace("e\u{301}", "é").to_lowercase()) {
            names.insert(n);
        }
    }
    let names: Vec<String> = names.into_iter().collect();
    let mut m = Model {
        next: 1,
        ..Model::default()
    };
    let mut script = Vec::new();
    let mut checks = Vec::new();
    for step in 0..30 {
        let id = format!("{step}");
        let choice = rng.below(9);
        let pick_open = |rng: &mut Rng, m: &Model| -> Option<u64> {
            (!m.open.is_empty()).then(|| m.open[rng.below(m.open.len())].0)
        };
        match choice {
            0 | 1 => {
                let name = &names[rng.below(names.len())];
                script.push(open(&format!("{id}o"), name));
                script.push(frames(&format!("{id}f"), 1));
                match m.open.iter().find(|(_, p)| p == name) {
                    Some((d, _)) => m.current = Some(*d),
                    None => {
                        m.open.push((m.next, name.clone()));
                        m.current = Some(m.next);
                        m.next += 1;
                    }
                }
            }
            2 => match pick_open(&mut rng, &m) {
                Some(d) => {
                    script.push(click_tab(&format!("{id}t"), &m.path(d)));
                    m.current = Some(d);
                }
                None => continue,
            },
            3 => match pick_open(&mut rng, &m) {
                Some(d) => {
                    script.push(click_entry(&format!("{id}e"), &m.path(d)));
                    m.current = Some(d);
                }
                None => continue,
            },
            4 => match pick_open(&mut rng, &m) {
                Some(d) => {
                    script.push(click_close(&format!("{id}x"), &m.path(d)));
                    if m.dirty.contains(&d) {
                        m.discard.insert(d);
                    } else {
                        m.close(d);
                    }
                }
                None => continue,
            },
            5 => match m.discard.iter().next().copied() {
                Some(d) => {
                    script.push(click_discard(&format!("{id}d"), &m.path(d)));
                    m.close(d);
                }
                None => continue,
            },
            6 | 7 => match m.current {
                Some(d) => {
                    script.push(click(&format!("{id}c"), json!({"label": "Editor"})));
                    if choice == 6 {
                        script.push(type_text(&format!("{id}y"), "q"));
                        m.dirty.insert(d);
                    } else {
                        script.push(key(&format!("{id}s"), "S", &["ctrl"]));
                        m.dirty.remove(&d);
                        m.discard.remove(&d);
                    }
                }
                None => continue,
            },
            _ => {
                if m.open.is_empty() {
                    continue;
                }
                let dir = if rng.below(2) == 0 { "down" } else { "up" };
                script.push(ui(
                    &format!("{id}r"),
                    "scroll",
                    json!({"target": {"label": "Tabs"}, "direction": dir, "steps": 1}),
                ));
            }
        }
        let snap = format!("{id}snap");
        script.push(snapshot(&snap));
        let title = m.current.map(|d| m.path(d));
        checks.push((snap, m.expected(), title));
    }
    script.push(ctl("final", "doc.list", json!({})));
    (names, script, checks)
}

#[test]
fn generated_sessions_follow_the_model() {
    for seed in 1..=40u64 {
        let (names, script, checks) = generated(seed);
        let dir = case_dir(&format!("gen-{seed}"));
        for n in &names {
            write(&dir.join(ROOT).join(n), b"t\n");
        }
        let copy = dir.join("replay");
        copy_tree(&dir.join(ROOT), &copy);
        let s = run(&dir, "s", &script, &[]);
        let what = |snap: &str| format!("seed {seed}, steps {}, at {snap}", checks.len());
        for (line, resp) in s.script.iter().zip(&s.resp) {
            assert_eq!(
                resp["ok"],
                json!(true),
                "seed {seed}: {line} → {}",
                short(resp)
            );
        }
        for (snap, want, title_path) in &checks {
            let root = s.snap(snap);
            assert_eq!(&shown(root, snap), want, "{}", what(snap));
            match title_path {
                Some(p) => assert!(
                    title(root).expect("Title").starts_with(p.as_str()),
                    "{}",
                    what(snap)
                ),
                None => assert!(title(root).is_none(), "{}", what(snap)),
            }
            if let Some(tabs) = tabs_node(root, snap) {
                for n in preorder(tabs) {
                    let l = label(n).unwrap_or_default();
                    if role(n) == Some("tab") && starts_ltr(l) && n["offscreen"] != json!(true) {
                        assert_eq!(row_texts(n), [l], "{}: rows", what(snap));
                    }
                }
            }
        }
        // The log replayed on a copy gives the same documents and files.
        let mut host = Host::on_disk(&copy).expect("Host::on_disk");
        for line in s.log.lines() {
            let r = host.handle_line(line.as_bytes()).expect("a response");
            assert!(
                r.contains(r#""ok":true"#),
                "seed {seed}: replay {line} → {r}"
            );
        }
        let list = host
            .handle_line(br#"{"command":"doc.list"}"#)
            .expect("doc.list");
        let list: Value = serde_json::from_str(&list).unwrap();
        assert_eq!(list["result"], *s.ok("final"), "seed {seed}: doc.list");
        let mut theirs = files_of(&copy);
        let mut ours = files_of(&dir.join(ROOT));
        theirs.retain(|k, _| !k.starts_with(".kiln-tmp"));
        ours.retain(|k, _| !k.starts_with(".kiln-tmp"));
        assert_eq!(theirs, ours, "seed {seed}: files");
        let _ = fs::remove_dir_all(&dir);
    }
}

/// A secondary click and modifiers on `Close`, `Discard` and a `Documents` entry act as a click;
/// a click on a tab leaves the editor unfocused and nothing focused.
#[test]
fn close_discard_and_entries_take_any_click() {
    let dir = case_dir("any-click");
    fixture(
        &dir,
        &[("a.txt", "a\n"), ("b.txt", "b\n"), ("c.txt", "c\n")],
    );
    let secondary = |id: &str, target: Value| {
        ui(
            id,
            "click",
            json!({"target": target, "button": "secondary"}),
        )
    };
    let modified = |id: &str, target: Value| {
        ui(
            id,
            "click",
            json!({"target": target, "modifiers": ["ctrl", "alt"]}),
        )
    };
    let script = vec![
        open("o1", "a.txt"),
        open("o2", "b.txt"),
        open("o3", "c.txt"),
        frames("f", 3),
        click("ce", json!({"label": "Editor"})),
        type_text("ty", "q"),
        click_tab("ct", "a.txt"),
        snapshot("s0"),
        secondary("x3", json!({"label": "Close c.txt"})),
        snapshot("s1"),
        modified("x1", json!({"label": "Close a.txt"})),
        snapshot("s2"),
        modified("e2", json!({"role": "label", "value": "c.txt"})),
        snapshot("s3"),
        secondary("d3", json!({"label": "Discard c.txt"})),
        snapshot("s4"),
    ];
    let s = run(&dir, "s", &script, &[]);
    s.all_ui_ok();
    let root = s.snap("s0");
    assert!(
        preorder(root).iter().all(|n| n.get("focused").is_none()),
        "s0: something focused after a tab click"
    );
    assert!(title(root).unwrap().starts_with("a.txt"));
    // c.txt is dirty: refused, Discard shown.
    let c = |snap: &str| {
        shown(s.snap(snap), snap)
            .into_iter()
            .find(|t| t.path == "c.txt")
    };
    assert!(c("s1").expect("c.txt").discard);
    assert!(message(s.snap("s1")).starts_with("unsaved_changes"));
    // a.txt (clean, current) closed with modifiers: c.txt (highest) current, Message cleared.
    assert_eq!(tab_labels(s.snap("s2"), "s2"), ["b.txt", "c.txt"]);
    assert!(title(s.snap("s2")).unwrap().starts_with("c.txt"));
    assert_eq!(message(s.snap("s2")), "");
    // A Documents entry with modifiers switches.
    assert!(c("s3").expect("c.txt").current);
    // A secondary click on Discard closes.
    assert_eq!(tab_labels(s.snap("s4"), "s4"), ["b.txt"]);
    let _ = fs::remove_dir_all(&dir);
}

/// A long path is painted on one row (a prefix of it, cut at the window's edge at most); a
/// `scroll` of the strip sends nothing.
#[test]
fn long_paths_and_scrolls() {
    let dir = case_dir("long");
    let long = format!("{}.txt", "x".repeat(120));
    let mut files: Vec<(String, &str)> = (0..40).map(|i| (format!("g{i:02}.txt"), "g\n")).collect();
    files.push((long.clone(), "l\n"));
    let refs: Vec<(&str, &str)> = files.iter().map(|(p, t)| (p.as_str(), *t)).collect();
    fixture(&dir, &refs);
    let mut script: Vec<String> = files
        .iter()
        .enumerate()
        .map(|(i, (p, _))| open(&format!("o{i}"), p))
        .collect();
    script.extend([
        frames("f", 5),
        snapshot("s0"),
        ui(
            "sc",
            "scroll",
            json!({"target": {"label": "Tabs"}, "direction": "down", "steps": 3}),
        ),
        frames("idle", 100),
    ]);
    let s = run(&dir, "s", &script, &[]);
    s.all_ui_ok();
    let root = s.snap("s0");
    let tab = preorder(root)
        .into_iter()
        .find(|n| role(n) == Some("tab") && label(n) == Some(long.as_str()))
        .expect("the long path's tab");
    let rows = row_texts(tab);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert!(long.starts_with(&rows[0]) && rows[0].len() > 20, "{rows:?}");
    let before = log_upto(&dir, "p0", &script, "s0", &[]);
    assert_eq!(
        s.log_entries().len(),
        before,
        "the scroll or idle frames logged something"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// Review round 1, h1: the `modified` mark of a dirty document whose path is wider than the
/// strip paints exactly `modified` on one row, and the tab's row holds its whole path at the
/// default size (a prefix of it, cut at the window's edge, in a narrow window).
#[test]
fn the_mark_of_a_wide_dirty_tab_is_one_row() {
    let dir = case_dir("wide-dirty");
    let wide = format!("d/{}.t", "x".repeat(180));
    fixture(
        &dir,
        &[("a.txt", "a\n"), (wide.as_str(), "w\n"), ("b.txt", "b\n")],
    );
    let script = vec![
        open("o1", "a.txt"),
        open("o2", &wide),
        open("o3", "b.txt"),
        frames("f", 2),
        click_tab("c", &wide),
        ui("fe", "focus", json!({"target": {"label": "Editor"}})),
        type_text("t", "Q"),
        snapshot("s"),
    ];
    for (size, whole) in [(None, true), (Some("800x320"), false)] {
        let extra: Vec<&str> = size.map_or_else(Vec::new, |s| vec!["--size", s]);
        let s = run(&dir, "s", &script, &extra);
        s.all_ui_ok();
        let root = s.snap("s");
        let what = format!("size {size:?}");
        let tab = preorder(root)
            .into_iter()
            .find(|n| role(n) == Some("tab") && label(n) == Some(wide.as_str()))
            .expect("the wide tab");
        let rows = row_texts(tab);
        assert_eq!(rows.len(), 1, "{what}: tab rows {rows:?}");
        if whole {
            assert_eq!(rows[0], wide, "{what}: the tab's row");
        } else {
            assert!(
                wide.starts_with(&rows[0]) && rows[0].len() > 20,
                "{what}: {rows:?}"
            );
        }
        let marks: Vec<&Value> = preorder(root)
            .into_iter()
            .filter(|n| value(n) == Some("modified"))
            .collect();
        assert_eq!(marks.len(), 1, "{what}");
        assert_eq!(row_texts(marks[0]), ["modified"], "{what}: the mark's rows");
        assert!(shown(root, &what)[1].modified, "{what}");
    }
    let _ = fs::remove_dir_all(&dir);
}
