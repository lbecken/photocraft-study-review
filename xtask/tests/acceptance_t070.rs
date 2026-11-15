//! T-070 acceptance tests (test author): the criteria the spec assigns to the local gate.
//!
//! - AC-1: `.github/workflows/platforms.yml` read as text against Interface (1): name,
//!   permissions, no `concurrency`, no `continue-on-error`, the triggers with the `paths-ignore`
//!   list derived from `xtask/ledger.toml` as T-041's Interface (1) derives it, the five jobs and
//!   their runner labels, the gate step, the reports uploaded whatever the gate's result,
//!   `gate-linux`'s condition, the window jobs, every `uses:` pinned by a 40-hex sha.
//!   No YAML crate is in the lock (and none is added): a small reader for the block subset a
//!   workflow uses is below, and the `if:` expressions are evaluated by a small evaluator for
//!   the operators they need, so the tests check what a line means, not how it is spelled.
//! - AC-2: `.gitattributes` on a toy repository cloned with `core.autocrlf=true`: an LF and a CRLF
//!   file are byte-equal to their blobs; without the attribute file the LF file changes (the
//!   negative control, which also shows that the clone converts at all on this machine).
//! - AC-3 (ruling Q-3 B, D-180: structure in the gate, the run by the reviewer and the
//!   `window-linux` job): every line of the R-251 block of `tasks/T-065-window.md` in the
//!   procedure file in order, the usage line naming the file, and the exit rule for a run in
//!   which no verdict can pass (no `kiln`, no tools on `PATH`; no display, no network).
//!
//! No Unix-only API is used (the files run on macOS and Windows once T-070 has merged).
//! A missing precondition (no `git`, no `sh`, the T-065 block not found) is a panic that says so.
#![cfg(test)]

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

// ---- files ---------------------------------------------------------------------------------

/// The repository root (the parent of the `xtask` package).
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a parent directory")
        .to_path_buf()
}

const WORKFLOW: &str = ".github/workflows/platforms.yml";
const PROCEDURE: &str = ".github/scripts/window-procedure.sh";

/// The text of a file under the root; the file not existing is an assertion failure (it is
/// the subject of the test, not a precondition).
fn subject(rel: &str) -> String {
    let p = root().join(rel);
    assert!(p.is_file(), "{rel} does not exist (T-070 adds it)");
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

/// A file the tests read as an oracle; missing is a precondition failure.
fn oracle(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel))
        .unwrap_or_else(|e| panic!("precondition: cannot read {rel} (the oracle): {e}"))
}

/// A fresh, empty scratch directory for one test.
fn scratch(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("t070-acc")
        .join(name);
    if d.exists() {
        std::fs::remove_dir_all(&d).unwrap();
    }
    std::fs::create_dir_all(&d).unwrap();
    d
}

// ---- processes (every one with a timeout, D-028 (8)) ----------------------------------------

struct Out {
    code: Option<i32>,
    stdout: Vec<u8>,
    stderr: String,
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
    Out {
        code: status.code(),
        stdout: t1.join().unwrap(),
        stderr: String::from_utf8_lossy(&t2.join().unwrap()).into_owned(),
    }
}

/// `git` with an isolated configuration (no system or global file, nothing inherited from the
/// environment `cargo test` runs in), in `dir`; panics on failure; returns stdout as bytes.
fn git(dir: &Path, cfg: &Path, args: &[&str]) -> Vec<u8> {
    let mut c = Command::new("git");
    for (k, _) in std::env::vars_os() {
        if k.to_str().is_some_and(|k| k.starts_with("GIT_")) {
            c.env_remove(k);
        }
    }
    c.env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", cfg)
        .env("GIT_AUTHOR_NAME", "Toy")
        .env("GIT_AUTHOR_EMAIL", "toy@example.invalid")
        .env("GIT_COMMITTER_NAME", "Toy")
        .env("GIT_COMMITTER_EMAIL", "toy@example.invalid")
        .args(args)
        .current_dir(dir);
    let o = run_to(&mut c, 120);
    assert_eq!(
        o.code,
        Some(0),
        "git {args:?} in {}: {}",
        dir.display(),
        o.stderr
    );
    o.stdout
}

/// The `sh` on `PATH`, as an absolute path (the exit-rule run empties `PATH`).
fn find_sh() -> PathBuf {
    let path = std::env::var_os("PATH").unwrap_or_default();
    for d in std::env::split_paths(&path) {
        for name in ["sh", "sh.exe"] {
            let p = d.join(name);
            if p.is_file() {
                return p;
            }
        }
    }
    panic!("precondition: no `sh` on PATH (the window procedure is a POSIX sh script)");
}

/// A path as `sh` takes it on every platform (forward slashes; Git Bash reads `C:/…`).
fn sh_path(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

// ---- a reader for the YAML subset of a workflow ----------------------------------------------

#[derive(Clone, Debug, PartialEq)]
enum Y {
    Map(Vec<(String, Y)>),
    Seq(Vec<Y>),
    Str(String),
    Null,
}

impl Y {
    fn get(&self, k: &str) -> Option<&Y> {
        match self {
            Y::Map(m) => m.iter().find(|(kk, _)| kk == k).map(|(_, v)| v),
            _ => None,
        }
    }
    fn at(&self, path: &[&str]) -> Option<&Y> {
        path.iter().try_fold(self, |y, k| y.get(k))
    }
    fn str(&self) -> Option<&str> {
        match self {
            Y::Str(s) => Some(s),
            _ => None,
        }
    }
    fn keys(&self) -> Vec<&str> {
        match self {
            Y::Map(m) => m.iter().map(|(k, _)| k.as_str()).collect(),
            _ => Vec::new(),
        }
    }
    fn seq(&self) -> &[Y] {
        match self {
            Y::Seq(s) => s,
            _ => &[],
        }
    }
    /// The scalar items of a sequence (a scalar alone counts as a one-item list).
    fn strs(&self) -> Vec<String> {
        match self {
            Y::Seq(s) => s
                .iter()
                .filter_map(|y| y.str().map(str::to_owned))
                .collect(),
            Y::Str(s) => vec![s.clone()],
            _ => Vec::new(),
        }
    }
    /// Every (key, value) pair anywhere in the tree.
    fn walk<'a>(&'a self, out: &mut Vec<(&'a str, &'a Y)>) {
        match self {
            Y::Map(m) => {
                for (k, v) in m {
                    out.push((k, v));
                    v.walk(out);
                }
            }
            Y::Seq(s) => s.iter().for_each(|y| y.walk(out)),
            _ => {}
        }
    }
}

