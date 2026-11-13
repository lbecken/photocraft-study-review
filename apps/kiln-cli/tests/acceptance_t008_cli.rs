//! T-008 acceptance tests through `kiln-cli` (`run`, `exec`, `control`) and the contract
//! documents: the reviewer's symlink exploit is refused end to end (AC-3), FIFOs, sockets and
//! directories are refused without blocking (AC-6), a link swapped in between requests is
//! refused at save (AC-4 through `control`), and the generated kind list and the protocol
//! documents state the rule (AC-9). Written by the test author from the task specification
//! alone.
//!
//! Every spawned process is killed after a timeout (D-028 (8)); a hang is a test failure. On the
//! base, `doc.open` of a FIFO blocks, so the FIFO tests fail there by timeout, the others by
//! assertion. No test counts or orders `error_kinds` or compares a whole document (D-043, D-051,
//! D-052).
#![cfg(test)]

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::{FileTypeExt, symlink};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const SECRET: &[u8] = b"s3cr3t-content\n";
/// Upper bound for one `kiln-cli` invocation or one `control` response (AC-6: 10 seconds).
const TIMEOUT: Duration = Duration::from_secs(10);
const NOT_A_FILE_WHEN: &str = "the path names a directory or another non-regular file where a \
                               file is needed, or a file where a directory is needed";

// ---------------------------------------------------------------------------------------------
// Fixtures.

/// A fresh `work/` with `root/`, `outside/` and `outside/secret.txt`; removed on drop.
struct Work {
    dir: PathBuf,
}

impl Work {
    fn new(name: &str) -> Work {
        let dir = std::env::temp_dir().join(format!("k8c-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let dir = fs::canonicalize(&dir).unwrap();
        fs::create_dir(dir.join("root")).unwrap();
        fs::create_dir(dir.join("outside")).unwrap();
        fs::write(dir.join("outside/secret.txt"), SECRET).unwrap();
        Work { dir }
    }

    fn at(&self, rel: &str) -> PathBuf {
        self.dir.join(rel)
    }

    fn link(&self, at: &str, target: &str) {
        symlink(target, self.at(at)).unwrap();
    }

    fn file(&self, at: &str, bytes: &[u8]) {
        let p = self.at(at);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, bytes).unwrap();
    }

    fn text(&self) -> String {
        self.dir.to_string_lossy().into_owned()
    }
}

impl Drop for Work {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn wait_or_kill(child: &mut Child, what: &str) -> ExitStatus {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{what} did not finish within {TIMEOUT:?} (blocked)");
        }
        thread::sleep(Duration::from_millis(5));
    }
}

/// Run `kiln-cli args` in `cwd` with empty stdin; kill it after [`TIMEOUT`].
fn kiln(args: &[&str], cwd: &Path) -> Output {
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
    let out = drain(Box::new(child.stdout.take().unwrap()));
    let err = drain(Box::new(child.stderr.take().unwrap()));
    let status = wait_or_kill(&mut child, &format!("kiln-cli {}", args.join(" ")));
    Output {
        status,
        stdout: out.join().unwrap(),
        stderr: err.join().unwrap(),
    }
}

fn mkfifo(path: &Path) {
    let mut child = Command::new("mkfifo")
        .arg(path)
        .stdin(Stdio::null())
        .spawn()
        .expect("spawn mkfifo");
    assert!(wait_or_kill(&mut child, "mkfifo").success());
    assert!(fs::symlink_metadata(path).unwrap().file_type().is_fifo());
}

/// Bind a Unix socket at a short path in the same file system and rename it to `path`.
fn socket_at(path: &Path) -> UnixListener {
    static N: AtomicUsize = AtomicUsize::new(0);
    let short = std::env::temp_dir().join(format!(
        "k8t{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_file(&short);
    let listener = UnixListener::bind(&short).unwrap();
    fs::rename(&short, path).unwrap();
    assert!(fs::symlink_metadata(path).unwrap().file_type().is_socket());
    listener
}

/// Every entry under `dir`, recursively, symlinks not followed: type, link target, bytes.
fn snapshot(dir: &Path) -> BTreeMap<String, String> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            let rel = p.strip_prefix(base).unwrap().to_string_lossy().into_owned();
            let ft = fs::symlink_metadata(&p).unwrap().file_type();
            let desc = if ft.is_symlink() {
                format!("link -> {}", fs::read_link(&p).unwrap().display())
            } else if ft.is_dir() {
                walk(base, &p, out);
                "dir".to_owned()
            } else if ft.is_file() {
                format!("file {}", fs::read(&p).unwrap().escape_ascii())
            } else if ft.is_fifo() {
                "fifo".to_owned()
            } else if ft.is_socket() {
                "socket".to_owned()
            } else {
                "other".to_owned()
            };
            out.insert(rel, desc);
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

fn assert_no_temp(dir: &Path) {
    for name in snapshot(dir).keys() {
        let last = name.rsplit('/').next().unwrap_or(name);
        assert!(
            !last.starts_with(".kiln-tmp-"),
            "temporary file left: {name}"
        );
    }
}

fn lines(stdout: &[u8]) -> Vec<Value> {
    String::from_utf8(stdout.to_vec())
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("bad line {l:?}: {e}")))
        .collect()
}

