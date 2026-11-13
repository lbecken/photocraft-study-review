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
//!    (`publish: T-NNN in progress`, the task file and `metrics/tasks.jsonl`) so every stop
//!    leaves main committed;
//! 4. the inputs and the regenerated `docs/scorecard.md` committed on `task/T-NNN`;
//! 5. the gate in the worktree;
//! 6. in_progress → self_verified → queued, `state: queued` on main (`publish: T-NNN queued`);
//!    main's history is append-only (D-040): nothing is ever reset, amended or rewritten, so a
//!    success adds six commits (ready, in progress, queued, content, queue record, merged);
//! 7. the inputs removed from the primary checkout (they come back with the fast-forward);
//! 8. `cargo xtask queue T-NNN` (child process, exit code passed through); on exit 0
//!    queued → merged, `state: merged` (`publish: T-NNN merged`) and `worktree rm T-NNN`.
//!
//! Steps 1–7 run under the queue's lock `<git-common-dir>/kiln-queue.lock`, released just before
//! step 8 (ruling D-047 (m)). From step 3 on, main's task file `state:` is committed with every
//! transition (D-047 (n)); a stop in steps 3, 4 or 7 without a pinned line prints
//! `publish: stopped at step <n>: <error>; T-NNN is <state>`.
//!
//! Commits use the repository's own commit configuration (signing included, no `--no-verify`).
//! The helper records no `run`.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result, bail};

use crate::git::{branch_exists, dirty_paths, git, git_try, rev};
use crate::queue::{GateRunner, GateXtask, Lock, LockHeld, is_evidence_path};
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
    // Ruling D-047 (l): a failing `git add` is a commit failure.
    let (ok, _, err) = git_try(dir, &add)?;
    if !ok {
        return Ok(Err(one_line(&err)));
    }
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
    Ok(if ok { Ok(()) } else { Err(one_line(&err)) })
}

/// `text`'s non-empty lines, trimmed, joined with `; `.
fn one_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("; ")
}

/// The task's current state in the working tree's `metrics/tasks.jsonl` (the `to` of its last
/// transition; `draft` if none).
fn ledger_state(root: &Path, task: &str) -> String {
    crate::util::read_jsonl(&root.join("metrics/tasks.jsonl"))
        .map(|r| crate::metrics::current_state(&r, task).0)
        .unwrap_or_else(|_| "unknown".into())
}

/// Ruling D-047 (n): a stop in step 3, 4 or 7 without a pinned line prints
/// `publish: stopped at step <n>: <error>; T-NNN is <state>`; exit 1 (ruling D-047 (q)).
fn stopped(root: &Path, step: u8, task: &str, error: &anyhow::Error) -> anyhow::Error {
    eprintln!(
        "{}",
        stop_line(step, task, error, &ledger_state(root, task))
    );
    Exit(1).into()
}

fn stop_line(step: u8, task: &str, error: &anyhow::Error, state: &str) -> String {
    format!(
        "publish: stopped at step {step}: {}; {task} is {state}",
        one_line(&format!("{error:#}"))
    )
}

/// Ruling D-047 (m): the queue's lock; held by a live process → the publish refusal.
fn take_lock(root: &Path, task: &str) -> Result<Lock> {
    let path = crate::queue::common_dir(root)?.join("kiln-queue.lock");
    Lock::acquire(&path, task).map_err(|e| match e.downcast_ref::<LockHeld>() {
        Some(h) => refused(format_args!(
            "another queue or publish run holds {}",
            h.path.display()
        )),
        None => e,
    })
}

/// `state:` of main's task file in the primary checkout set to `state`.
fn set_state(root: &Path, task_file: &str, state: &str) -> Result<()> {
    let current = std::fs::read_to_string(root.join(task_file))?;
    std::fs::write(root.join(task_file), with_state(&current, state))?;
    Ok(())
}

