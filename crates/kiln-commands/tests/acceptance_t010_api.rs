//! T-010 acceptance tests through the public API of `kiln-commands`: `doc.line` and
//! `doc.position` executed by name with [`Registry::execute`], the session log
//! ([`Context::log`], ruling q1 of D-059: `mutating: false`), the `rejected` reasons and the
//! reason table (`RejectReason::ALL`, `describe()`'s `error_kinds`). Written from the task
//! specification alone by the test author.
//!
//! Uses no new API (the new reason is looked up by its kind string), so the file compiles on
//! base and fails there by assertion (`unknown command`, no `not_a_boundary` in `ALL`).
//!
//! Oracle: a naive model of each small text, checked exhaustively: every char, byte and UTF-16
//! offset up to one past the end, every line up to `len_lines`, every column up to one past
//! the line's end.
#![cfg(test)]

use kiln_commands::{CommandError, Context, Registry, RejectReason, standard_registry};
use serde_json::{Value, json};

fn reg() -> Registry {
    standard_registry().expect("standard registry builds")
}

/// A context with `t` as file `t.txt`, opened as doc 1.
fn open(r: &Registry, t: &str) -> Context {
    let mut ctx = Context::with_files(&[("t.txt", t.as_bytes())]).expect("fixture");
    r.execute(&mut ctx, "doc.open", json!({"path": "t.txt"}))
        .expect("doc.open");
    ctx
}

/// The texts checked exhaustively: empty, no final newline, CR-only, `\r\r\n`, `\n\r`, empty
/// lines, 2-, 3- and 4-byte chars, astral chars next to breaks, the worked example.
const TEXTS: &[&str] = &[
    "",
    "a",
    "\n",
    "\r",
    "\r\n",
    "\r\r\n",
    "\n\r",
    "\r\n\r\n",
    "\n\n",
    "é\n\n",
    "x\r\n",
    "中😀",
    "😀\r\n😀",
    "a\rb\r\nc\nd",
    "😀\r",
    "\r\n😀",
    "a\r\nbé😀\rc\n",
];

/// What the spec says one request must give.
#[derive(Debug, PartialEq)]
enum Want {
    Ok(Value),
    Rejected(&'static str, String),
}

struct Model {
    chars: Vec<char>,
    /// (start, end, ending) per line.
    lines: Vec<(usize, usize, &'static str)>,
    byte_of: Vec<usize>,
    utf16_of: Vec<usize>,
}

impl Model {
    fn new(t: &str) -> Model {
        let chars: Vec<char> = t.chars().collect();
        let mut lines = Vec::new();
        let (mut start, mut i) = (0, 0);
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
            lines.push((start, i, ending));
            i += brk;
            start = i;
        }
        lines.push((start, chars.len(), "none"));
        let (mut byte_of, mut utf16_of) = (vec![0], vec![0]);
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

    fn mid_crlf(&self, c: usize) -> bool {
        c > 0 && c < self.chars.len() && self.chars[c - 1] == '\r' && self.chars[c] == '\n'
    }

    fn five(&self, c: usize) -> Want {
        if self.mid_crlf(c) {
            return Want::Rejected(
                "not_a_boundary",
                format!("invalid position: char index {c} is between the \\r and \\n of a CRLF"),
            );
        }
        let (line, (start, _, _)) = self
            .lines
            .iter()
            .enumerate()
            .find(|(_, (s, e, _))| *s <= c && c <= *e)
            .expect("on a line");
        Want::Ok(
            json!({"char": c, "byte": self.byte_of[c], "utf16": self.utf16_of[c],
            "line": line, "col": c - start}),
        )
    }

    fn oob(i: usize, len: usize) -> Want {
        Want::Rejected(
            "out_of_range",
            format!("invalid position: index {i} is out of bounds (length {len})"),
        )
    }

    fn from_char(&self, c: usize) -> Want {
        let n = self.chars.len();
        if c > n {
            return Self::oob(c, n);
        }
        self.five(c)
    }

    fn from_byte(&self, b: usize) -> Want {
        let n = *self.byte_of.last().unwrap();
        if b > n {
            return Self::oob(b, n);
        }
        match self.byte_of.iter().position(|&x| x == b) {
            Some(c) => self.five(c),
            None => Want::Rejected(
                "not_a_boundary",
                format!("invalid position: byte offset {b} is inside a multi-byte char"),
            ),
        }
    }

    fn from_utf16(&self, u: usize) -> Want {
        let n = *self.utf16_of.last().unwrap();
        if u > n {
            return Self::oob(u, n);
        }
        match self.utf16_of.iter().position(|&x| x == u) {
            Some(c) => self.five(c),
            None => Want::Rejected(
                "not_a_boundary",
                format!("invalid position: UTF-16 offset {u} is inside a surrogate pair"),
            ),
        }
    }

    fn line_oob(&self, prefix: &str, line: usize) -> Want {
        Want::Rejected(
            "out_of_range",
            format!(
                "{prefix}: line {line} is out of bounds (buffer has {} lines)",
                self.lines.len()
            ),
        )
    }

