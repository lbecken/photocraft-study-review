//! The `search.*` command group (T-021): find the matches of a query in an open document, and
//! replace one or all of them as one undo step.
//!
//! Both commands compile a `kiln_search::Query` and search the document's current text with
//! `CompiledQuery::find` / `replace_first` / `replace_all`, which hold at most `limit` ranges
//! (find) or one range and one replacement text per match (replace), never the capture slots of
//! more than one match. A replace is one `kiln_text::Buffer::apply_edits` call: one undo step,
//! the selection maps through each replacement, and with no match nothing changes (the open
//! typing group included). Protocol description: `docs/protocol/search.md`.

use std::ops::Range;

use kiln_search::{CharMatch, CompiledQuery, Query, QueryError, TemplateError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::reason::RejectReason;
use crate::registry::{Command, CommandError, Context, Example, Registry, RegistryError};

/// Register the `search.*` commands.
pub(crate) fn register(registry: &mut Registry) -> Result<(), RegistryError> {
    registry.register::<Find>()?;
    registry.register::<Replace>()?;
    Ok(())
}

/// Default of `limit` (both commands).
const DEFAULT_LIMIT: usize = 10_000;

/// Deserialize an optional parameter that is present: `null` is not a value for it (an absent
/// parameter is `None` through `#[serde(default)]`). As in `eol.rs` (ruling x1, D-084).
fn present<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(d).map(Some)
}

/// Publish the true default of an optional param instead of the `"default": null` that
/// `#[serde(default)]` adds to the schema: `null` is refused for every optional param here.
fn default_false(schema: &mut schemars::Schema) {
    schema.insert("default".to_owned(), json!(false));
}

/// As [`default_false`], for a param that defaults to true.
fn default_true(schema: &mut schemars::Schema) {
    schema.insert("default".to_owned(), json!(true));
}

/// As [`default_false`], for `limit`.
fn default_limit(schema: &mut schemars::Schema) {
    schema.insert("default".to_owned(), json!(DEFAULT_LIMIT));
}

/// As [`default_false`], for `from`.
fn default_zero(schema: &mut schemars::Schema) {
    schema.insert("default".to_owned(), json!(0));
}

/// `QueryError` → `rejected`/`invalid_pattern`, with the message of the Interface.
fn query_error(e: &QueryError) -> CommandError {
    let reason = match e {
        QueryError::Empty | QueryError::TooLong { .. } | QueryError::Pattern(_) => {
            RejectReason::InvalidPattern
        }
    };
    CommandError::rejected(reason, e.to_string())
}

/// `TemplateError` → `rejected`/`invalid_pattern` (D-044).
fn template_error(e: &TemplateError) -> CommandError {
    CommandError::rejected(
        RejectReason::InvalidPattern,
        format!("invalid replacement: {e}"),
    )
}

/// A range of chars `start..end`.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct Span {
    /// First char.
    start: usize,
    /// One past the last char.
    end: usize,
}

impl From<CharMatch> for Span {
    fn from(m: CharMatch) -> Self {
        Span {
            start: m.start,
            end: m.end,
        }
    }
}

/// The query params shared by both commands, compiled (`Empty` → `TooLong` → `Pattern`).
fn compile(
    query: &str,
    regex: Option<bool>,
    case_sensitive: Option<bool>,
    whole_word: Option<bool>,
) -> Result<CompiledQuery, CommandError> {
    Query {
        pattern: query.to_owned(),
        regex: regex.unwrap_or(false),
        case_sensitive: case_sensitive.unwrap_or(true),
        whole_word: whole_word.unwrap_or(false),
    }
    .compile()
    .map_err(|e| query_error(&e))
}

// ---------------------------------------------------------------------------------------------
// search.find

/// Params of `search.find`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct FindParams {
    /// Id of an open document.
    doc: u64,
    /// The literal text or regular expression (1 to 1,000 chars).
    query: String,
    /// Regex mode; default false (literal). `null` is refused.
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "bool", transform = default_false)]
    regex: Option<bool>,
    /// Compare chars exactly; default true. False: Unicode simple case folding. `null` is
    /// refused.
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "bool", transform = default_true)]
    case_sensitive: Option<bool>,
    /// Keep only matches with a word boundary at both ends; default false. `null` is refused.
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "bool", transform = default_false)]
    whole_word: Option<bool>,
    /// Most matches listed (1 to 100,000; default 10,000); `total` counts all. `null` is
    /// refused.
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "u32", range(min = 1, max = 100_000), transform = default_limit)]
    limit: Option<usize>,
}

