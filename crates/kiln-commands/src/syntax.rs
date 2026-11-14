//! The `syntax.*` command group (T-020): a document's syntax tree, highlight spans and bracket
//! matching, and its session state [`SyntaxState`].
//!
//! All three commands are read-only (`mutating: false`). They keep, per document id, the tree
//! of the last successful `syntax.*` call on that document (whichever of the three it was), so
//! that the next call can reuse it ([`SyntaxTree::update`]). The cache is not session state:
//! no result depends on it except `syntax.tree`'s `mode`, which says how the tree was
//! obtained. A refused or failed call leaves the cache as it was.
//!
//! A document longer than [`MAX_CHARS`] chars is refused by every command with `too_large`
//! before anything is parsed, updated or cached (D-056). The limit bounds the length of the
//! text a command accepts, not the time a parse takes or the memory it needs
//! (`docs/protocol/syntax.md`).
//! Protocol description: `docs/protocol/syntax.md`.

use std::collections::BTreeMap;

use kiln_syntax::brackets::match_bracket;
use kiln_syntax::highlight::{HighlightKind, highlight};
use kiln_syntax::{ErrorKind, ParseError, ParseMode, SyntaxTree};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::reason::{InternalReason, RejectReason};
use crate::registry::{
    Command, CommandError, Context, Docs, Example, Refusal, RefusalExample, Registry,
    RegistryError, SeedFile, no_such_document_example,
};

/// The most chars a document may have for a `syntax.*` command (2^20).
pub(crate) const MAX_CHARS: usize = 1_048_576;

/// Register the `syntax.*` commands.
pub(crate) fn register(registry: &mut Registry) -> Result<(), RegistryError> {
    registry.register::<Tree>()?;
    registry.register::<Highlight>()?;
    registry.register::<MatchBracket>()?;
    Ok(())
}

/// Session state of the `syntax.*` group: per document id, the tree (and so the text) of the
/// last successful `syntax.*` call on that document.
#[derive(Debug, Default)]
pub(crate) struct SyntaxState {
    trees: BTreeMap<u64, SyntaxTree>,
}

/// How the tree of a call was obtained (`syntax.tree`'s `mode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Mode {
    /// No tree was cached for the document: parsed from scratch.
    Full,
    /// The cached tree was reused for the changed region.
    Incremental,
    /// The text equals the cached text: nothing was parsed.
    Unchanged,
}

/// `internal`/`other` for a parse that returned no tree (D-044).
fn no_tree(e: ParseError) -> CommandError {
    CommandError::Internal {
        reason: InternalReason::Other,
        message: e.to_string(),
    }
}

impl SyntaxState {
    /// The tree of `text` for document `id`, from the cache when there is one, and how it was
    /// obtained. On success the cache holds this tree for `id`, and entries of documents that
    /// are no longer open are dropped. On error the cache is unchanged.
    fn tree(
        &mut self,
        id: u64,
        text: &str,
        open: &Docs,
    ) -> Result<(&SyntaxTree, Mode), CommandError> {
        let mode = match self.trees.get_mut(&id) {
            Some(tree) => match tree.update(text).map_err(no_tree)? {
                ParseMode::Unchanged => Mode::Unchanged,
                ParseMode::Incremental => Mode::Incremental,
                ParseMode::Full => Mode::Full,
            },
            None => {
                let tree = SyntaxTree::parse(text).map_err(no_tree)?;
                self.trees.insert(id, tree);
                Mode::Full
            }
        };
        self.trees
            .retain(|k, _| *k == id || open.map().contains_key(k));
        let tree = self.trees.get(&id).ok_or_else(|| CommandError::Internal {
            reason: InternalReason::Other,
            message: format!("no cached tree for document {id}"),
        })?;
        Ok((tree, mode))
    }

    /// The ids that have a cached tree (for tests).
    #[cfg(test)]
    fn cached(&self) -> Vec<(u64, String)> {
        self.trees
            .iter()
            .map(|(id, t)| (*id, t.text().to_owned()))
            .collect()
    }
}

/// A file of 32 chars that the setup of [`too_large_example`] grows.
const GROWN: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

