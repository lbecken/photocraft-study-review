//! The queue's preconditions (T-005), checked before any rebase, in this order: `task-file`,
//! `front-matter`, `state`, `config`, `touches`, `join`, `review`, `gate-path`, `dependencies`,
//! `approval`, `budget`, `main-history` (the queue checks `ledger` before all of them).
//!
//! Every input is read from a committed tree (D-026 R6): **main's** (task file, policy files,
//! `docs/decisions.md`, `metrics/`) or the task branch head's (its task-file copy,
//! review files, changed paths) — never from an uncommitted working-copy file, and never from
//! the task branch's copy of a rule file; `main-history` reads the `merged` records of main's
//! committed `metrics/queue.jsonl` (D-030 R12). A failed check is a [`Refusal`]; nothing is
//! changed or written.

use std::collections::BTreeMap;
use std::path::Path;

use crate::frontmatter::{self, Value};
use crate::git::{git, git_try};
use crate::policy::{GENERATED_FILE, LEDGER_FILE, Policy, path_under};

/// A queue precondition that does not hold: printed as `queue: refused: <check>: <reason>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    /// The check's name.
    pub check: &'static str,
    /// Why, on one line.
    pub reason: String,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let one_line: Vec<&str> = self
            .reason
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        write!(f, "queue: refused: {}: {}", self.check, one_line.join("; "))
    }
}

impl std::error::Error for Refusal {}

fn refuse<T>(check: &'static str, reason: impl Into<String>) -> Result<T, Refusal> {
    Err(Refusal {
        check,
        reason: reason.into(),
    })
}

/// Map an internal error to a refusal of `check`.
fn or_refuse<T>(check: &'static str, r: anyhow::Result<T>) -> Result<T, Refusal> {
    r.map_err(|e| Refusal {
        check,
        reason: format!("{e:#}"),
    })
}

/// What the checks established, for the rest of the queue run.
#[derive(Clone, Debug)]
pub struct Checked {
    /// Main's policy files.
    pub policy: Policy,
    /// Paths changed by the task branch since its merge base with main.
    pub changed: Vec<String>,
    /// The changed paths that are gate paths (D-026 R3), in `changed` order.
    pub gate_paths: Vec<String>,
    /// Whether main approves this task's changes to versions of main's lock (D-030 R11).
    pub versions_approved: bool,
    /// Run records of the task with `tokens: null` (counted 0 by `budget`).
    pub unknown_token_runs: usize,
}

/// File names that configure a tool the gate runs, wherever they are (clippy and rustfmt read
/// them per directory; the toolchain files are matched anywhere too, failing closed).
const GATE_FILE_NAMES: &[&str] = &[
    "rust-toolchain.toml",
    "rust-toolchain",
    "clippy.toml",
    ".clippy.toml",
    "rustfmt.toml",
    ".rustfmt.toml",
    "build.rs",
];

/// Whether changing `p` can change what the gate does, regardless of content (D-026 R3):
/// anything under `xtask/` or `.github/`, any `.cargo/` directory, any build script and the tool
/// configuration files above. The root `Cargo.toml` is a gate path only when certain tables
/// change ([`root_manifest_gate_tables_differ`]).
pub fn is_gate_path(p: &str) -> bool {
    let file = p.rsplit('/').next().unwrap_or(p);
    path_under(p, "xtask")
        || path_under(p, ".github")
        || p.split('/').any(|seg| seg == ".cargo")
        || GATE_FILE_NAMES.contains(&file)
}

/// The tables of a root `Cargo.toml` that configure the gate (D-026 R3): `[workspace.lints]`,
/// `[workspace.metadata]` and every `[profile.*]`. `None` if the text does not parse.
fn gate_tables(text: Option<&str>) -> Option<toml::Value> {
    let Some(text) = text else {
        return Some(toml::Value::Table(Default::default()));
    };
    let v: toml::Value = toml::from_str(text).ok()?;
    let mut out = toml::map::Map::new();
    let ws = v.get("workspace");
    for (k, x) in [
        ("workspace.lints", ws.and_then(|w| w.get("lints"))),
        ("workspace.metadata", ws.and_then(|w| w.get("metadata"))),
        ("profile", v.get("profile")),
    ] {
        if let Some(x) = x {
            out.insert(k.into(), x.clone());
        }
    }
    Some(toml::Value::Table(out))
}

/// Whether the task changed a value relative to main: the head's value differs from main's
/// **and** from the merge base's (a value the task did not touch, but main changed meanwhile, is
/// replaced by main's on the rebase: review F18). A manifest that does not parse at the head
/// counts as changed (fail closed).
fn task_changed<T: PartialEq>(h: Option<T>, m: Option<T>, b: Option<T>) -> bool {
    h.is_none() || (h != m && h != b)
}

/// Whether the root `Cargo.toml` changes a gate table relative to main (D-026 R3).
fn root_manifest_gate_tables_differ(
    head: Option<&str>,
    main: Option<&str>,
    base: Option<&str>,
) -> bool {
    task_changed(gate_tables(head), gate_tables(main), gate_tables(base))
}

/// The gate keys of any `Cargo.toml` (D-030 R10): its `[lints]` table (including
/// `lints.workspace`) and its package `build` key. `None` if the text does not parse.
fn manifest_gate_keys(text: Option<&str>) -> Option<toml::Value> {
    let Some(text) = text else {
        return Some(toml::Value::Table(Default::default()));
    };
    let v: toml::Value = toml::from_str(text).ok()?;
    let mut out = toml::map::Map::new();
    if let Some(x) = v.get("lints") {
        out.insert("lints".into(), x.clone());
    }
    if let Some(x) = v.get("package").and_then(|p| p.get("build")) {
        out.insert("package.build".into(), x.clone());
    }
    Some(toml::Value::Table(out))
}

