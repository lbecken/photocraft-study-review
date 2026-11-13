//! T-013 acceptance tests, Interface (5): `cargo xtask publish` (AC-9 success, AC-10 refusals and
//! stops).
//!
//! Written from the task file alone. The toy repository is T-005's fixture with a toy `xtask`
//! whose `scorecard` summarises `scorecard/items/*.toml` into `docs/scorecard.md` (and fails on
//! an item containing `malformed`), whose `scorecard --check` compares, and whose `ci` runs that
//! check and fails at step `red` when an item contains `red-gate`. Evaluations do not change the
//! toy scorecard. Everything runs through the `xtask` binary with a timeout.

use std::path::Path;
use std::process::Command;

#[path = "acceptance_t013_support.rs"]
mod t13;
use t13::*;

const PUBLISH_TOY_MAIN: &str = r##"use std::process::ExitCode;

fn items() -> Result<Vec<(String, String)>, String> {
    let mut v = Vec::new();
    if let Ok(rd) = std::fs::read_dir("scorecard/items") {
        for e in rd.filter_map(|e| e.ok()) {
            let n = e.file_name().to_string_lossy().into_owned();
            let body = std::fs::read_to_string(e.path()).unwrap_or_default();
            if body.contains("malformed") {
                return Err(format!("toy scorecard: scorecard/items/{n} is malformed"));
            }
            v.push((n, body));
        }
    }
    v.sort();
    Ok(v)
}

fn summary() -> Result<String, String> {
    let mut out = String::from("# Scorecard (publish toy)\n");
    for (n, b) in items()? {
        out.push_str(&format!("- {n}: {}\n", b.trim()));
    }
    Ok(out)
}

/// Items marked `edit-primary` are changed in the primary checkout while the gate runs (a
/// stand-in for an input edited between publish steps 4 and 7).
fn edit_primary() {
    let Ok(v) = items() else { return };
    let marked: Vec<String> = v.into_iter().filter(|(_, b)| b.contains("edit-primary")).map(|(n, _)| n).collect();
    if marked.is_empty() {
        return;
    }
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .output()
        .expect("git rev-parse");
    let common = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    let primary = std::path::Path::new(&common).parent().expect("primary checkout").to_path_buf();
    for n in marked {
        let p = primary.join("scorecard/items").join(n);
        let mut body = std::fs::read_to_string(&p).unwrap_or_default();
        body.push_str("edited = true\n");
        std::fs::write(&p, body).expect("edit the primary checkout's input");
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("ci") => {
            edit_primary();
            let report = args.iter().position(|a| a == "--report").and_then(|i| args.get(i + 1));
            let fresh = summary().ok() == std::fs::read_to_string("docs/scorecard.md").ok();
            let red = items().map(|v| v.iter().any(|(_, b)| b.contains("red-gate"))).unwrap_or(true);
            let failed = if !fresh { Some("scorecard") } else if red { Some("red") } else { None };
            let json = match failed {
                Some(s) => format!("{{\"passed\": false, \"failed_step\": \"{s}\", \"seconds\": 0.0, \"steps\": []}}"),
                None => "{\"passed\": true, \"failed_step\": null, \"seconds\": 0.0, \"steps\": []}".to_owned(),
            };
            if let Some(r) = report {
                std::fs::write(r, json).expect("write report");
            }
            println!("toy gate: {}", failed.map_or("PASSED".to_owned(), |s| format!("FAILED at {s}")));
            if failed.is_some() { ExitCode::FAILURE } else { ExitCode::SUCCESS }
        }
        Some("scorecard") => match summary() {
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
            Ok(s) => {
                if args.iter().any(|a| a == "--check") {
                    if std::fs::read_to_string("docs/scorecard.md").ok().as_deref() == Some(s.as_str()) {
                        ExitCode::SUCCESS
                    } else {
                        eprintln!("toy scorecard: docs/scorecard.md is stale");
                        ExitCode::FAILURE
                    }
                } else {
                    std::fs::create_dir_all("docs").expect("docs");
                    std::fs::write("docs/scorecard.md", s).expect("write scorecard");
                    ExitCode::SUCCESS
                }
            }
        },
        Some("schema") => {
            std::fs::create_dir_all("docs").expect("docs");
            std::fs::write("docs/commands.json", "{\"commands\": []}\n").expect("write schema");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("toy xtask: unknown command {args:?}");
            ExitCode::from(2)
        }
    }
}
"##;

const EXISTING_ITEM: &str = "id = \"existing\"\n";
const NEW_ITEM: &str = "id = \"x.y\"\nstatus = \"claimed\"\n";
const EVAL_V1: &str = "# Evaluation iteration 0\n\nnothing yet\n";
const EVAL_V2: &str = "# Evaluation iteration 0\n\nx.y confirmed\n";

