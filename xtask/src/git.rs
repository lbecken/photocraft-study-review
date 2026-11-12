//! Thin wrappers around the `git` CLI.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Result, bail};

use crate::util::capture;

/// A `git` command running in `dir`.
pub fn cmd(dir: &Path) -> Command {
    let mut c = Command::new("git");
    c.current_dir(dir);
    c
}

/// Run git in `dir`; return trimmed stdout or an error carrying stderr.
pub fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let out = capture(cmd(dir).args(args))?;
    if !out.ok {
        bail!(
            "git {} failed in {}: {}",
            args.join(" "),
            dir.display(),
            out.stderr.trim()
        );
    }
    Ok(out.stdout.trim().to_owned())
}

/// Run git in `dir`; return whether it succeeded, plus stdout and stderr.
pub fn git_try(dir: &Path, args: &[&str]) -> Result<(bool, String, String)> {
    let out = capture(cmd(dir).args(args))?;
    Ok((out.ok, out.stdout, out.stderr))
}

/// The sha `rev` resolves to.
pub fn rev(dir: &Path, rev: &str) -> Result<String> {
    git(
        dir,
        &["rev-parse", "--verify", &format!("{rev}^{{commit}}")],
    )
}

/// Whether branch `name` exists.
pub fn branch_exists(dir: &Path, name: &str) -> Result<bool> {
    Ok(git_try(
        dir,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/heads/{name}"),
        ],
    )?
    .0)
}

/// `git status --porcelain` lines (untracked included), optionally ignoring paths under
/// `ignore_prefix`.
pub fn dirty_paths(dir: &Path, ignore_prefix: Option<&str>) -> Result<Vec<String>> {
    // Not `git()`: trimming would eat the leading space of the first ` M path` line.
    let (ok, out, err) = git_try(dir, &["status", "--porcelain", "--untracked-files=all"])?;
    if !ok {
        bail!("git status failed in {}: {}", dir.display(), err.trim());
    }
    Ok(out
        .lines()
        .filter(|l| l.len() > 3)
        .filter(|l| ignore_prefix.is_none_or(|p| !l[3..].starts_with(p)))
        .map(str::to_owned)
        .collect())
}

/// The primary worktree (the checkout that owns `.git`), from any worktree.
pub fn primary_worktree(dir: &Path) -> Result<PathBuf> {
    let out = git(dir, &["worktree", "list", "--porcelain"])?;
    out.lines()
        .find_map(|l| l.strip_prefix("worktree "))
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("git worktree list returned nothing"))
}

/// The commit date of HEAD as `YYYY-MM-DD` (UTC).
pub fn head_date(dir: &Path) -> Result<String> {
    let secs: i64 = git(dir, &["log", "-1", "--format=%ct"])?.parse()?;
    Ok(crate::timefmt::date_of(secs))
}

#[cfg(test)]
pub mod testrepo {
    //! Deterministic throwaway git repositories for tests.
    use super::*;
    use crate::util::testutil::TempDir;

    /// Run git with a fixed identity and dates, isolated from user/system config.
    pub fn g(dir: &Path, args: &[&str]) -> String {
        let out = cmd(dir)
            .args(args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
            .env("GIT_AUTHOR_DATE", "2026-01-01T00:00:00Z")
            .env("GIT_COMMITTER_DATE", "2026-01-01T00:00:00Z")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    }

    /// A repo on `main` with one commit containing `files`.
    pub fn init(tag: &str, files: &[(&str, &str)]) -> TempDir {
        let d = TempDir::new(tag);
        g(d.path(), &["init", "-q", "-b", "main"]);
        g(d.path(), &["config", "user.name", "Test"]);
        g(d.path(), &["config", "user.email", "test@example.invalid"]);
        g(d.path(), &["config", "commit.gpgsign", "false"]);
        for (p, c) in files {
            write(d.path(), p, c);
        }
        g(d.path(), &["add", "-A"]);
        g(d.path(), &["commit", "-q", "-m", "init"]);
        d
    }

    /// Write a file (creating directories).
    pub fn write(dir: &Path, path: &str, content: &str) {
        let p = dir.join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    }

    /// Write files and commit them in `dir`.
    pub fn commit(dir: &Path, msg: &str, files: &[(&str, &str)]) -> String {
        for (p, c) in files {
            write(dir, p, c);
        }
        g(dir, &["add", "-A"]);
        g(dir, &["commit", "-q", "-m", msg]);
        g(dir, &["rev-parse", "HEAD"])
    }
}
