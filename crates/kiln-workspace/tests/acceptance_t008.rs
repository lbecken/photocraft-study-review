//! T-008 acceptance tests (Rust API of `kiln-workspace`): every disk path is confined to the
//! workspace root at open and save, and non-regular files are refused without blocking.
//! Written by the test author from the task specification alone (AC-1, AC-2, AC-4, AC-6, AC-7,
//! AC-8); the CLI and control-protocol halves are in `apps/kiln-cli/tests/acceptance_t008_cli.rs`.
//!
//! Errors are asserted by `Display` and `path()` only (never by variant), so this file compiles
//! on the base, where `VfsError::OutsideRoot` does not exist, and fails there by assertion or by
//! timeout. Every escape first shows, through `std::fs`, that its outside target really is
//! reachable through the link, and every refusal is followed by a snapshot comparison of the
//! whole fixture, so no test can pass with the check removed.
//!
//! Each fixture lives in a fresh directory `work/` under `std::env::temp_dir()` (short names, so
//! socket paths stay below 108 bytes). Every call that touches a FIFO, socket or device runs on
//! a helper thread and must return within [`HANG`]; on a hang the test unblocks the FIFO and
//! fails, it never blocks the suite (D-028 (8)).
#![cfg(test)]

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::{FileTypeExt, symlink};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use kiln_workspace::{DiskFs, Document, MemoryFs, RelPath, Vfs, VfsError};

/// Bytes of every outside file an escape must not reach.
const SECRET: &[u8] = b"s3cr3t-content\n";
/// How long a single VFS call may take before it counts as blocked.
const HANG: Duration = Duration::from_secs(5);
const OUTSIDE: &str = "resolves outside the root";
const NOT_A_FILE: &str = "not a regular file";

// ---------------------------------------------------------------------------------------------
// Fixtures.

/// A fresh `work/` with `root/`, `outside/` and `outside/secret.txt`; removed on drop.
struct Work {
    dir: PathBuf,
}

