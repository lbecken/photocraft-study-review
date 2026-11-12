//! Seeding a new target directory (T-005): copy only the build artifacts of packages that are
//! **not** workspace members, so the first build reuses compiled dependencies while every
//! workspace crate is rebuilt from the new checkout's own source.
//!
//! - Copies are real copies (never hard links, symlinks or a shared directory): a build in one
//!   checkout can never write into another checkout's target.
//! - Only `<profile>/{.fingerprint,build,deps}/<unit>-<16 hex>…` entries are considered, and an
//!   entry is copied only if its unit name is positively identified as a non-member. Nothing else
//!   (uplifted binaries, `incremental/`, `examples/`, unknown files) is copied.
//! - File mtimes are preserved. For a non-member package whose sources live inside the source
//!   checkout (an excluded path dependency), its source files in the destination checkout get the
//!   source checkout's mtimes **only if their content is byte-identical**; cargo then decides
//!   freshness in the new checkout exactly as it would in the source checkout. A file that
//!   differs keeps its new mtime, so cargo rebuilds that package.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};

use crate::util::capture;

/// What a seed copied.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    /// Files copied.
    pub files: u64,
    /// Bytes copied.
    pub bytes: u64,
}

fn norm(name: &str) -> String {
    name.replace('-', "_")
}

/// Package and target names of the workspace members of the workspace at `root`, normalised
/// (`-` → `_`), from `cargo metadata --no-deps`.
pub fn member_names(root: &Path) -> Result<BTreeSet<String>> {
    let out = capture(
        std::process::Command::new("cargo")
            .args([
                "metadata",
                "--no-deps",
                "--format-version",
                "1",
                "--offline",
            ])
            .current_dir(root)
            .env_remove("RUSTUP_TOOLCHAIN"),
    )?;
    if !out.ok {
        bail!(
            "cargo metadata failed in {}: {}",
            root.display(),
            out.stderr.trim()
        );
    }
    let v: serde_json::Value = serde_json::from_str(&out.stdout).context("parse cargo metadata")?;
    let mut names = BTreeSet::new();
    for p in v["packages"].as_array().into_iter().flatten() {
        if let Some(n) = p["name"].as_str() {
            names.insert(norm(n));
        }
        for t in p["targets"].as_array().into_iter().flatten() {
            if let Some(n) = t["name"].as_str() {
                names.insert(norm(n));
            }
        }
    }
    if names.is_empty() {
        bail!(
            "cargo metadata in {} lists no workspace member",
            root.display()
        );
    }
    Ok(names)
}

/// The unit name of a `.fingerprint`, `build` or `deps` entry: `<name>-<16 hex>[.<ext>]`, with
/// `lib` removed from library files in `deps`. `None` if the name has no such shape.
pub fn unit_name(entry: &str, in_deps: bool) -> Option<String> {
    let (stem, ext) = match entry.split_once('.') {
        Some((s, e)) => (s, Some(e)),
        None => (entry, None),
    };
    let (name, hash) = stem.rsplit_once('-')?;
    if hash.len() != 16 || !hash.chars().all(|c| c.is_ascii_hexdigit()) || name.is_empty() {
        return None;
    }
    let is_lib = matches!(ext, Some("rlib" | "rmeta" | "so" | "a" | "dylib"));
    let name = if in_deps && is_lib {
        name.strip_prefix("lib")?
    } else {
        name
    };
    (!name.is_empty()).then(|| norm(name))
}

fn copy_tree(src: &Path, dst: &Path, stats: &mut Stats) -> Result<()> {
    let meta = std::fs::symlink_metadata(src)?;
    if meta.file_type().is_symlink() {
        return Ok(());
    }
    if meta.is_dir() {
        std::fs::create_dir_all(dst)?;
        for e in std::fs::read_dir(src)? {
            let e = e?;
            copy_tree(&e.path(), &dst.join(e.file_name()), stats)?;
        }
        return Ok(());
    }
    let copy = || -> std::io::Result<u64> {
        let mut from = std::fs::File::open(src)?;
        let mut to = std::fs::File::create(dst)?;
        let n = std::io::copy(&mut from, &mut to)?;
        to.set_times(
            std::fs::FileTimes::new()
                .set_modified(meta.modified()?)
                .set_accessed(meta.accessed()?),
        )?;
        drop(to);
        std::fs::set_permissions(dst, meta.permissions())?;
        Ok(n)
    };
    stats.bytes +=
        copy().with_context(|| format!("copy {} to {}", src.display(), dst.display()))?;
    stats.files += 1;
    Ok(())
}

