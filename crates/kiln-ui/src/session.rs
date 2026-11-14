//! The session the UI and the script share: one [`kiln_control::Host`] (ADR-0008, ruling Q-1 A).
//!
//! Every request, the script's and the UI's own, goes through the host's
//! [`Host::handle_line`], i.e. through the control protocol and the command registry, so every
//! successful one is in the session log. The UI knows which documents are open and which one is
//! current from the responses of successful requests; what it shows of a document it reads with
//! read-only requests ([`Session::read`]), which the editor view sends only in the first frame
//! after a request or an input (T-056, "Frames, reads and the log").

use kiln_control::Host;
use serde_json::{Value, json};

use crate::json::canonical;

/// A refused request: its `kind` and `message`.
pub(crate) type Refused = (String, String);

/// The host and what the UI knows about it.
pub(crate) struct Session {
    host: Host,
    /// Open documents `(id, path)`, in id order, as the responses of `doc.open` and
    /// `doc.close` reported them.
    docs: Vec<(u64, String)>,
    /// The current document (T-056): the one most recently opened by a successful `doc.open`;
    /// after it is closed, the open document with the highest id.
    current: Option<u64>,
    /// Counts the successful requests other than the view's own reads (and changes of the
    /// current document): the view re-reads when this differs from what it last saw.
    changes: u64,
}

impl Session {
    pub(crate) fn new(host: Host) -> Self {
        Session {
            host,
            docs: Vec::new(),
            current: None,
            changes: 0,
        }
    }

    /// One request line in, the host's response out (`None` for a blank line), exactly as
    /// [`Host::handle_line`]; a successful response also updates what the UI knows.
    pub(crate) fn handle_line(&mut self, line: &[u8]) -> Option<String> {
        let response = self.host.handle_line(line)?;
        if self.observe(line, &response) {
            self.changes += 1;
        }
        Some(response)
    }

    /// Send the control request `command` with `params` (a request of the UI's own; it counts
    /// as a request for [`Session::changes`]). The result, or the refusal's kind and message.
    pub(crate) fn request(&mut self, command: &str, params: Value) -> Result<Value, Refused> {
        let line = canonical(&json!({"command": command, "params": params}));
        let response = self.handle_line(line.as_bytes()).unwrap_or_default();
        result_of(&response)
    }

    /// Send the read-only control request `command` (`mutating: false`); unlike
    /// [`Session::request`] it does not count as a change. It is logged like every successful
    /// request.
    pub(crate) fn read(&mut self, command: &str, params: Value) -> Result<Value, Refused> {
        let line = canonical(&json!({"command": command, "params": params}));
        let response = self.host.handle_line(line.as_bytes()).unwrap_or_default();
        result_of(&response)
    }

    /// The session log as a script ([`Host::log_script`]).
    pub(crate) fn log_script(&self) -> String {
        self.host.log_script()
    }

    /// The open documents `(id, path)` in id order.
    pub(crate) fn docs(&self) -> &[(u64, String)] {
        &self.docs
    }

    /// The current document, if any is open.
    pub(crate) fn current(&self) -> Option<u64> {
        self.current
    }

    /// Make `doc` the current document (for a panel that switches documents, T-058); an id
    /// that is not open is ignored.
    #[allow(dead_code)] // used by the panels of T-057 to T-061 (`panels::PanelView`)
    pub(crate) fn set_current(&mut self, doc: u64) {
        if self.docs.iter().any(|(id, _)| *id == doc) && self.current != Some(doc) {
            self.current = Some(doc);
            self.changes += 1;
        }
    }

    /// A number that changes with every successful request other than [`Session::read`]s.
    pub(crate) fn changes(&self) -> u64 {
        self.changes
    }

    /// Follow a response; true if it reported `ok:true`.
    fn observe(&mut self, line: &[u8], response: &str) -> bool {
        let Ok(response) = serde_json::from_str::<Value>(response) else {
            return false;
        };
        if response.get("ok") != Some(&Value::Bool(true)) {
            return false;
        }
        let Ok(request) = serde_json::from_slice::<Value>(line) else {
            return true;
        };
        let result = &response["result"];
        match request.get("command").and_then(Value::as_str) {
            Some("doc.open") => {
                if let (Some(doc), Some(path)) = (
                    result.get("doc").and_then(Value::as_u64),
                    result.get("path").and_then(Value::as_str),
                ) {
                    if let Err(at) = self.docs.binary_search_by_key(&doc, |(id, _)| *id) {
                        self.docs.insert(at, (doc, path.to_owned()));
                    }
                    self.current = Some(doc);
                }
            }
            Some("doc.close") => {
                if let Some(doc) = request
                    .get("params")
                    .and_then(|p| p.get("doc"))
                    .and_then(Value::as_u64)
                {
                    self.docs.retain(|(id, _)| *id != doc);
                    if self.current == Some(doc) {
                        self.current = self.docs.last().map(|(id, _)| *id);
                    }
                }
            }
            _ => {}
        }
        true
    }
}