impl Work {
    fn new(name: &str) -> Work {
        let dir = std::env::temp_dir().join(format!("k8a-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let dir = fs::canonicalize(&dir).unwrap();
        fs::create_dir(dir.join("root")).unwrap();
        fs::create_dir(dir.join("outside")).unwrap();
        fs::write(dir.join("outside/secret.txt"), SECRET).unwrap();
        Work { dir }
    }

    /// `work/<rel>`.
    fn at(&self, rel: &str) -> PathBuf {
        self.dir.join(rel)
    }

    fn root(&self) -> PathBuf {
        self.at("root")
    }

    /// Create the symlink `work/<at>` pointing to `target` (verbatim).
    fn link(&self, at: &str, target: &str) {
        symlink(target, self.at(at)).unwrap();
    }

    fn file(&self, at: &str, bytes: &[u8]) {
        let p = self.at(at);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, bytes).unwrap();
    }

    fn fs(&self) -> DiskFs {
        DiskFs::new(&self.root()).unwrap()
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

fn rp(s: &str) -> RelPath {
    RelPath::new(s).unwrap_or_else(|e| panic!("RelPath::new({s:?}) rejected: {e:?}"))
}

/// Make a FIFO at `path` with the `mkfifo` utility, killed after [`HANG`].
fn mkfifo(path: &Path) {
    let mut child = Command::new("mkfifo")
        .arg(path)
        .stdin(Stdio::null())
        .spawn()
        .expect("spawn mkfifo");
    let deadline = Instant::now() + HANG;
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("mkfifo did not finish within {HANG:?}");
        }
        thread::sleep(Duration::from_millis(5));
    };
    assert!(status.success(), "mkfifo {} failed", path.display());
    assert!(fs::symlink_metadata(path).unwrap().file_type().is_fifo());
}

/// Bind a Unix socket at a short path in the same file system and rename it to `path`. The
/// listener is returned so the socket stays live for the test.
fn socket_at(path: &Path) -> UnixListener {
    static N: AtomicUsize = AtomicUsize::new(0);
    let short = std::env::temp_dir().join(format!(
        "k8s{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_file(&short);
    let listener = UnixListener::bind(&short).unwrap();
    fs::rename(&short, path).unwrap();
    assert!(fs::symlink_metadata(path).unwrap().file_type().is_socket());
    listener
}

/// Every entry under `dir`, recursively, symlinks not followed: type, link target, bytes.
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
            } else if ft.is_fifo() {
                "fifo".to_owned()
            } else if ft.is_socket() {
                "socket".to_owned()
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

/// No `.kiln-tmp-*` anywhere under `dir`.
fn assert_no_temp(dir: &Path) {
    for name in snapshot(dir).keys() {
        let last = name.rsplit('/').next().unwrap_or(name);
        assert!(
            !last.starts_with(".kiln-tmp-"),
            "temporary file left behind: {name}"
        );
    }
}

/// The escape is real: reading `root/<link>` through the kernel gives `expected`.
fn assert_reaches(work: &Work, link: &str, expected: &[u8]) {
    assert_eq!(
        fs::read(work.root().join(link)).unwrap(),
        expected,
        "fixture: `{link}` does not reach the expected outside content"
    );
}

/// `text` names no host path and no link target (AC-8). The pinned phrase `resolves outside the
/// root` itself contains `outside`, so it is removed before the check (see the test-author log).
fn assert_clean(text: &str, work: &Work) {
    assert!(!text.contains(&work.text()), "names the work path: {text}");
    let text = text.replace(OUTSIDE, "");
    assert!(!text.contains("outside"), "names `outside`: {text}");
    assert!(!text.contains("rootx"), "names `rootx`: {text}");
}

/// `r` is an error whose `Display` is exactly `<p>: <what>` and whose `path()` is `p`.
fn assert_refused<T>(op: &str, r: Result<T, VfsError>, p: &str, what: &str, work: &Work) {
    match r {
        Ok(_) => panic!("{op}({p}) succeeded; expected `{p}: {what}`"),
        Err(e) => {
            assert_eq!(e.to_string(), format!("{p}: {what}"), "{op}({p})");
            assert_eq!(e.path().as_str(), p, "{op}({p}) path()");
            assert_clean(&e.to_string(), work);
        }
    }
}

/// Run `f` on a helper thread; fail if it does not return within [`HANG`]. On a hang, every
/// FIFO in `fifos` is opened read-write once, which releases a blocked open, so the stuck
/// thread can finish.
fn timed<T: Send + 'static>(
    what: &str,
    fifos: &[PathBuf],
    f: impl FnOnce() -> T + Send + 'static,
) -> T {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(f());
    });
    match rx.recv_timeout(HANG) {
        Ok(v) => v,
        Err(RecvTimeoutError::Timeout) => {
            for fifo in fifos {
                let _ = fs::OpenOptions::new().read(true).write(true).open(fifo);
            }
            panic!("{what} blocked: no result within {HANG:?}");
        }
        Err(RecvTimeoutError::Disconnected) => panic!("{what} panicked"),
    }
}

fn read_t(root: &Path, p: &str, fifos: &[PathBuf]) -> Result<Vec<u8>, VfsError> {
    let (root, path) = (root.to_path_buf(), rp(p));
    timed(&format!("read({p})"), fifos, move || {
        DiskFs::new(&root).unwrap().read(&path)
    })
}

fn write_t(root: &Path, p: &str, bytes: &[u8], fifos: &[PathBuf]) -> Result<(), VfsError> {
    let (root, path, bytes) = (root.to_path_buf(), rp(p), bytes.to_vec());
    timed(&format!("write_atomic({p})"), fifos, move || {
        DiskFs::new(&root).unwrap().write_atomic(&path, &bytes)
    })
}

