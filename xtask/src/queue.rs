//! `cargo xtask queue T-NNN`: the serial merge queue (CHARTER §4, METHODOLOGY §8, T-005).
//!
//! 1. take the lock `<git-common-dir>/kiln-queue.lock` (one queue at a time; a lock whose
//!    process is gone is reclaimed);
//! 2. refuse unless the primary checkout is on `main` with a clean working tree (paths under
//!    `metrics/` excepted: the queue itself appends there), the task worktree is clean and on its
//!    branch, and the branch is ahead of main;
//! 3. the preconditions of `crate::precheck` (task file, front matter, state, config, touches,
//!    join, review, approval, budget, main history), all read from main or the metrics records; a
//!    failure is a [`Refusal`] and writes nothing;
//! 4. build main's xtask from `base_sha` in a private build tree (`<git-common-dir>/kiln-queue/`),
//!    recompiling the xtask crate from a fresh export on every run (never a cached binary);
//! 5. rebase `task/T-NNN` onto `main` in `.worktrees/T-NNN`; a conflict only on generated paths
//!    (`xtask/generated.toml`) takes main's side and continues, and afterwards every generated
//!    file is regenerated and committed; any other conflict aborts and rejects;
//! 6. run main's gate in the worktree (and, if the task changes a gate path, the task's own gate
//!    too; both must pass);
//! 7. only if green, fast-forward `main` to it (never push);
//! 8. append one record to `metrics/queue.jsonl` and commit it on main (D-030 R12).

use std::collections::BTreeSet;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use serde::Serialize;

use crate::git::{dirty_paths, git, git_try, rev};
use crate::policy::Policy;
pub use crate::precheck::Refusal;
use crate::timefmt::{Clock, format_utc};
use crate::util::{append_jsonl, capture, read_jsonl, tail};
use crate::worktree::Spec;

/// One line of `metrics/queue.jsonl` (field names: METHODOLOGY §13; the rest are extras).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct QueueRecord {
    /// `T-NNN`.
    pub task: String,
    /// 1 for the first queue run of this task, then 2, 3, …
    pub attempt: u64,
    /// UTC start.
    pub started_at: String,
    /// UTC end.
    pub ended_at: String,
    /// `main` before the run.
    pub base_sha: String,
    /// The task head that was gated (after rebase; before it on conflict).
    pub head_sha: String,
    /// `merged`, `gate_failed`, `conflict` or `ff_failed`.
    pub result: String,
    /// First failing gate step (`main:`/`task:` prefixed under `main+task`), `rebase` or
    /// `regenerate:<path>` on conflict, null when merged.
    pub failed_step: Option<String>,
    /// Wall seconds spent in the gate(s).
    pub gate_seconds: f64,
    /// `main`, or `main+task` when the task changes a gate path.
    pub gate_xtask: String,
    /// The commit main's xtask was built from (= `base_sha`).
    pub gate_xtask_sha: String,
    /// Generated paths that were conflicted or changed by regeneration (file order).
    pub regenerated: Vec<String>,
    /// The gate paths the task changes (D-026 R3); printed, not written to the record.
    #[serde(skip)]
    pub gate_paths: Vec<String>,
    /// Run records with `tokens: null` that `budget` counted 0; printed, not written.
    #[serde(skip)]
    pub unknown_token_runs: usize,
}

/// What a gate run reports to the queue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GateOutcome {
    /// Green.
    pub passed: bool,
    /// First failing step.
    pub failed_step: Option<String>,
}

/// Which xtask runs the gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateXtask<'a> {
    /// The checkout's own xtask (`cargo run -p xtask -- ci`).
    Own,
    /// A prebuilt binary (main's xtask), run with the checkout as working directory.
    Binary(&'a Path),
}

/// Runs the gate in a worktree (real: xtask `ci`; tests: a fake).
pub trait GateRunner {
    /// Run the full gate in `worktree` with `xtask`.
    fn run(&self, worktree: &Path, xtask: GateXtask<'_>) -> Result<GateOutcome>;
    /// Build main's xtask from commit `sha` of the repository at `root`; return the binary.
    fn build_main_xtask(&self, root: &Path, sha: &str) -> Result<PathBuf> {
        build_main_xtask(root, sha)
    }
}

/// The real gate: xtask `ci --report …` in the worktree, with the worktree's target directory.
pub struct CargoGate;

impl GateRunner for CargoGate {
    fn run(&self, worktree: &Path, xtask: GateXtask<'_>) -> Result<GateOutcome> {
        let target = worktree.join("target");
        std::fs::create_dir_all(&target)?;
        let report = target.join("kiln-gate-report.json");
        let _ = std::fs::remove_file(&report);
        let mut cmd = match xtask {
            GateXtask::Own => {
                let mut c = std::process::Command::new("cargo");
                c.args(["run", "--quiet"]);
                if worktree.join("Cargo.lock").is_file() {
                    c.arg("--locked");
                }
                c.args(["--package", "xtask", "--"]);
                c
            }
            // Main's xtask in this worktree: built in a non-worktree export, so the stale-binary
            // guard (provenance.rs) lets it run without any exception.
            GateXtask::Binary(bin) => std::process::Command::new(bin),
        };
        let status = cmd
            .args(["ci", "--report"])
            .arg(&report)
            .current_dir(worktree)
            .env("CARGO_TARGET_DIR", &target)
            // The worktree's rust-toolchain.toml decides the toolchain, not the caller's.
            .env_remove("RUSTUP_TOOLCHAIN")
            .status()
            .context("start the gate")?;
        let parsed: Option<serde_json::Value> = std::fs::read_to_string(&report)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok());
        let Some(r) = parsed else {
            return Ok(GateOutcome {
                passed: false,
                failed_step: Some("gate-did-not-run".into()),
            });
        };
        let passed = status.success() && r["passed"] == serde_json::Value::Bool(true);
        let failed_step = r["failed_step"].as_str().map(str::to_owned);
        Ok(GateOutcome {
            passed,
            failed_step: if passed {
                None
            } else {
                failed_step.or(Some("unknown".into()))
            },
        })
    }
}

/// The repository's common git directory (absolute).
pub fn common_dir(root: &Path) -> Result<PathBuf> {
    let common = git(root, &["rev-parse", "--git-common-dir"])?;
    Ok(if Path::new(&common).is_absolute() {
        PathBuf::from(common)
    } else {
        root.join(common)
    })
}

/// Write the tree of commit `sha` to `dest` (which must not exist), with a private index: the
/// repository's own index and worktrees are untouched. `dest` is a plain directory, never a git
/// working tree.
pub fn export_tree(root: &Path, sha: &str, dest: &Path, index: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    let export = |args: &[&str]| -> Result<()> {
        let out = capture(
            crate::git::cmd(root)
                .args(args)
                .env("GIT_INDEX_FILE", index),
        )?;
        if !out.ok {
            bail!("git {} failed: {}", args.join(" "), out.stderr.trim());
        }
        Ok(())
    };
    export(&["read-tree", sha])?;
    let prefix = format!("{}/", dest.display());
    export(&[
        "checkout-index",
        "--all",
        "--force",
        &format!("--prefix={prefix}"),
    ])?;
    let _ = std::fs::remove_file(index);
    Ok(())
}

/// Build the xtask of commit `sha` in the private build tree
/// `<git-common-dir>/kiln-queue/main-xtask/` and return the binary (D-026 R1).
///
/// Every run starts from a fresh export of `sha` (`src/`, never a worktree) and, before
/// building, removes from the private `target/` every artifact of a workspace member of that
/// export (and all uplifted binaries, `incremental/` data and unknown files), so the xtask crate
/// is always compiled from the sources at `sha`; a binary or member artifact left by an earlier
/// run, or written there by task code during an earlier gate, is never reused. Artifacts of
/// non-member packages (registry dependencies) may be reused: an accepted, documented residual
/// risk.
pub fn build_main_xtask(root: &Path, sha: &str) -> Result<PathBuf> {
    let base = common_dir(root)?.join("kiln-queue").join("main-xtask");
    let (src, target) = (base.join("src"), base.join("target"));
    let bin = target
        .join("debug")
        .join(format!("xtask{}", std::env::consts::EXE_SUFFIX));
    // Left by older queues that reused a binary stamped with its sha; no longer read.
    let _ = std::fs::remove_file(base.join("built-from"));
    if src.exists() {
        std::fs::remove_dir_all(&src)?;
    }
    export_tree(root, sha, &src, &base.join("export.index"))?;
    let members = crate::seed::member_names(&src)
        .with_context(|| format!("list the workspace members of main at {sha}"))?;
    crate::seed::prune_to_non_members(&target, &members)?;
    if bin.exists() {
        bail!("could not remove the previous main xtask {}", bin.display());
    }
    let mut c = std::process::Command::new("cargo");
    c.args(["build", "--quiet", "--package", "xtask"]);
    if src.join("Cargo.lock").is_file() {
        c.arg("--locked");
    }
    let out = capture(
        c.current_dir(&src)
            .env("CARGO_TARGET_DIR", &target)
            .env_remove("RUSTUP_TOOLCHAIN"),
    )?;
    if !out.ok || !bin.is_file() {
        bail!(
            "building main's xtask at {sha} failed:\n{}",
            tail(&out.stderr, 30)
        );
    }
    Ok(bin)
}

/// Puts a task worktree back on drop (D-026 R5): aborts a rebase in progress, resets the branch
/// to `head` and removes untracked files (ignored files such as `target/` stay).
struct Restore<'a> {
    wt: &'a Path,
    head: &'a str,
    armed: bool,
}

impl Drop for Restore<'_> {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let _ = git_try(self.wt, &["rebase", "--abort"]);
        let _ = git_try(self.wt, &["reset", "--quiet", "--hard", self.head]);
        let _ = git_try(self.wt, &["clean", "--quiet", "-d", "--force"]);
    }
}

/// Whether `p` (in the primary checkout) is an evidence file whose uncommitted change is
/// refused at `ledger` (D-026 R6, D-027, D-030 R12): everything under `metrics/` (the queue now
/// commits its own `metrics/queue.jsonl` records, so there is no exemption), `docs/decisions.md`,
/// `tasks/`, `reviews/` and the two policy files.
pub fn is_evidence_path(p: &str) -> bool {
    let p = p.trim_matches('"');
    crate::policy::path_under(p, "metrics")
        || p == "docs/decisions.md"
        || crate::policy::path_under(p, "tasks")
        || crate::policy::path_under(p, "reviews")
        || p == crate::policy::GENERATED_FILE
        || p == crate::policy::LEDGER_FILE
}

