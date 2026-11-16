//! T-059 implementer tests: the status bar through the built `kiln` binary (`kiln script`),
//! beside the acceptance tests. They cover the worked examples of Pinned (g), columns counted in
//! chars for the alphabet's hard cases against values computed here, the target-collision rule
//! of Pinned (f) (a `Documents` entry whose path is a number stays one target), undo and save,
//! and an empty document.
//!
//! Every spawned process is killed after [`TIMEOUT`]; roots are temporary directories under
//! `CARGO_TARGET_TMPDIR`; nodes are found by role and label (or value). No Unix-only API.
#![cfg(test)]

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// Longest any spawned process may run.
const TIMEOUT: Duration = Duration::from_secs(300);

const ROOT: &str = "root";

/// Frames after a control request (a margin, not a pinned count).
const SETTLE: u64 = 10;

/// `kiln script` on `script` in `dir` with `--root root`: the responses by id.
fn run(dir: &Path, script: &[Value]) -> HashMap<String, Value> {
    let mut text = String::new();
    for l in script {
        text.push_str(&l.to_string());
        text.push('\n');
    }
    fs::write(dir.join("s.jsonl"), text).expect("write script");
    let mut child = Command::new(env!("CARGO_BIN_EXE_kiln"))
        .args(["script", "s.jsonl", "--root", ROOT])
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn kiln");
    let mut out = child.stdout.take().expect("stdout");
    let reader = thread::spawn(move || {
        let mut s = String::new();
        let _ = out.read_to_string(&mut s);
        s
    });
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if child.try_wait().expect("try_wait").is_some() {
            break;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("kiln script did not finish within {TIMEOUT:?}");
        }
        thread::sleep(Duration::from_millis(5));
    }
    let stdout = reader.join().expect("reader");
    let mut by_id = HashMap::new();
    for line in stdout.lines() {
        let v: Value = serde_json::from_str(line).expect("response is JSON");
        let id = v["id"].as_str().expect("string id").to_owned();
        by_id.insert(id, v);
    }
    assert_eq!(by_id.len(), script.len(), "one response per request");
    by_id
}

fn case(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("t059-status")
        .join(name);
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(d.join(ROOT)).expect("case dir");
    d
}

fn put(dir: &Path, path: &str, bytes: &[u8]) {
    let p = dir.join(ROOT).join(path);
    fs::create_dir_all(p.parent().expect("parent")).expect("dirs");
    fs::write(p, bytes).expect("write");
}

fn ok<'a>(r: &'a HashMap<String, Value>, id: &str) -> &'a Value {
    let v = &r[id];
    assert_eq!(v["ok"], json!(true), "`{id}` refused: {v}");
    &v["result"]
}

fn preorder(n: &Value) -> Vec<&Value> {
    let mut out = vec![n];
    for c in n["children"].as_array().into_iter().flatten() {
        out.extend(preorder(c));
    }
    out
}

/// The bar's four values `[Line, Column, Encoding, Line ending]` of snapshot `id`; each node of
/// role `label` and painting exactly its value on one row.
fn bar(r: &HashMap<String, Value>, id: &str) -> [String; 4] {
    let root = &ok(r, id)["root"];
    ["Line", "Column", "Encoding", "Line ending"].map(|name| {
        let found: Vec<&Value> = preorder(root)
            .into_iter()
            .filter(|n| n["label"] == json!(name))
            .collect();
        assert_eq!(found.len(), 1, "{id}: one node labelled {name}");
        let n = found[0];
        assert_eq!(n["role"], json!("label"), "{id}: {name}'s role");
        let v = n["value"].as_str().expect("value").to_owned();
        let rows: Vec<String> = n["rows"]
            .as_array()
            .expect("rows")
            .iter()
            .map(|row| {
                row.as_array()
                    .expect("row")
                    .iter()
                    .map(|run| run["text"].as_str().expect("text"))
                    .collect()
            })
            .collect();
        assert_eq!(
            rows.as_slice(),
            std::slice::from_ref(&v),
            "{id}: {name} paints its value"
        );
        v
    })
}

fn ui(id: &str, action: &str, mut fields: Value) -> Value {
    fields["id"] = json!(id);
    fields["ui"] = json!(action);
    fields
}

fn ctl(id: &str, command: &str, params: Value) -> Value {
    json!({"id": id, "command": command, "params": params})
}

