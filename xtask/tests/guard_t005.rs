//! Regression guards for T-005 (decision D-010): target-seeding properties that the base already
//! has (it copies the whole `target/`) and that the new, selective seeding must keep — a seeded
//! worktree builds its own source, builds never write into another checkout's target, and the
//! revert check leaves the task worktree's target alone. Written from the task file alone; every
//! test runs the `xtask` binary against throwaway repositories under `CARGO_TARGET_TMPDIR`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

#[cfg(test)]
mod support {
    use super::*;

    pub const XTASK: &str = env!("CARGO_BIN_EXE_xtask");

    pub fn scratch(name: &str) -> PathBuf {
        let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("t005-guard")
            .join(name);
        if d.exists() {
            std::fs::remove_dir_all(&d).unwrap();
        }
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A director-shell environment: no inherited cargo/git variables, offline cargo.
    pub fn hygiene(c: &mut Command) -> &mut Command {
        for (k, _) in std::env::vars_os() {
            let Some(k) = k.to_str() else { continue };
            if (k.starts_with("CARGO_") && k != "CARGO_HOME") || k.starts_with("GIT_") {
                c.env_remove(k);
            }
        }
        c.env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Toy Director")
            .env("GIT_AUTHOR_EMAIL", "toy@example.invalid")
            .env("GIT_COMMITTER_NAME", "Toy Director")
            .env("GIT_COMMITTER_EMAIL", "toy@example.invalid")
            .env("CARGO_NET_OFFLINE", "true")
    }

    pub struct Out {
        pub code: i32,
        pub stdout: String,
        pub stderr: String,
    }

    pub fn run(c: &mut Command) -> Out {
        let o = hygiene(c).output().unwrap();
        Out {
            code: o.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&o.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
        }
    }

    pub fn git(dir: &Path, args: &[&str]) -> String {
        let o = run(Command::new("git").args(args).current_dir(dir));
        assert_eq!(o.code, 0, "git {args:?} in {}: {}", dir.display(), o.stderr);
        o.stdout.trim().to_owned()
    }

    pub fn xtask(dir: &Path, args: &[&str]) -> Out {
        run(Command::new(XTASK).args(args).current_dir(dir))
    }

    pub fn write(dir: &Path, rel: &str, content: &str) {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    }

    pub fn commit(dir: &Path, msg: &str, files: &[(&str, &str)]) -> String {
        for (p, c) in files {
            write(dir, p, c);
        }
        git(dir, &["add", "-A"]);
        git(dir, &["commit", "-q", "-m", msg]);
        git(dir, &["rev-parse", "HEAD"])
    }

    pub fn init(root: &Path) {
        let lock = run(Command::new("cargo")
            .args(["generate-lockfile", "--offline"])
            .current_dir(root));
        assert_eq!(lock.code, 0, "generate-lockfile: {}", lock.stderr);
        git(root, &["init", "-q", "-b", "main"]);
        git(root, &["add", "-A"]);
        git(root, &["commit", "-q", "-m", "toy base"]);
    }

    pub fn common_files(root: &Path, members: &str) {
        write(
            root,
            "Cargo.toml",
            &format!("[workspace]\nresolver = \"2\"\nmembers = [{members}]\n"),
        );
        write(root, ".gitignore", ".worktrees/\ntarget/\n");
        write(
            root,
            "rust-toolchain.toml",
            "[toolchain]\nchannel = \"1.97.0\"\nprofile = \"minimal\"\n",
        );
    }

    /// One member binary `seedmember` using a path package outside the repository.
    pub fn seed_repo(name: &str) -> PathBuf {
        let s = scratch(name);
        let dep = s.join("outsidedep");
        write(
            &dep,
            "Cargo.toml",
            "[package]\nname = \"outsidedep\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
        );
        write(
            &dep,
            "src/lib.rs",
            "/// Toy.\npub fn base() -> u32 {\n    42\n}\n",
        );
        let root = s.join("repo");
        common_files(&root, "\"seedmember\"");
        write(
            &root,
            "seedmember/Cargo.toml",
            &format!(
                "[package]\nname = \"seedmember\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\noutsidedep = {{ path = \"{}\" }}\n",
                dep.display()
            ),
        );
        write(&root, "seedmember/src/main.rs", &member_src("main-v1"));
        init(&root);
        root
    }

    pub fn member_src(tag: &str) -> String {
        format!("fn main() {{\n    println!(\"{tag} {{}}\", outsidedep::base());\n}}\n")
    }

    pub fn cargo(dir: &Path, target: &Path, args: &[&str]) -> Out {
        let o = run(Command::new("cargo")
            .args(args)
            .args(["--offline", "--locked"])
            .current_dir(dir)
            .env("CARGO_TARGET_DIR", target));
        assert_eq!(
            o.code,
            0,
            "cargo {args:?} in {}: {}",
            dir.display(),
            o.stderr
        );
        o
    }

    /// Build in `dir` into `target`, then run the member binary; returns its output line.
    pub fn build_and_run(dir: &Path, target: &Path) -> String {
        cargo(dir, target, &["build", "--message-format=json"]);
        let o = run(&mut Command::new(target.join("debug/seedmember")));
        assert_eq!(o.code, 0, "seedmember: {}", o.stderr);
        o.stdout.trim().to_owned()
    }

    /// relative path → (kind, size, content hash) for every entry under `dir`.
    pub fn manifest(dir: &Path) -> BTreeMap<String, (char, u64, u64)> {
        use std::hash::{Hash, Hasher};
        let mut out = BTreeMap::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).unwrap() {
                let p = e.unwrap().path();
                let m = std::fs::symlink_metadata(&p).unwrap();
                let rel = p.strip_prefix(dir).unwrap().to_string_lossy().into_owned();
                if m.file_type().is_symlink() {
                    out.insert(rel, ('l', 0, 0));
                } else if m.is_dir() {
                    out.insert(rel, ('d', 0, 0));
                    stack.push(p);
                } else {
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    std::fs::read(&p).unwrap().hash(&mut h);
                    out.insert(rel, ('f', m.len(), h.finish()));
                }
            }
        }
        out
    }

    /// (device, inode) of every regular file under `dir`, and the symlinks found there.
    pub fn inodes(dir: &Path) -> (BTreeMap<(u64, u64), String>, Vec<String>) {
        use std::os::unix::fs::MetadataExt;
        let mut files = BTreeMap::new();
        let mut links = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).unwrap() {
                let p = e.unwrap().path();
                let m = std::fs::symlink_metadata(&p).unwrap();
                if m.file_type().is_symlink() {
                    links.push(p.display().to_string());
                } else if m.is_dir() {
                    stack.push(p);
                } else {
                    files.insert((m.dev(), m.ino()), p.display().to_string());
                }
            }
        }
        (files, links)
    }

    /// Symlinks under `seed`, and files under `seed` that are the same inode as a file under
    /// `source` (hard links). Cargo's own uplift hard links inside one target are fine.
    pub fn shared_entries(source: &Path, seed: &Path) -> Vec<String> {
        let (src, _) = inodes(source);
        let (files, mut out) = inodes(seed);
        out.extend(
            files
                .iter()
                .filter(|(k, _)| src.contains_key(k))
                .map(|(_, p)| p.clone()),
        );
        out
    }

    pub fn new_worktree(root: &Path, args: &[&str]) {
        let mut a = vec!["worktree", "new"];
        a.extend_from_slice(args);
        let o = xtask(root, &a);
        assert_eq!(o.code, 0, "worktree new {args:?}: {}{}", o.stdout, o.stderr);
    }
}

