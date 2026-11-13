//! Project tree (T-019): the project's directories and files, listed through [`Vfs::list_dir`].
//!
//! [`tree`] walks a directory of the project (default: the root) and lists what is below it
//! in preorder, each directory's entries in byte order of their names, by these rules
//! (`docs/protocol/project.md`), applied per entry in this order:
//! 1. `under` itself is not listed and is not subject to rules 2–3. Entry paths begin with
//!    `under` as given (also when `under` is reached through an in-root symlink).
//! 2. The entry whose path is exactly `target` when it is a directory, and every entry named
//!    `.git` whatever its kind, are skipped: counted in `ignored`, not descended.
//! 3. `.gitignore` files: one of kind [`EntryKind::File`] in a visited directory, and that of
//!    the root and of every directory on `under`'s lexical prefix (when `Vfs::list_dir` of its
//!    directory reports it as a file), apply to that directory's subtree, as in git: patterns
//!    relative to the file's directory, a deeper file overriding a shallower one, the last
//!    matching pattern of a file deciding. An ignored entry is counted in `ignored` and not
//!    descended (so nothing below an ignored directory can be re-included). A `.gitignore` is
//!    read through `Vfs::read` even when a pattern ignores it (ruling Q-8); one of another kind
//!    is never read. A leading UTF-8 byte-order mark is dropped; lines that are not UTF-8 and
//!    lines the matcher refuses are skipped. No other source is consulted.
//! 4. Symlinks and other special entries are listed with their kind and never descended.
//! 5. A directory whose path has [`MAX_DEPTH`] segments (or more, when `under` is that deep) is
//!    listed, not descended, and counted in `too_deep`.
//! 6. The walk stops when it would list entry `limit + 1`; then `truncated` is true.
//!
//! Only the gitignore *matcher* of the `ignore` crate is used; every directory and every
//! `.gitignore` is read through the [`Vfs`], so `DiskFs` confinement applies to all of them.

use ignore::gitignore::{Gitignore, GitignoreBuilder};

use crate::path::RelPath;
use crate::vfs::{DirListing, EntryKind, Vfs, VfsError};

/// A directory whose path has this many segments is listed but not descended (rule 5).
pub const MAX_DEPTH: usize = 128;

/// The name of the pattern files the walk reads.
const GITIGNORE: &str = ".gitignore";

/// One listed entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeEntry {
    /// Root-relative path, beginning with `under` as given.
    pub path: RelPath,
    /// The entry's own kind (a symlink is never followed).
    pub kind: EntryKind,
}

/// The result of [`tree`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectTree {
    /// The listed entries, in preorder; each directory's entries in byte order of their names.
    pub entries: Vec<TreeEntry>,
    /// Entries skipped by rule 2 (`target`, `.git`) or ignored by a `.gitignore` (rule 3), each
    /// counted once; their contents are not visited.
    pub ignored: usize,
    /// Names omitted by `Vfs::list_dir` (not UTF-8, or not a valid path segment) in the
    /// directories visited.
    pub skipped: usize,
    /// Directories listed but not descended because of the depth bound (rule 5).
    pub too_deep: usize,
    /// True iff an entry beyond `limit` exists (the walk stopped there).
    pub truncated: bool,
}

/// List the project below `under` (None: the root), at most `limit` entries.
///
/// Errors: those of `Vfs::list_dir` for `under` itself first (for example `NotAFile` when it
/// is a file); then an error listing a visited directory, or one on `under`'s prefix, or
/// reading a `.gitignore` of kind `File`, fails the whole walk and names that entry's path.
pub fn tree(vfs: &dyn Vfs, under: Option<&RelPath>, limit: usize) -> Result<ProjectTree, VfsError> {
    let listing = vfs.list_dir(under)?;
    let mut walk = Walk {
        vfs,
        limit,
        layers: Vec::new(),
        out: ProjectTree {
            entries: Vec::new(),
            ignored: 0,
            skipped: 0,
            too_deep: 0,
            truncated: false,
        },
    };
    if let Some(under) = under {
        // The `.gitignore` of the root and of every directory strictly above `under`, each
        // at its lexical path; `under`'s own is read when it is visited.
        let segments: Vec<&str> = under.as_str().split('/').collect();
        for depth in 0..segments.len() {
            let dir = match depth {
                0 => None,
                _ => Some(rel(&segments[..depth].join("/"))?),
            };
            let prefix_listing = vfs.list_dir(dir.as_ref())?;
            walk.push_gitignore(dir, &prefix_listing)?;
        }
    }
    walk.visit(under, &listing)?;
    Ok(walk.out)
}

/// The walk has listed `limit` entries and met one more.
struct Stop;

