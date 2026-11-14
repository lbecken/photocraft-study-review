//! The `cursor.*` command group (T-018): read, set and move the selection of a document.
//!
//! The selection lives in the document's `kiln_text::Buffer`, which maps it through every
//! edit (`doc.insert`/`delete`/`replace`, undo, redo, `eol.normalize`), so no edit command
//! knows about it. Offsets are chars. `cursor.set` and `cursor.move` end the document's typing
//! group when they succeed; a refused one changes nothing. Check order: params schema
//! (registry), then the document id, then `anchor`, then `head`. Protocol description:
//! `docs/protocol/cursor.md`.

use kiln_text::{Buffer, LineCol, Movement, PositionError, Selection};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::reason::{InternalReason, RejectReason};
use crate::registry::{
    Command, CommandError, Context, Example, Refusal, RefusalExample, Registry, RegistryError,
    SeedFile, no_such_document_example,
};

/// Register the `cursor.*` commands.
pub(crate) fn register(registry: &mut Registry) -> Result<(), RegistryError> {
    registry.register::<Get>()?;
    registry.register::<Set>()?;
    registry.register::<Move>()?;
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

/// Drop the `"default": null` that `#[serde(default)]` adds to the schema: an absent `head`
/// defaults to `anchor`, and `null` is not a value of the bare type.
fn no_default(schema: &mut schemars::Schema) {
    schema.remove("default");
}

/// A `u64` from the params as a `usize`; a value that does not fit saturates, which is beyond
/// every buffer length and so is refused with `out_of_range`.
fn index(n: u64) -> usize {
    usize::try_from(n).unwrap_or(usize::MAX)
}

/// Result of every `cursor.*` command: the selection of the document afterwards.
#[derive(Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub(crate) struct CursorResult {
    /// Id of the document.
    doc: u64,
    /// Char offset of the selection's anchor (the end that stays put while extending).
    anchor: usize,
    /// Char offset of the selection's head (the caret).
    head: usize,
    /// The smaller of `anchor` and `head`.
    start: usize,
    /// The larger of `anchor` and `head`.
    end: usize,
    /// Zero-based line of the head.
    line: usize,
    /// Zero-based column of the head, in chars.
    col: usize,
}

pub(crate) fn cursor_result(doc: u64, buffer: &Buffer) -> Result<CursorResult, CommandError> {
    let s = buffer.selection();
    // The buffer keeps the selection at valid positions, so this cannot fail.
    let LineCol { line, col } =
        buffer
            .char_to_line_col(s.head)
            .map_err(|e| CommandError::Internal {
                reason: InternalReason::Other,
                message: format!("selection head is not a valid position: {e}"),
            })?;
    Ok(CursorResult {
        doc,
        anchor: s.anchor,
        head: s.head,
        start: s.start(),
        end: s.end(),
        line,
        col,
    })
}

// ---------------------------------------------------------------------------------------------
// cursor.get

/// Params naming one open document.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct GetParams {
    /// Id of an open document.
    doc: u64,
}

pub(crate) struct Get;

impl Command for Get {
    const NAME: &'static str = "cursor.get";
    const DOC: &'static str = "Return a document's selection (anchor and head, char offsets) \
        and the head's line and column. Read-only.";
    const MUTATING: bool = false;
    type Params = GetParams;
    type Output = CursorResult;

    fn examples() -> Vec<Example> {
        vec![Example {
            description: "the selection follows an insert before it",
            initial_text: "",
            files: vec![("a.txt", "abc\r\ndef")],
            setup: vec![
                ("doc.open", json!({"path": "a.txt"})),
                ("cursor.set", json!({"doc": 1, "anchor": 1, "head": 6})),
                ("doc.insert", json!({"doc": 1, "offset": 0, "text": "XY"})),
            ],
            params: json!({"doc": 1}),
            result: json!({"doc": 1, "anchor": 3, "head": 8, "start": 3, "end": 8,
                "line": 1, "col": 1}),
        }]
    }

    fn refusals() -> Option<Vec<Refusal>> {
        Some(vec![Refusal::example(RejectReason::NoSuchDocument)])
    }

    fn refusal_examples() -> Vec<RefusalExample> {
        vec![no_such_document_example(json!({"doc": 9}))]
    }

