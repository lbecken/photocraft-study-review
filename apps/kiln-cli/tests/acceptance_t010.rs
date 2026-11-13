//! T-010 acceptance tests through the built `kiln-cli` binary (`run --root`, `run --log`,
//! `commands --schema`, `commands --audit`) and the committed documents (AC-1 … AC-6, rulings
//! q1–q4 of D-059). Written from the task specification alone by the test author.
//!
//! Oracle: a naive model in this file (split on `\r\n|\r|\n`; UTF-8 and UTF-16 lengths of
//! char prefixes) unless a test says otherwise. The file uses no new API: it compiles on base
//! and fails there by assertion (`doc.line` / `doc.position` are unknown commands, the schema
//! dump has neither command nor the kind `not_a_boundary`).
//!
//! Every spawned process is killed after [`TIMEOUT`] (D-028 (8)).
#![cfg(test)]

use std::collections::BTreeSet;
use std::io::Read;
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
        .join("acceptance_t010")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("create temp dir");
    d
}

/// Run `kiln-cli args` (empty stdin); kill it after [`TIMEOUT`].
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
    Output {
        status,
        stdout: out.join().expect("stdout reader"),
        stderr: err.join().expect("stderr reader"),
    }
}

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn repo_file(rel: &str) -> String {
    let p = repo_path(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

/// One request of a script and what its response must be.
enum Expect {
    /// `ok:true` with exactly this result.
    Result(Value),
    /// `ok:true`, result not checked here.
    AnyOk,
    /// `ok:false` with this code and kind, and (if given) exactly this message.
    Error {
        code: &'static str,
        kind: String,
        message: Option<String>,
    },
}

fn rejected(kind: &str, message: Option<String>) -> Expect {
    Expect::Error {
        code: "rejected",
        kind: kind.to_owned(),
        message,
    }
}

fn invalid_params() -> Expect {
    Expect::Error {
        code: "invalid_params",
        kind: "schema".to_owned(),
        message: None,
    }
}

/// A `kiln-cli run` script: requests numbered 1, 2, … with their expectations.
#[derive(Default)]
struct Script {
    lines: Vec<String>,
    expect: Vec<Expect>,
    /// (command, params) of the requests expected to succeed, in order.
    successes: Vec<(String, Value)>,
}

impl Script {
    fn push(&mut self, command: &str, params: Value, expect: Expect) -> usize {
        let id = self.lines.len() + 1;
        self.lines
            .push(json!({"id": id, "command": command, "params": params.clone()}).to_string());
        if !matches!(expect, Expect::Error { .. }) {
            self.successes.push((command.to_owned(), params));
        }
        self.expect.push(expect);
        id
    }

    /// Run the script with `--root root` (and `--log` into `work`); check every response and
    /// return the responses and the log file's text.
    fn run(&self, work: &Path, root: &Path, name: &str) -> (Vec<Value>, String) {
        let script = work.join(format!("{name}.jsonl"));
        let log = work.join(format!("{name}.log.jsonl"));
        let mut text = self.lines.join("\n");
        text.push('\n');
        std::fs::write(&script, text).expect("write script");
        let out = kiln(&[
            "run",
            script.to_str().expect("utf-8 path"),
            "--root",
            root.to_str().expect("utf-8 path"),
            "--log",
            log.to_str().expect("utf-8 path"),
        ]);
        let stdout = String::from_utf8(out.stdout.clone()).expect("stdout UTF-8");
        let responses: Vec<Value> = stdout
            .lines()
            .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("not JSON ({e}): {l}")))
            .collect();
        assert_eq!(
            responses.len(),
            self.lines.len(),
            "one response per request; stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let mut first_failure = None;
        for (i, (resp, want)) in responses.iter().zip(&self.expect).enumerate() {
            let id = i + 1;
            let req = &self.lines[i];
            assert_eq!(resp["id"], json!(id), "{req} -> {resp}");
            match want {
                Expect::Result(r) => {
                    assert_eq!(resp["ok"], json!(true), "{req} -> {resp}");
                    assert_eq!(&resp["result"], r, "{req} -> {resp}");
                }
                Expect::AnyOk => assert_eq!(resp["ok"], json!(true), "{req} -> {resp}"),
                Expect::Error {
                    code,
                    kind,
                    message,
                } => {
                    first_failure.get_or_insert(id);
                    assert_eq!(resp["ok"], json!(false), "{req} -> {resp}");
                    assert_eq!(resp["error"]["code"], json!(code), "{req} -> {resp}");
                    assert_eq!(resp["error"]["kind"], json!(kind), "{req} -> {resp}");
                    if let Some(m) = message {
                        assert_eq!(resp["error"]["message"], json!(m), "{req} -> {resp}");
                    }
                }
            }
        }
        let want_code = if first_failure.is_some() { 3 } else { 0 };
        assert_eq!(out.status.code(), Some(want_code), "exit code");
        let log = std::fs::read_to_string(&log).expect("read log");
        (responses, log)
    }
}

/// The log written by `--log` lists exactly the successful requests, numbered from 1, each as a
/// script line `{"command","id","params"}` (ruling q1: no `mutating` field).
fn assert_log_lists_successes(log: &str, script: &Script) {
    assert!(log.is_empty() || log.ends_with('\n'), "{log:?}");
    let got: Vec<Value> = log
        .lines()
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("log line ({e}): {l}")))
        .collect();
    assert_eq!(got.len(), script.successes.len(), "log:\n{log}");
    for (i, (entry, (command, params))) in got.iter().zip(&script.successes).enumerate() {
        let keys: BTreeSet<&str> = entry
            .as_object()
            .expect("log line is an object")
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(keys, BTreeSet::from(["command", "id", "params"]), "{entry}");
        assert_eq!(entry["id"], json!(i + 1), "{entry}");
        assert_eq!(entry["command"], json!(command), "{entry}");
        assert_eq!(&entry["params"], params, "{entry}");
    }
}

