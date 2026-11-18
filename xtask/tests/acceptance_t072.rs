//! T-072 acceptance tests (independent test author, run R-352), written from
//! `tasks/T-072-gate-green-on-macos.md` alone, under the director's rulings Q-3 A1, Q-4 A, Q-5 A
//! and Q-6 A (D-254) and the note of 11:20Z (AC-9 checks `--show-output`).
//!
//! - AC-1: each gate job has exactly one step writing `test-list.txt`, running
//!   `cargo test --workspace --locked --no-fail-fast -- --list` between the gate and the upload,
//!   whose set of conditions is exactly {`always()`}; the upload lists are exact; Pinned (c)'s
//!   order; the list step run with a fake `cargo` records its status and prints the binaries.
//! - AC-2: no line limiter in the list and output steps; the full-output step's printer run on
//!   a synthetic failing output prints every failing block (header + first 20 lines), every
//!   compile-error and `test result:` line, and an aborted binary's lines.
//! - AC-4: each core-crates label equals the command it runs.
//! - AC-5: the ledger equals main's plus `incidents/` (`any`); `paths-ignore` derived from it.
//! - AC-9: `gate-windows` alone has the general reported step; run with a fake `cargo` in a
//!   scratch tree of three test files (one failing) it reports each with its status.
//! - Interface (3), (4)(3), (5): the sentences of `docs/development.md` "CI".
//!
//! Steps are identified by the file they write (AC preamble, D-249). Scripts run in `bash`
//! (Git Bash on Windows; none found is a failure naming why); no platform cfg.
//! No YAML crate: the reader below is `acceptance_t070.rs`'s, copied unchanged.
#![cfg(test)]

const SCRATCH: &str = "t072-acc";

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

// ---- fakes ------------------------------------------------------------------------------------

/// A fake `cargo`: appends its arguments (one line, `$*`) to `$FAKE_LOG`, prints `$FAKE_OUT`
/// if set, and exits `$FAKE_RC`; with `$FAKE_FAIL` set it exits 101 when one argument equals it
/// and 0 otherwise.
const FAKE_CARGO: &str = r#"printf '%s\n' "$*" >> "$FAKE_LOG"
if [ -n "${FAKE_OUT:-}" ]; then cat "$FAKE_OUT"; fi
if [ -n "${FAKE_FAIL:-}" ]; then
  for a in "$@"; do
    if [ "$a" = "$FAKE_FAIL" ]; then echo "test result: FAILED. 0 passed; 1 failed"; exit 101; fi
  done
  echo "test result: ok. 1 passed; 0 failed"
  exit 0
fi
exit "${FAKE_RC:-0}""#;

