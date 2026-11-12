//! `cargo xtask layers`: the layer rules of CHARTER §3 over `cargo metadata`.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

use anyhow::{Context as _, Result, bail};
use serde::Deserialize;
use serde_json::Value;

use crate::util::{StepResult, capture, cargo, read_toml};

/// `xtask/layers.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Table {
    /// Highest layer the testkit may depend on.
    pub testkit_max_layer: u8,
    /// Package names that may not appear in a headless dependency closure.
    pub headless_forbidden: Vec<String>,
    /// One entry per workspace crate.
    #[serde(rename = "crate")]
    pub crates: Vec<Entry>,
}

/// One crate of the table.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// Package name.
    pub name: String,
    /// Layer number for layered crates.
    pub layer: Option<u8>,
    /// `testkit`, `app` or `tool` for unlayered crates.
    pub role: Option<String>,
    /// Apps only: must stay free of UI/GPU crates.
    #[serde(default)]
    pub headless: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Layer(u8),
    Testkit,
    App { headless: bool },
    Tool,
}

/// Dependency kind as cargo reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DepKind {
    /// `[dependencies]`.
    Normal,
    /// `[build-dependencies]`.
    Build,
    /// `[dev-dependencies]`.
    Dev,
}

impl DepKind {
    fn label(self) -> &'static str {
        match self {
            DepKind::Normal => "dependency",
            DepKind::Build => "build-dependency",
            DepKind::Dev => "dev-dependency",
        }
    }
}

/// The parts of `cargo metadata` the checker needs.
#[derive(Debug, Default)]
pub struct Graph {
    /// Workspace member package names.
    pub members: BTreeSet<String>,
    /// Declared dependencies between workspace members: (from, to, kind).
    pub internal: Vec<(String, String, DepKind)>,
    /// Resolved normal+build edges by package name, for every package in the graph.
    pub resolved: BTreeMap<String, BTreeSet<String>>,
}

fn kind_of(e: &Entry) -> Result<Kind> {
    match (e.layer, e.role.as_deref()) {
        (Some(l), None) => Ok(Kind::Layer(l)),
        (None, Some("testkit")) => Ok(Kind::Testkit),
        (None, Some("app")) => Ok(Kind::App {
            headless: e.headless,
        }),
        (None, Some("tool")) => Ok(Kind::Tool),
        _ => bail!(
            "layers.toml entry `{}` needs exactly one of `layer = N` or `role = testkit|app|tool`",
            e.name
        ),
    }
}

/// Parse `cargo metadata --format-version 1` JSON into a [`Graph`].
pub fn graph_from_metadata(meta: &Value) -> Result<Graph> {
    let packages = meta["packages"].as_array().context("metadata: packages")?;
    let name_of: BTreeMap<&str, &str> = packages
        .iter()
        .filter_map(|p| Some((p["id"].as_str()?, p["name"].as_str()?)))
        .collect();
    let member_ids: BTreeSet<&str> = meta["workspace_members"]
        .as_array()
        .context("metadata: workspace_members")?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let mut g = Graph::default();
    for id in &member_ids {
        g.members
            .insert(name_of.get(id).context("member id")?.to_string());
    }
    for p in packages {
        let Some(id) = p["id"].as_str() else { continue };
        if !member_ids.contains(id) {
            continue;
        }
        let from = p["name"].as_str().unwrap_or_default().to_owned();
        for d in p["dependencies"].as_array().into_iter().flatten() {
            let to = d["name"].as_str().unwrap_or_default();
            if !g.members.contains(to) || d["path"].is_null() {
                continue;
            }
            let kind = match d["kind"].as_str() {
                Some("dev") => DepKind::Dev,
                Some("build") => DepKind::Build,
                _ => DepKind::Normal,
            };
            g.internal.push((from.clone(), to.to_owned(), kind));
        }
    }
    for node in meta["resolve"]["nodes"]
        .as_array()
        .context("metadata: resolve")?
    {
        let from = name_of
            .get(node["id"].as_str().unwrap_or_default())
            .copied()
            .unwrap_or_default()
            .to_owned();
        let set = g.resolved.entry(from).or_default();
        for d in node["deps"].as_array().into_iter().flatten() {
            let non_dev = d["dep_kinds"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|k| k["kind"].as_str() != Some("dev"));
            if non_dev && let Some(n) = name_of.get(d["pkg"].as_str().unwrap_or_default()) {
                set.insert((*n).to_owned());
            }
        }
    }
    Ok(g)
}