/// One `.gitignore` in force: its directory (lexical; None: the root) and its patterns.
struct Layer {
    dir: Option<RelPath>,
    matcher: Gitignore,
}

struct Walk<'a> {
    vfs: &'a dyn Vfs,
    limit: usize,
    /// The `.gitignore` files in force, shallowest first.
    layers: Vec<Layer>,
    out: ProjectTree,
}

impl Walk<'_> {
    /// Read the `.gitignore` of `dir` if its listing reports one of kind `File`, and put its
    /// patterns in force. Returns whether a layer was pushed.
    fn push_gitignore(
        &mut self,
        dir: Option<RelPath>,
        listing: &DirListing,
    ) -> Result<bool, VfsError> {
        let is_file = listing
            .entries
            .iter()
            .any(|e| e.name == GITIGNORE && e.kind == EntryKind::File);
        if !is_file {
            return Ok(false);
        }
        let path = child(dir.as_ref(), GITIGNORE)?;
        let bytes = self.vfs.read(&path)?;
        let matcher = parse_gitignore(&path, &bytes)?;
        self.layers.push(Layer { dir, matcher });
        Ok(true)
    }

    /// Whether `path` (a directory iff `is_dir`) is ignored by the `.gitignore` files in force:
    /// the deepest file with a matching pattern decides, and within it the last match.
    fn is_ignored(&self, path: &RelPath, is_dir: bool) -> bool {
        for layer in self.layers.iter().rev() {
            let relative = match &layer.dir {
                None => Some(path.as_str()),
                Some(dir) => path
                    .as_str()
                    .strip_prefix(dir.as_str())
                    .and_then(|rest| rest.strip_prefix('/')),
            };
            let Some(relative) = relative else { continue };
            let found = layer.matcher.matched(relative, is_dir);
            if found.is_ignore() {
                return true;
            }
            if found.is_whitelist() {
                return false;
            }
        }
        false
    }

    /// Visit the directory `dir` (None: the root), whose listing is `listing`.
    fn visit(
        &mut self,
        dir: Option<&RelPath>,
        listing: &DirListing,
    ) -> Result<Option<Stop>, VfsError> {
        let pushed = self.push_gitignore(dir.cloned(), listing)?;
        self.out.skipped += listing.skipped;
        let result = self.visit_entries(dir, listing);
        if pushed {
            self.layers.pop();
        }
        result
    }

    fn visit_entries(
        &mut self,
        dir: Option<&RelPath>,
        listing: &DirListing,
    ) -> Result<Option<Stop>, VfsError> {
        for entry in &listing.entries {
            let path = child(dir, &entry.name)?;
            let is_dir = entry.kind == EntryKind::Dir;
            // Rule 2.
            if entry.name == ".git" || (is_dir && path.as_str() == "target") {
                self.out.ignored += 1;
                continue;
            }
            // Rule 3.
            if self.is_ignored(&path, is_dir) {
                self.out.ignored += 1;
                continue;
            }
            // Rule 4: only directories are descended. Rule 5: nor those at the depth bound.
            let mut descend = is_dir;
            if is_dir && path.as_str().split('/').count() >= MAX_DEPTH {
                self.out.too_deep += 1;
                descend = false;
            }
            // Rule 6.
            if self.out.entries.len() >= self.limit {
                self.out.truncated = true;
                return Ok(Some(Stop));
            }
            self.out.entries.push(TreeEntry {
                path: path.clone(),
                kind: entry.kind,
            });
            if descend {
                let sub = self.vfs.list_dir(Some(&path))?;
                if let Some(stop) = self.visit(Some(&path), &sub)? {
                    return Ok(Some(stop));
                }
            }
        }
        Ok(None)
    }
}

/// The patterns of the `.gitignore` at `path`, matched relative to its directory: a leading
/// UTF-8 byte-order mark is dropped, lines that are not UTF-8 and lines the matcher refuses are
/// skipped. Nothing but `bytes` is read.
fn parse_gitignore(path: &RelPath, bytes: &[u8]) -> Result<Gitignore, VfsError> {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    // Paths are matched relative to the file's directory; a root of `.` strips nothing.
    let mut builder = GitignoreBuilder::new(".");
    for line in bytes.split(|&b| b == b'\n') {
        if let Ok(line) = std::str::from_utf8(line) {
            // A refused line is skipped (ruling Q-5); the builder is unchanged by a refusal.
            let _ = builder.add_line(None, line);
        }
    }
    builder.build().map_err(|e| VfsError::Io {
        path: path.clone(),
        kind: std::io::ErrorKind::InvalidData,
        message: format!("the patterns cannot be used: {e}"),
    })
}

