//! Gate step `hygiene` (ADR-0003, T-013 Interface (6)): two cheap text checks of the checkout.
//!
//! - `reviews`: every `reviews/T-NNN-rN.md` of a medium/high task of iteration ≥ 1 carries the
//!   v1.1 review front matter (`task`, `round`, `tier`, `arm` and the five `mutants` counts,
//!   which must add up; at least 10 introduced in round 1). Iteration-0 and low-tier reviews are
//!   not checked; a review without a task file fails.
//! - `session-paths`: no file listed by `git ls-files` contains a session-specific scratch path
//!   (METHODOLOGY §12). A match is reported as `<path>:<line>`, never with the matched text.
//!
//! There is no allowlist for either check (golden rule 4). The step is never EMPTY: the second
//! check always runs.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;

use crate::frontmatter::{self, Value};
use crate::util::StepResult;

/// The five keys of a review's `mutants` front matter (METHODOLOGY §7).
pub const MUTANT_KEYS: &[&str] = &[
    "introduced",
    "equivalent",
    "killed",
    "killed_by_acceptance",
    "survived",
];

/// The review arm of a task (METHODOLOGY §6, T-013 Interface (4)): `independent` for high and
/// for medium with an even number, `implementer` for medium with an odd number, none for low.
/// Ruling D-047 (o): a task id that does not end in an ASCII digit has no arm (never a panic).
pub fn arm(task: &str, tier: &str) -> Option<&'static str> {
    let last = task.bytes().last().filter(u8::is_ascii_digit)?;
    let odd = (last - b'0') % 2 == 1;
    match tier {
        "high" => Some("independent"),
        "medium" if odd => Some("implementer"),
        "medium" => Some("independent"),
        _ => None,
    }
}

/// `T-NNN-rN.md` → (`T-NNN`, N).
fn review_name(file: &str) -> Option<(String, u64)> {
    let stem = file.strip_suffix(".md")?;
    let (task, round) = stem.rsplit_once("-r")?;
    if !crate::util::valid_task_id(task)
        || round.is_empty()
        || !round.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    Some((task.to_owned(), round.parse().ok()?))
}

/// The raw text between the opening and closing `---` lines.
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

fn strip_comment(s: &str) -> &str {
    match s.find(" #") {
        Some(i) => &s[..i],
        None if s.trim_start().starts_with('#') => "",
        None => s,
    }
}

/// The `mutants` map of a review's raw front matter: a flow map `{k: v, …}` on the key's line
/// (the template's form) or an indented block of `k: v` lines under `mutants:`.
/// `Ok(None)`: no `mutants` key. `Err`: present but malformed.
pub fn mutants_map(raw: &str) -> Result<Option<BTreeMap<String, String>>, String> {
    let lines: Vec<&str> = raw.lines().collect();
    let Some(i) = lines.iter().position(|l| {
        !l.starts_with(char::is_whitespace)
            && l.split_once(':')
                .is_some_and(|(k, _)| k.trim() == "mutants")
    }) else {
        return Ok(None);
    };
    let rest = strip_comment(lines[i].split_once(':').map_or("", |(_, v)| v)).trim();
    let mut map = BTreeMap::new();
    let mut add = |pair: &str| -> Result<(), String> {
        let Some((k, v)) = pair.split_once(':') else {
            return Err(format!(
                "`mutants` entry `{}` is not `key: value`",
                pair.trim()
            ));
        };
        let k = k.trim().to_owned();
        if map.insert(k.clone(), v.trim().to_owned()).is_some() {
            return Err(format!("`mutants` has `{k}` twice"));
        }
        Ok(())
    };
    if let Some(inner) = rest.strip_prefix('{') {
        let Some(inner) = inner.strip_suffix('}') else {
            return Err("`mutants` flow map is not closed with `}`".into());
        };
        for pair in inner.split(',').filter(|p| !p.trim().is_empty()) {
            add(pair)?;
        }
    } else if rest.is_empty() || rest == "null" || rest == "~" {
        for l in lines[i + 1..].iter() {
            if !l.starts_with(char::is_whitespace) {
                break;
            }
            let l = strip_comment(l);
            if l.trim().is_empty() {
                continue;
            }
            add(l)?;
        }
    } else {
        return Err(format!("`mutants` is `{rest}`, not a map"));
    }
    if map.is_empty() {
        return Err("`mutants` is empty".into());
    }
    Ok(Some(map))
}

