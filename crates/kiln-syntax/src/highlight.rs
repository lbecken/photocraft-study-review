//! Highlight spans over a [`crate::SyntaxTree`] (T-020).
//!
//! [`highlight`] gives, for a char range of the tree's text, spans that are sorted, contiguous
//! and non-overlapping, cover the range exactly (clipped at its ends) and never put two
//! adjacent spans of the same kind side by side. Each char gets one [`HighlightKind`]: the
//! first of these rules that applies.
//!
//! 1. `comment`: inside a `line_comment` or `block_comment` node (doc comments included).
//! 2. `string`: inside a `string_literal`, `raw_string_literal` or `char_literal` node,
//!    quotes and prefixes included.
//! 3. `number`: inside an `integer_literal` or `float_literal`.
//! 4. `keyword`: an anonymous leaf whose kind is in [`KEYWORDS`], or a named leaf of kind
//!    `self`, `crate`, `super` or `mutable_specifier`.
//! 5. `type`: a `type_identifier` or `primitive_type` leaf.
//! 6. `function`: the `name` of a `function_item` or `function_signature_item`; the callee of
//!    a `call_expression` (its `function` field if that is an `identifier`, the `name` field of
//!    a `scoped_identifier`, the `field` field of a `field_expression`; a `generic_function`'s
//!    `function` field resolved the same way).
//! 7. `plain`: everything else.
//!
//! The walk over the tree is iterative (a `TreeCursor`), so deep nesting cannot overflow the
//! stack, and it skips subtrees that lie outside the requested range.

use std::ops::Range;

use tree_sitter::Node;

use crate::SyntaxTree;

/// The kind of a highlight span. The order of the variants is the rule order: a char inside
/// nodes of several kinds gets the first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HighlightKind {
    /// Rule 1: a line or block comment, doc comments included.
    Comment,
    /// Rule 2: a string, raw string or char literal, quotes and prefixes included.
    String,
    /// Rule 3: an integer or float literal.
    Number,
    /// Rule 4: a keyword.
    Keyword,
    /// Rule 5: a type name or primitive type.
    Type,
    /// Rule 6: a function name at its definition or call.
    Function,
    /// Rule 7: everything else.
    Plain,
}

impl HighlightKind {
    /// Every kind, in rule order.
    pub const ALL: [HighlightKind; 7] = [
        HighlightKind::Comment,
        HighlightKind::String,
        HighlightKind::Number,
        HighlightKind::Keyword,
        HighlightKind::Type,
        HighlightKind::Function,
        HighlightKind::Plain,
    ];

    /// The lower-case name (`comment`, `string`, `number`, `keyword`, `type`, `function`,
    /// `plain`).
    pub fn as_str(self) -> &'static str {
        match self {
            HighlightKind::Comment => "comment",
            HighlightKind::String => "string",
            HighlightKind::Number => "number",
            HighlightKind::Keyword => "keyword",
            HighlightKind::Type => "type",
            HighlightKind::Function => "function",
            HighlightKind::Plain => "plain",
        }
    }
}

/// One highlight span: the chars `start..end` of the tree's text have kind `kind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HighlightSpan {
    /// Char offset of the first char.
    pub start: usize,
    /// Char offset after the last char (`start < end`).
    pub end: usize,
    /// The kind of every char in the span.
    pub kind: HighlightKind,
}

/// Anonymous leaf kinds that are keywords (rule 4).
pub const KEYWORDS: [&str; 58] = [
    "as",
    "async",
    "await",
    "break",
    "const",
    "continue",
    "crate",
    "dyn",
    "else",
    "enum",
    "extern",
    "false",
    "fn",
    "for",
    "gen",
    "if",
    "impl",
    "in",
    "let",
    "loop",
    "match",
    "mod",
    "move",
    "mut",
    "pub",
    "ref",
    "return",
    "self",
    "Self",
    "static",
    "struct",
    "super",
    "trait",
    "true",
    "type",
    "unsafe",
    "use",
    "where",
    "while",
    "yield",
    "abstract",
    "become",
    "box",
    "do",
    "final",
    "macro",
    "override",
    "priv",
    "typeof",
    "unsized",
    "virtual",
    "try",
    "union",
    "default",
    "safe",
    "raw",
    "auto",
    "macro_rules!",
];

