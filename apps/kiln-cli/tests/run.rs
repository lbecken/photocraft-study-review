//! `kiln-cli run` and `kiln-cli control` through the binary, against real temporary
//! directories (under `CARGO_TARGET_TMPDIR`).

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// Every spawned `kiln-cli` is killed and the test fails after this long (D-028 (8)).
const TIMEOUT: Duration = Duration::from_secs(60);

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

/// Run `cmd` to completion with stdout/stderr captured, killing it after [`TIMEOUT`].
#[cfg(test)]
fn output_with_timeout(cmd: &mut Command) -> Output {
    let mut child = cmd
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let drain = |r: Option<Box<dyn Read + Send>>| {
        thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut r) = r {
                let _ = r.read_to_end(&mut buf);
            }
            buf
        })
    };
    let out = drain(
        child
            .stdout
            .take()
            .map(|o| Box::new(o) as Box<dyn Read + Send>),
    );
    let err = drain(
        child
            .stderr
            .take()
            .map(|e| Box::new(e) as Box<dyn Read + Send>),
    );
    let status = wait_or_kill(&mut child);
    Output {
        status,
        stdout: out.join().unwrap(),
        stderr: err.join().unwrap(),
    }
}

#[cfg(test)]
fn wait_or_kill(child: &mut Child) -> std::process::ExitStatus {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("kiln-cli did not finish within {TIMEOUT:?}");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(test)]
fn kiln_cli(args: &[&str]) -> Output {
    output_with_timeout(
        Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
            .args(args)
            .stdout(Stdio::piped()),
    )
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
    let out = output_with_timeout(
        Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
            .args(["run", script.to_str().unwrap()])
            .current_dir(&root)
            .stdout(Stdio::piped()),
    );
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
    // Lines arrive through a channel so that a missing response fails after TIMEOUT instead
    // of blocking the suite.
    let (tx, rx) = mpsc::channel::<String>();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    thread::spawn(move || {
        loop {
            let mut line = String::new();
            match stdout.read_line(&mut line) {
                Ok(0) | Err(_) => return,
                Ok(_) => {
                    if tx.send(line).is_err() {
                        return;
                    }
                }
            }
        }
    });
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
        let Ok(line) = rx.recv_timeout(TIMEOUT) else {
            let _ = child.kill();
            let _ = child.wait();
            panic!("no response to {r} within {TIMEOUT:?}");
        };
        let v: serde_json::Value = serde_json::from_str(&line).unwrap();
        oks.push(v["ok"].as_bool().unwrap());
    }
    drop(stdin);
    assert_eq!(oks, [true, false, true, true]);
    assert!(wait_or_kill(&mut child).success());
    assert_eq!(fs::read(root.join("a.txt")).unwrap(), b"!hello\n");
}

#[test]
fn help_prints_usage_and_exits_0() {
    for args in [
        &["--help"][..],
        &["-h"],
        &["run", "--help"],
        &["run", "s.jsonl", "-h"],
        &["exec", "--help"],
        &["control", "--help"],
        &["commands", "-h"],
    ] {
        let out = kiln_cli(args);
        assert_eq!(out.status.code(), Some(0), "{args:?}: {out:?}");
        let text = String::from_utf8(out.stdout).unwrap();
        assert!(text.starts_with("usage:"), "{args:?}: {text}");
        assert!(text.contains("never saved"), "{args:?}: exec is explained");
    }
    // `-h` as the value of --root is a root, not a help request.
    assert_eq!(
        kiln_cli(&["run", "s.jsonl", "--root", "-h"]).status.code(),
        Some(2)
    );
}

#[test]
fn bad_root_error_names_the_path() {
    let base = fresh("bad-root");
    let missing = base.join("no-such-root");
    let file = base.join("file");
    fs::write(&file, b"x").unwrap();
    let script = write_script(&base, SESSION);
    for root in [&missing, &file] {
        let r = root.to_str().unwrap();
        for args in [
            vec!["run", script.to_str().unwrap(), "--root", r],
            vec!["control", "--root", r],
            vec!["exec", "doc.list", "{}", "--root", r],
        ] {
            let out = kiln_cli(&args);
            assert_eq!(out.status.code(), Some(2), "{args:?}");
            assert!(out.stdout.is_empty());
            let err = String::from_utf8(out.stderr).unwrap();
            assert!(err.contains(r), "{args:?}: {err}");
        }
    }
}

#[test]
fn run_exits_1_when_stdout_cannot_be_written() {
    // Linux only, like the rest of the disk layer (D-014: Unix).
    let full = Path::new("/dev/full");
    let root = root("full");
    let script = write_script(root.parent().unwrap(), SESSION);
    let sink = fs::OpenOptions::new().write(true).open(full).unwrap();
    let out = output_with_timeout(
        Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
            .args([
                "run",
                script.to_str().unwrap(),
                "--root",
                root.to_str().unwrap(),
            ])
            .stdout(sink),
    );
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("stdout"));
}

#[test]
fn exec_edits_are_not_saved_across_calls() {
    let root = root("exec-session");
    let r = root.to_str().unwrap();
    let out = kiln_cli(&["exec", "doc.open", r#"{"path":"a.txt"}"#, "--root", r]);
    assert_eq!(out.status.code(), Some(0));
    let out = kiln_cli(&["exec", "doc.save", r#"{"doc":1}"#, "--root", r]);
    assert_eq!(out.status.code(), Some(2));
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["code"], "rejected");
    assert_eq!(fs::read(root.join("a.txt")).unwrap(), b"hello\n");
}

#[test]
fn error_responses_carry_kind_through_run() {
    let root = root("kinds");
    let script = write_script(
        root.parent().unwrap(),
        "nope\n{\"command\":\"doc.open\",\"params\":{\"path\":\"missing.txt\"}}\n",
    );
    let out = run(&script, &root, &[]);
    assert_eq!(out.status.code(), Some(3));
    let kinds: Vec<String> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap()["error"]["kind"].to_string())
        .collect();
    assert_eq!(kinds, ["\"syntax\"", "\"not_found\""]);
}
