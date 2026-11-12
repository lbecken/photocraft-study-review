//! The registry: typed commands, schema generation, validation and dispatch.

use std::collections::BTreeMap;
use std::fmt;

use kiln_text::Buffer;
use kiln_workspace::{Document, MemoryFs, PathError, RelPath, Vfs};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// The state commands act on: a VFS, the documents opened from it, a scratch buffer (for the
/// `buffer.*` commands) and the log of successfully executed commands.
///
/// Only read access is public; mutation happens inside commands.
#[derive(Debug)]
pub struct Context {
    buffer: Buffer,
    vfs: Box<dyn Vfs>,
    docs: BTreeMap<u64, Document>,
    /// The id the next opened document gets (ids start at 1 and are never reused).
    next_doc: u64,
    log: Vec<LogEntry>,
}

impl Default for Context {
    fn default() -> Self {
        Self::with_vfs(Box::new(MemoryFs::new()))
    }
}

impl Context {
    /// An empty context: empty scratch buffer, empty [`MemoryFs`], no documents, empty log.
    pub fn new() -> Self {
        Self::default()
    }

    /// A context whose scratch buffer holds `text` (empty [`MemoryFs`]).
    pub fn with_text(text: &str) -> Self {
        Self {
            buffer: Buffer::from_text(text),
            ..Self::default()
        }
    }

    /// A context over a [`MemoryFs`] holding `files` (path, bytes). Fails on an invalid path.
    pub fn with_files(files: &[(&str, &[u8])]) -> Result<Self, PathError> {
        let mut fs = MemoryFs::new();
        for (path, bytes) in files {
            fs.insert(&RelPath::new(path)?, bytes);
        }
        Ok(Self::with_vfs(Box::new(fs)))
    }

    /// A context over `vfs` (empty scratch buffer, no documents, empty log).
    pub fn with_vfs(vfs: Box<dyn Vfs>) -> Self {
        Self {
            buffer: Buffer::new(),
            vfs,
            docs: BTreeMap::new(),
            next_doc: 1,
            log: Vec::new(),
        }
    }

    /// Read access to the scratch buffer.
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub(crate) fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    /// Read access to the VFS.
    pub fn vfs(&self) -> &dyn Vfs {
        self.vfs.as_ref()
    }

    /// Every successfully executed command, in execution order.
    pub fn log(&self) -> &[LogEntry] {
        &self.log
    }

    /// The open documents by id, in increasing id order.
    pub(crate) fn docs(&self) -> &BTreeMap<u64, Document> {
        &self.docs
    }

    /// The open document `id`, or a `rejected` error.
    pub(crate) fn doc(&self, id: u64) -> Result<&Document, CommandError> {
        self.docs.get(&id).ok_or_else(|| no_such_doc(id))
    }

    /// The open document `id` for editing, or a `rejected` error.
    pub(crate) fn doc_mut(&mut self, id: u64) -> Result<&mut Document, CommandError> {
        self.docs.get_mut(&id).ok_or_else(|| no_such_doc(id))
    }

    /// The open document `id` and the VFS, both mutable (for save).
    pub(crate) fn doc_and_vfs_mut(
        &mut self,
        id: u64,
    ) -> Result<(&mut Document, &mut dyn Vfs), CommandError> {
        let doc = self.docs.get_mut(&id).ok_or_else(|| no_such_doc(id))?;
        Ok((doc, self.vfs.as_mut()))
    }

    /// Add `doc` under the next id and return that id.
    pub(crate) fn add_doc(&mut self, doc: Document) -> u64 {
        let id = self.next_doc;
        self.next_doc += 1;
        self.docs.insert(id, doc);
        id
    }

    /// Remove document `id`, or a `rejected` error.
    pub(crate) fn remove_doc(&mut self, id: u64) -> Result<Document, CommandError> {
        self.docs.remove(&id).ok_or_else(|| no_such_doc(id))
    }
}

fn no_such_doc(id: u64) -> CommandError {
    CommandError::rejected(format!("no open document {id}"))
}

/// One successfully executed command, in execution order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogEntry {
    /// The command name.
    pub name: String,
    /// The params it was executed with, as given.
    pub params: Value,
    /// The command's [`Command::MUTATING`] flag.
    pub mutating: bool,
}

/// [`Registry::replay`] stopped: entry `index` of the log failed with `error`.
#[derive(Clone, Debug, PartialEq)]
pub struct ReplayError {
    /// Index of the failing log entry.
    pub index: usize,
    /// Why it failed.
    pub error: CommandError,
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "replay failed at log entry {}: {}",
            self.index, self.error
        )
    }
}

impl std::error::Error for ReplayError {}

/// An executable example of a command: on a fresh [`Context::with_text`]`(initial_text)` whose
/// [`MemoryFs`] holds `files`, each `setup` command is executed (each must succeed), then the
/// command with `params`; it must return exactly `result`. Examples are checked by
/// `cargo xtask schema`.
#[derive(Clone, Debug, Serialize)]
pub struct Example {
    /// What the example shows.
    pub description: &'static str,
    /// Text of the scratch buffer before the command runs.
    pub initial_text: &'static str,
    /// Initial files of the context's [`MemoryFs`]: (path, UTF-8 content).
    pub files: Vec<(&'static str, &'static str)>,
    /// Commands (name, params) executed before the example; each must succeed.
    pub setup: Vec<(&'static str, Value)>,
    /// Params passed to the command.
    pub params: Value,
    /// The exact expected result.
    pub result: Value,
}

/// The fresh context an example runs on: scratch buffer `initial_text`, MemoryFs `files`.
fn example_context(ex: &Example) -> Result<Context, PathError> {
    let mut fs = MemoryFs::new();
    for (path, content) in &ex.files {
        fs.insert(&RelPath::new(path)?, content.as_bytes());
    }
    Ok(Context {
        buffer: Buffer::from_text(ex.initial_text),
        ..Context::with_vfs(Box::new(fs))
    })
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
    /// True iff the command can change state (documents, buffer, files); false for read-only
    /// queries.
    const MUTATING: bool;

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
    mutating: bool,
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
                mutating: C::MUTATING,
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
    /// then validate the result against the result schema. Only a successful command is
    /// appended to the context's log; a failed one leaves the context unchanged.
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
        let logged = params.clone();
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
        ctx.log.push(LogEntry {
            name: name.to_owned(),
            params: logged,
            mutating: entry.mutating,
        });
        Ok(result)
    }