/// Named leaf kinds that are keywords (rule 4).
const NAMED_KEYWORDS: [&str; 4] = ["self", "crate", "super", "mutable_specifier"];

/// The kind a node gives to all of its chars by itself (rules 1–5), if any.
fn own_kind(node: Node<'_>) -> Option<HighlightKind> {
    let kind = node.kind();
    match kind {
        "line_comment" | "block_comment" => return Some(HighlightKind::Comment),
        "string_literal" | "raw_string_literal" | "char_literal" => {
            return Some(HighlightKind::String);
        }
        "integer_literal" | "float_literal" => return Some(HighlightKind::Number),
        _ => {}
    }
    if node.child_count() != 0 {
        return None;
    }
    if node.is_named() {
        if NAMED_KEYWORDS.contains(&kind) {
            Some(HighlightKind::Keyword)
        } else if kind == "type_identifier" || kind == "primitive_type" {
            Some(HighlightKind::Type)
        } else {
            None
        }
    } else if KEYWORDS.contains(&kind) {
        Some(HighlightKind::Keyword)
    } else {
        None
    }
}

/// The callee node of a `call_expression`'s `function` field (rule 6), if it has one.
fn callee(function: Node<'_>) -> Option<Node<'_>> {
    let mut node = function;
    // A `generic_function`'s `function` field is resolved the same way; its own `function`
    // field is never another `generic_function` in the grammar, but a loop costs nothing.
    loop {
        match node.kind() {
            "identifier" => return Some(node),
            "scoped_identifier" => return node.child_by_field_name("name"),
            "field_expression" => return node.child_by_field_name("field"),
            "generic_function" => node = node.child_by_field_name("function")?,
            _ => return None,
        }
    }
}

/// The node a node marks as `function` (rule 6), if any.
fn function_target(node: Node<'_>) -> Option<Node<'_>> {
    match node.kind() {
        "function_item" | "function_signature_item" => node.child_by_field_name("name"),
        "call_expression" => callee(node.child_by_field_name("function")?),
        _ => None,
    }
}

/// Byte offset of char `char_idx` in `text` (`text.len()` at or past the end).
fn byte_of_char(text: &str, char_idx: usize) -> usize {
    text.char_indices()
        .nth(char_idx)
        .map_or(text.len(), |(b, _)| b)
}

/// Highlight spans covering the chars `range` of `tree`'s text, clipped at the range ends.
/// `range.end` is clamped to the text's length in chars; an empty range gives no spans.
pub fn highlight(tree: &SyntaxTree, range: Range<usize>) -> Vec<HighlightSpan> {
    let text = tree.text();
    if range.start >= range.end {
        return Vec::new();
    }
    let start_byte = byte_of_char(text, range.start);
    let end_byte = start_byte + byte_of_char(&text[start_byte..], range.end - range.start);
    if start_byte >= end_byte {
        return Vec::new();
    }
    // The kind of each byte of the range (the first rule wins: the smallest kind).
    let mut kinds = vec![HighlightKind::Plain; end_byte - start_byte];
    let mut paint = |node: Node<'_>, kind: HighlightKind| {
        let s = node.start_byte().max(start_byte);
        let e = node.end_byte().min(end_byte);
        for k in kinds.iter_mut().take(e.saturating_sub(start_byte)).skip(s - start_byte) {
            if kind < *k {
                *k = kind;
            }
        }
    };

    let mut cursor = tree.root().walk();
    loop {
        let node = cursor.node();
        let overlaps = node.start_byte() < end_byte && node.end_byte() > start_byte;
        let mut descend = overlaps;
        if overlaps {
            if let Some(kind) = own_kind(node) {
                paint(node, kind);
                // Nothing inside a comment, string or number has an earlier rule.
                descend = false;
            }
            if let Some(target) = function_target(node) {
                paint(target, HighlightKind::Function);
            }
        }
        if descend && cursor.goto_first_child() {
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                return spans_of(text, start_byte, range.start, &kinds);
            }
        }
    }
}

