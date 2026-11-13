//! Structured reasons of command errors (ADR-0004).
//!
//! Every [`CommandError::Rejected`](crate::CommandError::Rejected) and
//! [`CommandError::Internal`](crate::CommandError::Internal) carries a reason, set where the
//! error is raised. The control protocol reports `kind = reason.as_str()` and never reads the
//! message. [`Registry::describe`](crate::Registry::describe) publishes every reason with its
//! description as `error_kinds`, so the list in `docs/commands.json` is generated from these
//! enums.
//!
//! Adding a kind: add one row to the table of [`RejectReason`] (or [`InternalReason`]) below,
//! then run `cargo xtask schema --write`. The row is the only place a variant is declared: the
//! enum, `ALL`, `as_str` and `description` are all generated from the table, so a variant
//! cannot exist without its `ALL` entry and its description. A lower-layer error enum maps to
//! reasons by an exhaustive `From` without a catch-all arm, so a new lower-layer variant does
//! not compile until it is mapped. Errors raised by one command group only are mapped in that
//! group's module; this file holds the shared mappings.

use kiln_text::{EditError, PositionError};
use kiln_workspace::{OpenError, ToolError, VfsError};
use serde::{Deserialize, Serialize};

/// Declares a reason enum from one table, one row per variant:
/// `/// doc` `Variant = "kind" => "description",`. Emits the enum (declaration order = row
/// order, snake-case serde form), `ALL` in row order, `as_str` and `description`.
macro_rules! reasons {
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $( $(#[$vmeta:meta])* $variant:ident = $kind:literal => $when:literal, )+
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name {
            $( $(#[$vmeta])* $variant, )+
        }

        impl $name {
            /// Every variant, in declaration order.
            pub const ALL: &'static [$name] = &[ $( $name::$variant, )+ ];

            /// The protocol kind: the snake-case name, equal to the serde form.
            pub fn as_str(self) -> &'static str {
                match self {
                    $( $name::$variant => $kind, )+
                }
            }

            /// When this kind is reported (the `when` of `error_kinds` in
            /// `docs/commands.json`).
            pub fn description(self) -> &'static str {
                match self {
                    $( $name::$variant => $when, )+
                }
            }
        }
    };
}

reasons! {
    /// Why a command refused a request (the `kind` of a `rejected` error).
    pub enum RejectReason {
        /// The path is not a valid root-relative path.
        InvalidPath = "invalid_path" =>
            "the path is not a valid root-relative path (see Paths in `docs/control-protocol.md`)",
        /// No open document has this id.
        NoSuchDocument = "no_such_document" => "no open document has this id",
        /// `doc.close` of a modified document without `discard`.
        UnsavedChanges = "unsaved_changes" =>
            "`doc.close` of a modified document without `discard`",
        /// The file (or, for a save, its directory) does not exist.
        NotFound = "not_found" => "the file (or, for a save, its directory) does not exist",
        /// The path names a directory.
        NotAFile = "not_a_file" => "the path names a directory",
        /// Any other file-system failure.
        IoError = "io_error" => "any other file-system failure (permissions, name too long, …)",
        /// The file is not valid UTF-8 / UTF-16 text.
        DecodeError = "decode_error" => "the file is not valid UTF-8 / UTF-16 text",
        /// An offset beyond the end of the text.
        OutOfRange = "out_of_range" => "an offset beyond the end of the text",
        /// A range whose start is after its end.
        InvertedRange = "inverted_range" => "a range whose start is after its end",
        /// Any other refusal.
        Other = "other" => "any other refusal",
        /// The search pattern is empty or is not a valid regular expression.
        InvalidPattern = "invalid_pattern" =>
            "the search pattern is empty or is not a valid regular expression",
        /// A file of a replace preview changed or was removed since the preview.
        StalePreview = "stale_preview" =>
            "a file of a replace preview changed or was removed since the preview",
        /// No stored replace preview has this id.
        NoSuchPreview = "no_such_preview" => "no stored replace preview has this id",
        /// External tools are off for this session.
        ToolUnavailable = "tool_unavailable" =>
            "external tools are off for this session (the default)",
        /// The external tool's program was not found.
        ToolNotFound = "tool_not_found" => "the external tool's program was not found",
        /// The external tool ran longer than its limit and was stopped.
        ToolTimeout = "tool_timeout" =>
            "the external tool ran longer than its limit and was stopped",
        /// The external tool failed or gave no usable result.
        ToolFailed = "tool_failed" => "the external tool failed or gave no usable result",
        /// The replayed tool recording has no output for this request.
        NoRecording = "no_recording" =>
            "the replayed tool recording has no output for this request",
        /// rustfmt could not format the text.
        FormatError = "format_error" => "rustfmt could not format the text",
        /// The file changed or was removed on disk since it was read.
        ChangedOnDisk = "changed_on_disk" =>
            "the file changed or was removed on disk since it was read",
        /// The keymap file or chord is not valid.
        InvalidKeymap = "invalid_keymap" => "the keymap file or chord is not valid",
        /// A position inside a character, a surrogate pair or a CRLF line break.
        NotABoundary = "not_a_boundary" =>
            "a position inside a character, a surrogate pair or a CRLF line break",
    }
}

reasons! {
    /// Why a command failed through a bug in Kiln (the `kind` of an `internal` error).
    pub enum InternalReason {
        /// A command panicked.
        Panic = "panic" => "a command panicked (a bug in Kiln)",
        /// A command's result violates its own schema.
        ResultSchema = "result_schema" =>
            "a command's result violates its own schema (a bug in Kiln)",
        /// Any other internal error (e.g. a result that does not serialize).
        Other = "other" => "any other internal error",
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

impl From<&PositionError> for RejectReason {
    fn from(e: &PositionError) -> Self {
        match e {
            PositionError::OutOfBounds { .. }
            | PositionError::LineOutOfBounds { .. }
            | PositionError::ColumnOutOfBounds { .. } => RejectReason::OutOfRange,
            PositionError::NotCharBoundary { .. }
            | PositionError::InsideSurrogatePair { .. }
            | PositionError::InsideLineBreak { .. } => RejectReason::NotABoundary,
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

    /// The kinds pinned by T-032, in pinned order. Later tasks append rows; this checks that
    /// the pinned names stay a subsequence of `ALL` in this order, not the count.
    const PINNED_REJECTED: [&str; 21] = [
        "invalid_path",
        "no_such_document",
        "unsaved_changes",
        "not_found",
        "not_a_file",
        "io_error",
        "decode_error",
        "out_of_range",
        "inverted_range",
        "other",
        "invalid_pattern",
        "stale_preview",
        "no_such_preview",
        "tool_unavailable",
        "tool_not_found",
        "tool_timeout",
        "tool_failed",
        "no_recording",
        "format_error",
        "changed_on_disk",
        "invalid_keymap",
    ];
    const PINNED_INTERNAL: [&str; 3] = ["panic", "result_schema", "other"];

    /// True iff `pinned` appears in `all` in order (other entries may lie between).
    fn is_ordered_subsequence(pinned: &[&str], all: &[&str]) -> bool {
        let mut rest = all.iter();
        pinned.iter().all(|p| rest.any(|a| a == p))
    }

    #[test]
    fn as_str_is_the_serde_form_and_pinned_kinds_are_in_all_in_order() {
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
        let kinds: Vec<&str> = RejectReason::ALL.iter().map(|r| r.as_str()).collect();
        let unique: BTreeSet<_> = kinds.iter().collect();
        assert_eq!(unique.len(), kinds.len(), "duplicate in ALL");
        assert!(
            is_ordered_subsequence(&PINNED_REJECTED, &kinds),
            "{kinds:?}"
        );
        let kinds: Vec<&str> = InternalReason::ALL.iter().map(|r| r.as_str()).collect();
        assert!(
            is_ordered_subsequence(&PINNED_INTERNAL, &kinds),
            "{kinds:?}"
        );
        // The helper itself: order matters, gaps are allowed.
        assert!(is_ordered_subsequence(&["a", "c"], &["a", "b", "c"]));
        assert!(!is_ordered_subsequence(&["c", "a"], &["a", "b", "c"]));
        assert!(!is_ordered_subsequence(&["d"], &["a", "b", "c"]));
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

    #[test]
    fn position_errors_map_to_out_of_range_or_not_a_boundary() {
        for (e, want) in [
            (
                PositionError::OutOfBounds { index: 9, len: 1 },
                RejectReason::OutOfRange,
            ),
            (
                PositionError::LineOutOfBounds {
                    line: 4,
                    len_lines: 4,
                },
                RejectReason::OutOfRange,
            ),
            (
                PositionError::ColumnOutOfBounds {
                    line: 1,
                    col: 4,
                    line_len: 3,
                },
                RejectReason::OutOfRange,
            ),
            (
                PositionError::NotCharBoundary { byte: 5 },
                RejectReason::NotABoundary,
            ),
            (
                PositionError::InsideSurrogatePair { utf16: 6 },
                RejectReason::NotABoundary,
            ),
            (
                PositionError::InsideLineBreak { char_idx: 2 },
                RejectReason::NotABoundary,
            ),
        ] {
            assert_eq!(RejectReason::from(&e), want, "{e:?}");
        }
        assert_eq!(RejectReason::NotABoundary.as_str(), "not_a_boundary");
        assert_eq!(
            RejectReason::NotABoundary.description(),
            "a position inside a character, a surrogate pair or a CRLF line break"
        );
        assert!(RejectReason::ALL.contains(&RejectReason::NotABoundary));
    }
}