// ---------------------------------------------------------------- naive model

/// One line of the naive split.
#[derive(Clone, Debug)]
struct NaiveLine {
    text: String,
    start: usize,
    end: usize,
    ending: &'static str,
}

/// The naive model of a text: chars, lines, and the five numbers of every char offset.
struct Model {
    chars: Vec<char>,
    lines: Vec<NaiveLine>,
    /// `byte_of[c]`, `utf16_of[c]` for `c` in `0..=len_chars`.
    byte_of: Vec<usize>,
    utf16_of: Vec<usize>,
}

impl Model {
    fn new(t: &str) -> Model {
        let chars: Vec<char> = t.chars().collect();
        let mut lines = Vec::new();
        let mut start = 0;
        let mut i = 0;
        while i < chars.len() {
            let (brk, ending) = match chars[i] {
                '\r' if chars.get(i + 1) == Some(&'\n') => (2, "crlf"),
                '\r' => (1, "cr"),
                '\n' => (1, "lf"),
                _ => {
                    i += 1;
                    continue;
                }
            };
            lines.push(NaiveLine {
                text: chars[start..i].iter().collect(),
                start,
                end: i,
                ending,
            });
            i += brk;
            start = i;
        }
        lines.push(NaiveLine {
            text: chars[start..].iter().collect(),
            start,
            end: chars.len(),
            ending: "none",
        });
        let mut byte_of = vec![0];
        let mut utf16_of = vec![0];
        for ch in &chars {
            byte_of.push(byte_of.last().unwrap() + ch.len_utf8());
            utf16_of.push(utf16_of.last().unwrap() + ch.len_utf16());
        }
        Model {
            chars,
            lines,
            byte_of,
            utf16_of,
        }
    }

    fn len_chars(&self) -> usize {
        self.chars.len()
    }

    fn len_bytes(&self) -> usize {
        *self.byte_of.last().unwrap()
    }

    fn len_utf16(&self) -> usize {
        *self.utf16_of.last().unwrap()
    }

    fn mid_crlf(&self, c: usize) -> bool {
        c > 0 && c < self.chars.len() && self.chars[c - 1] == '\r' && self.chars[c] == '\n'
    }

    fn valid_chars(&self) -> Vec<usize> {
        (0..=self.len_chars())
            .filter(|&c| !self.mid_crlf(c))
            .collect()
    }

    /// The five numbers of a valid char offset.
    fn five(&self, c: usize) -> Value {
        let (line, l) = self
            .lines
            .iter()
            .enumerate()
            .find(|(_, l)| l.start <= c && c <= l.end)
            .expect("valid offset lies on a line");
        json!({"char": c, "byte": self.byte_of[c], "utf16": self.utf16_of[c],
            "line": line, "col": c - l.start})
    }

    fn line_result(&self, i: usize) -> Value {
        let l = &self.lines[i];
        json!({"line": i, "text": l.text, "start": l.start, "end": l.end, "ending": l.ending})
    }

