//! T-036 guards through the Rust API of `kiln-workspace` (Interface 3 and 4): a root given
//! through a link that is retargeted between two calls (AC-3, TG-3) and a root whose path leads
//! nowhere (AC-4, TG-4) are refused with `OutsideRoot` by `read`, `is_file`, `write_atomic`,
//! `list_dir` and the save of a document opened before, nothing is created or changed, and
//! requests work again once the path leads to the root directory again. Base already behaves
//! so (T-008 r2 probes (e), (h); T-019 r1 "Root renamed away"); these tests must keep passing.
//! Written by the test author from `tasks/T-036-root-identity-follow-ups.md` alone.
//!
//! Errors are asserted by variant and `path()`; `Display` is only checked to name no part of
//! the host work path. Each fixture is a fresh `work/` under `std::env::temp_dir()`, removed on
//! drop.
//!
//! Platforms (T-092): the root's parent renamed while the root is held is a Unix test
//! (`vanished_root_is_refused_unix`): Windows refuses that rename (T-072 round 3); every other
//! test runs on every platform, with twin helpers for the directory link.
#![cfg(test)]

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};

use kiln_workspace::{DiskFs, Document, OpenError, RelPath, Vfs, VfsError};

const SECRET: &[u8] = b"s3cr3t-content\n";
const F_ORIGINAL: &[u8] = b"f original\n";
const OTHER_F: &[u8] = b"other f\n";
const EDIT: &str = "EDIT ";
const F_EDITED: &[u8] = b"EDIT f original\n";

// ---------------------------------------------------------------------------------------------
// Fixtures.

/// A fresh `work/` with `root/`, `outside/` and `outside/secret.txt`; removed on drop.
struct Work {
    dir: PathBuf,
}

