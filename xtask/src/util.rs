//! Small helpers shared by the subcommands: repository root, child processes, file walking.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context as _, Result, bail};

/// Outcome of a step or check: pass, fail, or "nothing registered" (reported, never silent).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// The check ran and holds.
    Pass,
    /// The check ran and found a problem (or could not run).
    Fail,
    /// The step exists but has nothing registered to check yet.
    Empty,
}

/// What a check reports: status plus a human-readable detail (one or more lines).
#[derive(Clone, Debug)]
pub struct StepResult {
    /// Pass / fail / empty.
    pub status: Status,
    /// Summary line(s).
    pub detail: String,
}

impl StepResult {
    /// A passing result.
    pub fn pass(detail: impl Into<String>) -> Self {
        Self {
            status: Status::Pass,
            detail: detail.into(),
        }
    }
    /// A failing result.
    pub fn fail(detail: impl Into<String>) -> Self {
        Self {
            status: Status::Fail,
            detail: detail.into(),
        }
    }
    /// An empty-registry result.
    pub fn empty(detail: impl Into<String>) -> Self {
        Self {
            status: Status::Empty,
            detail: detail.into(),
        }
    }
    /// Pass if `problems` is empty, else fail listing them.
    pub fn from_problems(ok: impl Into<String>, what: &str, problems: &[String]) -> Self {
        if problems.is_empty() {
            Self::pass(ok)
        } else {
            let mut d = format!("{} {what}:", problems.len());
            for p in problems {
                d.push_str("\n  - ");
                d.push_str(p);
            }
            Self::fail(d)
        }
    }
}

/// A refusal: printed as its one line on stderr, exit 1, nothing written (T-013 Interface).
/// The text carries its own prefix (`metrics: refused: …`, `worktree: refused: …`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused(pub String);

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let one: Vec<&str> = self
            .0
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        f.write_str(&one.join("; "))
    }
}

impl std::error::Error for Refused {}

/// The workspace root: the nearest ancestor of the current directory whose `Cargo.toml`
/// declares `[workspace]`.
pub fn repo_root() -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("current directory")?;
    for dir in cwd.ancestors() {
        let manifest = dir.join("Cargo.toml");
        if let Ok(text) = std::fs::read_to_string(&manifest)
            && text.lines().any(|l| l.trim() == "[workspace]")
        {
            return Ok(dir.to_path_buf());
        }
    }
    bail!("no workspace Cargo.toml above {}", cwd.display())
}

/// Captured output of a child process.
#[derive(Debug)]
pub struct Captured {
    /// Exit status was success.
    pub ok: bool,
    /// Standard output (lossy UTF-8).
    pub stdout: String,
    /// Standard error (lossy UTF-8).
    pub stderr: String,
}

/// Run `cmd`, capturing stdout and stderr.
pub fn capture(cmd: &mut Command) -> Result<Captured> {
    let out = cmd
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("failed to start {cmd:?}"))?;
    Ok(Captured {
        ok: out.status.success(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    })
}

/// A `cargo` command running in `dir`.
pub fn cargo(dir: &Path) -> Command {
    let mut c = Command::new("cargo");
    c.current_dir(dir);
    c
}

/// The last `n` lines of `text`.
pub fn tail(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

/// `path` relative to `root`, with `/` separators.
pub fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// All `.rs` files under `dir`, sorted; skips `target`, `.git`, `.worktrees` and any directory
/// whose path relative to `root` is in `exclude`.
pub fn rust_files(root: &Path, dir: &Path, exclude: &[&str]) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        if !d.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(&d).with_context(|| format!("read {}", d.display()))? {
            let path = entry?.path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if path.is_dir() {
                let r = rel(root, &path);
                if !matches!(name, "target" | ".git" | ".worktrees")
                    && !exclude.contains(&r.as_str())
                {
                    stack.push(path);
                }
            } else if name.ends_with(".rs") {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Read and parse a TOML file into `T`.
pub fn read_toml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

/// Validate a task id `T-NNN` (three or more digits).
pub fn valid_task_id(id: &str) -> bool {
    id.strip_prefix("T-")
        .is_some_and(|d| d.len() >= 3 && d.chars().all(|c| c.is_ascii_digit()))
}

/// `T-NNN` or an error.
pub fn task_id(id: &str) -> Result<String> {
    if valid_task_id(id) {
        Ok(id.to_owned())
    } else {
        bail!("`{id}` is not a task id of the form T-NNN")
    }
}

/// Append one JSON line to `path` (creating parent directories).
pub fn append_jsonl<T: serde::Serialize>(path: &Path, record: &T) -> Result<()> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let line = serde_json::to_string(record)?;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("open {}", path.display()))?;
    writeln!(f, "{line}")?;
    Ok(())
}

/// Read a JSON-lines file; a missing file is empty. Malformed lines are errors.
pub fn read_jsonl(path: &Path) -> Result<Vec<serde_json::Value>> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e).with_context(|| format!("read {}", path.display())),
    };
    text.lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, l)| {
            serde_json::from_str(l).with_context(|| format!("{}:{}", path.display(), i + 1))
        })
        .collect()
}

#[cfg(test)]
pub mod testutil {
    //! Throwaway directories for tests (no external crate).
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// A fresh empty directory, removed on drop.
    pub struct TempDir(pub PathBuf);

    impl TempDir {
        pub fn new(tag: &str) -> Self {
            let n = COUNTER.fetch_add(1, Ordering::SeqCst);
            let p = std::env::temp_dir()
                .join(format!("kiln-xtask-test-{}-{tag}-{n}", std::process::id()));
            let _ = std::fs::remove_dir_all(&p);
            std::fs::create_dir_all(&p).unwrap();
            TempDir(p)
        }
        pub fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_ids() {
        assert!(valid_task_id("T-001"));
        assert!(valid_task_id("T-1234"));
        assert!(!valid_task_id("T-01"));
        assert!(!valid_task_id("t-001"));
        assert!(!valid_task_id("T-001-tests"));
    }

    #[test]
    fn jsonl_round_trip() {
        let d = testutil::TempDir::new("jsonl");
        let p = d.path().join("m/x.jsonl");
        assert!(read_jsonl(&p).unwrap().is_empty());
        append_jsonl(&p, &serde_json::json!({"a": 1})).unwrap();
        append_jsonl(&p, &serde_json::json!({"a": 2})).unwrap();
        let v = read_jsonl(&p).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[1]["a"], 2);
    }
}
