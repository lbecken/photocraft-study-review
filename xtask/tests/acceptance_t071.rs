//! T-071 acceptance tests (test author), written from the task file alone: every test drives the
//! built `xtask` binary (`CARGO_BIN_EXE_xtask`) on toy repositories created under
//! `CARGO_TARGET_TMPDIR`; nothing here reads or writes the Kiln repository except the no-skip
//! self-test fixtures (`xtask/fixtures/no-skip/`), which are copied into every no-skip toy
//! because the step runs its self-test there first (Pinned (c)).
//!
//! - AC-1: one toy per new finding kind of Interface (2), with exactly that fault: exactly one
//!   FAIL line of that kind, naming `<file>:<line>` and the item; the same toy corrected passes.
//!   The items sit on one source line each, so the line is the same whichever of an item's
//!   lines (cfg attribute, `#[test]`, `fn`) the tool reports. For a finding about a registry
//!   entry the spec does not say whether `<file>:<line>` is the item's place or the entry's
//!   place in `xtask/platform-tests.toml`; both are accepted (see the test author's log).
//! - AC-2: a correct registry passes and the detail line counts it
//!   (`<n> platform test(s) registered (<p> pending)`); malformed registries stop the step.
//! - AC-3: the `inert-helper` rule on the `acceptance_t002.rs` pattern, real twins, a single
//!   helper and each trivial body form the Interface lists.
//! - AC-4: `scorecard --check` with a listed test registered for other platforms only; the
//!   test asserts each platform's specified behaviour (Interface (3)): on Linux the page is
//!   computed and compared, elsewhere it is neither compared nor written.
//!
//! Every process has a timeout (D-028 (8)). No Unix-only API; printed paths are compared with
//! `\` read as `/`; platform differences are decided at run time with `cfg!`, never with a cfg
//! attribute (this file is itself scanned by `no-skip`).
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
/// The task file of the toy's `pending` entries (`tasks/T-NNN-*.md`).
const TASK: &str = "T-900";
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
        .join("t071-acc")
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

    /// `xtask no-skip` with no `--today`: HEAD's commit date (`head_date`) is today.
    fn run_no_skip_at_head(&self, head_date: &str) -> Out {
        self.commit(head_date);
        let o = self.xtask(&["no-skip"], 120);
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

/// A place a finding may name.
type Place = (String, RangeInclusive<usize>);

fn src(file: &str, line: usize) -> Place {
    (file.to_owned(), line..=line)
}

fn entry_place(reg: &Reg, i: usize) -> Place {
    (REGISTRY.to_owned(), reg.ranges[i].0..=reg.ranges[i].1)
}

/// The step fails with exactly one FAIL line of `kind`, naming one of `places` and `item`.
fn assert_one(o: &Out, kind: &str, places: &[Place], item: &str, case: &str) {
    assert_eq!(
        o.code,
        Some(1),
        "{case}: no-skip must FAIL with a `{kind}` finding:\n{}",
        o.text
    );
    let ls = kind_lines(o, kind);
    assert_eq!(
        ls.len(),
        1,
        "{case}: exactly one `{kind}` FAIL line expected:\n{}",
        o.text
    );
    assert!(
        has_token(ls[0], item),
        "{case}: the `{kind}` line names the item `{item}`: {}",
        ls[0]
    );
    assert!(
        places.iter().any(|(f, r)| names_at(ls[0], f, r)),
        "{case}: the `{kind}` line names `<file>:<line>` as one of {places:?}: {}",
        ls[0]
    );
}

fn assert_pass(o: &Out, case: &str) {
    assert_eq!(o.code, Some(0), "{case}: no-skip must pass:\n{}", o.text);
}

// ---- the registry --------------------------------------------------------------------------

struct Reg {
    text: String,
    /// 1-based first and last line of each entry.
    ranges: Vec<(usize, usize)>,
}

fn reg(entries: &[String]) -> Reg {
    let mut text = String::from("# toy platform-test registry\n");
    let mut ranges = Vec::new();
    for e in entries {
        text.push('\n');
        let start = text.lines().count() + 1;
        text.push_str(e);
        ranges.push((start, text.lines().count()));
    }
    Reg { text, ranges }
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

fn cp(counterpart: &str) -> String {
    format!("counterpart = \"{counterpart}\"\n")
}

fn pending(task: &str, expires: &str) -> String {
    format!(
        "counterpart = \"pending\"\nreason = \"the counterpart is written by {task}\"\ntask = \"{task}\"\nexpires = \"{expires}\"\n"
    )
}

// ---- the common toy source -----------------------------------------------------------------

const PLAT: &str = "crates/toy/tests/plat.rs";
/// `unix_only` on line 2; `everywhere` (no cfg) on line 3; a non-test helper on line 4.
const PLAT_RS: &str = "// toy test file (scanned by no-skip, never compiled)\n#[cfg(unix)] #[test] fn unix_only() { assert_eq!(helper(), 1); }\n#[test] fn everywhere() { assert_eq!(2, 2); }\nfn helper() -> u32 { 1 }\n";

/// The toy with `PLAT_RS` and `entries` as its registry.
fn plat_toy(name: &str, entries: &[String]) -> (Toy, Reg) {
    let t = Toy::no_skip(name);
    t.write(PLAT, PLAT_RS);
    let r = reg(entries);
    t.write(REGISTRY, &r.text);
    (t, r)
}

fn set_reg(t: &Toy, entries: &[String]) -> Reg {
    let r = reg(entries);
    t.write(REGISTRY, &r.text);
    r
}

// ---- AC-1: one toy per new kind -------------------------------------------------------------

#[test]
fn ac1_platform_unlisted_names_file_line_and_item() {
    // (file, source, item, line, platforms of the corrected entry)
    let cases: [(&str, &str, &str, usize, &str); 4] = [
        (PLAT, PLAT_RS, "unix_only", 2, "unix"),
        (
            "crates/toy/tests/winmod.rs",
            "// a module of tests behind a platform cfg\n#[cfg(windows)] mod win { #[test] fn reserved_name() { assert_eq!(1, 1); } }\n",
            "win",
            2,
            "windows",
        ),
        (
            "crates/toy/tests/linux_only.rs",
            "#![cfg(target_os = \"linux\")]\n#[test]\nfn reads_proc() {\n    assert_eq!(1, 1);\n}\n",
            "<file>",
            1,
            "linux",
        ),
        (
            "crates/toy/src/lib.rs",
            "//! toy library\n#[cfg(test)]\nmod tests {\n    #[cfg(target_os = \"macos\")] #[test] fn bundle() { assert_eq!(1, 1); }\n}\n",
            "tests::bundle",
            4,
            "macos",
        ),
    ];
    for (file, source, item, line, platforms) in cases {
        let t = Toy::no_skip(&format!("unlisted-{}", platforms));
        t.write(file, source);
        let o = t.run_no_skip();
        let case = format!("{file} `{item}` unregistered");
        assert_one(&o, "platform-unlisted", &[src(file, line)], item, &case);
        assert!(
            kind_lines(&o, "cfg-disabled").is_empty(),
            "{case}: a platform cfg is no longer `cfg-disabled` (Interface (1)):\n{}",
            o.text
        );
        set_reg(&t, &[entry(file, item, platforms, NONE)]);
        assert_pass(&t.run_no_skip(), &format!("{file} `{item}` registered"));
    }
}

#[test]
fn ac1_platform_mismatch_names_file_line_and_item() {
    let (t, r) = plat_toy("mismatch", &[entry(PLAT, "unix_only", "windows", NONE)]);
    let o = t.run_no_skip();
    assert_one(
        &o,
        "platform-mismatch",
        &[src(PLAT, 2), entry_place(&r, 0)],
        "unix_only",
        "`unix_only` registered as windows",
    );
    set_reg(&t, &[entry(PLAT, "unix_only", "unix", NONE)]);
    assert_pass(&t.run_no_skip(), "`unix_only` registered as unix");
}

/// Interface (1): the cfg forms that mean each platform name, and nothing else.
#[test]
fn ac1_platform_mismatch_each_cfg_form_means_exactly_its_platform() {
    let forms = [
        ("unix", "unix"),
        ("not(windows)", "unix"),
        ("windows", "windows"),
        ("not(unix)", "windows"),
        ("target_os = \"linux\"", "linux"),
        ("target_os = \"macos\"", "macos"),
    ];
    let t = Toy::no_skip("forms");
    for (form, meant) in forms {
        let source = format!("// toy\n#[cfg({form})] #[test] fn gated() {{ assert_eq!(1, 1); }}\n");
        t.write(PLAT, &source);
        for name in ["unix", "windows", "linux", "macos"] {
            let r = set_reg(&t, &[entry(PLAT, "gated", name, NONE)]);
            let o = t.run_no_skip();
            let case = format!("cfg({form}) registered as `{name}`");
            if name == meant {
                assert_pass(&o, &case);
            } else {
                assert_one(
                    &o,
                    "platform-mismatch",
                    &[src(PLAT, 2), entry_place(&r, 0)],
                    "gated",
                    &case,
                );
            }
        }
    }
}

#[test]
fn ac1_platform_unused_names_the_entry() {
    let good = entry(PLAT, "unix_only", "unix", NONE);
    // An entry for an item the file does not have, and one for a file that does not exist.
    let faults = [
        (PLAT, "removed_test"),
        ("crates/toy/tests/gone.rs", "gone_test"),
    ];
    for (file, item) in faults {
        let (t, r) = plat_toy(
            &format!("unused-{item}"),
            &[good.clone(), entry(file, item, "unix", NONE)],
        );
        let o = t.run_no_skip();
        assert_one(
            &o,
            "platform-unused",
            &[(file.to_owned(), 0..=usize::MAX), entry_place(&r, 1)],
            item,
            &format!("entry for `{item}` in {file}"),
        );
        set_reg(&t, std::slice::from_ref(&good));
        assert_pass(&t.run_no_skip(), "the unused entry removed");
    }
}

#[test]
fn ac1_platform_sentence_missing_doc_or_quote() {
    let faults = [
        ("docs/no-such-doc.md", QUOTE, "doc missing"),
        (DOC, "Sockets are refused", "quote not in doc"),
    ];
    for (doc, quote, case) in faults {
        let (t, r) = plat_toy(
            &format!("sentence-{}", case.replace(' ', "-")),
            &[entry_doc(PLAT, "unix_only", "unix", doc, quote, NONE)],
        );
        let o = t.run_no_skip();
        assert_one(
            &o,
            "platform-sentence",
            &[src(PLAT, 2), entry_place(&r, 0)],
            "unix_only",
            case,
        );
        set_reg(&t, &[entry(PLAT, "unix_only", "unix", NONE)]);
        assert_pass(&t.run_no_skip(), &format!("{case}: corrected"));
    }
}

#[test]
fn ac1_platform_counterpart_must_exist_and_cover_the_left_out_platforms() {
    // Lines: 2 unix_only, 3 everywhere, 4 helper (PLAT_RS), then:
    let extra = "#[cfg(unix)] #[test] fn unix_twin() { assert_eq!(3, 3); }\n\
                 #[cfg(windows)] #[test] fn win_case() { assert_eq!(4, 4); }\n\
                 #[cfg(target_os = \"linux\")] #[test] fn linux_case() { assert_eq!(5, 5); }\n\
                 mod no_tests { fn inner() {} }\n\
                 mod portable { #[test] fn p() { assert_eq!(6, 6); } }\n\
                 #[cfg(not(unix))] #[test] fn not_unix_case() { assert_eq!(7, 7); }\n";
    let source = format!("{PLAT_RS}{extra}");
    // The other platform items, registered correctly.
    let others = [
        entry(PLAT, "unix_twin", "unix", NONE),
        entry(PLAT, "win_case", "windows", NONE),
        entry(PLAT, "not_unix_case", "windows", NONE),
    ];
    let p = |item: &str| format!("{PLAT}::{item}");
    // (item under test, its platforms, its line, counterpart, accepted?)
    let cases: Vec<(&str, &str, usize, String, bool)> = vec![
        (
            "unix_only",
            "unix",
            2,
            "crates/toy/tests/nofile.rs::x".into(),
            false,
        ),
        ("unix_only", "unix", 2, p("no_such_test"), false),
        ("unix_only", "unix", 2, p("helper"), false),
        ("unix_only", "unix", 2, p("no_tests"), false),
        ("unix_only", "unix", 2, p("unix_twin"), false),
        ("linux_case", "linux", 7, p("win_case"), false),
        ("unix_only", "unix", 2, p("win_case"), true),
        ("unix_only", "unix", 2, p("not_unix_case"), true),
        ("unix_only", "unix", 2, p("everywhere"), true),
        ("unix_only", "unix", 2, p("portable"), true),
        (
            "unix_only",
            "unix",
            2,
            "crates/toy/src/lib.rs::tests::portable_lib".into(),
            true,
        ),
        ("linux_case", "linux", 7, p("everywhere"), true),
    ];
    let t = Toy::no_skip("counterpart");
    t.write(PLAT, &source);
    t.write(
        "crates/toy/src/lib.rs",
        "//! toy\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn portable_lib() {\n        assert_eq!(1, 1);\n    }\n}\n",
    );
    for (item, platforms, line, counterpart, ok) in cases {
        // The item under test with the counterpart; the other of unix_only/linux_case: `none`.
        let other = if item == "unix_only" {
            entry(PLAT, "linux_case", "linux", NONE)
        } else {
            entry(PLAT, "unix_only", "unix", NONE)
        };
        let mut entries = vec![entry(PLAT, item, platforms, &cp(&counterpart)), other];
        entries.extend(others.iter().cloned());
        let r = set_reg(&t, &entries);
        let o = t.run_no_skip();
        let case = format!("`{item}` ({platforms}) with counterpart `{counterpart}`");
        if ok {
            assert_pass(&o, &case);
        } else {
            assert_one(
                &o,
                "platform-counterpart",
                &[src(PLAT, line), entry_place(&r, 0)],
                item,
                &case,
            );
        }
    }
}

#[test]
fn ac1_platform_pending_expired_date_and_task_file() {
    let (t, r) = plat_toy(
        "pending",
        &[entry(
            PLAT,
            "unix_only",
            "unix",
            &pending(TASK, "2026-11-19"),
        )],
    );
    let places = |r: &Reg| [src(PLAT, 2), entry_place(r, 0)];
    assert_one(
        &t.run_no_skip(),
        "platform-pending-expired",
        &places(&r),
        "unix_only",
        "pending, expired the day before today",
    );
    for expires in [TODAY, "2026-12-31"] {
        set_reg(
            &t,
            &[entry(PLAT, "unix_only", "unix", &pending(TASK, expires))],
        );
        assert_pass(
            &t.run_no_skip(),
            &format!("pending, expires {expires} (today {TODAY})"),
        );
    }
    // The task names no file `tasks/T-901-*.md` on main's tree.
    let r = set_reg(
        &t,
        &[entry(
            PLAT,
            "unix_only",
            "unix",
            &pending("T-901", "2026-12-31"),
        )],
    );
    assert_one(
        &t.run_no_skip(),
        "platform-pending-expired",
        &places(&r),
        "unix_only",
        "pending, task file missing",
    );
    t.write("tasks/T-901-toy-second.md", "# T-901 toy\n");
    assert_pass(&t.run_no_skip(), "pending, task file added");
    // Without --today, today is HEAD's commit date.
    let r = set_reg(
        &t,
        &[entry(
            PLAT,
            "unix_only",
            "unix",
            &pending(TASK, "2026-12-01"),
        )],
    );
    let o = t.run_no_skip_at_head("2026-12-05");
    assert_one(
        &o,
        "platform-pending-expired",
        &places(&r),
        "unix_only",
        "pending expired 2026-12-01, HEAD dated 2026-12-05, no --today",
    );
    assert_pass(
        &t.run_no_skip_at_head("2026-11-25"),
        "pending expires 2026-12-01, HEAD dated 2026-11-25, no --today",
    );
}

#[test]
fn ac1_inert_helper_names_file_line_and_item() {
    let file = "crates/toy/tests/helpers.rs";
    let fault = "// toy helpers\n#[cfg(unix)] fn mode_of(p: &str) -> Option<u32> { Some(p.len() as u32) } #[cfg(not(unix))] fn mode_of(_p: &str) -> Option<u32> { None }\n#[test] fn uses_it() { assert!(mode_of(\"x\").is_some()); }\n";
    let t = Toy::no_skip("inert");
    t.write(file, fault);
    assert_one(
        &t.run_no_skip(),
        "inert-helper",
        &[src(file, 2)],
        "mode_of",
        "twin with body `None`",
    );
    t.write(
        file,
        &fault.replace(
            "{ None }",
            "{ panic!(\"modes are not bits on this platform: this test cannot run here\") }",
        ),
    );
    assert_pass(&t.run_no_skip(), "the twin fails, naming why");
}

#[test]
fn ac1_platform_statement_names_file_line_and_item() {
    let file = "crates/toy/tests/stmt.rs";
    let fault = "// toy\n#[test] fn modes() { #[cfg(unix)] assert_eq!(1, 1); assert_eq!(2, 2); }\n";
    let t = Toy::no_skip("statement");
    t.write(file, fault);
    assert_one(
        &t.run_no_skip(),
        "platform-statement",
        &[src(file, 2)],
        "modes",
        "a unix statement with nothing in its place",
    );
    for complement in ["#[cfg(not(unix))]", "#[cfg(windows)]"] {
        t.write(
            file,
            &fault.replace(
                "assert_eq!(2, 2);",
                &format!("{complement} panic!(\"no modes on this platform\"); assert_eq!(2, 2);"),
            ),
        );
        assert_pass(&t.run_no_skip(), &format!("complement {complement}"));
    }
}

/// Interface (2) `platform-statement` details: the complement must be in the same block;
/// `use` items are exempt; `cfg(not(windows))` is complemented by `cfg(windows)`.
#[test]
fn ac1_platform_statement_same_block_and_use_exemption() {
    let file = "crates/toy/tests/stmt.rs";
    let t = Toy::no_skip("statement-block");
    // The complement in the outer block does not answer a statement in an inner block.
    t.write(
        file,
        "// toy\n#[test]\nfn modes() {\n    {\n        #[cfg(unix)]\n        assert_eq!(1, 1);\n    }\n    #[cfg(windows)]\n    assert_eq!(3, 3);\n}\n",
    );
    let o = t.run_no_skip();
    assert_eq!(o.code, Some(1), "complement in another block:\n{}", o.text);
    let ls = kind_lines(&o, "platform-statement");
    assert!(
        !ls.is_empty() && ls.iter().all(|l| has_token(l, "modes")),
        "complement in another block: a `platform-statement` line naming `modes`:\n{}",
        o.text
    );
    // Same block, the not(windows)/windows pair, and an exempt `use`: passes.
    t.write(
        file,
        "// toy\n#[test]\nfn modes() {\n    #[cfg(unix)]\n    use std::os::unix::fs::PermissionsExt as _;\n    #[cfg(not(windows))]\n    {\n        assert_eq!(1, 1);\n    }\n    #[cfg(windows)]\n    panic!(\"no modes on Windows\");\n}\n",
    );
    assert_pass(&t.run_no_skip(), "same block, use exempt");
    // The `use` alone is exempt; a statement beside it is not.
    t.write(
        file,
        "// toy\n#[test]\nfn modes() {\n    #[cfg(unix)]\n    use std::os::unix::fs::PermissionsExt as _;\n    #[cfg(not(windows))]\n    assert_eq!(1, 1);\n}\n",
    );
    let o = t.run_no_skip();
    assert_eq!(
        kind_lines(&o, "platform-statement").len(),
        1,
        "the not(windows) statement without complement (the use is exempt):\n{}",
        o.text
    );
}

// ---- AC-2: a correct registry ----------------------------------------------------------------

/// `unix` with a Windows counterpart (and that counterpart's own entry), `pending` with a task
/// file and a future date, `none` with a reason; then a module and an inner-file entry.
#[test]
fn ac2_correct_registry_passes_and_the_detail_line_counts_it() {
    let source = "// toy (never compiled)\n\
        #[cfg(unix)] #[test] fn fifo_refused() { assert_eq!(1, 1); }\n\
        #[cfg(windows)] #[test] fn fifo_name_refused_on_windows() { assert_eq!(1, 1); }\n\
        #[cfg(target_os = \"macos\")] #[test] fn bundle_case() { assert_eq!(1, 1); }\n\
        #[cfg(target_os = \"linux\")] #[test] fn proc_status() { assert_eq!(1, 1); }\n\
        #[test]\n\
        fn both_platforms() {\n\
            #[cfg(unix)]\n\
            {\n\
                assert_eq!(1, 1);\n\
            }\n\
            #[cfg(not(unix))]\n\
            panic!(\"named pipes are a Unix facility; this platform has none\");\n\
        }\n\
        #[cfg(unix)] fn make_link() -> u32 { std::process::id() }\n\
        #[cfg(windows)] fn make_link() -> u32 { std::process::id() + 1 }\n";
    let t = Toy::no_skip("correct");
    t.write(PLAT, source);
    let p = |i: &str| format!("{PLAT}::{i}");
    let mut entries = vec![
        entry(
            PLAT,
            "fifo_refused",
            "unix",
            &cp(&p("fifo_name_refused_on_windows")),
        ),
        entry(
            PLAT,
            "fifo_name_refused_on_windows",
            "windows",
            &cp(&p("fifo_refused")),
        ),
        entry(PLAT, "bundle_case", "macos", &pending(TASK, "2026-12-31")),
        entry(PLAT, "proc_status", "linux", NONE),
    ];
    set_reg(&t, &entries);
    let o = t.run_no_skip();
    assert_pass(&o, "a correct registry of four entries");
    assert!(
        o.text.contains("4 platform test(s) registered (1 pending)"),
        "the detail line counts 4 entries, 1 pending:\n{}",
        o.text
    );
    // A module of tests and a file's inner cfg; one more pending.
    t.write(
        "crates/toy/tests/unix_mod.rs",
        "// toy\n#[cfg(not(windows))]\nmod fifo {\n    #[test]\n    fn fifo_in_module() {\n        assert_eq!(1, 1);\n    }\n}\n",
    );
    t.write(
        "crates/toy/tests/linux_only.rs",
        "#![cfg(target_os = \"linux\")]\n#[test]\nfn reads_proc() {\n    assert_eq!(1, 1);\n}\n",
    );
    entries.push(entry(
        "crates/toy/tests/unix_mod.rs",
        "fifo",
        "unix",
        &cp(&p("fifo_name_refused_on_windows")),
    ));
    entries.push(entry(
        "crates/toy/tests/linux_only.rs",
        "<file>",
        "linux",
        &pending(TASK, "2026-12-31"),
    ));
    set_reg(&t, &entries);
    let o = t.run_no_skip();
    assert_pass(&o, "a correct registry of six entries");
    assert!(
        o.text.contains("6 platform test(s) registered (2 pending)"),
        "the detail line counts 6 entries, 2 pending:\n{}",
        o.text
    );
}

/// Interface (1), Pinned (a): a malformed registry is an error of the step: it fails (exit 1
/// or 2, not a pass and not a panic). For the errors Interface (1) names (unknown key, empty
/// `reason`, empty quote, malformed date) and for text that is not TOML, the failure names the
/// registry file. For a missing required field and an unknown platform name the spec says
/// "required" / lists the values but not which finding reports it: only the failure is checked.
#[test]
fn ac2_malformed_registry_stops_the_step() {
    let item = |tail: &str| entry(PLAT, "unix_only", "unix", tail);
    // (case, registry entry text, named in the Interface as a parse error)
    let cases = [
        (
            "unknown key",
            format!("{}owner = \"someone\"\n", item(NONE)),
            true,
        ),
        (
            "empty reason (none)",
            item("counterpart = \"none\"\nreason = \"\"\n"),
            true,
        ),
        (
            "empty quote",
            entry_doc(PLAT, "unix_only", "unix", DOC, "", NONE),
            true,
        ),
        ("malformed date", item(&pending(TASK, "31.12.2026")), true),
        ("not TOML", "[[test]\nfile = \n".to_owned(), true),
        (
            "reason missing (none)",
            item("counterpart = \"none\"\n"),
            false,
        ),
        (
            "task missing (pending)",
            item("counterpart = \"pending\"\nreason = \"later\"\nexpires = \"2026-12-31\"\n"),
            false,
        ),
        (
            "expires missing (pending)",
            item(&format!(
                "counterpart = \"pending\"\nreason = \"later\"\ntask = \"{TASK}\"\n"
            )),
            false,
        ),
        (
            "unknown platform name",
            entry(PLAT, "unix_only", "bsd", NONE),
            false,
        ),
    ];
    let (t, _) = plat_toy("malformed", &[item(NONE)]);
    assert_pass(&t.run_no_skip(), "control: the well-formed registry");
    for (case, text, parse_error) in cases {
        t.write(REGISTRY, &format!("# toy\n\n{text}"));
        let o = t.run_no_skip();
        assert!(
            matches!(o.code, Some(1) | Some(2)),
            "{case}: the step fails (exit 1 or 2, not a pass or a panic):\n{}",
            o.text
        );
        if parse_error {
            assert!(
                o.text.contains("platform-tests.toml"),
                "{case}: the failure names the registry:\n{}",
                o.text
            );
        }
    }
}

// ---- AC-3: inert helpers ---------------------------------------------------------------------

const T002: &str = "crates/toy/tests/t002.rs";
/// After `acceptance_t002.rs:169-196` (Pinned (g)). `mode_and_inode`: lines 4-13; `set_mode`:
/// lines 15-21; `make_symlink`: lines 24-32.
const T002_RS: &str = "// toy after acceptance_t002.rs:169-196 (never compiled)\nuse std::path::Path;\n\n#[cfg(unix)]\nfn mode_and_inode(p: &Path) -> Option<(u32, u64)> {\n    use std::os::unix::fs::MetadataExt;\n    let m = std::fs::metadata(p).ok()?;\n    Some((m.mode(), m.ino()))\n}\n#[cfg(not(unix))]\nfn mode_and_inode(_p: &Path) -> Option<(u32, u64)> {\n    None\n}\n\n#[cfg(unix)]\nfn set_mode(p: &Path, mode: u32) {\n    use std::os::unix::fs::PermissionsExt;\n    std::fs::set_permissions(p, std::fs::Permissions::from_mode(mode)).unwrap();\n}\n#[cfg(not(unix))]\nfn set_mode(_p: &Path, _mode: u32) {}\n\n\n#[cfg(unix)]\nfn make_symlink(target: &Path, link: &Path) {\n    std::os::unix::fs::symlink(target, link).unwrap();\n}\n\n#[cfg(windows)]\nfn make_symlink(target: &Path, link: &Path) {\n    std::os::windows::fs::symlink_file(target, link).unwrap();\n}\n\n#[test]\nfn modes_survive() {\n    let p = Path::new(\"x\");\n    set_mode(p, 0o600);\n    if let Some((m, _)) = mode_and_inode(p) {\n        assert_eq!(m & 0o777, 0o600);\n    }\n    make_symlink(p, Path::new(\"y\"));\n}\n";

#[test]
fn ac3_t002_pattern_fails_real_twins_and_a_single_helper_pass() {
    // The layout the line ranges below assume.
    let lines: Vec<&str> = T002_RS.lines().collect();
    assert!(lines[4].starts_with("fn mode_and_inode(p"));
    assert!(lines[15].starts_with("fn set_mode(p"));
    assert!(lines[24].starts_with("fn make_symlink(target"));
    let t = Toy::no_skip("t002");
    t.write(T002, T002_RS);
    let o = t.run_no_skip();
    assert_eq!(o.code, Some(1), "the t002 pattern fails:\n{}", o.text);
    let ls = kind_lines(&o, "inert-helper");
    assert_eq!(
        ls.len(),
        2,
        "two inert helpers (mode_and_inode, set_mode):\n{}",
        o.text
    );
    for (name, range) in [("mode_and_inode", 4..=13), ("set_mode", 15..=21)] {
        assert!(
            ls.iter()
                .any(|l| has_token(l, name) && names_at(l, T002, &range)),
            "an `inert-helper` line names `{name}` in {T002} at one of its lines {range:?}:\n{}",
            o.text
        );
    }
    assert!(
        !o.text.lines().any(|l| has_token(l, "make_symlink")),
        "make_symlink has real bodies on both platforms: no finding:\n{}",
        o.text
    );
    // Real bodies on every platform, plus a single cfg(unix) helper without twin: passes.
    let fixed = T002_RS
        .replace(
            "fn mode_and_inode(_p: &Path) -> Option<(u32, u64)> {\n    None\n}",
            "fn mode_and_inode(p: &Path) -> Option<(u32, u64)> {\n    let m = std::fs::metadata(p).ok()?;\n    Some((u32::from(m.permissions().readonly()), m.len()))\n}",
        )
        .replace(
            "fn set_mode(_p: &Path, _mode: u32) {}",
            "fn set_mode(p: &Path, mode: u32) {\n    let mut perm = std::fs::metadata(p).unwrap().permissions();\n    perm.set_readonly(mode & 0o200 == 0);\n    std::fs::set_permissions(p, perm).unwrap();\n}",
        );
    assert_ne!(fixed, T002_RS);
    let fixed = format!("{fixed}\n#[cfg(unix)]\nfn only_on_unix() -> u32 {{\n    0\n}}\n");
    t.write(T002, &fixed);
    assert_pass(&t.run_no_skip(), "real twins and a single helper");
}

/// Interface (2): a definition is trivial when it has no statement, or one expression that is a
/// literal, `None`, `()`, `Ok(())`, `Default::default()`, or a tuple or struct of these; the
/// twins are under platform cfgs of the Interface (1) forms; a real body on each side passes.
#[test]
fn ac3_each_trivial_body_form_and_each_platform_pair() {
    let file = "crates/toy/tests/forms.rs";
    // (return type, trivial body)
    let trivial = [
        ("()", "{}"),
        ("Option<u32>", "{ None }"),
        ("u32", "{ 0 }"),
        ("bool", "{ false }"),
        ("&'static str", "{ \"\" }"),
        ("()", "{ () }"),
        ("std::io::Result<()>", "{ Ok(()) }"),
        ("u32", "{ Default::default() }"),
        ("(Option<u32>, u32)", "{ (None, 0) }"),
        ("Pair", "{ Pair { a: None, b: 0 } }"),
    ];
    let real = |ty: &str| match ty {
        "()" => "{ std::process::id(); }".to_owned(),
        "Option<u32>" => "{ Some(std::process::id()) }".to_owned(),
        "u32" => "{ std::process::id() }".to_owned(),
        "bool" => "{ std::process::id() > 0 }".to_owned(),
        "&'static str" => "{ std::env::consts::OS }".to_owned(),
        "std::io::Result<()>" => "{ std::fs::metadata(\".\").map(|_| ()) }".to_owned(),
        "(Option<u32>, u32)" => "{ (Some(std::process::id()), 1) }".to_owned(),
        "Pair" => "{ Pair { a: Some(std::process::id()), b: 1 } }".to_owned(),
        other => panic!("no real body for {other}"),
    };
    let header = "// toy\nstruct Pair { a: Option<u32>, b: u32 }\n";
    let t = Toy::no_skip("forms-inert");
    let check = |real_cfg: &str, triv_cfg: &str, ty: &str, triv: &str, inert: bool| {
        let source = format!(
            "{header}#[cfg({real_cfg})] fn h() -> {ty} {} #[cfg({triv_cfg})] fn h() -> {ty} {triv}\n#[test] fn uses_h() {{ let _ = h(); }}\n",
            real(ty)
        );
        t.write(file, &source);
        let o = t.run_no_skip();
        let case = format!("cfg({real_cfg}) real / cfg({triv_cfg}) `{triv}`");
        if inert {
            assert_one(&o, "inert-helper", &[src(file, 3)], "h", &case);
        } else {
            assert_pass(&o, &case);
        }
    };
    for (ty, triv) in trivial {
        check("unix", "not(unix)", ty, triv, true);
    }
    // Each platform pair, and the trivial body on the Unix side.
    check("not(windows)", "windows", "u32", "{ 0 }", true);
    check("windows", "unix", "u32", "{ 0 }", true);
    check("windows", "not(windows)", "Option<u32>", "{ None }", true);
    check(
        "target_os = \"linux\"",
        "target_os = \"macos\"",
        "u32",
        "{ 0 }",
        true,
    );
    // Not trivial: a body that fails naming why; a call.
    check(
        "unix",
        "not(unix)",
        "u32",
        "{ panic!(\"this facility does not exist on this platform\") }",
        false,
    );
    check(
        "unix",
        "not(unix)",
        "u32",
        "{ std::process::id() + 1 }",
        false,
    );
    // Not platform cfgs: no inert-helper.
    check(
        "feature = \"a\"",
        "not(feature = \"a\")",
        "u32",
        "{ 0 }",
        false,
    );
}

// ---- AC-4: the scorecard step per platform ---------------------------------------------------

fn host() -> &'static str {
    if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(windows) {
        "windows"
    } else {
        panic!("not a supported platform (METHODOLOGY §8.1)")
    }
}

const SC_FILE: &str = "crates/toy-a/tests/t.rs";
const SC_RS: &str = "//! Toy integration tests.\n#[test]\nfn plain() {\n    assert_eq!(1 + 1, 2);\n}\n\n#[cfg(windows)]\n#[test]\nfn win_only() {\n    assert_eq!(2 + 2, 4);\n}\n\n#[cfg(target_os = \"macos\")]\n#[test]\nfn mac_only() {\n    assert_eq!(3 + 3, 6);\n}\n";
const SC_ITEM: &str = "id = \"toy.plat\"\ntitle = \"Toy platform item\"\ncrate = \"toy-a\"\ntier = \"low\"\ntests = [\"toy_a::t::plain\", \"toy_a::t::win_only\", \"toy_a::t::mac_only\"]\n";
const STALE: &str = "# stale page\n";

/// A cargo workspace with one crate whose test file is `SC_RS`, one scorecard item listing its
/// three tests, and `entries` as the registry.
fn scorecard_toy(name: &str, entries: &[String]) -> Toy {
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
    t.write(REGISTRY, &reg(entries).text);
    t.write(DOC, DOC_TEXT);
    t.write("scorecard/items/toy.plat.toml", SC_ITEM);
    t.write("docs/scorecard.md", STALE);
    t.write(".gitignore", "/target\n");
    t.commit(TODAY);
    t
}

fn sc_entries(win_platforms: &str) -> Vec<String> {
    vec![
        entry(SC_FILE, "win_only", win_platforms, NONE),
        entry(SC_FILE, "mac_only", "macos", NONE),
    ]
}

fn detail_line(o: &Out) -> &str {
    o.text
        .lines()
        .find(|l| l.contains("scorecard:"))
        .unwrap_or_default()
}

/// The status cell of the toy item's row on the page.
fn toy_status(page: &str) -> String {
    let row = page
        .lines()
        .find(|l| l.starts_with("| `toy.plat` |"))
        .unwrap_or_else(|| panic!("the page has a row for toy.plat:\n{page}"));
    row.split('|').nth(5).unwrap_or_default().trim().to_owned()
}

#[test]
fn ac4_scorecard_registered_absent_test_is_not_missing_and_the_page_is_linux_only() {
    let t = scorecard_toy("scorecard", &sc_entries("windows"));
    let h = host();
    if h == "linux" {
        // win_only and mac_only are absent here, each registered for a platform that is not
        // Linux: not `Missing`; the page is computed (written) and compared.
        let o = t.xtask(&["scorecard"], 600);
        assert_eq!(
            o.code,
            Some(0),
            "linux: scorecard writes the page:\n{}",
            o.text
        );
        assert!(
            !o.text.contains("does not exist"),
            "linux: a registered absent test is not Missing:\n{}",
            o.text
        );
        let page = t.read("docs/scorecard.md");
        assert_ne!(page, STALE, "linux: the page is written");
        assert_ne!(
            toy_status(&page),
            "planned",
            "linux: the item is not `planned` for a registered absent test:\n{page}"
        );
        let o = t.xtask(&["scorecard", "--check"], 600);
        assert_eq!(
            o.code,
            Some(0),
            "linux: --check on the fresh page:\n{}",
            o.text
        );
        t.write("docs/scorecard.md", &format!("{page}\nedited\n"));
        let o = t.xtask(&["scorecard", "--check"], 600);
        assert_eq!(
            o.code,
            Some(1),
            "linux: the page is compared (an edited page fails --check):\n{}",
            o.text
        );
    } else {
        // macOS, Windows: the built tests run, the absent one is registered for another
        // platform; the (stale) page is neither compared nor written.
        let o = t.xtask(&["scorecard", "--check"], 600);
        assert_eq!(
            o.code,
            Some(0),
            "{h}: --check passes with a stale page (not compared):\n{}",
            o.text
        );
        assert!(
            detail_line(&o).to_ascii_lowercase().contains(h),
            "{h}: the detail line names the platform:\n{}",
            o.text
        );
        assert_eq!(t.read("docs/scorecard.md"), STALE, "{h}: nothing written");
    }
    // An absent test whose entry does not exclude this platform is a problem: win_only
    // registered (wrongly) as `unix` is absent on Linux and macOS.
    t.write(REGISTRY, &reg(&sc_entries("unix")).text);
    let o = t.xtask(&["scorecard", "--check"], 600);
    if h == "windows" {
        assert_eq!(
            o.code,
            Some(0),
            "windows: win_only exists and runs here:\n{}",
            o.text
        );
    } else {
        assert_eq!(
            o.code,
            Some(1),
            "{h}: win_only registered unix:\n{}",
            o.text
        );
        assert!(
            o.text
                .lines()
                .any(|l| l.contains("toy_a::t::win_only") && l.contains("does not exist")),
            "{h}: a problem naming the absent test toy_a::t::win_only:\n{}",
            o.text
        );
        if h == "macos" {
            assert!(
                o.text
                    .lines()
                    .any(|l| l.contains("toy_a::t::win_only")
                        && l.contains("does not exist on macos")),
                "macos: `… does not exist on macos`:\n{}",
                o.text
            );
        }
    }
}

#[test]
fn ac4_scorecard_write_mode_writes_on_linux_and_is_refused_elsewhere() {
    let t = scorecard_toy("scorecard-write", &sc_entries("windows"));
    let h = host();
    let o = t.xtask(&["scorecard"], 600);
    if h == "linux" {
        assert_eq!(o.code, Some(0), "linux: write mode:\n{}", o.text);
        assert!(
            t.read("docs/scorecard.md").starts_with("# Scorecard"),
            "linux: the page is written"
        );
    } else {
        assert_eq!(o.code, Some(1), "{h}: write mode is refused:\n{}", o.text);
        assert!(
            o.text
                .lines()
                .any(|l| l.trim() == "scorecard: refused: docs/scorecard.md is computed on Linux"),
            "{h}: the one refusal line:\n{}",
            o.text
        );
        assert_eq!(t.read("docs/scorecard.md"), STALE, "{h}: nothing written");
    }
}
