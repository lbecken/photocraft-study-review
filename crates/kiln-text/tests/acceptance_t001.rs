//! Independent acceptance tests for T-001 (buffer, positions, lines, line endings, undo/redo).
//!
//! Written from the task file's Interface and Acceptance criteria only. Every oracle is built
//! here: a naive `Vec<char>` text model, a naive CRLF-first line split, a naive per-step undo
//! model that applies the grouping rule, round-trip properties and exact values. All generators
//! are fixed-seed (SplitMix64), so every run is identical.

use std::ops::Range;

use kiln_text::{Buffer, EditError, LineCol, LineEnding, LineEndings, PositionError};

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

    /// Uniform in `0..n` (0 when `n == 0`).
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    /// True with probability `pct` percent.
    fn chance(&mut self, pct: u64) -> bool {
        self.next_u64() % 100 < pct
    }
}

/// The alphabet: ASCII, 2-, 3- and 4-byte chars, and the three line breaks. It also holds the
/// chars that some rope libraries treat as line breaks but the spec does not (VT, FF, NEL,
/// LINE SEPARATOR, PARAGRAPH SEPARATOR): under the Interface they are ordinary chars.
const PIECES: &[&str] = &[
    "a", "b", "Z", " ", "é", "ж", "€", "中", "😀", "🦀", "\n", "\r", "\r\n", "\u{0B}", "\u{0C}",
    "\u{85}", "\u{2028}", "\u{2029}",
];

/// Pieces that contain no line break.
const NON_BREAK: &[&str] = &[
    "a", "b", "Z", " ", "é", "ж", "€", "中", "😀", "🦀", "\u{0B}", "\u{0C}", "\u{85}", "\u{2028}",
    "\u{2029}",
];

/// Single chars that are not `\n` / `\r` (typing chars for the grouping rule).
const TYPING: &[&str] = &["a", "b", "Z", " ", "é", "€", "😀", "\u{2028}"];

fn gen_text(rng: &mut Rng, max_pieces: usize) -> String {
    let n = rng.below(max_pieces + 1);
    let mut s = String::new();
    for _ in 0..n {
        s.push_str(PIECES[rng.below(PIECES.len())]);
    }
    s
}

/// Text to insert: sometimes empty, often one piece, sometimes several.
fn gen_insert_text(rng: &mut Rng) -> String {
    match rng.below(10) {
        0 => String::new(),
        1..=5 => PIECES[rng.below(PIECES.len())].to_owned(),
        _ => {
            let n = 2 + rng.below(3);
            let mut s = String::new();
            for _ in 0..n {
                s.push_str(PIECES[rng.below(PIECES.len())]);
            }
            s
        }
    }
}

fn gen_nonempty_insert_text(rng: &mut Rng) -> String {
    let t = gen_insert_text(rng);
    if t.is_empty() { "x".to_owned() } else { t }
}

/// A buffer of the generator: half built with `from_text`, half through a series of edits (so
/// the storage's own construction and edit paths are both exercised). Returns it with the text.
fn gen_buffer(rng: &mut Rng, max_pieces: usize) -> (Buffer, String) {
    if rng.chance(50) {
        let t = gen_text(rng, max_pieces);
        (Buffer::from_text(&t), t)
    } else {
        let mut b = Buffer::new();
        let mut m: Vec<char> = Vec::new();
        let steps = rng.below(max_pieces + 1);
        for _ in 0..steps {
            if !m.is_empty() && rng.chance(25) {
                let s = rng.below(m.len() + 1);
                let e = s + rng.below(m.len() - s + 1);
                let removed = b.delete(s..e);
                let expected: String = m.drain(s..e).collect();
                assert_eq!(removed, Ok(expected));
            } else {
                let at = rng.below(m.len() + 1);
                let t = PIECES[rng.below(PIECES.len())];
                assert_eq!(b.insert(at, t), Ok(()));
                m.splice(at..at, t.chars());
            }
        }
        let t: String = m.iter().collect();
        assert_eq!(b.text(), t);
        (b, t)
    }
}

// ---------------------------------------------------------------------------------------------
// Naive model
// ---------------------------------------------------------------------------------------------

fn chars_of(s: &str) -> Vec<char> {
    s.chars().collect()
}

fn model_len_bytes(m: &[char]) -> usize {
    m.iter().map(|c| c.len_utf8()).sum()
}

fn model_len_utf16(m: &[char]) -> usize {
    m.iter().map(|c| c.len_utf16()).sum()
}

/// One line of the naive CRLF-first split: content is `start..end` (char indices).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Line {
    start: usize,
    end: usize,
    ending: Option<LineEnding>,
}

/// Split on `\r\n|\n|\r`, CRLF matched first.
fn model_lines(m: &[char]) -> Vec<Line> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < m.len() {
        match m[i] {
            '\r' if m.get(i + 1) == Some(&'\n') => {
                out.push(Line {
                    start,
                    end: i,
                    ending: Some(LineEnding::Crlf),
                });
                i += 2;
                start = i;
            }
            '\r' => {
                out.push(Line {
                    start,
                    end: i,
                    ending: Some(LineEnding::Cr),
                });
                i += 1;
                start = i;
            }
            '\n' => {
                out.push(Line {
                    start,
                    end: i,
                    ending: Some(LineEnding::Lf),
                });
                i += 1;
                start = i;
            }
            _ => i += 1,
        }
    }
    out.push(Line {
        start,
        end: m.len(),
        ending: None,
    });
    out
}

fn model_line_endings(m: &[char]) -> LineEndings {
    let (lf, crlf, cr) = model_break_counts(m);
    match (lf > 0, crlf > 0, cr > 0) {
        (false, false, false) => LineEndings::None,
        (true, false, false) => LineEndings::Lf,
        (false, true, false) => LineEndings::Crlf,
        (false, false, true) => LineEndings::Cr,
        _ => LineEndings::Mixed,
    }
}

fn model_break_counts(m: &[char]) -> (usize, usize, usize) {
    let mut counts = (0, 0, 0);
    for l in model_lines(m) {
        match l.ending {
            Some(LineEnding::Lf) => counts.0 += 1,
            Some(LineEnding::Crlf) => counts.1 += 1,
            Some(LineEnding::Cr) => counts.2 += 1,
            None => {}
        }
    }
    counts
}

/// Most frequent break kind; ties go to Lf, then Crlf, then Cr.
fn model_dominant(m: &[char]) -> Option<LineEnding> {
    let (lf, crlf, cr) = model_break_counts(m);
    if lf + crlf + cr == 0 {
        None
    } else if lf >= crlf && lf >= cr {
        Some(LineEnding::Lf)
    } else if crlf >= cr {
        Some(LineEnding::Crlf)
    } else {
        Some(LineEnding::Cr)
    }
}

/// Prefix offsets: `bytes[c]` and `utf16[c]` for every `c` in `0..=len`.
struct Prefix {
    bytes: Vec<usize>,
    utf16: Vec<usize>,
}

fn model_prefix(m: &[char]) -> Prefix {
    let mut bytes = Vec::with_capacity(m.len() + 1);
    let mut utf16 = Vec::with_capacity(m.len() + 1);
    let (mut b, mut u) = (0, 0);
    bytes.push(0);
    utf16.push(0);
    for c in m {
        b += c.len_utf8();
        u += c.len_utf16();
        bytes.push(b);
        utf16.push(u);
    }
    Prefix { bytes, utf16 }
}

fn model_char_to_byte(m: &[char], p: &Prefix, c: usize) -> Result<usize, PositionError> {
    if c > m.len() {
        Err(PositionError::OutOfBounds {
            index: c,
            len: m.len(),
        })
    } else {
        Ok(p.bytes[c])
    }
}

fn model_char_to_utf16(m: &[char], p: &Prefix, c: usize) -> Result<usize, PositionError> {
    if c > m.len() {
        Err(PositionError::OutOfBounds {
            index: c,
            len: m.len(),
        })
    } else {
        Ok(p.utf16[c])
    }
}

fn model_byte_to_char(p: &Prefix, byte: usize) -> Result<usize, PositionError> {
    let total = p.bytes[p.bytes.len() - 1];
    if byte > total {
        return Err(PositionError::OutOfBounds {
            index: byte,
            len: total,
        });
    }
    match p.bytes.binary_search(&byte) {
        Ok(c) => Ok(c),
        Err(_) => Err(PositionError::NotCharBoundary { byte }),
    }
}

fn model_utf16_to_char(p: &Prefix, utf16: usize) -> Result<usize, PositionError> {
    let total = p.utf16[p.utf16.len() - 1];
    if utf16 > total {
        return Err(PositionError::OutOfBounds {
            index: utf16,
            len: total,
        });
    }
    match p.utf16.binary_search(&utf16) {
        Ok(c) => Ok(c),
        Err(_) => Err(PositionError::InsideSurrogatePair { utf16 }),
    }
}

