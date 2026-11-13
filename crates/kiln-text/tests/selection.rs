//! T-018 implementer tests: selection, anchors, batch edits, movement and line-ending
//! normalisation of `kiln_text::Buffer`, against exact values and a naive model.

use std::ops::Range;

use kiln_text::{BatchError, Buffer, EditError, LineEnding, Movement, PositionError, Selection};

fn sel(anchor: usize, head: usize) -> Selection {
    Selection { anchor, head }
}

// ---- naive model ------------------------------------------------------------------------------

/// One recorded primitive edit, `at` in the text before its step.
#[derive(Clone, Debug)]
struct MOp {
    at: usize,
    removed: Vec<char>,
    inserted: Vec<char>,
}

/// A naive model: `Vec<char>` text, positions mapped edit by edit, steps as lists of ops.
#[derive(Clone, Debug, Default)]
struct Model {
    text: Vec<char>,
    sel: (usize, usize),
    anchors: Vec<Option<usize>>,
    undo: Vec<Vec<MOp>>,
    redo: Vec<Vec<MOp>>,
    typing_open: bool,
}

impl Model {
    fn new(text: &str) -> Self {
        Self {
            text: text.chars().collect(),
            ..Self::default()
        }
    }

    fn string(&self) -> String {
        self.text.iter().collect()
    }

    fn inside_crlf(&self, p: usize) -> bool {
        p > 0 && p < self.text.len() && self.text[p - 1] == '\r' && self.text[p] == '\n'
    }

    fn valid(&self, p: usize) -> Result<(), PositionError> {
        if p > self.text.len() {
            Err(PositionError::OutOfBounds {
                index: p,
                len: self.text.len(),
            })
        } else if self.inside_crlf(p) {
            Err(PositionError::InsideLineBreak { char_idx: p })
        } else {
            Ok(())
        }
    }

    /// Replace `[s, e)` by `ins` and map every position (Mapping rule + CRLF snap).
    fn prim(&mut self, s: usize, e: usize, ins: &[char]) -> Vec<char> {
        let removed: Vec<char> = self.text.splice(s..e, ins.iter().copied()).collect();
        let n = ins.len();
        let map = |p: usize| {
            if p < s {
                p
            } else if p > e {
                p + n - (e - s)
            } else if p == s && e > s {
                s
            } else {
                s + n
            }
        };
        let snap = |m: &Model, p: usize| if m.inside_crlf(p) { p - 1 } else { p };
        let (a, h) = self.sel;
        self.sel = (snap(self, map(a)), snap(self, map(h)));
        for k in 0..self.anchors.len() {
            if let Some(p) = self.anchors[k] {
                self.anchors[k] = Some(snap(self, map(p)));
            }
        }
        removed
    }

    fn check_range(&self, r: &Range<usize>) -> Result<(), EditError> {
        let len_chars = self.text.len();
        if r.start > r.end {
            Err(EditError::InvertedRange {
                start: r.start,
                end: r.end,
            })
        } else if r.start > len_chars {
            Err(EditError::OutOfBounds {
                index: r.start,
                len_chars,
            })
        } else if r.end > len_chars {
            Err(EditError::OutOfBounds {
                index: r.end,
                len_chars,
            })
        } else {
            Ok(())
        }
    }

    fn single(&mut self, r: Range<usize>, text: &str, typing: bool) -> Result<(), EditError> {
        self.check_range(&r)?;
        if r.is_empty() && text.is_empty() {
            return Ok(());
        }
        let ins: Vec<char> = text.chars().collect();
        let removed = self.prim(r.start, r.end, &ins);
        self.redo.clear();
        if typing
            && self.typing_open
            && let Some([top]) = self.undo.last_mut().map(Vec::as_mut_slice)
            && top.at + top.inserted.len() == r.start
        {
            top.inserted.extend(ins);
            return Ok(());
        }
        self.undo.push(vec![MOp {
            at: r.start,
            removed,
            inserted: ins,
        }]);
        self.typing_open = typing;
        Ok(())
    }

