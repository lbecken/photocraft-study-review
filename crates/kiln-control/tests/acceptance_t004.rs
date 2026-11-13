//! T-004 acceptance tests through the declared public Rust API of `kiln_control::Host`.
//! Written from the task specification alone by the test author. On the base `Host` does not
//! exist, so this file fails to compile there (`compile`, weak evidence); every criterion it
//! covers is also covered by an assertion-failing test in `apps/kiln-cli/tests/acceptance_t004.rs`.
//!
//! The crate's dependencies are not known in advance, so this file uses only `std`,
//! `kiln_control` and `kiln_commands` (whose `LogEntry` is part of the declared API); JSON is
//! checked with the small parser below.
#![cfg(test)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kiln_control::{Host, HostError};

// ---------------------------------------------------------------- minimal JSON

/// A JSON value; numbers are kept as their source text.
#[derive(Clone, Debug, PartialEq)]
enum J {
    Null,
    Bool(bool),
    Num(String),
    Str(String),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}

struct P<'a> {
    s: &'a [u8],
    i: usize,
}

impl P<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }
    fn eat(&mut self, c: u8) -> bool {
        if self.s.get(self.i) == Some(&c) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn lit(&mut self, w: &str) -> Option<()> {
        if self.s[self.i..].starts_with(w.as_bytes()) {
            self.i += w.len();
            Some(())
        } else {
            None
        }
    }
    fn string(&mut self) -> Option<String> {
        if !self.eat(b'"') {
            return None;
        }
        let mut out = String::new();
        loop {
            let c = *self.s.get(self.i)?;
            self.i += 1;
            match c {
                b'"' => return Some(out),
                b'\\' => {
                    let e = *self.s.get(self.i)?;
                    self.i += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let h = std::str::from_utf8(self.s.get(self.i..self.i + 4)?).ok()?;
                            self.i += 4;
                            let mut cp = u32::from_str_radix(h, 16).ok()?;
                            if (0xD800..0xDC00).contains(&cp) {
                                self.lit("\\u")?;
                                let h2 =
                                    std::str::from_utf8(self.s.get(self.i..self.i + 4)?).ok()?;
                                self.i += 4;
                                let lo = u32::from_str_radix(h2, 16).ok()?;
                                cp = 0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00);
                            }
                            out.push(char::from_u32(cp)?);
                        }
                        _ => return None,
                    }
                }
                c if c < 0x20 => return None,
                _ => {
                    // Copy one UTF-8 scalar.
                    let start = self.i - 1;
                    let len = match c {
                        0x00..=0x7f => 1,
                        0xc0..=0xdf => 2,
                        0xe0..=0xef => 3,
                        _ => 4,
                    };
                    let bytes = self.s.get(start..start + len)?;
                    out.push_str(std::str::from_utf8(bytes).ok()?);
                    self.i = start + len;
                }
            }
        }
    }
    fn value(&mut self) -> Option<J> {
        self.ws();
        let v = match *self.s.get(self.i)? {
            b'n' => self.lit("null").map(|_| J::Null)?,
            b't' => self.lit("true").map(|_| J::Bool(true))?,
            b'f' => self.lit("false").map(|_| J::Bool(false))?,
            b'"' => J::Str(self.string()?),
            b'[' => {
                self.i += 1;
                let mut a = Vec::new();
                self.ws();
                if !self.eat(b']') {
                    loop {
                        a.push(self.value()?);
                        self.ws();
                        if self.eat(b']') {
                            break;
                        }
                        if !self.eat(b',') {
                            return None;
                        }
                    }
                }
                J::Arr(a)
            }
            b'{' => {
                self.i += 1;
                let mut o = Vec::new();
                self.ws();
                if !self.eat(b'}') {
                    loop {
                        self.ws();
                        let k = self.string()?;
                        self.ws();
                        if !self.eat(b':') {
                            return None;
                        }
                        o.push((k, self.value()?));
                        self.ws();
                        if self.eat(b'}') {
                            break;
                        }
                        if !self.eat(b',') {
                            return None;
                        }
                    }
                }
                J::Obj(o)
            }
            _ => {
                let start = self.i;
                while self.i < self.s.len()
                    && matches!(
                        self.s[self.i],
                        b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'
                    )
                {
                    self.i += 1;
                }
                if start == self.i {
                    return None;
                }
                J::Num(std::str::from_utf8(&self.s[start..self.i]).ok()?.to_owned())
            }
        };
        Some(v)
    }
}

fn parse(line: &str) -> J {
    let mut p = P {
        s: line.as_bytes(),
        i: 0,
    };
    let v = p.value().unwrap_or_else(|| panic!("not JSON: {line}"));
    p.ws();
    assert_eq!(p.i, line.len(), "trailing data after JSON: {line}");
    v
}