/// Task tier and iteration from the task file text.
fn task_meta(text: &str) -> Result<(String, Option<u64>), String> {
    let (fm, _) = frontmatter::parse(text).map_err(|e| e.to_string())?;
    let tier = fm
        .get("tier")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let iteration = fm
        .get("iteration")
        .and_then(Value::as_str)
        .and_then(|s| s.parse().ok());
    Ok((tier, iteration))
}

/// The problems of one review file (pure; `task_text` is its task file, if any).
pub fn review_problems(file: &str, text: &str, task_text: Option<&str>) -> Vec<String> {
    let Some((task, round)) = review_name(file) else {
        return Vec::new();
    };
    let p = |s: String| format!("reviews/{file}: {s}");
    let Some(task_text) = task_text else {
        return vec![p(format!("no tasks/{task}-*.md for this review"))];
    };
    let (tier, iteration) = match task_meta(task_text) {
        Ok(m) => m,
        Err(e) => return vec![p(format!("task file front matter: {e}"))],
    };
    if !matches!(tier.as_str(), "medium" | "high") || iteration.is_none_or(|i| i < 1) {
        return Vec::new();
    }
    let mut out = Vec::new();
    let fm = match frontmatter::parse(text) {
        Ok((m, _)) => m,
        Err(e) => return vec![p(format!("front matter does not parse: {e}"))],
    };
    let get = |k: &str| fm.get(k).and_then(Value::as_str).unwrap_or_default();
    if get("task") != task {
        out.push(p(format!("`task` is `{}`, not `{task}`", get("task"))));
    }
    if get("round").parse::<u64>().ok() != Some(round) {
        out.push(p(format!("`round` is `{}`, not `{round}`", get("round"))));
    }
    if get("tier") != tier {
        out.push(p(format!(
            "`tier` is `{}`, but the task's tier is `{tier}`",
            get("tier")
        )));
    }
    let want_arm = arm(&task, &tier).unwrap_or_default();
    if get("arm") != want_arm {
        out.push(p(format!(
            "`arm` is `{}`, but the task's arm is `{want_arm}`",
            get("arm")
        )));
    }
    let raw = raw_front_matter(text).unwrap_or_default();
    match mutants_map(&raw) {
        Ok(None) => out.push(p("no `mutants` front matter".into())),
        Err(e) => out.push(p(e)),
        Ok(Some(m)) => {
            let mut n = BTreeMap::new();
            for k in MUTANT_KEYS {
                match m.get(*k) {
                    None => out.push(p(format!("`mutants` lacks `{k}`"))),
                    Some(v) => match v.parse::<u64>() {
                        Ok(x) if v.bytes().all(|b| b.is_ascii_digit()) => {
                            n.insert(*k, x);
                        }
                        _ => out.push(p(format!(
                            "`mutants.{k}` is `{v}`, not a non-negative integer"
                        ))),
                    },
                }
            }
            if n.len() == MUTANT_KEYS.len() {
                let (i, e, k, ka, s) = (
                    n["introduced"],
                    n["equivalent"],
                    n["killed"],
                    n["killed_by_acceptance"],
                    n["survived"],
                );
                if e.checked_add(k).and_then(|x| x.checked_add(s)) != Some(i) {
                    out.push(p(format!(
                        "`mutants`: equivalent + killed + survived = {} != introduced {i}",
                        u128::from(e) + u128::from(k) + u128::from(s)
                    )));
                }
                if ka > k {
                    out.push(p(format!(
                        "`mutants`: killed_by_acceptance {ka} > killed {k}"
                    )));
                }
                if round == 1 && i < 10 {
                    out.push(p(format!(
                        "`mutants`: introduced {i} < 10 in round 1 (METHODOLOGY §7)"
                    )));
                }
            }
        }
    }
    out
}

