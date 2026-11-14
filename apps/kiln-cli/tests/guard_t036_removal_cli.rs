//! T-036 AC-2 (item 2, TG-1) through `kiln-cli control`, in a test binary of its own: the root
//! directory removed between two requests of one session and its path replaced, (i) by a link
//! to a new directory or (ii) by a new directory at the same path, and (iii) as the
//! unprivileged identity U, a root of mode 0711 at start made 0311 and then replaced as in (i).
//! `doc.open`, `doc.save` (of the document opened before) and `project.tree` give `rejected` /
//! `outside_root`, the document stays dirty, and nothing is read, listed or written in the
//! replacement (Interface 1 and 2, T-008 v9). Written by the test author from
//! `tasks/T-036-root-identity-follow-ups.md` alone.
//!
//! Making detection reliable (AC-2, required):
//! - every test holds the file-level lock [`LOCK`] for its whole body, so no two
//!   remove-and-replace windows of this binary overlap;
//! - each test runs up to [`K`] trials, each with a fresh `work/` and a new session;
//! - after the removal the test itself creates `work/c0`, `work/c1`, … one at a time until one
//!   has the removed root's (device, inode), or [`B`] exist; that directory (or the last) is the
//!   replacement;
//! - each trial writes one line to stderr (case, trial, old and new (device, inode), reused or
//!   not), and every assertion message includes it;
//! - the refusals are asserted in every trial whatever the file system did; a trial whose
//!   replacement got the removed root's number ends the loop with `break`.
//!
//! Case (iii) runs `kiln-cli` as U: uid/gid 65534 through `CommandExt::{uid, gid}` when the
//! test's effective uid is 0 (the fixture is given to U with `lchown`), otherwise the test's own
//! identity; a positive and a negative control run first in every trial. Responses are asserted
//! by `ok`, `code`, `kind` and `dirty`; no message text. Every child process and every response
//! has a 10 s timeout, which only detects hangs.
#![cfg(test)]

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt, lchown, symlink};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const SECRET: &[u8] = b"s3cr3t-content\n";
const F_ORIGINAL: &[u8] = b"f original\n";
/// Hang detector for every child process and every response; never a duration assertion.
const TIMEOUT: Duration = Duration::from_secs(10);
/// Trials per test (AC-2: K >= 5).
const K: usize = 5;
/// Directories created per reuse search at most (AC-2: B >= 64).
const B: usize = 64;
/// The unprivileged identity used when the test runs as uid 0 (Definitions, "U").
const NOBODY: u32 = 65534;

/// Serialises the remove-and-replace windows of this binary (poison-tolerant).
static LOCK: Mutex<()> = Mutex::new(());

// ---------------------------------------------------------------------------------------------
// Identity and fixture.

/// The identity `kiln-cli` runs as.
#[derive(Clone, Copy, Debug)]
struct Identity {
    /// True when the child is dropped to 65534 (the test runs as uid 0, case (iii) only).
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

/// A fresh `work/` (mode 0755) under `std::env::temp_dir()`, with `root/`, `outside/` and
/// `outside/secret.txt`; removed on drop, failed runs included (modes are restored first).
struct Work {
    dir: PathBuf,
}

impl Work {
    fn new(name: &str) -> Work {
        let dir = std::env::temp_dir().join(format!("k36e-{}-{name}", std::process::id()));
        if dir.exists() {
            unlock(&dir);
            fs::remove_dir_all(&dir).unwrap();
        }
        fs::create_dir_all(&dir).unwrap();
        set_mode(&dir, 0o755);
        let dir = fs::canonicalize(&dir).unwrap();
        fs::create_dir(dir.join("root")).unwrap();
        fs::create_dir(dir.join("outside")).unwrap();
        fs::write(dir.join("outside/secret.txt"), SECRET).unwrap();
        Work { dir }
    }

    fn at(&self, rel: &str) -> PathBuf {
        self.dir.join(rel)
    }

    fn file(&self, at: &str, bytes: &[u8], mode: u32) {
        let p = self.at(at);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, bytes).unwrap();
        set_mode(&p, mode);
    }

