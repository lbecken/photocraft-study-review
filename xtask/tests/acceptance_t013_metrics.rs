//! T-013 acceptance tests, Interface (1): `metrics add`, `metrics transition` (clock, time
//! order, METHODOLOGY §3 table, tokens rule, budget line, `iter-N`) — AC-1 … AC-5.
//!
//! Written from the task file alone. Every test drives the `xtask` binary in a throw-away git
//! repository under `CARGO_TARGET_TMPDIR` with isolated git configuration; nothing here reads
//! or writes the Kiln repository.

use serde_json::Value;

#[path = "acceptance_t013_support.rs"]
mod t13;
use t13::*;

#[cfg(test)]
fn tr(task: &str, to: &str, extra: &[&str]) -> Vec<String> {
    let mut a: Vec<String> = ["metrics", "transition", "--task", task, "--to", to]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    a.extend(extra.iter().map(|s| (*s).to_owned()));
    a
}

// ---- AC-1 clock ------------------------------------------------------------------------------

#[test]
fn add_and_transition_record_the_clock_time_by_default() {
    let l = Lite::new("ac1-default");
    l.task(&TaskSpec::new("T-001", "medium", 1));
    let started = ts(now_s() - 600);

    let before = now_s();
    let (rec, _) = l.appended(
        &strs(&add_args(
            "T-001",
            "implementer",
            Some(1000),
            &["--started-at", &started],
        )),
        "add without --ended-at",
    );
    let after = now_s();
    assert_eq!(rec["kind"], "run");
    assert_eq!(rec["started_at"], started.as_str());
    assert_in_bracket(rec["ended_at"].as_str().unwrap(), before, after, "ended_at");

    let before = now_s();
    let (rec, _) = l.appended(
        &strs(&tr("T-001", "specified", &[])),
        "transition without --at and --from",
    );
    let after = now_s();
    assert_eq!(rec["kind"], "transition");
    assert_eq!(
        (rec["from"].as_str(), rec["to"].as_str()),
        (Some("draft"), Some("specified"))
    );
    assert_eq!(rec["iteration"], 1);
    assert_in_bracket(rec["at"].as_str().unwrap(), before, after, "at");
}

#[test]
fn times_before_the_tasks_latest_record_or_after_now_are_refused() {
    let l = Lite::new("ac1-order");
    for id in ["T-002", "T-003", "T-004"] {
        l.task(&TaskSpec::new(id, "medium", 1));
    }
    let now = now_s();
    let l1 = now - 600;
    let l2 = now - 300; // T-002's latest record: a join
    l.seed(&[
        transition_rec("T-002", "draft", "specified", &ts(l1)),
        join_rec13("T-002", &ts(l2), &sha(1), 0, None, true),
        transition_rec("T-003", "draft", "specified", &ts(now - 60)),
    ]);

    // One second earlier than the latest record (a join's `at`): refused.
    l.refused(
        &strs(&tr("T-002", "ready", &["--at", &ts(l2 - 1)])),
        "transition 1 s early",
    );
    // Equal: appended, with exactly that time.
    let (rec, _) = l.appended(
        &strs(&tr("T-002", "ready", &["--at", &ts(l2)])),
        "transition at equal time",
    );
    assert_eq!(rec["at"], ts(l2).as_str());
    assert_eq!(rec["from"], "specified");

    // `add`: a time is `ended_at`; earlier than the latest is refused, equal accepted.
    l.refused(
        &strs(&add_args(
            "T-002",
            "implementer",
            Some(5),
            &["--started-at", &ts(l2 - 100), "--ended-at", &ts(l2 - 1)],
        )),
        "add ended 1 s early",
    );
    l.appended(
        &strs(&add_args(
            "T-002",
            "implementer",
            Some(5),
            &["--started-at", &ts(l2 - 100), "--ended-at", &ts(l2)],
        )),
        "add ended at equal time",
    );
    // A run's time is its `ended_at`, not its `started_at`.
    l.appended(
        &strs(&add_args(
            "T-002",
            "implementer",
            Some(5),
            &["--started-at", &ts(l2 + 20), "--ended-at", &ts(l2 + 100)],
        )),
        "add ended later",
    );
    l.refused(
        &strs(&tr("T-002", "in_progress", &["--at", &ts(l2 + 60)])),
        "transition between the run's start and end",
    );

    // In the future (one hour after now): refused.
    let hour = now_s() + 3600;
    l.refused(
        &strs(&tr("T-002", "in_progress", &["--at", &ts(hour)])),
        "transition in an hour",
    );
    l.refused(
        &strs(&add_args(
            "T-002",
            "implementer",
            Some(5),
            &["--ended-at", &ts(hour)],
        )),
        "add ending in an hour",
    );
    l.refused(
        &strs(&add_args(
            "T-002",
            "implementer",
            Some(5),
            &["--started-at", &ts(hour)],
        )),
        "add starting in an hour (default end)",
    );
    // started_at later than ended_at.
    l.refused(
        &strs(&add_args(
            "T-002",
            "implementer",
            Some(5),
            &["--started-at", &ts(l2 + 150), "--ended-at", &ts(l2 + 140)],
        )),
        "add starting after it ends",
    );

    // Another task's later records do not count.
    l.appended(
        &strs(&add_args(
            "T-004",
            "implementer",
            Some(5),
            &[
                "--started-at",
                &ts(now - 2000),
                "--ended-at",
                &ts(now - 1000),
            ],
        )),
        "add for a task without own records",
    );
    l.appended(
        &strs(&tr("T-003", "ready", &["--at", &ts(now - 60)])),
        "T-003 equal to its own latest",
    );
}

