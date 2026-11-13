//! `cargo xtask metrics add|transition|join-defects|report` over `metrics/tasks.jsonl` and
//! `metrics/queue.jsonl`. Record kinds and field names are the contract of METHODOLOGY §13.
//!
//! Since T-013 (v1.1 rules made mechanical): times default to the clock (`<now>`, injected as a
//! [`Clock`] so unit tests are deterministic; the binary uses the system clock and there is no
//! override); a time later than `<now>` or earlier than the task's latest record is refused; a
//! transition must be in METHODOLOGY §3's table; `--tokens 0` is refused (`--tokens-unknown`
//! records `null`); planner, evaluator, retrospective-analyst and director runs are booked to the
//! pseudo-task `iter-N`. Refusals are one stderr line `metrics: refused: …`, exit 1, nothing
//! appended ([`Refused`]); usage errors stay exit 2.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Result, bail};
use serde::Serialize;
use serde_json::Value;

use crate::timefmt::{Clock, format_utc, parse_utc};
use crate::util::{Refused, append_jsonl, read_jsonl, valid_task_id};

/// Roles a run may have (CHARTER §4).
pub const ROLES: &[&str] = &[
    "director",
    "planner",
    "implementer",
    "test-author",
    "reviewer",
    "integrator",
    "evaluator",
    "retrospective-analyst",
];

/// Roles whose runs are iteration overhead, booked to `iter-N` (METHODOLOGY §13).
pub const OVERHEAD_ROLES: &[&str] = &["director", "planner", "evaluator", "retrospective-analyst"];

/// Task states (CHARTER §4).
pub const STATES: &[&str] = &[
    "draft",
    "specified",
    "ready",
    "in_progress",
    "self_verified",
    "tests_joined",
    "reviewed",
    "queued",
    "merged",
    "evaluated",
    "blocked",
    "abandoned",
];

/// A `run` record.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RunRecord {
    /// Always `run`.
    pub kind: &'static str,
    /// `T-NNN` or `iter-N`.
    pub task: String,
    /// Iteration number.
    pub iteration: u64,
    /// `R-NNN`.
    pub run: String,
    /// One of [`ROLES`].
    pub role: String,
    /// low | medium | high; `null` for `iter-N`.
    pub tier: Option<String>,
    /// UTC start.
    pub started_at: String,
    /// UTC end.
    pub ended_at: String,
    /// Wall seconds.
    pub wall_s: u64,
    /// Tokens used; `null` when the run's usage is unknown (`--tokens-unknown`).
    pub tokens: Option<u64>,
    /// Tool calls made.
    pub tool_calls: u64,
    /// Runs of `cargo xtask ci`.
    pub gate_attempts: u64,
    /// `completed`, `blocked`, `incomplete`, …
    pub outcome: String,
    /// What the run lacked (`none` if nothing).
    pub missing_context: String,
    /// Free text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// A `transition` record.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TransitionRecord {
    /// Always `transition`.
    pub kind: &'static str,
    /// `T-NNN`.
    pub task: String,
    /// Iteration number.
    pub iteration: u64,
    /// UTC time.
    pub at: String,
    /// Previous state.
    pub from: String,
    /// New state.
    pub to: String,
}

/// A `join_defects` record (T-013 D-3): the director's classification of a failing join,
/// appended; the `join` record itself is never edited.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct JoinDefectsRecord {
    /// Always `join_defects`.
    pub kind: &'static str,
    /// `T-NNN`.
    pub task: String,
    /// UTC time.
    pub at: String,
    /// The `joined_sha` of the classified join.
    pub joined_sha: String,
    /// Failures of the first join that were real defects.
    pub acc_defects: u64,
}

/// Front matter of `tasks/T-NNN-*.md` that metrics use.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TaskMeta {
    /// Tier.
    pub tier: Option<String>,
    /// Iteration.
    pub iteration: Option<u64>,
    /// Budget.
    pub budget_tokens: Option<u64>,
    /// Who found the defect (`planner` for planned work).
    pub found_by: Option<String>,
    /// The task whose defect this fixes.
    pub origin_task: Option<String>,
}

fn meta_of(text: &str) -> Result<TaskMeta> {
    let (fm, _) = crate::frontmatter::parse(text)?;
    let s = |k: &str| fm.get(k).and_then(|v| v.as_str()).map(str::to_owned);
    let num = |k: &str| s(k).and_then(|v| v.parse().ok());
    Ok(TaskMeta {
        tier: s("tier"),
        iteration: num("iteration"),
        budget_tokens: num("budget_tokens"),
        found_by: s("found_by"),
        origin_task: s("origin_task"),
    })
}

/// Read every task file's metadata, keyed by its `id`.
pub fn task_metas(root: &Path) -> Result<BTreeMap<String, TaskMeta>> {
    let mut out = BTreeMap::new();
    let dir = root.join("tasks");
    if !dir.is_dir() {
        return Ok(out);
    }
    for e in std::fs::read_dir(dir)? {
        let p = e?.path();
        let name = p
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned();
        if !name.starts_with("T-") || !name.ends_with(".md") {
            continue;
        }
        let text = std::fs::read_to_string(&p)?;
        let (fm, _) = crate::frontmatter::parse(&text)
            .map_err(|e| anyhow::anyhow!("{}: {e}", p.display()))?;
        let Some(id) = fm.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        out.insert(id.to_owned(), meta_of(&text)?);
    }
    Ok(out)
}

fn refused(msg: impl std::fmt::Display) -> anyhow::Error {
    Refused(format!("metrics: refused: {msg}")).into()
}

/// What a record is booked to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Subject {
    /// A task `T-NNN`.
    Task(String),
    /// Iteration N's overhead, `iter-N`.
    Iter(u64),
}

impl Subject {
    /// Parse `T-NNN` or `iter-N` (N decimal without leading zeros).
    pub fn parse(s: &str) -> Result<Self> {
        if let Some(n) = s.strip_prefix("iter-")
            && !n.is_empty()
            && n.bytes().all(|b| b.is_ascii_digit())
            && (n == "0" || !n.starts_with('0'))
            && let Ok(n) = n.parse()
        {
            return Ok(Subject::Iter(n));
        }
        if valid_task_id(s) {
            return Ok(Subject::Task(s.to_owned()));
        }
        Err(refused(format_args!(
            "`{s}` is not a task id T-NNN or an iteration pseudo-task iter-N"
        )))
    }
    fn name(&self) -> String {
        match self {
            Subject::Task(t) => t.clone(),
            Subject::Iter(n) => format!("iter-{n}"),
        }
    }
}

