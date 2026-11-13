//! T-013 acceptance tests, Interface (6): the gate step `hygiene` with its `reviews` and
//! `session-paths` checks (AC-11, AC-12), its place in `cargo xtask ci` and CHARTER §4.
//!
//! Written from the task file and ADR-0003 alone. Each test builds a throw-away git repository
//! and runs `xtask hygiene` (or `xtask ci`) there; the "this repository's tree" checks run on a
//! fresh clone of the tested commit. A session-specific path is only ever built at run time, so
//! no tracked file of this checkout contains one.

use std::path::Path;
use std::process::Command;

#[path = "acceptance_t013_support.rs"]
mod t13;
use t13::*;

const MUTANTS_OK: &str =
    "{introduced: 12, equivalent: 1, killed: 10, killed_by_acceptance: 9, survived: 1}";

#[cfg(test)]
fn review(task: &str, round: u32, tier: &str, arm: &str, mutants: Option<&str>) -> String {
    let m = mutants
        .map(|m| {
            if m.starts_with('\n') {
                format!("mutants:{m}\n")
            } else {
                format!("mutants: {m}\n")
            }
        })
        .unwrap_or_default();
    format!(
        "---\ntask: {task}\nround: {round}\nrun: R-900\ntier: {tier}\narm: {arm}\nreviewed_sha: {s}\nprevious_sha: null\nverdict: approve\nfindings: {{defect: 0, spec_gap: 0, test_gap: 0, nit: 0}}\n{m}follow_ups: 0\n---\n\n# Review {task} round {round}\n",
        s = sha(5)
    )
}

#[cfg(test)]
/// Commit everything in `l`.
fn commit(l: &Lite, msg: &str) {
    git(&l.root, &["add", "-A"]);
    let o = run_to(
        Command::new("git")
            .args(["commit", "-q", "--allow-empty", "-m", msg])
            .current_dir(&l.root),
        60,
    );
    assert_ok(&o, "git commit");
}

#[cfg(test)]
/// Run `xtask hygiene`; return (passed, combined output).
fn hygiene_step(dir: &Path) -> (bool, String) {
    let o = xt(dir, &["hygiene"]);
    let all = format!("{}{}", o.stdout, o.stderr);
    match o.code {
        0 => {
            assert!(all.contains("PASS"), "exit 0 without PASS:\n{all}");
            (true, all)
        }
        1 => {
            assert!(all.contains("FAIL"), "exit 1 without FAIL:\n{all}");
            (false, all)
        }
        c => panic!("xtask hygiene: exit {c}\n{all}"),
    }
}

#[cfg(test)]
fn reviews_repo(name: &str) -> Lite {
    let l = Lite::new(name);
    l.task(&TaskSpec::new("T-021", "medium", 1));
    l.task(&TaskSpec::new("T-022", "medium", 0));
    l.task(&TaskSpec::new("T-023", "low", 1));
    l.task(&TaskSpec::new("T-024", "high", 1));
    l.task(&TaskSpec::new("T-026", "medium", 1));
    let r = |p: &str, c: String| write(&l.root, &format!("reviews/{p}"), &c);
    r(
        "T-021-r1.md",
        review("T-021", 1, "medium", "implementer", Some(MUTANTS_OK)),
    );
    // Round 2 may introduce fewer than 10.
    r(
        "T-021-r2.md",
        review(
            "T-021",
            2,
            "medium",
            "implementer",
            Some("{introduced: 9, equivalent: 0, killed: 8, killed_by_acceptance: 8, survived: 1}"),
        ),
    );
    // Iteration-0 and low tasks are not checked.
    r(
        "T-022-r1.md",
        review("T-022", 1, "medium", "independent", None).replace("arm: independent\n", ""),
    );
    r(
        "T-023-r1.md",
        review("T-023", 1, "low", "independent", None),
    );
    r(
        "T-024-r1.md",
        review(
            "T-024",
            1,
            "high",
            "independent",
            Some(
                "{introduced: 10, equivalent: 0, killed: 10, killed_by_acceptance: 0, survived: 0}",
            ),
        ),
    );
    r(
        "T-026-r1.md",
        review("T-026", 1, "medium", "independent", Some(MUTANTS_OK)),
    );
    // Ruling: `mutants` may also be an indented block map.
    l.task(&TaskSpec::new("T-028", "high", 1));
    r(
        "T-028-r1.md",
        review(
            "T-028",
            1,
            "high",
            "independent",
            Some(
                "\n  introduced: 10\n  equivalent: 0\n  killed: 9\n  killed_by_acceptance: 9\n  survived: 1",
            ),
        ),
    );
    // Other names under reviews/ are ignored.
    r(
        "T-021-join-triage.md",
        "# triage, no front matter\n".to_owned(),
    );
    r("notes.md", "notes\n".to_owned());
    commit(&l, "reviews");
    l
}

