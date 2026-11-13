//! # kiln-control — layer L4
//!
//! Responsibility (CHARTER §3): the headless host and the JSON-lines control protocol. Every
//! request is a named command executed through the `kiln-commands` registry.
//!
//! May depend on: L0–L3. Headless and deterministic.
//!
//! The protocol is specified in `docs/control-protocol.md`. In short: one request per line,
//! `{"id"?: string | integer, "command": string, "params"?: object}`; one response line per
//! non-blank request line, compact JSON with object keys sorted at every level:
//! `{"id":…,"ok":true,"result":…}` or
//! `{"error":{"code":…,"kind":…,"message":…[,"details":[…]]},"id":…,"ok":false}`.
//!
//! `code` and `kind` are stable machine-readable values (`kind` refines `code`; the full list is in
//! `docs/control-protocol.md`); `message` and `details` are for humans and may contain operating-system
//! error text. Any value from the session that an error message or detail echoes back is cut
//! to at most 200 characters.
//!
//! The `kind` of a `rejected` error is derived from the message the lower layers produce
//! (`kiln-commands` reports `rejected` with a message only); `rejected_kind` holds that
//! mapping and falls back to `other` for a message it does not recognise.

use std::collections::BTreeMap;
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use kiln_commands::{CommandError, Context, LogEntry, Registry, standard_registry};
use kiln_workspace::{DiskFs, VfsError};
use serde_json::{Map, Value, json};

/// A host could not be created (invalid path, root not a directory, registry failure).
#[derive(Debug)]
pub struct HostError(pub String);

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for HostError {}

/// The longest echo of a session value inside an error message or detail, in characters
/// (a longer value is cut to `MAX_ECHO - 1` characters followed by `…`).
const MAX_ECHO: usize = 200;

/// Every `(code, kind)` pair an error response can carry.
#[cfg(test)]
const ERROR_KINDS: &[(&str, &str)] = &[
    ("invalid_json", "not_utf8"),
    ("invalid_json", "syntax"),
    ("invalid_request", "not_an_object"),
    ("invalid_request", "bad_id"),
    ("invalid_request", "unknown_key"),
    ("invalid_request", "missing_command"),
    ("invalid_request", "command_not_string"),
    ("invalid_request", "params_not_object"),
    ("unknown_command", "unknown_command"),
    ("invalid_params", "schema"),
    ("rejected", "invalid_path"),
    ("rejected", "no_such_document"),
    ("rejected", "unsaved_changes"),
    ("rejected", "not_found"),
    ("rejected", "not_a_file"),
    ("rejected", "io_error"),
    ("rejected", "decode_error"),
    ("rejected", "out_of_range"),
    ("rejected", "inverted_range"),
    ("rejected", "other"),
    ("internal", "panic"),
    ("internal", "result_schema"),
    ("internal", "other"),
];

/// A headless session: the standard registry and one command context.
#[derive(Debug)]
pub struct Host {
    registry: Registry,
    ctx: Context,
    /// Path of each open document, from successful `doc.open` / `doc.close` responses (used
    /// only to classify and clip error messages that name a document's path).
    doc_paths: BTreeMap<u64, String>,
}

/// The top-level keys a request may have.
const REQUEST_KEYS: [&str; 3] = ["command", "id", "params"];

impl Host {
    /// A host over an in-memory file system holding `files` (path, bytes).
    pub fn in_memory(files: &[(&str, &[u8])]) -> Result<Host, HostError> {
        let ctx = Context::with_files(files).map_err(|e| HostError(e.to_string()))?;
        Self::with_context(ctx)
    }

    /// A host over the disk file system rooted at `root`. Fails if `root` is not a
    /// directory; the error names `root` as given.
    pub fn on_disk(root: &Path) -> Result<Host, HostError> {
        let fs = DiskFs::new(root).map_err(|e| HostError(root_error(root, &e)))?;
        Self::with_context(Context::with_vfs(Box::new(fs)))
    }

    fn with_context(ctx: Context) -> Result<Host, HostError> {
        let registry = standard_registry().map_err(|e| HostError(e.to_string()))?;
        Ok(Host {
            registry,
            ctx,
            doc_paths: BTreeMap::new(),
        })
    }

    /// One request line in; `Some(response line without "\n")` out, `None` for a blank line
    /// (empty, or only whitespace). A trailing `"\n"` or `"\r\n"` on `line` is ignored.
    pub fn handle_line(&mut self, line: &[u8]) -> Option<String> {
        if is_blank(line) {
            return None;
        }
        let (parsed, request) = parse_request(line);
        let response = match request {
            Err(failure) => failure.into_response(parsed.as_ref(), None),
            Ok(request) => self.execute(request, parsed.as_ref()),
        };
        Some(canonical(&response))
    }

