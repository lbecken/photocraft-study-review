//! The `nav.*` command group (T-023): a document's outline, going to a line and column or to
//! an outline item, and finding items by name across the project's Rust files.
//!
//! Every command that needs an outline parses the current text afresh
//! ([`SyntaxTree::parse`], then [`kiln_syntax::outline::outline`]); none reads or changes the
//! tree cache of the `syntax.*` commands. A document longer than [`MAX_CHARS`] chars is
//! refused by `nav.outline` and `nav.goto-symbol` with `too_large` before anything is parsed,
//! and skipped (`too_large`) by `nav.workspace-symbols`. `nav.goto-line` parses nothing.
//! The two `nav.goto-*` commands end the document's typing group when they succeed, as
//! `cursor.set` does; a refused request changes nothing. Protocol description:
//! `docs/protocol/nav.md`.

use kiln_syntax::outline::{ItemKind, OutlineItem, outline};
use kiln_syntax::{ParseError, SyntaxTree};
use kiln_text::{Buffer, LineCol, Selection};
use kiln_workspace::project as walk;
use kiln_workspace::{EntryKind, VfsError, decode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::cursor::{CursorResult, cursor_result};
use crate::reason::{InternalReason, RejectReason};
use crate::registry::{Command, CommandError, Context, Example, Registry, RegistryError};
use crate::syntax::MAX_CHARS;

/// Register the `nav.*` commands.
pub(crate) fn register(registry: &mut Registry) -> Result<(), RegistryError> {
    registry.register::<Outline>()?;
    registry.register::<GotoLine>()?;
    registry.register::<GotoSymbol>()?;
    registry.register::<WorkspaceSymbols>()?;
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

/// Publish the true default of `col` instead of the `"default": null` that
/// `#[serde(default)]` adds to the schema: `null` is refused.
fn default_zero(schema: &mut schemars::Schema) {
    schema.insert("default".to_owned(), json!(0));
}

/// Default of `nav.workspace-symbols`' `limit`.
const DEFAULT_LIMIT: usize = 200;

/// As [`default_zero`], for `limit`.
fn default_limit(schema: &mut schemars::Schema) {
    schema.insert("default".to_owned(), json!(DEFAULT_LIMIT));
}

/// The walk limit of `nav.workspace-symbols`: the largest `limit` of `project.tree`.
const WALK_LIMIT: usize = 1_000_000;

/// A `u64` from the params as a `usize`; a value that does not fit saturates.
fn index(n: u64) -> usize {
    usize::try_from(n).unwrap_or(usize::MAX)
}

/// `internal`/`other` for a parse that returned no tree (D-044).
fn no_tree(e: ParseError) -> CommandError {
    CommandError::Internal {
        reason: InternalReason::Other,
        message: e.to_string(),
    }
}

/// `internal`/`other` for a position the buffer should always accept.
fn bad_position(what: &str, e: &dyn std::fmt::Display) -> CommandError {
    CommandError::Internal {
        reason: InternalReason::Other,
        message: format!("{what}: {e}"),
    }
}

/// Refuse a document longer than [`MAX_CHARS`] with `too_large`.
fn check_size(doc: u64, len_chars: usize) -> Result<(), CommandError> {
    if len_chars > MAX_CHARS {
        return Err(CommandError::rejected(
            RejectReason::TooLarge,
            format!(
                "document {doc} has {len_chars} chars; nav.outline and nav.goto-symbol accept \
                 at most {MAX_CHARS}"
            ),
        ));
    }
    Ok(())
}

/// The outline of open document `doc`, after the checks `no_such_document` → `too_large`.
fn doc_outline(ctx: &Context, doc: u64) -> Result<Vec<OutlineItem>, CommandError> {
    let buffer = ctx.doc(doc)?.buffer();
    check_size(doc, buffer.len_chars())?;
    let tree = SyntaxTree::parse(&buffer.text()).map_err(no_tree)?;
    Ok(outline(&tree))
}

/// The kind of an outline item, as `ItemKind::as_str` names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum KindName {
    /// `fn` (with or without a body).
    Fn,
    /// `struct`.
    Struct,
    /// `enum`.
    Enum,
    /// `trait`.
    Trait,
    /// `impl`.
    Impl,
    /// `mod` (inline or `mod x;`).
    Mod,
}

fn kind_name(kind: ItemKind) -> KindName {
    match kind {
        ItemKind::Fn => KindName::Fn,
        ItemKind::Struct => KindName::Struct,
        ItemKind::Enum => KindName::Enum,
        ItemKind::Trait => KindName::Trait,
        ItemKind::Impl => KindName::Impl,
        ItemKind::Mod => KindName::Mod,
    }
}

/// One outline item (char offsets into the document's current text).
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct ItemOut {
    /// `fn`, `struct`, `enum`, `trait`, `impl` or `mod`.
    kind: KindName,
    /// The name; for `impl` the type, or `<trait> for <type>`, whitespace runs collapsed.
    name: String,
    /// Char offset of the item's start (attributes and doc comments are not part of it).
    start: usize,
    /// Char offset of the item's end.
    end: usize,
    /// Char offset of the name's start (for `impl`: of the type).
    name_start: usize,
    /// Char offset of the name's end (for `impl`: of the type).
    name_end: usize,
    /// Number of listed items that enclose this one.
    depth: usize,
}

impl From<&OutlineItem> for ItemOut {
    fn from(i: &OutlineItem) -> Self {
        ItemOut {
            kind: kind_name(i.kind),
            name: i.name.clone(),
            start: i.start,
            end: i.end,
            name_start: i.name_start,
            name_end: i.name_end,
            depth: i.depth,
        }
    }
}

/// The text of Pinned (g) example 1, used by the examples.
const EXAMPLE_1: &str =
    "impl<T> Display for Foo<T> { fn fmt(&self) {} }\nmod m;\ntrait T { fn x(&self); }";

// ---------------------------------------------------------------------------------------------
// nav.outline

/// Params naming one open document.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct DocParams {
    /// Id of an open document.
    doc: u64,
}

