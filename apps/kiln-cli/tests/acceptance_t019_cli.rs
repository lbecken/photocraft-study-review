//! T-019 acceptance tests through `kiln-cli` (`exec`, `run`, `control`, with `--root` and
//! `--memory-seed`): `project.tree` lists the session root as a tree, honours `.gitignore`
//! files, always skips the root's `target` directory and every `.git` entry, never follows a
//! symbolic link, and stops at the depth bound and at `limit`. Written by the test author from
//! the task specification alone (`tasks/T-019-project-tree.md`).
//!
//! Oracles: `git ls-files -z` in a fresh repository with no global or system configuration
//! (AC-1), a naive model of the walk rules written here (AC-2; it is first checked against git
//! on every tree, so a model that disagrees with git fails as a test bug before Kiln is asked),
//! memory against disk (AC-3) and exact values worked out in the specification (AC-2, AC-4 to
//! AC-6). Random trees come from a hand-written PRNG with fixed seeds.
//!
//! Every spawned process is killed after a timeout (D-028 (8)); a hang is a test failure. Every
//! fixture is built in a fresh temporary directory that is removed on drop. On the base
//! `kiln-cli exec project.tree` exits 2 with `unknown_command`, so each test fails there by
//! assertion. No test asserts a message text, stderr text, a whole document, the `commands`
//! list or `error_kinds` (Pinned (f)); refusals are checked by `code` and `kind` only.
#![cfg(test)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::{FileTypeExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// The ordinary per-process timeout (AC-1 and AC-6: 30 s); only a hang detector.
const TIMEOUT: Duration = Duration::from_secs(30);
/// AC-4's hang detector for the FIFO and the link loop (not a speed claim).
const HANG: Duration = Duration::from_secs(5);
/// Number of fixed-seed random trees per criterion (AC-1: at least 100).
const RANDOM_TREES: u64 = 120;

// ---------------------------------------------------------------------------------------------
// Processes.

/// Run `cmd` with `input` on stdin (or empty stdin), killing it after `timeout`.
fn spawn_with_timeout(mut cmd: Command, input: Option<&[u8]>, timeout: Duration) -> Output {
    let what = format!("{cmd:?}");
    let what = if what.len() > 300 {
        format!("{}…", &what[..300])
    } else {
        what
    };
    cmd.stdin(if input.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    })
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap_or_else(|e| panic!("spawn {what}: {e}"));
    let writer = input.map(|bytes| {
        let bytes = bytes.to_vec();
        let mut stdin = child.stdin.take().unwrap();
        thread::spawn(move || {
            let _ = stdin.write_all(&bytes);
        })
    });
    let drain = |mut r: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = r.read_to_end(&mut buf);
            buf
        })
    };
    let out = drain(Box::new(child.stdout.take().unwrap()));
    let err = drain(Box::new(child.stderr.take().unwrap()));
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{what} did not finish within {timeout:?} (hang)");
        }
        thread::sleep(Duration::from_millis(2));
    };
    if let Some(w) = writer {
        let _ = w.join();
    }
    Output {
        status,
        stdout: out.join().unwrap(),
        stderr: err.join().unwrap(),
    }
}

/// `kiln-cli args` in `cwd`.
fn kiln(args: &[&str], cwd: &Path, input: Option<&[u8]>, timeout: Duration) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kiln-cli"));
    cmd.args(args).current_dir(cwd);
    spawn_with_timeout(cmd, input, timeout)
}

