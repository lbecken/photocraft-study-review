//! T-008 AC-5 regression guard (D-010, D-021): symlinks whose location stays inside the root
//! keep working for `read` and `write_atomic`, also when the root itself is given through a
//! symlink, and `DiskFs::new` keeps its errors. Passes on the base by design and must pass after
//! T-008. Written by the test author from the task specification alone.
//!
//! Fixtures live in fresh directories under `std::env::temp_dir()`; no process is spawned and
//! nothing here touches a FIFO, socket or device.
//!
//! On Windows (T-074) no link is followed: every link on the walk answers `outside_root`, a link
//! whose location stays inside included, so the expected answers are chosen with `cfg!(windows)`
//! (T-092); the fixture and the checks that the links are kept are the same on every platform.
#![cfg(test)]

use std::fs;
use std::path::{Path, PathBuf};

use kiln_workspace::{DiskFs, RelPath, Vfs, VfsError};

struct Work {
    dir: PathBuf,
}

impl Work {
    fn new(name: &str) -> Work {
        let dir = std::env::temp_dir().join(format!("k8g-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Work {
            dir: fs::canonicalize(&dir).unwrap(),
        }
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

/// A symlink at `link` to `target`, verbatim.
#[cfg(unix)]
fn symlink(target: &Path, link: PathBuf) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}
/// Windows: a relative target is rebuilt with `PathBuf::push` from its components (ruling Q-B
/// of T-092); a directory link when the target, resolved from the link's directory, is an
/// existing directory when the link is made, else a file link (Pinned (d)).
#[cfg(windows)]
fn symlink(target: &Path, link: PathBuf) -> std::io::Result<()> {
    let t = if target.is_absolute() {
        target.to_path_buf()
    } else {
        let mut t = PathBuf::new();
        for c in target.components() {
            t.push(c.as_os_str());
        }
        t
    };
    if link.parent().unwrap().join(&t).is_dir() {
        std::os::windows::fs::symlink_dir(&t, &link)
    } else {
        std::os::windows::fs::symlink_file(&t, &link)
    }
}

fn rp(s: &str) -> RelPath {
    RelPath::new(s).unwrap()
}

/// The links of AC-5 and their verbatim targets (relative to `work/`).
fn links(w: &Work) -> Vec<(String, PathBuf)> {
    vec![
        ("root/in".into(), "real/t.txt".into()),
        ("root/din".into(), "real".into()),
        ("root/ain".into(), w.at("root/real/t.txt")),
        ("root/in2".into(), "in".into()),
        ("root/ghost".into(), "real/new.txt".into()),
        ("root/back".into(), "../outside/back2".into()),
        ("outside/back2".into(), "../root/real/t.txt".into()),
        ("rootlink".into(), "root".into()),
    ]
}

fn fixture(name: &str) -> Work {
    let w = Work::new(name);
    fs::create_dir_all(w.at("root/real")).unwrap();
    fs::create_dir_all(w.at("outside")).unwrap();
    fs::write(w.at("root/real/t.txt"), b"target\n").unwrap();
    for (at, target) in links(&w) {
        symlink(&target, w.at(&at)).unwrap();
    }
    w
}

/// Every link still points where it did.
fn assert_links_kept(w: &Work) {
    for (at, target) in links(w) {
        assert_eq!(
            fs::read_link(w.at(&at)).unwrap(),
            target,
            "link {at} changed"
        );
    }
    assert!(
        fs::symlink_metadata(w.at("root/real/t.txt"))
            .unwrap()
            .file_type()
            .is_file(),
        "real/t.txt is no longer a regular file"
    );
}

fn no_temp(dir: &Path) {
    for e in fs::read_dir(dir).unwrap() {
        let e = e.unwrap();
        let name = e.file_name().to_string_lossy().into_owned();
        assert!(
            !name.starts_with(".kiln-tmp-"),
            "temporary file left: {name}"
        );
        if e.file_type().unwrap().is_dir() {
            no_temp(&e.path());
        }
    }
}

#[test]
fn symlinks_inside_root_still_work() {
    for given in ["root", "rootlink"] {
        let w = fixture(&format!("in-{given}"));
        let mut fs_ = DiskFs::new(&w.at(given)).unwrap();
        if cfg!(windows) {
            // T-074: every link on the walk is `outside_root` on Windows; nothing is written.
            for p in ["in", "din/t.txt", "ain", "in2", "back"] {
                assert_eq!(
                    fs_.read(&rp(p)),
                    Err(VfsError::OutsideRoot { path: rp(p) }),
                    "Windows: read({p}) with root `{given}`"
                );
                assert!(
                    !fs_.is_file(&rp(p)),
                    "Windows: is_file({p}) with root `{given}`"
                );
                assert_eq!(
                    fs_.write_atomic(&rp(p), format!("via {p}\n").as_bytes()),
                    Err(VfsError::OutsideRoot { path: rp(p) }),
                    "Windows: write_atomic({p}) with root `{given}`"
                );
                assert_eq!(
                    fs::read(w.at("root/real/t.txt")).unwrap(),
                    b"target\n",
                    "Windows: write_atomic({p}) with root `{given}` changed the target"
                );
                assert_links_kept(&w);
            }
            assert_eq!(
                fs_.write_atomic(&rp("ghost"), b"g"),
                Err(VfsError::OutsideRoot { path: rp("ghost") }),
                "Windows: write_atomic(ghost) with root `{given}`"
            );
            assert!(!w.at("root/real/new.txt").exists());
        } else {
            for p in ["in", "din/t.txt", "ain", "in2", "back"] {
                assert_eq!(
                    fs_.read(&rp(p)).unwrap(),
                    b"target\n",
                    "read({p}) with root `{given}`"
                );
                assert!(fs_.is_file(&rp(p)), "is_file({p}) with root `{given}`");
            }
            for p in ["in", "din/t.txt", "ain", "in2", "back"] {
                let bytes = format!("via {p}\n");
                fs_.write_atomic(&rp(p), bytes.as_bytes())
                    .unwrap_or_else(|e| panic!("write_atomic({p}) with root `{given}`: {e}"));
                assert_eq!(
                    fs::read(w.at("root/real/t.txt")).unwrap(),
                    bytes.as_bytes(),
                    "write_atomic({p}) with root `{given}`"
                );
                assert_links_kept(&w);
            }
            assert!(!w.at("root/real/new.txt").exists());
            fs_.write_atomic(&rp("ghost"), b"g").unwrap();
            assert_eq!(fs::read(w.at("root/real/new.txt")).unwrap(), b"g");
        }
        assert_links_kept(&w);
        no_temp(&w.dir);
    }

    // DiskFs::new keeps its errors.
    let w = Work::new("new");
    symlink(&w.at("missing"), w.at("dangling")).unwrap();
    fs::write(w.at("file"), b"x").unwrap();
    symlink(&w.at("file"), w.at("tofile")).unwrap();
    let e = DiskFs::new(&w.at("dangling")).unwrap_err();
    assert!(matches!(e, VfsError::NotFound { .. }), "{e:?}");
    assert_eq!(e.path().as_str(), "(root)");
    let e = DiskFs::new(&w.at("tofile")).unwrap_err();
    assert!(
        matches!(
            e,
            VfsError::Io {
                kind: std::io::ErrorKind::NotADirectory,
                ..
            }
        ),
        "{e:?}"
    );
    assert_eq!(e.path().as_str(), "(root)");
}
