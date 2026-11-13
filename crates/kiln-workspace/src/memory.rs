//! In-memory VFS: the deterministic reference model of [`crate::DiskFs`].

use std::collections::BTreeMap;

use crate::path::RelPath;
use crate::vfs::{DirEntry, DirListing, EntryKind, Vfs, VfsError};

/// In-memory VFS for deterministic tests. A directory exists iff some file lies below it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemoryFs {
    files: BTreeMap<RelPath, Vec<u8>>,
    fail_writes: bool,
}

impl MemoryFs {
    /// An empty file system (only the root directory exists).
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the content of `path` directly (creating it, and implicitly its directories).
    pub fn insert(&mut self, path: &RelPath, bytes: &[u8]) {
        self.files.insert(path.clone(), bytes.to_vec());
    }

    /// The content of `path`, if it is a file.
    pub fn get(&self, path: &RelPath) -> Option<Vec<u8>> {
        self.files.get(path).cloned()
    }

    /// All file paths, sorted.
    pub fn paths(&self) -> Vec<RelPath> {
        self.files.keys().cloned().collect()
    }

    /// While set, every write_atomic fails with VfsError::Io (kind Other) and changes nothing.
    pub fn set_fail_writes(&mut self, fail: bool) {
        self.fail_writes = fail;
    }

    /// True iff some file lies strictly below `dir`.
    fn is_dir(&self, dir: &RelPath) -> bool {
        let prefix = format!("{dir}/");
        self.files
            .keys()
            .any(|p| p.as_str().starts_with(prefix.as_str()))
    }

    /// True iff every ancestor of `path` is a directory (the root always is).
    fn parent_is_dir(&self, path: &RelPath) -> bool {
        match path.parent() {
            None => true,
            Some(parent) => self.is_dir(&parent) && !self.files.contains_key(&parent),
        }
    }
}

impl Vfs for MemoryFs {
    fn read(&self, path: &RelPath) -> Result<Vec<u8>, VfsError> {
        if let Some(bytes) = self.files.get(path) {
            return Ok(bytes.clone());
        }
        if self.is_dir(path) {
            return Err(VfsError::NotAFile { path: path.clone() });
        }
        Err(VfsError::NotFound { path: path.clone() })
    }

    fn write_atomic(&mut self, path: &RelPath, bytes: &[u8]) -> Result<(), VfsError> {
        if self.fail_writes {
            return Err(VfsError::Io {
                path: path.clone(),
                kind: std::io::ErrorKind::Other,
                message: "simulated write failure".to_owned(),
            });
        }
        if self.is_dir(path) {
            return Err(VfsError::NotAFile { path: path.clone() });
        }
        if !self.parent_is_dir(path) {
            return Err(VfsError::NotFound { path: path.clone() });
        }
        self.files.insert(path.clone(), bytes.to_vec());
        Ok(())
    }

    fn is_file(&self, path: &RelPath) -> bool {
        self.files.contains_key(path)
    }

