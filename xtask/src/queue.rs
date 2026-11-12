//! `cargo xtask queue T-NNN`: the serial merge queue (CHARTER §4, METHODOLOGY §8).
//!
//! 1. take the lock `<git-common-dir>/kiln-queue.lock` (one queue at a time; a lock whose
//!    process is gone is reclaimed);
//! 2. refuse unless the primary checkout is on `main` with a clean working tree (paths under
//!    `metrics/` excepted: the queue itself appends there);
//! 3. rebase `task/T-NNN` onto `main` in `.worktrees/T-NNN`; on conflict abort and reject;
//! 4. run the full gate in that worktree, on the rebased commit;
//! 5. only if green, fast-forward `main` to it (never push);
//! 6. append one record to `metrics/queue.jsonl`.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use serde::Serialize;

use crate::git::{dirty_paths, git, git_try, rev};
use crate::timefmt::{Clock, format_utc};
use crate::util::{append_jsonl, read_jsonl};
use crate::worktree::Spec;

/// One line of `metrics/queue.jsonl` (field names: METHODOLOGY §13; `gate_seconds` extra).
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
    /// First failing gate step, `rebase` on conflict, null when merged.
    pub failed_step: Option<String>,
    /// Wall seconds spent in the gate.
    pub gate_seconds: f64,
}

/// What a gate run reports to the queue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GateOutcome {
    /// Green.
    pub passed: bool,
    /// First failing step.
    pub failed_step: Option<String>,
}

/// Runs the gate in a worktree (real: `cargo xtask ci`; tests: a fake).
pub trait GateRunner {
    /// Run the full gate in `worktree`.
    fn run(&self, worktree: &Path) -> Result<GateOutcome>;
}

/// The real gate: `cargo xtask ci --report …` in the worktree, with its own target directory.
pub struct CargoGate;

