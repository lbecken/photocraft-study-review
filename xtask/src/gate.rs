//! `cargo xtask ci`: the single gate (CHARTER §4), identical locally and in CI.
//!
//! Steps run cheap first, each printing one PASS/FAIL/EMPTY line with its time; the gate stops
//! at the first failure unless `--keep-going`. EMPTY means the step exists but nothing is
//! registered for it yet; it is listed in the summary, never silently passed.

use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use serde::Serialize;

use crate::util::{Status, StepResult, capture, cargo, tail};

/// The steps in CHARTER §4 order.
pub const STEPS: &[&str] = &[
    "fmt",
    "clippy",
    "layers",
    "ui-commands",
    "schema",
    "scorecard",
    "test",
    "no-skip",
    "fuzz",
    "budgets",
];

fn fmt_step(root: &Path) -> Result<StepResult> {
    let out = capture(cargo(root).args(["fmt", "--all", "--", "--check"]))?;
    Ok(if out.ok {
        StepResult::pass("formatted")
    } else {
        StepResult::fail(format!("run `cargo fmt --all`:\n{}", tail(&out.stdout, 40)))
    })
}

fn clippy_step(root: &Path) -> Result<StepResult> {
    let out = capture(cargo(root).args([
        "clippy",
        "--workspace",
        "--all-targets",
        "--locked",
        "--",
        "-D",
        "warnings",
    ]))?;
    Ok(if out.ok {
        StepResult::pass("no warnings (all targets, -D warnings)")
    } else {
        StepResult::fail(tail(&out.stderr, 60))
    })
}

/// Totals parsed from `cargo test` output.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct TestTotals {
    /// Passed tests.
    pub passed: u64,
    /// Failed tests.
    pub failed: u64,
    /// Ignored tests.
    pub ignored: u64,
    /// Number of `test result:` lines (test executables and doc-test runs).
    pub suites: u64,
}

/// Sum the `test result:` lines.
pub fn parse_totals(stdout: &str) -> TestTotals {
    let mut t = TestTotals::default();
    for line in stdout.lines() {
        let Some(rest) = line.trim().strip_prefix("test result: ") else {
            continue;
        };
        t.suites += 1;
        for part in rest.split([';', '.']) {
            let mut w = part.split_whitespace();
            if let (Some(n), Some(label)) = (w.next(), w.next())
                && let Ok(n) = n.parse::<u64>()
            {
                match label {
                    "passed" => t.passed += n,
                    "failed" => t.failed += n,
                    "ignored" => t.ignored += n,
                    _ => {}
                }
            }
        }
    }
    t
}

/// Decide the test step from the exit status and totals. Zero tests is a failure.
pub fn judge_tests(ok: bool, t: &TestTotals) -> StepResult {
    let summary = format!(
        "{} passed, {} failed, {} ignored in {} suites",
        t.passed, t.failed, t.ignored, t.suites
    );
    if !ok || t.failed > 0 {
        StepResult::fail(summary)
    } else if t.passed == 0 {
        StepResult::fail(format!("{summary}: zero tests ran"))
    } else {
        StepResult::pass(summary)
    }
}

fn test_step(root: &Path) -> Result<StepResult> {
    let out = capture(cargo(root).args(["test", "--workspace", "--locked"]))?;
    let totals = parse_totals(&out.stdout);
    let mut r = judge_tests(out.ok, &totals);
    if r.status == Status::Fail {
        let failures: Vec<&str> = out
            .stdout
            .lines()
            .filter(|l| l.ends_with("FAILED") || l.starts_with("---- "))
            .collect();
        r.detail = format!(
            "{}\n{}\n{}",
            r.detail,
            failures.join("\n"),
            tail(&out.stderr, 25)
        );
    }
    Ok(r)
}

/// Run one step by name.
pub fn run_step(root: &Path, name: &str) -> Result<StepResult> {
    match name {
        "fmt" => fmt_step(root),
        "clippy" => clippy_step(root),
        "layers" => crate::layers::check(root),
        "ui-commands" => crate::uicmds::check(root),
        "schema" => crate::schema::run(root, false),
        "scorecard" => crate::scorecard::run(root, true),
        "test" => test_step(root),
        "no-skip" => crate::noskip::check(root, &crate::noskip::today(root)?),
        "fuzz" | "budgets" => crate::registries::check(root, name),
        other => anyhow::bail!("unknown gate step `{other}`"),
    }
}

/// One step's line in the report.
#[derive(Debug, Serialize)]
pub struct StepReport {
    /// Step name.
    pub name: String,
    /// Outcome.
    pub status: Status,
    /// Wall seconds.
    pub seconds: f64,
    /// Detail text.
    pub detail: String,
}

