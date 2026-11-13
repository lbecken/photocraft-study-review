//! T-039 regression guards (D-010, D-021): the behaviour that the documents changed by T-039
//! state, which already holds on base.
//!
//! - AC-3: `--root`, `--memory` and `--memory-seed` exclude each other, `control` takes neither
//!   `--memory` nor `--memory-seed`, and a bad `--memory-seed` manifest is refused by `run` and
//!   `exec`: exit 2 and nothing on stdout. Under ruling z1 no message or stderr text is asserted.
//! - AC-5: what a `doc.undo` or `doc.redo` with nothing to do leaves unchanged, and that a
//!   successful undo or redo ends the typing group: the five sessions of Pinned (g), every
//!   response asserted exactly.
//! - AC-7: `doc.position` `byte` counts the UTF-8 bytes of the decoded text, not of the file
//!   (a UTF-8 file with a BOM and a UTF-16 LE file), every response asserted exactly.
//!
//! The expected values are those of the task's Pinned (g), computed from the grouping rule of
//! `crates/kiln-text/src/lib.rs` "Undo grouping" and re-run on the base binary.
//!
//! Every spawned process is killed after [`TIMEOUT`] (D-028 (8)).
#![cfg(test)]

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const TIMEOUT: Duration = Duration::from_secs(10);

/// The seed of every session: `a.txt` = `0123456789`, `bom.txt` = UTF-8 with BOM `héx`,
/// `u16.txt` = UTF-16 LE with BOM `héx`.
const MANIFEST: &str = r#"{"files":[
  {"path":"a.txt","text":"0123456789"},
  {"path":"bom.txt","hex":"EFBBBF68C3A978"},
  {"path":"u16.txt","hex":"FFFE6800E9007800"}
]}"#;

/// A fresh directory unique to `name` under `CARGO_TARGET_TMPDIR`, holding `m.json`
/// ([`MANIFEST`]) and `bad.json` (a manifest repeating the key `files`).
fn fresh(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("guard_t039_cli")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("m.json"), MANIFEST).unwrap();
    fs::write(dir.join("bad.json"), r#"{"files":[],"files":[]}"#).unwrap();
    dir
}

/// Run `kiln-cli args` in `cwd` with empty stdin; return (exit code, stdout). Killed after
/// [`TIMEOUT`].
fn kiln(args: &[&str], cwd: &Path) -> (i32, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn kiln-cli");
    let drain = |r: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut r = r;
            let mut buf = Vec::new();
            let _ = r.read_to_end(&mut buf);
            buf
        })
    };
    let out = drain(Box::new(child.stdout.take().expect("stdout")));
    let err = drain(Box::new(child.stderr.take().expect("stderr")));
    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait().expect("try_wait") {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("kiln-cli {args:?} did not finish within {TIMEOUT:?}");
        }
        thread::sleep(Duration::from_millis(5));
    };
    let stdout = String::from_utf8(out.join().expect("stdout reader")).expect("utf-8 stdout");
    let _ = err.join().expect("stderr reader");
    (status.code().expect("exit code"), stdout)
}

/// Run `requests` (commands with params; ids 0, 1, …) as a script through `kiln-cli run
/// --memory-seed m.json` and assert that every response's `result` equals `expected`, in order.
fn session(name: &str, requests: &[(&str, Value)], expected: &[Value]) {
    assert_eq!(
        requests.len(),
        expected.len(),
        "{name}: one expectation per request"
    );
    let dir = fresh(name);
    let script: String = requests
        .iter()
        .enumerate()
        .map(|(id, (command, params))| {
            format!(
                "{}\n",
                json!({"id": id, "command": command, "params": params})
            )
        })
        .collect();
    fs::write(dir.join("s.jsonl"), script).unwrap();
    let (code, stdout) = kiln(&["run", "s.jsonl", "--memory-seed", "m.json"], &dir);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        requests.len(),
        "{name}: one response per request:\n{stdout}"
    );
    for (id, (line, want)) in lines.iter().zip(expected).enumerate() {
        let got: Value = serde_json::from_str(line).expect("response is JSON");
        let want = json!({"id": id, "ok": true, "result": want});
        assert_eq!(got, want, "{name}: response {id} to {:?}", requests[id]);
    }
    assert_eq!(code, 0, "{name}: exit code of run");
}