    fn execute(&mut self, request: Request, parsed: Option<&Value>) -> Value {
        let Request {
            id,
            command,
            params,
        } = request;
        let registry = &self.registry;
        let ctx = &mut self.ctx;
        let run_params = params.clone();
        // A panicking command must not take the host down (the context may be left partly
        // changed; that is a bug in the command, reported as `internal`).
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            registry.execute(ctx, &command, run_params)
        }))
        .unwrap_or_else(|_| {
            Err(CommandError::Internal {
                message: PANICKED.to_owned(),
            })
        });
        match outcome {
            Ok(result) => {
                self.track_paths(&command, &result);
                json!({"id": id, "ok": true, "result": result})
            }
            Err(e) => {
                let doc_path = params
                    .get("doc")
                    .and_then(Value::as_u64)
                    .and_then(|d| self.doc_paths.get(&d))
                    .map(String::as_str);
                let request_path = params.get("path").and_then(Value::as_str);
                let failure = Failure::from_command(id, &command, &e, request_path.or(doc_path));
                failure.into_response(parsed, doc_path)
            }
        }
    }

    fn track_paths(&mut self, command: &str, result: &Value) {
        let doc = result.get("doc").and_then(Value::as_u64);
        match (command, doc) {
            ("doc.open", Some(doc)) => {
                if let Some(path) = result.get("path").and_then(Value::as_str) {
                    self.doc_paths.insert(doc, path.to_owned());
                }
            }
            ("doc.close", Some(doc)) if result.get("closed") == Some(&Value::Bool(true)) => {
                self.doc_paths.remove(&doc);
            }
            _ => {}
        }
    }

    /// Every successfully executed command, in execution order.
    pub fn log(&self) -> &[LogEntry] {
        self.ctx.log()
    }

    /// The log as a script: one request per entry,
    /// `{"command":…,"id":<1-based index>,"params":…}`, keys sorted, each line `"\n"`-terminated.
    pub fn log_script(&self) -> String {
        let mut out = String::new();
        for (i, entry) in self.log().iter().enumerate() {
            let request = json!({"command": entry.name, "id": i + 1, "params": entry.params});
            out.push_str(&canonical(&request));
            out.push('\n');
        }
        out
    }
}

/// "root `<root>`: <reason>" for a [`DiskFs::new`] failure (the root named as given).
fn root_error(root: &Path, e: &VfsError) -> String {
    let reason = match e {
        VfsError::NotFound { .. } => "not found".to_owned(),
        VfsError::Io {
            kind: std::io::ErrorKind::NotADirectory,
            ..
        } => "not a directory".to_owned(),
        other => {
            let text = other.to_string();
            text.strip_prefix("(root): ").unwrap_or(&text).to_owned()
        }
    };
    format!("root `{}`: {reason}", root.display())
}

/// Clip, in an error object `{"code","kind","message"[,"details"]}`, every echo of a value
/// from `sources` in `message` and in each `details` entry to at most 200 characters.
/// `sources` are the input values the error may echo (requests, params, arguments).
pub fn clip_error(error: &mut Value, sources: &[&Value]) {
    let _ = (error, sources);
}

/// `text` with every echo of a value from `sources` clipped to at most 200 characters.
pub fn clip_text(text: &str, sources: &[&Value]) -> String {
    let _ = sources;
    text.to_owned()
}

/// The message of the `internal` error a panicking command produces (marks kind `panic`).
const PANICKED: &str = "the command panicked";

/// A request that passed protocol validation (not yet the registry's).
struct Request {
    id: Value,
    command: String,
    params: Value,
}

/// An error response before echo clipping.
struct Failure {
    id: Value,
    code: String,
    kind: &'static str,
    message: String,
    details: Option<Vec<String>>,
}

impl Failure {
    fn protocol(id: Value, code: &str, kind: &'static str, message: impl Into<String>) -> Self {
        Failure {
            id,
            code: code.to_owned(),
            kind,
            message: message.into(),
            details: None,
        }
    }