/// Profile directories (those holding `.fingerprint`) up to two levels below `target`.
fn profile_dirs(target: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut level = vec![target.to_path_buf()];
    for _ in 0..2 {
        let mut next = Vec::new();
        for d in level {
            let Ok(rd) = std::fs::read_dir(&d) else {
                continue;
            };
            for e in rd {
                let p = e?.path();
                if !p.is_dir() || p.file_name().is_some_and(|n| n == "incremental") {
                    continue;
                }
                if p.join(".fingerprint").is_dir() {
                    out.push(p);
                } else {
                    next.push(p);
                }
            }
        }
        level = next;
    }
    out.sort();
    Ok(out)
}

/// Dependency paths listed in a rustc makefile-style dep-info file.
pub fn dep_info_paths(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        if line.starts_with('#') {
            continue;
        }
        // `path:` lines (one per dependency, no prerequisites).
        if let Some(p) = line.strip_suffix(':')
            && !p.is_empty()
        {
            out.push(p.replace("\\ ", " "));
        }
    }
    out
}

/// Give destination source files the source checkout's mtimes where the content is identical.
fn align_sources(
    src_root: &Path,
    src_target: &Path,
    dst_root: &Path,
    dep_infos: &[PathBuf],
) -> Result<()> {
    let mut rels = BTreeSet::new();
    for d in dep_infos {
        let Ok(text) = std::fs::read_to_string(d) else {
            continue;
        };
        for p in dep_info_paths(&text) {
            let abs = src_root.join(&p); // a relative path is relative to the workspace root
            if abs.starts_with(src_target) || abs.starts_with(src_root.join(".worktrees")) {
                continue;
            }
            if let Ok(rel) = abs.strip_prefix(src_root)
                && !rel.as_os_str().is_empty()
            {
                rels.insert(rel.to_path_buf());
            }
        }
    }
    for rel in rels {
        let (s, d) = (src_root.join(&rel), dst_root.join(&rel));
        let (Ok(sm), Ok(dm)) = (std::fs::metadata(&s), std::fs::metadata(&d)) else {
            continue;
        };
        if !sm.is_file() || !dm.is_file() || sm.len() != dm.len() {
            continue;
        }
        if std::fs::read(&s)? == std::fs::read(&d)? {
            std::fs::File::options()
                .write(true)
                .open(&d)?
                .set_modified(sm.modified()?)?;
        }
    }
    Ok(())
}

/// Seed `dst_target` (for the checkout `dst_root`) from `src_target` (built from `src_root`)
/// with the non-member artifacts named above. `members` are normalised member names.
pub fn seed_with(
    src_root: &Path,
    src_target: &Path,
    dst_root: &Path,
    dst_target: &Path,
    members: &BTreeSet<String>,
) -> Result<Stats> {
    let mut stats = Stats::default();
    let mut dep_infos = Vec::new();
    std::fs::create_dir_all(dst_target)?;
    for profile in profile_dirs(src_target)? {
        let rel = profile.strip_prefix(src_target)?;
        for sub in [".fingerprint", "build", "deps"] {
            let dir = profile.join(sub);
            let Ok(rd) = std::fs::read_dir(&dir) else {
                continue;
            };
            let out_dir = dst_target.join(rel).join(sub);
            std::fs::create_dir_all(&out_dir)?;
            for e in rd {
                let e = e?;
                let name = e.file_name().to_string_lossy().into_owned();
                let Some(unit) = unit_name(&name, sub == "deps") else {
                    continue;
                };
                if members.contains(&unit) {
                    continue;
                }
                let to = out_dir.join(&name);
                copy_tree(&e.path(), &to, &mut stats)?;
                if sub == "deps" && name.ends_with(".d") {
                    dep_infos.push(e.path());
                }
            }
        }
    }
    align_sources(src_root, src_target, dst_root, &dep_infos)?;
    Ok(stats)
}

