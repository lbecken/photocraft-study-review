//! The `doc.*` command group: open, edit, undo/redo, save, close and inspect documents.
//!
//! Documents are identified by a `u64` id assigned 1, 2, 3, … per [`Context`] as they are
//! opened, never reused. Offsets are char indices. Domain failures are `rejected`; every
//! command leaves the context unchanged when it fails.

use kiln_text::{LineEnding, LineEndings};
use kiln_workspace::{Document, Encoding, RelPath};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::registry::{Command, CommandError, Context, Example, Registry, RegistryError};

/// Register the `doc.*` commands.
pub(crate) fn register(registry: &mut Registry) -> Result<(), RegistryError> {
    registry.register::<Open>()?;
    registry.register::<Save>()?;
    registry.register::<Close>()?;
    registry.register::<Insert>()?;
    registry.register::<Delete>()?;
    registry.register::<Replace>()?;
    registry.register::<Undo>()?;
    registry.register::<Redo>()?;
    registry.register::<Text>()?;
    registry.register::<Info>()?;
    registry.register::<List>()?;
    Ok(())
}

/// Dominant line ending of a document's current text.
#[derive(Clone, Copy, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum LineEndingName {
    /// The text has no line break.
    None,
    /// `\n`.
    Lf,
    /// `\r\n`.
    Crlf,
    /// A lone `\r`.
    Cr,
}

/// Encoding detected when the document was opened (used for save).
#[derive(Clone, Copy, Debug, Serialize, JsonSchema)]
pub(crate) enum EncodingName {
    /// UTF-8 without a byte-order mark.
    #[serde(rename = "utf-8")]
    Utf8,
    /// UTF-8 with a byte-order mark.
    #[serde(rename = "utf-8-bom")]
    Utf8Bom,
    /// UTF-16 little-endian with a byte-order mark.
    #[serde(rename = "utf-16le")]
    Utf16Le,
    /// UTF-16 big-endian with a byte-order mark.
    #[serde(rename = "utf-16be")]
    Utf16Be,
}

fn encoding_name(e: Encoding) -> EncodingName {
    match e {
        Encoding::Utf8 => EncodingName::Utf8,
        Encoding::Utf8Bom => EncodingName::Utf8Bom,
        Encoding::Utf16Le => EncodingName::Utf16Le,
        Encoding::Utf16Be => EncodingName::Utf16Be,
    }
}

fn line_ending_name(doc: &Document) -> LineEndingName {
    match doc.buffer().dominant_line_ending() {
        None => LineEndingName::None,
        Some(LineEnding::Lf) => LineEndingName::Lf,
        Some(LineEnding::Crlf) => LineEndingName::Crlf,
        Some(LineEnding::Cr) => LineEndingName::Cr,
    }
}

fn mixed(doc: &Document) -> bool {
    doc.buffer().line_endings() == LineEndings::Mixed
}

// ---------------------------------------------------------------------------------------------
// doc.open

/// Params of `doc.open`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct OpenParams {
    /// Root-relative path of the file (`/`-separated; no `.`, `..` or empty segments).
    path: String,
}

/// Result of `doc.open`.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct OpenResult {
    /// Id of the document.
    doc: u64,
    /// True iff the path was already open (the existing document is returned, unchanged).
    already_open: bool,
    /// The document's path.
    path: String,
    /// Encoding detected at open.
    encoding: EncodingName,
    /// Dominant line ending of the current text.
    line_ending: LineEndingName,
    /// True iff the current text has line breaks of more than one kind.
    mixed_line_endings: bool,
    /// Length of the current text in chars.
    len_chars: usize,
}

pub(crate) struct Open;

impl Command for Open {
    const NAME: &'static str = "doc.open";
    const DOC: &'static str = "Open a file as a document and return its id. Opening a path that \
        is already open returns the existing document unchanged, with `already_open: true`.";
    const MUTATING: bool = true;
    type Params = OpenParams;
    type Output = OpenResult;

    fn examples() -> Vec<Example> {
        vec![
            Example {
                description: "open a CRLF file",
                initial_text: "",
                files: vec![("a.rs", "fn a() {}\r\n")],
                setup: vec![],
                params: json!({"path": "a.rs"}),
                result: json!({"already_open": false, "doc": 1, "encoding": "utf-8",
                    "len_chars": 11, "line_ending": "crlf", "mixed_line_endings": false,
                    "path": "a.rs"}),
            },
            Example {
                description: "re-opening an open, edited path returns the existing document",
                initial_text: "",
                files: vec![("a.txt", "a\nb\r\n")],
                setup: vec![
                    ("doc.open", json!({"path": "a.txt"})),
                    ("doc.insert", json!({"doc": 1, "offset": 0, "text": "\n"})),
                ],
                params: json!({"path": "a.txt"}),
                result: json!({"already_open": true, "doc": 1, "encoding": "utf-8",
                    "len_chars": 6, "line_ending": "lf", "mixed_line_endings": true,
                    "path": "a.txt"}),
            },
        ]
    }

    fn run(ctx: &mut Context, p: OpenParams) -> Result<OpenResult, CommandError> {
        let path = RelPath::new(&p.path).map_err(|e| CommandError::rejected(e.to_string()))?;
        let existing = ctx
            .docs()
            .iter()
            .find(|(_, d)| d.path() == &path)
            .map(|(id, _)| *id);
        let (id, already_open) = match existing {
            Some(id) => (id, true),
            None => {
                let doc = Document::open(ctx.vfs(), &path)
                    .map_err(|e| CommandError::rejected(e.to_string()))?;
                (ctx.add_doc(doc), false)
            }
        };
        let doc = ctx.doc(id)?;
        Ok(OpenResult {
            doc: id,
            already_open,
            path: doc.path().to_string(),
            encoding: encoding_name(doc.encoding()),
            line_ending: line_ending_name(doc),
            mixed_line_endings: mixed(doc),
            len_chars: doc.buffer().len_chars(),
        })
    }
}

