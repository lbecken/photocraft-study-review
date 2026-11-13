//! T-011 regression guards (D-010, D-021): behaviour the task must not change, so these pass on
//! the base and on the joined branch. AC-5: `--memory` alone is still an empty in-memory file
//! system for `run` and `exec`; `--root` with `--memory` is still the same usage error;
//! `control` still takes neither `--memory` nor `--memory-seed`. Written from the task
//! specification by the implementer before the implementation (experiment arm, METHODOLOGY §6).
//! Refusal messages are not pinned (only `code`/`kind`), except the pre-existing usage line.
//!
//! Every spawned process is killed after [`TIMEOUT`] (D-028 (8)).
#![cfg(test)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const TIMEOUT: Duration = Duration::from_secs(60);

fn fresh(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("guard_t011")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Run `kiln-cli args` in `cwd`, stdin closed; output files capture stdout/stderr so no pipe
/// can block; killed after [`TIMEOUT`].
fn kiln(args: &[&str], cwd: &Path) -> Output {
    let out_path = cwd.join(".guard-stdout");
    let err_path = cwd.join(".guard-stderr");
    let mut child = Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(std::fs::File::create(&out_path).unwrap())
        .stderr(std::fs::File::create(&err_path).unwrap())
        .spawn()
        .expect("spawn kiln-cli");
    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("kiln-cli did not exit within {TIMEOUT:?}");
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let out = Output {
        status,
        stdout: std::fs::read(&out_path).unwrap(),
        stderr: std::fs::read(&err_path).unwrap(),
    };
    std::fs::remove_file(out_path).unwrap();
    std::fs::remove_file(err_path).unwrap();
    out
}

fn code(out: &Output) -> i32 {
    out.status.code().expect("exited normally")
}

/// AC-5 / Pinned (g): `run --memory` sees no files, even when the current directory has them.
#[test]
fn run_memory_alone_is_still_empty() {
    let work = fresh("run");
    std::fs::write(work.join("a.txt"), "on disk\n").unwrap();
    std::fs::write(
        work.join("s.jsonl"),
        concat!(
            r#"{"command":"doc.open","params":{"path":"a.txt"}}"#,
            "\n",
            r#"{"id":2,"command":"doc.list"}"#,
            "\n"
        ),
    )
    .unwrap();
    let out = kiln(&["run", "s.jsonl", "--memory"], &work);
    assert_eq!(code(&out), 3);
    assert!(out.stderr.is_empty());
    let lines: Vec<Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(
        (&lines[0]["ok"], &lines[0]["id"], &lines[0]["error"]["code"]),
        (&json!(false), &Value::Null, &json!("rejected"))
    );
    assert_eq!(lines[0]["error"]["kind"], json!("not_found"));
    assert_eq!(
        lines[1],
        json!({"id": 2, "ok": true, "result": {"docs": []}})
    );
}

/// AC-5: `exec --memory` sees no files; `--text` still fills the scratch buffer.
#[test]
fn exec_memory_alone_is_still_empty() {
    let work = fresh("exec");
    std::fs::write(work.join("a.txt"), "on disk\n").unwrap();
    let out = kiln(
        &["exec", "doc.open", r#"{"path":"a.txt"}"#, "--memory"],
        &work,
    );
    assert_eq!(code(&out), 2);
    assert!(out.stdout.is_empty());
    let err: Value = serde_json::from_str(String::from_utf8(out.stderr).unwrap().trim()).unwrap();
    assert_eq!(
        (&err["code"], &err["kind"]),
        (&json!("rejected"), &json!("not_found"))
    );
    let out = kiln(
        &["exec", "buffer.text", "{}", "--memory", "--text", "abc"],
        &work,
    );
    assert_eq!(code(&out), 0);
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["text"], json!("abc"));
}

/// `--root` with `--memory` is still the same usage error; `control` takes neither `--memory`
/// nor `--memory-seed` (usage error, nothing on stdout).
#[test]
fn root_memory_conflict_and_control_options_unchanged() {
    let work = fresh("usage");
    std::fs::write(work.join("s.jsonl"), "{\"command\":\"doc.list\"}\n").unwrap();
    std::fs::write(work.join("m.json"), r#"{"files":[]}"#).unwrap();
    for args in [
        vec!["run", "s.jsonl", "--root", ".", "--memory"],
        vec!["exec", "doc.list", "{}", "--memory", "--root", "."],
    ] {
        let out = kiln(&args, &work);
        assert_eq!(code(&out), 2, "{args:?}");
        assert!(out.stdout.is_empty());
        let err = String::from_utf8(out.stderr).unwrap();
        assert_eq!(
            err.lines().next(),
            Some("kiln-cli: options `--root` and `--memory` cannot be combined"),
            "{args:?}"
        );
        assert!(err.contains("usage:"));
    }
    for args in [
        vec!["control", "--memory"],
        vec!["control", "--memory-seed", "m.json"],
    ] {
        let out = kiln(&args, &work);
        assert_eq!(code(&out), 2, "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
        assert!(String::from_utf8(out.stderr).unwrap().contains("usage:"));
    }
}
