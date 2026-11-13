//! Integration tests of `kiln_search::query` (T-021, implementer's own tests).

use kiln_search::{CharMatch, Query, QueryError};

fn query(pattern: &str, regex: bool, case_sensitive: bool, whole_word: bool) -> Query {
    Query {
        pattern: pattern.to_owned(),
        regex,
        case_sensitive,
        whole_word,
    }
}

fn spans(ms: &[CharMatch]) -> Vec<(usize, usize)> {
    ms.iter().map(|m| (m.start, m.end)).collect()
}

/// SplitMix64.
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        usize::try_from((z ^ (z >> 31)) % n as u64).expect("fits")
    }
}

/// Literal mode treats every regex metacharacter as a plain char: random texts and patterns
/// over an alphabet of metacharacters agree with a naive case-sensitive scan.
#[test]
fn literal_metacharacters_match_a_naive_scan() {
    const A: &[char] = &[
        '.', '*', '(', ')', '[', ']', '\\', '$', '^', '|', '?', '+', '{', '}', 'a',
    ];
    let mut rng = Rng(21);
    for _ in 0..2_000 {
        let text: Vec<char> = (0..rng.below(12)).map(|_| A[rng.below(A.len())]).collect();
        let pat: Vec<char> = (0..1 + rng.below(3))
            .map(|_| A[rng.below(A.len())])
            .collect();
        let mut want = Vec::new();
        let mut i = 0;
        while i + pat.len() <= text.len() {
            if text[i..i + pat.len()] == pat[..] {
                want.push((i, i + pat.len()));
                i += pat.len();
            } else {
                i += 1;
            }
        }
        let p: String = pat.iter().collect();
        let t: String = text.iter().collect();
        let found = query(&p, false, true, false)
            .compile()
            .unwrap()
            .find(&t, 100);
        assert_eq!(spans(&found.matches), want, "{p:?} in {t:?}");
        assert_eq!(found.total, want.len());
    }
}

/// Whole-word boundaries use `_` and Unicode alphanumerics (digits, other scripts).
#[test]
fn whole_word_boundaries_are_unicode() {
    let q = query("ab", false, true, true).compile().unwrap();
    let f = q.find("ab éab ab_ ab1 1ab 中ab (ab) ab", 100);
    assert_eq!(spans(&f.matches), [(0, 2), (24, 26), (28, 30)]);
    // Regex mode: a rejected match is not re-scanned (`a+` at 0..2 is dropped, not 1..2).
    let q = query("a+", true, true, true).compile().unwrap();
    assert_eq!(q.find("aab a", 100).total, 1);
    assert_eq!(
        q.find("_aa a", 100).matches,
        [CharMatch { start: 4, end: 5 }]
    );
}

/// `replace_first` is the first element of `replace_all` at or after `from`.
#[test]
fn replace_first_is_the_first_of_replace_all_from_an_offset() {
    let q = query("(é|😀)x?", true, false, false).compile().unwrap();
    let text = "Éx a 😀 éé b😀x";
    let all = q.replace_all(text, "[$1]").unwrap();
    assert_eq!(all.len(), 5);
    for from in 0..=text.chars().count() + 1 {
        let want = all.iter().find(|(m, _)| m.start >= from).cloned();
        assert_eq!(
            q.replace_first(text, from, "[$1]").unwrap(),
            want,
            "from {from}"
        );
    }
}

/// 200,000 matches with references expand without walking the text per match: the debug
/// test finishes (a quadratic expansion would take about 8·10^10 char steps; no time is
/// asserted).
#[test]
fn replace_all_with_references_on_many_matches() {
    let text = "ab".repeat(200_000);
    let q = query("(a)(b)", true, true, false).compile().unwrap();
    let all = q.replace_all(&text, "$2$1").unwrap();
    assert_eq!(all.len(), 200_000);
    assert!(all.iter().all(|(_, s)| s == "ba"));
    assert_eq!(
        all[199_999].0,
        CharMatch {
            start: 399_998,
            end: 400_000
        }
    );
    assert_eq!(q.find(&text, 0).total, 200_000);
}

/// The cap counts chars in both modes and is checked before compiling.
#[test]
fn pattern_cap_counts_chars() {
    for regex in [false, true] {
        assert!(
            query(&"😀".repeat(1000), regex, true, false)
                .compile()
                .is_ok()
        );
        assert_eq!(
            query(&"😀".repeat(1001), regex, true, false)
                .compile()
                .err(),
            Some(QueryError::TooLong { chars: 1001 })
        );
    }
}
