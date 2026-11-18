//! T-073 Interface (4) through the Rust API of `kiln-workspace`: the root and the save on
//! Windows, with the same tests asserting the unchanged behaviour on Linux and macOS. Written by
//! the test author from `tasks/T-073-windows-build-and-root-identity.md` alone (AC-W2: the
//! reported Windows step runs this file).
//!
//! Ruling Q-2 (D-245): no platform cfg in this file. Every test runs on every platform; where
//! the expected behaviour differs, the expectation is chosen at run time with `cfg!(windows)`,
//! and both branches assert. Only `std::fs` calls that exist on every platform are used.
//!
//! Interface (1) leaves one thing to be measured on the runner: whether a held root (or a
//! directory above it) can be renamed by the process. The tests accept either outcome there,
//! as the spec says, but check each: a refused rename is first shown not to be a general
//! refusal (a directory that is not held is renamed in the same place), and then the session
//! must still work on its root; an accepted one must make every call `OutsideRoot`. The outcome
//! is printed as one line `MEASURED: <test>: <platform>: <outcome>` on the test's (captured)
//! stdout: shown for a failing test, and for a passing one when the run is given
//! `-- --show-output` (then in the "successes" block, after the results, so it never splits a
//! `test <name> ... ok` line).
//!
//! Every failure message names the platform, the step, what was expected and what was seen
//! (for an OS error: its kind and raw OS error). No message text and no OS error number is
//! asserted. Each fixture is a fresh directory below `CARGO_TARGET_TMPDIR`, removed on drop.
#![cfg(test)]

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use kiln_workspace::{DirListing, DiskFs, Document, EntryKind, RelPath, Vfs, VfsError};

const OS: &str = std::env::consts::OS;
const F_ORIGINAL: &[u8] = b"f original\n";
const OTHER_F: &[u8] = b"other f\n";
const EDIT: &str = "EDIT ";
const F_EDITED: &[u8] = b"EDIT f original\n";
const NEW: &[u8] = b"new content\n";
const DECOY: &[u8] = b"not mine\n";
const TEMP_PREFIX: &str = ".kiln-tmp-";

// ---------------------------------------------------------------------------------------------
// Fixtures and helpers.

/// A fresh directory for one test; removed on drop (read-only files made writable first, as
/// Windows does not remove a read-only file).
struct Work {
    dir: PathBuf,
    /// The permissions of a freshly created file: used to make a read-only file writable again
    /// without a platform API.
    writable: fs::Permissions,
}

impl Work {
    fn new(name: &str) -> Work {
        let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join("acceptance_t073");
        fs::create_dir_all(&base)
            .unwrap_or_else(|e| panic!("[{OS}] fixture: create {}: {}", base.display(), os(&e)));
        let probe = base.join(format!(".probe-{name}-{}", std::process::id()));
        fs::write(&probe, b"").unwrap();
        let writable = fs::metadata(&probe).unwrap().permissions();
        fs::remove_file(&probe).unwrap();
        let dir = base.join(format!("{name}-{}", std::process::id()));
        if dir.exists() {
            make_writable(&dir, &writable);
            fs::remove_dir_all(&dir).unwrap_or_else(|e| {
                panic!("[{OS}] fixture: remove old {}: {}", dir.display(), os(&e))
            });
        }
        fs::create_dir_all(&dir)
            .unwrap_or_else(|e| panic!("[{OS}] fixture: create {}: {}", dir.display(), os(&e)));
        Work { dir, writable }
    }

    fn at(&self, rel: &str) -> PathBuf {
        self.dir.join(rel)
    }

    fn file(&self, rel: &str, bytes: &[u8]) {
        let p = self.at(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, bytes).unwrap_or_else(|e| panic!("[{OS}] fixture: write {rel}: {}", os(&e)));
    }

    fn dir(&self, rel: &str) {
        fs::create_dir_all(self.at(rel))
            .unwrap_or_else(|e| panic!("[{OS}] fixture: create dir {rel}: {}", os(&e)));
    }
}