/// `kiln-cli exec project.tree <params> <source…>` in `cwd`: `Ok(result)` on exit 0,
/// `Err(error object)` on exit 2 (the error object on stderr).
fn tree_exec(
    cwd: &Path,
    params: &Value,
    source: &[&str],
    timeout: Duration,
) -> Result<Value, Value> {
    let p = params.to_string();
    let mut args = vec!["exec", "project.tree", p.as_str()];
    args.extend_from_slice(source);
    let out = kiln(&args, cwd, None, timeout);
    match out.status.code() {
        Some(0) => {
            let text = String::from_utf8(out.stdout).expect("stdout is UTF-8");
            Ok(serde_json::from_str(text.trim_end()).unwrap_or_else(|e| {
                panic!("exec project.tree {p}: stdout is not one JSON value ({e})")
            }))
        }
        Some(2) => {
            let text = String::from_utf8_lossy(&out.stderr).into_owned();
            let first = text.lines().next().unwrap_or("");
            Err(serde_json::from_str(first).unwrap_or_else(|e| {
                panic!(
                    "exec project.tree {p}: exit 2 without an error object on stderr ({e}): {text}"
                )
            }))
        }
        other => panic!(
            "exec project.tree {p}: unexpected exit {other:?}; stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        ),
    }
}

/// `project.tree` must succeed; on the base this fails with the `unknown_command` response.
fn tree_ok(cwd: &Path, params: &Value, source: &[&str], timeout: Duration) -> Value {
    match tree_exec(cwd, params, source, timeout) {
        Ok(v) => v,
        Err(e) => panic!("project.tree {params} {source:?} was refused: {e}"),
    }
}

/// `project.tree` must be refused with this `code` and `kind` (message text is not checked).
fn tree_refused(cwd: &Path, params: &Value, source: &[&str], code: &str, kind: &str) {
    match tree_exec(cwd, params, source, TIMEOUT) {
        Ok(v) => panic!("project.tree {params} {source:?} succeeded, expected {code}/{kind}: {v}"),
        Err(e) => {
            assert_eq!(e["code"], json!(code), "project.tree {params}: {e}");
            assert_eq!(e["kind"], json!(kind), "project.tree {params}: {e}");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The result of `project.tree`.

#[derive(Clone, Debug, PartialEq, Eq)]
struct Listing {
    entries: Vec<(String, String)>,
    ignored: u64,
    skipped: u64,
    too_deep: u64,
    truncated: bool,
}

fn listing(v: &Value) -> Listing {
    let entries = v["entries"]
        .as_array()
        .unwrap_or_else(|| panic!("no `entries` array: {v}"))
        .iter()
        .map(|e| {
            let path = e["path"]
                .as_str()
                .unwrap_or_else(|| panic!("entry without path: {e}"));
            let kind = e["kind"]
                .as_str()
                .unwrap_or_else(|| panic!("entry without kind: {e}"));
            (path.to_owned(), kind.to_owned())
        })
        .collect();
    let count = |k: &str| {
        v[k].as_u64()
            .unwrap_or_else(|| panic!("`{k}` is not a non-negative integer: {v}"))
    };
    Listing {
        entries,
        ignored: count("ignored"),
        skipped: count("skipped"),
        too_deep: count("too_deep"),
        truncated: v["truncated"]
            .as_bool()
            .unwrap_or_else(|| panic!("`truncated` is not a boolean: {v}")),
    }
}

fn entries(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(p, k)| ((*p).to_owned(), (*k).to_owned()))
        .collect()
}

/// The `file` and `symlink` paths of a listing.
fn file_set(l: &Listing) -> BTreeSet<String> {
    l.entries
        .iter()
        .filter(|(_, k)| k == "file" || k == "symlink")
        .map(|(p, _)| p.clone())
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Fixtures.

/// A fresh temporary directory, removed on drop.
struct Work {
    dir: PathBuf,
}

impl Work {
    fn new(tag: &str) -> Work {
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "k19c-{}-{}-{tag}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Work {
            dir: fs::canonicalize(&dir).unwrap(),
        }
    }

    fn at(&self, rel: &str) -> PathBuf {
        self.dir.join(rel)
    }
}

impl Drop for Work {
    fn drop(&mut self) {
        if fs::remove_dir_all(&self.dir).is_err() {
            let mut rm = Command::new("rm");
            rm.arg("-rf").arg(&self.dir);
            let _ = spawn_with_timeout(rm, None, TIMEOUT);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Node {
    File(Vec<u8>),
    Dir,
    /// A symbolic link with this target text.
    Link(String),
}

/// Root-relative path → node. Every proper prefix of a path is a `Dir` (see [`normalize`]).
type Tree = BTreeMap<String, Node>;

fn parent_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(p, _)| p)
}

fn name_of(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, n)| n)
}

fn join(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_owned()
    } else {
        format!("{dir}/{name}")
    }
}

/// Add a `Dir` node for every proper prefix of every path.
fn normalize(t: &mut Tree) {
    let paths: Vec<String> = t.keys().cloned().collect();
    for p in paths {
        let mut dir = parent_of(&p).to_owned();
        while !dir.is_empty() {
            let prev = t.insert(dir.clone(), Node::Dir);
            assert!(
                matches!(prev, None | Some(Node::Dir)),
                "fixture bug: {dir} is both a directory and something else"
            );
            dir = parent_of(&dir).to_owned();
        }
    }
}

/// Build a tree from `(path, node)` pairs.
fn tree_of(items: &[(&str, Node)]) -> Tree {
    let mut t: Tree = items
        .iter()
        .map(|(p, n)| ((*p).to_owned(), n.clone()))
        .collect();
    normalize(&mut t);
    t
}

fn file(text: &str) -> Node {
    Node::File(text.as_bytes().to_vec())
}

fn link(target: &str) -> Node {
    Node::Link(target.to_owned())
}

/// Create `t` below `root` (which must not exist yet).
fn materialize(t: &Tree, root: &Path) {
    fs::create_dir_all(root).unwrap();
    for (p, n) in t {
        let at = root.join(p);
        match n {
            Node::Dir => fs::create_dir_all(&at).unwrap(),
            Node::File(bytes) => {
                fs::create_dir_all(at.parent().unwrap()).unwrap();
                fs::write(&at, bytes).unwrap();
            }
            Node::Link(_) => {}
        }
    }
    for (p, n) in t {
        if let Node::Link(target) = n {
            symlink(target, root.join(p)).unwrap();
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The `--memory-seed` manifest holding the files of `t` (directories are implied; a tree
/// for a manifest has no links).
fn manifest(t: &Tree) -> String {
    let files: Vec<Value> = t
        .iter()
        .filter_map(|(p, n)| match n {
            Node::File(bytes) => Some(json!({"path": p, "hex": hex(bytes)})),
            Node::Dir => None,
            Node::Link(_) => panic!("fixture bug: a manifest cannot hold the link {p}"),
        })
        .collect();
    json!({ "files": files }).to_string()
}

/// Every entry under `dir`, symlinks not followed and nothing opened except regular files:
/// type, link target, bytes.
fn snapshot(dir: &Path) -> BTreeMap<String, String> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            let rel = p.strip_prefix(base).unwrap().to_string_lossy().into_owned();
            let ft = fs::symlink_metadata(&p).unwrap().file_type();
            let desc = if ft.is_symlink() {
                format!("link -> {}", fs::read_link(&p).unwrap().display())
            } else if ft.is_dir() {
                walk(base, &p, out);
                "dir".to_owned()
            } else if ft.is_file() {
                format!("file {}", fs::read(&p).unwrap().escape_ascii())
            } else if ft.is_fifo() {
                "fifo".to_owned()
            } else {
                "other".to_owned()
            };
            out.insert(rel, desc);
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

fn mkfifo(path: &Path) {
    let mut cmd = Command::new("mkfifo");
    cmd.arg(path);
    let out = spawn_with_timeout(cmd, None, TIMEOUT);
    assert!(out.status.success(), "mkfifo {}", path.display());
    assert!(fs::symlink_metadata(path).unwrap().file_type().is_fifo());
}

// ---------------------------------------------------------------------------------------------
// git (AC-1's oracle).

fn git(args: &[&str], dir: &Path) -> Output {
    let mut cmd = Command::new("git");
    cmd.args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_CEILING_DIRECTORIES");
    let out = spawn_with_timeout(cmd, None, TIMEOUT);
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

/// `git init` in `root`, then the paths `git ls-files -z --cached --others --exclude-standard`
/// lists, minus those under the root's `target/`.
fn git_files(root: &Path) -> BTreeSet<String> {
    git(&["init", "-q"], root);
    let out = git(
        &[
            "-c",
            "core.excludesFile=",
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
        root,
    );
    out.stdout
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8(s.to_vec()).expect("git path is UTF-8"))
        .filter(|p| !p.starts_with("target/"))
        .collect()
}

// ---------------------------------------------------------------------------------------------
// The naive model of the walk rules (rules 1–5 from the root, Pinned (c)).

#[derive(Debug)]
enum PatKind {
    /// Matches the entry's name (`name`, `b/`, `!c`, `**/d`).
    Name(String),
    /// `*<suffix>`: the entry's name ends with the suffix (`*` matches any run, also empty).
    Suffix(String),
    /// `/a`: the path relative to the `.gitignore`'s directory is exactly this.
    Anchored(String),
    /// `e/**`: the relative path is strictly inside this directory.
    Inside(String),
}

#[derive(Debug)]
struct Pat {
    neg: bool,
    dir_only: bool,
    kind: PatKind,
}

/// Parse a `.gitignore` written with the pattern forms of S5 only (and `#` comments).
fn parse_gitignore(bytes: &[u8]) -> Vec<Pat> {
    let text = std::str::from_utf8(bytes).expect("fixture .gitignore is UTF-8");
    let mut pats = Vec::new();
    for line in text.split('\n') {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (neg, rest) = match line.strip_prefix('!') {
            Some(r) => (true, r),
            None => (false, line),
        };
        let (dir_only, rest) = match rest.strip_suffix('/') {
            Some(r) => (true, r),
            None => (false, rest),
        };
        let kind = if let Some(r) = rest.strip_prefix("**/") {
            PatKind::Name(r.to_owned())
        } else if let Some(r) = rest.strip_suffix("/**") {
            PatKind::Inside(r.to_owned())
        } else if let Some(r) = rest.strip_prefix('/') {
            PatKind::Anchored(r.to_owned())
        } else if let Some(r) = rest.strip_prefix('*') {
            PatKind::Suffix(r.to_owned())
        } else {
            PatKind::Name(rest.to_owned())
        };
        let (PatKind::Name(s) | PatKind::Suffix(s) | PatKind::Anchored(s) | PatKind::Inside(s)) =
            &kind;
        assert!(
            !s.is_empty() && !s.contains(['*', '?', '[', '/', '\\', '{', '}', ' ', '\t']),
            "fixture bug: pattern {line:?} is outside the forms of S5"
        );
        pats.push(Pat {
            neg,
            dir_only,
            kind,
        });
    }
    pats
}

/// git's decision for `path`: the last matching line of the deepest `.gitignore` that has one.
fn model_ignored(stack: &[(String, Vec<Pat>)], path: &str, is_dir: bool) -> bool {
    let name = name_of(path);
    for (base, pats) in stack.iter().rev() {
        let rel = if base.is_empty() {
            path
        } else {
            path.strip_prefix(&format!("{base}/"))
                .expect("model bug: .gitignore not on the path")
        };
        for p in pats.iter().rev() {
            if p.dir_only && !is_dir {
                continue;
            }
            let hit = match &p.kind {
                PatKind::Name(s) => name == s,
                PatKind::Suffix(s) => name.ends_with(s.as_str()),
                PatKind::Anchored(s) => rel == s,
                PatKind::Inside(s) => rel.starts_with(&format!("{s}/")),
            };
            if hit {
                return !p.neg;
            }
        }
    }
    false
}

#[derive(Debug, Default)]
struct Model {
    entries: Vec<(String, String)>,
    ignored: u64,
    too_deep: u64,
}

fn kind_name(n: &Node) -> &'static str {
    match n {
        Node::File(_) => "file",
        Node::Dir => "dir",
        Node::Link(_) => "symlink",
    }
}

/// The walk of `project.tree {}` over `t`, from the root.
fn model(t: &Tree) -> Model {
    let mut kids: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for p in t.keys() {
        kids.entry(parent_of(p)).or_default().push(name_of(p));
    }
    for v in kids.values_mut() {
        v.sort_unstable_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    }
    fn walk(
        t: &Tree,
        kids: &BTreeMap<&str, Vec<&str>>,
        dir: &str,
        stack: &mut Vec<(String, Vec<Pat>)>,
        m: &mut Model,
    ) {
        let names = kids.get(dir).cloned().unwrap_or_default();
        let gi = join(dir, ".gitignore");
        let pushed = if let Some(Node::File(bytes)) = t.get(&gi) {
            stack.push((dir.to_owned(), parse_gitignore(bytes)));
            true
        } else {
            false
        };
        for name in names {
            let path = join(dir, name);
            let node = &t[&path];
            let is_dir = *node == Node::Dir;
            if (path == "target" && is_dir) || name == ".git" {
                m.ignored += 1;
                continue;
            }
            if model_ignored(stack, &path, is_dir) {
                m.ignored += 1;
                continue;
            }
            m.entries.push((path.clone(), kind_name(node).to_owned()));
            if is_dir {
                if path.split('/').count() == 128 {
                    m.too_deep += 1;
                } else {
                    walk(t, kids, &path, stack, m);
                }
            }
        }
        if pushed {
            stack.pop();
        }
    }
    let mut m = Model::default();
    walk(t, &kids, "", &mut Vec::new(), &mut m);
    m
}

// ---------------------------------------------------------------------------------------------
// Random trees (fixed seeds; AC-1's generator limits).

/// SplitMix64.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }

    fn pick<'a, T>(&mut self, v: &'a [T]) -> &'a T {
        &v[self.below(v.len() as u64) as usize]
    }
}

const STEM: &[u8] = b"abcde0123_-";
const EXT: &[u8] = b"abcde0";

/// A name from `[a-e0-9._-]`: never `.`, `..`, `.git`, `.gitignore` or `target`.
fn random_name(r: &mut Rng) -> String {
    let mut s = String::new();
    if r.chance(15) {
        s.push('.');
    }
    for _ in 0..=r.below(2) {
        s.push(*r.pick(STEM) as char);
    }
    if r.chance(50) {
        s.push('.');
        s.push(*r.pick(EXT) as char);
    }
    s
}

fn fresh_name(r: &mut Rng, t: &Tree, dir: &str) -> String {
    loop {
        let n = random_name(r);
        if !t.contains_key(&join(dir, &n)) {
            return n;
        }
    }
}

fn gen_dir(r: &mut Rng, t: &mut Tree, dir: &str, depth: u32) {
    let n = if depth == 0 {
        1 + r.below(6)
    } else {
        r.below(5)
    };
    for _ in 0..n {
        let name = fresh_name(r, t, dir);
        let path = join(dir, &name);
        if depth < 3 && r.chance(35) {
            t.insert(path.clone(), Node::Dir);
            gen_dir(r, t, &path, depth + 1);
        } else {
            t.insert(path.clone(), Node::File(format!("{path}\n").into_bytes()));
        }
    }
}

/// The directories of `t` (with the root as ""), outside the root's `target`.
fn dirs_of(t: &Tree) -> Vec<String> {
    let mut v = vec![String::new()];
    v.extend(
        t.iter()
            .filter(|(p, n)| **n == Node::Dir && *p != "target" && !p.starts_with("target/"))
            .map(|(p, _)| p.clone()),
    );
    v
}

fn under<'a>(t: &'a Tree, dir: &str) -> impl Iterator<Item = (&'a String, &'a Node)> + 'a {
    let prefix = if dir.is_empty() {
        String::new()
    } else {
        format!("{dir}/")
    };
    t.iter().filter(move |(p, _)| p.starts_with(&prefix))
}

