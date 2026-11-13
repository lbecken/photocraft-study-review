//! Acceptance tests for T-005 round 4: the director's rulings R10–R13 of decision D-030, written
//! from the task file on `main` alone. Each test pairs the bad attempt (refused or made
//! harmless) with the legitimate case (still merges), so a queue that refuses everything fails.
//!
//! Every test drives the `xtask` binary under test against throwaway toy repositories built by
//! the shared fixture (`acceptance_t005_support.rs`) under `CARGO_TARGET_TMPDIR`. Evidence is
//! committed on main before each run (R6). Outside the R12 test, any `metrics/` change a run
//! leaves is committed by the test (a no-op once the queue commits its own record).

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

#[path = "acceptance_t005_support.rs"]
mod shared;
use shared::support::*;

/// File changes: path → new content, or `None` to delete.
type Files<'a> = &'a [(&'a str, Option<&'a str>)];
/// One file change.
type Change<'a> = (&'a str, Option<&'a str>);
/// Extra environment for a queue run.
type Env = Vec<(&'static str, String)>;

// ---- helpers ------------------------------------------------------------------------------

fn status_of(dir: &Path, path: &str) -> String {
    let o = run(Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=all", "--", path])
        .current_dir(dir));
    assert_eq!(o.code, 0, "git status: {}", o.stderr);
    o.stdout
}

/// Commit pending `metrics/` changes on main (a director ledger commit), if any.
fn commit_ledger(r: &Repo) {
    if !status_of(&r.root, "metrics").is_empty() {
        git(&r.root, &["add", "-A", "--", "metrics"]);
        git(&r.root, &["commit", "-q", "-m", "director: ledger"]);
    }
}

/// `xtask queue <id>` from a director shell, plus `env`.
fn queue_env(r: &Repo, id: &str, env: &[(&'static str, String)]) -> Out {
    let mut c = Command::new(XTASK);
    c.args(["queue", id])
        .current_dir(&r.root)
        .env_remove("KILN_XTASK_RUN_IN");
    for (k, v) in env {
        c.env(k, v);
    }
    run(&mut c)
}

/// Run the queue, assert exactly one new record for `id`; return output and record.
fn record_raw(r: &Repo, id: &str, env: &[(&'static str, String)]) -> (Out, Value) {
    let before = r.queue_lines().len();
    let o = queue_env(r, id, env);
    let lines = r.queue_lines();
    assert_eq!(
        lines.len(),
        before + 1,
        "queue {id} wrote no record (exit {}):\nstdout:\n{}\nstderr:\n{}",
        o.code,
        o.stdout,
        o.stderr
    );
    let rec = lines.last().cloned().unwrap_or(Value::Null);
    assert_eq!(rec["task"], id, "record: {rec}");
    (o, rec)
}

fn record(r: &Repo, id: &str, env: &[(&'static str, String)]) -> (Out, Value) {
    let out = record_raw(r, id, env);
    commit_ledger(r);
    out
}

/// Queue `id` and assert `merged`.
fn merged(r: &Repo, id: &str, env: &[(&'static str, String)]) -> (Out, Value) {
    let (o, rec) = record_raw(r, id, env);
    assert_eq!(
        rec["result"], "merged",
        "queue {id}: record {rec}\nstdout:\n{}\nstderr:\n{}",
        o.stdout, o.stderr
    );
    r.assert_only_records_since(rec["head_sha"].as_str().unwrap_or_default(), "merged head");
    commit_ledger(r);
    (o, rec)
}

/// Queue `id`; assert one refusal at `check` naming every one of `names`, and no change.
fn refused(r: &Repo, id: &str, check: &str, names: &[&str]) -> String {
    let before = r.snap(id);
    let o = queue_env(r, id, &[]);
    let after = r.snap(id);
    let lines: Vec<&str> = o
        .stderr
        .lines()
        .filter(|l| l.starts_with("queue: refused: "))
        .collect();
    assert_eq!(
        (o.code, lines.len()),
        (1, 1),
        "queue {id}: expected one refusal at `{check}`; exit {}\nstdout:\n{}\nstderr:\n{}",
        o.code,
        o.stdout,
        o.stderr
    );
    let prefix = format!("queue: refused: {check}: ");
    assert!(
        lines[0].starts_with(&prefix),
        "queue {id}: expected `{prefix}…`, got `{}`",
        lines[0]
    );
    let reason = lines[0][prefix.len()..].to_owned();
    for n in names {
        assert!(
            reason.contains(n),
            "queue {id}: refusal `{check}` should name `{n}`: {reason}"
        );
    }
    assert_eq!(before, after, "queue {id}: refusal `{check}` changed state");
    reason
}

fn commit_join(r: &Repo, id: &str, sha: &str) {
    let mut t = std::fs::read_to_string(r.root.join("metrics/tasks.jsonl")).unwrap_or_default();
    t.push_str(&join_rec(id, "joined", sha));
    t.push('\n');
    r.main_commit(
        "director: join record",
        &[("metrics/tasks.jsonl", Some(&t))],
    );
}

/// Committed medium/high evidence: code commit `h1`, its join record on main, an approving
/// round-1 review on the branch.
fn evidence(r: &Repo, id: &str, tier: &str, code: Files) -> String {
    let h1 = r.commit_wt(id, "implementation", code);
    commit_join(r, id, &h1);
    r.commit_wt(
        id,
        "review r1",
        &[(
            &format!("reviews/{id}-r1.md"),
            Some(&review_md(id, 1, tier, "approve", &h1)),
        )],
    );
    h1
}

/// Append a row to the decisions table on main.
fn decide(r: &Repo, decision: &str, reasoning: &str) {
    let mut t = read(&r.root, "docs/decisions.md");
    let n = t.lines().count();
    t.push_str(&format!(
        "| D-{n:03} | 2026-11-13 | {decision} | {reasoning} | yes |\n"
    ));
    r.main_commit("director: decision", &[("docs/decisions.md", Some(&t))]);
}

fn cargo_in(dir: &Path, args: &[&str]) {
    let o = run(Command::new("cargo")
        .args(args)
        .current_dir(dir)
        .env_remove("RUSTUP_TOOLCHAIN"));
    assert_eq!(
        o.code,
        0,
        "cargo {args:?} in {}: {}",
        dir.display(),
        o.stderr
    );
}

fn show(dir: &Path, rev: &str, path: &str) -> String {
    let o = run(Command::new("git")
        .args(["show", &format!("{rev}:{path}")])
        .current_dir(dir));
    assert_eq!(o.code, 0, "git show {rev}:{path}: {}", o.stderr);
    o.stdout
}

fn lock_version(lock: &str, name: &str) -> Option<String> {
    let key = format!("name = \"{name}\"\nversion = \"");
    let i = lock.find(&key)? + key.len();
    Some(lock[i..].split('"').next()?.to_owned())
}

const A_TOML: &str = "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";

/// Main gains a registry package `regdep` (1.0.0 and 1.1.0, served offline from a directory
/// source replacing crates.io), required by crate `a` as `"1"` and locked at 1.0.0. Done
/// before any merge, so `main-history` passes trivially.
fn add_registry_dep_on_main(r: &Repo) {
    for v in ["1.0.0", "1.1.0"] {
        let d = format!("vendored/regdep-{v}");
        write(
            &r.root,
            &format!("{d}/Cargo.toml"),
            &format!("[package]\nname = \"regdep\"\nversion = \"{v}\"\nedition = \"2021\"\n"),
        );
        write(
            &r.root,
            &format!("{d}/src/lib.rs"),
            "/// Toy.\npub fn r() -> u32 {\n    1\n}\n",
        );
        write(
            &r.root,
            &format!("{d}/.cargo-checksum.json"),
            "{\"files\":{},\"package\":null}\n",
        );
    }
    let cfg = read(&r.root, ".cargo/config.toml")
        + "\n[source.crates-io]\nreplace-with = \"toy-vendored\"\n\n[source.toy-vendored]\ndirectory = \"vendored\"\n";
    write(&r.root, ".cargo/config.toml", &cfg);
    write(
        &r.root,
        "crates/a/Cargo.toml",
        &format!("{A_TOML}\n[dependencies]\nregdep = \"1\"\n"),
    );
    cargo_in(&r.root, &["generate-lockfile", "--offline"]);
    let latest = read(&r.root, "Cargo.lock");
    let pinned = latest.replace(
        "name = \"regdep\"\nversion = \"1.1.0\"",
        "name = \"regdep\"\nversion = \"1.0.0\"",
    );
    assert_ne!(pinned, latest);
    write(&r.root, "Cargo.lock", &pinned);
    cargo_in(
        &r.root,
        &["metadata", "--offline", "--locked", "--format-version", "1"],
    );
    assert!(read(&r.root, "Cargo.lock").contains("source = \"registry+"));
    git(&r.root, &["add", "-A"]);
    git(&r.root, &["commit", "-q", "-m", "toy: registry dependency"]);
}

// ---- R10: lints tables and build keys are gate paths --------------------------------------

const GEN_RS: &str = "fn main() {}\n";
const LINTS: &str = "\n[lints.rust]\nunsafe_code = \"forbid\"\n";

/// Crate `a` on main: a `build` key naming `tools/gen_rs.rs`, and a `[lints]` table.
fn a_toml(build: Option<&str>, lints: &str) -> String {
    let b = build
        .map(|p| format!("build = \"{p}\"\n"))
        .unwrap_or_default();
    format!("{A_TOML}{b}{lints}")
}

fn lints_and_build_on_main(r: &Repo) {
    r.main_commit(
        "toy: build key and lints",
        &[
            (
                "crates/a/Cargo.toml",
                Some(&a_toml(Some("tools/gen_rs.rs"), LINTS)),
            ),
            ("crates/a/tools/gen_rs.rs", Some(GEN_RS)),
        ],
    );
}

#[test]
fn lints_tables_and_build_keys_are_gate_paths() {
    let r = Repo::new("r4-r10-low");
    lints_and_build_on_main(&r);
    r.add_task(
        "T-001",
        "low",
        "self_verified",
        &["crates/a", "crates/c"],
        &[],
    );
    let base = r.head("T-001");
    let a = "crates/a/Cargo.toml";
    let deny = a_toml(Some("tools/gen_rs.rs"), &LINTS.replace("forbid", "deny"));
    let no_lints = a_toml(Some("tools/gen_rs.rs"), "");
    let ws_lints = a_toml(Some("tools/gen_rs.rs"), "\n[lints]\nworkspace = true\n");
    let other_build = a_toml(Some("tools/gen2.rs"), LINTS);
    let no_build = a_toml(None, LINTS);
    let c_ws_lints = "lints.workspace = true\n\n[package]\nname = \"c\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";
    let c_build =
        "[package]\nname = \"c\"\nversion = \"0.1.0\"\nedition = \"2021\"\nbuild = \"make.rs\"\n";
    let cases: Vec<(&str, Vec<Change>)> = vec![
        ("lints changed", vec![(a, Some(deny.as_str()))]),
        ("lints removed", vec![(a, Some(no_lints.as_str()))]),
        ("lints.workspace set", vec![(a, Some(ws_lints.as_str()))]),
        (
            "build key changed",
            vec![
                (a, Some(other_build.as_str())),
                ("crates/a/tools/gen2.rs", Some(GEN_RS)),
            ],
        ),
        ("build key removed", vec![(a, Some(no_build.as_str()))]),
        (
            "file named by the build key edited",
            vec![(
                "crates/a/tools/gen_rs.rs",
                Some("fn main() { /* edited */ }\n"),
            )],
        ),
        (
            "new manifest with a dotted lints.workspace key",
            vec![
                ("crates/c/Cargo.toml", Some(c_ws_lints)),
                ("crates/c/src/lib.rs", Some("\n")),
            ],
        ),
        (
            "new manifest with a build key",
            vec![
                ("crates/c/Cargo.toml", Some(c_build)),
                ("crates/c/make.rs", Some(GEN_RS)),
                ("crates/c/src/lib.rs", Some("\n")),
            ],
        ),
    ];
    for (what, files) in &cases {
        r.commit_wt("T-001", what, files);
        // The refusal names the first file of the case (the gate path it introduces).
        refused(&r, "T-001", "gate-path", &[files[0].0]);
        r.reset_wt("T-001", &base);
    }
    // Medium is not enough either.
    r.add_task("T-002", "medium", "reviewed", &["crates/a"], &[]);
    evidence(&r, "T-002", "medium", &[(a, Some(&deny))]);
    refused(&r, "T-002", "gate-path", &[a]);

    // Legitimate: a manifest change that leaves lints and build alone, a source change, and a
    // file next to the build script that the build key does not name.
    let described = a_toml(Some("tools/gen_rs.rs"), LINTS).replace(
        "edition = \"2021\"\n",
        "edition = \"2021\"\ndescription = \"toy\"\n",
    );
    r.commit_wt(
        "T-001",
        "ordinary",
        &[
            (a, Some(&described)),
            ("crates/a/src/lib.rs", Some(LIB_V2)),
            ("crates/a/tools/notes.txt", Some("notes\n")),
        ],
    );
    let (_, rec) = merged(&r, "T-001", &[]);
    assert_eq!(rec["gate_xtask"], "main", "record: {rec}");
}

#[test]
fn lints_and_build_changes_on_a_high_task_need_a_gate_paths_approval() {
    let r = Repo::new("r4-r10-high");
    lints_and_build_on_main(&r);
    r.add_task("T-010", "high", "reviewed", &["crates/a"], &[]);
    r.add_task("T-011", "high", "reviewed", &["crates/a"], &[]);
    let a = "crates/a/Cargo.toml";
    let deny = a_toml(Some("tools/gen_rs.rs"), &LINTS.replace("forbid", "deny"));
    evidence(&r, "T-010", "high", &[(a, Some(&deny))]);
    let gen_rs = "crates/a/tools/gen_rs.rs";
    evidence(
        &r,
        "T-011",
        "high",
        &[(gen_rs, Some("fn main() { /* v2 */ }\n"))],
    );

    refused(&r, "T-010", "approval", &[]);
    decide(&r, "Approve T-010 for merge.", "toy");
    refused(&r, "T-010", "approval", &[]);
    decide(&r, "Approve T-011 for merge.", "it changes gate paths");
    refused(&r, "T-011", "approval", &[]);

    decide(&r, "Approve T-010 including its gate paths.", "toy");
    let (o, rec) = merged(&r, "T-010", &[]);
    assert_eq!(rec["gate_xtask"], "main+task", "record: {rec}");
    assert!(
        format!("{}{}", o.stdout, o.stderr).contains(a),
        "the queue must print the gate paths changed:\n{}\n{}",
        o.stdout,
        o.stderr
    );
    decide(&r, "Approve T-011: gate paths (build script).", "toy");
    let (o, rec) = merged(&r, "T-011", &[]);
    assert_eq!(rec["gate_xtask"], "main+task", "record: {rec}");
    assert!(
        format!("{}{}", o.stdout, o.stderr).contains(gen_rs),
        "the queue must print the gate paths changed:\n{}\n{}",
        o.stdout,
        o.stderr
    );
    assert_eq!(read(&r.root, a), deny);
}

// ---- R11: registry version changes need a `dependency versions` approval ------------------

#[test]
fn registry_version_change_needs_a_dependency_versions_approval() {
    let r = Repo::new("r4-r11-versions");
    add_registry_dep_on_main(&r);
    r.add_task("T-001", "low", "self_verified", &["crates/a"], &[]);
    r.add_task("T-002", "medium", "reviewed", &["crates/a"], &[]);
    r.add_task("T-003", "high", "reviewed", &["crates/a"], &[]);
    // Each bumps regdep 1.0.0 → 1.1.0 in the lock only.
    let wt = r.wt("T-001");
    cargo_in(&wt, &["update", "--offline", "-p", "regdep"]);
    let bumped = read(&wt, "Cargo.lock");
    git(&wt, &["checkout", "-q", "--", "Cargo.lock"]);
    assert_eq!(lock_version(&bumped, "regdep").as_deref(), Some("1.1.0"));
    let lock: Files = &[("Cargo.lock", Some(bumped.as_str()))];
    r.commit_wt("T-001", "bump", lock);
    refused(&r, "T-001", "dependencies", &[]);
    evidence(&r, "T-002", "medium", lock);
    refused(&r, "T-002", "dependencies", &[]);
    evidence(&r, "T-003", "high", lock);
    let table = |rows: &str| format!("{DECISIONS}{rows}");
    let row = |d: &str, why: &str| format!("| D-002 | 2026-11-13 | {d} | {why} | yes |\n");
    for text in [
        table(&row("Approve T-003 for merge.", "toy")),
        table(&row("Approve T-003 including its gate paths.", "toy")),
        table(&row(
            "Approve T-003 for merge.",
            "bumps dependency versions",
        )),
        table(&format!(
            "\n```\n{}```\n",
            row("Approve T-003: dependency versions.", "toy")
        )),
    ] {
        r.main_commit("director: decisions", &[("docs/decisions.md", Some(&text))]);
        refused(&r, "T-003", "dependencies", &[]);
    }
    // Legitimate: the approval names dependency versions.
    r.main_commit(
        "director: approve",
        &[(
            "docs/decisions.md",
            Some(&table(&row(
                "Approve T-003: dependency versions (regdep 1.1.0).",
                "toy",
            ))),
        )],
    );
    merged(&r, "T-003", &[]);
    assert_eq!(
        lock_version(&show(&r.root, "main", "Cargo.lock"), "regdep").as_deref(),
        Some("1.1.0")
    );
}

// ---- R12: the queue commits its own record ------------------------------------------------

/// Subject and files of main's tip commit.
fn tip(r: &Repo) -> (String, String) {
    (
        git(&r.root, &["log", "-1", "--format=%s", "main"]),
        git(
            &r.root,
            &["show", "--name-only", "--format=", "--no-renames", "main"],
        ),
    )
}

/// The last record in main's committed `metrics/queue.jsonl`.
fn committed_last_record(r: &Repo) -> Value {
    show(&r.root, "main", "metrics/queue.jsonl")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .next_back()
        .unwrap_or(Value::Null)
}

#[test]
fn queue_commits_its_own_record_and_hand_edits_are_refused() {
    let r = Repo::new("r4-r12-records");
    r.add_task("T-001", "low", "self_verified", &["crates/a"], &[]);
    r.add_task("T-002", "low", "self_verified", &["RED"], &[]);
    r.add_task("T-003", "low", "self_verified", &["crates/a"], &[]);
    r.add_task("T-004", "low", "self_verified", &["crates/a"], &[]);

    // A merged run: the merged head, then one record commit on top of it.
    r.commit_wt("T-001", "green", &[("crates/a/src/lib.rs", Some(LIB_V2))]);
    let (o, rec) = record_raw(&r, "T-001", &[]);
    assert_eq!(rec["result"], "merged", "record: {rec}\n{}", o.stderr);
    assert_eq!(
        tip(&r),
        (
            "queue: record T-001 merged".to_owned(),
            "metrics/queue.jsonl".to_owned()
        )
    );
    assert_eq!(
        git(&r.root, &["rev-parse", "main^"]),
        rec["head_sha"].as_str().unwrap_or_default()
    );
    assert_eq!(status(&r.root), "", "main's working tree must be clean");
    assert_eq!(committed_last_record(&r), rec);

    // A gate_failed run: only a record commit reaches main.
    let before = r.main_sha();
    r.commit_wt("T-002", "red", &[("RED", Some("red\n"))]);
    let (o, rec) = record_raw(&r, "T-002", &[]);
    assert_eq!(rec["result"], "gate_failed", "record: {rec}\n{}", o.stderr);
    assert_eq!(
        tip(&r),
        (
            "queue: record T-002 gate_failed".to_owned(),
            "metrics/queue.jsonl".to_owned()
        )
    );
    assert_eq!(git(&r.root, &["rev-parse", "main^"]), before);
    assert_eq!(status(&r.root), "", "main's working tree must be clean");
    assert_eq!(committed_last_record(&r), rec);

    // A refusal writes and commits nothing (the snapshot in `refused`).
    r.commit_wt("T-003", "outside", &[("README.md", Some("# changed\n"))]);
    refused(&r, "T-003", "touches", &["README.md"]);

    // A direct code commit on main is caught by `main-history` …
    r.commit_wt(
        "T-004",
        "green",
        &[("crates/a/src/extra.rs", Some("// x\n"))],
    );
    let good = r.main_sha();
    let bad = r.main_commit("direct code", &[("other/keep.txt", Some("direct\n"))]);
    refused(&r, "T-004", "main-history", &[&bad[..7]]);
    // … and a hand-written `merged` record for it, left uncommitted, is refused at `ledger`
    // and is not evidence.
    let fake = format!(
        "{{\"task\":\"T-999\",\"attempt\":1,\"started_at\":\"2026-11-13T00:00:00Z\",\"ended_at\":\"2026-11-13T00:00:01Z\",\"base_sha\":\"{bad}\",\"head_sha\":\"{bad}\",\"result\":\"merged\",\"failed_step\":null,\"gate_seconds\":0.0,\"gate_xtask\":\"main\",\"gate_xtask_sha\":\"{bad}\",\"regenerated\":[]}}\n"
    );
    let log = read(&r.root, "metrics/queue.jsonl") + &fake;
    write(&r.root, "metrics/queue.jsonl", &log);
    refused(&r, "T-004", "ledger", &[]);
    git(&r.root, &["checkout", "-q", "--", "metrics/queue.jsonl"]);
    refused(&r, "T-004", "main-history", &[&bad[..7]]);

    // Legitimate: without the direct commit, the task merges and its record is committed.
    r.reset_main(&good);
    let (_, rec) = record_raw(&r, "T-004", &[]);
    assert_eq!(rec["result"], "merged", "record: {rec}");
    assert_eq!(tip(&r).0, "queue: record T-004 merged");
    assert_eq!(status(&r.root), "");
}

// ---- R13 (a): a planted, future-dated fake xtask does not decide the gate -----------------

const FAKE_GATE: &str = "#!/bin/sh\n# planted fake xtask (T-005 R13 test)\nwhile [ $# -gt 0 ]; do\n  if [ \"$1\" = --report ]; then printf '%s' '{\"passed\": true, \"failed_step\": null, \"seconds\": 0.0, \"steps\": []}' > \"$2\"; fi\n  shift\ndone\necho 'planted gate: PASSED'\nexit 0\n";

/// Replace every built `xtask` executable under `dir` (an ELF whose name contains `xtask`
/// and has no extension) with a fake green gate dated one day in the future. Returns them.
fn plant_future_fakes(dir: &Path) -> Vec<PathBuf> {
    use std::os::unix::fs::PermissionsExt as _;
    let future = std::time::SystemTime::now() + std::time::Duration::from_secs(86_400);
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            let Ok(m) = std::fs::symlink_metadata(&p) else {
                continue;
            };
            if m.is_dir() {
                stack.push(p);
                continue;
            }
            let name = e.file_name().to_string_lossy().into_owned();
            if !m.is_file() || !name.contains("xtask") || name.contains('.') {
                continue;
            }
            if !std::fs::read(&p)
                .unwrap_or_default()
                .starts_with(b"\x7fELF")
            {
                continue;
            }
            let ok = std::fs::write(&p, FAKE_GATE).is_ok()
                && std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).is_ok()
                && std::fs::File::options()
                    .write(true)
                    .open(&p)
                    .and_then(|f| f.set_modified(future))
                    .is_ok();
            assert!(ok, "could not plant {}", p.display());
            out.push(p);
        }
    }
    out
}

#[test]
fn planted_future_dated_xtask_does_not_decide_the_gate() {
    let r = Repo::new("r4-r13-plant");
    let s = r.root.parent().unwrap().to_path_buf();
    // Every per-user location a cache could live in is redirected into this scratch directory
    // (cargo and rustup keep their real homes).
    let home_env = std::env::var("HOME").unwrap_or_default();
    let cargo_home = std::env::var("CARGO_HOME").unwrap_or_else(|_| format!("{home_env}/.cargo"));
    let rustup_home =
        std::env::var("RUSTUP_HOME").unwrap_or_else(|_| format!("{home_env}/.rustup"));
    let mut env: Env = vec![("CARGO_HOME", cargo_home), ("RUSTUP_HOME", rustup_home)];
    for (k, sub) in [
        ("HOME", "home"),
        ("TMPDIR", "tmp"),
        ("XDG_CACHE_HOME", "cache"),
    ] {
        let d = s.join(sub);
        std::fs::create_dir_all(&d).unwrap();
        env.push((k, d.to_string_lossy().into_owned()));
    }
    r.add_task("T-001", "low", "self_verified", &["crates/a"], &[]);
    r.add_task("T-002", "low", "self_verified", &["RED"], &[]);
    r.add_task("T-003", "low", "self_verified", &["crates/a"], &[]);
    r.commit_wt("T-001", "green", &[("crates/a/src/lib.rs", Some(LIB_V2))]);
    merged(&r, "T-001", &env);

    r.commit_wt("T-002", "red", &[("RED", Some("red\n"))]);
    let wt2 = r.wt("T-002");
    let o = run(Command::new("cargo")
        .args(["build", "--quiet", "--offline", "--package", "xtask"])
        .current_dir(&wt2)
        .env("CARGO_TARGET_DIR", wt2.join("target"))
        .env_remove("RUSTUP_TOOLCHAIN"));
    assert_eq!(o.code, 0, "prebuild: {}", o.stderr);
    // Twice: after a merged run, and again after a gate_failed run.
    for round in 1..=2 {
        let planted = plant_future_fakes(&s);
        assert!(!planted.is_empty(), "round {round}: nothing to plant over");
        let before = r.main_sha();
        let (o, rec) = record(&r, "T-002", &env);
        assert_eq!(
            (
                rec["result"].as_str(),
                rec["failed_step"].as_str(),
                rec["gate_xtask"].as_str()
            ),
            (Some("gate_failed"), Some("red"), Some("main")),
            "round {round}: a planted xtask decided the gate ({} planted: {planted:?}): record {rec}\nstdout:\n{}\nstderr:\n{}",
            planted.len(),
            o.stdout,
            o.stderr
        );
        assert_eq!(rec["gate_xtask_sha"], rec["base_sha"], "record: {rec}");
        assert!(
            !format!("{}{}", o.stdout, o.stderr).contains("planted gate: PASSED"),
            "round {round}: a planted binary ran"
        );
        r.assert_only_records_since(&before, "main before the run");
    }
    // Legitimate: a green task still merges after planting.
    r.commit_wt(
        "T-003",
        "green",
        &[("crates/a/src/extra.rs", Some("// x\n"))],
    );
    plant_future_fakes(&s);
    let (o, rec) = merged(&r, "T-003", &env);
    assert_eq!(rec["gate_xtask_sha"], rec["base_sha"], "record: {rec}");
    assert!(!format!("{}{}", o.stdout, o.stderr).contains("planted gate: PASSED"));
}

// ---- R13 (b): the lock version rule is applied to the rebased tree ------------------------

#[test]
fn lock_rule_applies_to_the_rebased_tree() {
    let r = Repo::new("r4-r13-rebased-lock");
    add_registry_dep_on_main(&r);
    r.add_task("T-000", "low", "self_verified", &["items"], &[]);
    r.add_task("T-030", "high", "reviewed", &["crates/a", "items"], &[]);
    r.add_task("T-031", "high", "reviewed", &["crates/a", "items"], &[]);
    let item = |name: &'static str, body: &'static str| -> Vec<(String, String)> {
        vec![
            (format!("items/{name}"), format!("{body}\n")),
            ("docs/scorecard.md".to_owned(), toy_summary(&[(name, body)])),
        ]
    };
    let commit = |id: &str, extra: &[(&str, &str)], files: Vec<(String, String)>| -> String {
        let mut v: Vec<(&str, Option<&str>)> = files
            .iter()
            .map(|(p, c)| (p.as_str(), Some(c.as_str())))
            .collect();
        for (p, c) in extra {
            v.push((p, Some(c)));
        }
        if id == "T-000" {
            r.commit_wt(id, "item", &v)
        } else {
            evidence(&r, id, "high", &v)
        }
    };
    let req = |v: &str| format!("{A_TOML}\n[dependencies]\nregdep = \"{v}\"\n");
    // T-030 raises the requirement to "1.1" but leaves its lock at 1.0.0: on its branch the
    // lock changes nothing. T-031 narrows it to "1.0", still satisfied by 1.0.0.
    let req30 = req("1.1");
    let req31 = req("1.0");
    commit("T-000", &[], item("a.toml", "alpha"));
    commit(
        "T-030",
        &[("crates/a/Cargo.toml", req30.as_str())],
        item("x.toml", "x"),
    );
    commit(
        "T-031",
        &[("crates/a/Cargo.toml", req31.as_str())],
        item("y.toml", "y"),
    );
    for id in ["T-030", "T-031"] {
        assert_eq!(
            lock_version(&read(&r.wt(id), "Cargo.lock"), "regdep").as_deref(),
            Some("1.0.0")
        );
        decide(&r, &format!("Approve {id} for merge."), "toy");
    }
    // Main moves: T-000 changes the generated scorecard, so both later rebases stop on a
    // generated-only conflict and the queue regenerates (Cargo.lock included).
    merged(&r, "T-000", &[]);

    // After rebasing and regenerating, T-030's lock has regdep 1.1.0: refused at
    // `dependencies`, nothing merged, branch and worktree untouched.
    let main_before = r.main_sha();
    let head_before = r.head("T-030");
    let records = r.queue_lines().len();
    let o = queue_env(&r, "T-030", &[]);
    let refusal = o
        .stderr
        .lines()
        .any(|l| l.starts_with("queue: refused: dependencies: "));
    let rec = r.queue_lines().get(records).cloned();
    let recorded = rec.as_ref().is_some_and(|v| {
        v["result"] != "merged"
            && v["failed_step"]
                .as_str()
                .is_some_and(|f| f.contains("dependencies"))
    });
    assert!(
        o.code != 0 && (refusal || recorded),
        "T-030: the rebased lock bumps regdep; expected `dependencies` (exit {}, record {rec:?})\nstdout:\n{}\nstderr:\n{}",
        o.code,
        o.stdout,
        o.stderr
    );
    r.assert_only_records_since(&main_before, "main before the run");
    assert_eq!(
        lock_version(&show(&r.root, "main", "Cargo.lock"), "regdep").as_deref(),
        Some("1.0.0")
    );
    assert_eq!(r.head("T-030"), head_before, "task branch moved");
    assert_eq!(status(&r.wt("T-030")), "", "task worktree not clean");
    commit_ledger(&r);

    // Legitimate: the same shape with a requirement the locked version satisfies merges.
    let (_, rec) = merged(&r, "T-031", &[]);
    assert!(
        rec["regenerated"]
            .as_array()
            .is_some_and(|a| a.iter().any(|p| p == "docs/scorecard.md")),
        "record: {rec}"
    );
    assert_eq!(
        lock_version(&show(&r.root, "main", "Cargo.lock"), "regdep").as_deref(),
        Some("1.0.0")
    );
    assert_eq!(read(&r.root, "crates/a/Cargo.toml"), req31);
}