    fn run(ctx: &mut Context, p: GetParams) -> Result<CursorResult, CommandError> {
        cursor_result(p.doc, ctx.doc(p.doc)?.buffer())
    }
}

// ---------------------------------------------------------------------------------------------
// cursor.set

/// Params of `cursor.set`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SetParams {
    /// Id of an open document.
    doc: u64,
    /// Char offset of the anchor (`0..=len_chars`, not between `\r` and `\n`).
    anchor: u64,
    /// Char offset of the head; defaults to `anchor` (a caret). Omit it for a caret; `null`
    /// is refused.
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "u64", transform = no_default)]
    head: Option<u64>,
}

fn position_refusal(which: &str, e: &PositionError) -> CommandError {
    CommandError::rejected(e.into(), format!("invalid {which}: {e}"))
}

pub(crate) struct Set;

impl Command for Set {
    const NAME: &'static str = "cursor.set";
    const DOC: &'static str = "Set a document's selection to `anchor`..`head` (char offsets; \
        `head` defaults to `anchor`). Ends the document's typing group.";
    const MUTATING: bool = true;
    type Params = SetParams;
    type Output = CursorResult;

    fn examples() -> Vec<Example> {
        vec![
            Example {
                description: "select across a CRLF",
                initial_text: "",
                files: vec![("a.txt", "abc\r\ndef")],
                setup: vec![("doc.open", json!({"path": "a.txt"}))],
                params: json!({"doc": 1, "anchor": 1, "head": 6}),
                result: json!({"doc": 1, "anchor": 1, "head": 6, "start": 1, "end": 6,
                    "line": 1, "col": 1}),
            },
            Example {
                description: "a caret (head defaults to anchor)",
                initial_text: "",
                files: vec![("a.txt", "abc\r\ndef")],
                setup: vec![("doc.open", json!({"path": "a.txt"}))],
                params: json!({"doc": 1, "anchor": 2}),
                result: json!({"doc": 1, "anchor": 2, "head": 2, "start": 2, "end": 2,
                    "line": 0, "col": 2}),
            },
        ]
    }

    fn refusals() -> Option<Vec<Refusal>> {
        Some(vec![
            Refusal::example(RejectReason::NoSuchDocument),
            Refusal::example(RejectReason::OutOfRange),
            Refusal::example(RejectReason::NotABoundary),
        ])
    }

    fn refusal_examples() -> Vec<RefusalExample> {
        vec![
            no_such_document_example(json!({"doc": 9, "anchor": 0})),
            RefusalExample {
                description: "an anchor beyond the end of the text",
                initial_text: "",
                files: vec![SeedFile::text("f.txt", "abc\r\ndef\n")],
                setup: vec![("doc.open", json!({"path": "f.txt"}))],
                params: json!({"doc": 1, "anchor": 99}),
                kind: RejectReason::OutOfRange,
            },
            RefusalExample {
                description: "a head between the CR and LF of a CRLF",
                initial_text: "",
                files: vec![SeedFile::text("f.txt", "abc\r\ndef\n")],
                setup: vec![("doc.open", json!({"path": "f.txt"}))],
                params: json!({"doc": 1, "anchor": 0, "head": 4}),
                kind: RejectReason::NotABoundary,
            },
        ]
    }

    fn run(ctx: &mut Context, p: SetParams) -> Result<CursorResult, CommandError> {
        let buffer = ctx.doc_mut(p.doc)?.buffer_mut();
        let anchor = index(p.anchor);
        let head = p.head.map_or(anchor, index);
        // Check each end separately so the refusal names the end at fault (anchor first).
        buffer
            .char_to_line_col(anchor)
            .map_err(|e| position_refusal("anchor", &e))?;
        buffer
            .char_to_line_col(head)
            .map_err(|e| position_refusal("head", &e))?;
        buffer
            .set_selection(Selection { anchor, head })
            .map_err(|e| position_refusal("position", &e))?;
        buffer.break_undo_group();
        cursor_result(p.doc, buffer)
    }
}

// ---------------------------------------------------------------------------------------------
// cursor.move

/// The unit of a `cursor.move` step.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum By {
    /// One char; a CRLF is one step.
    Char,
    /// Over non-word chars, then word chars (`_` or alphanumeric).
    Word,
    /// The same column on the next or previous line, clamped to its length.
    Line,
}

