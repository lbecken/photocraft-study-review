//! T-032 acceptance tests through the declared Rust API of `kiln-commands`: `RejectReason`,
//! `InternalReason`, `CommandError::{Rejected, Internal}` with a `reason`, the reason-taking
//! `CommandError::rejected` and the `From` mappings of the lower-layer errors (AC-1, AC-5 and the
//! reason half of AC-2/AC-3). Written from the task specification alone by the test author.
//!
//! On base this file does not compile (`RejectReason` does not exist): weak evidence by design.
//! AC-1 also fails on base by assertion in `acceptance_t032_json.rs`.
#![cfg(test)]

use std::collections::BTreeSet;
use std::io::ErrorKind;

use kiln_commands::{
    Command, CommandError, Context, Example, InternalReason, Registry, RejectReason,
    standard_registry,
};
use kiln_text::{Buffer, EditError};
use kiln_workspace::{
    DecodeError, Document, Encoding, MemoryFs, OpenError, RelPath, Tool, ToolError, VfsError,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::{Value, json};

fn reg() -> Registry {
    standard_registry().expect("standard registry builds")
}

fn ok(r: &Registry, ctx: &mut Context, name: &str, params: Value) -> Value {
    r.execute(ctx, name, params.clone())
        .unwrap_or_else(|e| panic!("{name} {params} failed: {e:?}"))
}

fn files_ctx(files: &[(&str, &[u8])], fail_writes: bool) -> Context {
    let mut fs = MemoryFs::new();
    for (p, b) in files {
        fs.insert(&RelPath::new(p).expect("valid fixture path"), b);
    }
    fs.set_fail_writes(fail_writes);
    Context::with_vfs(Box::new(fs))
}

/// One refusal: (what, files, fail writes, scratch text, setup, command, params, reason, message).
type Case = (
    &'static str,
    &'static [(&'static str, &'static [u8])],
    bool,
    &'static str,
    Vec<(&'static str, Value)>,
    &'static str,
    Value,
    &'static str,
    &'static str,
);

const ABC: &[(&str, &[u8])] = &[("a.txt", b"abc")];

/// Stands for the message of a refusal that opens a directory: not pinned, because T-008 rewords
/// it (ruling r6, D-052); its error class, code and reason are still checked.
const UNPINNED: &str = "<directory refusal: message not pinned (D-052)>";

/// The AC-1 list, then further refusal sites and hostile names whose messages look like another
/// kind's message (the reason must come from where the refusal is raised, not from text).
fn cases() -> Vec<Case> {
    let open_a = || ("doc.open", json!({"path": "a.txt"}));
    let dirty = || {
        vec![
            open_a(),
            ("doc.insert", json!({"doc": 1, "offset": 0, "text": "x"})),
        ]
    };
    vec![
        // AC-1, in its order.
        (
            "doc.open ../x",
            &[],
            false,
            "",
            vec![],
            "doc.open",
            json!({"path": "../x"}),
            "invalid_path",
            "invalid relative path \"../x\": path has a '..' segment",
        ),
        (
            "doc.text 9",
            &[],
            false,
            "",
            vec![],
            "doc.text",
            json!({"doc": 9}),
            "no_such_document",
            "no open document 9",
        ),
        (
            "dirty doc.close",
            ABC,
            false,
            "",
            dirty(),
            "doc.close",
            json!({"doc": 1}),
            "unsaved_changes",
            "document 1 has unsaved changes; save it or close with discard",
        ),
        (
            "doc.open missing",
            ABC,
            false,
            "",
            vec![],
            "doc.open",
            json!({"path": "missing.txt"}),
            "not_found",
            "cannot open missing.txt: not found",
        ),
        (
            "doc.open directory",
            &[("d/x", b"x")],
            false,
            "",
            vec![],
            "doc.open",
            json!({"path": "d"}),
            "not_a_file",
            UNPINNED,
        ),
        (
            "doc.open \\xff",
            &[("bad.bin", b"\xff")],
            false,
            "",
            vec![],
            "doc.open",
            json!({"path": "bad.bin"}),
            "decode_error",
            "cannot open bad.bin: invalid utf-8 at byte offset 0",
        ),
        (
            "doc.insert at 99",
            ABC,
            false,
            "",
            vec![open_a()],
            "doc.insert",
            json!({"doc": 1, "offset": 99, "text": "x"}),
            "out_of_range",
            "char index 99 is out of bounds (buffer has 3 chars)",
        ),
        (
            "doc.delete 2..1",
            ABC,
            false,
            "",
            vec![open_a()],
            "doc.delete",
            json!({"doc": 1, "start": 2, "end": 1}),
            "inverted_range",
            "range start 2 is after range end 1",
        ),
        (
            "buffer.insert at 99",
            &[],
            false,
            "abc",
            vec![],
            "buffer.insert",
            json!({"offset": 99, "text": "!"}),
            "out_of_range",
            "char index 99 is out of bounds (buffer has 3 chars)",
        ),
        (
            "doc.save, writes fail",
            ABC,
            true,
            "",
            dirty(),
            "doc.save",
            json!({"doc": 1}),
            "io_error",
            "cannot save a.txt: I/O error: simulated write failure",
        ),
        // Further sites of the same reasons.
        (
            "doc.open /abs.txt",
            &[],
            false,
            "",
            vec![],
            "doc.open",
            json!({"path": "/abs.txt"}),
            "invalid_path",
            "invalid relative path \"/abs.txt\": path starts with '/'",
        ),
        (
            "doc.save 4",
            ABC,
            false,
            "",
            vec![open_a()],
            "doc.save",
            json!({"doc": 4}),
            "no_such_document",
            "no open document 4",
        ),
        (
            "doc.close 2",
            &[],
            false,
            "",
            vec![],
            "doc.close",
            json!({"doc": 2}),
            "no_such_document",
            "no open document 2",
        ),
        (
            "doc.info 3",
            ABC,
            false,
            "",
            vec![open_a()],
            "doc.info",
            json!({"doc": 3}),
            "no_such_document",
            "no open document 3",
        ),
        (
            "doc.undo 5",
            &[],
            false,
            "",
            vec![],
            "doc.undo",
            json!({"doc": 5}),
            "no_such_document",
            "no open document 5",
        ),
        (
            "doc.insert 9 at 99 (document checked first)",
            ABC,
            false,
            "",
            vec![open_a()],
            "doc.insert",
            json!({"doc": 9, "offset": 99, "text": "x"}),
            "no_such_document",
            "no open document 9",
        ),
        (
            "doc.replace 1..9",
            ABC,
            false,
            "",
            vec![open_a()],
            "doc.replace",
            json!({"doc": 1, "start": 1, "end": 9, "text": "q"}),
            "out_of_range",
            "char index 9 is out of bounds (buffer has 3 chars)",
        ),
        (
            "doc.replace 3..1",
            ABC,
            false,
            "",
            vec![open_a()],
            "doc.replace",
            json!({"doc": 1, "start": 3, "end": 1, "text": "q"}),
            "inverted_range",
            "range start 3 is after range end 1",
        ),
        // Hostile names: each message reads like another kind's.
        (
            "missing file named like a refusal",
            ABC,
            false,
            "",
            vec![],
            "doc.open",
            json!({"path": "no open document 1"}),
            "not_found",
            "cannot open no open document 1: not found",
        ),
        (
            "missing file whose name ends like a directory refusal",
            ABC,
            false,
            "",
            vec![],
            "doc.open",
            json!({"path": "x: is a directory, not a file"}),
            "not_found",
            "cannot open x: is a directory, not a file: not found",
        ),
        (
            "directory named like an I/O error",
            &[("e: I/O error: z/f", b"x")],
            false,
            "",
            vec![],
            "doc.open",
            json!({"path": "e: I/O error: z"}),
            "not_a_file",
            UNPINNED,
        ),
        (
            "undecodable file named like a missing one",
            &[("m: not found", b"ok\xff")],
            false,
            "",
            vec![],
            "doc.open",
            json!({"path": "m: not found"}),
            "decode_error",
            "cannot open m: not found: invalid utf-8 at byte offset 2",
        ),
    ]
}

fn run_case(r: &Registry, case: &Case) -> CommandError {
    let (what, files, fail, text, setup, name, params, _, _) = case;
    let mut ctx = files_ctx(files, *fail);
    if !text.is_empty() {
        ok(
            r,
            &mut ctx,
            "buffer.insert",
            json!({"offset": 0, "text": text}),
        );
    }
    for (n, p) in setup {
        ok(r, &mut ctx, n, p.clone());
    }
    r.execute(&mut ctx, name, params.clone()).expect_err(what)
}

/// The expected reason of a case, named variant by variant (not through `as_str`).
fn expected(kind: &str) -> RejectReason {
    match kind {
        "invalid_path" => RejectReason::InvalidPath,
        "no_such_document" => RejectReason::NoSuchDocument,
        "unsaved_changes" => RejectReason::UnsavedChanges,
        "not_found" => RejectReason::NotFound,
        "not_a_file" => RejectReason::NotAFile,
        "io_error" => RejectReason::IoError,
        "decode_error" => RejectReason::DecodeError,
        "out_of_range" => RejectReason::OutOfRange,
        "inverted_range" => RejectReason::InvertedRange,
        other => panic!("no case expects {other}"),
    }
}

/// AC-1: each refusal is `Rejected` with exactly its reason and its unchanged message, and
/// `to_json()["reason"] == reason.as_str()`.
#[test]
fn refusals_carry_reasons_at_their_sources() {
    let r = reg();
    let mut seen = BTreeSet::new();
    for case in cases() {
        let (what, .., kind, message) = case;
        let err = run_case(&r, &case);
        let want = expected(kind);
        match &err {
            CommandError::Rejected { reason, .. } => assert_eq!(*reason, want, "{what}"),
            other => panic!("{what}: expected a refusal, got {other:?}"),
        }
        assert_eq!(err.to_json()["code"], "rejected", "{what}");
        assert_eq!(err.to_json()["reason"], want.as_str(), "{what}");
        seen.insert(kind);
        if message == UNPINNED {
            continue;
        }
        assert_eq!(
            err,
            CommandError::Rejected {
                reason: want,
                message: message.to_owned(),
            },
            "{what}"
        );
        assert_eq!(err, CommandError::rejected(want, message), "{what}");
        assert_eq!(err.to_json()["reason"], want.as_str(), "{what}");
        assert_eq!(
            err.to_json(),
            json!({"code": "rejected", "message": message, "reason": kind}),
            "{what}"
        );
    }
    assert_eq!(
        seen.len(),
        9,
        "every iteration-0 refusal reason but `other` is raised"
    );
}

/// `CommandError::rejected` keeps its reason and message as given (also for reasons no
/// command raises yet), and `to_json`/`Display` follow them.
#[test]
fn rejected_constructor_keeps_reason_and_message() {
    for &reason in RejectReason::ALL {
        for message in ["zz unrelated", "no open document 1", ""] {
            let e = CommandError::rejected(reason, message);
            assert_eq!(
                e,
                CommandError::Rejected {
                    reason,
                    message: message.to_owned(),
                }
            );
            assert_eq!(
                e.to_json(),
                json!({"code": "rejected", "message": message, "reason": reason.as_str()})
            );
            assert_eq!(e.to_string(), message);
        }
    }
    let owned: String = "owned message".to_owned();
    assert_eq!(
        CommandError::rejected(RejectReason::ToolTimeout, owned).to_json()["reason"],
        "tool_timeout"
    );
    for &reason in InternalReason::ALL {
        let e = CommandError::Internal {
            reason,
            message: "the command panicked".to_owned(),
        };
        assert_eq!(
            e.to_json(),
            json!({"code": "internal", "message": "the command panicked", "reason": reason.as_str()})
        );
        assert_eq!(e.to_string(), "the command panicked");
    }
}

// --------------------------------------------------------------------------------------------
// Internal errors raised by the registry, and reasons passing through it.

/// No params.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct NoParams {}

/// Declared as an integer, serialized as a string: violates its own result schema.
#[derive(Debug, JsonSchema)]
struct Lying(u64);

impl Serialize for Lying {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("not a number: {}", self.0))
    }
}

