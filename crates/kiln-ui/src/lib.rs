//! # kiln-ui — layer L5
//!
//! Responsibility (CHARTER §3): the egui desktop user interface. It may only dispatch
//! commands through the `kiln-commands` registry and read state; it never calls mutating
//! APIs of lower crates. `cargo xtask ui-commands` enforces this with the explicit import
//! allowlist in `xtask/ui-allowlist.toml`.
//!
//! May depend on: L0–L4.
//!
//! Status: intentionally empty. egui/eframe arrive with the first UI task, through review.
