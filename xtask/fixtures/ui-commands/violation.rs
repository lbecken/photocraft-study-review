// Self-test fixture for `cargo xtask ui-commands`: every lower-crate path below except
// `kiln_commands::Registry` must be reported. Not compiled.
use kiln_commands::Registry;
use kiln_text::{Buffer, EditError as E};
use kiln_commands::*;

fn edit(b: &mut kiln_text::Buffer, r: &Registry) {
    b.insert(0, "x").ok();
    let _ = ::kiln_workspace::open("x");
    let _ = format!("{:?}", kiln_search::find(b, "x"));
    extern crate kiln_control as ctl;
}
