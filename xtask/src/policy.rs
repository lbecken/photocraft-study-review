//! The queue's two policy files, always read from `main`'s tree (never from a task branch):
//!
//! - `xtask/generated.toml` — generated paths and the command that regenerates each (D-1);
//! - `xtask/ledger.toml` — the ledger paths the director may commit to on `main` directly (D-4).
//!
//! Both are parsed strictly: unknown keys, empty lists, empty commands, duplicate or malformed
//! paths, a path listed in both files, or a ledger entry under `xtask/`, `.github/` or `Cargo.*`
//! are errors.

use std::collections::BTreeSet;

use anyhow::{Context as _, Result, bail};
use serde::Deserialize;

/// Path of the generated-file policy in a checkout.
pub const GENERATED_FILE: &str = "xtask/generated.toml";
/// Path of the ledger policy in a checkout.
pub const LEDGER_FILE: &str = "xtask/ledger.toml";

/// One generated path.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Generated {
    /// Repository path of the generated file.
    pub path: String,
    /// argv that regenerates it in a checkout; the token `{xtask}` stands for main's xtask.
    pub command: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GeneratedFile {
    generated: Vec<Generated>,
}

/// What the director may do to a ledger path on `main`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    /// Add, modify or delete.
    Any,
    /// Only add new files.
    AddOnly,
}

/// One ledger entry.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LedgerEntry {
    /// A file or directory (a trailing `/` is ignored when matching).
    pub path: String,
    /// `any` or `add-only`.
    pub mode: Mode,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LedgerFile {
    ledger: Vec<LedgerEntry>,
}

/// Both policies, validated together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Policy {
    /// Generated paths, in file order.
    pub generated: Vec<Generated>,
    /// Ledger entries, in file order.
    pub ledger: Vec<LedgerEntry>,
}

/// Whether repository path `p` is `entry` or lies below it (`entry` may end in `/`).
pub fn path_under(p: &str, entry: &str) -> bool {
    let e = entry.trim_end_matches('/');
    !e.is_empty() && (p == e || p.strip_prefix(e).is_some_and(|r| r.starts_with('/')))
}

fn check_path(p: &str, what: &str) -> Result<String> {
    let n = p.trim_end_matches('/');
    let bad = n.is_empty()
        || n.starts_with('/')
        || n.contains('\\')
        || n.split('/').any(|s| s.is_empty() || s == "." || s == "..");
    if bad {
        bail!("{what}: `{p}` is not a relative repository path");
    }
    Ok(n.to_owned())
}

impl Policy {
    /// Parse and validate the two files' contents (`None` = file missing).
    pub fn parse(generated: Option<&str>, ledger: Option<&str>) -> Result<Self> {
        let generated = generated.with_context(|| format!("{GENERATED_FILE} is missing"))?;
        let ledger = ledger.with_context(|| format!("{LEDGER_FILE} is missing"))?;
        let g: GeneratedFile =
            toml::from_str(generated).with_context(|| format!("parse {GENERATED_FILE}"))?;
        let l: LedgerFile =
            toml::from_str(ledger).with_context(|| format!("parse {LEDGER_FILE}"))?;
        if g.generated.is_empty() {
            bail!("{GENERATED_FILE}: `generated` is empty");
        }
        if l.ledger.is_empty() {
            bail!("{LEDGER_FILE}: `ledger` is empty");
        }
        let mut gen_paths = BTreeSet::new();
        for e in &g.generated {
            let n = check_path(&e.path, GENERATED_FILE)?;
            if e.path.ends_with('/') {
                bail!("{GENERATED_FILE}: `{}` must be a file path", e.path);
            }
            if e.command.is_empty() || e.command[0].trim().is_empty() {
                bail!("{GENERATED_FILE}: `{}` has an empty command", e.path);
            }
            if !gen_paths.insert(n) {
                bail!("{GENERATED_FILE}: `{}` is listed twice", e.path);
            }
        }
        let mut led_paths = BTreeSet::new();
        for e in &l.ledger {
            let n = check_path(&e.path, LEDGER_FILE)?;
            let first = n.split('/').next().unwrap_or_default();
            if first == "xtask" || first == ".github" || first.starts_with("Cargo.") {
                bail!(
                    "{LEDGER_FILE}: `{}` can never be a ledger path (xtask/, .github/, Cargo.*)",
                    e.path
                );
            }
            if gen_paths.contains(&n) {
                bail!("`{}` is in both {GENERATED_FILE} and {LEDGER_FILE}", e.path);
            }
            if !led_paths.insert(n) {
                bail!("{LEDGER_FILE}: `{}` is listed twice", e.path);
            }
        }
        Ok(Policy {
            generated: g.generated,
            ledger: l.ledger,
        })
    }