// ---- AC-2 transitions ------------------------------------------------------------------------

const PATH: &[&str] = &[
    "draft",
    "specified",
    "ready",
    "in_progress",
    "self_verified",
    "tests_joined",
    "reviewed",
    "queued",
    "merged",
    "evaluated",
];

#[test]
fn a_medium_task_walks_the_whole_path_with_default_times() {
    let l = Lite::new("ac2-path");
    l.task(&TaskSpec::new("T-010", "medium", 1));
    for w in PATH.windows(2) {
        let before = now_s();
        // `--from`, when given, must equal the current state: give it on one step.
        let extra: Vec<&str> = if w[1] == "self_verified" {
            vec!["--from", w[0]]
        } else {
            vec![]
        };
        let (rec, _) = l.appended(
            &strs(&tr("T-010", w[1], &extra)),
            &format!("{} -> {}", w[0], w[1]),
        );
        let after = now_s();
        assert_eq!(rec["task"], "T-010");
        assert_eq!(
            (rec["from"].as_str(), rec["to"].as_str()),
            (Some(w[0]), Some(w[1]))
        );
        assert_in_bracket(rec["at"].as_str().unwrap(), before, after, w[1]);
    }
    assert_eq!(l.records().len(), PATH.len() - 1);
}

#[test]
fn transitions_outside_the_table_are_refused_and_change_nothing() {
    let l = Lite::new("ac2-refused");
    let t0 = now_s() - 3600;
    for (id, tier) in [
        ("T-011", "medium"),
        ("T-012", "medium"),
        ("T-013", "low"),
        ("T-014", "medium"),
        ("T-015", "medium"),
        ("T-016", "medium"),
        ("T-017", "high"),
    ] {
        l.task(&TaskSpec::new(id, tier, 1));
    }
    l.seed(&[
        transition_rec("T-011", "ready", "in_progress", &ts(t0)),
        transition_rec("T-012", "in_progress", "self_verified", &ts(t0)),
        transition_rec("T-013", "in_progress", "self_verified", &ts(t0)),
        transition_rec("T-014", "merged", "evaluated", &ts(t0)),
        transition_rec("T-015", "ready", "in_progress", &ts(t0)),
        transition_rec("T-015", "in_progress", "blocked", &ts(t0 + 1)),
        transition_rec("T-016", "ready", "in_progress", &ts(t0)),
        transition_rec("T-017", "in_progress", "self_verified", &ts(t0)),
    ]);
    // in_progress -> tests_joined: the spec's own example line.
    let line = l.refused(
        &strs(&tr("T-011", "tests_joined", &[])),
        "in_progress -> tests_joined",
    );
    assert_eq!(
        line,
        format!(
            "metrics: refused: T-011 is in_progress (since {}); in_progress -> tests_joined is not allowed (METHODOLOGY §3)",
            ts(t0)
        )
    );
    // self_verified -> queued: refused on medium and high, accepted on low.
    l.refused(
        &strs(&tr("T-012", "queued", &[])),
        "medium self_verified -> queued",
    );
    l.refused(
        &strs(&tr("T-017", "queued", &[])),
        "high self_verified -> queued",
    );
    l.appended(
        &strs(&tr("T-013", "queued", &[])),
        "low self_verified -> queued",
    );
    // `--from` that is not the current state.
    l.refused(
        &strs(&tr("T-016", "self_verified", &["--from", "ready"])),
        "--from ready while in_progress",
    );
    l.appended(
        &strs(&tr("T-016", "self_verified", &["--from", "in_progress"])),
        "--from matching",
    );
    // Nothing leaves `evaluated`.
    for to in [
        "draft",
        "specified",
        "ready",
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
        l.refused(&strs(&tr("T-014", to, &[])), &format!("evaluated -> {to}"));
    }
    // blocked (entered from in_progress) -> ready refused; -> in_progress accepted.
    l.refused(&strs(&tr("T-015", "ready", &[])), "blocked -> ready");
    let (rec, _) = l.appended(
        &strs(&tr("T-015", "in_progress", &[])),
        "blocked -> in_progress",
    );
    assert_eq!(rec["from"], "blocked");
    // iter-N has no states.
    l.refused(
        &strs(&tr("iter-1", "specified", &[])),
        "transition on iter-1",
    );
    // A task without a task file, or with two.
    l.refused(&strs(&tr("T-099", "specified", &[])), "no task file");
    write(
        &l.root,
        "tasks/T-018-a.md",
        &TaskSpec::new("T-018", "low", 1).md(),
    );
    write(
        &l.root,
        "tasks/T-018-b.md",
        &TaskSpec::new("T-018", "low", 1).md(),
    );
    l.refused(&strs(&tr("T-018", "specified", &[])), "two task files");
}

