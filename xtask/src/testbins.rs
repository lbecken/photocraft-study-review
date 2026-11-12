//! Build, list and run the workspace's test executables directly, so a test can be addressed
//! by a stable name: `<crate_ident>::<path>` for unit tests of a library or of the binary named
//! like the package, `<crate_ident>::<target>::<path>` for integration tests and other binaries.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use serde_json::Value;

use crate::util::{capture, cargo};

/// One test executable.
#[derive(Clone, Debug)]
pub struct TestBin {
    /// Package name.
    pub package: String,
    /// Target name.
    pub target: String,
    /// Target kind (`lib`, `bin`, `test`, …).
    pub kind: String,
    /// Path of the executable.
    pub exe: PathBuf,
    /// Directory of the package manifest (the cwd cargo test would use).
    pub manifest_dir: PathBuf,
}

impl TestBin {
    /// The stable name of test `name` in this executable.
    pub fn qualify(&self, name: &str) -> String {
        let ident = self.package.replace('-', "_");
        let by_package = matches!(
            self.kind.as_str(),
            "lib" | "rlib" | "proc-macro" | "cdylib" | "staticlib"
        ) || (self.kind == "bin" && self.target == self.package);
        if by_package {
            format!("{ident}::{name}")
        } else {
            format!("{ident}::{}::{name}", self.target.replace('-', "_"))
        }
    }
}

/// The result of building test executables.
#[derive(Debug, Default)]
pub struct Build {
    /// Executables that were built.
    pub bins: Vec<TestBin>,
    /// `package:target` of targets that failed to compile.
    pub failed: Vec<String>,
    /// Cargo succeeded.
    pub ok: bool,
    /// Cargo's stderr.
    pub stderr: String,
}

/// Parse cargo's `--message-format=json` stream.
pub fn parse_messages(stdout: &str) -> (Vec<TestBin>, Vec<String>) {
    let mut bins = Vec::new();
    let mut failed = Vec::new();
    for line in stdout.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let target_name = v["target"]["name"].as_str().unwrap_or_default().to_owned();
        let kind = v["target"]["kind"][0]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        let package = v["package_id"]
            .as_str()
            .and_then(package_name_from_id)
            .unwrap_or_default();
        match v["reason"].as_str() {
            Some("compiler-artifact") if v["profile"]["test"] == Value::Bool(true) => {
                if let Some(exe) = v["executable"].as_str() {
                    let manifest_dir = v["manifest_path"]
                        .as_str()
                        .map(|m| {
                            Path::new(m)
                                .parent()
                                .unwrap_or(Path::new("."))
                                .to_path_buf()
                        })
                        .unwrap_or_default();
                    bins.push(TestBin {
                        package,
                        target: target_name,
                        kind,
                        exe: PathBuf::from(exe),
                        manifest_dir,
                    });
                }
            }
            Some("compiler-message") if v["message"]["level"] == "error" => {
                let t = format!("{package}:{target_name}");
                if !failed.contains(&t) {
                    failed.push(t);
                }
            }
            _ => {}
        }
    }
    (bins, failed)
}

/// Package name from a cargo package id (`path+file:///x/kiln-text#0.0.0`, or the older
/// `kiln-text 0.0.0 (path+file:///x)`).
fn package_name_from_id(id: &str) -> Option<String> {
    if let Some((name, _)) = id.split_once(' ') {
        return Some(name.to_owned());
    }
    let (url, frag) = id.rsplit_once('#')?;
    match frag.split_once('@') {
        Some((name, _)) => Some(name.to_owned()),
        None => url.rsplit('/').next().map(str::to_owned),
    }
}

/// Build every test executable of the workspace in `dir` (`--keep-going`, so targets that
/// compile are still produced when another fails), optionally into `target_dir`.
pub fn build(dir: &Path, target_dir: Option<&Path>) -> Result<Build> {
    let mut c = cargo(dir);
    if let Some(t) = target_dir {
        c.env("CARGO_TARGET_DIR", t);
    }
    // `cargo build --tests` is `cargo test --no-run` with `--keep-going` support.
    let out = capture(c.args([
        "build",
        "--workspace",
        "--tests",
        "--locked",
        "--keep-going",
        "--message-format=json-render-diagnostics",
    ]))?;
    let (bins, failed) = parse_messages(&out.stdout);
    Ok(Build {
        bins,
        failed,
        ok: out.ok,
        stderr: out.stderr,
    })
}