/// Result of `nav.outline`.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct OutlineResult {
    /// Every item, in the order of its start (preorder).
    items: Vec<ItemOut>,
}

pub(crate) struct Outline;

impl Command for Outline {
    const NAME: &'static str = "nav.outline";
    const DOC: &'static str = "List a document's `fn`, `struct`, `enum`, `trait`, `impl` and \
        `mod` items that are not inside a function body, a block, an `extern` block, a \
        `const` or `static` value or a macro invocation, in the order of their start, each \
        with its name, char range, name range and nesting depth. Parses the current text \
        afresh; read-only. A document longer than 1048576 chars is refused with `too_large`.";
    const MUTATING: bool = false;
    type Params = DocParams;
    type Output = OutlineResult;

    fn examples() -> Vec<Example> {
        vec![Example {
            description: "an impl, a mod and a trait with their fns",
            initial_text: "",
            files: vec![("a.rs", EXAMPLE_1)],
            setup: vec![("doc.open", json!({"path": "a.rs"}))],
            params: json!({"doc": 1}),
            result: json!({"items": [
                {"kind": "impl", "name": "Display for Foo<T>", "start": 0, "end": 47,
                    "name_start": 20, "name_end": 26, "depth": 0},
                {"kind": "fn", "name": "fmt", "start": 29, "end": 45,
                    "name_start": 32, "name_end": 35, "depth": 1},
                {"kind": "mod", "name": "m", "start": 48, "end": 54,
                    "name_start": 52, "name_end": 53, "depth": 0},
                {"kind": "trait", "name": "T", "start": 55, "end": 79,
                    "name_start": 61, "name_end": 62, "depth": 0},
                {"kind": "fn", "name": "x", "start": 65, "end": 77,
                    "name_start": 68, "name_end": 69, "depth": 1},
            ]}),
        }]
    }

    fn run(ctx: &mut Context, p: DocParams) -> Result<OutlineResult, CommandError> {
        let items = doc_outline(ctx, p.doc)?;
        Ok(OutlineResult {
            items: items.iter().map(ItemOut::from).collect(),
        })
    }
}

// ---------------------------------------------------------------------------------------------
// nav.goto-line

/// Params of `nav.goto-line`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct GotoLineParams {
    /// Id of an open document.
    doc: u64,
    /// Zero-based line; one beyond the last line is clamped to the last line.
    line: u64,
    /// Zero-based column in chars; default 0. One beyond the line's content (without its line
    /// break) is clamped to its end. `null` is refused.
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "u64", transform = default_zero)]
    col: Option<u64>,
}

/// The caret position of (`line`, `col`) clamped to `buffer`: the line to the last line, the
/// column to the length of that line's content.
fn clamped(buffer: &Buffer, line: usize, col: usize) -> Result<usize, CommandError> {
    let line = line.min(buffer.len_lines().saturating_sub(1));
    let content = buffer
        .line(line)
        .map_err(|e| bad_position("line of a clamped position", &e))?;
    let col = col.min(content.chars().count());
    buffer
        .line_col_to_char(LineCol { line, col })
        .map_err(|e| bad_position("clamped position", &e))
}

