//! T-020 acceptance tests (AC-1 … AC-10) through the built `kiln-cli` binary: `kiln-cli run`
//! scripts and `kiln-cli control` sessions on fixtures in fresh temporary roots, and
//! `kiln-cli commands --schema`. Written by the independent test author from the task
//! specification alone (`tasks/T-020-syntax-tree-highlight-brackets.md`).
//!
//! Oracles: the corpus's recorded rustc verdict (MANIFEST `set`, AC-1); a from-scratch parse in a
//! fresh session (AC-2); the constructed partition property (AC-3); `proc_macro2` tokens (AC-4,
//! AC-6); `syn` fn items (AC-5); the exact values of the spec's Pinned (g) and AC-7, AC-8, AC-10.
//! No expected value is derived by running the implementation.
//!
//! On base no `syntax.*` command exists, so every test fails at its first `syntax.*` response
//! (`unknown_command`), or, in `syntax_commands_are_read_only`, at the missing schema entry.
//!
//! No test asserts a message text, a time, the list of commands or the number or order of
//! `error_kinds` entries; refusals are checked by `code` and `kind` only. Every spawned process
//! has [`HANG`] as a hang detector (D-028 (8)): it is not an assertion about speed.
#![cfg(test)]

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Output, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use serde_json::{Value, json};
use syn::visit::Visit;

/// Hang detector for every spawned `kiln-cli` (whole process for `run`/`commands`, each
/// response for `control`). Deliberately generous; reaching it means the process hung.
const HANG: Duration = Duration::from_secs(600);

/// The size limit of the spec (Size limit, D-056): 2^20 chars.
const LIMIT: usize = 1_048_576;

/// The seven highlight kinds (Interface, `syntax.highlight` spans).
const KINDS: [&str; 7] = [
    "comment", "string", "number", "keyword", "type", "function", "plain",
];

/// The pinned keyword list of highlight rule 4 (named leaves `self`, `crate`, `super` and
/// `mutable_specifier` have the texts `self`, `crate`, `super`, `mut`, all in this list).
const KEYWORDS: &[&str] = &[
    "as",
    "async",
    "await",
    "break",
    "const",
    "continue",
    "crate",
    "dyn",
    "else",
    "enum",
    "extern",
    "false",
    "fn",
    "for",
    "gen",
    "if",
    "impl",
    "in",
    "let",
    "loop",
    "match",
    "mod",
    "move",
    "mut",
    "pub",
    "ref",
    "return",
    "self",
    "Self",
    "static",
    "struct",
    "super",
    "trait",
    "true",
    "type",
    "unsafe",
    "use",
    "where",
    "while",
    "yield",
    "abstract",
    "become",
    "box",
    "do",
    "final",
    "macro",
    "override",
    "priv",
    "typeof",
    "unsized",
    "virtual",
    "try",
    "union",
    "default",
    "safe",
    "raw",
    "auto",
    "macro_rules!",
];

/// Worked example of Pinned (g) (30 chars).
const EX_HIGHLIGHT: &str = "fn f() { let s: u8 = 1; } // c";

/// Its spans, exactly as Pinned (g) gives them.
const EX_HIGHLIGHT_SPANS: &[(usize, usize, &str)] = &[
    (0, 2, "keyword"),
    (2, 3, "plain"),
    (3, 4, "function"),
    (4, 9, "plain"),
    (9, 12, "keyword"),
    (12, 16, "plain"),
    (16, 18, "type"),
    (18, 21, "plain"),
    (21, 22, "number"),
    (22, 26, "plain"),
    (26, 30, "comment"),
];

/// A text exercising highlight rules 4 to 6 beyond the worked example: named leaves
/// `mutable_specifier`, `self`, `super`; a `type_identifier`; callees through a
/// `scoped_identifier`, a `field_expression` and a `generic_function`. The spans are worked
/// out by hand from the rules of Interface (not given in the spec; see the test author's log).
const EX_RULES: &str = "pub fn m(&mut self) -> S { let mut v = super::f(); self.g(); h::<u8>(); }";

const EX_RULES_SPANS: &[(usize, usize, &str)] = &[
    (0, 3, "keyword"),
    (3, 4, "plain"),
    (4, 6, "keyword"),
    (6, 7, "plain"),
    (7, 8, "function"),
    (8, 10, "plain"),
    (10, 13, "keyword"),
    (13, 14, "plain"),
    (14, 18, "keyword"),
    (18, 23, "plain"),
    (23, 24, "type"),
    (24, 27, "plain"),
    (27, 30, "keyword"),
    (30, 31, "plain"),
    (31, 34, "keyword"),
    (34, 39, "plain"),
    (39, 44, "keyword"),
    (44, 46, "plain"),
    (46, 47, "function"),
    (47, 51, "plain"),
    (51, 55, "keyword"),
    (55, 56, "plain"),
    (56, 57, "function"),
    (57, 61, "plain"),
    (61, 62, "function"),
    (62, 65, "plain"),
    (65, 67, "type"),
    (67, 73, "plain"),
];

/// T-016's pinned sexp of `fn f() {}` (one line), quoted by Pinned (g).
const EX_SEXP: &str = "(source_file@0..9 (function_item@0..9 name:(identifier@3..4) \
                       parameters:(parameters@4..6) body:(block@7..9)))";

// ---------------------------------------------------------------------------------------------
// Fixtures and processes
// ---------------------------------------------------------------------------------------------

/// A fresh temporary directory (removed again when dropped, also on a failing test) holding a
/// `root/` for `--root` and the test's scripts beside it.
struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("acceptance_t020_cli")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("root")).unwrap();
        Fixture { dir }
    }

    fn root(&self) -> PathBuf {
        self.dir.join("root")
    }

    /// Write `bytes` to the root-relative path `rel`.
    fn write(&self, rel: &str, bytes: &[u8]) {
        let p = self.root().join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, bytes).unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Run `kiln-cli args` with empty stdin, capturing stdout and stderr. The process is killed
/// after [`HANG`] (hang detector, not a speed assertion).
fn kiln(args: &[&str]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn kiln-cli");
    let drain = |r: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut r = r;
            let mut buf = Vec::new();
            let _ = r.read_to_end(&mut buf);
            buf
        })
    };
    let out = drain(Box::new(child.stdout.take().unwrap()));
    let err = drain(Box::new(child.stderr.take().unwrap()));
    let status = wait_or_kill(&mut child, &format!("kiln-cli {args:?}"));
    Output {
        status,
        stdout: out.join().unwrap(),
        stderr: err.join().unwrap(),
    }
}

/// Wait for `child`; kill it and fail after [`HANG`] (hang detector).
fn wait_or_kill(child: &mut Child, what: &str) -> ExitStatus {
    let deadline = Instant::now() + HANG;
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("hang detector: {what} hung (still running after {HANG:?})");
        }
        thread::sleep(Duration::from_millis(5));
    }
}

/// A `kiln-cli run` script: requests get ids 0, 1, 2, … in push order.
#[derive(Default)]
struct Script {
    lines: Vec<String>,
}

impl Script {
    /// Append a request; returns its id (= index of its response).
    fn push(&mut self, command: &str, params: Value) -> usize {
        let id = self.lines.len();
        self.lines
            .push(json!({"id": id, "command": command, "params": params}).to_string());
        id
    }

    /// Run the script with `kiln-cli run <script> --root <fixture root>`; one response per
    /// request, matched by id. Returns the responses and the exit status.
    fn run(&self, fx: &Fixture, name: &str) -> (Vec<Value>, ExitStatus) {
        let path = fx.dir.join(format!("{name}.jsonl"));
        let mut text = self.lines.join("\n");
        text.push('\n');
        fs::write(&path, text).unwrap();
        let root = fx.root();
        let out = kiln(&[
            "run",
            path.to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
        ]);
        let stdout = String::from_utf8(out.stdout).expect("stdout is UTF-8");
        let responses: Vec<Value> = stdout
            .lines()
            .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("response {l:?}: {e}")))
            .collect();
        assert_eq!(
            responses.len(),
            self.lines.len(),
            "kiln-cli run {name}: one response per request expected (status {:?})",
            out.status
        );
        for (i, r) in responses.iter().enumerate() {
            assert_eq!(r["id"], json!(i), "response {i} out of order: {r}");
        }
        (responses, out.status)
    }
}

/// A `kiln-cli control --root <dir>` session: one request line, one response line.
struct Control {
    child: Child,
    stdin: Option<ChildStdin>,
    rx: Receiver<String>,
    next: u64,
}

impl Control {
    fn start(root: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
            .args(["control", "--root", root.to_str().unwrap()])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn kiln-cli control");
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        Control {
            child,
            stdin,
            rx,
            next: 0,
        }
    }

