//! T-037 AC-3 (guards) through `kiln-cli control` and `kiln-cli run`: `doc.open` of a path that
//! is already open does not need the file (ruling D-092 O1). After the file is deleted, replaced
//! by a directory, a FIFO or a link that stays inside the root, made unreadable, or made
//! undecodable, or after a directory on its path is removed or replaced by a file, or after the
//! root is made mode 0311, the re-open still answers the open document unchanged with
//! `already_open: true`, and reads or creates nothing. A session on the in-memory file system is
//! unchanged too (Interface 3). These pass on base (Pinned (g)) and must keep passing. Written
//! by the test author from `tasks/T-037-reopen-checks-confinement.md` alone.
//!
//! Each case runs in its own fixture and session: `doc.open` of the path (doc 1),
//! `doc.insert {"doc":1,"offset":0,"text":"E"}`, the change (checked through `std::fs` to be in
//! place), the re-open (its whole result object asserted), then `doc.info` and `doc.list`.
//! Every case runs even when an earlier one fails; the test then fails naming every failed case.
//! The unprivileged cases run `kiln-cli` as U: uid/gid 65534 through `CommandExt::{uid, gid}`
//! when the test's effective uid is 0 (the fixture is given to U with `lchown`), otherwise the
//! test's own identity; in the same session a negative control (a first `doc.open` of the
//! mode-0000 file `root/locked.txt` is refused `io_error`) shows that U does not bypass
//! permissions, else the case fails. No message text, document text or time is asserted (golden
//! rule 9). Every child process and every response has a 10 s timeout, which only detects hangs:
//! the FIFO case shows by it that the re-open does not block.
#![cfg(test)]

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt, lchown, symlink};
use std::os::unix::process::CommandExt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// Hang detector for every child process and every response; never a duration assertion.
const TIMEOUT: Duration = Duration::from_secs(10);
/// The unprivileged identity used when the test runs as uid 0 (Definitions, "U").
const NOBODY: u32 = 65534;
const SECRET: &[u8] = b"s3cr3t-content\n";
const OUTSIDE_C: &[u8] = b"outside c\n";
/// Bytes that do not decode (AC-3).
const UNDECODABLE: &[u8] = &[0xFF, 0xFE, 0xFF];

// ---------------------------------------------------------------------------------------------
// Identity and fixture.

/// The identity `kiln-cli` runs as.
#[derive(Clone, Copy, Debug)]
struct Identity {
    /// True when the child is dropped to 65534 (the test runs as uid 0).
    drop: bool,
    uid: u32,
    gid: u32,
}

impl Identity {
    /// The test's own identity, unchanged.
    const SELF: Identity = Identity {
        drop: false,
        uid: 0,
        gid: 0,
    };

    /// U, found from the owner of a file just created in `dir` (Definitions).
    fn unprivileged(dir: &Path) -> Identity {
        let probe = dir.join(".euid-probe");
        fs::write(&probe, b"").unwrap();
        let meta = fs::metadata(&probe).unwrap();
        fs::remove_file(&probe).unwrap();
        if meta.uid() == 0 {
            Identity {
                drop: true,
                uid: NOBODY,
                gid: NOBODY,
            }
        } else {
            Identity {
                drop: false,
                uid: meta.uid(),
                gid: meta.gid(),
            }
        }
    }

    fn apply(&self, cmd: &mut Command) {
        if self.drop {
            cmd.uid(self.uid).gid(self.gid);
        }
    }

    /// Give `path` and everything below it (links not followed) to U.
    fn give(&self, path: &Path) {
        if self.drop {
            lchown(path, Some(self.uid), Some(self.gid)).unwrap();
            if fs::symlink_metadata(path).unwrap().is_dir() {
                for e in fs::read_dir(path).unwrap() {
                    self.give(&e.unwrap().path());
                }
            }
        }
    }
}