    fn text(&self) -> String {
        self.dir.to_string_lossy().into_owned()
    }
}

/// Make every directory under `dir` (and `dir`) listable again, links not followed.
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

fn id_of(p: &Path) -> (u64, u64) {
    let m = fs::metadata(p).unwrap();
    (m.dev(), m.ino())
}

// ---------------------------------------------------------------------------------------------
// Running `kiln-cli`.

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

/// `kiln-cli exec doc.open {"path": <path>} --root <root>` as `id` in `cwd`: exit code and
/// stdout.
fn exec_open(id: Identity, cwd: &Path, root: &str, path: &str) -> (Option<i32>, Vec<u8>) {
    let params = json!({ "path": path }).to_string();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kiln-cli"));
    cmd.args(["exec", "doc.open", &params, "--root", root])
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    id.apply(&mut cmd);
    let mut child = cmd
        .spawn()
        .unwrap_or_else(|e| panic!("precondition: cannot spawn kiln-cli as uid {}: {e}", id.uid));
    let mut stdout = child.stdout.take().unwrap();
    let out = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });
    let status = wait_or_kill(&mut child, "kiln-cli exec doc.open");
    (status.code(), out.join().unwrap())
}

struct Session {
    child: Child,
    stdin: Option<ChildStdin>,
    out: Receiver<String>,
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

// ---------------------------------------------------------------------------------------------
// AC-2 through control.

/// One trial; true iff the replacement got the removed root's (device, inode).
///
/// `unreadable_after_start` is case (iii): `kiln-cli` runs as U, the root is 0711 at start and
/// made 0311 before it is removed and replaced by a link (`link` is then true).
fn trial(case: &str, link: bool, unreadable_after_start: bool, n: usize) -> bool {
    let w = Work::new(&format!(
        "{}-{n}",
        match (unreadable_after_start, link) {
            (true, _) => "mode",
            (false, true) => "link",
            (false, false) => "dir",
        }
    ));
    let id = if unreadable_after_start {
        Identity::unprivileged(&w.dir)
    } else {
        Identity::SELF
    };
    w.file("root/f.txt", F_ORIGINAL, 0o644);
    let root = w.at("root");
    if unreadable_after_start {
        w.file("open/f.txt", F_ORIGINAL, 0o644);
        w.file("open/locked.txt", F_ORIGINAL, 0o000);
        id.give(&w.dir);
        set_mode(&w.at("open"), 0o711);
        set_mode(&root, 0o711);
        // Controls, as U: U can run the binary and reach the fixture, and has no override.
        let (code, out) = exec_open(id, &w.dir, "open", "f.txt");
        assert!(
            code == Some(0) && !out.is_empty(),
            "precondition (positive control): as uid {}, `exec doc.open f.txt --root work/open` \
             did not succeed (exit {code:?}), so U cannot run kiln-cli or reach the fixture",
            id.uid
        );
        let (code, out) = exec_open(id, &w.dir, "open", "locked.txt");
        assert!(
            code == Some(2) && out.is_empty(),
            "precondition (negative control): as uid {}, `exec doc.open locked.txt` (mode 0000) \
             did not fail (exit {code:?}), so U overrides permission checks",
            id.uid
        );
    }

    let mut s = Session::start(id, &w.dir, "root");
    let r = s.ask(json!({"id":1,"command":"doc.open","params":{"path":"f.txt"}}));
    assert_eq!(r["ok"], json!(true), "{case} trial {n}: {r}");
    assert_eq!(r["result"]["doc"], json!(1), "{case} trial {n}: {r}");
    let r = s.ask(json!({"id":2,"command":"doc.insert",
                         "params":{"doc":1,"offset":0,"text":"EDIT "}}));
    assert_eq!(r["ok"], json!(true), "{case} trial {n}: {r}");

    // Between two requests: (iii) the mode change; then remove the root, and search for a
    // directory that gets its number.
    let old = id_of(&root);
    if unreadable_after_start {
        set_mode(&root, 0o311);
        // Removed entry by entry: the directory can no longer be listed by its owner.
        fs::remove_file(root.join("f.txt")).unwrap();
        fs::remove_dir(&root).unwrap();
    } else {
        fs::remove_dir_all(&root).unwrap();
    }
    let mut made = 0;
    let mut reused = false;
    while made < B && !reused {
        let c = w.at(&format!("c{made}"));
        fs::create_dir(&c).unwrap();
        made += 1;
        reused = id_of(&c) == old;
    }
    let name = format!("c{}", made - 1);
    let replacement = if link {
        symlink(&name, &root).unwrap();
        w.at(&name)
    } else {
        fs::rename(w.at(&name), &root).unwrap();
        root.clone()
    };
    fs::write(replacement.join("secret.txt"), SECRET).unwrap();
    for i in 0..made {
        let c = w.at(&format!("c{i}"));
        if c.exists() {
            id.give(&c);
        }
    }
    id.give(&root);
    let new = id_of(&root);
    let line = format!(
        "T-036 AC-2 case {case} trial {n}: old (dev {}, ino {}), new (dev {}, ino {}) after {made} \
         directories, reused: {}",
        old.0,
        old.1,
        new.0,
        new.1,
        if reused { "yes" } else { "no" }
    );
    eprintln!("{line}");
    // The replacement really is what the root path reaches now.
    assert_eq!(
        fs::read(root.join("secret.txt")).unwrap(),
        SECRET,
        "[{line}]"
    );
    assert!(!root.join("f.txt").exists(), "[{line}]");
    let before = snapshot(&w.dir);
    let replacement_before = snapshot(&replacement);

    let checks = [
        json!({"id":3,"command":"doc.open","params":{"path":"secret.txt"}}),
        json!({"id":4,"command":"doc.save","params":{"doc":1}}),
        json!({"id":5,"command":"project.tree","params":{}}),
    ];
    for request in checks {
        let id = request["id"].clone();
        let r = s.ask(request);
        assert_eq!(r["id"], id, "[{line}] {r}");
        assert_eq!(
            r["ok"],
            json!(false),
            "[{line}] request {id} not refused: {r}"
        );
        assert_eq!(r["error"]["code"], json!("rejected"), "[{line}] {r}");
        assert_eq!(r["error"]["kind"], json!("outside_root"), "[{line}] {r}");
        assert!(
            !r.to_string().contains(&w.text()),
            "[{line}] names the work path: {r}"
        );
    }
    let r = s.ask(json!({"id":6,"command":"doc.info","params":{"doc":1}}));
    assert_eq!(r["result"]["dirty"], json!(true), "[{line}] {r}");
    assert_eq!(s.finish().code(), Some(0), "[{line}]");

    assert_eq!(
        snapshot(&replacement),
        replacement_before,
        "[{line}] replacement changed"
    );
    for name in snapshot(&replacement).keys() {
        assert!(
            name != "f.txt" && name != "new.txt" && !name.starts_with(".kiln-tmp-"),
            "[{line}] {name} appeared in the replacement"
        );
    }
    assert_eq!(snapshot(&w.dir), before, "[{line}] work changed");
    reused
}

/// Up to [`K`] trials; a trial with reuse ends the loop (it has asserted everything).
fn removed_root_is_refused(case: &str, link: bool, unreadable_after_start: bool) {
    for n in 0..K {
        if trial(case, link, unreadable_after_start, n) {
            break;
        }
    }
}

#[test]
fn root_removed_and_replaced_by_link_is_refused_through_control() {
    let _lock = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    removed_root_is_refused("(i) link", true, false);
}

#[test]
fn root_removed_and_replaced_by_new_directory_is_refused_through_control() {
    let _lock = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    removed_root_is_refused("(ii) new directory", false, false);
}

#[test]
fn root_made_unreadable_after_start_stays_protected_through_control() {
    let _lock = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    removed_root_is_refused("(iii) 0711 at start, then 0311, then link", true, true);
}