    /// Expected answer of `doc.position` for the position form `at`, per the spec: the five
    /// numbers, or the refusal with kind and message (`invalid position: <Display>`).
    fn position(&self, at: &Value) -> Expect {
        let bad =
            |kind: &str, text: String| rejected(kind, Some(format!("invalid position: {text}")));
        let oob = |i: usize, len: usize| {
            bad(
                "out_of_range",
                format!("index {i} is out of bounds (length {len})"),
            )
        };
        let at_char = |c: usize| {
            if self.mid_crlf(c) {
                bad(
                    "not_a_boundary",
                    format!("char index {c} is between the \\r and \\n of a CRLF"),
                )
            } else {
                Expect::Result(self.five(c))
            }
        };
        let n = |v: &Value| v.as_u64().expect("u64") as usize;
        if let Some(c) = at.get("char") {
            let c = n(c);
            if c > self.len_chars() {
                return oob(c, self.len_chars());
            }
            return at_char(c);
        }
        if let Some(b) = at.get("byte") {
            let b = n(b);
            if b > self.len_bytes() {
                return oob(b, self.len_bytes());
            }
            return match self.byte_of.iter().position(|&x| x == b) {
                Some(c) => at_char(c),
                None => bad(
                    "not_a_boundary",
                    format!("byte offset {b} is inside a multi-byte char"),
                ),
            };
        }
        if let Some(u) = at.get("utf16") {
            let u = n(u);
            if u > self.len_utf16() {
                return oob(u, self.len_utf16());
            }
            return match self.utf16_of.iter().position(|&x| x == u) {
                Some(c) => at_char(c),
                None => bad(
                    "not_a_boundary",
                    format!("UTF-16 offset {u} is inside a surrogate pair"),
                ),
            };
        }
        let lc = &at["line_col"];
        let (line, col) = (n(&lc["line"]), n(&lc["col"]));
        if line >= self.lines.len() {
            return bad(
                "out_of_range",
                format!(
                    "line {line} is out of bounds (buffer has {} lines)",
                    self.lines.len()
                ),
            );
        }
        let l = &self.lines[line];
        let len = l.end - l.start;
        if col > len {
            return bad(
                "out_of_range",
                format!("column {col} is out of bounds for line {line} ({len} chars)"),
            );
        }
        Expect::Result(self.five(l.start + col))
    }
}

/// The worked example of Pinned (g).
const WORKED: &str = "a\r\nbé😀\rc\n";

/// SplitMix64: a fixed-seed generator, so the texts are the same on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// The corpus of AC-1/AC-2: hand-picked edge texts (empty, no final newline, CR-only, `\r\r\n`,
/// empty lines, the worked example) and 60 fixed-seed random texts over `a é 😀 中 \n \r\n \r`.
fn corpus() -> Vec<String> {
    let mut texts: Vec<String> = [
        "",
        "a",
        "\n",
        "\r",
        "\r\n",
        "\r\r\n\n",
        "\n\r",
        "abc",
        "中😀é",
        "\n\n\n",
        "x\r\ny",
        "😀\r\n😀\r😀\n😀",
        WORKED,
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    const ALPHABET: [&str; 7] = ["a", "é", "😀", "中", "\n", "\r\n", "\r"];
    let mut rng = Rng(0x7010_7010);
    for _ in 0..60 {
        let len = (rng.next() % 17) as usize;
        let t: String = (0..len)
            .map(|_| ALPHABET[(rng.next() % ALPHABET.len() as u64) as usize])
            .collect();
        texts.push(t);
    }
    texts
}

/// Write each text of `texts` as `t<i>.txt` (UTF-8) under `root`.
fn write_corpus(root: &Path, texts: &[String]) {
    for (i, t) in texts.iter().enumerate() {
        std::fs::write(root.join(format!("t{i:03}.txt")), t).expect("write corpus file");
    }
}

/// Open every corpus file (doc `i + 1`), expecting success.
fn open_corpus(s: &mut Script, texts: &[String]) {
    for i in 0..texts.len() {
        s.push(
            "doc.open",
            json!({"path": format!("t{i:03}.txt")}),
            Expect::AnyOk,
        );
    }
}

// ---------------------------------------------------------------- AC-1

/// The worked example of Pinned (g), exact: four lines with their text, start, end and break.
#[test]
fn worked_example_lines_are_exact() {
    let work = fresh_dir("worked-lines");
    let root = work.join("root");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("w.txt"), WORKED).unwrap();
    let mut s = Script::default();
    s.push("doc.open", json!({"path": "w.txt"}), Expect::AnyOk);
    let want = [
        json!({"line": 0, "text": "a", "start": 0, "end": 1, "ending": "crlf"}),
        json!({"line": 1, "text": "bé😀", "start": 3, "end": 6, "ending": "cr"}),
        json!({"line": 2, "text": "c", "start": 7, "end": 8, "ending": "lf"}),
        json!({"line": 3, "text": "", "start": 9, "end": 9, "ending": "none"}),
    ];
    for (l, w) in want.iter().enumerate() {
        s.push(
            "doc.line",
            json!({"doc": 1, "line": l}),
            Expect::Result(w.clone()),
        );
    }
    s.run(&work, &root, "s");
}

