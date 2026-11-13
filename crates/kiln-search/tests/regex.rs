//! Implementer's integration tests for `kiln_search::Regex` (T-017), through the public API.

use kiln_search::{Captures, CharMatch, PatternError, Regex, RegexOptions, TemplateError};
use std::time::{Duration, Instant};

const SENSITIVE: RegexOptions = RegexOptions {
    case_insensitive: false,
};
const INSENSITIVE: RegexOptions = RegexOptions {
    case_insensitive: true,
};

#[cfg(test)] // test helper: unwrap allowed
fn re(p: &str) -> Regex {
    Regex::new(p, SENSITIVE).unwrap()
}

fn r(start: usize, end: usize) -> Option<CharMatch> {
    Some(CharMatch { start, end })
}

#[cfg(test)] // test helper: unwrap allowed
fn wholes(re: &Regex, text: &str) -> Vec<(usize, usize)> {
    re.find_all(text)
        .iter()
        .map(|c| {
            let w = c.groups[0].unwrap();
            (w.start, w.end)
        })
        .collect()
}

fn starts(re: &Regex, text: &str) -> Vec<usize> {
    wholes(re, text).into_iter().map(|(s, _)| s).collect()
}

#[test]
fn worked_iteration_examples() {
    assert_eq!(wholes(&re("a*"), "baaab"), vec![(0, 0), (1, 4), (5, 5)]);
    let lines = "ab\rcd\r\nef\ngh";
    assert_eq!(starts(&re("^.|.$"), lines), vec![0, 1, 3, 4, 7, 8, 10, 11]);
    assert_eq!(
        wholes(&re(".+"), lines),
        vec![(0, 2), (3, 5), (7, 9), (10, 12)]
    );
    let ci = Regex::new("σ|ſ|k", INSENSITIVE).unwrap();
    assert_eq!(
        wholes(&ci, "ΣςSK\u{212A}"),
        vec![(0, 1), (1, 2), (2, 3), (3, 4), (4, 5)]
    );
    assert_eq!(
        wholes(&re("x*"), "é\r\n"),
        vec![(0, 0), (1, 1), (2, 2), (3, 3)]
    );
    assert_eq!(wholes(&re("b|(?:)|a"), "ba"), vec![(0, 1), (2, 2)]);
}

#[test]
fn captures_in_repetitions_keep_the_last_iteration() {
    let g = |p: &str| re(p).find_all("ab");
    assert_eq!(
        g("(?:(a)|b)*"),
        vec![Captures {
            groups: vec![r(0, 2), r(0, 1)]
        }]
    );
    assert_eq!(
        g("((a)|b)+"),
        vec![Captures {
            groups: vec![r(0, 2), r(1, 2), r(0, 1)]
        }]
    );
    assert_eq!(
        g("(?:(a)|(b))+"),
        vec![Captures {
            groups: vec![r(0, 2), r(0, 1), r(1, 2)]
        }]
    );
}

#[test]
fn nullable_quantifier_table_both_case_modes() {
    for options in [SENSITIVE, INSENSITIVE] {
        let f = |p: &str, t: &str| Regex::new(p, options).unwrap().find_all(t);
        let caps = |rows: &[(Option<CharMatch>, Option<CharMatch>)]| -> Vec<Captures> {
            rows.iter()
                .map(|&(w, g)| Captures { groups: vec![w, g] })
                .collect()
        };
        assert_eq!(f("(?:a|())*", "aa"), caps(&[(r(0, 2), None)]));
        assert_eq!(
            f("(|a)+", "aa"),
            caps(&[(r(0, 0), r(0, 0)), (r(1, 1), r(1, 1)), (r(2, 2), r(2, 2))])
        );
        let mut rows = vec![(r(0, 1), r(0, 1))];
        rows.extend((2..=8).map(|s| (r(s, s), r(s, s))));
        assert_eq!(f("( *.??)+", " k0σb0ks"), caps(&rows));
        assert_eq!(
            f("(^)*", "éS"),
            caps(&[(r(0, 0), r(0, 0)), (r(1, 1), None), (r(2, 2), None)])
        );
        assert_eq!(
            f("(a*)*", "b"),
            caps(&[(r(0, 0), r(0, 0)), (r(1, 1), r(1, 1))])
        );
    }
}

