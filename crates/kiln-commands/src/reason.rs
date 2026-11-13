//! Structured reasons of command errors (ADR-0004).
//!
//! Every [`CommandError::Rejected`](crate::CommandError::Rejected) and
//! [`CommandError::Internal`](crate::CommandError::Internal) carries a reason, set where the
//! error is raised. The control protocol reports `kind = reason.as_str()` and never reads the
//! message. [`Registry::describe`](crate::Registry::describe) publishes every reason with its
//! description as `error_kinds`, so the list in `docs/commands.json` is generated from these
//! enums.
//!
//! Adding a kind: add a variant (and its arm in `as_str` and `description`, and its entry in
//! `ALL`), then run `cargo xtask schema --write`. A lower-layer error enum maps to reasons by
//! an exhaustive `From` without a catch-all arm, so a new lower-layer variant does not compile
//! until it is mapped. Errors raised by one command group only are mapped in that group's
//! module; this file holds the shared mappings.

use kiln_text::EditError;
use kiln_workspace::{OpenError, ToolError, VfsError};
use serde::{Deserialize, Serialize};

/// Why a command refused a request (the `kind` of a `rejected` error).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectReason {
    /// The path is not a valid root-relative path.
    InvalidPath,
    /// No open document has this id.
    NoSuchDocument,
    /// `doc.close` of a modified document without `discard`.
    UnsavedChanges,
    /// The file (or, for a save, its directory) does not exist.
    NotFound,
    /// The path names a directory.
    NotAFile,
    /// Any other file-system failure.
    IoError,
    /// The file is not valid UTF-8 / UTF-16 text.
    DecodeError,
    /// An offset beyond the end of the text.
    OutOfRange,
    /// A range whose start is after its end.
    InvertedRange,
    /// Any other refusal.
    Other,
    /// The search pattern is empty or is not a valid regular expression.
    InvalidPattern,
    /// A file of a replace preview changed or was removed since the preview.
    StalePreview,
    /// No stored replace preview has this id.
    NoSuchPreview,
    /// External tools are off for this session.
    ToolUnavailable,
    /// The external tool's program was not found.
    ToolNotFound,
    /// The external tool ran longer than its limit and was stopped.
    ToolTimeout,
    /// The external tool failed or gave no usable result.
    ToolFailed,
    /// The replayed tool recording has no output for this request.
    NoRecording,
    /// rustfmt could not format the text.
    FormatError,
    /// The file changed or was removed on disk since it was read.
    ChangedOnDisk,
    /// The keymap file or chord is not valid.
    InvalidKeymap,
}

