//! The editor view through the built `kiln` binary (implementer's tests of T-056), against
//! temporary roots under `CARGO_TARGET_TMPDIR`. Nodes are found by role and label (`Title`,
//! `Editor`, `Message`); rows are matched to lines by their text; no test pins a whole snapshot,
//! a message text, the usage text, a layout, the command list or which reads the view sends.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

#[cfg(test)]
/// Every spawned `kiln` is killed and the test fails after this long (D-028 (8)).
const TIMEOUT: Duration = Duration::from_secs(300);

#[cfg(test)]
/// T-056 Pinned (g).
const A_RS: &str = "fn main() {\n    let x = 1;\n}\n";

#[cfg(test)]
/// The colour table of T-056 (Q-8 A).
const COLOURS: &[(&str, &str)] = &[
    ("plain", "#dcdcdc"),
    ("keyword", "#569cd6"),
    ("type", "#4ec9b0"),
    ("function", "#dcdcaa"),
    ("string", "#ce9178"),
    ("number", "#b5cea8"),
    ("comment", "#6a9955"),
];

#[cfg(test)]
fn colour_of(kind: &str) -> &'static str {
    COLOURS.iter().find(|(k, _)| *k == kind).unwrap().1
}

#[cfg(test)]
/// A fresh, empty directory unique to `name`.
fn fresh(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("kiln-editor-t056")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("root")).unwrap();
    dir
}

#[cfg(test)]
/// Write `files` (path, bytes) below `root`.
fn write_files(root: &Path, files: &[(&str, &[u8])]) {
    for (path, bytes) in files {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
}

#[cfg(test)]
fn wait_or_kill(child: &mut Child) {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if child.try_wait().unwrap().is_some() {
            return;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("kiln did not finish within {TIMEOUT:?}");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(test)]
/// What one `kiln script` run produced.
struct Run {
    code: Option<i32>,
    responses: Vec<Value>,
    log: String,
}

#[cfg(test)]
impl Run {
    /// The response with this `id`.
    fn by_id(&self, id: &str) -> &Value {
        self.responses
            .iter()
            .find(|r| r["id"] == id)
            .unwrap_or_else(|| panic!("no response {id}"))
    }

    /// The root node of the snapshot `id`.
    fn root(&self, id: &str) -> &Value {
        let r = self.by_id(id);
        assert_eq!(r["ok"], true, "{r}");
        &r["result"]["root"]
    }

    /// The `result` of the control request `id`.
    fn result(&self, id: &str) -> &Value {
        let r = self.by_id(id);
        assert_eq!(r["ok"], true, "{r}");
        &r["result"]
    }
}

#[cfg(test)]
/// `kiln script s.jsonl --root root --log l.jsonl [extra…]` in `dir`.
fn run(dir: &Path, lines: &[Value], extra: &[&str]) -> Run {
    let text: String = lines.iter().map(|l| format!("{l}\n")).collect();
    fs::write(dir.join("s.jsonl"), text).unwrap();
    let out_path = dir.join("stdout.txt");
    let mut child = Command::new(env!("CARGO_BIN_EXE_kiln"))
        .args(["script", "s.jsonl", "--root", "root", "--log", "l.jsonl"])
        .args(extra)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(fs::File::create(&out_path).unwrap())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    wait_or_kill(&mut child);
    let code = child.wait().unwrap().code();
    let responses = fs::read_to_string(&out_path)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let log = fs::read_to_string(dir.join("l.jsonl")).unwrap_or_default();
    Run {
        code,
        responses,
        log,
    }
}

#[cfg(test)]
fn ctl(id: &str, command: &str, params: Value) -> Value {
    json!({"id": id, "command": command, "params": params})
}

#[cfg(test)]
fn snapshot(id: &str) -> Value {
    json!({"id": id, "ui": "snapshot"})
}

#[cfg(test)]
fn frames(id: &str, count: u64) -> Value {
    json!({"id": id, "ui": "frames", "count": count})
}

#[cfg(test)]
fn key(id: &str, name: &str, modifiers: &[&str]) -> Value {
    json!({"id": id, "ui": "key", "key": name, "modifiers": modifiers})
}

#[cfg(test)]
fn editor_target() -> Value {
    json!({"role": "multiline_text_input", "label": "Editor"})
}

#[cfg(test)]
fn focus_editor(id: &str) -> Value {
    json!({"id": id, "ui": "focus", "target": editor_target()})
}

#[cfg(test)]
fn nodes(root: &Value) -> Vec<&Value> {
    fn walk<'a>(n: &'a Value, out: &mut Vec<&'a Value>) {
        out.push(n);
        for c in n["children"].as_array().into_iter().flatten() {
            walk(c, out);
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out);
    out
}

#[cfg(test)]
fn labelled<'a>(root: &'a Value, label: &str) -> Option<&'a Value> {
    let found: Vec<&Value> = nodes(root)
        .into_iter()
        .filter(|n| n["label"] == label)
        .collect();
    assert!(found.len() <= 1, "two nodes labelled {label}");
    found.first().copied()
}

