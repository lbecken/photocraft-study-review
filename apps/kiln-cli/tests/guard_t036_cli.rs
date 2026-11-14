//! T-036 guards through `kiln-cli control` (Interface 3 and 4): a root given through a link
//! that is retargeted between two requests (AC-3, TG-3) and a root whose path leads nowhere
//! (AC-4, TG-4) are refused with `rejected` / `outside_root` by `doc.open`, `doc.save` (of the
//! document opened before) and `project.tree`, the document stays dirty, nothing is created or
//! changed, and once the path leads to the root directory again the save works there. Base
//! already behaves so (T-008 r2 probes (e), (h); T-019 r1 "Root renamed away"). Written by the
//! test author from `tasks/T-036-root-identity-follow-ups.md` alone.
//!
//! Responses are asserted by `ok`, `code`, `kind` and the result fields named by the spec; no
//! message text. Every response is awaited for at most 10 s and the session for at most 10 s
//! after stdin closes; these only detect hangs.
#![cfg(test)]

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const SECRET: &[u8] = b"s3cr3t-content\n";
const F_ORIGINAL: &[u8] = b"f original\n";
const OTHER_F: &[u8] = b"other f\n";
const F_EDITED: &[u8] = b"EDIT f original\n";
const G_BYTES: &[u8] = b"g original\n";
/// Hang detector for the session and for each response; never a duration assertion.
const TIMEOUT: Duration = Duration::from_secs(10);

// ---------------------------------------------------------------------------------------------
// Fixtures.

/// A fresh `work/` with `root/`, `outside/` and `outside/secret.txt`; removed on drop.
struct Work {
    dir: PathBuf,
}

