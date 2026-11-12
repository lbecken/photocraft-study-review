//! `cargo xtask scorecard [--check]`: derive every item's status from its named tests and the
//! evaluator's confirmations, and write `docs/scorecard.md`. Status is never stored in an item.
//!
//! * `planned`     — no tests listed, or a listed test does not exist;
//! * `failing`     — every listed test exists, but one fails or is ignored;
//! * `implemented` — every listed test exists and passes;
//! * `verified`    — implemented, and the latest evaluation (`evaluations/*.md`, highest
//!   `iteration`) that mentions the item lists it in `confirmed_items` and has a `### <id>` block.
//!
//! `--check` fails if the committed doc is stale, an item is malformed, a listed test does not
//! exist or does not pass, or an evaluation is malformed or names an unknown item.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;

use anyhow::{Context as _, Result, bail};
use serde::Deserialize;

use crate::testbins::{self, Outcome};
use crate::util::{StepResult, rel};

/// One `scorecard/items/<id>.toml`.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    /// Dotted id, equal to the file stem.
    pub id: String,
    /// One-line behaviour title.
    pub title: String,
    /// Owning workspace crate.
    #[serde(rename = "crate")]
    pub krate: String,
    /// Risk tier: low | medium | high.
    pub tier: String,
    /// Qualified test names (`kiln_text::buffer::tests::insert_at_start`).
    #[serde(default)]
    pub tests: Vec<String>,
}

/// Derived status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    /// Not yet backed by existing tests.
    Planned,
    /// Tests exist but do not all pass.
    Failing,
    /// Tests exist and pass.
    Implemented,
    /// Implemented and confirmed by the evaluator.
    Verified,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Status::Planned => "planned",
            Status::Failing => "failing",
            Status::Implemented => "implemented",
            Status::Verified => "verified",
        })
    }
}

/// What is known about one test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestState {
    /// No such test.
    Missing,
    /// Ran with this outcome.
    Ran(Outcome),
}

/// The pure derivation.
pub fn derive(item: &Item, tests: &BTreeMap<String, TestState>, confirmed: bool) -> Status {
    if item.tests.is_empty() {
        return Status::Planned;
    }
    let states: Vec<TestState> = item
        .tests
        .iter()
        .map(|t| tests.get(t).copied().unwrap_or(TestState::Missing))
        .collect();
    if states.contains(&TestState::Missing) {
        Status::Planned
    } else if states.iter().any(|s| *s != TestState::Ran(Outcome::Passed)) {
        Status::Failing
    } else if confirmed {
        Status::Verified
    } else {
        Status::Implemented
    }
}

fn valid_id(id: &str) -> bool {
    let parts: Vec<&str> = id.split('.').collect();
    parts.len() >= 2
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        })
}