impl Drop for Work {
    fn drop(&mut self) {
        make_writable(&self.dir, &self.writable);
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Make every read-only file below `dir` writable (best effort).
fn make_writable(dir: &Path, writable: &fs::Permissions) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        let Ok(meta) = fs::symlink_metadata(&p) else {
            continue;
        };
        if meta.is_dir() {
            make_writable(&p, writable);
        } else if meta.permissions().readonly() {
            let _ = fs::set_permissions(&p, writable.clone());
        }
    }
}

/// An OS error with its kind and raw OS error, for failure messages and measurements.
fn os(e: &io::Error) -> String {
    format!(
        "kind {:?}, raw OS error {:?} ({e})",
        e.kind(),
        e.raw_os_error()
    )
}

/// One measurement line on the test's captured stdout (module docs).
fn measured(test: &str, outcome: &str) {
    println!("MEASURED: {test}: {OS}: {outcome}");
}

fn rp(s: &str) -> RelPath {
    RelPath::new(s).unwrap_or_else(|e| panic!("[{OS}] RelPath::new({s:?}) rejected: {e:?}"))
}

fn new_disk(root: &Path, ctx: &str) -> DiskFs {
    DiskFs::new(root).unwrap_or_else(|e| {
        panic!(
            "[{OS}] {ctx}: DiskFs::new({}) of an existing, readable directory: expected Ok, got {e:?}",
            root.display()
        )
    })
}

/// Every entry under `dir`, recursively: `dir`, or `file <bytes> ro=<read-only>`.
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
            let meta = fs::symlink_metadata(&p).unwrap();
            let desc = if meta.is_dir() {
                walk(base, &p, out);
                "dir".to_owned()
            } else if meta.is_file() {
                format!(
                    "file {} ro={}",
                    fs::read(&p).unwrap().escape_ascii(),
                    meta.permissions().readonly()
                )
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

/// The names directly in `dir`, sorted.
fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("[{OS}] read_dir {}: {}", dir.display(), os(&e)))
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

/// No entry named `.kiln-tmp-*` anywhere under `dir`, except those in `allowed`.
fn assert_no_temp(dir: &Path, allowed: &[&str], ctx: &str) {
    for rel in snapshot(dir).keys() {
        let last = rel.rsplit('/').next().unwrap_or(rel);
        assert!(
            !last.starts_with(TEMP_PREFIX) || allowed.contains(&rel.as_str()),
            "[{OS}] {ctx}: expected no {TEMP_PREFIX}* left, found {rel}"
        );
    }
}

/// `r` is `Err(OutsideRoot)` naming `path`.
fn assert_outside<T: Debug>(r: Result<T, VfsError>, path: &str, ctx: &str) {
    match r {
        Err(VfsError::OutsideRoot { path: p }) if p.as_str() == path => {}
        other => {
            panic!("[{OS}] {ctx}: expected Err(OutsideRoot {{ path: {path} }}), got {other:?}")
        }
    }
}

fn listing_names(l: &DirListing) -> Vec<(String, EntryKind)> {
    l.entries.iter().map(|e| (e.name.clone(), e.kind)).collect()
}

/// Whether the read-only attribute stops this process from opening a file for writing: false
/// on Unix as uid 0. Shown on a probe file in `w`.
fn read_only_stops_writes(w: &Work) -> bool {
    let probe = w.at(".ro-probe");
    fs::write(&probe, b"p").unwrap();
    let mut perms = fs::metadata(&probe).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&probe, perms).unwrap();
    let stops = fs::OpenOptions::new().write(true).open(&probe).is_err();
    fs::set_permissions(&probe, w.writable.clone()).unwrap();
    fs::remove_file(&probe).unwrap();
    stops
}

// ---------------------------------------------------------------------------------------------
// Interface (1): a root that exists is accepted, held and used.