// ---------------------------------------------------------------------------------------------
// doc.save

/// Params naming one open document.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct DocParams {
    /// Id of an open document.
    doc: u64,
}

/// Result of `doc.save`.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct SaveResult {
    /// Id of the document.
    doc: u64,
    /// Number of bytes written to the file.
    bytes_written: usize,
    /// Whether the document is dirty after the save (false).
    dirty: bool,
}

pub(crate) struct Save;

impl Command for Save {
    const NAME: &'static str = "doc.save";
    const DOC: &'static str = "Write the document to its file in its encoding, keeping line \
        endings verbatim (atomic replace). Ends the current typing group.";
    const MUTATING: bool = true;
    type Params = DocParams;
    type Output = SaveResult;

    fn examples() -> Vec<Example> {
        vec![Example {
            description: "save an edit",
            initial_text: "",
            files: vec![("a.txt", "hello\n")],
            setup: vec![
                ("doc.open", json!({"path": "a.txt"})),
                (
                    "doc.insert",
                    json!({"doc": 1, "offset": 5, "text": " world"}),
                ),
            ],
            params: json!({"doc": 1}),
            result: json!({"doc": 1, "bytes_written": 12, "dirty": false}),
        }]
    }

    fn run(ctx: &mut Context, p: DocParams) -> Result<SaveResult, CommandError> {
        let (doc, vfs) = ctx.doc_and_vfs_mut(p.doc)?;
        let bytes_written = doc
            .save(vfs)
            .map_err(|e| CommandError::rejected(format!("cannot save {e}")))?;
        doc.buffer_mut().break_undo_group();
        Ok(SaveResult {
            doc: p.doc,
            bytes_written,
            dirty: doc.is_dirty(),
        })
    }
}

// ---------------------------------------------------------------------------------------------
// doc.close

/// Params of `doc.close`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CloseParams {
    /// Id of an open document.
    doc: u64,
    /// Close even if the document has unsaved changes (they are lost; the file is untouched).
    #[serde(default)]
    discard: bool,
}

/// Result of `doc.close`.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct CloseResult {
    /// Id of the closed document (never reused).
    doc: u64,
    /// Whether the document is now closed (true).
    closed: bool,
}

pub(crate) struct Close;

impl Command for Close {
    const NAME: &'static str = "doc.close";
    const DOC: &'static str = "Close a document. A dirty document is refused unless \
        `discard` is true. The id is never reused.";
    const MUTATING: bool = true;
    type Params = CloseParams;
    type Output = CloseResult;

    fn examples() -> Vec<Example> {
        vec![
            Example {
                description: "close a clean document",
                initial_text: "",
                files: vec![("a.txt", "x")],
                setup: vec![("doc.open", json!({"path": "a.txt"}))],
                params: json!({"doc": 1}),
                result: json!({"doc": 1, "closed": true}),
            },
            Example {
                description: "discard unsaved changes",
                initial_text: "",
                files: vec![("a.txt", "x")],
                setup: vec![
                    ("doc.open", json!({"path": "a.txt"})),
                    ("doc.insert", json!({"doc": 1, "offset": 1, "text": "y"})),
                ],
                params: json!({"doc": 1, "discard": true}),
                result: json!({"doc": 1, "closed": true}),
            },
        ]
    }

    fn run(ctx: &mut Context, p: CloseParams) -> Result<CloseResult, CommandError> {
        if ctx.doc(p.doc)?.is_dirty() && !p.discard {
            return Err(CommandError::rejected(format!(
                "document {} has unsaved changes; save it or close with discard",
                p.doc
            )));
        }
        ctx.remove_doc(p.doc)?;
        Ok(CloseResult {
            doc: p.doc,
            closed: !ctx.docs().contains_key(&p.doc),
        })
    }
}

// ---------------------------------------------------------------------------------------------
// doc.insert / doc.delete / doc.replace

/// Params of `doc.insert`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct InsertParams {
    /// Id of an open document.
    doc: u64,
    /// Char index before which to insert; equal to the length to append.
    offset: usize,
    /// Text to insert.
    text: String,
}

/// Result of `doc.insert`.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct InsertResult {
    /// Length in chars after the edit.
    len_chars: usize,
    /// Whether the text now differs from the file's last opened or saved text.
    dirty: bool,
}

pub(crate) struct Insert;

impl Command for Insert {
    const NAME: &'static str = "doc.insert";
    const DOC: &'static str = "Insert text at a char offset of a document. Successive one-char \
        inserts form one undo step (kiln-text typing-group rule).";
    const MUTATING: bool = true;
    type Params = InsertParams;
    type Output = InsertResult;

    fn examples() -> Vec<Example> {
        vec![Example {
            description: "insert into an opened file",
            initial_text: "",
            files: vec![("a.txt", "hello\n")],
            setup: vec![("doc.open", json!({"path": "a.txt"}))],
            params: json!({"doc": 1, "offset": 5, "text": " world"}),
            result: json!({"len_chars": 12, "dirty": true}),
        }]
    }

    fn run(ctx: &mut Context, p: InsertParams) -> Result<InsertResult, CommandError> {
        let doc = ctx.doc_mut(p.doc)?;
        doc.buffer_mut()
            .insert(p.offset, &p.text)
            .map_err(|e| CommandError::rejected(e.to_string()))?;
        Ok(InsertResult {
            len_chars: doc.buffer().len_chars(),
            dirty: doc.is_dirty(),
        })
    }
}

