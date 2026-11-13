//! Implementer's integration tests for `kiln_syntax::highlight` and `kiln_syntax::brackets`
//! (T-020), through the public API, on the pinned corpus under `tests/corpus/` and on
//! hand-written texts. Generators are fixed-seed (SplitMix64); no test spawns a process.
#![cfg(test)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use kiln_syntax::SyntaxTree;
use kiln_syntax::brackets::{BracketPair, match_bracket};
use kiln_syntax::highlight::{HighlightKind, HighlightSpan, KEYWORDS, highlight};
use proc_macro2::{Delimiter, TokenStream, TokenTree};

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("corpus")
}

/// Every corpus file as (set, path, text), from `MANIFEST.toml`.
fn corpus() -> Vec<(String, String, String)> {
    let manifest = fs::read_to_string(corpus_dir().join("MANIFEST.toml")).unwrap();
    let table: toml::Table = toml::from_str(&manifest).unwrap();
    let files = table["file"].as_array().unwrap();
    let out: Vec<_> = files
        .iter()
        .map(|f| {
            let path = f["path"].as_str().unwrap().to_owned();
            let set = f["set"].as_str().unwrap().to_owned();
            let text = fs::read_to_string(corpus_dir().join(&path)).unwrap();
            (set, path, text)
        })
        .collect();
    assert!(out.iter().filter(|f| f.0 == "valid").count() >= 30);
    out
}

/// SplitMix64.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % (n as u64)) as usize
    }
}

/// The spans partition `start..end`: sorted, contiguous, non-empty, merged.
fn assert_partition(spans: &[HighlightSpan], start: usize, end: usize, ctx: &str) {
    if start == end {
        assert!(spans.is_empty(), "{ctx}: empty range gave {spans:?}");
        return;
    }
    assert_eq!(spans.first().unwrap().start, start, "{ctx}");
    assert_eq!(spans.last().unwrap().end, end, "{ctx}");
    for s in spans {
        assert!(s.start < s.end, "{ctx}: {s:?}");
        assert!(HighlightKind::ALL.contains(&s.kind), "{ctx}");
    }
    for w in spans.windows(2) {
        assert_eq!(w[0].end, w[1].start, "{ctx}: gap or overlap {w:?}");
        assert_ne!(w[0].kind, w[1].kind, "{ctx}: not merged {w:?}");
    }
}

/// The kind of every char of `start..end` according to `spans`.
fn per_char(spans: &[HighlightSpan]) -> Vec<HighlightKind> {
    spans
        .iter()
        .flat_map(|s| std::iter::repeat_n(s.kind, s.end - s.start))
        .collect()
}

#[test]
fn spans_partition_every_corpus_file_and_agree_on_sub_ranges() {
    let mut rng = Rng(0x7020);
    for (_, path, text) in corpus() {
        let tree = SyntaxTree::parse(&text).unwrap();
        let len = text.chars().count();
        let full = highlight(&tree, 0..len);
        assert_partition(&full, 0, len, &path);
        let kinds = per_char(&full);
        for _ in 0..60 {
            let a = rng.below(len + 1);
            let b = rng.below(len + 1);
            let (s, e) = (a.min(b), a.max(b));
            let sub = highlight(&tree, s..e);
            let ctx = format!("{path} {s}..{e}");
            assert_partition(&sub, s, e, &ctx);
            assert_eq!(per_char(&sub), kinds[s..e], "{ctx}");
        }
    }
}

#[test]
fn keyword_and_comment_spans_on_valid_files() {
    for (set, path, text) in corpus() {
        if set != "valid" {
            continue;
        }
        let tree = SyntaxTree::parse(&text).unwrap();
        let chars: Vec<char> = text.chars().collect();
        let spans = highlight(&tree, 0..chars.len());
        let mut seen = BTreeSet::new();
        for s in &spans {
            let t: String = chars[s.start..s.end].iter().collect();
            seen.insert(s.kind);
            match s.kind {
                HighlightKind::Keyword => {
                    // A keyword span may merge adjacent keywords without a gap (none in
                    // practice); every word of it is a pinned keyword.
                    assert!(KEYWORDS.contains(&t.as_str()), "{path}: keyword {t:?}");
                }
                HighlightKind::Comment => {
                    assert!(t.starts_with("//") || t.starts_with("/*"), "{path}: {t:?}");
                }
                HighlightKind::Number => {
                    assert!(t.starts_with(|c: char| c.is_ascii_digit()), "{path}: {t:?}");
                }
                _ => {}
            }
        }
        assert!(seen.contains(&HighlightKind::Keyword), "{path}");
    }
}

/// Collect every `( [ {` group whose source chars are the delimiter, as byte ranges.
fn groups(ts: TokenStream, text: &str, out: &mut Vec<(usize, usize)>) {
    let mut stack = vec![ts];
    while let Some(ts) = stack.pop() {
        for tt in ts {
            if let TokenTree::Group(g) = tt {
                let (o, c) = match g.delimiter() {
                    Delimiter::Parenthesis => ('(', ')'),
                    Delimiter::Bracket => ('[', ']'),
                    Delimiter::Brace => ('{', '}'),
                    Delimiter::None => {
                        stack.push(g.stream());
                        continue;
                    }
                };
                let open = g.span_open().byte_range().start;
                let close = g.span_close().byte_range().start;
                if text[open..].starts_with(o) && text[close..].starts_with(c) {
                    out.push((open, close));
                }
                stack.push(g.stream());
            }
        }
    }
}

