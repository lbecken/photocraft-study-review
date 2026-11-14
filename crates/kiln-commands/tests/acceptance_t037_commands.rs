//! T-037 AC-2 through the Rust API of `kiln-commands`: on a
//! `Context::with_vfs(Box::new(DiskFs::new(&root)?))`, `doc.open` of a path that is already open,
//! dispatched by name through `standard_registry()?.execute`, is refused
//! `rejected` / `outside_root` once the root path no longer leads to the root: (i) `root` renamed
//! to `root.away` (and, moved back, the re-open answers `already_open: true` again), (iv) `root`
//! removed, (v) `root` removed and replaced by a new directory at the same path that holds
//! `a.txt` = `alpha\n` (Interface 1, 2; ruling D-092 O1). The refused re-open leaves the document
//! open and unchanged and adds nothing to the session log. Written by the test author from
//! `tasks/T-037-reopen-checks-confinement.md` alone.
//!
//! Case (v) uses T-036's removal pattern: every test in this file holds the file-level lock
//! [`LOCK`] for its whole body; up to [`K`] trials, each with a fresh `work/` and a new context;
//! after the removal the test creates `work/c0`, `work/c1`, … one at a time until one has the
//! removed root's (device, inode) or [`B`] exist, and that directory (or the last) is renamed to
//! `root`; each trial writes one line to stderr (case, trial, old and new (device, inode), reused
//! or not) that every assertion message includes; the refusal is asserted in every trial, no
//! assertion requires reuse, and a trial with reuse ends the loop.
//!
//! Errors are asserted by `to_json()`'s `code` and `reason`, and the serialised error names no
//! part of the work path; no message text is asserted (golden rule 9).
#![cfg(test)]

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use kiln_commands::{Context, Registry, standard_registry};
use kiln_workspace::DiskFs;
use serde_json::{Value, json};

const SECRET: &[u8] = b"s3cr3t-content\n";
const ALPHA: &[u8] = b"alpha\n";
/// Trials of case (v) (AC-2: K = 5).
const K: usize = 5;
/// Directories created per reuse search at most (AC-2: B = 64).
const B: usize = 64;

/// Serialises the remove-and-replace windows of this binary (poison-tolerant).
static LOCK: Mutex<()> = Mutex::new(());

// ---------------------------------------------------------------------------------------------
// Fixture.

/// A fresh `work/` under `std::env::temp_dir()`: `root/` with `a.txt`, `b.txt`, `sub/c.txt`,
/// `real/t.txt`; `outside/` with `secret.txt` and `c.txt`. Removed on drop, failed runs
/// included.
struct Work {
    dir: PathBuf,
}

