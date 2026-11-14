//! Implementer's integration tests of T-008: `DiskFs` confines every location to its root and
//! refuses non-regular files without opening them. Variant matches live here (the acceptance
//! files assert `Display` and `path()` only). Every call that could block on a FIFO runs under
//! a hard timeout, so a regression fails instead of hanging the suite. T-036 adds the refusal of
//! a root that cannot be opened for reading, run as an unprivileged identity.

use kiln_workspace::{DiskFs, Document, RelPath, Vfs, VfsError};
use std::fs;
use std::os::unix::fs::{FileTypeExt, symlink};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use support::*;

#[cfg(test)]
mod support {
    use super::*;

    pub const SECRET: &[u8] = b"s3cr3t-content\n";
    pub const TIMEOUT: Duration = Duration::from_secs(5);

    pub fn p(s: &str) -> RelPath {
        RelPath::new(s).unwrap()
    }

    /// `work/` with `root/`, `outside/secret.txt`, `outside/d/`, `rootx/secret.txt`, under the
    /// system temp dir with a short name (socket paths are limited to 108 bytes).
    pub fn work(name: &str) -> PathBuf {
        let work = std::env::temp_dir().join(format!("k8i-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&work);
        fs::create_dir_all(work.join("root")).unwrap();
        fs::create_dir_all(work.join("outside/d")).unwrap();
        fs::create_dir_all(work.join("rootx")).unwrap();
        fs::write(work.join("outside/secret.txt"), SECRET).unwrap();
        fs::write(work.join("rootx/secret.txt"), SECRET).unwrap();
        assert_eq!(fs::read(work.join("outside/secret.txt")).unwrap(), SECRET);
        assert_eq!(fs::read(work.join("rootx/secret.txt")).unwrap(), SECRET);
        work
    }

    /// Every entry under `dir`, recursively, symlinks not followed: (relative path, type, link
    /// target or bytes).
    pub fn snapshot(dir: &Path) -> Vec<(String, String, Vec<u8>)> {
        fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, String, Vec<u8>)>) {
            let mut entries: Vec<_> = fs::read_dir(dir).unwrap().map(|e| e.unwrap()).collect();
            entries.sort_by_key(|e| e.file_name());
            for e in entries {
                let path = e.path();
                let rel = path
                    .strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                let ft = fs::symlink_metadata(&path).unwrap().file_type();
                if ft.is_symlink() {
                    let t = fs::read_link(&path).unwrap();
                    out.push((rel, "link".into(), t.to_string_lossy().as_bytes().to_vec()));
                } else if ft.is_dir() {
                    out.push((rel, "dir".into(), Vec::new()));
                    walk(base, &path, out);
                } else if ft.is_file() {
                    out.push((rel, "file".into(), fs::read(&path).unwrap()));
                } else {
                    out.push((rel, format!("{ft:?}"), Vec::new()));
                }
            }
        }
        let mut out = Vec::new();
        walk(dir, dir, &mut out);
        out
    }

    pub fn no_temp_files(dir: &Path) {
        for (rel, _, _) in snapshot(dir) {
            assert!(!rel.contains(".kiln-tmp-"), "temporary file left: {rel}");
        }
    }

    /// Run `f` on another thread; fail (never hang) if it takes longer than `TIMEOUT`.
    pub fn bounded<T: Send + 'static>(what: &str, f: impl FnOnce() -> T + Send + 'static) -> T {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(f());
        });
        rx.recv_timeout(TIMEOUT)
            .unwrap_or_else(|_| panic!("{what} blocked for more than {TIMEOUT:?}"))
    }

    /// `mkfifo <path>` under a timeout.
    pub fn mkfifo(path: &Path) {
        let mut child = Command::new("mkfifo").arg(path).spawn().unwrap();
        let deadline = Instant::now() + TIMEOUT;
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            if Instant::now() > deadline {
                let _ = child.kill();
                panic!("mkfifo timed out");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(fs::symlink_metadata(path).unwrap().file_type().is_fifo());
    }

    pub fn outside(path: &str) -> VfsError {
        VfsError::OutsideRoot { path: p(path) }
    }

    pub fn not_a_file(path: &str) -> VfsError {
        VfsError::NotAFile { path: p(path) }
    }

    pub fn check_text(e: &VfsError, work: &Path) {
        let text = e.to_string();
        let without = text.replace("resolves outside the root", "");
        assert!(!text.contains(work.to_string_lossy().as_ref()), "{text}");
        assert!(
            !without.contains("outside") && !without.contains("rootx"),
            "{text}"
        );
    }
}

