//! The session the UI and the script share: one [`kiln_control::Host`] (ADR-0008, ruling Q-1 A).
//!
//! Every request, the script's and the UI's own, goes through [`Session::handle_line`], i.e.
//! through the control protocol and the command registry. The UI reads what it shows from the
//! responses of successful requests (which documents are open); it never reads or changes the
//! session any other way, and it never sends a request of its own to look at the state, so a
//! script of control requests leaves exactly the log `kiln-cli` would.

use kiln_control::Host;
use serde_json::Value;

/// The host and what the UI knows about it.
pub(crate) struct Session {
    host: Host,
    /// Open documents `(id, path)`, in id order, as the responses of `doc.open` and
    /// `doc.close` reported them.
    docs: Vec<(u64, String)>,
}

impl Session {
    pub(crate) fn new(host: Host) -> Self {
        Session {
            host,
            docs: Vec::new(),
        }
    }

    /// One request line in, the host's response out (`None` for a blank line), exactly as
    /// [`Host::handle_line`]; a successful response also updates what the UI shows.
    pub(crate) fn handle_line(&mut self, line: &[u8]) -> Option<String> {
        let response = self.host.handle_line(line)?;
        self.observe(line, &response);
        Some(response)
    }

    /// The session log as a script ([`Host::log_script`]).
    pub(crate) fn log_script(&self) -> String {
        self.host.log_script()
    }

    /// The open documents `(id, path)` in id order.
    pub(crate) fn docs(&self) -> &[(u64, String)] {
        &self.docs
    }

    fn observe(&mut self, line: &[u8], response: &str) {
        let Ok(response) = serde_json::from_str::<Value>(response) else {
            return;
        };
        if response.get("ok") != Some(&Value::Bool(true)) {
            return;
        }
        let Ok(request) = serde_json::from_slice::<Value>(line) else {
            return;
        };
        let result = &response["result"];
        match request.get("command").and_then(Value::as_str) {
            Some("doc.open") => {
                let (Some(doc), Some(path)) = (
                    result.get("doc").and_then(Value::as_u64),
                    result.get("path").and_then(Value::as_str),
                ) else {
                    return;
                };
                if let Err(at) = self.docs.binary_search_by_key(&doc, |(id, _)| *id) {
                    self.docs.insert(at, (doc, path.to_owned()));
                }
            }
            Some("doc.close") => {
                if let Some(doc) = request
                    .get("params")
                    .and_then(|p| p.get("doc"))
                    .and_then(Value::as_u64)
                {
                    self.docs.retain(|(id, _)| *id != doc);
                }
            }
            _ => {}
        }
    }
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
    fn refused_requests_change_nothing() {
        let mut s = session();
        let r = s
            .handle_line(br#"{"command":"doc.open","params":{"path":"missing"}}"#)
            .unwrap();
        assert!(r.contains(r#""ok":false"#));
        s.handle_line(br#"{"command":"doc.close","params":{"doc":7}}"#);
        assert!(s.docs().is_empty());
        assert_eq!(s.log_script(), "");
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
