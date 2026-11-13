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
//!
//! Confinement (T-008, `docs/protocol/paths.md`):
//! - The root is resolved once, at construction (`fs::canonicalize`; a root given as a symlink
//!   to a directory is that directory).
//! - Every `read`, `write_atomic` and `is_file` first computes the *location* of its path by
//!   walking it from the root one component at a time, as the kernel does: each symlink is
//!   read and its target's components are walked in its place (an absolute target from `/`),
//!   and `..` pops the directory actually reached so far (physically, never lexically). The
//!   walk ends at an existing entry that is not a symlink, or at the first missing (or
//!   non-directory) component.
//! - The location is inside the root iff it equals the root or lies below it, compared
//!   component by component (`Path::starts_with` on symlink-free absolute paths, so a sibling
//!   `<root>x` is not inside). Only the final location decides. A location outside is refused
//!   with `OutsideRoot` before anything there is opened, created or removed; a failure to walk
//!   further once outside is also `OutsideRoot`, so nothing about the outside is reported.
//! - The file type is taken from `lstat` of the location, without opening it, so a FIFO,
//!   socket, device or directory is refused with `NotAFile` and never opened (opening a FIFO
//!   for reading blocks until a writer appears). Only then is a regular file opened, through
//!   the symlink-free location path; a save creates its temporary file in the location's
//!   directory and renames it onto the location.
//! - Precedence: `OutsideRoot`, then `NotFound`, then `NotAFile`, then `Io`. Nothing is cached:
//!   every call walks again, so what an earlier `read` saw does not authorise a later save.
//! - Accepted limitation (Decisions D-3 of T-008): the walk and the following open or rename
//!   are separate system calls. Another process that changes the tree *during* one call (for
//!   example swaps a component for a symlink, or the file for a FIFO, between the walk and the
//!   open) can make that call reach outside the root or block. A hard link inside the root to a
//!   file outside cannot be detected by path.
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

use std::collections::VecDeque;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Component, Path, PathBuf};

use crate::path::RelPath;
use crate::vfs::{Vfs, VfsError};

/// Prefix of the deterministic temporary file names used by `write_atomic`.
const TEMP_PREFIX: &str = ".kiln-tmp-";
/// How many temporary names `write_atomic` tries before giving up.
const TEMP_ATTEMPTS: usize = 100;
/// Upper bound on symlink hops when resolving a path (as Linux `MAXSYMLINKS`).
const MAX_SYMLINK_HOPS: usize = 40;

/// On-disk VFS rooted at a directory; RelPaths resolve below `root`, and every location they
/// reach must stay inside it (module docs).
#[derive(Debug)]
pub struct DiskFs {
    /// The root as resolved at construction: absolute, without symlinks, `.` or `..`.
    root: PathBuf,
}

/// One step of the walk to a location.
#[derive(Debug)]
enum Step {
    /// Restart from `/` (an absolute symlink target).
    Root,
    /// `..`: the parent of the directory reached so far.
    Parent,
    /// An entry of the directory reached so far.
    Name(OsString),
}

/// What the walk found at a location.
#[derive(Debug)]
enum Found {
    /// A regular file.
    File(fs::Metadata),
    /// A directory.
    Dir,
    /// Another non-regular entry: FIFO, socket, character or block device.
    Special,
    /// Nothing exists there, but its parent directory does (a save may create it).
    Missing,
    /// A component before the last does not exist or is not a directory.
    NoParent,
}

/// The location of a request path inside the root.
#[derive(Debug)]
struct Location {
    /// The location: absolute, without symlinks (unless one was swapped in since the walk).
    path: PathBuf,
    /// The directory the location's entry lives in (`Missing` and `File`).
    dir: PathBuf,
    found: Found,
}

impl DiskFs {
    /// A VFS rooted at `root`. Err if root is not a directory.
    ///
    /// The error cannot name a `RelPath` of the root (none is valid), and must not name the
    /// host path, so it uses the placeholder path `(root)`. The stored root is `root` fully
    /// resolved (a symlink to a directory is replaced by that directory).
    pub fn new(root: &Path) -> Result<DiskFs, VfsError> {
        let placeholder = RelPath::root_placeholder;
        let root_error = |e: io::Error| match e.kind() {
            io::ErrorKind::NotFound => VfsError::NotFound {
                path: placeholder(),
            },
            _ => io_error(&placeholder(), &e),
        };
        match fs::metadata(root) {
            Ok(meta) if meta.is_dir() => Ok(DiskFs {
                root: fs::canonicalize(root).map_err(root_error)?,
            }),
            Ok(_) => Err(VfsError::Io {
                path: placeholder(),
                kind: io::ErrorKind::NotADirectory,
                message: "the root is not a directory".to_owned(),
            }),
            Err(e) => Err(root_error(e)),
        }
    }

