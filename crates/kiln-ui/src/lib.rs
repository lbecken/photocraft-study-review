//! # kiln-ui — layer L5
//!
//! Responsibility (CHARTER §3): the egui desktop user interface. It may only dispatch
//! commands and read state; it never calls mutating APIs of lower crates. `cargo xtask
//! ui-commands` enforces this with the explicit import allowlist in `xtask/ui-allowlist.toml`.
//!
//! May depend on: L0–L4.
//!
//! The UI reaches the session through one [`kiln_control::Host`] (ADR-0008, ruling Q-1 A):
//! every change it makes is a control request, so it appears in the session log.
//!
//! What exists (T-055): the shell (a `Path` input, an `Open` button, a `Message` line and the
//! list of open `Documents`) and [`Script`], the runner of `kiln script`, which drives the UI
//! without a window from control requests and UI requests and reports what each frame shows
//! (`docs/protocol/ui.md`). The window is T-056.

mod json;
mod observe;
mod script;
mod session;
mod shell;

pub use script::{
    DEFAULT_SIZE, Files, KEY_NAMES, MAX_FRAMES, MAX_SCROLL_STEPS, SETTLE_CAP, SIZE_RANGE, Script,
    parse_size, response_ok,
};