    /// Execute every entry of `log` in order on `ctx` and return their results; stop at the
    /// first error (entries before it stay applied).
    pub fn replay(&self, ctx: &mut Context, log: &[LogEntry]) -> Result<Vec<Value>, ReplayError> {
        log.iter()
            .enumerate()
            .map(|(index, e)| {
                self.execute(ctx, &e.name, e.params.clone())
                    .map_err(|error| ReplayError { index, error })
            })
            .collect()
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
                    "mutating": e.mutating,
                    "params_schema": e.params_schema,
                    "result_schema": e.result_schema,
                    "examples": e.examples,
                })
            })
            .collect();
        json!({ "commands": commands })
    }

    /// Check every command's contract: each example's params validate, the example runs on a
    /// fresh context (scratch buffer `initial_text`, MemoryFs `files`, then every `setup`
    /// command, each of which must succeed) and returns exactly the expected result, which
    /// validates against the result schema. Returns one line per problem; empty means sound. An empty registry is a problem.
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
                let mut ctx = match example_context(ex) {
                    Ok(ctx) => ctx,
                    Err(err) => {
                        problems.push(format!("{label}: bad file path: {err}"));
                        continue;
                    }
                };
                let mut setup_ok = true;
                for (k, (setup_name, setup_params)) in ex.setup.iter().enumerate() {
                    if let Err(err) = self.execute(&mut ctx, setup_name, setup_params.clone()) {
                        problems.push(format!("{label}: setup {k} (`{setup_name}`) failed: {err}"));
                        setup_ok = false;
                        break;
                    }
                }
                if !setup_ok {
                    continue;
                }
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
        const MUTATING: bool = false;
        fn examples() -> Vec<Example> {
            vec![Example {
                description: "echo",
                initial_text: "",
                files: vec![],
                setup: vec![],
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
        const MUTATING: bool = false;
        fn examples() -> Vec<Example> {
            vec![Example {
                description: "wrong",
                initial_text: "",
                files: vec![],
                setup: vec![],
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
        const MUTATING: bool = false;
        fn examples() -> Vec<Example> {
            Echo::examples()
        }
        fn run(_: &mut Context, p: EchoParams) -> Result<EchoOut, CommandError> {
            Ok(EchoOut { n: p.n })
        }
    }

    struct NeedsSetup;
    impl Command for NeedsSetup {
        const NAME: &'static str = "test.needs-setup";
        const DOC: &'static str = "Examples with a failing setup and a bad file path.";
        const MUTATING: bool = true;
        type Params = EchoParams;
        type Output = EchoOut;
        fn examples() -> Vec<Example> {
            vec![
                Example {
                    description: "setup fails",
                    initial_text: "",
                    files: vec![("ok.txt", "x")],
                    setup: vec![("test.echo", json!({"n": "not a number"}))],
                    params: json!({"n": 1}),
                    result: json!({"n": 1}),
                },
                Example {
                    description: "bad path",
                    initial_text: "",
                    files: vec![("../x", "x")],
                    setup: vec![],
                    params: json!({"n": 1}),
                    result: json!({"n": 1}),
                },
                Example {
                    description: "sound",
                    initial_text: "",
                    files: vec![("ok.txt", "x")],
                    setup: vec![("test.echo", json!({"n": 3}))],
                    params: json!({"n": 1}),
                    result: json!({"n": 1}),
                },
            ]
        }
        fn run(_: &mut Context, p: EchoParams) -> Result<EchoOut, CommandError> {
            Ok(EchoOut { n: p.n })
        }
    }

    #[test]
    fn audit_reports_failing_setup_and_bad_example_paths() {
        let mut r = Registry::new();
        r.register::<Echo>().unwrap();
        r.register::<NeedsSetup>().unwrap();
        let problems = r.audit();
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(
            problems[0].contains("setup 0 (`test.echo`) failed"),
            "{problems:?}"
        );
        assert!(problems[1].contains("bad file path"), "{problems:?}");
    }

    #[test]
    fn execute_logs_only_successes_with_mutating_flag() {
        let mut r = Registry::new();
        r.register::<Echo>().unwrap();
        r.register::<NeedsSetup>().unwrap();
        let mut ctx = Context::new();
        r.execute(&mut ctx, "test.echo", json!({"n": 1})).unwrap();
        assert!(r.execute(&mut ctx, "test.echo", json!({})).is_err());
        assert!(r.execute(&mut ctx, "nope.nope", json!({})).is_err());
        r.execute(&mut ctx, "test.needs-setup", json!({"n": 2}))
            .unwrap();
        assert_eq!(
            ctx.log(),
            [
                LogEntry {
                    name: "test.echo".into(),
                    params: json!({"n": 1}),
                    mutating: false
                },
                LogEntry {
                    name: "test.needs-setup".into(),
                    params: json!({"n": 2}),
                    mutating: true
                },
            ]
        );
        let d = r.describe();
        assert_eq!(d["commands"][0]["mutating"], false);
        assert_eq!(d["commands"][1]["mutating"], true);
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