#[test]
fn escapes_are_refused_at_read_write_and_is_file() {
    let work = work("esc");
    let root = work.join("root");
    symlink("../outside/secret.txt", root.join("rel")).unwrap();
    symlink(work.join("outside/secret.txt"), root.join("abs")).unwrap();
    symlink("../outside", root.join("dlink")).unwrap();
    symlink("rel", root.join("chain")).unwrap();
    symlink("../outside/d", root.join("s")).unwrap();
    symlink("s/../secret.txt", root.join("t")).unwrap();
    symlink("../rootx/secret.txt", root.join("px")).unwrap();
    symlink("../outside/none.txt", root.join("gone")).unwrap();
    symlink("../outside/none/x", root.join("gone2")).unwrap();
    // Through the root given as a symlink, too.
    symlink("root", work.join("rootlink")).unwrap();
    let before = snapshot(&work);
    for root_arg in [work.join("root"), work.join("rootlink")] {
        let mut fs_ = DiskFs::new(&root_arg).unwrap();
        for path in [
            "rel",
            "abs",
            "dlink/secret.txt",
            "chain",
            "t",
            "px",
            "gone",
            "gone2",
        ] {
            let e = fs_.read(&p(path)).unwrap_err();
            assert_eq!(e, outside(path), "read {path}");
            assert_eq!(e.to_string(), format!("{path}: resolves outside the root"));
            check_text(&e, &work);
            assert!(!fs_.is_file(&p(path)), "is_file {path}");
        }
        for path in [
            "rel",
            "abs",
            "dlink/secret.txt",
            "dlink/new.txt",
            "dlink/missing/new.txt",
            "t",
            "px",
            "gone",
            "gone2",
        ] {
            assert_eq!(
                fs_.write_atomic(&p(path), b"PWNED"),
                Err(outside(path)),
                "write {path}"
            );
        }
        let e = Document::open(&fs_, &p("rel")).unwrap_err();
        assert_eq!(e.to_string(), "cannot open rel: resolves outside the root");
    }
    assert_eq!(snapshot(&work), before);
    no_temp_files(&work);
}