fn is_file_t(root: &Path, p: &str, fifos: &[PathBuf]) -> bool {
    let (root, path) = (root.to_path_buf(), rp(p));
    timed(&format!("is_file({p})"), fifos, move || {
        DiskFs::new(&root).unwrap().is_file(&path)
    })
}

/// AC-1/AC-2 fixtures: the escaping links of the specification.
fn escape_fixture(name: &str) -> Work {
    let w = Work::new(name);
    fs::create_dir(w.at("outside/d")).unwrap();
    w.file("rootx/secret.txt", SECRET);
    let abs = w.at("outside/secret.txt");
    w.link("root/rel", "../outside/secret.txt");
    w.link("root/abs", abs.to_str().unwrap());
    w.link("root/dlink", "../outside");
    w.link("root/chain", "rel");
    w.link("root/s", "../outside/d");
    w.link("root/t", "s/../secret.txt");
    w.link("root/px", "../rootx/secret.txt");
    w.link("rootlink", "root");
    for link in ["rel", "abs", "dlink/secret.txt", "chain", "t", "px"] {
        assert_reaches(&w, link, SECRET);
    }
    // `t` is outside only physically: lexically it would be `root/secret.txt`.
    assert!(!w.at("root/secret.txt").exists());
    w
}

// ---------------------------------------------------------------------------------------------
// AC-1

#[test]
fn open_through_escaping_symlinks_is_refused() {
    let w = escape_fixture("open");
    let before = snapshot(&w.dir);
    // The root given directly and through a symlink to it (Definitions: Root).
    for root in [w.root(), w.at("rootlink")] {
        let fs_ = DiskFs::new(&root).unwrap();
        for p in ["rel", "abs", "dlink/secret.txt", "chain", "t", "px"] {
            assert_refused("read", fs_.read(&rp(p)), p, OUTSIDE, &w);
            assert!(!fs_.is_file(&rp(p)), "is_file({p}) is true");
        }
        match Document::open(&fs_, &rp("rel")) {
            Ok(_) => panic!("Document::open(rel) succeeded"),
            Err(e) => {
                assert_eq!(e.to_string(), "cannot open rel: resolves outside the root");
                assert_clean(&e.to_string(), &w);
            }
        }
    }
    assert_eq!(snapshot(&w.dir), before);
}

// ---------------------------------------------------------------------------------------------
// AC-2

#[test]
fn save_through_escaping_symlinks_touches_nothing() {
    let w = escape_fixture("save");
    w.link("root/gone", "../outside/none.txt");
    // Preconditions of the targets that do not exist yet.
    assert!(fs::metadata(w.root().join("dlink")).unwrap().is_dir());
    assert_eq!(
        fs::canonicalize(w.root().join("dlink")).unwrap(),
        w.at("outside")
    );
    assert!(!w.at("outside/new.txt").exists());
    assert!(!w.at("outside/none.txt").exists());
    assert_eq!(
        fs::read_link(w.root().join("gone")).unwrap(),
        Path::new("../outside/none.txt")
    );
    let before = snapshot(&w.dir);
    for root in [w.root(), w.at("rootlink")] {
        let mut fs_ = DiskFs::new(&root).unwrap();
        for p in [
            "rel",
            "abs",
            "dlink/secret.txt",
            "dlink/new.txt",
            "chain",
            "t",
            "px",
            "gone",
        ] {
            assert_refused(
                "write_atomic",
                fs_.write_atomic(&rp(p), b"PWNED"),
                p,
                OUTSIDE,
                &w,
            );
            assert_eq!(snapshot(&w.dir), before, "after write_atomic({p})");
        }
    }
    assert_eq!(fs::read(w.at("outside/secret.txt")).unwrap(), SECRET);
    assert_eq!(fs::read(w.at("rootx/secret.txt")).unwrap(), SECRET);
    assert!(!w.at("outside/new.txt").exists());
    assert!(!w.at("outside/none.txt").exists());
    assert_no_temp(&w.dir);
}

// ---------------------------------------------------------------------------------------------
// AC-4