/// AC-1: for every corpus text saved as a UTF-8 file, `doc.line` of every line
/// `0..len_lines` equals the naive split, and `doc.info.len_lines` is the number of lines.
#[test]
fn doc_line_matches_naive_split_for_every_line() {
    let texts = corpus();
    let work = fresh_dir("lines");
    let root = work.join("root");
    std::fs::create_dir_all(&root).unwrap();
    write_corpus(&root, &texts);
    let mut s = Script::default();
    open_corpus(&mut s, &texts);
    let mut info_ids = Vec::new();
    for (i, t) in texts.iter().enumerate() {
        let doc = i + 1;
        let m = Model::new(t);
        info_ids.push((
            s.push("doc.info", json!({"doc": doc}), Expect::AnyOk),
            m.lines.len(),
        ));
        for l in 0..m.lines.len() {
            s.push(
                "doc.line",
                json!({"doc": doc, "line": l}),
                Expect::Result(m.line_result(l)),
            );
        }
    }
    let (responses, log) = s.run(&work, &root, "s");
    for (id, n) in info_ids {
        assert_eq!(
            responses[id - 1]["result"]["len_lines"],
            json!(n),
            "{}",
            s.lines[id - 1]
        );
    }
    assert_log_lists_successes(&log, &s);
}

// ---------------------------------------------------------------- AC-2

/// AC-2: `doc.position` from `{"char":c}` for every valid `c` gives the naive five numbers;
/// from each returned `byte`, `utf16` and `line_col` (taken from the first run's responses)
/// it gives the same five numbers. Every invalid offset in each unit is refused with the
/// spec's kind and message (inside a char, a surrogate pair or a CRLF; one past the end; a
/// line or column past the end).
#[test]
fn doc_position_round_trips_every_valid_position() {
    let texts = corpus();
    let work = fresh_dir("positions");
    let root = work.join("root");
    std::fs::create_dir_all(&root).unwrap();
    write_corpus(&root, &texts);
    let models: Vec<Model> = texts.iter().map(|t| Model::new(t)).collect();

    // Run 1: from char, every valid offset.
    let mut s1 = Script::default();
    open_corpus(&mut s1, &texts);
    let mut asked = Vec::new();
    for (i, m) in models.iter().enumerate() {
        for c in m.valid_chars() {
            let id = s1.push(
                "doc.position",
                json!({"doc": i + 1, "at": {"char": c}}),
                Expect::Result(m.five(c)),
            );
            asked.push((i, id));
        }
    }
    let (r1, _) = s1.run(&work, &root, "s1");

    // Run 2: from the returned byte, utf16 and line/col; then every offset in every unit,
    // valid or not, against the model.
    let mut s2 = Script::default();
    open_corpus(&mut s2, &texts);
    for (i, id) in asked {
        let five = r1[id - 1]["result"].clone();
        let doc = i + 1;
        for at in [
            json!({"byte": five["byte"]}),
            json!({"utf16": five["utf16"]}),
            json!({"line_col": {"line": five["line"], "col": five["col"]}}),
        ] {
            s2.push(
                "doc.position",
                json!({"doc": doc, "at": at}),
                Expect::Result(five.clone()),
            );
        }
    }
    for (i, m) in models.iter().enumerate() {
        let doc = i + 1;
        let mut forms = Vec::new();
        forms.extend((0..=m.len_chars() + 1).map(|c| json!({"char": c})));
        forms.extend((0..=m.len_bytes() + 1).map(|b| json!({"byte": b})));
        forms.extend((0..=m.len_utf16() + 1).map(|u| json!({"utf16": u})));
        for line in 0..=m.lines.len() {
            let width = m.lines.get(line).map_or(0, |l| l.end - l.start);
            forms
                .extend((0..=width + 1).map(|col| json!({"line_col": {"line": line, "col": col}})));
        }
        for at in forms {
            let want = m.position(&at);
            s2.push("doc.position", json!({"doc": doc, "at": at}), want);
        }
    }
    s2.run(&work, &root, "s2");
}

/// The worked example of Pinned (g): char → (byte, utf16, line, col) exactly as listed.
#[test]
fn worked_example_positions_are_exact() {
    let work = fresh_dir("worked-positions");
    let root = work.join("root");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("w.txt"), WORKED).unwrap();
    let mut s = Script::default();
    s.push("doc.open", json!({"path": "w.txt"}), Expect::AnyOk);
    let table = [
        (0, 0, 0, 0, 0),
        (1, 1, 1, 0, 1),
        (3, 3, 3, 1, 0),
        (4, 4, 4, 1, 1),
        (5, 6, 5, 1, 2),
        (6, 10, 7, 1, 3),
        (7, 11, 8, 2, 0),
        (8, 12, 9, 2, 1),
        (9, 13, 10, 3, 0),
    ];
    for (c, b, u, l, col) in table {
        let five = json!({"char": c, "byte": b, "utf16": u, "line": l, "col": col});
        for at in [
            json!({"char": c}),
            json!({"byte": b}),
            json!({"utf16": u}),
            json!({"line_col": {"line": l, "col": col}}),
        ] {
            s.push(
                "doc.position",
                json!({"doc": 1, "at": at}),
                Expect::Result(five.clone()),
            );
        }
    }
    s.run(&work, &root, "s");
}