/// `DiskFs::new` accepts an existing directory; reads, `is_file`, listing and a save work below
/// it. Rejects: a refused root, a read that does not reach the root's files, a listing that
/// does not show them.
#[test]
fn acw2_existing_root_is_accepted_and_used() {
    let w = Work::new("accepted");
    w.file("root/f.txt", F_ORIGINAL);
    w.file("root/sub/g.txt", OTHER_F);
    let mut disk = new_disk(&w.at("root"), "accepted");

    assert_eq!(
        disk.read(&rp("f.txt")),
        Ok(F_ORIGINAL.to_vec()),
        "[{OS}] accepted: read(f.txt)"
    );
    assert_eq!(
        disk.read(&rp("sub/g.txt")),
        Ok(OTHER_F.to_vec()),
        "[{OS}] accepted: read(sub/g.txt)"
    );
    assert!(
        disk.is_file(&rp("f.txt")),
        "[{OS}] accepted: is_file(f.txt) is false"
    );
    assert!(
        !disk.is_file(&rp("sub")),
        "[{OS}] accepted: is_file(sub) is true"
    );
    let listing = disk
        .list_dir(None)
        .unwrap_or_else(|e| panic!("[{OS}] accepted: list_dir(None): expected Ok, got {e:?}"));
    assert_eq!(
        listing_names(&listing),
        vec![
            ("f.txt".to_owned(), EntryKind::File),
            ("sub".to_owned(), EntryKind::Dir)
        ],
        "[{OS}] accepted: list_dir(None)"
    );
    assert_eq!(
        listing.skipped, 0,
        "[{OS}] accepted: list_dir(None).skipped"
    );

    let mut doc = Document::open(&disk, &rp("f.txt"))
        .unwrap_or_else(|e| panic!("[{OS}] accepted: Document::open(f.txt): {e:?}"));
    doc.buffer_mut().insert(0, EDIT).unwrap();
    doc.save(&mut disk)
        .unwrap_or_else(|e| panic!("[{OS}] accepted: save of f.txt: expected Ok, got {e:?}"));
    assert!(!doc.is_dirty(), "[{OS}] accepted: still dirty after save");
    assert_eq!(
        fs::read(w.at("root/f.txt")).unwrap(),
        F_EDITED,
        "[{OS}] accepted: f.txt on disk"
    );
    assert_no_temp(&w.dir, &[], "accepted: after save");
}

/// A root that does not exist is refused as now: `NotFound` naming `(root)`. Rejects `Ok`, `Io`.
#[test]
fn acw2_missing_root_is_refused() {
    let w = Work::new("missing");
    match DiskFs::new(&w.at("none")) {
        Err(VfsError::NotFound { path }) if path.as_str() == "(root)" => {}
        other => panic!("[{OS}] missing root: expected Err(NotFound {{ (root) }}), got {other:?}"),
    }
    assert_eq!(
        names(&w.dir),
        Vec::<String>::new(),
        "[{OS}] missing root: created something"
    );
}

/// A root that is a file is refused as now: `Io { kind: NotADirectory }` naming `(root)`.
/// Rejects `Ok`, `NotFound`, another kind.
#[test]
fn acw2_file_as_root_is_refused() {
    let w = Work::new("fileroot");
    w.file("root", F_ORIGINAL);
    match DiskFs::new(&w.at("root")) {
        Err(VfsError::Io {
            path,
            kind: ErrorKind::NotADirectory,
            ..
        }) if path.as_str() == "(root)" => {}
        other => panic!(
            "[{OS}] file as root: expected Err(Io {{ (root), kind: NotADirectory }}), got {other:?}"
        ),
    }
    assert_eq!(
        fs::read(w.at("root")).unwrap(),
        F_ORIGINAL,
        "[{OS}] file as root: changed"
    );
}

// ---------------------------------------------------------------------------------------------
// Interface (1): the root's identity. The root path is replaced by another directory at the
// same path; or, if the platform refuses to rename a held root, that refusal is recorded.

/// Which directory is renamed away to replace the root path.
#[derive(Clone, Copy, Debug)]
enum Away {
    /// `root` itself: `root` → `root.away`, a new `root` made.
    Root,
    /// The root's parent: root at `p/root`; `p` → `p.away`, a new `p/root` made.
    Parent,
}