    /// Kinds are only `File` and `Dir`, and `skipped` is always 0 (every stored name is a valid
    /// `RelPath` segment). The root always exists; another directory exists iff a file lies
    /// below it. A file → `NotAFile`; a missing path, or one below a file → `NotFound`.
    fn list_dir(&self, dir: Option<&RelPath>) -> Result<DirListing, VfsError> {
        let prefix = match dir {
            None => String::new(),
            Some(dir) => {
                if self.files.contains_key(dir) {
                    return Err(VfsError::NotAFile { path: dir.clone() });
                }
                if !self.is_dir(dir) {
                    return Err(VfsError::NotFound { path: dir.clone() });
                }
                format!("{dir}/")
            }
        };
        // Name → kind; a name stored both as a file and as a directory (possible only through
        // `insert`) is listed once, as the file `read` returns.
        let mut kinds: BTreeMap<&str, EntryKind> = BTreeMap::new();
        for path in self.files.keys() {
            let Some(rest) = path.as_str().strip_prefix(prefix.as_str()) else {
                continue;
            };
            let (name, kind) = match rest.split_once('/') {
                Some((name, _)) => (name, EntryKind::Dir),
                None => (rest, EntryKind::File),
            };
            let slot = kinds.entry(name).or_insert(kind);
            if kind == EntryKind::File {
                *slot = EntryKind::File;
            }
        }
        Ok(DirListing {
            entries: kinds
                .into_iter()
                .map(|(name, kind)| DirEntry {
                    name: name.to_owned(),
                    kind,
                })
                .collect(),
            skipped: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> RelPath {
        RelPath::new(s).unwrap()
    }

    #[test]
    fn write_then_read_round_trips_and_creates_top_level_files() {
        let mut fs = MemoryFs::new();
        fs.write_atomic(&p("a.txt"), b"one").unwrap();
        assert_eq!(fs.read(&p("a.txt")).unwrap(), b"one");
        fs.write_atomic(&p("a.txt"), b"").unwrap();
        assert_eq!(fs.read(&p("a.txt")).unwrap(), b"");
        assert!(fs.is_file(&p("a.txt")));
    }

    #[test]
    fn directories_are_implicit() {
        let mut fs = MemoryFs::new();
        fs.insert(&p("d/e/f.rs"), b"x");
        assert!(!fs.is_file(&p("d")));
        assert_eq!(fs.read(&p("d")), Err(VfsError::NotAFile { path: p("d") }));
        assert_eq!(
            fs.write_atomic(&p("d/e"), b"y"),
            Err(VfsError::NotAFile { path: p("d/e") })
        );
        fs.write_atomic(&p("d/e/g.rs"), b"y").unwrap();
        fs.write_atomic(&p("d/h.rs"), b"z").unwrap();
        assert_eq!(fs.paths(), vec![p("d/e/f.rs"), p("d/e/g.rs"), p("d/h.rs")]);
    }

    #[test]
    fn write_into_missing_parent_is_not_found_and_creates_nothing() {
        let mut fs = MemoryFs::new();
        fs.insert(&p("file"), b"x");
        let before = fs.clone();
        assert_eq!(
            fs.write_atomic(&p("missing/a"), b"y"),
            Err(VfsError::NotFound {
                path: p("missing/a")
            })
        );
        // A file is not a directory.
        assert_eq!(
            fs.write_atomic(&p("file/a"), b"y"),
            Err(VfsError::NotFound { path: p("file/a") })
        );
        assert_eq!(fs, before);
        assert_eq!(
            fs.read(&p("missing")),
            Err(VfsError::NotFound { path: p("missing") })
        );
        assert_eq!(
            fs.read(&p("file/a")),
            Err(VfsError::NotFound { path: p("file/a") })
        );
    }

    #[test]
    fn failing_writes_change_nothing() {
        let mut fs = MemoryFs::new();
        fs.insert(&p("a"), b"old");
        fs.set_fail_writes(true);
        let err = fs.write_atomic(&p("a"), b"new").unwrap_err();
        assert!(matches!(
            err,
            VfsError::Io {
                kind: std::io::ErrorKind::Other,
                ..
            }
        ));
        assert!(fs.write_atomic(&p("b"), b"new").is_err());
        assert_eq!(fs.get(&p("a")), Some(b"old".to_vec()));
        assert_eq!(fs.get(&p("b")), None);
        fs.set_fail_writes(false);
        fs.write_atomic(&p("a"), b"new").unwrap();
        assert_eq!(fs.get(&p("a")), Some(b"new".to_vec()));
    }

    fn entry(name: &str, kind: EntryKind) -> DirEntry {
        DirEntry {
            name: name.to_owned(),
            kind,
        }
    }

    #[test]
    fn list_dir_lists_implied_directories_in_byte_order() {
        let mut fs = MemoryFs::new();
        assert_eq!(fs.list_dir(None), Ok(DirListing::default()));
        fs.insert(&p("b.txt"), b"");
        fs.insert(&p("B.txt"), b"");
        fs.insert(&p("a/b.rs"), b"");
        fs.insert(&p("a.rs"), b"");
        fs.insert(&p("a/c/d"), b"");
        fs.insert(&p("ab/x"), b"");
        let root = fs.list_dir(None).unwrap();
        assert_eq!(
            root.entries,
            vec![
                entry("B.txt", EntryKind::File),
                entry("a", EntryKind::Dir),
                entry("a.rs", EntryKind::File),
                entry("ab", EntryKind::Dir),
                entry("b.txt", EntryKind::File),
            ]
        );
        assert_eq!(root.skipped, 0);
        assert_eq!(
            fs.list_dir(Some(&p("a"))).unwrap().entries,
            vec![entry("b.rs", EntryKind::File), entry("c", EntryKind::Dir)]
        );
        assert_eq!(
            fs.list_dir(Some(&p("a.rs"))),
            Err(VfsError::NotAFile { path: p("a.rs") })
        );
        assert_eq!(
            fs.list_dir(Some(&p("a.rs/x"))),
            Err(VfsError::NotFound { path: p("a.rs/x") })
        );
        assert_eq!(
            fs.list_dir(Some(&p("nope"))),
            Err(VfsError::NotFound { path: p("nope") })
        );
    }

    #[test]
    fn sibling_with_common_prefix_is_not_a_child() {
        let mut fs = MemoryFs::new();
        fs.insert(&p("ab/c"), b"x");
        assert_eq!(fs.read(&p("a")), Err(VfsError::NotFound { path: p("a") }));
        assert!(fs.write_atomic(&p("a"), b"y").is_ok());
    }
}
