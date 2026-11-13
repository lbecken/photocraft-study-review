//! T-019 AC-7 (Rust API, additional to the CLI tests): `Vfs::list_dir` on `MemoryFs` and
//! `DiskFs` gives entries sorted by name bytes with their own kinds; on disk a name containing
//! `\` and a name that is not UTF-8 are omitted and counted in `skipped`; listing a file, a
//! missing path and a link to an outside directory give `NotAFile`, `NotFound` and
//! `OutsideRoot`; `project::tree` gives Pinned (g)'s worked example. Written by the test author
//! from the task specification alone. Oracle: exact values.
//!
//! On the base this file does not compile (`list_dir`, `DirEntry`, `project::tree` do not exist
//! yet); that is the expected, weak, fail-on-base evidence (§6). Calls that could block (a FIFO)
//! run on a helper thread with a timeout; a hang is a test failure. Every fixture is a fresh
//! temporary directory removed on drop.
#![cfg(test)]

use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileTypeExt, symlink};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use kiln_workspace::project::{self, ProjectTree, TreeEntry};
use kiln_workspace::{DirEntry, DirListing, DiskFs, EntryKind, MemoryFs, RelPath, Vfs, VfsError};

const HANG: Duration = Duration::from_secs(5);

fn p(s: &str) -> RelPath {
    RelPath::new(s).unwrap()
}

fn listing(items: &[(&str, EntryKind)], skipped: usize) -> DirListing {
    DirListing {
        entries: items
            .iter()
            .map(|(n, k)| DirEntry {
                name: (*n).to_owned(),
                kind: *k,
            })
            .collect(),
        skipped,
    }
}

static N: AtomicUsize = AtomicUsize::new(0);

/// A fresh temporary directory, removed on drop.
struct Work {
    dir: PathBuf,
}

impl Work {
    fn new(tag: &str) -> Work {
        let dir = std::env::temp_dir().join(format!(
            "k19w-{}-{}-{tag}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Work {
            dir: fs::canonicalize(&dir).unwrap(),
        }
    }

    fn at(&self, rel: &str) -> PathBuf {
        self.dir.join(rel)
    }

    fn file(&self, rel: &str, bytes: &[u8]) {
        let at = self.at(rel);
        fs::create_dir_all(at.parent().unwrap()).unwrap();
        fs::write(at, bytes).unwrap();
    }
}

impl Drop for Work {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn mkfifo(path: &Path) {
    let mut child = Command::new("mkfifo")
        .arg(path)
        .stdin(Stdio::null())
        .spawn()
        .expect("spawn mkfifo");
    let deadline = Instant::now() + HANG;
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break s;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("mkfifo did not finish");
        }
        thread::sleep(Duration::from_millis(2));
    };
    assert!(status.success());
    assert!(fs::symlink_metadata(path).unwrap().file_type().is_fifo());
}

/// Bind a Unix socket at a short path and rename it to `path`.
fn socket_at(path: &Path) -> UnixListener {
    let short = std::env::temp_dir().join(format!(
        "k19s{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_file(&short);
    let listener = UnixListener::bind(&short).unwrap();
    fs::rename(&short, path).unwrap();
    assert!(fs::symlink_metadata(path).unwrap().file_type().is_socket());
    listener
}

/// `list_dir` on a helper thread; a call that blocks longer than [`HANG`] fails the test.
fn list_within(root: &Path, dir: Option<&str>) -> Result<DirListing, VfsError> {
    let root = root.to_path_buf();
    let dir = dir.map(p);
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let fs_ = DiskFs::new(&root).unwrap();
        let _ = tx.send(fs_.list_dir(dir.as_ref()));
    });
    rx.recv_timeout(HANG)
        .unwrap_or_else(|_| panic!("list_dir did not return within {HANG:?} (hang)"))
}

// ---------------------------------------------------------------------------------------------
// MemoryFs