fn open(path: &str) -> (&'static str, Value) {
    ("doc.open", json!({"path": path}))
}
fn ins(offset: usize, text: &str) -> (&'static str, Value) {
    (
        "doc.insert",
        json!({"doc": 1, "offset": offset, "text": text}),
    )
}
fn on_doc(command: &'static str) -> (&'static str, Value) {
    (command, json!({"doc": 1}))
}

fn opened_a() -> Value {
    json!({"already_open": false, "doc": 1, "encoding": "utf-8", "len_chars": 10,
           "line_ending": "none", "mixed_line_endings": false, "path": "a.txt"})
}
fn edit(dirty: bool, len_chars: usize) -> Value {
    json!({"dirty": dirty, "len_chars": len_chars})
}
fn undone(undone: bool, dirty: bool, len_chars: usize) -> Value {
    json!({"undone": undone, "dirty": dirty, "len_chars": len_chars})
}
fn redone(redone: bool, dirty: bool, len_chars: usize) -> Value {
    json!({"redone": redone, "dirty": dirty, "len_chars": len_chars})
}
fn info(can_undo: bool, can_redo: bool, dirty: bool, len_chars: usize) -> Value {
    json!({"can_redo": can_redo, "can_undo": can_undo, "dirty": dirty, "doc": 1,
           "encoding": "utf-8", "len_chars": len_chars, "len_lines": 1, "line_ending": "none",
           "mixed_line_endings": false, "path": "a.txt"})
}
fn text(t: &str) -> Value {
    json!({"len_chars": t.chars().count(), "text": t})
}
fn cursor(anchor: usize, head: usize) -> Value {
    json!({"anchor": anchor, "head": head, "col": head, "doc": 1, "line": 0,
           "start": anchor.min(head), "end": anchor.max(head)})
}

#[test]
fn memory_seed_conflicts_and_bad_manifest() {
    let dir = fresh("conflicts");
    fs::write(
        dir.join("x.jsonl"),
        "{\"id\":1,\"command\":\"doc.list\",\"params\":{}}\n",
    )
    .unwrap();
    let cases: [&[&str]; 10] = [
        &["run", "x.jsonl", "--memory", "--memory-seed", "m.json"],
        &["run", "x.jsonl", "--root", ".", "--memory-seed", "m.json"],
        &["run", "x.jsonl", "--memory", "--root", "."],
        &[
            "exec",
            "doc.list",
            "{}",
            "--memory",
            "--memory-seed",
            "m.json",
        ],
        &[
            "exec",
            "doc.list",
            "{}",
            "--root",
            ".",
            "--memory-seed",
            "m.json",
        ],
        &["exec", "doc.list", "{}", "--memory", "--root", "."],
        &["control", "--memory-seed", "m.json"],
        &["control", "--memory"],
        &["run", "x.jsonl", "--memory-seed", "bad.json"],
        &["exec", "doc.list", "{}", "--memory-seed", "bad.json"],
    ];
    for args in cases {
        let (code, stdout) = kiln(args, &dir);
        assert_eq!(code, 2, "exit code of kiln-cli {args:?}");
        assert!(
            stdout.is_empty(),
            "stdout of kiln-cli {args:?} not empty: {stdout}"
        );
    }
    // The same script and manifest without a conflict run, so the refusals above come from
    // the options and the bad manifest alone.
    let (code, stdout) = kiln(&["run", "x.jsonl", "--memory-seed", "m.json"], &dir);
    assert_eq!(
        (code, stdout.lines().count()),
        (0, 1),
        "control case: {stdout}"
    );
    let (code, stdout) = kiln(&["exec", "doc.list", "{}", "--memory-seed", "m.json"], &dir);
    assert_eq!(code, 0, "control case exec: {stdout}");
    assert!(!stdout.is_empty(), "control case exec printed nothing");
}

