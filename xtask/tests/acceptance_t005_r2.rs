//! Acceptance tests for T-005 round 2: the director's rulings D-019 (c) `config` runs just before
//! `touches`, (d) `guard_tNNN*.rs` files are allowed paths like `acceptance_tNNN*.rs`, (e)
//! `cargo xtask join` finds acceptance and guard tests under `xtask/tests`, and (f) `xtask`
//! refuses to run in a checkout other than the one it was built from, while legitimate uses
//! (its own checkout, an unrelated scratch repository as in every test here) keep working.
//! Written from the task file and D-019 alone; nothing here touches the Kiln repository except
//! reading this checkout's tracked files to build a private copy of `xtask` (ruling f).

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

#[path = "acceptance_t005_support.rs"]
mod shared;
use shared::support::*;

// ---- D-019 (c) ----------------------------------------------------------------------------

#[test]
fn config_is_checked_just_before_touches() {
    let r = Repo::new("r2-config-order");
    // Each task also changes README.md, outside its `touches`.
    let readme: &[(&str, Option<&str>)] = &[("README.md", Some("# changed\n"))];
    r.add_task("T-001", "low", "self_verified", &["crates/a"], &[]);
    r.commit_wt("T-001", "outside touches", readme);
    r.add_task("T-002", "low", "draft", &["crates/a"], &[]);
    r.commit_wt("T-002", "outside touches", readme);
    r.add_task("T-003", "low", "self_verified", &["crates/a"], &[]);
    let t3 = read(&r.wt("T-003"), "tasks/T-003-toy.md").replace("tier: low", "tier: medium");
    r.commit_wt(
        "T-003",
        "front matter and outside touches",
        &[
            ("README.md", Some("# changed\n")),
            ("tasks/T-003-toy.md", Some(&t3)),
        ],
    );
    let good = r.main_sha();
    // With valid config, T-001 is refused at `touches`.
    r.assert_refused("T-001", "touches", Some("README.md"));
    for broken in [
        vec![("xtask/generated.toml", None)],
        vec![("xtask/ledger.toml", Some("= = =\n"))],
    ] {
        r.main_commit("break config", &broken);
        // `config` comes before `touches` …
        r.assert_refused("T-001", "config", None);
        // … but after `state` and `front-matter`.
        r.assert_refused("T-002", "state", None);
        r.assert_refused("T-003", "front-matter", None);
        r.reset_main(&good);
    }
}

// ---- D-019 (d) ----------------------------------------------------------------------------

#[test]
fn guard_files_are_allowed_paths_for_medium_only() {
    let r = Repo::new("r2-guard-paths");
    r.add_task("T-007", "medium", "reviewed", &["crates/a"], &[]);
    r.add_task("T-008", "low", "self_verified", &["crates/a"], &[]);
    // Medium: an allowed path passes `touches` and is refused at `join` (no join record).
    let base = r.head("T-007");
    for (p, ok) in [
        ("crates/b/tests/guard_t007.rs", true),
        ("apps/k/tests/guard_t007_seed.rs", true),
        ("xtask/tests/guard_t007.rs", true),
        ("crates/b/tests/guard_t008.rs", false),
        ("crates/b/tests/sub/guard_t007.rs", false),
        ("crates/b/guard_t007.rs", false),
        ("crates/b/tests/guard_t007.txt", false),
    ] {
        r.commit_wt("T-007", p, &[(p, Some("// guard\n"))]);
        if ok {
            r.assert_refused("T-007", "join", None);
        } else {
            r.assert_refused("T-007", "touches", Some(p));
        }
        r.reset_wt("T-007", &base);
    }
    // Low: no test-file exemption at all.
    let base = r.head("T-008");
    for p in ["crates/b/tests/guard_t008.rs", "xtask/tests/guard_t008.rs"] {
        r.commit_wt("T-008", p, &[(p, Some("// guard\n"))]);
        r.assert_refused("T-008", "touches", Some(p));
        r.reset_wt("T-008", &base);
    }
}

