//! T-023 acceptance tests (AC-1 … AC-8) through the built `kiln-cli` binary: `kiln-cli run`
//! scripts and `kiln-cli control` sessions on fixtures in fresh temporary roots, and
//! `kiln-cli commands --schema`. Written by the implementer (odd-numbered medium task, the
//! experiment arm of METHODOLOGY §6) from the task specification alone
//! (`tasks/T-023-outline-and-navigation.md`), before any implementation code.
//!
//! Oracles: `syn` on the pinned corpus (AC-1); the exact values of Pinned (g) and of AC-6,
//! AC-7, AC-8; a naive clamp model written here (AC-3); consistency with `nav.outline` (AC-2,
//! AC-4); a naive model of the project search built from other public commands (AC-5). No
//! expected value is derived by running the implementation.
//!
//! On base no `nav.*` command exists: every test fails at its first `nav.*` response, which is
//! `unknown_command` where an `ok` response is asserted (or, in
//! `nav_commands_published_schema`, at the missing schema entry).
//!
//! No test asserts a message text, a time, the list of commands or `error_kinds`; refusals are
//! checked by `code` and `kind` only. Every spawned process has [`HANG`] as a hang detector
//! (D-028 (8)): it is not an assertion about speed. Symlinks and non-UTF-8 files are made with
//! the Unix API (Pinned (d)), without a `cfg` attribute (the gate's no-skip rule).
#![cfg(test)]

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Output, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use syn::spanned::Spanned;

/// Hang detector for every spawned `kiln-cli` (whole process for `run`/`commands`, each
/// response for `control`). Deliberately generous; reaching it means the process hung.
const HANG: Duration = Duration::from_secs(900);

/// The size limit (`syntax::MAX_CHARS`, T-020; D-108 Q-1): 2^20 chars.
const LIMIT: usize = 1_048_576;

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
            .join("acceptance_t023_cli")
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
    let status = wait_or_kill(&mut child, &format!("kiln-cli {:?}", &args[..1]));
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
            .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("response {l:.200}: {e}")))
            .collect();
        assert_eq!(
            responses.len(),
            self.lines.len(),
            "kiln-cli run {name}: one response per request expected (status {:?})",
            out.status
        );
        for (i, r) in responses.iter().enumerate() {
            assert_eq!(r["id"], json!(i), "response {i} out of order");
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
            serde_json::from_str(&resp).unwrap_or_else(|e| panic!("response {resp:.200}: {e}"));
        assert_eq!(v["id"], json!(id), "response out of order");
        v
    }

    /// The result of an `ok` response to `command`; otherwise fail.
    fn ok(&mut self, command: &str, params: Value) -> Value {
        let what = format!("{command} {params}");
        let r = self.call(command, params);
        ok(&r, &what).clone()
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

/// The result of an `ok` response; otherwise fail showing (the start of) the response.
fn ok<'a>(resp: &'a Value, what: &str) -> &'a Value {
    assert_eq!(
        resp["ok"],
        json!(true),
        "{what:.300}: expected an ok response, got {:.400}",
        resp.to_string()
    );
    &resp["result"]
}

/// Assert a refusal by `code` and `kind` only (no message is asserted).
fn refused(resp: &Value, code: &str, kind: &str, what: &str) {
    assert_eq!(
        resp["ok"],
        json!(false),
        "{what:.300}: expected a refusal, got {:.400}",
        resp.to_string()
    );
    assert_eq!(
        (
            resp["error"]["code"].as_str(),
            resp["error"]["kind"].as_str()
        ),
        (Some(code), Some(kind)),
        "{what:.300}: wrong code/kind in {:.400}",
        resp.to_string()
    );
}

/// `invalid_params` (protocol kind `schema`).
fn invalid_params(resp: &Value, what: &str) {
    refused(resp, "invalid_params", "schema", what);
}

fn u(v: &Value) -> usize {
    v.as_u64()
        .unwrap_or_else(|| panic!("not an unsigned integer: {v}")) as usize
}

// ---------------------------------------------------------------------------------------------
// Corpus (T-016 Pinned (b)): MANIFEST.toml read by its `path = "…"` and `set = "…"` lines
// ---------------------------------------------------------------------------------------------

fn corpus_dir() -> PathBuf {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/kiln-syntax/tests/corpus/"
    ))
    .to_path_buf()
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
    assert!(
        out.iter().filter(|(_, s)| s == "valid").count() >= 30,
        "fewer valid corpus files than expected"
    );
    assert!(out.iter().any(|(_, s)| s != "valid"), "only `valid` files");
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

fn corpus_text(path: &str) -> String {
    fs::read_to_string(corpus_dir().join(path)).unwrap()
}

/// The stem of a corpus path: its file name without `.rs.txt`.
fn stem(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap();
    name.strip_suffix(".rs.txt")
        .unwrap_or_else(|| panic!("corpus file {path} is not `.rs.txt`"))
        .to_owned()
}

// ---------------------------------------------------------------------------------------------
// Naive models
// ---------------------------------------------------------------------------------------------

/// Lines of `text` by `doc-positions.md`'s rule (split at LF, CRLF and lone CR): each line's
/// start (char offset) and its content length in chars, without the break.
fn lines(text: &str) -> Vec<(usize, usize)> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\n' => {
                out.push((start, i - start));
                i += 1;
                start = i;
            }
            '\r' => {
                out.push((start, i - start));
                i += if chars.get(i + 1) == Some(&'\n') {
                    2
                } else {
                    1
                };
                start = i;
            }
            _ => i += 1,
        }
    }
    out.push((start, chars.len() - start));
    out
}

/// The (line, col) of char offset `at` by the same rule (`at` not inside a CRLF).
fn line_col(lines: &[(usize, usize)], at: usize) -> (usize, usize) {
    let line = lines
        .iter()
        .rposition(|&(s, _)| s <= at)
        .expect("line of an offset");
    (line, at - lines[line].0)
}

/// The `CursorResult` of a caret or selection, by the model.
fn cursor(doc: u64, anchor: usize, head: usize, lines: &[(usize, usize)]) -> Value {
    let (line, col) = line_col(lines, head);
    json!({"doc": doc, "anchor": anchor, "head": head, "start": anchor.min(head),
        "end": anchor.max(head), "line": line, "col": col})
}

/// Deterministic PRNG (SplitMix64) for the fixed-seed cases.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// An outline item as the tests compare it: every field of `Item`.
fn item(
    kind: &str,
    name: &str,
    start: usize,
    end: usize,
    ns: usize,
    ne: usize,
    depth: usize,
) -> Value {
    json!({"kind": kind, "name": name, "start": start, "end": end, "name_start": ns,
        "name_end": ne, "depth": depth})
}

fn items_of(result: &Value, what: &str) -> Vec<Value> {
    result["items"]
        .as_array()
        .unwrap_or_else(|| panic!("{what}: no `items` list"))
        .clone()
}

/// The chars `range` of `text`.
fn char_slice(text: &str, start: usize, end: usize) -> String {
    text.chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}

// ---------------------------------------------------------------------------------------------
// AC-1: differential with syn on the `valid` corpus
// ---------------------------------------------------------------------------------------------

/// Items found by walking a `syn::File` as AC-1 says, with char offsets of the ident (for
/// `impl`: of `self_ty`).
#[derive(Debug, PartialEq, Eq)]
enum SynItem {
    Named {
        kind: &'static str,
        name: String,
        name_start: usize,
        depth: usize,
    },
    Impl {
        ty_start: usize,
        depth: usize,
    },
}

/// Char offset of a proc-macro2 location (1-based line on `\n` lines, column in chars).
fn offset_of(nl_starts: &[usize], lc: proc_macro2::LineColumn) -> usize {
    nl_starts[lc.line - 1] + lc.column
}

/// Char offsets where each `\n`-separated line starts.
fn nl_line_starts(text: &str) -> Vec<usize> {
    let mut out = vec![0];
    for (i, c) in text.chars().enumerate() {
        if c == '\n' {
            out.push(i + 1);
        }
    }
    out
}