fn random_pattern(r: &mut Rng, t: &Tree, dir: &str) -> String {
    let in_subtree: Vec<String> = under(t, dir).map(|(p, _)| name_of(p).to_owned()).collect();
    let dirs_below: Vec<String> = under(t, dir)
        .filter(|(_, n)| **n == Node::Dir)
        .map(|(p, _)| name_of(p).to_owned())
        .collect();
    let children: Vec<String> = under(t, dir)
        .filter(|(p, _)| parent_of(p) == dir)
        .map(|(p, _)| name_of(p).to_owned())
        .collect();
    let child_dirs: Vec<String> = under(t, dir)
        .filter(|(p, n)| parent_of(p) == dir && **n == Node::Dir)
        .map(|(p, _)| name_of(p).to_owned())
        .collect();
    let from = |r: &mut Rng, v: &[String]| {
        if v.is_empty() || r.chance(15) {
            random_name(r)
        } else {
            r.pick(v).clone()
        }
    };
    match r.below(20) {
        0..=3 => format!("*.{}", *r.pick(EXT) as char),
        4..=6 => format!("/{}", from(r, &children)),
        7..=9 => format!("{}/", from(r, &dirs_below)),
        10..=12 => format!("!{}", from(r, &in_subtree)),
        13..=15 => format!("**/{}", from(r, &in_subtree)),
        16..=17 => format!("{}/**", from(r, &child_dirs)),
        // Q-8 (D-097): patterns that match the name `.gitignore` itself.
        18 => (*r.pick(&["*.gitignore", "/.gitignore", "**/.gitignore", "!.gitignore"])).to_owned(),
        _ => "#c".to_owned(),
    }
}

