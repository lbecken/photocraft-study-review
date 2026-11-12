//! The registry: typed commands, schema generation, validation and dispatch.

use std::collections::BTreeMap;
use std::fmt;

use kiln_text::Buffer;
use schemars::JsonSchema;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

/// The state commands act on.
///
/// Placeholder for iteration 0: a single scratch buffer. It becomes the open-documents state of
/// `kiln-workspace` when that crate gains documents; commands keep the same calling shape.
/// Only read access is public; mutation happens inside commands.
#[derive(Clone, Debug, Default)]
pub struct Context {
    buffer: Buffer,
}

impl Context {
    /// An empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// A context whose scratch buffer holds `text`.
    pub fn with_text(text: &str) -> Self {
        Self {
            buffer: Buffer::from_text(text),
        }
    }

    /// Read access to the scratch buffer.
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub(crate) fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }
}

/// An executable example of a command: run on a fresh [`Context::with_text`]`(initial_text)`
/// with `params`, it must return exactly `result`. Examples are checked by `cargo xtask schema`.
#[derive(Clone, Debug, Serialize)]
pub struct Example {
    /// What the example shows.
    pub description: &'static str,
    /// Text of the scratch buffer before the command runs.
    pub initial_text: &'static str,
    /// Params passed to the command.
    pub params: Value,
    /// The exact expected result.
    pub result: Value,
}

/// A named command with typed params and result.
pub trait Command {
    /// Unique dotted name, e.g. `buffer.insert`.
    const NAME: &'static str;
    /// One-paragraph description for humans and the command palette.
    const DOC: &'static str;
    /// Params; their JSON Schema is generated from this type.
    type Params: DeserializeOwned + JsonSchema;
    /// Result; its JSON Schema is generated from this type.
    type Output: Serialize + JsonSchema;

    /// At least one executable example.
    fn examples() -> Vec<Example>;

    /// Execute against `ctx`. On error the context must be unchanged.
    fn run(ctx: &mut Context, params: Self::Params) -> Result<Self::Output, CommandError>;
}

/// Why a command invocation failed. Serialized as `{"code": …, "message": …, "details": […]}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum CommandError {
    /// No command has this name.
    UnknownCommand {
        /// Human-readable message.
        message: String,
    },
    /// Params did not validate against the command's params schema.
    InvalidParams {
        /// Human-readable message.
        message: String,
        /// One entry per validation error.
        details: Vec<String>,
    },
    /// The command ran and refused the request (e.g. an offset out of bounds).
    Rejected {
        /// Human-readable message.
        message: String,
    },
    /// A bug: the command produced a result that does not match its own schema.
    Internal {
        /// Human-readable message.
        message: String,
    },
}

impl CommandError {
    /// A [`CommandError::Rejected`] with `message`.
    pub fn rejected(message: impl Into<String>) -> Self {
        CommandError::Rejected {
            message: message.into(),
        }
    }

    /// The error as JSON.
    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|_| json!({"code": "internal"}))
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::UnknownCommand { message }
            | CommandError::Rejected { message }
            | CommandError::Internal { message } => f.write_str(message),
            CommandError::InvalidParams { message, details } => {
                write!(f, "{message}: {}", details.join("; "))
            }
        }
    }
}

impl std::error::Error for CommandError {}

/// A registration was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegistryError(pub String);

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RegistryError {}

type Runner = Box<dyn Fn(&mut Context, Value) -> Result<Value, CommandError> + Send + Sync>;

struct Entry {
    doc: &'static str,
    params_schema: Value,
    result_schema: Value,
    params_validator: jsonschema::Validator,
    result_validator: jsonschema::Validator,
    examples: Vec<Example>,
    run: Runner,
}

/// The set of registered commands.
#[derive(Default)]
pub struct Registry {
    entries: BTreeMap<&'static str, Entry>,
}

impl fmt::Debug for Registry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.entries.keys()).finish()
    }
}

fn validation_errors(validator: &jsonschema::Validator, value: &Value) -> Vec<String> {
    validator
        .iter_errors(value)
        .map(|e| {
            let at = e.instance_path().to_string();
            if at.is_empty() {
                e.to_string()
            } else {
                format!("{at}: {e}")
            }
        })
        .collect()
}

fn valid_name(name: &str) -> bool {
    let mut parts = name.split('.');
    let well_formed = |p: &str| {
        !p.is_empty()
            && p.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    };
    parts.clone().count() >= 2 && parts.all(well_formed)
}