fn syn_walk(items: &[syn::Item], depth: usize, nl: &[usize], out: &mut Vec<SynItem>) {
    let named = |kind, ident: &syn::Ident, out: &mut Vec<SynItem>| {
        out.push(SynItem::Named {
            kind,
            name: ident.to_string(),
            name_start: offset_of(nl, ident.span().start()),
            depth,
        });
    };
    for it in items {
        match it {
            syn::Item::Fn(f) => named("fn", &f.sig.ident, out),
            syn::Item::Struct(s) => named("struct", &s.ident, out),
            syn::Item::Enum(e) => named("enum", &e.ident, out),
            syn::Item::Trait(t) => {
                named("trait", &t.ident, out);
                for ti in &t.items {
                    if let syn::TraitItem::Fn(f) = ti {
                        out.push(SynItem::Named {
                            kind: "fn",
                            name: f.sig.ident.to_string(),
                            name_start: offset_of(nl, f.sig.ident.span().start()),
                            depth: depth + 1,
                        });
                    }
                }
            }
            syn::Item::Mod(m) => {
                named("mod", &m.ident, out);
                if let Some((_, content)) = &m.content {
                    syn_walk(content, depth + 1, nl, out);
                }
            }
            syn::Item::Impl(i) => {
                out.push(SynItem::Impl {
                    ty_start: offset_of(nl, i.self_ty.span().start()),
                    depth,
                });
                for ii in &i.items {
                    if let syn::ImplItem::Fn(f) = ii {
                        out.push(SynItem::Named {
                            kind: "fn",
                            name: f.sig.ident.to_string(),
                            name_start: offset_of(nl, f.sig.ident.span().start()),
                            depth: depth + 1,
                        });
                    }
                }
            }
            _ => {}
        }
    }
}

/// The `nav.outline` items of one result in AC-1's comparison form.
fn kiln_walk(items: &[Value]) -> Vec<SynItem> {
    items
        .iter()
        .map(|it| {
            let kind = it["kind"].as_str().unwrap();
            let depth = u(&it["depth"]);
            if kind == "impl" {
                SynItem::Impl {
                    ty_start: u(&it["name_start"]),
                    depth,
                }
            } else {
                let kind = match kind {
                    "fn" => "fn",
                    "struct" => "struct",
                    "enum" => "enum",
                    "trait" => "trait",
                    "mod" => "mod",
                    other => panic!("unexpected kind {other:?}"),
                };
                SynItem::Named {
                    kind,
                    name: it["name"].as_str().unwrap().to_owned(),
                    name_start: u(&it["name_start"]),
                    depth,
                }
            }
        })
        .collect()
}

/// AC-1 (`syntax.outline`): on every `valid` corpus file the outline equals the syn walk
/// (non-`impl` items by kind, name, name start and depth; `impl` items by the start of their
/// type and depth), in order.
#[test]
fn outline_agrees_with_syn() {
    let files = corpus_set("valid");
    let fx = Fixture::new("syn");
    let mut s = Script::default();
    let mut calls = Vec::new();
    for (i, p) in files.iter().enumerate() {
        let rel = format!("{}.rs", stem(p));
        fx.write(&rel, corpus_text(p).as_bytes());
        let open = s.push("doc.open", json!({ "path": rel }));
        let outline = s.push("nav.outline", json!({"doc": i + 1}));
        calls.push((p.clone(), open, outline));
    }
    let (resp, _) = s.run(&fx, "syn");
    let mut total = 0;
    for (p, open, outline) in calls {
        ok(&resp[open], &format!("doc.open {p}"));
        let got = kiln_walk(&items_of(
            ok(&resp[outline], &format!("nav.outline {p}")),
            &p,
        ));
        let text = corpus_text(&p);
        assert!(
            !text.starts_with('\u{feff}') && !text.starts_with("#!"),
            "{p}: starts with a BOM or `#!` (syn would strip it)"
        );
        let file = syn::parse_file(&text).unwrap_or_else(|e| panic!("{p}: syn: {e}"));
        let mut want = Vec::new();
        syn_walk(&file.items, 0, &nl_line_starts(&text), &mut want);
        let split = |v: Vec<SynItem>| -> (Vec<SynItem>, Vec<SynItem>) {
            v.into_iter()
                .partition(|x| matches!(x, SynItem::Impl { .. }))
        };
        let (got_impl, got_named) = split(got);
        let (want_impl, want_named) = split(want);
        assert_eq!(got_named, want_named, "{p}: non-impl items differ from syn");
        assert_eq!(got_impl, want_impl, "{p}: impl items differ from syn");
        total += got_named.len() + got_impl.len();
    }
    assert!(total > 100, "too few items over the valid corpus: {total}");
}

// ---------------------------------------------------------------------------------------------
// AC-2: worked examples, the tree cache, name slices
// ---------------------------------------------------------------------------------------------

/// Pinned (g) examples 1 to 5: text and every item, exactly.
fn worked_examples() -> Vec<(&'static str, Vec<Value>)> {
    vec![
        (
            "impl<T> Display for Foo<T> { fn fmt(&self) {} }\nmod m;\ntrait T { fn x(&self); }",
            vec![
                item("impl", "Display for Foo<T>", 0, 47, 20, 26, 0),
                item("fn", "fmt", 29, 45, 32, 35, 1),
                item("mod", "m", 48, 54, 52, 53, 0),
                item("trait", "T", 55, 79, 61, 62, 0),
                item("fn", "x", 65, 77, 68, 69, 1),
            ],
        ),
        (
            "fn a() {}\nfn b( {\nfn c() {}\nstruct D;\n",
            vec![
                item("fn", "a", 0, 9, 3, 4, 0),
                item("fn", "c", 18, 27, 21, 22, 0),
                item("struct", "D", 28, 37, 35, 36, 0),
            ],
        ),
        (
            "/// doc\n#[derive(Debug)]\npub struct S;\n#[cfg(test)]\nmod tests { fn t() {} }",
            vec![
                item("struct", "S", 25, 38, 36, 37, 0),
                item("mod", "tests", 52, 75, 56, 61, 0),
                item("fn", "t", 64, 73, 67, 68, 1),
            ],
        ),
        (
            "extern \"C\" { fn c(); }\nm! { fn inner() {} }\nfn outer() { fn nested() {} struct N; }\nconst _: () = { struct Q; };",
            vec![item("fn", "outer", 44, 83, 47, 52, 0)],
        ),
        (
            "impl  <T>  Foo < T ,\n U > {}\nimpl !Send for X {}",
            vec![
                item("impl", "Foo < T , U >", 0, 28, 11, 25, 0),
                item("impl", "Send for X", 29, 48, 44, 45, 0),
            ],
        ),
        // D-124 m2 (ruling Q-2 confirmed): the unclosed `fn b` is `ERROR@21..49` under
        // `source_file`, and the `mod n` inside it is listed because `ERROR` is a container.
        // Expected items derived from the `syntax.tree` node dump and the outline rules.
        (
            "mod m { fn a() { } } fn b() { mod n { fn c() {} }",
            vec![
                item("mod", "m", 0, 20, 4, 5, 0),
                item("fn", "a", 8, 18, 11, 12, 1),
                item("mod", "n", 30, 49, 34, 35, 0),
                item("fn", "c", 38, 47, 41, 42, 1),
            ],
        ),
    ]
}