/// Load items, returning (items, problems).
pub fn load_items(root: &Path, members: &BTreeSet<String>) -> Result<(Vec<Item>, Vec<String>)> {
    let dir = root.join("scorecard/items");
    let mut items = Vec::new();
    let mut problems = Vec::new();
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .with_context(|| format!("read {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    paths.sort();
    for p in paths {
        let r = rel(root, &p);
        if p.extension().and_then(|e| e.to_str()) != Some("toml") {
            problems.push(format!("{r}: not a .toml item file"));
            continue;
        }
        let item: Item = match std::fs::read_to_string(&p)
            .map_err(anyhow::Error::from)
            .and_then(|t| toml::from_str(&t).map_err(anyhow::Error::from))
        {
            Ok(i) => i,
            Err(e) => {
                problems.push(format!("{r}: {e}"));
                continue;
            }
        };
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
        if item.id != stem {
            problems.push(format!("{r}: id `{}` differs from the file name", item.id));
        }
        if !valid_id(&item.id) {
            problems.push(format!(
                "{r}: id `{}` must be dotted lowercase (`group.name`)",
                item.id
            ));
        }
        if item.title.trim().is_empty() {
            problems.push(format!("{r}: empty title"));
        }
        if !members.contains(&item.krate) {
            problems.push(format!(
                "{r}: crate `{}` is not a workspace crate",
                item.krate
            ));
        }
        if !matches!(item.tier.as_str(), "low" | "medium" | "high") {
            problems.push(format!("{r}: tier `{}` is not low|medium|high", item.tier));
        }
        let unique: BTreeSet<&String> = item.tests.iter().collect();
        if unique.len() != item.tests.len() {
            problems.push(format!("{r}: a test is listed twice"));
        }
        items.push(item);
    }
    if items.is_empty() {
        problems.push("scorecard/items is empty".to_owned());
    }
    Ok((items, problems))
}

/// Latest evaluator verdict per item: true = confirmed. Also returns problems.
pub fn load_evaluations(
    root: &Path,
    known: &BTreeSet<String>,
) -> Result<(BTreeMap<String, bool>, Vec<String>)> {
    let dir = root.join("evaluations");
    let mut problems = Vec::new();
    // item -> (iteration, confirmed, file)
    let mut latest: BTreeMap<String, (u64, bool)> = BTreeMap::new();
    if !dir.is_dir() {
        return Ok((BTreeMap::new(), problems));
    }
    let mut paths: Vec<_> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    paths.sort();
    for p in paths
        .into_iter()
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
    {
        let r = rel(root, &p);
        let text = std::fs::read_to_string(&p)?;
        let (fm, body) = match crate::frontmatter::parse(&text) {
            Ok(x) => x,
            Err(e) => {
                problems.push(format!("{r}: {e}"));
                continue;
            }
        };
        let Some(iteration) = fm
            .get("iteration")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<u64>().ok())
        else {
            problems.push(format!("{r}: front matter needs a numeric `iteration`"));
            continue;
        };
        let confirmed = fm
            .get("confirmed_items")
            .map(|v| v.as_list())
            .unwrap_or_default();
        let rejected = fm
            .get("rejected_items")
            .map(|v| v.as_list())
            .unwrap_or_default();
        let headings: BTreeSet<&str> = body
            .lines()
            .filter_map(|l| l.strip_prefix("### "))
            .map(str::trim)
            .collect();
        for (list, verdict) in [(&confirmed, true), (&rejected, false)] {
            for id in list {
                if !known.contains(id) {
                    problems.push(format!("{r}: unknown scorecard item `{id}`"));
                    continue;
                }
                if verdict && !headings.contains(id.as_str()) {
                    problems.push(format!(
                        "{r}: confirms `{id}` but has no `### {id}` record block"
                    ));
                    continue;
                }
                let e = latest.entry(id.clone()).or_insert((iteration, verdict));
                if iteration > e.0 || (iteration == e.0 && !verdict) {
                    *e = (iteration, verdict);
                }
            }
        }
    }
    Ok((
        latest.into_iter().map(|(k, (_, v))| (k, v)).collect(),
        problems,
    ))
}

fn md_escape(s: &str) -> String {
    s.replace('|', "\\|")
}

/// Render `docs/scorecard.md`.
pub fn render(rows: &[(Item, Status)]) -> String {
    let mut counts: BTreeMap<Status, usize> = BTreeMap::new();
    for (_, s) in rows {
        *counts.entry(*s).or_default() += 1;
    }
    let c = |s: Status| counts.get(&s).copied().unwrap_or(0);
    let mut out = String::new();
    out.push_str("# Scorecard\n\n");
    out.push_str("<!-- Generated by `cargo xtask scorecard` from scorecard/items/*.toml, test results and\n     evaluations/*.md. Do not edit; `cargo xtask scorecard --check` fails when this is stale. -->\n\n");
    out.push_str(&format!(
        "{} items: {} verified, {} implemented, {} failing, {} planned.\n\n",
        rows.len(),
        c(Status::Verified),
        c(Status::Implemented),
        c(Status::Failing),
        c(Status::Planned)
    ));
    out.push_str("| id | title | crate | tier | status | tests |\n|---|---|---|---|---|---|\n");
    for (item, status) in rows {
        let tests = if item.tests.is_empty() {
            "—".to_owned()
        } else {
            item.tests
                .iter()
                .map(|t| format!("`{t}`"))
                .collect::<Vec<_>>()
                .join("<br>")
        };
        out.push_str(&format!(
            "| `{}` | {} | {} | {} | {} | {} |\n",
            item.id,
            md_escape(&item.title),
            item.krate,
            item.tier,
            status,
            tests
        ));
    }
    out
}

fn workspace_members(root: &Path) -> Result<BTreeSet<String>> {
    let table: crate::layers::Table = crate::util::read_toml(&root.join("xtask/layers.toml"))?;
    Ok(table.crates.into_iter().map(|c| c.name).collect())
}

/// Resolve and run every listed test.
fn test_states(root: &Path, items: &[Item]) -> Result<BTreeMap<String, TestState>> {
    let wanted: BTreeSet<&String> = items.iter().flat_map(|i| &i.tests).collect();
    let mut states = BTreeMap::new();
    if wanted.is_empty() {
        return Ok(states);
    }
    let build = testbins::build(root, None)?;
    if !build.ok {
        bail!(
            "building test executables failed ({}):\n{}",
            build.failed.join(", "),
            crate::util::tail(&build.stderr, 40)
        );
    }
    // qualified name -> (bin index, unqualified)
    let mut index: BTreeMap<String, (usize, String)> = BTreeMap::new();
    for (i, bin) in build.bins.iter().enumerate() {
        for name in testbins::list(bin)? {
            let q = bin.qualify(&name);
            if let Some((j, _)) = index.insert(q.clone(), (i, name)) {
                bail!(
                    "test name `{q}` is ambiguous ({} and {})",
                    build.bins[j].exe.display(),
                    bin.exe.display()
                );
            }
        }
    }
    let mut per_bin: BTreeMap<usize, Vec<(String, String)>> = BTreeMap::new();
    for w in wanted {
        match index.get(w) {
            Some((i, name)) => per_bin
                .entry(*i)
                .or_default()
                .push((w.clone(), name.clone())),
            None => {
                states.insert(w.clone(), TestState::Missing);
            }
        }
    }
    for (i, tests) in per_bin {
        let names: Vec<String> = tests.iter().map(|(_, n)| n.clone()).collect();
        let results = testbins::run(&build.bins[i], &names)?;
        for (q, n) in tests {
            let o = results.get(&n).copied().unwrap_or(Outcome::Failed);
            states.insert(q, TestState::Ran(o));
        }
    }
    Ok(states)
}

/// Derive, then write the doc (`check == false`) or compare with it (`check == true`).
pub fn run(root: &Path, check: bool) -> Result<StepResult> {
    let members = workspace_members(root)?;
    let (items, mut problems) = load_items(root, &members)?;
    let known: BTreeSet<String> = items.iter().map(|i| i.id.clone()).collect();
    let (verdicts, eval_problems) = load_evaluations(root, &known)?;
    problems.extend(eval_problems);
    let states = test_states(root, &items)?;
    let mut rows = Vec::new();
    for item in &items {
        for t in &item.tests {
            match states.get(t) {
                Some(TestState::Missing) => problems.push(format!(
                    "{}: lists test `{t}`, which does not exist",
                    item.id
                )),
                Some(TestState::Ran(Outcome::Failed)) => {
                    problems.push(format!("{}: test `{t}` fails", item.id))
                }
                Some(TestState::Ran(Outcome::Ignored)) => {
                    problems.push(format!("{}: test `{t}` is ignored", item.id))
                }
                _ => {}
            }
        }
        let status = derive(
            item,
            &states,
            verdicts.get(&item.id).copied().unwrap_or(false),
        );
        rows.push((item.clone(), status));
    }
    let doc = render(&rows);
    let path = root.join("docs/scorecard.md");
    let summary = doc
        .lines()
        .find(|l| l.contains(" items: "))
        .unwrap_or_default()
        .to_owned();
    if check {
        let current = std::fs::read_to_string(&path).unwrap_or_default();
        if current != doc {
            problems.push(
                "docs/scorecard.md is stale; run `cargo xtask scorecard` and commit it".to_owned(),
            );
        }
    } else {
        std::fs::write(&path, &doc)?;
    }
    Ok(StepResult::from_problems(
        summary,
        "scorecard problems",
        &problems,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(tests: &[&str]) -> Item {
        Item {
            id: "text.insert".into(),
            title: "t".into(),
            krate: "kiln-text".into(),
            tier: "medium".into(),
            tests: tests.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn derivation_rules() {
        let mut st = BTreeMap::new();
        st.insert("a".to_string(), TestState::Ran(Outcome::Passed));
        st.insert("b".to_string(), TestState::Ran(Outcome::Failed));
        st.insert("c".to_string(), TestState::Ran(Outcome::Ignored));
        st.insert("m".to_string(), TestState::Missing);
        assert_eq!(derive(&item(&[]), &st, true), Status::Planned);
        assert_eq!(derive(&item(&["a", "m"]), &st, true), Status::Planned);
        assert_eq!(derive(&item(&["a", "zzz"]), &st, false), Status::Planned);
        assert_eq!(derive(&item(&["a", "b"]), &st, true), Status::Failing);
        assert_eq!(derive(&item(&["c"]), &st, false), Status::Failing);
        assert_eq!(derive(&item(&["a"]), &st, false), Status::Implemented);
        assert_eq!(derive(&item(&["a"]), &st, true), Status::Verified);
    }

    #[test]
    fn render_is_stable_and_counts() {
        let doc = render(&[
            (item(&["a"]), Status::Implemented),
            (item(&[]), Status::Planned),
        ]);
        assert!(doc.contains("2 items: 0 verified, 1 implemented, 0 failing, 1 planned."));
        assert!(doc.contains("| `text.insert` | t | kiln-text | medium | implemented | `a` |"));
        assert_eq!(
            doc,
            render(&[
                (item(&["a"]), Status::Implemented),
                (item(&[]), Status::Planned)
            ])
        );
    }

    #[test]
    fn items_are_validated() {
        let d = crate::util::testutil::TempDir::new("score-items");
        let dir = d.path().join("scorecard/items");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("text.insert.toml"),
            "id = \"text.insert\"\ntitle = \"x\"\ncrate = \"kiln-text\"\ntier = \"medium\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("bad.name.toml"),
            "id = \"other\"\ntitle = \"\"\ncrate = \"nope\"\ntier = \"urgent\"\n",
        )
        .unwrap();
        std::fs::write(dir.join("x.status.toml"), "id = \"x.status\"\ntitle = \"x\"\ncrate = \"kiln-text\"\ntier = \"low\"\nstatus = \"verified\"\n").unwrap();
        let members = BTreeSet::from(["kiln-text".to_string()]);
        let (items, problems) = load_items(d.path(), &members).unwrap();
        assert_eq!(items.len(), 2);
        // bad.name: id mismatch, id format, empty title, crate, tier; x.status: hand-set status rejected.
        assert_eq!(problems.len(), 6, "{problems:?}");
        assert!(
            problems
                .iter()
                .any(|p| p.contains("unknown field `status`")),
            "{problems:?}"
        );
    }

    #[test]
    fn evaluations_latest_verdict_wins_and_need_record_blocks() {
        let d = crate::util::testutil::TempDir::new("score-evals");
        let dir = d.path().join("evaluations");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("iter-0.md"), "---\niteration: 0\nconfirmed_items: [a.one, a.two]\nrejected_items: []\n---\n### a.one\nok\n### a.two\nok\n").unwrap();
        std::fs::write(dir.join("iter-1.md"), "---\niteration: 1\nconfirmed_items: [a.three, b.unknown]\nrejected_items: [a.two]\n---\n# no blocks\n").unwrap();
        let known: BTreeSet<String> = ["a.one", "a.two", "a.three"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let (v, problems) = load_evaluations(d.path(), &known).unwrap();
        assert_eq!(v.get("a.one"), Some(&true));
        assert_eq!(v.get("a.two"), Some(&false));
        assert_eq!(v.get("a.three"), None);
        assert_eq!(problems.len(), 2, "{problems:?}");
    }
}