#[test]
fn reviews_check_accepts_v11_front_matter_and_names_each_faulty_file() {
    let l = reviews_repo("ac11-reviews");
    let (ok, out) = hygiene_step(&l.root);
    assert!(ok, "the baseline must pass:\n{out}");

    let file = "reviews/T-021-r1.md";
    let good = read(&l.root, file);
    let faults: Vec<(&str, String)> = vec![
        (
            "no mutants",
            review("T-021", 1, "medium", "implementer", None),
        ),
        (
            "a missing key",
            review(
                "T-021",
                1,
                "medium",
                "implementer",
                Some("{introduced: 11, killed: 10, killed_by_acceptance: 9, survived: 1}"),
            ),
        ),
        (
            "introduced 9 in round 1",
            review(
                "T-021",
                1,
                "medium",
                "implementer",
                Some(
                    "{introduced: 9, equivalent: 0, killed: 8, killed_by_acceptance: 8, survived: 1}",
                ),
            ),
        ),
        (
            "sums do not add up",
            review(
                "T-021",
                1,
                "medium",
                "implementer",
                Some(
                    "{introduced: 12, equivalent: 1, killed: 10, killed_by_acceptance: 9, survived: 2}",
                ),
            ),
        ),
        (
            "killed_by_acceptance > killed",
            review(
                "T-021",
                1,
                "medium",
                "implementer",
                Some(
                    "{introduced: 12, equivalent: 1, killed: 10, killed_by_acceptance: 11, survived: 1}",
                ),
            ),
        ),
        (
            "a negative count",
            review(
                "T-021",
                1,
                "medium",
                "implementer",
                Some(
                    "{introduced: 10, equivalent: 0, killed: 11, killed_by_acceptance: 0, survived: -1}",
                ),
            ),
        ),
        (
            "a non-integer count",
            review(
                "T-021",
                1,
                "medium",
                "implementer",
                Some(
                    "{introduced: 12, equivalent: 1, killed: ten, killed_by_acceptance: 9, survived: 1}",
                ),
            ),
        ),
        (
            "arm: independent",
            review("T-021", 1, "medium", "independent", Some(MUTANTS_OK)),
        ),
        (
            "tier: high",
            review("T-021", 1, "high", "implementer", Some(MUTANTS_OK)),
        ),
        (
            "round: 2 in r1",
            review("T-021", 2, "medium", "implementer", Some(MUTANTS_OK)),
        ),
        (
            "task: T-026",
            review("T-026", 1, "medium", "implementer", Some(MUTANTS_OK)),
        ),
        (
            "no arm",
            review("T-021", 1, "medium", "implementer", Some(MUTANTS_OK))
                .replace("arm: implementer\n", ""),
        ),
        (
            "unparsable front matter",
            "---\ntask: T-021\nround: 1\n".to_owned(),
        ),
    ];
    for (what, text) in &faults {
        write(&l.root, file, text);
        commit(&l, what);
        let (ok, out) = hygiene_step(&l.root);
        assert!(!ok, "{what}: expected FAIL\n{out}");
        assert!(
            out.contains(&format!("{file}: ")),
            "{what}: FAIL must name {file}:\n{out}"
        );
        assert!(
            out.contains("reviews:"),
            "{what}: the detail must name the check `reviews:`\n{out}"
        );
        assert!(
            !out.contains("reviews/T-024-r1.md"),
            "{what}: a good review was named\n{out}"
        );
        write(&l.root, file, &good);
        commit(&l, "restore");
    }

    // An even medium task's arm is independent; implementer there fails.
    let even = "reviews/T-026-r1.md";
    write(
        &l.root,
        even,
        &review("T-026", 1, "medium", "implementer", Some(MUTANTS_OK)),
    );
    commit(&l, "even arm");
    let (ok, out) = hygiene_step(&l.root);
    assert!(!ok && out.contains(&format!("{even}: ")), "{out}");
    write(
        &l.root,
        even,
        &review("T-026", 1, "medium", "independent", Some(MUTANTS_OK)),
    );

    // A review without a task file fails.
    let orphan = "reviews/T-029-r1.md";
    write(
        &l.root,
        orphan,
        &review("T-029", 1, "medium", "implementer", Some(MUTANTS_OK)),
    );
    commit(&l, "orphan");
    let (ok, out) = hygiene_step(&l.root);
    assert!(!ok && out.contains(&format!("{orphan}: ")), "{out}");
    std::fs::remove_file(l.root.join(orphan)).unwrap();
    commit(&l, "no orphan");
    let (ok, out) = hygiene_step(&l.root);
    assert!(ok, "after the repairs:\n{out}");
}