/// AC-2 (`syntax.outline`): examples 1 to 5 of Pinned (g) and the `ERROR` example of D-124 m2
/// exactly; `nav.*` leave the tree that
/// `syntax.*` keep alone (N7); on every corpus file the name range of every non-`impl` item
/// slices out its name.
#[test]
fn outline_worked_examples() {
    let fx = Fixture::new("examples");
    let examples = worked_examples();
    let mut s = Script::default();
    let mut calls = Vec::new();
    for (i, (text, _)) in examples.iter().enumerate() {
        let rel = format!("ex{}.rs", i + 1);
        fx.write(&rel, text.as_bytes());
        let open = s.push("doc.open", json!({ "path": rel }));
        let outline = s.push("nav.outline", json!({"doc": i + 1}));
        calls.push((open, outline));
    }
    // N7 on example 1 (document 1): the second `syntax.tree` finds the tree it left.
    let t1 = s.push("syntax.tree", json!({"doc": 1}));
    let o1 = s.push("nav.outline", json!({"doc": 1}));
    let t2 = s.push("syntax.tree", json!({"doc": 1}));
    // ... and a `nav.*` call after an edit does not bring the cached tree up to date.
    let ins = s.push("doc.insert", json!({"doc": 1, "offset": 79, "text": " "}));
    let o2 = s.push("nav.outline", json!({"doc": 1}));
    let g2 = s.push("nav.goto-symbol", json!({"doc": 1, "index": 1}));
    let w2 = s.push("nav.workspace-symbols", json!({"query": "fmt"}));
    let t3 = s.push("syntax.tree", json!({"doc": 1}));
    let (resp, _) = s.run(&fx, "examples");
    for (i, ((_, want), (open, outline))) in examples.iter().zip(calls).enumerate() {
        ok(&resp[open], "doc.open");
        let got = items_of(ok(&resp[outline], &format!("example {}", i + 1)), "example");
        assert_eq!(
            &got,
            want,
            "worked example {} (Pinned (g) 1 to 5; 6: D-124 m2)",
            i + 1
        );
    }
    assert_eq!(ok(&resp[t1], "syntax.tree 1")["mode"], json!("full"));
    assert_eq!(
        items_of(ok(&resp[o1], "nav.outline after syntax.tree"), "o1"),
        examples[0].1
    );
    assert_eq!(
        ok(&resp[t2], "syntax.tree 2")["mode"],
        json!("unchanged"),
        "nav.outline changed the syntax.* tree"
    );
    ok(&resp[ins], "doc.insert");
    assert_eq!(
        items_of(ok(&resp[o2], "nav.outline after an edit"), "o2"),
        examples[0].1,
        "an insert at the end moves no item"
    );
    assert_eq!(
        ok(&resp[g2], "nav.goto-symbol")["cursor"]["head"],
        json!(35)
    );
    assert_eq!(
        ok(&resp[w2], "nav.workspace-symbols")["items"][0]["name"],
        json!("fmt")
    );
    assert_eq!(
        ok(&resp[t3], "syntax.tree 3")["mode"],
        json!("incremental"),
        "a nav.* command updated the syntax.* tree"
    );

    // `name_start..name_end` slices out `name` for every non-impl item of every corpus file.
    let all: Vec<String> = corpus().into_iter().map(|(p, _)| p).collect();
    let fx = Fixture::new("slices");
    let mut s = Script::default();
    let mut calls = Vec::new();
    for (i, p) in all.iter().enumerate() {
        let rel = format!("f{i}.rs");
        fx.write(&rel, corpus_text(p).as_bytes());
        s.push("doc.open", json!({ "path": rel }));
        calls.push((p.clone(), s.push("nav.outline", json!({"doc": i + 1}))));
    }
    let (resp, _) = s.run(&fx, "slices");
    let mut checked = 0;
    for (p, outline) in calls {
        let text = corpus_text(&p);
        for it in items_of(ok(&resp[outline], &p), &p) {
            if it["kind"] == json!("impl") {
                continue;
            }
            let (ns, ne) = (u(&it["name_start"]), u(&it["name_end"]));
            let (st, en) = (u(&it["start"]), u(&it["end"]));
            assert!(st <= ns && ns < ne && ne <= en, "{p}: ranges of {it}");
            assert_eq!(
                json!(char_slice(&text, ns, ne)),
                it["name"],
                "{p}: name range of {it}"
            );
            checked += 1;
        }
    }
    assert!(checked > 100, "too few items checked: {checked}");
}

// ---------------------------------------------------------------------------------------------
// AC-6: deep nesting
// ---------------------------------------------------------------------------------------------

/// AC-6 (`syntax.outline`): 20,000 nested inline modules around one fn give 20,001 items, the
/// last a `fn f` at depth 20,000, and the process exits normally (no stack overflow).
#[test]
fn outline_survives_deep_nesting() {
    const N: usize = 20_000;
    let text = format!("{}fn f() {{}}{}", "mod m {".repeat(N), "}".repeat(N));
    let len = text.chars().count();
    assert_eq!(len, 160_009);
    let fx = Fixture::new("deep");
    fx.write("deep.rs", text.as_bytes());
    let mut s = Script::default();
    s.push("doc.open", json!({"path": "deep.rs"}));
    let outline = s.push("nav.outline", json!({"doc": 1}));
    let (resp, status) = s.run(&fx, "deep");
    let items = items_of(ok(&resp[outline], "nav.outline deep"), "deep");
    assert_eq!(status.code(), Some(0), "kiln-cli run did not exit normally");
    assert_eq!(items.len(), N + 1);
    for (i, it) in items[..N].iter().enumerate() {
        assert_eq!(
            it,
            &item("mod", "m", 7 * i, len - i, 7 * i + 4, 7 * i + 5, i),
            "item {i}"
        );
    }
    assert_eq!(
        items[N],
        item("fn", "f", 7 * N, 7 * N + 9, 7 * N + 3, 7 * N + 4, N)
    );
}

// ---------------------------------------------------------------------------------------------
// AC-7: size limit
// ---------------------------------------------------------------------------------------------

/// `//` then `é` repeated: one comment of exactly `chars` chars.
fn comment_of(chars: usize) -> String {
    let mut t = String::from("//");
    t.push_str(&"é".repeat(chars - 2));
    t
}

/// AC-7: a document of exactly 1,048,576 chars is accepted; one char more is `too_large` for
/// `nav.outline` and `nav.goto-symbol` but not `nav.goto-line`; in a project such a file is
/// skipped `too_large` and the search goes on.
#[test]
fn outline_size_limit() {
    let fx = Fixture::new("size");
    fx.write("edge.rs", comment_of(LIMIT).as_bytes());
    fx.write("big.rs", comment_of(LIMIT + 1).as_bytes());
    fx.write("s.rs", b"fn x() {}\n");
    let mut s = Script::default();
    let open = s.push("doc.open", json!({"path": "edge.rs"}));
    let at_limit = s.push("nav.outline", json!({"doc": 1}));
    let sym_at_limit = s.push("nav.goto-symbol", json!({"doc": 1, "index": 0}));
    let ws1 = s.push("nav.workspace-symbols", json!({"query": "x"}));
    let ins = s.push(
        "doc.insert",
        json!({"doc": 1, "offset": LIMIT, "text": "é"}),
    );
    let cur0 = s.push("cursor.set", json!({"doc": 1, "anchor": 3, "head": 7}));
    let over = s.push("nav.outline", json!({"doc": 1}));
    let sym_over = s.push("nav.goto-symbol", json!({"doc": 1, "index": 0}));
    let cur1 = s.push("cursor.get", json!({"doc": 1}));
    let line_over = s.push("nav.goto-line", json!({"doc": 1, "line": 0, "col": 5}));
    // The open, edited document's text is the one searched: now over the limit too.
    let ws2 = s.push("nav.workspace-symbols", json!({"query": "x"}));
    let (resp, _) = s.run(&fx, "size");
    assert_eq!(ok(&resp[open], "doc.open")["len_chars"], json!(LIMIT));
    assert_eq!(
        items_of(ok(&resp[at_limit], "nav.outline at the limit"), "edge"),
        Vec::<Value>::new()
    );
    refused(
        &resp[sym_at_limit],
        "rejected",
        "out_of_range",
        "goto-symbol at the limit (accepted, no items)",
    );
    let w = ok(&resp[ws1], "workspace-symbols");
    assert_eq!(
        w["items"],
        json!([{"path": "s.rs", "kind": "fn", "name": "x", "start": 0, "end": 9,
            "name_start": 3, "name_end": 4, "line": 0, "col": 3}])
    );
    assert_eq!(
        w["skipped"],
        json!([{"path": "big.rs", "reason": "too_large"}])
    );
    assert_eq!(w["truncated"], json!(false));
    ok(&resp[ins], "doc.insert");
    let before = ok(&resp[cur0], "cursor.set").clone();
    refused(
        &resp[over],
        "rejected",
        "too_large",
        "nav.outline over the limit",
    );
    refused(
        &resp[sym_over],
        "rejected",
        "too_large",
        "goto-symbol over the limit",
    );
    assert_eq!(
        ok(&resp[cur1], "cursor.get"),
        &before,
        "a refusal moved the selection"
    );
    assert_eq!(
        ok(&resp[line_over], "nav.goto-line over the limit"),
        &json!({"doc": 1, "anchor": 5, "head": 5, "start": 5, "end": 5, "line": 0, "col": 5})
    );
    let w = ok(&resp[ws2], "workspace-symbols with the edited document");
    assert_eq!(
        w["skipped"],
        json!([{"path": "big.rs", "reason": "too_large"},
            {"path": "edge.rs", "reason": "too_large"}])
    );
    assert_eq!(w["items"].as_array().map(Vec::len), Some(1));
}

