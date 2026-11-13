//! Independent acceptance tests for T-017 (regular expressions in `kiln-search` with char-offset
//! matches, line-aware anchors, pinned empty-match iteration and replacement expansion).
//!
//! Written from the task file's Interface, Semantics, Pinned section and acceptance criteria
//! only. The oracles are built here:
//! - a naive backtracking matcher for the AC-1 subset (literal chars, `.`, `[…]`, `[^…]`,
//!   `*` `+` `?` and their lazy forms, `|`, `(…)`, `(?:…)`, `^`, `$`) that implements the
//!   spec's iteration, anchor, capture and case-folding rules (AC-1, AC-3);
//! - a naive implementation of the expansion rule (AC-2);
//! - a naive literal search, against which a fully escaped pattern must agree (AC-3);
//! - the exact values of Pinned (g) (AC-5, AC-6) and the exact properties of AC-4.
//!
//! All generators are fixed-seed (SplitMix64), so every run is identical. On the base these
//! fail to compile (the `kiln_search::regex` API and the `regex` dependency are the
//! implementer's): weak evidence by design (spec check R-067 note 16).
#![cfg(test)]

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use kiln_search::{Captures, CharMatch, PatternError, Regex, RegexOptions, TemplateError};

// ---------------------------------------------------------------------------------------------
// Deterministic generator
// ---------------------------------------------------------------------------------------------

/// SplitMix64: tiny, fixed-seed, no dependency.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..n` (n > 0).
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len())]
    }
}

// ---------------------------------------------------------------------------------------------
// Helpers on the API under test
// ---------------------------------------------------------------------------------------------

type Span = Option<(usize, usize)>;

fn opts(case_insensitive: bool) -> RegexOptions {
    RegexOptions { case_insensitive }
}

fn compile(pattern: &str, case_insensitive: bool) -> Regex {
    match Regex::new(pattern, opts(case_insensitive)) {
        Ok(re) => re,
        Err(e) => panic!("pattern {pattern:?} (ci={case_insensitive}) must compile, got {e:?}"),
    }
}

fn spans(caps: &Captures) -> Vec<Span> {
    caps.groups
        .iter()
        .map(|g| g.map(|m| (m.start, m.end)))
        .collect()
}

fn all_spans(re: &Regex, text: &str) -> Vec<Vec<Span>> {
    re.find_all(text).iter().map(spans).collect()
}

fn whole(re: &Regex, text: &str) -> Vec<(usize, usize)> {
    re.find_all(text)
        .iter()
        .map(|c| {
            let m = c.groups[0].expect("group 0 (the whole match) is always set");
            (m.start, m.end)
        })
        .collect()
}

fn cm(start: usize, end: usize) -> CharMatch {
    CharMatch { start, end }
}

// ---------------------------------------------------------------------------------------------
// Case folding predicate (AC-1): "equal, or equal to_lowercase(), or equal to_uppercase()"
// ---------------------------------------------------------------------------------------------

fn ci_eq(a: char, b: char) -> bool {
    a == b || a.to_lowercase().eq(b.to_lowercase()) || a.to_uppercase().eq(b.to_uppercase())
}

/// Every Unicode scalar value `m` with `ci_eq(m, c)`, for each `c` of the test alphabets,
/// computed by brute force over all scalar values (class ranges may contain members outside
/// the alphabet, so the class rule "some member satisfies the predicate" needs the full set).
fn fold_sets() -> &'static HashMap<char, Vec<char>> {
    static SETS: OnceLock<HashMap<char, Vec<char>>> = OnceLock::new();
    SETS.get_or_init(|| {
        let targets: Vec<char> = AC1_ALPHABET
            .iter()
            .chain(AC3_ALPHABET.iter())
            .copied()
            .collect::<HashSet<char>>()
            .into_iter()
            .collect();
        let key = |it: &mut dyn Iterator<Item = char>| {
            let mut k = ['\0'; 4];
            for (i, c) in it.take(4).enumerate() {
                k[i] = c;
            }
            k
        };
        let tkeys: Vec<(char, [char; 4], [char; 4])> = targets
            .iter()
            .map(|&c| (c, key(&mut c.to_lowercase()), key(&mut c.to_uppercase())))
            .collect();
        let mut sets: HashMap<char, Vec<char>> = HashMap::new();
        for m in (0u32..=0x10FFFF).filter_map(char::from_u32) {
            let lk = key(&mut m.to_lowercase());
            let uk = key(&mut m.to_uppercase());
            for (c, cl, cu) in &tkeys {
                if m == *c || lk == *cl || uk == *cu {
                    sets.entry(*c).or_default().push(m);
                }
            }
        }
        sets
    })
}

// ---------------------------------------------------------------------------------------------
// Naive model: pattern AST, printer, compiler to a backtracking program
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    Star,
    Plus,
    Quest,
}

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
        op: Op,
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
        Node::Repeat { inner, op, .. } => match op {
            Op::Star | Op::Quest => true,
            Op::Plus => nullable(inner),
        },
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
            out.push(match op {
                Op::Star => '*',
                Op::Plus => '+',
                Op::Quest => '?',
            });
            if *lazy {
                out.push('?');
            }
        }
    }
}

fn pattern_of(n: &Node) -> String {
    let mut s = String::new();
    print(n, &mut s);
    s
}

#[derive(Clone, Debug)]
enum Inst {
    Lit(char),
    Any,
    Class {
        negated: bool,
        items: Vec<(char, char)>,
    },
    Bol,
    Eol,
    Split(usize, usize),
    Jmp(usize),
    Save(usize),
    Match,
}

struct Program {
    insts: Vec<Inst>,
    groups: usize,
}

struct Compiler {
    insts: Vec<Inst>,
    groups: usize,
}

impl Compiler {
    fn emit(&mut self, i: Inst) -> usize {
        self.insts.push(i);
        self.insts.len() - 1
    }

    fn split(lazy: bool, body: usize, out: usize) -> Inst {
        if lazy {
            Inst::Split(out, body)
        } else {
            Inst::Split(body, out)
        }
    }