impl Work {
    fn new(name: &str) -> Work {
        let dir = std::env::temp_dir().join(format!("k37m-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let dir = fs::canonicalize(&dir).unwrap();
        let w = Work { dir };
        w.file("root/a.txt", ALPHA);
        w.file("root/b.txt", b"bravo\n");
        w.file("root/sub/c.txt", b"charlie\n");
        w.file("root/real/t.txt", b"tango\n");
        w.file("outside/secret.txt", SECRET);
        w.file("outside/c.txt", b"outside c\n");
        w
    }

    fn at(&self, rel: &str) -> PathBuf {
        self.dir.join(rel)
    }

    fn file(&self, at: &str, bytes: &[u8]) {
        let p = self.at(at);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, bytes).unwrap();
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

fn id_of(p: &Path) -> (u64, u64) {
    let m = fs::metadata(p).unwrap();
    (m.dev(), m.ino())
}

// ---------------------------------------------------------------------------------------------
// The session.

struct Session {
    reg: Registry,
    ctx: Context,
}

impl Session {
    /// A context on `DiskFs::new(root)`, `doc.open a.txt` (doc 1) and `doc.insert` of `E` at 0.
    fn open_and_edit(root: &Path, case: &str) -> Session {
        let reg = standard_registry().unwrap_or_else(|e| panic!("standard_registry: {e}"));
        let disk = DiskFs::new(root).unwrap_or_else(|e| panic!("{case}: DiskFs::new: {e:?}"));
        let mut s = Session {
            reg,
            ctx: Context::with_vfs(Box::new(disk)),
        };
        let r = s.ok("doc.open", json!({"path": "a.txt"}), case);
        assert_eq!(r["doc"], json!(1), "{case}: first doc.open: {r}");
        assert_eq!(
            r["already_open"],
            json!(false),
            "{case}: first doc.open: {r}"
        );
        s.ok(
            "doc.insert",
            json!({"doc": 1, "offset": 0, "text": "E"}),
            case,
        );
        s
    }

    fn ok(&mut self, name: &str, params: Value, case: &str) -> Value {
        self.reg
            .execute(&mut self.ctx, name, params.clone())
            .unwrap_or_else(|e| panic!("{case}: {name} {params} failed: {}", e.to_json()))
    }

    /// The re-open of `a.txt` is `Err` with `code` `rejected` and `reason` `outside_root`, names
    /// no part of the work path, and adds nothing to the log.
    fn reopen_refused(&mut self, w: &Work, case: &str) {
        let log_before = self.ctx.log().len();
        match self
            .reg
            .execute(&mut self.ctx, "doc.open", json!({"path": "a.txt"}))
        {
            Ok(v) => panic!("{case}: re-open not refused: Ok({v})"),
            Err(e) => {
                let j = e.to_json();
                assert_eq!(j["code"], json!("rejected"), "{case}: {j}");
                assert_eq!(j["reason"], json!("outside_root"), "{case}: {j}");
                assert!(
                    !j.to_string().contains(w.dir.to_string_lossy().as_ref()),
                    "{case}: the error names the work path: {j}"
                );
            }
        }
        assert_eq!(
            self.ctx.log().len(),
            log_before,
            "{case}: the refused re-open was logged"
        );
    }

    /// "Document unchanged": `doc.info` → dirty, `len_chars` 7, the undo history kept;
    /// `doc.list` → exactly one document, id 1, `a.txt`.
    fn assert_unchanged(&mut self, case: &str) {
        let r = self.ok("doc.info", json!({"doc": 1}), case);
        assert_eq!(r["dirty"], json!(true), "{case}: doc.info: {r}");
        assert_eq!(r["len_chars"], json!(7), "{case}: doc.info: {r}");
        assert_eq!(r["path"], json!("a.txt"), "{case}: doc.info: {r}");
        assert_eq!(r["can_undo"], json!(true), "{case}: doc.info: {r}");
        let r = self.ok("doc.list", json!({}), case);
        let docs = r["docs"].as_array().unwrap();
        assert_eq!(docs.len(), 1, "{case}: doc.list: {r}");
        assert_eq!(docs[0]["doc"], json!(1), "{case}: doc.list: {r}");
        assert_eq!(docs[0]["path"], json!("a.txt"), "{case}: doc.list: {r}");
    }
}

// ---------------------------------------------------------------------------------------------
// AC-2, Rust API.

/// (i): `root` renamed to `root.away`, then moved back.
fn renamed_case(case: &str) {
    let w = Work::new("renamed");
    let mut s = Session::open_and_edit(&w.at("root"), case);
    fs::rename(w.at("root"), w.at("root.away")).unwrap();
    assert!(
        fs::symlink_metadata(w.at("root")).is_err(),
        "{case}: fixture"
    );
    assert_eq!(
        fs::read(w.at("root.away/a.txt")).unwrap(),
        ALPHA,
        "{case}: fixture"
    );
    let before = snapshot(&w.dir);
    s.reopen_refused(&w, case);
    s.assert_unchanged(case);
    assert_eq!(snapshot(&w.dir), before, "{case}: work/ changed");

    fs::rename(w.at("root.away"), w.at("root")).unwrap();
    let r = s.ok("doc.open", json!({"path": "a.txt"}), case);
    assert_eq!(
        r,
        json!({"already_open": true, "doc": 1, "encoding": "utf-8", "len_chars": 7,
               "line_ending": "lf", "mixed_line_endings": false, "path": "a.txt"}),
        "{case}: moved back: re-open"
    );
    s.assert_unchanged(&format!("{case}: moved back"));
}

/// (iv): `root` removed, nothing in its place.
fn removed_case(case: &str) {
    let w = Work::new("removed");
    let mut s = Session::open_and_edit(&w.at("root"), case);
    fs::remove_dir_all(w.at("root")).unwrap();
    assert!(
        fs::symlink_metadata(w.at("root")).is_err(),
        "{case}: fixture"
    );
    let before = snapshot(&w.dir);
    s.reopen_refused(&w, case);
    s.assert_unchanged(case);
    assert_eq!(snapshot(&w.dir), before, "{case}: work/ changed");
}

/// One trial of (v); true iff the replacement got the removed root's (device, inode).
fn replaced_trial(case: &str, n: usize) -> bool {
    let w = Work::new(&format!("replaced-{n}"));
    let root = w.at("root");
    let mut s = Session::open_and_edit(&root, &format!("{case} trial {n}"));

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
    fs::rename(w.at(&format!("c{}", made - 1)), &root).unwrap();
    fs::write(root.join("a.txt"), ALPHA).unwrap();
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
    assert_eq!(fs::read(root.join("a.txt")).unwrap(), ALPHA, "[{line}]");
    let before = snapshot(&w.dir);
    let replacement_before = snapshot(&root);

    s.reopen_refused(&w, &format!("[{line}]"));
    s.assert_unchanged(&format!("[{line}]"));

    let replacement_after = snapshot(&root);
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
    reused
}

/// Up to [`K`] trials; a trial with reuse ends the loop (it has asserted everything).
fn replaced_case(case: &str) {
    for n in 0..K {
        if replaced_trial(case, n) {
            break;
        }
    }
}

/// A named case: the function gets the name for its assertion messages.
type Case = (&'static str, fn(&str));

#[test]
fn reopen_after_the_root_changed_is_refused() {
    let _lock = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let cases: [Case; 3] = [
        (
            "AC-2 (i) root renamed to root.away, then back",
            renamed_case,
        ),
        ("AC-2 (iv) root removed", removed_case),
        (
            "AC-2 (v) root removed, replaced by a new directory",
            replaced_case,
        ),
    ];
    let mut failed = Vec::new();
    for (case, f) in cases {
        if catch_unwind(AssertUnwindSafe(|| f(case))).is_err() {
            failed.push(case);
        }
    }
    assert!(
        failed.is_empty(),
        "failed cases (each one's message is above): {failed:?}"
    );
}
