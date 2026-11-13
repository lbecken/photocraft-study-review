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
//! `{"id":…,"ok":true,"result":…}` or `{"error":{"code":…,"message":…[,"details":[…]]},"id":…,"ok":false}`.
//!
//! Responses depend only on the request lines and the initial files: no clock, randomness,
//! thread timing or host path enters them (`DiskFs` error messages name `RelPath`s only).

use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use kiln_commands::{Context, LogEntry, Registry, standard_registry};
use kiln_workspace::DiskFs;
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

    /// A host over the disk file system rooted at `root`. Fails if `root` is not a directory.
    pub fn on_disk(root: &Path) -> Result<Host, HostError> {
        let fs = DiskFs::new(root).map_err(|e| HostError(e.to_string()))?;
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
        let response = match parse_request(line) {
            Err(failure) => failure,
            Ok(Request { id, command, params }) => {
                let registry = &self.registry;
                let ctx = &mut self.ctx;
                // A panicking command must not take the host down (the context may be left
                // partly changed; that is a bug in the command, reported as `internal`).
                let outcome = catch_unwind(AssertUnwindSafe(|| {
                    registry.execute(ctx, &command, params)
                }))
                .unwrap_or_else(|_| {
                    Err(kiln_commands::CommandError::Internal {
                        message: format!("command `{command}` panicked"),
                    })
                });
                match outcome {
                    Ok(result) => json!({"id": id, "ok": true, "result": result}),
                    Err(e) => json!({"error": e.to_json(), "id": id, "ok": false}),
                }
            }
        };
        Some(canonical(&response))
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

/// A request that passed protocol validation (not yet the registry's).
struct Request {
    id: Value,
    command: String,
    params: Value,
}

/// True iff `line` is empty or consists only of whitespace (any Unicode white space; a line
/// that is not UTF-8 is never blank).
fn is_blank(line: &[u8]) -> bool {
    std::str::from_utf8(line).is_ok_and(|s| s.chars().all(char::is_whitespace))
}

fn failure(id: Value, code: &str, message: impl Into<String>) -> Value {
    json!({"error": {"code": code, "message": message.into()}, "id": id, "ok": false})
}

/// Validate one non-blank line as a protocol request; on failure, the error response.
fn parse_request(line: &[u8]) -> Result<Request, Value> {
    let text = std::str::from_utf8(line).map_err(|e| {
        failure(
            Value::Null,
            "invalid_json",
            format!("request is not valid UTF-8: {e}"),
        )
    })?;
    let value: Value = serde_json::from_str(text).map_err(|e| {
        failure(
            Value::Null,
            "invalid_json",
            format!("request is not JSON: {e}"),
        )
    })?;
    let Value::Object(mut obj) = value else {
        return Err(failure(
            Value::Null,
            "invalid_request",
            "request must be a JSON object",
        ));
    };
    let id = match obj.remove("id") {
        None => Value::Null,
        Some(v @ Value::String(_)) => v,
        Some(Value::Number(n)) if n.is_i64() || n.is_u64() => Value::Number(n),
        Some(_) => {
            return Err(failure(
                Value::Null,
                "invalid_request",
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
        return Err(failure(
            id,
            "invalid_request",
            format!("unknown top-level key(s) {list}; allowed: `command`, `id`, `params`"),
        ));
    }
    let command = match obj.remove("command") {
        Some(Value::String(s)) => s,
        Some(_) => {
            return Err(failure(id, "invalid_request", "`command` must be a string"));
        }
        None => return Err(failure(id, "invalid_request", "`command` is missing")),
    };
    let params = match obj.remove("params") {
        None => Value::Object(Map::new()),
        Some(v @ Value::Object(_)) => v,
        Some(_) => {
            return Err(failure(id, "invalid_request", "`params` must be an object"));
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
        assert_eq!(call(&mut h, &list("18446744073709551615"))["id"], json!(u64::MAX));
        let v = call(&mut h, r#"{"command":"doc.list"}"#);
        assert_eq!(v["id"], Value::Null);
        assert_eq!(v["ok"], true);
    }

    fn code_and_id(h: &mut Host, line: &[u8]) -> (String, Value) {
        let v: Value = serde_json::from_str(&h.handle_line(line).unwrap()).unwrap();
        assert_eq!(v["ok"], false, "{v}");
        assert!(v["error"]["message"].is_string(), "{v}");
        (v["error"]["code"].as_str().unwrap().to_owned(), v["id"].clone())
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
            (br#"{"id":1.5,"command":"doc.list"}"#, "invalid_request", Value::Null),
            (br#"{"id":null,"command":"doc.list"}"#, "invalid_request", Value::Null),
            (br#"{"id":"q","command":3}"#, "invalid_request", json!("q")),
            (br#"{"command":"doc.list","params":[]}"#, "invalid_request", Value::Null),
            (br#"{"command":"doc.list","params":null}"#, "invalid_request", Value::Null),
            (br#"{"id":2,"command":"doc.list","extra":1}"#, "invalid_request", json!(2)),
            (br#"{"command":"no.such"}"#, "unknown_command", Value::Null),
            (br#"{"command":"doc.insert","params":{"doc":1}}"#, "invalid_params", Value::Null),
            (br#"{"command":"doc.close","params":{"doc":9}}"#, "rejected", Value::Null),
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
        assert!(v["error"]["details"].as_array().is_some_and(|d| !d.is_empty()));
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
        assert_eq!(lines[4], r#"{"command":"doc.save","id":5,"params":{"doc":1}}"#);

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
        let e = Host::on_disk(&file).unwrap_err();
        assert!(!e.to_string().contains(&*file.to_string_lossy()), "{e}");
        assert!(Host::on_disk(Path::new("/no/such/dir/for/kiln")).is_err());
    }

    #[test]
    fn on_disk_matches_in_memory() {
        let dir = std::env::temp_dir().join(format!("kiln-control-unit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), b"hello\n").unwrap();
        let mut disk = Host::on_disk(&dir).unwrap();
        let mut mem = host();
        for l in SESSION.iter().copied().chain([
            r#"{"command":"doc.open","params":{"path":"missing.txt"}}"#,
        ]) {
            let d = disk.handle_line(l.as_bytes()).unwrap();
            assert_eq!(d, mem.handle_line(l.as_bytes()).unwrap());
            assert!(!d.contains(&*dir.to_string_lossy()));
        }
        assert_eq!(std::fs::read(dir.join("a.txt")).unwrap(), b"hello world\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