#[cfg(test)]
/// The METHODOLOGY §3 table of Interface (1), as a predicate. `entered` is the state a
/// `blocked` task was blocked from.
fn allowed(tier: &str, from: &str, entered: &str, to: &str) -> bool {
    let mh = tier != "low";
    let base: &[&str] = match from {
        "draft" => &["specified"],
        "specified" => &["ready", "draft"],
        "ready" => &["in_progress"],
        "in_progress" => &["self_verified"],
        "self_verified" if mh => &["tests_joined", "in_progress"],
        "self_verified" => &["queued"],
        "tests_joined" if mh => &["reviewed", "in_progress"],
        "reviewed" if mh => &["queued"],
        "queued" => &["merged", "in_progress"],
        "merged" => &["evaluated"],
        _ => &[],
    };
    if base.contains(&to) {
        return true;
    }
    let listed = [
        "draft",
        "specified",
        "ready",
        "in_progress",
        "self_verified",
        "tests_joined",
        "reviewed",
        "queued",
    ];
    if listed.contains(&from) && (to == "blocked" || to == "abandoned") {
        return true;
    }
    from == "blocked" && (to == entered || to == "abandoned")
}

const STATES: &[&str] = &[
    "draft",
    "specified",
    "ready",
    "in_progress",
    "self_verified",
    "tests_joined",
    "reviewed",
    "queued",
    "merged",
    "evaluated",
    "blocked",
    "abandoned",
];

