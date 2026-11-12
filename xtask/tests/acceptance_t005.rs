//! Acceptance tests for T-005 (merge queue hardening), written from the task file alone.
//!
//! Every test drives the real `xtask` binary (`CARGO_BIN_EXE_xtask`) against throwaway git
//! repositories created under `CARGO_TARGET_TMPDIR`; nothing here touches the Kiln repository
//! itself. The queue repositories are tiny cargo workspaces with a toy `xtask` package (no
//! dependencies) whose `ci --report <file>` fails exactly when a file `RED` exists, whose
//! `scorecard` writes a deterministic summary of `items/` to `docs/scorecard.md` (and fails when
//! `items/poison.toml` exists), and whose `schema --write` writes a constant `docs/commands.json`.
//!
//! The two policy files (`xtask/generated.toml`, `xtask/ledger.toml`) are new in T-005 and their
//! TOML layout is not fixed by the spec, so each toy repository receives a copy of the files this
//! package ships (read at run time from `CARGO_MANIFEST_DIR`). Only when they do not exist yet
//! (on the base branch, whose queue does not read them) a stand-in with the spec's entries is
//! used instead.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

#[cfg(test)]
mod support {
    use super::*;
    use std::process::Command;

    pub const XTASK: &str = env!("CARGO_BIN_EXE_xtask");

    /// A fresh, empty scratch directory for one test.
    pub fn scratch(name: &str) -> PathBuf {
        let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("t005-acc")
            .join(name);
        if d.exists() {
            std::fs::remove_dir_all(&d).unwrap();
        }
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Strip the environment `cargo test` hands us, so a child behaves as in a director shell:
    /// no inherited target directory or package variables, isolated git config, offline cargo.
    pub fn hygiene(c: &mut Command) -> &mut Command {
        for (k, _) in std::env::vars_os() {
            let Some(k) = k.to_str() else { continue };
            if (k.starts_with("CARGO_") && k != "CARGO_HOME") || k.starts_with("GIT_") {
                c.env_remove(k);
            }
        }
        c.env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Toy Director")
            .env("GIT_AUTHOR_EMAIL", "toy@example.invalid")
            .env("GIT_COMMITTER_NAME", "Toy Director")
            .env("GIT_COMMITTER_EMAIL", "toy@example.invalid")
            .env("CARGO_NET_OFFLINE", "true")
    }

    /// Output of a child process.
    #[derive(Debug)]
    pub struct Out {
        pub code: i32,
        pub stdout: String,
        pub stderr: String,
    }

    pub fn run(c: &mut Command) -> Out {
        let o = hygiene(c).output().unwrap();
        Out {
            code: o.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&o.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
        }
    }

    /// git in `dir`; panics on failure; returns trimmed stdout.
    pub fn git(dir: &Path, args: &[&str]) -> String {
        let o = run(Command::new("git").args(args).current_dir(dir));
        assert_eq!(o.code, 0, "git {args:?} in {}: {}", dir.display(), o.stderr);
        o.stdout.trim().to_owned()
    }

    /// Raw `git status --porcelain` (untracked included), untrimmed.
    pub fn status(dir: &Path) -> String {
        let o = run(Command::new("git")
            .args(["status", "--porcelain", "--untracked-files=all"])
            .current_dir(dir));
        assert_eq!(o.code, 0, "git status: {}", o.stderr);
        o.stdout
    }

    pub fn xtask(dir: &Path, args: &[&str]) -> Out {
        run(Command::new(XTASK).args(args).current_dir(dir))
    }

    pub fn write(dir: &Path, rel: &str, content: &str) {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    }

    pub fn read(dir: &Path, rel: &str) -> String {
        std::fs::read_to_string(dir.join(rel)).unwrap()
    }

    /// Apply `files` (Some = write, None = delete) in `dir` and commit everything.
    pub fn commit_all(dir: &Path, msg: &str, files: &[(&str, Option<&str>)]) -> String {
        for (p, c) in files {
            match c {
                Some(c) => write(dir, p, c),
                None => std::fs::remove_file(dir.join(p)).unwrap(),
            }
        }
        git(dir, &["add", "-A"]);
        git(dir, &["commit", "-q", "-m", msg]);
        git(dir, &["rev-parse", "HEAD"])
    }

    // ---- toy workspace -------------------------------------------------------------------

    pub const TOY_XTASK_TOML: &str =
        "[package]\nname = \"xtask\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n";

    pub const GATE_RULE: &str = "let red = Path::new(\"RED\").exists(); // gate rule";
    pub const GATE_WEAKENED: &str = "let red = false; // gate rule (weakened by the task)";

    pub const TOY_XTASK_MAIN: &str = r##"use std::path::Path;
use std::process::ExitCode;

const COMMANDS_JSON: &str = "{\"commands\": []}\n";

fn summary() -> String {
    let mut names: Vec<String> = match std::fs::read_dir("items") {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect(),
        Err(_) => Vec::new(),
    };
    names.sort();
    let mut out = String::from("# Scorecard (toy)\n");
    for n in names {
        let body = std::fs::read_to_string(Path::new("items").join(&n)).unwrap_or_default();
        out.push_str("- ");
        out.push_str(&n);
        out.push_str(": ");
        out.push_str(body.trim());
        out.push('\n');
    }
    out
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("ci") => {
            let report = args
                .iter()
                .position(|a| a == "--report")
                .and_then(|i| args.get(i + 1));
            let red = Path::new("RED").exists(); // gate rule
            let json = if red {
                "{\"passed\": false, \"failed_step\": \"red\", \"seconds\": 0.0, \"steps\": []}"
            } else {
                "{\"passed\": true, \"failed_step\": null, \"seconds\": 0.0, \"steps\": []}"
            };
            if let Some(r) = report {
                std::fs::write(r, json).expect("write report");
            }
            println!("toy gate: {}", if red { "FAILED at red" } else { "PASSED" });
            if red { ExitCode::FAILURE } else { ExitCode::SUCCESS }
        }
        Some("scorecard") => {
            if Path::new("items/poison.toml").exists() {
                eprintln!("toy scorecard: items/poison.toml cannot be summarised");
                return ExitCode::FAILURE;
            }
            std::fs::create_dir_all("docs").expect("docs");
            std::fs::write("docs/scorecard.md", summary()).expect("write scorecard");
            ExitCode::SUCCESS
        }
        Some("schema") => {
            std::fs::create_dir_all("docs").expect("docs");
            std::fs::write("docs/commands.json", COMMANDS_JSON).expect("write schema");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("toy xtask: unknown command {args:?}");
            ExitCode::from(2)
        }
    }
}
"##;

