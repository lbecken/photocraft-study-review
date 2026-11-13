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
//! `code` and `kind` are stable machine-readable values (`kind` refines `code`); `message` and
//! `details` are for humans and may contain operating-system error text. Error lines are
//! size-capped ([`cap_error`]): every string in `message` and `details` is at most 200
//! characters and the whole line at most 4096 bytes.
//!
//! The `kind` of a `rejected` or `internal` error is the structured reason `kiln-commands` sets
//! where the error is raised (`reason.as_str()`, ADR-0004); no message text is consulted for
//! any code or kind, and the reason itself is not part of the error object. The full list of
//! kinds is `error_kinds` in `docs/commands.json` (generated from code; `kiln-cli commands
//! --schema`) plus the request-level rows of the table in `docs/control-protocol.md`.

use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use kiln_commands::{CommandError, Context, InternalReason, LogEntry, Registry, standard_registry};
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

/// Every `(code, kind)` pair of the table in `docs/control-protocol.md` (the request-level rows
/// and the kinds of iteration 0; later kinds are in `error_kinds` of `docs/commands.json`).
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
        Ok(Host { registry, ctx })
    }

    /// One request line in; `Some(response line without "\n")` out, `None` for a blank line
    /// (empty, or only whitespace). A trailing `"\n"` or `"\r\n"` on `line` is ignored.
    pub fn handle_line(&mut self, line: &[u8]) -> Option<String> {
        if is_blank(line) {
            return None;
        }
        let request = parse_request(line);
        let mut response = match request {
            Err(failure) => failure.into_response(),
            Ok(request) => self.execute(request),
        };
        // The one place every response leaves the host: error lines get the size caps.
        if response.get("error").is_some() {
            cap_error(&mut response);
        }
        Some(canonical(&response))
    }

    fn execute(&mut self, request: Request) -> Value {
        let Request {
            id,
            command,
            params,
        } = request;
        let registry = &self.registry;
        let ctx = &mut self.ctx;
        // A panicking command must not take the host down (the context may be left partly
        // changed; that is a bug in the command, reported as `internal`).
        let outcome = catch_unwind(AssertUnwindSafe(|| registry.execute(ctx, &command, params)))
            .unwrap_or_else(|_| {
                Err(CommandError::Internal {
                    reason: InternalReason::Panic,
                    message: PANICKED.to_owned(),
                })
            });
        match outcome {
            Ok(result) => json!({"id": id, "ok": true, "result": result}),
            Err(e) => Failure::from_command(id, &command, &e).into_response(),
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

/// The longest string allowed in an error's `message` or anywhere inside its `details`, in
/// characters (longer ones are cut to 199 characters followed by `…`).
pub const MAX_ERROR_STRING: usize = 200;

/// The longest error line allowed, in bytes; a longer one has its `details` replaced by
/// `{"truncated":true}`.
pub const MAX_ERROR_LINE: usize = 4096;

/// Apply the error caps (D-031) to `line`: a protocol response holding an `error` object, or
/// a bare error object `{"code","kind","message"[,"details"]}`. Every string in `message` and
/// anywhere inside `details` is cut to [`MAX_ERROR_STRING`] characters; then, if the compact
/// line is longer than [`MAX_ERROR_LINE`] bytes, `details` becomes `{"truncated":true}`.
pub fn cap_error(line: &mut Value) {
    let wrapped = line.get("error").is_some_and(Value::is_object);
    let error = if wrapped {
        &mut line["error"]
    } else {
        &mut *line
    };
    if let Some(message) = error.get_mut("message") {
        cap_strings(message);
    }
    if let Some(details) = error.get_mut("details") {
        cap_strings(details);
    }
    if canonical(line).len() > MAX_ERROR_LINE {
        let error = if wrapped {
            &mut line["error"]
        } else {
            &mut *line
        };
        if let Some(details) = error.get_mut("details") {
            *details = json!({"truncated": true});
        }
    }
}

/// `text` with every line cut to [`MAX_ERROR_STRING`] characters (for error text on stderr).
pub fn cap_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len().min(MAX_ERROR_LINE));
    for line in text.split_inclusive('\n') {
        match line.strip_suffix('\n') {
            Some(body) => {
                out.push_str(&cap_string(body));
                out.push('\n');
            }
            None => out.push_str(&cap_string(line)),
        }
    }
    out
}