/// A random tree. `links`: add symlinks to entries inside the tree. `fill`: every directory
/// holds at least one file (so the tree is the same in memory and on disk).
fn random_tree(seed: u64, links: bool, fill: bool) -> Tree {
    let mut r = Rng(seed.wrapping_mul(0x2545_f491_4f6c_dd1d) ^ 0x0019);
    let mut t = Tree::new();
    gen_dir(&mut r, &mut t, "", 0);
    if r.chance(30) {
        t.insert("target".to_owned(), Node::Dir);
        t.insert("target/k.a".to_owned(), file("built\n"));
        if r.chance(50) {
            t.insert("target/d/x".to_owned(), file("built\n"));
        }
    }
    let inner: Vec<String> = dirs_of(&t).into_iter().filter(|d| !d.is_empty()).collect();
    if !inner.is_empty() && r.chance(25) {
        // A `target` directory below the root is not special (rule 2).
        let d = r.pick(&inner).clone();
        t.insert(join(&d, "target/t0"), file("nested target\n"));
    }
    normalize(&mut t);
    if links {
        for _ in 0..r.below(4) {
            let dirs = dirs_of(&t);
            let at = r.pick(&dirs).clone();
            let targets: Vec<String> = t
                .iter()
                .filter(|(_, n)| !matches!(n, Node::Link(_)))
                .map(|(p, _)| p.clone())
                .collect();
            let to = r.pick(&targets).clone();
            let ups = if at.is_empty() {
                0
            } else {
                at.split('/').count()
            };
            let name = fresh_name(&mut r, &t, &at);
            t.insert(
                join(&at, &name),
                Node::Link(format!("{}{to}", "../".repeat(ups))),
            );
        }
    }
    if fill {
        let empty: Vec<String> = dirs_of(&t)
            .into_iter()
            .chain(t.keys().filter(|p| p.starts_with("target")).cloned())
            .filter(|d| {
                !d.is_empty()
                    && t.get(d) == Some(&Node::Dir)
                    && !under(&t, d).any(|(_, n)| matches!(n, Node::File(_)))
            })
            .collect();
        for d in empty {
            t.insert(join(&d, "f0"), file("filler\n"));
        }
    }
    for d in dirs_of(&t) {
        if r.chance(45) {
            let n = 1 + r.below(4);
            let lines: Vec<String> = (0..n).map(|_| random_pattern(&mut r, &t, &d)).collect();
            t.insert(
                join(&d, ".gitignore"),
                Node::File(format!("{}\n", lines.join("\n")).into_bytes()),
            );
        }
    }
    normalize(&mut t);
    t
}

// ---------------------------------------------------------------------------------------------
// Hand-built trees.

/// Pinned (g): the worked example.
fn worked_example() -> Tree {
    tree_of(&[
        (".gitignore", file("*.log\n/target\n")),
        ("B.txt", file("B\n")),
        ("a.rs", file("fn a() {}\n")),
        ("a/b.rs", file("fn b() {}\n")),
        ("x.log", file("log\n")),
        ("target/debug/k", file("k\n")),
        ("src/lib.rs", file("// lib\n")),
        ("src/.hidden", file("h\n")),
    ])
}

fn worked_example_entries() -> Vec<(String, String)> {
    entries(&[
        (".gitignore", "file"),
        ("B.txt", "file"),
        ("a", "dir"),
        ("a/b.rs", "file"),
        ("a.rs", "file"),
        ("src", "dir"),
        ("src/.hidden", "file"),
        ("src/lib.rs", "file"),
    ])
}