fn replaced_root_case(test: &str, away: Away) {
    let w = Work::new(test);
    let (root_rel, from, to, original) = match away {
        Away::Root => ("root", "root", "root.away", "root.away/f.txt"),
        Away::Parent => ("p/root", "p", "p.away", "p.away/root/f.txt"),
    };
    w.file(&format!("{root_rel}/f.txt"), F_ORIGINAL);
    // Control, in the same directory: a directory that no `DiskFs` holds can be renamed here,
    // so a refusal below comes from the hold, not from the place.
    w.file(&format!("ctl-{from}/f.txt"), F_ORIGINAL);
    if let Err(e) = fs::rename(w.at(&format!("ctl-{from}")), w.at(&format!("ctl-{to}"))) {
        panic!(
            "[{OS}] {test}: control: rename of a directory nobody holds failed: {}",
            os(&e)
        );
    }
    let root = w.at(root_rel);
    let mut disk = new_disk(&root, test);
    let mut doc = Document::open(&disk, &rp("f.txt"))
        .unwrap_or_else(|e| panic!("[{OS}] {test}: Document::open(f.txt): {e:?}"));
    doc.buffer_mut().insert(0, EDIT).unwrap();

    match fs::rename(w.at(from), w.at(to)) {
        Ok(()) => {
            measured(
                test,
                &format!("rename of {from} while the root is held: ok"),
            );
            // Another directory at the same path, with a file of the same name.
            w.file(&format!("{root_rel}/f.txt"), OTHER_F);
            assert_eq!(
                fs::read(w.at(original)).unwrap(),
                F_ORIGINAL,
                "[{OS}] {test}: fixture"
            );
            let before = snapshot(&w.dir);
            let step = format!("{test}: after {from} renamed to {to} and a new {root_rel} made");
            assert_outside(
                disk.read(&rp("f.txt")),
                "f.txt",
                &format!("{step}: read(f.txt)"),
            );
            assert!(
                !disk.is_file(&rp("f.txt")),
                "[{OS}] {step}: is_file(f.txt) is true"
            );
            assert_outside(
                disk.write_atomic(&rp("new.txt"), b"PWNED"),
                "new.txt",
                &format!("{step}: write_atomic(new.txt)"),
            );
            assert_outside(
                disk.list_dir(None),
                "(root)",
                &format!("{step}: list_dir(None)"),
            );
            assert_outside(
                disk.check_location(&rp("f.txt")),
                "f.txt",
                &format!("{step}: check_location(f.txt)"),
            );
            assert_outside(
                doc.save(&mut disk),
                "f.txt",
                &format!("{step}: save of f.txt opened before"),
            );
            assert!(
                doc.is_dirty(),
                "[{OS}] {step}: the document is no longer dirty"
            );
            assert_eq!(
                snapshot(&w.dir),
                before,
                "[{OS}] {step}: the work directory changed"
            );
            assert_no_temp(&w.dir, &[], &step);

            // Back: the new directory removed, the root moved back; the session works again.
            fs::remove_dir_all(w.at(from))
                .unwrap_or_else(|e| panic!("[{OS}] {test}: remove the new {from}: {}", os(&e)));
            fs::rename(w.at(to), w.at(from)).unwrap_or_else(|e| {
                panic!("[{OS}] {test}: rename {to} back to {from}: {}", os(&e))
            });
            let step = format!("{test}: after {to} renamed back to {from}");
            doc.save(&mut disk)
                .unwrap_or_else(|e| panic!("[{OS}] {step}: save: expected Ok, got {e:?}"));
            assert!(!doc.is_dirty(), "[{OS}] {step}: still dirty");
            assert_eq!(
                fs::read(root.join("f.txt")).unwrap(),
                F_EDITED,
                "[{OS}] {step}: f.txt"
            );
            assert_no_temp(&w.dir, &[], &step);
        }
        Err(e) if cfg!(windows) => {
            // The spec's other outcome (AC-W2): Windows refuses the replacement while the
            // root is held. Recorded; the session must still work on its root.
            measured(
                test,
                &format!(
                    "rename of {from} while the root is held: refused: {}",
                    os(&e)
                ),
            );
            let step = format!("{test}: after the refused rename of {from}");
            assert_eq!(
                fs::read(root.join("f.txt")).ok().as_deref(),
                Some(F_ORIGINAL),
                "[{OS}] {step}: the root's file is not in place"
            );
            assert_eq!(
                disk.read(&rp("f.txt")),
                Ok(F_ORIGINAL.to_vec()),
                "[{OS}] {step}: read(f.txt)"
            );
            doc.save(&mut disk)
                .unwrap_or_else(|e| panic!("[{OS}] {step}: save: expected Ok, got {e:?}"));
            assert_eq!(
                fs::read(root.join("f.txt")).unwrap(),
                F_EDITED,
                "[{OS}] {step}: f.txt"
            );
            assert_no_temp(&w.dir, &[], &step);
        }
        Err(e) => panic!(
            "[{OS}] {test}: rename of {from} while the root is held: expected Ok on this platform, got {}",
            os(&e)
        ),
    }
}