#[test]
fn every_state_pair_follows_the_table() {
    let l = Lite::new("ac2-table");
    let t0 = now_s() - 7200;
    let mut n = 100;
    let mut cases = Vec::new();
    for tier in ["low", "medium", "high"] {
        // (current state, state blocked from)
        let mut froms: Vec<(&str, &str)> = STATES
            .iter()
            .filter(|s| **s != "blocked")
            .map(|s| (*s, ""))
            .collect();
        for e in [
            "draft",
            "specified",
            "ready",
            "in_progress",
            "self_verified",
            "tests_joined",
            "reviewed",
            "queued",
        ] {
            froms.push(("blocked", e));
        }
        for (from, entered) in froms {
            for to in STATES {
                n += 1;
                let id = format!("T-{n:03}");
                l.task(&TaskSpec::new(&id, tier, 1));
                let mut seed = Vec::new();
                if from == "blocked" {
                    seed.push(transition_rec(&id, "draft", entered, &ts(t0)));
                    seed.push(transition_rec(&id, entered, "blocked", &ts(t0 + 1)));
                } else if from != "draft" {
                    seed.push(transition_rec(&id, "draft", from, &ts(t0)));
                }
                l.seed(&seed);
                cases.push((id, tier, from, entered, *to));
            }
        }
    }
    let mut wrong = Vec::new();
    for (id, tier, from, entered, to) in &cases {
        let before = l.bytes();
        let o = l.x(&strs(&tr(id, to, &[])));
        let after = l.bytes();
        let want = allowed(tier, from, entered, to);
        let ok = if want {
            o.code == 0 && after.len() > before.len() && after.starts_with(&before) && {
                let rec: Value = serde_json::from_slice(
                    after[before.len()..]
                        .strip_suffix(b"\n")
                        .unwrap_or(&after[before.len()..]),
                )
                .unwrap_or(Value::Null);
                rec["from"] == *from && rec["to"] == *to
            }
        } else {
            o.code == 1
                && after == before
                && o.stderr.trim_start().starts_with("metrics: refused: ")
        };
        if !ok {
            wrong.push(format!(
                "{tier} {from}{} -> {to}: want {} got exit {} {}",
                if entered.is_empty() {
                    String::new()
                } else {
                    format!(" (from {entered})")
                },
                if want { "accepted" } else { "refused" },
                o.code,
                o.stderr.trim()
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {} pairs wrong:\n{}",
        wrong.len(),
        cases.len(),
        wrong.join("\n")
    );
}

// ---- AC-3 tokens -----------------------------------------------------------------------------

#[test]
fn tokens_must_be_positive_or_declared_unknown() {
    let l = Lite::new("ac3-tokens");
    l.task(&TaskSpec::new("T-020", "medium", 1));
    l.refused(
        &strs(&add_args("T-020", "implementer", Some(0), &[])),
        "--tokens 0",
    );
    let mut both = add_args("T-020", "implementer", Some(5), &[]);
    both.push("--tokens-unknown".into());
    l.refused(&strs(&both), "--tokens and --tokens-unknown");
    let neither: Vec<String> = add_args("T-020", "implementer", None, &[])
        .into_iter()
        .filter(|a| a != "--tokens-unknown")
        .collect();
    l.refused(&strs(&neither), "neither option");
    let (rec, _) = l.appended(
        &strs(&add_args("T-020", "implementer", None, &[])),
        "--tokens-unknown",
    );
    assert!(
        rec.get("tokens").is_some_and(Value::is_null),
        "record: {rec}"
    );
    let (rec, _) = l.appended(
        &strs(&add_args("T-020", "implementer", Some(1), &[])),
        "--tokens 1",
    );
    assert_eq!(rec["tokens"], 1);
    // The raw line of an unknown run holds `"tokens":null`.
    let text = String::from_utf8(l.bytes()).unwrap();
    assert!(
        text.lines().next().unwrap().contains("\"tokens\":null"),
        "{text}"
    );
    // Usage errors stay exit 2, before the tokens rule.
    let mut bad = add_args("T-020", "implementer", Some(0), &[]);
    bad.extend(["--no-such-option".to_owned(), "x".to_owned()]);
    let before = l.bytes();
    let o = l.x(&strs(&bad));
    assert_eq!(o.code, 2, "unknown option: {}{}", o.stdout, o.stderr);
    assert_eq!(l.bytes(), before);
}

// ---- AC-4 budget line ------------------------------------------------------------------------

#[test]
fn add_prints_the_tasks_tokens_against_its_budget() {
    let l = Lite::new("ac4-budget");
    l.task(&TaskSpec {
        budget: Some(500_000),
        ..TaskSpec::new("T-020", "medium", 1)
    });
    let (_, o) = l.appended(
        &strs(&add_args("T-020", "implementer", Some(300_000), &[])),
        "first run",
    );
    assert_eq!(
        last_lines(&o.stdout, 1),
        ["T-020: 300000 tokens of budget 500000"],
        "{}",
        o.stdout
    );
    assert!(!o.stdout.contains("over its ceiling"), "{}", o.stdout);

    let over = "T-020: over its ceiling 500000 by 50000 tokens; the queue refuses it without a docs/decisions.md row \"Extend T-020 to <N>\" with N ≥ 550000";
    let (_, o) = l.appended(
        &strs(&add_args("T-020", "test-author", Some(250_000), &[])),
        "second run",
    );
    assert_eq!(
        last_lines(&o.stdout, 2),
        ["T-020: 550000 tokens of budget 500000", over],
        "stdout:\n{}",
        o.stdout
    );
    let (_, o) = l.appended(
        &strs(&add_args("T-020", "reviewer", None, &[])),
        "unknown run",
    );
    assert_eq!(
        last_lines(&o.stdout, 3),
        [
            "T-020: 550000 tokens of budget 500000",
            over,
            "T-020: 1 run(s) with unknown tokens not counted"
        ],
        "stdout:\n{}",
        o.stdout
    );

    // The ceiling is the task's last `Extend` row in the working tree's docs/decisions.md.
    write(
        &l.root,
        "docs/decisions.md",
        &format!(
            "{DECISIONS}| D-002 | 2026-11-13 | Extend T-020 to 600000 for the rework. | x | yes |\n| D-003 | 2026-11-13 | Extend T-020 to 520000 after all. | x | yes |\n"
        ),
    );
    let (_, o) = l.appended(
        &strs(&add_args("T-020", "reviewer", Some(1), &[])),
        "run with an Extend row",
    );
    assert_eq!(
        last_lines(&o.stdout, 3),
        [
            "T-020: 550001 tokens of budget 500000",
            "T-020: over its ceiling 520000 by 30001 tokens; the queue refuses it without a docs/decisions.md row \"Extend T-020 to <N>\" with N ≥ 550001",
            "T-020: 1 run(s) with unknown tokens not counted"
        ],
        "stdout:\n{}",
        o.stdout
    );
    write(
        &l.root,
        "docs/decisions.md",
        &format!(
            "{DECISIONS}| D-002 | 2026-11-13 | Extend T-020 to 600000 for the rework. | x | yes |\n"
        ),
    );
    let (_, o) = l.appended(
        &strs(&add_args("T-020", "reviewer", Some(1), &[])),
        "run within the extension",
    );
    assert_eq!(
        last_lines(&o.stdout, 2),
        [
            "T-020: 550002 tokens of budget 500000",
            "T-020: 1 run(s) with unknown tokens not counted"
        ],
        "stdout:\n{}",
        o.stdout
    );
}

// ---- AC-5 iter-N -----------------------------------------------------------------------------

#[test]
fn overhead_runs_are_booked_to_iter_n() {
    let l = Lite::new("ac5-iter");
    l.task(&TaskSpec::new("T-001", "medium", 1));
    let (rec, o) = l.appended(
        &strs(&add_args("iter-1", "evaluator", Some(1000), &[])),
        "evaluator on iter-1",
    );
    assert_eq!(rec["task"], "iter-1");
    assert_eq!(rec["iteration"], 1);
    assert!(rec.get("tier").is_some_and(Value::is_null), "record: {rec}");
    assert_eq!(
        last_lines(&o.stdout, 1),
        ["iter-1: 1000 tokens of overhead"],
        "{}",
        o.stdout
    );
    for role in ["director", "planner", "retrospective-analyst"] {
        l.appended(&strs(&add_args("iter-1", role, Some(10), &[])), role);
    }
    let (_, o) = l.appended(
        &strs(&add_args(
            "iter-1",
            "planner",
            Some(1),
            &["--iteration", "1"],
        )),
        "--iteration 1",
    );
    assert_eq!(
        last_lines(&o.stdout, 1),
        ["iter-1: 1031 tokens of overhead"],
        "{}",
        o.stdout
    );
    // Another iteration is its own pseudo-task.
    let (rec, o) = l.appended(
        &strs(&add_args("iter-12", "director", Some(7), &[])),
        "iter-12",
    );
    assert_eq!(rec["iteration"], 12);
    assert_eq!(
        last_lines(&o.stdout, 1),
        ["iter-12: 7 tokens of overhead"],
        "{}",
        o.stdout
    );

    for role in ["implementer", "test-author", "reviewer", "integrator"] {
        l.refused(
            &strs(&add_args("iter-1", role, Some(10), &[])),
            &format!("{role} on iter-1"),
        );
    }
    for role in ["planner", "director", "evaluator", "retrospective-analyst"] {
        l.refused(
            &strs(&add_args("T-001", role, Some(10), &[])),
            &format!("{role} on T-001"),
        );
    }
    l.refused(
        &strs(&add_args(
            "iter-1",
            "planner",
            Some(10),
            &["--iteration", "2"],
        )),
        "--iteration 2 on iter-1",
    );
    l.refused(
        &strs(&add_args("iter-01", "planner", Some(10), &[])),
        "iter-01 (leading zero)",
    );
    // Ruling: `--tier` on an iter-N record is refused.
    l.refused(
        &strs(&add_args("iter-1", "planner", Some(10), &["--tier", "low"])),
        "--tier on iter-1",
    );
}

#[test]
fn a_task_run_needs_exactly_one_task_file() {
    let l = Lite::new("ac5-taskfile");
    l.refused(
        &strs(&add_args(
            "T-030",
            "implementer",
            Some(10),
            &["--tier", "low", "--iteration", "1"],
        )),
        "no task file",
    );
    write(
        &l.root,
        "tasks/T-031-a.md",
        &TaskSpec::new("T-031", "low", 1).md(),
    );
    write(
        &l.root,
        "tasks/T-031-b.md",
        &TaskSpec::new("T-031", "low", 1).md(),
    );
    l.refused(
        &strs(&add_args("T-031", "implementer", Some(10), &[])),
        "two task files",
    );
    l.task(&TaskSpec::new("T-032", "high", 1));
    let (rec, _) = l.appended(
        &strs(&add_args("T-032", "implementer", Some(10), &[])),
        "one task file",
    );
    assert_eq!(
        (rec["tier"].as_str(), rec["iteration"].as_u64()),
        (Some("high"), Some(1))
    );
    // Ruling: a task file without budget_tokens prints `budget none` and no over line.
    l.task(&TaskSpec {
        budget: None,
        ..TaskSpec::new("T-033", "medium", 1)
    });
    let (_, o) = l.appended(
        &strs(&add_args("T-033", "implementer", Some(700), &[])),
        "task without budget_tokens",
    );
    assert_eq!(
        last_lines(&o.stdout, 1),
        ["T-033: 700 tokens of budget none"],
        "{}",
        o.stdout
    );
    assert!(!o.stdout.contains("over its ceiling"), "{}", o.stdout);
}