// ---- D-019 (e) ----------------------------------------------------------------------------

#[cfg(test)]
fn last_join_record(root: &Path) -> Value {
    let text = std::fs::read_to_string(root.join("metrics/tasks.jsonl")).unwrap_or_default();
    text.lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .rfind(|v| v["kind"] == "join")
        .unwrap_or(Value::Null)
}

#[cfg(test)]
fn test_reading(rel: &str, name: &str, assertion: &str) -> String {
    format!(
        "#[test]\nfn {name}() {{\n    let p = std::path::Path::new(env!(\"CARGO_MANIFEST_DIR\")).join(\"{rel}\");\n    {assertion}\n}}\n"
    )
}

#[test]
fn join_finds_acceptance_and_guard_tests_under_xtask_tests() {
    let r = Repo::new("r2-join-xtask-tests");
    for id in ["T-001", "T-002"] {
        r.add_task(id, "medium", "tests_joined", &["crates/a"], &[]);
        r.worktree(&format!("{id}-tests"));
    }
    // T-001: the acceptance test fails on base and passes once joined; the guard passes on
    // both (D-010), so it must not make the join refuse as "passes on base".
    let tests1 = r.commit_wt(
        "T-001-tests",
        "tests",
        &[
            (
                "xtask/tests/acceptance_t001.rs",
                Some(&test_reading(
                    "../crates/a/src/lib.rs",
                    "value_is_two",
                    "assert!(std::fs::read_to_string(&p).unwrap().contains(\"    2\\n\"));",
                )),
            ),
            (
                "xtask/tests/guard_t001.rs",
                Some(&test_reading(
                    "src/main.rs",
                    "toy_xtask_source_exists",
                    "assert!(p.is_file());",
                )),
            ),
        ],
    );
    r.commit_wt("T-001", "impl", &[("crates/a/src/lib.rs", Some(LIB_V2))]);
    let o = xtask(&r.root, &["join", "T-001"]);
    let rec = last_join_record(&r.root);
    assert_eq!(
        (
            o.code,
            rec["result"].as_str(),
            rec["base_fail_kind"].as_str()
        ),
        (0, Some("joined"), Some("fails")),
        "join T-001: {rec}\n{}\n{}",
        o.stdout,
        o.stderr
    );
    assert_eq!(rec["tests_sha"], tests1.as_str());
    assert!(rec["acc_total"].as_u64().unwrap_or(0) >= 1, "{rec}");

    // T-002: a guard found under xtask/tests that fails on the joined branch keeps the join
    // from succeeding.
    r.commit_wt(
        "T-002-tests",
        "tests",
        &[
            (
                "xtask/tests/acceptance_t002.rs",
                Some(&test_reading(
                    "../crates/a/src/new.rs",
                    "new_module_exists",
                    "assert!(p.is_file());",
                )),
            ),
            (
                "xtask/tests/guard_t002.rs",
                Some(&test_reading(
                    "../other/keep.txt",
                    "keep_file_survives",
                    "assert!(p.is_file());",
                )),
            ),
        ],
    );
    r.commit_wt(
        "T-002",
        "impl that breaks the guard",
        &[
            ("crates/a/src/new.rs", Some("// new\n")),
            ("other/keep.txt", None),
        ],
    );
    let o = xtask(&r.root, &["join", "T-002"]);
    let rec = last_join_record(&r.root);
    assert_eq!(
        rec["task"], "T-002",
        "no join record for T-002: {}",
        o.stderr
    );
    assert_ne!(rec["result"], "joined", "{rec}");
    assert_ne!(o.code, 0, "join T-002 must not succeed: {rec}");
}

// ---- D-019 (f) ----------------------------------------------------------------------------