fn frames(id: &str) -> Value {
    ui(id, "frames", json!({"count": SETTLE}))
}

fn snap(id: &str) -> Value {
    ui(id, "snapshot", json!({}))
}

fn s(v: [&str; 4]) -> [String; 4] {
    v.map(str::to_owned)
}

/// Pinned (g): the six opens, `mixed.txt`, `bom.txt`, `b.txt` and the click and type in
/// `src/a.rs`.
#[test]
fn worked_examples_of_pinned_g() {
    let dir = case("g");
    put(&dir, "src/a.rs", b"fn main() {\n    let x = 1;\n}\n");
    put(&dir, "b.txt", b"b\r\nc\r\n");
    put(&dir, "mixed.txt", b"x\ny\r\n");
    put(&dir, "bom.txt", b"\xEF\xBB\xBFhi\n");
    put(&dir, "d/e.txt", b"e");
    put(&dir, "\u{5e9}\u{5dc}\u{5d5}\u{5dd}.txt", "h\n".as_bytes());
    let mut script = Vec::new();
    for (k, p) in [
        "src/a.rs",
        "b.txt",
        "mixed.txt",
        "bom.txt",
        "d/e.txt",
        "\u{5e9}\u{5dc}\u{5d5}\u{5dd}.txt",
    ]
    .iter()
    .enumerate()
    {
        script.push(ctl(&format!("o{k}"), "doc.open", json!({"path": p})));
        script.push(frames(&format!("f{k}")));
        script.push(snap(&format!("s{k}")));
    }
    script.push(ctl("back", "doc.open", json!({"path": "src/a.rs"})));
    script.push(frames("fb"));
    script.push(ui(
        "c",
        "click",
        json!({"target": {"label": "Editor"}, "at": {"row": 1, "col": 4}}),
    ));
    script.push(ui("t", "type", json!({"text": "z"})));
    script.push(snap("st"));
    script.push(ctl("g", "cursor.get", json!({"doc": 1})));
    let r = run(&dir, &script);
    assert_eq!(bar(&r, "s0"), s(["Ln 1", "Col 1", "utf-8", "lf"]));
    assert_eq!(bar(&r, "s1"), s(["Ln 1", "Col 1", "utf-8", "crlf"]));
    assert_eq!(bar(&r, "s2"), s(["Ln 1", "Col 1", "utf-8", "lf (mixed)"]));
    assert_eq!(bar(&r, "s3"), s(["Ln 1", "Col 1", "utf-8-bom", "lf"]));
    assert_eq!(bar(&r, "s4"), s(["Ln 1", "Col 1", "utf-8", "none"]));
    assert_eq!(bar(&r, "s5"), s(["Ln 1", "Col 1", "utf-8", "lf"]));
    let g = ok(&r, "g");
    assert_eq!(
        (g["line"].clone(), g["col"].clone(), g["head"].clone()),
        (json!(1), json!(5), json!(17))
    );
    assert_eq!(bar(&r, "st"), s(["Ln 2", "Col 6", "utf-8", "lf"]));
}

/// Columns in chars: after each control `cursor.set` the column is the number of chars between
/// the line's start and the head, plus one, computed here from the text (a combining mark, a
/// joiner, a tab, an emoji (two UTF-16 units) and a CRLF each as the commands count them).
#[test]
fn columns_count_chars_in_a_utf16_document() {
    let dir = case("cols");
    let text = "a\te\u{301}\u{200d}\u{1f600}ffi\r\n\u{6f22}\u{5b57}\u{633}\u{644}\rz";
    let mut bytes = vec![0xFF, 0xFE];
    for u in text.encode_utf16() {
        bytes.extend_from_slice(&u.to_le_bytes());
    }
    put(&dir, "w.txt", &bytes);
    let chars: Vec<char> = text.chars().collect();
    // Every valid head (not between the CR and LF of the CRLF).
    let heads: Vec<usize> = (0..=chars.len())
        .filter(|&h| !(h > 0 && chars.get(h - 1) == Some(&'\r') && chars.get(h) == Some(&'\n')))
        .collect();
    let mut script = vec![ctl("o", "doc.open", json!({"path": "w.txt"})), frames("f")];
    for h in &heads {
        script.push(ctl(
            &format!("k{h}"),
            "cursor.set",
            json!({"doc": 1, "anchor": 0, "head": h}),
        ));
        script.push(frames(&format!("f{h}")));
        script.push(snap(&format!("s{h}")));
    }
    let r = run(&dir, &script);
    for h in heads {
        let (mut line, mut col, mut i) = (0, 0, 0);
        while i < h {
            match chars[i] {
                '\r' if chars.get(i + 1) == Some(&'\n') => {
                    line += 1;
                    col = 0;
                    i += 1;
                }
                '\r' | '\n' => {
                    line += 1;
                    col = 0;
                }
                _ => col += 1,
            }
            i += 1;
        }
        let shown = bar(&r, &format!("s{h}"));
        assert_eq!(
            shown,
            [
                format!("Ln {}", line + 1),
                format!("Col {}", col + 1),
                "utf-16le".to_owned(),
                "crlf (mixed)".to_owned()
            ],
            "head {h}"
        );
    }
}

