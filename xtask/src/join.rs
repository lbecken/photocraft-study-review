//! `cargo xtask join T-NNN` and `cargo xtask join --revert-check T-NNN` (METHODOLOGY §6).
//!
//! Acceptance tests are the integration-test targets `acceptance_tNNN*` (files
//! `crates/*/tests/acceptance_tNNN*.rs` or `apps/*/tests/…`) written on `task/T-NNN-tests`.
//!
//! `join`:
//! 1. **base**: in `.worktrees/T-NNN-tests` (base + tests only) build and run the acceptance
//!    tests. `base_fail_kind` is `fails` (every acceptance test compiles and fails), `does_not_compile`
//!    (no acceptance target compiles: weak evidence, the reviewer must run `--revert-check`),
//!    `mixed` (some do not compile, the rest fail), or `passes` (at least one passes on base:
//!    the test proves nothing, so the join is refused and undone);
//! 2. `impl_tests_green`: the implementer branch's own `cargo test --workspace` before the merge;
//! 3. merge `task/T-NNN-tests` into `task/T-NNN` (`--no-ff`); on conflict abort and refuse;
//! 4. run the acceptance tests on the joined branch (`acc_fail_first_join` failures), then the gate;
//! 5. append a `join` record to `metrics/tasks.jsonl`. Exit 0 only if the base evidence holds,
//!    every acceptance test passes on the joined branch, and the gate is green. Acceptance
//!    failures keep the merge: the task goes back to the implementer with the tests.
//!
//! `--revert-check [--revert <sha>…]`: in a temporary worktree at the joined `task/T-NNN`, revert
//! the implementation commits (default: every non-merge commit in `main..task/T-NNN` that is not
//! on `task/T-NNN-tests`; or the given ones, e.g. only the logic behind an API stub), keep the
//! tests, run the acceptance tests and record whether they now fail (`revert_check` record).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use serde::Serialize;

use crate::git::{dirty_paths, git, git_try, rev};
use crate::queue::GateRunner;
use crate::testbins::{self, Outcome};
use crate::timefmt::{Clock, format_utc};
use crate::util::{append_jsonl, capture, cargo, rust_files};
use crate::worktree::Spec;

/// Acceptance results in one checkout.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AccRun {
    /// (target, test) → outcome, for targets that compiled.
    pub results: BTreeMap<(String, String), Outcome>,
    /// Acceptance targets that did not compile.
    pub not_compiled: BTreeSet<String>,
}

/// Builds and runs tests (real: cargo; unit tests: a fake).
pub trait TestOracle {
    /// Acceptance targets with `prefix` in `dir`.
    fn acceptance(&self, dir: &Path, target_dir: &Path, prefix: &str) -> Result<AccRun>;
    /// Whether `cargo test --workspace` passes in `dir`.
    fn all_green(&self, dir: &Path, target_dir: &Path) -> Result<bool>;
}

/// The real oracle.
pub struct CargoOracle;

/// Acceptance target names present on disk in `dir` (file stems under `*/tests/`).
pub fn acceptance_targets(dir: &Path, prefix: &str) -> Result<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    for top in ["crates", "apps"] {
        for f in rust_files(dir, &dir.join(top), &[])? {
            let in_tests = f.parent().is_some_and(|p| p.ends_with("tests"));
            let stem = f.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
            if in_tests && stem.starts_with(prefix) {
                out.insert(stem.to_owned());
            }
        }
    }
    Ok(out)
}

impl TestOracle for CargoOracle {
    fn acceptance(&self, dir: &Path, target_dir: &Path, prefix: &str) -> Result<AccRun> {
        let wanted = acceptance_targets(dir, prefix)?;
        let mut run = AccRun::default();
        if wanted.is_empty() {
            return Ok(run);
        }
        let build = testbins::build(dir, Some(target_dir))?;
        for target in &wanted {
            let bin = build
                .bins
                .iter()
                .find(|b| b.kind == "test" && &b.target == target);
            match bin {
                None => {
                    run.not_compiled.insert(target.clone());
                }
                Some(bin) => {
                    for (name, o) in testbins::run(bin, &[])? {
                        run.results.insert((target.clone(), name), o);
                    }
                }
            }
        }
        Ok(run)
    }

    fn all_green(&self, dir: &Path, target_dir: &Path) -> Result<bool> {
        let out = capture(
            cargo(dir)
                .args(["test", "--workspace", "--locked"])
                .env("CARGO_TARGET_DIR", target_dir),
        )?;
        Ok(out.ok)
    }
}

