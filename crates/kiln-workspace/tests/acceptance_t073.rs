//! T-073 ruling Q-4 A (D-245), tested per ruling y1 (D-247): `kiln-workspace`'s manifest
//! declares `same-file` as a Windows-only dependency, `[target.'cfg(windows)'.dependencies]`,
//! with the version requirement `=1.0.6`, and nowhere else. Written by the test author from
//! `tasks/T-073-windows-build-and-root-identity.md` alone. Fails on base on every platform (no
//! such dependency); the runtime tests of Interface (4) are in `guard_t073.rs`.
//!
//! The manifest is read as text (no new dev-dependency: the lock may gain one line only) by a
//! small line reader that knows the TOML forms a dependency takes in a manifest:
//! - a key `same-file = "<req>"` or `same-file = { version = "<req>", … }` in a dependency
//!   table (the key bare or quoted);
//! - a dotted key `same-file.version = "<req>"` in a dependency table;
//! - a table `[<dependency table>.same-file]` with `version = "<req>"` in it.
//!
//! A dependency table is `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]` (also
//! `dev_dependencies`, `build_dependencies`) or one of these under `[target.<spec>]`. A line
//! that names `same-file` in any other way (a renamed dependency's `package = "same-file"`, a
//! table such as `[target.'cfg(windows)']` with a dotted `dependencies.same-file` key, a
//! multi-line form) is reported as not understood and fails the test, so an unknown form can
//! never pass. The Windows table is accepted as `cfg(windows)` in single or double quotes, with
//! any spaces in it and around the dots. The requirement must be exactly the string `=1.0.6`
//! (`= 1.0.6`, which Cargo reads the same, is not accepted: the ruling names `=1.0.6`).
#![cfg(test)]

use std::fs;
use std::path::Path;

const DEP: &str = "same-file";
const REQ: &str = "=1.0.6";

/// One declaration of `DEP` found in the manifest.
#[derive(Debug, PartialEq, Eq)]
struct Declaration {
    /// 1-based line of the key or table.
    line: usize,
    /// The dependency table, as its segments with quotes removed (e.g.
    /// `["target", "cfg(windows)", "dependencies"]`).
    table: Vec<String>,
    /// The version requirement, if one was found.
    version: Option<String>,
}

/// `s` with a `#` comment outside quotes removed, trimmed.
fn strip_comment(s: &str) -> &str {
    let mut quote: Option<char> = None;
    for (i, c) in s.char_indices() {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(q), _) if c == q => quote = None,
            (None, '#') => return s[..i].trim(),
            _ => {}
        }
    }
    s.trim()
}

/// Split a dotted TOML key (or table name) into segments, quotes removed, outer spaces trimmed.
fn segments(key: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    for c in key.chars() {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(q), _) if c == q => quote = None,
            (None, '.') => out.push(std::mem::take(&mut cur).trim().to_owned()),
            _ => cur.push(c),
        }
    }
    out.push(cur.trim().to_owned());
    out
}

/// Whether `table` (segments) is a dependency table.
fn is_dependency_table(table: &[String]) -> bool {
    let kind = |s: &str| {
        matches!(
            s,
            "dependencies"
                | "dev-dependencies"
                | "dev_dependencies"
                | "build-dependencies"
                | "build_dependencies"
        )
    };
    match table {
        [k] => kind(k),
        [t, _, k] => t == "target" && kind(k),
        _ => false,
    }
}

/// Whether `table` is exactly the Windows-only normal dependency table.
fn is_windows_table(table: &[String]) -> bool {
    match table {
        [t, spec, k] => {
            let spec: String = spec.chars().filter(|c| !c.is_whitespace()).collect();
            t == "target" && spec == "cfg(windows)" && k == "dependencies"
        }
        _ => false,
    }
}

/// The value of a TOML basic or literal string `v` (whole value), if it is one.
fn string_value(v: &str) -> Option<String> {
    let v = v.trim();
    let q = v.chars().next()?;
    if (q == '"' || q == '\'') && v.len() >= 2 && v.ends_with(q) {
        let inner = &v[1..v.len() - 1];
        (!inner.contains(q)).then(|| inner.to_owned())
    } else {
        None
    }
}

/// The `version` of an inline table `{ … }` (one line, no nested tables), if any.
fn inline_version(v: &str) -> Result<Option<String>, String> {
    let v = v.trim();
    let inner = v
        .strip_prefix('{')
        .and_then(|r| r.strip_suffix('}'))
        .ok_or_else(|| format!("not a string or a one-line inline table: {v}"))?;
    if inner.contains('{') || inner.contains('[') {
        return Err(format!("nested value in the inline table: {v}"));
    }
    let mut version = None;
    for part in inner.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let (k, val) = part
            .split_once('=')
            .ok_or_else(|| format!("inline table entry without `=`: {part}"))?;
        if segments(k) == ["version"] {
            version =
                Some(string_value(val).ok_or_else(|| format!("version is not a string: {part}"))?);
        }
    }
    Ok(version)
}