    fn insert(&mut self, at: usize, text: &str) -> Result<(), EditError> {
        let mut c = text.chars();
        let typing = matches!((c.next(), c.next()), (Some(ch), None) if ch != '\n' && ch != '\r');
        self.single(at..at, text, typing)
    }

    fn batch(&mut self, edits: &[(Range<usize>, &str)]) -> Result<(), BatchError> {
        for (i, (r, _)) in edits.iter().enumerate() {
            self.check_range(r)
                .map_err(|error| BatchError::Edit { index: i, error })?;
            if i > 0 {
                let p = &edits[i - 1].0;
                if !(p.start < r.start && p.end <= r.start) {
                    return Err(BatchError::Unordered { index: i });
                }
            }
        }
        let mut ops = Vec::new();
        for (r, t) in edits.iter().rev() {
            if r.is_empty() && t.is_empty() {
                continue;
            }
            let ins: Vec<char> = t.chars().collect();
            let removed = self.prim(r.start, r.end, &ins);
            ops.insert(
                0,
                MOp {
                    at: r.start,
                    removed,
                    inserted: ins,
                },
            );
        }
        if !ops.is_empty() {
            self.redo.clear();
            self.undo.push(ops);
            self.typing_open = false;
        }
        Ok(())
    }

    fn undo(&mut self) -> bool {
        let Some(step) = self.undo.pop() else {
            return false;
        };
        self.typing_open = false;
        // Where each op's inserted text lies after the step.
        let mut starts = Vec::new();
        let mut delta: isize = 0;
        for op in &step {
            starts.push((op.at as isize + delta) as usize);
            delta += op.inserted.len() as isize - op.removed.len() as isize;
        }
        // w1: inverse edits from the highest position to the lowest.
        for (op, at) in step.iter().zip(starts).rev() {
            self.prim(at, at + op.inserted.len(), &op.removed.clone());
        }
        self.redo.push(step);
        true
    }

    fn redo(&mut self) -> bool {
        let Some(step) = self.redo.pop() else {
            return false;
        };
        self.typing_open = false;
        for op in step.iter().rev() {
            self.prim(op.at, op.at + op.removed.len(), &op.inserted.clone());
        }
        self.undo.push(step);
        true
    }

    fn normalize(&mut self, to: LineEnding) -> usize {
        let target = match to {
            LineEnding::Lf => "\n",
            LineEnding::Crlf => "\r\n",
            LineEnding::Cr => "\r",
        };
        let mut edits = Vec::new();
        let mut i = 0;
        while i < self.text.len() {
            let (kind, w) = match (self.text[i], self.text.get(i + 1)) {
                ('\r', Some('\n')) => (Some(LineEnding::Crlf), 2),
                ('\r', _) => (Some(LineEnding::Cr), 1),
                ('\n', _) => (Some(LineEnding::Lf), 1),
                _ => (None, 1),
            };
            if let Some(k) = kind
                && k != to
            {
                edits.push((i..i + w, target));
            }
            i += w;
        }
        self.batch(&edits).unwrap();
        edits.len()
    }
}

struct Lcg(u64);

impl Lcg {
    fn next(&mut self, bound: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % bound.max(1)
    }

    fn text(&mut self, max: usize) -> String {
        const PIECES: &[&str] = &["a", "b", "_", " ", "é", "😀", "\n", "\r", "\r\n", "(", "1"];
        let n = self.next(max + 1);
        (0..n).map(|_| PIECES[self.next(PIECES.len())]).collect()
    }
}

fn assert_same(b: &Buffer, m: &Model, ids: &[kiln_text::AnchorId], ctx: &str) {
    assert_eq!(b.text(), m.string(), "text {ctx}");
    assert_eq!(b.selection(), sel(m.sel.0, m.sel.1), "selection {ctx}");
    for (id, p) in ids.iter().zip(&m.anchors) {
        assert_eq!(b.anchor(*id), *p, "anchor {ctx}");
    }
    assert_eq!(b.can_undo(), !m.undo.is_empty(), "can_undo {ctx}");
    assert_eq!(b.can_redo(), !m.redo.is_empty(), "can_redo {ctx}");
}