/// At least ten hand-built trees inside AC-1's generator limits.
fn hand_trees() -> Vec<(&'static str, Tree)> {
    vec![
        ("worked-example", worked_example()),
        (
            "negation",
            tree_of(&[
                (".gitignore", file("*.e\n!keep.e\n")),
                ("a.e", file("1")),
                ("keep.e", file("2")),
                ("s/keep.e", file("3")),
                ("s/x.e", file("4")),
                ("s/y.c", file("5")),
            ]),
        ),
        (
            "dir-only",
            tree_of(&[
                (".gitignore", file("b/\n")),
                ("b/in.c", file("1")),
                ("s/b", file("a file named b")),
                ("s/t/b/deep.c", file("3")),
                ("c", file("4")),
            ]),
        ),
        (
            "anchored",
            tree_of(&[
                (".gitignore", file("/a\n")),
                ("a/x.c", file("1")),
                ("s/a", file("2")),
                ("s/.gitignore", file("/b.c\n")),
                ("s/b.c", file("3")),
                ("s/t/b.c", file("4")),
                ("b.c", file("5")),
            ]),
        ),
        (
            "double-star-prefix",
            tree_of(&[
                ("s/.gitignore", file("**/d\n")),
                ("d", file("top d, not under s")),
                ("s/d", file("2")),
                ("s/t/d/x", file("3")),
                ("s/t/e", file("4")),
            ]),
        ),
        (
            "double-star-suffix",
            tree_of(&[
                (".gitignore", file("e/**\n!x\n")),
                ("e/x", file("re-included: e itself is not excluded")),
                ("e/y", file("2")),
                ("e/f/x", file("e/f is excluded, so not re-included")),
                ("e2", file("4")),
                ("s/e/z", file("5")),
            ]),
        ),
        (
            "deeper-overrides",
            tree_of(&[
                (".gitignore", file("*.a\n")),
                ("z.a", file("1")),
                ("s/.gitignore", file("!z.a\n")),
                ("s/z.a", file("2")),
                ("s/y.a", file("3")),
                ("s/t/z.a", file("4")),
                ("s/t/.gitignore", file("/z.a\n")),
            ]),
        ),
        (
            "no-reinclude-under-ignored-dir",
            tree_of(&[
                (".gitignore", file("b/\n!c.e\n")),
                ("b/c.e", file("1")),
                ("b/.gitignore", file("!c.e\n")),
                ("c.e", file("2")),
            ]),
        ),
        (
            "symlinks-inside",
            tree_of(&[
                (".gitignore", file("b/\n*.c\n")),
                ("d/x.e", file("1")),
                ("d/y.c", file("2")),
                ("lf", link("d/x.e")),
                ("ld", link("d")),
                ("b", link("d")),
                ("d/up", link("..")),
                ("d/sib", link("../lf")),
                ("lc.c", link("d/x.e")),
                ("target", link("d")),
            ]),
        ),
        (
            "q8-ignored-gitignore-still-read",
            tree_of(&[
                (".gitignore", file("/.gitignore\n*.d\n")),
                ("top.e", file("1")),
                ("k.d", file("2")),
                ("sub/.gitignore", file("*.gitignore\n**/s.e\n")),
                ("sub/s.e", file("3")),
                ("sub/keep.e", file("4")),
                ("sub/deep/.gitignore", file("!s.e\n")),
                ("sub/deep/s.e", file("5")),
                ("sub/deep/t.d", file("6")),
            ]),
        ),
        (
            "targets",
            tree_of(&[
                ("target/debug/k", file("1")),
                ("target/.gitignore", file("!k\n")),
                ("s/target/k", file("2")),
                ("s/.gitignore", file("**/q\n")),
                ("s/q", file("3")),
                ("x", file("4")),
            ]),
        ),
        (
            "target-is-a-file",
            tree_of(&[("target", file("not a directory\n")), ("a", file("1"))]),
        ),
        (
            "hidden-and-comments",
            tree_of(&[
                (".gitignore", file("# comment\n*.c\n\n!.c\n")),
                (".c", file("1")),
                ("a.c", file("2")),
                (".b/.c", file("3")),
                (".b/x.c", file("4")),
                ("e1/e2", Node::Dir),
                ("only-ignored/z.c", file("5")),
            ]),
        ),
    ]
}

// ---------------------------------------------------------------------------------------------
// AC-1

/// Model, git and Kiln on one tree (`root` must not exist yet; `git init` is run in it).
fn check_against_git(name: &str, t: &Tree, root: &Path) {
    materialize(t, root);
    let m = model(t);
    let git_set = git_files(root);
    let model_set: BTreeSet<String> = m
        .entries
        .iter()
        .filter(|(_, k)| k == "file" || k == "symlink")
        .map(|(p, _)| p.clone())
        .collect();
    assert_eq!(
        model_set, git_set,
        "TEST BUG: the test's naive model disagrees with git on tree {name}: {t:?}"
    );
    let cwd = root.parent().unwrap();
    let root_arg = root.to_str().unwrap();
    let got = listing(&tree_ok(cwd, &json!({}), &["--root", root_arg], TIMEOUT));
    assert_eq!(
        file_set(&got),
        git_set,
        "tree {name}: file/symlink entries differ from git ls-files: {t:?}"
    );
}

#[test]
fn tree_equals_git_ls_files() {
    let w = Work::new("ac1");
    for (i, (name, t)) in hand_trees().iter().enumerate() {
        check_against_git(name, t, &w.at(&format!("h{i}")));
    }
    for seed in 0..RANDOM_TREES {
        let t = random_tree(seed, true, false);
        check_against_git(&format!("seed {seed}"), &t, &w.at(&format!("r{seed}")));
        // Keep the disk use small.
        fs::remove_dir_all(w.at(&format!("r{seed}"))).unwrap();
    }
}

// ---------------------------------------------------------------------------------------------
// AC-2

/// Kiln's listing equals the model's: entries in preorder with byte-sorted siblings, and the
/// counts.
fn check_against_model(name: &str, t: &Tree, root: &Path) {
    materialize(t, root);
    let m = model(t);
    let got = listing(&tree_ok(
        root.parent().unwrap(),
        &json!({}),
        &["--root", root.to_str().unwrap()],
        TIMEOUT,
    ));
    assert_eq!(got.entries, m.entries, "tree {name}: entries: {t:?}");
    assert_eq!(got.ignored, m.ignored, "tree {name}: ignored: {t:?}");
    assert_eq!(got.too_deep, m.too_deep, "tree {name}: too_deep");
    assert_eq!(got.skipped, 0, "tree {name}: skipped");
    assert!(!got.truncated, "tree {name}: truncated");
}

#[test]
fn tree_order_and_counts() {
    let w = Work::new("ac2");
    // Pinned (g), exactly.
    materialize(&worked_example(), &w.at("ex"));
    let expected = Listing {
        entries: worked_example_entries(),
        ignored: 2,
        skipped: 0,
        too_deep: 0,
        truncated: false,
    };
    let got = tree_ok(&w.dir, &json!({}), &["--root", "ex"], TIMEOUT);
    assert_eq!(listing(&got), expected);
    // The same through `run`: one `ok` response carrying the same result.
    fs::write(
        w.at("s.jsonl"),
        format!(
            "{}\n",
            json!({"id": 1, "command": "project.tree", "params": {}})
        ),
    )
    .unwrap();
    let out = kiln(&["run", "s.jsonl", "--root", "ex"], &w.dir, None, TIMEOUT);
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<Value> = stdout
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 1, "{stdout}");
    assert_eq!(lines[0]["id"], json!(1), "{stdout}");
    assert_eq!(lines[0]["ok"], json!(true), "{stdout}");
    assert_eq!(lines[0]["result"], got, "{stdout}");
    assert_eq!(out.status.code(), Some(0));

    // Rule 2 for `.git` of every kind, at any depth; a dot name sorts by its bytes.
    let dot_git = tree_of(&[
        (".git/HEAD", file("ref\n")),
        ("s/.git", file("gitdir: elsewhere\n")),
        ("s/.gitx", file("listed\n")),
        ("s/t/.git", link("..")),
        ("s/t/-", file("1")),
        ("s/t/_", file("2")),
        ("s/t/Z", file("3")),
        ("s/t/\u{e9}", file("4")),
        ("s/t/z", file("5")),
    ]);
    check_against_model("dot-git", &dot_git, &w.at("dg"));
    let got = listing(&tree_ok(&w.dir, &json!({}), &["--root", "dg"], TIMEOUT));
    assert_eq!(got.ignored, 3, "{got:?}");
    assert_eq!(
        got.entries,
        entries(&[
            ("s", "dir"),
            ("s/.gitx", "file"),
            ("s/t", "dir"),
            ("s/t/-", "file"),
            ("s/t/Z", "file"),
            ("s/t/_", "file"),
            ("s/t/z", "file"),
            ("s/t/\u{e9}", "file"),
        ])
    );

    for (i, (name, t)) in hand_trees().iter().enumerate() {
        check_against_model(name, t, &w.at(&format!("h{i}")));
    }
    for seed in 0..RANDOM_TREES {
        let t = random_tree(seed, true, false);
        let root = w.at(&format!("r{seed}"));
        check_against_model(&format!("seed {seed}"), &t, &root);
        fs::remove_dir_all(root).unwrap();
    }
}

