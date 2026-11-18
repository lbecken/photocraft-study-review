//! On-disk VFS with atomic replace (decision D-014; Windows: T-073, "On Windows" below).
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
//!   to a directory is that directory), and that *directory* is the root: its device and inode
//!   are kept, and it is always held open for the life of the `DiskFs` (so the inode number
//!   cannot be reused by another directory after the root is removed; nothing is looked up
//!   through it). A root that cannot be opened for reading, whatever its search permission
//!   (for example mode `0311` or `0000`), cannot be held and is refused at construction
//!   (T-036).
//!   Every call resolves the root path *as given* again and walks from there only if it still
//!   reaches that same directory; if the root path or a directory above it has been
//!   replaced since (for example by a symlink to another directory), or leads nowhere, the
//!   call is refused with `OutsideRoot` before anything else is looked at.
//! - Every `read`, `write_atomic`, `is_file`, `list_dir` and `check_location` first computes the
//!   *location* of its path (for `list_dir` of the root: the root itself) by
//!   walking it from the root one component at a time, as the kernel does: each symlink is
//!   read and its target's components are walked in its place (an absolute target from `/`),
//!   and `..` pops the directory actually reached so far (physically, never lexically). The
//!   walk ends at an existing entry that is not a symlink, or at the first missing (or
//!   non-directory) component. `check_location` (T-037) stops after the walk: it answers
//!   `Ok(())` for an inside location of any kind (a file, a directory, a FIFO or other special
//!   file, nothing, or a missing parent) and opens nothing.
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
//! - Directory listings (`list_dir`, T-019): the location must be a directory (a file, FIFO,
//!   socket or device is `NotAFile`, and never opened); only that directory is opened and read.
//!   Each entry's kind is taken from the entry itself, never followed: a symlink is `Symlink`
//!   whatever it points to (a loop, the outside, nothing), a FIFO, socket or device is `Other`;
//!   no entry is opened, read or followed. Names that are not UTF-8 or not a valid `RelPath`
//!   segment (one containing `\`) are omitted and counted in `skipped`. Entries are sorted by
//!   the bytes of their names. A directory whose location has a host path too long for the
//!   system (`PATH_MAX`, 4,096 bytes on Linux) cannot be reached: `Io`.
//! - Precedence: `OutsideRoot`, then `NotFound`, then `NotAFile`, then `Io`. Nothing is cached:
//!   every call walks again, so what an earlier `read` saw does not authorise a later save.
//! - Link targets ending in `/` require a directory, as the kernel does.
//! - Accepted limitation (Decisions D-3 of T-008): the root check, the walk and the following
//!   open or rename are separate system calls. Another process that changes the tree *during*
//!   one call (for example swaps the root, a component or the file for a symlink, or the file
//!   for a FIFO, between the walk and the open) can make that call reach outside the root or
//!   block. A hard link inside the root to a file outside cannot be detected by path.
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
//!
//! On Windows (T-073; everything above is Unix's where it names Unix facilities):
//! - The root is held open as a `same_file::Handle` for the life of the `DiskFs`, and its
//!   identity is that handle's (volume serial number and file index); every call that re-checks
//!   the root opens the directory the given path reaches now and goes on only if it is the same
//!   file as the held one, otherwise `OutsideRoot`.
//! - The save's temporary file is created exclusively (`create_new`) with no mode. A read-only
//!   target is refused before that (the open for writing fails), so the target's `Permissions`
//!   copied as above are its whole attribute word (hidden, system, archive, …) without the
//!   read-only bit; its access list is not copied: the new file has the one its directory gives.
//!   Then it is renamed over the target. The directory sync stays best effort as above: any
//!   failure to open or sync the directory is ignored (unchecked on Windows, where opening a
//!   directory this way is expected to fail).
//! - The walk below the root (T-074) follows no link: every entry it reaches below the root,
//!   the last component included, is examined without being followed (`symlink_metadata`), and
//!   one whose attributes carry `FILE_ATTRIBUTE_REPARSE_POINT` (a file or directory symbolic
//!   link, a junction, or any other reparse point) makes the call `OutsideRoot` before anything
//!   there or beyond it is opened, created, replaced or removed, wherever it leads. So the
//!   location is the root joined with the request's segments. A listing gives such an entry
//!   kind `Symlink` (from the entry's own attributes), so it is never descended. Not on the
//!   walk: the root path and the directories above it (resolved as above).
//! - The temporary file (T-074) is created with `FILE_FLAG_OPEN_REPARSE_POINT` besides
//!   `create_new`, so a link or junction at a candidate name is never followed; a candidate
//!   whose create fails while an entry exists at that name (`symlink_metadata`, nothing
//!   followed or opened) is skipped as `AlreadyExists` is, and the entry is left as it was; a
//!   failure with nothing at the name is returned. Only when no name was free does the save
//!   fail (`AlreadyExists`, as above).
//! - Names that Windows gives a meaning (prefixes, reserved names, streams) and case are not
//!   decided here (T-095).

use std::collections::VecDeque;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Component, Path, PathBuf};