    fn node(&mut self, n: &Node) {
        match n {
            Node::Empty => {}
            Node::Lit(c) => {
                self.emit(Inst::Lit(*c));
            }
            Node::Any => {
                self.emit(Inst::Any);
            }
            Node::Class { negated, items } => {
                self.emit(Inst::Class {
                    negated: *negated,
                    items: items.clone(),
                });
            }
            Node::Bol => {
                self.emit(Inst::Bol);
            }
            Node::Eol => {
                self.emit(Inst::Eol);
            }
            Node::Group { capture, inner } => {
                if *capture {
                    // Groups are numbered by their opening parenthesis, i.e. pre-order.
                    self.groups += 1;
                    let g = self.groups;
                    self.emit(Inst::Save(2 * g));
                    self.node(inner);
                    self.emit(Inst::Save(2 * g + 1));
                } else {
                    self.node(inner);
                }
            }
            Node::Concat(v) => v.iter().for_each(|x| self.node(x)),
            Node::Alt(v) => {
                let mut jumps = Vec::new();
                for (i, b) in v.iter().enumerate() {
                    if i + 1 < v.len() {
                        let s = self.emit(Inst::Split(0, 0));
                        self.node(b);
                        jumps.push(self.emit(Inst::Jmp(0)));
                        let next = self.insts.len();
                        self.insts[s] = Inst::Split(s + 1, next);
                    } else {
                        self.node(b);
                    }
                }
                let end = self.insts.len();
                for j in jumps {
                    self.insts[j] = Inst::Jmp(end);
                }
            }
            Node::Repeat { inner, op, lazy } => match op {
                Op::Star => {
                    let l = self.emit(Inst::Split(0, 0));
                    self.node(inner);
                    self.emit(Inst::Jmp(l));
                    let out = self.insts.len();
                    self.insts[l] = Self::split(*lazy, l + 1, out);
                }
                Op::Plus => {
                    let l = self.insts.len();
                    self.node(inner);
                    let s = self.emit(Inst::Split(0, 0));
                    self.insts[s] = Self::split(*lazy, l, s + 1);
                }
                Op::Quest => {
                    let s = self.emit(Inst::Split(0, 0));
                    self.node(inner);
                    let out = self.insts.len();
                    self.insts[s] = Self::split(*lazy, s + 1, out);
                }
            },
        }
    }
}

fn program(n: &Node) -> Program {
    let mut c = Compiler {
        insts: Vec::new(),
        groups: 0,
    };
    c.emit(Inst::Save(0));
    c.node(n);
    c.emit(Inst::Save(1));
    c.emit(Inst::Match);
    Program {
        insts: c.insts,
        groups: c.groups,
    }
}

struct Matcher<'a> {
    prog: &'a Program,
    text: &'a [char],
    ci: bool,
}

impl Matcher<'_> {
    fn lit(&self, p: char, c: char) -> bool {
        if self.ci { ci_eq(p, c) } else { p == c }
    }

    fn in_items(items: &[(char, char)], c: char) -> bool {
        items.iter().any(|&(lo, hi)| lo <= c && c <= hi)
    }

    fn class(&self, negated: bool, items: &[(char, char)], c: char) -> bool {
        let member = if self.ci {
            // Folded before negation: some member m satisfies the predicate with c.
            fold_sets()
                .get(&c)
                .expect("text chars come from the test alphabets")
                .iter()
                .any(|&m| Self::in_items(items, m))
        } else {
            Self::in_items(items, c)
        };
        member != negated
    }

    /// `^`: start of text, after `\n`, after `\r\n` (i.e. after its `\n`), after a lone `\r`.
    fn bol(&self, pos: usize) -> bool {
        let t = self.text;
        pos == 0 || t[pos - 1] == '\n' || (t[pos - 1] == '\r' && t.get(pos) != Some(&'\n'))
    }

    /// `$`: end of text, before `\r`, before a `\n` that is not the second half of a CRLF.
    fn eol(&self, pos: usize) -> bool {
        let t = self.text;
        pos == t.len() || t[pos] == '\r' || (t[pos] == '\n' && (pos == 0 || t[pos - 1] != '\r'))
    }

    fn run(&self, pc: usize, pos: usize, slots: &mut Vec<Option<usize>>) -> bool {
        let t = self.text;
        match &self.prog.insts[pc] {
            Inst::Lit(p) => {
                pos < t.len() && self.lit(*p, t[pos]) && self.run(pc + 1, pos + 1, slots)
            }
            Inst::Any => {
                pos < t.len()
                    && t[pos] != '\r'
                    && t[pos] != '\n'
                    && self.run(pc + 1, pos + 1, slots)
            }
            Inst::Class { negated, items } => {
                pos < t.len()
                    && self.class(*negated, items, t[pos])
                    && self.run(pc + 1, pos + 1, slots)
            }
            Inst::Bol => self.bol(pos) && self.run(pc + 1, pos, slots),
            Inst::Eol => self.eol(pos) && self.run(pc + 1, pos, slots),
            Inst::Split(x, y) => self.run(*x, pos, slots) || self.run(*y, pos, slots),
            Inst::Jmp(x) => self.run(*x, pos, slots),
            Inst::Save(k) => {
                let old = slots[*k];
                slots[*k] = Some(pos);
                if self.run(pc + 1, pos, slots) {
                    return true;
                }
                slots[*k] = old;
                false
            }
            Inst::Match => true,
        }
    }

    /// Leftmost-first match whose start is >= `from`.
    fn find_from(&self, from: usize) -> Option<Vec<Span>> {
        for s in from..=self.text.len() {
            let mut slots = vec![None; 2 * (self.prog.groups + 1)];
            if self.run(0, s, &mut slots) {
                return Some(
                    (0..=self.prog.groups)
                        .map(|g| match (slots[2 * g], slots[2 * g + 1]) {
                            (Some(a), Some(b)) => Some((a, b)),
                            _ => None,
                        })
                        .collect(),
                );
            }
        }
        None
    }

    /// The spec's iteration rule.
    fn find_all(&self) -> Vec<Vec<Span>> {
        let mut out = Vec::new();
        let mut pos = 0;
        let mut last_end: Option<usize> = None;
        while pos <= self.text.len() {
            let Some(caps) = self.find_from(pos) else {
                break;
            };
            let (s, e) = caps[0].expect("whole match");
            if s == e && last_end == Some(s) {
                // Empty match at the previous match's end: discarded, restart at e+1.
                pos = s + 1;
                continue;
            }
            out.push(caps);
            last_end = Some(e);
            pos = if s == e { s + 1 } else { e };
        }
        out
    }
}

fn model_find_all(n: &Node, text: &str, ci: bool) -> (usize, Vec<Vec<Span>>) {
    let prog = program(n);
    let chars: Vec<char> = text.chars().collect();
    let m = Matcher {
        prog: &prog,
        text: &chars,
        ci,
    };
    (prog.groups, m.find_all())
}

// ---------------------------------------------------------------------------------------------
// Random patterns (AC-1 generator restriction: no quantifier on a nullable operand)
// ---------------------------------------------------------------------------------------------