// ---------------------------------------------------------------------------------------------
// AC-3

/// The results of `params` on the memory seed and on the disk root are identical (both `ok`
/// with equal results, or both refused with the same `code` and `kind`).
fn memory_equals_disk(w: &Work, name: &str, seed: &str, root: &str, params: &Value) -> Value {
    let mem = tree_exec(&w.dir, params, &["--memory-seed", seed], TIMEOUT);
    let disk = tree_exec(&w.dir, params, &["--root", root], TIMEOUT);
    match (&mem, &disk) {
        (Ok(m), Ok(d)) => {
            assert_eq!(m, d, "tree {name}, params {params}: memory vs disk");
            m.clone()
        }
        (Err(m), Err(d)) => {
            assert_eq!(
                (&m["code"], &m["kind"]),
                (&d["code"], &d["kind"]),
                "tree {name}, params {params}: memory vs disk refusals"
            );
            assert_eq!(
                m["code"],
                json!("rejected"),
                "tree {name}, params {params}: {m}"
            );
            Value::Null
        }
        _ => panic!("tree {name}, params {params}: memory {mem:?} vs disk {disk:?}"),
    }
}

#[test]
fn memory_and_disk_trees_agree() {
    let w = Work::new("ac3");
    // The worked example with empty directories added on disk only: equal apart from them.
    let t = worked_example();
    materialize(&t, &w.at("ex"));
    fs::write(w.at("ex.json"), manifest(&t)).unwrap();
    for params in [
        json!({}),
        json!({"limit": 3}),
        json!({"path": "a"}),
        json!({"path": "src"}),
        json!({"path": "target"}),
        json!({"path": "a.rs"}),
        json!({"path": "nope"}),
        json!({"path": "a.rs/x"}),
    ] {
        memory_equals_disk(&w, "worked example", "ex.json", "ex", &params);
    }
    let mem = listing(&tree_ok(
        &w.dir,
        &json!({}),
        &["--memory-seed", "ex.json"],
        TIMEOUT,
    ));
    assert_eq!(mem.entries, worked_example_entries());
    fs::create_dir(w.at("ex/e1")).unwrap();
    fs::create_dir_all(w.at("ex/src/e2/e3")).unwrap();
    let disk = listing(&tree_ok(&w.dir, &json!({}), &["--root", "ex"], TIMEOUT));
    let empty = ["e1", "src/e2", "src/e2/e3"];
    for e in empty {
        assert!(
            disk.entries.contains(&(e.to_owned(), "dir".to_owned())),
            "{e} missing on disk: {disk:?}"
        );
    }
    let mut without = disk.clone();
    without
        .entries
        .retain(|(p, _)| !empty.contains(&p.as_str()));
    assert_eq!(without, mem);

    for seed in 0..RANDOM_TREES {
        let t = random_tree(seed, false, true);
        let root = format!("r{seed}");
        let seed_file = format!("r{seed}.json");
        materialize(&t, &w.at(&root));
        fs::write(w.at(&seed_file), manifest(&t)).unwrap();
        let name = format!("seed {seed}");
        let full = memory_equals_disk(&w, &name, &seed_file, &root, &json!({}));
        assert_eq!(listing(&full).entries, model(&t).entries, "tree {name}");
        memory_equals_disk(&w, &name, &seed_file, &root, &json!({"limit": 4}));
        fs::remove_dir_all(w.at(&root)).unwrap();
        fs::remove_file(w.at(&seed_file)).unwrap();
    }
}

// ---------------------------------------------------------------------------------------------
// AC-4