struct Line {
    indent: usize,
    text: String,
    blank: bool,
}

/// Remove a ` #…` comment outside quotes.
fn strip_comment(s: &str) -> &str {
    let (mut sq, mut dq) = (false, false);
    let b = s.as_bytes();
    for (i, &c) in b.iter().enumerate() {
        match c {
            b'\'' if !dq => sq = !sq,
            b'"' if !sq => dq = !dq,
            b'#' if !sq && !dq && (i == 0 || b[i - 1] == b' ' || b[i - 1] == b'\t') => {
                return s[..i].trim_end();
            }
            _ => {}
        }
    }
    s.trim_end()
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('\'') && s.ends_with('\'') {
        s[1..s.len() - 1].replace("''", "'")
    } else if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1].replace("\\\"", "\"")
    } else {
        s.to_owned()
    }
}

/// Split at top-level commas (outside quotes, brackets and braces).
fn split_flow(s: &str) -> Vec<String> {
    let (mut out, mut cur, mut depth, mut sq, mut dq) =
        (Vec::new(), String::new(), 0, false, false);
    for c in s.chars() {
        match c {
            '\'' if !dq => sq = !sq,
            '"' if !sq => dq = !dq,
            '[' | '{' if !sq && !dq => depth += 1,
            ']' | '}' if !sq && !dq => depth -= 1,
            ',' if !sq && !dq && depth == 0 => {
                out.push(std::mem::take(&mut cur));
                continue;
            }
            _ => {}
        }
        cur.push(c);
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out
}

/// Position of the `:` that ends a mapping key (followed by a space or the end), outside quotes.
fn key_colon(s: &str) -> Option<usize> {
    let (mut sq, mut dq) = (false, false);
    let b = s.as_bytes();
    for (i, &c) in b.iter().enumerate() {
        match c {
            b'\'' if !dq => sq = !sq,
            b'"' if !sq => dq = !dq,
            b':' if !sq && !dq && (i + 1 == b.len() || b[i + 1] == b' ') => return Some(i),
            b'{' | b'[' if !sq && !dq && i == 0 => return None,
            _ => {}
        }
    }
    None
}

fn flow(s: &str) -> Y {
    let s = s.trim();
    if s.is_empty() || s == "~" || s == "null" {
        Y::Null
    } else if s.starts_with('[') && s.ends_with(']') {
        Y::Seq(
            split_flow(&s[1..s.len() - 1])
                .iter()
                .map(|x| flow(x))
                .collect(),
        )
    } else if s.starts_with('{') && s.ends_with('}') {
        Y::Map(
            split_flow(&s[1..s.len() - 1])
                .iter()
                .map(|kv| match key_colon(kv.trim()) {
                    Some(i) => (unquote(&kv.trim()[..i]), flow(&kv.trim()[i + 1..])),
                    None => (unquote(kv), Y::Null),
                })
                .collect(),
        )
    } else {
        Y::Str(unquote(s))
    }
}

struct Reader {
    lines: Vec<Line>,
    i: usize,
}

impl Reader {
    fn new(text: &str) -> Reader {
        let lines = text
            .lines()
            .map(|raw| {
                let indent = raw.len() - raw.trim_start_matches(' ').len();
                let t = raw.trim();
                Line {
                    indent,
                    blank: t.is_empty() || t.starts_with('#') || t == "---",
                    text: raw[indent..].trim_end().to_owned(),
                }
            })
            .collect();
        Reader { lines, i: 0 }
    }
    fn skip_blank(&mut self) {
        while self.i < self.lines.len() && self.lines[self.i].blank {
            self.i += 1;
        }
    }
    fn peek(&mut self) -> Option<(usize, bool)> {
        self.skip_blank();
        self.lines
            .get(self.i)
            .map(|l| (l.indent, l.text == "-" || l.text.starts_with("- ")))
    }
    fn block(&mut self, indent: usize) -> Y {
        match self.peek() {
            Some((ind, true)) if ind == indent => self.seq(indent),
            Some((ind, false)) if ind == indent => self.map(indent),
            _ => Y::Null,
        }
    }
    fn seq(&mut self, indent: usize) -> Y {
        let mut items = Vec::new();
        while let Some((ind, dash)) = self.peek() {
            if ind != indent || !dash {
                break;
            }
            let text = self.lines[self.i].text.clone();
            let rest = text[1..].trim_start();
            if rest.is_empty() {
                self.i += 1;
                let child = self.peek().map_or(0, |(i, _)| i);
                items.push(if child > indent {
                    self.block(child)
                } else {
                    Y::Null
                });
            } else if key_colon(rest).is_some() {
                // `- key: value`: the item is a mapping whose keys sit where `key` starts.
                let at = indent + (text.len() - rest.len());
                self.lines[self.i].indent = at;
                self.lines[self.i].text = rest.to_owned();
                items.push(self.map(at));
            } else {
                self.i += 1;
                items.push(flow(strip_comment(rest)));
            }
        }
        Y::Seq(items)
    }
    fn map(&mut self, indent: usize) -> Y {
        let mut m = Vec::new();
        while let Some((ind, dash)) = self.peek() {
            if ind != indent || dash {
                break;
            }
            let text = strip_comment(&self.lines[self.i].text).to_owned();
            let Some(c) = key_colon(&text) else {
                panic!("cannot read line {} of the workflow: {text}", self.i + 1)
            };
            let key = unquote(&text[..c]);
            let val = text[c + 1..].trim().to_owned();
            self.i += 1;
            let v = if val.starts_with('|') || val.starts_with('>') {
                let mut body = Vec::new();
                while self.i < self.lines.len() {
                    let l = &self.lines[self.i];
                    if !l.text.is_empty() && l.indent <= indent {
                        break;
                    }
                    body.push(l.text.clone());
                    self.i += 1;
                }
                Y::Str(body.join("\n"))
            } else if val.is_empty() {
                match self.peek() {
                    Some((i, _)) if i > indent => self.block(i),
                    Some((i, true)) if i == indent => self.seq(indent),
                    _ => Y::Null,
                }
            } else {
                flow(&val)
            };
            m.push((key, v));
        }
        Y::Map(m)
    }
}

fn parse(text: &str) -> Y {
    let mut r = Reader::new(text);
    let y = r.block(0);
    r.skip_blank();
    assert!(
        r.i >= r.lines.len(),
        "cannot read the workflow from line {}: {}",
        r.i + 1,
        r.lines[r.i].text
    );
    y
}

// ---- a small evaluator for GitHub `if:` expressions -------------------------------------------

/// What an `if:` is evaluated against.
#[derive(Clone, Copy, Debug)]
struct Ctx {
    event: &'static str,
    git_ref: &'static str,
    /// A previous step of the job failed.
    failed: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum V {
    S(String),
    B(bool),
}

impl V {
    fn truthy(&self) -> bool {
        match self {
            V::B(b) => *b,
            V::S(s) => !s.is_empty(),
        }
    }
    fn text(&self) -> String {
        match self {
            V::B(b) => b.to_string(),
            V::S(s) => s.clone(),
        }
    }
}

struct Ev<'a> {
    t: Vec<String>,
    i: usize,
    ctx: Ctx,
    src: &'a str,
}

fn tokens(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == '\'' {
            let mut j = i + 1;
            let mut lit = String::from("'");
            while j < cs.len() {
                if cs[j] == '\'' && cs.get(j + 1) == Some(&'\'') {
                    lit.push('\'');
                    j += 2;
                } else if cs[j] == '\'' {
                    break;
                } else {
                    lit.push(cs[j]);
                    j += 1;
                }
            }
            out.push(lit);
            i = j + 1;
        } else if i + 1 < cs.len()
            && ["==", "!=", "&&", "||"].contains(&&*format!("{c}{}", cs[i + 1]))
        {
            out.push(format!("{c}{}", cs[i + 1]));
            i += 2;
        } else if "!(),".contains(c) {
            out.push(c.to_string());
            i += 1;
        } else {
            let mut j = i;
            while j < cs.len() && (cs[j].is_alphanumeric() || "._-".contains(cs[j])) {
                j += 1;
            }
            assert!(j > i, "cannot read the expression {s:?} at {c:?}");
            out.push(cs[i..j].iter().collect());
            i = j;
        }
    }
    out
}

