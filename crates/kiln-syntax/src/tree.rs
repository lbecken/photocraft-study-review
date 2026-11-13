//! A tree-sitter syntax tree for one Rust source text.
//!
//! [`SyntaxTree::parse`] parses a text from scratch; [`SyntaxTree::update`] brings an existing
//! tree to a new text, reusing the old tree for the single changed region. Every input yields a
//! tree: malformed Rust shows up as [`ErrorSpan`]s, not as an `Err`.
//!
//! All offsets in this module's public API are char offsets (Unicode scalar values) into
//! [`SyntaxTree::text`]; CRLF counts as two chars.
//!
//! Every walk over the tree is iterative (a `TreeCursor`), so deep nesting cannot overflow the
//! stack.

use std::fmt;

use tree_sitter::{InputEdit, Node, Parser, Point, Tree};

/// The kind of an [`ErrorSpan`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// An `ERROR` node: text the parser could not fit into the grammar.
    Error,
    /// A `MISSING` node: a token the parser inserted (zero width) to recover.
    Missing,
}

/// One `ERROR` or `MISSING` node of a tree, in char offsets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorSpan {
    /// Whether the node is an `ERROR` or a `MISSING` node.
    pub kind: ErrorKind,
    /// Char offset of the node's start.
    pub start: usize,
    /// Char offset of the node's end (equal to `start` for a `MISSING` node).
    pub end: usize,
}

/// How [`SyntaxTree::update`] produced the new tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseMode {
    /// The new text equals the current text; nothing was parsed.
    Unchanged,
    /// The old tree was reused for the single changed region.
    Incremental,
    /// The text was parsed from scratch (only when the incremental tree had an error).
    Full,
}

/// tree-sitter produced no tree. `Display`: `tree-sitter returned no tree`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// tree-sitter rejected the grammar or returned no tree.
    NoTree,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::NoTree => f.write_str("tree-sitter returned no tree"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Bytes per block of the byte → char offset table.
const BLOCK: usize = 64;

/// A parsed Rust source text: the text and its tree-sitter tree.
///
/// Holds no `tree_sitter::Parser`; a parser is made per [`parse`](Self::parse) or
/// [`update`](Self::update).
#[derive(Clone)]
pub struct SyntaxTree {
    text: String,
    tree: Tree,
    len_chars: usize,
    /// Empty when `text` is ASCII (byte offset == char offset). Otherwise entry `i` is the
    /// number of chars in `text[..i * BLOCK]`.
    block_chars: Vec<usize>,
}

impl fmt::Debug for SyntaxTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SyntaxTree")
            .field("len_chars", &self.len_chars)
            .field("has_error", &self.has_error())
            .finish()
    }
}

/// A parser set to the pinned Rust grammar.
fn rust_parser() -> Result<Parser, ParseError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .map_err(|_| ParseError::NoTree)?;
    Ok(parser)
}

/// Whether byte `b` starts a char (is not a UTF-8 continuation byte).
fn is_char_start(b: u8) -> bool {
    (b & 0xC0) != 0x80
}

/// Count of chars in `bytes` (valid UTF-8, or a prefix of it).
fn count_chars(bytes: &[u8]) -> usize {
    bytes.iter().filter(|&&b| is_char_start(b)).count()
}

/// The tree-sitter point (row, byte column) of byte offset `byte` in `text`.
fn point_at(text: &str, byte: usize) -> Point {
    let before = &text.as_bytes()[..byte];
    let row = before.iter().filter(|&&b| b == b'\n').count();
    let line_start = before
        .iter()
        .rposition(|&b| b == b'\n')
        .map_or(0, |i| i + 1);
    Point {
        row,
        column: byte - line_start,
    }
}