impl RejectReason {
    /// Every variant, in declaration order.
    pub const ALL: &'static [RejectReason] = &[
        RejectReason::InvalidPath,
        RejectReason::NoSuchDocument,
        RejectReason::UnsavedChanges,
        RejectReason::NotFound,
        RejectReason::NotAFile,
        RejectReason::IoError,
        RejectReason::DecodeError,
        RejectReason::OutOfRange,
        RejectReason::InvertedRange,
        RejectReason::Other,
        RejectReason::InvalidPattern,
        RejectReason::StalePreview,
        RejectReason::NoSuchPreview,
        RejectReason::ToolUnavailable,
        RejectReason::ToolNotFound,
        RejectReason::ToolTimeout,
        RejectReason::ToolFailed,
        RejectReason::NoRecording,
        RejectReason::FormatError,
        RejectReason::ChangedOnDisk,
        RejectReason::InvalidKeymap,
    ];

    /// The protocol kind: the snake-case name, equal to the serde form.
    pub fn as_str(self) -> &'static str {
        match self {
            RejectReason::InvalidPath => "invalid_path",
            RejectReason::NoSuchDocument => "no_such_document",
            RejectReason::UnsavedChanges => "unsaved_changes",
            RejectReason::NotFound => "not_found",
            RejectReason::NotAFile => "not_a_file",
            RejectReason::IoError => "io_error",
            RejectReason::DecodeError => "decode_error",
            RejectReason::OutOfRange => "out_of_range",
            RejectReason::InvertedRange => "inverted_range",
            RejectReason::Other => "other",
            RejectReason::InvalidPattern => "invalid_pattern",
            RejectReason::StalePreview => "stale_preview",
            RejectReason::NoSuchPreview => "no_such_preview",
            RejectReason::ToolUnavailable => "tool_unavailable",
            RejectReason::ToolNotFound => "tool_not_found",
            RejectReason::ToolTimeout => "tool_timeout",
            RejectReason::ToolFailed => "tool_failed",
            RejectReason::NoRecording => "no_recording",
            RejectReason::FormatError => "format_error",
            RejectReason::ChangedOnDisk => "changed_on_disk",
            RejectReason::InvalidKeymap => "invalid_keymap",
        }
    }

    /// When this kind is reported (the `when` of `error_kinds` in `docs/commands.json`).
    pub fn description(self) -> &'static str {
        match self {
            RejectReason::InvalidPath => {
                "the path is not a valid root-relative path (see Paths in `docs/control-protocol.md`)"
            }
            RejectReason::NoSuchDocument => "no open document has this id",
            RejectReason::UnsavedChanges => "`doc.close` of a modified document without `discard`",
            RejectReason::NotFound => "the file (or, for a save, its directory) does not exist",
            RejectReason::NotAFile => "the path names a directory",
            RejectReason::IoError => {
                "any other file-system failure (permissions, name too long, …)"
            }
            RejectReason::DecodeError => "the file is not valid UTF-8 / UTF-16 text",
            RejectReason::OutOfRange => "an offset beyond the end of the text",
            RejectReason::InvertedRange => "a range whose start is after its end",
            RejectReason::Other => "any other refusal",
            RejectReason::InvalidPattern => {
                "the search pattern is empty or is not a valid regular expression"
            }
            RejectReason::StalePreview => {
                "a file of a replace preview changed or was removed since the preview"
            }
            RejectReason::NoSuchPreview => "no stored replace preview has this id",
            RejectReason::ToolUnavailable => {
                "external tools are off for this session (the default)"
            }
            RejectReason::ToolNotFound => "the external tool's program was not found",
            RejectReason::ToolTimeout => {
                "the external tool ran longer than its limit and was stopped"
            }
            RejectReason::ToolFailed => "the external tool failed or gave no usable result",
            RejectReason::NoRecording => {
                "the replayed tool recording has no output for this request"
            }
            RejectReason::FormatError => "rustfmt could not format the text",
            RejectReason::ChangedOnDisk => {
                "the file changed or was removed on disk since it was read"
            }
            RejectReason::InvalidKeymap => "the keymap file or chord is not valid",
        }
    }
}

/// Why a command failed through a bug in Kiln (the `kind` of an `internal` error).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InternalReason {
    /// A command panicked.
    Panic,
    /// A command's result violates its own schema.
    ResultSchema,
    /// Any other internal error (e.g. a result that does not serialize).
    Other,
}

impl InternalReason {
    /// Every variant, in declaration order.
    pub const ALL: &'static [InternalReason] = &[
        InternalReason::Panic,
        InternalReason::ResultSchema,
        InternalReason::Other,
    ];

    /// The protocol kind: the snake-case name, equal to the serde form.
    pub fn as_str(self) -> &'static str {
        match self {
            InternalReason::Panic => "panic",
            InternalReason::ResultSchema => "result_schema",
            InternalReason::Other => "other",
        }
    }

    /// When this kind is reported (the `when` of `error_kinds` in `docs/commands.json`).
    pub fn description(self) -> &'static str {
        match self {
            InternalReason::Panic => "a command panicked (a bug in Kiln)",
            InternalReason::ResultSchema => {
                "a command's result violates its own schema (a bug in Kiln)"
            }
            InternalReason::Other => "any other internal error",
        }
    }
}

// The shared mappings. Each is an exhaustive match with no catch-all arm: a variant added to
// the lower-layer enum must be mapped here before the crate compiles.

impl From<&VfsError> for RejectReason {
    fn from(e: &VfsError) -> Self {
        match e {
            VfsError::NotFound { .. } => RejectReason::NotFound,
            VfsError::NotAFile { .. } => RejectReason::NotAFile,
            VfsError::Io { .. } => RejectReason::IoError,
        }
    }
}

impl From<&OpenError> for RejectReason {
    fn from(e: &OpenError) -> Self {
        match e {
            OpenError::Vfs(e) => RejectReason::from(e),
            OpenError::Decode { .. } => RejectReason::DecodeError,
        }
    }
}

impl From<&EditError> for RejectReason {
    fn from(e: &EditError) -> Self {
        match e {
            EditError::OutOfBounds { .. } => RejectReason::OutOfRange,
            EditError::InvertedRange { .. } => RejectReason::InvertedRange,
        }
    }
}