#[cfg(test)]
fn editor(root: &Value) -> &Value {
    let e = labelled(root, "Editor").expect("an Editor node");
    assert_eq!(e["role"], "multiline_text_input");
    e
}

#[cfg(test)]
fn title(root: &Value) -> String {
    labelled(root, "Title").expect("a Title node")["value"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[cfg(test)]
fn message(root: &Value) -> String {
    labelled(root, "Message").unwrap()["value"]
        .as_str()
        .unwrap_or("")
        .to_owned()
}

#[cfg(test)]
/// Each painted row of `node` as its runs `(text, colour)`.
fn runs(node: &Value) -> Vec<Vec<(String, String)>> {
    node["rows"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|row| {
            row.as_array()
                .unwrap()
                .iter()
                .map(|r| {
                    (
                        r["text"].as_str().unwrap().to_owned(),
                        r["color"].as_str().unwrap().to_owned(),
                    )
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
fn row_texts(node: &Value) -> Vec<String> {
    runs(node)
        .iter()
        .map(|row| row.iter().map(|(t, _)| t.as_str()).collect())
        .collect()
}

#[cfg(test)]
/// `(anchor, head)` of a node's `selection` or of a `cursor.get` result.
fn sel(v: &Value) -> (u64, u64) {
    let s = if v.get("selection").is_some() {
        &v["selection"]
    } else {
        v
    };
    (s["anchor"].as_u64().unwrap(), s["head"].as_u64().unwrap())
}

#[cfg(test)]
/// The lines of `text` (split at `\r\n`, `\r`, `\n`) as (start, content).
fn lines_of(text: &str) -> Vec<(usize, String)> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let (mut start, mut i) = (0, 0);
    while i < chars.len() {
        if chars[i] == '\r' || chars[i] == '\n' {
            out.push((start, chars[start..i].iter().collect()));
            i += if chars[i] == '\r' && chars.get(i + 1) == Some(&'\n') {
                2
            } else {
                1
            };
            start = i;
        } else {
            i += 1;
        }
    }
    out.push((start, chars[start..].iter().collect()));
    out
}

#[cfg(test)]
fn ok_actions(run: &Run) {
    for r in &run.responses {
        assert_eq!(r["ok"], true, "{r}");
        if r["result"].get("settled").is_some() && r["id"] != "settle" {
            assert_eq!(r["result"]["settled"], true, "{r}");
        }
    }
}

#[test]
fn start_paths_open_in_order_and_the_last_successful_is_current() {
    let dir = fresh("start");
    write_files(
        &dir.join("root"),
        &[("a.rs", A_RS.as_bytes()), ("b.rs", b"fn b() {}\n")],
    );
    let lines = [snapshot("s"), ctl("l", "doc.list", json!({}))];
    let r = run(&dir, &lines, &["a.rs", "b.rs", "missing.rs"]);
    assert_eq!(r.code, Some(0));
    assert_eq!(r.responses.len(), 2, "start paths get no response line");
    let root = r.root("s");
    assert!(title(root).starts_with("b.rs"), "{}", title(root));
    assert!(message(root).starts_with("not_found"), "{}", message(root));
    assert_eq!(editor(root)["value"], "fn b() {}\n");
    let opens: Vec<Value> = r
        .log
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .filter(|l| l["command"] == "doc.open")
        .map(|l| l["params"]["path"].clone())
        .collect();
    assert_eq!(opens, [json!("a.rs"), json!("b.rs")]);
    // Options may follow the paths; with --memory every start path is refused.
    let r = run(&dir, &[snapshot("s")], &["a.rs", "--size", "400x300"]);
    assert!(title(r.root("s")).starts_with("a.rs"));
    fs::write(dir.join("s.jsonl"), "{\"id\":\"s\",\"ui\":\"snapshot\"}\n").unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_kiln"))
        .args(["script", "s.jsonl", "--memory", "a.rs"])
        .current_dir(&dir)
        .stdin(Stdio::null())
        .stdout(fs::File::create(dir.join("m.txt")).unwrap())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    wait_or_kill(&mut child);
    assert_eq!(child.wait().unwrap().code(), Some(0));
    let out: Value = serde_json::from_str(&fs::read_to_string(dir.join("m.txt")).unwrap()).unwrap();
    let root = &out["result"]["root"];
    assert!(message(root).starts_with("not_found"));
    assert!(labelled(root, "Editor").is_none());
    assert!(labelled(root, "Title").is_none());
}

#[test]
fn the_current_document_follows_opens_and_closes() {
    let dir = fresh("current");
    write_files(
        &dir.join("root"),
        &[("a.txt", b"a\n"), ("b.txt", b"b\n"), ("c.txt", b"c\n")],
    );
    let lines = [
        snapshot("none"),
        ctl("1", "doc.open", json!({"path": "a.txt"})),
        ctl("2", "doc.open", json!({"path": "b.txt"})),
        ctl("3", "doc.open", json!({"path": "c.txt"})),
        frames("f1", 1),
        snapshot("c"),
        ctl("4", "doc.open", json!({"path": "a.txt"})),
        frames("f2", 1),
        snapshot("a"),
        ctl("5", "doc.close", json!({"doc": 1})),
        frames("f3", 1),
        snapshot("highest"),
        ctl("6", "doc.close", json!({"doc": 3})),
        ctl("7", "doc.close", json!({"doc": 2})),
        frames("f4", 1),
        snapshot("empty"),
    ];
    let r = run(&dir, &lines, &[]);
    assert!(labelled(r.root("none"), "Editor").is_none());
    assert!(title(r.root("c")).starts_with("c.txt"));
    assert_eq!(r.result("4")["already_open"], true);
    assert!(title(r.root("a")).starts_with("a.txt"));
    assert_eq!(editor(r.root("a"))["value"], "a\n");
    assert!(title(r.root("highest")).starts_with("c.txt"));
    assert!(labelled(r.root("empty"), "Editor").is_none());
    assert!(labelled(r.root("empty"), "Title").is_none());
}

#[test]
fn the_document_is_shown_with_the_published_colours() {
    let dir = fresh("shown");
    write_files(&dir.join("root"), &[("src/a.rs", A_RS.as_bytes())]);
    let lines = [
        snapshot("s"),
        ctl("t", "doc.text", json!({"doc": 1})),
        ctl(
            "h",
            "syntax.highlight",
            json!({"doc": 1, "start": 0, "end": 11}),
        ),
    ];
    let r = run(&dir, &lines, &["src/a.rs"]);
    let root = r.root("s");
    let e = editor(root);
    assert_eq!(e["value"], A_RS);
    assert!(!title(root)["src/a.rs".len()..].contains("modified"));
    assert!(e.get("focused").is_none(), "the editor focuses itself");
    assert_eq!(sel(e), (0, 0));
    assert_eq!(row_texts(e), ["fn main() {", "    let x = 1;", "}"]);
    // Pinned (g): the first painted row.
    let first: Vec<(String, String)> =
        [("fn ", "#569cd6"), ("main", "#dcdcaa"), ("() {", "#dcdcdc")]
            .iter()
            .map(|(t, c)| ((*t).to_owned(), (*c).to_owned()))
            .collect();
    assert_eq!(runs(e)[0], first);
}

#[cfg(test)]
/// Check every painted char of `e` against the highlight spans of `text` (`spans` over the
/// whole text) and the table, blanks by the neighbour rule.
fn check_colours(e: &Value, text: &str, spans: &[Value]) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let kind_at = |at: usize| {
        spans
            .iter()
            .find(|s| {
                s["start"].as_u64().unwrap() as usize <= at
                    && at < s["end"].as_u64().unwrap() as usize
            })
            .map_or("plain", |s| s["kind"].as_str().unwrap())
    };
    let lines = lines_of(text);
    let mut checked = 0;
    for row in runs(e) {
        let row_text: String = row.iter().map(|(t, _)| t.as_str()).collect();
        let (start, content) = lines
            .iter()
            .find(|(_, l)| l.starts_with(&row_text))
            .unwrap_or_else(|| panic!("row `{row_text}` is no line's start"));
        assert!(content.starts_with(&row_text));
        let reported: Vec<String> = row
            .iter()
            .flat_map(|(t, c)| t.chars().map(move |_| c.clone()))
            .collect();
        let expected_raw: Vec<&str> = (0..reported.len())
            .map(|i| colour_of(kind_at(start + i)))
            .collect();
        let blank = |i: usize| chars[start + i] == ' ' || chars[start + i] == '\t';
        for i in 0..reported.len() {
            let want = if blank(i) {
                // the char before it in the row, or at the start the first one after it that
                // paints pixels
                if i > 0 {
                    reported[i - 1].as_str()
                } else {
                    (0..reported.len())
                        .find(|j| !blank(*j))
                        .map_or(expected_raw[i], |j| expected_raw[j])
                }
            } else {
                expected_raw[i]
            };
            assert_eq!(reported[i], want, "char {} of `{row_text}`", i);
            checked += 1;
        }
    }
    checked
}

#[test]
fn colours_follow_the_highlight_kinds() {
    let dir = fresh("colours");
    let rust = "//! doc é\r\nuse std::fmt;\r\n\tstruct Λ { s: String }\r\nfn f(x: u8) -> u64 {\r\n    let s = \"жλ\"; // c\r\n  g(1.5, 'c', 0x1f); x as u64\r\n}\r\n";
    let long: String = "// x\n".repeat(1_048_577 / 5) + "//";
    assert_eq!(long.chars().count(), 1_048_577);
    write_files(
        &dir.join("root"),
        &[
            ("a.rs", rust.as_bytes()),
            ("b.txt", b"fn main() { let x = 1; }\n"),
            ("big.rs", long.as_bytes()),
        ],
    );
    let lines = [
        snapshot("rs"),
        ctl("h", "syntax.highlight", json!({"doc": 1})),
        ctl("o2", "doc.open", json!({"path": "b.txt"})),
        frames("f", 1),
        snapshot("txt"),
        ctl("o3", "doc.open", json!({"path": "big.rs"})),
        frames("f2", 1),
        snapshot("big"),
    ];
    let r = run(&dir, &lines, &["a.rs"]);
    let spans = r.result("h")["spans"].as_array().unwrap().clone();
    let checked = check_colours(editor(r.root("rs")), rust, &spans);
    assert_eq!(
        checked,
        rust.chars().filter(|c| *c != '\r' && *c != '\n').count()
    );
    for snap in ["txt", "big"] {
        let e = editor(r.root(snap));
        assert!(!runs(e).is_empty());
        for row in runs(e) {
            for (_, c) in row {
                assert_eq!(c, "#dcdcdc", "{snap}");
            }
        }
    }
}

#[test]
fn clicks_put_the_caret_and_typing_inserts_there() {
    let dir = fresh("click");
    write_files(&dir.join("root"), &[("src/a.rs", A_RS.as_bytes())]);
    let lines = [
        json!({"id": "c", "ui": "click", "target": editor_target(), "at": {"row": 1, "col": 4}}),
        ctl("g1", "cursor.get", json!({"doc": 1})),
        json!({"id": "t", "ui": "type", "text": "y"}),
        ctl("x1", "doc.text", json!({"doc": 1})),
        ctl("g2", "cursor.get", json!({"doc": 1})),
        frames("settle", 1),
        snapshot("s1"),
        key("bs", "Backspace", &[]),
        ctl("x2", "doc.text", json!({"doc": 1})),
        ctl("g3", "cursor.get", json!({"doc": 1})),
        json!({"id": "sc", "ui": "click", "target": editor_target(), "at": {"row": 0, "col": 11}, "modifiers": ["shift"]}),
        ctl("g4", "cursor.get", json!({"doc": 1})),
        snapshot("s2"),
    ];
    let r = run(&dir, &lines, &["src/a.rs"]);
    assert_eq!(sel(r.result("g1")), (16, 16));
    assert_eq!(r.result("x1")["text"], "fn main() {\n    ylet x = 1;\n}\n");
    assert_eq!(sel(r.result("g2")), (17, 17));
    assert_eq!(r.result("g2")["col"], 5);
    let e = editor(r.root("s1"));
    assert_eq!(e["focused"], true);
    assert_eq!(sel(e), (17, 17));
    assert_eq!(e["value"], r.result("x1")["text"]);
    assert_eq!(r.result("x2")["text"], A_RS);
    assert_eq!(sel(r.result("g3")), (16, 16));
    // Just right of the row's last char: after it; shift keeps the anchor.
    assert_eq!(sel(r.result("g4")), (16, 11));
    assert_eq!(sel(editor(r.root("s2"))), (16, 11));
}

#[test]
fn keys_follow_the_input_table() {
    let dir = fresh("keys");
    write_files(
        &dir.join("root"),
        &[
            ("c1.txt", b"a\r\nb"),
            ("c2.txt", b"x\r\ny"),
            ("c3.txt", b"abc"),
            ("c4.txt", b"one\ntwo\nthree"),
        ],
    );
    let lines = [
        // a\r\nb, caret 3: Backspace takes the whole CRLF (Pinned (g)).
        ctl("s1", "cursor.set", json!({"doc": 1, "anchor": 3})),
        focus_editor("fo"),
        key("bs", "Backspace", &[]),
        ctl("x1", "doc.text", json!({"doc": 1})),
        // Delete at the end dispatches nothing; Delete before a CRLF takes it whole.
        key("end", "End", &[]),
        key("del", "Delete", &[]),
        ctl("x1b", "doc.text", json!({"doc": 1})),
        // x\r\ny, caret 1: Enter inserts the document's CRLF.
        ctl("o2", "doc.open", json!({"path": "c2.txt"})),
        ctl("s2", "cursor.set", json!({"doc": 2, "anchor": 1})),
        key("en", "Enter", &[]),
        ctl("x2", "doc.text", json!({"doc": 2})),
        key("dl", "ArrowLeft", &[]),
        key("de", "Delete", &[]),
        ctl("x2b", "doc.text", json!({"doc": 2})),
        // abc (no break): Enter inserts \n.
        ctl("o3", "doc.open", json!({"path": "c3.txt"})),
        key("en3", "Enter", &[]),
        ctl("x3", "doc.text", json!({"doc": 3})),
        // Home / End with and without shift; up and down; words; undo and redo.
        ctl("o4", "doc.open", json!({"path": "c4.txt"})),
        ctl("s4", "cursor.set", json!({"doc": 4, "anchor": 5})),
        key("e4", "End", &["shift"]),
        ctl("g4a", "cursor.get", json!({"doc": 4})),
        key("h4", "Home", &[]),
        ctl("g4b", "cursor.get", json!({"doc": 4})),
        key("dn", "ArrowDown", &["shift"]),
        ctl("g4c", "cursor.get", json!({"doc": 4})),
        key("up", "ArrowUp", &[]),
        ctl("g4d", "cursor.get", json!({"doc": 4})),
        key("wr", "ArrowRight", &["ctrl", "shift"]),
        ctl("g4e", "cursor.get", json!({"doc": 4})),
        json!({"id": "ty", "ui": "type", "text": "Z"}),
        ctl("x4a", "doc.text", json!({"doc": 4})),
        ctl("g4f", "cursor.get", json!({"doc": 4})),
        key("un", "Z", &["ctrl"]),
        ctl("x4b", "doc.text", json!({"doc": 4})),
        key("re", "Z", &["ctrl", "shift"]),
        ctl("x4c", "doc.text", json!({"doc": 4})),
        key("un2", "Z", &["ctrl"]),
        key("re2", "Y", &["ctrl"]),
        ctl("x4d", "doc.text", json!({"doc": 4})),
        frames("settle", 1),
        snapshot("s"),
        ctl("i4", "doc.info", json!({"doc": 4})),
        ctl("g4g", "cursor.get", json!({"doc": 4})),
    ];
    let r = run(&dir, &lines, &["c1.txt"]);
    ok_actions(&r);
    assert_eq!(r.result("x1")["text"], "ab");
    assert_eq!(r.result("x1b")["text"], "ab");
    assert_eq!(r.result("x2")["text"], "x\r\n\r\ny");
    assert_eq!(r.result("x2b")["text"], "x\r\ny");
    assert_eq!(r.result("x3")["text"], "\nabc");
    assert_eq!(sel(r.result("g4a")), (5, 7));
    assert_eq!(sel(r.result("g4b")), (4, 4));
    assert_eq!(sel(r.result("g4c")), (4, 8));
    assert_eq!(sel(r.result("g4d")), (4, 4));
    assert_eq!(sel(r.result("g4e")), (4, 7));
    assert_eq!(r.result("x4a")["text"], "one\nZ\nthree");
    assert_eq!(sel(r.result("g4f")), (5, 5));
    assert_eq!(r.result("x4b")["text"], "one\ntwo\nthree");
    assert_eq!(r.result("x4c")["text"], "one\nZ\nthree");
    assert_eq!(r.result("x4d")["text"], "one\nZ\nthree");
    let root = r.root("s");
    let e = editor(root);
    assert_eq!(e["value"], "one\nZ\nthree");
    assert_eq!(sel(e), sel(r.result("g4g")));
    assert_eq!(row_texts(e), ["one", "Z", "three"]);
    assert_eq!(r.result("i4")["dirty"], true);
    assert!(title(root)["c4.txt".len()..].contains("modified"));
}

#[test]
fn scrolling_keeps_rows_consecutive_and_the_caret_comes_into_view() {
    let dir = fresh("scroll");
    let text: String = (0..300).map(|i| format!("line number {i:03}\n")).collect();
    write_files(&dir.join("root"), &[("long.txt", text.as_bytes())]);
    let line_of = |row: &str| -> usize { row["line number ".len()..].parse().unwrap() };
    let mut lines = vec![
        snapshot("s0"),
        json!({"id": "sc1", "ui": "scroll", "target": editor_target(), "direction": "down", "steps": 1}),
        snapshot("s1"),
        json!({"id": "sc4", "ui": "scroll", "target": editor_target(), "direction": "down", "steps": 3}),
        snapshot("s4"),
        ctl("g0", "cursor.get", json!({"doc": 1})),
        json!({"id": "ck", "ui": "click", "target": editor_target()}),
        ctl("gc", "cursor.get", json!({"doc": 1})),
        ctl("home", "cursor.set", json!({"doc": 1, "anchor": 0})),
        frames("settle", 1),
    ];
    for i in 0..200 {
        lines.push(key(&format!("d{i}"), "ArrowDown", &[]));
    }
    lines.extend([
        snapshot("s200"),
        ctl(
            "set",
            "cursor.set",
            json!({"doc": 1, "anchor": 250 * 16 + 3}),
        ),
        frames("one", 1),
        snapshot("s250"),
        ctl("t", "doc.text", json!({"doc": 1})),
    ]);
    let r = run(&dir, &lines, &["long.txt"]);
    let rows = |id: &str| -> Vec<usize> {
        row_texts(editor(r.root(id)))
            .iter()
            .map(|t| line_of(t))
            .collect()
    };
    let consecutive = |v: &[usize]| v.windows(2).all(|w| w[1] == w[0] + 1);
    let (r0, r1, r4) = (rows("s0"), rows("s1"), rows("s4"));
    assert_eq!(r0[0], 0);
    assert!(r0.len() >= 2);
    assert!(r1[0] > 0 && r4[0] > r1[0], "{r1:?} {r4:?}");
    assert!(consecutive(&r0) && consecutive(&r1) && consecutive(&r4));
    assert_eq!(sel(r.result("g0")), (0, 0));
    // A click without `at` lands at the centre of the visible part: on a line with a row.
    let head = r.result("gc")["line"].as_u64().unwrap() as usize;
    assert!(r4.contains(&head), "{head} not in {r4:?}");
    assert!(rows("s200").contains(&200));
    assert!(rows("s250").contains(&250));
    assert_eq!(r.result("t")["text"], text.as_str());
    assert_eq!(r.by_id("one")["result"]["settled"], false);
}

#[test]
fn undo_redo_and_save_and_a_refused_save() {
    let dir = fresh("save");
    let mut files: Vec<(String, Vec<u8>)> = vec![("src/a.rs".into(), A_RS.into())];
    let lines = [
        focus_editor("fo"),
        json!({"id": "ty", "ui": "type", "text": "x"}),
        frames("settle", 1),
        snapshot("dirty"),
        key("sv", "S", &["ctrl"]),
        snapshot("saved"),
    ];
    write_files(&dir.join("root"), &[("src/a.rs", A_RS.as_bytes())]);
    let r = run(&dir, &lines, &["src/a.rs"]);
    ok_actions(&r);
    assert!(title(r.root("dirty"))["src/a.rs".len()..].contains("modified"));
    assert!(!title(r.root("saved"))["src/a.rs".len()..].contains("modified"));
    assert_eq!(
        fs::read(dir.join("root/src/a.rs")).unwrap(),
        format!("x{A_RS}").as_bytes()
    );
    // The 100 temporary names taken: the save is refused with `io_error`.
    let dir = fresh("refused-save");
    for i in 0..100 {
        files.push((format!("src/.kiln-tmp-{i}"), Vec::new()));
    }
    let refs: Vec<(&str, &[u8])> = files
        .iter()
        .map(|(p, b)| (p.as_str(), b.as_slice()))
        .collect();
    write_files(&dir.join("root"), &refs);
    let r = run(&dir, &lines, &["src/a.rs"]);
    let root = r.root("saved");
    assert!(message(root).starts_with("io_error"), "{}", message(root));
    assert!(title(root)["src/a.rs".len()..].contains("modified"));
    assert_eq!(
        fs::read(dir.join("root/src/a.rs")).unwrap(),
        A_RS.as_bytes()
    );
    assert_eq!(
        r.code,
        Some(0),
        "a UI refusal does not change the exit code"
    );
}

#[cfg(test)]
/// The `mutating` flag of every command (`docs/commands.json`).
fn mutating() -> Vec<(String, bool)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/commands.json");
    let v: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    v["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                c["name"].as_str().unwrap().to_owned(),
                c["mutating"] == true,
            )
        })
        .collect()
}

#[test]
fn the_log_replays_and_idle_frames_add_nothing() {
    let dir = fresh("replay");
    let files: [(&str, &[u8]); 2] = [("src/a.rs", A_RS.as_bytes()), ("c.txt", b"a\r\nb\r\n")];
    write_files(&dir.join("root"), &files);
    let mut lines = vec![
        json!({"id": "c", "ui": "click", "target": editor_target(), "at": {"row": 1, "col": 6}}),
        json!({"id": "t", "ui": "type", "text": "ab"}),
        key("l", "ArrowLeft", &["shift"]),
        json!({"id": "t2", "ui": "type", "text": "Q"}),
        key("en", "Enter", &[]),
        key("bs", "Backspace", &[]),
        key("un", "Z", &["ctrl"]),
        key("re", "Y", &["ctrl"]),
        key("sv", "S", &["ctrl"]),
        ctl("o", "doc.open", json!({"path": "c.txt"})),
        frames("settle", 1),
        key("e", "End", &[]),
        key("en2", "Enter", &[]),
        frames("f", 1),
    ];
    let base = run(&dir, &lines, &["src/a.rs"]);
    ok_actions(&base);
    let saved = fs::read(dir.join("root/src/a.rs")).unwrap();
    lines.push(frames("idle", 600));
    write_files(&dir.join("root"), &files);
    let idle = run(&dir, &lines, &["src/a.rs"]);
    assert_eq!(idle.log, base.log, "idle frames changed the log");
    // Replay the log with the oracle on a copy of the initial files.
    let mut host = kiln_control::Host::in_memory(&files).unwrap();
    for line in base.log.lines() {
        let response: Value =
            serde_json::from_str(&host.handle_line(line.as_bytes()).unwrap()).unwrap();
        assert_eq!(response["ok"], true, "{line}");
    }
    for doc in [1, 2] {
        let ask = |host: &mut kiln_control::Host, command: &str| -> Value {
            let line = json!({"command": command, "params": {"doc": doc}}).to_string();
            serde_json::from_str::<Value>(&host.handle_line(line.as_bytes()).unwrap()).unwrap()["result"].take()
        };
        let text = ask(&mut host, "doc.text");
        let cursor = ask(&mut host, "cursor.get");
        let info = ask(&mut host, "doc.info");
        // The same session through the binary, asked the same at its end.
        let mut check = lines.clone();
        check.extend([
            ctl("qt", "doc.text", json!({"doc": doc})),
            ctl("qc", "cursor.get", json!({"doc": doc})),
            ctl("qi", "doc.info", json!({"doc": doc})),
        ]);
        write_files(&dir.join("root"), &files);
        let r = run(&dir, &check, &["src/a.rs"]);
        assert_eq!(r.result("qt")["text"], text["text"]);
        assert_eq!(sel(r.result("qc")), sel(&cursor));
        for k in ["dirty", "can_undo", "can_redo"] {
            assert_eq!(r.result("qi")[k], info[k], "{k}");
        }
    }
    assert_eq!(
        saved,
        {
            let line = json!({"command": "doc.text", "params": {"doc": 1}}).to_string();
            let v: Value =
                serde_json::from_str(&host.handle_line(line.as_bytes()).unwrap()).unwrap();
            v["result"]["text"].as_str().unwrap().as_bytes().to_vec()
        },
        "the saved file is the replayed text"
    );
    // Every logged request the script did not send is read-only or a command of the Input table.
    let table = [
        "doc.insert",
        "doc.replace",
        "doc.delete",
        "cursor.set",
        "cursor.move",
        "doc.undo",
        "doc.redo",
        "doc.save",
        "doc.open",
    ];
    let flags = mutating();
    for line in base.log.lines() {
        let v: Value = serde_json::from_str(line).unwrap();
        let name = v["command"].as_str().unwrap();
        let is_mutating = flags.iter().find(|(n, _)| n == name).unwrap().1;
        assert!(!is_mutating || table.contains(&name), "{line}");
    }
}

#[test]
fn a_frame_after_a_control_request_is_not_settled() {
    let dir = fresh("settled");
    write_files(&dir.join("root"), &[("a.txt", b"abc\n")]);
    let mut lines = vec![
        ctl(
            "i",
            "doc.insert",
            json!({"doc": 1, "offset": 0, "text": "x"}),
        ),
        frames("first", 1),
    ];
    for n in 0..60 {
        lines.push(frames(&format!("r{n}"), 1));
    }
    lines.extend([snapshot("a"), frames("more", 1), snapshot("b")]);
    let r = run(&dir, &lines, &["a.txt"]);
    assert_eq!(r.by_id("first")["result"]["settled"], false);
    assert_eq!(r.by_id("r0")["result"]["settled"], true);
    assert_eq!(r.by_id("more")["result"]["settled"], true);
    assert_eq!(r.root("a"), r.root("b"));
    assert_eq!(editor(r.root("a"))["value"], "xabc\n");
}

/// Ruling x1: where the inserted `\r` and a `\n` after the selection form one break, the caret
/// is after the whole break, and nothing is refused.
#[test]
fn enter_before_a_newline_in_a_cr_document_puts_the_caret_after_the_break() {
    let dir = fresh("x1");
    write_files(
        &dir.join("root"),
        &[("cr.txt", b"x\ry\rz\n"), ("cr2.txt", b"a\rb\rcd\n")],
    );
    let lines = [
        ctl("i", "doc.info", json!({"doc": 1})),
        ctl("cs", "cursor.set", json!({"doc": 1, "anchor": 5})),
        focus_editor("fo"),
        key("en", "Enter", &[]),
        ctl("t1", "doc.text", json!({"doc": 1})),
        ctl("c1", "cursor.get", json!({"doc": 1})),
        // A selection followed by `\n` (the `d` of `cd\n`), replaced by Enter.
        ctl("o2", "doc.open", json!({"path": "cr2.txt"})),
        ctl("i2", "doc.info", json!({"doc": 2})),
        ctl(
            "sel",
            "cursor.set",
            json!({"doc": 2, "anchor": 5, "head": 6}),
        ),
        key("en2", "Enter", &[]),
        ctl("t2", "doc.text", json!({"doc": 2})),
        ctl("c2", "cursor.get", json!({"doc": 2})),
        frames("settle", 1),
        snapshot("s"),
    ];
    let r = run(&dir, &lines, &["cr.txt"]);
    ok_actions(&r);
    assert_eq!(r.result("i")["line_ending"], "cr");
    assert_eq!(r.result("t1")["text"], "x\ry\rz\r\n");
    assert_eq!(sel(r.result("c1")), (7, 7));
    assert_eq!(r.result("i2")["line_ending"], "cr");
    assert_eq!(r.result("t2")["text"], "a\rb\rc\r\n");
    assert_eq!(sel(r.result("c2")), (7, 7));
    let root = r.root("s");
    assert_eq!(message(root), "");
    assert_eq!(sel(editor(root)), (7, 7));
}

/// Ruling x3: a row that holds only blanks is reported in the `plain` colour, also inside a
/// comment.
#[test]
fn a_row_of_blanks_is_plain() {
    let dir = fresh("x3");
    write_files(
        &dir.join("root"),
        &[("a.rs", b"/* c\n  \t \n*/ fn f() {}\n")],
    );
    let r = run(&dir, &[snapshot("s")], &["a.rs"]);
    let rows = runs(editor(r.root("s")));
    let blank = rows
        .iter()
        .find(|row| {
            row.iter()
                .all(|(t, _)| t.chars().all(|c| c == ' ' || c == '\t'))
        })
        .expect("a row of blanks");
    for (_, c) in blank {
        assert_eq!(c, colour_of("plain"));
    }
    assert_eq!(
        rows[0],
        [("/* c".to_owned(), colour_of("comment").to_owned())]
    );
}

/// Ruling z1: a row's text is the laid-out text's chars in reading order (combining marks,
/// joiners, Arabic-Indic digits, ligatures in proportional type), and a click's `at` column
/// counts those chars.
#[test]
fn rows_report_the_text_and_clicks_count_its_chars() {
    let dir = fresh("z1");
    let lines = [
        "e\u{301}b",
        "ab\u{301}cd",
        "x\u{200d}yz",
        "\u{663}\u{663}",
        "\u{663}\u{663} ab",
        "modified fi office",
    ];
    let text = lines.join("\n") + "\n";
    write_files(
        &dir.join("root"),
        &[("u.txt", text.as_bytes()), ("modified.rs", b"fn f() {}\n")],
    );
    let mut script = vec![
        focus_editor("fo"),
        json!({"id": "ty", "ui": "type", "text": "y"}),
        snapshot("dirty"),
        ctl("o", "doc.open", json!({"path": "u.txt"})),
        frames("settle", 1),
        snapshot("s"),
    ];
    let mut starts = Vec::new();
    let mut at = 0;
    for (r, line) in lines.iter().enumerate() {
        starts.push(at);
        at += line.chars().count() + 1;
        for c in 0..=line.chars().count() {
            script.push(json!({"id": format!("c{r}_{c}"), "ui": "click", "target": editor_target(), "at": {"row": r, "col": c}}));
            script.push(ctl(&format!("g{r}_{c}"), "cursor.get", json!({"doc": 2})));
        }
    }
    let r = run(&dir, &script, &["modified.rs"]);
    let t = title(r.root("dirty"));
    let title_node = labelled(r.root("dirty"), "Title").unwrap();
    assert_eq!(row_texts(title_node).concat(), t);
    assert_eq!(row_texts(editor(r.root("s"))), lines);
    for (row, line) in lines.iter().enumerate() {
        for c in 0..=line.chars().count() {
            let head = sel(r.result(&format!("g{row}_{c}"))).1 as usize;
            assert_eq!(head, starts[row] + c, "row {row} col {c}");
        }
    }
}

/// Ruling z2: growing a document at its end, when it is longer than the view, settles, and the
/// caret's line is in view from the next frame on.
#[test]
fn growing_a_long_document_at_its_end_settles() {
    let dir = fresh("z2");
    let text: String = (0..300).map(|i| format!("line {i:03}\n")).collect();
    write_files(&dir.join("root"), &[("long.txt", text.as_bytes())]);
    let many: String = (0..80).map(|i| format!("\nadded {i:02}")).collect();
    let end = text.chars().count();
    let lines = [
        ctl("cs", "cursor.set", json!({"doc": 1, "anchor": end})),
        focus_editor("fo"),
        key("en", "Enter", &[]),
        json!({"id": "ty", "ui": "type", "text": "QQ"}),
        key("en2", "Enter", &[]),
        key("un", "Z", &["ctrl"]),
        key("re", "Y", &["ctrl"]),
        snapshot("s1"),
        ctl(
            "ins",
            "doc.insert",
            json!({"doc": 1, "offset": end + 4, "text": many}),
        ),
        frames("one", 1),
        snapshot("s2"),
        frames("settle", 60),
        snapshot("s3"),
    ];
    let r = run(&dir, &lines, &["long.txt"]);
    for id in ["en", "ty", "en2", "un", "re"] {
        assert_eq!(r.by_id(id)["result"]["settled"], true, "{id}");
    }
    assert!(row_texts(editor(r.root("s1"))).contains(&"QQ".to_owned()));
    assert!(row_texts(editor(r.root("s2"))).contains(&"added 79".to_owned()));
    assert_eq!(r.by_id("settle")["result"]["settled"], true);
    assert_eq!(
        row_texts(editor(r.root("s2"))),
        row_texts(editor(r.root("s3")))
    );
}