/// Random sorted (mostly valid) edit lists over the current length.
fn random_edits(g: &mut Lcg, len: usize) -> Vec<(Range<usize>, String)> {
    let mut edits = Vec::new();
    let mut min_start = 0;
    for _ in 0..g.next(5) {
        let start = min_start + g.next(4);
        if start > len {
            break;
        }
        let end = (start + g.next(3)).min(len);
        let text = if g.next(3) == 0 {
            String::new()
        } else {
            g.text(3)
        };
        edits.push((start..end, text));
        min_start = end.max(start + 1);
    }
    if g.next(6) == 0 && !edits.is_empty() {
        // Break the list: out of bounds, inverted, or unordered.
        let k = g.next(edits.len());
        match g.next(3) {
            0 => edits[k].0.end = len + 1 + g.next(3),
            1 => edits[k].0 = edits[k].0.end + 1..edits[k].0.start,
            _ => {
                let r = edits[k].0.clone();
                edits.push((r, "z".into()));
            }
        }
    }
    edits
}

// ---- differential test ------------------------------------------------------------------------

#[test]
fn random_sessions_match_naive_model() {
    for seed in [1u64, 2, 0x018, 0xfeed] {
        let mut g = Lcg(seed);
        let initial = g.text(40);
        let mut b = Buffer::from_text(&initial);
        let mut m = Model::new(&initial);
        let mut ids = Vec::new();
        for step in 0..1500 {
            let len = m.text.len();
            let ctx = format!("seed {seed} step {step}");
            match g.next(12) {
                0 | 1 => {
                    let at = g.next(len + 2);
                    let t = ["x", "é", "😀", "\n", "\r", "ab"][g.next(6)];
                    assert_eq!(b.insert(at, t), m.insert(at, t), "{ctx}");
                }
                2 => {
                    let s = g.next(len + 2);
                    let e = s + g.next(4);
                    assert_eq!(
                        b.delete(s..e).map(|_| ()),
                        m.single(s..e, "", false),
                        "{ctx}"
                    );
                }
                3 => {
                    let s = g.next(len + 1);
                    let e = (s + g.next(4)).min(len);
                    let t = g.text(3);
                    assert_eq!(
                        b.replace(s..e, &t).map(|_| ()),
                        m.single(s..e, &t, false),
                        "{ctx}"
                    );
                }
                4 | 5 => {
                    let owned = random_edits(&mut g, len);
                    let edits: Vec<(Range<usize>, &str)> =
                        owned.iter().map(|(r, t)| (r.clone(), t.as_str())).collect();
                    assert_eq!(b.apply_edits(&edits), m.batch(&edits), "{ctx} {edits:?}");
                }
                6 => assert_eq!(b.undo(), m.undo(), "{ctx}"),
                7 => assert_eq!(b.redo(), m.redo(), "{ctx}"),
                8 => {
                    let (a, h) = (g.next(len + 2), g.next(len + 2));
                    let expected = m.valid(a).and_then(|()| m.valid(h));
                    if expected.is_ok() {
                        m.sel = (a, h);
                    }
                    assert_eq!(b.set_selection(sel(a, h)), expected, "{ctx}");
                }
                9 => {
                    let p = g.next(len + 2);
                    match (b.add_anchor(p), m.valid(p)) {
                        (Ok(id), Ok(())) => {
                            ids.push(id);
                            m.anchors.push(Some(p));
                        }
                        (got, want) => assert_eq!(got.map(|_| ()), want, "{ctx}"),
                    }
                }
                10 if !ids.is_empty() => {
                    let k = g.next(ids.len());
                    assert_eq!(b.remove_anchor(ids[k]), m.anchors[k].is_some(), "{ctx}");
                    m.anchors[k] = None;
                }
                10 => b.break_undo_group(),
                _ => {
                    let to = [LineEnding::Lf, LineEnding::Crlf, LineEnding::Cr][g.next(3)];
                    assert_eq!(b.normalize_line_endings(to), m.normalize(to), "{ctx}");
                }
            }
            assert_same(&b, &m, &ids, &ctx);
        }
        while m.undo() {
            assert!(b.undo());
            assert_same(&b, &m, &ids, "final undo");
        }
        assert!(!b.undo());
        assert_eq!(b.text(), initial);
    }
}

