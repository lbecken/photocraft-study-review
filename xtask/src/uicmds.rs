//! `cargo xtask ui-commands`: `kiln-ui` may name lower-crate items only through an explicit
//! allowlist (`xtask/ui-allowlist.toml`), so it can dispatch commands but not call mutating APIs.
//!
//! The scan works on tokens (comments dropped, string literals ignored), walks into every group
//! including macro arguments, and expands `use` trees (`a::{b, c::d}`). Limitation: it sees
//! paths, not method calls on values; the allowlist must therefore only admit types whose
//! public API is read-only or dispatch-only.

use std::collections::BTreeSet;
use std::path::Path;
use std::str::FromStr;

use anyhow::{Context as _, Result, anyhow, bail};
use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};
use serde::Deserialize;

use crate::layers::Table;
use crate::util::{StepResult, read_toml, rel, rust_files};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AllowFile {
    #[serde(default)]
    allow: Vec<Allow>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Allow {
    path: String,
    reason: String,
}

/// A lower-crate path named in a source file.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Found {
    /// 1-based line.
    pub line: usize,
    /// The full path, segments joined by `::`.
    pub path: String,
}

fn is_path_sep(tokens: &[TokenTree], i: usize) -> bool {
    matches!(
        (tokens.get(i), tokens.get(i + 1)),
        (Some(TokenTree::Punct(a)), Some(TokenTree::Punct(b)))
            if a.as_char() == ':' && a.spacing() == Spacing::Joint && b.as_char() == ':'
    )
}

/// Parse the path continuing after `prefix` at `tokens[i]`; push every complete path found.
/// Returns the index after the path.
fn parse_tail(
    tokens: &[TokenTree],
    mut i: usize,
    prefix: Vec<String>,
    out: &mut Vec<(usize, Vec<String>)>,
    line: usize,
) -> usize {
    let mut path = prefix;
    loop {
        if !is_path_sep(tokens, i) {
            out.push((line, path));
            return i;
        }
        match tokens.get(i + 2) {
            Some(TokenTree::Ident(id)) => {
                path.push(id.to_string());
                i += 3;
            }
            Some(TokenTree::Punct(p)) if p.as_char() == '*' => {
                path.push("*".into());
                out.push((line, path));
                return i + 3;
            }
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => {
                let inner: Vec<TokenTree> = g.stream().into_iter().collect();
                for part in inner.split(|t| matches!(t, TokenTree::Punct(p) if p.as_char() == ','))
                {
                    match part.first() {
                        Some(TokenTree::Ident(id)) if id == "self" => {
                            out.push((line, path.clone()));
                        }
                        Some(TokenTree::Ident(id)) => {
                            let mut p = path.clone();
                            p.push(id.to_string());
                            parse_tail(part, 1, p, out, line);
                        }
                        Some(TokenTree::Punct(p)) if p.as_char() == '*' => {
                            let mut q = path.clone();
                            q.push("*".into());
                            out.push((line, q));
                        }
                        _ => {}
                    }
                }
                return i + 3;
            }
            _ => {
                // `Foo::<T>` or similar: the path ends here.
                out.push((line, path));
                return i;
            }
        }
    }
}

fn scan_stream(
    stream: TokenStream,
    crates: &BTreeSet<String>,
    out: &mut Vec<(usize, Vec<String>)>,
) {
    let tokens: Vec<TokenTree> = stream.into_iter().collect();
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            TokenTree::Group(g) => {
                scan_stream(g.stream(), crates, out);
                i += 1;
            }
            TokenTree::Ident(id) if crates.contains(&id.to_string()) => {
                // Skip `something::kiln_x` (a nested module of the same name), but not `::kiln_x`.
                let nested = i >= 2
                    && is_path_sep(&tokens, i - 2)
                    && matches!(tokens.get(i.wrapping_sub(3)), Some(TokenTree::Ident(_)));
                if nested {
                    i += 1;
                    continue;
                }
                let line = id.span().start().line;
                i = parse_tail(&tokens, i + 1, vec![id.to_string()], out, line);
            }
            _ => i += 1,
        }
    }
}

/// Every path in `source` that starts with one of `crates` (crate identifiers).
pub fn scan_source(source: &str, crates: &BTreeSet<String>) -> Result<Vec<Found>> {
    let stream = TokenStream::from_str(source).map_err(|e| anyhow!("tokenize: {e}"))?;
    let mut raw = Vec::new();
    scan_stream(stream, crates, &mut raw);
    let mut found: Vec<Found> = raw
        .into_iter()
        .map(|(line, p)| Found {
            line,
            path: p.join("::"),
        })
        .collect();
    found.sort();
    found.dedup();
    Ok(found)
}

/// Whether `path` is admitted by an allowlist entry (equal, or extending it by whole segments).
pub fn allowed(path: &str, allow: &[String]) -> bool {
    allow.iter().any(|a| {
        path == a
            || path
                .strip_prefix(a.as_str())
                .is_some_and(|r| r.starts_with("::"))
    })
}

/// Disallowed paths in `source`.
pub fn violations(source: &str, crates: &BTreeSet<String>, allow: &[String]) -> Result<Vec<Found>> {
    Ok(scan_source(source, crates)?
        .into_iter()
        .filter(|f| !allowed(&f.path, allow))
        .collect())
}

