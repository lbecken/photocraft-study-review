//! T-032 AC-4 regression guard (D-010, D-021; scoped as D-043): structured reasons change
//! nothing a user of `kiln-cli` can observe. Every expected value below was recorded by running
//! the base binary (`main` with T-015 merged, before T-032) and is embedded verbatim; the test
//! passes on the base by design and fails if any byte of a response, a log script, a saved
//! file, an exit code or the `exec` stderr changes. Written from the task specification alone
//! by the test author; the harness follows `guard_t015_cli.rs`, with its own seed.
//!
//! Scripts: the worked example of `docs/control-protocol.md` with its files (through `run` and
//! `control`), the `exec` cases including Pinned (g), and a fixed-seed session of more than 300
//! requests whose paths come from `[a-z0-9/._]`, never contain `: ` and never name a directory,
//! and whose ids are at most 64 characters (D-043). `kiln-cli commands` output is deliberately
//! not compared (D-043).
//!
//! Every spawned process is killed after [`TIMEOUT`] (D-028 (8)).
#![cfg(test)]

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(60);

// ---------------------------------------------------------------------------------------------
// Process helpers.

/// A fresh, empty directory unique to `name` under `CARGO_TARGET_TMPDIR`.
fn fresh(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("guard_t032_cli")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn wait_or_kill(child: &mut Child) -> ExitStatus {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().expect("try_wait") {
            return status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("kiln-cli did not finish within {TIMEOUT:?}");
        }
        thread::sleep(Duration::from_millis(5));
    }
}

/// Run `kiln-cli args` in `cwd` with `stdin` as its input; kill it after [`TIMEOUT`].
fn kiln(args: &[&str], cwd: &Path, stdin: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn kiln-cli");
    let input = stdin.to_vec();
    let mut pipe = child.stdin.take().expect("stdin");
    let writer = thread::spawn(move || {
        let _ = pipe.write_all(&input);
    });
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
    let status = wait_or_kill(&mut child);
    writer.join().expect("stdin writer");
    Output {
        status,
        stdout: out.join().expect("stdout reader"),
        stderr: err.join().expect("stderr reader"),
    }
}

/// Every file below `dir` as `path = escaped bytes` lines, sorted by path.
fn snapshot(dir: &Path) -> String {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                out.push((
                    format!("{}/", p.strip_prefix(base).unwrap().to_string_lossy()),
                    Vec::new(),
                ));
                walk(base, &p, out);
            } else {
                let rel = p.strip_prefix(base).unwrap().to_string_lossy().into_owned();
                out.push((rel, fs::read(&p).unwrap()));
            }
        }
    }
    let mut files = Vec::new();
    walk(dir, dir, &mut files);
    files.sort();
    files
        .iter()
        .map(|(p, b)| format!("{p} = {}\n", b.escape_ascii()))
        .collect()
}

fn write_files(root: &Path, files: &[(&str, &[u8])]) {
    for (path, bytes) in files {
        let p = root.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, bytes).unwrap();
    }
}