/// `line` is a `rejected` response with this id, kind and (if given) exact message.
fn assert_rejected(line: &Value, id: u64, kind: &str, message: Option<&str>) {
    assert_eq!(line["id"], json!(id), "{line}");
    assert_eq!(
        line["ok"],
        json!(false),
        "line {id} is not a refusal: {line}"
    );
    assert_eq!(line["error"]["code"], json!("rejected"), "{line}");
    assert_eq!(line["error"]["kind"], json!(kind), "{line}");
    if let Some(m) = message {
        assert_eq!(line["error"]["message"], json!(m), "{line}");
    }
}

fn script(requests: &[Value]) -> String {
    requests.iter().map(|r| format!("{r}\n")).collect()
}

// ---------------------------------------------------------------------------------------------
// AC-3

/// `work/` with the links of the reviewer's exploit and the script `s.jsonl`.
fn exploit_fixture(name: &str) -> Work {
    let w = Work::new(name);
    w.link("root/dlink", "../outside");
    let abs = w.at("outside/secret.txt");
    w.link("root/abs", abs.to_str().unwrap());
    // The escapes are real.
    assert_eq!(fs::read(w.at("root/dlink/secret.txt")).unwrap(), SECRET);
    assert_eq!(fs::read(w.at("root/abs")).unwrap(), SECRET);
    let s = script(&[
        json!({"id":1,"command":"doc.open","params":{"path":"dlink/secret.txt"}}),
        json!({"id":2,"command":"doc.insert","params":{"doc":1,"offset":0,"text":"PWNED "}}),
        json!({"id":3,"command":"doc.save","params":{"doc":1}}),
        json!({"id":4,"command":"doc.open","params":{"path":"abs"}}),
        json!({"id":5,"command":"doc.list","params":{}}),
    ]);
    fs::write(w.at("s.jsonl"), s).unwrap();
    w
}