/// Merge per-byte kinds into char spans; the range starts at byte `start_byte` (char
/// `start_char`). Only the first byte of each char is read: node bounds are char bounds.
fn spans_of(
    text: &str,
    start_byte: usize,
    start_char: usize,
    kinds: &[HighlightKind],
) -> Vec<HighlightSpan> {
    let mut spans: Vec<HighlightSpan> = Vec::new();
    let slice = &text[start_byte..start_byte + kinds.len()];
    for (i, (b, _)) in slice.char_indices().enumerate() {
        let kind = kinds[b];
        let at = start_char + i;
        match spans.last_mut() {
            Some(last) if last.kind == kind => last.end = at + 1,
            _ => spans.push(HighlightSpan {
                start: at,
                end: at + 1,
                kind,
            }),
        }
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(src: &str, range: Range<usize>) -> Vec<(usize, usize, &'static str)> {
        let tree = SyntaxTree::parse(src).expect("parse");
        highlight(&tree, range)
            .into_iter()
            .map(|s| (s.start, s.end, s.kind.as_str()))
            .collect()
    }

    #[test]
    fn worked_example_is_exact() {
        let src = "fn f() { let s: u8 = 1; } // c";
        assert_eq!(
            spans(src, 0..30),
            vec![
                (0, 2, "keyword"),
                (2, 3, "plain"),
                (3, 4, "function"),
                (4, 9, "plain"),
                (9, 12, "keyword"),
                (12, 16, "plain"),
                (16, 18, "type"),
                (18, 21, "plain"),
                (21, 22, "number"),
                (22, 26, "plain"),
                (26, 30, "comment"),
            ]
        );
    }

    #[test]
    fn ranges_are_clipped_and_empty_ranges_give_nothing() {
        let src = "fn f() { let s: u8 = 1; } // c";
        assert_eq!(spans(src, 1..4), vec![(1, 2, "keyword"), (2, 3, "plain"), (3, 4, "function")]);
        assert_eq!(spans(src, 27..29), vec![(27, 29, "comment")]);
        assert!(spans(src, 5..5).is_empty());
        assert!(spans("", 0..0).is_empty());
        // The end is clamped to the text.
        assert_eq!(spans("1", 0..9), vec![(0, 1, "number")]);
    }

    #[test]
    fn literals_and_prefixes_are_strings() {
        let src = "fn g() { let _ = (b'x', c\"s\", br#\"q\"#, '(', \"é\"); }";
        let got = spans(src, 0..src.chars().count());
        let text_of = |s: usize, e: usize| src.chars().skip(s).take(e - s).collect::<String>();
        let strings: Vec<String> = got
            .iter()
            .filter(|s| s.2 == "string")
            .map(|s| text_of(s.0, s.1))
            .collect();
        assert_eq!(strings, ["b'x'", "c\"s\"", "br#\"q\"#", "'('", "\"é\""]);
    }

    #[test]
    fn callees_are_functions_and_self_path_is_plain() {
        let src = "fn g() { a(); m::b(); x.c(); d::<u8>(); Self::new(); }";
        let got = spans(src, 0..src.chars().count());
        let text_of = |s: usize, e: usize| src.chars().skip(s).take(e - s).collect::<String>();
        let fns: Vec<String> = got
            .iter()
            .filter(|s| s.2 == "function")
            .map(|s| text_of(s.0, s.1))
            .collect();
        assert_eq!(fns, ["g", "a", "b", "c", "d", "new"]);
        let self_at = src.find("Self").expect("Self");
        assert!(got.iter().any(|s| s.0 <= self_at && self_at < s.1 && s.2 == "plain"));
    }

    #[test]
    fn keyword_leaves_and_named_keywords() {
        let src = "pub fn h(&mut self) -> crate::T { super::x() }";
        let got = spans(src, 0..src.chars().count());
        let text_of = |s: usize, e: usize| src.chars().skip(s).take(e - s).collect::<String>();
        let kws: Vec<String> = got
            .iter()
            .filter(|s| s.2 == "keyword")
            .map(|s| text_of(s.0, s.1))
            .collect();
        assert_eq!(kws, ["pub", "fn", "mut", "self", "crate", "super"]);
        assert!(KEYWORDS.contains(&"macro_rules!"));
        assert_eq!(HighlightKind::ALL.len(), 7);
    }
}
