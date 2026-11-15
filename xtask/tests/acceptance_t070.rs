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
//! - AC-3's tests of the procedure file are in `guard_t070.rs` (ruling m9, D-189: the file is the
//!   test author's and on the tests branch, so they pass on the base).
//!
//! No Unix-only API is used (the files run on macOS and Windows once T-070 has merged).
//! A missing precondition (no `git`, `ci.yml` or `xtask/ledger.toml` unreadable) is a panic that says so.
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
    /// The `id:` of the step that failed when `failed` (`steps.<id>.outcome` is `failure` for it
    /// and `success` for every other step); empty when no step is named.
    failed_step: &'static str,
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
            x if x.starts_with("steps.")
                && (x.ends_with(".outcome") || x.ends_with(".conclusion")) =>
            {
                let id = x["steps.".len()..x.rfind('.').unwrap()].to_owned();
                let failed = self.ctx.failed && !id.is_empty() && id == self.ctx.failed_step;
                V::S(if failed { "failure" } else { "success" }.into())
            }
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
        failed_step: "",
    }
}

const FAILED: Ctx = Ctx {
    event: "push",
    git_ref: "refs/heads/main",
    failed: true,
    failed_step: "",
};
const PASSED: Ctx = Ctx {
    event: "push",
    git_ref: "refs/heads/main",
    failed: false,
    failed_step: "",
};

/// The context after the step with `id:` `id` failed (`""` when the step has no `id:`).
fn failed_at(id: &str) -> Ctx {
    Ctx {
        failed_step: Box::leak(id.to_owned().into_boxed_str()),
        ..FAILED
    }
}

const CORE_PACKAGES: [&str; 4] = ["kiln-text", "kiln-syntax", "kiln-search", "xtask"];

/// Ruling m1 (D-182): the core-crates step runs one `cargo test --locked -p <package>` per
/// package of [`CORE_PACKAGES`] (written out, or as a `for <v> in <packages>; do … -p "$v" …`
/// loop), each appending to `core-crates.txt` (`>>` or `tee -a`, on the command or on the
/// loop's `done`), and none stops the others or the step: the job's `bash` runs with `-e`, so
/// each invocation (or the loop) is followed by `||`, or the run has `set +e` and ends in
/// `true`, `:` or `exit 0`. One invocation naming several packages fails (with one
/// `cargo test -p a -p b`, cargo runs no test when one package's tests do not compile).
/// Invocations are counted as commands (ruling m8, D-183): a statement invokes `cargo test`
/// only when its command word is `cargo test`; the words of an `echo`, `printf` or `:` line or
/// of a quoted string invoke nothing.
fn core_crates_check(run: &str) -> Result<(), String> {
    let text = run.replace("\\\n", " ");
    let lines: Vec<&str> = text.lines().map(|l| strip_comment(l).trim()).collect();
    let unq = |t: &str| {
        t.trim_matches(|c| c == '"' || c == '\'' || c == '{' || c == '}')
            .to_owned()
    };
    let appends = |st: &str| {
        let t: Vec<&str> = st.split_whitespace().collect();
        t.windows(2)
            .any(|w| w[0] == ">>" && w[1].trim_matches('"') == "core-crates.txt")
            || t.contains(&">>core-crates.txt")
            || t.windows(3).any(|w| {
                w[0] == "tee"
                    && (w[1] == "-a" || w[1] == "--append")
                    && w[2].trim_matches('"') == "core-crates.txt"
            })
    };
    let set_e_off = lines.iter().any(|l| l.starts_with("set +e"));
    let last = lines
        .iter()
        .rev()
        .find(|l| !l.is_empty())
        .copied()
        .unwrap_or("");
    let ends_ok = ["true", ":", "exit 0"].contains(&last);
    let mut loop_: Option<(String, Vec<String>)> = None;
    let mut done_ok = (false, false); // (appends, followed by ||)
    for l in &lines {
        if let Some(rest) = l.strip_prefix("for ") {
            let head = rest
                .split("; do")
                .next()
                .unwrap_or(rest)
                .split(";do")
                .next()
                .unwrap_or(rest);
            if let Some((v, list)) = head.split_once(" in ") {
                loop_ = Some((
                    v.trim().to_owned(),
                    list.split_whitespace().map(unq).collect(),
                ));
            }
        }
        if let Some(d) = l.split(';').map(str::trim).find(|x| x.starts_with("done")) {
            done_ok = (appends(d), d.contains("||"));
        }
    }
    let mut covered: Vec<String> = Vec::new();
    for l in &lines {
        for st in shell_statements(l) {
            let st = st.as_str();
            if !invokes_cargo_test(st) {
                continue;
            }
            let t: Vec<&str> = st.split_whitespace().collect();
            let ps: Vec<usize> = (0..t.len())
                .filter(|&i| t[i] == "-p" || t[i] == "--package")
                .collect();
            if ps.len() != 1 {
                return Err(format!(
                    "`{}` names {} packages; one `cargo test --locked -p <package>` per package (m1)",
                    st.trim(),
                    ps.len()
                ));
            }
            if !t.contains(&"--locked") {
                return Err(format!("`{}` has no --locked", st.trim()));
            }
            let pkg = unq(t.get(ps[0] + 1).copied().unwrap_or(""));
            let in_loop = match (&loop_, pkg.strip_prefix('$')) {
                (Some((v, list)), Some(var)) if var == v => {
                    covered.extend(list.iter().cloned());
                    true
                }
                (_, Some(var)) => {
                    return Err(format!(
                        "`{}`: ${var} is not a loop over the packages",
                        st.trim()
                    ));
                }
                _ => {
                    covered.push(pkg.clone());
                    false
                }
            };
            // The invocation's own redirect: what follows `||` runs only after a failure.
            let own = st.split("||").next().unwrap_or(st);
            if !(appends(own) || (in_loop && done_ok.0)) {
                return Err(format!(
                    "`{}` does not append to core-crates.txt (>> or tee -a)",
                    st.trim()
                ));
            }
            if !(st.contains("||") || (in_loop && done_ok.1) || (set_e_off && ends_ok)) {
                return Err(format!(
                    "`{}` stops the step on failure (bash -e): follow it with `||`, or `set +e` and end in `true`",
                    st.trim()
                ));
            }
        }
    }
    let mut got = covered.clone();
    got.sort();
    got.dedup();
    let mut want: Vec<String> = CORE_PACKAGES.iter().map(|p| (*p).to_owned()).collect();
    want.sort();
    if got != want || covered.len() != CORE_PACKAGES.len() {
        return Err(format!(
            "the step tests {covered:?}, not each of {CORE_PACKAGES:?} once"
        ));
    }
    Ok(())
}