/// The repository paths that a package `build` key names, in every `Cargo.toml` of tree `rev`
/// (D-030 R10). `build = true/false` names nothing beyond `build.rs`, already a gate path.
fn build_script_paths(
    root: &Path,
    rev: &str,
) -> anyhow::Result<std::collections::BTreeSet<String>> {
    let mut out = std::collections::BTreeSet::new();
    let files = git(root, &["ls-tree", "-r", "--name-only", rev])?;
    for m in files.lines().filter(|p| is_manifest(p)) {
        let Some(text) = show(root, rev, m)? else {
            continue;
        };
        let Ok(v) = toml::from_str::<toml::Value>(&text) else {
            continue;
        };
        let Some(b) = v
            .get("package")
            .and_then(|p| p.get("build"))
            .and_then(|b| b.as_str())
        else {
            continue;
        };
        let dir = m.strip_suffix("Cargo.toml").unwrap_or_default();
        let mut parts: Vec<&str> = dir.split('/').filter(|s| !s.is_empty()).collect();
        for seg in b.split('/') {
            match seg {
                "" | "." => {}
                ".." => {
                    parts.pop();
                }
                s => parts.push(s),
            }
        }
        out.insert(parts.join("/"));
    }
    Ok(out)
}

/// The dependency tables of a `Cargo.toml` (D-026 R4): `[dependencies]`, `[dev-dependencies]`,
/// `[build-dependencies]` (either spelling), the same under every `[target.*]`,
/// `[workspace.dependencies]`, `[patch.*]` and `[replace]`. `None` if the text does not parse.
fn dependency_tables(text: Option<&str>) -> Option<toml::Value> {
    const KINDS: &[&str] = &[
        "dependencies",
        "dev-dependencies",
        "dev_dependencies",
        "build-dependencies",
        "build_dependencies",
    ];
    let Some(text) = text else {
        return Some(toml::Value::Table(Default::default()));
    };
    let v: toml::Value = toml::from_str(text).ok()?;
    let mut out = toml::map::Map::new();
    for k in KINDS {
        if let Some(x) = v.get(*k) {
            out.insert((*k).into(), x.clone());
        }
    }
    for (cfg, t) in v
        .get("target")
        .and_then(toml::Value::as_table)
        .into_iter()
        .flatten()
    {
        for k in KINDS {
            if let Some(x) = t.get(*k) {
                out.insert(format!("target.{cfg}.{k}"), x.clone());
            }
        }
    }
    if let Some(x) = v.get("workspace").and_then(|w| w.get("dependencies")) {
        out.insert("workspace.dependencies".into(), x.clone());
    }
    for k in ["patch", "replace"] {
        if let Some(x) = v.get(k) {
            out.insert(k.into(), x.clone());
        }
    }
    Some(toml::Value::Table(out))
}

/// Whether two versions of a `Cargo.toml` differ in their dependency tables (unparsable: yes).
fn dependency_tables_differ(a: Option<&str>, b: Option<&str>) -> bool {
    let (a, b) = (dependency_tables(a), dependency_tables(b));
    a.is_none() || b.is_none() || a != b
}

fn is_manifest(p: &str) -> bool {
    p == "Cargo.toml" || p.ends_with("/Cargo.toml")
}

/// `git show <rev>:<path>`, or `None` if the path is not in that tree.
pub fn show(root: &Path, rev: &str, path: &str) -> anyhow::Result<Option<String>> {
    if !git_try(root, &["cat-file", "-e", &format!("{rev}:{path}")])?.0 {
        return Ok(None);
    }
    Ok(Some(
        crate::util::capture(crate::git::cmd(root).args(["show", &format!("{rev}:{path}")]))?
            .stdout,
    ))
}

/// The JSON lines of `path` in main's committed tree (missing: none; malformed: an error).
fn committed_jsonl(root: &Path, path: &str) -> anyhow::Result<Vec<serde_json::Value>> {
    let text = show(root, "main", path)?.unwrap_or_default();
    text.lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, l)| {
            serde_json::from_str(l).map_err(|e| anyhow::anyhow!("main:{path}:{}: {e}", i + 1))
        })
        .collect()
}

/// Whether commit `a` is `b` or an ancestor of it (unknown objects: no).
pub fn is_ancestor(root: &Path, a: &str, b: &str) -> anyhow::Result<bool> {
    Ok(git_try(root, &["merge-base", "--is-ancestor", a, b])?.0)
}

/// The raw text between the opening and closing `---` lines, if any.
fn raw_front_matter(text: &str) -> Option<String> {
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some("---") {
        return None;
    }
    let mut out = String::new();
    for l in lines {
        if l.trim() == "---" {
            return Some(out);
        }
        out.push_str(l);
        out.push('\n');
    }
    None
}

/// Whether `s` is a full lowercase 40-hex commit id.
pub fn is_full_sha(s: &str) -> bool {
    s.len() == 40
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// The task's number as written in its id (`T-007` → `007`).
fn number(task: &str) -> &str {
    task.strip_prefix("T-").unwrap_or(task)
}

/// The allowed-path rule of the `touches` check.
#[derive(Clone, Debug)]
pub struct Allowed<'a> {
    /// `T-NNN`.
    pub task: &'a str,
    /// Tier from main's task file.
    pub tier: &'a str,
    /// Main's `touches`.
    pub touches: &'a [String],
    /// Main's `scorecard_items`.
    pub scorecard_items: &'a [String],
    /// The task file's path.
    pub task_file: &'a str,
    /// Main's policy (generated paths are exempt).
    pub policy: &'a Policy,
}

