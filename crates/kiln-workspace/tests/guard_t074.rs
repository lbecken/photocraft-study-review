//! T-074 Interface (6) through the Rust API of `kiln-workspace` (`DiskFs`, `Vfs`, `Document`,
//! `project::tree`): links and junctions on the walk, a link at a temporary name, and the root
//! cases, on Windows; the same tests assert today's behaviour on Linux and macOS. Written by the
//! test author from `tasks/T-074-windows-confinement-rules.md` alone.
//!
//! Guards: on Linux and macOS they assert behaviour the spec keeps unchanged, so they pass on
//! base there. On Windows they are the runtime evidence of AC-W1 to AC-W4 (T-072's reported
//! step runs this file with `-- --show-output`); their fail-on-base evidence there is the probe
//! record `tasks/ci-runs/T-074-probe-1.md` (PROBE C1, C2: a relative file link inside the root
//! is followed at base; H2: a save beside a junction and a file link at `.kiln-tmp-0`/`-1`
//! fails at base).
//!
//! No platform cfg in this file (T-073 ruling Q-2, D-245; Interface (6)). Every test runs on
//! every platform; what differs is chosen at run time with `cfg!(windows)`, and both branches
//! assert. How links are made, without a platform cfg:
//! - a file symbolic link, everywhere: `std::fs::soft_link` (deprecated, the one link call
//!   std has on every platform: on Unix `symlink`, on Windows the call behind
//!   `std::os::windows::fs::symlink_file`, which works on the runner, PROBE A1, A5, A6;
//!   INFERRED from std, not run here on Windows);
//! - a directory symbolic link: on Unix the same call; on Windows `cmd /C mklink /D` (unprobed:
//!   the first CI round shows it);
//! - a junction: on Windows `cmd /C mklink /J` (PROBE A3, A4); Unix has none, so there the case
//!   uses a symbolic link to the directory's absolute path, and asserts Unix behaviour.
//!
//! Every spawned process has a timeout (60 s; D-028 (8)). A link the runner cannot make fails
//! the test naming the command and its output, never passes (METHODOLOGY §8.1). After every
//! step that is not expected to change anything the whole fixture is compared with its
//! snapshot (links by their targets), so a refused call is shown to have written nothing. Root
//! cases print `MEASURED: <case>: <platform>: <observed>` on the test's captured stdout (shown
//! for a passing test with `-- --show-output`), as `guard_t073.rs` does; the link cases do too.
//! Every failure names the platform. No message text and no OS error number is asserted.
//! Fixtures are fresh directories below `CARGO_TARGET_TMPDIR`, removed on drop.
#![cfg(test)]

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use kiln_workspace::project;
use kiln_workspace::{DiskFs, Document, EntryKind, OpenError, RelPath, Vfs, VfsError};

const OS: &str = std::env::consts::OS;
const IN_T: &[u8] = b"inside t\n";
const IN_X: &[u8] = b"inside x\n";
const IN_Y: &[u8] = b"inside y\n";
const OUT_T: &[u8] = b"outside t\n";
const OUT_X: &[u8] = b"outside x\n";
const PLAIN: &[u8] = b"plain f\n";
const NEW: &[u8] = b"new content\n";
const EDIT: &str = "EDIT ";
const REAL: &[u8] = b"real\n";
const OTHER: &[u8] = b"other\n";
const TEMP_PREFIX: &str = ".kiln-tmp-";
const SPAWN_TIMEOUT: Duration = Duration::from_secs(60);

// ---------------------------------------------------------------------------------------------
// Fixtures and helpers.

/// A fresh directory for one test; removed on drop.
struct Work {
    dir: PathBuf,
}

impl Work {
    fn new(name: &str) -> Work {
        let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join("guard_t074");
        let dir = base.join(format!("{name}-{}", std::process::id()));
        if fs::symlink_metadata(&dir).is_ok() {
            fs::remove_dir_all(&dir).unwrap_or_else(|e| {
                panic!("[{OS}] fixture: remove old {}: {}", dir.display(), os(&e))
            });
        }
        fs::create_dir_all(&dir)
            .unwrap_or_else(|e| panic!("[{OS}] fixture: create {}: {}", dir.display(), os(&e)));
        Work { dir }
    }

    /// `rel` (`/`-separated) below the fixture.
    fn at(&self, rel: &str) -> PathBuf {
        rel.split('/').fold(self.dir.clone(), |p, s| p.join(s))
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
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn os(e: &io::Error) -> String {
    format!(
        "kind {:?}, raw OS error {:?} ({e})",
        e.kind(),
        e.raw_os_error()
    )
}

fn measured(case: &str, outcome: &str) {
    println!("MEASURED: {case}: {OS}: {outcome}");
}

fn rp(s: &str) -> RelPath {
    RelPath::new(s).unwrap_or_else(|e| panic!("[{OS}] RelPath::new({s:?}) rejected: {e:?}"))
}

/// The kinds of link the runner can make (Interface (6)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// A file symbolic link with a relative target.
    FileRel,
    /// A file symbolic link with an absolute target.
    FileAbs,
    /// A directory symbolic link (relative target).
    DirSym,
    /// A junction (absolute target; on Unix a symbolic link to the absolute path).
    Junction,
}

impl Kind {
    fn is_dir_link(self) -> bool {
        matches!(self, Kind::DirSym | Kind::Junction)
    }