/// The refusal example of `too_large` for a command whose `params` name document 1 (also used
/// by `nav.*`, ruling Q-7): the setup opens a 32-char file, grows it to 32 × 32 × 32 × 32 =
/// 1,048,576 chars with three `search.replace` calls and inserts one more char, so the
/// document has [`MAX_CHARS`] + 1 chars without a megabyte literal.
pub(crate) fn too_large_example(params: Value) -> RefusalExample {
    let grow = (
        "search.replace",
        json!({"doc": 1, "query": "a", "replacement": GROWN, "all": true, "limit": 1}),
    );
    RefusalExample {
        description: "a document of 1,048,577 chars",
        initial_text: "",
        files: vec![SeedFile::text("g.rs", GROWN)],
        setup: vec![
            ("doc.open", json!({"path": "g.rs"})),
            grow.clone(),
            grow.clone(),
            grow,
            ("doc.insert", json!({"doc": 1, "offset": 0, "text": "b"})),
        ],
        params,
        kind: RejectReason::TooLarge,
    }
}

/// Refuse a document longer than [`MAX_CHARS`] with `too_large`.
fn check_size(doc: u64, len_chars: usize) -> Result<(), CommandError> {
    if len_chars > MAX_CHARS {
        return Err(CommandError::rejected(
            RejectReason::TooLarge,
            format!(
                "document {doc} has {len_chars} chars; syntax commands accept at most {MAX_CHARS}"
            ),
        ));
    }
    Ok(())
}

/// Refuse `offset > len_chars` with `out_of_range`.
fn check_offset(what: &str, offset: usize, len_chars: usize) -> Result<(), CommandError> {
    if offset > len_chars {
        return Err(CommandError::rejected(
            RejectReason::OutOfRange,
            format!("{what} {offset} is beyond the end of the text ({len_chars} chars)"),
        ));
    }
    Ok(())
}

/// Deserialize an optional parameter that is present: `null` is not a value for it (an absent
/// parameter is `None` through `#[serde(default)]`). Ruling x1 (D-084).
fn present<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(d).map(Some)
}

/// Drop the `"default": null` that `#[serde(default)]` adds to the schema: an absent `start`
/// or `end` has its documented default, and `null` is not a value of the bare type.
fn no_default(schema: &mut schemars::Schema) {
    schema.remove("default");
}

/// The text and length of open document `id`, after the checks every `syntax.*` command makes
/// first: `no_such_document`, then `too_large`.
fn checked_doc(docs: &Docs, id: u64) -> Result<(String, usize), CommandError> {
    let buffer = docs.get(id)?.buffer();
    let len_chars = buffer.len_chars();
    check_size(id, len_chars)?;
    Ok((buffer.text(), len_chars))
}

// ---------------------------------------------------------------------------------------------
// syntax.tree

/// Params naming one open document.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct DocParams {
    /// Id of an open document.
    doc: u64,
}

/// The kind of an error span.
#[derive(Clone, Copy, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ErrorSpanKind {
    /// An `ERROR` node: text the parser could not fit into the grammar.
    Error,
    /// A `MISSING` node: a token the parser inserted (zero width) to recover.
    Missing,
}

/// One `ERROR` or `MISSING` node, in char offsets.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct ErrorSpanOut {
    /// `error` or `missing`.
    kind: ErrorSpanKind,
    /// Char offset of the node's start.
    start: usize,
    /// Char offset of the node's end (equal to `start` for `missing`).
    end: usize,
}

/// Result of `syntax.tree`.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct TreeResult {
    /// True iff the tree holds an `ERROR` or `MISSING` node.
    has_error: bool,
    /// Every `ERROR` and `MISSING` node, in preorder.
    errors: Vec<ErrorSpanOut>,
    /// The tree in the pinned S-expression form (char offsets).
    sexp: String,
    /// Number of nodes, named and anonymous, the root included.
    node_count: usize,
    /// How the tree was obtained: `full` (no tree cached for the document, or reparsed after
    /// an incremental tree with an error), `incremental` (the cached tree reused), `unchanged`
    /// (the text equals the text of the document's last successful `syntax.*` call).
    mode: Mode,
}

pub(crate) struct Tree;

impl Command for Tree {
    const NAME: &'static str = "syntax.tree";
    const DOC: &'static str = "Report a document's syntax tree: whether it has errors, its \
        `ERROR` and `MISSING` nodes as char spans, its S-expression form, its node count, and \
        `mode`: how the tree was obtained (`full`, `incremental` or `unchanged`). Read-only; a \
        document longer than 1048576 chars is refused with `too_large`.";
    const MUTATING: bool = false;
    type Params = DocParams;
    type Output = TreeResult;

