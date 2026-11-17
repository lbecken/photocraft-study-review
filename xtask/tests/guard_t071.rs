//! T-071 guards (test author): today's `no-skip` and `scorecard` behaviour that must stay
//! (Interface (2) "The existing six kinds, `skips.toml`, its expiry rule and the self-test
//! stay"; Interface (1) "Any other cfg on a test stays a `cfg-disabled` finding"; Interface (3)
//! a listed test absent without a registry entry is a problem on every platform). Each passes on
//! the base and must pass after T-071. Like `acceptance_t071.rs`, every test drives the built
//! `xtask` binary on toy repositories under `CARGO_TARGET_TMPDIR` (the helpers are a copy of
//! that file's), with a timeout on every process and no Unix-only API.
#![cfg(test)]

use std::io::Read as _;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const XTASK: &str = env!("CARGO_BIN_EXE_xtask");
/// "Today" for every no-skip run (passed with `--today` and used as HEAD's commit date).
const TODAY: &str = "2026-11-20";
const REGISTRY: &str = "xtask/platform-tests.toml";
const DOC: &str = "docs/toy-protocol.md";
const QUOTE: &str = "Named pipes (FIFOs) are refused";
const DOC_TEXT: &str = "# Toy protocol\n\nNamed pipes (FIFOs) are refused on every platform that has them.\nOn Windows a reserved device name such as NUL is refused.\n";
const TASK_FILE: &str = "tasks/T-900-toy-counterparts.md";

// ---- files and processes -------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repo_root() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("xtask has a parent directory")
        .to_path_buf()
}

/// A fresh, empty scratch directory for one test.
fn scratch(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("t071-guard")
        .join(name);
    if d.exists() {
        std::fs::remove_dir_all(&d).unwrap();
    }
    std::fs::create_dir_all(&d).unwrap();
    d
}

struct Out {
    code: Option<i32>,
    /// stdout, then stderr, with `\` read as `/` (paths as Windows may print them).
    text: String,
}

/// The environment a director's shell would give a child: nothing inherited from `cargo test`
/// (target directory, package variables), an isolated git configuration, offline cargo.
fn hygiene<'c>(c: &'c mut Command, gitcfg: &Path) -> &'c mut Command {
    for (k, _) in std::env::vars_os() {
        let Some(k) = k.to_str() else { continue };
        if (k.starts_with("CARGO_") && k != "CARGO_HOME") || k.starts_with("GIT_") {
            c.env_remove(k);
        }
    }
    c.env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", gitcfg)
        .env("GIT_AUTHOR_NAME", "Toy")
        .env("GIT_AUTHOR_EMAIL", "toy@example.invalid")
        .env("GIT_COMMITTER_NAME", "Toy")
        .env("GIT_COMMITTER_EMAIL", "toy@example.invalid")
        .env("CARGO_NET_OFFLINE", "true")
}

fn run_to(c: &mut Command, secs: u64) -> Out {
    c.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = c
        .spawn()
        .unwrap_or_else(|e| panic!("precondition: cannot start {c:?}: {e}"));
    let mut so = child.stdout.take().unwrap();
    let mut se = child.stderr.take().unwrap();
    let t1 = std::thread::spawn(move || {
        let mut v = Vec::new();
        let _ = so.read_to_end(&mut v);
        v
    });
    let t2 = std::thread::spawn(move || {
        let mut v = Vec::new();
        let _ = se.read_to_end(&mut v);
        v
    });
    let deadline = Instant::now() + Duration::from_secs(secs);
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break s;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("timed out after {secs}s: {c:?}");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&t1.join().unwrap()),
        String::from_utf8_lossy(&t2.join().unwrap())
    )
    .replace('\\', "/");
    Out {
        code: status.code(),
        text,
    }
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let p = e.path();
        if p.is_dir() {
            copy_tree(&p, &to.join(e.file_name()));
        } else {
            std::fs::copy(&p, to.join(e.file_name())).unwrap();
        }
    }
}

/// Every `key = "value"` value in `text` (TOML-like, inline tables included).
fn quoted_values(text: &str, key: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (i, _) in text.match_indices(key) {
        if text[..i].bytes().last().is_some_and(is_word) {
            continue;
        }
        let rest = text[i + key.len()..].trim_start_matches([' ', '\t']);
        let Some(rest) = rest.strip_prefix('=') else {
            continue;
        };
        let rest = rest.trim_start_matches([' ', '\t']);
        let Some(rest) = rest.strip_prefix('"') else {
            continue;
        };
        if let Some(end) = rest.find(['"', '\n'])
            && rest[end..].starts_with('"')
        {
            out.push(rest[..end].to_owned());
        }
    }
    out
}