/// `Document::save` returns the `VfsError`; the command layer reports it as
/// `cannot save {VfsError}` (Interface, "Commands"), which is what is compared here.
fn save_message(e: &VfsError) -> String {
    format!("cannot save {e}")
}

#[test]
fn link_swapped_in_after_open_is_refused_at_save() {
    let w = Work::new("swap");
    w.file("root/a.txt", b"a original\n");
    w.file("root/sub/f.txt", b"sub original\n");
    w.file("root/c.txt", b"c original\n");
    w.file("outside/f.txt", b"outside f\n");
    w.file("root/real/t.txt", b"c original\n");
    let mut fs_ = w.fs();

    let mut a = Document::open(&fs_, &rp("a.txt")).unwrap();
    let mut f = Document::open(&fs_, &rp("sub/f.txt")).unwrap();
    let mut c = Document::open(&fs_, &rp("c.txt")).unwrap();
    for d in [&mut a, &mut f, &mut c] {
        d.buffer_mut().insert(0, "EDIT ").unwrap();
        assert!(d.is_dirty());
    }

    // (a) a.txt becomes a link to the outside secret.
    fs::remove_file(w.at("root/a.txt")).unwrap();
    w.link("root/a.txt", "../outside/secret.txt");
    assert_reaches(&w, "a.txt", SECRET);
    // (b) directory component `sub` becomes a link to ../outside, which holds a real f.txt.
    fs::remove_dir_all(w.at("root/sub")).unwrap();
    w.link("root/sub", "../outside");
    assert_reaches(&w, "sub/f.txt", b"outside f\n");
    let before = snapshot(&w.dir);

    let e = a.save(&mut fs_).unwrap_err();
    assert_eq!(
        save_message(&e),
        "cannot save a.txt: resolves outside the root"
    );
    assert_eq!(e.path().as_str(), "a.txt");
    assert_clean(&e.to_string(), &w);
    assert!(a.is_dirty(), "a.txt document is no longer dirty");

    let e = f.save(&mut fs_).unwrap_err();
    assert_eq!(
        save_message(&e),
        "cannot save sub/f.txt: resolves outside the root"
    );
    assert_eq!(e.path().as_str(), "sub/f.txt");
    assert_clean(&e.to_string(), &w);
    assert!(f.is_dirty(), "sub/f.txt document is no longer dirty");

    assert_eq!(snapshot(&w.dir), before);
    assert_eq!(fs::read(w.at("outside/secret.txt")).unwrap(), SECRET);
    assert_eq!(fs::read(w.at("outside/f.txt")).unwrap(), b"outside f\n");
    assert_eq!(
        fs::read_link(w.at("root/a.txt")).unwrap(),
        Path::new("../outside/secret.txt")
    );
    assert_no_temp(&w.dir);

    // (c) c.txt becomes a link to real/t.txt inside the root, holding the bytes c.txt had.
    fs::remove_file(w.at("root/c.txt")).unwrap();
    w.link("root/c.txt", "real/t.txt");
    assert_reaches(&w, "c.txt", b"c original\n");
    c.save(&mut fs_).unwrap();
    assert!(!c.is_dirty());
    assert_eq!(
        fs::read(w.at("root/real/t.txt")).unwrap(),
        b"EDIT c original\n"
    );
    assert_eq!(
        fs::read_link(w.at("root/c.txt")).unwrap(),
        Path::new("real/t.txt")
    );
    assert_no_temp(&w.dir);

    // Each call is checked again: the refused saves stay refused.
    assert_eq!(
        save_message(&a.save(&mut fs_).unwrap_err()),
        "cannot save a.txt: resolves outside the root"
    );
    assert_eq!(fs::read(w.at("outside/secret.txt")).unwrap(), SECRET);
}

// ---------------------------------------------------------------------------------------------
// AC-6