    pub const COMMANDS_JSON: &str = "{\"commands\": []}\n";

    /// The naive model of the toy `scorecard` command.
    pub fn toy_summary(items: &[(&str, &str)]) -> String {
        let mut v: Vec<(&str, &str)> = items.to_vec();
        v.sort();
        let mut out = String::from("# Scorecard (toy)\n");
        for (n, b) in v {
            out.push_str(&format!("- {n}: {}\n", b.trim()));
        }
        out
    }

    pub const LIB_V1: &str = "/// Toy value.\npub fn value() -> u32 {\n    1\n}\n";
    pub const LIB_V2: &str = "/// Toy value.\npub fn value() -> u32 {\n    2\n}\n";

    pub const DECISIONS: &str = "# Decisions\n\n| # | Date | Decision | Reasoning | Reversible? |\n|---|---|---|---|---|\n| D-001 | 2026-11-12 | Toy decision. | none | yes |\n";

    const FALLBACK_GENERATED: &str = "# stand-in (base only)\n[[file]]\npath = \"Cargo.lock\"\ncommand = [\"cargo\", \"update\", \"--workspace\"]\n\n[[file]]\npath = \"docs/commands.json\"\ncommand = [\"{xtask}\", \"schema\", \"--write\"]\n\n[[file]]\npath = \"docs/scorecard.md\"\ncommand = [\"{xtask}\", \"scorecard\"]\n";
    const FALLBACK_LEDGER: &str = "# stand-in (base only)\n[[path]]\npath = \"tasks/\"\nmode = \"any\"\n[[path]]\npath = \"metrics/\"\nmode = \"any\"\n[[path]]\npath = \"docs/decisions.md\"\nmode = \"any\"\n[[path]]\npath = \"docs/adr/\"\nmode = \"any\"\n[[path]]\npath = \"STATE.md\"\nmode = \"any\"\n[[path]]\npath = \"handoffs/\"\nmode = \"any\"\n[[path]]\npath = \"retrospectives/\"\nmode = \"any\"\n[[path]]\npath = \"evaluations/\"\nmode = \"any\"\n[[path]]\npath = \"methodology/\"\nmode = \"any\"\n[[path]]\npath = \"scorecard/items/\"\nmode = \"add-only\"\n";

