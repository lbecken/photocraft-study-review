//! T-056 acceptance tests: the editor view of `kiln script` (`tasks/T-056-editor-view.md`,
//! Interface, Pinned, AC-1 to AC-8).
//!
//! Every test drives the built `kiln` binary (`CARGO_BIN_EXE_kiln`) through `kiln script` with
//! temporary roots under `CARGO_TARGET_TMPDIR`; every spawned process is killed after
//! [`TIMEOUT`] (D-028 (8)). Nodes are found by role and label (`Title`, `Editor`, `Message`),
//! never by a whole snapshot, and the effect is checked against an independent source: control
//! requests in the same session (`doc.text`, `doc.line`, `doc.list`, `doc.info`, `cursor.get`,
//! `syntax.highlight`, `eol.counts`), `kiln_control::Host` replaying the session log on a copy
//! of the root (the one Rust API the spec allows, as oracle), the files on disk, or a naive model
//! of the Input table and `docs/protocol/cursor.md`.
//!
//! No layout is assumed beyond what the spec pins (T-055 ruling w7): rows are mapped to lines by
//! their text, never by index; an empty line has no row. Colours are compared with the table the
//! spec publishes (Q-8 A; the seven values below are copied from the spec). No test pins a
//! message text, the usage text, a whole snapshot, the command list or `error_kinds` (golden
//! rule 9, Pinned (f)).
#![cfg(test)]

use std::collections::{BTreeMap, HashMap};
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

/// `src/a.rs` of Pinned (g).
const A_RS: &str = "fn main() {\n    let x = 1;\n}\n";

/// The published colour of each `syntax.highlight` kind (Interface, "Colours"; Q-8 A).
const COLOURS: [(&str, &str); 7] = [
    ("plain", "#dcdcdc"),
    ("keyword", "#569cd6"),
    ("type", "#4ec9b0"),
    ("function", "#dcdcaa"),
    ("string", "#ce9178"),
    ("number", "#b5cea8"),
    ("comment", "#6a9955"),
];

fn colour_of(kind: &str) -> &'static str {
    COLOURS
        .iter()
        .find(|(k, _)| *k == kind)
        .map(|(_, c)| *c)
        .unwrap_or_else(|| panic!("syntax.highlight reported kind `{kind}`, not in the table"))
}

const PLAIN: &str = "#dcdcdc";

/// The mutating commands of the Input table ("Dispatches" column), the only mutating commands
/// the UI may log besides what the script sent (AC-7).
const INPUT_COMMANDS: [&str; 8] = [
    "doc.insert",
    "doc.replace",
    "doc.delete",
    "cursor.set",
    "cursor.move",
    "doc.undo",
    "doc.redo",
    "doc.save",
];

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

/// `kiln <args>` in `cwd` under the hang detector, stdin closed, stdout and stderr captured.
fn kiln(cwd: &Path, args: &[&str]) -> Out {
    let what = format!("kiln {}", args.join(" "));
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kiln"));
    cmd.args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
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
    let status = wait_with_timeout(&mut child, &what);
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

/// A fresh, empty directory for one test case.
fn case_dir(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("t056-acceptance")
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

fn editor_target() -> Value {
    json!({"label": "Editor"})
}

fn focus_editor(id: &str) -> String {
    ui(id, "focus", json!({"target": editor_target()}))
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

fn click_at(id: &str, row: usize, col: usize, shift: bool) -> String {
    let mut f = json!({"target": editor_target(), "at": {"row": row, "col": col}});
    if shift {
        f["modifiers"] = json!(["shift"]);
    }
    ui(id, "click", f)
}

fn scroll_editor(id: &str, direction: &str, steps: u64) -> String {
    ui(
        id,
        "scroll",
        json!({"target": editor_target(), "direction": direction, "steps": steps}),
    )
}

fn open(id: &str, path: &str) -> String {
    ctl(id, "doc.open", json!({"path": path}))
}

/// A marker for the log: `project.tree` with a `limit` no view read uses (read-only), so that
/// the script's own requests can be found in the session log (AC-6, AC-7).
fn marker(id: &str, n: u64) -> String {
    ctl(id, "project.tree", json!({"limit": 900_000 + n}))
}

// ---------------------------------------------------------------------------------------------
// Sessions.
// ---------------------------------------------------------------------------------------------

/// One `kiln script` run.
struct Sess {
    name: String,
    out: Out,
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
            "{}: UI action `{id}` refused: {r}",
            self.name
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
            "{}: snapshot `{id}` refused: {r}",
            self.name
        );
        &r["result"]["root"]
    }

    fn text(&self, id: &str) -> String {
        self.ok(id)["text"]
            .as_str()
            .expect("doc.text text")
            .to_owned()
    }

    /// `(anchor, head)` of a `cursor.get` response.
    fn sel(&self, id: &str) -> (usize, usize) {
        let r = self.ok(id);
        (as_usize(&r["anchor"]), as_usize(&r["head"]))
    }

    fn log_entries(&self) -> Vec<Value> {
        self.log
            .lines()
            .map(|l| serde_json::from_str(l).expect("log line is JSON"))
            .collect()
    }
}

fn as_usize(v: &Value) -> usize {
    usize::try_from(
        v.as_u64()
            .unwrap_or_else(|| panic!("not an unsigned integer: {v}")),
    )
    .expect("fits usize")
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
        out,
        script: script.to_vec(),
        resp,
        by_id,
        log,
    }
}

fn root_args() -> [&'static str; 2] {
    ["--root", "proj"]
}

// ---------------------------------------------------------------------------------------------
// Snapshot nodes.
// ---------------------------------------------------------------------------------------------

fn children(n: &Value) -> &[Value] {
    n["children"].as_array().map_or(&[][..], Vec::as_slice)
}

fn all_nodes(root: &Value) -> Vec<&Value> {
    let mut out = vec![root];
    let mut i = 0;
    while i < out.len() {
        let n = out[i];
        out.extend(children(n).iter());
        i += 1;
    }
    out
}

fn label(n: &Value) -> Option<&str> {
    n["label"].as_str()
}

fn by_label<'a>(root: &'a Value, l: &str) -> Vec<&'a Value> {
    all_nodes(root)
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

/// The one node labelled `Editor`, of role `multiline_text_input`.
fn editor(root: &Value) -> &Value {
    let found = by_label(root, "Editor");
    assert_eq!(
        found.len(),
        1,
        "expected exactly one node labelled `Editor`, found {}: {}",
        found.len(),
        short(root)
    );
    assert_eq!(
        found[0]["role"],
        json!("multiline_text_input"),
        "Editor role: {}",
        short(found[0])
    );
    found[0]
}

/// The one node labelled `Title`; its value.
fn title(root: &Value) -> String {
    let found = by_label(root, "Title");
    assert_eq!(
        found.len(),
        1,
        "expected exactly one node labelled `Title`, found {}: {}",
        found.len(),
        short(root)
    );
    found[0]["value"].as_str().unwrap_or_default().to_owned()
}

/// `Title` begins with `path`, and the part after the path contains `modified` iff `dirty`.
fn check_title(root: &Value, path: &str, dirty: bool, what: &str) {
    let t = title(root);
    assert!(
        t.starts_with(path),
        "{what}: Title `{t}` does not begin with `{path}`"
    );
    assert_eq!(
        t[path.len()..].contains("modified"),
        dirty,
        "{what}: Title `{t}` vs doc.info dirty {dirty}"
    );
}

fn message(root: &Value) -> String {
    let found = by_label(root, "Message");
    assert_eq!(found.len(), 1, "one node labelled Message: {}", short(root));
    found[0]["value"].as_str().unwrap_or_default().to_owned()
}

/// Neither a `Title` nor an `Editor` node.
fn no_editor(root: &Value, what: &str) {
    assert!(
        by_label(root, "Editor").is_empty() && by_label(root, "Title").is_empty(),
        "{what}: with no document open there is no Title and no Editor: {}",
        short(root)
    );
}

/// `selection` of `Editor` as `(anchor, head)`.
fn editor_sel(ed: &Value) -> (usize, usize) {
    let s = &ed["selection"];
    assert!(s.is_object(), "Editor without a selection: {}", short(ed));
    (as_usize(&s["anchor"]), as_usize(&s["head"]))
}

fn editor_value(ed: &Value) -> &str {
    ed["value"]
        .as_str()
        .unwrap_or_else(|| panic!("Editor without a value: {}", short(ed)))
}

/// Each painted row of `n` as its runs `(text, colour)`.
fn rows(n: &Value) -> Vec<Vec<(String, String)>> {
    n["rows"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    row.as_array()
                        .expect("row is an array")
                        .iter()
                        .map(|run| {
                            (
                                run["text"].as_str().expect("run text").to_owned(),
                                run["color"].as_str().expect("run color").to_owned(),
                            )
                        })
                        .collect()
                })
                .collect()
        })
        .unwrap_or_default()
}

fn row_texts(n: &Value) -> Vec<String> {
    rows(n)
        .iter()
        .map(|r| r.iter().map(|(t, _)| t.as_str()).collect())
        .collect()
}

/// The `Editor` node lies inside a `scroll_view` that contains no shell node.
fn check_editor_in_scroll_view(root: &Value) {
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
    let ed = editor(root);
    let mut chain = Vec::new();
    assert!(path_to(root, ed, &mut chain), "Editor is in the tree");
    let sv = chain
        .iter()
        .rev()
        .find(|n| n["role"] == json!("scroll_view"))
        .unwrap_or_else(|| panic!("Editor is inside no scroll_view: {}", short(root)));
    for n in all_nodes(sv) {
        if let Some(l) = label(n) {
            assert!(
                !["Path", "Open", "Message", "Documents"].contains(&l),
                "the scroll_view around Editor contains the shell node `{l}`"
            );
        }
    }
}