// ---------------------------------------------------------------- AC-3

/// AC-3: on the worked-example text the six refusals of the table give exactly the listed
/// kind and message with `ok:false`; `{"char":2}` is `not_a_boundary`; hostile `u64` values
/// are `out_of_range`; unknown, closed and `0` ids are `no_such_document` (q4); malformed
/// params are `invalid_params`/`schema`; precedence is params → id → position; afterwards
/// `doc.text` shows the text unchanged and the `--log` lists none of the refused requests.
#[test]
fn refusals_have_exact_kinds_and_messages_and_change_nothing() {
    let work = fresh_dir("refusals");
    let root = work.join("root");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("w.txt"), WORKED).unwrap();
    std::fs::write(root.join("c.txt"), "x\ny").unwrap();
    let text_result = json!({"text": WORKED, "len_chars": 9});
    let mut s = Script::default();
    s.push("doc.open", json!({"path": "w.txt"}), Expect::AnyOk);
    s.push(
        "doc.text",
        json!({"doc": 1}),
        Expect::Result(text_result.clone()),
    );
    let pos = |at: Value| json!({"doc": 1, "at": at});
    let msg = |m: &str| Some(m.to_owned());

    // The six rows of the table, exactly.
    s.push(
        "doc.position",
        pos(json!({"byte": 14})),
        rejected(
            "out_of_range",
            msg("invalid position: index 14 is out of bounds (length 13)"),
        ),
    );
    s.push(
        "doc.line",
        json!({"doc": 1, "line": 4}),
        rejected(
            "out_of_range",
            msg("invalid line: line 4 is out of bounds (buffer has 4 lines)"),
        ),
    );
    s.push(
        "doc.position",
        pos(json!({"line_col": {"line": 1, "col": 4}})),
        rejected(
            "out_of_range",
            msg("invalid position: column 4 is out of bounds for line 1 (3 chars)"),
        ),
    );
    s.push(
        "doc.position",
        pos(json!({"byte": 5})),
        rejected(
            "not_a_boundary",
            msg("invalid position: byte offset 5 is inside a multi-byte char"),
        ),
    );
    s.push(
        "doc.position",
        pos(json!({"byte": 7})),
        rejected(
            "not_a_boundary",
            msg("invalid position: byte offset 7 is inside a multi-byte char"),
        ),
    );
    s.push(
        "doc.position",
        pos(json!({"utf16": 6})),
        rejected(
            "not_a_boundary",
            msg("invalid position: UTF-16 offset 6 is inside a surrogate pair"),
        ),
    );
    s.push(
        "doc.position",
        pos(json!({"char": 2})),
        rejected(
            "not_a_boundary",
            msg("invalid position: char index 2 is between the \\r and \\n of a CRLF"),
        ),
    );
    // The same Display texts under the `invalid position:` prefix for the other forms.
    s.push(
        "doc.position",
        pos(json!({"char": 10})),
        rejected(
            "out_of_range",
            msg("invalid position: index 10 is out of bounds (length 9)"),
        ),
    );
    s.push(
        "doc.position",
        pos(json!({"utf16": 11})),
        rejected(
            "out_of_range",
            msg("invalid position: index 11 is out of bounds (length 10)"),
        ),
    );
    s.push(
        "doc.position",
        pos(json!({"line_col": {"line": 4, "col": 0}})),
        rejected(
            "out_of_range",
            msg("invalid position: line 4 is out of bounds (buffer has 4 lines)"),
        ),
    );
    // Pinned (e): any u64 is accepted by the schema and answered with out_of_range.
    let max = u64::MAX;
    for at in [
        json!({"char": max}),
        json!({"byte": max}),
        json!({"utf16": max}),
        json!({"line_col": {"line": max, "col": 0}}),
        json!({"line_col": {"line": 0, "col": max}}),
        json!({"line_col": {"line": max, "col": max}}),
    ] {
        s.push("doc.position", pos(at), rejected("out_of_range", None));
    }
    s.push(
        "doc.line",
        json!({"doc": 1, "line": max}),
        rejected("out_of_range", None),
    );
    // Ids: unknown, 0 (q4), closed.
    s.push("doc.open", json!({"path": "c.txt"}), Expect::AnyOk);
    s.push("doc.close", json!({"doc": 2}), Expect::AnyOk);
    for doc in [0u64, 2, 99, max] {
        s.push(
            "doc.line",
            json!({"doc": doc, "line": 0}),
            rejected("no_such_document", None),
        );
        s.push(
            "doc.position",
            json!({"doc": doc, "at": {"char": 0}}),
            rejected("no_such_document", None),
        );
    }
    // Params that do not match the schema.
    for p in [
        json!({"doc": 1, "at": {"char": 1, "byte": 1}}),
        json!({"doc": 1, "at": {}}),
        json!({"doc": 1, "at": {"char": -1}}),
        json!({"doc": 1, "at": {"char": 1.5}}),
        json!({"doc": 1, "at": {"char": "1"}}),
        json!({"doc": 1, "at": {"col": 1}}),
        json!({"doc": 1, "at": {"line_col": {"line": 0}}}),
        json!({"doc": 1, "at": {"line_col": {"line": 0, "col": -1}}}),
        json!({"doc": 1, "at": {"line_col": {"line": 0, "col": 0, "x": 0}}}),
        json!({"doc": 1, "at": {"char": 0}, "x": 1}),
        json!({"doc": 1}),
        json!({"at": {"char": 0}}),
        json!({"doc": -1, "at": {"char": 0}}),
        json!({"doc": 1, "at": 0}),
    ] {
        s.push("doc.position", p, invalid_params());
    }
    for p in [
        json!({"doc": 1, "line": -1}),
        json!({"doc": 1, "line": 0, "x": 1}),
        json!({"doc": 1}),
        json!({"line": 0}),
        json!({"doc": 1, "line": 0.5}),
        json!({"doc": -1, "line": 0}),
    ] {
        s.push("doc.line", p, invalid_params());
    }
    // Precedence (Pinned (a)): params schema, then the id, then the position or line.
    s.push("doc.line", json!({"doc": 99, "line": -1}), invalid_params());
    s.push(
        "doc.position",
        json!({"doc": 99, "at": {"char": -1}}),
        invalid_params(),
    );
    s.push(
        "doc.line",
        json!({"doc": 99, "line": 1000}),
        rejected("no_such_document", None),
    );
    s.push(
        "doc.position",
        json!({"doc": 99, "at": {"byte": 5}}),
        rejected("no_such_document", None),
    );
    s.push(
        "doc.position",
        json!({"doc": 2, "at": {"char": 2}}),
        rejected("no_such_document", None),
    );
    // Ruling t1 (D-064): the id is checked before the position in every unit: an
    // out-of-range value (just past the end, or huge) with an unknown, closed or 0 id is
    // `no_such_document`, never `out_of_range`.
    for doc in [0u64, 2, 99] {
        for at in [
            json!({"char": 10}),
            json!({"char": max}),
            json!({"byte": 14}),
            json!({"byte": max}),
            json!({"utf16": 11}),
            json!({"utf16": max}),
            json!({"line_col": {"line": 4, "col": 0}}),
            json!({"line_col": {"line": max, "col": 0}}),
            json!({"line_col": {"line": 1, "col": 4}}),
            json!({"line_col": {"line": 0, "col": max}}),
        ] {
            s.push(
                "doc.position",
                json!({"doc": doc, "at": at}),
                rejected("no_such_document", None),
            );
        }
    }
    // Nothing changed.
    s.push("doc.text", json!({"doc": 1}), Expect::Result(text_result));
    let (_, log) = s.run(&work, &root, "s");
    assert_log_lists_successes(&log, &s);
    assert_eq!(std::fs::read_to_string(root.join("w.txt")).unwrap(), WORKED);
}

