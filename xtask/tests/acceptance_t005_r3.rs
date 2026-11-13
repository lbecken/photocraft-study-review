//! Acceptance tests for T-005 round 3: the director's rulings R1–R9 of decision D-026, written
//! from the task file on `main` alone. Each ruling gets at least one attempt at the bad thing
//! (which must be refused or made harmless) and the legitimate case next to it (which must still
//! work), so a queue that simply refuses everything fails here too.
//!
//! Every test drives the `xtask` binary under test against throwaway toy repositories built by
//! the shared fixture (`acceptance_t005_support.rs`) under `CARGO_TARGET_TMPDIR`. Because R6
//! reads evidence only from main's committed tree, these tests commit join records, approval
//! rows and every `metrics/` change on main (director ledger commits) before each queue run.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

#[path = "acceptance_t005_support.rs"]
mod shared;
use shared::support::*;

/// File changes: path → new content, or `None` to delete.
type Files<'a> = &'a [(&'a str, Option<&'a str>)];
/// Extra environment for a queue run.
type Env = Vec<(&'static str, String)>;

const RUN_IN: &str = "KILN_XTASK_RUN_IN";

// ---- helpers ------------------------------------------------------------------------------

fn status_of(dir: &Path, path: &str) -> String {
    let o = run(Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=all", "--", path])
        .current_dir(dir));
    assert_eq!(o.code, 0, "git status: {}", o.stderr);
    o.stdout
}

/// Commit every pending change under `metrics/` on main (a director ledger commit).
fn commit_ledger(r: &Repo) {
    if !status_of(&r.root, "metrics").is_empty() {
        git(&r.root, &["add", "-A", "--", "metrics"]);
        git(&r.root, &["commit", "-q", "-m", "director: ledger"]);
    }
}

/// `xtask queue <id>` in the primary checkout, as from a director shell, plus `env`.
fn queue_env(r: &Repo, id: &str, env: &[(&'static str, String)]) -> Out {
    let mut c = Command::new(XTASK);
    c.args(["queue", id]).current_dir(&r.root);
    if !env.iter().any(|(k, _)| *k == RUN_IN) {
        c.env_remove(RUN_IN);
    }
    for (k, v) in env {
        c.env(k, v);
    }
    run(&mut c)
}

/// Run the queue, assert it appended exactly one record for `id`, return output and record.
/// The new `metrics/` lines are then committed on main.
fn record(r: &Repo, id: &str, env: &[(&'static str, String)]) -> (Out, Value) {
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
    commit_ledger(r);
    (o, rec)
}

/// Queue `id`, assert `merged` and that main is the merged head; return output and record.
fn merged(r: &Repo, id: &str, env: &[(&'static str, String)]) -> (Out, Value) {
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
    assert_eq!(
        (rec["task"].as_str(), rec["result"].as_str()),
        (Some(id), Some("merged")),
        "queue {id}: record {rec}\nstdout:\n{}\nstderr:\n{}",
        o.stdout,
        o.stderr
    );
    r.assert_only_records_since(rec["head_sha"].as_str().unwrap_or_default(), "merged head");
    assert_eq!(o.code, 0, "merged but exit {}: {}", o.code, o.stderr);
    commit_ledger(r);
    (o, rec)
}

/// Queue `id`; assert a refusal at `check` (exit 1, exactly one `queue: refused: <check>: `
/// stderr line whose reason contains every one of `names`) and that nothing changed (AC-9).
fn refused(
    r: &Repo,
    id: &str,
    check: &str,
    names: &[&str],
    env: &[(&'static str, String)],
) -> String {
    let before = r.snap(id);
    let o = queue_env(r, id, env);
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

/// Append a `joined` record for `sha` to `metrics/tasks.jsonl` and commit it on main.
fn commit_join(r: &Repo, id: &str, sha: &str) {
    let mut t = std::fs::read_to_string(r.root.join("metrics/tasks.jsonl")).unwrap_or_default();
    t.push_str(&join_rec(id, "joined", sha));
    t.push('\n');
    r.main_commit(
        "director: join record",
        &[("metrics/tasks.jsonl", Some(&t))],
    );
}

/// Medium/high evidence, all committed: code commit `h1` on the branch, a committed `joined`
/// record for it on main, and an approving round-1 review (full sha) on the branch.
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

/// Append a decisions-table row with `decision` / `reasoning` to main's `docs/decisions.md`.
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

/// `git show <rev>:<path>`.
fn show(dir: &Path, rev: &str, path: &str) -> String {
    let o = run(Command::new("git")
        .args(["show", &format!("{rev}:{path}")])
        .current_dir(dir));
    assert_eq!(o.code, 0, "git show {rev}:{path}: {}", o.stderr);
    o.stdout
}

/// Is a rebase (or other sequencer operation) left in progress in `wt`?
fn operation_in_progress(wt: &Path) -> bool {
    [
        "rebase-merge",
        "rebase-apply",
        "CHERRY_PICK_HEAD",
        "MERGE_HEAD",
    ]
    .iter()
    .any(|p| {
        let gp = git(wt, &["rev-parse", "--git-path", p]);
        let gp = if Path::new(&gp).is_absolute() {
            PathBuf::from(gp)
        } else {
            wt.join(gp)
        };
        gp.exists()
    })
}

const ROOT_TOML: &str = "[workspace]\nresolver = \"2\"\nmembers = [\"xtask\", \"crates/a\"]\n";
const A_TOML: &str = "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";
const DEP_TOML: &str = "[package]\nname = \"dep\"\nversion = \"1.0.0\"\nedition = \"2021\"\n";
const DEP_LINE: &str = "dep = { path = \"../../vendor/dep\" }\n";

/// Main gains a path package `dep` 1.0.0 (in `vendor/dep`) that crate `a` depends on, with the
/// lock regenerated. Done before any merge, so `main-history` passes trivially.
fn add_dep_on_main(r: &Repo) {
    write(&r.root, "vendor/dep/Cargo.toml", DEP_TOML);
    write(
        &r.root,
        "vendor/dep/src/lib.rs",
        "/// Toy.\npub fn d() -> u32 {\n    1\n}\n",
    );
    write(
        &r.root,
        "crates/a/Cargo.toml",
        &format!("{A_TOML}\n[dependencies]\n{DEP_LINE}"),
    );
    cargo_in(&r.root, &["generate-lockfile", "--offline"]);
    assert!(read(&r.root, "Cargo.lock").contains("name = \"dep\"\nversion = \"1.0.0\""));
    git(&r.root, &["add", "-A"]);
    git(&r.root, &["commit", "-q", "-m", "toy: path dependency"]);
}

/// Version of package `name` in a lock file's text.
fn lock_version(lock: &str, name: &str) -> Option<String> {
    let key = format!("name = \"{name}\"\nversion = \"");
    let i = lock.find(&key)? + key.len();
    Some(lock[i..].split('"').next()?.to_owned())
}

// ---- R1: the gate binary is rebuilt from base_sha every run -------------------------------

const FAKE_GATE: &str = "#!/bin/sh\n# tampered copy of xtask (T-005 R1 test)\nwhile [ $# -gt 0 ]; do\n  if [ \"$1\" = --report ]; then printf '%s' '{\"passed\": true, \"failed_step\": null, \"seconds\": 0.0, \"steps\": []}' > \"$2\"; fi\n  shift\ndone\necho 'tampered gate: PASSED'\nexit 0\n";

/// Overwrite every built `xtask` executable (ELF named `xtask` or `xtask-<hash>`) under `dir`
/// with a fake that reports a green gate; its mtime becomes newer than any source, so cargo
/// considers it fresh. Returns the files replaced.
fn tamper_xtask_binaries(dir: &Path) -> Vec<PathBuf> {
    use std::os::unix::fs::PermissionsExt as _;
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
            if !m.is_file() {
                continue;
            }
            let name = e.file_name().to_string_lossy().into_owned();
            if !(name == "xtask" || (name.starts_with("xtask-") && !name.contains('.'))) {
                continue;
            }
            let head = std::fs::read(&p).unwrap_or_default();
            if !head.starts_with(b"\x7fELF") {
                continue;
            }
            let ok = std::fs::write(&p, FAKE_GATE).is_ok()
                && std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).is_ok();
            assert!(ok, "could not tamper with {}", p.display());
            out.push(p);
        }
    }
    out
}

#[test]
fn tampered_cached_gate_binary_does_not_decide_the_gate() {
    let r = Repo::new("r3-r1-tamper");
    let scratch_dir = r.root.parent().unwrap().to_path_buf();
    // Keep any cache the queue makes in temporary or cache directories inside this scratch
    // directory, so the tampering below reaches it too.
    let tmp = scratch_dir.join("tmp");
    let cache = scratch_dir.join("cache");
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::create_dir_all(&cache).unwrap();
    let env: Env = vec![
        ("TMPDIR", tmp.to_string_lossy().into_owned()),
        ("XDG_CACHE_HOME", cache.to_string_lossy().into_owned()),
    ];
    r.add_task("T-001", "low", "self_verified", &["crates/a"], &[]);
    r.add_task("T-002", "low", "self_verified", &["RED"], &[]);
    r.add_task("T-003", "low", "self_verified", &["crates/a"], &[]);
    // A first, green run builds (and possibly caches) main's gate binary.
    r.commit_wt("T-001", "green", &[("crates/a/src/lib.rs", Some(LIB_V2))]);
    let (_, rec) = merged(&r, "T-001", &env);
    assert_eq!(rec["gate_xtask"], "main", "record: {rec}");

    // T-002 adds RED, which main's gate refuses. Its worktree also holds a built xtask.
    r.commit_wt("T-002", "add RED", &[("RED", Some("red\n"))]);
    let wt2 = r.wt("T-002");
    let o = run(Command::new("cargo")
        .args(["build", "--quiet", "--offline", "--package", "xtask"])
        .current_dir(&wt2)
        .env("CARGO_TARGET_DIR", wt2.join("target"))
        .env_remove("RUSTUP_TOOLCHAIN"));
    assert_eq!(o.code, 0, "prebuild: {}", o.stderr);
    // Something (e.g. a build script in an earlier gate run) overwrote every cached xtask.
    let tampered = tamper_xtask_binaries(&scratch_dir);
    assert!(!tampered.is_empty(), "nothing to tamper with");
    let main_before = r.main_sha();
    let (o, rec) = record(&r, "T-002", &env);
    assert_eq!(
        (
            rec["result"].as_str(),
            rec["failed_step"].as_str(),
            rec["gate_xtask"].as_str()
        ),
        (Some("gate_failed"), Some("red"), Some("main")),
        "a tampered cached xtask decided the gate: record {rec}\nstdout:\n{}\nstderr:\n{}",
        o.stdout,
        o.stderr
    );
    assert_eq!(rec["gate_xtask_sha"], rec["base_sha"], "record: {rec}");
    assert_eq!(
        show(&r.root, "main", "crates/a/src/lib.rs"),
        LIB_V2,
        "main content"
    );
    assert!(
        !r.root.join("RED").exists(),
        "RED reached the primary checkout"
    );
    assert_eq!(
        git(
            &r.root,
            &["rev-list", "--count", &format!("{main_before}..main")]
        ),
        "1",
        "main may only gain the ledger commit"
    );

    // Legitimate: after another tampering, a green task still merges, gated by main's xtask.
    r.commit_wt(
        "T-003",
        "green",
        &[("crates/a/src/extra.rs", Some("// extra\n"))],
    );
    tamper_xtask_binaries(&scratch_dir);
    let (_, rec) = merged(&r, "T-003", &env);
    assert_eq!(rec["gate_xtask"], "main", "record: {rec}");
    assert_eq!(rec["gate_xtask_sha"], rec["base_sha"], "record: {rec}");
}

// ---- R2: reviewed_sha must be a full ancestor id; only evidence commits after it ----------

#[test]
fn review_sha_must_be_a_full_ancestor_and_only_evidence_may_follow() {
    let r = Repo::new("r3-r2-review");
    let wt = r.add_task("T-001", "medium", "reviewed", &["crates/a"], &[]);
    let fork = git(&wt, &["merge-base", "main", "HEAD"]);
    let h1 = r.commit_wt("T-001", "impl", &[("crates/a/src/lib.rs", Some(LIB_V2))]);
    commit_join(&r, "T-001", &h1);
    let rv = |sha: &str| review_md("T-001", 1, "medium", "approve", sha);
    let review_path = "reviews/T-001-r1.md";

    // A commit on an unrelated branch with exactly the reviewed tree, but not an ancestor.
    let unrelated = git(
        &r.root,
        &[
            "commit-tree",
            &format!("{h1}^{{tree}}"),
            "-p",
            &fork,
            "-m",
            "same tree, other branch",
        ],
    );
    git(&r.root, &["branch", "other/unrelated", &unrelated]);
    let side = r.side_commit();
    for (what, sha) in [
        ("unrelated branch", unrelated.clone()),
        ("abbreviated id (12)", h1[..12].to_owned()),
        ("abbreviated id (7)", h1[..7].to_owned()),
        ("symbolic ref", "HEAD".to_owned()),
        ("branch name", "task/T-001".to_owned()),
        ("non-ancestor", side.clone()),
        ("ancestor before the code", fork.clone()),
    ] {
        r.commit_wt("T-001", what, &[(review_path, Some(&rv(&sha)))]);
        refused(&r, "T-001", "review", &[], &[]);
        r.reset_wt("T-001", &h1);
    }

    // Code committed after the review, alone or mixed into a review commit.
    r.commit_wt("T-001", "review", &[(review_path, Some(&rv(&h1)))]);
    let reviewed = r.head("T-001");
    for (what, files) in [
        (
            "code after review",
            vec![("crates/a/src/extra.rs", Some("// extra\n"))],
        ),
        (
            "code mixed into a review addendum",
            vec![
                (review_path, Some("addendum\n")),
                ("crates/a/src/lib.rs", Some(LIB_V1)),
            ],
        ),
        (
            // Under `touches`, but neither an acceptance nor a guard test file.
            "non-acceptance test file after review",
            vec![("crates/a/tests/helper.rs", Some("// helper\n"))],
        ),
        (
            "another task's acceptance file after review",
            vec![("crates/a/tests/acceptance_t002.rs", Some("// other\n"))],
        ),
        (
            "another task's support file after review",
            vec![(
                "crates/a/tests/acceptance_t002_support.rs",
                Some("// other\n"),
            )],
        ),
    ] {
        let files: Vec<(&str, Option<&str>)> = files;
        r.commit_wt("T-001", what, &files);
        refused(&r, "T-001", "review", &[], &[]);
        r.reset_wt("T-001", &reviewed);
    }

    // Legitimate: after reviewed_sha only review, task-log, generated and acceptance/guard
    // test-file commits.
    let task_file = "tasks/T-001-toy.md";
    let log = read(&wt, task_file).replace(
        "<!-- end implementer log -->",
        "- 2026-11-13 · R-903 · reviewed · did: note\n<!-- end implementer log -->",
    );
    r.commit_wt(
        "T-001",
        "review addendum",
        &[(review_path, Some(&(rv(&h1) + "\n## Note\nAddendum.\n")))],
    );
    r.commit_wt("T-001", "task log", &[(task_file, Some(&log))]);
    r.commit_wt(
        "T-001",
        "regenerate",
        &[(
            "docs/scorecard.md",
            Some(&toy_summary(&[("z.toml", "zeta")])),
        )],
    );
    r.commit_wt(
        "T-001",
        "acceptance and guard tests",
        &[
            ("crates/a/tests/acceptance_t001_more.rs", Some("// acc\n")),
            ("crates/a/tests/guard_t001.rs", Some("// guard\n")),
            (
                "crates/a/tests/acceptance_t001_support.rs",
                Some("// support\n"),
            ),
        ],
    );
    merged(&r, "T-001", &[]);
    assert_eq!(read(&r.root, "crates/a/src/lib.rs"), LIB_V2);
}

// ---- R3: gate paths need tier high and a `gate paths` approval ----------------------------

const ROOT_WITH_TABLES: &str = "[workspace]\nresolver = \"2\"\nmembers = [\"xtask\", \"crates/a\"]\n\n[workspace.lints.rust]\nunsafe_code = \"forbid\"\n\n[profile.dev]\nopt-level = 0\n";

fn root_with_tables_on_main(r: &Repo) {
    assert!(read(&r.root, "Cargo.toml").starts_with(ROOT_TOML));
    r.main_commit(
        "toy: lint and profile tables",
        &[("Cargo.toml", Some(ROOT_WITH_TABLES))],
    );
}

#[test]
fn gate_paths_need_tier_high_and_are_named() {
    let r = Repo::new("r3-r3-gate-low");
    root_with_tables_on_main(&r);
    let touches = [
        "xtask",
        ".cargo",
        "rust-toolchain.toml",
        "clippy.toml",
        "rustfmt.toml",
        ".github",
        "crates/a",
        "Cargo.toml",
    ];
    r.add_task("T-001", "low", "self_verified", &touches, &[]);
    let base = r.head("T-001");
    let xtask_src = format!("{TOY_XTASK_MAIN}// harmless tooling note\n");
    let toolchain = read(&r.root, "rust-toolchain.toml") + "# pinned\n";
    let cargo_cfg =
        read(&r.root, ".cargo/config.toml") + "\n[target.'cfg(all())']\nrunner = \"env\"\n";
    let metadata = format!("{ROOT_WITH_TABLES}\n[workspace.metadata.kiln]\nnote = \"x\"\n");
    let lints = ROOT_WITH_TABLES.replace("\"forbid\"", "\"deny\"");
    let profile = ROOT_WITH_TABLES.replace("opt-level = 0", "opt-level = 1");
    let release = format!("{ROOT_WITH_TABLES}\n[profile.release]\nlto = true\n");
    let cases: Vec<(&str, String)> = vec![
        ("xtask/src/main.rs", xtask_src),
        (".cargo/config.toml", cargo_cfg),
        ("rust-toolchain.toml", toolchain),
        ("clippy.toml", "too-many-arguments-threshold = 9\n".into()),
        ("rustfmt.toml", "max_width = 100\n".into()),
        (".github/workflows/ci.yml", "name: ci\n".into()),
        ("crates/a/build.rs", "fn main() {}\n".into()),
        ("Cargo.toml", metadata),
        ("Cargo.toml", lints),
        ("Cargo.toml", profile),
        ("Cargo.toml", release),
    ];
    for (path, content) in &cases {
        r.commit_wt("T-001", path, &[(path, Some(content))]);
        refused(&r, "T-001", "gate-path", &[path], &[]);
        r.reset_wt("T-001", &base);
    }
    // Medium is not enough either.
    r.add_task("T-002", "medium", "reviewed", &["clippy.toml"], &[]);
    evidence(
        &r,
        "T-002",
        "medium",
        &[("clippy.toml", Some("too-many-arguments-threshold = 9\n"))],
    );
    refused(&r, "T-002", "gate-path", &["clippy.toml"], &[]);

    // Legitimate: an ordinary root Cargo.toml change that leaves the lint, metadata and profile
    // tables alone is not a gate-path change.
    let ordinary = ROOT_WITH_TABLES.replace(
        "resolver = \"2\"\n",
        "resolver = \"2\"\n# ordinary comment\n",
    ) + "\n[workspace.package]\nlicense = \"MIT\"\n";
    r.commit_wt(
        "T-001",
        "ordinary root manifest change",
        &[("Cargo.toml", Some(&ordinary))],
    );
    let (_, rec) = merged(&r, "T-001", &[]);
    assert_eq!(rec["gate_xtask"], "main", "record: {rec}");
}

#[test]
fn gate_paths_on_a_high_task_need_a_gate_paths_approval() {
    let r = Repo::new("r3-r3-gate-high");
    root_with_tables_on_main(&r);
    r.add_task("T-010", "high", "reviewed", &["xtask"], &[]);
    r.add_task("T-011", "high", "reviewed", &["Cargo.toml"], &[]);
    let xtask_src = format!("{TOY_XTASK_MAIN}// harmless tooling note\n");
    evidence(
        &r,
        "T-010",
        "high",
        &[("xtask/src/main.rs", Some(&xtask_src))],
    );
    let profile = ROOT_WITH_TABLES.replace("opt-level = 0", "opt-level = 1");
    evidence(&r, "T-011", "high", &[("Cargo.toml", Some(&profile))]);

    // No approval row at all, an approval without `gate paths`, and `gate paths` only outside
    // the Decision cell.
    refused(&r, "T-010", "approval", &[], &[]);
    decide(&r, "Approve T-010 for merge.", "toy");
    refused(&r, "T-010", "approval", &[], &[]);
    decide(&r, "Approve T-010 for merge.", "it changes gate paths");
    refused(&r, "T-010", "approval", &[], &[]);
    decide(&r, "Approve T-011 for merge.", "toy");
    refused(&r, "T-011", "approval", &[], &[]);

    // Legitimate: the approval names gate paths; both gates run; the paths are printed.
    decide(&r, "Approve T-010 including its gate paths.", "toy");
    let (o, rec) = merged(&r, "T-010", &[]);
    assert_eq!(rec["gate_xtask"], "main+task", "record: {rec}");
    assert!(
        format!("{}{}", o.stdout, o.stderr).contains("xtask/src/main.rs"),
        "the queue must print the gate paths changed:\n{}\n{}",
        o.stdout,
        o.stderr
    );
    decide(&r, "Approve T-011: gate paths (dev profile).", "toy");
    let (o, rec) = merged(&r, "T-011", &[]);
    assert_eq!(rec["gate_xtask"], "main+task", "record: {rec}");
    assert!(
        format!("{}{}", o.stdout, o.stderr).contains("Cargo.toml"),
        "the queue must print the gate paths changed:\n{}\n{}",
        o.stdout,
        o.stderr
    );
    assert_eq!(read(&r.root, "Cargo.toml"), profile);
}

// ---- R4: dependency changes need tier high; the lock version rule always holds ------------

#[test]
fn dependency_and_lock_changes_need_tier_high() {
    let r = Repo::new("r3-r4-deps");
    add_dep_on_main(&r);
    let main_lock = read(&r.root, "Cargo.lock");
    r.add_task(
        "T-001",
        "low",
        "self_verified",
        &["crates/a", "Cargo.toml"],
        &[],
    );
    let base = r.head("T-001");
    let with = |extra: &str| format!("{A_TOML}\n[dependencies]\n{DEP_LINE}{extra}");
    let a_version_req = format!(
        "{A_TOML}\n[dependencies]\ndep = {{ path = \"../../vendor/dep\", version = \"1.0\" }}\n"
    );
    let a_dev = with(&format!("\n[dev-dependencies]\n{DEP_LINE}"));
    let a_build = with(&format!("\n[build-dependencies]\n{DEP_LINE}"));
    let a_target = with(&format!("\n[target.'cfg(unix)'.dependencies]\n{DEP_LINE}"));
    let root_deps =
        format!("{ROOT_TOML}\n[workspace.dependencies]\ndep = {{ path = \"vendor/dep\" }}\n");
    let lock_edit = main_lock.replace(
        "name = \"dep\"\nversion = \"1.0.0\"",
        "name = \"dep\"\nversion = \"1.0.1\"",
    );
    assert_ne!(lock_edit, main_lock);
    let cases: Vec<(&str, String)> = vec![
        ("crates/a/Cargo.toml", a_version_req),
        ("crates/a/Cargo.toml", a_dev),
        ("crates/a/Cargo.toml", a_build),
        ("crates/a/Cargo.toml", a_target),
        ("Cargo.toml", root_deps),
        ("Cargo.lock", lock_edit.clone()),
    ];
    for (path, content) in &cases {
        r.commit_wt("T-001", path, &[(path, Some(content))]);
        refused(&r, "T-001", "dependencies", &[], &[]);
        r.reset_wt("T-001", &base);
    }
    // Medium is not enough either.
    r.add_task("T-002", "medium", "reviewed", &["crates/a"], &[]);
    evidence(
        &r,
        "T-002",
        "medium",
        &[(
            "crates/a/Cargo.toml",
            Some(&with(&format!("\n[dev-dependencies]\n{DEP_LINE}"))),
        )],
    );
    refused(&r, "T-002", "dependencies", &[], &[]);

    // Legitimate: a low task's manifest change outside the dependency tables merges.
    let described = A_TOML.replace(
        "edition = \"2021\"\n",
        "edition = \"2021\"\ndescription = \"toy\"\n",
    );
    r.commit_wt(
        "T-001",
        "describe the package",
        &[(
            "crates/a/Cargo.toml",
            Some(&format!("{described}\n[dependencies]\n{DEP_LINE}")),
        )],
    );
    merged(&r, "T-001", &[]);
    // Legitimate: an approved high task adds a new package (no existing version changes).
    r.add_task("T-003", "high", "reviewed", &["crates/a", "vendor"], &[]);
    let wt3 = r.wt("T-003");
    write(
        &wt3,
        "vendor/dep2/Cargo.toml",
        &DEP_TOML
            .replace("\"dep\"", "\"dep2\"")
            .replace("1.0.0", "0.3.0"),
    );
    write(
        &wt3,
        "vendor/dep2/src/lib.rs",
        "/// Toy.\npub fn d2() -> u32 {\n    2\n}\n",
    );
    let a3 = read(&wt3, "crates/a/Cargo.toml") + "dep2 = { path = \"../../vendor/dep2\" }\n";
    write(&wt3, "crates/a/Cargo.toml", &a3);
    cargo_in(&wt3, &["generate-lockfile", "--offline"]);
    let lock3 = read(&wt3, "Cargo.lock");
    evidence(&r, "T-003", "high", &[("Cargo.lock", Some(&lock3))]);
    decide(&r, "Approve T-003 for merge.", "toy");
    merged(&r, "T-003", &[]);
    let lock = read(&r.root, "Cargo.lock");
    assert_eq!(lock_version(&lock, "dep2").as_deref(), Some("0.3.0"));
    assert_eq!(lock_version(&lock, "dep").as_deref(), Some("1.0.0"));
}

// ---- R5: no side effects on any non-merged outcome ----------------------------------------

#[test]
fn non_merged_outcomes_leave_the_task_branch_where_it_was() {
    let r = Repo::new("r3-r5-side-effects");
    // Main's toy gate moves main itself when the task tree holds `FF_RACE` (a race with a
    // director commit during the gate).
    let root = r.root.to_string_lossy().into_owned();
    let race = format!(
        "if Path::new(\"FF_RACE\").exists() {{\n                let _ = std::process::Command::new(\"git\").args([\"-C\", {root:?}, \"-c\", \"user.name=Race\", \"-c\", \"user.email=race@example.invalid\", \"commit\", \"-q\", \"--allow-empty\", \"-m\", \"race: main moves during the gate\"]).status();\n            }}\n            "
    );
    assert!(TOY_XTASK_MAIN.contains(GATE_RULE));
    let raced = TOY_XTASK_MAIN.replace(GATE_RULE, &format!("{race}{GATE_RULE}"));
    r.main_commit("toy: racing gate", &[("xtask/src/main.rs", Some(&raced))]);
    r.add_task("T-001", "low", "self_verified", &["RED"], &[]);
    r.add_task(
        "T-002",
        "low",
        "self_verified",
        &["crates/a", "FF_RACE"],
        &[],
    );
    r.add_task("T-003", "low", "self_verified", &["items"], &[]);
    r.add_task("T-004", "low", "self_verified", &["crates/a"], &[]);
    r.commit_wt("T-001", "red", &[("RED", Some("red\n"))]);
    r.commit_wt(
        "T-002",
        "race",
        &[
            ("FF_RACE", Some("race\n")),
            ("crates/a/src/lib.rs", Some(LIB_V2)),
        ],
    );
    r.commit_wt(
        "T-003",
        "poison",
        &[
            ("items/poison.toml", Some("poison\n")),
            (
                "docs/scorecard.md",
                Some(&toy_summary(&[("poison.toml", "poison")])),
            ),
        ],
    );
    r.commit_wt(
        "T-004",
        "outside touches",
        &[("README.md", Some("# changed\n"))],
    );
    // Main moves after every branch was cut, so each run has something to rebase onto; the
    // scorecard change makes T-003 a generated-only conflict whose regeneration fails.
    r.main_commit(
        "toy: scorecard on main",
        &[("docs/scorecard.md", Some(&toy_summary(&[("x.toml", "x")])))],
    );
    r.main_commit("director: state", &[("STATE.md", Some("state 1\n"))]);

    for (id, result) in [
        ("T-001", "gate_failed"),
        ("T-002", "ff_failed"),
        ("T-003", "conflict"),
    ] {
        let before = r.head(id);
        let (o, rec) = record(&r, id, &[]);
        assert_eq!(
            rec["result"], result,
            "record: {rec}\n{}\n{}",
            o.stdout, o.stderr
        );
        let wt = r.wt(id);
        assert_eq!(
            r.head(id),
            before,
            "{id}: after `{result}` the task branch must point where it did"
        );
        assert_eq!(
            git(&wt, &["rev-parse", &format!("refs/heads/task/{id}")]),
            before,
            "{id}: branch ref moved"
        );
        assert_eq!(status(&wt), "", "{id}: worktree not clean after `{result}`");
        assert!(
            !operation_in_progress(&wt),
            "{id}: operation left in progress"
        );
    }
    // A refusal, too (AC-9 snapshot, after main has moved).
    refused(&r, "T-004", "touches", &["README.md"], &[]);
    assert_eq!(
        r.head("T-004"),
        git(&r.wt("T-004"), &["rev-parse", "task/T-004"])
    );
}

// ---- R6: evidence only from main's committed tree; uncommitted evidence → `ledger` --------

#[test]
fn uncommitted_evidence_does_not_count() {
    let r = Repo::new("r3-r6-ledger");
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
    // The join record exists only in the primary checkout's working copy (untracked).
    let joined = format!("{}\n", join_rec("T-001", "joined", &h1));
    write(&r.root, "metrics/tasks.jsonl", &joined);
    let reason = refused(&r, "T-001", "ledger", &[], &[]);
    assert!(
        reason.to_lowercase().contains("commit"),
        "the `ledger` refusal should hint to commit the ledger: {reason}"
    );
    commit_ledger(&r);
    // A tracked evidence file modified but not committed.
    write(
        &r.root,
        "metrics/tasks.jsonl",
        &format!("{joined}{}\n", join_rec("T-009", "joined", &h1)),
    );
    refused(&r, "T-001", "ledger", &[], &[]);
    git(&r.root, &["checkout", "-q", "--", "metrics/tasks.jsonl"]);

    // Other evidence in the working copy only: an approval row, task front matter, a review
    // file, a policy file. Each is refused at `ledger`, whatever else would happen.
    r.add_task("T-002", "high", "reviewed", &["crates/a"], &[]);
    evidence(
        &r,
        "T-002",
        "high",
        &[("crates/a/src/b.rs", Some("// b\n"))],
    );
    let decisions = read(&r.root, "docs/decisions.md");
    write(
        &r.root,
        "docs/decisions.md",
        &format!("{decisions}| D-099 | 2026-11-13 | Approve T-002 for merge. | toy | yes |\n"),
    );
    refused(&r, "T-002", "ledger", &[], &[]);
    git(&r.root, &["checkout", "-q", "--", "docs/decisions.md"]);

    // T-003 would merge as committed; its working-copy front matter differs.
    r.add_task("T-003", "low", "self_verified", &["crates/a"], &[]);
    r.commit_wt("T-003", "code", &[("crates/a/src/c.rs", Some("// c\n"))]);
    let t3 = read(&r.root, "tasks/T-003-toy.md").replace("tier: low", "tier: medium");
    write(&r.root, "tasks/T-003-toy.md", &t3);
    refused(&r, "T-003", "ledger", &[], &[]);
    git(&r.root, &["checkout", "-q", "--", "tasks/T-003-toy.md"]);
    write(
        &r.root,
        "reviews/T-003-r1.md",
        &review_md("T-003", 1, "low", "approve", &h1),
    );
    refused(&r, "T-003", "ledger", &[], &[]);
    std::fs::remove_dir_all(r.root.join("reviews")).unwrap();

    r.add_task("T-004", "low", "self_verified", &["crates/a"], &[]);
    r.commit_wt(
        "T-004",
        "outside touches",
        &[("README.md", Some("# changed\n"))],
    );
    for policy in ["xtask/generated.toml", "xtask/ledger.toml"] {
        let text = read(&r.root, policy) + "\n# edited in the working copy only\n";
        write(&r.root, policy, &text);
        refused(&r, "T-004", "ledger", &[], &[]);
        git(&r.root, &["checkout", "-q", "--", policy]);
    }

    // Legitimate: the same evidence, committed, is accepted. (The queue commits its own
    // `metrics/queue.jsonl` record, D-030 R12, which replaced D-027's exemption.)
    merged(&r, "T-001", &[]);
    r.main_commit(
        "director: approve",
        &[(
            "docs/decisions.md",
            Some(&format!(
                "{decisions}| D-100 | 2026-11-13 | Approve T-002 for merge. | toy | yes |\n"
            )),
        )],
    );
    merged(&r, "T-002", &[]);
    merged(&r, "T-003", &[]);
}

// ---- R7: an approval row is a Markdown table row outside any code block -------------------

#[test]
fn approval_rows_in_code_blocks_or_prose_do_not_count() {
    let r = Repo::new("r3-r7-approval");
    r.add_task("T-003", "high", "reviewed", &["crates/a"], &[]);
    evidence(
        &r,
        "T-003",
        "high",
        &[("crates/a/src/lib.rs", Some(LIB_V2))],
    );
    let row = "| D-002 | 2026-11-13 | Approve T-003 for merge. | toy | yes |";
    let variants = [
        format!("\n```markdown\n{row}\n```\n"),
        format!("\n~~~\n{row}\n~~~\n"),
        format!("\nExample:\n\n    {row}\n"),
        "\nApprove T-003 for merge.\n".to_owned(),
        "\nWe will Approve T-003 for merge once the gate paths are reviewed.\n".to_owned(),
        // A pipe line on its own, with no header and delimiter row, is not a table.
        format!("\nNote:\n\n{row}\n"),
        // An unclosed fence runs to the end of the file.
        format!("\n```\n{row}\n"),
    ];
    for v in &variants {
        r.main_commit(
            "director: decisions",
            &[("docs/decisions.md", Some(&format!("{DECISIONS}{v}")))],
        );
        refused(&r, "T-003", "approval", &[], &[]);
    }
    // Legitimate: a real table row after a closed code block that mentions the task.
    let ok = format!(
        "{DECISIONS}\n```text\nApprove T-003 (example only)\n```\n\n| # | Date | Decision | Reasoning | Reversible? |\n|---|---|---|---|---|\n{row}\n"
    );
    r.main_commit("director: approve", &[("docs/decisions.md", Some(&ok))]);
    merged(&r, "T-003", &[]);
}

// ---- R8: KILN_XTASK_RUN_IN lets nothing through --------------------------------------------

#[test]
fn run_in_variable_lets_nothing_through() {
    let r = Repo::new("r3-r8-run-in");
    r.add_task("T-001", "low", "self_verified", &["RED"], &[]);
    r.add_task("T-002", "low", "self_verified", &["RED", "xtask"], &[]);
    r.add_task("T-003", "low", "self_verified", &["crates/a"], &[]);
    r.commit_wt("T-001", "red", &[("RED", Some("red\n"))]);
    let weakened = TOY_XTASK_MAIN.replace(GATE_RULE, GATE_WEAKENED);
    r.commit_wt(
        "T-002",
        "red and a weakened own gate",
        &[
            ("RED", Some("red\n")),
            ("xtask/src/main.rs", Some(&weakened)),
        ],
    );
    r.commit_wt(
        "T-003",
        "outside touches",
        &[("README.md", Some("# changed\n"))],
    );
    let values = [
        r.wt("T-001").to_string_lossy().into_owned(),
        r.wt("T-002").to_string_lossy().into_owned(),
        r.root.to_string_lossy().into_owned(),
        r.root.join("xtask").to_string_lossy().into_owned(),
        "1".to_owned(),
    ];
    let main_before = r.main_sha();
    for v in &values {
        let env: Env = vec![(RUN_IN, v.clone())];
        let (o, rec) = record(&r, "T-001", &env);
        assert_eq!(
            (
                rec["result"].as_str(),
                rec["failed_step"].as_str(),
                rec["gate_xtask"].as_str()
            ),
            (Some("gate_failed"), Some("red"), Some("main")),
            "{RUN_IN}={v}: record {rec}\n{}\n{}",
            o.stdout,
            o.stderr
        );
        refused(&r, "T-002", "gate-path", &["xtask/src/main.rs"], &env);
        refused(&r, "T-003", "touches", &["README.md"], &env);
    }
    assert!(!r.root.join("RED").exists());
    assert_eq!(show(&r.root, "main", "xtask/src/main.rs"), TOY_XTASK_MAIN);
    assert_eq!(
        git(&r.root, &["diff", "--name-only", &main_before, "main"]),
        "metrics/queue.jsonl",
        "main may only gain ledger commits"
    );
}

// ---- R9: end to end — the lock version rule, and regeneration touching other paths ---------

/// Main gains a registry package `regdep` (versions 1.0.0 and 1.1.0 served offline from a
/// directory source replacing crates.io) that crate `a` requires as `"1"`, locked at 1.0.0.
/// Done before any merge, so `main-history` passes trivially.
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
    // Pin the older version (cargo picks the newest); the lock stays valid (`--locked`).
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
    let lock = read(&r.root, "Cargo.lock");
    assert_eq!(lock_version(&lock, "regdep").as_deref(), Some("1.0.0"));
    assert!(lock.contains("source = \"registry+"), "{lock}");
    git(&r.root, &["add", "-A"]);
    git(&r.root, &["commit", "-q", "-m", "toy: registry dependency"]);
}

#[test]
fn lock_version_change_is_refused_with_or_without_a_conflict() {
    let r = Repo::new("r3-r9-lock");
    add_registry_dep_on_main(&r);
    r.add_task("T-020", "high", "reviewed", &["crates/a"], &[]);
    r.add_task("T-021", "high", "reviewed", &["crates/a"], &[]);
    // T-020 bumps the existing registry package in the lock only: no conflict with main.
    let wt0 = r.wt("T-020");
    cargo_in(&wt0, &["update", "--offline", "-p", "regdep"]);
    let lock0 = read(&wt0, "Cargo.lock");
    assert_eq!(lock_version(&lock0, "regdep").as_deref(), Some("1.1.0"));
    evidence(&r, "T-020", "high", &[("Cargo.lock", Some(&lock0))]);
    decide(&r, "Approve T-020 for merge.", "toy");
    refused(&r, "T-020", "dependencies", &[], &[]);

    // Legitimate (ruling 5): an approved high task bumps a workspace member's version; the
    // lock rule compares registry and git packages only, so it merges and main's lock moves …
    let wt1 = r.wt("T-021");
    let a1 =
        read(&wt1, "crates/a/Cargo.toml").replace("version = \"0.1.0\"", "version = \"0.2.0\"");
    write(&wt1, "crates/a/Cargo.toml", &a1);
    cargo_in(&wt1, &["metadata", "--offline", "--format-version", "1"]);
    let lock1 = read(&wt1, "Cargo.lock");
    assert_eq!(lock_version(&lock1, "a").as_deref(), Some("0.2.0"));
    assert_eq!(lock_version(&lock1, "regdep").as_deref(), Some("1.0.0"));
    evidence(&r, "T-021", "high", &[("Cargo.lock", Some(&lock1))]);
    decide(&r, "Approve T-021 for merge.", "toy");
    merged(&r, "T-021", &[]);
    let lock = read(&r.root, "Cargo.lock");
    assert_eq!(lock_version(&lock, "a").as_deref(), Some("0.2.0"));
    assert_eq!(lock_version(&lock, "regdep").as_deref(), Some("1.0.0"));
    // … and T-020, whose lock now also differs from main's in the rebase, is still refused.
    refused(&r, "T-020", "dependencies", &[], &[]);
    assert_eq!(
        lock_version(&show(&r.root, "main", "Cargo.lock"), "regdep").as_deref(),
        Some("1.0.0")
    );
}

const SCORECARD_WRITE: &str =
    "std::fs::write(\"docs/scorecard.md\", summary()).expect(\"write scorecard\");";

#[test]
fn regeneration_that_changes_a_non_generated_path_is_not_merged() {
    let r = Repo::new("r3-r9-regen-leak");
    // Main's toy scorecard also rewrites a tracked, non-generated file when `items/leak.toml`
    // exists.
    assert!(TOY_XTASK_MAIN.contains(SCORECARD_WRITE));
    let leaky = TOY_XTASK_MAIN.replace(
        SCORECARD_WRITE,
        &format!("if Path::new(\"items/leak.toml\").exists() {{\n                std::fs::write(\"notes/shared.txt\", \"leaked by regeneration\\n\").expect(\"leak\");\n            }}\n            {SCORECARD_WRITE}"),
    );
    r.main_commit(
        "toy: leaky scorecard",
        &[("xtask/src/main.rs", Some(&leaky))],
    );
    for id in ["T-001", "T-002", "T-003"] {
        r.add_task(id, "low", "self_verified", &["items"], &[]);
    }
    let item = |id: &str, name: &'static str, body: &'static str| {
        r.commit_wt(
            id,
            name,
            &[
                (&format!("items/{name}"), Some(&format!("{body}\n"))),
                ("docs/scorecard.md", Some(&toy_summary(&[(name, body)]))),
            ],
        );
    };
    item("T-001", "a.toml", "alpha");
    item("T-002", "b.toml", "beta");
    item("T-003", "leak.toml", "leak");
    merged(&r, "T-001", &[]);

    // T-003 conflicts only on the generated scorecard; regenerating it rewrites notes/.
    let main_before = r.main_sha();
    let head_before = r.head("T-003");
    let records = r.queue_lines().len();
    let o = queue_env(&r, "T-003", &[]);
    assert_ne!(
        o.code, 0,
        "T-003 must not merge:\n{}\n{}",
        o.stdout, o.stderr
    );
    // Same result kind as a failed regeneration (D-026 ruling on R9): `conflict` at
    // `regenerate:<path>`.
    let lines = r.queue_lines();
    assert_eq!(
        lines.len(),
        records + 1,
        "T-003 wrote no record: {}",
        o.stderr
    );
    let rec = &lines[records];
    assert_eq!(rec["result"], "conflict", "record: {rec}");
    assert!(
        rec["failed_step"]
            .as_str()
            .is_some_and(|f| f.starts_with("regenerate:")),
        "record: {rec}"
    );
    r.assert_only_records_since(&main_before, "main before the run");
    assert_eq!(show(&r.root, "main", "notes/shared.txt"), "shared\n");
    assert_eq!(read(&r.root, "notes/shared.txt"), "shared\n");
    assert!(!r.root.join("items/leak.toml").exists());
    assert_eq!(r.head("T-003"), head_before, "task branch moved");
    assert_eq!(status(&r.wt("T-003")), "", "task worktree not clean");
    commit_ledger(&r);

    // Legitimate: a generated-only conflict whose regeneration stays on generated paths merges.
    let (_, rec) = merged(&r, "T-002", &[]);
    assert_eq!(
        rec["regenerated"],
        serde_json::json!(["docs/scorecard.md"]),
        "record: {rec}"
    );
    assert_eq!(
        read(&r.root, "docs/scorecard.md"),
        toy_summary(&[("a.toml", "alpha"), ("b.toml", "beta")])
    );
    assert_eq!(read(&r.root, "notes/shared.txt"), "shared\n");
}