    /// This package's shipped policy file `name`, or the base-only stand-in.
    pub fn policy(name: &str) -> String {
        let shipped = Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
        let fallback = match name {
            "generated.toml" => FALLBACK_GENERATED,
            _ => FALLBACK_LEDGER,
        };
        std::fs::read_to_string(shipped).unwrap_or_else(|_| fallback.to_owned())
    }

    pub fn task_md(id: &str, tier: &str, state: &str, touches: &[&str], items: &[&str]) -> String {
        let mut t = String::new();
        for p in touches {
            t.push_str(&format!("  - {p}\n"));
        }
        format!(
            "---\nid: {id}\ntitle: \"Toy task {id}\"\ntier: {tier}\nstate: {state}\nowner_role: director\nscorecard_items: [{}]\ndepends_on: []\ntouches:\n{t}budget_tokens: 1000\niteration: 0\nfound_by: planner\norigin_task: null\n---\n\n# {id} Toy task\n\n## Log\n### implementer\n<!-- end implementer log -->\n",
            items.join(", ")
        )
    }

    pub fn review_md(id: &str, round: u32, tier: &str, verdict: &str, reviewed: &str) -> String {
        format!(
            "---\ntask: {id}\nround: {round}\nrun: R-900\ntier: {tier}\nreviewed_sha: {reviewed}\nprevious_sha: null\nverdict: {verdict}\nfindings: {{defect: 0, spec_gap: 0, test_gap: 0, nit: 0}}\n---\n\n# Review {id} round {round}\n"
        )
    }

    pub fn join_rec(task: &str, result: &str, joined_sha: &str) -> String {
        format!(
            "{{\"kind\":\"join\",\"task\":\"{task}\",\"at\":\"2026-11-12T00:00:00Z\",\"acc_total\":1,\"acc_fail_base\":1,\"base_fail_kind\":\"fails\",\"acc_fail_first_join\":0,\"acc_defects\":null,\"impl_tests_green\":true,\"gate_green\":true,\"tests_sha\":\"{joined_sha}\",\"joined_sha\":\"{joined_sha}\",\"result\":\"{result}\"}}"
        )
    }

    /// Everything a refusal must leave untouched (AC-9).
    #[derive(Debug, PartialEq, Eq)]
    pub struct Snap {
        main: String,
        branch: Option<String>,
        root_status: String,
        wt_status: Option<String>,
        queue_jsonl: Option<Vec<u8>>,
    }

    /// A toy repository: primary checkout `root`, task worktrees under `.worktrees/`.
    pub struct Repo {
        pub root: PathBuf,
    }

    impl Repo {
        pub fn new(name: &str) -> Repo {
            let root = scratch(name).join("repo");
            let files: Vec<(&str, String)> = vec![
                (
                    "Cargo.toml",
                    "[workspace]\nresolver = \"2\"\nmembers = [\"xtask\", \"crates/a\"]\n".into(),
                ),
                (
                    ".cargo/config.toml",
                    "[alias]\nxtask = \"run --quiet --package xtask --\"\n".into(),
                ),
                (
                    "rust-toolchain.toml",
                    "[toolchain]\nchannel = \"1.97.0\"\nprofile = \"minimal\"\n".into(),
                ),
                (".gitignore", ".worktrees/\ntarget/\n".into()),
                ("xtask/Cargo.toml", TOY_XTASK_TOML.into()),
                ("xtask/src/main.rs", TOY_XTASK_MAIN.into()),
                ("xtask/generated.toml", policy("generated.toml")),
                ("xtask/ledger.toml", policy("ledger.toml")),
                (
                    "crates/a/Cargo.toml",
                    "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2021\"\n".into(),
                ),
                ("crates/a/src/lib.rs", LIB_V1.into()),
                ("docs/scorecard.md", toy_summary(&[])),
                ("docs/commands.json", COMMANDS_JSON.into()),
                ("docs/decisions.md", DECISIONS.into()),
                ("docs/notes.md", "notes\n".into()),
                ("other/keep.txt", "keep\n".into()),
                ("notes/shared.txt", "shared\n".into()),
                ("README.md", "# toy\n".into()),
                ("STATE.md", "state 0\n".into()),
                ("metrics/notes.txt", "notes 0\n".into()),
                (
                    "scorecard/items/existing.toml",
                    "id = \"existing\"\n".into(),
                ),
            ];
            for (p, c) in &files {
                write(&root, p, c);
            }
            let lock = run(Command::new("cargo")
                .args(["generate-lockfile", "--offline"])
                .current_dir(&root));
            assert_eq!(lock.code, 0, "generate-lockfile: {}", lock.stderr);
            git(&root, &["init", "-q", "-b", "main"]);
            git(&root, &["add", "-A"]);
            git(&root, &["commit", "-q", "-m", "toy base"]);
            Repo { root }
        }

