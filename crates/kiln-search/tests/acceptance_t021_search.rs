//! T-021 acceptance tests through the Rust API of `kiln-search` (AC-7b, AC-10).
//!
//! Written by the implementer (medium tier, odd task number: no separate test author) before
//! any implementation code, from the task file's Interface and acceptance criteria. The
//! oracles are models built on T-017's merged `Regex::find_all` and `Regex::expand`:
//! - AC-10: `for_each_match` must yield exactly `find_all`'s captures, element by element, and
//!   its first k when the callback breaks after k calls;
//! - AC-7b: `CompiledQuery::find`, `replace_first` and `replace_all` must equal a model that
//!   runs `find_all` (literal mode: on the escaped pattern, which T-017 AC-3 shows equals a
//!   literal search), applies the whole-word filter of the spec to the whole-match ranges, and
//!   expands each match with `replacement_for` (regex mode) or inserts the replacement verbatim
//!   (literal mode).
//!
//! All generators are fixed-seed (SplitMix64). On the base this file does not compile:
//! `kiln_search::Query` and `Regex::for_each_match` do not exist there (weak evidence by
//! design, METHODOLOGY §6; the reviewer's mutation spot-check makes up for it).
#![cfg(test)]

use std::ops::ControlFlow;

use kiln_search::{
    Captures, CharMatch, CompiledQuery, Found, MAX_PATTERN_CHARS, Query, QueryError, Regex,
    RegexOptions,
};

// ---------------------------------------------------------------------------------------------
// Deterministic generator
// ---------------------------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------------------------
// Random patterns in T-017's subset, with T-017's restriction: no quantifier on a sub-pattern
// that can match the empty string.
// ---------------------------------------------------------------------------------------------

/// T-017's AC-1 alphabet.
const ALPHABET: &[char] = &[
    'a', 'b', 'A', '_', '0', 'é', 'σ', 'Σ', 'ς', 'ſ', 's', 'S', 'K', 'k', '\u{212A}', '\r', '\n',
    ' ', '😀',
];

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
        op: char,
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
        Node::Repeat { inner, op, .. } => *op != '+' || nullable(inner),
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