// ---------------------------------------------------------------------------------------------
// AC-3: nav.goto-line
// ---------------------------------------------------------------------------------------------

/// AC-3 (`nav.goto-line`): ≥ 2,000 fixed-seed random texts and line/col values agree with the
/// naive clamp model in the result and in a following `cursor.get`; Pinned (g) example 6;
/// refusals change nothing.
#[test]
fn goto_line_clamps() {
    const TEXTS: usize = 2_000;
    let alphabet = ["a", "b", "é", "😀", "\r", "\n"];
    let mut rng = Rng(0x7023_0001);
    let fx = Fixture::new("goto-line");
    let mut s = Script::default();
    let mut checks = Vec::new();
    for i in 0..TEXTS {
        let len = rng.below(25) as usize;
        let text: String = (0..len)
            .map(|_| alphabet[rng.below(alphabet.len() as u64) as usize])
            .collect();
        let rel = format!("t{i}.txt");
        fx.write(&rel, text.as_bytes());
        let doc = (i + 1) as u64;
        s.push("doc.open", json!({ "path": rel }));
        let ls = lines(&text);
        let last = (ls.len() - 1) as u64;
        let pick = |rng: &mut Rng| -> u64 {
            match rng.below(7) {
                0 => 0,
                1 => last,
                2 => last + 1,
                3 => rng.below(last + 3),
                4 => 1_000,
                5 => u64::from(u32::MAX),
                _ => rng.below(30),
            }
        };
        for _ in 0..4 {
            let line = pick(&mut rng);
            let col = if rng.below(4) == 0 {
                None
            } else {
                Some(pick(&mut rng))
            };
            let params = match col {
                Some(c) => json!({"doc": doc, "line": line, "col": c}),
                None => json!({"doc": doc, "line": line}),
            };
            let l = (line.min(last)) as usize;
            let c = (col.unwrap_or(0) as usize).min(ls[l].1);
            let head = ls[l].0 + c;
            let want = cursor(doc, head, head, &ls);
            assert_eq!(want["line"], json!(l));
            assert_eq!(want["col"], json!(c));
            let g = s.push("nav.goto-line", params.clone());
            let get = s.push("cursor.get", json!({ "doc": doc }));
            checks.push((params, g, get, want));
        }
    }
    // Pinned (g) example 6.
    fx.write("ex6.txt", b"ab\r\ncdef\n");
    let ex6 = (TEXTS + 1) as u64;
    s.push("doc.open", json!({"path": "ex6.txt"}));
    for (params, head, line, col) in [
        (json!({"doc": ex6, "line": 1, "col": 9}), 8, 1, 4),
        (json!({"doc": ex6, "line": 8}), 9, 2, 0),
        (json!({"doc": ex6, "line": 0, "col": 1}), 1, 0, 1),
    ] {
        let want = json!({"doc": ex6, "anchor": head, "head": head, "start": head, "end": head,
            "line": line, "col": col});
        let g = s.push("nav.goto-line", params.clone());
        let get = s.push("cursor.get", json!({ "doc": ex6 }));
        checks.push((params, g, get, want));
    }
    // Refusals: they change nothing (the selection set before stays).
    let set = s.push("cursor.set", json!({"doc": ex6, "anchor": 1, "head": 6}));
    let bad = [
        json!({"doc": ex6, "line": -1}),
        json!({"doc": ex6, "line": 0, "col": -1}),
        json!({"doc": ex6, "line": 1.5}),
        json!({"doc": ex6, "line": 0, "col": 0.5}),
        json!({"doc": ex6, "line": "1"}),
        json!({"doc": ex6, "line": 0, "col": "1"}),
        json!({"doc": ex6, "line": null}),
        json!({"doc": ex6, "line": 0, "col": null}),
        json!({"doc": ex6, "line": 0, "column": 1}),
        json!({"doc": ex6}),
    ];
    let mut refusals = Vec::new();
    for p in bad {
        let r = s.push("nav.goto-line", p.clone());
        refusals.push((
            p,
            r,
            "invalid_params",
            s.push("cursor.get", json!({ "doc": ex6 })),
        ));
    }
    for p in [
        json!({"doc": 9_999, "line": 0}),
        json!({"doc": 0, "line": 0, "col": 0}),
    ] {
        let r = s.push("nav.goto-line", p.clone());
        refusals.push((
            p,
            r,
            "no_such_document",
            s.push("cursor.get", json!({ "doc": ex6 })),
        ));
    }
    let (resp, _) = s.run(&fx, "goto-line");
    assert!(checks.len() >= 4 * TEXTS);
    for (params, g, get, want) in &checks {
        assert_eq!(
            ok(&resp[*g], &format!("nav.goto-line {params}")),
            want,
            "{params}"
        );
        assert_eq!(
            ok(&resp[*get], "cursor.get"),
            want,
            "cursor.get after {params}"
        );
    }
    let selection = ok(&resp[set], "cursor.set").clone();
    for (p, r, kind, get) in refusals {
        if kind == "invalid_params" {
            invalid_params(&resp[r], &format!("nav.goto-line {p}"));
        } else {
            refused(&resp[r], "rejected", kind, &format!("nav.goto-line {p}"));
        }
        assert_eq!(ok(&resp[get], "cursor.get"), &selection, "after {p}");
    }
}

/// One typing-group scenario on `text`: insert `x` at `at`, then `between` (if any), then
/// insert `y` at `at + 1`, then undo. Returns the text after the undo and the response to
/// `between`; every other step must succeed.
fn typing_group(
    fx: &Fixture,
    name: &str,
    text: &str,
    at: usize,
    between: Option<(&str, Value)>,
) -> (String, Option<Value>) {
    fx.write(&format!("{name}.rs"), text.as_bytes());
    let mut s = Script::default();
    s.push("doc.open", json!({"path": format!("{name}.rs")}));
    s.push("doc.insert", json!({"doc": 1, "offset": at, "text": "x"}));
    let mid = between.map(|(c, p)| s.push(c, p));
    s.push(
        "doc.insert",
        json!({"doc": 1, "offset": at + 1, "text": "y"}),
    );
    s.push("doc.undo", json!({"doc": 1}));
    let t = s.push("doc.text", json!({"doc": 1}));
    let (resp, _) = s.run(fx, name);
    for (i, r) in resp.iter().enumerate() {
        if Some(i) != mid {
            ok(r, &format!("{name} step {i}"));
        }
    }
    let after = ok(&resp[t], "doc.text")["text"]
        .as_str()
        .unwrap()
        .to_owned();
    (after, mid.map(|m| resp[m].clone()))
}