// ---- Mapping rule -----------------------------------------------------------------------------

#[test]
fn mapping_rule_four_cases_through_edits() {
    // Replace [2, 4) of "abcdef" by "XYZ": positions 1 (before), 2 (== s), 3 (inside),
    // 4 (== e), 5 (after).
    let mut b = Buffer::from_text("abcdef");
    let ids: Vec<_> = [1, 2, 3, 4, 5].map(|p| b.add_anchor(p).unwrap()).to_vec();
    b.replace(2..4, "XYZ").unwrap();
    let got: Vec<_> = ids.iter().map(|&id| b.anchor(id).unwrap()).collect();
    assert_eq!(got, [1, 2, 5, 5, 6]);
    // Pure insertion: a caret at the insertion point ends up after the text.
    let mut b = Buffer::from_text("abc");
    b.set_selection(sel(0, 1)).unwrap();
    b.insert(1, "QQ").unwrap();
    assert_eq!(b.selection(), sel(0, 3));
    // Delete: positions inside the range collapse to its start.
    let mut b = Buffer::from_text("abcdef");
    b.set_selection(sel(5, 3)).unwrap();
    b.delete(1..4).unwrap();
    assert_eq!(b.selection(), sel(2, 1));
}

#[test]
fn mapped_position_inside_crlf_snaps_before_cr() {
    // Inserting "\r" before an LF forms a CRLF; the caret at the insertion point would land
    // between "\r" and "\n" and moves to before the "\r".
    let mut b = Buffer::from_text("a\nb");
    b.set_selection(sel(1, 1)).unwrap();
    let id = b.add_anchor(1).unwrap();
    b.insert(1, "\r").unwrap();
    assert_eq!(b.text(), "a\r\nb");
    assert_eq!(b.selection(), sel(1, 1));
    assert_eq!(b.anchor(id), Some(1));
    // Deleting the char between "\r" and "\n" joins them; the end position snaps too.
    let mut b = Buffer::from_text("a\rX\nb");
    b.set_selection(sel(3, 0)).unwrap();
    b.delete(2..3).unwrap();
    assert_eq!(b.selection(), sel(1, 0));
}

#[test]
fn worked_example_selection_follows_insert_and_delete() {
    let mut b = Buffer::from_text("abc\r\ndef");
    b.set_selection(sel(1, 6)).unwrap();
    b.insert(0, "XY").unwrap();
    assert_eq!(b.selection(), sel(3, 8));
    b.delete(4..7).unwrap();
    assert_eq!(b.text(), "XYabdef");
    assert_eq!(b.selection(), sel(3, 5));
}

#[test]
fn invalid_selection_and_anchor_positions_are_refused() {
    let mut b = Buffer::from_text("a\r\nb");
    b.set_selection(sel(1, 0)).unwrap();
    assert_eq!(
        b.set_selection(sel(9, 2)),
        Err(PositionError::OutOfBounds { index: 9, len: 4 })
    );
    // anchor is checked before head
    assert_eq!(
        b.set_selection(sel(2, 9)),
        Err(PositionError::InsideLineBreak { char_idx: 2 })
    );
    assert_eq!(b.selection(), sel(1, 0));
    assert_eq!(
        b.add_anchor(2),
        Err(PositionError::InsideLineBreak { char_idx: 2 })
    );
    assert_eq!(
        b.add_anchor(5),
        Err(PositionError::OutOfBounds { index: 5, len: 4 })
    );
    let id = b.add_anchor(4).unwrap();
    assert!(b.remove_anchor(id));
    assert!(!b.remove_anchor(id));
    assert_eq!(b.anchor(id), None);
    assert_eq!(Buffer::new().selection(), sel(0, 0));
}

