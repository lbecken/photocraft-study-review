//! T-072 guard tests (independent test author, run R-352): what holds on the base and must keep
//! holding on the joined branch (the spec's G-1 to G-4 and what the director's brief names).
//!
//! - G-1 (T-070 review round 2, finding 1): the window-probe step, run as the workflow holds it
//!   with a fake `kiln` that never exits and ignores SIGTERM, ends within 120 s + 15 s and
//!   records the kill.
//! - G-2: every recording step (a `run:` step that runs after a failure, in every job but
//!   `window-probe`) ends with status 0 when every command it calls fails, and ends in `exit 0`.
//! - G-3: no `continue-on-error`; every `uses:` pinned by a 40-hex sha.
//! - G-4: the window jobs write none of the gate jobs' files and upload only their own record
//!   (not an exact pin of their steps: QUESTION in the log, D-043/D-069).
//! - T-073's step and its pinned command; the five jobs, their labels and conditions, and the
//!   gate step as the job's colour (Pinned (a)).
//!
//! Scripts run in `bash` (Git Bash on Windows; none found is a failure naming why, Pinned (d));
//! no platform cfg. The YAML reader and `if:` evaluator are `acceptance_t070.rs`'s, copied.
#![cfg(test)]

const SCRATCH: &str = "t072-guard";

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

/// The text of a file under the root; the file not existing is an assertion failure.
fn subject(rel: &str) -> String {
    let p = root().join(rel);
    assert!(p.is_file(), "{rel} does not exist");
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

/// A fresh, empty scratch directory for one test (under cargo's per-target temp directory).
fn scratch(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(SCRATCH)
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
    stdout: String,
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
        stdout: String::from_utf8_lossy(&t1.join().unwrap()).into_owned(),
        stderr: String::from_utf8_lossy(&t2.join().unwrap()).into_owned(),
    }
}

/// The `bash` the workflow's steps run in (Git Bash on Windows). On `PATH` first, leaving out
/// the WSL launchers (`System32`, `WindowsApps`), then Git for Windows' default place. None
/// found is a failure naming why (METHODOLOGY §8.1): these tests never pass without running.
fn find_bash() -> PathBuf {
    let path = std::env::var_os("PATH").unwrap_or_default();
    for d in std::env::split_paths(&path) {
        let low = d.to_string_lossy().to_ascii_lowercase();
        if low.contains("system32") || low.contains("windowsapps") {
            continue;
        }
        for name in ["bash", "bash.exe"] {
            let p = d.join(name);
            if p.is_file() {
                return p;
            }
        }
    }
    for p in [
        "C:/Program Files/Git/bin/bash.exe",
        "C:/Program Files/Git/usr/bin/bash.exe",
    ] {
        if Path::new(p).is_file() {
            return PathBuf::from(p);
        }
    }
    panic!(
        "no bash found (PATH without System32/WindowsApps, or Git for Windows): the workflow's \
         steps run in bash (Git Bash on Windows), so this test runs them there and cannot run here"
    );
}