pub(crate) struct GotoLine;

impl Command for GotoLine {
    const NAME: &'static str = "nav.goto-line";
    const DOC: &'static str = "Put a document's caret (anchor = head) at a zero-based line and \
        column (`col` default 0, in chars), each clamped to the document: the line to the last \
        line, the column to that line's length without its break. Never refuses a line or \
        column beyond the document. Ends the document's typing group.";
    const MUTATING: bool = true;
    type Params = GotoLineParams;
    type Output = CursorResult;

    fn examples() -> Vec<Example> {
        vec![
            Example {
                description: "a column beyond the line is clamped before its CRLF",
                initial_text: "",
                files: vec![("a.txt", "ab\r\ncdef\n")],
                setup: vec![("doc.open", json!({"path": "a.txt"}))],
                params: json!({"doc": 1, "line": 1, "col": 9}),
                result: json!({"doc": 1, "anchor": 8, "head": 8, "start": 8, "end": 8,
                    "line": 1, "col": 4}),
            },
            Example {
                description: "a line beyond the document is clamped to the last line",
                initial_text: "",
                files: vec![("a.txt", "ab\r\ncdef\n")],
                setup: vec![("doc.open", json!({"path": "a.txt"}))],
                params: json!({"doc": 1, "line": 8}),
                result: json!({"doc": 1, "anchor": 9, "head": 9, "start": 9, "end": 9,
                    "line": 2, "col": 0}),
            },
        ]
    }

    fn run(ctx: &mut Context, p: GotoLineParams) -> Result<CursorResult, CommandError> {
        let buffer = ctx.doc_mut(p.doc)?.buffer_mut();
        let head = clamped(buffer, index(p.line), p.col.map_or(0, index))?;
        buffer
            .set_selection(Selection { anchor: head, head })
            .map_err(|e| bad_position("clamped position", &e))?;
        buffer.break_undo_group();
        cursor_result(p.doc, buffer)
    }
}

// ---------------------------------------------------------------------------------------------
// nav.goto-symbol

/// Params of `nav.goto-symbol`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct GotoSymbolParams {
    /// Id of an open document.
    doc: u64,
    /// Index into the items `nav.outline` gives for the current text.
    index: u64,
}

/// Result of `nav.goto-symbol`.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct GotoSymbolResult {
    /// The item at `index`.
    item: ItemOut,
    /// The selection afterwards: anchor = `name_start`, head = `name_end`.
    cursor: CursorResult,
}

pub(crate) struct GotoSymbol;

impl Command for GotoSymbol {
    const NAME: &'static str = "nav.goto-symbol";
    const DOC: &'static str = "Select the name of item `index` of the document's outline (as \
        `nav.outline` lists it for the current text): anchor = `name_start`, head = \
        `name_end`. An index beyond the items is refused with `out_of_range`; a document \
        longer than 1048576 chars with `too_large`. Ends the document's typing group.";
    const MUTATING: bool = true;
    type Params = GotoSymbolParams;
    type Output = GotoSymbolResult;

    fn examples() -> Vec<Example> {
        vec![Example {
            description: "select the name of the second item",
            initial_text: "",
            files: vec![("a.rs", EXAMPLE_1)],
            setup: vec![("doc.open", json!({"path": "a.rs"}))],
            params: json!({"doc": 1, "index": 1}),
            result: json!({
                "item": {"kind": "fn", "name": "fmt", "start": 29, "end": 45,
                    "name_start": 32, "name_end": 35, "depth": 1},
                "cursor": {"doc": 1, "anchor": 32, "head": 35, "start": 32, "end": 35,
                    "line": 0, "col": 35},
            }),
        }]
    }

    fn run(ctx: &mut Context, p: GotoSymbolParams) -> Result<GotoSymbolResult, CommandError> {
        let items = doc_outline(ctx, p.doc)?;
        let item = items.get(index(p.index)).ok_or_else(|| {
            CommandError::rejected(
                RejectReason::OutOfRange,
                format!(
                    "index {} is beyond the outline of document {} ({} items)",
                    p.index,
                    p.doc,
                    items.len()
                ),
            )
        })?;
        let buffer = ctx.doc_mut(p.doc)?.buffer_mut();
        buffer
            .set_selection(Selection {
                anchor: item.name_start,
                head: item.name_end,
            })
            .map_err(|e| bad_position("name range of an outline item", &e))?;
        buffer.break_undo_group();
        Ok(GotoSymbolResult {
            item: ItemOut::from(item),
            cursor: cursor_result(p.doc, buffer)?,
        })
    }
}

