//! T-013 acceptance tests, Interface (7): the stale v1.0 references (AC-13). The `worktree new`
//! output is checked in `acceptance_t013_queue.rs`; here: the join's weak-evidence line (a toy
//! join through the binary) and the text of `docs/development.md` (read from this checkout;
//! the reviewer reads the rest).

#[path = "acceptance_t013_support.rs"]
mod t13;
use t13::*;

#[test]
fn a_join_with_weak_base_evidence_points_to_the_mutation_spot_check() {
    let r = Repo::new("t013-ac13-join");
    r.add_task("T-001", "medium", "in_progress", &["crates/a"], &[]);
    r.worktree("T-001-tests");
    r.commit_wt(
        "T-001",
        "implementation",
        &[(
            "crates/a/src/lib.rs",
            Some(&format!("{LIB_V1}\n/// New API.\npub fn extra() -> u32 {{\n    7\n}}\n")),
        )],
    );
    // The test calls the new API: it does not compile on base (weak evidence).
    r.commit_wt(
        "T-001-tests",
        "acceptance test",
        &[(
            "crates/a/tests/acceptance_t001.rs",
            Some("#[test]\nfn extra_is_seven() {\n    assert_eq!(a::extra(), 7);\n}\n"),
        )],
    );
    let o = xt(&r.root, &["join", "T-001"]);
    let rec = std::fs::read_to_string(r.root.join("metrics/tasks.jsonl")).unwrap_or_default();
    assert!(
        rec.contains("\"base_fail_kind\":\"does_not_compile\""),
        "toy join must record weak evidence (exit {}):\n{rec}\n{}\n{}",
        o.code,
        o.stdout,
        o.stderr
    );
    let weak: Vec<&str> = o.stderr.lines().filter(|l| l.starts_with("join: base evidence")).collect();
    assert_eq!(
        weak,
        ["join: base evidence is `does_not_compile` (weak); the reviewer's mutation spot-check must cover it (METHODOLOGY §6)"],
        "stderr:\n{}",
        o.stderr
    );
    assert!(!o.stderr.contains("revert-check"), "stderr:\n{}", o.stderr);
    assert!(!o.stdout.contains("revert-check"), "stdout:\n{}", o.stdout);
}

#[test]
fn development_md_has_no_stale_v10_references() {
    let text = std::fs::read_to_string(checkout_root().join("docs/development.md")).unwrap();
    for stale in [
        "export CARGO_TARGET_DIR",
        "O-2)",
        "Command schema changes are",
        "Timestamps are always passed in",
    ] {
        let at: Vec<usize> = text
            .lines()
            .enumerate()
            .filter(|(_, l)| l.contains(stale))
            .map(|(i, _)| i + 1)
            .collect();
        assert!(at.is_empty(), "docs/development.md still contains `{stale}` at line(s) {at:?}");
    }
    // The Join section no longer requires a revert check or a stub/logic commit split.
    let join = section(&text, "## Join");
    let flat = join.split_whitespace().collect::<Vec<_>>().join(" ");
    for required in [
        "the reviewer must run",
        "split such work into a stub commit and a logic commit",
    ] {
        assert!(!flat.contains(required), "Join section still says `{required}`:\n{join}");
    }
    assert!(
        flat.contains("mutation spot-check"),
        "Join section should name the reviewer's mutation spot-check:\n{join}"
    );
    // The new section and the new record kind are documented.
    assert!(text.contains("Publishing a ledger input"), "no \"Publishing a ledger input\" section");
    assert!(text.contains("join_defects"), "the join_defects record kind is not documented");
    assert!(text.contains("hygiene"), "the hygiene step is not documented");
}

/// The text from the heading starting with `heading` to the next `## ` heading.
fn section<'a>(text: &'a str, heading: &str) -> &'a str {
    let start = text
        .find(&format!("\n{heading}"))
        .unwrap_or_else(|| panic!("no `{heading}` heading"))
        + 1;
    let end = text[start + 3..]
        .find("\n## ")
        .map_or(text.len(), |e| start + 3 + e);
    &text[start..end]
}