/// AC-1 alphabet: `a b A _ 0 é σ Σ ς ſ s S`, `K` (U+004B), `k`, `K` (U+212A), `\r \n`, ` `, `😀`.
const AC1_ALPHABET: &[char] = &[
    'a', 'b', 'A', '_', '0', 'é', 'σ', 'Σ', 'ς', 'ſ', 's', 'S', 'K', 'k', '\u{212A}', '\r', '\n',
    ' ', '😀',
];

/// AC-3 alphabet: 1-, 2-, 3- and 4-byte chars and both line-ending chars.
const AC3_ALPHABET: &[char] = &['a', 'b', 'é', 'É', '中', '😀', '\r', '\n', ' ', 'B'];

struct Gen<'a> {
    rng: Rng,
    alphabet: &'a [char],
}

impl Gen<'_> {
    fn alt(&mut self, depth: usize) -> Node {
        let r = self.rng.below(10);
        let k = if r < 6 {
            1
        } else if r < 9 {
            2
        } else {
            3
        };
        let branches: Vec<Node> = (0..k).map(|_| self.concat(depth)).collect();
        if branches.len() == 1 {
            branches.into_iter().next().unwrap_or(Node::Empty)
        } else {
            Node::Alt(branches)
        }
    }

    fn concat(&mut self, depth: usize) -> Node {
        let k = if self.rng.below(20) == 0 {
            0
        } else {
            1 + self.rng.below(3)
        };
        if k == 0 {
            return Node::Empty;
        }
        Node::Concat((0..k).map(|_| self.piece(depth)).collect())
    }

    fn piece(&mut self, depth: usize) -> Node {
        let atom = self.atom(depth);
        if !nullable(&atom) && self.rng.below(10) < 3 {
            let op = self.rng.pick(&[Op::Star, Op::Plus, Op::Quest]);
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
            Node::Lit(self.rng.pick(self.alphabet))
        } else if r < 55 {
            Node::Any
        } else if r < 70 {
            let negated = self.rng.below(10) < 3;
            let n = 1 + self.rng.below(3);
            let items = (0..n)
                .map(|_| {
                    let a = self.rng.pick(self.alphabet);
                    if self.rng.below(10) < 6 {
                        (a, a)
                    } else {
                        let b = self.rng.pick(self.alphabet);
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
            Node::Lit(self.rng.pick(self.alphabet))
        }
    }

    fn text(&mut self, max_len: usize) -> String {
        let n = self.rng.below(max_len + 1);
        (0..n).map(|_| self.rng.pick(self.alphabet)).collect()
    }
}

/// Never quantifies a nullable operand (checked here, so a generator bug cannot slip a
/// pattern of the AC-6 class into AC-1).
fn respects_restriction(n: &Node) -> bool {
    match n {
        Node::Repeat { inner, .. } => !nullable(inner) && respects_restriction(inner),
        Node::Group { inner, .. } => respects_restriction(inner),
        Node::Concat(v) | Node::Alt(v) => v.iter().all(respects_restriction),
        _ => true,
    }
}

fn differential(seed: u64, pairs: usize, alphabet: &[char], max_text: usize, ci: bool) {
    let mut g = Gen {
        rng: Rng::new(seed),
        alphabet,
    };
    let mut nonempty = 0usize;
    let mut with_groups = 0usize;
    for i in 0..pairs {
        let node = g.alt(3);
        assert!(respects_restriction(&node));
        let pattern = pattern_of(&node);
        let text = g.text(max_text);
        let re = compile(&pattern, ci);
        let (groups, expected) = model_find_all(&node, &text, ci);
        assert_eq!(
            re.group_count(),
            groups,
            "pair {i}: group_count of {pattern:?}"
        );
        let got = re.find_all(&text);
        let got_spans: Vec<Vec<Span>> = got.iter().map(spans).collect();
        assert_eq!(
            got_spans, expected,
            "pair {i}: find_all({pattern:?}, ci={ci}) on {text:?}"
        );
        let n_chars = text.chars().count();
        for c in &got {
            assert_eq!(c.groups.len(), re.group_count() + 1);
            for m in c.groups.iter().flatten() {
                assert!(m.start <= m.end && m.end <= n_chars);
            }
        }
        if expected.iter().any(|c| c[0].is_some_and(|(s, e)| s < e)) {
            nonempty += 1;
        }
        if groups > 0 && expected.iter().any(|c| c[1..].iter().any(Option::is_some)) {
            with_groups += 1;
        }
    }
    // The corpus is not degenerate: many pairs have non-empty matches and set groups.
    assert!(
        nonempty > pairs / 5,
        "only {nonempty} pairs with a non-empty match"
    );
    assert!(
        with_groups > pairs / 20,
        "only {with_groups} pairs with a set group"
    );
}

// ---------------------------------------------------------------------------------------------
// AC-1
// ---------------------------------------------------------------------------------------------

/// AC-1, case-sensitive: 20,000 fixed-seed (pattern, text) pairs, `find_all` (groups included)
/// equals the naive backtracker.
#[test]
fn matches_equal_naive_backtracker() {
    differential(0x7017_0001, 20_000, AC1_ALPHABET, 10, false);
}

/// AC-1, `case_insensitive: true`: same generator, the model folds with the AC-1 predicate.
#[test]
fn case_insensitive_matches_equal_naive_model() {
    differential(0x7017_0002, 20_000, AC1_ALPHABET, 10, true);
}

/// The model's own sanity on a few hand-checked cases (guards against a model bug that would
/// make AC-1 vacuous or wrong).
#[test]
fn naive_model_self_check() {
    let lit = |s: &str| Node::Concat(s.chars().map(Node::Lit).collect());
    let star = |n: Node| Node::Repeat {
        inner: Box::new(n),
        op: Op::Star,
        lazy: false,
    };
    // `a*` on `baaab` (Pinned (g)) -- but `a*` is fine here: `a` is not nullable.
    let (_, m) = model_find_all(&star(Node::Lit('a')), "baaab", false);
    let w: Vec<_> = m.iter().map(|c| c[0]).collect();
    assert_eq!(w, vec![Some((0, 0)), Some((1, 4)), Some((5, 5))]);
    // `^.|.$` on the Pinned (g) text.
    let n = Node::Alt(vec![
        Node::Concat(vec![Node::Bol, Node::Any]),
        Node::Concat(vec![Node::Any, Node::Eol]),
    ]);
    let (_, m) = model_find_all(&n, "ab\rcd\r\nef\ngh", false);
    let starts: Vec<usize> = m.iter().map(|c| c[0].unwrap().0).collect();
    assert_eq!(starts, vec![0, 1, 3, 4, 7, 8, 10, 11]);
    // Case-insensitive `σ|ſ|k` on `ΣςSKK`.
    let n = Node::Alt(vec![lit("σ"), lit("ſ"), lit("k")]);
    let (_, m) = model_find_all(&n, "ΣςSK\u{212A}", true);
    assert_eq!(m.len(), 5);
    // `(?i)[^a]` excludes `A`.
    let n = Node::Class {
        negated: true,
        items: vec![('a', 'a')],
    };
    let (_, m) = model_find_all(&n, "aAb", true);
    let w: Vec<_> = m.iter().map(|c| c[0]).collect();
    assert_eq!(w, vec![Some((2, 3))]);
    // Escaping round trip of the printer.
    assert_eq!(pattern_of(&lit("a\r\n")), "a\\r\\n");
}

// ---------------------------------------------------------------------------------------------
// AC-2: expansion
// ---------------------------------------------------------------------------------------------

/// Naive expansion rule (Interface, "Expansion"). `names[i]` is the name of group `i`.
fn model_expand(
    group_count: usize,
    names: &[Option<&str>],
    caps: &Captures,
    text: &str,
    template: &str,
) -> Result<String, String> {
    let t: Vec<char> = template.chars().collect();
    let tc: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let is_name_char = |c: char| c.is_ascii_alphanumeric() || c == '_';
    while i < t.len() {
        if t[i] != '$' {
            out.push(t[i]);
            i += 1;
            continue;
        }
        if t.get(i + 1) == Some(&'$') {
            out.push('$');
            i += 2;
            continue;
        }
        let name: String;
        if t.get(i + 1) == Some(&'{') {
            match (i + 2..t.len()).find(|&j| t[j] == '}') {
                Some(j) => {
                    name = t[i + 2..j].iter().collect();
                    i = j + 1;
                }
                None => {
                    out.push('$');
                    i += 1;
                    continue;
                }
            }
        } else {
            let mut j = i + 1;
            while j < t.len() && is_name_char(t[j]) {
                j += 1;
            }
            if j == i + 1 {
                out.push('$');
                i += 1;
                continue;
            }
            name = t[i + 1..j].iter().collect();
            i = j;
        }
        // All ASCII digits and parses as usize: a group number; otherwise a group name.
        let number = if name.chars().all(|c| c.is_ascii_digit()) {
            name.parse::<usize>().ok()
        } else {
            None
        };
        let index = match number {
            Some(k) if k <= group_count => k,
            Some(_) => return Err(name),
            None => match names.iter().position(|n| *n == Some(name.as_str())) {
                Some(k) => k,
                None => return Err(name),
            },
        };
        if let Some(Some(m)) = caps.groups.get(index)
            && m.start <= m.end
            && m.end <= tc.len()
        {
            out.extend(&tc[m.start..m.end]);
        }
    }
    Ok(out)
}

struct ExpandCase {
    pattern: &'static str,
    group_count: usize,
    names: &'static [Option<&'static str>],
}

const EXPAND_CASES: &[ExpandCase] = &[
    ExpandCase {
        pattern: "(?<w>a)(b)?",
        group_count: 2,
        names: &[None, Some("w"), None],
    },
    ExpandCase {
        pattern: "",
        group_count: 0,
        names: &[None],
    },
    ExpandCase {
        pattern: "(a)(?<x>b)(?<_>c)?(?:d)",
        group_count: 3,
        names: &[None, None, Some("x"), Some("_")],
    },
    ExpandCase {
        pattern: "(?<a9>a)|(?<w_x>[^a])",
        group_count: 2,
        names: &[None, Some("a9"), Some("w_x")],
    },
    ExpandCase {
        pattern: "((((((((((.))))))))))(é)?",
        group_count: 11,
        names: &[None; 12],
    },
];

const TEMPLATE_ALPHABET: &[char] = &[
    '$', '$', '$', '$', '{', '}', '0', '1', '9', 'a', 'w', '_', 'x', '!', 'é', '😀',
];
const EXPAND_TEXT_ALPHABET: &[char] = &['a', 'b', 'c', 'd', 'é', '😀', '\r', '\n'];

/// AC-2: random templates and random captures (including too few or too many groups,
/// `start > end`, ranges past the end of `text`), `expand` equals the naive rule, `Ok` text and
/// `Err` reference alike; `Display` of the error is pinned; nothing panics.
#[test]
fn expansion_equals_naive_model() {
    let mut rng = Rng::new(0x7017_0003);
    let mut oks = 0usize;
    let mut errs = 0usize;
    let mut nonempty_ok = 0usize;
    for case in EXPAND_CASES {
        let re = compile(case.pattern, false);
        assert_eq!(re.group_count(), case.group_count, "{:?}", case.pattern);
        for i in 0..4_000 {
            let tlen = rng.below(7);
            let text: String = (0..tlen).map(|_| rng.pick(EXPAND_TEXT_ALPHABET)).collect();
            let n_chars = text.chars().count();
            let caps = if rng.below(10) < 3 {
                let found = re.find_all(&text);
                if found.is_empty() {
                    Captures {
                        groups: vec![None; case.group_count + 1],
                    }
                } else {
                    found[rng.below(found.len())].clone()
                }
            } else {
                let len = rng.below(case.group_count + 4);
                Captures {
                    groups: (0..len)
                        .map(|_| {
                            if rng.below(4) == 0 {
                                None
                            } else {
                                Some(cm(rng.below(n_chars + 3), rng.below(n_chars + 3)))
                            }
                        })
                        .collect(),
                }
            };
            let tl = rng.below(13);
            let template: String = (0..tl).map(|_| rng.pick(TEMPLATE_ALPHABET)).collect();
            let expected = model_expand(case.group_count, case.names, &caps, &text, &template);
            let got = re.expand(&caps, &text, &template);
            match (&got, &expected) {
                (Ok(g), Ok(e)) => {
                    assert_eq!(
                        g, e,
                        "{} #{i}: expand({template:?}) caps={caps:?} text={text:?}",
                        case.pattern
                    );
                    oks += 1;
                    if !e.is_empty() {
                        nonempty_ok += 1;
                    }
                }
                (Err(g), Err(e)) => {
                    assert_eq!(
                        g.reference, *e,
                        "{} #{i}: expand({template:?}) reference",
                        case.pattern
                    );
                    assert_eq!(g.to_string(), format!("no capture group \"{e}\""));
                    errs += 1;
                }
                _ => panic!(
                    "{} #{i}: expand({template:?}) caps={caps:?} text={text:?}: got {got:?}, \
                     expected {expected:?}",
                    case.pattern
                ),
            }
            // Whether a template is an error depends only on the pattern and the template.
            let empty_caps = Captures { groups: Vec::new() };
            assert_eq!(
                re.expand(&empty_caps, "", &template).is_err(),
                expected.is_err(),
                "{} #{i}: error status of {template:?} must not depend on caps or text",
                case.pattern
            );
        }
    }
    assert!(oks > 2_000 && errs > 2_000, "oks={oks} errs={errs}");
    assert!(nonempty_ok > 1_000, "nonempty_ok={nonempty_ok}");
}

/// AC-2 edge cases for captures that do not fit `text` or the pattern (each expands to
/// nothing), with a group that exists in the pattern.
#[test]
fn expansion_ignores_ranges_that_do_not_fit() {
    let re = compile("(a)(b)", false);
    let text = "é😀a";
    let run = |groups: Vec<Option<CharMatch>>, t: &str| {
        re.expand(&Captures { groups }, text, t)
            .expect("references to existing groups")
    };
    // In range: chars, not bytes.
    assert_eq!(run(vec![Some(cm(0, 3)), Some(cm(1, 3))], "<$1>"), "<😀a>");
    // start > end.
    assert_eq!(run(vec![Some(cm(0, 3)), Some(cm(2, 1))], "<$1>"), "<>");
    // end past the end of text (3 chars, 9 bytes).
    assert_eq!(run(vec![Some(cm(0, 3)), Some(cm(1, 4))], "<$1>"), "<>");
    assert_eq!(run(vec![Some(cm(0, 3)), Some(cm(5, 9))], "<$1>"), "<>");
    // Fewer groups than group_count() + 1.
    assert_eq!(run(vec![Some(cm(0, 3))], "<$2|$1|$0>"), "<||é😀a>");
    assert_eq!(run(vec![], "<$0$2>"), "<>");
    // More groups than group_count() + 1: index 3 is not a group of the pattern.
    let err = re
        .expand(
            &Captures {
                groups: vec![Some(cm(0, 1)); 5],
            },
            text,
            "$3",
        )
        .unwrap_err();
    assert_eq!(
        err,
        TemplateError {
            reference: "3".into()
        }
    );
    // Empty text, ranges at 0..0 are fine.
    assert_eq!(
        re.expand(
            &Captures {
                groups: vec![Some(cm(0, 0)); 3]
            },
            "",
            "[$1]"
        )
        .unwrap(),
        "[]"
    );
}

// ---------------------------------------------------------------------------------------------
// AC-3: offsets are chars
// ---------------------------------------------------------------------------------------------

/// AC-3: random patterns over a text alphabet with 2-, 3- and 4-byte chars and CRLF: every
/// `CharMatch` equals the model's char offsets, and slicing `text` by chars gives the text of
/// each group as the model computes it.
#[test]
fn offsets_are_chars() {
    for ci in [false, true] {
        let mut g = Gen {
            rng: Rng::new(0x7017_0004 + u64::from(ci)),
            alphabet: AC3_ALPHABET,
        };
        for i in 0..3_000 {
            let node = g.alt(2);
            let pattern = pattern_of(&node);
            let text = g.text(24);
            let re = compile(&pattern, ci);
            let (_, expected) = model_find_all(&node, &text, ci);
            let got = re.find_all(&text);
            assert_eq!(
                got.iter().map(spans).collect::<Vec<_>>(),
                expected,
                "#{i}: find_all({pattern:?}, ci={ci}) on {text:?}"
            );
            let chars: Vec<char> = text.chars().collect();
            for (c, e) in got.iter().zip(&expected) {
                for (m, em) in c.groups.iter().zip(e) {
                    if let (Some(m), Some((s, t))) = (m, em) {
                        let sliced: String =
                            text.chars().skip(m.start).take(m.end - m.start).collect();
                        let model: String = chars[*s..*t].iter().collect();
                        assert_eq!(sliced, model);
                    }
                }
            }
        }
    }

    // Constructed oracle: the offset of a needle after a known prefix is the prefix's char count.
    let prefixes = [
        "",
        "é",
        "中",
        "😀",
        "\r\n",
        "é中😀\r\n",
        "a\rb\nc\r\n😀😀中é",
    ];
    for prefix in prefixes {
        for needle in ["x", "中", "😀y", "\r\n", "é\r"] {
            let text = format!("{prefix}{needle}{prefix}");
            let re = compile(&escape(needle), false);
            let start = prefix.chars().count();
            let end = start + needle.chars().count();
            let got = whole(&re, &text);
            // The needle may also occur inside the prefix; the planted one is at `start`.
            assert!(
                got.contains(&(start, end)),
                "{needle:?} after {prefix:?}: expected {:?} in {got:?}",
                (start, end)
            );
            for (s, e) in got {
                let sliced: String = text.chars().skip(s).take(e - s).collect();
                assert_eq!(sliced, needle);
            }
        }
    }

    // Groups too: `(中)(😀+)` after a multi-byte prefix.
    let re = compile("(中)(😀+)", false);
    let text = "é\r\n中😀😀z";
    assert_eq!(
        all_spans(&re, text),
        vec![vec![Some((3, 6)), Some((3, 4)), Some((4, 6))]]
    );
    // The empty pattern matches at every char position (not byte position).
    let re = compile("", false);
    assert_eq!(
        whole(&re, "é中😀\r\n"),
        vec![(0, 0), (1, 1), (2, 2), (3, 3), (4, 4), (5, 5)]
    );
}

/// Naive non-overlapping literal search (needle non-empty), with the AC-1 case predicate.
fn literal_find_all(text: &[char], needle: &[char], ci: bool) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + needle.len() <= text.len() {
        let hit = needle
            .iter()
            .zip(&text[i..])
            .all(|(&p, &c)| if ci { ci_eq(p, c) } else { p == c });
        if hit {
            out.push((i, i + needle.len()));
            i += needle.len();
        } else {
            i += 1;
        }
    }
    out
}