/// Canonical form: keys sorted at every level, no whitespace, serde_json-style escapes.
fn canon(v: &J) -> String {
    fn esc(s: &str, out: &mut String) {
        out.push('"');
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                '\u{8}' => out.push_str("\\b"),
                '\u{c}' => out.push_str("\\f"),
                c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                c => out.push(c),
            }
        }
        out.push('"');
    }
    fn go(v: &J, out: &mut String) {
        match v {
            J::Null => out.push_str("null"),
            J::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            J::Num(n) => out.push_str(n),
            J::Str(s) => esc(s, out),
            J::Arr(a) => {
                out.push('[');
                for (i, x) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    go(x, out);
                }
                out.push(']');
            }
            J::Obj(o) => {
                let sorted: BTreeMap<&String, &J> = o.iter().map(|(k, v)| (k, v)).collect();
                out.push('{');
                for (i, (k, x)) in sorted.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    esc(k, out);
                    out.push(':');
                    go(x, out);
                }
                out.push('}');
            }
        }
    }
    let mut s = String::new();
    go(v, &mut s);
    s
}

fn get<'a>(v: &'a J, key: &str) -> Option<&'a J> {
    match v {
        J::Obj(o) => o.iter().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}

fn ok_of(line: &str) -> bool {
    match get(&parse(line), "ok") {
        Some(J::Bool(b)) => *b,
        other => panic!("`ok` is not a bool ({other:?}): {line}"),
    }
}

fn id_of(line: &str) -> J {
    get(&parse(line), "id")
        .cloned()
        .unwrap_or_else(|| panic!("no `id` key: {line}"))
}

fn error_code(line: &str) -> String {
    match get(&parse(line), "error").and_then(|e| get(e, "code")) {
        Some(J::Str(s)) => s.clone(),
        other => panic!("no error.code ({other:?}): {line}"),
    }
}

fn num(n: &str) -> J {
    J::Num(n.to_owned())
}
fn s(x: &str) -> J {
    J::Str(x.to_owned())
}

// ---------------------------------------------------------------- fixtures

fn fresh_dir(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("acceptance_t004_control")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn disk_host(name: &str, files: &[(&str, &[u8])]) -> (Host, PathBuf) {
    let root = fresh_dir(name);
    for (p, b) in files {
        let full = root.join(p);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, b).unwrap();
    }
    (Host::on_disk(&root).expect("on_disk host"), root)
}

fn feed(host: &mut Host, lines: &[&[u8]]) -> Vec<String> {
    lines.iter().filter_map(|l| host.handle_line(l)).collect()
}

const AC1: [&[u8]; 5] = [
    br#"{"id":1,"command":"doc.open","params":{"path":"a.txt"}}"#,
    br#"{"id":2,"command":"doc.insert","params":{"doc":1,"offset":5,"text":" world"}}"#,
    br#"{"id":3,"command":"doc.undo","params":{"doc":1}}"#,
    br#"{"id":4,"command":"doc.redo","params":{"doc":1}}"#,
    br#"{"id":5,"command":"doc.save","params":{"doc":1}}"#,
];

// ---------------------------------------------------------------- AC-9, AC-1

