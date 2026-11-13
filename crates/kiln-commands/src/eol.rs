//! The `eol.*` command group (T-018): count a document's line breaks per kind and normalise
//! them on explicit request.
//!
//! Nothing is normalised at open or save; `eol.normalize` is the only command that rewrites
//! line breaks. It is one `kiln_text::Buffer::normalize_line_endings` call: one batch with one
//! edit per line break that differs from the target, so it is one undo step, it maps the
//! selection like any edit, and when no break differs nothing changes at all (the open typing
//! group included). Protocol description: `docs/protocol/eol.md`.

use kiln_text::{Buffer, LineEnding, LineEndings};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::doc::LineEndingName;
use crate::registry::{Command, CommandError, Context, Example, Registry, RegistryError};

/// Register the `eol.*` commands.
pub(crate) fn register(registry: &mut Registry) -> Result<(), RegistryError> {
    registry.register::<Counts>()?;
    registry.register::<Normalize>()?;
    Ok(())
}

/// As `doc.info.line_ending`: the dominant kind (ties to LF, then CRLF, then CR), or `none`.
fn line_ending_name(buffer: &Buffer) -> LineEndingName {
    match buffer.dominant_line_ending() {
        None => LineEndingName::None,
        Some(LineEnding::Lf) => LineEndingName::Lf,
        Some(LineEnding::Crlf) => LineEndingName::Crlf,
        Some(LineEnding::Cr) => LineEndingName::Cr,
    }
}

/// As `doc.info.mixed_line_endings`.
fn mixed(buffer: &Buffer) -> bool {
    buffer.line_endings() == LineEndings::Mixed
}

// ---------------------------------------------------------------------------------------------
// eol.counts

/// Params naming one open document.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CountsParams {
    /// Id of an open document.
    doc: u64,
}

/// Result of `eol.counts`.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct CountsResult {
    /// Number of lone `\n` line breaks.
    lf: usize,
    /// Number of `\r\n` line breaks (matched before lone `\r` and `\n`).
    crlf: usize,
    /// Number of lone `\r` line breaks.
    cr: usize,
    /// Dominant line ending of the current text (as in `doc.info`).
    line_ending: LineEndingName,
    /// True iff the current text has line breaks of more than one kind (as in `doc.info`).
    mixed_line_endings: bool,
}

pub(crate) struct Counts;

impl Command for Counts {
    const NAME: &'static str = "eol.counts";
    const DOC: &'static str = "Count a document's line breaks per kind (`\\r\\n` first, then \
        lone `\\r` and `\\n`). Read-only.";
    const MUTATING: bool = false;
    type Params = CountsParams;
    type Output = CountsResult;

    fn examples() -> Vec<Example> {
        vec![Example {
            description: "a file with mixed line endings",
            initial_text: "",
            files: vec![("a.txt", "a\r\nb\nc\rd\r\n")],
            setup: vec![("doc.open", json!({"path": "a.txt"}))],
            params: json!({"doc": 1}),
            result: json!({"lf": 1, "crlf": 2, "cr": 1, "line_ending": "crlf",
                "mixed_line_endings": true}),
        }]
    }

    fn run(ctx: &mut Context, p: CountsParams) -> Result<CountsResult, CommandError> {
        let buffer = ctx.doc(p.doc)?.buffer();
        Ok(CountsResult {
            lf: buffer.line_break_count(LineEnding::Lf),
            crlf: buffer.line_break_count(LineEnding::Crlf),
            cr: buffer.line_break_count(LineEnding::Cr),
            line_ending: line_ending_name(buffer),
            mixed_line_endings: mixed(buffer),
        })
    }
}

// ---------------------------------------------------------------------------------------------
// eol.normalize

/// A target line ending.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Target {
    /// `\n`.
    Lf,
    /// `\r\n`.
    Crlf,
    /// A lone `\r`.
    Cr,
}

/// Params of `eol.normalize`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct NormalizeParams {
    /// Id of an open document.
    doc: u64,
    /// The line ending every break is rewritten to; defaults to the document's dominant one
    /// (`doc.info.line_ending`).
    #[serde(default)]
    to: Option<Target>,
}

/// Result of `eol.normalize`.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct NormalizeResult {
    /// Number of line breaks rewritten (those that differed from the target).
    changed: usize,
    /// Length in chars afterwards.
    len_chars: usize,
    /// Dominant line ending afterwards (as in `doc.info`).
    line_ending: LineEndingName,
    /// True iff the text still has line breaks of more than one kind (as in `doc.info`).
    mixed_line_endings: bool,
}

pub(crate) struct Normalize;

impl Command for Normalize {
    const NAME: &'static str = "eol.normalize";
    const DOC: &'static str = "Rewrite every line break of a document that differs from `to` \
        (default: the dominant line ending) as `to`, as one undo step. The selection maps \
        through each rewritten break. If no break differs, nothing changes.";
    const MUTATING: bool = true;
    type Params = NormalizeParams;
    type Output = NormalizeResult;

    fn examples() -> Vec<Example> {
        vec![
            Example {
                description: "normalise a mixed file to LF",
                initial_text: "",
                files: vec![("a.txt", "a\r\nb\nc\rd")],
                setup: vec![("doc.open", json!({"path": "a.txt"}))],
                params: json!({"doc": 1, "to": "lf"}),
                result: json!({"changed": 2, "len_chars": 7, "line_ending": "lf",
                    "mixed_line_endings": false}),
            },
            Example {
                description: "default target: the dominant line ending",
                initial_text: "",
                files: vec![("a.txt", "a\r\nb\r\nc\n")],
                setup: vec![("doc.open", json!({"path": "a.txt"}))],
                params: json!({"doc": 1}),
                result: json!({"changed": 1, "len_chars": 9, "line_ending": "crlf",
                    "mixed_line_endings": false}),
            },
        ]
    }