impl From<&ToolError> for RejectReason {
    fn from(e: &ToolError) -> Self {
        match e {
            ToolError::Unavailable => RejectReason::ToolUnavailable,
            ToolError::NotFound { .. } => RejectReason::ToolNotFound,
            ToolError::Timeout { .. } => RejectReason::ToolTimeout,
            ToolError::Failed { .. } => RejectReason::ToolFailed,
            ToolError::NoRecording { .. } => RejectReason::NoRecording,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use kiln_workspace::{DecodeError, Encoding, RelPath, Tool};

    use super::*;

    #[test]
    fn as_str_is_the_serde_form_and_all_is_complete() {
        for r in RejectReason::ALL {
            assert_eq!(serde_json::to_value(r).expect("ser"), r.as_str());
            let back: RejectReason = serde_json::from_value(r.as_str().into()).expect("de");
            assert_eq!(back, *r);
            assert!(!r.description().trim().is_empty(), "{r:?}");
        }
        for r in InternalReason::ALL {
            assert_eq!(serde_json::to_value(r).expect("ser"), r.as_str());
            let back: InternalReason = serde_json::from_value(r.as_str().into()).expect("de");
            assert_eq!(back, *r);
            assert!(!r.description().trim().is_empty(), "{r:?}");
        }
        let kinds: BTreeSet<_> = RejectReason::ALL.iter().map(|r| r.as_str()).collect();
        assert_eq!(kinds.len(), RejectReason::ALL.len(), "duplicate in ALL");
        assert_eq!(RejectReason::ALL.len(), 21);
        assert_eq!(
            InternalReason::ALL
                .iter()
                .map(|r| r.as_str())
                .collect::<Vec<_>>(),
            ["panic", "result_schema", "other"]
        );
    }

    /// `ALL` is in declaration order: the derived `Hash`/`Eq` give no order, so check the
    /// discriminants (declaration order) against the position in `ALL`.
    #[test]
    fn all_is_in_declaration_order() {
        for (i, r) in RejectReason::ALL.iter().enumerate() {
            assert_eq!(*r as usize, i, "{r:?}");
        }
        for (i, r) in InternalReason::ALL.iter().enumerate() {
            assert_eq!(*r as usize, i, "{r:?}");
        }
    }

    #[test]
    fn lower_layer_errors_map_to_their_reasons() {
        let p = RelPath::new("a.txt").expect("path");
        let nf = VfsError::NotFound { path: p.clone() };
        let dir = VfsError::NotAFile { path: p.clone() };
        let io = VfsError::Io {
            path: p.clone(),
            kind: std::io::ErrorKind::PermissionDenied,
            message: "denied".into(),
        };
        assert_eq!(RejectReason::from(&nf), RejectReason::NotFound);
        assert_eq!(RejectReason::from(&dir), RejectReason::NotAFile);
        assert_eq!(RejectReason::from(&io), RejectReason::IoError);
        assert_eq!(
            RejectReason::from(&OpenError::Vfs(nf)),
            RejectReason::NotFound
        );
        assert_eq!(
            RejectReason::from(&OpenError::Vfs(dir)),
            RejectReason::NotAFile
        );
        assert_eq!(
            RejectReason::from(&OpenError::Vfs(io)),
            RejectReason::IoError
        );
        let decode = OpenError::Decode {
            path: p,
            error: DecodeError {
                encoding: Encoding::Utf8,
                byte_offset: 0,
            },
        };
        assert_eq!(RejectReason::from(&decode), RejectReason::DecodeError);
        assert_eq!(
            RejectReason::from(&EditError::OutOfBounds {
                index: 9,
                len_chars: 1
            }),
            RejectReason::OutOfRange
        );
        assert_eq!(
            RejectReason::from(&EditError::InvertedRange { start: 2, end: 1 }),
            RejectReason::InvertedRange
        );
        let t = Tool::Cargo;
        for (e, want) in [
            (ToolError::Unavailable, RejectReason::ToolUnavailable),
            (ToolError::NotFound { tool: t }, RejectReason::ToolNotFound),
            (
                ToolError::Timeout {
                    tool: t,
                    seconds: 1,
                },
                RejectReason::ToolTimeout,
            ),
            (
                ToolError::Failed {
                    tool: t,
                    message: "x".into(),
                },
                RejectReason::ToolFailed,
            ),
            (
                ToolError::NoRecording { tool: t },
                RejectReason::NoRecording,
            ),
        ] {
            assert_eq!(RejectReason::from(&e), want, "{e:?}");
        }
    }
}
