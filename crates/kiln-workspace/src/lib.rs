//! # kiln-workspace — layer L2
//!
//! Responsibility (CHARTER §3): virtual file system (memory and disk), open documents,
//! project tree, find-in-files, diagnostics store, and external tools (`cargo check`,
//! `rustfmt`) behind a trait with a recorded-output fake.
//!
//! May depend on: L0–L1. Headless and deterministic: no wall clock, randomness or
//! thread-timing dependence in behaviour.
//!
//! Implemented so far:
//! - [`RelPath`]: validated root-relative paths.
//! - [`Vfs`] with [`MemoryFs`] (deterministic reference model) and [`DiskFs`] (atomic
//!   replace via a deterministic temporary file and rename).
//! - [`decode`] / [`encode`]: UTF-8 (with or without BOM) and UTF-16 LE/BE with BOM; invalid
//!   input is reported with a byte offset, never repaired.
//! - [`Document`]: a `kiln_text::Buffer` opened from a VFS, content-based dirty tracking and
//!   byte-exact save (line endings are kept verbatim).
//! - [`ToolRunner`]: the external-tool contract; the default runner [`NoTools`] never starts
//!   a process.

mod disk;
mod document;
mod encoding;
mod memory;
mod path;
mod vfs;

pub mod diagnostics;
pub mod find;
pub mod format;
pub mod project;
pub mod tools;

pub use disk::DiskFs;
pub use document::*;
pub use encoding::{DecodeError, Encoding, decode, encode};
pub use memory::MemoryFs;
pub use path::{PathError, RelPath};
pub use tools::{NoTools, Tool, ToolError, ToolOutput, ToolRequest, ToolRunner};
pub use vfs::*;