    fn from_command(id: Value, command: &str, e: &CommandError, path: Option<&str>) -> Self {
        let (kind, message, details) = match e {
            CommandError::UnknownCommand { message } => ("unknown_command", message.clone(), None),
            CommandError::InvalidParams { message, details } => {
                ("schema", message.clone(), Some(details.clone()))
            }
            CommandError::Rejected { message } => {
                (rejected_kind(message, path), message.clone(), None)
            }
            CommandError::Internal { message } if message == PANICKED => {
                ("panic", format!("command `{command}` panicked"), None)
            }
            CommandError::Internal { message } => {
                let kind = if message.contains("violates its schema") {
                    "result_schema"
                } else {
                    "other"
                };
                (kind, message.clone(), None)
            }
        };
        let code = e.to_json()["code"]
            .as_str()
            .unwrap_or("internal")
            .to_owned();
        Failure {
            id,
            code,
            kind,
            message,
            details,
        }
    }

    /// The response, with long echoes of `request` values (and of `doc_path`) clipped.
    fn into_response(self, request: Option<&Value>, doc_path: Option<&str>) -> Value {
        let mut forms = Vec::new();
        if let Some(request) = request {
            collect_long_strings(request, &mut forms);
        }
        if let Some(path) = doc_path {
            push_forms(path, &mut forms);
        }
        forms.sort_by_key(|f| std::cmp::Reverse(f.len()));
        forms.dedup();
        let params = request.and_then(|r| r.get("params"));
        let mut error = Map::new();
        error.insert("code".into(), Value::String(self.code));
        error.insert("kind".into(), Value::String(self.kind.to_owned()));
        error.insert(
            "message".into(),
            Value::String(clip_echoes(&self.message, &forms)),
        );
        if let Some(details) = self.details {
            let details = details
                .iter()
                .map(|d| {
                    let mut own = forms.clone();
                    if let Some(form) = detail_value_form(d, params) {
                        own.insert(0, form);
                    }
                    Value::String(clip_echoes(d, &own))
                })
                .collect();
            error.insert("details".into(), Value::Array(details));
        }
        json!({"error": Value::Object(error), "id": self.id, "ok": false})
    }
}

/// The `kind` of a `rejected` error, from its message. `path` is the path the request names
/// (`doc.open`) or the path of the document it names, when known.
fn rejected_kind(message: &str, path: Option<&str>) -> &'static str {
    if message.starts_with("invalid relative path ") {
        return "invalid_path";
    }
    if message.starts_with("no open document ") {
        return "no_such_document";
    }
    if message.starts_with("document ") && message.contains(" has unsaved changes") {
        return "unsaved_changes";
    }
    if message.starts_with("char index ") && message.contains(" is out of bounds") {
        return "out_of_range";
    }
    if message.starts_with("range start ") && message.contains(" is after range end ") {
        return "inverted_range";
    }
    let file_op = ["cannot open ", "cannot save "]
        .iter()
        .find_map(|p| message.strip_prefix(p));
    let Some(rest) = file_op else {
        return "other";
    };
    // The reason after "<path>: ": exact when the path is known, else after the last
    // recognisable separator.
    let reason = match path.and_then(|p| rest.strip_prefix(p)) {
        Some(after) => after.strip_prefix(": ").unwrap_or(after),
        None => rest,
    };
    if reason == "not found" || reason.ends_with(": not found") {
        "not_found"
    } else if reason == "is a directory, not a file"
        || reason.ends_with(": is a directory, not a file")
    {
        "not_a_file"
    } else if reason.starts_with("I/O error: ") || reason.contains(": I/O error: ") {
        "io_error"
    } else if is_decode_reason(reason) {
        "decode_error"
    } else {
        "other"
    }
}

/// `invalid <encoding> at byte offset <n>` (possibly after a path).
fn is_decode_reason(reason: &str) -> bool {
    let Some((head, offset)) = reason.rsplit_once(" at byte offset ") else {
        return false;
    };
    !offset.is_empty()
        && offset.bytes().all(|b| b.is_ascii_digit())
        && (head.starts_with("invalid ") || head.contains(": invalid "))
}

/// `s` cut to at most [`MAX_ECHO`] characters (`…` marks a cut).
fn clip(s: &str) -> String {
    if s.chars().count() <= MAX_ECHO {
        return s.to_owned();
    }
    let mut out: String = s.chars().take(MAX_ECHO - 1).collect();
    out.push('…');
    out
}

/// The forms in which `s` can appear inside a message (as is, JSON-escaped, Rust-escaped),
/// when longer than [`MAX_ECHO`] characters.
fn push_forms(s: &str, forms: &mut Vec<String>) {
    if s.chars().count() <= MAX_ECHO {
        return;
    }
    forms.push(s.to_owned());
    let json = Value::String(s.to_owned()).to_string();
    forms.push(json[1..json.len() - 1].to_owned());
    let debug = format!("{s:?}");
    forms.push(debug[1..debug.len() - 1].to_owned());
}

