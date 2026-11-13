//! `kiln-cli run` and `kiln-cli control` through the binary, against real temporary
//! directories (under `CARGO_TARGET_TMPDIR`).

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const SESSION: &str = r#"{"id":1,"command":"doc.open","params":{"path":"a.txt"}}
{"id":2,"command":"doc.insert","params":{"doc":1,"offset":5,"text":" world"}}
{"id":3,"command":"doc.undo","params":{"doc":1}}
{"id":4,"command":"doc.redo","params":{"doc":1}}
{"id":5,"command":"doc.save","params":{"doc":1}}
"#;

/// A fresh, empty directory unique to `name`.
#[cfg(test)]
fn fresh(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("kiln-cli-run")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A fresh root holding `a.txt` = "hello\n".
#[cfg(test)]
fn root(name: &str) -> PathBuf {
    let dir = fresh(name).join("root");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("a.txt"), b"hello\n").unwrap();
    dir
}

#[cfg(test)]
fn kiln_cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
        .args(args)
        .output()
        .unwrap()
}

#[cfg(test)]
fn run(script: &Path, root: &Path, extra: &[&str]) -> Output {
    let mut args = vec![
        "run",
        script.to_str().unwrap(),
        "--root",
        root.to_str().unwrap(),
    ];
    args.extend_from_slice(extra);
    kiln_cli(&args)
}

#[cfg(test)]
fn write_script(dir: &Path, text: &str) -> PathBuf {
    let p = dir.join("s.jsonl");
    fs::write(&p, text).unwrap();
    p
}

/// Every file below `dir` (relative path, bytes), sorted.
#[cfg(test)]
fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else {
                let rel = p.strip_prefix(dir).unwrap().to_string_lossy().into_owned();
                out.push((rel, fs::read(&p).unwrap()));
            }
        }
    }
    out.sort();
    out
}

#[test]
fn session_edits_undoes_and_saves_a_real_file() {
    let root = root("session");
    let script = write_script(root.parent().unwrap(), SESSION);
    let out = run(&script, &root, &[]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 5);
    assert_eq!(
        lines[4],
        r#"{"id":5,"ok":true,"result":{"bytes_written":12,"dirty":false,"doc":1}}"#
    );
    assert_eq!(fs::read(root.join("a.txt")).unwrap(), b"hello world\n");
    assert!(!stdout.contains(root.to_str().unwrap()));
}

const MIXED: &str = "{\"id\":\"s\",\"command\":\"doc.open\",\"params\":{\"path\":\"a.txt\"}}\n\
\n   \n\
not json\n\
{\"command\":\"doc.open\",\"params\":{\"path\":\"missing.txt\"}}\n\
{\"id\":9,\"command\":\"doc.insert\",\"params\":{\"doc\":1,\"offset\":0,\"text\":\"> \"}}\n\
{\"id\":10,\"command\":\"doc.insert\",\"params\":{\"doc\":1,\"offset\":999,\"text\":\"x\"}}\n\
{\"id\":11,\"command\":\"doc.save\",\"params\":{\"doc\":1}}\n\
{\"id\":12,\"command\":\"doc.list\"}\r\n";

#[test]
fn ids_echo_and_failures_exit_3() {
    let root = root("ids");
    let script = write_script(root.parent().unwrap(), MIXED);
    let out = run(&script, &root, &[]);
    assert_eq!(out.status.code(), Some(3), "{out:?}");
    let ids: Vec<serde_json::Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap()["id"].clone())
        .collect();
    let null = serde_json::Value::Null;
    assert_eq!(
        ids,
        [
            "s".into(),
            null.clone(),
            null,
            9.into(),
            10.into(),
            11.into(),
            12.into()
        ]
    );
    assert_eq!(fs::read(root.join("a.txt")).unwrap(), b"> hello\n");
}

