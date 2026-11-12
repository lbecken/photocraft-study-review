//! # kiln — desktop application (thin binary)
//!
//! Starts the `kiln-ui` desktop interface. The UI does not exist yet (scorecard `ui.*` items are
//! `planned`), so this binary only says so and exits with a failure status.

use std::process::ExitCode;

use kiln_ui as _;

fn main() -> ExitCode {
    eprintln!("kiln: the desktop UI is not built yet (see docs/scorecard.md, items ui.*)");
    ExitCode::FAILURE
}