// ---------------------------------------------------------------- AC-4

/// AC-4 (ruling q1): `doc.line`/`doc.position` requests (successful and refused) between two
/// single-char inserts leave `doc.info` unchanged, do not end the typing group (one `doc.undo`
/// removes both chars) and do not touch redo; `run --log` lists the successful requests as
/// script lines.
#[test]
fn line_and_position_are_read_only_and_logged() {
    let work = fresh_dir("readonly");
    let root = work.join("root");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("u.txt"), "ab\n").unwrap();
    let mut s = Script::default();
    let reads = |s: &mut Script| {
        s.push("doc.line", json!({"doc": 1, "line": 0}), Expect::AnyOk);
        s.push("doc.line", json!({"doc": 1, "line": 1}), Expect::AnyOk);
        s.push(
            "doc.position",
            json!({"doc": 1, "at": {"char": 1}}),
            Expect::AnyOk,
        );
        s.push(
            "doc.position",
            json!({"doc": 1, "at": {"byte": 1}}),
            Expect::AnyOk,
        );
        s.push(
            "doc.position",
            json!({"doc": 1, "at": {"utf16": 0}}),
            Expect::AnyOk,
        );
        s.push(
            "doc.position",
            json!({"doc": 1, "at": {"line_col": {"line": 0, "col": 1}}}),
            Expect::AnyOk,
        );
        s.push(
            "doc.position",
            json!({"doc": 1, "at": {"byte": 99}}),
            rejected("out_of_range", None),
        );
        s.push(
            "doc.line",
            json!({"doc": 1, "line": 9}),
            rejected("out_of_range", None),
        );
    };
    s.push("doc.open", json!({"path": "u.txt"}), Expect::AnyOk);
    s.push(
        "doc.insert",
        json!({"doc": 1, "offset": 0, "text": "x"}),
        Expect::AnyOk,
    );
    let before = s.push("doc.info", json!({"doc": 1}), Expect::AnyOk);
    reads(&mut s);
    let after = s.push("doc.info", json!({"doc": 1}), Expect::AnyOk);
    s.push(
        "doc.insert",
        json!({"doc": 1, "offset": 1, "text": "y"}),
        Expect::AnyOk,
    );
    s.push(
        "doc.undo",
        json!({"doc": 1}),
        Expect::Result(json!({"undone": true, "len_chars": 3, "dirty": false})),
    );
    s.push(
        "doc.text",
        json!({"doc": 1}),
        Expect::Result(json!({"text": "ab\n", "len_chars": 3})),
    );
    let undone = s.push("doc.info", json!({"doc": 1}), Expect::AnyOk);
    reads(&mut s);
    let undone_after = s.push("doc.info", json!({"doc": 1}), Expect::AnyOk);
    s.push(
        "doc.redo",
        json!({"doc": 1}),
        Expect::Result(json!({"redone": true, "len_chars": 5, "dirty": true})),
    );
    s.push(
        "doc.text",
        json!({"doc": 1}),
        Expect::Result(json!({"text": "xyab\n", "len_chars": 5})),
    );
    let (r, log) = s.run(&work, &root, "s");
    let info = |id: usize| r[id - 1]["result"].clone();
    assert_eq!(info(before)["can_undo"], json!(true));
    assert_eq!(info(before)["can_redo"], json!(false));
    assert_eq!(info(after), info(before), "reads changed doc.info");
    // One undo removed both chars: nothing is left to undo, and redo is available.
    assert_eq!(info(undone)["can_undo"], json!(false));
    assert_eq!(info(undone)["can_redo"], json!(true));
    assert_eq!(info(undone_after), info(undone), "reads changed doc.info");
    assert_log_lists_successes(&log, &s);
    assert!(
        log.contains(r#""command":"doc.line""#) && log.contains(r#""command":"doc.position""#),
        "{log}"
    );
}