/// Result of `search.find`.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct FindResult {
    /// The first `limit` matches, in order, as char ranges.
    matches: Vec<Span>,
    /// The exact number of all matches.
    total: usize,
    /// True iff `total > limit`.
    truncated: bool,
}

pub(crate) struct Find;

impl Command for Find {
    const NAME: &'static str = "search.find";
    const DOC: &'static str = "List the matches of a literal or regex query in an open \
        document as char ranges (at most `limit`), with the exact number of all matches. \
        Read-only.";
    const MUTATING: bool = false;
    type Params = FindParams;
    type Output = FindResult;

    fn examples() -> Vec<Example> {
        vec![
            Example {
                description: "case-insensitive literal, whole words only",
                initial_text: "",
                files: vec![("a.txt", "foo FOO fOo_x éFoo")],
                setup: vec![("doc.open", json!({"path": "a.txt"}))],
                params: json!({"doc": 1, "query": "Foo", "case_sensitive": false,
                    "whole_word": true}),
                result: json!({"matches": [{"start": 0, "end": 3}, {"start": 4, "end": 7}],
                    "total": 2, "truncated": false}),
            },
            Example {
                description: "regex with empty matches, limited",
                initial_text: "",
                files: vec![("a.txt", "baaab")],
                setup: vec![("doc.open", json!({"path": "a.txt"}))],
                params: json!({"doc": 1, "query": "a*", "regex": true, "limit": 2}),
                result: json!({"matches": [{"start": 0, "end": 0}, {"start": 1, "end": 4}],
                    "total": 3, "truncated": true}),
            },
        ]
    }

    fn run(ctx: &mut Context, p: FindParams) -> Result<FindResult, CommandError> {
        let buffer = ctx.doc(p.doc)?.buffer();
        let query = compile(&p.query, p.regex, p.case_sensitive, p.whole_word)?;
        let limit = p.limit.unwrap_or(DEFAULT_LIMIT);
        let found = query.find(&buffer.text(), limit);
        Ok(FindResult {
            matches: found.matches.into_iter().map(Span::from).collect(),
            total: found.total,
            truncated: found.total > limit,
        })
    }
}

// ---------------------------------------------------------------------------------------------
// search.replace

/// Params of `search.replace`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReplaceParams {
    /// Id of an open document.
    doc: u64,
    /// The literal text or regular expression (1 to 1,000 chars).
    query: String,
    /// Regex mode; default false (literal). `null` is refused.
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "bool", transform = default_false)]
    regex: Option<bool>,
    /// Compare chars exactly; default true. `null` is refused.
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "bool", transform = default_true)]
    case_sensitive: Option<bool>,
    /// Keep only matches with a word boundary at both ends; default false. `null` is refused.
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "bool", transform = default_false)]
    whole_word: Option<bool>,
    /// The replacement text; in regex mode `$1`, `${name}`, `$0` refer to groups and `$$` is
    /// `$`; in literal mode it is inserted verbatim.
    replacement: String,
    /// Replace every match (`from` is then ignored); default false: only the first match whose
    /// start is at or after `from`. `null` is refused.
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "bool", transform = default_false)]
    all: Option<bool>,
    /// With `all: false`: the char offset the first replaced match starts at or after;
    /// default 0. `null` is refused.
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "usize", transform = default_zero)]
    from: Option<usize>,
    /// Most `ranges` listed in the reply (1 to 100,000; default 10,000); every match is
    /// replaced whatever it is. `null` is refused.
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "u32", range(min = 1, max = 100_000), transform = default_limit)]
    limit: Option<usize>,
}

/// Result of `search.replace`.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct ReplaceResult {
    /// The exact number of replacements made.
    replaced: usize,
    /// Length in chars afterwards.
    len_chars: usize,
    /// The char ranges of the inserted texts in the new text, in order, for the first `limit`
    /// replacements.
    ranges: Vec<Span>,
    /// True iff `replaced > limit`.
    truncated: bool,
}

pub(crate) struct Replace;