// ---- Batch rule -------------------------------------------------------------------------------

#[test]
fn batch_is_one_undo_step_and_maps_positions() {
    let mut b = Buffer::from_text("one two three");
    b.set_selection(sel(4, 7)).unwrap();
    let id = b.add_anchor(13).unwrap();
    b.apply_edits(&[(0..3, "1"), (4..7, "2"), (8..13, "3")])
        .unwrap();
    assert_eq!(b.text(), "1 2 3");
    // 4 == s of a non-empty range -> 2; 7 == e -> s + n = 3; 13 == e -> 5.
    assert_eq!(b.selection(), sel(2, 3));
    assert_eq!(b.anchor(id), Some(5));
    assert!(b.undo());
    assert_eq!(b.text(), "one two three");
    assert!(!b.can_undo());
    assert!(b.redo());
    assert_eq!(b.text(), "1 2 3");
}

/// w1: a batch that deleted two adjacent ranges is undone by two insertions at one position,
/// highest first; the selection and anchors map through each.
#[test]
fn undo_of_adjacent_deletes_inserts_highest_first() {
    let mut b = Buffer::from_text("abcdef");
    b.apply_edits(&[(1..2, ""), (2..4, "")]).unwrap();
    assert_eq!(b.text(), "aef");
    b.set_selection(sel(1, 0)).unwrap();
    let at1 = b.add_anchor(1).unwrap();
    let at2 = b.add_anchor(2).unwrap();
    assert!(b.undo());
    assert_eq!(b.text(), "abcdef");
    // Inverse edits: insert "cd" at 1 (1 -> 3, 2 -> 4), then insert "b" at 1 (3 -> 4, 4 -> 5).
    assert_eq!(b.selection(), sel(4, 0));
    assert_eq!(b.anchor(at1), Some(4));
    assert_eq!(b.anchor(at2), Some(5));
    // Redo maps as the original batch did: delete [2, 4) first, then [1, 2).
    assert!(b.redo());
    assert_eq!(b.text(), "aef");
    assert_eq!(b.selection(), sel(1, 0));
    // [2, 4): 4 -> 2, 5 -> 3; then [1, 2): 2 -> 1, 3 -> 2.
    assert_eq!(b.anchor(at1), Some(1));
    assert_eq!(b.anchor(at2), Some(2));
}

#[test]
fn batch_ends_typing_group_and_is_never_joined() {
    let mut b = Buffer::new();
    b.insert(0, "a").unwrap();
    b.apply_edits(&[(1..1, "b")]).unwrap();
    b.insert(2, "c").unwrap();
    assert!(b.undo());
    assert_eq!(b.text(), "ab");
    assert!(b.undo());
    assert_eq!(b.text(), "a");
    assert!(b.undo());
    assert_eq!(b.text(), "");
}

#[test]
fn noop_batch_changes_nothing() {
    let mut b = Buffer::new();
    b.insert(0, "a").unwrap();
    assert!(b.undo());
    b.insert(0, "x").unwrap(); // clears redo, opens a group
    b.apply_edits(&[]).unwrap();
    b.apply_edits(&[(0..0, ""), (1..1, "")]).unwrap();
    b.insert(1, "y").unwrap(); // still joins
    assert!(b.undo());
    assert_eq!(b.text(), "");
    assert!(!b.can_undo());
}