fn model_inside_crlf(m: &[char], c: usize) -> bool {
    c > 0 && c < m.len() && m[c - 1] == '\r' && m[c] == '\n'
}

fn model_char_to_line_col(m: &[char], lines: &[Line], c: usize) -> Result<LineCol, PositionError> {
    if c > m.len() {
        return Err(PositionError::OutOfBounds {
            index: c,
            len: m.len(),
        });
    }
    if model_inside_crlf(m, c) {
        return Err(PositionError::InsideLineBreak { char_idx: c });
    }
    let line = lines.partition_point(|l| l.start <= c) - 1;
    Ok(LineCol {
        line,
        col: c - lines[line].start,
    })
}

fn model_line_col_to_char(lines: &[Line], pos: LineCol) -> Result<usize, PositionError> {
    if pos.line >= lines.len() {
        return Err(PositionError::LineOutOfBounds {
            line: pos.line,
            len_lines: lines.len(),
        });
    }
    let l = &lines[pos.line];
    let line_len = l.end - l.start;
    if pos.col > line_len {
        return Err(PositionError::ColumnOutOfBounds {
            line: pos.line,
            col: pos.col,
            line_len,
        });
    }
    Ok(l.start + pos.col)
}

/// What the model expects from an edit whose range or index it validates.
#[derive(Debug, PartialEq)]
enum Expect {
    /// Accepted.
    Accept,
    /// Rejected with exactly this error.
    Reject(EditError),
    /// Both range bounds lie beyond the end (start <= end): the spec does not say which bound
    /// is reported, so the variant, `len_chars` and that the index is one of the two bounds
    /// are checked (see the QUESTION in the test author log).
    RejectEitherBound {
        start: usize,
        end: usize,
        len_chars: usize,
    },
}

fn expect_index(c: usize, len: usize) -> Expect {
    if c > len {
        Expect::Reject(EditError::OutOfBounds {
            index: c,
            len_chars: len,
        })
    } else {
        Expect::Accept
    }
}

fn expect_range(r: &Range<usize>, len: usize) -> Expect {
    if r.start > r.end {
        Expect::Reject(EditError::InvertedRange {
            start: r.start,
            end: r.end,
        })
    } else if r.start > len {
        Expect::RejectEitherBound {
            start: r.start,
            end: r.end,
            len_chars: len,
        }
    } else if r.end > len {
        Expect::Reject(EditError::OutOfBounds {
            index: r.end,
            len_chars: len,
        })
    } else {
        Expect::Accept
    }
}

fn assert_rejected_as(actual: &EditError, expected: &Expect, ctx: &str) {
    match expected {
        Expect::Accept => assert_eq!(Some(actual), None, "{ctx}: model accepts, buffer rejected"),
        Expect::Reject(e) => assert_eq!(actual, e, "{ctx}"),
        Expect::RejectEitherBound {
            start,
            end,
            len_chars,
        } => assert!(
            matches!(
                actual,
                EditError::OutOfBounds { index, len_chars: l }
                    if l == len_chars && (index == start || index == end)
            ),
            "{ctx}: expected OutOfBounds at {start} or {end} with len_chars {len_chars}, \
             got {actual:?}"
        ),
    }
}

/// Generate a range for a text of `len` chars, covering valid, empty, inverted and
/// out-of-range cases. Inverted ranges stay within bounds; out-of-range ranges are non-empty.
fn gen_range(rng: &mut Rng, len: usize) -> Range<usize> {
    match rng.below(20) {
        0 if len > 0 => {
            let start = 1 + rng.below(len);
            let end = rng.below(start);
            Range { start, end }
        }
        1 => {
            let start = rng.below(len + 1);
            let end = len + 1 + rng.below(3);
            start..end
        }
        2 => {
            let start = len + 1 + rng.below(3);
            let end = start + 1 + rng.below(3);
            start..end
        }
        3 => {
            let s = rng.below(len + 1);
            s..s
        }
        _ => {
            let s = rng.below(len + 1);
            let e = s + rng.below((len - s).min(12) + 1);
            s..e
        }
    }
}

fn assert_matches_model(b: &Buffer, m: &[char], ctx: &str) {
    let text: String = m.iter().collect();
    assert_eq!(b.text(), text, "{ctx}: text");
    assert_eq!(b.len_chars(), m.len(), "{ctx}: len_chars");
    assert_eq!(b.len_bytes(), model_len_bytes(m), "{ctx}: len_bytes");
    assert_eq!(b.len_utf16(), model_len_utf16(m), "{ctx}: len_utf16");
    assert_eq!(b.len_lines(), model_lines(m).len(), "{ctx}: len_lines");
    assert_eq!(b.is_empty(), m.is_empty(), "{ctx}: is_empty");
}

#[derive(Clone, Copy)]
enum Kind {
    Insert,
    Delete,
    Replace,
}

/// AC-1 driver: run `ops` random ops of the given kinds against the model, checking after every
/// op. Returns how many ops were rejected (to show the generator reaches the error paths).
fn run_differential(seed: u64, ops: usize, kinds: &[Kind]) -> usize {
    let mut rng = Rng::new(seed);
    let mut b = Buffer::new();
    let mut m: Vec<char> = Vec::new();
    let mut rejected = 0;
    for i in 0..ops {
        let ctx = format!("seed {seed} op {i}");
        let len = m.len();
        // keep the text from growing without bound
        let kind = if len > 300 && kinds.len() > 1 {
            Kind::Delete
        } else {
            kinds[rng.below(kinds.len())]
        };
        match kind {
            Kind::Insert => {
                let mut t = gen_insert_text(&mut rng);
                let at = if rng.chance(8) {
                    len + 1 + rng.below(3)
                } else {
                    rng.below(len + 1)
                };
                if at > len && t.is_empty() {
                    t = "q".to_owned(); // out-of-range "" is not specified (QUESTION)
                }
                let exp = expect_index(at, len);
                match (b.insert(at, &t), &exp) {
                    (Ok(()), Expect::Accept) => {
                        m.splice(at..at, t.chars());
                    }
                    (Ok(()), _) => assert_eq!(None, Some(&exp), "{ctx}: insert({at}) accepted"),
                    (Err(e), _) => {
                        assert_rejected_as(&e, &exp, &ctx);
                        rejected += 1;
                    }
                }
            }
            Kind::Delete => {
                let r = gen_range(&mut rng, len);
                let exp = expect_range(&r, len);
                match (b.delete(r.clone()), &exp) {
                    (Ok(removed), Expect::Accept) => {
                        let expected: String = m.drain(r.clone()).collect();
                        assert_eq!(removed, expected, "{ctx}: removed by delete({r:?})");
                    }
                    (Ok(_), _) => assert_eq!(None, Some(&exp), "{ctx}: delete({r:?}) accepted"),
                    (Err(e), _) => {
                        assert_rejected_as(&e, &exp, &ctx);
                        rejected += 1;
                    }
                }
            }
            Kind::Replace => {
                let r = gen_range(&mut rng, len);
                let t = gen_insert_text(&mut rng);
                let exp = expect_range(&r, len);
                match (b.replace(r.clone(), &t), &exp) {
                    (Ok(removed), Expect::Accept) => {
                        let expected: String = m.splice(r.clone(), t.chars()).collect();
                        assert_eq!(removed, expected, "{ctx}: removed by replace({r:?}, {t:?})");
                    }
                    (Ok(_), _) => assert_eq!(None, Some(&exp), "{ctx}: replace({r:?}) accepted"),
                    (Err(e), _) => {
                        assert_rejected_as(&e, &exp, &ctx);
                        rejected += 1;
                    }
                }
            }
        }
        assert_matches_model(&b, &m, &ctx);
    }
    rejected
}

// ---------------------------------------------------------------------------------------------
// AC-1: edits against the naive model (text.insert, text.delete)
// ---------------------------------------------------------------------------------------------

#[test]
fn insert_matches_naive_model() {
    for seed in [0x7001_0001, 0x7001_0002, 0x7001_0003] {
        let rejected = run_differential(seed, 2_000, &[Kind::Insert]);
        assert!(
            rejected > 50,
            "generator must reach out-of-range inserts (got {rejected})"
        );
    }
    // exact examples
    let mut b = Buffer::from_text("héllo");
    b.insert(2, "😀").unwrap();
    assert_eq!(b.text(), "hé😀llo");
    b.insert(6, "\r\n").unwrap();
    assert_eq!(b.text(), "hé😀llo\r\n");
    b.insert(0, "").unwrap();
    assert_eq!(b.text(), "hé😀llo\r\n");
    assert_eq!(
        (b.len_chars(), b.len_bytes(), b.len_utf16(), b.len_lines()),
        (8, 12, 9, 2)
    );
}