/// Refuses to serialize.
#[derive(Debug, JsonSchema)]
struct Unserializable(u64);

impl Serialize for Unserializable {
    fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom(format!("boom {}", self.0)))
    }
}

fn example() -> Vec<Example> {
    vec![Example {
        description: "never run",
        initial_text: "",
        files: vec![],
        setup: vec![],
        params: json!({}),
        result: json!(0),
    }]
}

struct SchemaViolation;

impl Command for SchemaViolation {
    const NAME: &'static str = "t.schema";
    const DOC: &'static str = "Returns a result that violates its schema.";
    const MUTATING: bool = false;
    type Params = NoParams;
    type Output = Lying;
    fn examples() -> Vec<Example> {
        example()
    }
    fn run(_: &mut Context, _: NoParams) -> Result<Lying, CommandError> {
        Ok(Lying(7))
    }
}

struct NoSerialize;

impl Command for NoSerialize {
    const NAME: &'static str = "t.noser";
    const DOC: &'static str = "Returns a result that does not serialize.";
    const MUTATING: bool = false;
    type Params = NoParams;
    type Output = Unserializable;
    fn examples() -> Vec<Example> {
        example()
    }
    fn run(_: &mut Context, _: NoParams) -> Result<Unserializable, CommandError> {
        Ok(Unserializable(7))
    }
}