/// w4 and Pinned (a): per index, own validity before order; first failing index.
#[test]
#[allow(clippy::reversed_empty_ranges)]
fn batch_error_precedence() {
    let mut b = Buffer::from_text("abc");
    b.set_selection(sel(1, 2)).unwrap();
    let cases: Vec<(Vec<(Range<usize>, &str)>, BatchError)> = vec![
        // out of bounds and not after its predecessor: the range error wins
        (
            vec![(0..1, "x"), (0..9, "y")],
            BatchError::Edit {
                index: 1,
                error: EditError::OutOfBounds {
                    index: 9,
                    len_chars: 3,
                },
            },
        ),
        // inverted and unordered: inverted wins
        (
            vec![(2..3, ""), (2..1, "")],
            BatchError::Edit {
                index: 1,
                error: EditError::InvertedRange { start: 2, end: 1 },
            },
        ),
        // first failing index: 1 is unordered, 2 is out of bounds
        (
            vec![(1..2, ""), (0..1, ""), (5..6, "")],
            BatchError::Unordered { index: 1 },
        ),
        // same start is unordered, even for two insertions
        (
            vec![(1..1, "p"), (1..1, "q")],
            BatchError::Unordered { index: 1 },
        ),
        // starts before the end of the previous edit
        (
            vec![(0..2, ""), (1..3, "")],
            BatchError::Unordered { index: 1 },
        ),
        (
            vec![(4..4, "")],
            BatchError::Edit {
                index: 0,
                error: EditError::OutOfBounds {
                    index: 4,
                    len_chars: 3,
                },
            },
        ),
    ];
    for (edits, err) in cases {
        assert_eq!(b.apply_edits(&edits), Err(err), "{edits:?}");
        assert_eq!(b.text(), "abc");
        assert_eq!(b.selection(), sel(1, 2));
        assert!(!b.can_undo());
    }
    // Adjacent ranges are allowed (end == next start).
    b.apply_edits(&[(0..1, "A"), (1..2, "B")]).unwrap();
    assert_eq!(b.text(), "ABc");
}

// ---- typing groups (w3) -----------------------------------------------------------------------

#[test]
fn selection_and_anchor_calls_do_not_touch_typing_group() {
    let mut b = Buffer::new();
    b.insert(0, "a").unwrap();
    b.set_selection(sel(0, 1)).unwrap();
    assert!(b.set_selection(sel(9, 9)).is_err());
    let id = b.add_anchor(0).unwrap();
    assert!(b.add_anchor(9).is_err());
    assert!(b.remove_anchor(id));
    assert!(!b.remove_anchor(id));
    b.insert(1, "b").unwrap();
    assert!(b.undo());
    assert_eq!(b.text(), "");
    assert!(!b.can_undo());
}

// ---- Movement rule ----------------------------------------------------------------------------

#[test]
fn movement_worked_examples() {
    let b = Buffer::from_text("fn foo_bar(x: u8)");
    let mut head = 0;
    let mut fwd = Vec::new();
    for _ in 0..4 {
        head = b.moved(head, Movement::Word, true);
        fwd.push(head);
    }
    assert_eq!(fwd, [2, 10, 12, 16]);
    let mut back = Vec::new();
    for _ in 0..4 {
        head = b.moved(head, Movement::Word, false);
        back.push(head);
    }
    assert_eq!(back, [14, 11, 3, 0]);
    assert_eq!(b.moved(16, Movement::Word, true), 17);
    assert_eq!(b.moved(17, Movement::Word, true), 17);
    assert_eq!(b.moved(0, Movement::Word, false), 0);

    let b = Buffer::from_text("abcdef\nxy\nlonger");
    let h1 = b.moved(5, Movement::Line, true);
    assert_eq!(h1, 9);
    assert_eq!(b.moved(h1, Movement::Line, true), 12);
    assert_eq!(b.moved(12, Movement::Line, true), 16); // last line: len_chars
    assert_eq!(b.moved(12, Movement::Line, false), 9);
    assert_eq!(b.moved(9, Movement::Line, false), 2);
    assert_eq!(b.moved(4, Movement::Line, false), 0); // first line: 0

    let b = Buffer::from_text("a\r\nb");
    assert_eq!(b.moved(1, Movement::Char, true), 3);
    assert_eq!(b.moved(3, Movement::Char, false), 1);
    assert_eq!(b.moved(4, Movement::Char, true), 4);
    assert_eq!(b.moved(0, Movement::Char, false), 0);
    assert_eq!(b.moved(3, Movement::Char, true), 4);
}