    /// Whether `location` (absolute, symlink-free) is the root or below it, component by
    /// component.
    fn inside(&self, location: &Path) -> bool {
        location.starts_with(&self.root)
    }

    /// An error met while walking from `dir`: `OutsideRoot` once the walk is outside the root
    /// (so nothing about the outside is reported), otherwise `make()`.
    fn walk_error(&self, rel: &RelPath, dir: &Path, make: impl FnOnce() -> VfsError) -> VfsError {
        if self.inside(dir) {
            make()
        } else {
            VfsError::OutsideRoot { path: rel.clone() }
        }
    }

    /// Walk `rel` from the root to its location, following every symlink physically (module
    /// docs). `Err(OutsideRoot)` if the location is outside the root; nothing is opened.
    fn locate(&self, rel: &RelPath) -> Result<Location, VfsError> {
        let mut pending: VecDeque<Step> = rel
            .segments()
            .map(|s| Step::Name(OsString::from(s)))
            .collect();
        // The directory reached so far: absolute, without symlinks, `.` or `..`.
        let mut dir = self.root.clone();
        let mut hops = 0;
        let (path, found) = loop {
            let Some(step) = pending.pop_front() else {
                // Every step consumed and the last one ended on a directory.
                break (dir.clone(), Found::Dir);
            };
            let name = match step {
                Step::Root => {
                    dir = PathBuf::from("/");
                    continue;
                }
                Step::Parent => {
                    dir.pop();
                    continue;
                }
                Step::Name(name) => name,
            };
            let candidate = dir.join(&name);
            match fs::symlink_metadata(&candidate) {
                Ok(meta) if meta.file_type().is_symlink() => {
                    hops += 1;
                    if hops > MAX_SYMLINK_HOPS {
                        return Err(self.walk_error(rel, &dir, || VfsError::Io {
                            path: rel.clone(),
                            kind: io::ErrorKind::Other,
                            message: "too many levels of symbolic links".to_owned(),
                        }));
                    }
                    let target = fs::read_link(&candidate)
                        .map_err(|e| self.walk_error(rel, &dir, || io_error(rel, &e)))?;
                    for component in target.components().rev() {
                        match component {
                            Component::RootDir | Component::Prefix(_) => {
                                pending.push_front(Step::Root)
                            }
                            Component::CurDir => {}
                            Component::ParentDir => pending.push_front(Step::Parent),
                            Component::Normal(n) => pending.push_front(Step::Name(n.to_owned())),
                        }
                    }
                }
                Ok(meta) if meta.is_dir() => dir = candidate,
                Ok(meta) if pending.is_empty() => {
                    let found = if meta.is_file() {
                        Found::File(meta)
                    } else {
                        Found::Special
                    };
                    break (candidate, found);
                }
                // A non-directory before the last component.
                Ok(_) => break (candidate, Found::NoParent),
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                    ) =>
                {
                    let found = if pending.is_empty() {
                        Found::Missing
                    } else {
                        Found::NoParent
                    };
                    break (candidate, found);
                }
                Err(e) => return Err(self.walk_error(rel, &dir, || io_error(rel, &e))),
            }
        };
        if !self.inside(&path) {
            return Err(VfsError::OutsideRoot { path: rel.clone() });
        }
        Ok(Location { path, dir, found })
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
    /// `write_atomic` with two test seams: `created` runs on the temporary file right after its
    /// creation (to observe it, or to inject a failure there), and `rename` replaces the final
    /// rename. `write_atomic` passes a no-op and `fs::rename`.
    fn write_atomic_with(
        &mut self,
        path: &RelPath,
        bytes: &[u8],
        created: impl FnOnce(&Path) -> io::Result<()>,
        rename: impl FnOnce(&Path, &Path) -> io::Result<()>,
    ) -> Result<(), VfsError> {
        // Outside the root: refused before anything is created (`locate`).
        let Location {
            path: dest,
            dir,
            found,
        } = self.locate(path)?;
        let dir = dir.as_path();
        let permissions = match found {
            Found::NoParent => return Err(VfsError::NotFound { path: path.clone() }),
            // Never opened: opening a FIFO for writing blocks until a reader appears.
            Found::Dir | Found::Special => return Err(VfsError::NotAFile { path: path.clone() }),
            Found::Missing => None,
            Found::File(meta) => {
                // Respect write protection: rename alone would only need the directory to be
                // writable. Opening for write neither truncates nor writes.
                fs::OpenOptions::new()
                    .write(true)
                    .open(&dest)
                    .map_err(|e| io_error(path, &e))?;
                Some(meta.permissions())
            }
        };
        // Owner-only until the target's bits are applied; a new file gets the default mode.
        let mode = if permissions.is_some() { 0o600 } else { 0o666 };
        let (temp, file) = create_temp(dir, mode).map_err(|e| io_error(path, &e))?;

        if let Err(e) = created(&temp).and_then(|()| fill_temp(file, bytes, permissions)) {
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
        let location = self.locate(path)?;
        match location.found {
            Found::Missing | Found::NoParent => Err(VfsError::NotFound { path: path.clone() }),
            // Never opened: opening a FIFO for reading blocks until a writer appears.
            Found::Dir | Found::Special => Err(VfsError::NotAFile { path: path.clone() }),
            Found::File(_) => fs::read(&location.path).map_err(|e| lookup_error(path, &e)),
        }
    }

    fn write_atomic(&mut self, path: &RelPath, bytes: &[u8]) -> Result<(), VfsError> {
        self.write_atomic_with(path, bytes, |_| Ok(()), |from, to| fs::rename(from, to))
    }

    fn is_file(&self, path: &RelPath) -> bool {
        matches!(
            self.locate(path),
            Ok(Location {
                found: Found::File(_),
                ..
            })
        )
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
            fs_.write_atomic_with(
                &p("f"),
                b"secret",
                |_| Ok(()),
                |from, to| {
                    // Just before the rename: content complete, mode already the target's.
                    seen = Some((mode(from), fs::read(from).unwrap()));
                    fs::rename(from, to)
                },
            )
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
            .write_atomic_with(
                &p("f"),
                b"new",
                |_| Ok(()),
                |from, _to| {
                    used = Some(from.to_path_buf());
                    Err(io::Error::other("injected rename failure"))
                },
            )
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

    /// Kills "temp created with the default mode instead of owner-only" (review r2 TG-4): the
    /// mode is observed on the real save path at the moment the temp file is created, before
    /// any byte is written to it.
    #[test]
    fn save_creates_temp_owner_only_for_an_existing_target() {
        let dir = fresh_dir("temp-mode-at-create");
        let mut fs_ = DiskFs::new(&dir).unwrap();
        for target_mode in [0o600, 0o644, 0o666, 0o755] {
            fs::write(dir.join("f"), b"old").unwrap();
            set_mode(&dir.join("f"), target_mode);
            let mut at_create = None;
            fs_.write_atomic_with(
                &p("f"),
                b"secret",
                |temp| {
                    at_create = Some((mode(temp), fs::read(temp).unwrap()));
                    Ok(())
                },
                |from, to| fs::rename(from, to),
            )
            .unwrap();
            assert_eq!(
                at_create,
                Some((0o600, Vec::new())),
                "target {target_mode:o}"
            );
            assert_eq!(mode(&dir.join("f")), target_mode);
        }
    }

    /// Everything a failed save must leave as it was: the listing, and the target's bytes and
    /// mode.
    fn snapshot(dir: &Path) -> (Vec<String>, Vec<u8>, u32) {
        (
            listing(dir),
            fs::read(dir.join("f")).unwrap(),
            mode(&dir.join("f")),
        )
    }

    /// Kills "the target is emptied when a save fails after the writability check" (review r2
    /// TG-5), with three failures that can be provoked as root, all after that check.
    #[test]
    fn failed_save_after_writability_check_leaves_target_and_listing_unchanged() {
        let setup = |name: &str| {
            let dir = fresh_dir(name);
            fs::write(dir.join("f"), b"original content").unwrap();
            set_mode(&dir.join("f"), 0o640);
            dir
        };

        // 1. All temporary names taken (public API).
        let dir = setup("fail-exhausted");
        for n in 0..TEMP_ATTEMPTS {
            fs::write(dir.join(format!("{TEMP_PREFIX}{n}")), b"").unwrap();
        }
        let before = snapshot(&dir);
        let mut fs_ = DiskFs::new(&dir).unwrap();
        assert!(fs_.write_atomic(&p("f"), b"x").is_err());
        assert_eq!(snapshot(&dir), before);

        // 2. Failure right after the temp file was created.
        let dir = setup("fail-after-create");
        let before = snapshot(&dir);
        let mut fs_ = DiskFs::new(&dir).unwrap();
        let err = fs_
            .write_atomic_with(
                &p("f"),
                b"x",
                |_| Err(io::Error::other("injected")),
                |from, to| fs::rename(from, to),
            )
            .unwrap_err();
        assert!(matches!(err, VfsError::Io { .. }));
        assert_eq!(snapshot(&dir), before);

        // 3. Failure of the rename, after the temp file was filled.
        let dir = setup("fail-at-rename");
        let before = snapshot(&dir);
        let mut fs_ = DiskFs::new(&dir).unwrap();
        let err = fs_
            .write_atomic_with(
                &p("f"),
                b"x",
                |_| Ok(()),
                |_, _| Err(io::Error::other("injected")),
            )
            .unwrap_err();
        assert!(matches!(err, VfsError::Io { .. }));
        assert_eq!(snapshot(&dir), before);
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
