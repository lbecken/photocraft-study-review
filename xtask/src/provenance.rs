//! Stale-binary guard (T-005, D-019 (f)): `xtask` refuses to run when the checkout it was built
//! from is not the checkout it is being run in.
//!
//! Cargo names a workspace member's artifacts by its path *relative to the workspace root*, so
//! two checkouts that share a target directory produce the same file names, and mtime-based
//! freshness can then serve one checkout's `xtask` in the other (the T-005 join triage, RC-1).
//! The binary records where it was built from (`env!("CARGO_MANIFEST_DIR")`) and, before doing
//! anything, compares that with:
//!
//! 1. **launched by cargo** (`cargo xtask …`, `cargo run -p xtask`, and test processes spawned by
//!    `cargo test -p xtask`): cargo sets `CARGO_MANIFEST_DIR` (with `CARGO_PKG_NAME=xtask`) to
//!    the xtask package of the workspace it runs in; it must be the build's own package dir;
//! 2. **run directly**: if the binary sits inside `<checkout>/target/…` of a cargo workspace,
//!    that checkout must be the one it was built from.
//!
//! The one deliberate exception is the merge queue, which runs main's xtask (built in
//! `<git-common-dir>/kiln-queue/main-xtask/`) against a task worktree: it sets
//! [`RUN_IN_ENV`] to the canonical path of that worktree for the child it starts. The exception
//! applies only when the variable names exactly the workspace the command runs in (the nearest
//! `[workspace]` above the working directory); anywhere else it is ignored and the checks above
//! apply, so a value leaked to grandchildren or left in a shell excuses nothing else.

use std::path::{Path, PathBuf};

/// Set by the queue for main's xtask: the canonical workspace path it may run in.
pub const RUN_IN_ENV: &str = "KILN_XTASK_RUN_IN";