    fn examples() -> Vec<Example> {
        vec![
            Example {
                description: "the tree of a small function, first call",
                initial_text: "",
                files: vec![("a.rs", "fn f() {}")],
                setup: vec![("doc.open", json!({"path": "a.rs"}))],
                params: json!({"doc": 1}),
                result: json!({
                    "has_error": false,
                    "errors": [],
                    "sexp": "(source_file@0..9 (function_item@0..9 name:(identifier@3..4) \
                        parameters:(parameters@4..6) body:(block@7..9)))",
                    "node_count": 10,
                    "mode": "full",
                }),
            },
            Example {
                description: "after an edit the cached tree is reused",
                initial_text: "",
                files: vec![("a.rs", "fn f() {}")],
                setup: vec![
                    ("doc.open", json!({"path": "a.rs"})),
                    ("syntax.tree", json!({"doc": 1})),
                    ("doc.insert", json!({"doc": 1, "offset": 8, "text": " "})),
                ],
                params: json!({"doc": 1}),
                result: json!({
                    "has_error": false,
                    "errors": [],
                    "sexp": "(source_file@0..10 (function_item@0..10 name:(identifier@3..4) \
                        parameters:(parameters@4..6) body:(block@7..10)))",
                    "node_count": 10,
                    "mode": "incremental",
                }),
            },
            Example {
                description: "an unclosed parenthesis",
                initial_text: "",
                files: vec![("a.rs", "fn a() { (")],
                setup: vec![("doc.open", json!({"path": "a.rs"}))],
                params: json!({"doc": 1}),
                result: json!({
                    "has_error": true,
                    "errors": [{"kind": "error", "start": 0, "end": 10}],
                    "sexp": "(source_file@0..10 (ERROR@0..10 (identifier@3..4) \
                        (parameters@4..6)))",
                    "node_count": 9,
                    "mode": "full",
                }),
            },
        ]
    }

    fn refusals() -> Option<Vec<Refusal>> {
        Some(vec![
            Refusal::example(RejectReason::NoSuchDocument),
            Refusal::example(RejectReason::TooLarge),
        ])
    }

    fn refusal_examples() -> Vec<RefusalExample> {
        vec![
            no_such_document_example(json!({"doc": 9})),
            too_large_example(json!({"doc": 1})),
        ]
    }

    fn run(ctx: &mut Context, p: DocParams) -> Result<TreeResult, CommandError> {
        let parts = ctx.parts();
        let (text, _) = checked_doc(parts.docs, p.doc)?;
        let (tree, mode) = parts.syntax.tree(p.doc, &text, parts.docs)?;
        let errors = tree
            .errors()
            .into_iter()
            .map(|e| ErrorSpanOut {
                kind: match e.kind {
                    ErrorKind::Error => ErrorSpanKind::Error,
                    ErrorKind::Missing => ErrorSpanKind::Missing,
                },
                start: e.start,
                end: e.end,
            })
            .collect::<Vec<_>>();
        Ok(TreeResult {
            has_error: tree.has_error(),
            errors,
            sexp: tree.sexp(),
            node_count: tree.node_count(),
            mode,
        })
    }
}

// ---------------------------------------------------------------------------------------------
// syntax.highlight

/// Params of `syntax.highlight`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct HighlightParams {
    /// Id of an open document.
    doc: u64,
    /// Char offset where the spans start (default 0). Omit it for the default; `null` is
    /// refused.
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "usize", transform = no_default)]
    start: Option<usize>,
    /// Char offset where the spans end (default: the text's length in chars). Omit it for the
    /// default; `null` is refused.
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "usize", transform = no_default)]
    end: Option<usize>,
}

/// The kind of a highlight span.
#[derive(Clone, Copy, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SpanKind {
    /// Inside a line or block comment, doc comments included.
    Comment,
    /// Inside a string, raw string or char literal, quotes and prefixes included.
    String,
    /// Inside an integer or float literal.
    Number,
    /// A keyword.
    Keyword,
    /// A type name or primitive type.
    Type,
    /// A function name at its definition or call.
    Function,
    /// Everything else.
    Plain,
}

