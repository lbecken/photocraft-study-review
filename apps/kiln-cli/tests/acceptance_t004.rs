//! T-004 acceptance tests through the built `kiln-cli` binary (`run`, `control`, and the D-023
//! `--root` / `--memory` ruling for `exec` and `run`). Written from the task specification alone
//! by the test author. Every test drives the binary as a subprocess against real temporary
//! directories, so the file compiles on the base and fails there by assertion (the base has no
//! `run`/`control` subcommands and `exec` accepts no `--root`).

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use serde_json::Value;

// ---------------------------------------------------------------- helpers

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_kiln-cli")
}

/// A fresh, empty directory under cargo's per-target temp dir, unique per test and process.
fn fresh_dir(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("acceptance_t004_cli")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn write_files(root: &Path, files: &[(&str, &[u8])]) {
    for (p, bytes) in files {
        let full = root.join(p);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(full, bytes).unwrap();
    }
}

/// Every regular file under `root`, by relative path, with its bytes.
fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                walk(base, &p, out);
            } else {
                let rel = p.strip_prefix(base).unwrap().to_string_lossy().into_owned();
                out.insert(rel, std::fs::read(&p).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn kiln(args: &[&str], cwd: Option<&Path>) -> Output {
    let mut c = Command::new(bin());
    c.args(args).stdin(Stdio::null());
    if let Some(d) = cwd {
        c.current_dir(d);
    }
    c.output().expect("kiln-cli runs")
}

/// `kiln-cli run <script> --root <root> [--log <log>]`, the script written outside the root.
fn run_script(work: &Path, root: &Path, script: &[u8], log: Option<&Path>) -> Output {
    let script_path = work.join(format!(
        "script-{}.jsonl",
        root.file_name().unwrap().to_string_lossy()
    ));
    std::fs::write(&script_path, script).unwrap();
    let mut args = vec![
        "run".to_owned(),
        script_path.to_string_lossy().into_owned(),
        "--root".to_owned(),
        root.to_string_lossy().into_owned(),
    ];
    if let Some(l) = log {
        args.push("--log".to_owned());
        args.push(l.to_string_lossy().into_owned());
    }
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    kiln(&args, None)
}

fn stdout_lines(out: &Output) -> Vec<String> {
    let s = String::from_utf8(out.stdout.clone()).expect("stdout is UTF-8");
    assert!(
        s.is_empty() || s.ends_with('\n'),
        "every response line is \\n-terminated: {s:?}"
    );
    s.lines().map(str::to_owned).collect()
}

fn parse(line: &str) -> Value {
    serde_json::from_str(line).unwrap_or_else(|e| panic!("response is not JSON ({e}): {line}"))
}

fn code(out: &Output) -> i32 {
    out.status.code().expect("exited normally")
}

const AC1_SCRIPT: &str = concat!(
    r#"{"id":1,"command":"doc.open","params":{"path":"a.txt"}}"#,
    "\n",
    r#"{"id":2,"command":"doc.insert","params":{"doc":1,"offset":5,"text":" world"}}"#,
    "\n",
    r#"{"id":3,"command":"doc.undo","params":{"doc":1}}"#,
    "\n",
    r#"{"id":4,"command":"doc.redo","params":{"doc":1}}"#,
    "\n",
    r#"{"id":5,"command":"doc.save","params":{"doc":1}}"#,
    "\n",
);

const AC1_LINE5: &str = r#"{"id":5,"ok":true,"result":{"bytes_written":12,"dirty":false,"doc":1}}"#;

/// A script mixing successes, registry errors, protocol errors and path-bearing failures.
fn mixed_script() -> Vec<u8> {
    let mut s = Vec::new();
    for l in [
        r#"{"id":"open","command":"doc.open","params":{"path":"a.txt"}}"#,
        r#"{"id":2,"command":"doc.insert","params":{"text":"é!","offset":0,"doc":1}}"#,
        r#"{"id":3,"command":"doc.open","params":{"path":"missing.txt"}}"#,
        r#"{"id":4,"command":"doc.open","params":{"path":"../outside.txt"}}"#,
        r#"{"id":5,"command":"doc.open","params":{"path":"/etc/hosts"}}"#,
        r#"{"id":6,"command":"doc.insert","params":{"doc":1,"offset":999,"text":"x"}}"#,
        "not json",
        r#"{"command":"no.such"}"#,
        r#"{"id":9,"command":"doc.insert","params":{"doc":1}}"#,
        r#"{"command":"doc.open","params":{"path":"sub/b.rs"}}"#,
        r#"{"id":11,"command":"doc.text","params":{"doc":2}}"#,
        r#"{"id":12,"command":"doc.save","params":{"doc":1}}"#,
        r#"{"id":13,"command":"doc.list"}"#,
    ] {
        s.extend_from_slice(l.as_bytes());
        s.push(b'\n');
    }
    s
}

fn mixed_files() -> Vec<(&'static str, &'static [u8])> {
    vec![
        ("a.txt", b"hello\n".as_slice()),
        ("sub/b.rs", b"fn b() {}\r\n".as_slice()),
    ]
}

/// Asserts AC-7 on every line of `out`: JSON, canonical (sorted keys, compact), no root path.
fn assert_canonical(out: &Output, roots: &[&Path]) {
    let raw = String::from_utf8(out.stdout.clone()).expect("UTF-8 stdout");
    for line in raw.lines() {
        let v = parse(line);
        // serde_json without `preserve_order` serializes object keys sorted, compactly.
        let again = serde_json::to_string(&v).unwrap();
        assert_eq!(again, line, "response is not canonical JSON");
        assert!(!line.ends_with(' '), "trailing space: {line:?}");
        assert!(v["ok"].is_boolean(), "`ok` missing: {line}");
    }
    for r in roots {
        for form in [r.to_path_buf(), r.canonicalize().unwrap()] {
            let s = form.to_string_lossy().into_owned();
            assert!(
                !raw.contains(&s),
                "a response contains the absolute root path {s}: {raw}"
            );
        }
    }
}

// ---------------------------------------------------------------- AC-1 (control.session)

#[test]
fn open_edit_undo_redo_save_through_run() {
    let work = fresh_dir("ac1");
    let root = work.join("root");
    std::fs::create_dir(&root).unwrap();
    write_files(&root, &[("a.txt", b"hello\n")]);

    let out = run_script(&work, &root, AC1_SCRIPT.as_bytes(), None);
    assert_eq!(code(&out), 0, "{out:?}");
    let lines = stdout_lines(&out);
    assert_eq!(lines.len(), 5, "exactly one line per request: {lines:?}");
    for (i, l) in lines.iter().enumerate() {
        let v = parse(l);
        assert_eq!(v["ok"], Value::Bool(true), "line {}: {l}", i + 1);
        assert_eq!(v["id"], serde_json::json!(i + 1), "line {}: {l}", i + 1);
    }
    assert_eq!(lines[4], AC1_LINE5);
    assert_eq!(std::fs::read(root.join("a.txt")).unwrap(), b"hello world\n");
}

/// D-023: `--root` defaults to the current directory.
#[test]
fn run_root_defaults_to_current_directory() {
    let work = fresh_dir("ac1-cwd");
    let root = work.join("root");
    std::fs::create_dir(&root).unwrap();
    write_files(&root, &[("a.txt", b"hello\n")]);
    let script = work.join("s.jsonl");
    std::fs::write(&script, AC1_SCRIPT).unwrap();

    let out = kiln(&["run", script.to_str().unwrap()], Some(&root));
    assert_eq!(code(&out), 0, "{out:?}");
    let lines = stdout_lines(&out);
    assert_eq!(lines.len(), 5, "{lines:?}");
    assert_eq!(lines[4], AC1_LINE5);
    assert_eq!(std::fs::read(root.join("a.txt")).unwrap(), b"hello world\n");
}

/// D-023: `--memory` selects the empty in-memory file system; nothing on disk is visible or
/// changed.
#[test]
fn run_memory_flag_uses_empty_in_memory_fs() {
    let work = fresh_dir("memory");
    let root = work.join("root");
    std::fs::create_dir(&root).unwrap();
    write_files(&root, &[("a.txt", b"hello\n")]);
    let script = work.join("s.jsonl");
    std::fs::write(
        &script,
        concat!(
            r#"{"id":1,"command":"doc.list"}"#,
            "\n",
            r#"{"id":2,"command":"doc.open","params":{"path":"a.txt"}}"#,
            "\n"
        ),
    )
    .unwrap();

    let out = kiln(&["run", script.to_str().unwrap(), "--memory"], Some(&root));
    assert_eq!(code(&out), 3, "a.txt must not exist in memory: {out:?}");
    let lines = stdout_lines(&out);
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert_eq!(lines[0], r#"{"id":1,"ok":true,"result":{"docs":[]}}"#);
    let v = parse(&lines[1]);
    assert_eq!(v["ok"], Value::Bool(false), "{v}");
    assert_eq!(v["id"], serde_json::json!(2));
    assert_eq!(snapshot(&root).len(), 1);
    assert_eq!(std::fs::read(root.join("a.txt")).unwrap(), b"hello\n");
}

/// D-023: `exec` acts on real files under `--root` (default: current directory) and on an
/// empty in-memory file system with `--memory`.
#[test]
fn exec_opens_real_files_under_root() {
    let work = fresh_dir("exec");
    let root = work.join("root");
    std::fs::create_dir(&root).unwrap();
    write_files(&root, &[("a.txt", b"hello\n")]);
    let params = r#"{"path":"a.txt"}"#;

    let check = |out: &Output| {
        assert_eq!(code(out), 0, "{out:?}");
        let v: Value = serde_json::from_slice(&out.stdout).expect("JSON result");
        assert_eq!(v["doc"], serde_json::json!(1), "{v}");
        assert_eq!(v["path"], "a.txt", "{v}");
        assert_eq!(v["len_chars"], serde_json::json!(6), "{v}");
        assert_eq!(v["already_open"], Value::Bool(false), "{v}");
    };
    check(&kiln(
        &["exec", "doc.open", params, "--root", root.to_str().unwrap()],
        None,
    ));
    check(&kiln(&["exec", "doc.open", params], Some(&root)));

    let out = kiln(&["exec", "doc.open", params, "--memory"], Some(&root));
    assert_eq!(code(&out), 2, "no a.txt in an empty memory fs: {out:?}");
    assert!(out.stdout.is_empty(), "{out:?}");
    let err: Value = serde_json::from_slice(&out.stderr).expect("structured error");
    assert_eq!(err["code"], "rejected", "{err}");
    assert_eq!(std::fs::read(root.join("a.txt")).unwrap(), b"hello\n");
}

// ---------------------------------------------------------------- AC-2 (control.protocol)

#[test]
fn run_one_response_per_request_with_ids_echoed() {
    let work = fresh_dir("ac2");
    let root = work.join("root");
    std::fs::create_dir(&root).unwrap();
    write_files(&root, &[("a.txt", b"abc")]);
    // (line, Some(expected id) for non-blank lines)
    let script: Vec<(&str, Option<Value>)> = vec![
        ("", None),
        (
            r#"{"id":"first","command":"doc.open","params":{"path":"a.txt"}}"#,
            Some(serde_json::json!("first")),
        ),
        ("   \t ", None),
        (r#"{"command":"doc.list"}"#, Some(Value::Null)),
        (
            r#"{"id":0,"command":"doc.list"}"#,
            Some(serde_json::json!(0)),
        ),
        (
            r#"{"id":-3,"command":"no.such"}"#,
            Some(serde_json::json!(-3)),
        ),
        ("", None),
        ("", None),
        (
            r#"{"id":"","command":"doc.text","params":{"doc":1}}"#,
            Some(serde_json::json!("")),
        ),
        (
            r#"{"id":9007199254740993,"command":"doc.list"}"#,
            Some(serde_json::json!(9007199254740993_u64)),
        ),
        (
            r#"{"id":"dup","command":"doc.insert","params":{"doc":1,"offset":99,"text":"x"}}"#,
            Some(serde_json::json!("dup")),
        ),
        (
            r#"{"id":"dup","command":"doc.list"}"#,
            Some(serde_json::json!("dup")),
        ),
        (
            r#"  {"id":"ws","command":"doc.list"}  "#,
            Some(serde_json::json!("ws")),
        ),
        (
            r#"{"params":{"doc":1},"command":"doc.undo"}"#,
            Some(Value::Null),
        ),
    ];
    let mut bytes = Vec::new();
    for (l, _) in &script {
        bytes.extend_from_slice(l.as_bytes());
        bytes.push(b'\n');
    }
    let expected: Vec<&Value> = script.iter().filter_map(|(_, id)| id.as_ref()).collect();

    let out = run_script(&work, &root, &bytes, None);
    let lines = stdout_lines(&out);
    assert_eq!(
        lines.len(),
        expected.len(),
        "one response per non-blank line: {out:?}"
    );
    for (i, (line, id)) in lines.iter().zip(&expected).enumerate() {
        let v = parse(line);
        assert_eq!(&&v["id"], id, "response {} has the wrong id: {line}", i + 1);
        assert!(
            v.as_object().unwrap().contains_key("id"),
            "`id` key present: {line}"
        );
    }
    assert_eq!(code(&out), 3, "two requests failed: {out:?}");
}

// ---------------------------------------------------------------- AC-3 (cli.run)

#[test]
fn same_script_gives_byte_identical_output() {
    let work = fresh_dir("ac3");
    let root_a = work.join("a");
    let root_b = work.join("second-copy-with-a-longer-path").join("nested");
    for r in [&root_a, &root_b] {
        std::fs::create_dir_all(r).unwrap();
        write_files(r, &mixed_files());
    }
    let script = mixed_script();

    let out_a = run_script(&work, &root_a, &script, None);
    let out_b = run_script(&work, &root_b, &script, None);
    assert_eq!(
        code(&out_a),
        3,
        "the script contains failing requests: {out_a:?}"
    );
    assert_eq!(code(&out_b), 3, "{out_b:?}");
    assert_eq!(stdout_lines(&out_a).len(), 13, "{out_a:?}");
    assert_eq!(
        String::from_utf8_lossy(&out_a.stdout),
        String::from_utf8_lossy(&out_b.stdout),
        "stdout differs between runs"
    );
    assert_eq!(out_a.stdout, out_b.stdout);
    assert_eq!(
        snapshot(&root_a),
        snapshot(&root_b),
        "files differ between runs"
    );
    assert_eq!(
        std::fs::read(root_a.join("a.txt")).unwrap(),
        "é!hello\n".as_bytes(),
        "doc.save wrote the edit"
    );
    // Failures carry error messages, and they too are identical (checked above) and free of
    // host paths.
    assert_canonical(&out_a, &[&root_a, &work]);
    assert_canonical(&out_b, &[&root_b, &work]);

    // A third fresh copy gives the same bytes again.
    let root_c = work.join("c");
    std::fs::create_dir(&root_c).unwrap();
    write_files(&root_c, &mixed_files());
    let out_c = run_script(&work, &root_c, &script, None);
    assert_eq!(out_c.stdout, out_a.stdout);
}

// ---------------------------------------------------------------- AC-4 (cli.run)

#[test]
fn log_script_replays_and_is_idempotent() {
    let work = fresh_dir("ac4");
    let root1 = work.join("one");
    let root2 = work.join("two");
    for r in [&root1, &root2] {
        std::fs::create_dir(r).unwrap();
        write_files(r, &mixed_files());
    }
    let l1 = work.join("l1.jsonl");
    let l2 = work.join("l2.jsonl");

    let out1 = run_script(&work, &root1, &mixed_script(), Some(&l1));
    assert_eq!(code(&out1), 3, "{out1:?}");
    let log1 = std::fs::read_to_string(&l1).expect("--log file written");

    // Exactly the successful requests, in order, ids 1-based, keys sorted at every level.
    let expected = concat!(
        r#"{"command":"doc.open","id":1,"params":{"path":"a.txt"}}"#,
        "\n",
        r#"{"command":"doc.insert","id":2,"params":{"doc":1,"offset":0,"text":"é!"}}"#,
        "\n",
        r#"{"command":"doc.open","id":3,"params":{"path":"sub/b.rs"}}"#,
        "\n",
        r#"{"command":"doc.text","id":4,"params":{"doc":2}}"#,
        "\n",
        r#"{"command":"doc.save","id":5,"params":{"doc":1}}"#,
        "\n",
        r#"{"command":"doc.list","id":6,"params":{}}"#,
        "\n",
    );
    assert_eq!(log1, expected);

    // The successful responses of the first run, in order.
    let ok_ids: Vec<Value> = stdout_lines(&out1)
        .iter()
        .map(|l| parse(l))
        .filter(|v| v["ok"] == Value::Bool(true))
        .map(|v| v["id"].clone())
        .collect();
    assert_eq!(
        ok_ids,
        vec![
            serde_json::json!("open"),
            serde_json::json!(2),
            Value::Null,
            serde_json::json!(11),
            serde_json::json!(12),
            serde_json::json!(13)
        ]
    );

    // Round trip: replaying the log on a fresh copy leaves the same files, all requests succeed.
    let out2 = run_script(&work, &root2, log1.as_bytes(), Some(&l2));
    assert_eq!(code(&out2), 0, "{out2:?}");
    assert_eq!(stdout_lines(&out2).len(), 6);
    assert_eq!(snapshot(&root1), snapshot(&root2));
    // Idempotence: the replay's own log is byte-identical.
    assert_eq!(std::fs::read(&l2).unwrap(), std::fs::read(&l1).unwrap());

    // The replay's results equal the original run's successful results.
    let res1: Vec<Value> = stdout_lines(&out1)
        .iter()
        .map(|l| parse(l))
        .filter(|v| v["ok"] == Value::Bool(true))
        .map(|v| v["result"].clone())
        .collect();
    let res2: Vec<Value> = stdout_lines(&out2)
        .iter()
        .map(|l| parse(l)["result"].clone())
        .collect();
    assert_eq!(res1, res2);
}

/// `--log` of a script in which nothing succeeds is an empty file (zero lines).
#[test]
fn log_of_all_failing_script_is_empty() {
    let work = fresh_dir("ac4-empty");
    let root = work.join("root");
    std::fs::create_dir(&root).unwrap();
    let log = work.join("l.jsonl");
    let out = run_script(
        &work,
        &root,
        b"{\"command\":\"no.such\"}\nnot json\n",
        Some(&log),
    );
    assert_eq!(code(&out), 3, "{out:?}");
    assert_eq!(stdout_lines(&out).len(), 2);
    assert_eq!(std::fs::read(&log).expect("--log written"), b"");
}

// ---------------------------------------------------------------- AC-5 (control.errors)

#[test]
fn run_exits_3_when_a_request_fails() {
    let work = fresh_dir("ac5");
    let root = work.join("root");
    std::fs::create_dir(&root).unwrap();
    write_files(&root, &[("a.txt", b"hello\n")]);

    // (bad line, expected code, expected id)
    let cases: Vec<(&[u8], &str, Value)> = vec![
        (b"not json", "invalid_json", Value::Null),
        (b"[1,2]", "invalid_request", Value::Null),
        (br#"{"id":7}"#, "invalid_request", serde_json::json!(7)),
        (
            br#"{"id":1.5,"command":"doc.list"}"#,
            "invalid_request",
            Value::Null,
        ),
        (
            br#"{"command":"doc.list","params":[]}"#,
            "invalid_request",
            Value::Null,
        ),
        (
            br#"{"command":"doc.list","extra":1}"#,
            "invalid_request",
            Value::Null,
        ),
        (br#"{"command":"no.such"}"#, "unknown_command", Value::Null),
        (
            br#"{"command":"doc.insert","params":{"doc":1}}"#,
            "invalid_params",
            Value::Null,
        ),
        (b"\xff\xfe\x80 invalid utf-8", "invalid_json", Value::Null),
        // Edge cases the spec's rules imply.
        (br#""just a string""#, "invalid_request", Value::Null),
        (
            br#"{"id":"s","command":5}"#,
            "invalid_request",
            serde_json::json!("s"),
        ),
        (
            br#"{"id":"p","command":"doc.list","params":null}"#,
            "invalid_request",
            serde_json::json!("p"),
        ),
        (
            br#"{"id":true,"command":"doc.list"}"#,
            "invalid_request",
            Value::Null,
        ),
        (
            br#"{"id":[1],"command":"doc.list"}"#,
            "invalid_request",
            Value::Null,
        ),
        (
            br#"{"id":"x","command":"doc.list","extra":1}"#,
            "invalid_request",
            serde_json::json!("x"),
        ),
        (
            br#"{"command":"doc.list"} trailing"#,
            "invalid_json",
            Value::Null,
        ),
        (
            br#"{"id":"r","command":"doc.insert","params":{"doc":1,"offset":99,"text":"x"}}"#,
            "rejected",
            serde_json::json!("r"),
        ),
    ];
    let mut script = Vec::new();
    script.extend_from_slice(br#"{"id":"open","command":"doc.open","params":{"path":"a.txt"}}"#);
    script.push(b'\n');
    for (i, (bad, _, _)) in cases.iter().enumerate() {
        script.extend_from_slice(bad);
        script.push(b'\n');
        script
            .extend_from_slice(format!(r#"{{"id":"after-{i}","command":"doc.list"}}"#).as_bytes());
        script.push(b'\n');
    }
    let out = run_script(&work, &root, &script, None);
    assert_eq!(code(&out), 3, "{out:?}");
    let lines = stdout_lines(&out);
    assert_eq!(lines.len(), 1 + 2 * cases.len(), "{lines:#?}");
    assert_eq!(parse(&lines[0])["ok"], Value::Bool(true), "{}", lines[0]);
    for (i, (bad, want_code, want_id)) in cases.iter().enumerate() {
        let what = String::from_utf8_lossy(bad);
        let v = parse(&lines[1 + 2 * i]);
        assert_eq!(v["ok"], Value::Bool(false), "{what}: {v}");
        assert_eq!(v["error"]["code"], *want_code, "{what}: {v}");
        assert!(v["error"]["message"].is_string(), "{what}: {v}");
        assert_eq!(v["id"], *want_id, "{what}: {v}");
        assert!(v.get("result").is_none(), "{what}: {v}");
        let next = parse(&lines[2 + 2 * i]);
        assert_eq!(next["ok"], Value::Bool(true), "after {what}: {next}");
        assert_eq!(next["id"], serde_json::json!(format!("after-{i}")));
        assert_eq!(
            next["result"],
            serde_json::json!({"docs":[{"dirty":false,"doc":1,"path":"a.txt"}]}),
            "a failed request left state unchanged"
        );
    }
}

/// The host survives hostile input: deep nesting, a huge line, NULs, a lone CR.
#[test]
fn run_survives_hostile_lines() {
    let work = fresh_dir("hostile");
    let root = work.join("root");
    std::fs::create_dir(&root).unwrap();
    let mut script = Vec::new();
    script.extend_from_slice(&"[".repeat(100_000).into_bytes());
    script.push(b'\n');
    script.extend_from_slice(
        format!(r#"{{"id":"big","command":"{}"}}"#, "x".repeat(1 << 20)).as_bytes(),
    );
    script.push(b'\n');
    script.extend_from_slice(b"\0\0\0\n");
    script.extend_from_slice(br#"{"id":"last","command":"doc.list"}"#);
    script.push(b'\n');
    let out = run_script(&work, &root, &script, None);
    assert_eq!(code(&out), 3, "{:?}", out.status);
    let lines = stdout_lines(&out);
    assert_eq!(lines.len(), 4);
    for l in &lines[..3] {
        assert_eq!(parse(l)["ok"], Value::Bool(false));
    }
    assert_eq!(lines[3], r#"{"id":"last","ok":true,"result":{"docs":[]}}"#);
}

// ---------------------------------------------------------------- AC-6 (control.protocol)

#[test]
fn control_mode_answers_each_line_before_next() {
    let work = fresh_dir("ac6");
    let root = work.join("root");
    std::fs::create_dir(&root).unwrap();
    write_files(&root, &[("a.txt", b"hello\n")]);

    let mut child = Command::new(bin())
        .args(["control", "--root", root.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn kiln-cli control");
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel::<Option<String>>();
    std::thread::spawn(move || {
        let mut r = BufReader::new(stdout);
        loop {
            let mut s = String::new();
            match r.read_line(&mut s) {
                Ok(0) | Err(_) => {
                    let _ = tx.send(None);
                    return;
                }
                Ok(_) => {
                    if tx.send(Some(s)).is_err() {
                        return;
                    }
                }
            }
        }
    });

    let exchange: Vec<(&str, Option<&str>)> = vec![
        ("not json", None),
        (
            r#"{"id":1,"command":"doc.open","params":{"path":"a.txt"}}"#,
            None,
        ),
        (r#"{"id":2,"command":"no.such"}"#, None),
        (
            r#"{"id":3,"command":"doc.insert","params":{"doc":1,"offset":5,"text":" world"}}"#,
            Some(r#"{"id":3,"ok":true,"result":{"dirty":true,"len_chars":12}}"#),
        ),
        (
            r#"{"id":5,"command":"doc.save","params":{"doc":1}}"#,
            Some(AC1_LINE5),
        ),
    ];
    let mut oks = Vec::new();
    for (req, want) in &exchange {
        stdin.write_all(req.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();
        stdin.flush().unwrap();
        // A blank line between requests must produce nothing.
        stdin.write_all(b"\n").unwrap();
        stdin.flush().unwrap();
        let got = rx
            .recv_timeout(Duration::from_secs(60))
            .unwrap_or_else(|_| panic!("no response to {req} before the next request"))
            .unwrap_or_else(|| panic!("stdout closed before answering {req}"));
        assert!(got.ends_with('\n'), "{got:?}");
        let line = got.trim_end_matches('\n');
        let v = parse(line);
        if let Some(w) = want {
            assert_eq!(line, *w);
        }
        oks.push(v["ok"].clone());
    }
    assert_eq!(
        oks,
        vec![
            Value::Bool(false),
            Value::Bool(true),
            Value::Bool(false),
            Value::Bool(true),
            Value::Bool(true)
        ]
    );
    drop(stdin); // EOF
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(60)).ok(),
        Some(None),
        "no extra output after EOF"
    );
    let status = child.wait().unwrap();
    assert_eq!(
        status.code(),
        Some(0),
        "control exits 0 at EOF even after errors"
    );
    assert_eq!(std::fs::read(root.join("a.txt")).unwrap(), b"hello world\n");
}

// ---------------------------------------------------------------- AC-7

#[test]
fn run_output_is_canonical_json() {
    let work = fresh_dir("ac7");
    let root = work.join("root");
    std::fs::create_dir(&root).unwrap();
    write_files(&root, &mixed_files());
    let mut script = mixed_script();
    script.extend_from_slice(b"\xc3\x28\n[1,2]\n{\"id\":7}\n");
    let out = run_script(&work, &root, &script, None);
    assert_eq!(stdout_lines(&out).len(), 16, "{out:?}");
    assert_canonical(&out, &[&root, &work]);
}

// ---------------------------------------------------------------- AC-8

#[test]
fn cli_usage_errors_exit_2() {
    let work = fresh_dir("ac8");
    let root = work.join("root");
    std::fs::create_dir(&root).unwrap();
    write_files(&root, &[("a.txt", b"hello\n")]);
    let script = work.join("s.jsonl");
    std::fs::write(
        &script,
        r#"{"id":1,"command":"doc.list"}"#.to_owned() + "\n",
    )
    .unwrap();
    let s = script.to_str().unwrap();
    let r = root.to_str().unwrap();
    let file_root = root.join("a.txt");
    let fr = file_root.to_str().unwrap();
    let missing_dir = work.join("does-not-exist");
    let md = missing_dir.to_str().unwrap();
    let bad_log = work.join("no-such-dir").join("l.jsonl");
    let bl = bad_log.to_str().unwrap();
    let good_log = work.join("l.jsonl");
    let gl = good_log.to_str().unwrap();
    let missing_script = work.join("missing.jsonl");
    let ms = missing_script.to_str().unwrap();

    // Positive controls: the same shapes with valid inputs succeed.
    let ok = kiln(&["run", s, "--root", r], None);
    assert_eq!(code(&ok), 0, "{ok:?}");
    assert_eq!(
        String::from_utf8_lossy(&ok.stdout),
        "{\"id\":1,\"ok\":true,\"result\":{\"docs\":[]}}\n"
    );
    let ok = kiln(&["run", s, "--root", r, "--log", gl], None);
    assert_eq!(code(&ok), 0, "{ok:?}");
    assert!(good_log.is_file());

    // (args, why)
    let errors: Vec<(Vec<&str>, &str)> = vec![
        (vec!["run", ms, "--root", r], "missing script file"),
        (vec!["run", s, "--root", fr], "--root names a file"),
        (vec!["run", s, "--root", md], "--root does not exist"),
        (vec!["run", s, "--root", r, "--log", bl], "unwritable --log"),
        (vec!["run"], "run without a script"),
        (vec!["run", s, "--root"], "--root without a value"),
        (vec!["run", s, "--bogus"], "unknown flag"),
        (vec!["frobnicate"], "unknown subcommand"),
        (vec![], "no subcommand"),
        (
            vec!["control", "--root", fr],
            "control: --root names a file",
        ),
        (
            vec!["control", "--root", md],
            "control: --root does not exist",
        ),
        (vec!["control", "--bogus"], "control: unknown flag"),
    ];
    for (args, why) in errors {
        let out = kiln(&args, None);
        assert_eq!(code(&out), 2, "{why}: {out:?}");
        assert!(out.stdout.is_empty(), "{why}: nothing on stdout: {out:?}");
        assert!(!out.stderr.is_empty(), "{why}: message on stderr");
    }
    assert!(!bad_log.exists());

    // `control` with the positive counterpart exits 0 at EOF.
    let ok = kiln(&["control", "--root", r], None);
    assert_eq!(code(&ok), 0, "{ok:?}");
    assert!(ok.stdout.is_empty());
}
