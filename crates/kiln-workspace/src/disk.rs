//! On-disk VFS with atomic replace.
//!
//! `write_atomic` writes the new content to a temporary file next to the destination and
//! renames it over the destination, so readers see either the old or the new content.
//!
//! - Temporary name: `.<file name>.kiln-tmp` in the destination's directory. It is derived
//!   only from the destination name (no clock, no randomness), is always longer than the
//!   destination name so it never equals it, and is opened with truncation, so a leftover from
//!   a crashed save is simply overwritten by the next save of the same file. Consequence: a
//!   user file that happens to have exactly that name is overwritten and removed by a save.
//! - Symlinks: the destination path is resolved through any chain of symlinks first, so the
//!   link's target file is replaced and the link itself stays in place.
//! - Permissions of an existing destination are copied to the temporary file before the
//!   rename. Ownership and other metadata are not preserved.
//! - Durability: the temporary file is `fsync`ed before the rename (otherwise a crash after the
//!   rename could leave an empty or partial file under the destination name, defeating the
//!   point), and on Unix the directory is `fsync`ed after it on a best-effort basis (the
//!   replace has already happened, so a failure there is not reported as a failed save).
//! - Error messages are built from the `RelPath` and the OS error description only; the host
//!   root never appears in them.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::path::RelPath;
use crate::vfs::{Vfs, VfsError};

/// Suffix of the deterministic temporary file used by `write_atomic`.
const TEMP_SUFFIX: &str = ".kiln-tmp";
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

/// Write `bytes` to `temp` (created or truncated), with `permissions` if given, and sync it.
fn write_temp(temp: &Path, bytes: &[u8], permissions: Option<fs::Permissions>) -> io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(temp)?;
    file.write_all(bytes)?;
    if let Some(permissions) = permissions {
        file.set_permissions(permissions)?;
    }
    file.sync_all()
}

#[cfg(unix)]
fn sync_dir(dir: &Path) {
    // Best effort: the rename has happened; a failure here does not undo it.
    if let Ok(d) = fs::File::open(dir) {
        let _ = d.sync_all();
    }
}

#[cfg(not(unix))]
fn sync_dir(_dir: &Path) {}

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
        let dest = resolve_symlinks(path, self.resolve(path))?;
        let (Some(dir), Some(name)) = (dest.parent(), dest.file_name()) else {
            return Err(VfsError::Io {
                path: path.clone(),
                kind: io::ErrorKind::InvalidInput,
                message: "the destination has no file name".to_owned(),
            });
        };
        match fs::metadata(dir) {
            Ok(meta) if meta.is_dir() => {}
            Ok(_) => return Err(VfsError::NotFound { path: path.clone() }),
            Err(e) => return Err(lookup_error(path, &e)),
        }
        let permissions = match fs::metadata(&dest) {
            Ok(meta) if meta.is_dir() => return Err(VfsError::NotAFile { path: path.clone() }),
            Ok(meta) => Some(meta.permissions()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(io_error(path, &e)),
        };

        let mut temp_name = std::ffi::OsString::from(".");
        temp_name.push(name);
        temp_name.push(TEMP_SUFFIX);
        let temp = dir.join(temp_name);

        if let Err(e) = write_temp(&temp, bytes, permissions) {
            let _ = fs::remove_file(&temp);
            return Err(io_error(path, &e));
        }
        if let Err(e) = fs::rename(&temp, &dest) {
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

    #[test]
    fn stale_temp_file_does_not_block_save() {
        let dir = fresh_dir("stale");
        let mut fs_ = DiskFs::new(&dir).unwrap();
        fs::write(dir.join(".a.kiln-tmp"), b"leftover from a crash").unwrap();
        fs_.write_atomic(&p("a"), b"new").unwrap();
        assert_eq!(fs::read(dir.join("a")).unwrap(), b"new");
        assert_eq!(listing(&dir), ["a"]);
    }

    /// Platform helpers so the test below runs everywhere (no cfg on the test itself).
    #[cfg(unix)]
    mod platform {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        use std::path::Path;

        /// Give `path` non-default permission bits and return them.
        pub fn set_distinct_permissions(path: &Path) -> std::fs::Permissions {
            let perms = std::fs::Permissions::from_mode(0o640);
            std::fs::set_permissions(path, perms.clone()).unwrap();
            std::fs::metadata(path).unwrap().permissions()
        }

        pub fn file_id(path: &Path) -> Option<u64> {
            Some(std::fs::metadata(path).unwrap().ino())
        }

        pub fn symlink(target: &str, link: &Path) -> std::io::Result<()> {
            std::os::unix::fs::symlink(target, link)
        }
    }

    #[cfg(not(unix))]
    mod platform {
        use std::path::Path;

        pub fn set_distinct_permissions(path: &Path) -> std::fs::Permissions {
            std::fs::metadata(path).unwrap().permissions()
        }

        pub fn file_id(_path: &Path) -> Option<u64> {
            None
        }

        #[cfg(windows)]
        pub fn symlink(target: &str, link: &Path) -> std::io::Result<()> {
            std::os::windows::fs::symlink_file(target, link)
        }

        #[cfg(not(windows))]
        pub fn symlink(_target: &str, _link: &Path) -> std::io::Result<()> {
            Err(std::io::ErrorKind::Unsupported.into())
        }
    }

    #[test]
    fn replace_swaps_inode_keeps_permissions_and_follows_symlinks() {
        use platform::symlink;
        let dir = fresh_dir("replace");
        let mut fs_ = DiskFs::new(&dir).unwrap();
        fs::write(dir.join("f"), b"old").unwrap();
        let perms = platform::set_distinct_permissions(&dir.join("f"));
        let id = platform::file_id(&dir.join("f"));
        fs_.write_atomic(&p("f"), b"new").unwrap();
        // Inode numbers are a Unix notion (AC-10 says "(Unix)").
        if cfg!(unix) {
            assert_ne!(platform::file_id(&dir.join("f")), id);
        }
        assert_eq!(fs::metadata(dir.join("f")).unwrap().permissions(), perms);
        assert_eq!(fs::read(dir.join("f")).unwrap(), b"new");

        fs::create_dir(dir.join("t")).unwrap();
        fs::write(dir.join("t/target"), b"old").unwrap();
        symlink("t/target", &dir.join("link")).unwrap();
        symlink("link", &dir.join("link2")).unwrap();
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

        symlink("loop", &dir.join("loop")).unwrap();
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
        assert!(listing(&dir).iter().all(|n| !n.ends_with(TEMP_SUFFIX)));
    }
}