/// The root renamed away and another directory made at its path: every call is `OutsideRoot`
/// and nothing is written there; moved back, the session works again. On Windows a refused
/// rename is the spec's other outcome, recorded. Rejects: the new directory taken as the root
/// (identity not checked), a call that reads or writes there.
#[test]
fn acw2_root_replaced_at_same_path_is_outside_root() {
    replaced_root_case(
        "acw2_root_replaced_at_same_path_is_outside_root",
        Away::Root,
    );
}

/// As above, with the root's parent renamed away and a new `p/root` made.
#[test]
fn acw2_root_parent_replaced_is_outside_root() {
    replaced_root_case("acw2_root_parent_replaced_is_outside_root", Away::Parent);
}

// ---------------------------------------------------------------------------------------------
// Interface (3): the save.

/// A save replaces the target by a rename: a hard link to the old file keeps the old bytes
/// (the link is split, the target is a new file), the target has the new bytes, and no
/// `.kiln-tmp-*` is left. Rejects: the target written in place (the link would show the new
/// bytes), the temporary file left behind, the target not replaced.
#[test]
fn acw2_save_replaces_target_by_rename_and_leaves_no_temp() {
    let w = Work::new("replace");
    w.file("root/f.txt", F_ORIGINAL);
    fs::hard_link(w.at("root/f.txt"), w.at("root/alias.txt"))
        .unwrap_or_else(|e| panic!("[{OS}] replace: fixture: hard link: {}", os(&e)));
    let mut disk = new_disk(&w.at("root"), "replace");

    disk.write_atomic(&rp("f.txt"), NEW)
        .unwrap_or_else(|e| panic!("[{OS}] replace: write_atomic(f.txt): expected Ok, got {e:?}"));
    assert_eq!(
        fs::read(w.at("root/f.txt")).unwrap(),
        NEW,
        "[{OS}] replace: f.txt"
    );
    assert_eq!(
        fs::read(w.at("root/alias.txt")).unwrap(),
        F_ORIGINAL,
        "[{OS}] replace: a hard link to the old f.txt changed: the target was written in place, not replaced by a rename"
    );
    assert_eq!(
        names(&w.at("root")),
        ["alias.txt", "f.txt"],
        "[{OS}] replace: root entries"
    );
    assert_eq!(
        disk.read(&rp("f.txt")),
        Ok(NEW.to_vec()),
        "[{OS}] replace: read(f.txt)"
    );

    // Through a document, a second time.
    let mut doc = Document::open(&disk, &rp("f.txt")).unwrap();
    doc.buffer_mut().insert(0, EDIT).unwrap();
    doc.save(&mut disk)
        .unwrap_or_else(|e| panic!("[{OS}] replace: second save: expected Ok, got {e:?}"));
    assert_eq!(
        fs::read(w.at("root/f.txt")).unwrap(),
        b"EDIT new content\n",
        "[{OS}] replace: f.txt"
    );
    assert_eq!(
        names(&w.at("root")),
        ["alias.txt", "f.txt"],
        "[{OS}] replace: root entries after second save"
    );
}

