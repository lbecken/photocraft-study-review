//! Acceptance tests for T-002, round 2: the director's rulings of D-014 on save hygiene
//! (temp names, symlinks, permissions, long names, error variants). Written from the spec and
//! the rulings only. Supported platforms are Unix, so Unix APIs are used directly.
//!
//! The temp-file naming scheme is unknown and not assumed: every check is made on the
//! directory as a whole (snapshot before and after), with decoys planted under many plausible
//! temp names.

use kiln_workspace::{DiskFs, Document, MemoryFs, RelPath, Vfs, VfsError};
use support::*;

#[cfg(test)]
mod support {
    use super::*;
    use std::collections::BTreeMap;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::path::{Path, PathBuf};

    pub fn rp(s: &str) -> RelPath {
        RelPath::new(s).unwrap_or_else(|e| panic!("RelPath::new({s:?}) rejected: {e:?}"))
    }

    pub fn fresh_dir(name: &str) -> PathBuf {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("acceptance_t002_r2")
            .join(name);
        if dir.exists() {
            // Restore write permission on anything a test made read-only.
            restore_perms(&dir);
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn restore_perms(dir: &Path) {
        let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o755));
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                let Ok(m) = std::fs::symlink_metadata(&p) else {
                    continue;
                };
                if m.is_dir() {
                    restore_perms(&p);
                } else if m.is_file() {
                    let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644));
                }
            }
        }
    }

    pub fn put(p: &Path, bytes: &[u8]) {
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(p, bytes).unwrap();
    }

    pub fn set_mode(p: &Path, mode: u32) {
        std::fs::set_permissions(p, std::fs::Permissions::from_mode(mode)).unwrap();
    }

    pub fn mode(p: &Path) -> u32 {
        std::fs::metadata(p).unwrap().permissions().mode() & 0o7777
    }

    /// One directory entry, seen without following symlinks.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Entry {
        File { bytes: Vec<u8>, mode: u32, ino: u64 },
        Link { target: PathBuf },
        Dir { mode: u32 },
    }

    /// Every entry below `root` (relative `/`-path → entry), symlinks not followed.
    pub fn snapshot(root: &Path) -> BTreeMap<String, Entry> {
        fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Entry>) {
            for e in std::fs::read_dir(dir).unwrap() {
                let p = e.unwrap().path();
                let rel = p
                    .strip_prefix(root)
                    .unwrap()
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                let m = std::fs::symlink_metadata(&p).unwrap();
                let ft = m.file_type();
                if ft.is_symlink() {
                    out.insert(
                        rel,
                        Entry::Link {
                            target: std::fs::read_link(&p).unwrap(),
                        },
                    );
                } else if ft.is_dir() {
                    out.insert(
                        rel,
                        Entry::Dir {
                            mode: m.permissions().mode() & 0o7777,
                        },
                    );
                    walk(root, &p, out);
                } else {
                    out.insert(
                        rel,
                        Entry::File {
                            bytes: std::fs::read(&p).unwrap(),
                            mode: m.permissions().mode() & 0o7777,
                            ino: m.ino(),
                        },
                    );
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(root, root, &mut out);
        out
    }

    /// `before` with the entry at `rel` replaced by a file holding `bytes` (inode ignored).
    pub fn assert_only_target_changed(
        before: &BTreeMap<String, Entry>,
        after: &BTreeMap<String, Entry>,
        rel: &str,
        bytes: &[u8],
        ctx: &str,
    ) {
        let keys_b: Vec<&String> = before.keys().collect();
        let keys_a: Vec<&String> = after.keys().collect();
        assert_eq!(keys_a, keys_b, "{ctx}: set of entries changed");
        for (k, b) in before {
            let a = &after[k];
            if k == rel {
                match (b, a) {
                    (
                        Entry::File { mode: mb, .. },
                        Entry::File {
                            bytes: ba,
                            mode: ma,
                            ..
                        },
                    ) => {
                        assert_eq!(ba.as_slice(), bytes, "{ctx}: target content");
                        assert_eq!(ma, mb, "{ctx}: target mode");
                    }
                    _ => panic!("{ctx}: target {k} changed kind: {b:?} -> {a:?}"),
                }
            } else {
                assert_eq!(a, b, "{ctx}: entry {k} must be untouched");
            }
        }
    }

    /// Plausible temp-file names an implementation might derive from `name`.
    pub fn decoy_names(name: &str) -> Vec<String> {
        let mut v: Vec<String> = [
            ".{}.kiln-tmp",
            "{}.kiln-tmp",
            ".{}.kiln-tmp.0",
            ".{}.kiln-tmp-0",
            ".{}.kiln-tmp.1",
            ".{}.0.kiln-tmp",
            ".{}.kiln-save",
            "{}.kiln-save",
            ".{}.kiln",
            ".kiln-tmp-{}",
            ".kiln-save-{}",
            ".{}.tmp",
            "{}.tmp",
            ".{}.tmp.0",
            "{}.tmp.0",
            ".{}~",
            "{}~",
            ".{}.swp",
            ".#{}",
            "#{}#",
            ".{}.new",
            "{}.new",
            "{}.bak",
            ".{}.part",
            "{}.part",
            "{}.partial",
        ]
        .iter()
        .map(|p| p.replace("{}", name))
        .collect();
        v.extend(
            [
                ".kiln-tmp",
                ".kiln-tmp-0",
                ".kiln-tmp.0",
                ".kiln-tmp-1",
                ".kiln-save",
                ".tmp",
                ".tmp0",
                "tmp",
                ".save",
            ]
            .iter()
            .map(|s| s.to_string()),
        );
        v.sort();
        v.dedup();
        v
    }

    /// Can this process write a file whose permission bits forbid it (e.g. running as root)?
    /// Probed by opening a 0444 file for writing without truncating it.
    pub fn bits_do_not_stop_writes(dir: &Path) -> bool {
        let probe = dir.join("probe-ro");
        put(&probe, b"p");
        set_mode(&probe, 0o444);
        let can = std::fs::OpenOptions::new().write(true).open(&probe).is_ok();
        set_mode(&probe, 0o644);
        std::fs::remove_file(&probe).unwrap();
        can
    }

    pub fn insert_end(doc: &mut Document, s: &str) {
        let end = doc.buffer().len_chars();
        doc.buffer_mut().insert(end, s).unwrap();
    }
}

// ------------------------------------------------- D-014 (1) decoys and symlinks at temp names

/// Regular files at every plausible temp name: left byte-identical, in place, same inode and
/// mode; save succeeds (twice); the directory holds exactly the same entries afterwards.
#[test]
fn save_leaves_regular_files_at_temp_names_untouched() {
    let root = fresh_dir("decoy_files");
    let target = root.join("f.txt");
    put(&target, b"original\n");
    for (i, name) in decoy_names("f.txt").iter().enumerate() {
        let p = root.join(name);
        put(&p, format!("decoy {i} {name}\n").as_bytes());
        set_mode(&p, if i % 2 == 0 { 0o644 } else { 0o600 });
    }
    let before = snapshot(&root);

    let mut disk = DiskFs::new(&root).unwrap();
    let path = rp("f.txt");
    let mut doc = Document::open(&disk, &path).unwrap();
    insert_end(&mut doc, "edited\n");
    assert_eq!(doc.save(&mut disk), Ok(b"original\nedited\n".len()));
    assert_only_target_changed(
        &before,
        &snapshot(&root),
        "f.txt",
        b"original\nedited\n",
        "first save",
    );
    insert_end(&mut doc, "again\n");
    doc.save(&mut disk).unwrap();
    assert_only_target_changed(
        &before,
        &snapshot(&root),
        "f.txt",
        b"original\nedited\nagain\n",
        "second save",
    );
    assert!(!doc.is_dirty());
}

/// Symlinks at every plausible temp name, pointing at victim files elsewhere: no victim is
/// written, every link stays in place with the same target, and only the document changes.
#[test]
fn save_never_writes_through_symlinks_at_temp_names() {
    let root = fresh_dir("decoy_symlinks");
    put(&root.join("d/f.txt"), b"doc\n");
    let names = decoy_names("f.txt");
    for (i, name) in names.iter().enumerate() {
        let victim = root.join(format!("victims/v{i}"));
        put(&victim, format!("victim {i}\n").as_bytes());
        std::os::unix::fs::symlink(format!("../victims/v{i}"), root.join("d").join(name)).unwrap();
    }
    let before = snapshot(&root);

    let mut disk = DiskFs::new(&root).unwrap();
    let path = rp("d/f.txt");
    let mut doc = Document::open(&disk, &path).unwrap();
    doc.buffer_mut().insert(0, "new ").unwrap();
    assert_eq!(doc.save(&mut disk), Ok(b"new doc\n".len()));
    assert_only_target_changed(&before, &snapshot(&root), "d/f.txt", b"new doc\n", "save");
    assert_eq!(disk.read(&path).unwrap(), b"new doc\n".to_vec());
}

/// Symlinks at temp names that point back at the document itself, and dangling symlinks:
/// the document is not truncated or doubled, the links survive, and no dangling link's
/// target gets created.
#[test]
fn save_ignores_self_pointing_and_dangling_symlinks_at_temp_names() {
    for (case, self_pointing) in [("self", true), ("dangling", false)] {
        let root = fresh_dir(&format!("decoy_links_{case}"));
        put(&root.join("f.txt"), b"keep me\n");
        for (i, name) in decoy_names("f.txt").iter().enumerate() {
            let to = if self_pointing {
                "f.txt".to_string()
            } else {
                format!("ghost-{i}")
            };
            std::os::unix::fs::symlink(&to, root.join(name)).unwrap();
        }
        let before = snapshot(&root);
        let mut disk = DiskFs::new(&root).unwrap();
        let path = rp("f.txt");
        let mut doc = Document::open(&disk, &path).unwrap();
        assert_eq!(doc.buffer().text(), "keep me\n");
        insert_end(&mut doc, "and this\n");
        assert_eq!(
            doc.save(&mut disk),
            Ok(b"keep me\nand this\n".len()),
            "{case}"
        );
        assert_only_target_changed(
            &before,
            &snapshot(&root),
            "f.txt",
            b"keep me\nand this\n",
            case,
        );
    }
}

/// Saving through a symlinked document, with decoys next to both the link and its target:
/// only the link's target file changes.
#[test]
fn save_through_symlink_with_decoys_changes_only_the_link_target() {
    let root = fresh_dir("symlink_doc_decoys");
    put(&root.join("real/t.txt"), b"target\n");
    std::fs::create_dir_all(root.join("work")).unwrap();
    std::os::unix::fs::symlink("../real/t.txt", root.join("work/l.txt")).unwrap();
    for dir in ["real", "work"] {
        for base in ["t.txt", "l.txt"] {
            for (i, name) in decoy_names(base).iter().enumerate() {
                let p = root.join(dir).join(name);
                if !p.exists() {
                    put(&p, format!("{dir} {base} {i}\n").as_bytes());
                }
            }
        }
    }
    let before = snapshot(&root);
    let mut disk = DiskFs::new(&root).unwrap();
    let mut doc = Document::open(&disk, &rp("work/l.txt")).unwrap();
    insert_end(&mut doc, "via link\n");
    doc.save(&mut disk).unwrap();
    assert_only_target_changed(
        &before,
        &snapshot(&root),
        "real/t.txt",
        b"target\nvia link\n",
        "through link",
    );
}

/// A document whose own name looks like a temp name of another document: saving either one
/// leaves the other untouched.
#[test]
fn documents_named_like_temp_files_do_not_clobber_each_other() {
    let root = fresh_dir("temp_named_docs");
    let names = decoy_names("f.txt");
    put(&root.join("f.txt"), b"main\n");
    for n in &names {
        put(&root.join(n), format!("{n}\n").as_bytes());
    }
    let mut disk = DiskFs::new(&root).unwrap();
    let mut expected: std::collections::BTreeMap<String, Vec<u8>> = std::fs::read_dir(&root)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (
                e.file_name().to_string_lossy().into_owned(),
                std::fs::read(e.path()).unwrap(),
            )
        })
        .collect();
    for n in std::iter::once("f.txt".to_string()).chain(names.iter().cloned()) {
        let mut doc = Document::open(&disk, &rp(&n)).unwrap();
        insert_end(&mut doc, "+");
        doc.save(&mut disk).unwrap();
        expected.get_mut(&n).unwrap().push(b'+');
        let now: std::collections::BTreeMap<String, Vec<u8>> = std::fs::read_dir(&root)
            .unwrap()
            .map(|e| {
                let e = e.unwrap();
                (
                    e.file_name().to_string_lossy().into_owned(),
                    std::fs::read(e.path()).unwrap(),
                )
            })
            .collect();
        assert_eq!(now, expected, "after saving {n}");
    }
}