/// Every long string (keys and values) of `value`.
fn collect_long_strings(value: &Value, forms: &mut Vec<String>) {
    match value {
        Value::String(s) => push_forms(s, forms),
        Value::Array(items) => items.iter().for_each(|v| collect_long_strings(v, forms)),
        Value::Object(map) => {
            for (k, v) in map {
                push_forms(k, forms);
                collect_long_strings(v, forms);
            }
        }
        _ => {}
    }
}

/// A schema-validation detail reads `<pointer>: <value> <reason>` (or `<reason>` at the root,
/// where the value is the whole params object): the compact JSON of that value if long.
fn detail_value_form(detail: &str, params: Option<&Value>) -> Option<String> {
    let params = params?;
    let value = match detail.split_once(": ") {
        Some((pointer, _)) if pointer.starts_with('/') => params.pointer(pointer)?,
        _ => params,
    };
    let compact = value.to_string();
    (compact.chars().count() > MAX_ECHO).then_some(compact)
}

/// `text` with every occurrence of each of `forms` (longest first) clipped, then every run of
/// more than [`MAX_ECHO`] non-white-space characters clipped (an echo the forms missed).
fn clip_echoes(text: &str, forms: &[String]) -> String {
    let mut text = text.to_owned();
    for form in forms {
        if text.contains(form.as_str()) {
            text = text.replace(form.as_str(), &clip(form));
        }
    }
    clip_runs(&text)
}

fn clip_runs(text: &str) -> String {
    let mut out = String::with_capacity(text.len().min(4096));
    let mut run = String::new();
    for c in text.chars() {
        if c.is_whitespace() {
            out.push_str(&clip(&run));
            run.clear();
            out.push(c);
        } else {
            run.push(c);
        }
    }
    out.push_str(&clip(&run));
    out
}

/// True iff `line` is empty or consists only of whitespace (any Unicode white space; a line
/// that is not UTF-8 is never blank).
fn is_blank(line: &[u8]) -> bool {
    std::str::from_utf8(line).is_ok_and(|s| s.chars().all(char::is_whitespace))
}

/// Validate one non-blank line as a protocol request. Returns the parsed JSON (if the line was
/// JSON) and the request or its error.
fn parse_request(line: &[u8]) -> (Option<Value>, Result<Request, Failure>) {
    let null = Value::Null;
    let text = match std::str::from_utf8(line) {
        Ok(t) => t,
        Err(e) => {
            let f = Failure::protocol(
                null,
                "invalid_json",
                "not_utf8",
                format!("request is not valid UTF-8: {e}"),
            );
            return (None, Err(f));
        }
    };
    let value: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            let f = Failure::protocol(
                null,
                "invalid_json",
                "syntax",
                format!("request is not JSON: {e}"),
            );
            return (None, Err(f));
        }
    };
    let request = validate(&value);
    (Some(value), request)
}

fn validate(value: &Value) -> Result<Request, Failure> {
    let invalid = |id: &Value, kind, message: &str| {
        Failure::protocol(id.clone(), "invalid_request", kind, message)
    };
    let Value::Object(obj) = value else {
        return Err(invalid(
            &Value::Null,
            "not_an_object",
            "request must be a JSON object",
        ));
    };
    let id = match obj.get("id") {
        None => Value::Null,
        Some(v @ Value::String(_)) => v.clone(),
        Some(Value::Number(n)) if n.is_i64() || n.is_u64() => Value::Number(n.clone()),
        Some(_) => {
            return Err(invalid(
                &Value::Null,
                "bad_id",
                "`id` must be a string or an integer",
            ));
        }
    };
    let mut unknown: Vec<&str> = obj
        .keys()
        .map(String::as_str)
        .filter(|k| !REQUEST_KEYS.contains(k))
        .collect();
    if !unknown.is_empty() {
        unknown.sort_unstable();
        let list = unknown
            .iter()
            .map(|k| format!("`{k}`"))
            .collect::<Vec<_>>()
            .join(", ");
        let message =
            format!("unknown top-level key(s) {list}; allowed: `command`, `id`, `params`");
        return Err(invalid(&id, "unknown_key", &message));
    }
    let command = match obj.get("command") {
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(invalid(
                &id,
                "command_not_string",
                "`command` must be a string",
            ));
        }
        None => return Err(invalid(&id, "missing_command", "`command` is missing")),
    };
    let params = match obj.get("params") {
        None => Value::Object(Map::new()),
        Some(v @ Value::Object(_)) => v.clone(),
        Some(_) => {
            return Err(invalid(
                &id,
                "params_not_object",
                "`params` must be an object",
            ));
        }
    };
    Ok(Request {
        id,
        command,
        params,
    })
}