    /// Whether `p` is a generated path (exact match).
    pub fn is_generated(&self, p: &str) -> bool {
        self.generated.iter().any(|g| g.path == p)
    }

    /// Whether a commit on main may make change `status` (`A`, `M`, `D`, …) to path `p`.
    pub fn ledger_allows(&self, p: &str, status: char) -> bool {
        self.ledger.iter().any(|e| {
            path_under(p, &e.path)
                && match e.mode {
                    Mode::Any => true,
                    Mode::AddOnly => status == 'A',
                }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GEN: &str =
        "[[generated]]\npath = \"docs/s.md\"\ncommand = [\"{xtask}\", \"scorecard\"]\n";
    const LED: &str = "[[ledger]]\npath = \"tasks/\"\nmode = \"any\"\n\n[[ledger]]\npath = \"scorecard/items/\"\nmode = \"add-only\"\n";

    #[test]
    fn path_under_is_segment_based() {
        assert!(path_under("crates/a/src/x.rs", "crates/a"));
        assert!(path_under("crates/a/src/x.rs", "crates/a/"));
        assert!(path_under("crates/a", "crates/a"));
        assert!(!path_under("crates/ab/x.rs", "crates/a"));
        assert!(!path_under("crates", "crates/a"));
        assert!(!path_under("x", ""));
    }

    #[test]
    fn repository_files_parse() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let read = |f: &str| std::fs::read_to_string(root.join(f.trim_start_matches("xtask/")));
        let p = Policy::parse(
            Some(&read(GENERATED_FILE).unwrap()),
            Some(&read(LEDGER_FILE).unwrap()),
        )
        .unwrap();
        let paths: Vec<&str> = p.generated.iter().map(|g| g.path.as_str()).collect();
        assert_eq!(
            paths,
            ["Cargo.lock", "docs/commands.json", "docs/scorecard.md"]
        );
        assert!(p.ledger_allows("STATE.md", 'M'));
        assert!(p.ledger_allows("scorecard/items/x.toml", 'A'));
        assert!(!p.ledger_allows("scorecard/items/x.toml", 'M'));
        assert!(!p.ledger_allows("xtask/src/main.rs", 'M'));
        assert!(!p.ledger_allows("Cargo.lock", 'M'));
        assert!(!p.ledger_allows("docs/development.md", 'M'));
        assert!(p.ledger_allows("docs/decisions.md", 'M'));
    }

    #[test]
    fn modes_and_generated() {
        let p = Policy::parse(Some(GEN), Some(LED)).unwrap();
        assert!(p.is_generated("docs/s.md"));
        assert!(!p.is_generated("docs"));
        assert!(p.ledger_allows("tasks/T-001-x.md", 'D'));
        assert!(p.ledger_allows("scorecard/items/new.toml", 'A'));
        assert!(!p.ledger_allows("scorecard/items/old.toml", 'D'));
    }

    #[test]
    fn strictness() {
        let bad = |g: &str, l: &str| Policy::parse(Some(g), Some(l)).unwrap_err().to_string();
        assert!(Policy::parse(None, Some(LED)).is_err());
        assert!(Policy::parse(Some(GEN), None).is_err());
        assert!(bad("generated = []\n", LED).contains("empty"));
        assert!(bad(GEN, "ledger = []\n").contains("empty"));
        bad(&format!("{GEN}extra = 1\n"), LED);
        bad(GEN, &format!("{LED}color = \"red\"\n"));
        bad("[[generated]]\npath = \"a\"\ncommand = []\n", LED);
        bad(
            "[[generated]]\npath = \"a\"\ncommand = [\"x\"]\nwhy = \"y\"\n",
            LED,
        );
        bad(GEN, "[[ledger]]\npath = \"tasks/\"\nmode = \"sometimes\"\n");
        for p in [
            "xtask/",
            "xtask/ledger.toml",
            ".github",
            "Cargo.toml",
            "Cargo.lock",
            "../x",
            "/abs",
            "",
        ] {
            let l = format!("[[ledger]]\npath = \"{p}\"\nmode = \"any\"\n");
            bad(GEN, &l);
        }
        let both = "[[ledger]]\npath = \"docs/s.md\"\nmode = \"any\"\n";
        assert!(bad(GEN, both).contains("both"));
        let twice = format!("{GEN}{}", GEN.replace("[[generated]]", "\n[[generated]]"));
        assert!(bad(&twice, LED).contains("twice"));
        assert!(bad("not toml [", LED).contains("parse"));
    }
}