/// Params naming a reason by index into `ALL`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PickParams {
    /// Index into `RejectReason::ALL` or `InternalReason::ALL`.
    i: usize,
}

struct RefuseWith;

impl Command for RefuseWith {
    const NAME: &'static str = "t.refuse";
    const DOC: &'static str = "Refuses with the reason it is given.";
    const MUTATING: bool = false;
    type Params = PickParams;
    type Output = u64;
    fn examples() -> Vec<Example> {
        example()
    }
    fn run(_: &mut Context, p: PickParams) -> Result<u64, CommandError> {
        Err(CommandError::rejected(
            RejectReason::ALL[p.i],
            "cannot open a: not found",
        ))
    }
}

struct FailWith;

impl Command for FailWith {
    const NAME: &'static str = "t.fail";
    const DOC: &'static str = "Fails internally with the reason it is given.";
    const MUTATING: bool = false;
    type Params = PickParams;
    type Output = u64;
    fn examples() -> Vec<Example> {
        example()
    }
    fn run(_: &mut Context, p: PickParams) -> Result<u64, CommandError> {
        Err(CommandError::Internal {
            reason: InternalReason::ALL[p.i],
            message: "result of `t.fail` violates its schema: x".to_owned(),
        })
    }
}

fn custom_registry() -> Registry {
    let mut r = Registry::new();
    r.register::<SchemaViolation>().expect("register t.schema");
    r.register::<NoSerialize>().expect("register t.noser");
    r.register::<RefuseWith>().expect("register t.refuse");
    r.register::<FailWith>().expect("register t.fail");
    r
}