/// The machine-readable gate report (`--report <path>`), read by the queue.
#[derive(Debug, Serialize)]
pub struct Report {
    /// Whether the gate passed.
    pub passed: bool,
    /// The first failed step, if any.
    pub failed_step: Option<String>,
    /// Total wall seconds.
    pub seconds: f64,
    /// Steps that ran, in order.
    pub steps: Vec<StepReport>,
}

fn label(s: Status) -> &'static str {
    match s {
        Status::Pass => "PASS ",
        Status::Fail => "FAIL ",
        Status::Empty => "EMPTY",
    }
}

/// Run the gate. Returns whether it passed.
pub fn run(root: &Path, keep_going: bool, report: Option<&Path>) -> Result<bool> {
    let start = Instant::now();
    let mut steps = Vec::new();
    println!("gate: {} steps: {}", STEPS.len(), STEPS.join(", "));
    for (i, name) in STEPS.iter().enumerate() {
        println!("---- [{}/{}] {name}", i + 1, STEPS.len());
        let t0 = Instant::now();
        let r = run_step(root, name).unwrap_or_else(|e| StepResult::fail(format!("error: {e:#}")));
        let secs = t0.elapsed().as_secs_f64();
        let mut lines = r.detail.lines();
        println!(
            "{} {name:<12} {secs:>6.1}s  {}",
            label(r.status),
            lines.next().unwrap_or_default()
        );
        for l in lines {
            println!("      {l}");
        }
        let failed = r.status == Status::Fail;
        steps.push(StepReport {
            name: (*name).to_owned(),
            status: r.status,
            seconds: secs,
            detail: r.detail,
        });
        if failed && !keep_going {
            break;
        }
    }
    let failed: Vec<&str> = steps
        .iter()
        .filter(|s| s.status == Status::Fail)
        .map(|s| s.name.as_str())
        .collect();
    let empty: Vec<&str> = steps
        .iter()
        .filter(|s| s.status == Status::Empty)
        .map(|s| s.name.as_str())
        .collect();
    let total = start.elapsed().as_secs_f64();
    let passed = failed.is_empty();
    if passed {
        let note = if empty.is_empty() {
            String::new()
        } else {
            format!(
                "; {} step(s) EMPTY (nothing registered): {}",
                empty.len(),
                empty.join(", ")
            )
        };
        println!("GATE PASSED in {total:.1}s ({} steps{note})", steps.len());
    } else {
        let skipped = STEPS.len() - steps.len();
        let tail = if skipped > 0 {
            format!("; {skipped} later step(s) not run")
        } else {
            String::new()
        };
        println!("GATE FAILED in {total:.1}s: {}{tail}", failed.join(", "));
    }
    if let Some(path) = report {
        let r = Report {
            passed,
            failed_step: failed.first().map(|s| (*s).to_owned()),
            seconds: total,
            steps,
        };
        std::fs::write(path, serde_json::to_string_pretty(&r)?)?;
    }
    Ok(passed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn totals_sum_all_suites() {
        let out = "test result: ok. 3 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.01s\n\
                   test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n";
        assert_eq!(
            parse_totals(out),
            TestTotals {
                passed: 5,
                failed: 1,
                ignored: 1,
                suites: 2
            }
        );
    }

    #[test]
    fn zero_tests_fails() {
        let t = parse_totals(
            "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s",
        );
        assert_eq!(judge_tests(true, &t).status, Status::Fail);
        assert_eq!(
            judge_tests(true, &TestTotals::default()).status,
            Status::Fail
        );
    }

    #[test]
    fn failures_and_exit_status_fail() {
        let t = TestTotals {
            passed: 4,
            failed: 0,
            ignored: 0,
            suites: 1,
        };
        assert_eq!(judge_tests(true, &t).status, Status::Pass);
        assert_eq!(judge_tests(false, &t).status, Status::Fail);
        let f = TestTotals { failed: 1, ..t };
        assert_eq!(judge_tests(true, &f).status, Status::Fail);
    }

    #[test]
    fn step_order_matches_charter() {
        assert_eq!(STEPS.first(), Some(&"fmt"));
        let pos = |s: &str| STEPS.iter().position(|x| *x == s).unwrap();
        assert!(pos("clippy") < pos("layers") && pos("layers") < pos("ui-commands"));
        assert!(pos("schema") < pos("scorecard") && pos("scorecard") < pos("test"));
        assert!(
            pos("test") < pos("no-skip")
                && pos("no-skip") < pos("fuzz")
                && pos("fuzz") < pos("budgets")
        );
    }
}