use crate::path::RelPath;
use crate::vfs::{DirEntry, DirListing, EntryKind, Vfs, VfsError};

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
    /// The root path as given at construction, made absolute (lexically: a symlink in it is
    /// kept as a symlink). Each call resolves it again ([`DiskFs::current_root`]) and goes on
    /// only if it still leads to the directory recorded at construction (ruling v8).
    given: PathBuf,
    /// Unix: device and inode of the root directory at construction: the root *is* this
    /// directory, whatever later happens to the path that led to it (Windows: `root_handle`).
    #[cfg(unix)]
    root_id: (u64, u64),
    /// The root directory, held open (read-only, never read through) for as long as this
    /// `DiskFs` lives, so that its inode cannot be freed and given to another directory: a
    /// root removed and replaced by a link to a new directory that reused the inode number
    /// would otherwise pass the `root_id` check. It is only an identity anchor: nothing is
    /// ever looked up through it, every walk starts from `given` resolved again. Every
    /// `DiskFs` holds one: a root that cannot be opened for reading is refused by
    /// [`DiskFs::new`] (T-036).
    #[cfg(unix)]
    _root_handle: fs::File,
    /// Windows (T-073): the root directory held open for as long as this `DiskFs` lives, and
    /// its identity: a directory reached later is the root iff its handle equals this one (same
    /// volume serial number and file index). Nothing is looked up through it.
    #[cfg(windows)]
    root_handle: same_file::Handle,
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
    /// The entry reached by the steps before must be a directory (a link target ending in
    /// `/`, as the kernel requires).
    DirMark,
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
    /// A VFS rooted at `root`. Err if root does not exist, is not a directory, or cannot be
    /// opened for reading (then it cannot be held open, so it is refused: `Io` with
    /// `PermissionDenied` for lack of permission, the open's own kind otherwise). Every
    /// `DiskFs` returned holds its root directory open for as long as it lives and takes the
    /// root's identity from the directory it holds: (device, inode) on Unix, the
    /// `same_file::Handle` (volume serial number, file index) on Windows.
    ///
    /// The error cannot name a `RelPath` of the root (none is valid), and must not name the
    /// host path, so it uses the placeholder path `(root)`. The root is the directory `root`
    /// leads to now; `root` itself is kept (made absolute) and resolved again on every call.
    pub fn new(root: &Path) -> Result<DiskFs, VfsError> {
        let placeholder = RelPath::root_placeholder;
        let root_error = |e: io::Error| match e.kind() {
            io::ErrorKind::NotFound => VfsError::NotFound {
                path: placeholder(),
            },
            _ => io_error(&placeholder(), &e),
        };
        match fs::metadata(root) {
            Ok(meta) if meta.is_dir() => {
                let resolved = fs::canonicalize(root).map_err(root_error)?;
                // A root that cannot be held open is refused (T-036): `PermissionDenied` for a
                // directory the process may not open for reading (whatever its search
                // permission), the open's own kind for any other failure.
                let open_error = |e: io::Error| io_error(&placeholder(), &e);
                #[cfg(unix)]
                let handle = fs::File::open(&resolved).map_err(open_error)?;
                // The identity is that of the directory actually held.
                #[cfg(unix)]
                let held = handle.metadata().map_err(open_error)?;
                // Windows (T-073): held and identified by a `same_file::Handle` (it opens the
                // directory with the flag Windows requires for directories).
                #[cfg(windows)]
                let root_handle = same_file::Handle::from_path(&resolved).map_err(open_error)?;
                Ok(DiskFs {
                    given: std::path::absolute(root).map_err(root_error)?,
                    #[cfg(unix)]
                    root_id: (held.dev(), held.ino()),
                    #[cfg(unix)]
                    _root_handle: handle,
                    #[cfg(windows)]
                    root_handle,
                })
            }
            Ok(_) => Err(VfsError::Io {
                path: placeholder(),
                kind: io::ErrorKind::NotADirectory,
                message: "the root is not a directory".to_owned(),
            }),
            Err(e) => Err(root_error(e)),
        }
    }

    /// The path at which the root directory is reached now: the root path as given resolved
    /// again (`fs::canonicalize`), which must still be the directory found at construction
    /// (Unix: same device and inode, and the held handle keeps that inode from being reused;
    /// Windows: the same volume serial number and file index, [`is_held_root`]). If the
    /// root path, or a directory above it, has since been replaced (for example by a symlink
    /// to another directory), or no longer leads anywhere, every location is outside the
    /// root: `OutsideRoot`. If the given path still leads to the root directory through other
    /// directories (an ancestor renamed and linked back), the path reached now is used.
    fn current_root(&self, rel: &RelPath) -> Result<PathBuf, VfsError> {
        let outside = || VfsError::OutsideRoot { path: rel.clone() };
        let root = fs::canonicalize(&self.given).map_err(|_| outside())?;
        let meta = fs::symlink_metadata(&root).map_err(|_| outside())?;
        #[cfg(unix)]
        if meta.is_dir() && (meta.dev(), meta.ino()) == self.root_id {
            Ok(root)
        } else {
            Err(outside())
        }
        // Windows (T-073): the directory reached now must be the same file as the held root.
        #[cfg(windows)]
        if meta.is_dir() && is_held_root(same_file::Handle::from_path(&root), &self.root_handle) {
            Ok(root)
        } else {
            Err(outside())
        }
    }

    /// Walk `rel` from the root to its location, following every symlink physically (module
    /// docs). `Err(OutsideRoot)` if the location is outside the root; nothing is opened.
    fn locate(&self, rel: &RelPath) -> Result<Location, VfsError> {
        self.locate_steps(rel, rel.segments())
    }

    /// [`DiskFs::locate`] of the path made of `segments` (none: the root itself); errors name
    /// `rel`.
    fn locate_steps<'s>(
        &self,
        rel: &RelPath,
        segments: impl Iterator<Item = &'s str>,
    ) -> Result<Location, VfsError> {
        let root = self.current_root(rel)?;
        // Whether `location` (absolute, symlink-free) is the root or below it, component by
        // component.
        let inside = |location: &Path| location.starts_with(&root);
        // An error met while walking from `dir`: `OutsideRoot` once the walk is outside the
        // root (so nothing about the outside is reported), otherwise `make()`.
        let walk_error = |dir: &Path, make: &dyn Fn() -> VfsError| {
            if inside(dir) {
                make()
            } else {
                VfsError::OutsideRoot { path: rel.clone() }
            }
        };
        let mut pending: VecDeque<Step> = segments.map(|s| Step::Name(OsString::from(s))).collect();
        // The directory reached so far: absolute, without symlinks, `.` or `..`.
        let mut dir = root.clone();
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
                // Reached only after a directory: nothing to check.
                Step::DirMark => continue,
                Step::Name(name) => name,
            };
            // The last real step: only `DirMark`s may follow.
            let last = pending.iter().all(|s| matches!(s, Step::DirMark));
            // This entry must be a directory (a link target ending in `/`).
            let must_be_dir = matches!(pending.front(), Some(Step::DirMark));
            let candidate = dir.join(&name);
            match fs::symlink_metadata(&candidate) {
                // Windows (T-074): no link is followed; an entry that is a reparse point (a
                // symbolic link, a junction or any other) is refused before anything there or
                // beyond it is opened, wherever it leads.
                #[cfg(windows)]
                Ok(meta) if is_reparse_point(&meta) => {
                    return Err(VfsError::OutsideRoot { path: rel.clone() });
                }
                Ok(meta) if meta.file_type().is_symlink() => {
                    hops += 1;
                    if hops > MAX_SYMLINK_HOPS {
                        return Err(walk_error(&dir, &|| VfsError::Io {
                            path: rel.clone(),
                            kind: io::ErrorKind::Other,
                            message: "too many levels of symbolic links".to_owned(),
                        }));
                    }
                    let target = fs::read_link(&candidate)
                        .map_err(|e| walk_error(&dir, &|| io_error(rel, &e)))?;
                    // `Path::components` drops a trailing `/`, which requires a directory.
                    if target.as_os_str().as_encoded_bytes().last() == Some(&b'/') {
                        pending.push_front(Step::DirMark);
                    }
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
                Ok(meta) if last && !must_be_dir => {
                    let found = if meta.is_file() {
                        Found::File(meta)
                    } else {
                        Found::Special
                    };
                    break (candidate, found);
                }
                // A non-directory before the last component, or where a directory is required.
                Ok(_) => break (candidate, Found::NoParent),
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                    ) =>
                {
                    let found = if last && !must_be_dir {
                        Found::Missing
                    } else {
                        Found::NoParent
                    };
                    break (candidate, found);
                }
                Err(e) => return Err(walk_error(&dir, &|| io_error(rel, &e))),
            }
        };
        if !inside(&path) {
            return Err(VfsError::OutsideRoot { path: rel.clone() });
        }
        Ok(Location { path, dir, found })
    }
}

/// Whether the directory reached now is the held root: only when its identity could be read
/// (`now` is `Ok`) and equals the held one; a failed read answers "outside" (T-073, ruling c5).
/// No platform cfg: the Windows root check calls it, and its unit test runs on every platform.
/// The Unix check compares `(dev, ino)` itself and does not use it.
#[cfg_attr(not(windows), allow(dead_code))]
fn is_held_root<T: PartialEq, E>(now: Result<T, E>, held: &T) -> bool {
    now.is_ok_and(|now| now == *held)
}

/// Windows (T-074): `FILE_ATTRIBUTE_REPARSE_POINT`.
#[cfg(windows)]
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;

/// Windows (T-074): `FILE_FLAG_OPEN_REPARSE_POINT`: the temporary create acts on the name
/// itself, so a link or junction there is never followed.
#[cfg(windows)]
const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;