/// A save of a new file creates it with the bytes and leaves no `.kiln-tmp-*`.
#[test]
fn acw2_save_of_new_file_creates_it_and_leaves_no_temp() {
    let w = Work::new("newfile");
    w.dir("root/sub");
    let mut disk = new_disk(&w.at("root"), "newfile");
    disk.write_atomic(&rp("sub/new.txt"), NEW)
        .unwrap_or_else(|e| {
            panic!("[{OS}] newfile: write_atomic(sub/new.txt): expected Ok, got {e:?}")
        });
    assert_eq!(
        fs::read(w.at("root/sub/new.txt")).unwrap(),
        NEW,
        "[{OS}] newfile: content"
    );
    assert_eq!(
        names(&w.at("root/sub")),
        ["new.txt"],
        "[{OS}] newfile: sub entries"
    );
    assert_eq!(
        names(&w.at("root")),
        ["sub"],
        "[{OS}] newfile: root entries"
    );
    assert!(
        !fs::metadata(w.at("root/sub/new.txt"))
            .unwrap()
            .permissions()
            .readonly(),
        "[{OS}] newfile: the new file is read-only"
    );
}

/// The temporary file is created exclusively: files already at `.kiln-tmp-0` and `.kiln-tmp-1`
/// are left byte-identical, the save succeeds, and no other `.kiln-tmp-*` is left. Rejects: a
/// temporary file opened with truncation (a decoy overwritten and renamed away), a save
/// refused because the first names are taken.
#[test]
fn acw2_save_creates_temp_exclusively() {
    let w = Work::new("exclusive");
    w.file("root/f.txt", F_ORIGINAL);
    w.file("root/.kiln-tmp-0", DECOY);
    w.file("root/.kiln-tmp-1", DECOY);
    let mut disk = new_disk(&w.at("root"), "exclusive");
    disk.write_atomic(&rp("f.txt"), NEW).unwrap_or_else(|e| {
        panic!("[{OS}] exclusive: write_atomic(f.txt) beside two decoys: expected Ok, got {e:?}")
    });
    assert_eq!(
        fs::read(w.at("root/f.txt")).unwrap(),
        NEW,
        "[{OS}] exclusive: f.txt"
    );
    for d in [".kiln-tmp-0", ".kiln-tmp-1"] {
        assert_eq!(
            fs::read(w.at(&format!("root/{d}"))).ok().as_deref(),
            Some(DECOY),
            "[{OS}] exclusive: the existing {d} was changed or removed"
        );
    }
    assert_eq!(
        names(&w.at("root")),
        [".kiln-tmp-0", ".kiln-tmp-1", "f.txt"],
        "[{OS}] exclusive: root entries"
    );
}

/// A save keeps the target's permissions: a writable target stays writable (not read-only).
/// Rejects: the temporary file's permissions made read-only, or another file's applied.
#[test]
fn acw2_save_keeps_writable_target_writable() {
    let w = Work::new("keepperm");
    w.file("root/f.txt", F_ORIGINAL);
    let mut disk = new_disk(&w.at("root"), "keepperm");
    disk.write_atomic(&rp("f.txt"), NEW)
        .unwrap_or_else(|e| panic!("[{OS}] keepperm: write_atomic(f.txt): expected Ok, got {e:?}"));
    assert_eq!(
        fs::read(w.at("root/f.txt")).unwrap(),
        NEW,
        "[{OS}] keepperm: f.txt after first save"
    );
    let meta = fs::metadata(w.at("root/f.txt")).unwrap();
    assert!(
        !meta.permissions().readonly(),
        "[{OS}] keepperm: the saved target is read-only"
    );
    // And it can be written again (by the process, then by the next save).
    fs::OpenOptions::new()
        .write(true)
        .open(w.at("root/f.txt"))
        .unwrap_or_else(|e| {
            panic!(
                "[{OS}] keepperm: open the saved target for writing: {}",
                os(&e)
            )
        });
    disk.write_atomic(&rp("f.txt"), F_ORIGINAL)
        .unwrap_or_else(|e| panic!("[{OS}] keepperm: second write_atomic: expected Ok, got {e:?}"));
    assert_eq!(
        fs::read(w.at("root/f.txt")).unwrap(),
        F_ORIGINAL,
        "[{OS}] keepperm: f.txt"
    );
    assert_no_temp(&w.dir, &[], "keepperm");
}