impl Command for Replace {
    const NAME: &'static str = "search.replace";
    const DOC: &'static str = "Replace the first match of a literal or regex query at or \
        after `from`, or every match (`all`), in an open document, as one undo step; regex \
        replacements may refer to capture groups. The selection maps through each \
        replacement. With no match, nothing changes.";
    const MUTATING: bool = true;
    type Params = ReplaceParams;
    type Output = ReplaceResult;

    fn examples() -> Vec<Example> {
        vec![
            Example {
                description: "swap with capture groups, every match",
                initial_text: "",
                files: vec![("a.txt", "a1 b2")],
                setup: vec![("doc.open", json!({"path": "a.txt"}))],
                params: json!({"doc": 1, "query": "(\\w)(\\d)", "regex": true,
                    "replacement": "$2$1", "all": true}),
                result: json!({"replaced": 2, "len_chars": 5,
                    "ranges": [{"start": 0, "end": 2}, {"start": 3, "end": 5}],
                    "truncated": false}),
            },
            Example {
                description: "the first literal match at or after an offset",
                initial_text: "",
                files: vec![("a.txt", "aaa b aa")],
                setup: vec![("doc.open", json!({"path": "a.txt"}))],
                params: json!({"doc": 1, "query": "aa", "replacement": "x", "from": 1}),
                result: json!({"replaced": 1, "len_chars": 7,
                    "ranges": [{"start": 6, "end": 7}], "truncated": false}),
            },
        ]
    }