// ---------------------------------------------------------------------------------------------
// nav.workspace-symbols

/// Params of `nav.workspace-symbols`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkspaceParams {
    /// Text to find in item names, ignoring case (not empty).
    query: String,
    /// Most items listed (1 to 10,000; default 200). `null` is refused.
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "u32", range(min = 1, max = 10_000), transform = default_limit)]
    limit: Option<usize>,
}

/// Why a file was not searched (the name of an existing error kind).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SkipReason {
    /// The file was not found when it was read.
    NotFound,
    /// The path was not a regular file when it was read.
    NotAFile,
    /// The path resolved outside the root when it was read.
    OutsideRoot,
    /// Any other read failure.
    IoError,
    /// The file is not valid UTF-8 / UTF-16 text.
    DecodeError,
    /// The text is longer than 1,048,576 chars.
    TooLarge,
}

impl From<&VfsError> for SkipReason {
    fn from(e: &VfsError) -> Self {
        match e {
            VfsError::NotFound { .. } => SkipReason::NotFound,
            VfsError::NotAFile { .. } => SkipReason::NotAFile,
            VfsError::OutsideRoot { .. } => SkipReason::OutsideRoot,
            VfsError::Io { .. } => SkipReason::IoError,
        }
    }
}

/// A file that was not searched.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct Skipped {
    /// Root-relative path of the file.
    path: String,
    /// Why it was not searched.
    reason: SkipReason,
}

/// One matching item.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct Symbol {
    /// Root-relative path of the file.
    path: String,
    /// `fn`, `struct`, `enum`, `trait`, `impl` or `mod`.
    kind: KindName,
    /// The item's name, as `nav.outline` gives it.
    name: String,
    /// Char offset of the item's start in the file's text.
    start: usize,
    /// Char offset of the item's end.
    end: usize,
    /// Char offset of the name's start (for `impl`: of the type).
    name_start: usize,
    /// Char offset of the name's end.
    name_end: usize,
    /// Zero-based line of `name_start`.
    line: usize,
    /// Zero-based column of `name_start`, in chars.
    col: usize,
}

/// Result of `nav.workspace-symbols`.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct WorkspaceResult {
    /// The first `limit` matches: exact names first, then prefixes, then the others; within a
    /// rank by the file's place in the walk, then by start.
    items: Vec<Symbol>,
    /// True iff more than `limit` items matched.
    truncated: bool,
    /// The files not searched, in walk order.
    skipped: Vec<Skipped>,
    /// True iff the walk stopped at 1,000,000 entries.
    walk_truncated: bool,
    /// Directories not descended because of the depth bound (as `project.tree`'s `too_deep`).
    too_deep: usize,
}

/// The text of a project file: the open document's current text, or the file decoded.
fn file_text(ctx: &Context, path: &kiln_workspace::RelPath) -> Result<String, SkipReason> {
    if let Some(doc) = ctx.docs().values().find(|d| d.path() == path) {
        return Ok(doc.buffer().text());
    }
    let bytes = ctx.vfs().read(path).map_err(|e| SkipReason::from(&e))?;
    let (text, _) = decode(&bytes).map_err(|_| SkipReason::DecodeError)?;
    Ok(text)
}

/// Rank of a name for the lowercased query: 0 exact, 1 prefix, 2 contains; None: no match.
fn rank(name: &str, query: &str) -> Option<usize> {
    let name = name.to_lowercase();
    if name == query {
        Some(0)
    } else if name.starts_with(query) {
        Some(1)
    } else if name.contains(query) {
        Some(2)
    } else {
        None
    }
}

pub(crate) struct WorkspaceSymbols;

impl Command for WorkspaceSymbols {
    const NAME: &'static str = "nav.workspace-symbols";
    const DOC: &'static str = "Find outline items whose name contains `query` (ignoring case) \
        in the project's `.rs` files, as `project.tree {}` lists them; an open document's \
        current text is searched in place of its file. Exact matches first, then prefixes, \
        then the others; at most `limit` (default 200). Files that cannot be read or decoded, \
        or are longer than 1048576 chars, are listed in `skipped`. Read-only.";
    const MUTATING: bool = false;
    type Params = WorkspaceParams;
    type Output = WorkspaceResult;

