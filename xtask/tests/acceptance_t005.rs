//! Acceptance tests for T-005 (merge queue hardening), written from the task file alone.
//!
//! Every test drives the real `xtask` binary (`CARGO_BIN_EXE_xtask`) against throwaway git
//! repositories created under `CARGO_TARGET_TMPDIR`; nothing here touches the Kiln repository
//! itself. The queue repositories are tiny cargo workspaces with a toy `xtask` package (no
//! dependencies) whose `ci --report <file>` fails exactly when a file `RED` exists, whose
//! `scorecard` writes a deterministic summary of `items/` to `docs/scorecard.md` (and fails when
//! `items/poison.toml` exists), and whose `schema --write` writes a constant `docs/commands.json`.
//!
//! The fixture lives in `acceptance_t005_support.rs`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

#[path = "acceptance_t005_support.rs"]
mod shared;
use shared::support;

use support::*;

/// File changes: path → new content, or `None` to delete.
type Changes<'a> = Vec<(&'a str, Option<&'a str>)>;
/// One file change with owned content.
type OwnedChange = (&'static str, Option<String>);

// ---- AC-1 ---------------------------------------------------------------------------------

#[test]
fn self_weakened_gate_fails_under_mains_gate() {
    let r = Repo::new("ac1");
    let base = r.main_sha();
    assert!(TOY_XTASK_MAIN.contains(GATE_RULE));
    let weakened = TOY_XTASK_MAIN.replace(GATE_RULE, GATE_WEAKENED);
    // An `xtask/` change is a gate path (D-026 R3): only an approved high task reaches the
    // gates, so T-001 carries high-tier evidence and a `gate paths` approval row.
    r.add_task("T-001", "high", "reviewed", &["RED", "xtask"], &[]);
    r.add_task("T-002", "low", "self_verified", &["RED"], &[]);
    r.with_evidence(
        "T-001",
        "high",
        &[
            ("RED", Some("red\n")),
            ("xtask/src/main.rs", Some(&weakened)),
        ],
    );
    let decisions = format!(
        "{DECISIONS}| D-002 | 2026-11-13 | Approve T-001 including its gate paths. | toy | yes |\n"
    );
    r.main_commit(
        "director: approve",
        &[("docs/decisions.md", Some(&decisions))],
    );
    let main_before = r.main_sha();
    assert_ne!(main_before, base);
    let rec = r.queue_record("T-001");
    assert_eq!(
        (
            rec["result"].as_str(),
            rec["failed_step"].as_str(),
            rec["gate_xtask"].as_str()
        ),
        (Some("gate_failed"), Some("main:red"), Some("main+task")),
        "record: {rec}"
    );
    assert_eq!(rec["gate_xtask_sha"], rec["base_sha"]);
    assert_eq!(r.main_sha(), main_before, "main must not move");

    r.commit_wt("T-002", "add RED only", &[("RED", Some("red\n"))]);
    let rec = r.queue_record("T-002");
    assert_eq!(
        (
            rec["result"].as_str(),
            rec["failed_step"].as_str(),
            rec["gate_xtask"].as_str()
        ),
        (Some("gate_failed"), Some("red"), Some("main")),
        "record: {rec}"
    );
    assert_eq!(r.main_sha(), main_before, "main must not move");
}

// ---- AC-2 ---------------------------------------------------------------------------------

#[test]
fn gate_runs_mains_xtask_and_records_it() {
    let r = Repo::new("ac2");
    r.add_task("T-001", "low", "self_verified", &["crates/a"], &[]);
    r.add_task("T-002", "low", "self_verified", &["crates/a"], &[]);
    let weakened = TOY_XTASK_MAIN.replace(GATE_RULE, GATE_WEAKENED);
    r.commit_wt(
        "T-001",
        "code plus a sneaky gate change",
        &[
            ("crates/a/src/lib.rs", Some(LIB_V2)),
            ("xtask/src/main.rs", Some(&weakened)),
        ],
    );
    r.assert_refused("T-001", "touches", Some("xtask/src/main.rs"));

    let base = r.main_sha();
    r.commit_wt(
        "T-002",
        "green change",
        &[("crates/a/src/lib.rs", Some(LIB_V2))],
    );
    let rec = r.assert_merged("T-002");
    assert_eq!(rec["gate_xtask"], "main", "record: {rec}");
    assert_eq!(rec["base_sha"], base.as_str());
    assert_eq!(rec["gate_xtask_sha"], base.as_str(), "record: {rec}");
    assert!(rec["failed_step"].is_null());
    assert_eq!(read(&r.root, "crates/a/src/lib.rs"), LIB_V2);
}

// ---- AC-3 ---------------------------------------------------------------------------------

#[test]
fn refuses_unaccepted_states() {
    let r = Repo::new("ac3");
    let refused_any = [
        "draft",
        "specified",
        "ready",
        "in_progress",
        "tests_joined",
        "merged",
        "blocked",
    ];
    let mut n = 10;
    let mut next = || {
        n += 1;
        format!("T-{n:03}")
    };
    // Every tier refuses these; low also refuses `reviewed`, medium also `self_verified`.
    let mut cases: Vec<(String, &str, &str)> = Vec::new();
    for s in refused_any {
        cases.push((next(), "low", s));
        cases.push((next(), "medium", s));
    }
    cases.push((next(), "low", "reviewed"));
    cases.push((next(), "medium", "self_verified"));
    cases.push((next(), "high", "self_verified"));
    for (id, tier, state) in &cases {
        r.add_task(id, tier, state, &["crates/a"], &[]);
        r.commit_wt(id, "code", &[("crates/a/src/lib.rs", Some(LIB_V2))]);
        r.assert_refused(id, "state", None);
    }
    // Accepted states pass `state`: observed as a refusal at the next failing check
    // (`touches`, by a change to README.md outside `touches`).
    for (tier, state) in [
        ("low", "self_verified"),
        ("low", "queued"),
        ("medium", "reviewed"),
        ("medium", "queued"),
        ("high", "reviewed"),
    ] {
        let id = next();
        r.add_task(&id, tier, state, &["crates/a"], &[]);
        if tier == "low" {
            r.commit_wt(&id, "code", &[("README.md", Some("# changed\n"))]);
        } else {
            r.with_evidence(&id, tier, &[("README.md", Some("# changed\n"))]);
        }
        r.assert_refused(&id, "touches", Some("README.md"));
    }
    // And a low task in `self_verified` really merges.
    let id = next();
    r.add_task(&id, "low", "self_verified", &["crates/a"], &[]);
    r.commit_wt(&id, "code", &[("crates/a/src/lib.rs", Some(LIB_V2))]);
    r.assert_merged(&id);
}

// ---- AC-4 ---------------------------------------------------------------------------------

#[test]
fn refuses_front_matter_changes_on_task_branch() {
    let r = Repo::new("ac4");
    let wt = r.add_task("T-001", "low", "self_verified", &["crates/a"], &[]);
    let base = r.head("T-001");
    let file = "tasks/T-001-toy.md";
    let original = read(&wt, file);
    let variants = [
        ("tier", original.replace("tier: low", "tier: medium")),
        (
            "touches",
            original.replace("  - crates/a\n", "  - crates/a\n  - other\n"),
        ),
        (
            "state",
            original.replace("state: self_verified", "state: queued"),
        ),
        ("id", original.replace("id: T-001", "id: T-002")),
        (
            "scorecard_items",
            original.replace("scorecard_items: []", "scorecard_items: [x]"),
        ),
    ];
    for (what, text) in &variants {
        assert_ne!(text, &original, "variant {what} must change the file");
        r.commit_wt(
            "T-001",
            &format!("change {what}"),
            &[("crates/a/src/lib.rs", Some(LIB_V2)), (file, Some(text))],
        );
        r.assert_refused("T-001", "front-matter", None);
        r.reset_wt("T-001", &base);
    }
    // Appending to the log body only is fine.
    let appended = original.replace(
        "<!-- end implementer log -->",
        "- 2026-11-12 · R-901 · in_progress · did: toy change\n<!-- end implementer log -->",
    );
    r.commit_wt(
        "T-001",
        "code and log entry",
        &[
            ("crates/a/src/lib.rs", Some(LIB_V2)),
            (file, Some(&appended)),
        ],
    );
    r.assert_merged("T-001");
    assert_eq!(read(&r.root, file), appended);
}

// ---- AC-5 ---------------------------------------------------------------------------------

#[test]
fn refuses_paths_outside_touches() {
    // A medium task so that an allowed change is observable without a gate run: it passes
    // `touches` and is refused at the next check, `join` (no join record exists).
    let r = Repo::new("ac5a");
    let touches = ["crates/a", "docs/notes.md", "scratch/"];
    r.add_task("T-001", "medium", "reviewed", &touches, &[]);
    let base = r.head("T-001");
    let task_file = "tasks/T-001-toy.md";
    // Body-only change of the task file (front matter kept).
    let task_appended = read(&r.wt("T-001"), task_file) + "\nappended\n";
    // (description, changes, path the queue must name if refused)
    type Case<'a> = (&'a str, Vec<(&'a str, Option<&'a str>)>, Option<&'a str>);
    let cases: Vec<Case> = vec![
        (
            "file in touched dir",
            vec![("crates/a/src/x.rs", Some("// x\n"))],
            None,
        ),
        (
            "exact file entry",
            vec![("docs/notes.md", Some("more notes\n"))],
            None,
        ),
        (
            "trailing slash entry",
            vec![("scratch/deep/f.txt", Some("f\n"))],
            None,
        ),
        (
            "the task file itself",
            vec![(task_file, Some(task_appended.as_str()))],
            None,
        ),
        (
            "sibling with a common prefix",
            vec![("crates/ab/x.rs", Some("// x\n"))],
            Some("crates/ab/x.rs"),
        ),
        (
            "file extending an exact entry",
            vec![("docs/notes.md.bak", Some("bak\n"))],
            Some("docs/notes.md.bak"),
        ),
        (
            "plain outside path",
            vec![("README.md", Some("# changed\n"))],
            Some("README.md"),
        ),
        (
            "deleted outside path",
            vec![("other/keep.txt", None)],
            Some("other/keep.txt"),
        ),
        (
            "one bad path among good ones",
            vec![
                ("crates/a/src/y.rs", Some("// y\n")),
                ("notes/shared.txt", Some("changed\n")),
            ],
            Some("notes/shared.txt"),
        ),
        (
            "rename out of touches (no rename detection)",
            vec![
                ("crates/a/src/lib.rs", None),
                ("other/lib.rs", Some(LIB_V1)),
            ],
            Some("other/lib.rs"),
        ),
    ];
    for (what, changes, bad) in cases {
        // The model must agree with the expectation.
        let changed: Vec<&str> = changes.iter().map(|(p, _)| *p).collect();
        let model_bad: Vec<&str> = changed
            .iter()
            .copied()
            .filter(|p| !allowed(p, &touches, "medium", "001", &[], task_file))
            .collect();
        assert_eq!(
            model_bad,
            bad.into_iter().collect::<Vec<_>>(),
            "model, case {what}"
        );
        r.commit_wt("T-001", what, &changes);
        match bad {
            Some(p) => {
                r.assert_refused("T-001", "touches", Some(p));
            }
            None => {
                r.assert_refused("T-001", "join", None);
            }
        }
        r.reset_wt("T-001", &base);
    }
}

#[test]
fn allows_acceptance_scorecard_and_review_paths_for_medium_only() {
    let r = Repo::new("ac5b");
    let touches = ["crates/a"];
    r.add_task("T-007", "medium", "reviewed", &touches, &["item-one"]);
    r.add_task("T-008", "low", "self_verified", &touches, &[]);
    let medium_cases: [(&str, bool); 10] = [
        ("crates/a/tests/acceptance_t007_x.rs", true),
        ("crates/b/tests/acceptance_t007.rs", true),
        ("apps/k/tests/acceptance_t007_y.rs", true),
        ("xtask/tests/acceptance_t007.rs", true),
        ("scorecard/items/item-one.toml", true),
        ("reviews/T-007-r1.md", true),
        ("scorecard/items/item-two.toml", false),
        // Another task's acceptance file outside `touches`.
        ("crates/b/tests/acceptance_t008.rs", false),
        ("crates/b/tests/sub/acceptance_t007.rs", false),
        ("crates/b/acceptance_t007.rs", false),
    ];
    let base = r.head("T-007");
    for (p, ok) in medium_cases {
        assert_eq!(
            allowed(
                p,
                &touches,
                "medium",
                "007",
                &["item-one"],
                "tasks/T-007-toy.md"
            ),
            ok,
            "model: {p}"
        );
        r.commit_wt("T-007", p, &[(p, Some("// acceptance\n"))]);
        if ok {
            r.assert_refused("T-007", "join", None);
        } else {
            r.assert_refused("T-007", "touches", Some(p));
        }
        r.reset_wt("T-007", &base);
    }
    let base = r.head("T-008");
    for p in [
        "crates/b/tests/acceptance_t008.rs",
        "xtask/tests/acceptance_t008.rs",
        "reviews/T-008-r1.md",
    ] {
        assert!(
            !allowed(p, &touches, "low", "008", &[], "tasks/T-008-toy.md"),
            "model: {p}"
        );
        r.commit_wt("T-008", p, &[(p, Some("// acceptance\n"))]);
        r.assert_refused("T-008", "touches", Some(p));
        r.reset_wt("T-008", &base);
    }
}

// ---- AC-6 ---------------------------------------------------------------------------------

#[test]
fn refuses_missing_or_failed_join() {
    let r = Repo::new("ac6");
    r.add_task("T-001", "medium", "reviewed", &["crates/a"], &[]);
    let h1 = r.commit_wt("T-001", "impl", &[("crates/a/src/lib.rs", Some(LIB_V2))]);
    r.commit_wt(
        "T-001",
        "review",
        &[(
            "reviews/T-001-r1.md",
            Some(&review_md("T-001", 1, "medium", "approve", &h1)),
        )],
    );
    // No metrics/tasks.jsonl at all.
    r.assert_refused("T-001", "join", None);
    let side = r.side_commit();
    let bad_sets: Vec<Vec<String>> = vec![
        vec![],
        vec![join_rec("T-001", "acceptance_failed", &h1)],
        vec![join_rec("T-001", "gate_failed", &h1)],
        vec![join_rec("T-009", "joined", &h1)],
        vec![join_rec("T-001", "joined", &side)],
        vec![join_rec(
            "T-001",
            "joined",
            "0123456789abcdef0123456789abcdef01234567",
        )],
        vec![
            "{\"kind\":\"revert_check\",\"task\":\"T-001\",\"result\":\"joined\",\"joined_sha\":\""
                .to_owned()
                + &h1
                + "\"}",
        ],
    ];
    for set in &bad_sets {
        r.set_tasks_jsonl(set);
        r.assert_refused("T-001", "join", None);
    }
    // A joined record for an ancestor of the head (h1; the head is the review commit) passes.
    r.set_tasks_jsonl(&[
        join_rec("T-001", "acceptance_failed", &h1),
        join_rec("T-001", "joined", &h1),
    ]);
    r.assert_merged("T-001");
}

// ---- AC-7 ---------------------------------------------------------------------------------

#[test]
fn refuses_missing_rejected_or_stale_review() {
    let r = Repo::new("ac7");
    let wt = r.add_task("T-001", "medium", "reviewed", &["crates/a"], &[]);
    let h1 = r.commit_wt("T-001", "impl", &[("crates/a/src/lib.rs", Some(LIB_V2))]);
    r.set_tasks_jsonl(&[join_rec("T-001", "joined", &h1)]);
    let side = r.side_commit();
    let rv = |n: u32, verdict: &str, sha: &str| review_md("T-001", n, "medium", verdict, sha);

    // Missing review file.
    r.assert_refused("T-001", "review", None);
    // A review file not named `reviews/T-001-r<N>.md` does not count.
    r.commit_wt(
        "T-001",
        "not a round file",
        &[("reviews/T-001-notes.md", Some(&rv(1, "approve", &h1)))],
    );
    r.assert_refused("T-001", "review", None);
    r.reset_wt("T-001", &h1);
    // Highest round requests changes, although round 1 approved.
    r.commit_wt(
        "T-001",
        "r1 approve, r2 request-changes",
        &[
            ("reviews/T-001-r1.md", Some(&rv(1, "approve", &h1))),
            ("reviews/T-001-r2.md", Some(&rv(2, "request-changes", &h1))),
        ],
    );
    r.assert_refused("T-001", "review", None);
    r.reset_wt("T-001", &h1);
    // Rounds are numbers: r10 is higher than r2.
    r.commit_wt(
        "T-001",
        "r2 approve, r10 request-changes",
        &[
            ("reviews/T-001-r2.md", Some(&rv(2, "approve", &h1))),
            (
                "reviews/T-001-r10.md",
                Some(&rv(10, "request-changes", &h1)),
            ),
        ],
    );
    r.assert_refused("T-001", "review", None);
    r.reset_wt("T-001", &h1);
    // reviewed_sha not an ancestor of the head.
    r.commit_wt(
        "T-001",
        "stale sha",
        &[("reviews/T-001-r1.md", Some(&rv(1, "approve", &side)))],
    );
    r.assert_refused("T-001", "review", None);
    r.reset_wt("T-001", &h1);
    // A code change committed after reviewed_sha.
    r.commit_wt(
        "T-001",
        "approve",
        &[("reviews/T-001-r1.md", Some(&rv(1, "approve", &h1)))],
    );
    let reviewed = r.head("T-001");
    r.commit_wt(
        "T-001",
        "sneaky code",
        &[("crates/a/src/extra.rs", Some("// extra\n"))],
    );
    r.assert_refused("T-001", "review", None);
    r.reset_wt("T-001", &reviewed);
    // A later commit touching only the review file and the task log passes.
    let task_file = "tasks/T-001-toy.md";
    let log = read(&wt, task_file).replace(
        "<!-- end implementer log -->",
        "- 2026-11-12 · R-902 · reviewed · did: note\n<!-- end implementer log -->",
    );
    let review = rv(1, "approve", &h1) + "\n## Note\nAddendum.\n";
    r.commit_wt(
        "T-001",
        "review note and log",
        &[
            ("reviews/T-001-r1.md", Some(&review)),
            (task_file, Some(&log)),
        ],
    );
    r.assert_merged("T-001");
}

// ---- AC-8 ---------------------------------------------------------------------------------

#[test]
fn refuses_high_without_approval_row() {
    let r = Repo::new("ac8");
    r.add_task("T-003", "high", "reviewed", &["crates/a"], &[]);
    r.with_evidence("T-003", "high", &[("crates/a/src/lib.rs", Some(LIB_V2))]);
    r.assert_refused("T-003", "approval", None);
    let row = |decision: &str, reason: &str| {
        format!("{DECISIONS}| D-002 | 2026-11-12 | {decision} | {reason} | yes |\n")
    };
    for text in [
        row("Do not Approve T-003 yet.", "toy"),
        row("Approve T-004 for merge.", "toy"),
        row("Defer.", "Approve T-003 later"),
    ] {
        r.main_commit("director: decisions", &[("docs/decisions.md", Some(&text))]);
        r.assert_refused("T-003", "approval", None);
    }
    r.main_commit(
        "director: approve",
        &[(
            "docs/decisions.md",
            Some(&row("Approve T-003 for merge (toy).", "ok")),
        )],
    );
    r.assert_merged("T-003");
}

// ---- AC-9 ---------------------------------------------------------------------------------

#[test]
fn refusals_change_nothing() {
    let r = Repo::new("ac9");
    // A merged record first, so metrics/queue.jsonl is non-empty.
    r.add_task("T-001", "low", "self_verified", &["crates/a"], &[]);
    r.commit_wt("T-001", "code", &[("crates/a/src/lib.rs", Some(LIB_V2))]);
    r.assert_merged("T-001");
    let queue_bytes = std::fs::read(r.root.join("metrics/queue.jsonl")).unwrap();
    let code: &[(&str, Option<&str>)] = &[("crates/a/src/lib.rs", Some(LIB_V1))];

    // task-file: no task file on main.
    r.worktree("T-010");
    r.commit_wt("T-010", "code", code);
    r.assert_refused("T-010", "task-file", None);
    // task-file: two files.
    r.main_commit(
        "director: two files",
        &[
            (
                "tasks/T-011-a.md",
                Some(&task_md(
                    "T-011",
                    "low",
                    "self_verified",
                    &["crates/a"],
                    &[],
                )),
            ),
            (
                "tasks/T-011-b.md",
                Some(&task_md(
                    "T-011",
                    "low",
                    "self_verified",
                    &["crates/a"],
                    &[],
                )),
            ),
        ],
    );
    r.worktree("T-011");
    r.commit_wt("T-011", "code", code);
    r.assert_refused("T-011", "task-file", None);
    // task-file: id mismatch, invalid tier, missing touches.
    let mismatch = task_md("T-013", "low", "self_verified", &["crates/a"], &[]);
    let bad_tier = task_md("T-014", "extreme", "self_verified", &["crates/a"], &[]);
    let no_touches = task_md("T-015", "low", "self_verified", &[], &[]).replace("touches:\n", "");
    for (id, text) in [
        ("T-012", mismatch),
        ("T-014", bad_tier),
        ("T-015", no_touches),
    ] {
        r.main_commit(
            "director: odd task",
            &[(&format!("tasks/{id}-toy.md"), Some(&text))],
        );
        r.worktree(id);
        r.commit_wt(id, "code", code);
        r.assert_refused(id, "task-file", None);
    }
    // task-file: the file exists only on the task branch.
    r.worktree("T-016");
    r.commit_wt(
        "T-016",
        "code and my own task file",
        &[
            ("crates/a/src/lib.rs", Some(LIB_V1)),
            (
                "tasks/T-016-toy.md",
                Some(&task_md(
                    "T-016",
                    "low",
                    "self_verified",
                    &["crates/a"],
                    &[],
                )),
            ),
        ],
    );
    r.assert_refused("T-016", "task-file", None);
    // front-matter.
    let wt = r.add_task("T-017", "low", "self_verified", &["crates/a"], &[]);
    let t = read(&wt, "tasks/T-017-toy.md").replace("tier: low", "tier: high");
    r.commit_wt("T-017", "tier", &[("tasks/T-017-toy.md", Some(&t))]);
    r.assert_refused("T-017", "front-matter", None);
    // state.
    r.add_task("T-018", "low", "draft", &["crates/a"], &[]);
    r.commit_wt("T-018", "code", code);
    r.assert_refused("T-018", "state", None);
    // touches.
    r.add_task("T-019", "low", "self_verified", &["crates/a"], &[]);
    r.commit_wt("T-019", "readme", &[("README.md", Some("# x\n"))]);
    r.assert_refused("T-019", "touches", Some("README.md"));
    // join.
    r.add_task("T-020", "medium", "reviewed", &["crates/a"], &[]);
    r.commit_wt("T-020", "code", code);
    r.assert_refused("T-020", "join", None);
    // review.
    r.add_task("T-021", "medium", "reviewed", &["crates/a"], &[]);
    let h = r.commit_wt("T-021", "code", code);
    r.set_tasks_jsonl(&[join_rec("T-021", "joined", &h)]);
    r.assert_refused("T-021", "review", None);
    // approval.
    r.add_task("T-022", "high", "reviewed", &["crates/a"], &[]);
    r.with_evidence("T-022", "high", code);
    r.assert_refused("T-022", "approval", None);
    // main-history.
    r.add_task("T-023", "low", "self_verified", &["crates/a"], &[]);
    r.commit_wt("T-023", "code", code);
    let good = r.main_sha();
    let bad = r.main_commit(
        "direct code on main",
        &[("other/keep.txt", Some("direct\n"))],
    );
    r.assert_refused("T-023", "main-history", Some(&bad[..7]));
    r.reset_main(&good);

    assert_eq!(
        std::fs::read(r.root.join("metrics/queue.jsonl")).unwrap(),
        queue_bytes,
        "no refusal may write to metrics/queue.jsonl"
    );
}

// ---- AC-12 --------------------------------------------------------------------------------

#[cfg(test)]
mod build {
    use super::support::*;
    use super::*;
    use std::process::Command;

    /// A workspace with one member (`seedmember`, a binary) depending on a path package that
    /// lies outside the workspace and outside the repository (`outsidedep`), referenced by an
    /// absolute path so the primary checkout and every worktree resolve the same package.
    pub fn repo(name: &str) -> PathBuf {
        let s = scratch(name);
        let dep = s.join("outsidedep");
        write(
            &dep,
            "Cargo.toml",
            "[package]\nname = \"outsidedep\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
        );
        write(
            &dep,
            "src/lib.rs",
            "/// Toy.\npub fn base() -> u32 {\n    42\n}\n",
        );
        let root = s.join("repo");
        write(
            &root,
            "Cargo.toml",
            "[workspace]\nresolver = \"2\"\nmembers = [\"seedmember\"]\n",
        );
        write(&root, ".gitignore", ".worktrees/\ntarget/\n");
        write(
            &root,
            "rust-toolchain.toml",
            "[toolchain]\nchannel = \"1.97.0\"\nprofile = \"minimal\"\n",
        );
        write(
            &root,
            "seedmember/Cargo.toml",
            &format!(
                "[package]\nname = \"seedmember\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\noutsidedep = {{ path = \"{}\" }}\n",
                dep.display()
            ),
        );
        write(&root, "seedmember/src/main.rs", &member_src("main-v1"));
        let lock = run(Command::new("cargo")
            .args(["generate-lockfile", "--offline"])
            .current_dir(&root));
        assert_eq!(lock.code, 0, "generate-lockfile: {}", lock.stderr);
        git(&root, &["init", "-q", "-b", "main"]);
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "-q", "-m", "toy base"]);
        root
    }

    pub fn member_src(tag: &str) -> String {
        format!("fn main() {{\n    println!(\"{tag} {{}}\", outsidedep::base());\n}}\n")
    }

    /// `cargo build` in `dir` into `target`; returns (target name, fresh) per artifact.
    pub fn build(dir: &Path, target: &Path) -> Vec<(String, bool)> {
        let o = run(Command::new("cargo")
            .args(["build", "--offline", "--locked", "--message-format=json"])
            .current_dir(dir)
            .env("CARGO_TARGET_DIR", target));
        assert_eq!(o.code, 0, "cargo build in {}: {}", dir.display(), o.stderr);
        o.stdout
            .lines()
            .filter_map(|l| serde_json::from_str::<Value>(l).ok())
            .filter(|v| v["reason"] == "compiler-artifact")
            .map(|v| {
                (
                    v["target"]["name"].as_str().unwrap_or_default().to_owned(),
                    v["fresh"] == Value::Bool(true),
                )
            })
            .collect()
    }

    /// Every entry under `dir`: relative path → (kind, size, content hash).
    pub fn manifest(dir: &Path) -> BTreeMap<String, (char, u64, u64)> {
        use std::hash::{Hash, Hasher};
        let mut out = BTreeMap::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).unwrap() {
                let p = e.unwrap().path();
                let m = std::fs::symlink_metadata(&p).unwrap();
                let rel = p.strip_prefix(dir).unwrap().to_string_lossy().into_owned();
                if m.file_type().is_symlink() {
                    out.insert(rel, ('l', 0, 0));
                } else if m.is_dir() {
                    out.insert(rel, ('d', 0, 0));
                    stack.push(p);
                } else {
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    std::fs::read(&p).unwrap().hash(&mut h);
                    out.insert(rel, ('f', m.len(), h.finish()));
                }
            }
        }
        out
    }
}