#[test]
fn links_inside_and_excursions_back_inside_are_followed() {
    let work = work("in");
    let root = work.join("root");
    fs::create_dir(root.join("real")).unwrap();
    fs::write(root.join("real/t.txt"), b"target\n").unwrap();
    symlink("real/t.txt", root.join("in")).unwrap();
    symlink("real", root.join("din")).unwrap();
    symlink(root.join("real/t.txt"), root.join("ain")).unwrap();
    symlink("in", root.join("in2")).unwrap();
    symlink("real/new.txt", root.join("ghost")).unwrap();
    symlink("../outside/back2", root.join("back")).unwrap();
    symlink("../root/real/t.txt", work.join("outside/back2")).unwrap();
    // `..` after a directory link that stays inside.
    symlink("din/../real/t.txt", root.join("dd")).unwrap();
    symlink("root", work.join("rootlink")).unwrap();
    for root_arg in [work.join("root"), work.join("rootlink")] {
        let mut fs_ = DiskFs::new(&root_arg).unwrap();
        for path in ["in", "din/t.txt", "ain", "in2", "back", "dd"] {
            fs::write(root.join("real/t.txt"), b"target\n").unwrap();
            assert_eq!(fs_.read(&p(path)).unwrap(), b"target\n", "read {path}");
            assert!(fs_.is_file(&p(path)));
            fs_.write_atomic(&p(path), path.as_bytes()).unwrap();
            assert_eq!(fs::read(root.join("real/t.txt")).unwrap(), path.as_bytes());
        }
        for link in ["in", "din", "ain", "in2", "back", "dd", "ghost"] {
            assert!(
                fs::symlink_metadata(root.join(link))
                    .unwrap()
                    .file_type()
                    .is_symlink()
            );
        }
        assert_eq!(
            fs_.read(&p("ghost")),
            Err(VfsError::NotFound { path: p("ghost") })
        );
        fs_.write_atomic(&p("ghost"), b"g").unwrap();
        assert_eq!(fs::read(root.join("real/new.txt")).unwrap(), b"g");
        fs::remove_file(root.join("real/new.txt")).unwrap();
    }
    no_temp_files(&work);
    // DiskFs::new keeps its errors.
    symlink("nowhere", work.join("dangling")).unwrap();
    let e = DiskFs::new(&work.join("dangling")).unwrap_err();
    assert!(matches!(e, VfsError::NotFound { .. }));
    assert_eq!(e.path().as_str(), "(root)");
    symlink("outside/secret.txt", work.join("filelink")).unwrap();
    assert!(matches!(
        DiskFs::new(&work.join("filelink")).unwrap_err(),
        VfsError::Io {
            kind: std::io::ErrorKind::NotADirectory,
            ..
        }
    ));
}

#[test]
fn non_regular_files_are_refused_without_blocking() {
    let work = work("fifo");
    let root = work.join("root");
    mkfifo(&root.join("p"));
    mkfifo(&work.join("outside/fifo"));
    // Bind at a short path, then move into the root (same file system).
    let sock = work.join("s");
    let _listener = UnixListener::bind(&sock).unwrap();
    fs::rename(&sock, root.join("s")).unwrap();
    fs::create_dir(root.join("d")).unwrap();
    fs::write(root.join("d/keep"), b"keep").unwrap();
    symlink("p", root.join("lp")).unwrap();
    symlink("../outside/fifo", root.join("po")).unwrap();
    symlink(".", root.join("self")).unwrap();
    symlink("..", root.join("up")).unwrap();
    symlink("/dev/null", root.join("dn")).unwrap();
    let before = snapshot(&work);
    let root2 = root.clone();
    bounded("reads", move || {
        let mut fs_ = DiskFs::new(&root2).unwrap();
        for path in ["p", "s", "d", "lp", "self"] {
            let e = fs_.read(&p(path)).unwrap_err();
            assert_eq!(e, not_a_file(path), "read {path}");
            assert_eq!(e.to_string(), format!("{path}: not a regular file"));
            assert!(!fs_.is_file(&p(path)));
        }
        for path in ["p", "s", "d", "lp"] {
            assert_eq!(
                fs_.write_atomic(&p(path), b"x"),
                Err(not_a_file(path)),
                "write {path}"
            );
        }
        for path in ["po", "up", "dn"] {
            assert_eq!(fs_.read(&p(path)), Err(outside(path)), "read {path}");
            assert_eq!(
                fs_.write_atomic(&p(path), b"x"),
                Err(outside(path)),
                "write {path}"
            );
            assert!(!fs_.is_file(&p(path)));
        }
    });
    assert_eq!(snapshot(&work), before);
    assert!(
        fs::symlink_metadata(root.join("s"))
            .unwrap()
            .file_type()
            .is_socket()
    );
    no_temp_files(&work);
    bounded("/dev/null", || {
        let fs_ = DiskFs::new(Path::new("/dev")).unwrap();
        assert_eq!(fs_.read(&p("null")), Err(not_a_file("null")));
        assert!(!fs_.is_file(&p("null")));
    });
}

