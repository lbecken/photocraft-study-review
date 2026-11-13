//! T-006 regression guards (D-010) for the D-013 spec-gap rulings on the text core: each test
//! passes on the base by design and fails if the ruling it pins is broken. Public API only.

use kiln_text::{Buffer, EditError};

fn oob(index: usize, len_chars: usize) -> EditError {
    EditError::OutOfBounds { index, len_chars }
}

fn inv(start: usize, end: usize) -> EditError {
    EditError::InvertedRange { start, end }
}

/// R1 (D-013 SG-1): inverted range first, then `start`, then `end`.
#[test]
#[allow(clippy::reversed_empty_ranges)]
fn edit_error_precedence_is_inverted_then_start_then_end() {
    let mut b = Buffer::from_text("abc");
    let cases = [
        (9..5, inv(9, 5)),
        (5..4, inv(5, 4)),
        (4..9, oob(4, 3)),
        (1..9, oob(9, 3)),
    ];
    for (range, want) in cases.clone() {
        assert_eq!(b.delete(range.clone()), Err(want), "delete({range:?})");
    }
    for (range, want) in cases {
        assert_eq!(
            b.replace(range.clone(), "x"),
            Err(want),
            "replace({range:?}, \"x\")"
        );
    }
    assert_eq!(b.text(), "abc");
    assert!(!b.can_undo());
}

/// R2 (D-013 SG-2): an `Err` edit or a `false` redo leaves the open typing group open.
#[test]
#[allow(clippy::reversed_empty_ranges)]
fn rejected_edit_or_failed_redo_keeps_typing_group_open() {
    type Interrupt = (&'static str, fn(&mut Buffer));
    let interrupts: [Interrupt; 5] = [
        ("insert(99, \"x\")", |b| assert!(b.insert(99, "x").is_err())),
        ("delete(5..9)", |b| assert!(b.delete(5..9).is_err())),
        ("delete(2..1)", |b| assert!(b.delete(2..1).is_err())),
        ("replace(7..7, \"y\")", |b| assert!(b.replace(7..7, "y").is_err())),
        ("redo()", |b| assert!(!b.redo())),
    ];
    let typed_with = |x: fn(&mut Buffer)| {
        let mut b = Buffer::new();
        b.insert(0, "a").expect("insert a");
        b.insert(1, "b").expect("insert b");
        x(&mut b);
        b.insert(2, "c").expect("insert c");
        assert_eq!(b.text(), "abc");
        assert!(b.undo(), "one undo step exists");
        b
    };
    for (name, x) in interrupts {
        let b = typed_with(x);
        assert_eq!(b.text(), "", "after {name}: one undo removes the whole group");
        assert!(!b.can_undo(), "after {name}: the group was one step");
    }
    // Control: a real group break makes "c" its own step, so the probe tells the two apart.
    let b = typed_with(|b| b.break_undo_group());
    assert_eq!(b.text(), "ab");
    assert!(b.can_undo());
}

/// R3 (D-013 SG-3): indices are validated before emptiness.
#[test]
fn empty_edit_out_of_range_is_out_of_bounds() {
    let mut b = Buffer::from_text("abc");
    assert_eq!(b.insert(9, ""), Err(oob(9, 3)));
    assert_eq!(b.delete(9..9), Err(oob(9, 3)));
    assert_eq!(b.replace(9..9, ""), Err(oob(9, 3)));
    assert_eq!(b.insert(3, ""), Ok(()));
    assert_eq!(b.delete(3..3), Ok(String::new()));
    assert_eq!(b.text(), "abc");
    assert!(!b.can_undo());
}