#[test]
fn insert_rejects_out_of_range_and_leaves_buffer_unchanged() {
    let mut b = Buffer::from_text("ab");
    assert_eq!(
        b.insert(3, "x"),
        Err(EditError::OutOfBounds {
            index: 3,
            len_chars: 2
        })
    );
    assert_eq!(
        b.insert(usize::MAX, "x"),
        Err(EditError::OutOfBounds {
            index: usize::MAX,
            len_chars: 2
        })
    );
    assert_eq!(b.text(), "ab");
    assert!(!b.can_undo(), "a rejected insert must record nothing");

    let mut e = Buffer::new();
    assert_eq!(
        e.insert(1, "x"),
        Err(EditError::OutOfBounds {
            index: 1,
            len_chars: 0
        })
    );
    assert_eq!(e.text(), "");
    assert!(e.is_empty());

    // char indices, not bytes: "é😀" has 2 chars, 6 bytes
    let mut u = Buffer::from_text("é😀");
    assert_eq!(
        u.insert(3, "x"),
        Err(EditError::OutOfBounds {
            index: 3,
            len_chars: 2
        })
    );
    u.insert(2, "x").unwrap();
    assert_eq!(u.text(), "é😀x");

    // a rejected insert leaves the history as it was
    let mut h = Buffer::from_text("ab");
    h.insert(2, "cd").unwrap();
    h.undo();
    assert!(h.can_redo());
    assert!(h.insert(9, "zz").is_err());
    assert_eq!(h.text(), "ab");
    assert!(h.can_redo(), "a rejected insert must not clear redo");
    assert!(h.redo());
    assert_eq!(h.text(), "abcd");
}

#[test]
fn delete_and_replace_match_naive_model() {
    for seed in [0x7001_0011, 0x7001_0012, 0x7001_0013] {
        let rejected = run_differential(seed, 2_000, &[Kind::Insert, Kind::Delete, Kind::Replace]);
        assert!(
            rejected > 50,
            "generator must reach rejected edits (got {rejected})"
        );
    }
    // exact examples
    let mut b = Buffer::from_text("a😀b\r\nc");
    assert_eq!(b.delete(1..2).unwrap(), "😀");
    assert_eq!(b.text(), "ab\r\nc");
    assert_eq!(b.replace(2..4, "\n").unwrap(), "\r\n");
    assert_eq!(b.text(), "ab\nc");
    assert_eq!(b.replace(0..0, "€").unwrap(), "");
    assert_eq!(b.text(), "€ab\nc");
    assert_eq!(b.replace(1..3, "").unwrap(), "ab");
    assert_eq!(b.text(), "€\nc");
    assert_eq!(b.delete(3..3).unwrap(), "");
    assert_eq!(b.text(), "€\nc");
}

#[test]
fn delete_rejects_inverted_and_out_of_range() {
    let mut b = Buffer::from_text("abc");
    let inverted = Range { start: 2, end: 1 };
    assert_eq!(
        b.delete(inverted.clone()),
        Err(EditError::InvertedRange { start: 2, end: 1 })
    );
    assert_eq!(
        b.delete(1..9),
        Err(EditError::OutOfBounds {
            index: 9,
            len_chars: 3
        })
    );
    assert_eq!(
        b.delete(3..4),
        Err(EditError::OutOfBounds {
            index: 4,
            len_chars: 3
        })
    );
    assert_eq!(
        b.delete(0..usize::MAX),
        Err(EditError::OutOfBounds {
            index: usize::MAX,
            len_chars: 3
        })
    );
    assert_eq!(
        b.replace(inverted, "x"),
        Err(EditError::InvertedRange { start: 2, end: 1 })
    );
    assert_eq!(
        b.replace(2..4, "x"),
        Err(EditError::OutOfBounds {
            index: 4,
            len_chars: 3
        })
    );
    assert_eq!(
        b.replace(0..4, ""),
        Err(EditError::OutOfBounds {
            index: 4,
            len_chars: 3
        })
    );
    assert_eq!(b.text(), "abc");
    assert!(!b.can_undo(), "rejected edits must record nothing");
    assert!(!b.can_redo());

    // char indices, not bytes
    let mut u = Buffer::from_text("😀é");
    assert_eq!(
        u.delete(0..3),
        Err(EditError::OutOfBounds {
            index: 3,
            len_chars: 2
        })
    );
    assert_eq!(u.delete(0..2).unwrap(), "😀é");
    assert_eq!(u.text(), "");

    // empty buffer
    let mut e = Buffer::new();
    assert_eq!(
        e.delete(0..1),
        Err(EditError::OutOfBounds {
            index: 1,
            len_chars: 0
        })
    );
    assert_eq!(e.delete(0..0).unwrap(), "");
    assert_eq!(e.text(), "");
}

// ---------------------------------------------------------------------------------------------
// AC-2, AC-3: positions (text.positions)
// ---------------------------------------------------------------------------------------------

/// Compare all six conversions with the model for every input of a buffer: every char index,
/// every byte offset, every UTF-16 offset (valid or not) and every line/column, one past each.
fn check_conversions(b: &Buffer, text: &str, ctx: &str) {
    let m = chars_of(text);
    let p = model_prefix(&m);
    let lines = model_lines(&m);
    let len_bytes = model_len_bytes(&m);
    let len_utf16 = model_len_utf16(&m);
    for c in 0..=m.len() + 2 {
        assert_eq!(
            b.char_to_byte(c),
            model_char_to_byte(&m, &p, c),
            "{ctx}: char_to_byte({c})"
        );
        assert_eq!(
            b.char_to_utf16(c),
            model_char_to_utf16(&m, &p, c),
            "{ctx}: char_to_utf16({c})"
        );
        assert_eq!(
            b.char_to_line_col(c),
            model_char_to_line_col(&m, &lines, c),
            "{ctx}: char_to_line_col({c})"
        );
    }
    for byte in 0..=len_bytes + 2 {
        assert_eq!(
            b.byte_to_char(byte),
            model_byte_to_char(&p, byte),
            "{ctx}: byte_to_char({byte})"
        );
    }
    for u in 0..=len_utf16 + 2 {
        assert_eq!(
            b.utf16_to_char(u),
            model_utf16_to_char(&p, u),
            "{ctx}: utf16_to_char({u})"
        );
    }
    for line in 0..=lines.len() {
        let max_col = lines.get(line).map_or(1, |l| l.end - l.start + 2);
        for col in 0..=max_col {
            let pos = LineCol { line, col };
            assert_eq!(
                b.line_col_to_char(pos),
                model_line_col_to_char(&lines, pos),
                "{ctx}: line_col_to_char({pos:?})"
            );
        }
    }
}

#[test]
fn conversions_match_naive_model() {
    let mut rng = Rng::new(0x7001_0021);
    for i in 0..60 {
        let (b, text) = gen_buffer(&mut rng, 40);
        check_conversions(&b, &text, &format!("generated buffer {i} {text:?}"));
    }
    // exact values
    let b = Buffer::from_text("aé€😀\r\nx\ry");
    assert_eq!(b.len_chars(), 9);
    assert_eq!(b.len_bytes(), 1 + 2 + 3 + 4 + 2 + 1 + 1 + 1);
    assert_eq!(b.len_utf16(), 10);
    assert_eq!(b.char_to_byte(4), Ok(10));
    assert_eq!(b.byte_to_char(10), Ok(4));
    assert_eq!(b.char_to_utf16(4), Ok(5));
    assert_eq!(b.utf16_to_char(5), Ok(4));
    assert_eq!(b.char_to_line_col(4), Ok(LineCol { line: 0, col: 4 }));
    assert_eq!(b.char_to_line_col(6), Ok(LineCol { line: 1, col: 0 }));
    assert_eq!(b.char_to_line_col(7), Ok(LineCol { line: 1, col: 1 }));
    assert_eq!(b.char_to_line_col(8), Ok(LineCol { line: 2, col: 0 }));
    assert_eq!(b.char_to_line_col(9), Ok(LineCol { line: 2, col: 1 }));
    assert_eq!(b.line_col_to_char(LineCol { line: 2, col: 1 }), Ok(9));
    // between \r and \n the byte and UTF-16 conversions still exist
    assert_eq!(b.char_to_byte(5), Ok(11));
    assert_eq!(b.char_to_utf16(5), Ok(6));
    // empty buffer
    let e = Buffer::new();
    assert_eq!(e.char_to_byte(0), Ok(0));
    assert_eq!(e.byte_to_char(0), Ok(0));
    assert_eq!(e.char_to_utf16(0), Ok(0));
    assert_eq!(e.utf16_to_char(0), Ok(0));
    assert_eq!(e.char_to_line_col(0), Ok(LineCol { line: 0, col: 0 }));
    assert_eq!(e.line_col_to_char(LineCol { line: 0, col: 0 }), Ok(0));
}

