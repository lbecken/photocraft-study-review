//! T-021 acceptance tests through the built `kiln-cli` binary (`control --root`, `run --root`):
//! `search.find` and `search.replace` on an open document (AC-1 to AC-9, AC-7a, AC-11).
//!
//! Written by the implementer (medium tier, odd task number: no separate test author) before
//! any implementation code, from the task file alone. The oracles are built here:
//! - a naive literal scan with T-017's case-insensitive test predicate (AC-1);
//! - a naive backtracking model of T-017's iteration, anchor, capture and case-folding rules,
//!   copied from `crates/kiln-search/tests/acceptance_t017.rs` (AC-2);
//! - the spec's whole-word filter applied to the naive results (AC-3);
//! - a naive replacement with T-017's expansion rule, the naive mapping rule of
//!   `docs/protocol/cursor.md` applied to the edits from the last to the first, and undo/redo
//!   round trips (AC-4);
//! - the exact values of Pinned (g) and of AC-6, AC-7a, AC-8, AC-9, AC-11.
//!
//! No test reads `error.message`, stderr, a generated document, the command list or
//! `error_kinds`; a refusal is checked by `code` and `kind` only (golden rule 9, Pinned (f)).
//! No test asserts a time: every spawned process is killed after a timeout, which is a hang
//! detector only (D-028 (8), D-101 Q-1).
//!
//! The file uses no new API: it compiles on base and fails there by assertion (every
//! `search.*` request answers `unknown_command`). All generators are fixed-seed (SplitMix64).
#![cfg(test)]

use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

// ---------------------------------------------------------------- process helpers

/// Hang detector for every spawned `kiln-cli`: not a time assertion (D-101 Q-1).
const HANG_DETECTOR: Duration = Duration::from_secs(120);

/// A fresh, empty directory under cargo's per-target temp dir, unique per test and process.
fn fresh_dir(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("acceptance_t021")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("create temp dir");
    d
}

/// Run `program args` with `stdin` as its input; kill it after [`HANG_DETECTOR`].
fn spawn(program: &str, args: &[&str], stdin: Vec<u8>) -> Output {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut input = child.stdin.take().expect("stdin");
    // A writer thread, so a large input cannot deadlock against a full stdout pipe.
    let writer = thread::spawn(move || {
        let _ = input.write_all(&stdin);
    });
    let drain = |r: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut r = r;
            let mut buf = Vec::new();
            let _ = r.read_to_end(&mut buf);
            buf
        })
    };
    let out = drain(Box::new(child.stdout.take().expect("stdout")));
    let err = drain(Box::new(child.stderr.take().expect("stderr")));
    let deadline = Instant::now() + HANG_DETECTOR;
    let status = loop {
        if let Some(status) = child.try_wait().expect("try_wait") {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("hang detector: {program} {args:?} did not finish within {HANG_DETECTOR:?}");
        }
        thread::sleep(Duration::from_millis(5));
    };
    writer.join().expect("stdin writer");
    Output {
        status,
        stdout: out.join().expect("stdout reader"),
        stderr: err.join().expect("stderr reader"),
    }
}

fn kiln() -> &'static str {
    env!("CARGO_BIN_EXE_kiln-cli")
}

fn parse_lines(out: &Output, want: usize) -> Vec<Value> {
    let stdout = String::from_utf8(out.stdout.clone()).expect("stdout UTF-8");
    let responses: Vec<Value> = stdout
        .lines()
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("not JSON ({e}): {l}")))
        .collect();
    assert_eq!(
        responses.len(),
        want,
        "one response per request (exit {:?})",
        out.status
    );
    responses
}

// ---------------------------------------------------------------- requests and expectations