/// Seed `dst_target` from `src_target`; members are those of both checkouts.
pub fn seed(
    src_root: &Path,
    src_target: &Path,
    dst_root: &Path,
    dst_target: &Path,
) -> Result<Stats> {
    if profile_dirs(src_target)?.is_empty() {
        // Nothing was ever built there: nothing to classify or copy.
        std::fs::create_dir_all(dst_target)?;
        return Ok(Stats::default());
    }
    let mut members = member_names(src_root)?;
    members.extend(member_names(dst_root)?);
    seed_with(src_root, src_target, dst_root, dst_target, &members)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::testutil::TempDir;

    const H: &str = "0123456789abcdef";

    #[test]
    fn unit_names() {
        assert_eq!(
            unit_name(&format!("serde-{H}"), false).as_deref(),
            Some("serde")
        );
        assert_eq!(
            unit_name(&format!("kiln-text-{H}"), false).as_deref(),
            Some("kiln_text")
        );
        assert_eq!(
            unit_name(&format!("libserde-{H}.rlib"), true).as_deref(),
            Some("serde")
        );
        assert_eq!(
            unit_name(&format!("liblibc-{H}.rmeta"), true).as_deref(),
            Some("libc")
        );
        assert_eq!(
            unit_name(&format!("libc-{H}.d"), true).as_deref(),
            Some("libc")
        );
        assert_eq!(
            unit_name(&format!("xtask-{H}"), true).as_deref(),
            Some("xtask")
        );
        assert_eq!(unit_name("xtask", true), None);
        assert_eq!(unit_name("serde-123", false), None);
        assert_eq!(unit_name(&format!("-{H}"), false), None);
    }

    #[test]
    fn members_of_this_workspace() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let m = member_names(root).unwrap();
        for n in ["xtask", "kiln_text", "kiln_cli", "kiln_commands"] {
            assert!(m.contains(n), "{n}: {m:?}");
        }
        assert!(!m.contains("serde"));
    }

    #[test]
    fn dep_info_parsing() {
        let t = "/t/deps/a-1.d: a/src/lib.rs a/src/my\\ mod.rs\n\na/src/lib.rs:\na/src/my\\ mod.rs:\n# env-dep:X\n";
        assert_eq!(dep_info_paths(t), ["a/src/lib.rs", "a/src/my mod.rs"]);
    }

    fn write(p: &Path, s: &str) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, s).unwrap();
    }

    #[test]
    fn copies_only_non_member_units_and_aligns_identical_sources() {
        let d = TempDir::new("seed");
        let (src, dst) = (d.path().join("src"), d.path().join("dst"));
        let st = src.join("target");
        for f in [
            format!("debug/.fingerprint/helper-{H}/lib-helper"),
            format!("debug/.fingerprint/app-{H}/bin-app"),
            format!("debug/deps/libhelper-{H}.rlib"),
            format!("debug/deps/app-{H}"),
            format!("debug/deps/app-{H}.d"),
            format!("debug/build/cc-{H}/output"),
            format!("debug/incremental/helper-{H}/x"),
            "debug/app".into(),
            "debug/libhelper.rlib".into(),
            "debug/deps/notes.txt".into(),
            format!("x86_64-unknown-linux-gnu/debug/deps/libhelper-{H}.rlib"),
        ] {
            write(&st.join(&f), "bin");
        }
        std::fs::create_dir_all(st.join("x86_64-unknown-linux-gnu/debug/.fingerprint")).unwrap();
        write(
            &st.join(format!("debug/deps/helper-{H}.d")),
            "x: helper/src/lib.rs\n\nhelper/src/lib.rs:\nhelper/src/other.rs:\n",
        );
        write(&src.join("helper/src/lib.rs"), "same");
        write(&src.join("helper/src/other.rs"), "old");
        std::thread::sleep(std::time::Duration::from_millis(20));
        write(&dst.join("helper/src/lib.rs"), "same");
        write(&dst.join("helper/src/other.rs"), "new");
        let members: BTreeSet<String> = ["app".to_owned()].into();
        let dt = dst.join("target");
        let stats = seed_with(&src, &st, &dst, &dt, &members).unwrap();
        let mut got = Vec::new();
        let mut stack = vec![dt.clone()];
        while let Some(p) = stack.pop() {
            for e in std::fs::read_dir(&p).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    stack.push(p);
                } else {
                    got.push(crate::util::rel(&dt, &p));
                }
            }
        }
        got.sort();
        assert_eq!(
            got,
            [
                format!("debug/.fingerprint/helper-{H}/lib-helper"),
                format!("debug/build/cc-{H}/output"),
                format!("debug/deps/helper-{H}.d"),
                format!("debug/deps/libhelper-{H}.rlib"),
                format!("x86_64-unknown-linux-gnu/debug/deps/libhelper-{H}.rlib"),
            ]
        );
        assert_eq!(stats.files, 5);
        // Copies, not links.
        std::fs::write(dt.join(format!("debug/deps/libhelper-{H}.rlib")), "changed").unwrap();
        assert_eq!(
            std::fs::read_to_string(st.join(format!("debug/deps/libhelper-{H}.rlib"))).unwrap(),
            "bin"
        );
        let mtime = |p: &Path| std::fs::metadata(p).unwrap().modified().unwrap();
        assert_eq!(
            mtime(&dst.join("helper/src/lib.rs")),
            mtime(&src.join("helper/src/lib.rs"))
        );
        assert_ne!(
            mtime(&dst.join("helper/src/other.rs")),
            mtime(&src.join("helper/src/other.rs"))
        );
        assert_eq!(
            mtime(&dt.join(format!("debug/build/cc-{H}/output"))),
            mtime(&st.join(format!("debug/build/cc-{H}/output")))
        );
    }
}
