//! `cargo xtask metrics add|transition|report` over `metrics/tasks.jsonl` and
//! `metrics/queue.jsonl`. Record kinds and field names are the contract of METHODOLOGY §13.
//! Every timestamp is passed in on the command line (or, for queue/join, taken by the CLI entry
//! point); nothing here reads the clock.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Result, bail};
use serde::Serialize;
use serde_json::Value;

use crate::timefmt::{format_utc, parse_utc};
use crate::util::{append_jsonl, read_jsonl, task_id};

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
    /// `T-NNN`.
    pub task: String,
    /// Iteration number.
    pub iteration: u64,
    /// `R-NNN`.
    pub run: String,
    /// One of [`ROLES`].
    pub role: String,
    /// low | medium | high.
    pub tier: String,
    /// UTC start.
    pub started_at: String,
    /// UTC end.
    pub ended_at: String,
    /// Wall seconds.
    pub wall_s: u64,
    /// Tokens used.
    pub tokens: u64,
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

/// Front matter of `tasks/T-NNN-*.md` that metrics use.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TaskMeta {
    /// Tier.
    pub tier: Option<String>,
    /// Iteration.
    pub iteration: Option<u64>,
    /// Budget.
    pub budget_tokens: Option<u64>,
}

/// Read every task file's metadata.
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
        let (fm, _) = crate::frontmatter::parse(&std::fs::read_to_string(&p)?)
            .map_err(|e| anyhow::anyhow!("{}: {e}", p.display()))?;
        let Some(id) = fm.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        let num = |k: &str| {
            fm.get(k)
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
        };
        out.insert(
            id.to_owned(),
            TaskMeta {
                tier: fm.get("tier").and_then(|v| v.as_str()).map(str::to_owned),
                iteration: num("iteration"),
                budget_tokens: num("budget_tokens"),
            },
        );
    }
    Ok(out)
}

/// Parsed `--flag value` options.
pub struct Opts(BTreeMap<String, String>);

impl Opts {
    /// Parse `--k v` pairs; `aliases` maps alternative names to canonical ones.
    pub fn parse(args: &[String], known: &[&str], aliases: &[(&str, &str)]) -> Result<Self> {
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
            if !known.contains(&k) {
                bail!("unknown option --{k}");
            }
            let Some(v) = it.next() else {
                bail!("--{k} needs a value")
            };
            if m.insert(k.to_owned(), v.clone()).is_some() {
                bail!("--{k} given twice");
            }
        }
        Ok(Opts(m))
    }
    fn get(&self, k: &str) -> Option<&str> {
        self.0.get(k).map(String::as_str)
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

fn normalize_ts(s: &str) -> Result<(i64, String)> {
    let t = parse_utc(s)?;
    if !s.contains('T') {
        bail!("timestamp `{s}` needs a time (YYYY-MM-DDTHH:MM:SSZ)");
    }
    Ok((t, format_utc(t)))
}

/// Build a run record from options (pure; `metas` supplies defaults for tier/iteration).
pub fn run_record(o: &Opts, metas: &BTreeMap<String, TaskMeta>) -> Result<RunRecord> {
    let task = task_id(o.req("task")?)?;
    let meta = metas.get(&task).cloned().unwrap_or_default();
    let run = o.req("run")?.to_owned();
    if !crate::util::valid_task_id(&run.replacen("R-", "T-", 1)) {
        bail!("--run `{run}` must look like R-NNN");
    }
    let role = o.req("role")?.to_owned();
    if !ROLES.contains(&role.as_str()) {
        bail!("--role `{role}` is not one of {}", ROLES.join(", "));
    }
    let tier = match o.get("tier") {
        Some(t) => t.to_owned(),
        None => meta.tier.clone().ok_or_else(|| {
            anyhow::anyhow!("--tier is required (no tasks/{task}-*.md with a tier)")
        })?,
    };
    if !matches!(tier.as_str(), "low" | "medium" | "high") {
        bail!("--tier `{tier}` is not low|medium|high");
    }
    let iteration = match o.num("iteration")? {
        Some(i) => i,
        None => meta.iteration.ok_or_else(|| {
            anyhow::anyhow!("--iteration is required (no tasks/{task}-*.md with an iteration)")
        })?,
    };
    let (s, started_at) = normalize_ts(o.req("started-at")?)?;
    let (e, ended_at) = normalize_ts(o.req("ended-at")?)?;
    if e < s {
        bail!("--ended-at is before --started-at");
    }
    let wall_s = match o.num("wall-s")? {
        Some(w) => w,
        None => u64::try_from(e - s).unwrap_or(0),
    };
    let outcome = o.req("outcome")?.to_owned();
    if outcome.trim().is_empty() {
        bail!("--outcome is empty");
    }
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
        tokens: o.req_num("tokens")?,
        tool_calls: o.req_num("tool-calls")?,
        gate_attempts: o.req_num("gate-attempts")?,
        outcome,
        missing_context: o.get("missing-context").unwrap_or("none").to_owned(),
        note: o.get("note").map(str::to_owned),
    })
}