// ---------------------------------------------------------------- AC-5

fn schema_dump() -> Value {
    let out = kiln(&["commands", "--schema"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    serde_json::from_slice(&out.stdout).expect("schema dump is JSON")
}

fn command_entry<'a>(dump: &'a Value, name: &str) -> &'a Value {
    let found: Vec<&Value> = dump["commands"]
        .as_array()
        .expect("commands list")
        .iter()
        .filter(|c| c["name"] == json!(name))
        .collect();
    assert_eq!(found.len(), 1, "`{name}` listed once in commands --schema");
    found[0]
}

/// `true` iff `v` (an example's `files`, in whatever JSON shape) contains the string `s`.
fn contains_string(v: &Value, s: &str) -> bool {
    match v {
        Value::String(x) => x == s,
        Value::Array(a) => a.iter().any(|x| contains_string(x, s)),
        Value::Object(o) => o.values().any(|x| contains_string(x, s)),
        _ => false,
    }
}

/// AC-5 (and q1): `commands --schema` lists both commands, read-only (`mutating: false`), with
/// object params and result schemas naming the spec's keys and at least one example each, one
/// of them on the CRLF/astral worked text; every example whose setup only opens its one file
/// agrees with the naive model; `commands --audit` exits 0.
#[test]
fn schema_lists_both_commands_with_examples_and_audit_passes() {
    let dump = schema_dump();
    let mut on_worked = false;
    for (name, param_keys, result_keys) in [
        (
            "doc.line",
            &["doc", "line"][..],
            &["line", "text", "start", "end", "ending"][..],
        ),
        (
            "doc.position",
            &["doc", "at"][..],
            &["char", "byte", "utf16", "line", "col"][..],
        ),
    ] {
        let c = command_entry(&dump, name);
        assert_eq!(c["mutating"], json!(false), "{name}");
        for (schema, keys) in [
            ("params_schema", param_keys),
            ("result_schema", result_keys),
        ] {
            let props = c[schema]["properties"]
                .as_object()
                .unwrap_or_else(|| panic!("{name}.{schema} has properties: {}", c[schema]));
            for k in keys {
                assert!(props.contains_key(*k), "{name}.{schema} lacks `{k}`");
            }
        }
        let examples = c["examples"].as_array().expect("examples list");
        assert!(!examples.is_empty(), "{name} has no example");
        for ex in examples {
            on_worked |= contains_string(&ex["files"], WORKED);
            // An example on one file whose setup is just opening it: check it with the model.
            let files = ex["files"].as_array().expect("files list");
            let setup = ex["setup"].as_array().expect("setup list");
            if files.len() != 1 || setup.len() != 1 {
                continue;
            }
            let file = &files[0];
            let (path, text) = match (file.get(0), file.get(1), file.get("path")) {
                (Some(p), Some(t), _) => (p.clone(), t.as_str().map(str::to_owned)),
                _ => continue,
            };
            let Some(text) = text else { continue };
            if setup[0] != json!(["doc.open", {"path": path}]) || ex["params"]["doc"] != json!(1) {
                continue;
            }
            let m = Model::new(&text);
            let want = if name == "doc.line" {
                let l = ex["params"]["line"].as_u64().expect("line") as usize;
                m.line_result(l)
            } else {
                match m.position(&ex["params"]["at"]) {
                    Expect::Result(r) => r,
                    _ => panic!("example {ex} is not a valid position"),
                }
            };
            assert_eq!(ex["result"], want, "example of {name}: {ex}");
        }
    }
    assert!(on_worked, "no example on the text {WORKED:?}");
    let audit = kiln(&["commands", "--audit"]);
    assert_eq!(audit.status.code(), Some(0), "{audit:?}");
    let list = kiln(&["commands"]);
    assert_eq!(list.status.code(), Some(0), "{list:?}");
    let names = String::from_utf8_lossy(&list.stdout);
    assert!(
        names.contains("doc.line") && names.contains("doc.position"),
        "{names}"
    );
}

