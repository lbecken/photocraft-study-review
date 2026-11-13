//! `cargo xtask worktree new|rm|list`: one task = one branch = one worktree = one target dir.
//!
//! `new T-NNN` creates `.worktrees/T-NNN` on `task/T-NNN` from `main`; `new T-NNN --tests` (or
//! `new T-NNN-tests`) creates `.worktrees/T-NNN-tests` on `task/T-NNN-tests` for the test author.
//! The worktree's target directory is `<worktree>/target`, cargo's default for that workspace:
//! no target-directory variable is printed or needed (D-019). `new` first requires the task to
//! be `ready` in main's committed task file (METHODOLOGY §5.1, T-013 (3)). Unless `--no-seed`, it is seeded
//! from main's `target/` with copies of the artifacts of non-member packages only
//! (`crate::seed`), so the first build reuses compiled dependencies and rebuilds every workspace
//! crate from the worktree's own source.

use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

use crate::git::{branch_exists, git, git_try};
use crate::util::{Refused, task_id};

/// Which worktree of a task.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spec {
    /// `T-NNN`.
    pub task: String,
    /// The test author's worktree.
    pub tests: bool,
}

impl Spec {
    /// Parse `T-NNN` or `T-NNN-tests`; `--tests` also selects the tests worktree.
    pub fn parse(arg: &str, tests_flag: bool) -> Result<Self> {
        match arg.strip_suffix("-tests") {
            Some(base) => Ok(Spec {
                task: task_id(base)?,
                tests: true,
            }),
            None => Ok(Spec {
                task: task_id(arg)?,
                tests: tests_flag,
            }),
        }
    }
    /// `T-NNN` or `T-NNN-tests`.
    pub fn name(&self) -> String {
        if self.tests {
            format!("{}-tests", self.task)
        } else {
            self.task.clone()
        }
    }
    /// `task/<name>`.
    pub fn branch(&self) -> String {
        format!("task/{}", self.name())
    }
    /// `<root>/.worktrees/<name>`.
    pub fn path(&self, root: &Path) -> PathBuf {
        root.join(".worktrees").join(self.name())
    }
}

/// A created worktree.
#[derive(Debug)]
pub struct Info {
    /// Worktree directory.
    pub path: PathBuf,
    /// Branch name.
    pub branch: String,
    /// Target directory.
    pub target: PathBuf,
    /// What was copied from main's `target/` (`None`: not seeded).
    pub seeded: Option<crate::seed::Stats>,
}

/// Refuse unless `task` is `ready` in main's committed tree (T-013 Interface (3)). A main
/// without any `tasks/` directory (repositories that do not track tasks) is not checked.
pub fn require_ready_on_main(root: &Path, task: &str) -> Result<()> {
    let listing = git(root, &["ls-tree", "--name-only", "main", "--", "tasks/"])?;
    if listing.trim().is_empty() {
        return Ok(());
    }
    let prefix = format!("tasks/{task}-");
    let found: Vec<&str> = listing
        .lines()
        .filter(|l| l.starts_with(&prefix) && l.ends_with(".md"))
        .collect();
    let file = match found.as_slice() {
        [one] => *one,
        [] => {
            return Err(Refused(format!("worktree: refused: no tasks/{task}-*.md on main")).into());
        }
        many => {
            return Err(Refused(format!(
                "worktree: refused: {} files match tasks/{task}-*.md on main: {}",
                many.len(),
                many.join(", ")
            ))
            .into());
        }
    };
    let text = crate::precheck::show(root, "main", file)?.unwrap_or_default();
    let state = crate::frontmatter::parse(&text)
        .ok()
        .and_then(|(m, _)| m.get("state").and_then(|v| v.as_str()).map(str::to_owned))
        .unwrap_or_else(|| "without a state".to_owned());
    if state != "ready" {
        return Err(Refused(format!(
            "worktree: refused: {task} is {state} on main; worktree new needs ready"
        ))
        .into());
    }
    Ok(())
}

/// Create the worktree for `spec` from `main`, after the `ready` check.
pub fn create(root: &Path, spec: &Spec, seed: bool) -> Result<Info> {
    require_ready_on_main(root, &spec.task)?;
    add(root, spec, seed)
}

/// Unit-test fixtures of other modules (queue, join) whose toy task files are in later states:
/// the worktree without the `ready` check. Compiled into tests only, so the binary has no
/// bypass (D-026 R8).
#[cfg(test)]
pub fn create_unchecked_for_tests(root: &Path, spec: &Spec, seed: bool) -> Result<Info> {
    add(root, spec, seed)
}

fn add(root: &Path, spec: &Spec, seed: bool) -> Result<Info> {
    let path = spec.path(root);
    let branch = spec.branch();
    if path.exists() {
        bail!("{} already exists", path.display());
    }
    if branch_exists(root, &branch)? {
        bail!("branch {branch} already exists (remove it or reuse it with `git worktree add`)");
    }
    std::fs::create_dir_all(root.join(".worktrees"))?;
    let p = path.to_string_lossy().into_owned();
    git(
        root,
        &["worktree", "add", "--quiet", "-b", &branch, &p, "main"],
    )?;
    let target = path.join("target");
    let main_target = root.join("target");
    let mut seeded = None;
    if seed && main_target.is_dir() {
        seeded = Some(crate::seed::seed(root, &main_target, &path, &target)?);
    } else {
        std::fs::create_dir_all(&target)?;
    }
    Ok(Info {
        path,
        branch,
        target,
        seeded,
    })
}