impl From<HighlightKind> for SpanKind {
    fn from(k: HighlightKind) -> Self {
        match k {
            HighlightKind::Comment => SpanKind::Comment,
            HighlightKind::String => SpanKind::String,
            HighlightKind::Number => SpanKind::Number,
            HighlightKind::Keyword => SpanKind::Keyword,
            HighlightKind::Type => SpanKind::Type,
            HighlightKind::Function => SpanKind::Function,
            HighlightKind::Plain => SpanKind::Plain,
        }
    }
}

/// One highlight span, in char offsets.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct SpanOut {
    /// Char offset of the span's first char.
    start: usize,
    /// Char offset after the span's last char.
    end: usize,
    /// The kind of every char in the span.
    kind: SpanKind,
}

/// Result of `syntax.highlight`.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct HighlightResult {
    /// Sorted, contiguous, non-overlapping spans covering `start..end` exactly; adjacent
    /// spans of the same kind are merged. Empty for an empty range.
    spans: Vec<SpanOut>,
}

pub(crate) struct Highlight;

impl Command for Highlight {
    const NAME: &'static str = "syntax.highlight";
    const DOC: &'static str = "Return highlight spans that cover the chars `start..end` of a \
        document exactly once (default: the whole text), in seven kinds: `comment`, `string`, \
        `number`, `keyword`, `type`, `function`, `plain` (rules in `docs/protocol/syntax.md`). \
        Read-only; a document longer than 1048576 chars is refused with `too_large`.";
    const MUTATING: bool = false;
    type Params = HighlightParams;
    type Output = HighlightResult;

    fn examples() -> Vec<Example> {
        vec![
            Example {
                description: "the whole text",
                initial_text: "",
                files: vec![("a.rs", "fn f() { let s: u8 = 1; } // c")],
                setup: vec![("doc.open", json!({"path": "a.rs"}))],
                params: json!({"doc": 1}),
                result: json!({"spans": [
                    {"start": 0, "end": 2, "kind": "keyword"},
                    {"start": 2, "end": 3, "kind": "plain"},
                    {"start": 3, "end": 4, "kind": "function"},
                    {"start": 4, "end": 9, "kind": "plain"},
                    {"start": 9, "end": 12, "kind": "keyword"},
                    {"start": 12, "end": 16, "kind": "plain"},
                    {"start": 16, "end": 18, "kind": "type"},
                    {"start": 18, "end": 21, "kind": "plain"},
                    {"start": 21, "end": 22, "kind": "number"},
                    {"start": 22, "end": 26, "kind": "plain"},
                    {"start": 26, "end": 30, "kind": "comment"},
                ]}),
            },
            Example {
                description: "a range, clipped at its ends",
                initial_text: "",
                files: vec![("a.rs", "fn f() { let s: u8 = 1; } // c")],
                setup: vec![("doc.open", json!({"path": "a.rs"}))],
                params: json!({"doc": 1, "start": 10, "end": 17}),
                result: json!({"spans": [
                    {"start": 10, "end": 12, "kind": "keyword"},
                    {"start": 12, "end": 16, "kind": "plain"},
                    {"start": 16, "end": 17, "kind": "type"},
                ]}),
            },
        ]
    }

    fn refusals() -> Option<Vec<Refusal>> {
        Some(vec![
            Refusal::example(RejectReason::NoSuchDocument),
            Refusal::example(RejectReason::TooLarge),
            Refusal::example(RejectReason::OutOfRange),
            Refusal::example(RejectReason::InvertedRange),
        ])
    }

    fn refusal_examples() -> Vec<RefusalExample> {
        vec![
            no_such_document_example(json!({"doc": 9})),
            too_large_example(json!({"doc": 1})),
            RefusalExample {
                description: "a start beyond the end of the text",
                initial_text: "",
                files: vec![SeedFile::text("a.rs", "fn f() { let s: u8 = 1; }")],
                setup: vec![("doc.open", json!({"path": "a.rs"}))],
                params: json!({"doc": 1, "start": 99}),
                kind: RejectReason::OutOfRange,
            },
            RefusalExample {
                description: "a start after the end",
                initial_text: "",
                files: vec![SeedFile::text("a.rs", "fn f() { let s: u8 = 1; }")],
                setup: vec![("doc.open", json!({"path": "a.rs"}))],
                params: json!({"doc": 1, "start": 5, "end": 2}),
                kind: RejectReason::InvertedRange,
            },
        ]
    }