impl GateRunner for CargoGate {
    fn run(&self, worktree: &Path) -> Result<GateOutcome> {
        let target = worktree.join("target");
        std::fs::create_dir_all(&target)?;
        let report = target.join("kiln-gate-report.json");
        let _ = std::fs::remove_file(&report);
        let status = std::process::Command::new("cargo")
            .args(["xtask", "ci", "--report"])
            .arg(&report)
            .current_dir(worktree)
            .env("CARGO_TARGET_DIR", &target)
            // The worktree's rust-toolchain.toml decides the toolchain, not the caller's.
            .env_remove("RUSTUP_TOOLCHAIN")
            .status()
            .context("start cargo xtask ci")?;
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

/// Run the queue for `task` in the repository whose primary checkout is `root`.
/// Returns the record (also appended to `metrics/queue.jsonl`). Refusals return `Err` and write
/// nothing.
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
    let common = git(root, &["rev-parse", "--git-common-dir"])?;
    let common = if Path::new(&common).is_absolute() {
        PathBuf::from(common)
    } else {
        root.join(common)
    };
    let _lock = Lock::acquire(&common.join("kiln-queue.lock"), &spec.task)?;

    let head = git(root, &["symbolic-ref", "--quiet", "HEAD"]).unwrap_or_default();
    if head != "refs/heads/main" {
        bail!(
            "{} must have `main` checked out (found `{head}`)",
            root.display()
        );
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
        head_sha: rev(&wt, "HEAD")?,
        result: String::new(),
        failed_step: None,
        gate_seconds: 0.0,
    };

    let (rebased, _, err) = git_try(&wt, &["rebase", "--quiet", "main"])?;
    if !rebased {
        let _ = git_try(&wt, &["rebase", "--abort"]);
        eprintln!(
            "queue: rebase of {} onto main conflicts; aborted:\n{}",
            spec.branch(),
            err.trim()
        );
        record.result = "conflict".into();
        record.failed_step = Some("rebase".into());
    } else {
        record.head_sha = rev(&wt, "HEAD")?;
        let g0 = clock.now_millis();
        let outcome = gate.run(&wt)?;
        record.gate_seconds = ((clock.now_millis() - g0) as f64 / 100.0).round() / 10.0;
        if !outcome.passed {
            record.result = "gate_failed".into();
            record.failed_step = outcome.failed_step;
        } else if rev(root, "main")? != base_sha {
            record.result = "ff_failed".into();
            record.failed_step = Some("main-moved".into());
        } else {
            let (ok, _, err) = git_try(root, &["merge", "--ff-only", "--quiet", &record.head_sha])?;
            if ok {
                record.result = "merged".into();
            } else {
                eprintln!("queue: fast-forward failed: {}", err.trim());
                record.result = "ff_failed".into();
                record.failed_step = Some("fast-forward".into());
            }
        }
    }
    record.ended_at = format_utc(now_secs(clock));
    append_jsonl(&metrics, &record)?;
    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::testrepo::{self, g};
    use crate::worktree;
    use std::cell::Cell;

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

    struct FakeGate(bool);
    impl GateRunner for FakeGate {
        fn run(&self, wt: &Path) -> Result<GateOutcome> {
            // The gate must see the rebased tree: the base file from main is present.
            assert!(wt.join("base.txt").is_file());
            Ok(if self.0 {
                GateOutcome {
                    passed: true,
                    failed_step: None,
                }
            } else {
                GateOutcome {
                    passed: false,
                    failed_step: Some("test".into()),
                }
            })
        }
    }

    fn setup(tag: &str) -> crate::util::testutil::TempDir {
        let repo = testrepo::init(
            tag,
            &[
                ("base.txt", "base\n"),
                (".gitignore", ".worktrees/\ntarget/\n"),
            ],
        );
        worktree::create(repo.path(), &Spec::parse("T-001", false).unwrap(), false).unwrap();
        repo
    }

    fn read_records(root: &Path) -> Vec<serde_json::Value> {
        read_jsonl(&root.join("metrics/queue.jsonl")).unwrap()
    }

    #[test]
    fn green_gate_fast_forwards_main_and_logs() {
        let repo = setup("q-green");
        let root = repo.path();
        let wt = root.join(".worktrees/T-001");
        testrepo::commit(&wt, "feature", &[("feature.txt", "f")]);
        // main moves meanwhile: the queue must rebase.
        testrepo::commit(root, "other", &[("other.txt", "o")]);
        let r = run(root, "T-001", &clock(), &FakeGate(true)).unwrap();
        assert_eq!(r.result, "merged");
        assert_eq!(rev(root, "main").unwrap(), r.head_sha);
        assert!(root.join("feature.txt").is_file() && root.join("other.txt").is_file());
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
                "head_sha",
                "result",
                "started_at",
                "task"
            ]
        );
        assert!(recs[0]["failed_step"].is_null());
    }

    #[test]
    fn red_gate_leaves_main_alone_and_counts_attempts() {
        let repo = setup("q-red");
        let root = repo.path();
        let wt = root.join(".worktrees/T-001");
        testrepo::commit(&wt, "broken", &[("feature.txt", "f")]);
        let before = rev(root, "main").unwrap();
        let r1 = run(root, "T-001", &clock(), &FakeGate(false)).unwrap();
        assert_eq!(
            (r1.result.as_str(), r1.failed_step.as_deref(), r1.attempt),
            ("gate_failed", Some("test"), 1)
        );
        assert_eq!(rev(root, "main").unwrap(), before);
        let r2 = run(root, "T-001", &clock(), &FakeGate(true)).unwrap();
        assert_eq!((r2.result.as_str(), r2.attempt), ("merged", 2));
    }

    #[test]
    fn conflict_is_aborted_and_rejected() {
        let repo = setup("q-conflict");
        let root = repo.path();
        let wt = root.join(".worktrees/T-001");
        testrepo::commit(&wt, "task edit", &[("base.txt", "task\n")]);
        testrepo::commit(root, "main edit", &[("base.txt", "main\n")]);
        let before = rev(root, "main").unwrap();
        let task_head = rev(&wt, "HEAD").unwrap();
        let r = run(root, "T-001", &clock(), &FakeGate(true)).unwrap();
        assert_eq!(
            (r.result.as_str(), r.failed_step.as_deref()),
            ("conflict", Some("rebase"))
        );
        assert_eq!(rev(root, "main").unwrap(), before);
        assert_eq!(
            rev(&wt, "HEAD").unwrap(),
            task_head,
            "rebase must be aborted"
        );
        assert!(dirty_paths(&wt, None).unwrap().is_empty());
    }

    #[test]
    fn refusals_write_nothing() {
        let repo = setup("q-refuse");
        let root = repo.path();
        // Nothing to merge.
        assert!(run(root, "T-001", &clock(), &FakeGate(true)).is_err());
        let wt = root.join(".worktrees/T-001");
        testrepo::commit(&wt, "feature", &[("feature.txt", "f")]);
        // Dirty main (metrics/ is exempt, tracked or not).
        testrepo::commit(root, "metrics file", &[("metrics/tasks.jsonl", "")]);
        testrepo::write(root, "metrics/tasks.jsonl", "{}\n");
        testrepo::write(root, "metrics/new.jsonl", "{}\n");
        assert_eq!(
            dirty_paths(root, Some("metrics/")).unwrap(),
            Vec::<String>::new()
        );
        testrepo::write(root, "scratch.txt", "x");
        let e = run(root, "T-001", &clock(), &FakeGate(true)).unwrap_err();
        assert!(e.to_string().contains("dirty"), "{e}");
        std::fs::remove_file(root.join("scratch.txt")).unwrap();
        // Lock held by a live process (this one).
        let common = root.join(".git/kiln-queue.lock");
        std::fs::write(&common, format!("{} T-999\n", std::process::id())).unwrap();
        let e = run(root, "T-001", &clock(), &FakeGate(true)).unwrap_err();
        assert!(e.to_string().contains("another queue run"), "{e}");
        // Stale lock (no such process) is reclaimed.
        std::fs::write(&common, "999999999 T-999\n").unwrap();
        assert_eq!(
            run(root, "T-001", &clock(), &FakeGate(true))
                .unwrap()
                .result,
            "merged"
        );
        assert!(!common.exists(), "lock released");
        // Not on main.
        g(root, &["checkout", "-q", "-b", "elsewhere"]);
        assert!(run(root, "T-001", &clock(), &FakeGate(true)).is_err());
        assert_eq!(read_records(root).len(), 1);
    }
}