/// Classify the base evidence. An empty run is never evidence.
pub fn base_fail_kind(base: &AccRun) -> &'static str {
    let any_pass = base.results.values().any(|o| *o == Outcome::Passed);
    if base.results.is_empty() && base.not_compiled.is_empty() {
        "no_acceptance_tests"
    } else if any_pass {
        "passes"
    } else if base.not_compiled.is_empty() {
        "fails"
    } else if base.results.is_empty() {
        "does_not_compile"
    } else {
        "mixed"
    }
}

/// The `join` record of `metrics/tasks.jsonl` (METHODOLOGY §13 fields first, then extras).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct JoinRecord {
    /// Always `join`.
    pub kind: &'static str,
    /// `T-NNN`.
    pub task: String,
    /// UTC time of the join.
    pub at: String,
    /// Acceptance tests on the joined branch.
    pub acc_total: u64,
    /// Acceptance tests that did not pass on base (failed, or their target did not compile).
    pub acc_fail_base: u64,
    /// `fails` | `does_not_compile` | `mixed` | `passes`.
    pub base_fail_kind: String,
    /// Acceptance tests failing on the joined branch.
    pub acc_fail_first_join: u64,
    /// Failures the director classed as implementation defects (null until classified).
    pub acc_defects: Option<u64>,
    /// The implementer branch's own tests passed before the merge.
    pub impl_tests_green: bool,
    /// The gate passed on the joined branch (null if not run).
    pub gate_green: Option<bool>,
    /// Head of `task/T-NNN-tests` that was joined.
    pub tests_sha: String,
    /// Head of `task/T-NNN` after the join (pre-join head if undone).
    pub joined_sha: String,
    /// `joined`, `acceptance_failed`, `gate_failed`, `refused_base_passes`, `conflict`, `no_acceptance_tests`.
    pub result: String,
}

/// Run the join.
pub fn join(
    root: &Path,
    task: &str,
    acc_defects: Option<u64>,
    clock: &dyn Clock,
    oracle: &dyn TestOracle,
    gate: &dyn GateRunner,
) -> Result<JoinRecord> {
    let spec = Spec::parse(task, false)?;
    let tests = Spec {
        tests: true,
        ..spec.clone()
    };
    let (wt, twt) = (spec.path(root), tests.path(root));
    for (dir, s) in [(&wt, &spec), (&twt, &tests)] {
        if !dir.is_dir() {
            bail!(
                "{} does not exist (cargo xtask worktree new {})",
                dir.display(),
                s.name()
            );
        }
        let d = dirty_paths(dir, None)?;
        if !d.is_empty() {
            bail!(
                "{} has uncommitted changes:\n{}",
                dir.display(),
                d.join("\n")
            );
        }
    }
    let prefix = format!("acceptance_{}", spec.task.to_lowercase().replace('-', ""));
    let target = wt.join("target");
    let tests_target = twt.join("target");
    let mut rec = JoinRecord {
        kind: "join",
        task: spec.task.clone(),
        at: format_utc(clock.now_millis().div_euclid(1000)),
        acc_total: 0,
        acc_fail_base: 0,
        base_fail_kind: String::new(),
        acc_fail_first_join: 0,
        acc_defects,
        impl_tests_green: false,
        gate_green: None,
        tests_sha: rev(&twt, "HEAD")?,
        joined_sha: rev(&wt, "HEAD")?,
        result: String::new(),
    };
    let metrics = root.join("metrics/tasks.jsonl");

    let base = oracle.acceptance(&twt, &tests_target, &prefix)?;
    if base.results.is_empty() && base.not_compiled.is_empty() {
        rec.result = "no_acceptance_tests".into();
        append_jsonl(&metrics, &rec)?;
        bail!(
            "no acceptance tests ({prefix}*.rs under crates/*/tests or apps/*/tests) on {}",
            tests.branch()
        );
    }
    rec.base_fail_kind = base_fail_kind(&base).to_owned();
    rec.impl_tests_green = oracle.all_green(&wt, &target)?;

    let pre = rec.joined_sha.clone();
    let msg = format!(
        "Join {} into {}\n\nTask: {}\nRole: director",
        tests.branch(),
        spec.branch(),
        spec.task
    );
    let (merged, _, err) = git_try(
        &wt,
        &["merge", "--no-ff", "--quiet", "-m", &msg, &tests.branch()],
    )?;
    if !merged {
        let _ = git_try(&wt, &["merge", "--abort"]);
        rec.result = "conflict".into();
        append_jsonl(&metrics, &rec)?;
        bail!(
            "merging {} into {} conflicts (aborted): {}",
            tests.branch(),
            spec.branch(),
            err.trim()
        );
    }

    let joined = oracle.acceptance(&wt, &target, &prefix)?;
    rec.acc_total = joined.results.len() as u64;
    rec.acc_fail_first_join = joined
        .results
        .values()
        .filter(|o| **o != Outcome::Passed)
        .count() as u64;
    let per_target = |t: &str| joined.results.keys().filter(|(tt, _)| tt == t).count() as u64;
    rec.acc_fail_base = base
        .results
        .values()
        .filter(|o| **o != Outcome::Passed)
        .count() as u64
        + base.not_compiled.iter().map(|t| per_target(t)).sum::<u64>();

    if rec.base_fail_kind == "passes" {
        git(&wt, &["reset", "--quiet", "--hard", &pre])?;
        rec.result = "refused_base_passes".into();
        append_jsonl(&metrics, &rec)?;
        let passing: Vec<String> = base
            .results
            .iter()
            .filter(|(_, o)| **o == Outcome::Passed)
            .map(|((t, n), _)| format!("{t}::{n}"))
            .collect();
        bail!(
            "acceptance tests pass on base, so they prove nothing; join undone. Passing on base: {}",
            passing.join(", ")
        );
    }
    rec.joined_sha = rev(&wt, "HEAD")?;
    if !joined.not_compiled.is_empty() {
        rec.result = "acceptance_failed".into();
        rec.acc_fail_first_join += joined.not_compiled.len() as u64;
    } else if rec.acc_fail_first_join > 0 {
        rec.result = "acceptance_failed".into();
    } else {
        let g = gate.run(&wt, crate::queue::GateXtask::Own)?;
        rec.gate_green = Some(g.passed);
        rec.result = if g.passed {
            "joined".into()
        } else {
            "gate_failed".into()
        };
    }
    append_jsonl(&metrics, &rec)?;
    Ok(rec)
}

