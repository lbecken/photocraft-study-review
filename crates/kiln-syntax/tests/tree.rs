//! Implementer's integration tests for `kiln_syntax::tree` (T-016), through the public API.
//! They use a small private corpus written here, not `tests/corpus/`.

use kiln_syntax::{ErrorKind, ErrorSpan, ParseMode, SyntaxTree};

/// Small valid Rust sources (edition 2024).
const VALID: &[&str] = &[
    "fn f() {}\n",
    "fn f(x: Option<i32>) { if let Some(a) = x && a > 1 { } }\n",
    "fn g() -> impl Sized + use<> { 0 }\n",
    "fn h() { let Some(x) = Some(1) else { return }; let _ = x; }\n",
    "/// Doc.\npub struct P<'a, T: Clone, const N: usize> where T: Default { r: &'a [T; N] }\n",
    "fn é() -> &'static str { \"😀 ünï\" }\r\nfn r#type() {}\r\n",
    "async fn a() { let c = move || 1u8; let _ = async { c() }.await; }\n",
    "macro_rules! m { ($($x:expr),*) => { 0 $(+ $x)* }; }\nconst C: i32 = m!(1, 2);\n",
    "unsafe extern \"C\" { fn x(); }\nenum E { A = 1, B = 2 }\nstruct T(u8, u16);\n",
    "fn m(v: u8) -> u8 { 'a: loop { break 'a match v { 0 | 1 => 2, n @ 3..=9 if n > 4 => n, _ => 0 } } }\n",
    "/* a /* nested */ b */ fn c() { let _ = (b\"x\", b'y', 'z', r#\"q\"#, c\"s\", 1_000i64, 2.5f32); }\n",
];

/// Small sources with syntax errors.
const INVALID: &[&str] = &[
    "fn a() { (\n",
    "fn (\n",
    "fn f() { let = ; }\n",
    "struct S { a: u8\n",
    "fn f() { let x = 1 }\n",
    "impl X for { fn }\n",
];

/// `SyntaxTree::parse`, unwrapped (a macro so the unwrap sits in the calling `#[test]`).
macro_rules! parse {
    ($text:expr) => {
        SyntaxTree::parse($text).unwrap()
    };
}

/// Checks that every char offset in the sexp and the error spans lies within the text.
fn assert_offsets_in_range(tree: &SyntaxTree) -> Result<(), String> {
    let len = tree.text().chars().count();
    for span in tree.errors() {
        assert!(
            span.start <= span.end && span.end <= len,
            "{span:?} len {len}"
        );
    }
    let sexp = tree.sexp();
    for piece in sexp.split('@').skip(1) {
        let range: String = piece
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        let mut parts = range.split("..");
        let s: Option<usize> = parts.next().and_then(|p| p.parse().ok());
        let e: Option<usize> = parts.next().and_then(|p| p.parse().ok());
        match (s, e) {
            (Some(s), Some(e)) if s <= e && e <= len => {}
            _ => return Err(format!("bad range {range:?} len {len}")),
        }
    }
    Ok(())
}

#[test]
fn worked_example_sexp_is_exact() {
    assert_eq!(
        parse!("fn f() {}").sexp(),
        "(source_file@0..9 (function_item@0..9 name:(identifier@3..4) \
         parameters:(parameters@4..6) body:(block@7..9)))"
    );
}

#[test]
fn missing_forms_are_exact() {
    let cases = [
        (
            "fn f() { let x = 1 }",
            "value:(integer_literal@17..18) (MISSING \";\"@18..18)",
        ),
        ("fn f() { 1 + }", "right:(MISSING identifier@12..12)"),
        (
            "fn f() { let s = r#\"abc\"; }",
            "value:(string_literal@23..24 (MISSING \"\\\"\"@24..24))",
        ),
    ];
    for (src, want) in cases {
        let t = parse!(src);
        assert!(t.sexp().contains(want), "{src}: {}", t.sexp());
        assert!(t.has_error());
        assert!(t.errors().iter().any(|e| e.kind == ErrorKind::Missing));
    }
    let t = parse!("fn f() { 1 + }");
    assert_eq!(
        t.errors(),
        vec![ErrorSpan {
            kind: ErrorKind::Missing,
            start: 12,
            end: 12
        }]
    );
}

#[test]
fn worked_has_error_values() {
    for src in [
        "fn f(x: Option<i32>) { if let Some(a) = x && a > 1 { } }",
        "fn g() -> impl Sized + use<> { 0 }",
        "fn h() { let Some(x) = Some(1) else { return }; }",
    ] {
        let t = parse!(src);
        assert!(!t.has_error(), "{src}: {}", t.sexp());
        assert!(t.errors().is_empty());
    }
    let t = parse!("fn a() { (");
    assert!(t.has_error());
    assert!(!t.errors().is_empty());
}

#[test]
fn error_node_prints_as_error() {
    let t = parse!("fn a() { ( } }");
    assert!(t.sexp().contains("(ERROR@"), "{}", t.sexp());
    assert!(t.errors().iter().any(|e| e.kind == ErrorKind::Error));
}

#[test]
fn private_valid_corpus_parses_clean_and_invalid_reports_errors() {
    for src in VALID {
        let t = parse!(src);
        assert!(!t.has_error(), "{src:?}: {}", t.sexp());
        assert_eq!(t.errors(), vec![]);
        assert_eq!(t.text(), *src);
    }
    for src in INVALID {
        let t = parse!(src);
        assert!(t.has_error(), "{src:?}");
        assert!(!t.errors().is_empty(), "{src:?}");
        assert_offsets_in_range(&t).unwrap();
    }
}