#[test]
fn line_movement_with_crlf_and_astral_chars() {
    // Lines: "é😀x" (CRLF), "" (CR), "ab".
    let b = Buffer::from_text("é😀x\r\n\rab");
    assert_eq!(b.moved(3, Movement::Line, true), 5); // col 3 clamped to 0 on the empty line
    assert_eq!(b.moved(5, Movement::Line, true), 6);
    assert_eq!(b.moved(8, Movement::Line, false), 5);
    assert_eq!(b.moved(5, Movement::Line, false), 0);
    assert_eq!(b.moved(2, Movement::Char, true), 3);
    assert_eq!(b.moved(5, Movement::Char, false), 3);
}

// ---- normalize (w2) ---------------------------------------------------------------------------

#[test]
fn normalize_worked_example_and_undo() {
    let mut b = Buffer::from_text("a\r\nb\nc\rd");
    b.set_selection(sel(3, 8)).unwrap();
    let id = b.add_anchor(7).unwrap();
    assert_eq!(b.normalize_line_endings(LineEnding::Lf), 2);
    assert_eq!(b.text(), "a\nb\nc\nd");
    assert_eq!(b.selection(), sel(2, 7));
    assert_eq!(b.anchor(id), Some(6));
    assert!(b.undo());
    assert_eq!(b.text(), "a\r\nb\nc\rd");
    assert!(!b.can_undo());
    assert!(b.redo());
    assert_eq!(b.text(), "a\nb\nc\nd");
    assert_eq!(b.normalize_line_endings(LineEnding::Crlf), 3);
    assert_eq!(b.text(), "a\r\nb\r\nc\r\nd");
}

#[test]
fn normalize_that_changes_nothing_leaves_history_and_group() {
    let mut b = Buffer::from_text("x\ny");
    b.insert(3, "!").unwrap();
    assert!(b.undo()); // a redo step exists
    b.set_selection(sel(1, 3)).unwrap();
    assert_eq!(b.normalize_line_endings(LineEnding::Lf), 0);
    assert!(b.can_redo());
    assert!(!b.can_undo());
    assert_eq!(b.selection(), sel(1, 3));
    // An open typing group stays open across a normalize that changes nothing.
    b.insert(0, "a").unwrap();
    b.insert(1, "b").unwrap();
    assert_eq!(b.normalize_line_endings(LineEnding::Lf), 0);
    assert_eq!(Buffer::new().normalize_line_endings(LineEnding::Cr), 0);
    b.insert(2, "c").unwrap();
    assert!(b.undo());
    assert_eq!(b.text(), "x\ny");
    assert!(!b.can_undo());
}

#[test]
fn normalize_that_changes_ends_typing_group() {
    let mut b = Buffer::from_text("x\ny");
    b.insert(0, "a").unwrap();
    assert_eq!(b.normalize_line_endings(LineEnding::Cr), 1);
    b.insert(1, "b").unwrap();
    assert_eq!(b.text(), "abx\ry");
    assert!(b.undo());
    assert_eq!(b.text(), "ax\ry");
    assert!(b.undo());
    assert_eq!(b.text(), "ax\ny");
    assert!(b.undo());
    assert_eq!(b.text(), "x\ny");
}

#[test]
fn line_break_counts() {
    let b = Buffer::from_text("a\r\r\nb\n\rc\r");
    assert_eq!(b.line_break_count(LineEnding::Lf), 1);
    assert_eq!(b.line_break_count(LineEnding::Crlf), 1);
    assert_eq!(b.line_break_count(LineEnding::Cr), 3);
}
