//! # xtask — engineering tool (role "tool"; no layer)
//!
//! Responsibility (CHARTER §3/§4): the gate (`cargo xtask ci`), the architecture checks, the
//! scorecard, task worktrees, the serial merge queue, the test join and metrics. It depends on
//! no workspace crate, so a broken product crate cannot stop it from building and reporting.
//! See `docs/development.md` for usage.

mod frontmatter;
mod gate;
mod git;
mod hygiene;
mod join;
mod layers;
mod metrics;
mod noskip;
mod policy;
mod precheck;
mod provenance;
mod queue;
mod registries;
mod schema;
mod scorecard;
mod seed;
mod testbins;
mod timefmt;
mod uicmds;
mod util;
mod worktree;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Result, bail};

use crate::util::{Status, StepResult};

const USAGE: &str = "\
usage: cargo xtask <command>

gate
  ci [--keep-going] [--report <path>]   run every gate step (CHARTER §4 order)
  fmt | clippy | layers | ui-commands | hygiene | test | fuzz | budgets
  no-skip [--today YYYY-MM-DD]          default today: HEAD's commit date
  schema [--write]                      check (or rewrite) docs/commands.json
  scorecard [--check]                   derive statuses; write (or check) docs/scorecard.md

tasks
  worktree new T-NNN [--tests] [--no-seed]   also: worktree new T-NNN-tests
  worktree rm T-NNN [--tests] [--force]
  worktree list
  queue T-NNN                           serial merge queue: rebase, gate, fast-forward main
  join T-NNN [--acc-defects N]          merge task/T-NNN-tests into task/T-NNN, record evidence
  join --revert-check T-NNN [--revert <sha>]...

metrics
  metrics add --task T-NNN --run R-NNN --role <role> --tokens N --tool-calls N
              --gate-attempts N --outcome <outcome> --started-at <ts> --ended-at <ts>
              [--wall-s N] [--iteration N] [--tier T] [--missing-context S] [--note S]
  metrics transition --task T-NNN --from <state> --to <state> --at <ts> [--iteration N]
  metrics report";

fn print_step(name: &str, r: &StepResult) -> bool {
    let label = match r.status {
        Status::Pass => "PASS ",
        Status::Fail => "FAIL ",
        Status::Empty => "EMPTY",
    };
    println!("{label} {name}: {}", r.detail);
    r.status != Status::Fail
}

fn flag(args: &mut Vec<String>, name: &str) -> bool {
    let before = args.len();
    args.retain(|a| a != name);
    args.len() != before
}

fn value(args: &mut Vec<String>, name: &str) -> Result<Option<String>> {
    match args.iter().position(|a| a == name) {
        None => Ok(None),
        Some(i) if i + 1 < args.len() => {
            let v = args.remove(i + 1);
            args.remove(i);
            Ok(Some(v))
        }
        Some(_) => bail!("{name} needs a value"),
    }
}

fn no_more(args: &[String]) -> Result<()> {
    if args.is_empty() {
        Ok(())
    } else {
        bail!("unexpected arguments: {}", args.join(" "))
    }
}

fn primary() -> Result<PathBuf> {
    git::primary_worktree(&util::repo_root()?)
}