/// Params of `doc.delete`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeleteParams {
    /// Id of an open document.
    doc: u64,
    /// First char index to remove.
    start: usize,
    /// Char index after the last one to remove (`start <= end <= len_chars`).
    end: usize,
}

/// Result of `doc.delete` and `doc.replace`.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct RemovedResult {
    /// The text that was removed.
    removed: String,
    /// Length in chars after the edit.
    len_chars: usize,
    /// Whether the text now differs from the file's last opened or saved text.
    dirty: bool,
}

pub(crate) struct Delete;

impl Command for Delete {
    const NAME: &'static str = "doc.delete";
    const DOC: &'static str = "Delete the chars `start..end` of a document and return them.";
    const MUTATING: bool = true;
    type Params = DeleteParams;
    type Output = RemovedResult;

    fn examples() -> Vec<Example> {
        vec![Example {
            description: "delete a word",
            initial_text: "",
            files: vec![("a.txt", "hello world")],
            setup: vec![("doc.open", json!({"path": "a.txt"}))],
            params: json!({"doc": 1, "start": 5, "end": 11}),
            result: json!({"removed": " world", "len_chars": 5, "dirty": true}),
        }]
    }

    fn run(ctx: &mut Context, p: DeleteParams) -> Result<RemovedResult, CommandError> {
        let doc = ctx.doc_mut(p.doc)?;
        let removed = doc
            .buffer_mut()
            .delete(p.start..p.end)
            .map_err(|e| CommandError::rejected(e.to_string()))?;
        Ok(RemovedResult {
            removed,
            len_chars: doc.buffer().len_chars(),
            dirty: doc.is_dirty(),
        })
    }
}

/// Params of `doc.replace`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReplaceParams {
    /// Id of an open document.
    doc: u64,
    /// First char index to replace.
    start: usize,
    /// Char index after the last one to replace (`start <= end <= len_chars`).
    end: usize,
    /// Replacement text.
    text: String,
}

pub(crate) struct Replace;

impl Command for Replace {
    const NAME: &'static str = "doc.replace";
    const DOC: &'static str = "Replace the chars `start..end` of a document with text, as one \
        undo step, and return the removed text.";
    const MUTATING: bool = true;
    type Params = ReplaceParams;
    type Output = RemovedResult;

    fn examples() -> Vec<Example> {
        vec![Example {
            description: "rename an identifier",
            initial_text: "",
            files: vec![("a.rs", "fn a() {}\n")],
            setup: vec![("doc.open", json!({"path": "a.rs"}))],
            params: json!({"doc": 1, "start": 3, "end": 4, "text": "main"}),
            result: json!({"removed": "a", "len_chars": 13, "dirty": true}),
        }]
    }

    fn run(ctx: &mut Context, p: ReplaceParams) -> Result<RemovedResult, CommandError> {
        let doc = ctx.doc_mut(p.doc)?;
        let removed = doc
            .buffer_mut()
            .replace(p.start..p.end, &p.text)
            .map_err(|e| CommandError::rejected(e.to_string()))?;
        Ok(RemovedResult {
            removed,
            len_chars: doc.buffer().len_chars(),
            dirty: doc.is_dirty(),
        })
    }
}

// ---------------------------------------------------------------------------------------------
// doc.undo / doc.redo

/// Result of `doc.undo`.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct UndoResult {
    /// True iff a step was undone (false: nothing to undo, nothing changed).
    undone: bool,
    /// Length in chars afterwards.
    len_chars: usize,
    /// Whether the text now differs from the file's last opened or saved text.
    dirty: bool,
}

pub(crate) struct Undo;

impl Command for Undo {
    const NAME: &'static str = "doc.undo";
    const DOC: &'static str = "Undo the most recent undo step of a document.";
    const MUTATING: bool = true;
    type Params = DocParams;
    type Output = UndoResult;

    fn examples() -> Vec<Example> {
        vec![
            Example {
                description: "undo a typing group",
                initial_text: "",
                files: vec![("a.txt", "x")],
                setup: vec![
                    ("doc.open", json!({"path": "a.txt"})),
                    ("doc.insert", json!({"doc": 1, "offset": 0, "text": "a"})),
                    ("doc.insert", json!({"doc": 1, "offset": 1, "text": "b"})),
                ],
                params: json!({"doc": 1}),
                result: json!({"undone": true, "len_chars": 1, "dirty": false}),
            },
            Example {
                description: "nothing to undo",
                initial_text: "",
                files: vec![("a.txt", "x")],
                setup: vec![("doc.open", json!({"path": "a.txt"}))],
                params: json!({"doc": 1}),
                result: json!({"undone": false, "len_chars": 1, "dirty": false}),
            },
        ]
    }

    fn run(ctx: &mut Context, p: DocParams) -> Result<UndoResult, CommandError> {
        let doc = ctx.doc_mut(p.doc)?;
        let undone = doc.buffer_mut().undo();
        Ok(UndoResult {
            undone,
            len_chars: doc.buffer().len_chars(),
            dirty: doc.is_dirty(),
        })
    }
}

/// Result of `doc.redo`.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct RedoResult {
    /// True iff a step was redone (false: nothing to redo, nothing changed).
    redone: bool,
    /// Length in chars afterwards.
    len_chars: usize,
    /// Whether the text now differs from the file's last opened or saved text.
    dirty: bool,
}

pub(crate) struct Redo;

