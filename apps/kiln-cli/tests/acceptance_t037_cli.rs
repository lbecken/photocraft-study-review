//! T-037 AC-1, AC-2 (i)–(iii) and AC-4 through `kiln-cli control`: `doc.open` of a path that is
//! already open checks the root and the path's location (Interface 1, ruling D-092 O1). A path
//! that now leads outside the root, or a root path that no longer leads to the root, is refused
//! `rejected` / `outside_root`; a walk that fails inside the root (a link loop, a directory on
//! the way that cannot be searched) is refused `rejected` / `io_error` (ruling D-092 on Q-2). A
//! refused re-open leaves the document as it was (Interface 2). Written by the test author from
//! `tasks/T-037-reopen-checks-confinement.md` alone.
//!
//! Each case runs in its own fixture and session: `doc.open` of the path (doc 1),
//! `doc.insert {"doc":1,"offset":0,"text":"E"}`, the change (made by the test, between two
//! requests, and checked through `std::fs` to be in place), the re-open, then `doc.info` and
//! `doc.list`. Every case of a test runs even when an earlier one fails; the test then fails
//! naming every failed case (each case's own panic message is printed above it).
//!
//! Refusals are asserted by `ok`, `error.code` and `error.kind`, plus one absence check: the
//! serialised response does not contain the work directory's path. No message text, document
//! text or time is asserted (golden rule 9). The unprivileged cases of AC-4 run `kiln-cli` as
//! U: uid/gid 65534 through `CommandExt::{uid, gid}` when the test's effective uid is 0 (the
//! fixture is given to U with `lchown`), otherwise the test's own identity; in the same session
//! a negative control (a first `doc.open` of the mode-0000 file `root/locked.txt` is refused
//! `io_error`) shows that U does not bypass permissions, else the case fails. Every child
//! process and every response has a 10 s timeout, which only detects hangs.
#![cfg(test)]

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt, lchown, symlink};
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
        let dir = std::env::temp_dir().join(format!("k37c-{}-{name}", std::process::id()));
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

