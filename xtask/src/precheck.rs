//! The queue's preconditions (T-005), checked before any rebase, in this order: `task-file`,
//! `front-matter`, `state`, `config`, `touches`, `join`, `review`, `approval`, `main-history`.
//!
//! Every input is read from **main's** tree (task file, policy files, `docs/decisions.md`), from
//! the task branch head (its task-file copy, review files, changed paths) or from the primary
//! checkout's `metrics/` records — never from the task branch's copy of a rule file. A failed
//! check is a [`Refusal`]; nothing is changed or written.

use std::collections::BTreeMap;
use std::path::Path;

use crate::frontmatter::{self, Value};
use crate::git::{git, git_try, rev};
use crate::policy::{GENERATED_FILE, LEDGER_FILE, Policy, path_under};
use crate::util::read_jsonl;

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
        let segs: Vec<&str> = p.split('/').collect();
        let file = segs.last().copied().unwrap_or_default();
        let in_tests_dir = match segs.as_slice() {
            [top, _, "tests", _] => *top == "crates" || *top == "apps",
            ["xtask", "tests", _] => true,
            _ => false,
        };
        let acceptance = in_tests_dir
            && file.starts_with(&format!("acceptance_t{}", number(self.task)))
            && file.ends_with(".rs");
        let item = self
            .scorecard_items
            .iter()
            .any(|id| p == format!("scorecard/items/{id}.toml"));
        acceptance || item || p.starts_with(&format!("reviews/{}-", self.task))
    }
}

/// Whether `text` (a `docs/decisions.md`) has a table row whose Decision cell begins with
/// `Approve <task>` (the id followed by a non-alphanumeric character or the cell's end).
pub fn has_approval(text: &str, task: &str) -> bool {
    let cells = |l: &str| -> Vec<String> {
        let t = l.trim();
        let t = t.strip_prefix('|').unwrap_or(t);
        let t = t.strip_suffix('|').unwrap_or(t);
        t.split('|').map(|c| c.trim().to_owned()).collect()
    };
    let want = format!("Approve {task}");
    let mut col: Option<usize> = None;
    let mut prev_table = false;
    for line in text.lines() {
        let is_row = line.trim_start().starts_with('|');
        if !is_row {
            col = None;
            prev_table = false;
            continue;
        }
        let c = cells(line);
        if !prev_table {
            col = c.iter().position(|x| x == "Decision");
            prev_table = true;
            continue;
        }
        let Some(i) = col else { continue };
        if let Some(cell) = c.get(i)
            && let Some(rest) = cell.strip_prefix(&want)
            && !rest.starts_with(|ch: char| ch.is_ascii_alphanumeric())
        {
            return true;
        }
    }
    false
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
        let recs = or_refuse(J, read_jsonl(&root.join("metrics/tasks.jsonl")))?;
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
                    format!("metrics/tasks.jsonl has no `joined` join record for {task}")
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
        let Ok(reviewed) = rev(root, reviewed) else {
            return refuse(
                R,
                format!("{review_file}: reviewed_sha `{reviewed}` is not a commit"),
            );
        };
        if !or_refuse(R, is_ancestor(root, &reviewed, head))? {
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
        let review_prefix = format!("reviews/{task}-");
        for c in after.lines() {
            for (_, p) in or_refuse(R, commit_changes(root, c))? {
                if !(p.starts_with(&review_prefix) || p == task_file) {
                    return refuse(
                        R,
                        format!(
                            "commit {} after reviewed_sha changes `{p}` (only {review_prefix}* and {task_file} may change after review)",
                            &c[..12.min(c.len())]
                        ),
                    );
                }
            }
        }
    }

    if tier == "high" {
        let d = or_refuse("approval", show(root, "main", "docs/decisions.md"))?;
        if !d.is_some_and(|t| has_approval(&t, task)) {
            return refuse(
                "approval",
                format!(
                    "main's docs/decisions.md has no row whose Decision begins with `Approve {task}`"
                ),
            );
        }
    }

    // main-history
    const MH: &str = "main-history";
    let queue = or_refuse(MH, read_jsonl(&root.join("metrics/queue.jsonl")))?;
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

    Ok(Checked { policy, changed })
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
            "crates/b/tests/sub/acceptance_t007.rs",
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
        assert!(!low.allows("reviews/T-007-r1.md"));
        assert!(!low.allows("scorecard/items/text.insert.toml"));
        assert!(
            low.allows("crates/a/tests/acceptance_t007.rs"),
            "inside touches"
        );
    }

    #[test]
    fn approval_rows() {
        let t = "# Log\n\n| # | Date | Decision | Why |\n|---|---|---|---|\n| D-1 | d | Approve T-005 for merge. | x |\n";
        assert!(has_approval(t, "T-005"));
        assert!(!has_approval(t, "T-00"));
        assert!(!has_approval(t, "T-006"));
        let wrong_col =
            "| # | Date | Decision | Why |\n|---|---|---|---|\n| D-1 | d | ok | Approve T-005 |\n";
        assert!(!has_approval(wrong_col, "T-005"));
        let not_begin = "| # | Decision |\n|---|---|\n| D-1 | We Approve T-005 |\n";
        assert!(!has_approval(not_begin, "T-005"));
        let longer = "| # | Decision |\n|---|---|\n| D-1 | Approve T-0050 |\n";
        assert!(!has_approval(longer, "T-005"));
        assert!(!has_approval("Approve T-005\n", "T-005"));
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