/// AC-3 (constructed oracle): a pattern with every metacharacter escaped gives the same
/// matches as a naive literal search, in char offsets, in both case modes.
#[test]
fn escaped_literal_matches_literal_search() {
    const META: &[char] = &[
        '\\', '.', '+', '*', '?', '(', ')', '|', '[', ']', '{', '}', '^', '$', '#', '&', '-', '~',
    ];
    let mut rng = Rng::new(0x7017_0005);
    let alphabet: Vec<char> = META.iter().chain(AC1_ALPHABET).copied().collect();
    let mut hits = 0usize;
    for i in 0..3_000 {
        let ci = i % 2 == 1;
        let nl = 1 + rng.below(4);
        let needle: Vec<char> = (0..nl).map(|_| rng.pick(&alphabet)).collect();
        let mut text: Vec<char> = Vec::new();
        for _ in 0..rng.below(6) {
            for _ in 0..rng.below(4) {
                text.push(rng.pick(&alphabet));
            }
            if rng.below(2) == 0 {
                text.extend(&needle);
            }
        }
        let needle_s: String = needle.iter().collect();
        let text_s: String = text.iter().collect();
        let re = compile(&escape(&needle_s), ci);
        let expected = literal_find_all(&text, &needle, ci);
        hits += expected.len();
        assert_eq!(
            whole(&re, &text_s),
            expected,
            "#{i}: literal {needle_s:?} (ci={ci}) in {text_s:?}"
        );
    }
    assert!(hits > 2_000, "hits={hits}");
}