/// `r` is `rejected` / `kind`, and names no part of the work path.
fn assert_refused(r: &Value, kind: &str, w: &Work, case: &str) {
    assert_eq!(r["ok"], json!(false), "{case}: re-open not refused: {r}");
    assert_eq!(r["error"]["code"], json!("rejected"), "{case}: {r}");
    assert_eq!(r["error"]["kind"], json!(kind), "{case}: {r}");
    assert!(
        !r.to_string().contains(&w.text()),
        "{case}: the response names the work path: {r}"
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

/// Run every case; fail naming every case that failed.
fn run_cases(cases: &[(&str, &dyn Fn(&str))]) {
    let mut failed = Vec::new();
    for (case, f) in cases {
        if catch_unwind(AssertUnwindSafe(|| f(case))).is_err() {
            failed.push(*case);
        }
    }
    assert!(
        failed.is_empty(),
        "failed cases (each one's message is above): {failed:?}"
    );
}

/// One refused re-open case: fixture with the root at `root_rel`, session as `id` (U when
/// `unprivileged`), open and edit `path`, `change` (which also checks itself through `std::fs`),
/// re-open → `rejected` / `kind`, document unchanged. When `snapshot_work`, the snapshot of
/// `work/` taken after the change is unchanged by the requests. Returns the session and fixture
/// for any follow-up.
struct Refused<'a> {
    name: &'a str,
    root_rel: &'a str,
    path: &'a str,
    unprivileged: bool,
    snapshot_work: bool,
    kind: &'a str,
    change: &'a dyn Fn(&Work),
}

fn refused_case(c: &Refused, case: &str) -> (Session, Work) {
    let w = Work::new(c.name, c.root_rel);
    let id = if c.unprivileged {
        let id = Identity::unprivileged(&w.dir);
        w.file(&format!("{}/locked.txt", c.root_rel), b"locked\n");
        set_mode(&w.in_root("locked.txt"), 0o000);
        id.give(&w.dir);
        id
    } else {
        Identity::SELF
    };
    let mut s = Session::start(id, &w.dir, c.root_rel);
    open_and_edit(&mut s, c.path, case);
    if c.unprivileged {
        negative_control(&mut s, id, case);
    }
    (c.change)(&w);
    let before = c.snapshot_work.then(|| snapshot(&w.dir));

    let r = s.ask("doc.open", json!({"path": c.path}));
    assert_refused(&r, c.kind, &w, case);
    assert_unchanged(&mut s, c.path, case);
    if let Some(before) = before {
        assert_eq!(snapshot(&w.dir), before, "{case}: work/ changed");
    }
    (s, w)
}

// ---------------------------------------------------------------------------------------------
// AC-1: the path now leads outside the root.

fn outside_case(name: &str, path: &str, change: &dyn Fn(&Work), case: &str) {
    let (s, w) = refused_case(
        &Refused {
            name,
            root_rel: "root",
            path,
            unprivileged: false,
            snapshot_work: true,
            kind: "outside_root",
            change,
        },
        case,
    );
    assert!(
        fs::symlink_metadata(w.at("outside/none.txt")).is_err(),
        "{case}: outside/none.txt was created"
    );
    assert_eq!(s.finish().code(), Some(0), "{case}: control exit");
}

fn replace_by_link(w: &Work, at: &str, target: &Path) {
    let p = w.in_root(at);
    if fs::symlink_metadata(&p).unwrap().is_dir() {
        fs::remove_dir_all(&p).unwrap();
    } else {
        fs::remove_file(&p).unwrap();
    }
    symlink(target, &p).unwrap();
}

#[test]
fn reopen_of_a_path_now_outside_is_refused() {
    run_cases(&[
        ("AC-1 (i) a.txt -> ../outside/secret.txt", &|case| {
            outside_case(
                "ac1-rel",
                "a.txt",
                &|w| {
                    replace_by_link(w, "a.txt", Path::new("../outside/secret.txt"));
                    assert_eq!(fs::read(w.in_root("a.txt")).unwrap(), SECRET, "fixture");
                },
                case,
            );
        }),
        ("AC-1 (ii) a.txt -> <abs>/outside/secret.txt", &|case| {
            outside_case(
                "ac1-abs",
                "a.txt",
                &|w| {
                    let target = w.at("outside/secret.txt");
                    assert!(target.is_absolute(), "fixture");
                    replace_by_link(w, "a.txt", &target);
                    assert_eq!(fs::read(w.in_root("a.txt")).unwrap(), SECRET, "fixture");
                },
                case,
            );
        }),
        (
            "AC-1 (iii) a.txt -> ../outside/none.txt (dangling)",
            &|case| {
                outside_case(
                    "ac1-dangling",
                    "a.txt",
                    &|w| {
                        replace_by_link(w, "a.txt", Path::new("../outside/none.txt"));
                        assert!(
                            fs::symlink_metadata(w.in_root("a.txt"))
                                .unwrap()
                                .file_type()
                                .is_symlink(),
                            "fixture"
                        );
                        assert!(fs::metadata(w.in_root("a.txt")).is_err(), "fixture");
                    },
                    case,
                );
            },
        ),
        ("AC-1 (iv) sub/c.txt open, sub -> ../outside", &|case| {
            outside_case(
                "ac1-sub",
                "sub/c.txt",
                &|w| {
                    replace_by_link(w, "sub", Path::new("../outside"));
                    assert_eq!(
                        fs::read(w.in_root("sub/c.txt")).unwrap(),
                        OUTSIDE_C,
                        "fixture"
                    );
                },
                case,
            );
        }),
    ]);
}

// ---------------------------------------------------------------------------------------------
// AC-2 (i)–(iii): the root path no longer leads to the root.

#[test]
fn reopen_after_the_root_moved_is_refused() {
    run_cases(&[
        ("AC-2 (i) root renamed to root.away, then back", &|case| {
            let (mut s, w) = refused_case(
                &Refused {
                    name: "ac2-rename",
                    root_rel: "root",
                    path: "a.txt",
                    unprivileged: false,
                    snapshot_work: true,
                    kind: "outside_root",
                    change: &|w| {
                        fs::rename(w.at("root"), w.at("root.away")).unwrap();
                        assert!(fs::symlink_metadata(w.at("root")).is_err(), "fixture");
                        assert_eq!(
                            fs::read(w.at("root.away/a.txt")).unwrap(),
                            b"alpha\n",
                            "fixture"
                        );
                    },
                },
                case,
            );
            // Moved back: the re-open answers the open document again.
            fs::rename(w.at("root.away"), w.at("root")).unwrap();
            let r = s.ask("doc.open", json!({"path": "a.txt"}));
            assert_eq!(r["ok"], json!(true), "{case}: moved back: {r}");
            assert_eq!(r["result"], reopened("a.txt"), "{case}: moved back: {r}");
            assert_unchanged(&mut s, "a.txt", &format!("{case}: moved back"));
            assert_eq!(s.finish().code(), Some(0), "{case}: control exit");
        }),
        (
            "AC-2 (ii) root renamed to root.away, root -> outside",
            &|case| {
                let (s, _w) = refused_case(
                    &Refused {
                        name: "ac2-link",
                        root_rel: "root",
                        path: "a.txt",
                        unprivileged: false,
                        snapshot_work: true,
                        kind: "outside_root",
                        change: &|w| {
                            fs::rename(w.at("root"), w.at("root.away")).unwrap();
                            symlink("outside", w.at("root")).unwrap();
                            assert_eq!(
                                fs::read(w.at("root/secret.txt")).unwrap(),
                                SECRET,
                                "fixture"
                            );
                        },
                    },
                    case,
                );
                assert_eq!(s.finish().code(), Some(0), "{case}: control exit");
            },
        ),
        ("AC-2 (iii) --root p/root, p renamed to p.away", &|case| {
            let (s, _w) = refused_case(
                &Refused {
                    name: "ac2-parent",
                    root_rel: "p/root",
                    path: "a.txt",
                    unprivileged: false,
                    snapshot_work: true,
                    kind: "outside_root",
                    change: &|w| {
                        fs::rename(w.at("p"), w.at("p.away")).unwrap();
                        assert!(fs::symlink_metadata(w.at("p")).is_err(), "fixture");
                        assert_eq!(
                            fs::read(w.at("p.away/root/a.txt")).unwrap(),
                            b"alpha\n",
                            "fixture"
                        );
                    },
                },
                case,
            );
            assert_eq!(s.finish().code(), Some(0), "{case}: control exit");
        }),
    ]);
}

// ---------------------------------------------------------------------------------------------
// AC-4: the walk fails inside the root.

#[test]
fn reopen_whose_walk_fails_inside_is_io_error() {
    run_cases(&[
        ("AC-4 (i) a.txt -> loop2, loop2 -> a.txt", &|case| {
            let (s, _w) = refused_case(
                &Refused {
                    name: "ac4-loop",
                    root_rel: "root",
                    path: "a.txt",
                    unprivileged: false,
                    snapshot_work: true,
                    kind: "io_error",
                    change: &|w| {
                        replace_by_link(w, "a.txt", Path::new("loop2"));
                        symlink("a.txt", w.in_root("loop2")).unwrap();
                        assert!(
                            fs::symlink_metadata(w.in_root("a.txt"))
                                .unwrap()
                                .file_type()
                                .is_symlink(),
                            "fixture"
                        );
                        assert!(
                            fs::metadata(w.in_root("a.txt")).is_err(),
                            "fixture: no loop"
                        );
                    },
                },
                case,
            );
            assert_eq!(s.finish().code(), Some(0), "{case}: control exit");
        }),
        ("AC-4 (ii) as U: sub/c.txt open, sub made 0600", &|case| {
            let (s, w) = refused_case(
                &Refused {
                    name: "ac4-sub",
                    root_rel: "root",
                    path: "sub/c.txt",
                    unprivileged: true,
                    snapshot_work: false,
                    kind: "io_error",
                    change: &|w| {
                        set_mode(&w.in_root("sub"), 0o600);
                        let mode = fs::symlink_metadata(w.in_root("sub")).unwrap().mode();
                        assert_eq!(mode & 0o7777, 0o600, "fixture");
                    },
                },
                case,
            );
            set_mode(&w.in_root("sub"), 0o755);
            assert_eq!(s.finish().code(), Some(0), "{case}: control exit");
        }),
        ("AC-4 (iii) as U: root made 0000 after the start", &|case| {
            let (s, w) = refused_case(
                &Refused {
                    name: "ac4-root",
                    root_rel: "root",
                    path: "a.txt",
                    unprivileged: true,
                    snapshot_work: false,
                    kind: "io_error",
                    change: &|w| {
                        set_mode(&w.root(), 0o000);
                        let mode = fs::symlink_metadata(w.root()).unwrap().mode();
                        assert_eq!(mode & 0o7777, 0o000, "fixture");
                    },
                },
                case,
            );
            set_mode(&w.root(), 0o755);
            assert_eq!(s.finish().code(), Some(0), "{case}: control exit");
        }),
    ]);
}