/// A result-schema violation is `Internal{ResultSchema}`, a result that does not serialize is
/// `Internal{Other}` (messages unchanged), and the registry passes a command's own reason
/// through untouched, whatever the message says.
#[test]
fn internal_errors_carry_reasons() {
    let r = custom_registry();
    let mut ctx = Context::new();
    match r.execute(&mut ctx, "t.schema", json!({})) {
        Err(CommandError::Internal { reason, message }) => {
            assert_eq!(reason, InternalReason::ResultSchema);
            assert_eq!(
                message,
                "result of `t.schema` violates its schema: \"not a number: 7\" is not of type \
                 \"integer\""
            );
        }
        other => panic!("t.schema: {other:?}"),
    }
    assert_eq!(
        r.execute(&mut ctx, "t.noser", json!({})),
        Err(CommandError::Internal {
            reason: InternalReason::Other,
            message: "result of `t.noser` does not serialize: boom 7".to_owned(),
        })
    );
    for (i, &reason) in RejectReason::ALL.iter().enumerate() {
        assert_eq!(
            r.execute(&mut ctx, "t.refuse", json!({"i": i})),
            Err(CommandError::rejected(reason, "cannot open a: not found")),
            "{reason:?}"
        );
    }
    for (i, &reason) in InternalReason::ALL.iter().enumerate() {
        assert_eq!(
            r.execute(&mut ctx, "t.fail", json!({"i": i})),
            Err(CommandError::Internal {
                reason,
                message: "result of `t.fail` violates its schema: x".to_owned(),
            }),
            "{reason:?}"
        );
    }
    assert!(ctx.log().is_empty(), "a failed command was logged");
}