/// Params of `cursor.move`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct MoveParams {
    /// Id of an open document.
    doc: u64,
    /// The unit of the step.
    by: By,
    /// True to move towards the end of the text (down for `line`).
    forward: bool,
    /// True to move only the head (extend the selection); false (default) to collapse the
    /// selection at the moved head.
    #[serde(default)]
    extend: bool,
}

pub(crate) struct Move;

impl Command for Move {
    const NAME: &'static str = "cursor.move";
    const DOC: &'static str = "Move a document's selection head one step by char, word or line, \
        computed from the head before the move. Without `extend` the selection collapses at \
        the new head; with it only the head moves. Ends the document's typing group.";
    const MUTATING: bool = true;
    type Params = MoveParams;
    type Output = CursorResult;

    fn examples() -> Vec<Example> {
        vec![
            Example {
                description: "a word forward",
                initial_text: "",
                files: vec![("a.rs", "fn foo_bar(x: u8)")],
                setup: vec![
                    ("doc.open", json!({"path": "a.rs"})),
                    ("cursor.set", json!({"doc": 1, "anchor": 2})),
                ],
                params: json!({"doc": 1, "by": "word", "forward": true}),
                result: json!({"doc": 1, "anchor": 10, "head": 10, "start": 10, "end": 10,
                    "line": 0, "col": 10}),
            },
            Example {
                description: "extend a line down (no goal column)",
                initial_text: "",
                files: vec![("a.txt", "abcdef\nxy\nlonger")],
                setup: vec![
                    ("doc.open", json!({"path": "a.txt"})),
                    ("cursor.set", json!({"doc": 1, "anchor": 5})),
                ],
                params: json!({"doc": 1, "by": "line", "forward": true, "extend": true}),
                result: json!({"doc": 1, "anchor": 5, "head": 9, "start": 5, "end": 9,
                    "line": 1, "col": 2}),
            },
        ]
    }

    fn refusals() -> Option<Vec<Refusal>> {
        Some(vec![Refusal::example(RejectReason::NoSuchDocument)])
    }

    fn refusal_examples() -> Vec<RefusalExample> {
        vec![no_such_document_example(
            json!({"doc": 9, "by": "char", "forward": true}),
        )]
    }