/// AC-3 (`nav.goto-line`), typing group: a successful `nav.goto-line` ends the typing group
/// (one undo removes only the char typed after it), as `cursor.set` does; without it, the two
/// chars form one group. `dirty` and `can_undo` are unchanged by `nav.goto-line` alone.
#[test]
fn goto_line_ends_typing_group() {
    let fx = Fixture::new("goto-line-group");
    let text = "abc\ndef\n";
    // Control: without a move, "x" and "y" join one typing group and one undo removes both.
    assert_eq!(typing_group(&fx, "plain", text, 0, None).0, text);
    // With `cursor.set` (the documented reference behaviour) only "y" is undone.
    let (after, mid) = typing_group(
        &fx,
        "set",
        text,
        0,
        Some(("cursor.set", json!({"doc": 1, "anchor": 1}))),
    );
    ok(&mid.unwrap(), "cursor.set");
    assert_eq!(after, "xabc\ndef\n");
    // With `nav.goto-line` to the caret after "x" (line 0, col 1): only "y" is undone.
    let (after, mid) = typing_group(
        &fx,
        "goto",
        text,
        0,
        Some(("nav.goto-line", json!({"doc": 1, "line": 0, "col": 1}))),
    );
    assert_eq!(ok(&mid.unwrap(), "nav.goto-line")["head"], json!(1));
    assert_eq!(
        after, "xabc\ndef\n",
        "nav.goto-line did not end the typing group"
    );
    // A refused `nav.goto-line` changes nothing: the group stays open.
    for (bad, code, kind) in [
        (json!({"doc": 7, "line": 0}), "rejected", "no_such_document"),
        (json!({"doc": 1, "line": -1}), "invalid_params", "schema"),
    ] {
        let (after, mid) = typing_group(&fx, "refused", text, 0, Some(("nav.goto-line", bad)));
        refused(&mid.unwrap(), code, kind, "refused nav.goto-line");
        assert_eq!(
            after, text,
            "a refused nav.goto-line ended the typing group"
        );
    }

    // The step itself succeeds and lands on the caret the next insert uses; `dirty` and
    // `can_undo` are unchanged by it, before and after an edit.
    fx.write("info.rs", text.as_bytes());
    let mut s = Script::default();
    s.push("doc.open", json!({"path": "info.rs"}));
    let i0 = s.push("doc.info", json!({"doc": 1}));
    let g0 = s.push("nav.goto-line", json!({"doc": 1, "line": 1, "col": 2}));
    let i1 = s.push("doc.info", json!({"doc": 1}));
    s.push("doc.insert", json!({"doc": 1, "offset": 0, "text": "x"}));
    let i2 = s.push("doc.info", json!({"doc": 1}));
    let g1 = s.push("nav.goto-line", json!({"doc": 1, "line": 0, "col": 1}));
    let i3 = s.push("doc.info", json!({"doc": 1}));
    let t = s.push("doc.text", json!({"doc": 1}));
    let (resp, _) = s.run(&fx, "info");
    assert_eq!(ok(&resp[g0], "goto 0")["head"], json!(6));
    assert_eq!(ok(&resp[g1], "goto 1")["head"], json!(1));
    let flags = |r: &Value| {
        (
            r["dirty"].clone(),
            r["can_undo"].clone(),
            r["can_redo"].clone(),
        )
    };
    assert_eq!(
        flags(ok(&resp[i0], "info 0")),
        (json!(false), json!(false), json!(false))
    );
    assert_eq!(
        flags(ok(&resp[i1], "info 1")),
        flags(ok(&resp[i0], "info 0"))
    );
    assert_eq!(
        flags(ok(&resp[i2], "info 2")),
        (json!(true), json!(true), json!(false))
    );
    assert_eq!(
        flags(ok(&resp[i3], "info 3")),
        flags(ok(&resp[i2], "info 2"))
    );
    assert_eq!(ok(&resp[t], "doc.text")["text"], json!("xabc\ndef\n"));
}

// ---------------------------------------------------------------------------------------------
// AC-4: nav.goto-symbol
// ---------------------------------------------------------------------------------------------

/// AC-4 (`nav.goto-symbol`): on every corpus file, every index selects its item's name (result
/// and `cursor.get`); the item count is `out_of_range` and moves nothing; Pinned (g) example
/// 1; the typing-group check of AC-3 with `nav.goto-symbol`.
#[test]
fn goto_symbol_selects_name() {
    let fx = Fixture::new("goto-symbol");
    let all: Vec<String> = corpus().into_iter().map(|(p, _)| p).collect();
    for (i, p) in all.iter().enumerate() {
        fx.write(&format!("f{i}.rs"), corpus_text(p).as_bytes());
    }
    let ex1 = worked_examples()[0].0;
    fx.write("ex1.rs", ex1.as_bytes());
    let mut c = Control::start(&fx.root());
    let mut selected = 0;
    for (i, p) in all.iter().enumerate() {
        let doc = c.ok("doc.open", json!({"path": format!("f{i}.rs")}))["doc"]
            .as_u64()
            .unwrap();
        let text = c.ok("doc.text", json!({ "doc": doc }))["text"]
            .as_str()
            .unwrap()
            .to_owned();
        let ls = lines(&text);
        let items = items_of(&c.ok("nav.outline", json!({ "doc": doc })), p);
        for (k, it) in items.iter().enumerate() {
            let (ns, ne) = (u(&it["name_start"]), u(&it["name_end"]));
            let want = cursor(doc, ns, ne, &ls);
            let r = c.ok("nav.goto-symbol", json!({"doc": doc, "index": k}));
            assert_eq!(&r["item"], it, "{p}: item {k}");
            assert_eq!(r["cursor"], want, "{p}: cursor of item {k}");
            assert_eq!(
                c.ok("cursor.get", json!({ "doc": doc })),
                want,
                "{p}: cursor.get {k}"
            );
            selected += 1;
        }
        let before = c.ok("cursor.get", json!({ "doc": doc }));
        for index in [
            items.len() as u64,
            items.len() as u64 + 1,
            u64::from(u32::MAX),
        ] {
            let r = c.call("nav.goto-symbol", json!({"doc": doc, "index": index}));
            refused(
                &r,
                "rejected",
                "out_of_range",
                &format!("{p}: index {index}"),
            );
            assert_eq!(
                c.ok("cursor.get", json!({ "doc": doc })),
                before,
                "{p}: after refusal"
            );
        }
    }
    assert!(selected > 500, "too few items selected: {selected}");

    // Pinned (g) example 1, and the refusals of the params schema and the document.
    let doc = c.ok("doc.open", json!({"path": "ex1.rs"}))["doc"]
        .as_u64()
        .unwrap();
    let r = c.ok("nav.goto-symbol", json!({"doc": doc, "index": 1}));
    assert_eq!(r["item"], item("fn", "fmt", 29, 45, 32, 35, 1));
    assert_eq!(
        r["cursor"],
        json!({"doc": doc, "anchor": 32, "head": 35, "start": 32, "end": 35, "line": 0, "col": 35})
    );
    let r = c.call("nav.goto-symbol", json!({"doc": doc, "index": 5}));
    refused(&r, "rejected", "out_of_range", "example 1 index 5");
    for bad in [
        json!({"doc": doc, "index": -1}),
        json!({"doc": doc, "index": 0.5}),
        json!({"doc": doc, "index": "0"}),
        json!({"doc": doc, "index": null}),
        json!({ "doc": doc }),
        json!({"doc": doc, "index": 0, "extra": 1}),
    ] {
        let r = c.call("nav.goto-symbol", bad.clone());
        invalid_params(&r, &format!("nav.goto-symbol {bad}"));
    }
    let r = c.call("nav.goto-symbol", json!({"doc": 9_999, "index": 0}));
    refused(&r, "rejected", "no_such_document", "unknown doc");
    assert_eq!(
        c.ok("cursor.get", json!({ "doc": doc }))["head"],
        json!(35),
        "a refusal moved the selection"
    );
    assert_eq!(c.finish().code(), Some(0));

    // Typing group: "fn f() {}" + "x" at 4 gives `fx`; goto-symbol 0 selects 3..5 and "y" is
    // typed at 5, right after "x".
    let text = "fn f() {}\nfn g() {}\n";
    assert_eq!(typing_group(&fx, "sym-plain", text, 4, None).0, text);
    let (after, mid) = typing_group(
        &fx,
        "sym-goto",
        text,
        4,
        Some(("nav.goto-symbol", json!({"doc": 1, "index": 0}))),
    );
    assert_eq!(
        ok(&mid.unwrap(), "nav.goto-symbol")["cursor"]["head"],
        json!(5)
    );
    assert_eq!(
        after, "fn fx() {}\nfn g() {}\n",
        "nav.goto-symbol did not end the typing group"
    );
    for (bad, code, kind) in [
        (json!({"doc": 1, "index": 2}), "rejected", "out_of_range"),
        (
            json!({"doc": 7, "index": 0}),
            "rejected",
            "no_such_document",
        ),
    ] {
        let (after, mid) =
            typing_group(&fx, "sym-refused", text, 4, Some(("nav.goto-symbol", bad)));
        refused(&mid.unwrap(), code, kind, "refused nav.goto-symbol");
        assert_eq!(
            after, text,
            "a refused nav.goto-symbol ended the typing group"
        );
    }
    fx.write("syminfo.rs", text.as_bytes());
    let mut s = Script::default();
    s.push("doc.open", json!({"path": "syminfo.rs"}));
    let i0 = s.push("doc.info", json!({"doc": 1}));
    let g0 = s.push("nav.goto-symbol", json!({"doc": 1, "index": 1}));
    let i1 = s.push("doc.info", json!({"doc": 1}));
    s.push("doc.insert", json!({"doc": 1, "offset": 4, "text": "x"}));
    let i2 = s.push("doc.info", json!({"doc": 1}));
    let g1 = s.push("nav.goto-symbol", json!({"doc": 1, "index": 0}));
    let i3 = s.push("doc.info", json!({"doc": 1}));
    let t = s.push("doc.text", json!({"doc": 1}));
    let (resp, _) = s.run(&fx, "syminfo");
    assert_eq!(
        ok(&resp[g0], "goto-symbol 0")["cursor"]["anchor"],
        json!(13)
    );
    assert_eq!(ok(&resp[g1], "goto-symbol 1")["item"]["name"], json!("fx"));
    let flags = |r: &Value| {
        (
            r["dirty"].clone(),
            r["can_undo"].clone(),
            r["can_redo"].clone(),
        )
    };
    assert_eq!(
        flags(ok(&resp[i0], "info 0")),
        (json!(false), json!(false), json!(false))
    );
    assert_eq!(
        flags(ok(&resp[i1], "info 1")),
        flags(ok(&resp[i0], "info 0"))
    );
    assert_eq!(
        flags(ok(&resp[i2], "info 2")),
        (json!(true), json!(true), json!(false))
    );
    assert_eq!(
        flags(ok(&resp[i3], "info 3")),
        flags(ok(&resp[i2], "info 2"))
    );
    assert_eq!(
        ok(&resp[t], "doc.text")["text"],
        json!("fn fx() {}\nfn g() {}\n")
    );
}