/// A shell line split into statements at `;`, `&&` and a lone `&` outside quotes (`||` and `|`
/// stay inside the statement: they belong to it).
fn shell_statements(line: &str) -> Vec<String> {
    let cs: Vec<char> = line.chars().collect();
    let (mut out, mut cur) = (Vec::new(), String::new());
    let (mut sq, mut dq) = (false, false);
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        match c {
            '\\' if !sq && i + 1 < cs.len() => {
                cur.push(c);
                cur.push(cs[i + 1]);
                i += 2;
                continue;
            }
            '\'' if !dq => sq = !sq,
            '"' if !sq => dq = !dq,
            ';' if !sq && !dq => {
                out.push(std::mem::take(&mut cur));
                i += 1;
                continue;
            }
            '&' if !sq && !dq => {
                let redirect = cur.ends_with('>') || cs.get(i + 1) == Some(&'>');
                if cs.get(i + 1) == Some(&'&') {
                    out.push(std::mem::take(&mut cur));
                    i += 2;
                    continue;
                }
                if !redirect {
                    out.push(std::mem::take(&mut cur));
                    i += 1;
                    continue;
                }
            }
            _ => {}
        }
        cur.push(c);
        i += 1;
    }
    out.push(cur);
    out.into_iter().filter(|s| !s.trim().is_empty()).collect()
}