#[test]
fn brackets_agree_with_proc_macro2_groups_on_valid_files() {
    for (set, path, text) in corpus() {
        if set != "valid" {
            continue;
        }
        let tree = SyntaxTree::parse(&text).unwrap();
        let ts = TokenStream::from_str(&text).unwrap();
        let mut gs = Vec::new();
        groups(ts, &text, &mut gs);
        assert!(!gs.is_empty(), "{path}");
        let char_of = |b: usize| text[..b].chars().count();
        let mut paired = BTreeSet::new();
        for (o, c) in gs {
            let want = Some(BracketPair {
                open: char_of(o),
                close: char_of(c),
            });
            assert_eq!(match_bracket(&tree, char_of(o)), want, "{path} open {o}");
            assert_eq!(match_bracket(&tree, char_of(c)), want, "{path} close {c}");
            paired.insert(char_of(o));
            paired.insert(char_of(c));
        }
        for (i, ch) in text.chars().enumerate() {
            if "()[]{}".contains(ch) && !paired.contains(&i) {
                assert_eq!(match_bracket(&tree, i), None, "{path} {i} {ch}");
            }
        }
    }
}

#[test]
fn deep_nesting_is_answered_without_recursion() {
    let n = 500_000;
    let text = format!("fn f() {{ {}1{} }}", "(".repeat(n), ")".repeat(n));
    let len = text.chars().count();
    assert_eq!(len, 1_000_012);
    let tree = SyntaxTree::parse(&text).unwrap();
    assert!(!tree.has_error());
    let spans = highlight(&tree, 0..len);
    assert_partition(&spans, 0, len, "deep");
    assert_eq!(
        match_bracket(&tree, 9),
        Some(BracketPair {
            open: 9,
            close: 1_000_009
        })
    );
    assert_eq!(
        match_bracket(&tree, 4),
        Some(BracketPair { open: 4, close: 5 })
    );
    assert_eq!(
        match_bracket(&tree, 9 + n - 1),
        Some(BracketPair {
            open: 9 + n - 1,
            close: 9 + n + 1
        })
    );
}

#[test]
fn crlf_and_multibyte_offsets_are_chars() {
    let text = "// é\r\nfn é() { \"(\" }\r\n";
    let tree = SyntaxTree::parse(text).unwrap();
    let len = text.chars().count();
    let spans = highlight(&tree, 0..len);
    assert_partition(&spans, 0, len, "crlf");
    // A range end between `\r` and `\n` is accepted.
    let cut = text.chars().position(|c| c == '\r').unwrap() + 1;
    assert_partition(&highlight(&tree, 0..cut), 0, cut, "cut");
    let open = text.chars().position(|c| c == '{').unwrap();
    let close = text.chars().position(|c| c == '}').unwrap();
    assert_eq!(
        match_bracket(&tree, open),
        Some(BracketPair { open, close })
    );
    let quoted = text[..text.find("\"(").unwrap()].chars().count() + 1;
    assert_eq!(text.chars().nth(quoted), Some('('));
    assert_eq!(match_bracket(&tree, quoted), None);
}

/// Byte ranges of every literal token whose source slice is its text (doc comments, which
/// proc-macro2 desugars into string literals, are excluded by that).
fn literals(ts: TokenStream, text: &str, out: &mut Vec<(usize, usize)>) {
    let mut stack = vec![ts];
    while let Some(ts) = stack.pop() {
        for tt in ts {
            match tt {
                TokenTree::Group(g) => stack.push(g.stream()),
                TokenTree::Literal(l) => {
                    let r = l.span().byte_range();
                    if text.get(r.clone()) == Some(l.to_string().as_str()) {
                        out.push((r.start, r.end));
                    }
                }
                _ => {}
            }
        }
    }
}

#[test]
fn string_and_number_spans_are_the_literal_tokens_on_valid_files() {
    for (set, path, text) in corpus() {
        if set != "valid" {
            continue;
        }
        let tree = SyntaxTree::parse(&text).unwrap();
        let len = text.chars().count();
        let kinds = per_char(&highlight(&tree, 0..len));
        let mut lits = Vec::new();
        literals(TokenStream::from_str(&text).unwrap(), &text, &mut lits);
        assert!(!lits.is_empty(), "{path}");
        let mut want = vec![false; len];
        for (s, e) in lits {
            let (s, e) = (text[..s].chars().count(), text[..e].chars().count());
            want[s..e].iter_mut().for_each(|w| *w = true);
        }
        for (i, k) in kinds.iter().enumerate() {
            let got = matches!(k, HighlightKind::String | HighlightKind::Number);
            assert_eq!(got, want[i], "{path} char {i} {k:?}");
        }
    }
}
