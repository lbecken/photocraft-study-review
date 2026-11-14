//! T-036 AC-1 through the Rust API of `kiln-workspace`: `DiskFs::new` refuses a root that the
//! calling process cannot open for reading (modes 0311 and 0000), with
//! `VfsError::Io { kind: PermissionDenied }` and the placeholder path `(root)` (Interface 1,
//! Pinned (a)). Written by the test author from `tasks/T-036-root-identity-follow-ups.md` alone.
//!
//! The gate runs as uid 0, where permission bits do not apply, so the assertions run in a child:
//! this test binary re-run with `--exact` as the unprivileged identity U (uid/gid 65534 through
//! `CommandExt::{uid, gid}` when the test's effective uid is 0, otherwise the test's own
//! identity), on a fixture given to U. The child runs a positive control (U opens a readable
//! root and reads a file in it) and a negative control (U cannot read a mode-0000 file it owns)
//! first, and writes a marker file once every assertion has run; the parent asserts that the
//! child passed and that the marker exists and is owned by U. Both identity branches run the
//! whole scenario; nothing is skipped. Every child has a 10 s timeout, which only detects hangs.
//! No message text is asserted: `Display` is only checked to name no part of the work path.
#![cfg(test)]

use std::collections::BTreeMap;
use std::fs;
use std::io::{ErrorKind, Read};
use std::os::unix::fs::{MetadataExt, PermissionsExt, lchown};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use kiln_workspace::{DiskFs, RelPath, Vfs, VfsError};

const SECRET: &[u8] = b"s3cr3t-content\n";
const F_BYTES: &[u8] = b"f readable\n";
/// Hang detector for every child process (AC "Processes and time"); never a duration assertion.
const TIMEOUT: Duration = Duration::from_secs(10);
/// Set in the child run: the work directory.
const CHILD_WORK: &str = "KILN_T036_AC1_CHILD_WORK";
/// Set in the child run: the mode of `work/root`, as four octal digits.
const CHILD_MODE: &str = "KILN_T036_AC1_CHILD_MODE";
/// The unprivileged identity used when the test runs as uid 0 (Definitions, "U").
const NOBODY: u32 = 65534;
const TEST_NAME: &str = "unreadable_root_is_refused_by_new";

// ---------------------------------------------------------------------------------------------
// Identity and fixture.

/// The identity the process under test runs as.
#[derive(Clone, Copy, Debug)]
struct Identity {
    /// True when the test runs as uid 0 and the child is dropped to 65534.
    drop: bool,
    uid: u32,
    gid: u32,
}