#[test]
fn swaps_between_open_and_save_are_checked_again() {
    let work = work("swap");
    let root = work.join("root");
    let mut fs_ = DiskFs::new(&root).unwrap();
    fs::write(root.join("a.txt"), b"a\n").unwrap();
    fs::create_dir(root.join("sub")).unwrap();
    fs::write(root.join("sub/f.txt"), b"f\n").unwrap();
    fs::write(work.join("outside/f.txt"), b"outside f\n").unwrap();
    fs::write(root.join("g.txt"), b"g\n").unwrap();
    fs::write(root.join("c.txt"), b"c\n").unwrap();
    fs::create_dir(root.join("real")).unwrap();
    fs::write(root.join("real/t.txt"), b"c\n").unwrap();

    let mut docs: Vec<Document> = ["a.txt", "sub/f.txt", "g.txt", "c.txt"]
        .iter()
        .map(|path| {
            let mut d = Document::open(&fs_, &p(path)).unwrap();
            d.buffer_mut().insert(0, "EDIT ").unwrap();
            d
        })
        .collect();
    fs::remove_file(root.join("a.txt")).unwrap();
    symlink("../outside/secret.txt", root.join("a.txt")).unwrap();
    fs::rename(root.join("sub"), root.join("sub-old")).unwrap();
    symlink("../outside", root.join("sub")).unwrap();
    fs::remove_file(root.join("g.txt")).unwrap();
    mkfifo(&root.join("g.txt"));
    fs::remove_file(root.join("c.txt")).unwrap();
    symlink("real/t.txt", root.join("c.txt")).unwrap();
    let before = snapshot(&work);

    assert_eq!(docs[0].save(&mut fs_), Err(outside("a.txt")));
    assert_eq!(docs[1].save(&mut fs_), Err(outside("sub/f.txt")));
    let mut g = docs.remove(2);
    let (g, mut fs_) = bounded("save onto FIFO", move || {
        assert_eq!(g.save(&mut fs_), Err(not_a_file("g.txt")));
        (g, fs_)
    });
    assert_eq!(snapshot(&work), before);
    assert!(docs[0].is_dirty() && docs[1].is_dirty() && g.is_dirty());
    no_temp_files(&work);

    docs[2].save(&mut fs_).unwrap();
    assert_eq!(fs::read(root.join("real/t.txt")).unwrap(), b"EDIT c\n");
    assert!(
        fs::symlink_metadata(root.join("c.txt"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read(work.join("outside/secret.txt")).unwrap(), SECRET);
}

#[test]
fn walk_corner_cases() {
    let work = work("corner");
    let root = work.join("root");
    let mut fs_ = DiskFs::new(&root).unwrap();
    fs::write(root.join("f.txt"), b"f").unwrap();
    // A loop inside the root: I/O error as before; a loop entered outside reveals nothing.
    symlink("loop", root.join("loop")).unwrap();
    let e = fs_.read(&p("loop")).unwrap_err();
    assert_eq!(
        e.to_string(),
        "loop: I/O error: too many levels of symbolic links"
    );
    symlink("../outside/oloop", root.join("toloop")).unwrap();
    symlink("oloop", work.join("outside/oloop")).unwrap();
    assert_eq!(fs_.read(&p("toloop")), Err(outside("toloop")));
    // Exactly 40 hops are followed, 41 are not.
    let mut prev = "f.txt".to_owned();
    for i in 0..41 {
        let name = format!("h{i}");
        symlink(&prev, root.join(&name)).unwrap();
        prev = name;
    }
    assert_eq!(fs_.read(&p("h39")).unwrap(), b"f");
    assert!(matches!(fs_.read(&p("h40")), Err(VfsError::Io { .. })));
    // A non-directory or missing component inside the root is NotFound.
    for path in ["f.txt/x", "nodir/x"] {
        assert_eq!(
            fs_.read(&p(path)),
            Err(VfsError::NotFound { path: p(path) })
        );
        assert_eq!(
            fs_.write_atomic(&p(path), b"x"),
            Err(VfsError::NotFound { path: p(path) })
        );
    }
    // A link to the root's own parent and back in by name is inside.
    symlink("../root/f.txt", root.join("viaparent")).unwrap();
    assert_eq!(fs_.read(&p("viaparent")).unwrap(), b"f");
    // An absolute link through `/` to a file inside the root.
    let canonical = fs::canonicalize(&root).unwrap();
    symlink(canonical.join("f.txt"), root.join("absin")).unwrap();
    assert_eq!(fs_.read(&p("absin")).unwrap(), b"f");
    no_temp_files(&work);
}

/// T-036: the environment variable that marks the re-run of this test binary as the
/// unprivileged identity U; its value is the fixture directory.
const T036_CHILD: &str = "KILN_T036_CONFINEMENT_CHILD";
/// What the child run writes to its marker file once every assertion has run.
const T036_DONE: &str = "unreadable roots: all assertions ran\n";

/// T-036 (Interface 1): a root that cannot be opened for reading is refused by `DiskFs::new`
/// whatever its search permission, with `Io { kind: PermissionDenied }` naming `(root)`; a
/// root readable at start is held, and stays protected after it is made unreadable and then
/// removed and replaced. The gate runs as uid 0, which bypasses permission bits, so the
/// assertions run in a re-run of this test binary as U (uid/gid 65534 when the test runs as
/// uid 0, the test's own identity otherwise), on a fixture owned by U. The parent fails, naming
/// the precondition, if the child cannot be started, fails, or did not reach its end.
#[test]
fn root_that_cannot_be_opened_for_reading_is_refused_by_new() {
    if let Some(work) = std::env::var_os(T036_CHILD) {
        t036_child(Path::new(&work));
    } else {
        t036_parent();
    }
}

/// The parent: builds the fixture, hands it to U, runs the child as U under a timeout.
fn t036_parent() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt, chown};
    use std::os::unix::process::CommandExt;
    let work = work("t036-unreadable");
    fs::set_permissions(&work, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(work.join("root/f.txt"), b"in\n").unwrap();
    // The owner of a file we just created is our effective uid.
    let euid = fs::metadata(work.join("root/f.txt")).unwrap().uid();
    let mut child = Command::new(std::env::current_exe().unwrap());
    child
        .args([
            "--exact",
            "root_that_cannot_be_opened_for_reading_is_refused_by_new",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(T036_CHILD, &work);
    if euid == 0 {
        // Hand the fixture to uid/gid 65534 and run the child as that identity.
        let mut stack = vec![work.clone()];
        while let Some(path) = stack.pop() {
            chown(&path, Some(65534), Some(65534)).unwrap();
            if fs::symlink_metadata(&path).unwrap().is_dir() {
                stack.extend(fs::read_dir(&path).unwrap().map(|e| e.unwrap().path()));
            }
        }
        child.uid(65534).gid(65534);
    }
    let mut child = child.spawn().unwrap_or_else(|e| {
        panic!("precondition: could not run this test binary as the unprivileged identity: {e}")
    });
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            panic!("the unprivileged child run took more than 10 s");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(
        status.success(),
        "the unprivileged child run failed: {status}"
    );
    assert_eq!(
        fs::read(work.join("done")).unwrap_or_default(),
        T036_DONE.as_bytes(),
        "the unprivileged child run did not reach the end of its assertions"
    );
    let _ = fs::remove_dir_all(&work);
}

/// The child, run as U on the fixture `work` it owns.
fn t036_child(work: &Path) {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let set_mode = |path: &Path, mode: u32| {
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    };
    let root = work.join("root");

    // Controls: U reaches the fixture, and U cannot override permission checks.
    set_mode(&root, 0o711);
    let fs_ = DiskFs::new(&root).unwrap_or_else(|e| {
        panic!("precondition: a readable root must be accepted for the test identity: {e}")
    });
    assert_eq!(fs_.read(&p("f.txt")).unwrap(), b"in\n");
    fs::write(root.join("locked.txt"), b"locked\n").unwrap();
    set_mode(&root.join("locked.txt"), 0o000);
    assert!(
        matches!(
            fs_.read(&p("locked.txt")),
            Err(VfsError::Io {
                kind: std::io::ErrorKind::PermissionDenied,
                ..
            })
        ),
        "precondition: the test identity must not bypass permission checks"
    );
    drop(fs_);
    fs::remove_file(root.join("locked.txt")).unwrap();

    // Every root that cannot be opened for reading is refused, whatever its search bit.
    for mode in [0o311, 0o300, 0o200, 0o100, 0o000] {
        let before = snapshot(work);
        set_mode(&root, mode);
        let err = DiskFs::new(&root).expect_err(&format!("mode {mode:o} accepted"));
        assert!(
            matches!(
                err,
                VfsError::Io {
                    kind: std::io::ErrorKind::PermissionDenied,
                    ..
                }
            ),
            "mode {mode:o}: {err:?}"
        );
        assert_eq!(err.path(), &p("(root)"), "mode {mode:o}");
        assert!(
            !err.to_string().contains(work.to_string_lossy().as_ref()),
            "mode {mode:o}: {err}"
        );
        // Through a symlink to the root as well.
        let _ = fs::remove_file(work.join("rootlink"));
        symlink("root", work.join("rootlink")).unwrap();
        let err = DiskFs::new(&work.join("rootlink")).expect_err("link to unreadable root");
        assert!(matches!(
            err,
            VfsError::Io {
                kind: std::io::ErrorKind::PermissionDenied,
                ..
            }
        ));
        fs::remove_file(work.join("rootlink")).unwrap();
        set_mode(&root, 0o711);
        assert_eq!(snapshot(work), before, "mode {mode:o}: something changed");
    }

    // A root readable (and so held) at start stays protected after it is made unreadable,
    // then removed and replaced by a link to a new directory.
    set_mode(&root, 0o711);
    let mut fs_ = DiskFs::new(&root).unwrap();
    let mut doc = Document::open(&fs_, &p("f.txt")).unwrap();
    doc.buffer_mut().insert(0, "PWNED ").unwrap();
    set_mode(&root, 0o311);
    let old = fs::metadata(&root).unwrap();
    set_mode(&root, 0o711);
    fs::remove_dir_all(&root).unwrap();
    let mut replacement = work.join("c0");
    for i in 0..64 {
        replacement = work.join(format!("c{i}"));
        fs::create_dir(&replacement).unwrap();
        let m = fs::metadata(&replacement).unwrap();
        assert_ne!(
            (m.dev(), m.ino()),
            (old.dev(), old.ino()),
            "the held root's inode number was reused"
        );
    }
    fs::write(replacement.join("secret.txt"), SECRET).unwrap();
    symlink(replacement.file_name().unwrap(), &root).unwrap();
    assert_eq!(fs::read(root.join("secret.txt")).unwrap(), SECRET);
    let before = snapshot(work);
    assert_eq!(fs_.read(&p("secret.txt")), Err(outside("secret.txt")));
    assert!(!fs_.is_file(&p("secret.txt")));
    assert_eq!(
        fs_.write_atomic(&p("new.txt"), b"PWNED"),
        Err(outside("new.txt"))
    );
    assert_eq!(fs_.list_dir(None), Err(outside("(root)")));
    assert_eq!(doc.save(&mut fs_), Err(outside("f.txt")));
    assert!(doc.is_dirty());
    assert_eq!(snapshot(work), before);
    no_temp_files(work);

    fs::write(work.join("done"), T036_DONE).unwrap();
}