#[test]
fn seed_reuses_non_member_artifacts_only() {
    let root = build::repo("ac12");
    let main_target = root.join("target");
    let first = build::build(&root, &main_target);
    assert!(first.iter().any(|(n, _)| n == "outsidedep"), "{first:?}");
    let o = xtask(&root, &["worktree", "new", "T-001"]);
    assert_eq!(o.code, 0, "worktree new: {}{}", o.stdout, o.stderr);
    let wt = root.join(".worktrees/T-001");
    let wt_target = wt.join("target");
    // The seed: no file or directory of the member, no incremental data, but something.
    let seeded = build::manifest(&wt_target);
    let member_files: Vec<&String> = seeded
        .keys()
        .filter(|k| {
            Path::new(k)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .contains("seedmember")
        })
        .collect();
    assert!(
        member_files.is_empty(),
        "member artifacts were seeded: {member_files:?}"
    );
    let incremental: Vec<&String> = seeded
        .keys()
        .filter(|k| {
            Path::new(k)
                .components()
                .any(|c| c.as_os_str() == "incremental")
        })
        .collect();
    assert!(
        incremental.is_empty(),
        "incremental data was seeded: {incremental:?}"
    );
    assert!(
        seeded.keys().any(|k| k.contains("outsidedep")),
        "the non-member's artifacts must be seeded: {seeded:?}"
    );
    // First build in the worktree: the non-member is fresh, the member is rebuilt.
    let built = build::build(&wt, &wt_target);
    let fresh_of = |name: &str| -> Vec<bool> {
        built
            .iter()
            .filter(|(n, _)| n == name)
            .map(|(_, f)| *f)
            .collect()
    };
    assert_eq!(fresh_of("outsidedep"), [true], "{built:?}");
    assert_eq!(fresh_of("seedmember"), [false], "{built:?}");
}