/// The xtask package directory this binary was compiled from.
pub fn built_manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn canon(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

fn is_workspace_root(dir: &Path) -> bool {
    std::fs::read_to_string(dir.join("Cargo.toml"))
        .is_ok_and(|t| t.lines().any(|l| l.trim() == "[workspace]"))
}

/// The checkout whose `target/` holds `exe` (`<checkout>/target/…/xtask`), if any.
pub fn owning_checkout(exe: &Path) -> Option<PathBuf> {
    exe.ancestors()
        .skip(1)
        .find(|a| {
            a.file_name().is_some_and(|n| n == "target")
                && a.parent().is_some_and(is_workspace_root)
        })
        .and_then(Path::parent)
        .map(Path::to_path_buf)
}

/// What the guard looks at (gathered from the process by [`Inputs::current`]).
#[derive(Clone, Debug, Default)]
pub struct Inputs {
    /// Package dir the binary was built from.
    pub built: PathBuf,
    /// `current_exe()`.
    pub exe: Option<PathBuf>,
    /// Runtime `CARGO_MANIFEST_DIR`, when `CARGO_PKG_NAME` is `xtask`.
    pub cargo_manifest_dir: Option<PathBuf>,
    /// Runtime [`RUN_IN_ENV`].
    pub run_in: Option<PathBuf>,
    /// The workspace containing the working directory.
    pub workspace: Option<PathBuf>,
}

impl Inputs {
    /// The running process's inputs.
    pub fn current() -> Self {
        let cargo_manifest_dir = (std::env::var_os("CARGO_PKG_NAME").as_deref()
            == Some(std::ffi::OsStr::new("xtask")))
        .then(|| std::env::var_os("CARGO_MANIFEST_DIR").map(PathBuf::from))
        .flatten();
        Inputs {
            built: built_manifest_dir(),
            exe: std::env::current_exe().ok(),
            cargo_manifest_dir,
            run_in: std::env::var_os(RUN_IN_ENV).map(PathBuf::from),
            workspace: crate::util::repo_root().ok(),
        }
    }
}

/// `Err(one line naming both checkouts)` if this binary must not run here.
pub fn check(i: &Inputs) -> Result<(), String> {
    let built = canon(&i.built);
    let built_ws = built
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| built.clone());
    if let (Some(r), Some(w)) = (&i.run_in, &i.workspace)
        && canon(r) == canon(w)
    {
        return Ok(()); // the queue's deliberate main-gate run
    }
    let refuse = |other: &Path, how: &str| {
        Err(format!(
            "stale xtask binary: built from {} but {how} {}; rebuild it there (`touch xtask/src/main.rs` or `cargo clean -p xtask`) and never share a target directory between checkouts",
            built_ws.display(),
            other.display()
        ))
    };
    if let Some(m) = &i.cargo_manifest_dir {
        let m = canon(m);
        if m != built {
            let ws = m.parent().map(Path::to_path_buf).unwrap_or(m);
            return refuse(&ws, "cargo is running it for the workspace");
        }
    }
    if let Some(owner) = i.exe.as_deref().and_then(owning_checkout) {
        let owner = canon(&owner);
        if owner != built_ws {
            return refuse(&owner, "it sits in the target directory of");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::testutil::TempDir;

    fn ws(root: &Path, name: &str) -> PathBuf {
        let d = root.join(name);
        std::fs::create_dir_all(d.join("xtask")).unwrap();
        std::fs::write(d.join("Cargo.toml"), "[workspace]\nmembers = [\"xtask\"]\n").unwrap();
        d
    }

    #[test]
    fn same_checkout_passes_other_checkout_is_refused() {
        let d = TempDir::new("prov");
        let (p, w) = (ws(d.path(), "primary"), ws(d.path(), "wt"));
        let own = Inputs {
            built: w.join("xtask"),
            exe: Some(w.join("target/debug/xtask")),
            cargo_manifest_dir: Some(w.join("xtask")),
            run_in: None,
            workspace: Some(w.clone()),
        };
        assert_eq!(check(&own), Ok(()));
        // RC-1: main's build served by `cargo xtask` in the worktree.
        let stale = Inputs {
            built: p.join("xtask"),
            ..own.clone()
        };
        let e = check(&stale).unwrap_err();
        assert!(!e.contains('\n'));
        assert!(e.contains(&canon(&p).display().to_string()), "{e}");
        assert!(e.contains(&canon(&w).display().to_string()), "{e}");
        // Run directly (no cargo env): the binary's location gives it away.
        let direct = Inputs {
            cargo_manifest_dir: None,
            ..stale.clone()
        };
        assert!(check(&direct).unwrap_err().contains("target directory"));
        // A queue exception for a different workspace excuses nothing.
        let leaked = Inputs {
            run_in: Some(p.clone()),
            ..stale.clone()
        };
        assert!(check(&leaked).is_err());
    }

    #[test]
    fn toy_repositories_and_the_queue_main_gate_pass() {
        let d = TempDir::new("prov-toy");
        let (w, toy) = (ws(d.path(), "wt"), ws(d.path(), "toy"));
        // A test spawned by `cargo test -p xtask` runs the binary inside a toy repository:
        // inherited CARGO_MANIFEST_DIR is the build's own package; the binary lives in its target.
        let test_child = Inputs {
            built: w.join("xtask"),
            exe: Some(w.join("target/debug/xtask")),
            cargo_manifest_dir: Some(w.join("xtask")),
            run_in: None,
            workspace: Some(toy.clone()),
        };
        assert_eq!(check(&test_child), Ok(()));
        let scrubbed = Inputs {
            cargo_manifest_dir: None,
            ..test_child.clone()
        };
        assert_eq!(check(&scrubbed), Ok(()));
        // The queue runs main's xtask (private build tree, not a workspace's target) in `w`,
        // with the director's `cargo xtask queue` environment inherited.
        let main_build = d.path().join("git/kiln-queue/main-xtask");
        std::fs::create_dir_all(main_build.join("src/xtask")).unwrap();
        let queue = Inputs {
            built: main_build.join("src/xtask"),
            exe: Some(main_build.join("target/debug/xtask")),
            cargo_manifest_dir: Some(toy.join("xtask")),
            run_in: Some(w.clone()),
            workspace: Some(w.clone()),
        };
        assert_eq!(check(&queue), Ok(()));
        assert!(
            check(&Inputs {
                run_in: None,
                ..queue
            })
            .is_err(),
            "only with the exception"
        );
    }

    #[test]
    fn owner_of_a_binary() {
        let d = TempDir::new("prov-owner");
        let w = ws(d.path(), "wt");
        assert_eq!(
            owning_checkout(&w.join("target/debug/xtask")),
            Some(w.clone())
        );
        assert_eq!(
            owning_checkout(&w.join("target/x86_64/debug/deps/xtask-1")),
            Some(w)
        );
        assert_eq!(
            owning_checkout(&d.path().join("elsewhere/target/debug/xtask")),
            None
        );
        assert_eq!(owning_checkout(Path::new("/usr/bin/xtask")), None);
    }

    #[test]
    fn this_binary_is_built_from_this_checkout() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        assert!(is_workspace_root(root));
        assert_eq!(built_manifest_dir(), Path::new(env!("CARGO_MANIFEST_DIR")));
    }
}