fn files_under(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            files_under(&p, out);
        } else {
            out.push(p);
        }
    }
}

// ---- toy repositories ----------------------------------------------------------------------

struct Toy {
    dir: PathBuf,
    gitcfg: PathBuf,
}

impl Toy {
    /// An empty git repository on branch `main` (no files).
    fn bare(name: &str) -> Toy {
        let s = scratch(name);
        let dir = s.join("repo");
        std::fs::create_dir_all(&dir).unwrap();
        let gitcfg = s.join("gitconfig");
        std::fs::write(&gitcfg, "").unwrap();
        let t = Toy { dir, gitcfg };
        t.git(&["init", "-q"], &[]);
        t.git(&["symbolic-ref", "HEAD", "refs/heads/main"], &[]);
        t
    }

    /// A toy for `no-skip`: a workspace root, an empty `skips.toml` and registry, the
    /// self-test fixtures, the toy doc and the toy task file.
    fn no_skip(name: &str) -> Toy {
        let t = Toy::bare(name);
        t.write(
            "Cargo.toml",
            "[workspace]\nmembers = []\nresolver = \"2\"\n",
        );
        t.write("xtask/skips.toml", "# toy allowlist: empty\n");
        t.write(REGISTRY, "# toy registry: empty\n");
        t.write(DOC, DOC_TEXT);
        t.write(TASK_FILE, "# T-900 Toy counterparts\n");
        t.copy_fixtures();
        t
    }

    /// The self-test fixtures, and any file outside them that a fixture names as a `doc` or
    /// a `task` (the self-test runs in the toy, so what it reads must be there).
    fn copy_fixtures(&self) {
        let src = manifest_dir().join("fixtures").join("no-skip");
        assert!(src.is_dir(), "precondition: {} is missing", src.display());
        copy_tree(&src, &self.dir.join("xtask/fixtures/no-skip"));
        let mut files = Vec::new();
        files_under(&src, &mut files);
        for f in files {
            let Ok(text) = std::fs::read_to_string(&f) else {
                continue;
            };
            for doc in quoted_values(&text, "doc") {
                let from = repo_root().join(&doc);
                let to = self.dir.join(&doc);
                if from.is_file() && !to.exists() {
                    std::fs::create_dir_all(to.parent().unwrap()).unwrap();
                    std::fs::copy(&from, &to).unwrap();
                }
            }
            for task in quoted_values(&text, "task") {
                let Ok(rd) = std::fs::read_dir(repo_root().join("tasks")) else {
                    continue;
                };
                for e in rd {
                    let e = e.unwrap();
                    let n = e.file_name().to_string_lossy().into_owned();
                    if n.starts_with(&format!("{task}-")) && n.ends_with(".md") {
                        self.write(
                            &format!("tasks/{n}"),
                            &std::fs::read_to_string(e.path()).unwrap(),
                        );
                    }
                }
            }
        }
    }

    fn write(&self, rel: &str, text: &str) {
        let p = self.dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, text).unwrap();
    }

    fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.dir.join(rel)).unwrap()
    }

    fn git(&self, args: &[&str], envs: &[(&str, &str)]) {
        let mut c = Command::new("git");
        hygiene(&mut c, &self.gitcfg);
        c.args(args).current_dir(&self.dir);
        for (k, v) in envs {
            c.env(k, v);
        }
        let o = run_to(&mut c, 120);
        assert_eq!(o.code, Some(0), "git {args:?}: {}", o.text);
    }

    /// Commit the whole tree on `main`, dated `date` (so main's tree is the working tree and
    /// HEAD's commit date is `date`).
    fn commit(&self, date: &str) {
        let when = format!("{date}T12:00:00+00:00");
        self.git(&["add", "-A"], &[]);
        self.git(
            &["commit", "-q", "--allow-empty", "--no-verify", "-m", "toy"],
            &[("GIT_AUTHOR_DATE", &when), ("GIT_COMMITTER_DATE", &when)],
        );
    }

    fn xtask(&self, args: &[&str], secs: u64) -> Out {
        let mut c = Command::new(XTASK);
        hygiene(&mut c, &self.gitcfg);
        c.args(args).current_dir(&self.dir);
        run_to(&mut c, secs)
    }

    /// `xtask no-skip --today TODAY` on the committed tree.
    fn run_no_skip(&self) -> Out {
        self.commit(TODAY);
        let o = self.xtask(&["no-skip", "--today", TODAY], 120);
        precondition_self_test(&o);
        o
    }
}

