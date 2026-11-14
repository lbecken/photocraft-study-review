//! T-036 AC-2 (item 2, TG-1) through the Rust API of `kiln-workspace`, in a test binary of its
//! own: the root directory removed between two calls and its path replaced, (i) by a link to a
//! new directory or (ii) by a new directory at the same path, is refused by `read`, `is_file`,
//! `write_atomic`, `list_dir` and the save of a document opened before, and nothing is read,
//! listed or written in the replacement (Interface 2, T-008 v9). Written by the test author from
//! `tasks/T-036-root-identity-follow-ups.md` alone.
//!
//! Making detection reliable (AC-2, required):
//! - every test holds the file-level lock [`LOCK`] for its whole body, so no two
//!   remove-and-replace windows of this binary overlap;
//! - each test runs up to [`K`] trials, each with a fresh `work/` and a new `DiskFs`;
//! - after the removal the test itself creates `work/c0`, `work/c1`, … one at a time until one
//!   has the removed root's (device, inode), or [`B`] exist; that directory (or the last) is the
//!   replacement;
//! - each trial writes one line to stderr (case, trial, old and new (device, inode), reused or
//!   not), and every assertion message includes it;
//! - the refusals are asserted in every trial whatever the file system did; a trial whose
//!   replacement got the removed root's number ends the loop with `break`.
//!
//! Errors are asserted by variant and `path()`; `Display` only names no part of the work path.
#![cfg(test)]

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::fs;
use std::os::unix::fs::{MetadataExt, symlink};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use kiln_workspace::{DiskFs, Document, RelPath, Vfs, VfsError};

const SECRET: &[u8] = b"s3cr3t-content\n";
const F_ORIGINAL: &[u8] = b"f original\n";
/// Trials per test (AC-2: K >= 5).
const K: usize = 5;
/// Directories created per reuse search at most (AC-2: B >= 64).
const B: usize = 64;

/// Serialises the remove-and-replace windows of this binary (poison-tolerant).
static LOCK: Mutex<()> = Mutex::new(());

// ---------------------------------------------------------------------------------------------
// Fixtures.

/// A fresh `work/` with `root/`, `outside/` and `outside/secret.txt`; removed on drop.
struct Work {
    dir: PathBuf,
}

impl Work {
    fn new(name: &str) -> Work {
        let dir = std::env::temp_dir().join(format!("k36r-{}-{name}", std::process::id()));
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
}

impl Drop for Work {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn rp(s: &str) -> RelPath {
    RelPath::new(s).unwrap_or_else(|e| panic!("RelPath::new({s:?}) rejected: {e:?}"))
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

/// `r` is `Err(OutsideRoot)` naming `path`, and its `Display` names no part of the work path.
fn assert_outside<T: Debug>(r: Result<T, VfsError>, path: &str, w: &Work, what: &str) {
    match r {
        Err(e @ VfsError::OutsideRoot { .. }) => {
            assert_eq!(e.path().as_str(), path, "{what}: path()");
            let text = e.to_string();
            let full = w.dir.to_string_lossy();
            let name = w.dir.file_name().unwrap().to_string_lossy();
            assert!(
                !text.contains(full.as_ref()) && !text.contains(name.as_ref()),
                "{what}: Display names the host work path: {text:?}"
            );
        }
        other => panic!("{what}: expected Err(OutsideRoot {{ path: {path} }}), got {other:?}"),
    }
}

fn id_of(p: &Path) -> (u64, u64) {
    let m = fs::metadata(p).unwrap();
    (m.dev(), m.ino())
}

// ---------------------------------------------------------------------------------------------
// AC-2, Rust API.

/// One trial of `case`; true iff the replacement got the removed root's (device, inode).
fn trial(case: &str, link: bool, n: usize) -> bool {
    let w = Work::new(&format!("{}-{n}", if link { "link" } else { "dir" }));
    fs::write(w.at("root/f.txt"), F_ORIGINAL).unwrap();
    let root = w.at("root");
    let mut fs_ = DiskFs::new(&root).unwrap();
    let mut doc = Document::open(&fs_, &rp("f.txt")).unwrap();
    doc.buffer_mut().insert(0, "EDIT ").unwrap();

    // Between two calls: remove the root, then search for a directory that gets its number.
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
    fs::write(replacement.join("secret.txt"), SECRET).unwrap();
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
    assert_eq!(fs::read(root.join("secret.txt")).unwrap(), SECRET, "[{line}]");
    assert!(!root.join("f.txt").exists(), "[{line}]");
    let before = snapshot(&w.dir);
    let replacement_before = snapshot(&replacement);

    assert_outside(
        fs_.read(&rp("secret.txt")),
        "secret.txt",
        &w,
        &format!("[{line}] read(secret.txt)"),
    );
    assert!(
        !fs_.is_file(&rp("secret.txt")),
        "[{line}] is_file(secret.txt)"
    );
    assert_outside(
        fs_.write_atomic(&rp("new.txt"), b"PWNED"),
        "new.txt",
        &w,
        &format!("[{line}] write_atomic(new.txt)"),
    );
    assert_outside(
        fs_.list_dir(None),
        "(root)",
        &w,
        &format!("[{line}] list_dir(None)"),
    );
    assert_outside(
        fs_.list_dir(Some(&rp("secret.txt"))),
        "secret.txt",
        &w,
        &format!("[{line}] list_dir(secret.txt)"),
    );
    assert_outside(
        doc.save(&mut fs_),
        "f.txt",
        &w,
        &format!("[{line}] save of f.txt opened before"),
    );
    assert!(doc.is_dirty(), "[{line}] the document is no longer dirty");

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
fn removed_root_is_refused(case: &str, link: bool) {
    for n in 0..K {
        if trial(case, link, n) {
            break;
        }
    }
}

#[test]
fn root_removed_and_replaced_by_link_is_refused() {
    let _lock = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    removed_root_is_refused("(i) link", true);
}

#[test]
fn root_removed_and_replaced_by_new_directory_is_refused() {
    let _lock = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    removed_root_is_refused("(ii) new directory", false);
}
