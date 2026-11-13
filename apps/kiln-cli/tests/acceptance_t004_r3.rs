//! T-004 acceptance tests for the director's rulings after review r2 (decision D-031, rulings 9,
//! 10 and 12), through the built `kiln-cli` binary. Written from the task specification alone by
//! the test author. Every test compiles on the base and fails there by assertion. Every spawned
//! process has a timeout (D-028 ruling 8).
#![cfg(test)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

const TIMEOUT: Duration = Duration::from_secs(120);
const MAX_LINE_BYTES: usize = 4096;
const MAX_STRING_CHARS: usize = 200;

// ---------------------------------------------------------------- helpers

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_kiln-cli")
}

fn fresh_dir(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("acceptance_t004_r3")
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

/// Runs `kiln-cli args` with `stdin` (a file, or none) and a timeout.
fn kiln_with(args: &[&str], stdin: Option<&Path>) -> Output {
    let mut c = Command::new(bin());
    c.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(match stdin {
            Some(p) => Stdio::from(std::fs::File::open(p).unwrap()),
            None => Stdio::null(),
        });
    let mut child = c.spawn().expect("spawn kiln-cli");
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
    kiln_with(args, None)
}

fn code(out: &Output) -> i32 {
    out.status.code().expect("exited normally")
}

fn stdout_lines(out: &Output) -> Vec<String> {
    String::from_utf8(out.stdout.clone())
        .expect("UTF-8 stdout")
        .lines()
        .map(str::to_owned)
        .collect()
}

/// Every string in `v`, object keys included.
fn strings(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) => out.push(s.clone()),
        Value::Array(a) => a.iter().for_each(|x| strings(x, out)),
        Value::Object(o) => {
            for (k, x) in o {
                out.push(k.clone());
                strings(x, out);
            }
        }
        _ => {}
    }
}

/// D-031 ruling 9 on one error line (a stdout response, or a JSON error on stderr).
fn assert_capped_error_line(line: &str, what: &str) {
    assert!(
        line.len() <= MAX_LINE_BYTES,
        "{what}: error line is {} bytes (max {MAX_LINE_BYTES})",
        line.len()
    );
    let v: Value = serde_json::from_str(line)
        .unwrap_or_else(|e| panic!("{what}: error line is not JSON ({e}): {line}"));
    let mut all = Vec::new();
    strings(&v, &mut all);
    for s in all {
        let n = s.chars().count();
        assert!(
            n <= MAX_STRING_CHARS,
            "{what}: a string of {n} chars (max {MAX_STRING_CHARS}) in {}…",
            line.chars().take(300).collect::<String>()
        );
    }
}

fn is_error_response(line: &str) -> bool {
    serde_json::from_str::<Value>(line).is_ok_and(|v| v["ok"] == Value::Bool(false))
}