    fn run(ctx: &mut Context, p: HighlightParams) -> Result<HighlightResult, CommandError> {
        let parts = ctx.parts();
        let (text, len_chars) = checked_doc(parts.docs, p.doc)?;
        let start = p.start.unwrap_or(0);
        let end = p.end.unwrap_or(len_chars);
        check_offset("start", start, len_chars)?;
        check_offset("end", end, len_chars)?;
        if start > end {
            return Err(CommandError::rejected(
                RejectReason::InvertedRange,
                format!("start {start} is after end {end}"),
            ));
        }
        let (tree, _) = parts.syntax.tree(p.doc, &text, parts.docs)?;
        let spans = highlight(tree, start..end)
            .into_iter()
            .map(|s| SpanOut {
                start: s.start,
                end: s.end,
                kind: s.kind.into(),
            })
            .collect();
        Ok(HighlightResult { spans })
    }
}

// ---------------------------------------------------------------------------------------------
// syntax.match-bracket

/// Params of `syntax.match-bracket`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct MatchBracketParams {
    /// Id of an open document.
    doc: u64,
    /// Char offset of the bracket (`offset <= len_chars`).
    offset: usize,
}

/// A matched bracket pair, in char offsets.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct BracketOut {
    /// Char offset of the opening bracket.
    open: usize,
    /// Char offset of the closing bracket.
    close: usize,
}

/// Result of `syntax.match-bracket`.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct MatchBracketResult {
    /// The pair the bracket at `offset` belongs to, or `null`: not a bracket, inside a
    /// string, char literal or comment, without a partner, or `offset` = `len_chars`.
    bracket: Option<BracketOut>,
}

pub(crate) struct MatchBracket;

impl Command for MatchBracket {
    const NAME: &'static str = "syntax.match-bracket";
    const DOC: &'static str = "If the char at `offset` is one of `( ) [ ] { }` outside strings, \
        char literals and comments and has a partner, return the offsets of the opening and \
        closing bracket; otherwise `bracket` is `null`. Partners are bracket leaves of the same \
        kind that are direct children of the same syntax node, paired left to right by a \
        stack; a bracket the parser inserted is never a partner (rules in \
        `docs/protocol/syntax.md`). Read-only; a document longer than 1048576 chars is refused \
        with `too_large`.";
    const MUTATING: bool = false;
    type Params = MatchBracketParams;
    type Output = MatchBracketResult;

    fn examples() -> Vec<Example> {
        vec![
            Example {
                description: "the braces of a function body",
                initial_text: "",
                files: vec![("a.rs", "fn f() { let s: u8 = 1; } // c")],
                setup: vec![("doc.open", json!({"path": "a.rs"}))],
                params: json!({"doc": 1, "offset": 7}),
                result: json!({"bracket": {"open": 7, "close": 24}}),
            },
            Example {
                description: "an unmatched bracket",
                initial_text: "",
                files: vec![("a.rs", "fn a() { (")],
                setup: vec![("doc.open", json!({"path": "a.rs"}))],
                params: json!({"doc": 1, "offset": 9}),
                result: json!({"bracket": null}),
            },
        ]
    }

    fn refusals() -> Option<Vec<Refusal>> {
        Some(vec![
            Refusal::example(RejectReason::NoSuchDocument),
            Refusal::example(RejectReason::TooLarge),
            Refusal::example(RejectReason::OutOfRange),
        ])
    }

    fn refusal_examples() -> Vec<RefusalExample> {
        vec![
            no_such_document_example(json!({"doc": 9, "offset": 0})),
            too_large_example(json!({"doc": 1, "offset": 0})),
            RefusalExample {
                description: "an offset beyond the end of the text",
                initial_text: "",
                files: vec![SeedFile::text("a.rs", "fn f() { let s: u8 = 1; }")],
                setup: vec![("doc.open", json!({"path": "a.rs"}))],
                params: json!({"doc": 1, "offset": 99}),
                kind: RejectReason::OutOfRange,
            },
        ]
    }