fn rel(path: &str) -> Result<RelPath, VfsError> {
    RelPath::new(path).map_err(|e| VfsError::Io {
        path: RelPath::root_placeholder(),
        kind: std::io::ErrorKind::InvalidInput,
        message: e.to_string(),
    })
}

/// The path of `name` in `dir` (None: the root). `name` is a valid segment (`Vfs::list_dir`
/// omits every other name), so this does not fail in practice.
fn child(dir: Option<&RelPath>, name: &str) -> Result<RelPath, VfsError> {
    match dir {
        None => rel(name),
        Some(dir) => rel(&format!("{dir}/{name}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryFs;

    fn p(s: &str) -> RelPath {
        RelPath::new(s).unwrap()
    }

    fn memory(files: &[(&str, &str)]) -> MemoryFs {
        let mut fs = MemoryFs::new();
        for (path, content) in files {
            fs.insert(&p(path), content.as_bytes());
        }
        fs
    }

    fn listed(t: &ProjectTree) -> Vec<(String, EntryKind)> {
        t.entries
            .iter()
            .map(|e| (e.path.to_string(), e.kind))
            .collect()
    }

    #[test]
    fn worked_example() {
        let fs = memory(&[
            (".gitignore", "*.log\n/target\n"),
            ("B.txt", ""),
            ("a.rs", ""),
            ("a/b.rs", ""),
            ("x.log", ""),
            ("target/debug/k", ""),
            ("src/lib.rs", ""),
            ("src/.hidden", ""),
        ]);
        let t = tree(&fs, None, 50_000).unwrap();
        use EntryKind::{Dir, File};
        let want: Vec<(String, EntryKind)> = [
            (".gitignore", File),
            ("B.txt", File),
            ("a", Dir),
            ("a/b.rs", File),
            ("a.rs", File),
            ("src", Dir),
            ("src/.hidden", File),
            ("src/lib.rs", File),
        ]
        .iter()
        .map(|(p, k)| (p.to_string(), *k))
        .collect();
        assert_eq!(listed(&t), want);
        assert_eq!(
            (t.ignored, t.skipped, t.too_deep, t.truncated),
            (2, 0, 0, false)
        );
    }

    #[test]
    fn limit_edges() {
        let fs = memory(&[("a", ""), ("b", ""), (".git/x", ""), ("c", "")]);
        let t = tree(&fs, None, 3).unwrap();
        assert_eq!(t.entries.len(), 3);
        assert!(!t.truncated);
        assert_eq!(t.ignored, 1);
        let t = tree(&fs, None, 2).unwrap();
        assert_eq!(t.entries.len(), 2);
        assert!(t.truncated);
    }

    #[test]
    fn negation_and_deeper_files() {
        let fs = memory(&[
            (".gitignore", "*.log\n"),
            ("a.log", ""),
            ("sub/.gitignore", "!keep.log\n"),
            ("sub/keep.log", ""),
            ("sub/drop.log", ""),
        ]);
        let t = tree(&fs, None, 100).unwrap();
        let paths: Vec<String> = listed(&t).into_iter().map(|(p, _)| p).collect();
        assert_eq!(
            paths,
            [".gitignore", "sub", "sub/.gitignore", "sub/keep.log"]
        );
        assert_eq!(t.ignored, 2);
    }

    #[test]
    fn under_uses_prefix_gitignore_and_is_not_itself_subject() {
        let fs = memory(&[
            (".gitignore", "build/\n/a/b/x\n"),
            ("a/b/x", ""),
            ("a/b/y", ""),
            ("build/out", ""),
        ]);
        let t = tree(&fs, Some(&p("a/b")), 100).unwrap();
        let paths: Vec<String> = listed(&t).into_iter().map(|(p, _)| p).collect();
        assert_eq!(paths, ["a/b/y"]);
        let t = tree(&fs, Some(&p("build")), 100).unwrap();
        let paths: Vec<String> = listed(&t).into_iter().map(|(p, _)| p).collect();
        assert_eq!(paths, ["build/out"]);
        assert_eq!(
            tree(&fs, Some(&p("a/b/x")), 100),
            Err(VfsError::NotAFile { path: p("a/b/x") })
        );
    }

    #[test]
    fn bom_and_invalid_lines() {
        let mut fs = MemoryFs::new();
        fs.insert(&p(".gitignore"), b"\xEF\xBB\xBF*.log\n\xff*.rs\n[\n");
        fs.insert(&p("a.log"), b"");
        fs.insert(&p("b.rs"), b"");
        let t = tree(&fs, None, 100).unwrap();
        let paths: Vec<String> = listed(&t).into_iter().map(|(p, _)| p).collect();
        assert_eq!(paths, [".gitignore", "b.rs"]);
    }
}