#[test]
fn symlinks_are_listed_not_followed() {
    let w = Work::new("ac4");
    fs::create_dir_all(w.at("root")).unwrap();
    fs::create_dir_all(w.at("outside")).unwrap();
    fs::write(w.at("outside/secret.txt"), b"s3cr3t\n").unwrap();
    fs::write(w.at("outside/.gitignore"), b"*\n").unwrap();
    fs::write(w.at("outside-ignore"), b"*\n").unwrap();
    symlink("../outside", w.at("root/out")).unwrap();
    symlink(".", w.at("root/self")).unwrap();
    symlink("loop2", w.at("root/loop1")).unwrap();
    symlink("loop1", w.at("root/loop2")).unwrap();
    mkfifo(&w.at("root/p"));
    symlink("../outside-ignore", w.at("root/.gitignore")).unwrap();
    fs::write(w.at("root/keep.rs"), b"fn keep() {}\n").unwrap();
    // The outside files exist and the links reach them.
    assert_eq!(fs::read(w.at("root/out/secret.txt")).unwrap(), b"s3cr3t\n");
    assert_eq!(fs::read(w.at("root/out/.gitignore")).unwrap(), b"*\n");
    assert_eq!(fs::read(w.at("root/.gitignore")).unwrap(), b"*\n");
    assert!(w.at("root/self/keep.rs").is_file());
    let before = snapshot(&w.dir);

    let kinds = [
        (".gitignore", "symlink"),
        ("keep.rs", "file"),
        ("loop1", "symlink"),
        ("loop2", "symlink"),
        ("out", "symlink"),
        ("p", "other"),
        ("self", "symlink"),
    ];
    let expected = Listing {
        entries: entries(&kinds),
        ignored: 0,
        skipped: 0,
        too_deep: 0,
        truncated: false,
    };
    let top = tree_ok(&w.dir, &json!({}), &["--root", "root"], HANG);
    assert_eq!(listing(&top), expected);

    match tree_exec(&w.dir, &json!({"path": "out"}), &["--root", "root"], HANG) {
        Ok(v) => panic!("project.tree out listed the outside: {v}"),
        Err(e) => {
            assert_eq!(e["code"], json!("rejected"), "{e}");
            assert_eq!(e["kind"], json!("outside_root"), "{e}");
        }
    }
    match tree_exec(&w.dir, &json!({"path": "p"}), &["--root", "root"], HANG) {
        Ok(v) => panic!("project.tree p (a FIFO) succeeded: {v}"),
        Err(e) => {
            assert_eq!(e["code"], json!("rejected"), "{e}");
            assert_eq!(e["kind"], json!("not_a_file"), "{e}");
        }
    }

    let prefixed: Vec<(String, String)> = kinds
        .iter()
        .map(|(p, k)| (format!("self/{p}"), (*k).to_owned()))
        .collect();
    let via_self = tree_ok(&w.dir, &json!({"path": "self"}), &["--root", "root"], HANG);
    assert_eq!(
        listing(&via_self),
        Listing {
            entries: prefixed,
            ..expected.clone()
        }
    );

    // The same through `control`: the refusal leaves the session usable and unchanged.
    let input: String = [
        json!({"id": 1, "command": "project.tree", "params": {"path": "out"}}),
        json!({"id": 2, "command": "project.tree", "params": {}}),
        json!({"id": 3, "command": "project.tree", "params": {"path": "self"}}),
    ]
    .iter()
    .map(|r| format!("{r}\n"))
    .collect();
    let out = kiln(
        &["control", "--root", "root"],
        &w.dir,
        Some(input.as_bytes()),
        HANG,
    );
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    let got: Vec<Value> = stdout
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(got.len(), 3, "{stdout}");
    assert_eq!(got[0]["id"], json!(1));
    assert_eq!(got[0]["ok"], json!(false), "{stdout}");
    assert_eq!(got[0]["error"]["code"], json!("rejected"), "{stdout}");
    assert_eq!(got[0]["error"]["kind"], json!("outside_root"), "{stdout}");
    assert_eq!(got[1]["ok"], json!(true), "{stdout}");
    assert_eq!(got[1]["result"], top, "{stdout}");
    assert_eq!(got[2]["ok"], json!(true), "{stdout}");
    assert_eq!(got[2]["result"], via_self, "{stdout}");

    // Nothing was read through, written or changed; the outside files are byte-identical.
    assert_eq!(snapshot(&w.dir), before);
    assert_eq!(fs::read(w.at("outside/secret.txt")).unwrap(), b"s3cr3t\n");
    assert_eq!(fs::read(w.at("outside/.gitignore")).unwrap(), b"*\n");
    assert_eq!(fs::read(w.at("outside-ignore")).unwrap(), b"*\n");
    let stdout_all = format!("{top}{via_self}{stdout}");
    assert!(!stdout_all.contains("secret.txt"), "the outside is listed");
    assert!(
        !stdout_all.contains(w.dir.to_str().unwrap()),
        "a host path is reported"
    );
}

// ---------------------------------------------------------------------------------------------
// AC-5

#[test]
fn limits_and_refusals() {
    let w = Work::new("ac5");
    materialize(&worked_example(), &w.at("ex"));
    let all = worked_example_entries();
    let src = ["--root", "ex"];
    let run = |params: Value| listing(&tree_ok(&w.dir, &params, &src, TIMEOUT));

    let l = run(json!({"limit": 3}));
    assert_eq!(
        l,
        Listing {
            entries: all[..3].to_vec(),
            ignored: 0,
            skipped: 0,
            too_deep: 0,
            truncated: true,
        }
    );
    let l = run(json!({"limit": 7}));
    assert_eq!((l.entries.as_slice(), l.truncated), (&all[..7], true));
    for limit in [8, 9, 1_000_000] {
        let l = run(json!({ "limit": limit }));
        assert_eq!(
            l,
            Listing {
                entries: all.clone(),
                ignored: 2,
                skipped: 0,
                too_deep: 0,
                truncated: false,
            },
            "limit {limit}"
        );
    }
    let l = run(json!({"limit": 1}));
    assert_eq!((l.entries.as_slice(), l.truncated), (&all[..1], true));

    // `path`: located, not listed itself; entries begin with it.
    let l = run(json!({"path": "a"}));
    assert_eq!(
        l,
        Listing {
            entries: entries(&[("a/b.rs", "file")]),
            ignored: 0,
            skipped: 0,
            too_deep: 0,
            truncated: false,
        }
    );
    let l = run(json!({"path": "src"}));
    assert_eq!(
        l.entries,
        entries(&[("src/.hidden", "file"), ("src/lib.rs", "file")])
    );
    // A named `target` (and a named ignored directory) is listed (rule 1, Q-3 (iii)).
    let l = run(json!({"path": "target"}));
    assert_eq!(
        (l.entries, l.ignored),
        (
            entries(&[("target/debug", "dir"), ("target/debug/k", "file")]),
            0
        )
    );

    // Refusals: code and kind only.
    let r = |params: Value, code: &str, kind: &str| tree_refused(&w.dir, &params, &src, code, kind);
    r(json!({"path": "a.rs"}), "rejected", "not_a_file");
    r(json!({"path": "nope"}), "rejected", "not_found");
    r(json!({"path": "../x"}), "rejected", "invalid_path");
    r(json!({"path": "/etc"}), "rejected", "invalid_path");
    r(json!({"path": ""}), "rejected", "invalid_path");
    r(json!({"limit": 0}), "invalid_params", "schema");
    r(json!({"limit": 1_000_001}), "invalid_params", "schema");
    r(json!({"limit": -1}), "invalid_params", "schema");
    r(json!({"limit": "3"}), "invalid_params", "schema");
    r(json!({"path": 3}), "invalid_params", "schema");
    r(json!({"depth": 1}), "invalid_params", "schema");

    // Counts cover what the walk met before it stopped (Q-7).
    let t = tree_of(&[
        (".gitignore", file("*.c\n")),
        ("a.c", file("1")),
        ("b.d", file("2")),
        ("c.d", file("3")),
    ]);
    materialize(&t, &w.at("met"));
    let met = |limit: u64| {
        listing(&tree_ok(
            &w.dir,
            &json!({ "limit": limit }),
            &["--root", "met"],
            TIMEOUT,
        ))
    };
    let l = met(1);
    assert_eq!(
        (l.entries, l.ignored, l.truncated),
        (entries(&[(".gitignore", "file")]), 1, true)
    );
    let l = met(2);
    assert_eq!(
        (l.entries, l.ignored, l.truncated),
        (entries(&[(".gitignore", "file"), ("b.d", "file")]), 1, true)
    );
    let l = met(3);
    assert_eq!((l.entries.len(), l.ignored, l.truncated), (3, 1, false));

    let t = tree_of(&[
        (".gitignore", file("*.c\n")),
        ("a.d", file("1")),
        ("b.d", file("2")),
        ("z.c", file("3")),
    ]);
    materialize(&t, &w.at("late"));
    let late = |limit: u64| {
        listing(&tree_ok(
            &w.dir,
            &json!({ "limit": limit }),
            &["--root", "late"],
            TIMEOUT,
        ))
    };
    let l = late(1);
    assert_eq!(
        (l.entries, l.ignored, l.truncated),
        (entries(&[(".gitignore", "file")]), 0, true)
    );
    // Exactly `limit` entries and nothing more to list: not truncated (an ignored entry after
    // the last one does not count as an entry beyond the limit).
    let l = late(3);
    assert_eq!((l.entries.len(), l.ignored, l.truncated), (3, 1, false));
}