#[test]
fn reviewer_exploit_is_refused_end_to_end() {
    let w = exploit_fixture("exploit");
    let before = snapshot(&w.dir);
    let out = kiln(&["run", "s.jsonl", "--root", "root"], &w.dir);
    let got = lines(&out.stdout);
    assert_eq!(
        got.len(),
        5,
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert_rejected(
        &got[0],
        1,
        "outside_root",
        Some("cannot open dlink/secret.txt: resolves outside the root"),
    );
    assert_rejected(&got[1], 2, "no_such_document", None);
    assert_rejected(&got[2], 3, "no_such_document", None);
    assert_rejected(
        &got[3],
        4,
        "outside_root",
        Some("cannot open abs: resolves outside the root"),
    );
    assert_eq!(got[4], json!({"id":5,"ok":true,"result":{"docs":[]}}));
    assert_eq!(out.status.code(), Some(3));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.contains(&w.text()), "stdout names the work path");
    assert!(
        !stdout.contains("s3cr3t-content"),
        "stdout leaks the secret"
    );
    assert_eq!(snapshot(&w.dir), before);
    assert_eq!(fs::read(w.at("outside/secret.txt")).unwrap(), SECRET);

    // Differential: the same script on a copy at another absolute path, byte-identical output.
    let w2 = exploit_fixture("exploit-copy");
    let out2 = kiln(&["run", "s.jsonl", "--root", "root"], &w2.dir);
    assert_eq!(
        String::from_utf8_lossy(&out2.stdout),
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(out2.status.code(), Some(3));

    // `exec` reports the same refusal; the lexical refusal still comes first.
    let out = kiln(
        &["exec", "doc.open", r#"{"path":"abs"}"#, "--root", "root"],
        &w.dir,
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        "{\"code\":\"rejected\",\"kind\":\"outside_root\",\
         \"message\":\"cannot open abs: resolves outside the root\"}\n"
    );
    assert_eq!(out.stdout, b"");
    assert_eq!(out.status.code(), Some(2));
    let out = kiln(
        &[
            "exec",
            "doc.open",
            r#"{"path":"../outside/secret.txt"}"#,
            "--root",
            "root",
        ],
        &w.dir,
    );
    let err: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["kind"], json!("invalid_path"), "{err}");
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(snapshot(&w.dir), before);
}

// ---------------------------------------------------------------------------------------------
// AC-6

#[test]
fn fifo_and_socket_do_not_hang_run() {
    let w = Work::new("run-nonreg");
    mkfifo(&w.at("root/p"));
    let _listener = socket_at(&w.at("root/s"));
    w.file("root/d/x.txt", b"keep\n");
    let s = script(&[
        json!({"id":1,"command":"doc.open","params":{"path":"p"}}),
        json!({"id":2,"command":"doc.open","params":{"path":"s"}}),
        json!({"id":3,"command":"doc.open","params":{"path":"d"}}),
        json!({"id":4,"command":"doc.list","params":{}}),
    ]);
    fs::write(w.at("s.jsonl"), s).unwrap();
    let before = snapshot(&w.dir);
    let out = kiln(&["run", "s.jsonl", "--root", "root"], &w.dir);
    let got = lines(&out.stdout);
    assert_eq!(
        got.len(),
        4,
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    for (i, p) in ["p", "s", "d"].iter().enumerate() {
        assert_rejected(
            &got[i],
            i as u64 + 1,
            "not_a_file",
            Some(&format!("cannot open {p}: not a regular file")),
        );
    }
    assert_eq!(got[3]["id"], json!(4));
    assert_eq!(got[3]["ok"], json!(true), "{}", got[3]);
    assert_eq!(out.status.code(), Some(3));
    assert_eq!(snapshot(&w.dir), before);
}

#[test]
fn fifo_does_not_hang_exec() {
    let w = Work::new("exec-fifo");
    mkfifo(&w.at("root/p"));
    let out = kiln(
        &["exec", "doc.open", r#"{"path":"p"}"#, "--root", "root"],
        &w.dir,
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        "{\"code\":\"rejected\",\"kind\":\"not_a_file\",\
         \"message\":\"cannot open p: not a regular file\"}\n"
    );
    assert_eq!(out.stdout, b"");
    assert_eq!(out.status.code(), Some(2));
    assert!(
        fs::symlink_metadata(w.at("root/p"))
            .unwrap()
            .file_type()
            .is_fifo()
    );
}

// ---------------------------------------------------------------------------------------------
// AC-4 through the control protocol

/// A live `kiln-cli control` session; killed on drop.
struct Session {
    child: Child,
    stdin: Option<ChildStdin>,
    out: Receiver<String>,
}

impl Session {
    fn start(cwd: &Path) -> Session {
        let mut child = Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
            .args(["control", "--root", "root"])
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn kiln-cli control");
        let stdout = child.stdout.take().unwrap();
        let (tx, out) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        let stdin = child.stdin.take();
        Session { child, stdin, out }
    }

    /// Send one request and wait (at most [`TIMEOUT`]) for its response line.
    fn ask(&mut self, request: Value) -> Value {
        let stdin = self.stdin.as_mut().unwrap();
        writeln!(stdin, "{request}").unwrap();
        stdin.flush().unwrap();
        match self.out.recv_timeout(TIMEOUT) {
            Ok(line) => serde_json::from_str(&line).unwrap(),
            Err(e) => {
                let _ = self.child.kill();
                panic!("no response to {request} within {TIMEOUT:?}: {e}");
            }
        }
    }

    fn finish(mut self) -> ExitStatus {
        drop(self.stdin.take());
        wait_or_kill(&mut self.child, "kiln-cli control")
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn swap_between_requests_is_refused_through_control() {
    let w = Work::new("control-swap");
    w.file("root/a.txt", b"a original\n");
    w.file("root/sub/f.txt", b"sub original\n");
    w.file("root/c.txt", b"c original\n");
    w.file("root/real/t.txt", b"c original\n");
    w.file("outside/f.txt", b"outside f\n");

    let mut s = Session::start(&w.dir);
    for (id, path) in [(1, "a.txt"), (2, "sub/f.txt"), (3, "c.txt")] {
        let r = s.ask(json!({"id":id,"command":"doc.open","params":{"path":path}}));
        assert_eq!(r["ok"], json!(true), "{r}");
        assert_eq!(r["result"]["doc"], json!(id), "{r}");
    }
    for doc in 1..=3 {
        let r = s.ask(json!({"id":10 + doc,"command":"doc.insert",
                             "params":{"doc":doc,"offset":0,"text":"EDIT "}}));
        assert_eq!(r["ok"], json!(true), "{r}");
    }
    // Only now, after the insert responses arrived, the file system changes.
    fs::remove_file(w.at("root/a.txt")).unwrap();
    w.link("root/a.txt", "../outside/secret.txt");
    fs::remove_dir_all(w.at("root/sub")).unwrap();
    w.link("root/sub", "../outside");
    fs::remove_file(w.at("root/c.txt")).unwrap();
    w.link("root/c.txt", "real/t.txt");
    assert_eq!(fs::read(w.at("root/a.txt")).unwrap(), SECRET);
    assert_eq!(fs::read(w.at("root/sub/f.txt")).unwrap(), b"outside f\n");
    assert_eq!(fs::read(w.at("root/c.txt")).unwrap(), b"c original\n");
    let before = snapshot(&w.dir);

    // (a) and (b): refused, nothing changes, the documents stay dirty.
    let r = s.ask(json!({"id":21,"command":"doc.save","params":{"doc":1}}));
    assert_rejected(
        &r,
        21,
        "outside_root",
        Some("cannot save a.txt: resolves outside the root"),
    );
    let r = s.ask(json!({"id":22,"command":"doc.save","params":{"doc":2}}));
    assert_rejected(
        &r,
        22,
        "outside_root",
        Some("cannot save sub/f.txt: resolves outside the root"),
    );
    assert_eq!(snapshot(&w.dir), before);
    for doc in [1, 2] {
        let r = s.ask(json!({"id":30 + doc,"command":"doc.info","params":{"doc":doc}}));
        assert_eq!(r["result"]["dirty"], json!(true), "{r}");
    }
    assert_eq!(fs::read(w.at("outside/secret.txt")).unwrap(), SECRET);
    assert_eq!(fs::read(w.at("outside/f.txt")).unwrap(), b"outside f\n");
    assert_eq!(
        fs::read_link(w.at("root/a.txt")).unwrap(),
        Path::new("../outside/secret.txt")
    );

    // (c): a link that stays inside is followed; the link is kept.
    let r = s.ask(json!({"id":23,"command":"doc.save","params":{"doc":3}}));
    assert_eq!(r["ok"], json!(true), "{r}");
    assert_eq!(
        fs::read(w.at("root/real/t.txt")).unwrap(),
        b"EDIT c original\n"
    );
    assert_eq!(
        fs::read_link(w.at("root/c.txt")).unwrap(),
        Path::new("real/t.txt")
    );
    let r = s.ask(json!({"id":33,"command":"doc.info","params":{"doc":3}}));
    assert_eq!(r["result"]["dirty"], json!(false), "{r}");
    assert_no_temp(&w.dir);
    assert_eq!(s.finish().code(), Some(0));
}

// ---------------------------------------------------------------------------------------------
// AC-9

fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn documents_state_the_confinement_rule() {
    let out = kiln(
        &["commands", "--schema"],
        Path::new(env!("CARGO_MANIFEST_DIR")),
    );
    assert_eq!(out.status.code(), Some(0));
    let schema: Value = serde_json::from_slice(&out.stdout).unwrap();
    let rejected = schema["error_kinds"]["rejected"].as_array().unwrap();
    let when = |kind: &str| -> Option<Value> {
        rejected
            .iter()
            .find(|e| e["kind"] == json!(kind))
            .map(|e| e["when"].clone())
    };
    assert_eq!(
        when("outside_root"),
        Some(json!("the path resolves outside the root"))
    );
    assert_eq!(when("not_a_file"), Some(json!(NOT_A_FILE_WHEN)));

    let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
    let protocol = squash(&fs::read_to_string(docs.join("control-protocol.md")).unwrap());
    assert!(
        protocol.contains(&squash(
            "- A path whose location is outside the root is refused (`outside_root`); see \
             `docs/protocol/paths.md`."
        )),
        "control-protocol.md lacks the new Paths bullet"
    );
    assert!(
        !protocol
            .contains("Symbolic links are followed, including links that point outside the root"),
        "control-protocol.md still claims symlinks are followed out of the root"
    );
    for bullet in [
        "- Refused (`rejected` / `invalid_path`) by a purely lexical check: an empty path, an \
         absolute path (leading `/`), a `..` or `.` segment, an empty segment (`a//b`, a trailing \
         `/`), a backslash, a NUL character.",
        "- A save replaces the file by writing a temporary file `.kiln-tmp-<n>` next to it and \
         renaming it over the target (on Unix); through a symlink, the link's target is replaced \
         and the link is kept.",
    ] {
        assert!(
            protocol.contains(&squash(bullet)),
            "control-protocol.md lost a Paths bullet: {bullet}"
        );
    }
    let paths =
        fs::read_to_string(docs.join("protocol/paths.md")).expect("docs/protocol/paths.md exists");
    assert!(paths.contains("outside_root"));
    assert!(paths.contains("not_a_file"));
}