// ---------------------------------------------------------------------------------------------
// AC-4: errors; Pinned (e): pathological patterns
// ---------------------------------------------------------------------------------------------

/// The engine's own message for `pattern` with the spec's modes (the spec: "`message` is the
/// engine's `Display`").
fn engine_message(pattern: &str, ci: bool) -> String {
    match regex::RegexBuilder::new(pattern)
        .multi_line(true)
        .crlf(true)
        .case_insensitive(ci)
        .build()
    {
        Ok(_) => panic!("engine accepts {pattern:?}"),
        Err(e) => e.to_string(),
    }
}

/// The control protocol's cut: at most 200 chars, else the first 199 then `…`.
fn cut(s: &str) -> String {
    if s.chars().count() > 200 {
        let mut t: String = s.chars().take(199).collect();
        t.push('…');
        t
    } else {
        s.to_string()
    }
}

fn pattern_error(pattern: &str, ci: bool) -> PatternError {
    match Regex::new(pattern, opts(ci)) {
        Ok(re) => panic!("{pattern:?} must be a PatternError, got {re:?}"),
        Err(e) => e,
    }
}

fn nested(depth: usize) -> String {
    format!("{}a{}", "(".repeat(depth), ")".repeat(depth))
}

/// AC-4: each named invalid pattern is `Err(PatternError)` with a non-empty message of at most
/// 200 chars and `Display == message`; a long engine message is cut to exactly its first 199
/// chars followed by `…`, never splitting a char.
#[test]
fn invalid_patterns_are_errors() {
    let long_a = format!("({}", "a".repeat(300));
    let long_e = format!("({}", "é".repeat(300));
    let long_emoji = format!("({}", "😀".repeat(250));
    for ci in [false, true] {
        for p in ["(", "[a-", "a{99999999}", "\\p{Nope}", nested(300).as_str()] {
            let e = pattern_error(p, ci);
            let n = e.message.chars().count();
            assert!(n > 0 && n <= 200, "{p:?}: message of {n} chars");
            assert_eq!(e.to_string(), e.message);
            let _: &dyn std::error::Error = &e;
        }
        // 300 nested groups exceed the nest limit of 250; its engine message is long.
        let p = nested(300);
        let e = pattern_error(&p, ci);
        assert_eq!(e.message, cut(&engine_message(&p, ci)));
        // Long engine messages: exactly the first 199 chars, then `…`.
        for p in [&long_a, &long_e, &long_emoji] {
            let engine = engine_message(p, ci);
            assert!(engine.chars().count() > 200, "probe must be long");
            let e = pattern_error(p, ci);
            let expected: String = engine.chars().take(199).chain(['…']).collect();
            assert_eq!(e.message, expected, "{p:?}");
            assert_eq!(e.message.chars().count(), 200);
        }
    }
    // The nest limit itself: 250 levels compile, deeper ones are errors (non-capturing groups,
    // so the group cap of ruling s1 does not interfere).
    let nc = |d: usize| format!("{}a{}", "(?:".repeat(d), ")".repeat(d));
    assert_eq!(compile(&nc(250), false).group_count(), 0);
    let e = pattern_error(&nc(251), false);
    assert_eq!(e.message, cut(&engine_message(&nc(251), false)));
    // Ruling s1: 250 nested capture groups pass the crate, then hit the cap; 251 keep the
    // crate's error.
    assert_eq!(pattern_error(&nested(250), false).message, CAP_MESSAGE);
    let e = pattern_error(&nested(251), false);
    assert_eq!(e.message, cut(&engine_message(&nested(251), false)));
}