impl Ev<'_> {
    fn next(&mut self) -> Option<String> {
        let t = self.t.get(self.i).cloned();
        self.i += 1;
        t
    }
    fn eat(&mut self, s: &str) -> bool {
        if self.t.get(self.i).map(String::as_str) == Some(s) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn or(&mut self) -> V {
        let mut v = self.and();
        while self.eat("||") {
            let r = self.and();
            v = if v.truthy() { v } else { r };
        }
        v
    }
    fn and(&mut self) -> V {
        let mut v = self.cmp();
        while self.eat("&&") {
            let r = self.cmp();
            v = if v.truthy() { r } else { v };
        }
        v
    }
    fn cmp(&mut self) -> V {
        let l = self.unary();
        if self.eat("==") {
            let r = self.unary();
            V::B(l.text().eq_ignore_ascii_case(&r.text()))
        } else if self.eat("!=") {
            let r = self.unary();
            V::B(!l.text().eq_ignore_ascii_case(&r.text()))
        } else {
            l
        }
    }
    fn unary(&mut self) -> V {
        if self.eat("!") {
            return V::B(!self.unary().truthy());
        }
        if self.eat("(") {
            let v = self.or();
            assert!(self.eat(")"), "unbalanced ( in {:?}", self.src);
            return v;
        }
        let t = self
            .next()
            .unwrap_or_else(|| panic!("expression ends early: {:?}", self.src));
        if let Some(lit) = t.strip_prefix('\'') {
            return V::S(lit.to_owned());
        }
        if self.eat("(") {
            let mut args = Vec::new();
            if !self.eat(")") {
                loop {
                    args.push(self.or());
                    if self.eat(")") {
                        break;
                    }
                    assert!(self.eat(","), "bad call in {:?}", self.src);
                }
            }
            let a = |i: usize| {
                args.get(i)
                    .map(V::text)
                    .unwrap_or_default()
                    .to_ascii_lowercase()
            };
            return V::B(match t.as_str() {
                "always" => true,
                "success" => !self.ctx.failed,
                "failure" => self.ctx.failed,
                "cancelled" => false,
                "startsWith" => a(0).starts_with(&a(1)),
                "endsWith" => a(0).ends_with(&a(1)),
                "contains" => a(0).contains(&a(1)),
                f => panic!("the test cannot evaluate function {f} in {:?}", self.src),
            });
        }
        match t.as_str() {
            "true" => V::B(true),
            "false" => V::B(false),
            "github.event_name" => V::S(self.ctx.event.into()),
            "github.ref" => V::S(self.ctx.git_ref.into()),
            "github.ref_name" => V::S(
                self.ctx
                    .git_ref
                    .strip_prefix("refs/heads/")
                    .unwrap_or(self.ctx.git_ref)
                    .into(),
            ),
            x => panic!("the test cannot evaluate {x} in {:?}", self.src),
        }
    }
}