/// A bench with the fake `cargo`, the env it reads, and `out` as what it prints.
fn cargo_bench(name: &str, out: &str, rc: u8) -> (Bench, Vec<(&'static str, String)>) {
    let b = bench(name);
    b.fake("cargo", FAKE_CARGO);
    let out_file = b.dir.join("fake-out.txt");
    std::fs::write(&out_file, out).unwrap();
    let env = vec![
        ("FAKE_LOG", sh_path(&b.dir.join("cargo.log"))),
        ("FAKE_OUT", sh_path(&out_file)),
        ("FAKE_RC", rc.to_string()),
    ];
    (b, env)
}

fn cargo_log(b: &Bench) -> Vec<String> {
    std::fs::read_to_string(b.dir.join("cargo.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

fn trimmed_lines(s: &str) -> Vec<String> {
    s.lines().map(|l| l.trim().to_owned()).collect()
}

const LIST_CMD: &str = "cargo test --workspace --locked --no-fail-fast -- --list";
const OUTPUT_CMD: &str = "cargo test --workspace --locked --no-fail-fast";

/// What `cargo test … -- --list` prints: two test binaries, one doc-test group and an error.
const LIST_OUT: &str = "   Compiling kiln-a v0.1.0 (crates/kiln-a)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.10s
     Running unittests src/lib.rs (target/debug/deps/kiln_a-00aa11)
alpha::one: test
alpha::two: test

2 tests, 0 benchmarks
     Running tests/zeta_list.rs (target/debug/deps/zeta_list-00bb22)
zeta_one: test

1 test, 0 benchmarks
   Doc-tests kiln_a
src/lib.rs - f (line 3): test

1 test, 0 benchmarks
error: could not compile `kiln-b` (test \"omega_list\") due to 2 previous errors
";

/// Synthetic output of `cargo test --workspace --no-fail-fast`: compile errors, one binary
/// with `FAILING` failing tests whose blocks have 25 lines each after the header, one binary
/// that aborts (T-079 round 1's form), one green binary.
const FAILING: usize = 260;
const BLOCK: usize = 25;

fn failing_output() -> String {
    let mut s = String::new();
    s.push_str("   Compiling kiln-a v0.1.0 (crates/kiln-a)\n");
    s.push_str("error[E0433]: failed to resolve: could not find `unix` in `os`\n");
    s.push_str("error[E0599]: no method named `is_fifo` found for struct `FileType`\n");
    s.push_str(
        "error: could not compile `kiln-workspace` (test \"confinement\") due to 2 previous errors\n",
    );
    s.push_str("warning: build failed, waiting for other jobs to finish...\n");
    s.push_str("     Running unittests src/lib.rs (target/debug/deps/kiln_a-00aa11)\n\n");
    s.push_str(&format!("running {FAILING} tests\n"));
    for i in 0..FAILING {
        s.push_str(&format!("test t{i:03} ... FAILED\n"));
    }
    s.push_str("\nfailures:\n\n");
    for i in 0..FAILING {
        s.push_str(&format!("---- t{i:03} stdout ----\n"));
        s.push_str(&format!(
            "thread 't{i:03}' panicked at crates/kiln-a/src/lib.rs:{i}:5:\n"
        ));
        for k in 2..=BLOCK {
            s.push_str(&format!("  detail {k} of t{i:03}\n"));
        }
        s.push('\n');
    }
    s.push_str("failures:\n");
    for i in 0..FAILING {
        s.push_str(&format!("    t{i:03}\n"));
    }
    s.push_str(&format!(
        "\ntest result: FAILED. 0 passed; {FAILING} failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s\n\n"
    ));
    s.push_str("error: test failed, to rerun pass `-p kiln-a --lib`\n");
    s.push_str("     Running tests/abort_bin.rs (target/debug/deps/abort_bin-00cc33)\n\n");
    s.push_str("running 3 tests\ntest fine_one ... ok\n\n");
    s.push_str("thread 'deep_tree' (278035) has overflowed its stack\n");
    s.push_str("fatal runtime error: stack overflow, aborting\n");
    s.push_str("error: test failed, to rerun pass `-p kiln-a --test abort_bin`\n\nCaused by:\n");
    s.push_str("  process didn't exit successfully: `target/debug/deps/abort_bin-00cc33` (signal: 6, SIGABRT: process abort signal)\n");
    s.push_str("     Running tests/green_bin.rs (target/debug/deps/green_bin-00dd44)\n\n");
    s.push_str("running 1 test\ntest green_one ... ok\n\n");
    s.push_str("test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n\n");
    s.push_str(
        "error: 2 targets failed:\n    `-p kiln-a --lib`\n    `-p kiln-a --test abort_bin`\n",
    );
    s
}

/// Runs the job's full-test-output step (the one step writing `test-output.txt`) on
/// [`failing_output`] with a fake `cargo` exiting 101; checks the invocation and the step's
/// status, returns its log as trimmed lines.
fn run_output_step(tag: &str, id: &str) -> Vec<String> {
    run_output_step_on(tag, id, &failing_output())
}

/// As [`run_output_step`] on `out`; the log's colour codes are removed before the lines are
/// compared (a printer may print a line with or without them).
fn run_output_step_on(tag: &str, id: &str, out: &str) -> Vec<String> {
    let w = workflow();
    let st = steps(job(&w, id));
    let hits = writers(st, "test-output.txt");
    assert_eq!(
        hits.len(),
        1,
        "job `{id}`: steps writing test-output.txt: {hits:?}"
    );
    let (b, env) = cargo_bench(&format!("output-{tag}-{id}"), out, 101);
    let o = b.run(run_of(&st[hits[0]]), &env, 120);
    assert_eq!(
        o.code,
        Some(0),
        "job `{id}`: the full-test-output step must end with status 0 whatever cargo's (recorded, not used):\n{}",
        o.stderr
    );
    assert_eq!(
        cargo_log(&b),
        ["test --workspace --locked --no-fail-fast"],
        "job `{id}`: the full-test-output step's cargo invocations"
    );
    let got = std::fs::read_to_string(b.tree.join("test-output.txt")).unwrap_or_default();
    assert!(
        got.contains(out),
        "job `{id}`: test-output.txt does not hold cargo's whole output"
    );
    let lines = trimmed_lines(&strip_ansi(&o.stdout));
    std::fs::remove_dir_all(&b.dir).unwrap();
    lines
}

/// `s` without ANSI escape sequences (`ESC [ … letter`).
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\u{1b}' && it.peek() == Some(&'[') {
            it.next();
            for d in it.by_ref() {
                if d.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Cargo's colours under `CARGO_TERM_COLOR=always` (the workflow's `env`): status words bold
/// green, `error` bold red, then the message bold.
fn colour(text: &str) -> String {
    text.lines()
        .map(|l| {
            for w in ["Running", "Doc-tests", "Compiling", "Finished"] {
                if l.trim_start().starts_with(w) {
                    let lead = &l[..l.len() - l.trim_start().len()];
                    let rest = &l.trim_start()[w.len()..];
                    return format!("\u{1b}[1m\u{1b}[92m{lead}{w}\u{1b}[0m{rest}");
                }
            }
            if let Some(rest) = l.strip_prefix("error") {
                return format!("\u{1b}[1m\u{1b}[91merror\u{1b}[0m\u{1b}[1m{rest}\u{1b}[0m");
            }
            if let Some(rest) = l.strip_prefix("test result: FAILED") {
                return format!("test result: \u{1b}[31mFAILED\u{1b}[0m{rest}");
            }
            if let Some(rest) = l.strip_prefix("test result: ok") {
                return format!("test result: \u{1b}[32mok\u{1b}[0m{rest}");
            }
            l.to_owned()
        })
        .map(|l| l + "\n")
        .collect()
}

/// The 25 lines after the header of a failing block that quotes a step's log (the model is
/// CI round 1 of this task, `tasks/ci-runs/T-072-round-1.md`): binary-heading-shaped lines
/// (`     Running …`, `   Doc-tests …`) inside an open block.
fn quoting_block() -> Vec<String> {
    let mut v: Vec<String> = [
        "thread 'quoting_one' (184487) panicked at xtask/tests/acceptance_t072.rs:966:17:",
        "job `gate-macos`: no line `TEST LIST: exit status 101`:",
        "     Running unittests src/lib.rs (target/debug/deps/quoted_a-00ee55)",
        "     Running tests/quoted_list.rs (target/debug/deps/quoted_list-00ff66)",
        "   Doc-tests quoted_a",
        "error: could not compile `quoted-b` (test \"quoted_omega\") due to 2 previous errors",
        "TEST LIST: exit status 101 (recorded; the gate step decides the colour)",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    for k in v.len() + 1..=BLOCK {
        v.push(format!("  quoted detail {k}"));
    }
    v
}

/// One binary with two failing tests: `quoting_one`, whose block is [`quoting_block`], and
/// `after_quote`, whose block follows it; then a green binary.
fn quoting_output() -> String {
    let mut s = String::new();
    s.push_str("     Running tests/quoting.rs (target/debug/deps/quoting-00ab12)\n\n");
    s.push_str("running 2 tests\ntest quoting_one ... FAILED\ntest after_quote ... FAILED\n\nfailures:\n\n");
    s.push_str("---- quoting_one stdout ----\n");
    for l in quoting_block() {
        s.push_str(&l);
        s.push('\n');
    }
    s.push_str("\n---- after_quote stdout ----\n");
    s.push_str("thread 'after_quote' panicked at xtask/tests/quoting.rs:9:5:\n");
    s.push_str("  after detail 2\n\n");
    s.push_str("failures:\n    quoting_one\n    after_quote\n\n");
    s.push_str("test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n\n");
    s.push_str("error: test failed, to rerun pass `-p xtask --test quoting`\n");
    s.push_str("     Running tests/green_after.rs (target/debug/deps/green_after-00cd34)\n\n");
    s.push_str("running 1 test\ntest green_two ... ok\n\n");
    s.push_str("test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n\n");
    s.push_str("error: 1 target failed:\n    `-p xtask --test quoting`\n");
    s
}

// ---- AC-1: the test list --------------------------------------------------------------------

#[test]
fn ac1_each_gate_job_has_one_always_test_list_step_between_gate_and_upload() {
    let w = workflow();
    for id in GATE_JOBS {
        let st = steps(job(&w, id));
        let hits = writers(st, "test-list.txt");
        assert_eq!(
            hits.len(),
            1,
            "job `{id}`: exactly one step must write test-list.txt (steps: {hits:?})"
        );
        let i = hits[0];
        let sts = statements(run_of(&st[i]));
        assert!(
            sts.iter().any(|x| invokes_cargo_test(x)
                && squash(x).contains(&squash(LIST_CMD))
                && stmt_writes(x, "test-list.txt")),
            "job `{id}`: the test-list step does not run `{LIST_CMD} > test-list.txt`: {sts:?}"
        );
        let (gate, up) = (gate_idx(id, st), upload_idx(id, st));
        assert!(
            gate < i && i < up,
            "job `{id}`: the test-list step (#{i}) is not after the gate (#{gate}) and before the upload (#{up})"
        );
        assert_eq!(
            conds(&st[i]),
            ["always()"],
            "job `{id}`: the test-list step's set of conditions must be exactly {{always()}}"
        );
    }
}

#[test]
fn ac1_each_upload_lists_exactly_the_jobs_evidence_files() {
    let w = workflow();
    for id in GATE_JOBS {
        let st = steps(job(&w, id));
        let mut got = upload_paths(st, upload_idx(id, st));
        got.sort();
        let mut want = vec![
            "core-crates.txt",
            "gate-report.json",
            "test-list.txt",
            "test-output.txt",
        ];
        if id == "gate-windows" {
            want.extend(["windows-tests.txt", "workspace-tests.txt"]);
        }
        want.sort();
        assert_eq!(got, want, "job `{id}`: the upload's path list");
    }
}

#[test]
fn ac1_evidence_steps_in_the_pinned_order() {
    // Pinned (c): gate, report, core crates, [T-073's step,] test list, [(6),] full test output
    // (gate failed), disk, upload.
    let w = workflow();
    for id in GATE_JOBS {
        let st = steps(job(&w, id));
        let one = |f: &str| {
            let h = writers(st, f);
            assert_eq!(h.len(), 1, "job `{id}`: steps writing {f}: {h:?}");
            h[0]
        };
        let gate = gate_idx(id, st);
        let report = step_idx(id, st, "prints gate-report.json", |s| {
            let r = run_of(s);
            r.contains("cat gate-report.json") && !r.contains(GATE_CMD)
        });
        let core = one("core-crates.txt");
        let list = one("test-list.txt");
        let out = one("test-output.txt");
        let disk = step_idx(id, st, "prints the disk lines", |s| {
            run_of(s).contains("df -h")
        });
        let up = upload_idx(id, st);
        let order: Vec<(&str, usize)> = if id == "gate-windows" {
            vec![
                ("gate", gate),
                ("report", report),
                ("core crates", core),
                ("T-073's step", one("workspace-tests.txt")),
                ("test list", list),
                ("general Windows step", one("windows-tests.txt")),
                ("full test output", out),
                ("disk", disk),
                ("upload", up),
            ]
        } else {
            vec![
                ("gate", gate),
                ("report", report),
                ("core crates", core),
                ("test list", list),
                ("full test output", out),
                ("disk", disk),
                ("upload", up),
            ]
        };
        assert!(
            order.windows(2).all(|p| p[0].1 < p[1].1),
            "job `{id}`: steps out of Pinned (c)'s order: {order:?}"
        );
    }
}

#[test]
fn ac1_the_list_step_runs_the_list_records_its_status_and_prints_the_binaries() {
    let w = workflow();
    for id in GATE_JOBS {
        let st = steps(job(&w, id));
        let hits = writers(st, "test-list.txt");
        assert_eq!(
            hits.len(),
            1,
            "job `{id}`: steps writing test-list.txt: {hits:?}"
        );
        let (b, env) = cargo_bench(&format!("list-{id}"), LIST_OUT, 101);
        let o = b.run(run_of(&st[hits[0]]), &env, 120);
        assert_eq!(
            o.code,
            Some(0),
            "job `{id}`: the list step must end with status 0 whatever cargo's:\n{}",
            o.stderr
        );
        assert_eq!(
            cargo_log(&b),
            ["test --workspace --locked --no-fail-fast -- --list"],
            "job `{id}`: the list step's cargo invocations (it runs no test)"
        );
        let file = std::fs::read_to_string(b.tree.join("test-list.txt")).unwrap_or_default();
        assert!(
            file.contains(LIST_OUT),
            "job `{id}`: test-list.txt does not hold cargo's whole list:\n{file}"
        );
        let log = trimmed_lines(&o.stdout);
        let mut last = 0;
        for want in LIST_OUT.lines().map(str::trim).filter(|l| {
            l.starts_with("Running ") || l.starts_with("Doc-tests ") || l.starts_with("error")
        }) {
            let at = log.iter().position(|l| l == want).unwrap_or_else(|| {
                panic!(
                    "job `{id}`: the list step's log lacks `{want}`:\n{}",
                    o.stdout
                )
            });
            last = last.max(at);
        }
        let status = log
            .iter()
            .position(|l| l == "TEST LIST: exit status 101")
            .unwrap_or_else(|| {
                panic!(
                    "job `{id}`: no line `TEST LIST: exit status 101`:\n{}",
                    o.stdout
                )
            });
        assert!(
            status > last,
            "job `{id}`: `TEST LIST: exit status 101` is not printed after the binaries' lines"
        );
        std::fs::remove_dir_all(&b.dir).unwrap();
    }
}

// ---- AC-2: no line cap; the printer ------------------------------------------------------------

/// The line limiters of a statement's pipeline segments: `head`, `tail`, `grep -m`/`--max-count`,
/// a `sed` that quits or prints a numeric range, an `awk` comparing `NR` with a number.
fn line_limiters(st: &str) -> Vec<String> {
    let mut out = Vec::new();
    for seg in st.replace("||", "\u{1}").split('|') {
        let seg = seg.replace('\u{1}', "||");
        let w: Vec<&str> = seg.split_whitespace().collect();
        let Some(cmd) = w.first().map(|c| c.trim_start_matches(['(', '{'])) else {
            continue;
        };
        let limited = match cmd {
            "head" | "tail" => true,
            "grep" | "egrep" => w.iter().any(|a| {
                *a == "--max-count"
                    || a.starts_with("--max-count=")
                    || (a.starts_with("-m") && !a.starts_with("--"))
                    || (a.starts_with('-')
                        && !a.starts_with("--")
                        && a.contains('m')
                        && a.chars().skip(1).all(|c| c.is_ascii_alphabetic()))
            }),
            "sed" => w.iter().skip(1).any(|a| {
                let a = a.trim_matches(|c| c == '\'' || c == '"');
                let digits = a.trim_end_matches(['p', 'q', 'Q', 'd']);
                (a.ends_with('q') || a.ends_with('Q') || a.ends_with('p'))
                    && !digits.is_empty()
                    && digits
                        .chars()
                        .all(|c| c.is_ascii_digit() || c == ',' || c == '$')
            }),
            "awk" | "gawk" | "mawk" => {
                let s = squash(&seg);
                ["NR<", "NR>", "NR==", "NR<=", "NR>="].iter().any(|p| {
                    s.match_indices(p).any(|(i, _)| {
                        s[i + p.len()..]
                            .trim_start_matches('=')
                            .starts_with(|c: char| c.is_ascii_digit())
                    })
                })
            }
            _ => false,
        };
        if limited {
            out.push(seg.trim().to_owned());
        }
    }
    out
}

#[test]
fn ac2_no_command_in_the_list_or_output_steps_limits_output_by_lines() {
    let w = workflow();
    for id in GATE_JOBS {
        let st = steps(job(&w, id));
        let list = writers(st, "test-list.txt");
        let out = writers(st, "test-output.txt");
        assert!(
            !list.is_empty() && !out.is_empty(),
            "job `{id}`: a step writing test-list.txt ({list:?}) and one writing test-output.txt ({out:?}) must exist"
        );
        let mut found: Vec<String> = Vec::new();
        for i in list.into_iter().chain(out) {
            for x in statements(run_of(&st[i])) {
                found.extend(line_limiters(&x));
            }
        }
        assert_eq!(
            found,
            Vec::<String>::new(),
            "job `{id}`: commands limiting output by lines in the list/output steps"
        );
    }
}

#[test]
fn ac2_the_printer_prints_every_failing_tests_header_and_its_first_20_lines() {
    for id in GATE_JOBS {
        let log = run_output_step("blocks", id);
        let has = |l: &str| log.iter().any(|x| x == l);
        for i in 0..FAILING {
            let head = format!("---- t{i:03} stdout ----");
            assert!(
                has(&head),
                "job `{id}`: the log lacks `{head}` (no line cap)"
            );
            assert!(
                has(&format!(
                    "thread 't{i:03}' panicked at crates/kiln-a/src/lib.rs:{i}:5:"
                )),
                "job `{id}`: the log lacks line 1 after `{head}`"
            );
            for k in 2..=20 {
                let l = format!("detail {k} of t{i:03}");
                assert!(
                    has(&l),
                    "job `{id}`: the log lacks line {k} after `{head}` (`{l}`)"
                );
            }
            for k in 21..=BLOCK {
                let l = format!("detail {k} of t{i:03}");
                assert!(
                    !has(&l),
                    "job `{id}`: the log prints line {k} after `{head}`; the block is the header and the first 20 lines"
                );
            }
        }
    }
}

#[test]
fn ac2_the_printer_prints_every_compile_error_and_test_result_line() {
    for id in GATE_JOBS {
        let log = run_output_step("errors", id);
        for want in failing_output().lines().map(str::trim).filter(|l| {
            l.starts_with("error[")
                || l.starts_with("error: could not compile")
                || l.starts_with("test result:")
        }) {
            assert!(
                log.iter().any(|x| x == want),
                "job `{id}`: the log lacks `{want}`"
            );
        }
    }
}

#[test]
fn ac2_the_printer_prints_an_aborted_binarys_running_line_and_abort_lines() {
    for id in GATE_JOBS {
        let log = run_output_step("abort", id);
        for want in [
            "Running tests/abort_bin.rs (target/debug/deps/abort_bin-00cc33)",
            "thread 'deep_tree' (278035) has overflowed its stack",
            "fatal runtime error: stack overflow, aborting",
        ] {
            assert!(
                log.iter().any(|x| x == want),
                "job `{id}`: the log lacks the aborted binary's line `{want}`"
            );
        }
    }
}

#[test]
fn ac2_the_printer_keeps_heading_shaped_lines_inside_a_failing_block() {
    // Ruling u1 (D-260): while a failing block is open, a line shaped like a binary heading is
    // printed as part of the block and counts towards its 20 lines.
    let block = quoting_block();
    for id in GATE_JOBS {
        let log = run_output_step_on("quoting", id, &quoting_output());
        let head = "---- quoting_one stdout ----";
        let at = log
            .iter()
            .position(|l| l == head)
            .unwrap_or_else(|| panic!("job `{id}`: the log lacks `{head}`:\n{}", log.join("\n")));
        let want: Vec<String> = block[..20].iter().map(|l| l.trim().to_owned()).collect();
        let got: Vec<String> = log.iter().skip(at + 1).take(20).cloned().collect();
        assert_eq!(
            got, want,
            "job `{id}`: the 20 lines after `{head}` must be the block's own, heading-shaped lines included"
        );
        for l in &block[20..] {
            assert!(
                !log.iter().any(|x| x == l.trim()),
                "job `{id}`: the log prints `{}`, the block's line 21 or later",
                l.trim()
            );
        }
        for l in &block[2..5] {
            let n = log.iter().filter(|x| *x == l.trim()).count();
            assert_eq!(
                n,
                1,
                "job `{id}`: the block's heading-shaped line `{}` is printed {n} times (once, in the block)",
                l.trim()
            );
        }
        let next = "---- after_quote stdout ----";
        let j = log
            .iter()
            .position(|l| l == next)
            .unwrap_or_else(|| panic!("job `{id}`: the log lacks `{next}`"));
        assert!(
            j > at + 20
                && log[at + 21..j]
                    .iter()
                    .all(|l| !block.iter().any(|b| b.trim() == l)),
            "job `{id}`: a line of the quoting block is printed again before `{next}`:\n{}",
            log.join("\n")
        );
        assert!(
            log.iter()
                .any(|l| l == "thread 'after_quote' panicked at xtask/tests/quoting.rs:9:5:"),
            "job `{id}`: the next block's first line is missing"
        );
    }
}

#[test]
fn ac2_the_printer_reads_cargos_coloured_output() {
    // `CARGO_TERM_COLOR=always` (the workflow's env): cargo's lines start with colour codes.
    let plain = failing_output();
    let coloured = colour(&plain);
    assert_ne!(plain, coloured, "precondition: the fixture is coloured");
    for id in GATE_JOBS {
        let log = run_output_step_on("colour", id, &coloured);
        let has = |l: &str| log.iter().any(|x| x == l);
        for want in plain.lines().map(str::trim).filter(|l| {
            l.starts_with("error[")
                || l.starts_with("error: could not compile")
                || l.starts_with("test result:")
                || l.starts_with("Running tests/abort_bin.rs")
                || l.starts_with("fatal runtime error")
        }) {
            assert!(
                has(want),
                "job `{id}`: on coloured output the log lacks `{want}`"
            );
        }
        for i in [0, FAILING / 2, FAILING - 1] {
            let head = format!("---- t{i:03} stdout ----");
            assert!(
                has(&head),
                "job `{id}`: on coloured output the log lacks `{head}`"
            );
            assert!(
                has(&format!("detail 20 of t{i:03}")),
                "job `{id}`: on coloured output the log lacks line 20 after `{head}`"
            );
        }
    }
}

#[test]
fn ac1_the_list_step_prints_the_binaries_of_a_coloured_list() {
    // `CARGO_TERM_COLOR=always`: the list's `Running`/`Doc-tests`/`error` lines are coloured.
    let coloured = colour(LIST_OUT);
    let w = workflow();
    for id in GATE_JOBS {
        let st = steps(job(&w, id));
        let hits = writers(st, "test-list.txt");
        assert_eq!(
            hits.len(),
            1,
            "job `{id}`: steps writing test-list.txt: {hits:?}"
        );
        let (b, env) = cargo_bench(&format!("list-colour-{id}"), &coloured, 101);
        let o = b.run(run_of(&st[hits[0]]), &env, 120);
        assert_eq!(
            o.code,
            Some(0),
            "job `{id}`: the list step's status:\n{}",
            o.stderr
        );
        let log = trimmed_lines(&strip_ansi(&o.stdout));
        for want in LIST_OUT.lines().map(str::trim).filter(|l| {
            l.starts_with("Running ") || l.starts_with("Doc-tests ") || l.starts_with("error")
        }) {
            assert!(
                log.iter().any(|l| l == want),
                "job `{id}`: on a coloured list the log lacks `{want}`:\n{}",
                o.stdout
            );
        }
        std::fs::remove_dir_all(&b.dir).unwrap();
    }
}

// ---- AC-4: the core-crates label ---------------------------------------------------------------

#[test]
fn ac4_each_core_crates_label_equals_the_command_it_runs() {
    let w = workflow();
    for id in GATE_JOBS {
        let st = steps(job(&w, id));
        let hits = writers(st, "core-crates.txt");
        assert_eq!(
            hits.len(),
            1,
            "job `{id}`: steps writing core-crates.txt: {hits:?}"
        );
        let sts = statements(run_of(&st[hits[0]]));
        let label = sts
            .iter()
            .filter(|x| x.starts_with("echo ") && x.contains("====") && x.contains("cargo test"))
            .map(|x| {
                let arg = x["echo ".len()..].split(">>").next().unwrap_or("").trim();
                let arg = arg.trim_matches(|c| c == '"' || c == '\'');
                arg.trim_start_matches("====").trim().to_owned()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            label.len(),
            1,
            "job `{id}`: one label line `echo \"==== cargo test …\"`: {label:?}"
        );
        let cmds: Vec<String> = sts
            .iter()
            .filter(|x| invokes_cargo_test(x))
            .map(|x| {
                let c = x
                    .split(">>")
                    .next()
                    .unwrap_or("")
                    .split(" >")
                    .next()
                    .unwrap_or("");
                c.split_whitespace()
                    .map(|t| t.trim_matches(|c| c == '"' || c == '\''))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        assert_eq!(
            cmds.len(),
            1,
            "job `{id}`: one cargo test command: {cmds:?}"
        );
        assert!(
            cmds[0].contains("--no-fail-fast"),
            "job `{id}`: the core-crates command lost --no-fail-fast: `{}`",
            cmds[0]
        );
        let norm = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(
            norm(&label[0]),
            norm(&cmds[0]),
            "job `{id}`: the core-crates label must equal the command it runs"
        );
    }
}

// ---- AC-5: incidents/ a ledger path ---------------------------------------------------------

fn ledger_entries() -> Vec<(String, String)> {
    let t: toml::Value = toml::from_str(&subject("xtask/ledger.toml"))
        .unwrap_or_else(|e| panic!("xtask/ledger.toml does not parse: {e}"));
    t.get("ledger")
        .and_then(toml::Value::as_array)
        .unwrap_or_else(|| panic!("xtask/ledger.toml has no [[ledger]]"))
        .iter()
        .map(|e| {
            (
                e.get("path")
                    .and_then(toml::Value::as_str)
                    .unwrap()
                    .to_owned(),
                e.get("mode")
                    .and_then(toml::Value::as_str)
                    .unwrap()
                    .to_owned(),
            )
        })
        .collect()
}

/// Main's ledger at the base (`b96ee55`, S-2: 9 `any`, 1 `add-only`).
const MAIN_LEDGER: [(&str, &str); 10] = [
    ("tasks/", "any"),
    ("metrics/", "any"),
    ("docs/decisions.md", "any"),
    ("docs/adr/", "any"),
    ("STATE.md", "any"),
    ("handoffs/", "any"),
    ("retrospectives/", "any"),
    ("evaluations/", "any"),
    ("methodology/", "any"),
    ("scorecard/items/", "add-only"),
];

#[test]
fn ac5_the_ledger_is_exactly_mains_plus_incidents_any() {
    let mut got = ledger_entries();
    got.sort();
    let mut want: Vec<(String, String)> = MAIN_LEDGER
        .iter()
        .map(|(p, m)| ((*p).to_owned(), (*m).to_owned()))
        .chain([("incidents/".to_owned(), "any".to_owned())])
        .collect();
    want.sort();
    assert_eq!(
        got, want,
        "the set of (path, mode) entries of xtask/ledger.toml"
    );
}

#[test]
fn ac5_paths_ignore_is_derived_from_the_ledger_and_holds_incidents() {
    let mut derived = Vec::new();
    for (p, m) in ledger_entries() {
        if m == "any" {
            let e = p.trim_end_matches('/');
            derived.push(e.to_owned());
            derived.push(format!("{e}/**"));
        }
    }
    let w = workflow();
    let got = w
        .at(&["on", "push", "paths-ignore"])
        .map(Y::strs)
        .unwrap_or_default();
    assert_eq!(
        got, derived,
        "push.paths-ignore against the list derived from xtask/ledger.toml"
    );
    let at = got.iter().position(|x| x == "incidents");
    assert!(
        at.is_some_and(|i| got.get(i + 1).map(String::as_str) == Some("incidents/**")),
        "push.paths-ignore lacks `incidents` followed by `incidents/**`: {got:?}"
    );
}

// ---- AC-9 (Q-5 A): the general reported Windows step ----------------------------------------

#[test]
fn ac9_gate_windows_alone_has_the_general_workspace_test_step() {
    let w = workflow();
    for id in GATE_JOBS {
        let st = steps(job(&w, id));
        let hits = writers(st, "windows-tests.txt");
        if id != "gate-windows" {
            assert_eq!(
                hits,
                Vec::<usize>::new(),
                "job `{id}`: steps writing windows-tests.txt"
            );
            continue;
        }
        assert_eq!(
            hits.len(),
            1,
            "job `{id}`: exactly one step must write windows-tests.txt"
        );
        let s = &st[hits[0]];
        let sts = statements(run_of(s));
        assert_eq!(
            sts.first().map(String::as_str),
            Some("set +e"),
            "job `{id}`: the step must start with `set +e`"
        );
        assert_eq!(
            sts.last().map(String::as_str),
            Some("exit 0"),
            "job `{id}`: the step must end with `exit 0`"
        );
        assert_eq!(
            conds(s),
            ["always()"],
            "job `{id}`: the step's set of conditions must be exactly {{always()}}"
        );
        let cargo: Vec<&String> = sts.iter().filter(|x| invokes_cargo_test(x)).collect();
        assert_eq!(
            cargo.len(),
            1,
            "job `{id}`: one `cargo test` statement in the loop: {cargo:?}"
        );
        let words: Vec<&str> = cargo[0].split_whitespace().collect();
        for flag in ["--locked", "--no-fail-fast", "--test"] {
            assert!(
                words.contains(&flag),
                "job `{id}`: `{}` lacks {flag}",
                cargo[0]
            );
        }
        assert!(
            squash(cargo[0]).contains("-pkiln-workspace"),
            "job `{id}`: `{}` is not `-p kiln-workspace`",
            cargo[0]
        );
        // Director's note (11:20Z): Interface (6) passes `-- --show-output`.
        let sep = words.iter().position(|w| *w == "--");
        assert!(
            sep.is_some_and(|i| words[i + 1..].contains(&"--show-output")),
            "job `{id}`: `{}` does not pass `-- --show-output`",
            cargo[0]
        );
        let up = upload_idx(id, st);
        assert!(
            upload_paths(st, up)
                .iter()
                .any(|p| p == "windows-tests.txt"),
            "job `{id}`: the upload does not include windows-tests.txt"
        );
    }
}

#[test]
fn ac9_the_windows_step_reports_every_workspace_test_target_with_its_status() {
    let w = workflow();
    let st = steps(job(&w, "gate-windows"));
    let hits = writers(st, "windows-tests.txt");
    assert_eq!(
        hits.len(),
        1,
        "gate-windows: steps writing windows-tests.txt: {hits:?}"
    );
    let (b, mut env) = cargo_bench("windows-step", "", 0);
    env.retain(|(k, _)| *k == "FAKE_LOG");
    env.push(("FAKE_FAIL", "beta_t2".to_owned()));
    let tests = b.tree.join("crates/kiln-workspace/tests");
    std::fs::create_dir_all(tests.join("support")).unwrap();
    for f in [
        "gamma_t3.rs",
        "alpha_t1.rs",
        "beta_t2.rs",
        "support/mod.rs",
        "README.md",
    ] {
        std::fs::write(tests.join(f), "// scratch\n").unwrap();
    }
    let o = b.run(run_of(&st[hits[0]]), &env, 120);
    assert_eq!(
        o.code,
        Some(0),
        "gate-windows: the step must end with status 0:\n{}",
        o.stderr
    );
    assert_eq!(
        cargo_log(&b),
        [
            "test --locked --no-fail-fast -p kiln-workspace --test alpha_t1 -- --show-output",
            "test --locked --no-fail-fast -p kiln-workspace --test beta_t2 -- --show-output",
            "test --locked --no-fail-fast -p kiln-workspace --test gamma_t3 -- --show-output",
        ],
        "gate-windows: one invocation per tests/<t>.rs, in name order (no fixed list)"
    );
    let file = std::fs::read_to_string(b.tree.join("windows-tests.txt")).unwrap_or_default();
    let status: Vec<&str> = file
        .lines()
        .filter(|l| l.starts_with("==== ") && l.contains("exit status"))
        .collect();
    assert_eq!(
        status,
        [
            "==== alpha_t1: exit status 0",
            "==== beta_t2: exit status 101",
            "==== gamma_t3: exit status 0",
        ],
        "gate-windows: the status lines of windows-tests.txt:\n{file}"
    );
    std::fs::remove_dir_all(&b.dir).unwrap();
}

// ---- Interface (3), (4)(3), (5): docs/development.md "CI" -----------------------------------------

fn ci_section() -> String {
    let t = subject("docs/development.md");
    let start = t
        .find("\n## CI\n")
        .unwrap_or_else(|| panic!("docs/development.md has no `## CI` section"));
    let rest = &t[start + 1..];
    let end = rest[3..].find("\n## ").map_or(rest.len(), |i| i + 3);
    rest[..end].to_owned()
}

#[test]
fn docs_ci_names_every_evidence_file_and_the_list_command() {
    let s = ci_section();
    for want in [
        "test-list.txt",
        "test-output.txt",
        "workspace-tests.txt",
        "windows-tests.txt",
        LIST_CMD,
        OUTPUT_CMD,
    ] {
        assert!(
            s.contains(want),
            "docs/development.md \"CI\" does not name `{want}`"
        );
    }
}

#[test]
fn docs_ci_has_no_sentence_saying_the_workspace_does_not_build_until_t073() {
    let s = ci_section().replace('\n', " ");
    let squashed = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let found: Vec<&str> = squashed
        .split(". ")
        .filter(|x| x.contains("does not build until T-073"))
        .collect();
    assert_eq!(found, Vec::<&str>::new(), "stale sentences (Interface (5))");
}

#[test]
fn docs_ci_says_scheduled_runs_start_late() {
    let s = ci_section().replace('\n', " ");
    let squashed = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let words = |x: &str| -> Vec<String> {
        x.split(|c: char| !c.is_ascii_alphanumeric())
            .map(str::to_ascii_lowercase)
            .collect()
    };
    assert!(
        squashed.split(". ").any(|x| {
            let w = words(x);
            w.iter().any(|w| w == "late")
                && w.iter().any(|w| w.starts_with("start"))
                && w.iter().any(|w| w.starts_with("schedul") || w == "nightly")
        }),
        "docs/development.md \"CI\" has no sentence saying scheduled runs start late (Interface (3))"
    );
}

#[test]
fn docs_ci_table_shows_a_green_gate_macos_with_the_inode_columns() {
    let s = ci_section();
    let at = s
        .find("Runners, wall times and disk")
        .unwrap_or_else(|| panic!("no \"Runners, wall times and disk\" paragraph"));
    let row = s[at..]
        .lines()
        .find(|l| l.starts_with("| `gate-macos` |"))
        .unwrap_or_else(|| panic!("the table has no `gate-macos` row"));
    let cells: Vec<&str> = row.trim_matches('|').split('|').map(str::trim).collect();
    assert!(
        cells.len() >= 5,
        "the gate-macos row has {} cells: {row}",
        cells.len()
    );
    assert!(
        cells[1].starts_with("green"),
        "the gate-macos row's result is not green: {row}"
    );
    let pct = cells[3]
        .split(|c: char| c.is_whitespace() || c == '`')
        .filter(|t| t.len() > 1 && t.ends_with('%'))
        .count();
    assert_eq!(
        pct, 2,
        "the gate-macos disk cell must show the record's columns as printed, inode columns included (capacity and %iused): {}",
        cells[3]
    );
}