#[cfg(test)]
fn summary_of(items: &[(&str, &str)]) -> String {
    let mut v = items.to_vec();
    v.sort();
    let mut out = String::from("# Scorecard (publish toy)\n");
    for (n, b) in v {
        out.push_str(&format!("- {n}: {}\n", b.trim()));
    }
    out
}

#[cfg(test)]
/// A toy repository on main with the publish toy xtask, a consistent scorecard and an
/// evaluation file.
fn publish_repo(name: &str) -> Repo {
    let r = Repo::new(name);
    r.main_commit(
        "toy: publish xtask, scorecard, evaluation",
        &[
            ("xtask/src/main.rs", Some(PUBLISH_TOY_MAIN)),
            (
                "docs/scorecard.md",
                Some(&summary_of(&[("existing.toml", EXISTING_ITEM)])),
            ),
            ("evaluations/iter-0.md", Some(EVAL_V1)),
        ],
    );
    r
}

const SLUG_FILE: &str = "tasks/T-050-publish-publish-x.md";

#[cfg(test)]
fn publish(r: &Repo, inputs: &[&str]) -> Out {
    let mut a = vec![
        "publish",
        "T-050",
        "--title",
        "Publish x",
        "--iteration",
        "1",
    ];
    a.extend_from_slice(inputs);
    xt(&r.root, &a)
}

#[cfg(test)]
/// Everything a refusal must leave untouched: refs, worktrees, the primary checkout's status,
/// the bytes of the inputs and of the metrics file.
fn snap(r: &Repo, inputs: &[&str]) -> Vec<String> {
    let mut v = vec![
        git(
            &r.root,
            &["for-each-ref", "--format=%(refname) %(objectname)"],
        ),
        git(&r.root, &["worktree", "list", "--porcelain"]),
        status(&r.root),
        format!(
            "{:?}",
            std::fs::read(r.root.join("metrics/tasks.jsonl")).ok()
        ),
    ];
    for i in inputs {
        v.push(format!("{i}: {:?}", std::fs::read(r.root.join(i)).ok()));
    }
    v
}

#[cfg(test)]
fn assert_publish_refused(r: &Repo, inputs: &[&str], what: &str) -> String {
    let before = snap(r, inputs);
    let o = publish(r, inputs);
    let line = assert_refusal(&o, "publish: refused: ", what);
    assert_eq!(
        snap(r, inputs),
        before,
        "{what}: the refusal changed something"
    );
    line
}

#[cfg(test)]
/// Mini front-matter reader: `(key, value)` in order; a block list becomes `[a, b]`.
fn front_matter(text: &str) -> Vec<(String, String)> {
    let body = text.strip_prefix("---\n").expect("front matter");
    let fm = &body[..body.find("\n---\n").expect("end of front matter")];
    let mut out: Vec<(String, String)> = Vec::new();
    for line in fm.lines() {
        if let Some(item) = line.trim_start().strip_prefix("- ") {
            let last = out.last_mut().expect("list item without key");
            let inner = last
                .1
                .trim_start_matches('[')
                .trim_end_matches(']')
                .to_owned();
            last.1 = if inner.is_empty() {
                format!("[{}]", item.trim())
            } else {
                format!("[{inner}, {}]", item.trim())
            };
        } else {
            let (k, v) = line.split_once(':').expect("key: value");
            out.push((k.trim().to_owned(), v.trim().to_owned()));
        }
    }
    out
}

#[cfg(test)]
fn first_parent_subjects(r: &Repo, since: &str) -> Vec<(String, String)> {
    git(
        &r.root,
        &[
            "rev-list",
            "--first-parent",
            "--reverse",
            &format!("{since}..main"),
        ],
    )
    .lines()
    .map(|c| (c.to_owned(), git(&r.root, &["log", "-1", "--format=%s", c])))
    .collect()
}

#[cfg(test)]
fn transitions(r: &Repo, task: &str) -> Vec<serde_json::Value> {
    read(&r.root, "metrics/tasks.jsonl")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
        .filter(|v| v["kind"] == "transition" && v["task"] == task)
        .collect()
}

