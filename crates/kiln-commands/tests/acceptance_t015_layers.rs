//! T-015 AC-5 acceptance test (the five new dependency edges), written from the task
//! specification alone by the test author. It reads the workspace's dependency graph through
//! `cargo metadata` (offline, no resolution) and fails on base by assertion: today neither
//! edge exists. Only the presence of each edge is checked, so later tasks that add their own
//! dependencies do not break it; `cargo xtask layers` (a gate step) checks the layer rule.
#![cfg(test)]

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

const TIMEOUT: Duration = Duration::from_secs(120);

fn metadata() -> Value {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut child = Command::new(cargo)
        .args([
            "metadata",
            "--no-deps",
            "--offline",
            "--format-version",
            "1",
        ])
        .arg("--manifest-path")
        .arg(workspace.join("Cargo.toml"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn cargo metadata");
    let stdout = child.stdout.take().expect("stdout");
    let reader = std::thread::spawn(move || {
        let mut stdout = stdout;
        let mut buf = Vec::new();
        let _ = std::io::Read::read_to_end(&mut stdout, &mut buf);
        buf
    });
    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait().expect("try_wait") {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("cargo metadata did not finish within {TIMEOUT:?}");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(status.success(), "cargo metadata failed: {status:?}");
    serde_json::from_slice(&reader.join().expect("reader")).expect("metadata JSON")
}

/// The normal (non-dev, non-build) dependencies of `package`: (name, path if a path dep).
fn normal_deps(meta: &Value, package: &str) -> Vec<(String, Option<String>)> {
    let pkg = meta["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .find(|p| p["name"] == package)
        .unwrap_or_else(|| panic!("package {package} in the workspace"));
    pkg["dependencies"]
        .as_array()
        .expect("dependencies")
        .iter()
        .filter(|d| d["kind"].is_null())
        .map(|d| {
            (
                d["name"].as_str().expect("name").to_owned(),
                d["path"].as_str().map(str::to_owned),
            )
        })
        .collect()
}

fn assert_edge(deps: &[(String, Option<String>)], from: &str, to: &str, path: Option<&str>) {
    let found = deps.iter().find(|(name, _)| name == to);
    let Some((_, dep_path)) = found else {
        panic!("{from} has no normal dependency on {to}: {deps:?}");
    };
    match path {
        Some(suffix) => {
            let p = dep_path
                .as_deref()
                .unwrap_or_else(|| panic!("{from} -> {to} is a path dependency"));
            assert!(
                Path::new(p).ends_with(suffix),
                "{from} -> {to} points at {p}, not at {suffix}"
            );
        }
        None => assert!(dep_path.is_none(), "{from} -> {to} comes from the registry"),
    }
}

/// AC-5: kiln-commands → kiln-syntax, kiln-search; kiln-workspace → kiln-search, serde,
/// serde_json (all normal dependencies; the internal ones are path dependencies on the crates
/// of layer L1).
#[test]
fn the_five_new_dependency_edges_exist() {
    let meta = metadata();
    let commands = normal_deps(&meta, "kiln-commands");
    assert_edge(
        &commands,
        "kiln-commands",
        "kiln-syntax",
        Some("crates/kiln-syntax"),
    );
    assert_edge(
        &commands,
        "kiln-commands",
        "kiln-search",
        Some("crates/kiln-search"),
    );
    let workspace = normal_deps(&meta, "kiln-workspace");
    assert_edge(
        &workspace,
        "kiln-workspace",
        "kiln-search",
        Some("crates/kiln-search"),
    );
    assert_edge(&workspace, "kiln-workspace", "serde", None);
    assert_edge(&workspace, "kiln-workspace", "serde_json", None);
    // The existing edges stay.
    assert_edge(
        &commands,
        "kiln-commands",
        "kiln-workspace",
        Some("crates/kiln-workspace"),
    );
    assert_edge(
        &workspace,
        "kiln-workspace",
        "kiln-text",
        Some("crates/kiln-text"),
    );
    // kiln-workspace is L2: it gains no edge to L3 or above.
    for upper in [
        "kiln-commands",
        "kiln-control",
        "kiln-ui",
        "kiln-cli",
        "kiln",
    ] {
        assert!(
            !workspace.iter().any(|(n, _)| n == upper),
            "kiln-workspace depends on {upper}"
        );
    }
}