#[test]
fn conversions_round_trip_on_valid_positions() {
    let mut rng = Rng::new(0x7001_0022);
    let mut crlf_seen = 0;
    for i in 0..60 {
        let (b, text) = gen_buffer(&mut rng, 40);
        let m = chars_of(&text);
        for c in 0..=b.len_chars() {
            let ctx = format!("buffer {i} {text:?} char {c}");
            let byte = b.char_to_byte(c).unwrap();
            assert_eq!(b.byte_to_char(byte), Ok(c), "{ctx}");
            let u = b.char_to_utf16(c).unwrap();
            assert_eq!(b.utf16_to_char(u), Ok(c), "{ctx}");
            if model_inside_crlf(&m, c) {
                crlf_seen += 1;
                assert_eq!(
                    b.char_to_line_col(c),
                    Err(PositionError::InsideLineBreak { char_idx: c }),
                    "{ctx}"
                );
            } else {
                let lc = b.char_to_line_col(c).unwrap();
                assert_eq!(b.line_col_to_char(lc), Ok(c), "{ctx}");
            }
        }
        // and the other direction: every valid LineCol maps to a char that maps back
        for line in 0..b.len_lines() {
            let line_len = b.line(line).unwrap().chars().count();
            for col in 0..=line_len {
                let pos = LineCol { line, col };
                let c = b.line_col_to_char(pos).unwrap();
                assert_eq!(
                    b.char_to_line_col(c),
                    Ok(pos),
                    "buffer {i} {text:?} {pos:?}"
                );
            }
        }
    }
    assert!(crlf_seen > 0, "generator must produce CRLF positions");
}

#[test]
fn invalid_positions_give_exact_errors() {
    // byte inside a multi-byte char
    let b = Buffer::from_text("aé€😀");
    assert_eq!(
        b.byte_to_char(2),
        Err(PositionError::NotCharBoundary { byte: 2 })
    );
    assert_eq!(
        b.byte_to_char(4),
        Err(PositionError::NotCharBoundary { byte: 4 })
    );
    assert_eq!(
        b.byte_to_char(5),
        Err(PositionError::NotCharBoundary { byte: 5 })
    );
    for byte in 7..10 {
        assert_eq!(
            b.byte_to_char(byte),
            Err(PositionError::NotCharBoundary { byte })
        );
    }
    assert_eq!(b.byte_to_char(6), Ok(3));
    assert_eq!(b.byte_to_char(10), Ok(4));
    // UTF-16 between the surrogate halves of 😀
    let s = Buffer::from_text("😀");
    assert_eq!(
        s.utf16_to_char(1),
        Err(PositionError::InsideSurrogatePair { utf16: 1 })
    );
    assert_eq!(s.utf16_to_char(2), Ok(1));
    let s2 = Buffer::from_text("a😀b😀");
    assert_eq!(
        s2.utf16_to_char(2),
        Err(PositionError::InsideSurrogatePair { utf16: 2 })
    );
    assert_eq!(
        s2.utf16_to_char(5),
        Err(PositionError::InsideSurrogatePair { utf16: 5 })
    );
    assert_eq!(s2.utf16_to_char(4), Ok(3));
    // above the length, in the unit of the call: len_chars = 4, len_bytes = 10, len_utf16 = 5
    assert_eq!(
        b.char_to_byte(5),
        Err(PositionError::OutOfBounds { index: 5, len: 4 })
    );
    assert_eq!(
        b.char_to_utf16(5),
        Err(PositionError::OutOfBounds { index: 5, len: 4 })
    );
    assert_eq!(
        b.char_to_line_col(5),
        Err(PositionError::OutOfBounds { index: 5, len: 4 })
    );
    assert_eq!(
        b.byte_to_char(11),
        Err(PositionError::OutOfBounds { index: 11, len: 10 })
    );
    assert_eq!(
        b.utf16_to_char(6),
        Err(PositionError::OutOfBounds { index: 6, len: 5 })
    );
    assert_eq!(
        b.char_to_byte(usize::MAX),
        Err(PositionError::OutOfBounds {
            index: usize::MAX,
            len: 4
        })
    );
    let e = Buffer::new();
    assert_eq!(
        e.char_to_byte(1),
        Err(PositionError::OutOfBounds { index: 1, len: 0 })
    );
    assert_eq!(
        e.byte_to_char(1),
        Err(PositionError::OutOfBounds { index: 1, len: 0 })
    );
    assert_eq!(
        e.utf16_to_char(1),
        Err(PositionError::OutOfBounds { index: 1, len: 0 })
    );
    // between \r and \n
    let c = Buffer::from_text("a\r\nb\r\n");
    assert_eq!(
        c.char_to_line_col(2),
        Err(PositionError::InsideLineBreak { char_idx: 2 })
    );
    assert_eq!(
        c.char_to_line_col(5),
        Err(PositionError::InsideLineBreak { char_idx: 5 })
    );
    assert_eq!(c.char_to_line_col(1), Ok(LineCol { line: 0, col: 1 }));
    assert_eq!(c.char_to_line_col(6), Ok(LineCol { line: 2, col: 0 }));
    // a lone \r followed by \r\n: only the index inside the CRLF is invalid
    let rr = Buffer::from_text("\r\r\n");
    assert_eq!(rr.char_to_line_col(1), Ok(LineCol { line: 1, col: 0 }));
    assert_eq!(
        rr.char_to_line_col(2),
        Err(PositionError::InsideLineBreak { char_idx: 2 })
    );
    // line == len_lines()
    assert_eq!(c.len_lines(), 3);
    assert_eq!(
        c.line(3),
        Err(PositionError::LineOutOfBounds {
            line: 3,
            len_lines: 3
        })
    );
    assert_eq!(
        c.line_start(3),
        Err(PositionError::LineOutOfBounds {
            line: 3,
            len_lines: 3
        })
    );
    assert_eq!(
        c.line_col_to_char(LineCol { line: 3, col: 0 }),
        Err(PositionError::LineOutOfBounds {
            line: 3,
            len_lines: 3
        })
    );
    assert_eq!(
        e.line_col_to_char(LineCol { line: 1, col: 0 }),
        Err(PositionError::LineOutOfBounds {
            line: 1,
            len_lines: 1
        })
    );
    // col == line_len + 1
    assert_eq!(
        c.line_col_to_char(LineCol { line: 0, col: 2 }),
        Err(PositionError::ColumnOutOfBounds {
            line: 0,
            col: 2,
            line_len: 1
        })
    );
    assert_eq!(
        c.line_col_to_char(LineCol { line: 2, col: 1 }),
        Err(PositionError::ColumnOutOfBounds {
            line: 2,
            col: 1,
            line_len: 0
        })
    );
    let w = Buffer::from_text("é😀\nx");
    assert_eq!(
        w.line_col_to_char(LineCol { line: 0, col: 3 }),
        Err(PositionError::ColumnOutOfBounds {
            line: 0,
            col: 3,
            line_len: 2
        })
    );
    assert_eq!(w.line_col_to_char(LineCol { line: 0, col: 2 }), Ok(2));
}

#[test]
fn error_types_implement_error_and_display() {
    fn is_error<E: std::error::Error>(_: &E) {}
    let e = EditError::InvertedRange { start: 2, end: 1 };
    is_error(&e);
    assert!(!e.to_string().is_empty());
    let errors = [
        PositionError::OutOfBounds { index: 1, len: 0 },
        PositionError::NotCharBoundary { byte: 1 },
        PositionError::InsideSurrogatePair { utf16: 1 },
        PositionError::InsideLineBreak { char_idx: 1 },
        PositionError::LineOutOfBounds {
            line: 1,
            len_lines: 1,
        },
        PositionError::ColumnOutOfBounds {
            line: 0,
            col: 2,
            line_len: 1,
        },
    ];
    for p in &errors {
        is_error(p);
        assert!(!p.to_string().is_empty(), "{p:?}");
    }
    assert!(LineCol { line: 0, col: 9 } < LineCol { line: 1, col: 0 });
}

// ---------------------------------------------------------------------------------------------
// AC-4, AC-5: lines and line endings (text.lines)
// ---------------------------------------------------------------------------------------------

const LINE_CORPUS: &[&str] = &[
    "",
    "a",
    "\n",
    "a\n",
    "\r\n",
    "\r",
    "\r\r\n",
    "\n\r",
    "a\r\nb\rc\nd",
    "é\n😀\r\n",
];

fn check_lines(b: &Buffer, text: &str, ctx: &str) {
    let m = chars_of(text);
    let lines = model_lines(&m);
    assert_eq!(b.len_lines(), lines.len(), "{ctx}: len_lines");
    for (i, l) in lines.iter().enumerate() {
        let content: String = m[l.start..l.end].iter().collect();
        assert_eq!(b.line(i), Ok(content), "{ctx}: line({i})");
        assert_eq!(b.line_start(i), Ok(l.start), "{ctx}: line_start({i})");
    }
    let n = lines.len();
    assert_eq!(
        b.line(n),
        Err(PositionError::LineOutOfBounds {
            line: n,
            len_lines: n
        }),
        "{ctx}"
    );
}