#[test]
fn expansion_examples() {
    let p = re("(?<w>a)(b)?");
    assert_eq!(p.group_count(), 2);
    let text = "ac";
    let caps = p.find_all(text).remove(0);
    assert_eq!(caps.groups, vec![r(0, 1), r(0, 1), None]);
    let ex = |t: &str| p.expand(&caps, text, t);
    assert_eq!(
        ex("[$1|$2|${w}|$w|$0|$$|$|$!]"),
        Ok("[a||a|a|a|$|$|$!]".to_owned())
    );
    assert_eq!(ex("${ab"), Ok("${ab".to_owned()));
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
        let err = ex(template).unwrap_err();
        assert_eq!(
            err,
            TemplateError {
                reference: reference.to_owned()
            },
            "{template}"
        );
        assert_eq!(err.to_string(), format!("no capture group \"{reference}\""));
    }
}

#[test]
fn expansion_tolerates_captures_that_do_not_fit() {
    let p = re("(a)(b)");
    let text = "é中";
    let odd = Captures {
        groups: vec![r(1, 2), r(2, 1), r(1, 3)],
    };
    assert_eq!(p.expand(&odd, text, "[$0|$1|$2]"), Ok("[中||]".to_owned()));
    let short = Captures { groups: vec![] };
    assert_eq!(p.expand(&short, text, "[$0$2]"), Ok("[]".to_owned()));
    // An error depends on the template only, not on the captures.
    assert!(p.expand(&short, text, "$3").is_err());
    let long = Captures {
        groups: vec![r(0, 1); 6],
    };
    assert_eq!(p.expand(&long, text, "$2"), Ok("é".to_owned()));
    assert!(p.expand(&long, text, "$5").is_err());
}

#[test]
fn invalid_patterns_are_pattern_errors() {
    let nested = format!("{}a{}", "(".repeat(300), ")".repeat(300));
    for p in ["(", "[a-", "a{99999999}", r"\p{Nope}", nested.as_str()] {
        let e: PatternError = Regex::new(p, SENSITIVE).unwrap_err();
        let n = e.message.chars().count();
        assert!(n > 0 && n <= 200, "{p}: {n}");
        assert_eq!(e.to_string(), e.message);
    }
    let long = format!("({}", "a".repeat(300));
    let full = ::regex::Regex::new(&long).unwrap_err().to_string();
    assert!(full.chars().count() > 200);
    let e = Regex::new(&long, SENSITIVE).unwrap_err();
    let expected: String = full.chars().take(199).chain(std::iter::once('…')).collect();
    assert_eq!(e.message, expected);
}

#[test]
fn empty_pattern_is_valid() {
    assert_eq!(starts(&re(""), "a😀\r\n"), vec![0, 1, 2, 3, 4]);
}

#[test]
fn inline_flags_override_the_modes() {
    assert_eq!(wholes(&re("(?s).+"), "a\r\nb"), vec![(0, 4)]);
    assert_eq!(starts(&re("(?-m)^."), "a\nb"), vec![0]);
    assert_eq!(starts(&re("(?i)σ"), "Σς"), vec![0, 1]);
    assert_eq!(
        starts(&Regex::new("(?-i)a", INSENSITIVE).unwrap(), "aA"),
        vec![0]
    );
    assert_eq!(
        starts(&Regex::new("(?i)[^a]", SENSITIVE).unwrap(), "aAb"),
        vec![2]
    );
}

