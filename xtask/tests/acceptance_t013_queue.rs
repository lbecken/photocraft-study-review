//! T-013 acceptance tests, Interface (2) queue precondition `budget` (AC-6) and (3) `worktree
//! new` needs `ready` on main (AC-7, and AC-13's `CARGO_TARGET_DIR` line).
//!
//! Written from the task file alone. Toy repositories from T-005's fixture, under
//! `CARGO_TARGET_TMPDIR`, driven through the `xtask` binary only.

use std::path::Path;

#[path = "acceptance_t013_support.rs"]
mod t13;
use t13::*;

#[cfg(test)]
/// `docs/decisions.md`: the toy table plus one row per Decision cell, then `trailer`.
fn decisions(cells: &[&str], trailer: &str) -> String {
    let mut s = DECISIONS.to_owned();
    for (i, c) in cells.iter().enumerate() {
        s.push_str(&format!(
            "| D-{:03} | 2026-11-13 | {c} | x | yes |\n",
            i + 10
        ));
    }
    s.push_str(trailer);
    s
}

/// Rows and lines that are not an extension of T-001 (Interface (2), AC-6).
const NOT_EXTENSIONS: &[&str] = &[
    "Extend T-001 for the rework.",
    "Extend T-001-tests to 5000",
    "Extend T-0010 to 5000",
    "We Extend T-001 to 5000",
];
/// A fenced code block and a plain paragraph with an `Extend` line, after the table; a row
/// whose Reasoning cell (not Decision) says `Extend`.
const TRAILER: &str = "\n| # | Date | Decision | Reasoning | Reversible? |\n|---|---|---|---|---|\n| D-090 | 2026-11-13 | Note. | Extend T-001 to 7000 | yes |\n\nExtend T-001 to 8000\n\n```text\n| # | Date | Decision | Reasoning | Reversible? |\n|---|---|---|---|---|\n| D-099 | 2026-11-13 | Extend T-001 to 9000 | x | yes |\n```\n";

#[cfg(test)]
fn commit_decisions(r: &Repo, cells: &[&str]) {
    r.main_commit(
        "director: decisions",
        &[("docs/decisions.md", Some(&decisions(cells, TRAILER)))],
    );
}

#[cfg(test)]
/// Append run records to main's committed `metrics/tasks.jsonl`.
fn commit_runs(r: &Repo, task: &str, tier: &str, tokens: &[Option<u64>]) {
    let mut lines: Vec<String> = std::fs::read_to_string(r.root.join("metrics/tasks.jsonl"))
        .map(|t| t.lines().map(str::to_owned).collect())
        .unwrap_or_default();
    for (i, t) in tokens.iter().enumerate() {
        lines.push(run_rec(task, Some(tier), 0, *t, &ts(now_s() - 3600 + i as i64)).to_string());
    }
    r.set_tasks_jsonl(&lines);
}

#[cfg(test)]
fn over(task: &str, s: u64, ceiling: u64, source: &str) -> String {
    format!(
        "{task} has {s} recorded tokens, over its ceiling {ceiling} ({source}); a docs/decisions.md row \"Extend {task} to <N>\" with N ≥ {s} is needed"
    )
}

