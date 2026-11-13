//! T-010 implementer tests: `doc.line` and `doc.position` through the registry, against a
//! naive model (split on `\r\n|\r|\n`; UTF-8 and UTF-16 lengths of prefixes).
#![cfg(test)]

use kiln_commands::{CommandError, Context, Registry, RejectReason, standard_registry};
use serde_json::{Value, json};

const T: &str = "a\r\nbé😀\rc\n";

fn reg() -> Registry {
    standard_registry().expect("standard registry builds")
}

fn ctx_with(text: &str) -> Context {
    let mut ctx = Context::with_files(&[("a.txt", text.as_bytes())]).expect("ctx");
    let r = reg();
    assert_eq!(
        ok(&r, &mut ctx, "doc.open", json!({"path": "a.txt"}))["doc"],
        1
    );
    ctx
}

fn ok(r: &Registry, ctx: &mut Context, name: &str, params: Value) -> Value {
    r.execute(ctx, name, params.clone())
        .unwrap_or_else(|e| panic!("{name} {params}: {e:?}"))
}

fn err(r: &Registry, ctx: &mut Context, name: &str, params: Value) -> CommandError {
    r.execute(ctx, name, params.clone())
        .err()
        .unwrap_or_else(|| panic!("{name} {params}: expected an error"))
}

/// (text, start, end, ending) of every line, by a naive split.
fn naive_lines(t: &str) -> Vec<(String, usize, usize, &'static str)> {
    let chars: Vec<char> = t.chars().collect();
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < chars.len() {
        let (len, name) = match chars[i] {
            '\r' if chars.get(i + 1) == Some(&'\n') => (2, "crlf"),
            '\r' => (1, "cr"),
            '\n' => (1, "lf"),
            _ => {
                i += 1;
                continue;
            }
        };
        out.push((chars[start..i].iter().collect(), start, i, name));
        i += len;
        start = i;
    }
    out.push((chars[start..].iter().collect(), start, chars.len(), "none"));
    out
}

/// The five numbers of char offset `c`, or `None` if `c` is inside a CRLF.
fn naive_position(t: &str, c: usize) -> Option<Value> {
    let chars: Vec<char> = t.chars().collect();
    if c > 0 && c < chars.len() && chars[c - 1] == '\r' && chars[c] == '\n' {
        return None;
    }
    let prefix: String = chars[..c].iter().collect();
    let (line, start) = naive_lines(t)
        .iter()
        .enumerate()
        .filter(|(_, l)| l.1 <= c)
        .map(|(i, l)| (i, l.1))
        .last()
        .unwrap();
    Some(json!({
        "char": c,
        "byte": prefix.len(),
        "utf16": prefix.encode_utf16().count(),
        "line": line,
        "col": c - start,
    }))
}

/// A small deterministic generator (xorshift), so the texts are fixed.
fn texts() -> Vec<String> {
    let alphabet = ["a", "é", "😀", "中", "\n", "\r\n", "\r"];
    let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut out = vec![String::new(), "\r".into(), "\r\n".into(), T.into()];
    for _ in 0..60 {
        let mut t = String::new();
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        for _ in 0..(s % 24) {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            t.push_str(alphabet[(s % alphabet.len() as u64) as usize]);
        }
        out.push(t);
    }
    out
}

