//! The `buffer.*` command group.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::registry::{Command, CommandError, Context, Example, Registry, RegistryError};

/// Register the `buffer.*` commands.
pub(crate) fn register(registry: &mut Registry) -> Result<(), RegistryError> {
    registry.register::<Insert>()?;
    registry.register::<Text>()?;
    Ok(())
}

/// Params of `buffer.insert`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct InsertParams {
    /// Char index (Unicode scalar values, not bytes) before which to insert; equal to the
    /// buffer length to append.
    offset: usize,
    /// Text to insert.
    text: String,
}

/// Result of `buffer.insert`.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct InsertResult {
    /// Buffer length in chars after the insert.
    len_chars: usize,
}

pub(crate) struct Insert;

impl Command for Insert {
    const NAME: &'static str = "buffer.insert";
    const DOC: &'static str = "Insert text at a char offset of the scratch buffer.";
    type Params = InsertParams;
    type Output = InsertResult;
    const MUTATING: bool = true;

    fn examples() -> Vec<Example> {
        vec![
            Example {
                description: "append to an empty buffer",
                initial_text: "",
                files: vec![],
                setup: vec![],
                params: json!({"offset": 0, "text": "fn main() {}"}),
                result: json!({"len_chars": 12}),
            },
            Example {
                description: "offsets count chars, not bytes",
                initial_text: "héllo",
                files: vec![],
                setup: vec![],
                params: json!({"offset": 2, "text": "X"}),
                result: json!({"len_chars": 6}),
            },
        ]
    }

    fn run(ctx: &mut Context, p: InsertParams) -> Result<InsertResult, CommandError> {
        let buffer = ctx.buffer_mut();
        buffer
            .insert(p.offset, &p.text)
            .map_err(|e| CommandError::rejected((&e).into(), e.to_string()))?;
        Ok(InsertResult {
            len_chars: buffer.len_chars(),
        })
    }
}

/// Params of `buffer.text` (none).
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct TextParams {}

/// Result of `buffer.text`.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct TextResult {
    /// The whole buffer text.
    text: String,
    /// Its length in chars.
    len_chars: usize,
}

pub(crate) struct Text;

impl Command for Text {
    const NAME: &'static str = "buffer.text";
    const DOC: &'static str = "Return the whole text of the scratch buffer.";
    type Params = TextParams;
    type Output = TextResult;
    const MUTATING: bool = false;

    fn examples() -> Vec<Example> {
        vec![Example {
            description: "read the buffer",
            initial_text: "let x = 1;",
            files: vec![],
            setup: vec![],
            params: json!({}),
            result: json!({"text": "let x = 1;", "len_chars": 10}),
        }]
    }

    fn run(ctx: &mut Context, _: TextParams) -> Result<TextResult, CommandError> {
        let buffer = ctx.buffer();
        Ok(TextResult {
            text: buffer.text(),
            len_chars: buffer.len_chars(),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{Context, standard_registry};
    use serde_json::json;

    #[test]
    fn insert_then_text_round_trip() {
        let r = standard_registry().unwrap();
        let mut ctx = Context::new();
        let out = r
            .execute(
                &mut ctx,
                "buffer.insert",
                json!({"offset": 0, "text": "ab"}),
            )
            .unwrap();
        assert_eq!(out, json!({"len_chars": 2}));
        r.execute(&mut ctx, "buffer.insert", json!({"offset": 1, "text": "é"}))
            .unwrap();
        let text = r.execute(&mut ctx, "buffer.text", json!({})).unwrap();
        assert_eq!(text, json!({"text": "aéb", "len_chars": 3}));
    }

    #[test]
    fn insert_out_of_bounds_is_rejected_and_state_unchanged() {
        let r = standard_registry().unwrap();
        let mut ctx = Context::with_text("ab");
        let err = r
            .execute(&mut ctx, "buffer.insert", json!({"offset": 5, "text": "x"}))
            .unwrap_err();
        assert_eq!(err.to_json()["code"], "rejected");
        assert_eq!(ctx.buffer().text(), "ab");
    }

    #[test]
    fn insert_rejects_missing_text_param() {
        let r = standard_registry().unwrap();
        let err = r
            .execute(&mut Context::new(), "buffer.insert", json!({"offset": 0}))
            .unwrap_err();
        assert_eq!(err.to_json()["code"], "invalid_params");
    }

    #[test]
    fn standard_registry_passes_its_own_audit() {
        let r = standard_registry().unwrap();
        let buffer_names: Vec<_> = r
            .names()
            .into_iter()
            .filter(|n| n.starts_with("buffer."))
            .collect();
        assert_eq!(buffer_names, vec!["buffer.insert", "buffer.text"]);
        assert_eq!(r.audit(), Vec::<String>::new());
    }
}