// ---- AC-14 --------------------------------------------------------------------------------

#[test]
fn generated_only_conflict_is_regenerated_and_merged() {
    let r = Repo::new("ac14");
    r.add_task("T-001", "low", "self_verified", &["items"], &[]);
    r.add_task("T-002", "low", "self_verified", &["items"], &[]);
    let lock_before = read(&r.root, "Cargo.lock");
    r.commit_wt(
        "T-001",
        "item a",
        &[
            ("items/a.toml", Some("alpha\n")),
            (
                "docs/scorecard.md",
                Some(&toy_summary(&[("a.toml", "alpha")])),
            ),
        ],
    );
    // T-002 has two commits, each regenerating the scorecard from its own tree.
    r.commit_wt(
        "T-002",
        "item b",
        &[
            ("items/b.toml", Some("beta\n")),
            (
                "docs/scorecard.md",
                Some(&toy_summary(&[("b.toml", "beta")])),
            ),
        ],
    );
    r.commit_wt(
        "T-002",
        "item c",
        &[
            ("items/c.toml", Some("gamma\n")),
            (
                "docs/scorecard.md",
                Some(&toy_summary(&[("b.toml", "beta"), ("c.toml", "gamma")])),
            ),
        ],
    );
    let rec1 = r.assert_merged("T-001");
    assert_eq!(rec1["regenerated"], serde_json::json!([]), "record: {rec1}");
    let rec2 = r.assert_merged("T-002");
    assert_eq!(
        rec2["regenerated"],
        serde_json::json!(["docs/scorecard.md"]),
        "record: {rec2}"
    );
    let expected = toy_summary(&[("a.toml", "alpha"), ("b.toml", "beta"), ("c.toml", "gamma")]);
    assert_eq!(read(&r.root, "docs/scorecard.md"), expected);
    assert_eq!(
        read(&r.root, "Cargo.lock"),
        lock_before,
        "Cargo.lock must not change"
    );
    assert_eq!(read(&r.root, "docs/commands.json"), COMMANDS_JSON);
    let msg = git(&r.root, &["log", "-1", "--format=%B", "main"]);
    let lines: Vec<&str> = msg.lines().map(str::trim).collect();
    for want in [
        "queue: regenerate generated files",
        "Task: T-002",
        "Role: director",
    ] {
        assert!(
            lines.contains(&want),
            "regeneration commit message lacks `{want}`:\n{msg}"
        );
    }
    assert_eq!(
        git(&r.root, &["show", "--name-only", "--format=", "main"]),
        "docs/scorecard.md"
    );
    // Idempotence: regenerating on merged main changes nothing.
    let private = r.root.parent().unwrap().join("regen-target");
    let o = run(std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--offline",
            "--package",
            "xtask",
            "--",
            "scorecard",
        ])
        .current_dir(&r.root)
        .env("CARGO_TARGET_DIR", &private));
    assert_eq!(o.code, 0, "toy scorecard: {}", o.stderr);
    let tracked_changes = git(&r.root, &["status", "--porcelain", "--untracked-files=no"]);
    assert_eq!(tracked_changes, "", "regenerating on main changed files");
}