impl Command for Redo {
    const NAME: &'static str = "doc.redo";
    const DOC: &'static str = "Redo the most recently undone step of a document.";
    const MUTATING: bool = true;
    type Params = DocParams;
    type Output = RedoResult;

    fn examples() -> Vec<Example> {
        vec![Example {
            description: "redo an undone insert",
            initial_text: "",
            files: vec![("a.txt", "x")],
            setup: vec![
                ("doc.open", json!({"path": "a.txt"})),
                ("doc.insert", json!({"doc": 1, "offset": 1, "text": "yz"})),
                ("doc.undo", json!({"doc": 1})),
            ],
            params: json!({"doc": 1}),
            result: json!({"redone": true, "len_chars": 3, "dirty": true}),
        }]
    }

    fn run(ctx: &mut Context, p: DocParams) -> Result<RedoResult, CommandError> {
        let doc = ctx.doc_mut(p.doc)?;
        let redone = doc.buffer_mut().redo();
        Ok(RedoResult {
            redone,
            len_chars: doc.buffer().len_chars(),
            dirty: doc.is_dirty(),
        })
    }
}

// ---------------------------------------------------------------------------------------------
// doc.text / doc.info / doc.list

/// Result of `doc.text`.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct TextResult {
    /// The document's whole current text.
    text: String,
    /// Its length in chars.
    len_chars: usize,
}

pub(crate) struct Text;

impl Command for Text {
    const NAME: &'static str = "doc.text";
    const DOC: &'static str = "Return the whole current text of a document.";
    const MUTATING: bool = false;
    type Params = DocParams;
    type Output = TextResult;

    fn examples() -> Vec<Example> {
        vec![Example {
            description: "line breaks are returned verbatim",
            initial_text: "",
            files: vec![("a.txt", "a\r\nb")],
            setup: vec![("doc.open", json!({"path": "a.txt"}))],
            params: json!({"doc": 1}),
            result: json!({"text": "a\r\nb", "len_chars": 4}),
        }]
    }

    fn run(ctx: &mut Context, p: DocParams) -> Result<TextResult, CommandError> {
        let buffer = ctx.doc(p.doc)?.buffer();
        Ok(TextResult {
            text: buffer.text(),
            len_chars: buffer.len_chars(),
        })
    }
}

/// Result of `doc.info`.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct InfoResult {
    /// Id of the document.
    doc: u64,
    /// The document's path.
    path: String,
    /// Encoding detected at open (used for save).
    encoding: EncodingName,
    /// Dominant line ending of the current text.
    line_ending: LineEndingName,
    /// True iff the current text has line breaks of more than one kind.
    mixed_line_endings: bool,
    /// Length in chars.
    len_chars: usize,
    /// Number of lines (line breaks + 1).
    len_lines: usize,
    /// Whether the text differs from the file's last opened or saved text.
    dirty: bool,
    /// Whether `doc.undo` would change the text.
    can_undo: bool,
    /// Whether `doc.redo` would change the text.
    can_redo: bool,
}

pub(crate) struct Info;

impl Command for Info {
    const NAME: &'static str = "doc.info";
    const DOC: &'static str = "Return a document's path, encoding, line endings, length, dirty \
        flag and undo/redo availability.";
    const MUTATING: bool = false;
    type Params = DocParams;
    type Output = InfoResult;

    fn examples() -> Vec<Example> {
        vec![Example {
            description: "an edited document",
            initial_text: "",
            files: vec![("src/a.rs", "a\r\nb\r\n")],
            setup: vec![
                ("doc.open", json!({"path": "src/a.rs"})),
                ("doc.insert", json!({"doc": 1, "offset": 1, "text": "X"})),
            ],
            params: json!({"doc": 1}),
            result: json!({"doc": 1, "path": "src/a.rs", "encoding": "utf-8",
                "line_ending": "crlf", "mixed_line_endings": false, "len_chars": 7,
                "len_lines": 3, "dirty": true, "can_undo": true, "can_redo": false}),
        }]
    }

    fn run(ctx: &mut Context, p: DocParams) -> Result<InfoResult, CommandError> {
        let doc = ctx.doc(p.doc)?;
        let buffer = doc.buffer();
        Ok(InfoResult {
            doc: p.doc,
            path: doc.path().to_string(),
            encoding: encoding_name(doc.encoding()),
            line_ending: line_ending_name(doc),
            mixed_line_endings: mixed(doc),
            len_chars: buffer.len_chars(),
            len_lines: buffer.len_lines(),
            dirty: doc.is_dirty(),
            can_undo: buffer.can_undo(),
            can_redo: buffer.can_redo(),
        })
    }
}

/// Params of `doc.list` (none).
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ListParams {}

/// One open document in `doc.list`.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct ListItem {
    /// Id of the document.
    doc: u64,
    /// Its path.
    path: String,
    /// Whether it has unsaved changes.
    dirty: bool,
}

/// Result of `doc.list`.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct ListResult {
    /// The open documents, sorted by id.
    docs: Vec<ListItem>,
}

pub(crate) struct List;

impl Command for List {
    const NAME: &'static str = "doc.list";
    const DOC: &'static str = "List the open documents, sorted by id.";
    const MUTATING: bool = false;
    type Params = ListParams;
    type Output = ListResult;

    fn examples() -> Vec<Example> {
        vec![Example {
            description: "two documents, one edited",
            initial_text: "",
            files: vec![("a.txt", "a"), ("b.txt", "b")],
            setup: vec![
                ("doc.open", json!({"path": "a.txt"})),
                ("doc.open", json!({"path": "b.txt"})),
                ("doc.insert", json!({"doc": 2, "offset": 0, "text": "!"})),
            ],
            params: json!({}),
            result: json!({"docs": [
                {"doc": 1, "path": "a.txt", "dirty": false},
                {"doc": 2, "path": "b.txt", "dirty": true},
            ]}),
        }]
    }