/// The `revert_check` record.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RevertRecord {
    /// Always `revert_check`.
    pub kind: &'static str,
    /// `T-NNN`.
    pub task: String,
    /// UTC time.
    pub at: String,
    /// Commits reverted.
    pub reverted: Vec<String>,
    /// Acceptance tests found after the revert.
    pub acc_total: u64,
    /// Of those, not passing.
    pub acc_fail: u64,
    /// `fails` | `does_not_compile` | `mixed` | `passes` | `no_acceptance_tests` | `revert_conflict`.
    pub result_kind: String,
}

/// Implementation commits: non-merge commits on the task branch, not on main or the tests branch.
pub fn implementation_commits(root: &Path, spec: &Spec) -> Result<Vec<String>> {
    let tests = Spec {
        tests: true,
        ..spec.clone()
    };
    let out = git(
        root,
        &[
            "rev-list",
            "--no-merges",
            &spec.branch(),
            "^main",
            &format!("^{}", tests.branch()),
        ],
    )?;
    Ok(out.lines().map(str::to_owned).collect())
}

/// Run the revert check.
pub fn revert_check(
    root: &Path,
    task: &str,
    only: &[String],
    clock: &dyn Clock,
    oracle: &dyn TestOracle,
) -> Result<RevertRecord> {
    let spec = Spec::parse(task, false)?;
    let tests = Spec {
        tests: true,
        ..spec.clone()
    };
    let tests_ahead: u64 = git(
        root,
        &["rev-list", "--count", &format!("main..{}", tests.branch())],
    )?
    .parse()?;
    if tests_ahead == 0 {
        bail!(
            "{} has no commits beyond main: there are no acceptance tests to check",
            tests.branch()
        );
    }
    if !git_try(
        root,
        &[
            "merge-base",
            "--is-ancestor",
            &tests.branch(),
            &spec.branch(),
        ],
    )?
    .0
    {
        bail!(
            "{} is not joined into {} yet (run cargo xtask join {})",
            tests.branch(),
            spec.branch(),
            spec.task
        );
    }
    let commits = if only.is_empty() {
        implementation_commits(root, &spec)?
    } else {
        only.iter()
            .map(|c| rev(root, c))
            .collect::<Result<Vec<_>>>()?
    };
    if commits.is_empty() {
        bail!("no implementation commits to revert");
    }
    let tmp: PathBuf = root
        .join(".worktrees")
        .join(format!("{}-revert-check", spec.task));
    if tmp.exists() {
        let _ = git_try(
            root,
            &["worktree", "remove", "--force", &tmp.to_string_lossy()],
        );
    }
    git(
        root,
        &[
            "worktree",
            "add",
            "--quiet",
            "--detach",
            &tmp.to_string_lossy(),
            &spec.branch(),
        ],
    )?;
    let result = (|| -> Result<(u64, u64, String)> {
        let mut args = vec!["revert", "--no-commit"];
        args.extend(commits.iter().map(String::as_str)); // rev-list order: newest first
        let (ok, _, err) = git_try(&tmp, &args)?;
        if !ok {
            eprintln!("revert-check: reverting conflicts: {}", err.trim());
            return Ok((0, 0, "revert_conflict".into()));
        }
        let prefix = format!("acceptance_{}", spec.task.to_lowercase().replace('-', ""));
        // Never share a target directory between two source trees: cargo's mtime-based
        // freshness can then reuse artifacts built from the other tree (bootstrap §7 defect 4).
        let target = tmp.join("target");
        // The private target gets copies of non-member artifacts only (`crate::seed`); the task
        // worktree's target is only read.
        let task_target = spec.path(root).join("target");
        if task_target.is_dir() {
            crate::seed::seed(&spec.path(root), &task_target, &tmp, &target)?;
        }
        let run = oracle.acceptance(&tmp, &target, &prefix)?;
        let fail = run
            .results
            .values()
            .filter(|o| **o != Outcome::Passed)
            .count() as u64;
        Ok((
            run.results.len() as u64,
            fail,
            base_fail_kind(&run).to_owned(),
        ))
    })();
    let _ = git_try(
        root,
        &["worktree", "remove", "--force", &tmp.to_string_lossy()],
    );
    let (acc_total, acc_fail, result_kind) = result?;
    let rec = RevertRecord {
        kind: "revert_check",
        task: spec.task.clone(),
        at: format_utc(clock.now_millis().div_euclid(1000)),
        reverted: commits,
        acc_total,
        acc_fail,
        result_kind,
    };
    append_jsonl(&root.join("metrics/tasks.jsonl"), &rec)?;
    Ok(rec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::testrepo;
    use crate::queue::GateOutcome;
    use crate::util::read_jsonl;
    use crate::worktree;

    struct FixedClock;
    impl Clock for FixedClock {
        fn now_millis(&self) -> i64 {
            1_793_000_000_000
        }
    }

    /// Acceptance tests "pass" when the implementation file says `impl = done`, "fail" when it
    /// exists without that, and "do not compile" when it is absent.
    struct FileOracle;
    impl TestOracle for FileOracle {
        fn acceptance(&self, dir: &Path, _: &Path, prefix: &str) -> Result<AccRun> {
            let mut run = AccRun::default();
            for t in acceptance_targets(dir, prefix)? {
                match std::fs::read_to_string(dir.join("crates/k/src/lib.rs")) {
                    Err(_) => {
                        run.not_compiled.insert(t);
                    }
                    Ok(src) => {
                        let o = if src.contains("impl = done") {
                            Outcome::Passed
                        } else {
                            Outcome::Failed
                        };
                        run.results.insert((t.clone(), "ac_1".into()), o);
                        run.results.insert((t, "ac_2".into()), o);
                    }
                }
            }
            Ok(run)
        }
        fn all_green(&self, _: &Path, _: &Path) -> Result<bool> {
            Ok(true)
        }
    }

    struct Gate(bool);
    impl GateRunner for Gate {
        fn run(&self, _: &Path, _: crate::queue::GateXtask<'_>) -> Result<GateOutcome> {
            Ok(GateOutcome {
                passed: self.0,
                failed_step: None,
            })
        }
    }

    fn setup(tag: &str, base_lib: Option<&str>) -> crate::util::testutil::TempDir {
        let mut files = vec![(".gitignore", ".worktrees/\n"), ("README", "r")];
        if let Some(l) = base_lib {
            files.push(("crates/k/src/lib.rs", l));
        }
        let repo = testrepo::init(tag, &files);
        let root = repo.path();
        for tests in [false, true] {
            worktree::create(
                root,
                &Spec {
                    task: "T-004".into(),
                    tests,
                },
                false,
            )
            .unwrap();
        }
        testrepo::commit(
            &root.join(".worktrees/T-004-tests"),
            "tests",
            &[("crates/k/tests/acceptance_t004.rs", "#[test] fn ac_1() {}")],
        );
        repo
    }

    #[test]
    fn join_with_failing_base_and_passing_implementation() {
        let repo = setup("join-ok", Some("// stub\n"));
        let root = repo.path();
        testrepo::commit(
            &root.join(".worktrees/T-004"),
            "impl",
            &[("crates/k/src/lib.rs", "impl = done\n")],
        );
        let r = join(root, "T-004", None, &FixedClock, &FileOracle, &Gate(true)).unwrap();
        assert_eq!(
            (
                r.base_fail_kind.as_str(),
                r.acc_total,
                r.acc_fail_base,
                r.acc_fail_first_join
            ),
            ("fails", 2, 2, 0)
        );
        assert_eq!(r.result, "joined");
        assert_eq!(r.at, "2026-10-26T07:33:20Z");
        let recs = read_jsonl(&root.join("metrics/tasks.jsonl")).unwrap();
        assert_eq!(recs[0]["kind"], "join");
        assert!(recs[0]["acc_defects"].is_null());
        // The joined branch holds the tests.
        assert!(
            root.join(".worktrees/T-004/crates/k/tests/acceptance_t004.rs")
                .is_file()
        );
        // Revert check: reverting the implementation makes the acceptance tests fail.
        let rc = revert_check(root, "T-004", &[], &FixedClock, &FileOracle).unwrap();
        assert_eq!(
            (rc.result_kind.as_str(), rc.acc_fail, rc.reverted.len()),
            ("fails", 2, 1)
        );
        assert!(!root.join(".worktrees/T-004-revert-check").exists());
    }

    #[test]
    fn compile_failure_on_base_is_recorded_as_weak_evidence() {
        let repo = setup("join-dnc", None);
        let root = repo.path();
        testrepo::commit(
            &root.join(".worktrees/T-004"),
            "impl",
            &[("crates/k/src/lib.rs", "impl = done\n")],
        );
        let r = join(
            root,
            "T-004",
            Some(0),
            &FixedClock,
            &FileOracle,
            &Gate(true),
        )
        .unwrap();
        assert_eq!(
            (r.base_fail_kind.as_str(), r.acc_fail_base, r.acc_defects),
            ("does_not_compile", 2, Some(0))
        );
        // Reverting the only implementation commit removes the file again: still does not compile.
        let rc = revert_check(root, "T-004", &[], &FixedClock, &FileOracle).unwrap();
        assert_eq!(rc.result_kind, "does_not_compile");
    }

    #[test]
    fn tests_that_pass_on_base_are_refused_and_the_merge_undone() {
        let repo = setup("join-vacuous", Some("impl = done\n"));
        let root = repo.path();
        let wt = root.join(".worktrees/T-004");
        let before = testrepo::commit(&wt, "impl", &[("crates/k/src/other.rs", "x")]);
        let e = join(root, "T-004", None, &FixedClock, &FileOracle, &Gate(true)).unwrap_err();
        assert!(e.to_string().contains("pass on base"), "{e}");
        assert_eq!(rev(&wt, "HEAD").unwrap(), before);
        let recs = read_jsonl(&root.join("metrics/tasks.jsonl")).unwrap();
        assert_eq!(recs[0]["result"], "refused_base_passes");
    }

    #[test]
    fn acceptance_failures_keep_the_merge_for_the_implementer() {
        let repo = setup("join-fail", Some("// stub\n"));
        let root = repo.path();
        testrepo::commit(
            &root.join(".worktrees/T-004"),
            "impl",
            &[("crates/k/src/lib.rs", "// half done\n")],
        );
        let r = join(root, "T-004", None, &FixedClock, &FileOracle, &Gate(true)).unwrap();
        assert_eq!(
            (r.result.as_str(), r.acc_fail_first_join, r.gate_green),
            ("acceptance_failed", 2, None)
        );
    }

    #[test]
    fn revert_check_refuses_without_tests_branch_commits() {
        let repo = testrepo::init(
            "revert-empty",
            &[
                (".gitignore", ".worktrees/\n"),
                ("crates/k/src/lib.rs", "x"),
            ],
        );
        let root = repo.path();
        for tests in [false, true] {
            worktree::create(
                root,
                &Spec {
                    task: "T-009".into(),
                    tests,
                },
                false,
            )
            .unwrap();
        }
        testrepo::commit(
            &root.join(".worktrees/T-009"),
            "impl",
            &[("crates/k/src/lib.rs", "impl = done\n")],
        );
        let e = revert_check(root, "T-009", &[], &FixedClock, &FileOracle).unwrap_err();
        assert!(e.to_string().contains("no commits beyond main"), "{e}");
    }

    #[test]
    fn base_kind_classification() {
        let mut r = AccRun::default();
        assert_eq!(base_fail_kind(&r), "no_acceptance_tests");
        r.results.insert(("a".into(), "x".into()), Outcome::Failed);
        assert_eq!(base_fail_kind(&r), "fails");
        r.not_compiled.insert("b".into());
        assert_eq!(base_fail_kind(&r), "mixed");
        r.results.clear();
        assert_eq!(base_fail_kind(&r), "does_not_compile");
        r.results.insert(("a".into(), "y".into()), Outcome::Passed);
        assert_eq!(base_fail_kind(&r), "passes");
    }
}