/// Ruling s1 (D-060): the exact message for more than 100 explicit capture groups.
const CAP_MESSAGE: &str = "the pattern has more than 100 capture groups";

/// Ruling s1: exactly 100 explicit capture groups (named ones included) compile and work;
/// non-capturing groups do not count.
#[test]
fn hundred_capture_groups_are_accepted() {
    for ci in [false, true] {
        // 99 plain + 1 named = 100 explicit groups, plus 50 non-capturing groups.
        let pattern = format!("{}(?P<n>b){}", "(a)".repeat(99), "(?:c)".repeat(50));
        let re = compile(&pattern, ci);
        assert_eq!(re.group_count(), 100, "ci={ci}");
        let text = format!("é{}b{}😀", "a".repeat(99), "c".repeat(50));
        let found = re.find_all(&text);
        assert_eq!(found.len(), 1, "ci={ci}");
        let mut expected = vec![Some(cm(1, 151))];
        expected.extend((1..=100).map(|g| Some(cm(g, g + 1))));
        assert_eq!(found[0].groups, expected, "ci={ci}");
        assert_eq!(
            re.expand(&found[0], &text, "${n}$100$1"),
            Ok("bba".to_string())
        );
    }
}

/// Ruling s1: 101 explicit groups are refused with the exact message, whether the 101st is
/// plain or named; non-capturing groups alongside do not change that.
#[test]
fn more_than_hundred_capture_groups_are_refused() {
    for ci in [false, true] {
        for pattern in [
            "(a)".repeat(101),
            format!("{}(?P<n>b)", "(a)".repeat(100)),
            format!("{}(?:c){}", "(a)".repeat(50), "(a)".repeat(51)),
            format!("({})", "(a)".repeat(100)),
        ] {
            let e = pattern_error(&pattern, ci);
            assert_eq!(e.message, CAP_MESSAGE, "ci={ci}");
            assert_eq!(e.to_string(), CAP_MESSAGE);
        }
    }
}