    fn run(ctx: &mut Context, p: ReplaceParams) -> Result<ReplaceResult, CommandError> {
        let buffer = ctx.doc(p.doc)?.buffer();
        let query = compile(&p.query, p.regex, p.case_sensitive, p.whole_word)?;
        query
            .check_template(&p.replacement)
            .map_err(|e| template_error(&e))?;
        let all = p.all.unwrap_or(false);
        let len = buffer.len_chars();
        let from = p.from.unwrap_or(0);
        if !all && from > len {
            return Err(CommandError::rejected(
                RejectReason::OutOfRange,
                format!(
                    "invalid from: char index {from} is beyond the end of the text ({len} chars)"
                ),
            ));
        }
        let text = buffer.text();
        let plan = if all {
            query.replace_all(&text, &p.replacement)
        } else {
            query
                .replace_first(&text, from, &p.replacement)
                .map(|first| first.into_iter().collect())
        }
        .map_err(|e| template_error(&e))?;
        drop(text);

        let limit = p.limit.unwrap_or(DEFAULT_LIMIT);
        // Chars inserted and removed by the replacements before the current one.
        let (mut added, mut removed) = (0usize, 0usize);
        let mut ranges = Vec::new();
        for (m, with) in plan.iter().take(limit) {
            let n = with.chars().count();
            let start = m.start - removed + added;
            ranges.push(Span {
                start,
                end: start + n,
            });
            added += n;
            removed += m.end - m.start;
        }
        if !plan.is_empty() {
            let edits: Vec<(Range<usize>, &str)> = plan
                .iter()
                .map(|(m, with)| (m.start..m.end, with.as_str()))
                .collect();
            // The matches of one search are sorted, disjoint and in bounds: a valid batch.
            ctx.doc_mut(p.doc)?
                .buffer_mut()
                .apply_edits(&edits)
                .map_err(|e| CommandError::rejected(RejectReason::Other, e.to_string()))?;
        }
        Ok(ReplaceResult {
            replaced: plan.len(),
            len_chars: ctx.doc(p.doc)?.buffer().len_chars(),
            ranges,
            truncated: plan.len() > limit,
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

    fn err(r: &Registry, ctx: &mut Context, name: &str, params: Value) -> (String, String) {
        let e = r.execute(ctx, name, params).unwrap_err().to_json();
        (
            e["code"].as_str().unwrap_or_default().to_owned(),
            e["reason"].as_str().unwrap_or_default().to_owned(),
        )
    }

    fn text(r: &Registry, ctx: &mut Context) -> String {
        ok(r, ctx, "doc.text", json!({"doc": 1}))["text"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    #[test]
    fn replace_all_and_undo() {
        let (r, mut ctx) = setup("baaab");
        let v = ok(
            &r,
            &mut ctx,
            "search.replace",
            json!({"doc": 1, "query": "a*", "regex": true, "replacement": "-", "all": true}),
        );
        assert_eq!(v["replaced"], 3);
        assert_eq!(text(&r, &mut ctx), "-b-b-");
        ok(&r, &mut ctx, "doc.undo", json!({"doc": 1}));
        assert_eq!(text(&r, &mut ctx), "baaab");
    }

    #[test]
    fn kinds_of_refusals() {
        let (r, mut ctx) = setup("a1 b2");
        let pattern = ("rejected".to_owned(), "invalid_pattern".to_owned());
        for p in [
            json!({"doc": 1, "query": "", "replacement": ""}),
            json!({"doc": 1, "query": "(\\w)", "regex": true, "replacement": "$3"}),
            json!({"doc": 1, "query": "(a)".repeat(101), "regex": true, "replacement": ""}),
            json!({"doc": 1, "query": "a".repeat(1001), "replacement": ""}),
        ] {
            assert_eq!(err(&r, &mut ctx, "search.replace", p), pattern);
        }
        assert_eq!(
            err(
                &r,
                &mut ctx,
                "search.replace",
                json!({"doc": 1, "query": "a", "replacement": "", "from": 6})
            ),
            ("rejected".to_owned(), "out_of_range".to_owned())
        );
        assert_eq!(text(&r, &mut ctx), "a1 b2");
    }

    /// q1 (D-116): every optional param publishes its true default, never `null`.
    #[test]
    fn optional_params_publish_their_defaults() {
        let r = standard_registry().unwrap();
        let described = r.describe();
        let props = |name: &str| {
            described["commands"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["name"] == name)
                .unwrap()["params_schema"]["properties"]
                .clone()
        };
        for name in ["search.find", "search.replace"] {
            let p = props(name);
            assert_eq!(p["regex"]["default"], json!(false), "{name}");
            assert_eq!(p["case_sensitive"]["default"], json!(true), "{name}");
            assert_eq!(p["whole_word"]["default"], json!(false), "{name}");
            assert_eq!(p["limit"]["default"], json!(10_000), "{name}");
        }
        let p = props("search.replace");
        assert_eq!(p["all"]["default"], json!(false));
        assert_eq!(p["from"]["default"], json!(0));
        assert!(p["replacement"].get("default").is_none());
    }

    #[test]
    fn no_match_keeps_the_typing_group() {
        let (r, mut ctx) = setup("abc");
        ok(
            &r,
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": 3, "text": "d"}),
        );
        let v = ok(
            &r,
            &mut ctx,
            "search.replace",
            json!({"doc": 1, "query": "z", "replacement": "y", "all": true}),
        );
        assert_eq!(
            v,
            json!({"replaced": 0, "len_chars": 4, "ranges": [], "truncated": false})
        );
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
    fn selection_maps_through_each_replacement() {
        let (r, mut ctx) = setup("xa xa x");
        ok(
            &r,
            &mut ctx,
            "cursor.set",
            json!({"doc": 1, "anchor": 4, "head": 7}),
        );
        ok(
            &r,
            &mut ctx,
            "search.replace",
            json!({"doc": 1, "query": "a", "replacement": "AAA", "all": true}),
        );
        let c = ok(&r, &mut ctx, "cursor.get", json!({"doc": 1}));
        // Last edit first: 4 is the start of the second `a` (4..5) and stays, 7 -> 9; then
        // the first `a` (1..2) shifts both by 2.
        assert_eq!(
            (c["anchor"].as_u64(), c["head"].as_u64()),
            (Some(6), Some(11))
        );
    }

    #[test]
    fn reply_ranges_are_limited() {
        let (r, mut ctx) = setup("x x x x");
        let v = ok(
            &r,
            &mut ctx,
            "search.replace",
            json!({"doc": 1, "query": "x", "replacement": "", "all": true, "limit": 2}),
        );
        assert_eq!(
            v,
            json!({"replaced": 4, "len_chars": 3,
                "ranges": [{"start": 0, "end": 0}, {"start": 1, "end": 1}], "truncated": true})
        );
        let schema = r.describe();
        let find = schema["commands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "search.find")
            .unwrap();
        let limit = &find["params_schema"]["properties"]["limit"];
        assert_eq!(
            (limit["minimum"].as_u64(), limit["maximum"].as_u64()),
            (Some(1), Some(100_000))
        );
    }
}
