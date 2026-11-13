//! `cargo xtask publish T-NNN --title "<title>" --iteration N <path>...` (T-013 Interface (5)):
//! carries ledger inputs that change generated output (a new `scorecard/items/<id>.toml`, a new
//! or modified `evaluations/*.md`) to `main` as a low-tier publish task in one command.
//!
//! Steps, in order; each step's failure stops the command:
//! 1. checks (nothing changes; `publish: refused: …`, exit 1), including the would-be
//!    `docs/scorecard.md`, computed by main's `xtask/generated.toml` command for it in a private export of main's
//!    tree plus the inputs under `<git-common-dir>/kiln-publish/` (the primary checkout's
//!    `docs/scorecard.md` is never written);
//! 2. the task file on main (`publish: T-NNN ready`), transitions draft → specified → ready;
//! 3. the worktree (`worktree new`), ready → in_progress, committed on main at once
//!    (`publish: T-NNN in progress`, `metrics/` only) so every stop leaves main committed;
//! 4. the inputs and the regenerated `docs/scorecard.md` committed on `task/T-NNN`;
//! 5. the gate in the worktree;
//! 6. in_progress → self_verified → queued, `state: queued` on main (`publish: T-NNN queued`);
//!    main's history is append-only (D-040): nothing is ever reset, amended or rewritten, so a
//!    success adds six commits (ready, in progress, queued, content, queue record, merged);
//! 7. the inputs removed from the primary checkout (they come back with the fast-forward);
//! 8. `cargo xtask queue T-NNN` (child process, exit code passed through); on exit 0
//!    queued → merged, `state: merged` (`publish: T-NNN merged`) and `worktree rm T-NNN`.
//!
//! Commits use the repository's own commit configuration (signing included, no `--no-verify`).
//! The helper records no `run`.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result, bail};

use crate::git::{branch_exists, dirty_paths, git, git_try, rev};
use crate::queue::{GateRunner, GateXtask, is_evidence_path};
use crate::timefmt::{Clock, format_utc};
use crate::util::{Exit, Refused, capture, tail, valid_task_id};
use crate::worktree::Spec;

fn refused(msg: impl std::fmt::Display) -> anyhow::Error {
    Refused(format!("publish: refused: {msg}")).into()
}

/// Parsed command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Args {
    /// `T-NNN` as given (checked in step 1).
    pub task: String,
    /// Title.
    pub title: String,
    /// Iteration.
    pub iteration: u64,
    /// Input paths, relative to the repository root.
    pub inputs: Vec<String>,
}

impl Args {
    /// Parse `T-NNN --title <t> --iteration N <path>...` (usage errors: exit 2).
    pub fn parse(args: &[String]) -> Result<Self> {
        let usage = "usage: publish T-NNN --title \"<title>\" --iteration N <path>...";
        let mut it = args.iter();
        let (mut task, mut title, mut iteration, mut inputs) = (None, None, None, Vec::new());
        while let Some(a) = it.next() {
            match a.as_str() {
                "--title" | "--iteration" => {
                    let Some(v) = it.next() else {
                        bail!("{a} needs a value\n{usage}")
                    };
                    let slot = if a == "--title" {
                        &mut title
                    } else {
                        &mut iteration
                    };
                    if slot.replace(v.clone()).is_some() {
                        bail!("{a} given twice");
                    }
                }
                s if s.starts_with("--") => bail!("unknown option {s}\n{usage}"),
                s if task.is_none() => task = Some(s.to_owned()),
                s => inputs.push(s.to_owned()),
            }
        }
        let (Some(task), Some(title), Some(iteration)) = (task, title, iteration) else {
            bail!("{usage}");
        };
        let iteration = iteration
            .parse()
            .map_err(|_| anyhow::anyhow!("--iteration must be a non-negative integer"))?;
        Ok(Args {
            task,
            title,
            iteration,
            inputs,
        })
    }
}

/// `publish-` + the title in lowercase, every run of non-alphanumerics replaced by `-`, trimmed
/// of `-`, cut to 40 characters (T-013 Interface (5) step 2; the cut applies to the title part).
pub fn slug(title: &str) -> String {
    let mut s = String::new();
    for c in title.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            s.push(c);
        } else if !s.ends_with('-') {
            s.push('-');
        }
    }
    let t: String = s.trim_matches('-').chars().take(40).collect();
    format!("publish-{}", t.trim_end_matches('-'))
}