    fn absolute(self) -> bool {
        matches!(self, Kind::FileAbs | Kind::Junction)
    }
}

/// Run `cmd` (a link-making command) with a timeout; `Err` names what failed and its output.
fn run(mut cmd: Command, what: &str) -> Result<(), String> {
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{what}: spawn failed: {}", os(&e)))?;
    let deadline = Instant::now() + SPAWN_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("{what}: no exit within {SPAWN_TIMEOUT:?}; killed"));
            }
            Err(e) => return Err(format!("{what}: wait failed: {}", os(&e))),
        }
    }
    let out = child
        .wait_with_output()
        .map_err(|e| format!("{what}: output: {}", os(&e)))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{what}: {} stdout {:?} stderr {:?}",
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ))
    }
}

/// A symbolic link at `link` to `target` with the one call std has on every platform (module
/// docs): a symbolic link on Unix, a file symbolic link on Windows.
#[allow(deprecated)]
fn soft_link(target: &Path, link: &Path) -> Result<(), String> {
    fs::soft_link(target, link).map_err(|e| format!("std::fs::soft_link: {}", os(&e)))
}

/// `cmd /C mklink <flag> <link> <target>` (Windows only; module docs).
fn mklink(flag: &str, target: &Path, link: &Path) -> Result<(), String> {
    let mut cmd = Command::new("cmd");
    cmd.arg("/C").arg("mklink").arg(flag).arg(link).arg(target);
    run(cmd, &format!("cmd /C mklink {flag}"))
}

/// The relative path from directory `from` to `to` (both `/`-separated, below the fixture).
fn relative(from: &str, to: &str) -> PathBuf {
    let from: Vec<&str> = from.split('/').filter(|s| !s.is_empty()).collect();
    let to: Vec<&str> = to.split('/').collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut p = PathBuf::new();
    for _ in common..from.len() {
        p.push("..");
    }
    for s in &to[common..] {
        p.push(s);
    }
    p
}

/// Make a link of `kind` at `link` (fixture-relative) to `target` (fixture-relative; it need
/// not exist), and check it is a link. Panics naming why if the runner cannot make it.
fn plant(w: &Work, kind: Kind, link: &str, target: &str) -> PathBuf {
    let link_path = w.at(link);
    let parent = link.rsplit_once('/').map_or("", |(p, _)| p);
    let target_path = if kind.absolute() {
        w.at(target)
    } else {
        relative(parent, target)
    };
    let made = match kind {
        Kind::FileRel | Kind::FileAbs => soft_link(&target_path, &link_path),
        Kind::DirSym if cfg!(windows) => mklink("/D", &target_path, &link_path),
        Kind::Junction if cfg!(windows) => mklink("/J", &target_path, &link_path),
        Kind::DirSym | Kind::Junction => soft_link(&target_path, &link_path),
    };
    if let Err(e) = made {
        panic!(
            "[{OS}] fixture: cannot make a {kind:?} at {link} -> {}: {e}; the runner cannot set up \
             this case, so the test fails (METHODOLOGY §8.1)",
            target_path.display()
        );
    }
    match fs::symlink_metadata(&link_path) {
        Ok(m) if m.file_type().is_symlink() => {}
        other => panic!(
            "[{OS}] fixture: the {kind:?} made at {link} is not reported as a link: {other:?}"
        ),
    }
    target_path
}

/// Remove the link of `kind` at `link` (on Windows a directory link is removed as a directory,
/// PROBE E4).
fn unplant(w: &Work, kind: Kind, link: &str) {
    let p = w.at(link);
    let r = if cfg!(windows) && kind.is_dir_link() {
        fs::remove_dir(&p)
    } else {
        fs::remove_file(&p)
    };
    r.unwrap_or_else(|e| panic!("[{OS}] fixture: remove the {kind:?} at {link}: {}", os(&e)));
}