#[test]
fn noop_undo_and_redo_change_nothing() {
    // No-op redo: the typing group stays open, so `b` joins `a` and one undo removes both.
    session(
        "noop_redo",
        &[
            open("a.txt"),
            ins(10, "a"),
            on_doc("doc.redo"),
            ins(11, "b"),
            on_doc("doc.info"),
            on_doc("doc.undo"),
            on_doc("doc.text"),
        ],
        &[
            opened_a(),
            edit(true, 11),
            redone(false, true, 11),
            edit(true, 12),
            info(true, false, true, 12),
            undone(true, false, 10),
            text("0123456789"),
        ],
    );
    // The same without the redo: the same result.
    session(
        "no_redo",
        &[
            open("a.txt"),
            ins(10, "a"),
            ins(11, "b"),
            on_doc("doc.info"),
            on_doc("doc.undo"),
            on_doc("doc.text"),
        ],
        &[
            opened_a(),
            edit(true, 11),
            edit(true, 12),
            info(true, false, true, 12),
            undone(true, false, 10),
            text("0123456789"),
        ],
    );
    // A successful redo ends the typing group: only `b` is undone afterwards.
    session(
        "successful_redo",
        &[
            open("a.txt"),
            ins(0, "x"),
            on_doc("doc.undo"),
            ins(10, "a"),
            on_doc("doc.undo"),
            on_doc("doc.redo"),
            ins(11, "b"),
            on_doc("doc.undo"),
            on_doc("doc.text"),
        ],
        &[
            opened_a(),
            edit(true, 11),
            undone(true, false, 10),
            edit(true, 11),
            undone(true, false, 10),
            redone(true, true, 11),
            edit(true, 12),
            undone(true, true, 11),
            text("0123456789a"),
        ],
    );
    // No-op undo: text, dirty flag, selection and redo history unchanged.
    session(
        "noop_undo",
        &[
            open("a.txt"),
            ("cursor.set", json!({"doc": 1, "anchor": 3, "head": 5})),
            on_doc("doc.undo"),
            on_doc("doc.info"),
            on_doc("cursor.get"),
            ins(0, "xy"),
            on_doc("doc.undo"),
            on_doc("doc.undo"),
            on_doc("doc.info"),
            on_doc("doc.redo"),
            on_doc("doc.text"),
        ],
        &[
            opened_a(),
            cursor(3, 5),
            undone(false, false, 10),
            info(false, false, false, 10),
            cursor(3, 5),
            edit(true, 12),
            undone(true, false, 10),
            undone(false, false, 10),
            info(false, true, false, 10),
            redone(true, true, 12),
            text("xy0123456789"),
        ],
    );
    // No-op redo with a selection: the selection and the document's info unchanged.
    session(
        "noop_redo_selection",
        &[
            open("a.txt"),
            ("cursor.set", json!({"doc": 1, "anchor": 2})),
            on_doc("doc.info"),
            on_doc("doc.redo"),
            on_doc("cursor.get"),
            on_doc("doc.info"),
        ],
        &[
            opened_a(),
            cursor(2, 2),
            info(false, false, false, 10),
            redone(false, false, 10),
            cursor(2, 2),
            info(false, false, false, 10),
        ],
    );
    // A successful undo ends the typing group: `b` at 1 would continue the `a` step, but only
    // `b` is undone afterwards.
    session(
        "successful_undo",
        &[
            open("a.txt"),
            ins(0, "a"),
            ins(5, "z"),
            on_doc("doc.undo"),
            ins(1, "b"),
            on_doc("doc.undo"),
            on_doc("doc.text"),
        ],
        &[
            opened_a(),
            edit(true, 11),
            edit(true, 12),
            undone(true, true, 11),
            edit(true, 12),
            undone(true, true, 11),
            text("a0123456789"),
        ],
    );
}

#[test]
fn byte_offsets_ignore_bom_and_encoding() {
    let pos = |doc: u64, at: Value| ("doc.position", json!({"doc": doc, "at": at}));
    let form = |ch: usize, byte: usize| json!({"byte": byte, "char": ch, "col": ch, "line": 0, "utf16": ch});
    let opened = |doc: u64, path: &str, encoding: &str| {
        json!({"already_open": false, "doc": doc, "encoding": encoding, "len_chars": 3,
               "line_ending": "none", "mixed_line_endings": false, "path": path})
    };
    session(
        "bytes",
        &[
            open("bom.txt"),
            ("doc.text", json!({"doc": 1})),
            pos(1, json!({"char": 2})),
            pos(1, json!({"char": 3})),
            pos(1, json!({"byte": 3})),
            open("u16.txt"),
            ("doc.text", json!({"doc": 2})),
            pos(2, json!({"char": 2})),
            pos(2, json!({"char": 3})),
        ],
        &[
            opened(1, "bom.txt", "utf-8-bom"),
            text("héx"),
            form(2, 3),
            form(3, 4),
            form(2, 3),
            opened(2, "u16.txt", "utf-16le"),
            text("héx"),
            form(2, 3),
            form(3, 4),
        ],
    );
}