#[test]
fn two_roots_give_identical_output_and_files_and_log_replays() {
    let a = root("diff-a");
    let b = root("diff-b/nested");
    let script = write_script(a.parent().unwrap(), MIXED);
    let log_a = a.parent().unwrap().join("l.jsonl");
    let log_b = b.parent().unwrap().join("l.jsonl");
    let out_a = run(&script, &a, &["--log", log_a.to_str().unwrap()]);
    let out_b = run(&script, &b, &["--log", log_b.to_str().unwrap()]);
    assert_eq!(out_a.status.code(), Some(3));
    assert_eq!(out_a.stdout, out_b.stdout);
    assert_eq!(snapshot(&a), snapshot(&b));
    let log = fs::read_to_string(&log_a).unwrap();
    assert_eq!(fs::read_to_string(&log_b).unwrap(), log);
    // Only the successful requests: open, insert(9), save, list.
    let commands: Vec<String> = log
        .lines()
        .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap()["command"].to_string())
        .collect();
    assert_eq!(
        commands,
        [
            "\"doc.open\"",
            "\"doc.insert\"",
            "\"doc.save\"",
            "\"doc.list\""
        ]
    );

    // Replaying the log on a fresh copy reproduces the files and the log itself.
    let c = root("diff-c");
    let log_c = c.parent().unwrap().join("l2.jsonl");
    let out_c = run(&log_a, &c, &["--log", log_c.to_str().unwrap()]);
    assert_eq!(out_c.status.code(), Some(0), "{out_c:?}");
    assert_eq!(snapshot(&c), snapshot(&a));
    assert_eq!(fs::read_to_string(&log_c).unwrap(), log);
}

#[test]
fn usage_errors_exit_2_with_empty_stdout() {
    let root = root("usage");
    let base = root.parent().unwrap();
    let missing = base.join("missing.jsonl");
    let out = run(&missing, &root, &[]);
    assert_eq!((out.status.code(), out.stdout.is_empty()), (Some(2), true));

    let script = write_script(base, SESSION);
    let out = run(&script, &root.join("a.txt"), &[]);
    assert_eq!((out.status.code(), out.stdout.is_empty()), (Some(2), true));

    // Unwritable log: refused before any command runs, so a.txt is untouched.
    let bad_log = base.join("no-such-dir").join("l.jsonl");
    let out = run(&script, &root, &["--log", bad_log.to_str().unwrap()]);
    assert_eq!((out.status.code(), out.stdout.is_empty()), (Some(2), true));
    assert_eq!(fs::read(root.join("a.txt")).unwrap(), b"hello\n");

    assert_eq!(kiln_cli(&["frobnicate"]).status.code(), Some(2));
    assert_eq!(
        kiln_cli(&["control", "--root", root.join("a.txt").to_str().unwrap()])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(kiln_cli(&["control", "extra"]).status.code(), Some(2));
}

#[test]
fn run_defaults_to_the_current_directory() {
    let root = root("cwd");
    let script = write_script(root.parent().unwrap(), SESSION);
    let out = Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
        .args(["run", script.to_str().unwrap()])
        .current_dir(&root)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(fs::read(root.join("a.txt")).unwrap(), b"hello world\n");
}

#[test]
fn exec_acts_on_real_files_under_root() {
    let root = root("exec");
    let out = kiln_cli(&[
        "exec",
        "doc.open",
        r#"{"path":"a.txt"}"#,
        "--root",
        root.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["doc"], 1);
}

#[test]
fn control_answers_each_request_before_the_next() {
    let root = root("control");
    let mut child = Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
        .args(["control", "--root", root.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let requests = [
        r#"{"id":1,"command":"doc.open","params":{"path":"a.txt"}}"#,
        "garbage",
        r#"{"id":3,"command":"doc.insert","params":{"doc":1,"offset":0,"text":"!"}}"#,
        r#"{"id":4,"command":"doc.save","params":{"doc":1}}"#,
    ];
    let mut oks = Vec::new();
    for r in requests {
        stdin.write_all(r.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();
        stdin.flush().unwrap();
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        let v: serde_json::Value = serde_json::from_str(&line).unwrap();
        oks.push(v["ok"].as_bool().unwrap());
    }
    drop(stdin);
    assert_eq!(oks, [true, false, true, true]);
    assert!(child.wait().unwrap().success());
    assert_eq!(fs::read(root.join("a.txt")).unwrap(), b"!hello\n");
}