#[test]
fn lines_match_naive_split_for_lf_crlf_cr() {
    for text in LINE_CORPUS {
        check_lines(&Buffer::from_text(text), text, &format!("corpus {text:?}"));
    }
    // exact values
    let b = Buffer::from_text("a\r\nb\rc\nd");
    assert_eq!(b.len_lines(), 4);
    let got: Vec<String> = (0..4).map(|i| b.line(i).unwrap()).collect();
    assert_eq!(got, ["a", "b", "c", "d"]);
    let starts: Vec<usize> = (0..4).map(|i| b.line_start(i).unwrap()).collect();
    assert_eq!(starts, [0, 3, 5, 7]);
    let b = Buffer::from_text("a\n");
    assert_eq!(b.len_lines(), 2);
    assert_eq!(
        (b.line(0), b.line(1)),
        (Ok("a".to_owned()), Ok(String::new()))
    );
    assert_eq!((b.line_start(0), b.line_start(1)), (Ok(0), Ok(2)));
    let e = Buffer::new();
    assert_eq!(
        (e.len_lines(), e.line(0), e.line_start(0)),
        (1, Ok(String::new()), Ok(0))
    );
    // chars that are NOT line breaks under the spec (VT, FF, NEL, LS, PS)
    let odd = "a\u{0B}b\u{0C}c\u{85}d\u{2028}e\u{2029}f";
    let ob = Buffer::from_text(odd);
    assert_eq!(ob.len_lines(), 1);
    assert_eq!(ob.line(0), Ok(odd.to_owned()));
    assert_eq!(ob.line_endings(), LineEndings::None);
    // edits that join and split a CRLF
    let mut j = Buffer::from_text("\rx\n");
    check_lines(&j, "\rx\n", "before join");
    j.delete(1..2).unwrap();
    check_lines(&j, "\r\n", "after join");
    assert_eq!(j.len_lines(), 2);
    j.insert(1, "y").unwrap();
    check_lines(&j, "\ry\n", "after split");
    assert_eq!(j.len_lines(), 3);
    j.replace(1..2, "").unwrap();
    j.insert(0, "\r").unwrap();
    check_lines(&j, "\r\r\n", "after insert");
    // generated strings, from_text and edit-built
    let mut rng = Rng::new(0x7001_0041);
    for i in 0..200 {
        let (b, text) = gen_buffer(&mut rng, 30);
        check_lines(&b, &text, &format!("generated {i} {text:?}"));
    }
}

#[test]
fn line_endings_style_exact_values() {
    let cases: &[(&str, LineEndings)] = &[
        ("", LineEndings::None),
        ("abc", LineEndings::None),
        ("a\nb\n", LineEndings::Lf),
        ("a\r\nb\r\n", LineEndings::Crlf),
        ("a\rb", LineEndings::Cr),
        ("a\nb\r\n", LineEndings::Mixed),
        ("a\r\r\n", LineEndings::Mixed),
        ("\r\n", LineEndings::Crlf),
        ("\n\r", LineEndings::Mixed),
        ("\r\r", LineEndings::Cr),
        ("a\u{85}b\u{2028}", LineEndings::None),
    ];
    for (text, want) in cases {
        assert_eq!(Buffer::from_text(text).line_endings(), *want, "{text:?}");
    }
    // the style follows edits
    let mut b = Buffer::from_text("a\r\nb");
    assert_eq!(b.line_endings(), LineEndings::Crlf);
    b.insert(4, "\n").unwrap();
    assert_eq!(b.line_endings(), LineEndings::Mixed);
    b.delete(1..2).unwrap(); // "a\nb\n"
    assert_eq!(b.text(), "a\nb\n");
    assert_eq!(b.line_endings(), LineEndings::Lf);
    let mut rng = Rng::new(0x7001_0051);
    for i in 0..300 {
        let (b, text) = gen_buffer(&mut rng, 12);
        assert_eq!(
            b.line_endings(),
            model_line_endings(&chars_of(&text)),
            "generated {i} {text:?}"
        );
    }
}