// ------------------------------------------------- D-014 (2) never more readable than the target

/// After saving, the target keeps its exact mode (0600 stays 0600, etc.), and no entry in the
/// directory is group- or world-readable unless it was before.
#[test]
fn saved_file_is_never_more_readable_than_the_target() {
    for m in [0o600, 0o640, 0o604, 0o644, 0o660, 0o700, 0o755] {
        let root = fresh_dir(&format!("mode_{m:o}"));
        let target = root.join("secret.txt");
        put(&target, b"s3cret\n");
        set_mode(&target, m);
        put(&root.join("private.txt"), b"p");
        set_mode(&root.join("private.txt"), 0o600);
        let before = snapshot(&root);

        let mut disk = DiskFs::new(&root).unwrap();
        let mut doc = Document::open(&disk, &rp("secret.txt")).unwrap();
        insert_end(&mut doc, "more secret\n");
        doc.save(&mut disk).unwrap();
        assert_eq!(mode(&target), m, "mode {m:o} preserved");
        assert_only_target_changed(
            &before,
            &snapshot(&root),
            "secret.txt",
            b"s3cret\nmore secret\n",
            &format!("mode {m:o}"),
        );
        // A second (unedited) save keeps it too.
        doc.save(&mut disk).unwrap();
        assert_eq!(mode(&target), m, "mode {m:o} after second save");
    }
}

