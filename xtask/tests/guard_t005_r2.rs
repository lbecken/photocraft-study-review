//! Regression guard for T-005 ruling D-019 (a): the `front-matter` check passes when the task
//! branch's copy of the front matter equals main's or is unchanged since the merge base, so the
//! director may advance `state` on main after the branch was cut. The base queue has no
//! front-matter check and already merges these tasks; the new check must keep doing so.

#[path = "acceptance_t005_support.rs"]
mod shared;
use shared::support::*;

#[test]
fn front_matter_may_lag_or_match_mains_state() {
    let r = Repo::new("r2-front-matter-lag");
    let file1 = "tasks/T-001-toy.md";
    let file2 = "tasks/T-002-toy.md";
    r.add_task("T-001", "low", "in_progress", &["crates/a"], &[]);
    r.add_task("T-002", "low", "in_progress", &["crates/a"], &[]);
    // The director advances both tasks on main after the branches were cut.
    for f in [file1, file2] {
        let t = read(&r.root, f).replace("state: in_progress", "state: self_verified");
        r.main_commit("director: self_verified", &[(f, Some(&t))]);
    }
    // T-001's copy is unchanged since the merge base (it lags main).
    r.commit_wt("T-001", "code", &[("crates/a/src/lib.rs", Some(LIB_V2))]);
    r.assert_merged("T-001");
    // T-002's copy was updated to equal main's.
    let t2 = read(&r.root, file2);
    r.commit_wt(
        "T-002",
        "code and synced front matter",
        &[
            ("crates/a/src/extra.rs", Some("// extra\n")),
            (file2, Some(&t2)),
        ],
    );
    r.assert_merged("T-002");
    assert_eq!(read(&r.root, file2), t2);
}