impl Registry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `C`. Refuses duplicate or malformed names, empty docs, missing examples and
    /// schemas that do not compile.
    pub fn register<C: Command + 'static>(&mut self) -> Result<(), RegistryError> {
        let name = C::NAME;
        if !valid_name(name) {
            return Err(RegistryError(format!(
                "command name `{name}` must be dotted lowercase, e.g. `group.verb`"
            )));
        }
        if self.entries.contains_key(name) {
            return Err(RegistryError(format!("duplicate command name `{name}`")));
        }
        if C::DOC.trim().is_empty() {
            return Err(RegistryError(format!("command `{name}` has an empty doc")));
        }
        let examples = C::examples();
        if examples.is_empty() {
            return Err(RegistryError(format!("command `{name}` has no example")));
        }
        let params_schema = schemars::schema_for!(C::Params).to_value();
        let result_schema = schemars::schema_for!(C::Output).to_value();
        let compile = |schema: &Value, what: &str| {
            jsonschema::validator_for(schema).map_err(|e| {
                RegistryError(format!(
                    "command `{name}`: {what} schema does not compile: {e}"
                ))
            })
        };
        let params_validator = compile(&params_schema, "params")?;
        let result_validator = compile(&result_schema, "result")?;
        let run: Runner = Box::new(|ctx, params| {
            let typed: C::Params =
                serde_json::from_value(params).map_err(|e| CommandError::InvalidParams {
                    message: format!("params for `{}` do not deserialize", C::NAME),
                    details: vec![e.to_string()],
                })?;
            let output = C::run(ctx, typed)?;
            serde_json::to_value(output).map_err(|e| CommandError::Internal {
                message: format!("result of `{}` does not serialize: {e}", C::NAME),
            })
        });
        self.entries.insert(
            name,
            Entry {
                doc: C::DOC,
                params_schema,
                result_schema,
                params_validator,
                result_validator,
                examples,
                run,
            },
        );
        Ok(())
    }

    /// Registered command names, sorted.
    pub fn names(&self) -> Vec<&'static str> {
        self.entries.keys().copied().collect()
    }

    /// The doc string of `name`, if registered.
    pub fn doc(&self, name: &str) -> Option<&'static str> {
        self.entries.get(name).map(|e| e.doc)
    }

    /// Execute `name` with JSON `params`: validate params against the params schema, run,
    /// then validate the result against the result schema.
    pub fn execute(
        &self,
        ctx: &mut Context,
        name: &str,
        params: Value,
    ) -> Result<Value, CommandError> {
        let entry = self
            .entries
            .get(name)
            .ok_or_else(|| CommandError::UnknownCommand {
                message: format!("unknown command `{name}`"),
            })?;
        let errors = validation_errors(&entry.params_validator, &params);
        if !errors.is_empty() {
            return Err(CommandError::InvalidParams {
                message: format!("invalid params for `{name}`"),
                details: errors,
            });
        }
        let result = (entry.run)(ctx, params)?;
        let errors = validation_errors(&entry.result_validator, &result);
        if !errors.is_empty() {
            return Err(CommandError::Internal {
                message: format!(
                    "result of `{name}` violates its schema: {}",
                    errors.join("; ")
                ),
            });
        }
        Ok(result)
    }

    /// Machine-readable description of every command (the content of `docs/commands.json`).
    pub fn describe(&self) -> Value {
        let commands: Vec<Value> = self
            .entries
            .iter()
            .map(|(name, e)| {
                json!({
                    "name": name,
                    "doc": e.doc,
                    "params_schema": e.params_schema,
                    "result_schema": e.result_schema,
                    "examples": e.examples,
                })
            })
            .collect();
        json!({ "commands": commands })
    }

    /// Check every command's contract: each example's params validate, the example runs on a
    /// fresh context and returns exactly the expected result, which validates against the result
    /// schema. Returns one line per problem; empty means sound. An empty registry is a problem.
    pub fn audit(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.entries.is_empty() {
            problems.push("no commands are registered".to_owned());
        }
        for (name, e) in &self.entries {
            for (i, ex) in e.examples.iter().enumerate() {
                let label = format!("`{name}` example {i} ({})", ex.description);
                for err in validation_errors(&e.params_validator, &ex.params) {
                    problems.push(format!("{label}: params invalid: {err}"));
                }
                for err in validation_errors(&e.result_validator, &ex.result) {
                    problems.push(format!("{label}: expected result invalid: {err}"));
                }
                let mut ctx = Context::with_text(ex.initial_text);
                match self.execute(&mut ctx, name, ex.params.clone()) {
                    Ok(actual) if actual == ex.result => {}
                    Ok(actual) => problems.push(format!(
                        "{label}: returned {actual}, example says {}",
                        ex.result
                    )),
                    Err(err) => problems.push(format!("{label}: failed: {err}")),
                }
            }
        }
        problems
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    struct EchoParams {
        n: u32,
    }

    #[derive(Serialize, JsonSchema)]
    struct EchoOut {
        n: u32,
    }

    struct Echo;
    impl Command for Echo {
        const NAME: &'static str = "test.echo";
        const DOC: &'static str = "Echo n.";
        type Params = EchoParams;
        type Output = EchoOut;
        fn examples() -> Vec<Example> {
            vec![Example {
                description: "echo",
                initial_text: "",
                params: json!({"n": 1}),
                result: json!({"n": 1}),
            }]
        }
        fn run(_: &mut Context, p: EchoParams) -> Result<EchoOut, CommandError> {
            Ok(EchoOut { n: p.n })
        }
    }

    struct Liar;
    impl Command for Liar {
        const NAME: &'static str = "test.liar";
        const DOC: &'static str = "Example claims the wrong result.";
        type Params = EchoParams;
        type Output = EchoOut;
        fn examples() -> Vec<Example> {
            vec![Example {
                description: "wrong",
                initial_text: "",
                params: json!({"n": 1}),
                result: json!({"n": 2}),
            }]
        }
        fn run(_: &mut Context, p: EchoParams) -> Result<EchoOut, CommandError> {
            Ok(EchoOut { n: p.n })
        }
    }

    struct Undocumented;
    impl Command for Undocumented {
        const NAME: &'static str = "test.undocumented";
        const DOC: &'static str = " ";
        type Params = EchoParams;
        type Output = EchoOut;
        fn examples() -> Vec<Example> {
            Echo::examples()
        }
        fn run(_: &mut Context, p: EchoParams) -> Result<EchoOut, CommandError> {
            Ok(EchoOut { n: p.n })
        }
    }

    #[test]
    fn rejects_duplicate_names() {
        let mut r = Registry::new();
        r.register::<Echo>().unwrap();
        let err = r.register::<Echo>().unwrap_err();
        assert!(err.0.contains("duplicate"), "{err}");
    }

    #[test]
    fn rejects_empty_doc() {
        let mut r = Registry::new();
        assert!(r.register::<Undocumented>().is_err());
    }

    #[test]
    fn rejects_params_that_violate_schema() {
        let mut r = Registry::new();
        r.register::<Echo>().unwrap();
        let mut ctx = Context::new();
        for bad in [
            json!({}),
            json!({"n": -1}),
            json!({"n": 1, "extra": true}),
            json!("x"),
        ] {
            let err = r.execute(&mut ctx, "test.echo", bad.clone()).unwrap_err();
            assert!(
                matches!(err, CommandError::InvalidParams { .. }),
                "{bad} gave {err:?}"
            );
        }
    }

    #[test]
    fn unknown_command_is_an_error() {
        let r = Registry::new();
        let err = r
            .execute(&mut Context::new(), "nope.nope", json!({}))
            .unwrap_err();
        assert_eq!(err.to_json()["code"], "unknown_command");
    }

    #[test]
    fn audit_catches_wrong_example_and_empty_registry() {
        assert_eq!(Registry::new().audit(), vec!["no commands are registered"]);
        let mut r = Registry::new();
        r.register::<Liar>().unwrap();
        let problems = r.audit();
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("example says"), "{problems:?}");
    }

    #[test]
    fn describe_lists_schemas_and_examples() {
        let mut r = Registry::new();
        r.register::<Echo>().unwrap();
        let d = r.describe();
        let c = &d["commands"][0];
        assert_eq!(c["name"], "test.echo");
        assert_eq!(c["params_schema"]["additionalProperties"], false);
        assert_eq!(c["examples"][0]["params"]["n"], 1);
    }

    #[test]
    fn name_rules() {
        assert!(valid_name("buffer.insert"));
        assert!(valid_name("ws.find-in-files"));
        assert!(!valid_name("insert"));
        assert!(!valid_name("Buffer.insert"));
        assert!(!valid_name("buffer..insert"));
    }
}