// ------------------------------------------------- D-014 (3) target the user cannot write

/// A target with write permission removed: as an ordinary user, `save` returns
/// `Io { kind: PermissionDenied }`, the bytes and the whole directory are untouched and the
/// document stays dirty. When permission bits do not stop writes for this process (root), the
/// test checks the other branch strictly: either the same refusal, or a complete save that keeps
/// the read-only mode — never a partial state or an extra file.
#[test]
fn save_to_unwritable_target_is_permission_denied_and_untouched() {
    for m in [0o444, 0o400, 0o555] {
        let root = fresh_dir(&format!("readonly_{m:o}"));
        let privileged = bits_do_not_stop_writes(&root);
        let target = root.join("ro.txt");
        put(&target, b"read only\n");
        put(&root.join("other.txt"), b"o");
        let mut disk = DiskFs::new(&root).unwrap();
        let path = rp("ro.txt");
        let mut doc = Document::open(&disk, &path).unwrap();
        set_mode(&target, m);
        insert_end(&mut doc, "attempt\n");
        let before = snapshot(&root);

        let r = doc.save(&mut disk);
        let after = snapshot(&root);
        let denied = matches!(
            &r,
            Err(VfsError::Io { path: p, kind: std::io::ErrorKind::PermissionDenied, .. }) if *p == path
        );
        if !privileged {
            assert!(
                denied,
                "mode {m:o}: expected PermissionDenied for ro.txt, got {r:?}"
            );
        }
        if denied {
            assert_eq!(
                after, before,
                "mode {m:o}: refused save changed the directory"
            );
            assert_eq!(std::fs::read(&target).unwrap(), b"read only\n".to_vec());
            assert!(doc.is_dirty(), "mode {m:o}: still dirty");
            let shown = r.unwrap_err().to_string();
            assert!(shown.contains("ro.txt"), "{shown}");
            assert!(!shown.contains(&*root.to_string_lossy()), "{shown}");
        } else {
            // Only reachable when permission bits do not stop this process.
            assert!(privileged);
            assert_eq!(r, Ok(b"read only\nattempt\n".len()), "mode {m:o}");
            assert_only_target_changed(
                &before,
                &after,
                "ro.txt",
                b"read only\nattempt\n",
                &format!("privileged, mode {m:o}"),
            );
            assert!(!doc.is_dirty());
        }
        set_mode(&target, 0o644);
    }
}