/// What the response to one request must be.
#[derive(Clone, Debug)]
enum Expect {
    /// `ok:true` with exactly this result.
    Result(Value),
    /// `ok:true`, result not checked.
    AnyOk,
    /// `ok:true` and these fields of the result equal these values.
    Fields(Vec<(&'static str, Value)>),
    /// `ok:true` with the same result as the request with this (1-based) id.
    SameAs(usize),
    /// `ok:false` with this code and kind (messages are not pinned, Pinned (f)).
    Error {
        code: &'static str,
        kind: &'static str,
    },
}

fn rejected(kind: &'static str) -> Expect {
    Expect::Error {
        code: "rejected",
        kind,
    }
}

fn schema_error() -> Expect {
    Expect::Error {
        code: "invalid_params",
        kind: "schema",
    }
}

/// A list of requests numbered 1, 2, … with their expectations.
#[derive(Default)]
struct Requests {
    lines: Vec<String>,
    expect: Vec<Expect>,
}

impl Requests {
    fn push(&mut self, command: &str, params: Value, expect: Expect) -> usize {
        let id = self.lines.len() + 1;
        self.lines
            .push(json!({"id": id, "command": command, "params": params}).to_string());
        self.expect.push(expect);
        id
    }

    fn input(&self) -> Vec<u8> {
        let mut text = self.lines.join("\n");
        text.push('\n');
        text.into_bytes()
    }

    fn any_error(&self) -> bool {
        self.expect
            .iter()
            .any(|e| matches!(e, Expect::Error { .. }))
    }

    /// Check every response against its expectation, in order; the panic names the request.
    fn check(&self, responses: &[Value]) {
        for (i, (resp, want)) in responses.iter().zip(&self.expect).enumerate() {
            let ctx = || {
                let from = i.saturating_sub(4);
                let mut shown = resp.to_string();
                if shown.len() > 2000 {
                    shown.truncate(2000);
                    shown.push_str(" …");
                }
                let mut line = self.lines[i].clone();
                if line.len() > 2000 {
                    line.truncate(2000);
                    line.push_str(" …");
                }
                let preceding: Vec<String> = self.lines[from..i]
                    .iter()
                    .map(|l| l.chars().take(300).collect())
                    .collect();
                format!(
                    "request #{}: {line}\n  -> {shown}\n  preceding requests:\n    {}",
                    i + 1,
                    preceding.join("\n    ")
                )
            };
            assert_eq!(resp["id"], json!(i + 1), "{}", ctx());
            match want {
                Expect::Result(r) => {
                    assert_eq!(resp["ok"], json!(true), "{}", ctx());
                    assert!(&resp["result"] == r, "expected result {r}\n{}", ctx());
                }
                Expect::AnyOk => assert_eq!(resp["ok"], json!(true), "{}", ctx()),
                Expect::Fields(fields) => {
                    assert_eq!(resp["ok"], json!(true), "{}", ctx());
                    for (k, v) in fields {
                        assert_eq!(&resp["result"][*k], v, "field {k}\n{}", ctx());
                    }
                }
                Expect::SameAs(id) => {
                    assert_eq!(resp["ok"], json!(true), "{}", ctx());
                    assert_eq!(
                        resp["result"],
                        responses[id - 1]["result"],
                        "same result as request #{id}\n{}",
                        ctx()
                    );
                }
                Expect::Error { code, kind } => {
                    assert_eq!(resp["ok"], json!(false), "{}", ctx());
                    assert_eq!(resp["error"]["code"], json!(code), "{}", ctx());
                    assert_eq!(resp["error"]["kind"], json!(kind), "{}", ctx());
                }
            }
        }
    }

    /// Send all requests to one `kiln-cli control --root root` session; check responses.
    fn run_control(&self, root: &Path) -> Vec<Value> {
        let out = spawn(
            kiln(),
            &["control", "--root", root.to_str().expect("utf-8 path")],
            self.input(),
        );
        let responses = parse_lines(&out, self.lines.len());
        self.check(&responses);
        assert_eq!(out.status.code(), Some(0), "control exit code");
        responses
    }

    /// Run as a `kiln-cli run` script with `--root root`; check exit code and responses.
    fn run_script(&self, work: &Path, root: &Path) -> Vec<Value> {
        let script = work.join("s.jsonl");
        std::fs::write(&script, self.input()).expect("write script");
        let out = spawn(
            kiln(),
            &[
                "run",
                script.to_str().expect("utf-8 path"),
                "--root",
                root.to_str().expect("utf-8 path"),
            ],
            Vec::new(),
        );
        let responses = parse_lines(&out, self.lines.len());
        self.check(&responses);
        let want_code = if self.any_error() { 3 } else { 0 };
        assert_eq!(out.status.code(), Some(want_code), "run exit code");
        responses
    }
}

/// A root holding `a.txt` with `text`, and a session whose document 1 is that file.
fn session(name: &str, text: &str) -> (PathBuf, Requests) {
    let work = fresh_dir(name);
    let root = work.join("root");
    std::fs::create_dir_all(&root).expect("root");
    std::fs::write(root.join("a.txt"), text).expect("write a.txt");
    let mut r = Requests::default();
    r.push("doc.open", json!({"path": "a.txt"}), Expect::AnyOk);
    (work, r)
}

fn text_result(text: &str) -> Expect {
    Expect::Result(json!({"text": text, "len_chars": text.chars().count()}))
}

fn ranges_json(ranges: &[(usize, usize)]) -> Value {
    Value::Array(
        ranges
            .iter()
            .map(|&(s, e)| json!({"start": s, "end": e}))
            .collect(),
    )
}

fn find_result(all: &[(usize, usize)], limit: usize) -> Expect {
    Expect::Result(json!({
        "matches": ranges_json(&all[..all.len().min(limit)]),
        "total": all.len(),
        "truncated": all.len() > limit,
    }))
}

// ---------------------------------------------------------------- deterministic generator

/// SplitMix64: tiny, fixed-seed, no dependency.
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..n` (`n > 0`).
    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next_u64() % n as u64).expect("fits")
    }

    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len())]
    }
}

/// AC-1 alphabet: `a b A B _ 0 é É σ Σ ς ſ s S K k \r \n` and space.
const LIT_ALPHABET: &[char] = &[
    'a', 'b', 'A', 'B', '_', '0', 'é', 'É', 'σ', 'Σ', 'ς', 'ſ', 's', 'S', 'K', 'k', '\r', '\n', ' ',
];

/// T-017's AC-1 alphabet (AC-2's regex subset).
const RE_ALPHABET: &[char] = &[
    'a', 'b', 'A', '_', '0', 'é', 'σ', 'Σ', 'ς', 'ſ', 's', 'S', 'K', 'k', '\u{212A}', '\r', '\n',
    ' ', '😀',
];

fn random_string(rng: &mut Rng, alphabet: &[char], max_len: usize) -> String {
    let n = rng.below(max_len + 1);
    (0..n).map(|_| rng.pick(alphabet)).collect()
}

/// A literal pattern: often a piece of `text` (so it matches), else 1 to 3 random chars.
fn random_literal(rng: &mut Rng, text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    if !chars.is_empty() && rng.below(2) == 0 {
        let s = rng.below(chars.len());
        let e = (s + 1 + rng.below(3)).min(chars.len());
        return chars[s..e].iter().collect();
    }
    let n = 1 + rng.below(3);
    (0..n).map(|_| rng.pick(LIT_ALPHABET)).collect()
}

// ---------------------------------------------------------------- naive literal model (AC-1)

/// T-017's test predicate: "equal, or equal lowercase, or equal uppercase".
fn ci_eq(a: char, b: char) -> bool {
    a == b || a.to_lowercase().eq(b.to_lowercase()) || a.to_uppercase().eq(b.to_uppercase())
}

/// Occurrences of `pattern` in `text`, left to right, non-overlapping.
fn naive_literal(text: &str, pattern: &str, case_sensitive: bool) -> Vec<(usize, usize)> {
    let t: Vec<char> = text.chars().collect();
    let p: Vec<char> = pattern.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while !p.is_empty() && i + p.len() <= t.len() {
        let hit = p.iter().zip(&t[i..]).all(|(&pc, &tc)| {
            if case_sensitive {
                pc == tc
            } else {
                ci_eq(pc, tc)
            }
        });
        if hit {
            out.push((i, i + p.len()));
            i += p.len();
        } else {
            i += 1;
        }
    }
    out
}

// ---------------------------------------------------------------- whole-word filter (AC-3)

fn is_word(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

fn whole_word(text: &str, ms: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let t: Vec<char> = text.chars().collect();
    ms.iter()
        .copied()
        .filter(|&(s, e)| (s == 0 || !is_word(t[s - 1])) && (e == t.len() || !is_word(t[e])))
        .collect()
}

// ---------------------------------------------------------------- naive regex model (AC-2)
// Copied from `crates/kiln-search/tests/acceptance_t017.rs` (Reading list of T-021).

type Span = Option<(usize, usize)>;

/// Every Unicode scalar value `m` with `ci_eq(m, c)`, for each `c` of the test alphabets, by
/// brute force (class ranges may hold members outside the alphabet).
fn fold_sets() -> &'static HashMap<char, Vec<char>> {
    static SETS: OnceLock<HashMap<char, Vec<char>>> = OnceLock::new();
    SETS.get_or_init(|| {
        let targets: Vec<char> = RE_ALPHABET
            .iter()
            .chain(LIT_ALPHABET.iter())
            .copied()
            .collect::<HashSet<char>>()
            .into_iter()
            .collect();
        let key = |it: &mut dyn Iterator<Item = char>| {
            let mut k = ['\0'; 4];
            for (i, c) in it.take(4).enumerate() {
                k[i] = c;
            }
            k
        };
        let tkeys: Vec<(char, [char; 4], [char; 4])> = targets
            .iter()
            .map(|&c| (c, key(&mut c.to_lowercase()), key(&mut c.to_uppercase())))
            .collect();
        let mut sets: HashMap<char, Vec<char>> = HashMap::new();
        for m in (0u32..=0x10FFFF).filter_map(char::from_u32) {
            let lk = key(&mut m.to_lowercase());
            let uk = key(&mut m.to_uppercase());
            for (c, cl, cu) in &tkeys {
                if m == *c || lk == *cl || uk == *cu {
                    sets.entry(*c).or_default().push(m);
                }
            }
        }
        sets
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    Star,
    Plus,
    Quest,
}

#[derive(Clone, Debug)]
enum Node {
    Empty,
    Lit(char),
    Any,
    Class {
        negated: bool,
        items: Vec<(char, char)>,
    },
    Bol,
    Eol,
    Group {
        capture: bool,
        inner: Box<Node>,
    },
    Concat(Vec<Node>),
    Alt(Vec<Node>),
    Repeat {
        inner: Box<Node>,
        op: Op,
        lazy: bool,
    },
}

fn nullable(n: &Node) -> bool {
    match n {
        Node::Empty | Node::Bol | Node::Eol => true,
        Node::Lit(_) | Node::Any | Node::Class { .. } => false,
        Node::Group { inner, .. } => nullable(inner),
        Node::Concat(v) => v.iter().all(nullable),
        Node::Alt(v) => v.iter().any(nullable),
        Node::Repeat { inner, op, .. } => match op {
            Op::Star | Op::Quest => true,
            Op::Plus => nullable(inner),
        },
    }
}

fn escape_char(out: &mut String, c: char) {
    match c {
        '\r' => out.push_str("\\r"),
        '\n' => out.push_str("\\n"),
        '\\' | '.' | '+' | '*' | '?' | '(' | ')' | '|' | '[' | ']' | '{' | '}' | '^' | '$'
        | '#' | '&' | '-' | '~' => {
            out.push('\\');
            out.push(c);
        }
        _ => out.push(c),
    }
}

fn print(n: &Node, out: &mut String) {
    match n {
        Node::Empty => {}
        Node::Lit(c) => escape_char(out, *c),
        Node::Any => out.push('.'),
        Node::Class { negated, items } => {
            out.push('[');
            if *negated {
                out.push('^');
            }
            for &(lo, hi) in items {
                escape_char(out, lo);
                if hi != lo {
                    out.push('-');
                    escape_char(out, hi);
                }
            }
            out.push(']');
        }
        Node::Bol => out.push('^'),
        Node::Eol => out.push('$'),
        Node::Group { capture, inner } => {
            out.push_str(if *capture { "(" } else { "(?:" });
            print(inner, out);
            out.push(')');
        }
        Node::Concat(v) => v.iter().for_each(|x| print(x, out)),
        Node::Alt(v) => {
            for (i, b) in v.iter().enumerate() {
                if i > 0 {
                    out.push('|');
                }
                print(b, out);
            }
        }
        Node::Repeat { inner, op, lazy } => {
            print(inner, out);
            out.push(match op {
                Op::Star => '*',
                Op::Plus => '+',
                Op::Quest => '?',
            });
            if *lazy {
                out.push('?');
            }
        }
    }
}

fn pattern_of(n: &Node) -> String {
    let mut s = String::new();
    print(n, &mut s);
    s
}

#[derive(Clone, Debug)]
enum Inst {
    Lit(char),
    Any,
    Class {
        negated: bool,
        items: Vec<(char, char)>,
    },
    Bol,
    Eol,
    Split(usize, usize),
    Jmp(usize),
    Save(usize),
    Match,
}

struct Program {
    insts: Vec<Inst>,
    groups: usize,
}

struct Compiler {
    insts: Vec<Inst>,
    groups: usize,
}

impl Compiler {
    fn emit(&mut self, i: Inst) -> usize {
        self.insts.push(i);
        self.insts.len() - 1
    }

    fn split(lazy: bool, body: usize, out: usize) -> Inst {
        if lazy {
            Inst::Split(out, body)
        } else {
            Inst::Split(body, out)
        }
    }

    fn node(&mut self, n: &Node) {
        match n {
            Node::Empty => {}
            Node::Lit(c) => {
                self.emit(Inst::Lit(*c));
            }
            Node::Any => {
                self.emit(Inst::Any);
            }
            Node::Class { negated, items } => {
                self.emit(Inst::Class {
                    negated: *negated,
                    items: items.clone(),
                });
            }
            Node::Bol => {
                self.emit(Inst::Bol);
            }
            Node::Eol => {
                self.emit(Inst::Eol);
            }
            Node::Group { capture, inner } => {
                if *capture {
                    // Groups are numbered by their opening parenthesis, i.e. pre-order.
                    self.groups += 1;
                    let g = self.groups;
                    self.emit(Inst::Save(2 * g));
                    self.node(inner);
                    self.emit(Inst::Save(2 * g + 1));
                } else {
                    self.node(inner);
                }
            }
            Node::Concat(v) => v.iter().for_each(|x| self.node(x)),
            Node::Alt(v) => {
                let mut jumps = Vec::new();
                for (i, b) in v.iter().enumerate() {
                    if i + 1 < v.len() {
                        let s = self.emit(Inst::Split(0, 0));
                        self.node(b);
                        jumps.push(self.emit(Inst::Jmp(0)));
                        let next = self.insts.len();
                        self.insts[s] = Inst::Split(s + 1, next);
                    } else {
                        self.node(b);
                    }
                }
                let end = self.insts.len();
                for j in jumps {
                    self.insts[j] = Inst::Jmp(end);
                }
            }
            Node::Repeat { inner, op, lazy } => match op {
                Op::Star => {
                    let l = self.emit(Inst::Split(0, 0));
                    self.node(inner);
                    self.emit(Inst::Jmp(l));
                    let out = self.insts.len();
                    self.insts[l] = Self::split(*lazy, l + 1, out);
                }
                Op::Plus => {
                    let l = self.insts.len();
                    self.node(inner);
                    let s = self.emit(Inst::Split(0, 0));
                    self.insts[s] = Self::split(*lazy, l, s + 1);
                }
                Op::Quest => {
                    let s = self.emit(Inst::Split(0, 0));
                    self.node(inner);
                    let out = self.insts.len();
                    self.insts[s] = Self::split(*lazy, s + 1, out);
                }
            },
        }
    }
}

fn program(n: &Node) -> Program {
    let mut c = Compiler {
        insts: Vec::new(),
        groups: 0,
    };
    c.emit(Inst::Save(0));
    c.node(n);
    c.emit(Inst::Save(1));
    c.emit(Inst::Match);
    Program {
        insts: c.insts,
        groups: c.groups,
    }
}

struct Matcher<'a> {
    prog: &'a Program,
    text: &'a [char],
    ci: bool,
}

impl Matcher<'_> {
    fn lit(&self, p: char, c: char) -> bool {
        if self.ci { ci_eq(p, c) } else { p == c }
    }

    fn in_items(items: &[(char, char)], c: char) -> bool {
        items.iter().any(|&(lo, hi)| lo <= c && c <= hi)
    }

    fn class(&self, negated: bool, items: &[(char, char)], c: char) -> bool {
        let member = if self.ci {
            // Folded before negation: some member m satisfies the predicate with c.
            fold_sets()
                .get(&c)
                .expect("text chars come from the test alphabets")
                .iter()
                .any(|&m| Self::in_items(items, m))
        } else {
            Self::in_items(items, c)
        };
        member != negated
    }

    /// `^`: start of text, after `\n`, after `\r\n` (i.e. after its `\n`), after a lone `\r`.
    fn bol(&self, pos: usize) -> bool {
        let t = self.text;
        pos == 0 || t[pos - 1] == '\n' || (t[pos - 1] == '\r' && t.get(pos) != Some(&'\n'))
    }

    /// `$`: end of text, before `\r`, before a `\n` that is not the second half of a CRLF.
    fn eol(&self, pos: usize) -> bool {
        let t = self.text;
        pos == t.len() || t[pos] == '\r' || (t[pos] == '\n' && (pos == 0 || t[pos - 1] != '\r'))
    }

    fn run(&self, pc: usize, pos: usize, slots: &mut Vec<Option<usize>>) -> bool {
        let t = self.text;
        match &self.prog.insts[pc] {
            Inst::Lit(p) => {
                pos < t.len() && self.lit(*p, t[pos]) && self.run(pc + 1, pos + 1, slots)
            }
            Inst::Any => {
                pos < t.len()
                    && t[pos] != '\r'
                    && t[pos] != '\n'
                    && self.run(pc + 1, pos + 1, slots)
            }
            Inst::Class { negated, items } => {
                pos < t.len()
                    && self.class(*negated, items, t[pos])
                    && self.run(pc + 1, pos + 1, slots)
            }
            Inst::Bol => self.bol(pos) && self.run(pc + 1, pos, slots),
            Inst::Eol => self.eol(pos) && self.run(pc + 1, pos, slots),
            Inst::Split(x, y) => self.run(*x, pos, slots) || self.run(*y, pos, slots),
            Inst::Jmp(x) => self.run(*x, pos, slots),
            Inst::Save(k) => {
                let old = slots[*k];
                slots[*k] = Some(pos);
                if self.run(pc + 1, pos, slots) {
                    return true;
                }
                slots[*k] = old;
                false
            }
            Inst::Match => true,
        }
    }

    /// Leftmost-first match whose start is >= `from`.
    fn find_from(&self, from: usize) -> Option<Vec<Span>> {
        for s in from..=self.text.len() {
            let mut slots = vec![None; 2 * (self.prog.groups + 1)];
            if self.run(0, s, &mut slots) {
                return Some(
                    (0..=self.prog.groups)
                        .map(|g| match (slots[2 * g], slots[2 * g + 1]) {
                            (Some(a), Some(b)) => Some((a, b)),
                            _ => None,
                        })
                        .collect(),
                );
            }
        }
        None
    }

    /// T-017's iteration rule.
    fn find_all(&self) -> Vec<Vec<Span>> {
        let mut out = Vec::new();
        let mut pos = 0;
        let mut last_end: Option<usize> = None;
        while pos <= self.text.len() {
            let Some(caps) = self.find_from(pos) else {
                break;
            };
            let (s, e) = caps[0].expect("whole match");
            if s == e && last_end == Some(s) {
                pos = s + 1;
                continue;
            }
            out.push(caps);
            last_end = Some(e);
            pos = if s == e { s + 1 } else { e };
        }
        out
    }
}

/// (group count, every match with its groups) of the model.
fn model_regex(n: &Node, text: &str, ci: bool) -> (usize, Vec<Vec<Span>>) {
    let prog = program(n);
    let chars: Vec<char> = text.chars().collect();
    let m = Matcher {
        prog: &prog,
        text: &chars,
        ci,
    };
    (prog.groups, m.find_all())
}

fn wholes(caps: &[Vec<Span>]) -> Vec<(usize, usize)> {
    caps.iter().map(|c| c[0].expect("whole match")).collect()
}

/// T-017's generator, with its restriction: no quantifier on a nullable operand.
struct Gen {
    rng: Rng,
}

impl Gen {
    fn alt(&mut self, depth: usize) -> Node {
        let k = match self.rng.below(10) {
            0..=5 => 1,
            6..=8 => 2,
            _ => 3,
        };
        let mut branches: Vec<Node> = (0..k).map(|_| self.concat(depth)).collect();
        if branches.len() == 1 {
            branches.pop().unwrap_or(Node::Empty)
        } else {
            Node::Alt(branches)
        }
    }

    fn concat(&mut self, depth: usize) -> Node {
        if self.rng.below(20) == 0 {
            return Node::Empty;
        }
        let k = 1 + self.rng.below(3);
        Node::Concat((0..k).map(|_| self.piece(depth)).collect())
    }

    fn piece(&mut self, depth: usize) -> Node {
        let atom = self.atom(depth);
        if !nullable(&atom) && self.rng.below(10) < 3 {
            let op = self.rng.pick(&[Op::Star, Op::Plus, Op::Quest]);
            let lazy = self.rng.below(10) < 3;
            Node::Repeat {
                inner: Box::new(atom),
                op,
                lazy,
            }
        } else {
            atom
        }
    }

    fn atom(&mut self, depth: usize) -> Node {
        let r = self.rng.below(100);
        if r < 45 {
            Node::Lit(self.rng.pick(RE_ALPHABET))
        } else if r < 55 {
            Node::Any
        } else if r < 70 {
            let negated = self.rng.below(10) < 3;
            let n = 1 + self.rng.below(3);
            let items = (0..n)
                .map(|_| {
                    let a = self.rng.pick(RE_ALPHABET);
                    if self.rng.below(10) < 6 {
                        (a, a)
                    } else {
                        let b = self.rng.pick(RE_ALPHABET);
                        (a.min(b), a.max(b))
                    }
                })
                .collect();
            Node::Class { negated, items }
        } else if r < 75 {
            Node::Bol
        } else if r < 80 {
            Node::Eol
        } else if depth > 0 {
            Node::Group {
                capture: self.rng.below(10) < 6,
                inner: Box::new(self.alt(depth - 1)),
            }
        } else {
            Node::Lit(self.rng.pick(RE_ALPHABET))
        }
    }
}

fn respects_restriction(n: &Node) -> bool {
    match n {
        Node::Repeat { inner, .. } => !nullable(inner) && respects_restriction(inner),
        Node::Group { inner, .. } => respects_restriction(inner),
        Node::Concat(v) | Node::Alt(v) => v.iter().all(respects_restriction),
        _ => true,
    }
}

// ---------------------------------------------------------------- naive expansion (AC-4)

/// T-017's expansion rule for a pattern with `group_count` unnamed groups: `Err(name)` for the
/// first reference to a group the pattern does not have.
fn model_expand(
    group_count: usize,
    caps: &[Span],
    text: &[char],
    template: &str,
) -> Result<String, String> {
    let t: Vec<char> = template.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let is_name_char = |c: char| c.is_ascii_alphanumeric() || c == '_';
    while i < t.len() {
        if t[i] != '$' {
            out.push(t[i]);
            i += 1;
            continue;
        }
        if t.get(i + 1) == Some(&'$') {
            out.push('$');
            i += 2;
            continue;
        }
        let name: String;
        if t.get(i + 1) == Some(&'{') {
            match (i + 2..t.len()).find(|&j| t[j] == '}') {
                Some(j) => {
                    name = t[i + 2..j].iter().collect();
                    i = j + 1;
                }
                None => {
                    out.push('$');
                    i += 1;
                    continue;
                }
            }
        } else {
            let mut j = i + 1;
            while j < t.len() && is_name_char(t[j]) {
                j += 1;
            }
            if j == i + 1 {
                out.push('$');
                i += 1;
                continue;
            }
            name = t[i + 1..j].iter().collect();
            i = j;
        }
        let number = if !name.is_empty() && name.chars().all(|c| c.is_ascii_digit()) {
            name.parse::<usize>().ok()
        } else {
            None
        };
        // The generated patterns have no named groups: every name is a missing group.
        let index = match number {
            Some(k) if k <= group_count => k,
            _ => return Err(name),
        };
        if let Some(Some((s, e))) = caps.get(index) {
            out.extend(&text[*s..*e]);
        }
    }
    Ok(out)
}

/// T-017's AC-2 template alphabet.
const TEMPLATE_ALPHABET: &[char] = &[
    '$', '$', '$', '$', '{', '}', '0', '1', '9', 'a', 'w', '_', 'x', '!', 'é', '😀', '+',
];

fn random_template(rng: &mut Rng) -> String {
    let n = rng.below(9);
    (0..n).map(|_| rng.pick(TEMPLATE_ALPHABET)).collect()
}

// ---------------------------------------------------------------- naive replace + mapping

/// One replacement: the chars `[s, e)` of the original text become `with`.
#[derive(Clone, Debug)]
struct Edit {
    s: usize,
    e: usize,
    with: String,
}

/// The text after `edits` (sorted, disjoint), and the ranges of the inserted texts in it.
fn apply(text: &str, edits: &[Edit]) -> (String, Vec<(usize, usize)>) {
    let t: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut ranges = Vec::new();
    let (mut at, mut len) = (0, 0);
    for ed in edits {
        out.extend(&t[at..ed.s]);
        len += ed.s - at;
        let n = ed.with.chars().count();
        out.push_str(&ed.with);
        ranges.push((len, len + n));
        len += n;
        at = ed.e;
    }
    out.extend(&t[at..]);
    (out, ranges)
}

fn inside_crlf(text: &[char], p: usize) -> bool {
    p > 0 && p < text.len() && text[p - 1] == '\r' && text[p] == '\n'
}

/// The mapping rule of `docs/protocol/cursor.md` for one position and one edit.
fn map_pos(p: usize, s: usize, e: usize, n: usize) -> usize {
    if p < s {
        p
    } else if p > e {
        p + n - (e - s)
    } else if p == s && e > s {
        s
    } else {
        s + n
    }
}

/// The selection after `edits`, applied from the last to the first, each mapping both ends
/// and then snapping out of a CRLF of the text as it is after that edit. No-op edits (empty
/// range, empty text) are skipped.
fn map_selection(text: &str, edits: &[Edit], anchor: usize, head: usize) -> (usize, usize) {
    let mut t: Vec<char> = text.chars().collect();
    let (mut a, mut h) = (anchor, head);
    for ed in edits.iter().rev() {
        let with: Vec<char> = ed.with.chars().collect();
        if ed.s == ed.e && with.is_empty() {
            continue;
        }
        a = map_pos(a, ed.s, ed.e, with.len());
        h = map_pos(h, ed.s, ed.e, with.len());
        t.splice(ed.s..ed.e, with);
        if inside_crlf(&t, a) {
            a -= 1;
        }
        if inside_crlf(&t, h) {
            h -= 1;
        }
    }
    (a, h)
}

/// A valid position of `text` (not inside a CRLF).
fn random_position(rng: &mut Rng, text: &str) -> usize {
    let t: Vec<char> = text.chars().collect();
    let p = rng.below(t.len() + 1);
    if inside_crlf(&t, p) { p - 1 } else { p }
}

/// One random query of AC-4 with its model: the pattern, the params, the matches with groups
/// and the group count (literal: 0 groups, group 0 only).
struct Case {
    params: Value,
    text: String,
    matches: Vec<Vec<Span>>,
    groups: usize,
    regex: bool,
}

fn random_case(rng: &mut Rng) -> Case {
    let regex = rng.below(10) < 6;
    let case_sensitive = rng.below(2) == 0;
    let ww = rng.below(10) < 2;
    let (pattern, text, mut matches, groups) = if regex {
        let mut g = Gen {
            rng: Rng(rng.next_u64()),
        };
        let node = g.alt(3);
        assert!(respects_restriction(&node));
        let pattern = pattern_of(&node);
        let text = random_string(rng, RE_ALPHABET, 10);
        let (groups, matches) = model_regex(&node, &text, !case_sensitive);
        (pattern, text, matches, groups)
    } else {
        let text = random_string(rng, LIT_ALPHABET, 12);
        let pattern = random_literal(rng, &text);
        let matches = naive_literal(&text, &pattern, case_sensitive)
            .into_iter()
            .map(|m| vec![Some(m)])
            .collect();
        (pattern, text, matches, 0)
    };
    if ww {
        let keep = whole_word(&text, &wholes(&matches));
        matches.retain(|c| keep.contains(&c[0].expect("whole match")));
    }
    Case {
        params: json!({"doc": 1, "query": pattern, "regex": regex,
            "case_sensitive": case_sensitive, "whole_word": ww}),
        text,
        matches,
        groups,
        regex,
    }
}

/// `base` with the fields of `extra` set.
fn merged(base: &Value, extra: Value) -> Value {
    let mut p = base.clone();
    for (k, v) in extra.as_object().expect("object") {
        p[k] = v.clone();
    }
    p
}

impl Case {
    /// The edits of replacing `matches` with `template`, or `Err` for a template error.
    fn edits(&self, matches: &[Vec<Span>], template: &str) -> Result<Vec<Edit>, String> {
        let t: Vec<char> = self.text.chars().collect();
        if self.regex {
            // Whether a template is an error depends on the pattern and the template only.
            model_expand(self.groups, &[], &t, template)?;
        }
        matches
            .iter()
            .map(|c| {
                let (s, e) = c[0].expect("whole match");
                let with = if self.regex {
                    model_expand(self.groups, c, &t, template)?
                } else {
                    template.to_owned()
                };
                Ok(Edit { s, e, with })
            })
            .collect()
    }

    fn with(&self, extra: Value) -> Value {
        merged(&self.params, extra)
    }
}

/// A `search.replace` reply for `edits` on `text` with the default limit.
fn replace_result(text: &str, edits: &[Edit]) -> (String, Expect) {
    let (after, ranges) = apply(text, edits);
    let limit = 10_000;
    let reply = Expect::Result(json!({
        "replaced": edits.len(),
        "len_chars": after.chars().count(),
        "ranges": ranges_json(&ranges[..ranges.len().min(limit)]),
        "truncated": ranges.len() > limit,
    }));
    (after, reply)
}

fn is_noop(edits: &[Edit]) -> bool {
    edits.iter().all(|e| e.s == e.e && e.with.is_empty())
}

// =============================================================================================
// AC-1, AC-3 (literal part)
// =============================================================================================

/// The AC-1 inputs: 5,000 fixed-seed (text, literal pattern) pairs.
fn literal_inputs() -> Vec<(String, String)> {
    let mut rng = Rng(0x7021_0001);
    (0..5_000)
        .map(|_| {
            let text = random_string(&mut rng, LIT_ALPHABET, 14);
            let pattern = random_literal(&mut rng, &text);
            (text, pattern)
        })
        .collect()
}

/// Sends every pair, in one `control` session, with both case settings and `whole_word` as
/// given; the expectation of each `search.find` is the naive scan (filtered for whole words).
fn literal_differential(name: &str, ww: bool) -> usize {
    let (work, mut r) = session(name, "");
    let mut len = 0;
    let mut nonempty = 0;
    for (text, pattern) in literal_inputs() {
        r.push(
            "doc.replace",
            json!({"doc": 1, "start": 0, "end": len, "text": text}),
            Expect::AnyOk,
        );
        len = text.chars().count();
        for cs in [true, false] {
            let mut want = naive_literal(&text, &pattern, cs);
            if ww {
                want = whole_word(&text, &want);
            }
            if !want.is_empty() {
                nonempty += 1;
            }
            r.push(
                "search.find",
                json!({"doc": 1, "query": pattern, "case_sensitive": cs, "whole_word": ww}),
                find_result(&want, 10_000),
            );
        }
    }
    r.run_control(&work.join("root"));
    nonempty
}

/// AC-1 (`search.literal`): for 5,000 fixed-seed (text, pattern) pairs over the AC-1 alphabet,
/// literal `search.find` with `case_sensitive` true and false equals the naive scan.
#[test]
fn literal_find_matches_naive_scan() {
    let nonempty = literal_differential("literal", false);
    assert!(nonempty > 4_000, "only {nonempty} queries with a match");
    // The naive scan's own sanity on hand-checked cases.
    assert_eq!(naive_literal("aaa", "aa", true), vec![(0, 2)]);
    assert_eq!(
        naive_literal("ΣςσſSs", "σ", false),
        vec![(0, 1), (1, 2), (2, 3)]
    );
    assert_eq!(naive_literal("ſSs", "s", false).len(), 3);
    assert_eq!(naive_literal("a\r\nb\nc", "\n", true), vec![(2, 3), (4, 5)]);
    assert_eq!(naive_literal("a\r\nb\rc", "\r\n", true), vec![(1, 3)]);
    // Regex metacharacters are plain chars in literal mode.
    let (work, mut r) = session("literal-meta", "a.b(a*)$1");
    for (q, want) in [
        ("a*", vec![(4, 6)]),
        (".", vec![(1, 2)]),
        ("(", vec![(3, 4)]),
        ("$1", vec![(7, 9)]),
    ] {
        r.push(
            "search.find",
            json!({"doc": 1, "query": q}),
            find_result(&want, 10_000),
        );
    }
    r.run_control(&work.join("root"));
}

// =============================================================================================
// AC-2, AC-3 (regex part)
// =============================================================================================

/// The AC-2 inputs: 5,000 fixed-seed (pattern node, text, case_sensitive) triples.
fn regex_inputs() -> Vec<(Node, String, bool)> {
    let mut g = Gen {
        rng: Rng(0x7021_0002),
    };
    (0..5_000)
        .map(|i| {
            let node = g.alt(3);
            assert!(respects_restriction(&node));
            let text = random_string(&mut g.rng, RE_ALPHABET, 10);
            (node, text, i % 2 == 0)
        })
        .collect()
}

fn regex_differential(name: &str, ww: bool) -> usize {
    let (work, mut r) = session(name, "");
    let mut len = 0;
    let mut nonempty = 0;
    for (node, text, cs) in regex_inputs() {
        r.push(
            "doc.replace",
            json!({"doc": 1, "start": 0, "end": len, "text": text}),
            Expect::AnyOk,
        );
        len = text.chars().count();
        let (_, caps) = model_regex(&node, &text, !cs);
        let mut want = wholes(&caps);
        if ww {
            want = whole_word(&text, &want);
        }
        if want.iter().any(|&(s, e)| s < e) {
            nonempty += 1;
        }
        r.push(
            "search.find",
            json!({"doc": 1, "query": pattern_of(&node), "regex": true, "case_sensitive": cs,
                "whole_word": ww}),
            find_result(&want, 10_000),
        );
    }
    r.run_control(&work.join("root"));
    nonempty
}

/// AC-2 (`search.regex`): for 5,000 random (pattern, text) pairs in T-017's subset (with its
/// generator restriction), half case-sensitive, `search.find` equals the naive backtracker.
#[test]
fn regex_find_matches_naive_backtracker() {
    let nonempty = regex_differential("regex", false);
    assert!(
        nonempty > 1_000,
        "only {nonempty} queries with a non-empty match"
    );
}

/// AC-2 (`search.regex`), refusals: `(`, `[a-`, `a{99999999}` and the empty pattern give
/// `rejected`/`invalid_pattern` and leave the session unchanged (`doc.text`, `doc.info`), for
/// `search.find` and `search.replace`; the empty pattern also in literal mode. 101 groups are
/// refused; 100 groups are accepted and the 100th group is usable in a replacement.
#[test]
fn invalid_patterns_are_reported() {
    let text = "aab\r\nab";
    let (work, mut r) = session("invalid", text);
    r.push(
        "doc.insert",
        json!({"doc": 1, "offset": 7, "text": "x"}),
        Expect::AnyOk,
    );
    let t0 = r.push("doc.text", json!({"doc": 1}), Expect::AnyOk);
    let i0 = r.push("doc.info", json!({"doc": 1}), Expect::AnyOk);
    let c0 = r.push("cursor.get", json!({"doc": 1}), Expect::AnyOk);
    let mut bad: Vec<(String, bool)> = ["(", "[a-", "a{99999999}", ""]
        .iter()
        .map(|p| ((*p).to_owned(), true))
        .collect();
    bad.push((String::new(), false));
    bad.push(("(a)".repeat(101), true));
    for (pattern, regex) in &bad {
        for cs in [true, false] {
            r.push(
                "search.find",
                json!({"doc": 1, "query": pattern, "regex": regex, "case_sensitive": cs}),
                rejected("invalid_pattern"),
            );
            for all in [true, false] {
                r.push(
                    "search.replace",
                    json!({"doc": 1, "query": pattern, "regex": regex, "case_sensitive": cs,
                        "replacement": "z", "all": all}),
                    rejected("invalid_pattern"),
                );
            }
        }
        r.push("doc.text", json!({"doc": 1}), Expect::SameAs(t0));
        r.push("doc.info", json!({"doc": 1}), Expect::SameAs(i0));
        r.push("cursor.get", json!({"doc": 1}), Expect::SameAs(c0));
    }
    // The typing group of the insert is still open: one undo removes "x" and "y".
    r.push(
        "doc.insert",
        json!({"doc": 1, "offset": 8, "text": "y"}),
        Expect::AnyOk,
    );
    r.push("doc.undo", json!({"doc": 1}), Expect::AnyOk);
    r.push("doc.text", json!({"doc": 1}), text_result(text));
    r.run_control(&work.join("root"));

    // 100 groups: accepted, 100-group matches, `$100` names the last group; `$101` does not.
    let hundred = format!("{}(b)", "(a)".repeat(99));
    let text = format!("{}b{}b", "a".repeat(99), "a".repeat(99));
    let (work, mut r) = session("hundred", &text);
    r.push(
        "search.find",
        json!({"doc": 1, "query": "(.)".repeat(100), "regex": true}),
        find_result(&[(0, 100), (100, 200)], 10_000),
    );
    r.push(
        "search.find",
        json!({"doc": 1, "query": hundred, "regex": true}),
        find_result(&[(0, 100), (100, 200)], 10_000),
    );
    r.push(
        "search.replace",
        json!({"doc": 1, "query": hundred, "regex": true, "replacement": "$101", "all": true}),
        rejected("invalid_pattern"),
    );
    r.push(
        "search.replace",
        json!({"doc": 1, "query": hundred, "regex": true, "replacement": "<$100$1>",
            "all": true}),
        Expect::Result(json!({"replaced": 2, "len_chars": 8,
            "ranges": ranges_json(&[(0, 4), (4, 8)]), "truncated": false})),
    );
    r.push("doc.text", json!({"doc": 1}), text_result("<ba><ba>"));
    r.run_control(&work.join("root"));
}

/// AC-3 (`search.whole-word`): on the AC-1 and AC-2 inputs, `whole_word: true` equals the
/// naive whole-word filter of the result without it.
#[test]
fn whole_word_filters_at_identifier_boundaries() {
    let lit = literal_differential("ww-literal", true);
    let re = regex_differential("ww-regex", true);
    assert!(
        lit > 1_000 && re > 300,
        "literal {lit}, regex {re} non-empty results"
    );
    // The filter's own sanity (Pinned (g) rows).
    assert_eq!(
        whole_word("foo FOO fOo_x éFoo", &[(0, 3), (4, 7), (8, 11), (15, 18)]),
        vec![(0, 3), (4, 7)]
    );
    assert_eq!(whole_word("aaa b aa", &[(0, 2), (6, 8)]), vec![(6, 8)]);
}

// =============================================================================================
// AC-4
// =============================================================================================

/// AC-4 (`search.replace`, `all: true`): for random texts, queries and templates, `doc.text`
/// equals the naive replacement and the reply equals the model (`replaced` = match count); a
/// template naming a missing group is `rejected`/`invalid_pattern` (also with no match) and
/// changes nothing; one `doc.undo` restores the original text exactly and `doc.redo`
/// re-applies; `replaced: 0` leaves `can_undo` and an open typing group as they were; the
/// selection set by `cursor.set` maps by the naive mapping rule.
#[test]
fn replace_all_matches_naive_model_and_undoes_in_one_step() {
    let mut rng = Rng(0x7021_0004);
    let (work, mut r) = session("replace-all", "");
    let mut len = 0;
    let (mut replaced_cases, mut refused, mut refused_no_match) = (0, 0, 0);
    let (mut zero, mut mapped, mut typed) = (0, 0, 0);
    for _ in 0..3_000 {
        let case = random_case(&mut rng);
        r.push(
            "doc.replace",
            json!({"doc": 1, "start": 0, "end": len, "text": case.text}),
            Expect::AnyOk,
        );
        len = case.text.chars().count();
        let template = random_template(&mut rng);
        if rng.below(10) == 0 {
            // An open typing group, a replace with no match (the alphabets have no `z`),
            // then a one-char insert right after the first: one undo removes both chars.
            typed += 1;
            r.push(
                "doc.insert",
                json!({"doc": 1, "offset": len, "text": "\n"}),
                Expect::AnyOk,
            );
            let info = r.push("doc.info", json!({"doc": 1}), Expect::AnyOk);
            r.push(
                "search.replace",
                case.with(
                    json!({"query": "zz", "regex": false, "replacement": template,
                    "all": true}),
                ),
                Expect::Result(json!({"replaced": 0, "len_chars": len + 1, "ranges": [],
                    "truncated": false})),
            );
            r.push("doc.info", json!({"doc": 1}), Expect::SameAs(info));
            r.push(
                "doc.insert",
                json!({"doc": 1, "offset": len + 1, "text": "\n"}),
                Expect::AnyOk,
            );
            r.push("doc.undo", json!({"doc": 1}), Expect::AnyOk);
            r.push("doc.text", json!({"doc": 1}), text_result(&case.text));
            continue;
        }
        let sel = (rng.below(3) == 0).then(|| {
            (
                random_position(&mut rng, &case.text),
                random_position(&mut rng, &case.text),
            )
        });
        if let Some((a, h)) = sel {
            r.push(
                "cursor.set",
                json!({"doc": 1, "anchor": a, "head": h}),
                Expect::AnyOk,
            );
        }
        let info = r.push("doc.info", json!({"doc": 1}), Expect::AnyOk);
        let cursor = r.push("cursor.get", json!({"doc": 1}), Expect::AnyOk);
        let params = case.with(json!({"replacement": template, "all": true}));
        match case.edits(&case.matches, &template) {
            Err(_) => {
                refused += 1;
                if case.matches.is_empty() {
                    refused_no_match += 1;
                }
                r.push("search.replace", params, rejected("invalid_pattern"));
                r.push("doc.info", json!({"doc": 1}), Expect::SameAs(info));
                r.push("cursor.get", json!({"doc": 1}), Expect::SameAs(cursor));
                r.push("doc.text", json!({"doc": 1}), text_result(&case.text));
            }
            Ok(edits) => {
                let (after, reply) = replace_result(&case.text, &edits);
                r.push("search.replace", params, reply);
                r.push("doc.text", json!({"doc": 1}), text_result(&after));
                if edits.is_empty() {
                    zero += 1;
                    r.push("doc.info", json!({"doc": 1}), Expect::SameAs(info));
                } else {
                    replaced_cases += 1;
                }
                if let Some((a, h)) = sel {
                    let (ma, mh) = map_selection(&case.text, &edits, a, h);
                    mapped += 1;
                    r.push(
                        "cursor.get",
                        json!({"doc": 1}),
                        Expect::Fields(vec![("anchor", json!(ma)), ("head", json!(mh))]),
                    );
                }
                if !is_noop(&edits) {
                    r.push(
                        "doc.undo",
                        json!({"doc": 1}),
                        Expect::Fields(vec![("undone", json!(true))]),
                    );
                    r.push("doc.text", json!({"doc": 1}), text_result(&case.text));
                    r.push(
                        "doc.redo",
                        json!({"doc": 1}),
                        Expect::Fields(vec![("redone", json!(true))]),
                    );
                    r.push("doc.text", json!({"doc": 1}), text_result(&after));
                }
                len = after.chars().count();
            }
        }
    }
    r.run_control(&work.join("root"));
    assert!(
        replaced_cases > 1_000
            && refused > 300
            && refused_no_match > 30
            && zero > 100
            && mapped > 300
            && typed > 100,
        "replaced {replaced_cases}, refused {refused} ({refused_no_match} without a match), \
         zero {zero}, mapped {mapped}, typed {typed}"
    );
}

/// AC-4 (`search.replace`, `all: false`): `from: f` replaces exactly the first match with
/// `start >= f` (matches found from char 0); `from > len_chars` is `rejected`/`out_of_range`
/// with `all: false` and ignored with `all: true`; `from` inside a CRLF is not refused; a
/// template error is reported before `out_of_range`.
#[test]
fn replace_one_from_offset() {
    let mut rng = Rng(0x7021_0005);
    let (work, mut r) = session("replace-one", "");
    let mut len = 0;
    let (mut one, mut none, mut far) = (0, 0, 0);
    for _ in 0..3_000 {
        let case = random_case(&mut rng);
        r.push(
            "doc.replace",
            json!({"doc": 1, "start": 0, "end": len, "text": case.text}),
            Expect::AnyOk,
        );
        len = case.text.chars().count();
        let template = random_template(&mut rng);
        let from = rng.below(len + 3);
        let params = case.with(json!({"replacement": template, "from": from}));
        let first: Vec<Vec<Span>> = case
            .matches
            .iter()
            .find(|c| c[0].expect("whole").0 >= from)
            .cloned()
            .into_iter()
            .collect();
        match case.edits(&first, &template) {
            Err(_) => {
                r.push("search.replace", params, rejected("invalid_pattern"));
                r.push("doc.text", json!({"doc": 1}), text_result(&case.text));
            }
            Ok(_) if from > len => {
                far += 1;
                r.push("search.replace", params, rejected("out_of_range"));
                r.push("doc.text", json!({"doc": 1}), text_result(&case.text));
                // With `all: true` the same `from` is ignored.
                let all = case.with(json!({"replacement": template, "from": from, "all": true}));
                let edits = case
                    .edits(&case.matches, &template)
                    .expect("template checked above");
                let (after, reply) = replace_result(&case.text, &edits);
                r.push("search.replace", all, reply);
                r.push("doc.text", json!({"doc": 1}), text_result(&after));
                len = after.chars().count();
            }
            Ok(edits) => {
                if edits.is_empty() {
                    none += 1;
                } else {
                    one += 1;
                }
                let (after, reply) = replace_result(&case.text, &edits);
                r.push("search.replace", params, reply);
                r.push("doc.text", json!({"doc": 1}), text_result(&after));
                if !is_noop(&edits) {
                    r.push("doc.undo", json!({"doc": 1}), Expect::AnyOk);
                    r.push("doc.text", json!({"doc": 1}), text_result(&case.text));
                    r.push("doc.redo", json!({"doc": 1}), Expect::AnyOk);
                }
                len = after.chars().count();
            }
        }
    }
    r.run_control(&work.join("root"));
    assert!(
        one > 800 && none > 300 && far > 300,
        "one {one}, none {none}, far {far}"
    );

    // `from` inside a CRLF is not refused; the first match at or after it is replaced.
    let (work, mut r) = session("replace-crlf", "b\r\nb");
    r.push(
        "search.replace",
        json!({"doc": 1, "query": "b", "replacement": "c", "from": 2}),
        Expect::Result(json!({"replaced": 1, "len_chars": 4,
            "ranges": ranges_json(&[(3, 4)]), "truncated": false})),
    );
    r.push("doc.text", json!({"doc": 1}), text_result("b\r\nc"));
    // Precedence: the template error before out_of_range; out_of_range only for all: false.
    r.push(
        "search.replace",
        json!({"doc": 1, "query": "(b)", "regex": true, "replacement": "$2", "from": 99}),
        rejected("invalid_pattern"),
    );
    r.push(
        "search.replace",
        json!({"doc": 1, "query": "", "replacement": "x", "from": 99}),
        rejected("invalid_pattern"),
    );
    r.push(
        "search.replace",
        json!({"doc": 1, "query": "b", "replacement": "x", "from": 5}),
        rejected("out_of_range"),
    );
    r.push(
        "search.replace",
        json!({"doc": 1, "query": "b", "replacement": "x", "from": 4}),
        Expect::Result(json!({"replaced": 0, "len_chars": 4, "ranges": [],
            "truncated": false})),
    );
    r.push("doc.text", json!({"doc": 1}), text_result("b\r\nc"));
    r.run_control(&work.join("root"));
}

/// Schema and document refusals of both commands (Interface, Pinned (a)): `null` for an
/// optional param, an unknown param, `limit` 0 and 100001 are `invalid_params`; a document
/// that is not open is `no_such_document`, before the pattern is looked at.
#[test]
fn params_and_document_are_checked_first() {
    let (work, mut r) = session("params", "abc");
    let t0 = r.push("doc.text", json!({"doc": 1}), Expect::AnyOk);
    let base = json!({"doc": 1, "query": "b"});
    for command in ["search.find", "search.replace"] {
        let own = if command == "search.replace" {
            merged(&base, json!({"replacement": "x"}))
        } else {
            base.clone()
        };
        for bad in [
            json!({"regex": null}),
            json!({"case_sensitive": null}),
            json!({"whole_word": null}),
            json!({"limit": null}),
            json!({"limit": 0}),
            json!({"limit": 100_001}),
            json!({"limit": -1}),
            json!({"extra": 1}),
            json!({"query": null}),
            json!({"doc": "1"}),
        ] {
            r.push(command, merged(&own, bad), schema_error());
        }
        r.push(command, merged(&own, json!({"limit": 1})), Expect::AnyOk);
        r.push(
            command,
            merged(&own, json!({"limit": 100_000})),
            Expect::AnyOk,
        );
        r.push("doc.undo", json!({"doc": 1}), Expect::AnyOk);
        r.push(
            command,
            merged(&own, json!({"doc": 2, "query": ""})),
            rejected("no_such_document"),
        );
        r.push(
            command,
            merged(&own, json!({"doc": 0, "query": "(", "regex": true})),
            rejected("no_such_document"),
        );
    }
    let own = merged(&base, json!({"replacement": "x"}));
    for bad in [
        json!({"all": null}),
        json!({"from": null}),
        json!({"replacement": null}),
        json!({"from": -1}),
    ] {
        r.push("search.replace", merged(&own, bad), schema_error());
    }
    r.push("search.replace", base.clone(), schema_error());
    r.push("search.find", json!({"doc": 1}), schema_error());
    r.push("doc.text", json!({"doc": 1}), Expect::SameAs(t0));
    r.run_control(&work.join("root"));
}

// =============================================================================================
// AC-5
// =============================================================================================

/// AC-5: every worked example of Pinned (g) holds exactly.
#[test]
fn worked_examples_hold() {
    let (work, mut r) = session("g1", "foo FOO fOo_x éFoo");
    r.push(
        "search.find",
        json!({"doc": 1, "query": "Foo", "case_sensitive": false}),
        find_result(&[(0, 3), (4, 7), (8, 11), (15, 18)], 10_000),
    );
    r.push(
        "search.find",
        json!({"doc": 1, "query": "Foo", "case_sensitive": false, "whole_word": true}),
        find_result(&[(0, 3), (4, 7)], 10_000),
    );
    r.run_control(&work.join("root"));

    let (work, mut r) = session("g2", "aaa b aa");
    r.push(
        "search.find",
        json!({"doc": 1, "query": "aa"}),
        find_result(&[(0, 2), (6, 8)], 10_000),
    );
    r.push(
        "search.find",
        json!({"doc": 1, "query": "aa", "whole_word": true}),
        find_result(&[(6, 8)], 10_000),
    );
    r.run_control(&work.join("root"));

    let (work, mut r) = session("g3", "baaab");
    r.push(
        "search.find",
        json!({"doc": 1, "query": "a*", "regex": true}),
        find_result(&[(0, 0), (1, 4), (5, 5)], 10_000),
    );
    r.push(
        "search.replace",
        json!({"doc": 1, "query": "a*", "regex": true, "replacement": "-", "all": true}),
        Expect::Result(json!({"replaced": 3, "len_chars": 5,
            "ranges": ranges_json(&[(0, 1), (2, 3), (4, 5)]), "truncated": false})),
    );
    r.push("doc.text", json!({"doc": 1}), text_result("-b-b-"));
    r.run_control(&work.join("root"));

    let (work, mut r) = session("g4", "a1 b2");
    r.push(
        "search.find",
        json!({"doc": 1, "query": "(\\w)(\\d)", "regex": true}),
        find_result(&[(0, 2), (3, 5)], 10_000),
    );
    let i0 = r.push("doc.info", json!({"doc": 1}), Expect::AnyOk);
    // `$3` and `$x` name missing groups: refused, text unchanged.
    for bad in ["$3", "$x"] {
        r.push(
            "search.replace",
            json!({"doc": 1, "query": "(\\w)(\\d)", "regex": true, "replacement": bad,
                "all": true}),
            rejected("invalid_pattern"),
        );
        r.push("doc.text", json!({"doc": 1}), text_result("a1 b2"));
        r.push("doc.info", json!({"doc": 1}), Expect::SameAs(i0));
    }
    r.push(
        "search.replace",
        json!({"doc": 1, "query": "(\\w)(\\d)", "regex": true, "replacement": "$2$1",
            "all": true}),
        Expect::Fields(vec![("replaced", json!(2))]),
    );
    r.push("doc.text", json!({"doc": 1}), text_result("1a 2b"));
    r.push("doc.undo", json!({"doc": 1}), Expect::AnyOk);
    r.push("doc.text", json!({"doc": 1}), text_result("a1 b2"));
    r.push(
        "search.replace",
        json!({"doc": 1, "query": "(\\w)(\\d)", "regex": true, "replacement": "$0-",
            "all": true}),
        Expect::Fields(vec![("replaced", json!(2))]),
    );
    r.push("doc.text", json!({"doc": 1}), text_result("a1- b2-"));
    r.push("doc.undo", json!({"doc": 1}), Expect::AnyOk);
    // Literal mode: `$1` is inserted verbatim.
    r.push(
        "search.replace",
        json!({"doc": 1, "query": "b", "replacement": "$1", "all": true}),
        Expect::Result(json!({"replaced": 1, "len_chars": 6,
            "ranges": ranges_json(&[(3, 5)]), "truncated": false})),
    );
    r.push("doc.text", json!({"doc": 1}), text_result("a1 $12"));
    r.run_control(&work.join("root"));
}

// =============================================================================================
// AC-6, AC-11
// =============================================================================================

/// AC-6: 20,001 matches with `limit: 10` → 10 matches, `total: 20001`, `truncated: true`.
#[test]
fn find_limit_counts_every_match() {
    let text = "x ".repeat(20_001);
    let all: Vec<(usize, usize)> = (0..20_001).map(|i| (2 * i, 2 * i + 1)).collect();
    let (work, mut r) = session("limit", &text);
    r.push(
        "search.find",
        json!({"doc": 1, "query": "x", "limit": 10}),
        find_result(&all, 10),
    );
    r.push(
        "search.find",
        json!({"doc": 1, "query": "X", "case_sensitive": false, "whole_word": true,
            "limit": 10}),
        find_result(&all, 10),
    );
    r.push(
        "search.find",
        json!({"doc": 1, "query": "x", "regex": true, "limit": 10}),
        find_result(&all, 10),
    );
    // Default limit 10,000; limit equal to the total is not truncated.
    r.push(
        "search.find",
        json!({"doc": 1, "query": "x"}),
        find_result(&all, 10_000),
    );
    r.push(
        "search.find",
        json!({"doc": 1, "query": "x", "limit": 20_001}),
        find_result(&all, 20_001),
    );
    r.run_script(&work, &work.join("root"));
}

/// AC-11: on AC-6's text, `search.replace {all:true, limit:10}` → `replaced: 20001`, the first
/// 10 `ranges` of the naive model, `truncated: true`, and every match replaced.
#[test]
fn replace_reply_is_limited() {
    let text = "x ".repeat(20_001);
    let edits: Vec<Edit> = (0..20_001)
        .map(|i| Edit {
            s: 2 * i,
            e: 2 * i + 1,
            with: "yz".to_owned(),
        })
        .collect();
    let (after, ranges) = apply(&text, &edits);
    let (work, mut r) = session("reply-limit", &text);
    r.push(
        "search.replace",
        json!({"doc": 1, "query": "x", "replacement": "yz", "all": true, "limit": 10}),
        Expect::Result(json!({"replaced": 20_001, "len_chars": 60_003,
            "ranges": ranges_json(&ranges[..10]), "truncated": true})),
    );
    r.push("doc.text", json!({"doc": 1}), text_result(&after));
    r.push("doc.undo", json!({"doc": 1}), Expect::AnyOk);
    r.push("doc.text", json!({"doc": 1}), text_result(&text));
    // `limit` equal to the number of replacements: every range, not truncated.
    r.push(
        "search.replace",
        json!({"doc": 1, "query": "x", "replacement": "yz", "all": true, "limit": 20_001}),
        Expect::Result(json!({"replaced": 20_001, "len_chars": 60_003,
            "ranges": ranges_json(&ranges), "truncated": false})),
    );
    r.run_script(&work, &work.join("root"));
    assert_eq!(after, "yz ".repeat(20_001));
}

// =============================================================================================
// AC-7a
// =============================================================================================

/// AC-7a: `ab` × 200,000, regex `(a)b`, replacement `$1$1`, `all: true` → `replaced: 200000`
/// and the text `aa` × 200,000. The whole run has the 120-second hang detector; an expansion
/// that walks the text from its start per reference (about 8·10^10 char steps here) never
/// finishes within it. Not a time assertion.
#[test]
fn replace_all_is_linear() {
    let (work, mut r) = session("linear", &"ab".repeat(200_000));
    r.push(
        "search.replace",
        json!({"doc": 1, "query": "(a)b", "regex": true, "replacement": "$1$1", "all": true,
            "limit": 3}),
        Expect::Result(json!({"replaced": 200_000, "len_chars": 400_000,
            "ranges": ranges_json(&[(0, 2), (2, 4), (4, 6)]), "truncated": true})),
    );
    r.push(
        "doc.text",
        json!({"doc": 1}),
        text_result(&"aa".repeat(200_000)),
    );
    r.run_script(&work, &work.join("root"));
}

// =============================================================================================
// AC-8
// =============================================================================================

/// AC-8: under a 512 MiB address-space cap (`ulimit -v 524288`; Unix, D-014) and the
/// 120-second hang detector, on `x` × 400,000 with the regex `()` × 100: `search.find
/// {limit:10}` → `0..0` … `9..9`, `total: 400001`, `truncated`; `search.replace {replacement:
/// "-", all:true}` → `replaced: 400001`, `len_chars: 800001`, 10,000 `ranges`, `truncated`;
/// `doc.undo` restores the text. Every line `ok`, exit 0. Keeping every match's slots needs
/// about 0.96 GB and aborts under the cap.
#[cfg(unix)]
#[test]
fn search_memory_is_bounded() {
    let text = "x".repeat(400_000);
    let (work, mut r) = session("memory", &text);
    let pattern = "()".repeat(100);
    let first: Vec<(usize, usize)> = (0..10).map(|i| (i, i)).collect();
    r.push(
        "search.find",
        json!({"doc": 1, "query": pattern, "regex": true, "limit": 10}),
        Expect::Result(json!({"matches": ranges_json(&first), "total": 400_001,
            "truncated": true})),
    );
    let ranges: Vec<(usize, usize)> = (0..10_000).map(|i| (2 * i, 2 * i + 1)).collect();
    r.push(
        "search.replace",
        json!({"doc": 1, "query": pattern, "regex": true, "replacement": "-", "all": true}),
        Expect::Result(json!({"replaced": 400_001, "len_chars": 800_001,
            "ranges": ranges_json(&ranges), "truncated": true})),
    );
    r.push(
        "doc.undo",
        json!({"doc": 1}),
        Expect::Fields(vec![("undone", json!(true))]),
    );
    r.push("doc.text", json!({"doc": 1}), text_result(&text));
    let script = work.join("s.jsonl");
    std::fs::write(&script, r.input()).expect("write script");
    let shell = format!(
        "ulimit -v 524288; exec '{}' run '{}' --root '{}'",
        kiln(),
        script.display(),
        work.join("root").display()
    );
    let out = spawn("sh", &["-c", &shell], Vec::new());
    let responses = parse_lines(&out, r.lines.len());
    r.check(&responses);
    assert_eq!(out.status.code(), Some(0), "every line ok, exit 0");
}

// =============================================================================================
// AC-9
// =============================================================================================

/// AC-9: in both modes and both case settings a 1,001-char pattern is
/// `rejected`/`invalid_pattern` and the session is unchanged; a 1,000-char pattern (`a`
/// repeated) is accepted. The cap counts chars: 1,000 `é` (2,000 bytes) are accepted.
#[test]
fn pattern_length_is_capped() {
    let text = format!("{}{}", "a".repeat(2_500), "é".repeat(2_001));
    let (work, mut r) = session("cap", &text);
    r.push(
        "doc.insert",
        json!({"doc": 1, "offset": 0, "text": "a"}),
        Expect::AnyOk,
    );
    let t0 = r.push("doc.text", json!({"doc": 1}), Expect::AnyOk);
    let i0 = r.push("doc.info", json!({"doc": 1}), Expect::AnyOk);
    for regex in [false, true] {
        for cs in [true, false] {
            for long in ["a".repeat(1_001), "é".repeat(1_001), "(".repeat(1_001)] {
                r.push(
                    "search.find",
                    json!({"doc": 1, "query": long, "regex": regex, "case_sensitive": cs}),
                    rejected("invalid_pattern"),
                );
                r.push(
                    "search.replace",
                    json!({"doc": 1, "query": long, "regex": regex, "case_sensitive": cs,
                        "replacement": "b", "all": true}),
                    rejected("invalid_pattern"),
                );
            }
            r.push("doc.text", json!({"doc": 1}), Expect::SameAs(t0));
            r.push("doc.info", json!({"doc": 1}), Expect::SameAs(i0));
            r.push(
                "search.find",
                json!({"doc": 1, "query": "a".repeat(1_000), "regex": regex,
                    "case_sensitive": cs}),
                find_result(&[(0, 1_000), (1_000, 2_000)], 10_000),
            );
            r.push(
                "search.find",
                json!({"doc": 1, "query": "é".repeat(1_000), "regex": regex,
                    "case_sensitive": cs}),
                find_result(&[(2_501, 3_501), (3_501, 4_501)], 10_000),
            );
        }
    }
    r.run_control(&work.join("root"));
}