    /// Send one request and return its response. Waiting longer than [`HANG`] for it kills the
    /// process and fails (hang detector).
    fn call(&mut self, command: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        let line = json!({"id": id, "command": command, "params": params}).to_string();
        let stdin = self.stdin.as_mut().unwrap();
        stdin.write_all(line.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();
        stdin.flush().unwrap();
        let resp = match self.rx.recv_timeout(HANG) {
            Ok(r) => r,
            Err(e) => {
                let _ = self.child.kill();
                panic!("hang detector: kiln-cli control gave no response to {command} ({e:?})");
            }
        };
        let v: Value =
            serde_json::from_str(&resp).unwrap_or_else(|e| panic!("response {resp:?}: {e}"));
        assert_eq!(v["id"], json!(id), "response out of order: {v}");
        v
    }

    /// Close stdin and wait for a normal exit (hang detector as for [`kiln`]).
    fn finish(mut self) -> ExitStatus {
        drop(self.stdin.take());
        wait_or_kill(&mut self.child, "kiln-cli control")
    }
}

impl Drop for Control {
    fn drop(&mut self) {
        drop(self.stdin.take());
        if !matches!(self.child.try_wait(), Ok(Some(_))) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// The result of an `ok` response; otherwise fail showing the response.
fn ok<'a>(resp: &'a Value, what: &str) -> &'a Value {
    assert_eq!(
        resp["ok"],
        json!(true),
        "{what}: expected an ok response, got {resp}"
    );
    &resp["result"]
}

/// Assert a refusal by `code` and `kind` only (no message is asserted).
fn refused(resp: &Value, code: &str, kind: &str, what: &str) {
    assert_eq!(
        resp["ok"],
        json!(false),
        "{what}: expected a refusal, got {resp}"
    );
    assert_eq!(
        (
            resp["error"]["code"].as_str(),
            resp["error"]["kind"].as_str()
        ),
        (Some(code), Some(kind)),
        "{what}: wrong code/kind in {resp}"
    );
}

fn doc_id(open: &Value, what: &str) -> u64 {
    ok(open, what)["doc"].as_u64().unwrap()
}

fn text_of(resp: &Value, what: &str) -> String {
    ok(resp, what)["text"].as_str().unwrap().to_owned()
}

// ---------------------------------------------------------------------------------------------
// Corpus (Pinned (b)): read MANIFEST.toml by its `path = "…"` and `set = "…"` lines
// ---------------------------------------------------------------------------------------------

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/kiln-syntax/tests/corpus")
}

/// `(path, set)` of every corpus entry, in MANIFEST order.
fn corpus() -> Vec<(String, String)> {
    let manifest = fs::read_to_string(corpus_dir().join("MANIFEST.toml")).unwrap();
    let quoted = |line: &str, key: &str| -> Option<String> {
        let rest = line.strip_prefix(key)?.trim_start().strip_prefix('=')?;
        let v = rest.trim().strip_prefix('"')?.strip_suffix('"')?;
        Some(v.to_owned())
    };
    let mut out = Vec::new();
    let mut path: Option<String> = None;
    for line in manifest.lines() {
        if let Some(p) = quoted(line, "path") {
            path = Some(p);
        } else if let Some(s) = quoted(line, "set") {
            out.push((path.take().expect("`set` before `path`"), s));
        }
    }
    for set in ["valid", "invalid"] {
        assert!(out.iter().any(|(_, s)| s == set), "no `{set}` corpus file");
    }
    out
}

/// The corpus paths of one set.
fn corpus_set(set: &str) -> Vec<String> {
    corpus()
        .into_iter()
        .filter(|(_, s)| s == set)
        .map(|(p, _)| p)
        .collect()
}

/// Copy the given corpus files into the fixture root at their manifest paths.
fn copy_corpus(fx: &Fixture, paths: &[String]) {
    for p in paths {
        fx.write(p, &fs::read(corpus_dir().join(p)).unwrap());
    }
}

// ---------------------------------------------------------------------------------------------
// Offsets, spans, randomness
// ---------------------------------------------------------------------------------------------

/// Byte offset (at a char boundary) → char offset, for `text`.
fn byte_to_char(text: &str) -> Vec<usize> {
    let mut map = vec![usize::MAX; text.len() + 1];
    let mut n = 0;
    for (b, _) in text.char_indices() {
        map[b] = n;
        n += 1;
    }
    map[text.len()] = n;
    map
}

fn char_range(map: &[usize], r: &Range<usize>) -> Range<usize> {
    let (a, b) = (map[r.start], map[r.end]);
    assert!(
        a != usize::MAX && b != usize::MAX,
        "byte range {r:?} not on char boundaries"
    );
    a..b
}

/// The spans of a `syntax.highlight` result as `(start, end, kind)`.
fn spans(result: &Value, what: &str) -> Vec<(usize, usize, String)> {
    result["spans"]
        .as_array()
        .unwrap_or_else(|| panic!("{what}: no spans list in {result}"))
        .iter()
        .map(|s| {
            (
                s["start"].as_u64().unwrap_or_else(|| panic!("{what}: {s}")) as usize,
                s["end"].as_u64().unwrap_or_else(|| panic!("{what}: {s}")) as usize,
                s["kind"]
                    .as_str()
                    .unwrap_or_else(|| panic!("{what}: {s}"))
                    .to_owned(),
            )
        })
        .collect()
}

/// AC-3's property: sorted, contiguous, non-overlapping, merged, covering `[start, end)`
/// exactly, every kind one of the seven; an empty range gives no span.
fn assert_partition(sp: &[(usize, usize, String)], start: usize, end: usize, what: &str) {
    if start == end {
        assert!(
            sp.is_empty(),
            "{what}: empty range {start}..{end} gave {sp:?}"
        );
        return;
    }
    assert!(!sp.is_empty(), "{what}: no spans for {start}..{end}");
    assert_eq!(
        sp[0].0, start,
        "{what}: first span does not start at {start}"
    );
    assert_eq!(
        sp[sp.len() - 1].1,
        end,
        "{what}: last span does not end at {end}"
    );
    for (i, s) in sp.iter().enumerate() {
        assert!(s.0 < s.1, "{what}: empty or inverted span {s:?}");
        assert!(
            KINDS.contains(&s.2.as_str()),
            "{what}: unknown kind in {s:?}"
        );
        if let Some(n) = sp.get(i + 1) {
            assert_eq!(s.1, n.0, "{what}: spans {s:?} and {n:?} not contiguous");
            assert_ne!(
                s.2, n.2,
                "{what}: adjacent spans {s:?} and {n:?} not merged"
            );
        }
    }
}

/// The kind of every char of a partition of `0..len`.
fn per_char(sp: &[(usize, usize, String)], len: usize) -> Vec<String> {
    let mut out = vec![String::new(); len];
    for (a, b, k) in sp {
        for slot in &mut out[*a..*b] {
            *slot = k.clone();
        }
    }
    out
}

/// Run-length encoding of `kinds[start..end]` as spans (the clipped, merged spans).
fn rle(kinds: &[String], start: usize, end: usize) -> Vec<(usize, usize, String)> {
    let mut out: Vec<(usize, usize, String)> = Vec::new();
    for (i, k) in kinds.iter().enumerate().take(end).skip(start) {
        match out.last_mut() {
            Some(last) if &last.2 == k => last.1 = i + 1,
            _ => out.push((i, i + 1, k.clone())),
        }
    }
    out
}

fn expect_spans(want: &[(usize, usize, &str)]) -> Vec<(usize, usize, String)> {
    want.iter()
        .map(|(a, b, k)| (*a, *b, (*k).to_owned()))
        .collect()
}

/// Deterministic xorshift64 (fixed seeds; no wall clock, no OS randomness).
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    /// Uniform-ish in `0..n` (`n > 0`).
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// `true` iff char offset `o` lies between the `\r` and `\n` of a CRLF.
fn inside_crlf(chars: &[char], o: usize) -> bool {
    o > 0 && o < chars.len() && chars[o - 1] == '\r' && chars[o] == '\n'
}

/// A char offset in `0..=len` that is not inside a CRLF.
fn boundary(rng: &mut Rng, chars: &[char]) -> usize {
    let mut o = rng.below(chars.len() + 1);
    if inside_crlf(chars, o) {
        o += 1;
    }
    o
}

// ---------------------------------------------------------------------------------------------
// AC-1 (syntax.parse)
// ---------------------------------------------------------------------------------------------