    fn examples() -> Vec<Example> {
        vec![Example {
            description: "exact matches first, then prefixes, then the others",
            initial_text: "",
            files: vec![
                ("a.rs", "fn foo() {}\nstruct FooBar;\n"),
                ("b/c.rs", "mod foo { fn barfoo() {} }\n"),
                ("z.txt", "fn foo() {}"),
            ],
            setup: vec![],
            params: json!({"query": "FOO"}),
            result: json!({
                "items": [
                    {"path": "a.rs", "kind": "fn", "name": "foo", "start": 0, "end": 11,
                        "name_start": 3, "name_end": 6, "line": 0, "col": 3},
                    {"path": "b/c.rs", "kind": "mod", "name": "foo", "start": 0, "end": 26,
                        "name_start": 4, "name_end": 7, "line": 0, "col": 4},
                    {"path": "a.rs", "kind": "struct", "name": "FooBar", "start": 12,
                        "end": 26, "name_start": 19, "name_end": 25, "line": 1, "col": 7},
                    {"path": "b/c.rs", "kind": "fn", "name": "barfoo", "start": 10, "end": 24,
                        "name_start": 13, "name_end": 19, "line": 0, "col": 13},
                ],
                "truncated": false,
                "skipped": [],
                "walk_truncated": false,
                "too_deep": 0,
            }),
        }]
    }