/// Every layer-rule violation; empty means the graph satisfies the table.
pub fn check_graph(table: &Table, g: &Graph) -> Vec<String> {
    let mut problems = Vec::new();
    if g.members.is_empty() {
        problems.push("the workspace has no members".to_owned());
    }
    let mut kinds: BTreeMap<&str, Kind> = BTreeMap::new();
    for e in &table.crates {
        match kind_of(e) {
            Ok(k) => {
                if kinds.insert(&e.name, k).is_some() {
                    problems.push(format!("`{}` is listed twice in layers.toml", e.name));
                }
            }
            Err(err) => problems.push(err.to_string()),
        }
        if !g.members.contains(&e.name) {
            problems.push(format!(
                "layers.toml lists `{}`, which is not a workspace crate",
                e.name
            ));
        }
    }
    for m in &g.members {
        if !kinds.contains_key(m.as_str()) {
            problems.push(format!(
                "workspace crate `{m}` is missing from xtask/layers.toml"
            ));
        }
    }
    for (from, to, dk) in &g.internal {
        let (Some(&a), Some(&b)) = (kinds.get(from.as_str()), kinds.get(to.as_str())) else {
            continue;
        };
        let what = format!("`{from}` has a {} on `{to}`", dk.label());
        let verdict: Option<String> = match (a, b) {
            (Kind::Tool, _) => Some("the tool may not depend on workspace crates".to_owned()),
            (_, Kind::App { .. }) => Some("nothing may depend on an app".to_owned()),
            (_, Kind::Tool) => Some("nothing may depend on the tool".to_owned()),
            (Kind::Layer(x), Kind::Layer(y)) if y > x => {
                Some(format!("upward dependency (L{x} -> L{y})"))
            }
            (Kind::Layer(x), Kind::Layer(y)) if y == x => {
                Some(format!("same-layer dependency (L{x} -> L{y}) is forbidden"))
            }
            (Kind::Layer(_), Kind::Layer(_)) => None,
            (Kind::Layer(_) | Kind::App { .. }, Kind::Testkit) if *dk != DepKind::Dev => {
                Some("the testkit may only be a dev-dependency".to_owned())
            }
            (Kind::Layer(_) | Kind::App { .. }, Kind::Testkit) => None,
            (Kind::Testkit, Kind::Layer(y)) if y > table.testkit_max_layer => Some(format!(
                "the testkit may depend on layers <= L{} only",
                table.testkit_max_layer
            )),
            (Kind::Testkit, Kind::Layer(_)) | (Kind::App { .. }, Kind::Layer(_)) => None,
            (Kind::Testkit, Kind::Testkit) => Some("self-dependency".to_owned()),
        };
        if let Some(v) = verdict {
            problems.push(format!("{what}: {v}"));
        }
    }
    let forbidden: BTreeSet<&str> = table
        .headless_forbidden
        .iter()
        .map(String::as_str)
        .collect();
    for (name, kind) in &kinds {
        let headless = match kind {
            Kind::Layer(l) => *l <= 4,
            Kind::Testkit => true,
            Kind::App { headless } => *headless,
            Kind::Tool => false,
        };
        if !headless || !g.members.contains(*name) {
            continue;
        }
        // Breadth-first search with parent links to report the chain.
        let mut parent: BTreeMap<&str, &str> = BTreeMap::new();
        let mut queue = VecDeque::from([*name]);
        let mut seen = BTreeSet::from([*name]);
        while let Some(cur) = queue.pop_front() {
            for next in g.resolved.get(cur).into_iter().flatten() {
                let next = next.as_str();
                if !seen.insert(next) {
                    continue;
                }
                parent.insert(next, cur);
                if forbidden.contains(next) {
                    let mut chain = vec![next];
                    let mut at = next;
                    while let Some(p) = parent.get(at) {
                        chain.push(p);
                        at = p;
                    }
                    chain.reverse();
                    problems.push(format!(
                        "headless crate `{name}` pulls in UI/GPU crate `{next}`: {}",
                        chain.join(" -> ")
                    ));
                } else {
                    queue.push_back(next);
                }
            }
        }
    }
    problems
}