/// Whether a statement's command is `cargo test`: after the keywords that open a compound
/// command (`do`, `then`, `else`, `if`, `while`, `until`, `!`, `{`, `(`, `time`) and variable
/// assignments, the first two words are `cargo` and `test`. `echo … cargo test …` is not.
fn invokes_cargo_test(st: &str) -> bool {
    let mut words = st
        .split_whitespace()
        .map(|w| w.trim_start_matches(['(', '{']));
    let first = loop {
        match words.next() {
            None => return false,
            Some("" | "do" | "then" | "else" | "if" | "while" | "until" | "!" | "time") => {}
            Some(w) if w.contains('=') && !w.starts_with('-') && !w.starts_with('"') => {}
            Some(w) => break w,
        }
    };
    first == "cargo" && words.next() == Some("test")
}

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
                r.contains("cargo test") && r.contains("core-crates.txt")
            },
        );
        if let Err(e) = core_crates_check(run_of(&st[core])) {
            panic!("job `{id}`: the core-crates step: {e}");
        }
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
        // "Only when the gate failed": `failure()`, or `failure() && steps.<gate id>.outcome ==
        // 'failure'` (accepted by the director, D-182), and any equivalent form.
        let c = st[tout].get("if");
        let gate_id = st[gate].get("id").and_then(Y::str).unwrap_or("");
        assert!(
            runs(c, failed_at(gate_id)) && !runs(c, PASSED),
            "job `{id}`: the test-output step must run when the gate failed and only then (if: {c:?})"
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
    // m3 (D-182): the upload runs whatever the procedure's result.
    let c = st[up].get("if");
    assert!(
        runs(c, FAILED) && runs(c, PASSED),
        "window-linux: the upload does not run whatever the procedure's result (if: {c:?})"
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
    // m3 (D-182): the upload runs whatever the probe's result, and one leg failing does not
    // cancel the other (`fail-fast` is true unless set to false).
    let c = st[up].get("if");
    assert!(
        runs(c, FAILED) && runs(c, PASSED),
        "window-probe: the upload does not run whatever the probe's result (if: {c:?})"
    );
    let ff = job(&w, id).at(&["strategy", "fail-fast"]).and_then(Y::str);
    assert_eq!(
        ff,
        Some("false"),
        "window-probe: `strategy.fail-fast` must be false, or one leg's failure cancels the other"
    );
}

// ---- g4 (D-193): the window jobs' steps -------------------------------------------------------

/// The statements of a `run:` script (comments removed, `\`-continued lines joined), in order.
fn statements(run: &str) -> Vec<String> {
    let text = run.replace("\\\n", " ");
    text.lines()
        .flat_map(|l| shell_statements(strip_comment(l).trim()))
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

/// A statement's words from its command word on (after the keywords that open a compound
/// command and after variable assignments).
fn command_words(st: &str) -> Vec<String> {
    let words: Vec<String> = st
        .split_whitespace()
        .map(|w| w.trim_start_matches(['(', '{']).to_owned())
        .collect();
    let skip = words
        .iter()
        .take_while(|w| {
            matches!(
                w.as_str(),
                "" | "do" | "then" | "else" | "elif" | "if" | "while" | "until" | "!" | "time"
            ) || (w.contains('=') && !w.starts_with('-') && !w.starts_with('"'))
        })
        .count();
    words[skip..].to_vec()
}

/// A statement whose command only prints or does nothing (`echo`, `printf`, `:`). A statement
/// of assignments only (`limit=120`) is not one: its words are operands.
fn is_message(st: &str) -> bool {
    matches!(
        command_words(st).first().map(String::as_str),
        Some("echo" | "printf" | ":")
    )
}

/// The values of every `env:` the step sees (workflow, job, step), as text.
fn env_values(w: &Y, j: &Y, s: &Y) -> Vec<String> {
    let mut out = Vec::new();
    for e in [w.get("env"), j.get("env"), s.get("env")]
        .into_iter()
        .flatten()
    {
        if let Y::Map(m) = e {
            out.extend(m.iter().filter_map(|(_, v)| v.str().map(str::to_owned)));
        }
    }
    out
}

fn env_keys(w: &Y, j: &Y, s: &Y) -> Vec<String> {
    [w.get("env"), j.get("env"), s.get("env")]
        .into_iter()
        .flatten()
        .flat_map(|e| e.keys().into_iter().map(str::to_owned).collect::<Vec<_>>())
        .collect()
}

fn build_step(id: &str, st: &[Y]) -> usize {
    step_idx(id, st, "runs `cargo build -p kiln --locked`", |s| {
        statements(run_of(s)).iter().any(|x| {
            let c = command_words(x);
            c.len() >= 2 && c[0] == "cargo" && c[1] == "build" && x.contains("-p kiln")
        })
    })
}

fn procedure_step(st: &[Y]) -> usize {
    step_idx(
        "window-linux",
        st,
        "runs the procedure file into window-procedure.txt",
        |s| {
            run_of(s).contains(".github/scripts/window-procedure.sh")
                && run_of(s).contains("window-procedure.txt")
        },
    )
}

fn probe_step(st: &[Y]) -> usize {
    step_idx(
        "window-probe",
        st,
        "runs kiln --root … --script … into window-probe.txt",
        |s| {
            let r = run_of(s);
            r.contains("--root") && r.contains("--script") && r.contains("window-probe.txt")
        },
    )
}

/// The `target/` sub-directory the build statement of `run` writes the binary to (`debug` for
/// the dev profile, `release` for `--release`, the profile's name for `--profile`); an error
/// for a build that puts it elsewhere (`--target`, `--target-dir`).
fn build_profile_dir(run: &str) -> Result<String, String> {
    let st = statements(run)
        .into_iter()
        .find(|x| {
            let c = command_words(x);
            c.len() >= 2 && c[0] == "cargo" && c[1] == "build"
        })
        .ok_or("no `cargo build` statement")?;
    let c = command_words(&st);
    let mut dir = "debug".to_owned();
    for (i, w) in c.iter().enumerate() {
        if w == "--target"
            || w.starts_with("--target=")
            || w == "--target-dir"
            || w.starts_with("--target-dir=")
        {
            return Err(format!(
                "`{st}` writes the binary outside target/<profile>/"
            ));
        }
        if w == "--release" || w == "-r" {
            dir = "release".into();
        }
        let p = if w == "--profile" {
            c.get(i + 1).cloned()
        } else {
            w.strip_prefix("--profile=").map(str::to_owned)
        };
        if let Some(p) = p {
            dir = match p.trim_matches('"') {
                "dev" | "test" => "debug".into(),
                "bench" => "release".into(),
                other => other.into(),
            };
        }
    }
    Ok(dir)
}

#[test]
fn ac1_window_linux_procedure_status_is_not_masked() {
    // Interface (1): `window-linux` is red when the procedure's exit status is non-zero. The
    // step's `bash` runs with `-e` and `pipefail`; nothing may undo them or swallow the status.
    let w = workflow();
    let st = steps(job(&w, "window-linux"));
    let s = &st[procedure_step(st)];
    if let Some(sh) = s.get("shell").and_then(Y::str) {
        assert_eq!(
            sh, "bash",
            "window-linux: the procedure step runs in `{sh}`, not bash (-eo pipefail)"
        );
    }
    let sts = statements(run_of(s));
    for x in &sts {
        assert!(
            !x.contains("||"),
            "window-linux: the procedure step masks a failure: `{x}`"
        );
        let c = command_words(x);
        assert!(
            !(c.first().map(String::as_str) == Some("set")
                && c.iter().skip(1).any(|f| f.starts_with('+'))),
            "window-linux: the procedure step turns off -e or pipefail: `{x}`"
        );
        assert!(
            !(c.first().map(String::as_str) == Some("exit")
                && c.get(1).map(String::as_str) == Some("0")),
            "window-linux: the procedure step ends with `exit 0`: `{x}`"
        );
    }
    let last = sts.last().map(String::as_str).unwrap_or("");
    assert!(
        !["true", ":"].contains(&last),
        "window-linux: the procedure step ends with `{last}`, so its status is always 0"
    );
}

#[test]
fn ac1_window_jobs_build_the_profile_whose_binary_they_run() {
    // Interface (1): `cargo build -p kiln --locked`, then the procedure on `target/debug/kiln`.
    // The binary the procedure (or the probe) runs is the one the job built.
    let w = workflow();
    for id in ["window-linux", "window-probe"] {
        let j = job(&w, id);
        let st = steps(j);
        let b = build_step(id, st);
        for k in env_keys(&w, j, &st[b]) {
            assert!(
                ![
                    "CARGO_BUILD_TARGET",
                    "CARGO_TARGET_DIR",
                    "CARGO_BUILD_TARGET_DIR"
                ]
                .contains(&k.as_str()),
                "job `{id}`: env `{k}` moves the binary out of target/<profile>/"
            );
        }
        let dir = build_profile_dir(run_of(&st[b])).unwrap_or_else(|e| panic!("job `{id}`: {e}"));
        let user = if id == "window-linux" {
            procedure_step(st)
        } else {
            probe_step(st)
        };
        let text = format!(
            "{}\n{}",
            run_of(&st[user]),
            env_values(&w, j, &st[user]).join("\n")
        );
        let mut runs_dirs = Vec::new();
        let mut rest = text.as_str();
        while let Some(i) = rest.find("target/") {
            let tail = &rest[i + "target/".len()..];
            if let Some((d, after)) = tail.split_once('/')
                && after.starts_with("kiln")
            {
                runs_dirs.push(d.to_owned());
            }
            rest = tail;
        }
        if id == "window-linux" {
            assert!(
                !runs_dirs.is_empty(),
                "window-linux: the procedure step names no target/<profile>/kiln"
            );
        }
        for d in runs_dirs {
            assert_eq!(
                d, dir,
                "job `{id}`: builds target/{dir}/ but runs target/{d}/kiln"
            );
        }
    }
}

#[test]
fn ac1_window_probe_exits_with_kiln_status() {
    // Ruling m5 (D-182): a probe leg's colour is kiln's exit status. The step's last command
    // exits with a status saved from `$?`.
    let w = workflow();
    let st = steps(job(&w, "window-probe"));
    let sts = statements(run_of(&st[probe_step(st)]));
    let saved: Vec<String> = sts
        .iter()
        .flat_map(|x| x.split_whitespace().map(str::to_owned).collect::<Vec<_>>())
        .filter_map(|t| t.strip_suffix("=$?").map(str::to_owned))
        .filter(|v| !v.is_empty() && v.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
        .collect();
    assert!(
        !saved.is_empty(),
        "window-probe: the probe step saves no exit status (`v=$?`)"
    );
    let last = sts.last().cloned().unwrap_or_default();
    let c = command_words(&last);
    let ok = c.len() == 2
        && c[0] == "exit"
        && saved.iter().any(|v| {
            [
                format!("\"${v}\""),
                format!("${v}"),
                format!("${{{v}}}"),
                format!("\"${{{v}}}\""),
            ]
            .contains(&c[1])
        });
    assert!(
        ok,
        "window-probe: the probe step does not end by exiting with kiln's saved status ({saved:?}): `{last}`"
    );
}

#[test]
fn ac1_window_probe_runs_after_a_failed_build_and_records_not_run() {
    // Interface (1) and ruling g4: the probe step runs whatever the build's result (so a record
    // is written and uploaded) and records "not run" when the build failed.
    let w = workflow();
    let j = job(&w, "window-probe");
    let st = steps(j);
    let b = build_step("window-probe", st);
    let p = probe_step(st);
    assert!(b < p, "window-probe: the probe step precedes the build");
    let bid = st[b].get("id").and_then(Y::str).unwrap_or("");
    assert!(
        !bid.is_empty(),
        "window-probe: the build step has no `id:`, so the probe cannot read its result"
    );
    let c = st[p].get("if");
    assert!(
        runs(c, failed_at(bid)) && runs(c, PASSED),
        "window-probe: the probe step does not run whatever the build's result (if: {c:?})"
    );
    let text = squash(&format!(
        "{}\n{}",
        run_of(&st[p]),
        env_values(&w, j, &st[p]).join("\n")
    ));
    assert!(
        [
            format!("steps.{bid}.outcome"),
            format!("steps.{bid}.conclusion")
        ]
        .iter()
        .any(|r| text.contains(r.as_str())),
        "window-probe: the probe step does not read the build's result (steps.{bid}.outcome)"
    );
    assert!(
        run_of(&st[p]).to_ascii_lowercase().contains("not run"),
        "window-probe: the probe step records no \"not run\""
    );
}

#[test]
fn ac1_window_probe_is_bounded_to_120_seconds() {
    // Interface (1): one run bounded to 120 s. The bound is a command's operand (or an `env:`
    // value), not a message's text.
    let w = workflow();
    let j = job(&w, "window-probe");
    let st = steps(j);
    let p = probe_step(st);
    let numbers = |s: &str| -> Vec<String> {
        s.split(|c: char| !c.is_ascii_digit())
            .filter(|n| !n.is_empty())
            .map(str::to_owned)
            .collect()
    };
    let mut found: Vec<String> = statements(run_of(&st[p]))
        .iter()
        .filter(|x| !is_message(x))
        .flat_map(|x| numbers(x))
        .collect();
    found.extend(env_values(&w, j, &st[p]).iter().flat_map(|v| numbers(v)));
    assert!(
        found.iter().any(|n| n == "120"),
        "window-probe: no command of the probe step carries the 120 s bound (numbers: {found:?})"
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