    fn run(ctx: &mut Context, p: MatchBracketParams) -> Result<MatchBracketResult, CommandError> {
        let parts = ctx.parts();
        let (text, len_chars) = checked_doc(parts.docs, p.doc)?;
        check_offset("offset", p.offset, len_chars)?;
        let (tree, _) = parts.syntax.tree(p.doc, &text, parts.docs)?;
        let bracket = match_bracket(tree, p.offset).map(|b| BracketOut {
            open: b.open,
            close: b.close,
        });
        Ok(MatchBracketResult { bracket })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;
    use crate::standard_registry;

    fn run(reg: &Registry, ctx: &mut Context, name: &str, params: Value) -> Value {
        reg.execute(ctx, name, params)
            .unwrap_or_else(|e| panic!("{name}: {e:?}"))
    }

    fn kind_of(err: CommandError) -> String {
        err.to_json()["reason"].as_str().unwrap_or("").to_owned()
    }

    #[test]
    fn size_limit_is_chars_and_refusals_keep_the_cache() {
        let reg = standard_registry().expect("registry");
        // 1,048,576 chars, 2,097,150 bytes: one comment token.
        let text = format!("//{}", "é".repeat(MAX_CHARS - 2));
        let mut ctx = Context::with_files(&[("big.rs", text.as_bytes())]).expect("ctx");
        run(&reg, &mut ctx, "doc.open", json!({"path": "big.rs"}));
        let first = run(&reg, &mut ctx, "syntax.tree", json!({"doc": 1}));
        assert_eq!(first["mode"], "full");
        assert_eq!(first["has_error"], false);
        let cached = ctx.parts().syntax.cached();
        run(
            &reg,
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": 0, "text": "x"}),
        );
        for (name, params) in [
            ("syntax.tree", json!({"doc": 1})),
            ("syntax.highlight", json!({"doc": 1})),
            ("syntax.match-bracket", json!({"doc": 1, "offset": 0})),
            (
                "syntax.match-bracket",
                json!({"doc": 1, "offset": 2_000_000}),
            ),
            ("syntax.highlight", json!({"doc": 1, "start": 5, "end": 1})),
        ] {
            let err = reg.execute(&mut ctx, name, params).expect_err(name);
            assert!(matches!(err, CommandError::Rejected { .. }), "{err:?}");
            assert_eq!(kind_of(err), "too_large", "{name}");
            assert_eq!(ctx.parts().syntax.cached(), cached, "{name}");
        }
        run(&reg, &mut ctx, "doc.undo", json!({"doc": 1}));
        let again = run(&reg, &mut ctx, "syntax.tree", json!({"doc": 1}));
        assert_eq!(again["mode"], "unchanged");
        assert_eq!(RejectReason::TooLarge.as_str(), "too_large");
        assert!(check_size(1, MAX_CHARS).is_ok());
        assert!(check_size(1, MAX_CHARS + 1).is_err());
    }

    #[test]
    fn error_order_and_refusals_leave_the_cache() {
        let reg = standard_registry().expect("registry");
        let mut ctx = Context::with_files(&[("a.rs", b"fn f() {}")]).expect("ctx");
        run(&reg, &mut ctx, "doc.open", json!({"path": "a.rs"}));
        run(&reg, &mut ctx, "syntax.tree", json!({"doc": 1}));
        let cached = ctx.parts().syntax.cached();
        run(
            &reg,
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": 0, "text": "//"}),
        );
        for (name, params, kind) in [
            ("syntax.tree", json!({"doc": 2}), "no_such_document"),
            ("syntax.tree", json!({"doc": 0}), "no_such_document"),
            (
                "syntax.match-bracket",
                json!({"doc": 1, "offset": 12}),
                "out_of_range",
            ),
            (
                "syntax.highlight",
                json!({"doc": 1, "start": 12}),
                "out_of_range",
            ),
            (
                "syntax.highlight",
                json!({"doc": 1, "start": 9, "end": 12}),
                "out_of_range",
            ),
            (
                "syntax.highlight",
                json!({"doc": 1, "start": 5, "end": 2}),
                "inverted_range",
            ),
        ] {
            let err = reg.execute(&mut ctx, name, params).expect_err(name);
            assert_eq!(kind_of(err), kind, "{name}");
            assert_eq!(ctx.parts().syntax.cached(), cached, "{name}");
        }
        for bad in [
            json!({"doc": 1, "start": null}),
            json!({"doc": 1, "end": null}),
            json!({"doc": 1, "x": 1}),
        ] {
            let err = reg
                .execute(&mut ctx, "syntax.highlight", bad.clone())
                .expect_err("null");
            assert!(matches!(err, CommandError::InvalidParams { .. }), "{bad}");
        }
        // The edited text is then reparsed; the comment holds the whole text.
        let t = run(&reg, &mut ctx, "syntax.tree", json!({"doc": 1}));
        assert_ne!(t["mode"], "unchanged");
        let h = run(&reg, &mut ctx, "syntax.highlight", json!({"doc": 1}));
        assert_eq!(
            h["spans"],
            json!([{"start": 0, "end": 11, "kind": "comment"}])
        );
        let e = run(
            &reg,
            &mut ctx,
            "syntax.highlight",
            json!({"doc": 1, "start": 11}),
        );
        assert_eq!(e["spans"], json!([]));
        let b = run(
            &reg,
            &mut ctx,
            "syntax.match-bracket",
            json!({"doc": 1, "offset": 11}),
        );
        assert_eq!(b["bracket"], Value::Null);
    }

    #[test]
    fn mode_follows_the_cache_and_closed_documents_are_dropped() {
        let reg = standard_registry().expect("registry");
        let mut ctx =
            Context::with_files(&[("a.rs", b"fn f() {}"), ("b.rs", b"fn g() {}")]).expect("ctx");
        run(&reg, &mut ctx, "doc.open", json!({"path": "a.rs"}));
        run(&reg, &mut ctx, "doc.open", json!({"path": "b.rs"}));
        // Any of the three commands fills the cache.
        run(
            &reg,
            &mut ctx,
            "syntax.match-bracket",
            json!({"doc": 1, "offset": 4}),
        );
        assert_eq!(
            run(&reg, &mut ctx, "syntax.tree", json!({"doc": 1}))["mode"],
            "unchanged"
        );
        run(&reg, &mut ctx, "syntax.highlight", json!({"doc": 2}));
        assert_eq!(ctx.parts().syntax.cached().len(), 2);
        run(&reg, &mut ctx, "doc.close", json!({"doc": 2}));
        run(
            &reg,
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": 8, "text": " "}),
        );
        assert_eq!(
            run(&reg, &mut ctx, "syntax.tree", json!({"doc": 1}))["mode"],
            "incremental"
        );
        let ids: Vec<u64> = ctx
            .parts()
            .syntax
            .cached()
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(ids, [1]);
        // An edit that leaves an error: reparsed in full.
        run(
            &reg,
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": 9, "text": "("}),
        );
        let t = run(&reg, &mut ctx, "syntax.tree", json!({"doc": 1}));
        assert_eq!(t["mode"], "full");
        assert_eq!(t["has_error"], true);
    }

    #[test]
    fn commands_are_not_mutating_and_leave_no_document_change() {
        let reg = standard_registry().expect("registry");
        let d = reg.describe();
        for name in ["syntax.tree", "syntax.highlight", "syntax.match-bracket"] {
            let c = d["commands"]
                .as_array()
                .expect("commands")
                .iter()
                .find(|c| c["name"] == name)
                .expect(name);
            assert_eq!(c["mutating"], false, "{name}");
        }
        assert!(
            d["error_kinds"]["rejected"]
                .as_array()
                .expect("rejected")
                .iter()
                .any(|k| k["kind"] == "too_large")
        );
        let mut ctx = Context::with_files(&[("a.rs", b"fn f() {}")]).expect("ctx");
        run(&reg, &mut ctx, "doc.open", json!({"path": "a.rs"}));
        run(
            &reg,
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": 9, "text": "a"}),
        );
        run(&reg, &mut ctx, "syntax.tree", json!({"doc": 1}));
        run(&reg, &mut ctx, "syntax.highlight", json!({"doc": 1}));
        run(
            &reg,
            &mut ctx,
            "syntax.match-bracket",
            json!({"doc": 1, "offset": 4}),
        );
        run(
            &reg,
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": 10, "text": "b"}),
        );
        run(&reg, &mut ctx, "doc.undo", json!({"doc": 1}));
        let text = run(&reg, &mut ctx, "doc.text", json!({"doc": 1}));
        assert_eq!(text["text"], "fn f() {}");
    }
}