// ---- AC-15 --------------------------------------------------------------------------------

#[test]
fn mixed_or_failed_regeneration_conflict_is_rejected() {
    let r = Repo::new("ac15");
    for id in ["T-001", "T-002", "T-003"] {
        r.add_task(id, "low", "self_verified", &["items", "notes"], &[]);
    }
    r.commit_wt(
        "T-001",
        "a",
        &[
            ("items/a.toml", Some("alpha\n")),
            ("notes/shared.txt", Some("one\n")),
            (
                "docs/scorecard.md",
                Some(&toy_summary(&[("a.toml", "alpha")])),
            ),
        ],
    );
    r.commit_wt(
        "T-002",
        "b",
        &[
            ("items/b.toml", Some("beta\n")),
            ("notes/shared.txt", Some("two\n")),
            (
                "docs/scorecard.md",
                Some(&toy_summary(&[("b.toml", "beta")])),
            ),
        ],
    );
    r.commit_wt(
        "T-003",
        "poison",
        &[
            ("items/poison.toml", Some("poison\n")),
            (
                "docs/scorecard.md",
                Some("# Scorecard (toy)\n- poison.toml: hand-written\n"),
            ),
        ],
    );
    r.assert_merged("T-001");

    let main_before = r.main_sha();
    let head_before = r.head("T-002");
    let rec = r.queue_record("T-002");
    assert_eq!(
        (rec["result"].as_str(), rec["failed_step"].as_str()),
        (Some("conflict"), Some("rebase")),
        "record: {rec}"
    );
    assert_eq!(r.main_sha(), main_before, "main must not move");
    assert_eq!(r.head("T-002"), head_before, "task branch must not move");
    assert_eq!(
        git(&r.wt("T-002"), &["rev-parse", "--abbrev-ref", "HEAD"]),
        "task/T-002",
        "rebase must be aborted"
    );
    assert_eq!(status(&r.wt("T-002")), "", "task worktree left dirty");

    let rec = r.queue_record("T-003");
    assert_eq!(
        (rec["result"].as_str(), rec["failed_step"].as_str()),
        (Some("conflict"), Some("regenerate:docs/scorecard.md")),
        "record: {rec}"
    );
    assert_eq!(r.main_sha(), main_before, "main must not move");
}

