//! T-013 acceptance tests, Interface (4) and the `join-defects` part of (1): `metrics report`
//! with the v1.1 metric definitions and `metrics join-defects` — AC-8.
//!
//! Written from the task file alone; every test drives the `xtask` binary in a throw-away git
//! repository with fixture records (oracle: hand-computed values, exact lines, exact bytes).

use serde_json::Value;

#[path = "acceptance_t013_support.rs"]
mod t13;
use t13::*;

#[cfg(test)]
/// Lines of the report, trimmed.
fn lines(s: &str) -> Vec<String> {
    s.lines().map(|l| l.trim().to_owned()).collect()
}

#[cfg(test)]
/// Whether `line` contains `n` as a whole number (not part of a longer digit run).
fn has_num(line: &str, n: u64) -> bool {
    let n = n.to_string();
    line.match_indices(&n).any(|(i, _)| {
        let b = line.as_bytes();
        let before = i == 0 || !b[i - 1].is_ascii_digit();
        let j = i + n.len();
        let after = j == b.len() || !b[j].is_ascii_digit();
        before && after
    })
}

struct Fixture {
    l: Lite,
    at35: String,
}

#[cfg(test)]
fn fixture(name: &str) -> Fixture {
    let l = Lite::new(name);
    let task = |id, tier, it, budget| {
        l.task(&TaskSpec {
            budget: Some(budget),
            ..TaskSpec::new(id, tier, it)
        })
    };
    task("T-031", "medium", 1, 500_000);
    task("T-032", "high", 1, 400_000);
    task("T-034", "medium", 1, 500_000);
    task("T-035", "high", 1, 500_000);
    task("T-036", "medium", 0, 500_000);
    task("T-037", "medium", 1, 500_000);
    task("T-038", "medium", 1, 500_000);
    task("T-039", "medium", 0, 500_000);
    for (id, fb, origin) in [
        ("T-040", "reviewer", "T-031"),
        ("T-041", "reviewer", "T-038"),
        ("T-042", "planner", "T-031"),
    ] {
        l.task(&TaskSpec {
            found_by: fb,
            origin: Some(origin),
            ..TaskSpec::new(id, "medium", 1)
        });
    }
    let t = now_s() - 86_400;
    let at = |k: i64| ts(t + k);
    let at35 = at(5);
    l.seed(&[
        run_rec("T-031", Some("medium"), 1, Some(200_000), &at(0)),
        run_rec("T-031", Some("medium"), 1, Some(100_000), &at(1)),
        run_rec("T-031", Some("medium"), 1, None, &at(2)),
        run_rec("T-032", Some("high"), 1, Some(500_000), &at(0)),
        run_rec("T-038", Some("medium"), 1, Some(999_999), &at(0)),
        run_rec("iter-1", None, 1, Some(60_000), &at(0)),
        run_rec("iter-1", None, 1, Some(40_000), &at(1)),
        // Two joins of one task, both with impl_tests_green: every join counts.
        join_rec13("T-031", &at(3), &sha(31), 1, Some(1), true),
        join_rec13("T-031", &at(4), &sha(32), 2, Some(2), true),
        join_rec13("T-032", &at(3), &sha(320), 0, None, true),
        // impl_tests_green: false is not acceptance-caught.
        join_rec13("T-034", &at(3), &sha(34), 5, Some(5), false),
        // An unclassified failing join of an iteration-1 task, and of an iteration-0 task.
        join_rec13("T-035", &at35, &sha(35), 2, None, true),
        join_rec13("T-036", &at(5), &sha(36), 1, None, true),
        join_rec13("T-037", &at(5), &sha(37), 0, None, true),
        join_rec13("T-039", &at(5), &sha(39), 1, None, true),
        join_rec13("T-039", &at(6), &sha(39), 1, None, true),
    ]);
    l.queue_merged(&["T-031", "T-032"]);
    write(
        &l.root,
        "docs/decisions.md",
        &format!(
            "{DECISIONS}| D-002 | 2026-11-13 | Extend T-038 to 1200000 for the rework. | x | yes |\n"
        ),
    );
    Fixture { l, at35 }
}

#[cfg(test)]
fn caught_total(report: &str) -> Option<u64> {
    lines(report).iter().find_map(|l| {
        let rest = l.strip_prefix("acceptance-caught defects: ")?;
        let d: String = rest.chars().take_while(char::is_ascii_digit).collect();
        d.parse().ok()
    })
}

