//! T-037 AC-2 (iv)–(vi) through `kiln-cli control`, in a test binary of its own: the root
//! directory removed between two requests of one session, (iv) with nothing in its place, (v)
//! replaced by a new directory at the same path that holds `a.txt` = `alpha\n`, (vi) replaced by
//! a link to such a directory. `doc.open` of the open `a.txt` is then refused `rejected` /
//! `outside_root`, the document stays as it was, and nothing is read or written in the
//! replacement (Interface 1, 2; ruling D-092 O1). Each case runs with the document edited (`E`
//! inserted) and, by ruling D-125 k1, again with it opened and not edited. Written by the test
//! author from `tasks/T-037-reopen-checks-confinement.md` alone.
//!
//! T-036's removal pattern (AC-2, required for (v) and (vi)):
//! - the test holds the file-level lock [`LOCK`] for its whole body, so no two
//!   remove-and-replace windows of this binary overlap;
//! - (v) and (vi) each run up to [`K`] trials, each with a fresh `work/` and a new session;
//! - after the removal the test itself creates `work/c0`, `work/c1`, … one at a time until one
//!   has the removed root's (device, inode), or [`B`] exist; that directory (or the last) is the
//!   replacement (renamed to `root`, or linked as `root`);
//! - each trial writes one line to stderr (case, trial, old and new (device, inode), reused or
//!   not), and every assertion message includes it;
//! - the refusal is asserted in every trial whatever the file system did, no assertion requires
//!   reuse, and a trial with reuse ends the loop.
//!
//! Refusals are asserted by `ok`, `error.code` and `error.kind`, and the response names no part
//! of the work path; no message text or time is asserted (golden rule 9). Every child process
//! and every response has a 10 s timeout, which only detects hangs.
#![cfg(test)]

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// Hang detector for every child process and every response; never a duration assertion.
const TIMEOUT: Duration = Duration::from_secs(10);
const SECRET: &[u8] = b"s3cr3t-content\n";
const OUTSIDE_C: &[u8] = b"outside c\n";
const ALPHA: &[u8] = b"alpha\n";
/// Trials per replacement case (AC-2: K = 5).
const K: usize = 5;
/// Directories created per reuse search at most (AC-2: B = 64).
const B: usize = 64;

/// Serialises the remove-and-replace windows of this binary (poison-tolerant).
static LOCK: Mutex<()> = Mutex::new(());