fn lower_crate_idents(table: &Table) -> BTreeSet<String> {
    table
        .crates
        .iter()
        .filter(|c| {
            c.name != "kiln-ui"
                && (c.layer.is_some_and(|l| l < 5) || c.role.as_deref() == Some("testkit"))
        })
        .map(|c| c.name.replace('-', "_"))
        .collect()
}

/// Run the self-test on the fixtures, then check `crates/kiln-ui`.
pub fn check(root: &Path) -> Result<StepResult> {
    let table: Table = read_toml(&root.join("xtask/layers.toml"))?;
    let crates = lower_crate_idents(&table);
    let file: AllowFile = read_toml(&root.join("xtask/ui-allowlist.toml"))?;
    for a in &file.allow {
        if a.reason.trim().is_empty() {
            bail!("ui-allowlist entry `{}` has no reason", a.path);
        }
    }
    let allow: Vec<String> = file.allow.iter().map(|a| a.path.clone()).collect();

    // Self-test: the checker must catch the known violations and accept the clean fixture.
    let fixtures = root.join("xtask/fixtures/ui-commands");
    let bad =
        std::fs::read_to_string(fixtures.join("violation.rs")).context("violation fixture")?;
    let caught: BTreeSet<String> = violations(&bad, &crates, &allow)?
        .into_iter()
        .map(|f| f.path)
        .collect();
    let expected: BTreeSet<String> = [
        "kiln_text::Buffer",
        "kiln_text::EditError",
        "kiln_commands::*",
        "kiln_workspace::open",
        "kiln_search::find",
        "kiln_control",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    if caught != expected {
        bail!("self-test failed: violation fixture reported {caught:?}, expected {expected:?}");
    }
    let clean = std::fs::read_to_string(fixtures.join("clean.rs")).context("clean fixture")?;
    let noise = violations(&clean, &crates, &allow)?;
    if !noise.is_empty() {
        bail!("self-test failed: clean fixture reported {noise:?}");
    }

    let ui = root.join("crates/kiln-ui");
    let files = rust_files(root, &ui, &[])?;
    if !files.iter().any(|f| f.ends_with("src/lib.rs")) {
        bail!("crates/kiln-ui/src/lib.rs not found; nothing to scan");
    }
    let mut problems = Vec::new();
    for f in &files {
        let src = std::fs::read_to_string(f)?;
        for v in violations(&src, &crates, &allow).with_context(|| rel(root, f))? {
            problems.push(format!(
                "{}:{}: `{}` is not in xtask/ui-allowlist.toml (kiln-ui may only dispatch commands and read state)",
                rel(root, f),
                v.line,
                v.path
            ));
        }
    }
    Ok(StepResult::from_problems(
        format!(
            "self-test ok; {} kiln-ui file(s) scanned, {} allowlisted path(s)",
            files.len(),
            allow.len()
        ),
        "disallowed paths in kiln-ui",
        &problems,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crates() -> BTreeSet<String> {
        [
            "kiln_text",
            "kiln_commands",
            "kiln_workspace",
            "kiln_search",
            "kiln_control",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    fn paths(src: &str) -> Vec<String> {
        scan_source(src, &crates())
            .unwrap()
            .into_iter()
            .map(|f| f.path)
            .collect()
    }

    #[test]
    fn expands_use_trees_and_ignores_comments_and_strings() {
        let p = paths(
            "use kiln_text::{Buffer, edit::{Op, self}}; // kiln_text::Hidden\nfn f() { let _ = \"kiln_text::Str\"; }",
        );
        assert_eq!(
            p,
            [
                "kiln_text::Buffer",
                "kiln_text::edit",
                "kiln_text::edit::Op"
            ]
        );
    }

    #[test]
    fn finds_paths_inside_macros_and_absolute_paths() {
        let p = paths(
            "fn f() { println!(\"{}\", kiln_text::Buffer::new().len_chars()); ::kiln_search::find(); }",
        );
        assert_eq!(p, ["kiln_search::find", "kiln_text::Buffer::new"]);
    }

    #[test]
    fn bare_crate_alias_and_glob_are_reported() {
        let allow = vec!["kiln_commands::Registry".to_string()];
        let v: Vec<String> = violations(
            "use kiln_text as t; use kiln_commands::*; use kiln_commands::Registry;",
            &crates(),
            &allow,
        )
        .unwrap()
        .into_iter()
        .map(|f| f.path)
        .collect();
        assert_eq!(v, ["kiln_commands::*", "kiln_text"]);
    }

    #[test]
    fn allowlist_matches_whole_segments_only() {
        let allow = vec!["kiln_commands::Registry".to_string()];
        assert!(allowed("kiln_commands::Registry", &allow));
        assert!(allowed("kiln_commands::Registry::execute", &allow));
        assert!(!allowed("kiln_commands::RegistryMut", &allow));
        assert!(!allowed("kiln_commands", &allow));
    }

    #[test]
    fn nested_module_of_same_name_is_not_the_crate() {
        assert!(paths("fn f() { my::kiln_text::Buffer::new(); }").is_empty());
    }

    #[test]
    fn self_test_fixtures_behave() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let r = check(root).unwrap();
        assert_eq!(r.status, crate::util::Status::Pass, "{}", r.detail);
    }
}
