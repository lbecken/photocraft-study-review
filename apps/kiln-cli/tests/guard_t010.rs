//! T-010 regression guards (D-010): existing behaviour the task must not change, through the
//! built `kiln-cli` binary. They pass on base and must pass after the join.
//!
//! - `doc.info.len_lines` counts `\r\n`, `\r` and `\n` as one break each (AC-1 compares the new
//!   `doc.line` with it).
//! - The existing read-only document commands (`doc.info`, `doc.text`, `doc.list`) and a
//!   refused insert do not end a typing group (AC-4 holds the new commands to the same rule).
//!
//! Every spawned process is killed after [`TIMEOUT`] (D-028 (8)).
#![cfg(test)]

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const TIMEOUT: Duration = Duration::from_secs(60);

fn fresh_dir(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("guard_t010")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("create temp dir");
    d
}

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

/// Run `requests` (command, params) with `--root root`; return the responses.
fn run(work: &Path, root: &Path, requests: &[(&str, Value)]) -> Vec<Value> {
    let script = work.join("s.jsonl");
    let text: String = requests
        .iter()
        .enumerate()
        .map(|(i, (c, p))| format!("{}\n", json!({"id": i + 1, "command": c, "params": p})))
        .collect();
    std::fs::write(&script, text).unwrap();
    let out = kiln(&[
        "run",
        script.to_str().unwrap(),
        "--root",
        root.to_str().unwrap(),
    ]);
    let responses: Vec<Value> = String::from_utf8(out.stdout)
        .expect("UTF-8")
        .lines()
        .map(|l| serde_json::from_str(l).expect("JSON"))
        .collect();
    assert_eq!(responses.len(), requests.len());
    responses
}

/// Naive line count: breaks `\r\n`, `\r`, `\n` (one each) + 1.
fn naive_len_lines(t: &str) -> usize {
    t.replace("\r\n", "\n").matches(['\r', '\n']).count() + 1
}

#[test]
fn doc_info_len_lines_counts_every_break_kind_once() {
    let texts = [
        "",
        "a",
        "\n",
        "\r",
        "\r\n",
        "\r\r\n\n",
        "\n\r",
        "a\r\nbé😀\rc\n",
        "x\ry",
        "😀\r\n",
    ];
    let work = fresh_dir("len-lines");
    let root = work.join("root");
    std::fs::create_dir_all(&root).unwrap();
    let mut req = Vec::new();
    for (i, t) in texts.iter().enumerate() {
        std::fs::write(root.join(format!("t{i}.txt")), t).unwrap();
        req.push(("doc.open", json!({"path": format!("t{i}.txt")})));
        req.push(("doc.info", json!({"doc": i + 1})));
    }
    let r = run(&work, &root, &req);
    for (i, t) in texts.iter().enumerate() {
        let info = &r[2 * i + 1];
        assert_eq!(info["ok"], json!(true), "{info}");
        assert_eq!(
            info["result"]["len_lines"],
            json!(naive_len_lines(t)),
            "{t:?}"
        );
    }
    assert_eq!(naive_len_lines("a\r\nbé😀\rc\n"), 4);
}

#[test]
fn read_only_commands_do_not_end_a_typing_group() {
    let work = fresh_dir("typing");
    let root = work.join("root");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("u.txt"), "ab\n").unwrap();
    let r = run(
        &work,
        &root,
        &[
            ("doc.open", json!({"path": "u.txt"})),
            ("doc.insert", json!({"doc": 1, "offset": 0, "text": "x"})),
            ("doc.info", json!({"doc": 1})),
            ("doc.text", json!({"doc": 1})),
            ("doc.list", json!({})),
            ("doc.insert", json!({"doc": 1, "offset": 99, "text": "z"})),
            ("doc.insert", json!({"doc": 1, "offset": 1, "text": "y"})),
            ("doc.undo", json!({"doc": 1})),
            ("doc.text", json!({"doc": 1})),
            ("doc.info", json!({"doc": 1})),
        ],
    );
    assert_eq!(r[5]["error"]["kind"], json!("out_of_range"), "{}", r[5]);
    assert_eq!(r[7]["result"]["undone"], json!(true), "{}", r[7]);
    assert_eq!(r[8]["result"]["text"], json!("ab\n"), "{}", r[8]);
    assert_eq!(r[9]["result"]["can_undo"], json!(false), "{}", r[9]);
    assert_eq!(r[9]["result"]["can_redo"], json!(true), "{}", r[9]);
}