/// Every entry below `dir`: `dir`, `file <bytes>`, `link -> <target>` (never followed), or
/// `other`.
fn snapshot(dir: &Path) -> BTreeMap<String, String> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for e in entries {
            let p = e.unwrap().path();
            let rel = p
                .strip_prefix(base)
                .unwrap()
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            let meta = fs::symlink_metadata(&p).unwrap();
            let desc = if meta.file_type().is_symlink() {
                format!("link -> {:?}", fs::read_link(&p).map_err(|e| e.kind()))
            } else if meta.is_dir() {
                walk(base, &p, out);
                "dir".to_owned()
            } else if meta.is_file() {
                file_desc(&fs::read(&p).unwrap())
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

fn file_desc(bytes: &[u8]) -> String {
    format!("file {}", bytes.escape_ascii())
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

fn listing(
    r: Result<kiln_workspace::DirListing, VfsError>,
) -> Result<Vec<(String, EntryKind)>, VfsError> {
    r.map(|l| l.entries.into_iter().map(|e| (e.name, e.kind)).collect())
}

fn entry(name: &str, kind: EntryKind) -> (String, EntryKind) {
    (name.to_owned(), kind)
}

/// Record a mismatch instead of stopping, so one run shows every case.
fn same<T: PartialEq + Debug>(problems: &mut Vec<String>, ctx: &str, got: T, want: T) {
    if got != want {
        problems.push(format!("[{OS}] {ctx}: expected {want:?}, got {got:?}"));
    }
}

fn finish(test: &str, problems: Vec<String>) {
    assert!(
        problems.is_empty(),
        "[{OS}] {test}: {} mismatch(es):\n{}",
        problems.len(),
        problems.join("\n")
    );
}

/// A short label of a result, for `MEASURED:` lines.
fn label<T>(r: &Result<T, VfsError>) -> String {
    match r {
        Ok(_) => "Ok".to_owned(),
        Err(VfsError::OutsideRoot { .. }) => "OutsideRoot".to_owned(),
        Err(VfsError::NotFound { .. }) => "NotFound".to_owned(),
        Err(VfsError::NotAFile { .. }) => "NotAFile".to_owned(),
        Err(VfsError::Io { kind, .. }) => format!("Io({kind:?})"),
    }
}

fn outside(p: &RelPath) -> VfsError {
    VfsError::OutsideRoot { path: p.clone() }
}

fn new_disk(root: &Path, ctx: &str) -> DiskFs {
    DiskFs::new(root).unwrap_or_else(|e| {
        panic!(
            "[{OS}] {ctx}: DiskFs::new({}): expected Ok, got {e:?}",
            root.display()
        )
    })
}

// ---------------------------------------------------------------------------------------------
// Interface (1), (2): a link or junction on the walk.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dest {
    Inside,
    Outside,
    Nowhere,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pos {
    /// The link is a middle component: requests go to `l/x.txt`, the listing to `l/s`.
    Middle,
    /// The link is the last component: requests and the listing go to `l`.
    Last,
}

/// What every call answers in one case.
struct Expect {
    read: Result<Vec<u8>, VfsError>,
    is_file: bool,
    check: Result<(), VfsError>,
    list: Result<Vec<(String, EntryKind)>, VfsError>,
    /// `Ok(text bytes)` when the document opens; it is then edited and saved (`Ok`).
    open: Result<Vec<u8>, OpenError>,
    write: Result<(), VfsError>,
    /// The fixture file (`/`-separated) the save and the write reach, if any; nothing else
    /// may change.
    written: Option<&'static str>,
}

/// Windows (Interface (1)): no link is followed, every reparse point on the way is
/// `OutsideRoot` before anything is opened, created or replaced.
fn windows_expect(req: &RelPath, list: &RelPath) -> Expect {
    Expect {
        read: Err(outside(req)),
        is_file: false,
        check: Err(outside(req)),
        list: Err(outside(list)),
        open: Err(OpenError::Vfs(outside(req))),
        write: Err(outside(req)),
        written: None,
    }
}

/// Unix today: every link is followed as the kernel does (module docs of `disk.rs`).
fn unix_expect(
    kind: Kind,
    dest: Dest,
    pos: Pos,
    nested: bool,
    req: &RelPath,
    list: &RelPath,
) -> Expect {
    let to_dir = pos == Pos::Middle || kind.is_dir_link();
    let nf = |p: &RelPath| VfsError::NotFound { path: p.clone() };
    let naf = |p: &RelPath| VfsError::NotAFile { path: p.clone() };
    match (dest, pos) {
        (Dest::Outside, _) => windows_expect(req, list),
        (Dest::Inside, Pos::Last) if !to_dir => Expect {
            read: Ok(IN_T.to_vec()),
            is_file: true,
            check: Ok(()),
            list: Err(naf(list)),
            open: Ok(IN_T.to_vec()),
            write: Ok(()),
            written: Some("root/t.txt"),
        },
        (Dest::Inside, Pos::Last) => Expect {
            read: Err(naf(req)),
            is_file: false,
            check: Ok(()),
            list: Ok(vec![
                entry("s", EntryKind::Dir),
                entry("x.txt", EntryKind::File),
            ]),
            open: Err(OpenError::Vfs(naf(req))),
            write: Err(naf(req)),
            written: None,
        },
        (Dest::Inside, Pos::Middle) => Expect {
            read: Ok(IN_X.to_vec()),
            is_file: true,
            check: Ok(()),
            list: Ok(vec![entry("y.txt", EntryKind::File)]),
            open: Ok(IN_X.to_vec()),
            write: Ok(()),
            written: Some(if nested {
                "root/e/x.txt"
            } else {
                "root/d/x.txt"
            }),
        },
        // A dangling link as the last component: a save creates its target.
        (Dest::Nowhere, Pos::Last) => Expect {
            read: Err(nf(req)),
            is_file: false,
            check: Ok(()),
            list: Err(nf(list)),
            open: Err(OpenError::Vfs(nf(req))),
            write: Ok(()),
            written: Some(if to_dir { "root/gone" } else { "root/gone.txt" }),
        },
        (Dest::Nowhere, Pos::Middle) => Expect {
            read: Err(nf(req)),
            is_file: false,
            check: Ok(()),
            list: Err(nf(list)),
            open: Err(OpenError::Vfs(nf(req))),
            write: Err(nf(req)),
            written: None,
        },
    }
}

/// One case: a fixture with a root holding `d/` (`x.txt`, `s/y.txt`), `f.txt` and `t.txt`, a
/// directory `out/` beside it holding the same shape, and a link `root/l` of `kind` to the
/// destination; then every call through it.
fn link_case(test: &str, kind: Kind, dest: Dest, pos: Pos, problems: &mut Vec<String>) {
    link_case_at(test, kind, dest, pos, false, problems);
}

/// [`link_case`]; with `nested`, the link is below the first level (ruling r1, D-257): at
/// `root/d/l`, with requests `d/l/x.txt` (listing `d/l/s`) or `d/l`, and an inside directory
/// destination `root/e` (the same shape as `d`, so the link does not lead to its own parent).
fn link_case_at(
    test: &str,
    kind: Kind,
    dest: Dest,
    pos: Pos,
    nested: bool,
    problems: &mut Vec<String>,
) {
    let level = if nested {
        "below the first level"
    } else {
        "at the first level"
    };
    let case = format!("{test}: {kind:?} to {dest:?} as {pos:?} component, {level}");
    let w = Work::new(&format!(
        "{kind:?}-{dest:?}-{pos:?}-{}",
        if nested { "nested" } else { "top" }
    ));
    w.file("root/f.txt", PLAIN);
    w.file("root/t.txt", IN_T);
    w.file("root/d/x.txt", IN_X);
    w.file("root/d/s/y.txt", IN_Y);
    if nested {
        w.file("root/e/x.txt", IN_X);
        w.file("root/e/s/y.txt", IN_Y);
    }
    w.file("out/t.txt", OUT_T);
    w.file("out/d/x.txt", OUT_X);
    w.file("out/d/s/y.txt", OUT_X);
    let to_dir = pos == Pos::Middle || kind.is_dir_link();
    let target = match (dest, to_dir) {
        (Dest::Inside, false) => "root/t.txt",
        (Dest::Inside, true) if nested => "root/e",
        (Dest::Inside, true) => "root/d",
        (Dest::Outside, false) => "out/t.txt",
        (Dest::Outside, true) => "out/d",
        (Dest::Nowhere, false) => "root/gone.txt",
        (Dest::Nowhere, true) => "root/gone",
    };
    let at = if nested { "d/" } else { "" };
    plant(&w, kind, &format!("root/{at}l"), target);
    let (req, list) = match pos {
        Pos::Middle => (rp(&format!("{at}l/x.txt")), rp(&format!("{at}l/s"))),
        Pos::Last => (rp(&format!("{at}l")), rp(&format!("{at}l"))),
    };
    let want = if cfg!(windows) {
        windows_expect(&req, &list)
    } else {
        unix_expect(kind, dest, pos, nested, &req, &list)
    };
    let before = snapshot(&w.dir);
    let mut disk = new_disk(&w.at("root"), &case);

    let read = disk.read(&req);
    let is_file = disk.is_file(&req);
    let check = disk.check_location(&req);
    let listed = listing(disk.list_dir(Some(&list)));
    measured(
        &case,
        &format!(
            "read {}, is_file {is_file}, check_location {}, list_dir {}",
            label(&read),
            label(&check),
            label(&listed)
        ),
    );
    same(problems, &format!("{case}: read({req})"), read, want.read);
    same(
        problems,
        &format!("{case}: is_file({req})"),
        is_file,
        want.is_file,
    );
    same(
        problems,
        &format!("{case}: check_location({req})"),
        check,
        want.check,
    );
    same(
        problems,
        &format!("{case}: list_dir({list})"),
        listed,
        want.list,
    );

    // Interface (2): the link itself is listed `symlink`, and the project tree never descends
    // into it (both platforms; base behaviour, PROBE C7).
    let (root_list, tree_want) = if nested {
        same(
            problems,
            &format!("{case}: list_dir(d)"),
            listing(disk.list_dir(Some(&rp("d")))),
            Ok(vec![
                entry("l", EntryKind::Symlink),
                entry("s", EntryKind::Dir),
                entry("x.txt", EntryKind::File),
            ]),
        );
        (
            vec![
                entry("d", EntryKind::Dir),
                entry("e", EntryKind::Dir),
                entry("f.txt", EntryKind::File),
                entry("t.txt", EntryKind::File),
            ],
            vec![
                entry("d", EntryKind::Dir),
                entry("d/l", EntryKind::Symlink),
                entry("d/s", EntryKind::Dir),
                entry("d/s/y.txt", EntryKind::File),
                entry("d/x.txt", EntryKind::File),
                entry("e", EntryKind::Dir),
                entry("e/s", EntryKind::Dir),
                entry("e/s/y.txt", EntryKind::File),
                entry("e/x.txt", EntryKind::File),
                entry("f.txt", EntryKind::File),
                entry("t.txt", EntryKind::File),
            ],
        )
    } else {
        (
            vec![
                entry("d", EntryKind::Dir),
                entry("f.txt", EntryKind::File),
                entry("l", EntryKind::Symlink),
                entry("t.txt", EntryKind::File),
            ],
            vec![
                entry("d", EntryKind::Dir),
                entry("d/s", EntryKind::Dir),
                entry("d/s/y.txt", EntryKind::File),
                entry("d/x.txt", EntryKind::File),
                entry("f.txt", EntryKind::File),
                entry("l", EntryKind::Symlink),
                entry("t.txt", EntryKind::File),
            ],
        )
    };
    same(
        problems,
        &format!("{case}: list_dir(root)"),
        listing(disk.list_dir(None)),
        Ok(root_list),
    );
    let tree = project::tree(&disk, None, 1000).map(|t| {
        t.entries
            .into_iter()
            .map(|e| (e.path.as_str().to_owned(), e.kind))
            .collect::<Vec<_>>()
    });
    same(
        problems,
        &format!("{case}: project::tree(root)"),
        tree,
        Ok(tree_want),
    );
    same(
        problems,
        &format!("{case}: the fixture after the reading calls"),
        snapshot(&w.dir),
        before.clone(),
    );

    // A document through the link: opened, edited and saved where it opens.
    let opened = Document::open(&disk, &req);
    match (opened, &want.open) {
        (Ok(mut doc), Ok(text)) => {
            same(
                problems,
                &format!("{case}: Document::open({req}) text"),
                doc.buffer().text().into_bytes(),
                text.clone(),
            );
            doc.buffer_mut().insert(0, EDIT).unwrap();
            let saved = doc.save(&mut disk).map(|_| ());
            measured(&case, &format!("Document open Ok, save {}", label(&saved)));
            same(problems, &format!("{case}: save of {req}"), saved, Ok(()));
            let mut edited = EDIT.as_bytes().to_vec();
            edited.extend_from_slice(text);
            let at = want
                .written
                .expect("a document that opens is written somewhere");
            same(
                problems,
                &format!("{case}: {at} after the save"),
                fs::read(w.at(at)).ok(),
                Some(edited),
            );
        }
        (opened, want_open) => {
            let opened = opened.map(|d| d.buffer().text().into_bytes());
            measured(&case, &format!("Document open {:?}", opened.as_ref().err()));
            same(
                problems,
                &format!("{case}: Document::open({req})"),
                opened,
                want_open.clone(),
            );
        }
    }

    let written = disk.write_atomic(&req, NEW);
    measured(&case, &format!("write_atomic {}", label(&written)));
    same(
        problems,
        &format!("{case}: write_atomic({req})"),
        written,
        want.write,
    );

    // Nothing changed but the file the calls were expected to reach; the link is still the
    // same link; no temporary file left.
    let mut after_want = before;
    if let Some(at) = want.written {
        after_want.insert(at.to_owned(), file_desc(NEW));
    }
    let after = snapshot(&w.dir);
    same(
        problems,
        &format!("{case}: the fixture at the end"),
        &after,
        &after_want,
    );
    let temps: Vec<&String> = after
        .keys()
        .filter(|k| k.rsplit('/').next().unwrap_or(k).starts_with(TEMP_PREFIX))
        .collect();
    same(
        problems,
        &format!("{case}: {TEMP_PREFIX}* left"),
        temps,
        Vec::new(),
    );
}

fn link_kind_test(test: &str, kind: Kind) {
    let mut problems = Vec::new();
    for dest in [Dest::Inside, Dest::Outside, Dest::Nowhere] {
        for pos in [Pos::Middle, Pos::Last] {
            link_case(test, kind, dest, pos, &mut problems);
        }
    }
    finish(test, problems);
}

/// A file symbolic link with a relative target (inside, outside, dangling; middle, last).
/// Windows: every call `OutsideRoot`, nothing read or written (fails at base: PROBE C1 `read
/// sr: Ok`, C2). Unix: followed. Rejects (Windows): the link followed (base); the target
/// created through a dangling link; `NotFound` for a dangling link; a listing of the link as
/// `file`.
#[test]
fn awa1_file_link_relative_on_the_walk() {
    link_kind_test("awa1_file_link_relative_on_the_walk", Kind::FileRel);
}

/// A file symbolic link with an absolute target. Windows: `OutsideRoot` (base too, PROBE C1
/// `read sf`, C4).
#[test]
fn awa1_file_link_absolute_on_the_walk() {
    link_kind_test("awa1_file_link_absolute_on_the_walk", Kind::FileAbs);
}

/// A directory symbolic link (relative target). Windows: `OutsideRoot`.
#[test]
fn awa1_directory_link_on_the_walk() {
    link_kind_test("awa1_directory_link_on_the_walk", Kind::DirSym);
}

/// A junction (Unix: a symbolic link to the absolute path). Windows: `OutsideRoot`, also when
/// it leads inside (base too: PROBE C1, C3, C5, C6, C8); listed `symlink` (C7).
#[test]
fn awa1_junction_on_the_walk() {
    link_kind_test("awa1_junction_on_the_walk", Kind::Junction);
}

/// Ruling r1 (D-257): a link of every kind below the first level, at `root/d/l` (inside,
/// outside, nowhere; middle and last component: `d/l/x.txt`, `d/l`). Windows: every call
/// `OutsideRoot`, nothing changed; Unix: followed, as today. Rejects (Windows): a walk that
/// checks only the first or only the last component of the path for a reparse point.
#[test]
fn awa1_link_below_the_first_level() {
    let test = "awa1_link_below_the_first_level";
    let mut problems = Vec::new();
    for kind in [Kind::FileRel, Kind::FileAbs, Kind::DirSym, Kind::Junction] {
        for dest in [Dest::Inside, Dest::Outside, Dest::Nowhere] {
            for pos in [Pos::Middle, Pos::Last] {
                link_case_at(test, kind, dest, pos, true, &mut problems);
            }
        }
    }
    finish(test, problems);
}

// ---------------------------------------------------------------------------------------------
// Interface (3): a link at a temporary name.

/// A save of `root/f.txt` beside the planted links (name, kind, fixture-relative target): the
/// save `Ok` (by `write_atomic` and through a `Document`), the target holds the new bytes, each
/// link is still the same link, `out/` unchanged, each missing target still missing, and the
/// root holds exactly the planted names and `f.txt`.
fn temp_case(test: &str, plants: &[(&str, Kind, &str)], out_files: &[(&str, &[u8])]) {
    let w = Work::new(test);
    w.file("root/f.txt", PLAIN);
    w.dir("out");
    for (rel, bytes) in out_files {
        w.file(rel, bytes);
    }
    for (name, kind, target) in plants {
        let made = plant(&w, *kind, &format!("root/{name}"), target);
        let exists = fs::symlink_metadata(w.at(target)).is_ok();
        measured(
            test,
            &format!(
                "fixture: {kind:?} made at {name} -> {} (target exists: {exists})",
                made.display()
            ),
        );
    }
    let before = snapshot(&w.dir);
    let mut disk = new_disk(&w.at("root"), test);
    let mut problems = Vec::new();

    let written = disk.write_atomic(&rp("f.txt"), NEW);
    measured(
        test,
        &format!("write_atomic(f.txt) beside the links: {written:?}"),
    );
    same(&mut problems, "write_atomic(f.txt)", written, Ok(()));
    same(
        &mut problems,
        "f.txt after write_atomic",
        fs::read(w.at("root/f.txt")).ok(),
        Some(NEW.to_vec()),
    );
    let saved = match Document::open(&disk, &rp("f.txt")) {
        Ok(mut doc) => {
            doc.buffer_mut().insert(0, EDIT).unwrap();
            doc.save(&mut disk).map(|_| ())
        }
        Err(e) => Err(VfsError::Io {
            path: rp("f.txt"),
            kind: io::ErrorKind::Other,
            message: format!("Document::open failed: {e:?}"),
        }),
    };
    measured(
        test,
        &format!("Document save of f.txt beside the links: {saved:?}"),
    );
    same(&mut problems, "Document save of f.txt", saved, Ok(()));
    let mut edited = EDIT.as_bytes().to_vec();
    edited.extend_from_slice(NEW);
    let mut after_want = before;
    after_want.insert("root/f.txt".to_owned(), file_desc(&edited));
    // The links (by their targets), `out/` and every missing target, all as before.
    same(
        &mut problems,
        "the fixture after both saves",
        snapshot(&w.dir),
        after_want,
    );
    for (_, _, target) in plants {
        if !out_files.iter().any(|(rel, _)| rel == target) && *target != "out" {
            same(
                &mut problems,
                &format!("{target} (missing before)"),
                fs::symlink_metadata(w.at(target))
                    .map(|_| ())
                    .map_err(|e| e.kind()),
                Err(io::ErrorKind::NotFound),
            );
        }
    }
    let mut want_names: Vec<String> = plants.iter().map(|(n, _, _)| (*n).to_owned()).collect();
    want_names.push("f.txt".to_owned());
    want_names.sort();
    same(
        &mut problems,
        "root entries",
        names(&w.at("root")),
        want_names,
    );
    finish(test, problems);
}

/// (a) PROBE H's pair: a junction at `.kiln-tmp-0` to an outside directory and a file link at
/// `.kiln-tmp-1` to a missing outside file. Fails at base on Windows (H2: `PermissionDenied`,
/// os error 5). Rejects: the save refused; a link replaced or removed; the missing file
/// created.
#[test]
fn awa3_temp_names_held_by_a_junction_and_a_dangling_file_link() {
    temp_case(
        "awa3_temp_names_held_by_a_junction_and_a_dangling_file_link",
        &[
            (".kiln-tmp-0", Kind::Junction, "out"),
            (".kiln-tmp-1", Kind::FileAbs, "out/t"),
        ],
        &[],
    );
}

/// (b) A junction alone at `.kiln-tmp-0` to an outside directory (with a file in it).
#[test]
fn awa3_temp_name_held_by_a_junction() {
    temp_case(
        "awa3_temp_name_held_by_a_junction",
        &[(".kiln-tmp-0", Kind::Junction, "out")],
        &[("out/o.txt", OUT_T)],
    );
}

/// (c) A file symbolic link alone at `.kiln-tmp-0` to an existing outside file: that file is
/// neither opened for writing nor changed (Pinned (b2)).
#[test]
fn awa3_temp_name_held_by_a_file_link_to_an_outside_file() {
    temp_case(
        "awa3_temp_name_held_by_a_file_link_to_an_outside_file",
        &[(".kiln-tmp-0", Kind::FileAbs, "out/t.txt")],
        &[("out/t.txt", OUT_T)],
    );
}

/// (d) A dangling file symbolic link alone at `.kiln-tmp-0` to a missing outside path: the
/// path is still missing afterwards (Pinned (b2)). Rejects: the temporary file created through
/// the link (outside the root) and the link renamed over the target.
#[test]
fn awa3_temp_name_held_by_a_dangling_file_link_to_outside() {
    temp_case(
        "awa3_temp_name_held_by_a_dangling_file_link_to_outside",
        &[(".kiln-tmp-0", Kind::FileAbs, "out/missing")],
        &[],
    );
}

// ---------------------------------------------------------------------------------------------
// Interface (4): the root cases (the same behaviour on every platform, except `..`).

/// After the root was replaced or removed: every call `OutsideRoot`, a document opened before
/// cannot be saved, and the fixture is unchanged.
fn assert_every_call_outside(
    w: &Work,
    disk: &mut DiskFs,
    doc: Option<&mut Document>,
    case: &str,
    problems: &mut Vec<String>,
) {
    let before = snapshot(&w.dir);
    let f = rp("f.txt");
    let read = disk.read(&f);
    let write = disk.write_atomic(&rp("new.txt"), NEW);
    let is_file = disk.is_file(&f);
    let list = disk.list_dir(None).map(|_| ());
    let check = disk.check_location(&f);
    let saved = doc.map(|d| d.save(disk).map(|_| ()));
    measured(
        case,
        &format!(
            "read {}, write_atomic {}, is_file {is_file}, list_dir {}, check_location {}, save {}",
            label(&read),
            label(&write),
            label(&list),
            label(&check),
            saved.as_ref().map_or("-".to_owned(), label)
        ),
    );
    same(
        problems,
        &format!("{case}: read(f.txt)"),
        read,
        Err(outside(&f)),
    );
    same(
        problems,
        &format!("{case}: write_atomic(new.txt)"),
        write,
        Err(outside(&rp("new.txt"))),
    );
    same(problems, &format!("{case}: is_file(f.txt)"), is_file, false);
    same(
        problems,
        &format!("{case}: list_dir(None)"),
        list.map_err(|e| match e {
            VfsError::OutsideRoot { path } => format!("OutsideRoot naming {}", path.as_str()),
            other => format!("{other:?}"),
        }),
        Err("OutsideRoot naming (root)".to_owned()),
    );
    same(
        problems,
        &format!("{case}: check_location(f.txt)"),
        check,
        Err(outside(&f)),
    );
    if let Some(saved) = saved {
        same(
            problems,
            &format!("{case}: save of f.txt"),
            saved,
            Err(outside(&f)),
        );
    }
    same(
        problems,
        &format!("{case}: the fixture"),
        snapshot(&w.dir),
        before,
    );
}

/// The session works on `root_dir` (fixture-relative), whose `f.txt` holds `want`: read, then
/// a save that reaches exactly that file.
fn assert_works(
    w: &Work,
    disk: &mut DiskFs,
    root_dir: &str,
    want: &[u8],
    case: &str,
    problems: &mut Vec<String>,
) {
    let read = disk.read(&rp("f.txt"));
    measured(case, &format!("read f.txt: {}", label(&read)));
    same(
        problems,
        &format!("{case}: read(f.txt)"),
        read,
        Ok(want.to_vec()),
    );
    let mut after_want = snapshot(&w.dir);
    let write = disk.write_atomic(&rp("f.txt"), NEW);
    same(
        problems,
        &format!("{case}: write_atomic(f.txt)"),
        write,
        Ok(()),
    );
    after_want.insert(format!("{root_dir}/f.txt"), file_desc(NEW));
    same(
        problems,
        &format!("{case}: the fixture after the save"),
        snapshot(&w.dir),
        after_want,
    );
}

/// `--root` given as a link of `kind` to `real`: the root is `real`; retargeted to `other`,
/// every call `OutsideRoot` and nothing in `other` read or written; pointed back, the session
/// works again.
fn root_through_link(test: &str, kind: Kind) {
    let w = Work::new(test);
    w.file("real/f.txt", REAL);
    w.file("other/f.txt", OTHER);
    plant(&w, kind, "link", "real");
    let mut problems = Vec::new();
    let mut disk = new_disk(&w.at("link"), test);
    let read = disk.read(&rp("f.txt"));
    measured(test, &format!("{kind:?} as --root: read f.txt: {read:?}"));
    same(
        &mut problems,
        "through the link: read(f.txt)",
        read,
        Ok(REAL.to_vec()),
    );
    let mut doc = Document::open(&disk, &rp("f.txt"))
        .unwrap_or_else(|e| panic!("[{OS}] {test}: Document::open(f.txt): {e:?}"));
    doc.buffer_mut().insert(0, EDIT).unwrap();

    unplant(&w, kind, "link");
    plant(&w, kind, "link", "other");
    assert_every_call_outside(
        &w,
        &mut disk,
        Some(&mut doc),
        &format!("{test}: {kind:?} retargeted to another directory"),
        &mut problems,
    );

    unplant(&w, kind, "link");
    plant(&w, kind, "link", "real");
    assert_works(
        &w,
        &mut disk,
        "real",
        REAL,
        &format!("{test}: {kind:?} pointed back"),
        &mut problems,
    );
    finish(test, problems);
}

/// Interface (4), first bullet: a junction as `--root` (PROBE E2-E7).
#[test]
fn awa4_root_given_through_a_junction_and_retargeted() {
    root_through_link(
        "awa4_root_given_through_a_junction_and_retargeted",
        Kind::Junction,
    );
}

/// Interface (4), first bullet: a directory symbolic link as `--root` (D-256 Q-4; unprobed).
#[test]
fn awa4_root_given_through_a_directory_link_and_retargeted() {
    root_through_link(
        "awa4_root_given_through_a_directory_link_and_retargeted",
        Kind::DirSym,
    );
}

/// Interface (4), second bullet: the root renamed away and a junction to another directory at
/// its path: every call `OutsideRoot`, the other directory unchanged (PROBE G1-G3); a junction
/// at the path to the held root: works (G5).
#[test]
fn awa4_root_replaced_by_a_junction() {
    let test = "awa4_root_replaced_by_a_junction";
    let w = Work::new(test);
    w.file("root/f.txt", REAL);
    w.file("other/f.txt", OTHER);
    let mut problems = Vec::new();
    let mut disk = new_disk(&w.at("root"), test);
    let mut doc = Document::open(&disk, &rp("f.txt"))
        .unwrap_or_else(|e| panic!("[{OS}] {test}: Document::open(f.txt): {e:?}"));
    doc.buffer_mut().insert(0, EDIT).unwrap();
    let renamed = fs::rename(w.at("root"), w.at("root.real"));
    measured(test, &format!("rename of the held root: {renamed:?}"));
    renamed.unwrap_or_else(|e| {
        panic!(
            "[{OS}] {test}: rename the held root away (PROBE G1: Ok): {}",
            os(&e)
        )
    });
    plant(&w, Kind::Junction, "root", "other");
    assert_every_call_outside(
        &w,
        &mut disk,
        Some(&mut doc),
        &format!("{test}: a junction to another directory at the root's path"),
        &mut problems,
    );
    unplant(&w, Kind::Junction, "root");
    plant(&w, Kind::Junction, "root", "root.real");
    assert_works(
        &w,
        &mut disk,
        "root.real",
        REAL,
        &format!("{test}: a junction at the root's path to the held root"),
        &mut problems,
    );
    finish(test, problems);
}

/// Interface (4), third bullet: the held root removed while the session runs: empty
/// (`remove_dir`), then a new directory at the name (PROBE F1-F5); with a file
/// (`remove_dir_all`, F7, F8); and removed with a junction to a new directory put at the name
/// (unprobed, Interface (4) last paragraph). Every call `OutsideRoot` after each.
#[test]
fn awa4_root_removed_while_held() {
    let test = "awa4_root_removed_while_held";
    let mut problems = Vec::new();

    let w = Work::new("removed-empty");
    w.dir("root");
    let mut disk = new_disk(&w.at("root"), test);
    let removed = fs::remove_dir(w.at("root"));
    measured(
        test,
        &format!("remove_dir of the held empty root: {removed:?}"),
    );
    removed.unwrap_or_else(|e| {
        panic!(
            "[{OS}] {test}: remove_dir(held root) (PROBE F1: Ok): {}",
            os(&e)
        )
    });
    assert_every_call_outside(
        &w,
        &mut disk,
        None,
        &format!("{test}: empty root removed"),
        &mut problems,
    );
    w.dir("root");
    assert_every_call_outside(
        &w,
        &mut disk,
        None,
        &format!("{test}: empty root removed, a new directory at its name"),
        &mut problems,
    );
    same(
        &mut problems,
        "the new directory",
        names(&w.at("root")),
        Vec::<String>::new(),
    );

    let w = Work::new("removed-full");
    w.file("root/f.txt", REAL);
    let mut disk = new_disk(&w.at("root"), test);
    let mut doc = Document::open(&disk, &rp("f.txt"))
        .unwrap_or_else(|e| panic!("[{OS}] {test}: Document::open(f.txt): {e:?}"));
    doc.buffer_mut().insert(0, EDIT).unwrap();
    let removed = fs::remove_dir_all(w.at("root"));
    measured(
        test,
        &format!("remove_dir_all of the held root with a file: {removed:?}"),
    );
    removed.unwrap_or_else(|e| {
        panic!(
            "[{OS}] {test}: remove_dir_all(held root) (PROBE F7: Ok): {}",
            os(&e)
        )
    });
    assert_every_call_outside(
        &w,
        &mut disk,
        Some(&mut doc),
        &format!("{test}: root with a file removed"),
        &mut problems,
    );

    let w = Work::new("removed-junction");
    w.file("root/f.txt", REAL);
    w.file("fresh/f.txt", OTHER);
    let mut disk = new_disk(&w.at("root"), test);
    let mut doc = Document::open(&disk, &rp("f.txt"))
        .unwrap_or_else(|e| panic!("[{OS}] {test}: Document::open(f.txt): {e:?}"));
    doc.buffer_mut().insert(0, EDIT).unwrap();
    fs::remove_dir_all(w.at("root"))
        .unwrap_or_else(|e| panic!("[{OS}] {test}: remove_dir_all(held root): {}", os(&e)));
    plant(&w, Kind::Junction, "root", "fresh");
    assert_every_call_outside(
        &w,
        &mut disk,
        Some(&mut doc),
        &format!("{test}: root removed, a junction to a new directory at its name"),
        &mut problems,
    );
    finish(test, problems);
}

/// Interface (4), fourth bullet: `..` in the `--root` text after a junction. Windows: resolved
/// by its text, so the root is the `b` beside the junction (PROBE K2-K4); Unix: physically, the
/// `b` beside the junction's target. A save reaches the same `b`.
#[test]
fn awa4_dotdot_after_a_junction_in_the_root_text() {
    let test = "awa4_dotdot_after_a_junction_in_the_root_text";
    let w = Work::new(test);
    w.dir("k/a/real");
    w.file("k/a/b/f.txt", b"physical\n");
    w.file("k/b/f.txt", b"lexical\n");
    plant(&w, Kind::Junction, "k/link", "k/a/real");
    let given = w.at("k").join("link").join("..").join("b");
    let mut disk = new_disk(&given, test);
    let (root_dir, want): (&str, &[u8]) = if cfg!(windows) {
        ("k/b", b"lexical\n")
    } else {
        ("k/a/b", b"physical\n")
    };
    let mut problems = Vec::new();
    assert_works(
        &w,
        &mut disk,
        root_dir,
        want,
        &format!("{test}: --root k/link/../b"),
        &mut problems,
    );
    finish(test, problems);
}

/// (e) Ruling r2 (D-257): a dangling junction alone at `.kiln-tmp-0` (Unix: a dangling symbolic
/// link to an absolute path) to a missing outside directory: the save goes through the next
/// free name, the junction is neither followed nor changed, the missing path still missing.
/// Whether the runner can make a dangling junction is unchecked: the `MEASURED:` line shows
/// it, and the test fails naming why if it cannot.
#[test]
fn awa3_temp_name_held_by_a_dangling_junction() {
    temp_case(
        "awa3_temp_name_held_by_a_dangling_junction",
        &[(".kiln-tmp-0", Kind::Junction, "out/gone")],
        &[],
    );
}