fn precondition_self_test(o: &Out) {
    assert!(
        !o.text.contains("self-test failed"),
        "precondition: no-skip's self-test failed in the toy repository (a fixture of xtask/fixtures/no-skip/ may need a file this toy lacks):\n{}",
        o.text
    );
}

// ---- reading the output --------------------------------------------------------------------

fn is_word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'-' || c == b'_'
}

/// `tok` occurs in `line` not as part of a longer name (`unix_only` is not in `unix_only_2`,
/// `ignore` is not in `doc-ignore`).
fn has_token(line: &str, tok: &str) -> bool {
    let b = line.as_bytes();
    let mut from = 0;
    while let Some(i) = line[from..].find(tok) {
        let s = from + i;
        let e = s + tok.len();
        let first = tok.as_bytes()[0];
        let last = tok.as_bytes()[tok.len() - 1];
        let before_ok = s == 0 || !is_word(first) || !is_word(b[s - 1]);
        let after_ok = e == b.len() || !is_word(last) || !is_word(b[e]);
        if before_ok && after_ok {
            return true;
        }
        from = s + 1;
    }
    false
}

/// `line` names `<file>:<n>` for some `n` in `lines` (and no longer number).
fn names_at(line: &str, file: &str, lines: &RangeInclusive<usize>) -> bool {
    let pat = format!("{file}:");
    let mut from = 0;
    while let Some(i) = line[from..].find(&pat) {
        let s = from + i + pat.len();
        let digits: String = line[s..].chars().take_while(char::is_ascii_digit).collect();
        if let Ok(n) = digits.parse::<usize>()
            && lines.contains(&n)
        {
            return true;
        }
        from = s;
    }
    false
}

fn kind_lines<'a>(o: &'a Out, kind: &str) -> Vec<&'a str> {
    o.text.lines().filter(|l| has_token(l, kind)).collect()
}

fn assert_pass(o: &Out, case: &str) {
    assert_eq!(o.code, Some(0), "{case}: no-skip must pass:\n{}", o.text);
}

// ---- the registry --------------------------------------------------------------------------

struct Reg {
    text: String,
}

fn reg(entries: &[String]) -> Reg {
    let mut text = String::from("# toy platform-test registry\n");
    for e in entries {
        text.push('\n');
        text.push_str(e);
    }
    Reg { text }
}

fn entry_doc(
    file: &str,
    item: &str,
    platforms: &str,
    doc: &str,
    quote: &str,
    tail: &str,
) -> String {
    format!(
        "[[test]]\nfile = \"{file}\"\nitem = \"{item}\"\nplatforms = \"{platforms}\"\ndoc = \"{doc}\"\nquote = \"{quote}\"\n{tail}"
    )
}

fn entry(file: &str, item: &str, platforms: &str, tail: &str) -> String {
    entry_doc(file, item, platforms, DOC, QUOTE, tail)
}

const NONE: &str =
    "counterpart = \"none\"\nreason = \"the toy has this facility on the named platforms only\"\n";

// ---- the common toy source -----------------------------------------------------------------

const PLAT: &str = "crates/toy/tests/plat.rs";
fn set_reg(t: &Toy, entries: &[String]) {
    t.write(REGISTRY, &reg(entries).text);
}

// ---- no-skip: what stays ---------------------------------------------------------------

const OLD: &str = "crates/toy/tests/old.rs";
/// One item per line: (kind, item, line).
const OLD_RS: &str = "// toy: today's six kinds (never compiled)\n#[doc = \"```ignore\\nlet x = 1;\\n```\"] pub fn documented() {}\n#[test] #[ignore = \"slow\"] fn slow_case() {}\n#[test] #[cfg_attr(target_os = \"linux\", ignore)] fn linux_ignored() {}\n#[test] fn helper_skip() { skip_if_no_display!(); }\n#[test] fn early_exit() { if std::env::var(\"CI\").is_err() { return; } assert!(true); }\n#[test] #[cfg(feature = \"slow\")] fn feature_gated() {}\n";

