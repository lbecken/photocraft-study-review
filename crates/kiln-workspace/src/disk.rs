//! On-disk VFS with atomic replace (supported platforms: Unix only, decision D-014).
//!
//! `write_atomic` writes the new content to a temporary file next to the destination and
//! renames it over the destination, so readers see either the old or the new content.
//!
//! - A save never reads, writes, replaces or deletes any file other than its target and a
//!   temporary file it created itself (D-014 (1)). The temporary file is created exclusively
//!   (`O_CREAT|O_EXCL`, which also refuses an existing symlink) under the first free name of
//!   the fixed sequence `.kiln-tmp-0`, `.kiln-tmp-1`, … `.kiln-tmp-99` in the
//!   destination's directory. The names are short and independent of the target's name length,
//!   and derived without clock or randomness. A leftover from a crashed save is never touched;
//!   it only makes the next save use the next name. Only when every candidate exists does the
//!   save fail (`AlreadyExists`).
//! - Permissions (D-014 (2)): for an existing target the temporary file is created owner-only
//!   (`0600`) and receives the target's permission bits before the rename, so the new content
//!   is never more widely readable than the old. A new file is created with the default mode
//!   `0666` reduced by the umask, as a plain create would.
//! - A target the caller cannot open for writing is refused with `PermissionDenied` and left
//!   untouched (D-014 (3)); the check opens it for writing without truncating or writing.
//! - Symlinks: the destination path is resolved through any chain of symlinks first, so the
//!   link's target file is replaced and the link itself stays in place.
//! - Documented limitations (D-014 (4)): because the content is swapped in as a new inode, a
//!   hard link to the target keeps the old content (the link is split), the file's owner and
//!   group become those of the saving process, and extended attributes and ACLs are not
//!   carried over.
//! - Durability: the temporary file is `fsync`ed before the rename (otherwise a crash after the
//!   rename could leave an empty or partial file under the destination name), and the
//!   directory is `fsync`ed after it on a best-effort basis (the replace has already happened,
//!   so a failure there is not reported as a failed save).
//! - On any failure after the temporary file was created, that file is removed.
//! - Error messages are built from the `RelPath` and the OS error description only; the host
//!   root never appears in them.

use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use crate::path::RelPath;
use crate::vfs::{Vfs, VfsError};

/// Prefix of the deterministic temporary file names used by `write_atomic`.
const TEMP_PREFIX: &str = ".kiln-tmp-";
/// How many temporary names `write_atomic` tries before giving up.
const TEMP_ATTEMPTS: usize = 100;
/// Upper bound on symlink hops when resolving a destination (as Linux `MAXSYMLINKS`).
const MAX_SYMLINK_HOPS: usize = 40;

/// On-disk VFS rooted at a directory; RelPaths resolve below `root`.
#[derive(Debug)]
pub struct DiskFs {
    root: PathBuf,
}

impl DiskFs {
    /// A VFS rooted at `root`. Err if root is not a directory.
    ///
    /// The error cannot name a `RelPath` of the root (none is valid), and must not name the
    /// host path, so it uses the placeholder path `(root)`.
    pub fn new(root: &Path) -> Result<DiskFs, VfsError> {
        let placeholder = RelPath::root_placeholder;
        match fs::metadata(root) {
            Ok(meta) if meta.is_dir() => Ok(DiskFs {
                root: root.to_path_buf(),
            }),
            Ok(_) => Err(VfsError::Io {
                path: placeholder(),
                kind: io::ErrorKind::NotADirectory,
                message: "the root is not a directory".to_owned(),
            }),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Err(VfsError::NotFound {
                path: placeholder(),
            }),
            Err(e) => Err(io_error(&placeholder(), &e)),
        }
    }

    fn resolve(&self, path: &RelPath) -> PathBuf {
        let mut full = self.root.clone();
        for segment in path.segments() {
            full.push(segment);
        }
        full
    }
}

fn io_error(path: &RelPath, e: &io::Error) -> VfsError {
    VfsError::Io {
        path: path.clone(),
        kind: e.kind(),
        message: e.to_string(),
    }
}