// ---------------------------------------------------------------- AC-6

const NOT_A_BOUNDARY_WHEN: &str =
    "a position inside a character, a surrogate pair or a CRLF line break";

/// The `error_kinds.rejected` entries of `dump` whose kind is `kind`.
fn kind_entries<'a>(dump: &'a Value, kind: &str) -> Vec<&'a Value> {
    dump["error_kinds"]["rejected"]
        .as_array()
        .expect("error_kinds.rejected list")
        .iter()
        .filter(|e| e["kind"] == json!(kind))
        .collect()
}

/// AC-6 (q2, q3): the generated kind list (`commands --schema` and the committed
/// `docs/commands.json`) lists `not_a_boundary` once with the pinned description, keeps
/// `out_of_range`; the committed file's entries for the two commands and the new kind equal
/// the generated ones (the `schema` step's check, restricted to what this task adds); and
/// `docs/protocol/doc-positions.md` exists and documents both commands, the validity rule's
/// refusals and both kinds. (`docs/control-protocol.md` being untouched is left to the
/// reviewer's diff check: q3 forbids pinning its bytes.)
#[test]
fn not_a_boundary_is_a_generated_kind_and_documented() {
    let dump = schema_dump();
    let file: Value =
        serde_json::from_str(&repo_file("docs/commands.json")).expect("docs/commands.json JSON");
    for (what, d) in [("commands --schema", &dump), ("docs/commands.json", &file)] {
        let e = kind_entries(d, "not_a_boundary");
        assert_eq!(e.len(), 1, "{what}: not_a_boundary listed once: {e:?}");
        assert_eq!(
            *e[0],
            json!({"kind": "not_a_boundary", "when": NOT_A_BOUNDARY_WHEN}),
            "{what}"
        );
        assert_eq!(kind_entries(d, "out_of_range").len(), 1, "{what}");
    }
    for name in ["doc.line", "doc.position"] {
        assert_eq!(
            command_entry(&file, name),
            command_entry(&dump, name),
            "docs/commands.json entry of {name} is not the generated one"
        );
    }
    let doc_path = repo_path("docs/protocol/doc-positions.md");
    assert!(doc_path.is_file(), "{} missing", doc_path.display());
    let doc = repo_file("docs/protocol/doc-positions.md");
    for needle in [
        "doc.line",
        "doc.position",
        "not_a_boundary",
        "out_of_range",
        "line_col",
        "utf16",
        "crlf",
    ] {
        assert!(
            doc.contains(needle),
            "doc-positions.md does not mention `{needle}`"
        );
    }
}