#[test]
fn guard_the_six_existing_kinds_are_still_reported_with_file_line_and_item() {
    let t = Toy::no_skip("six-kinds");
    t.write(OLD, OLD_RS);
    let o = t.run_no_skip();
    assert_eq!(o.code, Some(1), "the six kinds fail the step:\n{}", o.text);
    for (kind, item, line) in [
        ("doc-ignore", "documented", 2),
        ("ignore", "slow_case", 3),
        ("cfg-attr-ignore", "linux_ignored", 4),
        ("skip-call", "helper_skip", 5),
        ("early-return", "early_exit", 6),
        ("cfg-disabled", "feature_gated", 7),
    ] {
        assert!(
            kind_lines(&o, kind)
                .iter()
                .any(|l| has_token(l, item) && names_at(l, OLD, &(line..=line))),
            "a `{kind}` line naming {OLD}:{line} and `{item}`:\n{}",
            o.text
        );
    }
}

#[test]
fn guard_skips_toml_still_allowlists_until_expiry_and_refuses_unused_entries() {
    let t = Toy::no_skip("skips");
    t.write(
        OLD,
        "// toy\n#[test] #[cfg(feature = \"slow\")] fn feature_gated() {}\n",
    );
    let allow = |expires: &str| {
        format!(
            "[[skip]]\nfile = \"{OLD}\"\nitem = \"feature_gated\"\nkind = \"cfg-disabled\"\nreason = \"toy: the slow feature is built elsewhere\"\nexpires = \"{expires}\"\n"
        )
    };
    t.write("xtask/skips.toml", &allow("2026-12-31"));
    assert_pass(&t.run_no_skip(), "listed, unexpired");
    t.write("xtask/skips.toml", &allow(TODAY));
    assert_pass(&t.run_no_skip(), "listed, expires today");
    t.write("xtask/skips.toml", &allow("2026-11-19"));
    let o = t.run_no_skip();
    assert_eq!(o.code, Some(1), "listed, expired:\n{}", o.text);
    t.write(
        "xtask/skips.toml",
        &format!(
            "{}\n[[skip]]\nfile = \"{OLD}\"\nitem = \"no_such\"\nreason = \"toy\"\nexpires = \"2026-12-31\"\n",
            allow("2026-12-31")
        ),
    );
    let o = t.run_no_skip();
    assert_eq!(o.code, Some(1), "an unused skips.toml entry:\n{}", o.text);
}

/// Interface (1) and ruling p5 (D-220): exactly the six cfg forms mean a platform;
/// `target_os = "windows"`, `any(...)`, `all(...)` and every other cfg on a test stay
/// `cfg-disabled`, registered or not.
#[test]
fn guard_other_cfgs_stay_cfg_disabled_registered_or_not() {
    let t = Toy::no_skip("other-cfgs");
    for (form, platforms) in [
        ("feature = \"x\"", "unix"),
        ("target_os = \"freebsd\"", "unix"),
        ("all(unix, feature = \"x\")", "unix"),
        ("target_os = \"windows\"", "windows"),
        ("any(unix, windows)", "unix"),
    ] {
        t.write(
            PLAT,
            &format!("// toy\n#[cfg({form})] #[test] fn gated() {{ assert_eq!(1, 1); }}\n"),
        );
        for registered in [false, true] {
            if registered {
                set_reg(&t, &[entry(PLAT, "gated", platforms, NONE)]);
            } else {
                set_reg(&t, &[]);
            }
            let o = t.run_no_skip();
            let case = format!("cfg({form}), registered: {registered}");
            assert_eq!(o.code, Some(1), "{case}:\n{}", o.text);
            assert!(
                kind_lines(&o, "cfg-disabled")
                    .iter()
                    .any(|l| has_token(l, "gated") && names_at(l, PLAT, &(2..=2))),
                "{case}: a `cfg-disabled` line naming {PLAT}:2 and `gated`:\n{}",
                o.text
            );
        }
    }
}

#[test]
fn guard_the_self_test_still_runs_before_the_scan() {
    let t = Toy::no_skip("self-test");
    t.write(PLAT, "// toy\n#[test] fn plain() { assert_eq!(1, 1); }\n");
    assert_pass(&t.run_no_skip(), "control: intact fixtures");
    let fx = "xtask/fixtures/no-skip";
    let violations = t.read(&format!("{fx}/violations.rs"));
    let clean = t.read(&format!("{fx}/clean.rs"));
    t.write(&format!("{fx}/violations.rs"), &clean);
    t.commit(TODAY);
    let o = t.xtask(&["no-skip", "--today", TODAY], 120);
    assert!(
        matches!(o.code, Some(1) | Some(2)),
        "a violations fixture with no findings fails the step:\n{}",
        o.text
    );
    t.write(&format!("{fx}/violations.rs"), &violations);
    t.write(&format!("{fx}/clean.rs"), &violations);
    t.commit(TODAY);
    let o = t.xtask(&["no-skip", "--today", TODAY], 120);
    assert!(
        matches!(o.code, Some(1) | Some(2)),
        "a clean fixture with findings fails the step:\n{}",
        o.text
    );
}