// ---------------------------------------------------------------------------------------------
// AC-5: nav.workspace-symbols against a naive model
// ---------------------------------------------------------------------------------------------

/// One symbol of the model: the file's place in the walk, and the `Symbol` object.
struct ModelSymbol {
    name: String,
    symbol: Value,
}

/// The model of a project: symbols in walk order then outline order, and the skipped files.
struct Model {
    symbols: Vec<ModelSymbol>,
    skipped: Vec<Value>,
}

/// Build the model from public commands in a fresh session: `project.tree {}`, then per
/// `file` entry ending in `.rs`, `doc.open` (a refusal → `skipped` with its kind), `nav.outline`,
/// `doc.position` of each name start, `doc.close`.
fn build_model(root: &Path) -> Model {
    let mut c = Control::start(root);
    let tree = c.ok("project.tree", json!({}));
    assert_eq!(tree["truncated"], json!(false));
    let mut symbols = Vec::new();
    let mut skipped = Vec::new();
    for e in tree["entries"].as_array().unwrap() {
        let path = e["path"].as_str().unwrap();
        if e["kind"] != json!("file") || !path.ends_with(".rs") {
            continue;
        }
        let open = c.call("doc.open", json!({ "path": path }));
        if open["ok"] != json!(true) {
            skipped.push(json!({"path": path, "reason": open["error"]["kind"]}));
            continue;
        }
        let doc = open["result"]["doc"].as_u64().unwrap();
        for it in items_of(&c.ok("nav.outline", json!({ "doc": doc })), path) {
            let pos = c.ok(
                "doc.position",
                json!({"doc": doc, "at": {"char": it["name_start"]}}),
            );
            symbols.push(ModelSymbol {
                name: it["name"].as_str().unwrap().to_owned(),
                symbol: json!({"path": path, "kind": it["kind"], "name": it["name"],
                    "start": it["start"], "end": it["end"], "name_start": it["name_start"],
                    "name_end": it["name_end"], "line": pos["line"], "col": pos["col"]}),
            });
        }
        c.ok("doc.close", json!({ "doc": doc }));
    }
    assert_eq!(c.finish().code(), Some(0));
    Model { symbols, skipped }
}

/// The model's answer to `query` with `limit` (default 200): (items, truncated).
fn model_answer(model: &Model, query: &str, limit: Option<usize>) -> (Vec<Value>, bool) {
    let q = query.to_lowercase();
    let mut ranked: Vec<(u8, &Value)> = model
        .symbols
        .iter()
        .filter_map(|m| {
            let n = m.name.to_lowercase();
            if n == q {
                Some((0, &m.symbol))
            } else if n.starts_with(&q) {
                Some((1, &m.symbol))
            } else if n.contains(&q) {
                Some((2, &m.symbol))
            } else {
                None
            }
        })
        .collect();
    ranked.sort_by_key(|(r, _)| *r); // stable: walk order, then start
    let limit = limit.unwrap_or(200);
    let truncated = ranked.len() > limit;
    (
        ranked
            .into_iter()
            .take(limit)
            .map(|(_, s)| s.clone())
            .collect(),
        truncated,
    )
}