/// Run the check on the workspace at `root`.
pub fn check(root: &Path) -> Result<StepResult> {
    let table: Table = read_toml(&root.join("xtask/layers.toml"))?;
    let out = capture(cargo(root).args(["metadata", "--format-version", "1", "--locked"]))?;
    if !out.ok {
        bail!("cargo metadata failed: {}", out.stderr.trim());
    }
    let meta: Value = serde_json::from_str(&out.stdout).context("parse cargo metadata")?;
    let g = graph_from_metadata(&meta)?;
    let problems = check_graph(&table, &g);
    Ok(StepResult::from_problems(
        format!(
            "{} crates, {} internal edges, layer table holds",
            g.members.len(),
            g.internal.len()
        ),
        "layer violations",
        &problems,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Table {
        toml::from_str(
            r#"
            testkit_max_layer = 4
            headless_forbidden = ["kiln-ui", "winit"]
            [[crate]]
            name = "a0"
            layer = 0
            [[crate]]
            name = "b1"
            layer = 1
            [[crate]]
            name = "c1"
            layer = 1
            [[crate]]
            name = "d2"
            layer = 2
            [[crate]]
            name = "kiln-ui"
            layer = 5
            [[crate]]
            name = "tk"
            role = "testkit"
            [[crate]]
            name = "cli"
            role = "app"
            headless = true
            [[crate]]
            name = "desk"
            role = "app"
            [[crate]]
            name = "xtask"
            role = "tool"
            "#,
        )
        .unwrap()
    }

    fn graph(edges: &[(&str, &str, DepKind)], external: &[(&str, &str)]) -> Graph {
        let mut g = Graph::default();
        for m in [
            "a0", "b1", "c1", "d2", "kiln-ui", "tk", "cli", "desk", "xtask",
        ] {
            g.members.insert(m.into());
            g.resolved.entry(m.into()).or_default();
        }
        for (f, t, k) in edges {
            g.internal.push((f.to_string(), t.to_string(), *k));
            if *k != DepKind::Dev {
                g.resolved
                    .entry(f.to_string())
                    .or_default()
                    .insert(t.to_string());
            }
        }
        for (f, t) in external {
            g.resolved
                .entry(f.to_string())
                .or_default()
                .insert(t.to_string());
        }
        g
    }

    use DepKind::*;

    #[test]
    fn clean_graph_passes() {
        let g = graph(
            &[
                ("b1", "a0", Normal),
                ("d2", "b1", Normal),
                ("d2", "c1", Normal),
                ("kiln-ui", "d2", Normal),
                ("d2", "tk", Dev),
                ("tk", "a0", Normal),
                ("cli", "d2", Normal),
                ("desk", "kiln-ui", Normal),
            ],
            &[("kiln-ui", "winit"), ("a0", "serde")],
        );
        assert_eq!(check_graph(&table(), &g), Vec::<String>::new());
    }

    #[test]
    fn upward_dependency_fails_even_as_dev_dependency() {
        let p = check_graph(&table(), &graph(&[("a0", "b1", Dev)], &[]));
        assert_eq!(p.len(), 1, "{p:?}");
        assert!(p[0].contains("upward"), "{p:?}");
    }

    #[test]
    fn same_layer_dependency_fails() {
        let p = check_graph(&table(), &graph(&[("b1", "c1", Normal)], &[]));
        assert!(p[0].contains("same-layer"), "{p:?}");
    }

    #[test]
    fn unlisted_and_stale_crates_fail() {
        let mut g = graph(&[], &[]);
        g.members.insert("new-crate".into());
        g.members.remove("desk");
        let p = check_graph(&table(), &g);
        assert!(
            p.iter().any(|m| m.contains("`new-crate` is missing")),
            "{p:?}"
        );
        assert!(
            p.iter().any(|m| m.contains("`desk`, which is not")),
            "{p:?}"
        );
    }

    #[test]
    fn testkit_only_as_dev_dependency() {
        let p = check_graph(&table(), &graph(&[("d2", "tk", Normal)], &[]));
        assert!(p[0].contains("only be a dev-dependency"), "{p:?}");
    }

    #[test]
    fn tool_and_apps_rules() {
        let p = check_graph(
            &table(),
            &graph(&[("xtask", "a0", Normal), ("cli", "desk", Normal)], &[]),
        );
        assert_eq!(p.len(), 2, "{p:?}");
    }

    #[test]
    fn ui_crate_in_headless_closure_fails_with_chain() {
        let g = graph(
            &[("d2", "b1", Normal)],
            &[("b1", "fancy-widgets"), ("fancy-widgets", "winit")],
        );
        let p = check_graph(&table(), &g);
        // b1 and d2 both reach winit.
        assert_eq!(p.len(), 2, "{p:?}");
        assert!(
            p.iter()
                .any(|m| m.contains("`d2`") && m.contains("d2 -> b1 -> fancy-widgets -> winit")),
            "{p:?}"
        );
    }

    #[test]
    fn headless_app_may_not_reach_the_ui() {
        let p = check_graph(&table(), &graph(&[("cli", "kiln-ui", Normal)], &[]));
        assert!(p.iter().any(|m| m.contains("`cli` pulls in")), "{p:?}");
    }

    #[test]
    fn parses_cargo_metadata_shape() {
        let meta = serde_json::json!({
            "workspace_members": ["p0", "p1"],
            "packages": [
                {"id": "p0", "name": "a0", "dependencies": []},
                {"id": "p1", "name": "b1", "dependencies": [
                    {"name": "a0", "kind": null, "path": "/x/a0"},
                    {"name": "serde", "kind": null}
                ]},
                {"id": "s", "name": "serde", "dependencies": []}
            ],
            "resolve": {"nodes": [
                {"id": "p1", "deps": [
                    {"pkg": "p0", "dep_kinds": [{"kind": null}]},
                    {"pkg": "s", "dep_kinds": [{"kind": "dev"}]}
                ]}
            ]}
        });
        let g = graph_from_metadata(&meta).unwrap();
        assert_eq!(g.internal, vec![("b1".into(), "a0".into(), Normal)]);
        assert_eq!(g.resolved["b1"], BTreeSet::from(["a0".to_string()]));
    }
}