    fn run(ctx: &mut Context, p: WorkspaceParams) -> Result<WorkspaceResult, CommandError> {
        if p.query.is_empty() {
            return Err(CommandError::rejected(
                RejectReason::InvalidPattern,
                "the search pattern is empty",
            ));
        }
        let limit = p.limit.unwrap_or(DEFAULT_LIMIT);
        let query = p.query.to_lowercase();
        let tree = walk::tree(ctx.vfs(), None, WALK_LIMIT).map_err(|e| {
            let message = match &e {
                VfsError::NotAFile { path } => format!("cannot list {path}: not a directory"),
                _ => format!("cannot list the project: {e}"),
            };
            CommandError::rejected((&e).into(), message)
        })?;
        // At most `limit` kept matches per rank; `matched` counts all.
        let mut ranks: [Vec<Symbol>; 3] = [Vec::new(), Vec::new(), Vec::new()];
        let mut matched = 0usize;
        let mut skipped = Vec::new();
        for entry in &tree.entries {
            if entry.kind != EntryKind::File || !entry.path.as_str().ends_with(".rs") {
                continue;
            }
            let path = entry.path.to_string();
            let text = match file_text(ctx, &entry.path) {
                Ok(text) => text,
                Err(reason) => {
                    skipped.push(Skipped { path, reason });
                    continue;
                }
            };
            if text.chars().count() > MAX_CHARS {
                skipped.push(Skipped {
                    path,
                    reason: SkipReason::TooLarge,
                });
                continue;
            }
            let items = outline(&SyntaxTree::parse(&text).map_err(no_tree)?);
            // Built only when a match of this file is kept, for its line and column.
            let mut lines: Option<Buffer> = None;
            for item in &items {
                let Some(r) = rank(&item.name, &query) else {
                    continue;
                };
                matched += 1;
                if ranks[r].len() >= limit {
                    continue;
                }
                let buffer = lines.get_or_insert_with(|| Buffer::from_text(&text));
                let LineCol { line, col } = buffer
                    .char_to_line_col(item.name_start)
                    .map_err(|e| bad_position("name start of an outline item", &e))?;
                ranks[r].push(Symbol {
                    path: path.clone(),
                    kind: kind_name(item.kind),
                    name: item.name.clone(),
                    start: item.start,
                    end: item.end,
                    name_start: item.name_start,
                    name_end: item.name_end,
                    line,
                    col,
                });
            }
        }
        let items: Vec<Symbol> = ranks.into_iter().flatten().take(limit).collect();
        Ok(WorkspaceResult {
            items,
            truncated: matched > limit,
            skipped,
            walk_truncated: tree.truncated,
            too_deep: tree.too_deep,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use kiln_workspace::{DirListing, MemoryFs, RelPath, Vfs, VfsError};
    use serde_json::{Value, json};

    use crate::registry::Context;

    fn reg() -> crate::Registry {
        crate::standard_registry().unwrap()
    }

    fn ctx_with(files: &[(&str, &[u8])]) -> Context {
        Context::with_files(files).unwrap()
    }

    fn kind(e: &crate::CommandError) -> (String, String) {
        let j = e.to_json();
        (
            j["code"].as_str().unwrap_or("").to_owned(),
            j["reason"].as_str().unwrap_or("").to_owned(),
        )
    }

    /// A `MemoryFs` whose `read` of the paths in `fail` fails with the given error (as if the
    /// file had changed after `list_dir` listed it); `reads` counts every `read`.
    #[derive(Debug)]
    struct FailingFs {
        inner: MemoryFs,
        fail: Vec<VfsError>,
        reads: Rc<Cell<usize>>,
    }

    impl Vfs for FailingFs {
        fn read(&self, path: &RelPath) -> Result<Vec<u8>, VfsError> {
            self.reads.set(self.reads.get() + 1);
            if let Some(e) = self.fail.iter().find(|e| e.path() == path) {
                return Err(e.clone());
            }
            self.inner.read(path)
        }
        fn write_atomic(&mut self, path: &RelPath, bytes: &[u8]) -> Result<(), VfsError> {
            self.inner.write_atomic(path, bytes)
        }
        fn is_file(&self, path: &RelPath) -> bool {
            self.inner.is_file(path)
        }
        fn list_dir(&self, dir: Option<&RelPath>) -> Result<DirListing, VfsError> {
            self.inner.list_dir(dir)
        }
    }

    fn p(s: &str) -> RelPath {
        RelPath::new(s).unwrap()
    }

    #[test]
    fn read_failures_are_skipped_with_their_kind_and_the_search_goes_on() {
        let mut inner = MemoryFs::new();
        for name in ["a.rs", "b.rs", "c.rs", "d.rs", "e.rs"] {
            inner.insert(&p(name), b"fn hit() {}");
        }
        let fail = vec![
            VfsError::NotFound { path: p("a.rs") },
            VfsError::NotAFile { path: p("b.rs") },
            VfsError::OutsideRoot { path: p("c.rs") },
            VfsError::Io {
                path: p("d.rs"),
                kind: std::io::ErrorKind::PermissionDenied,
                message: "denied".to_owned(),
            },
        ];
        let fs = FailingFs {
            inner,
            fail,
            reads: Rc::default(),
        };
        let mut ctx = Context::with_vfs(Box::new(fs));
        let out = reg()
            .execute(&mut ctx, "nav.workspace-symbols", json!({"query": "hit"}))
            .unwrap();
        assert_eq!(
            out["skipped"],
            json!([
                {"path": "a.rs", "reason": "not_found"},
                {"path": "b.rs", "reason": "not_a_file"},
                {"path": "c.rs", "reason": "outside_root"},
                {"path": "d.rs", "reason": "io_error"},
            ])
        );
        assert_eq!(out["items"].as_array().unwrap().len(), 1);
        assert_eq!(out["items"][0]["path"], json!("e.rs"));
    }

    #[test]
    fn skip_reasons_are_existing_kinds() {
        use super::SkipReason::*;
        for r in [
            NotFound,
            NotAFile,
            OutsideRoot,
            IoError,
            DecodeError,
            TooLarge,
        ] {
            let name = serde_json::to_value(r).unwrap();
            let name = name.as_str().unwrap();
            assert!(
                crate::RejectReason::ALL.iter().any(|k| k.as_str() == name),
                "{name} is not a kind"
            );
        }
    }

    #[test]
    fn open_document_is_searched_and_its_file_not_read() {
        let mut inner = MemoryFs::new();
        inner.insert(&p("a.rs"), b"fn on_disk() {}");
        let reads = Rc::new(Cell::new(0));
        let fs = FailingFs {
            inner,
            fail: vec![],
            reads: Rc::clone(&reads),
        };
        let mut ctx = Context::with_vfs(Box::new(fs));
        let r = reg();
        r.execute(&mut ctx, "doc.open", json!({"path": "a.rs"}))
            .unwrap();
        r.execute(
            &mut ctx,
            "doc.replace",
            json!({"doc": 1, "start": 0, "end": 15, "text": "fn in_memory() {}"}),
        )
        .unwrap();
        assert_eq!(reads.get(), 1);
        let out = r
            .execute(&mut ctx, "nav.workspace-symbols", json!({"query": "_"}))
            .unwrap();
        assert_eq!(out["items"][0]["name"], json!("in_memory"));
        assert_eq!(out["items"].as_array().unwrap().len(), 1);
        assert_eq!(reads.get(), 1, "the open document's file was read");
    }

    #[test]
    fn ranks_then_walk_order_then_start() {
        let mut ctx = ctx_with(&[
            ("a.rs", b"fn xab() {}\nfn ab() {}\nfn abx() {}\n"),
            ("b.rs", b"fn ab() {}\nfn xxab() {}\n"),
        ]);
        let out = reg()
            .execute(&mut ctx, "nav.workspace-symbols", json!({"query": "AB"}))
            .unwrap();
        let got: Vec<(String, String)> = out["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| {
                (
                    i["path"].as_str().unwrap().to_owned(),
                    i["name"].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        let want: Vec<(String, String)> = [
            ("a.rs", "ab"),
            ("b.rs", "ab"),
            ("a.rs", "abx"),
            ("a.rs", "xab"),
            ("b.rs", "xxab"),
        ]
        .iter()
        .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
        .collect();
        assert_eq!(got, want);
        assert_eq!(out["truncated"], json!(false));
        // limit 2 keeps the two exact matches; five matched.
        let out = reg()
            .execute(
                &mut ctx,
                "nav.workspace-symbols",
                json!({"query": "ab", "limit": 2}),
            )
            .unwrap();
        assert_eq!(out["items"].as_array().unwrap().len(), 2);
        assert_eq!(out["items"][1]["path"], json!("b.rs"));
        assert_eq!(out["truncated"], json!(true));
    }

    #[test]
    fn decode_error_and_too_large_are_skipped() {
        let big = format!("//{}", "x".repeat(super::MAX_CHARS - 1));
        let mut ctx = ctx_with(&[
            ("a.rs", b"fn \xff() {}"),
            ("b.rs", big.as_bytes()),
            ("c.rs", b"\xef\xbb\xbffn bom() {}"),
        ]);
        let out = reg()
            .execute(&mut ctx, "nav.workspace-symbols", json!({"query": "bom"}))
            .unwrap();
        assert_eq!(
            out["skipped"],
            json!([{"path": "a.rs", "reason": "decode_error"},
                {"path": "b.rs", "reason": "too_large"}])
        );
        // The BOM is not part of the text.
        assert_eq!(out["items"][0]["start"], json!(0));
        assert_eq!(out["items"][0]["name_start"], json!(3));
    }

    #[test]
    fn walk_errors_fail_the_command() {
        // A `.gitignore` that is a file but cannot be read fails the walk with its kind.
        let mut inner = MemoryFs::new();
        inner.insert(&p(".gitignore"), b"x\n");
        inner.insert(&p("a.rs"), b"fn a() {}");
        let fs = FailingFs {
            inner,
            fail: vec![VfsError::Io {
                path: p(".gitignore"),
                kind: std::io::ErrorKind::PermissionDenied,
                message: "denied".to_owned(),
            }],
            reads: Rc::default(),
        };
        let mut ctx = Context::with_vfs(Box::new(fs));
        let e = reg()
            .execute(&mut ctx, "nav.workspace-symbols", json!({"query": "a"}))
            .unwrap_err();
        assert_eq!(kind(&e), ("rejected".to_owned(), "io_error".to_owned()));
        assert!(ctx.log().is_empty());
    }

    #[test]
    fn refusals_and_their_order() {
        let r = reg();
        let mut ctx = ctx_with(&[("a.rs", b"fn a() {}")]);
        for bad in [
            json!({"query": "a", "limit": 0}),
            json!({"query": "a", "limit": 10_001}),
            json!({"query": "a", "limit": null}),
            json!({"query": "", "limit": 0}),
            json!({"limit": 1}),
        ] {
            let e = r
                .execute(&mut ctx, "nav.workspace-symbols", bad.clone())
                .unwrap_err();
            assert_eq!(kind(&e).0, "invalid_params", "{bad}");
        }
        let e = r
            .execute(&mut ctx, "nav.workspace-symbols", json!({"query": ""}))
            .unwrap_err();
        assert_eq!(kind(&e), ("rejected".into(), "invalid_pattern".into()));
        assert!(
            r.execute(
                &mut ctx,
                "nav.workspace-symbols",
                json!({"query": "a", "limit": 10_000})
            )
            .is_ok()
        );
        for (name, params) in [
            ("nav.outline", json!({"doc": 1})),
            ("nav.goto-line", json!({"doc": 1, "line": 0})),
            ("nav.goto-symbol", json!({"doc": 1, "index": 0})),
        ] {
            let e = r.execute(&mut ctx, name, params).unwrap_err();
            assert_eq!(kind(&e), ("rejected".into(), "no_such_document".into()));
        }
        r.execute(&mut ctx, "doc.open", json!({"path": "a.rs"}))
            .unwrap();
        for bad in [
            json!({"doc": 1, "line": 0, "col": null}),
            json!({"doc": 1, "line": -1}),
            json!({"doc": 1, "line": 0, "x": 0}),
        ] {
            let e = r.execute(&mut ctx, "nav.goto-line", bad).unwrap_err();
            assert_eq!(kind(&e).0, "invalid_params");
        }
        let e = r
            .execute(&mut ctx, "nav.goto-symbol", json!({"doc": 1, "index": 1}))
            .unwrap_err();
        assert_eq!(kind(&e), ("rejected".into(), "out_of_range".into()));
        let log: Vec<&str> = ctx.log().iter().map(|l| l.name.as_str()).collect();
        assert_eq!(
            log,
            vec!["nav.workspace-symbols", "doc.open"],
            "refusals are not logged"
        );
    }

    #[test]
    fn size_limit_is_exact() {
        let r = reg();
        let at = format!("//{}", "é".repeat(super::MAX_CHARS - 2));
        let mut ctx = ctx_with(&[("a.rs", at.as_bytes())]);
        r.execute(&mut ctx, "doc.open", json!({"path": "a.rs"}))
            .unwrap();
        let out = r
            .execute(&mut ctx, "nav.outline", json!({"doc": 1}))
            .unwrap();
        assert_eq!(out, json!({"items": []}));
        r.execute(
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": 0, "text": " "}),
        )
        .unwrap();
        for (name, params) in [
            ("nav.outline", json!({"doc": 1})),
            ("nav.goto-symbol", json!({"doc": 1, "index": 0})),
        ] {
            let e = r.execute(&mut ctx, name, params).unwrap_err();
            assert_eq!(kind(&e), ("rejected".into(), "too_large".into()));
        }
        let out = r
            .execute(
                &mut ctx,
                "nav.goto-line",
                json!({"doc": 1, "line": 0, "col": 5}),
            )
            .unwrap();
        assert_eq!(out["head"], json!(5));
    }

    #[test]
    fn goto_line_clamps_on_crlf_and_cr() {
        let r = reg();
        let mut ctx = ctx_with(&[("a.txt", "ab\r\ncdef\rg".as_bytes())]);
        r.execute(&mut ctx, "doc.open", json!({"path": "a.txt"}))
            .unwrap();
        let head = |ctx: &mut Context, params: Value| -> (u64, u64, u64) {
            let out = r.execute(ctx, "nav.goto-line", params).unwrap();
            (
                out["head"].as_u64().unwrap(),
                out["line"].as_u64().unwrap(),
                out["col"].as_u64().unwrap(),
            )
        };
        assert_eq!(
            head(&mut ctx, json!({"doc": 1, "line": 0, "col": 99})),
            (2, 0, 2)
        );
        assert_eq!(
            head(&mut ctx, json!({"doc": 1, "line": 1, "col": 99})),
            (8, 1, 4)
        );
        assert_eq!(head(&mut ctx, json!({"doc": 1, "line": 2})), (9, 2, 0));
        assert_eq!(
            head(
                &mut ctx,
                json!({"doc": 1, "line": u64::MAX, "col": u64::MAX})
            ),
            (10, 2, 1)
        );
    }

    #[test]
    fn nav_does_not_touch_the_syntax_cache() {
        let r = reg();
        let mut ctx = ctx_with(&[("a.rs", b"fn a() {}")]);
        r.execute(&mut ctx, "doc.open", json!({"path": "a.rs"}))
            .unwrap();
        r.execute(&mut ctx, "syntax.tree", json!({"doc": 1}))
            .unwrap();
        r.execute(
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": 9, "text": " "}),
        )
        .unwrap();
        r.execute(&mut ctx, "nav.outline", json!({"doc": 1}))
            .unwrap();
        r.execute(&mut ctx, "nav.goto-symbol", json!({"doc": 1, "index": 0}))
            .unwrap();
        r.execute(&mut ctx, "nav.workspace-symbols", json!({"query": "a"}))
            .unwrap();
        let out = r
            .execute(&mut ctx, "syntax.tree", json!({"doc": 1}))
            .unwrap();
        assert_eq!(out["mode"], json!("incremental"));
    }

    #[test]
    fn kind_names_are_as_str() {
        use kiln_syntax::outline::ItemKind::*;
        for k in [Fn, Struct, Enum, Trait, Impl, Mod] {
            assert_eq!(
                serde_json::to_value(super::kind_name(k)).unwrap(),
                json!(k.as_str())
            );
        }
    }
}