/// Check `reviews`: (problems, review files seen, of which checked).
fn check_reviews(root: &Path) -> Result<(Vec<String>, usize, usize)> {
    let dir = root.join("reviews");
    let mut problems = Vec::new();
    let (mut seen, mut checked) = (0, 0);
    if !dir.is_dir() {
        return Ok((problems, seen, checked));
    }
    let mut tasks: BTreeMap<String, Vec<std::path::PathBuf>> = BTreeMap::new();
    if root.join("tasks").is_dir() {
        for e in std::fs::read_dir(root.join("tasks"))? {
            let path = e?.path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_owned();
            if let Some(rest) = name.strip_prefix("T-")
                && name.ends_with(".md")
            {
                let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
                if rest[digits.len()..].starts_with('-') {
                    tasks.entry(format!("T-{digits}")).or_default().push(path);
                }
            }
        }
    }
    let mut files: Vec<String> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter_map(|e| e.file_name().to_str().map(str::to_owned))
        .collect();
    files.sort();
    for file in files {
        let Some((task, _)) = review_name(&file) else {
            continue;
        };
        seen += 1;
        let text = match std::fs::read_to_string(dir.join(&file)) {
            Ok(t) => t,
            Err(e) => {
                problems.push(format!("reviews/{file}: cannot read: {e}"));
                continue;
            }
        };
        let task_text = match tasks.get(&task).map(Vec::as_slice) {
            None | Some([]) => None,
            Some([one]) => Some(std::fs::read_to_string(one)?),
            Some(many) => {
                problems.push(format!(
                    "reviews/{file}: {} task files match tasks/{task}-*.md",
                    many.len()
                ));
                continue;
            }
        };
        let applies = task_text.as_deref().is_none_or(|t| {
            task_meta(t).is_ok_and(|(tier, it)| {
                matches!(tier.as_str(), "medium" | "high") && it.is_some_and(|i| i >= 1)
            })
        });
        if applies {
            checked += 1;
        }
        problems.extend(review_problems(&file, &text, task_text.as_deref()));
    }
    Ok((problems, seen, checked))
}

fn is_posix_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\x0b' | '\x0c' | '\r')
}

/// Whether `s` starts with the UUID shape `8-4-4-4-12` of lowercase hex digits.
fn starts_with_uuid(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 36 {
        return false;
    }
    b[..36].iter().enumerate().all(|(i, c)| match i {
        8 | 13 | 18 | 23 => *c == b'-',
        _ => c.is_ascii_digit() || (b'a'..=b'f').contains(c),
    })
}

/// Whether `line` contains a match of
/// `/tmp/claude-[0-9]+/[^/[:space:]]+/<uuid>` (T-013 Interface (6)).
pub fn has_session_path(line: &str) -> bool {
    const P: &str = "/tmp/claude-";
    let mut from = 0;
    while let Some(i) = line[from..].find(P) {
        let start = from + i;
        from = start + 1;
        let rest = &line[start + P.len()..];
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 || !rest[digits..].starts_with('/') {
            continue;
        }
        let seg = &rest[digits + 1..];
        let len: usize = seg
            .chars()
            .take_while(|c| *c != '/' && !is_posix_space(*c))
            .map(char::len_utf8)
            .sum();
        if len == 0 || !seg[len..].starts_with('/') {
            continue;
        }
        if starts_with_uuid(&seg[len + 1..]) {
            return true;
        }
    }
    false
}