impl Work {
    fn new(name: &str) -> Work {
        let dir = std::env::temp_dir().join(format!("k36g-{}-{name}", std::process::id()));
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
}

impl Drop for Work {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// A link at `link` to the directory `target` (a single name beside it).
#[cfg(unix)]
fn symlink(target: &str, link: PathBuf) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}
/// Windows: a directory symbolic link; the target is built with `PathBuf::push` (ruling Q-B of
/// T-092).
#[cfg(windows)]
fn symlink(target: &str, link: PathBuf) -> std::io::Result<()> {
    let mut t = PathBuf::new();
    t.push(target);
    std::os::windows::fs::symlink_dir(t, link)
}

/// Remove the directory link at `link`.
#[cfg(unix)]
fn remove_dir_link(link: PathBuf) -> std::io::Result<()> {
    fs::remove_file(link)
}
/// Windows: a directory link is removed as a directory (ruling Q-B of T-092, `T-074-probe-1.md`
/// E4).
#[cfg(windows)]
fn remove_dir_link(link: PathBuf) -> std::io::Result<()> {
    fs::remove_dir(link)
}

fn rp(s: &str) -> RelPath {
    RelPath::new(s).unwrap_or_else(|e| panic!("RelPath::new({s:?}) rejected: {e:?}"))
}

/// Every entry under `dir`, recursively, links not followed: type, link target and bytes. Keys
/// are `/`-joined relative paths on every platform.
fn snapshot(dir: &Path) -> BTreeMap<String, String> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            let rel = p
                .strip_prefix(base)
                .unwrap()
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
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

/// Every request of Interface 2 is refused; `doc` (opened before, edited) stays dirty.
fn assert_all_refused(fs_: &mut DiskFs, doc: &mut Document, w: &Work, reads: &[&str], note: &str) {
    for p in reads {
        assert_outside(fs_.read(&rp(p)), p, w, &format!("{note}: read({p})"));
        assert!(!fs_.is_file(&rp(p)), "{note}: is_file({p}) is true");
    }
    assert_outside(
        fs_.write_atomic(&rp("new.txt"), b"PWNED"),
        "new.txt",
        w,
        &format!("{note}: write_atomic(new.txt)"),
    );
    assert_outside(
        fs_.list_dir(None),
        "(root)",
        w,
        &format!("{note}: list_dir(None)"),
    );
    assert_outside(
        doc.save(fs_),
        "f.txt",
        w,
        &format!("{note}: save of f.txt opened before"),
    );
    assert!(doc.is_dirty(), "{note}: the document is no longer dirty");
}

// ---------------------------------------------------------------------------------------------
// AC-3 (item 3, TG-3).

#[test]
fn retargeted_root_link_is_refused() {
    let w = Work::new("retarget");
    w.file("root/f.txt", F_ORIGINAL);
    w.file("other/secret.txt", SECRET);
    w.file("other/f.txt", OTHER_F);
    symlink("root", w.at("rootlink")).unwrap();
    let mut fs_ = DiskFs::new(&w.at("rootlink")).unwrap();
    let mut doc = Document::open(&fs_, &rp("f.txt")).unwrap();
    doc.buffer_mut().insert(0, EDIT).unwrap();

    // Retarget the link between two calls, and show the other directory is really reached.
    remove_dir_link(w.at("rootlink")).unwrap();
    symlink("other", w.at("rootlink")).unwrap();
    assert_eq!(fs::read(w.at("rootlink/secret.txt")).unwrap(), SECRET);
    assert_eq!(fs::read(w.at("rootlink/f.txt")).unwrap(), OTHER_F);
    let before = snapshot(&w.dir);

    assert_all_refused(
        &mut fs_,
        &mut doc,
        &w,
        &["secret.txt", "f.txt"],
        "rootlink -> other",
    );
    assert_outside(
        fs_.list_dir(Some(&rp("secret.txt"))),
        "secret.txt",
        &w,
        "rootlink -> other: list_dir(secret.txt)",
    );
    match Document::open(&fs_, &rp("secret.txt")) {
        Err(OpenError::Vfs(e)) => assert_outside(
            Err::<(), _>(e),
            "secret.txt",
            &w,
            "rootlink -> other: Document::open(secret.txt)",
        ),
        other => panic!("rootlink -> other: Document::open(secret.txt): {other:?}"),
    }
    assert_eq!(snapshot(&w.dir), before, "rootlink -> other: work changed");
    assert_eq!(fs::read(w.at("root/f.txt")).unwrap(), F_ORIGINAL);
    assert_eq!(fs::read(w.at("other/f.txt")).unwrap(), OTHER_F);
    assert_nothing_created(&w.dir, "rootlink -> other");

    // Point the link back: the save of the earlier document works there again.
    remove_dir_link(w.at("rootlink")).unwrap();
    symlink("root", w.at("rootlink")).unwrap();
    doc.save(&mut fs_)
        .unwrap_or_else(|e| panic!("rootlink -> root again: save failed: {e:?}"));
    assert!(!doc.is_dirty());
    assert_eq!(fs::read(w.at("root/f.txt")).unwrap(), F_EDITED);
    assert_eq!(fs::read(w.at("other/f.txt")).unwrap(), OTHER_F);
    assert_eq!(fs_.read(&rp("f.txt")).unwrap(), F_EDITED);
    assert_nothing_created(&w.dir, "rootlink -> root again");
}

// ---------------------------------------------------------------------------------------------
// AC-4 (item 4, TG-4).

/// The three ways the root path comes to lead nowhere, with nothing put in its place.
#[derive(Clone, Copy, Debug)]
enum Vanish {
    /// (i) `root` renamed to `root.away`.
    RenameRoot,
    /// (ii) the root at `work/p/root`, `p` renamed to `p.away` (Unix: Windows refuses to rename
    /// a directory above the held root).
    #[cfg(unix)]
    RenameParent,
    /// (iii) `root` removed with `remove_dir_all`.
    Remove,
}

fn vanished_root_case(case: Vanish) {
    let note = format!("{case:?}");
    let w = Work::new(&format!("vanish-{note}"));
    let (root_rel, away, back) = match case {
        Vanish::RenameRoot => ("root", Some(("root", "root.away")), "root.away/f.txt"),
        #[cfg(unix)]
        Vanish::RenameParent => ("p/root", Some(("p", "p.away")), "p.away/root/f.txt"),
        Vanish::Remove => ("root", None, ""),
    };
    w.file(&format!("{root_rel}/f.txt"), F_ORIGINAL);
    let root = w.at(root_rel);
    let mut fs_ = DiskFs::new(&root).unwrap();
    let mut doc = Document::open(&fs_, &rp("f.txt")).unwrap();
    doc.buffer_mut().insert(0, EDIT).unwrap();

    // The change, between two calls; the root path now leads nowhere.
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

    assert_all_refused(&mut fs_, &mut doc, &w, &["f.txt", "missing.txt"], &note);
    assert_eq!(snapshot(&w.dir), before, "{note}: work changed");
    assert!(
        fs::symlink_metadata(&root).is_err(),
        "{note}: the root path was created"
    );
    assert_nothing_created(&w.dir, &note);

    // Moved back: the save works into root/f.txt.
    if let Some((from, to)) = away {
        fs::rename(w.at(to), w.at(from)).unwrap();
        doc.save(&mut fs_)
            .unwrap_or_else(|e| panic!("{note}: moved back: save failed: {e:?}"));
        assert!(!doc.is_dirty(), "{note}: moved back: still dirty");
        assert_eq!(fs::read(root.join("f.txt")).unwrap(), F_EDITED, "{note}");
        assert_nothing_created(&w.dir, &format!("{note}: moved back"));
    }
}

#[test]
fn vanished_root_is_refused() {
    for case in [Vanish::RenameRoot, Vanish::Remove] {
        vanished_root_case(case);
    }
}

/// The Unix part of `vanished_root_is_refused`: the root's parent renamed while the root is
/// held, a rename Windows refuses (`T-072-round-3.md`: "rename of p while the root is held:
/// refused").
#[test]
#[cfg(unix)]
fn vanished_root_is_refused_unix() {
    vanished_root_case(Vanish::RenameParent);
}
