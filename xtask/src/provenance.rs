//! Stale-binary guard (T-005, D-019 (f)): `xtask` refuses to run when the checkout it was built
//! from and the workspace it is run in are **different working trees of the same git
//! repository** (same common git directory).
//!
//! Cargo names a workspace member's artifacts by its path *relative to the workspace root*, so
//! two checkouts that share a target directory produce the same file names, and mtime-based
//! freshness can then serve one checkout's `xtask` in the other (the T-005 join triage, RC-1).
//! The binary records where it was built from (`env!("CARGO_MANIFEST_DIR")`, the xtask package;
//! its parent is the workspace). Before doing anything it resolves:
//!
//! - *built*: the git working tree whose top level is that workspace;
//! - *run*: the git working tree whose top level is the cargo workspace above the working
//!   directory.
//!
//! It refuses (exit 2, one line naming both) only if both exist, differ, and share a common git
//! directory. In its own checkout it runs; in an unrelated repository (the toy repositories tests
//! create) it runs; if either side is not a git working tree it runs.
//!
//! The queue's deliberate run of main's xtask against a task worktree needs no exception and has
//! none (D-026 R8): main's xtask is built in `<git-common-dir>/kiln-queue/main-xtask/src`, an
//! export inside the git directory that is not a working tree, so *built* does not exist and the
//! guard lets it run. There is no environment variable or flag that relaxes the guard.

use std::path::{Path, PathBuf};

/// The xtask package directory this binary was compiled from.
pub fn built_manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn canon(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// `git rev-parse` in `dir`, isolated from inherited `GIT_*` repository variables.
fn rev_parse(dir: &Path, what: &str) -> Option<PathBuf> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", what])
        .current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_INDEX_FILE")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    let p = PathBuf::from(&s);
    Some(canon(&if p.is_absolute() { p } else { dir.join(p) }))
}

/// A git working tree: its top level and common git directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tree {
    /// `git rev-parse --show-toplevel`.
    pub top: PathBuf,
    /// `git rev-parse --git-common-dir`.
    pub common: PathBuf,
}

/// The working tree whose top level is exactly `dir` (`None` if `dir` is not one).
pub fn tree_at(dir: &Path) -> Option<Tree> {
    let top = rev_parse(dir, "--show-toplevel")?;
    if top != canon(dir) {
        return None;
    }
    Some(Tree {
        common: rev_parse(dir, "--git-common-dir")?,
        top,
    })
}

/// What the guard looks at (gathered from the process by [`Inputs::current`]).
#[derive(Clone, Debug, Default)]
pub struct Inputs {
    /// Package dir the binary was built from.
    pub built: PathBuf,
    /// The cargo workspace containing the working directory.
    pub workspace: Option<PathBuf>,
}

impl Inputs {
    /// The running process's inputs.
    pub fn current() -> Self {
        Inputs {
            built: built_manifest_dir(),
            workspace: crate::util::repo_root().ok(),
        }
    }
}

/// `Err(one line naming both checkouts)` if this binary must not run here.
pub fn check(i: &Inputs) -> Result<(), String> {
    let Some(ws) = &i.workspace else {
        return Ok(());
    };
    let built_ws = i.built.parent().unwrap_or(&i.built);
    let (Some(built), Some(run)) = (tree_at(built_ws), tree_at(ws)) else {
        return Ok(());
    };
    if built.top != run.top && built.common == run.common {
        return Err(format!(
            "stale xtask binary: built from {} but run in {} (another working tree of the same repository); rebuild it there (`touch xtask/src/main.rs` or `cargo clean -p xtask`) and never share a target directory between checkouts",
            built.top.display(),
            run.top.display()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::testrepo::{self, g};

    fn inputs(built_ws: &Path, ws: &Path) -> Inputs {
        Inputs {
            built: built_ws.join("xtask"),
            workspace: Some(ws.to_path_buf()),
        }
    }

    #[test]
    fn other_worktree_of_the_same_repository_is_refused() {
        let repo = testrepo::init("prov", &[("Cargo.toml", "[workspace]\n"), ("xtask/x", "")]);
        let p = repo.path();
        let w = p.join(".worktrees/T-001");
        g(
            p,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "task/T-001",
                &w.to_string_lossy(),
                "main",
            ],
        );
        assert_eq!(check(&inputs(p, p)), Ok(()), "own checkout");
        assert_eq!(check(&inputs(&w, &w)), Ok(()), "own worktree");
        let e = check(&inputs(p, &w)).unwrap_err();
        assert!(!e.contains('\n'));
        assert!(e.contains(&canon(p).display().to_string()), "{e}");
        assert!(e.contains(&canon(&w).display().to_string()), "{e}");
        assert!(check(&inputs(&w, p)).is_err(), "the reverse too");
    }

    #[test]
    fn unrelated_repositories_and_non_worktrees_run() {
        let a = testrepo::init("prov-a", &[("Cargo.toml", "[workspace]\n")]);
        let toy = testrepo::init("prov-toy", &[("Cargo.toml", "[workspace]\n")]);
        assert_eq!(check(&inputs(a.path(), toy.path())), Ok(()));
        // Main's xtask in the queue's private export inside the git directory: not a working tree.
        let export = a.path().join(".git/kiln-queue/main-xtask/src");
        std::fs::create_dir_all(export.join("xtask")).unwrap();
        assert_eq!(tree_at(&export), None);
        assert_eq!(check(&inputs(&export, a.path())), Ok(()));
        // A subdirectory is not a working tree's top level; no workspace: nothing to compare.
        assert_eq!(tree_at(&a.path().join(".git")), None);
        assert_eq!(check(&Inputs::default()), Ok(()));
    }

    #[test]
    fn no_variable_relaxes_the_guard() {
        // D-026 R8: the queue's old bypass variable is gone from the guard and the queue, and
        // the guard reads no environment at all.
        let old = concat!("KILN_", "XTASK_RUN_IN");
        for (name, src) in [
            ("provenance.rs", include_str!("provenance.rs")),
            ("queue.rs", include_str!("queue.rs")),
        ] {
            assert!(!src.contains(old), "{name} still mentions {old}");
        }
        let guard = include_str!("provenance.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        assert!(!guard.contains(concat!("env::", "var")), "the guard reads no variable");
    }

    #[test]
    fn this_binary_records_its_checkout() {
        assert_eq!(built_manifest_dir(), Path::new(env!("CARGO_MANIFEST_DIR")));
        let i = Inputs {
            built: built_manifest_dir(),
            workspace: built_manifest_dir().parent().map(Path::to_path_buf),
        };
        assert_eq!(check(&i), Ok(()));
    }
}
