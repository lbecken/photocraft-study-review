//! The virtual file system trait and its error type.

use std::fmt;

use crate::path::RelPath;

/// File-system errors. Display names the RelPath only, never an absolute or host path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VfsError {
    /// The file (or, for a write, its parent directory) does not exist.
    NotFound {
        /// The path the operation was asked about.
        path: RelPath,
    },
    /// The location of the path is not a regular file (a directory, FIFO, socket or device).
    NotAFile {
        /// The path the operation was asked about.
        path: RelPath,
    },
    /// The location of the path (after following every symlink on it) is outside the root.
    OutsideRoot {
        /// The path the operation was asked about.
        path: RelPath,
    },
    /// Any other I/O failure.
    Io {
        /// The path the operation was asked about.
        path: RelPath,
        /// The kind of the underlying I/O error.
        kind: std::io::ErrorKind,
        /// A description of the failure that names no host path.
        message: String,
    },
}

impl VfsError {
    /// The path the failed operation was asked about.
    pub fn path(&self) -> &RelPath {
        match self {
            VfsError::NotFound { path }
            | VfsError::NotAFile { path }
            | VfsError::OutsideRoot { path }
            | VfsError::Io { path, .. } => path,
        }
    }
}

impl fmt::Display for VfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VfsError::NotFound { path } => write!(f, "{path}: not found"),
            VfsError::NotAFile { path } => write!(f, "{path}: not a regular file"),
            VfsError::OutsideRoot { path } => write!(f, "{path}: resolves outside the root"),
            VfsError::Io { path, message, .. } => write!(f, "{path}: I/O error: {message}"),
        }
    }
}

impl std::error::Error for VfsError {}

/// The kind of a directory entry, taken from the entry itself (a symlink is never followed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    /// A regular file.
    File,
    /// A directory.
    Dir,
    /// A symbolic link, whatever it points to.
    Symlink,
    /// Anything else: FIFO, socket, character or block device.
    Other,
}

/// One entry of a directory listing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirEntry {
    /// The entry's name (one path segment).
    pub name: String,
    /// The entry's own kind.
    pub kind: EntryKind,
}

/// A directory listing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DirListing {
    /// The entries, sorted by the bytes of their names.
    pub entries: Vec<DirEntry>,
    /// How many names were omitted because they are not valid UTF-8 or not a valid `RelPath`
    /// segment (for example a name containing `\`).
    pub skipped: usize,
}

/// Files, and directory listings; directories exist implicitly (MemoryFs) or on disk (DiskFs).
pub trait Vfs: std::fmt::Debug {
    /// The whole content of the file at `path`.
    fn read(&self, path: &RelPath) -> Result<Vec<u8>, VfsError>;
    /// Replace the file's whole content atomically: afterwards the file holds exactly `bytes`,
    /// or (on Err) exactly what it held before (or still does not exist). Creates the file if
    /// absent; never creates directories.
    fn write_atomic(&mut self, path: &RelPath, bytes: &[u8]) -> Result<(), VfsError>;
    /// True iff `path` names an existing regular file (for `DiskFs`: whose location is inside
    /// the root).
    fn is_file(&self, path: &RelPath) -> bool;
    /// List `dir` (None = the root). `skipped` counts names that are not valid UTF-8 or not a
    /// valid RelPath segment (for example a name containing `\`); those are omitted.
    ///
    /// Errors: `NotFound` for a missing directory (or one below a file), `NotAFile` when the
    /// location is a file or another non-directory, and for `DiskFs` `OutsideRoot` and `Io` as
    /// for `read`. An error about the root itself names the placeholder path `(root)`.
    fn list_dir(&self, dir: Option<&RelPath>) -> Result<DirListing, VfsError>;
}