#[test]
fn non_regular_files_are_refused_without_blocking() {
    let w = Work::new("nonreg");
    let fifo = w.at("root/p");
    mkfifo(&fifo);
    let _listener = socket_at(&w.at("root/s"));
    w.file("root/d/x.txt", b"keep\n");
    w.link("root/lp", "p");
    let fifos = vec![fifo.clone()];
    let before = snapshot(&w.dir);

    for p in ["p", "s", "d", "lp"] {
        assert_refused("read", read_t(&w.root(), p, &fifos), p, NOT_A_FILE, &w);
        assert!(!is_file_t(&w.root(), p, &fifos), "is_file({p}) is true");
    }
    let root = w.root();
    let opened = timed("Document::open(p)", &fifos, move || {
        Document::open(&DiskFs::new(&root).unwrap(), &rp("p")).map(|_| ())
    });
    assert_eq!(
        opened.unwrap_err().to_string(),
        "cannot open p: not a regular file"
    );
    assert_eq!(snapshot(&w.dir), before);

    // A device, read-only: nothing is ever written under /dev.
    let null = fs::metadata("/dev/null").unwrap();
    assert!(null.file_type().is_char_device());
    let dev = Path::new("/dev");
    assert_refused("read", read_t(dev, "null", &[]), "null", NOT_A_FILE, &w);
    assert!(!is_file_t(dev, "null", &[]));

    // MemoryFs: same behaviour, the widened message.
    let mut mem = MemoryFs::new();
    mem.insert(&rp("d/x.txt"), b"keep\n");
    assert_refused("MemoryFs::read", mem.read(&rp("d")), "d", NOT_A_FILE, &w);
    assert_refused(
        "MemoryFs::write_atomic",
        mem.write_atomic(&rp("d"), b"x"),
        "d",
        NOT_A_FILE,
        &w,
    );
    assert!(!mem.is_file(&rp("d")));
    assert_eq!(mem.read(&rp("d/x.txt")).unwrap(), b"keep\n");
}

// ---------------------------------------------------------------------------------------------
// AC-7

#[test]
fn save_onto_non_regular_files_leaves_them_alone() {
    let w = Work::new("nonregsave");
    let fifo = w.at("root/p");
    mkfifo(&fifo);
    let _listener = socket_at(&w.at("root/s"));
    w.file("root/d/x.txt", b"keep\n");
    w.link("root/lp", "p");
    let mut fifos = vec![fifo.clone()];
    let before = snapshot(&w.dir);

    for p in ["p", "s", "d", "lp"] {
        assert_refused(
            "write_atomic",
            write_t(&w.root(), p, b"PWNED", &fifos),
            p,
            NOT_A_FILE,
            &w,
        );
        assert_eq!(snapshot(&w.dir), before, "after write_atomic({p})");
    }
    let meta = |p: &str| fs::symlink_metadata(w.at(p)).unwrap().file_type();
    assert!(meta("root/p").is_fifo());
    assert!(meta("root/s").is_socket());
    assert!(meta("root/d").is_dir());
    assert_eq!(fs::read(w.at("root/d/x.txt")).unwrap(), b"keep\n");
    assert_no_temp(&w.dir);

    // A regular file replaced by a FIFO between open and save.
    w.file("root/f.txt", b"regular\n");
    let fs_ = w.fs();
    let mut doc = Document::open(&fs_, &rp("f.txt")).unwrap();
    doc.buffer_mut().insert(0, "EDIT ").unwrap();
    fs::remove_file(w.at("root/f.txt")).unwrap();
    mkfifo(&w.at("root/f.txt"));
    fifos.push(w.at("root/f.txt"));
    let root = w.root();
    let (doc, saved) = timed("Document::save(f.txt)", &fifos, move || {
        let mut doc = doc;
        let r = doc.save(&mut DiskFs::new(&root).unwrap());
        (doc, r)
    });
    let e = saved.unwrap_err();
    assert_eq!(save_message(&e), "cannot save f.txt: not a regular file");
    assert_eq!(e.path().as_str(), "f.txt");
    assert!(meta("root/f.txt").is_fifo());
    assert!(doc.is_dirty());
    assert_no_temp(&w.dir);
}