/// Q-3 (ii), (iii): the `.gitignore` files of the root and of the `path`'s prefix apply; the
/// `path` itself is not listed and not subject to the skip and ignore rules.
#[test]
fn tree_under_a_path_applies_prefix_gitignores() {
    let w = Work::new("ac5p");
    let t = tree_of(&[
        (".gitignore", file("*.e\nd/\n")),
        ("s/a.e", file("1")),
        ("s/b.c", file("2")),
        ("s/t/.gitignore", file("!a.e\n")),
        ("s/t/a.e", file("3")),
        ("s/t/u/.gitignore", file("*.c\n")),
        ("s/t/u/v.c", file("4")),
        ("s/t/u/w.d", file("5")),
        ("d/x.c", file("6")),
        ("d/d/y.c", file("7")),
        (".git/HEAD", file("8")),
    ]);
    materialize(&t, &w.at("p"));
    let run = |params: Value| listing(&tree_ok(&w.dir, &params, &["--root", "p"], TIMEOUT));
    let l = run(json!({"path": "s"}));
    assert_eq!(
        (l.entries, l.ignored),
        (
            entries(&[
                ("s/b.c", "file"),
                ("s/t", "dir"),
                ("s/t/.gitignore", "file"),
                ("s/t/a.e", "file"),
                ("s/t/u", "dir"),
                ("s/t/u/.gitignore", "file"),
                ("s/t/u/w.d", "file"),
            ]),
            2
        )
    );
    let l = run(json!({"path": "s/t/u"}));
    assert_eq!(
        (l.entries, l.ignored),
        (
            entries(&[("s/t/u/.gitignore", "file"), ("s/t/u/w.d", "file")]),
            1
        )
    );
    let l = run(json!({"path": "d"}));
    assert_eq!((l.entries, l.ignored), (entries(&[("d/x.c", "file")]), 1));
    let l = run(json!({"path": ".git"}));
    assert_eq!(
        (l.entries, l.ignored),
        (entries(&[(".git/HEAD", "file")]), 0)
    );
}

// ---------------------------------------------------------------------------------------------
// AC-6

fn chain(n: usize) -> String {
    vec!["d"; n].join("/")
}

#[test]
fn deep_trees_stop_at_the_depth_bound() {
    let w = Work::new("ac6");
    let bound: Vec<(String, String)> = (1..=128).map(|i| (chain(i), "dir".to_owned())).collect();
    let expected = Listing {
        entries: bound,
        ignored: 0,
        skipped: 0,
        too_deep: 1,
        truncated: false,
    };

    // 130 nested directories with a file at the bottom.
    let bottom = format!("{}/f.rs", chain(130));
    fs::create_dir_all(w.at(&format!("c130/{}", chain(130)))).unwrap();
    fs::write(w.at(&format!("c130/{bottom}")), b"fn deep() {}\n").unwrap();
    let l = listing(&tree_ok(&w.dir, &json!({}), &["--root", "c130"], TIMEOUT));
    assert!(
        l.entries.iter().all(|(_, k)| k == "dir"),
        "a file entry is listed"
    );
    assert_eq!(l, expected);
    // The same in memory.
    fs::write(
        w.at("c130.json"),
        json!({"files": [{"path": bottom, "text": "fn deep() {}\n"}]}).to_string(),
    )
    .unwrap();
    let l = listing(&tree_ok(
        &w.dir,
        &json!({}),
        &["--memory-seed", "c130.json"],
        TIMEOUT,
    ));
    assert_eq!(l, expected);

    // D-100 y4, on disk and in memory. (ii) The depth rule comes before the limit rule
    // (Pinned (c)): with `limit: 127` the directory at the bound is the entry that makes
    // `truncated` true, and it is counted in `too_deep`; with `limit: 128` all 128 are listed
    // and nothing lies beyond. (i) A `path` with 127 segments lists its 128-segment child and
    // does not descend it; a `path` with 128 segments descends no directory.
    for source in [["--root", "c130"], ["--memory-seed", "c130.json"]] {
        let run = |params: Value| listing(&tree_ok(&w.dir, &params, &source, TIMEOUT));
        assert_eq!(
            run(json!({"limit": 127})),
            Listing {
                entries: expected.entries[..127].to_vec(),
                truncated: true,
                ..expected.clone()
            },
            "{source:?}"
        );
        assert_eq!(run(json!({"limit": 128})), expected, "{source:?}");
        let l = run(json!({ "path": chain(127) }));
        assert_eq!(
            (l.entries, l.too_deep, l.truncated),
            (vec![(chain(128), "dir".to_owned())], 1, false),
            "{source:?}"
        );
        let l = run(json!({ "path": chain(128) }));
        assert_eq!(
            l.entries,
            vec![(chain(129), "dir".to_owned())],
            "{source:?}: a path at the bound descends no directory"
        );
        assert!(!l.truncated, "{source:?}");
    }

    // 2,100 nested directories: longer than PATH_MAX on the host, built by `mkdir -p`.
    fs::create_dir(w.at("c2100")).unwrap();
    let mut mkdir = Command::new("mkdir");
    mkdir.arg("-p").arg(chain(2100)).current_dir(w.at("c2100"));
    let out = spawn_with_timeout(mkdir, None, TIMEOUT);
    assert!(out.status.success(), "mkdir -p of the 2,100-deep chain");
    let mut find = Command::new("find");
    // `find`'s start point `d` is depth 0: exactly one directory at depth 2,099 or more.
    find.args(["d", "-mindepth", "2099", "-type", "d"])
        .current_dir(w.at("c2100"));
    let out = spawn_with_timeout(find, None, TIMEOUT);
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).lines().count(),
        1,
        "the chain is 2,100 deep"
    );
    let l = listing(&tree_ok(&w.dir, &json!({}), &["--root", "c2100"], TIMEOUT));
    assert_eq!(l, expected);
}