use support::*;

// AC-10: guard. Base copies the whole target, which already yields the right behaviour here,
// because cargo hashes the member's path into its artifact names; the new selective seed must
// keep it.
#[test]
fn seeded_worktree_builds_its_own_source() {
    let root = seed_repo("ac10");
    let main_target = root.join("target");
    assert_eq!(build_and_run(&root, &main_target), "main-v1 42");
    new_worktree(&root, &["T-001"]);
    let wt = root.join(".worktrees/T-001");
    let wt_target = wt.join("target");
    commit(
        &wt,
        "task behaviour",
        &[("seedmember/src/main.rs", &member_src("task-v1"))],
    );
    assert_eq!(build_and_run(&wt, &wt_target), "task-v1 42");
    assert_eq!(build_and_run(&root, &main_target), "main-v1 42");
    commit(
        &root,
        "main behaviour",
        &[("seedmember/src/main.rs", &member_src("main-v2"))],
    );
    assert_eq!(build_and_run(&root, &main_target), "main-v2 42");
    assert_eq!(build_and_run(&wt, &wt_target), "task-v1 42");
}

// AC-11: guard. Base seeds with `cp -a --reflink=auto`, an independent copy; a seed made of hard
// links, symlinks or a shared directory would fail this.
#[test]
fn worktree_build_does_not_touch_main_target() {
    let root = seed_repo("ac11");
    let main_target = root.join("target");
    build_and_run(&root, &main_target);
    new_worktree(&root, &["T-001"]);
    let wt = root.join(".worktrees/T-001");
    let wt_target = wt.join("target");
    let link = std::fs::symlink_metadata(&wt_target).unwrap();
    assert!(
        link.is_dir() && !link.file_type().is_symlink(),
        "target must be a real directory"
    );
    assert_eq!(
        shared_entries(&main_target, &wt_target),
        Vec::<String>::new(),
        "seed must be a copy"
    );

    let main_before = manifest(&main_target);
    commit(
        &wt,
        "task behaviour",
        &[("seedmember/src/main.rs", &member_src("task-v1"))],
    );
    build_and_run(&wt, &wt_target);
    assert_eq!(
        manifest(&main_target),
        main_before,
        "worktree build changed main's target"
    );

    let wt_before = manifest(&wt_target);
    commit(
        &root,
        "main behaviour",
        &[("seedmember/src/main.rs", &member_src("main-v2"))],
    );
    build_and_run(&root, &main_target);
    assert_eq!(
        manifest(&wt_target),
        wt_before,
        "main build changed the worktree's target"
    );
}

