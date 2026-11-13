//! Implementer's integration tests of T-019: `Vfs::list_dir` on both file systems and the
//! project walk `kiln_workspace::project::tree`, on disk fixtures with symlinks, FIFOs,
//! sockets and odd names. Every call that could block on a FIFO runs under a hard timeout.

use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileTypeExt, symlink};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use kiln_workspace::project::{MAX_DEPTH, ProjectTree, tree};
use kiln_workspace::{DirEntry, DiskFs, EntryKind, MemoryFs, RelPath, Vfs, VfsError};

#[cfg(test)]
mod support {
    use super::*;

    pub const TIMEOUT: Duration = Duration::from_secs(5);
    pub const SECRET: &[u8] = b"*\n";

    pub fn p(s: &str) -> RelPath {
        RelPath::new(s).unwrap()
    }

    /// `work/root/` and `work/outside/` (holding `f.rs` and a `.gitignore` of `*`), under the
    /// system temp dir with a short name (socket paths are limited to 108 bytes).
    pub fn work(name: &str) -> PathBuf {
        let work = std::env::temp_dir().join(format!("k19i-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&work);
        fs::create_dir_all(work.join("root")).unwrap();
        fs::create_dir_all(work.join("outside")).unwrap();
        fs::write(work.join("outside/f.rs"), b"fn f() {}\n").unwrap();
        fs::write(work.join("outside/.gitignore"), SECRET).unwrap();
        work
    }

    pub fn write(root: &Path, rel: &str, content: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
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

    pub fn entry(name: &str, kind: EntryKind) -> DirEntry {
        DirEntry {
            name: name.to_owned(),
            kind,
        }
    }

    pub fn listed(t: &ProjectTree) -> Vec<(String, EntryKind)> {
        t.entries
            .iter()
            .map(|e| (e.path.to_string(), e.kind))
            .collect()
    }

    pub fn paths(t: &ProjectTree) -> Vec<String> {
        t.entries.iter().map(|e| e.path.to_string()).collect()
    }

    pub fn disk_tree(root: &Path, under: Option<&str>) -> Result<ProjectTree, VfsError> {
        let root = root.to_path_buf();
        let under = under.map(p);
        bounded("tree", move || {
            let fs_ = DiskFs::new(&root).unwrap();
            tree(&fs_, under.as_ref(), 50_000)
        })
    }
}

use support::*;

#[test]
fn disk_list_dir_reports_own_kinds_sorted_and_skips_odd_names() {
    let work = work("kinds");
    let root = work.join("root");
    write(&root, "b.rs", "");
    write(&root, "B.rs", "");
    write(&root, "d/x", "");
    symlink("d", root.join("to_dir")).unwrap();
    symlink("../outside", root.join("out")).unwrap();
    symlink("nowhere", root.join("dangling")).unwrap();
    symlink("loop2", root.join("loop1")).unwrap();
    symlink("loop1", root.join("loop2")).unwrap();
    mkfifo(&root.join("fifo"));
    let _socket = UnixListener::bind(root.join("sock")).unwrap();
    fs::write(root.join("back\\slash"), b"").unwrap();
    fs::write(root.join(OsStr::from_bytes(b"n\xff")), b"").unwrap();

    let r = root.clone();
    let listing = bounded("list_dir", move || {
        DiskFs::new(&r).unwrap().list_dir(None).unwrap()
    });
    use EntryKind::*;
    assert_eq!(
        listing.entries,
        vec![
            entry("B.rs", File),
            entry("b.rs", File),
            entry("d", Dir),
            entry("dangling", Symlink),
            entry("fifo", Other),
            entry("loop1", Symlink),
            entry("loop2", Symlink),
            entry("out", Symlink),
            entry("sock", Other),
            entry("to_dir", Symlink),
        ]
    );
    assert_eq!(listing.skipped, 2);
    // A symlink on the way that stays inside is followed (T-008's location rule).
    let fs_ = DiskFs::new(&root).unwrap();
    assert_eq!(
        fs_.list_dir(Some(&p("to_dir"))).unwrap().entries,
        vec![entry("x", File)]
    );
    fs::remove_dir_all(&work).unwrap();
}

#[test]
fn disk_list_dir_errors_follow_t008_precedence() {
    let work = work("errors");
    let root = work.join("root");
    write(&root, "f.rs", "");
    symlink("../outside", root.join("out")).unwrap();
    symlink("loop2", root.join("loop1")).unwrap();
    symlink("loop1", root.join("loop2")).unwrap();
    mkfifo(&root.join("fifo"));
    let r = root.clone();
    let results = bounded("list_dir", move || {
        let fs_ = DiskFs::new(&r).unwrap();
        ["f.rs", "fifo", "nope", "f.rs/x", "out", "out/f.rs", "loop1"]
            .iter()
            .map(|s| fs_.list_dir(Some(&p(s))))
            .collect::<Vec<_>>()
    });
    assert_eq!(results[0], Err(VfsError::NotAFile { path: p("f.rs") }));
    assert_eq!(results[1], Err(VfsError::NotAFile { path: p("fifo") }));
    assert_eq!(results[2], Err(VfsError::NotFound { path: p("nope") }));
    assert_eq!(results[3], Err(VfsError::NotFound { path: p("f.rs/x") }));
    assert_eq!(results[4], Err(VfsError::OutsideRoot { path: p("out") }));
    assert_eq!(
        results[5],
        Err(VfsError::OutsideRoot {
            path: p("out/f.rs")
        })
    );
    assert!(matches!(&results[6], Err(VfsError::Io { path, .. }) if path == &p("loop1")));

    // The root path replaced by a link elsewhere: the root's error names `(root)` only.
    let fs_ = DiskFs::new(&root).unwrap();
    fs::rename(&root, work.join("old")).unwrap();
    symlink("outside", &root).unwrap();
    let err = fs_.list_dir(None).unwrap_err();
    assert!(matches!(err, VfsError::OutsideRoot { .. }));
    assert_eq!(err.path().as_str(), "(root)");
    assert!(!err.to_string().contains(work.to_string_lossy().as_ref()));
    fs::remove_dir_all(&work).unwrap();
}

#[test]
fn memory_list_dir_errors() {
    let mut fs_ = MemoryFs::new();
    fs_.insert(&p("a/b"), b"");
    assert_eq!(
        fs_.list_dir(Some(&p("a/b"))),
        Err(VfsError::NotAFile { path: p("a/b") })
    );
    assert_eq!(
        fs_.list_dir(Some(&p("a/b/c"))),
        Err(VfsError::NotFound { path: p("a/b/c") })
    );
    assert_eq!(
        fs_.list_dir(Some(&p("z"))),
        Err(VfsError::NotFound { path: p("z") })
    );
}

/// AC-4's fixture, with the walk: links and FIFOs listed by kind, never followed or opened,
/// and neither outside `*` pattern applies.
#[test]
fn symlinks_and_fifos_are_listed_not_followed() {
    let work = work("links");
    let root = work.join("root");
    symlink("../outside", root.join("out")).unwrap();
    symlink(".", root.join("self")).unwrap();
    symlink("loop2", root.join("loop1")).unwrap();
    symlink("loop1", root.join("loop2")).unwrap();
    mkfifo(&root.join("p"));
    symlink("../outside/.gitignore", root.join(".gitignore")).unwrap();
    write(&root, "keep.rs", "");
    let before = fs::read(work.join("outside/.gitignore")).unwrap();

    use EntryKind::*;
    let want = [
        (".gitignore", Symlink),
        ("keep.rs", File),
        ("loop1", Symlink),
        ("loop2", Symlink),
        ("out", Symlink),
        ("p", Other),
        ("self", Symlink),
    ];
    let t = disk_tree(&root, None).unwrap();
    let expect: Vec<(String, EntryKind)> = want.iter().map(|(s, k)| (s.to_string(), *k)).collect();
    assert_eq!(listed(&t), expect);
    assert_eq!(
        (t.ignored, t.skipped, t.too_deep, t.truncated),
        (0, 0, 0, false)
    );

    assert_eq!(
        disk_tree(&root, Some("out")),
        Err(VfsError::OutsideRoot { path: p("out") })
    );
    let t = disk_tree(&root, Some("self")).unwrap();
    let expect: Vec<(String, EntryKind)> = want
        .iter()
        .map(|(s, k)| (format!("self/{s}"), *k))
        .collect();
    assert_eq!(listed(&t), expect);

    // A `.gitignore` that is a FIFO or a directory contributes no pattern.
    fs::remove_file(root.join(".gitignore")).unwrap();
    mkfifo(&root.join(".gitignore"));
    let t = disk_tree(&root, None).unwrap();
    assert_eq!(t.entries[0].kind, Other);
    assert_eq!(t.entries.len(), 7);
    let sub = root.join("sub");
    fs::create_dir_all(sub.join(".gitignore")).unwrap();
    write(&sub, ".gitignore/x", "*\n");
    write(&sub, "y", "");
    let t = disk_tree(&sub, None).unwrap();
    assert_eq!(paths(&t), [".gitignore", ".gitignore/x", "y"]);

    assert_eq!(fs::read(work.join("outside/.gitignore")).unwrap(), before);
    fs::remove_dir_all(&work).unwrap();
}

/// A tree built both on disk and in memory gives the same walk.
#[test]
fn memory_and_disk_walks_agree() {
    let files = [
        (".gitignore", "*.log\n/target\nbuild/\n!keep.log\n"),
        ("a/.gitignore", "/x\n**/deep\n"),
        ("a/x", ""),
        ("a/y/x", ""),
        ("a/y/deep/z", ""),
        ("a/y/z", ""),
        ("b.log", ""),
        ("keep.log", ""),
        ("build/o", ""),
        ("target/debug/k", ""),
        ("src/target/t", ""),
        ("src/.git/HEAD", ""),
        ("src/.git2/HEAD", ""),
        ("Z", ""),
    ];
    let work = work("agree");
    let root = work.join("root");
    let mut mem = MemoryFs::new();
    for (path, content) in files {
        write(&root, path, content);
        mem.insert(&p(path), content.as_bytes());
    }
    let disk = DiskFs::new(&root).unwrap();
    for under in [
        None,
        Some("a"),
        Some("a/y"),
        Some("src"),
        Some("target"),
        Some("build"),
    ] {
        let under = under.map(p);
        let on_disk = tree(&disk, under.as_ref(), 1000);
        let in_memory = tree(&mem, under.as_ref(), 1000);
        assert_eq!(on_disk, in_memory, "under {under:?}");
    }
    let t = tree(&mem, None, 1000).unwrap();
    assert_eq!(
        paths(&t),
        [
            ".gitignore",
            "Z",
            "a",
            "a/.gitignore",
            "a/y",
            "a/y/x",
            "a/y/z",
            "keep.log",
            "src",
            "src/.git2",
            "src/.git2/HEAD",
            "src/target",
            "src/target/t"
        ]
    );
    // a/x, a/y/deep, b.log, build, target, src/.git
    assert_eq!(t.ignored, 6);
    // `under` is not subject to the rules, its contents are.
    assert_eq!(
        paths(&tree(&mem, Some(&p("target")), 10).unwrap()),
        ["target/debug", "target/debug/k"]
    );
    fs::remove_dir_all(&work).unwrap();
}

/// Q-8: a `.gitignore` that a pattern ignores is still read; it is not listed.
#[test]
fn an_ignored_gitignore_is_still_read() {
    let mut mem = MemoryFs::new();
    mem.insert(&p(".gitignore"), b".*\n");
    mem.insert(&p("sub/.gitignore"), b"secret.txt\n");
    mem.insert(&p("sub/secret.txt"), b"");
    mem.insert(&p("sub/keep.txt"), b"");
    mem.insert(&p("top.txt"), b"");
    let t = tree(&mem, None, 100).unwrap();
    assert_eq!(paths(&t), ["sub", "sub/keep.txt", "top.txt"]);
    assert_eq!(t.ignored, 3);
}

/// Pinned (i) S6, Kiln side: the documented differences from git.
#[test]
fn documented_differences_from_git() {
    let work = work("s6");
    let root = work.join("root");
    // Braces are alternatives.
    write(&root, "br/.gitignore", "{a,b}\n");
    write(&root, "br/a", "");
    write(&root, "br/b", "");
    write(&root, "br/{a,b}", "");
    // A trailing tab is removed from a pattern.
    write(&root, "tab/.gitignore", "*.log\t\n");
    write(&root, "tab/x.log", "");
    // A directory holding its own `.git` is listed with its contents.
    write(&root, "nested/.git/HEAD", "");
    write(&root, "nested/f", "");
    // Odd names are skipped.
    write(&root, "odd/ok", "");
    fs::write(root.join("odd/back\\slash"), b"").unwrap();
    fs::write(root.join("odd").join(OsStr::from_bytes(b"n\xff")), b"").unwrap();

    let t = disk_tree(&root, None).unwrap();
    assert_eq!(
        paths(&t),
        [
            "br",
            "br/.gitignore",
            "br/{a,b}",
            "nested",
            "nested/f",
            "odd",
            "odd/ok",
            "tab",
            "tab/.gitignore"
        ]
    );
    assert_eq!(t.skipped, 2);
    // br/a, br/b, nested/.git, tab/x.log
    assert_eq!(t.ignored, 4);
    fs::remove_dir_all(&work).unwrap();
}

#[test]
fn depth_bound_on_disk_and_in_memory() {
    let work = work("deep");
    let root = work.join("root");
    let chain = vec!["d"; 130].join("/");
    write(&root, &format!("{chain}/f"), "");
    let mut mem = MemoryFs::new();
    mem.insert(&p(&format!("{chain}/f")), b"");
    let on_disk = disk_tree(&root, None).unwrap();
    let in_memory = tree(&mem, None, 50_000).unwrap();
    assert_eq!(on_disk, in_memory);
    assert_eq!(on_disk.entries.len(), MAX_DEPTH);
    assert!(on_disk.entries.iter().all(|e| e.kind == EntryKind::Dir));
    assert_eq!(
        on_disk
            .entries
            .last()
            .unwrap()
            .path
            .as_str()
            .split('/')
            .count(),
        MAX_DEPTH
    );
    assert_eq!(on_disk.too_deep, 1);
    // From a deep `under`, no directory at or beyond the bound is descended either.
    let under = vec!["d"; 129].join("/");
    let t = tree(&mem, Some(&p(&under)), 50_000).unwrap();
    assert_eq!(paths(&t), [format!("{under}/d")]);
    assert_eq!(t.too_deep, 1);
    fs::remove_dir_all(&work).unwrap();
}

#[test]
fn limit_counts_listed_entries_only() {
    let mut mem = MemoryFs::new();
    for f in [".gitignore", "a/x", "a/y", "b.log", "c"] {
        let content: &[u8] = if f == ".gitignore" { b"*.log\n" } else { b"" };
        mem.insert(&p(f), content);
    }
    let all = tree(&mem, None, 100).unwrap();
    assert_eq!(paths(&all), [".gitignore", "a", "a/x", "a/y", "c"]);
    for limit in 0..=6 {
        let t = tree(&mem, None, limit).unwrap();
        let n = limit.min(5);
        assert_eq!(t.entries, all.entries[..n].to_vec(), "limit {limit}");
        assert_eq!(t.truncated, limit < 5, "limit {limit}");
    }
    // The walk met `b.log` (ignored) only when it got that far.
    assert_eq!(tree(&mem, None, 3).unwrap().ignored, 0);
    assert_eq!(tree(&mem, None, 4).unwrap().ignored, 1);
}

#[test]
fn walk_writes_nothing() {
    let work = work("ro");
    let root = work.join("root");
    write(&root, ".gitignore", "*.o\n");
    write(&root, "a/b.o", "x");
    write(&root, "a/c.rs", "y");
    let before = fs::read_dir(root.join("a")).unwrap().count();
    let meta = fs::metadata(root.join("a/c.rs")).unwrap();
    disk_tree(&root, None).unwrap();
    assert_eq!(fs::read_dir(root.join("a")).unwrap().count(), before);
    assert_eq!(
        fs::metadata(root.join("a/c.rs"))
            .unwrap()
            .modified()
            .unwrap(),
        meta.modified().unwrap()
    );
    fs::remove_dir_all(&work).unwrap();
}

/// Ruling y3: a `.gitignore` whose patterns cannot be compiled together (here: too many for
/// the matcher's regex size limit) contributes no patterns, and the listing goes on; another
/// `.gitignore` still applies.
#[test]
fn a_gitignore_that_cannot_be_compiled_contributes_no_patterns() {
    let patterns = |n: usize| {
        let mut text: String = (0..n).map(|i| format!("*a{i}*b*[c-x]?d*e\n")).collect();
        text.push_str("*.log\n");
        text
    };
    let fs_with = |n: usize| {
        let mut mem = MemoryFs::new();
        mem.insert(&p(".gitignore"), patterns(n).as_bytes());
        mem.insert(&p("x.log"), b"");
        mem.insert(&p("sub/.gitignore"), b"*.tmp\n");
        mem.insert(&p("sub/y.tmp"), b"");
        mem.insert(&p("sub/z"), b"");
        mem
    };
    // A file of the same form that compiles: its `*.log` applies.
    let small = tree(&fs_with(100), None, 100).unwrap();
    assert_eq!(
        paths(&small),
        [".gitignore", "sub", "sub/.gitignore", "sub/z"]
    );
    // 20,000 such patterns exceed the size limit: none applies, nothing fails.
    let big = tree(&fs_with(20_000), None, 100).unwrap();
    assert_eq!(
        paths(&big),
        [".gitignore", "sub", "sub/.gitignore", "sub/z", "x.log"]
    );
    assert_eq!((big.ignored, big.truncated), (1, false));
}