/// Whether a job or step with this `if:` (None = no `if:`) runs in `ctx`. As GitHub does, an
/// expression without a status function is read as `success() && (expr)`.
fn runs(cond: Option<&Y>, ctx: Ctx) -> bool {
    let Some(cond) = cond else { return !ctx.failed };
    let raw = cond
        .str()
        .unwrap_or_else(|| panic!("`if:` is not a scalar: {cond:?}"));
    let s = raw.trim();
    let s = s
        .strip_prefix("${{")
        .and_then(|s| s.strip_suffix("}}"))
        .unwrap_or(s);
    let mut ev = Ev {
        t: tokens(s),
        i: 0,
        ctx,
        src: raw,
    };
    let v = ev.or();
    assert!(ev.i >= ev.t.len(), "trailing tokens in `if: {raw}`");
    let status = ["always(", "success(", "failure(", "cancelled("]
        .iter()
        .any(|f| s.replace(' ', "").contains(f));
    v.truthy() && (status || !ctx.failed)
}

const EVENTS: [(&str, &str); 5] = [
    ("push", "refs/heads/main"),
    ("push", "refs/heads/ci/T-070"),
    ("schedule", "refs/heads/main"),
    ("workflow_dispatch", "refs/heads/main"),
    ("workflow_dispatch", "refs/heads/ci/T-070"),
];

fn ok(event: &'static str, git_ref: &'static str) -> Ctx {
    Ctx {
        event,
        git_ref,
        failed: false,
    }
}

const FAILED: Ctx = Ctx {
    event: "push",
    git_ref: "refs/heads/main",
    failed: true,
};
const PASSED: Ctx = Ctx {
    event: "push",
    git_ref: "refs/heads/main",
    failed: false,
};

// ---- workflow helpers ---------------------------------------------------------------------------

fn workflow() -> Y {
    parse(&subject(WORKFLOW))
}

fn job<'a>(w: &'a Y, id: &str) -> &'a Y {
    w.at(&["jobs", id])
        .unwrap_or_else(|| panic!("{WORKFLOW} has no job `{id}`"))
}

fn steps(j: &Y) -> &[Y] {
    j.get("steps").map_or(&[], Y::seq)
}

fn run_of(s: &Y) -> &str {
    s.get("run").and_then(Y::str).unwrap_or("")
}

fn uses_of(s: &Y) -> &str {
    s.get("uses").and_then(Y::str).unwrap_or("")
}

/// The index of the one step matching `f`, named `what` in the failure.
fn step_idx(job_id: &str, st: &[Y], what: &str, f: impl Fn(&Y) -> bool) -> usize {
    let hits: Vec<usize> = (0..st.len()).filter(|&i| f(&st[i])).collect();
    assert!(!hits.is_empty(), "job `{job_id}` has no step that {what}");
    hits[0]
}

