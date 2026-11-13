//! T-018 regression guards through the built `kiln-cli` binary: lasting contracts the task
//! must not change (Pinned (f), "Out of scope"). They pass on base and must pass after T-018.
//!
//! - Bytes as read: nothing normalises line endings at open, at save or in `doc.insert`
//!   (io.crlf); only an explicit `eol.normalize` may.
//! - The selection is not part of `doc.info`, and the results of the existing edit commands
//!   keep their values.
//!
//! Every spawned process is killed after [`TIMEOUT`] (D-028 (8)).
#![cfg(test)]

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const TIMEOUT: Duration = Duration::from_secs(120);

fn fresh_dir(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("guard_t018")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("create temp dir");
    d
}

/// Run `kiln-cli args` (empty stdin); kill it after [`TIMEOUT`].
fn kiln(args: &[&str]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
        .args(args)
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
    Output {
        status,
        stdout: out.join().expect("stdout reader"),
        stderr: err.join().expect("stderr reader"),
    }
}

/// Run `requests` (command, params) as a `kiln-cli run` script with `--root root`; every
/// request must succeed. Returns the results.
fn run_ok(work: &Path, root: &Path, requests: &[(&str, Value)]) -> Vec<Value> {
    let script = work.join("script.jsonl");
    let lines: Vec<String> = requests
        .iter()
        .enumerate()
        .map(|(i, (c, p))| json!({"id": i + 1, "command": c, "params": p}).to_string())
        .collect();
    std::fs::write(&script, lines.join("\n") + "\n").expect("write script");
    let out = kiln(&[
        "run",
        script.to_str().expect("utf-8 path"),
        "--root",
        root.to_str().expect("utf-8 path"),
    ]);
    let stdout = String::from_utf8(out.stdout).expect("stdout UTF-8");
    let responses: Vec<Value> = stdout
        .lines()
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("not JSON ({e}): {l}")))
        .collect();
    assert_eq!(responses.len(), requests.len(), "one response per request");
    for (line, resp) in lines.iter().zip(&responses) {
        assert_eq!(resp["ok"], json!(true), "{line} -> {resp}");
    }
    assert_eq!(out.status.code(), Some(0), "exit code");
    responses.into_iter().map(|r| r["result"].clone()).collect()
}

/// io.crlf: a mixed file opened and saved keeps its bytes; text inserted with `doc.insert`
/// (line breaks included) is written exactly as given, the rest of the file unchanged.
#[test]
fn open_save_and_insert_keep_line_ending_bytes() {
    let work = fresh_dir("bytes");
    let root = work.join("root");
    std::fs::create_dir_all(&root).expect("root");
    let mixed = "a\r\nb\nc\r\r\nd\ré😀\n";
    std::fs::write(root.join("keep.txt"), mixed).expect("write");
    std::fs::write(root.join("edit.txt"), mixed).expect("write");
    let results = run_ok(
        &work,
        &root,
        &[
            ("doc.open", json!({"path": "keep.txt"})),
            ("doc.text", json!({"doc": 1})),
            ("doc.save", json!({"doc": 1})),
            ("doc.open", json!({"path": "edit.txt"})),
            (
                "doc.insert",
                json!({"doc": 2, "offset": 3, "text": "x\ny\r"}),
            ),
            ("doc.insert", json!({"doc": 2, "offset": 0, "text": "\r\n"})),
            ("doc.save", json!({"doc": 2})),
        ],
    );
    assert_eq!(results[1]["text"], json!(mixed), "doc.text is verbatim");
    assert_eq!(
        std::fs::read(root.join("keep.txt")).expect("read"),
        mixed.as_bytes()
    );
    let mut want: Vec<char> = mixed.chars().collect();
    want.splice(3..3, "x\ny\r".chars());
    want.splice(0..0, "\r\n".chars());
    let want: String = want.into_iter().collect();
    assert_eq!(
        std::fs::read(root.join("edit.txt")).expect("read"),
        want.as_bytes()
    );
}

/// The selection is not part of `doc.info`, and the existing edit commands' results keep
/// their values (no selection field is added to them).
#[test]
fn doc_results_carry_no_selection() {
    let work = fresh_dir("results");
    let root = work.join("root");
    std::fs::create_dir_all(&root).expect("root");
    std::fs::write(root.join("a.txt"), "ab\r\ncd").expect("write");
    let results = run_ok(
        &work,
        &root,
        &[
            ("doc.open", json!({"path": "a.txt"})),
            ("doc.insert", json!({"doc": 1, "offset": 0, "text": "é"})),
            ("doc.delete", json!({"doc": 1, "start": 0, "end": 1})),
            (
                "doc.replace",
                json!({"doc": 1, "start": 0, "end": 1, "text": "X"}),
            ),
            ("doc.undo", json!({"doc": 1})),
            ("doc.redo", json!({"doc": 1})),
            ("doc.info", json!({"doc": 1})),
        ],
    );
    assert_eq!(results[1], json!({"len_chars": 7, "dirty": true}));
    assert_eq!(
        results[2],
        json!({"removed": "é", "len_chars": 6, "dirty": false})
    );
    assert_eq!(
        results[3],
        json!({"removed": "a", "len_chars": 6, "dirty": true})
    );
    assert_eq!(
        results[4],
        json!({"undone": true, "len_chars": 6, "dirty": false})
    );
    assert_eq!(
        results[5],
        json!({"redone": true, "len_chars": 6, "dirty": true})
    );
    let info = results[6].as_object().expect("doc.info object");
    for key in ["anchor", "head", "selection", "cursor", "start", "end"] {
        assert!(!info.contains_key(key), "doc.info has no `{key}`: {info:?}");
    }
    assert_eq!(info["len_chars"], json!(6));
    assert_eq!(info["line_ending"], json!("crlf"));
    assert_eq!(info["mixed_line_endings"], json!(false));
}