/// The `result` of a response line, or its error's kind and message.
fn result_of(response: &str) -> Result<Value, Refused> {
    let mut v: Value = serde_json::from_str(response).unwrap_or(Value::Null);
    if v.get("ok") == Some(&Value::Bool(true)) {
        return Ok(v["result"].take());
    }
    let error = &v["error"];
    Err((
        error["kind"].as_str().unwrap_or("internal").to_owned(),
        error["message"].as_str().unwrap_or("").to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> Session {
        let host = Host::in_memory(&[("a.txt", b"a\n"), ("b.txt", b"b\n")]).unwrap();
        Session::new(host)
    }

    #[test]
    fn opens_and_closes_are_followed_in_id_order() {
        let mut s = session();
        s.handle_line(br#"{"command":"doc.open","params":{"path":"b.txt"}}"#);
        s.handle_line(br#"{"command":"doc.open","params":{"path":"a.txt"}}"#);
        s.handle_line(br#"{"command":"doc.open","params":{"path":"a.txt"}}"#);
        let paths: Vec<&str> = s.docs().iter().map(|(_, p)| p.as_str()).collect();
        assert_eq!(paths, ["b.txt", "a.txt"]);
        s.handle_line(br#"{"command":"doc.close","params":{"doc":1}}"#);
        assert_eq!(s.docs(), [(2, "a.txt".to_owned())]);
    }

    #[test]
    fn the_current_document_follows_opens_and_closes() {
        let mut s = session();
        assert_eq!(s.current(), None);
        s.request("doc.open", json!({"path": "a.txt"})).unwrap();
        s.request("doc.open", json!({"path": "b.txt"})).unwrap();
        assert_eq!(s.current(), Some(2));
        // An open that answers `already_open: true` makes it current too.
        s.request("doc.open", json!({"path": "a.txt"})).unwrap();
        assert_eq!(s.current(), Some(1));
        s.request("doc.close", json!({"doc": 2})).unwrap();
        assert_eq!(s.current(), Some(1));
        s.request("doc.open", json!({"path": "b.txt"})).unwrap();
        assert_eq!(s.current(), Some(3));
        // Closing the current one makes the highest open id current.
        s.request("doc.close", json!({"doc": 3})).unwrap();
        assert_eq!(s.current(), Some(1));
        s.request("doc.close", json!({"doc": 1})).unwrap();
        assert_eq!(s.current(), None);
    }

    #[test]
    fn reads_are_logged_but_are_not_changes() {
        let mut s = session();
        s.request("doc.open", json!({"path": "a.txt"})).unwrap();
        let changes = s.changes();
        let text = s.read("doc.text", json!({"doc": 1})).unwrap();
        assert_eq!(text["text"], "a\n");
        assert_eq!(s.changes(), changes);
        assert!(s.log_script().contains("doc.text"));
        let refused = s.request("doc.text", json!({"doc": 9})).unwrap_err();
        assert_eq!(refused.0, "no_such_document");
        assert_eq!(s.changes(), changes);
        s.request("doc.list", json!({})).unwrap();
        assert_eq!(s.changes(), changes + 1);
    }

    #[test]
    fn refused_requests_change_nothing() {
        let mut s = session();
        let r = s
            .handle_line(br#"{"command":"doc.open","params":{"path":"missing"}}"#)
            .unwrap();
        assert!(r.contains(r#""ok":false"#));
        s.handle_line(br#"{"command":"doc.close","params":{"doc":7}}"#);
        assert!(s.docs().is_empty());
        assert_eq!(s.log_script(), "");
        assert_eq!(s.changes(), 0);
    }

    #[test]
    fn responses_are_the_hosts() {
        let mut s = session();
        let mut h = Host::in_memory(&[("a.txt", b"a\n"), ("b.txt", b"b\n")]).unwrap();
        for line in [
            &br#"{"id":1,"command":"doc.open","params":{"path":"a.txt"}}"#[..],
            b"not json",
            b"   ",
            br#"{"id":2,"command":"doc.list"}"#,
        ] {
            assert_eq!(s.handle_line(line), h.handle_line(line));
        }
        assert_eq!(s.log_script(), h.log_script());
    }
}