/// A path as `bash` takes it on every platform (forward slashes; Git Bash reads `C:/…`).
fn sh_path(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// A scratch bench for running one step's `run:` script as GitHub runs it
/// (`bash --noprofile --norc -eo pipefail <file>`), in `tree`, with `bin` first on `PATH`.
struct Bench {
    dir: PathBuf,
    tree: PathBuf,
    bin: PathBuf,
}

fn bench(name: &str) -> Bench {
    let dir = scratch(name);
    let tree = dir.join("tree");
    let bin = dir.join("bin");
    std::fs::create_dir_all(&tree).unwrap();
    std::fs::create_dir_all(&bin).unwrap();
    Bench { dir, tree, bin }
}

impl Bench {
    /// A fake command `name` in `bin` (a bash script).
    fn fake(&self, name: &str, body: &str) {
        std::fs::write(
            self.bin.join(name),
            format!("#!/usr/bin/env bash\n{body}\n"),
        )
        .unwrap();
    }
    /// Run `script` as a step; `env` is added to the inherited environment.
    fn run(&self, script: &str, env: &[(&str, String)], secs: u64) -> Out {
        let bash = find_bash();
        let file = self.dir.join("step.sh");
        std::fs::write(&file, format!("{script}\n")).unwrap();
        let chmod = run_to(
            Command::new(&bash)
                .arg("-c")
                .arg("chmod +x \"$1\"/* 2>/dev/null; exit 0")
                .arg("_")
                .arg(sh_path(&self.bin)),
            60,
        );
        assert_eq!(chmod.code, Some(0), "chmod of the fakes: {}", chmod.stderr);
        let old = std::env::var_os("PATH").unwrap_or_default();
        let mut dirs = vec![self.bin.clone()];
        dirs.extend(std::env::split_paths(&old));
        let path = std::env::join_paths(dirs).unwrap();
        let mut c = Command::new(&bash);
        c.arg("--noprofile")
            .arg("--norc")
            .arg("-eo")
            .arg("pipefail")
            .arg(sh_path(&file))
            .env("PATH", path)
            .current_dir(&self.tree);
        for (k, v) in env {
            c.env(k, v);
        }
        run_to(&mut c, secs)
    }
}

// ---- workflow helpers -----------------------------------------------------------------------

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

/// Whitespace removed.
fn squash(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// The statements of a `run:` script (comments removed, `\`-continued lines joined), in order.
fn statements(run: &str) -> Vec<String> {
    let text = run.replace("\\\n", " ");
    text.lines()
        .flat_map(|l| shell_statements(strip_comment(l).trim()))
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Whether a statement writes `file`: a `>`, `>>`, `&>` redirect to it, or `tee [-a] file`.
fn stmt_writes(st: &str, file: &str) -> bool {
    let t: Vec<String> = st
        .split_whitespace()
        .map(|w| w.trim_matches(|c| c == '"' || c == '\'').to_owned())
        .collect();
    let is = |w: &str| w.trim_matches(|c| c == '"' || c == '\'') == file;
    t.iter().enumerate().any(|(i, w)| {
        let redirect = ["1>", ">", ">>", "&>", "&>>", "1>>"];
        if redirect.contains(&w.as_str()) {
            return t.get(i + 1).is_some_and(|n| is(n));
        }
        if let Some(rest) = redirect
            .iter()
            .rev()
            .find_map(|r| w.strip_prefix(r).filter(|x| !x.is_empty()))
        {
            return is(rest);
        }
        if w == "tee" {
            let mut j = i + 1;
            while t.get(j).is_some_and(|x| x.starts_with('-')) {
                j += 1;
            }
            return t.get(j).is_some_and(|n| is(n));
        }
        false
    })
}

/// The indices of the steps whose `run:` writes `file`.
fn writers(st: &[Y], file: &str) -> Vec<usize> {
    (0..st.len())
        .filter(|&i| {
            statements(run_of(&st[i]))
                .iter()
                .any(|x| stmt_writes(x, file))
        })
        .collect()
}

/// The index of the first step matching `f`, named `what` in the failure.
fn step_idx(job_id: &str, st: &[Y], what: &str, f: impl Fn(&Y) -> bool) -> usize {
    (0..st.len())
        .find(|&i| f(&st[i]))
        .unwrap_or_else(|| panic!("job `{job_id}` has no step that {what}"))
}

const GATE_CMD: &str = "cargo xtask ci --keep-going --report gate-report.json";

fn gate_idx(id: &str, st: &[Y]) -> usize {
    step_idx(id, st, "runs the gate", |s| run_of(s).contains(GATE_CMD))
}

fn upload_idx(id: &str, st: &[Y]) -> usize {
    step_idx(id, st, "uses actions/upload-artifact", |s| {
        uses_of(s).starts_with("actions/upload-artifact@")
    })
}

/// The upload step's `path:` list, one entry per non-empty line.
fn upload_paths(st: &[Y], up: usize) -> Vec<String> {
    st[up]
        .at(&["with", "path"])
        .and_then(Y::str)
        .unwrap_or("")
        .lines()
        .map(|l| strip_comment(l).trim().to_owned())
        .filter(|l| !l.is_empty())
        .collect()
}

/// The set of a step's conditions: its `if:` (without `${{ }}` and whitespace) split at
/// top-level `&&`, sorted; no `if:` is GitHub's implicit `success()`.
fn conds(s: &Y) -> Vec<String> {
    let Some(c) = s.get("if").and_then(Y::str) else {
        return vec!["success()".to_owned()];
    };
    let t = c.trim();
    let t = t
        .strip_prefix("${{")
        .and_then(|x| x.strip_suffix("}}"))
        .unwrap_or(t);
    let mut v: Vec<String> = squash(t).split("&&").map(str::to_owned).collect();
    v.sort();
    v.dedup();
    v
}

const GATE_JOBS: [&str; 3] = ["gate-macos", "gate-windows", "gate-linux"];
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

const FAILED: Ctx = Ctx {
    event: "push",
    git_ref: "refs/heads/main",
    failed: true,
    failed_step: "",
};
/// The gate step (`id: gate`) failed.
const GATE_FAILED: Ctx = Ctx {
    event: "push",
    git_ref: "refs/heads/main",
    failed: true,
    failed_step: "gate",
};
const PASSED: Ctx = Ctx {
    event: "push",
    git_ref: "refs/heads/main",
    failed: false,
    failed_step: "",
};

fn ok(event: &'static str, git_ref: &'static str) -> Ctx {
    Ctx {
        event,
        git_ref,
        failed: false,
        failed_step: "",
    }
}

/// Every `(key, value)` pair of the tree, keys only.
fn all_keys(y: &Y, out: &mut Vec<String>) {
    match y {
        Y::Map(m) => {
            for (k, v) in m {
                out.push(k.clone());
                all_keys(v, out);
            }
        }
        Y::Seq(s) => s.iter().for_each(|v| all_keys(v, out)),
        _ => {}
    }
}

fn job_ids(w: &Y) -> Vec<String> {
    match w.get("jobs") {
        Some(Y::Map(m)) => m.iter().map(|(k, _)| k.clone()).collect(),
        _ => Vec::new(),
    }
}

// ---- G-1: the probe's bound as behaviour (T-070 review round 2, finding 1) ----------------------

#[test]
fn g1_probe_with_a_kiln_that_never_exits_ends_within_135_s_and_records_the_kill() {
    let w = workflow();
    let st = steps(job(&w, "window-probe"));
    let p = step_idx(
        "window-probe",
        st,
        "runs kiln --root … --script … into window-probe.txt",
        |s| {
            let r = run_of(s);
            r.contains("--root") && r.contains("--script") && r.contains("window-probe.txt")
        },
    );
    let b = bench("probe");
    // A kiln that never exits and ignores SIGTERM: only a SIGKILL ends it.
    let k = b.tree.join("target/debug");
    std::fs::create_dir_all(&k).unwrap();
    std::fs::write(
        k.join("kiln"),
        "#!/usr/bin/env bash\ntrap '' TERM\nwhile :; do sleep 1; done\n",
    )
    .unwrap();
    let rt = b.dir.join("runner-temp");
    std::fs::create_dir_all(&rt).unwrap();
    let mut env: Vec<(&str, String)> = vec![("RUNNER_TEMP", sh_path(&rt))];
    let keys: Vec<(String, String)> = match st[p].get("env") {
        Some(Y::Map(m)) => m
            .iter()
            .map(|(k, v)| {
                let v = v.str().unwrap_or("");
                let val = if squash(v).contains("steps.") {
                    "success".to_owned()
                } else if v.contains("${{") {
                    "fake-os".to_owned()
                } else {
                    v.to_owned()
                };
                (k.clone(), val)
            })
            .collect(),
        _ => Vec::new(),
    };
    let keys: Vec<(&'static str, String)> = keys
        .into_iter()
        .map(|(k, v)| (&*Box::leak(k.into_boxed_str()), v))
        .collect();
    env.extend(keys);
    // The fake's own `chmod` (Bench::run makes `bin/*` executable; kiln is in the tree).
    let chmod = run_to(
        Command::new(find_bash())
            .arg("-c")
            .arg("chmod +x \"$1\"")
            .arg("_")
            .arg(sh_path(&k.join("kiln"))),
        60,
    );
    assert_eq!(
        chmod.code,
        Some(0),
        "chmod of the fake kiln: {}",
        chmod.stderr
    );
    let t0 = Instant::now();
    let o = b.run(run_of(&st[p]), &env, 240);
    let secs = t0.elapsed().as_secs_f64();
    assert!(
        secs <= 135.0,
        "the probe step took {secs:.1} s with a kiln that never exits (bound 120 s + 15 s)"
    );
    assert!(
        secs >= 115.0,
        "the probe step ended after {secs:.1} s, before the 120 s bound: did kiln run?\n{}",
        o.stdout
    );
    assert_ne!(
        o.code,
        Some(0),
        "the probe step's status is kiln's, and kiln was killed"
    );
    let rec = std::fs::read_to_string(b.tree.join("window-probe.txt")).unwrap_or_default();
    assert!(
        rec.lines().any(|l| l.to_ascii_lowercase().contains("kill")),
        "window-probe.txt does not record the kill:\n{rec}"
    );
    std::fs::remove_dir_all(&b.dir).unwrap();
}

// ---- G-2: every recording step ends with exit status 0 ---------------------------------------

/// The recording steps: in every job but `window-probe` (whose probe step's status is kiln's),
/// each step with a `run:` that runs after an earlier step (or the gate step) failed.
fn recording_steps(w: &Y) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    for id in job_ids(w) {
        if id == "window-probe" {
            continue;
        }
        let st = steps(job(w, &id));
        for (i, s) in st.iter().enumerate() {
            if !run_of(s).is_empty()
                && (runs(s.get("if"), FAILED) || runs(s.get("if"), GATE_FAILED))
            {
                out.push((id.clone(), i));
            }
        }
    }
    out
}

fn assert_recording_steps_present(w: &Y, rec: &[(String, usize)]) {
    for id in GATE_JOBS {
        let st = steps(job(w, id));
        let names: Vec<&str> = rec
            .iter()
            .filter(|(j, _)| j == id)
            .map(|(_, i)| st[*i].get("name").and_then(Y::str).unwrap_or(""))
            .collect();
        for f in [
            "gate-report.json",
            "core-crates.txt",
            "test-output.txt",
            "du -sh",
        ] {
            assert!(
                rec.iter()
                    .any(|(j, i)| j == id && run_of(&st[*i]).contains(f)),
                "job `{id}`: no recording step mentions `{f}` (found {names:?})"
            );
        }
    }
    assert!(
        rec.iter().any(|(j, _)| j == "window-linux"),
        "window-linux has no recording step (the package lines)"
    );
}

#[test]
fn g2_every_recording_step_exits_0_when_every_command_it_calls_fails() {
    let w = workflow();
    let rec = recording_steps(&w);
    assert_recording_steps_present(&w, &rec);
    for (n, (id, i)) in rec.iter().enumerate() {
        let st = steps(job(&w, id));
        let name = st[*i].get("name").and_then(Y::str).unwrap_or("");
        let b = bench(&format!("rec-{n}"));
        for cmd in [
            "cargo",
            "df",
            "du",
            "dpkg-query",
            "ldconfig",
            "xvfb-run",
            "Xvfb",
            "sudo",
        ] {
            b.fake(
                cmd,
                "echo \"fake $0 $*: failing\"\necho 'error: fake failure' >&2\nexit 101",
            );
        }
        let o = b.run(run_of(&st[*i]), &[], 120);
        assert_eq!(
            o.code,
            Some(0),
            "job `{id}`: the recording step `{name}` does not end with status 0 when its commands fail:\n{}\n{}",
            o.stdout,
            o.stderr
        );
        std::fs::remove_dir_all(&b.dir).unwrap();
    }
}

#[test]
fn g2_every_recording_step_ends_with_exit_0() {
    let w = workflow();
    let rec = recording_steps(&w);
    assert_recording_steps_present(&w, &rec);
    for (id, i) in &rec {
        let st = steps(job(&w, id));
        let name = st[*i].get("name").and_then(Y::str).unwrap_or("");
        let sts = statements(run_of(&st[*i]));
        assert_eq!(
            sts.last().map(String::as_str),
            Some("exit 0"),
            "job `{id}`: the recording step `{name}` does not end with `exit 0`"
        );
    }
}

// ---- G-3: no continue-on-error, every action pinned -------------------------------------------

#[test]
fn g3_no_continue_on_error_and_every_uses_pinned_by_sha() {
    let text = subject(WORKFLOW);
    let w = parse(&text);
    let mut keys = Vec::new();
    all_keys(&w, &mut keys);
    assert!(
        !keys.iter().any(|k| k == "continue-on-error"),
        "the workflow has a `continue-on-error` key"
    );
    for (n, l) in text.lines().enumerate() {
        assert!(
            !strip_comment(l).contains("continue-on-error"),
            "{WORKFLOW}:{} mentions `continue-on-error` outside a comment",
            n + 1
        );
    }
    let mut uses = 0;
    for id in job_ids(&w) {
        for s in steps(job(&w, &id)) {
            let u = uses_of(s);
            if u.is_empty() {
                continue;
            }
            uses += 1;
            let pinned = u.split_once('@').is_some_and(|(a, sha)| {
                a.contains('/')
                    && sha.len() == 40
                    && sha
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            });
            assert!(
                pinned,
                "job `{id}`: `uses: {u}` is not pinned by a 40-hex sha"
            );
        }
    }
    let raw = text
        .lines()
        .filter(|l| {
            strip_comment(l)
                .trim_start()
                .trim_start_matches("- ")
                .starts_with("uses:")
        })
        .count();
    assert_eq!(uses, raw, "every `uses:` line was read as a step's `uses`");
    assert!(uses > 0, "no `uses:` found");
}

// ---- G-4: the window jobs keep to their own records ---------------------------------------------

#[test]
fn g4_window_jobs_write_and_upload_only_their_own_records() {
    let w = workflow();
    for (id, own) in [
        ("window-linux", "window-procedure.txt"),
        ("window-probe", "window-probe.txt"),
    ] {
        let st = steps(job(&w, id));
        for f in [
            "gate-report.json",
            "core-crates.txt",
            "test-list.txt",
            "test-output.txt",
            "workspace-tests.txt",
            "windows-tests.txt",
        ] {
            assert_eq!(
                writers(st, f),
                Vec::<usize>::new(),
                "job `{id}`: steps writing the gate jobs' {f}"
            );
        }
        let ups: Vec<usize> = (0..st.len())
            .filter(|&i| uses_of(&st[i]).starts_with("actions/upload-artifact@"))
            .collect();
        assert_eq!(ups.len(), 1, "job `{id}`: one upload step");
        assert_eq!(
            upload_paths(st, ups[0]),
            [own],
            "job `{id}`: the upload's path list"
        );
        assert!(
            runs(st[ups[0]].get("if"), FAILED) && runs(st[ups[0]].get("if"), PASSED),
            "job `{id}`: the upload does not run whatever the earlier steps' result"
        );
    }
}

// ---- T-073's step and its pinned command ---------------------------------------------------------

#[test]
fn guard_t073_step_keeps_its_command_file_and_condition() {
    const CMD: &str = "cargo test --locked --no-fail-fast -p kiln-workspace --lib --test acceptance_t073 \
         --test guard_t073 -- --show-output";
    let w = workflow();
    for id in GATE_JOBS {
        let st = steps(job(&w, id));
        let hits: Vec<usize> = (0..st.len())
            .filter(|&i| {
                statements(run_of(&st[i]))
                    .iter()
                    .any(|x| invokes_cargo_test(x) && squash(x).contains(&squash(CMD)))
            })
            .collect();
        if id != "gate-windows" {
            assert_eq!(hits, Vec::<usize>::new(), "job `{id}` runs T-073's command");
            continue;
        }
        assert_eq!(hits.len(), 1, "job `{id}`: one step runs `{CMD}`");
        let s = &st[hits[0]];
        assert_eq!(
            writers(st, "workspace-tests.txt"),
            hits,
            "job `{id}`: T-073's step writes workspace-tests.txt"
        );
        assert_eq!(
            conds(s),
            ["always()"],
            "job `{id}`: T-073's step's conditions"
        );
        let sts = statements(run_of(s));
        assert_eq!(
            sts.first().map(String::as_str),
            Some("set +e"),
            "job `{id}`: T-073's step starts with `set +e`"
        );
        assert_eq!(
            sts.last().map(String::as_str),
            Some("exit 0"),
            "job `{id}`: T-073's step ends with `exit 0`"
        );
        let (gate, up) = (gate_idx(id, st), upload_idx(id, st));
        assert!(
            gate < hits[0] && hits[0] < up,
            "job `{id}`: T-073's step is not between gate and upload"
        );
        assert!(
            upload_paths(st, up)
                .iter()
                .any(|p| p == "workspace-tests.txt"),
            "job `{id}`: the upload lacks workspace-tests.txt"
        );
    }
}

// ---- the jobs, the counted ones among them, and their colour ---------------------------------------

#[test]
fn guard_jobs_labels_conditions_and_the_gate_step_decide_the_colour() {
    let w = workflow();
    let mut ids = job_ids(&w);
    ids.sort();
    assert_eq!(
        ids,
        [
            "gate-linux",
            "gate-macos",
            "gate-windows",
            "window-linux",
            "window-probe"
        ],
        "the workflow's jobs"
    );
    for (id, label) in [
        ("gate-macos", "macos-15"),
        ("gate-windows", "windows-2025"),
        ("gate-linux", "ubuntu-24.04"),
        ("window-linux", "ubuntu-24.04"),
    ] {
        assert_eq!(
            job(&w, id).get("runs-on").and_then(Y::str),
            Some(label),
            "runs-on of `{id}`"
        );
    }
    let probe = job(&w, "window-probe");
    let mut legs = probe
        .at(&["strategy", "matrix", "os"])
        .map(Y::strs)
        .unwrap_or_default();
    legs.sort();
    assert_eq!(legs, ["macos-15", "windows-2025"], "window-probe's legs");
    assert_eq!(
        probe.at(&["strategy", "fail-fast"]).and_then(Y::str),
        Some("false"),
        "window-probe: one leg must not cancel the other"
    );
    let events = [
        ("push", "refs/heads/main"),
        ("push", "refs/heads/ci/candidate"),
        ("schedule", "refs/heads/main"),
        ("workflow_dispatch", "refs/heads/main"),
    ];
    for (e, r) in events {
        for id in ["gate-macos", "gate-windows", "window-linux", "window-probe"] {
            assert!(
                runs(job(&w, id).get("if"), ok(e, r)),
                "job `{id}` does not run for {e} on {r}"
            );
        }
        assert_eq!(
            runs(job(&w, "gate-linux").get("if"), ok(e, r)),
            !(e == "push" && r == "refs/heads/main"),
            "gate-linux for {e} on {r}"
        );
    }
    // Pinned (a): a gate job's colour is its gate step's exit status.
    for id in GATE_JOBS {
        let st = steps(job(&w, id));
        let g = gate_idx(id, st);
        assert_eq!(
            st[g].get("id").and_then(Y::str),
            Some("gate"),
            "job `{id}`: the gate step's id"
        );
        assert!(
            st[g].get("if").is_none(),
            "job `{id}`: the gate step has an `if:`"
        );
        assert_eq!(
            statements(run_of(&st[g])),
            [GATE_CMD],
            "job `{id}`: the gate step runs the gate and nothing else"
        );
    }
}