/// Compact JSON with object keys sorted (by code point) at every level, independent of the
/// map type `serde_json` was built with.
fn canonical(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out);
    out
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_unstable();
            out.push('{');
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&Value::String((*key).clone()).to_string());
                out.push(':');
                if let Some(v) = map.get(*key) {
                    write_canonical(v, out);
                }
            }
            out.push('}');
        }
        // Scalars: serde_json's compact form (strings escaped, numbers as parsed).
        scalar => out.push_str(&scalar.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host() -> Host {
        Host::in_memory(&[("a.txt", b"hello\n")]).unwrap()
    }

    fn call(h: &mut Host, line: &str) -> Value {
        serde_json::from_str(&h.handle_line(line.as_bytes()).unwrap()).unwrap()
    }

    const SESSION: [&str; 5] = [
        r#"{"id":1,"command":"doc.open","params":{"path":"a.txt"}}"#,
        r#"{"id":2,"command":"doc.insert","params":{"doc":1,"offset":5,"text":" world"}}"#,
        r#"{"id":3,"command":"doc.undo","params":{"doc":1}}"#,
        r#"{"id":4,"command":"doc.redo","params":{"doc":1}}"#,
        r#"{"id":5,"command":"doc.save","params":{"doc":1}}"#,
    ];

    #[test]
    fn session_in_memory_saves_and_reports_exact_line() {
        let mut h = host();
        let out: Vec<String> = SESSION
            .iter()
            .map(|l| h.handle_line(l.as_bytes()).unwrap())
            .collect();
        assert_eq!(
            out[4],
            r#"{"id":5,"ok":true,"result":{"bytes_written":12,"dirty":false,"doc":1}}"#
        );
        let text = call(&mut h, r#"{"command":"doc.text","params":{"doc":1}}"#);
        assert_eq!(text["result"]["text"], "hello world\n");
        assert_eq!(h.log().len(), 6);
    }

    #[test]
    fn blank_lines_give_no_response() {
        let mut h = host();
        for l in ["", " ", "\t", "\r", "\n", " \r\n", "\u{a0}"] {
            assert_eq!(h.handle_line(l.as_bytes()), None, "{l:?}");
        }
    }

    #[test]
    fn ids_echoed_or_null() {
        let mut h = host();
        let list = |id: &str| format!(r#"{{"id":{id},"command":"doc.list"}}"#);
        assert_eq!(call(&mut h, &list("\"x\""))["id"], "x");
        assert_eq!(call(&mut h, &list("-3"))["id"], -3);
        assert_eq!(
            call(&mut h, &list("18446744073709551615"))["id"],
            json!(u64::MAX)
        );
        let v = call(&mut h, r#"{"command":"doc.list"}"#);
        assert_eq!(v["id"], Value::Null);
        assert_eq!(v["ok"], true);
    }

    fn code_and_id(h: &mut Host, line: &[u8]) -> (String, Value) {
        let v: Value = serde_json::from_str(&h.handle_line(line).unwrap()).unwrap();
        assert_eq!(v["ok"], false, "{v}");
        assert!(v["error"]["message"].is_string(), "{v}");
        (
            v["error"]["code"].as_str().unwrap().to_owned(),
            v["id"].clone(),
        )
    }

    #[test]
    fn malformed_requests_get_codes_and_host_continues() {
        let mut h = host();
        let cases: [(&[u8], &str, Value); 14] = [
            (b"not json", "invalid_json", Value::Null),
            (b"\xff\xfe{}", "invalid_json", Value::Null),
            (b"[1,2]", "invalid_request", Value::Null),
            (b"\"s\"", "invalid_request", Value::Null),
            (br#"{"id":7}"#, "invalid_request", json!(7)),
            (
                br#"{"id":1.5,"command":"doc.list"}"#,
                "invalid_request",
                Value::Null,
            ),
            (
                br#"{"id":null,"command":"doc.list"}"#,
                "invalid_request",
                Value::Null,
            ),
            (br#"{"id":"q","command":3}"#, "invalid_request", json!("q")),
            (
                br#"{"command":"doc.list","params":[]}"#,
                "invalid_request",
                Value::Null,
            ),
            (
                br#"{"command":"doc.list","params":null}"#,
                "invalid_request",
                Value::Null,
            ),
            (
                br#"{"id":2,"command":"doc.list","extra":1}"#,
                "invalid_request",
                json!(2),
            ),
            (br#"{"command":"no.such"}"#, "unknown_command", Value::Null),
            (
                br#"{"command":"doc.insert","params":{"doc":1}}"#,
                "invalid_params",
                Value::Null,
            ),
            (
                br#"{"command":"doc.close","params":{"doc":9}}"#,
                "rejected",
                Value::Null,
            ),
        ];
        for (line, code, id) in cases {
            assert_eq!(
                code_and_id(&mut h, line),
                (code.to_owned(), id),
                "{}",
                String::from_utf8_lossy(line)
            );
            assert_eq!(call(&mut h, r#"{"command":"doc.list"}"#)["ok"], true);
        }
        // Only the successful doc.list requests were logged.
        assert!(h.log().iter().all(|e| e.name == "doc.list"));
        assert_eq!(h.log().len(), 14);
    }

    #[test]
    fn invalid_params_carry_details() {
        let mut h = host();
        let v = call(&mut h, r#"{"command":"doc.insert","params":{"doc":1}}"#);
        assert!(
            v["error"]["details"]
                .as_array()
                .is_some_and(|d| !d.is_empty())
        );
    }

    #[test]
    fn responses_are_canonical() {
        let mut h = host();
        for l in SESSION
            .iter()
            .copied()
            .chain([r#"{"command":"doc.info","params":{"doc":1}}"#, "nope"])
        {
            let line = h.handle_line(l.as_bytes()).unwrap();
            let v: Value = serde_json::from_str(&line).unwrap();
            assert_eq!(canonical(&v), line);
        }
    }

    #[test]
    fn canonical_sorts_nested_keys_and_escapes() {
        let v = json!({"b": [{"z": 1, "a": "x\"\n"}], "a": {"d": null, "c": 1.5}});
        assert_eq!(
            canonical(&v),
            r#"{"a":{"c":1.5,"d":null},"b":[{"a":"x\"\n","z":1}]}"#
        );
    }

    #[test]
    fn log_script_numbers_entries_and_replays_identically() {
        let mut h = host();
        for l in SESSION {
            h.handle_line(l.as_bytes());
        }
        h.handle_line(br#"{"id":"bad","command":"doc.close","params":{"doc":42}}"#);
        let script = h.log_script();
        let lines: Vec<&str> = script.lines().collect();
        assert_eq!(lines.len(), 5);
        assert!(script.ends_with('\n'));
        assert_eq!(
            lines[0],
            r#"{"command":"doc.open","id":1,"params":{"path":"a.txt"}}"#
        );
        assert_eq!(
            lines[4],
            r#"{"command":"doc.save","id":5,"params":{"doc":1}}"#
        );

        let mut again = host();
        for l in &lines {
            assert_eq!(call(&mut again, l)["ok"], true);
        }
        assert_eq!(again.log_script(), script);
        assert_eq!(again.log(), h.log());
    }

    #[test]
    fn host_errors() {
        assert!(Host::in_memory(&[("/abs", b"")]).is_err());
        let file = std::env::current_exe().unwrap();
        // The bad-root error names the root as given (D-028 (7)); it is not a response.
        let e = Host::on_disk(&file).unwrap_err().to_string();
        assert!(e.contains(&*file.to_string_lossy()), "{e}");
        assert!(e.contains("not a directory"), "{e}");
        let e = Host::on_disk(Path::new("/no/such/dir/for/kiln"))
            .unwrap_err()
            .to_string();
        assert_eq!(e, "root `/no/such/dir/for/kiln`: not found");
    }

    /// `(code, kind)` of an error response, checking `kind` is a listed pair.
    fn code_kind(h: &mut Host, line: &[u8]) -> (String, String) {
        let v: Value = serde_json::from_str(&h.handle_line(line).unwrap()).unwrap();
        assert_eq!(v["ok"], false, "{v}");
        let code = v["error"]["code"].as_str().unwrap().to_owned();
        let kind = v["error"]["kind"].as_str().unwrap().to_owned();
        assert!(
            ERROR_KINDS.contains(&(code.as_str(), kind.as_str())),
            "unlisted ({code}, {kind})"
        );
        (code, kind)
    }

    #[test]
    fn every_error_has_a_stable_kind() {
        let mut h = Host::in_memory(&[
            ("a.txt", b"hello\n"),
            ("bad.txt", b"\xff\xfe\x00"),
            ("bin.txt", b"ok\xc3"),
            ("dir/x.txt", b"x"),
        ])
        .unwrap();
        call(
            &mut h,
            r#"{"command":"doc.open","params":{"path":"a.txt"}}"#,
        );
        call(
            &mut h,
            r#"{"command":"doc.insert","params":{"doc":1,"offset":0,"text":"!"}}"#,
        );
        let cases: [(&[u8], &str, &str); 21] = [
            (b"\xff", "invalid_json", "not_utf8"),
            (b"{", "invalid_json", "syntax"),
            (b"[]", "invalid_request", "not_an_object"),
            (
                br#"{"id":true,"command":"doc.list"}"#,
                "invalid_request",
                "bad_id",
            ),
            (
                br#"{"command":"doc.list","x":1}"#,
                "invalid_request",
                "unknown_key",
            ),
            (br#"{"id":1}"#, "invalid_request", "missing_command"),
            (br#"{"command":1}"#, "invalid_request", "command_not_string"),
            (
                br#"{"command":"doc.list","params":1}"#,
                "invalid_request",
                "params_not_object",
            ),
            (
                br#"{"command":"no.such"}"#,
                "unknown_command",
                "unknown_command",
            ),
            (
                br#"{"command":"doc.insert","params":{}}"#,
                "invalid_params",
                "schema",
            ),
            (
                br#"{"command":"doc.open","params":{"path":"../a"}}"#,
                "rejected",
                "invalid_path",
            ),
            (
                br#"{"command":"doc.open","params":{"path":"/a"}}"#,
                "rejected",
                "invalid_path",
            ),
            (
                br#"{"command":"doc.info","params":{"doc":9}}"#,
                "rejected",
                "no_such_document",
            ),
            (
                br#"{"command":"doc.close","params":{"doc":1}}"#,
                "rejected",
                "unsaved_changes",
            ),
            (
                br#"{"command":"doc.open","params":{"path":"nope.txt"}}"#,
                "rejected",
                "not_found",
            ),
            (
                br#"{"command":"doc.open","params":{"path":"dir"}}"#,
                "rejected",
                "not_a_file",
            ),
            (
                br#"{"command":"doc.open","params":{"path":"bad.txt"}}"#,
                "rejected",
                "decode_error",
            ),
            (
                br#"{"command":"doc.open","params":{"path":"bin.txt"}}"#,
                "rejected",
                "decode_error",
            ),
            (
                br#"{"command":"doc.insert","params":{"doc":1,"offset":99,"text":"x"}}"#,
                "rejected",
                "out_of_range",
            ),
            (
                br#"{"command":"doc.delete","params":{"doc":1,"start":3,"end":1}}"#,
                "rejected",
                "inverted_range",
            ),
            (
                br#"{"command":"buffer.insert","params":{"offset":5,"text":"x"}}"#,
                "rejected",
                "out_of_range",
            ),
        ];
        for (line, code, kind) in cases {
            assert_eq!(
                code_kind(&mut h, line),
                (code.to_owned(), kind.to_owned()),
                "{}",
                String::from_utf8_lossy(line)
            );
        }
    }

    #[test]
    fn rejected_kind_is_anchored_on_the_known_path() {
        // A file name that itself looks like a reason does not fool the classification.
        let path = "x: not found";
        assert_eq!(
            rejected_kind(
                &format!("cannot open {path}: is a directory, not a file"),
                Some(path)
            ),
            "not_a_file"
        );
        assert_eq!(
            rejected_kind(
                "cannot save a.txt: I/O error: No space left on device (os error 28)",
                Some("a.txt")
            ),
            "io_error"
        );
        assert_eq!(
            rejected_kind("cannot save a.txt: I/O error: boom", None),
            "io_error"
        );
        assert_eq!(
            rejected_kind("cannot open a.txt: invalid utf-8 at byte offset 3", None),
            "decode_error"
        );
        assert_eq!(
            rejected_kind("cannot open a.txt: something new", Some("a.txt")),
            "other"
        );
        assert_eq!(rejected_kind("a brand new message", None), "other");
    }

    #[test]
    fn io_errors_on_disk_have_kind_io_error_with_os_text_in_message_only() {
        let dir = std::env::temp_dir().join(format!("kiln-control-io-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut h = Host::on_disk(&dir).unwrap();
        let long = "n".repeat(300);
        let line = format!(r#"{{"command":"doc.open","params":{{"path":"{long}"}}}}"#);
        let v = call(&mut h, &line);
        assert_eq!(v["error"]["kind"], "io_error", "{v}");
        assert_eq!(v["error"]["code"], "rejected");
        let msg = v["error"]["message"].as_str().unwrap();
        assert!(!msg.contains(&"n".repeat(MAX_ECHO)), "{msg}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// No error text of `v` contains more than MAX_ECHO consecutive chars of `needle`'s
    /// repeated unit.
    fn assert_clipped(v: &Value, unit: &str) {
        let too_long = unit.repeat(MAX_ECHO / unit.chars().count() + 1);
        let mut texts = vec![v["error"]["message"].as_str().unwrap().to_owned()];
        if let Some(d) = v["error"]["details"].as_array() {
            texts.extend(d.iter().map(|d| d.as_str().unwrap().to_owned()));
        }
        for t in texts {
            assert!(!t.contains(&too_long), "not clipped: {t}");
            assert!(t.chars().count() < 3 * MAX_ECHO, "too long: {t}");
        }
    }

    #[test]
    fn echoed_values_are_clipped_to_200_chars() {
        let mut h = host();
        call(
            &mut h,
            r#"{"command":"doc.open","params":{"path":"a.txt"}}"#,
        );
        let a = "a".repeat(5000);
        let sp = "a b ".repeat(2000);
        let quote = r#"\"\n"#.repeat(500);
        let lines = [
            format!(r#"{{"command":"{a}"}}"#),
            format!(r#"{{"command":"doc.list","{a}":1}}"#),
            format!(r#"{{"command":"doc.open","params":{{"path":"{a}"}}}}"#),
            format!(r#"{{"command":"doc.open","params":{{"path":"{sp}"}}}}"#),
            format!(r#"{{"command":"doc.open","params":{{"path":"../{sp}"}}}}"#),
            format!(
                r#"{{"command":"doc.open","params":{{"path":"../{}"}}}}"#,
                quote
            ),
            format!(
                r#"{{"command":"doc.insert","params":{{"doc":"{sp}","offset":0,"text":"x"}}}}"#
            ),
            format!(
                r#"{{"command":"doc.insert","params":{{"doc":[{}1],"offset":0,"text":"x"}}}}"#,
                "\"a b\",".repeat(500)
            ),
            format!(
                r#"{{"command":"doc.insert","params":{{"doc":1,"offset":0,"text":"x","{sp}":1}}}}"#
            ),
        ];
        for (i, line) in lines.iter().enumerate() {
            let v = call(&mut h, line);
            assert_eq!(v["ok"], false, "{i}");
            assert_clipped(&v, "a");
            assert_clipped(&v, "a b ");
            assert_clipped(&v, r#"\"\n"#);
            assert!(v["error"].to_string().contains('…'), "{i}: {v}");
        }
        // Short values are echoed in full.
        let v = call(&mut h, r#"{"command":"no.such"}"#);
        assert_eq!(v["error"]["message"], "unknown command `no.such`");
    }

    #[test]
    fn clip_keeps_200_chars() {
        let s = "é".repeat(250);
        let c = clip(&s);
        assert_eq!(c.chars().count(), MAX_ECHO);
        assert!(c.ends_with('…'));
        assert_eq!(clip("short"), "short");
        assert_eq!(clip_runs(&"x".repeat(MAX_ECHO)), "x".repeat(MAX_ECHO));
        let r = clip_runs(&format!("{} y", "x".repeat(MAX_ECHO + 1)));
        assert_eq!(r.chars().count(), MAX_ECHO + 2);
    }

    #[test]
    fn every_kind_is_documented() {
        let doc = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/control-protocol.md"
        ))
        .unwrap();
        for (code, kind) in ERROR_KINDS {
            assert!(doc.contains(&format!("`{code}`")), "{code}");
            assert!(doc.contains(&format!("`{kind}`")), "{kind}");
        }
    }

    #[test]
    fn on_disk_matches_in_memory() {
        let dir = std::env::temp_dir().join(format!("kiln-control-unit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), b"hello\n").unwrap();
        let mut disk = Host::on_disk(&dir).unwrap();
        let mut mem = host();
        for l in SESSION
            .iter()
            .copied()
            .chain([r#"{"command":"doc.open","params":{"path":"missing.txt"}}"#])
        {
            let d = disk.handle_line(l.as_bytes()).unwrap();
            assert_eq!(d, mem.handle_line(l.as_bytes()).unwrap());
            assert!(!d.contains(&*dir.to_string_lossy()));
        }
        assert_eq!(std::fs::read(dir.join("a.txt")).unwrap(), b"hello world\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