/// No node is focused.
fn nothing_focused(root: &Value, what: &str) {
    for n in all_nodes(root) {
        assert!(
            n.get("focused").is_none(),
            "{what}: a node is focused: {}",
            short(n)
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Text model: lines (doc-positions.md), rows to lines.
// ---------------------------------------------------------------------------------------------

/// A line: first char offset and its chars without the break.
#[derive(Clone, Debug)]
struct Line {
    start: usize,
    text: Vec<char>,
}

impl Line {
    fn end(&self) -> usize {
        self.start + self.text.len()
    }
    fn string(&self) -> String {
        self.text.iter().collect()
    }
}

/// Lines split at `\r\n`, a lone `\r` and `\n`.
fn lines_of(t: &[char]) -> Vec<Line> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < t.len() {
        match t[i] {
            '\r' if t.get(i + 1) == Some(&'\n') => {
                out.push(Line {
                    start,
                    text: t[start..i].to_vec(),
                });
                i += 2;
                start = i;
            }
            '\r' | '\n' => {
                out.push(Line {
                    start,
                    text: t[start..i].to_vec(),
                });
                i += 1;
                start = i;
            }
            _ => i += 1,
        }
    }
    out.push(Line {
        start,
        text: t[start..].to_vec(),
    });
    out
}

fn chars(s: &str) -> Vec<char> {
    s.chars().collect()
}

/// The line index of each painted row: the rows are consecutive document lines without the
/// empty ones (an empty line has no row), each row's text the line's first chars (all of them
/// when `exact`, else a non-empty prefix: a long line is cut at the right edge). Panics unless
/// exactly one placement fits.
fn map_rows(texts: &[String], lines: &[Line], exact: bool, what: &str) -> Vec<usize> {
    assert!(!texts.is_empty(), "{what}: Editor has no painted row");
    let fits = |row: &str, line: &Line| {
        let r = chars(row);
        !r.is_empty()
            && if exact {
                r == line.text
            } else {
                line.text.starts_with(&r)
            }
    };
    let mut placements = Vec::new();
    for first in 0..lines.len() {
        let mut at = first;
        let mut got = Vec::new();
        let mut good = true;
        for row in texts {
            while at < lines.len() && lines[at].text.is_empty() && at != first {
                at += 1;
            }
            if at >= lines.len() || !fits(row, &lines[at]) {
                good = false;
                break;
            }
            got.push(at);
            at += 1;
        }
        if good {
            placements.push(got);
        }
    }
    assert!(
        !placements.is_empty(),
        "{what}: the rows of Editor are not consecutive non-empty lines of the document \
         (each its line's text{}): rows {texts:?}",
        if exact { "" } else { " from its first char" }
    );
    assert_eq!(
        placements.len(),
        1,
        "{what}: the rows fit several places of the fixture (a test bug): {texts:?}"
    );
    placements.remove(0)
}

/// The non-empty lines among `0..=k` for some k: the rows of a view at the top.
fn check_rows_from_line0(texts: &[String], lines: &[Line], exact: bool, what: &str) -> Vec<usize> {
    let m = map_rows(texts, lines, exact, what);
    let first_non_empty = lines
        .iter()
        .position(|l| !l.text.is_empty())
        .expect("fixture has a non-empty line");
    assert_eq!(
        m[0], first_non_empty,
        "{what}: the first row is line {} (`{}`), not the first line of the document",
        m[0], texts[0]
    );
    m
}

// ---------------------------------------------------------------------------------------------
// Colours.
// ---------------------------------------------------------------------------------------------

/// The kind of each char offset, from a `syntax.highlight` result over the whole text.
fn kinds(highlight: &Value, len: usize) -> Vec<String> {
    let mut out = vec![String::new(); len];
    for s in highlight["spans"].as_array().expect("spans") {
        let (a, b) = (as_usize(&s["start"]), as_usize(&s["end"]));
        for slot in &mut out[a..b] {
            s["kind"].as_str().expect("kind").clone_into(slot);
        }
    }
    assert!(
        out.iter().all(|k| !k.is_empty()),
        "syntax.highlight spans do not cover the text"
    );
    out
}

/// Every char of every painted row of `ed` is painted `plain` (a document whose rows need not
/// be mapped to lines: every char has the same colour).
fn check_all_plain(ed: &Value, what: &str) {
    let rs = rows(ed);
    assert!(!rs.is_empty(), "{what}: Editor has no painted row");
    for row in rs {
        for (t, c) in &row {
            assert_eq!(c, PLAIN, "{what}: `{t}` is not painted plain (row {row:?})");
        }
    }
}

fn blank(c: char) -> bool {
    c == ' ' || c == '\t'
}

/// The colours the rule gives the chars of one row: a char that paints pixels has `want(i)`;
/// a space or tab the colour of the char before it in the row, at the start of the row of the
/// first one after it that paints pixels.
fn expected_row_colours(row: &[char], want: impl Fn(usize) -> String) -> Vec<String> {
    let mut out: Vec<Option<String>> = row
        .iter()
        .enumerate()
        .map(|(i, c)| (!blank(*c)).then(|| want(i)))
        .collect();
    // x3 (D-157): in a row of blanks only, the blanks are painted `plain`.
    let first = out
        .iter()
        .flatten()
        .next()
        .cloned()
        .unwrap_or_else(|| PLAIN.to_owned());
    let mut prev = first;
    for c in &mut out {
        match c {
            Some(x) => prev = x.clone(),
            None => *c = Some(prev.clone()),
        }
    }
    out.into_iter().map(Option::unwrap_or_default).collect()
}

/// The reported colour of each char of a row.
fn row_char_colours(row: &[(String, String)]) -> (Vec<char>, Vec<String>) {
    let mut cs = Vec::new();
    let mut cols = Vec::new();
    for (t, c) in row {
        for ch in t.chars() {
            cs.push(ch);
            cols.push(c.clone());
        }
    }
    (cs, cols)
}

/// Every painted row of `ed`, mapped to its line, has the colours of `kind_at` by the rule.
fn check_colours(ed: &Value, lines: &[Line], kind_at: &dyn Fn(usize) -> String, what: &str) {
    let rs = rows(ed);
    let texts = row_texts(ed);
    let m = map_rows(&texts, lines, false, what);
    for (row, line) in rs.iter().zip(&m) {
        let (cs, got) = row_char_colours(row);
        let start = lines[*line].start;
        let want = expected_row_colours(&cs, |i| colour_of(&kind_at(start + i)).to_owned());
        if let Some(i) = (0..cs.len()).find(|i| got[*i] != want[*i]) {
            panic!(
                "{what}: line {line} char {i} `{}` (offset {}, kind `{}`) is painted {} , \
                 expected {} (row {:?})",
                cs[i],
                start + i,
                kind_at(start + i),
                got[i],
                want[i],
                row
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Replay and the log (AC-6, AC-7).
// ---------------------------------------------------------------------------------------------

fn log_line(e: &Value) -> String {
    e.to_string()
}

/// Replay `log` with `Host` on `root`; every request must succeed again. The responses.
fn replay(root: &Path, log: &[Value]) -> (Host, Vec<Value>) {
    let mut host = Host::on_disk(root).expect("Host::on_disk on the replay copy");
    let mut out = Vec::new();
    for e in log {
        let line = log_line(e);
        let resp = host
            .handle_line(line.as_bytes())
            .expect("a response per line");
        let v: Value = serde_json::from_str(&resp).expect("JSON");
        assert_eq!(
            v["ok"],
            json!(true),
            "replaying the log: {line} answered {resp}"
        );
        out.push(v);
    }
    (host, out)
}

fn host_req(host: &mut Host, command: &str, params: Value) -> Value {
    let line = json!({"command": command, "params": params}).to_string();
    let resp = host.handle_line(line.as_bytes()).expect("response");
    serde_json::from_str(&resp).expect("JSON")
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
            Some((
                v["command"].as_str()?.to_owned(),
                v.get("params").cloned().unwrap_or_else(|| json!({})),
            ))
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
        let p = e.get("params").cloned().unwrap_or_else(|| json!({}));
        if k < mine.len() && mine[k].0 == c && mine[k].1 == p {
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

/// AC-7 on one session: the `--log` replayed with `Host` on a copy of the initial root gives
/// the same `doc.text`, `cursor.get` and `doc.info` for every document and the same files; every
/// logged request the script did not send is read-only or a mutating command of the Input
/// table; and the UI did log something (so that the check is not vacuous).
fn check_dispatch_only(s: &Sess, root: &Path, initial: &Path, docs: &[u64]) {
    for (line, resp) in s.script.iter().zip(&s.resp) {
        let v: Value = serde_json::from_str(line).expect("script line");
        if v.get("ui").is_some() {
            assert_eq!(
                resp["ok"],
                json!(true),
                "{}: UI request refused: {line} → {resp}",
                s.name
            );
        }
    }
    let flags = mutating_flags();
    let extra = ui_logged(s);
    assert!(
        extra
            .iter()
            .any(|e| flags.get(e["command"].as_str().unwrap_or_default()) == Some(&true)),
        "{}: the UI logged no mutating command (the session edited nothing through the UI)",
        s.name
    );
    for e in &extra {
        let c = e["command"].as_str().unwrap_or_default();
        let m = *flags
            .get(c)
            .unwrap_or_else(|| panic!("{}: logged command `{c}` is not in commands.json", s.name));
        assert!(
            !m || INPUT_COMMANDS.contains(&c),
            "{}: the UI logged the mutating command {e}, which is not in the Input table",
            s.name
        );
    }
    let copy = root.with_file_name(format!(
        "{}-replay",
        root.file_name().expect("name").to_string_lossy()
    ));
    let _ = fs::remove_dir_all(&copy);
    copy_dir(initial, &copy);
    let (mut host, _) = replay(&copy, &s.log_entries());
    for d in docs {
        let p = json!({"doc": d});
        let text = host_req(&mut host, "doc.text", p.clone());
        let cur = host_req(&mut host, "cursor.get", p.clone());
        let info = host_req(&mut host, "doc.info", p.clone());
        assert_eq!(
            text["result"]["text"],
            s.ok(&format!("end-text-{d}"))["text"],
            "{}: doc.text of doc {d} after the replay",
            s.name
        );
        assert_eq!(
            cur["result"],
            *s.ok(&format!("end-cur-{d}")),
            "{}: cursor.get of doc {d} after the replay",
            s.name
        );
        let orig = s.ok(&format!("end-info-{d}"));
        for k in ["dirty", "can_undo", "can_redo", "len_chars", "line_ending"] {
            assert_eq!(
                info["result"][k], orig[k],
                "{}: doc.info `{k}` of doc {d} after the replay",
                s.name
            );
        }
    }
    assert_eq!(
        tree(&copy),
        tree(root),
        "{}: files after the replay",
        s.name
    );
    let _ = fs::remove_dir_all(&copy);
}

/// The final reads every AC-7 session ends with.
fn end_reads(docs: &[u64]) -> Vec<String> {
    let mut out = vec![ctl("end-list", "doc.list", json!({}))];
    for d in docs {
        out.push(ctl(&format!("end-text-{d}"), "doc.text", json!({"doc": d})));
        out.push(ctl(
            &format!("end-cur-{d}"),
            "cursor.get",
            json!({"doc": d}),
        ));
        out.push(ctl(&format!("end-info-{d}"), "doc.info", json!({"doc": d})));
    }
    out
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

    fn flag(&mut self) -> bool {
        self.below(2) == 1
    }
}

// =============================================================================================
// AC-1: the document shown (oracle: doc.text, doc.line, doc.list in the same session).
// =============================================================================================

/// The checks of AC-1 on a snapshot that shows `src/a.rs` (doc `doc`), unmodified.
fn check_a_rs_shown(s: &Sess, snap_id: &str, text_id: &str, line_ids: &[String], what: &str) {
    let root = s.snap(snap_id);
    check_title(root, "src/a.rs", false, what);
    let ed = editor(root);
    let text = s.text(text_id);
    assert_eq!(text, A_RS, "{what}: doc.text");
    assert_eq!(editor_value(ed), text, "{what}: Editor value vs doc.text");
    // Rows equal the document's lines from line 0 (doc.line in the same session).
    let lines: Vec<Line> = line_ids
        .iter()
        .map(|id| {
            let r = s.ok(id);
            Line {
                start: as_usize(&r["start"]),
                text: chars(r["text"].as_str().expect("line text")),
            }
        })
        .collect();
    let texts = row_texts(ed);
    check_rows_from_line0(&texts, &lines, true, what);
    let expected: Vec<String> = lines
        .iter()
        .filter(|l| !l.text.is_empty())
        .map(Line::string)
        .collect();
    assert_eq!(
        texts, expected,
        "{what}: the rows of a 3-line document are its non-empty lines"
    );
    check_editor_in_scroll_view(root);
    let cur = s.sel("cur");
    assert_eq!(
        editor_sel(ed),
        cur,
        "{what}: Editor selection vs cursor.get"
    );
}

fn ac1_fixture(dir: &Path) -> PathBuf {
    let root = dir.join("proj");
    write(&root.join("src/a.rs"), A_RS.as_bytes());
    write(&root.join("a.rs"), b"fn a() {}\n");
    write(&root.join("b.rs"), b"fn b() { let bee = 2; }\n");
    write(&root.join("c.rs"), b"// c\nfn c() {}\n");
    write(&root.join("modified.rs"), b"fn m() {}\n");
    root
}

/// The reads after which [`check_a_rs_shown`] runs.
fn a_rs_reads() -> (Vec<String>, Vec<String>) {
    let mut lines = vec![
        snapshot("s"),
        ctl("text", "doc.text", json!({"doc": 1})),
        ctl("cur", "cursor.get", json!({"doc": 1})),
        ctl("list", "doc.list", json!({})),
    ];
    let mut ids = Vec::new();
    for l in 0..4 {
        let id = format!("line{l}");
        lines.push(ctl(&id, "doc.line", json!({"doc": 1, "line": l})));
        ids.push(id);
    }
    (lines, ids)
}

#[test]
fn ac1_start_path_shows_the_document() {
    let dir = case_dir("ac1-start");
    ac1_fixture(&dir);
    let (script, ids) = a_rs_reads();
    let s = run(&dir, "s", &script, &["--root", "proj", "src/a.rs"]);
    check_a_rs_shown(&s, "s", "text", &ids, "start path");
    nothing_focused(s.snap("s"), "a start path");
    let docs = &s.ok("list")["docs"];
    assert_eq!(docs[0]["path"], json!("src/a.rs"), "doc.list: {docs}");
    let opens: Vec<Value> = s
        .log_entries()
        .into_iter()
        .filter(|e| e["command"] == json!("doc.open"))
        .collect();
    assert_eq!(opens.len(), 1, "the start path is logged as one doc.open");
    assert_eq!(opens[0]["params"], json!({"path": "src/a.rs"}));
    assert_eq!(
        s.out.code, 0,
        "exit code of a script whose responses are all ok"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn ac1_path_and_open_show_the_document() {
    let dir = case_dir("ac1-path");
    ac1_fixture(&dir);
    let mut script = vec![
        ui("f", "focus", json!({"target": {"label": "Path"}})),
        type_text("t", "src/a.rs"),
        ui("c", "click", json!({"target": {"label": "Open"}})),
    ];
    let (reads, ids) = a_rs_reads();
    script.extend(reads);
    let s = run(&dir, "s", &script, &root_args());
    for id in ["f", "t", "c"] {
        s.act(id);
    }
    check_a_rs_shown(&s, "s", "text", &ids, "Path and Open");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn ac1_control_open_shows_the_document_after_one_frame() {
    let dir = case_dir("ac1-control");
    ac1_fixture(&dir);
    let mut script = vec![snapshot("s0"), open("o", "src/a.rs"), frames("f", 1)];
    let (reads, ids) = a_rs_reads();
    script.extend(reads);
    let s = run(&dir, "s", &script, &root_args());
    no_editor(s.snap("s0"), "a fresh session");
    s.ok("o");
    s.act("f");
    check_a_rs_shown(&s, "s", "text", &ids, "control doc.open");
    nothing_focused(s.snap("s"), "an opened document");
    let _ = fs::remove_dir_all(&dir);
}

/// Start paths `a.rs b.rs missing.rs`: `b.rs` is current, `Message` begins with `not_found`,
/// the log holds the two opens, no response line for a start path, the exit code is the
/// script's own (0 here, 3 with a refused control request); options may come between paths.
#[test]
fn ac1_start_paths_in_order_last_success_is_current() {
    let dir = case_dir("ac1-paths");
    ac1_fixture(&dir);
    let script = vec![
        snapshot("s"),
        ctl("text", "doc.text", json!({"doc": 2})),
        ctl("list", "doc.list", json!({})),
    ];
    for (name, args) in [
        (
            "before",
            vec!["--root", "proj", "a.rs", "b.rs", "missing.rs"],
        ),
        (
            "between",
            vec!["a.rs", "--root", "proj", "b.rs", "missing.rs"],
        ),
    ] {
        let s = run(&dir, name, &script, &args);
        let root = s.snap("s");
        check_title(root, "b.rs", false, name);
        assert_eq!(
            editor_value(editor(root)),
            s.text("text"),
            "{name}: Editor shows b.rs (doc 2)"
        );
        let m = message(root);
        assert!(
            m.starts_with("not_found"),
            "{name}: Message `{m}` after a refused start path"
        );
        let paths: Vec<&Value> = s.ok("list")["docs"]
            .as_array()
            .expect("docs")
            .iter()
            .map(|d| &d["path"])
            .collect();
        assert_eq!(paths, [&json!("a.rs"), &json!("b.rs")], "{name}: doc.list");
        let opens: Vec<Value> = s
            .log_entries()
            .into_iter()
            .filter(|e| e["command"] == json!("doc.open"))
            .map(|e| e["params"]["path"].clone())
            .collect();
        assert_eq!(
            opens,
            [json!("a.rs"), json!("b.rs")],
            "{name}: logged opens"
        );
        assert_eq!(
            s.out.code, 0,
            "{name}: a refused start path does not change the exit code"
        );
    }
    // A refused control request of the script makes the exit code 3, start paths or not.
    let bad = vec![snapshot("s"), open("o", "nope.rs")];
    let s = run(&dir, "bad", &bad, &["--root", "proj", "a.rs"]);
    assert_eq!(s.r("o")["ok"], json!(false));
    assert_eq!(s.out.code, 3, "exit code with a refused control request");
    // A successful start path after a refused one clears Message.
    let s = run(
        &dir,
        "clear",
        &[snapshot("s")],
        &["--root", "proj", "missing.rs", "a.rs"],
    );
    assert_eq!(
        message(s.snap("s")),
        "",
        "Message after a successful start path"
    );
    check_title(s.snap("s"), "a.rs", false, "clear");
    // With --memory every start path is refused (not_found).
    let s = run(&dir, "memory", &[snapshot("s")], &["--memory", "a.rs"]);
    let root = s.snap("s");
    assert!(
        message(root).starts_with("not_found"),
        "--memory: Message `{}`",
        message(root)
    );
    no_editor(root, "--memory");
    assert_eq!(s.out.code, 0, "--memory: exit code");
    let _ = fs::remove_dir_all(&dir);
}

/// The current document: a second open makes a document current (`already_open: true`), a
/// close of the current one makes the open document with the highest id current, none open
/// shows no `Editor`.
#[test]
fn ac1_current_document_rule() {
    let dir = case_dir("ac1-current");
    ac1_fixture(&dir);
    let script = vec![
        open("o1", "a.rs"),
        open("o2", "b.rs"),
        open("o3", "c.rs"),
        frames("f0", 1),
        snapshot("s-c"),
        open("o2b", "b.rs"),
        frames("f1", 1),
        snapshot("s-b"),
        open("o1b", "a.rs"),
        frames("f2", 1),
        snapshot("s-a"),
        ctl("x1", "doc.close", json!({"doc": 1})),
        frames("f3", 1),
        snapshot("s-after-a"),
        ctl("x3", "doc.close", json!({"doc": 3})),
        frames("f4", 1),
        snapshot("s-after-c"),
        ctl("x2", "doc.close", json!({"doc": 2})),
        frames("f5", 1),
        snapshot("s-none"),
        ctl("t1", "doc.text", json!({"doc": 2})),
    ];
    let s = run(&dir, "s", &script, &root_args());
    assert_eq!(s.ok("o2b")["already_open"], json!(true));
    let files: HashMap<&str, String> = [
        ("a.rs", "fn a() {}\n"),
        ("b.rs", "fn b() { let bee = 2; }\n"),
        ("c.rs", "// c\nfn c() {}\n"),
    ]
    .into_iter()
    .map(|(p, t)| (p, t.to_owned()))
    .collect();
    for (snap, path) in [
        ("s-c", "c.rs"),
        ("s-b", "b.rs"),
        ("s-a", "a.rs"),
        // Closing a.rs (doc 1): open are b.rs (2) and c.rs (3); the highest id is c.rs, not
        // the document current before a.rs (b.rs).
        ("s-after-a", "c.rs"),
        ("s-after-c", "b.rs"),
    ] {
        let root = s.snap(snap);
        check_title(root, path, false, snap);
        assert_eq!(
            editor_value(editor(root)),
            files[path],
            "{snap}: Editor shows {path}"
        );
    }
    no_editor(s.snap("s-none"), "after closing every document");
    let _ = fs::remove_dir_all(&dir);
}

/// `Title` looks for `modified` after the path only (B7), and is not fooled by a path that
/// contains the word.
#[test]
fn ac1_title_of_a_path_containing_modified() {
    let dir = case_dir("ac1-title");
    ac1_fixture(&dir);
    let script = vec![
        snapshot("s0"),
        ctl("i0", "doc.info", json!({"doc": 1})),
        ctl(
            "ins",
            "doc.insert",
            json!({"doc": 1, "offset": 0, "text": "x"}),
        ),
        frames("f", 1),
        snapshot("s1"),
        ctl("i1", "doc.info", json!({"doc": 1})),
    ];
    let s = run(&dir, "s", &script, &["--root", "proj", "modified.rs"]);
    assert_eq!(s.ok("i0")["dirty"], json!(false));
    check_title(s.snap("s0"), "modified.rs", false, "clean");
    assert_eq!(s.ok("i1")["dirty"], json!(true));
    check_title(s.snap("s1"), "modified.rs", true, "after an insert");
    assert_eq!(editor_value(editor(s.snap("s1"))), "xfn m() {}\n");
    let _ = fs::remove_dir_all(&dir);
}

/// A focus or a click on `Editor` focuses it (nothing else does).
#[test]
fn ac1_editor_is_focused_by_focus_or_click() {
    let dir = case_dir("ac1-focus");
    ac1_fixture(&dir);
    for (name, act) in [
        ("focus", focus_editor("a")),
        (
            "click",
            ui("a", "click", json!({"target": editor_target()})),
        ),
    ] {
        let script = vec![
            open("o", "src/a.rs"),
            frames("f", 1),
            snapshot("s0"),
            act,
            snapshot("s1"),
        ];
        let s = run(&dir, name, &script, &root_args());
        nothing_focused(s.snap("s0"), name);
        s.act("a");
        let ed = editor(s.snap("s1"));
        assert_eq!(
            ed["focused"],
            json!(true),
            "{name}: Editor focused: {}",
            short(ed)
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// Lines are not wrapped and the view does not scroll horizontally: a long line is one row, a
/// non-empty prefix of the line; an empty line has no row.
#[test]
fn ac1_long_line_is_cut_not_wrapped() {
    let dir = case_dir("ac1-long");
    let root = dir.join("proj");
    let long: String = (0..3000)
        .map(|i| char::from(b'a' + (i % 26) as u8))
        .collect();
    let text = format!("short line\n\n{long}\nend line\n");
    write(&root.join("long.txt"), text.as_bytes());
    let script = vec![
        open("o", "long.txt"),
        frames("f", 1),
        snapshot("s0"),
        scroll_editor("r", "right", 5),
        snapshot("s1"),
    ];
    let s = run(&dir, "s", &script, &root_args());
    let ed = editor(s.snap("s0"));
    let texts = row_texts(ed);
    let lines = lines_of(&chars(&text));
    let m = check_rows_from_line0(&texts, &lines, false, "long line");
    assert_eq!(
        m,
        [0, 2, 3],
        "rows: one per non-empty line, no wrapped row: {texts:?}"
    );
    assert_eq!(texts[0], "short line");
    assert!(
        texts[1].chars().count() < 3000,
        "a 3000-char line is cut at the right edge"
    );
    assert_eq!(texts[2], "end line");
    // A horizontal scroll (accepted or refused) shows the same rows.
    assert_eq!(
        row_texts(editor(s.snap("s1"))),
        texts,
        "the view scrolled horizontally"
    );
    let _ = fs::remove_dir_all(&dir);
}

// =============================================================================================
// AC-2: colours (oracle: syntax.highlight over the rows in the same session, the table).
// =============================================================================================

/// 20 Rust files (keywords, types, functions, strings, numbers, comments, blanks inside and at
/// the start of lines, CRLF, non-ASCII), each long enough to scroll. The first is Pinned (g)'s
/// `src/a.rs`.
fn rust_corpus() -> Vec<(String, String)> {
    let bodies: [&str; 20] = [
        A_RS,
        "pub struct Point {\n    pub x: i32,\n    y: f64,\n}\nimpl Point {\n    pub fn new() -> Self { Self { x: 0, y: 0.5 } }\n}\n",
        "enum E { A, B(u8) }\nfn pick(e: E) -> &'static str {\n    match e { E::A => \"a\\\"b\", E::B(_) => \"é\" }\n}\n",
        "fn raw() {\n    let r = r#\"raw \"q\" λ\"#;\n    let b = b\"bytes\";\n    let c = 'ж';\n    let bc = b'x';\n}\n",
        "//! Crate doc é.\n/// Item doc λ.\nfn d() {}\n/* block\n   comment ж */\nfn e() { /* inner */ }\n",
        "fn nums() {\n    let a = 1_000;\n    let b = 0b1010;\n    let c = 1.5f32;\n    let d = 2e10;\n    let h = 0xff_u8;\n}\n",
        "fn generic<'a, T: Clone>(x: &'a T) -> T\nwhere\n    T: Default,\n{\n    x.clone()\n}\ntrait Tr { fn m(&self); }\n",
        "fn mac() {\n    println!(\"{} λ\", 1);\n    let v = vec![1, 2, 3];\n    assert_eq!(v.len(), 3);\n}\n",
        "async fn run() {\n    loop { break; }\n    while false {}\n    for i in 0..3 { let _ = i; }\n    other().await;\n}\n",
        "static mut COUNT: u32 = 0;\nextern \"C\" { fn abs(x: i32) -> i32; }\nunsafe fn u() { COUNT += 1; }\n",
        "use std::collections::HashMap;\nmod inner { pub fn f() {} }\nfn g() { crate::inner::f(); super::h(); }\n",
        "fn closures() {\n    let add = |x: i32| x + 1;\n    let m = move || add(2);\n    m();\n}\n",
        "fn unicode() {\n    let λ = \"é λ ж\";\n    let жук = 'é'; // ж comment\n}\n",
        "fn tabs() {\n\tlet a = 1;\n\t\tlet b =\t2;\n\tcall(a,\tb);\n}\n",
        "fn spaces() {\n    let   a   =   1;\n        let b = a  +  2   ;\n    f  (  a  )  ;\n}\n",
        "const MAX: usize = 10;\ntype Alias = Vec<String>;\nimpl Default for Thing { fn default() -> Self { Thing } }\n",
        "#[derive(Debug, Clone)]\n#[allow(dead_code)]\nstruct Thing;\n#[inline]\nfn attr() -> bool { true }\n",
        "fn nested(o: Option<u8>) {\n    if let Some(v) = o {\n        if v > 1 { return; } else if v == 0 { } else { }\n    }\n}\n",
        "fn fallible() -> Result<(), String> {\n    let n: i64 = \"5\".parse().map_err(|e| format!(\"{e}\"))?;\n    Ok(())\n}\n",
        "#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() { assert!(true); }\n}\n",
    ];
    let pads = [
        "    let v{k} = {k} + 0x{k}; // é {k}",
        "\tlet s{k}: &str = \"λ{k} ж\";",
        "    if v{k} > 1.5e3 {{ return; }} /* c{k} */",
        "    call_{k}(x.method_{k}(), Vec::<u8>::new());",
        "    let c{k} = 'ж'; let t{k}: String = String::new();",
        "  \t  let w{k} = Some({k}).unwrap_or(0) as u64;",
        "    // comment line {k} with  two  spaces",
    ];
    let mut out = Vec::new();
    for (i, body) in bodies.iter().enumerate() {
        let mut text = (*body).to_owned();
        if i > 0 {
            text.push_str(&format!("fn pad_{i}() {{\n"));
            for k in 0..70 {
                if k % 9 == 4 {
                    text.push('\n');
                    continue;
                }
                let t = pads[(k + i) % pads.len()].replace("{k}", &format!("{i}_{k}"));
                let t = t.replace("{{", "{").replace("}}", "}");
                text.push_str(&t);
                text.push('\n');
            }
            text.push_str("} // end\n");
        }
        if i % 4 == 2 {
            text = text.replace('\n', "\r\n");
        }
        let path = if i == 0 {
            "src/a.rs".to_owned()
        } else {
            format!("src/f{i:02}.rs")
        };
        out.push((path, text));
    }
    out
}

#[test]
fn ac2_colours_of_rust_files() {
    let dir = case_dir("ac2-rust");
    let root = dir.join("proj");
    let corpus = rust_corpus();
    for (p, t) in &corpus {
        write(&root.join(p), t.as_bytes());
    }
    let mut script = Vec::new();
    for (i, (p, _)) in corpus.iter().enumerate() {
        let d = i + 1;
        script.push(open(&format!("o{d}"), p));
        script.push(frames(&format!("f{d}"), 1));
        script.push(snapshot(&format!("s{d}")));
        script.push(ctl(&format!("h{d}"), "syntax.highlight", json!({"doc": d})));
        script.push(ctl(&format!("t{d}"), "doc.text", json!({"doc": d})));
        if i % 6 == 5 {
            // A selection over painted text does not change colours.
            script.push(focus_editor(&format!("fo{d}")));
            for k in 0..3 {
                script.push(key(&format!("sd{d}-{k}"), "ArrowDown", &["shift"]));
            }
            for k in 0..4 {
                script.push(key(&format!("sr{d}-{k}"), "ArrowRight", &["shift"]));
            }
            script.push(snapshot(&format!("sel{d}")));
            script.push(ctl(&format!("c{d}"), "cursor.get", json!({"doc": d})));
        }
        script.push(scroll_editor(&format!("sc{d}"), "down", 2));
        script.push(snapshot(&format!("ss{d}")));
    }
    let s = run(&dir, "s", &script, &root_args());
    for (i, (p, file)) in corpus.iter().enumerate() {
        let d = i + 1;
        let text = s.text(&format!("t{d}"));
        assert_eq!(&text, file, "{p}: doc.text");
        let t = chars(&text);
        let lines = lines_of(&t);
        let k = kinds(s.ok(&format!("h{d}")), t.len());
        let kind_at = |o: usize| k[o].clone();
        let root = s.snap(&format!("s{d}"));
        check_title(root, p, false, p);
        let ed = editor(root);
        check_colours(ed, &lines, &kind_at, p);
        if i % 6 == 5 {
            let (a, h) = s.sel(&format!("c{d}"));
            assert_ne!(a, h, "{p}: precondition: shift+arrows selected text");
            let ed = editor(s.snap(&format!("sel{d}")));
            assert_eq!(
                editor_sel(ed),
                (a, h),
                "{p}: Editor selection vs cursor.get"
            );
            check_colours(ed, &lines, &kind_at, &format!("{p} with a selection"));
        }
        s.act(&format!("sc{d}"));
        let ed = editor(s.snap(&format!("ss{d}")));
        check_colours(ed, &lines, &kind_at, &format!("{p} after a scroll"));
    }
    // The worked example of Pinned (g): the first painted row of src/a.rs, runs exactly.
    let ed = editor(s.snap("s1"));
    let first = &rows(ed)[0];
    let want: Vec<(String, String)> =
        [("fn ", "#569cd6"), ("main", "#dcdcaa"), ("() {", "#dcdcdc")]
            .iter()
            .map(|(t, c)| ((*t).to_owned(), (*c).to_owned()))
            .collect();
    assert_eq!(
        first, &want,
        "Pinned (g): the first painted row of src/a.rs"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// A `.txt` document and a `.rs` document of 1,048,577 chars (`syntax.highlight` →
/// `too_large`) are painted all `plain`; the refused read is not shown in `Message`.
#[test]
fn ac2_plain_documents() {
    let dir = case_dir("ac2-plain");
    let root = dir.join("proj");
    let rusty = "fn main() { let s = \"str\"; } // comment 42\n\tlet  x: u8 = 0x1f;\n";
    write(&root.join("rusty.txt"), rusty.as_bytes());
    let unit = "fn f() -> u8 { 42 } // c\n"; // 25 chars
    let mut big = unit.repeat(1_048_577 / unit.len());
    while big.chars().count() < 1_048_577 {
        big.push('x');
    }
    assert_eq!(big.chars().count(), 1_048_577);
    write(&root.join("big.rs"), big.as_bytes());
    let script = vec![
        open("o1", "rusty.txt"),
        frames("f1", 1),
        snapshot("s1"),
        open("o2", "big.rs"),
        frames("f2", 1),
        snapshot("s2"),
        ctl(
            "h2",
            "syntax.highlight",
            json!({"doc": 2, "start": 0, "end": 10}),
        ),
        ctl("i2", "doc.info", json!({"doc": 2})),
        scroll_editor("sc", "down", 3),
        snapshot("s3"),
    ];
    let s = run(&dir, "s", &script, &root_args());
    let plain = |_: usize| "plain".to_owned();
    check_colours(
        editor(s.snap("s1")),
        &lines_of(&chars(rusty)),
        &plain,
        "rusty.txt",
    );
    assert_eq!(
        s.r("h2")["error"]["kind"],
        json!("too_large"),
        "precondition: big.rs is too large for syntax.highlight"
    );
    assert_eq!(as_usize(&s.ok("i2")["len_chars"]), 1_048_577);
    for id in ["s2", "s3"] {
        let root = s.snap(id);
        check_title(root, "big.rs", false, id);
        let ed = editor(root);
        assert!(
            row_texts(ed)
                .iter()
                .all(|t| unit.trim_end().starts_with(t.as_str()) || t.chars().all(|c| c == 'x')),
            "{id}: rows of big.rs are not its lines: {:?}",
            row_texts(ed)
        );
        check_all_plain(ed, &format!("big.rs {id}"));
        assert_eq!(
            message(root),
            "",
            "{id}: a refused read is not shown in Message"
        );
    }
    s.act("sc");
    let _ = fs::remove_dir_all(&dir);
}

// =============================================================================================
// AC-3: caret and pointer (oracle: cursor.get, doc.text, doc.line).
// =============================================================================================

/// A document of distinct non-empty lines with mixed line endings, tabs and non-ASCII chars.
fn click_fixture() -> String {
    let words = ["alpha", "βeta", "gamma", "дельта", "eps", "zéta"];
    let ends = ["\n", "\r\n", "\r", "\n"];
    let mut t = String::new();
    for k in 0..24 {
        let lead = if k % 5 == 1 {
            "\t"
        } else if k % 5 == 3 {
            "    "
        } else {
            ""
        };
        t.push_str(&format!(
            "{lead}{k:02} {} λ ж  {}",
            words[k % 6],
            "x".repeat(k % 7)
        ));
        t.push_str(ends[k % 4]);
    }
    t
}

/// One click of the AC-3 session.
struct ClickStep {
    pre: String,
    row: usize,
    col: usize,
    row_text: String,
    line: usize,
    shift: bool,
    typed: bool,
    head: usize,
    anchor: usize,
    text_after: Vec<char>,
}

/// The base of the AC-3 session: open the fixture as doc 1 and show it.
fn ac3_base() -> Vec<String> {
    vec![open("o", "click.txt"), frames("f", 1)]
}

fn ac3_step_lines(k: usize, st: &ClickStep) -> Vec<String> {
    let mut l = vec![
        snapshot(&st.pre),
        click_at(&format!("click{k}"), st.row, st.col, st.shift),
        snapshot(&format!("post{k}")),
        ctl(&format!("cur{k}"), "cursor.get", json!({"doc": 1})),
        ctl(
            &format!("line{k}"),
            "doc.line",
            json!({"doc": 1, "line": st.line}),
        ),
    ];
    if st.typed {
        l.push(type_text(&format!("type{k}"), "Q"));
        l.push(snapshot(&format!("tsnap{k}")));
        l.push(ctl(&format!("ttext{k}"), "doc.text", json!({"doc": 1})));
        l.push(ctl(&format!("tcur{k}"), "cursor.get", json!({"doc": 1})));
    }
    l
}

/// Build the AC-3 session step by step: each click's row and column are drawn from the rows
/// of the snapshot just before it (a run of the session so far; runs are deterministic, and
/// the final run re-checks each row). Returns the final script, its run and the steps.
fn ac3_session(dir: &Path) -> (Sess, Vec<ClickStep>) {
    let initial = chars(&click_fixture());
    let mut text = initial.clone();
    let mut anchor = 0;
    let mut rng = Rng(0x0056_0003);
    let mut steps: Vec<ClickStep> = Vec::new();
    let mut script = ac3_base();
    for k in 0..9 {
        let pre = format!("pre{k}");
        let mut probe = script.clone();
        probe.push(snapshot(&pre));
        let s = run(dir, &format!("probe{k}"), &probe, &root_args());
        let ed = editor(s.snap(&pre));
        let texts = row_texts(ed);
        let lines = lines_of(&text);
        let m = map_rows(&texts, &lines, false, &format!("before click {k}"));
        let row = rng.below(texts.len());
        let line = m[row];
        let row_len = texts[row].chars().count();
        let cut = row_len < lines[line].text.len();
        let col = rng.below(if cut { row_len } else { row_len + 1 });
        let shift = k % 3 == 2;
        let typed = k % 3 == 1;
        let head = lines[line].start + col;
        if !shift {
            anchor = head;
        }
        let mut after = text.clone();
        if typed {
            after.insert(head, 'Q');
        }
        let st = ClickStep {
            pre,
            row,
            col,
            row_text: texts[row].clone(),
            line,
            shift,
            typed,
            head,
            anchor,
            text_after: after.clone(),
        };
        script.extend(ac3_step_lines(k, &st));
        steps.push(st);
        if typed {
            anchor = head + 1;
        }
        text = after;
    }
    script.extend(end_reads(&[1]));
    let s = run(dir, "s", &script, &root_args());
    (s, steps)
}

fn ac3_root(dir: &Path) -> PathBuf {
    let root = dir.join("proj");
    write(&root.join("click.txt"), click_fixture().as_bytes());
    root
}

#[test]
fn ac3_click_at_row_and_col_moves_the_caret() {
    let dir = case_dir("ac3-click");
    ac3_root(&dir);
    let (s, steps) = ac3_session(&dir);
    let mut prev_head = 0;
    for (k, st) in steps.iter().enumerate() {
        let what = format!(
            "click {k} at row {} col {} (shift {})",
            st.row, st.col, st.shift
        );
        assert_eq!(
            row_texts(editor(s.snap(&st.pre)))[st.row],
            st.row_text,
            "{what}: the session is not deterministic"
        );
        s.act(&format!("click{k}"));
        let line = s.ok(&format!("line{k}"));
        let (a, h) = s.sel(&format!("cur{k}"));
        assert_eq!(
            h,
            as_usize(&line["start"]) + st.col,
            "{what}: cursor.get head vs doc.line start + col"
        );
        assert_eq!(h, st.head, "{what}: head");
        let want_anchor = if st.shift { st.anchor } else { h };
        assert_eq!(
            a, want_anchor,
            "{what}: anchor (shift keeps the old anchor)"
        );
        if st.shift {
            assert_eq!(a, prev_head, "{what}: shift-click keeps the anchor");
        }
        assert_eq!(
            editor_sel(editor(s.snap(&format!("post{k}")))),
            (a, h),
            "{what}: Editor selection vs cursor.get"
        );
        prev_head = h;
        if st.typed {
            s.act(&format!("type{k}"));
            let text = s.text(&format!("ttext{k}"));
            assert_eq!(
                chars(&text),
                st.text_after,
                "{what}: typing `Q` inserts it at the clicked offset {h}"
            );
            let (ta, th) = s.sel(&format!("tcur{k}"));
            assert_eq!(
                (ta, th),
                (h + 1, h + 1),
                "{what}: caret after the typed char"
            );
            let ed = editor(s.snap(&format!("tsnap{k}")));
            assert_eq!(editor_value(ed), text, "{what}: Editor value vs doc.text");
            assert_eq!(
                editor_sel(ed),
                (ta, th),
                "{what}: Editor selection after typing"
            );
            prev_head = th;
        }
    }
    let _ = fs::remove_dir_all(&dir);
}

/// 300 distinct non-empty lines.
fn lines300() -> String {
    (0..300)
        .map(|k| format!("line {k:03} {}\n", ["é", "λ", "ж", "ab"][k % 4]))
        .collect()
}

/// A click without `at` lands at the centre of the visible part of `Editor`: after scrolling
/// line 0 out of view, the head goes to a line that has a row.
#[test]
fn ac3_click_lands_in_the_visible_part() {
    let dir = case_dir("ac3-centre");
    let root = dir.join("proj");
    let text = lines300();
    write(&root.join("l.txt"), text.as_bytes());
    let script = vec![
        open("o", "l.txt"),
        frames("f", 1),
        scroll_editor("sc", "down", 10),
        snapshot("s"),
        ui("c", "click", json!({"target": editor_target()})),
        ctl("cur", "cursor.get", json!({"doc": 1})),
        snapshot("s2"),
    ];
    let s = run(&dir, "s", &script, &root_args());
    s.act("sc");
    let lines = lines_of(&chars(&text));
    let texts = row_texts(editor(s.snap("s")));
    let m = map_rows(&texts, &lines, false, "after scrolling");
    assert!(
        !m.contains(&0),
        "precondition: 10 scroll steps take line 0 out of view (rows {texts:?})"
    );
    s.act("c");
    let line = as_usize(&s.ok("cur")["line"]);
    assert!(
        m.contains(&line),
        "the click without `at` put the head on line {line}, which has no row in the \
         snapshot before it (rows are lines {}..={})",
        m[0],
        m[m.len() - 1]
    );
    let _ = fs::remove_dir_all(&dir);
}

// =============================================================================================
// AC-4: keys (oracle: a naive model of the Input table and cursor.md; eol.counts).
// =============================================================================================

/// The naive model: text and selection, as the Input table and `docs/protocol/cursor.md`
/// define them.
#[derive(Clone)]
struct Model {
    t: Vec<char>,
    a: usize,
    h: usize,
}

fn word(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

impl Model {
    fn snap(&self, p: usize) -> usize {
        if p > 0 && p < self.t.len() && self.t[p - 1] == '\r' && self.t[p] == '\n' {
            p - 1
        } else {
            p
        }
    }

    /// One primitive edit `[s, e)` → `ins`, the selection mapped by the mapping rule.
    fn edit(&mut self, s: usize, e: usize, ins: &str) {
        let ins = chars(ins);
        let n = ins.len();
        let map = |p: usize| {
            if p < s {
                p
            } else if p > e {
                p + n - (e - s)
            } else if p == s && e > s {
                s
            } else {
                s + n
            }
        };
        let (a, h) = (map(self.a), map(self.h));
        self.t.splice(s..e, ins);
        self.a = self.snap(a);
        self.h = self.snap(h);
    }

    fn sorted(&self) -> (usize, usize) {
        (self.a.min(self.h), self.a.max(self.h))
    }

    fn typed(&mut self, text: &str) {
        if self.a == self.h {
            self.edit(self.h, self.h, text);
        } else {
            let (s, e) = self.sorted();
            self.edit(s, e, text);
            let c = s + text.chars().count();
            assert_eq!(
                self.snap(c),
                c,
                "model: cursor.set to a non-boundary (a test bug)"
            );
            self.a = c;
            self.h = c;
        }
    }

    fn line_ending(&self) -> &'static str {
        let (lf, crlf, cr) = self.eol_counts();
        if lf + crlf + cr == 0 {
            return "\n";
        }
        if lf >= crlf && lf >= cr {
            "\n"
        } else if crlf >= cr {
            "\r\n"
        } else {
            "\r"
        }
    }

    fn eol_counts(&self) -> (usize, usize, usize) {
        let (mut lf, mut crlf, mut cr) = (0, 0, 0);
        let mut i = 0;
        while i < self.t.len() {
            match self.t[i] {
                '\r' if self.t.get(i + 1) == Some(&'\n') => {
                    crlf += 1;
                    i += 1;
                }
                '\r' => cr += 1,
                '\n' => lf += 1,
                _ => {}
            }
            i += 1;
        }
        (lf, crlf, cr)
    }

    fn char_fwd(&self, h: usize) -> usize {
        if h >= self.t.len() {
            h
        } else if self.t[h] == '\r' && self.t.get(h + 1) == Some(&'\n') {
            h + 2
        } else {
            h + 1
        }
    }

    fn char_back(&self, h: usize) -> usize {
        if h == 0 {
            0
        } else if h >= 2 && self.t[h - 2] == '\r' && self.t[h - 1] == '\n' {
            h - 2
        } else {
            h - 1
        }
    }

    fn word_fwd(&self, h: usize) -> usize {
        let mut i = h;
        while i < self.t.len() && !word(self.t[i]) {
            i += 1;
        }
        while i < self.t.len() && word(self.t[i]) {
            i += 1;
        }
        i
    }

    fn word_back(&self, h: usize) -> usize {
        let mut i = h;
        while i > 0 && !word(self.t[i - 1]) {
            i -= 1;
        }
        while i > 0 && word(self.t[i - 1]) {
            i -= 1;
        }
        i
    }

    fn line_index(&self, p: usize) -> (Vec<Line>, usize) {
        let lines = lines_of(&self.t);
        let i = lines.iter().rposition(|l| l.start <= p).expect("a line");
        (lines, i)
    }

    fn line_move(&self, h: usize, down: bool) -> usize {
        let (lines, i) = self.line_index(h);
        let col = h - lines[i].start;
        if down {
            match lines.get(i + 1) {
                Some(l) => l.start + col.min(l.text.len()),
                None => self.t.len(),
            }
        } else if i == 0 {
            0
        } else {
            let l = &lines[i - 1];
            l.start + col.min(l.text.len())
        }
    }

    fn mv(&mut self, to: usize, extend: bool) {
        self.h = to;
        if !extend {
            self.a = to;
        }
    }

    /// Whether `Enter` may be pressed (AC-4): no space or tab before the caret on its line,
    /// and the inserted line ending does not join a `\r` and a `\n` around it into a CRLF.
    fn enter_allowed(&self) -> bool {
        let (s, e) = self.sorted();
        let le = self.line_ending();
        let joins = (le == "\n" && s > 0 && self.t[s - 1] == '\r')
            || (le == "\r" && e < self.t.len() && self.t[e] == '\n');
        let blank_before = |p: usize| {
            let (lines, i) = self.line_index(p);
            self.t[lines[i].start..p].iter().any(|c| blank(*c))
        };
        !joins && !blank_before(self.h) && !blank_before(s)
    }

    fn key(&mut self, name: &str, ctrl: bool, shift: bool) {
        match name {
            "Enter" => {
                let le = self.line_ending();
                self.typed(le);
            }
            "Backspace" | "Delete" => {
                if self.a != self.h {
                    let (s, e) = self.sorted();
                    self.edit(s, e, "");
                } else if name == "Backspace" && self.h > 0 {
                    let p = self.char_back(self.h);
                    self.edit(p, self.h, "");
                } else if name == "Delete" && self.h < self.t.len() {
                    let n = self.char_fwd(self.h);
                    self.edit(self.h, n, "");
                }
            }
            "ArrowLeft" => {
                let to = if ctrl {
                    self.word_back(self.h)
                } else {
                    self.char_back(self.h)
                };
                self.mv(to, shift);
            }
            "ArrowRight" => {
                let to = if ctrl {
                    self.word_fwd(self.h)
                } else {
                    self.char_fwd(self.h)
                };
                self.mv(to, shift);
            }
            "ArrowUp" | "ArrowDown" => {
                let to = self.line_move(self.h, name == "ArrowDown");
                self.mv(to, shift);
            }
            "Home" | "End" => {
                let (lines, i) = self.line_index(self.h);
                let to = if name == "Home" {
                    lines[i].start
                } else {
                    lines[i].end()
                };
                self.mv(to, shift);
            }
            other => panic!("model: key {other}"),
        }
    }
}

/// One step of an AC-4 session: the script line and the model after it.
fn ac4_session(path: &str, initial: &str, seed: u64, n: usize) -> (Vec<String>, Vec<Model>) {
    let mut rng = Rng(seed);
    let mut m = Model {
        t: chars(initial),
        a: 0,
        h: 0,
    };
    let mut script = vec![open("o", path), frames("f", 1), focus_editor("focus")];
    let mut models = Vec::new();
    let texts = ["a", "Zé", " ", "λ_", "ж;", "(x)", "é é", "_9"];
    for k in 0..n {
        let id = format!("k{k}");
        let line = loop {
            match rng.below(12) {
                0 | 1 => {
                    let t = texts[rng.below(texts.len())];
                    m.typed(t);
                    break type_text(&id, t);
                }
                2 => {
                    if m.enter_allowed() {
                        m.key("Enter", false, false);
                        break key(&id, "Enter", &[]);
                    }
                }
                3 => {
                    m.key("Backspace", false, false);
                    break key(&id, "Backspace", &[]);
                }
                4 => {
                    m.key("Delete", false, false);
                    break key(&id, "Delete", &[]);
                }
                5..=7 => {
                    let name = if rng.flag() {
                        "ArrowLeft"
                    } else {
                        "ArrowRight"
                    };
                    let (ctrl, shift) = (rng.flag(), rng.flag());
                    m.key(name, ctrl, shift);
                    let mods: Vec<&str> = [("ctrl", ctrl), ("shift", shift)]
                        .iter()
                        .filter(|(_, on)| *on)
                        .map(|(n, _)| *n)
                        .collect();
                    break key(&id, name, &mods);
                }
                8 | 9 => {
                    let name = if rng.flag() { "ArrowUp" } else { "ArrowDown" };
                    let shift = rng.flag();
                    m.key(name, false, shift);
                    break key(&id, name, if shift { &["shift"] } else { &[] });
                }
                _ => {
                    let name = if rng.flag() { "Home" } else { "End" };
                    let shift = rng.flag();
                    m.key(name, false, shift);
                    break key(&id, name, if shift { &["shift"] } else { &[] });
                }
            }
        };
        script.push(line);
        script.push(snapshot(&format!("s{k}")));
        script.push(ctl(&format!("t{k}"), "doc.text", json!({"doc": 1})));
        script.push(ctl(&format!("c{k}"), "cursor.get", json!({"doc": 1})));
        models.push(m.clone());
    }
    script.push(ctl("eol", "eol.counts", json!({"doc": 1})));
    script.extend(end_reads(&[1]));
    (script, models)
}

/// The documents of AC-4: LF, CRLF, CR-only, no break, mixed.
const AC4_DOCS: [(&str, &str); 5] = [
    (
        "lf.txt",
        "fn main() {\nlet x = 1;\n  indented é\n}\nlast λ line\n",
    ),
    (
        "crlf.txt",
        "alpha beta\r\ngamma_delta\r\n  eps λ\r\nzeta\r\n",
    ),
    ("cr.txt", "one two\rthree\rfour ж\r"),
    ("nobreak.txt", "no break here é"),
    ("mixed.txt", "a\r\nb\nc\rd\r\n\re f\n"),
];

const AC4_SEEDS: [u64; 2] = [0x0056_0401, 0x0056_0402];

fn ac4_root(dir: &Path) -> PathBuf {
    let root = dir.join("proj");
    for (p, t) in AC4_DOCS {
        write(&root.join(p), t.as_bytes());
    }
    root
}

#[test]
fn ac4_keys_follow_the_input_table() {
    let dir = case_dir("ac4");
    ac4_root(&dir);
    for (p, initial) in AC4_DOCS {
        for seed in AC4_SEEDS {
            let name = format!("{}-{seed:x}", p.replace('.', "-"));
            let (script, models) = ac4_session(p, initial, seed, 45);
            let s = run(&dir, &name, &script, &root_args());
            s.act("focus");
            for (k, m) in models.iter().enumerate() {
                let what = format!("{name} step {k} ({})", s.script[3 + 4 * k]);
                s.act(&format!("k{k}"));
                let ed = editor(s.snap(&format!("s{k}")));
                let text = s.text(&format!("t{k}"));
                let sel = s.sel(&format!("c{k}"));
                assert_eq!(editor_value(ed), text, "{what}: Editor value vs doc.text");
                assert_eq!(
                    editor_sel(ed),
                    sel,
                    "{what}: Editor selection vs cursor.get"
                );
                assert_eq!(
                    chars(&text),
                    m.t,
                    "{what}: doc.text vs the model: {:?}",
                    m.t.iter().collect::<String>()
                );
                assert_eq!(sel, (m.a, m.h), "{what}: selection vs the model");
            }
            let last = models.last().expect("steps");
            let eol = s.ok("eol");
            let (lf, crlf, cr) = last.eol_counts();
            assert_eq!(
                (
                    as_usize(&eol["lf"]),
                    as_usize(&eol["crlf"]),
                    as_usize(&eol["cr"])
                ),
                (lf, crlf, cr),
                "{name}: eol.counts vs the model"
            );
            if p == "lf.txt" || p == "crlf.txt" {
                let kinds = [lf, crlf, cr].iter().filter(|n| **n > 0).count();
                assert!(
                    kinds <= 1,
                    "{name}: a single-kind document became mixed: {eol}"
                );
                assert_eq!(eol["mixed_line_endings"], json!(false), "{name}");
            }
        }
    }
    let _ = fs::remove_dir_all(&dir);
}

// =============================================================================================
// AC-5: scrolling and the caret in view (oracle: doc.line, cursor.get).
// =============================================================================================

#[test]
fn ac5_scroll_steps_move_the_view_only() {
    let dir = case_dir("ac5-scroll");
    let root = dir.join("proj");
    let text = lines300();
    write(&root.join("l.txt"), text.as_bytes());
    let lines = lines_of(&chars(&text));
    let mut firsts = Vec::new();
    for steps in [1, 2, 5] {
        let script = vec![
            open("o", "l.txt"),
            ctl(
                "set",
                "cursor.set",
                json!({"doc": 1, "anchor": 3, "head": 5}),
            ),
            frames("f", 1),
            snapshot("s0"),
            ctl("t0", "doc.text", json!({"doc": 1})),
            ctl("c0", "cursor.get", json!({"doc": 1})),
            scroll_editor("sc", "down", steps),
            snapshot("s1"),
            ctl("t1", "doc.text", json!({"doc": 1})),
            ctl("c1", "cursor.get", json!({"doc": 1})),
        ];
        let s = run(&dir, &format!("steps{steps}"), &script, &root_args());
        let what = format!("{steps} steps");
        let m0 = check_rows_from_line0(&row_texts(editor(s.snap("s0"))), &lines, true, &what);
        assert_eq!(m0, (0..m0.len()).collect::<Vec<_>>(), "{what}: rows 0..k");
        s.act("sc");
        let ed = editor(s.snap("s1"));
        let m1 = map_rows(&row_texts(ed), &lines, true, &what);
        assert!(
            m1[0] > 0,
            "{what}: the first row is still line 0 after scrolling down"
        );
        assert_eq!(
            m1,
            (m1[0]..m1[0] + m1.len()).collect::<Vec<_>>(),
            "{what}: rows are consecutive lines"
        );
        assert_eq!(
            s.text("t1"),
            s.text("t0"),
            "{what}: the scroll changed the text"
        );
        assert_eq!(
            s.sel("c1"),
            s.sel("c0"),
            "{what}: the scroll changed the selection"
        );
        assert_eq!(editor_sel(ed), s.sel("c1"), "{what}: Editor selection");
        assert_eq!(editor_value(ed), s.text("t1"), "{what}: Editor value");
        firsts.push(m1[0]);
    }
    assert!(
        firsts[0] <= firsts[1] && firsts[1] <= firsts[2],
        "more steps scroll at least as far as fewer: first rows {firsts:?} for 1, 2, 5 steps"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn ac5_caret_line_stays_in_view() {
    let dir = case_dir("ac5-caret");
    let root = dir.join("proj");
    let text = lines300();
    write(&root.join("l.txt"), text.as_bytes());
    let lines = lines_of(&chars(&text));
    let mut script = vec![open("o", "l.txt"), frames("f", 1), focus_editor("focus")];
    let checks = [50, 120, 200];
    for k in 1..=200 {
        script.push(key(&format!("down{k}"), "ArrowDown", &[]));
        if checks.contains(&k) {
            script.push(snapshot(&format!("s{k}")));
            script.push(ctl(&format!("c{k}"), "cursor.get", json!({"doc": 1})));
        }
    }
    let line250 = lines[250].start + 2;
    script.extend([
        ctl("l250", "doc.line", json!({"doc": 1, "line": 250})),
        ctl("set", "cursor.set", json!({"doc": 1, "anchor": line250})),
        frames("f250", 1),
        snapshot("s250"),
        ctl("c250", "cursor.get", json!({"doc": 1})),
        ctl("set0", "cursor.set", json!({"doc": 1, "anchor": 0})),
        frames("f0", 1),
        snapshot("s0"),
    ]);
    let s = run(&dir, "s", &script, &root_args());
    s.act("focus");
    for k in checks {
        s.act(&format!("down{k}"));
        let cur = s.ok(&format!("c{k}"));
        assert_eq!(as_usize(&cur["line"]), k, "{k} ArrowDowns from line 0");
        let m = map_rows(
            &row_texts(editor(s.snap(&format!("s{k}")))),
            &lines,
            true,
            "keys",
        );
        assert!(
            m.contains(&k),
            "after {k} ArrowDowns line {k} (the caret's) is not among the rows {}..={}",
            m[0],
            m[m.len() - 1]
        );
    }
    assert_eq!(
        as_usize(&s.ok("l250")["start"]) + 2,
        line250,
        "doc.line 250"
    );
    s.ok("set");
    for (snap, cur, line) in [("s250", "c250", 250), ("s0", "", 0)] {
        if !cur.is_empty() {
            assert_eq!(as_usize(&s.ok(cur)["line"]), line);
        }
        let m = map_rows(&row_texts(editor(s.snap(snap))), &lines, true, snap);
        assert!(
            m.contains(&line),
            "a control cursor.set to line {line} and one frame: line {line} is not among the \
             rows {}..={}",
            m[0],
            m[m.len() - 1]
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

// =============================================================================================
// AC-6: undo, redo, save (oracle: a second session from the same log; file bytes).
// =============================================================================================

/// The edits and keys of the AC-6 session, each followed by a checkpoint: a snapshot, the
/// script's `doc.text` and `doc.info`, and a marker that finds them in the log.
fn ac6_script() -> (Vec<String>, Vec<&'static str>) {
    let acts: Vec<(&str, String)> = vec![
        ("type", type_text("a1", "abc")),
        ("right", key("a2", "ArrowRight", &[])),
        ("typeX", type_text("a3", "X")),
        ("enter", key("a4", "Enter", &[])),
        ("undo1", key("a5", "Z", &["ctrl"])),
        ("undo2", key("a6", "Z", &["ctrl"])),
        ("redo-shift", key("a7", "Z", &["ctrl", "shift"])),
        ("redo-y", key("a8", "Y", &["ctrl"])),
        ("undo3", key("a9", "Z", &["ctrl"])),
        ("undo4", key("a10", "Z", &["ctrl"])),
        ("undo5", key("a11", "Z", &["ctrl"])),
        ("undo-none", key("a12", "Z", &["ctrl"])),
        ("redo1", key("a13", "Y", &["ctrl"])),
        ("redo2", key("a14", "Z", &["ctrl", "shift"])),
        ("save", key("a15", "S", &["ctrl"])),
        ("after-save", type_text("a16", "more")),
    ];
    let mut script = vec![open("o", "src/a.rs"), frames("f", 1), focus_editor("focus")];
    let mut names = Vec::new();
    for (i, (name, line)) in acts.into_iter().enumerate() {
        script.push(line);
        script.push(snapshot(&format!("s{i}")));
        script.push(ctl(&format!("t{i}"), "doc.text", json!({"doc": 1})));
        script.push(ctl(&format!("i{i}"), "doc.info", json!({"doc": 1})));
        script.push(marker(&format!("m{i}"), i as u64));
        names.push(name);
    }
    script.extend(end_reads(&[1]));
    (script, names)
}

fn ac6_root(dir: &Path) -> PathBuf {
    let root = dir.join("proj");
    write(&root.join("src/a.rs"), A_RS.as_bytes());
    write(&root.join("other.txt"), b"other\n");
    root
}

#[test]
fn ac6_undo_redo_save_agree_with_a_second_session() {
    let dir = case_dir("ac6");
    let root = ac6_root(&dir);
    let initial = dir.join("initial");
    copy_dir(&root, &initial);
    let (script, names) = ac6_script();
    let s = run(&dir, "s", &script, &root_args());
    s.act("focus");
    // The second session: the log replayed on a copy of the initial files.
    let copy = dir.join("second");
    copy_dir(&initial, &copy);
    let log = s.log_entries();
    let (_, replayed) = replay(&copy, &log);
    let mut saved_text = None;
    for (i, name) in names.iter().enumerate() {
        let what = format!("after {name}");
        s.act(&format!("a{}", i + 1));
        let root_node = s.snap(&format!("s{i}"));
        let text = s.text(&format!("t{i}"));
        let info = s.ok(&format!("i{i}"));
        let dirty = info["dirty"].as_bool().expect("dirty");
        assert_eq!(
            editor_value(editor(root_node)),
            text,
            "{what}: Editor value vs doc.text"
        );
        check_title(root_node, "src/a.rs", dirty, &what);
        // Where the script's reads are in the log: right before the marker.
        let m = log
            .iter()
            .position(|e| {
                e["command"] == json!("project.tree")
                    && e["params"]["limit"] == json!(900_000 + i as u64)
            })
            .unwrap_or_else(|| panic!("{what}: the marker is not in the log"));
        assert_eq!(
            log[m - 2]["command"],
            json!("doc.text"),
            "{what}: log order"
        );
        assert_eq!(
            log[m - 1]["command"],
            json!("doc.info"),
            "{what}: log order"
        );
        assert_eq!(
            replayed[m - 2]["result"]["text"],
            json!(text),
            "{what}: doc.text vs the second session"
        );
        assert_eq!(
            replayed[m - 1]["result"],
            *info,
            "{what}: doc.info vs the second session"
        );
        if *name == "save" {
            assert!(!dirty, "{what}: ctrl+S leaves the document clean");
            saved_text = Some(text.clone());
        }
    }
    // The undo/redo keys did something: the text went back to the file's and forward again.
    assert_eq!(
        s.text("t11"),
        A_RS,
        "four undos and one more restore the file's text"
    );
    assert_eq!(s.ok("i11")["can_redo"], json!(true), "undo: can_redo");
    assert_ne!(s.text("t4"), s.text("t3"), "ctrl+Z changed the text");
    assert_ne!(s.text("t6"), s.text("t5"), "ctrl+shift+Z changed the text");
    assert_ne!(s.text("t7"), s.text("t6"), "ctrl+Y changed the text");
    // The saved file: its bytes are the text at the save and those the second session wrote.
    let saved = saved_text.expect("the save checkpoint");
    assert_ne!(saved, A_RS, "the save wrote an edited text");
    let on_disk = fs::read(root.join("src/a.rs")).expect("read a.rs");
    assert_eq!(on_disk, saved.as_bytes(), "ctrl+S wrote the file");
    assert_eq!(
        fs::read(copy.join("src/a.rs")).expect("read the second session's a.rs"),
        on_disk,
        "file bytes vs the second session's"
    );
    assert_eq!(tree(&copy), tree(&root), "files vs the second session");
    let _ = fs::remove_dir_all(&dir);
}

/// A save refused because the 100 temporary names exist leaves the file unchanged, `Title`
/// with `modified` and `Message` beginning with the kind a control `doc.save` reports in a
/// second session (`io_error`).
#[test]
fn ac6_refused_save_is_shown() {
    let dir = case_dir("ac6-refused");
    let root = dir.join("proj");
    write(&root.join("src/a.rs"), A_RS.as_bytes());
    for n in 0..100 {
        write(
            &root.join(format!("src/.kiln-tmp-{n}")),
            format!("tmp {n}").as_bytes(),
        );
    }
    // The second session (control requests only) names the kind.
    let second = dir.join("second");
    copy_dir(&root, &second);
    let mut host = Host::on_disk(&second).expect("Host");
    assert_eq!(
        host_req(&mut host, "doc.open", json!({"path": "src/a.rs"}))["ok"],
        json!(true)
    );
    host_req(
        &mut host,
        "doc.insert",
        json!({"doc": 1, "offset": 0, "text": "x"}),
    );
    let refused = host_req(&mut host, "doc.save", json!({"doc": 1}));
    assert_eq!(
        refused["ok"],
        json!(false),
        "precondition: the save is refused: {refused}"
    );
    let kind = refused["error"]["kind"].as_str().expect("kind").to_owned();
    assert_eq!(kind, "io_error", "Pinned (a), (g): the refused save's kind");
    let before = tree(&root);
    assert_eq!(
        before.len(),
        102,
        "precondition: src, a.rs and the 100 temporary files exist"
    );
    let script = vec![
        open("o", "src/a.rs"),
        frames("f", 1),
        focus_editor("focus"),
        type_text("ty", "x"),
        snapshot("s0"),
        key("save", "S", &["ctrl"]),
        snapshot("s1"),
        ctl("info", "doc.info", json!({"doc": 1})),
        ctl("text", "doc.text", json!({"doc": 1})),
    ];
    let s = run(&dir, "s", &script, &root_args());
    for id in ["focus", "ty", "save"] {
        s.act(id);
    }
    check_title(s.snap("s0"), "src/a.rs", true, "before the save");
    let r = s.snap("s1");
    assert_eq!(
        s.ok("info")["dirty"],
        json!(true),
        "the refused save left the document dirty"
    );
    assert_eq!(
        s.text("text"),
        format!("x{A_RS}"),
        "text after the refused save"
    );
    check_title(r, "src/a.rs", true, "after the refused save");
    let m = message(r);
    assert!(
        m.starts_with(&kind),
        "Message `{m}` does not begin with `{kind}`"
    );
    assert_eq!(tree(&root), before, "the refused save changed files");
    assert_eq!(s.out.code, 0, "a UI refusal does not change the exit code");
    let _ = fs::remove_dir_all(&dir);
}

// =============================================================================================
// AC-7: dispatch only (oracle: replay, `mutating` flags, log comparison).
// =============================================================================================

#[test]
fn ac7_replay_of_the_click_session() {
    let dir = case_dir("ac7-click");
    let root = ac3_root(&dir);
    let initial = dir.join("initial");
    copy_dir(&root, &initial);
    let (s, _) = ac3_session(&dir);
    check_dispatch_only(&s, &root, &initial, &[1]);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn ac7_replay_of_the_key_sessions() {
    let dir = case_dir("ac7-keys");
    let root = ac4_root(&dir);
    let initial = dir.join("initial");
    copy_dir(&root, &initial);
    for (p, text) in AC4_DOCS {
        let (script, _) = ac4_session(p, text, AC4_SEEDS[0], 45);
        let s = run(&dir, &p.replace('.', "-"), &script, &root_args());
        check_dispatch_only(&s, &root, &initial, &[1]);
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn ac7_replay_of_the_undo_save_session() {
    let dir = case_dir("ac7-undo");
    let root = ac6_root(&dir);
    let initial = dir.join("initial");
    copy_dir(&root, &initial);
    let (script, _) = ac6_script();
    let s = run(&dir, "s", &script, &root_args());
    check_dispatch_only(&s, &root, &initial, &[1]);
    assert_ne!(
        fs::read(root.join("src/a.rs")).expect("read"),
        A_RS.as_bytes(),
        "precondition: the session saved an edit"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// On an idle editor (after an action, or one frame after the last control request), 600
/// frames add nothing to the log.
#[test]
fn ac7_idle_frames_add_nothing() {
    let dir = case_dir("ac7-idle");
    let root = dir.join("proj");
    write(&root.join("src/a.rs"), A_RS.as_bytes());
    let cases: Vec<(&str, Vec<String>)> = vec![
        (
            "after-action",
            vec![
                open("o", "src/a.rs"),
                frames("f", 1),
                focus_editor("focus"),
                type_text("ty", "y"),
                key("k", "ArrowRight", &["shift"]),
            ],
        ),
        (
            "after-control",
            vec![
                open("o", "src/a.rs"),
                frames("f", 1),
                ctl(
                    "ins",
                    "doc.insert",
                    json!({"doc": 1, "offset": 0, "text": "z"}),
                ),
                frames("f1", 1),
            ],
        ),
    ];
    for (name, prefix) in cases {
        let mut with = prefix.clone();
        with.push(frames("idle", 600));
        with.push(ctl("end", "doc.text", json!({"doc": 1})));
        let mut without = prefix.clone();
        without.push(ctl("end", "doc.text", json!({"doc": 1})));
        write(&root.join("src/a.rs"), A_RS.as_bytes());
        let a = run(&dir, &format!("{name}-with"), &with, &root_args());
        write(&root.join("src/a.rs"), A_RS.as_bytes());
        let b = run(&dir, &format!("{name}-without"), &without, &root_args());
        for (line, resp) in a.script.iter().zip(&a.resp) {
            assert_eq!(resp["ok"], json!(true), "{name}: {line} → {resp}");
        }
        assert_eq!(a.text("end"), b.text("end"), "{name}: doc.text");
        assert_eq!(a.log, b.log, "{name}: 600 idle frames added to the log");
        if name == "after-action" {
            assert!(
                ui_logged(&a)
                    .iter()
                    .any(|e| e["command"] == json!("doc.insert")),
                "{name}: precondition: the typed text was dispatched"
            );
        }
    }
    let _ = fs::remove_dir_all(&dir);
}

// =============================================================================================
// AC-8: frames report whether the UI still asks for a frame (oracle: settled vs snapshots).
// =============================================================================================

#[test]
fn ac8_settled_false_after_a_control_request() {
    let dir = case_dir("ac8");
    let root = dir.join("proj");
    write(&root.join("src/a.rs"), A_RS.as_bytes());
    let mut script = vec![open("o", "src/a.rs")];
    for k in 0..10 {
        script.push(frames(&format!("idle{k}"), 1));
    }
    script.push(snapshot("s-idle"));
    script.push(ctl(
        "ins",
        "doc.insert",
        json!({"doc": 1, "offset": 0, "text": "q"}),
    ));
    for k in 0..=61 {
        script.push(frames(&format!("f{k}"), 1));
        script.push(snapshot(&format!("s{k}")));
    }
    let s = run(&dir, "s", &script, &root_args());
    let settled = |id: &str| {
        let r = s.act(id);
        assert_eq!(r["frames"], json!(1), "`frames` count 1 runs one frame");
        r["settled"].as_bool().expect("settled")
    };
    assert!(
        settled("idle9"),
        "precondition: the UI is idle before the insert"
    );
    s.ok("ins");
    assert!(
        !settled("f0"),
        "the first `frames` count 1 after a control request answers settled: true"
    );
    editor(s.snap("s-idle"));
    let first = (1..=60)
        .find(|k| settled(&format!("f{k}")))
        .unwrap_or_else(|| panic!("not settled within 60 further `frames` requests"));
    assert!(settled(&format!("f{}", first + 1)), "settled stays true");
    assert_eq!(
        s.snap(&format!("s{}", first + 1)),
        s.snap(&format!("s{first}")),
        "a frame after settled: true changed the snapshot"
    );
    assert_eq!(
        editor_value(editor(s.snap(&format!("s{first}")))),
        format!("q{A_RS}"),
        "the insert is shown"
    );
    let _ = fs::remove_dir_all(&dir);
}

// =============================================================================================
// Rulings of review round 1 (D-159): z1 rows are the shown chars, z2 settling, z4 test gaps.
// =============================================================================================

/// Lines whose chars the toolkit shapes into more or fewer glyphs than chars (z1): a combining
/// acute after `e` and after `b`, a zero-width joiner, Arabic-Indic digits, ligature candidates.
const SHAPED: &str = "e\u{301}b first\nab\u{301}cd\nx\u{200d}yz\n\u{663}\u{663}\noffice ffi fl\n";

/// z1 (a), y5: for every node that shows text, its one painted row is exactly that text; the
/// editor's rows are exactly its lines (none of these is long enough to be cut).
#[test]
fn z1_rows_equal_the_text_each_node_shows() {
    let dir = case_dir("z1-rows");
    let root = dir.join("proj");
    write(&root.join("src/a.rs"), A_RS.as_bytes());
    write(&root.join("z.txt"), SHAPED.as_bytes());
    let script = vec![
        open("o1", "src/a.rs"),
        frames("f1", 1),
        focus_editor("fe"),
        type_text("ty", "w"),
        snapshot("s1"),
        open("o2", "z.txt"),
        frames("f2", 1),
        ui("fp", "focus", json!({"target": {"label": "Path"}})),
        type_text("tp", "src/missing.rs"),
        ui("c", "click", json!({"target": {"label": "Open"}})),
        snapshot("s2"),
        ctl("t2", "doc.text", json!({"doc": 2})),
    ];
    let s = run(&dir, "s", &script, &root_args());
    for id in ["fe", "ty", "fp", "tp", "c"] {
        s.act(id);
    }
    check_title(s.snap("s1"), "src/a.rs", true, "after an edit");
    assert!(
        message(s.snap("s2")).starts_with("not_found"),
        "precondition: Message shows the refused open"
    );
    for snap in ["s1", "s2"] {
        let root_node = s.snap(snap);
        let mut seen = Vec::new();
        for n in all_nodes(root_node) {
            if label(n) == Some("Editor") || n.get("rows").is_none() {
                continue;
            }
            let shown = n["value"]
                .as_str()
                .or_else(|| label(n))
                .unwrap_or_else(|| panic!("{snap}: a node without text has rows: {}", short(n)));
            assert_eq!(
                row_texts(n),
                [shown.to_owned()],
                "{snap}: the rows of a {} node are not the text it shows",
                n["role"]
            );
            seen.push(label(n).unwrap_or(shown).to_owned());
        }
        for want in ["Title", "Path", "Open"] {
            assert!(
                seen.iter().any(|l| l == want),
                "{snap}: {want} was not checked: {seen:?}"
            );
        }
    }
    let text = s.text("t2");
    assert_eq!(text, SHAPED, "doc.text");
    let ed = editor(s.snap("s2"));
    assert_eq!(editor_value(ed), text, "Editor value");
    check_rows_from_line0(
        &row_texts(ed),
        &lines_of(&chars(&text)),
        true,
        "shaped lines",
    );
    let _ = fs::remove_dir_all(&dir);
}

/// z1 (b): a click with `at` on lines with combining marks, a joiner and shaped digits puts the
/// caret on the char the column names (oracle: `cursor.get`; typing there, the naive model).
#[test]
fn z1_click_at_counts_chars_on_shaped_lines() {
    let dir = case_dir("z1-click");
    let root = dir.join("proj");
    write(&root.join("z.txt"), SHAPED.as_bytes());
    let lines = lines_of(&chars(SHAPED));
    // (row, col): the rows of a fresh view are lines 0..k (checked below).
    let clicks = [(1, 3), (1, 1), (0, 3), (2, 2), (3, 1), (1, 5)];
    let mut script = vec![open("o", "z.txt"), frames("f", 1), snapshot("s0")];
    for (k, (r, c)) in clicks.iter().enumerate() {
        script.push(click_at(&format!("c{k}"), *r, *c, false));
        script.push(ctl(&format!("g{k}"), "cursor.get", json!({"doc": 1})));
        script.push(snapshot(&format!("s{}", k + 1)));
    }
    script.push(type_text("ty", "Q"));
    script.push(ctl("t", "doc.text", json!({"doc": 1})));
    let s = run(&dir, "s", &script, &root_args());
    let m = check_rows_from_line0(&row_texts(editor(s.snap("s0"))), &lines, true, "fresh view");
    assert!(
        m.len() >= 4 && m[..4] == [0, 1, 2, 3],
        "precondition: rows 0..4 are lines 0..4"
    );
    for (k, (r, c)) in clicks.iter().enumerate() {
        s.act(&format!("c{k}"));
        let want = lines[*r].start + c;
        let sel = s.sel(&format!("g{k}"));
        assert_eq!(
            sel,
            (want, want),
            "click at row {r} col {c}: caret at char {want}"
        );
        assert_eq!(
            editor_sel(editor(s.snap(&format!("s{}", k + 1)))),
            sel,
            "Editor selection"
        );
    }
    s.act("ty");
    let (r, c) = clicks[clicks.len() - 1];
    let mut model = chars(SHAPED);
    model.insert(lines[r].start + c, 'Q');
    assert_eq!(chars(&s.text("t")), model, "Q typed at the clicked char");
    let _ = fs::remove_dir_all(&dir);
}

/// z2: in a document longer than the view, `Enter` with the caret at the very end, and an undo
/// and a redo right after it, each settle within 60 frames.
#[test]
fn z2_enter_undo_redo_at_the_end_of_a_long_document_settle() {
    let dir = case_dir("z2");
    let root = dir.join("proj");
    let text = lines300();
    write(&root.join("l.txt"), text.as_bytes());
    let end = text.chars().count();
    let script = vec![
        open("o", "l.txt"),
        ctl("set", "cursor.set", json!({"doc": 1, "anchor": end})),
        frames("f", 1),
        focus_editor("focus"),
        key("enter", "Enter", &[]),
        ctl("t1", "doc.text", json!({"doc": 1})),
        key("undo", "Z", &["ctrl"]),
        ctl("t2", "doc.text", json!({"doc": 1})),
        key("redo", "Y", &["ctrl"]),
        ctl("t3", "doc.text", json!({"doc": 1})),
        snapshot("s"),
    ];
    let s = run(&dir, "s", &script, &root_args());
    s.act("focus");
    for id in ["enter", "undo", "redo"] {
        assert_eq!(
            s.act(id)["settled"],
            json!(true),
            "`{id}` at the end of a long document did not settle within 60 frames"
        );
    }
    assert_eq!(
        s.text("t1"),
        format!("{text}\n"),
        "Enter inserted a line break"
    );
    assert_eq!(s.text("t2"), text, "undo");
    assert_eq!(s.text("t3"), format!("{text}\n"), "redo");
    assert_eq!(editor_value(editor(s.snap("s"))), s.text("t3"));
    let _ = fs::remove_dir_all(&dir);
}

/// z4 (a): text with chars outside ASCII typed over a selection (either direction) leaves the
/// caret after it, counted in chars (oracle: `cursor.get`, the naive model).
#[test]
fn z4_non_ascii_typed_over_a_selection_caret_counts_chars() {
    let dir = case_dir("z4-type");
    let root = dir.join("proj");
    let initial = "h\u{e9}llo w\u{f6}rld\nsecond\n";
    write(&root.join("u.txt"), initial.as_bytes());
    let typed = "\u{3bb}\u{436}\u{e9}\u{1f600}";
    let mut script = vec![open("o", "u.txt"), frames("f", 1), focus_editor("focus")];
    for (k, (a, h)) in [(1, 4), (9, 2)].iter().enumerate() {
        script.push(ctl(
            &format!("set{k}"),
            "cursor.set",
            json!({"doc": 1, "anchor": a, "head": h}),
        ));
        script.push(frames(&format!("f{k}"), 1));
        script.push(type_text(&format!("ty{k}"), typed));
        script.push(ctl(&format!("t{k}"), "doc.text", json!({"doc": 1})));
        script.push(ctl(&format!("c{k}"), "cursor.get", json!({"doc": 1})));
        script.push(snapshot(&format!("s{k}")));
    }
    let s = run(&dir, "s", &script, &root_args());
    let mut m = Model {
        t: chars(initial),
        a: 0,
        h: 0,
    };
    for (k, (a, h)) in [(1, 4), (9, 2)].iter().enumerate() {
        s.act(&format!("ty{k}"));
        m.a = *a;
        m.h = *h;
        m.typed(typed);
        assert_eq!(
            chars(&s.text(&format!("t{k}"))),
            m.t,
            "text after typing over {a}..{h}"
        );
        let sel = s.sel(&format!("c{k}"));
        assert_eq!(
            sel,
            (m.a, m.h),
            "caret after typing over {a}..{h}, in chars"
        );
        assert_eq!(
            editor_sel(editor(s.snap(&format!("s{k}")))),
            sel,
            "Editor selection"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// z4 (b): a successful save through the UI clears `Message` (made non-empty by a refused open).
#[test]
fn z4_successful_ui_save_clears_message() {
    let dir = case_dir("z4-save");
    let root = dir.join("proj");
    write(&root.join("src/a.rs"), A_RS.as_bytes());
    let script = vec![
        open("o", "src/a.rs"),
        frames("f", 1),
        ui("fp", "focus", json!({"target": {"label": "Path"}})),
        type_text("tp", "src/missing.rs"),
        ui("c", "click", json!({"target": {"label": "Open"}})),
        snapshot("s0"),
        focus_editor("fe"),
        type_text("ty", "x"),
        key("save", "S", &["ctrl"]),
        snapshot("s1"),
        ctl("i", "doc.info", json!({"doc": 1})),
    ];
    let s = run(&dir, "s", &script, &root_args());
    for id in ["fp", "tp", "c", "fe", "ty", "save"] {
        s.act(id);
    }
    assert!(
        message(s.snap("s0")).starts_with("not_found"),
        "precondition: Message shows the refused open"
    );
    assert_eq!(s.ok("i")["dirty"], json!(false), "the save succeeded");
    assert_eq!(
        fs::read(root.join("src/a.rs")).expect("read"),
        format!("x{A_RS}").as_bytes()
    );
    assert_eq!(
        message(s.snap("s1")),
        "",
        "a successful UI save clears Message"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// z4 (c): a path that only contains `.rs` is not a Rust document: painted all `plain`,
/// although `syntax.highlight` gives its text other kinds.
#[test]
fn z4_a_path_that_only_contains_rs_is_plain() {
    let dir = case_dir("z4-plain");
    let root = dir.join("proj");
    for p in ["x.rs.txt", "d.rs/a.txt", "a.rsx"] {
        write(&root.join(p), A_RS.as_bytes());
    }
    let mut script = Vec::new();
    for (i, p) in ["x.rs.txt", "d.rs/a.txt", "a.rsx"].iter().enumerate() {
        let d = i + 1;
        script.push(open(&format!("o{d}"), p));
        script.push(frames(&format!("f{d}"), 1));
        script.push(snapshot(&format!("s{d}")));
        script.push(ctl(&format!("h{d}"), "syntax.highlight", json!({"doc": d})));
    }
    let s = run(&dir, "s", &script, &root_args());
    let lines = lines_of(&chars(A_RS));
    for (i, p) in ["x.rs.txt", "d.rs/a.txt", "a.rsx"].iter().enumerate() {
        let d = i + 1;
        let k = kinds(s.ok(&format!("h{d}")), A_RS.chars().count());
        assert!(
            k.iter().any(|x| x != "plain"),
            "precondition: {p}'s text has non-plain kinds"
        );
        let root_node = s.snap(&format!("s{d}"));
        check_title(root_node, p, false, p);
        check_colours(editor(root_node), &lines, &|_| "plain".to_owned(), p);
    }
    let _ = fs::remove_dir_all(&dir);
}
