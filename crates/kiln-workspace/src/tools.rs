//! External tools (`cargo`, `rustfmt`) behind a trait.
//!
//! Commands never start a process themselves: they hand a [`ToolRequest`] to a
//! [`ToolRunner`]. The default runner, [`NoTools`], refuses every request, so no process can
//! be started unless a session explicitly installs another runner (CHARTER §3 decision 4).

use std::fmt;

/// An external tool Kiln can ask a [`ToolRunner`] to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tool {
    /// `cargo` (e.g. `cargo check`).
    Cargo,
    /// `rustfmt`.
    Rustfmt,
}

impl Tool {
    /// The program name: `"cargo"` or `"rustfmt"`.
    pub fn as_str(self) -> &'static str {
        match self {
            Tool::Cargo => "cargo",
            Tool::Rustfmt => "rustfmt",
        }
    }
}

/// One run of an external tool. The working directory is the project root.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ToolRequest {
    /// The tool to run.
    pub tool: Tool,
    /// Its arguments, in order.
    pub args: Vec<String>,
    /// Text written to its standard input, if any.
    pub stdin: Option<String>,
}

/// What a finished tool run produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolOutput {
    /// The exit status; `None` if the process was killed by a signal.
    pub status: Option<i32>,
    /// Its standard output.
    pub stdout: String,
    /// Its standard error.
    pub stderr: String,
}

/// Why a tool request produced no [`ToolOutput`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolError {
    /// External tools are off for this session (the [`NoTools`] runner).
    Unavailable,
    /// The tool's program was not found.
    NotFound {
        /// The tool.
        tool: Tool,
    },
    /// The tool did not finish within `seconds`.
    Timeout {
        /// The tool.
        tool: Tool,
        /// The time limit, in seconds.
        seconds: u64,
    },
    /// The tool could not be run.
    Failed {
        /// The tool.
        tool: Tool,
        /// What went wrong.
        message: String,
    },
    /// A replaying runner has no recorded output for this request.
    NoRecording {
        /// The tool.
        tool: Tool,
    },
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolError::Unavailable => f.write_str(
                "external tools are off for this session (start with --tools real to run them)",
            ),
            ToolError::NotFound { tool } => write!(f, "`{}` was not found", tool.as_str()),
            ToolError::Timeout { tool, seconds } => {
                write!(f, "`{}` timed out after {seconds} s", tool.as_str())
            }
            ToolError::Failed { tool, message } => {
                write!(f, "`{}` failed: {message}", tool.as_str())
            }
            ToolError::NoRecording { tool } => {
                write!(f, "no recorded output for this `{}` request", tool.as_str())
            }
        }
    }
}

impl std::error::Error for ToolError {}

/// Runs external tools for a session.
pub trait ToolRunner: fmt::Debug {
    /// Run `request` and return its output, or why there is none.
    fn run(&mut self, request: &ToolRequest) -> Result<ToolOutput, ToolError>;
}

/// The default runner: external tools are off, every run is refused with
/// [`ToolError::Unavailable`] and no process is ever started.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoTools;

impl ToolRunner for NoTools {
    fn run(&mut self, _request: &ToolRequest) -> Result<ToolOutput, ToolError> {
        Err(ToolError::Unavailable)
    }
}