#[test]
fn queue_refuses_a_task_over_its_ceiling_until_an_extend_row_covers_it() {
    let r = Repo::new("t013-ac6-budget");
    // T-001: high, reviewed, joined, approved; 700 + 400 committed tokens over budget 1000.
    r.add_task("T-001", "high", "reviewed", &["crates/a"], &[]);
    r.with_evidence("T-001", "high", &[("crates/a/src/lib.rs", Some(LIB_V2))]);
    commit_runs(&r, "T-001", "high", &[Some(700), Some(400)]);
    let approve = "Approve T-001 for merge.";
    commit_decisions(&r, &[approve]);
    let reason = r.assert_refused("T-001", "budget", None);
    assert_eq!(reason, over("T-001", 1100, 1000, "budget_tokens"));

    // Rows without a ceiling, of another id, not at the cell's start, in another cell, outside
    // a table or inside a fenced block do not count.
    let mut cells = vec![approve];
    cells.extend_from_slice(NOT_EXTENSIONS);
    commit_decisions(&r, &cells);
    let reason = r.assert_refused("T-001", "budget", None);
    assert_eq!(reason, over("T-001", 1100, 1000, "budget_tokens"));

    // A ceiling below the sum: refused, naming it.
    cells.push("Extend T-001 to 1050.");
    commit_decisions(&r, &cells);
    let reason = r.assert_refused("T-001", "budget", None);
    assert_eq!(reason, over("T-001", 1100, 1050, "Extend row"));

    // The last row wins: a later lower ceiling overrides an earlier higher one.
    cells.push("Extend T-001 to 2000 (review rework)");
    cells.push("Extend T-001 to 1099");
    commit_decisions(&r, &cells);
    let reason = r.assert_refused("T-001", "budget", None);
    assert_eq!(reason, over("T-001", 1100, 1099, "Extend row"));

    // `budget` comes before `main-history`.
    let good = r.main_sha();
    r.main_commit(
        "direct code on main",
        &[("other/keep.txt", Some("direct\n"))],
    );
    r.assert_refused("T-001", "budget", None);
    r.reset_main(&good);

    // N equal to the sum: merges.
    cells.push("Extend T-001 to 1100");
    commit_decisions(&r, &cells);
    r.assert_merged("T-001");
}

#[test]
fn budget_sits_after_approval_counts_unknown_runs_as_zero_and_needs_budget_tokens() {
    let r = Repo::new("t013-ac6-order");
    // Over budget and unapproved: `approval` is refused first.
    r.add_task("T-002", "high", "reviewed", &["crates/a"], &[]);
    r.with_evidence("T-002", "high", &[("crates/a/src/lib.rs", Some(LIB_V2))]);
    commit_runs(&r, "T-002", "high", &[Some(5000)]);
    r.assert_refused("T-002", "approval", None);

    // A task file without budget_tokens fails `budget`.
    let no_budget = task_md("T-004", "low", "self_verified", &["notes"], &[])
        .replace("budget_tokens: 1000\n", "");
    assert!(!no_budget.contains("budget_tokens"));
    r.main_commit(
        "director: add T-004",
        &[("tasks/T-004-toy.md", Some(&no_budget))],
    );
    r.worktree("T-004");
    r.commit_wt("T-004", "notes", &[("notes/shared.txt", Some("t4\n"))]);
    let reason = r.assert_refused("T-004", "budget", None);
    assert!(reason.contains("T-004"), "{reason}");

    // Within budget (900 known + one unknown run, counted 0).
    r.add_task("T-003", "low", "self_verified", &["other"], &[]);
    r.commit_wt("T-003", "keep", &[("other/keep.txt", Some("t3\n"))]);
    commit_runs(&r, "T-003", "low", &[Some(900), None]);
    // Uncommitted run records in the working copy are not read: `ledger` refuses first.
    let mut text = read(&r.root, "metrics/tasks.jsonl");
    text.push_str(&run_rec("T-003", Some("low"), 0, Some(50_000), &ts(now_s() - 60)).to_string());
    text.push('\n');
    write(&r.root, "metrics/tasks.jsonl", &text);
    r.assert_refused("T-003", "ledger", Some("metrics/tasks.jsonl"));
    git(&r.root, &["checkout", "--", "metrics/tasks.jsonl"]);
    r.assert_merged("T-003");
}

// ---- AC-7 worktree new needs ready ----------------------------------------------------------

#[cfg(test)]
/// Everything `worktree new` could change.
fn wt_snap(root: &Path) -> (String, String, String, Vec<String>) {
    let mut dirs: Vec<String> = std::fs::read_dir(root.join(".worktrees"))
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    dirs.sort();
    (
        git(root, &["for-each-ref", "--format=%(refname) %(objectname)"]),
        git(root, &["worktree", "list", "--porcelain"]),
        status(root),
        dirs,
    )
}

#[cfg(test)]
fn assert_wt_refused(r: &Repo, args: &[&str], line: &str) {
    let before = wt_snap(&r.root);
    let mut a = vec!["worktree", "new"];
    a.extend_from_slice(args);
    let o = xt(&r.root, &a);
    let got = assert_refusal(&o, "worktree: refused: ", &format!("worktree new {args:?}"));
    assert_eq!(got, line, "worktree new {args:?}");
    assert_eq!(
        wt_snap(&r.root),
        before,
        "worktree new {args:?} changed something"
    );
}