/// What is not a platform-conditional test, helper or statement: `cfg!` at run time, the
/// `cfg(test)` module, a cfg'd `use` beside a test, a single platform helper.
#[test]
fn guard_runtime_cfg_and_cfg_test_are_not_findings() {
    let t = Toy::no_skip("not-findings");
    t.write(
        "crates/toy/src/lib.rs",
        "//! toy\n#[cfg(unix)]\npub fn unix_helper() -> u32 {\n    std::process::id()\n}\n\n#[cfg(test)]\nmod tests {\n    #[cfg(unix)]\n    use std::os::unix::fs::PermissionsExt as _;\n\n    #[test]\n    fn runtime_branch() {\n        if cfg!(windows) {\n            assert_eq!(1, 1);\n        } else {\n            assert_eq!(2, 2);\n        }\n    }\n}\n",
    );
    assert_pass(&t.run_no_skip(), "runtime cfg!, cfg(test), single helper");
}

// ---- scorecard: what stays ----------------------------------------------------------------

/// A listed test that exists nowhere, with no registry entry, is a problem on every platform.
#[test]
fn guard_scorecard_listed_test_absent_without_entry_is_a_problem_everywhere() {
    let t = scorecard_toy("scorecard-missing");
    let o = t.xtask(&["scorecard", "--check"], 600);
    assert_eq!(o.code, Some(1), "an absent, unregistered test:\n{}", o.text);
    assert!(
        o.text
            .lines()
            .any(|l| l.contains("toy_a::t::nope") && l.contains("does not exist")),
        "a problem naming toy_a::t::nope as not existing:\n{}",
        o.text
    );
}

const SC_FILE: &str = "crates/toy-a/tests/t.rs";
const SC_RS: &str = "//! Toy integration tests.\n#[test]\nfn plain() {\n    assert_eq!(1 + 1, 2);\n}\n\n#[cfg(windows)]\n#[test]\nfn win_only() {\n    assert_eq!(2 + 2, 4);\n}\n\n#[cfg(target_os = \"macos\")]\n#[test]\nfn mac_only() {\n    assert_eq!(3 + 3, 6);\n}\n";
const SC_ITEM: &str = "id = \"toy.plat\"\ntitle = \"Toy platform item\"\ncrate = \"toy-a\"\ntier = \"low\"\ntests = [\"toy_a::t::plain\", \"toy_a::t::nope\"]\n";
const STALE: &str = "# stale page\n";

/// A cargo workspace with one crate whose test file is `SC_RS`, one scorecard item listing its
/// `plain` and a test that exists nowhere, and an empty registry.
fn scorecard_toy(name: &str) -> Toy {
    let t = Toy::bare(name);
    t.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/toy-a\"]\nresolver = \"2\"\n",
    );
    t.write(
        "Cargo.lock",
        "# This file is automatically @generated by Cargo.\n# It is not intended for manual editing.\nversion = 3\n\n[[package]]\nname = \"toy-a\"\nversion = \"0.1.0\"\n",
    );
    t.write(
        "crates/toy-a/Cargo.toml",
        "[package]\nname = \"toy-a\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n",
    );
    t.write("crates/toy-a/src/lib.rs", "//! Toy library.\n");
    t.write(SC_FILE, SC_RS);
    t.write(
        "xtask/layers.toml",
        "testkit_max_layer = 4\nheadless_forbidden = []\n\n[[crate]]\nname = \"toy-a\"\nlayer = 0\n",
    );
    t.write("xtask/skips.toml", "# toy\n");
    t.write(REGISTRY, "# toy registry: empty\n");
    t.write(DOC, DOC_TEXT);
    t.write("scorecard/items/toy.plat.toml", SC_ITEM);
    t.write("docs/scorecard.md", STALE);
    t.write(".gitignore", "/target\n");
    t.commit(TODAY);
    t
}