impl Allowed<'_> {
    /// Whether `p` is one of the task's acceptance tests or regression guards: an
    /// `acceptance_tNNN*.rs` or `guard_tNNN*.rs` file directly in a `crates/*/tests/`,
    /// `apps/*/tests/` or `xtask/tests/` directory (D-010, D-019 (d)).
    pub fn is_test_file(&self, p: &str) -> bool {
        let segs: Vec<&str> = p.split('/').collect();
        let file = segs.last().copied().unwrap_or_default();
        let in_tests_dir = match segs.as_slice() {
            [top, _, "tests", _] => *top == "crates" || *top == "apps",
            ["xtask", "tests", _] => true,
            _ => false,
        };
        let n = number(self.task);
        in_tests_dir
            && (file.starts_with(&format!("acceptance_t{n}"))
                || file.starts_with(&format!("guard_t{n}")))
            && file.ends_with(".rs")
    }

    /// Whether a commit after `reviewed_sha` may change `p` (D-026 R2): review files, the task
    /// file, generated paths and acceptance/guard test files.
    pub fn allowed_after_review(&self, p: &str) -> bool {
        p.starts_with(&format!("reviews/{}-", self.task))
            || p == self.task_file
            || self.policy.is_generated(p)
            || self.is_test_file(p)
    }

    /// Whether the task branch may change path `p`.
    pub fn allows(&self, p: &str) -> bool {
        if self.touches.iter().any(|t| path_under(p, t))
            || p == self.task_file
            || self.policy.is_generated(p)
        {
            return true;
        }
        if self.tier == "low" {
            return false;
        }
        let acceptance = self.is_test_file(p);
        let item = self
            .scorecard_items
            .iter()
            .any(|id| p == format!("scorecard/items/{id}.toml"));
        acceptance || item || p.starts_with(&format!("reviews/{}-", self.task))
    }
}

/// The cells of a Markdown table line (`| a | b |`), split on unescaped `|`.
fn table_cells(line: &str) -> Vec<String> {
    let t = line.trim();
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = if t.ends_with('|') && !t.ends_with("\\|") {
        &t[..t.len() - 1]
    } else {
        t
    };
    let mut cells = Vec::new();
    let mut cur = String::new();
    let mut chars = t.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => {
                cur.push('|');
                chars.next();
            }
            '|' => cells.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    cells.push(cur);
    cells.into_iter().map(|c| c.trim().to_owned()).collect()
}

/// Whether `line` is a table delimiter row (`|---|:--:|`).
fn is_delimiter_row(line: &str) -> bool {
    let cells = table_cells(line);
    !cells.is_empty()
        && cells.iter().all(|c| {
            let c = c.strip_prefix(':').unwrap_or(c);
            let c = c.strip_suffix(':').unwrap_or(c);
            !c.is_empty() && c.chars().all(|ch| ch == '-')
        })
}

/// The Decision cells of the Markdown table rows of `text` (a `docs/decisions.md`), D-026 R7.
///
/// Only real table rows count: a line that starts with `|` after at most three spaces, inside a
/// table whose header row (with a `Decision` cell) is followed by a delimiter row. Lines inside
/// fenced code blocks (```` ``` ```` or `~~~`), indented code (four spaces or a tab) and HTML
/// comments are never rows.
pub fn decision_cells(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut fence: Option<(char, usize)> = None;
    let mut in_comment = false;
    // (Decision column, whether the delimiter row has been seen) of the current table.
    let mut table: Option<(Option<usize>, bool)> = None;
    for line in text.lines() {
        let indent = line.len() - line.trim_start_matches(' ').len();
        let body = line.trim_start_matches(' ');
        let code_indent = indent >= 4 || body.starts_with('\t');
        if let Some((ch, n)) = fence {
            let run = body.chars().take_while(|c| *c == ch).count();
            if !code_indent && run >= n && body[run * ch.len_utf8()..].trim().is_empty() {
                fence = None;
            }
            continue;
        }
        if in_comment {
            if line.contains("-->") {
                in_comment = false;
            }
            table = None;
            continue;
        }
        if !code_indent {
            for ch in ['`', '~'] {
                let run = body.chars().take_while(|c| *c == ch).count();
                if run >= 3 {
                    fence = Some((ch, run));
                }
            }
            if fence.is_some() {
                table = None;
                continue;
            }
            if body.starts_with("<!--") {
                in_comment = !body.contains("-->");
                table = None;
                continue;
            }
        }
        let is_row = !code_indent && body.starts_with('|');
        if !is_row {
            table = None;
            continue;
        }
        match table {
            None => {
                let c = table_cells(line);
                table = Some((c.iter().position(|x| x == "Decision"), false));
            }
            Some((col, false)) => {
                if !is_delimiter_row(line) {
                    // Not a table after all: treat this line as a new candidate header.
                    let c = table_cells(line);
                    table = Some((c.iter().position(|x| x == "Decision"), false));
                } else {
                    table = Some((col, true));
                }
            }
            Some((Some(i), true)) => {
                if let Some(cell) = table_cells(line).get(i) {
                    out.push(cell.clone());
                }
            }
            Some((None, true)) => {}
        }
    }
    out
}