#[cfg(test)]
fn assert_wt_created(r: &Repo, args: &[&str], name: &str) {
    let mut a = vec!["worktree", "new"];
    a.extend_from_slice(args);
    a.push("--no-seed");
    let o = xt(&r.root, &a);
    assert_ok(&o, &format!("worktree new {args:?}"));
    assert!(r.wt(name).is_dir(), "no .worktrees/{name}");
    git(
        &r.root,
        &["rev-parse", "--verify", &format!("refs/heads/task/{name}")],
    );
    // AC-13: no CARGO_TARGET_DIR line any more.
    assert!(
        !o.stdout.contains("CARGO_TARGET_DIR") && !o.stderr.contains("CARGO_TARGET_DIR"),
        "worktree new prints CARGO_TARGET_DIR:\n{}{}",
        o.stdout,
        o.stderr
    );
}

#[test]
fn worktree_new_needs_the_task_ready_on_main() {
    let r = Repo::new("t013-ac7-ready");
    let file = "tasks/T-001-toy.md";
    let spec = task_md("T-001", "medium", "specified", &["crates/a"], &[]);
    r.main_commit("director: add T-001", &[(file, Some(&spec))]);
    let line = "worktree: refused: T-001 is specified on main; worktree new needs ready";
    for args in [
        &["T-001"][..],
        &["T-001", "--tests"],
        &["T-001-tests"],
        &["T-001", "--no-seed"],
    ] {
        assert_wt_refused(&r, args, line);
    }
    // `ready` only in the uncommitted working copy: still refused (main's committed tree).
    let ready = spec.replace("state: specified", "state: ready");
    write(&r.root, file, &ready);
    assert_wt_refused(&r, &["T-001"], line);
    assert_wt_refused(&r, &["T-001-tests"], line);
    // Every other state is refused with its name.
    for st in [
        "draft",
        "in_progress",
        "self_verified",
        "tests_joined",
        "reviewed",
        "queued",
        "merged",
        "evaluated",
        "blocked",
        "abandoned",
    ] {
        r.main_commit(
            &format!("director: T-001 {st}"),
            &[(
                file,
                Some(&spec.replace("state: specified", &format!("state: {st}"))),
            )],
        );
        assert_wt_refused(
            &r,
            &["T-001"],
            &format!("worktree: refused: T-001 is {st} on main; worktree new needs ready"),
        );
    }
    // No task file of that id on main.
    assert_wt_refused(
        &r,
        &["T-005"],
        "worktree: refused: no tasks/T-005-*.md on main",
    );
    assert_wt_refused(
        &r,
        &["T-005-tests"],
        "worktree: refused: no tasks/T-005-*.md on main",
    );
    // A file only in the working copy does not count either.
    write(
        &r.root,
        "tasks/T-006-toy.md",
        &task_md("T-006", "low", "ready", &["crates/a"], &[]),
    );
    assert_wt_refused(
        &r,
        &["T-006"],
        "worktree: refused: no tasks/T-006-*.md on main",
    );

    // Committed `ready` on main (working copy says otherwise): created, in all three forms.
    r.main_commit("director: T-001 ready", &[(file, Some(&ready))]);
    write(&r.root, file, &spec);
    assert_wt_created(&r, &["T-001"], "T-001");
    assert_wt_created(&r, &["T-001", "--tests"], "T-001-tests");
    let t2 = task_md("T-002", "high", "ready", &["crates/a"], &[]);
    r.main_commit("director: add T-002", &[("tasks/T-002-toy.md", Some(&t2))]);
    assert_wt_created(&r, &["T-002-tests"], "T-002-tests");
}

#[test]
fn worktree_new_checks_no_state_without_a_tasks_directory_on_main() {
    let r = Repo::new("t013-ac7-notasks");
    // Even an uncommitted task file in the working copy is not read.
    write(
        &r.root,
        "tasks/T-001-toy.md",
        &task_md("T-001", "low", "draft", &["crates/a"], &[]),
    );
    assert_wt_created(&r, &["T-001"], "T-001");
    assert_wt_created(&r, &["T-002-tests"], "T-002-tests");
}