/// Check `session-paths`: (problems, tracked files scanned, files skipped as not UTF-8).
fn check_session_paths(root: &Path) -> Result<(Vec<String>, usize, usize)> {
    let (ok, out, err) = crate::git::git_try(root, &["ls-files", "-z"])?;
    if !ok {
        anyhow::bail!("git ls-files failed in {}: {}", root.display(), err.trim());
    }
    let mut problems = Vec::new();
    let (mut scanned, mut skipped) = (0, 0);
    for path in out.split('\0').filter(|p| !p.is_empty()) {
        let full = root.join(path);
        let Ok(meta) = std::fs::symlink_metadata(&full) else {
            continue; // tracked but deleted in the working tree
        };
        if meta.file_type().is_symlink() {
            // Ruling D-047 (o): git stores a symlink as its target text; check that text.
            let Ok(target) = std::fs::read_link(&full) else {
                continue;
            };
            let Some(text) = target.to_str() else {
                skipped += 1;
                continue;
            };
            scanned += 1;
            for (n, line) in text.lines().enumerate() {
                if has_session_path(line) {
                    problems.push(format!(
                        "session-paths: {path}:{}: contains a session-specific scratch path",
                        n + 1
                    ));
                }
            }
            continue;
        }
        if !meta.is_file() {
            continue; // submodule or other non-file entry
        }
        let bytes = std::fs::read(&full)?;
        let Ok(text) = String::from_utf8(bytes) else {
            skipped += 1;
            continue;
        };
        scanned += 1;
        for (n, line) in text.lines().enumerate() {
            if has_session_path(line) {
                problems.push(format!(
                    "session-paths: {path}:{}: contains a session-specific scratch path",
                    n + 1
                ));
            }
        }
    }
    Ok((problems, scanned, skipped))
}