// --------------------------------------------------------------------------------------------
// AC-5: the shared `From` mappings.

fn p(s: &str) -> RelPath {
    RelPath::new(s).expect("valid path")
}

/// Each variant of `VfsError`, `OpenError`, `EditError` and `ToolError` maps to its pinned
/// reason (`From<&…>`), including errors produced by the lower layers themselves.
#[test]
fn lower_layer_errors_map_to_pinned_reasons() {
    let vfs = [
        (VfsError::NotFound { path: p("a") }, RejectReason::NotFound),
        (VfsError::NotAFile { path: p("a") }, RejectReason::NotAFile),
        (
            VfsError::Io {
                path: p("a"),
                kind: ErrorKind::PermissionDenied,
                message: "not found".to_owned(),
            },
            RejectReason::IoError,
        ),
        (
            VfsError::Io {
                path: p("not found"),
                kind: ErrorKind::NotFound,
                message: "is a directory, not a file".to_owned(),
            },
            RejectReason::IoError,
        ),
    ];
    for (e, want) in &vfs {
        assert_eq!(RejectReason::from(e), *want, "{e:?}");
        let open = OpenError::Vfs(e.clone());
        assert_eq!(RejectReason::from(&open), *want, "{open:?}");
    }
    for encoding in [
        Encoding::Utf8,
        Encoding::Utf8Bom,
        Encoding::Utf16Le,
        Encoding::Utf16Be,
    ] {
        let open = OpenError::Decode {
            path: p("missing: not found"),
            error: DecodeError {
                encoding,
                byte_offset: 3,
            },
        };
        assert_eq!(RejectReason::from(&open), RejectReason::DecodeError);
    }
    let edit = [
        (
            EditError::OutOfBounds {
                index: 9,
                len_chars: 3,
            },
            RejectReason::OutOfRange,
        ),
        (
            EditError::InvertedRange { start: 2, end: 1 },
            RejectReason::InvertedRange,
        ),
    ];
    for (e, want) in &edit {
        assert_eq!(RejectReason::from(e), *want, "{e:?}");
    }
    for tool in [Tool::Cargo, Tool::Rustfmt] {
        let tools = [
            (ToolError::Unavailable, RejectReason::ToolUnavailable),
            (ToolError::NotFound { tool }, RejectReason::ToolNotFound),
            (
                ToolError::Timeout { tool, seconds: 30 },
                RejectReason::ToolTimeout,
            ),
            (
                ToolError::Failed {
                    tool,
                    message: "timed out".to_owned(),
                },
                RejectReason::ToolFailed,
            ),
            (ToolError::NoRecording { tool }, RejectReason::NoRecording),
        ];
        for (e, want) in &tools {
            assert_eq!(RejectReason::from(e), *want, "{e:?}");
        }
    }

    // Errors as the lower layers raise them.
    let mut b = Buffer::from_text("abc");
    let e = b.insert(99, "x").expect_err("out of bounds");
    assert_eq!(RejectReason::from(&e), RejectReason::OutOfRange);
    let (start, end) = (2, 1);
    let e = b.delete(start..end).expect_err("inverted");
    assert_eq!(RejectReason::from(&e), RejectReason::InvertedRange);
    let mut fs = MemoryFs::new();
    fs.insert(&p("d/x"), b"x");
    fs.insert(&p("bad.bin"), b"\xff");
    for (path, want) in [
        ("missing", RejectReason::NotFound),
        ("d", RejectReason::NotAFile),
        ("bad.bin", RejectReason::DecodeError),
    ] {
        let e = Document::open(&fs, &p(path)).expect_err(path);
        assert_eq!(RejectReason::from(&e), want, "{path}");
    }
}