/// Step 2 before its commit: the task file and the two transitions. On failure, ruling D-047
/// (k): the task file is removed and `metrics/tasks.jsonl` restored to the bytes it had, so
/// main is as it was found; the error is returned unchanged.
fn write_ready(
    root: &Path,
    task: &str,
    task_file: &str,
    text: &str,
    clock: &dyn Clock,
) -> Result<()> {
    let ledger = root.join("metrics/tasks.jsonl");
    let (had_tasks, had_metrics) = (root.join("tasks").is_dir(), root.join("metrics").is_dir());
    let before = match std::fs::read(&ledger) {
        Ok(b) => Some(b),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e).with_context(|| format!("read {}", ledger.display())),
    };
    let result = (|| -> Result<()> {
        std::fs::create_dir_all(root.join("tasks"))?;
        std::fs::write(root.join(task_file), text)?;
        transition(root, task, "specified", clock)?;
        transition(root, task, "ready", clock)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(root.join(task_file));
        let _ = match &before {
            Some(b) => std::fs::write(&ledger, b),
            None => std::fs::remove_file(&ledger),
        };
        if !had_tasks {
            let _ = std::fs::remove_dir(root.join("tasks"));
        }
        if !had_metrics {
            let _ = std::fs::remove_dir(root.join("metrics"));
        }
    }
    result
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
    // Ruling D-047 (p): the slug needs a letter or digit (the slug's own alphabet).
    if !a
        .title
        .chars()
        .flat_map(char::to_lowercase)
        .any(|c| c.is_ascii_alphanumeric())
    {
        return Err(refused("the title has no letter or digit"));
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
    // Ruling D-047 (k): a task number publish (or anything else) has used is not reused.
    let records = crate::util::read_jsonl(&root.join("metrics/tasks.jsonl"))?;
    if records
        .iter()
        .any(|r| r["task"].as_str() == Some(task.as_str()))
    {
        return Err(refused(format_args!(
            "{task} already has records in metrics/tasks.jsonl"
        )));
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
    // Interface (5) pins both lines exactly: the scorecard's own error, and the no-change line.
    if let Err(e) = result {
        return Err(refused(e));
    }
    if regenerated == blob(root, "main", "docs/scorecard.md")? {
        return Err(refused(
            "the inputs change no generated file; commit them on main as a ledger commit",
        ));
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
    // Ruling D-047 (m): the queue's lock, from before step 1 touches the queue's private build
    // tree until just before step 8 (dropped on every earlier stop).
    let lock = take_lock(root, &task)?;
    // 1.
    let (xtask, argv, inputs) = checks(root, a, gate)?;
    let now = || format_utc(clock.now_millis().div_euclid(1000));

    // 2.
    let task_file = format!("tasks/{task}-{}.md", slug(&a.title));
    let text = task_file_text(a, &inputs, "ready", &now());
    write_ready(root, &task, &task_file, &text, clock)?;
    ledger_commit(root, &format!("publish: {task} ready"), &task, &task_file)?;
    println!("publish: {task_file} committed on main ({task} ready)");

    // 3. Ruling D-047 (n): from here on main's committed task file `state:` follows the ledger.
    let spec = Spec::parse(&task, false)?;
    let info = (|| -> Result<crate::worktree::Info> {
        let info = crate::worktree::create(root, &spec, true)?;
        println!(
            "publish: worktree {} on {}",
            info.path.display(),
            info.branch
        );
        transition(root, &task, "in_progress", clock)?;
        set_state(root, &task_file, "in_progress")?;
        Ok(info)
    })()
    .map_err(|e| stopped(root, 3, &task, &e))?;
    // Ruling Q-b: main's ledger is committed wherever the command stops.
    ledger_commit(
        root,
        &format!("publish: {task} in progress"),
        &task,
        &task_file,
    )?;

    // 4.
    let wt = info.path.clone();
    // Ruling D-047 (q): every stop in step 4 exits 1, a failed content commit included.
    content_commit(root, &wt, &task, &inputs, &argv, &info.branch)
        .map_err(|e| stopped(root, 4, &task, &e))?;

    // 5.
    let outcome = gate.run(&wt, GateXtask::Binary(&xtask))?;
    if !outcome.passed {
        eprintln!(
            "publish: gate failed in .worktrees/{task} at {}; {task} stays in_progress; fix the input and re-run the gate there, or abandon (metrics transition --to abandoned; worktree rm {task} --force)",
            outcome.failed_step.as_deref().unwrap_or("unknown")
        );
        return Err(Exit(1).into());
    }

    // 6.
    transition(root, &task, "self_verified", clock)?;
    transition(root, &task, "queued", clock)?;
    set_state(root, &task_file, "queued")?;
    ledger_commit(root, &format!("publish: {task} queued"), &task, &task_file)?;

    // 7. Every input is compared before any is removed.
    hand_over(root, &inputs, &spec.branch()).map_err(|e| stopped(root, 7, &task, &e))?;

    // 8. The queue takes the lock itself.
    drop(lock);
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
    set_state(root, &task_file, "merged")?;
    ledger_commit(root, &format!("publish: {task} merged"), &task, &task_file)?;
    println!("{}", crate::worktree::remove(root, &spec, false)?);
    println!("publish: {task} merged");
    Ok(())
}

/// Step 4: copy the inputs from the primary checkout `root` into the worktree `wt`, regenerate `docs/scorecard.md` there and
/// commit exactly those paths on `branch`.
fn content_commit(
    root: &Path,
    wt: &Path,
    task: &str,
    inputs: &[String],
    argv: &[String],
    branch: &str,
) -> Result<()> {
    for p in inputs {
        let src = root.join(p);
        let dest = wt.join(p);
        if let Some(d) = dest.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::copy(&src, &dest).with_context(|| format!("copy {p} into .worktrees/{task}"))?;
    }
    if let Err(e) = scorecard_in(wt, argv, &wt.join("target"))? {
        bail!("regenerating docs/scorecard.md in .worktrees/{task} failed: {e}");
    }
    let mut content = inputs.to_vec();
    content.push("docs/scorecard.md".into());
    let stray: Vec<String> = dirty_paths(wt, None)?
        .iter()
        .map(|l| porcelain_path(l))
        .filter(|p| !content.contains(p))
        .collect();
    if !stray.is_empty() {
        bail!(
            "the content commit would contain other paths ({})",
            stray.join(", ")
        );
    }
    let subject = format!(
        "{task}: publish {}; regenerate docs/scorecard.md",
        inputs.join(" ")
    );
    if let Err(e) = commit(wt, &subject, task, &content)? {
        bail!("commit failed in .worktrees/{task}: {e}");
    }
    let committed = git(wt, &["show", "--name-only", "--format=", "HEAD"])?;
    let mut got: Vec<&str> = committed.lines().filter(|l| !l.is_empty()).collect();
    got.sort();
    let mut want: Vec<&str> = content.iter().map(String::as_str).collect();
    want.sort();
    want.retain(|p| got.contains(p) || *p != "docs/scorecard.md");
    if got != want {
        bail!(
            "the content commit on {branch} contains {} instead of {}",
            got.join(", "),
            want.join(", ")
        );
    }
    Ok(())
}

/// Step 7: check every input's bytes against `branch`'s copy, then remove each from the primary
/// checkout (untracked: deleted; tracked: restored to main's version).
fn hand_over(root: &Path, inputs: &[String], branch: &str) -> Result<()> {
    for p in inputs {
        let branch_copy = blob(root, branch, p)?;
        if branch_copy.as_deref() != Some(std::fs::read(root.join(p))?.as_slice()) {
            bail!("{p} in the primary checkout differs from {branch}; not removed");
        }
    }
    for p in inputs {
        if blob(root, "HEAD", p)?.is_some() {
            git(root, &["checkout", "--quiet", "HEAD", "--", p])?;
        } else {
            std::fs::remove_file(root.join(p))?;
        }
        println!("publish: {p} moved to {branch}");
    }
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

    /// A gate whose run calls `during(worktree)` first; counts main-xtask builds.
    struct HookGate<'a> {
        pass: bool,
        during: &'a dyn Fn(&Path),
        builds: std::cell::Cell<u32>,
    }
    impl GateRunner for HookGate<'_> {
        fn run(&self, wt: &Path, _: GateXtask<'_>) -> Result<crate::queue::GateOutcome> {
            (self.during)(wt);
            Ok(crate::queue::GateOutcome {
                passed: self.pass,
                failed_step: (!self.pass).then(|| "red".into()),
            })
        }
        fn build_main_xtask(&self, _: &Path, _: &str) -> Result<PathBuf> {
            self.builds.set(self.builds.get() + 1);
            Ok(PathBuf::from("/fake/main-xtask"))
        }
    }

    fn hook_gate(during: &dyn Fn(&Path)) -> HookGate<'_> {
        HookGate {
            pass: true,
            during,
            builds: std::cell::Cell::new(0),
        }
    }

    fn no_queue(_: &Path, _: &str) -> Result<i32> {
        panic!("the queue must not run")
    }

    /// Everything a refusal must leave unchanged: main's tip, branches, worktrees, status, ledger.
    fn snapshot(root: &Path) -> (String, String, String, String, Option<Vec<u8>>) {
        (
            rev(root, "main").unwrap(),
            git(root, &["branch", "--list"]).unwrap(),
            git(root, &["worktree", "list", "--porcelain"]).unwrap(),
            git(root, &["status", "--porcelain", "--ignored"]).unwrap(),
            std::fs::read(root.join("metrics/tasks.jsonl")).ok(),
        )
    }

    fn refusal(e: &anyhow::Error) -> String {
        e.downcast_ref::<Refused>()
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("not a refusal: {e:#}"))
    }

    /// main's committed task-file state and the last committed transition of T-050.
    fn committed_state(root: &Path) -> (String, String) {
        let file = git(root, &["ls-tree", "--name-only", "main", "--", "tasks/"])
            .unwrap()
            .lines()
            .find(|l| l.starts_with("tasks/T-050-"))
            .unwrap()
            .to_owned();
        let text = crate::precheck::show(root, "main", &file).unwrap().unwrap();
        let fm = crate::frontmatter::parse(&text).unwrap().0;
        let ledger = crate::precheck::show(root, "main", "metrics/tasks.jsonl")
            .unwrap()
            .unwrap_or_default();
        let last = ledger
            .lines()
            .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
            .rfind(|v| v["kind"] == "transition" && v["task"] == "T-050")
            .map(|v| v["to"].as_str().unwrap().to_owned())
            .unwrap();
        (fm["state"].as_str().unwrap().to_owned(), last)
    }

    #[test]
    fn step_one_refuses_a_used_task_number_and_a_title_without_letters() {
        let repo = toy();
        let root = repo.path();
        // Ruling D-047 (k): an abandoned T-050 whose task file was deleted.
        crate::git::testrepo::commit(
            root,
            "ledger",
            &[(
                "metrics/tasks.jsonl",
                "{\"kind\":\"transition\",\"task\":\"T-050\",\"iteration\":1,\"at\":\"2026-11-13T01:00:00Z\",\"from\":\"draft\",\"to\":\"abandoned\"}\n",
            )],
        );
        let before = snapshot(root);
        let clock = Tick(std::cell::Cell::new(1_793_000_000_000));
        let gate = hook_gate(&|_| {});
        let e = run_with(root, &args(), &clock, &gate, &no_queue).unwrap_err();
        assert_eq!(
            refusal(&e),
            "publish: refused: T-050 already has records in metrics/tasks.jsonl"
        );
        assert_eq!(snapshot(root), before);
        assert_eq!(gate.builds.get(), 0, "refused before main's xtask is built");
        // Ruling D-047 (p).
        for title in ["!!!", "---", " ? ", "Ü"] {
            let a = Args {
                task: "T-051".into(),
                title: title.into(),
                ..args()
            };
            let e = run_with(root, &a, &clock, &gate, &no_queue).unwrap_err();
            assert_eq!(
                refusal(&e),
                "publish: refused: the title has no letter or digit",
                "{title:?}"
            );
            assert_eq!(snapshot(root), before);
        }
    }

    #[test]
    fn a_held_queue_lock_refuses_publish_before_anything_is_built() {
        let repo = toy();
        let root = repo.path();
        let lock = root.join(".git/kiln-queue.lock");
        // Held by a live process (this one), as by a running queue.
        let holder = format!("{} T-099\n", std::process::id());
        std::fs::write(&lock, &holder).unwrap();
        let before = snapshot(root);
        let clock = Tick(std::cell::Cell::new(1_793_000_000_000));
        let gate = hook_gate(&|_| {});
        let e = run_with(root, &args(), &clock, &gate, &no_queue).unwrap_err();
        assert_eq!(
            refusal(&e),
            format!(
                "publish: refused: another queue or publish run holds {}",
                lock.display()
            )
        );
        assert_eq!(snapshot(root), before);
        assert_eq!(gate.builds.get(), 0);
        assert!(!root.join(".git/kiln-publish").exists());
        assert!(!root.join(".git/kiln-queue").exists());
        assert_eq!(std::fs::read_to_string(&lock).unwrap(), holder);
        std::fs::remove_file(&lock).unwrap();
    }

    #[test]
    fn the_lock_is_held_through_step_seven_and_released_for_the_queue() {
        let repo = toy();
        let root = repo.path().to_path_buf();
        let lock = root.join(".git/kiln-queue.lock");
        let clock = Tick(std::cell::Cell::new(1_793_000_000_000));
        let seen = std::cell::Cell::new(false);
        let during = |_: &Path| {
            let held = std::fs::read_to_string(&lock).unwrap();
            assert!(
                held.starts_with(&format!("{} T-050", std::process::id())),
                "{held}"
            );
            seen.set(true);
        };
        let gate = hook_gate(&during);
        let queue = |r: &Path, _: &str| -> Result<i32> {
            assert!(
                !r.join(".git/kiln-queue.lock").exists(),
                "released before step 8"
            );
            Ok(3)
        };
        let e = run_with(&root, &args(), &clock, &gate, &queue).unwrap_err();
        assert_eq!(e.downcast_ref::<Exit>(), Some(&Exit(3)));
        assert!(seen.get());
        assert_eq!(committed_state(&root), ("queued".into(), "queued".into()));
    }

    #[test]
    fn a_failed_step_two_before_its_commit_leaves_main_as_found() {
        let repo = toy();
        let root = repo.path();
        let before = snapshot(root);
        // A clock that runs backwards: the second transition is earlier than the first.
        struct Back(std::cell::Cell<i64>);
        impl Clock for Back {
            fn now_millis(&self) -> i64 {
                let t = self.0.get();
                self.0.set(t - 1000);
                t
            }
        }
        let clock = Back(std::cell::Cell::new(1_793_000_000_000));
        let gate = hook_gate(&|_| {});
        let e = run_with(root, &args(), &clock, &gate, &no_queue).unwrap_err();
        // The failure's own message is kept (the metrics refusal).
        assert!(refusal(&e).starts_with("metrics: refused: "), "{e:#}");
        assert_eq!(snapshot(root), before);
        assert!(!root.join("metrics").exists());
        assert!(std::fs::read_dir(root.join("tasks")).unwrap().all(|e| {
            !e.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("T-050")
        }));
    }

    #[test]
    fn a_failing_git_add_is_a_commit_failure() {
        // Ruling D-047 (l), step 2: a locked index.
        let repo = toy();
        let root = repo.path();
        let tip = rev(root, "main").unwrap();
        std::fs::write(root.join(".git/index.lock"), "").unwrap();
        let clock = Tick(std::cell::Cell::new(1_793_000_000_000));
        let e = run_with(root, &args(), &clock, &hook_gate(&|_| {}), &no_queue).unwrap_err();
        assert_eq!(e.downcast_ref::<Exit>(), Some(&Exit(2)), "{e:#}");
        assert_eq!(rev(root, "main").unwrap(), tip);
        std::fs::remove_file(root.join(".git/index.lock")).unwrap();
        // Step 6: the gate run locks the index.
        let repo = toy();
        let root = repo.path().to_path_buf();
        let index_lock = root.join(".git/index.lock");
        let during = |_: &Path| std::fs::write(&index_lock, "").unwrap();
        let e = run_with(&root, &args(), &clock, &hook_gate(&during), &no_queue).unwrap_err();
        assert_eq!(e.downcast_ref::<Exit>(), Some(&Exit(2)), "{e:#}");
        std::fs::remove_file(&index_lock).unwrap();
        assert_eq!(
            committed_state(&root),
            ("in_progress".into(), "in_progress".into())
        );
    }

    #[test]
    fn state_follows_the_ledger_where_publish_stops() {
        // Ruling D-047 (n), step 3: the worktree cannot be made (`.worktrees` is a file).
        let repo = toy();
        let root = repo.path().to_path_buf();
        std::fs::write(root.join(".git/info/exclude"), ".worktrees\n").unwrap();
        std::fs::write(root.join(".worktrees"), "not a directory\n").unwrap();
        let clock = Tick(std::cell::Cell::new(1_793_000_000_000));
        let e = run_with(&root, &args(), &clock, &hook_gate(&|_| {}), &no_queue).unwrap_err();
        assert_eq!(e.downcast_ref::<Exit>(), Some(&Exit(1)), "{e:#}");
        assert_eq!(committed_state(&root), ("ready".into(), "ready".into()));
        assert_eq!(ledger_state(&root, "T-050"), "ready");

        // Step 4: the regeneration fails in the worktree only.
        let repo = toy();
        let root = repo.path().to_path_buf();
        let gen_toml = "[[generated]]\npath = \"docs/scorecard.md\"\ncommand = [\"sh\", \"-c\", \"case \\\"$CARGO_TARGET_DIR\\\" in *.worktrees*) echo boom >&2; exit 1;; esac; cat scorecard/items/* evaluations/* > docs/scorecard.md\"]\n";
        crate::git::testrepo::write(&root, "xtask/generated.toml", gen_toml);
        crate::git::testrepo::g(
            &root,
            &["commit", "-q", "-m", "gen", "--", "xtask/generated.toml"],
        );
        let e = run_with(
            &root,
            &args(),
            &clock,
            &hook_gate(&|_| panic!("no gate")),
            &no_queue,
        )
        .unwrap_err();
        assert_eq!(e.downcast_ref::<Exit>(), Some(&Exit(1)), "{e:#}");
        assert_eq!(
            committed_state(&root),
            ("in_progress".into(), "in_progress".into())
        );
        let mut dirty = dirty_paths(&root, None).unwrap();
        dirty.sort();
        assert_eq!(
            dirty,
            [" M evaluations/iter-0.md", "?? scorecard/items/x.y.toml"]
        );

        // Step 7: an input changes during the gate; nothing is removed.
        let repo = toy();
        let root = repo.path().to_path_buf();
        let during = |_: &Path| {
            std::fs::write(root.join("scorecard/items/x.y.toml"), "edited\n").unwrap();
        };
        let e = run_with(&root, &args(), &clock, &hook_gate(&during), &no_queue).unwrap_err();
        assert_eq!(e.downcast_ref::<Exit>(), Some(&Exit(1)), "{e:#}");
        assert_eq!(committed_state(&root), ("queued".into(), "queued".into()));
        // Neither input was removed (the unchanged one is compared before any removal).
        assert_eq!(
            std::fs::read_to_string(root.join("evaluations/iter-0.md")).unwrap(),
            "new\n"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("scorecard/items/x.y.toml")).unwrap(),
            "edited\n"
        );
    }

    #[test]
    fn step_one_refuses_no_input_and_a_dirty_path_outside_the_inputs() {
        // Ruling D-047 (p), findings 6 and 8.
        let repo = toy();
        let root = repo.path();
        let clock = Tick(std::cell::Cell::new(1_793_000_000_000));
        let gate = hook_gate(&|_| {});
        let before = snapshot(root);
        let none = Args {
            inputs: vec![],
            ..args()
        };
        let e = run_with(root, &none, &clock, &gate, &no_queue).unwrap_err();
        assert!(refusal(&e).starts_with("publish: refused: "), "{e:#}");
        assert_eq!(snapshot(root), before);
        crate::git::testrepo::write(root, "notes.txt", "draft\n");
        let before = snapshot(root);
        let e = run_with(root, &args(), &clock, &gate, &no_queue).unwrap_err();
        let r = refusal(&e);
        assert!(
            r.starts_with("publish: refused: ") && r.contains("notes.txt"),
            "{r}"
        );
        assert_eq!(snapshot(root), before);
        assert_eq!(gate.builds.get(), 0);
        assert!(!root.join(".git/kiln-queue.lock").exists());
    }

    #[test]
    fn a_failed_content_commit_in_step_four_exits_one() {
        // Ruling D-047 (q): a stop in step 4 exits 1, a failed commit on task/T-NNN included;
        // exit 2 is the ledger commits' pinned line only.
        let repo = toy();
        let root = repo.path().to_path_buf();
        let hook = root.join(".git/hooks/pre-commit");
        std::fs::create_dir_all(hook.parent().unwrap()).unwrap();
        std::fs::write(
            &hook,
            "#!/bin/sh\ncase \"$(git symbolic-ref --short HEAD)\" in task/*) echo hook says no >&2; exit 1;; esac\n",
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
        let clock = Tick(std::cell::Cell::new(1_793_000_000_000));
        let e = run_with(
            &root,
            &args(),
            &clock,
            &hook_gate(&|_| panic!("no gate")),
            &no_queue,
        )
        .unwrap_err();
        assert_eq!(e.downcast_ref::<Exit>(), Some(&Exit(1)), "{e:#}");
        assert_eq!(
            committed_state(&root),
            ("in_progress".into(), "in_progress".into())
        );
        assert!(
            !root.join(".git/kiln-queue.lock").exists(),
            "released on the stop"
        );
    }

    #[test]
    fn stop_lines() {
        let e = anyhow::anyhow!("x.toml differs\nfrom the branch");
        assert_eq!(
            stop_line(7, "T-050", &e, "queued"),
            "publish: stopped at step 7: x.toml differs; from the branch; T-050 is queued"
        );
    }
}
