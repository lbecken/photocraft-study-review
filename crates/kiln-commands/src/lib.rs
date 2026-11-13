//! # kiln-commands — layer L3
//!
//! Responsibility (CHARTER §3): the command registry. Every state change goes through a
//! named command whose JSON params and results are described by JSON Schemas (generated from
//! the typed Rust structs) and validated by the registry. The UI, the CLI, the control
//! protocol and tests all go through [`Registry::execute`], which records every successful
//! command in the context's log ([`Context::log`]); [`Registry::replay`] re-executes a log.
//!
//! May depend on: L0–L2. Headless and deterministic.
//!
//! Adding a command: implement [`Command`] in the module of its group (one module per group,
//! CHARTER §3 decision 6), register it in that module's `register`, give it at least one
//! [`Example`], then run `cargo xtask schema --write` to refresh `docs/commands.json`.
//!
//! Errors: a refusal is [`CommandError::rejected`] with a [`RejectReason`] set where it is
//! raised, and an internal error carries an [`InternalReason`] (ADR-0004). Adding a kind is
//! one variant with its description in `reason.rs`, then `cargo xtask schema --write`: the
//! kinds are published as `error_kinds` in `docs/commands.json`.

mod buffer;
mod cursor;
mod diag;
mod doc;
mod edit;
mod eol;
mod find;
mod fmt;
mod keymap;
mod nav;
mod palette;
mod project;
mod reason;
mod registry;
mod search;
mod syntax;
mod tool;

pub use reason::{InternalReason, RejectReason};
pub use registry::{
    Command, CommandError, Context, Example, LogEntry, Registry, RegistryError, ReplayError,
};

/// The registry with every built-in command group registered.
pub fn standard_registry() -> Result<Registry, RegistryError> {
    let mut registry = Registry::new();
    buffer::register(&mut registry)?;
    doc::register(&mut registry)?;
    cursor::register(&mut registry)?;
    eol::register(&mut registry)?;
    project::register(&mut registry)?;
    syntax::register(&mut registry)?;
    search::register(&mut registry)?;
    find::register(&mut registry)?;
    nav::register(&mut registry)?;
    tool::register(&mut registry)?;
    diag::register(&mut registry)?;
    edit::register(&mut registry)?;
    fmt::register(&mut registry)?;
    palette::register(&mut registry)?;
    keymap::register(&mut registry)?;
    Ok(registry)
}
