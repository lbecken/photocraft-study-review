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

/// Files only; directories exist implicitly (MemoryFs) or on disk (DiskFs).
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
}