#[test]
fn memory_list_dir_sorts_by_bytes_and_reports_kinds() {
    let mut m = MemoryFs::new();
    assert_eq!(m.list_dir(None), Ok(DirListing::default()));
    for (path, bytes) in [
        ("b.rs", &b"b"[..]),
        ("B.rs", b"B"),
        ("_.txt", b"_"),
        ("a/x", b"x"),
        ("a/y/z", b"z"),
        ("\u{e9}", b"e"),
        ("Z", b""),
    ] {
        m.insert(&p(path), bytes);
    }
    assert_eq!(
        m.list_dir(None),
        Ok(listing(
            &[
                ("B.rs", EntryKind::File),
                ("Z", EntryKind::File),
                ("_.txt", EntryKind::File),
                ("a", EntryKind::Dir),
                ("b.rs", EntryKind::File),
                ("\u{e9}", EntryKind::File),
            ],
            0
        ))
    );
    assert_eq!(
        m.list_dir(Some(&p("a"))),
        Ok(listing(&[("x", EntryKind::File), ("y", EntryKind::Dir)], 0))
    );
    assert_eq!(
        m.list_dir(Some(&p("a/y"))),
        Ok(listing(&[("z", EntryKind::File)], 0))
    );
    assert_eq!(
        m.list_dir(Some(&p("b.rs"))),
        Err(VfsError::NotAFile { path: p("b.rs") })
    );
    assert_eq!(
        m.list_dir(Some(&p("nope"))),
        Err(VfsError::NotFound { path: p("nope") })
    );
    assert_eq!(
        m.list_dir(Some(&p("b.rs/q"))),
        Err(VfsError::NotFound { path: p("b.rs/q") })
    );
    // A prefix of a name is not a directory.
    assert_eq!(
        m.list_dir(Some(&p("a/y/z/w"))),
        Err(VfsError::NotFound { path: p("a/y/z/w") })
    );
    assert_eq!(
        m.list_dir(Some(&p("b"))),
        Err(VfsError::NotFound { path: p("b") })
    );
}

// ---------------------------------------------------------------------------------------------
// DiskFs

#[test]
fn disk_list_dir_reports_entries_by_their_own_kind_and_skips_bad_names() {
    let w = Work::new("disk");
    fs::create_dir_all(w.at("root")).unwrap();
    w.file("outside/secret.txt", b"s3cr3t\n");
    w.file("root/B.rs", b"B");
    w.file("root/a.rs", b"a");
    w.file("root/sub/x", b"x");
    w.file("root/\u{e9}", b"e");
    symlink("sub", w.at("root/l")).unwrap();
    symlink("nowhere", w.at("root/dang")).unwrap();
    symlink("../outside", w.at("root/out")).unwrap();
    symlink("loop2", w.at("root/loop1")).unwrap();
    symlink("loop1", w.at("root/loop2")).unwrap();
    symlink("a.rs", w.at("root/la")).unwrap();
    mkfifo(&w.at("root/p"));
    let _listener = socket_at(&w.at("root/s"));
    // Two names no RelPath segment can carry.
    fs::write(w.at("root").join(OsStr::from_bytes(b"n\xff")), b"n").unwrap();
    fs::write(w.at("root/b\\s"), b"bs").unwrap();
    fs::create_dir(w.at("root").join(OsStr::from_bytes(b"d\xfe"))).unwrap();
    assert_eq!(fs::read(w.at("root/out/secret.txt")).unwrap(), b"s3cr3t\n");

    let root = w.at("root");
    assert_eq!(
        list_within(&root, None),
        Ok(listing(
            &[
                ("B.rs", EntryKind::File),
                ("a.rs", EntryKind::File),
                ("dang", EntryKind::Symlink),
                ("l", EntryKind::Symlink),
                ("la", EntryKind::Symlink),
                ("loop1", EntryKind::Symlink),
                ("loop2", EntryKind::Symlink),
                ("out", EntryKind::Symlink),
                ("p", EntryKind::Other),
                ("s", EntryKind::Other),
                ("sub", EntryKind::Dir),
                ("\u{e9}", EntryKind::File),
            ],
            3
        ))
    );
    assert_eq!(
        list_within(&root, Some("sub")),
        Ok(listing(&[("x", EntryKind::File)], 0))
    );
    // An in-root link to a directory is followed to locate `dir` (T-008's rules).
    assert_eq!(
        list_within(&root, Some("l")),
        Ok(listing(&[("x", EntryKind::File)], 0))
    );
    assert_eq!(
        list_within(&root, Some("a.rs")),
        Err(VfsError::NotAFile { path: p("a.rs") })
    );
    assert_eq!(
        list_within(&root, Some("la")),
        Err(VfsError::NotAFile { path: p("la") })
    );
    assert_eq!(
        list_within(&root, Some("p")),
        Err(VfsError::NotAFile { path: p("p") })
    );
    assert_eq!(
        list_within(&root, Some("s")),
        Err(VfsError::NotAFile { path: p("s") })
    );
    assert_eq!(
        list_within(&root, Some("nope")),
        Err(VfsError::NotFound { path: p("nope") })
    );
    assert_eq!(
        list_within(&root, Some("dang")),
        Err(VfsError::NotFound { path: p("dang") })
    );
    assert_eq!(
        list_within(&root, Some("out")),
        Err(VfsError::OutsideRoot { path: p("out") })
    );
    assert_eq!(
        list_within(&root, Some("out/x")),
        Err(VfsError::OutsideRoot { path: p("out/x") })
    );
    match list_within(&root, Some("loop1")) {
        Err(VfsError::Io { path, .. }) => assert_eq!(path, p("loop1")),
        other => panic!("list_dir(loop1): expected Io, got {other:?}"),
    }
    // Nothing was followed into or changed.
    assert_eq!(fs::read(w.at("outside/secret.txt")).unwrap(), b"s3cr3t\n");
    assert!(
        fs::symlink_metadata(w.at("root/p"))
            .unwrap()
            .file_type()
            .is_fifo()
    );
}