        pub fn wt(&self, id: &str) -> PathBuf {
            self.root.join(".worktrees").join(id)
        }

        pub fn main_sha(&self) -> String {
            git(&self.root, &["rev-parse", "main"])
        }

        /// A director commit on main touching exactly `files`.
        pub fn main_commit(&self, msg: &str, files: &[(&str, Option<&str>)]) -> String {
            let mut paths = Vec::new();
            for (p, c) in files {
                match c {
                    Some(c) => write(&self.root, p, c),
                    None => {
                        let _ = std::fs::remove_file(self.root.join(p));
                    }
                }
                paths.push(*p);
            }
            let mut args = vec!["add", "-A", "--"];
            args.extend(paths);
            git(&self.root, &args);
            git(&self.root, &["commit", "-q", "-m", msg]);
            self.main_sha()
        }

        /// Put main back to `sha` (untracked metrics files survive).
        pub fn reset_main(&self, sha: &str) {
            git(&self.root, &["reset", "-q", "--hard", sha]);
        }

        /// Commit the task file on main, then `xtask worktree new <id> --no-seed`.
        pub fn add_task(
            &self,
            id: &str,
            tier: &str,
            state: &str,
            touches: &[&str],
            items: &[&str],
        ) -> PathBuf {
            let path = format!("tasks/{id}-toy.md");
            self.main_commit(
                &format!("director: add {id}"),
                &[(&path, Some(&task_md(id, tier, state, touches, items)))],
            );
            self.worktree(id)
        }

        pub fn worktree(&self, id: &str) -> PathBuf {
            let o = xtask(&self.root, &["worktree", "new", id, "--no-seed"]);
            assert_eq!(o.code, 0, "worktree new {id}: {}{}", o.stdout, o.stderr);
            self.wt(id)
        }

        pub fn commit_wt(&self, id: &str, msg: &str, files: &[(&str, Option<&str>)]) -> String {
            commit_all(&self.wt(id), msg, files)
        }

        pub fn reset_wt(&self, id: &str, sha: &str) {
            git(&self.wt(id), &["reset", "-q", "--hard", sha]);
        }

        pub fn head(&self, id: &str) -> String {
            git(&self.wt(id), &["rev-parse", "HEAD"])
        }

        /// A commit object that is on no branch (not an ancestor of anything).
        pub fn side_commit(&self) -> String {
            git(
                &self.root,
                &["commit-tree", "main^{tree}", "-p", "main", "-m", "side"],
            )
        }

        pub fn set_tasks_jsonl(&self, lines: &[String]) {
            let mut s = String::new();
            for l in lines {
                s.push_str(l);
                s.push('\n');
            }
            write(&self.root, "metrics/tasks.jsonl", &s);
        }

        pub fn queue_lines(&self) -> Vec<Value> {
            match std::fs::read_to_string(self.root.join("metrics/queue.jsonl")) {
                Ok(t) => t
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(|l| serde_json::from_str(l).unwrap())
                    .collect(),
                Err(_) => Vec::new(),
            }
        }

        pub fn snap(&self, id: &str) -> Snap {
            let wt = self.wt(id);
            Snap {
                main: self.main_sha(),
                branch: wt.is_dir().then(|| git(&wt, &["rev-parse", "HEAD"])),
                root_status: status(&self.root),
                wt_status: wt.is_dir().then(|| status(&wt)),
                queue_jsonl: std::fs::read(self.root.join("metrics/queue.jsonl")).ok(),
            }
        }

        pub fn queue(&self, id: &str) -> Out {
            xtask(&self.root, &["queue", id])
        }

        /// Run the queue and return the record it appended (asserting it appended one).
        pub fn queue_record(&self, id: &str) -> Value {
            let before = self.queue_lines().len();
            let o = self.queue(id);
            let lines = self.queue_lines();
            assert_eq!(
                lines.len(),
                before + 1,
                "queue {id} wrote no record (exit {}):\n{}\n{}",
                o.code,
                o.stdout,
                o.stderr
            );
            let rec = lines.last().unwrap().clone();
            assert_eq!(rec["task"], id);
            rec
        }