/// AC-5 (`nav.workspace-symbol`): ≥ 30 queries over a project of the `valid` corpus (plus an
/// ignored copy, a copy under `target/`, a symlink, a `.txt` copy and a non-UTF-8 `.rs` file)
/// equal the naive model; Pinned (g) example 7; an open, edited document is searched with its
/// current text.
#[test]
fn workspace_symbols_match_naive_model() {
    let files = corpus_set("valid");
    let fx = Fixture::new("workspace");
    let mut stems = Vec::new();
    for p in &files {
        let st = stem(p);
        fx.write(&format!("src/{st}.rs"), corpus_text(p).as_bytes());
        stems.push(st);
    }
    let first = corpus_text(&files[0]);
    fx.write(".gitignore", b"ignored.rs\n");
    fx.write("ignored.rs", first.as_bytes());
    fx.write("target/copy.rs", first.as_bytes());
    fx.write("notes.txt", first.as_bytes());
    fx.write("bad.rs", b"fn \xff() {}");
    // D-124 m3: a name ending in `.RS` does not end in `.rs` (compared exactly): not searched.
    fx.write("src/UPPER.RS", b"fn upper_only_symbol() {}\n");
    symlink(format!("src/{}.rs", stems[0]), fx.root().join("link.rs")).unwrap();

    let model = build_model(&fx.root());
    assert_eq!(
        model.skipped,
        vec![json!({"path": "bad.rs", "reason": "decode_error"})],
        "model: the non-UTF-8 file"
    );
    assert!(model.symbols.len() > 300, "model too small");
    for m in &model.symbols {
        let p = m.symbol["path"].as_str().unwrap();
        assert!(p.starts_with("src/"), "model searched {p}");
    }

    // Queries: exact names, prefixes, inner substrings, case-changed, impl names, no match.
    let mut names: Vec<String> = model.symbols.iter().map(|m| m.name.clone()).collect();
    names.sort();
    names.dedup();
    let mut queries: Vec<(String, Option<usize>)> = Vec::new();
    let step = names.len() / 8;
    for k in 0..8 {
        queries.push((names[k * step + 1].clone(), None));
    }
    let long: Vec<&String> = names.iter().filter(|n| n.chars().count() >= 6).collect();
    let lstep = long.len() / 6;
    for k in 0..6 {
        let n: String = long[k * lstep].chars().take(3).collect();
        queries.push((n, None));
        let inner: String = long[k * lstep + 1].chars().skip(1).take(3).collect();
        queries.push((inner, None));
    }
    for k in 0..4 {
        queries.push((long[k * lstep + 2].to_uppercase(), None));
    }
    let mixed: String = long[3]
        .chars()
        .enumerate()
        .map(|(i, ch)| {
            if i % 2 == 0 {
                ch.to_ascii_uppercase()
            } else {
                ch
            }
        })
        .collect();
    queries.push((mixed, None));
    for q in ["for", " for ", "FOR", "Display", "new", "fmt", "Iterator"] {
        queries.push((q.to_owned(), None));
    }
    for (q, l) in [
        ("e", None),
        ("e", Some(3)),
        ("e", Some(10_000)),
        ("new", Some(1)),
        ("s", Some(57)),
        ("zz_no_such_symbol_zz", None),
        ("qqqxqqq", Some(5)),
    ] {
        queries.push((q.to_owned(), l));
    }
    assert!(queries.len() >= 30);

    let mut c = Control::start(&fx.root());
    let mut nonempty = 0;
    for (q, limit) in &queries {
        let params = match limit {
            Some(l) => json!({"query": q, "limit": l}),
            None => json!({ "query": q }),
        };
        let r = c.ok("nav.workspace-symbols", params.clone());
        let (want, truncated) = model_answer(&model, q, *limit);
        assert_eq!(items_of(&r, "ws"), want, "items for {params}");
        assert_eq!(r["truncated"], json!(truncated), "truncated for {params}");
        assert_eq!(r["skipped"], json!(model.skipped), "skipped for {params}");
        assert_eq!(r["walk_truncated"], json!(false), "{params}");
        assert_eq!(r["too_deep"], json!(0), "{params}");
        if !want.is_empty() {
            nonempty += 1;
        }
    }
    assert!(
        nonempty >= 28,
        "too few queries matched anything: {nonempty}"
    );
    let r = c.ok(
        "nav.workspace-symbols",
        json!({"query": "upper_only_symbol"}),
    );
    assert_eq!(r["items"], json!([]), "a `.RS` file was searched");
    for q in ["upper", "UPPER_ONLY", "symbol"] {
        let r = c.ok("nav.workspace-symbols", json!({ "query": q }));
        for it in items_of(&r, q) {
            let p = it["path"].as_str().unwrap();
            assert!(p.ends_with(".rs"), "query {q}: searched {p}");
        }
    }
    assert!(
        queries.iter().any(|(q, l)| model_answer(&model, q, *l).1),
        "no query was truncated"
    );

    // Open documents: the current text is searched, saved or not; after close it is not.
    let (path, text) = stems
        .iter()
        .zip(&files)
        .map(|(st, p)| (format!("src/{st}.rs"), corpus_text(p)))
        .find(|(_, t)| t.ends_with('\n'))
        .expect("a corpus file ending in a line break");
    let doc = c.ok("doc.open", json!({ "path": path }))["doc"]
        .as_u64()
        .unwrap();
    let l = text.chars().count();
    c.ok(
        "doc.insert",
        json!({"doc": doc, "offset": l, "text": "fn zzz_new() {}"}),
    );
    let pos = c.ok("doc.position", json!({"doc": doc, "at": {"char": l + 3}}));
    let want = json!([{"path": path, "kind": "fn", "name": "zzz_new", "start": l,
        "end": l + 15, "name_start": l + 3, "name_end": l + 10, "line": pos["line"],
        "col": pos["col"]}]);
    for q in ["zzz_new", "ZZZ_NEW", "zzz_"] {
        let r = c.ok("nav.workspace-symbols", json!({ "query": q }));
        assert_eq!(r["items"], want, "open edited document, query {q}");
    }
    let r = c.ok(
        "nav.workspace-symbols",
        json!({"query": "zzz_new", "limit": 1}),
    );
    assert_eq!(r["truncated"], json!(false));
    c.ok("doc.close", json!({"doc": doc, "discard": true}));
    let r = c.ok("nav.workspace-symbols", json!({"query": "zzz_new"}));
    assert_eq!(r["items"], json!([]), "closed document still searched");
    assert_eq!(c.finish().code(), Some(0));
    assert_eq!(fs::read_to_string(fx.root().join(&path)).unwrap(), text);

    // Pinned (g) example 7.
    let fx7 = Fixture::new("example7");
    fx7.write("a.rs", b"fn foo() {}\nstruct FooBar;\n");
    fx7.write("b/c.rs", b"mod foo { fn barfoo() {} }\n");
    fx7.write("z.txt", b"fn foo() {}");
    let mut s = Script::default();
    let all = s.push("nav.workspace-symbols", json!({"query": "FOO"}));
    let two = s.push("nav.workspace-symbols", json!({"query": "FOO", "limit": 2}));
    let empty = s.push("nav.workspace-symbols", json!({"query": ""}));
    let (resp, _) = s.run(&fx7, "example7");
    let foo = json!({"path": "a.rs", "kind": "fn", "name": "foo", "start": 0, "end": 11,
        "name_start": 3, "name_end": 6, "line": 0, "col": 3});
    let mod_foo = json!({"path": "b/c.rs", "kind": "mod", "name": "foo", "start": 0,
        "end": 26, "name_start": 4, "name_end": 7, "line": 0, "col": 4});
    let foobar = json!({"path": "a.rs", "kind": "struct", "name": "FooBar", "start": 12,
        "end": 26, "name_start": 19, "name_end": 25, "line": 1, "col": 7});
    let barfoo = json!({"path": "b/c.rs", "kind": "fn", "name": "barfoo", "start": 10,
        "end": 24, "name_start": 13, "name_end": 19, "line": 0, "col": 13});
    let r = ok(&resp[all], "example 7");
    assert_eq!(
        r,
        &json!({"items": [foo, mod_foo, foobar, barfoo], "truncated": false, "skipped": [],
            "walk_truncated": false, "too_deep": 0})
    );
    let r = ok(&resp[two], "example 7, limit 2");
    assert_eq!(r["items"], json!([foo, mod_foo]));
    assert_eq!(r["truncated"], json!(true));
    refused(&resp[empty], "rejected", "invalid_pattern", "empty query");
}

// ---------------------------------------------------------------------------------------------
// AC-8: limits and refusals
// ---------------------------------------------------------------------------------------------