// --------------------------------------------------------------------------------------------
// Names, order and descriptions of the reasons.

/// Today's kinds with today's "When" cells (`invalid_path` reworded), then the eleven new
/// kinds, in declaration order (Interface, "Reasons").
const REJECTED: &[(RejectReason, &str, &str)] = &[
    (
        RejectReason::InvalidPath,
        "invalid_path",
        "the path is not a valid root-relative path (see Paths in `docs/control-protocol.md`)",
    ),
    (
        RejectReason::NoSuchDocument,
        "no_such_document",
        "no open document has this id",
    ),
    (
        RejectReason::UnsavedChanges,
        "unsaved_changes",
        "`doc.close` of a modified document without `discard`",
    ),
    (
        RejectReason::NotFound,
        "not_found",
        "the path leads to nothing, or a component on its way that must be a directory is missing or is not a directory (a save creates a missing file, never a missing directory)",
    ),
    (
        RejectReason::NotAFile,
        "not_a_file",
        "the path leads to a directory, FIFO, socket or device where a regular file is needed, or to a file, FIFO, socket or device where a directory is needed",
    ),
    (
        RejectReason::IoError,
        "io_error",
        "any other file-system failure (permissions, name too long, …)",
    ),
    (
        RejectReason::DecodeError,
        "decode_error",
        "the file is not valid UTF-8 / UTF-16 text",
    ),
    (
        RejectReason::OutOfRange,
        "out_of_range",
        "an offset beyond the end of the text (in chars, bytes or UTF-16 code units), a line beyond the last line, a column beyond the end of its line, or an outline item index beyond the last item",
    ),
    (
        RejectReason::InvertedRange,
        "inverted_range",
        "a range whose start is after its end",
    ),
    (RejectReason::Other, "other", "any other refusal"),
    (
        RejectReason::InvalidPattern,
        "invalid_pattern",
        "the search pattern is empty, longer than 1,000 chars, or in regex mode not accepted by the regex engine or with more than 100 capture groups; or a regex replacement refers to a capture group the pattern does not have",
    ),
    (
        RejectReason::StalePreview,
        "stale_preview",
        "a file of a replace preview changed or was removed since the preview",
    ),
    (
        RejectReason::NoSuchPreview,
        "no_such_preview",
        "no stored replace preview has this id",
    ),
    (
        RejectReason::ToolUnavailable,
        "tool_unavailable",
        "external tools are off for this session (the default)",
    ),
    (
        RejectReason::ToolNotFound,
        "tool_not_found",
        "the external tool's program was not found",
    ),
    (
        RejectReason::ToolTimeout,
        "tool_timeout",
        "the external tool ran longer than its limit and was stopped",
    ),
    (
        RejectReason::ToolFailed,
        "tool_failed",
        "the external tool failed or gave no usable result",
    ),
    (
        RejectReason::NoRecording,
        "no_recording",
        "the replayed tool recording has no output for this request",
    ),
    (
        RejectReason::FormatError,
        "format_error",
        "rustfmt could not format the text",
    ),
    (
        RejectReason::ChangedOnDisk,
        "changed_on_disk",
        "the file changed or was removed on disk since it was read",
    ),
    (
        RejectReason::InvalidKeymap,
        "invalid_keymap",
        "the keymap file or chord is not valid",
    ),
];

const INTERNAL: &[(InternalReason, &str, &str)] = &[
    (
        InternalReason::Panic,
        "panic",
        "a command panicked (a bug in Kiln)",
    ),
    (
        InternalReason::ResultSchema,
        "result_schema",
        "a command's result violates its own schema (a bug in Kiln)",
    ),
    (InternalReason::Other, "other", "any other internal error"),
];

