//! T-004 acceptance tests for the director's rulings after review r1 (decision D-028, rulings
//! 2–8), through the built `kiln-cli` binary. Written from the task specification alone by the
//! test author. Every test compiles on the base and fails there by assertion. Every spawned
//! process has a timeout (ruling 8).
#![cfg(test)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

const TIMEOUT: Duration = Duration::from_secs(120);

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_kiln-cli")
}

fn fresh_dir(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("acceptance_t004_r2")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn wait_with_timeout(child: &mut Child) -> ExitStatus {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if let Some(st) = child.try_wait().expect("try_wait") {
            return st;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("kiln-cli did not exit within {TIMEOUT:?}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Runs `cmd` with a timeout; stdout is piped unless `pipe_stdout` is false (then `cmd` set it).
fn output_with_timeout(mut cmd: Command, pipe_stdout: bool) -> Output {
    if pipe_stdout {
        cmd.stdout(Stdio::piped());
    }
    let mut child = cmd.stderr(Stdio::piped()).spawn().expect("spawn kiln-cli");
    let drain = |r: Option<Box<dyn std::io::Read + Send>>| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut r) = r {
                let _ = r.read_to_end(&mut buf);
            }
            buf
        })
    };
    let out_t = drain(child.stdout.take().map(|r| Box::new(r) as _));
    let err_t = drain(child.stderr.take().map(|r| Box::new(r) as _));
    let status = wait_with_timeout(&mut child);
    Output {
        status,
        stdout: out_t.join().expect("stdout reader"),
        stderr: err_t.join().expect("stderr reader"),
    }
}

fn kiln(args: &[&str]) -> Output {
    let mut c = Command::new(bin());
    c.args(args).stdin(Stdio::null());
    output_with_timeout(c, true)
}

fn code(out: &Output) -> i32 {
    out.status.code().expect("exited normally")
}

fn lines(out: &Output) -> Vec<Value> {
    String::from_utf8(out.stdout.clone())
        .expect("UTF-8 stdout")
        .lines()
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("not JSON ({e}): {l}")))
        .collect()
}

fn longest_run(s: &str, c: char) -> usize {
    let (mut best, mut cur) = (0, 0);
    for x in s.chars() {
        cur = if x == c { cur + 1 } else { 0 };
        best = best.max(cur);
    }
    best
}

// ---------------------------------------------------------------- ruling 2: --help

#[test]
fn help_flags_print_usage_and_exit_0() {
    for flag in ["--help", "-h"] {
        let out = kiln(&[flag]);
        assert_eq!(code(&out), 0, "`kiln-cli {flag}`: {out:?}");
        let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
        assert!(
            text.contains("usage"),
            "`{flag}` prints usage on stdout: {text}"
        );
        for sub in ["commands", "exec", "run", "control"] {
            assert!(text.contains(sub), "`{flag}` usage names `{sub}`: {text}");
        }
    }
    for sub in ["commands", "exec", "run", "control"] {
        let out = kiln(&[sub, "--help"]);
        assert_eq!(code(&out), 0, "`kiln-cli {sub} --help`: {out:?}");
        let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
        assert!(text.contains("usage"), "`{sub} --help`: {text}");
        assert!(text.contains(sub), "`{sub} --help` names `{sub}`: {text}");
    }
}

// ---------------------------------------------------------------- ruling 3: stdout errors

#[test]
fn run_and_control_exit_1_when_stdout_cannot_be_written() {
    assert!(Path::new("/dev/full").exists(), "this test needs /dev/full");
    let work = fresh_dir("devfull");
    let root = work.join("root");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("a.txt"), b"hello\n").unwrap();
    let script = work.join("s.jsonl");
    std::fs::write(
        &script,
        "{\"id\":1,\"command\":\"doc.list\"}\n{\"id\":2,\"command\":\"doc.open\",\"params\":{\"path\":\"a.txt\"}}\n",
    )
    .unwrap();

    // Positive control: the same invocation with a writable stdout exits 0.
    let ok = kiln(&[
        "run",
        script.to_str().unwrap(),
        "--root",
        root.to_str().unwrap(),
    ]);
    assert_eq!(code(&ok), 0, "{ok:?}");

    let mut c = Command::new(bin());
    c.args([
        "run",
        script.to_str().unwrap(),
        "--root",
        root.to_str().unwrap(),
    ])
    .stdin(Stdio::null())
    .stdout(std::fs::File::create("/dev/full").unwrap());
    let out = output_with_timeout(c, false);
    assert_eq!(code(&out), 1, "run with an unwritable stdout: {out:?}");

    let mut c = Command::new(bin());
    c.args(["control", "--root", root.to_str().unwrap()])
        .stdin(std::fs::File::open(&script).unwrap())
        .stdout(std::fs::File::create("/dev/full").unwrap());
    let out = output_with_timeout(c, false);
    assert_eq!(code(&out), 1, "control with an unwritable stdout: {out:?}");
}