    fn run(ctx: &mut Context, p: MoveParams) -> Result<CursorResult, CommandError> {
        let buffer = ctx.doc_mut(p.doc)?.buffer_mut();
        let old = buffer.selection();
        let by = match p.by {
            By::Char => Movement::Char,
            By::Word => Movement::Word,
            By::Line => Movement::Line,
        };
        let head = buffer.moved(old.head, by, p.forward);
        let anchor = if p.extend { old.anchor } else { head };
        buffer
            .set_selection(Selection { anchor, head })
            .map_err(|e| position_refusal("position", &e))?;
        buffer.break_undo_group();
        cursor_result(p.doc, buffer)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use crate::{Context, Registry, standard_registry};

    fn setup(text: &str) -> (Registry, Context) {
        let r = standard_registry().unwrap();
        let mut ctx = Context::with_files(&[("a.txt", text.as_bytes())]).unwrap();
        r.execute(&mut ctx, "doc.open", json!({"path": "a.txt"}))
            .unwrap();
        (r, ctx)
    }

    fn ok(r: &Registry, ctx: &mut Context, name: &str, params: Value) -> Value {
        r.execute(ctx, name, params.clone())
            .unwrap_or_else(|e| panic!("{name} {params}: {e}"))
    }

    fn refusal(r: &Registry, ctx: &mut Context, name: &str, params: Value) -> Value {
        let e = r.execute(ctx, name, params).unwrap_err().to_json();
        json!([e["code"], e["reason"]])
    }

    fn ends(v: &Value) -> (u64, u64) {
        (v["anchor"].as_u64().unwrap(), v["head"].as_u64().unwrap())
    }

    #[test]
    fn worked_example_follows_insert_and_delete() {
        let (r, mut ctx) = setup("abc\r\ndef");
        ok(
            &r,
            &mut ctx,
            "cursor.set",
            json!({"doc": 1, "anchor": 1, "head": 6}),
        );
        ok(
            &r,
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": 0, "text": "XY"}),
        );
        let v = ok(&r, &mut ctx, "cursor.get", json!({"doc": 1}));
        assert_eq!(ends(&v), (3, 8));
        ok(
            &r,
            &mut ctx,
            "doc.delete",
            json!({"doc": 1, "start": 4, "end": 7}),
        );
        let v = ok(&r, &mut ctx, "cursor.get", json!({"doc": 1}));
        assert_eq!(
            v,
            json!({"doc": 1, "anchor": 3, "head": 5, "start": 3, "end": 5, "line": 0, "col": 5})
        );
    }

    #[test]
    fn move_examples_and_collapse() {
        let (r, mut ctx) = setup("fn foo_bar(x: u8)");
        let mut heads = Vec::new();
        for forward in [true, true, true, true, false, false, false, false] {
            let v = ok(
                &r,
                &mut ctx,
                "cursor.move",
                json!({"doc": 1, "by": "word", "forward": forward}),
            );
            assert_eq!(v["anchor"], v["head"]);
            heads.push(v["head"].as_u64().unwrap());
        }
        assert_eq!(heads, [2, 10, 12, 16, 14, 11, 3, 0]);
        let (r, mut ctx) = setup("a\r\nb");
        ok(&r, &mut ctx, "cursor.set", json!({"doc": 1, "anchor": 1}));
        let v = ok(
            &r,
            &mut ctx,
            "cursor.move",
            json!({"doc": 1, "by": "char", "forward": true}),
        );
        assert_eq!(v["head"], 3);
        assert_eq!((v["line"].as_u64(), v["col"].as_u64()), (Some(1), Some(0)));
        let (r, mut ctx) = setup("abcdef\nxy\nlonger");
        ok(&r, &mut ctx, "cursor.set", json!({"doc": 1, "anchor": 5}));
        let p = json!({"doc": 1, "by": "line", "forward": true, "extend": true});
        assert_eq!(ends(&ok(&r, &mut ctx, "cursor.move", p.clone())), (5, 9));
        let v = ok(&r, &mut ctx, "cursor.move", p);
        assert_eq!(ends(&v), (5, 12));
        assert_eq!(
            (v["start"].as_u64(), v["end"].as_u64()),
            (Some(5), Some(12))
        );
        assert_eq!((v["line"].as_u64(), v["col"].as_u64()), (Some(2), Some(2)));
    }

    #[test]
    fn refusals_change_nothing() {
        let (r, mut ctx) = setup("a\r\nb");
        ok(
            &r,
            &mut ctx,
            "cursor.set",
            json!({"doc": 1, "anchor": 4, "head": 0}),
        );
        let before = ok(&r, &mut ctx, "cursor.get", json!({"doc": 1}));
        let out = json!(["rejected", "out_of_range"]);
        let nab = json!(["rejected", "not_a_boundary"]);
        let nsd = json!(["rejected", "no_such_document"]);
        for (params, want) in [
            (json!({"doc": 1, "anchor": 5}), &out),
            (json!({"doc": 1, "anchor": 2}), &nab),
            (json!({"doc": 1, "anchor": 0, "head": 5}), &out),
            (json!({"doc": 1, "anchor": 0, "head": 2}), &nab),
            // anchor is checked before head
            (json!({"doc": 1, "anchor": 2, "head": 5}), &nab),
            (json!({"doc": 1, "anchor": 5, "head": 2}), &out),
            (json!({"doc": 1, "anchor": u64::MAX}), &out),
            // the document is checked before the positions
            (json!({"doc": 2, "anchor": 9}), &nsd),
            (json!({"doc": 0, "anchor": 0}), &nsd),
        ] {
            assert_eq!(
                refusal(&r, &mut ctx, "cursor.set", params.clone()),
                *want,
                "{params}"
            );
            assert_eq!(ok(&r, &mut ctx, "cursor.get", json!({"doc": 1})), before);
        }
        let move_p = json!({"doc": 7, "by": "char", "forward": true});
        assert_eq!(refusal(&r, &mut ctx, "cursor.move", move_p), nsd);
        assert_eq!(refusal(&r, &mut ctx, "cursor.get", json!({"doc": 0})), nsd);
        let bad = json!(["invalid_params", null]);
        let schema = refusal(
            &r,
            &mut ctx,
            "cursor.move",
            json!({"doc": 1, "by": "page", "forward": true}),
        );
        assert_eq!(schema[0], bad[0]);
    }

    /// w3: typing "a", moving the cursor, typing "b" is two undo steps through the commands
    /// (the buffer-level equivalent is one, see kiln-text's tests); a refused cursor command
    /// leaves the group open.
    #[test]
    fn cursor_commands_end_the_typing_group_only_on_success() {
        for (name, params) in [
            ("cursor.set", json!({"doc": 1, "anchor": 1})),
            (
                "cursor.move",
                json!({"doc": 1, "by": "char", "forward": true}),
            ),
        ] {
            let (r, mut ctx) = setup("");
            ok(
                &r,
                &mut ctx,
                "doc.insert",
                json!({"doc": 1, "offset": 0, "text": "a"}),
            );
            ok(&r, &mut ctx, name, params);
            ok(
                &r,
                &mut ctx,
                "doc.insert",
                json!({"doc": 1, "offset": 1, "text": "b"}),
            );
            ok(&r, &mut ctx, "doc.undo", json!({"doc": 1}));
            let text = ok(&r, &mut ctx, "doc.text", json!({"doc": 1}));
            assert_eq!(text["text"], "a", "{name}");
        }
        let (r, mut ctx) = setup("");
        ok(
            &r,
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": 0, "text": "a"}),
        );
        refusal(&r, &mut ctx, "cursor.set", json!({"doc": 1, "anchor": 9}));
        ok(&r, &mut ctx, "cursor.get", json!({"doc": 1}));
        ok(
            &r,
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": 1, "text": "b"}),
        );
        ok(&r, &mut ctx, "doc.undo", json!({"doc": 1}));
        let text = ok(&r, &mut ctx, "doc.text", json!({"doc": 1}));
        assert_eq!(text["text"], "");
    }

    /// x1: `null` is not a value of an optional parameter; absent `head` still means a caret.
    #[test]
    fn null_head_is_invalid_params() {
        let (r, mut ctx) = setup("abc");
        ok(
            &r,
            &mut ctx,
            "cursor.set",
            json!({"doc": 1, "anchor": 1, "head": 2}),
        );
        let e = refusal(
            &r,
            &mut ctx,
            "cursor.set",
            json!({"doc": 1, "anchor": 3, "head": null}),
        );
        assert_eq!(e[0], "invalid_params");
        assert_eq!(
            ends(&ok(&r, &mut ctx, "cursor.get", json!({"doc": 1}))),
            (1, 2)
        );
        let e = refusal(
            &r,
            &mut ctx,
            "cursor.move",
            json!({"doc": 1, "by": "char", "forward": true, "extend": null}),
        );
        assert_eq!(e[0], "invalid_params");
        let v = ok(&r, &mut ctx, "cursor.set", json!({"doc": 1, "anchor": 3}));
        assert_eq!(ends(&v), (3, 3));
        let described = r.describe();
        let schema = &described["commands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "cursor.set")
            .unwrap()["params_schema"]["properties"]["head"];
        assert_eq!(schema["type"], "integer");
        assert!(schema.get("default").is_none());
    }

    #[test]
    fn selection_follows_undo_and_redo() {
        let (r, mut ctx) = setup("hello");
        ok(&r, &mut ctx, "cursor.set", json!({"doc": 1, "anchor": 5}));
        ok(
            &r,
            &mut ctx,
            "doc.replace",
            json!({"doc": 1, "start": 1, "end": 4, "text": "X"}),
        );
        assert_eq!(
            ends(&ok(&r, &mut ctx, "cursor.get", json!({"doc": 1}))),
            (3, 3)
        );
        ok(&r, &mut ctx, "doc.undo", json!({"doc": 1}));
        assert_eq!(
            ends(&ok(&r, &mut ctx, "cursor.get", json!({"doc": 1}))),
            (5, 5)
        );
        ok(&r, &mut ctx, "doc.redo", json!({"doc": 1}));
        assert_eq!(
            ends(&ok(&r, &mut ctx, "cursor.get", json!({"doc": 1}))),
            (3, 3)
        );
    }
}