/// Pinned (f): a document whose path is a number (`1`, `7`) keeps one `Documents` target
/// `{"role":"label","value":<path>}` while the bar shows `Ln 1`, `Col 1`, `Ln 7`, `Col 7`.
#[test]
fn a_numeric_path_stays_one_documents_target() {
    let dir = case("numeric");
    put(&dir, "1", b"one\n");
    put(&dir, "7", b"a\nb\nc\nd\ne\nf\nseven!\n");
    let script = vec![
        ctl("o1", "doc.open", json!({"path": "1"})),
        ctl("o7", "doc.open", json!({"path": "7"})),
        ctl("k", "cursor.set", json!({"doc": 2, "anchor": 18})),
        frames("f"),
        snap("s0"),
        ui(
            "e1",
            "click",
            json!({"target": {"role": "label", "value": "1"}}),
        ),
        snap("s1"),
        ui(
            "e7",
            "click",
            json!({"target": {"role": "label", "value": "7"}}),
        ),
        snap("s2"),
    ];
    let r = run(&dir, &script);
    assert_eq!(bar(&r, "s0"), s(["Ln 7", "Col 7", "utf-8", "lf"]));
    ok(&r, "e1");
    assert_eq!(bar(&r, "s1"), s(["Ln 1", "Col 1", "utf-8", "lf"]));
    ok(&r, "e7");
    assert_eq!(bar(&r, "s2"), s(["Ln 7", "Col 7", "utf-8", "lf"]));
}

/// Typing a line break, undo by `ctrl`+`Z` and a save by `ctrl`+`S`: the bar follows
/// `cursor.get` and `doc.info` and shows no dirty state; an empty document shows `none`.
#[test]
fn undo_save_and_an_empty_document() {
    let dir = case("undo");
    put(&dir, "c.txt", b"ab\r\ncd");
    put(&dir, "empty.txt", b"");
    let script = vec![
        ctl("o", "doc.open", json!({"path": "c.txt"})),
        frames("f"),
        ui("fe", "focus", json!({"target": {"label": "Editor"}})),
        ui("end", "key", json!({"key": "End"})),
        ui("en", "key", json!({"key": "Enter"})),
        snap("s1"),
        ui("z", "key", json!({"key": "Z", "modifiers": ["ctrl"]})),
        snap("s2"),
        ui("t", "type", json!({"text": "\u{1f600}"})),
        ui("sv", "key", json!({"key": "S", "modifiers": ["ctrl"]})),
        snap("s3"),
        ctl("g", "cursor.get", json!({"doc": 1})),
        ctl("i", "doc.info", json!({"doc": 1})),
        ctl("oe", "doc.open", json!({"path": "empty.txt"})),
        frames("fe2"),
        snap("s4"),
    ];
    let r = run(&dir, &script);
    assert_eq!(bar(&r, "s1"), s(["Ln 2", "Col 1", "utf-8", "crlf"]));
    assert_eq!(bar(&r, "s2"), s(["Ln 1", "Col 3", "utf-8", "crlf"]));
    assert_eq!(bar(&r, "s3"), s(["Ln 1", "Col 4", "utf-8", "crlf"]));
    assert_eq!(ok(&r, "g")["col"], json!(3));
    assert_eq!(ok(&r, "i")["dirty"], json!(false), "precondition: saved");
    assert_eq!(bar(&r, "s4"), s(["Ln 1", "Col 1", "utf-8", "none"]));
}