#[cfg(test)]
/// A session-specific scratch path, assembled at run time.
fn session_path() -> (String, String) {
    let uuid = ["3f2a9c1e", "7b4d", "4e8a", "9c0f", "1a2b3c4d5e6f"].join("-");
    (
        format!(
            "/{}/{}-{}/{}/{}/scratchpad/m.py",
            "tmp", "claude", 0, "-home-user", uuid
        ),
        uuid,
    )
}

#[test]
fn session_paths_check_flags_tracked_files_by_path_and_line_only() {
    let l = Lite::new("ac12-paths");
    let (p, uuid) = session_path();
    let (ok, out) = hygiene_step(&l.root);
    assert!(ok, "a clean toy repository passes:\n{out}");

    write(
        &l.root,
        "notes/a.md",
        &format!("one\ntwo\nsee {p} for the script\n"),
    );
    write(&l.root, "deep/dir/b.txt", &format!("{p}\n"));
    commit(&l, "paths");
    let (ok, out) = hygiene_step(&l.root);
    assert!(!ok, "tracked session paths must FAIL:\n{out}");
    assert!(out.contains("notes/a.md:3"), "names notes/a.md:3\n{out}");
    assert!(
        out.contains("deep/dir/b.txt:1"),
        "names deep/dir/b.txt:1\n{out}"
    );
    assert!(out.contains("session-paths:"), "names the check\n{out}");
    assert!(
        !out.contains(&uuid) && !out.contains("/tmp/claude"),
        "never prints the matched text:\n{out}"
    );

    // Untracked: not checked.
    git(
        &l.root,
        &["rm", "-q", "--cached", "notes/a.md", "deep/dir/b.txt"],
    );
    git(&l.root, &["commit", "-q", "-m", "untrack"]);
    assert!(l.root.join("notes/a.md").exists());
    assert!(
        git(&l.root, &["ls-files"])
            .lines()
            .all(|f| f != "notes/a.md")
    );
    let (ok, out) = hygiene_step(&l.root);
    assert!(ok, "untracked files are not checked:\n{out}");
    // Gone from disk too, so later `git add -A` does not track them again.
    std::fs::remove_file(l.root.join("notes/a.md")).unwrap();
    std::fs::remove_file(l.root.join("deep/dir/b.txt")).unwrap();

    // Texts that do not match the regular expression.
    let near_misses = [
        // The pattern itself, as written in the task file.
        "/tmp/claude-[0-9]+/[^/[:space:]]+/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}".to_owned(),
        p.replace(&uuid, &uuid.to_uppercase()),
        p.replace("claude-0", "claude-x"),
        p.replace("-home-user", "a b"),
        p.replace("-home-user/", ""),
        p.replace(&uuid, &uuid[..uuid.len() - 1]),
    ];
    let mut text = String::new();
    for n in &near_misses {
        text.push_str(n);
        text.push('\n');
    }
    write(&l.root, "notes/regex.md", &text);
    // A non-UTF-8 tracked file is skipped, even if it holds the bytes of a match.
    let mut bin = vec![0xff, 0xfe, 0x00];
    bin.extend_from_slice(p.as_bytes());
    bin.push(b'\n');
    std::fs::write(l.root.join("blob.bin"), bin).unwrap();
    git(&l.root, &["add", "notes/regex.md", "blob.bin"]);
    commit(&l, "near misses and binary");
    let (ok, out) = hygiene_step(&l.root);
    assert!(ok, "near misses and non-UTF-8 files pass:\n{out}");

    // The shortest matching form, mid-line, does fail.
    write(
        &l.root,
        "notes/regex.md",
        &format!("{text}x{}y\n", p.trim_end_matches("/scratchpad/m.py")),
    );
    commit(&l, "short");
    let (ok, out) = hygiene_step(&l.root);
    assert!(
        !ok && out.contains(&format!("notes/regex.md:{}", near_misses.len() + 1)),
        "{out}"
    );
}