// ---------------------------------------------------------------- ruling 4: exec is one command

#[test]
fn exec_is_one_command_session_and_usage_says_so() {
    let work = fresh_dir("exec-session");
    let root = work.join("root");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("a.txt"), b"hello\n").unwrap();
    let r = root.to_str().unwrap();

    // Reading works: the file is opened from disk.
    let out = kiln(&["exec", "doc.open", r#"{"path":"a.txt"}"#, "--root", r]);
    assert_eq!(code(&out), 0, "{out:?}");
    let v: Value = serde_json::from_slice(&out.stdout).expect("JSON result");
    assert_eq!(v["len_chars"], serde_json::json!(6), "{v}");

    // Each exec is a fresh session: the document opened above does not exist any more, so an
    // edit and a save cannot follow it.
    for (name, params) in [
        ("doc.insert", r#"{"doc":1,"offset":0,"text":"EDIT"}"#),
        ("doc.save", r#"{"doc":1}"#),
    ] {
        let out = kiln(&["exec", name, params, "--root", r]);
        assert_eq!(code(&out), 2, "`exec {name}` in a fresh session: {out:?}");
        assert!(out.stdout.is_empty(), "{out:?}");
    }
    assert_eq!(std::fs::read(root.join("a.txt")).unwrap(), b"hello\n");
    let names: Vec<_> = std::fs::read_dir(&root).unwrap().collect();
    assert_eq!(names.len(), 1, "exec created no files");

    // The usage text says exec does not save and points to run / control.
    for args in [vec!["--help"], vec!["exec", "--help"]] {
        let out = kiln(&args);
        assert_eq!(code(&out), 0, "{args:?}: {out:?}");
        let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
        assert!(
            text.contains("save"),
            "{args:?}: usage says exec edits are not saved: {text}"
        );
        assert!(
            text.contains("run") && text.contains("control"),
            "{args:?}: usage points to run / control: {text}"
        );
    }
}

// ---------------------------------------------------------------- ruling 5: stable `kind`

/// A script whose requests fail in many different ways (protocol, registry, path, OS).
fn failing_script() -> String {
    [
        "not json",
        "[1,2]",
        r#"{"id":7}"#,
        r#"{"command":"no.such"}"#,
        r#"{"command":"doc.insert","params":{"doc":1}}"#,
        r#"{"command":"doc.open","params":{"path":"missing.txt"}}"#,
        r#"{"command":"doc.open","params":{"path":"../outside.txt"}}"#,
        r#"{"command":"doc.open","params":{"path":"sub"}}"#,
        r#"{"command":"doc.open","params":{"path":"a.txt"}}"#,
        r#"{"command":"doc.insert","params":{"doc":1,"offset":999,"text":"x"}}"#,
        r#"{"command":"doc.close","params":{"doc":9}}"#,
    ]
    .iter()
    .map(|l| format!("{l}\n"))
    .collect()
}

fn seed(root: &Path) {
    std::fs::create_dir_all(root.join("sub")).unwrap();
    std::fs::write(root.join("a.txt"), b"hello\n").unwrap();
    std::fs::write(root.parent().unwrap().join("outside.txt"), b"outside\n").unwrap();
}

fn run_on(work: &Path, root: &Path, script: &str) -> Output {
    let s = work.join(format!(
        "script-{}.jsonl",
        root.file_name().unwrap().to_string_lossy()
    ));
    std::fs::write(&s, script).unwrap();
    kiln(&["run", s.to_str().unwrap(), "--root", root.to_str().unwrap()])
}

#[test]
fn error_responses_carry_a_stable_kind() {
    let work = fresh_dir("kind");
    let root_a = work.join("a");
    let root_b = work.join("deeper").join("bb");
    for r in [&root_a, &root_b] {
        std::fs::create_dir_all(r).unwrap();
        seed(r);
    }
    let script = failing_script();
    let out_a = run_on(&work, &root_a, &script);
    let out_b = run_on(&work, &root_b, &script);
    assert_eq!(code(&out_a), 3, "{out_a:?}");
    let (la, lb) = (lines(&out_a), lines(&out_b));
    assert_eq!(la.len(), 11, "{out_a:?}");
    assert_eq!(lb.len(), 11, "{out_b:?}");

    let kinds = |ls: &[Value]| -> Vec<String> {
        ls.iter()
            .filter(|v| v["ok"] == Value::Bool(false))
            .map(|v| {
                let k = v["error"]["kind"]
                    .as_str()
                    .unwrap_or_else(|| panic!("error without a string `kind`: {v}"));
                assert!(
                    !k.is_empty()
                        && k.chars().all(|c| c.is_ascii_lowercase()
                            || c.is_ascii_digit()
                            || "_.-".contains(c)),
                    "`kind` is not a machine-readable token (OS text belongs in `message`): {v}"
                );
                k.to_owned()
            })
            .collect()
    };
    let (ka, kb) = (kinds(&la), kinds(&lb));
    assert_eq!(
        ka.len(),
        10,
        "every request but the a.txt open fails: {la:?}"
    );
    assert_eq!(ka, kb, "kinds differ between the two roots");
    // A second run on the same root gives the same kinds again.
    let out_a2 = run_on(&work, &root_a, &script);
    assert_eq!(kinds(&lines(&out_a2)), ka);
}

// ---------------------------------------------------------------- ruling 6: truncation

#[test]
fn huge_values_are_echoed_truncated_to_200_chars() {
    let work = fresh_dir("truncate");
    let root = work.join("root");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("a.txt"), b"hello\n").unwrap();
    let big = |c: char| c.to_string().repeat(100_000);
    let script = [
        format!(r#"{{"command":"{}"}}"#, big('x')),
        format!(
            r#"{{"command":"doc.open","params":{{"path":"{}"}}}}"#,
            big('y')
        ),
        format!(
            r#"{{"command":"doc.open","params":{{"path":"d/{}.txt"}}}}"#,
            big('w')
        ),
        format!(
            r#"{{"command":"doc.insert","params":{{"doc":1,"offset":"{}","text":"a"}}}}"#,
            big('z')
        ),
        format!(r#"{{"command":"doc.list","{}":1}}"#, big('k')),
        format!(r#"{{"command":"doc.list","params":{{"{}":1}}}}"#, big('q')),
        format!("{{\"command\":\"{}", big('u')),
    ]
    .iter()
    .map(|l| format!("{l}\n"))
    .collect::<String>();
    let out = run_on(&work, &root, &script);
    assert_eq!(code(&out), 3, "{:?}", out.status);
    let ls = lines(&out);
    assert_eq!(ls.len(), 7);
    for (v, c) in ls.iter().zip(['x', 'y', 'w', 'z', 'k', 'q', 'u']) {
        assert_eq!(v["ok"], Value::Bool(false));
        let msg = v["error"]["message"].as_str().expect("message");
        assert!(
            longest_run(msg, c) <= 200,
            "`{c}` value echoed with {} chars (max 200)",
            longest_run(msg, c)
        );
        let details = v["error"]["details"].to_string();
        assert!(longest_run(&details, c) <= 200, "`{c}` in details");
    }

    // The same through `exec`, whose structured error goes to stderr.
    let name = big('x');
    let out = kiln(&["exec", &name, "{}", "--memory"]);
    assert_eq!(code(&out), 2, "{:?}", out.status);
    let err: Value = serde_json::from_slice(&out.stderr).expect("structured error");
    let msg = err["message"].as_str().expect("message");
    assert!(
        longest_run(msg, 'x') <= 200,
        "exec echoed {} chars",
        longest_run(msg, 'x')
    );
}

// ---------------------------------------------------------------- ruling 7: bad root names path

#[test]
fn bad_root_error_names_the_path() {
    let work = fresh_dir("badroot");
    let file = work.join("a-file-not-a-dir.txt");
    std::fs::write(&file, b"x").unwrap();
    let missing = work.join("no-such-root-dir");
    let script = work.join("s.jsonl");
    std::fs::write(&script, "{\"command\":\"doc.list\"}\n").unwrap();
    let s = script.to_str().unwrap();
    for bad in [&file, &missing] {
        let b = bad.to_str().unwrap();
        for args in [
            vec!["run", s, "--root", b],
            vec!["control", "--root", b],
            vec!["exec", "doc.list", "{}", "--root", b],
        ] {
            let out = kiln(&args);
            assert_eq!(code(&out), 2, "{args:?}: {out:?}");
            assert!(out.stdout.is_empty(), "{args:?}: {out:?}");
            let err = String::from_utf8_lossy(&out.stderr);
            assert!(err.contains(b), "{args:?}: the error names {b}: {err}");
        }
    }
}