/// The engine's own iterator, converted to chars by brute force.
#[cfg(test)] // test helper: unwrap allowed
fn engine_find_all(pattern: &str, ci: bool, text: &str) -> Vec<Captures> {
    let e = ::regex::RegexBuilder::new(pattern)
        .multi_line(true)
        .crlf(true)
        .case_insensitive(ci)
        .build()
        .unwrap();
    let ch = |b: usize| text[..b].chars().count();
    e.captures_iter(text)
        .map(|c| Captures {
            groups: (0..c.len())
                .map(|i| {
                    c.get(i).map(|m| CharMatch {
                        start: ch(m.start()),
                        end: ch(m.end()),
                    })
                })
                .collect(),
        })
        .collect()
}

/// Deterministic xorshift, so the run is reproducible.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[(self.next() % xs.len() as u64) as usize]
    }
}

#[test]
fn find_all_agrees_with_the_engines_iterator_and_offsets_are_chars() {
    let atoms = [
        "a", "é", "σ", "😀", "中", ".", "^", "$", r"\b", "", "(a)", "(é|)", "[a-z]", "[^a]", r"\r",
        r"\n", "(?:a|b)", "(x?)", "k", "ſ",
    ];
    let quants = ["", "", "*", "+", "?", "*?", "??", "{2}"];
    let chars = [
        'a', 'b', 'é', '中', '😀', '\r', '\n', 'Σ', 'ς', 'K', '\u{212A}', ' ', 'x',
    ];
    let mut rng = Rng(0x5eed_7017);
    for _ in 0..3000 {
        let mut pattern = String::new();
        for _ in 0..(1 + rng.next() % 4) {
            if !pattern.is_empty() && rng.next().is_multiple_of(5) {
                pattern.push('|');
            }
            pattern.push_str(rng.pick(&atoms));
            pattern.push_str(rng.pick(&quants));
        }
        let text: String = (0..rng.next() % 12).map(|_| *rng.pick(&chars)).collect();
        let ci = rng.next().is_multiple_of(2);
        let options = RegexOptions {
            case_insensitive: ci,
        };
        let Ok(ours) = Regex::new(&pattern, options) else {
            assert!(::regex::Regex::new(&pattern).is_err(), "{pattern}");
            continue;
        };
        let got = ours.find_all(&text);
        assert_eq!(
            got,
            engine_find_all(&pattern, ci, &text),
            "{pattern:?} on {text:?}"
        );
        for c in &got {
            assert_eq!(c.groups.len(), ours.group_count() + 1);
            for g in c.groups.iter().flatten() {
                assert!(g.start <= g.end && g.end <= text.chars().count());
            }
        }
        // `$0` expands to the matched text, which is the chars of the whole match.
        for c in &got {
            let w = c.groups[0].unwrap();
            let slice: String = text.chars().skip(w.start).take(w.end - w.start).collect();
            assert_eq!(ours.expand(c, &text, "$0"), Ok(slice));
        }
    }
}

#[test]
fn pathological_patterns_answer_quickly() {
    let limit = Duration::from_secs(5);
    let a = "a".repeat(100_000);
    let t = Instant::now();
    assert!(re("(a*)*b").find_all(&a).is_empty());
    assert!(t.elapsed() < limit);
    let t = Instant::now();
    let _ = Regex::new(r"\p{L}{1000}", SENSITIVE);
    assert!(t.elapsed() < limit);
    let t = Instant::now();
    let deep = format!("{}a{}", "(".repeat(1000), ")".repeat(1000));
    assert!(Regex::new(&deep, SENSITIVE).is_err());
    assert!(t.elapsed() < limit);
    let t = Instant::now();
    let big = "(a)".repeat(34_000);
    assert!(Regex::new(&big, SENSITIVE).is_err());
    assert!(t.elapsed() < limit);
}

#[test]
fn many_matches_are_linear() {
    let text = "é".repeat(100_000);
    let t = Instant::now();
    assert_eq!(re("").find_all(&text).len(), 100_001);
    assert_eq!(re("(é)").find_all(&text).len(), 100_000);
    assert!(t.elapsed() < Duration::from_secs(5));
}