// AC-13: guard. Base already gives the revert check a private copy of the task target.
#[test]
fn revert_check_leaves_task_target_untouched() {
    let s = scratch("ac13");
    let root = s.join("repo");
    common_files(&root, "\"crates/k\"");
    write(
        &root,
        "crates/k/Cargo.toml",
        "[package]\nname = \"k\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(
        &root,
        "crates/k/src/lib.rs",
        "/// Toy.\npub fn answer() -> u32 {\n    0\n}\n",
    );
    init(&root);
    new_worktree(&root, &["T-001", "--no-seed"]);
    new_worktree(&root, &["T-001", "--tests", "--no-seed"]);
    let wt = root.join(".worktrees/T-001");
    let twt = root.join(".worktrees/T-001-tests");
    commit(
        &twt,
        "acceptance test",
        &[(
            "crates/k/tests/acceptance_t001.rs",
            "#[test]\nfn answer_is_42() {\n    assert_eq!(k::answer(), 42);\n}\n",
        )],
    );
    commit(
        &wt,
        "implementation",
        &[(
            "crates/k/src/lib.rs",
            "/// Toy.\npub fn answer() -> u32 {\n    42\n}\n",
        )],
    );
    git(
        &wt,
        &["merge", "--no-ff", "-q", "-m", "join", "task/T-001-tests"],
    );
    let wt_target = wt.join("target");
    cargo(&wt, &wt_target, &["test"]);
    let before = manifest(&wt_target);

    let o = xtask(&root, &["join", "--revert-check", "T-001"]);
    assert_eq!(o.code, 0, "revert check: {}{}", o.stdout, o.stderr);
    let rec: Value = serde_json::from_str(&o.stdout).unwrap();
    assert_eq!(rec["result_kind"], "fails", "{rec}");
    assert_eq!(
        manifest(&wt_target),
        before,
        "revert check changed the task worktree's target"
    );

    // The task worktree still builds and tests its own (unreverted) tree.
    let t = cargo(&wt, &wt_target, &["test"]);
    assert!(
        t.stdout.contains("test answer_is_42 ... ok"),
        "{}",
        t.stdout
    );
}
