//! # kiln-syntax — layer L1
//!
//! Responsibility (CHARTER §3): tree-sitter integration for Rust, incremental reparse,
//! highlight spans, outline and bracket matching.
//!
//! May depend on: `kiln-text` (L0). Must stay headless: no UI or GPU crate may appear in its
//! dependency closure (`cargo xtask layers`).
//!
//! Status: intentionally empty. The tree-sitter dependency arrives with the first task that
//! needs it, through review (dependencies are high-tier changes).