/// Build a transition record (pure).
pub fn transition_record(o: &Opts, metas: &BTreeMap<String, TaskMeta>) -> Result<TransitionRecord> {
    let task = task_id(o.req("task")?)?;
    let from = o.req("from")?.to_owned();
    let to = o.req("to")?.to_owned();
    for s in [&from, &to] {
        if !STATES.contains(&s.as_str()) {
            bail!("state `{s}` is not one of {}", STATES.join(", "));
        }
    }
    let iteration = match o.num("iteration")? {
        Some(i) => i,
        None => metas
            .get(&task)
            .and_then(|m| m.iteration)
            .ok_or_else(|| anyhow::anyhow!("--iteration is required"))?,
    };
    let (_, at) = normalize_ts(o.req("at")?)?;
    Ok(TransitionRecord {
        kind: "transition",
        task,
        iteration,
        at,
        from,
        to,
    })
}

/// `metrics add …`.
pub fn add(root: &Path, args: &[String]) -> Result<String> {
    let o = Opts::parse(
        args,
        &[
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
        ],
        &[("seconds", "wall-s"), ("tool-uses", "tool-calls")],
    )?;
    let rec = run_record(&o, &task_metas(root)?)?;
    append_jsonl(&root.join("metrics/tasks.jsonl"), &rec)?;
    Ok(serde_json::to_string(&rec)?)
}