#[test]
fn publish_carries_a_ledger_input_to_main_in_one_command() {
    let r = publish_repo("t013-ac9-publish");
    write(&r.root, "scorecard/items/x.y.toml", NEW_ITEM);
    write(&r.root, "evaluations/iter-0.md", EVAL_V2);
    let start = r.main_sha();
    let inputs = ["scorecard/items/x.y.toml", "evaluations/iter-0.md"];

    let before = now_s();
    let o = publish(&r, &inputs);
    let after = now_s();
    assert_ok(&o, "publish");
    for i in inputs {
        assert!(
            o.stdout
                .contains(&format!("publish: {i} moved to task/T-050")),
            "hand-over line for {i}:\n{}",
            o.stdout
        );
    }

    // main's first-parent history: exactly these six commits, in this order (D-040).
    let log = first_parent_subjects(&r, &start);
    let subjects: Vec<&str> = log.iter().map(|(_, s)| s.as_str()).collect();
    assert_eq!(
        subjects.len(),
        6,
        "first-parent log since the start: {subjects:#?}"
    );
    assert_eq!(subjects[0], "publish: T-050 ready");
    assert_eq!(subjects[1], "publish: T-050 in progress");
    assert_eq!(subjects[2], "publish: T-050 queued");
    assert!(
        subjects[3].starts_with("T-050: publish ")
            && subjects[3].ends_with("; regenerate docs/scorecard.md")
            && inputs.iter().all(|i| subjects[3].contains(i)),
        "content commit subject: {}",
        subjects[3]
    );
    assert_eq!(subjects[4], "queue: record T-050 merged");
    assert_eq!(subjects[5], "publish: T-050 merged");
    // main is append-only (D-040): the pre-publish tip and every value main took during the
    // call (its reflog) are ancestors of the final head, so no commit was reset or folded.
    let head = r.main_sha();
    let mut tips = vec![start.clone()];
    tips.extend(
        git(&r.root, &["reflog", "--format=%H", "main"])
            .lines()
            .map(str::to_owned),
    );
    for t in &tips {
        let o = run_to(
            Command::new("git")
                .args(["merge-base", "--is-ancestor", t, &head])
                .current_dir(&r.root),
            60,
        );
        assert_eq!(
            o.code, 0,
            "main was at {t}, which is not an ancestor of {head}"
        );
    }
    for (c, s) in &log {
        let body = git(&r.root, &["log", "-1", "--format=%B", c]);
        let lines: Vec<&str> = body.lines().map(str::trim).collect();
        assert!(
            lines.contains(&"Task: T-050") && lines.contains(&"Role: director"),
            "commit `{s}` lacks Task/Role lines:\n{body}"
        );
    }
    // The ledger commits touch exactly the task file and the metrics file.
    for i in [0, 2, 5] {
        let mut files: Vec<String> = git(&r.root, &["show", "--name-only", "--format=", &log[i].0])
            .lines()
            .map(str::to_owned)
            .collect();
        files.sort();
        assert_eq!(
            files,
            ["metrics/tasks.jsonl", SLUG_FILE],
            "files of `{}`",
            log[i].1
        );
    }
    // The `in progress` commit records the transition and touches only ledger files of T-050.
    let files = git(&r.root, &["show", "--name-only", "--format=", &log[1].0]);
    assert!(
        files.lines().any(|f| f == "metrics/tasks.jsonl")
            && files
                .lines()
                .all(|f| f == "metrics/tasks.jsonl" || f == SLUG_FILE),
        "files of `publish: T-050 in progress`: {files}"
    );
    // The content commit: the inputs and docs/scorecard.md only, inputs byte-identical.
    let mut files: Vec<String> = git(&r.root, &["show", "--name-only", "--format=", &log[3].0])
        .lines()
        .map(str::to_owned)
        .collect();
    files.sort();
    assert_eq!(
        files,
        [
            "docs/scorecard.md",
            "evaluations/iter-0.md",
            "scorecard/items/x.y.toml"
        ]
    );
    assert_eq!(git_show(&r.root, "main:scorecard/items/x.y.toml"), NEW_ITEM);
    assert_eq!(git_show(&r.root, "main:evaluations/iter-0.md"), EVAL_V2);
    assert_eq!(
        git_show(&r.root, "main:docs/scorecard.md"),
        summary_of(&[("existing.toml", EXISTING_ITEM), ("x.y.toml", NEW_ITEM)])
    );

    // Transitions draft → … → merged, in non-decreasing time within the call.
    let tr = transitions(&r, "T-050");
    let pairs: Vec<(String, String)> = tr
        .iter()
        .map(|t| {
            (
                t["from"].as_str().unwrap().to_owned(),
                t["to"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    let want = [
        "draft",
        "specified",
        "ready",
        "in_progress",
        "self_verified",
        "queued",
        "merged",
    ];
    let want_pairs: Vec<(String, String)> = want
        .windows(2)
        .map(|w| (w[0].to_owned(), w[1].to_owned()))
        .collect();
    assert_eq!(pairs, want_pairs);
    let mut last = before;
    for t in &tr {
        let at = t["at"].as_str().unwrap();
        assert_in_bracket(at, before, after, "transition");
        assert!(parse_ts(at) >= last, "transition times decrease: {tr:?}");
        last = parse_ts(at);
    }
    // The records are committed on main.
    assert_eq!(
        git_show(&r.root, "main:metrics/tasks.jsonl"),
        read(&r.root, "metrics/tasks.jsonl")
    );

    // The task file: exactly Interface (5) step 2, state merged.
    let listed = git(&r.root, &["ls-tree", "--name-only", "main", "tasks/"]);
    assert_eq!(listed, SLUG_FILE);
    let text = git_show(&r.root, &format!("main:{SLUG_FILE}"));
    let fm = front_matter(&text);
    let expect: Vec<(String, String)> = [
        ("id", "T-050"),
        ("title", "\"Publish x\""),
        ("tier", "low"),
        ("state", "merged"),
        ("owner_role", "director"),
        ("scorecard_items", "[]"),
        ("depends_on", "[]"),
        (
            "touches",
            "[evaluations/iter-0.md, scorecard/items/x.y.toml]",
        ),
        ("budget_tokens", "150000"),
        ("iteration", "1"),
        ("found_by", "planner"),
        ("origin_task", "null"),
    ]
    .iter()
    .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
    .collect();
    assert_eq!(fm, expect, "front matter of {SLUG_FILE}:\n{text}");
    let log_line = text
        .lines()
        .find(|l| l.ends_with(" · created by cargo xtask publish"))
        .unwrap_or_else(|| panic!("no director log line:\n{text}"));
    let t = log_line
        .strip_prefix("- ")
        .and_then(|l| l.strip_suffix(" · created by cargo xtask publish"))
        .unwrap_or_else(|| panic!("log line form: {log_line}"));
    assert_in_bracket(t, before, after, "director log line");

    // Worktree removed; the primary checkout is main, clean; the scorecard check passes.
    assert!(!r.wt("T-050").exists(), ".worktrees/T-050 still exists");
    assert_eq!(git(&r.root, &["rev-parse", "--abbrev-ref", "HEAD"]), "main");
    assert_eq!(status(&r.root), "", "primary checkout not clean");
    let check = run_to(
        hygiene(&mut Command::new("cargo"))
            .args(["xtask", "scorecard", "--check"])
            .current_dir(&r.root),
        XTASK_TIMEOUT_S,
    );
    assert_ok(&check, "cargo xtask scorecard --check on main");
}

#[test]
fn publish_refusals_change_nothing() {
    let r = publish_repo("t013-ac10-refusals");
    r.main_commit(
        "director: T-049",
        &[(
            "tasks/T-049-toy.md",
            Some(&task_md("T-049", "low", "merged", &["crates/a"], &[])),
        )],
    );
    let item = "scorecard/items/x.y.toml";
    let eval = "evaluations/iter-0.md";
    // Each case starts from a clean primary checkout and dirties only what it tests, so the
    // working-tree rule of Interface (5) step 1 never decides a case by accident.
    let clean = || {
        git(&r.root, &["checkout", "-q", "--", "."]);
        git(&r.root, &["clean", "-fdq"]);
        assert_eq!(status(&r.root), "", "clean primary checkout");
    };
    let both = || {
        write(&r.root, item, NEW_ITEM);
        write(&r.root, eval, EVAL_V2);
    };

    // An input that changes no generated file (the toy scorecard ignores evaluations).
    clean();
    write(&r.root, eval, EVAL_V2);
    let line = assert_publish_refused(&r, &[eval], "evaluation only");
    assert_eq!(
        line,
        "publish: refused: the inputs change no generated file; commit them on main as a ledger commit"
    );
    // An input under docs/.
    clean();
    write(&r.root, item, NEW_ITEM);
    write(&r.root, "docs/notes.md", "changed notes\n");
    let line = assert_publish_refused(&r, &[item, "docs/notes.md"], "input under docs/");
    assert!(line.contains("docs/notes.md"), "{line}");
    // An existing scorecard item is not new.
    clean();
    write(
        &r.root,
        "scorecard/items/existing.toml",
        "id = \"existing\"\nstatus = \"x\"\n",
    );
    let line = assert_publish_refused(&r, &["scorecard/items/existing.toml"], "modified item");
    assert!(line.contains("scorecard/items/existing.toml"), "{line}");
    // An input equal to main's version.
    clean();
    write(&r.root, item, NEW_ITEM);
    let line = assert_publish_refused(&r, &[item, eval], "unchanged input");
    assert!(line.contains(eval), "{line}");
    // No input.
    clean();
    assert_publish_refused(&r, &[], "no input");
    // A malformed id.
    clean();
    write(&r.root, item, NEW_ITEM);
    let before = snap(&r, &[item]);
    let o = xt(
        &r.root,
        &[
            "publish",
            "T-50",
            "--title",
            "Publish x",
            "--iteration",
            "1",
            item,
        ],
    );
    assert_refusal(&o, "publish: refused: ", "malformed id");
    assert_eq!(snap(&r, &[item]), before);
    // Another uncommitted change in tasks/.
    clean();
    both();
    write(
        &r.root,
        "tasks/T-049-toy.md",
        &task_md("T-049", "low", "evaluated", &["crates/a"], &[]),
    );
    let line = assert_publish_refused(&r, &[item, eval], "uncommitted change in tasks/");
    assert!(line.contains("tasks/T-049-toy.md"), "{line}");
    // A malformed item: the scorecard's own error.
    clean();
    write(&r.root, "scorecard/items/bad.toml", "malformed = [\n");
    let line = assert_publish_refused(&r, &["scorecard/items/bad.toml"], "malformed item");
    assert!(
        line.contains("toy scorecard: scorecard/items/bad.toml is malformed"),
        "the scorecard's own error: {line}"
    );
    // A branch task/T-050 already exists.
    clean();
    both();
    git(&r.root, &["branch", "task/T-050", "main"]);
    assert_publish_refused(&r, &[item, eval], "existing branch");
    git(&r.root, &["branch", "-D", "task/T-050"]);
    // An existing tasks/T-050-*.md on main (last: it moves main).
    clean();
    r.main_commit(
        "director: T-050 by hand",
        &[(
            "tasks/T-050-old.md",
            Some(&task_md("T-050", "low", "draft", &["crates/a"], &[])),
        )],
    );
    both();
    let line = assert_publish_refused(&r, &[item, eval], "existing task file");
    assert!(line.contains("T-050"), "{line}");
}

#[test]
fn publish_stops_when_the_gate_fails_and_keeps_the_inputs() {
    let r = publish_repo("t013-ac10-gate");
    let item = "scorecard/items/x.y.toml";
    let red = "id = \"x.y\"\nnote = \"red-gate\"\n";
    write(&r.root, item, red);
    let start = r.main_sha();
    let o = publish(&r, &[item]);
    assert_eq!(o.code, 1, "gate failure exit:\n{}\n{}", o.stdout, o.stderr);
    let want = "publish: gate failed in .worktrees/T-050 at red; T-050 stays in_progress; fix the input and re-run the gate there, or abandon (metrics transition --to abandoned; worktree rm T-050 --force)";
    assert!(
        format!("{}\n{}", o.stdout, o.stderr)
            .lines()
            .any(|l| l == want),
        "expected the step-5 line\nstdout:\n{}\nstderr:\n{}",
        o.stdout,
        o.stderr
    );
    let tr = transitions(&r, "T-050");
    assert_eq!(
        tr.last().map(|t| t["to"].clone()),
        Some("in_progress".into()),
        "{tr:?}"
    );
    assert_eq!(
        read(&r.root, item),
        red,
        "the primary checkout's input must stay"
    );
    assert!(r.wt("T-050").is_dir(), "the worktree stays for the fix");
    // Ruling: main's ledger stays committed where publish stops: `in_progress` in main's
    // committed metrics, the working tree clean apart from the input itself.
    let committed: Vec<serde_json::Value> = git_show(&r.root, "main:metrics/tasks.jsonl")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
        .filter(|v| v["kind"] == "transition" && v["task"] == "T-050")
        .collect();
    assert_eq!(
        committed.last().map(|t| t["to"].clone()),
        Some("in_progress".into()),
        "main's committed transitions: {committed:?}"
    );
    assert_eq!(
        status(&r.root),
        format!("?? {item}\n"),
        "primary checkout status"
    );
    // Ruling n (D-047): main's committed task file says what the last transition says.
    assert_eq!(
        committed_state(&r),
        "in_progress",
        "`state:` of main's committed {SLUG_FILE} after a red gate"
    );
    // D-040: the `ready` and `in progress` commits both remain, nothing after them.
    let subjects: Vec<String> = first_parent_subjects(&r, &start)
        .into_iter()
        .map(|(_, s)| s)
        .collect();
    assert_eq!(
        subjects,
        ["publish: T-050 ready", "publish: T-050 in progress"],
        "main's first-parent log after a red gate"
    );
    assert!(r.queue_lines().is_empty(), "the queue must not run");
}

#[test]
fn a_failed_ready_commit_stops_publish_and_prints_the_commit_command() {
    let r = publish_repo("t013-ac10-commit");
    let item = "scorecard/items/x.y.toml";
    write(&r.root, item, NEW_ITEM);
    let start = r.main_sha();
    let signer = r.root.parent().unwrap().join("no-sign.sh");
    std::fs::write(&signer, "#!/bin/sh\necho signing refused >&2\nexit 1\n").unwrap();
    let o = sh(Path::new("/"), &format!("chmod +x '{}'", signer.display()));
    assert_ok(&o, "chmod");
    git(&r.root, &["config", "commit.gpgsign", "true"]);
    git(
        &r.root,
        &["config", "gpg.program", &signer.to_string_lossy()],
    );

    let o = publish(&r, &[item]);
    assert_eq!(
        o.code, 2,
        "commit failure exit:\n{}\n{}",
        o.stdout, o.stderr
    );
    let lines: Vec<&str> = o.stderr.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 1, "one stderr line:\n{}", o.stderr);
    let tail = format!(
        "; commit by hand: git commit -m \"publish: T-050 ready\" -m \"Task: T-050\" -m \"Role: director\" -- {SLUG_FILE} metrics/tasks.jsonl"
    );
    assert!(
        lines[0].starts_with("publish: commit failed: ") && lines[0].ends_with(&tail),
        "step-2 line: {}",
        lines[0]
    );
    // Nothing after the failed commit.
    assert_eq!(r.main_sha(), start, "main moved");
    assert!(
        !r.wt("T-050").exists(),
        "a worktree was made after the failed commit"
    );
    assert!(
        git(&r.root, &["branch", "--list", "task/T-050"]).is_empty(),
        "a branch was made after the failed commit"
    );

    // The printed command, run after signing is repaired, commits exactly those two paths.
    git(&r.root, &["config", "--unset", "commit.gpgsign"]);
    let cmd = &lines[0][lines[0].find("commit by hand: ").unwrap() + "commit by hand: ".len()..];
    let o = sh(&r.root, cmd);
    assert_ok(&o, "the printed command");
    let mut files: Vec<String> = git(&r.root, &["show", "--name-only", "--format=", "HEAD"])
        .lines()
        .map(str::to_owned)
        .collect();
    files.sort();
    assert_eq!(files, ["metrics/tasks.jsonl", SLUG_FILE]);
    assert_eq!(
        git(&r.root, &["log", "-1", "--format=%s", "main"]),
        "publish: T-050 ready"
    );
    assert_eq!(git(&r.root, &["rev-parse", "main~1"]), start);
}

// ---- review round 1: findings 6–8 and rulings k–n, p (D-047) ----------------------------------

#[cfg(test)]
/// `state:` of main's committed publish task file.
fn committed_state(r: &Repo) -> String {
    let text = git_show(&r.root, &format!("main:{SLUG_FILE}"));
    front_matter(&text)
        .into_iter()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| v)
        .unwrap_or_else(|| panic!("no state in main's {SLUG_FILE}:\n{text}"))
}

#[cfg(test)]
/// Every file under `dir` with its length and modification time (empty if `dir` is absent).
fn tree_snap(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.filter_map(Result::ok) {
            let p = e.path();
            let m = std::fs::symlink_metadata(&p).unwrap();
            if m.is_dir() {
                stack.push(p.clone());
            }
            out.push(format!(
                "{} {} {:?}",
                p.display(),
                m.len(),
                m.modified().ok()
            ));
        }
    }
    out.sort();
    out
}

#[test]
fn publish_refuses_a_checkout_dirty_outside_the_inputs_and_ledger_paths() {
    // Finding 6: Interface (5) step 1, "apart from the inputs, the primary checkout passes the
    // checks the queue applies to it" (the queue's working-tree rule: no dirty path but
    // `metrics/`); D-039 b: one line naming the offending path, nothing changed.
    let r = publish_repo("t013-f6-dirty");
    let item = "scorecard/items/x.y.toml";
    let clean = || {
        git(&r.root, &["checkout", "-q", "--", "."]);
        git(&r.root, &["clean", "-fdq"]);
        assert_eq!(status(&r.root), "", "clean primary checkout");
    };
    // A modified tracked file outside every ledger path.
    clean();
    write(&r.root, item, NEW_ITEM);
    write(&r.root, "other/keep.txt", "changed by hand\n");
    assert_eq!(read(&r.root, "other/keep.txt"), "changed by hand\n");
    let line = assert_publish_refused(&r, &[item], "modified other/keep.txt");
    assert!(line.contains("other/keep.txt"), "names the path: {line}");
    assert_eq!(read(&r.root, "other/keep.txt"), "changed by hand\n");
    // An untracked file at the root.
    clean();
    write(&r.root, item, NEW_ITEM);
    write(&r.root, "stray.txt", "stray\n");
    let line = assert_publish_refused(&r, &[item], "untracked stray.txt");
    assert!(line.contains("stray.txt"), "names the path: {line}");
    assert!(r.root.join("stray.txt").is_file(), "stray.txt must stay");
    assert!(r.queue_lines().is_empty(), "the queue must not run");
}

#[test]
fn an_input_changed_during_the_gate_is_not_handed_over() {
    // Finding 7: step 7 checks each input's bytes in the primary checkout against the committed
    // branch copy before removing it. The toy gate edits the primary checkout's input (step 5).
    // Ruling n: no pinned line for a step-7 stop → `publish: stopped at step 7: <error>; T-NNN
    // is <state>`; main's ledger committed, `state:` equal to the last transition (`queued`).
    let r = publish_repo("t013-f7-edited");
    let item = "scorecard/items/x.y.toml";
    let body = "id = \"x.y\"\nnote = \"edit-primary\"\n";
    write(&r.root, item, body);
    let start = r.main_sha();
    let o = publish(&r, &[item]);
    assert!(
        o.code != 0 && o.code != 101,
        "a step-7 stop exits non-zero without a panic; got {}\n{}\n{}",
        o.code,
        o.stdout,
        o.stderr
    );
    let all = format!("{}\n{}", o.stdout, o.stderr);
    assert!(
        all.lines()
            .any(|l| l.starts_with("publish: stopped at step 7: ")
                && l.ends_with("; T-050 is queued")),
        "expected `publish: stopped at step 7: …; T-050 is queued`\nstdout:\n{}\nstderr:\n{}",
        o.stdout,
        o.stderr
    );
    assert!(
        !all.contains(&format!("publish: {item} moved to task/T-050")),
        "an edited input must not be handed over:\n{all}"
    );
    // The edited input stays in the primary checkout with its edited bytes.
    assert_eq!(
        read(&r.root, item),
        format!("{body}edited = true\n"),
        "the primary checkout's (edited) input must stay as it is"
    );
    // The branch holds the copy of step 4, not the edit.
    assert_eq!(git_show(&r.root, &format!("task/T-050:{item}")), body);
    // Main: ready, in progress, queued; no queue run, nothing merged.
    let subjects: Vec<String> = first_parent_subjects(&r, &start)
        .into_iter()
        .map(|(_, s)| s)
        .collect();
    assert_eq!(
        subjects,
        [
            "publish: T-050 ready",
            "publish: T-050 in progress",
            "publish: T-050 queued"
        ],
        "main's first-parent log after a step-7 stop"
    );
    assert!(r.queue_lines().is_empty(), "the queue must not run");
    let tr = transitions(&r, "T-050");
    assert_eq!(
        tr.last().map(|t| t["to"].clone()),
        Some("queued".into()),
        "{tr:?}"
    );
    assert_eq!(
        git_show(&r.root, "main:metrics/tasks.jsonl"),
        read(&r.root, "metrics/tasks.jsonl"),
        "every ledger change is committed on main"
    );
    assert_eq!(committed_state(&r), "queued");
    assert_eq!(
        status(&r.root),
        format!("?? {item}\n"),
        "primary checkout status"
    );
}

#[test]
fn publish_without_an_input_is_refused_for_that_reason() {
    // Finding 8: step 1 "at least one input". With no input the refusal is that check's, not the
    // later "the inputs change no generated file; commit them on main" (there is nothing to
    // commit), and nothing changes.
    let r = publish_repo("t013-f8-noinput");
    assert_eq!(status(&r.root), "");
    let line = assert_publish_refused(&r, &[], "no input");
    assert_ne!(
        line,
        "publish: refused: the inputs change no generated file; commit them on main as a ledger commit",
        "no input is refused by the at-least-one-input check"
    );
    assert!(
        git(&r.root, &["ls-tree", "--name-only", "main", "tasks/"]).is_empty(),
        "no task file on main"
    );
}

#[test]
fn publish_refuses_an_id_that_already_has_ledger_records() {
    // Ruling k: step 1 refuses a T-NNN with any record in the working tree's
    // metrics/tasks.jsonl, with the pinned line; nothing written (no task file left behind).
    let r = publish_repo("t013-k-records");
    let t = now_s() - 3600;
    r.set_tasks_jsonl(&[
        transition_rec("T-050", "draft", "specified", &ts(t)).to_string(),
        transition_rec("T-050", "specified", "abandoned", &ts(t + 1)).to_string(),
    ]);
    assert!(git_show(&r.root, "main:metrics/tasks.jsonl").contains("\"T-050\""));
    assert_eq!(status(&r.root), "");
    let item = "scorecard/items/x.y.toml";
    write(&r.root, item, NEW_ITEM);
    let line = assert_publish_refused(&r, &[item], "id with records");
    assert_eq!(
        line,
        "publish: refused: T-050 already has records in metrics/tasks.jsonl"
    );
    assert!(!r.root.join(SLUG_FILE).exists(), "no task file written");
    // A second call gives the same refusal (nothing was left behind to change the reason).
    let line = assert_publish_refused(&r, &[item], "id with records, again");
    assert_eq!(
        line,
        "publish: refused: T-050 already has records in metrics/tasks.jsonl"
    );
}

#[test]
fn a_failing_git_add_in_step_2_is_a_commit_failure() {
    // Ruling l: a failing `git add` (a locked index) in step 2 → exit 2 and the pinned step-2
    // line, then nothing else.
    let r = publish_repo("t013-l-indexlock");
    let item = "scorecard/items/x.y.toml";
    write(&r.root, item, NEW_ITEM);
    let start = r.main_sha();
    let lock = r.root.join(".git/index.lock");
    std::fs::write(&lock, "").unwrap();
    let o = publish(&r, &[item]);
    std::fs::remove_file(&lock).unwrap();
    assert_eq!(
        o.code, 2,
        "commit failure exit:\n{}\n{}",
        o.stdout, o.stderr
    );
    let lines: Vec<&str> = o.stderr.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 1, "one stderr line:\n{}", o.stderr);
    let tail = format!(
        "; commit by hand: git commit -m \"publish: T-050 ready\" -m \"Task: T-050\" -m \"Role: director\" -- {SLUG_FILE} metrics/tasks.jsonl"
    );
    assert!(
        lines[0].starts_with("publish: commit failed: ") && lines[0].ends_with(&tail),
        "step-2 line: {}",
        lines[0]
    );
    assert!(
        lines[0].contains("index.lock"),
        "the git error names the lock: {}",
        lines[0]
    );
    assert_eq!(r.main_sha(), start, "main moved");
    assert!(!r.wt("T-050").exists(), "a worktree was made");
    assert!(
        git(&r.root, &["branch", "--list", "task/T-050"]).is_empty(),
        "a branch was made"
    );
}