/// Remove the worktree; delete its branch only if it is merged into `main`.
pub fn remove(root: &Path, spec: &Spec, force: bool) -> Result<String> {
    let path = spec.path(root);
    let p = path.to_string_lossy().into_owned();
    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(&p);
    git(root, &args)?;
    let branch = spec.branch();
    if !branch_exists(root, &branch)? {
        return Ok(format!("removed {p}"));
    }
    let merged = git_try(root, &["merge-base", "--is-ancestor", &branch, "main"])?.0;
    if merged {
        git(root, &["branch", "-d", &branch])?;
        Ok(format!("removed {p}; deleted merged branch {branch}"))
    } else {
        Ok(format!(
            "removed {p}; kept branch {branch} (not merged into main)"
        ))
    }
}

/// Describe the task worktrees.
pub fn list(root: &Path) -> Result<String> {
    let out = git(root, &["worktree", "list", "--porcelain"])?;
    let prefix = root.join(".worktrees");
    let mut rows = Vec::new();
    for block in out.split("\n\n") {
        let mut path = None;
        let mut branch = String::from("(detached)");
        for l in block.lines() {
            if let Some(p) = l.strip_prefix("worktree ") {
                path = Some(PathBuf::from(p));
            } else if let Some(b) = l.strip_prefix("branch refs/heads/") {
                branch = b.to_owned();
            }
        }
        let Some(path) = path else { continue };
        if !path.starts_with(&prefix) {
            continue;
        }
        let counts = git(
            root,
            &[
                "rev-list",
                "--left-right",
                "--count",
                &format!("main...{branch}"),
            ],
        )
        .unwrap_or_default();
        let (behind, ahead) = counts.split_once('\t').unwrap_or(("?", "?"));
        rows.push(format!(
            "{:<16} {:<22} ahead {ahead:>3}  behind {behind:>3}  {}",
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default(),
            branch,
            path.display()
        ));
    }
    if rows.is_empty() {
        Ok("no task worktrees".to_owned())
    } else {
        Ok(rows.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::testrepo;

    #[test]
    fn parse_specs() {
        assert_eq!(
            Spec::parse("T-001", false).unwrap(),
            Spec {
                task: "T-001".into(),
                tests: false
            }
        );
        assert_eq!(Spec::parse("T-001", true).unwrap().name(), "T-001-tests");
        assert_eq!(
            Spec::parse("T-001-tests", false).unwrap().branch(),
            "task/T-001-tests"
        );
        assert!(Spec::parse("T-1", false).is_err());
        assert!(Spec::parse("feature", false).is_err());
    }

    #[test]
    fn create_needs_ready_on_main() {
        let repo = testrepo::init(
            "wt-ready",
            &[(
                "tasks/T-008-x.md",
                "---\nid: T-008\nstate: specified\n---\n",
            )],
        );
        let root = repo.path();
        let spec = Spec::parse("T-008", false).unwrap();
        let e = create(root, &spec, false).unwrap_err();
        assert_eq!(
            e.to_string(),
            "worktree: refused: T-008 is specified on main; worktree new needs ready"
        );
        assert!(!spec.path(root).exists());
        assert!(!branch_exists(root, "task/T-008").unwrap());
        // Ready only in the working copy: still refused.
        testrepo::write(
            root,
            "tasks/T-008-x.md",
            "---\nid: T-008\nstate: ready\n---\n",
        );
        assert!(create(root, &spec, false).is_err());
        let e = create(root, &Spec::parse("T-009", true).unwrap(), false).unwrap_err();
        assert_eq!(
            e.to_string(),
            "worktree: refused: no tasks/T-009-*.md on main"
        );
        testrepo::g(root, &["commit", "-qam", "ready"]);
        assert!(create(root, &Spec::parse("T-008-tests", false).unwrap(), false).is_ok());
    }

    #[test]
    fn create_seed_list_remove() {
        let repo = testrepo::init("wt", &[("a.txt", "a")]);
        let root = repo.path();
        let spec = Spec::parse("T-007", false).unwrap();
        let info = create(root, &spec, false).unwrap();
        assert!(info.seeded.is_none());
        assert_eq!(
            std::fs::read_dir(&info.target).unwrap().count(),
            0,
            "empty target"
        );
        assert_eq!(
            git(&info.path, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap(),
            "task/T-007"
        );
        assert!(
            create(root, &spec, false).is_err(),
            "second create must refuse"
        );
        assert!(list(root).unwrap().contains("task/T-007"));
        // Unmerged branch with a commit is kept on removal.
        testrepo::commit(&info.path, "work", &[("b.txt", "b")]);
        let msg = remove(root, &spec, true).unwrap();
        assert!(msg.contains("kept branch"), "{msg}");
        assert!(!info.path.exists());
    }
}