// ---- AC-16 --------------------------------------------------------------------------------

#[test]
fn generated_paths_are_exempt_from_touches_by_mains_list() {
    let r = Repo::new("ac16");
    // Medium tasks: passing `touches` shows as a refusal at `join` (no join record).
    r.add_task("T-001", "medium", "reviewed", &["crates/a"], &[]);
    let lock = read(&r.root, "Cargo.lock");
    let bumped = lock.replace(
        "name = \"a\"\nversion = \"0.1.0\"",
        "name = \"a\"\nversion = \"0.2.0\"",
    );
    assert_ne!(bumped, lock, "toy Cargo.lock lists package a 0.1.0");
    r.commit_wt(
        "T-001",
        "bump a",
        &[
            (
                "crates/a/Cargo.toml",
                Some("[package]\nname = \"a\"\nversion = \"0.2.0\"\nedition = \"2021\"\n"),
            ),
            ("Cargo.lock", Some(&bumped)),
            (
                "docs/commands.json",
                Some("{\"commands\": [\"toy.new\"]}\n"),
            ),
        ],
    );
    r.assert_refused("T-001", "join", None);

    // The branch's own generated.toml grants nothing.
    let wt = r.add_task(
        "T-002",
        "medium",
        "reviewed",
        &["crates/a", "xtask/generated.toml"],
        &[],
    );
    let own = read(&wt, "xtask/generated.toml");
    assert!(
        own.contains("docs/commands.json"),
        "policy lists docs/commands.json:\n{own}"
    );
    let own = own.replace("docs/commands.json", "extra/gen.txt");
    r.commit_wt(
        "T-002",
        "self-exempt a path",
        &[
            ("xtask/generated.toml", Some(&own)),
            ("extra/gen.txt", Some("gen\n")),
        ],
    );
    r.assert_refused("T-002", "touches", Some("extra/gen.txt"));
}