// ---------------------------------------------------------------------------------------------
// Fixture.

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
        let dir = std::env::temp_dir().join(format!("k37r-{}-{name}", std::process::id()));
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
    /// `kiln-cli control --root <root>`, run in `cwd` as the test's own identity.
    fn start(cwd: &Path, root: &str) -> Session {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kiln-cli"));
        cmd.args(["control", "--root", root])
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = cmd
            .spawn()
            .unwrap_or_else(|e| panic!("precondition: cannot spawn kiln-cli control: {e}"));
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

/// Length in chars of `a.txt` as opened (`alpha\n`), and after the insert of `E` (AC: 7).
fn doc_len(edited: bool) -> usize {
    if edited { 7 } else { 6 }
}

/// `doc.open <path>` (doc 1) and, when `edited`, `doc.insert` of `E` at 0 (ruling D-125 k1 adds
/// the cases with the document not edited).
fn open_and_edit(s: &mut Session, path: &str, edited: bool, case: &str) {
    let r = s.ask("doc.open", json!({ "path": path }));
    assert_eq!(r["ok"], json!(true), "{case}: first doc.open: {r}");
    assert_eq!(r["result"]["doc"], json!(1), "{case}: first doc.open: {r}");
    assert_eq!(
        r["result"]["already_open"],
        json!(false),
        "{case}: first doc.open: {r}"
    );
    if edited {
        let r = s.ask("doc.insert", json!({"doc": 1, "offset": 0, "text": "E"}));
        assert_eq!(r["ok"], json!(true), "{case}: doc.insert: {r}");
    }
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

/// "Document unchanged": `doc.info` → dirty iff `edited`, the length, the undo history kept;
/// `doc.list` → exactly one document, id 1, with `path`.
fn assert_unchanged(s: &mut Session, path: &str, edited: bool, case: &str) {
    let r = s.ask("doc.info", json!({"doc": 1}));
    assert_eq!(r["ok"], json!(true), "{case}: doc.info: {r}");
    assert_eq!(r["result"]["dirty"], json!(edited), "{case}: doc.info: {r}");
    assert_eq!(
        r["result"]["len_chars"],
        json!(doc_len(edited)),
        "{case}: doc.info: {r}"
    );
    assert_eq!(r["result"]["path"], json!(path), "{case}: doc.info: {r}");
    assert_eq!(
        r["result"]["can_undo"],
        json!(edited),
        "{case}: doc.info: {r}"
    );
    let r = s.ask("doc.list", json!({}));
    assert_eq!(r["ok"], json!(true), "{case}: doc.list: {r}");
    let docs = r["result"]["docs"].as_array().unwrap();
    assert_eq!(docs.len(), 1, "{case}: doc.list: {r}");
    assert_eq!(docs[0]["doc"], json!(1), "{case}: doc.list: {r}");
    assert_eq!(docs[0]["path"], json!(path), "{case}: doc.list: {r}");
}

/// A named case: the closure gets the name for its assertion messages.
type Case<'a> = (&'a str, &'a dyn Fn(&str));

/// Run every case; fail naming every case that failed.
fn run_cases(cases: &[Case]) {
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

fn id_of(p: &Path) -> (u64, u64) {
    let m = fs::metadata(p).unwrap();
    (m.dev(), m.ino())
}

// ---------------------------------------------------------------------------------------------
// AC-2 (iv)–(vi).

/// (iv): `root` removed with `remove_dir_all`, nothing in its place.
fn removed_case(edited: bool, case: &str) {
    let w = Work::new(&format!("removed-{edited}"), "root");
    let mut s = Session::start(&w.dir, "root");
    open_and_edit(&mut s, "a.txt", edited, case);
    fs::remove_dir_all(w.root()).unwrap();
    assert!(
        fs::symlink_metadata(w.root()).is_err(),
        "{case}: fixture: root still exists"
    );
    let before = snapshot(&w.dir);

    let r = s.ask("doc.open", json!({"path": "a.txt"}));
    assert_refused(&r, "outside_root", &w, case);
    assert_unchanged(&mut s, "a.txt", edited, case);
    assert_eq!(snapshot(&w.dir), before, "{case}: work/ changed");
    assert!(
        fs::symlink_metadata(w.root()).is_err(),
        "{case}: the root path was created"
    );
    assert_eq!(s.finish().code(), Some(0), "{case}: control exit");
}

/// One trial of (v) (`link` false) or (vi) (`link` true); true iff the replacement got the
/// removed root's (device, inode).
fn replaced_trial(case: &str, link: bool, edited: bool, n: usize) -> bool {
    let w = Work::new(
        &format!("{}-{edited}-{n}", if link { "link" } else { "dir" }),
        "root",
    );
    let root = w.root();
    let mut s = Session::start(&w.dir, "root");
    open_and_edit(&mut s, "a.txt", edited, &format!("{case} trial {n}"));

    // Between two requests: remove the root, then search for a directory that gets its number.
    let old = id_of(&root);
    fs::remove_dir_all(&root).unwrap();
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
    fs::write(replacement.join("a.txt"), ALPHA).unwrap();
    let new = id_of(&root);
    let line = format!(
        "T-037 AC-2 case {case} trial {n}: old (dev {}, ino {}), new (dev {}, ino {}) after {made} \
         directories, reused: {}",
        old.0,
        old.1,
        new.0,
        new.1,
        if reused { "yes" } else { "no" }
    );
    eprintln!("{line}");
    // The replacement really is what the root path reaches now.
    assert_eq!(fs::read(root.join("a.txt")).unwrap(), ALPHA, "[{line}]");
    assert_eq!(
        fs::symlink_metadata(&root)
            .unwrap()
            .file_type()
            .is_symlink(),
        link,
        "[{line}] fixture"
    );
    let before = snapshot(&w.dir);
    let replacement_before = snapshot(&replacement);

    let r = s.ask("doc.open", json!({"path": "a.txt"}));
    assert_refused(&r, "outside_root", &w, &format!("[{line}]"));
    assert_unchanged(&mut s, "a.txt", edited, &format!("[{line}]"));

    let replacement_after = snapshot(&replacement);
    assert_eq!(
        replacement_after, replacement_before,
        "[{line}] replacement changed"
    );
    assert_eq!(
        replacement_after.keys().collect::<Vec<_>>(),
        vec!["a.txt"],
        "[{line}] the replacement gained a file"
    );
    assert_eq!(snapshot(&w.dir), before, "[{line}] work/ changed");
    assert_eq!(s.finish().code(), Some(0), "[{line}] control exit");
    reused
}

/// Up to [`K`] trials; a trial with reuse ends the loop (it has asserted everything).
fn replaced_case(case: &str, link: bool, edited: bool) {
    for n in 0..K {
        if replaced_trial(case, link, edited, n) {
            break;
        }
    }
}

#[test]
fn reopen_after_the_root_was_removed_is_refused() {
    let _lock = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    run_cases(&[
        ("AC-2 (iv) root removed, nothing in its place", &|case| {
            removed_case(true, case);
        }),
        (
            "AC-2 (v) root removed, replaced by a new directory",
            &|case| {
                replaced_case(case, false, true);
            },
        ),
        (
            "AC-2 (vi) root removed, replaced by a link to a new directory",
            &|case| {
                replaced_case(case, true, true);
            },
        ),
        ("AC-2 (iv) unedited (k1)", &|case| {
            removed_case(false, case);
        }),
        ("AC-2 (v) unedited (k1)", &|case| {
            replaced_case(case, false, false);
        }),
        ("AC-2 (vi) unedited (k1)", &|case| {
            replaced_case(case, true, false);
        }),
    ]);
}