/// Windows (T-074): whether `meta` (of the entry itself, not followed) carries the
/// reparse-point attribute: a file or directory symbolic link, a junction, or any other
/// reparse point.
#[cfg(windows)]
fn is_reparse_point(meta: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
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
#[cfg(unix)]
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

/// Windows (T-073): [`create_temp`] without a mode (the `0600`/`0666` modes are Unix's; the
/// target's permissions are applied afterwards as on Unix), so `_mode` is not used.
/// T-074: the create opens the name itself (`FILE_FLAG_OPEN_REPARSE_POINT`), and a name whose
/// create fails while an entry exists there (`symlink_metadata`, nothing followed or opened;
/// for example a junction, which does not answer `AlreadyExists`) is taken: the next is tried.
#[cfg(windows)]
fn create_temp(dir: &Path, _mode: u32) -> io::Result<(PathBuf, fs::File)> {
    use std::os::windows::fs::OpenOptionsExt;
    for n in 0..TEMP_ATTEMPTS {
        let temp = dir.join(format!("{TEMP_PREFIX}{n}"));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(&temp)
        {
            Ok(file) => return Ok((temp, file)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(_) if fs::symlink_metadata(&temp).is_ok() => continue,
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

    fn list_dir(&self, dir: Option<&RelPath>) -> Result<DirListing, VfsError> {
        let placeholder = RelPath::root_placeholder();
        let (label, location) = match dir {
            None => (
                &placeholder,
                self.locate_steps(&placeholder, std::iter::empty())?,
            ),
            Some(dir) => (dir, self.locate(dir)?),
        };
        match location.found {
            Found::Missing | Found::NoParent => {
                return Err(VfsError::NotFound {
                    path: label.clone(),
                });
            }
            // Never opened: a FIFO, socket or device is not a directory.
            Found::File(_) | Found::Special => {
                return Err(VfsError::NotAFile {
                    path: label.clone(),
                });
            }
            Found::Dir => {}
        }
        let mut listing = DirListing::default();
        for entry in fs::read_dir(&location.path).map_err(|e| lookup_error(label, &e))? {
            let entry = entry.map_err(|e| io_error(label, &e))?;
            // The entry's own type (`lstat` semantics): a symlink is never followed.
            let file_type = entry.file_type().map_err(|e| io_error(label, &e))?;
            let name = match entry.file_name().into_string() {
                Ok(name) if RelPath::new(&name).is_ok() => name,
                _ => {
                    listing.skipped += 1;
                    continue;
                }
            };
            let kind = if file_type.is_symlink() {
                EntryKind::Symlink
            } else if file_type.is_dir() {
                EntryKind::Dir
            } else if file_type.is_file() {
                EntryKind::File
            } else {
                EntryKind::Other
            };
            // Windows (T-074): any reparse point is `Symlink`, so it is never descended; the
            // attributes are the entry's own, from the directory read (nothing followed).
            #[cfg(windows)]
            let kind = if is_reparse_point(&entry.metadata().map_err(|e| io_error(label, &e))?) {
                EntryKind::Symlink
            } else {
                kind
            };
            listing.entries.push(DirEntry { name, kind });
        }
        listing
            .entries
            .sort_unstable_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
        Ok(listing)
    }

    /// The root check and the walk (`locate`), and nothing after them: `Ok(())` for an inside
    /// location of any kind (file, directory, special file, nothing, missing parent); nothing
    /// is opened.
    fn check_location(&self, path: &RelPath) -> Result<(), VfsError> {
        self.locate(path).map(|_| ())
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

    /// T-073 (ruling c5): the decision of the Windows root check, on every platform: inside only
    /// for an identity that was read and equals the held one; a failed read is outside.
    #[test]
    fn held_root_check_is_inside_only_for_a_read_identity_equal_to_the_held_one() {
        let held = (7_u64, 42_u64);
        assert!(is_held_root(Ok::<_, io::Error>((7_u64, 42_u64)), &held));
        assert!(!is_held_root(Ok::<_, io::Error>((7_u64, 43_u64)), &held));
        assert!(!is_held_root(Ok::<_, io::Error>((8_u64, 42_u64)), &held));
        assert!(!is_held_root(
            Err::<(u64, u64), _>(io::Error::other("the identity could not be read")),
            &held
        ));
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

    #[cfg(unix)]
    fn mode(path: &Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path).unwrap().permissions().mode() & 0o7777
    }

    #[cfg(unix)]
    fn set_mode(path: &Path, mode: u32) {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    }

    #[cfg(unix)]
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

    #[cfg(unix)]
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

    #[cfg(unix)]
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
        // The root is resolved at construction (`fs::canonicalize`), so the temporary file is
        // named below the canonical directory: on macOS the temp dir is reached through a link.
        assert_eq!(used, Some(dir.canonicalize().unwrap().join(".kiln-tmp-1")));
        assert_eq!(listing(&dir), [".kiln-tmp-0", "f"]);
        assert_eq!(fs::read(dir.join("f")).unwrap(), b"old");
        assert_eq!(fs::read(dir.join(".kiln-tmp-0")).unwrap(), b"not mine");
    }

    #[cfg(unix)]
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
    #[cfg(unix)]
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
    #[cfg(unix)]
    fn snapshot(dir: &Path) -> (Vec<String>, Vec<u8>, u32) {
        (
            listing(dir),
            fs::read(dir.join("f")).unwrap(),
            mode(&dir.join("f")),
        )
    }

    /// Kills "the target is emptied when a save fails after the writability check" (review r2
    /// TG-5), with three failures that can be provoked as root, all after that check.
    #[cfg(unix)]
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

    #[cfg(unix)]
    #[test]
    fn new_file_gets_default_mode() {
        let dir = fresh_dir("newmode");
        fs::write(dir.join("reference"), b"").unwrap();
        let mut fs_ = DiskFs::new(&dir).unwrap();
        fs_.write_atomic(&p("new"), b"x").unwrap();
        assert_eq!(mode(&dir.join("new")), mode(&dir.join("reference")));
    }

    #[cfg(unix)]
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

    /// `work/` with `root/f.txt`, `outside/secret.txt` and `other/` (for parent swaps).
    #[cfg(unix)]
    fn swap_fixture(name: &str) -> PathBuf {
        let work = fresh_dir(name);
        fs::create_dir_all(work.join("parent/root")).unwrap();
        fs::create_dir_all(work.join("outside")).unwrap();
        fs::write(work.join("parent/root/f.txt"), b"in\n").unwrap();
        fs::write(work.join("outside/secret.txt"), b"s3cr3t-content\n").unwrap();
        work
    }

    /// Review r1 D-1 (ruling v4): the root path replaced by a symlink to another directory
    /// between two requests; a session keeps its root directory.
    #[cfg(unix)]
    #[test]
    fn root_replaced_by_a_symlink_between_requests_is_outside() {
        use std::os::unix::fs::symlink;
        let work = swap_fixture("root-swap");
        let root = work.join("parent/root");
        let mut fs_ = DiskFs::new(&root).unwrap();
        let mut doc = crate::Document::open(&fs_, &p("f.txt")).unwrap();
        doc.buffer_mut().insert(0, "PWNED ").unwrap();
        fs::rename(&root, work.join("parent/root.real")).unwrap();
        symlink("../outside", &root).unwrap();
        // The swap works: the outside file is reachable through the root path now.
        assert_eq!(
            fs::read(root.join("secret.txt")).unwrap(),
            b"s3cr3t-content\n"
        );
        fs::write(root.join("f.txt"), b"outside f\n").unwrap();
        let before = listing(&work.join("outside"));
        for path in ["secret.txt", "f.txt", "new.txt"] {
            let want = VfsError::OutsideRoot { path: p(path) };
            assert_eq!(fs_.read(&p(path)), Err(want.clone()), "read {path}");
            assert_eq!(
                fs_.write_atomic(&p(path), b"PWNED"),
                Err(want),
                "write {path}"
            );
            assert!(!fs_.is_file(&p(path)));
        }
        assert_eq!(
            doc.save(&mut fs_),
            Err(VfsError::OutsideRoot { path: p("f.txt") })
        );
        assert!(doc.is_dirty());
        assert_eq!(
            fs::read(work.join("outside/secret.txt")).unwrap(),
            b"s3cr3t-content\n"
        );
        assert_eq!(
            fs::read(work.join("outside/f.txt")).unwrap(),
            b"outside f\n"
        );
        assert_eq!(listing(&work.join("outside")), before);
        assert_eq!(
            fs::read(work.join("parent/root.real/f.txt")).unwrap(),
            b"in\n"
        );

        // Renamed with nothing in its place: still nothing read or written anywhere.
        fs::remove_file(&root).unwrap();
        assert!(fs_.read(&p("f.txt")).is_err());
        assert!(fs_.write_atomic(&p("g.txt"), b"x").is_err());
        assert!(!work.join("parent/root.real/g.txt").exists());
        // A new directory at the root path is not the root either.
        fs::create_dir(&root).unwrap();
        fs::write(root.join("f.txt"), b"new dir\n").unwrap();
        assert_eq!(
            fs_.read(&p("f.txt")),
            Err(VfsError::OutsideRoot { path: p("f.txt") })
        );
    }

    /// Review r1 D-1 (ruling v4): a directory above the root replaced by a symlink to another
    /// directory that has a `root` of its own.
    #[cfg(unix)]
    #[test]
    fn parent_replaced_by_a_symlink_between_requests_is_outside() {
        use std::os::unix::fs::symlink;
        let work = swap_fixture("parent-swap");
        fs::create_dir_all(work.join("outside/root")).unwrap();
        fs::write(work.join("outside/root/secret.txt"), b"s3cr3t-content\n").unwrap();
        let root = work.join("parent/root");
        let mut fs_ = DiskFs::new(&root).unwrap();
        assert_eq!(fs_.read(&p("f.txt")).unwrap(), b"in\n");
        fs::rename(work.join("parent"), work.join("parent.real")).unwrap();
        symlink("outside", work.join("parent")).unwrap();
        assert_eq!(
            fs::read(root.join("secret.txt")).unwrap(),
            b"s3cr3t-content\n"
        );
        for path in ["secret.txt", "new.txt"] {
            let want = VfsError::OutsideRoot { path: p(path) };
            assert_eq!(fs_.read(&p(path)), Err(want.clone()), "read {path}");
            assert_eq!(
                fs_.write_atomic(&p(path), b"PWNED"),
                Err(want),
                "write {path}"
            );
        }
        assert_eq!(
            fs::read(work.join("outside/root/secret.txt")).unwrap(),
            b"s3cr3t-content\n"
        );
        assert_eq!(listing(&work.join("outside/root")), ["secret.txt"]);

        // The parent renamed and linked back to itself: the root directory is unchanged and
        // reached through a new path, so the session keeps working there.
        fs::remove_file(work.join("parent")).unwrap();
        symlink("parent.real", work.join("parent")).unwrap();
        assert_eq!(fs_.read(&p("f.txt")).unwrap(), b"in\n");
        fs_.write_atomic(&p("f.txt"), b"saved\n").unwrap();
        assert_eq!(
            fs::read(work.join("parent.real/root/f.txt")).unwrap(),
            b"saved\n"
        );
        assert!(
            fs::symlink_metadata(work.join("parent"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    /// Ruling v4: the root removed (its inode freed unless held) and replaced by a link to a
    /// new directory outside; on Linux file systems such a directory often gets the freed inode
    /// number, which a bare (device, inode) comparison would take for the root.
    #[cfg(unix)]
    #[test]
    fn root_removed_and_replaced_by_a_link_to_a_new_directory_is_outside() {
        use std::os::unix::fs::symlink;
        let work = swap_fixture("root-reuse");
        let root = work.join("parent/root");
        let mut fs_ = DiskFs::new(&root).unwrap();
        let old = fs::metadata(&root).unwrap();
        fs::remove_dir_all(&root).unwrap();
        // New directories outside; without the held handle one of them usually gets the root's
        // freed inode number.
        for i in 0..4 {
            let d = work.join(format!("outside/x{i}"));
            fs::create_dir(&d).unwrap();
            fs::write(d.join("secret.txt"), b"s3cr3t-content\n").unwrap();
            let m = fs::metadata(&d).unwrap();
            assert!(
                (m.dev(), m.ino()) != (old.dev(), old.ino()),
                "the removed root's inode was reused while the DiskFs holds it"
            );
        }
        symlink("../outside/x0", &root).unwrap();
        assert_eq!(
            fs::read(root.join("secret.txt")).unwrap(),
            b"s3cr3t-content\n"
        );
        let before = listing(&work.join("outside"));
        for path in ["secret.txt", "new.txt"] {
            let want = VfsError::OutsideRoot { path: p(path) };
            assert_eq!(fs_.read(&p(path)), Err(want.clone()), "read {path}");
            assert_eq!(
                fs_.write_atomic(&p(path), b"PWNED"),
                Err(want),
                "write {path}"
            );
        }
        assert_eq!(
            fs::read(work.join("outside/x0/secret.txt")).unwrap(),
            b"s3cr3t-content\n"
        );
        assert_eq!(listing(&work.join("outside")), before);
        assert_eq!(listing(&work.join("outside/x0")), ["secret.txt"]);
    }

    /// Ruling v8: a root given through a symlink keeps working while the link leads to the
    /// root directory (also after the directory moved and the link followed it), and is
    /// outside once the link is pointed at another directory, including for a document opened
    /// before.
    #[cfg(unix)]
    #[test]
    fn root_given_through_a_link_that_is_retargeted() {
        use std::os::unix::fs::symlink;
        let work = swap_fixture("root-retarget");
        let link = work.join("rootlink");
        symlink("parent/root", &link).unwrap();
        let mut fs_ = DiskFs::new(&link).unwrap();
        assert_eq!(fs_.read(&p("f.txt")).unwrap(), b"in\n");
        let mut doc = crate::Document::open(&fs_, &p("f.txt")).unwrap();
        doc.buffer_mut().insert(0, "PWNED ").unwrap();
        // The directory moves and the link follows it: still the root.
        fs::rename(work.join("parent/root"), work.join("parent/moved")).unwrap();
        fs::remove_file(&link).unwrap();
        symlink("parent/moved", &link).unwrap();
        assert_eq!(fs_.read(&p("f.txt")).unwrap(), b"in\n");
        // The link now leads to another directory: outside, at open and at save.
        fs::write(work.join("outside/f.txt"), b"outside f\n").unwrap();
        fs::remove_file(&link).unwrap();
        symlink("outside", &link).unwrap();
        assert_eq!(
            fs::read(link.join("secret.txt")).unwrap(),
            b"s3cr3t-content\n"
        );
        let before = listing(&work.join("outside"));
        for path in ["secret.txt", "f.txt", "new.txt"] {
            let want = VfsError::OutsideRoot { path: p(path) };
            assert_eq!(fs_.read(&p(path)), Err(want.clone()), "read {path}");
            assert_eq!(
                fs_.write_atomic(&p(path), b"PWNED"),
                Err(want),
                "write {path}"
            );
        }
        assert_eq!(
            doc.save(&mut fs_),
            Err(VfsError::OutsideRoot { path: p("f.txt") })
        );
        assert!(doc.is_dirty());
        assert_eq!(listing(&work.join("outside")), before);
        assert_eq!(
            fs::read(work.join("outside/f.txt")).unwrap(),
            b"outside f\n"
        );
        assert_eq!(
            fs::read(work.join("outside/secret.txt")).unwrap(),
            b"s3cr3t-content\n"
        );
        assert_eq!(fs::read(work.join("parent/moved/f.txt")).unwrap(), b"in\n");
    }

    /// Review r1 N-1 (ruling v7): a link target ending in `/` requires a directory.
    #[cfg(unix)]
    #[test]
    fn link_target_with_trailing_slash_requires_a_directory() {
        use std::os::unix::fs::symlink;
        let dir = fresh_dir("trail");
        let mut fs_ = DiskFs::new(&dir).unwrap();
        fs::create_dir(dir.join("real")).unwrap();
        fs::write(dir.join("real/t.txt"), b"t").unwrap();
        symlink("real/t.txt/", dir.join("trail")).unwrap();
        symlink("real/new.txt/", dir.join("newtrail")).unwrap();
        symlink("real/", dir.join("dtrail")).unwrap();
        symlink("trail", dir.join("viatrail")).unwrap();
        // The kernel agrees: the file cannot be reached through the trailing slash.
        assert!(fs::read(dir.join("trail")).is_err());
        for path in ["trail", "viatrail", "newtrail"] {
            let want = VfsError::NotFound { path: p(path) };
            assert_eq!(fs_.read(&p(path)), Err(want.clone()), "read {path}");
            assert_eq!(fs_.write_atomic(&p(path), b"x"), Err(want), "write {path}");
            assert!(!fs_.is_file(&p(path)));
        }
        assert_eq!(fs::read(dir.join("real/t.txt")).unwrap(), b"t");
        assert!(!dir.join("real/new.txt").exists());
        // A directory through a trailing slash is still a directory; below it, files work.
        assert_eq!(
            fs_.read(&p("dtrail")),
            Err(VfsError::NotAFile { path: p("dtrail") })
        );
        assert_eq!(fs_.read(&p("dtrail/t.txt")).unwrap(), b"t");
        assert_eq!(listing(&dir.join("real")), ["t.txt"]);
    }

    /// A fresh work directory (as [`fresh_dir`]) removed when dropped, also when the test
    /// fails; search permission is restored on every directory first.
    #[cfg(unix)]
    struct Work(PathBuf);

    #[cfg(unix)]
    impl Work {
        fn new(name: &str) -> Work {
            Work(fresh_dir(name))
        }
    }

    #[cfg(unix)]
    impl Drop for Work {
        fn drop(&mut self) {
            use std::os::unix::fs::PermissionsExt;
            fn restore(dir: &Path) {
                let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o755));
                if let Ok(entries) = fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        if entry.file_type().is_ok_and(|t| t.is_dir()) {
                            restore(&entry.path());
                        }
                    }
                }
            }
            restore(&self.0);
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// Wait for `child` at most 10 s (a hang detector), killing it after that.
    #[cfg(unix)]
    fn wait_bounded(child: &mut std::process::Child) -> bool {
        for _ in 0..1000 {
            if let Some(status) = child.try_wait().unwrap() {
                return status.success();
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let _ = child.kill();
        let _ = child.wait();
        false
    }

    #[cfg(unix)]
    fn mkfifo(path: &Path) {
        use std::os::unix::fs::FileTypeExt;
        let mut child = std::process::Command::new("mkfifo")
            .arg(path)
            .stdin(std::process::Stdio::null())
            .spawn()
            .expect("spawn mkfifo");
        assert!(wait_bounded(&mut child), "mkfifo failed or hung");
        assert!(fs::symlink_metadata(path).unwrap().file_type().is_fifo());
    }

    /// `root/` with `a.txt`, `sub/c.txt`, `real/t.txt`, and `outside/secret.txt` next to it.
    #[cfg(unix)]
    fn location_fixture(name: &str) -> (Work, PathBuf) {
        let work = Work::new(name);
        let root = work.0.join("root");
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::create_dir_all(root.join("real")).unwrap();
        fs::create_dir_all(work.0.join("outside")).unwrap();
        fs::write(root.join("a.txt"), b"alpha\n").unwrap();
        fs::write(root.join("sub/c.txt"), b"charlie\n").unwrap();
        fs::write(root.join("real/t.txt"), b"tango\n").unwrap();
        fs::write(work.0.join("outside/secret.txt"), b"s3cr3t-content\n").unwrap();
        (work, root)
    }

    /// T-037: `check_location` is `Ok` for an inside location of every kind, and opens nothing
    /// (a FIFO at the location is answered without blocking and stays a FIFO).
    #[cfg(unix)]
    #[test]
    fn check_location_is_ok_for_every_inside_location() {
        use std::os::unix::fs::{FileTypeExt, symlink};
        let (_work, root) = location_fixture("check-inside");
        let fs_ = DiskFs::new(&root).unwrap();
        fs::create_dir(root.join("dir")).unwrap();
        mkfifo(&root.join("fifo"));
        symlink("a.txt", root.join("to-a")).unwrap();
        symlink("none.txt", root.join("dangling")).unwrap();
        symlink("real/t.txt/", root.join("trail")).unwrap();
        symlink("../root/sub", root.join("via-out-and-back")).unwrap();
        fs::write(root.join("bad.txt"), [0xFF, 0xFE, 0xFF]).unwrap();
        let paths = [
            "a.txt",                  // a regular file
            "dir",                    // a directory
            "sub",                    // a directory
            "fifo",                   // a special file, never opened
            "none.txt",               // nothing there, parent exists
            "nope/x.txt",             // a missing parent
            "a.txt/x",                // a parent that is a file
            "to-a",                   // a link to a file inside
            "dangling",               // a dangling link inside
            "trail",                  // a trailing `/` to a file
            "via-out-and-back/c.txt", // leaves the root and comes back
            "bad.txt",                // undecodable
        ];
        // On its own thread, so that a check that opened the FIFO would be noticed (hang
        // detector of 10 s) instead of blocking the test run.
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let results: Vec<_> = paths
                .iter()
                .map(|s| (*s, fs_.check_location(&p(s))))
                .collect();
            let _ = tx.send(results);
        });
        let results = match rx.recv_timeout(std::time::Duration::from_secs(10)) {
            Ok(results) => results,
            Err(e) => {
                // Release a reader blocked on the FIFO before failing.
                let _ = fs::OpenOptions::new().write(true).open(root.join("fifo"));
                panic!("check_location did not answer: {e}");
            }
        };
        for (path, result) in results {
            assert_eq!(result, Ok(()), "{path}");
        }
        assert!(
            fs::symlink_metadata(root.join("fifo"))
                .unwrap()
                .file_type()
                .is_fifo()
        );
        assert!(!root.join("none.txt").exists());
        assert!(!root.join("nope").exists());
    }

    /// T-037: a location outside, a link loop, and a root that is no longer the root.
    #[cfg(unix)]
    #[test]
    fn check_location_refuses_outside_loops_and_a_replaced_root() {
        use std::os::unix::fs::symlink;
        let (work, root) = location_fixture("check-refuse");
        let fs_ = DiskFs::new(&root).unwrap();
        let secret = work.0.join("outside/secret.txt");
        symlink("../outside/secret.txt", root.join("rel-out")).unwrap();
        symlink(&secret, root.join("abs-out")).unwrap();
        symlink("../outside/none.txt", root.join("dangling-out")).unwrap();
        symlink("../outside", root.join("dir-out")).unwrap();
        symlink("loop2", root.join("loop1")).unwrap();
        symlink("loop1", root.join("loop2")).unwrap();
        for path in [
            "rel-out",
            "abs-out",
            "dangling-out",
            "dir-out/secret.txt",
            "dir-out/none.txt",
        ] {
            assert_eq!(
                fs_.check_location(&p(path)),
                Err(VfsError::OutsideRoot { path: p(path) }),
                "{path}"
            );
        }
        assert!(matches!(
            fs_.check_location(&p("loop1")),
            Err(VfsError::Io { path, .. }) if path == p("loop1")
        ));
        assert!(!work.0.join("outside/none.txt").exists());

        // The root renamed away: every location is outside; moved back: inside again.
        let away = work.0.join("root.away");
        fs::rename(&root, &away).unwrap();
        assert!(!root.exists());
        assert_eq!(
            fs_.check_location(&p("a.txt")),
            Err(VfsError::OutsideRoot { path: p("a.txt") })
        );
        // A link at the root path to another directory is not the root.
        symlink("outside", &root).unwrap();
        assert_eq!(
            fs_.check_location(&p("secret.txt")),
            Err(VfsError::OutsideRoot {
                path: p("secret.txt")
            })
        );
        fs::remove_file(&root).unwrap();
        fs::rename(&away, &root).unwrap();
        assert_eq!(fs_.check_location(&p("a.txt")), Ok(()));
        // Removed, and replaced by a new directory holding the same name: still outside.
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(
            fs_.check_location(&p("a.txt")),
            Err(VfsError::OutsideRoot { path: p("a.txt") })
        );
        fs::create_dir(&root).unwrap();
        fs::write(root.join("a.txt"), b"alpha\n").unwrap();
        assert_eq!(
            fs_.check_location(&p("a.txt")),
            Err(VfsError::OutsideRoot { path: p("a.txt") })
        );
        assert_eq!(fs::read(&secret).unwrap(), b"s3cr3t-content\n");
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
            // T-037: no links here, so every location is inside for both (consumes no seed).
            assert_eq!(
                disk.check_location(&path),
                mem.check_location(&path),
                "check_location {path}"
            );
        }
        for path in mem.paths() {
            assert_eq!(disk.read(&path).unwrap(), mem.get(&path).unwrap());
        }
        assert!(listing(&dir).iter().all(|n| !n.starts_with(TEMP_PREFIX)));
    }

    // ---- T-074: the counterparts of the nine Unix-only link tests (Interface (5)) ----------
    // No platform cfg on these tests: each asserts Unix behaviour on Unix and the Windows rule
    // (no link followed, every reparse point `OutsideRoot`) on Windows, chosen by `cfg!(windows)`
    // where the two differ. The link helpers below are defined per platform, each doing its work.

    /// A symbolic link at `link` to the file `target` (relative to the link's directory, or
    /// absolute, as given).
    #[cfg(unix)]
    fn file_link(target: &Path, link: &Path) {
        std::os::unix::fs::symlink(target, link)
            .unwrap_or_else(|e| panic!("symlink {}: {e}", link.display()));
    }

    /// A file symbolic link (`symlink_file`; the runner allows it, T-074 probe A1, A5, A6).
    #[cfg(windows)]
    fn file_link(target: &Path, link: &Path) {
        std::os::windows::fs::symlink_file(target, link).unwrap_or_else(|e| {
            panic!(
                "symlink_file {}: {e} (a file symbolic link cannot be made here)",
                link.display()
            )
        });
    }

    /// A symbolic link at `link` to the directory `target` (relative or absolute, as given).
    #[cfg(unix)]
    fn dir_link(target: &Path, link: &Path) {
        std::os::unix::fs::symlink(target, link)
            .unwrap_or_else(|e| panic!("symlink {}: {e}", link.display()));
    }

    /// A directory symbolic link (`symlink_dir`; probe A2).
    #[cfg(windows)]
    fn dir_link(target: &Path, link: &Path) {
        std::os::windows::fs::symlink_dir(target, link).unwrap_or_else(|e| {
            panic!(
                "symlink_dir {}: {e} (a directory symbolic link cannot be made here)",
                link.display()
            )
        });
    }

    /// Unix has no junctions: a symbolic link to the same absolute directory stands in for one.
    #[cfg(unix)]
    fn junction(target: &Path, link: &Path) {
        assert!(target.is_absolute(), "a junction target is absolute");
        std::os::unix::fs::symlink(target, link)
            .unwrap_or_else(|e| panic!("symlink {}: {e}", link.display()));
    }

    /// A junction at `link` to the absolute directory `target`, by `cmd /C mklink /J` (probe A3),
    /// waited for at most 30 s; a junction that cannot be made fails the test, naming why.
    #[cfg(windows)]
    fn junction(target: &Path, link: &Path) {
        use std::process::{Command, Stdio};
        assert!(target.is_absolute(), "a junction target is absolute");
        let mut child = Command::new("cmd")
            .arg("/C")
            .arg("mklink")
            .arg("/J")
            .arg(link)
            .arg(target)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("cannot run `cmd /C mklink /J` to make a junction: {e}"));
        let mut status = None;
        for _ in 0..3000 {
            if let Some(s) = child.try_wait().unwrap() {
                status = Some(s);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let Some(status) = status else {
            let _ = child.kill();
            let _ = child.wait();
            panic!("`mklink /J {}` did not finish within 30 s", link.display());
        };
        assert!(
            status.success(),
            "`mklink /J {}` failed ({status}): a junction cannot be made here",
            link.display()
        );
    }

    /// Whether `path` is itself a link (junctions included: `is_symlink` on Windows, probe B1).
    fn is_link(path: &Path) -> bool {
        fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink())
    }

    /// Remove the link (file or directory link, or junction) at `path`, never its target.
    fn remove_link(path: &Path) {
        assert!(is_link(path), "{} is not a link", path.display());
        fs::remove_file(path)
            .or_else(|_| fs::remove_dir(path))
            .unwrap_or_else(|e| panic!("remove link {}: {e}", path.display()));
    }

    /// Every call of the API answers `OutsideRoot` for each of `paths` (`is_file` false), and
    /// `list_dir` of the root too: the root path no longer leads to the root.
    fn assert_every_call_outside(fs_: &mut DiskFs, paths: &[&str], context: &str) {
        for path in paths {
            let want = VfsError::OutsideRoot { path: p(path) };
            assert_eq!(
                fs_.read(&p(path)),
                Err(want.clone()),
                "{context}: read {path}"
            );
            assert_eq!(
                fs_.write_atomic(&p(path), b"PWNED"),
                Err(want.clone()),
                "{context}: write {path}"
            );
            assert!(!fs_.is_file(&p(path)), "{context}: is_file {path}");
            assert_eq!(
                fs_.list_dir(Some(&p(path))),
                Err(want.clone()),
                "{context}: list_dir {path}"
            );
            assert_eq!(
                fs_.check_location(&p(path)),
                Err(want),
                "{context}: check_location {path}"
            );
        }
        assert_eq!(
            fs_.list_dir(None),
            Err(VfsError::OutsideRoot {
                path: RelPath::root_placeholder()
            }),
            "{context}: list_dir of the root"
        );
    }

    /// Counterpart of `symlink_at_temp_name_is_not_followed` (registry #2; T-074 Interface (3)):
    /// a link or junction at a temporary name is never followed or changed, and the save takes
    /// the next name. The same on every platform (Unix: `O_EXCL` refuses the link; Windows: the
    /// create opens the name itself and a taken name is skipped). Four fixtures, each in a fresh
    /// directory: (a) the probe's pair (junction at `-0` to an outside directory, file link at
    /// `-1` to a missing outside file); (b) a junction alone; (c) a file link alone to an
    /// existing outside file; (d) a dangling file link alone to a missing outside path.
    #[test]
    fn link_at_temp_name_is_not_followed_on_every_platform() {
        for case in ["a-pair", "b-junction", "c-file-link", "d-dangling"] {
            let work = fresh_dir(&format!("tmp-link-{case}"));
            let root = work.join("root");
            let outside = work.join("outside");
            fs::create_dir_all(&root).unwrap();
            fs::create_dir_all(&outside).unwrap();
            fs::write(outside.join("precious"), b"precious").unwrap();
            fs::write(root.join("f"), b"old").unwrap();
            let missing = outside.join("missing");
            let t0 = root.join(format!("{TEMP_PREFIX}0"));
            let planted: Vec<PathBuf> = match case {
                "a-pair" => {
                    let t1 = root.join(format!("{TEMP_PREFIX}1"));
                    junction(&outside, &t0);
                    file_link(&missing, &t1);
                    vec![t0, t1]
                }
                "b-junction" => {
                    junction(&outside, &t0);
                    vec![t0]
                }
                "c-file-link" => {
                    file_link(&outside.join("precious"), &t0);
                    vec![t0]
                }
                _ => {
                    file_link(&missing, &t0);
                    vec![t0]
                }
            };
            let targets: Vec<PathBuf> = planted.iter().map(|l| fs::read_link(l).unwrap()).collect();
            let mut fs_ = DiskFs::new(&root).unwrap();
            let mut used = None;
            fs_.write_atomic_with(
                &p("f"),
                b"EDIT",
                |_| Ok(()),
                |from, to| {
                    used = Some(from.to_path_buf());
                    fs::rename(from, to)
                },
            )
            .unwrap_or_else(|e| panic!("{case}: save: {e:?}"));
            // The first name not planted.
            let next = format!("{TEMP_PREFIX}{}", planted.len());
            assert_eq!(
                used,
                Some(root.canonicalize().unwrap().join(&next)),
                "{case}"
            );
            assert_eq!(fs::read(root.join("f")).unwrap(), b"EDIT", "{case}");
            fs_.write_atomic(&p("f"), b"EDIT 2")
                .unwrap_or_else(|e| panic!("{case}: second save: {e:?}"));
            assert_eq!(fs::read(root.join("f")).unwrap(), b"EDIT 2", "{case}");
            assert!(
                fs::symlink_metadata(root.join("f"))
                    .unwrap()
                    .file_type()
                    .is_file(),
                "{case}"
            );
            for (link, target) in planted.iter().zip(&targets) {
                assert!(is_link(link), "{case}: {} is still a link", link.display());
                assert_eq!(&fs::read_link(link).unwrap(), target, "{case}");
            }
            assert_eq!(listing(&outside), ["precious"], "{case}: outside unchanged");
            assert_eq!(fs::read(outside.join("precious")).unwrap(), b"precious");
            assert!(
                fs::symlink_metadata(&missing).is_err(),
                "{case}: still missing"
            );
            let mut want: Vec<String> = planted
                .iter()
                .map(|l| l.file_name().unwrap().to_string_lossy().into_owned())
                .collect();
            want.push("f".to_owned());
            assert_eq!(listing(&root), want, "{case}: no temporary file left");
        }
    }

    /// Counterpart of `replace_swaps_inode_keeps_permissions_and_follows_symlinks` (#8) and of
    /// `link_target_with_trailing_slash_requires_a_directory` (#13): a save and a read through a
    /// file link (relative and absolute), a directory link and a junction on the way. Unix: the
    /// link is followed, its target replaced and the link kept. Windows: `OutsideRoot`, and
    /// link, target and directory unchanged.
    #[test]
    fn save_through_a_link_on_every_platform() {
        let work = fresh_dir("save-through-link");
        let root = work.join("root");
        let t = root.join("t");
        fs::create_dir_all(&t).unwrap();
        fs::write(t.join("target"), b"old").unwrap();
        file_link(&Path::new("t").join("target"), &root.join("rel"));
        file_link(&t.join("target"), &root.join("abs"));
        dir_link(Path::new("t"), &root.join("dl"));
        junction(&t, &root.join("j"));
        let links = ["abs", "dl", "j", "rel"];
        let before: Vec<PathBuf> = links
            .iter()
            .map(|l| fs::read_link(root.join(l)).unwrap())
            .collect();
        let mut fs_ = DiskFs::new(&root).unwrap();
        let mut doc = crate::Document::open(&fs_, &p("t/target")).unwrap();
        for path in ["rel", "abs", "dl/target", "j/target"] {
            let bytes = format!("via {path}");
            let read = fs_.read(&p(path));
            let saved = fs_.write_atomic(&p(path), bytes.as_bytes());
            if cfg!(windows) {
                let want = VfsError::OutsideRoot { path: p(path) };
                assert_eq!(read, Err(want.clone()), "read {path}");
                assert_eq!(saved, Err(want), "save {path}");
                assert!(!fs_.is_file(&p(path)), "is_file {path}");
                assert_eq!(fs::read(t.join("target")).unwrap(), b"old", "{path}");
            } else {
                assert!(read.is_ok(), "read {path}: {read:?}");
                assert_eq!(saved, Ok(()), "save {path}");
                assert!(fs_.is_file(&p(path)), "is_file {path}");
                assert_eq!(fs::read(t.join("target")).unwrap(), bytes.as_bytes());
            }
        }
        for (link, target) in links.iter().zip(&before) {
            assert!(is_link(&root.join(link)), "{link} is still a link");
            assert_eq!(&fs::read_link(root.join(link)).unwrap(), target, "{link}");
        }
        assert_eq!(listing(&root), ["abs", "dl", "j", "rel", "t"]);
        assert_eq!(listing(&t), ["target"]);
        // The target itself, reached without a link, saves on every platform.
        doc.buffer_mut().insert(0, "direct ").unwrap();
        doc.save(&mut fs_).unwrap();
        assert!(fs::read(t.join("target")).unwrap().starts_with(b"direct "));
    }

    /// Counterpart of `root_replaced_by_a_symlink_between_requests_is_outside` (#9; T-074
    /// Interface (4), second bullet): the root renamed away and a junction to another directory
    /// put at its path: every call `OutsideRoot`, at open and at save, the other directory
    /// unchanged; a junction at the path to the held root: the session works again. The same
    /// on every platform (Unix: a symbolic link in place of the junction).
    #[test]
    fn root_replaced_by_a_link_on_every_platform() {
        let work = fresh_dir("root-replaced-by-link");
        let root = work.join("root");
        let real = work.join("root.real");
        let other = work.join("other");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&other).unwrap();
        fs::write(root.join("f.txt"), b"in\n").unwrap();
        fs::write(other.join("f.txt"), b"other\n").unwrap();
        let mut fs_ = DiskFs::new(&root).unwrap();
        let mut doc = crate::Document::open(&fs_, &p("f.txt")).unwrap();
        doc.buffer_mut().insert(0, "PWNED ").unwrap();
        fs::rename(&root, &real).unwrap();
        junction(&other, &root);
        // The swap works: the other directory is reached through the root path now.
        assert_eq!(fs::read(root.join("f.txt")).unwrap(), b"other\n");
        assert_every_call_outside(&mut fs_, &["f.txt", "new.txt"], "replaced");
        assert_eq!(
            doc.save(&mut fs_),
            Err(VfsError::OutsideRoot { path: p("f.txt") })
        );
        assert!(doc.is_dirty());
        assert_eq!(listing(&other), ["f.txt"]);
        assert_eq!(fs::read(other.join("f.txt")).unwrap(), b"other\n");
        assert_eq!(fs::read(real.join("f.txt")).unwrap(), b"in\n");
        // A junction at the path to the held root: the session works there again.
        remove_link(&root);
        junction(&real, &root);
        assert_eq!(fs_.read(&p("f.txt")).unwrap(), b"in\n");
        doc.save(&mut fs_).unwrap();
        assert_eq!(fs::read(real.join("f.txt")).unwrap(), b"PWNED in\n");
        assert_eq!(listing(&real), ["f.txt"]);
        assert_eq!(listing(&other), ["f.txt"]);
    }

    /// Counterpart of `root_removed_and_replaced_by_a_link_to_a_new_directory_is_outside` (#11;
    /// T-074 Interface (4), third bullet and last paragraph): the held root removed while a
    /// session runs, empty (`remove_dir`) or with a file in it (`remove_dir_all`): every call
    /// `OutsideRoot`, also after a new directory takes its name, and after a junction to a new
    /// directory is put at its name; nothing there is read or written. The same on every
    /// platform (Unix: a symbolic link in place of the junction).
    #[test]
    fn root_removed_on_every_platform() {
        let work = fresh_dir("root-removed");
        // Empty, removed with `remove_dir`; then a new directory at the name.
        let root = work.join("root");
        fs::create_dir_all(&root).unwrap();
        let mut fs_ = DiskFs::new(&root).unwrap();
        fs::remove_dir(&root).unwrap();
        assert!(fs::symlink_metadata(&root).is_err());
        assert_every_call_outside(&mut fs_, &["x", "f.txt"], "removed empty");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("f.txt"), b"new dir\n").unwrap();
        assert_every_call_outside(&mut fs_, &["x", "f.txt"], "new directory at the name");
        assert_eq!(listing(&root), ["f.txt"]);
        assert_eq!(fs::read(root.join("f.txt")).unwrap(), b"new dir\n");

        // With a file, removed with `remove_dir_all`; then a junction to a new directory.
        let root2 = work.join("root2");
        fs::create_dir_all(&root2).unwrap();
        fs::write(root2.join("f.txt"), b"in\n").unwrap();
        let mut fs2 = DiskFs::new(&root2).unwrap();
        assert_eq!(fs2.read(&p("f.txt")).unwrap(), b"in\n");
        fs::remove_dir_all(&root2).unwrap();
        assert!(fs::symlink_metadata(&root2).is_err());
        assert_every_call_outside(&mut fs2, &["x", "f.txt"], "removed with a file");
        let fresh = work.join("fresh");
        fs::create_dir(&fresh).unwrap();
        fs::write(fresh.join("f.txt"), b"fresh\n").unwrap();
        junction(&fresh, &root2);
        assert_eq!(fs::read(root2.join("f.txt")).unwrap(), b"fresh\n");
        assert_every_call_outside(&mut fs2, &["x", "f.txt"], "junction to a new directory");
        assert_eq!(listing(&fresh), ["f.txt"]);
        assert_eq!(fs::read(fresh.join("f.txt")).unwrap(), b"fresh\n");
    }

    /// Counterpart of `root_given_through_a_link_that_is_retargeted` (#12; T-074 Interface (4),
    /// first bullet, D-256 Q-4): `--root` given once as a junction and once as a directory
    /// symbolic link leads to the root; pointed elsewhere while the session runs, every call
    /// `OutsideRoot` (also the save of a document opened before) and nothing read or written
    /// there; pointed back, the session works again. The same on every platform (Unix: both
    /// are symbolic links).
    #[test]
    fn root_given_through_a_link_on_every_platform() {
        for kind in ["junction", "dir-link"] {
            let work = fresh_dir(&format!("root-through-{kind}"));
            let real = work.join("real");
            let other = work.join("other");
            fs::create_dir_all(&real).unwrap();
            fs::create_dir_all(&other).unwrap();
            fs::write(real.join("f.txt"), b"real\n").unwrap();
            fs::write(other.join("f.txt"), b"other\n").unwrap();
            let link = work.join("rootlink");
            let make = |target: &Path| match kind {
                "junction" => junction(target, &link),
                _ => dir_link(target, &link),
            };
            make(&real);
            let mut fs_ = DiskFs::new(&link).unwrap();
            assert_eq!(fs_.read(&p("f.txt")).unwrap(), b"real\n", "{kind}");
            let mut doc = crate::Document::open(&fs_, &p("f.txt")).unwrap();
            doc.buffer_mut().insert(0, "PWNED ").unwrap();
            remove_link(&link);
            make(&other);
            assert_eq!(fs::read(link.join("f.txt")).unwrap(), b"other\n", "{kind}");
            assert_every_call_outside(&mut fs_, &["f.txt", "new.txt"], kind);
            assert_eq!(
                doc.save(&mut fs_),
                Err(VfsError::OutsideRoot { path: p("f.txt") }),
                "{kind}"
            );
            assert!(doc.is_dirty(), "{kind}");
            assert_eq!(listing(&other), ["f.txt"], "{kind}");
            assert_eq!(fs::read(other.join("f.txt")).unwrap(), b"other\n", "{kind}");
            // Pointed back: the session works there again.
            remove_link(&link);
            make(&real);
            assert_eq!(fs_.read(&p("f.txt")).unwrap(), b"real\n", "{kind}");
            doc.save(&mut fs_).unwrap();
            assert_eq!(
                fs::read(real.join("f.txt")).unwrap(),
                b"PWNED real\n",
                "{kind}"
            );
            assert_eq!(listing(&real), ["f.txt"], "{kind}");
            assert_eq!(listing(&other), ["f.txt"], "{kind}");
        }
    }

    /// `root/` with `a.txt`, `sub/c.txt` and `dir/`, and `outside/secret.txt` next to it.
    fn link_fixture(name: &str) -> (PathBuf, PathBuf) {
        let work = fresh_dir(name);
        let root = work.join("root");
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::create_dir_all(root.join("dir")).unwrap();
        fs::create_dir_all(work.join("outside")).unwrap();
        fs::write(root.join("a.txt"), b"alpha\n").unwrap();
        fs::write(root.join("sub/c.txt"), b"charlie\n").unwrap();
        fs::write(work.join("outside/secret.txt"), b"s3cr3t-content\n").unwrap();
        (work, root)
    }

    /// Counterpart of `check_location_is_ok_for_every_inside_location` (#14): `check_location`
    /// is `Ok` for a file, a directory, nothing and a missing parent on every platform; for a
    /// file link, a directory link and a junction leading inside, at the last and at a middle
    /// component: `Ok` on Unix (followed, inside), `OutsideRoot` on Windows. Nothing is created.
    #[test]
    fn check_location_inside_on_every_platform() {
        let (_work, root) = link_fixture("check-inside-links");
        file_link(Path::new("a.txt"), &root.join("to-a"));
        dir_link(Path::new("sub"), &root.join("dsub"));
        junction(&root.join("sub"), &root.join("jsub"));
        let fs_ = DiskFs::new(&root).unwrap();
        for path in ["a.txt", "dir", "sub", "sub/c.txt", "none.txt", "nope/x.txt"] {
            assert_eq!(fs_.check_location(&p(path)), Ok(()), "{path}");
        }
        for path in [
            "to-a",
            "dsub",
            "jsub",
            "dsub/c.txt",
            "jsub/c.txt",
            "jsub/none.txt",
        ] {
            let result = fs_.check_location(&p(path));
            if cfg!(windows) {
                assert_eq!(
                    result,
                    Err(VfsError::OutsideRoot { path: p(path) }),
                    "{path}"
                );
            } else {
                assert_eq!(result, Ok(()), "{path}");
            }
        }
        assert!(fs::symlink_metadata(root.join("none.txt")).is_err());
        assert!(fs::symlink_metadata(root.join("nope")).is_err());
        assert!(fs::symlink_metadata(root.join("sub/none.txt")).is_err());
        assert_eq!(listing(&root.join("sub")), ["c.txt"]);
    }

    /// Counterpart of `check_location_refuses_outside_loops_and_a_replaced_root` (#15): a
    /// junction to outside is `OutsideRoot` on every platform; a junction to inside is `Ok` on
    /// Unix and `OutsideRoot` on Windows; a junction loop is `Io` on Unix (too many links) and
    /// `OutsideRoot` on Windows; a root replaced by a junction to another directory is
    /// `OutsideRoot` on every platform, and moved back it is the root again.
    #[test]
    fn check_location_outside_on_every_platform() {
        let (work, root) = link_fixture("check-outside-links");
        let outside = work.join("outside");
        // Unix: absolute link targets from the canonical root, so that a loop meets no other
        // link (macOS reaches the temp dir through one) and is answered `Io` from inside.
        let anchor = if cfg!(windows) {
            root.clone()
        } else {
            root.canonicalize().unwrap()
        };
        junction(&outside, &root.join("jout"));
        junction(&anchor.join("sub"), &root.join("jin"));
        // A loop: `l1` to `l2` (a directory while `l1` is made), then `l2` a junction to `l1`.
        fs::create_dir(root.join("l2")).unwrap();
        junction(&anchor.join("l2"), &root.join("l1"));
        fs::remove_dir(root.join("l2")).unwrap();
        junction(&anchor.join("l1"), &root.join("l2"));
        let fs_ = DiskFs::new(&root).unwrap();
        for path in ["jout", "jout/secret.txt", "jout/none.txt"] {
            assert_eq!(
                fs_.check_location(&p(path)),
                Err(VfsError::OutsideRoot { path: p(path) }),
                "{path}"
            );
        }
        for path in ["jin", "jin/c.txt"] {
            let result = fs_.check_location(&p(path));
            if cfg!(windows) {
                assert_eq!(
                    result,
                    Err(VfsError::OutsideRoot { path: p(path) }),
                    "{path}"
                );
            } else {
                assert_eq!(result, Ok(()), "{path}");
            }
        }
        for path in ["l1", "l1/x"] {
            let result = fs_.check_location(&p(path));
            if cfg!(windows) {
                assert_eq!(
                    result,
                    Err(VfsError::OutsideRoot { path: p(path) }),
                    "{path}"
                );
            } else {
                assert!(
                    matches!(&result, Err(VfsError::Io { path: got, .. }) if *got == p(path)),
                    "{path}: {result:?}"
                );
            }
        }
        assert!(fs::symlink_metadata(outside.join("none.txt")).is_err());
        assert_eq!(listing(&outside), ["secret.txt"]);

        // The root renamed away and a junction to another directory at its path: outside.
        let away = work.join("root.away");
        fs::rename(&root, &away).unwrap();
        junction(&outside, &root);
        for path in ["secret.txt", "a.txt"] {
            assert_eq!(
                fs_.check_location(&p(path)),
                Err(VfsError::OutsideRoot { path: p(path) }),
                "replaced root: {path}"
            );
        }
        // Moved back: the root again.
        remove_link(&root);
        fs::rename(&away, &root).unwrap();
        assert_eq!(fs_.check_location(&p("a.txt")), Ok(()));
    }
}