/// Ruling s1: the 34,000-group pattern (100 kB) is refused with the cap message within 5 s.
#[test]
fn huge_group_pattern_is_refused_within_five_seconds() {
    let big = "(a)".repeat(34_000);
    let e = within_five_seconds("(a) x 34,000", || pattern_error(&big, false));
    assert_eq!(e.message, CAP_MESSAGE);
}

/// Ruling s1 precedence: a pattern the crate rejects keeps the crate's error even when it also
/// has more than 100 groups.
#[test]
fn crate_error_takes_precedence_over_group_cap() {
    for ci in [false, true] {
        for pattern in [
            format!("{}(", "(a)".repeat(101)),
            format!("{}[a-", "(a)".repeat(150)),
            format!("{}\\p{{Nope}}", "(a)".repeat(101)),
        ] {
            let e = pattern_error(&pattern, ci);
            assert_ne!(e.message, CAP_MESSAGE);
            assert_eq!(e.message, cut(&engine_message(&pattern, ci)), "ci={ci}");
        }
    }
}

/// AC-4 boundary: a message of exactly 200 chars is not cut, one of 201 is. The probes are
/// found by searching pattern lengths against the engine's own message length.
#[test]
fn error_message_cut_boundary() {
    let mut found = HashSet::new();
    for family in ["(", "[", "a{"] {
        for k in 0..400 {
            let p = format!("{family}{}", "b".repeat(k));
            let engine = engine_message(&p, false);
            let n = engine.chars().count();
            if n == 200 || n == 201 {
                let e = pattern_error(&p, false);
                assert_eq!(e.message, cut(&engine), "{p:?} ({n} chars)");
                if n == 200 {
                    assert_eq!(e.message, engine);
                } else {
                    assert!(e.message.ends_with('…'));
                }
                found.insert(n);
            }
        }
    }
    assert!(!found.is_empty(), "no probe of 200 or 201 chars found");
}

fn within_five_seconds<T>(what: &str, f: impl FnOnce() -> T) -> T {
    let t = Instant::now();
    let r = f();
    let d = t.elapsed();
    assert!(d < Duration::from_secs(5), "{what} took {d:?}");
    r
}

/// Pinned (e): pathological patterns answer or error within 5 s in a debug build, no panic.
#[test]
fn pathological_patterns_within_five_seconds() {
    let a100k = "a".repeat(100_000);
    let re = within_five_seconds("compile (a*)*b", || compile("(a*)*b", false));
    let got = within_five_seconds("(a*)*b on 100,000 a", || re.find_all(&a100k));
    assert!(got.is_empty());

    within_five_seconds("\\p{L}{1000}", || {
        match Regex::new("\\p{L}{1000}", opts(false)) {
            Ok(re) => {
                let _ = re.find_all(&"é".repeat(2_000));
            }
            Err(e) => assert!(!e.message.is_empty() && e.message.chars().count() <= 200),
        }
    });

    within_five_seconds("1,000 nested groups", || {
        let e = pattern_error(&nested(1_000), false);
        assert!(e.message.chars().count() <= 200);
    });

    let big = "(a)".repeat(34_000);
    within_five_seconds("100 kB pattern", || match Regex::new(&big, opts(false)) {
        Ok(re) => {
            assert_eq!(re.group_count(), 34_000);
            // A text without a match: with a match, the engine's capture search for 34,000
            // groups needs about 55 GB (observed with regex 1.13.1; see the test-author log).
            assert!(re.find_all(&"ab".repeat(1_000)).is_empty());
        }
        Err(e) => assert!(e.message.chars().count() <= 200),
    });
}

// ---------------------------------------------------------------------------------------------
// AC-5: worked examples (Pinned (g)); semantics spot checks
// ---------------------------------------------------------------------------------------------

/// AC-5: each worked example of Pinned (g) holds exactly.
#[test]
fn worked_examples_hold() {
    // `a*` on `baaab`.
    assert_eq!(
        whole(&compile("a*", false), "baaab"),
        vec![(0, 0), (1, 4), (5, 5)]
    );
    // `^.|.$` on `ab\rcd\r\nef\ngh`.
    let text = "ab\rcd\r\nef\ngh";
    let got = whole(&compile("^.|.$", false), text);
    let expected: Vec<(usize, usize)> = [0, 1, 3, 4, 7, 8, 10, 11]
        .iter()
        .map(|&s| (s, s + 1))
        .collect();
    assert_eq!(got, expected);
    // `.+` on the same text.
    assert_eq!(
        whole(&compile(".+", false), text),
        vec![(0, 2), (3, 5), (7, 9), (10, 12)]
    );
    // Case-insensitive `σ|ſ|k` on `ΣςSKK` (last = U+212A).
    assert_eq!(
        whole(&compile("σ|ſ|k", true), "ΣςSK\u{212A}"),
        vec![(0, 1), (1, 2), (2, 3), (3, 4), (4, 5)]
    );
    // ... and case-sensitively none of the five.
    assert_eq!(whole(&compile("σ|ſ|k", false), "ΣςSK\u{212A}"), vec![]);
    // `x*` on `é\r\n`: empty matches at 0, 1, 2, 3.
    assert_eq!(
        whole(&compile("x*", false), "é\r\n"),
        vec![(0, 0), (1, 1), (2, 2), (3, 3)]
    );
    // `b|(?:)|a` on `ba`.
    assert_eq!(
        whole(&compile("b|(?:)|a", false), "ba"),
        vec![(0, 1), (2, 2)]
    );
    // Captures in repetitions.
    assert_eq!(
        all_spans(&compile("(?:(a)|b)*", false), "ab"),
        vec![vec![Some((0, 2)), Some((0, 1))]]
    );
    assert_eq!(
        all_spans(&compile("((a)|b)+", false), "ab"),
        vec![vec![Some((0, 2)), Some((1, 2)), Some((0, 1))]]
    );
    assert_eq!(
        all_spans(&compile("(?:(a)|(b))+", false), "ab"),
        vec![vec![Some((0, 2)), Some((0, 1)), Some((1, 2))]]
    );

    // Expansion with `(?<w>a)(b)?` matched on `ac`.
    let re = compile("(?<w>a)(b)?", false);
    assert_eq!(re.group_count(), 2);
    let found = re.find_all("ac");
    assert_eq!(found.len(), 1);
    let caps = &found[0];
    assert_eq!(caps.groups, vec![Some(cm(0, 1)), Some(cm(0, 1)), None]);
    assert_eq!(
        re.expand(caps, "ac", "[$1|$2|${w}|$w|$0|$$|$|$!]"),
        Ok("[a||a|a|a|$|$|$!]".to_string())
    );
    assert_eq!(re.expand(caps, "ac", "${ab"), Ok("${ab".to_string()));
    for (template, reference) in [
        ("$9", "9"),
        ("$x!", "x"),
        ("${}", ""),
        ("$1a", "1a"),
        ("${a!}", "a!"),
        ("${1 }", "1 "),
        ("$99999999999999999999", "99999999999999999999"),
        ("$w$9", "9"),
    ] {
        let err = re.expand(caps, "ac", template);
        assert_eq!(
            err,
            Err(TemplateError {
                reference: reference.to_string()
            }),
            "{template:?}"
        );
        let err = err.unwrap_err();
        assert_eq!(err.to_string(), format!("no capture group \"{reference}\""));
        let _: &dyn std::error::Error = &err;
    }
    // `$12x` (named in the Interface's list of differences) is the reference `12x`.
    assert_eq!(
        re.expand(caps, "ac", "$12x"),
        Err(TemplateError {
            reference: "12x".into()
        })
    );
}