/// AC-1: every `valid` corpus file gives `has_error: false`, `errors: []` and every `invalid`
/// file `has_error: true` with ≥ 1 error, all inside the text, through `kiln-cli run`.
/// Oracle: the corpus's recorded rustc verdict (MANIFEST `set`).
#[test]
fn corpus_parses_through_protocol() {
    let fx = Fixture::new("parse");
    let files: Vec<(String, String)> = corpus()
        .into_iter()
        .filter(|(_, s)| s == "valid" || s == "invalid")
        .collect();
    let paths: Vec<String> = files.iter().map(|(p, _)| p.clone()).collect();
    copy_corpus(&fx, &paths);
    // T-016 AC-5 (pinned through `SyntaxTree::sexp`/`errors`): `fn f() { let x = 1 }` has a
    // MISSING `;` at 18..18.
    fx.write("missing.rs.txt", b"fn f() { let x = 1 }");
    let mut s = Script::default();
    let ids: Vec<(usize, usize)> = files
        .iter()
        .enumerate()
        .map(|(i, (p, _))| {
            let open = s.push("doc.open", json!({ "path": p }));
            let tree = s.push("syntax.tree", json!({ "doc": i + 1 }));
            (open, tree)
        })
        .collect();
    s.push("doc.open", json!({"path": "missing.rs.txt"}));
    let missing = s.push("syntax.tree", json!({ "doc": files.len() + 1 }));
    let (r, _) = s.run(&fx, "parse");
    let m = ok(&r[missing], "syntax.tree of `fn f() { let x = 1 }`");
    assert_eq!(m["has_error"], json!(true));
    assert!(
        m["errors"]
            .as_array()
            .unwrap()
            .contains(&json!({"kind": "missing", "start": 18, "end": 18})),
        "no MISSING error at 18..18: {}",
        m["errors"]
    );
    assert!(
        m["sexp"]
            .as_str()
            .unwrap()
            .contains(r#"value:(integer_literal@17..18) (MISSING ";"@18..18)"#),
        "sexp is not T-016's: {}",
        m["sexp"]
    );
    for ((path, set), (open, tree)) in files.iter().zip(ids) {
        let len = ok(&r[open], path)["len_chars"].as_u64().unwrap();
        let t = ok(&r[tree], &format!("syntax.tree of {path}"));
        assert_eq!(t["mode"], json!("full"), "{path}: first call: {t}");
        assert!(t["node_count"].as_u64().unwrap() >= 1, "{path}: {t}");
        assert!(
            !t["sexp"].as_str().unwrap().is_empty(),
            "{path}: empty sexp"
        );
        let errors = t["errors"].as_array().unwrap();
        if set == "valid" {
            assert_eq!(
                t["has_error"],
                json!(false),
                "{path} (valid): {}",
                t["errors"]
            );
            assert!(errors.is_empty(), "{path} (valid): errors {errors:?}");
        } else {
            assert_eq!(t["has_error"], json!(true), "{path} (invalid)");
            assert!(!errors.is_empty(), "{path} (invalid): no error span");
            for e in errors {
                let (a, b) = (e["start"].as_u64().unwrap(), e["end"].as_u64().unwrap());
                assert!(a <= b && b <= len, "{path}: error {e} outside 0..{len}");
                let kind = e["kind"].as_str().unwrap();
                assert!(kind == "error" || kind == "missing", "{path}: {e}");
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// AC-2 (syntax.incremental)
// ---------------------------------------------------------------------------------------------

/// Snippets the random edits insert (no `\r`, no BOM; some open strings, comments, brackets).
const SNIPPETS: &[&str] = &[
    "x",
    " ",
    "\n",
    "(",
    ")",
    "{",
    "}",
    "[",
    "]",
    ";",
    "\"",
    "'",
    "//",
    "/*",
    "*/",
    "é",
    "€",
    "fn g() {}",
    "let v = 1;",
    " 0x1F ",
    "r#\"s\"#",
    "b'a'",
    "1.5e3",
    "match x { _ => () }",
    "struct S<T>(T);",
];

/// The fields of `syntax.tree` that must equal a from-scratch parse.
fn tree_fields(t: &Value) -> Value {
    json!({
        "has_error": t["has_error"],
        "errors": t["errors"],
        "sexp": t["sexp"],
        "node_count": t["node_count"],
    })
}

/// One `kiln-cli control` session doing AC-2's random edits on `files` (index, path), each
/// followed by `doc.text` and `syntax.tree` and checked against the `mode` rule. Returns, per
/// file, the text and tree fields after each call, and the count of each `mode`.
#[allow(clippy::type_complexity)]
fn edit_session(
    root: &Path,
    files: &[(usize, String)],
    edits: usize,
) -> (Vec<(String, Vec<(String, Value)>)>, BTreeMap<String, usize>) {
    let mut session = Control::start(root);
    let mut seen: Vec<(String, Vec<(String, Value)>)> = Vec::new();
    let mut modes: BTreeMap<String, usize> = BTreeMap::new();
    for (fi, path) in files {
        let fi = *fi;
        let doc = doc_id(&session.call("doc.open", json!({ "path": path })), path);
        let mut rng = Rng::new(0x7020 + fi as u64);
        let mut text = text_of(&session.call("doc.text", json!({ "doc": doc })), path);
        let first = session.call("syntax.tree", json!({ "doc": doc }));
        let first = ok(&first, &format!("first syntax.tree of {path}")).clone();
        assert_eq!(first["mode"], json!("full"), "{path}: first call");
        let mut records = vec![(text.clone(), tree_fields(&first))];
        let mut prev_tree_text = text.clone();
        let mut typing_at: Option<usize> = None;
        for step in 0..edits {
            let chars: Vec<char> = text.chars().collect();
            let len = chars.len();
            let roll = rng.below(100);
            let (cmd, params) = if roll < 20 {
                // Whitespace next to whitespace: keeps an error-free tree error-free.
                let from = rng.below(len + 1);
                let o = (from..len)
                    .find(|&i| chars[i] == ' ' || chars[i] == '\n')
                    .map_or(len, |i| if inside_crlf(&chars, i) { i - 1 } else { i });
                typing_at = None;
                ("doc.insert", json!({"doc": doc, "offset": o, "text": " "}))
            } else if roll < 35 {
                let o = boundary(&mut rng, &chars);
                let t = SNIPPETS[rng.below(SNIPPETS.len())];
                typing_at = (t.chars().count() == 1).then_some(o + 1);
                ("doc.insert", json!({"doc": doc, "offset": o, "text": t}))
            } else if roll < 48 {
                // One-char typing at the end of the previous one-char insert (a typing group).
                let o = typing_at
                    .filter(|&o| o <= len && !inside_crlf(&chars, o))
                    .unwrap_or_else(|| boundary(&mut rng, &chars));
                let c = ["a", "b", "(", ")", " ", "\"", "/", "1"][rng.below(8)];
                typing_at = Some(o + 1);
                ("doc.insert", json!({"doc": doc, "offset": o, "text": c}))
            } else if roll < 60 {
                let a = boundary(&mut rng, &chars);
                let mut b = (a + 1 + rng.below(12)).min(len);
                if inside_crlf(&chars, b) {
                    b += 1;
                }
                typing_at = None;
                ("doc.delete", json!({"doc": doc, "start": a, "end": b}))
            } else if roll < 72 {
                let a = boundary(&mut rng, &chars);
                let mut b = (a + rng.below(9)).min(len);
                if inside_crlf(&chars, b) {
                    b += 1;
                }
                let t = SNIPPETS[rng.below(SNIPPETS.len())];
                typing_at = None;
                (
                    "doc.replace",
                    json!({"doc": doc, "start": a, "end": b, "text": t}),
                )
            } else if roll < 78 {
                // A replace by the same text: the text stays as it was.
                let a = boundary(&mut rng, &chars).min(len.saturating_sub(1));
                let b = (a + 1).min(len);
                let same: String = chars[a..b].iter().collect();
                typing_at = None;
                (
                    "doc.replace",
                    json!({"doc": doc, "start": a, "end": b, "text": same}),
                )
            } else {
                typing_at = None;
                ("doc.undo", json!({ "doc": doc }))
            };
            let what = format!("{path} step {step}: {cmd} {params}");
            ok(&session.call(cmd, params), &what);
            text = text_of(&session.call("doc.text", json!({ "doc": doc })), &what);
            let t = session.call("syntax.tree", json!({ "doc": doc }));
            let t = ok(&t, &format!("syntax.tree after {what}"));
            let mode = t["mode"].as_str().unwrap_or_default().to_owned();
            if text == prev_tree_text {
                assert_eq!(mode, "unchanged", "{what}: text equals the previous call's");
            } else {
                assert!(
                    mode == "incremental" || mode == "full",
                    "{what}: text changed but mode is {mode:?}"
                );
                if mode == "incremental" {
                    assert_eq!(
                        t["has_error"],
                        json!(false),
                        "{what}: `incremental` reported for a tree with errors"
                    );
                }
            }
            *modes.entry(mode).or_default() += 1;
            prev_tree_text = text.clone();
            records.push((text.clone(), tree_fields(t)));
            assert!(text.chars().count() <= LIMIT);
        }
        seen.push((path.clone(), records));
    }
    assert!(
        session.finish().success(),
        "kiln-cli control exited abnormally"
    );
    (seen, modes)
}

/// AC-2: for every `valid` file, 100 fixed-seed random `doc.insert`/`doc.delete`/
/// `doc.replace`/`doc.undo` in one `kiln-cli control` session, each followed by `doc.text` and
/// `syntax.tree`; every tree equals `syntax.tree` in a fresh `kiln-cli run` session on the same
/// text, and `mode` follows the cache rule. Plus the worked example of Pinned (g) (`fn f() {}`:
/// T-016's sexp, `full`, `unchanged`, then `incremental` after inserting a space).
/// Oracle: from-scratch parse; exact value.
#[test]
fn incremental_equals_fresh_session() {
    // Worked example (Pinned (g)).
    {
        let fx = Fixture::new("incremental-example");
        fx.write("f.rs.txt", b"fn f() {}");
        let mut s = Script::default();
        let open = s.push("doc.open", json!({"path": "f.rs.txt"}));
        let t1 = s.push("syntax.tree", json!({"doc": 1}));
        let t2 = s.push("syntax.tree", json!({"doc": 1}));
        let ins = s.push("doc.insert", json!({"doc": 1, "offset": 8, "text": " "}));
        let t3 = s.push("syntax.tree", json!({"doc": 1}));
        // The cache holds the text of the last successful call of any of the three commands.
        let ins2 = s.push("doc.insert", json!({"doc": 1, "offset": 0, "text": " "}));
        let hl = s.push("syntax.highlight", json!({"doc": 1}));
        let t4 = s.push("syntax.tree", json!({"doc": 1}));
        let ins3 = s.push("doc.insert", json!({"doc": 1, "offset": 0, "text": " "}));
        let mb = s.push("syntax.match-bracket", json!({"doc": 1, "offset": 0}));
        let t5 = s.push("syntax.tree", json!({"doc": 1}));
        let (r, _) = s.run(&fx, "example");
        ok(&r[open], "doc.open");
        let a = ok(&r[t1], "first syntax.tree of `fn f() {}`");
        assert_eq!(a["sexp"], json!(EX_SEXP));
        assert_eq!(a["has_error"], json!(false));
        assert_eq!(a["errors"], json!([]));
        assert_eq!(a["mode"], json!("full"));
        let b = ok(&r[t2], "second syntax.tree");
        assert_eq!(b["mode"], json!("unchanged"));
        assert_eq!(tree_fields(a), tree_fields(b));
        ok(&r[ins], "doc.insert");
        let c = ok(&r[t3], "syntax.tree after inserting a space");
        assert_eq!(c["mode"], json!("incremental"));
        assert_eq!(c["has_error"], json!(false));
        for id in [ins2, hl, ins3, mb] {
            ok(&r[id], "step");
        }
        let d = ok(&r[t4], "syntax.tree after insert + syntax.highlight");
        assert_eq!(
            d["mode"],
            json!("unchanged"),
            "syntax.highlight did not update the cache"
        );
        let e = ok(&r[t5], "syntax.tree after insert + syntax.match-bracket");
        assert_eq!(
            e["mode"],
            json!("unchanged"),
            "syntax.match-bracket did not update the cache"
        );
    }

    const EDITS: usize = 100;
    let valid = corpus_set("valid");
    let fx = Fixture::new("incremental");
    copy_corpus(&fx, &valid);
    // Three `kiln-cli control` sessions in parallel (run time only; each file's edits are
    // sequential within its session and the seeds are fixed per file).
    const SESSIONS: usize = 3;
    let root = fx.root();
    let indexed: Vec<(usize, String)> = valid.iter().cloned().enumerate().collect();
    let chunk_len = indexed.len().div_ceil(SESSIONS);
    let results = thread::scope(|sc| {
        let handles: Vec<_> = indexed
            .chunks(chunk_len)
            .map(|chunk| {
                let root = &root;
                sc.spawn(move || edit_session(root, chunk, EDITS))
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e)))
            .collect::<Vec<_>>()
    });
    // Per file: the texts seen and, per text, the tree fields reported.
    let mut seen: Vec<(String, Vec<(String, Value)>)> = Vec::new();
    let mut modes: BTreeMap<String, usize> = BTreeMap::new();
    for (s, m) in results {
        seen.extend(s);
        for (k, v) in m {
            *modes.entry(k).or_default() += v;
        }
    }

    // Fresh sessions: one `kiln-cli run` per corpus file, each distinct text as its own file.
    for (fi, (path, records)) in seen.iter().enumerate() {
        let fresh = Fixture::new(&format!("incremental-fresh-{fi}"));
        let mut distinct: Vec<&str> = Vec::new();
        for (t, _) in records {
            if !distinct.contains(&t.as_str()) {
                distinct.push(t);
            }
        }
        let mut s = Script::default();
        let mut ids = Vec::new();
        for (i, t) in distinct.iter().enumerate() {
            let name = format!("t{i}.rs.txt");
            fresh.write(&name, t.as_bytes());
            let o = s.push("doc.open", json!({ "path": name }));
            let x = s.push("doc.text", json!({ "doc": i + 1 }));
            let y = s.push("syntax.tree", json!({ "doc": i + 1 }));
            ids.push((o, x, y));
        }
        let (r, _) = s.run(&fresh, "fresh");
        let mut expected: BTreeMap<&str, Value> = BTreeMap::new();
        for (t, (o, x, y)) in distinct.iter().zip(ids) {
            ok(&r[o], "fresh doc.open");
            assert_eq!(
                text_of(&r[x], "fresh doc.text"),
                *t,
                "{path}: the fresh session does not hold the same text"
            );
            let tree = ok(&r[y], &format!("fresh syntax.tree for {path}"));
            assert_eq!(tree["mode"], json!("full"), "{path}: fresh session");
            expected.insert(t, tree_fields(tree));
        }
        for (step, (t, got)) in records.iter().enumerate() {
            assert_eq!(
                got,
                &expected[t.as_str()],
                "{path}: syntax.tree after step {step} differs from a fresh parse of the same text"
            );
        }
    }
    eprintln!("AC-2 modes seen: {modes:?}");
}

// ---------------------------------------------------------------------------------------------
// AC-3 (syntax.highlight, partition)
// ---------------------------------------------------------------------------------------------

/// AC-3: for every corpus file (all sets) and ≥ 50 fixed-seed random sub-ranges (some empty,
/// some with only `start` or only `end`), the spans are a merged partition of the range with the
/// seven kinds, equal to the whole-text spans clipped to the range; an empty range or an empty
/// document gives `spans: []`; offsets between `\r` and `\n` are accepted; out-of-range,
/// inverted and `null` bounds are refused. Oracle: constructed property.
#[test]
fn highlight_partitions_text() {
    const RANGES: usize = 60;
    let all: Vec<String> = corpus().into_iter().map(|(p, _)| p).collect();
    let fx = Fixture::new("partition");
    copy_corpus(&fx, &all);
    fx.write("empty.rs.txt", b"");
    let mut c = Control::start(&fx.root());
    let mut checked_crlf = false;
    for (fi, path) in all.iter().enumerate() {
        let doc = doc_id(&c.call("doc.open", json!({ "path": path })), path);
        let text = text_of(&c.call("doc.text", json!({ "doc": doc })), path);
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        let full = c.call("syntax.highlight", json!({ "doc": doc }));
        let full = spans(ok(&full, &format!("syntax.highlight of {path}")), path);
        assert_partition(&full, 0, len, &format!("{path} whole text"));
        let kinds = per_char(&full, len);
        let mut rng = Rng::new(0x3020 + fi as u64);
        for i in 0..RANGES {
            let mut a = rng.below(len + 1);
            let mut b = if i % 10 == 0 { a } else { rng.below(len + 1) };
            if a > b {
                std::mem::swap(&mut a, &mut b);
            }
            let params = match i % 6 {
                1 => json!({"doc": doc, "start": a}),
                2 => json!({"doc": doc, "end": b}),
                _ => json!({"doc": doc, "start": a, "end": b}),
            };
            let (a, b) = match i % 6 {
                1 => (a, len),
                2 => (0, b),
                _ => (a, b),
            };
            let what = format!("{path}: syntax.highlight {params}");
            let r = c.call("syntax.highlight", params);
            let sp = spans(ok(&r, &what), &what);
            assert_partition(&sp, a, b, &what);
            assert_eq!(
                sp,
                rle(&kinds, a, b),
                "{what}: not the whole-text spans clipped"
            );
        }
        // A start or end between `\r` and `\n` is a plain char offset (no boundary check).
        if let Some(cr) = (1..len).find(|&o| inside_crlf(&chars, o)) {
            let (a, b) = (cr, (cr + 7).min(len));
            let what = format!("{path}: range {a}..{b} starting inside a CRLF");
            let r = c.call(
                "syntax.highlight",
                json!({"doc": doc, "start": a, "end": b}),
            );
            let sp = spans(ok(&r, &what), &what);
            assert_partition(&sp, a, b, &what);
            assert_eq!(sp, rle(&kinds, a, b), "{what}");
            let r = c.call(
                "syntax.highlight",
                json!({"doc": doc, "start": 0, "end": cr}),
            );
            let sp = spans(ok(&r, &what), &what);
            assert_eq!(sp, rle(&kinds, 0, cr), "{what} (end inside the CRLF)");
            checked_crlf = true;
        }
        // Explicit empty ranges, at the start and at the end.
        for o in [0, len / 2, len] {
            let r = c.call(
                "syntax.highlight",
                json!({"doc": doc, "start": o, "end": o}),
            );
            assert_eq!(
                ok(&r, &format!("{path} empty range at {o}"))["spans"],
                json!([])
            );
        }
        // Refusals (code and kind only).
        let r = c.call("syntax.highlight", json!({"doc": doc, "end": len + 1}));
        refused(
            &r,
            "rejected",
            "out_of_range",
            &format!("{path} end = len + 1"),
        );
        let r = c.call("syntax.highlight", json!({"doc": doc, "start": len + 1}));
        refused(
            &r,
            "rejected",
            "out_of_range",
            &format!("{path} start = len + 1"),
        );
        if len >= 2 {
            let r = c.call(
                "syntax.highlight",
                json!({"doc": doc, "start": 2, "end": 1}),
            );
            refused(
                &r,
                "rejected",
                "inverted_range",
                &format!("{path} start > end"),
            );
        }
        let r = c.call("syntax.highlight", json!({"doc": doc, "start": null}));
        refused(
            &r,
            "invalid_params",
            "schema",
            &format!("{path} start: null"),
        );
    }
    assert!(checked_crlf, "no corpus file with a CRLF");
    let doc = doc_id(
        &c.call("doc.open", json!({"path": "empty.rs.txt"})),
        "empty",
    );
    let r = c.call("syntax.highlight", json!({ "doc": doc }));
    assert_eq!(ok(&r, "highlight of an empty document")["spans"], json!([]));
    let r = c.call("syntax.highlight", json!({"doc": 0}));
    refused(&r, "rejected", "no_such_document", "doc 0");
    assert!(c.finish().success(), "kiln-cli control exited abnormally");
}

// ---------------------------------------------------------------------------------------------
// AC-4 (syntax.highlight, differential with proc-macro2)
// ---------------------------------------------------------------------------------------------

/// `true` iff the source slice is a (desugared) comment, which proc-macro2 turns into `#[doc]`
/// tokens whose spans cover the comment.
fn is_comment_slice(s: &str) -> bool {
    s.starts_with("//") || s.starts_with("/*")
}

/// Tokens of `text` by proc-macro2: literals `(char range, "string"|"number")` whose source
/// slice equals the token text; bracket groups `(open, close)` char offsets whose source chars
/// are the delimiters. Sanity: every token whose slice does not match is a doc comment.
struct Tokens {
    literals: Vec<(Range<usize>, &'static str)>,
    groups: Vec<(usize, usize)>,
}

fn tokenize(text: &str, what: &str) -> Tokens {
    let ts: TokenStream = text
        .parse()
        .unwrap_or_else(|e| panic!("{what}: proc-macro2 cannot lex the file: {e:?}"));
    let map = byte_to_char(text);
    let mut out = Tokens {
        literals: Vec::new(),
        groups: Vec::new(),
    };
    let mut stack = vec![ts];
    while let Some(ts) = stack.pop() {
        for tt in ts {
            match tt {
                TokenTree::Group(g) => {
                    let pair = match g.delimiter() {
                        Delimiter::Parenthesis => Some(('(', ')')),
                        Delimiter::Bracket => Some(('[', ']')),
                        Delimiter::Brace => Some(('{', '}')),
                        Delimiter::None => None,
                    };
                    if let Some((o, cl)) = pair {
                        let (ro, rc) = (g.span_open().byte_range(), g.span_close().byte_range());
                        let at = |r: &Range<usize>| text[r.start..].chars().next();
                        if at(&ro) == Some(o) && at(&rc) == Some(cl) {
                            out.groups
                                .push((char_range(&map, &ro).start, char_range(&map, &rc).start));
                        } else {
                            assert!(
                                is_comment_slice(&text[ro.start..]),
                                "{what}: group at byte {ro:?} is neither a delimiter nor a doc comment"
                            );
                        }
                    }
                    stack.push(g.stream());
                }
                TokenTree::Literal(l) => {
                    let r = l.span().byte_range();
                    let s = l.to_string();
                    if text[r.clone()] == s {
                        let kind = if s.starts_with(|c: char| c.is_ascii_digit()) {
                            "number"
                        } else {
                            "string"
                        };
                        out.literals.push((char_range(&map, &r), kind));
                    } else {
                        assert!(
                            is_comment_slice(&text[r.clone()]),
                            "{what}: literal {s} at byte {r:?} does not match its source"
                        );
                    }
                }
                TokenTree::Ident(i) => {
                    let r = i.span().byte_range();
                    assert!(
                        i == text[r.clone()] || is_comment_slice(&text[r.clone()]),
                        "{what}: ident {i} at byte {r:?} does not match its source (oracle offsets)"
                    );
                }
                TokenTree::Punct(_) => {}
            }
        }
    }
    out
}

/// AC-4: on every `valid` file the chars of the matching proc-macro2 `Literal` tokens are
/// exactly the chars of `string` and `number` spans (string-like literals `string`, numeric
/// ones `number`); every `keyword` span's text is in the pinned list; every `comment` span
/// starts with `//` or `/*`. Oracle: an independent tokenizer.
#[test]
fn highlight_agrees_with_tokenizer() {
    let valid = corpus_set("valid");
    let fx = Fixture::new("tokenizer");
    copy_corpus(&fx, &valid);
    let mut s = Script::default();
    let ids: Vec<(usize, usize)> = valid
        .iter()
        .enumerate()
        .map(|(i, p)| {
            s.push("doc.open", json!({ "path": p }));
            let t = s.push("doc.text", json!({ "doc": i + 1 }));
            let h = s.push("syntax.highlight", json!({ "doc": i + 1 }));
            (t, h)
        })
        .collect();
    let (r, _) = s.run(&fx, "tokenizer");
    // Oracle first, for every file.
    let oracles: Vec<(String, Tokens)> = valid
        .iter()
        .zip(&ids)
        .map(|(p, (t, _))| {
            let text = text_of(&r[*t], p);
            let tok = tokenize(&text, p);
            (text, tok)
        })
        .collect();
    let lits: usize = oracles.iter().map(|(_, t)| t.literals.len()).sum();
    assert!(lits > 0, "the oracle found no literal in the corpus");
    for ((path, (_, h)), (text, tok)) in valid.iter().zip(&ids).zip(&oracles) {
        let chars: Vec<char> = text.chars().collect();
        let sp = spans(ok(&r[*h], &format!("syntax.highlight of {path}")), path);
        assert_partition(&sp, 0, chars.len(), path);
        let mut want: Vec<Option<&str>> = vec![None; chars.len()];
        for (range, kind) in &tok.literals {
            for slot in &mut want[range.clone()] {
                *slot = Some(kind);
            }
        }
        let mut got: Vec<Option<&str>> = vec![None; chars.len()];
        let mut bad = Vec::new();
        for (a, b, k) in &sp {
            let slice: String = chars[*a..*b].iter().collect();
            match k.as_str() {
                "string" | "number" => {
                    for slot in &mut got[*a..*b] {
                        *slot = Some(k.as_str());
                    }
                }
                "keyword" if !KEYWORDS.contains(&slice.as_str()) => {
                    bad.push(format!("keyword span {a}..{b} {slice:?} not in the list"));
                }
                "comment" if !is_comment_slice(&slice) => {
                    bad.push(format!("comment span {a}..{b} {slice:?}"));
                }
                _ => {}
            }
        }
        if let Some(o) = (0..chars.len()).find(|&o| want[o] != got[o]) {
            let ctx: String = chars[o.saturating_sub(10)..(o + 10).min(chars.len())]
                .iter()
                .collect();
            bad.push(format!(
                "at char {o} (near {ctx:?}): tokenizer says {:?}, highlight says {:?}",
                want[o], got[o]
            ));
        }
        assert!(bad.is_empty(), "{path}: {bad:#?}");
    }
}

// ---------------------------------------------------------------------------------------------
// AC-5 (syntax.highlight, functions and types)
// ---------------------------------------------------------------------------------------------

/// Byte ranges of fn definition names (free fns, impl and trait methods), not descending into
/// fn bodies.
#[derive(Default)]
struct FnNames(Vec<(Range<usize>, String)>);

impl<'ast> Visit<'ast> for FnNames {
    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        self.0
            .push((f.sig.ident.span().byte_range(), f.sig.ident.to_string()));
    }
    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
        self.0
            .push((f.sig.ident.span().byte_range(), f.sig.ident.to_string()));
    }
    fn visit_trait_item_fn(&mut self, f: &'ast syn::TraitItemFn) {
        self.0
            .push((f.sig.ident.span().byte_range(), f.sig.ident.to_string()));
    }
}

/// AC-5: on every `valid` file each fn definition name found by `syn` is a `function` span at
/// the ident's offsets; the worked example of Pinned (g) gives exactly its spans (the `type`
/// kind has only this hand-written expectation). Oracle: syn + exact value.
#[test]
fn highlight_marks_fn_names() {
    let valid = corpus_set("valid");
    let fx = Fixture::new("fn-names");
    copy_corpus(&fx, &valid);
    fx.write("example.rs.txt", EX_HIGHLIGHT.as_bytes());
    fx.write("rules.rs.txt", EX_RULES.as_bytes());
    let mut s = Script::default();
    let ids: Vec<(usize, usize)> = valid
        .iter()
        .enumerate()
        .map(|(i, p)| {
            s.push("doc.open", json!({ "path": p }));
            let t = s.push("doc.text", json!({ "doc": i + 1 }));
            let h = s.push("syntax.highlight", json!({ "doc": i + 1 }));
            (t, h)
        })
        .collect();
    let ex_doc = valid.len() + 1;
    s.push("doc.open", json!({"path": "example.rs.txt"}));
    let ex = s.push("syntax.highlight", json!({ "doc": ex_doc }));
    let ex_sub = s.push(
        "syntax.highlight",
        json!({"doc": ex_doc, "start": 10, "end": 17}),
    );
    s.push("doc.open", json!({"path": "rules.rs.txt"}));
    let rules = s.push("syntax.highlight", json!({ "doc": ex_doc + 1 }));
    let (r, _) = s.run(&fx, "fn-names");
    let mut total = 0;
    let oracles: Vec<(String, Vec<Range<usize>>)> = valid
        .iter()
        .zip(&ids)
        .map(|(p, (t, _))| {
            let text = text_of(&r[*t], p);
            let file = syn::parse_file(&text).unwrap_or_else(|e| panic!("{p}: syn: {e}"));
            let mut names = FnNames::default();
            names.visit_file(&file);
            let map = byte_to_char(&text);
            let ranges = names
                .0
                .iter()
                .map(|(br, name)| {
                    assert_eq!(&text[br.clone()], name, "{p}: oracle offsets of fn {name}");
                    char_range(&map, br)
                })
                .collect::<Vec<_>>();
            total += ranges.len();
            (text, ranges)
        })
        .collect();
    assert!(total > 0, "syn found no fn in the corpus");
    for ((path, (_, h)), (text, names)) in valid.iter().zip(&ids).zip(&oracles) {
        let sp = spans(ok(&r[*h], &format!("syntax.highlight of {path}")), path);
        assert_partition(&sp, 0, text.chars().count(), path);
        let missing: Vec<&Range<usize>> = names
            .iter()
            .filter(|n| {
                !sp.iter()
                    .any(|(a, b, k)| *a == n.start && *b == n.end && k == "function")
            })
            .collect();
        assert!(
            missing.is_empty(),
            "{path}: fn names (char ranges) without a `function` span: {missing:?}"
        );
    }
    let got = spans(ok(&r[ex], "worked example"), "worked example");
    assert_eq!(
        got,
        expect_spans(EX_HIGHLIGHT_SPANS),
        "worked example spans"
    );
    let got = spans(ok(&r[ex_sub], "worked example 10..17"), "worked example");
    assert_eq!(
        got,
        expect_spans(&[(10, 12, "keyword"), (12, 16, "plain"), (16, 17, "type")]),
        "worked example clipped to 10..17"
    );
    let want = expect_spans(EX_RULES_SPANS);
    let chars: Vec<char> = EX_RULES.chars().collect();
    for (a, b, k) in &want {
        let slice: String = chars[*a..*b].iter().collect();
        if k == "keyword" {
            assert!(KEYWORDS.contains(&slice.as_str()), "fixture: {slice:?}");
        }
    }
    let got = spans(ok(&r[rules], "rules fixture"), "rules fixture");
    assert_eq!(got, want, "spans of {EX_RULES:?}");
}

// ---------------------------------------------------------------------------------------------
// AC-6 (syntax.brackets, differential)
// ---------------------------------------------------------------------------------------------

fn is_bracket(c: char) -> bool {
    matches!(c, '(' | ')' | '[' | ']' | '{' | '}')
}

/// AC-6: on every `valid` file, `match-bracket` at the open and at the close offset of each
/// proc-macro2 bracket group returns that pair; at every other offset holding a bracket char
/// (inside strings, chars, comments) it returns `null`. Oracle: an independent tokenizer.
#[test]
fn brackets_agree_with_token_groups() {
    let valid = corpus_set("valid");
    let fx = Fixture::new("brackets");
    copy_corpus(&fx, &valid);
    let mut c = Control::start(&fx.root());
    // Oracle first, for every file.
    let mut files = Vec::new();
    for path in &valid {
        let doc = doc_id(&c.call("doc.open", json!({ "path": path })), path);
        let text = text_of(&c.call("doc.text", json!({ "doc": doc })), path);
        let tok = tokenize(&text, path);
        let mut want: BTreeMap<usize, Value> = BTreeMap::new();
        for (o, cl) in &tok.groups {
            let pair = json!({"open": o, "close": cl});
            assert!(want.insert(*o, pair.clone()).is_none(), "{path}: oracle");
            assert!(want.insert(*cl, pair).is_none(), "{path}: oracle");
        }
        let brackets: Vec<usize> = text
            .chars()
            .enumerate()
            .filter(|(_, ch)| is_bracket(*ch))
            .map(|(i, _)| i)
            .collect();
        files.push((path, doc, want, brackets));
    }
    let mut nulls = 0;
    for (path, doc, want, brackets) in &files {
        let mut bad = Vec::new();
        for &o in brackets {
            let r = c.call("syntax.match-bracket", json!({"doc": doc, "offset": o}));
            let got = &ok(&r, &format!("{path}: syntax.match-bracket at {o}"))["bracket"];
            let expected = want.get(&o).cloned().unwrap_or(Value::Null);
            if expected.is_null() {
                nulls += 1;
            }
            if *got != expected {
                bad.push(format!("offset {o}: want {expected}, got {got}"));
            }
        }
        assert!(
            want.keys().all(|o| brackets.contains(o)),
            "{path}: oracle group outside the text's bracket chars"
        );
        assert!(bad.is_empty(), "{path}: {} mismatches: {bad:#?}", bad.len());
    }
    assert!(
        nulls > 0,
        "no bracket inside a string, char or comment in the corpus"
    );
    assert!(c.finish().success(), "kiln-cli control exited abnormally");
}

// ---------------------------------------------------------------------------------------------
// AC-7 (syntax.brackets, editing)
// ---------------------------------------------------------------------------------------------

/// AC-7: the worked examples of Pinned (g) through `kiln-cli run`, including the two
/// stack-pairing examples; `offset = len_chars` → `null`, `len_chars + 1` →
/// `rejected`/`out_of_range`; `<`/`>` and brackets in strings and comments → `null`; the same
/// answers after edits turn one example text into another and after undo. Oracle: exact value.
#[test]
fn bracket_worked_examples() {
    let fx = Fixture::new("bracket-examples");
    let files: [(&str, &str); 5] = [
        ("e1.rs.txt", EX_HIGHLIGHT),
        ("e2.rs.txt", "fn a() { ("),
        ("e3.rs.txt", ") ( ( ) ] ["),
        ("e4.rs.txt", "fn a() { ( ) ("),
        ("e5.rs.txt", "fn g<T>(x: &str) { let s = \"(\"; } // )"),
    ];
    for (name, text) in files {
        fx.write(name, text.as_bytes());
    }
    fx.write("edit.rs.txt", b"fn a() { (");
    // Offsets of e5 checked against its text, so the expectations below are not miscounted.
    let e5: Vec<char> = files[4].1.chars().collect();
    for (o, ch) in [
        (4, '<'),
        (6, '>'),
        (7, '('),
        (15, ')'),
        (17, '{'),
        (28, '('),
        (32, '}'),
        (37, ')'),
    ] {
        assert_eq!(e5[o], ch, "e5 offset {o}");
    }
    assert_eq!(e5.len(), 38);

    let pair = |o: usize, c: usize| json!({"open": o, "close": c});
    let null = Value::Null;
    // (doc, offset, expected bracket)
    let cases: Vec<(u64, usize, Value)> = vec![
        (1, 4, pair(4, 5)),
        (1, 5, pair(4, 5)),
        (1, 7, pair(7, 24)),
        (1, 24, pair(7, 24)),
        (1, 0, null.clone()),
        (1, 3, null.clone()),
        (1, 30, null.clone()),
        (2, 9, null.clone()),
        (2, 10, null.clone()),
        (3, 0, null.clone()),
        (3, 2, null.clone()),
        (3, 8, null.clone()),
        (3, 10, null.clone()),
        (3, 4, pair(4, 6)),
        (3, 6, pair(4, 6)),
        (4, 9, pair(9, 11)),
        (4, 11, pair(9, 11)),
        (4, 7, null.clone()),
        (4, 13, null.clone()),
        (5, 4, null.clone()),
        (5, 6, null.clone()),
        (5, 7, pair(7, 15)),
        (5, 15, pair(7, 15)),
        (5, 17, pair(17, 32)),
        (5, 32, pair(17, 32)),
        (5, 28, null.clone()),
        (5, 37, null.clone()),
        (5, 38, null.clone()),
    ];
    let mut s = Script::default();
    for (name, _) in files {
        s.push("doc.open", json!({ "path": name }));
    }
    let tree2 = s.push("syntax.tree", json!({"doc": 2}));
    let case_ids: Vec<usize> = cases
        .iter()
        .map(|(d, o, _)| s.push("syntax.match-bracket", json!({"doc": d, "offset": o})))
        .collect();
    let beyond: Vec<(usize, &str)> = [(1u64, 31usize), (2, 11), (3, 12), (4, 15), (5, 39)]
        .iter()
        .map(|(d, o)| {
            (
                s.push("syntax.match-bracket", json!({"doc": d, "offset": o})),
                files[*d as usize - 1].0,
            )
        })
        .collect();
    let no_doc = s.push("syntax.match-bracket", json!({"doc": 0, "offset": 0}));
    // Editing: doc 6 starts as `fn a() { (` and is turned into the other examples.
    let mut edits: Vec<(usize, usize, Value)> = Vec::new(); // (response id, offset, expected)
    let mut probe = |s: &mut Script, list: &[(usize, Value)]| {
        for (o, v) in list {
            let id = s.push("syntax.match-bracket", json!({"doc": 6, "offset": o}));
            edits.push((id, *o, v.clone()));
        }
    };
    let mut edit_ids = Vec::new();
    edit_ids.push(s.push("doc.open", json!({"path": "edit.rs.txt"})));
    probe(&mut s, &[(9, null.clone())]);
    edit_ids.push(s.push(
        "doc.insert",
        json!({"doc": 6, "offset": 10, "text": " ) ("}),
    ));
    probe(
        &mut s,
        &[
            (9, pair(9, 11)),
            (11, pair(9, 11)),
            (7, null.clone()),
            (13, null.clone()),
        ],
    );
    edit_ids.push(s.push(
        "doc.replace",
        json!({"doc": 6, "start": 0, "end": 14, "text": EX_HIGHLIGHT}),
    ));
    probe(
        &mut s,
        &[
            (4, pair(4, 5)),
            (7, pair(7, 24)),
            (24, pair(7, 24)),
            (0, null.clone()),
        ],
    );
    edit_ids.push(s.push("doc.undo", json!({"doc": 6})));
    probe(&mut s, &[(9, pair(9, 11)), (13, null.clone())]);
    edit_ids.push(s.push("doc.undo", json!({"doc": 6})));
    probe(&mut s, &[(9, null.clone()), (10, null.clone())]);
    let edit_text = s.push("doc.text", json!({"doc": 6}));
    let (r, _) = s.run(&fx, "bracket-examples");

    for i in 0..files.len() {
        ok(&r[i], files[i].0);
    }
    let t = ok(&r[tree2], "syntax.tree of `fn a() { (`");
    assert_eq!(t["has_error"], json!(true));
    for ((d, o, want), id) in cases.iter().zip(case_ids) {
        let got = &ok(&r[id], &format!("match-bracket doc {d} offset {o}"))["bracket"];
        assert_eq!(got, want, "{} offset {o}", files[*d as usize - 1].0);
    }
    for (id, name) in beyond {
        refused(
            &r[id],
            "rejected",
            "out_of_range",
            &format!("{name}: len_chars + 1"),
        );
    }
    refused(&r[no_doc], "rejected", "no_such_document", "doc 0");
    for id in edit_ids {
        ok(&r[id], "edit step");
    }
    for (id, o, want) in edits {
        let got = &ok(&r[id], &format!("edited doc offset {o}"))["bracket"];
        assert_eq!(*got, want, "edited doc, offset {o}");
    }
    assert_eq!(text_of(&r[edit_text], "doc.text"), "fn a() { (");
}

// ---------------------------------------------------------------------------------------------
// AC-8 (syntax.parse, size limit)
// ---------------------------------------------------------------------------------------------

/// AC-8: `//` + 1,048,574 `é` (1,048,576 chars, 2,097,150 bytes) is accepted by all three
/// commands; after inserting one char every `syntax.*` call is `rejected`/`too_large` (before
/// `out_of_range` and `inverted_range`); after `doc.undo` it is accepted again with `mode:
/// "unchanged"`; `commands --schema` publishes `too_large` under `rejected`.
/// Oracle: exact value + the naive model `len_chars > 1048576`.
#[test]
fn oversized_documents_are_refused() {
    let fx = Fixture::new("size-limit");
    let mut text = String::from("//");
    text.push_str(&"é".repeat(LIMIT - 2));
    assert_eq!(text.chars().count(), LIMIT);
    assert_eq!(text.len(), 2_097_150);
    fx.write("big.rs.txt", text.as_bytes());
    drop(text);
    let mut s = Script::default();
    let open = s.push("doc.open", json!({"path": "big.rs.txt"}));
    let tree_at = s.push("syntax.tree", json!({"doc": 1}));
    let bracket_at = s.push("syntax.match-bracket", json!({"doc": 1, "offset": 0}));
    let hl_at = s.push("syntax.highlight", json!({"doc": 1}));
    let ins = s.push("doc.insert", json!({"doc": 1, "offset": 0, "text": "x"}));
    let too: Vec<(usize, &str)> = vec![
        (s.push("syntax.tree", json!({"doc": 1})), "syntax.tree"),
        (
            s.push("syntax.highlight", json!({"doc": 1})),
            "syntax.highlight",
        ),
        (
            s.push("syntax.match-bracket", json!({"doc": 1, "offset": 0})),
            "syntax.match-bracket 0",
        ),
        (
            s.push(
                "syntax.match-bracket",
                json!({"doc": 1, "offset": 2_000_000}),
            ),
            "syntax.match-bracket 2000000",
        ),
        (
            s.push("syntax.highlight", json!({"doc": 1, "start": 2_000_000})),
            "syntax.highlight start 2000000",
        ),
        (
            s.push("syntax.highlight", json!({"doc": 1, "start": 5, "end": 2})),
            "syntax.highlight 5..2",
        ),
    ];
    let other_doc = s.push("syntax.tree", json!({"doc": 2}));
    let info = s.push("doc.info", json!({"doc": 1}));
    let undo = s.push("doc.undo", json!({"doc": 1}));
    let tree_back = s.push("syntax.tree", json!({"doc": 1}));
    let (r, _) = s.run(&fx, "size-limit");

    assert_eq!(ok(&r[open], "doc.open")["len_chars"], json!(LIMIT));
    let t = ok(&r[tree_at], "syntax.tree at exactly the limit");
    assert_eq!(t["has_error"], json!(false));
    assert_eq!(t["mode"], json!("full"));
    assert_eq!(
        ok(&r[bracket_at], "syntax.match-bracket at the limit")["bracket"],
        Value::Null
    );
    let sp = spans(ok(&r[hl_at], "syntax.highlight at the limit"), "limit");
    assert_eq!(sp, expect_spans(&[(0, LIMIT, "comment")]));
    assert_eq!(ok(&r[ins], "doc.insert")["len_chars"], json!(LIMIT + 1));
    for (id, what) in too {
        refused(
            &r[id],
            "rejected",
            "too_large",
            &format!("{what} at {} chars", LIMIT + 1),
        );
    }
    refused(&r[other_doc], "rejected", "no_such_document", "doc 2");
    assert_eq!(ok(&r[info], "doc.info")["len_chars"], json!(LIMIT + 1));
    let u = ok(&r[undo], "doc.undo");
    assert_eq!(u["undone"], json!(true));
    assert_eq!(u["len_chars"], json!(LIMIT));
    let t = ok(&r[tree_back], "syntax.tree back at the limit");
    assert_eq!(t["has_error"], json!(false));
    assert_eq!(
        t["mode"],
        json!("unchanged"),
        "the refused calls must leave the cache entry of the first call"
    );

    let out = kiln(&["commands", "--schema"]);
    assert_eq!(out.status.code(), Some(0), "commands --schema failed");
    let dump: Value = serde_json::from_slice(&out.stdout).unwrap();
    let rejected = dump["error_kinds"]["rejected"].as_array().unwrap();
    assert!(
        rejected.iter().any(|e| e["kind"] == json!("too_large")),
        "error_kinds.rejected has no `too_large` entry"
    );
}

// ---------------------------------------------------------------------------------------------
// AC-9 (read-only)
// ---------------------------------------------------------------------------------------------

/// AC-9: `commands --schema` gives `mutating: false` for the three commands; in one session the
/// observable state (`doc.info`, `cursor.get`, `doc.text`, `doc.list`) is equal before and after
/// valid and refused calls of all three; and they do not end a typing group (`"a"`, the three
/// commands, `"b"` at the next offset, one `doc.undo` restores the text from before `"a"`).
/// Oracle: exact value (T-007 (b)'s observable state; `mode` is not compared).
#[test]
fn syntax_commands_are_read_only() {
    let out = kiln(&["commands", "--schema"]);
    assert_eq!(out.status.code(), Some(0), "commands --schema failed");
    let dump: Value = serde_json::from_slice(&out.stdout).unwrap();
    let commands = dump["commands"].as_array().unwrap();
    for name in ["syntax.tree", "syntax.highlight", "syntax.match-bracket"] {
        let entry = commands
            .iter()
            .find(|c| c["name"] == json!(name))
            .unwrap_or_else(|| panic!("`{name}` is not in `commands --schema`"));
        assert_eq!(entry["mutating"], json!(false), "{name}");
    }

    let path = corpus_set("valid")
        .into_iter()
        .find(|p| !fs::read(corpus_dir().join(p)).unwrap().contains(&b'\r'))
        .expect("a valid corpus file without CR");
    let fx = Fixture::new("read-only");
    copy_corpus(&fx, std::slice::from_ref(&path));
    let file_len = fs::read_to_string(corpus_dir().join(&path))
        .unwrap()
        .chars()
        .count();
    assert!(file_len > 20);
    let len = file_len + 2; // after the kept "zz" below
    let mut s = Script::default();
    let setup = [
        s.push("doc.open", json!({ "path": path })),
        s.push("doc.insert", json!({"doc": 1, "offset": 0, "text": "zz"})),
        s.push("doc.insert", json!({"doc": 1, "offset": 0, "text": "yy"})),
        s.push("doc.undo", json!({"doc": 1})),
        s.push("cursor.set", json!({"doc": 1, "anchor": 5, "head": 12})),
    ];
    let snapshot = |s: &mut Script| -> Vec<usize> {
        vec![
            s.push("doc.info", json!({"doc": 1})),
            s.push("cursor.get", json!({"doc": 1})),
            s.push("doc.text", json!({"doc": 1})),
            s.push("doc.list", json!({})),
        ]
    };
    let before = snapshot(&mut s);
    let valid_calls = [
        s.push("syntax.tree", json!({"doc": 1})),
        s.push("syntax.highlight", json!({"doc": 1})),
        s.push("syntax.highlight", json!({"doc": 1, "start": 1, "end": 7})),
        s.push("syntax.match-bracket", json!({"doc": 1, "offset": 3})),
    ];
    let refusals = [
        (s.push("syntax.tree", json!({"doc": 0})), "no_such_document"),
        (
            s.push("syntax.highlight", json!({"doc": 1, "start": 7, "end": 1})),
            "inverted_range",
        ),
        (
            s.push("syntax.match-bracket", json!({"doc": 1, "offset": len + 1})),
            "out_of_range",
        ),
    ];
    let after = snapshot(&mut s);
    // Typing group: "a" at 3, the three commands, "b" at 4, one undo.
    let p = 3;
    let group = [
        s.push("doc.insert", json!({"doc": 1, "offset": p, "text": "a"})),
        s.push("syntax.tree", json!({"doc": 1})),
        s.push("syntax.highlight", json!({"doc": 1})),
        s.push("syntax.match-bracket", json!({"doc": 1, "offset": p})),
        s.push(
            "doc.insert",
            json!({"doc": 1, "offset": p + 1, "text": "b"}),
        ),
        s.push("doc.undo", json!({"doc": 1})),
    ];
    let restored = s.push("doc.text", json!({"doc": 1}));
    let (r, _) = s.run(&fx, "read-only");

    for id in setup {
        ok(&r[id], "setup");
    }
    let snap = |ids: &[usize]| -> Vec<Value> {
        ids.iter()
            .map(|&id| ok(&r[id], "snapshot").clone())
            .collect()
    };
    let b = snap(&before);
    assert_eq!(b[0]["len_chars"], json!(len));
    assert_eq!(b[0]["can_undo"], json!(true));
    assert_eq!(b[0]["can_redo"], json!(true));
    assert_eq!(b[0]["dirty"], json!(true));
    assert_eq!(
        (b[1]["anchor"].clone(), b[1]["head"].clone()),
        (json!(5), json!(12))
    );
    for id in valid_calls {
        ok(&r[id], "syntax.* call");
    }
    for (id, kind) in refusals {
        refused(&r[id], "rejected", kind, "refused syntax.* call");
    }
    assert_eq!(
        snap(&after),
        b,
        "observable state changed by syntax.* calls"
    );
    for id in group {
        ok(&r[id], "typing-group step");
    }
    assert_eq!(
        text_of(&r[restored], "doc.text after undo"),
        b[2]["text"].as_str().unwrap(),
        "one undo did not remove both typed chars: a syntax.* call ended the typing group"
    );
}

// ---------------------------------------------------------------------------------------------
// AC-10 (deep nesting)
// ---------------------------------------------------------------------------------------------

/// AC-10: `fn f() { ` + 500,000 `(` + `1` + 500,000 `)` + ` }` (1,000,012 chars): `syntax.tree`
/// has no error, `syntax.highlight` covers `0..1000012` (exact spans from the rules),
/// `match-bracket` pairs offsets 9/1000009 and 4/5, and `kiln-cli run` exits normally (no stack
/// overflow). No time is asserted. Oracle: exact value.
#[test]
fn deep_nesting_is_answered() {
    const DEPTH: usize = 500_000;
    let fx = Fixture::new("deep");
    let mut text = String::from("fn f() { ");
    text.push_str(&"(".repeat(DEPTH));
    text.push('1');
    text.push_str(&")".repeat(DEPTH));
    text.push_str(" }");
    let len = text.chars().count();
    assert_eq!(len, 1_000_012);
    fx.write("deep.rs.txt", text.as_bytes());
    drop(text);
    let mut s = Script::default();
    let open = s.push("doc.open", json!({"path": "deep.rs.txt"}));
    let tree = s.push("syntax.tree", json!({"doc": 1}));
    let hl = s.push("syntax.highlight", json!({"doc": 1}));
    let m9 = s.push("syntax.match-bracket", json!({"doc": 1, "offset": 9}));
    let m_close = s.push(
        "syntax.match-bracket",
        json!({"doc": 1, "offset": 1_000_009}),
    );
    let m4 = s.push("syntax.match-bracket", json!({"doc": 1, "offset": 4}));
    let (r, status) = s.run(&fx, "deep");
    ok(&r[open], "doc.open");
    let t = ok(&r[tree], "syntax.tree of the deep text");
    assert_eq!(t["has_error"], json!(false));
    assert_eq!(t["errors"], json!([]));
    let sp = spans(ok(&r[hl], "syntax.highlight of the deep text"), "deep");
    assert_partition(&sp, 0, len, "deep");
    assert_eq!(
        sp,
        expect_spans(&[
            (0, 2, "keyword"),
            (2, 3, "plain"),
            (3, 4, "function"),
            (4, 9 + DEPTH, "plain"),
            (9 + DEPTH, 10 + DEPTH, "number"),
            (10 + DEPTH, len, "plain"),
        ])
    );
    let pair = |o: usize, c: usize| json!({"open": o, "close": c});
    assert_eq!(ok(&r[m9], "offset 9")["bracket"], pair(9, 1_000_009));
    assert_eq!(
        ok(&r[m_close], "offset 1000009")["bracket"],
        pair(9, 1_000_009)
    );
    assert_eq!(ok(&r[m4], "offset 4")["bracket"], pair(4, 5));
    assert_eq!(status.code(), Some(0), "kiln-cli run did not exit normally");
}