/// Byte lengths `(prefix, suffix)` of the common char prefix and suffix of `old` and `new`,
/// not overlapping in either text.
fn common_affixes(old: &str, new: &str) -> (usize, usize) {
    let (a, b) = (old.as_bytes(), new.as_bytes());
    let mut prefix = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    while !(old.is_char_boundary(prefix) && new.is_char_boundary(prefix)) {
        prefix -= 1;
    }
    let max_suffix = a.len().min(b.len()) - prefix;
    let mut suffix = a
        .iter()
        .rev()
        .zip(b.iter().rev())
        .take(max_suffix)
        .take_while(|(x, y)| x == y)
        .count();
    while !(old.is_char_boundary(a.len() - suffix) && new.is_char_boundary(b.len() - suffix)) {
        suffix -= 1;
    }
    (prefix, suffix)
}

/// Quote an anonymous node kind for the S-expression form: `\` and `"` are escaped.
fn push_quoted(out: &mut String, kind: &str) {
    out.push('"');
    for c in kind.chars() {
        if c == '\\' || c == '"' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
}

impl SyntaxTree {
    /// Parse `text` from scratch.
    pub fn parse(text: &str) -> Result<SyntaxTree, ParseError> {
        let mut parser = rust_parser()?;
        let tree = parser.parse(text, None).ok_or(ParseError::NoTree)?;
        Ok(Self::from_parts(text.to_owned(), tree))
    }

    /// Bring the tree to `new_text`. `Ok(Unchanged)` iff `new_text` equals the current text.
    /// Otherwise reuse the old tree for the single changed region (common char prefix and
    /// suffix of old and new text) and report `Ok(Incremental)`. When the incrementally parsed
    /// tree has an error, the text is parsed again from scratch and `Ok(Full)` is reported, so
    /// the result always equals a parse from scratch. On `Err`, `self` is unchanged (old text,
    /// old tree).
    pub fn update(&mut self, new_text: &str) -> Result<ParseMode, ParseError> {
        if new_text == self.text {
            return Ok(ParseMode::Unchanged);
        }
        let old = self.text.as_str();
        let (prefix, suffix) = common_affixes(old, new_text);
        let old_end = old.len() - suffix;
        let new_end = new_text.len() - suffix;
        let edit = InputEdit {
            start_byte: prefix,
            old_end_byte: old_end,
            new_end_byte: new_end,
            start_position: point_at(old, prefix),
            old_end_position: point_at(old, old_end),
            new_end_position: point_at(new_text, new_end),
        };
        let mut edited = self.tree.clone();
        edited.edit(&edit);

        let mut parser = rust_parser()?;
        let incremental = parser
            .parse(new_text, Some(&edited))
            .ok_or(ParseError::NoTree)?;
        let candidate = Self::from_parts(new_text.to_owned(), incremental);
        if !candidate.has_error() {
            *self = candidate;
            return Ok(ParseMode::Incremental);
        }
        let mut parser = rust_parser()?;
        let full = parser.parse(new_text, None).ok_or(ParseError::NoTree)?;
        *self = Self::from_parts(candidate.text, full);
        Ok(ParseMode::Full)
    }

    /// The text this tree was parsed from.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Whether the tree holds any `ERROR` or `MISSING` node.
    pub fn has_error(&self) -> bool {
        let mut found = false;
        self.walk_errors(|_| {
            found = true;
            false
        });
        found
    }

    /// Every `ERROR` and `MISSING` node, in preorder, as char spans.
    pub fn errors(&self) -> Vec<ErrorSpan> {
        let mut spans = Vec::new();
        self.walk_errors(|node| {
            let kind = if node.is_missing() {
                ErrorKind::Missing
            } else {
                ErrorKind::Error
            };
            spans.push(ErrorSpan {
                kind,
                start: self.char_at_byte(node.start_byte()),
                end: self.char_at_byte(node.end_byte()),
            });
            true
        });
        spans
    }

    /// The tree in the pinned S-expression form: preorder over named nodes plus `MISSING`
    /// nodes, each as `(KIND@START..END` + ` child`… + `)`, char offsets, `FIELD:` before a
    /// child reached through a field, `(MISSING KIND@S..S)` with an anonymous KIND in double
    /// quotes (`\` and `"` escaped).
    pub fn sexp(&self) -> String {
        let mut out = String::new();
        let mut cursor = self.tree.walk();
        // For each ancestor of the cursor's node: whether it was printed (needs a `)`).
        let mut open: Vec<bool> = Vec::new();
        loop {
            let node = cursor.node();
            let printed = node.is_named() || node.is_missing();
            if printed {
                if !out.is_empty() {
                    out.push(' ');
                }
                if let Some(field) = cursor.field_name() {
                    out.push_str(field);
                    out.push(':');
                }
                out.push('(');
                if node.is_missing() {
                    out.push_str("MISSING ");
                    if node.is_named() {
                        out.push_str(node.kind());
                    } else {
                        push_quoted(&mut out, node.kind());
                    }
                } else if node.is_error() {
                    out.push_str("ERROR");
                } else {
                    out.push_str(node.kind());
                }
                out.push('@');
                out.push_str(&self.char_at_byte(node.start_byte()).to_string());
                out.push_str("..");
                out.push_str(&self.char_at_byte(node.end_byte()).to_string());
            }
            if cursor.goto_first_child() {
                open.push(printed);
                continue;
            }
            if printed {
                out.push(')');
            }
            loop {
                if cursor.goto_next_sibling() {
                    break;
                }
                if !cursor.goto_parent() {
                    return out;
                }
                if open.pop().unwrap_or(false) {
                    out.push(')');
                }
            }
        }
    }

    /// Number of nodes in the tree, named and anonymous, the root included.
    pub fn node_count(&self) -> usize {
        self.tree.root_node().descendant_count()
    }

    /// The root node, for the sibling modules (highlight, brackets, outline).
    #[allow(dead_code)] // used by highlight (T-020) and outline (T-023)
    pub(crate) fn root(&self) -> Node<'_> {
        self.tree.root_node()
    }

    /// The char offset of byte offset `byte` in `text()` (clamped to the text's end). A byte
    /// inside a multi-byte char maps to the offset just after that char's start.
    #[allow(dead_code)] // byte offset in `text()` → char offset
    pub(crate) fn char_at_byte(&self, byte: usize) -> usize {
        let byte = byte.min(self.text.len());
        if self.block_chars.is_empty() {
            return byte;
        }
        let block = byte / BLOCK;
        let base = self
            .block_chars
            .get(block)
            .copied()
            .unwrap_or(self.len_chars);
        base + count_chars(&self.text.as_bytes()[block * BLOCK..byte])
    }

    fn from_parts(text: String, tree: Tree) -> SyntaxTree {
        let (len_chars, block_chars) = if text.is_ascii() {
            (text.len(), Vec::new())
        } else {
            let mut table = Vec::with_capacity(text.len() / BLOCK + 1);
            let mut chars = 0;
            for chunk in text.as_bytes().chunks(BLOCK) {
                table.push(chars);
                chars += count_chars(chunk);
            }
            table.push(chars);
            (chars, table)
        };
        SyntaxTree {
            text,
            tree,
            len_chars,
            block_chars,
        }
    }

    /// Visit every `ERROR` and `MISSING` node in preorder until `visit` returns `false`.
    /// Subtrees without an error (tree-sitter's error cost is zero) are skipped.
    fn walk_errors(&self, mut visit: impl FnMut(Node<'_>) -> bool) {
        let mut cursor = self.tree.walk();
        loop {
            let node = cursor.node();
            let mut descend = node.has_error();
            if node.is_error() || node.is_missing() {
                if !visit(node) {
                    return;
                }
                descend = descend && node.is_error();
            }
            if descend && cursor.goto_first_child() {
                continue;
            }
            loop {
                if cursor.goto_next_sibling() {
                    break;
                }
                if !cursor.goto_parent() {
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every node, by an unpruned cursor walk: (is_error, is_missing).
    fn all_nodes(tree: &SyntaxTree) -> Vec<(bool, bool)> {
        let mut out = Vec::new();
        let mut cursor = tree.tree.walk();
        loop {
            let n = cursor.node();
            out.push((n.is_error(), n.is_missing()));
            if cursor.goto_first_child() {
                continue;
            }
            loop {
                if cursor.goto_next_sibling() {
                    break;
                }
                if !cursor.goto_parent() {
                    return out;
                }
            }
        }
    }

    #[test]
    fn set_language_succeeds_for_pinned_grammar() {
        let mut parser = Parser::new();
        assert!(
            parser
                .set_language(&tree_sitter_rust::LANGUAGE.into())
                .is_ok()
        );
        assert!(rust_parser().is_ok());
    }

    #[test]
    fn parse_error_display() {
        assert_eq!(
            ParseError::NoTree.to_string(),
            "tree-sitter returned no tree"
        );
        let e: &dyn std::error::Error = &ParseError::NoTree;
        assert_eq!(e.to_string(), "tree-sitter returned no tree");
    }

    #[test]
    fn pruned_error_walk_matches_full_walk() {
        for src in [
            "fn f() {}",
            "fn a() { (",
            "fn f() { let x = 1 }",
            "fn f() { 1 + }",
            "fn f() { let s = r#\"abc\"; }",
            "}}}{{{ fn ( let = ;",
            "struct S { a: [u8; }",
        ] {
            let t = SyntaxTree::parse(src).unwrap();
            let full: usize = all_nodes(&t).iter().filter(|(e, m)| *e || *m).count();
            assert_eq!(t.errors().len(), full, "{src}");
            assert_eq!(t.has_error(), full > 0, "{src}");
        }
    }

    #[test]
    fn node_count_equals_cursor_walk() {
        for src in ["", "fn f() {}", "fn a() { (", "fn f() { let x = 1 }"] {
            let t = SyntaxTree::parse(src).unwrap();
            assert_eq!(t.node_count(), all_nodes(&t).len(), "{src}");
        }
    }

    #[test]
    fn char_at_byte_counts_chars() {
        let text = format!("{}é😀x{}", "a".repeat(70), "é".repeat(100));
        let t = SyntaxTree::parse(&text).unwrap();
        for (ci, (bi, _)) in text.char_indices().enumerate() {
            assert_eq!(t.char_at_byte(bi), ci);
        }
        assert_eq!(t.char_at_byte(text.len()), text.chars().count());
        assert_eq!(t.char_at_byte(text.len() + 10), text.chars().count());
        let ascii = SyntaxTree::parse("fn f() {}").unwrap();
        assert_eq!(ascii.char_at_byte(4), 4);
        assert_eq!(ascii.root().kind(), "source_file");
    }

    #[test]
    fn common_affixes_respect_char_boundaries() {
        // "é" = C3 A9, "è" = C3 A8: common byte prefix of 1 must back up to 0.
        assert_eq!(common_affixes("é", "è"), (0, 0));
        assert_eq!(common_affixes("abc", "abc"), (3, 0));
        assert_eq!(common_affixes("aXc", "aYYc"), (1, 1));
        assert_eq!(common_affixes("aa", "aaa"), (2, 0));
        // "😀" and "😁" share three leading bytes.
        assert_eq!(common_affixes("x😀y", "x😁y"), (1, 1));
        // "é" = C3 A9, "©" = C2 A9: one common suffix byte that is not a char start must back
        // up to 0 (T-016 review N2).
        assert_eq!(common_affixes("é", "©"), (0, 0));
    }

    #[test]
    fn point_at_counts_rows_and_byte_columns() {
        assert_eq!(point_at("ab\ncé\nd", 0), Point { row: 0, column: 0 });
        assert_eq!(point_at("ab\ncé\nd", 6), Point { row: 1, column: 3 });
        assert_eq!(point_at("ab\ncé\nd", 7), Point { row: 2, column: 0 });
    }

    #[test]
    fn quoting_escapes_backslash_and_quote() {
        let mut s = String::new();
        push_quoted(&mut s, "a\"b\\c;");
        assert_eq!(s, r#""a\"b\\c;""#);
    }
}