// ---- AC-17 --------------------------------------------------------------------------------

#[test]
fn main_history_tolerates_only_ledger_commits() {
    let r = Repo::new("ac17");
    r.add_task("T-001", "low", "self_verified", &["crates/a"], &[]);
    r.commit_wt("T-001", "code", &[("crates/a/src/lib.rs", Some(LIB_V2))]);
    r.assert_merged("T-001");
    // Director ledger commits after the merge.
    let t1 = read(&r.root, "tasks/T-001-toy.md").replace("state: self_verified", "state: merged");
    r.main_commit(
        "director: T-001 merged",
        &[("tasks/T-001-toy.md", Some(&t1))],
    );
    r.main_commit(
        "director: metrics",
        &[
            ("metrics/notes.txt", Some("notes 1\n")),
            ("metrics/extra.txt", Some("x\n")),
        ],
    );
    r.main_commit("director: state", &[("STATE.md", Some("state 1\n"))]);
    r.main_commit(
        "director: new item",
        &[("scorecard/items/x.toml", Some("id = \"x\"\n"))],
    );
    r.add_task("T-002", "low", "self_verified", &["crates/a"], &[]);
    r.commit_wt("T-002", "code", &[("crates/a/src/lib.rs", Some(LIB_V1))]);
    r.assert_merged("T-002");

    r.add_task("T-003", "low", "self_verified", &["crates/a"], &[]);
    r.commit_wt("T-003", "code", &[("crates/a/src/lib.rs", Some(LIB_V2))]);
    let good = r.main_sha();
    let lock = read(&r.root, "Cargo.lock");
    let lock_changed = lock + "\n";
    let state_ok = "state 2\n";
    let weakened = TOY_XTASK_MAIN.replace(GATE_RULE, GATE_WEAKENED);
    let cases: Vec<(&str, Changes)> = vec![
        ("code", vec![("crates/a/src/extra.rs", Some("// direct\n"))]),
        (
            "xtask",
            vec![("xtask/src/main.rs", Some(weakened.as_str()))],
        ),
        ("lock", vec![("Cargo.lock", Some(lock_changed.as_str()))]),
        (
            "ci config",
            vec![(".github/workflows/ci.yml", Some("on: push\n"))],
        ),
        (
            "item modified",
            vec![("scorecard/items/existing.toml", Some("id = \"changed\"\n"))],
        ),
        (
            "item deleted",
            vec![("scorecard/items/existing.toml", None)],
        ),
        (
            "ledger plus code",
            vec![
                ("STATE.md", Some(state_ok)),
                ("README.md", Some("# direct\n")),
            ],
        ),
    ];
    for (what, files) in &cases {
        let bad = r.main_commit(&format!("direct: {what}"), files);
        r.assert_refused("T-003", "main-history", Some(&bad[..7]));
        // A later ledger commit does not hide the bad one.
        r.main_commit("director: state", &[("STATE.md", Some("state 3\n"))]);
        r.assert_refused("T-003", "main-history", Some(&bad[..7]));
        r.reset_main(&good);
    }
}