#[test]
fn report_computes_the_v11_metrics() {
    let f = fixture("ac8-report");
    let o = f.l.x(&["metrics", "report"]);
    let r = &o.stdout;
    let ls = lines(r);
    // An unclassified iteration-1 join: still the full report, exit 1, exactly one line.
    assert_eq!(
        o.code, 1,
        "report with an unclassified join must exit 1:\n{r}\n{}",
        o.stderr
    );
    let unclassified: Vec<String> = format!("{}\n{}", o.stdout, o.stderr)
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("unclassified join:"))
        .map(str::to_owned)
        .collect();
    assert_eq!(
        unclassified,
        [format!(
            "unclassified join: T-035 at {} joined_sha {}: 2 failure(s), acc_defects missing",
            f.at35,
            sha(35)
        )],
        "report:\n{r}"
    );
    assert!(
        r.contains("budget overruns"),
        "the report must still be printed in full:\n{r}"
    );

    // Acceptance-caught: 1 + 2 over both joins of T-031; T-034 (impl tests red) not counted.
    assert_eq!(caught_total(r), Some(3), "report:\n{r}");

    // Tokens per merged task without overhead (merged: T-031 medium/odd, T-032 high).
    let line_with = |words: &[&str], n: u64| {
        ls.iter()
            .any(|l| words.iter().all(|w| l.contains(w)) && has_num(l, n))
    };
    assert!(
        line_with(&["medium"], 300_000),
        "medium merged task: 300000\n{r}"
    );
    assert!(
        line_with(&["high"], 500_000),
        "high merged task: 500000\n{r}"
    );
    assert!(
        line_with(&["implementer"], 300_000),
        "implementer arm: 300000\n{r}"
    );
    assert!(
        line_with(&["independent"], 500_000),
        "independent arm: 500000\n{r}"
    );
    // With overhead, iteration 1: (300000 + 500000 + 100000) / 2.
    assert!(
        line_with(&["overhead"], 450_000),
        "with overhead: 450000\n{r}"
    );
    // iter-1 runs are tier `overhead` in tokens by tier.
    assert!(
        ls.iter()
            .any(|l| l.starts_with("overhead") && has_num(l, 100_000)),
        "tokens by tier lists overhead 100000\n{r}"
    );
    assert!(
        !ls.iter().any(|l| l.contains("T-") && l.contains("iter-1")),
        "iter-1 is never in a task's total\n{r}"
    );

    // Known defects merged by decision.
    let low = r.to_lowercase();
    assert!(
        low.contains("known defects merged by decision"),
        "heading missing:\n{r}"
    );
    assert!(
        ls.iter().any(|l| l.contains("T-040 (from T-031)")),
        "T-040 listed\n{r}"
    );
    assert!(
        !r.contains("T-041 (from"),
        "T-041's origin is not merged\n{r}"
    );
    assert!(
        !r.contains("T-042 (from"),
        "T-042 is not found_by reviewer\n{r}"
    );

    // Budget overruns with the extension.
    assert!(
        ls.iter().any(|l| l.contains("T-038")
            && l.contains("extend to: 1200000")
            && l.contains("within ceiling: yes")),
        "T-038 over budget, extended to 1200000, within it\n{r}"
    );
    assert!(
        ls.iter().any(|l| l.contains("T-032")
            && l.contains("extend to: none")
            && l.contains("within ceiling: no")),
        "T-032 over budget, no extension\n{r}"
    );
    assert!(
        !ls.iter()
            .any(|l| l.contains("T-031") && l.contains("within ceiling")),
        "T-031 is within its budget\n{r}"
    );
}

#[cfg(test)]
fn jd(task: &str, s: &str, n: &str) -> Vec<String> {
    [
        "metrics",
        "join-defects",
        "--task",
        task,
        "--joined-sha",
        s,
        "--acc-defects",
        n,
    ]
    .iter()
    .map(|x| (*x).to_owned())
    .collect()
}

#[test]
fn join_defects_appends_one_record_and_classifies_the_join() {
    let f = fixture("ac8-join-defects");
    let l = &f.l;
    let s35 = sha(35);
    // Refusals, each leaving the file byte-identical.
    l.refused(
        &strs(&jd("T-035", &s35, "3")),
        "N > acc_fail_first_join (3 > 2)",
    );
    l.refused(&strs(&jd("T-035", &sha(999), "1")), "no join with that sha");
    l.refused(
        &strs(&jd("T-036", &s35, "1")),
        "the sha is another task's join",
    );
    l.refused(
        &strs(&jd("T-031", &sha(31), "1")),
        "join has its own acc_defects",
    );
    l.refused(
        &strs(&jd("T-037", &sha(37), "0")),
        "acc_fail_first_join is 0",
    );
    l.refused(
        &strs(&jd("T-039", &sha(39), "1")),
        "two joins with that sha",
    );
    l.refused(&strs(&jd("iter-1", &s35, "1")), "iter-1");

    let old = l.bytes();
    let before = now_s();
    let (rec, _) = l.appended(&strs(&jd("T-035", &s35, "1")), "join-defects 1");
    let after = now_s();
    let new = l.bytes();
    assert!(
        new.starts_with(&old),
        "old lines must be a byte prefix of the new file"
    );
    let line = std::str::from_utf8(&new[old.len()..]).unwrap().trim_end();
    assert!(
        !line.contains(": ") && !line.contains(", "),
        "not compact JSON: {line}"
    );
    let obj = rec.as_object().unwrap();
    let keys: Vec<&str> = obj.keys().map(String::as_str).collect();
    let mut want = vec!["acc_defects", "at", "joined_sha", "kind", "task"];
    let mut got = keys.clone();
    got.sort();
    want.sort();
    assert_eq!(got, want, "record: {rec}");
    assert_eq!(rec["kind"], "join_defects");
    assert_eq!(rec["task"], "T-035");
    assert_eq!(rec["joined_sha"], s35.as_str());
    assert_eq!(rec["acc_defects"], 1);
    assert_in_bracket(
        rec["at"].as_str().unwrap(),
        before,
        after,
        "join_defects at",
    );

    // A second classification of the same join is refused.
    l.refused(
        &strs(&jd("T-035", &s35, "1")),
        "earlier join_defects for that join",
    );
    l.refused(
        &strs(&jd("T-035", &s35, "2")),
        "earlier join_defects for that join (other N)",
    );

    // Now the report has no unclassified join, exits 0, and the total grew by 1.
    let o = l.x(&["metrics", "report"]);
    assert_eq!(
        o.code, 0,
        "report after classification:\n{}\n{}",
        o.stdout, o.stderr
    );
    assert!(
        !format!("{}{}", o.stdout, o.stderr).contains("unclassified join:"),
        "{}",
        o.stdout
    );
    assert_eq!(caught_total(&o.stdout), Some(4), "report:\n{}", o.stdout);
    let recs = l.records();
    assert_eq!(
        recs.iter()
            .filter(|r: &&Value| r["kind"] == "join_defects")
            .count(),
        1
    );
}