    fn run(ctx: &mut Context, _: ListParams) -> Result<ListResult, CommandError> {
        Ok(ListResult {
            docs: ctx
                .docs()
                .iter()
                .map(|(id, d)| ListItem {
                    doc: *id,
                    path: d.path().to_string(),
                    dirty: d.is_dirty(),
                })
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{CommandError, Context, LogEntry, Registry, standard_registry};
    use kiln_text::Buffer;
    use kiln_workspace::RelPath;
    use serde_json::{Value, json};

    fn reg() -> Registry {
        standard_registry().unwrap()
    }

    fn ok(r: &Registry, ctx: &mut Context, name: &str, params: Value) -> Value {
        r.execute(ctx, name, params.clone())
            .unwrap_or_else(|e| panic!("{name} {params}: {e}"))
    }

    fn code(r: &Registry, ctx: &mut Context, name: &str, params: Value) -> String {
        let err = r.execute(ctx, name, params).unwrap_err();
        err.to_json()["code"].as_str().unwrap().to_owned()
    }

    fn file(ctx: &Context, path: &str) -> Vec<u8> {
        ctx.vfs().read(&RelPath::new(path).unwrap()).unwrap()
    }

    /// Everything observable about the documents and files of a context.
    fn snapshot(r: &Registry, ctx: &mut Context, paths: &[&str]) -> Value {
        let list = ok(r, ctx, "doc.list", json!({}));
        let mut docs = Vec::new();
        for d in list["docs"].as_array().unwrap() {
            let id = d["doc"].clone();
            docs.push(json!({
                "text": ok(r, ctx, "doc.text", json!({"doc": id})),
                "info": ok(r, ctx, "doc.info", json!({"doc": id})),
            }));
        }
        let files: Vec<Value> = paths
            .iter()
            .map(|p| {
                ctx.vfs()
                    .read(&RelPath::new(p).unwrap())
                    .map_or(Value::Null, |b| json!(b))
            })
            .collect();
        json!({"list": list, "docs": docs, "files": files})
    }

    #[test]
    fn open_edit_save_slice() {
        let r = reg();
        let mut ctx = Context::with_files(&[("a.txt", b"hello\n")]).unwrap();
        assert_eq!(
            ok(&r, &mut ctx, "doc.open", json!({"path": "a.txt"}))["doc"],
            1
        );
        assert_eq!(
            ok(
                &r,
                &mut ctx,
                "doc.insert",
                json!({"doc": 1, "offset": 5, "text": " world"})
            ),
            json!({"len_chars": 12, "dirty": true})
        );
        assert_eq!(
            ok(&r, &mut ctx, "doc.save", json!({"doc": 1})),
            json!({"doc": 1, "bytes_written": 12, "dirty": false})
        );
        assert_eq!(file(&ctx, "a.txt"), b"hello world\n");
        assert_eq!(
            ok(&r, &mut ctx, "doc.info", json!({"doc": 1}))["dirty"],
            false
        );
    }

    /// Tiny deterministic generator for the differential test (test code only).
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self, n: usize) -> usize {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((self.0 >> 33) % n as u64) as usize
        }
    }

    #[test]
    fn edits_match_buffer_model() {
        let r = reg();
        let mut ctx = Context::with_files(&[("m.txt", "ab\r\ncé😀\n".as_bytes())]).unwrap();
        ok(&r, &mut ctx, "doc.open", json!({"path": "m.txt"}));
        let mut model = Buffer::from_text("ab\r\ncé😀\n");
        let mut rng = Lcg(7);
        let pieces = ["x", "y", "\n", "\r\n", "é", "😀z", "", "abc"];
        let mut rejected = 0;
        for _ in 0..800 {
            let len = model.len_chars();
            let a = rng.next(len + 3);
            let b = rng.next(len + 3);
            let text = pieces[rng.next(pieces.len())];
            let before = snapshot(&r, &mut ctx, &[]);
            let (name, params, expect) = match rng.next(5) {
                0 => (
                    "doc.insert",
                    json!({"doc": 1, "offset": a, "text": text}),
                    model
                        .insert(a, text)
                        .map(|()| json!({"len_chars": model.len_chars()})),
                ),
                1 => (
                    "doc.delete",
                    json!({"doc": 1, "start": a, "end": b}),
                    model
                        .delete(a..b)
                        .map(|s| json!({"removed": s, "len_chars": model.len_chars()})),
                ),
                2 => (
                    "doc.replace",
                    json!({"doc": 1, "start": a, "end": b, "text": text}),
                    model
                        .replace(a..b, text)
                        .map(|s| json!({"removed": s, "len_chars": model.len_chars()})),
                ),
                3 => {
                    let u = model.undo();
                    (
                        "doc.undo",
                        json!({"doc": 1}),
                        Ok(json!({"undone": u, "len_chars": model.len_chars()})),
                    )
                }
                _ => {
                    let u = model.redo();
                    (
                        "doc.redo",
                        json!({"doc": 1}),
                        Ok(json!({"redone": u, "len_chars": model.len_chars()})),
                    )
                }
            };
            let got = r.execute(&mut ctx, name, params.clone());
            match expect {
                Ok(mut want) => {
                    let got = got.unwrap();
                    let dirty = got["dirty"].clone();
                    want["dirty"] = json!(model.text() != "ab\r\ncé😀\n");
                    assert_eq!(dirty, want["dirty"]);
                    assert_eq!(got, want, "{name} {params}");
                }
                Err(_) => {
                    rejected += 1;
                    assert_eq!(got.unwrap_err().to_json()["code"], "rejected");
                    assert_eq!(snapshot(&r, &mut ctx, &[]), before);
                }
            }
            let t = ok(&r, &mut ctx, "doc.text", json!({"doc": 1}));
            assert_eq!(
                t,
                json!({"text": model.text(), "len_chars": model.len_chars()})
            );
            let info = ok(&r, &mut ctx, "doc.info", json!({"doc": 1}));
            assert_eq!(info["can_undo"], model.can_undo());
            assert_eq!(info["can_redo"], model.can_redo());
            assert_eq!(info["len_lines"], model.len_lines());
        }
        assert!(rejected > 20, "too few rejected edits: {rejected}");
    }

    #[test]
    fn documents_are_independent_and_ids_never_reused() {
        let r = reg();
        let mut ctx = Context::with_files(&[("a.txt", b"a"), ("b.txt", b"b")]).unwrap();
        ok(&r, &mut ctx, "doc.open", json!({"path": "a.txt"}));
        assert_eq!(
            ok(&r, &mut ctx, "doc.open", json!({"path": "b.txt"}))["doc"],
            2
        );
        ok(
            &r,
            &mut ctx,
            "doc.insert",
            json!({"doc": 2, "offset": 1, "text": "!"}),
        );
        let b_before = ok(&r, &mut ctx, "doc.info", json!({"doc": 2}));
        ok(
            &r,
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": 0, "text": "x"}),
        );
        ok(&r, &mut ctx, "doc.undo", json!({"doc": 1}));
        ok(&r, &mut ctx, "doc.redo", json!({"doc": 1}));
        ok(&r, &mut ctx, "doc.save", json!({"doc": 1}));
        assert_eq!(ok(&r, &mut ctx, "doc.info", json!({"doc": 2})), b_before);
        assert_eq!(file(&ctx, "b.txt"), b"b");
        assert_eq!(file(&ctx, "a.txt"), b"xa");
        assert_eq!(
            ok(&r, &mut ctx, "doc.list", json!({})),
            json!({"docs": [{"doc": 1, "path": "a.txt", "dirty": false},
                            {"doc": 2, "path": "b.txt", "dirty": true}]})
        );
        assert_eq!(
            ok(&r, &mut ctx, "doc.close", json!({"doc": 1})),
            json!({"doc": 1, "closed": true})
        );
        assert_eq!(
            ok(&r, &mut ctx, "doc.list", json!({})),
            json!({"docs": [{"doc": 2, "path": "b.txt", "dirty": true}]})
        );
        for (name, params) in [
            ("doc.text", json!({"doc": 1})),
            ("doc.info", json!({"doc": 1})),
            ("doc.save", json!({"doc": 1})),
            ("doc.undo", json!({"doc": 1})),
            ("doc.close", json!({"doc": 1, "discard": true})),
            ("doc.insert", json!({"doc": 1, "offset": 0, "text": "x"})),
        ] {
            assert_eq!(code(&r, &mut ctx, name, params), "rejected", "{name}");
        }
        let re = ok(&r, &mut ctx, "doc.open", json!({"path": "a.txt"}));
        assert_eq!(
            (re["doc"].clone(), re["already_open"].clone()),
            (json!(3), json!(false))
        );
        assert_eq!(
            ok(&r, &mut ctx, "doc.info", json!({"doc": 3}))["can_undo"],
            false
        );
    }

    #[test]
    fn dirty_follows_saved_text() {
        let r = reg();
        let mut ctx = Context::with_files(&[("a.txt", b"q")]).unwrap();
        let dirty = |ctx: &mut Context| ok(&r, ctx, "doc.info", json!({"doc": 1}))["dirty"].clone();
        ok(&r, &mut ctx, "doc.open", json!({"path": "a.txt"}));
        assert_eq!(dirty(&mut ctx), false);
        ok(
            &r,
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": 1, "text": "\n"}),
        );
        assert_eq!(dirty(&mut ctx), true);
        ok(&r, &mut ctx, "doc.save", json!({"doc": 1}));
        assert_eq!(dirty(&mut ctx), false);
        ok(
            &r,
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": 0, "text": "\n"}),
        );
        assert_eq!(
            ok(&r, &mut ctx, "doc.undo", json!({"doc": 1}))["dirty"],
            false
        );
        let u = ok(&r, &mut ctx, "doc.undo", json!({"doc": 1}));
        assert_eq!(u, json!({"undone": true, "len_chars": 1, "dirty": true}));
    }

    #[test]
    fn line_endings_detected_and_preserved() {
        let r = reg();
        let files: [(&str, &[u8], &str, bool); 5] = [
            ("lf.txt", b"a\nb\n", "lf", false),
            ("crlf.txt", b"a\r\nb\r\n", "crlf", false),
            ("cr.txt", b"a\rb", "cr", false),
            ("mix.txt", b"a\r\nb\r\nc\n", "crlf", true),
            ("none.txt", b"abc", "none", false),
        ];
        let init: Vec<(&str, &[u8])> = files.iter().map(|f| (f.0, f.1)).collect();
        let mut ctx = Context::with_files(&init).unwrap();
        for (i, (path, bytes, le, mixed)) in files.iter().enumerate() {
            let o = ok(&r, &mut ctx, "doc.open", json!({"path": path}));
            assert_eq!(o["doc"], i + 1);
            assert_eq!(o["line_ending"], *le, "{path}");
            assert_eq!(o["mixed_line_endings"], *mixed, "{path}");
            ok(&r, &mut ctx, "doc.save", json!({"doc": i + 1}));
            assert_eq!(file(&ctx, path), *bytes);
        }
        ok(
            &r,
            &mut ctx,
            "doc.insert",
            json!({"doc": 2, "offset": 1, "text": "X"}),
        );
        ok(&r, &mut ctx, "doc.save", json!({"doc": 2}));
        assert_eq!(file(&ctx, "crlf.txt"), b"aX\r\nb\r\n");
        ok(
            &r,
            &mut ctx,
            "doc.insert",
            json!({"doc": 4, "offset": 1, "text": "X"}),
        );
        ok(&r, &mut ctx, "doc.save", json!({"doc": 4}));
        assert_eq!(file(&ctx, "mix.txt"), b"aX\r\nb\r\nc\n");
        ok(
            &r,
            &mut ctx,
            "doc.insert",
            json!({"doc": 2, "offset": 0, "text": "\n"}),
        );
        ok(&r, &mut ctx, "doc.save", json!({"doc": 2}));
        assert_eq!(file(&ctx, "crlf.txt"), b"\naX\r\nb\r\n");
        let info = ok(&r, &mut ctx, "doc.info", json!({"doc": 2}));
        assert_eq!(
            (
                info["line_ending"].clone(),
                info["mixed_line_endings"].clone()
            ),
            (json!("crlf"), json!(true))
        );
    }

    #[test]
    fn errors_change_nothing() {
        let r = reg();
        let mut ctx =
            Context::with_files(&[("a.txt", b"abc"), ("bad.txt", b"\xff"), ("d/x", b"")]).unwrap();
        ok(&r, &mut ctx, "doc.open", json!({"path": "a.txt"}));
        ok(
            &r,
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": 3, "text": "d"}),
        );
        let before = snapshot(&r, &mut ctx, &["a.txt"]);
        let log_len = ctx.log().len();
        for path in ["missing.txt", "bad.txt", "../x", "d", "", "/a"] {
            assert_eq!(
                code(&r, &mut ctx, "doc.open", json!({"path": path})),
                "rejected",
                "{path}"
            );
        }
        for (name, params) in [
            ("doc.insert", json!({"doc": 1, "offset": 0})),
            ("doc.insert", json!({"doc": "1", "offset": 0, "text": "x"})),
            (
                "doc.insert",
                json!({"doc": 1, "offset": 0, "text": "x", "extra": 1}),
            ),
            ("doc.close", json!({"doc": 1, "discard": "yes"})),
            ("doc.list", json!({"all": true})),
            ("doc.open", json!({})),
            ("doc.delete", json!({"doc": 1, "start": -1, "end": 0})),
        ] {
            assert_eq!(
                code(&r, &mut ctx, name, params.clone()),
                "invalid_params",
                "{params}"
            );
        }
        for (name, params) in [
            ("doc.insert", json!({"doc": 1, "offset": 5, "text": "x"})),
            ("doc.delete", json!({"doc": 1, "start": 2, "end": 1})),
            (
                "doc.replace",
                json!({"doc": 1, "start": 0, "end": 9, "text": ""}),
            ),
            ("doc.close", json!({"doc": 1})),
            ("doc.close", json!({"doc": 1, "discard": false})),
            ("doc.save", json!({"doc": 7})),
        ] {
            assert_eq!(
                code(&r, &mut ctx, name, params.clone()),
                "rejected",
                "{params}"
            );
        }
        assert_eq!(ctx.log().len(), log_len);
        assert_eq!(snapshot(&r, &mut ctx, &["a.txt"]), before);
        let again = ok(&r, &mut ctx, "doc.open", json!({"path": "a.txt"}));
        assert_eq!(
            again,
            json!({"doc": 1, "already_open": true, "path": "a.txt", "encoding": "utf-8",
                   "line_ending": "none", "mixed_line_endings": false, "len_chars": 4})
        );
        assert_eq!(snapshot(&r, &mut ctx, &["a.txt"]), before);
        assert_eq!(
            ok(
                &r,
                &mut ctx,
                "doc.close",
                json!({"doc": 1, "discard": true})
            ),
            json!({"doc": 1, "closed": true})
        );
        assert_eq!(file(&ctx, "a.txt"), b"abc");
        assert_eq!(ok(&r, &mut ctx, "doc.list", json!({})), json!({"docs": []}));
    }

    #[test]
    fn failed_save_is_rejected_and_changes_nothing() {
        let mut fs = kiln_workspace::MemoryFs::new();
        fs.insert(&RelPath::new("a").unwrap(), b"x");
        fs.set_fail_writes(true);
        let r = reg();
        let mut ctx = Context::with_vfs(Box::new(fs));
        ok(&r, &mut ctx, "doc.open", json!({"path": "a"}));
        ok(
            &r,
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": 1, "text": "y"}),
        );
        let before = snapshot(&r, &mut ctx, &["a"]);
        let n = ctx.log().len();
        assert_eq!(
            code(&r, &mut ctx, "doc.save", json!({"doc": 1})),
            "rejected"
        );
        assert_eq!(ctx.log().len(), n);
        assert_eq!(snapshot(&r, &mut ctx, &["a"]), before);
        // The failed save did not break the typing group: one undo removes "y" and "z".
        ok(
            &r,
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": 2, "text": "z"}),
        );
        ok(&r, &mut ctx, "doc.undo", json!({"doc": 1}));
        assert_eq!(ok(&r, &mut ctx, "doc.text", json!({"doc": 1}))["text"], "x");
    }

    #[test]
    fn typing_groups_and_save_breaks_group() {
        let r = reg();
        for save_between in [false, true] {
            let mut ctx = Context::with_files(&[("a.txt", b"Z")]).unwrap();
            ok(&r, &mut ctx, "doc.open", json!({"path": "a.txt"}));
            for (i, c) in ["a", "b", "c"].iter().enumerate() {
                if save_between && i == 2 {
                    ok(&r, &mut ctx, "doc.save", json!({"doc": 1}));
                }
                ok(
                    &r,
                    &mut ctx,
                    "doc.insert",
                    json!({"doc": 1, "offset": i, "text": c}),
                );
            }
            ok(&r, &mut ctx, "doc.undo", json!({"doc": 1}));
            let want = if save_between { "abZ" } else { "Z" };
            assert_eq!(
                ok(&r, &mut ctx, "doc.text", json!({"doc": 1}))["text"],
                want
            );
        }
    }

    #[test]
    fn log_records_successes_and_replay_reproduces_state() {
        let r = reg();
        let files: [(&str, &[u8]); 3] =
            [("a.txt", b"one\r\n"), ("b.txt", b"two\n"), ("bad", b"\xfe")];
        let paths = ["a.txt", "b.txt", "bad"];
        let mut rng = Lcg(42);
        for _round in 0..20 {
            let mut ctx = Context::with_files(&files).unwrap();
            let mut expected_log = Vec::new();
            let mut results = Vec::new();
            for _ in 0..60 {
                let doc = rng.next(4) as u64;
                let n = rng.next(6);
                let (name, params) = match rng.next(12) {
                    0 => ("doc.open", json!({"path": paths[rng.next(3)]})),
                    1 => ("doc.save", json!({"doc": doc})),
                    2 => (
                        "doc.close",
                        json!({"doc": doc, "discard": rng.next(2) == 0}),
                    ),
                    3 => ("doc.close", json!({"doc": doc})),
                    4 => ("doc.insert", json!({"doc": doc, "offset": n, "text": "k"})),
                    5 => ("doc.delete", json!({"doc": doc, "start": n / 2, "end": n})),
                    6 => (
                        "doc.replace",
                        json!({"doc": doc, "start": 0, "end": n, "text": "\n"}),
                    ),
                    7 => ("doc.undo", json!({"doc": doc})),
                    8 => ("doc.redo", json!({"doc": doc})),
                    9 => ("doc.text", json!({"doc": doc})),
                    10 => ("doc.list", json!({})),
                    _ => ("buffer.insert", json!({"offset": n, "text": "s"})),
                };
                if let Ok(v) = r.execute(&mut ctx, name, params.clone()) {
                    let mutating = !matches!(name, "doc.text" | "doc.list");
                    expected_log.push(LogEntry {
                        name: name.into(),
                        params,
                        mutating,
                    });
                    results.push(v);
                }
            }
            assert_eq!(ctx.log(), expected_log.as_slice());
            let log = ctx.log().to_vec();
            let mut fresh = Context::with_files(&files).unwrap();
            assert_eq!(r.replay(&mut fresh, &log).unwrap(), results);
            assert_eq!(fresh.log(), log.as_slice());
            assert_eq!(fresh.buffer().text(), ctx.buffer().text());
            let mut third = Context::with_files(&files).unwrap();
            r.replay(&mut third, fresh.log()).unwrap();
            assert_eq!(third.log(), log.as_slice());
            let s1 = snapshot(&r, &mut ctx, &paths);
            assert_eq!(snapshot(&r, &mut fresh, &paths), s1);
            assert_eq!(snapshot(&r, &mut third, &paths), s1);
        }
    }

    #[test]
    fn replay_stops_at_first_failure() {
        let r = reg();
        let log = vec![
            LogEntry {
                name: "doc.open".into(),
                params: json!({"path": "a"}),
                mutating: true,
            },
            LogEntry {
                name: "doc.text".into(),
                params: json!({"doc": 9}),
                mutating: false,
            },
            LogEntry {
                name: "doc.open".into(),
                params: json!({"path": "b"}),
                mutating: true,
            },
        ];
        let mut ctx = Context::with_files(&[("a", b"x"), ("b", b"y")]).unwrap();
        let err = r.replay(&mut ctx, &log).unwrap_err();
        assert_eq!(err.index, 1);
        assert!(matches!(err.error, CommandError::Rejected { .. }));
        assert_eq!(
            err.to_string(),
            "replay failed at log entry 1: no open document 9"
        );
        // No rollback: the entry before the failure stays applied, the one after never ran.
        assert_eq!(ctx.log(), &log[..1]);
        assert_eq!(
            ok(&r, &mut ctx, "doc.list", json!({})),
            json!({"docs": [{"doc": 1, "path": "a", "dirty": false}]})
        );
    }

    #[test]
    fn describe_lists_doc_commands_with_mutating_flag() {
        let d = reg().describe();
        let cmds = d["commands"].as_array().unwrap();
        let doc: Vec<&Value> = cmds
            .iter()
            .filter(|c| c["name"].as_str().unwrap().starts_with("doc."))
            .collect();
        assert_eq!(doc.len(), 11);
        for c in cmds {
            let name = c["name"].as_str().unwrap();
            let read_only = ["doc.text", "doc.info", "doc.list", "buffer.text"].contains(&name);
            assert_eq!(c["mutating"], !read_only, "{name}");
            for ex in c["examples"].as_array().unwrap() {
                assert!(ex["files"].is_array() && ex["setup"].is_array(), "{name}");
            }
        }
        assert_eq!(reg().audit(), Vec::<String>::new());
    }
}