// ---- AC-18 --------------------------------------------------------------------------------

#[test]
fn invalid_policy_files_are_refused() {
    let r = Repo::new("ac18");
    r.add_task("T-001", "low", "self_verified", &["crates/a"], &[]);
    r.commit_wt("T-001", "code", &[("crates/a/src/lib.rs", Some(LIB_V2))]);
    let good = r.main_sha();
    let generated = read(&r.root, "xtask/generated.toml");
    let ledger = read(&r.root, "xtask/ledger.toml");
    let cases: Vec<(&str, Vec<OwnedChange>)> = vec![
        (
            "generated.toml missing",
            vec![("xtask/generated.toml", None)],
        ),
        ("ledger.toml missing", vec![("xtask/ledger.toml", None)]),
        (
            "generated.toml not TOML",
            vec![(
                "xtask/generated.toml",
                Some("this is [[[ not toml\n".into()),
            )],
        ),
        (
            "ledger.toml not TOML",
            vec![("xtask/ledger.toml", Some("= = =\n".into()))],
        ),
        (
            "generated.toml unknown key",
            vec![(
                "xtask/generated.toml",
                Some(format!("unknown_key_for_test = true\n{generated}")),
            )],
        ),
        (
            "ledger.toml unknown key",
            vec![(
                "xtask/ledger.toml",
                Some(format!("unknown_key_for_test = true\n{ledger}")),
            )],
        ),
        (
            "generated.toml empty",
            vec![("xtask/generated.toml", Some(String::new()))],
        ),
        (
            "ledger.toml empty",
            vec![("xtask/ledger.toml", Some(String::new()))],
        ),
    ];
    for (what, files) in &cases {
        let files: Vec<(&str, Option<&str>)> =
            files.iter().map(|(p, c)| (*p, c.as_deref())).collect();
        r.main_commit(&format!("break: {what}"), &files);
        r.assert_refused("T-001", "config", None);
        r.reset_main(&good);
    }
    r.assert_merged("T-001");
}