/// The task file written in step 2 (with `state`).
pub fn task_file_text(a: &Args, inputs: &[String], state: &str, now: &str) -> String {
    let mut touches = String::new();
    for p in inputs {
        touches.push_str(&format!("  - {p}\n"));
    }
    let list: Vec<String> = inputs.iter().map(|p| format!("`{p}`")).collect();
    format!(
        "---\nid: {id}\ntitle: \"{title}\"\ntier: low\nstate: {state}\nowner_role: director\nscorecard_items: []\ndepends_on: []\ntouches:\n{touches}budget_tokens: 150000\niteration: {it}\nfound_by: planner\norigin_task: null\n---\n\n\
# {id} {title}\n\n\
## Goal\npublish {inputs} and the regenerated `docs/scorecard.md`.\n\n\
## Interface\nn/a: a ledger-input publish task created by `cargo xtask publish` (T-013, D-034).\n\n\
## Pinned\nn/a: the change is mechanical (the inputs as written by their author, and the scorecard the gate regenerates).\n\n\
## Acceptance criteria\n\
- AC-1: every input on the branch is byte-identical to the file its author wrote.\n\
- AC-2: `docs/scorecard.md` is regenerated by `cargo xtask scorecard` and the gate is green.\n\
- AC-3: no other path changes.\n\n\
## Out of scope and implementer notes\nAny change to items, tests or code; defects an evaluator files are separate tasks.\n\n\
## Questions and rulings\nNone.\n\n\
## Log\n\
### director\n- {now} · created by cargo xtask publish\n<!-- end director log -->\n\
### planner\n<!-- end planner log -->\n\
### implementer\n<!-- end implementer log -->\n\
### test author\n<!-- end test author log -->\n\
### reviewer\n<!-- end reviewer log -->\n",
        id = a.task,
        title = a.title,
        it = a.iteration,
        inputs = list.join(", "),
    )
}