fn set_mode(path: &Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

/// A fresh `work/` (mode 0755) under `std::env::temp_dir()` holding the spec's fixture, with the
/// root at `work/<root_rel>`: `a.txt`, `b.txt`, `sub/c.txt`, `real/t.txt`; and `work/outside/`
/// with `secret.txt` and `c.txt`. Removed on drop, failed runs included (modes restored first).
struct Work {
    dir: PathBuf,
    root_rel: String,
}

impl Work {
    fn new(name: &str, root_rel: &str) -> Work {
        let dir = std::env::temp_dir().join(format!("k37g-{}-{name}", std::process::id()));
        if fs::symlink_metadata(&dir).is_ok() {
            unlock(&dir);
            fs::remove_dir_all(&dir).unwrap();
        }
        fs::create_dir_all(&dir).unwrap();
        set_mode(&dir, 0o755);
        let dir = fs::canonicalize(&dir).unwrap();
        let w = Work {
            dir,
            root_rel: root_rel.to_owned(),
        };
        w.file(&format!("{root_rel}/a.txt"), b"alpha\n");
        w.file(&format!("{root_rel}/b.txt"), b"bravo\n");
        w.file(&format!("{root_rel}/sub/c.txt"), b"charlie\n");
        w.file(&format!("{root_rel}/real/t.txt"), b"tango\n");
        w.file("outside/secret.txt", SECRET);
        w.file("outside/c.txt", OUTSIDE_C);
        w
    }

    fn at(&self, rel: &str) -> PathBuf {
        self.dir.join(rel)
    }

    /// `work/<root_rel>/<rel>`.
    fn in_root(&self, rel: &str) -> PathBuf {
        self.dir.join(&self.root_rel).join(rel)
    }

    fn root(&self) -> PathBuf {
        self.dir.join(&self.root_rel)
    }

    fn file(&self, at: &str, bytes: &[u8]) {
        let p = self.at(at);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, bytes).unwrap();
    }

    fn text(&self) -> String {
        self.dir.to_string_lossy().into_owned()
    }
}

/// Make every directory under `dir` (and `dir`) searchable and listable again, links not
/// followed.
fn unlock(dir: &Path) {
    let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o755));
    if let Ok(entries) = fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if fs::symlink_metadata(&p).is_ok_and(|m| m.is_dir()) {
                unlock(&p);
            }
        }
    }
}

impl Drop for Work {
    fn drop(&mut self) {
        unlock(&self.dir);
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Every entry under `dir`, recursively, links not followed: type, link target and bytes.
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

// ---------------------------------------------------------------------------------------------
// Running `kiln-cli control`.

fn wait_or_kill(child: &mut Child, what: &str) -> ExitStatus {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("hang detector: {what} did not finish within {TIMEOUT:?}; killed");
        }
        thread::sleep(Duration::from_millis(5));
    }
}

struct Session {
    child: Child,
    stdin: Option<ChildStdin>,
    out: Receiver<String>,
    next_id: u64,
}

impl Session {
    /// `kiln-cli control --root <root>` as `id`, run in `cwd`.
    fn start(id: Identity, cwd: &Path, root: &str) -> Session {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kiln-cli"));
        cmd.args(["control", "--root", root])
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        id.apply(&mut cmd);
        let mut child = cmd.spawn().unwrap_or_else(|e| {
            panic!(
                "precondition: cannot spawn kiln-cli control as uid {}: {e}",
                id.uid
            )
        });
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
        Session {
            child,
            stdin,
            out,
            next_id: 1,
        }
    }

