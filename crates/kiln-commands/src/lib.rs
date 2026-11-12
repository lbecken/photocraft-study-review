//! # kiln-commands — layer L3
//!
//! Responsibility (CHARTER §3): the command registry. Every state change goes through a
//! named command whose JSON params and results are described by JSON Schemas (generated from
//! the typed Rust structs) and validated by the registry. The UI, the CLI, the control
//! protocol and tests all go through [`Registry::execute`]. (Command log for replay: not yet.)
//!
//! May depend on: L0–L2. Headless and deterministic.
//!
//! Adding a command: implement [`Command`] in the module of its group (one module per group,
//! CHARTER §3 decision 6), register it in that module's `register`, give it at least one
//! [`Example`], then run `cargo xtask schema --write` to refresh `docs/commands.json`.

mod buffer;
mod registry;

pub use registry::{Command, CommandError, Context, Example, Registry, RegistryError};

/// The registry with every built-in command group registered.
pub fn standard_registry() -> Result<Registry, RegistryError> {
    let mut registry = Registry::new();
    buffer::register(&mut registry)?;
    Ok(registry)
}