/// Compare `actual` with the recorded base value; on a difference, save `actual` next to the
/// test's scratch files and name the first differing line.
fn same(what: &str, actual: &str, expected: &str) {
    if actual != expected {
        let dump = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("guard_t032_cli")
            .join(format!("{what}.actual"));
        let _ = fs::write(&dump, actual);
        let (a, e): (Vec<&str>, Vec<&str>) = (actual.lines().collect(), expected.lines().collect());
        let i = (0..a.len().max(e.len()))
            .find(|&i| a.get(i) != e.get(i))
            .unwrap_or(0);
        panic!(
            "{what} differs from the base recording (actual saved to {}):\n\
             first difference at line {}:\n  base:   {:?}\n  actual: {:?}",
            dump.display(),
            i + 1,
            e.get(i),
            a.get(i)
        );
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("UTF-8 output")
}

// ---------------------------------------------------------------------------------------------
// The worked example of docs/control-protocol.md.

const WORKED_SCRIPT: &str = r#"{"id":1,"command":"doc.open","params":{"path":"a.txt"}}
{"id":2,"command":"doc.insert","params":{"doc":1,"offset":5,"text":" world"}}
{"id":3,"command":"doc.undo","params":{"doc":1}}
{"id":4,"command":"doc.redo","params":{"doc":1}}
{"id":"oops","command":"doc.insert","params":{"doc":1}}
{"id":5,"command":"doc.save","params":{"doc":1}}
"#;

const WORKED_STDOUT: &str = r#"{"id":1,"ok":true,"result":{"already_open":false,"doc":1,"encoding":"utf-8","len_chars":6,"line_ending":"lf","mixed_line_endings":false,"path":"a.txt"}}
{"id":2,"ok":true,"result":{"dirty":true,"len_chars":12}}
{"id":3,"ok":true,"result":{"dirty":false,"len_chars":6,"undone":true}}
{"id":4,"ok":true,"result":{"dirty":true,"len_chars":12,"redone":true}}
{"error":{"code":"invalid_params","details":["\"offset\" is a required property","\"text\" is a required property"],"kind":"schema","message":"invalid params for `doc.insert`"},"id":"oops","ok":false}
{"id":5,"ok":true,"result":{"bytes_written":12,"dirty":false,"doc":1}}
"#;

const WORKED_LOG: &str = r#"{"command":"doc.open","id":1,"params":{"path":"a.txt"}}
{"command":"doc.insert","id":2,"params":{"doc":1,"offset":5,"text":" world"}}
{"command":"doc.undo","id":3,"params":{"doc":1}}
{"command":"doc.redo","id":4,"params":{"doc":1}}
{"command":"doc.save","id":5,"params":{"doc":1}}
"#;

const WORKED_CONTROL_IN: &str = r#"{"id":1,"command":"doc.open","params":{"path":"a.txt"}}
{"id":2,"command":"doc.text","params":{"doc":1}}
"#;

const WORKED_CONTROL_OUT: &str = r#"{"id":1,"ok":true,"result":{"already_open":false,"doc":1,"encoding":"utf-8","len_chars":12,"line_ending":"lf","mixed_line_endings":false,"path":"a.txt"}}
{"id":2,"ok":true,"result":{"len_chars":12,"text":"hello world\n"}}
"#;

fn worked_example() {
    let work = fresh("worked");
    let proj = work.join("proj");
    write_files(&proj, &[("a.txt", b"hello\n")]);
    fs::write(work.join("s.jsonl"), WORKED_SCRIPT).unwrap();

    let out = kiln(
        &["run", "s.jsonl", "--root", "proj", "--log", "l.jsonl"],
        &work,
        b"",
    );
    assert_eq!(out.status.code(), Some(3), "worked example: {out:?}");
    same("worked_stdout", &text(&out.stdout), WORKED_STDOUT);
    same("worked_stderr", &text(&out.stderr), "");
    same(
        "worked_log",
        &text(&fs::read(work.join("l.jsonl")).unwrap()),
        WORKED_LOG,
    );
    same("worked_files", &snapshot(&proj), "a.txt = hello world\\n\n");

    let out = kiln(
        &["control", "--root", "proj"],
        &work,
        WORKED_CONTROL_IN.as_bytes(),
    );
    assert_eq!(out.status.code(), Some(0), "worked control: {out:?}");
    same(
        "worked_control_stdout",
        &text(&out.stdout),
        WORKED_CONTROL_OUT,
    );
    same("worked_control_stderr", &text(&out.stderr), "");
    same(
        "worked_files_after_control",
        &snapshot(&proj),
        "a.txt = hello world\\n\n",
    );
}

// ---------------------------------------------------------------------------------------------
// `exec` (Pinned (g) and a few more one-shot cases).

/// (args, exit code, stdout, stderr), recorded on base.
const EXEC_CASES: &[(&[&str], i32, &str, &str)] = &[
    (
        &["exec", "doc.open", r#"{"path":"x"}"#, "--memory"],
        2,
        "",
        "{\"code\":\"rejected\",\"kind\":\"not_found\",\"message\":\"cannot open x: not found\"}\n",
    ),
    (
        &["exec", "doc.open", r#"{"path":"a.txt"}"#, "--root", "proj"],
        0,
        "{\"already_open\":false,\"doc\":1,\"encoding\":\"utf-8\",\"len_chars\":6,\"line_ending\":\"lf\",\"mixed_line_endings\":false,\"path\":\"a.txt\"}\n",
        "",
    ),
    (
        &["exec", "doc.text", r#"{"doc":1}"#, "--root", "proj"],
        2,
        "",
        "{\"code\":\"rejected\",\"kind\":\"no_such_document\",\"message\":\"no open document 1\"}\n",
    ),
    (
        &["exec", "doc.open", r#"{"path":"../x"}"#, "--root", "proj"],
        2,
        "",
        "{\"code\":\"rejected\",\"kind\":\"invalid_path\",\"message\":\"invalid relative path \\\"../x\\\": path has a '..' segment\"}\n",
    ),
    (
        &[
            "exec",
            "buffer.insert",
            r#"{"offset":3,"text":"!"}"#,
            "--text",
            "abc",
            "--memory",
        ],
        0,
        "{\"len_chars\":4}\n",
        "",
    ),
    (
        &[
            "exec",
            "buffer.insert",
            r#"{"offset":9,"text":"!"}"#,
            "--text",
            "abc",
            "--memory",
        ],
        2,
        "",
        "{\"code\":\"rejected\",\"kind\":\"out_of_range\",\"message\":\"char index 9 is out of bounds (buffer has 3 chars)\"}\n",
    ),
    (
        &[
            "exec",
            "buffer.text",
            "{}",
            "--text",
            "é😀\r\n",
            "--root",
            "proj",
        ],
        0,
        "{\"len_chars\":4,\"text\":\"é😀\\r\\n\"}\n",
        "",
    ),
    (
        &["exec", "zz.no-such-command", "{}", "--memory"],
        2,
        "",
        "{\"code\":\"unknown_command\",\"kind\":\"unknown_command\",\"message\":\"unknown command `zz.no-such-command`\"}\n",
    ),
    (
        &["exec", "doc.list", "{", "--memory"],
        2,
        "",
        "{\"code\":\"invalid_json\",\"kind\":\"syntax\",\"message\":\"params are not JSON: EOF while parsing an object at line 1 column 1\"}\n",
    ),
    (
        &[
            "exec",
            "doc.open",
            r#"{"path":"bad.bin"}"#,
            "--root",
            "proj",
        ],
        2,
        "",
        "{\"code\":\"rejected\",\"kind\":\"decode_error\",\"message\":\"cannot open bad.bin: invalid utf-8 at byte offset 2\"}\n",
    ),
    (
        &[
            "exec",
            "doc.open",
            r#"{"path":"sub/none.txt"}"#,
            "--root",
            "proj",
        ],
        2,
        "",
        "{\"code\":\"rejected\",\"kind\":\"not_found\",\"message\":\"cannot open sub/none.txt: not found\"}\n",
    ),
    (
        &["exec", "doc.close", r#"{"doc":3}"#, "--memory"],
        2,
        "",
        "{\"code\":\"rejected\",\"kind\":\"no_such_document\",\"message\":\"no open document 3\"}\n",
    ),
];

fn exec_cases() {
    let work = fresh("exec");
    let proj = work.join("proj");
    write_files(
        &proj,
        &[
            ("a.txt", b"hello\n"),
            ("bad.bin", b"ok\xff\n"),
            ("sub/c.rs", b""),
        ],
    );
    let before = snapshot(&proj);
    for (i, (args, code, stdout, stderr)) in EXEC_CASES.iter().enumerate() {
        let out = kiln(args, &work, b"");
        assert_eq!(out.status.code(), Some(*code), "exec {args:?}: {out:?}");
        same(&format!("exec_{i}_stdout"), &text(&out.stdout), stdout);
        same(&format!("exec_{i}_stderr"), &text(&out.stderr), stderr);
    }
    same("exec_files", &snapshot(&proj), &before);
}

// ---------------------------------------------------------------------------------------------
// The fixed-seed session.

/// Deterministic generator (64-bit LCG, Knuth's MMIX constants), fixed seed.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len() as u64) as usize]
    }
}

/// Files of the session root. No path in the session names a directory (D-043).
const SESSION_FILES: &[(&str, &[u8])] = &[
    ("a.txt", b"hello\n"),
    ("b.txt", b"one\r\ntwo\r\n"),
    ("m.txt", b"x\ny\r\nz\rw"),
    ("empty.txt", b""),
    ("u16.txt", b"\xff\xfeh\x00i\x00\n\x00"),
    ("u16be.txt", b"\xfe\xff\x00h\x00i\x00\n"),
    ("bom.txt", b"\xef\xbb\xbfbom\n"),
    ("bad.bin", b"ok\x80\x81\n"),
    ("sub/c.rs", b"fn main() {}\n"),
    ("sub/deep/d.txt", "é😀\r\n".as_bytes()),
];

/// Request paths: only `[a-z0-9/._]`, never `: `, never a directory (D-043). Existing files,
/// missing files in existing directories, and lexically invalid paths.
const PATHS: &[&str] = &[
    "a.txt",
    "a.txt",
    "b.txt",
    "m.txt",
    "empty.txt",
    "u16.txt",
    "u16be.txt",
    "bom.txt",
    "bad.bin",
    "sub/c.rs",
    "sub/c.rs",
    "sub/deep/d.txt",
    "missing.txt",
    "sub/missing.rs",
    "a//b.txt",
    "/abs.txt",
    "../up.txt",
    "./a.txt",
    "sub/../a.txt",
];

/// The paths of [`PATHS`] that open successfully (for the generator's model).
const READABLE: &[&str] = &[
    "a.txt",
    "b.txt",
    "m.txt",
    "empty.txt",
    "u16.txt",
    "u16be.txt",
    "bom.txt",
    "sub/c.rs",
    "sub/deep/d.txt",
];

const TEXTS: &[&str] = &[
    "x",
    "ab",
    " ",
    "\n",
    "\r\n",
    "\r",
    "é",
    "😀",
    "fn f() {}\n",
    "tab\t",
    "q",
];

/// Requests that are malformed at the protocol level (one line each; ids ≤ 64 chars).
const MALFORMED: &[&str] = &[
    "not json",
    "{",
    "[1,2]",
    "\"str\"",
    r#"{"id":1.5,"command":"doc.list"}"#,
    r#"{"id":true,"command":"doc.list"}"#,
    r#"{"id":"k","command":"doc.list","extra":1}"#,
    r#"{"id":"m"}"#,
    r#"{"id":"n","command":7}"#,
    r#"{"id":"p","command":"doc.list","params":[1]}"#,
    r#"{"id":"u","command":"zz.no-such-command","params":{}}"#,
    r#"{"id":"v","command":"zz.no-such-command","params":{"doc":1}}"#,
    r#"{"id":"w","command":"doc.insert","params":{"doc":1,"offset":-1,"text":"x"}}"#,
    r#"{"id":"y","command":"doc.close","params":{"doc":1,"discard":"yes"}}"#,
    r#"{"id":"z","command":"doc.open","params":{"path":"a.txt","x":1}}"#,
    r#"{"id":"0123456789012345678901234567890123456789012345678901234567890123","command":"doc.list"}"#,
    r#"{"id":-7,"command":"buffer.text"}"#,
    "",
    "   ",
];

fn session_script() -> String {
    let mut rng = Lcg(0x7032_2026_1113);
    let mut lines = Vec::new();
    let mut requests = 0;
    let mut n = 0u64;
    // A rough model of the open documents (id, path), so that most requests name one: ids are
    // predicted as the server assigns them (1, 2, …); it may drift, which only adds refusals.
    let mut open: Vec<(u64, &str)> = Vec::new();
    let mut next_id = 1;
    while requests < 320 {
        n += 1;
        let doc = match rng.below(10) {
            0 => 0,
            1 => next_id + 3,
            _ if open.is_empty() => 1,
            _ => open[rng.below(open.len() as u64) as usize].0,
        };
        let off = rng.below(16);
        let (a, b) = (rng.below(14), rng.below(14));
        let txt = serde_json::to_string(rng.pick(TEXTS)).unwrap();
        let path = rng.pick(PATHS);
        let id = if rng.below(8) == 0 {
            format!("\"r{n}\"")
        } else {
            n.to_string()
        };
        let body = match rng.below(24) {
            0..=3 => {
                if READABLE.contains(&path) && !open.iter().any(|(_, p)| *p == path) {
                    open.push((next_id, path));
                    next_id += 1;
                }
                format!(r#""command":"doc.open","params":{{"path":"{path}"}}"#)
            }
            5..=7 => format!(
                r#""command":"doc.insert","params":{{"doc":{doc},"offset":{off},"text":{txt}}}"#
            ),
            8 => {
                format!(r#""command":"doc.delete","params":{{"doc":{doc},"start":{a},"end":{b}}}"#)
            }
            9 => format!(
                r#""command":"doc.replace","params":{{"doc":{doc},"start":{a},"end":{b},"text":{txt}}}"#
            ),
            10 => format!(r#""command":"doc.undo","params":{{"doc":{doc}}}"#),
            11 => format!(r#""command":"doc.redo","params":{{"doc":{doc}}}"#),
            12 => format!(r#""command":"doc.info","params":{{"doc":{doc}}}"#),
            13 => format!(r#""command":"doc.text","params":{{"doc":{doc}}}"#),
            4 | 14 | 15 => format!(r#""command":"doc.save","params":{{"doc":{doc}}}"#),
            16 => format!(r#""command":"doc.close","params":{{"doc":{doc}}}"#),
            17 => {
                open.retain(|(id, _)| *id != doc);
                format!(r#""command":"doc.close","params":{{"doc":{doc},"discard":true}}"#)
            }
            18 => r#""command":"doc.list""#.to_owned(),
            19 => format!(r#""command":"buffer.insert","params":{{"offset":{off},"text":{txt}}}"#),
            20 => r#""command":"buffer.text","params":{}"#.to_owned(),
            _ => {
                let line = rng.pick(MALFORMED);
                if !line.trim().is_empty() {
                    requests += 1;
                }
                lines.push(line.to_owned());
                continue;
            }
        };
        requests += 1;
        lines.push(format!("{{\"id\":{id},{body}}}"));
    }
    let mut script = lines.join("\n");
    script.push('\n');
    script
}

#[test]
fn session_script_meets_the_d043_constraints() {
    let script = session_script();
    let mut requests = 0;
    for line in script.lines() {
        if line.trim().is_empty() {
            continue;
        }
        requests += 1;
        assert!(!line.contains(": "), "{line}");
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if let Some(id) = v.get("id") {
            assert!(
                id.to_string().trim_matches('"').chars().count() <= 64,
                "{line}"
            );
        }
        if let Some(p) = v.pointer("/params/path").and_then(|p| p.as_str()) {
            assert!(
                p.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "/._".contains(c)),
                "{p}"
            );
            let target = Path::new("x").join(p.trim_start_matches('/'));
            assert!(!p.ends_with('/') && p != "." && p != "..", "{p}");
            assert!(
                !SESSION_FILES.iter().any(|(f, _)| {
                    Path::new(f)
                        .parent()
                        .is_some_and(|d| d != Path::new("") && Path::new("x").join(d) == target)
                }),
                "{p} names a directory"
            );
        }
    }
    assert!(requests >= 300, "{requests} requests");
}

fn session(mode: &str) {
    let work = fresh(&format!("session-{mode}"));
    let root = work.join("root");
    write_files(&root, SESSION_FILES);
    let script = session_script();
    fs::write(work.join("s.jsonl"), &script).unwrap();
    let out = match mode {
        "run" => kiln(
            &["run", "s.jsonl", "--root", "root", "--log", "l.jsonl"],
            &work,
            b"",
        ),
        _ => kiln(&["control", "--root", "root"], &work, script.as_bytes()),
    };
    let expected_code = if mode == "run" { 3 } else { 0 };
    assert_eq!(
        out.status.code(),
        Some(expected_code),
        "session {mode}: {:?}",
        out.status
    );
    same(&format!("session_{mode}_stderr"), &text(&out.stderr), "");
    same(
        &format!("session_{mode}_stdout"),
        &text(&out.stdout),
        SESSION_STDOUT,
    );
    same(
        &format!("session_{mode}_files"),
        &snapshot(&root),
        SESSION_FILES_AFTER,
    );
    if mode == "run" {
        let log = text(&fs::read(work.join("l.jsonl")).unwrap());
        same("session_log", &log, SESSION_LOG);
        // The log replays on a fresh copy of the initial files to the same files, and its own
        // log is byte-identical to it.
        let replay = fresh("session-replay");
        let root2 = replay.join("root");
        write_files(&root2, SESSION_FILES);
        fs::write(replay.join("s.jsonl"), &log).unwrap();
        let out = kiln(
            &["run", "s.jsonl", "--root", "root", "--log", "l.jsonl"],
            &replay,
            b"",
        );
        assert_eq!(out.status.code(), Some(0), "replay: {out:?}");
        same(
            "session_replay_files",
            &snapshot(&root2),
            SESSION_FILES_AFTER,
        );
        same(
            "session_replay_log",
            &text(&fs::read(replay.join("l.jsonl")).unwrap()),
            SESSION_LOG,
        );
    }
}

/// AC-4: the worked example, the `exec` cases and the fixed-seed session (through `run` and
/// through `control`) print exactly what the base printed and leave exactly the base's files.
#[test]
fn protocol_bytes_are_unchanged() {
    worked_example();
    exec_cases();
    session("run");
    session("control");
}

// ---------------------------------------------------------------------------------------------
// Recorded on base (main with T-015 merged, before T-032) with this file's own generator.

const SESSION_STDOUT: &str = r#"{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 1"},"id":1,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 1"},"id":2,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 1"},"id":"r3","ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 1"},"id":"r4","ok":false}
{"error":{"code":"invalid_request","kind":"unknown_key","message":"unknown top-level key(s) `extra`; allowed: `command`, `id`, `params`"},"id":"k","ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 1"},"id":6,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 1"},"id":7,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 0"},"id":8,"ok":false}
{"id":9,"ok":true,"result":{"already_open":false,"doc":1,"encoding":"utf-8","len_chars":4,"line_ending":"crlf","mixed_line_endings":false,"path":"sub/deep/d.txt"}}
{"id":10,"ok":true,"result":{"already_open":false,"doc":2,"encoding":"utf-8","len_chars":13,"line_ending":"lf","mixed_line_endings":false,"path":"sub/c.rs"}}
{"id":11,"ok":true,"result":{"len_chars":4,"text":"é😀\r\n"}}
{"id":12,"ok":true,"result":{"dirty":true,"len_chars":17}}
{"id":13,"ok":true,"result":{"docs":[{"dirty":false,"doc":1,"path":"sub/deep/d.txt"},{"dirty":true,"doc":2,"path":"sub/c.rs"}]}}
{"id":14,"ok":true,"result":{"already_open":false,"doc":3,"encoding":"utf-8","len_chars":6,"line_ending":"lf","mixed_line_endings":false,"path":"a.txt"}}
{"id":15,"ok":true,"result":{"dirty":false,"len_chars":6,"undone":false}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 0"},"id":16,"ok":false}
{"id":17,"ok":true,"result":{"dirty":false,"len_chars":6,"undone":false}}
{"error":{"code":"invalid_params","details":["/offset: -1 is less than the minimum of 0"],"kind":"schema","message":"invalid params for `doc.insert`"},"id":"w","ok":false}
{"id":19,"ok":true,"result":{"dirty":true,"len_chars":7}}
{"id":20,"ok":true,"result":{"docs":[{"dirty":false,"doc":1,"path":"sub/deep/d.txt"},{"dirty":true,"doc":2,"path":"sub/c.rs"},{"dirty":true,"doc":3,"path":"a.txt"}]}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 0"},"id":21,"ok":false}
{"id":22,"ok":true,"result":{"dirty":true,"len_chars":17,"removed":""}}
{"id":23,"ok":true,"result":{"already_open":false,"doc":4,"encoding":"utf-16le","len_chars":3,"line_ending":"lf","mixed_line_endings":false,"path":"u16.txt"}}
{"id":24,"ok":true,"result":{"already_open":true,"doc":3,"encoding":"utf-8","len_chars":7,"line_ending":"lf","mixed_line_endings":true,"path":"a.txt"}}
{"error":{"code":"invalid_request","kind":"params_not_object","message":"`params` must be an object"},"id":"p","ok":false}
{"error":{"code":"invalid_json","kind":"syntax","message":"request is not JSON: expected ident at line 1 column 2"},"id":null,"ok":false}
{"id":27,"ok":true,"result":{"already_open":false,"doc":5,"encoding":"utf-8","len_chars":0,"line_ending":"none","mixed_line_endings":false,"path":"empty.txt"}}
{"id":28,"ok":true,"result":{"bytes_written":0,"dirty":false,"doc":5}}
{"id":"r29","ok":true,"result":{"len_chars":0,"text":""}}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 6 is out of bounds (buffer has 0 chars)"},"id":30,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 9"},"id":"r31","ok":false}
{"error":{"code":"invalid_request","kind":"not_an_object","message":"request must be a JSON object"},"id":null,"ok":false}
{"error":{"code":"rejected","kind":"invalid_path","message":"invalid relative path \"../up.txt\": path has a '..' segment"},"id":33,"ok":false}
{"error":{"code":"rejected","kind":"decode_error","message":"cannot open bad.bin: invalid utf-8 at byte offset 2"},"id":"r34","ok":false}
{"id":35,"ok":true,"result":{"len_chars":0,"text":""}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 9"},"id":36,"ok":false}
{"id":37,"ok":true,"result":{"dirty":true,"len_chars":17,"redone":false}}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 3 is out of bounds (buffer has 0 chars)"},"id":38,"ok":false}
{"id":39,"ok":true,"result":{"len_chars":0,"text":""}}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 2 is out of bounds (buffer has 0 chars)"},"id":40,"ok":false}
{"error":{"code":"invalid_request","kind":"bad_id","message":"`id` must be a string or an integer"},"id":null,"ok":false}
{"id":42,"ok":true,"result":{"bytes_written":8,"dirty":false,"doc":4}}
{"error":{"code":"invalid_request","kind":"unknown_key","message":"unknown top-level key(s) `extra`; allowed: `command`, `id`, `params`"},"id":"k","ok":false}
{"id":44,"ok":true,"result":{"already_open":false,"doc":6,"encoding":"utf-8-bom","len_chars":4,"line_ending":"lf","mixed_line_endings":false,"path":"bom.txt"}}
{"id":45,"ok":true,"result":{"closed":true,"doc":3}}
{"error":{"code":"rejected","kind":"decode_error","message":"cannot open bad.bin: invalid utf-8 at byte offset 2"},"id":46,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 10"},"id":47,"ok":false}
{"id":48,"ok":true,"result":{"docs":[{"dirty":false,"doc":1,"path":"sub/deep/d.txt"},{"dirty":true,"doc":2,"path":"sub/c.rs"},{"dirty":false,"doc":4,"path":"u16.txt"},{"dirty":false,"doc":5,"path":"empty.txt"},{"dirty":false,"doc":6,"path":"bom.txt"}]}}
{"id":49,"ok":true,"result":{"already_open":false,"doc":7,"encoding":"utf-8","len_chars":6,"line_ending":"lf","mixed_line_endings":false,"path":"a.txt"}}
{"id":50,"ok":true,"result":{"docs":[{"dirty":false,"doc":1,"path":"sub/deep/d.txt"},{"dirty":true,"doc":2,"path":"sub/c.rs"},{"dirty":false,"doc":4,"path":"u16.txt"},{"dirty":false,"doc":5,"path":"empty.txt"},{"dirty":false,"doc":6,"path":"bom.txt"},{"dirty":false,"doc":7,"path":"a.txt"}]}}
{"id":51,"ok":true,"result":{"dirty":false,"len_chars":4,"redone":false}}
{"error":{"code":"rejected","kind":"invalid_path","message":"invalid relative path \"/abs.txt\": path starts with '/'"},"id":52,"ok":false}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 4 is out of bounds (buffer has 0 chars)"},"id":"r53","ok":false}
{"id":54,"ok":true,"result":{"len_chars":17,"text":"fn matab\tin() {}\n"}}
{"id":55,"ok":true,"result":{"bytes_written":6,"dirty":false,"doc":7}}
{"error":{"code":"unknown_command","kind":"unknown_command","message":"unknown command `zz.no-such-command`"},"id":"u","ok":false}
{"error":{"code":"rejected","kind":"invalid_path","message":"invalid relative path \"./a.txt\": path has a '.' segment"},"id":57,"ok":false}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 14 is out of bounds (buffer has 0 chars)"},"id":58,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 0"},"id":59,"ok":false}
{"id":60,"ok":true,"result":{"already_open":true,"doc":1,"encoding":"utf-8","len_chars":4,"line_ending":"crlf","mixed_line_endings":false,"path":"sub/deep/d.txt"}}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 13 is out of bounds (buffer has 0 chars)"},"id":61,"ok":false}
{"id":"r62","ok":true,"result":{"closed":true,"doc":4}}
{"id":63,"ok":true,"result":{"len_chars":0,"text":""}}
{"id":64,"ok":true,"result":{"len_chars":0,"text":""}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 11"},"id":65,"ok":false}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 13 is out of bounds (buffer has 0 chars)"},"id":66,"ok":false}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 8 is out of bounds (buffer has 0 chars)"},"id":67,"ok":false}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 11 is out of bounds (buffer has 4 chars)"},"id":68,"ok":false}
{"id":69,"ok":true,"result":{"dirty":true,"len_chars":18}}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 14 is out of bounds (buffer has 4 chars)"},"id":70,"ok":false}
{"id":71,"ok":true,"result":{"docs":[{"dirty":false,"doc":1,"path":"sub/deep/d.txt"},{"dirty":true,"doc":2,"path":"sub/c.rs"},{"dirty":false,"doc":5,"path":"empty.txt"},{"dirty":false,"doc":6,"path":"bom.txt"},{"dirty":false,"doc":7,"path":"a.txt"}]}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 0"},"id":72,"ok":false}
{"id":"r73","ok":true,"result":{"already_open":false,"doc":8,"encoding":"utf-8","len_chars":10,"line_ending":"crlf","mixed_line_endings":false,"path":"b.txt"}}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 6 is out of bounds (buffer has 0 chars)"},"id":"r74","ok":false}
{"error":{"code":"invalid_json","kind":"syntax","message":"request is not JSON: expected ident at line 1 column 2"},"id":null,"ok":false}
{"error":{"code":"rejected","kind":"inverted_range","message":"range start 1 is after range end 0"},"id":76,"ok":false}
{"id":77,"ok":true,"result":{"dirty":true,"len_chars":20}}
{"error":{"code":"invalid_json","kind":"syntax","message":"request is not JSON: expected ident at line 1 column 2"},"id":null,"ok":false}
{"id":79,"ok":true,"result":{"closed":true,"doc":8}}
{"id":80,"ok":true,"result":{"already_open":false,"doc":9,"encoding":"utf-8","len_chars":8,"line_ending":"lf","mixed_line_endings":true,"path":"m.txt"}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 0"},"id":81,"ok":false}
{"id":82,"ok":true,"result":{"dirty":true,"len_chars":9,"removed":""}}
{"id":83,"ok":true,"result":{"len_chars":4,"text":"é😀\r\n"}}
{"id":84,"ok":true,"result":{"bytes_written":8,"dirty":false,"doc":1}}
{"id":85,"ok":true,"result":{"already_open":true,"doc":2,"encoding":"utf-8","len_chars":18,"line_ending":"lf","mixed_line_endings":false,"path":"sub/c.rs"}}
{"error":{"code":"rejected","kind":"not_found","message":"cannot open sub/missing.rs: not found"},"id":"r86","ok":false}
{"error":{"code":"rejected","kind":"not_found","message":"cannot open sub/missing.rs: not found"},"id":"r87","ok":false}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 12 is out of bounds (buffer has 4 chars)"},"id":88,"ok":false}
{"id":89,"ok":true,"result":{"len_chars":1}}
{"error":{"code":"invalid_request","kind":"not_an_object","message":"request must be a JSON object"},"id":null,"ok":false}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 11 is out of bounds (buffer has 1 chars)"},"id":91,"ok":false}
{"id":92,"ok":true,"result":{"closed":true,"doc":5}}
{"id":93,"ok":true,"result":{"bytes_written":8,"dirty":false,"doc":1}}
{"id":"r94","ok":true,"result":{"already_open":false,"doc":10,"encoding":"utf-16le","len_chars":3,"line_ending":"lf","mixed_line_endings":false,"path":"u16.txt"}}
{"error":{"code":"rejected","kind":"invalid_path","message":"invalid relative path \"./a.txt\": path has a '.' segment"},"id":95,"ok":false}
{"id":96,"ok":true,"result":{"len_chars":1,"text":"\r"}}
{"id":97,"ok":true,"result":{"already_open":true,"doc":9,"encoding":"utf-8","len_chars":9,"line_ending":"lf","mixed_line_endings":true,"path":"m.txt"}}
{"id":98,"ok":true,"result":{"already_open":true,"doc":7,"encoding":"utf-8","len_chars":6,"line_ending":"lf","mixed_line_endings":false,"path":"a.txt"}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 4"},"id":99,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 0"},"id":100,"ok":false}
{"id":101,"ok":true,"result":{"already_open":true,"doc":9,"encoding":"utf-8","len_chars":9,"line_ending":"lf","mixed_line_endings":true,"path":"m.txt"}}
{"id":102,"ok":true,"result":{"dirty":false,"len_chars":8,"undone":true}}
{"error":{"code":"invalid_params","details":["/discard: \"yes\" is not of type \"boolean\""],"kind":"schema","message":"invalid params for `doc.close`"},"id":"y","ok":false}
{"id":104,"ok":true,"result":{"len_chars":8,"text":"x\ny\r\nz\rw"}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 4"},"id":105,"ok":false}
{"id":106,"ok":true,"result":{"docs":[{"dirty":false,"doc":1,"path":"sub/deep/d.txt"},{"dirty":true,"doc":2,"path":"sub/c.rs"},{"dirty":false,"doc":6,"path":"bom.txt"},{"dirty":false,"doc":7,"path":"a.txt"},{"dirty":false,"doc":9,"path":"m.txt"},{"dirty":false,"doc":10,"path":"u16.txt"}]}}
{"error":{"code":"invalid_request","kind":"unknown_key","message":"unknown top-level key(s) `extra`; allowed: `command`, `id`, `params`"},"id":"k","ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 0"},"id":108,"ok":false}
{"error":{"code":"invalid_params","details":["/offset: -1 is less than the minimum of 0"],"kind":"schema","message":"invalid params for `doc.insert`"},"id":"w","ok":false}
{"id":110,"ok":true,"result":{"closed":true,"doc":1}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 4"},"id":111,"ok":false}
{"id":112,"ok":true,"result":{"dirty":true,"len_chars":12}}
{"error":{"code":"invalid_params","details":["/discard: \"yes\" is not of type \"boolean\""],"kind":"schema","message":"invalid params for `doc.close`"},"id":"y","ok":false}
{"error":{"code":"invalid_params","details":["/offset: -1 is less than the minimum of 0"],"kind":"schema","message":"invalid params for `doc.insert`"},"id":"w","ok":false}
{"error":{"code":"rejected","kind":"not_found","message":"cannot open missing.txt: not found"},"id":115,"ok":false}
{"error":{"code":"rejected","kind":"unsaved_changes","message":"document 9 has unsaved changes; save it or close with discard"},"id":116,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 13"},"id":"r117","ok":false}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 14 is out of bounds (buffer has 4 chars)"},"id":118,"ok":false}
{"id":119,"ok":true,"result":{"already_open":false,"doc":11,"encoding":"utf-8","len_chars":0,"line_ending":"none","mixed_line_endings":false,"path":"empty.txt"}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 4"},"id":120,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 4"},"id":121,"ok":false}
{"id":123,"ok":true,"result":{"already_open":false,"doc":12,"encoding":"utf-8","len_chars":10,"line_ending":"crlf","mixed_line_endings":false,"path":"b.txt"}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 4"},"id":124,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 4"},"id":125,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 15"},"id":"r126","ok":false}
{"id":127,"ok":true,"result":{"already_open":true,"doc":10,"encoding":"utf-16le","len_chars":3,"line_ending":"lf","mixed_line_endings":false,"path":"u16.txt"}}
{"error":{"code":"rejected","kind":"invalid_path","message":"invalid relative path \"sub/../a.txt\": path has a '..' segment"},"id":"r128","ok":false}
{"id":129,"ok":true,"result":{"already_open":true,"doc":9,"encoding":"utf-8","len_chars":12,"line_ending":"lf","mixed_line_endings":true,"path":"m.txt"}}
{"id":130,"ok":true,"result":{"docs":[{"dirty":true,"doc":2,"path":"sub/c.rs"},{"dirty":false,"doc":6,"path":"bom.txt"},{"dirty":false,"doc":7,"path":"a.txt"},{"dirty":true,"doc":9,"path":"m.txt"},{"dirty":false,"doc":10,"path":"u16.txt"},{"dirty":false,"doc":11,"path":"empty.txt"},{"dirty":false,"doc":12,"path":"b.txt"}]}}
{"error":{"code":"rejected","kind":"not_found","message":"cannot open missing.txt: not found"},"id":131,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 15"},"id":132,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 4"},"id":133,"ok":false}
{"id":134,"ok":true,"result":{"bytes_written":18,"dirty":false,"doc":2}}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 10 is out of bounds (buffer has 0 chars)"},"id":135,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 0"},"id":136,"ok":false}
{"error":{"code":"rejected","kind":"inverted_range","message":"range start 9 is after range end 4"},"id":137,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 4"},"id":138,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 4"},"id":"r139","ok":false}
{"error":{"code":"rejected","kind":"invalid_path","message":"invalid relative path \"sub/../a.txt\": path has a '..' segment"},"id":140,"ok":false}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 7 is out of bounds (buffer has 1 chars)"},"id":141,"ok":false}
{"id":"r142","ok":true,"result":{"len_chars":1,"text":"\r"}}
{"error":{"code":"rejected","kind":"inverted_range","message":"range start 12 is after range end 5"},"id":143,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 0"},"id":144,"ok":false}
{"error":{"code":"rejected","kind":"invalid_path","message":"invalid relative path \"sub/../a.txt\": path has a '..' segment"},"id":145,"ok":false}
{"id":146,"ok":true,"result":{"dirty":false,"len_chars":4,"undone":false}}
{"id":147,"ok":true,"result":{"bytes_written":7,"dirty":false,"doc":6}}
{"id":148,"ok":true,"result":{"len_chars":3,"text":"hi\n"}}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 12 is out of bounds (buffer has 1 chars)"},"id":149,"ok":false}
{"id":150,"ok":true,"result":{"bytes_written":18,"dirty":false,"doc":2}}
{"id":151,"ok":true,"result":{"docs":[{"dirty":false,"doc":2,"path":"sub/c.rs"},{"dirty":false,"doc":6,"path":"bom.txt"},{"dirty":false,"doc":7,"path":"a.txt"},{"dirty":true,"doc":9,"path":"m.txt"},{"dirty":false,"doc":10,"path":"u16.txt"},{"dirty":false,"doc":11,"path":"empty.txt"},{"dirty":false,"doc":12,"path":"b.txt"}]}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 0"},"id":152,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 4"},"id":153,"ok":false}
{"id":154,"ok":true,"result":{"closed":true,"doc":2}}
{"error":{"code":"invalid_json","kind":"syntax","message":"request is not JSON: expected ident at line 1 column 2"},"id":null,"ok":false}
{"id":156,"ok":true,"result":{"bytes_written":7,"dirty":false,"doc":6}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 15"},"id":157,"ok":false}
{"id":158,"ok":true,"result":{"already_open":false,"doc":13,"encoding":"utf-8","len_chars":18,"line_ending":"lf","mixed_line_endings":false,"path":"sub/c.rs"}}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 12 is out of bounds (buffer has 6 chars)"},"id":159,"ok":false}
{"error":{"code":"invalid_request","kind":"not_an_object","message":"request must be a JSON object"},"id":null,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 15"},"id":161,"ok":false}
{"id":162,"ok":true,"result":{"len_chars":12,"text":"xtab\t\ny\r\nz\rw"}}
{"id":"r163","ok":true,"result":{"len_chars":2}}
{"error":{"code":"rejected","kind":"invalid_path","message":"invalid relative path \"a//b.txt\": path has an empty segment"},"id":164,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 15"},"id":165,"ok":false}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 15 is out of bounds (buffer has 0 chars)"},"id":166,"ok":false}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 4 is out of bounds (buffer has 3 chars)"},"id":"r167","ok":false}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 10 is out of bounds (buffer has 2 chars)"},"id":"r168","ok":false}
{"id":169,"ok":true,"result":{"dirty":true,"len_chars":8}}
{"id":170,"ok":true,"result":{"already_open":true,"doc":13,"encoding":"utf-8","len_chars":18,"line_ending":"lf","mixed_line_endings":false,"path":"sub/c.rs"}}
{"id":"r171","ok":true,"result":{"dirty":false,"len_chars":6,"undone":true}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 0"},"id":172,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 2"},"id":173,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 15"},"id":174,"ok":false}
{"id":175,"ok":true,"result":{"len_chars":12,"text":"xtab\t\ny\r\nz\rw"}}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 11 is out of bounds (buffer has 2 chars)"},"id":176,"ok":false}
{"error":{"code":"rejected","kind":"not_found","message":"cannot open sub/missing.rs: not found"},"id":177,"ok":false}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 4 is out of bounds (buffer has 2 chars)"},"id":178,"ok":false}
{"id":179,"ok":true,"result":{"bytes_written":12,"dirty":false,"doc":9}}
{"id":180,"ok":true,"result":{"bytes_written":6,"dirty":false,"doc":7}}
{"id":181,"ok":true,"result":{"closed":true,"doc":7}}
{"id":182,"ok":true,"result":{"dirty":true,"len_chars":13}}
{"id":183,"ok":true,"result":{"already_open":true,"doc":6,"encoding":"utf-8-bom","len_chars":4,"line_ending":"lf","mixed_line_endings":false,"path":"bom.txt"}}
{"id":184,"ok":true,"result":{"bytes_written":0,"dirty":false,"doc":11}}
{"id":185,"ok":true,"result":{"bytes_written":0,"dirty":false,"doc":11}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 0"},"id":186,"ok":false}
{"error":{"code":"invalid_json","kind":"syntax","message":"request is not JSON: EOF while parsing an object at line 1 column 1"},"id":null,"ok":false}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 3 is out of bounds (buffer has 2 chars)"},"id":188,"ok":false}
{"id":189,"ok":true,"result":{"len_chars":2,"text":"\r\n"}}
{"id":"r190","ok":true,"result":{"docs":[{"dirty":false,"doc":6,"path":"bom.txt"},{"dirty":true,"doc":9,"path":"m.txt"},{"dirty":false,"doc":10,"path":"u16.txt"},{"dirty":false,"doc":11,"path":"empty.txt"},{"dirty":false,"doc":12,"path":"b.txt"},{"dirty":false,"doc":13,"path":"sub/c.rs"}]}}
{"id":191,"ok":true,"result":{"bytes_written":7,"dirty":false,"doc":6}}
{"id":192,"ok":true,"result":{"dirty":false,"len_chars":4,"redone":false}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 0"},"id":"r193","ok":false}
{"error":{"code":"invalid_request","kind":"not_an_object","message":"request must be a JSON object"},"id":null,"ok":false}
{"id":195,"ok":true,"result":{"len_chars":2,"text":"\r\n"}}
{"error":{"code":"rejected","kind":"not_found","message":"cannot open sub/missing.rs: not found"},"id":196,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 2"},"id":197,"ok":false}
{"id":"r198","ok":true,"result":{"already_open":true,"doc":9,"encoding":"utf-8","len_chars":13,"line_ending":"lf","mixed_line_endings":true,"path":"m.txt"}}
{"id":199,"ok":true,"result":{"dirty":true,"len_chars":14,"removed":""}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 15"},"id":200,"ok":false}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 7 is out of bounds (buffer has 3 chars)"},"id":"r201","ok":false}
{"id":"r202","ok":true,"result":{"already_open":true,"doc":12,"encoding":"utf-8","len_chars":10,"line_ending":"crlf","mixed_line_endings":false,"path":"b.txt"}}
{"id":203,"ok":true,"result":{"len_chars":2,"text":"\r\n"}}
{"error":{"code":"invalid_request","kind":"unknown_key","message":"unknown top-level key(s) `extra`; allowed: `command`, `id`, `params`"},"id":"k","ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 2"},"id":205,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 0"},"id":206,"ok":false}
{"id":207,"ok":true,"result":{"bytes_written":8,"dirty":false,"doc":10}}
{"id":208,"ok":true,"result":{"bytes_written":7,"dirty":false,"doc":6}}
{"id":209,"ok":true,"result":{"docs":[{"dirty":false,"doc":6,"path":"bom.txt"},{"dirty":true,"doc":9,"path":"m.txt"},{"dirty":false,"doc":10,"path":"u16.txt"},{"dirty":false,"doc":11,"path":"empty.txt"},{"dirty":false,"doc":12,"path":"b.txt"},{"dirty":false,"doc":13,"path":"sub/c.rs"}]}}
{"id":210,"ok":true,"result":{"bytes_written":8,"dirty":false,"doc":10}}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 9 is out of bounds (buffer has 3 chars)"},"id":211,"ok":false}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 15 is out of bounds (buffer has 3 chars)"},"id":212,"ok":false}
{"id":213,"ok":true,"result":{"already_open":true,"doc":12,"encoding":"utf-8","len_chars":10,"line_ending":"crlf","mixed_line_endings":false,"path":"b.txt"}}
{"id":214,"ok":true,"result":{"dirty":true,"len_chars":14,"redone":false}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 0"},"id":"r215","ok":false}
{"id":216,"ok":true,"result":{"dirty":false,"len_chars":4,"undone":false}}
{"id":217,"ok":true,"result":{"already_open":false,"doc":14,"encoding":"utf-8","len_chars":6,"line_ending":"lf","mixed_line_endings":false,"path":"a.txt"}}
{"id":218,"ok":true,"result":{"closed":true,"doc":10}}
{"id":219,"ok":true,"result":{"dirty":true,"len_chars":5}}
{"id":220,"ok":true,"result":{"docs":[{"dirty":true,"doc":6,"path":"bom.txt"},{"dirty":true,"doc":9,"path":"m.txt"},{"dirty":false,"doc":11,"path":"empty.txt"},{"dirty":false,"doc":12,"path":"b.txt"},{"dirty":false,"doc":13,"path":"sub/c.rs"},{"dirty":false,"doc":14,"path":"a.txt"}]}}
{"error":{"code":"rejected","kind":"invalid_path","message":"invalid relative path \"./a.txt\": path has a '.' segment"},"id":221,"ok":false}
{"id":222,"ok":true,"result":{"len_chars":0,"text":""}}
{"id":223,"ok":true,"result":{"bytes_written":0,"dirty":false,"doc":11}}
{"error":{"code":"invalid_request","kind":"missing_command","message":"`command` is missing"},"id":"m","ok":false}
{"error":{"code":"rejected","kind":"inverted_range","message":"range start 8 is after range end 1"},"id":"r225","ok":false}
{"id":226,"ok":true,"result":{"closed":true,"doc":6}}
{"id":227,"ok":true,"result":{"len_chars":10,"text":"one\r\ntwo\r\n"}}
{"error":{"code":"invalid_request","kind":"not_an_object","message":"request must be a JSON object"},"id":null,"ok":false}
{"error":{"code":"invalid_request","kind":"not_an_object","message":"request must be a JSON object"},"id":null,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 16"},"id":230,"ok":false}
{"id":231,"ok":true,"result":{"dirty":true,"len_chars":18}}
{"id":232,"ok":true,"result":{"len_chars":2,"text":"\r\n"}}
{"id":233,"ok":true,"result":{"len_chars":0,"text":""}}
{"id":234,"ok":true,"result":{"dirty":true,"len_chars":20}}
{"id":235,"ok":true,"result":{"already_open":true,"doc":13,"encoding":"utf-8","len_chars":18,"line_ending":"lf","mixed_line_endings":false,"path":"sub/c.rs"}}
{"id":236,"ok":true,"result":{"docs":[{"dirty":true,"doc":9,"path":"m.txt"},{"dirty":false,"doc":11,"path":"empty.txt"},{"dirty":true,"doc":12,"path":"b.txt"},{"dirty":false,"doc":13,"path":"sub/c.rs"},{"dirty":false,"doc":14,"path":"a.txt"}]}}
{"id":237,"ok":true,"result":{"bytes_written":21,"dirty":false,"doc":9}}
{"id":238,"ok":true,"result":{"bytes_written":0,"dirty":false,"doc":11}}
{"id":"r239","ok":true,"result":{"dirty":true,"len_chars":4}}
{"id":240,"ok":true,"result":{"bytes_written":18,"dirty":false,"doc":13}}
{"error":{"code":"rejected","kind":"not_found","message":"cannot open sub/missing.rs: not found"},"id":241,"ok":false}
{"id":242,"ok":true,"result":{"bytes_written":20,"dirty":false,"doc":12}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 0"},"id":243,"ok":false}
{"error":{"code":"rejected","kind":"decode_error","message":"cannot open bad.bin: invalid utf-8 at byte offset 2"},"id":"r244","ok":false}
{"id":245,"ok":true,"result":{"len_chars":2,"text":"\r\n"}}
{"id":246,"ok":true,"result":{"len_chars":2,"text":"\r\n"}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 17"},"id":247,"ok":false}
{"id":248,"ok":true,"result":{"already_open":true,"doc":13,"encoding":"utf-8","len_chars":18,"line_ending":"lf","mixed_line_endings":false,"path":"sub/c.rs"}}
{"id":"r249","ok":true,"result":{"len_chars":2,"text":"\r\n"}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 0"},"id":250,"ok":false}
{"id":251,"ok":true,"result":{"dirty":true,"len_chars":20}}
{"id":252,"ok":true,"result":{"bytes_written":4,"dirty":false,"doc":11}}
{"id":253,"ok":true,"result":{"bytes_written":20,"dirty":false,"doc":12}}
{"id":254,"ok":true,"result":{"bytes_written":20,"dirty":false,"doc":12}}
{"id":255,"ok":true,"result":{"already_open":true,"doc":13,"encoding":"utf-8","len_chars":20,"line_ending":"lf","mixed_line_endings":false,"path":"sub/c.rs"}}
{"id":256,"ok":true,"result":{"bytes_written":4,"dirty":false,"doc":11}}
{"id":257,"ok":true,"result":{"dirty":true,"len_chars":14,"undone":true}}
{"id":258,"ok":true,"result":{"bytes_written":20,"dirty":false,"doc":13}}
{"error":{"code":"rejected","kind":"inverted_range","message":"range start 6 is after range end 0"},"id":"r259","ok":false}
{"id":260,"ok":true,"result":{"already_open":true,"doc":9,"encoding":"utf-8","len_chars":14,"line_ending":"lf","mixed_line_endings":true,"path":"m.txt"}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 17"},"id":262,"ok":false}
{"error":{"code":"unknown_command","kind":"unknown_command","message":"unknown command `zz.no-such-command`"},"id":"u","ok":false}
{"id":265,"ok":true,"result":{"already_open":false,"doc":15,"encoding":"utf-8-bom","len_chars":4,"line_ending":"lf","mixed_line_endings":false,"path":"bom.txt"}}
{"id":"r266","ok":true,"result":{"dirty":false,"len_chars":6,"redone":false}}
{"id":"r267","ok":true,"result":{"dirty":true,"len_chars":8}}
{"id":268,"ok":true,"result":{"dirty":true,"len_chars":18,"undone":true}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 0"},"id":"r269","ok":false}
{"id":270,"ok":true,"result":{"dirty":true,"len_chars":11,"removed":"😀\n z"}}
{"id":271,"ok":true,"result":{"docs":[{"dirty":true,"doc":9,"path":"m.txt"},{"dirty":false,"doc":11,"path":"empty.txt"},{"dirty":false,"doc":12,"path":"b.txt"},{"dirty":true,"doc":13,"path":"sub/c.rs"},{"dirty":true,"doc":14,"path":"a.txt"},{"dirty":false,"doc":15,"path":"bom.txt"}]}}
{"id":272,"ok":true,"result":{"dirty":true,"len_chars":9,"removed":"one\r\ntwo\r\nf"}}
{"error":{"code":"rejected","kind":"invalid_path","message":"invalid relative path \"../up.txt\": path has a '..' segment"},"id":273,"ok":false}
{"error":{"code":"rejected","kind":"unsaved_changes","message":"document 12 has unsaved changes; save it or close with discard"},"id":274,"ok":false}
{"error":{"code":"rejected","kind":"inverted_range","message":"range start 2 is after range end 0"},"id":275,"ok":false}
{"error":{"code":"rejected","kind":"invalid_path","message":"invalid relative path \"a//b.txt\": path has an empty segment"},"id":"r276","ok":false}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 14 is out of bounds (buffer has 2 chars)"},"id":277,"ok":false}
{"id":"r278","ok":true,"result":{"already_open":true,"doc":14,"encoding":"utf-8","len_chars":8,"line_ending":"lf","mixed_line_endings":false,"path":"a.txt"}}
{"id":"r279","ok":true,"result":{"len_chars":3}}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 10 is out of bounds (buffer has 8 chars)"},"id":"r280","ok":false}
{"error":{"code":"rejected","kind":"inverted_range","message":"range start 9 is after range end 4"},"id":281,"ok":false}
{"id":282,"ok":true,"result":{"len_chars":3,"text":"\n\r\n"}}
{"id":283,"ok":true,"result":{"docs":[{"dirty":true,"doc":9,"path":"m.txt"},{"dirty":false,"doc":11,"path":"empty.txt"},{"dirty":true,"doc":12,"path":"b.txt"},{"dirty":true,"doc":13,"path":"sub/c.rs"},{"dirty":true,"doc":14,"path":"a.txt"},{"dirty":false,"doc":15,"path":"bom.txt"}]}}
{"id":284,"ok":true,"result":{"dirty":true,"len_chars":10}}
{"id":"r285","ok":true,"result":{"dirty":true,"len_chars":19}}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 9 is out of bounds (buffer has 3 chars)"},"id":"r286","ok":false}
{"id":287,"ok":true,"result":{"dirty":true,"len_chars":11}}
{"id":288,"ok":true,"result":{"already_open":true,"doc":14,"encoding":"utf-8","len_chars":8,"line_ending":"lf","mixed_line_endings":false,"path":"a.txt"}}
{"id":289,"ok":true,"result":{"bytes_written":14,"dirty":false,"doc":12}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 18"},"id":290,"ok":false}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 5 is out of bounds (buffer has 3 chars)"},"id":291,"ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 18"},"id":292,"ok":false}
{"id":293,"ok":true,"result":{"bytes_written":4,"dirty":false,"doc":11}}
{"error":{"code":"rejected","kind":"invalid_path","message":"invalid relative path \"a//b.txt\": path has an empty segment"},"id":294,"ok":false}
{"id":"r295","ok":true,"result":{"already_open":true,"doc":13,"encoding":"utf-8","len_chars":19,"line_ending":"lf","mixed_line_endings":false,"path":"sub/c.rs"}}
{"id":296,"ok":true,"result":{"already_open":false,"doc":16,"encoding":"utf-16le","len_chars":3,"line_ending":"lf","mixed_line_endings":false,"path":"u16.txt"}}
{"id":297,"ok":true,"result":{"dirty":true,"len_chars":11,"redone":false}}
{"id":298,"ok":true,"result":{"already_open":false,"doc":17,"encoding":"utf-16be","len_chars":3,"line_ending":"lf","mixed_line_endings":false,"path":"u16be.txt"}}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 6 is out of bounds (buffer has 3 chars)"},"id":299,"ok":false}
{"id":"r300","ok":true,"result":{"len_chars":19,"text":"fxn matabx\tin() {}\n"}}
{"id":"r301","ok":true,"result":{"bytes_written":8,"dirty":false,"doc":14}}
{"id":-7,"ok":true,"result":{"len_chars":3,"text":"\n\r\n"}}
{"id":303,"ok":true,"result":{"len_chars":3,"text":"\n\r\n"}}
{"id":304,"ok":true,"result":{"len_chars":3,"text":"\n\r\n"}}
{"id":305,"ok":true,"result":{"bytes_written":11,"dirty":false,"doc":9}}
{"error":{"code":"invalid_params","details":["Additional properties are not allowed ('x' was unexpected)"],"kind":"schema","message":"invalid params for `doc.open`"},"id":"z","ok":false}
{"id":"r307","ok":true,"result":{"bytes_written":19,"dirty":false,"doc":13}}
{"id":308,"ok":true,"result":{"len_chars":4}}
{"id":309,"ok":true,"result":{"already_open":true,"doc":16,"encoding":"utf-16le","len_chars":3,"line_ending":"lf","mixed_line_endings":false,"path":"u16.txt"}}
{"error":{"code":"rejected","kind":"out_of_range","message":"char index 10 is out of bounds (buffer has 3 chars)"},"id":310,"ok":false}
{"id":311,"ok":true,"result":{"already_open":true,"doc":14,"encoding":"utf-8","len_chars":8,"line_ending":"lf","mixed_line_endings":false,"path":"a.txt"}}
{"id":312,"ok":true,"result":{"can_redo":false,"can_undo":true,"dirty":false,"doc":12,"encoding":"utf-8","len_chars":11,"len_lines":2,"line_ending":"lf","mixed_line_endings":false,"path":"b.txt"}}
{"id":313,"ok":true,"result":{"closed":true,"doc":13}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 13"},"id":"r314","ok":false}
{"error":{"code":"invalid_request","kind":"params_not_object","message":"`params` must be an object"},"id":"p","ok":false}
{"id":316,"ok":true,"result":{"docs":[{"dirty":false,"doc":9,"path":"m.txt"},{"dirty":false,"doc":11,"path":"empty.txt"},{"dirty":false,"doc":12,"path":"b.txt"},{"dirty":false,"doc":14,"path":"a.txt"},{"dirty":false,"doc":15,"path":"bom.txt"},{"dirty":false,"doc":16,"path":"u16.txt"},{"dirty":false,"doc":17,"path":"u16be.txt"}]}}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 20"},"id":317,"ok":false}
{"id":318,"ok":true,"result":{"can_redo":false,"can_undo":true,"dirty":false,"doc":11,"encoding":"utf-8","len_chars":4,"len_lines":1,"line_ending":"none","mixed_line_endings":false,"path":"empty.txt"}}
{"error":{"code":"rejected","kind":"not_found","message":"cannot open sub/missing.rs: not found"},"id":"r319","ok":false}
{"error":{"code":"invalid_request","kind":"not_an_object","message":"request must be a JSON object"},"id":null,"ok":false}
{"id":321,"ok":true,"result":{"dirty":false,"len_chars":3,"redone":false}}
{"error":{"code":"invalid_request","kind":"command_not_string","message":"`command` must be a string"},"id":"n","ok":false}
{"error":{"code":"rejected","kind":"no_such_document","message":"no open document 0"},"id":"r323","ok":false}
"#;

const SESSION_LOG: &str = r#"{"command":"doc.open","id":1,"params":{"path":"sub/deep/d.txt"}}
{"command":"doc.open","id":2,"params":{"path":"sub/c.rs"}}
{"command":"doc.text","id":3,"params":{"doc":1}}
{"command":"doc.insert","id":4,"params":{"doc":2,"offset":5,"text":"tab\t"}}
{"command":"doc.list","id":5,"params":{}}
{"command":"doc.open","id":6,"params":{"path":"a.txt"}}
{"command":"doc.undo","id":7,"params":{"doc":3}}
{"command":"doc.undo","id":8,"params":{"doc":3}}
{"command":"doc.insert","id":9,"params":{"doc":3,"offset":4,"text":"\r"}}
{"command":"doc.list","id":10,"params":{}}
{"command":"doc.delete","id":11,"params":{"doc":2,"end":4,"start":4}}
{"command":"doc.open","id":12,"params":{"path":"u16.txt"}}
{"command":"doc.open","id":13,"params":{"path":"a.txt"}}
{"command":"doc.open","id":14,"params":{"path":"empty.txt"}}
{"command":"doc.save","id":15,"params":{"doc":5}}
{"command":"buffer.text","id":16,"params":{}}
{"command":"buffer.text","id":17,"params":{}}
{"command":"doc.redo","id":18,"params":{"doc":2}}
{"command":"buffer.text","id":19,"params":{}}
{"command":"doc.save","id":20,"params":{"doc":4}}
{"command":"doc.open","id":21,"params":{"path":"bom.txt"}}
{"command":"doc.close","id":22,"params":{"discard":true,"doc":3}}
{"command":"doc.list","id":23,"params":{}}
{"command":"doc.open","id":24,"params":{"path":"a.txt"}}
{"command":"doc.list","id":25,"params":{}}
{"command":"doc.redo","id":26,"params":{"doc":6}}
{"command":"doc.text","id":27,"params":{"doc":2}}
{"command":"doc.save","id":28,"params":{"doc":7}}
{"command":"doc.open","id":29,"params":{"path":"sub/deep/d.txt"}}
{"command":"doc.close","id":30,"params":{"doc":4}}
{"command":"buffer.text","id":31,"params":{}}
{"command":"buffer.text","id":32,"params":{}}
{"command":"doc.insert","id":33,"params":{"doc":2,"offset":8,"text":"x"}}
{"command":"doc.list","id":34,"params":{}}
{"command":"doc.open","id":35,"params":{"path":"b.txt"}}
{"command":"doc.insert","id":36,"params":{"doc":8,"offset":5,"text":"fn f() {}\n"}}
{"command":"doc.close","id":37,"params":{"discard":true,"doc":8}}
{"command":"doc.open","id":38,"params":{"path":"m.txt"}}
{"command":"doc.replace","id":39,"params":{"doc":9,"end":7,"start":7,"text":"😀"}}
{"command":"doc.text","id":40,"params":{"doc":1}}
{"command":"doc.save","id":41,"params":{"doc":1}}
{"command":"doc.open","id":42,"params":{"path":"sub/c.rs"}}
{"command":"buffer.insert","id":43,"params":{"offset":0,"text":"\r"}}
{"command":"doc.close","id":44,"params":{"discard":true,"doc":5}}
{"command":"doc.save","id":45,"params":{"doc":1}}
{"command":"doc.open","id":46,"params":{"path":"u16.txt"}}
{"command":"buffer.text","id":47,"params":{}}
{"command":"doc.open","id":48,"params":{"path":"m.txt"}}
{"command":"doc.open","id":49,"params":{"path":"a.txt"}}
{"command":"doc.open","id":50,"params":{"path":"m.txt"}}
{"command":"doc.undo","id":51,"params":{"doc":9}}
{"command":"doc.text","id":52,"params":{"doc":9}}
{"command":"doc.list","id":53,"params":{}}
{"command":"doc.close","id":54,"params":{"discard":true,"doc":1}}
{"command":"doc.insert","id":55,"params":{"doc":9,"offset":1,"text":"tab\t"}}
{"command":"doc.open","id":56,"params":{"path":"empty.txt"}}
{"command":"doc.open","id":57,"params":{"path":"b.txt"}}
{"command":"doc.open","id":58,"params":{"path":"u16.txt"}}
{"command":"doc.open","id":59,"params":{"path":"m.txt"}}
{"command":"doc.list","id":60,"params":{}}
{"command":"doc.save","id":61,"params":{"doc":2}}
{"command":"buffer.text","id":62,"params":{}}
{"command":"doc.undo","id":63,"params":{"doc":6}}
{"command":"doc.save","id":64,"params":{"doc":6}}
{"command":"doc.text","id":65,"params":{"doc":10}}
{"command":"doc.save","id":66,"params":{"doc":2}}
{"command":"doc.list","id":67,"params":{}}
{"command":"doc.close","id":68,"params":{"doc":2}}
{"command":"doc.save","id":69,"params":{"doc":6}}
{"command":"doc.open","id":70,"params":{"path":"sub/c.rs"}}
{"command":"doc.text","id":71,"params":{"doc":9}}
{"command":"buffer.insert","id":72,"params":{"offset":1,"text":"\n"}}
{"command":"doc.insert","id":73,"params":{"doc":7,"offset":3,"text":"ab"}}
{"command":"doc.open","id":74,"params":{"path":"sub/c.rs"}}
{"command":"doc.undo","id":75,"params":{"doc":7}}
{"command":"doc.text","id":76,"params":{"doc":9}}
{"command":"doc.save","id":77,"params":{"doc":9}}
{"command":"doc.save","id":78,"params":{"doc":7}}
{"command":"doc.close","id":79,"params":{"discard":true,"doc":7}}
{"command":"doc.insert","id":80,"params":{"doc":9,"offset":9,"text":" "}}
{"command":"doc.open","id":81,"params":{"path":"bom.txt"}}
{"command":"doc.save","id":82,"params":{"doc":11}}
{"command":"doc.save","id":83,"params":{"doc":11}}
{"command":"buffer.text","id":84,"params":{}}
{"command":"doc.list","id":85,"params":{}}
{"command":"doc.save","id":86,"params":{"doc":6}}
{"command":"doc.redo","id":87,"params":{"doc":6}}
{"command":"buffer.text","id":88,"params":{}}
{"command":"doc.open","id":89,"params":{"path":"m.txt"}}
{"command":"doc.replace","id":90,"params":{"doc":9,"end":8,"start":8,"text":"😀"}}
{"command":"doc.open","id":91,"params":{"path":"b.txt"}}
{"command":"buffer.text","id":92,"params":{}}
{"command":"doc.save","id":93,"params":{"doc":10}}
{"command":"doc.save","id":94,"params":{"doc":6}}
{"command":"doc.list","id":95,"params":{}}
{"command":"doc.save","id":96,"params":{"doc":10}}
{"command":"doc.open","id":97,"params":{"path":"b.txt"}}
{"command":"doc.redo","id":98,"params":{"doc":9}}
{"command":"doc.undo","id":99,"params":{"doc":6}}
{"command":"doc.open","id":100,"params":{"path":"a.txt"}}
{"command":"doc.close","id":101,"params":{"discard":true,"doc":10}}
{"command":"doc.insert","id":102,"params":{"doc":6,"offset":4,"text":"\r"}}
{"command":"doc.list","id":103,"params":{}}
{"command":"doc.text","id":104,"params":{"doc":11}}
{"command":"doc.save","id":105,"params":{"doc":11}}
{"command":"doc.close","id":106,"params":{"discard":true,"doc":6}}
{"command":"doc.text","id":107,"params":{"doc":12}}
{"command":"doc.insert","id":108,"params":{"doc":9,"offset":3,"text":"tab\t"}}
{"command":"buffer.text","id":109,"params":{}}
{"command":"doc.text","id":110,"params":{"doc":11}}
{"command":"doc.insert","id":111,"params":{"doc":12,"offset":10,"text":"fn f() {}\n"}}
{"command":"doc.open","id":112,"params":{"path":"sub/c.rs"}}
{"command":"doc.list","id":113,"params":{}}
{"command":"doc.save","id":114,"params":{"doc":9}}
{"command":"doc.save","id":115,"params":{"doc":11}}
{"command":"doc.insert","id":116,"params":{"doc":11,"offset":0,"text":"tab\t"}}
{"command":"doc.save","id":117,"params":{"doc":13}}
{"command":"doc.save","id":118,"params":{"doc":12}}
{"command":"buffer.text","id":119,"params":{}}
{"command":"buffer.text","id":120,"params":{}}
{"command":"doc.open","id":121,"params":{"path":"sub/c.rs"}}
{"command":"buffer.text","id":122,"params":{}}
{"command":"doc.insert","id":123,"params":{"doc":13,"offset":12,"text":"ab"}}
{"command":"doc.save","id":124,"params":{"doc":11}}
{"command":"doc.save","id":125,"params":{"doc":12}}
{"command":"doc.save","id":126,"params":{"doc":12}}
{"command":"doc.open","id":127,"params":{"path":"sub/c.rs"}}
{"command":"doc.save","id":128,"params":{"doc":11}}
{"command":"doc.undo","id":129,"params":{"doc":9}}
{"command":"doc.save","id":130,"params":{"doc":13}}
{"command":"doc.open","id":131,"params":{"path":"m.txt"}}
{"command":"doc.open","id":132,"params":{"path":"bom.txt"}}
{"command":"doc.redo","id":133,"params":{"doc":14}}
{"command":"doc.insert","id":134,"params":{"doc":14,"offset":3,"text":"ab"}}
{"command":"doc.undo","id":135,"params":{"doc":13}}
{"command":"doc.replace","id":136,"params":{"doc":9,"end":12,"start":8,"text":" "}}
{"command":"doc.list","id":137,"params":{}}
{"command":"doc.delete","id":138,"params":{"doc":12,"end":11,"start":0}}
{"command":"doc.open","id":139,"params":{"path":"a.txt"}}
{"command":"buffer.insert","id":140,"params":{"offset":0,"text":"\n"}}
{"command":"buffer.text","id":141,"params":{}}
{"command":"doc.list","id":142,"params":{}}
{"command":"doc.insert","id":143,"params":{"doc":12,"offset":1,"text":"😀"}}
{"command":"doc.insert","id":144,"params":{"doc":13,"offset":1,"text":"x"}}
{"command":"doc.insert","id":145,"params":{"doc":12,"offset":7,"text":"q"}}
{"command":"doc.open","id":146,"params":{"path":"a.txt"}}
{"command":"doc.save","id":147,"params":{"doc":12}}
{"command":"doc.save","id":148,"params":{"doc":11}}
{"command":"doc.open","id":149,"params":{"path":"sub/c.rs"}}
{"command":"doc.open","id":150,"params":{"path":"u16.txt"}}
{"command":"doc.redo","id":151,"params":{"doc":9}}
{"command":"doc.open","id":152,"params":{"path":"u16be.txt"}}
{"command":"doc.text","id":153,"params":{"doc":13}}
{"command":"doc.save","id":154,"params":{"doc":14}}
{"command":"buffer.text","id":155,"params":{}}
{"command":"buffer.text","id":156,"params":{}}
{"command":"buffer.text","id":157,"params":{}}
{"command":"doc.save","id":158,"params":{"doc":9}}
{"command":"doc.save","id":159,"params":{"doc":13}}
{"command":"buffer.insert","id":160,"params":{"offset":0,"text":" "}}
{"command":"doc.open","id":161,"params":{"path":"u16.txt"}}
{"command":"doc.open","id":162,"params":{"path":"a.txt"}}
{"command":"doc.info","id":163,"params":{"doc":12}}
{"command":"doc.close","id":164,"params":{"doc":13}}
{"command":"doc.list","id":165,"params":{}}
{"command":"doc.info","id":166,"params":{"doc":11}}
{"command":"doc.redo","id":167,"params":{"doc":16}}
"#;

const SESSION_FILES_AFTER: &str = r#"a.txt = helablo\n
b.txt = n\xf0\x9f\x98\x80 f() q{}\n
bad.bin = ok\x80\x81\n
bom.txt = \xef\xbb\xbfbom\n
empty.txt = tab\t
m.txt = xtab\t\ny\r \rw
sub/ = 
sub/c.rs = fxn matabx\tin() {}\n
sub/deep/ = 
sub/deep/d.txt = \xc3\xa9\xf0\x9f\x98\x80\r\n
u16.txt = \xff\xfeh\x00i\x00\n\x00
u16be.txt = \xfe\xff\x00h\x00i\x00\n
"#;