    fn from_line_col(&self, line: usize, col: usize) -> Want {
        let Some(&(start, end, _)) = self.lines.get(line) else {
            return self.line_oob("invalid position", line);
        };
        if col > end - start {
            return Want::Rejected(
                "out_of_range",
                format!(
                    "invalid position: column {col} is out of bounds for line {line} ({} chars)",
                    end - start
                ),
            );
        }
        self.five(start + col)
    }

    fn line(&self, line: usize) -> Want {
        let Some(&(start, end, ending)) = self.lines.get(line) else {
            return self.line_oob("invalid line", line);
        };
        let text: String = self.chars[start..end].iter().collect();
        Want::Ok(json!({"line": line, "text": text, "start": start, "end": end, "ending": ending}))
    }
}

/// Execute and compare with `want`; a success must be appended to the log with
/// `mutating: false` and params as given, a refusal must leave the log unchanged.
fn check(r: &Registry, ctx: &mut Context, name: &str, params: Value, want: &Want) {
    let before = ctx.log().len();
    let got = r.execute(ctx, name, params.clone());
    match (want, got) {
        (Want::Ok(w), Ok(v)) => {
            assert_eq!(&v, w, "{name} {params}");
            assert_eq!(ctx.log().len(), before + 1, "{name} {params} not logged");
            let e = ctx.log().last().unwrap();
            assert_eq!(e.name, name);
            assert_eq!(e.params, params);
            assert!(!e.mutating, "{name} logged as mutating");
        }
        (Want::Rejected(kind, msg), Err(CommandError::Rejected { reason, message })) => {
            assert_eq!(reason.as_str(), *kind, "{name} {params}: {message}");
            assert_eq!(&message, msg, "{name} {params}");
            assert_eq!(
                ctx.log().len(),
                before,
                "refused {name} {params} was logged"
            );
        }
        (w, g) => panic!("{name} {params}: want {w:?}, got {g:?}"),
    }
}

/// Every offset, line and column of every small text, in every form, against the model;
/// afterwards the text, `doc.info` and the history are unchanged.
#[test]
fn exhaustive_lines_and_positions_on_small_texts() {
    let r = reg();
    for t in TEXTS {
        let m = Model::new(t);
        let mut ctx = open(&r, t);
        let info = r.execute(&mut ctx, "doc.info", json!({"doc": 1})).unwrap();
        assert_eq!(info["len_lines"], json!(m.lines.len()), "{t:?}");
        let pos = |at: Value| json!({"doc": 1, "at": at});
        for l in 0..=m.lines.len() {
            check(
                &r,
                &mut ctx,
                "doc.line",
                json!({"doc": 1, "line": l}),
                &m.line(l),
            );
        }
        for c in 0..=m.chars.len() + 1 {
            check(
                &r,
                &mut ctx,
                "doc.position",
                pos(json!({"char": c})),
                &m.from_char(c),
            );
        }
        for b in 0..=*m.byte_of.last().unwrap() + 1 {
            check(
                &r,
                &mut ctx,
                "doc.position",
                pos(json!({"byte": b})),
                &m.from_byte(b),
            );
        }
        for u in 0..=*m.utf16_of.last().unwrap() + 1 {
            check(
                &r,
                &mut ctx,
                "doc.position",
                pos(json!({"utf16": u})),
                &m.from_utf16(u),
            );
        }
        for line in 0..=m.lines.len() {
            let width = m.lines.get(line).map_or(0, |(s, e, _)| e - s);
            for col in 0..=width + 1 {
                check(
                    &r,
                    &mut ctx,
                    "doc.position",
                    pos(json!({"line_col": {"line": line, "col": col}})),
                    &m.from_line_col(line, col),
                );
            }
        }
        let text = r.execute(&mut ctx, "doc.text", json!({"doc": 1})).unwrap();
        assert_eq!(text["text"], json!(t));
        let after = r.execute(&mut ctx, "doc.info", json!({"doc": 1})).unwrap();
        assert_eq!(after, info, "{t:?}");
    }
}

/// Round trip: for every valid char offset, each returned form converts back to the same five
/// numbers (conversions are exact inverses on valid positions).
#[test]
fn every_returned_form_converts_back_to_the_same_position() {
    let r = reg();
    for t in TEXTS {
        let m = Model::new(t);
        let mut ctx = open(&r, t);
        for c in 0..=m.chars.len() {
            if m.mid_crlf(c) {
                continue;
            }
            let five = r
                .execute(
                    &mut ctx,
                    "doc.position",
                    json!({"doc": 1, "at": {"char": c}}),
                )
                .unwrap_or_else(|e| panic!("{t:?} char {c}: {e:?}"));
            for at in [
                json!({"char": five["char"]}),
                json!({"byte": five["byte"]}),
                json!({"utf16": five["utf16"]}),
                json!({"line_col": {"line": five["line"], "col": five["col"]}}),
            ] {
                let back = r
                    .execute(&mut ctx, "doc.position", json!({"doc": 1, "at": at}))
                    .unwrap_or_else(|e| panic!("{t:?} {at}: {e:?}"));
                assert_eq!(back, five, "{t:?} {at}");
            }
        }
    }
}

/// Ids (q4: `doc: 0` included), closed documents, schema mismatches and precedence through the
/// API: `invalid_params` → `no_such_document` → the position or line check.
#[test]
fn ids_params_and_precedence() {
    let r = reg();
    let mut ctx =
        Context::with_files(&[("a.txt", b"a\r\nb".as_slice()), ("b.txt", b"x")]).expect("fixture");
    r.execute(&mut ctx, "doc.open", json!({"path": "a.txt"}))
        .unwrap();
    r.execute(&mut ctx, "doc.open", json!({"path": "b.txt"}))
        .unwrap();
    r.execute(&mut ctx, "doc.close", json!({"doc": 2})).unwrap();
    let logged = ctx.log().len();
    let no_doc = |e: CommandError| match e {
        CommandError::Rejected { reason, .. } => reason == RejectReason::NoSuchDocument,
        _ => false,
    };
    for doc in [0u64, 2, 3, u64::MAX] {
        for (name, params) in [
            ("doc.line", json!({"doc": doc, "line": 0})),
            ("doc.line", json!({"doc": doc, "line": u64::MAX})),
            ("doc.position", json!({"doc": doc, "at": {"char": 0}})),
            ("doc.position", json!({"doc": doc, "at": {"char": 2}})),
            (
                "doc.position",
                json!({"doc": doc, "at": {"byte": u64::MAX}}),
            ),
        ] {
            let e = r.execute(&mut ctx, name, params.clone()).unwrap_err();
            assert!(no_doc(e.clone()), "{name} {params}: {e:?}");
        }
    }
    for (name, params) in [
        ("doc.line", json!({"doc": 9, "line": -1})),
        ("doc.line", json!({"doc": 1, "line": 0, "extra": true})),
        ("doc.line", json!({"doc": 1})),
        (
            "doc.position",
            json!({"doc": 9, "at": {"char": 0, "utf16": 0}}),
        ),
        (
            "doc.position",
            json!({"doc": 1, "at": {"char": 0, "utf16": 0}}),
        ),
        (
            "doc.position",
            json!({"doc": 1, "at": {"line_col": {"col": 0}}}),
        ),
        ("doc.position", json!({"doc": 1, "at": {"byte": -3}})),
        ("doc.position", json!({"doc": 1, "at": {}})),
        (
            "doc.position",
            json!({"doc": 1, "at": {"char": 0}, "line": 0}),
        ),
    ] {
        let e = r.execute(&mut ctx, name, params.clone()).unwrap_err();
        assert!(
            matches!(e, CommandError::InvalidParams { .. }),
            "{name} {params}: {e:?}"
        );
    }
    // Hostile u64 on an open document: out_of_range, no panic.
    for (name, params) in [
        ("doc.line", json!({"doc": 1, "line": u64::MAX})),
        ("doc.position", json!({"doc": 1, "at": {"char": u64::MAX}})),
        ("doc.position", json!({"doc": 1, "at": {"utf16": u64::MAX}})),
        (
            "doc.position",
            json!({"doc": 1, "at": {"line_col": {"line": 0, "col": u64::MAX}}}),
        ),
    ] {
        match r.execute(&mut ctx, name, params.clone()) {
            Err(CommandError::Rejected { reason, .. }) => {
                assert_eq!(reason, RejectReason::OutOfRange, "{name} {params}")
            }
            other => panic!("{name} {params}: {other:?}"),
        }
    }
    assert_eq!(ctx.log().len(), logged, "a refused request was logged");
}

/// The reason table gains `not_a_boundary` with the pinned description, published in
/// `describe()`'s `error_kinds.rejected`; `out_of_range` keeps its row (q2).
#[test]
fn not_a_boundary_reason_is_declared_and_published() {
    let found: Vec<&RejectReason> = RejectReason::ALL
        .iter()
        .filter(|r| r.as_str() == "not_a_boundary")
        .collect();
    assert_eq!(found.len(), 1, "RejectReason::ALL has no not_a_boundary");
    let r = *found[0];
    assert_eq!(
        r.description(),
        "a position inside a character, a surrogate pair or a CRLF line break"
    );
    let de: RejectReason = serde_json::from_value(json!("not_a_boundary")).expect("serde form");
    assert_eq!(de, r);
    assert_ne!(r, RejectReason::OutOfRange);
    let d = reg().describe();
    let rejected = d["error_kinds"]["rejected"].as_array().expect("rejected");
    assert!(
        rejected.contains(&json!({"kind": "not_a_boundary", "when": r.description()})),
        "{rejected:?}"
    );
    assert!(rejected.iter().any(|e| e["kind"] == json!("out_of_range")));
    for name in ["doc.line", "doc.position"] {
        let c = d["commands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == json!(name))
            .unwrap_or_else(|| panic!("{name} not described"));
        assert_eq!(c["mutating"], json!(false), "{name}");
    }
}
