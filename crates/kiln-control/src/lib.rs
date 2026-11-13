//! # kiln-control — layer L4
//!
//! Responsibility (CHARTER §3): the headless host and the JSON-lines control protocol. Every
//! request is a named command executed through the `kiln-commands` registry.
//!
//! May depend on: L0–L3. Headless and deterministic.
//!
//! The protocol is specified in `docs/control-protocol.md`.

use std::fmt;
use std::path::Path;

use kiln_commands::LogEntry;

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
    log: Vec<LogEntry>,
}

impl Host {
    /// A host over an in-memory file system holding `files` (path, bytes).
    pub fn in_memory(files: &[(&str, &[u8])]) -> Result<Host, HostError> {
        let _ = files;
        Err(HostError("not implemented".to_owned()))
    }

    /// A host over the disk file system rooted at `root`.
    pub fn on_disk(root: &Path) -> Result<Host, HostError> {
        let _ = root;
        Err(HostError("not implemented".to_owned()))
    }

    /// One request line in; `Some(response line without "\n")` out, `None` for a blank line.
    pub fn handle_line(&mut self, line: &[u8]) -> Option<String> {
        let _ = line;
        None
    }

    /// Every successfully executed command, in execution order.
    pub fn log(&self) -> &[LogEntry] {
        &self.log
    }

    /// The log as a script: one request per entry,
    /// `{"command":…,"id":<1-based index>,"params":…}`, keys sorted, each line `"\n"`-terminated.
    pub fn log_script(&self) -> String {
        String::new()
    }
}