/// A target with the read-only attribute: where that stops the process from writing (Windows
/// always; Unix unless uid 0), the save is refused with `Io { kind: PermissionDenied }` naming
/// the path, nothing in the root changes and the document stays modified. Where it does not
/// (Unix as uid 0), the save completes and the target keeps its read-only permissions (they
/// are copied to the new file). Rejects: a refused target overwritten, replaced or left with a
/// temporary file; another error kind; on the privileged branch the permissions dropped.
#[test]
fn acw2_save_of_read_only_target_is_refused_and_changes_nothing() {
    let w = Work::new("readonly");
    w.file("root/f.txt", F_ORIGINAL);
    w.file("root/other.txt", OTHER_F);
    let stops = read_only_stops_writes(&w);
    assert!(
        stops || !cfg!(windows),
        "[{OS}] readonly: precondition: the read-only attribute did not stop opening a file for writing"
    );
    let mut disk = new_disk(&w.at("root"), "readonly");
    let mut doc = Document::open(&disk, &rp("f.txt")).unwrap();
    doc.buffer_mut().insert(0, EDIT).unwrap();
    let mut perms = fs::metadata(w.at("root/f.txt")).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(w.at("root/f.txt"), perms).unwrap();
    let before = snapshot(&w.dir);

    let direct = disk.write_atomic(&rp("f.txt"), NEW);
    if stops {
        match &direct {
            Err(VfsError::Io {
                path,
                kind: ErrorKind::PermissionDenied,
                ..
            }) if path.as_str() == "f.txt" => {}
            other => panic!(
                "[{OS}] readonly: write_atomic(f.txt) of a read-only target: expected Err(Io {{ f.txt, kind: PermissionDenied }}), got {other:?}"
            ),
        }
        assert_eq!(
            snapshot(&w.dir),
            before,
            "[{OS}] readonly: refused write_atomic changed the work directory"
        );
        let saved = doc.save(&mut disk);
        assert!(
            matches!(
                &saved,
                Err(VfsError::Io {
                    kind: ErrorKind::PermissionDenied,
                    ..
                })
            ),
            "[{OS}] readonly: save of the document: expected Err(Io {{ kind: PermissionDenied }}), got {saved:?}"
        );
        assert!(
            doc.is_dirty(),
            "[{OS}] readonly: the document is no longer dirty"
        );
        assert_eq!(
            snapshot(&w.dir),
            before,
            "[{OS}] readonly: refused save changed the work directory"
        );
        assert_eq!(
            fs::read(w.at("root/f.txt")).unwrap(),
            F_ORIGINAL,
            "[{OS}] readonly: f.txt"
        );
    } else {
        // Unix as uid 0: permission bits do not stop this process.
        assert_eq!(
            direct,
            Ok(()),
            "[{OS}] readonly (privileged): write_atomic(f.txt)"
        );
        assert_eq!(
            fs::read(w.at("root/f.txt")).unwrap(),
            NEW,
            "[{OS}] readonly (privileged): f.txt"
        );
        assert!(
            fs::metadata(w.at("root/f.txt"))
                .unwrap()
                .permissions()
                .readonly(),
            "[{OS}] readonly (privileged): the saved target lost its read-only permissions"
        );
        assert_eq!(
            fs::read(w.at("root/other.txt")).unwrap(),
            OTHER_F,
            "[{OS}] readonly (privileged): other.txt"
        );
        assert_eq!(
            names(&w.at("root")),
            ["f.txt", "other.txt"],
            "[{OS}] readonly (privileged): root entries"
        );
    }
    assert!(
        fs::metadata(w.at("root/f.txt"))
            .unwrap()
            .permissions()
            .readonly(),
        "[{OS}] readonly: f.txt is no longer read-only"
    );
    assert_no_temp(&w.dir, &[], "readonly");
    fs::set_permissions(w.at("root/f.txt"), w.writable.clone()).unwrap();
}