/// Whitespace removed (to compare `${{ github.run_id }}` and `${{github.run_id}}`).
fn squash(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

fn is_sha_pin(uses: &str) -> bool {
    match uses.split_once('@') {
        Some((action, sha)) => {
            action.contains('/')
                && sha.len() == 40
                && sha
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        }
        None => false,
    }
}

/// The sha `ci.yml` pins for an action (`actions/checkout`, `Swatinem/rust-cache`).
fn ci_yml_sha(action: &str) -> String {
    let ci = parse(&oracle(".github/workflows/ci.yml"));
    let mut pairs = Vec::new();
    ci.walk(&mut pairs);
    pairs
        .iter()
        .filter(|(k, _)| *k == "uses")
        .filter_map(|(_, v)| v.str())
        .find_map(|u| u.strip_prefix(&format!("{action}@")).map(str::to_owned))
        .unwrap_or_else(|| panic!("precondition: ci.yml pins no {action}"))
}

/// T-041 Interface (1): for each `mode = "any"` entry of `xtask/ledger.toml`, in file order, `e`
/// and `e/**` with `e` the path without a trailing `/`; `add-only` entries contribute nothing.
fn ledger_paths_ignore() -> Vec<String> {
    let t: toml::Value = toml::from_str(&oracle("xtask/ledger.toml"))
        .unwrap_or_else(|e| panic!("precondition: xtask/ledger.toml does not parse: {e}"));
    let entries = t
        .get("ledger")
        .and_then(toml::Value::as_array)
        .unwrap_or_else(|| panic!("precondition: xtask/ledger.toml has no [[ledger]]"));
    let mut out = Vec::new();
    for e in entries {
        let path = e.get("path").and_then(toml::Value::as_str).unwrap();
        let mode = e.get("mode").and_then(toml::Value::as_str).unwrap();
        if mode == "any" {
            let p = path.trim_end_matches('/');
            out.push(p.to_owned());
            out.push(format!("{p}/**"));
        }
    }
    assert!(!out.is_empty(), "precondition: no mode-any ledger entry");
    out
}

const GATE_JOBS: [(&str, &str); 3] = [
    ("gate-macos", "macos-15"),
    ("gate-windows", "windows-2025"),
    ("gate-linux", "ubuntu-24.04"),
];

// ---- AC-1 ------------------------------------------------------------------------------------

#[test]
fn ac1_name_permissions_no_concurrency_no_continue_on_error() {
    let text = subject(WORKFLOW);
    let w = parse(&text);
    assert_eq!(w.get("name").and_then(Y::str), Some("platforms"), "name");
    let mut pairs = Vec::new();
    w.walk(&mut pairs);
    let perms: Vec<&Y> = pairs
        .iter()
        .filter(|(k, _)| *k == "permissions")
        .map(|(_, v)| *v)
        .collect();
    assert!(w.get("permissions").is_some(), "no top-level `permissions`");
    let want = Y::Map(vec![("contents".into(), Y::Str("read".into()))]);
    for p in perms {
        assert_eq!(
            p, &want,
            "`permissions` must be `contents: read` and nothing else"
        );
    }
    for bad in ["concurrency", "continue-on-error"] {
        assert!(
            !pairs.iter().any(|(k, _)| *k == bad),
            "the workflow has a `{bad}` key"
        );
        // Also outside what the reader understood (comments excepted).
        for (n, l) in text.lines().enumerate() {
            assert!(
                !strip_comment(l).contains(bad),
                "{WORKFLOW}:{} mentions `{bad}` outside a comment",
                n + 1
            );
        }
    }
}

#[test]
fn ac1_triggers_and_the_ledger_filter() {
    let w = workflow();
    let on = w
        .get("on")
        .unwrap_or_else(|| panic!("{WORKFLOW} has no `on`"));
    for k in on.keys() {
        assert!(
            !k.starts_with("pull_request"),
            "trigger `{k}`: no code from forks may run"
        );
    }
    for k in ["push", "schedule", "workflow_dispatch"] {
        assert!(on.get(k).is_some(), "no `{k}` trigger");
    }
    let push = on.get("push").unwrap();
    let mut branches = push.get("branches").map(Y::strs).unwrap_or_default();
    branches.sort();
    assert_eq!(branches, ["ci/**", "main"], "push branches");
    let ignore = push.get("paths-ignore").map(Y::strs).unwrap_or_default();
    assert_eq!(
        ignore,
        ledger_paths_ignore(),
        "push.paths-ignore must equal, in order, the list derived from xtask/ledger.toml"
    );
    for k in ["paths", "branches-ignore"] {
        assert!(push.get(k).is_none(), "push has `{k}`");
    }
    // Once a day at a fixed minute.
    let sched = on.get("schedule").unwrap().seq();
    assert_eq!(sched.len(), 1, "exactly one schedule entry");
    let cron = sched[0]
        .get("cron")
        .and_then(Y::str)
        .unwrap_or_else(|| panic!("schedule has no cron"));
    let f: Vec<&str> = cron.split_whitespace().collect();
    assert_eq!(f.len(), 5, "cron {cron:?} has 5 fields");
    let num = |s: &str, max: u32| s.parse::<u32>().is_ok_and(|n| n <= max) && !s.starts_with('+');
    assert!(
        num(f[0], 59) && num(f[1], 23) && f[2..] == ["*", "*", "*"],
        "cron {cron:?} is not once a day at a fixed minute"
    );
    match on.get("workflow_dispatch").unwrap() {
        Y::Null => {}
        Y::Map(m) => assert!(
            m.iter().all(|(k, _)| k != "inputs"),
            "workflow_dispatch has inputs"
        ),
        other => panic!("workflow_dispatch: {other:?}"),
    }
}

#[test]
fn ac1_five_jobs_with_their_runner_labels_and_conditions() {
    let w = workflow();
    for (id, label) in GATE_JOBS.iter().chain(&[("window-linux", "ubuntu-24.04")]) {
        assert_eq!(
            job(&w, id).get("runs-on").and_then(Y::str),
            Some(*label),
            "runs-on of `{id}`"
        );
    }
    // window-probe: a matrix over the two labels.
    let probe = job(&w, "window-probe");
    let ro = squash(probe.get("runs-on").and_then(Y::str).unwrap_or(""));
    let key = ro
        .strip_prefix("${{matrix.")
        .and_then(|s| s.strip_suffix("}}"))
        .unwrap_or_else(|| panic!("window-probe runs-on {ro:?} is not a matrix value"));
    let mut labels = probe
        .at(&["strategy", "matrix", key])
        .map(Y::strs)
        .unwrap_or_default();
    labels.sort();
    assert_eq!(labels, ["macos-15", "windows-2025"], "window-probe matrix");
    // Every trigger runs every job but gate-linux.
    for id in ["gate-macos", "gate-windows", "window-linux", "window-probe"] {
        for (e, r) in EVENTS {
            assert!(
                runs(job(&w, id).get("if"), ok(e, r)),
                "job `{id}` does not run for {e} on {r}"
            );
        }
    }
    // gate-linux: only ci/** pushes, schedule and dispatch; not pushes to main (Q-1 A).
    let cond = job(&w, "gate-linux").get("if");
    for (e, r) in EVENTS {
        let want = !(e == "push" && r == "refs/heads/main");
        assert_eq!(
            runs(cond, ok(e, r)),
            want,
            "gate-linux for {e} on {r} (if: {cond:?})"
        );
    }
}

#[test]
fn ac1_gate_jobs_run_the_gate_and_upload_the_reports_whatever_its_result() {
    let w = workflow();
    let checkout = ci_yml_sha("actions/checkout");
    let cache = ci_yml_sha("Swatinem/rust-cache");
    for (id, _) in GATE_JOBS {
        let st = steps(job(&w, id));
        let co = step_idx(id, st, "uses actions/checkout at ci.yml's sha", |s| {
            uses_of(s) == format!("actions/checkout@{checkout}")
        });
        let tc = step_idx(
            id,
            st,
            "installs the toolchain and prints the three versions",
            |s| {
                let r = run_of(s);
                [
                    "rustup toolchain install",
                    "cargo --version",
                    "cargo clippy --version",
                    "rustfmt --version",
                ]
                .iter()
                .all(|x| r.contains(x))
            },
        );
        let ca = step_idx(id, st, "uses Swatinem/rust-cache at ci.yml's sha", |s| {
            uses_of(s) == format!("Swatinem/rust-cache@{cache}")
        });
        let gate = step_idx(
            id,
            st,
            "runs `cargo xtask ci --keep-going --report gate-report.json`",
            |s| run_of(s).contains("cargo xtask ci --keep-going --report gate-report.json"),
        );
        let core = step_idx(
            id,
            st,
            "runs the core-crates tests into core-crates.txt",
            |s| {
                let r = run_of(s);
                r.contains("cargo test")
                    && r.contains("--locked")
                    && ["kiln-text", "kiln-syntax", "kiln-search", "xtask"]
                        .iter()
                        .all(|p| r.contains(&format!("-p {p}")))
                    && r.contains("core-crates.txt")
            },
        );
        let tout = step_idx(
            id,
            st,
            "runs the workspace tests into test-output.txt",
            |s| {
                let r = run_of(s);
                r.contains("cargo test --workspace --locked --no-fail-fast")
                    && r.contains("test-output.txt")
            },
        );
        let disk = step_idx(id, st, "prints the disk line", |s| {
            let r = run_of(s);
            r.contains("df -h") && r.contains("du -sh target")
        });
        let up = step_idx(id, st, "uses actions/upload-artifact", |s| {
            uses_of(s).starts_with("actions/upload-artifact@")
        });
        let order = [co, tc, ca, gate, core, tout, disk, up];
        assert!(
            order.windows(2).all(|p| p[0] < p[1]),
            "job `{id}`: steps out of the Interface's order (checkout, toolchain, cache, gate, \
             core crates, test output, disk, upload): {order:?}"
        );
        assert!(
            st[gate].get("if").is_none(),
            "job `{id}`: the gate step has an `if:`"
        );
        for (i, what) in [(core, "core-crates"), (disk, "disk"), (up, "upload")] {
            assert!(
                runs(st[i].get("if"), FAILED) && runs(st[i].get("if"), PASSED),
                "job `{id}`: the {what} step does not run whatever the gate's result"
            );
        }
        let c = st[tout].get("if");
        assert!(
            runs(c, FAILED) && !runs(c, PASSED),
            "job `{id}`: the test-output step must run on failure only"
        );
        let with = st[up].get("with").cloned().unwrap_or(Y::Null);
        assert_eq!(
            with.get("name").and_then(Y::str).map(squash),
            Some(format!("{id}-${{{{github.run_id}}}}")),
            "job `{id}`: artefact name"
        );
        assert_eq!(
            with.get("retention-days").and_then(Y::str),
            Some("30"),
            "job `{id}`: retention"
        );
        let path = with.get("path").and_then(Y::str).unwrap_or("");
        for f in ["gate-report.json", "core-crates.txt", "test-output.txt"] {
            assert!(
                path.contains(f),
                "job `{id}`: the upload does not include {f}"
            );
        }
    }
}

#[test]
fn ac1_window_jobs_build_run_and_upload() {
    let w = workflow();
    let id = "window-linux";
    let st = steps(job(&w, id));
    let build = step_idx(id, st, "runs `cargo build -p kiln --locked`", |s| {
        run_of(s).contains("cargo build -p kiln --locked")
    });
    let proc_ = step_idx(
        id,
        st,
        "runs the procedure file into window-procedure.txt",
        |s| {
            let r = run_of(s);
            r.contains("sh .github/scripts/window-procedure.sh target/debug/kiln ")
                && r.contains("RUNNER_TEMP")
                && r.contains("window-procedure.txt")
        },
    );
    step_idx(id, st, "lists the X packages with dpkg-query -W", |s| {
        let r = run_of(s);
        r.contains("dpkg-query -W") && r.contains("libxkbcommon-x11-0") && r.contains("libxcb-xkb1")
    });
    let up = step_idx(
        id,
        st,
        "uploads window-procedure.txt as window-linux-<run_id>",
        |s| {
            let with = s.get("with").cloned().unwrap_or(Y::Null);
            uses_of(s).starts_with("actions/upload-artifact@")
                && with.get("name").and_then(Y::str).map(squash).as_deref()
                    == Some("window-linux-${{github.run_id}}")
                && with
                    .get("path")
                    .and_then(Y::str)
                    .is_some_and(|p| p.contains("window-procedure.txt"))
        },
    );
    assert!(
        build < proc_ && proc_ < up,
        "window-linux: build, procedure, upload out of order"
    );

    let id = "window-probe";
    let st = steps(job(&w, id));
    let build = step_idx(id, st, "runs `cargo build -p kiln --locked`", |s| {
        run_of(s).contains("cargo build -p kiln --locked")
    });
    let probe = step_idx(
        id,
        st,
        "runs kiln --root … --script … into window-probe.txt",
        |s| {
            let r = run_of(s);
            r.contains("--root") && r.contains("--script") && r.contains("window-probe.txt")
        },
    );
    let up = step_idx(
        id,
        st,
        "uploads window-probe.txt as window-probe-<os>-<run_id>",
        |s| {
            let with = s.get("with").cloned().unwrap_or(Y::Null);
            let name = with
                .get("name")
                .and_then(Y::str)
                .map(squash)
                .unwrap_or_default();
            uses_of(s).starts_with("actions/upload-artifact@")
                && name.starts_with("window-probe-")
                && name.contains("${{matrix.")
                && name.ends_with("${{github.run_id}}")
                && with
                    .get("path")
                    .and_then(Y::str)
                    .is_some_and(|p| p.contains("window-probe.txt"))
        },
    );
    assert!(
        build < probe && probe < up,
        "window-probe: build, probe, upload out of order"
    );
}

#[test]
fn ac1_defaults_env_checkout_and_every_action_pinned_by_sha() {
    let text = subject(WORKFLOW);
    let w = parse(&text);
    assert_eq!(
        w.at(&["defaults", "run", "shell"]).and_then(Y::str),
        Some("bash"),
        "defaults.run.shell"
    );
    let jobs = w.get("jobs").cloned().unwrap_or(Y::Null);
    assert!(!jobs.keys().is_empty(), "no jobs");
    for id in jobs.keys() {
        let j = jobs.get(id).unwrap();
        for (k, v) in [("CARGO_TERM_COLOR", "always"), ("CARGO_INCREMENTAL", "0")] {
            let got = j
                .at(&["env", k])
                .or_else(|| w.at(&["env", k]))
                .and_then(Y::str);
            assert_eq!(got, Some(v), "job `{id}`: env {k}");
        }
        for s in steps(j) {
            if uses_of(s).starts_with("actions/checkout@") {
                assert_eq!(
                    s.at(&["with", "fetch-depth"]).and_then(Y::str),
                    Some("1"),
                    "job `{id}`: fetch-depth"
                );
                assert_eq!(
                    s.at(&["with", "persist-credentials"]).and_then(Y::str),
                    Some("false"),
                    "job `{id}`: persist-credentials"
                );
            }
        }
    }
    let mut pairs = Vec::new();
    w.walk(&mut pairs);
    let uses: Vec<&str> = pairs
        .iter()
        .filter(|(k, _)| *k == "uses")
        .filter_map(|(_, v)| v.str())
        .collect();
    assert!(!uses.is_empty(), "no `uses:` at all");
    for u in &uses {
        assert!(
            is_sha_pin(u),
            "`uses: {u}` is not pinned by a full 40-hex commit sha"
        );
    }
    for a in ["actions/checkout", "Swatinem/rust-cache"] {
        let sha = ci_yml_sha(a);
        for u in uses.iter().filter(|u| u.starts_with(&format!("{a}@"))) {
            assert_eq!(
                *u,
                &format!("{a}@{sha}"),
                "{a} at another sha than ci.yml's"
            );
        }
    }
    // The release named in a comment: on the `uses:` line or the line just above it.
    let lines: Vec<&str> = text.lines().collect();
    let names_release = |c: &str| {
        let b = c.as_bytes();
        (0..b.len()).any(|i| {
            (b[i] == b'v' && b.get(i + 1).is_some_and(u8::is_ascii_digit))
                || (b[i].is_ascii_digit() && b.get(i + 1) == Some(&b'.'))
        })
    };
    for (n, l) in lines.iter().enumerate() {
        let code = strip_comment(l);
        if !(code.trim_start().starts_with("uses:") || code.trim_start().starts_with("- uses:")) {
            continue;
        }
        let same = &l[code.len()..];
        let above = (0..n)
            .rev()
            .map(|i| lines[i].trim())
            .find(|t| !t.is_empty())
            .unwrap_or("");
        let ok = (same.contains('#') && names_release(same))
            || (above.starts_with('#') && names_release(above));
        assert!(ok, "{WORKFLOW}:{}: no comment naming the release", n + 1);
    }
}

// ---- AC-2 ------------------------------------------------------------------------------------

/// A toy repository with an LF and a CRLF file (and `attrs` as `.gitattributes` if given),
/// committed with no conversion, cloned with `core.autocrlf=true`; returns, per file, whether
/// the checked-out bytes equal the blob.
fn autocrlf_clone(name: &str, attrs: Option<&str>) -> Vec<(&'static str, bool, Vec<u8>)> {
    let d = scratch(name);
    let cfg = d.join("empty-gitconfig");
    std::fs::write(&cfg, "").unwrap();
    let src = d.join("src");
    std::fs::create_dir_all(&src).unwrap();
    git(&src, &cfg, &["-c", "init.defaultBranch=main", "init", "-q"]);
    std::fs::write(src.join("lf.txt"), b"one\ntwo\n").unwrap();
    std::fs::write(src.join("crlf.txt"), b"one\r\ntwo\r\n").unwrap();
    if let Some(a) = attrs {
        std::fs::write(src.join(".gitattributes"), a).unwrap();
    }
    let raw = ["-c", "core.autocrlf=false", "-c", "core.safecrlf=false"];
    git(&src, &cfg, &[&raw[..], &["add", "-A"]].concat());
    git(
        &src,
        &cfg,
        &[&raw[..], &["commit", "-q", "-m", "toy"]].concat(),
    );
    git(
        &d,
        &cfg,
        &["clone", "-q", "-c", "core.autocrlf=true", "src", "dst"],
    );
    let dst = d.join("dst");
    let got = ["lf.txt", "crlf.txt"]
        .into_iter()
        .map(|f| {
            let blob = git(&dst, &cfg, &["cat-file", "-p", &format!("HEAD:{f}")]);
            let work = std::fs::read(dst.join(f)).unwrap();
            (f, work == blob, work)
        })
        .collect();
    std::fs::remove_dir_all(&d).unwrap();
    got
}

#[test]
fn ac2_autocrlf_clone_keeps_every_byte_with_the_attribute_file() {
    // Negative control first: without the attribute file the LF file is converted. This is
    // also the precondition that `core.autocrlf=true` converts on this machine.
    let control = autocrlf_clone("control", None);
    assert!(
        !control[0].1,
        "precondition: without .gitattributes a core.autocrlf=true clone left lf.txt as \
         committed, so this machine does not show the conversion the attribute must prevent"
    );
    let attrs = subject(".gitattributes");
    for (f, equal, bytes) in autocrlf_clone("with-attributes", Some(&attrs)) {
        assert!(
            equal,
            "with the task's .gitattributes, {f} differs from its blob after a \
             core.autocrlf=true clone: {bytes:?}"
        );
    }
}

#[test]
fn ac2_gitattributes_is_the_one_pinned_line() {
    let a = subject(".gitattributes");
    assert!(
        a == "* -text\n" || a == "* -text",
        ".gitattributes must be exactly the one line `* -text`: {a:?}"
    );
}

// ---- AC-3 ------------------------------------------------------------------------------------

/// The R-251 block of `tasks/T-065-window.md`: the ```` ```sh ```` fence after "Window
/// procedure (replaces R-248's in full)" up to its closing fence.
fn r251_block() -> Vec<String> {
    let t = oracle("tasks/T-065-window.md");
    let lines: Vec<&str> = t.lines().collect();
    let at = lines
        .iter()
        .position(|l| l.contains("Window procedure (replaces R-248's in full)"))
        .unwrap_or_else(|| panic!("precondition: the R-251 entry is not in tasks/T-065-window.md"));
    let open = (at..lines.len())
        .find(|&i| lines[i] == "```sh")
        .unwrap_or_else(|| panic!("precondition: no ```sh fence after the R-251 heading"));
    let close = (open + 1..lines.len())
        .find(|&i| lines[i] == "```")
        .unwrap_or_else(|| panic!("precondition: the R-251 block is not closed"));
    let block: Vec<String> = lines[open + 1..close]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    let cases = block.iter().filter(|l| l.starts_with("case_ ")).count();
    assert_eq!(cases, 6, "precondition: the R-251 block has six case lines");
    block
}

#[test]
fn ac3_procedure_holds_every_line_of_r251_in_order() {
    let block = r251_block();
    let text = subject(PROCEDURE);
    let file: Vec<&str> = text.lines().collect();
    let mut from = 0;
    for want in block.iter().filter(|l| !l.contains("Usage: sh ac2.sh")) {
        match (from..file.len()).find(|&i| file[i] == want) {
            Some(i) => from = i + 1,
            None => {
                panic!("{PROCEDURE}: R-251's line is missing, changed or out of order: {want:?}")
            }
        }
    }
    for case in ["ac2", "ac2x3", "dblclick", "scroll100", "ctrlq", "noroot"] {
        assert!(
            file.iter()
                .any(|l| l.starts_with(&format!("case_ {case} "))),
            "{PROCEDURE}: no case {case}"
        );
    }
}

#[test]
fn ac3_usage_line_names_the_file() {
    let text = subject(PROCEDURE);
    assert!(
        text.lines()
            .any(|l| l.contains("Usage:") && l.contains("window-procedure.sh")),
        "{PROCEDURE}: no usage line naming window-procedure.sh"
    );
    assert!(!text.contains("ac2.sh"), "{PROCEDURE} still names ac2.sh");
}

#[test]
fn ac3_exit_status_is_1_when_no_verdict_passes() {
    // No kiln (a path that does not exist) and an empty PATH: nothing the procedure calls exists,
    // so no case can pass, nothing reaches a display or the network, and the script must still
    // run to its end and exit 1 (R-251's block always exits 0).
    let text = subject(PROCEDURE);
    assert!(!text.is_empty());
    let sh = find_sh();
    let d = scratch("exit-rule");
    let empty_path = d.join("empty-path");
    let work = d.join("work");
    std::fs::create_dir_all(&empty_path).unwrap();
    std::fs::create_dir_all(&work).unwrap();
    let o = run_to(
        Command::new(&sh)
            .arg(sh_path(&root().join(PROCEDURE)))
            .arg(sh_path(&d.join("no-such-kiln")))
            .arg(sh_path(&work))
            .env("PATH", &empty_path)
            .current_dir(&d),
        600,
    );
    let out = String::from_utf8_lossy(&o.stdout).into_owned();
    let lines: Vec<&str> = out.lines().collect();
    assert!(
        lines.contains(&"WINDOW CASES FAIL")
            && lines.iter().any(|l| l.starts_with("PLAIN NOT RUN")),
        "the procedure did not run to its end (no `WINDOW CASES FAIL` and `PLAIN NOT RUN`):\n{out}\n{}",
        o.stderr
    );
    for pass in ["WINDOW CASES PASS", "AC-3 PASS"] {
        assert!(
            !lines.contains(&pass),
            "printed {pass} without a kiln:\n{out}"
        );
    }
    assert!(
        !lines.iter().any(|l| l.starts_with("PLAIN PASS")),
        "printed PLAIN PASS without a kiln"
    );
    assert_eq!(
        o.code,
        Some(1),
        "exit status with every verdict failing:\n{out}"
    );
    std::fs::remove_dir_all(&d).unwrap();
}
