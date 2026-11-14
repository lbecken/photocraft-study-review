//! Outline (symbols) of a [`crate::SyntaxTree`]: its `fn`, `struct`, `enum`, `trait`, `impl`
//! and `mod` items with their names, char ranges and nesting depth.
//!
//! Rules (T-023 Interface):
//! - item nodes: `function_item` and `function_signature_item` (`fn`), `struct_item`,
//!   `enum_item`, `trait_item`, `impl_item`, `mod_item`;
//! - the walk starts at the root, whatever its kind; an item node is listed iff every node
//!   strictly between it and the root is `source_file`, `declaration_list`, `mod_item`,
//!   `impl_item`, `trait_item` or `ERROR`, and it has its `name` field (for `impl`: its `type`
//!   field). So nothing inside a function body, a block, an `extern` block, a `const` or
//!   `static` value or a macro invocation is listed;
//! - `depth` is the number of listed items that enclose the item;
//! - `name` is the text of the `name` field; for `impl`, the text of the `type` field, or
//!   `<trait> for <type>` when there is a `trait` field, with every run of whitespace collapsed
//!   to one space. `name_start..name_end` is the `name` (for `impl`: `type`) field;
//!   `start..end` is the whole item node (attributes and doc comments are siblings, not
//!   inside it). All offsets are chars into [`SyntaxTree::text`].
//!
//! The walk is iterative (an explicit stack), so deep nesting cannot overflow the stack.

use tree_sitter::{Node, TreeCursor};

use crate::SyntaxTree;

/// The kind of an outline item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind {
    /// `fn`, with or without a body.
    Fn,
    /// `struct`.
    Struct,
    /// `enum`.
    Enum,
    /// `trait`.
    Trait,
    /// `impl` (inherent or of a trait).
    Impl,
    /// `mod`, inline or `mod x;`.
    Mod,
}

impl ItemKind {
    /// `"fn"`, `"struct"`, `"enum"`, `"trait"`, `"impl"` or `"mod"`.
    pub fn as_str(self) -> &'static str {
        match self {
            ItemKind::Fn => "fn",
            ItemKind::Struct => "struct",
            ItemKind::Enum => "enum",
            ItemKind::Trait => "trait",
            ItemKind::Impl => "impl",
            ItemKind::Mod => "mod",
        }
    }

    /// The kind of an item node, if `kind` is one.
    fn of_node(kind: &str) -> Option<ItemKind> {
        Some(match kind {
            "function_item" | "function_signature_item" => ItemKind::Fn,
            "struct_item" => ItemKind::Struct,
            "enum_item" => ItemKind::Enum,
            "trait_item" => ItemKind::Trait,
            "impl_item" => ItemKind::Impl,
            "mod_item" => ItemKind::Mod,
            _ => return None,
        })
    }
}

/// One item of an outline. Offsets are chars into the parsed text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineItem {
    /// The item's kind.
    pub kind: ItemKind,
    /// The item's name (for `impl`: its type, or `<trait> for <type>`, whitespace collapsed).
    pub name: String,
    /// Start of the item node.
    pub start: usize,
    /// End of the item node.
    pub end: usize,
    /// Start of the `name` field (for `impl`: the `type` field).
    pub name_start: usize,
    /// End of the `name` field (for `impl`: the `type` field).
    pub name_end: usize,
    /// Number of listed items that enclose this one.
    pub depth: usize,
}

/// Whether the walk descends into `node` (a node that may lie between an item and the root).
fn is_container(node: Node<'_>) -> bool {
    node.is_error()
        || matches!(
            node.kind(),
            "source_file" | "declaration_list" | "mod_item" | "impl_item" | "trait_item"
        )
}

/// `s` with every run of whitespace (`char::is_whitespace`) replaced by one space.
fn collapse_whitespace(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !in_space {
                out.push(' ');
            }
            in_space = true;
        } else {
            out.push(c);
            in_space = false;
        }
    }
    out
}

/// The outline item of `node` at `depth`, if `node` is an item node with its name field.
fn item_of(tree: &SyntaxTree, node: Node<'_>, depth: usize) -> Option<OutlineItem> {
    let kind = ItemKind::of_node(node.kind())?;
    let text = tree.text();
    let source = |n: Node<'_>| text.get(n.start_byte()..n.end_byte()).unwrap_or("");
    let (name_node, name) = if kind == ItemKind::Impl {
        let ty = node.child_by_field_name("type")?;
        let name = match node.child_by_field_name("trait") {
            Some(tr) => format!("{} for {}", source(tr), source(ty)),
            None => source(ty).to_owned(),
        };
        (ty, collapse_whitespace(&name))
    } else {
        let n = node.child_by_field_name("name")?;
        (n, source(n).to_owned())
    };
    Some(OutlineItem {
        kind,
        name,
        start: tree.char_at_byte(node.start_byte()),
        end: tree.char_at_byte(node.end_byte()),
        name_start: tree.char_at_byte(name_node.start_byte()),
        name_end: tree.char_at_byte(name_node.end_byte()),
        depth,
    })
}