#[test]
fn offsets_are_chars_and_crlf_is_two() {
    // `é` is 2 bytes, `😀` 4 bytes; offsets count chars.
    let t = parse!("fn é() {}");
    assert_eq!(
        t.sexp(),
        "(source_file@0..9 (function_item@0..9 name:(identifier@3..4) \
         parameters:(parameters@4..6) body:(block@7..9)))"
    );
    let t = parse!("fn a() {}\r\nfn b() {}");
    assert!(t.sexp().contains("(function_item@11..20"), "{}", t.sexp());
    let t = parse!("fn f() { \"😀\"; 1 + }");
    // `+` is char 16 (byte 19); the MISSING operand sits right after it.
    assert_eq!(t.errors()[0].start, 17);
}

/// Deterministic xorshift generator for the edit loop.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

#[test]
fn incremental_update_equals_fresh_parse() {
    let extra: Vec<char> = "{}();\"'/\r\né😀".chars().collect();
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let (mut unchanged, mut incremental, mut full) = (0, 0, 0);
    for src in VALID.iter().chain(INVALID) {
        let mut tree = parse!(src);
        for _ in 0..120 {
            let chars: Vec<char> = tree.text().chars().collect();
            let pos = rng.below(chars.len() + 1);
            let del = match rng.below(3) {
                0 => 0,
                _ => rng.below(21).min(chars.len() - pos),
            };
            let ins_len = if rng.below(3) == 1 { 0 } else { rng.below(21) };
            let mut ins = String::new();
            for _ in 0..ins_len {
                if rng.below(2) == 0 || chars.is_empty() {
                    ins.push(extra[rng.below(extra.len())]);
                } else {
                    ins.push(chars[rng.below(chars.len())]);
                }
            }
            let mut new: String = chars[..pos].iter().collect();
            new.push_str(&ins);
            new.extend(&chars[pos + del..]);

            let before = tree.text().to_owned();
            let mode = tree.update(&new).unwrap();
            let fresh = parse!(&new);
            assert_eq!(tree.text(), new);
            assert_eq!(tree.sexp(), fresh.sexp(), "{before:?} -> {new:?}");
            assert_eq!(tree.errors(), fresh.errors());
            assert_eq!(tree.node_count(), fresh.node_count());
            assert_eq!(tree.has_error(), fresh.has_error());
            if new == before {
                assert_eq!(mode, ParseMode::Unchanged);
                unchanged += 1;
            } else if !fresh.has_error() {
                assert_eq!(mode, ParseMode::Incremental, "{before:?} -> {new:?}");
                incremental += 1;
            } else {
                assert_ne!(mode, ParseMode::Unchanged);
                if mode == ParseMode::Full {
                    full += 1;
                } else {
                    incremental += 1;
                }
            }
        }
    }
    assert!(
        unchanged > 0 && incremental > 0,
        "{unchanged} {incremental} {full}"
    );
}

#[test]
fn update_to_same_text_is_unchanged() {
    let mut t = parse!("fn f() {}");
    assert_eq!(t.update("fn f() {}"), Ok(ParseMode::Unchanged));
    assert_eq!(t.update("fn g() {}"), Ok(ParseMode::Incremental));
    assert_eq!(t.text(), "fn g() {}");
    assert_eq!(
        t.update("fn g() { (").map(|m| m != ParseMode::Unchanged),
        Ok(true)
    );
    assert!(t.has_error());
    assert_eq!(t.update(""), Ok(ParseMode::Incremental));
    assert_eq!(t.sexp(), "(source_file@0..0)");
}

#[test]
fn hostile_inputs_parse() {
    let deep_valid = format!("fn f() {{ {}1{} }}", "(".repeat(10_000), ")".repeat(10_000));
    let bytes: Vec<u8> = {
        let mut rng = Rng(42);
        (0..200_000).map(|_| rng.next() as u8).collect()
    };
    let inputs = [
        String::new(),
        "\0".to_owned(),
        "\r".to_owned(),
        "(".repeat(200_000),
        String::from_utf8_lossy(&bytes).into_owned(),
        deep_valid.clone(),
        "this is not Rust at all ¿¡ 😀".to_owned(),
    ];
    for input in &inputs {
        let t = parse!(input);
        let _ = t.sexp();
        let _ = t.node_count();
        assert_offsets_in_range(&t).unwrap();
    }
    let t = parse!(&deep_valid);
    assert!(!t.has_error());
    assert!(t.node_count() > 20_000);
    assert_eq!(parse!("").sexp(), "(source_file@0..0)");
}

#[test]
fn tree_is_clone_debug_send() {
    fn is_send<T: Send>(_: &T) {}
    let t = parse!("fn é() {}");
    is_send(&t);
    let c = t.clone();
    assert_eq!(c.sexp(), t.sexp());
    let dbg = format!("{t:?}");
    assert!(dbg.contains('9') && dbg.contains("false"), "{dbg}");
    assert!(!dbg.contains("fn é"), "{dbg}");
}

#[test]
fn workspace_sources_parse_without_errors() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut dirs = vec![root.join("xtask/src")];
    for top in ["crates", "apps"] {
        for entry in std::fs::read_dir(root.join(top)).unwrap() {
            dirs.push(entry.unwrap().path().join("src"));
        }
    }
    let mut files = 0;
    while let Some(dir) = dirs.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let text = std::fs::read_to_string(&path).unwrap();
                let t = parse!(&text);
                assert_eq!(t.errors(), vec![], "{}", path.display());
                files += 1;
            }
        }
    }
    assert!(files > 10, "{files}");
}