/// `metrics transition …`.
pub fn transition(root: &Path, args: &[String]) -> Result<String> {
    let o = Opts::parse(args, &["task", "from", "to", "at", "iteration"], &[])?;
    let rec = transition_record(&o, &task_metas(root)?)?;
    append_jsonl(&root.join("metrics/tasks.jsonl"), &rec)?;
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

/// The summary measures (pure).
pub fn report_text(tasks: &[Value], queue: &[Value], metas: &BTreeMap<String, TaskMeta>) -> String {
    let s = |v: &Value, k: &str| v[k].as_str().unwrap_or_default().to_owned();
    let n = |v: &Value, k: &str| v[k].as_u64().unwrap_or(0);
    let runs: Vec<&Value> = tasks.iter().filter(|r| r["kind"] == "run").collect();
    let mut out = String::new();

    let mut tokens_by_tier: BTreeMap<String, (u64, BTreeSet<String>)> = BTreeMap::new();
    let mut tokens_by_task: BTreeMap<String, u64> = BTreeMap::new();
    let mut gate_by_task: BTreeMap<String, u64> = BTreeMap::new();
    let mut tier_of: BTreeMap<String, String> = BTreeMap::new();
    for r in &runs {
        let e = tokens_by_tier.entry(s(r, "tier")).or_default();
        e.0 += n(r, "tokens");
        e.1.insert(s(r, "task"));
        *tokens_by_task.entry(s(r, "task")).or_default() += n(r, "tokens");
        *gate_by_task.entry(s(r, "task")).or_default() += n(r, "gate_attempts");
        tier_of.entry(s(r, "task")).or_insert_with(|| s(r, "tier"));
    }
    for (t, m) in metas {
        if let Some(tier) = &m.tier {
            tier_of.insert(t.clone(), tier.clone());
        }
    }
    out.push_str(&format!(
        "runs: {} over {} tasks\n",
        runs.len(),
        tokens_by_task.len()
    ));
    out.push_str("tokens by tier (all runs):\n");
    for (tier, (tok, ts)) in &tokens_by_tier {
        out.push_str(&format!(
            "  {tier:<7} {tok:>10} tokens, {} task(s)\n",
            ts.len()
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
            merged_at.len()
        ));
    }

    // Tokens per merged task by tier.
    let mut merged_tok: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for t in merged_at.keys() {
        let tier = tier_of.get(t).cloned().unwrap_or_else(|| "unknown".into());
        let e = merged_tok.entry(tier).or_default();
        e.0 += tokens_by_task.get(t).copied().unwrap_or(0);
        e.1 += 1;
    }
    out.push_str("tokens per merged task by tier:\n");
    if merged_tok.is_empty() {
        out.push_str("  (no merged tasks)\n");
    }
    for (tier, (tok, cnt)) in &merged_tok {
        out.push_str(&format!(
            "  {tier:<7} {:>10.0} (n={cnt})\n",
            *tok as f64 / *cnt as f64
        ));
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

    // Joins.
    let mut seen = BTreeSet::new();
    let mut kinds: BTreeMap<String, u64> = BTreeMap::new();
    let (mut caught, mut unclassified) = (0, 0);
    for j in tasks.iter().filter(|r| r["kind"] == "join") {
        let t = s(j, "task");
        if !seen.insert(t) {
            continue; // first join per task only
        }
        let kind = s(j, "base_fail_kind");
        let kind = if kind.is_empty() {
            "unrecorded".to_owned()
        } else {
            kind
        };
        *kinds.entry(kind).or_default() += 1;
        if j["impl_tests_green"] == Value::Bool(true) {
            match j["acc_defects"].as_u64() {
                Some(d) => caught += d,
                None if n(j, "acc_fail_first_join") > 0 => unclassified += 1,
                None => {}
            }
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
        out.push_str(&format!("acceptance-caught defects: {caught}"));
        if unclassified > 0 {
            out.push_str(&format!(
                " (+{unclassified} join(s) with failures not yet classified)"
            ));
        }
        out.push('\n');
    }

    // Budget overruns.
    let mut over = Vec::new();
    let mut with_budget = 0;
    for (t, m) in metas {
        if let Some(b) = m.budget_tokens {
            with_budget += 1;
            let used = tokens_by_task.get(t).copied().unwrap_or(0);
            if used > b {
                over.push(format!("{t} {used}/{b}"));
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
    out
}

/// `metrics report`.
pub fn report(root: &Path) -> Result<String> {
    let tasks = read_jsonl(&root.join("metrics/tasks.jsonl"))?;
    let queue = read_jsonl(&root.join("metrics/queue.jsonl"))?;
    Ok(report_text(&tasks, &queue, &task_metas(root)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    const ADD: &[&str] = &[
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

    #[test]
    fn run_record_uses_methodology_fields_and_task_defaults() {
        let mut metas = BTreeMap::new();
        metas.insert(
            "T-001".to_string(),
            TaskMeta {
                tier: Some("medium".into()),
                iteration: Some(0),
                budget_tokens: Some(400000),
            },
        );
        let o = Opts::parse(
            &args("--task T-001 --run R-003 --role implementer --tokens 1200 --tool-uses 40 --gate-attempts 2 --outcome completed --started-at 2026-11-12T10:00:00Z --ended-at 2026-11-12T11:00:00+01:00"),
            ADD,
            &[("seconds", "wall-s"), ("tool-uses", "tool-calls")],
        )
        .unwrap();
        let r = run_record(&o, &metas).unwrap();
        assert_eq!(
            (r.tier.as_str(), r.iteration, r.wall_s, r.tool_calls),
            ("medium", 0, 0, 40)
        );
        assert_eq!(r.ended_at, "2026-11-12T10:00:00Z");
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
    }

    #[test]
    fn run_record_rejects_bad_input() {
        let metas = BTreeMap::new();
        let base = "--task T-001 --run R-001 --role implementer --tokens 1 --tool-calls 1 --gate-attempts 1 --outcome completed --started-at 2026-11-12T10:00:00Z --ended-at 2026-11-12T10:00:05Z";
        let parse = |extra: &str| Opts::parse(&args(&format!("{base} {extra}")), ADD, &[]);
        assert!(
            run_record(&parse("--iteration 0").unwrap(), &metas).is_err(),
            "tier missing"
        );
        assert_eq!(
            run_record(&parse("--iteration 0 --tier low").unwrap(), &metas)
                .unwrap()
                .wall_s,
            5
        );
        assert!(run_record(&parse("--iteration 0 --tier urgent").unwrap(), &metas).is_err());
        assert!(Opts::parse(&args("--bogus 1"), ADD, &[]).is_err());
        let bad_role = base.replace("implementer", "coder");
        assert!(
            run_record(
                &Opts::parse(
                    &args(&format!("{bad_role} --tier low --iteration 0")),
                    ADD,
                    &[]
                )
                .unwrap(),
                &metas
            )
            .is_err()
        );
    }

    #[test]
    fn report_measures() {
        let tasks = vec![
            json!({"kind":"transition","task":"T-001","iteration":0,"at":"2026-11-12T10:00:00Z","from":"specified","to":"ready"}),
            json!({"kind":"transition","task":"T-002","iteration":0,"at":"2026-11-12T10:00:00Z","from":"specified","to":"ready"}),
            json!({"kind":"run","task":"T-001","tier":"medium","tokens":1000,"gate_attempts":2}),
            json!({"kind":"run","task":"T-001","tier":"medium","tokens":500,"gate_attempts":1}),
            json!({"kind":"run","task":"T-002","tier":"low","tokens":300,"gate_attempts":1}),
            json!({"kind":"join","task":"T-001","base_fail_kind":"fails","acc_fail_first_join":1,"acc_defects":1,"impl_tests_green":true}),
            json!({"kind":"join","task":"T-001","base_fail_kind":"fails","acc_fail_first_join":0,"acc_defects":0,"impl_tests_green":true}),
        ];
        let queue = vec![
            json!({"task":"T-001","attempt":1,"result":"merged","ended_at":"2026-11-12T12:00:00Z"}),
            json!({"task":"T-002","attempt":1,"result":"gate_failed","ended_at":"2026-11-12T12:00:00Z"}),
            json!({"task":"T-002","attempt":2,"result":"merged","ended_at":"2026-11-12T14:00:00Z"}),
        ];
        let mut metas = BTreeMap::new();
        metas.insert(
            "T-001".to_string(),
            TaskMeta {
                tier: Some("medium".into()),
                iteration: Some(0),
                budget_tokens: Some(1200),
            },
        );
        let r = report_text(&tasks, &queue, &metas);
        assert!(
            r.contains("first-pass queue success: 1/2 tasks (50%)"),
            "{r}"
        );
        assert!(
            r.contains("lead time (ready -> merged): median 3.00 h, max 4.00 h (n=2)"),
            "{r}"
        );
        assert!(r.contains("medium        1500 tokens, 1 task(s)"), "{r}");
        assert!(r.contains("low            300 (n=1)"), "{r}");
        assert!(r.contains("first joins: 1 (base evidence: fails 1)"), "{r}");
        assert!(r.contains("acceptance-caught defects: 1\n"), "{r}");
        assert!(
            r.contains("budget overruns: 1/1 tasks with a budget (T-001 1500/1200)"),
            "{r}"
        );
    }

    #[test]
    fn transitions_validate_states() {
        let o = Opts::parse(&args("--task T-001 --from ready --to in_progress --at 2026-11-12T10:00:00Z --iteration 0"), &["task", "from", "to", "at", "iteration"], &[]).unwrap();
        assert_eq!(
            transition_record(&o, &BTreeMap::new()).unwrap().to,
            "in_progress"
        );
        let o = Opts::parse(
            &args("--task T-001 --from ready --to done --at 2026-11-12T10:00:00Z --iteration 0"),
            &["task", "from", "to", "at", "iteration"],
            &[],
        )
        .unwrap();
        assert!(transition_record(&o, &BTreeMap::new()).is_err());
    }
}