#[test]
fn disk_root_errors_carry_the_root_placeholder() {
    let w = Work::new("root");
    w.file("root/a.rs", b"a");
    let fs_ = DiskFs::new(&w.at("root")).unwrap();
    assert_eq!(
        fs_.list_dir(None),
        Ok(listing(&[("a.rs", EntryKind::File)], 0))
    );
    // The root path no longer leads to the root directory.
    fs::rename(w.at("root"), w.at("moved")).unwrap();
    fs::create_dir(w.at("root")).unwrap();
    let err = fs_.list_dir(None).unwrap_err();
    assert!(
        matches!(err, VfsError::OutsideRoot { .. }),
        "list_dir(None) after the root was replaced: {err:?}"
    );
    assert_eq!(err.path().as_str(), "(root)");
    assert!(!err.to_string().contains(w.dir.to_str().unwrap()));
}

// ---------------------------------------------------------------------------------------------
// project::tree

fn tree_entries(items: &[(&str, EntryKind)]) -> Vec<TreeEntry> {
    items
        .iter()
        .map(|(path, kind)| TreeEntry {
            path: p(path),
            kind: *kind,
        })
        .collect()
}

#[test]
fn tree_gives_the_worked_example() {
    let mut m = MemoryFs::new();
    let w = Work::new("tree");
    for (path, bytes) in [
        (".gitignore", &b"*.log\n/target\n"[..]),
        ("B.txt", b"B\n"),
        ("a.rs", b"fn a() {}\n"),
        ("a/b.rs", b"fn b() {}\n"),
        ("x.log", b"log\n"),
        ("target/debug/k", b"k\n"),
        ("src/lib.rs", b"// lib\n"),
        ("src/.hidden", b"h\n"),
    ] {
        m.insert(&p(path), bytes);
        w.file(&format!("root/{path}"), bytes);
    }
    let all = tree_entries(&[
        (".gitignore", EntryKind::File),
        ("B.txt", EntryKind::File),
        ("a", EntryKind::Dir),
        ("a/b.rs", EntryKind::File),
        ("a.rs", EntryKind::File),
        ("src", EntryKind::Dir),
        ("src/.hidden", EntryKind::File),
        ("src/lib.rs", EntryKind::File),
    ]);
    let full = ProjectTree {
        entries: all.clone(),
        ignored: 2,
        skipped: 0,
        too_deep: 0,
        truncated: false,
    };
    let disk = DiskFs::new(&w.at("root")).unwrap();
    for (name, vfs) in [("memory", &m as &dyn Vfs), ("disk", &disk as &dyn Vfs)] {
        assert_eq!(project::tree(vfs, None, 50_000), Ok(full.clone()), "{name}");
        assert_eq!(project::tree(vfs, None, 8), Ok(full.clone()), "{name}");
        assert_eq!(
            project::tree(vfs, None, 3),
            Ok(ProjectTree {
                entries: all[..3].to_vec(),
                ignored: 0,
                skipped: 0,
                too_deep: 0,
                truncated: true,
            }),
            "{name}"
        );
        assert_eq!(
            project::tree(vfs, Some(&p("a")), 50_000),
            Ok(ProjectTree {
                entries: tree_entries(&[("a/b.rs", EntryKind::File)]),
                ignored: 0,
                skipped: 0,
                too_deep: 0,
                truncated: false,
            }),
            "{name}"
        );
        assert_eq!(
            project::tree(vfs, Some(&p("a.rs")), 50_000),
            Err(VfsError::NotAFile { path: p("a.rs") }),
            "{name}"
        );
        assert_eq!(
            project::tree(vfs, Some(&p("nope")), 50_000),
            Err(VfsError::NotFound { path: p("nope") }),
            "{name}"
        );
    }
}

#[test]
fn tree_counts_skipped_names_on_disk() {
    let w = Work::new("skip");
    w.file("root/ok.rs", b"ok");
    w.file("root/sub/fine", b"f");
    fs::write(w.at("root/sub").join(OsStr::from_bytes(b"bad\xff")), b"b").unwrap();
    fs::write(w.at("root/sub/back\\slash"), b"b").unwrap();
    let disk = DiskFs::new(&w.at("root")).unwrap();
    assert_eq!(
        project::tree(&disk, None, 50_000),
        Ok(ProjectTree {
            entries: tree_entries(&[
                ("ok.rs", EntryKind::File),
                ("sub", EntryKind::Dir),
                ("sub/fine", EntryKind::File),
            ]),
            ignored: 0,
            skipped: 2,
            too_deep: 0,
            truncated: false,
        })
    );
}