/// The capped error object `{"code","kind","message"[,"details"]}` for `e`, raised by
/// `command` with `params` (as the protocol reports it, without `id`). The kind comes from
/// `e` alone: `params` and the message are not consulted for it.
pub fn command_error_json(command: &str, _params: &Value, e: &CommandError) -> Value {
    let mut response = Failure::from_command(Value::Null, command, e).into_response();
    let mut error = response["error"].take();
    cap_error(&mut error);
    error
}

/// The message the host gives the `internal`/`panic` error of a panicking command (reported
/// as `` command `<name>` panicked ``).
const PANICKED: &str = "the command panicked";

/// A request that passed protocol validation (not yet the registry's).
struct Request {
    id: Value,
    command: String,
    params: Value,
}

/// An error response before the size caps.
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

    /// The failure for `e`: the kind is the reason's name; no message is read.
    fn from_command(id: Value, command: &str, e: &CommandError) -> Self {
        let (kind, message, details) = match e {
            CommandError::UnknownCommand { message } => ("unknown_command", message.clone(), None),
            CommandError::InvalidParams { message, details } => {
                ("schema", message.clone(), Some(details.clone()))
            }
            CommandError::Rejected { reason, message } => (reason.as_str(), message.clone(), None),
            CommandError::Internal {
                reason: InternalReason::Panic,
                ..
            } => (
                InternalReason::Panic.as_str(),
                format!("command `{command}` panicked"),
                None,
            ),
            CommandError::Internal { reason, message } => (reason.as_str(), message.clone(), None),
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

    /// The response (capped later, in [`Host::handle_line`]).
    fn into_response(self) -> Value {
        let mut error = Map::new();
        error.insert("code".into(), Value::String(self.code));
        error.insert("kind".into(), Value::String(self.kind.to_owned()));
        error.insert("message".into(), Value::String(self.message));
        if let Some(details) = self.details {
            let details = details.into_iter().map(Value::String).collect();
            error.insert("details".into(), Value::Array(details));
        }
        json!({"error": Value::Object(error), "id": self.id, "ok": false})
    }
}

/// `s` cut to at most [`MAX_ERROR_STRING`] characters: the first 199 and `…`. Works on
/// characters, so a multi-byte character is never split.
fn cap_string(s: &str) -> String {
    if s.chars().count() <= MAX_ERROR_STRING {
        return s.to_owned();
    }
    let mut out: String = s.chars().take(MAX_ERROR_STRING - 1).collect();
    out.push('…');
    out
}

/// Cap every string inside `value` (array items and object values, at any depth).
fn cap_strings(value: &mut Value) {
    match value {
        Value::String(s) => {
            if s.chars().count() > MAX_ERROR_STRING {
                *s = cap_string(s);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(cap_strings),
        Value::Object(map) => map.values_mut().for_each(cap_strings),
        _ => {}
    }
}

/// True iff `line` is empty or consists only of whitespace (any Unicode white space; a line
/// that is not UTF-8 is never blank).
fn is_blank(line: &[u8]) -> bool {
    std::str::from_utf8(line).is_ok_and(|s| s.chars().all(char::is_whitespace))
}

/// Validate one non-blank line as a protocol request.
fn parse_request(line: &[u8]) -> Result<Request, Failure> {
    let null = Value::Null;
    let text = std::str::from_utf8(line).map_err(|e| {
        Failure::protocol(
            null.clone(),
            "invalid_json",
            "not_utf8",
            format!("request is not valid UTF-8: {e}"),
        )
    })?;
    let value: Value = serde_json::from_str(text).map_err(|e| {
        Failure::protocol(
            null.clone(),
            "invalid_json",
            "syntax",
            format!("request is not JSON: {e}"),
        )
    })?;
    validate(value)
}

fn validate(value: Value) -> Result<Request, Failure> {
    let invalid = |id: &Value, kind, message: &str| {
        Failure::protocol(id.clone(), "invalid_request", kind, message)
    };
    let Value::Object(mut obj) = value else {
        return Err(invalid(
            &Value::Null,
            "not_an_object",
            "request must be a JSON object",
        ));
    };
    let id = match obj.remove("id") {
        None => Value::Null,
        Some(v @ Value::String(_)) => v,
        Some(Value::Number(n)) if n.is_i64() || n.is_u64() => Value::Number(n),
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
    let command = match obj.remove("command") {
        Some(Value::String(s)) => s,
        Some(_) => {
            return Err(invalid(
                &id,
                "command_not_string",
                "`command` must be a string",
            ));
        }
        None => return Err(invalid(&id, "missing_command", "`command` is missing")),
    };
    let params = match obj.remove("params") {
        None => Value::Object(Map::new()),
        Some(v @ Value::Object(_)) => v,
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
    use kiln_commands::RejectReason;

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

    /// Replaces the `rejected_kind` message-classification tests (golden rule 4, disclosed in
    /// the T-032 log): the kind follows the reason, never the message, including messages
    /// the old classification read differently.
    #[test]
    fn kind_follows_the_reason_not_the_message() {
        let messages = [
            "zz unrelated",
            "no open document 1",
            "cannot open x: not found",
            "cannot open x: not found: is a directory, not a file",
            "invalid relative path \"..\": `..` segment",
            "",
        ];
        for r in RejectReason::ALL {
            for m in messages {
                let v = command_error_json(
                    "doc.open",
                    &json!({"path": "x"}),
                    &CommandError::rejected(*r, m),
                );
                assert_eq!(v["code"], "rejected", "{r:?} {m:?}");
                assert_eq!(v["kind"], r.as_str(), "{r:?} {m:?}");
                assert_eq!(v["message"], m, "{r:?} {m:?}");
                assert!(
                    v.get("reason").is_none(),
                    "the protocol object carries no reason"
                );
            }
        }
        for r in InternalReason::ALL {
            for m in messages {
                let e = CommandError::Internal {
                    reason: *r,
                    message: m.to_owned(),
                };
                let v = command_error_json("doc.text", &json!({}), &e);
                assert_eq!(v["code"], "internal", "{r:?} {m:?}");
                assert_eq!(v["kind"], r.as_str(), "{r:?} {m:?}");
                let want = if *r == InternalReason::Panic {
                    "command `doc.text` panicked".to_owned()
                } else {
                    m.to_owned()
                };
                assert_eq!(v["message"], want, "{r:?} {m:?}");
            }
        }
        // The worked example of T-032: the message names no document, the reason governs.
        let e = CommandError::rejected(RejectReason::NotFound, "no open document 1");
        assert_eq!(
            command_error_json("doc.open", &json!({}), &e)["kind"],
            "not_found"
        );
    }

    /// The table's `rejected`/`internal` rows are all reasons, and their kinds are in
    /// `error_kinds` of the registry's description.
    #[test]
    fn table_kinds_are_reasons_and_in_the_description() {
        let d = standard_registry().unwrap().describe();
        for (code, kind) in ERROR_KINDS {
            if *code != "rejected" && *code != "internal" {
                continue;
            }
            let listed = d["error_kinds"][*code].as_array().unwrap();
            assert!(listed.iter().any(|k| k["kind"] == *kind), "{code}/{kind}");
        }
    }

    #[test]
    fn io_errors_on_disk_have_kind_io_error_with_os_text_in_message_only() {
        let dir = std::env::temp_dir().join(format!("kiln-control-io-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut h = Host::on_disk(&dir).unwrap();
        let line = json!({"command": "doc.open", "params": {"path": "n".repeat(300)}}).to_string();
        let v = call(&mut h, &line);
        assert_eq!(v["error"]["kind"], "io_error", "{v}");
        assert_eq!(v["error"]["code"], "rejected");
        assert!(v["error"]["message"].as_str().unwrap().chars().count() <= MAX_ERROR_STRING);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Every string inside `v`, at any depth.
    fn strings(v: &Value, out: &mut Vec<String>) {
        match v {
            Value::String(s) => out.push(s.clone()),
            Value::Array(a) => a.iter().for_each(|x| strings(x, out)),
            Value::Object(m) => m.values().for_each(|x| strings(x, out)),
            _ => {}
        }
    }

    /// The D-031 caps hold for the response line `line`.
    fn assert_capped(line: &str) {
        assert!(
            line.len() <= MAX_ERROR_LINE,
            "{} bytes: {line:.200}",
            line.len()
        );
        let v: Value = serde_json::from_str(line).unwrap();
        let mut all = Vec::new();
        strings(&v["error"]["message"], &mut all);
        strings(&v["error"]["details"], &mut all);
        for s in all {
            assert!(s.chars().count() <= MAX_ERROR_STRING, "{s:.300}");
        }
    }

    #[test]
    fn cap_string_cuts_on_characters() {
        assert_eq!(cap_string("short"), "short");
        let exact = "x".repeat(MAX_ERROR_STRING);
        assert_eq!(cap_string(&exact), exact);
        // Multi-byte characters (2-, 3- and 4-byte) are never split.
        for c in ['é', '€', '😀'] {
            let s = c.to_string().repeat(MAX_ERROR_STRING + 1);
            let capped = cap_string(&s);
            assert_eq!(capped.chars().count(), MAX_ERROR_STRING);
            assert!(capped.ends_with('…'));
            assert!(capped.chars().take(MAX_ERROR_STRING - 1).all(|x| x == c));
        }
        // A cut right after a multi-byte character at position 199.
        let s = format!("{}é{}", "a".repeat(198), "b".repeat(10));
        assert_eq!(cap_string(&s), format!("{}é…", "a".repeat(198)));
    }

    #[test]
    fn cap_error_caps_strings_in_message_and_nested_details() {
        let long = "w x ".repeat(100);
        let mut e = json!({
            "code": "invalid_params",
            "kind": "schema",
            "message": long,
            "details": [long, {"deep": [long, 1, null]}, "short"],
        });
        cap_error(&mut e);
        let cut = cap_string(&long);
        assert_eq!(cut.chars().count(), MAX_ERROR_STRING);
        assert_eq!(e["message"], cut.as_str());
        assert_eq!(
            e["details"],
            json!([cut, {"deep": [cut, 1, null]}, "short"])
        );
        assert_eq!(
            (e["code"].clone(), e["kind"].clone()),
            (json!("invalid_params"), json!("schema"))
        );
        // Wrapped in a response, the same.
        let mut r = json!({"error": {"code": "rejected", "kind": "other", "message": long}, "id": 1, "ok": false});
        cap_error(&mut r);
        assert_eq!(r["error"]["message"], cut.as_str());
        assert_eq!(r["id"], 1);
    }

    #[test]
    fn cap_error_replaces_details_when_the_line_is_too_long() {
        // 30 details of 200 chars: each string is within the cap, the line is not.
        let details: Vec<String> = (0..30).map(|i| format!("{i:03}").repeat(66)).collect();
        let mut r = json!({"error": {"code": "invalid_params", "kind": "schema",
            "message": "invalid params", "details": details}, "id": "x", "ok": false});
        cap_error(&mut r);
        assert_eq!(r["error"]["details"], json!({"truncated": true}));
        assert_eq!(r["error"]["message"], "invalid params");
        assert!(canonical(&r).len() <= MAX_ERROR_LINE);
        // Just under the limit, details are kept.
        let few: Vec<String> = details[..10].to_vec();
        let mut r = json!({"error": {"code": "invalid_params", "kind": "schema",
            "message": "m", "details": few.clone()}, "id": 1, "ok": false});
        cap_error(&mut r);
        assert_eq!(r["error"]["details"], json!(few));
        // Multi-byte strings: the byte limit is on the encoded line.
        let wide: Vec<String> = (0..8).map(|_| "😀".repeat(150)).collect();
        let mut r = json!({"error": {"code": "invalid_params", "kind": "schema",
            "message": "m", "details": wide}, "id": 1, "ok": false});
        cap_error(&mut r);
        assert_eq!(r["error"]["details"], json!({"truncated": true}));
    }

    #[test]
    fn cap_text_caps_each_line() {
        let long = "é".repeat(500);
        let text = format!("kiln-cli: {long}\nusage:\n  short\n{long}");
        let capped = cap_text(&text);
        let lines: Vec<&str> = capped.split('\n').collect();
        assert_eq!(lines.len(), 4);
        assert!(lines.iter().all(|l| l.chars().count() <= MAX_ERROR_STRING));
        assert_eq!(&lines[1..3], &["usage:", "  short"]);
        assert!(lines[0].starts_with("kiln-cli: éé") && lines[0].ends_with('…'));
        assert_eq!(cap_text("a\nb\n"), "a\nb\n");
    }

    #[test]
    fn every_error_response_is_capped() {
        let mut h = host();
        call(
            &mut h,
            r#"{"command":"doc.open","params":{"path":"a.txt"}}"#,
        );
        let a = "a".repeat(5000);
        let sp = "a b ".repeat(2000);
        let short_strings = vec!["a b"; 2000];
        let short_obj: Map<String, Value> =
            (0..2000).map(|i| (format!("k{i}"), json!("a b"))).collect();
        let lines = [
            json!({"command": a}).to_string(),
            json!({"command": "doc.list", sp.clone(): 1}).to_string(),
            json!({"command": "doc.open", "params": {"path": sp}}).to_string(),
            json!({"command": "doc.open", "params": {"path": format!("../{sp}")}}).to_string(),
            json!({"command": "doc.insert", "params": {"doc": sp, "offset": 0, "text": "x"}})
                .to_string(),
            // R2-D1: arrays and objects of short strings to commands without params.
            json!({"command": "doc.list", "params": {"x": short_strings}}).to_string(),
            json!({"command": "buffer.text", "params": {"x": short_obj}}).to_string(),
            json!({"command": "doc.insert", "params": {"doc": [sp, sp], "offset": 0, "text": "x"}})
                .to_string(),
            // Many unknown keys: an unbounded message (R2-N2).
            Value::Object((0..300).map(|i| (format!("key {i}"), json!(1))).collect()).to_string(),
            format!("{{\"command\":\"{}", "é".repeat(3000)),
        ];
        for line in &lines {
            let out = h.handle_line(line.as_bytes()).unwrap();
            assert!(out.contains(r#""ok":false"#), "{out:.200}");
            assert_capped(&out);
        }
        // Short errors are not touched.
        let v = call(&mut h, r#"{"command":"no.such"}"#);
        assert_eq!(v["error"]["message"], "unknown command `no.such`");
    }

    #[test]
    fn command_error_json_has_kind_and_caps() {
        let e = CommandError::rejected(
            RejectReason::NotFound,
            format!("cannot open {}: not found", "z".repeat(400)),
        );
        let v = command_error_json("doc.open", &json!({"path": "z".repeat(400)}), &e);
        assert_eq!(v["code"], "rejected");
        assert_eq!(v["kind"], "not_found");
        assert!(v.get("id").is_none());
        assert_eq!(
            v["message"].as_str().unwrap().chars().count(),
            MAX_ERROR_STRING
        );
        let e = CommandError::UnknownCommand {
            message: "unknown command `q`".into(),
        };
        assert_eq!(
            command_error_json("q", &json!({}), &e),
            json!({"code": "unknown_command", "kind": "unknown_command", "message": "unknown command `q`"})
        );
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