/// AC-8 (`nav.workspace-symbol`): params refusals; the default `limit` of 200 and the largest
/// of 10,000 over 250 files in tree order; a file below the depth bound is not searched and
/// `too_deep` counts the directory.
#[test]
fn workspace_symbols_limits_and_refusals() {
    let fx = Fixture::new("limits");
    let mut names: Vec<String> = (0..250).map(|i| format!("f{i}.rs")).collect();
    for n in &names {
        fx.write(n, b"fn hit() {}");
    }
    names.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    let hit = |p: &str| {
        json!({"path": p, "kind": "fn", "name": "hit", "start": 0, "end": 11,
            "name_start": 3, "name_end": 6, "line": 0, "col": 3})
    };
    let mut s = Script::default();
    let bad: Vec<(Value, &str)> = vec![
        (json!({"query": "hit", "limit": 0}), "invalid_params"),
        (json!({"query": "hit", "limit": 10_001}), "invalid_params"),
        (json!({"query": "hit", "limit": null}), "invalid_params"),
        (json!({"query": "hit", "limit": -1}), "invalid_params"),
        (json!({"query": "hit", "limit": 2.5}), "invalid_params"),
        (json!({"query": "hit", "limit": "5"}), "invalid_params"),
        (json!({}), "invalid_params"),
        (json!({"limit": 5}), "invalid_params"),
        (json!({"query": null}), "invalid_params"),
        (json!({"query": 5}), "invalid_params"),
        (json!({"query": "hit", "path": "src"}), "invalid_params"),
        (json!({"query": "", "limit": 0}), "invalid_params"),
        (json!({"query": ""}), "invalid_pattern"),
        (json!({"query": "", "limit": 10}), "invalid_pattern"),
    ];
    let bad_ids: Vec<usize> = bad
        .iter()
        .map(|(p, _)| s.push("nav.workspace-symbols", p.clone()))
        .collect();
    let default = s.push("nav.workspace-symbols", json!({"query": "hit"}));
    let max = s.push(
        "nav.workspace-symbols",
        json!({"query": "HIT", "limit": 10_000}),
    );
    let exact = s.push(
        "nav.workspace-symbols",
        json!({"query": "hit", "limit": 250}),
    );
    let one_less = s.push(
        "nav.workspace-symbols",
        json!({"query": "it", "limit": 249}),
    );
    let (resp, _) = s.run(&fx, "limits");
    for ((p, kind), id) in bad.iter().zip(bad_ids) {
        if *kind == "invalid_params" {
            invalid_params(&resp[id], &format!("{p}"));
        } else {
            refused(&resp[id], "rejected", kind, &format!("{p}"));
        }
    }
    let all: Vec<Value> = names.iter().map(|n| hit(n)).collect();
    let r = ok(&resp[default], "default limit");
    assert_eq!(items_of(r, "default"), all[..200].to_vec());
    assert_eq!(r["truncated"], json!(true));
    assert_eq!(r["walk_truncated"], json!(false));
    let r = ok(&resp[max], "limit 10000");
    assert_eq!(items_of(r, "max"), all);
    assert_eq!(r["truncated"], json!(false));
    let r = ok(&resp[exact], "limit 250");
    assert_eq!(items_of(r, "exact"), all);
    assert_eq!(r["truncated"], json!(false));
    let r = ok(&resp[one_less], "limit 249");
    assert_eq!(items_of(r, "one less"), all[..249].to_vec());
    assert_eq!(r["truncated"], json!(true));

    // The depth bound: a directory whose path has 128 segments is listed, not descended.
    let fx = Fixture::new("depth");
    let dir = |n: usize| vec!["d"; n].join("/");
    fx.write(&format!("{}/x.rs", dir(130)), b"fn deep() {}");
    fx.write(&format!("{}/y.rs", dir(127)), b"fn deep_ok() {}");
    fx.write("top.rs", b"fn shallow() {}");
    let mut s = Script::default();
    let deep = s.push("nav.workspace-symbols", json!({"query": "deep"}));
    let top = s.push("nav.workspace-symbols", json!({"query": "shallow"}));
    let (resp, _) = s.run(&fx, "depth");
    let r = ok(&resp[deep], "deep");
    let y = format!("{}/y.rs", dir(127));
    assert_eq!(
        r,
        &json!({"items": [{"path": y, "kind": "fn", "name": "deep_ok", "start": 0, "end": 15,
            "name_start": 3, "name_end": 10, "line": 0, "col": 3}], "truncated": false,
            "skipped": [], "walk_truncated": false, "too_deep": 1})
    );
    let r = ok(&resp[top], "shallow");
    assert_eq!(r["items"][0]["path"], json!("top.rs"));
    assert_eq!(r["too_deep"], json!(1));
}

/// D-124 m4 (also guards m1): the search walks beyond the 50,000 entries `project.tree {}`
/// lists (ruling Q-3: limit 1,000,000). A root with 50,000 empty files in `a/` and `zz.rs` after
/// them: `project.tree {}` stops before `zz.rs`, the search still finds its symbol.
#[test]
fn workspace_symbols_search_beyond_project_tree_default() {
    let fx = Fixture::new("beyond-default");
    let dir = fx.root().join("a");
    fs::create_dir_all(&dir).unwrap();
    for i in 0..50_000 {
        fs::write(dir.join(format!("e{i:05}")), b"").unwrap();
    }
    fx.write("zz.rs", b"fn hit_beyond() {}\n");
    let mut s = Script::default();
    let tree = s.push("project.tree", json!({}));
    let full = s.push("project.tree", json!({"limit": 1_000_000}));
    let ws = s.push("nav.workspace-symbols", json!({"query": "hit"}));
    let (resp, _) = s.run(&fx, "beyond-default");
    let has_zz = |r: &Value| {
        r["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["path"] == json!("zz.rs"))
    };
    let t = ok(&resp[tree], "project.tree {}");
    assert_eq!(t["truncated"], json!(true));
    assert!(
        !has_zz(t),
        "project.tree {{}} reached zz.rs: the case tests nothing"
    );
    let t = ok(&resp[full], "project.tree limit 1000000");
    assert_eq!(t["truncated"], json!(false));
    assert!(has_zz(t));
    let r = ok(&resp[ws], "nav.workspace-symbols beyond the default");
    assert_eq!(
        r,
        &json!({"items": [{"path": "zz.rs", "kind": "fn", "name": "hit_beyond", "start": 0,
            "end": 18, "name_start": 3, "name_end": 13, "line": 0, "col": 3}],
            "truncated": false, "skipped": [], "walk_truncated": false, "too_deep": 0})
    );
}

// ---------------------------------------------------------------------------------------------
// Published schema (Interface: mutating flags; optional params publish their true default)
// ---------------------------------------------------------------------------------------------

/// Every `"default"` keyword anywhere in `v`, with its path.
fn defaults(v: &Value, at: &str, out: &mut Vec<(String, Value)>) {
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                if k == "default" {
                    out.push((at.to_owned(), x.clone()));
                }
                defaults(x, &format!("{at}/{k}"), out);
            }
        }
        Value::Array(a) => {
            for (i, x) in a.iter().enumerate() {
                defaults(x, &format!("{at}/{i}"), out);
            }
        }
        _ => {}
    }
}

/// The four commands are published with their `mutating` flag (Interface table), and no
/// optional param publishes `"default": null`; `col` (if it has a default) publishes 0 and
/// `limit` 200.
#[test]
fn nav_commands_published_schema() {
    let out = kiln(&["commands", "--schema"]);
    assert_eq!(out.status.code(), Some(0), "commands --schema failed");
    let dump: Value = serde_json::from_slice(&out.stdout).unwrap();
    let commands = dump["commands"].as_array().unwrap();
    for (name, mutating) in [
        ("nav.outline", false),
        ("nav.goto-line", true),
        ("nav.goto-symbol", true),
        ("nav.workspace-symbols", false),
    ] {
        let entry = commands
            .iter()
            .find(|c| c["name"] == json!(name))
            .unwrap_or_else(|| panic!("`{name}` is not in `commands --schema`"));
        assert_eq!(entry["mutating"], json!(mutating), "{name}");
        let mut found = Vec::new();
        defaults(&entry["params_schema"], "", &mut found);
        for (at, d) in found {
            assert!(!d.is_null(), "{name}: `default: null` at {at}");
            if at.ends_with("/col") {
                assert_eq!(d, json!(0), "{name}: default of col");
            }
            if at.ends_with("/limit") {
                assert_eq!(d, json!(200), "{name}: default of limit");
            }
        }
        assert!(
            !entry["examples"].as_array().unwrap().is_empty(),
            "{name}: no example"
        );
    }
}