/// Escapes every metacharacter, so the pattern matches `s` literally.
fn escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        escape_char(&mut out, c);
    }
    out
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
            out.push(*op);
            if *lazy {
                out.push('?');
            }
        }
    }
}

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
            let op = self.rng.pick(&['*', '+', '?']);
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
            Node::Lit(self.rng.pick(ALPHABET))
        } else if r < 55 {
            Node::Any
        } else if r < 70 {
            let negated = self.rng.below(10) < 3;
            let n = 1 + self.rng.below(3);
            let items = (0..n)
                .map(|_| {
                    let a = self.rng.pick(ALPHABET);
                    if self.rng.below(10) < 6 {
                        (a, a)
                    } else {
                        let b = self.rng.pick(ALPHABET);
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
            Node::Lit(self.rng.pick(ALPHABET))
        }
    }

    fn pattern(&mut self) -> String {
        let mut s = String::new();
        print(&self.alt(3), &mut s);
        s
    }

    fn text(&mut self, max_len: usize) -> String {
        let n = self.rng.below(max_len + 1);
        (0..n).map(|_| self.rng.pick(ALPHABET)).collect()
    }

    /// A literal pattern: 1 to 3 alphabet chars, or a piece of `text` (so it often matches).
    fn literal(&mut self, text: &str) -> String {
        let chars: Vec<char> = text.chars().collect();
        if !chars.is_empty() && self.rng.below(2) == 0 {
            let s = self.rng.below(chars.len());
            let e = (s + 1 + self.rng.below(3)).min(chars.len());
            return chars[s..e].iter().collect();
        }
        let n = 1 + self.rng.below(3);
        (0..n).map(|_| self.rng.pick(ALPHABET)).collect()
    }

    /// A replacement template from T-017's AC-2 template alphabet.
    fn template(&mut self) -> String {
        const T: &[char] = &[
            '$', '$', '$', '$', '{', '}', '0', '1', '9', 'a', 'w', '_', 'x', '!', 'é', '😀', '+',
        ];
        let n = self.rng.below(9);
        (0..n).map(|_| self.rng.pick(T)).collect()
    }
}

// ---------------------------------------------------------------------------------------------
// The model on merged `find_all`
// ---------------------------------------------------------------------------------------------

fn is_word(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

/// The spec's whole-word filter: boundaries at both ends of `[s, e)`.
fn whole_word_ok(text: &[char], m: CharMatch) -> bool {
    (m.start == 0 || !is_word(text[m.start - 1])) && (m.end == text.len() || !is_word(text[m.end]))
}

/// The engine pattern of a query (literal mode: escaped).
fn engine_pattern(q: &Query) -> String {
    if q.regex {
        q.pattern.clone()
    } else {
        escape(&q.pattern)
    }
}

fn model_regex(q: &Query) -> Regex {
    Regex::new(
        &engine_pattern(q),
        RegexOptions {
            case_insensitive: !q.case_sensitive,
        },
    )
    .unwrap_or_else(|e| panic!("model pattern of {q:?} must compile: {e:?}"))
}

/// The matches of `q` on `text`: `find_all`, then the whole-word filter.
fn model_matches(q: &Query, text: &str) -> Vec<Captures> {
    let chars: Vec<char> = text.chars().collect();
    model_regex(q)
        .find_all(text)
        .into_iter()
        .filter(|c| {
            !q.whole_word || whole_word_ok(&chars, c.groups[0].expect("group 0 is always set"))
        })
        .collect()
}

fn model_found(q: &Query, text: &str, limit: usize) -> Found {
    let all = model_matches(q, text);
    Found {
        matches: all
            .iter()
            .take(limit)
            .map(|c| c.groups[0].expect("whole match"))
            .collect(),
        total: all.len(),
    }
}

/// The model's replacement text for one match (`Err` = a template error).
fn model_replacement(q: &Query, caps: &Captures, text: &str, template: &str) -> Option<String> {
    if q.regex {
        model_regex(q).expand(caps, text, template).ok()
    } else {
        Some(template.to_owned())
    }
}

/// Whether `template` is accepted for `q` (it depends on the pattern and the template only).
fn model_template_ok(q: &Query, template: &str) -> bool {
    !q.regex
        || model_regex(q)
            .expand(&Captures { groups: Vec::new() }, "", template)
            .is_ok()
}

fn compile(q: &Query) -> CompiledQuery {
    q.compile()
        .unwrap_or_else(|e| panic!("{q:?} must compile, got {e:?}"))
}

fn query(pattern: &str, regex: bool, case_sensitive: bool, whole_word: bool) -> Query {
    Query {
        pattern: pattern.to_owned(),
        regex,
        case_sensitive,
        whole_word,
    }
}

// ---------------------------------------------------------------------------------------------
// AC-10
// ---------------------------------------------------------------------------------------------

/// The captures `for_each_match` yields, copied, stopping after `stop_after` calls if given.
fn collect_each(re: &Regex, text: &str, stop_after: Option<usize>) -> Vec<Captures> {
    let mut out = Vec::new();
    re.for_each_match(text, &mut |caps: &Captures| {
        out.push(caps.clone());
        if stop_after == Some(out.len()) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    });
    out
}

/// AC-10: without a break `for_each_match` yields exactly `find_all(text)`, element by
/// element (groups included), on the AC-2 inputs in both case modes; a break after k calls
/// yields its first k and no further call.
#[test]
fn for_each_match_equals_find_all() {
    let mut g = Gen {
        rng: Rng(0x7021_0010),
    };
    let mut with_matches = 0usize;
    let mut broken = 0usize;
    for i in 0..6_000 {
        let pattern = g.pattern();
        let text = g.text(12);
        let ci = i % 2 == 1;
        let re = Regex::new(
            &pattern,
            RegexOptions {
                case_insensitive: ci,
            },
        )
        .unwrap_or_else(|e| panic!("#{i} {pattern:?} must compile: {e:?}"));
        let all = re.find_all(&text);
        let each = collect_each(&re, &text, None);
        assert_eq!(each, all, "#{i}: {pattern:?} ci={ci} on {text:?}");
        if !all.is_empty() {
            with_matches += 1;
            let k = 1 + g.rng.below(all.len());
            let first = collect_each(&re, &text, Some(k));
            assert_eq!(
                first,
                all[..k],
                "#{i}: break after {k}: {pattern:?} on {text:?}"
            );
            if k < all.len() {
                broken += 1;
            }
        }
    }
    assert!(
        with_matches > 3_000,
        "only {with_matches} inputs with a match"
    );
    assert!(
        broken > 500,
        "only {broken} inputs where the break cut the iteration"
    );

    // 100 groups, many matches; and the empty pattern's empty matches.
    let hundred = Regex::new(&"(a)".repeat(100), RegexOptions::default()).expect("100 groups");
    let text = "a".repeat(450);
    assert_eq!(collect_each(&hundred, &text, None), hundred.find_all(&text));
    let empties = Regex::new("()()", RegexOptions::default()).expect("empty groups");
    let text = "xé😀\r\ny";
    assert_eq!(collect_each(&empties, text, None), empties.find_all(text));
    assert_eq!(
        collect_each(&empties, text, Some(3)),
        empties.find_all(text)[..3]
    );
}

// ---------------------------------------------------------------------------------------------
// AC-7b
// ---------------------------------------------------------------------------------------------

/// AC-7b: on the AC-4 inputs (literal and regex queries, both case modes, with and without
/// whole-word, T-017's template alphabet), `replace_all` equals `replacement_for` applied to
/// each match of the `find_all` model; `replace_first` equals the model's first match with
/// `start >= from`; `find(text, limit)` (including `limit` 0) equals the first `limit`
/// whole-match ranges and the count of the model; `check_template` agrees with the model.
#[test]
fn replace_all_equals_per_match_expansion() {
    let mut g = Gen {
        rng: Rng(0x7021_0007),
    };
    let (mut oks, mut errs, mut nonempty) = (0usize, 0usize, 0usize);
    for i in 0..6_000 {
        let text = g.text(12);
        let regex = g.rng.below(10) < 6;
        let pattern = if regex { g.pattern() } else { g.literal(&text) };
        if pattern.is_empty() {
            continue;
        }
        let q = query(&pattern, regex, g.rng.below(2) == 0, g.rng.below(10) < 3);
        let cq = compile(&q);
        let template = g.template();
        let caps = model_matches(&q, &text);

        // find, with limit 0 (count only), a small limit and a large one.
        for limit in [0, 1 + g.rng.below(3), 10_000] {
            assert_eq!(
                cq.find(&text, limit),
                model_found(&q, &text, limit),
                "#{i}: find({limit}) of {q:?} on {text:?}"
            );
        }

        let template_ok = model_template_ok(&q, &template);
        assert_eq!(
            cq.check_template(&template).is_ok(),
            template_ok,
            "#{i}: check_template({template:?}) for {q:?}"
        );
        let got_all = cq.replace_all(&text, &template);
        if template_ok {
            oks += 1;
            let expected: Vec<(CharMatch, String)> = caps
                .iter()
                .map(|c| {
                    let r = cq
                        .replacement_for(c, &text, &template)
                        .unwrap_or_else(|e| panic!("#{i}: replacement_for: {e:?}"));
                    assert_eq!(
                        Some(r.clone()),
                        model_replacement(&q, c, &text, &template),
                        "#{i}: replacement_for({template:?}) of {q:?}"
                    );
                    (c.groups[0].expect("whole match"), r)
                })
                .collect();
            if expected.iter().any(|(_, r)| !r.is_empty()) {
                nonempty += 1;
            }
            assert_eq!(
                got_all.as_ref().ok(),
                Some(&expected),
                "#{i}: replace_all({template:?}) of {q:?} on {text:?}"
            );
            let n = text.chars().count();
            let from = g.rng.below(n + 2);
            let first = expected.iter().find(|(m, _)| m.start >= from).cloned();
            assert_eq!(
                cq.replace_first(&text, from, &template).ok(),
                Some(first),
                "#{i}: replace_first(from {from}, {template:?}) of {q:?} on {text:?}"
            );
        } else {
            errs += 1;
            assert!(
                got_all.is_err(),
                "#{i}: replace_all must refuse {template:?}"
            );
            assert!(
                cq.replace_first(&text, 0, &template).is_err(),
                "#{i}: replace_first must refuse {template:?}"
            );
        }
    }
    assert!(oks > 2_000 && errs > 500, "oks={oks} errs={errs}");
    assert!(nonempty > 1_000, "nonempty={nonempty}");

    // Literal mode: `$1` is inserted verbatim; capture groups do not exist there.
    let cq = compile(&query("b", false, true, false));
    assert!(cq.check_template("$1${x}").is_ok());
    let got = cq.replace_all("a1 b2", "$1").expect("literal template");
    assert_eq!(got, vec![(CharMatch { start: 3, end: 4 }, "$1".to_owned())]);
}

/// AC-7b, compile: `Empty`, then `TooLong` (over `MAX_PATTERN_CHARS` chars, both modes,
/// before compiling), then `Pattern`; the cap counts chars, not bytes.
#[test]
fn query_errors_in_order() {
    assert_eq!(MAX_PATTERN_CHARS, 1000);
    for regex in [false, true] {
        for cs in [false, true] {
            assert_eq!(
                query("", regex, cs, false).compile().err(),
                Some(QueryError::Empty)
            );
            assert_eq!(
                query(&"(".repeat(1001), regex, cs, false).compile().err(),
                Some(QueryError::TooLong { chars: 1001 })
            );
            assert_eq!(
                query(&"é".repeat(1001), regex, cs, false).compile().err(),
                Some(QueryError::TooLong { chars: 1001 })
            );
            let cq = compile(&query(&"é".repeat(1000), regex, cs, false));
            assert_eq!(cq.find(&"é".repeat(2001), 10).total, 2);
            let cq = compile(&query(&"a".repeat(1000), regex, cs, false));
            assert_eq!(
                cq.find(&"a".repeat(2500), 10).matches,
                vec![
                    CharMatch {
                        start: 0,
                        end: 1000
                    },
                    CharMatch {
                        start: 1000,
                        end: 2000
                    }
                ]
            );
        }
    }
    for bad in ["(", "[a-", "a{99999999}"] {
        assert!(
            matches!(
                query(bad, true, true, false).compile(),
                Err(QueryError::Pattern(_))
            ),
            "{bad:?}"
        );
        // In literal mode the same text is a plain char sequence.
        assert!(query(bad, false, true, false).compile().is_ok(), "{bad:?}");
    }
    assert!(matches!(
        query(&"(a)".repeat(101), true, true, false).compile(),
        Err(QueryError::Pattern(_))
    ));
    let cq = compile(&query(
        &format!("{}(b)", "(a)".repeat(99)),
        true,
        true,
        false,
    ));
    let text = format!("{}b", "a".repeat(99));
    assert_eq!(
        cq.replace_all(&text, "$100").expect("group 100 exists"),
        vec![(CharMatch { start: 0, end: 100 }, "b".to_owned())]
    );
    assert!(cq.check_template("$101").is_err());
}