        /// Queue `id`, assert a merge, return the record.
        pub fn assert_merged(&self, id: &str) -> Value {
            let rec = self.queue_record(id);
            assert_eq!(rec["result"], "merged", "record: {rec}");
            assert_eq!(self.main_sha(), rec["head_sha"].as_str().unwrap());
            rec
        }

        /// Queue `id`; assert a refusal at `check` (exit 1, exactly one
        /// `queue: refused: <check>: ` stderr line, mentioning `names` if given) and that
        /// nothing changed (AC-9). Returns the reason text.
        pub fn assert_refused(&self, id: &str, check: &str, names: Option<&str>) -> String {
            let before = self.snap(id);
            let o = self.queue(id);
            let after = self.snap(id);
            let refusals: Vec<&str> = o
                .stderr
                .lines()
                .filter(|l| l.starts_with("queue: refused: "))
                .collect();
            assert_eq!(
                (o.code, refusals.len()),
                (1, 1),
                "queue {id}: expected one refusal at `{check}`; exit {}\nstdout:\n{}\nstderr:\n{}",
                o.code,
                o.stdout,
                o.stderr
            );
            let prefix = format!("queue: refused: {check}: ");
            assert!(
                refusals[0].starts_with(&prefix),
                "queue {id}: expected `{prefix}…`, got `{}`",
                refusals[0]
            );
            let reason = refusals[0][prefix.len()..].to_owned();
            if let Some(n) = names {
                assert!(
                    reason.contains(n),
                    "queue {id}: refusal `{check}` should name `{n}`: {reason}"
                );
            }
            assert_eq!(before, after, "queue {id}: refusal `{check}` changed state");
            reason
        }

        /// A medium/high task with complete evidence: code commit `h1`, a `joined` record for
        /// it and an approving round-1 review committed after it. Returns `h1`.
        pub fn with_evidence(&self, id: &str, tier: &str, code: &[(&str, Option<&str>)]) -> String {
            let h1 = self.commit_wt(id, "implementation", code);
            let mut lines: Vec<String> =
                std::fs::read_to_string(self.root.join("metrics/tasks.jsonl"))
                    .map(|t| t.lines().map(str::to_owned).collect())
                    .unwrap_or_default();
            lines.push(join_rec(id, "joined", &h1));
            self.set_tasks_jsonl(&lines);
            self.commit_wt(
                id,
                "review r1",
                &[(
                    &format!("reviews/{id}-r1.md"),
                    Some(&review_md(id, 1, tier, "approve", &h1)),
                )],
            );
            h1
        }
    }

    /// The allowed-path rule of the spec as a predicate (naive model, AC-5).
    pub fn allowed(
        p: &str,
        touches: &[&str],
        tier: &str,
        num: &str,
        items: &[&str],
        task_file: &str,
    ) -> bool {
        let generated = ["Cargo.lock", "docs/commands.json", "docs/scorecard.md"];
        if generated.contains(&p) || p == task_file {
            return true;
        }
        if touches.iter().any(|t| {
            let t = t.trim_end_matches('/');
            p == t || p.starts_with(&format!("{t}/"))
        }) {
            return true;
        }
        if tier == "low" {
            return false;
        }
        let parts: Vec<&str> = p.split('/').collect();
        let acc_file = |f: &str| f.starts_with(&format!("acceptance_t{num}")) && f.ends_with(".rs");
        let in_tests_dir = match parts.as_slice() {
            ["crates" | "apps", _, "tests", f] => acc_file(f),
            ["xtask", "tests", f] => acc_file(f),
            _ => false,
        };
        in_tests_dir
            || items
                .iter()
                .any(|i| p == format!("scorecard/items/{i}.toml"))
            || p.starts_with(&format!("reviews/T-{num}-"))
    }
}

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
    r.add_task("T-001", "low", "self_verified", &["RED", "xtask"], &[]);
    r.add_task("T-002", "low", "self_verified", &["RED"], &[]);
    let main_before = r.main_sha();
    assert_ne!(main_before, base);
    r.commit_wt(
        "T-001",
        "add RED and make my own gate always pass",
        &[
            ("RED", Some("red\n")),
            ("xtask/src/main.rs", Some(&weakened)),
        ],
    );
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
        ("crates/a/tests/acceptance_t008.rs", false),
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