#[test]
fn memory_and_disk_hosts_give_same_responses() {
    // The AC-9 wording, literally: a.txt = "x" (so the insert at offset 5 fails on both).
    let mut mem = Host::in_memory(&[("a.txt", b"x")]).expect("in_memory host");
    let (mut disk, root) = disk_host("ac9-x", &[("a.txt", b"x")]);
    let m = feed(&mut mem, &AC1);
    let d = feed(&mut disk, &AC1);
    assert_eq!(m.len(), 5, "{m:?}");
    assert_eq!(m, d, "memory and disk hosts differ");
    assert!(ok_of(&m[0]) && !ok_of(&m[1]), "{m:?}");
    assert_eq!(std::fs::read(root.join("a.txt")).unwrap(), b"x");

    // The AC-1 file, where every request succeeds.
    let mut mem = Host::in_memory(&[("a.txt", b"hello\n")]).expect("in_memory host");
    let (mut disk, root) = disk_host("ac9-hello", &[("a.txt", b"hello\n")]);
    let m = feed(&mut mem, &AC1);
    let d = feed(&mut disk, &AC1);
    assert_eq!(m, d, "memory and disk hosts differ");
    assert_eq!(m.len(), 5);
    assert!(m.iter().all(|l| ok_of(l)), "{m:?}");
    assert_eq!(
        m[4],
        r#"{"id":5,"ok":true,"result":{"bytes_written":12,"dirty":false,"doc":1}}"#
    );
    assert!(m.iter().all(|l| !l.ends_with('\n')), "no trailing newline");
    assert_eq!(std::fs::read(root.join("a.txt")).unwrap(), b"hello world\n");
    // The memory host's file changed too: a second open of the same path is already open,
    // so check its text instead.
    let t = mem
        .handle_line(br#"{"id":"t","command":"doc.text","params":{"doc":1}}"#)
        .unwrap();
    assert_eq!(
        t,
        r#"{"id":"t","ok":true,"result":{"len_chars":12,"text":"hello world\n"}}"#
    );
}

#[test]
fn host_constructors_reject_bad_roots() {
    let d = fresh_dir("bad-root");
    let file = d.join("f.txt");
    std::fs::write(&file, b"x").unwrap();
    let e: Result<Host, HostError> = Host::on_disk(&file);
    let e = e.err().expect("a file is not a root");
    assert!(!e.0.is_empty());
    assert!(!format!("{e}").is_empty());
    let _: &dyn std::error::Error = &e;
    assert!(Host::on_disk(&d.join("missing")).is_err());
    assert!(Host::on_disk(&d).is_ok());
    assert!(Host::in_memory(&[]).is_ok());
}

// ---------------------------------------------------------------- AC-2

#[test]
fn one_response_per_request_with_ids_echoed() {
    let mut h = Host::in_memory(&[("a.txt", b"abc")]).unwrap();
    let script: Vec<(&[u8], Option<J>)> = vec![
        (b"", None),
        (
            br#"{"id":"a","command":"doc.open","params":{"path":"a.txt"}}"#,
            Some(s("a")),
        ),
        (b"   \t  ", None),
        (b"\r", None),
        (br#"{"command":"doc.list"}"#, Some(J::Null)),
        (br#"{"id":42,"command":"no.such"}"#, Some(num("42"))),
        (br#"{"id":-1,"command":"doc.list"}"#, Some(num("-1"))),
        (br#"{"id":"","command":"doc.list"}"#, Some(s(""))),
        (br#"{"id":"q\"\n","command":"doc.list"}"#, Some(s("q\"\n"))),
        (b"\n", None),
        (
            br#"{"command":"doc.insert","params":{"doc":1,"offset":0,"text":"z"}}"#,
            Some(J::Null),
        ),
    ];
    let mut got = Vec::new();
    for (line, want) in &script {
        let r = h.handle_line(line);
        match want {
            None => assert_eq!(r, None, "blank line {line:?} produces nothing"),
            Some(id) => {
                let r = r.unwrap_or_else(|| panic!("no response to {line:?}"));
                assert_eq!(&id_of(&r), id, "{r}");
                got.push(r);
            }
        }
    }
    assert_eq!(got.len(), 8);
    assert!(!ok_of(&got[2]) && ok_of(&got[7]), "{got:?}");
}

// ---------------------------------------------------------------- AC-5

#[test]
fn malformed_requests_get_structured_errors_and_host_continues() {
    let mut h = Host::in_memory(&[("a.txt", b"hello\n")]).unwrap();
    let open = h
        .handle_line(br#"{"id":"o","command":"doc.open","params":{"path":"a.txt"}}"#)
        .unwrap();
    assert!(ok_of(&open), "{open}");
    let cases: Vec<(&[u8], &str, J)> = vec![
        (b"not json", "invalid_json", J::Null),
        (b"[1,2]", "invalid_request", J::Null),
        (br#"{"id":7}"#, "invalid_request", num("7")),
        (
            br#"{"id":1.5,"command":"doc.list"}"#,
            "invalid_request",
            J::Null,
        ),
        (
            br#"{"command":"doc.list","params":[]}"#,
            "invalid_request",
            J::Null,
        ),
        (
            br#"{"command":"doc.list","extra":1}"#,
            "invalid_request",
            J::Null,
        ),
        (br#"{"command":"no.such"}"#, "unknown_command", J::Null),
        (
            br#"{"command":"doc.insert","params":{"doc":1}}"#,
            "invalid_params",
            J::Null,
        ),
        (b"\xff\xfe\xfd", "invalid_json", J::Null),
        (
            b"{\"id\":\"\xc3\x28\",\"command\":\"doc.list\"}",
            "invalid_json",
            J::Null,
        ),
        (b"42", "invalid_request", J::Null),
        (b"null", "invalid_request", J::Null),
        (br#"{"id":"k","command":null}"#, "invalid_request", s("k")),
        (
            br#"{"id":{"a":1},"command":"doc.list"}"#,
            "invalid_request",
            J::Null,
        ),
        (
            br#"{"id":3,"command":"doc.insert","params":{"doc":1,"offset":100,"text":"x"}}"#,
            "rejected",
            num("3"),
        ),
    ];
    for (bad, code, id) in &cases {
        let what = String::from_utf8_lossy(bad);
        let r = h
            .handle_line(bad)
            .unwrap_or_else(|| panic!("no response to {what}"));
        assert!(!ok_of(&r), "{what}: {r}");
        assert_eq!(error_code(&r), *code, "{what}: {r}");
        assert_eq!(&id_of(&r), id, "{what}: {r}");
        assert!(
            matches!(
                get(get(&parse(&r), "error").unwrap(), "message"),
                Some(J::Str(_))
            ),
            "{what}: {r}"
        );
        let next = h
            .handle_line(br#"{"id":"n","command":"doc.text","params":{"doc":1}}"#)
            .unwrap();
        assert_eq!(
            next, r#"{"id":"n","ok":true,"result":{"len_chars":6,"text":"hello\n"}}"#,
            "after {what}"
        );
    }
    // Hostile input does not panic and leaves the host usable.
    for bad in [
        "[".repeat(200_000).into_bytes(),
        "{\"a\":".repeat(50_000).into_bytes(),
        vec![0u8; 1000],
        format!(r#"{{"command":"{}"}}"#, "y".repeat(1 << 20)).into_bytes(),
    ] {
        let r = h.handle_line(&bad).expect("a response");
        assert!(!ok_of(&r));
    }
    // Only successful requests are logged: the open plus every doc.text.
    assert_eq!(h.log().len(), 1 + cases.len());
    assert_eq!(h.log()[0].name, "doc.open");
}

// ---------------------------------------------------------------- AC-4 (library side)

#[test]
fn log_script_lists_successful_requests_with_1_based_ids() {
    let mut h = Host::in_memory(&[("a.txt", b"ab")]).unwrap();
    for l in [
        &br#"{"id":"x","command":"doc.open","params":{"path":"a.txt"}}"#[..],
        b"garbage",
        br#"{"command":"doc.insert","params":{"text":"Z","offset":1,"doc":1}}"#,
        br#"{"command":"doc.insert","params":{"text":"Z","offset":9,"doc":1}}"#,
        br#"{"id":99,"command":"doc.list"}"#,
    ] {
        h.handle_line(l);
    }
    assert_eq!(h.log().len(), 3);
    assert_eq!(
        h.log_script(),
        concat!(
            r#"{"command":"doc.open","id":1,"params":{"path":"a.txt"}}"#,
            "\n",
            r#"{"command":"doc.insert","id":2,"params":{"doc":1,"offset":1,"text":"Z"}}"#,
            "\n",
            r#"{"command":"doc.list","id":3,"params":{}}"#,
            "\n",
        )
    );
    // Replaying the script in a fresh host reproduces the same log script.
    let script = h.log_script();
    let mut h2 = Host::in_memory(&[("a.txt", b"ab")]).unwrap();
    for l in script.lines() {
        assert!(ok_of(&h2.handle_line(l.as_bytes()).unwrap()));
    }
    assert_eq!(h2.log_script(), script);
    assert_eq!(Host::in_memory(&[]).unwrap().log_script(), "");
}

// ---------------------------------------------------------------- AC-7

#[test]
fn responses_are_canonical_json() {
    let (mut h, root) = disk_host(
        "ac7",
        &[
            ("a.txt", b"tab\there \"q\" \\ \x01 \xc3\xa9\n"),
            ("sub/b.rs", b"fn b(){}\r\n"),
        ],
    );
    let lines: Vec<&[u8]> = vec![
        br#"{"id":"o","command":"doc.open","params":{"path":"a.txt"}}"#,
        br#"{"command":"doc.text","params":{"doc":1}}"#,
        br#"{"command":"doc.open","params":{"path":"sub/b.rs"}}"#,
        br#"{"command":"doc.info","params":{"doc":2}}"#,
        br#"{"command":"doc.list"}"#,
        br#"{"command":"doc.open","params":{"path":"nope.txt"}}"#,
        br#"{"command":"doc.open","params":{"path":"../up.txt"}}"#,
        br#"{"command":"doc.open","params":{"path":"/abs.txt"}}"#,
        br#"{"command":"doc.insert","params":{"doc":1,"offset":1000,"text":"x"}}"#,
        br#"{"command":"doc.insert","params":{"doc":1}}"#,
        br#"{"command":"no.such"}"#,
        b"not json",
        b"[1]",
        br#"{"command":"doc.save","params":{"doc":1}}"#,
    ];
    let out = feed(&mut h, &lines);
    assert_eq!(out.len(), lines.len());
    let root_s = root.to_string_lossy().into_owned();
    let canon_root = root.canonicalize().unwrap().to_string_lossy().into_owned();
    for l in &out {
        assert_eq!(&canon(&parse(l)), l, "not canonical");
        assert!(!l.contains('\n'));
        assert!(
            !l.contains(&root_s) && !l.contains(&canon_root),
            "host path leaked: {l}"
        );
    }
    assert_eq!(
        out[1],
        r#"{"id":null,"ok":true,"result":{"len_chars":19,"text":"tab\there \"q\" \\ \u0001 é\n"}}"#
    );
}