impl Work {
    fn new(name: &str) -> Work {
        let dir = std::env::temp_dir().join(format!("k36d-{}-{name}", std::process::id()));
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

/// No `new.txt` and no `.kiln-tmp-*` anywhere under `dir`.
fn assert_nothing_created(dir: &Path, note: &str) {
    for name in snapshot(dir).keys() {
        let last = name.rsplit('/').next().unwrap_or(name);
        assert!(
            !last.starts_with(".kiln-tmp-") && last != "new.txt",
            "{note}: created {name}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The `control` driver.

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
}

impl Session {
    /// `kiln-cli control --root <root>`, run in `cwd`.
    fn start(cwd: &Path, root: &Path) -> Session {
        let mut child = Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
            .arg("control")
            .arg("--root")
            .arg(root)
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

    /// Send one request and wait (at most [`TIMEOUT`], a hang detector) for its response.
    fn ask(&mut self, request: Value) -> Value {
        let stdin = self.stdin.as_mut().unwrap();
        writeln!(stdin, "{request}").unwrap();
        stdin.flush().unwrap();
        match self.out.recv_timeout(TIMEOUT) {
            Ok(line) => serde_json::from_str(&line).unwrap(),
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

/// `r` answers request `id` with `rejected` / `outside_root` and names no host work path.
fn assert_outside(r: &Value, id: u64, w: &Work, note: &str) {
    assert_eq!(r["id"], json!(id), "{note}: {r}");
    assert_eq!(r["ok"], json!(false), "{note}: request {id} not refused: {r}");
    assert_eq!(r["error"]["code"], json!("rejected"), "{note}: {r}");
    assert_eq!(r["error"]["kind"], json!("outside_root"), "{note}: {r}");
    assert!(
        !r.to_string().contains(&w.text()),
        "{note}: names the work path: {r}"
    );
}

fn assert_ok(r: &Value, id: u64, note: &str) {
    assert_eq!(r["id"], json!(id), "{note}: {r}");
    assert_eq!(r["ok"], json!(true), "{note}: request {id} failed: {r}");
}

/// Open `f.txt` as document 1 and edit it (ids 1 and 2).
fn open_and_edit(s: &mut Session, note: &str) {
    let r = s.ask(json!({"id":1,"command":"doc.open","params":{"path":"f.txt"}}));
    assert_ok(&r, 1, note);
    assert_eq!(r["result"]["doc"], json!(1), "{note}: {r}");
    let r = s.ask(json!({"id":2,"command":"doc.insert",
                         "params":{"doc":1,"offset":0,"text":"EDIT "}}));
    assert_ok(&r, 2, note);
}

/// Requests from id 10 on: each of `opens` refused, `project.tree {}` refused, the save of
/// document 1 refused, and document 1 still dirty.
fn assert_all_refused(s: &mut Session, w: &Work, opens: &[&str], note: &str) {
    let mut id = 10;
    for p in opens {
        let r = s.ask(json!({"id":id,"command":"doc.open","params":{"path":p}}));
        assert_outside(&r, id, w, &format!("{note}: doc.open {p}"));
        id += 1;
    }
    let r = s.ask(json!({"id":id,"command":"project.tree","params":{}}));
    assert_outside(&r, id, w, &format!("{note}: project.tree"));
    id += 1;
    let r = s.ask(json!({"id":id,"command":"doc.save","params":{"doc":1}}));
    assert_outside(&r, id, w, &format!("{note}: doc.save of document 1"));
    id += 1;
    let r = s.ask(json!({"id":id,"command":"doc.info","params":{"doc":1}}));
    assert_ok(&r, id, note);
    assert_eq!(r["result"]["dirty"], json!(true), "{note}: {r}");
}

// ---------------------------------------------------------------------------------------------
// AC-3 (item 3, TG-3).

#[test]
fn retargeted_root_link_is_refused_through_control() {
    let w = Work::new("retarget");
    w.file("root/f.txt", F_ORIGINAL);
    w.file("other/secret.txt", SECRET);
    w.file("other/f.txt", OTHER_F);
    symlink("root", w.at("rootlink")).unwrap();
    let note = "rootlink -> other";

    let mut s = Session::start(&w.dir, Path::new("rootlink"));
    open_and_edit(&mut s, "start");

    // Retarget the link between two requests, and show the other directory is really reached.
    fs::remove_file(w.at("rootlink")).unwrap();
    symlink("other", w.at("rootlink")).unwrap();
    assert_eq!(fs::read(w.at("rootlink/secret.txt")).unwrap(), SECRET);
    assert_eq!(fs::read(w.at("rootlink/f.txt")).unwrap(), OTHER_F);
    let before = snapshot(&w.dir);

    assert_all_refused(&mut s, &w, &["secret.txt"], note);
    assert_eq!(snapshot(&w.dir), before, "{note}: work changed");
    assert_eq!(fs::read(w.at("root/f.txt")).unwrap(), F_ORIGINAL);
    assert_eq!(fs::read(w.at("other/f.txt")).unwrap(), OTHER_F);
    assert_nothing_created(&w.dir, note);

    // Point the link back: the save of document 1 works there again.
    fs::remove_file(w.at("rootlink")).unwrap();
    symlink("root", w.at("rootlink")).unwrap();
    let r = s.ask(json!({"id":30,"command":"doc.save","params":{"doc":1}}));
    assert_ok(&r, 30, "rootlink -> root again: doc.save");
    let r = s.ask(json!({"id":31,"command":"doc.info","params":{"doc":1}}));
    assert_eq!(r["result"]["dirty"], json!(false), "{r}");
    assert_eq!(s.finish().code(), Some(0));
    assert_eq!(fs::read(w.at("root/f.txt")).unwrap(), F_EDITED);
    assert_eq!(fs::read(w.at("other/f.txt")).unwrap(), OTHER_F);
    assert_eq!(fs::read(w.at("other/secret.txt")).unwrap(), SECRET);
    assert_nothing_created(&w.dir, "rootlink -> root again");
}

// ---------------------------------------------------------------------------------------------
// AC-4 (item 4, TG-4).

/// The three ways the root path comes to lead nowhere, with nothing put in its place.
#[derive(Clone, Copy, Debug)]
enum Vanish {
    /// (i) `root` renamed to `root.away`.
    RenameRoot,
    /// (ii) the root at `work/p/root`, `p` renamed to `p.away`.
    RenameParent,
    /// (iii) `root` removed with `remove_dir_all`.
    Remove,
}

fn vanished_root_case(case: Vanish) {
    let note = format!("{case:?}");
    let w = Work::new(&format!("vanish-{note}"));
    let (root_rel, away, back) = match case {
        Vanish::RenameRoot => ("root", Some(("root", "root.away")), "root.away/f.txt"),
        Vanish::RenameParent => ("p/root", Some(("p", "p.away")), "p.away/root/f.txt"),
        Vanish::Remove => ("root", None, ""),
    };
    w.file(&format!("{root_rel}/f.txt"), F_ORIGINAL);
    w.file(&format!("{root_rel}/g.txt"), G_BYTES);
    let root = w.at(root_rel);

    let mut s = Session::start(&w.dir, Path::new(root_rel));
    open_and_edit(&mut s, &note);

    // The change, between two requests; the root path now leads nowhere.
    match away {
        Some((from, to)) => fs::rename(w.at(from), w.at(to)).unwrap(),
        None => fs::remove_dir_all(&root).unwrap(),
    }
    assert!(
        fs::symlink_metadata(&root).is_err(),
        "{note}: fixture: the root path still leads somewhere"
    );
    if away.is_some() {
        assert_eq!(fs::read(w.at(back)).unwrap(), F_ORIGINAL, "{note}: fixture");
    }
    let before = snapshot(&w.dir);

    // `doc.open f.txt` is not sent: f.txt is document 1, and `doc.open` of an open document
    // returns it without reading the disk (`already_open`); see the QUESTION in the T-036 log.
    // `g.txt` is an existing file of the root that is not open.
    assert_all_refused(&mut s, &w, &["g.txt", "missing.txt"], &note);
    assert_eq!(snapshot(&w.dir), before, "{note}: work changed");
    assert!(
        fs::symlink_metadata(&root).is_err(),
        "{note}: the root path was created"
    );
    assert_nothing_created(&w.dir, &note);

    // Moved back: the save works into root/f.txt.
    if let Some((from, to)) = away {
        fs::rename(w.at(to), w.at(from)).unwrap();
        let r = s.ask(json!({"id":30,"command":"doc.save","params":{"doc":1}}));
        assert_ok(&r, 30, &format!("{note}: moved back: doc.save"));
        assert_eq!(fs::read(root.join("f.txt")).unwrap(), F_EDITED, "{note}");
        assert_nothing_created(&w.dir, &format!("{note}: moved back"));
    }
    assert_eq!(s.finish().code(), Some(0), "{note}");
}

#[test]
fn vanished_root_is_refused_through_control() {
    for case in [Vanish::RenameRoot, Vanish::RenameParent, Vanish::Remove] {
        vanished_root_case(case);
    }
}