/// Map an error from looking up `path` itself: a missing file or a missing/non-directory
/// ancestor is `NotFound`.
fn lookup_error(path: &RelPath, e: &io::Error) -> VfsError {
    match e.kind() {
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory => {
            VfsError::NotFound { path: path.clone() }
        }
        _ => io_error(path, e),
    }
}

/// Follow `path` through symlinks to the path that is finally not a symlink (it may not
/// exist). Fails on a symlink loop.
fn resolve_symlinks(rel: &RelPath, mut path: PathBuf) -> Result<PathBuf, VfsError> {
    for _ in 0..MAX_SYMLINK_HOPS {
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                let target = fs::read_link(&path).map_err(|e| io_error(rel, &e))?;
                path = match path.parent() {
                    Some(dir) if target.is_relative() => dir.join(target),
                    _ => target,
                };
            }
            Ok(_) => return Ok(path),
            Err(e) => match e.kind() {
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory => return Ok(path),
                _ => return Err(io_error(rel, &e)),
            },
        }
    }
    Err(VfsError::Io {
        path: rel.clone(),
        kind: io::ErrorKind::Other,
        message: "too many levels of symbolic links".to_owned(),
    })
}

/// Create a new temporary file in `dir` exclusively, under the first free candidate name.
/// Nothing that already exists (file, directory or symlink) is opened or changed.
fn create_temp(dir: &Path, mode: u32) -> io::Result<(PathBuf, fs::File)> {
    for n in 0..TEMP_ATTEMPTS {
        let temp = dir.join(format!("{TEMP_PREFIX}{n}"));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(mode)
            .open(&temp)
        {
            Ok(file) => return Ok((temp, file)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!("all {TEMP_ATTEMPTS} temporary file names are taken"),
    ))
}

/// Fill the freshly created temporary `file`, give it `permissions` if given, and sync it.
fn fill_temp(
    mut file: fs::File,
    bytes: &[u8],
    permissions: Option<fs::Permissions>,
) -> io::Result<()> {
    file.write_all(bytes)?;
    if let Some(permissions) = permissions {
        file.set_permissions(permissions)?;
    }
    file.sync_all()
}

fn sync_dir(dir: &Path) {
    // Best effort: the rename has happened; a failure here does not undo it.
    if let Ok(d) = fs::File::open(dir) {
        let _ = d.sync_all();
    }
}

impl DiskFs {
    /// `write_atomic` with the final rename injectable, so tests can make it fail.
    fn write_atomic_with(
        &mut self,
        path: &RelPath,
        bytes: &[u8],
        rename: impl FnOnce(&Path, &Path) -> io::Result<()>,
    ) -> Result<(), VfsError> {
        let dest = resolve_symlinks(path, self.resolve(path))?;
        let Some(dir) = dest.parent() else {
            return Err(VfsError::Io {
                path: path.clone(),
                kind: io::ErrorKind::InvalidInput,
                message: "the destination has no parent directory".to_owned(),
            });
        };
        match fs::metadata(dir) {
            Ok(meta) if meta.is_dir() => {}
            Ok(_) => return Err(VfsError::NotFound { path: path.clone() }),
            Err(e) => return Err(lookup_error(path, &e)),
        }
        let permissions = match fs::metadata(&dest) {
            Ok(meta) if meta.is_dir() => return Err(VfsError::NotAFile { path: path.clone() }),
            Ok(meta) => {
                if meta.is_file() {
                    // Respect write protection: rename alone would only need the directory to
                    // be writable. Opening for write neither truncates nor writes.
                    fs::OpenOptions::new()
                        .write(true)
                        .open(&dest)
                        .map_err(|e| io_error(path, &e))?;
                }
                Some(meta.permissions())
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(io_error(path, &e)),
        };
        // Owner-only until the target's bits are applied; a new file gets the default mode.
        let mode = if permissions.is_some() { 0o600 } else { 0o666 };
        let (temp, file) = create_temp(dir, mode).map_err(|e| io_error(path, &e))?;

        if let Err(e) = fill_temp(file, bytes, permissions) {
            let _ = fs::remove_file(&temp);
            return Err(io_error(path, &e));
        }
        if let Err(e) = rename(&temp, &dest) {
            let _ = fs::remove_file(&temp);
            return Err(match e.kind() {
                io::ErrorKind::IsADirectory | io::ErrorKind::DirectoryNotEmpty => {
                    VfsError::NotAFile { path: path.clone() }
                }
                _ => io_error(path, &e),
            });
        }
        sync_dir(dir);
        Ok(())
    }
}

impl Vfs for DiskFs {
    fn read(&self, path: &RelPath) -> Result<Vec<u8>, VfsError> {
        let full = self.resolve(path);
        let meta = fs::metadata(&full).map_err(|e| lookup_error(path, &e))?;
        if meta.is_dir() {
            return Err(VfsError::NotAFile { path: path.clone() });
        }
        fs::read(&full).map_err(|e| lookup_error(path, &e))
    }

    fn write_atomic(&mut self, path: &RelPath, bytes: &[u8]) -> Result<(), VfsError> {
        self.write_atomic_with(path, bytes, |from, to| fs::rename(from, to))
    }

    fn is_file(&self, path: &RelPath) -> bool {
        fs::metadata(self.resolve(path)).is_ok_and(|m| m.is_file())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryFs;

    fn p(s: &str) -> RelPath {
        RelPath::new(s).unwrap()
    }

    /// A fresh, empty directory under the system temp dir, unique to this test.
    fn fresh_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kiln-workspace-unit-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn listing(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn new_rejects_missing_root_and_file_root_without_naming_them() {
        let dir = fresh_dir("new");
        let missing = dir.join("missing");
        let err = DiskFs::new(&missing).unwrap_err();
        assert!(matches!(err, VfsError::NotFound { .. }));
        assert!(!err.to_string().contains(dir.to_string_lossy().as_ref()));
        fs::write(dir.join("f"), b"").unwrap();
        let err = DiskFs::new(&dir.join("f")).unwrap_err();
        assert!(matches!(
            err,
            VfsError::Io {
                kind: io::ErrorKind::NotADirectory,
                ..
            }
        ));
        assert!(DiskFs::new(&dir).is_ok());
    }

    #[test]
    fn write_read_and_errors() {
        let dir = fresh_dir("basic");
        let mut fs_ = DiskFs::new(&dir).unwrap();
        fs_.write_atomic(&p("a.txt"), b"one").unwrap();
        assert_eq!(fs_.read(&p("a.txt")).unwrap(), b"one");
        fs_.write_atomic(&p("a.txt"), b"two\r\n").unwrap();
        assert_eq!(fs_.read(&p("a.txt")).unwrap(), b"two\r\n");
        assert!(fs_.is_file(&p("a.txt")));
        assert_eq!(listing(&dir), ["a.txt"]);

        assert_eq!(
            fs_.read(&p("nope")),
            Err(VfsError::NotFound { path: p("nope") })
        );
        assert_eq!(
            fs_.read(&p("a.txt/x")),
            Err(VfsError::NotFound { path: p("a.txt/x") })
        );
        assert_eq!(
            fs_.write_atomic(&p("missing/x"), b""),
            Err(VfsError::NotFound {
                path: p("missing/x")
            })
        );
        assert_eq!(
            fs_.write_atomic(&p("a.txt/x"), b""),
            Err(VfsError::NotFound { path: p("a.txt/x") })
        );
        fs::create_dir(dir.join("d")).unwrap();
        assert_eq!(fs_.read(&p("d")), Err(VfsError::NotAFile { path: p("d") }));
        assert_eq!(
            fs_.write_atomic(&p("d"), b""),
            Err(VfsError::NotAFile { path: p("d") })
        );
        assert!(!fs_.is_file(&p("d")));
        fs_.write_atomic(&p("d/n.rs"), b"fn main() {}").unwrap();
        assert_eq!(listing(&dir), ["a.txt", "d"]);
        assert_eq!(listing(&dir.join("d")), ["n.rs"]);
    }

    fn mode(path: &Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path).unwrap().permissions().mode() & 0o7777
    }

    fn set_mode(path: &Path, mode: u32) {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    }

    #[test]
    fn leftover_at_temp_name_is_never_touched() {
        let dir = fresh_dir("stale");
        let mut fs_ = DiskFs::new(&dir).unwrap();
        fs::write(dir.join("a"), b"old").unwrap();
        // Longer than the new content, and read-only: neither may matter or be changed.
        fs::write(dir.join(".kiln-tmp-0"), b"leftover from a crash, long").unwrap();
        set_mode(&dir.join(".kiln-tmp-0"), 0o444);
        fs_.write_atomic(&p("a"), b"new").unwrap();
        fs_.write_atomic(&p("a"), b"newer").unwrap();
        assert_eq!(fs::read(dir.join("a")).unwrap(), b"newer");
        assert_eq!(
            fs::read(dir.join(".kiln-tmp-0")).unwrap(),
            b"leftover from a crash, long"
        );
        assert_eq!(mode(&dir.join(".kiln-tmp-0")), 0o444);
        assert_eq!(listing(&dir), [".kiln-tmp-0", "a"]);
    }

    #[test]
    fn symlink_at_temp_name_is_not_followed() {
        use std::os::unix::fs::symlink;
        let dir = fresh_dir("tmp-symlink");
        let outside = fresh_dir("tmp-symlink-outside");
        fs::write(outside.join("precious"), b"precious").unwrap();
        fs::write(dir.join("f"), b"old").unwrap();
        symlink(outside.join("precious"), dir.join(".kiln-tmp-0")).unwrap();
        // A dangling symlink must not be followed either (O_EXCL refuses it).
        symlink(outside.join("nonexistent"), dir.join(".kiln-tmp-1")).unwrap();
        let mut fs_ = DiskFs::new(&dir).unwrap();
        fs_.write_atomic(&p("f"), b"EDIT").unwrap();
        assert_eq!(fs::read(outside.join("precious")).unwrap(), b"precious");
        assert!(!outside.join("nonexistent").exists());
        assert_eq!(listing(&outside), ["precious"]);
        let f = fs::symlink_metadata(dir.join("f")).unwrap();
        assert!(f.file_type().is_file());
        assert_eq!(fs::read(dir.join("f")).unwrap(), b"EDIT");
        for link in [".kiln-tmp-0", ".kiln-tmp-1"] {
            assert!(
                fs::symlink_metadata(dir.join(link))
                    .unwrap()
                    .file_type()
                    .is_symlink()
            );
        }
        assert_eq!(listing(&dir), [".kiln-tmp-0", ".kiln-tmp-1", "f"]);
    }

    #[test]
    fn all_temp_names_taken_fails_without_changes_then_recovers() {
        let dir = fresh_dir("exhausted");
        fs::write(dir.join("f"), b"old").unwrap();
        for n in 0..TEMP_ATTEMPTS {
            fs::write(dir.join(format!("{TEMP_PREFIX}{n}")), b"user").unwrap();
        }
        let before = listing(&dir);
        let mut fs_ = DiskFs::new(&dir).unwrap();
        let err = fs_.write_atomic(&p("f"), b"new").unwrap_err();
        assert!(matches!(
            err,
            VfsError::Io {
                kind: io::ErrorKind::AlreadyExists,
                ..
            }
        ));
        assert_eq!(listing(&dir), before);
        assert_eq!(fs::read(dir.join("f")).unwrap(), b"old");
        for n in 0..TEMP_ATTEMPTS {
            assert_eq!(
                fs::read(dir.join(format!("{TEMP_PREFIX}{n}"))).unwrap(),
                b"user"
            );
        }
        fs::remove_file(dir.join(format!("{TEMP_PREFIX}57"))).unwrap();
        fs_.write_atomic(&p("f"), b"new").unwrap();
        assert_eq!(fs::read(dir.join("f")).unwrap(), b"new");
        assert_eq!(listing(&dir).len(), before.len() - 1);
    }

    #[test]
    fn longest_file_names_can_be_saved() {
        let dir = fresh_dir("long");
        let mut fs_ = DiskFs::new(&dir).unwrap();
        for len in [245, 246, 250, 255] {
            let name = "n".repeat(len);
            fs_.write_atomic(&p(&name), b"one").unwrap();
            fs_.write_atomic(&p(&name), b"two").unwrap();
            assert_eq!(fs_.read(&p(&name)).unwrap(), b"two");
        }
        assert_eq!(listing(&dir).len(), 4);
    }

    #[test]
    fn temp_file_is_created_owner_only_and_gets_target_mode_before_rename() {
        let dir = fresh_dir("perm");
        // The temporary file itself is owner-only from its creation.
        let (temp, _file) = create_temp(&dir, 0o600).unwrap();
        assert_eq!(mode(&temp), 0o600);
        fs::remove_file(&temp).unwrap();

        let mut fs_ = DiskFs::new(&dir).unwrap();
        for target_mode in [0o600, 0o640, 0o644, 0o755] {
            fs::write(dir.join("f"), b"old").unwrap();
            set_mode(&dir.join("f"), target_mode);
            let mut seen = None;
            fs_.write_atomic_with(&p("f"), b"secret", |from, to| {
                // Just before the rename: content complete, mode already the target's.
                seen = Some((mode(from), fs::read(from).unwrap()));
                fs::rename(from, to)
            })
            .unwrap();
            assert_eq!(seen, Some((target_mode, b"secret".to_vec())));
            assert_eq!(mode(&dir.join("f")), target_mode);
            assert_eq!(listing(&dir), ["f"]);
        }
    }

    #[test]
    fn failed_rename_removes_own_temp_and_keeps_target() {
        let dir = fresh_dir("rename-fail");
        fs::write(dir.join("f"), b"old").unwrap();
        fs::write(dir.join(".kiln-tmp-0"), b"not mine").unwrap();
        let mut fs_ = DiskFs::new(&dir).unwrap();
        let mut used = None;
        let err = fs_
            .write_atomic_with(&p("f"), b"new", |from, _to| {
                used = Some(from.to_path_buf());
                Err(io::Error::other("injected rename failure"))
            })
            .unwrap_err();
        assert_eq!(
            err,
            VfsError::Io {
                path: p("f"),
                kind: io::ErrorKind::Other,
                message: "injected rename failure".to_owned()
            }
        );
        assert_eq!(used, Some(dir.join(".kiln-tmp-1")));
        assert_eq!(listing(&dir), [".kiln-tmp-0", "f"]);
        assert_eq!(fs::read(dir.join("f")).unwrap(), b"old");
        assert_eq!(fs::read(dir.join(".kiln-tmp-0")).unwrap(), b"not mine");
    }

    #[test]
    fn read_only_target_is_refused_unless_root() {
        use std::os::unix::fs::MetadataExt;
        let dir = fresh_dir("readonly");
        fs::write(dir.join("f"), b"old").unwrap();
        // The owner of a file we just created is our effective uid.
        let euid = fs::metadata(dir.join("f")).unwrap().uid();
        set_mode(&dir.join("f"), 0o444);
        let ino = fs::metadata(dir.join("f")).unwrap().ino();
        let mut fs_ = DiskFs::new(&dir).unwrap();
        let result = fs_.write_atomic(&p("f"), b"new");
        if euid == 0 {
            // Root may write any file, so no file is write-protected against it.
            assert_eq!(result, Ok(()));
            assert_eq!(fs::read(dir.join("f")).unwrap(), b"new");
        } else {
            assert!(matches!(
                result,
                Err(VfsError::Io {
                    kind: io::ErrorKind::PermissionDenied,
                    ..
                })
            ));
            assert_eq!(fs::read(dir.join("f")).unwrap(), b"old");
            assert_eq!(fs::metadata(dir.join("f")).unwrap().ino(), ino);
        }
        assert_eq!(mode(&dir.join("f")), 0o444);
        assert_eq!(listing(&dir), ["f"]);
    }

    #[test]
    fn new_file_gets_default_mode() {
        let dir = fresh_dir("newmode");
        fs::write(dir.join("reference"), b"").unwrap();
        let mut fs_ = DiskFs::new(&dir).unwrap();
        fs_.write_atomic(&p("new"), b"x").unwrap();
        assert_eq!(mode(&dir.join("new")), mode(&dir.join("reference")));
    }

    #[test]
    fn replace_swaps_inode_keeps_permissions_and_follows_symlinks() {
        use std::os::unix::fs::{MetadataExt, symlink};
        let dir = fresh_dir("replace");
        let mut fs_ = DiskFs::new(&dir).unwrap();
        fs::write(dir.join("f"), b"old").unwrap();
        set_mode(&dir.join("f"), 0o640);
        let ino = fs::metadata(dir.join("f")).unwrap().ino();
        fs_.write_atomic(&p("f"), b"new").unwrap();
        assert_ne!(fs::metadata(dir.join("f")).unwrap().ino(), ino);
        assert_eq!(mode(&dir.join("f")), 0o640);
        assert_eq!(fs::read(dir.join("f")).unwrap(), b"new");

        fs::create_dir(dir.join("t")).unwrap();
        fs::write(dir.join("t/target"), b"old").unwrap();
        symlink("t/target", dir.join("link")).unwrap();
        symlink("link", dir.join("link2")).unwrap();
        fs_.write_atomic(&p("link2"), b"via link").unwrap();
        assert!(
            fs::symlink_metadata(dir.join("link"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(
            fs::symlink_metadata(dir.join("link2"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read(dir.join("t/target")).unwrap(), b"via link");
        assert_eq!(listing(&dir), ["f", "link", "link2", "t"]);
        assert_eq!(listing(&dir.join("t")), ["target"]);

        symlink("loop", dir.join("loop")).unwrap();
        assert!(matches!(
            fs_.write_atomic(&p("loop"), b"x"),
            Err(VfsError::Io { .. })
        ));
    }

    #[test]
    fn errors_are_root_independent() {
        let run = |name: &str| {
            let dir = fresh_dir(name);
            fs::create_dir(dir.join("d")).unwrap();
            let mut fs_ = DiskFs::new(&dir).unwrap();
            let results = vec![
                fs_.read(&p("x")).map_err(|e| e.to_string()),
                fs_.read(&p("d")).map_err(|e| e.to_string()),
                fs_.write_atomic(&p("q/x"), b"1")
                    .map(|_| Vec::new())
                    .map_err(|e| e.to_string()),
            ];
            for r in &results {
                if let Err(msg) = r {
                    assert!(!msg.contains(dir.to_string_lossy().as_ref()));
                }
            }
            results
        };
        assert_eq!(run("root-a"), run("root-b"));
    }

    /// Fixed-seed differential test: MemoryFs is the reference model of DiskFs.
    #[test]
    fn matches_memory_model_on_random_operations() {
        let dir = fresh_dir("diff");
        let mut disk = DiskFs::new(&dir).unwrap();
        let mut mem = MemoryFs::new();
        // Directories must exist in both: on disk explicitly, in memory via a file below.
        fs::create_dir_all(dir.join("d/e")).unwrap();
        fs::write(dir.join("d/e/keep"), b"k").unwrap();
        mem.insert(&p("d/e/keep"), b"k");

        let paths = [
            "a", "b", "d", "d/a", "d/e", "d/e/a", "d/e/keep", "m/a", "a/x",
        ];
        let mut seed: u64 = 0xd15c;
        let mut next = move |bound: usize| {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((seed >> 33) as usize) % bound.max(1)
        };
        for step in 0..400 {
            let path = p(paths[next(paths.len())]);
            match next(3) {
                0 => {
                    let bytes: Vec<u8> = (0..next(6)).map(|i| (step + i) as u8).collect();
                    assert_eq!(
                        disk.write_atomic(&path, &bytes),
                        mem.write_atomic(&path, &bytes),
                        "write {path}"
                    );
                }
                1 => assert_eq!(disk.read(&path), mem.read(&path), "read {path}"),
                _ => assert_eq!(disk.is_file(&path), mem.is_file(&path), "is_file {path}"),
            }
        }
        for path in mem.paths() {
            assert_eq!(disk.read(&path).unwrap(), mem.get(&path).unwrap());
        }
        assert!(listing(&dir).iter().all(|n| !n.starts_with(TEMP_PREFIX)));
    }
}