/// Every declaration of `DEP` in `manifest`, or the lines naming it in a form not understood.
fn declarations(manifest: &str) -> Result<Vec<Declaration>, Vec<String>> {
    let mut found: Vec<Declaration> = Vec::new();
    let mut problems = Vec::new();
    let mut table: Vec<String> = Vec::new();
    // Index in `found` of the declaration whose own table (`[… .same-file]`) is open.
    let mut own_table: Option<usize> = None;
    for (i, raw) in manifest.lines().enumerate() {
        let line_no = i + 1;
        let line = strip_comment(raw);
        if line.is_empty() {
            continue;
        }
        let names_dep = line.contains(DEP);
        if let Some(header) = line.strip_prefix('[') {
            let Some(name) = header.strip_suffix(']') else {
                if names_dep {
                    problems.push(format!("line {line_no}: not understood: {raw}"));
                }
                continue;
            };
            let name = name
                .strip_prefix('[')
                .and_then(|n| n.strip_suffix(']'))
                .unwrap_or(name);
            let segs = segments(name);
            own_table = None;
            if segs.last().map(String::as_str) == Some(DEP)
                && is_dependency_table(&segs[..segs.len() - 1])
            {
                found.push(Declaration {
                    line: line_no,
                    table: segs[..segs.len() - 1].to_vec(),
                    version: None,
                });
                own_table = Some(found.len() - 1);
                table = segs;
            } else {
                if names_dep {
                    problems.push(format!("line {line_no}: table not understood: {raw}"));
                }
                table = segs;
            }
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            if names_dep {
                problems.push(format!("line {line_no}: not understood: {raw}"));
            }
            continue;
        };
        let key = segments(key);
        if let Some(idx) = own_table {
            // Inside `[… .same-file]`.
            if key == ["version"] {
                match string_value(value) {
                    Some(v) => found[idx].version = Some(v),
                    None => {
                        problems.push(format!("line {line_no}: version is not a string: {raw}"))
                    }
                }
            } else if names_dep {
                problems.push(format!("line {line_no}: not understood: {raw}"));
            }
            continue;
        }
        if !names_dep {
            continue;
        }
        let is_dep_table = is_dependency_table(&table);
        match key.as_slice() {
            [k] if k == DEP && is_dep_table => {
                let version = match string_value(value) {
                    Some(v) => Ok(Some(v)),
                    None => inline_version(value),
                };
                match version {
                    Ok(version) => found.push(Declaration {
                        line: line_no,
                        table: table.clone(),
                        version,
                    }),
                    Err(e) => problems.push(format!("line {line_no}: {e}: {raw}")),
                }
            }
            [k, v] if k == DEP && v == "version" && is_dep_table => match string_value(value) {
                Some(req) => found.push(Declaration {
                    line: line_no,
                    table: table.clone(),
                    version: Some(req),
                }),
                None => problems.push(format!("line {line_no}: version is not a string: {raw}")),
            },
            _ => problems.push(format!(
                "line {line_no}: names {DEP} in a form not understood: {raw}"
            )),
        }
    }
    if problems.is_empty() {
        Ok(found)
    } else {
        Err(problems)
    }
}

/// Ruling Q-4 A: exactly one declaration of `same-file` in `kiln-workspace`'s manifest, under
/// `[target.'cfg(windows)'.dependencies]`, with the requirement `=1.0.6`. Rejects: the
/// dependency missing (base); in `[dependencies]`; in both places; under `cfg(unix)` or
/// another target table; in `[dev-dependencies]`; with `1.0.6`, `1` or no version.
#[test]
fn acq4_same_file_is_declared_only_as_windows_dependency_at_exact_version() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: kind {:?} ({e})", path.display(), e.kind()));
    let found = declarations(&text).unwrap_or_else(|p| {
        panic!(
            "crates/kiln-workspace/Cargo.toml names {DEP} in a form this test does not read; \
             expected only `{DEP} = \"{REQ}\"` (or an equivalent form, module docs) under \
             [target.'cfg(windows)'.dependencies]:\n{}",
            p.join("\n")
        )
    });
    let expected = vec![Declaration {
        line: found.first().map_or(0, |d| d.line),
        table: vec![
            "target".to_owned(),
            "cfg(windows)".to_owned(),
            "dependencies".to_owned(),
        ],
        version: Some(REQ.to_owned()),
    }];
    let normalised: Vec<Declaration> = found
        .into_iter()
        .map(|d| Declaration {
            table: if is_windows_table(&d.table) {
                vec![
                    "target".to_owned(),
                    "cfg(windows)".to_owned(),
                    "dependencies".to_owned(),
                ]
            } else {
                d.table
            },
            ..d
        })
        .collect();
    assert_eq!(
        normalised, expected,
        "crates/kiln-workspace/Cargo.toml: expected exactly one declaration of {DEP}, under \
         [target.'cfg(windows)'.dependencies] with version \"{REQ}\" (ruling Q-4 A); found the \
         declarations on the left (an empty list: {DEP} is not declared)"
    );
}