/// `true` iff `want` occurs in `all` in this order (later tasks may add variants elsewhere).
fn is_subsequence<T: PartialEq>(want: &[T], all: &[T]) -> bool {
    let mut it = all.iter();
    want.iter().all(|w| it.any(|a| a == w))
}

/// `as_str` equals the serde form and round-trips; descriptions are pinned; `ALL` lists each
/// variant once, the pinned ones in declaration order.
#[test]
fn reasons_have_pinned_names_order_and_descriptions() {
    for (reason, kind, when) in REJECTED {
        assert_eq!(reason.as_str(), *kind);
        assert_eq!(
            serde_json::to_value(reason).expect("serialize"),
            json!(kind)
        );
        assert_eq!(
            serde_json::from_value::<RejectReason>(json!(kind)).expect("deserialize"),
            *reason
        );
        assert_eq!(reason.description(), *when, "{kind}");
    }
    for (reason, kind, when) in INTERNAL {
        assert_eq!(reason.as_str(), *kind);
        assert_eq!(
            serde_json::to_value(reason).expect("serialize"),
            json!(kind)
        );
        assert_eq!(
            serde_json::from_value::<InternalReason>(json!(kind)).expect("deserialize"),
            *reason
        );
        assert_eq!(reason.description(), *when, "{kind}");
    }
    let rejected: Vec<RejectReason> = REJECTED.iter().map(|(r, ..)| *r).collect();
    let internal: Vec<InternalReason> = INTERNAL.iter().map(|(r, ..)| *r).collect();
    assert!(
        is_subsequence(&rejected, RejectReason::ALL),
        "RejectReason::ALL misses a pinned variant or breaks declaration order: {:?}",
        RejectReason::ALL
    );
    assert!(
        is_subsequence(&internal, InternalReason::ALL),
        "InternalReason::ALL: {:?}",
        InternalReason::ALL
    );
    for all in [
        RejectReason::ALL
            .iter()
            .map(|r| r.as_str())
            .collect::<Vec<_>>(),
        InternalReason::ALL.iter().map(|r| r.as_str()).collect(),
    ] {
        let unique: BTreeSet<&str> = all.iter().copied().collect();
        assert_eq!(unique.len(), all.len(), "duplicate kind in {all:?}");
    }
    for r in RejectReason::ALL {
        assert_eq!(
            serde_json::to_value(r).expect("serialize"),
            json!(r.as_str())
        );
        assert!(!r.description().trim().is_empty(), "{r:?}");
    }
    for r in InternalReason::ALL {
        assert_eq!(
            serde_json::to_value(r).expect("serialize"),
            json!(r.as_str())
        );
        assert!(!r.description().trim().is_empty(), "{r:?}");
    }
}

/// `describe()` publishes `error_kinds` as `ALL` with descriptions; its other key is kept.
#[test]
fn describe_publishes_error_kinds_from_all() {
    let d = reg().describe();
    let keys: BTreeSet<&str> = d
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys, BTreeSet::from(["commands", "error_kinds"]));
    let declared: BTreeSet<&str> = d["commands"]
        .as_array()
        .expect("commands")
        .iter()
        .flat_map(|c| c["refusals"].as_array().expect("refusals").iter())
        .map(|k| k.as_str().expect("kind string"))
        .collect();
    let rejected: Vec<Value> = RejectReason::ALL
        .iter()
        .filter(|r| declared.contains(r.as_str()))
        .map(|r| json!({"kind": r.as_str(), "when": r.description()}))
        .collect();
    let internal: Vec<Value> = InternalReason::ALL
        .iter()
        .map(|r| json!({"kind": r.as_str(), "when": r.description()}))
        .collect();
    assert_eq!(
        d["error_kinds"],
        json!({"rejected": rejected, "internal": internal})
    );
    assert!(d["commands"].as_array().is_some_and(|c| !c.is_empty()));
}
