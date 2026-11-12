//! Validated paths relative to a file-system root.

use std::fmt;

/// A validated path relative to a file-system root: non-empty, `/`-separated, no leading `/`,
/// no empty, `.` or `..` segment, no `\` and no NUL. Display/as_str give it exactly as given.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelPath(String);

/// Why a string was rejected as a [`RelPath`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathError {
    /// The rejected input, as given.
    pub path: String,
    /// A human-readable reason.
    pub reason: String,
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid relative path {:?}: {}", self.path, self.reason)
    }
}

impl std::error::Error for PathError {}

impl RelPath {
    /// Validate `path` and wrap it.
    pub fn new(path: &str) -> Result<RelPath, PathError> {
        let reject = |reason: &str| {
            Err(PathError {
                path: path.to_owned(),
                reason: reason.to_owned(),
            })
        };
        if path.is_empty() {
            return reject("path is empty");
        }
        if path.contains('\0') {
            return reject("path contains a NUL character");
        }
        if path.contains('\\') {
            return reject("path contains a backslash");
        }
        if path.starts_with('/') {
            return reject("path starts with '/'");
        }
        for segment in path.split('/') {
            match segment {
                "" => return reject("path has an empty segment"),
                "." => return reject("path has a '.' segment"),
                ".." => return reject("path has a '..' segment"),
                _ => {}
            }
        }
        Ok(RelPath(path.to_owned()))
    }

    /// The path exactly as given to [`RelPath::new`].
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// `(root)`: the path used in errors about a file-system root itself, which has no valid
    /// `RelPath` and whose host path must not appear in error text.
    pub(crate) fn root_placeholder() -> RelPath {
        RelPath("(root)".to_owned())
    }

    /// The `/`-separated segments.
    pub(crate) fn segments(&self) -> impl Iterator<Item = &str> {
        self.0.split('/')
    }

    /// The parent directory, or `None` for a path directly below the root.
    pub(crate) fn parent(&self) -> Option<RelPath> {
        self.0.rfind('/').map(|i| RelPath(self.0[..i].to_owned()))
    }
}

impl fmt::Display for RelPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_paths_verbatim() {
        for p in [
            "a",
            "a/b.rs",
            "src/main.rs",
            "a b/c",
            ".hidden",
            "a/..b",
            "...",
            "é/😀",
        ] {
            let r = RelPath::new(p).unwrap();
            assert_eq!(r.as_str(), p);
            assert_eq!(r.to_string(), p);
        }
    }

    #[test]
    fn rejects_invalid_paths() {
        for p in [
            "", "/a", "a/", "a//b", "./a", "a/../b", "..", ".", "a/.", "a\\b", "a\0b", "/",
        ] {
            let err = RelPath::new(p).unwrap_err();
            assert_eq!(err.path, p);
            assert!(!err.reason.is_empty());
        }
    }

    #[test]
    fn parent_and_segments() {
        let r = RelPath::new("a/b/c.rs").unwrap();
        assert_eq!(r.parent(), Some(RelPath::new("a/b").unwrap()));
        assert_eq!(r.segments().collect::<Vec<_>>(), ["a", "b", "c.rs"]);
        assert_eq!(RelPath::new("x").unwrap().parent(), None);
        assert_eq!(RelPath::new("(root)").unwrap(), RelPath::root_placeholder());
    }
}
