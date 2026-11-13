//! Independent acceptance tests for T-018 through the public Rust API of `kiln-text` (AC-6 and
//! the Interface's selection, anchor, movement and line-ending API).
//!
//! Written from the task file's Interface, Mapping, Batch and Movement rules only. Oracles are
//! built here: a naive `Vec<char>` text model that maps every tracked position through each
//! primitive edit with the spec's mapping rule (incl. the CRLF snap), a naive history with the
//! crate's typing-group rule, naive last-to-first batch application, exact values for the
//! pinned errors and the worked examples. All generators are fixed-seed (SplitMix64).
//!
//! On base this file does not compile (the API is new); the CLI tests in
//! `apps/kiln-cli/tests/acceptance_t018.rs` fail there by assertion.

use std::ops::Range;

use kiln_text::{
    AnchorId, BatchError, Buffer, EditError, LineEnding, Movement, PositionError, Selection,
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

    fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

/// Text pieces: 1-, 2-, 3- and 4-byte chars, every line-break kind, `\r\r\n` and `\n\r`.
const PIECES: &[&str] = &[
    "a",
    "b",
    "_",
    " ",
    "é",
    "€",
    "😀",
    "\n",
    "\r",
    "\r\n",
    "\r\r\n",
    "\n\r",
    "xy",
    "é😀€",
];

/// `min` to `min + spread - 1` random pieces.
fn random_text(rng: &mut Rng, min: usize, spread: usize) -> String {
    let pieces = min + rng.below(spread);
    (0..pieces).map(|_| *rng.pick(PIECES)).collect()
}

fn chars(s: &str) -> Vec<char> {
    s.chars().collect()
}

fn inside_crlf(text: &[char], p: usize) -> bool {
    p > 0 && p < text.len() && text[p - 1] == '\r' && text[p] == '\n'
}

/// The mapping rule for one primitive edit `[s, e)` → `n` chars; `after` is the text after it.
fn map(after: &[char], p: usize, s: usize, e: usize, n: usize) -> usize {
    let r = if p < s {
        p
    } else if p > e {
        p + n - (e - s)
    } else if p == s && e > s {
        s
    } else {
        s + n
    };
    if inside_crlf(after, r) { r - 1 } else { r }
}

/// A position near `near`, or anywhere, within `0..=len`, never inside a CRLF.
fn valid_pos(rng: &mut Rng, text: &[char], near: &[usize]) -> usize {
    let len = text.len();
    let base = if !near.is_empty() && rng.chance(50) {
        *rng.pick(near)
    } else {
        rng.below(len + 1)
    };
    let p = match rng.below(4) {
        0 => base.saturating_sub(1),
        1 => base + 1,
        _ => base,
    }
    .min(len);
    if inside_crlf(text, p) { p - 1 } else { p }
}

/// A random sorted, non-overlapping edit list (ranges in `text`), with insertions, deletions,
/// replacements, adjacent edits and edits at the ends; ranges often start or end at `near`.
fn random_edits(rng: &mut Rng, text: &[char], near: &[usize]) -> Vec<(Range<usize>, String)> {
    let len = text.len();
    let k = rng.below(5);
    let mut points: Vec<usize> = (0..2 * k)
        .map(|_| {
            if !near.is_empty() && rng.chance(40) {
                let p = *rng.pick(near);
                match rng.below(3) {
                    0 => p.saturating_sub(1),
                    1 => (p + 1).min(len),
                    _ => p,
                }
            } else {
                rng.below(len + 1)
            }
        })
        .collect();
    points.sort_unstable();
    let mut edits: Vec<(Range<usize>, String)> = Vec::new();
    for pair in points.chunks(2) {
        let (s, mut e) = (pair[0], pair[1]);
        if rng.chance(25) {
            e = s; // a pure insertion
        }
        if let Some((prev, _)) = edits.last()
            && s <= prev.start
        {
            continue;
        }
        let ins = if rng.chance(25) {
            String::new()
        } else {
            random_text(rng, 1, 2)
        };
        edits.push((s..e, ins));
    }
    edits
}

fn as_refs(edits: &[(Range<usize>, String)]) -> Vec<(Range<usize>, &str)> {
    edits.iter().map(|(r, t)| (r.clone(), t.as_str())).collect()
}

// ---------------------------------------------------------------------------------------------
// Naive model: text, history (typing-group rule), tracked positions
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum Step {
    /// At `at`, `removed` was replaced by `inserted`.
    Op {
        at: usize,
        removed: Vec<char>,
        inserted: Vec<char>,
    },
    /// A batch: the texts before and after it (its undo mapping is not modelled).
    Batch { before: Vec<char>, after: Vec<char> },
}

#[derive(Clone, Debug, Default)]
struct Model {
    text: Vec<char>,
    undo: Vec<Step>,
    redo: Vec<Step>,
    typing_open: bool,
    /// Tracked positions: selection anchor, selection head, then the anchors.
    pos: Vec<usize>,
}

impl Model {
    fn primitive(&mut self, s: usize, e: usize, ins: &[char]) -> Vec<char> {
        let removed: Vec<char> = self.text.splice(s..e, ins.iter().copied()).collect();
        let text = self.text.clone();
        for p in &mut self.pos {
            *p = map(&text, *p, s, e, ins.len());
        }
        removed
    }

    fn record(&mut self, step: Step, typing: bool) {
        self.redo.clear();
        if typing
            && self.typing_open
            && let (
                Some(Step::Op { at, inserted, .. }),
                Step::Op {
                    at: new_at,
                    inserted: new_ins,
                    ..
                },
            ) = (self.undo.last_mut(), &step)
            && *at + inserted.len() == *new_at
        {
            inserted.extend(new_ins);
            return;
        }
        self.undo.push(step);
        self.typing_open = typing;
    }

    /// `Buffer::insert` (`is_insert`, may join a typing group) or `Buffer::replace` (never
    /// joins one).
    fn edit(&mut self, s: usize, e: usize, text: &str, is_insert: bool) {
        let ins = chars(text);
        if s == e && ins.is_empty() {
            return;
        }
        let typing = is_insert && ins.len() == 1 && ins[0] != '\n' && ins[0] != '\r';
        let removed = self.primitive(s, e, &ins);
        self.record(
            Step::Op {
                at: s,
                removed,
                inserted: ins,
            },
            typing,
        );
    }

    /// Naive batch: last to first, mapping through each edit; one step unless all no-ops.
    fn batch(&mut self, edits: &[(Range<usize>, String)]) {
        if edits.iter().all(|(r, t)| r.is_empty() && t.is_empty()) {
            return;
        }
        let before = self.text.clone();
        for (r, t) in edits.iter().rev() {
            self.primitive(r.start, r.end, &chars(t));
        }
        let after = self.text.clone();
        self.typing_open = false;
        self.record(Step::Batch { before, after }, false);
    }

    /// Returns (changed, positions known): after a batch undo the positions are unknown.
    fn undo(&mut self) -> (bool, bool) {
        let Some(step) = self.undo.pop() else {
            return (false, true);
        };
        self.typing_open = false;
        let known = match &step {
            Step::Op {
                at,
                removed,
                inserted,
            } => {
                self.primitive(*at, at + inserted.len(), removed);
                true
            }
            Step::Batch { before, .. } => {
                self.text = before.clone();
                false
            }
        };
        self.redo.push(step);
        (true, known)
    }

    fn redo(&mut self) -> (bool, bool) {
        let Some(step) = self.redo.pop() else {
            return (false, true);
        };
        self.typing_open = false;
        let known = match &step {
            Step::Op {
                at,
                removed,
                inserted,
            } => {
                self.primitive(*at, at + removed.len(), inserted);
                true
            }
            Step::Batch { after, .. } => {
                self.text = after.clone();
                false
            }
        };
        self.undo.push(step);
        (true, known)
    }
}

/// The buffer's selection and the given anchors, as the model's `pos` layout.
fn observed(b: &Buffer, ids: &[AnchorId]) -> Vec<usize> {
    let s = b.selection();
    let mut v = vec![s.anchor, s.head];
    v.extend(ids.iter().map(|id| b.anchor(*id).expect("live anchor")));
    v
}

// ---------------------------------------------------------------------------------------------
// AC-6: batches
// ---------------------------------------------------------------------------------------------

/// AC-6: `apply_edits` on random sorted, non-overlapping edit lists equals the naive
/// last-to-first application (text, selection and anchors mapped through each edit); one
/// `undo` restores the text exactly and `redo` re-applies it; invalid lists give the pinned
/// `BatchError` and change nothing (text, selection, anchors, history, typing group); a batch
/// ends an open typing group and never joins one; an empty or all-no-op list changes nothing.
#[test]
fn batch_edits_and_anchors() {
    let mut rng = Rng(0x7018_BA7C);
    for case in 0..400 {
        let original = random_text(&mut rng, 0, 12);
        let text = chars(&original);
        let mut b = Buffer::from_text(&original);
        assert_eq!(
            b.selection(),
            Selection { anchor: 0, head: 0 },
            "new buffer"
        );
        // An anchor at every valid position, so every boundary case is covered.
        let valid: Vec<usize> = (0..=text.len())
            .filter(|&p| !inside_crlf(&text, p))
            .collect();
        let ids: Vec<AnchorId> = valid
            .iter()
            .map(|&p| b.add_anchor(p).expect("valid anchor"))
            .collect();
        let sel = Selection {
            anchor: valid_pos(&mut rng, &text, &[]),
            head: valid_pos(&mut rng, &text, &[]),
        };
        b.set_selection(sel).expect("valid selection");
        let mut m = Model {
            text: text.clone(),
            pos: [vec![sel.anchor, sel.head], valid.clone()].concat(),
            ..Model::default()
        };
        let edits = random_edits(&mut rng, &text, &valid);
        let ctx = format!("case {case}: {original:?} edits {edits:?} selection {sel:?}");
        b.apply_edits(&as_refs(&edits))
            .unwrap_or_else(|e| panic!("{ctx}: {e:?}"));
        m.batch(&edits);
        let want: String = m.text.iter().collect();
        assert_eq!(b.text(), want, "{ctx}");
        assert_eq!(
            observed(&b, &ids),
            m.pos,
            "{ctx}: [sel.anchor, sel.head, anchors...]"
        );
        let s = b.selection();
        assert_eq!(s.start(), s.anchor.min(s.head), "{ctx}");
        assert_eq!(s.end(), s.anchor.max(s.head), "{ctx}");
        assert_eq!(s.is_empty(), s.anchor == s.head, "{ctx}");
        if m.undo.is_empty() {
            // Empty or all no-ops: the history is unchanged.
            assert!(!b.can_undo(), "{ctx}");
            assert!(!b.undo(), "{ctx}");
        } else {
            assert!(b.undo(), "{ctx}");
            assert_eq!(b.text(), original, "{ctx}: one undo restores");
            assert!(!b.can_undo(), "{ctx}: one step");
            assert!(b.redo(), "{ctx}");
            assert_eq!(b.text(), want, "{ctx}: redo re-applies");
            assert!(!b.can_redo(), "{ctx}");
        }
    }

    // Invalid lists: the first failing index in order, and nothing changes.
    let len = 7; // "ab\r\ncdé"
    let oob = |index| EditError::OutOfBounds {
        index,
        len_chars: len,
    };
    let invalid: Vec<(Vec<(Range<usize>, &str)>, BatchError)> = vec![
        (
            vec![(2..3, "x"), (2..4, "y")],
            BatchError::Unordered { index: 1 },
        ),
        (
            vec![(2..4, "x"), (3..5, "y")],
            BatchError::Unordered { index: 1 },
        ),
        (
            vec![(3..4, ""), (1..2, "")],
            BatchError::Unordered { index: 1 },
        ),
        (
            vec![(1..1, "x"), (1..1, "y")],
            BatchError::Unordered { index: 1 },
        ),
        (
            vec![(0..1, ""), (1..2, ""), (1..3, ""), (4..99, "")],
            BatchError::Unordered { index: 2 },
        ),
        (
            vec![(0..1, "x"), (2..9, "y")],
            BatchError::Edit {
                index: 1,
                error: oob(9),
            },
        ),
        (
            vec![(8..8, "x")],
            BatchError::Edit {
                index: 0,
                error: oob(8),
            },
        ),
        (
            vec![(0..1, ""), (2..99, ""), (1..1, "")],
            BatchError::Edit {
                index: 1,
                error: oob(99),
            },
        ),
        (
            vec![(5..3, "")],
            BatchError::Edit {
                index: 0,
                error: EditError::InvertedRange { start: 5, end: 3 },
            },
        ),
        (
            vec![(0..1, "x"), (3..2, "")],
            BatchError::Edit {
                index: 1,
                error: EditError::InvertedRange { start: 3, end: 2 },
            },
        ),
    ];
    for (edits, want) in invalid {
        let mut b = Buffer::from_text("b\r\ncdé");
        let ids: Vec<AnchorId> = [0, 1, 3, 6]
            .iter()
            .map(|&p| b.add_anchor(p).expect("valid anchor"))
            .collect();
        b.set_selection(Selection { anchor: 6, head: 3 })
            .expect("valid selection");
        b.insert(0, "a").expect("insert"); // opens a typing group
        let text = b.text();
        assert_eq!(text, "ab\r\ncdé");
        let before = observed(&b, &ids);
        assert_eq!(b.apply_edits(&edits), Err(want), "{edits:?}");
        assert_eq!(b.text(), text, "{edits:?}");
        assert_eq!(observed(&b, &ids), before, "{edits:?}");
        assert!(!b.can_redo(), "{edits:?}");
        // The typing group is still open: "y" joins "a", one undo removes both.
        b.insert(1, "y").expect("insert");
        assert!(b.undo());
        assert_eq!(b.text(), "b\r\ncdé", "{edits:?}: typing group kept");
        assert!(!b.can_undo(), "{edits:?}");
    }

    // A batch ends an open typing group ...
    let mut b = Buffer::new();
    b.insert(0, "a").expect("insert");
    b.insert(1, "b").expect("insert");
    b.apply_edits(&[(2..2, "c")]).expect("batch");
    assert!(b.undo());
    assert_eq!(b.text(), "ab");
    assert!(b.undo());
    assert_eq!(b.text(), "");
    // ... and never joins one.
    let mut b = Buffer::new();
    b.apply_edits(&[(0..0, "a")]).expect("batch");
    b.insert(1, "b").expect("insert");
    assert!(b.undo());
    assert_eq!(b.text(), "a");
    assert!(b.undo());
    assert_eq!(b.text(), "");
    // An empty or all-no-op list leaves the history and the typing group unchanged.
    let mut b = Buffer::new();
    b.insert(0, "a").expect("insert");
    b.insert(1, "b").expect("insert");
    b.apply_edits(&[]).expect("empty batch");
    b.apply_edits(&[(1..1, ""), (2..2, "")])
        .expect("no-op batch");
    b.insert(2, "c").expect("insert");
    assert!(b.undo());
    assert_eq!(b.text(), "");
    assert!(!b.can_undo());
    // A successful batch clears redo.
    let mut b = Buffer::from_text("xyz");
    b.insert(0, "a").expect("insert");
    assert!(b.undo());
    assert!(b.can_redo());
    b.apply_edits(&[(0..1, "X"), (2..3, "")]).expect("batch");
    assert_eq!(b.text(), "Xy");
    assert!(!b.can_redo());
}

// ---------------------------------------------------------------------------------------------
// Anchors and selection through random edit sequences
// ---------------------------------------------------------------------------------------------

/// Anchors and the selection follow the mapping rule through random sequences of inserts,
/// deletes, replaces, undo, redo and batches (at, before, after and across tracked positions;
/// 2-, 3- and 4-byte chars; CRLF made and split), compared with the naive model after every
/// step. After undoing or redoing a batch the positions are re-read (their mapping depends on
/// how the batch is undone, which the spec leaves open), the text is still checked.
#[test]
fn anchors_follow_random_edit_sequences() {
    let mut rng = Rng(0x7018_A11C);
    for seq in 0..40 {
        let original = random_text(&mut rng, 4, 10);
        let mut b = Buffer::from_text(&original);
        let mut m = Model {
            text: chars(&original),
            pos: vec![0, 0],
            ..Model::default()
        };
        let mut ids: Vec<AnchorId> = Vec::new();
        let mut removed_ids: Vec<AnchorId> = Vec::new();
        let mut last_typed: Option<usize> = None;
        for step in 0..150 {
            let len = m.text.len();
            let typed = last_typed.take();
            let roll = rng.below(100);
            let what;
            match roll {
                0..=14 if typed.is_some() => {
                    let at = typed.expect("checked");
                    let c = *rng.pick(&["a", "é", "€", "😀"]);
                    what = format!("insert({at}, {c:?})");
                    b.insert(at, c).expect("insert");
                    m.edit(at, at, c, true);
                    last_typed = Some(at + 1);
                }
                0..=24 => {
                    let at = rng.below(len + 1);
                    let at = if rng.chance(50) {
                        *rng.pick(&m.pos)
                    } else {
                        at
                    };
                    let t = if rng.chance(40) {
                        (*rng.pick(&["\n", "\r", "x", "😀"])).to_owned()
                    } else {
                        random_text(&mut rng, 1, 3)
                    };
                    what = format!("insert({at}, {t:?})");
                    b.insert(at, &t).expect("insert");
                    m.edit(at, at, &t, true);
                    if t.chars().count() == 1 {
                        last_typed = Some(at + 1);
                    }
                }
                25..=44 => {
                    let near = m.pos.clone();
                    let a = if rng.chance(60) {
                        *rng.pick(&near)
                    } else {
                        rng.below(len + 1)
                    };
                    let a = match rng.below(3) {
                        0 => a.saturating_sub(1),
                        1 => (a + 1).min(len),
                        _ => a,
                    };
                    let e = (a + rng.below(4)).min(len);
                    let t = if rng.chance(50) {
                        String::new()
                    } else {
                        random_text(&mut rng, 1, 2)
                    };
                    what = format!("replace({a}..{e}, {t:?})");
                    b.replace(a..e, &t).expect("replace");
                    m.edit(a, e, &t, false);
                }
                45..=56 => {
                    what = "undo".to_owned();
                    let (changed, known) = m.undo();
                    assert_eq!(b.undo(), changed, "seq {seq} step {step}: undo");
                    if !known {
                        m.pos = observed(&b, &ids);
                    }
                }
                57..=64 => {
                    what = "redo".to_owned();
                    let (changed, known) = m.redo();
                    assert_eq!(b.redo(), changed, "seq {seq} step {step}: redo");
                    if !known {
                        m.pos = observed(&b, &ids);
                    }
                }
                65..=76 => {
                    let near = m.pos.clone();
                    let edits = random_edits(&mut rng, &m.text, &near);
                    what = format!("apply_edits({edits:?})");
                    b.apply_edits(&as_refs(&edits)).expect("valid batch");
                    m.batch(&edits);
                }
                77..=84 => {
                    let p = valid_pos(&mut rng, &m.text, &m.pos.clone());
                    what = format!("add_anchor({p})");
                    ids.push(b.add_anchor(p).expect("valid anchor"));
                    m.pos.push(p);
                    b.break_undo_group();
                    m.typing_open = false;
                }
                85..=88 if !ids.is_empty() => {
                    let i = rng.below(ids.len());
                    what = format!("remove_anchor(#{i})");
                    let id = ids.remove(i);
                    m.pos.remove(i + 2);
                    assert!(b.remove_anchor(id));
                    removed_ids.push(id);
                    b.break_undo_group();
                    m.typing_open = false;
                }
                _ => {
                    let near = m.pos.clone();
                    let s = Selection {
                        anchor: valid_pos(&mut rng, &m.text, &near),
                        head: valid_pos(&mut rng, &m.text, &near),
                    };
                    what = format!("set_selection({s:?})");
                    b.set_selection(s).expect("valid selection");
                    m.pos[0] = s.anchor;
                    m.pos[1] = s.head;
                    b.break_undo_group();
                    m.typing_open = false;
                }
            }
            let ctx = format!("seq {seq} step {step} after {what}, text {:?}", b.text());
            let want: String = m.text.iter().collect();
            assert_eq!(b.text(), want, "{ctx}");
            assert_eq!(
                observed(&b, &ids),
                m.pos,
                "{ctx}: [sel.anchor, sel.head, anchors...]"
            );
            for id in &removed_ids {
                assert_eq!(b.anchor(*id), None, "{ctx}: removed anchor");
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Validity, ids, movement, line endings
// ---------------------------------------------------------------------------------------------

/// `set_selection` and `add_anchor` refuse a position beyond the text (`OutOfBounds`) or
/// between `\r` and `\n` (`InsideLineBreak`), anchor before head, and change nothing; anchor
/// ids are distinct; a removed anchor reads `None` and cannot be removed twice.
#[test]
fn selection_and_anchor_validity() {
    let mut b = Buffer::from_text("ab\r\ncé😀"); // 7 chars; 3 is inside the CRLF
    b.set_selection(Selection { anchor: 1, head: 6 })
        .expect("valid");
    let keep = Selection { anchor: 1, head: 6 };
    let cases = [
        (
            Selection { anchor: 8, head: 0 },
            PositionError::OutOfBounds { index: 8, len: 7 },
        ),
        (
            Selection { anchor: 0, head: 9 },
            PositionError::OutOfBounds { index: 9, len: 7 },
        ),
        (
            Selection { anchor: 3, head: 0 },
            PositionError::InsideLineBreak { char_idx: 3 },
        ),
        (
            Selection { anchor: 0, head: 3 },
            PositionError::InsideLineBreak { char_idx: 3 },
        ),
        (
            Selection { anchor: 3, head: 9 },
            PositionError::InsideLineBreak { char_idx: 3 },
        ),
        (
            Selection { anchor: 9, head: 3 },
            PositionError::OutOfBounds { index: 9, len: 7 },
        ),
    ];
    for (s, want) in cases {
        assert_eq!(b.set_selection(s), Err(want), "{s:?}");
        assert_eq!(b.selection(), keep, "{s:?}");
    }
    let s = Selection { anchor: 7, head: 2 };
    b.set_selection(s).expect("ends are valid");
    assert_eq!(b.selection(), s);
    assert_eq!((s.start(), s.end(), s.is_empty()), (2, 7, false));
    let caret = Selection { anchor: 4, head: 4 };
    assert_eq!((caret.start(), caret.end(), caret.is_empty()), (4, 4, true));

    assert_eq!(
        b.add_anchor(8),
        Err(PositionError::OutOfBounds { index: 8, len: 7 })
    );
    assert_eq!(
        b.add_anchor(3),
        Err(PositionError::InsideLineBreak { char_idx: 3 })
    );
    let ids: Vec<AnchorId> = [0, 2, 2, 4, 7]
        .iter()
        .map(|&p| b.add_anchor(p).expect("valid"))
        .collect();
    let mut sorted = ids.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), ids.len(), "distinct ids");
    assert_eq!(b.anchor(ids[1]), Some(2));
    assert!(b.remove_anchor(ids[1]));
    assert_eq!(b.anchor(ids[1]), None);
    assert!(!b.remove_anchor(ids[1]));
    assert_eq!(b.anchor(ids[2]), Some(2), "the other anchor at 2 stays");
    // Anchors follow edits; a removed one stays removed.
    b.insert(0, "é").expect("insert");
    assert_eq!(
        ids.iter().map(|id| b.anchor(*id)).collect::<Vec<_>>(),
        vec![Some(1), None, Some(3), Some(5), Some(8)]
    );
    assert_eq!(b.selection(), Selection { anchor: 8, head: 3 });
}

/// The worked examples of Pinned (g) and the movement rule's ends, through `Buffer::moved`
/// (which does not change the buffer).
#[test]
fn movement_worked_examples() {
    let b = Buffer::from_text("fn foo_bar(x: u8)");
    let mut head = 0;
    let mut heads = Vec::new();
    for _ in 0..4 {
        head = b.moved(head, Movement::Word, true);
        heads.push(head);
    }
    assert_eq!(heads, [2, 10, 12, 16]);
    assert_eq!(b.moved(16, Movement::Word, true), 17);
    heads.clear();
    for _ in 0..4 {
        head = b.moved(head, Movement::Word, false);
        heads.push(head);
    }
    assert_eq!(heads, [14, 11, 3, 0]);
    assert_eq!(b.moved(0, Movement::Word, false), 0);
    assert_eq!(b.selection(), Selection { anchor: 0, head: 0 });

    let b = Buffer::from_text("abcdef\nxy\nlonger");
    assert_eq!(b.moved(5, Movement::Line, true), 9);
    assert_eq!(b.moved(9, Movement::Line, true), 12);
    assert_eq!(b.moved(12, Movement::Line, true), 16);
    assert_eq!(b.moved(12, Movement::Line, false), 9);
    assert_eq!(b.moved(9, Movement::Line, false), 2);
    assert_eq!(b.moved(4, Movement::Line, false), 0);

    let b = Buffer::from_text("a\r\nb");
    assert_eq!(b.moved(1, Movement::Char, true), 3);
    assert_eq!(b.moved(3, Movement::Char, false), 1);
    assert_eq!(b.moved(4, Movement::Char, true), 4);
    assert_eq!(b.moved(0, Movement::Char, false), 0);
    // Columns count chars: line movement over multi-byte chars and a lone CR.
    let b = Buffer::from_text("é😀€x\rab\r\n😀");
    assert_eq!(b.moved(3, Movement::Line, true), 7);
    assert_eq!(b.moved(7, Movement::Line, true), 10);
    assert_eq!(b.moved(10, Movement::Line, false), 6);
    assert_eq!(b.moved(6, Movement::Line, false), 1);
    assert_eq!(b.moved(3, Movement::Char, true), 4);
    assert_eq!(b.moved(4, Movement::Char, true), 5);
    assert_eq!(b.moved(7, Movement::Char, true), 9);
    assert_eq!(b.moved(9, Movement::Char, false), 7);
}

/// `normalize_line_endings` rewrites every break as `to` (CRLF matched first), returns the
/// number of breaks rewritten, is one undo step, and is idempotent (a second call rewrites
/// nothing and, as a no-op, adds no undo step).
#[test]
fn normalize_line_endings_is_one_undo_step_and_idempotent() {
    let mut rng = Rng(0x7018_E0E0);
    let mut texts: Vec<String> = ["a\r\nb\nc\rd", "\r\r\n\n\r", "é😀\n€\r", "flat", ""]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    for _ in 0..60 {
        texts.push(random_text(&mut rng, 1, 12));
    }
    for text in &texts {
        for (to, brk) in [
            (LineEnding::Lf, "\n"),
            (LineEnding::Crlf, "\r\n"),
            (LineEnding::Cr, "\r"),
        ] {
            let c = chars(text);
            let (mut want, mut changed, mut i) = (String::new(), 0, 0);
            while i < c.len() {
                let n = match c[i] {
                    '\r' if c.get(i + 1) == Some(&'\n') => 2,
                    '\r' | '\n' => 1,
                    _ => 0,
                };
                if n == 0 {
                    want.push(c[i]);
                    i += 1;
                } else {
                    let old: String = c[i..i + n].iter().collect();
                    changed += usize::from(old != brk);
                    want.push_str(brk);
                    i += n;
                }
            }
            let mut b = Buffer::from_text(text);
            assert_eq!(b.normalize_line_endings(to), changed, "{text:?} {to:?}");
            assert_eq!(b.text(), want, "{text:?} {to:?}");
            assert_eq!(
                b.normalize_line_endings(to),
                0,
                "{text:?} {to:?}: idempotent"
            );
            assert_eq!(b.text(), want, "{text:?} {to:?}: idempotent");
            if changed == 0 {
                assert!(!b.can_undo(), "{text:?} {to:?}: nothing rewritten");
                continue;
            }
            assert!(b.undo());
            assert_eq!(b.text(), *text, "{text:?} {to:?}: one undo restores");
            assert!(!b.can_undo(), "{text:?} {to:?}: one step");
            assert!(b.redo());
            assert_eq!(b.text(), want, "{text:?} {to:?}: redo");
        }
    }
}
