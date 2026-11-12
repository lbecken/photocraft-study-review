//! # kiln-workspace — layer L2
//!
//! Responsibility (CHARTER §3): virtual file system (memory and disk), open documents,
//! project tree, find-in-files, diagnostics store, and external tools (`cargo check`,
//! `rustfmt`) behind a trait with a recorded-output fake.
//!
//! May depend on: L0–L1. Headless and deterministic: no wall clock, randomness or
//! thread-timing dependence in behaviour.
//!
//! Status: intentionally empty until its first scorecard task.