#[test]
fn publish_refuses_while_the_queue_lock_is_held_and_releases_it_for_the_queue() {
    // Ruling m: lock `<git-common-dir>/kiln-queue.lock` held by a live process (this test's own)
    // → exit 1, `publish: refused: another queue or publish run holds <lock path>`, nothing
    // changed, the queue's build tree included. Without the lock, publish merges (the lock is
    // released before step 8, where the queue takes it itself).
    let r = publish_repo("t013-m-lock");
    let item = "scorecard/items/x.y.toml";
    write(&r.root, item, NEW_ITEM);
    let lock = r.root.join(".git/kiln-queue.lock");
    let holder = format!("{} T-099\n", std::process::id());
    std::fs::write(&lock, &holder).unwrap();
    assert!(
        Path::new("/proc")
            .join(std::process::id().to_string())
            .exists()
    );
    let build = r.root.join(".git/kiln-queue");
    let build_before = tree_snap(&build);
    let line = assert_publish_refused(&r, &[item], "queue lock held");
    let prefix = "publish: refused: another queue or publish run holds ";
    let named = line
        .strip_prefix(prefix)
        .unwrap_or_else(|| panic!("expected `{prefix}<lock path>`, got `{line}`"));
    let named = if Path::new(named).is_absolute() {
        Path::new(named).to_path_buf()
    } else {
        r.root.join(named)
    };
    assert_eq!(
        named.canonicalize().ok(),
        lock.canonicalize().ok(),
        "the line names the lock file: {line}"
    );
    assert_eq!(
        std::fs::read_to_string(&lock).unwrap(),
        holder,
        "the held lock is untouched"
    );
    assert_eq!(
        tree_snap(&build),
        build_before,
        "the queue's build tree changed"
    );
    assert!(
        git(&r.root, &["ls-tree", "--name-only", "main", "tasks/"]).is_empty(),
        "no task file on main"
    );

    // The lock released: a normal publish merges and leaves no live lock behind.
    std::fs::remove_file(&lock).unwrap();
    let o = publish(&r, &[item]);
    assert_ok(&o, "publish without a held lock");
    assert_eq!(
        git(&r.root, &["log", "-1", "--format=%s", "main"]),
        "publish: T-050 merged"
    );
    assert_eq!(
        r.queue_lines().last().map(|q| q["result"].clone()),
        Some("merged".into()),
        "the queue merged T-050"
    );
    if let Ok(left) = std::fs::read_to_string(&lock) {
        let pid = left.split_whitespace().next().unwrap_or_default();
        assert!(
            pid.is_empty() || !Path::new("/proc").join(pid).exists(),
            "a live process still holds {}: {left}",
            lock.display()
        );
    }
}

#[test]
fn publish_refuses_a_title_without_a_letter_or_digit() {
    // Ruling p, last sentence (finding 13).
    let r = publish_repo("t013-p-title");
    let item = "scorecard/items/x.y.toml";
    write(&r.root, item, NEW_ITEM);
    for title in ["!!!", "... ???", "_"] {
        let before = snap(&r, &[item]);
        let o = xt(
            &r.root,
            &[
                "publish",
                "T-050",
                "--title",
                title,
                "--iteration",
                "1",
                item,
            ],
        );
        let line = assert_refusal(&o, "publish: refused: ", title);
        assert_eq!(
            line, "publish: refused: the title has no letter or digit",
            "title {title:?}"
        );
        assert_eq!(
            snap(&r, &[item]),
            before,
            "title {title:?} changed something"
        );
    }
}