/// Interface: derives and defaults the dependents rely on (T-021 holds a `Regex`).
#[test]
fn api_contract() {
    fn clone_debug<T: Clone + std::fmt::Debug>(_: &T) {}
    let re = compile("(a)(?<n>b)", false);
    clone_debug(&re);
    let copy = re.clone();
    assert_eq!(copy.group_count(), 2);
    assert_eq!(all_spans(&copy, "xab"), all_spans(&re, "xab"));
    assert_eq!(RegexOptions::default(), opts(false));
    // CharMatch orders by start, then end.
    let mut v = vec![cm(2, 3), cm(0, 4), cm(0, 1)];
    v.sort();
    assert_eq!(v, vec![cm(0, 1), cm(0, 4), cm(2, 3)]);
    // `len == group_count() + 1` also with no match of an optional group.
    let re = compile("(a)|(b)|(c)", false);
    for c in re.find_all("cba") {
        assert_eq!(c.groups.len(), 4);
    }
    assert_eq!(
        all_spans(&re, "cba"),
        vec![
            vec![Some((0, 1)), None, None, Some((0, 1))],
            vec![Some((1, 2)), None, Some((1, 2)), None],
            vec![Some((2, 3)), Some((2, 3)), None, None],
        ]
    );
}

/// Semantics: line anchors and `.` follow Kiln's line model; inline flags override options
/// and modes for their scope, as in the crate.
#[test]
fn line_model_and_inline_flags() {
    // `$` never between `\r` and `\n`; `^` never between them either.
    assert_eq!(whole(&compile("$", false), "a\r\nb"), vec![(1, 1), (4, 4)]);
    assert_eq!(whole(&compile("^", false), "a\r\nb"), vec![(0, 0), (3, 3)]);
    // Lone `\r` is a terminator for both anchors.
    assert_eq!(
        whole(&compile("^", false), "a\rb\r"),
        vec![(0, 0), (2, 2), (4, 4)]
    );
    assert_eq!(
        whole(&compile("$", false), "a\rb\r"),
        vec![(1, 1), (3, 3), (4, 4)]
    );
    // `.` excludes `\r` and `\n`; `(?s)` includes them.
    assert_eq!(whole(&compile(".", false), "\r\n"), vec![]);
    assert_eq!(
        whole(&compile("(?s).", false), "\r\n"),
        vec![(0, 1), (1, 2)]
    );
    // `(?-m)`: `^` only at the start of the text.
    assert_eq!(whole(&compile("(?-m)^a", false), "a\na"), vec![(0, 1)]);
    // `(?-R)`: a lone `\r` is no longer a terminator.
    assert_eq!(whole(&compile("^b", false), "a\rb"), vec![(2, 3)]);
    assert_eq!(whole(&compile("(?-R)^b", false), "a\rb"), vec![]);
    // `(?i)` / `(?-i)` override the option for their scope.
    assert_eq!(whole(&compile("(?i)k", false), "K"), vec![(0, 1)]);
    assert_eq!(whole(&compile("(?-i)k", true), "K"), vec![]);
    assert_eq!(whole(&compile("a(?-i:b)", true), "ABAb"), vec![(2, 4)]);
    // Case folding is simple folding: `ſ` matches `s` and `S`, `(?i)[^a]` excludes `A`.
    assert_eq!(whole(&compile("ſ", true), "sSx"), vec![(0, 1), (1, 2)]);
    assert_eq!(whole(&compile("[^a]", true), "aAb"), vec![(2, 3)]);
    // Anchors see the whole text after a match: `^a` after `ba` is not at a line start.
    assert_eq!(whole(&compile("b|^a", false), "ba"), vec![(0, 1)]);
}

// ---------------------------------------------------------------------------------------------
// AC-6: quantified sub-patterns that can match empty (hand-written expectations)
// ---------------------------------------------------------------------------------------------

/// AC-6: each row of the Pinned (g) table holds exactly, in both case modes.
#[test]
fn nullable_quantifier_examples_hold() {
    for ci in [false, true] {
        assert_eq!(
            all_spans(&compile("(?:a|())*", ci), "aa"),
            vec![vec![Some((0, 2)), None]],
            "ci={ci}"
        );
        assert_eq!(
            all_spans(&compile("(|a)+", ci), "aa"),
            vec![
                vec![Some((0, 0)), Some((0, 0))],
                vec![Some((1, 1)), Some((1, 1))],
                vec![Some((2, 2)), Some((2, 2))],
            ],
            "ci={ci}"
        );
        let mut expected = vec![vec![Some((0, 1)), Some((0, 1))]];
        for s in 2..=8 {
            expected.push(vec![Some((s, s)), Some((s, s))]);
        }
        assert_eq!(
            all_spans(&compile("( *.??)+", ci), " k0σb0ks"),
            expected,
            "ci={ci}"
        );
        assert_eq!(
            all_spans(&compile("(^)*", ci), "éS"),
            vec![
                vec![Some((0, 0)), Some((0, 0))],
                vec![Some((1, 1)), None],
                vec![Some((2, 2)), None],
            ],
            "ci={ci}"
        );
        assert_eq!(
            all_spans(&compile("(a*)*", ci), "b"),
            vec![
                vec![Some((0, 0)), Some((0, 0))],
                vec![Some((1, 1)), Some((1, 1))],
            ],
            "ci={ci}"
        );
    }
}