/// `text` with its front-matter `state:` line set to `state`.
fn with_state(text: &str, state: &str) -> String {
    let mut out = String::new();
    let mut in_fm = false;
    let mut done = false;
    for (i, l) in text.split_inclusive('\n').enumerate() {
        if l.trim() == "---" {
            in_fm = i == 0;
        }
        if in_fm && !done && l.starts_with("state:") {
            out.push_str(&format!("state: {state}\n"));
            done = true;
        } else {
            out.push_str(l);
        }
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Item,
    Evaluation,
}

fn kind(p: &str) -> Option<Kind> {
    let one = |rest: &str, ext: &str| {
        rest.strip_suffix(ext)
            .is_some_and(|stem| !stem.is_empty() && !stem.contains('/') && !stem.starts_with('.'))
    };
    if let Some(r) = p.strip_prefix("scorecard/items/")
        && one(r, ".toml")
    {
        return Some(Kind::Item);
    }
    if let Some(r) = p.strip_prefix("evaluations/")
        && one(r, ".md")
    {
        return Some(Kind::Evaluation);
    }
    None
}

/// The bytes of `path` in commit `rev` (`None`: absent).
fn blob(root: &Path, rev: &str, path: &str) -> Result<Option<Vec<u8>>> {
    let spec = format!("{rev}:{path}");
    if !git_try(root, &["cat-file", "-e", &spec])?.0 {
        return Ok(None);
    }
    let out = crate::git::cmd(root)
        .args(["cat-file", "blob", &spec])
        .output()
        .context("git cat-file")?;
    if !out.status.success() {
        bail!("git cat-file blob {spec} failed");
    }
    Ok(Some(out.stdout))
}

fn porcelain_path(l: &str) -> String {
    l[3..].trim_matches('"').to_owned()
}

/// Commit exactly `paths` with the repository's configuration; `Err(git stderr)` on failure.
fn commit(dir: &Path, subject: &str, task: &str, paths: &[String]) -> Result<Result<(), String>> {
    let mut add = vec!["add", "--"];
    add.extend(paths.iter().map(String::as_str));
    git(dir, &add)?;
    let task_line = format!("Task: {task}");
    let mut args = vec![
        "commit",
        "--quiet",
        "-m",
        subject,
        "-m",
        &task_line,
        "-m",
        "Role: director",
        "--",
    ];
    args.extend(paths.iter().map(String::as_str));
    let (ok, _, err) = git_try(dir, &args)?;
    Ok(if ok {
        Ok(())
    } else {
        Err(err.trim().lines().collect::<Vec<_>>().join("; "))
    })
}

/// Step 2/6/8 commit on main, or the exit-2 line with the hand command.
fn ledger_commit(root: &Path, subject: &str, task: &str, task_file: &str) -> Result<()> {
    ledger_commit_paths(
        root,
        subject,
        task,
        &[task_file.to_owned(), "metrics/tasks.jsonl".to_owned()],
    )
}

fn ledger_commit_paths(root: &Path, subject: &str, task: &str, paths: &[String]) -> Result<()> {
    if let Err(e) = commit(root, subject, task, paths)? {
        eprintln!(
            "publish: commit failed: {e}; commit by hand: git commit -m \"{subject}\" -m \"Task: {task}\" -m \"Role: director\" -- {}",
            paths.join(" ")
        );
        return Err(Exit(2).into());
    }
    Ok(())
}

fn transition(root: &Path, task: &str, to: &str, clock: &dyn Clock) -> Result<()> {
    let args: Vec<String> = ["--task", task, "--to", to]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    crate::metrics::transition(root, &args, clock)?;
    Ok(())
}

/// The command main's `xtask/generated.toml` lists for `docs/scorecard.md`, with `{xtask}`
/// replaced by main's xtask as the queue's regeneration does (T-013 ruling (3)).
fn scorecard_command(root: &Path, xtask: &Path) -> Result<Vec<String>> {
    let show = |p: &str| crate::precheck::show(root, "main", p);
    let policy = crate::policy::Policy::parse(
        show(crate::policy::GENERATED_FILE)?.as_deref(),
        show(crate::policy::LEDGER_FILE)?.as_deref(),
    )
    .map_err(|e| refused(format_args!("{}: {e:#}", crate::policy::GENERATED_FILE)))?;
    let Some(g) = policy
        .generated
        .iter()
        .find(|g| g.path == "docs/scorecard.md")
    else {
        return Err(refused(format_args!(
            "main's {} lists no command for docs/scorecard.md",
            crate::policy::GENERATED_FILE
        )));
    };
    Ok(g.command
        .iter()
        .map(|a| a.replace("{xtask}", &xtask.to_string_lossy()))
        .collect())
}

/// Run the regeneration command `argv` in `dir`; `Err(the command's own error)` on failure.
fn scorecard_in(dir: &Path, argv: &[String], target: &Path) -> Result<Result<(), String>> {
    let Some((prog, rest)) = argv.split_first() else {
        return Ok(Err("empty regeneration command".into()));
    };
    let out = capture(
        Command::new(prog)
            .args(rest)
            .current_dir(dir)
            .env("CARGO_TARGET_DIR", target)
            .env_remove("RUSTUP_TOOLCHAIN"),
    )?;
    if out.ok {
        return Ok(Ok(()));
    }
    let text = format!("{}\n{}", out.stdout, out.stderr);
    let last = tail(&text, 20);
    let lines: Vec<&str> = last
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    Ok(Err(lines.join("; ")))
}

/// Step 1: the checks. Returns main's xtask binary, the scorecard command and the inputs in
/// byte order.
fn checks(
    root: &Path,
    a: &Args,
    gate: &dyn GateRunner,
) -> Result<(PathBuf, Vec<String>, Vec<String>)> {
    let head = git(root, &["symbolic-ref", "--quiet", "HEAD"]).unwrap_or_default();
    if head != "refs/heads/main" {
        return Err(refused(format_args!(
            "{} must have `main` checked out (found `{head}`)",
            root.display()
        )));
    }
    let task = &a.task;
    if !valid_task_id(task) {
        return Err(refused(format_args!("`{task}` is not a task id T-NNN")));
    }
    if a.title.trim().is_empty() || a.title.contains(['"', '\\', '\n', '\r']) {
        return Err(refused(
            "the title must be non-empty and contain no double quote, backslash or line break",
        ));
    }
    let prefix = format!("tasks/{task}-");
    let on_main = git(root, &["ls-tree", "--name-only", "main", "--", "tasks/"])?;
    if let Some(f) = on_main
        .lines()
        .find(|l| l.starts_with(&prefix) && l.ends_with(".md"))
    {
        return Err(refused(format_args!("{f} exists on main")));
    }
    if let Ok(rd) = std::fs::read_dir(root.join("tasks")) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with(&format!("{task}-")) && name.ends_with(".md") {
                return Err(refused(format_args!(
                    "tasks/{name} exists in the working tree"
                )));
            }
        }
    }
    let spec = Spec::parse(task, false)?;
    if branch_exists(root, &spec.branch())? {
        return Err(refused(format_args!("branch {} exists", spec.branch())));
    }
    if spec.path(root).exists() {
        return Err(refused(format_args!(".worktrees/{task} exists")));
    }
    if a.inputs.is_empty() {
        return Err(refused("no input paths given"));
    }
    let mut inputs: Vec<String> = Vec::new();
    for p in &a.inputs {
        let p = p.strip_prefix("./").unwrap_or(p).to_owned();
        if inputs.contains(&p) {
            return Err(refused(format_args!("{p} is given twice")));
        }
        let Some(k) = kind(&p) else {
            return Err(refused(format_args!(
                "{p} is not a new scorecard/items/<id>.toml or an evaluations/*.md"
            )));
        };
        let full = root.join(&p);
        let is_file = std::fs::symlink_metadata(&full).is_ok_and(|m| m.is_file());
        if !is_file {
            return Err(refused(format_args!(
                "{p} is not a file in the working tree"
            )));
        }
        let main = blob(root, "main", &p)?;
        if k == Kind::Item && main.is_some() {
            return Err(refused(format_args!(
                "{p} exists on main; an item input must be new"
            )));
        }
        if main.as_deref() == Some(std::fs::read(&full)?.as_slice()) {
            return Err(refused(format_args!(
                "{p} does not differ from main's tree"
            )));
        }
        inputs.push(p);
    }
    inputs.sort();
    // The queue's working-tree rules (step 1 of the queue and `ledger`), apart from the inputs.
    let others: Vec<String> = dirty_paths(root, None)?
        .iter()
        .map(|l| porcelain_path(l))
        .filter(|p| !inputs.contains(p))
        .collect();
    let ledger: Vec<&String> = others.iter().filter(|p| is_evidence_path(p)).collect();
    if !ledger.is_empty() {
        return Err(refused(format_args!(
            "ledger: uncommitted changes to evidence files ({}); commit the ledger on main first (evidence is read from main's committed tree)",
            ledger
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    if !others.is_empty() {
        return Err(refused(format_args!(
            "the main working tree is dirty apart from the inputs: {}",
            others.join(", ")
        )));
    }
    // The would-be docs/scorecard.md: main's generated.toml command (with main's xtask, as the
    // queue builds it) in a private export of main + inputs; the checkout is not written.
    let main_sha = rev(root, "main")?;
    let xtask = gate.build_main_xtask(root, &main_sha)?;
    let argv = scorecard_command(root, &xtask)?;
    let base = crate::queue::common_dir(root)?.join("kiln-publish");
    let (tree, target) = (base.join("tree"), base.join("target"));
    if tree.exists() {
        std::fs::remove_dir_all(&tree)?;
    }
    crate::queue::export_tree(root, &main_sha, &tree, &base.join("export.index"))?;
    if let Ok(members) = crate::seed::member_names(&tree) {
        crate::seed::prune_to_non_members(&target, &members)?;
    }
    for p in &inputs {
        let dest = tree.join(p);
        if let Some(d) = dest.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::copy(root.join(p), &dest)?;
    }
    let result = scorecard_in(&tree, &argv, &target)?;
    let regenerated = std::fs::read(tree.join("docs/scorecard.md")).ok();
    let _ = std::fs::remove_dir_all(&tree);
    let named = inputs.join(", ");
    if let Err(e) = result {
        return Err(refused(format_args!("{named}: {e}")));
    }
    if regenerated == blob(root, "main", "docs/scorecard.md")? {
        return Err(refused(format_args!(
            "the inputs change no generated file; commit them on main as a ledger commit ({named})"
        )));
    }
    Ok((xtask, argv, inputs))
}

/// Run the publish helper; `Ok(())` is exit 0, other exits are [`Exit`] or [`Refused`].
pub fn run(root: &Path, a: &Args, clock: &dyn Clock) -> Result<()> {
    run_with(root, a, clock, &crate::queue::CargoGate, &queue_child)
}

/// Step 8's queue: `cargo xtask queue T-NNN` as a child (this binary), output passed through;
/// its exit code.
fn queue_child(root: &Path, task: &str) -> Result<i32> {
    let exe = std::env::current_exe().context("locate the xtask binary")?;
    let status = Command::new(exe)
        .args(["queue", task])
        .current_dir(root)
        .status()
        .context("start the queue")?;
    Ok(status.code().unwrap_or(1))
}

/// [`run`] with the gate and the queue injected (unit tests use fakes).
pub fn run_with(
    root: &Path,
    a: &Args,
    clock: &dyn Clock,
    gate: &dyn GateRunner,
    queue: &dyn Fn(&Path, &str) -> Result<i32>,
) -> Result<()> {
    let task = a.task.clone();
    // 1.
    let (xtask, argv, inputs) = checks(root, a, gate)?;
    let now = || format_utc(clock.now_millis().div_euclid(1000));

    // 2.
    let task_file = format!("tasks/{task}-{}.md", slug(&a.title));
    let text = task_file_text(a, &inputs, "ready", &now());
    std::fs::create_dir_all(root.join("tasks"))?;
    std::fs::write(root.join(&task_file), &text)?;
    transition(root, &task, "specified", clock)?;
    transition(root, &task, "ready", clock)?;
    ledger_commit(root, &format!("publish: {task} ready"), &task, &task_file)?;
    println!("publish: {task_file} committed on main ({task} ready)");

    // 3.
    let spec = Spec::parse(&task, false)?;
    let info = crate::worktree::create(root, &spec, true)?;
    println!(
        "publish: worktree {} on {}",
        info.path.display(),
        info.branch
    );
    transition(root, &task, "in_progress", clock)?;
    // Ruling Q-b: main's ledger is committed wherever the command stops.
    ledger_commit_paths(
        root,
        &format!("publish: {task} in progress"),
        &task,
        &["metrics/tasks.jsonl".to_owned()],
    )?;

    // 4.
    let wt = info.path.clone();
    for p in &inputs {
        let dest = wt.join(p);
        if let Some(d) = dest.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::copy(root.join(p), &dest)?;
    }
    if let Err(e) = scorecard_in(&wt, &argv, &wt.join("target"))? {
        eprintln!(
            "publish: regenerating docs/scorecard.md in .worktrees/{task} failed: {e}; {task} stays in_progress"
        );
        return Err(Exit(1).into());
    }
    let mut content = inputs.clone();
    content.push("docs/scorecard.md".into());
    let stray: Vec<String> = dirty_paths(&wt, None)?
        .iter()
        .map(|l| porcelain_path(l))
        .filter(|p| !content.contains(p))
        .collect();
    if !stray.is_empty() {
        eprintln!(
            "publish: the content commit would contain other paths ({}); {task} stays in_progress",
            stray.join(", ")
        );
        return Err(Exit(1).into());
    }
    let subject = format!(
        "{task}: publish {}; regenerate docs/scorecard.md",
        inputs.join(" ")
    );
    if let Err(e) = commit(&wt, &subject, &task, &content)? {
        eprintln!("publish: commit failed in .worktrees/{task}: {e}; {task} stays in_progress");
        return Err(Exit(2).into());
    }
    let committed = git(&wt, &["show", "--name-only", "--format=", "HEAD"])?;
    let mut got: Vec<&str> = committed.lines().filter(|l| !l.is_empty()).collect();
    got.sort();
    let mut want: Vec<&str> = content.iter().map(String::as_str).collect();
    want.sort();
    want.retain(|p| got.contains(p) || *p != "docs/scorecard.md");
    if got != want {
        eprintln!(
            "publish: the content commit on {} contains {} instead of {}; {task} stays in_progress",
            info.branch,
            got.join(", "),
            want.join(", ")
        );
        return Err(Exit(1).into());
    }

    // 5.
    let outcome = gate.run(&wt, GateXtask::Binary(&xtask))?;
    if !outcome.passed {
        eprintln!(
            "publish: gate failed in .worktrees/{task} at {}; {task} stays in_progress (committed on main); fix the input and re-run the gate there, or abandon (metrics transition --to abandoned; worktree rm {task} --force)",
            outcome.failed_step.as_deref().unwrap_or("unknown")
        );
        return Err(Exit(1).into());
    }

    // 6.
    transition(root, &task, "self_verified", clock)?;
    transition(root, &task, "queued", clock)?;
    let current = std::fs::read_to_string(root.join(&task_file))?;
    std::fs::write(root.join(&task_file), with_state(&current, "queued"))?;
    ledger_commit(root, &format!("publish: {task} queued"), &task, &task_file)?;

    // 7.
    for p in &inputs {
        let branch_copy = blob(root, &spec.branch(), p)?;
        if branch_copy.as_deref() != Some(std::fs::read(root.join(p))?.as_slice()) {
            eprintln!(
                "publish: {p} in the primary checkout differs from {}; not removed; {task} stays queued",
                spec.branch()
            );
            return Err(Exit(1).into());
        }
        if blob(root, "HEAD", p)?.is_some() {
            git(root, &["checkout", "--quiet", "HEAD", "--", p])?;
        } else {
            std::fs::remove_file(root.join(p))?;
        }
        println!("publish: {p} moved to {}", spec.branch());
    }

    // 8.
    let code = queue(root, &task)?;
    if code != 0 {
        eprintln!(
            "publish: queue did not merge {task} (exit {code}); the task stays queued; the inputs are on {} (git show {}:<path>)",
            spec.branch(),
            spec.branch()
        );
        return Err(Exit(u8::try_from(code).unwrap_or(1)).into());
    }
    transition(root, &task, "merged", clock)?;
    let current = std::fs::read_to_string(root.join(&task_file))?;
    std::fs::write(root.join(&task_file), with_state(&current, "merged"))?;
    ledger_commit(root, &format!("publish: {task} merged"), &task, &task_file)?;
    println!("{}", crate::worktree::remove(root, &spec, false)?);
    println!("publish: {task} merged");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs() {
        assert_eq!(slug("Publish x"), "publish-publish-x");
        assert_eq!(
            slug("  Iter-0 evaluation (final)!"),
            "publish-iter-0-evaluation-final"
        );
        let long = slug(&"a".repeat(50));
        assert_eq!(long.len(), "publish-".len() + 40);
        assert_eq!(slug("Ünïcode"), "publish-n-code");
        // Ruling Q-c: trailing `-` trimmed after the cut.
        let cut = slug(&format!("{} tail", "b".repeat(39)));
        assert_eq!(cut, format!("publish-{}", "b".repeat(39)));
    }

    #[test]
    fn args_and_kinds() {
        let v = |s: &str| s.split_whitespace().map(String::from).collect::<Vec<_>>();
        let a = Args::parse(&v("T-050 --title X --iteration 1 a b")).unwrap();
        assert_eq!(
            (a.task.as_str(), a.iteration, a.inputs.len()),
            ("T-050", 1, 2)
        );
        assert!(Args::parse(&v("T-050 --title X a")).is_err());
        assert!(Args::parse(&v("T-050 --title X --iteration x a")).is_err());
        assert!(Args::parse(&v("T-050 --bogus 1")).is_err());
        assert_eq!(kind("scorecard/items/x.y.toml"), Some(Kind::Item));
        assert_eq!(kind("evaluations/iter-0.md"), Some(Kind::Evaluation));
        for bad in [
            "docs/x.md",
            "scorecard/items/a/b.toml",
            "evaluations/.md",
            "evaluations/x.txt",
        ] {
            assert_eq!(kind(bad), None, "{bad}");
        }
    }

    #[test]
    fn scorecard_command_comes_from_mains_generated_toml() {
        use crate::git::testrepo;
        let led = "[[ledger]]\npath = \"tasks/\"\nmode = \"any\"\n";
        let gen_toml = "[[generated]]\npath = \"docs/scorecard.md\"\ncommand = [\"{xtask}\", \"scorecard\", \"--x\"]\n";
        let repo = testrepo::init(
            "pub-cmd",
            &[
                ("xtask/generated.toml", gen_toml),
                ("xtask/ledger.toml", led),
            ],
        );
        let argv = scorecard_command(repo.path(), Path::new("/bin/main-xtask")).unwrap();
        assert_eq!(argv, ["/bin/main-xtask", "scorecard", "--x"]);
        // The working copy is not read: only main's committed file.
        testrepo::write(repo.path(), "xtask/generated.toml", "broken [");
        assert!(scorecard_command(repo.path(), Path::new("x")).is_ok());
        let other = gen_toml.replace("docs/scorecard.md", "docs/other.md");
        testrepo::commit(repo.path(), "other", &[("xtask/generated.toml", &other)]);
        let e = scorecard_command(repo.path(), Path::new("x")).unwrap_err();
        assert!(e.to_string().starts_with("publish: refused: "), "{e}");
        // A failing command reports its own error.
        let d = crate::util::testutil::TempDir::new("pub-run");
        let argv: Vec<String> = ["sh", "-c", "echo bad item >&2; exit 1"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        let r = scorecard_in(d.path(), &argv, d.path()).unwrap();
        assert_eq!(r, Err("bad item".to_owned()));
    }

    /// A clock that moves one second per reading.
    struct Tick(std::cell::Cell<i64>);
    impl Clock for Tick {
        fn now_millis(&self) -> i64 {
            let t = self.0.get();
            self.0.set(t + 1000);
            t
        }
    }

    /// The toy's gate passes iff `pass`; main's xtask is a fake path.
    struct Gate {
        pass: bool,
    }
    impl GateRunner for Gate {
        fn run(&self, _: &Path, _: GateXtask<'_>) -> Result<crate::queue::GateOutcome> {
            Ok(crate::queue::GateOutcome {
                passed: self.pass,
                failed_step: (!self.pass).then(|| "red".into()),
            })
        }
        fn build_main_xtask(&self, _: &Path, _: &str) -> Result<PathBuf> {
            Ok(PathBuf::from("/fake/main-xtask"))
        }
    }

    fn toy() -> crate::util::testutil::TempDir {
        use crate::git::testrepo;
        let gen_toml = "[[generated]]\npath = \"docs/scorecard.md\"\ncommand = [\"sh\", \"-c\", \"cat scorecard/items/* evaluations/* > docs/scorecard.md\"]\n";
        let led = ["tasks/", "metrics/", "evaluations/"]
            .iter()
            .map(|p| format!("[[ledger]]\npath = \"{p}\"\nmode = \"any\"\n\n"))
            .collect::<String>()
            + "[[ledger]]\npath = \"scorecard/items/\"\nmode = \"add-only\"\n";
        let repo = testrepo::init(
            "publish-e2e",
            &[
                (".gitignore", ".worktrees/\ntarget/\n"),
                ("xtask/generated.toml", gen_toml),
                ("xtask/ledger.toml", &led),
                ("scorecard/items/a.b.toml", "a\n"),
                ("evaluations/iter-0.md", "old\n"),
                ("docs/scorecard.md", "a\nold\n"),
                (
                    "tasks/T-001-x.md",
                    "---\nid: T-001\ntier: low\nstate: merged\n---\n",
                ),
            ],
        );
        testrepo::write(repo.path(), "scorecard/items/x.y.toml", "x\n");
        testrepo::write(repo.path(), "evaluations/iter-0.md", "new\n");
        repo
    }

    fn first_parent_subjects(root: &Path, since: &str) -> Vec<String> {
        git(
            root,
            &[
                "log",
                "--reverse",
                "--first-parent",
                "--format=%s",
                &format!("{since}..main"),
            ],
        )
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect()
    }

    fn args() -> Args {
        Args {
            task: "T-050".into(),
            title: "Publish x".into(),
            iteration: 1,
            inputs: vec![
                "scorecard/items/x.y.toml".into(),
                "evaluations/iter-0.md".into(),
            ],
        }
    }

    #[test]
    fn success_appends_six_commits_and_never_rewrites_main() {
        let repo = toy();
        let root = repo.path();
        let tip = rev(root, "main").unwrap();
        let clock = Tick(std::cell::Cell::new(1_793_000_000_000));
        let queue_clock = Tick(std::cell::Cell::new(1_793_000_100_000));
        let gate = Gate { pass: true };
        let queue = |r: &Path, t: &str| -> Result<i32> {
            let rec = crate::queue::run(r, t, &queue_clock, &Gate { pass: true })?;
            Ok(if rec.result == "merged" { 0 } else { 1 })
        };
        run_with(root, &args(), &clock, &gate, &queue).unwrap();
        assert_eq!(
            first_parent_subjects(root, &tip),
            [
                "publish: T-050 ready",
                "publish: T-050 in progress",
                "publish: T-050 queued",
                "T-050: publish evaluations/iter-0.md scorecard/items/x.y.toml; regenerate docs/scorecard.md",
                "queue: record T-050 merged",
                "publish: T-050 merged",
            ]
        );
        // D-040: every value main took during the run (its reflog since the tip) is an
        // ancestor of the final head, and so is the pre-publish tip.
        let head = rev(root, "main").unwrap();
        let reflog = git(root, &["reflog", "show", "--format=%H", "main"]).unwrap();
        let mut seen = 0;
        for c in reflog.lines() {
            assert!(
                crate::precheck::is_ancestor(root, c, &head).unwrap(),
                "main once pointed at {c}, which is not an ancestor of {head}"
            );
            seen += 1;
            if c == tip {
                break;
            }
        }
        assert!(seen >= 7, "reflog covers the run: {reflog}");
        assert!(crate::precheck::is_ancestor(root, &tip, &head).unwrap());
        assert!(git(root, &["status", "--porcelain"]).unwrap().is_empty());
        assert!(!root.join(".worktrees/T-050").exists());
        assert_eq!(
            std::fs::read_to_string(root.join("docs/scorecard.md")).unwrap(),
            "a\nx\nnew\n"
        );
    }

    #[test]
    fn red_gate_keeps_ready_and_in_progress_committed() {
        let repo = toy();
        let root = repo.path();
        let tip = rev(root, "main").unwrap();
        let clock = Tick(std::cell::Cell::new(1_793_000_000_000));
        let queue = |_: &Path, _: &str| -> Result<i32> { panic!("no queue after a red gate") };
        let e = run_with(root, &args(), &clock, &Gate { pass: false }, &queue).unwrap_err();
        assert_eq!(e.downcast_ref::<Exit>(), Some(&Exit(1)));
        assert_eq!(
            first_parent_subjects(root, &tip),
            ["publish: T-050 ready", "publish: T-050 in progress"]
        );
        let committed = crate::precheck::show(root, "main", "metrics/tasks.jsonl")
            .unwrap()
            .unwrap();
        assert!(
            committed
                .trim_end()
                .ends_with("\"from\":\"ready\",\"to\":\"in_progress\"}"),
            "{committed}"
        );
        // Only the inputs are left in the primary checkout.
        let mut dirty = dirty_paths(root, None).unwrap();
        dirty.sort();
        assert_eq!(
            dirty,
            [" M evaluations/iter-0.md", "?? scorecard/items/x.y.toml"]
        );
    }

    #[test]
    fn task_file_front_matter() {
        let a = Args {
            task: "T-050".into(),
            title: "Publish x".into(),
            iteration: 1,
            inputs: vec![],
        };
        let inputs = vec![
            "evaluations/iter-0.md".to_owned(),
            "scorecard/items/x.y.toml".to_owned(),
        ];
        let t = task_file_text(&a, &inputs, "ready", "2026-11-13T00:00:00Z");
        let (fm, body) = crate::frontmatter::parse(&t).unwrap();
        assert_eq!(fm["state"].as_str(), Some("ready"));
        assert_eq!(fm["touches"].as_list(), inputs);
        assert_eq!(fm["budget_tokens"].as_str(), Some("150000"));
        assert!(body.contains("- 2026-11-13T00:00:00Z · created by cargo xtask publish\n"));
        let q = with_state(&t, "queued");
        assert_eq!(
            crate::frontmatter::parse(&q).unwrap().0["state"].as_str(),
            Some("queued")
        );
        assert_eq!(q.replace("state: queued", "state: ready"), t);
    }
}