    fn run(ctx: &mut Context, p: NormalizeParams) -> Result<NormalizeResult, CommandError> {
        let buffer = ctx.doc_mut(p.doc)?.buffer_mut();
        let to = match p.to {
            Some(Target::Lf) => Some(LineEnding::Lf),
            Some(Target::Crlf) => Some(LineEnding::Crlf),
            Some(Target::Cr) => Some(LineEnding::Cr),
            None => buffer.dominant_line_ending(),
        };
        // No target means no line break: nothing to rewrite.
        let changed = to.map_or(0, |to| buffer.normalize_line_endings(to));
        Ok(NormalizeResult {
            changed,
            len_chars: buffer.len_chars(),
            line_ending: line_ending_name(buffer),
            mixed_line_endings: mixed(buffer),
        })
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

    fn text(r: &Registry, ctx: &mut Context) -> String {
        ok(r, ctx, "doc.text", json!({"doc": 1}))["text"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    #[test]
    fn worked_example_normalize_and_undo() {
        let (r, mut ctx) = setup("a\r\nb\nc\rd");
        let v = ok(&r, &mut ctx, "eol.normalize", json!({"doc": 1, "to": "lf"}));
        assert_eq!(
            v,
            json!({"changed": 2, "len_chars": 7, "line_ending": "lf", "mixed_line_endings": false})
        );
        assert_eq!(text(&r, &mut ctx), "a\nb\nc\nd");
        ok(&r, &mut ctx, "doc.undo", json!({"doc": 1}));
        assert_eq!(text(&r, &mut ctx), "a\r\nb\nc\rd");
        let info = ok(&r, &mut ctx, "doc.info", json!({"doc": 1}));
        assert_eq!(info["can_undo"], false);
        ok(&r, &mut ctx, "doc.redo", json!({"doc": 1}));
        assert_eq!(text(&r, &mut ctx), "a\nb\nc\nd");
    }

    #[test]
    fn counts_and_default_target() {
        let (r, mut ctx) = setup("a\r\nb\nc\rd\n\r\n\r");
        let v = ok(&r, &mut ctx, "eol.counts", json!({"doc": 1}));
        assert_eq!(
            v,
            json!({"lf": 2, "crlf": 2, "cr": 2, "line_ending": "lf", "mixed_line_endings": true})
        );
        let v = ok(&r, &mut ctx, "eol.normalize", json!({"doc": 1}));
        assert_eq!(v["changed"], 4);
        assert_eq!(text(&r, &mut ctx), "a\nb\nc\nd\n\n\n");
        let v = ok(&r, &mut ctx, "eol.counts", json!({"doc": 1}));
        assert_eq!(
            v,
            json!({"lf": 6, "crlf": 0, "cr": 0, "line_ending": "lf", "mixed_line_endings": false})
        );
    }

    #[test]
    fn no_line_break_changes_nothing() {
        let (r, mut ctx) = setup("abc");
        ok(
            &r,
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": 3, "text": "d"}),
        );
        for p in [json!({"doc": 1}), json!({"doc": 1, "to": "cr"})] {
            let v = ok(&r, &mut ctx, "eol.normalize", p);
            assert_eq!(
                v,
                json!({"changed": 0, "len_chars": 4, "line_ending": "none",
                    "mixed_line_endings": false})
            );
        }
        // The typing group is still open: "e" joins "d".
        ok(
            &r,
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": 4, "text": "e"}),
        );
        ok(&r, &mut ctx, "doc.undo", json!({"doc": 1}));
        assert_eq!(text(&r, &mut ctx), "abc");
    }

    #[test]
    fn normalize_maps_the_selection() {
        let (r, mut ctx) = setup("a\rb\rc");
        ok(
            &r,
            &mut ctx,
            "cursor.set",
            json!({"doc": 1, "anchor": 2, "head": 5}),
        );
        let v = ok(
            &r,
            &mut ctx,
            "eol.normalize",
            json!({"doc": 1, "to": "crlf"}),
        );
        assert_eq!(v["changed"], 2);
        let c = ok(&r, &mut ctx, "cursor.get", json!({"doc": 1}));
        // 2 is the end of the first break -> s + n = 3; 5 shifts by 2.
        assert_eq!(
            (c["anchor"].as_u64(), c["head"].as_u64()),
            (Some(3), Some(7))
        );
    }

    #[test]
    fn refusals() {
        let (r, mut ctx) = setup("a\nb");
        for (name, p) in [
            ("eol.counts", json!({"doc": 2})),
            ("eol.normalize", json!({"doc": 0, "to": "lf"})),
        ] {
            let e = r.execute(&mut ctx, name, p).unwrap_err().to_json();
            assert_eq!(
                (e["code"].as_str(), e["reason"].as_str()),
                (Some("rejected"), Some("no_such_document"))
            );
        }
        let e = r
            .execute(&mut ctx, "eol.normalize", json!({"doc": 1, "to": "crcr"}))
            .unwrap_err()
            .to_json();
        assert_eq!(e["code"], "invalid_params");
        assert_eq!(text(&r, &mut ctx), "a\nb");
    }
}