/// Push the children of `node` onto `stack` so that the first child is on top.
fn push_children<'t>(
    node: Node<'t>,
    depth: usize,
    stack: &mut Vec<(Node<'t>, usize)>,
    cursor: &mut TreeCursor<'t>,
) {
    let from = stack.len();
    stack.extend(node.children(cursor).map(|child| (child, depth)));
    stack[from..].reverse();
}

/// The items of `tree` in preorder (the order of their `start`).
pub fn outline(tree: &SyntaxTree) -> Vec<OutlineItem> {
    let root = tree.root();
    let mut items = Vec::new();
    let mut cursor = root.walk();
    // Nodes still to visit, each with the number of listed items enclosing it; the top of the
    // stack is the next node in preorder.
    let mut stack: Vec<(Node<'_>, usize)> = Vec::new();
    push_children(root, 0, &mut stack, &mut cursor);
    while let Some((node, depth)) = stack.pop() {
        let mut inner = depth;
        if let Some(item) = item_of(tree, node, depth) {
            items.push(item);
            inner += 1;
        }
        if is_container(node) {
            push_children(node, inner, &mut stack, &mut cursor);
        }
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(text: &str) -> Vec<(ItemKind, String, usize, usize, usize, usize, usize)> {
        let tree = SyntaxTree::parse(text).unwrap();
        outline(&tree)
            .into_iter()
            .map(|i| {
                (
                    i.kind,
                    i.name,
                    i.start,
                    i.end,
                    i.name_start,
                    i.name_end,
                    i.depth,
                )
            })
            .collect()
    }

    #[test]
    fn as_str_names() {
        let all = [
            (ItemKind::Fn, "fn"),
            (ItemKind::Struct, "struct"),
            (ItemKind::Enum, "enum"),
            (ItemKind::Trait, "trait"),
            (ItemKind::Impl, "impl"),
            (ItemKind::Mod, "mod"),
        ];
        for (k, s) in all {
            assert_eq!(k.as_str(), s);
        }
    }

    #[test]
    fn collapses_whitespace_runs() {
        assert_eq!(collapse_whitespace("a  b\n\t c"), "a b c");
        assert_eq!(collapse_whitespace("Foo < T ,\n U >"), "Foo < T , U >");
        assert_eq!(collapse_whitespace("x"), "x");
        assert_eq!(collapse_whitespace("a\u{3000}b"), "a b");
    }

    #[test]
    fn enum_and_struct_and_signature() {
        use ItemKind::*;
        assert_eq!(
            items("enum E { A }\nstruct S { a: u8 }\ntrait T { fn s(); fn d() {} }"),
            vec![
                (Enum, "E".into(), 0, 12, 5, 6, 0),
                (Struct, "S".into(), 13, 31, 20, 21, 0),
                (Trait, "T".into(), 32, 61, 38, 39, 0),
                (Fn, "s".into(), 42, 49, 45, 46, 1),
                (Fn, "d".into(), 50, 59, 53, 54, 1),
            ]
        );
    }

    #[test]
    fn offsets_are_chars() {
        use ItemKind::*;
        // "é" is two bytes; the offsets count it as one char.
        assert_eq!(
            items("// é\nfn a() {}"),
            vec![(Fn, "a".into(), 5, 14, 8, 9, 0)]
        );
    }

    #[test]
    fn nested_mods_and_impls_count_depth() {
        use ItemKind::*;
        assert_eq!(
            items("mod a { mod b { impl X { fn f() {} } } }"),
            vec![
                (Mod, "a".into(), 0, 40, 4, 5, 0),
                (Mod, "b".into(), 8, 38, 12, 13, 1),
                (Impl, "X".into(), 16, 36, 21, 22, 2),
                (Fn, "f".into(), 25, 34, 28, 29, 3),
            ]
        );
    }

    #[test]
    fn empty_and_itemless_texts() {
        assert!(items("").is_empty());
        assert!(items("// only a comment\nuse a::b;\nconst C: u8 = 1;").is_empty());
    }
}