fn refusal(e: &CommandError) -> (RejectReason, String) {
    match e {
        CommandError::Rejected { reason, message } => (*reason, message.clone()),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn worked_example_lines_and_positions() {
    let r = reg();
    let mut ctx = ctx_with(T);
    let want = [
        json!({"line": 0, "text": "a", "start": 0, "end": 1, "ending": "crlf"}),
        json!({"line": 1, "text": "bé😀", "start": 3, "end": 6, "ending": "cr"}),
        json!({"line": 2, "text": "c", "start": 7, "end": 8, "ending": "lf"}),
        json!({"line": 3, "text": "", "start": 9, "end": 9, "ending": "none"}),
    ];
    for (i, w) in want.iter().enumerate() {
        assert_eq!(
            &ok(&r, &mut ctx, "doc.line", json!({"doc": 1, "line": i})),
            w
        );
    }
    let pos = [
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
    for (c, b, u, l, col) in pos {
        let w = json!({"char": c, "byte": b, "utf16": u, "line": l, "col": col});
        for at in [
            json!({"char": c}),
            json!({"byte": b}),
            json!({"utf16": u}),
            json!({"line_col": {"line": l, "col": col}}),
        ] {
            assert_eq!(
                ok(&r, &mut ctx, "doc.position", json!({"doc": 1, "at": at})),
                w,
                "{at}"
            );
        }
    }
}

#[test]
fn lines_and_positions_match_the_naive_model_and_round_trip() {
    let r = reg();
    for t in texts() {
        let mut ctx = ctx_with(&t);
        let lines = naive_lines(&t);
        let info = ok(&r, &mut ctx, "doc.info", json!({"doc": 1}));
        assert_eq!(info["len_lines"], lines.len(), "{t:?}");
        for (i, (text, start, end, ending)) in lines.iter().enumerate() {
            assert_eq!(
                ok(&r, &mut ctx, "doc.line", json!({"doc": 1, "line": i})),
                json!({"line": i, "text": text, "start": start, "end": end, "ending": ending}),
                "{t:?} line {i}"
            );
        }
        for c in 0..=t.chars().count() {
            let got = r.execute(
                &mut ctx,
                "doc.position",
                json!({"doc": 1, "at": {"char": c}}),
            );
            match naive_position(&t, c) {
                None => {
                    let (reason, _) = refusal(&got.unwrap_err());
                    assert_eq!(reason, RejectReason::NotABoundary, "{t:?} {c}");
                }
                Some(w) => {
                    assert_eq!(got.expect("valid"), w, "{t:?} {c}");
                    for at in [
                        json!({"byte": w["byte"]}),
                        json!({"utf16": w["utf16"]}),
                        json!({"line_col": {"line": w["line"], "col": w["col"]}}),
                    ] {
                        assert_eq!(
                            ok(&r, &mut ctx, "doc.position", json!({"doc": 1, "at": at})),
                            w,
                            "{t:?} {at}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn the_six_refusals_and_their_kinds() {
    let r = reg();
    let mut ctx = ctx_with(T);
    let before = ok(&r, &mut ctx, "doc.info", json!({"doc": 1}));
    let log_len = ctx.log().len();
    let cases = [
        (
            "doc.position",
            json!({"doc": 1, "at": {"byte": 14}}),
            RejectReason::OutOfRange,
            "invalid position: index 14 is out of bounds (length 13)",
        ),
        (
            "doc.line",
            json!({"doc": 1, "line": 4}),
            RejectReason::OutOfRange,
            "invalid line: line 4 is out of bounds (buffer has 4 lines)",
        ),
        (
            "doc.position",
            json!({"doc": 1, "at": {"line_col": {"line": 1, "col": 4}}}),
            RejectReason::OutOfRange,
            "invalid position: column 4 is out of bounds for line 1 (3 chars)",
        ),
        (
            "doc.position",
            json!({"doc": 1, "at": {"byte": 5}}),
            RejectReason::NotABoundary,
            "invalid position: byte offset 5 is inside a multi-byte char",
        ),
        (
            "doc.position",
            json!({"doc": 1, "at": {"byte": 7}}),
            RejectReason::NotABoundary,
            "invalid position: byte offset 7 is inside a multi-byte char",
        ),
        (
            "doc.position",
            json!({"doc": 1, "at": {"utf16": 6}}),
            RejectReason::NotABoundary,
            "invalid position: UTF-16 offset 6 is inside a surrogate pair",
        ),
        (
            "doc.position",
            json!({"doc": 1, "at": {"char": 2}}),
            RejectReason::NotABoundary,
            "invalid position: char index 2 is between the \\r and \\n of a CRLF",
        ),
        (
            "doc.position",
            json!({"doc": 1, "at": {"byte": 2}}),
            RejectReason::NotABoundary,
            "invalid position: char index 2 is between the \\r and \\n of a CRLF",
        ),
        (
            "doc.position",
            json!({"doc": 1, "at": {"char": 10}}),
            RejectReason::OutOfRange,
            "invalid position: index 10 is out of bounds (length 9)",
        ),
        (
            "doc.position",
            json!({"doc": 1, "at": {"utf16": 11}}),
            RejectReason::OutOfRange,
            "invalid position: index 11 is out of bounds (length 10)",
        ),
        (
            "doc.position",
            json!({"doc": 1, "at": {"line_col": {"line": 4, "col": 0}}}),
            RejectReason::OutOfRange,
            "invalid position: line 4 is out of bounds (buffer has 4 lines)",
        ),
    ];
    for (name, params, reason, message) in cases {
        let e = err(&r, &mut ctx, name, params.clone());
        assert_eq!(refusal(&e), (reason, message.to_owned()), "{name} {params}");
        let j = e.to_json();
        assert_eq!(j["code"], "rejected", "{j}");
        assert_eq!(j["message"], message, "{j}");
    }
    assert_eq!(ctx.log().len(), log_len);
    assert_eq!(ok(&r, &mut ctx, "doc.info", json!({"doc": 1})), before);
    assert_eq!(ok(&r, &mut ctx, "doc.text", json!({"doc": 1}))["text"], T);
}

#[test]
fn huge_integers_are_out_of_range_not_a_panic() {
    let r = reg();
    let mut ctx = ctx_with(T);
    let m = u64::MAX;
    for (name, params) in [
        ("doc.line", json!({"doc": 1, "line": m})),
        ("doc.position", json!({"doc": 1, "at": {"char": m}})),
        ("doc.position", json!({"doc": 1, "at": {"byte": m}})),
        ("doc.position", json!({"doc": 1, "at": {"utf16": m}})),
        (
            "doc.position",
            json!({"doc": 1, "at": {"line_col": {"line": m, "col": m}}}),
        ),
        (
            "doc.position",
            json!({"doc": 1, "at": {"line_col": {"line": 0, "col": m}}}),
        ),
    ] {
        let (reason, msg) = refusal(&err(&r, &mut ctx, name, params.clone()));
        assert_eq!(reason, RejectReason::OutOfRange, "{name} {params}");
        assert!(msg.contains(&m.to_string()), "{msg}");
    }
}

#[test]
fn precedence_params_then_document_then_position() {
    let r = reg();
    let mut ctx = ctx_with(T);
    let code = |e: CommandError| e.to_json()["code"].as_str().unwrap().to_owned();
    for (name, params) in [
        (
            "doc.position",
            json!({"doc": 1, "at": {"char": 1, "byte": 1}}),
        ),
        ("doc.position", json!({"doc": 1, "at": {}})),
        ("doc.position", json!({"doc": 1, "at": {"char": -1}})),
        (
            "doc.position",
            json!({"doc": 1, "at": {"line_col": {"line": 0}}}),
        ),
        (
            "doc.position",
            json!({"doc": 1, "at": {"line_col": {"line": 0, "col": 0, "x": 1}}}),
        ),
        ("doc.position", json!({"doc": 1, "at": {"char": 1}, "x": 1})),
        ("doc.line", json!({"doc": 1, "line": -1})),
        ("doc.line", json!({"doc": 1})),
        ("doc.line", json!({"doc": 9, "line": -1})),
        ("doc.line", json!({"doc": 1, "line": 0, "extra": true})),
    ] {
        assert_eq!(
            code(err(&r, &mut ctx, name, params.clone())),
            "invalid_params",
            "{params}"
        );
    }
    for (name, params) in [
        ("doc.line", json!({"doc": 9, "line": 99})),
        ("doc.line", json!({"doc": 0, "line": 0})),
        ("doc.position", json!({"doc": 9, "at": {"byte": 5}})),
    ] {
        let (reason, _) = refusal(&err(&r, &mut ctx, name, params.clone()));
        assert_eq!(reason, RejectReason::NoSuchDocument, "{params}");
    }
    ok(&r, &mut ctx, "doc.close", json!({"doc": 1}));
    let (reason, _) = refusal(&err(&r, &mut ctx, "doc.line", json!({"doc": 1, "line": 0})));
    assert_eq!(reason, RejectReason::NoSuchDocument);
}

#[test]
fn read_only_logged_and_typing_group_kept() {
    let r = reg();
    let mut ctx = ctx_with("xy");
    ok(
        &r,
        &mut ctx,
        "doc.insert",
        json!({"doc": 1, "offset": 2, "text": "a"}),
    );
    let info = ok(&r, &mut ctx, "doc.info", json!({"doc": 1}));
    ok(&r, &mut ctx, "doc.line", json!({"doc": 1, "line": 0}));
    ok(
        &r,
        &mut ctx,
        "doc.position",
        json!({"doc": 1, "at": {"char": 1}}),
    );
    assert_eq!(ok(&r, &mut ctx, "doc.info", json!({"doc": 1})), info);
    let tail: Vec<(String, bool)> = ctx
        .log()
        .iter()
        .rev()
        .take(3)
        .map(|e| (e.name.clone(), e.mutating))
        .collect();
    assert_eq!(
        tail,
        [
            ("doc.info".to_owned(), false),
            ("doc.position".to_owned(), false),
            ("doc.line".to_owned(), false),
        ]
    );
    // The open typing group still joins the next one-char insert: one undo removes both.
    ok(
        &r,
        &mut ctx,
        "doc.insert",
        json!({"doc": 1, "offset": 3, "text": "b"}),
    );
    ok(&r, &mut ctx, "doc.undo", json!({"doc": 1}));
    assert_eq!(
        ok(&r, &mut ctx, "doc.text", json!({"doc": 1}))["text"],
        "xy"
    );
}

#[test]
fn describe_publishes_both_commands_and_the_new_kind() {
    let r = reg();
    let d = r.describe();
    for name in ["doc.line", "doc.position"] {
        let c = d["commands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == name)
            .unwrap_or_else(|| panic!("{name} not described"));
        assert_eq!(c["mutating"], false, "{name}");
        assert!(!c["examples"].as_array().unwrap().is_empty(), "{name}");
    }
    let kinds = d["error_kinds"]["rejected"].as_array().unwrap();
    assert!(kinds.contains(&json!({
        "kind": "not_a_boundary",
        "when": "a position inside a character, a surrogate pair or a CRLF line break",
    })));
    assert_eq!(r.audit(), Vec::<String>::new());
}