#[cfg(test)]
/// A clone of the commit under test (this checkout's HEAD), a repository of the test's own.
fn clone_of_head(name: &str) -> std::path::PathBuf {
    let src = checkout_root();
    let head = git(&src, &["rev-parse", "HEAD"]);
    let dst = scratch(&format!("t013-{name}")).join("clone");
    let o = run_to(
        Command::new("git")
            .args(["clone", "-q", "--shared", "--no-checkout"])
            .arg(&src)
            .arg(&dst),
        300,
    );
    assert_ok(&o, "git clone");
    git(&dst, &["checkout", "-q", "--detach", &head]);
    dst
}

#[test]
fn this_repositorys_tree_passes_hygiene() {
    let dst = clone_of_head("ac11-self");
    let (ok, out) = hygiene_step(&dst);
    assert!(ok, "hygiene on this repository's tree:\n{out}");
    let _ = std::fs::remove_dir_all(dst.parent().unwrap());
}

#[test]
fn ci_and_charter_list_hygiene_between_ui_commands_and_schema() {
    // A toy workspace whose `fmt` step fails at once, so the gate stops after its first line.
    let l = Lite::new("ac12-ci");
    write(
        &l.root,
        "Cargo.toml",
        "[workspace]\nresolver = \"2\"\nmembers = [\"a\"]\n",
    );
    write(
        &l.root,
        "a/Cargo.toml",
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(&l.root, "a/src/lib.rs", "pub   fn  x( ){ }\n");
    commit(&l, "toy crate");
    let o = xt(&l.root, &["ci"]);
    let first = o
        .stdout
        .lines()
        .find(|l| l.starts_with("gate: "))
        .unwrap_or_default();
    assert_eq!(
        first,
        "gate: 11 steps: fmt, clippy, layers, ui-commands, hygiene, schema, scorecard, test, no-skip, fuzz, budgets",
        "stdout:\n{}\nstderr:\n{}",
        o.stdout,
        o.stderr
    );

    let charter = std::fs::read_to_string(checkout_root().join("docs/CHARTER.md")).unwrap();
    let flat = charter.split_whitespace().collect::<Vec<_>>().join(" ");
    let start = flat
        .find("Steps, cheap first:")
        .expect("CHARTER §4 gate line");
    let line = &flat[start
        ..start
            + flat[start..]
                .find("enforced budgets")
                .expect("end of gate line")];
    let pos = |w: &str| line.find(w).unwrap_or_else(|| panic!("{w} not in: {line}"));
    assert!(line.contains("`ui-commands`, `hygiene`"), "{line}");
    assert!(pos("`hygiene`") < pos("`schema`"), "{line}");
    assert_eq!(line.matches("hygiene").count(), 1, "{line}");
}

#[test]
fn session_paths_checks_a_tracked_symlinks_target() {
    // Ruling o (D-047): `session-paths` checks a tracked symlink's target text as it checks a
    // file's content (`<path>:<line>`, never the matched text).
    let l = Lite::new("o-symlink");
    let (p, uuid) = session_path();
    std::fs::create_dir_all(l.root.join("links")).unwrap();
    std::os::unix::fs::symlink("/tmp/elsewhere/m.py", l.root.join("links/plain")).unwrap();
    commit(&l, "plain link");
    let (ok, out) = hygiene_step(&l.root);
    assert!(ok, "a link to an ordinary path passes:\n{out}");

    std::os::unix::fs::symlink(&p, l.root.join("links/session")).unwrap();
    commit(&l, "session link");
    assert!(
        git(&l.root, &["ls-files"])
            .lines()
            .any(|f| f == "links/session"),
        "the link is tracked"
    );
    assert_eq!(
        git(&l.root, &["cat-file", "-p", "HEAD:links/session"]),
        p,
        "git stores the link's target text"
    );
    let (ok, out) = hygiene_step(&l.root);
    assert!(!ok, "a tracked link to a session path must FAIL:\n{out}");
    assert!(
        out.contains("session-paths:") && out.contains("links/session:1"),
        "names links/session:1 under session-paths\n{out}"
    );
    assert!(
        !out.contains("links/plain"),
        "the ordinary link is not named\n{out}"
    );
    assert!(
        !out.contains(&uuid) && !out.contains("/tmp/claude"),
        "never prints the matched text:\n{out}"
    );
}
