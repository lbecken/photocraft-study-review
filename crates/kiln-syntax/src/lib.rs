//! # kiln-syntax — layer L1
//!
//! Responsibility (CHARTER §3): tree-sitter integration for Rust, incremental reparse,
//! highlight spans, outline and bracket matching.
//!
//! May depend on: `kiln-text` (L0). Must stay headless: no UI or GPU crate may appear in its
//! dependency closure (`cargo xtask layers`).
//!
//! Status: [`tree`] parses Rust with tree-sitter (`tree-sitter` 0.27.1, `tree-sitter-rust`
//! 0.24.2, pinned in this crate's manifest) into a [`SyntaxTree`]: error and missing nodes as
//! char spans, a pinned S-expression form and incremental update that always equals a parse
//! from scratch. [`highlight`] gives highlight spans in seven kinds that partition a char
//! range of the text, and [`brackets`] matches a bracket with its partner, ignoring brackets
//! inside strings, char literals and comments. [`outline`] lists the `fn`, `struct`, `enum`,
//! `trait`, `impl` and `mod` items of a tree with their names, char ranges and nesting depth.
//! Building this crate needs a C compiler (the grammar and the tree-sitter runtime are C).

pub mod tree;

pub mod brackets;
pub mod highlight;
pub mod outline;

pub use tree::{ErrorKind, ErrorSpan, ParseError, ParseMode, SyntaxTree};
