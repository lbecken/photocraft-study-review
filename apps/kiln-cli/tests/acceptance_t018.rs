//! T-018 acceptance tests through the built `kiln-cli` binary (`control --root`, `run --root`)
//! and the committed generated files. Written from the task specification alone by the
//! independent test author.
//!
//! Oracles are built here: a naive `Vec<char>` model of the text, of the undo history with the
//! kiln-text typing-group rule, of the selection mapping rule (incl. the CRLF snap) and of the
//! movement and collapse rules (AC-1); the worked examples of Pinned (g) as exact values
//! (AC-2, AC-3); a naive CRLF-first count and `\r\n|\r|\n` replacement plus the bytes on disk
//! (AC-4, AC-5). All generators are fixed-seed (SplitMix64).
//!
//! The file uses no new API: it compiles on base and fails there by assertion (`cursor.*` and
//! `eol.*` are unknown commands). Every spawned process is killed after [`TIMEOUT`]
//! (D-028 (8)).
#![cfg(test)]

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

// ---------------------------------------------------------------- process helpers

const TIMEOUT: Duration = Duration::from_secs(120);

/// A fresh, empty directory under cargo's per-target temp dir, unique per test and process.
fn fresh_dir(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("acceptance_t018")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("create temp dir");
    d
}

fn repo_file(rel: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

/// Run `kiln-cli args` with `stdin` as its input; kill it after [`TIMEOUT`].
fn kiln(args: &[&str], stdin: Vec<u8>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn kiln-cli");
    let mut input = child.stdin.take().expect("stdin");
    // A writer thread, so a large input cannot deadlock against a full stdout pipe.
    let writer = thread::spawn(move || {
        let _ = input.write_all(&stdin);
        // `input` is dropped here: end of input.
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
    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait().expect("try_wait") {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("kiln-cli {args:?} did not finish within {TIMEOUT:?}");
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

fn parse_lines(out: &Output, want: usize) -> Vec<Value> {
    let stdout = String::from_utf8(out.stdout.clone()).expect("stdout UTF-8");
    let responses: Vec<Value> = stdout
        .lines()
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("not JSON ({e}): {l}")))
        .collect();
    assert_eq!(
        responses.len(),
        want,
        "one response per request; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
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
    /// `ok:false` with this code and kind (messages are not pinned by T-018).
    Error { code: &'static str, kind: String },
}

fn rejected(kind: &str) -> Expect {
    Expect::Error {
        code: "rejected",
        kind: kind.to_owned(),
    }
}

fn schema_error() -> Expect {
    Expect::Error {
        code: "invalid_params",
        kind: "schema".to_owned(),
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

    /// Check every response against its expectation, in order; the panic names the request
    /// and the requests just before it.
    fn check(&self, responses: &[Value]) {
        for (i, (resp, want)) in responses.iter().zip(&self.expect).enumerate() {
            let ctx = || {
                let from = i.saturating_sub(6);
                format!(
                    "request #{}: {}\n  -> {resp}\n  preceding requests:\n    {}",
                    i + 1,
                    self.lines[i],
                    self.lines[from..i].join("\n    ")
                )
            };
            assert_eq!(resp["id"], json!(i + 1), "{}", ctx());
            match want {
                Expect::Result(r) => {
                    assert_eq!(resp["ok"], json!(true), "{}", ctx());
                    assert_eq!(&resp["result"], r, "{}", ctx());
                }
                Expect::AnyOk => assert_eq!(resp["ok"], json!(true), "{}", ctx()),
                Expect::Error { code, kind } => {
                    assert_eq!(resp["ok"], json!(false), "{}", ctx());
                    assert_eq!(resp["error"]["code"], json!(code), "{}", ctx());
                    assert_eq!(resp["error"]["kind"], json!(kind), "{}", ctx());
                }
            }
        }
    }

    /// Run as a `kiln-cli run` script with `--root root`; check exit code and responses.
    fn run_script(&self, work: &Path, root: &Path, name: &str) -> Vec<Value> {
        let script = work.join(format!("{name}.jsonl"));
        std::fs::write(&script, self.input()).expect("write script");
        let out = kiln(
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

    /// Send all requests to one `kiln-cli control --root root` session; check responses.
    fn run_control(&self, root: &Path) -> Vec<Value> {
        let out = kiln(
            &["control", "--root", root.to_str().expect("utf-8 path")],
            self.input(),
        );
        let responses = parse_lines(&out, self.lines.len());
        self.check(&responses);
        assert_eq!(out.status.code(), Some(0), "control exit code");
        responses
    }
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

    fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

/// Text pieces: 1-, 2-, 3- and 4-byte chars, word and non-word chars, every line-break kind
/// and the adjacent pairs `\r\r\n` and `\n\r`.
const PIECES: &[&str] = &[
    "a",
    "b",
    "_",
    "x9",
    " ",
    "é",
    "€",
    "😀",
    "\n",
    "\r",
    "\r\n",
    "\r\r\n",
    "\n\r",
    "foo_bar",
    ", ",
    "é😀€",
];

/// `min` to `min + spread - 1` random pieces.
fn random_text(rng: &mut Rng, min: usize, spread: usize) -> String {
    let pieces = min + rng.below(spread);
    (0..pieces).map(|_| *rng.pick(PIECES)).collect()
}

// ---------------------------------------------------------------- naive model (AC-1)

fn is_word(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

/// Lines of `text` as (start, content end) char offsets, split on CRLF, lone CR, lone LF.
fn lines_of(text: &[char]) -> Vec<(usize, usize)> {
    let mut lines = Vec::new();
    let (mut start, mut i) = (0, 0);
    while i < text.len() {
        let len = break_len(text, i);
        if len > 0 {
            lines.push((start, i));
            i += len;
            start = i;
        } else {
            i += 1;
        }
    }
    lines.push((start, text.len()));
    lines
}

/// Length of the line break starting at `i` (2 for CRLF, 1 for a lone CR or LF, else 0).
fn break_len(text: &[char], i: usize) -> usize {
    match text[i] {
        '\r' if text.get(i + 1) == Some(&'\n') => 2,
        '\r' | '\n' => 1,
        _ => 0,
    }
}

fn inside_crlf(text: &[char], p: usize) -> bool {
    p > 0 && p < text.len() && text[p - 1] == '\r' && text[p] == '\n'
}

/// Line and column of `p` (not inside a CRLF).
fn line_col(text: &[char], p: usize) -> (usize, usize) {
    let lines = lines_of(text);
    let line = lines
        .iter()
        .rposition(|&(start, _)| start <= p)
        .expect("line 0 starts at 0");
    (line, p - lines[line].0)
}

/// The head's new position for one movement step (Movement rule of the spec).
fn moved(text: &[char], from: usize, by: &str, forward: bool) -> usize {
    let len = text.len();
    match (by, forward) {
        ("char", true) => {
            if from >= len {
                len
            } else if text[from] == '\r' && text.get(from + 1) == Some(&'\n') {
                from + 2
            } else {
                from + 1
            }
        }
        ("char", false) => {
            if from == 0 {
                0
            } else if from >= 2 && text[from - 2] == '\r' && text[from - 1] == '\n' {
                from - 2
            } else {
                from - 1
            }
        }
        ("word", true) => {
            let mut i = from;
            while i < len && !is_word(text[i]) {
                i += 1;
            }
            while i < len && is_word(text[i]) {
                i += 1;
            }
            i
        }
        ("word", false) => {
            let mut i = from;
            while i > 0 && !is_word(text[i - 1]) {
                i -= 1;
            }
            while i > 0 && is_word(text[i - 1]) {
                i -= 1;
            }
            i
        }
        ("line", _) => {
            let lines = lines_of(text);
            let (line, col) = line_col(text, from);
            if forward {
                match lines.get(line + 1) {
                    Some(&(start, end)) => start + col.min(end - start),
                    None => len,
                }
            } else if line == 0 {
                0
            } else {
                let (start, end) = lines[line - 1];
                start + col.min(end - start)
            }
        }
        _ => unreachable!("movement {by}"),
    }
}

/// One undo step: at `at`, `removed` was replaced by `inserted`.
#[derive(Clone, Debug)]
struct Op {
    at: usize,
    removed: Vec<char>,
    inserted: Vec<char>,
}

/// Naive model of one open document: text, saved text, history, selection.
#[derive(Clone, Debug)]
struct Model {
    text: Vec<char>,
    saved: Vec<char>,
    undo: Vec<Op>,
    redo: Vec<Op>,
    typing_open: bool,
    anchor: usize,
    head: usize,
}

impl Model {
    fn new(text: &str) -> Self {
        let text: Vec<char> = text.chars().collect();
        Model {
            saved: text.clone(),
            text,
            undo: Vec::new(),
            redo: Vec::new(),
            typing_open: false,
            anchor: 0,
            head: 0,
        }
    }

    fn len(&self) -> usize {
        self.text.len()
    }

    fn string(&self) -> String {
        self.text.iter().collect()
    }

    fn dirty(&self) -> bool {
        self.text != self.saved
    }

    /// The mapping rule for one primitive edit `[s, e)` → `n` chars, on the text after it.
    fn map(&self, p: usize, s: usize, e: usize, n: usize) -> usize {
        let r = if p < s {
            p
        } else if p > e {
            p + n - (e - s)
        } else if p == s && e > s {
            s
        } else {
            s + n
        };
        if inside_crlf(&self.text, r) { r - 1 } else { r }
    }

    /// Apply one primitive edit and map the selection through it.
    fn primitive(&mut self, s: usize, e: usize, ins: &[char]) -> Vec<char> {
        let removed: Vec<char> = self.text.splice(s..e, ins.iter().copied()).collect();
        self.anchor = self.map(self.anchor, s, e, ins.len());
        self.head = self.map(self.head, s, e, ins.len());
        removed
    }

    fn record(&mut self, op: Op, typing: bool) {
        self.redo.clear();
        if typing
            && self.typing_open
            && let Some(top) = self.undo.last_mut()
            && top.at + top.inserted.len() == op.at
        {
            top.inserted.extend(op.inserted);
            return;
        }
        self.undo.push(op);
        self.typing_open = typing;
    }

    fn insert(&mut self, at: usize, text: &str) {
        let ins: Vec<char> = text.chars().collect();
        if ins.is_empty() {
            return;
        }
        let typing = ins.len() == 1 && ins[0] != '\n' && ins[0] != '\r';
        self.primitive(at, at, &ins);
        self.record(
            Op {
                at,
                removed: Vec::new(),
                inserted: ins,
            },
            typing,
        );
    }

    fn replace(&mut self, s: usize, e: usize, text: &str) -> String {
        let ins: Vec<char> = text.chars().collect();
        if s == e && ins.is_empty() {
            return String::new();
        }
        let removed = self.primitive(s, e, &ins);
        let out = removed.iter().collect();
        self.record(
            Op {
                at: s,
                removed,
                inserted: ins,
            },
            false,
        );
        out
    }

    fn undo(&mut self) -> bool {
        let Some(op) = self.undo.pop() else {
            return false;
        };
        self.typing_open = false;
        self.primitive(op.at, op.at + op.inserted.len(), &op.removed);
        self.redo.push(op);
        true
    }

    fn redo(&mut self) -> bool {
        let Some(op) = self.redo.pop() else {
            return false;
        };
        self.typing_open = false;
        self.primitive(op.at, op.at + op.removed.len(), &op.inserted);
        self.undo.push(op);
        true
    }

    /// Validate one position of `cursor.set`: `None` if valid, else the refusal kind.
    fn refusal(&self, p: usize) -> Option<&'static str> {
        if p > self.len() {
            Some("out_of_range")
        } else if inside_crlf(&self.text, p) {
            Some("not_a_boundary")
        } else {
            None
        }
    }

    fn cursor(&self, doc: u64) -> Value {
        let (line, col) = line_col(&self.text, self.head);
        json!({
            "doc": doc,
            "anchor": self.anchor,
            "head": self.head,
            "start": self.anchor.min(self.head),
            "end": self.anchor.max(self.head),
            "line": line,
            "col": col,
        })
    }
}

/// A position near the selection or the ends, or anywhere, within `0..=len`.
fn interesting(rng: &mut Rng, m: &Model) -> usize {
    let len = m.len();
    let base = match rng.below(8) {
        0 => m.anchor,
        1 => m.head,
        2 => 0,
        3 => len,
        _ => rng.below(len + 1),
    };
    let p = match rng.below(4) {
        0 => base.saturating_sub(1),
        1 => base + 1,
        _ => base,
    };
    p.min(len)
}

/// Append one random command for doc 1 (and its expectation) and update the model.
fn random_command(
    rng: &mut Rng,
    m: &mut Model,
    reqs: &mut Requests,
    last_typed: &mut Option<usize>,
) {
    let len = m.len();
    let roll = rng.below(100);
    let typed = last_typed.take();
    match roll {
        // Typing: a one-char insert right after the previous one (joins a typing group).
        0..=11 if typed.is_some() => {
            let at = typed.expect("checked").min(len);
            let c = *rng.pick(&["a", "é", "€", "😀", "_", " "]);
            m.insert(at, c);
            *last_typed = Some(at + 1);
            reqs.push(
                "doc.insert",
                json!({"doc": 1, "offset": at, "text": c}),
                Expect::Result(json!({"len_chars": m.len(), "dirty": m.dirty()})),
            );
        }
        0..=21 => {
            let at = interesting(rng, m);
            let text = if rng.chance(50) {
                (*rng.pick(&["a", "é", "😀", "\n", "\r", "\r\n", "€"])).to_owned()
            } else {
                random_text(rng, 1, 3)
            };
            if text.chars().count() == 1 {
                *last_typed = Some(at + 1);
            }
            m.insert(at, &text);
            reqs.push(
                "doc.insert",
                json!({"doc": 1, "offset": at, "text": text}),
                Expect::Result(json!({"len_chars": m.len(), "dirty": m.dirty()})),
            );
        }
        22..=33 => {
            let a = interesting(rng, m);
            let b = if rng.chance(50) {
                interesting(rng, m)
            } else {
                (a + rng.below(4)).min(len)
            };
            let (s, e) = (a.min(b), a.max(b));
            let removed = m.replace(s, e, "");
            reqs.push(
                "doc.delete",
                json!({"doc": 1, "start": s, "end": e}),
                Expect::Result(
                    json!({"removed": removed, "len_chars": m.len(), "dirty": m.dirty()}),
                ),
            );
        }
        34..=45 => {
            let a = interesting(rng, m);
            let b = if rng.chance(50) {
                interesting(rng, m)
            } else {
                (a + rng.below(4)).min(len)
            };
            let (s, e) = (a.min(b), a.max(b));
            let text = if rng.chance(15) {
                String::new()
            } else {
                random_text(rng, 1, 2)
            };
            let removed = m.replace(s, e, &text);
            reqs.push(
                "doc.replace",
                json!({"doc": 1, "start": s, "end": e, "text": text}),
                Expect::Result(
                    json!({"removed": removed, "len_chars": m.len(), "dirty": m.dirty()}),
                ),
            );
        }
        46..=54 => {
            let undone = m.undo();
            reqs.push(
                "doc.undo",
                json!({"doc": 1}),
                Expect::Result(json!({"undone": undone, "len_chars": m.len(), "dirty": m.dirty()})),
            );
        }
        55..=61 => {
            let redone = m.redo();
            reqs.push(
                "doc.redo",
                json!({"doc": 1}),
                Expect::Result(json!({"redone": redone, "len_chars": m.len(), "dirty": m.dirty()})),
            );
        }
        62..=71 => {
            // cursor.set, sometimes refused (beyond the text, or inside a CRLF).
            let pos = |rng: &mut Rng, m: &Model| -> usize {
                match rng.below(10) {
                    0 => m.len() + 1 + rng.below(3),
                    1 => {
                        let inside: Vec<usize> =
                            (1..m.len()).filter(|&p| inside_crlf(&m.text, p)).collect();
                        if inside.is_empty() {
                            interesting(rng, m)
                        } else {
                            *rng.pick(&inside)
                        }
                    }
                    _ => {
                        let p = interesting(rng, m);
                        if inside_crlf(&m.text, p) { p - 1 } else { p }
                    }
                }
            };
            let anchor = pos(rng, m);
            let head = if rng.chance(25) {
                None
            } else {
                Some(pos(rng, m))
            };
            let params = match head {
                Some(h) => json!({"doc": 1, "anchor": anchor, "head": h}),
                None => json!({"doc": 1, "anchor": anchor}),
            };
            let h = head.unwrap_or(anchor);
            match m.refusal(anchor).or_else(|| m.refusal(h)) {
                Some(kind) => {
                    // Refused: nothing changes, an open typing group stays open.
                    reqs.push("cursor.set", params, rejected(kind));
                    *last_typed = typed;
                }
                None => {
                    m.anchor = anchor;
                    m.head = h;
                    m.typing_open = false;
                    // A one-char insert may follow at the same place: a new undo step.
                    *last_typed = typed;
                    reqs.push("cursor.set", params, Expect::Result(m.cursor(1)));
                }
            }
        }
        72..=89 => {
            let by = *rng.pick(&["char", "word", "line"]);
            let forward = rng.chance(50);
            let extend = rng.below(3);
            let new_head = moved(&m.text, m.head, by, forward);
            m.head = new_head;
            if extend != 1 {
                m.anchor = new_head;
            }
            m.typing_open = false;
            *last_typed = typed;
            let params = match extend {
                0 => json!({"doc": 1, "by": by, "forward": forward}),
                1 => json!({"doc": 1, "by": by, "forward": forward, "extend": true}),
                _ => json!({"doc": 1, "by": by, "forward": forward, "extend": false}),
            };
            reqs.push("cursor.move", params, Expect::Result(m.cursor(1)));
        }
        90..=93 => {
            // A rejected edit maps nothing and changes nothing.
            let at = len + 1 + rng.below(3);
            reqs.push(
                "doc.insert",
                json!({"doc": 1, "offset": at, "text": "z"}),
                rejected("out_of_range"),
            );
            *last_typed = typed;
        }
        94..=96 => {
            // A no-op edit maps nothing.
            let at = interesting(rng, m);
            reqs.push(
                "doc.delete",
                json!({"doc": 1, "start": at, "end": at}),
                Expect::Result(json!({"removed": "", "len_chars": m.len(), "dirty": m.dirty()})),
            );
            *last_typed = typed;
        }
        _ => {
            reqs.push(
                "doc.text",
                json!({"doc": 1}),
                Expect::Result(json!({"text": m.string(), "len_chars": m.len()})),
            );
            *last_typed = typed;
        }
    }
}

/// AC-1: fixed-seed random sessions through `kiln-cli control`; after every command,
/// `cursor.get` equals the naive model (mapping, CRLF snap, undo/redo mapping, movement,
/// collapse), and every command's own result equals the model's.
#[test]
fn selection_follows_random_sessions() {
    const SESSIONS: u64 = 8;
    const COMMANDS: usize = 300;
    let mut total = 0;
    for seed in 0..SESSIONS {
        let mut rng = Rng(0x7018_0000 + seed);
        let root = fresh_dir(&format!("random-{seed}"));
        let initial = random_text(&mut rng, 6, 10);
        std::fs::write(root.join("s.txt"), &initial).expect("write file");
        let mut m = Model::new(&initial);
        let mut reqs = Requests::default();
        reqs.push("doc.open", json!({"path": "s.txt"}), Expect::AnyOk);
        // A new document's selection is 0, 0.
        reqs.push("cursor.get", json!({"doc": 1}), Expect::Result(m.cursor(1)));
        let mut last_typed = None;
        for _ in 0..COMMANDS {
            random_command(&mut rng, &mut m, &mut reqs, &mut last_typed);
            total += 1;
            reqs.push("cursor.get", json!({"doc": 1}), Expect::Result(m.cursor(1)));
        }
        reqs.push(
            "doc.text",
            json!({"doc": 1}),
            Expect::Result(json!({"text": m.string(), "len_chars": m.len()})),
        );
        reqs.run_control(&root);
    }
    assert!(total >= 2000, "{total} commands");
}

// ---------------------------------------------------------------- worked examples (AC-2)

fn cursor(doc: u64, anchor: usize, head: usize, line: usize, col: usize) -> Expect {
    Expect::Result(json!({
        "doc": doc, "anchor": anchor, "head": head,
        "start": anchor.min(head), "end": anchor.max(head), "line": line, "col": col,
    }))
}

/// AC-2: every worked example of Pinned (g), exactly, through `kiln-cli run`; plus the
/// collapse/extend rule of `cursor.move` and the movement rule at the text ends.
#[test]
fn cursor_worked_examples() {
    let work = fresh_dir("examples");
    let root = work.join("root");
    std::fs::create_dir_all(&root).expect("root");
    let files: [(&str, &str); 5] = [
        ("map.txt", "abc\r\ndef"),
        ("word.rs", "fn foo_bar(x: u8)"),
        ("line.txt", "abcdef\nxy\nlonger"),
        ("char.txt", "a\r\nb"),
        ("eol.txt", "a\r\nb\nc\rd"),
    ];
    for (name, text) in files {
        std::fs::write(root.join(name), text).expect("write file");
    }
    let mut r = Requests::default();
    for (i, (name, _)) in files.iter().enumerate() {
        r.push("doc.open", json!({"path": name}), Expect::AnyOk);
        // A new document's selection is 0, 0.
        r.push(
            "cursor.get",
            json!({"doc": i + 1}),
            cursor(i as u64 + 1, 0, 0, 0, 0),
        );
    }

    // (g) 1: mapping through doc.insert and doc.delete.
    r.push(
        "cursor.set",
        json!({"doc": 1, "anchor": 1, "head": 6}),
        cursor(1, 1, 6, 1, 1),
    );
    r.push(
        "doc.insert",
        json!({"doc": 1, "offset": 0, "text": "XY"}),
        Expect::AnyOk,
    );
    r.push("cursor.get", json!({"doc": 1}), cursor(1, 3, 8, 1, 1));
    r.push(
        "doc.delete",
        json!({"doc": 1, "start": 4, "end": 7}),
        Expect::Result(json!({"removed": "c\r\n", "len_chars": 7, "dirty": true})),
    );
    r.push(
        "doc.text",
        json!({"doc": 1}),
        Expect::Result(json!({"text": "XYabdef", "len_chars": 7})),
    );
    r.push("cursor.get", json!({"doc": 1}), cursor(1, 3, 5, 0, 5));

    // (g) 2: word movement forward then backward (collapsed: anchor follows head).
    for head in [2, 10, 12, 16] {
        r.push(
            "cursor.move",
            json!({"doc": 2, "by": "word", "forward": true}),
            cursor(2, head, head, 0, head),
        );
    }
    for head in [14, 11, 3, 0] {
        r.push(
            "cursor.move",
            json!({"doc": 2, "by": "word", "forward": false}),
            cursor(2, head, head, 0, head),
        );
    }
    // Extend moves only the head; a later move without extend collapses at the moved head.
    r.push(
        "cursor.set",
        json!({"doc": 2, "anchor": 3}),
        cursor(2, 3, 3, 0, 3),
    );
    r.push(
        "cursor.move",
        json!({"doc": 2, "by": "word", "forward": true, "extend": true}),
        cursor(2, 3, 10, 0, 10),
    );
    r.push(
        "cursor.move",
        json!({"doc": 2, "by": "char", "forward": true, "extend": true}),
        cursor(2, 3, 11, 0, 11),
    );
    r.push(
        "cursor.move",
        json!({"doc": 2, "by": "word", "forward": false, "extend": false}),
        cursor(2, 3, 3, 0, 3),
    );
    r.push(
        "cursor.set",
        json!({"doc": 2, "anchor": 10, "head": 10}),
        cursor(2, 10, 10, 0, 10),
    );
    r.push(
        "cursor.move",
        json!({"doc": 2, "by": "word", "forward": false, "extend": true}),
        cursor(2, 10, 3, 0, 3),
    );
    // The movement starts from the head, not from the selection's start or end.
    r.push(
        "cursor.move",
        json!({"doc": 2, "by": "char", "forward": true}),
        cursor(2, 4, 4, 0, 4),
    );

    // (g) 3: line movement keeps the column, clamped, with no goal column.
    r.push(
        "cursor.set",
        json!({"doc": 3, "anchor": 5}),
        cursor(3, 5, 5, 0, 5),
    );
    r.push(
        "cursor.move",
        json!({"doc": 3, "by": "line", "forward": true}),
        cursor(3, 9, 9, 1, 2),
    );
    r.push(
        "cursor.move",
        json!({"doc": 3, "by": "line", "forward": true}),
        cursor(3, 12, 12, 2, 2),
    );
    // On the last line, down gives len_chars; on the first, up gives 0.
    r.push(
        "cursor.move",
        json!({"doc": 3, "by": "line", "forward": true}),
        cursor(3, 16, 16, 2, 6),
    );
    r.push(
        "cursor.move",
        json!({"doc": 3, "by": "line", "forward": false}),
        cursor(3, 9, 9, 1, 2),
    );
    r.push(
        "cursor.move",
        json!({"doc": 3, "by": "line", "forward": false}),
        cursor(3, 2, 2, 0, 2),
    );
    r.push(
        "cursor.move",
        json!({"doc": 3, "by": "line", "forward": false}),
        cursor(3, 0, 0, 0, 0),
    );

    // (g) 4: char movement steps over a CRLF as one; never past either end.
    r.push(
        "cursor.set",
        json!({"doc": 4, "anchor": 1}),
        cursor(4, 1, 1, 0, 1),
    );
    r.push(
        "cursor.move",
        json!({"doc": 4, "by": "char", "forward": true}),
        cursor(4, 3, 3, 1, 0),
    );
    r.push(
        "cursor.move",
        json!({"doc": 4, "by": "char", "forward": true}),
        cursor(4, 4, 4, 1, 1),
    );
    r.push(
        "cursor.move",
        json!({"doc": 4, "by": "char", "forward": true}),
        cursor(4, 4, 4, 1, 1),
    );
    r.push(
        "cursor.move",
        json!({"doc": 4, "by": "char", "forward": false}),
        cursor(4, 3, 3, 1, 0),
    );
    r.push(
        "cursor.move",
        json!({"doc": 4, "by": "char", "forward": false}),
        cursor(4, 1, 1, 0, 1),
    );
    r.push(
        "cursor.move",
        json!({"doc": 4, "by": "char", "forward": false}),
        cursor(4, 0, 0, 0, 0),
    );
    r.push(
        "cursor.move",
        json!({"doc": 4, "by": "char", "forward": false}),
        cursor(4, 0, 0, 0, 0),
    );

    // (g) 5: eol.normalize to LF, one undo restores.
    r.push(
        "eol.counts",
        json!({"doc": 5}),
        Expect::Result(json!({"lf": 1, "crlf": 1, "cr": 1, "line_ending": "lf",
            "mixed_line_endings": true})),
    );
    r.push(
        "eol.normalize",
        json!({"doc": 5, "to": "lf"}),
        Expect::Result(json!({"changed": 2, "len_chars": 7, "line_ending": "lf",
            "mixed_line_endings": false})),
    );
    r.push(
        "doc.text",
        json!({"doc": 5}),
        Expect::Result(json!({"text": "a\nb\nc\nd", "len_chars": 7})),
    );
    r.push(
        "eol.counts",
        json!({"doc": 5}),
        Expect::Result(json!({"lf": 3, "crlf": 0, "cr": 0, "line_ending": "lf",
            "mixed_line_endings": false})),
    );
    r.push(
        "doc.undo",
        json!({"doc": 5}),
        Expect::Result(json!({"undone": true, "len_chars": 8, "dirty": false})),
    );
    r.push(
        "doc.text",
        json!({"doc": 5}),
        Expect::Result(json!({"text": "a\r\nb\nc\rd", "len_chars": 8})),
    );
    r.run_script(&work, &root, "examples");
}

// ---------------------------------------------------------------- refusals (AC-3)

/// AC-3 and Pinned (a): refusals of `cursor.set` (beyond the text → `out_of_range`, inside a
/// CRLF → `not_a_boundary`, `anchor` checked before `head`), unknown documents for every new
/// command (`no_such_document`, after the params schema, before the position), schema errors;
/// afterwards `cursor.get` is unchanged and an open typing group is still open.
#[test]
fn cursor_refusals() {
    let work = fresh_dir("refusals");
    let root = work.join("root");
    std::fs::create_dir_all(&root).expect("root");
    // "ab\r\ncé😀": 7 chars; 3 lies between \r and \n.
    std::fs::write(root.join("a.txt"), "ab\r\ncé😀").expect("write file");
    std::fs::write(root.join("b.txt"), "x").expect("write file");
    let mut r = Requests::default();
    r.push("doc.open", json!({"path": "a.txt"}), Expect::AnyOk);
    r.push(
        "cursor.set",
        json!({"doc": 1, "anchor": 1, "head": 6}),
        cursor(1, 1, 6, 1, 2),
    );
    let unchanged = || cursor(1, 1, 6, 1, 2);

    let refused: [(Value, &str); 9] = [
        (json!({"doc": 1, "anchor": 8}), "out_of_range"),
        (json!({"doc": 1, "anchor": 8, "head": 0}), "out_of_range"),
        (json!({"doc": 1, "anchor": 1_000_000}), "out_of_range"),
        (json!({"doc": 1, "anchor": 3}), "not_a_boundary"),
        (json!({"doc": 1, "anchor": 3, "head": 0}), "not_a_boundary"),
        (json!({"doc": 1, "anchor": 0, "head": 8}), "out_of_range"),
        (json!({"doc": 1, "anchor": 0, "head": 3}), "not_a_boundary"),
        // Precedence: anchor before head.
        (json!({"doc": 1, "anchor": 3, "head": 8}), "not_a_boundary"),
        (json!({"doc": 1, "anchor": 9, "head": 3}), "out_of_range"),
    ];
    for (params, kind) in refused {
        r.push("cursor.set", params, rejected(kind));
        r.push("cursor.get", json!({"doc": 1}), unchanged());
    }
    // The ends of the text and the position before the \r are valid.
    r.push(
        "cursor.set",
        json!({"doc": 1, "anchor": 7, "head": 2}),
        cursor(1, 7, 2, 0, 2),
    );
    r.push(
        "cursor.set",
        json!({"doc": 1, "anchor": 0, "head": 7}),
        cursor(1, 0, 7, 1, 3),
    );
    r.push(
        "cursor.set",
        json!({"doc": 1, "anchor": 1, "head": 6}),
        unchanged(),
    );

    // Unknown documents: never opened, id 0, closed; the document is checked before positions.
    r.push("doc.open", json!({"path": "b.txt"}), Expect::AnyOk);
    r.push("doc.close", json!({"doc": 2}), Expect::AnyOk);
    for doc in [0, 2, 9] {
        let calls = [
            ("cursor.get", json!({"doc": doc})),
            ("cursor.set", json!({"doc": doc, "anchor": 0})),
            (
                "cursor.set",
                json!({"doc": doc, "anchor": 999, "head": 999}),
            ),
            (
                "cursor.move",
                json!({"doc": doc, "by": "char", "forward": true}),
            ),
            ("eol.counts", json!({"doc": doc})),
            ("eol.normalize", json!({"doc": doc})),
            ("eol.normalize", json!({"doc": doc, "to": "crlf"})),
        ];
        for (command, params) in calls {
            r.push(command, params, rejected("no_such_document"));
        }
    }
    r.push("cursor.get", json!({"doc": 1}), unchanged());

    // Params schema first: even for an unknown document.
    let bad: [(&str, Value); 8] = [
        ("cursor.set", json!({"doc": 9, "anchor": "1"})),
        ("cursor.set", json!({"doc": 1})),
        ("cursor.set", json!({"doc": 1, "anchor": -1})),
        (
            "cursor.move",
            json!({"doc": 1, "by": "page", "forward": true}),
        ),
        ("cursor.move", json!({"doc": 1, "by": "char"})),
        (
            "cursor.move",
            json!({"doc": 1, "by": "char", "forward": true, "extend": "yes"}),
        ),
        ("eol.normalize", json!({"doc": 1, "to": "LF"})),
        ("eol.normalize", json!({"doc": 9, "to": "none"})),
    ];
    for (command, params) in bad {
        r.push(command, params, schema_error());
        r.push("cursor.get", json!({"doc": 1}), unchanged());
    }

    // A refused cursor.set does not end the typing group; a successful one does.
    r.push(
        "cursor.set",
        json!({"doc": 1, "anchor": 0}),
        cursor(1, 0, 0, 0, 0),
    );
    r.push(
        "doc.insert",
        json!({"doc": 1, "offset": 0, "text": "p"}),
        Expect::AnyOk,
    );
    r.push(
        "cursor.set",
        json!({"doc": 1, "anchor": 99}),
        rejected("out_of_range"),
    );
    r.push(
        "cursor.set",
        json!({"doc": 1, "anchor": 4}),
        rejected("not_a_boundary"),
    );
    r.push(
        "doc.insert",
        json!({"doc": 1, "offset": 1, "text": "q"}),
        Expect::AnyOk,
    );
    r.push("cursor.get", json!({"doc": 1}), cursor(1, 2, 2, 0, 2));
    r.push(
        "doc.undo",
        json!({"doc": 1}),
        Expect::Result(json!({"undone": true, "len_chars": 7, "dirty": false})),
    );
    r.push("cursor.get", json!({"doc": 1}), cursor(1, 0, 0, 0, 0));
    r.push(
        "doc.insert",
        json!({"doc": 1, "offset": 0, "text": "p"}),
        Expect::AnyOk,
    );
    r.push(
        "cursor.set",
        json!({"doc": 1, "anchor": 1}),
        cursor(1, 1, 1, 0, 1),
    );
    r.push(
        "doc.insert",
        json!({"doc": 1, "offset": 1, "text": "q"}),
        Expect::AnyOk,
    );
    r.push(
        "doc.undo",
        json!({"doc": 1}),
        Expect::Result(json!({"undone": true, "len_chars": 8, "dirty": true})),
    );
    r.push(
        "doc.text",
        json!({"doc": 1}),
        Expect::Result(json!({"text": "pab\r\ncé😀", "len_chars": 8})),
    );
    // So does a successful cursor.move, even one that does not move the head.
    r.push(
        "cursor.set",
        json!({"doc": 1, "anchor": 0}),
        cursor(1, 0, 0, 0, 0),
    );
    r.push(
        "doc.insert",
        json!({"doc": 1, "offset": 1, "text": "q"}),
        Expect::AnyOk,
    );
    r.push("cursor.get", json!({"doc": 1}), cursor(1, 0, 0, 0, 0));
    r.push(
        "cursor.move",
        json!({"doc": 1, "by": "line", "forward": false}),
        cursor(1, 0, 0, 0, 0),
    );
    r.push(
        "doc.insert",
        json!({"doc": 1, "offset": 2, "text": "r"}),
        Expect::AnyOk,
    );
    r.push(
        "doc.undo",
        json!({"doc": 1}),
        Expect::Result(json!({"undone": true, "len_chars": 9, "dirty": true})),
    );
    r.push(
        "doc.text",
        json!({"doc": 1}),
        Expect::Result(json!({"text": "pqab\r\ncé😀", "len_chars": 9})),
    );
    r.run_script(&work, &root, "refusals");
}

// ---------------------------------------------------------------- line endings (AC-4, AC-5)

/// Naive per-kind break counts, CRLF matched first: (lf, crlf, cr).
fn naive_counts(text: &str) -> (usize, usize, usize) {
    let chars: Vec<char> = text.chars().collect();
    let (mut lf, mut crlf, mut cr) = (0, 0, 0);
    let mut i = 0;
    while i < chars.len() {
        match break_len(&chars, i) {
            2 => crlf += 1,
            1 if chars[i] == '\n' => lf += 1,
            1 => cr += 1,
            _ => {}
        }
        i += break_len(&chars, i).max(1);
    }
    (lf, crlf, cr)
}

/// Dominant kind: most frequent; ties go to LF, then CRLF, then CR; "none" without breaks.
fn naive_dominant((lf, crlf, cr): (usize, usize, usize)) -> &'static str {
    if lf + crlf + cr == 0 {
        "none"
    } else if lf >= crlf && lf >= cr {
        "lf"
    } else if crlf >= cr {
        "crlf"
    } else {
        "cr"
    }
}

fn break_text(kind: &str) -> &'static str {
    match kind {
        "lf" => "\n",
        "crlf" => "\r\n",
        "cr" => "\r",
        _ => unreachable!("kind {kind}"),
    }
}

/// Every `\r\n|\r|\n` (CRLF first) replaced by `to`; and how many breaks differed from `to`.
fn naive_normalize(text: &str, to: &str) -> (String, usize) {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut changed = 0;
    let mut i = 0;
    while i < chars.len() {
        let len = break_len(&chars, i);
        if len == 0 {
            out.push(chars[i]);
            i += 1;
        } else {
            let old: String = chars[i..i + len].iter().collect();
            if old != break_text(to) {
                changed += 1;
            }
            out.push_str(break_text(to));
            i += len;
        }
    }
    (out, changed)
}

fn counts_result(text: &str) -> Value {
    let c = naive_counts(text);
    let kinds = [c.0, c.1, c.2].iter().filter(|&&n| n > 0).count();
    json!({"lf": c.0, "crlf": c.1, "cr": c.2, "line_ending": naive_dominant(c),
        "mixed_line_endings": kinds > 1})
}

/// Texts with known endings: the worked example, every tie, no break, empty, and random mixes.
fn eol_texts() -> Vec<String> {
    let mut texts: Vec<String> = [
        "a\r\nb\nc\rd",
        "a\nb\r\nc",
        "a\r\nb\rc",
        "\r\r\n\n",
        "x\ry\r\nz\r",
        "\n\r",
        "\r\n\r\n\n",
        "é😀\r\r€\n\r\n\r",
        "no breaks é😀€",
        "",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    let mut rng = Rng(0x7018_0E01);
    for _ in 0..14 {
        texts.push(random_text(&mut rng, 2, 14));
    }
    texts
}

/// AC-4: for random mixed texts saved as files, `eol.counts` equals the naive count; after
/// `eol.normalize {to}` (each target, and none) and `doc.save`, the file's bytes equal the
/// naive replacement and `changed` the number of breaks that differed from the target.
#[test]
fn eol_counts_and_normalize_match_naive_model() {
    let work = fresh_dir("eol");
    let root = work.join("root");
    std::fs::create_dir_all(&root).expect("root");
    let mut r = Requests::default();
    let mut files = Vec::new();
    let mut doc = 0u64;
    for (i, text) in eol_texts().iter().enumerate() {
        for to in [Some("lf"), Some("crlf"), Some("cr"), None] {
            doc += 1;
            let name = format!("t{i}-{}.txt", to.unwrap_or("dominant"));
            std::fs::write(root.join(&name), text).expect("write file");
            let target = to.unwrap_or_else(|| naive_dominant(naive_counts(text)));
            let (want, changed) = if target == "none" {
                (text.clone(), 0)
            } else {
                naive_normalize(text, target)
            };
            r.push("doc.open", json!({"path": name}), Expect::AnyOk);
            r.push(
                "eol.counts",
                json!({"doc": doc}),
                Expect::Result(counts_result(text)),
            );
            let params = match to {
                Some(t) => json!({"doc": doc, "to": t}),
                None => json!({"doc": doc}),
            };
            let after = naive_counts(&want);
            r.push(
                "eol.normalize",
                params,
                Expect::Result(json!({
                    "changed": changed,
                    "len_chars": want.chars().count(),
                    "line_ending": naive_dominant(after),
                    "mixed_line_endings": false,
                })),
            );
            r.push(
                "eol.counts",
                json!({"doc": doc}),
                Expect::Result(counts_result(&want)),
            );
            r.push("doc.save", json!({"doc": doc}), Expect::AnyOk);
            files.push((name, text.clone(), want));
        }
    }
    r.run_script(&work, &root, "eol");
    for (name, original, want) in files {
        let got = std::fs::read(root.join(&name)).expect("read saved file");
        assert_eq!(
            got,
            want.as_bytes(),
            "{name}: original {original:?}, saved {:?}",
            String::from_utf8_lossy(&got)
        );
    }
}

/// AC-5: after `eol.normalize`, one `doc.undo` restores the exact text and `doc.redo`
/// re-applies it (one undo step each way); a file opened and saved without `eol.normalize`
/// keeps its bytes; a text without line breaks is left alone.
#[test]
fn eol_normalize_is_one_undo_step() {
    let work = fresh_dir("eol-undo");
    let root = work.join("root");
    std::fs::create_dir_all(&root).expect("root");
    let mut r = Requests::default();
    let mut doc = 0u64;
    let mut checks = Vec::new();
    let texts: Vec<String> = eol_texts()
        .into_iter()
        .filter(|t| naive_counts(t) != (0, 0, 0))
        .take(12)
        .collect();
    for (i, text) in texts.iter().enumerate() {
        let n = text.chars().count();
        for to in ["lf", "crlf", "cr"] {
            doc += 1;
            let name = format!("u{i}-{to}.txt");
            std::fs::write(root.join(&name), text).expect("write file");
            let (want, changed) = naive_normalize(text, to);
            let m = want.chars().count();
            let text_is =
                |t: &str| Expect::Result(json!({"text": t, "len_chars": t.chars().count()}));
            r.push("doc.open", json!({"path": name}), Expect::AnyOk);
            r.push(
                "eol.normalize",
                json!({"doc": doc, "to": to}),
                Expect::Result(
                    json!({"changed": changed, "len_chars": m, "line_ending": to,
                    "mixed_line_endings": false}),
                ),
            );
            r.push("doc.text", json!({"doc": doc}), text_is(&want));
            if changed == 0 {
                continue;
            }
            r.push(
                "doc.undo",
                json!({"doc": doc}),
                Expect::Result(json!({"undone": true, "len_chars": n, "dirty": false})),
            );
            r.push("doc.text", json!({"doc": doc}), text_is(text));
            // One step: nothing older to undo.
            r.push(
                "doc.undo",
                json!({"doc": doc}),
                Expect::Result(json!({"undone": false, "len_chars": n, "dirty": false})),
            );
            r.push(
                "doc.redo",
                json!({"doc": doc}),
                Expect::Result(json!({"redone": true, "len_chars": m, "dirty": true})),
            );
            r.push("doc.text", json!({"doc": doc}), text_is(&want));
            r.push(
                "doc.redo",
                json!({"doc": doc}),
                Expect::Result(json!({"redone": false, "len_chars": m, "dirty": true})),
            );
            r.push("doc.save", json!({"doc": doc}), Expect::AnyOk);
            checks.push((name, want));
        }
    }
    // A text without line breaks: nothing changes, nothing to undo.
    std::fs::write(root.join("flat.txt"), "é😀 x").expect("write file");
    doc += 1;
    r.push("doc.open", json!({"path": "flat.txt"}), Expect::AnyOk);
    for to in [json!({"doc": doc}), json!({"doc": doc, "to": "crlf"})] {
        r.push(
            "eol.normalize",
            to,
            Expect::Result(json!({"changed": 0, "len_chars": 4, "line_ending": "none",
                "mixed_line_endings": false})),
        );
    }
    r.push(
        "doc.undo",
        json!({"doc": doc}),
        Expect::Result(json!({"undone": false, "len_chars": 4, "dirty": false})),
    );
    // io.crlf regression: open and save (with an edit undone) without eol.normalize.
    let mixed = "a\r\nb\nc\r\r\nd\r";
    std::fs::write(root.join("keep.txt"), mixed).expect("write file");
    doc += 1;
    r.push("doc.open", json!({"path": "keep.txt"}), Expect::AnyOk);
    r.push(
        "eol.counts",
        json!({"doc": doc}),
        Expect::Result(counts_result(mixed)),
    );
    r.push("doc.save", json!({"doc": doc}), Expect::AnyOk);
    r.push(
        "doc.insert",
        json!({"doc": doc, "offset": 0, "text": "z"}),
        Expect::AnyOk,
    );
    r.push("doc.undo", json!({"doc": doc}), Expect::AnyOk);
    r.push("doc.save", json!({"doc": doc}), Expect::AnyOk);
    checks.push(("keep.txt".to_owned(), mixed.to_owned()));
    r.run_script(&work, &root, "eol-undo");
    for (name, want) in checks {
        let got = std::fs::read(root.join(&name)).expect("read saved file");
        assert_eq!(got, want.as_bytes(), "{name}");
    }
}

// ---------------------------------------------------------------- generated docs

/// The generated `docs/commands.json` lists the five new commands with the mutating flags of
/// the spec's table, and the refusal kinds they use are published (no new kind is needed).
#[test]
fn commands_json_lists_cursor_and_eol_commands() {
    let doc: Value = serde_json::from_str(&repo_file("docs/commands.json")).expect("JSON");
    let commands = doc["commands"].as_array().expect("commands array");
    for (name, mutating) in [
        ("cursor.get", false),
        ("cursor.set", true),
        ("cursor.move", true),
        ("eol.counts", false),
        ("eol.normalize", true),
    ] {
        let entry = commands
            .iter()
            .find(|c| c["name"] == json!(name))
            .unwrap_or_else(|| panic!("{name} missing from docs/commands.json"));
        assert_eq!(entry["mutating"], json!(mutating), "{name}");
    }
    let kinds = doc["error_kinds"].to_string();
    for kind in ["no_such_document", "out_of_range", "not_a_boundary"] {
        assert!(
            kinds.contains(&format!("\"{kind}\"")),
            "{kind} in error_kinds"
        );
    }
    for (file, names) in [
        (
            "docs/protocol/cursor.md",
            ["cursor.get", "cursor.set", "cursor.move"].as_slice(),
        ),
        (
            "docs/protocol/eol.md",
            ["eol.counts", "eol.normalize"].as_slice(),
        ),
    ] {
        let text = repo_file(file);
        for name in names {
            assert!(text.contains(name), "{file} describes {name}");
        }
    }
}