/// The `hygiene` step.
pub fn check(root: &Path) -> Result<StepResult> {
    let (mut problems, reviews, checked) = check_reviews(root)?;
    for p in &mut problems {
        *p = format!("reviews: {p}");
    }
    let (paths, scanned, skipped) = check_session_paths(root)?;
    problems.extend(paths);
    Ok(StepResult::from_problems(
        format!(
            "reviews: {checked} of {reviews} review file(s) checked (medium/high, iteration >= 1), ok; session-paths: {scanned} tracked file(s) clean ({skipped} not UTF-8, skipped)"
        ),
        "hygiene problems",
        &problems,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session_path() -> String {
        // Built at run time so no tracked file contains one (T-013 Interface (6)).
        format!(
            "/tmp/claude-{}/-home-x/{}-{}-{}-{}-{}/scratchpad/m.py",
            0, "0123abcd", "4567", "89ab", "cdef", "0123456789ab"
        )
    }

    #[test]
    fn session_path_regex() {
        assert!(has_session_path(&format!("see {}", session_path())));
        assert!(!has_session_path(
            "/tmp/claude-[0-9]+/[^/[:space:]]+/[0-9a-f]{8}-[0-9a-f]{4}"
        ));
        let p = session_path();
        assert!(!has_session_path(&p.replace("claude-0", "claude-")));
        assert!(!has_session_path(&p.replace("-home-x", "")));
        assert!(!has_session_path(&p.replace("-home-x", "a b")));
        assert!(!has_session_path(&p.replace("0123abcd", "0123ABCD")));
        assert!(!has_session_path(&p.replace("0123abcd", "0123abc")));
        // A first near-miss does not hide a later match on the same line.
        assert!(has_session_path(&format!("/tmp/claude-x/ {p}")));
    }

    const TASK: &str = "---\nid: T-021\ntier: medium\niteration: 1\n---\n";
    const GOOD: &str = "---\ntask: T-021\nround: 1\ntier: medium\narm: implementer\nmutants: {introduced: 12, equivalent: 1, killed: 10, killed_by_acceptance: 9, survived: 1}\n---\n";

    #[test]
    fn review_front_matter_rules() {
        assert!(review_problems("T-021-r1.md", GOOD, Some(TASK)).is_empty());
        let block = GOOD.replace(
            "mutants: {introduced: 12, equivalent: 1, killed: 10, killed_by_acceptance: 9, survived: 1}",
            "mutants:\n  introduced: 12\n  equivalent: 1\n  killed: 10\n  killed_by_acceptance: 9\n  survived: 1",
        );
        assert!(review_problems("T-021-r1.md", &block, Some(TASK)).is_empty());
        let bad = |from: &str, to: &str, file: &str| {
            let v = review_problems(file, &GOOD.replace(from, to), Some(TASK));
            assert!(!v.is_empty(), "{from} -> {to} accepted");
            assert!(
                v.iter()
                    .all(|p| p.starts_with(&format!("reviews/{file}: ")))
            );
        };
        bad(
            "mutants: {introduced: 12, equivalent: 1, killed: 10, killed_by_acceptance: 9, survived: 1}\n",
            "",
            "T-021-r1.md",
        );
        bad(", survived: 1", "", "T-021-r1.md");
        bad(
            "introduced: 12, equivalent: 1",
            "introduced: 9, equivalent: 1",
            "T-021-r1.md",
        );
        bad("equivalent: 1", "equivalent: 2", "T-021-r1.md");
        bad(
            "killed_by_acceptance: 9",
            "killed_by_acceptance: 11",
            "T-021-r1.md",
        );
        bad("arm: implementer", "arm: independent", "T-021-r1.md");
        bad("tier: medium", "tier: high", "T-021-r1.md");
        bad("round: 1", "round: 2", "T-021-r1.md");
        bad("equivalent: 1", "equivalent: -1", "T-021-r1.md");
        // introduced 9 is fine in round 2.
        let r2 = GOOD
            .replace("round: 1", "round: 2")
            .replace(
                "introduced: 12, equivalent: 1",
                "introduced: 9, equivalent: 1",
            )
            .replace("killed: 10", "killed: 7")
            .replace("killed_by_acceptance: 9", "killed_by_acceptance: 7");
        assert!(review_problems("T-021-r2.md", &r2, Some(TASK)).is_empty());
        // Not checked: iteration 0, low tier, other names. Checked: a review without a task.
        let old = "---\ntask: T-021\nround: 1\n---\n";
        assert!(
            review_problems(
                "T-021-r1.md",
                old,
                Some(&TASK.replace("iteration: 1", "iteration: 0"))
            )
            .is_empty()
        );
        assert!(
            review_problems("T-021-r1.md", old, Some(&TASK.replace("medium", "low"))).is_empty()
        );
        assert!(review_problems("T-021-join-triage.md", old, None).is_empty());
        assert_eq!(review_problems("T-021-r1.md", GOOD, None).len(), 1);
    }

    #[test]
    fn arms() {
        assert_eq!(arm("T-021", "medium"), Some("implementer"));
        assert_eq!(arm("T-020", "medium"), Some("independent"));
        assert_eq!(arm("T-021", "high"), Some("independent"));
        assert_eq!(arm("T-021", "low"), None);
        // Ruling D-047 (o): no panic, no arm, when the id does not end in an ASCII digit.
        for bad in ["T-021 ", "T-02!", "T-", "", "T-02\u{0663}", "T-021a"] {
            for tier in ["high", "medium", "low"] {
                assert_eq!(arm(bad, tier), None, "{bad:?} {tier}");
            }
        }
    }

    #[test]
    fn session_paths_checks_a_tracked_symlinks_target() {
        use crate::git::testrepo;
        let repo = testrepo::init("hyg-link", &[("a.txt", "clean\n")]);
        let root = repo.path();
        std::os::unix::fs::symlink(session_path(), root.join("link")).unwrap();
        std::os::unix::fs::symlink("a.txt", root.join("ok-link")).unwrap();
        // Untracked: not checked.
        let (problems, _, _) = check_session_paths(root).unwrap();
        assert!(problems.is_empty(), "{problems:?}");
        testrepo::g(root, &["add", "link", "ok-link"]);
        let (problems, scanned, _) = check_session_paths(root).unwrap();
        assert_eq!(
            problems,
            ["session-paths: link:1: contains a session-specific scratch path"]
        );
        assert_eq!(scanned, 3);
        assert!(!problems[0].contains("/tmp/claude-"));
    }
}