/// Hostile params of several shapes (name, params JSON). Each is invalid for `doc.insert`.
fn hostile_params() -> Vec<(&'static str, String)> {
    let huge = "x".repeat(100_000);
    let arr: Vec<String> = (0..15_000).map(|i| format!("\"a b {}\"", i % 10)).collect();
    let mut nested = String::from("1");
    for _ in 0..100 {
        nested = format!("{{\"a b\":{nested}}}");
    }
    let keys: Vec<String> = (0..5_000)
        .map(|i| format!("\"unknown key {i}\":1"))
        .collect();
    vec![
        (
            "huge string",
            format!(r#"{{"doc":1,"offset":0,"text":0,"x":"{huge}"}}"#),
        ),
        (
            "huge string value of the wrong type",
            format!(r#"{{"doc":1,"offset":"{huge}","text":"a"}}"#),
        ),
        (
            "huge array of short strings with spaces",
            format!(r#"{{"doc":1,"offset":[{}],"text":"a"}}"#, arr.join(",")),
        ),
        (
            "deeply nested object",
            format!(r#"{{"doc":1,"offset":{nested},"text":"a"}}"#),
        ),
        (
            "thousands of unknown keys",
            format!(r#"{{"doc":1,"offset":0,"text":"a",{}}}"#, keys.join(",")),
        ),
        (
            "huge multi-byte string",
            format!(
                r#"{{"doc":1,"offset":"{}","text":"a"}}"#,
                "é😀".repeat(15_000)
            ),
        ),
    ]
}

/// Whole hostile request lines for `run` / `control` (protocol-level as well as params-level).
fn hostile_lines() -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = hostile_params()
        .into_iter()
        .map(|(what, p)| {
            (
                what.to_owned(),
                format!(r#"{{"command":"doc.insert","params":{p}}}"#),
            )
        })
        .collect();
    let keys: Vec<String> = (0..5_000).map(|i| format!("\"top {i}\":1")).collect();
    v.push((
        "huge command name".into(),
        format!(r#"{{"command":"{}"}}"#, "c".repeat(100_000)),
    ));
    v.push((
        "huge multi-byte command name".into(),
        format!(r#"{{"command":"{}"}}"#, "ü".repeat(60_000)),
    ));
    v.push((
        "thousands of unknown top-level keys".into(),
        format!(r#"{{"command":"doc.list",{}}}"#, keys.join(",")),
    ));
    v.push((
        "huge array as params".into(),
        format!(
            r#"{{"command":"doc.list","params":[{}]}}"#,
            vec!["\"s p\""; 20_000].join(",")
        ),
    ));
    v.push((
        "huge path".into(),
        format!(
            r#"{{"command":"doc.open","params":{{"path":"{}"}}}}"#,
            "p".repeat(100_000)
        ),
    ));
    v
}

// ---------------------------------------------------------------- ruling 9: caps

#[test]
fn run_and_control_error_responses_obey_size_caps() {
    let work = fresh_dir("caps-run");
    let root = work.join("root");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("a.txt"), b"hello\n").unwrap();
    let hostile = hostile_lines();
    let mut script = String::from("{\"command\":\"doc.open\",\"params\":{\"path\":\"a.txt\"}}\n");
    for (_, l) in &hostile {
        script.push_str(l);
        script.push('\n');
    }
    // A small invalid value is still reported intact.
    script.push_str(r#"{"command":"doc.insert","params":{"doc":1,"offset":"zero","text":"a"}}"#);
    script.push('\n');
    script.push_str(r#"{"command":"no.such.cmd"}"#);
    script.push('\n');
    let s = work.join("s.jsonl");
    std::fs::write(&s, &script).unwrap();
    let r = root.to_str().unwrap();

    for mode in ["run", "control"] {
        let out = if mode == "run" {
            kiln(&["run", s.to_str().unwrap(), "--root", r])
        } else {
            kiln_with(&["control", "--root", r], Some(&s))
        };
        let want = if mode == "run" { 3 } else { 0 };
        assert_eq!(code(&out), want, "{mode}: {:?}", out.status);
        let lines = stdout_lines(&out);
        assert_eq!(
            lines.len(),
            hostile.len() + 3,
            "{mode}: one response per request"
        );
        assert!(!is_error_response(&lines[0]), "{mode}: {}", lines[0]);
        for (i, (what, _)) in hostile.iter().enumerate() {
            let line = &lines[1 + i];
            assert!(is_error_response(line), "{mode}: {what} must fail");
            assert_capped_error_line(line, &format!("{mode}: {what}"));
            if what == "huge command name" {
                // The message names the command, so it must have been cut, with a trailing `…`.
                let v: Value = serde_json::from_str(line).unwrap();
                let msg = v["error"]["message"].as_str().expect("message");
                assert!(
                    msg.ends_with('…'),
                    "{mode}: truncated message ends with `…`: {msg}"
                );
            }
        }
        let small = &lines[hostile.len() + 1];
        assert_capped_error_line(small, mode);
        assert!(
            small.contains("zero"),
            "{mode}: small value reported intact: {small}"
        );
        let unknown = &lines[hostile.len() + 2];
        assert_capped_error_line(unknown, mode);
        assert!(unknown.contains("no.such.cmd"), "{mode}: {unknown}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            err.len() <= MAX_LINE_BYTES,
            "{mode}: stderr is {} bytes",
            err.len()
        );
    }
}

#[test]
fn exec_errors_on_stderr_obey_size_caps() {
    let work = fresh_dir("caps-exec");
    let root = work.join("root");
    std::fs::create_dir(&root).unwrap();
    let r = root.to_str().unwrap();
    let mut cases: Vec<(String, String, String)> = hostile_params()
        .into_iter()
        .map(|(what, p)| (what.to_owned(), "doc.insert".to_owned(), p))
        .collect();
    cases.push(("huge command name".into(), "n".repeat(100_000), "{}".into()));
    cases.push((
        "huge multi-byte path".into(),
        "doc.open".into(),
        format!(r#"{{"path":"{}"}}"#, "ö".repeat(50_000)),
    ));
    for (what, name, params) in &cases {
        let out = kiln(&["exec", name, params, "--root", r]);
        assert_eq!(code(&out), 2, "exec {what}: {:?}", out.status);
        assert!(out.stdout.is_empty(), "exec {what}");
        let err = String::from_utf8(out.stderr.clone()).expect("UTF-8 stderr");
        assert_capped_error_line(err.trim_end_matches('\n'), &format!("exec {what}"));
    }
    // Small values stay intact.
    let out = kiln(&[
        "exec",
        "doc.insert",
        r#"{"doc":1,"offset":"zero","text":"a"}"#,
        "--root",
        r,
    ]);
    let err = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_capped_error_line(err.trim_end_matches('\n'), "exec small");
    assert!(err.contains("zero"), "{err}");
    let out = kiln(&["exec", "no.such.cmd", "{}", "--root", r]);
    let err = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_capped_error_line(err.trim_end_matches('\n'), "exec unknown");
    assert!(err.contains("no.such.cmd"), "{err}");
}

#[test]
fn usage_errors_on_stderr_obey_size_caps_and_name_the_problem() {
    let work = fresh_dir("caps-usage");
    let s = work.join("s.jsonl");
    std::fs::write(&s, "{\"command\":\"doc.list\"}\n").unwrap();
    let s = s.to_str().unwrap();
    let r = work.to_str().unwrap();
    let huge_flag = format!("--{}", "f".repeat(10_000));
    // Positive control: valid invocation exits 0.
    assert_eq!(code(&kiln(&["run", s, "--root", r])), 0);
    for args in [
        vec!["run", s, huge_flag.as_str()],
        vec!["control", huge_flag.as_str()],
        vec!["exec", "doc.list", "{}", huge_flag.as_str()],
    ] {
        let out = kiln(&args);
        assert_eq!(code(&out), 2, "{:?}", out.status);
        assert!(out.stdout.is_empty());
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            err.len() <= MAX_LINE_BYTES,
            "usage error is {} bytes",
            err.len()
        );
        let longest = err
            .split(|c: char| c != 'f')
            .map(str::len)
            .max()
            .unwrap_or(0);
        assert!(
            longest <= MAX_STRING_CHARS,
            "flag echoed with {longest} chars"
        );
        assert!(err.contains("--ffff"), "names the flag (truncated): {err}");
    }
}

// ---------------------------------------------------------------- ruling 12: usage names problem

#[test]
fn usage_error_names_the_offending_flag_or_missing_argument() {
    let work = fresh_dir("usage-names");
    let s = work.join("s.jsonl");
    std::fs::write(&s, "{\"command\":\"doc.list\"}\n").unwrap();
    let s = s.to_str().unwrap();
    let r = work.to_str().unwrap();
    assert_eq!(code(&kiln(&["run", s, "--root", r])), 0, "positive control");

    for (args, needle) in [
        (vec!["run", s, "--root", r, "--bogus-flag"], "--bogus-flag"),
        (vec!["control", "--root", r, "--frobnicate"], "--frobnicate"),
        (vec!["exec", "doc.list", "{}", "--wibble"], "--wibble"),
        (vec!["run", s, "--log-it"], "--log-it"),
    ] {
        let out = kiln(&args);
        assert_eq!(code(&out), 2, "{args:?}: {out:?}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            err.contains(needle),
            "{args:?}: stderr names `{needle}`: {err}"
        );
    }
    let says_missing = |e: &str| {
        let e = e.to_lowercase();
        [
            "missing", "required", "requires", "expected", "expects", "needs",
        ]
        .iter()
        .any(|w| e.contains(w))
    };
    for (args, needle) in [
        (vec!["run"], "script"),
        (vec!["run", s, "--root"], "--root"),
        (vec!["run", s, "--log"], "--log"),
        (vec!["control", "--root"], "--root"),
        (vec!["exec"], "name"),
    ] {
        let out = kiln(&args);
        assert_eq!(code(&out), 2, "{args:?}: {out:?}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            says_missing(&err) && err.contains(needle),
            "{args:?}: stderr says `{needle}` is missing: {err}"
        );
    }
}

// ---------------------------------------------------------------- ruling 10: documented kinds

/// The kinds documented in `docs/control-protocol.md`: every inline-code token that looks like
/// a machine-readable identifier. (The document's exact layout is not specified, so this is a
/// superset; membership in it is still a real constraint on returned kinds.)
fn documented_kinds() -> BTreeSet<String> {
    let doc = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/control-protocol.md");
    assert!(
        doc.is_file(),
        "docs/control-protocol.md must exist ({})",
        doc.display()
    );
    let text = std::fs::read_to_string(&doc).unwrap();
    assert!(
        text.contains("kind"),
        "the protocol document describes `kind`"
    );
    // Drop code fences so inline-code backticks pair up.
    let text = text.replace("```", "\n");
    let mut out = BTreeSet::new();
    for (i, part) in text.split('`').enumerate() {
        let token = part.trim();
        if i % 2 == 1
            && !token.is_empty()
            && token
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_.-".contains(c))
        {
            out.insert(token.to_owned());
        }
    }
    out
}

/// The eight representative failures (D-031 ruling 10), in two variants that differ only in the
/// values involved. (label, variant A, variant B, expected `code`)
fn representative() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    vec![
        (
            "unknown command",
            r#"{"command":"no.such"}"#,
            r#"{"command":"other.missing"}"#,
            "unknown_command",
        ),
        (
            "malformed JSON",
            "not json at all",
            r#"{"command" "doc.list"}"#,
            "invalid_json",
        ),
        (
            "wrong param type",
            r#"{"command":"doc.insert","params":{"doc":1,"offset":"zero","text":"a"}}"#,
            r#"{"command":"doc.insert","params":{"doc":1,"offset":0,"text":7}}"#,
            "invalid_params",
        ),
        (
            "missing param",
            r#"{"command":"doc.insert","params":{"doc":1,"offset":0}}"#,
            r#"{"command":"doc.insert","params":{"doc":1,"text":"a"}}"#,
            "invalid_params",
        ),
        (
            "out-of-range offset",
            r#"{"command":"doc.insert","params":{"doc":1,"offset":999,"text":"a"}}"#,
            r#"{"command":"doc.insert","params":{"doc":1,"offset":77,"text":"b"}}"#,
            "rejected",
        ),
        (
            "unknown document",
            r#"{"command":"doc.text","params":{"doc":42}}"#,
            r#"{"command":"doc.text","params":{"doc":43}}"#,
            "rejected",
        ),
        (
            "file not found",
            r#"{"command":"doc.open","params":{"path":"missing.txt"}}"#,
            r#"{"command":"doc.open","params":{"path":"sub/other-missing.rs"}}"#,
            "rejected",
        ),
        (
            "path refused",
            r#"{"command":"doc.open","params":{"path":"../outside.txt"}}"#,
            r#"{"command":"doc.open","params":{"path":"sub/../../outside.txt"}}"#,
            "rejected",
        ),
        (
            "request not an object",
            "42",
            r#""a string""#,
            "invalid_request",
        ),
        (
            "request without command",
            r#"{"id":7}"#,
            r#"{"params":{}}"#,
            "invalid_request",
        ),
    ]
}

#[test]
fn error_kinds_are_documented_and_pinned_to_situations() {
    let documented = documented_kinds();
    let work = fresh_dir("kinds");
    let mut by_root = Vec::new();
    for name in ["one", "two"] {
        let root = work.join(name);
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("a.txt"), b"hello\n").unwrap();
        std::fs::write(work.join("outside.txt"), b"outside\n").unwrap();
        let mut script =
            String::from("{\"command\":\"doc.open\",\"params\":{\"path\":\"a.txt\"}}\n");
        for (_, a, b, _) in representative() {
            script.push_str(&format!("{a}\n{b}\n"));
        }
        let s = work.join(format!("{name}.jsonl"));
        std::fs::write(&s, &script).unwrap();
        let out = kiln(&["run", s.to_str().unwrap(), "--root", root.to_str().unwrap()]);
        assert_eq!(code(&out), 3, "{out:?}");
        let lines = stdout_lines(&out);
        assert_eq!(lines.len(), 1 + 2 * representative().len(), "{out:?}");
        assert!(!is_error_response(&lines[0]));
        by_root.push(lines);
    }
    assert_eq!(
        by_root[0], by_root[1],
        "same script on two roots, same responses"
    );

    let mut kind_of: BTreeMap<&str, String> = BTreeMap::new();
    let mut code_of_kind: BTreeMap<String, String> = BTreeMap::new();
    for (i, (label, _, _, want_code)) in representative().iter().enumerate() {
        let mut kinds = Vec::new();
        for line in [&by_root[0][1 + 2 * i], &by_root[0][2 + 2 * i]] {
            let v: Value = serde_json::from_str(line).unwrap();
            assert_eq!(v["ok"], Value::Bool(false), "{label}: {line}");
            assert_eq!(v["error"]["code"], *want_code, "{label}: {line}");
            let k = v["error"]["kind"]
                .as_str()
                .unwrap_or_else(|| panic!("{label}: no string `kind`: {line}"))
                .to_owned();
            assert!(
                documented.contains(&k),
                "{label}: kind `{k}` is not in docs/control-protocol.md ({documented:?})"
            );
            kinds.push(k);
        }
        assert_eq!(
            kinds[0], kinds[1],
            "{label}: the kind depends on the situation, not values"
        );
        if let Some(c) = code_of_kind.insert(kinds[0].clone(), want_code.to_string()) {
            assert_eq!(&c, want_code, "kind `{}` used with two codes", kinds[0]);
        }
        kind_of.insert(label, kinds[0].clone());
    }
    // `exec` errors carry the same kinds as `run` for the same situation.
    let root = work.join("one");
    let r = root.to_str().unwrap();
    for (label, name, params) in [
        ("unknown command", "no.such", "{}"),
        ("file not found", "doc.open", r#"{"path":"missing.txt"}"#),
        ("path refused", "doc.open", r#"{"path":"../outside.txt"}"#),
        ("unknown document", "doc.text", r#"{"doc":42}"#),
        (
            "wrong param type",
            "doc.insert",
            r#"{"doc":1,"offset":"zero","text":"a"}"#,
        ),
    ] {
        let out = kiln(&["exec", name, params, "--root", r]);
        assert_eq!(code(&out), 2, "exec {label}: {out:?}");
        let err: Value = serde_json::from_slice(&out.stderr)
            .unwrap_or_else(|e| panic!("exec {label}: stderr is not JSON ({e}): {out:?}"));
        let k = err["kind"]
            .as_str()
            .unwrap_or_else(|| panic!("exec {label}: no `kind`: {err}"));
        assert!(documented.contains(k), "exec {label}: `{k}` undocumented");
        assert_eq!(k, kind_of[label], "exec {label}: kind differs from run");
    }
}