/// Whether Decision cell `cell` begins with `Approve <task>`: the id followed by the cell's end
/// or a character that cannot continue an id (not alphanumeric, `-` or `_`).
fn approves(cell: &str, task: &str) -> bool {
    cell.strip_prefix(&format!("Approve {task}"))
        .is_some_and(|rest| {
            !rest.starts_with(|ch: char| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
        })
}

/// The ceiling `N` of a Decision cell that begins with `Extend <task> to N` (T-013 (2), D-4):
/// `N` decimal digits followed by the cell's end or a character that is neither a digit nor an
/// id character. `Extend <task>` without `to <digits>` is not an extension.
fn extension_of(cell: &str, task: &str) -> Option<u64> {
    let rest = cell.strip_prefix(&format!("Extend {task} to "))?;
    let n = rest.bytes().take_while(u8::is_ascii_digit).count();
    if n == 0
        || rest[n..].starts_with(|ch: char| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return None;
    }
    rest[..n].parse().ok()
}

/// The ceiling of the last (in table order) `Extend <task> to N` row of `text` (a
/// `docs/decisions.md`); rows are recognised exactly as for `approval` (D-026 R7).
pub fn extension(text: &str, task: &str) -> Option<u64> {
    decision_cells(text)
        .iter()
        .rev()
        .find_map(|c| extension_of(c, task))
}

/// Whether `text` (a `docs/decisions.md`) has a table row whose Decision cell begins with
/// `Approve <task>` and, if `gate_paths`, also contains the words `gate paths` (D-026 R3 (c)).
pub fn has_approval(text: &str, task: &str, gate_paths: bool) -> bool {
    has_approval_with(text, task, if gate_paths { &["gate paths"] } else { &[] })
}

/// Whether `text` has a table row whose Decision cell begins with `Approve <task>` and contains
/// every phrase of `words` (case-sensitive): `gate paths` (D-026 R3), `dependency versions`
/// (D-030 R11).
pub fn has_approval_with(text: &str, task: &str, words: &[&str]) -> bool {
    decision_cells(text)
        .iter()
        .any(|c| approves(c, task) && words.iter().all(|w| c.contains(w)))
}

/// The paths a commit changes against its first parent, with their status letter.
fn commit_changes(root: &Path, c: &str) -> anyhow::Result<Vec<(char, String)>> {
    let out = git(
        root,
        &[
            "diff",
            "--name-status",
            "--no-renames",
            &format!("{c}^1"),
            c,
        ],
    )?;
    Ok(out
        .lines()
        .filter_map(|l| {
            let (s, p) = l.split_once('\t')?;
            Some((s.chars().next()?, p.to_owned()))
        })
        .collect())
}

fn diff_keys(a: &str, b: &str) -> String {
    let parse = |t: &str| {
        frontmatter::parse(&format!("---\n{t}---\n"))
            .map(|(m, _)| m)
            .unwrap_or_default()
    };
    let (ma, mb): (BTreeMap<String, Value>, BTreeMap<String, Value>) = (parse(a), parse(b));
    let keys: Vec<&String> = ma
        .keys()
        .chain(mb.keys())
        .filter(|k| ma.get(*k) != mb.get(*k))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    if keys.is_empty() {
        "formatting or comments".into()
    } else {
        keys.iter()
            .map(|k| format!("`{k}`"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Run every check for `task` (branch `branch`, head `head`) in the repository whose primary
/// checkout is `root`.
pub fn check(root: &Path, task: &str, branch: &str, head: &str) -> Result<Checked, Refusal> {
    // task-file
    const TF: &str = "task-file";
    let listing = or_refuse(TF, git(root, &["ls-tree", "--name-only", "main", "tasks/"]))?;
    let prefix = format!("tasks/{task}-");
    let found: Vec<&str> = listing
        .lines()
        .filter(|l| l.starts_with(&prefix) && l.ends_with(".md"))
        .collect();
    let task_file = match found.as_slice() {
        [one] => (*one).to_owned(),
        [] => return refuse(TF, format!("no {prefix}*.md on main")),
        many => {
            return refuse(
                TF,
                format!(
                    "{} files match {prefix}*.md on main: {}",
                    many.len(),
                    many.join(", ")
                ),
            );
        }
    };
    let main_text = or_refuse(TF, show(root, "main", &task_file))?.unwrap_or_default();
    let fm = match frontmatter::parse(&main_text) {
        Ok((m, _)) => m,
        Err(e) => return refuse(TF, format!("{task_file} on main: {e}")),
    };
    if fm.get("id").and_then(Value::as_str) != Some(task) {
        return refuse(
            TF,
            format!("{task_file} on main does not have `id: {task}`"),
        );
    }
    let tier = fm
        .get("tier")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    if !matches!(tier.as_str(), "low" | "medium" | "high") {
        return refuse(
            TF,
            format!("{task_file} on main: `tier` is `{tier}`, not low, medium or high"),
        );
    }
    let touches = match fm.get("touches") {
        Some(Value::List(l)) => l.clone(),
        _ => return refuse(TF, format!("{task_file} on main has no `touches` list")),
    };
    let scorecard_items = fm
        .get("scorecard_items")
        .map(Value::as_list)
        .unwrap_or_default();
    let state = fm
        .get("state")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();

    // front-matter
    const FM: &str = "front-matter";
    let base = or_refuse(FM, git(root, &["merge-base", "main", branch]))?;
    let main_raw = raw_front_matter(&main_text).unwrap_or_default();
    let head_text = or_refuse(FM, show(root, head, &task_file))?;
    let base_text = or_refuse(FM, show(root, &base, &task_file))?;
    match (&head_text, &base_text) {
        (None, None) => {} // the branch never had the file and does not add it
        (None, Some(_)) => return refuse(FM, format!("{branch} deletes {task_file}")),
        (Some(h), _) => {
            let Some(head_raw) = raw_front_matter(h) else {
                return refuse(FM, format!("{task_file} on {branch} has no front matter"));
            };
            let unchanged =
                base_text.as_deref().and_then(raw_front_matter).as_ref() == Some(&head_raw);
            if head_raw != main_raw && !unchanged {
                return refuse(
                    FM,
                    format!(
                        "{branch} changes the front matter of {task_file} ({}); builders may append to the log body only",
                        diff_keys(&main_raw, &head_raw)
                    ),
                );
            }
        }
    }

    // state
    let accepted = state == "queued"
        || (state == "reviewed" && tier != "low")
        || (state == "self_verified" && tier == "low");
    if !accepted {
        let want = if tier == "low" {
            "`self_verified` or `queued`"
        } else {
            "`reviewed` or `queued`"
        };
        return refuse(
            "state",
            format!("main's {task_file} has state `{state}`; a {tier} task needs {want}"),
        );
    }

    // config
    const CF: &str = "config";
    let gen_text = or_refuse(CF, show(root, "main", GENERATED_FILE))?;
    let led_text = or_refuse(CF, show(root, "main", LEDGER_FILE))?;
    let policy = or_refuse(
        CF,
        Policy::parse(gen_text.as_deref(), led_text.as_deref()).map_err(|e| e.context("on main")),
    )?;

    // touches
    let changed: Vec<String> = or_refuse(
        "touches",
        git(root, &["diff", "--name-only", "--no-renames", &base, head]),
    )?
    .lines()
    .map(str::to_owned)
    .collect();
    let rule = Allowed {
        task,
        tier: &tier,
        touches: &touches,
        scorecard_items: &scorecard_items,
        task_file: &task_file,
        policy: &policy,
    };
    let outside: Vec<&str> = changed
        .iter()
        .map(String::as_str)
        .filter(|p| !rule.allows(p))
        .collect();
    if !outside.is_empty() {
        return refuse(
            "touches",
            format!(
                "{branch} changes path(s) outside touches [{}]: {}",
                touches.join(", "),
                outside.join(", ")
            ),
        );
    }

    if tier != "low" {
        // join
        const J: &str = "join";
        let recs = or_refuse(J, committed_jsonl(root, "metrics/tasks.jsonl"))?;
        let mut joined = 0;
        let mut ok = false;
        for r in recs.iter().filter(|r| {
            r["kind"] == "join" && r["task"].as_str() == Some(task) && r["result"] == "joined"
        }) {
            joined += 1;
            if let Some(s) = r["joined_sha"].as_str()
                && or_refuse(J, is_ancestor(root, s, head))?
            {
                ok = true;
                break;
            }
        }
        if !ok {
            return refuse(
                J,
                if joined == 0 {
                    format!(
                        "main's committed metrics/tasks.jsonl has no `joined` join record for {task}"
                    )
                } else {
                    format!(
                        "no `joined` join record of {task} has a joined_sha that is an ancestor of {head}"
                    )
                },
            );
        }

        // review
        const R: &str = "review";
        let files = or_refuse(R, git(root, &["ls-tree", "--name-only", head, "reviews/"]))?;
        let rprefix = format!("reviews/{task}-r");
        let newest = files
            .lines()
            .filter_map(|l| {
                let n = l.strip_prefix(&rprefix)?.strip_suffix(".md")?;
                (!n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
                    .then(|| (n.parse::<u64>().ok(), l.to_owned()))
            })
            .filter_map(|(n, l)| Some((n?, l)))
            .max();
        let Some((round, review_file)) = newest else {
            return refuse(R, format!("no {rprefix}<N>.md on {branch}"));
        };
        let text = or_refuse(R, show(root, head, &review_file))?.unwrap_or_default();
        let rfm = match frontmatter::parse(&text) {
            Ok((m, _)) => m,
            Err(e) => return refuse(R, format!("{review_file}: {e}")),
        };
        let verdict = rfm
            .get("verdict")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if verdict != "approve" {
            return refuse(
                R,
                format!("the newest review {review_file} (round {round}) has verdict `{verdict}`"),
            );
        }
        let reviewed = rfm
            .get("reviewed_sha")
            .and_then(Value::as_str)
            .unwrap_or_default();
        // A full commit id only (D-026 R2): a branch name or other revision expression would
        // follow the head and make the stale-review rule vacuous.
        if !is_full_sha(reviewed) {
            return refuse(
                R,
                format!("{review_file}: reviewed_sha `{reviewed}` is not a full 40-hex commit id"),
            );
        }
        let is_commit = or_refuse(
            R,
            git_try(root, &["cat-file", "-e", &format!("{reviewed}^{{commit}}")]),
        )?
        .0;
        if !is_commit {
            return refuse(
                R,
                format!("{review_file}: reviewed_sha `{reviewed}` is not a commit"),
            );
        }
        if !or_refuse(R, is_ancestor(root, reviewed, head))? {
            return refuse(
                R,
                format!("{review_file}: reviewed_sha {reviewed} is not an ancestor of {head}"),
            );
        }
        let after = or_refuse(
            R,
            git(
                root,
                &["rev-list", "--reverse", &format!("{reviewed}..{head}")],
            ),
        )?;
        for c in after.lines() {
            for (_, p) in or_refuse(R, commit_changes(root, c))? {
                if !rule.allowed_after_review(&p) {
                    return refuse(
                        R,
                        format!(
                            "stale review: commit {} after reviewed_sha {} changes `{p}` (after review only reviews/{task}-*, {task_file}, generated paths and acceptance/guard tests may change)",
                            &c[..12.min(c.len())],
                            &reviewed[..12]
                        ),
                    );
                }
            }
        }
    }

    // gate-path (D-026 R3)
    const GP: &str = "gate-path";
    let mut gate_paths = Vec::new();
    let mut build_scripts = or_refuse(GP, build_script_paths(root, head))?;
    build_scripts.extend(or_refuse(GP, build_script_paths(root, "main"))?);
    for p in &changed {
        let mut gate = is_gate_path(p) || build_scripts.contains(p);
        if is_manifest(p) {
            let at = |r: &str| or_refuse(GP, show(root, r, p));
            let (h, m, b) = (at(head)?, at("main")?, at(&base)?);
            let (h, m, b) = (h.as_deref(), m.as_deref(), b.as_deref());
            gate |= task_changed(
                manifest_gate_keys(h),
                manifest_gate_keys(m),
                manifest_gate_keys(b),
            );
            if p == "Cargo.toml" {
                gate |= root_manifest_gate_tables_differ(h, m, b);
            }
        }
        if gate {
            gate_paths.push(p.clone());
        }
    }
    if !gate_paths.is_empty() && tier != "high" {
        return refuse(
            GP,
            format!(
                "{branch} changes gate path(s) {}; such a task must be tier high (it is {tier})",
                gate_paths.join(", ")
            ),
        );
    }

    // dependencies (D-026 R4)
    const DP: &str = "dependencies";
    let mut dep_changes = Vec::new();
    for p in &changed {
        if crate::queue::is_lock(p) {
            dep_changes.push(p.clone());
        } else if is_manifest(p) {
            let at = |r: &str| or_refuse(DP, show(root, r, p));
            if dependency_tables_differ(at(&base)?.as_deref(), at(head)?.as_deref()) {
                dep_changes.push(format!("dependency tables of {p}"));
            }
        }
    }
    if !dep_changes.is_empty() && tier != "high" {
        return refuse(
            DP,
            format!(
                "{branch} changes dependencies ({}); such a task must be tier high (it is {tier})",
                dep_changes.join(", ")
            ),
        );
    }
    let mut version_changes = Vec::new();
    for p in changed.iter().filter(|p| crate::queue::is_lock(p)) {
        let (Some(main_lock), Some(head_lock)) = (
            or_refuse(DP, show(root, "main", p))?,
            or_refuse(DP, show(root, head, p))?,
        ) else {
            continue;
        };
        // A version the task's own diff changed (head differs from the merge base too); a
        // version that is merely stale because main moved is replaced by main's on the rebase
        // or regeneration and checked again on the rebased tree before the gate.
        let own: Vec<String> = match or_refuse(DP, show(root, &base, p))? {
            Some(base_lock) => or_refuse(
                DP,
                crate::queue::lock_version_changes(&base_lock, &head_lock),
            )?,
            None => Vec::new(),
        };
        let pkg = |c: &String| c.split(' ').next().unwrap_or_default().to_owned();
        let own: std::collections::BTreeSet<String> = own.iter().map(pkg).collect();
        let bumped: Vec<String> = or_refuse(
            DP,
            crate::queue::lock_version_changes(&main_lock, &head_lock),
        )?
        .into_iter()
        .filter(|c| own.contains(&pkg(c)))
        .collect();
        version_changes.extend(bumped.into_iter().map(|c| format!("{p}: {c}")));
    }
    // D-030 R11: such a change merges only for a tier-high task approved for it.
    let decisions = or_refuse(DP, show(root, "main", "docs/decisions.md"))?;
    let versions_approved = tier == "high"
        && decisions
            .as_deref()
            .is_some_and(|t| has_approval_with(t, task, &["dependency versions"]));
    if !version_changes.is_empty() && !versions_approved {
        return refuse(
            DP,
            format!(
                "{branch} changes the version of package(s) already in main's lock ({}); that needs tier high and an `Approve {task}` row in main's docs/decisions.md containing `dependency versions`",
                version_changes.join(", ")
            ),
        );
    }

    // approval (with D-026 R3 (c) for gate paths)
    if tier == "high" {
        let d = or_refuse("approval", show(root, "main", "docs/decisions.md"))?;
        let need_gate = !gate_paths.is_empty();
        if !d.is_some_and(|t| has_approval(&t, task, need_gate)) {
            return refuse(
                "approval",
                if need_gate {
                    format!(
                        "main's docs/decisions.md has no table row whose Decision begins with `Approve {task}` and contains `gate paths` (the task changes gate paths: {})",
                        gate_paths.join(", ")
                    )
                } else {
                    format!(
                        "main's docs/decisions.md has no row whose Decision begins with `Approve {task}`"
                    )
                },
            );
        }
    }

    // budget (T-013 (2), METHODOLOGY §11): main's committed run records against the ceiling.
    const BU: &str = "budget";
    let recs = or_refuse(BU, committed_jsonl(root, "metrics/tasks.jsonl"))?;
    let runs = recs
        .iter()
        .filter(|r| r["kind"] == "run" && r["task"].as_str() == Some(task));
    let (mut spent, mut unknown_token_runs) = (0u64, 0usize);
    for r in runs {
        match r["tokens"].as_u64() {
            Some(t) => spent = spent.saturating_add(t),
            None => unknown_token_runs += 1,
        }
    }
    let budget = fm
        .get("budget_tokens")
        .and_then(Value::as_str)
        .and_then(|s| s.parse::<u64>().ok());
    let Some(budget) = budget else {
        return refuse(
            BU,
            format!("{task_file} on main has no numeric `budget_tokens`"),
        );
    };
    let decisions = or_refuse(BU, show(root, "main", "docs/decisions.md"))?;
    let (ceiling, source) = match decisions.as_deref().and_then(|t| extension(t, task)) {
        Some(n) => (n, "Extend row"),
        None => (budget, "budget_tokens"),
    };
    if spent > ceiling {
        return refuse(
            BU,
            format!(
                "{task} has {spent} recorded tokens, over its ceiling {ceiling} ({source}); a docs/decisions.md row \"Extend {task} to <N>\" with N ≥ {spent} is needed"
            ),
        );
    }

    // main-history
    const MH: &str = "main-history";
    // D-030 R12: `merged` records only from main's committed tree.
    let queue = or_refuse(MH, committed_jsonl(root, "metrics/queue.jsonl"))?;
    let last_merged = queue
        .iter()
        .rev()
        .find(|r| r["result"] == "merged")
        .and_then(|r| r["head_sha"].as_str().map(str::to_owned));
    if let Some(m) = last_merged {
        if !or_refuse(MH, is_ancestor(root, &m, "main"))? {
            return refuse(
                MH,
                format!(
                    "the last merged head {m} (metrics/queue.jsonl) is not an ancestor of main"
                ),
            );
        }
        let commits = or_refuse(
            MH,
            git(
                root,
                &[
                    "rev-list",
                    "--first-parent",
                    "--reverse",
                    &format!("{m}..main"),
                ],
            ),
        )?;
        for c in commits.lines() {
            for (status, p) in or_refuse(MH, commit_changes(root, c))? {
                if !policy.ledger_allows(&p, status) {
                    let subject = git(root, &["log", "-1", "--format=%s", c]).unwrap_or_default();
                    return refuse(
                        MH,
                        format!(
                            "commit {c} (\"{subject}\") on main after the last queue merge changes `{p}` ({status}), which {LEDGER_FILE} does not allow"
                        ),
                    );
                }
            }
        }
    }

    Ok(Checked {
        policy,
        changed,
        gate_paths,
        versions_approved,
        unknown_token_runs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GEN: &str = "[[generated]]\npath = \"Cargo.lock\"\ncommand = [\"true\"]\n";
    const LED: &str = "[[ledger]]\npath = \"tasks/\"\nmode = \"any\"\n";

    fn rule<'a>(
        tier: &'a str,
        touches: &'a [String],
        items: &'a [String],
        p: &'a Policy,
    ) -> Allowed<'a> {
        Allowed {
            task: "T-007",
            tier,
            touches,
            scorecard_items: items,
            task_file: "tasks/T-007-x.md",
            policy: p,
        }
    }

    #[test]
    fn allowed_paths() {
        let p = Policy::parse(Some(GEN), Some(LED)).unwrap();
        let touches = vec!["crates/a".to_owned(), "docs/x.md".to_owned()];
        let items = vec!["text.insert".to_owned()];
        let m = rule("medium", &touches, &items, &p);
        for ok in [
            "crates/a/src/x.rs",
            "docs/x.md",
            "tasks/T-007-x.md",
            "Cargo.lock",
            "crates/a/tests/acceptance_t007_x.rs",
            "crates/b/tests/acceptance_t007.rs",
            "apps/c/tests/acceptance_t007.rs",
            "xtask/tests/acceptance_t007.rs",
            "xtask/tests/guard_t007.rs",
            "crates/b/tests/guard_t007_seed.rs",
            "scorecard/items/text.insert.toml",
            "reviews/T-007-r1.md",
        ] {
            assert!(m.allows(ok), "{ok}");
        }
        for bad in [
            "crates/ab/x.rs",
            "docs/x.md.bak",
            "tasks/T-008-x.md",
            "crates/a2/tests/acceptance_t008.rs",
            "crates/b/tests/acceptance_t008.rs",
            "crates/b/tests/guard_t008.rs",
            "crates/b/tests/sub/acceptance_t007.rs",
            "crates/b/src/guard_t007.rs",
            "crates/b/src/acceptance_t007.rs",
            "xtask/tests/acceptance_t007.txt",
            "scorecard/items/other.toml",
            "reviews/T-0071-r1.md",
            "reviews/T-008-r1.md",
        ] {
            assert!(!m.allows(bad), "{bad}");
        }
        let low = rule("low", &touches, &items, &p);
        assert!(!low.allows("crates/b/tests/acceptance_t007.rs"));
        assert!(!low.allows("xtask/tests/guard_t007.rs"));
        assert!(!low.allows("reviews/T-007-r1.md"));
        assert!(!low.allows("scorecard/items/text.insert.toml"));
        assert!(
            low.allows("crates/a/tests/acceptance_t007.rs"),
            "inside touches"
        );
    }

    #[test]
    fn approval_rows() {
        let has = |t: &str, task: &str| has_approval(t, task, false);
        let t = "# Log\n\n| # | Date | Decision | Why |\n|---|---|---|---|\n| D-1 | d | Approve T-005 for merge. | x |\n";
        assert!(has(t, "T-005"));
        assert!(!has(t, "T-00"));
        assert!(!has(t, "T-006"));
        let wrong_col =
            "| # | Date | Decision | Why |\n|---|---|---|---|\n| D-1 | d | ok | Approve T-005 |\n";
        assert!(!has(wrong_col, "T-005"));
        let not_begin = "| # | Decision |\n|---|---|\n| D-1 | We Approve T-005 |\n";
        assert!(!has(not_begin, "T-005"));
        let longer = "| # | Decision |\n|---|---|\n| D-1 | Approve T-0050 |\n";
        assert!(!has(longer, "T-005"));
        assert!(!has("Approve T-005\n", "T-005"));
        // D-026 R7: only real table rows outside code blocks and comments.
        let row = "| # | Decision |\n|:--|---:|\n| D-1 | Approve T-005 |\n";
        assert!(has(row, "T-005"));
        assert!(has(&format!("intro\n\n{row}"), "T-005"));
        for fenced in [
            format!("```\n{row}```\n"),
            format!("~~~markdown\n{row}~~~\n"),
            format!("````\n```\n{row}````\n"),
            format!("<!--\n{row}-->\n"),
            row.lines()
                .map(|l| format!("    {l}\n"))
                .collect::<String>(),
            row.lines().map(|l| format!("\t{l}\n")).collect::<String>(),
            row.lines().map(|l| format!("> {l}\n")).collect::<String>(),
            "| # | Decision |\n| D-1 | Approve T-005 |\n".to_owned(), // no delimiter row
        ] {
            assert!(!has(&fenced, "T-005"), "{fenced}");
        }
        assert!(
            has(&format!("```\ncode\n```\n\n{row}"), "T-005"),
            "after a closed fence"
        );
        assert!(!has(
            "| # | Decision |\n|---|---|\n| D-1 | Approve T-005-tests |\n",
            "T-005"
        ));
        assert!(!has(
            "| # | Decision |\n|---|---|\n| D-1 | Approve T-005_x |\n",
            "T-005"
        ));
        assert!(has(
            "| # | Decision |\n|---|---|\n| D-1 | Approve T-005, a \\| b |\n",
            "T-005"
        ));
        // D-026 R3 (c): a gate-path task needs the words `gate paths` in the same cell.
        let gp = "| # | Decision | Why |\n|---|---|---|\n| D-1 | Approve T-005 incl. gate paths. | x |\n";
        assert!(has_approval(gp, "T-005", true));
        assert!(!has_approval(row, "T-005", true));
        // F16: the wording is case-sensitive.
        let upper = "| # | Decision |\n|---|---|\n| D-1 | Approve T-005, Gate Paths. |\n";
        assert!(!has_approval(upper, "T-005", true));
        let both = "| # | Decision |\n|---|---|\n| D-1 | Approve T-005: gate paths and dependency versions. |\n";
        assert!(has_approval_with(
            both,
            "T-005",
            &["gate paths", "dependency versions"]
        ));
        assert!(!has_approval_with(
            gp,
            "T-005",
            &["gate paths", "dependency versions"]
        ));
        let other_cell =
            "| # | Decision | Why |\n|---|---|---|\n| D-1 | Approve T-005 | gate paths |\n";
        assert!(!has_approval(other_cell, "T-005", true));
    }

    #[test]
    fn after_review_paths_and_full_shas() {
        let p = Policy::parse(Some(GEN), Some(LED)).unwrap();
        let touches = vec!["crates/a".to_owned()];
        let m = rule("medium", &touches, &[], &p);
        for ok in [
            "reviews/T-007-r2.md",
            "tasks/T-007-x.md",
            "Cargo.lock",
            "crates/b/tests/acceptance_t007.rs",
            "xtask/tests/guard_t007_r2.rs",
        ] {
            assert!(m.allowed_after_review(ok), "{ok}");
        }
        for bad in [
            "crates/a/src/x.rs",
            "crates/b/tests/acceptance_t008.rs",
            "crates/b/tests/helper.rs",
            "reviews/T-008-r1.md",
            "tasks/T-007-y.md",
        ] {
            assert!(!m.allowed_after_review(bad), "{bad}");
        }
        assert!(is_full_sha("0123456789abcdef0123456789abcdef01234567"));
        for bad in [
            "0123456789ABCDEF0123456789abcdef01234567",
            "0123456789abcdef0123456789abcdef0123456",
            "0123456789abcdef0123456789abcdef012345678",
            "task/T-007",
            "HEAD",
            "0123456789abcdef0123456789abcdef0123456g",
        ] {
            assert!(!is_full_sha(bad), "{bad}");
        }
    }

    #[test]
    fn gate_path_predicate_and_tables() {
        for p in [
            "xtask",
            "xtask/src/main.rs",
            "xtask/layers.toml",
            "xtask/tests/acceptance_t009.rs",
            ".cargo/config.toml",
            "crates/a/.cargo/config.toml",
            ".github/workflows/ci.yml",
            "rust-toolchain.toml",
            "rust-toolchain",
            "clippy.toml",
            "crates/a/.clippy.toml",
            "rustfmt.toml",
            "crates/a/build.rs",
        ] {
            assert!(is_gate_path(p), "{p}");
        }
        for p in [
            "xtaskx/a",
            "docs/clippy.toml.md",
            "src/xtask/a.rs",
            "crates/a/src/build.rs.bak",
            "Cargo.toml",
            "Cargo.lock",
            ".githubx/a",
        ] {
            assert!(!is_gate_path(p), "{p}");
        }
        let m = "[workspace]\nmembers = []\n[profile.release]\nlto = true\n";
        assert!(!root_manifest_gate_tables_differ(Some(m), Some(m), Some(m)));
        let w = "[workspace]\n";
        assert!(
            root_manifest_gate_tables_differ(Some(m), Some(w), Some(w)),
            "task changed it"
        );
        // F18: changed on main only (head = base), or changed to main's value: not the task's.
        assert!(!root_manifest_gate_tables_differ(Some(w), Some(m), Some(w)));
        assert!(!root_manifest_gate_tables_differ(Some(m), Some(m), Some(w)));
        assert!(root_manifest_gate_tables_differ(
            Some("x ["),
            Some(m),
            Some(m)
        ));
        // D-030 R10: member [lints] and package build keys.
        let lw = "[package]\nname = \"a\"\n[lints]\nworkspace = true\n";
        let plain = "[package]\nname = \"a\"\n";
        let k = |t: &str| manifest_gate_keys(Some(t));
        assert!(task_changed(k(plain), k(lw), k(lw)), "lints removed");
        assert!(task_changed(k(lw), k(plain), k(plain)), "lints added");
        assert!(task_changed(
            k("[package]\nname = \"a\"\n[lints.rust]\nunsafe_code = \"allow\"\n"),
            k(lw),
            k(lw)
        ));
        assert!(task_changed(
            k("[package]\nname = \"a\"\nbuild = \"gen.rs\"\n"),
            k(plain),
            k(plain)
        ));
        assert!(!task_changed(
            k(&format!("{plain}version = \"1\"\n")),
            k(plain),
            k(plain)
        ));
        let d = "[package]\nname = \"a\"\n[dependencies]\nx = \"1\"\n";
        assert!(!dependency_tables_differ(
            Some(d),
            Some(&format!("# c\n{d}"))
        ));
        assert!(dependency_tables_differ(None, Some(d)));
        assert!(!dependency_tables_differ(
            None,
            Some("[package]\nname = \"a\"\n")
        ));
        assert!(dependency_tables_differ(
            Some(d),
            Some("[workspace.dependencies]\nx = \"1\"\n")
        ));
        assert!(dependency_tables_differ(Some(d), Some("broken [")));
    }

    #[test]
    fn refusal_is_one_line() {
        let r = Refusal {
            check: "config",
            reason: "a\n  b\n".into(),
        };
        assert_eq!(r.to_string(), "queue: refused: config: a; b");
    }

    #[test]
    fn raw_front_matter_blocks() {
        assert_eq!(
            raw_front_matter("---\na: 1\n---\nbody\n").as_deref(),
            Some("a: 1\n")
        );
        assert_eq!(raw_front_matter("a: 1\n"), None);
        assert_eq!(diff_keys("a: 1\nb: 2\n", "a: 1\nb: 3\n"), "`b`");
    }
}
