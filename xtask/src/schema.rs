//! `cargo xtask schema [--write]`: every registered command has a unique name, a doc string,
//! params and result schemas and at least one example that validates and runs (the registry's
//! own audit, via `kiln-cli commands --audit`); the dump is committed as `docs/commands.json`.
//!
//! xtask does not link the product crates (see `xtask/Cargo.toml`), so it asks the CLI.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::util::{StepResult, capture, cargo, tail};

fn cli(root: &Path, args: &[&str]) -> Result<crate::util::Captured> {
    let mut c = cargo(root);
    c.args(["run", "--quiet", "--locked", "-p", "kiln-cli", "--"])
        .args(args);
    capture(&mut c)
}

/// Structural problems of a `commands --schema` dump.
pub fn structural_problems(dump: &Value) -> Vec<String> {
    let mut problems = Vec::new();
    let Some(cmds) = dump["commands"].as_array() else {
        return vec!["dump has no `commands` array".to_owned()];
    };
    if cmds.is_empty() {
        problems.push("no commands are registered".to_owned());
    }
    let mut names = BTreeSet::new();
    for (i, c) in cmds.iter().enumerate() {
        let name = c["name"].as_str().unwrap_or_default();
        let label = if name.is_empty() {
            format!("command #{i}")
        } else {
            format!("`{name}`")
        };
        if name.is_empty() {
            problems.push(format!("{label}: no name"));
        } else if !names.insert(name) {
            problems.push(format!("{label}: duplicate name"));
        }
        if c["doc"].as_str().is_none_or(|d| d.trim().is_empty()) {
            problems.push(format!("{label}: no doc string"));
        }
        for key in ["params_schema", "result_schema"] {
            if !c[key].is_object() {
                problems.push(format!("{label}: no {key}"));
            }
        }
        match c["examples"].as_array() {
            Some(ex) if !ex.is_empty() => {
                for (j, e) in ex.iter().enumerate() {
                    if e.get("params").is_none() || e.get("result").is_none() {
                        problems.push(format!("{label}: example {j} lacks params or result"));
                    }
                }
            }
            _ => problems.push(format!("{label}: no example")),
        }
    }
    problems
}

/// Run the check, or rewrite `docs/commands.json` when `write`.
pub fn run(root: &Path, write: bool) -> Result<StepResult> {
    let audit = cli(root, &["commands", "--audit"])?;
    if !audit.ok {
        return Ok(StepResult::fail(format!(
            "kiln-cli commands --audit failed:\n{}",
            tail(&audit.stderr, 30)
        )));
    }
    let out = cli(root, &["commands", "--schema"])?;
    if !out.ok {
        bail!(
            "kiln-cli commands --schema failed: {}",
            tail(&out.stderr, 30)
        );
    }
    let dump: Value =
        serde_json::from_str(&out.stdout).context("parse commands --schema output")?;
    let mut problems = structural_problems(&dump);
    let text = format!("{}\n", serde_json::to_string_pretty(&dump)?);
    let path = root.join("docs/commands.json");
    if write {
        std::fs::write(&path, &text)?;
    } else if std::fs::read_to_string(&path).unwrap_or_default() != text {
        problems.push(
            "docs/commands.json is stale; run `cargo xtask schema --write` and commit it"
                .to_owned(),
        );
    }
    let n = dump["commands"].as_array().map_or(0, Vec::len);
    Ok(StepResult::from_problems(
        format!("{n} commands; audit ok; docs/commands.json current"),
        "schema problems",
        &problems,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn structural_checks() {
        assert_eq!(
            structural_problems(&json!({"commands": []})),
            ["no commands are registered"]
        );
        let good = json!({"name": "a.b", "doc": "d", "params_schema": {}, "result_schema": {}, "examples": [{"params": {}, "result": {}}]});
        assert!(structural_problems(&json!({"commands": [good.clone()]})).is_empty());
        let p = structural_problems(&json!({"commands": [good.clone(), good]}));
        assert_eq!(p, ["`a.b`: duplicate name"]);
        let bad = json!({"name": "x.y", "doc": " ", "params_schema": {}, "examples": []});
        assert_eq!(structural_problems(&json!({"commands": [bad]})).len(), 3);
    }
}