fn real_main() -> Result<bool> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        println!("{USAGE}");
        return Ok(false);
    }
    let cmd = args.remove(0);
    match cmd.as_str() {
        "ci" => {
            let keep_going = flag(&mut args, "--keep-going");
            let report = value(&mut args, "--report")?;
            no_more(&args)?;
            gate::run(
                &util::repo_root()?,
                keep_going,
                report.as_deref().map(Path::new),
            )
        }
        "fmt" | "clippy" | "layers" | "ui-commands" | "hygiene" | "test" | "fuzz" | "budgets" => {
            no_more(&args)?;
            let r = gate::run_step(&util::repo_root()?, &cmd)?;
            Ok(print_step(&cmd, &r))
        }
        "no-skip" => {
            let root = util::repo_root()?;
            let today = match value(&mut args, "--today")? {
                Some(t) => t,
                None => noskip::today(&root)?,
            };
            no_more(&args)?;
            Ok(print_step("no-skip", &noskip::check(&root, &today)?))
        }
        "schema" => {
            let write = flag(&mut args, "--write");
            no_more(&args)?;
            Ok(print_step(
                "schema",
                &schema::run(&util::repo_root()?, write)?,
            ))
        }
        "scorecard" => {
            let check = flag(&mut args, "--check");
            no_more(&args)?;
            Ok(print_step(
                "scorecard",
                &scorecard::run(&util::repo_root()?, check)?,
            ))
        }
        "worktree" => {
            let sub = if args.is_empty() {
                String::new()
            } else {
                args.remove(0)
            };
            let root = primary()?;
            match sub.as_str() {
                "new" => {
                    let tests = flag(&mut args, "--tests");
                    let seed = !flag(&mut args, "--no-seed");
                    let [task] = args.as_slice() else {
                        bail!("usage: worktree new T-NNN [--tests]")
                    };
                    let spec = worktree::Spec::parse(task, tests)?;
                    let info = worktree::create(&root, &spec, seed)?;
                    println!("worktree: {}", info.path.display());
                    println!("branch:   {}", info.branch);
                    println!(
                        "target:   {} ({})",
                        info.target.display(),
                        match &info.seeded {
                            Some(s) => format!(
                                "seeded with {} files ({} MB) of non-member artifacts from main's target/",
                                s.files,
                                s.bytes / 1_000_000
                            ),
                            None => "empty".into(),
                        }
                    );
                    println!(
                        "\nuse:\n  cd {}\n  export CARGO_TARGET_DIR={}",
                        info.path.display(),
                        info.target.display()
                    );
                    Ok(true)
                }
                "rm" => {
                    let tests = flag(&mut args, "--tests");
                    let force = flag(&mut args, "--force");
                    let [task] = args.as_slice() else {
                        bail!("usage: worktree rm T-NNN [--tests] [--force]")
                    };
                    println!(
                        "{}",
                        worktree::remove(&root, &worktree::Spec::parse(task, tests)?, force)?
                    );
                    Ok(true)
                }
                "list" => {
                    no_more(&args)?;
                    println!("{}", worktree::list(&root)?);
                    Ok(true)
                }
                _ => bail!("usage: worktree new|rm|list"),
            }
        }
        "queue" => {
            let [task] = args.as_slice() else {
                bail!("usage: queue T-NNN")
            };
            let r = match queue::run(&primary()?, task, &timefmt::SystemClock, &queue::CargoGate) {
                Ok(r) => r,
                Err(e) => match e.downcast_ref::<queue::Refusal>() {
                    // Exit 1, one line: `queue: refused: <check>: <reason>`.
                    Some(refusal) => {
                        eprintln!("{refusal}");
                        return Ok(false);
                    }
                    None => return Err(e),
                },
            };
            println!(
                "queue {} attempt {}: {}{} (base {}, head {}, gate {:.1}s, gate xtask {} @ {}{}{})",
                r.task,
                r.attempt,
                r.result,
                r.failed_step
                    .as_deref()
                    .map(|s| format!(" at {s}"))
                    .unwrap_or_default(),
                &r.base_sha[..12.min(r.base_sha.len())],
                &r.head_sha[..12.min(r.head_sha.len())],
                r.gate_seconds,
                r.gate_xtask,
                &r.gate_xtask_sha[..12.min(r.gate_xtask_sha.len())],
                if r.regenerated.is_empty() {
                    String::new()
                } else {
                    format!(", regenerated {}", r.regenerated.join(" "))
                },
                if r.gate_paths.is_empty() {
                    String::new()
                } else {
                    format!(", gate paths changed: {}", r.gate_paths.join(" "))
                }
            );
            if r.result == "merged" {
                println!("main fast-forwarded to {} (not pushed)", r.head_sha);
            }
            Ok(r.result == "merged")
        }
        "join" => {
            let root = primary()?;
            if flag(&mut args, "--revert-check") {
                let mut only = Vec::new();
                while let Some(s) = value(&mut args, "--revert")? {
                    only.push(s);
                }
                let [task] = args.as_slice() else {
                    bail!("usage: join --revert-check T-NNN [--revert <sha>]...")
                };
                let r = join::revert_check(
                    &root,
                    task,
                    &only,
                    &timefmt::SystemClock,
                    &join::CargoOracle,
                )?;
                println!("{}", serde_json::to_string_pretty(&r)?);
                if r.result_kind != "fails" {
                    eprintln!(
                        "revert check: acceptance tests do not fail by assertion with the implementation reverted ({})",
                        r.result_kind
                    );
                }
                Ok(r.result_kind == "fails")
            } else {
                let acc_defects = value(&mut args, "--acc-defects")?
                    .map(|v| v.parse::<u64>())
                    .transpose()?;
                let [task] = args.as_slice() else {
                    bail!("usage: join T-NNN [--acc-defects N]")
                };
                let r = join::join(
                    &root,
                    task,
                    acc_defects,
                    &timefmt::SystemClock,
                    &join::CargoOracle,
                    &queue::CargoGate,
                )?;
                println!("{}", serde_json::to_string_pretty(&r)?);
                if r.base_fail_kind != "fails" {
                    eprintln!(
                        "join: base evidence is `{}` (weak); the reviewer must run `cargo xtask join --revert-check {}`",
                        r.base_fail_kind, r.task
                    );
                }
                Ok(r.result == "joined")
            }
        }
        "metrics" => {
            let sub = if args.is_empty() {
                String::new()
            } else {
                args.remove(0)
            };
            let root = primary()?;
            match sub.as_str() {
                "add" => println!("{}", metrics::add(&root, &args)?),
                "transition" => println!("{}", metrics::transition(&root, &args)?),
                "report" => {
                    no_more(&args)?;
                    print!("{}", metrics::report(&root)?);
                }
                _ => bail!("usage: metrics add|transition|report"),
            }
            Ok(true)
        }
        "help" | "--help" | "-h" => {
            println!("{USAGE}");
            Ok(true)
        }
        other => bail!("unknown command `{other}`\n\n{USAGE}"),
    }
}

fn main() -> ExitCode {
    // Refuse a binary built from another checkout than the one it runs in (D-019 (f)).
    if let Err(e) = provenance::check(&provenance::Inputs::current()) {
        eprintln!("xtask: {e}");
        return ExitCode::from(2);
    }
    match real_main() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("xtask: {e:#}");
            ExitCode::from(2)
        }
    }
}