// ------------------------------------------------- D-014 (4) 255-byte names

/// File names of exactly 255 bytes (the usual NAME_MAX) open, edit and save, and leave no
/// extra entry behind — temp names derived by lengthening the name would not fit.
#[test]
fn file_with_255_byte_name_opens_edits_and_saves() {
    let ascii = format!("{}.txt", "n".repeat(251));
    let multibyte = format!("{}x", "\u{e9}".repeat(127));
    assert_eq!(ascii.len(), 255);
    assert_eq!(multibyte.len(), 255);
    for name in [ascii, multibyte] {
        let root = fresh_dir(&format!("long_name_{}", name.chars().count()));
        put(&root.join("sub").join(&name), b"long\r\n");
        put(&root.join("sub/neighbour"), b"n");
        let before = snapshot(&root);
        let rel = format!("sub/{name}");
        let mut disk = DiskFs::new(&root).unwrap();
        let path = rp(&rel);
        assert!(disk.is_file(&path));
        let mut doc = Document::open(&disk, &path).unwrap();
        assert_eq!(doc.buffer().text(), "long\r\n");
        insert_end(&mut doc, "name\r\n");
        assert_eq!(doc.save(&mut disk), Ok(b"long\r\nname\r\n".len()));
        assert!(!doc.is_dirty());
        assert_only_target_changed(&before, &snapshot(&root), &rel, b"long\r\nname\r\n", &rel);
        assert_eq!(disk.read(&path).unwrap(), b"long\r\nname\r\n".to_vec());
    }
}