/// Test names (unqualified) in `bin`.
pub fn list(bin: &TestBin) -> Result<Vec<String>> {
    let out = capture(
        std::process::Command::new(&bin.exe)
            .args(["--list", "--format", "terse"])
            .current_dir(&bin.manifest_dir)
            .env("CARGO_MANIFEST_DIR", &bin.manifest_dir),
    )?;
    if !out.ok {
        bail!("{} --list failed: {}", bin.exe.display(), out.stderr.trim());
    }
    Ok(out
        .stdout
        .lines()
        .filter_map(|l| l.strip_suffix(": test"))
        .map(str::to_owned)
        .collect())
}

/// Outcome of one test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Passed.
    Passed,
    /// Failed.
    Failed,
    /// Reported as ignored.
    Ignored,
}

/// Parse libtest's pretty output (`test name ... ok`).
pub fn parse_results(stdout: &str) -> BTreeMap<String, Outcome> {
    stdout
        .lines()
        .filter_map(|l| l.strip_prefix("test "))
        .filter_map(|l| {
            let (name, status) = l.rsplit_once(" ... ")?;
            let o = match status.trim() {
                "ok" => Outcome::Passed,
                "FAILED" => Outcome::Failed,
                s if s.starts_with("ignored") => Outcome::Ignored,
                _ => return None,
            };
            Some((name.to_owned(), o))
        })
        .collect()
}

/// Run `names` (exact, unqualified) in `bin`, or every test when `names` is empty.
pub fn run(bin: &TestBin, names: &[String]) -> Result<BTreeMap<String, Outcome>> {
    let mut c = std::process::Command::new(&bin.exe);
    c.current_dir(&bin.manifest_dir)
        .env("CARGO_MANIFEST_DIR", &bin.manifest_dir);
    if !names.is_empty() {
        c.arg("--exact").args(names);
    }
    let out = capture(&mut c)?;
    Ok(parse_results(&out.stdout))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bin(package: &str, target: &str, kind: &str) -> TestBin {
        TestBin {
            package: package.into(),
            target: target.into(),
            kind: kind.into(),
            exe: PathBuf::new(),
            manifest_dir: PathBuf::new(),
        }
    }

    #[test]
    fn qualified_names() {
        assert_eq!(
            bin("kiln-text", "kiln_text", "lib").qualify("buffer::tests::a"),
            "kiln_text::buffer::tests::a"
        );
        assert_eq!(
            bin("kiln-cli", "kiln-cli", "bin").qualify("tests::a"),
            "kiln_cli::tests::a"
        );
        assert_eq!(
            bin("kiln-cli", "cli", "test").qualify("exec"),
            "kiln_cli::cli::exec"
        );
        assert_eq!(
            bin("kiln-text", "acceptance_t001", "test").qualify("x"),
            "kiln_text::acceptance_t001::x"
        );
    }

    #[test]
    fn package_ids() {
        assert_eq!(
            package_name_from_id("path+file:///w/crates/kiln-text#0.0.0").as_deref(),
            Some("kiln-text")
        );
        assert_eq!(
            package_name_from_id("path+file:///w/apps/kiln#kiln-cli@0.0.0").as_deref(),
            Some("kiln-cli")
        );
        assert_eq!(
            package_name_from_id("kiln-text 0.0.0 (path+file:///w)").as_deref(),
            Some("kiln-text")
        );
    }

    #[test]
    fn parses_libtest_output() {
        let r = parse_results(
            "running 3 tests\ntest a::b ... ok\ntest c ... FAILED\ntest d ... ignored, slow\n\ntest result: FAILED.",
        );
        assert_eq!(r["a::b"], Outcome::Passed);
        assert_eq!(r["c"], Outcome::Failed);
        assert_eq!(r["d"], Outcome::Ignored);
        assert_eq!(r.len(), 3);
    }

    #[test]
    fn parses_cargo_messages() {
        let msgs = r#"{"reason":"compiler-artifact","package_id":"path+file:///w/crates/kiln-text#0.0.0","manifest_path":"/w/crates/kiln-text/Cargo.toml","target":{"name":"kiln_text","kind":["lib"]},"profile":{"test":true},"executable":"/t/kiln_text-1"}
{"reason":"compiler-artifact","package_id":"path+file:///w/crates/kiln-text#0.0.0","target":{"name":"kiln_text","kind":["lib"]},"profile":{"test":false},"executable":null}
{"reason":"compiler-message","package_id":"path+file:///w/crates/kiln-text#0.0.0","target":{"name":"acceptance_t001","kind":["test"]},"message":{"level":"error"}}"#;
        let (bins, failed) = parse_messages(msgs);
        assert_eq!(bins.len(), 1);
        assert_eq!(bins[0].manifest_dir, PathBuf::from("/w/crates/kiln-text"));
        assert_eq!(failed, ["kiln-text:acceptance_t001"]);
    }
}