    /// Send one request and wait (at most [`TIMEOUT`], a hang detector) for its response.
    fn ask(&mut self, command: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let request = json!({"id": id, "command": command, "params": params});
        let stdin = self.stdin.as_mut().unwrap();
        writeln!(stdin, "{request}").unwrap();
        stdin.flush().unwrap();
        match self.out.recv_timeout(TIMEOUT) {
            Ok(line) => {
                let r: Value = serde_json::from_str(&line).unwrap();
                assert_eq!(r["id"], json!(id), "response to {request}: {r}");
                r
            }
            Err(e) => {
                let _ = self.child.kill();
                panic!("hang detector: no response to {request} within {TIMEOUT:?}: {e}");
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

// ---------------------------------------------------------------------------------------------
// Assertions shared by the cases.

/// Length in chars of the open document after the insert of `E` (AC: 7, 9 for `sub/c.txt`).
fn edited_len(path: &str) -> usize {
    match path {
        "a.txt" => 7,
        "sub/c.txt" => 9,
        other => panic!("no fixture length for {other}"),
    }
}

/// The exact result of a re-open that succeeds (AC-2 (i), Pins).
fn reopened(path: &str) -> Value {
    json!({"already_open": true, "doc": 1, "encoding": "utf-8", "len_chars": edited_len(path),
           "line_ending": "lf", "mixed_line_endings": false, "path": path})
}

/// `doc.open <path>` (doc 1) and `doc.insert` of `E` at 0.
fn open_and_edit(s: &mut Session, path: &str, case: &str) {
    let r = s.ask("doc.open", json!({ "path": path }));
    assert_eq!(r["ok"], json!(true), "{case}: first doc.open: {r}");
    assert_eq!(r["result"]["doc"], json!(1), "{case}: first doc.open: {r}");
    assert_eq!(
        r["result"]["already_open"],
        json!(false),
        "{case}: first doc.open: {r}"
    );
    let r = s.ask("doc.insert", json!({"doc": 1, "offset": 0, "text": "E"}));
    assert_eq!(r["ok"], json!(true), "{case}: doc.insert: {r}");
}

/// The negative control of the unprivileged cases: a first `doc.open` of the mode-0000 file
/// `root/locked.txt` is refused `io_error` as U.
fn negative_control(s: &mut Session, id: Identity, case: &str) {
    let r = s.ask("doc.open", json!({"path": "locked.txt"}));
    assert!(
        r["ok"] == json!(false) && r["error"]["kind"] == json!("io_error"),
        "{case}: precondition (negative control): as uid {}, a first doc.open of the mode-0000 \
         file locked.txt was not refused io_error, so the identity bypasses permission checks: \
         {r}",
        id.uid
    );
}

/// "Document unchanged": `doc.info` → dirty, the edited length, the undo history kept;
/// `doc.list` → exactly one document, id 1, with `path`.
fn assert_unchanged(s: &mut Session, path: &str, case: &str) {
    let r = s.ask("doc.info", json!({"doc": 1}));
    assert_eq!(r["ok"], json!(true), "{case}: doc.info: {r}");
    assert_eq!(r["result"]["dirty"], json!(true), "{case}: doc.info: {r}");
    assert_eq!(
        r["result"]["len_chars"],
        json!(edited_len(path)),
        "{case}: doc.info: {r}"
    );
    assert_eq!(r["result"]["path"], json!(path), "{case}: doc.info: {r}");
    assert_eq!(
        r["result"]["can_undo"],
        json!(true),
        "{case}: doc.info: {r}"
    );
    let r = s.ask("doc.list", json!({}));
    assert_eq!(r["ok"], json!(true), "{case}: doc.list: {r}");
    let docs = r["result"]["docs"].as_array().unwrap();
    assert_eq!(docs.len(), 1, "{case}: doc.list: {r}");
    assert_eq!(docs[0]["doc"], json!(1), "{case}: doc.list: {r}");
    assert_eq!(docs[0]["path"], json!(path), "{case}: doc.list: {r}");
}

/// Run `run` on every case; fail naming every case that failed.
fn run_cases<T>(cases: &[(&str, T)], run: impl Fn(&T, &str)) {
    let mut failed = Vec::new();
    for (case, c) in cases {
        if catch_unwind(AssertUnwindSafe(|| run(c, case))).is_err() {
            failed.push(*case);
        }
    }
    assert!(
        failed.is_empty(),
        "failed cases (each one's message is above): {failed:?}"
    );
}

/// One kept re-open case: fixture, session as U when `unprivileged` (negative control first),
/// open and edit `path`, `change` (which also checks itself through `std::fs`), then `between`
/// (requests sent after the change, before the re-open), the re-open → the exact result object,
/// the document unchanged, then `after` (checks of the disk). For a case run as the test's own
/// identity the snapshot of `work/` taken after the change is also unchanged by the requests
/// (nothing created, renamed or written: Pinned (b)).
struct Kept<'a> {
    name: &'a str,
    path: &'a str,
    unprivileged: bool,
    change: &'a dyn Fn(&Work),
    between: &'a dyn Fn(&mut Session, &Work, &str),
    after: &'a dyn Fn(&Work, &str),
}

fn nothing_between(_: &mut Session, _: &Work, _: &str) {}
fn nothing_after(_: &Work, _: &str) {}

fn kept_case(c: &Kept, case: &str) {
    let w = Work::new(c.name, "root");
    let id = if c.unprivileged {
        let id = Identity::unprivileged(&w.dir);
        w.file("root/locked.txt", b"locked\n");
        set_mode(&w.in_root("locked.txt"), 0o000);
        id.give(&w.dir);
        id
    } else {
        Identity::SELF
    };
    let mut s = Session::start(id, &w.dir, "root");
    open_and_edit(&mut s, c.path, case);
    if c.unprivileged {
        negative_control(&mut s, id, case);
    }
    (c.change)(&w);
    id.give(&w.root());
    let before = (!c.unprivileged).then(|| snapshot(&w.dir));

    (c.between)(&mut s, &w, case);
    let r = s.ask("doc.open", json!({"path": c.path}));
    assert_eq!(r["ok"], json!(true), "{case}: re-open refused: {r}");
    assert_eq!(r["result"], reopened(c.path), "{case}: re-open: {r}");
    assert_unchanged(&mut s, c.path, case);
    (c.after)(&w, case);
    if let Some(before) = before {
        assert_eq!(snapshot(&w.dir), before, "{case}: work/ changed");
    }
    assert_eq!(s.finish().code(), Some(0), "{case}: control exit");
}

/// `mkfifo <path>` under the hang detector.
fn mkfifo(path: &Path) {
    let mut child = Command::new("mkfifo")
        .arg(path)
        .stdin(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("precondition: cannot spawn mkfifo: {e}"));
    assert!(
        wait_or_kill(&mut child, "mkfifo").success(),
        "mkfifo failed"
    );
}

fn is_fifo(p: &Path) -> bool {
    fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_fifo())
}

fn mode_of(p: &Path) -> u32 {
    fs::symlink_metadata(p).unwrap().mode() & 0o7777
}

fn replace_by_link(w: &Work, at: &str, target: &str) {
    fs::remove_file(w.in_root(at)).unwrap();
    symlink(target, w.in_root(at)).unwrap();
    assert_eq!(
        fs::read_link(w.in_root(at)).unwrap(),
        Path::new(target),
        "fixture"
    );
}

#[test]
fn reopen_does_not_need_the_file() {
    let cases: Vec<(&str, Kept)> = vec![
        (
            "a.txt deleted",
            Kept {
                name: "deleted",
                path: "a.txt",
                unprivileged: false,
                change: &|w| {
                    fs::remove_file(w.in_root("a.txt")).unwrap();
                    assert!(fs::symlink_metadata(w.in_root("a.txt")).is_err(), "fixture");
                },
                between: &nothing_between,
                after: &|w, case| {
                    assert!(
                        fs::symlink_metadata(w.in_root("a.txt")).is_err(),
                        "{case}: a.txt was created"
                    );
                },
            },
        ),
        (
            "a.txt replaced by a directory",
            Kept {
                name: "dir",
                path: "a.txt",
                unprivileged: false,
                change: &|w| {
                    fs::remove_file(w.in_root("a.txt")).unwrap();
                    fs::create_dir(w.in_root("a.txt")).unwrap();
                    assert!(w.in_root("a.txt").is_dir(), "fixture");
                },
                between: &nothing_between,
                after: &nothing_after,
            },
        ),
        (
            "a.txt replaced by a FIFO",
            Kept {
                name: "fifo",
                path: "a.txt",
                unprivileged: false,
                change: &|w| {
                    fs::remove_file(w.in_root("a.txt")).unwrap();
                    mkfifo(&w.in_root("a.txt"));
                    assert!(is_fifo(&w.in_root("a.txt")), "fixture");
                },
                between: &nothing_between,
                after: &|w, case| {
                    assert!(is_fifo(&w.in_root("a.txt")), "{case}: no longer a FIFO");
                },
            },
        ),
        (
            "a.txt replaced by the link b.txt",
            Kept {
                name: "link-b",
                path: "a.txt",
                unprivileged: false,
                change: &|w| {
                    replace_by_link(w, "a.txt", "b.txt");
                    assert_eq!(fs::read(w.in_root("a.txt")).unwrap(), b"bravo\n", "fixture");
                },
                between: &nothing_between,
                after: &|w, case| {
                    assert_eq!(
                        fs::read(w.in_root("b.txt")).unwrap(),
                        b"bravo\n",
                        "{case}: b.txt changed"
                    );
                },
            },
        ),
        (
            "a.txt replaced by the dangling link none.txt",
            Kept {
                name: "dangling",
                path: "a.txt",
                unprivileged: false,
                change: &|w| {
                    replace_by_link(w, "a.txt", "none.txt");
                    assert!(fs::metadata(w.in_root("a.txt")).is_err(), "fixture");
                },
                between: &nothing_between,
                after: &|w, case| {
                    assert!(
                        fs::symlink_metadata(w.in_root("none.txt")).is_err(),
                        "{case}: root/none.txt was created"
                    );
                },
            },
        ),
        (
            "a.txt replaced by the link real/t.txt/",
            Kept {
                name: "trailing",
                path: "a.txt",
                unprivileged: false,
                change: &|w| {
                    replace_by_link(w, "a.txt", "real/t.txt/");
                    assert!(fs::metadata(w.in_root("a.txt")).is_err(), "fixture");
                    assert_eq!(
                        fs::read(w.in_root("real/t.txt")).unwrap(),
                        b"tango\n",
                        "fixture"
                    );
                },
                between: &nothing_between,
                after: &nothing_after,
            },
        ),
        (
            "a.txt overwritten with FF FE FF",
            Kept {
                name: "undecodable",
                path: "a.txt",
                unprivileged: false,
                change: &|w| {
                    fs::write(w.in_root("a.txt"), UNDECODABLE).unwrap();
                    fs::write(w.in_root("bad.txt"), UNDECODABLE).unwrap();
                    assert_eq!(
                        fs::read(w.in_root("a.txt")).unwrap(),
                        UNDECODABLE,
                        "fixture"
                    );
                },
                between: &|s, w, case| {
                    // The bytes do not decode: a first open of a copy is refused decode_error.
                    let r = s.ask("doc.open", json!({"path": "bad.txt"}));
                    assert_eq!(r["ok"], json!(false), "{case}: bad.txt opened: {r}");
                    assert_eq!(r["error"]["code"], json!("rejected"), "{case}: {r}");
                    assert_eq!(r["error"]["kind"], json!("decode_error"), "{case}: {r}");
                    assert!(!r.to_string().contains(&w.text()), "{case}: {r}");
                },
                after: &nothing_after,
            },
        ),
        (
            "sub/c.txt open, sub removed",
            Kept {
                name: "sub-removed",
                path: "sub/c.txt",
                unprivileged: false,
                change: &|w| {
                    fs::remove_dir_all(w.in_root("sub")).unwrap();
                    assert!(fs::symlink_metadata(w.in_root("sub")).is_err(), "fixture");
                },
                between: &nothing_between,
                after: &|w, case| {
                    assert!(
                        fs::symlink_metadata(w.in_root("sub")).is_err(),
                        "{case}: sub was created"
                    );
                },
            },
        ),
        (
            "sub/c.txt open, sub replaced by a regular file",
            Kept {
                name: "sub-file",
                path: "sub/c.txt",
                unprivileged: false,
                change: &|w| {
                    fs::remove_dir_all(w.in_root("sub")).unwrap();
                    fs::write(w.in_root("sub"), b"now a file\n").unwrap();
                    assert!(w.in_root("sub").is_file(), "fixture");
                },
                between: &nothing_between,
                after: &nothing_after,
            },
        ),
        (
            "as U: a.txt made mode 0000",
            Kept {
                name: "u-file-0000",
                path: "a.txt",
                unprivileged: true,
                change: &|w| {
                    set_mode(&w.in_root("a.txt"), 0o000);
                    assert_eq!(mode_of(&w.in_root("a.txt")), 0o000, "fixture");
                },
                between: &nothing_between,
                after: &nothing_after,
            },
        ),
        (
            "as U: a.txt replaced by a directory of mode 0000",
            Kept {
                name: "u-dir-0000",
                path: "a.txt",
                unprivileged: true,
                change: &|w| {
                    fs::remove_file(w.in_root("a.txt")).unwrap();
                    fs::create_dir(w.in_root("a.txt")).unwrap();
                    set_mode(&w.in_root("a.txt"), 0o000);
                    assert!(w.in_root("a.txt").is_dir(), "fixture");
                    assert_eq!(mode_of(&w.in_root("a.txt")), 0o000, "fixture");
                },
                between: &nothing_between,
                after: &nothing_after,
            },
        ),
        (
            "as U: the root made mode 0311",
            Kept {
                name: "u-root-0311",
                path: "a.txt",
                unprivileged: true,
                change: &|w| {
                    set_mode(&w.root(), 0o311);
                    assert_eq!(mode_of(&w.root()), 0o311, "fixture");
                },
                between: &nothing_between,
                after: &nothing_after,
            },
        ),
    ];
    run_cases(&cases, kept_case);
}

#[test]
fn reopen_in_a_memory_session_is_unchanged() {
    let w = Work::new("memory", "root");
    let script = [
        json!({"id": 1, "command": "doc.open", "params": {"path": "a.txt"}}),
        json!({"id": 2, "command": "doc.insert",
               "params": {"doc": 1, "offset": 0, "text": "E"}}),
        json!({"id": 3, "command": "doc.open", "params": {"path": "a.txt"}}),
    ]
    .iter()
    .map(|v| format!("{v}\n"))
    .collect::<String>();
    fs::write(w.at("s.jsonl"), script).unwrap();
    fs::write(
        w.at("m.json"),
        json!({"files": [{"path": "a.txt", "text": "alpha\n"}]}).to_string(),
    )
    .unwrap();
    // The on-disk a.txt is removed: the session must not depend on it.
    fs::remove_file(w.in_root("a.txt")).unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
        .args(["run", "s.jsonl", "--memory-seed", "m.json"])
        .current_dir(&w.dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("precondition: cannot spawn kiln-cli run: {e}"));
    let mut stdout = child.stdout.take().unwrap();
    let out = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });
    let status = wait_or_kill(&mut child, "kiln-cli run --memory-seed");
    let out = String::from_utf8(out.join().unwrap()).unwrap();
    assert_eq!(status.code(), Some(0), "kiln-cli run: {out}");
    let lines: Vec<Value> = out
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 3, "kiln-cli run: {out}");
    for (i, r) in lines.iter().enumerate() {
        assert_eq!(r["id"], json!(i + 1), "{r}");
        assert_eq!(r["ok"], json!(true), "{r}");
    }
    assert_eq!(
        lines[2]["result"],
        reopened("a.txt"),
        "re-open: {}",
        lines[2]
    );
    assert!(
        fs::symlink_metadata(w.in_root("a.txt")).is_err(),
        "the memory session wrote root/a.txt"
    );
}