impl Identity {
    /// U, found from the owner of a file just created in `dir` (Definitions).
    fn find(dir: &Path) -> Identity {
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

    /// Run `cmd` as U.
    fn apply(&self, cmd: &mut Command) {
        if self.drop {
            cmd.uid(self.uid).gid(self.gid);
        }
    }

    /// Give `path` and everything below it (links not followed) to U.
    fn give(&self, path: &Path) {
        if self.drop {
            lchown(path, Some(self.uid), Some(self.gid)).unwrap();
            let meta = fs::symlink_metadata(path).unwrap();
            if meta.is_dir() {
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

/// A fresh `work/` (mode 0755) under `std::env::temp_dir()`, with `outside/secret.txt`;
/// removed on drop, failed runs included (modes are restored first so the removal can list
/// every directory whatever the identity).
struct Work {
    dir: PathBuf,
}

impl Work {
    fn new(name: &str) -> Work {
        let dir = std::env::temp_dir().join(format!("k36a-{}-{name}", std::process::id()));
        if dir.exists() {
            unlock(&dir);
            fs::remove_dir_all(&dir).unwrap();
        }
        fs::create_dir_all(&dir).unwrap();
        set_mode(&dir, 0o755);
        let dir = fs::canonicalize(&dir).unwrap();
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

fn rp(s: &str) -> RelPath {
    RelPath::new(s).unwrap_or_else(|e| panic!("RelPath::new({s:?}) rejected: {e:?}"))
}

/// `text` names no part of the host work path `work` (neither the path nor its last
/// component).
fn assert_names_no_work_path(text: &str, work: &Path, what: &str) {
    let full = work.to_string_lossy();
    let name = work.file_name().unwrap().to_string_lossy();
    assert!(
        !text.contains(full.as_ref()) && !text.contains(name.as_ref()),
        "{what} names the host work path: {text:?}"
    );
}

/// Wait for `child`, killing it after [`TIMEOUT`] (a hang detector, not a time assertion).
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

/// Run `cmd` with null stdin, collecting stdout and stderr, under the hang detector.
fn run(mut cmd: Command, what: &str) -> (ExitStatus, Vec<u8>, Vec<u8>) {
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("precondition: cannot spawn {what}: {e}"));
    let drain = |r: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut r = r;
            let mut buf = Vec::new();
            let _ = r.read_to_end(&mut buf);
            buf
        })
    };
    let out = drain(Box::new(child.stdout.take().unwrap()));
    let err = drain(Box::new(child.stderr.take().unwrap()));
    let status = wait_or_kill(&mut child, what);
    (status, out.join().unwrap(), err.join().unwrap())
}

// ---------------------------------------------------------------------------------------------
// AC-1, Rust API.

/// The child run, as U: controls first, then the refusal for the current mode, then the marker.
fn child_run(work: &Path, mode: &str) {
    // Positive control: U reaches the fixture and opens a readable root.
    let open = DiskFs::new(&work.join("open")).unwrap_or_else(|e| {
        panic!("precondition (positive control): U cannot open the readable root work/open: {e:?}")
    });
    assert_eq!(
        open.read(&rp("f.txt")).unwrap_or_else(|e| panic!(
            "precondition (positive control): U cannot read work/open/f.txt: {e:?}"
        )),
        F_BYTES,
        "precondition (positive control): work/open/f.txt"
    );
    // Negative control: U has no permission override.
    assert!(
        open.read(&rp("locked.txt")).is_err(),
        "precondition (negative control): U read work/open/locked.txt (mode 0000, owned by U), \
         so it overrides permission checks"
    );

    // The case: a root U cannot open for reading is refused by `new`.
    match DiskFs::new(&work.join("root")) {
        Ok(fs_) => panic!("mode {mode}: DiskFs::new(work/root) accepted the root: {fs_:?}"),
        Err(e) => {
            assert!(
                matches!(
                    e,
                    VfsError::Io {
                        kind: ErrorKind::PermissionDenied,
                        ..
                    }
                ),
                "mode {mode}: expected Io {{ kind: PermissionDenied }}, got {e:?}"
            );
            assert_eq!(e.path().as_str(), "(root)", "mode {mode}: path()");
            assert_names_no_work_path(&e.to_string(), work, &format!("mode {mode}: Display"));
        }
    }
    // `new` of the readable root is still Ok after the refusal.
    assert!(
        DiskFs::new(&work.join("open")).is_ok(),
        "mode {mode}: DiskFs::new(work/open) is not Ok"
    );
    // Every assertion above ran.
    fs::write(work.join(format!("mark-{mode}")), mode).unwrap();
}

fn parent_run() {
    let w = Work::new("new");
    let id = Identity::find(&w.dir);
    w.file("open/f.txt", F_BYTES, 0o644);
    w.file("open/locked.txt", F_BYTES, 0o000);
    w.file("root/f.txt", F_BYTES, 0o644);
    id.give(&w.dir);
    set_mode(&w.at("open"), 0o711);
    set_mode(&w.at("root"), 0o755);
    // The protected files exist and hold their bytes.
    assert_eq!(fs::read(w.at("root/f.txt")).unwrap(), F_BYTES);
    let before = (snapshot(&w.at("root")), snapshot(&w.at("open")));
    let exe = std::env::current_exe().unwrap();

    // Both modes run before anything is asserted, so a failure reports every mode.
    let mut failures = Vec::new();
    for mode in [0o311, 0o000] {
        let text = format!("{mode:04o}");
        set_mode(&w.at("root"), mode);
        let mut cmd = Command::new(&exe);
        cmd.args([TEST_NAME, "--exact", "--nocapture", "--test-threads=1"])
            .env(CHILD_WORK, &w.dir)
            .env(CHILD_MODE, &text)
            .current_dir(&w.dir);
        id.apply(&mut cmd);
        let (status, out, err) = run(cmd, &format!("the child run for mode {text}"));
        set_mode(&w.at("root"), 0o755);
        let mark = w.at(&format!("mark-{text}"));
        let mark_owner = fs::metadata(&mark).map(|m| m.uid()).ok();
        let mark_bytes = fs::read(&mark).ok();
        if !status.success() {
            failures.push(format!(
                "mode {text}: the child run as uid {} failed ({status}).\n--- child stdout\n{}\n\
                 --- child stderr\n{}",
                id.uid,
                String::from_utf8_lossy(&out),
                String::from_utf8_lossy(&err)
            ));
        } else if mark_bytes.as_deref() != Some(text.as_bytes()) {
            failures.push(format!(
                "mode {text}: the child run passed without running its assertions (marker {mark_bytes:?})"
            ));
        } else if mark_owner != Some(id.uid) {
            failures.push(format!(
                "mode {text}: the child did not run as U (marker owner {mark_owner:?}, U is uid {})",
                id.uid
            ));
        }
        // Nothing is created or changed by the refused construction.
        if (snapshot(&w.at("root")), snapshot(&w.at("open"))) != before {
            failures.push(format!("mode {text}: the fixture changed"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn unreadable_root_is_refused_by_new() {
    match (std::env::var(CHILD_WORK), std::env::var(CHILD_MODE)) {
        (Ok(work), Ok(mode)) => child_run(Path::new(&work), &mode),
        _ => parent_run(),
    }
}