/// The metadata of the one `tasks/<task>-*.md` in the working tree (refused otherwise).
pub fn task_file_meta(root: &Path, task: &str) -> Result<TaskMeta> {
    let prefix = format!("{task}-");
    let mut found = Vec::new();
    if let Ok(rd) = std::fs::read_dir(root.join("tasks")) {
        for e in rd {
            let p = e?.path();
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            if name.starts_with(&prefix) && name.ends_with(".md") && p.is_file() {
                found.push(p);
            }
        }
    }
    found.sort();
    match found.as_slice() {
        [one] => {
            let text = std::fs::read_to_string(one)?;
            meta_of(&text).map_err(|e| {
                refused(format_args!(
                    "tasks/{}: front matter does not parse: {e}",
                    one.file_name().and_then(|n| n.to_str()).unwrap_or_default()
                ))
            })
        }
        [] => Err(refused(format_args!(
            "no tasks/{task}-*.md in the working tree"
        ))),
        many => Err(refused(format_args!(
            "{} files match tasks/{task}-*.md: {}",
            many.len(),
            many.iter()
                .filter_map(|p| p.file_name().and_then(|n| n.to_str()))
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

/// Parsed `--flag value` options and value-less `--flag`s.
pub struct Opts(BTreeMap<String, String>);

impl Opts {
    /// Parse `--k v` pairs (`known`) and bare `--k` (`flags`); `aliases` maps alternative names
    /// to canonical ones.
    pub fn parse(
        args: &[String],
        known: &[&str],
        flags: &[&str],
        aliases: &[(&str, &str)],
    ) -> Result<Self> {
        let mut m = BTreeMap::new();
        let mut it = args.iter();
        while let Some(a) = it.next() {
            let Some(k) = a.strip_prefix("--") else {
                bail!("unexpected argument `{a}`")
            };
            let k = aliases
                .iter()
                .find(|(al, _)| *al == k)
                .map_or(k, |(_, c)| c);
            let v = if flags.contains(&k) {
                String::new()
            } else if known.contains(&k) {
                let Some(v) = it.next() else {
                    bail!("--{k} needs a value")
                };
                v.clone()
            } else {
                bail!("unknown option --{k}");
            };
            if m.insert(k.to_owned(), v).is_some() {
                bail!("--{k} given twice");
            }
        }
        Ok(Opts(m))
    }
    fn get(&self, k: &str) -> Option<&str> {
        self.0.get(k).map(String::as_str)
    }
    fn has(&self, k: &str) -> bool {
        self.0.contains_key(k)
    }
    fn req(&self, k: &str) -> Result<&str> {
        self.get(k)
            .ok_or_else(|| anyhow::anyhow!("--{k} is required"))
    }
    fn num(&self, k: &str) -> Result<Option<u64>> {
        self.get(k)
            .map(|v| {
                v.parse()
                    .map_err(|_| anyhow::anyhow!("--{k} must be a non-negative integer"))
            })
            .transpose()
    }
    fn req_num(&self, k: &str) -> Result<u64> {
        self.num(k)?
            .ok_or_else(|| anyhow::anyhow!("--{k} is required"))
    }
}

fn normalize_ts(opt: &str, s: &str) -> Result<(i64, String)> {
    let t = parse_utc(s).map_err(|e| anyhow::anyhow!("--{opt}: {e}"))?;
    if !s.contains('T') {
        bail!("--{opt}: timestamp `{s}` needs a time (YYYY-MM-DDTHH:MM:SSZ)");
    }
    Ok((t, format_utc(t)))
}

fn now_secs(clock: &dyn Clock) -> i64 {
    clock.now_millis().div_euclid(1000)
}

/// A record's time: `ended_at` for a run, `at` for every other kind.
fn record_time(r: &Value) -> Option<i64> {
    let k = if r["kind"] == "run" { "ended_at" } else { "at" };
    r[k].as_str().and_then(|s| parse_utc(s).ok())
}

/// The latest time among the records of `task`.
fn latest_time(records: &[Value], task: &str) -> Option<i64> {
    records
        .iter()
        .filter(|r| r["task"].as_str() == Some(task))
        .filter_map(record_time)
        .max()
}

/// Refuse `t` (option `opt`) later than now or earlier than the task's latest record.
fn check_time(opt: &str, t: i64, now: i64, latest: Option<i64>, task: &str) -> Result<()> {
    if t > now {
        return Err(refused(format_args!(
            "--{opt} {} is later than now ({})",
            format_utc(t),
            format_utc(now)
        )));
    }
    if let Some(l) = latest
        && t < l
    {
        return Err(refused(format_args!(
            "--{opt} {} is earlier than {task}'s latest record ({})",
            format_utc(t),
            format_utc(l)
        )));
    }
    Ok(())
}

/// The options of `metrics add`.
const ADD_OPTS: &[&str] = &[
    "task",
    "run",
    "role",
    "tier",
    "iteration",
    "started-at",
    "ended-at",
    "wall-s",
    "tokens",
    "tool-calls",
    "gate-attempts",
    "outcome",
    "missing-context",
    "note",
];
const ADD_FLAGS: &[&str] = &["tokens-unknown"];
const ADD_ALIASES: &[(&str, &str)] = &[("seconds", "wall-s"), ("tool-uses", "tool-calls")];

/// Build a run record (pure apart from `task_meta`, which reads the task file of a `T-NNN`):
/// checks in the order of T-013 Pinned (a) after option parsing.
pub fn run_record(
    o: &Opts,
    task_meta: &dyn Fn(&str) -> Result<TaskMeta>,
    records: &[Value],
    now: i64,
) -> Result<RunRecord> {
    // Options parse (usage, exit 2): required options and value forms.
    let task_arg = o.req("task")?;
    let run = o.req("run")?.to_owned();
    if !valid_task_id(&run.replacen("R-", "T-", 1)) || !run.starts_with("R-") {
        bail!("--run `{run}` must look like R-NNN");
    }
    let role = o.req("role")?.to_owned();
    if !ROLES.contains(&role.as_str()) {
        bail!("--role `{role}` is not one of {}", ROLES.join(", "));
    }
    let tokens_opt = o.num("tokens")?;
    let tool_calls = o.req_num("tool-calls")?;
    let gate_attempts = o.req_num("gate-attempts")?;
    let outcome = o.req("outcome")?.to_owned();
    if outcome.trim().is_empty() {
        bail!("--outcome is empty");
    }
    let started_raw = o.req("started-at")?;
    let (s, started_at) = normalize_ts("started-at", started_raw)?;
    let ended = o
        .get("ended-at")
        .map(|e| normalize_ts("ended-at", e))
        .transpose()?;
    let iteration_opt = o.num("iteration")?;
    let wall_opt = o.num("wall-s")?;
    if let Some(t) = o.get("tier")
        && !matches!(t, "low" | "medium" | "high")
    {
        bail!("--tier `{t}` is not low|medium|high");
    }

    // Task id form and task file / iter-N role rules.
    let subject = Subject::parse(task_arg)?;
    let (task, tier, iteration) = match &subject {
        Subject::Iter(n) => {
            if !OVERHEAD_ROLES.contains(&role.as_str()) {
                return Err(refused(format_args!(
                    "a {role} run cannot be booked to iter-{n}; iteration overhead is {} runs only (METHODOLOGY §13)",
                    OVERHEAD_ROLES.join(", ")
                )));
            }
            if let Some(i) = iteration_opt
                && i != *n
            {
                return Err(refused(format_args!(
                    "--iteration {i} does not match iter-{n}"
                )));
            }
            if o.has("tier") {
                return Err(refused(format_args!(
                    "iter-{n} runs have no tier (tier is null); drop --tier"
                )));
            }
            (subject.name(), None, *n)
        }
        Subject::Task(t) => {
            let meta = task_meta(t)?;
            let iteration = match iteration_opt.or(meta.iteration) {
                Some(i) => i,
                None => {
                    return Err(refused(format_args!(
                        "tasks/{t}-*.md has no `iteration` and --iteration is not given"
                    )));
                }
            };
            if OVERHEAD_ROLES.contains(&role.as_str()) {
                return Err(refused(format_args!(
                    "a {role} run is iteration overhead: book it to iter-{iteration}, not {t} (METHODOLOGY §13)"
                )));
            }
            let tier = match o.get("tier").map(str::to_owned).or(meta.tier) {
                Some(x) if matches!(x.as_str(), "low" | "medium" | "high") => x,
                Some(x) => {
                    return Err(refused(format_args!(
                        "tasks/{t}-*.md has tier `{x}`, not low|medium|high"
                    )));
                }
                None => {
                    return Err(refused(format_args!(
                        "tasks/{t}-*.md has no `tier` and --tier is not given"
                    )));
                }
            };
            (t.clone(), Some(tier), iteration)
        }
    };

    // Tokens rule.
    let unknown = o.has("tokens-unknown");
    let tokens = match (tokens_opt, unknown) {
        (Some(_), true) => {
            return Err(refused(
                "give exactly one of --tokens N and --tokens-unknown, not both",
            ));
        }
        (None, false) => {
            return Err(refused(
                "give exactly one of --tokens N (N ≥ 1) and --tokens-unknown",
            ));
        }
        (Some(0), false) => {
            return Err(refused(
                "--tokens 0: a run used tokens; give --tokens N (N ≥ 1) or --tokens-unknown",
            ));
        }
        (Some(n), false) => Some(n),
        (None, true) => None,
    };

    // Time rules.
    let (e, ended_at) = ended.unwrap_or_else(|| (now, format_utc(now)));
    if s > now {
        return Err(refused(format_args!(
            "--started-at {started_at} is later than now ({})",
            format_utc(now)
        )));
    }
    check_time("ended-at", e, now, latest_time(records, &task), &task)?;
    if e < s {
        return Err(refused(format_args!(
            "--started-at {started_at} is later than ended_at {ended_at}"
        )));
    }
    let wall_s = wall_opt.unwrap_or_else(|| u64::try_from(e - s).unwrap_or(0));
    Ok(RunRecord {
        kind: "run",
        task,
        iteration,
        run,
        role,
        tier,
        started_at,
        ended_at,
        wall_s,
        tokens,
        tool_calls,
        gate_attempts,
        outcome,
        missing_context: o.get("missing-context").unwrap_or("none").to_owned(),
        note: o.get("note").map(str::to_owned),
    })
}

/// The budget lines printed after a successful `add` (T-013 Interface (1)).
pub fn budget_lines(
    task: &str,
    records: &[Value],
    budget: Option<u64>,
    extension: Option<u64>,
) -> Vec<String> {
    let (mut sum, mut unknown) = (0u64, 0usize);
    for r in records
        .iter()
        .filter(|r| r["kind"] == "run" && r["task"].as_str() == Some(task))
    {
        match r["tokens"].as_u64() {
            Some(t) => sum = sum.saturating_add(t),
            None => unknown += 1,
        }
    }
    let mut out = Vec::new();
    if task.starts_with("iter-") {
        out.push(format!("{task}: {sum} tokens of overhead"));
        return out;
    }
    match budget {
        Some(b) => out.push(format!("{task}: {sum} tokens of budget {b}")),
        None => out.push(format!("{task}: {sum} tokens of budget none")),
    }
    if let Some(ceiling) = extension.or(budget)
        && sum > ceiling
    {
        out.push(format!(
            "{task}: over its ceiling {ceiling} by {} tokens; the queue refuses it without a docs/decisions.md row \"Extend {task} to <N>\" with N ≥ {sum}",
            sum - ceiling
        ));
    }
    if unknown > 0 {
        out.push(format!(
            "{task}: {unknown} run(s) with unknown tokens not counted"
        ));
    }
    out
}

/// The working tree's `docs/decisions.md` (missing: empty).
fn decisions_text(root: &Path) -> String {
    std::fs::read_to_string(root.join("docs/decisions.md")).unwrap_or_default()
}

/// `metrics add …`: appends a run record; returns the record and the budget lines.
pub fn add(root: &Path, args: &[String], clock: &dyn Clock) -> Result<String> {
    let o = Opts::parse(args, ADD_OPTS, ADD_FLAGS, ADD_ALIASES)?;
    let path = root.join("metrics/tasks.jsonl");
    let mut records = read_jsonl(&path)?;
    let rec = run_record(&o, &|t| task_file_meta(root, t), &records, now_secs(clock))?;
    append_jsonl(&path, &rec)?;
    let mut out = serde_json::to_string(&rec)?;
    records.push(serde_json::to_value(&rec)?);
    let (budget, extension) = match &rec.task {
        t if t.starts_with("iter-") => (None, None),
        t => (
            task_file_meta(root, t)?.budget_tokens,
            crate::precheck::extension(&decisions_text(root), t),
        ),
    };
    for l in budget_lines(&rec.task, &records, budget, extension) {
        out.push('\n');
        out.push_str(&l);
    }
    Ok(out)
}

/// The task's current state (the `to` of its latest transition, else `draft`), the time it
/// was entered, and, if it is `blocked`, the state it was blocked from.
pub fn current_state(records: &[Value], task: &str) -> (String, Option<String>, Option<String>) {
    let mut state = ("draft".to_owned(), None, None);
    for r in records
        .iter()
        .filter(|r| r["kind"] == "transition" && r["task"].as_str() == Some(task))
    {
        let to = r["to"].as_str().unwrap_or_default().to_owned();
        let from = r["from"].as_str().map(str::to_owned);
        let blocked_from = if to == "blocked" { from } else { None };
        state = (to, r["at"].as_str().map(str::to_owned), blocked_from);
    }
    state
}

/// Whether METHODOLOGY §3 allows `from -> to` for a task of `tier` (T-013 Interface (1)).
pub fn transition_allowed(from: &str, to: &str, tier: &str, blocked_from: Option<&str>) -> bool {
    let tested = matches!(tier, "medium" | "high");
    let terminal = matches!(from, "evaluated" | "abandoned");
    if terminal {
        return false;
    }
    if from == "blocked" {
        return to == "abandoned" || blocked_from == Some(to);
    }
    if matches!(to, "blocked" | "abandoned") {
        return from != "merged";
    }
    match (from, to) {
        ("draft", "specified") => true,
        ("specified", "ready" | "draft") => true,
        ("ready", "in_progress") => true,
        ("in_progress", "self_verified") => true,
        ("self_verified", "queued") => tier == "low",
        ("self_verified", "tests_joined" | "in_progress") => tested,
        ("tests_joined", "reviewed" | "in_progress") => tested,
        ("reviewed", "queued") => tested,
        ("queued", "merged" | "in_progress") => true,
        ("merged", "evaluated") => true,
        _ => false,
    }
}

/// Build a transition record (pure apart from `task_meta`).
pub fn transition_record(
    o: &Opts,
    task_meta: &dyn Fn(&str) -> Result<TaskMeta>,
    records: &[Value],
    now: i64,
) -> Result<TransitionRecord> {
    let task_arg = o.req("task")?;
    let to = o.req("to")?.to_owned();
    let from_opt = o.get("from").map(str::to_owned);
    for s in std::iter::once(&to).chain(from_opt.as_ref()) {
        if !STATES.contains(&s.as_str()) {
            bail!("state `{s}` is not one of {}", STATES.join(", "));
        }
    }
    let iteration_opt = o.num("iteration")?;
    let at = o.get("at").map(|a| normalize_ts("at", a)).transpose()?;
    // id
    let task = match Subject::parse(task_arg)? {
        Subject::Task(t) => t,
        Subject::Iter(n) => {
            return Err(refused(format_args!(
                "iter-{n} is iteration overhead, not a task: it has no states"
            )));
        }
    };
    // task file
    let meta = task_meta(&task)?;
    let tier = meta.tier.clone().unwrap_or_default();
    let iteration = match iteration_opt.or(meta.iteration) {
        Some(i) => i,
        None => {
            return Err(refused(format_args!(
                "tasks/{task}-*.md has no `iteration` and --iteration is not given"
            )));
        }
    };
    // --from matches current
    let (current, since, blocked_from) = current_state(records, &task);
    let since = since.map_or_else(
        || "no transition recorded".to_owned(),
        |s| format!("since {s}"),
    );
    if let Some(f) = &from_opt
        && *f != current
    {
        return Err(refused(format_args!(
            "{task} is {current} ({since}), not {f} (--from {f})"
        )));
    }
    // table
    if !transition_allowed(&current, &to, &tier, blocked_from.as_deref()) {
        return Err(refused(format_args!(
            "{task} is {current} ({since}); {current} -> {to} is not allowed (METHODOLOGY §3)"
        )));
    }
    // time
    let (t, at) = at.unwrap_or_else(|| (now, format_utc(now)));
    check_time("at", t, now, latest_time(records, &task), &task)?;
    Ok(TransitionRecord {
        kind: "transition",
        task,
        iteration,
        at,
        from: current,
        to,
    })
}

/// `metrics transition …`.
pub fn transition(root: &Path, args: &[String], clock: &dyn Clock) -> Result<String> {
    let o = Opts::parse(args, &["task", "from", "to", "at", "iteration"], &[], &[])?;
    let path = root.join("metrics/tasks.jsonl");
    let records = read_jsonl(&path)?;
    let rec = transition_record(&o, &|t| task_file_meta(root, t), &records, now_secs(clock))?;
    append_jsonl(&path, &rec)?;
    Ok(serde_json::to_string(&rec)?)
}

/// Build a `join_defects` record (pure apart from `task_meta`).
pub fn join_defects_record(
    o: &Opts,
    task_meta: &dyn Fn(&str) -> Result<TaskMeta>,
    records: &[Value],
    now: i64,
) -> Result<JoinDefectsRecord> {
    let task_arg = o.req("task")?;
    let sha = o.req("joined-sha")?.to_owned();
    let n = o.req_num("acc-defects")?;
    let task = match Subject::parse(task_arg)? {
        Subject::Task(t) => t,
        Subject::Iter(i) => {
            return Err(refused(format_args!(
                "iter-{i} is iteration overhead, not a task: it has no joins"
            )));
        }
    };
    task_meta(&task)?;
    if !crate::precheck::is_full_sha(&sha) {
        return Err(refused(format_args!(
            "--joined-sha `{sha}` is not a full 40-hex lowercase commit id"
        )));
    }
    let joins: Vec<&Value> = records
        .iter()
        .filter(|r| {
            r["kind"] == "join"
                && r["task"].as_str() == Some(task.as_str())
                && r["joined_sha"].as_str() == Some(sha.as_str())
        })
        .collect();
    let j = match joins.as_slice() {
        [one] => *one,
        [] => {
            return Err(refused(format_args!(
                "no join record of {task} has joined_sha {sha}"
            )));
        }
        many => {
            return Err(refused(format_args!(
                "{} join records of {task} have joined_sha {sha}",
                many.len()
            )));
        }
    };
    if let Some(d) = j["acc_defects"].as_u64() {
        return Err(refused(format_args!(
            "the join of {task} at {sha} already has acc_defects {d}"
        )));
    }
    if records.iter().any(|r| {
        r["kind"] == "join_defects"
            && r["task"].as_str() == Some(task.as_str())
            && r["joined_sha"].as_str() == Some(sha.as_str())
    }) {
        return Err(refused(format_args!(
            "the join of {task} at {sha} is already classified by a join_defects record"
        )));
    }
    let failures = j["acc_fail_first_join"].as_u64().unwrap_or(0);
    if failures == 0 {
        return Err(refused(format_args!(
            "the join of {task} at {sha} has acc_fail_first_join 0: nothing to classify"
        )));
    }
    if n > failures {
        return Err(refused(format_args!(
            "--acc-defects {n} exceeds the join's acc_fail_first_join {failures}"
        )));
    }
    Ok(JoinDefectsRecord {
        kind: "join_defects",
        task,
        at: format_utc(now),
        joined_sha: sha,
        acc_defects: n,
    })
}

/// `metrics join-defects …`.
pub fn join_defects(root: &Path, args: &[String], clock: &dyn Clock) -> Result<String> {
    let o = Opts::parse(args, &["task", "joined-sha", "acc-defects"], &[], &[])?;
    let path = root.join("metrics/tasks.jsonl");
    let records = read_jsonl(&path)?;
    let rec = join_defects_record(&o, &|t| task_file_meta(root, t), &records, now_secs(clock))?;
    append_jsonl(&path, &rec)?;
    Ok(serde_json::to_string(&rec)?)
}

fn median(v: &mut [f64]) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    let n = v.len();
    Some(if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    })
}

/// The arm of a task (T-013 Interface (4)); `none` for low or unknown tiers.
fn arm_label(task: &str, tier: &str) -> &'static str {
    crate::hygiene::arm(task, tier).unwrap_or("none")
}

/// Known and unknown token sums.
#[derive(Clone, Copy, Debug, Default)]
struct Tok {
    known: u64,
    unknown: usize,
}

impl Tok {
    fn add(&mut self, r: &Value) {
        match r["tokens"].as_u64() {
            Some(t) => self.known = self.known.saturating_add(t),
            None => self.unknown += 1,
        }
    }
    fn unknown_note(&self, sep: &str) -> String {
        if self.unknown == 0 {
            String::new()
        } else {
            format!("{sep}{} run(s) with unknown tokens excluded", self.unknown)
        }
    }
}

/// The report text and whether it is complete (no unclassified join): pure.
pub fn report_text(
    tasks: &[Value],
    queue: &[Value],
    metas: &BTreeMap<String, TaskMeta>,
    decisions: &str,
) -> (String, bool) {
    let s = |v: &Value, k: &str| v[k].as_str().unwrap_or_default().to_owned();
    let n = |v: &Value, k: &str| v[k].as_u64().unwrap_or(0);
    let is_iter = |t: &str| t.starts_with("iter-");
    let runs: Vec<&Value> = tasks.iter().filter(|r| r["kind"] == "run").collect();
    let mut out = String::new();

    let mut tier_of: BTreeMap<String, String> = BTreeMap::new();
    for r in &runs {
        let t = s(r, "task");
        if !is_iter(&t) && !s(r, "tier").is_empty() {
            tier_of.entry(t).or_insert_with(|| s(r, "tier"));
        }
    }
    for (t, m) in metas {
        if let Some(tier) = &m.tier {
            tier_of.insert(t.clone(), tier.clone());
        }
    }
    let tier = |t: &str| tier_of.get(t).cloned().unwrap_or_else(|| "unknown".into());

    let mut tokens_by_tier: BTreeMap<String, (Tok, BTreeSet<String>)> = BTreeMap::new();
    let mut tokens_by_task: BTreeMap<String, Tok> = BTreeMap::new();
    let mut overhead: BTreeMap<u64, (Tok, usize)> = BTreeMap::new();
    let mut gate_by_task: BTreeMap<String, u64> = BTreeMap::new();
    for r in &runs {
        let t = s(r, "task");
        if is_iter(&t) {
            let e = tokens_by_tier.entry("overhead".into()).or_default();
            e.0.add(r);
            e.1.insert(t.clone());
            let it = t
                .strip_prefix("iter-")
                .and_then(|i| i.parse().ok())
                .unwrap_or_else(|| n(r, "iteration"));
            let o = overhead.entry(it).or_default();
            o.0.add(r);
            o.1 += 1;
            continue;
        }
        let tr = if s(r, "tier").is_empty() {
            tier(&t)
        } else {
            s(r, "tier")
        };
        let e = tokens_by_tier.entry(tr).or_default();
        e.0.add(r);
        e.1.insert(t.clone());
        tokens_by_task.entry(t.clone()).or_default().add(r);
        *gate_by_task.entry(t).or_default() += n(r, "gate_attempts");
    }
    let overhead_runs: usize = overhead.values().map(|(_, c)| c).sum();
    out.push_str(&format!(
        "runs: {} over {} tasks{}\n",
        runs.len(),
        tokens_by_task.len(),
        if overhead_runs > 0 {
            format!(" and {overhead_runs} iter-N overhead run(s)")
        } else {
            String::new()
        }
    ));
    out.push_str("tokens by tier (all runs; iter-N runs as overhead):\n");
    for (tr, (tok, ts)) in &tokens_by_tier {
        out.push_str(&format!(
            "  {tr:<7} {:>10} tokens, {} task(s){}\n",
            tok.known,
            ts.len(),
            tok.unknown_note("; ")
        ));
    }

    // Queue measures.
    let mut first: BTreeMap<String, String> = BTreeMap::new();
    let mut merged_at: BTreeMap<String, i64> = BTreeMap::new();
    for q in queue {
        let t = s(q, "task");
        if n(q, "attempt") == 1 {
            first.insert(t.clone(), s(q, "result"));
        }
        if q["result"] == "merged"
            && let Ok(at) = parse_utc(&s(q, "ended_at"))
        {
            merged_at.entry(t).or_insert(at);
        }
    }
    let merged: BTreeSet<String> = queue
        .iter()
        .filter(|q| q["result"] == "merged")
        .map(|q| s(q, "task"))
        .collect();
    let queued: BTreeSet<String> = queue.iter().map(|q| s(q, "task")).collect();
    let first_pass = first.values().filter(|r| *r == "merged").count();
    if queued.is_empty() {
        out.push_str("first-pass queue success: no queue runs yet\n");
    } else {
        out.push_str(&format!(
            "first-pass queue success: {first_pass}/{} tasks ({:.0}%); queue runs {}; merged tasks {}\n",
            queued.len(),
            100.0 * first_pass as f64 / queued.len() as f64,
            queue.len(),
            merged.len()
        ));
    }

    // Tokens per merged task, without overhead (by tier, by arm) and with overhead.
    let mut by_tier: BTreeMap<String, (Tok, u64)> = BTreeMap::new();
    let mut by_arm: BTreeMap<String, (Tok, u64)> = BTreeMap::new();
    let mut by_iter: BTreeMap<u64, (Tok, u64)> = BTreeMap::new();
    for t in &merged {
        let tok = tokens_by_task.get(t).copied().unwrap_or_default();
        let tr = tier(t);
        for (map, key) in [
            (&mut by_tier, tr.clone()),
            (&mut by_arm, arm_label(t, &tr).to_owned()),
        ] {
            let e = map.entry(key).or_default();
            e.0.known += tok.known;
            e.0.unknown += tok.unknown;
            e.1 += 1;
        }
        let it = metas.get(t).and_then(|m| m.iteration).or_else(|| {
            runs.iter()
                .find(|r| r["task"].as_str() == Some(t.as_str()))
                .and_then(|r| r["iteration"].as_u64())
        });
        if let Some(it) = it {
            let e = by_iter.entry(it).or_default();
            e.0.known += tok.known;
            e.0.unknown += tok.unknown;
            e.1 += 1;
        }
    }
    for (title, map) in [("tier", &by_tier), ("arm", &by_arm)] {
        out.push_str(&format!(
            "tokens per merged task by {title} (without overhead):\n"
        ));
        if map.is_empty() {
            out.push_str("  (no merged tasks)\n");
        }
        for (k, (tok, cnt)) in map {
            out.push_str(&format!(
                "  {k:<7} {:>10.0} (n={cnt}{})\n",
                tok.known as f64 / *cnt as f64,
                tok.unknown_note("; ")
            ));
        }
    }
    out.push_str("tokens per merged task with iter-N overhead, by iteration:\n");
    let iters: BTreeSet<u64> = by_iter.keys().chain(overhead.keys()).copied().collect();
    if iters.is_empty() {
        out.push_str("  (no merged tasks)\n");
    }
    for it in iters {
        let (tok, cnt) = by_iter.get(&it).copied().unwrap_or_default();
        let (otok, oruns) = overhead.get(&it).copied().unwrap_or_default();
        let unknown = Tok {
            known: 0,
            unknown: tok.unknown + otok.unknown,
        };
        if cnt == 0 {
            out.push_str(&format!(
                "  iteration {it} with overhead: no merged tasks ({} overhead tokens over {oruns} iter-{it} run(s){})\n",
                otok.known,
                unknown.unknown_note("; ")
            ));
        } else {
            out.push_str(&format!(
                "  iteration {it} with overhead: {:.0} (n={cnt}; tasks {} + overhead {} tokens over {oruns} iter-{it} run(s){})\n",
                (tok.known + otok.known) as f64 / cnt as f64,
                tok.known,
                otok.known,
                unknown.unknown_note("; ")
            ));
        }
    }

    // Lead time: transition -> ready until merged.
    let mut ready_at: BTreeMap<String, i64> = BTreeMap::new();
    for t in tasks
        .iter()
        .filter(|r| r["kind"] == "transition" && r["to"] == "ready")
    {
        if let Ok(at) = parse_utc(&s(t, "at")) {
            let e = ready_at.entry(s(t, "task")).or_insert(at);
            *e = (*e).min(at);
        }
    }
    let mut leads: Vec<f64> = merged_at
        .iter()
        .filter_map(|(t, m)| ready_at.get(t).map(|r| (m - r) as f64 / 3600.0))
        .collect();
    let without_ready = merged_at
        .keys()
        .filter(|t| !ready_at.contains_key(*t))
        .count();
    match (median(&mut leads), leads.last().copied()) {
        (Some(med), Some(max)) => out.push_str(&format!(
            "lead time (ready -> merged): median {med:.2} h, max {max:.2} h (n={})",
            leads.len()
        )),
        _ => out.push_str("lead time (ready -> merged): no data"),
    }
    if without_ready > 0 {
        out.push_str(&format!(
            "; {without_ready} merged task(s) lack a ready transition"
        ));
    }
    out.push('\n');

    if !gate_by_task.is_empty() {
        let total: u64 = gate_by_task.values().sum();
        out.push_str(&format!(
            "gate attempts per task: mean {:.1} (total {total})\n",
            total as f64 / gate_by_task.len() as f64
        ));
    }

    // Joins. A join's classification: its own acc_defects, else its join_defects record.
    let mut classified: BTreeMap<(String, String), u64> = BTreeMap::new();
    for r in tasks.iter().filter(|r| r["kind"] == "join_defects") {
        if let Some(d) = r["acc_defects"].as_u64() {
            classified
                .entry((s(r, "task"), s(r, "joined_sha")))
                .or_insert(d);
        }
    }
    let class = |j: &Value| {
        j["acc_defects"]
            .as_u64()
            .or_else(|| classified.get(&(s(j, "task"), s(j, "joined_sha"))).copied())
    };
    let joins: Vec<&Value> = tasks.iter().filter(|r| r["kind"] == "join").collect();
    let mut seen = BTreeSet::new();
    let mut kinds: BTreeMap<String, u64> = BTreeMap::new();
    for j in &joins {
        if !seen.insert(s(j, "task")) {
            continue; // first join per task only
        }
        let kind = s(j, "base_fail_kind");
        let kind = if kind.is_empty() {
            "unrecorded".to_owned()
        } else {
            kind
        };
        *kinds.entry(kind).or_default() += 1;
    }
    let mut caught = 0u64;
    let mut caught_tier: BTreeMap<String, u64> = BTreeMap::new();
    let mut caught_arm: BTreeMap<String, u64> = BTreeMap::new();
    let mut green = 0;
    for j in joins
        .iter()
        .filter(|j| j["impl_tests_green"] == Value::Bool(true))
    {
        green += 1;
        let d = class(j).unwrap_or(0);
        let t = s(j, "task");
        let tr = tier(&t);
        caught += d;
        *caught_arm.entry(arm_label(&t, &tr).to_owned()).or_default() += d;
        *caught_tier.entry(tr).or_default() += d;
    }
    let mut unclassified = Vec::new();
    for j in &joins {
        let t = s(j, "task");
        let it = metas.get(&t).and_then(|m| m.iteration);
        let k = n(j, "acc_fail_first_join");
        if it.is_some_and(|i| i >= 1) && k > 0 && class(j).is_none() {
            unclassified.push(format!(
                "unclassified join: {t} at {} joined_sha {}: {k} failure(s), acc_defects missing",
                s(j, "at"),
                s(j, "joined_sha")
            ));
        }
    }
    if seen.is_empty() {
        out.push_str("joins: none yet\n");
    } else {
        let k: Vec<String> = kinds.iter().map(|(k, v)| format!("{k} {v}")).collect();
        out.push_str(&format!(
            "first joins: {} (base evidence: {})\n",
            seen.len(),
            k.join(", ")
        ));
    }
    out.push_str(&format!(
        "acceptance-caught defects: {caught} (over {green} join(s) with impl_tests_green)\n"
    ));
    for (k, v) in &caught_tier {
        out.push_str(&format!("  tier {k}: {v}\n"));
    }
    for (k, v) in &caught_arm {
        out.push_str(&format!("  arm {k}: {v}\n"));
    }
    for u in &unclassified {
        out.push_str(u);
        out.push('\n');
    }

    // Known defects merged by decision: follow-ups filed from a review of a merged task.
    let known: Vec<String> = metas
        .iter()
        .filter(|(_, m)| m.found_by.as_deref() == Some("reviewer"))
        .filter_map(|(t, m)| {
            let o = m.origin_task.as_deref()?;
            merged.contains(o).then(|| format!("{t} (from {o})"))
        })
        .collect();
    out.push_str(&format!(
        "known defects merged by decision: {}\n",
        known.len()
    ));
    for k in &known {
        out.push_str(&format!("  {k}\n"));
    }

    // Budget overruns.
    let mut over = Vec::new();
    let mut detail = Vec::new();
    let mut with_budget = 0;
    for (t, m) in metas {
        if let Some(b) = m.budget_tokens {
            with_budget += 1;
            let used = tokens_by_task.get(t).map_or(0, |x| x.known);
            if used > b {
                over.push(format!("{t} {used}/{b}"));
                let ext = crate::precheck::extension(decisions, t);
                detail.push(format!(
                    "  {t} {used}/{b}: extend to: {}, within ceiling: {}",
                    ext.map_or_else(|| "none".to_owned(), |e| e.to_string()),
                    if ext.is_some_and(|e| used <= e) {
                        "yes"
                    } else {
                        "no"
                    }
                ));
            }
        }
    }
    out.push_str(&format!(
        "budget overruns: {}/{with_budget} tasks with a budget{}\n",
        over.len(),
        if over.is_empty() {
            String::new()
        } else {
            format!(" ({})", over.join(", "))
        }
    ));
    for d in detail {
        out.push_str(&d);
        out.push('\n');
    }
    (out, unclassified.is_empty())
}

/// `metrics report`: the text, and whether every failing join of iteration ≥ 1 is classified.
pub fn report(root: &Path) -> Result<(String, bool)> {
    let tasks = read_jsonl(&root.join("metrics/tasks.jsonl"))?;
    let queue = read_jsonl(&root.join("metrics/queue.jsonl"))?;
    Ok(report_text(
        &tasks,
        &queue,
        &task_metas(root)?,
        &decisions_text(root),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    fn opts(s: &str) -> Opts {
        Opts::parse(&args(s), ADD_OPTS, ADD_FLAGS, ADD_ALIASES).unwrap()
    }

    fn meta(tier: &str, iteration: u64, budget: u64) -> TaskMeta {
        TaskMeta {
            tier: Some(tier.into()),
            iteration: Some(iteration),
            budget_tokens: Some(budget),
            ..TaskMeta::default()
        }
    }

    fn metas_fn(m: TaskMeta) -> impl Fn(&str) -> Result<TaskMeta> {
        move |t: &str| {
            if t == "T-001" || t == "T-002" {
                Ok(m.clone())
            } else {
                Err(refused(format_args!(
                    "no tasks/{t}-*.md in the working tree"
                )))
            }
        }
    }

    const NOW: i64 = 1_794_650_400; // 2026-11-14T10:00:00Z

    fn refusal(r: Result<impl std::fmt::Debug>) -> String {
        let e = r.unwrap_err();
        let Some(Refused(msg)) = e.downcast_ref::<Refused>() else {
            panic!("not a refusal: {e:#}");
        };
        assert!(msg.starts_with("metrics: refused: "), "{msg}");
        msg.clone()
    }

    const BASE: &str = "--task T-001 --run R-003 --role implementer --tool-uses 40 --gate-attempts 2 --outcome completed --started-at 2026-11-14T09:00:00Z";

    #[test]
    fn run_record_fields_defaults_and_clock() {
        let f = metas_fn(meta("medium", 0, 400000));
        let r = run_record(&opts(&format!("{BASE} --tokens 1200")), &f, &[], NOW).unwrap();
        assert_eq!(
            (r.tier.as_deref(), r.iteration, r.wall_s, r.tool_calls),
            (Some("medium"), 0, 3600, 40)
        );
        assert_eq!(r.ended_at, "2026-11-14T10:00:00Z");
        let v = serde_json::to_value(&r).unwrap();
        let keys: BTreeSet<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        let want: BTreeSet<&str> = [
            "kind",
            "task",
            "iteration",
            "run",
            "role",
            "tier",
            "started_at",
            "ended_at",
            "wall_s",
            "tokens",
            "tool_calls",
            "gate_attempts",
            "outcome",
            "missing_context",
        ]
        .into();
        assert_eq!(keys, want);
        let u = run_record(&opts(&format!("{BASE} --tokens-unknown")), &f, &[], NOW).unwrap();
        assert!(
            serde_json::to_string(&u)
                .unwrap()
                .contains("\"tokens\":null")
        );
    }

    #[test]
    fn run_record_refusals() {
        let f = metas_fn(meta("medium", 1, 400000));
        for (extra, needle) in [
            ("--tokens 0", "--tokens 0"),
            ("--tokens 5 --tokens-unknown", "not both"),
            ("", "exactly one"),
            (
                "--tokens 5 --ended-at 2026-11-14T11:00:00Z",
                "later than now",
            ),
        ] {
            let m = refusal(run_record(&opts(&format!("{BASE} {extra}")), &f, &[], NOW));
            assert!(m.contains(needle), "{extra}: {m}");
        }
        // Earlier than the task's latest record; equal is fine.
        let recs = [json!({"kind":"transition","task":"T-001","at":"2026-11-14T09:30:00Z"})];
        let early = format!("{BASE} --tokens 5 --ended-at 2026-11-14T09:29:59Z");
        assert!(refusal(run_record(&opts(&early), &f, &recs, NOW)).contains("earlier"));
        let equal = format!("{BASE} --tokens 5 --ended-at 2026-11-14T09:30:00Z");
        assert!(run_record(&opts(&equal), &f, &recs, NOW).is_ok());
        // Role and iter-N rules.
        let planner = BASE.replace("implementer", "planner");
        assert!(
            refusal(run_record(
                &opts(&format!("{planner} --tokens 5")),
                &f,
                &[],
                NOW
            ))
            .contains("book it to iter-1")
        );
        let iter = BASE.replace("T-001", "iter-1");
        assert!(
            refusal(run_record(
                &opts(&format!("{iter} --tokens 5")),
                &f,
                &[],
                NOW
            ))
            .contains("cannot be booked")
        );
        let ev = iter.replace("implementer", "evaluator");
        let r = run_record(&opts(&format!("{ev} --tokens 5")), &f, &[], NOW).unwrap();
        assert_eq!((r.task.as_str(), r.iteration, r.tier), ("iter-1", 1, None));
        assert!(
            refusal(run_record(
                &opts(&format!("{ev} --tokens 5 --iteration 2")),
                &f,
                &[],
                NOW
            ))
            .contains("--iteration 2")
        );
        assert!(
            refusal(run_record(
                &opts(&format!("{ev} --tokens 5 --tier low")),
                &f,
                &[],
                NOW
            ))
            .contains("tier")
        );
        for bad in ["iter-01", "iter-", "T-01", "x"] {
            let a = BASE.replace("T-001", bad);
            assert!(
                refusal(run_record(&opts(&format!("{a} --tokens 5")), &f, &[], NOW))
                    .contains("not a task id")
            );
        }
        let missing = BASE.replace("T-001", "T-009");
        assert!(
            refusal(run_record(
                &opts(&format!("{missing} --tokens 5")),
                &f,
                &[],
                NOW
            ))
            .contains("no tasks/T-009-*.md")
        );
        // Precedence: the id/role rule before the tokens rule.
        assert!(
            refusal(run_record(
                &opts(&format!("{planner} --tokens 0")),
                &f,
                &[],
                NOW
            ))
            .contains("iter-1")
        );
        // Usage errors are not refusals.
        assert!(Opts::parse(&args("--bogus 1"), ADD_OPTS, ADD_FLAGS, &[]).is_err());
        let e = run_record(&opts(&format!("{BASE} --tokens x")), &f, &[], NOW);
        assert!(e.is_err());
        let e = run_record(&opts(&format!("{BASE} --tokens x")), &f, &[], NOW).unwrap_err();
        assert!(e.downcast_ref::<Refused>().is_none());
    }

    #[test]
    fn budget_lines_follow_the_interface() {
        let rec = |t: &str, tok: Value| json!({"kind":"run","task":t,"tokens":tok});
        let recs = vec![
            rec("T-020", json!(300000)),
            rec("T-020", json!(250000)),
            rec("T-021", json!(9)),
        ];
        assert_eq!(
            budget_lines("T-020", &recs, Some(500000), None),
            [
                "T-020: 550000 tokens of budget 500000",
                "T-020: over its ceiling 500000 by 50000 tokens; the queue refuses it without a docs/decisions.md row \"Extend T-020 to <N>\" with N ≥ 550000",
            ]
        );
        assert_eq!(
            budget_lines("T-020", &recs, Some(500000), Some(600000)),
            ["T-020: 550000 tokens of budget 500000"]
        );
        let mut more = recs.clone();
        more.push(rec("T-020", Value::Null));
        assert_eq!(
            budget_lines("T-020", &more, Some(500000), Some(520000))[1..],
            [
                "T-020: over its ceiling 520000 by 30000 tokens; the queue refuses it without a docs/decisions.md row \"Extend T-020 to <N>\" with N ≥ 550000".to_owned(),
                "T-020: 1 run(s) with unknown tokens not counted".to_owned(),
            ]
        );
        assert_eq!(
            budget_lines("iter-1", &[rec("iter-1", json!(7))], None, None),
            ["iter-1: 7 tokens of overhead"]
        );
    }

    fn topts(s: &str) -> Opts {
        Opts::parse(
            &args(s),
            &["task", "from", "to", "at", "iteration"],
            &[],
            &[],
        )
        .unwrap()
    }

    #[test]
    fn transitions_follow_the_table_and_the_clock() {
        let f = metas_fn(meta("medium", 1, 1));
        let tr = |from: &str, to: &str, at: &str| json!({"kind":"transition","task":"T-001","at":at,"from":from,"to":to});
        let recs = vec![tr("ready", "in_progress", "2026-11-14T09:00:00Z")];
        let m = refusal(transition_record(
            &topts("--task T-001 --to tests_joined"),
            &f,
            &recs,
            NOW,
        ));
        assert_eq!(
            m,
            "metrics: refused: T-001 is in_progress (since 2026-11-14T09:00:00Z); in_progress -> tests_joined is not allowed (METHODOLOGY §3)"
        );
        assert!(
            refusal(transition_record(
                &topts("--task T-001 --to self_verified --at 2026-11-14T08:59:59Z"),
                &f,
                &recs,
                NOW
            ))
            .contains("earlier")
        );
        assert!(
            refusal(transition_record(
                &topts("--task T-001 --to self_verified --at 2026-11-14T11:00:00Z"),
                &f,
                &recs,
                NOW
            ))
            .contains("later than now")
        );
        assert!(
            refusal(transition_record(
                &topts("--task T-001 --from ready --to self_verified"),
                &f,
                &recs,
                NOW
            ))
            .contains("not ready")
        );
        let r =
            transition_record(&topts("--task T-001 --to self_verified"), &f, &recs, NOW).unwrap();
        assert_eq!(
            (r.from.as_str(), r.at.as_str(), r.iteration),
            ("in_progress", "2026-11-14T10:00:00Z", 1)
        );
        assert!(
            refusal(transition_record(
                &topts("--task iter-1 --to ready"),
                &f,
                &[],
                NOW
            ))
            .contains("iter-1")
        );
        // Fresh task: draft.
        assert!(transition_record(&topts("--task T-001 --to specified"), &f, &[], NOW).is_ok());
        assert!(
            refusal(transition_record(
                &topts("--task T-001 --to ready"),
                &f,
                &[],
                NOW
            ))
            .contains("draft (no transition recorded)")
        );
        // Blocked returns only where it came from.
        let b = vec![tr("in_progress", "blocked", "2026-11-14T09:00:00Z")];
        assert!(transition_record(&topts("--task T-001 --to in_progress"), &f, &b, NOW).is_ok());
        assert!(transition_record(&topts("--task T-001 --to ready"), &f, &b, NOW).is_err());
    }

    #[test]
    fn transition_table() {
        let ok = |a, b, t| transition_allowed(a, b, t, None);
        assert!(ok("self_verified", "queued", "low"));
        assert!(!ok("self_verified", "queued", "medium"));
        assert!(!ok("self_verified", "tests_joined", "low"));
        assert!(ok("reviewed", "queued", "high"));
        assert!(!ok("reviewed", "queued", "low"));
        assert!(ok("queued", "abandoned", "low"));
        assert!(!ok("merged", "blocked", "low"));
        assert!(ok("merged", "evaluated", "low"));
        for to in STATES {
            assert!(!ok("evaluated", to, "medium"));
            assert!(!ok("abandoned", to, "medium"));
        }
        assert!(transition_allowed(
            "blocked",
            "abandoned",
            "low",
            Some("ready")
        ));
        assert!(!transition_allowed(
            "blocked",
            "blocked",
            "low",
            Some("ready")
        ));
    }

    #[test]
    fn join_defects_rules() {
        let f = metas_fn(meta("medium", 1, 1));
        let sha = "a".repeat(40);
        let jo = |sha: &str, fails: u64, d: Value| json!({"kind":"join","task":"T-001","at":"2026-11-14T09:00:00Z","joined_sha":sha,"acc_fail_first_join":fails,"acc_defects":d,"impl_tests_green":true});
        let o = |n: u64| {
            Opts::parse(
                &args(&format!(
                    "--task T-001 --joined-sha {sha} --acc-defects {n}"
                )),
                &["task", "joined-sha", "acc-defects"],
                &[],
                &[],
            )
            .unwrap()
        };
        let recs = vec![jo(&sha, 2, Value::Null)];
        assert!(refusal(join_defects_record(&o(3), &f, &recs, NOW)).contains("exceeds"));
        let r = join_defects_record(&o(1), &f, &recs, NOW).unwrap();
        assert_eq!(
            serde_json::to_string(&r).unwrap(),
            format!(
                "{{\"kind\":\"join_defects\",\"task\":\"T-001\",\"at\":\"2026-11-14T10:00:00Z\",\"joined_sha\":\"{sha}\",\"acc_defects\":1}}"
            )
        );
        let mut again = recs.clone();
        again.push(serde_json::to_value(&r).unwrap());
        assert!(
            refusal(join_defects_record(&o(1), &f, &again, NOW)).contains("already classified")
        );
        assert!(
            refusal(join_defects_record(
                &o(1),
                &f,
                &[jo(&sha, 2, json!(1))],
                NOW
            ))
            .contains("already has")
        );
        assert!(
            refusal(join_defects_record(
                &o(0),
                &f,
                &[jo(&sha, 0, Value::Null)],
                NOW
            ))
            .contains("acc_fail_first_join 0")
        );
        assert!(refusal(join_defects_record(&o(1), &f, &[], NOW)).contains("no join record"));
        let two = vec![jo(&sha, 2, Value::Null), jo(&sha, 2, Value::Null)];
        assert!(refusal(join_defects_record(&o(1), &f, &two, NOW)).contains("2 join records"));
    }

    #[test]
    fn report_does_not_panic_on_malformed_task_ids() {
        // Ruling D-047 (o): such a task has arm `none`, reported under neither arm.
        let mut tasks = Vec::new();
        for t in [
            "T-021 ",
            "T-02!",
            "T-",
            "",
            "T-02\u{0663}",
            "T-021a",
            "\u{e9}",
        ] {
            for tier in ["high", "medium"] {
                tasks
                    .push(json!({"kind":"run","task":t,"tier":tier,"tokens":10,"gate_attempts":1}));
            }
            tasks.push(json!({"kind":"join","task":t,"base_fail_kind":"fails","acc_fail_first_join":1,"acc_defects":1,"impl_tests_green":true,"joined_sha":"a","at":"2026-11-13T01:00:00Z"}));
        }
        let queue = vec![
            json!({"task":"T-021 ","attempt":1,"result":"merged","ended_at":"2026-11-12T12:00:00Z"}),
        ];
        let (r, _) = report_text(&tasks, &queue, &BTreeMap::new(), "");
        assert!(!r.contains("arm independent"), "{r}");
        assert!(!r.contains("arm implementer"), "{r}");
    }

    #[test]
    fn report_measures() {
        let tasks = vec![
            json!({"kind":"transition","task":"T-001","iteration":0,"at":"2026-11-12T10:00:00Z","from":"specified","to":"ready"}),
            json!({"kind":"transition","task":"T-002","iteration":0,"at":"2026-11-12T10:00:00Z","from":"specified","to":"ready"}),
            json!({"kind":"run","task":"T-001","tier":"medium","tokens":1000,"gate_attempts":2}),
            json!({"kind":"run","task":"T-001","tier":"medium","tokens":500,"gate_attempts":1}),
            json!({"kind":"run","task":"T-001","tier":"medium","tokens":null,"gate_attempts":0}),
            json!({"kind":"run","task":"T-002","iteration":0,"tier":"low","tokens":300,"gate_attempts":1}),
            json!({"kind":"run","task":"iter-0","iteration":0,"tier":null,"tokens":200,"gate_attempts":0}),
            json!({"kind":"join","task":"T-001","base_fail_kind":"fails","acc_fail_first_join":1,"acc_defects":1,"impl_tests_green":true}),
            json!({"kind":"join","task":"T-001","base_fail_kind":"fails","acc_fail_first_join":3,"acc_defects":null,"impl_tests_green":true,"joined_sha":"b","at":"x"}),
            json!({"kind":"join_defects","task":"T-001","joined_sha":"b","acc_defects":2}),
            json!({"kind":"join","task":"T-001","acc_fail_first_join":5,"acc_defects":5,"impl_tests_green":false}),
        ];
        let queue = vec![
            json!({"task":"T-001","attempt":1,"result":"merged","ended_at":"2026-11-12T12:00:00Z"}),
            json!({"task":"T-002","attempt":1,"result":"gate_failed","ended_at":"2026-11-12T12:00:00Z"}),
            json!({"task":"T-002","attempt":2,"result":"merged","ended_at":"2026-11-12T14:00:00Z"}),
        ];
        let mut metas = BTreeMap::new();
        metas.insert("T-001".to_string(), meta("medium", 0, 1200));
        metas.insert(
            "T-003".to_string(),
            TaskMeta {
                found_by: Some("reviewer".into()),
                origin_task: Some("T-001".into()),
                ..meta("low", 1, 10)
            },
        );
        let decisions = "| ID | Decision |\n|---|---|\n| D-1 | Extend T-001 to 1400 |\n";
        let (r, ok) = report_text(&tasks, &queue, &metas, decisions);
        assert!(ok, "{r}");
        assert!(
            r.contains("first-pass queue success: 1/2 tasks (50%)"),
            "{r}"
        );
        assert!(
            r.contains("lead time (ready -> merged): median 3.00 h, max 4.00 h (n=2)"),
            "{r}"
        );
        assert!(
            r.contains(
                "medium        1500 tokens, 1 task(s); 1 run(s) with unknown tokens excluded"
            ),
            "{r}"
        );
        assert!(r.contains("overhead        200 tokens, 1 task(s)"), "{r}");
        assert!(r.contains("low            300 (n=1)"), "{r}");
        assert!(
            r.contains("medium        1500 (n=1; 1 run(s) with unknown tokens excluded)"),
            "{r}"
        );
        assert!(r.contains("implementer       1500 (n=1; 1 run(s)"), "{r}");
        assert!(r.contains("none           300 (n=1)"), "{r}");
        assert!(
            r.contains(
                "iteration 0 with overhead: 1000 (n=2; tasks 1800 + overhead 200 tokens over 1 iter-0 run(s)"
            ),
            "{r}"
        );
        assert!(r.contains("first joins: 1 (base evidence: fails 1)"), "{r}");
        assert!(
            r.contains("\nacceptance-caught defects: 3 (over 2 join(s)"),
            "{r}"
        );
        assert!(
            r.contains("\n  tier medium: 3\n  arm implementer: 3\n"),
            "{r}"
        );
        assert!(
            r.contains("known defects merged by decision: 1\n  T-003 (from T-001)\n"),
            "{r}"
        );
        assert!(
            r.contains("budget overruns: 1/2 tasks with a budget (T-001 1500/1200)"),
            "{r}"
        );
        assert!(
            r.contains("T-001 1500/1200: extend to: 1400, within ceiling: no"),
            "{r}"
        );
        // An unclassified failing join of an iteration-1 task fails the report.
        let mut metas1 = metas.clone();
        metas1.insert("T-001".to_string(), meta("medium", 1, 1200));
        let mut t2 = tasks.clone();
        t2.retain(|r| r["kind"] != "join_defects");
        let (r, ok) = report_text(&t2, &queue, &metas1, "");
        assert!(!ok);
        assert_eq!(r.matches("unclassified join: ").count(), 1, "{r}");
        assert!(
            r.contains(
                "unclassified join: T-001 at x joined_sha b: 3 failure(s), acc_defects missing"
            ),
            "{r}"
        );
        // Same records on an iteration-0 task: not listed.
        assert!(report_text(&t2, &queue, &metas, "").1);
    }
}