/// Exclusive queue lock, released on drop.
pub struct Lock(PathBuf);

impl Lock {
    /// Take the lock at `path`, reclaiming it if the recorded process no longer exists.
    pub fn acquire(path: &Path, task: &str) -> Result<Self> {
        for _ in 0..2 {
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
            {
                Ok(mut f) => {
                    writeln!(f, "{} {task}", std::process::id())?;
                    return Ok(Lock(path.to_path_buf()));
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    let holder = std::fs::read_to_string(path).unwrap_or_default();
                    let pid = holder.split_whitespace().next().unwrap_or_default();
                    let alive = !pid.is_empty() && Path::new("/proc").join(pid).exists();
                    if alive || !Path::new("/proc").is_dir() {
                        bail!(
                            "another queue run holds {} ({}); one at a time",
                            path.display(),
                            holder.trim()
                        );
                    }
                    eprintln!(
                        "queue: reclaiming stale lock {} ({})",
                        path.display(),
                        holder.trim()
                    );
                    std::fs::remove_file(path)?;
                }
                Err(e) => return Err(e).with_context(|| format!("create {}", path.display())),
            }
        }
        bail!("could not take {}", path.display())
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn now_secs(clock: &dyn Clock) -> i64 {
    clock.now_millis().div_euclid(1000)
}

/// `git` with a non-interactive editor (rebase --continue must not open one).
fn git_rebase(dir: &Path, args: &[&str]) -> Result<(bool, String)> {
    let out = capture(crate::git::cmd(dir).args(args).env("GIT_EDITOR", "true"))?;
    Ok((out.ok, format!("{}{}", out.stdout, out.stderr)))
}

/// How the rebase ended.
#[derive(Debug)]
enum Rebased {
    /// Rebased; these generated paths conflicted and were resolved to main's side.
    Done(BTreeSet<String>),
    /// A conflict on a non-generated path (aborted).
    Conflict(String),
}

fn unmerged(wt: &Path) -> Result<Vec<String>> {
    Ok(git(wt, &["diff", "--name-only", "--diff-filter=U"])?
        .lines()
        .map(str::to_owned)
        .collect())
}

/// Rebase onto main; resolve conflicts that lie only in generated paths by taking main's side.
fn rebase(wt: &Path, policy: &Policy) -> Result<Rebased> {
    let mut conflicted = BTreeSet::new();
    let (mut ok, mut out) = git_rebase(wt, &["rebase", "--quiet", "main"])?;
    while !ok {
        let paths = unmerged(wt)?;
        let only_generated = !paths.is_empty() && paths.iter().all(|p| policy.is_generated(p));
        if !only_generated {
            let _ = git_try(wt, &["rebase", "--abort"]);
            return Ok(Rebased::Conflict(out));
        }
        for p in &paths {
            // During a rebase HEAD is main plus the commits applied so far: main's side.
            if git_try(wt, &["cat-file", "-e", &format!("HEAD:{p}")])?.0 {
                git(wt, &["checkout", "HEAD", "--", p])?;
            } else {
                git(wt, &["rm", "--quiet", "--force", "--", p])?;
            }
            conflicted.insert(p.clone());
        }
        (ok, out) = git_rebase(wt, &["rebase", "--continue"])?;
        if !ok && unmerged(wt)?.is_empty() {
            // Not a further conflict (e.g. a commit left empty): give up rather than guess.
            let _ = git_try(wt, &["rebase", "--abort"]);
            return Ok(Rebased::Conflict(out));
        }
    }
    Ok(Rebased::Done(conflicted))
}

/// `(name, source) → versions` of the packages with a source in a `Cargo.lock`.
fn lock_versions(
    text: &str,
) -> Result<std::collections::BTreeMap<(String, String), BTreeSet<String>>> {
    let v: toml::Value = toml::from_str(text).context("parse Cargo.lock")?;
    let mut out: std::collections::BTreeMap<(String, String), BTreeSet<String>> =
        Default::default();
    for p in v
        .get("package")
        .and_then(|p| p.as_array())
        .into_iter()
        .flatten()
    {
        let (Some(name), Some(version)) = (
            p.get("name").and_then(|x| x.as_str()),
            p.get("version").and_then(|x| x.as_str()),
        ) else {
            continue;
        };
        // Path packages (workspace members, path dependencies) have no source: their version
        // is set by the source tree, not by resolution.
        let Some(source) = p.get("source").and_then(|x| x.as_str()) else {
            continue;
        };
        out.entry((name.to_owned(), source.to_owned()))
            .or_default()
            .insert(version.to_owned());
    }
    Ok(out)
}

/// The registry/git packages of `main_lock` whose version the regenerated `new_lock` changed
/// (a version of main's lock disappears while a version main did not have appears).
pub fn lock_version_changes(main_lock: &str, new_lock: &str) -> Result<Vec<String>> {
    let (old, new) = (lock_versions(main_lock)?, lock_versions(new_lock)?);
    let mut changes = Vec::new();
    for (key, was) in &old {
        let Some(now) = new.get(key) else { continue };
        let lost: Vec<&String> = was.difference(now).collect();
        let gained: Vec<&String> = now.difference(was).collect();
        if !lost.is_empty() && !gained.is_empty() {
            changes.push(format!(
                "{} {} -> {}",
                key.0,
                lost.iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join("/"),
                gained
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join("/")
            ));
        }
    }
    Ok(changes)
}

/// Whether `path` is a `Cargo.lock` (at the root or below).
pub fn is_lock(path: &str) -> bool {
    path == "Cargo.lock" || path.ends_with("/Cargo.lock")
}

/// Run every generated entry's command in `wt`; commit what changed. On failure returns the
/// failing path. Returns the generated paths that changed.
fn regenerate(
    wt: &Path,
    task: &str,
    base_sha: &str,
    policy: &Policy,
    xtask: &Path,
) -> Result<std::result::Result<BTreeSet<String>, String>> {
    let gen_paths: BTreeSet<&str> = policy.generated.iter().map(|g| g.path.as_str()).collect();
    for g in &policy.generated {
        let main_lock = if is_lock(&g.path) {
            crate::precheck::show(wt, base_sha, &g.path)?
        } else {
            None
        };
        if let Some(text) = &main_lock {
            std::fs::write(wt.join(&g.path), text)?;
        }
        let argv: Vec<String> = g
            .command
            .iter()
            .map(|a| a.replace("{xtask}", &xtask.to_string_lossy()))
            .collect();
        let out = capture(
            std::process::Command::new(&argv[0])
                .args(&argv[1..])
                .current_dir(wt)
                .env("CARGO_TARGET_DIR", wt.join("target"))
                .env_remove("RUSTUP_TOOLCHAIN"),
        );
        let failed = match out {
            Err(e) => Some(format!("{e:#}")),
            Ok(o) if !o.ok => Some(tail(&format!("{}{}", o.stdout, o.stderr), 20)),
            Ok(_) => None,
        };
        if let Some(why) = failed {
            eprintln!(
                "queue: regenerating {} failed ({}):\n{why}",
                g.path,
                argv.join(" ")
            );
            return Ok(Err(g.path.clone()));
        }
        if let Some(text) = &main_lock {
            let new = std::fs::read_to_string(wt.join(&g.path)).unwrap_or_default();
            let changes = lock_version_changes(text, &new)?;
            if !changes.is_empty() {
                eprintln!(
                    "queue: regenerating {} changed versions of main's packages: {}",
                    g.path,
                    changes.join(", ")
                );
                return Ok(Err(g.path.clone()));
            }
        }
        let stray: Vec<String> = dirty_paths(wt, None)?
            .into_iter()
            .filter(|l| !gen_paths.contains(&l[3..]))
            .collect();
        if !stray.is_empty() {
            eprintln!(
                "queue: regenerating {} changed non-generated paths:\n{}",
                g.path,
                stray.join("\n")
            );
            return Ok(Err(g.path.clone()));
        }
    }
    let changed: BTreeSet<String> = dirty_paths(wt, None)?
        .into_iter()
        .map(|l| l[3..].to_owned())
        .collect();
    if !changed.is_empty() {
        let mut args = vec!["add", "--all", "--"];
        args.extend(changed.iter().map(String::as_str));
        git(wt, &args)?;
        let msg = format!("queue: regenerate generated files\n\nTask: {task}\nRole: director");
        git(wt, &["commit", "--quiet", "--no-verify", "-m", &msg])?;
    }
    Ok(Ok(changed))
}

/// Run the queue for `task` in the repository whose primary checkout is `root`.
/// Returns the record (also appended to `metrics/queue.jsonl`). Refusals return `Err` and write
/// nothing; a failed precondition is a [`Refusal`] in the error chain.
pub fn run(
    root: &Path,
    task: &str,
    clock: &dyn Clock,
    gate: &dyn GateRunner,
) -> Result<QueueRecord> {
    let spec = Spec::parse(task, false)?;
    if spec.tests {
        bail!("queue the task branch, not its tests branch (join it first)");
    }
    let _lock = Lock::acquire(&common_dir(root)?.join("kiln-queue.lock"), &spec.task)?;

    let head = git(root, &["symbolic-ref", "--quiet", "HEAD"]).unwrap_or_default();
    if head != "refs/heads/main" {
        bail!(
            "{} must have `main` checked out (found `{head}`)",
            root.display()
        );
    }
    // D-026 R6: evidence is read from main's committed tree, so uncommitted evidence would be
    // silently ignored; refuse instead, before any other check.
    let ledger: Vec<String> = dirty_paths(root, None)?
        .iter()
        .map(|l| l[3..].to_owned())
        .filter(|p| is_evidence_path(p))
        .collect();
    if !ledger.is_empty() {
        return Err(Refusal {
            check: "ledger",
            reason: format!(
                "uncommitted changes to evidence files ({}); commit the ledger on main first (evidence is read from main's committed tree)",
                ledger.join(", ")
            ),
        }
        .into());
    }
    let dirty = dirty_paths(root, Some("metrics/"))?;
    if !dirty.is_empty() {
        bail!(
            "the main working tree is dirty; commit or stash first:\n{}",
            dirty.join("\n")
        );
    }
    let wt = spec.path(root);
    if !wt.is_dir() {
        bail!(
            "{} does not exist (cargo xtask worktree new {})",
            wt.display(),
            spec.task
        );
    }
    let wt_branch = git(&wt, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    if wt_branch != spec.branch() {
        bail!(
            "{} is on `{wt_branch}`, expected `{}`",
            wt.display(),
            spec.branch()
        );
    }
    let wt_dirty = dirty_paths(&wt, None)?;
    if !wt_dirty.is_empty() {
        bail!(
            "{} has uncommitted changes:\n{}",
            wt.display(),
            wt_dirty.join("\n")
        );
    }
    let base_sha = rev(root, "main")?;
    let ahead: u64 = git(
        root,
        &["rev-list", "--count", &format!("main..{}", spec.branch())],
    )?
    .parse()?;
    if ahead == 0 {
        bail!(
            "{} has no commits beyond main; nothing to queue",
            spec.branch()
        );
    }
    let task_head = rev(&wt, "HEAD")?;
    let checked = crate::precheck::check(root, &spec.task, &spec.branch(), &task_head)?;
    let both = !checked.gate_paths.is_empty();
    if both {
        eprintln!(
            "queue: {} changes gate paths: {} (main's gate, then its own)",
            spec.task,
            checked.gate_paths.join(", ")
        );
    }
    let main_xtask = gate.build_main_xtask(root, &base_sha)?;

    let metrics = root.join("metrics/queue.jsonl");
    let attempt = 1 + read_jsonl(&metrics)?
        .iter()
        .filter(|r| r["task"].as_str() == Some(spec.task.as_str()))
        .count() as u64;
    let started = now_secs(clock);
    let mut record = QueueRecord {
        task: spec.task.clone(),
        attempt,
        started_at: format_utc(started),
        ended_at: String::new(),
        base_sha: base_sha.clone(),
        head_sha: task_head.clone(),
        result: String::new(),
        failed_step: None,
        gate_seconds: 0.0,
        gate_xtask: if both { "main+task" } else { "main" }.into(),
        gate_xtask_sha: base_sha.clone(),
        regenerated: Vec::new(),
        gate_paths: checked.gate_paths.clone(),
        unknown_token_runs: checked.unknown_token_runs,
    };

    // D-026 R5: on every outcome but `merged` (and on any error from here on) the task branch
    // goes back to `task_head` and its worktree is left clean.
    let mut restore = Restore {
        wt: &wt,
        head: &task_head,
        armed: true,
    };
    let mut ready = false;
    match rebase(&wt, &checked.policy)? {
        Rebased::Conflict(out) => {
            eprintln!(
                "queue: rebase of {} onto main conflicts; aborted:\n{}",
                spec.branch(),
                out.trim()
            );
            record.result = "conflict".into();
            record.failed_step = Some("rebase".into());
        }
        Rebased::Done(conflicted) if conflicted.is_empty() => ready = true,
        Rebased::Done(conflicted) => {
            match regenerate(&wt, &spec.task, &base_sha, &checked.policy, &main_xtask) {
                Ok(Ok(changed)) => {
                    record.regenerated = checked
                        .policy
                        .generated
                        .iter()
                        .map(|g| g.path.clone())
                        .filter(|p| conflicted.contains(p) || changed.contains(p))
                        .collect();
                    ready = true;
                }
                Ok(Err(path)) => {
                    record.result = "conflict".into();
                    record.failed_step = Some(format!("regenerate:{path}"));
                    record.regenerated = conflicted.into_iter().collect();
                }
                Err(e) => {
                    return Err(e.context("regenerating generated files"));
                }
            }
        }
    }
    if ready {
        // D-026 R4: the lock rule on every run, on the tree that would become main.
        let mut locks: BTreeSet<&str> = BTreeSet::from(["Cargo.lock"]);
        locks.extend(
            checked
                .changed
                .iter()
                .map(String::as_str)
                .filter(|p| is_lock(p)),
        );
        for p in locks {
            let (Some(main_lock), Some(new_lock)) = (
                crate::precheck::show(root, &base_sha, p)?,
                crate::precheck::show(&wt, "HEAD", p)?,
            ) else {
                continue;
            };
            let bumped = lock_version_changes(&main_lock, &new_lock)?;
            if !bumped.is_empty() && !checked.versions_approved {
                return Err(Refusal {
                    check: "dependencies",
                    reason: format!(
                        "{p} after the rebase changes the version of package(s) already in main's lock: {} (needs tier high and an approval row containing `dependency versions`)",
                        bumped.join(", ")
                    ),
                }
                .into());
            }
        }
        record.head_sha = rev(&wt, "HEAD")?;
        let g0 = clock.now_millis();
        let mut outcome = gate.run(&wt, GateXtask::Binary(&main_xtask))?;
        let mut prefix = "main:";
        if outcome.passed && both {
            outcome = gate.run(&wt, GateXtask::Own)?;
            prefix = "task:";
        }
        record.gate_seconds = ((clock.now_millis() - g0) as f64 / 100.0).round() / 10.0;
        if !outcome.passed {
            record.result = "gate_failed".into();
            record.failed_step = outcome
                .failed_step
                .map(|s| if both { format!("{prefix}{s}") } else { s });
        } else if rev(root, "main")? != base_sha {
            record.result = "ff_failed".into();
            record.failed_step = Some("main-moved".into());
        } else {
            let (ok, _, err) = git_try(root, &["merge", "--ff-only", "--quiet", &record.head_sha])?;
            if ok {
                record.result = "merged".into();
                restore.armed = false;
            } else {
                eprintln!("queue: fast-forward failed: {}", err.trim());
                record.result = "ff_failed".into();
                record.failed_step = Some("fast-forward".into());
            }
        }
    }
    drop(restore);
    record.ended_at = format_utc(now_secs(clock));
    append_jsonl(&metrics, &record)?;
    commit_record(root, &record)?;
    Ok(record)
}

/// D-030 R12: commit the record just appended, on main, as a commit that touches only
/// `metrics/queue.jsonl` (subject `queue: record T-NNN <result>`). It lands after the
/// fast-forward of a merge (so the merged record's `head_sha` is its parent and the record
/// commit is a ledger commit for the next `main-history` check) and after a non-merged outcome
/// alike; the task branch is never touched by it.
fn commit_record(root: &Path, record: &QueueRecord) -> Result<()> {
    let path = "metrics/queue.jsonl";
    git(root, &["add", "--", path])?;
    let msg = format!(
        "queue: record {} {}\n\nTask: {}\nRole: director",
        record.task, record.result, record.task
    );
    let mut args: Vec<&str> = Vec::new();
    // A repository without an identity (fresh toy repositories) still gets its record.
    let has_identity = git_try(root, &["config", "user.email"])?.0
        || std::env::var_os("GIT_COMMITTER_EMAIL").is_some();
    if !has_identity {
        args.extend([
            "-c",
            "user.name=kiln queue",
            "-c",
            "user.email=queue@kiln.invalid",
        ]);
    }
    args.extend(["commit", "--quiet", "--no-verify", "-m", &msg, "--", path]);
    let (ok, _, err) = git_try(root, &args)?;
    if !ok {
        bail!(
            "the record was appended to {path} but committing it on main failed (commit it by hand): {}",
            err.trim()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::testrepo::{self, g};
    use crate::worktree;
    use std::cell::{Cell, RefCell};

    /// A clock that advances 1.5 s per reading, from a fixed instant.
    struct FakeClock(Cell<i64>);
    impl Clock for FakeClock {
        fn now_millis(&self) -> i64 {
            let t = self.0.get();
            self.0.set(t + 1500);
            t
        }
    }
    fn clock() -> FakeClock {
        FakeClock(Cell::new(1_793_000_000_000))
    }

    /// Main's gate passes iff `main`; the task's own gate iff `own`. Records each call.
    struct FakeGate {
        main: bool,
        own: bool,
        calls: RefCell<Vec<String>>,
    }
    fn gate(main: bool, own: bool) -> FakeGate {
        FakeGate {
            main,
            own,
            calls: RefCell::new(Vec::new()),
        }
    }
    impl GateRunner for FakeGate {
        fn run(&self, wt: &Path, xtask: GateXtask<'_>) -> Result<GateOutcome> {
            // The gate must see the rebased tree: the base file from main is present.
            assert!(wt.join("base.txt").is_file());
            let (who, pass) = match xtask {
                GateXtask::Binary(b) => {
                    assert_eq!(b, Path::new("/fake/main-xtask"));
                    ("main", self.main)
                }
                GateXtask::Own => ("own", self.own),
            };
            self.calls.borrow_mut().push(who.into());
            Ok(GateOutcome {
                passed: pass,
                failed_step: (!pass).then(|| "test".into()),
            })
        }
        fn build_main_xtask(&self, root: &Path, sha: &str) -> Result<PathBuf> {
            assert_eq!(rev(root, "main")?, sha);
            self.calls.borrow_mut().push("build".into());
            Ok(PathBuf::from("/fake/main-xtask"))
        }
    }

    const GEN: &str = "[[generated]]\npath = \"gen.txt\"\ncommand = [\"sh\", \"-c\", \"ls items | sort > gen.txt\"]\n";
    const LED: &str = "[[ledger]]\npath = \"tasks/\"\nmode = \"any\"\n\n[[ledger]]\npath = \"metrics/\"\nmode = \"any\"\n\n[[ledger]]\npath = \"STATE.md\"\nmode = \"any\"\n\n[[ledger]]\npath = \"docs/decisions.md\"\nmode = \"any\"\n\n[[ledger]]\npath = \"scorecard/items/\"\nmode = \"add-only\"\n";

    fn task_md(tier: &str, state: &str, touches: &[&str]) -> String {
        format!(
            "---\nid: T-001\ntier: {tier}\nstate: {state}\nscorecard_items: []\ntouches: [{}]\nbudget_tokens: 1000\n---\n# T-001\n\n## Log\n",
            touches.join(", ")
        )
    }

    fn setup_with(tag: &str, task: &str) -> crate::util::testutil::TempDir {
        let repo = testrepo::init(
            tag,
            &[
                ("base.txt", "base\n"),
                (".gitignore", ".worktrees/\ntarget/\n"),
                ("tasks/T-001-thing.md", task),
                ("xtask/generated.toml", GEN),
                ("xtask/ledger.toml", LED),
                ("items/a", ""),
                ("gen.txt", "a\n"),
            ],
        );
        worktree::create_unchecked_for_tests(
            repo.path(),
            &Spec::parse("T-001", false).unwrap(),
            false,
        )
        .unwrap();
        repo
    }

    fn setup(tag: &str) -> crate::util::testutil::TempDir {
        setup_with(
            tag,
            &task_md(
                "low",
                "self_verified",
                &["feature.txt", "items", "base.txt"],
            ),
        )
    }

    /// D-030 R12: every commit on main after `since` is a queue record commit: subject
    /// `queue: record T-001 <result>`, touching only `metrics/queue.jsonl`; and the committed
    /// file equals the working copy (nothing left uncommitted).
    fn assert_only_records_since(root: &Path, since: &str) {
        let commits = g(root, &["rev-list", &format!("{since}..main")]);
        for c in commits.lines() {
            let subject = g(root, &["log", "-1", "--format=%s", c]);
            assert!(
                subject.starts_with("queue: record T-001 "),
                "{c}: {subject}"
            );
            let files = g(root, &["diff", "--name-only", &format!("{c}^"), c]);
            assert_eq!(files, "metrics/queue.jsonl", "{c}");
        }
        assert!(
            dirty_paths(root, Some("never/"))
                .unwrap()
                .iter()
                .all(|l| !l.contains("metrics/")),
            "record committed"
        );
    }

    fn read_records(root: &Path) -> Vec<serde_json::Value> {
        read_jsonl(&root.join("metrics/queue.jsonl")).unwrap()
    }

    fn refusal(r: Result<QueueRecord>) -> Refusal {
        r.unwrap_err().downcast::<Refusal>().unwrap()
    }

    #[test]
    fn budget_counts_mains_committed_runs_against_the_last_extend_row() {
        let repo = setup("q-budget");
        let root = repo.path();
        let wt = root.join(".worktrees/T-001");
        testrepo::commit(&wt, "feature", &[("feature.txt", "f")]);
        let rec = |t: &str| {
            format!(
                "{{\"kind\":\"run\",\"task\":\"T-001\",\"role\":\"implementer\",\"tokens\":{t}}}\n"
            )
        };
        let jsonl = format!(
            "{}{}{}{{\"kind\":\"run\",\"task\":\"T-002\",\"tokens\":9999}}\n",
            rec("700"),
            rec("400"),
            rec("null")
        );
        testrepo::commit(root, "runs", &[("metrics/tasks.jsonl", &jsonl)]);
        assert_refused(
            root,
            "budget",
            "T-001 has 1100 recorded tokens, over its ceiling 1000 (budget_tokens); a docs/decisions.md row \"Extend T-001 to <N>\" with N ≥ 1100 is needed",
        );
        let table = |rows: &str| format!("| ID | Decision |\n|---|---|\n{rows}");
        // Not extensions: no ceiling, another id, a fenced line.
        testrepo::commit(
            root,
            "no",
            &[(
                "docs/decisions.md",
                &format!(
                    "```\n| D-1 | Extend T-001 to 5000 |\n```\n\n{}",
                    table(
                        "| D-2 | Extend T-001 |\n| D-3 | Extend T-001-tests to 5000 |\n| D-4 | Extend T-001 to 5000x |\n"
                    )
                ),
            )],
        );
        assert_refused(root, "budget", "over its ceiling 1000 (budget_tokens)");
        // The last row wins.
        testrepo::commit(
            root,
            "low",
            &[(
                "docs/decisions.md",
                &table("| D-5 | Extend T-001 to 5000 tokens |\n| D-6 | Extend T-001 to 1050 |\n"),
            )],
        );
        assert_refused(root, "budget", "over its ceiling 1050 (Extend row)");
        testrepo::commit(
            root,
            "ok",
            &[(
                "docs/decisions.md",
                &table("| D-6 | Extend T-001 to 1050 |\n| D-7 | Extend T-001 to 1100 |\n"),
            )],
        );
        let r = run(root, "T-001", &clock(), &gate(true, true)).unwrap();
        assert_eq!((r.result.as_str(), r.unknown_token_runs), ("merged", 1));
    }

    #[test]
    fn green_gate_fast_forwards_main_and_logs() {
        let repo = setup("q-green");
        let root = repo.path();
        let wt = root.join(".worktrees/T-001");
        testrepo::commit(&wt, "feature", &[("feature.txt", "f")]);
        // main moves meanwhile (a ledger commit): the queue must rebase.
        testrepo::commit(root, "other", &[("STATE.md", "o")]);
        let base = rev(root, "main").unwrap();
        let fake = gate(true, true);
        let r = run(root, "T-001", &clock(), &fake).unwrap();
        assert_eq!(r.result, "merged");
        assert_eq!(*fake.calls.borrow(), ["build", "main"]);
        assert_eq!(
            (r.gate_xtask.as_str(), r.gate_xtask_sha.as_str()),
            ("main", base.as_str())
        );
        assert!(r.regenerated.is_empty());
        assert_eq!(rev(root, "main^").unwrap(), r.head_sha);
        assert_only_records_since(root, &r.head_sha);
        assert!(root.join("feature.txt").is_file() && root.join("STATE.md").is_file());
        assert_eq!(r.started_at, "2026-10-26T07:33:20Z");
        assert_eq!(r.ended_at, "2026-10-26T07:33:24Z");
        assert_eq!(r.gate_seconds, 1.5);
        let recs = read_records(root);
        assert_eq!(recs.len(), 1);
        let keys: Vec<&str> = recs[0]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            [
                "attempt",
                "base_sha",
                "ended_at",
                "failed_step",
                "gate_seconds",
                "gate_xtask",
                "gate_xtask_sha",
                "head_sha",
                "regenerated",
                "result",
                "started_at",
                "task"
            ]
        );
        assert!(recs[0]["failed_step"].is_null());
        assert_eq!(recs[0]["regenerated"], serde_json::json!([]));
    }

    #[test]
    fn red_gate_leaves_main_alone_and_counts_attempts() {
        let repo = setup("q-red");
        let root = repo.path();
        let wt = root.join(".worktrees/T-001");
        testrepo::commit(&wt, "broken", &[("feature.txt", "f")]);
        let before = rev(root, "main").unwrap();
        let head0 = rev(&wt, "HEAD").unwrap();
        let r1 = run(root, "T-001", &clock(), &gate(false, true)).unwrap();
        assert_eq!(
            (r1.result.as_str(), r1.failed_step.as_deref(), r1.attempt),
            ("gate_failed", Some("test"), 1)
        );
        assert_only_records_since(root, &before);
        assert_eq!(rev(&wt, "HEAD").unwrap(), head0);
        let r2 = run(root, "T-001", &clock(), &gate(true, false)).unwrap();
        assert_eq!((r2.result.as_str(), r2.attempt), ("merged", 2));
    }

    /// Commit a `joined` record for the worktree head on main and an approving review of it on
    /// the branch (medium/high evidence). Returns the reviewed head.
    fn evidence(root: &Path, wt: &Path) -> String {
        let h = rev(wt, "HEAD").unwrap();
        let mut text =
            std::fs::read_to_string(root.join("metrics/tasks.jsonl")).unwrap_or_default();
        text.push_str(&format!(
            "{{\"kind\":\"join\",\"task\":\"T-001\",\"result\":\"joined\",\"joined_sha\":\"{h}\"}}\n"
        ));
        testrepo::commit(root, "join record", &[("metrics/tasks.jsonl", &text)]);
        testrepo::commit(
            wt,
            "review",
            &[(
                "reviews/T-001-r1.md",
                &format!("---\ntask: T-001\nround: 1\nreviewed_sha: {h}\nverdict: approve\n---\n"),
            )],
        );
        h
    }

    fn approve(root: &Path, decision: &str) {
        testrepo::commit(
            root,
            "approve",
            &[(
                "docs/decisions.md",
                &format!("| # | Decision |\n|---|---|\n| D-1 | {decision} |\n"),
            )],
        );
    }

    #[test]
    fn gate_path_changes_need_tier_high_and_run_both_gates() {
        // D-026 R3 (a): a gate path in a low or medium task is refused at `gate-path`.
        let repo = setup_with(
            "q-gp-low",
            &task_md("low", "self_verified", &["feature.txt", "xtask"]),
        );
        let root = repo.path();
        let wt = root.join(".worktrees/T-001");
        testrepo::commit(&wt, "weaken", &[("xtask/src/main.rs", "fn main() {}")]);
        assert_refused(root, "gate-path", "xtask/src/main.rs");
        assert_refused(root, "gate-path", "tier high");

        // (b) + (c): a high task runs both gates and needs `gate paths` in its approval row.
        let repo = setup_with(
            "q-both",
            &task_md("high", "reviewed", &["feature.txt", "xtask"]),
        );
        let root = repo.path();
        let wt = root.join(".worktrees/T-001");
        testrepo::commit(&wt, "weaken", &[("xtask/src/main.rs", "fn main() {}")]);
        evidence(root, &wt);
        approve(root, "Approve T-001 for the queue.");
        assert_refused(root, "approval", "gate paths");
        approve(
            root,
            "Approve T-001 for the queue, including its gate paths.",
        );
        let head0 = rev(&wt, "HEAD").unwrap();
        let before = rev(root, "main").unwrap();
        let fake = gate(false, true);
        let r = run(root, "T-001", &clock(), &fake).unwrap();
        assert_eq!(
            (
                r.result.as_str(),
                r.failed_step.as_deref(),
                r.gate_xtask.as_str()
            ),
            ("gate_failed", Some("main:test"), "main+task")
        );
        assert_eq!(r.gate_paths, ["xtask/src/main.rs"]);
        assert_eq!(
            *fake.calls.borrow(),
            ["build", "main"],
            "own gate not reached"
        );
        assert_eq!(rev(&wt, "HEAD").unwrap(), head0, "branch put back (R5)");
        let fake = gate(true, false);
        let r = run(root, "T-001", &clock(), &fake).unwrap();
        assert_eq!(r.failed_step.as_deref(), Some("task:test"));
        assert_eq!(*fake.calls.borrow(), ["build", "main", "own"]);
        assert_only_records_since(root, &before);
        assert_eq!(rev(&wt, "HEAD").unwrap(), head0, "branch put back (R5)");
        let r = run(root, "T-001", &clock(), &gate(true, true)).unwrap();
        assert_eq!(
            (r.result.as_str(), r.gate_xtask.as_str()),
            ("merged", "main+task")
        );
    }

    #[test]
    fn widened_gate_paths_are_refused_for_low_tasks() {
        for p in [
            ".cargo/config.toml",
            "app/.cargo/config.toml",
            "rust-toolchain.toml",
            "clippy.toml",
            "app/clippy.toml",
            "rustfmt.toml",
            ".rustfmt.toml",
            ".github/workflows/ci.yml",
            "app/build.rs",
            "build.rs",
        ] {
            let top = p.split('/').next().unwrap();
            let repo = setup_with(
                "q-gp-wide",
                &task_md("low", "self_verified", &["feature.txt", top]),
            );
            let root = repo.path();
            testrepo::commit(&root.join(".worktrees/T-001"), "g", &[(p, "x")]);
            assert_refused(root, "gate-path", p);
        }
        // The root Cargo.toml is a gate path only when a gate table changes.
        let manifest = "[workspace]\nmembers = []\n\n[profile.dev]\nopt-level = 0\n";
        for (changed, is_gate) in [
            (
                "[workspace]\nmembers = [\"a\"]\n\n[profile.dev]\nopt-level = 0\n",
                false,
            ),
            (
                "# comment\n[workspace]\nmembers = []\n\n[profile.dev]\nopt-level = 0\n",
                false,
            ),
            (
                "[workspace]\nmembers = []\n\n[profile.dev]\nopt-level = 1\n",
                true,
            ),
            (
                "[workspace]\nmembers = []\n[workspace.lints.rust]\nunsafe_code = \"allow\"\n\n[profile.dev]\nopt-level = 0\n",
                true,
            ),
            (
                "[workspace]\nmembers = []\n[workspace.metadata.x]\na = 1\n\n[profile.dev]\nopt-level = 0\n",
                true,
            ),
            (
                "[workspace]\nmembers = []\n\n[profile.dev]\nopt-level = 0\n[profile.test]\ndebug = 0\n",
                true,
            ),
            ("not toml [", true),
        ] {
            let repo = testrepo::init(
                "q-gp-root",
                &[
                    ("base.txt", "base\n"),
                    (".gitignore", ".worktrees/\ntarget/\n"),
                    (
                        "tasks/T-001-thing.md",
                        &task_md("low", "self_verified", &["Cargo.toml"]),
                    ),
                    ("xtask/generated.toml", GEN),
                    ("xtask/ledger.toml", LED),
                    ("Cargo.toml", manifest),
                ],
            );
            let root = repo.path();
            worktree::create_unchecked_for_tests(
                root,
                &Spec::parse("T-001", false).unwrap(),
                false,
            )
            .unwrap();
            testrepo::commit(
                &root.join(".worktrees/T-001"),
                "m",
                &[("Cargo.toml", changed)],
            );
            if is_gate {
                assert_refused(root, "gate-path", "Cargo.toml");
            } else {
                let r = run(root, "T-001", &clock(), &gate(true, true)).unwrap();
                assert_eq!(
                    (r.result.as_str(), r.gate_xtask.as_str()),
                    ("merged", "main"),
                    "{changed}"
                );
            }
        }
    }

    #[test]
    fn member_lints_and_custom_build_scripts_are_gate_paths() {
        // D-030 R10.
        let app = "[package]\nname = \"app\"\nversion = \"0.1.0\"\nbuild = \"gen/make.rs\"\n\n[lints]\nworkspace = true\n";
        let mk = || {
            let repo = testrepo::init(
                "q-gp-r10",
                &[
                    ("base.txt", "base\n"),
                    (".gitignore", ".worktrees/\ntarget/\n"),
                    (
                        "tasks/T-001-thing.md",
                        &task_md("low", "self_verified", &["app", "lib"]),
                    ),
                    ("xtask/generated.toml", GEN),
                    ("xtask/ledger.toml", LED),
                    ("app/Cargo.toml", app),
                    ("app/gen/make.rs", "fn main() {}\n"),
                    ("lib/Cargo.toml", "[package]\nname = \"lib\"\n"),
                ],
            );
            worktree::create_unchecked_for_tests(
                repo.path(),
                &Spec::parse("T-001", false).unwrap(),
                false,
            )
            .unwrap();
            repo
        };
        for (path, content) in [
            (
                "app/Cargo.toml",
                app.replace("[lints]\nworkspace = true\n", ""),
            ),
            (
                "app/Cargo.toml",
                app.replace("workspace = true", "workspace = false"),
            ),
            (
                "app/Cargo.toml",
                format!("{app}\n[lints.rust]\nunsafe_code = \"allow\"\n")
                    .replace("[lints]\nworkspace = true\n", ""),
            ),
            ("app/Cargo.toml", app.replace("gen/make.rs", "gen/other.rs")),
            (
                "app/gen/make.rs",
                "fn main() { /* weakened */ }\n".to_owned(),
            ),
            (
                "lib/Cargo.toml",
                "[package]\nname = \"lib\"\nbuild = \"../lib/src/b.rs\"\n".to_owned(),
            ),
            (
                "lib/Cargo.toml",
                "[package]\nname = \"lib\"\n[lints]\nworkspace = true\n".to_owned(),
            ),
        ] {
            let repo = mk();
            let root = repo.path();
            testrepo::commit(&root.join(".worktrees/T-001"), "w", &[(path, &content)]);
            assert_refused(root, "gate-path", path);
        }
        // A file that some package's build key names on the head (new key) is a gate path too.
        let repo = mk();
        let root = repo.path();
        testrepo::commit(
            &root.join(".worktrees/T-001"),
            "w",
            &[
                (
                    "lib/Cargo.toml",
                    "[package]\nname = \"lib\"\nbuild = \"src/b.rs\"\n",
                ),
                ("lib/src/b.rs", ""),
            ],
        );
        let reason = refusal(run(root, "T-001", &clock(), &gate(true, true))).reason;
        assert!(reason.contains("lib/Cargo.toml, lib/src/b.rs"), "{reason}");
        // Other manifest and source edits stay ordinary.
        let repo = mk();
        let root = repo.path();
        testrepo::commit(
            &root.join(".worktrees/T-001"),
            "ok",
            &[
                ("app/Cargo.toml", &app.replace("0.1.0", "0.2.0")),
                ("app/src/lib.rs", ""),
            ],
        );
        let r = run(root, "T-001", &clock(), &gate(true, true)).unwrap();
        assert_eq!(
            (r.result.as_str(), r.gate_xtask.as_str()),
            ("merged", "main")
        );
    }

    #[test]
    fn dependency_changes_need_tier_high_and_keep_mains_versions() {
        let reg = "registry+https://github.com/rust-lang/crates.io-index";
        let lock = |v: &str| {
            format!(
                "version = 4\n\n[[package]]\nname = \"serde\"\nversion = \"{v}\"\nsource = \"{reg}\"\n"
            )
        };
        let manifest =
            |deps: &str| format!("[package]\nname = \"app\"\nversion = \"0.1.0\"\n{deps}");
        let mk = |tag: &str, tier: &str, state: &str| {
            let repo = testrepo::init(
                tag,
                &[
                    ("base.txt", "base\n"),
                    (".gitignore", ".worktrees/\ntarget/\n"),
                    (
                        "tasks/T-001-thing.md",
                        &task_md(tier, state, &["app", "Cargo.lock"]),
                    ),
                    ("xtask/generated.toml", GEN),
                    ("xtask/ledger.toml", LED),
                    ("Cargo.lock", &lock("1.0.1")),
                    ("app/Cargo.toml", &manifest("")),
                ],
            );
            worktree::create_unchecked_for_tests(
                repo.path(),
                &Spec::parse("T-001", false).unwrap(),
                false,
            )
            .unwrap();
            repo
        };
        // (R4) Cargo.lock or a dependency table in a low task: `dependencies`.
        let repo = mk("q-dep-low", "low", "self_verified");
        let root = repo.path();
        let wt = root.join(".worktrees/T-001");
        testrepo::commit(
            &wt,
            "lock",
            &[("Cargo.lock", &format!("{}\n", lock("1.0.1")))],
        );
        assert_refused(root, "dependencies", "Cargo.lock");
        g(&wt, &["reset", "-q", "--hard", "HEAD~1"]);
        for deps in [
            "\n[dependencies]\nserde = \"1\"\n",
            "\n[dev-dependencies]\nserde = \"1\"\n",
            "\n[build-dependencies]\nserde = \"1\"\n",
            "\n[target.'cfg(unix)'.dependencies]\nserde = \"1\"\n",
            "\n[patch.crates-io]\nserde = { path = \"x\" }\n",
        ] {
            testrepo::commit(&wt, "deps", &[("app/Cargo.toml", &manifest(deps))]);
            assert_refused(root, "dependencies", "dependency tables of app/Cargo.toml");
            g(&wt, &["reset", "-q", "--hard", "HEAD~1"]);
        }
        // Non-dependency manifest edits and new manifests without dependencies pass.
        testrepo::commit(
            &wt,
            "meta",
            &[
                ("app/Cargo.toml", &manifest("description = \"x\"\n")),
                ("app/sub/Cargo.toml", &manifest("")),
            ],
        );
        assert_eq!(
            run(root, "T-001", &clock(), &gate(true, true))
                .unwrap()
                .result,
            "merged"
        );

        // A high task may change the lock, but never the version of a package of main's lock.
        let repo = mk("q-dep-high", "high", "reviewed");
        let root = repo.path();
        let wt = root.join(".worktrees/T-001");
        testrepo::commit(&wt, "bump", &[("Cargo.lock", &lock("1.0.2"))]);
        evidence(root, &wt);
        approve(root, "Approve T-001.");
        assert_refused(root, "dependencies", "serde 1.0.1 -> 1.0.2");
        assert_refused(root, "dependencies", "dependency versions");
        // D-030 R11: with `dependency versions` in the approval row the change merges.
        approve(root, "Approve T-001, Dependency Versions included."); // F16: case matters
        assert_refused(root, "dependencies", "serde 1.0.1 -> 1.0.2");
        approve(root, "Approve T-001, including its dependency versions.");
        let r = run(root, "T-001", &clock(), &gate(true, true)).unwrap();
        assert_eq!(r.result, "merged", "{r:?}");
        assert!(
            std::fs::read_to_string(root.join("Cargo.lock"))
                .unwrap()
                .contains("1.0.2")
        );
    }

    #[test]
    fn lock_rule_is_applied_to_the_rebased_tree() {
        // D-030 R13 (b): the only bump appears after the rebase (a regeneration command for
        // another generated file rewrites the lock after the lock's own check ran), so only the
        // check on the rebased tree can catch it. Refusal `dependencies`; branch put back.
        let reg = "registry+https://github.com/rust-lang/crates.io-index";
        let lock = format!(
            "version = 4\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.1\"\nsource = \"{reg}\"\n"
        );
        let policy = "[[generated]]\npath = \"Cargo.lock\"\ncommand = [\"true\"]\n\n[[generated]]\npath = \"gen.txt\"\ncommand = [\"sh\", \"-c\", \"ls items | sort > gen.txt; sed -i s/1.0.1/1.0.2/ Cargo.lock\"]\n";
        let repo = testrepo::init(
            "q-lock-rebased",
            &[
                ("base.txt", "base\n"),
                (".gitignore", ".worktrees/\ntarget/\n"),
                (
                    "tasks/T-001-thing.md",
                    &task_md("low", "self_verified", &["items"]),
                ),
                ("xtask/generated.toml", policy),
                ("xtask/ledger.toml", LED),
                ("Cargo.lock", &lock),
                ("items/a", ""),
                ("gen.txt", "a\n"),
            ],
        );
        let root = repo.path();
        worktree::create_unchecked_for_tests(root, &Spec::parse("T-001", false).unwrap(), false)
            .unwrap();
        let wt = root.join(".worktrees/T-001");
        testrepo::commit(&wt, "b", &[("items/b", ""), ("gen.txt", "a\nb\n")]);
        testrepo::commit(root, "c", &[("items/c", ""), ("gen.txt", "a\nc\n")]);
        let (main0, head0) = (rev(root, "main").unwrap(), rev(&wt, "HEAD").unwrap());
        let fake = gate(true, true);
        let r = refusal(run(root, "T-001", &clock(), &fake));
        assert_eq!(r.check, "dependencies", "{r}");
        assert!(
            r.reason.contains("after the rebase") && r.reason.contains("serde 1.0.1 -> 1.0.2"),
            "{r}"
        );
        assert_eq!(*fake.calls.borrow(), ["build"], "the gate never ran");
        assert_eq!(rev(root, "main").unwrap(), main0);
        assert_eq!(rev(&wt, "HEAD").unwrap(), head0);
        assert!(dirty_paths(&wt, None).unwrap().is_empty());
        assert!(read_records(root).is_empty(), "a refusal writes nothing");
    }

    /// A gate that litters the worktree and, if `move_main`, lets main move meanwhile.
    struct MessyGate {
        root: PathBuf,
        pass: bool,
        move_main: bool,
    }
    impl GateRunner for MessyGate {
        fn run(&self, wt: &Path, _: GateXtask<'_>) -> Result<GateOutcome> {
            testrepo::write(wt, "litter.txt", "x");
            testrepo::write(wt, "base.txt", "edited by the gate\n");
            if self.move_main {
                testrepo::commit(&self.root, "meanwhile", &[("STATE.md", "moved")]);
            }
            Ok(GateOutcome {
                passed: self.pass,
                failed_step: (!self.pass).then(|| "test".into()),
            })
        }
        fn build_main_xtask(&self, _: &Path, _: &str) -> Result<PathBuf> {
            Ok(PathBuf::from("/fake/main-xtask"))
        }
    }

    #[test]
    fn non_merged_outcomes_leave_the_branch_and_worktree_as_they_were() {
        // D-026 R5: gate_failed and ff_failed put the branch back and clean the worktree, so the
        // review and join evidence (ancestors of the old head) stay valid for a re-queue.
        for (pass, move_main, result) in [(false, false, "gate_failed"), (true, true, "ff_failed")]
        {
            let repo = setup("q-restore");
            let root = repo.path();
            let wt = root.join(".worktrees/T-001");
            testrepo::commit(&wt, "f", &[("feature.txt", "f")]);
            testrepo::commit(root, "ledger", &[("STATE.md", "s")]); // forces a real rebase
            let head0 = rev(&wt, "HEAD").unwrap();
            let gate = MessyGate {
                root: root.to_path_buf(),
                pass,
                move_main,
            };
            let r = run(root, "T-001", &clock(), &gate).unwrap();
            assert_eq!(r.result, result, "{r:?}");
            assert_ne!(r.head_sha, head0, "the gate ran on the rebased head");
            assert_eq!(
                rev(&wt, "HEAD").unwrap(),
                head0,
                "{result}: branch put back"
            );
            assert_eq!(
                g(root, &["rev-parse", "task/T-001"]),
                head0,
                "{result}: the branch ref itself"
            );
            assert!(
                dirty_paths(&wt, None).unwrap().is_empty(),
                "{result}: worktree clean"
            );
            assert_eq!(
                std::fs::read_to_string(wt.join("base.txt")).unwrap(),
                "base\n"
            );
        }
    }

    #[test]
    fn conflict_is_aborted_and_rejected() {
        let repo = setup("q-conflict");
        let root = repo.path();
        let wt = root.join(".worktrees/T-001");
        testrepo::commit(&wt, "task edit", &[("base.txt", "task\n")]);
        testrepo::commit(
            root,
            "main edit",
            &[("STATE.md", "x"), ("base.txt", "main\n")],
        );
        let before = rev(root, "main").unwrap();
        let task_head = rev(&wt, "HEAD").unwrap();
        let r = run(root, "T-001", &clock(), &gate(true, true)).unwrap();
        assert_eq!(
            (r.result.as_str(), r.failed_step.as_deref()),
            ("conflict", Some("rebase"))
        );
        assert_only_records_since(root, &before);
        assert_eq!(
            rev(&wt, "HEAD").unwrap(),
            task_head,
            "rebase must be aborted"
        );
        assert!(dirty_paths(&wt, None).unwrap().is_empty());
    }

    #[test]
    fn generated_only_conflicts_are_regenerated() {
        let repo = setup("q-gen");
        let root = repo.path();
        let wt = root.join(".worktrees/T-001");
        testrepo::commit(&wt, "item b", &[("items/b", ""), ("gen.txt", "a\nb\n")]);
        testrepo::commit(&wt, "more", &[("feature.txt", "f")]);
        // Another task merged meanwhile with its own item and regenerated file.
        testrepo::commit(root, "item c", &[("items/c", ""), ("gen.txt", "a\nc\n")]);
        let r = run(root, "T-001", &clock(), &gate(true, true)).unwrap();
        assert_eq!(r.result, "merged", "{r:?}");
        assert_eq!(r.regenerated, ["gen.txt"]);
        assert_eq!(
            std::fs::read_to_string(root.join("gen.txt")).unwrap(),
            "a\nb\nc\n"
        );
        let msg = g(root, &["log", "-1", "--format=%B", "main^"]);
        assert_eq!(
            msg,
            "queue: regenerate generated files\n\nTask: T-001\nRole: director"
        );
        assert!(root.join("feature.txt").is_file());
        assert!(dirty_paths(&wt, None).unwrap().is_empty());
    }

    #[test]
    fn mixed_or_failing_regeneration_is_a_conflict() {
        let repo = setup("q-gen-mixed");
        let root = repo.path();
        let wt = root.join(".worktrees/T-001");
        testrepo::commit(&wt, "b", &[("gen.txt", "a\nb\n"), ("base.txt", "task\n")]);
        testrepo::commit(root, "c", &[("gen.txt", "a\nc\n"), ("base.txt", "main\n")]);
        let (main0, head0) = (rev(root, "main").unwrap(), rev(&wt, "HEAD").unwrap());
        let r = run(root, "T-001", &clock(), &gate(true, true)).unwrap();
        assert_eq!(
            (r.result.as_str(), r.failed_step.as_deref()),
            ("conflict", Some("rebase"))
        );
        assert_only_records_since(root, &main0);
        assert_eq!(rev(&wt, "HEAD").unwrap(), head0);

        let repo = setup("q-gen-fail");
        let root = repo.path();
        let wt = root.join(".worktrees/T-001");
        testrepo::commit(&wt, "b", &[("gen.txt", "a\nb\n"), ("items/b", "")]);
        // main's generator is broken now (a ledger-free change, so it lands via a fake merge
        // record; here: before any merge, so main-history passes trivially).
        let broken = GEN.replace("ls items | sort > gen.txt", "exit 3");
        testrepo::commit(
            root,
            "c",
            &[("gen.txt", "a\nc\n"), ("xtask/generated.toml", &broken)],
        );
        let (main0, head0) = (rev(root, "main").unwrap(), rev(&wt, "HEAD").unwrap());
        let r = run(root, "T-001", &clock(), &gate(true, true)).unwrap();
        assert_eq!(
            (r.result.as_str(), r.failed_step.as_deref()),
            ("conflict", Some("regenerate:gen.txt"))
        );
        assert_only_records_since(root, &main0);
        assert_eq!(rev(&wt, "HEAD").unwrap(), head0, "task branch restored");
        assert!(dirty_paths(&wt, None).unwrap().is_empty());
    }

    #[test]
    fn leaky_regeneration_is_a_failed_regeneration() {
        // D-026 R9 / D-027: a regeneration command that changes a non-generated path gives the
        // failed-regeneration result; nothing is merged and the branch is put back (R5).
        let repo = setup("q-gen-leak");
        let root = repo.path();
        let wt = root.join(".worktrees/T-001");
        testrepo::commit(&wt, "b", &[("gen.txt", "a\nb\n"), ("items/b", "")]);
        let leaky = GEN.replace(
            "ls items | sort > gen.txt",
            "ls items | sort > gen.txt; echo x > base.txt",
        );
        testrepo::commit(
            root,
            "c",
            &[("gen.txt", "a\nc\n"), ("xtask/generated.toml", &leaky)],
        );
        let (main0, head0) = (rev(root, "main").unwrap(), rev(&wt, "HEAD").unwrap());
        let r = run(root, "T-001", &clock(), &gate(true, true)).unwrap();
        assert_eq!(
            (r.result.as_str(), r.failed_step.as_deref()),
            ("conflict", Some("regenerate:gen.txt"))
        );
        assert_only_records_since(root, &main0);
        assert_eq!(rev(&wt, "HEAD").unwrap(), head0);
        assert!(dirty_paths(&wt, None).unwrap().is_empty());
    }

    /// Snapshot of everything a refusal must leave alone.
    fn snapshot(root: &Path) -> Vec<String> {
        let wt = root.join(".worktrees/T-001");
        vec![
            rev(root, "main").unwrap(),
            rev(&wt, "HEAD").unwrap(),
            dirty_paths(root, None).unwrap().join("|"),
            dirty_paths(&wt, None).unwrap().join("|"),
            std::fs::read_to_string(root.join("metrics/queue.jsonl")).unwrap_or_default(),
        ]
    }

    fn assert_refused(root: &Path, check: &str, needle: &str) {
        let before = snapshot(root);
        let fake = gate(true, true);
        let r = refusal(run(root, "T-001", &clock(), &fake));
        assert_eq!(r.check, check, "{r}");
        assert!(
            r.to_string()
                .starts_with(&format!("queue: refused: {check}: "))
        );
        assert!(r.reason.contains(needle), "{r}");
        assert!(fake.calls.borrow().is_empty(), "nothing built or gated");
        assert_eq!(snapshot(root), before);
    }

    #[test]
    fn state_front_matter_touches_and_config_refusals() {
        for state in [
            "draft",
            "ready",
            "in_progress",
            "reviewed",
            "tests_joined",
            "merged",
            "blocked",
        ] {
            let repo = setup_with("q-state", &task_md("low", state, &["feature.txt"]));
            testrepo::commit(
                &repo.path().join(".worktrees/T-001"),
                "f",
                &[("feature.txt", "f")],
            );
            assert_refused(repo.path(), "state", state);
        }
        let repo = setup("q-fm");
        let root = repo.path();
        let wt = root.join(".worktrees/T-001");
        let tf = "tasks/T-001-thing.md";
        let original = std::fs::read_to_string(wt.join(tf)).unwrap();
        testrepo::commit(
            &wt,
            "touches",
            &[(tf, &original.replace("feature.txt,", "feature.txt, src,"))],
        );
        assert_refused(root, "front-matter", "`touches`");
        g(&wt, &["reset", "-q", "--hard", "HEAD~1"]);
        testrepo::commit(
            &wt,
            "log",
            &[
                (tf, &format!("{original}- implementer: did x\n")),
                ("feature.txt", "f"),
            ],
        );
        // The director moved main's state after the branch was made: still fine.
        testrepo::commit(
            root,
            "state",
            &[(tf, &original.replace("self_verified", "queued"))],
        );
        testrepo::commit(&wt, "outside", &[("src/x.rs", "x"), ("items_b", "")]);
        assert_refused(root, "touches", ": items_b, src/x.rs");
        g(&wt, &["reset", "-q", "--hard", "HEAD~1"]);
        g(&wt, &["rm", "-q", "items/a"]);
        g(&wt, &["commit", "-q", "-m", "delete"]);
        // items/a is inside touches; deleting gen.txt (generated) is exempt.
        g(&wt, &["rm", "-q", "gen.txt"]);
        g(&wt, &["commit", "-q", "-m", "delete gen"]);
        testrepo::commit(
            root,
            "bad policy",
            &[(
                "xtask/ledger.toml",
                "[[ledger]]\npath = \"xtask/\"\nmode = \"any\"\n",
            )],
        );
        assert_refused(root, "config", "xtask/");
        g(root, &["rm", "-q", "xtask/ledger.toml"]);
        g(root, &["commit", "-q", "-m", "no ledger"]);
        assert_refused(root, "config", "missing");
        testrepo::commit(root, "ledger back", &[("xtask/ledger.toml", LED)]);
        assert_eq!(
            run(root, "T-001", &clock(), &gate(true, true))
                .unwrap()
                .result,
            "merged"
        );
    }

    #[test]
    fn task_file_refusals() {
        let repo = setup_with(
            "q-tf",
            &task_md("urgent", "self_verified", &["feature.txt"]),
        );
        let root = repo.path();
        testrepo::commit(&root.join(".worktrees/T-001"), "f", &[("feature.txt", "f")]);
        assert_refused(root, "task-file", "urgent");
        testrepo::commit(root, "two", &[("tasks/T-001-other.md", "x")]);
        assert_refused(root, "task-file", "2 files");
        g(
            root,
            &["rm", "-q", "tasks/T-001-other.md", "tasks/T-001-thing.md"],
        );
        g(root, &["commit", "-q", "-m", "none"]);
        assert_refused(root, "task-file", "no tasks/T-001-*.md");
        let no_touches = "---\nid: T-001\ntier: low\nstate: self_verified\n---\n";
        testrepo::commit(root, "nt", &[("tasks/T-001-thing.md", no_touches)]);
        assert_refused(root, "task-file", "touches");
        let wrong_id = task_md("low", "self_verified", &["x"]).replace("id: T-001", "id: T-002");
        testrepo::commit(root, "id", &[("tasks/T-001-thing.md", &wrong_id)]);
        assert_refused(root, "task-file", "id: T-001");
    }

    #[test]
    fn medium_and_high_evidence() {
        let repo = setup_with("q-ev", &task_md("high", "reviewed", &["feature.txt"]));
        let root = repo.path();
        let wt = root.join(".worktrees/T-001");
        let joined = testrepo::commit(&wt, "impl", &[("feature.txt", "f")]);
        assert_refused(root, "join", "no `joined`");
        let join = |sha: &str, result: &str| {
            crate::util::append_jsonl(
                &root.join("metrics/tasks.jsonl"),
                &serde_json::json!({"kind": "join", "task": "T-001", "result": result, "joined_sha": sha}),
            )
            .unwrap();
            // D-026 R6: an uncommitted record is refused, not read.
            assert_refused(root, "ledger", "metrics/tasks.jsonl");
            g(root, &["add", "metrics/tasks.jsonl"]);
            g(root, &["commit", "-q", "-m", "join record"]);
        };
        join(&joined, "acceptance_failed");
        assert_refused(root, "join", "no `joined`");
        // An ancestor of the head that is main's own commit: fine too.
        join(&g(root, &["merge-base", "main", "task/T-001"]), "joined");
        let review = |round: u32, verdict: &str, sha: &str| {
            format!(
                "---\ntask: T-001\nround: {round}\nreviewed_sha: {sha}\nverdict: {verdict}\n---\n"
            )
        };
        assert_refused(root, "review", "no reviews/T-001-r<N>.md");
        testrepo::commit(
            &wt,
            "r1",
            &[("reviews/T-001-r1.md", &review(1, "approve", &joined))],
        );
        testrepo::commit(
            &wt,
            "r2",
            &[(
                "reviews/T-001-r2.md",
                &review(2, "request-changes", &joined),
            )],
        );
        assert_refused(root, "review", "request-changes");
        testrepo::commit(
            &wt,
            "r10",
            &[("reviews/T-001-r10.md", &review(10, "approve", "0123abcd"))],
        );
        assert_refused(root, "review", "not a full 40-hex commit id");
        // D-026 R2: a symbolic or abbreviated reviewed_sha is refused even though it resolves.
        for symbolic in ["task/T-001", "HEAD", &joined[..12], &joined.to_uppercase()] {
            testrepo::commit(
                &wt,
                "r10",
                &[("reviews/T-001-r10.md", &review(10, "approve", symbolic))],
            );
            assert_refused(root, "review", "not a full 40-hex commit id");
        }
        let missing = "0123456789abcdef0123456789abcdef01234567";
        testrepo::commit(
            &wt,
            "r10",
            &[("reviews/T-001-r10.md", &review(10, "approve", missing))],
        );
        assert_refused(root, "review", "not a commit");
        let side = g(
            root,
            &["commit-tree", "-m", "side", &format!("{joined}^{{tree}}")],
        );
        testrepo::commit(
            &wt,
            "r10",
            &[("reviews/T-001-r10.md", &review(10, "approve", &side))],
        );
        assert_refused(root, "review", "not an ancestor");
        testrepo::commit(
            &wt,
            "r10",
            &[("reviews/T-001-r10.md", &review(10, "approve", &joined))],
        );
        let tf = "tasks/T-001-thing.md";
        let log = format!(
            "{}- reviewer: ok\n",
            std::fs::read_to_string(wt.join(tf)).unwrap()
        );
        testrepo::commit(&wt, "log", &[(tf, &log)]);
        assert_refused(root, "approval", "Approve T-001");
        testrepo::commit(
            root,
            "approve",
            &[(
                "docs/decisions.md",
                "| # | Date | Decision | Why |\n|---|---|---|---|\n| D-1 | 2026-01-01 | Approve T-001 (queue). | ok |\n",
            )],
        );
        let late = testrepo::commit(&wt, "late fix", &[("feature.txt", "g")]);
        assert_refused(root, "review", &late[..12]);
        assert_refused(root, "review", "stale review");
        g(&wt, &["reset", "-q", "--hard", "HEAD~1"]);
        // After review: generated paths and the task's acceptance/guard tests may still change
        // (D-026 R2).
        testrepo::commit(
            &wt,
            "after review",
            &[
                ("gen.txt", "a\nz\n"),
                ("crates/a/tests/acceptance_t001.rs", "// t\n"),
                ("apps/b/tests/guard_t001_x.rs", "// g\n"),
            ],
        );
        let r = run(root, "T-001", &clock(), &gate(true, true)).unwrap();
        assert_eq!(r.result, "merged");
    }

    #[test]
    fn main_history_tolerates_only_ledger_commits() {
        let repo = setup_with("q-hist", &task_md("low", "queued", &["feature.txt", "f2"]));
        let root = repo.path();
        let wt = root.join(".worktrees/T-001");
        testrepo::commit(&wt, "f", &[("feature.txt", "f")]);
        // Before any merge record, history is not checked.
        testrepo::commit(root, "code on main", &[("code.rs", "x")]);
        assert_eq!(
            run(root, "T-001", &clock(), &gate(true, true))
                .unwrap()
                .result,
            "merged"
        );
        testrepo::commit(&wt, "f2", &[("f2", "x")]);
        testrepo::commit(
            root,
            "ledger",
            &[
                (
                    "tasks/T-001-thing.md",
                    &task_md("low", "queued", &["feature.txt", "f2"])
                        .replace("## Log\n", "## Log\n- x\n"),
                ),
                ("metrics/tasks.jsonl", ""),
                ("STATE.md", "s"),
                ("scorecard/items/x.toml", "id = 1"),
            ],
        );
        let mark = rev(root, "main").unwrap();
        testrepo::commit(root, "edit item", &[("scorecard/items/x.toml", "id = 2")]);
        let bad = rev(root, "main").unwrap();
        assert_refused(root, "main-history", &bad);
        g(root, &["reset", "-q", "--hard", &mark]);
        for (p, c) in [
            ("src/a.rs", "x"),
            ("xtask/src/main.rs", "x"),
            ("Cargo.lock", "x"),
        ] {
            let c = testrepo::commit(root, "direct", &[(p, c)]);
            assert_refused(root, "main-history", &c);
            // F15 / D-030 R12: a fake `merged` record cannot move the anchor: uncommitted it is
            // refused at `ledger`, and only committed records are read.
            let fake =
                format!("{{\"task\":\"T-009\",\"result\":\"merged\",\"head_sha\":\"{c}\"}}\n");
            let real = std::fs::read_to_string(root.join("metrics/queue.jsonl")).unwrap();
            std::fs::write(root.join("metrics/queue.jsonl"), format!("{real}{fake}")).unwrap();
            assert_refused(root, "ledger", "metrics/queue.jsonl");
            std::fs::write(root.join("metrics/queue.jsonl"), real).unwrap();
            g(root, &["reset", "-q", "--hard", &mark]);
        }
        let r = run(root, "T-001", &clock(), &gate(true, true)).unwrap();
        assert_eq!(r.result, "merged");
        // The record commit after the merge is itself tolerated by the next main-history check.
        assert_eq!(rev(root, "main^").unwrap(), r.head_sha);
        assert_eq!(
            g(root, &["log", "-1", "--format=%s"]),
            "queue: record T-001 merged"
        );
        assert!(g(root, &["show", "main:metrics/queue.jsonl"]).contains(&r.head_sha));
        testrepo::commit(&wt, "f3", &[("f2", "y")]);
        let r = run(root, "T-001", &clock(), &gate(true, true)).unwrap();
        assert_eq!(r.result, "merged");
    }

    #[test]
    fn main_xtask_is_rebuilt_from_base_on_every_run() {
        // D-026 R1, with real cargo on a tiny workspace (no dependencies).
        let repo = testrepo::init(
            "q-rebuild",
            &[
                (
                    "Cargo.toml",
                    "[workspace]\nresolver = \"2\"\nmembers = [\"xtask\"]\n",
                ),
                (
                    "xtask/Cargo.toml",
                    "[package]\nname = \"xtask\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
                ),
                (
                    "xtask/src/main.rs",
                    "fn main() { println!(\"built from base\"); }\n",
                ),
                (".gitignore", "target/\n"),
            ],
        );
        let root = repo.path();
        let sha = rev(root, "main").unwrap();
        let out = |bin: &Path| {
            let o = std::process::Command::new(bin).output().unwrap();
            String::from_utf8_lossy(&o.stdout).trim().to_owned()
        };
        let bin = build_main_xtask(root, &sha).unwrap();
        assert_eq!(out(&bin), "built from base");
        let src = bin
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("src");
        assert_eq!(
            crate::provenance::tree_at(&src),
            None,
            "the export is not a working tree"
        );
        // Task code during a gate overwrites the cached binary and the member's deps artifact
        // (newer mtime, so a freshness check would trust it) and edits the export.
        let fake = "#!/bin/sh\necho poisoned\n";
        let deps = bin.parent().unwrap().join("deps");
        for e in std::fs::read_dir(&deps).unwrap() {
            let p = e.unwrap().path();
            let n = p.file_name().unwrap().to_string_lossy().into_owned();
            if n.starts_with("xtask-") && !n.contains('.') {
                std::fs::write(&p, fake).unwrap();
            }
        }
        std::fs::write(&bin, fake).unwrap();
        // D-030 R13 (a): and future-dates everything of the member, so cargo's mtime freshness
        // would trust it if the private target were not pruned.
        let future = std::time::SystemTime::now() + std::time::Duration::from_secs(86_400 * 365);
        let profile = bin.parent().unwrap().to_path_buf();
        let mut stack = vec![profile.clone()];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    stack.push(p);
                } else {
                    let rel = crate::util::rel(&profile, &p);
                    if rel.contains("xtask") {
                        std::fs::File::options()
                            .write(true)
                            .open(&p)
                            .unwrap()
                            .set_modified(future)
                            .unwrap();
                    }
                }
            }
        }
        std::fs::write(
            src.join("xtask/src/main.rs"),
            "fn main() { println!(\"edited\"); }\n",
        )
        .unwrap();
        assert_eq!(out(&bin), "poisoned");
        let bin2 = build_main_xtask(root, &sha).unwrap();
        assert_eq!(bin2, bin);
        assert_eq!(out(&bin2), "built from base", "rebuilt, not reused");
        assert!(
            !bin.parent()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("built-from")
                .exists()
        );
    }

    #[test]
    fn lock_version_changes_detects_bumps_only() {
        let lock = |pkgs: &[(&str, &str, Option<&str>)]| {
            let mut s = String::from("version = 4\n");
            for (n, v, src) in pkgs {
                s.push_str(&format!(
                    "\n[[package]]\nname = \"{n}\"\nversion = \"{v}\"\n"
                ));
                if let Some(src) = src {
                    s.push_str(&format!("source = \"{src}\"\n"));
                }
            }
            s
        };
        let reg = Some("registry+https://github.com/rust-lang/crates.io-index");
        let main = lock(&[("serde", "1.0.1", reg), ("app", "0.1.0", None)]);
        let same = lock(&[
            ("serde", "1.0.1", reg),
            ("app", "0.2.0", None),
            ("new", "1.0.0", reg),
        ]);
        assert!(lock_version_changes(&main, &same).unwrap().is_empty());
        let extra = lock(&[("serde", "1.0.1", reg), ("serde", "1.0.9", reg)]);
        assert!(lock_version_changes(&main, &extra).unwrap().is_empty());
        let removed = lock(&[("app", "0.1.0", None)]);
        assert!(lock_version_changes(&main, &removed).unwrap().is_empty());
        let bumped = lock(&[("serde", "1.0.2", reg)]);
        assert_eq!(
            lock_version_changes(&main, &bumped).unwrap(),
            ["serde 1.0.1 -> 1.0.2"]
        );
    }

    #[test]
    fn basic_refusals_write_nothing() {
        let repo = setup("q-refuse");
        let root = repo.path();
        // Nothing to merge.
        assert!(run(root, "T-001", &clock(), &gate(true, true)).is_err());
        let wt = root.join(".worktrees/T-001");
        testrepo::commit(&wt, "feature", &[("feature.txt", "f")]);
        // D-026 R6: uncommitted evidence (tracked or not) is refused at `ledger`; the queue's
        // own log metrics/queue.jsonl is exempt.
        testrepo::commit(root, "metrics file", &[("metrics/tasks.jsonl", "")]);
        for (p, c) in [
            ("metrics/tasks.jsonl", "{}\n"),
            ("metrics/new.jsonl", "{}\n"),
            ("docs/decisions.md", "x"),
            ("tasks/T-002-x.md", "x"),
            ("reviews/T-001-r1.md", "x"),
            ("xtask/ledger.toml", "x"),
            ("xtask/generated.toml", "x"),
        ] {
            let had = root
                .join(p)
                .is_file()
                .then(|| std::fs::read(root.join(p)).unwrap());
            testrepo::write(root, p, c);
            assert_refused(root, "ledger", p);
            match had {
                Some(b) => std::fs::write(root.join(p), b).unwrap(),
                None => std::fs::remove_file(root.join(p)).unwrap(),
            }
        }
        // D-030 R12: no exemption for the queue's own log any more.
        testrepo::write(root, "metrics/queue.jsonl", "");
        assert_refused(root, "ledger", "metrics/queue.jsonl");
        std::fs::remove_file(root.join("metrics/queue.jsonl")).unwrap();
        assert!(is_evidence_path("metrics/tasks.jsonl") && is_evidence_path("metrics/queue.jsonl"));
        assert!(!is_evidence_path("docs/decisions.md.bak") && !is_evidence_path("STATE.md"));
        testrepo::write(root, "scratch.txt", "x");
        let e = run(root, "T-001", &clock(), &gate(true, true)).unwrap_err();
        assert!(e.to_string().contains("dirty"), "{e}");
        std::fs::remove_file(root.join("scratch.txt")).unwrap();
        // Lock held by a live process (this one).
        let common = root.join(".git/kiln-queue.lock");
        std::fs::write(&common, format!("{} T-999\n", std::process::id())).unwrap();
        let e = run(root, "T-001", &clock(), &gate(true, true)).unwrap_err();
        assert!(e.to_string().contains("another queue run"), "{e}");
        // Stale lock (no such process) is reclaimed.
        std::fs::write(&common, "999999999 T-999\n").unwrap();
        assert_eq!(
            run(root, "T-001", &clock(), &gate(true, true))
                .unwrap()
                .result,
            "merged"
        );
        assert!(!common.exists(), "lock released");
        // Not on main.
        g(root, &["checkout", "-q", "-b", "elsewhere"]);
        assert!(run(root, "T-001", &clock(), &gate(true, true)).is_err());
        assert_eq!(read_records(root).len(), 1);
    }
}