// ---------------------------------------------------------------------------------------------
// AC-8

#[test]
fn outside_root_wins_over_not_found() {
    let w = Work::new("prec");
    w.file("root/real/keep.txt", b"keep\n");
    mkfifo(&w.at("outside/fifo"));
    w.link("root/gone", "../outside/none.txt");
    w.link("root/ghost2", "real/none.txt");
    w.link("root/up", "..");
    w.link("root/self", ".");
    w.link("root/po", "../outside/fifo");
    w.link("root/dn", "/dev/null");
    w.link("root/dlink", "../outside");
    let fifos = vec![w.at("outside/fifo")];
    // Preconditions: what each link reaches.
    assert!(!w.at("outside/none.txt").exists());
    assert!(!w.at("root/real/none.txt").exists());
    assert_eq!(fs::canonicalize(w.at("root/up")).unwrap(), w.dir);
    assert_eq!(fs::canonicalize(w.at("root/self")).unwrap(), w.root());
    assert!(fs::metadata(w.at("root/po")).unwrap().file_type().is_fifo());
    assert!(
        fs::metadata(w.at("root/dn"))
            .unwrap()
            .file_type()
            .is_char_device()
    );
    assert_eq!(
        fs::canonicalize(w.at("root/dlink")).unwrap(),
        w.at("outside")
    );
    let before = snapshot(&w.dir);
    let root = w.root();

    // Dangling outside: outside_root, not not_found, for read and write.
    assert_refused("read", read_t(&root, "gone", &fifos), "gone", OUTSIDE, &w);
    assert_refused(
        "write_atomic",
        write_t(&root, "gone", b"PWNED", &fifos),
        "gone",
        OUTSIDE,
        &w,
    );
    assert!(!w.at("outside/none.txt").exists());
    // Resolution left the root and failed further out: outside_root.
    for p in ["dlink/nodir/x.txt", "dlink/secret.txt/x"] {
        assert_refused("read", read_t(&root, p, &fifos), p, OUTSIDE, &w);
        assert_refused(
            "write_atomic",
            write_t(&root, p, b"PWNED", &fifos),
            p,
            OUTSIDE,
            &w,
        );
    }
    // Dangling inside: not found.
    assert_refused(
        "read",
        read_t(&root, "ghost2", &fifos),
        "ghost2",
        "not found",
        &w,
    );
    // Missing components inside the root: not found, as today.
    for p in ["nope/x.txt", "real/keep.txt/x"] {
        assert_refused("read", read_t(&root, p, &fifos), p, "not found", &w);
    }
    // Outside wins over not-a-file.
    assert_refused("read", read_t(&root, "up", &fifos), "up", OUTSIDE, &w);
    assert!(!is_file_t(&root, "up", &fifos));
    // The root itself is a directory inside.
    assert_refused(
        "read",
        read_t(&root, "self", &fifos),
        "self",
        NOT_A_FILE,
        &w,
    );
    // An outside FIFO is refused as outside, without blocking.
    assert_refused("read", read_t(&root, "po", &fifos), "po", OUTSIDE, &w);
    assert!(!is_file_t(&root, "po", &fifos));
    // An outside device.
    assert_refused("read", read_t(&root, "dn", &fifos), "dn", OUTSIDE, &w);
    assert!(!is_file_t(&root, "dn", &fifos));
    // The lexical refusal comes first, before any VFS call.
    assert!(RelPath::new("../outside/secret.txt").is_err());

    assert_eq!(snapshot(&w.dir), before);
    assert_eq!(fs::read(w.at("outside/secret.txt")).unwrap(), SECRET);
    assert_no_temp(&w.dir);
}

// ---------------------------------------------------------------------------------------------
// Ruling v4 (D-072): the root path, or a parent directory of it, replaced by a symlink between
// two requests. The root is the directory resolved at construction.

