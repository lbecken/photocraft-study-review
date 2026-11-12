//! `fuzz` and `budgets` gate steps. Their registries exist now (`xtask/fuzz.toml`,
//! `xtask/budgets.toml`) but no runner is implemented: an empty registry is reported as
//! EMPTY ("no targets registered yet"), and any registered target FAILS until the task that
//! adds the runner lands. Never a silent pass.

use std::path::Path;

use anyhow::Result;
use serde::Deserialize;

use crate::util::{StepResult, read_toml};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    #[serde(default)]
    target: Vec<toml::Table>,
}

/// Pure decision for a registry with `names` registered.
pub fn decide(step: &str, file: &str, names: &[String]) -> StepResult {
    if names.is_empty() {
        StepResult::empty(format!("no targets registered yet ({file})"))
    } else {
        StepResult::fail(format!(
            "{} target(s) registered in {file} ({}) but xtask has no {step} runner yet; registered targets must run",
            names.len(),
            names.join(", ")
        ))
    }
}

/// Check the registry `xtask/<step>.toml`.
pub fn check(root: &Path, step: &str) -> Result<StepResult> {
    let file = format!("xtask/{step}.toml");
    let reg: Registry = read_toml(&root.join(&file))?;
    let names: Vec<String> = reg
        .target
        .iter()
        .map(|t| {
            t.get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("<unnamed>")
                .to_owned()
        })
        .collect();
    Ok(decide(step, &file, &names))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::Status;

    #[test]
    fn empty_is_reported_and_registered_without_runner_fails() {
        assert_eq!(decide("fuzz", "f", &[]).status, Status::Empty);
        assert_eq!(decide("fuzz", "f", &["cmd".into()]).status, Status::Fail);
    }

    #[test]
    fn workspace_registries_parse() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        for step in ["fuzz", "budgets"] {
            check(root, step).unwrap();
        }
    }
}
