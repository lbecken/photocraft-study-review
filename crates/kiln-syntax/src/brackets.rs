//! Bracket matching over a [`crate::SyntaxTree`] (T-020).
//!
//! [`match_bracket`] answers for the char at an offset: if it is one of `( ) [ ] { }` and an
//! anonymous leaf of the tree (so not inside a string, char literal or comment), its partner
//! is looked for among the anonymous leaves of the same bracket pair that are direct children
//! of the same parent node. Those leaves are taken left to right; each closing leaf pairs with
//! the nearest unpaired opening leaf before it (a stack), and a leaf left unpaired has no
//! partner. `MISSING` leaves (inserted by the parser, zero width) hold no char and take no
//! part. `<` and `>` are not brackets.
//!
//! The descent to the leaf and the scan of the parent's children are iterative (a
//! `TreeCursor`), so deep nesting cannot overflow the stack.

use crate::SyntaxTree;

/// A matched pair: the char offsets of the opening and the closing bracket.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BracketPair {
    /// Char offset of the opening bracket.
    pub open: usize,
    /// Char offset of the closing bracket.
    pub close: usize,
}

/// The opening and closing char of the bracket pair `c` belongs to, if `c` is a bracket.
fn pair_of(c: char) -> Option<(&'static str, &'static str)> {
    match c {
        '(' | ')' => Some(("(", ")")),
        '[' | ']' => Some(("[", "]")),
        '{' | '}' => Some(("{", "}")),
        _ => None,
    }
}

/// The pair the bracket at char `offset` of `tree`'s text belongs to, or `None`: not a
/// bracket, not an anonymous leaf (inside a string, char literal or comment), no partner, or
/// `offset` at or past the end of the text.
pub fn match_bracket(tree: &SyntaxTree, offset: usize) -> Option<BracketPair> {
    let text = tree.text();
    let (byte, c) = text.char_indices().nth(offset)?;
    let (open_kind, close_kind) = pair_of(c)?;

    // Descend to the smallest node holding the byte.
    let mut cursor = tree.root().walk();
    while cursor.node().child_count() > 0 {
        if cursor.goto_first_child_for_byte(byte).is_none() || cursor.node().start_byte() > byte {
            // The byte lies in no child (hidden content of the node, e.g. a comment's text).
            return None;
        }
    }
    let leaf = cursor.node();
    if leaf.is_named()
        || leaf.is_missing()
        || leaf.start_byte() != byte
        || leaf.end_byte() != byte + c.len_utf8()
        || leaf.kind().chars().ne(std::iter::once(c))
    {
        return None;
    }
    if !cursor.goto_parent() {
        return None;
    }

    // Pair the parent's direct bracket leaves of this kind left to right with a stack.
    let mut stack: Vec<usize> = Vec::new();
    let mut found: Option<(usize, usize)> = None;
    if cursor.goto_first_child() {
        loop {
            let n = cursor.node();
            if !n.is_named() && !n.is_missing() && n.child_count() == 0 {
                let kind = n.kind();
                if kind == open_kind {
                    stack.push(n.start_byte());
                } else if kind == close_kind
                    && let Some(open) = stack.pop()
                {
                    let close = n.start_byte();
                    if open == byte || close == byte {
                        found = Some((open, close));
                        break;
                    }
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    let (open, close) = found?;
    Some(BracketPair {
        open: tree.char_at_byte(open),
        close: tree.char_at_byte(close),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(src: &str, offset: usize) -> Option<(usize, usize)> {
        let tree = SyntaxTree::parse(src).expect("parse");
        match_bracket(&tree, offset).map(|p| (p.open, p.close))
    }

    #[test]
    fn worked_examples() {
        let src = "fn f() { let s: u8 = 1; } // c";
        assert_eq!(at(src, 4), Some((4, 5)));
        assert_eq!(at(src, 5), Some((4, 5)));
        assert_eq!(at(src, 7), Some((7, 24)));
        assert_eq!(at(src, 24), Some((7, 24)));
        assert_eq!(at(src, 0), None);
        assert_eq!(at(src, 30), None);
        assert_eq!(at(src, 99), None);
        assert_eq!(at("fn a() { (", 9), None);
    }

    #[test]
    fn stack_pairing_under_error_parents() {
        let src = ") ( ( ) ] [";
        for o in [0, 2, 8, 10] {
            assert_eq!(at(src, o), None, "{o}");
        }
        assert_eq!(at(src, 4), Some((4, 6)));
        assert_eq!(at(src, 6), Some((4, 6)));
        let src = "fn a() { ( ) (";
        assert_eq!(at(src, 9), Some((9, 11)));
        assert_eq!(at(src, 11), Some((9, 11)));
        assert_eq!(at(src, 7), None);
        assert_eq!(at(src, 13), None);
    }

    #[test]
    fn brackets_in_strings_chars_and_comments_are_ignored() {
        let src = "fn g() { let _ = (\"(\", '[', r#\"{\"#); } // ( ) /* [ ] */\n/* { } */";
        for (i, c) in src.chars().enumerate() {
            if "()[]{}".contains(c) {
                // Real brackets: 4, 5, 7, 17, 34, 37; in literals: 19, 24, 31; then comments.
                let want_some = !(i > 17 && i < 33) && i <= 37;
                assert_eq!(at(src, i).is_some(), want_some, "{i} {c}");
            }
        }
        // Offsets are chars: a multi-byte char before the brackets.
        assert_eq!(at("fn é() {}", 4), Some((4, 5)));
        assert_eq!(at("fn é() {}", 7), Some((7, 8)));
        // `<`/`>` are not brackets.
        assert_eq!(at("fn f<T>() {}", 4), None);
    }
}