/// A swap case: the root path given to `DiskFs::new`, the directory the swapped path leads to
/// (holding `secret.txt` and `f.txt`), and the rename + symlink that performs the swap.
struct Swap {
    root: PathBuf,
    target: PathBuf,
    moved: PathBuf,
    aside: PathBuf,
    link_to: &'static str,
}

impl Swap {
    /// `parent == false`: `work/root` becomes a link to `outside`. `parent == true`: the root
    /// is `work/p/root` and `work/p` becomes a link to `evil`, which holds a `root/`.
    fn new(w: &Work, parent: bool) -> Swap {
        let s = if parent {
            Swap {
                root: w.at("p/root"),
                target: w.at("evil/root"),
                moved: w.at("p"),
                aside: w.at("p.real"),
                link_to: "evil",
            }
        } else {
            Swap {
                root: w.at("root"),
                target: w.at("outside"),
                moved: w.at("root"),
                aside: w.at("root.real"),
                link_to: "outside",
            }
        };
        fs::create_dir_all(&s.root).unwrap();
        fs::write(s.root.join("f.txt"), b"f original\n").unwrap();
        fs::create_dir_all(&s.target).unwrap();
        fs::write(s.target.join("secret.txt"), SECRET).unwrap();
        fs::write(s.target.join("f.txt"), b"outside f\n").unwrap();
        s
    }

    /// Replace the moved directory by a symlink; then show the outside files are reachable
    /// through the root path with ordinary file-system calls.
    fn swap(&self) {
        fs::rename(&self.moved, &self.aside).unwrap();
        symlink(self.link_to, &self.moved).unwrap();
        assert_eq!(fs::read(self.root.join("secret.txt")).unwrap(), SECRET);
        assert_eq!(fs::read(self.root.join("f.txt")).unwrap(), b"outside f\n");
    }
}

fn swapped_root_is_refused(name: &str, parent: bool) {
    let w = Work::new(name);
    let s = Swap::new(&w, parent);
    let mut fs_ = DiskFs::new(&s.root).unwrap();
    let mut doc = Document::open(&fs_, &rp("f.txt")).unwrap();
    doc.buffer_mut().insert(0, "EDIT ").unwrap();

    s.swap();
    // The whole work directory (the moved-aside original root included), links not followed.
    let before = snapshot(&w.dir);

    assert_refused(
        "read",
        fs_.read(&rp("secret.txt")),
        "secret.txt",
        OUTSIDE,
        &w,
    );
    assert!(!fs_.is_file(&rp("secret.txt")));
    match Document::open(&fs_, &rp("secret.txt")) {
        Ok(_) => panic!("Document::open(secret.txt) succeeded after the swap"),
        Err(e) => assert_eq!(
            e.to_string(),
            "cannot open secret.txt: resolves outside the root"
        ),
    }
    assert_refused(
        "write_atomic",
        fs_.write_atomic(&rp("secret.txt"), b"PWNED"),
        "secret.txt",
        OUTSIDE,
        &w,
    );
    match doc.save(&mut fs_) {
        Ok(_) => panic!("save of f.txt (opened before the swap) succeeded after the swap"),
        Err(e) => {
            assert_eq!(
                save_message(&e),
                "cannot save f.txt: resolves outside the root"
            );
            assert_eq!(e.path().as_str(), "f.txt");
        }
    }
    assert!(doc.is_dirty());
    assert_eq!(snapshot(&w.dir), before);
    assert_eq!(fs::read(s.target.join("secret.txt")).unwrap(), SECRET);
    assert_eq!(fs::read(s.target.join("f.txt")).unwrap(), b"outside f\n");
    assert_no_temp(&w.dir);
}

#[test]
fn root_swapped_between_requests_is_refused() {
    swapped_root_is_refused("rootswap", false);
}

#[test]
fn parent_swapped_between_requests_is_refused() {
    swapped_root_is_refused("parentswap", true);
}