#[test]
fn dominant_line_ending_matches_naive_count() {
    let cases: &[(&str, Option<LineEnding>)] = &[
        ("", None),
        ("abc", None),
        ("a\r\nb\r\nc\n", Some(LineEnding::Crlf)),
        ("a\nb\r\n", Some(LineEnding::Lf)),
        ("a\r\nb\r", Some(LineEnding::Crlf)),
        ("a\r\r\r\n", Some(LineEnding::Cr)),
        ("a\nb\r", Some(LineEnding::Lf)),
        ("\n\r\n\r", Some(LineEnding::Lf)),
        ("\r\r\n\n\n", Some(LineEnding::Lf)),
        ("\r", Some(LineEnding::Cr)),
    ];
    for (text, want) in cases {
        assert_eq!(
            Buffer::from_text(text).dominant_line_ending(),
            *want,
            "{text:?}"
        );
    }
    let mut rng = Rng::new(0x7001_0061);
    for i in 0..300 {
        let (b, text) = gen_buffer(&mut rng, 16);
        assert_eq!(
            b.dominant_line_ending(),
            model_dominant(&chars_of(&text)),
            "generated {i} {text:?}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// AC-6, AC-8: undo and redo of one step (undo.undo, undo.redo)
// ---------------------------------------------------------------------------------------------

/// One valid, non-no-op edit of the given kind on a text of `len` chars.
fn gen_valid_edit(rng: &mut Rng, kind: Kind, len: usize) -> (Range<usize>, String) {
    match kind {
        Kind::Insert => {
            let at = rng.below(len + 1);
            (at..at, gen_nonempty_insert_text(rng))
        }
        Kind::Delete => {
            let s = rng.below(len);
            let e = s + 1 + rng.below(len - s);
            (s..e, String::new())
        }
        Kind::Replace => {
            let s = rng.below(len + 1);
            let e = s + rng.below(len - s + 1);
            let t = if s == e {
                gen_nonempty_insert_text(rng)
            } else {
                gen_insert_text(rng)
            };
            (s..e, t)
        }
    }
}

fn apply_edit(b: &mut Buffer, kind: Kind, r: &Range<usize>, t: &str) {
    match kind {
        Kind::Insert => assert_eq!(b.insert(r.start, t), Ok(())),
        Kind::Delete => {
            assert!(b.delete(r.clone()).is_ok(), "delete({r:?})");
        }
        Kind::Replace => {
            assert!(b.replace(r.clone(), t).is_ok(), "replace({r:?}, {t:?})");
        }
    }
}

fn spliced(text: &str, r: &Range<usize>, t: &str) -> String {
    let mut m = chars_of(text);
    m.splice(r.clone(), t.chars());
    m.iter().collect()
}

/// A buffer from the generator, often with history already on it, ended with a group break so
/// the next edit is its own step.
fn gen_buffer_for_undo(rng: &mut Rng) -> Buffer {
    let (mut b, _) = gen_buffer(rng, 30);
    if rng.chance(50) {
        let at = rng.below(b.len_chars() + 1);
        assert_eq!(b.insert(at, "x"), Ok(()));
    }
    b.break_undo_group();
    b
}

#[test]
fn undo_restores_text_before_each_edit_kind() {
    let mut rng = Rng::new(0x7001_0071);
    for i in 0..300 {
        let kind = [Kind::Insert, Kind::Delete, Kind::Replace][i % 3];
        let mut b = gen_buffer_for_undo(&mut rng);
        if matches!(kind, Kind::Delete) && b.len_chars() == 0 {
            b = Buffer::from_text("😀\r\n");
        }
        let before = b.text();
        let (r, t) = gen_valid_edit(&mut rng, kind, b.len_chars());
        apply_edit(&mut b, kind, &r, &t);
        let after = spliced(&before, &r, &t);
        assert_eq!(b.text(), after, "case {i}");
        assert!(b.can_undo(), "case {i}");
        assert!(b.undo(), "case {i}: undo after an edit");
        let m = chars_of(&before);
        assert_matches_model(
            &b,
            &m,
            &format!("case {i}: undo of {r:?} {t:?} on {before:?}"),
        );
        assert!(b.can_redo(), "case {i}");
    }
    // exact: a replace is one step
    let mut b = Buffer::from_text("hello");
    assert_eq!(b.replace(1..4, "ipp").unwrap(), "ell");
    assert_eq!(b.text(), "hippo");
    assert!(b.undo());
    assert_eq!(b.text(), "hello");
    assert!(!b.undo());
    // exact: delete of a CRLF and of a surrogate-pair char
    let mut d = Buffer::from_text("a\r\n😀");
    d.delete(1..3).unwrap();
    d.delete(1..2).unwrap();
    assert_eq!(d.text(), "a");
    assert!(d.undo());
    assert_eq!(d.text(), "a😀");
    assert!(d.undo());
    assert_eq!(d.text(), "a\r\n😀");
    assert_eq!(d.len_lines(), 2);
}

#[test]
fn undo_and_redo_with_empty_history_are_noops() {
    for start in ["", "abc", "a\r\n😀"] {
        let mut b = if start.is_empty() {
            Buffer::new()
        } else {
            Buffer::from_text(start)
        };
        assert!(!b.can_undo(), "{start:?}");
        assert!(!b.can_redo(), "{start:?}");
        assert!(!b.undo(), "{start:?}");
        assert!(!b.redo(), "{start:?}");
        assert_eq!(b.text(), start);
        assert!(!b.can_undo());
        assert!(!b.can_redo());
    }
    assert!(!Buffer::default().undo());
    // once everything is undone, undo is a no-op; once everything is redone, redo is
    let mut b = Buffer::from_text("abc");
    b.delete(0..1).unwrap();
    assert!(b.undo());
    assert_eq!(b.text(), "abc");
    assert!(!b.can_undo());
    assert!(!b.undo());
    assert_eq!(b.text(), "abc");
    assert!(b.can_redo(), "a failed undo must not drop the redo step");
    assert!(b.redo());
    assert_eq!(b.text(), "bc");
    assert!(!b.can_redo());
    assert!(!b.redo());
    assert_eq!(b.text(), "bc");
    assert!(b.can_undo(), "a failed redo must not drop the undo step");
    assert!(b.undo());
    assert_eq!(b.text(), "abc");
}

#[test]
fn redo_reapplies_undone_edit() {
    let mut rng = Rng::new(0x7001_0081);
    for i in 0..300 {
        let kind = [Kind::Insert, Kind::Delete, Kind::Replace][i % 3];
        let mut b = gen_buffer_for_undo(&mut rng);
        if matches!(kind, Kind::Delete) && b.len_chars() == 0 {
            b = Buffer::from_text("é\r");
        }
        let before = b.text();
        let (r, t) = gen_valid_edit(&mut rng, kind, b.len_chars());
        apply_edit(&mut b, kind, &r, &t);
        let after = spliced(&before, &r, &t);
        assert!(b.undo(), "case {i}");
        assert_eq!(b.text(), before, "case {i}");
        assert!(b.redo(), "case {i}: redo after undo");
        assert_matches_model(
            &b,
            &chars_of(&after),
            &format!("case {i}: redo of {r:?} {t:?}"),
        );
        assert!(!b.can_redo(), "case {i}");
        assert!(!b.redo(), "case {i}");
        assert_eq!(b.text(), after, "case {i}");
        // the redone step is undoable again
        assert!(b.undo(), "case {i}");
        assert_eq!(b.text(), before, "case {i}");
    }
    // exact: redo of several undone steps, in order
    let mut b = Buffer::new();
    b.insert(0, "ab").unwrap();
    b.delete(0..1).unwrap();
    b.replace(0..1, "XY").unwrap();
    assert_eq!(b.text(), "XY");
    assert!(b.undo() && b.undo() && b.undo());
    assert_eq!(b.text(), "");
    assert!(b.redo());
    assert_eq!(b.text(), "ab");
    assert!(b.redo());
    assert_eq!(b.text(), "b");
    assert!(b.redo());
    assert_eq!(b.text(), "XY");
    assert!(!b.redo());
}

#[test]
fn edit_after_undo_clears_redo() {
    for kind in [Kind::Insert, Kind::Delete, Kind::Replace] {
        let mut b = Buffer::from_text("abc");
        b.insert(3, "de").unwrap();
        b.delete(0..1).unwrap();
        assert!(b.undo());
        assert_eq!(b.text(), "abcde");
        assert!(b.can_redo());
        match kind {
            Kind::Insert => b.insert(0, "z").unwrap(),
            Kind::Delete => {
                b.delete(4..5).unwrap();
            }
            Kind::Replace => {
                b.replace(1..2, "Q").unwrap();
            }
        }
        let now = b.text();
        assert!(!b.can_redo(), "edit after undo must clear redo");
        assert!(!b.redo());
        assert_eq!(b.text(), now);
        // the new edit undoes, then the edit before the undone one
        assert!(b.undo());
        assert_eq!(b.text(), "abcde");
        assert!(b.undo());
        assert_eq!(b.text(), "abc");
        assert!(!b.undo());
    }
    // replace of an empty range with text also clears redo
    let mut b = Buffer::from_text("x");
    b.delete(0..1).unwrap();
    assert!(b.undo());
    b.replace(1..1, "y").unwrap();
    assert!(!b.can_redo());
    assert!(!b.redo());
    assert_eq!(b.text(), "xy");
}

// ---------------------------------------------------------------------------------------------
// AC-9: grouping (undo.grouping)
// ---------------------------------------------------------------------------------------------

fn typed_ab() -> Buffer {
    let mut b = Buffer::new();
    assert_eq!(b.insert(0, "a"), Ok(()));
    assert_eq!(b.insert(1, "b"), Ok(()));
    b
}

#[test]
fn adjacent_typed_chars_group_into_one_step() {
    let mut b = typed_ab();
    b.insert(2, "c").unwrap();
    assert_eq!(b.text(), "abc");
    assert!(b.undo());
    assert_eq!(b.text(), "");
    assert!(!b.can_undo());
    assert!(b.redo());
    assert_eq!(b.text(), "abc");
    assert!(!b.can_redo());

    // multi-byte single chars group too, typed in the middle of existing text
    let mut m = Buffer::from_text("x\r\ny");
    m.insert(3, "é").unwrap();
    m.insert(4, "😀").unwrap();
    m.insert(5, "中").unwrap();
    assert_eq!(m.text(), "x\r\né😀中y");
    assert!(m.undo());
    assert_eq!(m.text(), "x\r\ny");
    assert!(!m.undo());

    // a no-op does not break the group
    let mut n = typed_ab();
    assert_eq!(n.delete(0..0).unwrap(), "");
    n.insert(1, "").unwrap();
    assert_eq!(n.replace(2..2, "").unwrap(), "");
    n.insert(2, "c").unwrap();
    assert!(n.undo());
    assert_eq!(n.text(), "");

    // two groups separated by an explicit break
    let mut g = typed_ab();
    g.break_undo_group();
    g.insert(2, "c").unwrap();
    g.insert(3, "d").unwrap();
    assert!(g.undo());
    assert_eq!(g.text(), "ab");
    assert!(g.undo());
    assert_eq!(g.text(), "");
}

#[test]
fn group_breaks_on_gap_multichar_linebreak_and_explicit_break() {
    // third insert at 1 instead of 2
    let mut b = typed_ab();
    b.insert(1, "c").unwrap();
    assert_eq!(b.text(), "acb");
    assert!(b.undo());
    assert_eq!(b.text(), "ab");
    // third insert of "cd"
    let mut b = typed_ab();
    b.insert(2, "cd").unwrap();
    assert!(b.undo());
    assert_eq!(b.text(), "ab");
    // third insert of "\n", "\r", "\r\n"
    for brk in ["\n", "\r", "\r\n"] {
        let mut b = typed_ab();
        b.insert(2, brk).unwrap();
        assert!(b.undo(), "{brk:?}");
        assert_eq!(b.text(), "ab", "{brk:?}");
    }
    // a char typed after a line break starts a new step
    let mut b = typed_ab();
    b.insert(2, "\n").unwrap();
    b.insert(3, "c").unwrap();
    assert!(b.undo());
    assert_eq!(b.text(), "ab\n");
    // explicit break
    let mut b = typed_ab();
    b.break_undo_group();
    b.insert(2, "c").unwrap();
    assert!(b.undo());
    assert_eq!(b.text(), "ab");
    // a group of one multi-char insert is never joined
    let mut b = Buffer::new();
    b.insert(0, "ab").unwrap();
    b.insert(2, "c").unwrap();
    assert!(b.undo());
    assert_eq!(b.text(), "ab");
    // a delete breaks the group and is its own step
    let mut b = typed_ab();
    b.delete(1..2).unwrap();
    b.insert(1, "c").unwrap();
    assert_eq!(b.text(), "ac");
    assert!(b.undo());
    assert_eq!(b.text(), "a");
    assert!(b.undo());
    assert_eq!(b.text(), "ab");
    assert!(b.undo());
    assert_eq!(b.text(), "");
    // replace of an empty range never joins and is never joined
    let mut b = typed_ab();
    b.replace(2..2, "c").unwrap();
    b.insert(3, "d").unwrap();
    assert_eq!(b.text(), "abcd");
    assert!(b.undo());
    assert_eq!(b.text(), "abc");
    assert!(b.undo());
    assert_eq!(b.text(), "ab");
    assert!(b.undo());
    assert_eq!(b.text(), "");
    // undo and redo break the group
    let mut b = typed_ab();
    assert!(b.undo());
    assert!(b.redo());
    b.insert(2, "c").unwrap();
    assert!(b.undo());
    assert_eq!(b.text(), "ab");
    let mut b = typed_ab();
    b.break_undo_group();
    b.insert(2, "c").unwrap();
    assert!(b.undo());
    assert_eq!(b.text(), "ab");
    b.insert(2, "d").unwrap(); // after undo: new step, not joined to "ab"
    assert!(b.undo());
    assert_eq!(b.text(), "ab");
    // undo ends the group even when the next typed char lands right after the group's end
    let mut b = Buffer::new();
    b.insert(0, "a").unwrap(); // group "a", next adjacent index 1
    b.insert(0, "z").unwrap(); // not adjacent: own step
    assert!(b.undo());
    assert_eq!(b.text(), "a");
    b.insert(1, "b").unwrap(); // index 1 is right after "a", but an undo happened since
    assert!(b.undo());
    assert_eq!(b.text(), "a");
}

// ---------------------------------------------------------------------------------------------
// AC-7, AC-10: round trip and history hygiene (undo.roundtrip)
// ---------------------------------------------------------------------------------------------

/// Naive history: one (before, after) text pair per undo step, with the grouping rule.
struct History {
    text: Vec<char>,
    undo: Vec<(String, String)>,
    redo: Vec<(String, String)>,
    /// Char index right after the last typed char, while the top step can still be joined.
    open: Option<usize>,
}

impl History {
    fn new(text: &str) -> Self {
        History {
            text: chars_of(text),
            undo: Vec::new(),
            redo: Vec::new(),
            open: None,
        }
    }

    fn current(&self) -> String {
        self.text.iter().collect()
    }

    fn typing_char(t: &str) -> bool {
        let mut it = t.chars();
        matches!((it.next(), it.next()), (Some(c), None) if c != '\n' && c != '\r')
    }

    fn insert(&mut self, at: usize, t: &str) {
        if t.is_empty() {
            return;
        }
        let before = self.current();
        self.text.splice(at..at, t.chars());
        let after = self.current();
        let typing = Self::typing_char(t);
        match self.undo.last_mut() {
            Some(top) if typing && self.open == Some(at) => top.1 = after,
            _ => self.undo.push((before, after)),
        }
        self.open = if typing { Some(at + 1) } else { None };
        self.redo.clear();
    }

    fn replace(&mut self, r: Range<usize>, t: &str) {
        if r.is_empty() && t.is_empty() {
            return;
        }
        let before = self.current();
        self.text.splice(r, t.chars());
        let after = self.current();
        self.undo.push((before, after));
        self.open = None;
        self.redo.clear();
    }

    fn undo(&mut self) -> bool {
        match self.undo.pop() {
            Some(step) => {
                self.text = chars_of(&step.0);
                self.redo.push(step);
                self.open = None;
                true
            }
            None => false,
        }
    }

    fn redo(&mut self) -> bool {
        match self.redo.pop() {
            Some(step) => {
                self.text = chars_of(&step.1);
                self.undo.push(step);
                self.open = None;
                true
            }
            None => false,
        }
    }
}

fn assert_history(b: &Buffer, h: &History, ctx: &str) {
    assert_eq!(b.text(), h.current(), "{ctx}: text");
    assert_eq!(b.can_undo(), !h.undo.is_empty(), "{ctx}: can_undo");
    assert_eq!(b.can_redo(), !h.redo.is_empty(), "{ctx}: can_redo");
}

#[test]
fn undo_all_restores_original_redo_all_restores_final() {
    for seed in [0x7001_0091, 0x7001_0092, 0x7001_0093, 0x7001_0094] {
        let mut rng = Rng::new(seed);
        let start = gen_text(&mut rng, 30);
        let mut b = Buffer::from_text(&start);
        let mut h = History::new(&start);
        let mut successful = 0;
        let mut step = 0;
        while successful < 250 {
            step += 1;
            let ctx = format!("seed {seed:#x} step {step}");
            let len = h.text.len();
            let roll = rng.below(100);
            if roll < 40 {
                // typing: at the open position if there is one, else anywhere
                let at = match h.open {
                    Some(p) if rng.chance(85) => p,
                    _ => rng.below(len + 1),
                };
                let t = TYPING[rng.below(TYPING.len())];
                b.insert(at, t).unwrap();
                h.insert(at, t);
                successful += 1;
            } else if roll < 55 {
                let at = rng.below(len + 1);
                let t = gen_insert_text(&mut rng);
                b.insert(at, &t).unwrap();
                h.insert(at, &t);
                successful += usize::from(!t.is_empty());
            } else if roll < 68 {
                let r = gen_range_valid(&mut rng, len);
                let removed = b.delete(r.clone()).unwrap();
                let expected: String = h.text[r.clone()].iter().collect();
                assert_eq!(removed, expected, "{ctx}");
                successful += usize::from(!r.is_empty());
                h.replace(r, "");
            } else if roll < 80 {
                let r = gen_range_valid(&mut rng, len);
                let t = gen_insert_text(&mut rng);
                let removed = b.replace(r.clone(), &t).unwrap();
                let expected: String = h.text[r.clone()].iter().collect();
                assert_eq!(removed, expected, "{ctx}");
                successful += usize::from(!(r.is_empty() && t.is_empty()));
                h.replace(r, &t);
            } else if roll < 88 {
                b.break_undo_group();
                h.open = None;
            } else if roll < 95 {
                assert_eq!(b.undo(), h.undo(), "{ctx}: undo");
            } else if !h.redo.is_empty() {
                // redo only when there is something to redo (a failed redo's effect on an
                // open typing group is not specified)
                assert!(b.redo(), "{ctx}: redo");
                h.redo();
            }
            assert_history(&b, &h, &ctx);
        }
        if !h.redo.is_empty() {
            b.insert(0, "!").unwrap();
            h.insert(0, "!");
        }
        assert_history(&b, &h, &format!("seed {seed:#x} end"));
        let final_text = b.text();
        let steps: Vec<(String, String)> = h.undo.clone();
        assert!(
            steps.len() >= 20,
            "seed {seed:#x}: too few steps ({})",
            steps.len()
        );
        let joined = successful.saturating_sub(steps.len());
        assert!(
            joined > 10,
            "seed {seed:#x}: generator must form typing groups ({joined})"
        );
        // undo to the start, seeing exactly the step boundaries in reverse
        for (k, (before, _)) in steps.iter().enumerate().rev() {
            assert!(b.undo(), "seed {seed:#x}: undo of step {k}");
            assert_eq!(
                &b.text(),
                before,
                "seed {seed:#x}: text after undoing step {k}"
            );
        }
        assert!(!b.undo(), "seed {seed:#x}: more undo steps than the model");
        assert_eq!(b.text(), start, "seed {seed:#x}: undo-all");
        // redo to the end, seeing the boundaries forward
        for (k, (_, after)) in steps.iter().enumerate() {
            assert!(b.redo(), "seed {seed:#x}: redo of step {k}");
            assert_eq!(
                &b.text(),
                after,
                "seed {seed:#x}: text after redoing step {k}"
            );
        }
        assert!(!b.redo(), "seed {seed:#x}: more redo steps than the model");
        assert_eq!(b.text(), final_text, "seed {seed:#x}: redo-all");
    }
}

fn gen_range_valid(rng: &mut Rng, len: usize) -> Range<usize> {
    let s = rng.below(len + 1);
    let e = s + rng.below((len - s).min(10) + 1);
    s..e
}

/// A buffer with both undo and redo available: "x" (step 1), "xyz" (step 2), then one undo.
fn with_history() -> Buffer {
    let mut b = Buffer::new();
    assert_eq!(b.insert(0, "x"), Ok(()));
    b.break_undo_group();
    assert_eq!(b.insert(1, "yz"), Ok(()));
    assert!(b.undo());
    assert_eq!(b.text(), "x");
    b
}

#[test]
fn noop_and_rejected_edits_do_not_touch_history() {
    type Op = fn(&mut Buffer) -> bool; // returns true if the op returned Ok
    // (name, is a no-op (Ok) rather than a rejected edit (Err), op), on the text "x"
    let ops: &[(&str, bool, Op)] = &[
        ("insert(1, \"\")", true, |b| b.insert(1, "").is_ok()),
        ("insert(0, \"\")", true, |b| b.insert(0, "").is_ok()),
        ("delete(1..1)", true, |b| b.delete(1..1).is_ok()),
        ("delete(0..0)", true, |b| b.delete(0..0).is_ok()),
        ("replace(1..1, \"\")", true, |b| b.replace(1..1, "").is_ok()),
        ("insert(9, \"q\")", false, |b| b.insert(9, "q").is_ok()),
        ("delete(1..0)", false, |b| {
            b.delete(Range { start: 1, end: 0 }).is_ok()
        }),
        ("delete(0..9)", false, |b| b.delete(0..9).is_ok()),
        ("replace(1..0, \"a\")", false, |b| {
            b.replace(Range { start: 1, end: 0 }, "a").is_ok()
        }),
        ("replace(0..9, \"a\")", false, |b| {
            b.replace(0..9, "a").is_ok()
        }),
        ("replace(2..3, \"\")", false, |b| {
            b.replace(2..3, "").is_ok()
        }),
    ];
    for &(name, noop, op) in ops {
        // with undo and redo both available
        let mut b = with_history();
        let ok = op(&mut b);
        assert_eq!(ok, noop, "{name}: Ok for no-ops, Err for rejected edits");
        assert_eq!(b.text(), "x", "{name}");
        assert!(b.can_undo(), "{name}: can_undo");
        assert!(
            b.can_redo(),
            "{name}: can_redo (a no-op or rejected edit must not clear redo)"
        );
        assert!(b.redo(), "{name}");
        assert_eq!(b.text(), "xyz", "{name}: redo");
        assert!(b.undo() && b.undo(), "{name}");
        assert_eq!(b.text(), "", "{name}: undo-all");
        assert!(!b.undo(), "{name}: no extra step recorded");
        // with empty history
        let mut e = Buffer::from_text("x");
        assert_eq!(op(&mut e), noop, "{name}");
        assert_eq!(e.text(), "x");
        assert!(!e.can_undo(), "{name}: nothing recorded");
        assert!(!e.can_redo(), "{name}");
        assert!(!e.undo(), "{name}");
        assert_eq!(e.text(), "x");
    }
}

// ---------------------------------------------------------------------------------------------
// AC-11: large text
// ---------------------------------------------------------------------------------------------

#[test]
fn large_buffer_edits_match_naive_model() {
    const TARGET: usize = 10 * 1024 * 1024;
    let mut rng = Rng::new(0x7001_00a1);
    let mut text = String::with_capacity(TARGET + 256);
    let breaks = ["\n", "\r\n", "\r"];
    while text.len() < TARGET {
        let n = rng.below(49);
        for _ in 0..n {
            text.push_str(NON_BREAK[rng.below(NON_BREAK.len())]);
        }
        text.push_str(breaks[rng.below(3)]);
    }
    let mut b = Buffer::from_text(&text);
    let mut m = chars_of(&text);
    drop(text);
    let mut len_bytes = model_len_bytes(&m);
    let mut len_utf16 = model_len_utf16(&m);
    let lines0 = model_lines(&m).len();
    assert!(len_bytes >= TARGET);
    assert!(
        (150_000..=260_000).contains(&lines0),
        "about 200 000 lines, got {lines0}"
    );
    assert_matches_model(&b, &m, "large: from_text");

    for i in 0..250 {
        let ctx = format!("large edit {i}");
        let len = m.len();
        let r = gen_range_valid(&mut rng, len);
        let t = gen_insert_text(&mut rng);
        let removed = match i % 3 {
            0 => {
                b.insert(r.start, &t).unwrap();
                m.splice(r.start..r.start, t.chars());
                String::new()
            }
            1 => {
                let removed = b.delete(r.clone()).unwrap();
                let expected: String = m.drain(r).collect();
                assert_eq!(removed, expected, "{ctx}");
                removed
            }
            _ => {
                let removed = b.replace(r.clone(), &t).unwrap();
                let expected: String = m.splice(r, t.chars()).collect();
                assert_eq!(removed, expected, "{ctx}");
                removed
            }
        };
        let inserted = if i % 3 == 1 { "" } else { t.as_str() };
        len_bytes = len_bytes + inserted.len() - removed.len();
        len_utf16 = len_utf16 + inserted.encode_utf16().count() - removed.encode_utf16().count();
        assert_eq!(b.len_chars(), m.len(), "{ctx}: len_chars");
        assert_eq!(b.len_bytes(), len_bytes, "{ctx}: len_bytes");
        assert_eq!(b.len_utf16(), len_utf16, "{ctx}: len_utf16");
        if i % 25 == 24 {
            assert_matches_model(&b, &m, &ctx);
        }
    }
    assert_matches_model(&b, &m, "large: after edits");

    // conversions at random positions, the model computed in one pass
    let lines = model_lines(&m);
    let mut samples: Vec<usize> = (0..250).map(|_| rng.below(m.len() + 1)).collect();
    samples.extend([0, m.len()]);
    samples.sort_unstable();
    let mut k = 0;
    let (mut byte, mut u16) = (0usize, 0usize);
    for (c, ch) in m.iter().chain(std::iter::once(&'\0')).enumerate() {
        while k < samples.len() && samples[k] == c {
            let ctx = format!("large conversion at char {c}");
            assert_eq!(b.char_to_byte(c), Ok(byte), "{ctx}");
            assert_eq!(b.byte_to_char(byte), Ok(c), "{ctx}");
            assert_eq!(b.char_to_utf16(c), Ok(u16), "{ctx}");
            assert_eq!(b.utf16_to_char(u16), Ok(c), "{ctx}");
            if c < m.len() && ch.len_utf8() > 1 {
                assert_eq!(
                    b.byte_to_char(byte + 1),
                    Err(PositionError::NotCharBoundary { byte: byte + 1 }),
                    "{ctx}"
                );
            }
            if c < m.len() && ch.len_utf16() == 2 {
                assert_eq!(
                    b.utf16_to_char(u16 + 1),
                    Err(PositionError::InsideSurrogatePair { utf16: u16 + 1 }),
                    "{ctx}"
                );
            }
            let want = model_char_to_line_col(&m, &lines, c);
            assert_eq!(b.char_to_line_col(c), want, "{ctx}");
            if let Ok(lc) = want {
                assert_eq!(b.line_col_to_char(lc), Ok(c), "{ctx}");
            }
            k += 1;
        }
        if c < m.len() {
            byte += ch.len_utf8();
            u16 += ch.len_utf16();
        }
    }
    assert_eq!(k, samples.len());
    assert_eq!(
        b.char_to_byte(m.len() + 1),
        Err(PositionError::OutOfBounds {
            index: m.len() + 1,
            len: m.len()
        })
    );
    for _ in 0..100 {
        let i = rng.below(lines.len());
        let l = &lines[i];
        let content: String = m[l.start..l.end].iter().collect();
        assert_eq!(b.line(i), Ok(content), "large line({i})");
        assert_eq!(b.line_start(i), Ok(l.start), "large line_start({i})");
    }
    assert_eq!(b.line_endings(), model_line_endings(&m));
    assert_eq!(b.dominant_line_ending(), model_dominant(&m));
}

// ---------------------------------------------------------------------------------------------
// AC-12: equality and clones
// ---------------------------------------------------------------------------------------------

#[test]
fn equality_ignores_history_and_clones_are_independent() {
    let plain = Buffer::from_text("abc");
    let mut edited = Buffer::new();
    edited.insert(0, "ac").unwrap();
    edited.insert(1, "b").unwrap();
    assert_eq!(edited, plain, "equal text, different history");
    let mut undone = Buffer::from_text("abc");
    undone.delete(0..3).unwrap();
    assert!(undone.undo());
    assert!(undone.can_redo());
    assert_eq!(undone, plain, "equal text, with redo history");
    assert_eq!(Buffer::new(), Buffer::default());
    assert_eq!(Buffer::new(), Buffer::from_text(""));
    assert_ne!(Buffer::from_text("abc"), Buffer::from_text("abd"));
    assert_ne!(Buffer::from_text("a\n"), Buffer::from_text("a\r\n"));

    // undo on a clone leaves the original alone
    let mut original = Buffer::from_text("abc");
    original.insert(3, "def").unwrap();
    original.break_undo_group();
    original.delete(0..1).unwrap();
    let mut copy = original.clone();
    assert_eq!(copy, original);
    assert!(copy.undo());
    assert_eq!(copy.text(), "abcdef");
    assert_eq!(original.text(), "bcdef");
    assert!(!original.can_redo(), "the original's redo is its own");
    assert!(copy.undo());
    assert!(!copy.undo());
    assert_eq!(copy.text(), "abc");
    assert_eq!(original.text(), "bcdef");
    assert!(original.can_undo());
    assert!(original.undo());
    assert_eq!(original.text(), "abcdef");
    // an edit on the clone clears only the clone's redo
    let copy2 = {
        let mut c = original.clone();
        c.insert(0, "Z").unwrap();
        c
    };
    assert!(!copy2.can_redo());
    assert!(original.can_redo());
    assert!(original.redo());
    assert_eq!(original.text(), "bcdef");
    assert_eq!(copy2.text(), "Zabcdef");
    // a clone of a buffer with no history has none
    let mut fresh = Buffer::from_text("q").clone();
    assert!(!fresh.undo());
}