// ------------------------------------------------- D-014 (5) error variants

/// `NotFound.path` is the requested path (also when the missing piece is a parent directory),
/// on both VFSs; nothing is created.
#[test]
fn not_found_names_the_requested_path() {
    let root = fresh_dir("not_found_path");
    put(&root.join("d/x"), b"x");
    let mut disk = DiskFs::new(&root).unwrap();
    let mut mem = MemoryFs::new();
    mem.insert(&rp("d/x"), b"x");
    for (label, vfs) in [
        ("memory", &mut mem as &mut dyn Vfs),
        ("disk", &mut disk as &mut dyn Vfs),
    ] {
        for p in ["missing", "nodir/f", "d/nodir/f", "a/b/c/d", "d/missing"] {
            let path = rp(p);
            assert_eq!(
                vfs.read(&path),
                Err(VfsError::NotFound { path: path.clone() }),
                "{label}: read {p}"
            );
            assert_eq!(
                Document::open(&*vfs, &path).map(|_| ()),
                Err(kiln_workspace::OpenError::Vfs(VfsError::NotFound {
                    path: path.clone()
                })),
                "{label}: open {p}"
            );
            if p.contains("nodir") || p.starts_with("a/") {
                assert_eq!(
                    vfs.write_atomic(&path, b"w"),
                    Err(VfsError::NotFound { path: path.clone() }),
                    "{label}: write {p}"
                );
                assert!(!vfs.is_file(&path), "{label}: {p} created");
            }
        }
    }
    assert_eq!(mem.paths(), vec![rp("d/x")]);
    let names: Vec<String> = snapshot(&root).into_keys().collect();
    assert_eq!(names, vec!["d".to_string(), "d/x".to_string()]);
}

/// Saving onto a path that has become a directory returns exactly `NotAFile { path }`, and
/// leaves the directory tree untouched; `write_atomic` onto a directory does the same on both
/// VFSs.
#[test]
fn save_onto_directory_is_not_a_file() {
    let root = fresh_dir("not_a_file");
    put(&root.join("d/f.txt"), b"f");
    let mut disk = DiskFs::new(&root).unwrap();
    let path = rp("d/f.txt");
    let mut doc = Document::open(&disk, &path).unwrap();
    std::fs::remove_file(root.join("d/f.txt")).unwrap();
    put(&root.join("d/f.txt/inner"), b"i");
    for n in decoy_names("f.txt") {
        put(&root.join("d").join(&n), n.as_bytes());
    }
    insert_end(&mut doc, "!");
    let before = snapshot(&root);
    assert_eq!(
        doc.save(&mut disk),
        Err(VfsError::NotAFile { path: path.clone() })
    );
    assert_eq!(
        snapshot(&root),
        before,
        "failed save left the tree unchanged"
    );
    assert!(doc.is_dirty());

    let mut mem = MemoryFs::new();
    mem.insert(&rp("d/f.txt/inner"), b"i");
    assert_eq!(
        mem.write_atomic(&path, b"w"),
        Err(VfsError::NotAFile { path: path.clone() })
    );
    assert_eq!(
        disk.write_atomic(&rp("d"), b"w"),
        Err(VfsError::NotAFile { path: rp("d") })
    );
    assert_eq!(snapshot(&root), before);
}