/// The checkout this test binary (and `CARGO_BIN_EXE_xtask`) was built from.
#[cfg(test)]
fn this_checkout() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Copy this checkout's files (tracked and untracked-unignored; read only) into `dest`, make it
/// a git repository and build its `xtask` into a private, persistent target directory outside
/// `dest`. Returns the built binary.
#[cfg(test)]
fn build_private_xtask(dest: &Path) -> PathBuf {
    let src = this_checkout();
    let o = run(Command::new("git")
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .current_dir(&src));
    assert_eq!(o.code, 0, "ls-files: {}", o.stderr);
    for rel in o.stdout.split('\0').filter(|s| !s.is_empty()) {
        let from = src.join(rel);
        if !from.is_file() {
            continue;
        }
        let to = dest.join(rel);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::copy(&from, &to).unwrap();
    }
    git(dest, &["init", "-q", "-b", "main"]);
    git(dest, &["add", "-A"]);
    git(dest, &["commit", "-q", "-m", "copy"]);
    let target = Path::new(env!("CARGO_TARGET_TMPDIR")).join("t005-r2-private-xtask-target");
    let b = run(Command::new("cargo")
        .args(["build", "--offline", "--locked", "--package", "xtask"])
        .current_dir(dest)
        .env("CARGO_TARGET_DIR", &target));
    assert_eq!(b.code, 0, "build private xtask: {}", b.stderr);
    target.join("debug/xtask")
}

#[cfg(test)]
fn run_bin(bin: &Path, dir: &Path, args: &[&str]) -> Out {
    run(Command::new(bin).args(args).current_dir(dir))
}

#[cfg(test)]
fn names(line: &str, p: &Path) -> bool {
    let canon = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    line.contains(&*p.to_string_lossy()) || line.contains(&*canon.to_string_lossy())
}

#[test]
fn xtask_refuses_a_checkout_it_was_not_built_from() {
    let s = scratch("r2-built-from");
    let built = s.join("built");
    let bin = build_private_xtask(&built);
    // Another checkout of the same repository, whose xtask source differs.
    let other = s.join("other");
    git(
        &built,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "other",
            &other.to_string_lossy(),
        ],
    );
    let main_rs = other.join("xtask/src/main.rs");
    let changed = std::fs::read_to_string(&main_rs).unwrap() + "// changed in the other checkout\n";
    std::fs::write(&main_rs, changed).unwrap();
    commit_all(&other, "change xtask", &[]);

    // Refused: exit 2, one stderr line naming both checkouts.
    let o = run_bin(&bin, &other, &["worktree", "list"]);
    let lines: Vec<&str> = o.stderr.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(
        (o.code, lines.len()),
        (2, 1),
        "xtask built in {} run in {}: exit {}\nstdout:\n{}\nstderr:\n{}",
        built.display(),
        other.display(),
        o.code,
        o.stdout,
        o.stderr
    );
    assert!(
        names(lines[0], &built) && names(lines[0], &other),
        "refusal must name both {} and {}: {}",
        built.display(),
        other.display(),
        lines[0]
    );
    // Also from a subdirectory of the other checkout.
    let o = run_bin(&bin, &other.join("crates"), &["worktree", "list"]);
    assert_eq!(
        o.code, 2,
        "subdirectory of the other checkout: {}",
        o.stderr
    );

    // Legitimate: its own checkout, and a subdirectory of it.
    let o = run_bin(&bin, &built, &["worktree", "list"]);
    assert_eq!(o.code, 0, "own checkout: {}{}", o.stdout, o.stderr);
    let o = run_bin(&bin, &built.join("xtask"), &["worktree", "list"]);
    assert_eq!(
        o.code, 0,
        "own checkout, subdirectory: {}{}",
        o.stdout, o.stderr
    );
    // Legitimate: an unrelated scratch repository (how every T-005 test drives xtask), with the
    // private binary and with the binary under test.
    let r = Repo::new("r2-built-from-unrelated");
    let o = run_bin(&bin, &r.root, &["worktree", "list"]);
    assert_eq!(o.code, 0, "unrelated repository: {}{}", o.stdout, o.stderr);
    let o = xtask(&r.root, &["worktree", "list"]);
    assert_eq!(
        o.code, 0,
        "binary under test, unrelated repository: {}{}",
        o.stdout, o.stderr
    );
}
