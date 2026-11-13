//! Shared fixture of the T-005 acceptance and guard tests (included with `#[path]`; as its own
//! test target it holds no tests). A toy repository is a tiny cargo workspace with a toy `xtask`
//! package (no dependencies) whose `ci --report <file>` fails exactly when a file `RED` exists,
//! whose `scorecard` writes a deterministic summary of `items/` to `docs/scorecard.md` (and fails
//! when `items/poison.toml` exists), and whose `schema --write` writes a constant
//! `docs/commands.json`. The policy files `xtask/generated.toml` / `xtask/ledger.toml` are the
//! ones this package ships (their layout is the schema, D-019); a stand-in with the spec's
//! entries is used only where they do not exist yet (the base branch, whose queue ignores them).
#![allow(dead_code)]
#[cfg(test)]
pub(crate) mod support {
    use std::path::{Path, PathBuf};

    use serde_json::Value;
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
        // Keep what the caller set explicitly (e.g. its own CARGO_TARGET_DIR).
        let explicit: Vec<std::ffi::OsString> =
            c.get_envs().map(|(k, _)| k.to_os_string()).collect();
        for (k, _) in std::env::vars_os() {
            if explicit.contains(&k) {
                continue;
            }
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
            // Evidence counts only when committed on main (D-026 R6): a director ledger commit.
            let o = run(Command::new("git")
                .args(["status", "--porcelain", "--", "metrics/tasks.jsonl"])
                .current_dir(&self.root));
            if !o.stdout.trim().is_empty() {
                git(&self.root, &["add", "--", "metrics/tasks.jsonl"]);
                git(
                    &self.root,
                    &[
                        "commit",
                        "-q",
                        "-m",
                        "director: join records",
                        "--",
                        "metrics/tasks.jsonl",
                    ],
                );
            }
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
